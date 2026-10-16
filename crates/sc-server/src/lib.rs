//! The drill server (openspec/changes/coop-drill design sections 1, 4 and 6): one authoritative drill, the
//! connections that play it, snapshots at 20 Hz, a status page, a log line per round.
//!
//! It is a library so the binary and the in-process network test run the same server (CLAUDE.md 6.3: one code path
//! for every game). It is the core plus a network adapter: every rule is `sc-core::combat`'s; this file maps peers
//! to players and messages to the drill's calls.

pub mod bridge;

use std::collections::BTreeMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::atomic::Ordering;
use std::time::Instant;

use sc_core::combat::chat::{self, ChatLog};
use sc_core::combat::{data::DrillData, Drill, DrillEvent, Refusal, DT, TICK_HZ};
use sc_net::link::Link;
use sc_net::msg::{ClientMsg, CrewEntry, ServerMsg, Snapshot, StickMsg, PROTOCOL};
use sc_net::transport::{Chan, Dial, Impair, NetEvent, PeerId, ServerNet};

/// Snapshots a second (netcode-and-sessions section 3).
pub const SNAPSHOT_HZ: f64 = 20.0;
/// The power grid rides in every this-many snapshots (2 Hz, power-grid 14), and in every other one while
/// Engineering drags a fader, so its ghost keeps up with the hand.
pub const POWER_EVERY: u64 = 10;
pub const POWER_PREVIEW_EVERY: u64 = 2;
/// A peer silent this long is dropped and its seat returns to automation (netcode-and-sessions section 2:
/// "the server also drops a client silent for 5 s"). Every client pings once a second, so a live one never is.
/// A peer is silent only when it has been silent this long on both clocks (browser-single-player design 4): the
/// server's own ticks, so a whole game that pauses (a hidden browser tab runs no frames) drops nobody, because its
/// server paused too; and the wall clock, so a server stepped faster than real time (the tests, a headless capture)
/// does not drop a client that pings once a real second. A dedicated server ticks in real time, so for it the two
/// are the same 5 s.
pub const SILENT_S: f64 = 5.0;

/// Where the server listens (browser-single-player design 2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Listen {
    /// Signalling and status on an address, for example `0.0.0.0:7700`; peers over WebRTC.
    Addr(SocketAddr),
    /// The local link: clients in this process, no socket and no thread (single player).
    Local,
}

impl From<SocketAddr> for Listen {
    fn from(a: SocketAddr) -> Self {
        Listen::Addr(a)
    }
}

/// What the server was started with.
#[derive(Clone, Debug)]
pub struct ServerConfig {
    /// Where it listens: an address, or the local link.
    pub listen: Listen,
    /// The address peers' UDP sockets bind and offer.
    pub host_ip: IpAddr,
    /// The test impairment.
    pub impair: Impair,
    /// The drill's seed.
    pub seed: u64,
    /// The bridge the crew walk on (coop-drill design 9), or none for claims that seat at once.
    pub bridge: Option<sc_core::combat::bodies::Bridge>,
}

struct Conn {
    slot: Option<u8>,
    stick: Link,
    snap: Link,
    dropped: u64,
    opened: Instant,
    /// When anything last arrived from this peer: the server's tick count, and the wall clock.
    heard_tick: u64,
    heard: Instant,
}

/// A round as the log records it.
#[derive(Clone, Debug, PartialEq)]
pub struct RoundResult {
    /// The round.
    pub round: u32,
    /// "Victory", "Defeat" or "Withdrew".
    pub outcome: String,
    /// Seconds of Engage.
    pub engage_s: f64,
    /// Who held each station ("name (bot)" or "automation").
    pub crew: String,
    /// The stats line.
    pub stats: String,
}

/// The server.
pub struct DrillServer {
    /// The drill.
    pub drill: Drill,
    net: ServerNet,
    conns: BTreeMap<PeerId, Conn>,
    snap_acc: f64,
    /// Snapshots sent: the power grid rides in some (POWER_EVERY).
    snaps_sent: u64,
    crew_dirty: bool,
    /// Rounds finished.
    pub results: Vec<RoundResult>,
    tick_ms: Vec<f64>,
    started: Instant,
    /// Ticks run: the clock the silence rule reads.
    ticks: u64,
    /// The crew chat's last lines, sent to a late joiner (crew-chat design 1).
    chat: ChatLog,
}

impl DrillServer {
    /// Start listening.
    pub fn start(data: DrillData, cfg: &ServerConfig) -> std::io::Result<Self> {
        let net = match cfg.listen {
            Listen::Addr(a) => ServerNet::start(a, cfg.host_ip, cfg.impair, cfg.seed)?,
            Listen::Local => ServerNet::local(),
        };
        let mut drill = Drill::new(data, cfg.seed);
        if let Some(b) = &cfg.bridge {
            drill.set_bridge(b.clone());
        }
        // Repairs on deck (all-hands 5): the Tern's jobs and their games, from the repository's root.
        match sc_core::repair::data::RepairData::load_for_ship(|rel| {
            std::fs::read_to_string(rel).map_err(|e| format!("{rel}: {e}"))
        }) {
            Ok(r) => drill.set_repairs(r),
            Err(e) => eprintln!("sc-server: no repairs on deck ({e})"),
        }
        Ok(Self {
            drill,
            net,
            conns: BTreeMap::new(),
            snap_acc: 0.0,
            snaps_sent: 0,
            crew_dirty: false,
            results: Vec::new(),
            tick_ms: Vec::with_capacity(1024),
            started: Instant::now(),
            ticks: 0,
            chat: ChatLog::default(),
        })
    }

    /// The signalling address (`0.0.0.0:0` on the local link).
    pub fn addr(&self) -> SocketAddr {
        self.net.addr
    }

    /// Where a client connects to reach this server: its address, or the local link's door.
    pub fn dial(&self) -> Dial {
        self.net.dial()
    }

    fn send(&self, peer: PeerId, m: &ServerMsg) {
        self.net.send(peer, Chan::Command, m.encode());
    }

    fn broadcast(&self, m: &ServerMsg) {
        let b = m.encode();
        for (p, c) in &self.conns {
            if c.slot.is_some() {
                self.net.send(*p, Chan::Command, b.clone());
            }
        }
    }

    fn peer_of(&self, slot: u8) -> Option<PeerId> {
        self.conns.iter().find(|(_, c)| c.slot == Some(slot)).map(|(p, _)| *p)
    }

    /// Take in the network; returns lines worth logging.
    pub fn poll(&mut self) -> Vec<String> {
        let mut log = Vec::new();
        let mut events = self.net.poll();
        for e in &events {
            if let NetEvent::Data(p, ..) = e {
                if let Some(c) = self.conns.get_mut(p) {
                    c.heard_tick = self.ticks;
                    c.heard = Instant::now();
                }
            }
        }
        let limit = (SILENT_S * TICK_HZ).ceil() as u64;
        let silent: Vec<PeerId> = self
            .conns
            .iter()
            .filter(|(_, c)| {
                self.ticks.saturating_sub(c.heard_tick) > limit && c.heard.elapsed().as_secs_f64() > SILENT_S
            })
            .map(|(p, _)| *p)
            .collect();
        for p in silent {
            log.push(format!("peer {p} silent for {SILENT_S} s: dropped"));
            self.net.close(p);
            events.push(NetEvent::Closed(p));
        }
        for e in events {
            match e {
                NetEvent::Open(p) => {
                    self.conns.insert(
                        p,
                        Conn {
                            slot: None,
                            stick: Link::default(),
                            snap: Link::default(),
                            dropped: 0,
                            opened: Instant::now(),
                            heard_tick: self.ticks,
                            heard: Instant::now(),
                        },
                    );
                }
                NetEvent::Closed(p) => {
                    if let Some(c) = self.conns.remove(&p) {
                        if let Some(slot) = c.slot {
                            let name = self
                                .drill
                                .players
                                .iter()
                                .find(|x| x.slot == slot)
                                .map(|x| x.name.clone())
                                .unwrap_or_default();
                            self.drill.leave(slot);
                            self.crew_dirty = true;
                            log.push(format!(
                                "{name} (slot {slot}) left after {:.0} s",
                                c.opened.elapsed().as_secs_f64()
                            ));
                        }
                    }
                }
                NetEvent::Data(p, Chan::Command, d) => match ClientMsg::decode(&d) {
                    Ok(m) => {
                        if let Some(line) = self.on_message(p, m) {
                            log.push(line);
                        }
                    }
                    Err(_) => {
                        if let Some(c) = self.conns.get_mut(&p) {
                            c.dropped += 1;
                        }
                    }
                },
                NetEvent::Data(p, Chan::Input, d) => {
                    let Some(c) = self.conns.get_mut(&p) else { continue };
                    match StickMsg::decode(&d) {
                        Ok(m) => {
                            if c.stick.receive(&m.header) {
                                if let Some(slot) = c.slot {
                                    // A stick from a seat that is not the helm's is simply not applied.
                                    let _ = self.drill.command(slot, m.stick);
                                }
                            }
                        }
                        Err(_) => c.dropped += 1,
                    }
                }
                NetEvent::Data(p, Chan::Snapshot, _) => {
                    if let Some(c) = self.conns.get_mut(&p) {
                        c.dropped += 1;
                    }
                }
            }
        }
        log
    }

    fn on_message(&mut self, p: PeerId, m: ClientMsg) -> Option<String> {
        let slot = self.conns.get(&p)?.slot;
        match (slot, m) {
            (None, ClientMsg::Hello { protocol, name, bot, station }) => {
                if protocol != PROTOCOL {
                    self.send(p, &ServerMsg::Refused(Refusal::BadValue));
                    return Some(format!("refused {name}: protocol {protocol}, ours is {PROTOCOL}"));
                }
                match self.drill.join(&name, bot) {
                    Ok(s) => {
                        self.conns.get_mut(&p)?.slot = Some(s);
                        self.send(
                            p,
                            &ServerMsg::Welcome {
                                protocol: PROTOCOL,
                                slot: s,
                                mission: self.drill.data.mission.id.clone(),
                            },
                        );
                        // A bot walks to its station; a player on foot walks there themselves (the law), so a station
                        // out of their reach is no refusal, only where they are headed.
                        if let Some(st) = station {
                            match self.drill.claim(s, st) {
                                Ok(()) | Err(Refusal::TooFar) => {}
                                Err(r) => self.send(p, &ServerMsg::Refused(r)),
                            }
                        }
                        // A late joiner sees the last lines, oldest first (crew-chat design 1).
                        for l in self.chat.lines() {
                            self.send(p, &ServerMsg::Chat(l.clone()));
                        }
                        self.crew_dirty = true;
                        let addr = self
                            .net
                            .peer_stats()
                            .into_iter()
                            .find(|x| x.0 == p)
                            .map(|x| x.1.to_string())
                            .unwrap_or_default();
                        Some(format!(
                            "{name}{} joined from {addr} as slot {s}, wanting {}",
                            if bot { " (bot)" } else { "" },
                            station.map(|s| s.name()).unwrap_or("nothing")
                        ))
                    }
                    Err(r) => {
                        self.send(p, &ServerMsg::Refused(r));
                        Some(format!("refused {name}: {}", r.text()))
                    }
                }
            }
            (Some(s), ClientMsg::Claim(st)) => {
                match self.drill.claim(s, st) {
                    Ok(()) => self.crew_dirty = true,
                    Err(r) => self.send(p, &ServerMsg::Refused(r)),
                }
                None
            }
            (Some(s), ClientMsg::Ready(r)) => {
                let _ = self.drill.set_ready(s, r);
                self.crew_dirty = true;
                None
            }
            // START MISSION (start-mission 2): the drill decides who may; anyone else hears why not.
            (Some(s), ClientMsg::StartMission) => {
                if let Err(r) = self.drill.start_mission(s) {
                    self.send(p, &ServerMsg::Refused(r));
                }
                None
            }
            (Some(s), ClientMsg::Command(c)) => {
                // A person's order or ack from a console is theirs in chat too (crew-chat design 3); a bot says its own.
                let said = self.console_said(s, &c);
                match self.drill.command(s, c) {
                    Err(r) => self.send(p, &ServerMsg::Refused(r)),
                    Ok(()) => {
                        if let Some(x) = said {
                            self.post(s, x.kind, x.text);
                        }
                    }
                }
                None
            }
            (_, ClientMsg::Ping(t)) => {
                self.send(p, &ServerMsg::Pong(t));
                None
            }
            // The player's own walk: refused steps are dropped (the next one from where the walk is will do).
            (Some(s), ClientMsg::Walk { pos, yaw }) => {
                let _ = self.drill.walked(s, pos, yaw);
                None
            }
            (Some(s), ClientMsg::TakeExtinguisher) => {
                if let Err(r) = self.drill.take_extinguisher(s) {
                    self.send(p, &ServerMsg::Refused(r));
                }
                None
            }
            (Some(s), ClientMsg::Spray(on)) => {
                let _ = self.drill.spray(s, on);
                None
            }
            // Repairs on deck (all-hands 5): the order back is the round to play, or none when the job is done.
            (Some(s), ClientMsg::Dock(job)) => {
                match self.drill.dock(s, usize::from(job)) {
                    Ok(o) => self.send(p, &ServerMsg::Repair { job, next: Some(order_bytes(o)) }),
                    Err(r) => self.send(p, &ServerMsg::Refused(r)),
                }
                None
            }
            (Some(s), ClientMsg::RoundDone { job, round, play_s }) => {
                match self.drill.round_done(s, usize::from(job), u32::from(round), f64::from(play_s)) {
                    Ok(next) => self.send(p, &ServerMsg::Repair { job, next: next.map(order_bytes) }),
                    Err(r) => self.send(p, &ServerMsg::Refused(r)),
                }
                None
            }
            (Some(s), ClientMsg::Undock) => {
                self.drill.undock(s);
                None
            }
            // Crew chat (crew-chat design 1): stamped with who and when, kept, and sent to everyone at a speaker's pace.
            (Some(s), ClientMsg::Chat { kind, text }) => {
                if let Some(text) = chat::clean_text(&text) {
                    self.post(s, kind, text);
                }
                None
            }
            (Some(s), ClientMsg::Stand) => {
                match self.drill.stand(s) {
                    Ok(()) => self.crew_dirty = true,
                    Err(r) => self.send(p, &ServerMsg::Refused(r)),
                }
                None
            }
            _ => None,
        }
    }

    /// Keep a line from the player in `slot`, stamped with their name, station and the drill's clock, and send it to
    /// everyone, unless they spoke inside their gap (crew-chat design 1 and 4).
    fn post(&mut self, slot: u8, kind: chat::Kind, text: String) {
        let Some(who) = self.drill.players.iter().find(|x| x.slot == slot) else { return };
        let line = chat::Line {
            slot: Some(slot),
            name: who.name.clone(),
            station: who.station,
            npc: who.bot,
            kind,
            text,
            at_s: self.drill.clock_s(),
        };
        if self.chat.post(line.clone(), &self.drill.data.chat.limits) {
            self.broadcast(&ServerMsg::Chat(line));
        }
    }

    /// What a person's console command says in chat: the captain's order and a station's ack, by the core's mapping. A
    /// bot's are none here: it says its own through the chat message (crew-chat design 3).
    fn console_said(&self, slot: u8, c: &sc_core::combat::Command) -> Option<chat::Said> {
        use sc_core::combat::Command;
        let who = self.drill.players.iter().find(|x| x.slot == slot)?;
        if who.bot {
            return None;
        }
        let d = &self.drill;
        match *c {
            Command::Order(..) => chat::said_for(&d.data, who.station?, c),
            Command::Ack(st) => {
                d.bridge.orders.iter().find(|o| o.to == st && !o.done).and_then(|o| chat::said_ack(&d.data, st, o.verb))
            }
            _ => None,
        }
    }

    /// Run one tick: step the drill, send what happened, and a snapshot when one is due. Returns lines to log.
    pub fn tick(&mut self) -> Vec<String> {
        let t0 = Instant::now();
        self.ticks += 1;
        let mut log = Vec::new();
        let before: Vec<_> = self.drill.players.iter().map(|p| (p.slot, p.station)).collect();
        self.drill.step();
        // A body that sat down this tick changed who holds a station (coop-drill design 9).
        if self.drill.players.iter().map(|p| (p.slot, p.station)).ne(before.iter().copied()) {
            self.crew_dirty = true;
        }
        if self.crew_dirty {
            self.crew_dirty = false;
            let crew = self
                .drill
                .players
                .iter()
                .map(|p| CrewEntry {
                    slot: p.slot,
                    name: p.name.clone(),
                    station: p.station,
                    ready: p.ready,
                    bot: p.bot,
                })
                .collect();
            self.broadcast(&ServerMsg::Crew(crew));
        }
        for e in self.drill.drain_events() {
            let m = match e {
                DrillEvent::Fired { bolt, owner, pos, vel } => ServerMsg::Fired { bolt, owner, pos, vel },
                DrillEvent::Hit { bolt, target, face, damage_mj } => ServerMsg::Hit { bolt, target, face, damage_mj },
                DrillEvent::Launched { missile, .. } => ServerMsg::Launched { missile },
                DrillEvent::Detonated { missile, pos, damage_mj } => ServerMsg::Detonated { missile, pos, damage_mj },
                DrillEvent::Destroyed { ship } => ServerMsg::Destroyed(ship),
                DrillEvent::Beam { owner, from, to, hit, .. } => ServerMsg::Beam { owner, from, to, hit },
                DrillEvent::DecoyOut { ship } => ServerMsg::Decoy(ship),
                DrillEvent::Burst { ship, pos, size } => ServerMsg::Burst { ship, pos, size },
                // A fooled missile shows as its early burst or its miss.
                DrillEvent::Fooled { .. } => continue,
                DrillEvent::Phase { phase, outcome } => {
                    log.push(format!("round {} phase {phase:?}", self.drill.round));
                    if phase == sc_core::combat::Phase::Debrief {
                        let r = self.round_result();
                        log.push(format!(
                            "round {} result: {} in {:.0} s; crew {}; {}",
                            r.round, r.outcome, r.engage_s, r.crew, r.stats
                        ));
                        self.results.push(r);
                    }
                    // A new Muster clears everyone's ready mark.
                    self.crew_dirty = true;
                    ServerMsg::Phase(phase, outcome)
                }
                DrillEvent::Relieved { slot, station } => {
                    let name = self.drill.players.iter().find(|p| p.slot == slot).map(|p| p.name.clone());
                    log.push(format!("{} relieved at {}", name.unwrap_or_default(), station.name()));
                    self.crew_dirty = true;
                    continue;
                }
                DrillEvent::Refused { slot, reason } => {
                    if let Some(p) = self.peer_of(slot) {
                        self.send(p, &ServerMsg::Refused(reason));
                    }
                    continue;
                }
                // The drill's own crew (automation, damage control, the computer), paced by the drill.
                DrillEvent::Said(line) => {
                    if !self.chat.post(line.clone(), &self.drill.data.chat.limits) {
                        continue;
                    }
                    ServerMsg::Chat(line)
                }
            };
            self.broadcast(&m);
        }
        self.snap_acc += SNAPSHOT_HZ * DT;
        if self.snap_acc >= 1.0 {
            self.snap_acc -= 1.0;
            let mut s = Snapshot::of(&self.drill);
            self.snaps_sent += 1;
            let every = if self.drill.power_preview.is_some() { POWER_PREVIEW_EVERY } else { POWER_EVERY };
            if self.snaps_sent % every != 0 {
                s.power = None;
            }
            // The routing rides half way between the grid's (power-routing 6.1): odd, so never beside it, even while
            // a fader's preview sends the grid every other snapshot.
            if self.snaps_sent % POWER_EVERY != POWER_EVERY / 2 {
                s.route = None;
            }
            for (p, c) in self.conns.iter_mut() {
                if c.slot.is_some() {
                    s.header = c.snap.next_header();
                    self.net.send(*p, Chan::Snapshot, s.encode());
                }
            }
        }
        let ms = t0.elapsed().as_secs_f64() * 1000.0;
        if self.tick_ms.len() >= 1024 {
            self.tick_ms.remove(0);
        }
        self.tick_ms.push(ms);
        if self.drill.tick % (TICK_HZ as u64) == 0 {
            self.net.set_status(self.status_json());
        }
        log
    }

    fn round_result(&self) -> RoundResult {
        let d = &self.drill;
        let crew = sc_core::combat::Station::ALL
            .iter()
            .map(|s| {
                let who = d
                    .players
                    .iter()
                    .find(|p| p.station == Some(*s))
                    .map(|p| format!("{}{}", p.name, if p.bot { " (bot)" } else { "" }))
                    .unwrap_or_else(|| "automation".into());
                format!("{} {who}", s.name())
            })
            .collect::<Vec<_>>()
            .join(", ");
        let st = &d.stats;
        RoundResult {
            round: d.round,
            outcome: format!("{:?}", d.outcome),
            engage_s: st.engage_s,
            crew,
            stats: format!(
                "Tern bolts {}/{} hit, Hound bolts {}/{} hit, missiles {}/{} hit, dealt {:.0} MJ, taken {:.0} MJ, Tern hull {:.0} MJ",
                st.tern_hits, st.tern_shots, st.enemy_hits, st.enemy_shots, st.missile_hits, st.missiles_fired, st.damage_dealt_mj, st.damage_taken_mj, d.ships[0].hull
            ),
        }
    }

    /// The status page's JSON: the drill, the crew, each connection's counters, the tick's cost.
    pub fn status_json(&self) -> String {
        let d = &self.drill;
        let mut sorted = self.tick_ms.clone();
        sorted.sort_by(f64::total_cmp);
        let pct = |q: f64| sorted.get(((sorted.len() as f64 - 1.0) * q).round() as usize).copied().unwrap_or(0.0);
        let stats = self.net.peer_stats();
        let conns: Vec<serde_json::Value> = self
            .conns
            .iter()
            .map(|(p, c)| {
                let s = stats.iter().find(|x| x.0 == *p);
                let secs = c.opened.elapsed().as_secs_f64().max(1e-3);
                let (wi, wo) = s
                    .map(|s| (s.2.wire_in.load(Ordering::Relaxed), s.2.wire_out.load(Ordering::Relaxed)))
                    .unwrap_or((0, 0));
                serde_json::json!({
                    "peer": p,
                    "addr": s.map(|s| s.1.to_string()),
                    "slot": c.slot,
                    "seconds": secs.round(),
                    "stick_loss": c.stick.loss(),
                    "stick_stale": c.stick.stale,
                    "dropped_messages": c.dropped,
                    "wire_in_kbit_s": wi as f64 * 8.0 / 1000.0 / secs,
                    "wire_out_kbit_s": wo as f64 * 8.0 / 1000.0 / secs,
                    "impaired": s.map(|s| s.2.impaired.load(Ordering::Relaxed)),
                    "refused_writes": s.map(|s| s.2.refused_writes.load(Ordering::Relaxed)),
                })
            })
            .collect();
        serde_json::json!({
            "mission": d.data.mission.id,
            "round": d.round,
            "phase": format!("{:?}", d.phase),
            "phase_s": (d.phase_s * 10.0).round() / 10.0,
            "outcome": format!("{:?}", d.outcome),
            "uptime_s": self.started.elapsed().as_secs(),
            "tick_ms": { "p50": pct(0.5), "p99": pct(0.99), "max": pct(1.0) },
            "players": d.players.iter().map(|p| serde_json::json!({
                "slot": p.slot, "name": p.name, "station": p.station.map(|s| s.id()), "ready": p.ready, "bot": p.bot,
            })).collect::<Vec<_>>(),
            "connections": conns,
            "rounds": self.results.iter().map(|r| serde_json::json!({
                "round": r.round, "outcome": r.outcome, "engage_s": r.engage_s.round(), "crew": r.crew, "stats": r.stats,
            })).collect::<Vec<_>>(),
        })
        .to_string()
    }
}

/// A repair order as it travels: the round, its level, whether it opens with the part (all-hands 5).
fn order_bytes(o: sc_core::repair::Order) -> (u8, u8, bool) {
    (o.round.min(255) as u8, o.level.min(255) as u8, o.part)
}
