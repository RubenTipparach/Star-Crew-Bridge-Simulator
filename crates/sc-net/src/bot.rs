//! A bot crew member (openspec/changes/coop-drill design 3): a client session whose seat is played by
//! `sc-core::combat::automation` at the `bot` profile, deciding from snapshots and sending console commands like a
//! person would.
//!
//! One implementation for the headless `sc-bot`, the in-process test and the game client's `--bot` (CLAUDE.md
//! 6.1). It lives beside the session it drives; it defines no rule: every decision is the core's automation, and
//! where that automation is passive (Science's scan) it sends the command a player would.

use crate::session::{Session, Stage};
use sc_core::combat::automation::{self, Memory};
use sc_core::combat::data::{DrillData, Profile};
use sc_core::combat::{Bridge, Command, Phase, Station};

/// A bot playing one station.
pub struct Bot {
    /// The station it plays.
    pub station: Station,
    profile: Profile,
    mem: Memory,
    /// Seconds to read the briefing before READY.
    pub read_s: f64,
    muster_seen: Option<f64>,
    claimed_at: f64,
    /// When the bot next decides at a station the core's automation has no function for (its reaction time).
    next_s: f64,
    /// When a hostile was last inside the captain's normal range, for the captain's automation.
    hostile_near_s: f64,
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
            claimed_at: f64::NEG_INFINITY,
            next_s: 0.0,
            hostile_near_s: f64::NEG_INFINITY,
        }
    }

    /// Decide and send, given the session's newest state. Call every frame after `session.update()`.
    pub fn drive(&mut self, s: &mut Session, data: &DrillData) {
        if s.stage != Stage::Joined {
            return;
        }
        // Take the station whenever it is not ours, in any phase: a bot that joins mid-fight takes over from
        // automation at once. A refused claim (a person holds it) is retried every 2 s.
        let now = s.now_s();
        if s.station() != Some(self.station) {
            if now - self.claimed_at >= 2.0 {
                s.claim(self.station);
                self.claimed_at = now;
            }
            return;
        }
        match s.phase() {
            Some(Phase::Muster) => {
                self.mem = Memory::default();
                self.next_s = 0.0;
                self.hostile_near_s = f64::NEG_INFINITY;
                let seen = *self.muster_seen.get_or_insert(now);
                if !s.ready() && now - seen >= self.read_s {
                    s.set_ready(true);
                }
            }
            Some(Phase::Engage) => {
                self.muster_seen = None;
                let Some(pic) = s.picture() else { return };
                let now = s.latest().map(|x| x.phase_s).unwrap_or(0.0);
                let p = &self.profile;
                let mut cmds = match self.station {
                    Station::Helm => automation::helm(&pic, &p.helm, &data.tern_flight, &mut self.mem.helm, now),
                    Station::Tactical => automation::tactical(&pic, &p.tactical, data, &mut self.mem.tactical, now),
                    Station::Captain => {
                        // The captain's auto-condition, from what the snapshot shows.
                        let range = pic.hostile.as_ref().map(|h| (h.pos - pic.own.pos).length());
                        if range.is_some_and(|r| r <= p.captain.normal_beyond_m) {
                            self.hostile_near_s = now;
                        }
                        let red_alert = s.latest().is_some_and(|x| x.bridge.red_alert);
                        let bridge = Bridge { red_alert, hostile_near_s: self.hostile_near_s, ..Bridge::default() };
                        automation::captain(&pic, &bridge, &p.captain, &mut self.mem.captain, now)
                    }
                    Station::Engineering | Station::Science => Vec::new(),
                };
                // The other stations, and every station's orders, at the profile's reaction time.
                let react = match self.station {
                    Station::Helm => p.helm.reaction_s,
                    Station::Tactical => p.tactical.reaction_s,
                    Station::Engineering => p.engineering.reaction_s,
                    Station::Science => p.science.reaction_s,
                    Station::Captain => p.captain.reaction_s,
                };
                if now >= self.next_s {
                    self.next_s = now + react;
                    if let Some(b) = s.latest().map(|x| x.bridge.clone()) {
                        if b.orders.iter().any(|(to, _, done)| *to == self.station && !done) {
                            cmds.push(Command::Ack(self.station));
                        }
                        match self.station {
                            // Science scans the hostile until the scan is done.
                            Station::Science if b.scan < 1.0 && !b.scanning && pic.hostile.is_some() => {
                                cmds.push(Command::Scan(true));
                            }
                            _ => {}
                        }
                    }
                }
                for c in cmds {
                    s.command(c);
                }
            }
            _ => self.muster_seen = None,
        }
    }
}
