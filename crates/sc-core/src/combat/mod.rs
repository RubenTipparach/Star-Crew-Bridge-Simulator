//! The co-op drill's combat: one Tern, one enemy, the stations that fly and fight her, and the drill's phases
//! (openspec/changes/coop-drill design sections 1-2).
//!
//! It lives in the core because the server runs it and the client previews with it (the hit chance, the steering
//! a Target order applies), with no clock, socket or GPU (CLAUDE.md 6.2). It steps at the server's fixed tick in
//! `f64`, in the drill's one system frame; randomness is seeded per purpose (CLAUDE.md 6.4); commands are applied
//! in a stable order. Ship-local axes: +X port, +Y dorsal, +Z bow. A ship's attitude maps ship-local to system.

pub mod attitude;
pub mod automation;
pub mod data;

use crate::replay::ReplayHash;
use crate::rng::Rng;
use data::{Arc, CombatBlock, DrillData, FlightBlock, ShieldBlock};
use glam::{DQuat, DVec3};

/// The server's tick rate, in Hz (engine-stack design section 5).
pub const TICK_HZ: f64 = 30.0;
/// One tick, in seconds.
pub const DT: f64 = 1.0 / TICK_HZ;
/// The most players a drill takes (netcode-and-sessions section 8).
pub const MAX_PLAYERS: usize = 8;
/// The most bolts in flight at once; a turret that would spawn past it does not fire.
pub const MAX_BOLTS: usize = 256;
/// The most missiles in flight at once.
pub const MAX_MISSILES: usize = 16;
/// The Tern's id in every message.
pub const TERN_ID: u16 = 1;
/// The enemy's id.
pub const ENEMY_ID: u16 = 2;
/// The longest player name, in characters (the lobby's).
pub const MAX_NAME_CHARS: usize = 24;

/// A player's name as the drill keeps it: no control characters, at most [`MAX_NAME_CHARS`]. The server applies it
/// when a player joins and a client before it sends its hello, so a long name is shortened, not refused.
pub fn clean_name(name: &str) -> String {
    name.chars().filter(|c| !c.is_control()).take(MAX_NAME_CHARS).collect()
}

/// A bridge station (bridge-stations section 2, the drill's five).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Station {
    /// Flies the ship.
    Helm = 0,
    /// Locks, fires and sets the shields.
    Tactical = 1,
    /// Runs the power grid (not simulated in the drill yet).
    Engineering = 2,
    /// Scans, pings, tunes and balances the shields.
    Science = 3,
    /// The condition, orders and the viewscreen.
    Captain = 4,
}

impl Station {
    /// Every station, in seat order.
    pub const ALL: [Station; 5] =
        [Station::Helm, Station::Tactical, Station::Engineering, Station::Science, Station::Captain];
    /// From a data id (`helm`).
    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|s| s.id() == id)
    }
    /// The data id.
    pub fn id(self) -> &'static str {
        match self {
            Self::Helm => "helm",
            Self::Tactical => "tactical",
            Self::Engineering => "engineering",
            Self::Science => "science",
            Self::Captain => "captain",
        }
    }
    /// What a screen calls it.
    pub fn name(self) -> &'static str {
        match self {
            Self::Helm => "Helm",
            Self::Tactical => "Tactical",
            Self::Engineering => "Engineering",
            Self::Science => "Science",
            Self::Captain => "Captain",
        }
    }
    /// From its wire byte.
    pub fn from_u8(v: u8) -> Option<Self> {
        Self::ALL.get(usize::from(v)).copied()
    }
}

/// The drill's phase (design section 1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    /// Joining, claiming stations, reading the briefing.
    Muster = 0,
    /// The last seconds before the enemy appears.
    Countdown = 1,
    /// The fight.
    Engage = 2,
    /// The result.
    Debrief = 3,
}

impl Phase {
    /// From its wire byte.
    pub fn from_u8(v: u8) -> Option<Self> {
        [Self::Muster, Self::Countdown, Self::Engage, Self::Debrief].get(usize::from(v)).copied()
    }
}

/// How a drill ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// Not ended.
    None = 0,
    /// The enemy was destroyed.
    Victory = 1,
    /// The Tern was lost.
    Defeat = 2,
    /// Time ran out and the enemy withdrew.
    Withdrew = 3,
}

impl Outcome {
    /// From its wire byte.
    pub fn from_u8(v: u8) -> Option<Self> {
        [Self::None, Self::Victory, Self::Defeat, Self::Withdrew].get(usize::from(v)).copied()
    }
}

/// The helm's autopilot (flight-and-navigation section 5; the console's HOLD, COURSE, CHASE, MATCH and EVADE).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HelmMode {
    /// Hold attitude and speed: the stick flies the ship, and at rest the rates come to zero.
    Hold = 0,
    /// Bow on the mission's waypoint.
    Course = 1,
    /// Bow on the designated target (the nearest hostile if none).
    Chase = 2,
    /// The target's course and speed.
    Match = 3,
    /// Jink: lateral and vertical set points thrown every few seconds, the stick still flying.
    Evade = 4,
}

impl HelmMode {
    /// From its wire byte.
    pub fn from_u8(v: u8) -> Option<Self> {
        [Self::Hold, Self::Course, Self::Chase, Self::Match, Self::Evade].get(usize::from(v)).copied()
    }
}

/// A turret's mode (weapons-and-shields section 7; the console's mode box, tapped round in this order).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TurretMode {
    /// Fires at the designated target when it bears and the fire discipline allows.
    Auto = 0,
    /// Fires at the designated target only (the drill has one hostile, so as AUTO).
    Target = 1,
    /// Point defence: fires only at inbound missiles (the drill's enemy carries none).
    Pd = 2,
    /// Holds fire.
    Hold = 3,
}

impl TurretMode {
    /// From its wire byte.
    pub fn from_u8(v: u8) -> Option<Self> {
        [Self::Auto, Self::Target, Self::Pd, Self::Hold].get(usize::from(v)).copied()
    }
    /// The next mode a tap on the mode box gives.
    pub fn next(self) -> Self {
        Self::from_u8((self as u8 + 1) % 4).unwrap_or(Self::Auto)
    }
    /// The word the console shows.
    pub fn word(self) -> &'static str {
        match self {
            Self::Auto => "AUTO",
            Self::Target => "TARGET",
            Self::Pd => "PD",
            Self::Hold => "HOLD",
        }
    }
}

/// A missile tube's state (weapons-and-shields section 6, the subset this drill uses).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TubeState {
    /// Nothing in it.
    Empty = 0,
    /// A missile is going in.
    Loading = 1,
    /// Loaded, arming.
    Arming = 2,
    /// Ready to fire.
    Armed = 3,
}

impl TubeState {
    /// From its wire byte.
    pub fn from_u8(v: u8) -> Option<Self> {
        [Self::Empty, Self::Loading, Self::Arming, Self::Armed].get(usize::from(v)).copied()
    }
}

/// A missile tube.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tube {
    /// Its state.
    pub state: TubeState,
    /// Seconds left in Loading or Arming.
    pub timer_s: f64,
}

/// A turret's state.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Turret {
    /// Its capacitor, in MJ.
    pub capacitor_mj: f64,
    /// Seconds until it may fire again.
    pub cooldown_s: f64,
    /// Where it points, a unit vector in the system frame.
    pub aim: DVec3,
    /// Whether its arc holds the target.
    pub bearing: bool,
    /// Its hit chance at the target now (0-1), the number the console shows.
    pub hit_chance: f64,
    /// Its mode.
    pub mode: TurretMode,
    /// Heat in its sink, MJ.
    pub heat_mj: f64,
    /// Locked out by heat, until the sink cools below its resume share.
    pub cooling: bool,
    /// Seconds since it last fired.
    pub since_fire_s: f64,
}

/// Which side a ship fights for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    /// The crew's.
    Tern,
    /// The enemy's.
    Enemy,
}

/// A ship in the drill.
#[derive(Clone, Debug, PartialEq)]
pub struct Ship {
    /// Its id in every message.
    pub id: u16,
    /// Its side.
    pub side: Side,
    /// In the fight (the enemy appears at Engage).
    pub active: bool,
    /// Not destroyed.
    pub alive: bool,
    /// Position, metres, system frame.
    pub pos: DVec3,
    /// Attitude: ship-local to system.
    pub rot: DQuat,
    /// Velocity, m/s, system frame.
    pub vel: DVec3,
    /// Body rates, rad/s, ship-local (x about +X, y about +Y, z about +Z).
    pub rates: DVec3,
    /// Forward speed set point, m/s.
    pub speed_set: f64,
    /// Lateral (to starboard) and vertical (up) speed set points, m/s: the strafe pad.
    pub strafe: [f64; 2],
    /// Stick [yaw, pitch, roll] in [-1, 1]: yaw to port, pitch nose up, roll port up.
    pub stick: [f64; 3],
    /// The autopilot.
    pub helm_mode: HelmMode,
    /// An attitude order the helm holds, until the stick moves or a pointing mode takes over.
    pub order: Option<DQuat>,
    /// Shield faces, MJ: bow, stern, port, starboard, dorsal, ventral.
    pub faces: [f64; 6],
    /// The shield preset's index.
    pub preset: u8,
    /// Hull left, MJ.
    pub hull: f64,
    /// Turrets, in the order of the ship's data.
    pub turrets: Vec<Turret>,
    /// Missile tubes.
    pub tubes: Vec<Tube>,
    /// Missiles left aboard, not counting the tubes.
    pub magazine: u32,
    /// The designated target.
    pub lock_target: Option<u16>,
    /// Seconds the lock has been building.
    pub lock_s: f64,
}

impl Ship {
    /// The bow, a unit vector in the system frame.
    pub fn forward(&self) -> DVec3 {
        self.rot * DVec3::Z
    }
}

/// A pulse-cannon bolt in flight.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bolt {
    /// Its id in the fired and hit events.
    pub id: u32,
    /// The ship that fired it.
    pub owner: u16,
    /// Position, system frame.
    pub pos: DVec3,
    /// Velocity, system frame.
    pub vel: DVec3,
    /// Seconds left.
    pub life_s: f64,
    /// Damage it delivers, MJ.
    pub damage_mj: f64,
}

/// A missile in flight.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Missile {
    /// Its id.
    pub id: u16,
    /// The ship that fired it.
    pub owner: u16,
    /// Its target.
    pub target: u16,
    /// Position, system frame.
    pub pos: DVec3,
    /// Velocity, system frame.
    pub vel: DVec3,
    /// Seconds since launch.
    pub age_s: f64,
    /// Where it was relative to the target at the end of the last tick.
    pub rel_prev: DVec3,
}

/// Who operates a station.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operator {
    /// Automation (bridge-stations section 3).
    Auto,
    /// A player, by slot.
    Player(u8),
}

/// A player in the drill.
#[derive(Clone, Debug, PartialEq)]
pub struct Player {
    /// The slot, 0-7, stable while connected.
    pub slot: u8,
    /// The officer's name.
    pub name: String,
    /// The station held, if any.
    pub station: Option<Station>,
    /// Ready to begin.
    pub ready: bool,
    /// A bot client.
    pub bot: bool,
}

/// A console command (bridge-stations section 7, the drill's subset).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Command {
    /// Helm: the forward speed set point (m/s) and the stick [yaw, pitch, roll] in [-1, 1].
    Stick {
        /// m/s.
        speed_set_mps: f64,
        /// Yaw to port.
        yaw: f64,
        /// Nose up.
        pitch: f64,
        /// Port up.
        roll: f64,
    },
    /// Helm: the strafe pad's lateral (to starboard) and vertical set points, m/s.
    Strafe {
        /// To starboard.
        lat_mps: f64,
        /// Up.
        vert_mps: f64,
    },
    /// Helm: the autopilot.
    Helm(HelmMode),
    /// Helm: an attitude order [heading, pitch, roll] in degrees (whole degrees are taken), or none to cancel it.
    Orient(Option<[f64; 3]>),
    /// Helm: speed and drift to zero, the autopilot to HOLD.
    AllStop,
    /// Tactical: designate a target, or clear it.
    Lock(Option<u16>),
    /// Tactical: a turret's mode.
    TurretMode(u8, TurretMode),
    /// Tactical: a shield preset by index.
    Preset(u8),
    /// Tactical: load a tube.
    Load(u8),
    /// Tactical: fire a tube.
    Fire(u8),
    /// Science: start or stop scanning the enemy.
    Scan(bool),
    /// Science: an active ping.
    Ping,
    /// Science: the shield's frequency band, 0-3 (A-D).
    Freq(u8),
    /// The captain: red alert on, or stand down.
    Alert(bool),
    /// The captain: brace on or off.
    Brace(bool),
    /// The captain: take the viewscreen with a camera (an index in [`FEEDS`]), or give it back.
    Viewscreen(Option<u8>),
    /// The captain: an order to a station, the verb by its index in that station's verbs (`data/stations.json`).
    Order(Station, u8),
    /// A station acknowledges the captain's newest order to it.
    Ack(Station),
}

/// The viewscreen's cameras, by index (the consoles' camera pad).
pub const FEEDS: [&str; 6] = ["TARGET", "FWD", "CHASE", "PORT", "AFT", "STBD"];

impl Command {
    /// Whether station `s` may issue it. The shield presets are Tactical's and Science's (weapons-and-shields 11).
    pub fn may(&self, s: Station) -> bool {
        match self {
            Self::Stick { .. } | Self::Strafe { .. } | Self::Helm(_) | Self::Orient(_) | Self::AllStop => {
                s == Station::Helm
            }
            Self::Lock(_) | Self::TurretMode(..) | Self::Load(_) | Self::Fire(_) => s == Station::Tactical,
            Self::Preset(_) => matches!(s, Station::Tactical | Station::Science),
            Self::Scan(_) | Self::Ping | Self::Freq(_) => s == Station::Science,
            Self::Alert(_) | Self::Brace(_) | Self::Viewscreen(_) | Self::Order(..) => s == Station::Captain,
            Self::Ack(to) => s == *to,
        }
    }
}

/// Why a request was refused; a console shows the reason.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// The player does not hold the station that issues it.
    NotYourStation = 1,
    /// Another player holds that station.
    StationTaken = 2,
    /// A value out of range or not finite.
    BadValue = 3,
    /// No such tube or preset.
    NoSuch = 4,
    /// The tube is not in a state for that.
    TubeNotReady = 5,
    /// No lock on the target.
    NotLocked = 6,
    /// The target is outside the missile seeker's cone off the bow.
    OutOfCone = 7,
    /// No missiles left.
    MagazineEmpty = 8,
    /// The drill is full.
    Full = 9,
    /// Not during this phase.
    WrongPhase = 10,
}

impl Refusal {
    /// From its wire byte.
    pub fn from_u8(v: u8) -> Option<Self> {
        use Refusal::*;
        [
            NotYourStation,
            StationTaken,
            BadValue,
            NoSuch,
            TubeNotReady,
            NotLocked,
            OutOfCone,
            MagazineEmpty,
            Full,
            WrongPhase,
        ]
        .into_iter()
        .find(|r| *r as u8 == v)
    }
    /// What a console says.
    pub fn text(self) -> &'static str {
        match self {
            Self::NotYourStation => "Not your station",
            Self::StationTaken => "Station taken",
            Self::BadValue => "Bad value",
            Self::NoSuch => "No such control",
            Self::TubeNotReady => "Tube not ready",
            Self::NotLocked => "No lock",
            Self::OutOfCone => "Target off the bow",
            Self::MagazineEmpty => "Magazine empty",
            Self::Full => "Crew is full",
            Self::WrongPhase => "Not now",
        }
    }
}

/// Something that happened in a tick, for the server to send and the tests to read.
#[derive(Clone, Debug, PartialEq)]
pub enum DrillEvent {
    /// A bolt left a turret.
    Fired {
        /// Its id.
        bolt: u32,
        /// The ship.
        owner: u16,
        /// Where, system frame.
        pos: DVec3,
        /// Its velocity.
        vel: DVec3,
    },
    /// A bolt struck a ship.
    Hit {
        /// Its id.
        bolt: u32,
        /// The ship struck.
        target: u16,
        /// The face that took it (0-5).
        face: u8,
        /// Damage, MJ.
        damage_mj: f64,
    },
    /// A missile left a tube.
    Launched {
        /// Its id.
        missile: u16,
        /// The ship.
        owner: u16,
    },
    /// A missile exploded.
    Detonated {
        /// Its id.
        missile: u16,
        /// Where, system frame.
        pos: DVec3,
        /// Damage done, MJ (0 for a self-destruct).
        damage_mj: f64,
    },
    /// A ship was destroyed.
    Destroyed {
        /// The ship.
        ship: u16,
    },
    /// The phase changed.
    Phase {
        /// The new phase.
        phase: Phase,
        /// The outcome so far.
        outcome: Outcome,
    },
    /// A command was refused when it came to be applied.
    Refused {
        /// The player's slot.
        slot: u8,
        /// Why.
        reason: Refusal,
    },
}

/// What a drill counts, for the debrief.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Stats {
    /// Bolts the Tern fired.
    pub tern_shots: u32,
    /// Of those, hits.
    pub tern_hits: u32,
    /// Bolts the enemy fired.
    pub enemy_shots: u32,
    /// Of those, hits.
    pub enemy_hits: u32,
    /// Missiles fired.
    pub missiles_fired: u32,
    /// Missiles that did damage.
    pub missile_hits: u32,
    /// Damage done to the enemy, MJ.
    pub damage_dealt_mj: f64,
    /// Damage the Tern took, MJ.
    pub damage_taken_mj: f64,
    /// Seconds of Engage.
    pub engage_s: f64,
}

/// The chance a bolt passes within `radius_m` of the aim point, for an aim error of `sigma_rad` (a 2D normal, one
/// sigma) at `range_m` (weapons-and-shields section 4). The turrets' fire decision, the automation and the Tactical
/// console all call this one function.
pub fn hit_chance(range_m: f64, sigma_rad: f64, radius_m: f64) -> f64 {
    let sigma_m = (range_m.max(0.0) * sigma_rad).max(1e-6);
    1.0 - (-(radius_m * radius_m) / (2.0 * sigma_m * sigma_m)).exp()
}

/// A gun's aim at a target moving at constant velocity relative to the shooter.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Aim {
    /// The unit direction to fire, system frame.
    pub dir: DVec3,
    /// Seconds to intercept.
    pub time_s: f64,
    /// Distance the bolt flies to intercept, metres.
    pub range_m: f64,
    /// The target's angular rate across the line of sight, deg/s.
    pub angular_rate_deg_s: f64,
}

/// The lead solution: fire along `dir` at `speed_mps` (relative to the shooter) to meet a target `rel` away moving
/// at `vel_rel` relative to the shooter. None when the bolt cannot catch it.
pub fn lead(rel: DVec3, vel_rel: DVec3, speed_mps: f64) -> Option<Aim> {
    let a = vel_rel.length_squared() - speed_mps * speed_mps;
    let b = 2.0 * rel.dot(vel_rel);
    let c = rel.length_squared();
    let t = if a.abs() < 1e-9 {
        if b >= 0.0 {
            return None;
        }
        -c / b
    } else {
        let disc = b * b - 4.0 * a * c;
        if disc < 0.0 {
            return None;
        }
        let s = disc.sqrt();
        let (t1, t2) = ((-b - s) / (2.0 * a), (-b + s) / (2.0 * a));
        let (lo, hi) = (t1.min(t2), t1.max(t2));
        if lo > 0.0 {
            lo
        } else if hi > 0.0 {
            hi
        } else {
            return None;
        }
    };
    let aim_point = rel + vel_rel * t;
    let range = rel.length().max(1e-6);
    let across = vel_rel - rel * (vel_rel.dot(rel) / (range * range));
    Some(Aim {
        dir: aim_point.normalize_or_zero(),
        time_s: t,
        range_m: speed_mps * t,
        angular_rate_deg_s: (across.length() / range).to_degrees(),
    })
}

/// The stick that turns a ship's bow toward `desired` (system frame), wings level with the drill's plane, at
/// `rate_frac` of its rate limits. It flies the slew to that attitude ([`attitude::slew`]): the helm's pointing modes,
/// the enemy's flying and the bot helm all steer with this one rule (CLAUDE.md 6.1).
pub fn steer_toward(rot: DQuat, rates: DVec3, flight: &FlightBlock, desired: DVec3, rate_frac: f64) -> [f64; 3] {
    let plan = attitude::slew(rot, rates, attitude::bow_on(desired), flight);
    attitude::stick_for(plan.w, flight).map(|v| v.clamp(-rate_frac, rate_frac))
}

/// One tick of full-assist flight (coop-drill design 2.1).
pub fn fly(ship: &mut Ship, flight: &FlightBlock, dt: f64) {
    let lim = flight.rate_limit_deg_s.map(f64::to_radians);
    let acc = flight.ang_accel_deg_s2.map(f64::to_radians);
    // Body rates toward the stick's: yaw is about +Y, pitch (nose up) about -X, roll (port up) about +Z.
    let want = DVec3::new(-ship.stick[1] * lim[1], ship.stick[0] * lim[0], ship.stick[2] * lim[2]);
    let accs = DVec3::new(acc[1], acc[0], acc[2]);
    let step = |cur: f64, want: f64, a: f64| cur + ((want - cur) / flight.tau_w_s).clamp(-a, a) * dt;
    ship.rates = DVec3::new(
        step(ship.rates.x, want.x, accs.x),
        step(ship.rates.y, want.y, accs.y),
        step(ship.rates.z, want.z, accs.z),
    );
    ship.rot = (ship.rot * DQuat::from_scaled_axis(ship.rates * dt)).normalize();
    // Linear, full assist (coop-drill design 2.1): forward speed to its set point at the drive's limits, and the
    // velocity across the bow to the strafe pad's lateral and vertical set points with the same tau_v (zero
    // strafe: the assist cancels drift). The RCS's clamp on that is flight-and-navigation's, not yet the drill's.
    let fwd = ship.forward();
    let v_fwd = ship.vel.dot(fwd);
    let side = ship.vel - fwd * v_fwd;
    let set = ship.speed_set.clamp(flight.speed_min_mps, flight.speed_max_mps);
    let sm = flight.strafe_max_mps;
    let want_side = ship.rot * DVec3::new(-ship.strafe[0].clamp(-sm, sm), ship.strafe[1].clamp(-sm, sm), 0.0);
    let dv = ((set - v_fwd) / flight.tau_v_s).clamp(-flight.accel_rev_mps2, flight.accel_fwd_mps2) * dt;
    ship.vel = fwd * (v_fwd + dv) + want_side + (side - want_side) * (-dt / flight.tau_v_s).exp();
    ship.pos += ship.vel * dt;
}

/// Where the helm is told to point (flight-and-navigation 6a and 5): its attitude order, else its pointing mode's
/// attitude (COURSE the waypoint, CHASE the target, MATCH the target's course), else nowhere (the stick flies).
/// The server flies it and the console's navball draws it as a ghost; `target` is the target's position and velocity.
pub fn helm_goal(
    mode: HelmMode,
    order: Option<DQuat>,
    pos: DVec3,
    waypoint: DVec3,
    target: Option<(DVec3, DVec3)>,
) -> Option<DQuat> {
    order.or(match (mode, target) {
        (HelmMode::Course, _) => Some(attitude::bow_on(waypoint - pos)),
        (HelmMode::Chase, Some((p, _))) => Some(attitude::bow_on(p - pos)),
        (HelmMode::Match, Some((_, v))) if v.length() > 1.0 => Some(attitude::bow_on(v)),
        _ => None,
    })
}

/// Where the bow and stern caps end: a point whose normalized direction is within acos(2/3) (48.19 deg) of the long
/// axis is the bow or the stern (weapons-and-shields section 11). Each face is then one sixth of the sphere.
pub const SHIELD_CAP_COS: f64 = 2.0 / 3.0;

/// The face of a point `o` on or about a shield, in ship axes from the shield's centre, for semi-axes `axes`
/// (weapons-and-shields section 11, "The face of a hit"): bow 0, stern 1, port 2, starboard 3, dorsal 4, ventral 5.
/// Divided by the semi-axes, `o` is a direction on the unit sphere: within the caps' angle of the long axis it is the
/// bow or the stern; otherwise the band between them is cut into quarters at the diagonals. The hit resolution, the
/// shield view and its patches all call this one function (CLAUDE.md 6.1).
pub fn shield_face(o: DVec3, axes: DVec3) -> u8 {
    let n = o / axes;
    let l = n.length().max(1e-12);
    if n.z.abs() / l >= SHIELD_CAP_COS {
        return if n.z >= 0.0 { 0 } else { 1 };
    }
    if n.x.abs() >= n.y.abs() {
        if n.x >= 0.0 {
            2
        } else {
            3
        }
    } else if n.y >= 0.0 {
        4
    } else {
        5
    }
}

/// The shield face a point in the system frame strikes on a ship whose shield is `e`.
pub fn face_of(ship: &Ship, e: &data::Ellipsoid, point: DVec3) -> u8 {
    let o = ship.rot.inverse() * (point - ship.pos) - DVec3::from_array(e.centre_m);
    shield_face(o, DVec3::from_array(e.axes_m))
}

/// Whether a turret's arc holds a target in ship-axes direction `d`: above the mount's plane, or no more than
/// `overlap_deg` below it.
pub fn arc_bears(arc: Arc, d: DVec3, overlap_deg: f64) -> bool {
    match arc.facing() {
        None => true,
        Some(f) => d.normalize_or_zero().dot(f).clamp(-1.0, 1.0).asin().to_degrees() >= -overlap_deg,
    }
}

/// The angle between a ship's bow and a target, in degrees, for a ship at `pos` with attitude `rot`.
pub fn off_bow_deg(pos: DVec3, rot: DQuat, target: DVec3) -> f64 {
    let to = (target - pos).normalize_or_zero();
    (rot * DVec3::Z).dot(to).clamp(-1.0, 1.0).acos().to_degrees()
}

/// An order from the captain (bridge-stations 5).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Order {
    /// To whom.
    pub to: Station,
    /// The verb, by its index in that station's verbs.
    pub verb: u8,
    /// Acknowledged.
    pub done: bool,
    /// When it was sent, seconds into Engage.
    pub at_s: f64,
}

/// The bridge's state beyond the ships: the condition, brace, the viewscreen, the orders, and Science's scan, ping
/// and frequency band.
#[derive(Clone, Debug, PartialEq)]
pub struct Bridge {
    /// Red alert.
    pub red_alert: bool,
    pub braced: bool,
    /// The captain's camera on every viewscreen, if taken (an index in [`FEEDS`]).
    pub view: Option<u8>,
    /// The last orders, newest first, at most [`MAX_ORDERS`].
    pub orders: Vec<Order>,
    /// How far Science's scan of the enemy has gone, 0-1.
    pub scan: f64,
    /// Science is scanning (a manned scan).
    pub scanning: bool,
    /// Seconds left of an active ping's ring.
    pub ping_s: f64,
    /// The Tern's shield band, 0-3 (A-D), and the seconds left of a retune.
    pub band: u8,
    pub retune_s: f64,
    /// Who set the shields last: Tactical or Science.
    pub shields_by: Station,
    /// The captain's automation: when a hostile was last within its normal range, seconds into Engage.
    pub hostile_near_s: f64,
}

/// The most orders the bridge keeps.
pub const MAX_ORDERS: usize = 4;

impl Default for Bridge {
    fn default() -> Self {
        Self {
            red_alert: false,
            braced: false,
            view: None,
            orders: Vec::with_capacity(MAX_ORDERS + 1),
            scan: 0.0,
            scanning: false,
            ping_s: 0.0,
            band: 0,
            retune_s: 0.0,
            shields_by: Station::Science,
            hostile_near_s: f64::NEG_INFINITY,
        }
    }
}

/// The co-op drill: the authoritative state the server steps and every client renders.
#[derive(Clone, Debug)]
pub struct Drill {
    /// Its data.
    pub data: DrillData,
    /// The first round's seed; each round's is derived from it.
    pub seed: u64,
    /// Rounds begun, from 0.
    pub round: u32,
    /// Ticks since the server started.
    pub tick: u64,
    /// The phase.
    pub phase: Phase,
    /// Seconds in the phase.
    pub phase_s: f64,
    /// How the round ended.
    pub outcome: Outcome,
    /// The Tern (index 0) and the enemy (index 1).
    pub ships: [Ship; 2],
    /// Bolts in flight.
    pub bolts: Vec<Bolt>,
    /// Missiles in flight.
    pub missiles: Vec<Missile>,
    /// Players, by slot order.
    pub players: Vec<Player>,
    /// This round's numbers.
    pub stats: Stats,
    /// The bridge: condition, orders, viewscreen, scan, ping and band.
    pub bridge: Bridge,
    events: Vec<DrillEvent>,
    queue: Vec<(Station, u64, Option<u8>, Command)>,
    arrivals: u64,
    next_bolt: u32,
    next_missile: u16,
    rng: Rng,
    auto: automation::Memory,
}

fn new_ship(id: u16, side: Side, combat: &CombatBlock, shields: &ShieldBlock, gun_capacitor_mj: f64) -> Ship {
    Ship {
        id,
        side,
        active: side == Side::Tern,
        alive: true,
        pos: DVec3::ZERO,
        rot: DQuat::IDENTITY,
        vel: DVec3::ZERO,
        rates: DVec3::ZERO,
        speed_set: 0.0,
        strafe: [0.0; 2],
        stick: [0.0; 3],
        helm_mode: HelmMode::Hold,
        order: None,
        faces: shields.presets[0].faces_mj,
        preset: 0,
        hull: combat.hull_mj,
        turrets: combat
            .turrets
            .iter()
            .map(|_| Turret {
                capacitor_mj: gun_capacitor_mj,
                cooldown_s: 0.0,
                aim: DVec3::Z,
                bearing: false,
                hit_chance: 0.0,
                mode: TurretMode::Hold,
                heat_mj: 0.0,
                cooling: false,
                since_fire_s: f64::INFINITY,
            })
            .collect(),
        tubes: combat
            .tubes
            .as_ref()
            .map(|t| (0..t.count).map(|_| Tube { state: TubeState::Empty, timer_s: 0.0 }).collect())
            .unwrap_or_default(),
        magazine: combat.tubes.as_ref().map(|t| t.magazine).unwrap_or(0),
        lock_target: None,
        lock_s: 0.0,
    }
}

impl Drill {
    /// A drill in Muster, with the ships at their start.
    pub fn new(data: DrillData, seed: u64) -> Self {
        let tern_cap = data.gun(&data.tern_combat.gun).capacitor_mj;
        let enemy_cap = data.gun(&data.enemy.combat.gun).capacitor_mj;
        let ships = [
            new_ship(TERN_ID, Side::Tern, &data.tern_combat, &data.tern_shields, tern_cap),
            new_ship(ENEMY_ID, Side::Enemy, &data.enemy.combat, &data.enemy.shields, enemy_cap),
        ];
        let mut d = Self {
            data,
            seed,
            round: 0,
            tick: 0,
            phase: Phase::Muster,
            phase_s: 0.0,
            outcome: Outcome::None,
            ships,
            bolts: Vec::with_capacity(MAX_BOLTS),
            missiles: Vec::with_capacity(MAX_MISSILES),
            players: Vec::with_capacity(MAX_PLAYERS),
            stats: Stats::default(),
            bridge: Bridge::default(),
            events: Vec::new(),
            queue: Vec::new(),
            arrivals: 0,
            next_bolt: 1,
            next_missile: 1,
            rng: Rng::for_purpose(seed, 0, "drill"),
            auto: automation::Memory::default(),
        };
        d.reset_round();
        d
    }

    fn round_seed(&self) -> u64 {
        Rng::for_purpose(self.seed, u64::from(self.round), "round").next_u64()
    }

    /// Put the ships at their start for a new round.
    fn reset_round(&mut self) {
        let tern_cap = self.data.gun(&self.data.tern_combat.gun).capacitor_mj;
        let enemy_cap = self.data.gun(&self.data.enemy.combat.gun).capacitor_mj;
        self.ships[0] = new_ship(TERN_ID, Side::Tern, &self.data.tern_combat, &self.data.tern_shields, tern_cap);
        self.ships[1] = new_ship(ENEMY_ID, Side::Enemy, &self.data.enemy.combat, &self.data.enemy.shields, enemy_cap);
        let m = &self.data.mission;
        self.ships[0].speed_set = m.tern_start_speed_mps;
        self.ships[0].vel = DVec3::Z * m.tern_start_speed_mps;
        let start = DVec3::from_array(m.enemy_start_m);
        self.ships[1].pos = start;
        // The enemy starts facing the Tern.
        self.ships[1].rot = DQuat::from_rotation_arc(DVec3::Z, (-start).normalize_or_zero());
        self.ships[1].speed_set = self.data.enemy.ai.approach_speed_mps;
        self.ships[1].vel = self.ships[1].forward() * self.data.enemy.ai.approach_speed_mps;
        self.bolts.clear();
        self.missiles.clear();
        self.stats = Stats::default();
        self.bridge = Bridge::default();
        self.outcome = Outcome::None;
        self.rng = Rng::for_purpose(self.round_seed(), 0, "fire");
        self.auto = automation::Memory::default();
    }

    fn set_phase(&mut self, phase: Phase) {
        self.phase = phase;
        self.phase_s = 0.0;
        self.events.push(DrillEvent::Phase { phase, outcome: self.outcome });
    }

    /// Who operates a station.
    pub fn operator(&self, s: Station) -> Operator {
        self.players.iter().find(|p| p.station == Some(s)).map(|p| Operator::Player(p.slot)).unwrap_or(Operator::Auto)
    }

    /// A player joins; returns their slot.
    pub fn join(&mut self, name: &str, bot: bool) -> Result<u8, Refusal> {
        if self.players.len() >= MAX_PLAYERS {
            return Err(Refusal::Full);
        }
        let slot = (0..MAX_PLAYERS as u8).find(|s| !self.players.iter().any(|p| p.slot == *s)).ok_or(Refusal::Full)?;
        let name = clean_name(name);
        self.players.push(Player { slot, name, station: None, ready: false, bot });
        self.players.sort_by_key(|p| p.slot);
        Ok(slot)
    }

    /// A player leaves; their station returns to automation on the next tick.
    pub fn leave(&mut self, slot: u8) {
        self.players.retain(|p| p.slot != slot);
        self.queue.retain(|q| q.2 != Some(slot));
    }

    fn player_mut(&mut self, slot: u8) -> Option<&mut Player> {
        self.players.iter_mut().find(|p| p.slot == slot)
    }

    /// A player takes a station, if nobody else holds it and the drill uses it.
    pub fn claim(&mut self, slot: u8, s: Station) -> Result<(), Refusal> {
        if !self.data.mission.stations.iter().any(|id| id == s.id()) {
            return Err(Refusal::NoSuch);
        }
        match self.operator(s) {
            Operator::Player(other) if other != slot => return Err(Refusal::StationTaken),
            _ => {}
        }
        let p = self.player_mut(slot).ok_or(Refusal::NotYourStation)?;
        p.station = Some(s);
        Ok(())
    }

    /// A player is ready (or not) to begin.
    pub fn set_ready(&mut self, slot: u8, ready: bool) -> Result<(), Refusal> {
        let p = self.player_mut(slot).ok_or(Refusal::NotYourStation)?;
        p.ready = ready;
        Ok(())
    }

    /// Queue a player's command for the next tick, after checking the station and the values.
    pub fn command(&mut self, slot: u8, cmd: Command) -> Result<(), Refusal> {
        let s = self.players.iter().find(|p| p.slot == slot).and_then(|p| p.station).ok_or(Refusal::NotYourStation)?;
        if !cmd.may(s) {
            return Err(Refusal::NotYourStation);
        }
        validate(&cmd)?;
        self.arrivals += 1;
        self.queue.push((s, self.arrivals, Some(slot), cmd));
        Ok(())
    }

    /// Events since the last drain, oldest first.
    pub fn drain_events(&mut self) -> Vec<DrillEvent> {
        std::mem::take(&mut self.events)
    }

    /// The picture a station's automation decides from, from the Tern's side.
    pub fn picture(&self) -> automation::Picture {
        automation::Picture::from_drill(self)
    }

    /// One tick (design section 2: commands, automation, flight, weapons, projectiles, damage, mission).
    pub fn step(&mut self) {
        self.tick += 1;
        self.phase_s += DT;
        // Commands, by station, then arrival (CLAUDE.md 6.4).
        let mut queue = std::mem::take(&mut self.queue);
        queue.sort_by_key(|q| (q.0, q.1));
        if self.phase == Phase::Engage {
            for (st, _, slot, cmd) in queue {
                if let Err(reason) = self.apply(st, cmd) {
                    if let Some(slot) = slot {
                        self.events.push(DrillEvent::Refused { slot, reason });
                    }
                }
            }
        }
        match self.phase {
            Phase::Muster => {
                let seated: Vec<&Player> = self.players.iter().filter(|p| p.station.is_some()).collect();
                if !seated.is_empty() && seated.iter().all(|p| p.ready) {
                    self.set_phase(Phase::Countdown);
                }
            }
            Phase::Countdown => {
                if !self.players.iter().any(|p| p.station.is_some()) {
                    self.set_phase(Phase::Muster);
                } else if self.phase_s >= self.data.mission.countdown_s {
                    self.ships[1].active = true;
                    self.set_phase(Phase::Engage);
                }
            }
            Phase::Engage => {
                self.run_automation();
                self.bridge_tick();
                self.engage_tick();
                self.stats.engage_s += DT;
                let ended = if !self.ships[1].alive {
                    Some(Outcome::Victory)
                } else if !self.ships[0].alive {
                    Some(Outcome::Defeat)
                } else if self.phase_s >= self.data.mission.engage_limit_s {
                    Some(Outcome::Withdrew)
                } else {
                    None
                };
                if let Some(o) = ended {
                    self.outcome = o;
                    self.set_phase(Phase::Debrief);
                }
            }
            Phase::Debrief => {
                if self.phase_s >= self.data.mission.debrief_s {
                    self.round += 1;
                    self.reset_round();
                    for p in &mut self.players {
                        p.ready = false;
                    }
                    self.set_phase(Phase::Muster);
                }
            }
        }
    }

    fn run_automation(&mut self) {
        let pic = self.picture();
        let t = self.phase_s;
        let science_manned = self.operator(Station::Science) != Operator::Auto;
        for s in Station::ALL {
            if self.operator(s) != Operator::Auto || !self.data.mission.stations.iter().any(|id| id == s.id()) {
                continue;
            }
            let profile = self.data.profile("automation").clone();
            let mut cmds = match s {
                Station::Helm => automation::helm(&pic, &profile.helm, &self.data.tern_flight, &mut self.auto.helm, t),
                Station::Tactical => {
                    let mut c = automation::tactical(&pic, &profile.tactical, &self.data, &mut self.auto.tactical, t);
                    // The shields are Science's when Science is manned (weapons-and-shields 11, "Automation").
                    if science_manned {
                        c.retain(|c| !matches!(c, Command::Preset(_)));
                    }
                    c
                }
                Station::Captain => {
                    automation::captain(&pic, &self.bridge, &profile.captain, &mut self.auto.captain, t)
                }
                // Science's automation is its passive scan (in the tick); Engineering has no grid to run yet.
                Station::Engineering | Station::Science => Vec::new(),
            };
            // Every automation acknowledges its orders once it has had its reaction time.
            let react = match s {
                Station::Helm => profile.helm.reaction_s,
                Station::Tactical => profile.tactical.reaction_s,
                Station::Engineering => profile.engineering.reaction_s,
                Station::Science => profile.science.reaction_s,
                Station::Captain => profile.captain.reaction_s,
            };
            if self.bridge.orders.iter().any(|o| o.to == s && !o.done && t - o.at_s >= react) {
                cmds.push(Command::Ack(s));
            }
            for c in cmds {
                // Automation's commands take the same path as a console's; a refusal is its own business.
                let _ = self.apply(s, c);
            }
        }
    }

    /// The bridge's own clock: Science's scan (manned while scanning, or passive under automation), the ping's ring
    /// and a retune running down.
    fn bridge_tick(&mut self) {
        let enemy_up = self.ships[1].active && self.ships[1].alive;
        let manned = self.operator(Station::Science) != Operator::Auto;
        let scan_s = self.data.tern_combat.sensors.map(|s| s.scan_s);
        let b = &mut self.bridge;
        if let (true, Some(scan_s)) = (enemy_up, scan_s) {
            let rate = if manned {
                if b.scanning {
                    1.0 / scan_s
                } else {
                    0.0
                }
            } else {
                1.0 / (scan_s * self.data.profile("automation").science.scan_factor)
            };
            b.scan = (b.scan + rate * DT).min(1.0);
            if b.scan >= 1.0 {
                b.scanning = false;
            }
        }
        b.ping_s = (b.ping_s - DT).max(0.0);
        b.retune_s = (b.retune_s - DT).max(0.0);
        // When a hostile was last inside the captain's normal range, for the automation's stand-down.
        let near = self.data.profile("automation").captain.normal_beyond_m;
        if enemy_up && (self.ships[1].pos - self.ships[0].pos).length() <= near {
            b.hostile_near_s = self.phase_s;
        }
    }

    /// Apply a validated command from station `from` (the one path for players, bots and automation).
    fn apply(&mut self, from: Station, cmd: Command) -> Result<(), Refusal> {
        validate(&cmd)?;
        if let Some(r) = self.apply_bridge(cmd) {
            return r;
        }
        let lock_time = self.data.tern_combat.lock.time_s;
        let enemy_pos = self.ships[1].pos;
        let enemy_alive = self.ships[1].alive && self.ships[1].active;
        let tern = &mut self.ships[0];
        match cmd {
            Command::Stick { speed_set_mps, yaw, pitch, roll } => {
                tern.speed_set =
                    speed_set_mps.clamp(self.data.tern_flight.speed_min_mps, self.data.tern_flight.speed_max_mps);
                // The stick sets rates, and cancels an attitude order and a pointing mode (FN 6a); EVADE jinks on.
                if yaw.abs() > 0.05 || pitch.abs() > 0.05 || roll.abs() > 0.05 {
                    tern.order = None;
                    if tern.helm_mode != HelmMode::Evade {
                        tern.helm_mode = HelmMode::Hold;
                    }
                }
                tern.stick = [yaw, pitch, roll];
            }
            Command::Strafe { lat_mps, vert_mps } => {
                let m = self.data.tern_flight.strafe_max_mps;
                tern.strafe = [lat_mps.clamp(-m, m), vert_mps.clamp(-m, m)];
            }
            Command::Helm(m) => {
                if tern.helm_mode == HelmMode::Evade && m != HelmMode::Evade {
                    tern.strafe = [0.0; 2];
                }
                tern.helm_mode = m;
                if !matches!(m, HelmMode::Hold | HelmMode::Evade) {
                    tern.order = None;
                }
            }
            Command::Orient(o) => {
                tern.order = o.map(|[h, p, r]| {
                    attitude::from_hpr(
                        attitude::whole_deg('h', h),
                        attitude::whole_deg('p', p),
                        attitude::whole_deg('r', r),
                    )
                });
                if tern.order.is_some() && tern.helm_mode != HelmMode::Evade {
                    tern.helm_mode = HelmMode::Hold;
                }
            }
            Command::AllStop => {
                tern.speed_set = 0.0;
                tern.strafe = [0.0; 2];
                tern.helm_mode = HelmMode::Hold;
            }
            Command::Lock(t) => {
                if t.is_some_and(|id| id != ENEMY_ID || !enemy_alive) {
                    return Err(Refusal::NoSuch);
                }
                if tern.lock_target != t {
                    tern.lock_target = t;
                    tern.lock_s = 0.0;
                }
            }
            Command::TurretMode(i, m) => {
                tern.turrets.get_mut(usize::from(i)).ok_or(Refusal::NoSuch)?.mode = m;
            }
            Command::Preset(i) => {
                let p = self.data.tern_shields.presets.get(usize::from(i)).ok_or(Refusal::NoSuch)?;
                tern.preset = i;
                for (f, cap) in tern.faces.iter_mut().zip(p.faces_mj) {
                    *f = f.min(cap);
                }
                self.bridge.shields_by = from;
            }
            Command::Load(i) => {
                let tubes = self.data.tern_combat.tubes.as_ref().ok_or(Refusal::NoSuch)?;
                let tube = tern.tubes.get_mut(usize::from(i)).ok_or(Refusal::NoSuch)?;
                if tube.state != TubeState::Empty {
                    return Err(Refusal::TubeNotReady);
                }
                if tern.magazine == 0 {
                    return Err(Refusal::MagazineEmpty);
                }
                tern.magazine -= 1;
                *tube = Tube { state: TubeState::Loading, timer_s: tubes.load_s };
            }
            Command::Fire(i) => {
                let tubes = self.data.tern_combat.tubes.clone().ok_or(Refusal::NoSuch)?;
                let missile = self.data.missile(&tubes.missile).clone();
                let tube = *tern.tubes.get(usize::from(i)).ok_or(Refusal::NoSuch)?;
                if tube.state != TubeState::Armed {
                    return Err(Refusal::TubeNotReady);
                }
                if tern.lock_target != Some(ENEMY_ID) || tern.lock_s < lock_time || !enemy_alive {
                    return Err(Refusal::NotLocked);
                }
                if off_bow_deg(tern.pos, tern.rot, enemy_pos) > missile.seeker_half_angle_deg {
                    return Err(Refusal::OutOfCone);
                }
                if self.missiles.len() >= MAX_MISSILES {
                    return Err(Refusal::TubeNotReady);
                }
                tern.tubes[usize::from(i)] = Tube { state: TubeState::Empty, timer_s: 0.0 };
                let muzzle = tern.pos + tern.rot * DVec3::from_array(tubes.muzzle_m[usize::from(i)]);
                let vel = tern.vel + tern.forward() * missile.eject_mps;
                let id = self.next_missile;
                self.next_missile = self.next_missile.wrapping_add(1).max(1);
                self.missiles.push(Missile {
                    id,
                    owner: TERN_ID,
                    target: ENEMY_ID,
                    pos: muzzle,
                    vel,
                    age_s: 0.0,
                    rel_prev: muzzle - enemy_pos,
                });
                self.stats.missiles_fired += 1;
                self.events.push(DrillEvent::Launched { missile: id, owner: TERN_ID });
            }
            _ => {}
        }
        Ok(())
    }

    /// Science's and the captain's commands, which act on the bridge rather than the ship; none for the rest.
    fn apply_bridge(&mut self, cmd: Command) -> Option<Result<(), Refusal>> {
        let enemy_up = self.ships[1].active && self.ships[1].alive;
        let b = &mut self.bridge;
        let r = match cmd {
            Command::Scan(on) => {
                b.scanning = on && enemy_up && b.scan < 1.0;
                Ok(())
            }
            Command::Ping => match self.data.tern_combat.sensors {
                Some(s) => {
                    b.ping_s = s.ping_s;
                    Ok(())
                }
                None => Err(Refusal::NoSuch),
            },
            Command::Freq(band) => match self.data.tern_shields.frequency {
                Some(f) => {
                    if band != b.band {
                        b.band = band;
                        b.retune_s = f.retune_s;
                    }
                    Ok(())
                }
                None => Err(Refusal::NoSuch),
            },
            Command::Alert(on) => {
                b.red_alert = on;
                Ok(())
            }
            Command::Brace(on) => {
                b.braced = on;
                Ok(())
            }
            Command::Viewscreen(v) => {
                b.view = v;
                Ok(())
            }
            Command::Order(to, verb) => {
                if usize::from(verb) >= self.data.order_verbs(to).len() {
                    Err(Refusal::NoSuch)
                } else {
                    b.orders.insert(0, Order { to, verb, done: false, at_s: self.phase_s });
                    b.orders.truncate(MAX_ORDERS);
                    Ok(())
                }
            }
            Command::Ack(s) => {
                if let Some(o) = b.orders.iter_mut().find(|o| o.to == s && !o.done) {
                    o.done = true;
                }
                Ok(())
            }
            _ => return None,
        };
        Some(r)
    }

    fn enemy_ai(&mut self) {
        let (tern, enemy) = (&self.ships[0], &self.ships[1]);
        if !enemy.active || !enemy.alive {
            return;
        }
        let ai = &self.data.enemy.ai;
        let r = tern.pos - enemy.pos;
        let range = r.length().max(1.0);
        let rh = r / range;
        let (desired, speed) = if range > ai.orbit_m * 1.3 {
            (rh, ai.approach_speed_mps)
        } else {
            let axis = DQuat::from_rotation_x(ai.orbit_tilt_deg.to_radians()) * DVec3::Y;
            let tangent = rh.cross(axis).normalize_or(DVec3::X);
            let radial = (range - ai.orbit_m) / ai.orbit_m;
            ((tangent + rh * (radial * 2.0)).normalize_or_zero(), ai.orbit_speed_mps)
        };
        let stick = steer_toward(enemy.rot, enemy.rates, &self.data.enemy.flight, desired, 1.0);
        let e = &mut self.ships[1];
        e.stick = stick;
        e.speed_set = speed;
        e.lock_target = Some(TERN_ID);
        e.lock_s = self.data.enemy.combat.lock.time_s;
        for t in &mut e.turrets {
            t.mode = TurretMode::Target;
        }
    }

    /// The Tern's autopilot and attitude order: where the helm is told to point, and the set points a mode drives.
    fn helm_tick(&mut self) {
        let (enemy_pos, enemy_vel, enemy_up) =
            (self.ships[1].pos, self.ships[1].vel, self.ships[1].alive && self.ships[1].active);
        let flight = self.data.tern_flight.clone();
        let waypoint = DVec3::from_array(self.data.mission.waypoint_m);
        let k = (self.phase_s / flight.jink_period_s).floor() as u64;
        let jink = {
            let mut rng = Rng::for_purpose(self.round_seed(), k, "jink");
            [(rng.next_f64() - 0.5) * 2.0 * flight.jink_mps, (rng.next_f64() - 0.5) * 2.0 * flight.jink_mps]
        };
        let tern = &mut self.ships[0];
        let goal =
            helm_goal(tern.helm_mode, tern.order, tern.pos, waypoint, enemy_up.then_some((enemy_pos, enemy_vel)));
        if let Some(g) = goal {
            tern.stick = attitude::stick_for(attitude::slew(tern.rot, tern.rates, g, &flight).w, &flight);
        }
        if tern.helm_mode == HelmMode::Match && enemy_up {
            tern.speed_set = enemy_vel.length().clamp(flight.speed_min_mps, flight.speed_max_mps);
        }
        if tern.helm_mode == HelmMode::Evade {
            let m = flight.strafe_max_mps;
            tern.strafe = [jink[0].clamp(-m, m), jink[1].clamp(-m, m)];
        }
    }

    fn engage_tick(&mut self) {
        self.helm_tick();
        self.enemy_ai();
        let tf = self.data.tern_flight.clone();
        let ef = self.data.enemy.flight.clone();
        if self.ships[0].alive {
            fly(&mut self.ships[0], &tf, DT);
        }
        if self.ships[1].alive && self.ships[1].active {
            fly(&mut self.ships[1], &ef, DT);
        }
        self.locks_and_tubes();
        self.shields();
        for (shooter, target) in [(0usize, 1usize), (1, 0)] {
            self.turrets(shooter, target);
        }
        self.bolts_step();
        self.missiles_step();
    }

    fn locks_and_tubes(&mut self) {
        let enemy = (self.ships[1].pos, self.ships[1].alive && self.ships[1].active);
        let lock = self.data.tern_combat.lock.clone();
        let arm_s = self.data.tern_combat.tubes.as_ref().map(|t| t.arm_s).unwrap_or(0.0);
        let tern = &mut self.ships[0];
        if tern.lock_target == Some(ENEMY_ID) {
            if enemy.1 && (enemy.0 - tern.pos).length() <= lock.range_m {
                tern.lock_s = (tern.lock_s + DT).min(lock.time_s);
            } else {
                tern.lock_s = 0.0;
            }
        }
        for t in &mut tern.tubes {
            match t.state {
                TubeState::Loading | TubeState::Arming => {
                    t.timer_s -= DT;
                    if t.timer_s <= 0.0 {
                        *t = if t.state == TubeState::Loading {
                            Tube { state: TubeState::Arming, timer_s: arm_s }
                        } else {
                            Tube { state: TubeState::Armed, timer_s: 0.0 }
                        };
                    }
                }
                TubeState::Empty | TubeState::Armed => {}
            }
        }
    }

    fn shields(&mut self) {
        for (i, ship) in self.ships.iter_mut().enumerate() {
            let block = if i == 0 { &self.data.tern_shields } else { &self.data.enemy.shields };
            let caps = block.presets[usize::from(ship.preset).min(block.presets.len() - 1)].faces_mj;
            let total: f64 = caps.iter().sum::<f64>().max(1e-9);
            for (f, cap) in ship.faces.iter_mut().zip(caps) {
                *f = (*f + block.regen_mj_s * cap / total * DT).min(cap);
            }
        }
    }

    fn turrets(&mut self, si: usize, ti: usize) {
        let combat = if si == 0 { self.data.tern_combat.clone() } else { self.data.enemy.combat.clone() };
        let gun = self.data.gun(&combat.gun).clone();
        let target_radius =
            if ti == 0 { self.data.tern_combat.hit_radius_m } else { self.data.enemy.combat.hit_radius_m };
        let (shooter, target) = (&self.ships[si], &self.ships[ti]);
        // AUTO and TARGET fire at the designated target once it is locked; PD only at inbound missiles, and the drill's
        // enemy carries none; HOLD never.
        let can_fire = shooter.alive
            && shooter.active
            && target.alive
            && target.active
            && shooter.lock_target == Some(target.id)
            && shooter.lock_s >= combat.lock.time_s;
        let (s_pos, s_rot, s_vel, t_pos, t_vel) = (shooter.pos, shooter.rot, shooter.vel, target.pos, target.vel);
        let mut fired = Vec::new();
        for (k, mount) in combat.turrets.iter().enumerate() {
            let turret = &mut self.ships[si].turrets[k];
            turret.capacitor_mj = (turret.capacitor_mj + gun.charge_mw * DT).min(gun.capacitor_mj);
            turret.cooldown_s = (turret.cooldown_s - DT).max(-DT);
            turret.since_fire_s += DT;
            // The sink sheds into the coolant; full, it locks the turret out until it cools to its resume share.
            turret.heat_mj = (turret.heat_mj - gun.heat_shed_mw * DT).max(0.0);
            if turret.cooling && turret.heat_mj < gun.heat_resume_frac * gun.heat_sink_mj {
                turret.cooling = false;
            }
            let muzzle = s_pos + s_rot * DVec3::from_array(mount.position_m);
            turret.bearing = arc_bears(mount.arc, s_rot.inverse() * (t_pos - muzzle), combat.arc_overlap_deg);
            let Some(aim) = lead(t_pos - muzzle, t_vel - s_vel, gun.speed_mps) else {
                turret.hit_chance = 0.0;
                continue;
            };
            let sigma_rad = (combat.aim_sigma_base_deg + gun.sigma_rate_k * aim.angular_rate_deg_s).to_radians();
            let p = if aim.time_s <= gun.life_s { hit_chance(aim.range_m, sigma_rad, target_radius) } else { 0.0 };
            turret.aim = aim.dir;
            turret.hit_chance = p;
            let mode_fires = matches!(turret.mode, TurretMode::Auto | TurretMode::Target);
            if can_fire
                && mode_fires
                && !turret.cooling
                && turret.bearing
                && p >= gun.min_hit_chance
                && turret.capacitor_mj >= gun.draw_mj
                && turret.cooldown_s <= 0.0
                && self.bolts.len() + fired.len() < MAX_BOLTS
            {
                turret.capacitor_mj -= gun.draw_mj;
                turret.cooldown_s += 1.0 / gun.rate_hz;
                turret.since_fire_s = 0.0;
                turret.heat_mj += gun.heat_per_bolt_mj;
                if turret.heat_mj >= gun.heat_sink_mj {
                    turret.cooling = true;
                }
                fired.push((muzzle, aim.dir, sigma_rad));
            }
        }
        for (muzzle, dir, sigma) in fired {
            let d = scatter(&mut self.rng, dir, sigma);
            let id = self.next_bolt;
            self.next_bolt = self.next_bolt.wrapping_add(1).max(1);
            let bolt = Bolt {
                id,
                owner: self.ships[si].id,
                pos: muzzle,
                vel: s_vel + d * gun.speed_mps,
                life_s: gun.life_s,
                damage_mj: gun.damage_mj,
            };
            self.events.push(DrillEvent::Fired { bolt: id, owner: bolt.owner, pos: bolt.pos, vel: bolt.vel });
            if si == 0 {
                self.stats.tern_shots += 1;
            } else {
                self.stats.enemy_shots += 1;
            }
            self.bolts.push(bolt);
        }
    }

    /// A hit of `dmg` MJ at `point` on ship `ti` from a weapon on `band` (a missile has none): its face absorbs what it
    /// can and the hull takes the rest. On the Tern the shield's band scales what the face loses (weapons-and-shields
    /// 11: `match_factor` when the bands match, `retune_factor` while retuning); what it cannot absorb reaches the
    /// hull at the hit's own size.
    fn damage(&mut self, ti: usize, point: DVec3, dmg: f64, band: Option<u8>) -> u8 {
        let e = if ti == 0 { self.data.tern_shields.ellipsoid } else { self.data.enemy.shields.ellipsoid };
        let k = match (ti, self.data.tern_shields.frequency) {
            (0, Some(f)) if self.bridge.retune_s > 0.0 => f.retune_factor,
            (0, Some(f)) if band == Some(self.bridge.band) => f.match_factor,
            _ => 1.0,
        };
        let ship = &mut self.ships[ti];
        let face = face_of(ship, &e, point);
        let absorbed = ship.faces[usize::from(face)].min(dmg * k);
        ship.faces[usize::from(face)] -= absorbed;
        ship.hull -= dmg - absorbed / k;
        if ti == 1 {
            self.stats.damage_dealt_mj += dmg;
        } else {
            self.stats.damage_taken_mj += dmg;
        }
        if ship.hull <= 0.0 && ship.alive {
            ship.hull = 0.0;
            ship.alive = false;
            self.events.push(DrillEvent::Destroyed { ship: ship.id });
        }
        face
    }

    fn bolts_step(&mut self) {
        let radii = [self.data.tern_combat.hit_radius_m, self.data.enemy.combat.hit_radius_m];
        let mut i = 0;
        while i < self.bolts.len() {
            let b = self.bolts[i];
            let ti = if b.owner == TERN_ID { 1 } else { 0 };
            let t = &self.ships[ti];
            let mut hit = None;
            if t.alive && t.active {
                let p0 = b.pos - t.pos;
                let d = (b.vel - t.vel) * DT;
                let u = if d.length_squared() > 0.0 { (-p0.dot(d) / d.length_squared()).clamp(0.0, 1.0) } else { 0.0 };
                let closest = p0 + d * u;
                if closest.length() <= radii[ti] {
                    hit = Some(t.pos + closest);
                }
            }
            if let Some(point) = hit {
                let band = if b.owner == TERN_ID { &self.data.tern_combat.gun } else { &self.data.enemy.combat.gun };
                let band = Some(self.data.gun(band).band_index());
                let face = self.damage(ti, point, b.damage_mj, band);
                if ti == 1 {
                    self.stats.tern_hits += 1;
                } else {
                    self.stats.enemy_hits += 1;
                }
                self.events.push(DrillEvent::Hit {
                    bolt: b.id,
                    target: self.ships[ti].id,
                    face,
                    damage_mj: b.damage_mj,
                });
                self.bolts.swap_remove(i);
                continue;
            }
            let b = &mut self.bolts[i];
            b.pos += b.vel * DT;
            b.life_s -= DT;
            if b.life_s <= 0.0 {
                self.bolts.swap_remove(i);
            } else {
                i += 1;
            }
        }
    }

    fn missiles_step(&mut self) {
        let Some(tubes) = self.data.tern_combat.tubes.clone() else { return };
        let md = self.data.missile(&tubes.missile).clone();
        let mut i = 0;
        while i < self.missiles.len() {
            let target = &self.ships[1];
            let m = &mut self.missiles[i];
            m.age_s += DT;
            let motor = if m.age_s < md.boost_s {
                md.boost_mps2
            } else if m.age_s < md.boost_s + md.sustain_s {
                md.sustain_mps2
            } else {
                0.0
            };
            let along = m.vel.normalize_or(DVec3::Z);
            let mut acc = along * motor;
            let los = target.pos - m.pos;
            let seeing = target.alive
                && target.active
                && along.dot(los.normalize_or_zero()).clamp(-1.0, 1.0).acos().to_degrees() <= md.seeker_half_angle_deg;
            if seeing {
                // Proportional navigation: a = N Vc (Omega x los), limited to the motor's (or the sustain's) push.
                let r2 = los.length_squared().max(1.0);
                let v_rel = target.vel - m.vel;
                let omega = los.cross(v_rel) / r2;
                let closing = -los.dot(v_rel) / r2.sqrt();
                let lat = (omega.cross(los.normalize_or_zero())) * (md.nav_gain * closing);
                acc += lat.clamp_length_max(motor.max(md.sustain_mps2));
            }
            m.vel += acc * DT;
            m.pos += m.vel * DT;
            let rel_now = m.pos - target.pos;
            let seg = rel_now - m.rel_prev;
            let u = if seg.length_squared() > 0.0 {
                (-m.rel_prev.dot(seg) / seg.length_squared()).clamp(0.0, 1.0)
            } else {
                1.0
            };
            let closest = (m.rel_prev + seg * u).length();
            m.rel_prev = rel_now;
            let (id, pos, age) = (m.id, m.pos, m.age_s);
            let burst = target.alive && target.active && closest <= md.blast_radius_m && (u < 1.0 || closest < 2.0);
            if burst {
                let dmg = md.warhead_mj * (1.0 - closest / md.blast_radius_m);
                let point = target.pos + (pos - target.pos).normalize_or_zero() * closest;
                self.damage(1, point, dmg, None);
                self.stats.missile_hits += 1;
                self.events.push(DrillEvent::Detonated { missile: id, pos, damage_mj: dmg });
                self.missiles.swap_remove(i);
            } else if age >= md.life_s || !target.alive {
                self.events.push(DrillEvent::Detonated { missile: id, pos, damage_mj: 0.0 });
                self.missiles.swap_remove(i);
            } else {
                i += 1;
            }
        }
    }

    /// The replay hash of the drill's state (CLAUDE.md 6.4).
    pub fn hash(&self) -> u64 {
        let mut h = ReplayHash::default();
        h.u64(self.tick).u64(self.phase as u64).u64(u64::from(self.round)).f64(self.phase_s);
        for s in &self.ships {
            for v in [s.pos, s.vel, s.rates] {
                h.f64(v.x).f64(v.y).f64(v.z);
            }
            h.f64(s.rot.x).f64(s.rot.y).f64(s.rot.z).f64(s.rot.w).f64(s.hull);
            for f in s.faces {
                h.f64(f);
            }
        }
        h.u64(self.bolts.len() as u64).u64(self.missiles.len() as u64);
        h.value()
    }
}

/// Reject a command with a value no console can send (CLAUDE.md 6.6: non-finite numbers stop at the edge).
pub fn validate(cmd: &Command) -> Result<(), Refusal> {
    match *cmd {
        Command::Stick { speed_set_mps, yaw, pitch, roll } => {
            let ok = speed_set_mps.is_finite()
                && speed_set_mps.abs() <= 10_000.0
                && [yaw, pitch, roll].iter().all(|v| v.is_finite() && v.abs() <= 1.0);
            if ok {
                Ok(())
            } else {
                Err(Refusal::BadValue)
            }
        }
        Command::Strafe { lat_mps, vert_mps } => {
            if [lat_mps, vert_mps].iter().all(|v| v.is_finite() && v.abs() <= 1000.0) {
                Ok(())
            } else {
                Err(Refusal::BadValue)
            }
        }
        Command::Orient(Some(hpr)) => {
            if hpr.iter().all(|v| v.is_finite() && v.abs() <= 720.0) {
                Ok(())
            } else {
                Err(Refusal::BadValue)
            }
        }
        Command::Freq(b) if b >= 4 => Err(Refusal::BadValue),
        Command::Viewscreen(Some(v)) if usize::from(v) >= FEEDS.len() => Err(Refusal::BadValue),
        Command::Order(Station::Captain, _) | Command::Ack(Station::Captain) => Err(Refusal::NoSuch),
        _ => Ok(()),
    }
}

/// `dir` perturbed by a 2D normal angular error of `sigma_rad` (one sigma per axis), seeded.
fn scatter(rng: &mut Rng, dir: DVec3, sigma_rad: f64) -> DVec3 {
    let (e1, e2) = dir.any_orthonormal_pair();
    let u1 = rng.next_f64().max(1e-12);
    let u2 = rng.next_f64();
    let r = (-2.0 * u1.ln()).sqrt() * sigma_rad;
    let a = std::f64::consts::TAU * u2;
    (dir + e1 * (r * a.cos()) + e2 * (r * a.sin())).normalize()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn drill() -> Drill {
        Drill::new(DrillData::shipped(), 7)
    }

    fn run_to_engage(d: &mut Drill) {
        for _ in 0..(TICK_HZ as usize * 10) {
            d.step();
            if d.phase == Phase::Engage {
                return;
            }
        }
        panic!("the drill never reached Engage");
    }

    #[test]
    fn the_shipped_drill_data_loads() {
        let d = DrillData::shipped();
        assert_eq!(d.mission.enemy, "hound");
        assert_eq!(d.tern_combat.turrets.len(), 4);
    }

    #[test]
    fn a_drill_with_nobody_aboard_stays_in_muster() {
        let mut d = drill();
        for _ in 0..(TICK_HZ as usize * 60) {
            d.step();
        }
        assert_eq!(d.phase, Phase::Muster);
    }

    #[test]
    fn a_seated_ready_crew_starts_the_countdown_and_then_the_fight() {
        let mut d = drill();
        let a = d.join("Ensign A", false).unwrap();
        d.claim(a, Station::Helm).unwrap();
        d.step();
        assert_eq!(d.phase, Phase::Muster, "not ready yet");
        d.set_ready(a, true).unwrap();
        d.step();
        assert_eq!(d.phase, Phase::Countdown);
        run_to_engage(&mut d);
        assert!(d.ships[1].active, "the Hound appears at Engage");
    }

    #[test]
    fn a_taken_station_is_refused_and_a_command_from_the_wrong_seat_too() {
        let mut d = drill();
        let a = d.join("A", false).unwrap();
        let b = d.join("B", false).unwrap();
        d.claim(a, Station::Helm).unwrap();
        assert_eq!(d.claim(b, Station::Helm), Err(Refusal::StationTaken));
        assert_eq!(d.command(b, Command::AllStop), Err(Refusal::NotYourStation));
        assert_eq!(d.command(a, Command::TurretMode(0, TurretMode::Auto)), Err(Refusal::NotYourStation));
    }

    #[test]
    fn a_non_finite_stick_is_refused() {
        let mut d = drill();
        let a = d.join("A", false).unwrap();
        d.claim(a, Station::Helm).unwrap();
        let bad = Command::Stick { speed_set_mps: 10.0, yaw: f64::NAN, pitch: 0.0, roll: 0.0 };
        assert_eq!(d.command(a, bad), Err(Refusal::BadValue));
    }

    #[test]
    fn the_hit_chance_predicts_the_hit_rate() {
        // A still target, a still shooter: fire 4,000 bolts at the range where the predicted chance is 50 %.
        let data = DrillData::shipped();
        let radius = data.enemy.combat.hit_radius_m;
        let sigma = 0.3f64.to_radians();
        // 1 - exp(-R^2 / 2 s^2) = 0.5  =>  s = R / sqrt(2 ln 2)
        let range = radius / (2.0 * std::f64::consts::LN_2).sqrt() / sigma;
        assert!((hit_chance(range, sigma, radius) - 0.5).abs() < 1e-9);
        let mut rng = Rng::for_purpose(1, 2, "calibration");
        let n = 4000;
        let mut hits = 0;
        for _ in 0..n {
            let d = scatter(&mut rng, DVec3::Z, sigma);
            // Where the bolt crosses the target's plane, against the hit sphere's disc.
            let at = d * (range / d.z);
            if (at - DVec3::Z * range).length() <= radius {
                hits += 1;
            }
        }
        let rate = f64::from(hits) / f64::from(n);
        assert!(
            (0.47..=0.53).contains(&rate),
            "observed {rate:.3} against predicted 0.5: the preview must be the outcome"
        );
    }

    #[test]
    fn the_lead_solution_meets_a_crossing_target() {
        let rel = DVec3::new(0.0, 0.0, 1500.0);
        let vel = DVec3::new(200.0, 0.0, 0.0);
        let a = lead(rel, vel, 1500.0).unwrap();
        let bolt_at = a.dir * 1500.0 * a.time_s;
        let target_at = rel + vel * a.time_s;
        assert!((bolt_at - target_at).length() < 1e-6);
    }

    #[test]
    fn full_assist_reaches_the_set_point_and_turns_at_the_rate_limit() {
        let data = DrillData::shipped();
        let mut d = drill();
        let s = &mut d.ships[0];
        s.vel = DVec3::ZERO;
        s.speed_set = 400.0;
        for _ in 0..(TICK_HZ as usize * 40) {
            fly(s, &data.tern_flight, DT);
        }
        assert!((s.forward().dot(s.vel) - 400.0).abs() < 1.0, "0 to 400 m/s inside 40 s (FN: 27 s)");
        s.stick = [1.0, 0.0, 0.0];
        for _ in 0..(TICK_HZ as usize * 5) {
            fly(s, &data.tern_flight, DT);
        }
        assert!((s.rates.y.to_degrees() - 18.0).abs() < 0.5, "yaw settles at the 18 deg/s limit");
    }

    #[test]
    fn steering_brings_the_bow_onto_a_target() {
        let data = DrillData::shipped();
        let mut d = drill();
        let s = &mut d.ships[0];
        let target = DVec3::new(3000.0, 800.0, 1000.0);
        for _ in 0..(TICK_HZ as usize * 30) {
            s.stick = steer_toward(s.rot, s.rates, &data.tern_flight, target - s.pos, 1.0);
            fly(s, &data.tern_flight, DT);
        }
        let off = off_bow_deg(s.pos, s.rot, target);
        assert!(off < 3.0, "bow within 3 deg after 30 s, got {off:.1}");
    }

    #[test]
    fn a_shield_face_absorbs_before_the_hull() {
        let mut d = drill();
        let pos = d.ships[0].pos;
        let bow_point = pos + DVec3::Z * 10.0;
        let face = d.damage(0, bow_point, 50.0, None);
        assert_eq!(face, 0, "a hit ahead strikes the bow face");
        assert_eq!(d.ships[0].faces[0], 0.0);
        assert!((d.ships[0].hull - (d.data.tern_combat.hull_mj - 10.0)).abs() < 1e-9, "40 MJ absorbed, 10 to the hull");
    }

    /// A drill played to its end: each seat held by a bot client (the `bot` profile through the same commands a
    /// console sends) or, when false, left to the server's automation.
    fn crew_drill(helm_bot: bool, tactical_bot: bool, seed: u64) -> Drill {
        let mut d = Drill::new(DrillData::shipped(), seed);
        // A player in Muster starts the drill; a seat a bot does not play is given back to automation at Engage.
        let h = d.join("Helm", true).unwrap();
        let t = d.join("Tactical", true).unwrap();
        d.claim(h, Station::Helm).unwrap();
        d.claim(t, Station::Tactical).unwrap();
        d.set_ready(h, true).unwrap();
        d.set_ready(t, true).unwrap();
        run_to_engage(&mut d);
        if !helm_bot {
            d.leave(h);
        }
        if !tactical_bot {
            d.leave(t);
        }
        let bot = d.data.profile("bot").clone();
        let mut mem = automation::Memory::default();
        while d.phase == Phase::Engage {
            let pic = d.picture();
            let now = d.phase_s;
            if helm_bot {
                for c in automation::helm(&pic, &bot.helm, &d.data.tern_flight, &mut mem.helm, now) {
                    let _ = d.command(h, c);
                }
            }
            if tactical_bot {
                for c in automation::tactical(&pic, &bot.tactical, &d.data, &mut mem.tactical, now) {
                    let _ = d.command(t, c);
                }
            }
            d.step();
        }
        d
    }

    #[test]
    fn two_bots_at_helm_and_tactical_win_the_drill() {
        for seed in [1, 2, 3] {
            let d = crew_drill(true, true, seed);
            assert_eq!(d.outcome, Outcome::Victory, "seed {seed}: {:?}", d.stats);
            assert!(
                d.stats.engage_s < 300.0,
                "seed {seed}: a crew wins inside five minutes, took {:.0} s",
                d.stats.engage_s
            );
        }
    }

    #[test]
    fn left_to_automation_the_drill_is_lost() {
        // The drill is a co-op test: automation alone (bridge-stations' lower competence) must not win it.
        for seed in [1, 2, 3] {
            let d = crew_drill(false, false, seed);
            assert_eq!(d.outcome, Outcome::Defeat, "seed {seed}: {:?}", d.stats);
        }
    }

    #[test]
    fn tactical_alone_fires_no_missile_because_nobody_puts_the_bow_on_the_hound() {
        let d = crew_drill(false, true, 1);
        assert_eq!(d.stats.missiles_fired, 0, "the seeker's cone needs a helm");
        assert_ne!(d.outcome, Outcome::Victory);
    }

    #[test]
    fn the_same_seed_and_commands_give_the_same_drill() {
        let a = crew_drill(true, true, 5);
        let b = crew_drill(true, true, 5);
        assert_eq!(a.hash(), b.hash());
        assert_eq!(a.tick, b.tick);
    }

    #[test]
    fn automation_alone_never_fires_a_missile() {
        let mut d = Drill::new(DrillData::shipped(), 9);
        let h = d.join("Helm", false).unwrap();
        d.claim(h, Station::Helm).unwrap();
        d.set_ready(h, true).unwrap();
        run_to_engage(&mut d);
        for _ in 0..(TICK_HZ as usize * 60) {
            d.step();
        }
        assert_eq!(d.stats.missiles_fired, 0, "bridge-stations: automation never fires missiles on its own");
        assert_eq!(d.ships[0].lock_target, Some(ENEMY_ID), "but it locks the Hound");
    }

    #[test]
    fn a_debrief_leads_to_a_new_muster_with_the_crew_unready() {
        let mut d = crew_drill(true, true, 4);
        assert_eq!(d.phase, Phase::Debrief);
        for _ in 0..(TICK_HZ as usize * 30) {
            d.step();
        }
        assert_eq!(d.phase, Phase::Muster);
        assert_eq!(d.round, 1);
        assert!(d.players.iter().all(|p| !p.ready));
        assert!(d.ships[1].alive && !d.ships[1].active, "a fresh Hound waits for the next Engage");
    }

    /// A drill in Engage with one player at helm and one at tactical, for driving commands by hand.
    fn engaged() -> (Drill, u8, u8) {
        let mut d = drill();
        let h = d.join("Helm", false).unwrap();
        let t = d.join("Tactical", false).unwrap();
        d.claim(h, Station::Helm).unwrap();
        d.claim(t, Station::Tactical).unwrap();
        d.set_ready(h, true).unwrap();
        d.set_ready(t, true).unwrap();
        run_to_engage(&mut d);
        (d, h, t)
    }

    fn steps(d: &mut Drill, s: f64) {
        for _ in 0..(s * TICK_HZ) as usize {
            d.step();
        }
    }

    #[test]
    fn the_strafe_pad_drifts_the_ship_sideways_to_its_set_point() {
        let (mut d, h, _) = engaged();
        d.command(h, Command::Strafe { lat_mps: 50.0, vert_mps: -20.0 }).unwrap();
        steps(&mut d, 1.0);
        let local = d.ships[0].rot.inverse() * d.ships[0].vel;
        // tau_v 1 s: 63% of the way after a second.
        assert!((-local.x - 50.0 * (1.0 - (-1.0f64).exp())).abs() < 2.0, "to starboard after 1 s: {:.1}", -local.x);
        steps(&mut d, 20.0);
        let local = d.ships[0].rot.inverse() * d.ships[0].vel;
        assert!((-local.x - 50.0).abs() < 0.5 && (local.y + 20.0).abs() < 0.5, "the set points are reached: {local:?}");
        d.command(h, Command::AllStop).unwrap();
        steps(&mut d, 30.0);
        let local = d.ships[0].rot.inverse() * d.ships[0].vel;
        assert!(local.length() < 0.5, "all stop takes speed and drift to zero: {local:?}");
    }

    #[test]
    fn an_attitude_order_turns_the_ship_there_in_about_its_planned_time_and_holds_it() {
        let (mut d, h, _) = engaged();
        let goal = attitude::from_hpr(45.0, 10.0, -30.0);
        let plan = attitude::slew(d.ships[0].rot, d.ships[0].rates, goal, &d.data.tern_flight);
        d.command(h, Command::Orient(Some([45.0, 10.0, -30.0]))).unwrap();
        steps(&mut d, plan.time_s * 1.6 + 1.0);
        let left = attitude::slew(d.ships[0].rot, d.ships[0].rates, goal, &d.data.tern_flight);
        assert!(
            left.angle_deg < 1.0,
            "within a degree after the planned {:.1} s: {:.2} left",
            plan.time_s,
            left.angle_deg
        );
        let (hh, p, r, _) = attitude::hpr_of(d.ships[0].rot);
        assert!((hh - 45.0).abs() < 1.0 && (p - 10.0).abs() < 1.0 && (r + 30.0).abs() < 1.0, "{hh:.1} {p:.1} {r:.1}");
        assert!(d.ships[0].order.is_some(), "an order is held until cancelled");
        d.command(h, Command::Stick { speed_set_mps: 60.0, yaw: 0.6, pitch: 0.0, roll: 0.0 }).unwrap();
        d.step();
        assert!(d.ships[0].order.is_none(), "the stick cancels the order");
    }

    #[test]
    fn chase_puts_the_bow_on_the_target_and_course_on_the_waypoint() {
        let (mut d, h, _) = engaged();
        d.command(h, Command::Helm(HelmMode::Chase)).unwrap();
        steps(&mut d, 20.0);
        let off = off_bow_deg(d.ships[0].pos, d.ships[0].rot, d.ships[1].pos);
        assert!(off < 8.0, "chase keeps the bow on the Hound: {off:.1} deg");
        d.command(h, Command::Helm(HelmMode::Course)).unwrap();
        steps(&mut d, 20.0);
        let wp = DVec3::from_array(d.data.mission.waypoint_m);
        let off = off_bow_deg(d.ships[0].pos, d.ships[0].rot, wp);
        assert!(off < 2.0, "course puts the bow on the waypoint: {off:.1} deg");
    }

    #[test]
    fn match_takes_the_targets_course_and_speed() {
        let (mut d, h, _) = engaged();
        d.command(h, Command::Helm(HelmMode::Match)).unwrap();
        steps(&mut d, 3.0);
        let hound = d.ships[1].vel.length().clamp(d.data.tern_flight.speed_min_mps, d.data.tern_flight.speed_max_mps);
        // Set at the start of the tick from the Hound's speed then; it changes by at most its acceleration in a tick.
        let slack = d.data.enemy.flight.accel_fwd_mps2.max(d.data.enemy.flight.accel_rev_mps2) * DT;
        assert!((d.ships[0].speed_set - hound).abs() <= slack + 1e-9, "the speed set point follows the Hound's speed");
    }

    #[test]
    fn evade_throws_jinks_from_the_round_seed_and_leaving_it_stops_them() {
        let (mut d, h, _) = engaged();
        d.command(h, Command::Helm(HelmMode::Evade)).unwrap();
        d.step();
        let a = d.ships[0].strafe;
        assert!(a != [0.0; 2] && a.iter().all(|v| v.abs() <= d.data.tern_flight.jink_mps));
        let period = d.data.tern_flight.jink_period_s;
        steps(&mut d, period + 0.1);
        assert_ne!(d.ships[0].strafe, a, "a new jink every period");
        d.command(h, Command::Helm(HelmMode::Hold)).unwrap();
        d.step();
        assert_eq!(d.ships[0].strafe, [0.0; 2], "leaving EVADE takes its drift off");
    }

    #[test]
    fn a_turret_on_hold_never_fires_and_auto_fires_at_the_locked_target() {
        let (mut d, _, t) = engaged();
        d.command(t, Command::Lock(Some(ENEMY_ID))).unwrap();
        steps(&mut d, 25.0);
        assert_eq!(d.stats.tern_shots, 0, "every turret starts on HOLD");
        for i in 0..4 {
            d.command(t, Command::TurretMode(i, TurretMode::Auto)).unwrap();
        }
        steps(&mut d, 25.0);
        assert!(d.stats.tern_shots > 0, "AUTO fires at the locked Hound in its arc");
    }

    #[test]
    fn heat_locks_a_turret_out_and_it_fires_again_when_cool() {
        let data = DrillData::shipped();
        let gun = data.gun(&data.tern_combat.gun).clone();
        let mut d = drill();
        let tu = &mut d.ships[0].turrets[0];
        tu.heat_mj = gun.heat_sink_mj;
        tu.cooling = true;
        d.ships[0].turrets[0].mode = TurretMode::Auto;
        // Cooling from full to the resume share takes (1 - resume) x sink / shed seconds: 6 s for the twin pulse.
        let cool_s = (1.0 - gun.heat_resume_frac) * gun.heat_sink_mj / gun.heat_shed_mw;
        for _ in 0..((cool_s - 0.5) * TICK_HZ) as usize {
            d.turrets(0, 1);
        }
        assert!(d.ships[0].turrets[0].cooling, "still locked out before it has cooled");
        for _ in 0..(1.0 * TICK_HZ) as usize {
            d.turrets(0, 1);
        }
        assert!(!d.ships[0].turrets[0].cooling, "free again below {:.0}% of its sink", gun.heat_resume_frac * 100.0);
    }

    #[test]
    fn each_turret_bears_on_its_own_side_of_the_ship() {
        let m = &DrillData::shipped().tern_combat.turrets;
        let arc = |id: &str| m.iter().find(|t| t.id == id).unwrap().arc;
        assert!(arc_bears(arc("port"), DVec3::new(1.0, 0.0, 0.2), 10.0));
        assert!(!arc_bears(arc("port"), DVec3::new(-1.0, 0.0, 0.2), 10.0), "the port turret cannot fire to starboard");
        assert!(arc_bears(arc("starboard"), DVec3::new(-1.0, 0.0, 0.0), 10.0));
        assert!(
            arc_bears(arc("dorsal"), DVec3::new(0.0, -0.1, 1.0), 10.0),
            "ten degrees below the mount's plane bears"
        );
        assert!(!arc_bears(arc("ventral"), DVec3::new(0.0, 1.0, 0.0), 10.0));
    }

    #[test]
    fn the_drills_turrets_are_the_layouts_four_mounts() {
        let layout: serde_json::Value =
            serde_json::from_str(include_str!("../../../../data/ships/tern/layout.json")).unwrap();
        let mounts = layout["mounts"].as_array().unwrap();
        let data = DrillData::shipped();
        for (t, id) in
            data.tern_combat.turrets.iter().zip(["turret_dorsal", "turret_ventral", "turret_port", "turret_stbd"])
        {
            let m = mounts.iter().find(|m| m["id"] == id).unwrap_or_else(|| panic!("the layout has {id}"));
            let c: Vec<f64> = m["center_m"].as_array().unwrap().iter().map(|v| v.as_f64().unwrap()).collect();
            assert_eq!(t.position_m.to_vec(), c, "{} sits where the layout puts {id}", t.id);
            let f: Vec<f64> = m["facing"].as_array().unwrap().iter().map(|v| v.as_f64().unwrap()).collect();
            assert_eq!(t.arc.facing().unwrap().to_array().to_vec(), f, "{} faces as {id} does", t.id);
        }
    }

    #[test]
    fn a_hit_on_the_port_side_forward_of_midships_is_port_not_bow() {
        // weapons-and-shields 11: 20 m forward on the port side is port with the round rule (a cube would say bow).
        let e = DrillData::shipped().tern_shields.ellipsoid;
        let o = DVec3::new(12.0, 0.0, 20.0);
        assert_eq!(shield_face(o, DVec3::from_array(e.axes_m)), 2);
        assert_eq!(shield_face(DVec3::new(0.0, 0.0, 50.0), DVec3::from_array(e.axes_m)), 0);
        assert_eq!(shield_face(DVec3::new(0.0, -12.0, 0.0), DVec3::from_array(e.axes_m)), 5);
    }

    /// A drill in Engage with a player at each of `stations`; their slots in the same order.
    fn engaged_at(stations: &[Station]) -> (Drill, Vec<u8>) {
        let mut d = drill();
        let slots: Vec<u8> = stations
            .iter()
            .map(|s| {
                let p = d.join(s.name(), false).unwrap();
                d.claim(p, *s).unwrap();
                d.set_ready(p, true).unwrap();
                p
            })
            .collect();
        run_to_engage(&mut d);
        (d, slots)
    }

    #[test]
    fn a_manned_scan_takes_scan_s_and_the_automation_one_and_a_half_times_as_long() {
        let scan_s = DrillData::shipped().tern_combat.sensors.unwrap().scan_s;
        let (mut d, s) = engaged_at(&[Station::Science]);
        steps(&mut d, 1.0);
        assert_eq!(d.bridge.scan, 0.0, "a manned Science scans only when told to");
        d.command(s[0], Command::Scan(true)).unwrap();
        steps(&mut d, scan_s / 2.0);
        assert!((d.bridge.scan - 0.5).abs() < 0.02, "half a scan in half its time: {}", d.bridge.scan);
        steps(&mut d, scan_s / 2.0 + 0.1);
        assert_eq!(d.bridge.scan, 1.0);
        assert!(!d.bridge.scanning, "a finished scan stops");
        // Empty, Science's automation passive-scans at 1.5 times the time (bridge-stations 3).
        let (mut d, _) = engaged_at(&[Station::Helm]);
        steps(&mut d, scan_s);
        assert!((d.bridge.scan - 1.0 / 1.5).abs() < 0.02, "automation's scan in scan_s: {}", d.bridge.scan);
    }

    #[test]
    fn tactical_automation_leaves_the_shields_to_a_manned_science() {
        let (mut d, s) = engaged_at(&[Station::Science]);
        d.command(s[0], Command::Preset(1)).unwrap();
        steps(&mut d, 8.0);
        assert_eq!(d.ships[0].preset, 1, "the automation did not put its own preset back");
        assert_eq!(d.bridge.shields_by, Station::Science);
        let (mut d, _) = engaged_at(&[Station::Helm]);
        d.ships[0].preset = 1;
        steps(&mut d, 8.0);
        assert_eq!(d.ships[0].preset, 0, "with Science empty, Tactical's automation sets the shields");
        assert_eq!(d.bridge.shields_by, Station::Tactical);
    }

    #[test]
    fn a_matching_band_spares_the_shield_and_a_retune_costs_it() {
        let data = DrillData::shipped();
        let f = data.tern_shields.frequency.unwrap();
        let hound_band = data.gun(&data.enemy.combat.gun).band_index();
        let face_loss = |band: u8, retune: bool| {
            let mut d = drill();
            d.bridge.band = band;
            d.bridge.retune_s = if retune { f.retune_s } else { 0.0 };
            let before = d.ships[0].faces[0];
            d.damage(0, d.ships[0].pos + DVec3::Z * 60.0, 10.0, Some(hound_band));
            before - d.ships[0].faces[0]
        };
        let other = (hound_band + 1) % 4;
        assert!((face_loss(other, false) - 10.0).abs() < 1e-9);
        assert!((face_loss(hound_band, false) - 10.0 * f.match_factor).abs() < 1e-9, "a matching band loses less");
        assert!((face_loss(hound_band, true) - 10.0 * f.retune_factor).abs() < 1e-9, "retuning loses more");
        // A retune is started by changing the band, and only Science may.
        let (mut d, s) = engaged_at(&[Station::Science, Station::Helm]);
        assert_eq!(d.command(s[1], Command::Freq(2)), Err(Refusal::NotYourStation));
        d.command(s[0], Command::Freq(2)).unwrap();
        d.step();
        assert_eq!((d.bridge.band, d.bridge.retune_s > 0.0), (2, true));
        steps(&mut d, f.retune_s + 0.1);
        assert_eq!(d.bridge.retune_s, 0.0);
    }

    #[test]
    fn the_captains_automation_calls_red_alert_when_a_hostile_closes() {
        let (mut d, _) = engaged_at(&[Station::Helm]);
        let p = d.data.profile("automation").captain.clone();
        assert!(d.data.mission.enemy_start_m.iter().map(|v| v * v).sum::<f64>().sqrt() < p.red_within_m);
        steps(&mut d, p.reaction_s + 0.1);
        assert!(d.bridge.red_alert, "the Hound starts inside the red alert range");
        // Manned, the condition is the captain's.
        let (mut d, s) = engaged_at(&[Station::Captain]);
        steps(&mut d, p.reaction_s + 0.1);
        assert!(!d.bridge.red_alert, "a seated captain decides the condition");
        d.command(s[0], Command::Alert(true)).unwrap();
        d.step();
        assert!(d.bridge.red_alert);
    }

    #[test]
    fn an_order_waits_for_its_station_to_acknowledge_it() {
        let (mut d, s) = engaged_at(&[Station::Captain, Station::Helm]);
        assert_eq!(d.command(s[1], Command::Order(Station::Tactical, 0)), Err(Refusal::NotYourStation));
        d.command(s[0], Command::Order(Station::Helm, 1)).unwrap();
        d.command(s[0], Command::Order(Station::Tactical, 0)).unwrap();
        d.step();
        assert_eq!(d.bridge.orders.len(), 2);
        assert_eq!((d.bridge.orders[0].to, d.bridge.orders[1].verb), (Station::Tactical, 1));
        // Tactical is empty: its automation acknowledges after its reaction time. Helm waits for its player.
        let react = d.data.profile("automation").tactical.reaction_s;
        steps(&mut d, react + 0.1);
        assert!(d.bridge.orders[0].done && !d.bridge.orders[1].done);
        assert_eq!(d.command(s[0], Command::Ack(Station::Helm)), Err(Refusal::NotYourStation));
        d.command(s[1], Command::Ack(Station::Helm)).unwrap();
        d.step();
        assert!(d.bridge.orders[1].done);
        // A verb the station does not have is refused.
        d.command(s[0], Command::Order(Station::Science, 9)).unwrap();
        d.step();
        assert_eq!(d.bridge.orders.len(), 2);
    }

    #[test]
    fn the_captain_takes_the_viewscreen_and_gives_it_back() {
        let (mut d, s) = engaged_at(&[Station::Captain]);
        d.command(s[0], Command::Viewscreen(Some(4))).unwrap();
        d.command(s[0], Command::Brace(true)).unwrap();
        d.step();
        assert_eq!((d.bridge.view, d.bridge.braced), (Some(4), true));
        assert_eq!(d.command(s[0], Command::Viewscreen(Some(9))), Err(Refusal::BadValue));
        d.command(s[0], Command::Viewscreen(None)).unwrap();
        d.step();
        assert_eq!(d.bridge.view, None);
    }
}
