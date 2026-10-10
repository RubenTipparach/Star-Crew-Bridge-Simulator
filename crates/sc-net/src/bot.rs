//! A bot crew member (openspec/changes/coop-drill design 3): a client session whose seat is played by
//! `sc-core::combat::automation` at the `bot` profile, deciding from snapshots and sending console commands like a
//! person would.
//!
//! One implementation for the headless `sc-bot`, the in-process test and the game client's `--bot` (CLAUDE.md
//! 6.1). It lives beside the session it drives; it defines no rule: every decision is the core's automation.
//!
//! On foot (design 9, when the snapshot carries bodies) a claim is a walk, and the bot moves when it should: relieved
//! by a player it walks to the most important station no one holds; sat at a less important station while a more
//! important one stands empty, nobody on the way, for `bot_posts.reassign_s`, it gets up and goes there.

use crate::session::{Session, Stage};
use sc_core::combat::automation::{self, Memory};
use sc_core::combat::bodies::Posture;
use sc_core::combat::data::{DrillData, Profile};
use sc_core::combat::{Phase, Station};

/// A bot playing one station.
pub struct Bot {
    /// The station it asked for when it joined.
    pub station: Station,
    profile: Profile,
    posts: Vec<Station>,
    reassign_s: f64,
    empty_since: [Option<f64>; 2],
    playing: Option<Station>,
    mem: Memory,
    /// Seconds to read the briefing before READY.
    pub read_s: f64,
    muster_seen: Option<f64>,
    claimed_at: f64,
}

impl Bot {
    /// A bot for `station`, with the drill's `bot` profile.
    pub fn new(data: &DrillData, station: Station) -> Self {
        Self {
            station,
            profile: data.profile("bot").clone(),
            posts: data.stations.bot_posts.stations(),
            reassign_s: data.stations.bot_posts.reassign_s,
            empty_since: [None; 2],
            playing: None,
            mem: Memory::default(),
            read_s: data.mission.bot_ready_s,
            muster_seen: None,
            claimed_at: f64::NEG_INFINITY,
        }
    }

    /// Decide and send, given the session's newest state. Call every frame after `session.update()`.
    pub fn drive(&mut self, s: &mut Session, data: &DrillData) {
        if s.stage != Stage::Joined {
            return;
        }
        let now = s.now_s();
        let on_foot = s.latest().is_some_and(|x| !x.bodies.is_empty());
        if on_foot {
            if !self.walk(s, now) {
                return;
            }
        } else if s.station() != Some(self.station) {
            // Take the station whenever it is not ours, in any phase: a bot that joins mid-fight takes over from
            // automation at once. A refused claim (a person holds it) is retried every 2 s.
            if now - self.claimed_at >= 2.0 {
                s.claim(self.station);
                self.claimed_at = now;
            }
            return;
        }
        let Some(station) = s.station() else { return };
        if self.playing != Some(station) {
            self.playing = Some(station);
            self.mem = Memory::default();
        }
        match s.phase() {
            Some(Phase::Muster) => {
                self.mem = Memory::default();
                let seen = *self.muster_seen.get_or_insert(now);
                if !s.ready() && now - seen >= self.read_s {
                    s.set_ready(true);
                }
            }
            Some(Phase::Engage) => {
                self.muster_seen = None;
                let Some(pic) = s.picture() else { return };
                let now = s.latest().map(|x| x.phase_s).unwrap_or(0.0);
                let cmds = match station {
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
            _ => self.muster_seen = None,
        }
    }

    /// On foot: decide where to be. Returns whether the bot is seated at a station and should play it.
    fn walk(&mut self, s: &mut Session, now: f64) -> bool {
        let Some(snap) = s.latest() else { return false };
        let me = s.slot;
        let mine = snap.bodies.iter().find(|b| Some(b.slot) == me).copied();
        let still = |p: Posture| p == Posture::Seated || p == Posture::Standing;
        if mine.is_none_or(|b| b.going.is_some() || !still(b.posture)) {
            return false;
        }
        let held = |st: Station| s.crew.iter().any(|c| Some(c.slot) != me && c.station == Some(st));
        let walked = |st: Station| snap.bodies.iter().any(|b| Some(b.slot) != me && b.going == Some(st));
        let free = |st: Station| !held(st) && !walked(st);
        let phase = snap.phase;
        match s.station() {
            Some(cur) => {
                // A more important station empty, nobody on the way, for long enough: get up and go.
                let rank = self.posts.iter().position(|p| *p == cur).unwrap_or(self.posts.len());
                let mut go = None;
                for (i, p) in self.posts.iter().enumerate() {
                    let k = *p as usize;
                    if i < rank && free(*p) {
                        let since = *self.empty_since[k].get_or_insert(now);
                        if go.is_none() && now - since >= self.reassign_s && phase != Phase::Countdown {
                            go = Some(*p);
                        }
                    } else {
                        self.empty_since[k] = None;
                    }
                }
                if let Some(p) = go {
                    if now - self.claimed_at >= 2.0 {
                        s.claim(p);
                        self.claimed_at = now;
                        self.empty_since = [None; 2];
                    }
                    return false;
                }
                true
            }
            None => {
                // Not seated: the station asked for if it is free, else the most important free post.
                let pick = std::iter::once(self.station).chain(self.posts.iter().copied()).find(|p| free(*p));
                if let Some(p) = pick {
                    if now - self.claimed_at >= 2.0 {
                        s.claim(p);
                        self.claimed_at = now;
                    }
                }
                false
            }
        }
    }
}
