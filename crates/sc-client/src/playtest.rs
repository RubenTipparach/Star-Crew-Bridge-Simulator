//! `--playtest`: a scripted player for a recording of the game as it is played (CLAUDE.md 12: a playtest follows a
//! body through the ship; the law: one game, played on foot).
//!
//! It plays single player the way a person does, through the same body and keys: it walks from where it joined to a
//! station by the walk grid (`sc-core::bots::steer`, the bot crew's own steering, Use at the ladders), presses E to sit
//! at the seat in reach, works the console a while (at Engineering, the schematic in 3D and then the board), holds
//! Stand to get up, walks to the next station or down the decks to a room, and so on. It is a capture instrument
//! (CLAUDE.md 4): it decides nothing a player could not, and the server checks it as it checks a player.

use sc_core::bots::steer;
use sc_core::combat::Station;
use sc_core::nav::Waypoint;
use sc_core::walk::Input;

use crate::onfoot::OnFoot;

/// One beat of the script.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Beat {
    /// Walk to just behind a station's seat.
    To(Station),
    /// Walk to a room's floor, by its layout id.
    Room(&'static str),
    /// Press E: sit at the seat in reach.
    Sit,
    /// Seated: work the console this many seconds.
    Work(f64),
    /// Hold Stand until up.
    Stand,
    /// On foot: look round the room this many seconds.
    Look(f64),
    /// To the damage-control locker for an extinguisher (all-hands 4).
    Locker,
    /// To the nearest fire, and spray it until it is out or the agent is gone.
    Fight,
    /// To the nearest damaged machine, E, and its repair game played to the end (all-hands 5).
    Repair,
}

/// What the player's hands meet this frame (all-hands 4): the locker, an extinguisher carried, a fire.
pub struct Hands<'a> {
    /// Within reach of the damage-control locker.
    pub at_locker: bool,
    /// Carrying an extinguisher with agent left.
    pub carrying: bool,
    /// Standing in a burning room.
    pub in_fire: bool,
    /// The burning rooms' floor points.
    pub fires: &'a [[f32; 3]],
    /// The locker, to walk to.
    pub locker: Option<[f32; 3]>,
    /// The damaged machines with a repair job, to walk to.
    pub jobs: &'a [[f32; 3]],
    /// Within reach of one.
    pub at_job: bool,
    /// Its repair game is open.
    pub repairing: bool,
}

/// What the script does with its hands this frame: E at the locker, F held.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct HandsOut {
    /// Take an extinguisher (E at the locker).
    pub take: bool,
    /// Spray (F held).
    pub spray: bool,
    /// Dock at the machine in reach (E).
    pub repair: bool,
}

/// What the script does this frame, seated.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AtConsole {
    /// Engineering's schematic: open, in 3D, turned this many degrees.
    pub schematic: Option<(bool, f64)>,
    /// Get up now (Stand held its time).
    pub stand: bool,
}

/// The script and where it is.
pub struct Playtest {
    beats: Vec<Beat>,
    k: usize,
    /// Seconds in this beat.
    t: f64,
    plan: Option<Vec<Waypoint>>,
    /// Where the beat walks to, for arriving.
    goal: [f32; 3],
    /// Where the feet were when the walk last made headway, and seconds since; a sidestep's seconds left and way.
    stuck: ([f32; 3], f64, f64, f32),
    /// Local seconds the script has run, and when it last finished fighting a fire (a fire is answered at most once a
    /// minute, so the round's other beats are seen too).
    clock: f64,
    fought_at: f64,
    /// When it last finished a repair, and whether the one in hand has opened.
    repaired_at: f64,
    repair_open: bool,
}

/// A fire is answered at most this often, seconds.
const FIGHT_EVERY_S: f64 = 60.0;
/// The spray is aimed from this far off the fire, metres.
const SPRAY_FROM_M: f32 = 2.2;

/// Behind a seat, where a person stands to sit down, metres.
const BEHIND_M: f32 = 0.8;
/// The beat the tour comes round to again: the first walk away from the player's own station.
const LOOP_FROM: usize = 4;
/// A walk that has not arrived in this long is given up, seconds.
const GIVE_UP_S: f64 = 90.0;
/// A room's middle may stand under machinery: the walk goes to the nearest floor joined to the feet, this near, metres
/// (the server's muster places snap the same way).
const ROOM_REACH_M: f32 = 6.0;
/// Under this much headway in `STUCK_S` is stuck (a door's jamb, a chair): re-plan from here and step aside.
const STUCK_M: f32 = 0.3;
const STUCK_S: f64 = 2.5;

impl Playtest {
    /// The script for a player whose own station is `own`: to it, the console, up, across the bridge to another
    /// station (taking it from its NPC), down the decks to engineering and the medbay, back, and so on, a fight's worth.
    pub fn new(own: Station) -> Self {
        let other = if own == Station::Tactical { Station::Helm } else { Station::Tactical };
        let beats = vec![
            Beat::To(own),
            Beat::Sit,
            Beat::Work(45.0),
            Beat::Stand,
            Beat::To(other),
            Beat::Sit,
            Beat::Work(25.0),
            Beat::Stand,
            Beat::Room("engineering"),
            Beat::Look(6.0),
            Beat::Room("damage_control"),
            Beat::Look(5.0),
            Beat::Room("medbay"),
            Beat::Look(5.0),
            Beat::To(own),
            Beat::Sit,
            Beat::Work(70.0),
            Beat::Stand,
            Beat::To(Station::Science),
            Beat::Sit,
            Beat::Work(20.0),
            Beat::Stand,
            Beat::To(own),
            Beat::Sit,
            Beat::Work(40.0),
            Beat::Stand,
        ];
        Self {
            beats,
            k: 0,
            t: 0.0,
            plan: None,
            goal: [0.0; 3],
            stuck: ([0.0; 3], 0.0, 0.0, 1.0),
            clock: 0.0,
            fought_at: f64::NEG_INFINITY,
            repaired_at: f64::NEG_INFINITY,
            repair_open: false,
        }
    }

    /// The film's script (`--film`, chase-camera 3): to the player's own station, sit, and stay there; no fire gets
    /// it up.
    pub fn film(own: Station) -> Self {
        let mut p = Self::new(own);
        p.beats = vec![Beat::To(own), Beat::Sit, Beat::Work(f64::INFINITY)];
        p.fought_at = f64::INFINITY;
        p
    }

    fn beat(&self) -> Option<Beat> {
        self.beats.get(self.k).copied()
    }

    fn next(&mut self) {
        println!("sc-client: playtest: {:?} done after {:.1} s", self.beat(), self.t);
        self.k += 1;
        // After the first sit, the round goes on: the tour again, as long as the fight lasts.
        if self.k >= self.beats.len() {
            self.k = LOOP_FROM;
        }
        self.t = 0.0;
        self.plan = None;
    }

    /// On foot this frame: the walk's input, the facing turned toward the way (and the eye's tilt), and the station to
    /// sit at (E) when the beat says so. `seats` are the stations' seats; `room` finds a room's floor by id.
    #[allow(clippy::too_many_arguments)]
    pub fn on_foot(
        &mut self,
        foot: &OnFoot,
        seats: &[(Station, [f32; 3], f32)],
        in_reach: Option<Station>,
        room: impl Fn(&str) -> Option<[f32; 3]>,
        hands: &Hands<'_>,
        look: &mut (f32, f32),
        dt: f64,
    ) -> (Input, Option<Station>, HandsOut) {
        self.t += dt;
        self.clock += dt;
        let still = Input { yaw: look.0, ..Input::default() };
        // A fire burning, and none fought for a minute: to the locker (unless carrying), then to the fire.
        let fighting = matches!(self.beat(), Some(Beat::Locker | Beat::Fight | Beat::Repair));
        // A damaged machine and no fire to fight: to it, and repair it (at most one a minute, as the fires).
        if !fighting
            && hands.fires.is_empty()
            && !hands.jobs.is_empty()
            && self.clock - self.repaired_at > FIGHT_EVERY_S
        {
            self.beats.insert(self.k, Beat::Repair);
            self.t = 0.0;
            self.plan = None;
            self.repair_open = false;
        }
        let fighting = matches!(self.beat(), Some(Beat::Locker | Beat::Fight | Beat::Repair));
        if !fighting && !hands.fires.is_empty() && self.clock - self.fought_at > FIGHT_EVERY_S {
            let mut add = vec![Beat::Fight];
            if !hands.carrying {
                add.insert(0, Beat::Locker);
            }
            for (i, b) in add.into_iter().enumerate() {
                self.beats.insert(self.k + i, b);
            }
            self.t = 0.0;
            self.plan = None;
        }
        match self.beat() {
            Some(Beat::Locker) => {
                let Some(l) = hands.locker else {
                    self.next();
                    return (still, None, HandsOut::default());
                };
                if hands.at_locker {
                    self.next();
                    return (still, None, HandsOut { take: true, ..HandsOut::default() });
                }
                self.goal = foot.grid.snap_joined(l, 1.5, foot.body.feet).unwrap_or(l);
                (self.walk(foot, look, dt, |_| false), None, HandsOut::default())
            }
            Some(Beat::Fight) => {
                let feet = foot.body.feet;
                let Some(f) = hands
                    .fires
                    .iter()
                    .min_by(|a, b| {
                        let d = |p: &[f32; 3]| (p[0] - feet[0]).hypot(p[2] - feet[2]) + (p[1] - feet[1]).abs() * 4.0;
                        d(a).total_cmp(&d(b))
                    })
                    .copied()
                else {
                    self.fought_at = self.clock;
                    self.next();
                    return (still, None, HandsOut::default());
                };
                if !hands.carrying && self.t > 1.0 {
                    self.fought_at = self.clock;
                    self.next();
                    return (still, None, HandsOut::default());
                }
                // Where to stand: floor joined to the feet near the fire (a room's middle may be under machinery).
                let stand = if self.plan.is_none() {
                    foot.grid.snap_joined(f, ROOM_REACH_M, feet).unwrap_or(f)
                } else {
                    self.goal
                };
                let arrived = (stand[0] - feet[0]).hypot(stand[2] - feet[2]) < 1.0;
                let near = (f[0] - feet[0]).hypot(f[2] - feet[2]) < SPRAY_FROM_M + 0.6 || arrived;
                if hands.in_fire && near && (f[1] - feet[1]).abs() < 1.0 {
                    // Face the flames, low, and spray.
                    let want = (f[0] - feet[0]).atan2(f[2] - feet[2]);
                    let d =
                        (want - look.0 + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
                    look.0 += d * (dt as f32 * 6.0).min(1.0);
                    look.1 += (-0.25 - look.1) * (dt as f32 * 4.0).min(1.0);
                    return (
                        Input { yaw: look.0, ..Input::default() },
                        None,
                        HandsOut { spray: true, ..HandsOut::default() },
                    );
                }
                self.goal = stand;
                (self.walk(foot, look, dt, |_| false), None, HandsOut::default())
            }
            Some(Beat::Repair) => {
                // The game open: its steady hand plays it (`DeckRun`); stand still until it closes.
                if hands.repairing {
                    self.repair_open = true;
                    return (still, None, HandsOut::default());
                }
                if self.repair_open || self.t > GIVE_UP_S {
                    self.repaired_at = self.clock;
                    self.repair_open = false;
                    self.next();
                    return (still, None, HandsOut::default());
                }
                let feet = foot.body.feet;
                let Some(j) = hands
                    .jobs
                    .iter()
                    .min_by(|a, b| {
                        let d = |p: &[f32; 3]| (p[0] - feet[0]).hypot(p[2] - feet[2]) + (p[1] - feet[1]).abs() * 4.0;
                        d(a).total_cmp(&d(b))
                    })
                    .copied()
                else {
                    self.repaired_at = self.clock;
                    self.next();
                    return (still, None, HandsOut::default());
                };
                if hands.at_job {
                    // E, again every second until the game opens.
                    let press = (self.t % 1.0) < dt;
                    return (still, None, HandsOut { repair: press, ..HandsOut::default() });
                }
                if self.plan.is_none() {
                    self.goal = foot.grid.snap_joined(j, ROOM_REACH_M, feet).unwrap_or(j);
                }
                (self.walk(foot, look, dt, |_| false), None, HandsOut::default())
            }
            other => {
                let (i, sit) = self.on_foot_beat(other, foot, seats, in_reach, room, look, dt);
                (i, sit, HandsOut::default())
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn on_foot_beat(
        &mut self,
        beat: Option<Beat>,
        foot: &OnFoot,
        seats: &[(Station, [f32; 3], f32)],
        in_reach: Option<Station>,
        room: impl Fn(&str) -> Option<[f32; 3]>,
        look: &mut (f32, f32),
        dt: f64,
    ) -> (Input, Option<Station>) {
        let still = Input { yaw: look.0, ..Input::default() };
        match beat {
            Some(Beat::To(st)) => {
                let Some((_, p, yaw)) = seats.iter().find(|s| s.0 == st).copied() else {
                    self.next();
                    return (still, None);
                };
                self.goal = [p[0] - yaw.sin() * BEHIND_M, p[1], p[2] - yaw.cos() * BEHIND_M];
                (self.walk(foot, look, dt, |_| in_reach == Some(st)), None)
            }
            Some(Beat::Room(id)) => {
                let Some(g) = room(id).map(|g| foot.grid.snap_joined(g, ROOM_REACH_M, foot.body.feet).unwrap_or(g))
                else {
                    self.next();
                    return (still, None);
                };
                self.goal = g;
                (self.walk(foot, look, dt, |f| (f[0] - g[0]).hypot(f[2] - g[2]) < 1.2), None)
            }
            Some(Beat::Sit) => {
                // E, again every half second until the body sits (the server may not have the last step yet); the beat
                // ends when it is seated (`at_console`), or is given up.
                let press = (self.t / 0.5).floor() != ((self.t - dt) / 0.5).floor() || self.t <= dt + 1e-9;
                let sit = press.then_some(in_reach).flatten();
                if self.t > 8.0 {
                    self.next();
                }
                (still, sit)
            }
            Some(Beat::Look(secs)) => {
                look.0 += (0.6 * dt) as f32;
                look.1 = -0.05 + 0.1 * (self.t * 0.8).sin() as f32;
                if self.t >= secs {
                    self.next();
                }
                (Input { yaw: look.0, ..Input::default() }, None)
            }
            // Seated beats while on foot: the seat was lost (taken); go on.
            Some(Beat::Work(_) | Beat::Stand | Beat::Locker | Beat::Fight | Beat::Repair) => {
                self.next();
                (still, None)
            }
            None => (still, None),
        }
    }

    /// The walk toward `goal` by the walk grid; the beat ends once `arrived` says so (or it is given up).
    fn walk(&mut self, foot: &OnFoot, look: &mut (f32, f32), dt: f64, arrived: impl Fn([f32; 3]) -> bool) -> Input {
        let feet = foot.body.feet;
        if (self.t / 2.0).floor() != ((self.t - dt) / 2.0).floor() {
            let left = self.plan.as_ref().map_or(0, Vec::len);
            println!(
                "sc-client: playtest: {:?} feet {feet:.2?} goal {:.2?}, {left} waypoints left",
                self.beat(),
                self.goal
            );
        }
        if arrived(feet) || self.t > GIVE_UP_S {
            self.next();
            return Input { yaw: look.0, ..Input::default() };
        }
        // Stuck: a fresh plan from where the feet are, and a step to one side, then the other next time.
        let (at, since, side, way) = self.stuck;
        if (feet[0] - at[0]).hypot(feet[2] - at[2]) > STUCK_M || foot.body.climbing() {
            self.stuck = (feet, 0.0, side - dt, way);
        } else if since + dt > STUCK_S {
            println!("sc-client: playtest: stuck at {feet:.2?}, a fresh plan and a step aside");
            self.stuck = (feet, 0.0, 0.6, -way);
            self.plan = None;
        } else {
            self.stuck = (at, since + dt, side - dt, way);
        }
        if self.stuck.2 > 0.0 {
            return Input { right: self.stuck.3, forward: -0.3, yaw: look.0, ..Input::default() };
        }
        let goal = self.goal;
        let plan = self.plan.get_or_insert_with(|| match foot.grid.path(&foot.world, feet, goal) {
            Some(p) => p,
            None => {
                println!("sc-client: playtest: no path from {feet:.2?} to {goal:.2?}: straight on");
                vec![Waypoint::Walk(goal)]
            }
        });
        if plan.is_empty() {
            // At the plan's end but not yet in reach: the last steps straight on.
            plan.push(Waypoint::Walk(self.goal));
        }
        let input = steer(plan, feet, &mut look.0, foot.world.ladder_reach_m, dt as f32);
        look.1 += (-0.06 - look.1) * (dt as f32 * 3.0).min(1.0);
        input
    }

    /// Seated this frame at `st`: what the console shows, and whether to get up (a fire burning and none fought for a
    /// minute gets it up after a few seconds' work).
    pub fn at_console(&mut self, st: Station, fire: bool, dt: f64) -> AtConsole {
        if self.beat() == Some(Beat::Sit) {
            self.next();
        }
        self.t += dt;
        self.clock += dt;
        let mut out = AtConsole { schematic: None, stand: false };
        if fire
            && self.clock - self.fought_at > FIGHT_EVERY_S
            && matches!(self.beat(), Some(Beat::Work(_)))
            && self.t > 8.0
        {
            self.next();
            out.stand = true;
            return out;
        }
        match self.beat() {
            Some(Beat::Work(secs)) => {
                // At Engineering the grid in 3D, turning, then the board, then the panels; a long sit goes round.
                if st == Station::Engineering {
                    let k = (self.t % 60.0) / 60.0;
                    out.schematic = if k < 0.45 {
                        Some((true, 35.0 * (self.t * 0.12).sin()))
                    } else if k < 0.8 {
                        Some((false, 0.0))
                    } else {
                        None
                    };
                }
                if self.t >= secs {
                    self.next();
                }
            }
            Some(Beat::Stand) => {
                // Stand held its time, then up.
                out.stand = self.t >= 0.6;
                if out.stand {
                    self.next();
                }
            }
            // On-foot beats while seated: get up for them.
            Some(_) => out.stand = true,
            None => {}
        }
        out
    }
}
