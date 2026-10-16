//! A client's session with a drill server (openspec/changes/coop-drill design 4-5): the hello, the crew list, the
//! snapshot buffer for interpolation, the round trip, and the automation picture a bot decides from.
//!
//! It lives in `sc-net` so the game client and the headless test client share one implementation of the protocol's
//! client side (CLAUDE.md 6.1). It decides nothing: a bot's decisions come from `sc-core::combat::automation`.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use sc_core::combat::automation::{Hostile, Own, Picture};
use sc_core::combat::chat::{Kind as ChatKind, Line as ChatLine};
use sc_core::combat::{Command, Phase, Refusal, Station, ENEMY_ID};

use crate::link::Link;
use crate::msg::{ClientMsg, CrewEntry, ServerMsg, Snapshot, StickMsg, PROTOCOL};
use crate::transport::{Chan, ClientNet, Dial, Impair, NetEvent};

/// How far behind the newest snapshot remote bodies are drawn (netcode-and-sessions section 3: two intervals).
pub const INTERP_DELAY_S: f64 = 0.1;
/// How quickly the playback clock is pulled toward its goal, seconds: a time constant long beside a frame, so the pace
/// of what is drawn is the frame's own step and only drift is taken out (chase-orbit design 2).
pub const PLAYBACK_SYNC_S: f64 = 1.0;
/// A playback clock further than this from its goal, seconds, jumps to it (the first snapshot, a new drill, a stall).
pub const PLAYBACK_SNAP_S: f64 = 0.5;

/// The clock snapshots are played back by, in server seconds (a snapshot's tick times `combat::DT`), chase-orbit
/// design 2: it runs at the client's frame step and is pulled gently toward the newest snapshot less
/// [`INTERP_DELAY_S`], so two snapshots handed over in one frame, or a film stepped on its own clock, still play back
/// evenly. Snapshots used to be timed by when they arrived, which made the ships step (design 1).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Playback {
    /// Where playback is, server seconds; none until the first advance with a snapshot.
    pub at_s: Option<f64>,
}

impl Playback {
    /// Move on by the frame's `dt` seconds, toward `newest_s`, the newest snapshot's server time.
    pub fn advance(&mut self, dt: f64, newest_s: f64) {
        let goal = newest_s - INTERP_DELAY_S;
        let dt = if dt.is_finite() { dt.max(0.0) } else { 0.0 };
        let at = match self.at_s {
            Some(at) if (goal - at).abs() <= PLAYBACK_SNAP_S => {
                let at = at + dt;
                at + (goal - at) * (1.0 - (-dt / PLAYBACK_SYNC_S).exp())
            }
            _ => goal,
        };
        self.at_s = Some(at.min(newest_s));
    }

    /// The two of `times` (server seconds, oldest first) that bracket the playback clock, and the blend between them;
    /// before the oldest, the oldest; past the newest (or never advanced), the newest.
    pub fn bracket(&self, times: &[f64]) -> Option<(usize, usize, f64)> {
        let last = times.len().checked_sub(1)?;
        let Some(t) = self.at_s else { return Some((last, last, 0.0)) };
        if t >= times[last] {
            return Some((last, last, 0.0));
        }
        if t <= times[0] {
            return Some((0, 0, 0.0));
        }
        let b = times.iter().position(|x| *x > t)?;
        let (ta, tb) = (times[b - 1], times[b]);
        let f = if tb > ta { (t - ta) / (tb - ta) } else { 0.0 };
        Some((b - 1, b, f))
    }
}

/// A visual event the client draws (bolts and explosions are drawn, not sent as state).
#[derive(Clone, Debug, PartialEq)]
pub enum Visual {
    /// A laser bank fired.
    Beam {
        /// The ship.
        owner: u16,
        /// From, system frame.
        from: glam::DVec3,
        /// To.
        to: glam::DVec3,
        /// Whether it struck.
        hit: bool,
    },
    /// A ship threw a countermeasure.
    Decoy(u16),
    /// Something burst aboard a ship: a hit past its armour, a system destroyed, a cascade.
    Burst {
        /// Where, system frame.
        pos: glam::DVec3,
        /// Its size.
        size: f64,
    },
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

/// A repair job's next round as the server gave it: the round, its level, whether it opens with the part.
pub type RepairRound = (u8, u8, bool);

/// The most chat lines a session holds until they are drained (a session nobody drains, a bot's, keeps its newest).
pub const CHAT_HELD: usize = 64;

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
    /// The clock the snapshots are drawn by (chase-orbit design 2).
    playback: Playback,
    snap_link: Link,
    stick_link: Link,
    /// Visual events since the last drain.
    visuals: Vec<Visual>,
    /// The last refusal and when.
    pub refused: Option<(Refusal, f64)>,
    /// Repair orders since the client last took them (all-hands 5): a job, and its next round (round, level, part) or
    /// none when it is done.
    pub repairs: Vec<(u8, Option<RepairRound>)>,
    /// The smoothed round trip, in milliseconds.
    pub rtt_ms: Option<f64>,
    /// Chat lines since the client last took them, oldest first, at most [`CHAT_HELD`] (crew-chat design 1).
    chat: VecDeque<ChatLine>,
    epoch: Instant,
    last_ping: f64,
    /// Messages that failed to decode.
    pub dropped: u64,
    /// When anything last arrived from the server, session seconds.
    heard_s: f64,
}

impl Session {
    /// Connect to `server` (its signalling address, or a local server's [`Dial`]) as `name`, wanting `station`.
    pub fn connect(
        server: impl Into<Dial>,
        name: &str,
        bot: bool,
        station: Option<Station>,
        impair: Impair,
        seed: u64,
    ) -> Result<Self, String> {
        let net = ClientNet::dial(&server.into(), impair, seed)?;
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
            playback: Playback::default(),
            snap_link: Link::default(),
            stick_link: Link::default(),
            visuals: Vec::new(),
            refused: None,
            repairs: Vec::new(),
            rtt_ms: None,
            chat: VecDeque::new(),
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
                    Ok(mut s) => {
                        if self.snap_link.receive(&s.header) {
                            // The power grid rides in some snapshots only: the rest carry the last one on.
                            if s.power.is_none() {
                                s.power = self.snaps.back().and_then(|(_, p)| p.power.clone());
                            }
                            if s.route.is_none() {
                                s.route = self.snaps.back().and_then(|(_, p)| p.route.clone());
                            }
                            // A tick that goes back is a new drill: the old one's snapshots are not played back.
                            if self.snaps.back().is_some_and(|(_, b)| b.tick > s.tick) {
                                self.snaps.clear();
                            }
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
            ServerMsg::Beam { owner, from, to, hit } => self.visuals.push(Visual::Beam { owner, from, to, hit }),
            ServerMsg::Decoy(id) => self.visuals.push(Visual::Decoy(id)),
            ServerMsg::Burst { pos, size, .. } => self.visuals.push(Visual::Burst { pos, size }),
            ServerMsg::Launched { .. } | ServerMsg::Phase(..) => {}
            ServerMsg::Repair { job, next } => self.repairs.push((job, next)),
            ServerMsg::Chat(line) => {
                if self.chat.len() >= CHAT_HELD {
                    self.chat.pop_front();
                }
                self.chat.push_back(line);
            }
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

    /// Chat lines since the last call, oldest first.
    pub fn drain_chat(&mut self) -> Vec<ChatLine> {
        self.chat.drain(..).collect()
    }

    /// Say a line of chat (a player's typing is [`ChatKind::Say`]), cleaned as the server keeps it; an empty line is
    /// not sent. The server stamps who and when.
    pub fn say(&self, kind: ChatKind, text: &str) {
        if let Some(text) = sc_core::combat::chat::clean_text(text) {
            self.net.send(Chan::Command, ClientMsg::Chat { kind, text }.encode());
        }
    }

    /// Visual events since the last call.
    pub fn drain_visuals(&mut self) -> Vec<Visual> {
        std::mem::take(&mut self.visuals)
    }

    /// The newest snapshot.
    pub fn latest(&self) -> Option<&Snapshot> {
        self.snaps.back().map(|s| &s.1)
    }

    /// Move the playback clock on by this frame's step, seconds (the frame's own: a film's fixed step, a game's frame
    /// time). Call once a frame, after [`Session::update`].
    pub fn advance(&mut self, dt: f64) {
        if let Some((_, newest)) = self.snaps.back() {
            self.playback.advance(dt, tick_s(newest));
        }
    }

    /// The two snapshots around the playback clock (chase-orbit design 2: by their server ticks, about
    /// `INTERP_DELAY_S` behind the newest) and the blend between them.
    pub fn interpolation(&self) -> Option<(&Snapshot, &Snapshot, f64)> {
        let times: Vec<f64> = self.snaps.iter().map(|(_, s)| tick_s(s)).collect();
        let (a, b, f) = self.playback.bracket(&times)?;
        Some((&self.snaps[a].1, &self.snaps[b].1, f))
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

    /// The player's walk put their body here (feet, ship frame, metres; facing, radians).
    pub fn walked(&self, pos: [f32; 3], yaw: f32) {
        self.net.send(Chan::Command, ClientMsg::Walk { pos, yaw }.encode());
    }

    /// Get up from the seat.
    pub fn stand(&self) {
        self.net.send(Chan::Command, ClientMsg::Stand.encode());
    }

    /// Take an extinguisher at the damage-control locker.
    pub fn take_extinguisher(&self) {
        self.net.send(Chan::Command, ClientMsg::TakeExtinguisher.encode());
    }

    /// Spray the extinguisher carried, or stop.
    pub fn spray(&self, on: bool) {
        self.net.send(Chan::Command, ClientMsg::Spray(on).encode());
    }

    /// Dock at a repair job.
    pub fn dock(&self, job: u8) {
        self.net.send(Chan::Command, ClientMsg::Dock(job).encode());
    }

    /// A round of a repair job landed, in `play_s` seconds.
    pub fn round_done(&self, job: u8, round: u8, play_s: f32) {
        self.net.send(Chan::Command, ClientMsg::RoundDone { job, round, play_s }.encode());
    }

    /// Stand up from a repair.
    pub fn undock(&self) {
        self.net.send(Chan::Command, ClientMsg::Undock.encode());
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
                hull_mj: t.hull,
            },
            hostile: e.filter(|e| e.active && e.alive).map(|e| Hostile { id: ENEMY_ID, pos: e.pos, vel: e.vel }),
        })
    }
}

/// A snapshot's server time, seconds.
fn tick_s(s: &Snapshot) -> f64 {
    s.tick as f64 * sc_core::combat::DT
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

#[cfg(test)]
mod tests {
    use super::*;
    use sc_core::combat::DT;

    /// Play a server that ticks `ticks_per_frame` (cycled) and snapshots at 20 Hz from 30 ticks, as the solo host does,
    /// drawing each frame by the playback clock; the server time drawn on each frame.
    fn drawn(frame_dt: &[f64], ticks_per_frame: &[u32], frames: usize) -> Vec<f64> {
        let (mut tick, mut acc, mut pb) = (0u64, 0.0f64, Playback::default());
        let mut times: Vec<f64> = Vec::new();
        let mut out = Vec::new();
        for i in 0..frames {
            for _ in 0..ticks_per_frame[i % ticks_per_frame.len()] {
                tick += 1;
                acc += 20.0 * DT;
                if acc >= 1.0 {
                    acc -= 1.0;
                    times.push(tick as f64 * DT);
                }
            }
            let Some(newest) = times.last().copied() else { continue };
            pb.advance(frame_dt[i % frame_dt.len()], newest);
            let (a, b, f) = pb.bracket(&times).unwrap();
            out.push(times[a] + (times[b] - times[a]) * f);
        }
        out
    }

    #[test]
    fn a_film_stepped_a_tick_a_frame_plays_back_in_even_steps() {
        // Regression (chase-orbit design 1): drawn by arrival, the film's heading advanced 6.2, 10.6 and 8.8 mrad on
        // three frames in turn, a third of them showing the newest snapshot unblended.
        let t = drawn(&[DT], &[1], 300);
        for w in t[60..].windows(2) {
            assert!((w[1] - w[0] - DT).abs() < 1e-3, "each frame shows one tick more: {} s", w[1] - w[0]);
        }
    }

    #[test]
    fn frames_that_run_two_ticks_or_none_still_play_back_at_the_frames_pace() {
        // A Pi frame rate (45 a second) over 30 ticks a second: frames run 0, 1 or 2 ticks, and two snapshots can arrive
        // in one frame with one arrival time.
        let dts = [1.0 / 45.0];
        let ticks = [1, 0, 1, 2, 0, 0, 1, 1, 0];
        let t = drawn(&dts, &ticks, 900);
        let steps: Vec<f64> = t[120..].windows(2).map(|w| w[1] - w[0]).collect();
        let worst = steps.iter().map(|s| (s - dts[0]).abs() / dts[0]).fold(0.0, f64::max);
        assert!(worst < 0.1, "the drawn time keeps the frame's step within 10 %: worst {:.0} %", worst * 100.0);
    }

    #[test]
    fn the_playback_clock_stays_behind_the_newest_snapshot_and_jumps_after_a_stall() {
        let mut pb = Playback::default();
        pb.advance(0.016, 10.0);
        assert_eq!(pb.at_s, Some(10.0 - INTERP_DELAY_S), "the first snapshot sets it");
        pb.advance(5.0, 10.0);
        assert!(pb.at_s.unwrap() <= 10.0, "never past the newest snapshot");
        pb.advance(0.016, 100.0);
        assert_eq!(pb.at_s, Some(100.0 - INTERP_DELAY_S), "a new drill's ticks: it jumps");
        assert_eq!(Playback::default().bracket(&[1.0, 2.0]), Some((1, 1, 0.0)), "never advanced: the newest");
    }
}
