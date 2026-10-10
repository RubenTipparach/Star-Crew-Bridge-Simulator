//! The co-op drill's combat: one Tern, one enemy, the stations that fly and fight her, and the drill's phases
//! (openspec/changes/coop-drill design sections 1-2).
//!
//! It lives in the core because the server runs it and the client previews with it (the hit chance, the steering
//! a Target order applies), with no clock, socket or GPU (CLAUDE.md 6.2). It steps at the server's fixed tick in
//! `f64`, in the drill's one system frame; randomness is seeded per purpose (CLAUDE.md 6.4); commands are applied
//! in a stable order. Ship-local axes: +X port, +Y dorsal, +Z bow. A ship's attitude maps ship-local to system.

pub mod automation;
pub mod bodies;
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

/// A station the drill uses.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Station {
    /// Flies the ship.
    Helm = 0,
    /// Locks, fires and sets the shields.
    Tactical = 1,
}

impl Station {
    /// Both, in seat order.
    pub const ALL: [Station; 2] = [Station::Helm, Station::Tactical];
    /// From a data id (`helm`).
    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "helm" => Some(Self::Helm),
            "tactical" => Some(Self::Tactical),
            _ => None,
        }
    }
    /// The data id.
    pub fn id(self) -> &'static str {
        match self {
            Self::Helm => "helm",
            Self::Tactical => "tactical",
        }
    }
    /// What a screen calls it.
    pub fn name(self) -> &'static str {
        match self {
            Self::Helm => "Helm",
            Self::Tactical => "Tactical",
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

/// What the helm's stick is doing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HelmMode {
    /// The stick flies the ship.
    Manual = 0,
    /// Bow on the locked or nearest hostile, until the stick moves.
    Target = 1,
    /// Pitch and roll to the drill's plane, until the stick moves.
    Level = 2,
}

impl HelmMode {
    /// From its wire byte.
    pub fn from_u8(v: u8) -> Option<Self> {
        [Self::Manual, Self::Target, Self::Level].get(usize::from(v)).copied()
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
    /// Stick [yaw, pitch, roll] in [-1, 1]: yaw to port, pitch nose up, roll port up.
    pub stick: [f64; 3],
    /// What drives the stick.
    pub helm_mode: HelmMode,
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
    /// Whether the turrets may fire.
    pub weapons_free: bool,
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
    /// Helm: what drives the stick.
    Helm(HelmMode),
    /// Helm: set point to zero.
    AllStop,
    /// Tactical: designate a target, or clear it.
    Lock(Option<u16>),
    /// Tactical: free the turrets or hold them.
    WeaponsFree(bool),
    /// Tactical: a shield preset by index.
    Preset(u8),
    /// Tactical: load a tube.
    Load(u8),
    /// Tactical: fire a tube.
    Fire(u8),
}

impl Command {
    /// The station that may issue it.
    pub fn station(&self) -> Station {
        match self {
            Self::Stick { .. } | Self::Helm(_) | Self::AllStop => Station::Helm,
            _ => Station::Tactical,
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
    /// The body is on its way somewhere already (coop-drill design 9).
    Walking = 11,
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
            Walking,
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
            Self::Walking => "On the way",
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
    /// A player relieved a bot at a station; the bot is getting up (design 9).
    Relieved {
        /// The bot's slot.
        slot: u8,
        /// The station.
        station: Station,
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

/// The stick that turns a ship's bow toward `desired` (system frame) at `rate_frac` of its rate limits, and rolls
/// it level with the drill's plane (+Y up). The Target and Level orders, the enemy's flying and the bot helm all
/// steer with this one function (CLAUDE.md 6.1).
pub fn steer_toward(rot: DQuat, rates: DVec3, flight: &FlightBlock, desired: DVec3, rate_frac: f64) -> [f64; 3] {
    let d = rot.inverse() * desired.normalize_or_zero();
    let yaw_err = d.x.atan2(d.z);
    let pitch_err = d.y.atan2(d.x.hypot(d.z));
    let port = rot * DVec3::X;
    let dorsal = rot * DVec3::Y;
    let roll_err = -(port.y).atan2(dorsal.y);
    // A rate proportional to the error, damped by the rate the ship already has, as a share of the limit.
    let gain = 1.5;
    let lim = flight.rate_limit_deg_s.map(f64::to_radians);
    let axis = |err: f64, rate: f64, limit: f64| ((err * gain - 0.35 * rate) / limit).clamp(-rate_frac, rate_frac);
    [axis(yaw_err, rates.y, lim[0]), axis(pitch_err, -rates.x, lim[1]), axis(roll_err, rates.z, lim[2])]
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
    let fwd = ship.forward();
    let v_fwd = ship.vel.dot(fwd);
    let side = ship.vel - fwd * v_fwd;
    let set = ship.speed_set.clamp(flight.speed_min_mps, flight.speed_max_mps);
    let dv = ((set - v_fwd) / flight.tau_v_s).clamp(-flight.accel_rev_mps2, flight.accel_fwd_mps2) * dt;
    ship.vel = fwd * (v_fwd + dv) + side * (-dt / flight.tau_v_s).exp();
    ship.pos += ship.vel * dt;
}

/// The shield face a point on a ship strikes: the ship-local axis nearest the direction from its centre.
pub fn face_of(ship: &Ship, point: DVec3) -> u8 {
    let l = ship.rot.inverse() * (point - ship.pos);
    let a = l.abs();
    if a.z >= a.x && a.z >= a.y {
        if l.z >= 0.0 {
            0
        } else {
            1
        }
    } else if a.x >= a.y {
        if l.x >= 0.0 {
            2
        } else {
            3
        }
    } else if l.y >= 0.0 {
        4
    } else {
        5
    }
}

/// A target's elevation above a ship's plane, in degrees, seen from a point on the ship.
pub fn elevation_deg(rot: DQuat, from: DVec3, target: DVec3) -> f64 {
    let d = (rot.inverse() * (target - from)).normalize_or_zero();
    d.y.clamp(-1.0, 1.0).asin().to_degrees()
}

/// Whether a turret's arc holds a target at `elevation_deg`.
pub fn arc_bears(arc: Arc, elevation_deg: f64, overlap_deg: f64) -> bool {
    match arc {
        Arc::All => true,
        Arc::Dorsal => elevation_deg >= -overlap_deg,
        Arc::Ventral => elevation_deg <= overlap_deg,
    }
}

/// The angle between a ship's bow and a target, in degrees, for a ship at `pos` with attitude `rot`.
pub fn off_bow_deg(pos: DVec3, rot: DQuat, target: DVec3) -> f64 {
    let to = (target - pos).normalize_or_zero();
    (rot * DVec3::Z).dot(to).clamp(-1.0, 1.0).acos().to_degrees()
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
    events: Vec<DrillEvent>,
    queue: Vec<(Station, u64, Option<u8>, Command)>,
    arrivals: u64,
    next_bolt: u32,
    next_missile: u16,
    rng: Rng,
    auto: automation::Memory,
    /// The bridge the crew walk on, when the drill has one (design 9); without it a claim seats at once.
    pub bridge: Option<bodies::Bridge>,
    /// Every crew member's body, by slot order.
    pub bodies: Vec<bodies::CrewBody>,
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
        stick: [0.0; 3],
        helm_mode: HelmMode::Manual,
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
        weapons_free: false,
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
            events: Vec::new(),
            queue: Vec::new(),
            arrivals: 0,
            next_bolt: 1,
            next_missile: 1,
            rng: Rng::for_purpose(seed, 0, "drill"),
            auto: automation::Memory::default(),
            bridge: None,
            bodies: Vec::new(),
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
        let name: String = name.chars().filter(|c| !c.is_control()).take(MAX_NAME_CHARS).collect();
        self.players.push(Player { slot, name, station: None, ready: false, bot });
        self.players.sort_by_key(|p| p.slot);
        if let Some(b) = &self.bridge {
            self.bodies.push(bodies::CrewBody::at_muster(slot, b));
            self.bodies.sort_by_key(|b| b.slot);
        }
        Ok(slot)
    }

    /// Put the crew on foot on `bridge` (design 9): every body to the muster point, every station to automation.
    pub fn set_bridge(&mut self, bridge: bodies::Bridge) {
        self.bodies = self.players.iter().map(|p| bodies::CrewBody::at_muster(p.slot, &bridge)).collect();
        for p in &mut self.players {
            p.station = None;
        }
        self.bridge = Some(bridge);
    }

    /// Slot `slot`'s body.
    pub fn body(&self, slot: u8) -> Option<&bodies::CrewBody> {
        self.bodies.iter().find(|b| b.slot == slot)
    }

    /// A player leaves; their station returns to automation on the next tick.
    pub fn leave(&mut self, slot: u8) {
        self.players.retain(|p| p.slot != slot);
        self.bodies.retain(|b| b.slot != slot);
        self.queue.retain(|q| q.2 != Some(slot));
    }

    /// Slot `slot`'s player.
    pub fn player(&self, slot: u8) -> Option<&Player> {
        self.players.iter().find(|p| p.slot == slot)
    }

    fn player_mut(&mut self, slot: u8) -> Option<&mut Player> {
        self.players.iter_mut().find(|p| p.slot == slot)
    }

    /// A player takes a station, if nobody else holds it and the drill uses it.
    pub fn claim(&mut self, slot: u8, s: Station) -> Result<(), Refusal> {
        if !self.data.mission.stations.iter().any(|id| id == s.id()) {
            return Err(Refusal::NoSuch);
        }
        if self.bridge.is_some() {
            return self.claim_on_foot(slot, s);
        }
        match self.operator(s) {
            Operator::Player(other) if other != slot => return Err(Refusal::StationTaken),
            _ => {}
        }
        let p = self.player_mut(slot).ok_or(Refusal::NotYourStation)?;
        p.station = Some(s);
        Ok(())
    }

    /// A claim on foot (design 9): a walk order. The body gets up, its station goes to automation, it walks to the
    /// seat and becomes the operator when it sits. A player relieves a bot; nobody walks into a player's seat, or one
    /// someone is already walking to.
    fn claim_on_foot(&mut self, slot: u8, s: Station) -> Result<(), Refusal> {
        let Some(bridge) = self.bridge.clone() else { return Err(Refusal::NoSuch) };
        if !bridge.has(s) {
            return Err(Refusal::NoSuch);
        }
        let me = self.players.iter().find(|p| p.slot == slot).ok_or(Refusal::NotYourStation)?;
        let (me_bot, me_station) = (me.bot, me.station);
        let body = self.bodies.iter().find(|b| b.slot == slot).ok_or(Refusal::NotYourStation)?;
        if body.busy() {
            return Err(Refusal::Walking);
        }
        if me_station == Some(s) {
            return Ok(());
        }
        if self.bodies.iter().any(|b| b.slot != slot && b.walking_to() == Some(s)) {
            return Err(Refusal::StationTaken);
        }
        if let Operator::Player(other) = self.operator(s) {
            let other_bot = self.players.iter().any(|p| p.slot == other && p.bot);
            if !(other_bot && !me_bot) {
                return Err(Refusal::StationTaken);
            }
            // Relieved (BS 6): the bot stands and goes; its own logic picks where next.
            if let Some(p) = self.player_mut(other) {
                p.station = None;
            }
            if let Some(b) = self.bodies.iter_mut().find(|b| b.slot == other) {
                b.go(bodies::Spot::Muster, &bridge);
            }
            self.events.push(DrillEvent::Relieved { slot: other, station: s });
        }
        if let Some(p) = self.player_mut(slot) {
            p.station = None;
        }
        if let Some(b) = self.bodies.iter_mut().find(|b| b.slot == slot) {
            b.go(bodies::Spot::Seat(s), &bridge);
        }
        Ok(())
    }

    fn step_bodies(&mut self) {
        let Some(bridge) = self.bridge.as_ref() else { return };
        let mut sat = Vec::new();
        for b in &mut self.bodies {
            if let Some(s) = b.step(bridge, DT as f32) {
                sat.push((b.slot, s));
            }
        }
        for (slot, s) in sat {
            if let Some(p) = self.players.iter_mut().find(|p| p.slot == slot) {
                p.station = Some(s);
            }
        }
    }

    /// A player is ready (or not) to begin.
    pub fn set_ready(&mut self, slot: u8, ready: bool) -> Result<(), Refusal> {
        let p = self.player_mut(slot).ok_or(Refusal::NotYourStation)?;
        p.ready = ready;
        Ok(())
    }

    /// Queue a player's command for the next tick, after checking the station and the values.
    pub fn command(&mut self, slot: u8, cmd: Command) -> Result<(), Refusal> {
        let s = cmd.station();
        if self.operator(s) != Operator::Player(slot) {
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
        self.step_bodies();
        // Commands, by station, then arrival (CLAUDE.md 6.4).
        let mut queue = std::mem::take(&mut self.queue);
        queue.sort_by_key(|q| (q.0, q.1));
        if self.phase == Phase::Engage {
            for (_, _, slot, cmd) in queue {
                if let Err(reason) = self.apply(cmd) {
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
        for s in Station::ALL {
            if self.operator(s) != Operator::Auto || !self.data.mission.stations.iter().any(|id| id == s.id()) {
                continue;
            }
            let profile = self.data.profile("automation").clone();
            let cmds = match s {
                Station::Helm => automation::helm(&pic, &profile.helm, &self.data.tern_flight, &mut self.auto.helm, t),
                Station::Tactical => {
                    automation::tactical(&pic, &profile.tactical, &self.data, &mut self.auto.tactical, t)
                }
            };
            for c in cmds {
                // Automation's commands take the same path as a console's; a refusal is its own business.
                let _ = self.apply(c);
            }
        }
    }

    /// Apply a validated command to the Tern (the one path for players, bots and automation).
    fn apply(&mut self, cmd: Command) -> Result<(), Refusal> {
        validate(&cmd)?;
        let lock_time = self.data.tern_combat.lock.time_s;
        let enemy_pos = self.ships[1].pos;
        let enemy_alive = self.ships[1].alive && self.ships[1].active;
        let tern = &mut self.ships[0];
        match cmd {
            Command::Stick { speed_set_mps, yaw, pitch, roll } => {
                tern.speed_set =
                    speed_set_mps.clamp(self.data.tern_flight.speed_min_mps, self.data.tern_flight.speed_max_mps);
                if yaw.abs() > 0.05 || pitch.abs() > 0.05 || roll.abs() > 0.05 {
                    tern.helm_mode = HelmMode::Manual;
                }
                if tern.helm_mode == HelmMode::Manual {
                    tern.stick = [yaw, pitch, roll];
                }
            }
            Command::Helm(m) => tern.helm_mode = m,
            Command::AllStop => tern.speed_set = 0.0,
            Command::Lock(t) => {
                if t.is_some_and(|id| id != ENEMY_ID || !enemy_alive) {
                    return Err(Refusal::NoSuch);
                }
                if tern.lock_target != t {
                    tern.lock_target = t;
                    tern.lock_s = 0.0;
                }
            }
            Command::WeaponsFree(f) => tern.weapons_free = f,
            Command::Preset(i) => {
                let p = self.data.tern_shields.presets.get(usize::from(i)).ok_or(Refusal::NoSuch)?;
                tern.preset = i;
                for (f, cap) in tern.faces.iter_mut().zip(p.faces_mj) {
                    *f = f.min(cap);
                }
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
        }
        Ok(())
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
        e.weapons_free = true;
    }

    fn engage_tick(&mut self) {
        // Helm orders drive the Tern's stick.
        let target_pos = self.ships[1].pos;
        let tern = &self.ships[0];
        match tern.helm_mode {
            HelmMode::Manual => {}
            HelmMode::Target => {
                let s = steer_toward(tern.rot, tern.rates, &self.data.tern_flight, target_pos - tern.pos, 1.0);
                self.ships[0].stick = s;
            }
            HelmMode::Level => {
                let f = tern.forward();
                let flat = DVec3::new(f.x, 0.0, f.z).normalize_or(DVec3::Z);
                let s = steer_toward(tern.rot, tern.rates, &self.data.tern_flight, flat, 1.0);
                self.ships[0].stick = [0.0, s[1], s[2]];
            }
        }
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
        let can_fire = shooter.alive
            && shooter.active
            && target.alive
            && target.active
            && shooter.weapons_free
            && shooter.lock_target == Some(target.id)
            && shooter.lock_s >= combat.lock.time_s;
        let (s_pos, s_rot, s_vel, t_pos, t_vel) = (shooter.pos, shooter.rot, shooter.vel, target.pos, target.vel);
        let mut fired = Vec::new();
        for (k, mount) in combat.turrets.iter().enumerate() {
            let turret = &mut self.ships[si].turrets[k];
            turret.capacitor_mj = (turret.capacitor_mj + gun.charge_mw * DT).min(gun.capacitor_mj);
            turret.cooldown_s = (turret.cooldown_s - DT).max(-DT);
            let muzzle = s_pos + s_rot * DVec3::from_array(mount.position_m);
            let elev = elevation_deg(s_rot, muzzle, t_pos);
            turret.bearing = arc_bears(mount.arc, elev, combat.arc_overlap_deg);
            let Some(aim) = lead(t_pos - muzzle, t_vel - s_vel, gun.speed_mps) else {
                turret.hit_chance = 0.0;
                continue;
            };
            let sigma_rad = (combat.aim_sigma_base_deg + gun.sigma_rate_k * aim.angular_rate_deg_s).to_radians();
            let p = if aim.time_s <= gun.life_s { hit_chance(aim.range_m, sigma_rad, target_radius) } else { 0.0 };
            turret.aim = aim.dir;
            turret.hit_chance = p;
            if can_fire
                && turret.bearing
                && p >= gun.min_hit_chance
                && turret.capacitor_mj >= gun.draw_mj
                && turret.cooldown_s <= 0.0
                && self.bolts.len() + fired.len() < MAX_BOLTS
            {
                turret.capacitor_mj -= gun.draw_mj;
                turret.cooldown_s += 1.0 / gun.rate_hz;
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

    fn damage(&mut self, ti: usize, point: DVec3, dmg: f64) -> u8 {
        let ship = &mut self.ships[ti];
        let face = face_of(ship, point);
        let absorbed = ship.faces[usize::from(face)].min(dmg);
        ship.faces[usize::from(face)] -= absorbed;
        ship.hull -= dmg - absorbed;
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
                let face = self.damage(ti, point, b.damage_mj);
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
                self.damage(1, point, dmg);
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
        for b in &self.bodies {
            h.u64(u64::from(b.slot)).f64(f64::from(b.pos[0])).f64(f64::from(b.pos[2])).u64(b.posture as u64);
        }
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
        assert_eq!(d.command(a, Command::WeaponsFree(true)), Err(Refusal::NotYourStation));
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
        let face = d.damage(0, bow_point, 50.0);
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

    fn on_foot() -> Drill {
        let mut d = drill();
        d.set_bridge(bodies::tests::bridge());
        d
    }

    fn walk_until(d: &mut Drill, mut done: impl FnMut(&Drill) -> bool) -> f64 {
        for k in 0..(TICK_HZ as usize * 20) {
            if done(d) {
                return k as f64 * DT;
            }
            d.step();
        }
        panic!("it never happened");
    }

    #[test]
    fn on_foot_a_claim_is_a_walk_and_the_station_is_automated_until_the_body_sits() {
        let mut d = on_foot();
        let a = d.join("Ens. Holt", false).expect("joins");
        d.claim(a, Station::Helm).expect("a walk order");
        assert_eq!(d.operator(Station::Helm), Operator::Auto, "automation holds helm while the body walks");
        assert_eq!(d.body(a).and_then(|b| b.walking_to()), Some(Station::Helm));
        assert_eq!(d.claim(a, Station::Tactical), Err(Refusal::Walking), "one walk at a time");
        let t = walk_until(&mut d, |d| d.operator(Station::Helm) == Operator::Player(a));
        assert!(t > 2.0 && t < 6.0, "the muster point to the helm seat is a few seconds' walk: {t}");
        assert_eq!(d.body(a).map(|b| b.posture), Some(bodies::Posture::Seated));
        // Changing station: up, across, down.
        d.claim(a, Station::Tactical).expect("a walk order");
        assert_eq!(d.operator(Station::Helm), Operator::Auto, "the seat left goes to automation at once");
        let t = walk_until(&mut d, |d| d.operator(Station::Tactical) == Operator::Player(a));
        assert!(t > 1.5 && t < 4.0, "seat to seat across the bridge: {t}");
    }

    #[test]
    fn a_player_relieves_a_bot_and_nobody_walks_into_a_players_seat() {
        let mut d = on_foot();
        let bot = d.join("Lt. Venn (bot)", true).expect("joins");
        let me = d.join("Ens. Holt", false).expect("joins");
        let other = d.join("Ens. Rook", false).expect("joins");
        d.claim(bot, Station::Helm).expect("walks");
        walk_until(&mut d, |d| d.operator(Station::Helm) == Operator::Player(bot));
        d.claim(me, Station::Helm).expect("a player relieves a bot");
        assert!(d.drain_events().iter().any(|e| matches!(e, DrillEvent::Relieved { slot, .. } if *slot == bot)));
        assert_eq!(d.player(bot).and_then(|p| p.station), None, "the bot is up");
        assert_eq!(d.claim(other, Station::Helm), Err(Refusal::StationTaken), "someone is on the way");
        walk_until(&mut d, |d| d.operator(Station::Helm) == Operator::Player(me));
        assert_eq!(d.claim(other, Station::Helm), Err(Refusal::StationTaken), "a player's seat");
        assert_eq!(d.claim(bot, Station::Helm), Err(Refusal::StationTaken), "a bot never relieves a player");
    }

    #[test]
    fn leaving_takes_the_body_off_the_bridge() {
        let mut d = on_foot();
        let a = d.join("Ens. Holt", false).expect("joins");
        assert!(d.body(a).is_some());
        d.leave(a);
        assert!(d.bodies.is_empty());
    }
}
