//! The server browser's messages and their client (matchmaker design 10.4): one schema read by the matchmaker
//! (`sc-matchmaker`), the server that lists itself and the client that browses and joins (CLAUDE.md 6.1).
//!
//! Everything is JSON over [`crate::http`], with unknown fields refused. An SDP rides as an opaque JSON value: the
//! matchmaker relays it unchanged and never parses it.

use crate::http::{call, Url};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::time::Duration;

/// The header a server's requests carry its host key in.
pub const KEY_HEADER: &str = "x-host-key";
/// The largest SDP (offer or answer) relayed, in bytes of its JSON (design section 6).
pub const MAX_SDP_BYTES: usize = 8 * 1024;
/// The longest session name kept, in characters (as an officer's name).
pub const MAX_SESSION_NAME_CHARS: usize = 32;
/// The longest a server's offer poll is held, in seconds; the client waits a little longer for the answer.
pub const OFFER_POLL_S: u64 = 20;

/// A server's registration: what the list shows of it, and whether it is shown at all.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Register {
    /// The session's name in the list.
    pub name: String,
    /// Shown in the public list; otherwise joined by code only.
    pub listed: bool,
    /// The mission's id (`drill-hound`).
    pub mission: String,
    /// The network protocol version ([`crate::msg::PROTOCOL`]).
    pub protocol: u16,
    /// Seats in all.
    pub seats: u8,
    /// The code it held before, asked back after a restart.
    #[serde(default)]
    pub previous_code: Option<String>,
}

/// The matchmaker's answer to a registration.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Registered {
    /// The listing's id, in the paths of the server's later requests.
    pub id: String,
    /// The join code.
    pub code: String,
    /// How often to heartbeat, in seconds.
    pub heartbeat_s: f64,
}

/// A server's heartbeat: its state as the list shows it.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Heartbeat {
    /// The drill's phase (`Muster`, `Fight`, `Debrief`).
    pub phase: String,
    /// Players aboard (bots included).
    pub crew: u8,
}

/// One row of the public list. It holds no address: a client reaches a server through the relay.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Listing {
    /// The join code.
    pub code: String,
    /// The session's name.
    pub name: String,
    /// The mission's id.
    pub mission: String,
    /// The phase at the last heartbeat.
    pub phase: String,
    /// Players aboard.
    pub crew: u8,
    /// Seats in all.
    pub seats: u8,
    /// The server's protocol version: a client with another cannot join.
    pub protocol: u16,
}

/// The public list.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct List {
    /// Running listed sessions, by name.
    pub servers: Vec<Listing>,
}

/// A client's join: its offer and its protocol.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Join {
    /// The client's protocol version, checked before the offer is relayed.
    pub protocol: u16,
    /// The client's SDP offer, relayed unchanged.
    pub offer: serde_json::Value,
}

/// An offer waiting for a server.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Offer {
    /// The join's id, to answer.
    pub join_id: String,
    /// The client's SDP offer.
    pub offer: serde_json::Value,
}

/// The offers a server's poll collects.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Offers {
    /// Oldest first.
    pub offers: Vec<Offer>,
}

/// A server's answer to one join, or its refusal.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Answer {
    /// The join it answers.
    pub join_id: String,
    /// The server's SDP answer; none when refused.
    #[serde(default)]
    pub answer: Option<serde_json::Value>,
    /// Why the server refused, when it did.
    #[serde(default)]
    pub refused: Option<String>,
}

/// What a refused request says.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Refusal {
    /// The reason, for a player to read.
    pub reason: String,
}

/// Parse a body, refusing unknown fields.
pub fn parse<T: DeserializeOwned>(body: &[u8]) -> Result<T, String> {
    serde_json::from_slice(body).map_err(|e| format!("bad message: {e}"))
}

/// A matchmaker, as its clients reach it, with the STUN servers its joins use (design 10.5).
#[derive(Clone, Debug)]
pub struct Matchmaker {
    /// Its address.
    pub url: Url,
    /// STUN servers (`host:port`) a peer socket asks for its public address before its offer or answer.
    pub stun: Vec<String>,
}

/// `data/matchmaker/client.json`: where the deployed matchmaker is, and the STUN servers.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ClientConfig {
    url: String,
    #[serde(default)]
    stun: Vec<String>,
}

impl Matchmaker {
    /// A matchmaker at `url` (`http://host:port`), with no STUN servers (a LAN test).
    pub fn new(url: &str) -> Result<Self, String> {
        Ok(Self { url: Url::parse(url)?, stun: Vec::new() })
    }

    /// The matchmaker in `root`'s `data/matchmaker/client.json`, or at `url` instead when given, with that file's STUN
    /// servers either way.
    pub fn configured(root: &std::path::Path, url: Option<&str>) -> Result<Self, String> {
        let path = root.join("data/matchmaker/client.json");
        let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let c: ClientConfig = serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
        Ok(Self { url: Url::parse(url.unwrap_or(&c.url))?, stun: c.stun })
    }

    fn request<T: DeserializeOwned>(
        &self,
        method: &str,
        path: &str,
        key: Option<&str>,
        body: &[u8],
        wait: Duration,
    ) -> Result<T, (u16, String)> {
        let headers: Vec<(&str, &str)> = key.map(|k| vec![(KEY_HEADER, k)]).unwrap_or_default();
        let r = call(&self.url, method, path, &headers, body, wait).map_err(|e| (0, e))?;
        if r.status != 200 {
            let reason = parse::<Refusal>(&r.body).map(|f| f.reason).unwrap_or_else(|_| format!("HTTP {}", r.status));
            return Err((r.status, reason));
        }
        parse(&r.body).map_err(|e| (r.status, e))
    }

    /// Register a server; the answer holds its code.
    pub fn register(&self, key: &str, reg: &Register) -> Result<Registered, String> {
        let body = serde_json::to_vec(reg).map_err(|e| e.to_string())?;
        self.request("POST", "/v1/servers", Some(key), &body, Duration::from_secs(10)).map_err(|e| e.1)
    }

    /// Heartbeat a listing. An error's code 404 means the matchmaker no longer knows it (it restarted): register again.
    pub fn heartbeat(&self, key: &str, id: &str, beat: &Heartbeat) -> Result<(), (u16, String)> {
        let body = serde_json::to_vec(beat).map_err(|e| (0, e.to_string()))?;
        self.request::<serde_json::Value>(
            "POST",
            &format!("/v1/servers/{id}/heartbeat"),
            Some(key),
            &body,
            Duration::from_secs(10),
        )
        .map(|_| ())
    }

    /// Wait for offers to a listing (at most [`OFFER_POLL_S`]); none when the poll ran out.
    pub fn offers(&self, key: &str, id: &str) -> Result<Offers, (u16, String)> {
        self.request("GET", &format!("/v1/servers/{id}/offers"), Some(key), &[], Duration::from_secs(OFFER_POLL_S + 10))
    }

    /// Send the answer (or refusal) to a join.
    pub fn answer(&self, key: &str, id: &str, a: &Answer) -> Result<(), (u16, String)> {
        let body = serde_json::to_vec(a).map_err(|e| (0, e.to_string()))?;
        self.request::<serde_json::Value>(
            "POST",
            &format!("/v1/servers/{id}/answers"),
            Some(key),
            &body,
            Duration::from_secs(10),
        )
        .map(|_| ())
    }

    /// Take a listing off the list (a clean stop).
    pub fn unlist(&self, key: &str, id: &str) -> Result<(), String> {
        self.request::<serde_json::Value>(
            "DELETE",
            &format!("/v1/servers/{id}"),
            Some(key),
            &[],
            Duration::from_secs(5),
        )
        .map(|_| ())
        .map_err(|e| e.1)
    }

    /// The public list.
    pub fn list(&self) -> Result<List, String> {
        self.request("GET", "/v1/servers", None, &[], Duration::from_secs(10)).map_err(|e| e.1)
    }

    /// Join the session holding `code` with an offer; the server's answer, or the reason it was refused.
    pub fn join(&self, code: &str, join: &Join) -> Result<serde_json::Value, String> {
        let body = serde_json::to_vec(join).map_err(|e| e.to_string())?;
        let code = normal_code(code);
        self.request::<serde_json::Value>("POST", &format!("/v1/join/{code}"), None, &body, Duration::from_secs(20))
            .map_err(|e| e.1)
    }
}

/// Crockford's base-32 alphabet: digits and letters without I, L, O and U (design section 3).
pub const CODE_ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
/// Characters in a join code.
pub const CODE_LEN: usize = 6;

/// A typed code as the matchmaker keeps it: upper case, with the letters that read as digits read as them
/// (O as 0, I and L as 1), and anything else dropped.
pub fn normal_code(typed: &str) -> String {
    typed
        .chars()
        .filter_map(|c| match c.to_ascii_uppercase() {
            'O' => Some('0'),
            'I' | 'L' => Some('1'),
            c if CODE_ALPHABET.contains(&(c as u8)) && c.is_ascii() => Some(c),
            _ => None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_typed_code_reads_lower_case_and_lookalike_letters_as_the_code() {
        assert_eq!(normal_code("k7m-2qx"), "K7M2QX");
        assert_eq!(normal_code("o1lI"), "0111", "O reads as 0, I and L as 1");
    }

    #[test]
    fn messages_refuse_unknown_fields() {
        let ok = br#"{"phase":"Fight","crew":2}"#;
        assert_eq!(parse::<Heartbeat>(ok).unwrap(), Heartbeat { phase: "Fight".into(), crew: 2 });
        assert!(parse::<Heartbeat>(br#"{"phase":"Fight","crew":2,"extra":1}"#).is_err());
    }
}
