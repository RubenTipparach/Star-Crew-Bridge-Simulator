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
}

/// Behind a seat, where a person stands to sit down, metres.
const BEHIND_M: f32 = 0.8;
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
            Beat::Work(f64::INFINITY),
        ];
        Self { beats, k: 0, t: 0.0, plan: None, goal: [0.0; 3], stuck: ([0.0; 3], 0.0, 0.0, 1.0) }
    }

    fn beat(&self) -> Option<Beat> {
        self.beats.get(self.k).copied()
    }

    fn next(&mut self) {
        println!("sc-client: playtest: {:?} done after {:.1} s", self.beat(), self.t);
        self.k += 1;
        self.t = 0.0;
        self.plan = None;
    }

    /// On foot this frame: the walk's input, the facing turned toward the way (and the eye's tilt), and the station to
    /// sit at (E) when the beat says so. `seats` are the stations' seats; `room` finds a room's floor by id.
    pub fn on_foot(
        &mut self,
        foot: &OnFoot,
        seats: &[(Station, [f32; 3], f32)],
        in_reach: Option<Station>,
        room: impl Fn(&str) -> Option<[f32; 3]>,
        look: &mut (f32, f32),
        dt: f64,
    ) -> (Input, Option<Station>) {
        self.t += dt;
        let still = Input { yaw: look.0, ..Input::default() };
        match self.beat() {
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
            Some(Beat::Work(_) | Beat::Stand) => {
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

    /// Seated this frame at `st`: what the console shows, and whether to get up.
    pub fn at_console(&mut self, st: Station, dt: f64) -> AtConsole {
        if self.beat() == Some(Beat::Sit) {
            self.next();
        }
        self.t += dt;
        let mut out = AtConsole { schematic: None, stand: false };
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
