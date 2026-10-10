//! A client's session with a drill server (openspec/changes/coop-drill design 4-5): the hello, the crew list, the
//! snapshot buffer for interpolation, the round trip, and the automation picture a bot decides from.
//!
//! It lives in `sc-net` so the game client and the headless test client share one implementation of the protocol's
//! client side (CLAUDE.md 6.1). It decides nothing: a bot's decisions come from `sc-core::combat::automation`.

use std::collections::VecDeque;
use std::net::SocketAddr;
use std::time::{Duration, Instant};

use sc_core::combat::automation::{Hostile, Own, Picture};
use sc_core::combat::{Command, Phase, Refusal, Station, ENEMY_ID};

use crate::link::Link;
use crate::msg::{ClientMsg, CrewEntry, ServerMsg, Snapshot, StickMsg, PROTOCOL};
use crate::transport::{Chan, ClientNet, Impair, NetEvent};

/// How far behind the newest snapshot remote bodies are drawn (netcode-and-sessions section 3: two intervals).
pub const INTERP_DELAY_S: f64 = 0.1;

/// A visual event the client draws (bolts and explosions are drawn, not sent as state).
#[derive(Clone, Debug, PartialEq)]
pub enum Visual {
    /// A bolt left a turret.
    Fired {
        /// Id.
        bolt: u32,
        /// The ship.
        owner: u16,
        /// Where.
        pos: glam::DVec3,
        /// Velocity.
        vel: glam::DVec3,
    },
    /// A bolt struck.
    Hit {
        /// Id.
        bolt: u32,
        /// Ship struck.
        target: u16,
        /// Face.
        face: u8,
    },
    /// A missile exploded.
    Detonated {
        /// Where.
        pos: glam::DVec3,
        /// Damage done.
        damage_mj: f64,
    },
    /// A ship was destroyed.
    Destroyed(u16),
}

/// Where the session is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    /// Waiting for the channels.
    Connecting,
    /// Hello sent, waiting for Welcome.
    Joining,
    /// In the drill.
    Joined,
    /// The connection ended.
    Closed,
}

/// A client's session.
pub struct Session {
    net: ClientNet,
    /// Where it is.
    pub stage: Stage,
    name: String,
    bot: bool,
    want: Option<Station>,
    /// Our slot, once welcomed.
    pub slot: Option<u8>,
    /// The mission's id, once welcomed.
    pub mission: String,
    /// The crew.
    pub crew: Vec<CrewEntry>,
    /// Snapshots received, newest last, with the local time each arrived.
    snaps: VecDeque<(f64, Snapshot)>,
    snap_link: Link,
    stick_link: Link,
    /// Visual events since the last drain.
    visuals: Vec<Visual>,
    /// The last refusal and when.
    pub refused: Option<(Refusal, f64)>,
    /// The smoothed round trip, in milliseconds.
    pub rtt_ms: Option<f64>,
    epoch: Instant,
    last_ping: f64,
    /// Messages that failed to decode.
    pub dropped: u64,
    /// When anything last arrived from the server, session seconds.
    heard_s: f64,
}

impl Session {
    /// Connect to `server` (its signalling address) as `name`, wanting `station`.
    pub fn connect(
        server: SocketAddr,
        name: &str,
        bot: bool,
        station: Option<Station>,
        impair: Impair,
        seed: u64,
    ) -> Result<Self, String> {
        let net = ClientNet::connect(server, impair, seed)?;
        Ok(Self {
            net,
            stage: Stage::Connecting,
            // A name past the protocol's limit would make the server refuse the hello: shortened here, as the
            // server would keep it ("Engineering officer (bot)" is 25 characters).
            name: sc_core::combat::clean_name(name),
            bot,
            want: station,
            slot: None,
            mission: String::new(),
            crew: Vec::new(),
            snaps: VecDeque::new(),
            snap_link: Link::default(),
            stick_link: Link::default(),
            visuals: Vec::new(),
            refused: None,
            rtt_ms: None,
            epoch: Instant::now(),
            last_ping: -10.0,
            dropped: 0,
            heard_s: 0.0,
        })
    }

    /// Seconds since the session began, the client's clock.
    pub fn now_s(&self) -> f64 {
        self.epoch.elapsed().as_secs_f64()
    }

    /// Take in everything that arrived; send the hello and pings when due.
    pub fn update(&mut self) {
        let now = self.now_s();
        for e in self.net.poll() {
            if matches!(e, NetEvent::Data(..)) {
                self.heard_s = now;
            }
            match e {
                NetEvent::Open(_) => {
                    if self.stage == Stage::Connecting {
                        let hello = ClientMsg::Hello {
                            protocol: PROTOCOL,
                            name: self.name.clone(),
                            bot: self.bot,
                            station: self.want,
                        };
                        self.net.send(Chan::Command, hello.encode());
                        self.stage = Stage::Joining;
                    }
                }
                NetEvent::Closed(_) => self.stage = Stage::Closed,
                NetEvent::Data(_, Chan::Command, d) => match ServerMsg::decode(&d) {
                    Ok(m) => self.on_message(m, now),
                    Err(_) => self.dropped += 1,
                },
                NetEvent::Data(_, Chan::Snapshot, d) => match Snapshot::decode(&d) {
                    Ok(s) => {
                        if self.snap_link.receive(&s.header) {
                            self.snaps.push_back((now, s));
                            while self.snaps.len() > 40 {
                                self.snaps.pop_front();
                            }
                        }
                    }
                    Err(_) => self.dropped += 1,
                },
                NetEvent::Data(_, Chan::Input, _) => self.dropped += 1,
            }
        }
        if self.stage == Stage::Joined && now - self.last_ping >= 1.0 {
            self.last_ping = now;
            let ms = (now * 1000.0) as u32;
            self.net.send(Chan::Command, ClientMsg::Ping(ms).encode());
        }
    }

    fn on_message(&mut self, m: ServerMsg, now: f64) {
        match m {
            ServerMsg::Welcome { slot, mission, .. } => {
                self.slot = Some(slot);
                self.mission = mission;
                self.stage = Stage::Joined;
            }
            ServerMsg::Refused(r) => self.refused = Some((r, now)),
            ServerMsg::Crew(c) => self.crew = c,
            ServerMsg::Fired { bolt, owner, pos, vel } => self.visuals.push(Visual::Fired { bolt, owner, pos, vel }),
            ServerMsg::Hit { bolt, target, face, .. } => self.visuals.push(Visual::Hit { bolt, target, face }),
            ServerMsg::Detonated { pos, damage_mj, .. } => self.visuals.push(Visual::Detonated { pos, damage_mj }),
            ServerMsg::Destroyed(id) => self.visuals.push(Visual::Destroyed(id)),
            ServerMsg::Launched { .. } | ServerMsg::Phase(..) => {}
            ServerMsg::Pong(ms) => {
                let rtt = (now * 1000.0 - f64::from(ms)).max(0.0);
                self.rtt_ms = Some(self.rtt_ms.map_or(rtt, |r| r * 0.8 + rtt * 0.2));
            }
        }
    }

    /// Seconds since anything arrived from the server (or since the session began): a joined session silent for the
    /// server's own 5 s has lost it, however long the transport takes to say so.
    pub fn silent_s(&self) -> f64 {
        self.now_s() - self.heard_s
    }

    /// Visual events since the last call.
    pub fn drain_visuals(&mut self) -> Vec<Visual> {
        std::mem::take(&mut self.visuals)
    }

    /// The newest snapshot.
    pub fn latest(&self) -> Option<&Snapshot> {
        self.snaps.back().map(|s| &s.1)
    }

    /// The two snapshots around the render time (`INTERP_DELAY_S` behind the newest) and the blend between them.
    pub fn interpolation(&self) -> Option<(&Snapshot, &Snapshot, f64)> {
        let (t_new, newest) = self.snaps.back()?;
        let t = self.now_s() - INTERP_DELAY_S;
        if self.snaps.len() < 2 || t >= *t_new {
            return Some((newest, newest, 0.0));
        }
        for w in self.snaps.iter().zip(self.snaps.iter().skip(1)) {
            let ((ta, a), (tb, b)) = w;
            if t >= *ta && t <= *tb {
                let f = if tb > ta { (t - ta) / (tb - ta) } else { 0.0 };
                return Some((a, b, f));
            }
        }
        let (_, oldest) = self.snaps.front()?;
        Some((oldest, oldest, 0.0))
    }

    /// The share of snapshots that never arrived.
    pub fn snapshot_loss(&self) -> f64 {
        self.snap_link.loss()
    }

    /// The transport's counters.
    pub fn stats(&self) -> &crate::transport::PeerStats {
        &self.net.stats
    }

    /// The station this player holds, from the crew list.
    pub fn station(&self) -> Option<Station> {
        let slot = self.slot?;
        self.crew.iter().find(|c| c.slot == slot).and_then(|c| c.station)
    }

    /// Whether this player is marked ready.
    pub fn ready(&self) -> bool {
        self.slot.is_some_and(|s| self.crew.iter().any(|c| c.slot == s && c.ready))
    }

    /// Claim a station.
    pub fn claim(&self, s: Station) {
        self.net.send(Chan::Command, ClientMsg::Claim(s).encode());
    }

    /// Ready, or not.
    pub fn set_ready(&self, r: bool) {
        self.net.send(Chan::Command, ClientMsg::Ready(r).encode());
    }

    /// Send a console command: the stick on the input channel (newest wins), anything else reliably.
    pub fn command(&mut self, c: Command) {
        match c {
            Command::Stick { .. } => {
                let m = StickMsg { header: self.stick_link.next_header(), stick: c };
                self.net.send(Chan::Input, m.encode());
            }
            _ => self.net.send(Chan::Command, ClientMsg::Command(c).encode()),
        }
    }

    /// The phase, from the newest snapshot.
    pub fn phase(&self) -> Option<Phase> {
        self.latest().map(|s| s.phase)
    }

    /// The picture a bot decides from, from the newest snapshot (what the console shows, nothing more).
    pub fn picture(&self) -> Option<Picture> {
        let s = self.latest()?;
        let t = s.ships.first()?;
        let e = s.ships.get(1);
        Some(Picture {
            own: Own {
                pos: t.pos,
                rot: t.rot,
                vel: t.vel,
                rates: t.rates,
                speed_set: t.speed_set,
                helm_mode: t.helm_mode,
                preset: t.preset,
                lock_target: t.lock_target,
                locked: t.lock_target.is_some() && t.lock_frac >= 1.0,
                turret_modes: t.turrets.iter().map(|x| x.mode).collect(),
                tubes: t.tubes.iter().map(|x| x.0).collect(),
                magazine: u32::from(t.magazine),
            },
            hostile: e.filter(|e| e.active && e.alive).map(|e| Hostile { id: ENEMY_ID, pos: e.pos, vel: e.vel }),
        })
    }
}

/// Wait up to `timeout` for `f` to hold, updating the session between tries (tests and tools).
pub fn wait_until(s: &mut Session, timeout: Duration, mut f: impl FnMut(&Session) -> bool) -> bool {
    let end = Instant::now() + timeout;
    while Instant::now() < end {
        s.update();
        if f(s) {
            return true;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    false
}
