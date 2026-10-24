//! The matchmaker (openspec/changes/matchmaker, design 10): the server browser's list of running sessions, each with
//! a join code, and the relay that carries a client's offer to a server and the server's answer back. It holds no
//! game state and keeps nothing on disk; game traffic never passes through it.
//!
//! [`Hub::handle`] answers one request and is all the behaviour; `main.rs` only reads requests off sockets, one
//! thread a connection, so the tests drive the hub directly. A long poll (a server waiting for offers, a client
//! waiting for its answer) waits on one condition variable that every change notifies.

use sc_net::http::Request;
use sc_net::matchmaker::{
    self as mm, Answer, Heartbeat, Join, List, Listing, Offer, Offers, Refusal, Register, Registered, CODE_ALPHABET,
    CODE_LEN, KEY_HEADER, MAX_SDP_BYTES, MAX_SESSION_NAME_CHARS, OFFER_POLL_S,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::net::IpAddr;
use std::sync::{Condvar, Mutex};
use std::time::{Duration, Instant};

/// The matchmaker's tuning (`data/matchmaker/config.json`), validated on load (CLAUDE.md 6.5).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// How often a server heartbeats, in seconds.
    pub heartbeat_s: f64,
    /// A listing with no heartbeat for this long drops off the list, in seconds.
    pub listing_ttl_s: f64,
    /// How long a dropped listing's code is kept for its host key, in seconds.
    pub code_hold_s: f64,
    /// How long a client's join waits for the server's answer, in seconds.
    pub answer_wait_s: f64,
    /// Join attempts allowed from one address in a minute.
    pub join_attempts_per_min: u32,
    /// Join attempts allowed from one address in a day.
    pub join_attempts_per_day: u32,
    /// Listings held at once; a registration past it is refused.
    pub max_listings: usize,
}

impl Config {
    /// Parse and check the config: every time positive, the limits at least one.
    pub fn parse(text: &str) -> Result<Self, String> {
        let c: Config = serde_json::from_str(text).map_err(|e| format!("data/matchmaker/config.json: {e}"))?;
        for (name, v) in [
            ("heartbeat_s", c.heartbeat_s),
            ("listing_ttl_s", c.listing_ttl_s),
            ("code_hold_s", c.code_hold_s),
            ("answer_wait_s", c.answer_wait_s),
        ] {
            if !(v.is_finite() && v > 0.0) {
                return Err(format!("data/matchmaker/config.json: {name} must be a positive number of seconds"));
            }
        }
        if c.listing_ttl_s <= c.heartbeat_s {
            return Err("data/matchmaker/config.json: listing_ttl_s must be longer than heartbeat_s".into());
        }
        if c.join_attempts_per_min == 0 || c.join_attempts_per_day == 0 || c.max_listings == 0 {
            return Err("data/matchmaker/config.json: the limits must be at least 1".into());
        }
        Ok(c)
    }
}

/// The shortest host key accepted, in characters.
pub const MIN_KEY_CHARS: usize = 16;

struct Server {
    key: String,
    code: String,
    reg: Register,
    beat: Heartbeat,
    seen: Instant,
    offers: VecDeque<Offer>,
}

#[derive(Default)]
struct Inner {
    /// Live listings by id.
    servers: BTreeMap<String, Server>,
    /// Codes of dropped listings, kept for their key until the time given.
    held: HashMap<String, (String, Instant)>,
    /// Joins waiting for their answer, by join id.
    answers: HashMap<String, Option<Answer>>,
    /// Each address's join attempts in the last day.
    attempts: HashMap<IpAddr, VecDeque<Instant>>,
}

/// The matchmaker's state and its one request handler.
pub struct Hub {
    cfg: Config,
    inner: Mutex<Inner>,
    changed: Condvar,
}

/// A response: its HTTP status and JSON body.
pub type Reply = (u16, Vec<u8>);

fn json<T: Serialize>(v: &T) -> Reply {
    (200, serde_json::to_vec(v).unwrap_or_default())
}

fn refuse(status: u16, reason: impl Into<String>) -> Reply {
    (status, serde_json::to_vec(&Refusal { reason: reason.into() }).unwrap_or_default())
}

/// `n` random characters of `alphabet`, from the operating system's secure source.
fn random(alphabet: &[u8], n: usize) -> String {
    let mut bytes = vec![0u8; n];
    getrandom::getrandom(&mut bytes).expect("the operating system's random source");
    // 256 is a multiple of the 32 and 16 character alphabets used here, so taking the remainder is unbiased.
    bytes.iter().map(|b| alphabet[usize::from(*b) % alphabet.len()] as char).collect()
}

/// A session name as the list shows it: no control characters, trimmed, at most [`MAX_SESSION_NAME_CHARS`].
pub fn clean_session_name(name: &str) -> String {
    let n: String = name.chars().filter(|c| !c.is_control()).take(MAX_SESSION_NAME_CHARS).collect();
    let n = n.trim();
    if n.is_empty() {
        "Unnamed crew".into()
    } else {
        n.to_owned()
    }
}

impl Hub {
    /// An empty matchmaker.
    pub fn new(cfg: Config) -> Self {
        Self { cfg, inner: Mutex::new(Inner::default()), changed: Condvar::new() }
    }

    /// Drop listings past their time to live (holding their codes) and held codes past theirs.
    fn sweep(&self, i: &mut Inner, now: Instant) {
        let ttl = Duration::from_secs_f64(self.cfg.listing_ttl_s);
        let hold = Duration::from_secs_f64(self.cfg.code_hold_s);
        let gone: Vec<String> =
            i.servers.iter().filter(|(_, s)| now.duration_since(s.seen) > ttl).map(|(id, _)| id.clone()).collect();
        for id in gone {
            if let Some(s) = i.servers.remove(&id) {
                i.held.insert(s.code, (s.key, now + hold));
            }
        }
        i.held.retain(|_, (_, until)| *until > now);
    }

    /// Answer one request from `from`. Long polls block the calling thread, at most [`OFFER_POLL_S`] or the config's
    /// `answer_wait_s`.
    pub fn handle(&self, req: &Request, from: IpAddr) -> Reply {
        let path = req.path.split('?').next().unwrap_or_default();
        let parts: Vec<&str> = path.trim_matches('/').split('/').collect();
        let key = req.header(KEY_HEADER).unwrap_or_default();
        match (req.method.as_str(), parts.as_slice()) {
            ("GET", ["v1", "servers"]) => self.list(),
            ("POST", ["v1", "servers"]) => self.register(key, &req.body),
            ("POST", ["v1", "servers", id, "heartbeat"]) => self.heartbeat(key, id, &req.body),
            ("GET", ["v1", "servers", id, "offers"]) => self.offers(key, id),
            ("POST", ["v1", "servers", id, "answers"]) => self.answer(key, id, &req.body),
            ("DELETE", ["v1", "servers", id]) => self.unlist(key, id),
            ("POST", ["v1", "join", code]) => self.join(code, &req.body, from),
            ("GET", ["healthz"]) => (200, b"{}".to_vec()),
            ("OPTIONS", _) => (200, Vec::new()),
            _ => refuse(404, "no such request"),
        }
    }

    fn list(&self) -> Reply {
        let mut i = self.inner.lock().expect("hub");
        self.sweep(&mut i, Instant::now());
        let mut servers: Vec<Listing> = i
            .servers
            .values()
            .filter(|s| s.reg.listed)
            .map(|s| Listing {
                code: s.code.clone(),
                name: s.reg.name.clone(),
                mission: s.reg.mission.clone(),
                phase: s.beat.phase.clone(),
                crew: s.beat.crew,
                seats: s.reg.seats,
                protocol: s.reg.protocol,
            })
            .collect();
        servers.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()).then(a.code.cmp(&b.code)));
        json(&List { servers })
    }

    fn register(&self, key: &str, body: &[u8]) -> Reply {
        if key.chars().count() < MIN_KEY_CHARS {
            return refuse(401, format!("a host key of at least {MIN_KEY_CHARS} characters is needed"));
        }
        let mut reg: Register = match mm::parse(body) {
            Ok(r) => r,
            Err(e) => return refuse(400, e),
        };
        reg.name = clean_session_name(&reg.name);
        reg.mission = reg.mission.chars().filter(char::is_ascii_graphic).take(64).collect();
        let now = Instant::now();
        let mut i = self.inner.lock().expect("hub");
        self.sweep(&mut i, now);
        // A server that restarted before its listing dropped replaces it, keeping the code.
        let mine: Option<String> = i.servers.iter().find(|(_, s)| s.key == key).map(|(id, _)| id.clone());
        let mut code = mine.as_ref().and_then(|id| i.servers.remove(id)).map(|s| s.code);
        if code.is_none() {
            if let Some(want) = reg.previous_code.as_deref().map(mm::normal_code) {
                if i.held.get(&want).is_some_and(|(k, _)| k == key) {
                    i.held.remove(&want);
                    code = Some(want);
                }
            }
        }
        if i.servers.len() >= self.cfg.max_listings {
            return refuse(503, "the matchmaker is full");
        }
        let code = code.unwrap_or_else(|| loop {
            let c = random(CODE_ALPHABET, CODE_LEN);
            if !i.servers.values().any(|s| s.code == c) && !i.held.contains_key(&c) {
                break c;
            }
        });
        let id = random(b"0123456789abcdef", 16);
        i.servers.insert(
            id.clone(),
            Server {
                key: key.to_owned(),
                code: code.clone(),
                reg,
                beat: Heartbeat { phase: "Muster".into(), crew: 0 },
                seen: now,
                offers: VecDeque::new(),
            },
        );
        self.changed.notify_all();
        json(&Registered { id, code, heartbeat_s: self.cfg.heartbeat_s })
    }

    /// The listing `id`, if `key` made it.
    fn owned<'a>(i: &'a mut Inner, key: &str, id: &str) -> Result<&'a mut Server, Reply> {
        match i.servers.get_mut(id) {
            Some(s) if s.key == key => Ok(s),
            Some(_) => Err(refuse(401, "that listing is another host's")),
            None => Err(refuse(404, "no such listing (register again)")),
        }
    }

    fn heartbeat(&self, key: &str, id: &str, body: &[u8]) -> Reply {
        let beat: Heartbeat = match mm::parse(body) {
            Ok(b) => b,
            Err(e) => return refuse(400, e),
        };
        let now = Instant::now();
        let mut i = self.inner.lock().expect("hub");
        self.sweep(&mut i, now);
        match Self::owned(&mut i, key, id) {
            Ok(s) => {
                s.beat = Heartbeat { phase: beat.phase.chars().take(16).collect(), crew: beat.crew.min(s.reg.seats) };
                s.seen = now;
                (200, b"{}".to_vec())
            }
            Err(r) => r,
        }
    }

    fn offers(&self, key: &str, id: &str) -> Reply {
        let end = Instant::now() + Duration::from_secs(OFFER_POLL_S);
        let mut i = self.inner.lock().expect("hub");
        loop {
            let now = Instant::now();
            self.sweep(&mut i, now);
            match Self::owned(&mut i, key, id) {
                Ok(s) => {
                    if !s.offers.is_empty() || now >= end {
                        let offers: Vec<Offer> = s.offers.drain(..).collect();
                        return json(&Offers { offers });
                    }
                }
                Err(r) => return r,
            }
            i = self.changed.wait_timeout(i, end.saturating_duration_since(now)).expect("hub").0;
        }
    }

    fn answer(&self, key: &str, id: &str, body: &[u8]) -> Reply {
        if body.len() > MAX_SDP_BYTES + 1024 {
            return refuse(413, "the answer is too large");
        }
        let a: Answer = match mm::parse(body) {
            Ok(a) => a,
            Err(e) => return refuse(400, e),
        };
        let mut i = self.inner.lock().expect("hub");
        if let Err(r) = Self::owned(&mut i, key, id) {
            return r;
        }
        match i.answers.get_mut(&a.join_id) {
            Some(slot @ None) => {
                *slot = Some(a);
                self.changed.notify_all();
                (200, b"{}".to_vec())
            }
            _ => refuse(404, "no join is waiting for that answer"),
        }
    }

    fn unlist(&self, key: &str, id: &str) -> Reply {
        let now = Instant::now();
        let mut i = self.inner.lock().expect("hub");
        if let Err(r) = Self::owned(&mut i, key, id) {
            return r;
        }
        if let Some(s) = i.servers.remove(id) {
            let hold = Duration::from_secs_f64(self.cfg.code_hold_s);
            i.held.insert(s.code, (s.key, now + hold));
        }
        self.changed.notify_all();
        (200, b"{}".to_vec())
    }

    /// Count a join attempt from `from`; false when it is over a limit.
    fn attempt(&self, i: &mut Inner, from: IpAddr, now: Instant) -> bool {
        let day = Duration::from_secs(86_400);
        let a = i.attempts.entry(from).or_default();
        while a.front().is_some_and(|t| now.duration_since(*t) > day) {
            a.pop_front();
        }
        let minute = a.iter().filter(|t| now.duration_since(**t) <= Duration::from_secs(60)).count();
        if minute >= self.cfg.join_attempts_per_min as usize || a.len() >= self.cfg.join_attempts_per_day as usize {
            return false;
        }
        a.push_back(now);
        true
    }

    fn join(&self, code: &str, body: &[u8], from: IpAddr) -> Reply {
        let now = Instant::now();
        let mut i = self.inner.lock().expect("hub");
        // Counted before the code is looked at, so guessing is limited whatever the guesses (design section 3).
        if !self.attempt(&mut i, from, now) {
            return refuse(429, "too many join attempts; wait a minute");
        }
        if body.len() > MAX_SDP_BYTES + 1024 {
            return refuse(413, "the offer is too large");
        }
        let join: Join = match mm::parse(body) {
            Ok(j) => j,
            Err(e) => return refuse(400, e),
        };
        self.sweep(&mut i, now);
        let code = mm::normal_code(code);
        let Some(server) = i.servers.values_mut().find(|s| s.code == code) else {
            return refuse(404, format!("no crew holds the code {code}"));
        };
        if join.protocol != server.reg.protocol {
            return refuse(
                409,
                format!(
                    "this game speaks protocol {}, the crew's server {}: update the {}",
                    join.protocol,
                    server.reg.protocol,
                    if join.protocol < server.reg.protocol { "game" } else { "server" }
                ),
            );
        }
        if server.beat.crew >= server.reg.seats {
            return refuse(409, "that crew is full");
        }
        let join_id = random(b"0123456789abcdef", 16);
        server.offers.push_back(Offer { join_id: join_id.clone(), offer: join.offer });
        i.answers.insert(join_id.clone(), None);
        self.changed.notify_all();
        let end = now + Duration::from_secs_f64(self.cfg.answer_wait_s);
        loop {
            if let Some(Some(a)) = i.answers.get(&join_id) {
                let a = a.clone();
                i.answers.remove(&join_id);
                return match (a.answer, a.refused) {
                    (Some(sdp), _) => json(&sdp),
                    (None, reason) => refuse(409, reason.unwrap_or_else(|| "the server refused".into())),
                };
            }
            let left = end.saturating_duration_since(Instant::now());
            if left.is_zero() {
                i.answers.remove(&join_id);
                for s in i.servers.values_mut() {
                    s.offers.retain(|o| o.join_id != join_id);
                }
                return refuse(504, "the crew's server did not answer");
            }
            i = self.changed.wait_timeout(i, left).expect("hub").0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn cfg() -> Config {
        Config::parse(include_str!("../../../data/matchmaker/config.json")).expect("the shipped config")
    }

    fn req(method: &str, path: &str, key: &str, body: &str) -> Request {
        Request {
            method: method.into(),
            path: path.into(),
            headers: vec![(KEY_HEADER.into(), key.into())],
            body: body.as_bytes().to_vec(),
        }
    }

    const KEY: &str = "0123456789abcdef0123";
    const HOME: IpAddr = IpAddr::V4(std::net::Ipv4Addr::new(10, 0, 0, 2));

    fn reg(hub: &Hub, name: &str, listed: bool) -> Registered {
        let body = format!(
            r#"{{"name":"{name}","listed":{listed},"mission":"drill-hound","protocol":{},"seats":8}}"#,
            sc_net::msg::PROTOCOL
        );
        let (st, b) = hub.handle(&req("POST", "/v1/servers", KEY, &body), HOME);
        assert_eq!(st, 200, "{}", String::from_utf8_lossy(&b));
        mm::parse(&b).unwrap()
    }

    fn list(hub: &Hub) -> List {
        mm::parse(&hub.handle(&req("GET", "/v1/servers", "", ""), HOME).1).unwrap()
    }

    #[test]
    fn a_listed_server_shows_its_phase_and_crew_and_an_unlisted_one_is_not_shown() {
        let hub = Hub::new(cfg());
        let r = reg(&hub, "Shoals Patrol", true);
        assert_eq!(r.code.len(), CODE_LEN);
        let beat = hub.handle(
            &req("POST", &format!("/v1/servers/{}/heartbeat", r.id), KEY, r#"{"phase":"Fight","crew":2}"#),
            HOME,
        );
        assert_eq!(beat.0, 200);
        let l = list(&hub);
        assert_eq!(l.servers.len(), 1);
        assert_eq!(
            (l.servers[0].name.as_str(), l.servers[0].phase.as_str(), l.servers[0].crew),
            ("Shoals Patrol", "Fight", 2)
        );
        let other = Hub::new(cfg());
        reg(&other, "Private", false);
        assert!(list(&other).servers.is_empty(), "an unlisted server is joined by code only");
    }

    #[test]
    fn a_listing_drops_off_after_its_time_to_live_and_its_host_gets_the_code_back() {
        let hub = Hub::new(Config { heartbeat_s: 0.01, listing_ttl_s: 0.05, ..cfg() });
        let r = reg(&hub, "Shoals Patrol", true);
        std::thread::sleep(Duration::from_millis(80));
        assert!(list(&hub).servers.is_empty(), "a silent server drops off the list");
        let body = format!(
            r#"{{"name":"Shoals Patrol","listed":true,"mission":"drill-hound","protocol":{},"seats":8,"previous_code":"{}"}}"#,
            sc_net::msg::PROTOCOL,
            r.code
        );
        let again: Registered = mm::parse(&hub.handle(&req("POST", "/v1/servers", KEY, &body), HOME).1).unwrap();
        assert_eq!(again.code, r.code, "the code is kept for the host that held it");
    }

    #[test]
    fn another_host_cannot_heartbeat_a_listing_or_take_its_code() {
        let hub = Hub::new(cfg());
        let r = reg(&hub, "Shoals Patrol", true);
        let other = "ffffffffffffffffffff";
        let st = hub
            .handle(
                &req("POST", &format!("/v1/servers/{}/heartbeat", r.id), other, r#"{"phase":"Fight","crew":1}"#),
                HOME,
            )
            .0;
        assert_eq!(st, 401);
        assert_eq!(hub.handle(&req("DELETE", &format!("/v1/servers/{}", r.id), other, ""), HOME).0, 401);
    }

    #[test]
    fn a_join_is_relayed_to_the_server_and_its_answer_back() {
        let hub = Arc::new(Hub::new(cfg()));
        let r = reg(&hub, "Shoals Patrol", true);
        let (h2, id) = (hub.clone(), r.id.clone());
        let server = std::thread::spawn(move || {
            let o: Offers =
                mm::parse(&h2.handle(&req("GET", &format!("/v1/servers/{id}/offers"), KEY, ""), HOME).1).unwrap();
            assert_eq!(o.offers.len(), 1);
            assert_eq!(o.offers[0].offer, serde_json::json!({"sdp": "offer"}));
            let a = serde_json::to_string(&Answer {
                join_id: o.offers[0].join_id.clone(),
                answer: Some(serde_json::json!({"sdp": "answer"})),
                refused: None,
            })
            .unwrap();
            assert_eq!(h2.handle(&req("POST", &format!("/v1/servers/{id}/answers"), KEY, &a), HOME).0, 200);
        });
        let body = format!(r#"{{"protocol":{},"offer":{{"sdp":"offer"}}}}"#, sc_net::msg::PROTOCOL);
        let (st, b) = hub.handle(&req("POST", &format!("/v1/join/{}", r.code.to_lowercase()), "", &body), HOME);
        server.join().unwrap();
        assert_eq!(st, 200, "{}", String::from_utf8_lossy(&b));
        assert_eq!(serde_json::from_slice::<serde_json::Value>(&b).unwrap(), serde_json::json!({"sdp": "answer"}));
    }

    #[test]
    fn a_wrong_code_and_another_protocol_are_refused_with_their_reasons() {
        let hub = Hub::new(cfg());
        let r = reg(&hub, "Shoals Patrol", true);
        let (st, b) = hub.handle(&req("POST", "/v1/join/ZZZZZZ", "", r#"{"protocol":1,"offer":{}}"#), HOME);
        assert_eq!(st, 404);
        assert!(String::from_utf8_lossy(&b).contains("ZZZZZZ"));
        let (st, b) =
            hub.handle(&req("POST", &format!("/v1/join/{}", r.code), "", r#"{"protocol":1,"offer":{}}"#), HOME);
        assert_eq!(st, 409);
        assert!(String::from_utf8_lossy(&b).contains("protocol"), "the refusal names the mismatch");
    }

    #[test]
    fn the_eleventh_join_attempt_in_a_minute_is_refused_before_any_code_is_checked() {
        let hub = Hub::new(cfg());
        let guess = |hub: &Hub| hub.handle(&req("POST", "/v1/join/AAAAAA", "", r#"{"protocol":1,"offer":{}}"#), HOME).0;
        for _ in 0..10 {
            assert_eq!(guess(&hub), 404);
        }
        assert_eq!(guess(&hub), 429);
    }

    #[test]
    fn session_names_are_cleaned_for_the_list() {
        assert_eq!(clean_session_name("  Shoals\u{7} Patrol  "), "Shoals Patrol");
        assert_eq!(clean_session_name(""), "Unnamed crew");
        assert_eq!(clean_session_name(&"x".repeat(80)).chars().count(), MAX_SESSION_NAME_CHARS);
    }
}
