//! A bot crew member (openspec/changes/coop-drill design 3): a client session whose seat is played by
//! `sc-core::combat::automation` at the `bot` profile, deciding from snapshots and sending console commands like a
//! person would.
//!
//! One implementation for the headless `sc-bot`, the in-process test and the game client's `--bot` (CLAUDE.md
//! 6.1). It lives beside the session it drives; it defines no rule: every decision is the core's automation, and
//! where that automation is passive (Science's scan) it sends the command a player would.
//!
//! On foot (design 9, when the snapshot carries bodies) a claim is a walk, and the bot moves when it should: relieved
//! by a player it walks to the most important station no one holds; sat at a less important station while a more
//! important one stands empty, nobody on the way, for `bot_posts.reassign_s`, it gets up and goes there.
//!
//! On MANUAL (openspec/changes/single-player design 4) it stays: it walks, sits and readies as ever, and in a round
//! it decides nothing and sends nothing, so the player can play its station through its session.
//!
//! It says what it does (openspec/changes/crew-chat design 3) through the chat a player types in: "At Helm" when it
//! sits, the line of each command it sends by the core's one mapping (`sc-core::combat::chat`), the ack of the
//! captain's order, a bot captain's orders. Never a line for something it did not send; its pace is a `Voice`'s.

use crate::session::{Session, Stage};
use sc_core::combat::automation::{self, Memory};
use sc_core::combat::bodies::Posture;
use sc_core::combat::chat::{self, Said, Voice};
use sc_core::combat::data::{DrillData, Profile};
use sc_core::combat::patrol::{self, PatrolMemory};
use sc_core::combat::seats::{claim_rule, ClaimRule, Occupant};
use sc_core::combat::{may_start, Bridge, Command, Phase, Station};

/// A bot playing one station.
pub struct Bot {
    /// The station it asked for when it joined.
    pub station: Station,
    profile: Profile,
    posts: Vec<Station>,
    reassign_s: f64,
    empty_since: [Option<f64>; Station::ALL.len()],
    playing: Option<Station>,
    mem: Memory,
    /// Seconds to read the briefing before READY.
    pub read_s: f64,
    muster_seen: Option<f64>,
    /// When it last pressed START MISSION (a crew of bots alone), seconds.
    start_asked: f64,
    claimed_at: f64,
    /// When the bot next decides at a station the core's automation has no function for (its reaction time).
    next_s: f64,
    /// When a hostile was last inside the captain's normal range, for the captain's automation.
    hostile_near_s: f64,
    /// MANUAL: seated and silent in a round (single player); AUTO when false.
    pub manual: bool,
    /// What it has to say, and its pace (crew-chat design 4).
    voice: Voice,
    /// A hunting helm has said it is bringing her about onto the hostile.
    hunting: bool,
    /// Its patrol decisions' memory (patrol-mission design 9).
    patrol_mem: PatrolMemory,
}

impl Bot {
    /// A bot for `station`, with the drill's `bot` profile.
    pub fn new(data: &DrillData, station: Station) -> Self {
        Self {
            station,
            profile: data.profile("bot").clone(),
            posts: data.stations.bot_posts.stations(),
            reassign_s: data.stations.bot_posts.reassign_s,
            empty_since: [None; Station::ALL.len()],
            playing: None,
            mem: Memory::default(),
            read_s: data.mission.bot_ready_s,
            muster_seen: None,
            start_asked: f64::NEG_INFINITY,
            claimed_at: f64::NEG_INFINITY,
            next_s: 0.0,
            hostile_near_s: f64::NEG_INFINITY,
            manual: false,
            voice: Voice::default(),
            hunting: false,
            patrol_mem: PatrolMemory::default(),
        }
    }

    /// Decide and send, given the session's newest state, and say what is due. Call every frame after
    /// `session.update()`.
    pub fn drive(&mut self, s: &mut Session, data: &DrillData) {
        self.decide(s, data);
        if s.stage == Stage::Joined {
            if let Some(x) = self.next_line(data, s.now_s()) {
                s.say(x.kind, &x.text);
            }
        }
    }

    /// Offer the line of each command it sends (crew-chat design 3), by the mapping the drill's automation uses; an
    /// ack says the verb of the newest order to it that is not done (`orders`: the snapshot's, newest first).
    pub fn say_for(
        &mut self,
        data: &DrillData,
        station: Station,
        cmds: &[Command],
        orders: &[(Station, u8, bool)],
        now_s: f64,
    ) {
        for c in cmds {
            let said = match c {
                Command::Ack(st) => orders
                    .iter()
                    .find(|(to, _, done)| to == st && !done)
                    .and_then(|(_, verb, _)| chat::said_ack(data, *st, *verb)),
                _ => chat::said_for(data, station, c),
            };
            if let Some(x) = said {
                self.voice.offer(x, now_s, &data.chat.limits);
            }
        }
    }

    /// The line it says now, if one is due (its pace is its voice's).
    pub fn next_line(&mut self, data: &DrillData, now_s: f64) -> Option<Said> {
        self.voice.take(now_s, &data.chat.limits)
    }

    fn decide(&mut self, s: &mut Session, data: &DrillData) {
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
            if let Some(x) = chat::said_seated(data, station) {
                self.voice.offer(x, now, &data.chat.limits);
            }
        }
        match s.phase() {
            Some(Phase::Muster) => {
                self.mem = Memory::default();
                self.patrol_mem = PatrolMemory::default();
                self.next_s = 0.0;
                self.hostile_near_s = f64::NEG_INFINITY;
                let seen = *self.muster_seen.get_or_insert(now);
                if !s.ready() && now - seen >= self.read_s {
                    s.set_ready(true);
                }
                // A crew of bots alone (a networking rig, a capture's `--bot`) has no person to press START MISSION:
                // once every seated member has read the briefing, a bot presses it (start-mission 1 and 2). With a
                // person aboard `may_start` refuses a bot, and it never asks.
                let crew: Vec<(u8, Option<Station>, bool)> =
                    s.crew.iter().map(|c| (c.slot, c.station, c.bot)).collect();
                let all_read = s.crew.iter().filter(|c| c.station.is_some()).all(|c| c.ready);
                let me = s.slot.unwrap_or(u8::MAX);
                if s.ready() && all_read && may_start(&crew, me).is_ok() && now - self.start_asked >= 2.0 {
                    s.start_mission();
                    self.start_asked = now;
                }
            }
            // A patrol under way (patrol-mission design 9): the station's part by the rule the drill's automation uses.
            Some(Phase::Patrol) => {
                self.muster_seen = None;
                if self.manual {
                    self.patrol_mem = PatrolMemory::default();
                    return;
                }
                let cmds = self.patrol_part(s, data, station, now, false);
                self.say_for(data, station, &cmds, &[], now);
                for c in cmds {
                    s.command(c);
                }
            }
            Some(Phase::Engage) => {
                self.muster_seen = None;
                if self.manual {
                    // The player plays this station: nothing of its own, and back on AUTO it decides afresh.
                    self.mem = Memory::default();
                    self.next_s = 0.0;
                    return;
                }
                let Some(pic) = s.picture() else { return };
                let clock = now;
                let now = s.latest().map(|x| x.phase_s).unwrap_or(0.0);
                let p = &self.profile;
                let mut cmds = match station {
                    Station::Helm => {
                        let c = automation::helm(&pic, &p.helm, &data.tern_flight, &mut self.mem.helm, now);
                        // A hunting helm says so once, when it first turns onto the hostile.
                        if pic.hostile.is_none() {
                            self.hunting = false;
                        } else if p.helm.hunt && !c.is_empty() && !self.hunting {
                            self.hunting = true;
                            if let Some(x) = data.chat.line("helm_hunt", &[]) {
                                self.voice.offer(x, clock, &data.chat.limits);
                            }
                        }
                        c
                    }
                    Station::Tactical => automation::tactical(&pic, &p.tactical, data, &mut self.mem.tactical, now),
                    Station::Captain => {
                        // The captain's auto-condition, from what the snapshot shows.
                        let range = pic.hostile.as_ref().map(|h| (h.pos - pic.own.pos).length());
                        if range.is_some_and(|r| r <= p.captain.normal_beyond_m) {
                            self.hostile_near_s = now;
                        }
                        let red_alert = s.latest().is_some_and(|x| x.bridge.red_alert);
                        let scan = s.latest().map_or(0.0, |x| x.bridge.scan);
                        let bridge =
                            Bridge { red_alert, hostile_near_s: self.hostile_near_s, scan, ..Bridge::default() };
                        let mut c = automation::captain(&pic, &bridge, &p.captain, &mut self.mem.captain, now);
                        // A bot captain gives orders (crew-chat design 3); the drill's automation gives none.
                        if let Some(o) = &p.captain.orders {
                            c.extend(automation::captain_orders(&pic, &bridge, data, o, &mut self.mem.captain, now));
                        }
                        c
                    }
                    Station::Engineering | Station::Science => Vec::new(),
                    // A laser station fires its bank by the server's own rule (laser-banks 3).
                    Station::LaserPort | Station::LaserStbd => {
                        let bank = station.laser_bank().map_or(0, usize::from);
                        let l = s.latest().and_then(|x| x.ships.first().and_then(|t| t.lasers.get(bank)).copied());
                        match l {
                            Some(l)
                                if sc_core::combat::laser_should_fire(l.charge, l.bearing, l.hit_chance, &p.lasers) =>
                            {
                                vec![Command::LaserFire(bank as u8)]
                            }
                            _ => Vec::new(),
                        }
                    }
                    // The gunner's turret stays on AUTO under a bot.
                    Station::Gunner => Vec::new(),
                };
                // The other stations, and every station's orders, at the profile's reaction time.
                let react = p.reaction_s(station);
                if now >= self.next_s {
                    self.next_s = now + react;
                    if let Some(b) = s.latest().map(|x| x.bridge.clone()) {
                        if b.orders.iter().any(|(to, _, done)| *to == station && !done) {
                            cmds.push(Command::Ack(station));
                        }
                        match station {
                            // Science scans the hostile until the scan is done.
                            Station::Science if b.scan < 1.0 && !b.scanning && pic.hostile.is_some() => {
                                cmds.push(Command::Scan(true));
                            }
                            _ => {}
                        }
                    }
                }
                // In a patrol the jump drive works in a fight: the station's part of it (patrol-mission 5.3).
                cmds.extend(self.patrol_part(s, data, station, clock, true));
                let orders = s.latest().map(|x| x.bridge.orders.clone()).unwrap_or_default();
                self.say_for(data, station, &cmds, &orders, clock);
                for c in cmds {
                    s.command(c);
                }
            }
            _ => self.muster_seen = None,
        }
    }

    /// The station's patrol commands now (`patrol::decide`), from the newest snapshot; in a fight without the helm's
    /// flying, which the fight's own rule does.
    fn patrol_part(&mut self, s: &Session, data: &DrillData, station: Station, now: f64, fight: bool) -> Vec<Command> {
        let (Some(snap), Some(pd)) = (s.latest(), data.patrol.as_ref()) else { return Vec::new() };
        let captain_manned = s.crew.iter().any(|c| c.station == Some(Station::Captain));
        let Some(pic) = snap.patrol.as_ref().and_then(|p| p.picture(data, &snap.ships, captain_manned)) else {
            return Vec::new();
        };
        let react = self.profile.reaction_s(station);
        let mut cmds = patrol::decide(station, &pic, &pd.block, &mut self.patrol_mem, now, react);
        if fight {
            cmds.retain(|c| !matches!(c, Command::Helm(_) | Command::Stick { .. }));
        }
        cmds
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
        // A seat is free to a bot by the one seat rule (relieve-bots 2): nobody holds it or walks to it.
        let who = |slot: u8| Occupant { slot, bot: s.crew.iter().any(|c| c.slot == slot && c.bot) };
        let holder =
            |st: Station| s.crew.iter().find(|c| Some(c.slot) != me && c.station == Some(st)).map(|c| who(c.slot));
        let walker =
            |st: Station| snap.bodies.iter().find(|b| Some(b.slot) != me && b.going == Some(st)).map(|b| who(b.slot));
        let free = |st: Station| claim_rule(true, holder(st), walker(st)) == ClaimRule::Sit;
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
                        self.empty_since = [None; Station::ALL.len()];
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

#[cfg(test)]
mod tests {
    use super::*;
    use sc_core::combat::chat::Kind;
    use sc_core::combat::TurretMode;

    #[test]
    fn a_bot_that_fires_a_torpedo_says_so() {
        let data = DrillData::shipped();
        let mut bot = Bot::new(&data, Station::Tactical);
        let cmds = [Command::Lock(Some(2)), Command::TurretMode(0, TurretMode::Auto), Command::Fire(0)];
        bot.say_for(&data, Station::Tactical, &cmds, &[], 10.0);
        let x = bot.next_line(&data, 10.0).expect("a line");
        assert_eq!((x.kind, x.text.as_str()), (Kind::Intent, "Torpedo away, tube 1"), "the torpedo outranks the lock");
        assert_eq!(bot.next_line(&data, 10.5), None, "and nothing more inside its gap");
    }

    #[test]
    fn a_bot_acknowledges_the_captains_order_by_its_verb() {
        let data = DrillData::shipped();
        let engage = data.order_verbs(Station::Tactical).iter().position(|v| v[0] == "ENGAGE").unwrap() as u8;
        let mut bot = Bot::new(&data, Station::Tactical);
        bot.say_for(
            &data,
            Station::Tactical,
            &[Command::Ack(Station::Tactical)],
            &[(Station::Tactical, engage, false)],
            1.0,
        );
        let x = bot.next_line(&data, 1.0).expect("an ack");
        assert_eq!((x.kind, x.text.as_str()), (Kind::Ack, "Aye, engaging the Hound corvette"));
    }

    #[test]
    fn a_bot_says_nothing_it_did_not_do() {
        let data = DrillData::shipped();
        let mut bot = Bot::new(&data, Station::Helm);
        let stick = Command::Stick { speed_set_mps: 120.0, yaw: 0.4, pitch: 0.0, roll: 0.0 };
        bot.say_for(&data, Station::Helm, &[stick, Command::Ack(Station::Helm)], &[], 1.0);
        assert_eq!(bot.next_line(&data, 1.0), None, "flying says nothing, and an ack with no order to it has no verb");
    }
}
