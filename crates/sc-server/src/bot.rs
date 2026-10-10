//! A bot crew member (openspec/changes/coop-drill design 3): a client session whose seat is played by
//! `sc-core::combat::automation` at the `bot` profile, deciding from snapshots and sending console commands like a
//! person would.
//!
//! One implementation for the headless `sc-bot`, the in-process test and the game client's `--bot` (CLAUDE.md
//! 6.1). It lives with the server's crate because that crate already links the core and the network, and the
//! client depends on it for this alone.

use sc_core::combat::automation::{self, Memory};
use sc_core::combat::data::{DrillData, Profile};
use sc_core::combat::{Phase, Station};
use sc_net::session::{Session, Stage};

/// A bot playing one station.
pub struct Bot {
    /// The station it plays.
    pub station: Station,
    profile: Profile,
    mem: Memory,
    /// Seconds to read the briefing before READY.
    pub read_s: f64,
    muster_seen: Option<f64>,
    claimed: bool,
}

impl Bot {
    /// A bot for `station`, with the drill's `bot` profile.
    pub fn new(data: &DrillData, station: Station) -> Self {
        Self {
            station,
            profile: data.profile("bot").clone(),
            mem: Memory::default(),
            read_s: data.mission.bot_ready_s,
            muster_seen: None,
            claimed: false,
        }
    }

    /// Decide and send, given the session's newest state. Call every frame after `session.update()`.
    pub fn drive(&mut self, s: &mut Session, data: &DrillData) {
        if s.stage != Stage::Joined {
            return;
        }
        match s.phase() {
            Some(Phase::Muster) => {
                self.mem = Memory::default();
                if s.station() != Some(self.station) {
                    if !self.claimed {
                        s.claim(self.station);
                        self.claimed = true;
                    }
                    return;
                }
                let now = s.now_s();
                let seen = *self.muster_seen.get_or_insert(now);
                if !s.ready() && now - seen >= self.read_s {
                    s.set_ready(true);
                }
            }
            Some(Phase::Engage) => {
                self.muster_seen = None;
                self.claimed = false;
                let Some(pic) = s.picture() else { return };
                let now = s.latest().map(|x| x.phase_s).unwrap_or(0.0);
                let cmds = match self.station {
                    Station::Helm => {
                        automation::helm(&pic, &self.profile.helm, &data.tern_flight, &mut self.mem.helm, now)
                    }
                    Station::Tactical => {
                        automation::tactical(&pic, &self.profile.tactical, data, &mut self.mem.tactical, now)
                    }
                };
                for c in cmds {
                    s.command(c);
                }
            }
            _ => {
                self.muster_seen = None;
                self.claimed = false;
            }
        }
    }
}
