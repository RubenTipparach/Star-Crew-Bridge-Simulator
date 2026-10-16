//! `--playtest`: a scripted player for a recording of the game as it is played (CLAUDE.md 12: a playtest follows a
//! body through the ship; the law: one game, played on foot).
//!
//! It plays single player the way a person does, through the same body and keys: it walks from where it joined to a
//! station by the walk grid (`sc-core::bots::steer`, the bot crew's own steering, Use at the ladders), presses E to sit
//! at the seat in reach, works the console a while (at Engineering, the schematic in 3D and then the board), holds
//! the stand key to get up (the key a player holds, through the same hold and its ring: leave-station 2), presses a
//! console's controls where a player's pointer would, walks to the next station or down the decks to a room, and so
//! on. It is a capture instrument (CLAUDE.md 4): it decides nothing a player could not, and the server checks it as it
//! checks a player.

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
    /// Walk to a window and look out of it this many seconds (patrol-mission design 11: the outside as it changes).
    Gaze(Window, f64),
    /// Seated: wait for the fight (Engage), then work this many seconds more.
    Engaged(f64),
    /// Seated: press a control of the console, where a player's pointer would.
    Press(Control),
    /// Walk to the bridge's main viewscreen and look at it this many seconds (captains-viewscreen 3).
    Screen(f64),
}

/// A console control the script presses (the drill finds it on the console as drawn and clicks its middle).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Control {
    /// The captain's SCREEN: take the viewscreen.
    Screen,
    /// The viewscreen's camera pad: TARGET.
    Target,
    /// The viewscreen's CHASE.
    Chase,
}

/// Where a person stands to watch the bridge's main viewscreen (ship frame, metres) and the way they face (radians,
/// 0 the bow): on the deck before the helm and tactical desks, square to the screen (the deck's `viewscreen` view,
/// centred at (0.0, 5.2, 31.5) facing aft), a little over 3 m from it.
pub const MAIN_SCREEN_STAND: ([f32; 3], f32) = ([0.0, 3.5, 28.3], 0.0);

/// A window the patrol's script looks out of: where a person stands in front of it (ship frame, metres) and the way
/// they face (radians about +Y, 0 the bow). Each stands 1.6 m in from its window's centre in the layout; a test holds
/// them to `command_suite.json` and `exterior_windows.json`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Window {
    /// The bridge's forward-port window (`p_bridge_window_p`).
    BridgePort,
    /// The bridge's forward-starboard window (`p_bridge_window_s`).
    BridgeStbd,
    /// The mess's middle window, deck B starboard (`p_mess_window_2`).
    Mess,
}

impl Window {
    /// The portal's id in the layout's patches.
    pub fn id(self) -> &'static str {
        match self {
            Self::BridgePort => "p_bridge_window_p",
            Self::BridgeStbd => "p_bridge_window_s",
            Self::Mess => "p_mess_window_2",
        }
    }

    /// Where to stand, feet on the floor (ship frame, metres), and which way to face (radians).
    pub fn stand(self) -> ([f32; 3], f32) {
        match self {
            Self::BridgePort => ([3.25, 3.5, 29.45], 0.785),
            Self::BridgeStbd => ([-3.25, 3.5, 29.45], -0.785),
            Self::Mess => ([-9.47, 0.0, 12.81], -1.455),
        }
    }
}

/// What the player's hands meet this frame (all-hands 4): the locker, an extinguisher carried, a fire.
pub struct Hands<'a> {
    /// Within reach of an extinguisher to take: a full bracket, or the damage-control locker.
    pub at_locker: bool,
    /// Carrying an extinguisher with agent left.
    pub carrying: bool,
    /// Standing in a burning room.
    pub in_fire: bool,
    /// The burning rooms' floor points.
    pub fires: &'a [[f32; 3]],
    /// The nearest place to take one, to walk to: a full bracket, or the locker.
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
    /// Hold the stand key now (the body stands once it has been held its time).
    pub stand: bool,
    /// Press this control now.
    pub press: Option<Control>,
}

/// The script and where it is.
pub struct Playtest {
    beats: Vec<Beat>,
    /// The beat the script comes round to again once it has run through.
    loop_from: usize,
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
/// How far back from a fire's middle the hands stand to spray it, metres (the cone's reach, fire.extinguisher.reach_m
/// 3.0, less a step).
const STAND_BACK_M: f32 = 2.4;

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
        if own == Station::Captain {
            return Self::on_screen();
        }
        Self::with(Self::drill_beats(own), LOOP_FROM)
    }

    /// The captain's script (captains-viewscreen, leave-station, relieve-bots): to the captain's chair; in the fight,
    /// TARGET and CHASE on his viewscreen, and SCREEN to put it on every screen; up with the stand key; to the bridge's
    /// main viewscreen to watch his picture on it; on to a window; to the helm, taken from its NPC (who swaps into the
    /// captain's chair); then the drill's tour.
    fn on_screen() -> Self {
        let own = Station::Captain;
        let mut beats = vec![
            Beat::To(own),
            Beat::Sit,
            Beat::Engaged(6.0),
            Beat::Press(Control::Target),
            Beat::Work(2.0),
            Beat::Press(Control::Chase),
            Beat::Work(3.0),
            Beat::Press(Control::Screen),
            Beat::Work(6.0),
            Beat::Stand,
            Beat::Screen(14.0),
            Beat::Gaze(Window::BridgeStbd, 5.0),
            Beat::To(Station::Helm),
            Beat::Sit,
            Beat::Work(20.0),
            Beat::Stand,
            Beat::Screen(8.0),
        ];
        let loop_from = beats.len();
        beats.extend(Self::drill_beats(own).into_iter().skip(LOOP_FROM));
        Self::with(beats, loop_from)
    }

    /// The patrol's script (patrol-mission): straight to the station before the drive is up (at the helm, to UNDOCK from
    /// the seat), up again to watch the bay's wall slide past from the bridge's windows, down to the mess's windows,
    /// back to work, up to watch the throat and the station fall away, then the drill's tour as long as the fight lasts.
    pub fn patrol(own: Station) -> Self {
        let mut beats = vec![
            Beat::To(own),
            Beat::Sit,
            // Long enough for the drive to come up and, at the helm, UNDOCK to be pressed from the seat.
            Beat::Work(40.0),
            Beat::Stand,
            Beat::Gaze(Window::BridgeStbd, 10.0),
            Beat::Gaze(Window::BridgePort, 6.0),
            Beat::Room("mess"),
            Beat::Gaze(Window::Mess, 8.0),
            Beat::To(own),
            Beat::Sit,
            Beat::Work(60.0),
            Beat::Stand,
            Beat::Gaze(Window::BridgePort, 8.0),
            Beat::Gaze(Window::BridgeStbd, 8.0),
        ];
        let loop_from = beats.len();
        beats.extend(Self::drill_beats(own).into_iter().skip(LOOP_FROM));
        Self::with(beats, loop_from)
    }

    /// The drill's script: to the player's own station, the console, up, across to another station, the decks, back.
    fn drill_beats(own: Station) -> Vec<Beat> {
        let other = if own == Station::Tactical { Station::Helm } else { Station::Tactical };
        vec![
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
        ]
    }

    fn with(beats: Vec<Beat>, loop_from: usize) -> Self {
        Self {
            beats,
            loop_from,
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
            self.k = self.loop_from;
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
                // One in hand: on to the fire.
                if hands.carrying {
                    self.next();
                    return (still, None, HandsOut::default());
                }
                let Some(l) = hands.locker else {
                    self.next();
                    return (still, None, HandsOut::default());
                };
                // At a bracket or the locker: E, again every half second until one is in hand (the server may not
                // have the last step yet, and takes from where it has the body).
                if hands.at_locker {
                    let take = (self.t / 0.5).floor() != ((self.t - dt) / 0.5).floor();
                    if self.t > GIVE_UP_S {
                        self.next();
                    }
                    return (still, None, HandsOut { take, ..HandsOut::default() });
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
                // Where to stand: back from the fire toward where the body comes from, the cone's reach off it, on floor
                // joined to the feet (a room's middle may be under machinery), so the flames are in front, not round the
                // feet.
                let stand = if self.plan.is_none() {
                    let (dx, dz) = (feet[0] - f[0], feet[2] - f[2]);
                    let l = dx.hypot(dz).max(1e-3);
                    let back = [f[0] + dx / l * STAND_BACK_M, f[1], f[2] + dz / l * STAND_BACK_M];
                    foot.grid.snap_joined(back, ROOM_REACH_M, feet).unwrap_or(f)
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
            Some(Beat::Gaze(w, secs)) => {
                let (at, yaw) = w.stand();
                let g = foot.grid.snap_joined(at, ROOM_REACH_M, foot.body.feet).unwrap_or(at);
                let arrived = (foot.body.feet[0] - g[0]).hypot(foot.body.feet[2] - g[2]) < 0.6;
                if !arrived && self.goal != g {
                    self.goal = g;
                    self.plan = None;
                }
                if !arrived && self.t < GIVE_UP_S {
                    // Walk there first; the beat's clock starts at the window.
                    let input = self.walk(foot, look, dt, |_| false);
                    return (input, None);
                }
                // At the window: turn to face out and look, a slow sweep either side.
                if self.goal != [f32::MAX; 3] {
                    self.goal = [f32::MAX; 3];
                    self.t = 0.0;
                    println!("sc-client: playtest: looking out of {}", w.id());
                }
                let d = (yaw - look.0 + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
                look.0 += d * (dt as f32 * 2.0).min(1.0) + (0.25 * (self.t * 0.5).sin() as f32) * dt as f32;
                look.1 += (-0.02 - look.1) * (dt as f32 * 2.0).min(1.0);
                if self.t >= secs {
                    self.next();
                }
                (Input { yaw: look.0, ..Input::default() }, None)
            }
            Some(Beat::Screen(secs)) => {
                // Walk before the main viewscreen, then face it, level, and watch.
                let (at, yaw) = MAIN_SCREEN_STAND;
                let g = foot.grid.snap_joined(at, ROOM_REACH_M, foot.body.feet).unwrap_or(at);
                let arrived = (foot.body.feet[0] - g[0]).hypot(foot.body.feet[2] - g[2]) < 0.6;
                if !arrived && self.goal != g {
                    self.goal = g;
                    self.plan = None;
                }
                if !arrived && self.t < GIVE_UP_S {
                    return (self.walk(foot, look, dt, |_| false), None);
                }
                if self.goal != [f32::MAX; 3] {
                    self.goal = [f32::MAX; 3];
                    self.t = 0.0;
                    println!("sc-client: playtest: watching the main viewscreen");
                }
                let d = (yaw - look.0 + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
                look.0 += d * (dt as f32 * 3.0).min(1.0);
                look.1 += (0.08 - look.1) * (dt as f32 * 2.0).min(1.0);
                if self.t >= secs {
                    self.next();
                }
                (Input { yaw: look.0, ..Input::default() }, None)
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
            Some(
                Beat::Work(_)
                | Beat::Stand
                | Beat::Engaged(_)
                | Beat::Press(_)
                | Beat::Locker
                | Beat::Fight
                | Beat::Repair,
            ) => {
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
    /// `engaged`: the fight is on (Engage).
    pub fn at_console(&mut self, st: Station, fire: bool, engaged: bool, dt: f64) -> AtConsole {
        if self.beat() == Some(Beat::Sit) {
            self.next();
        }
        self.t += dt;
        self.clock += dt;
        let mut out = AtConsole { schematic: None, stand: false, press: None };
        if fire
            && self.clock - self.fought_at > FIGHT_EVERY_S
            && matches!(self.beat(), Some(Beat::Work(_)))
            && self.t > 8.0
        {
            // Up for the fire: the stand key held until the body is on its feet (the Stand beat ends there).
            if self.beats.get(self.k + 1) != Some(&Beat::Stand) {
                self.beats.insert(self.k + 1, Beat::Stand);
            }
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
            // The stand key held until the body is up; the beat ends on foot.
            Some(Beat::Stand) => out.stand = true,
            Some(Beat::Engaged(secs)) => {
                if !engaged {
                    self.t = 0.0;
                } else if self.t >= secs {
                    self.next();
                }
            }
            Some(Beat::Press(c)) => {
                out.press = Some(c);
                self.next();
            }
            // On-foot beats while seated: get up for them.
            Some(_) => out.stand = true,
            None => {}
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_patrols_windows_stand_in_front_of_the_layouts_windows() {
        // The windows are the layout's patches' portals (command_suite.json, exterior_windows.json): a person stands
        // 1.6 m in from each centre, on its floor, facing out along its normal.
        let mut portals = Vec::new();
        for text in [
            include_str!("../../../data/ships/tern/command_suite.json"),
            include_str!("../../../data/ships/tern/exterior_windows.json"),
        ] {
            let v: serde_json::Value = serde_json::from_str(text).unwrap();
            portals.extend(v["portals"].as_array().cloned().unwrap_or_default());
        }
        for w in [Window::BridgePort, Window::BridgeStbd, Window::Mess] {
            let p = portals.iter().find(|p| p["id"] == w.id()).unwrap_or_else(|| panic!("no portal {}", w.id()));
            let c: Vec<f64> = (0..3).map(|k| p["center_m"][k].as_f64().unwrap()).collect();
            let n: Vec<f64> = (0..3).map(|k| p["normal"][k].as_f64().unwrap()).collect();
            let (at, yaw) = w.stand();
            let want = [c[0] - n[0] * 1.6, c[2] - n[2] * 1.6];
            let off = (f64::from(at[0]) - want[0]).hypot(f64::from(at[2]) - want[1]);
            assert!(off < 0.02, "{} stands {off} m off", w.id());
            assert!((f64::from(yaw) - n[0].atan2(n[2])).abs() < 0.01, "{} faces out", w.id());
        }
    }
}
