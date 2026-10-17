//! The co-op drill's combat: one Tern, one enemy, the stations that fly and fight her, and the drill's phases
//! (openspec/changes/coop-drill design sections 1-2).
//!
//! It lives in the core because the server runs it and the client previews with it (the hit chance, the steering
//! a Target order applies), with no clock, socket or GPU (CLAUDE.md 6.2). It steps at the server's fixed tick in
//! `f64`, in the drill's one system frame; randomness is seeded per purpose (CLAUDE.md 6.4); commands are applied
//! in a stable order. Ship-local axes: +X port, +Y dorsal, +Z bow. A ship's attitude maps ship-local to system.

pub mod attitude;
pub mod automation;
pub mod bodies;
pub mod chat;
pub mod damage;
pub mod data;
pub mod fighters;
pub mod patrol;
pub mod seats;

use crate::power;
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
/// Repairs on deck (all-hands 5): the repair games' data (the rules, the Tern's jobs and each game's file) and the
/// jobs open now with who is at them (`repair::Repairs`, the server's checks).
#[derive(Debug, Clone)]
pub struct DeckRepairs {
    /// The games, the rules and the Tern's jobs.
    pub data: crate::repair::data::RepairData,
    /// The jobs open now.
    pub book: crate::repair::Repairs,
}

/// A repair job on deck, open or not: its index in the Tern's jobs, its game, and the damaged load a player works it
/// at (its place), with the state it is repaired from.
#[derive(Debug, Clone, PartialEq)]
pub struct DeckJob {
    /// Its index in `repairs.json`'s jobs.
    pub job: usize,
    /// The damaged load's index (`ShipLayout::loads`), where the job is worked.
    pub load: usize,
    /// Where to stand near, ship frame, metres.
    pub at: DVec3,
    /// The state it is repaired from: the worst of its loads.
    pub state: crate::repair::State,
}

/// The Tern's repair jobs that need a hand (all-hands 5): each job with a load under `nominal` (integrity, percent),
/// at its most damaged load. One rule for the server's jobs and the client's prompts.
pub fn deck_jobs(
    data: &crate::repair::data::RepairData,
    l: &damage::ShipLayout,
    integrity: &[f64],
    nominal: f64,
) -> Vec<DeckJob> {
    use crate::repair::State;
    data.ship
        .jobs
        .iter()
        .enumerate()
        .filter_map(|(k, j)| {
            let (li, v) = j
                .loads
                .iter()
                .filter_map(|id| l.loads.iter().position(|ld| ld.id == *id))
                .filter_map(|li| integrity.get(li).map(|v| (li, *v)))
                .filter(|(_, v)| *v < nominal)
                .min_by(|a, b| a.1.total_cmp(&b.1))?;
            let state = if v <= 0.0 {
                State::Destroyed
            } else if v < data.damage.disabled_to_pct {
                State::Disabled
            } else {
                State::Damaged
            };
            Some(DeckJob { job: k, load: li, at: l.loads[li].centre, state })
        })
        .collect()
}

/// What a player carries on foot (all-hands 4): an extinguisher and the agent left in it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Kit {
    /// The player's slot.
    pub slot: u8,
    /// Seconds of agent left (0: empty, or none carried).
    pub agent_s: f64,
    /// Spraying it now.
    pub spraying: bool,
}

/// The most one walk message may move a body, metres: a ladder's climb or a lift's run between two messages, never a
/// jump across the ship (a guard on the wire, not a rule of the walk: the walk is `sc-core::walk`'s).
pub const WALK_STEP_MAX_M: f32 = 6.0;
/// The most bolts in flight at once; a turret that would spawn past it does not fire.
pub const MAX_BOLTS: usize = 256;
/// The most missiles in flight at once.
pub const MAX_MISSILES: usize = 16;

/// The enemy fighters' ids in every message (enemy-fighters 1).
pub const FIGHTER_IDS: [u16; 2] = [10, 11];
/// The turret the gunner holds, by its mount's id (enemy-fighters 3: the dorsal pod's).
pub const GUNNER_TURRET: &str = "dorsal";
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

/// A bridge station (bridge-stations section 2): the drill's five, the two laser banks' stations (laser-banks 3) and
/// the dorsal turret's gunner (enemy-fighters 3).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Station {
    /// Flies the ship.
    Helm = 0,
    /// Locks, fires and sets the shields.
    Tactical = 1,
    /// Runs the power grid and the reactor's loop.
    Engineering = 2,
    /// Scans, pings, tunes and balances the shields.
    Science = 3,
    /// The condition, orders and the viewscreen.
    Captain = 4,
    /// The port laser bank (laser-banks 3).
    LaserPort = 5,
    /// The starboard laser bank.
    LaserStbd = 6,
    /// The dorsal turret by hand (enemy-fighters 3).
    Gunner = 7,
}

impl Station {
    /// Every station, in seat order.
    pub const ALL: [Station; 8] = [
        Station::Helm,
        Station::Tactical,
        Station::Engineering,
        Station::Science,
        Station::Captain,
        Station::LaserPort,
        Station::LaserStbd,
        Station::Gunner,
    ];
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
            Self::LaserPort => "laser_port",
            Self::LaserStbd => "laser_stbd",
            Self::Gunner => "gunner",
        }
    }
    /// What a screen calls it, in full (ui-art-direction design 12: "Starboard laser", never an abbreviation).
    pub fn name(self) -> &'static str {
        match self {
            Self::Helm => "Helm",
            Self::Tactical => "Tactical",
            Self::Engineering => "Engineering",
            Self::Science => "Science",
            Self::Captain => "Captain",
            Self::LaserPort => "Port laser",
            Self::LaserStbd => "Starboard laser",
            Self::Gunner => "Gunner",
        }
    }
    /// The laser bank a station fires, by its index in the ship's banks.
    pub fn laser_bank(self) -> Option<u8> {
        match self {
            Self::LaserPort => Some(0),
            Self::LaserStbd => Some(1),
            _ => None,
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
    /// A patrol under way with no fight on (patrol-mission): docked, departing, in transit, at a point.
    Patrol = 4,
}

impl Phase {
    /// From its wire byte.
    pub fn from_u8(v: u8) -> Option<Self> {
        [Self::Muster, Self::Countdown, Self::Engage, Self::Debrief, Self::Patrol].get(usize::from(v)).copied()
    }

    /// The crew are at work: the consoles take commands (Engage, or a patrol under way).
    pub fn live(self) -> bool {
        matches!(self, Self::Engage | Self::Patrol)
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
    /// Held by the gunner (enemy-fighters 3): it fires along `manual_aim` while `trigger` is held.
    pub manual: bool,
    /// The gunner's aim, a unit vector in the ship's frame.
    pub manual_aim: DVec3,
    /// The gunner's trigger.
    pub trigger: bool,
    /// Bolts left before it must reload, and the seconds of reload left (0: not reloading).
    pub rounds: u32,
    pub reload_s: f64,
}

/// A laser bank's state (laser-banks 1).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LaserBank {
    /// Its capacitor, MJ.
    pub charge_mj: f64,
    /// Whether its arc holds the locked target.
    pub bearing: bool,
    /// The chance a shot now hits (0-1), the number its console shows.
    pub hit_chance: f64,
    /// Seconds since it last fired.
    pub since_fire_s: f64,
}

/// What a hit came from: it decides what armour does (armour-and-missiles 1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WeaponKind {
    /// A cannon bolt.
    Cannon,
    /// A laser bank's beam.
    Laser,
    /// A missile's blast.
    Missile,
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
    /// Laser banks, port first (laser-banks).
    pub lasers: Vec<LaserBank>,
    /// Countermeasures left (armour-and-missiles 3).
    pub decoys: u32,
    /// Seconds left of the decoy burst around it, if one is out.
    pub decoy_s: f64,
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
    /// Pulled off by a decoy: it flies on blind and destroys itself at the end of its life.
    pub fooled: bool,
    /// Where its body points, a unit vector: it starts as the launcher's bow, its motor pushes along it, its seeker
    /// looks along it, and it turns toward the target at its turn rate while the seeker sees.
    pub heading: DVec3,
    /// Where on the target it aims, in the target's frame: a point on its long axis, drawn at launch along the middle
    /// four fifths of its hull, so hits land from bow to stern and not all amidships (ship-damage 1: parts of the ship).
    pub aim_m: DVec3,
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
    /// The station they last got up from or walked off, for a bot they relieve to take (relieve-bots design 3).
    pub left: Option<Station>,
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
    /// The captain: the camera on the bridge's main viewscreen (an index in [`FEEDS`]) and whether it chases (seen
    /// from behind the ship), as his own console shows it (chase-camera 2).
    MainView {
        /// The camera, an index in [`FEEDS`].
        feed: u8,
        /// Chase on: from behind the ship.
        chase: bool,
        /// The zoom, an index in [`ZOOMS`] (viewscreen-zoom 3).
        zoom: u8,
        /// ORBIT on: his orbit's heading, elevation and distance as three bytes (`exterior::Orbit::quantize`,
        /// chase-orbit design 6); none: off.
        orbit: Option<[u8; 3]>,
    },
    /// The captain: an order to a station, the verb by its index in that station's verbs (`data/stations.json`).
    Order(Station, u8),
    /// A station acknowledges the captain's newest order to it.
    Ack(Station),
    /// A laser station: fire its bank (0 port, 1 starboard).
    LaserFire(u8),
    /// Tactical: throw a countermeasure.
    Decoy,
    /// The gunner: the barrel's direction in the ship's frame, degrees (yaw to port of the bow, pitch up).
    GunnerAim {
        /// Yaw, degrees.
        yaw_deg: f64,
        /// Pitch, degrees.
        pitch_deg: f64,
    },
    /// The gunner: the trigger held or let go.
    GunnerTrigger(bool),
    /// Engineering: the power grid and the reactor's loop (power-grid 2.10).
    Power(PowerCmd),
    /// Engineering: shore power connected (true) or dropped, docked (patrol-mission 4.1).
    Shore(bool),
    /// Helm: open the clamps and back out of the berth (patrol-mission 4.2).
    Undock,
    /// Helm: spool the jump drive toward the patrol's next place (true), or abort the spool (patrol-mission 5).
    Jump(bool),
    /// Science: sweep the point the Tern is at (patrol-mission 5.2).
    Sweep,
    /// The captain: a standing order to the helm in the patrol (patrol-mission 9).
    Patrol(patrol::PatrolOrder),
}

/// Engineering's hands on the power grid (power-grid 5, reactor-cooling 5), as the console mockup's controls.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PowerCmd {
    /// A preset, by its index in [`crate::power::PowerData::presets`].
    Preset(u8),
    /// A group's fader let go at a setpoint.
    Group(u8, f64),
    /// A fader being dragged at a setpoint: the server answers with the solve's own result (none: let go).
    Preview(Option<(u8, f64)>),
    /// A tap on a group's priority.
    Prio(u8),
    /// A group's feed breaker held: open it, or close it.
    Breaker(u8),
    /// The lock beside an open breaker: lock it open, or unlock it.
    Lock(u8),
    /// A switch on the one-line diagram held: a generator's breaker, then a tie's.
    Switch(u8),
    /// SCRAM, confirmed.
    Scram,
    /// The reactor on AUTO (true) or MANUAL.
    ReactorAuto(bool),
    /// The loop on AUTO (true) or MANUAL.
    CoolingAuto(bool),
    /// A lever on the loop ([`LEVERS`]) to a value: a speed, a share, or the makeup valve shut (0) or open (1).
    Lever(u8, f64),
    /// A node's conduit box switched on (true) or off, by its index in `power.json`'s nodes (power-routing 1).
    Box(u8, bool),
    /// A conduit closed (true) or opened at its box, by its index in `power.json`'s conduits.
    Run(u8, bool),
    /// A system swapped to its second feed (true) or back, by its index in `power.json`'s loads.
    Feed(u8, bool),
}

/// The loop's levers, by index: the core pumps' speed, the radiator pumps', the chiller's share, the makeup valve.
pub const LEVERS: [&str; 4] = ["pumps", "radiators", "chiller", "makeup"];

/// The viewscreen's cameras, by index (the consoles' camera pad).
pub const FEEDS: [&str; 6] = ["TARGET", "FWD", "CHASE", "PORT", "AFT", "STBD"];

/// The viewscreen's zooms, times magnified (viewscreen-zoom 1; owner, 2026-10-14: "1x,2x,4x, 10x, 20x").
pub const ZOOMS: [f64; 5] = [1.0, 2.0, 4.0, 10.0, 20.0];

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
            Self::Alert(_) | Self::Brace(_) | Self::Viewscreen(_) | Self::MainView { .. } | Self::Order(..) => {
                s == Station::Captain
            }
            Self::Ack(to) => s == *to,
            Self::LaserFire(b) => s.laser_bank() == Some(*b),
            Self::Decoy => s == Station::Tactical,
            Self::GunnerAim { .. } | Self::GunnerTrigger(_) => s == Station::Gunner,
            Self::Power(_) | Self::Shore(_) => s == Station::Engineering,
            Self::Undock | Self::Jump(_) => s == Station::Helm,
            Self::Sweep => s == Station::Science,
            Self::Patrol(_) => s == Station::Captain,
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
    /// The laser bank holds less than its least charge (laser-banks 1).
    NotCharged = 12,
    /// The system is too damaged to work (ship-damage 4).
    Damaged = 13,
    /// The body is not standing within reach of that seat: walk there (bridge-stations 6).
    TooFar = 14,
    /// The ship is not ready for that: the drive, the umbilical or the jump's cooldown (patrol-mission).
    NotReady = 15,
    /// A mass lock holds the jump drive (flight-and-navigation 7).
    MassLocked = 16,
    /// Only the person in the captain's seat may start the mission (start-mission 2).
    CaptainsCall = 17,
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
            NotCharged,
            Damaged,
            TooFar,
            NotReady,
            MassLocked,
            CaptainsCall,
        ]
        .into_iter()
        .find(|r| *r as u8 == v)
    }
    /// What a console says.
    pub fn text(self) -> &'static str {
        match self {
            Self::NotYourStation => "Not your station",
            Self::StationTaken => "Station taken",
            Self::BadValue => "Out of range",
            Self::NoSuch => "Not on this ship",
            Self::TubeNotReady => "Tube not ready",
            Self::NotLocked => "No lock",
            Self::OutOfCone => "Target off the bow",
            Self::MagazineEmpty => "Magazine empty",
            Self::Full => "Crew is full",
            Self::WrongPhase => "Not at this stage",
            Self::Walking => "On the way",
            Self::TooFar => "Walk to the seat",
            Self::Damaged => "Damaged",
            Self::NotCharged => "Not charged",
            Self::NotReady => "Not ready",
            Self::MassLocked => "Mass lock",
            Self::CaptainsCall => "Captain's call",
        }
    }
}

/// Who may press START MISSION (start-mission 2), from the crew as (slot, station held, bot): a seated person who holds
/// the captain's seat, or any seated person when no person holds it (single player, where an NPC has it). The server
/// applies it in [`Drill::start_mission`]; a client asks it only to decide whether to draw the control.
pub fn may_start(crew: &[(u8, Option<Station>, bool)], slot: u8) -> Result<(), Refusal> {
    let &(_, station, bot) = crew.iter().find(|c| c.0 == slot).ok_or(Refusal::NotYourStation)?;
    if bot || station.is_none() {
        return Err(Refusal::NotYourStation);
    }
    let captain = crew.iter().find(|c| c.1 == Some(Station::Captain) && !c.2);
    match captain {
        Some(c) if c.0 != slot => Err(Refusal::CaptainsCall),
        _ => Ok(()),
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
    /// A laser bank fired (laser-banks 2).
    Beam {
        /// The ship.
        owner: u16,
        /// The bank's index.
        bank: u8,
        /// From the emitter, system frame.
        from: DVec3,
        /// To where it struck, or where it passed, system frame.
        to: DVec3,
        /// Whether it struck.
        hit: bool,
        /// Its energy, MJ.
        energy_mj: f64,
    },
    /// Something burst aboard a ship: a hit past its armour, a system destroyed, a cascade (ship-damage 6).
    Burst {
        /// The ship.
        ship: u16,
        /// Where, system frame.
        pos: DVec3,
        /// Its size: MJ past the armour, or integrity points.
        size: f64,
    },
    /// A ship threw a countermeasure (armour-and-missiles 3).
    DecoyOut {
        /// The ship.
        ship: u16,
    },
    /// A decoy pulled a missile off, or set it off early.
    Fooled {
        /// The missile.
        missile: u16,
        /// Set off early, rather than pulled off.
        early: bool,
    },
    /// A player relieved a bot at a station; the bot is getting up (design 9).
    Relieved {
        /// The bot's slot.
        slot: u8,
        /// The station.
        station: Station,
    },
    /// The drill's own crew said something (crew-chat design 3): automation at an empty seat, damage control or the
    /// computer. The server keeps it and sends it to everyone.
    Said(chat::Line),
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
    /// Laser shots the Tern fired, and of those, hits.
    pub tern_laser_shots: u32,
    /// Its laser hits.
    pub tern_laser_hits: u32,
    /// Laser shots the enemy fired.
    pub enemy_laser_shots: u32,
    /// Its laser hits.
    pub enemy_laser_hits: u32,
    /// Missiles the enemy fired.
    pub enemy_missiles_fired: u32,
    /// The enemy's missiles that did damage.
    pub enemy_missile_hits: u32,
    /// Countermeasures thrown, both ships.
    pub decoys_thrown: u32,
    /// Missiles a decoy fooled, both ways.
    pub missiles_fooled: u32,
    /// The enemy's fighters destroyed.
    pub fighters_down: u32,
}

/// A flight block with its thrust and turning scaled by the drive's capability (ship-damage 4).
fn scaled_flight(f: &data::FlightBlock, k: f64) -> data::FlightBlock {
    let k = k.clamp(0.0, 1.0);
    let mut g = f.clone();
    g.accel_fwd_mps2 *= k;
    g.accel_rev_mps2 *= k;
    g.rate_limit_deg_s = g.rate_limit_deg_s.map(|v| v * k);
    g.ang_accel_deg_s2 = g.ang_accel_deg_s2.map(|v| v * k);
    g
}

/// A tube's system id in a ship's layout: `tube_1` for tube 0.
fn tube_system(i: u8) -> String {
    format!("tube_{}", u32::from(i) + 1)
}

/// A turret's load id in a ship's layout: `turret_dorsal` for the mount `dorsal` (`turret_stbd` for `starboard`).
fn turret_system(mount: &str) -> String {
    format!("turret_{}", if mount == "starboard" { "stbd" } else { mount })
}

/// Whether a laser bank should fire (laser-banks 3): full, bearing, and a good enough shot. The server's automation,
/// the enemy and a bot client all decide by this one rule.
pub fn laser_should_fire(charge_frac: f64, bearing: bool, hit_chance: f64, p: &data::LaserProfile) -> bool {
    bearing && charge_frac >= 1.0 - 1e-6 && hit_chance >= p.full_min_hit_chance
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
    /// The camera on the bridge's main viewscreen: the captain's console's (an index in [`FEEDS`], FWD to start).
    pub main_view: u8,
    /// Whether the main viewscreen's camera chases (from behind the ship), as the captain's console's does.
    pub main_chase: bool,
    /// The main viewscreen's zoom, the captain's console's: an index in [`ZOOMS`] (1x to start).
    pub main_zoom: u8,
    /// The main viewscreen's orbit, the captain's console's (chase-orbit design 6): three bytes, or none (ORBIT off).
    pub main_orbit: Option<[u8; 3]>,
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
            main_view: 1,
            main_chase: false,
            main_zoom: 0,
            main_orbit: None,
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
    /// What the drill's own crew are saying (crew-chat design 3).
    voices: chat::DrillVoices,
    /// The bridge the crew walk on, when the drill has one (design 9); without it a claim seats at once.
    pub on_foot: Option<bodies::Bridge>,
    /// Every crew member's body, by slot order.
    pub bodies: Vec<bodies::CrewBody>,
    /// What the players carry on foot (all-hands 4): an extinguisher and its agent.
    pub kits: Vec<Kit>,
    /// Repairs on deck (all-hands 5): the games' data and the jobs open on the Tern's damaged loads; none until the
    /// server hands the data over.
    pub repairs: Option<DeckRepairs>,
    /// The enemy's fighters (enemy-fighters 1).
    pub fighters: Vec<fighters::Fighter>,
    /// Each ship's insides, the Tern's then the Hound's: armour, systems, fires, teams (ship-damage).
    pub damage: [damage::DamageState; 2],
    /// The captain's raised repair, a system's id (ship-damage 5).
    pub repair_priority: Option<String>,
    /// The Tern's power grid and cooling (power-grid, reactor-cooling).
    pub power: power::PowerState,
    /// A fader being dragged on Engineering, and the solve's answer: group, setpoint, MW wanted and delivered.
    pub power_preview: Option<(u8, f64, f64, f64)>,
    power_ask: Option<(u8, f64)>,
    /// Seconds simulated toward the power grid's next sub-step.
    power_s: f64,
    /// Seconds the bay's crew have held the reactor panel's reset (power-grid 3).
    reset_held_s: f64,
    hits: u64,
    /// The patrol, when the mission is one (patrol-mission).
    pub patrol: Option<patrol::PatrolState>,
    /// Each automated station's patrol decisions' memory, by `Station as usize`.
    patrol_mem: [patrol::PatrolMemory; Station::ALL.len()],
}

fn new_ship(id: u16, side: Side, combat: &CombatBlock, shields: &ShieldBlock, gun: &data::Gun) -> Ship {
    let gun_capacitor_mj = gun.capacitor_mj;
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
                manual: false,
                manual_aim: DVec3::Z,
                trigger: false,
                rounds: gun.rounds,
                reload_s: 0.0,
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
        lasers: combat
            .lasers
            .as_ref()
            .map(|l| {
                l.banks
                    .iter()
                    .map(|_| LaserBank { charge_mj: 0.0, bearing: false, hit_chance: 0.0, since_fire_s: f64::INFINITY })
                    .collect()
            })
            .unwrap_or_default(),
        decoys: combat.decoys.as_ref().map(|d| d.count).unwrap_or(0),
        decoy_s: 0.0,
    }
}

impl Drill {
    /// A drill in Muster, with the ships at their start.
    pub fn new(data: DrillData, seed: u64) -> Self {
        let ships = [
            new_ship(TERN_ID, Side::Tern, &data.tern_combat, &data.tern_shields, data.gun(&data.tern_combat.gun)),
            new_ship(ENEMY_ID, Side::Enemy, &data.enemy.combat, &data.enemy.shields, data.gun(&data.enemy.combat.gun)),
        ];
        let mut damage = [damage::DamageState::new(&data.tern_layout), damage::DamageState::new(&data.hound_layout)];
        // All hands (all-hands 1): no damage-control teams aboard the Tern; the players put it right.
        if data.mission.all_hands() {
            damage[0].teams.clear();
        }
        let power = power::PowerState::new(&data.power);
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
            voices: chat::DrillVoices::default(),
            on_foot: None,
            bodies: Vec::new(),
            kits: Vec::new(),
            repairs: None,
            fighters: Vec::new(),
            damage,
            repair_priority: None,
            power,
            power_preview: None,
            power_ask: None,
            power_s: 0.0,
            reset_held_s: 0.0,
            hits: 0,
            patrol: None,
            patrol_mem: Default::default(),
        };
        d.reset_round();
        d
    }

    fn round_seed(&self) -> u64 {
        Rng::for_purpose(self.seed, u64::from(self.round), "round").next_u64()
    }

    /// Put the ships at their start for a new round.
    fn reset_round(&mut self) {
        let d = &self.data;
        let tern = new_ship(TERN_ID, Side::Tern, &d.tern_combat, &d.tern_shields, d.gun(&d.tern_combat.gun));
        let enemy = new_ship(ENEMY_ID, Side::Enemy, &d.enemy.combat, &d.enemy.shields, d.gun(&d.enemy.combat.gun));
        self.ships = [tern, enemy];
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
        self.voices.new_round();
        self.fighters.clear();
        self.damage =
            [damage::DamageState::new(&self.data.tern_layout), damage::DamageState::new(&self.data.hound_layout)];
        if self.data.mission.all_hands() {
            self.damage[0].teams.clear();
        }
        self.repair_priority = None;
        self.power = power::PowerState::new(&self.data.power);
        self.power_preview = None;
        self.power_ask = None;
        self.power_s = 0.0;
        self.reset_held_s = 0.0;
        self.patrol_start();
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
        self.players.push(Player { slot, name, station: None, ready: false, bot, left: None });
        self.players.sort_by_key(|p| p.slot);
        if let Some(b) = &self.on_foot {
            let mut body = bodies::CrewBody::at_muster(slot, b);
            // A player's body is theirs to walk (the law: one game, played on foot); a bot's follows its paths.
            if !bot {
                body.set_free();
            }
            self.bodies.push(body);
            self.bodies.sort_by_key(|b| b.slot);
        }
        Ok(slot)
    }

    /// Put the crew on foot on `bridge` (design 9): every body to the muster point, every station to automation.
    pub fn set_bridge(&mut self, bridge: bodies::Bridge) {
        self.bodies = self
            .players
            .iter()
            .map(|p| {
                let mut b = bodies::CrewBody::at_muster(p.slot, &bridge);
                if !p.bot {
                    b.set_free();
                }
                b
            })
            .collect();
        for p in &mut self.players {
            p.station = None;
        }
        self.on_foot = Some(bridge);
    }

    /// Slot `slot`'s body.
    pub fn body(&self, slot: u8) -> Option<&bodies::CrewBody> {
        self.bodies.iter().find(|b| b.slot == slot)
    }

    /// A player leaves; their station returns to automation on the next tick.
    pub fn leave(&mut self, slot: u8) {
        self.players.retain(|p| p.slot != slot);
        self.bodies.retain(|b| b.slot != slot);
        self.kits.retain(|k| k.slot != slot);
        self.undock(slot);
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
        if self.on_foot.is_some() {
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
    /// seat and becomes the operator when it sits. Who may take the seat is `seats::claim_rule` (relieve-bots 2): a
    /// person relieves the bots holding it or walking to it, and each goes on to its relief post (design 3).
    fn claim_on_foot(&mut self, slot: u8, s: Station) -> Result<(), Refusal> {
        let Some(bridge) = self.on_foot.clone() else { return Err(Refusal::NoSuch) };
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
        // A player sits only where they walked to: standing within reach of the seat, on its deck.
        if body.free() && bridge.seat_within_reach(body.pos) != Some(s) {
            return Err(Refusal::TooFar);
        }
        let holder = match self.operator(s) {
            Operator::Player(o) if o != slot => self.occupant(o),
            _ => None,
        };
        let walker = self
            .bodies
            .iter()
            .find(|b| b.slot != slot && b.walking_to() == Some(s))
            .and_then(|b| self.occupant(b.slot));
        let rule = seats::claim_rule(me_bot, holder, walker);
        if let seats::ClaimRule::Taken(_) = rule {
            return Err(Refusal::StationTaken);
        }
        // Up from their own station first, so a bot relieved now can swap into it.
        if let Some(p) = self.player_mut(slot) {
            p.left = p.station.or(p.left);
            p.station = None;
        }
        if rule == seats::ClaimRule::Relieve {
            let mut bots: Vec<u8> = [holder, walker].into_iter().flatten().map(|o| o.slot).collect();
            bots.dedup();
            for b in bots {
                self.relieve(b, slot, s, &bridge);
            }
        }
        if let Some(b) = self.bodies.iter_mut().find(|b| b.slot == slot) {
            b.go(bodies::Spot::Seat(s), &bridge);
        }
        Ok(())
    }

    /// Who a slot is, for the seat rule.
    fn occupant(&self, slot: u8) -> Option<seats::Occupant> {
        self.players.iter().find(|p| p.slot == slot).map(|p| seats::Occupant { slot, bot: p.bot })
    }

    /// Bot `bot` relieved of seat `s` by person `by` (relieve-bots 3): it stands, or turns from its walk, and goes to
    /// the station `by` just left when that is unmanned, else the nearest unmanned station, else its muster place.
    fn relieve(&mut self, bot: u8, by: u8, s: Station, bridge: &bodies::Bridge) {
        if let Some(p) = self.player_mut(bot) {
            p.station = None;
        }
        let left = self.player(by).and_then(|p| p.left);
        let from = self.body(bot).map_or([0.0; 3], |b| b.pos);
        let candidates: Vec<Station> = self
            .data
            .mission
            .stations
            .iter()
            .filter_map(|id| Station::from_id(id))
            .filter(|st| *st != s && bridge.has(*st))
            .collect();
        let free = |st: Station| {
            self.operator(st) == Operator::Auto
                && !self.bodies.iter().any(|b| b.slot != bot && b.walking_to() == Some(st))
        };
        let dist = |st: Station| {
            let p = bridge.at(bodies::Spot::Seat(st), None);
            (p[0] - from[0]).hypot(p[2] - from[2]) + (p[1] - from[1]).abs()
        };
        let to = seats::relief_post(left, &candidates, free, dist).map_or(bodies::Spot::Muster, bodies::Spot::Seat);
        if let Some(b) = self.bodies.iter_mut().find(|b| b.slot == bot) {
            b.go(to, bridge);
        }
        self.events.push(DrillEvent::Relieved { slot: bot, station: s });
    }

    /// A player gets up (hold E, bridge-stations 6 and 8.6): the body stands behind the seat and is theirs to walk;
    /// the station goes to automation at once.
    pub fn stand(&mut self, slot: u8) -> Result<(), Refusal> {
        let Some(bridge) = self.on_foot.clone() else { return Err(Refusal::NoSuch) };
        if self.players.iter().any(|p| p.slot == slot && p.bot) {
            return Err(Refusal::NotYourStation);
        }
        let body = self.bodies.iter_mut().find(|b| b.slot == slot).ok_or(Refusal::NotYourStation)?;
        if body.posture != bodies::Posture::Seated {
            return Err(Refusal::Walking);
        }
        body.go(bodies::Spot::Free, &bridge);
        if let Some(p) = self.player_mut(slot) {
            p.left = p.station.or(p.left);
            p.station = None;
        }
        Ok(())
    }

    /// A player's walk moved their body (the client's walk on the walk world, `sc-core::walk`, is where it can go):
    /// taken while the body is theirs to walk and the step is no jump across the ship.
    pub fn walked(&mut self, slot: u8, pos: [f32; 3], yaw: f32) -> Result<(), Refusal> {
        if !(pos.iter().all(|v| v.is_finite()) && yaw.is_finite()) {
            return Err(Refusal::BadValue);
        }
        let body = self.bodies.iter_mut().find(|b| b.slot == slot).ok_or(Refusal::NotYourStation)?;
        if !body.free() {
            return Err(Refusal::Walking);
        }
        let d = pos.iter().zip(body.pos).map(|(a, b)| (a - b) * (a - b)).sum::<f32>().sqrt();
        if d > WALK_STEP_MAX_M {
            return Err(Refusal::BadValue);
        }
        body.walked(pos, yaw);
        Ok(())
    }

    /// A player takes an extinguisher (all-hands 4, safety-points 5a), standing within a seat's reach of the
    /// damage-control locker, which never empties, or of a full bracket, which is empty for `fire.bracket_restock_s`
    /// after. An empty bracket alone in reach gives nothing.
    pub fn take_extinguisher(&mut self, slot: u8) -> Result<(), Refusal> {
        let body = self.bodies.iter().find(|b| b.slot == slot).ok_or(Refusal::NotYourStation)?;
        if !body.free() {
            return Err(Refusal::Walking);
        }
        let (p, reach) = (body.pos, self.data.stations.seats.reach_m);
        let dist = |l: &DVec3| (l.x - f64::from(p[0])).hypot(l.z - f64::from(p[2]));
        let near = |l: &DVec3| dist(l) <= reach && (l.y - f64::from(p[1])).abs() < 1.5;
        if !self.data.tern_layout.lockers.iter().any(near) {
            let layout = &self.data.tern_layout;
            let in_reach: Vec<usize> = (0..layout.brackets.len()).filter(|k| near(&layout.brackets[*k].wall)).collect();
            if in_reach.is_empty() {
                return Err(Refusal::TooFar);
            }
            let full = in_reach
                .into_iter()
                .filter(|k| self.damage[0].bracket_full(*k))
                .min_by(|a, b| dist(&layout.brackets[*a].wall).total_cmp(&dist(&layout.brackets[*b].wall)))
                .ok_or(Refusal::NoSuch)?;
            self.damage[0].brackets[full] = self.data.damage.fire.bracket_restock_s;
        }
        let agent_s = self.data.damage.fire.extinguisher_s;
        match self.kits.iter_mut().find(|k| k.slot == slot) {
            Some(k) => *k = Kit { slot, agent_s, spraying: false },
            None => self.kits.push(Kit { slot, agent_s, spraying: false }),
        }
        Ok(())
    }

    /// Hand the drill the repair games' data: from now jobs open on the Tern's damaged loads and players can work them.
    pub fn set_repairs(&mut self, data: crate::repair::data::RepairData) {
        self.repairs = Some(DeckRepairs { data, book: crate::repair::Repairs::default() });
    }

    /// Open a job on each load that needs one, and close a job whose loads are all back (all-hands 5).
    fn open_repair_jobs(&mut self) {
        let nominal = self.data.damage.systems.nominal_from_pct;
        let Some(r) = self.repairs.as_mut() else { return };
        let need = deck_jobs(&r.data, &self.data.tern_layout, &self.damage[0].integrity, nominal);
        let ids: Vec<String> = need.iter().map(|j| r.data.ship.jobs[j.job].id.clone()).collect();
        r.book.jobs.retain(|s| ids.contains(&s.id) && !s.job.done());
        for (j, id) in need.iter().zip(&ids) {
            if r.book.job(id).is_some() {
                continue;
            }
            let sj = &r.data.ship.jobs[j.job];
            let Some(file) = r.data.games.get(&sj.game) else { continue };
            let job = crate::repair::Job::for_game(file, &r.data.rules, &r.data.damage, j.state);
            r.book.open(id, file.clone(), sj.workers, job, j.state);
        }
    }

    /// A player docks at a repair job (all-hands 5): on foot, within reach of its damaged load; the round to play.
    pub fn dock(&mut self, slot: u8, job: usize) -> Result<crate::repair::Order, Refusal> {
        self.open_repair_jobs();
        let nominal = self.data.damage.systems.nominal_from_pct;
        let body = self.bodies.iter().find(|b| b.slot == slot).ok_or(Refusal::NotYourStation)?;
        if !body.free() {
            return Err(Refusal::Walking);
        }
        let p = DVec3::new(f64::from(body.pos[0]), f64::from(body.pos[1]), f64::from(body.pos[2]));
        let r = self.repairs.as_mut().ok_or(Refusal::NoSuch)?;
        let (reach, dy) = (r.data.ship.reach_m, r.data.ship.reach_dy_m);
        let need = deck_jobs(&r.data, &self.data.tern_layout, &self.damage[0].integrity, nominal);
        let j = need.iter().find(|j| j.job == job).ok_or(Refusal::NoSuch)?;
        let near = self.data.tern_layout.loads.iter().enumerate().any(|(li, ld)| {
            r.data.ship.jobs[job].loads.contains(&ld.id)
                && self.damage[0].integrity.get(li).is_some_and(|v| *v < nominal)
                && (ld.centre.x - p.x).hypot(ld.centre.z - p.z) <= reach
                && (ld.centre.y - p.y).abs() <= dy
        });
        if !near {
            return Err(Refusal::TooFar);
        }
        let id = r.data.ship.jobs[j.job].id.clone();
        r.book.dock(u32::from(slot), &id, true, u32::MAX, &r.data.damage).map_err(|e| match e {
            crate::repair::Refusal::Full => Refusal::StationTaken,
            _ => Refusal::NoSuch,
        })
    }

    /// A player landed a round of a job they are docked at, played in `play_s` seconds: the server's checks (the round
    /// given, no faster than any hand), then each damaged load of the job up a share; the next round, if any is left.
    pub fn round_done(
        &mut self,
        slot: u8,
        job: usize,
        round: u32,
        play_s: f64,
    ) -> Result<Option<crate::repair::Order>, Refusal> {
        let nominal = self.data.damage.systems.nominal_from_pct;
        let r = self.repairs.as_mut().ok_or(Refusal::NoSuch)?;
        let sj = r.data.ship.jobs.get(job).ok_or(Refusal::NoSuch)?.clone();
        let next = r.book.round_done(u32::from(slot), &sj.id, round, play_s).map_err(|_| Refusal::BadValue)?;
        let share = r.book.job(&sj.id).map_or(0.0, |s| s.job.share());
        for (li, ld) in self.data.tern_layout.loads.iter().enumerate() {
            if sj.loads.contains(&ld.id) {
                if let Some(v) = self.damage[0].integrity.get_mut(li) {
                    if *v < nominal {
                        *v = (*v + share).min(nominal);
                    }
                }
            }
        }
        self.open_repair_jobs();
        // Its loads back to nominal, the job closes whatever rounds its game had left: the player is done.
        let open = self.repairs.as_ref().is_some_and(|r| r.book.job(&sj.id).is_some());
        Ok(next.filter(|_| open))
    }

    /// A player stands up from a repair; the round they held goes back to the list.
    pub fn undock(&mut self, slot: u8) {
        if let Some(r) = self.repairs.as_mut() {
            r.book.undock(u32::from(slot));
        }
    }

    /// A player starts or stops spraying their extinguisher: on foot, with agent left.
    pub fn spray(&mut self, slot: u8, on: bool) -> Result<(), Refusal> {
        let free = self.bodies.iter().any(|b| b.slot == slot && b.free());
        let k = self.kits.iter_mut().find(|k| k.slot == slot && k.agent_s > 0.0).ok_or(Refusal::NoSuch)?;
        if on && !free {
            return Err(Refusal::Walking);
        }
        k.spraying = on;
        Ok(())
    }

    /// The Tern's rooms where a player is spraying an extinguisher now, one entry a player: where the body's chest is.
    fn sprays(&self) -> Vec<usize> {
        let rooms = &self.data.tern_layout.rooms;
        self.kits
            .iter()
            .filter(|k| k.spraying && k.agent_s > 0.0)
            .filter_map(|k| self.bodies.iter().find(|b| b.slot == k.slot && b.free()))
            .filter_map(|b| {
                let chest = DVec3::new(f64::from(b.pos[0]), f64::from(b.pos[1]) + 1.2, f64::from(b.pos[2]));
                rooms.iter().position(|r| r.contains(chest))
            })
            .collect()
    }

    fn step_bodies(&mut self) {
        let Some(bridge) = self.on_foot.as_ref() else { return };
        let mut sat = Vec::new();
        for b in &mut self.bodies {
            if let Some(s) = b.step(bridge, DT as f32) {
                sat.push((b.slot, s));
            }
        }
        // On foot a person musters by sitting at their station: the briefing's READY is not on screen there, so a
        // person who sat in Muster waited for a READY they could not press (browser-page-fit design 6).
        let muster = self.phase == Phase::Muster;
        for (slot, s) in sat {
            if let Some(p) = self.players.iter_mut().find(|p| p.slot == slot) {
                p.station = Some(s);
                if muster && !p.bot {
                    p.ready = true;
                }
            }
        }
    }

    /// A player is ready (or not) to begin.
    pub fn set_ready(&mut self, slot: u8, ready: bool) -> Result<(), Refusal> {
        let p = self.player_mut(slot).ok_or(Refusal::NotYourStation)?;
        p.ready = ready;
        Ok(())
    }

    /// START MISSION (start-mission 2): in Muster, a seated person who may start ([`may_start`]) moves the drill to
    /// Countdown at once, whoever else is ready or seated. Anything else is refused and changes nothing.
    pub fn start_mission(&mut self, slot: u8) -> Result<(), Refusal> {
        if self.phase != Phase::Muster {
            return Err(Refusal::WrongPhase);
        }
        let crew: Vec<(u8, Option<Station>, bool)> = self.players.iter().map(|p| (p.slot, p.station, p.bot)).collect();
        may_start(&crew, slot)?;
        self.set_phase(Phase::Countdown);
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
        self.step_bodies();
        self.power_tick();
        // Commands, by station, then arrival (CLAUDE.md 6.4).
        let mut queue = std::mem::take(&mut self.queue);
        queue.sort_by_key(|q| (q.0, q.1));
        if self.phase.live() {
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
                // On foot the round waits for every person aboard to reach a seat: it once began while the player
                // was still walking up from their muster place (browser-page-fit design 6).
                let people_seated =
                    self.on_foot.is_none() || self.players.iter().filter(|p| !p.bot).all(|p| p.station.is_some());
                if !seated.is_empty() && seated.iter().all(|p| p.ready) && people_seated {
                    self.set_phase(Phase::Countdown);
                }
            }
            Phase::Countdown => {
                if !self.players.iter().any(|p| p.station.is_some()) {
                    self.set_phase(Phase::Muster);
                } else if self.phase_s >= self.data.mission.countdown_s {
                    if self.patrol.is_some() {
                        // A patrol starts in the berth; the enemy waits at its point (patrol-mission 5.3).
                        self.set_phase(Phase::Patrol);
                    } else {
                        self.ships[1].active = true;
                        self.set_phase(Phase::Engage);
                    }
                }
            }
            Phase::Patrol => {
                self.patrol_automation();
                self.bridge_tick();
                self.patrol_tick();
                self.damage_tick();
            }
            Phase::Engage => {
                self.run_automation();
                self.bridge_tick();
                self.engage_tick();
                self.computer_reports();
                self.stats.engage_s += DT;
                // In a patrol the jump drive works in a fight too: it is how the Tern escapes (patrol-mission 5.3).
                if self.patrol.is_some() {
                    self.patrol_automation();
                    self.patrol_drive();
                }
                let ended = if !self.ships[1].alive {
                    Some(Outcome::Victory)
                } else if !self.ships[0].alive {
                    Some(Outcome::Defeat)
                } else if self.phase_s >= self.data.mission.engage_limit_s {
                    Some(Outcome::Withdrew)
                } else {
                    None
                };
                match (ended, self.patrol.is_some()) {
                    // The patrol goes on after its fight: the outcome waits for the jump home.
                    (Some(Outcome::Victory | Outcome::Withdrew), true) => {
                        self.ships[1].active = false;
                        self.set_phase(Phase::Patrol);
                    }
                    (Some(o), _) => {
                        self.outcome = o;
                        self.set_phase(Phase::Debrief);
                    }
                    (None, _) => {}
                }
            }
            Phase::Debrief => {
                // The teams keep working after the fight (ship-damage 9): the next drill waits for them.
                self.damage_tick();
                let m = &self.data.mission;
                if self.phase_s >= m.debrief_s && (self.put_right() || self.phase_s >= m.debrief_max_s) {
                    self.round += 1;
                    self.reset_round();
                    for p in &mut self.players {
                        p.ready = false;
                    }
                    self.set_phase(Phase::Muster);
                }
            }
        }
        self.speak();
    }

    /// The drill's clock: seconds since it began, the game time a chat line carries (crew-chat design 1).
    pub fn clock_s(&self) -> f64 {
        self.tick as f64 * DT
    }

    /// The computer's reports (crew-chat design 3): the hull and the shields falling past a report step, a new breach,
    /// the enemy destroyed. Each once a round.
    fn computer_reports(&mut self) {
        let now = self.clock_s();
        let d = &self.data;
        let (chat, lim) = (&d.chat, &d.chat.limits);
        let t = &self.ships[0];
        let v = &mut self.voices;
        let hull_pct = (100.0 * t.hull / d.tern_combat.hull_mj).max(0.0);
        if let Some(i) = chat::step_reached(&chat.report_steps_pct, v.hull_step, hull_pct) {
            v.hull_step = i + 1;
            if let Some(x) = chat.line("hull", &[("pct", &format!("{:.0}", hull_pct.floor()))]) {
                v.computer.offer(x, now, lim);
            }
        }
        let sh = &d.tern_shields;
        let caps: f64 = sh.presets[usize::from(t.preset).min(sh.presets.len() - 1)].faces_mj.iter().sum();
        let shield_pct = (100.0 * t.faces.iter().sum::<f64>() / caps.max(1e-9)).max(0.0);
        if let Some(i) = chat::step_reached(&chat.report_steps_pct, v.shield_step, shield_pct) {
            v.shield_step = i + 1;
            if let Some(x) = chat.line("shields_low", &[("pct", &format!("{:.0}", shield_pct.floor()))]) {
                v.computer.offer(x, now, lim);
            }
        }
        let breaches = &self.damage[0].breaches;
        if breaches.len() > v.breaches {
            v.breaches = breaches.len();
            let room = breaches.last().and_then(|b| d.tern_layout.rooms.get(b.room)).map(|r| chat.room(&r.id));
            if let Some(x) = room.and_then(|r| chat.line("breach", &[("room", &r)])) {
                v.computer.offer(x, now, lim);
            }
        }
        if !self.ships[1].alive && !v.killed {
            v.killed = true;
            if let Some(x) = chat.line("target_destroyed", &[]) {
                v.computer.offer(x, now, lim);
            }
        }
    }

    /// Every voice of the drill's own crew whose line is due says it, as a [`DrillEvent::Said`] (crew-chat design 3
    /// and 4). A station a crew member took over says nothing more as automation.
    fn speak(&mut self) {
        let now = self.clock_s();
        for s in Station::ALL {
            if self.operator(s) != Operator::Auto {
                self.voices.stations[s as usize].clear();
            }
        }
        let chat = &self.data.chat;
        let v = &mut self.voices;
        let mut said = Vec::new();
        for s in Station::ALL {
            if let Some(x) = v.stations[s as usize].take(now, &chat.limits) {
                said.push((s.name().to_owned(), Some(s), x));
            }
        }
        if let Some(x) = v.damage_control.take(now, &chat.limits) {
            said.push((chat.speakers.damage_control.clone(), None, x));
        }
        if let Some(x) = v.computer.take(now, &chat.limits) {
            said.push((chat.speakers.computer.clone(), None, x));
        }
        for (name, station, x) in said {
            let line = chat::Line { slot: None, name, station, npc: true, kind: x.kind, text: x.text, at_s: now };
            self.events.push(DrillEvent::Said(line));
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
                Station::Engineering => {
                    let mut c = automation::engineering(
                        &self.bridge,
                        &self.data,
                        &profile.engineering,
                        &mut self.auto.engineering,
                        t,
                    );
                    c.extend(self.engineering_reroute(profile.engineering.reaction_s, t));
                    c
                }
                // Science's automation is its passive scan (in the tick).
                Station::Science => Vec::new(),
                // A laser station fires its bank by the rule the enemy uses (laser-banks 3).
                Station::LaserPort | Station::LaserStbd => {
                    let bank = s.laser_bank().map_or(0, usize::from);
                    if self.laser_wanted(0, bank, &profile.lasers) {
                        vec![Command::LaserFire(bank as u8)]
                    } else {
                        Vec::new()
                    }
                }
                // Unmanned, the gunner's turret is on AUTO: it lets go of the hand.
                Station::Gunner => {
                    if let Some(k) = self.gunner_turret() {
                        let tr = &mut self.ships[0].turrets[k];
                        tr.manual = false;
                        tr.trigger = false;
                    }
                    Vec::new()
                }
            };
            // Tactical throws a countermeasure by the rule the enemy uses (armour-and-missiles 3).
            if s == Station::Tactical && self.decoy_wanted(0) {
                cmds.push(Command::Decoy);
            }
            // Every automation acknowledges its orders once it has had its reaction time.
            let react = profile.reaction_s(s);
            if self.bridge.orders.iter().any(|o| o.to == s && !o.done && t - o.at_s >= react) {
                cmds.push(Command::Ack(s));
            }
            // What it does, it says (crew-chat design 3), by the mapping a bot uses.
            let now = self.clock_s();
            for c in &cmds {
                let said = match c {
                    Command::Ack(st) => self
                        .bridge
                        .orders
                        .iter()
                        .find(|o| o.to == *st && !o.done)
                        .and_then(|o| chat::said_ack(&self.data, *st, o.verb)),
                    _ => chat::said_for(&self.data, s, c),
                };
                if let Some(x) = said {
                    self.voices.stations[s as usize].offer(x, now, &self.data.chat.limits);
                }
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
        if matches!(cmd, Command::Shore(_) | Command::Undock | Command::Jump(_) | Command::Sweep | Command::Patrol(_)) {
            return self.apply_patrol(cmd);
        }
        if let Some(r) = self.apply_bridge(cmd) {
            return r;
        }
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
            Command::Load(i) => return self.load_tube(0, i),
            Command::Fire(i) => return self.fire_tube(0, i),
            Command::LaserFire(b) => return self.fire_laser(0, b),
            Command::Decoy => return self.throw_decoy(0),
            Command::GunnerAim { yaw_deg, pitch_deg } => {
                let k = self.gunner_turret().ok_or(Refusal::NoSuch)?;
                let (y, p) = (yaw_deg.to_radians(), pitch_deg.to_radians());
                let t = &mut self.ships[0].turrets[k];
                t.manual = true;
                t.manual_aim = DVec3::new(y.sin() * p.cos(), p.sin(), y.cos() * p.cos());
            }
            Command::GunnerTrigger(on) => {
                let k = self.gunner_turret().ok_or(Refusal::NoSuch)?;
                let t = &mut self.ships[0].turrets[k];
                t.manual = true;
                t.trigger = on;
            }
            Command::Power(p) => return self.apply_power(p),
            _ => {}
        }
        Ok(())
    }

    /// Engineering's command on the grid (the console mockup's `act`, `holdDone` and `dragTo`).
    fn apply_power(&mut self, cmd: PowerCmd) -> Result<(), Refusal> {
        use power::{Breaker, Mode};
        let d = &self.data.power;
        let p = &mut self.power;
        let groups = d.file.groups.len();
        let group = |g: u8| (usize::from(g) < groups).then_some(usize::from(g)).ok_or(Refusal::NoSuch);
        let ok = |done: bool| if done { Ok(()) } else { Err(Refusal::NoSuch) };
        match cmd {
            PowerCmd::Preset(i) => {
                let name = *d.presets().get(usize::from(i)).ok_or(Refusal::NoSuch)?;
                ok(p.apply_preset(d, name))
            }
            PowerCmd::Group(g, sp) => {
                let g = group(g)?;
                p.set_group(d, g, sp.min(d.group_max(g)));
                self.power_ask = None;
                self.power_preview = None;
                Ok(())
            }
            PowerCmd::Preview(ask) => {
                if let Some((g, _)) = ask {
                    group(g)?;
                }
                self.power_ask = ask;
                if ask.is_none() {
                    self.power_preview = None;
                }
                Ok(())
            }
            PowerCmd::Prio(g) => ok(p.cycle_priority(d, group(g)?)),
            PowerCmd::Breaker(g) => {
                let g = group(g)?;
                let next = if p.group_breaker[g] == Breaker::Closed { Breaker::Open } else { Breaker::Closed };
                ok(p.set_group_breaker(d, g, next))
            }
            PowerCmd::Lock(g) => {
                let g = group(g)?;
                match p.group_breaker[g] {
                    Breaker::Closed => Err(Refusal::NoSuch),
                    Breaker::Open => ok(p.set_group_breaker(d, g, Breaker::Locked)),
                    Breaker::Locked => ok(p.set_group_breaker(d, g, Breaker::Open)),
                }
            }
            PowerCmd::Switch(k) => ok(p.toggle_switch(d, usize::from(k))),
            PowerCmd::Scram => {
                if p.reactor.state != power::ReactorState::Running {
                    return Err(Refusal::WrongPhase);
                }
                p.scram(d, power::ScramCause::Manual);
                Ok(())
            }
            PowerCmd::ReactorAuto(auto) => {
                p.set_reactor(d, if auto { Mode::Auto } else { Mode::Manual }, None);
                Ok(())
            }
            PowerCmd::CoolingAuto(auto) => {
                p.set_cooling(d, Some(if auto { Mode::Auto } else { Mode::Manual }), None, None, None, None);
                Ok(())
            }
            PowerCmd::Box(k, on) => ok(p.set_box(usize::from(k), on)),
            PowerCmd::Run(k, closed) => ok(p.set_conduit(usize::from(k), closed)),
            PowerCmd::Feed(k, alt) => ok(p.set_feed(d, usize::from(k), alt)),
            PowerCmd::Lever(k, v) => {
                // A hand on a lever takes the loop from its automation (reactor-cooling design 5).
                let m = Some(Mode::Manual);
                match k {
                    0 => p.set_cooling(d, m, Some(v), None, None, None),
                    1 => p.set_cooling(d, m, None, Some(v), None, None),
                    2 => p.set_cooling(d, m, None, None, Some(v), None),
                    _ => p.set_cooling(d, m, None, None, None, Some(v >= 0.5)),
                }
                Ok(())
            }
        }
    }

    /// Engineering's automation routing round a cut (power-routing 3): once a spare run would bring power back to a
    /// dark room, for its reaction time, it closes it.
    fn engineering_reroute(&mut self, reaction_s: f64, t: f64) -> Option<Command> {
        let s = &self.damage[0];
        // A system whose panel is dark swaps to its other feed first: it costs no live run.
        let swap = self.power.transfer(&self.data.power, &s.node_health, &s.conduits);
        let k = if swap.is_some() { None } else { self.power.reroute(&self.data.power, &s.node_health, &s.conduits) };
        let cmd = |k: usize| Command::Power(PowerCmd::Run(k as u8, true));
        let act = swap.map(|i| Command::Power(PowerCmd::Feed(i as u8, !self.power.on_alt[i]))).or(k.map(cmd));
        let mem = &mut self.auto.engineering;
        match act {
            None => {
                mem.reroute_since = None;
                None
            }
            Some(c) => {
                let since = *mem.reroute_since.get_or_insert(t);
                (t - since >= reaction_s).then(|| {
                    mem.reroute_since = None;
                    c
                })
            }
        }
    }

    /// What the fight asks of each load (power-grid 7, the console mockup's `activity`): the push and the turn the
    /// helm asks for, shields regenerating, turrets firing, banks charging, tubes loading, the sensors pinging or
    /// scanning. The helm's request, not the push it got, so a starved drive keeps asking and the console shows the
    /// shortfall. The air, comms and the bays are not simulated in the drill: they ask for nothing.
    fn power_activity(&self) -> std::collections::BTreeMap<String, f64> {
        let mut a = std::collections::BTreeMap::new();
        if self.phase != Phase::Engage || !self.ships[0].alive {
            return a;
        }
        let t = &self.ships[0];
        let on = |b: bool| f64::from(u8::from(b));
        let fl = &self.data.tern_flight;
        let want = (t.speed_set.clamp(fl.speed_min_mps, fl.speed_max_mps) - t.vel.dot(t.forward())) / fl.tau_v_s;
        let push = (want.abs() / if want >= 0.0 { fl.accel_fwd_mps2 } else { fl.accel_rev_mps2 }).min(1.0);
        a.insert("drive".to_owned(), push);
        a.insert("dampers".to_owned(), push);
        a.insert("rcs".to_owned(), t.stick.iter().fold(0.0_f64, |m, v| m.max(v.abs())).min(1.0));
        let sh = &self.data.tern_shields;
        let caps = sh.presets[usize::from(t.preset).min(sh.presets.len() - 1)].faces_mj;
        a.insert("shield_regen".to_owned(), on(t.faces.iter().zip(caps).any(|(f, c)| *f < c - 1e-9)));
        let gun = self.data.gun(&self.data.tern_combat.gun);
        for (m, tr) in self.data.tern_combat.turrets.iter().zip(&t.turrets) {
            a.insert(turret_system(&m.id), on(tr.since_fire_s < 1.0 / gun.rate_hz.max(1e-9) + DT));
        }
        if let Some(l) = &self.data.tern_combat.lasers {
            let w = self.data.laser(&l.laser);
            for (m, b) in l.banks.iter().zip(&t.lasers) {
                a.insert(m.id.clone(), on(b.charge_mj < w.capacitor_mj - 1e-9));
            }
        }
        let loading = |s: &Tube| s.state == TubeState::Loading;
        a.insert("hoist".to_owned(), on(t.tubes.iter().any(loading)));
        for (i, tb) in t.tubes.iter().enumerate() {
            a.insert(tube_system(i as u8), on(loading(tb)));
        }
        a.insert("sensors_active".to_owned(), on(self.bridge.ping_s > 0.0 || self.bridge.scanning));
        a
    }

    /// The power grid's sub-steps due this tick (power-grid 2), the fader's preview, and the bay crew at the reactor
    /// panel: once the reset would be taken and a train can carry the reactor, they hold it for its time plus
    /// Engineering's reaction and reset; while neither train can, they call the repair teams to the reactor.
    fn power_tick(&mut self) {
        self.power_s += DT;
        let act = self.power_activity();
        let d = &self.data.power;
        let dt = d.dt();
        let hold = d.file.reactor.reset.hold_s + self.data.profile("automation").engineering.reaction_s;
        while self.power_s >= dt - 1e-9 {
            self.power_s -= dt;
            let s = &mut self.damage[0];
            let mut ship = power::Ship {
                integrity: &mut s.integrity,
                node_health: &s.node_health,
                conduits: &s.conduits,
                activity: &act,
            };
            self.power.step(d, &mut ship);
            // A run open at its box carries no surge either (power-routing 1): the damage state mirrors the switches.
            for (c, closed) in s.conduits.iter_mut().zip(&self.power.conduit_breaker) {
                c.open = !closed;
            }
            let scrammed = self.power.reactor.state == power::ReactorState::Scrammed;
            let hold_up = self.power.trains_hold(d, &s.integrity);
            if scrammed && !hold_up {
                self.repair_priority = Some(d.file.reactor.system.clone());
            }
            if scrammed && hold_up && self.power.reset_refusal(d).is_none() {
                self.reset_held_s += dt;
                if self.reset_held_s >= hold {
                    let _ = self.power.scram_reset(d);
                    self.reset_held_s = 0.0;
                }
            } else {
                self.reset_held_s = 0.0;
            }
            if let Some((g, sp)) = self.power_ask {
                let ship = power::Ship {
                    integrity: &mut s.integrity,
                    node_health: &s.node_health,
                    conduits: &s.conduits,
                    activity: &act,
                };
                let (want, got) = self.power.preview_group(d, &ship, usize::from(g), sp);
                self.power_preview = Some((g, sp, want, got));
            }
        }
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
            Command::MainView { feed, chase, zoom, orbit } => {
                b.main_view = feed;
                b.main_chase = chase;
                b.main_zoom = zoom;
                // An orbit is a chase camera's: none without CHASE.
                b.main_orbit = orbit.filter(|_| chase);
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
        // The captain's REPAIR to Engineering raises the worst-damaged system for the teams (ship-damage 5).
        if let (Command::Order(Station::Engineering, verb), Ok(())) = (cmd, r) {
            let repair =
                self.data.order_verbs(Station::Engineering).get(usize::from(verb)).is_some_and(|v| v[0] == "REPAIR");
            if repair {
                self.repair_priority = self.worst_system(0);
            }
        }
        Some(r)
    }

    /// Ship `si`'s most damaged system below nominal, by its system id (a load's own id where it names none).
    fn worst_system(&self, si: usize) -> Option<String> {
        let l = self.layout(si);
        let nominal = self.data.damage.systems.nominal_from_pct;
        let s = &self.damage[si];
        (0..l.loads.len())
            .filter(|&i| s.integrity[i] < nominal)
            .min_by(|&a, &b| s.integrity[a].total_cmp(&s.integrity[b]))
            .map(|i| if l.loads[i].system.is_empty() { l.loads[i].id.clone() } else { l.loads[i].system.clone() })
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
        // A tube armed and a lock held: the nose on the Tern for the seeker (armour-and-missiles 5), as the Tern's
        // helm does for its tubes.
        let missile_run = enemy.tubes.iter().any(|t| t.state == TubeState::Armed) && enemy.magazine + 2 > 0;
        let (desired, speed) = if range > ai.orbit_m * 1.3 || missile_run {
            (rh, ai.approach_speed_mps.min(ai.orbit_speed_mps * 1.5))
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
        let waypoint = self.waypoint();
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
        // A damaged drive pushes and turns with what is left of it (ship-damage 4).
        let tf = scaled_flight(&self.data.tern_flight, self.capability(0, "impulse_drive"));
        let ef = scaled_flight(&self.data.enemy.flight, self.capability(1, "impulse_drive"));
        if self.ships[0].alive {
            fly(&mut self.ships[0], &tf, DT);
        }
        if self.ships[1].alive && self.ships[1].active {
            fly(&mut self.ships[1], &ef, DT);
        }
        self.locks_and_tubes();
        self.lasers_tick();
        self.enemy_weapons();
        self.shields();
        for (shooter, target) in [(0usize, 1usize), (1, 0)] {
            self.turrets(shooter, target);
        }
        self.fighters_tick();
        self.bolts_step();
        self.missiles_step();
        self.damage_tick();
    }

    /// The enemy's fighters (enemy-fighters 1, 4): launched from a working hangar, flown by their AI at the Tern's
    /// turrets and laser banks; their shots become bolts with the enemy's aim error.
    fn fighters_tick(&mut self) {
        let fd = self.data.fighter.clone();
        let hound_up = self.ships[1].alive && self.ships[1].active;
        if self.fighters.is_empty()
            && hound_up
            && self.phase_s >= fd.launch.after_engage_s
            && self.capability(1, "hangar_launch") > 0.0
        {
            let h = &self.ships[1];
            let m = fighters::Mothership { pos: h.pos, rot: h.rot, vel: h.vel };
            self.fighters = fighters::launch(&m, &fd, FIGHTER_IDS);
        }
        if self.fighters.iter().all(|f| !f.alive) {
            return;
        }
        let t = &self.ships[0];
        let mut parts: Vec<DVec3> =
            self.data.tern_combat.turrets.iter().map(|m| DVec3::from_array(m.position_m)).collect();
        if let Some(l) = &self.data.tern_combat.lasers {
            parts.extend(l.banks.iter().map(|b| DVec3::from_array(b.position_m)));
        }
        let prey = fighters::Prey { pos: t.pos, rot: t.rot, vel: t.vel, parts: &parts };
        let range = self.data.tern_combat.fighter_range_m;
        let threatened: Vec<bool> = self.fighters.iter().map(|f| (f.pos - prey.pos).length() <= range).collect();
        let shots = fighters::step(&mut self.fighters, &fd, &prey, &threatened, DT, &mut self.rng);
        let sigma = self.data.enemy.combat.aim_sigma_base_deg.to_radians();
        for shot in shots {
            if self.bolts.len() >= MAX_BOLTS {
                break;
            }
            let from = self.fighters.iter().find(|f| f.id == shot.owner).map_or(DVec3::ZERO, |f| f.vel);
            let gun = shot.vel - from;
            let dir = scatter(&mut self.rng, gun.normalize_or(DVec3::Z), sigma);
            let id = self.next_bolt;
            self.next_bolt = self.next_bolt.wrapping_add(1).max(1);
            let bolt = Bolt {
                id,
                owner: shot.owner,
                pos: shot.pos,
                vel: from + dir * gun.length(),
                life_s: shot.life_s,
                damage_mj: shot.damage_mj,
            };
            self.events.push(DrillEvent::Fired { bolt: id, owner: bolt.owner, pos: bolt.pos, vel: bolt.vel });
            self.stats.enemy_shots += 1;
            self.bolts.push(bolt);
        }
    }

    /// Fires burn and spread, teams travel and repair, for both ships (ship-damage 5).
    fn damage_tick(&mut self) {
        self.open_repair_jobs();
        let tern_sprays = self.sprays();
        for k in &mut self.kits {
            if k.spraying {
                k.agent_s = (k.agent_s - DT).max(0.0);
                k.spraying = k.agent_s > 0.0;
            }
        }
        for si in 0..2 {
            if !(self.ships[si].alive && self.ships[si].active) {
                continue;
            }
            let l = if si == 0 { &self.data.tern_layout } else { &self.data.hound_layout };
            let priority = if si == 0 { self.repair_priority.as_deref() } else { None };
            // The Tern's players spraying an extinguisher in a room: a hand each at its fire (all-hands 4).
            let sprays = if si == 0 { tern_sprays.as_slice() } else { &[] };
            let ev = damage::tick(l, &mut self.damage[si], &self.data.damage, DT, priority, sprays);
            for e in ev {
                if let damage::DamageEvent::HullBurned { mj } = e {
                    self.hull_loss(si, mj);
                } else if si == 0 {
                    // Damage control says what its teams do (crew-chat design 3).
                    if let Some(x) = chat::said_damage(&self.data, &self.data.tern_layout, &e) {
                        let now = self.clock_s();
                        self.voices.damage_control.offer(x, now, &self.data.chat.limits);
                    }
                }
            }
        }
    }

    /// The Tern is put right after a fight (ship-damage 9): no fire burns and every system is at nominal or better.
    pub fn put_right(&self) -> bool {
        let s = &self.damage[0];
        let nominal = self.data.damage.systems.nominal_from_pct;
        s.fires.is_empty() && s.integrity.iter().all(|v| *v >= nominal)
    }

    /// Ship `si`'s structural hull loses `mj`; at zero it is destroyed.
    fn hull_loss(&mut self, si: usize, mj: f64) {
        let ship = &mut self.ships[si];
        ship.hull -= mj;
        if ship.hull <= 0.0 && ship.alive {
            ship.hull = 0.0;
            ship.alive = false;
            self.events.push(DrillEvent::Destroyed { ship: ship.id });
        }
    }

    /// Locks build for both ships; tubes load and arm (armour-and-missiles 2); decoy bursts run down.
    fn locks_and_tubes(&mut self) {
        for si in 0..2 {
            let ti = 1 - si;
            // Damaged sensors lock more slowly; destroyed, not at all (ship-damage 4).
            let sensors = self.capability(si, "sensor_array");
            let (t_id, t_pos, t_up) =
                (self.ships[ti].id, self.ships[ti].pos, self.ships[ti].alive && self.ships[ti].active);
            let combat = self.data.combat_of(si);
            let lock = combat.lock.clone();
            let arm_s = combat.tubes.as_ref().map(|t| t.arm_s).unwrap_or(0.0);
            let rates: Vec<f64> = (0..self.ships[si].tubes.len()).map(|i| self.loading_rate(si, i as u8)).collect();
            let ship = &mut self.ships[si];
            if ship.lock_target == Some(t_id) {
                if t_up && (t_pos - ship.pos).length() <= lock.range_m {
                    ship.lock_s = (ship.lock_s + DT * sensors).min(lock.time_s);
                } else {
                    ship.lock_s = 0.0;
                }
            }
            ship.decoy_s = (ship.decoy_s - DT).max(0.0);
            for (t, rate) in ship.tubes.iter_mut().zip(&rates) {
                match t.state {
                    TubeState::Loading | TubeState::Arming => {
                        t.timer_s -= DT * if t.state == TubeState::Loading { *rate } else { 1.0 };
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
    }

    /// The Tern's turret the gunner holds: the dorsal one (enemy-fighters 3).
    fn gunner_turret(&self) -> Option<usize> {
        self.data.tern_combat.turrets.iter().position(|t| t.id == GUNNER_TURRET)
    }

    /// Ship `si`'s insides (ship-damage).
    pub fn layout(&self, si: usize) -> &damage::ShipLayout {
        if si == 0 {
            &self.data.tern_layout
        } else {
            &self.data.hound_layout
        }
    }

    /// The capability (0-1) of system or load `id` aboard ship `si` (ship-damage 4); 1 for a part it does not have.
    pub fn capability(&self, si: usize, id: &str) -> f64 {
        if si == 0 {
            // The Tern's grid is simulated: a system works with its integrity and the power it gets.
            let (d, s) = (&self.data.power, &self.damage[0]);
            return damage::capability_by(&self.data.tern_layout, id, |i| self.power.works(d, s.integrity[i], i));
        }
        self.damage_capability(si, id)
    }

    /// Capability 0-1 of a system by its damage alone: what a crew can still work by hand without power (a tube's
    /// hand crank, weapons-and-shields 7).
    pub fn damage_capability(&self, si: usize, id: &str) -> f64 {
        damage::capability(self.layout(si), &self.damage[si], &self.data.damage, id)
    }

    /// How fast ship `si`'s tube `i` loads, a share of its powered rate: its supply (reload time as 1 / supply,
    /// power-grid 6), never slower than the crew's hand crank (weapons-and-shields 7). A ship with no grid loads at 1.
    fn loading_rate(&self, si: usize, i: u8) -> f64 {
        let Some(t) = self.data.combat_of(si).tubes.as_ref() else { return 1.0 };
        if si != 0 {
            return 1.0;
        }
        let crank = t.crank_load_s.map_or(0.0, |c| t.load_s / c.max(1e-9));
        let supply = self.data.power.load(&tube_system(i)).map_or(1.0, |k| self.power.supply(k));
        supply.max(crank)
    }

    /// Whether ship `si` holds a full lock on the other ship.
    fn locked(&self, si: usize) -> bool {
        let ti = 1 - si;
        let t = &self.ships[ti];
        self.ships[si].lock_target == Some(t.id)
            && t.alive
            && t.active
            && self.ships[si].lock_s >= self.data.combat_of(si).lock.time_s
    }

    /// Load tube `i` of ship `si` from its magazine (the Tern's Tactical and the enemy alike).
    fn load_tube(&mut self, si: usize, i: u8) -> Result<(), Refusal> {
        let tubes = self.data.combat_of(si).tubes.clone().ok_or(Refusal::NoSuch)?;
        if self.damage_capability(si, &tube_system(i)) <= 0.0 {
            return Err(Refusal::Damaged);
        }
        let ship = &mut self.ships[si];
        let tube = ship.tubes.get_mut(usize::from(i)).ok_or(Refusal::NoSuch)?;
        if tube.state != TubeState::Empty {
            return Err(Refusal::TubeNotReady);
        }
        if ship.magazine == 0 {
            return Err(Refusal::MagazineEmpty);
        }
        ship.magazine -= 1;
        *tube = Tube { state: TubeState::Loading, timer_s: tubes.load_s };
        Ok(())
    }

    /// Fire tube `i` of ship `si` at the other ship, which it must hold locked and inside its missile's seeker cone.
    fn fire_tube(&mut self, si: usize, i: u8) -> Result<(), Refusal> {
        let ti = 1 - si;
        let tubes = self.data.combat_of(si).tubes.clone().ok_or(Refusal::NoSuch)?;
        let missile = self.data.missile(&tubes.missile).clone();
        let tube = *self.ships[si].tubes.get(usize::from(i)).ok_or(Refusal::NoSuch)?;
        if self.damage_capability(si, &tube_system(i)) <= 0.0 {
            return Err(Refusal::Damaged);
        }
        if tube.state != TubeState::Armed {
            return Err(Refusal::TubeNotReady);
        }
        if !self.locked(si) {
            return Err(Refusal::NotLocked);
        }
        let (t_id, t_pos, t_rot) = (self.ships[ti].id, self.ships[ti].pos, self.ships[ti].rot);
        let secs = &self.layout(ti).sections;
        let (lo, hi) = secs.iter().fold((f64::MAX, f64::MIN), |(lo, hi), q| (lo.min(q.z_m), hi.max(q.z_m)));
        let aim_m = DVec3::new(0.0, 0.0, lo + (hi - lo) * (0.1 + 0.8 * self.rng.next_f64()));
        let aim = t_pos + t_rot * aim_m;
        let ship = &mut self.ships[si];
        if off_bow_deg(ship.pos, ship.rot, t_pos) > missile.seeker_half_angle_deg {
            return Err(Refusal::OutOfCone);
        }
        if self.missiles.len() >= MAX_MISSILES {
            return Err(Refusal::TubeNotReady);
        }
        ship.tubes[usize::from(i)] = Tube { state: TubeState::Empty, timer_s: 0.0 };
        let muzzle = ship.pos + ship.rot * DVec3::from_array(tubes.muzzle_m[usize::from(i)]);
        let vel = ship.vel + ship.forward() * missile.eject_mps;
        let owner = ship.id;
        let id = self.next_missile;
        self.next_missile = self.next_missile.wrapping_add(1).max(1);
        self.missiles.push(Missile {
            id,
            owner,
            target: t_id,
            pos: muzzle,
            vel,
            age_s: 0.0,
            rel_prev: muzzle - aim,
            fooled: false,
            heading: ship.forward(),
            aim_m,
        });
        if si == 0 {
            self.stats.missiles_fired += 1;
        } else {
            self.stats.enemy_missiles_fired += 1;
        }
        self.events.push(DrillEvent::Launched { missile: id, owner });
        Ok(())
    }

    /// Laser banks charge, and each one's bearing and hit chance on the other ship (laser-banks 1-2).
    fn lasers_tick(&mut self) {
        for si in 0..2 {
            let ti = 1 - si;
            let combat = self.data.combat_of(si).clone();
            let Some(block) = combat.lasers else { continue };
            let w = self.data.laser(&block.laser).clone();
            let radius = self.data.combat_of(ti).hit_radius_m;
            let locked = self.locked(si);
            let (s_pos, s_rot, s_vel, s_up) = (
                self.ships[si].pos,
                self.ships[si].rot,
                self.ships[si].vel,
                self.ships[si].alive && self.ships[si].active,
            );
            let (t_pos, t_vel) = (self.ships[ti].pos, self.ships[ti].vel);
            let reactor = self.capability(si, "reactor");
            let bank_caps: Vec<f64> = block.banks.iter().map(|m| self.capability(si, &m.id)).collect();
            for (k, mount) in block.banks.iter().enumerate() {
                let b = &mut self.ships[si].lasers[k];
                b.since_fire_s += DT;
                // A damaged bank or reactor charges with what is left of it (ship-damage 4).
                if s_up {
                    b.charge_mj = (b.charge_mj + w.charge_mw * reactor * bank_caps[k] * DT).min(w.capacitor_mj);
                }
                let emitter = s_pos + s_rot * DVec3::from_array(mount.position_m);
                let rel = t_pos - emitter;
                let range = rel.length().max(1.0);
                b.bearing = locked && arc_bears(mount.arc, s_rot.inverse() * rel, combat.arc_overlap_deg);
                let across = (t_vel - s_vel).reject_from(rel / range).length();
                let rate_deg_s = (across / range).to_degrees();
                let sigma = (w.sigma_base_deg + w.sigma_rate_k * rate_deg_s).to_radians();
                b.hit_chance = if b.bearing && range <= w.range_m { hit_chance(range, sigma, radius) } else { 0.0 };
            }
        }
    }

    /// Fire bank `bank` of ship `si`: its whole capacitor in one beam that arrives at once (laser-banks 2).
    fn fire_laser(&mut self, si: usize, bank: u8) -> Result<(), Refusal> {
        let ti = 1 - si;
        let block = self.data.combat_of(si).lasers.clone().ok_or(Refusal::NoSuch)?;
        let mount = block.banks.get(usize::from(bank)).ok_or(Refusal::NoSuch)?.clone();
        let w = self.data.laser(&block.laser).clone();
        let b = *self.ships[si].lasers.get(usize::from(bank)).ok_or(Refusal::NoSuch)?;
        if self.capability(si, &mount.id) <= 0.0 {
            return Err(Refusal::Damaged);
        }
        if b.charge_mj < w.min_charge_frac * w.capacitor_mj {
            return Err(Refusal::NotCharged);
        }
        if !self.locked(si) {
            return Err(Refusal::NotLocked);
        }
        if !b.bearing {
            return Err(Refusal::OutOfCone);
        }
        let ship = &self.ships[si];
        let emitter = ship.pos + ship.rot * DVec3::from_array(mount.position_m);
        let (t_pos, radius) = (self.ships[ti].pos, self.data.combat_of(ti).hit_radius_m);
        let dir = (t_pos - emitter).normalize_or(DVec3::Z);
        let range = (t_pos - emitter).length();
        if range > w.range_m {
            return Err(Refusal::OutOfCone);
        }
        let hit = self.rng.next_f64() < b.hit_chance;
        let energy = b.charge_mj;
        let l = &mut self.ships[si].lasers[usize::from(bank)];
        l.charge_mj = 0.0;
        l.since_fire_s = 0.0;
        let owner = self.ships[si].id;
        let to = if hit {
            // The beam meets the near side of the target along its line.
            let point = t_pos - dir * radius * 0.8;
            self.hit_ship(ti, point, dir, energy, WeaponKind::Laser, None, 1.0);
            point
        } else {
            let off = scatter(&mut self.rng, dir, (radius * 3.0 / range.max(1.0)).atan());
            emitter + off * w.range_m
        };
        if si == 0 {
            self.stats.tern_laser_shots += 1;
            self.stats.tern_laser_hits += u32::from(hit);
        } else {
            self.stats.enemy_laser_shots += 1;
            self.stats.enemy_laser_hits += u32::from(hit);
        }
        self.events.push(DrillEvent::Beam { owner, bank, from: emitter, to, hit, energy_mj: energy });
        Ok(())
    }

    /// Whether bank `bank` of ship `si` should fire now (laser-banks 3; the Tern's automation and the enemy alike).
    fn laser_wanted(&self, si: usize, bank: usize, p: &data::LaserProfile) -> bool {
        let Some(block) = self.data.combat_of(si).lasers.as_ref() else { return false };
        let w = self.data.laser(&block.laser);
        let Some(b) = self.ships[si].lasers.get(bank) else { return false };
        laser_should_fire(b.charge_mj / w.capacitor_mj.max(1e-9), b.bearing, b.hit_chance, p)
    }

    /// Throw one of ship `si`'s countermeasures: every inbound missile within its reach rolls once, by its time to
    /// impact (armour-and-missiles 3).
    fn throw_decoy(&mut self, si: usize) -> Result<(), Refusal> {
        let block = self.data.combat_of(si).decoys.clone().ok_or(Refusal::NoSuch)?;
        let d = self.data.decoy(&block.decoy).clone();
        let ship = &mut self.ships[si];
        if ship.decoys == 0 {
            return Err(Refusal::MagazineEmpty);
        }
        if ship.decoy_s > 0.0 {
            return Err(Refusal::TubeNotReady);
        }
        ship.decoys -= 1;
        ship.decoy_s = d.life_s;
        let (id, pos, vel) = (ship.id, ship.pos, ship.vel);
        self.stats.decoys_thrown += 1;
        self.events.push(DrillEvent::DecoyOut { ship: id });
        let mut i = 0;
        while i < self.missiles.len() {
            let m = self.missiles[i];
            let rel = pos - m.pos;
            let dist = rel.length();
            if m.target != id || m.fooled || dist > d.range_m {
                i += 1;
                continue;
            }
            let closing = (m.vel - vel).dot(rel / dist.max(1e-6)).max(1.0);
            let (off, early) = d.chances(dist / closing);
            let roll = self.rng.next_f64();
            if roll < off {
                self.missiles[i].fooled = true;
                self.stats.missiles_fooled += 1;
                self.events.push(DrillEvent::Fooled { missile: m.id, early: false });
                i += 1;
            } else if roll < off + early {
                self.stats.missiles_fooled += 1;
                self.events.push(DrillEvent::Fooled { missile: m.id, early: true });
                self.events.push(DrillEvent::Detonated { missile: m.id, pos: m.pos, damage_mj: 0.0 });
                self.missiles.swap_remove(i);
            } else {
                i += 1;
            }
        }
        Ok(())
    }

    /// Whether ship `si` should throw a countermeasure now: one left, none out, and the nearest missile at it
    /// 1.2-2.5 s from impact (armour-and-missiles 3; the Tern's automation and the enemy alike).
    fn decoy_wanted(&self, si: usize) -> bool {
        let s = &self.ships[si];
        if s.decoys == 0 || s.decoy_s > 0.0 || self.data.combat_of(si).decoys.is_none() {
            return false;
        }
        self.missiles.iter().filter(|m| m.target == s.id && !m.fooled).any(|m| {
            let rel = s.pos - m.pos;
            let closing = (m.vel - s.vel).dot(rel.normalize_or_zero()).max(1.0);
            (1.2..=2.5).contains(&(rel.length() / closing))
        })
    }

    /// The enemy's weapons: the Tern's tactics, by the same rules (armour-and-missiles 5, laser-banks 4).
    fn enemy_weapons(&mut self) {
        if !(self.ships[1].alive && self.ships[1].active) {
            return;
        }
        for i in 0..self.ships[1].tubes.len() as u8 {
            let state = self.ships[1].tubes[usize::from(i)].state;
            if state == TubeState::Empty {
                let _ = self.load_tube(1, i);
            } else if state == TubeState::Armed {
                let _ = self.fire_tube(1, i);
            }
        }
        if self.decoy_wanted(1) {
            let _ = self.throw_decoy(1);
        }
        let p = self.data.profile("automation").lasers.clone();
        for k in 0..self.ships[1].lasers.len() {
            if self.laser_wanted(1, k, &p) {
                let _ = self.fire_laser(1, k as u8);
            }
        }
    }

    fn shields(&mut self) {
        // A damaged generator regenerates with what is left of it (ship-damage 4).
        let generator = [self.capability(0, "shield_generator"), self.capability(1, "shield_generator")];
        for (i, ship) in self.ships.iter_mut().enumerate() {
            let block = if i == 0 { &self.data.tern_shields } else { &self.data.enemy.shields };
            let caps = block.presets[usize::from(ship.preset).min(block.presets.len() - 1)].faces_mj;
            let total: f64 = caps.iter().sum::<f64>().max(1e-9);
            for (f, cap) in ship.faces.iter_mut().zip(caps) {
                *f = (*f + block.regen_mj_s * generator[i] * cap / total * DT).min(cap);
            }
        }
    }

    fn turrets(&mut self, si: usize, ti: usize) {
        let combat = self.data.combat_of(si).clone();
        let gun = self.data.gun(&combat.gun).clone();
        let target_radius = self.data.combat_of(ti).hit_radius_m;
        let can_fire = self.ships[si].alive && self.ships[si].active && self.locked(si);
        let (s_pos, s_rot, s_vel, s_id) =
            (self.ships[si].pos, self.ships[si].rot, self.ships[si].vel, self.ships[si].id);
        let (t_pos, t_vel) = (self.ships[ti].pos, self.ships[ti].vel);
        // Point defence (armour-and-missiles 4): missiles at this ship, within its reach, not already fooled.
        let missile_radius =
            self.data.combat_of(ti).tubes.as_ref().map_or(1.0, |t| self.data.missile(&t.missile).hit_radius_m);
        let inbound: Vec<(DVec3, DVec3)> = self
            .missiles
            .iter()
            .filter(|m| m.target == s_id && !m.fooled && (m.pos - s_pos).length() <= combat.pd_range_m)
            .map(|m| (m.pos, m.vel))
            .collect();
        // The Tern's turrets take fighters after missiles (enemy-fighters 2).
        let fighter_radius = self.data.fighter.hit_radius_m;
        let near_fighters: Vec<(DVec3, DVec3)> = if si == 0 {
            self.fighters
                .iter()
                .filter(|f| f.out() && (f.pos - s_pos).length() <= combat.fighter_range_m)
                .map(|f| (f.pos, f.vel))
                .collect()
        } else {
            Vec::new()
        };
        // A broken turret does not fire (ship-damage 4); a damaged or starved one charges with what it has
        // (power-grid 6).
        let caps: Vec<f64> = combat.turrets.iter().map(|m| self.capability(si, &turret_system(&m.id))).collect();
        let working: Vec<bool> = caps.iter().map(|c| *c > 0.0).collect();
        let mut fired = Vec::new();
        for (k, mount) in combat.turrets.iter().enumerate() {
            let turret = &mut self.ships[si].turrets[k];
            turret.capacitor_mj = (turret.capacitor_mj + gun.charge_mw * caps[k] * DT).min(gun.capacitor_mj);
            turret.cooldown_s = (turret.cooldown_s - DT).max(-DT);
            turret.since_fire_s += DT;
            // Out of rounds, it reloads, then has its rounds again.
            if turret.reload_s > 0.0 {
                turret.reload_s -= DT;
                if turret.reload_s <= 0.0 {
                    turret.reload_s = 0.0;
                    turret.rounds = gun.rounds;
                }
            }
            // The sink sheds into the coolant; full, it locks the turret out until it cools to its resume share.
            turret.heat_mj = (turret.heat_mj - gun.heat_shed_mw * DT).max(0.0);
            if turret.cooling && turret.heat_mj < gun.heat_resume_frac * gun.heat_sink_mj {
                turret.cooling = false;
            }
            let muzzle = s_pos + s_rot * DVec3::from_array(mount.position_m);
            let ready = working[k]
                && !turret.cooling
                && turret.rounds > 0
                && turret.capacitor_mj >= gun.draw_mj
                && turret.cooldown_s <= 0.0;
            let base_sigma = combat.aim_sigma_base_deg;
            // The gunner's hand (enemy-fighters 3): along the aim while the trigger is held, at half the sigma.
            let shot = if turret.manual {
                let dir = (s_rot * turret.manual_aim).normalize_or(s_rot * DVec3::Z);
                turret.aim = dir;
                turret.bearing = arc_bears(mount.arc, turret.manual_aim, combat.arc_overlap_deg);
                (turret.trigger && turret.bearing && ready).then_some((dir, (base_sigma * 0.5).to_radians()))
            } else {
                // AUTO and PD take an inbound missile first; AUTO and TARGET fire at the locked ship.
                let pd = matches!(turret.mode, TurretMode::Auto | TurretMode::Pd);
                let missile = if pd {
                    inbound
                        .iter()
                        .filter(|(p, _)| arc_bears(mount.arc, s_rot.inverse() * (*p - muzzle), combat.arc_overlap_deg))
                        .min_by(|a, b| (a.0 - muzzle).length().total_cmp(&(b.0 - muzzle).length()))
                } else {
                    None
                };
                let fighter = if pd && missile.is_none() {
                    near_fighters
                        .iter()
                        .filter(|(p, _)| arc_bears(mount.arc, s_rot.inverse() * (*p - muzzle), combat.arc_overlap_deg))
                        .min_by(|a, b| (a.0 - muzzle).length().total_cmp(&(b.0 - muzzle).length()))
                } else {
                    None
                };
                // A missile is small and fast: the turret's aim error against one is its point-defence sigma.
                let (aim_pos, aim_vel, radius, fires, sigma_base) = match (missile, fighter) {
                    (Some(&(p, v)), _) => (p, v, missile_radius, true, combat.pd_aim_sigma_deg),
                    (None, Some(&(p, v))) => (p, v, fighter_radius, true, base_sigma),
                    (None, None) => (
                        t_pos,
                        t_vel,
                        target_radius,
                        can_fire && matches!(turret.mode, TurretMode::Auto | TurretMode::Target),
                        base_sigma,
                    ),
                };
                turret.bearing = arc_bears(mount.arc, s_rot.inverse() * (aim_pos - muzzle), combat.arc_overlap_deg);
                match lead(aim_pos - muzzle, aim_vel - s_vel, gun.speed_mps) {
                    Some(aim) => {
                        let sigma_rad = (sigma_base + gun.sigma_rate_k * aim.angular_rate_deg_s).to_radians();
                        let p = if aim.time_s <= gun.life_s { hit_chance(aim.range_m, sigma_rad, radius) } else { 0.0 };
                        turret.aim = aim.dir;
                        turret.hit_chance = p;
                        (fires && turret.bearing && p >= gun.min_hit_chance && ready).then_some((aim.dir, sigma_rad))
                    }
                    None => {
                        turret.hit_chance = 0.0;
                        None
                    }
                }
            };
            if let Some((dir, sigma)) = shot {
                if self.bolts.len() + fired.len() < MAX_BOLTS {
                    let turret = &mut self.ships[si].turrets[k];
                    turret.capacitor_mj -= gun.draw_mj;
                    turret.cooldown_s += 1.0 / gun.rate_hz;
                    turret.since_fire_s = 0.0;
                    turret.heat_mj += gun.heat_per_bolt_mj;
                    if turret.heat_mj >= gun.heat_sink_mj {
                        turret.cooling = true;
                    }
                    turret.rounds = turret.rounds.saturating_sub(1);
                    if turret.rounds == 0 {
                        turret.reload_s = gun.reload_s;
                    }
                    fired.push((muzzle, dir, sigma));
                }
            }
        }
        for (muzzle, dir, sigma) in fired {
            let d = scatter(&mut self.rng, dir, sigma);
            let id = self.next_bolt;
            self.next_bolt = self.next_bolt.wrapping_add(1).max(1);
            let bolt = Bolt {
                id,
                owner: s_id,
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

    /// A hit of `dmg` MJ arriving at `point` along `dir` on ship `ti`, from `weapon` on `band` (a gun's; none for a
    /// missile or a laser). The shield's face there holds what it can of `shield_share` of the hit: a cannon bolt
    /// counts at its gun's `capital_factor` (the caller applies it), a missile's blast meets the face at its
    /// `shield_share` (armour-and-missiles 1), and on the Tern the shield's band scales what the face loses
    /// (weapons-and-shields 11). What passes goes on into the ship (`after_shield`). Returns the face.
    #[allow(clippy::too_many_arguments)]
    fn hit_ship(
        &mut self,
        ti: usize,
        point: DVec3,
        dir: DVec3,
        dmg: f64,
        weapon: WeaponKind,
        band: Option<u8>,
        shield_share: f64,
    ) -> u8 {
        let e = if ti == 0 { self.data.tern_shields.ellipsoid } else { self.data.enemy.shields.ellipsoid };
        let k = match (ti, self.data.tern_shields.frequency) {
            (0, Some(f)) if self.bridge.retune_s > 0.0 => f.retune_factor,
            (0, Some(f)) if band.is_some() && band == Some(self.bridge.band) => f.match_factor,
            _ => 1.0,
        };
        let ship = &mut self.ships[ti];
        let face = face_of(ship, &e, point);
        let meets = dmg * shield_share;
        let absorbed = ship.faces[usize::from(face)].min(meets * k);
        ship.faces[usize::from(face)] -= absorbed;
        let rest = dmg - absorbed / k;
        if ti == 1 {
            self.stats.damage_dealt_mj += dmg;
        } else {
            self.stats.damage_taken_mj += dmg;
        }
        if rest > 1e-9 {
            self.after_shield(ti, point, dir, rest, weapon);
        }
        face
    }

    /// What passed a shield goes into the ship (ship-damage 1-3): the path is taken into the ship's frame, meets its
    /// hull (a shot that passes the shield bubble can still miss the hull itself), and the armour section there, the
    /// march inside and the cascade decide what breaks; what passes the armour also comes off the structural hull.
    fn after_shield(&mut self, ti: usize, point: DVec3, dir: DVec3, energy_mj: f64, weapon: WeaponKind) {
        let (pos, rot) = (self.ships[ti].pos, self.ships[ti].rot);
        let inv = rot.inverse();
        let p = inv * (point - pos);
        let d = match weapon {
            WeaponKind::Missile => (DVec3::new(0.0, 0.0, p.z) - p).normalize_or(-p.normalize_or(DVec3::Z)),
            WeaponKind::Cannon | WeaponKind::Laser => (inv * dir).normalize_or(-p.normalize_or(DVec3::Z)),
        };
        let l = if ti == 0 { &self.data.tern_layout } else { &self.data.hound_layout };
        let entry = match weapon {
            // A blast drives straight in, toward the ship's long axis beneath it.
            WeaponKind::Missile => damage::hull_entry(l, p, DVec3::new(0.0, 0.0, p.z)),
            WeaponKind::Cannon | WeaponKind::Laser => damage::hull_entry(l, p - d * 40.0, p + d * 120.0),
        };
        let Some(entry) = entry else { return };
        self.hits += 1;
        let w = match weapon {
            WeaponKind::Cannon => damage::Weapon::Cannon,
            WeaponKind::Laser => damage::Weapon::Laser,
            WeaponKind::Missile => damage::Weapon::Missile,
        };
        let seed = self.round_seed();
        let out =
            damage::resolve_hit(l, &mut self.damage[ti], &self.data.damage, entry, d, energy_mj, w, self.hits, seed);
        let id = self.ships[ti].id;
        if out.passed_mj > 0.0 {
            self.events.push(DrillEvent::Burst { ship: id, pos: pos + rot * entry, size: out.passed_mj });
        }
        for (at, size) in &out.bursts {
            self.events.push(DrillEvent::Burst { ship: id, pos: pos + rot * *at, size: *size });
        }
        self.hull_loss(ti, out.passed_mj);
    }

    fn bolts_step(&mut self) {
        let radii = [self.data.tern_combat.hit_radius_m, self.data.enemy.combat.hit_radius_m];
        let mut i = 0;
        'bolts: while i < self.bolts.len() {
            let b = self.bolts[i];
            let step = |pos: DVec3, vel: DVec3, radius: f64| -> Option<DVec3> {
                let p0 = b.pos - pos;
                let d = (b.vel - vel) * DT;
                let u = if d.length_squared() > 0.0 { (-p0.dot(d) / d.length_squared()).clamp(0.0, 1.0) } else { 0.0 };
                let closest = p0 + d * u;
                (closest.length() <= radius).then_some(pos + closest)
            };
            // A missile in the bolt's path is killed by it (point defence, armour-and-missiles 4).
            for k in 0..self.missiles.len() {
                let m = self.missiles[k];
                let mi = if m.owner == TERN_ID { 0 } else { 1 };
                let radius =
                    self.data.combat_of(mi).tubes.as_ref().map_or(1.0, |t| self.data.missile(&t.missile).hit_radius_m);
                if m.owner != b.owner && step(m.pos, m.vel, radius).is_some() {
                    self.events.push(DrillEvent::Detonated { missile: m.id, pos: m.pos, damage_mj: 0.0 });
                    self.missiles.swap_remove(k);
                    self.bolts.swap_remove(i);
                    continue 'bolts;
                }
            }
            // The Tern's bolts strike fighters at full damage (enemy-fighters 2).
            if b.owner == TERN_ID {
                let radius = self.data.fighter.hit_radius_m;
                if let Some(k) = self.fighters.iter().position(|f| f.out() && step(f.pos, f.vel, radius).is_some()) {
                    let fd = self.data.fighter.clone();
                    let f = &mut self.fighters[k];
                    let id = f.id;
                    let down = fighters::hit(f, &fd, b.damage_mj);
                    self.stats.tern_hits += 1;
                    self.events.push(DrillEvent::Hit { bolt: b.id, target: id, face: 0, damage_mj: b.damage_mj });
                    if down {
                        self.stats.fighters_down += 1;
                        self.events.push(DrillEvent::Destroyed { ship: id });
                    }
                    self.bolts.swap_remove(i);
                    continue;
                }
            }
            let ti = if b.owner == TERN_ID { 1 } else { 0 };
            let t = &self.ships[ti];
            let hit = if t.alive && t.active { step(t.pos, t.vel, radii[ti]) } else { None };
            if let Some(point) = hit {
                let gun = if b.owner == TERN_ID { &self.data.tern_combat.gun } else { &self.data.enemy.combat.gun };
                let gun = self.data.gun(gun);
                let (band, factor) = (Some(gun.band_index()), gun.capital_factor);
                let dir = b.vel.normalize_or(DVec3::Z);
                let face = self.hit_ship(ti, point, dir, b.damage_mj * factor, WeaponKind::Cannon, band, 1.0);
                if ti == 1 {
                    self.stats.tern_hits += 1;
                } else {
                    self.stats.enemy_hits += 1;
                }
                self.events.push(DrillEvent::Hit {
                    bolt: b.id,
                    target: self.ships[ti].id,
                    face,
                    damage_mj: b.damage_mj * factor,
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
        let mut i = 0;
        while i < self.missiles.len() {
            let m0 = self.missiles[i];
            let si = if m0.owner == TERN_ID { 0 } else { 1 };
            let ti = 1 - si;
            let Some(tubes) = self.data.combat_of(si).tubes.clone() else {
                self.missiles.swap_remove(i);
                continue;
            };
            let md = self.data.missile(&tubes.missile).clone();
            let target = &self.ships[ti];
            let (t_vel, t_up) = (target.vel, target.alive && target.active);
            // It homes on its aim point on the target, not the target's middle.
            let t_pos = target.pos + target.rot * self.missiles[i].aim_m;
            let m = &mut self.missiles[i];
            m.age_s += DT;
            let motor = if m.age_s < md.boost_s {
                md.boost_mps2
            } else if m.age_s < md.boost_s + md.sustain_s {
                md.sustain_mps2
            } else {
                0.0
            };
            let los = t_pos - m.pos;
            let seeing = !m.fooled
                && t_up
                && m.heading.dot(los.normalize_or_zero()).clamp(-1.0, 1.0).acos().to_degrees()
                    <= md.seeker_half_angle_deg;
            if seeing {
                // The body turns toward the target at its turn rate.
                let to = los.normalize_or(m.heading);
                let angle = m.heading.dot(to).clamp(-1.0, 1.0).acos();
                if angle > 1e-9 {
                    let step = (md.turn_deg_s.to_radians() * DT).min(angle);
                    let axis = m.heading.cross(to).normalize_or(DVec3::Y);
                    m.heading = (DQuat::from_axis_angle(axis, step) * m.heading).normalize();
                }
            }
            let mut acc = m.heading * motor;
            if seeing {
                // Proportional navigation: a = N Vc (Omega x los), limited to the motor's (or the sustain's) push.
                let r2 = los.length_squared().max(1.0);
                let v_rel = t_vel - m.vel;
                let omega = los.cross(v_rel) / r2;
                let closing = -los.dot(v_rel) / r2.sqrt();
                let lat = (omega.cross(los.normalize_or_zero())) * (md.nav_gain * closing);
                acc += lat.clamp_length_max(motor.max(md.sustain_mps2));
            }
            m.vel += acc * DT;
            m.pos += m.vel * DT;
            let rel_now = m.pos - t_pos;
            let seg = rel_now - m.rel_prev;
            let u = if seg.length_squared() > 0.0 {
                (-m.rel_prev.dot(seg) / seg.length_squared()).clamp(0.0, 1.0)
            } else {
                1.0
            };
            let closest = (m.rel_prev + seg * u).length();
            m.rel_prev = rel_now;
            let (id, pos, age, fooled, vel) = (m.id, m.pos, m.age_s, m.fooled, m.vel);
            let burst = !fooled && t_up && closest <= md.blast_radius_m && (u < 1.0 || closest < 2.0);
            if burst {
                let dmg = md.warhead_mj * (1.0 - closest / md.blast_radius_m);
                let point = t_pos + (pos - t_pos).normalize_or_zero() * closest;
                let dir = (t_pos - point).normalize_or(vel.normalize_or(DVec3::Z));
                self.hit_ship(ti, point, dir, dmg, WeaponKind::Missile, None, md.shield_share);
                if si == 0 {
                    self.stats.missile_hits += 1;
                } else {
                    self.stats.enemy_missile_hits += 1;
                }
                self.events.push(DrillEvent::Detonated { missile: id, pos, damage_mj: dmg });
                self.missiles.swap_remove(i);
            } else if age >= md.life_s || !t_up {
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

/// The patrol (openspec/changes/patrol-mission): its start in the berth, its tick, the jump drive and the encounter.
impl Drill {
    /// Docked in the berth on shore power with the drive's breaker open (design 4.1); none for the drill.
    fn patrol_start(&mut self) {
        let Some(d) = self.data.patrol.as_ref() else {
            self.patrol = None;
            return;
        };
        let st = patrol::PatrolState::new(d);
        let t = &mut self.ships[0];
        t.pos = st.local(d.berth_pos);
        t.rot = d.berth_rot;
        t.vel = DVec3::ZERO;
        t.rates = DVec3::ZERO;
        t.speed_set = 0.0;
        t.stick = [0.0; 3];
        t.strafe = [0.0; 2];
        self.patrol = Some(st);
        self.patrol_mem = Default::default();
        if let Some(g) = self.drive_group() {
            self.power.set_group_breaker(&self.data.power, g, power::Breaker::Open);
        }
    }

    /// The power group that feeds the impulse drive (Engineering's breaker for it).
    pub fn drive_group(&self) -> Option<usize> {
        patrol::drive_group(&self.data.power)
    }

    /// Where the helm's COURSE flies, local frame: in a patrol the place it fights at or goes to next, else the
    /// mission's waypoint (the drill's relay).
    pub fn waypoint(&self) -> DVec3 {
        match (&self.patrol, &self.data.patrol) {
            (Some(s), Some(d)) if self.phase == Phase::Engage => s.local(d.places[s.at].pos),
            (Some(s), Some(d)) => s.course_point(d),
            _ => DVec3::from_array(self.data.mission.waypoint_m),
        }
    }

    /// The drive is ready for the berth: shore power dropped and the impulse drive working (design 4.1).
    pub fn drive_ready(&self) -> bool {
        match (&self.patrol, &self.data.patrol) {
            (Some(s), Some(d)) => !s.shore && self.capability(0, "impulse_drive") >= d.block.drive_ready_min,
            _ => false,
        }
    }

    /// What holds the jump drive where the Tern is, by name: a hostile ship in reach, a body, a station (FN 7).
    pub fn mass_lock(&self) -> Option<String> {
        let (s, d) = (self.patrol.as_ref()?, self.data.patrol.as_ref()?);
        let (t, e) = (&self.ships[0], &self.ships[1]);
        if e.active && e.alive && (e.pos - t.pos).length() < d.jump.ship_m {
            return Some(self.data.enemy.name.clone());
        }
        d.system.mass_lock(s.origin + t.pos, d.jump.planet_radii)
    }

    /// What a station sees of the patrol (design 9).
    pub fn patrol_picture(&self) -> Option<patrol::PatrolPicture> {
        let (s, d) = (self.patrol.as_ref()?, self.data.patrol.as_ref()?);
        let g = self.drive_group();
        let t = &self.ships[0];
        Some(patrol::PatrolPicture {
            stage: s.stage,
            shore: s.shore,
            drive_open: g.is_some_and(|g| self.power.group_breaker[g] != power::Breaker::Closed),
            drive_ready: self.drive_ready(),
            drive_group: g.unwrap_or(0) as u8,
            spooling: s.spooling,
            cooldown_s: s.cooldown_s,
            locked: self.mass_lock().is_some(),
            swept_here: s.swept[s.at],
            sweeping: s.sweeping,
            done: s.next.is_none() || s.stage == patrol::Stage::Home,
            ordered: s.ordered,
            captain_manned: self.operator(Station::Captain) != Operator::Auto,
            hostile: self.ships[1].active && self.ships[1].alive,
            next_dir: s.course_point(d) - t.pos,
            rot: t.rot,
            rates: t.rates,
            on_course: t.helm_mode == HelmMode::Course,
        })
    }

    /// The ship's computer says a patrol line with its place.
    fn patrol_say(&mut self, id: &str, place: &str) {
        if let Some(x) = self.data.chat.line(id, &[("place", place)]) {
            let now = self.clock_s();
            self.voices.computer.offer(x, now, &self.data.chat.limits);
        }
    }

    /// The empty seats' patrol decisions, by the rule a bot uses (design 9), at the automation's reaction times.
    fn patrol_automation(&mut self) {
        let (Some(pic), Some(d)) = (self.patrol_picture(), self.data.patrol.as_ref()) else { return };
        let block = d.block.clone();
        let now = self.clock_s();
        let profile = self.data.profile("automation").clone();
        for s in Station::ALL {
            if self.operator(s) != Operator::Auto || !self.data.mission.stations.iter().any(|id| id == s.id()) {
                continue;
            }
            let mem = &mut self.patrol_mem[s as usize];
            let mut cmds = patrol::decide(s, &pic, &block, mem, now, profile.reaction_s(s));
            // In a fight the drill's automation flies the helm; the patrol's own decisions are the jump.
            if self.phase == Phase::Engage {
                cmds.retain(|c| !matches!(c, Command::Helm(_) | Command::Stick { .. }));
            }
            for c in &cmds {
                if let Some(x) = chat::said_for(&self.data, s, c) {
                    self.voices.stations[s as usize].offer(x, now, &self.data.chat.limits);
                }
            }
            for c in cmds {
                let _ = self.apply(s, c);
            }
        }
    }

    /// The patrol's commands (design 7); none in a mission that is not a patrol.
    fn apply_patrol(&mut self, cmd: Command) -> Result<(), Refusal> {
        use patrol::Stage;
        let lock = self.mass_lock();
        let cap = self.capability(0, "impulse_drive");
        let (Some(s), Some(d)) = (self.patrol.as_mut(), self.data.patrol.as_ref()) else { return Err(Refusal::NoSuch) };
        let mut locked_by = None;
        match cmd {
            Command::Shore(on) => {
                if s.stage != Stage::Docked {
                    return Err(Refusal::WrongPhase);
                }
                s.shore = on;
            }
            Command::Undock => {
                if s.stage != Stage::Docked {
                    return Err(Refusal::WrongPhase);
                }
                if s.shore || cap < d.block.drive_ready_min {
                    return Err(Refusal::NotReady);
                }
                s.stage = Stage::Releasing;
                s.clamps_s = d.block.clamps_s;
                s.ordered = None;
            }
            Command::Jump(true) => {
                if s.stage != Stage::Underway {
                    return Err(Refusal::WrongPhase);
                }
                if s.next.is_none() {
                    return Err(Refusal::NoSuch);
                }
                if !s.spooling {
                    if s.cooldown_s > 0.0 {
                        return Err(Refusal::NotReady);
                    }
                    if lock.is_some() {
                        locked_by = lock;
                    } else {
                        s.spooling = true;
                        s.spool_mj = 0.0;
                        s.aligned_s = 0.0;
                        s.ordered = None;
                    }
                }
            }
            Command::Jump(false) => {
                s.spooling = false;
                s.spool_mj = 0.0;
                s.aligned_s = 0.0;
            }
            Command::Sweep => {
                if s.stage != Stage::Underway {
                    return Err(Refusal::WrongPhase);
                }
                if !s.swept[s.at] && !s.sweeping {
                    s.sweeping = true;
                    s.sweep = 0.0;
                }
            }
            Command::Patrol(o) => s.ordered = Some(o),
            _ => return Err(Refusal::NoSuch),
        }
        if let Some(what) = locked_by {
            self.patrol_say("mass_lock", &what);
            return Err(Refusal::MassLocked);
        }
        Ok(())
    }

    /// One tick of the patrol outside a fight (design 4): the berth holds the Tern, the clamps open, the departure is
    /// flown, then the helm flies; the jump drive and the sweep run.
    fn patrol_tick(&mut self) {
        use patrol::Stage;
        let (Some(s), Some(d)) = (self.patrol.as_ref(), self.data.patrol.as_ref()) else { return };
        let (berth_pos, berth_rot) = (s.local(d.berth_pos), d.berth_rot);
        let (stage, leg) = (s.stage, s.leg);
        let legs = d.departure.clone();
        let tf = scaled_flight(&self.data.tern_flight, self.capability(0, "impulse_drive"));
        let ready = self.drive_ready();
        let mut said: Vec<&str> = Vec::new();
        match stage {
            Stage::Docked | Stage::Releasing => {
                // Clamped: the berth holds the Tern where it is (design 4.2).
                let t = &mut self.ships[0];
                t.pos = berth_pos;
                t.rot = berth_rot;
                t.vel = DVec3::ZERO;
                t.rates = DVec3::ZERO;
                let s = self.patrol.as_mut().expect("checked");
                if stage == Stage::Docked && ready && !s.drive_said {
                    s.drive_said = true;
                    said.push("drive_ready");
                }
                if stage == Stage::Releasing {
                    s.clamps_s -= DT;
                    if s.clamps_s <= 0.0 {
                        s.stage = Stage::Departing;
                        said.push("clamps_released");
                    }
                }
            }
            Stage::Departing => {
                let (spd, strafe, done) = {
                    let t = &self.ships[0];
                    patrol::departure_setpoints(t.pos, t.rot, berth_pos, berth_rot, &legs, leg)
                };
                let t = &mut self.ships[0];
                t.stick = [0.0; 3];
                t.helm_mode = HelmMode::Hold;
                t.order = None;
                if done {
                    let s = self.patrol.as_mut().expect("checked");
                    s.leg += 1;
                    if s.leg >= legs.len() {
                        s.stage = Stage::Underway;
                        s.undocked = true;
                        said.push("clear_of_berth");
                    }
                } else {
                    t.speed_set = spd;
                    t.strafe = strafe;
                }
                fly(&mut self.ships[0], &tf, DT);
            }
            Stage::Underway => {
                self.helm_tick();
                fly(&mut self.ships[0], &tf, DT);
            }
            Stage::Home => {}
        }
        for id in said {
            self.patrol_say(id, "");
        }
        self.patrol_drive();
    }

    /// The jump drive and the sweep (design 5), in and out of a fight: the cooldown runs down, a sweep runs, the
    /// spool fills at the drive's power while nothing locks it, and when it is full with the bow held on the jump
    /// vector the Tern jumps.
    fn patrol_drive(&mut self) {
        let lock = self.mass_lock();
        let cap = self.capability(0, "impulse_drive");
        let (Some(s), Some(d)) = (self.patrol.as_mut(), self.data.patrol.as_ref()) else { return };
        let j = &d.jump;
        s.cooldown_s = (s.cooldown_s - DT).max(0.0);
        let mut said: Vec<(&str, String)> = Vec::new();
        if s.sweeping {
            s.sweep = (s.sweep + DT / d.block.sweep_s).min(1.0);
            if s.sweep >= 1.0 {
                s.sweeping = false;
                s.swept[s.at] = true;
                let e = &self.ships[1];
                let contact = e.active && e.alive && (e.pos - self.ships[0].pos).length() <= d.block.sweep_range_m;
                said.push((if contact { "sweep_contact" } else { "sweep_clear" }, d.places[s.at].name.clone()));
            }
        }
        let mut jump_now = false;
        if s.spooling {
            if let Some(what) = lock {
                s.spooling = false;
                s.spool_mj = 0.0;
                s.aligned_s = 0.0;
                said.push(("mass_lock", what));
            } else {
                s.spool_mj = (s.spool_mj + j.spool_mw * cap * DT).min(j.spool_mj);
                let t = &self.ships[0];
                if patrol::aligned(t.rot, t.rates, s.course_point(d) - t.pos, j) {
                    s.aligned_s += DT;
                } else {
                    s.aligned_s = 0.0;
                }
                jump_now = s.spool_mj >= j.spool_mj && s.aligned_s >= j.align_last_s;
            }
        }
        for (id, place) in said {
            self.patrol_say(id, &place);
        }
        if jump_now {
            self.patrol_jump();
        }
    }

    /// The jump (FN 7): the Tern moves to the next place's arrival point, scattered, in one tick, moving as it was;
    /// the drill's origin moves with it. Arriving at the encounter's point starts the drill's fight; arriving home
    /// ends the mission.
    fn patrol_jump(&mut self) {
        let seed = self.round_seed();
        let (Some(s), Some(d)) = (self.patrol.as_mut(), self.data.patrol.as_ref()) else { return };
        let Some(to) = s.next else { return };
        let mut rng = Rng::for_purpose(seed, u64::from(s.jumps), "arrival");
        let scatter = loop {
            let p = DVec3::new(rng.next_f64(), rng.next_f64(), rng.next_f64()) * 2.0 - DVec3::ONE;
            if p.length_squared() <= 1.0 {
                break p * d.jump.arrival_scatter_m;
            }
        };
        let place = d.places[to].clone();
        s.origin = place.arrival + scatter;
        s.at = to;
        s.next = if to == 0 {
            None
        } else if to + 1 < d.places.len() {
            Some(to + 1)
        } else {
            Some(0)
        };
        s.spooling = false;
        s.spool_mj = 0.0;
        s.aligned_s = 0.0;
        s.cooldown_s = d.jump.cooldown_s;
        s.ordered = None;
        s.jumps += 1;
        s.sweep = 0.0;
        s.sweeping = false;
        let encounter = to == d.encounter && !s.met;
        if encounter {
            s.met = true;
        }
        let home = to == 0;
        if home {
            s.stage = patrol::Stage::Home;
        }
        let met = s.met;
        // The Tern at the new origin; what it left behind stays behind.
        self.ships[0].pos = DVec3::ZERO;
        self.bolts.clear();
        self.missiles.clear();
        self.fighters.clear();
        self.ships[1].active = false;
        self.patrol_say(if home { "home" } else { "arrived" }, &place.name);
        if home {
            self.outcome = if met && !self.ships[1].alive { Outcome::Victory } else { Outcome::Withdrew };
            self.set_phase(Phase::Debrief);
        } else if encounter {
            self.start_encounter();
        } else if self.phase == Phase::Engage {
            self.set_phase(Phase::Patrol);
        }
    }

    /// The drill's fight at the encounter's point (design 5.3): the enemy placed from the Tern's arrival as the drill
    /// places it from the Tern's start, facing it, and Engage begins.
    fn start_encounter(&mut self) {
        let start = DVec3::from_array(self.data.mission.enemy_start_m);
        let approach = self.data.enemy.ai.approach_speed_mps;
        let (pos, rot) = (self.ships[0].pos, self.ships[0].rot);
        let e = &mut self.ships[1];
        e.pos = pos + rot * start;
        e.rot = DQuat::from_rotation_arc(DVec3::Z, (pos - e.pos).normalize_or_zero());
        e.speed_set = approach;
        e.vel = e.forward() * approach;
        e.active = true;
        self.set_phase(Phase::Engage);
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
        Command::GunnerAim { yaw_deg, pitch_deg } => {
            if yaw_deg.is_finite() && pitch_deg.is_finite() && yaw_deg.abs() <= 360.0 && pitch_deg.abs() <= 90.0 {
                Ok(())
            } else {
                Err(Refusal::BadValue)
            }
        }
        Command::Power(PowerCmd::Group(_, sp) | PowerCmd::Preview(Some((_, sp))))
            if !sp.is_finite() || !(0.0..=2.0).contains(&sp) =>
        {
            Err(Refusal::BadValue)
        }
        Command::Power(PowerCmd::Lever(k, v))
            if usize::from(k) >= LEVERS.len() || !v.is_finite() || !(0.0..=2.0).contains(&v) =>
        {
            Err(Refusal::BadValue)
        }
        Command::LaserFire(b) if b >= 4 => Err(Refusal::BadValue),
        Command::Freq(b) if b >= 4 => Err(Refusal::BadValue),
        Command::Viewscreen(Some(v)) if usize::from(v) >= FEEDS.len() => Err(Refusal::BadValue),
        Command::MainView { feed, zoom, .. }
            if usize::from(feed) >= FEEDS.len() || usize::from(zoom) >= ZOOMS.len() =>
        {
            Err(Refusal::BadValue)
        }
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

    /// A patrol with players at `stations`, in Phase::Patrol, docked.
    fn patrolling(stations: &[Station]) -> (Drill, Vec<u8>) {
        let mut d = Drill::new(DrillData::shipped_patrol(), 7);
        let slots: Vec<u8> = stations
            .iter()
            .map(|s| {
                let p = d.join(s.name(), false).unwrap();
                d.claim(p, *s).unwrap();
                d.set_ready(p, true).unwrap();
                p
            })
            .collect();
        for _ in 0..(TICK_HZ as usize * 10) {
            d.step();
            if d.phase == Phase::Patrol {
                return (d, slots);
            }
        }
        panic!("the patrol never began");
    }

    /// Run until `f` holds, at most `max_s` seconds; returns the seconds it took.
    fn run_until(d: &mut Drill, max_s: f64, f: impl Fn(&Drill) -> bool) -> f64 {
        for i in 0..(max_s * TICK_HZ) as usize {
            if f(d) {
                return i as f64 * DT;
            }
            d.step();
        }
        panic!("not within {max_s} s: phase {:?}, patrol {:?}", d.phase, d.patrol.as_ref().map(|p| (p.stage, p.at)));
    }

    #[test]
    fn a_patrol_begins_docked_and_the_berth_holds_the_tern() {
        let (mut d, _) = patrolling(&[Station::Tactical]);
        let p = d.patrol.as_ref().unwrap();
        assert_eq!((p.stage, p.shore), (patrol::Stage::Docked, true));
        assert!(!d.ships[1].active, "the Hound waits at its point");
        let berth = d.ships[0].rot;
        d.ships[0].speed_set = 100.0;
        steps(&mut d, 2.0);
        assert!(d.ships[0].pos.length() < 1e-9 && d.ships[0].rot.angle_between(berth) < 1e-9, "clamped");
    }

    #[test]
    fn the_clamps_stay_shut_until_the_drive_is_up_and_shore_power_is_off() {
        let (mut d, s) = patrolling(&[Station::Helm, Station::Engineering]);
        d.command(s[0], Command::Undock).unwrap();
        steps(&mut d, 0.1);
        assert_eq!(d.patrol.as_ref().unwrap().stage, patrol::Stage::Docked, "on shore power, the drive cold");
        let g = d.drive_group().unwrap() as u8;
        d.command(s[1], Command::Power(PowerCmd::Breaker(g))).unwrap();
        d.command(s[1], Command::Shore(false)).unwrap();
        run_until(&mut d, 30.0, |d| d.drive_ready());
        d.command(s[0], Command::Undock).unwrap();
        steps(&mut d, 0.1);
        assert_eq!(d.patrol.as_ref().unwrap().stage, patrol::Stage::Releasing);
    }

    #[test]
    fn undocking_flies_the_tern_out_of_the_bay_and_down_the_throat_under_the_limits() {
        let (mut d, s) = patrolling(&[Station::Helm, Station::Engineering]);
        let g = d.drive_group().unwrap() as u8;
        d.command(s[1], Command::Power(PowerCmd::Breaker(g))).unwrap();
        d.command(s[1], Command::Shore(false)).unwrap();
        run_until(&mut d, 30.0, |d| d.drive_ready());
        d.command(s[0], Command::Undock).unwrap();
        let (mut fastest, mut took): (f64, f64) = (0.0, 0.0);
        while d.patrol.as_ref().is_some_and(|p| p.stage != patrol::Stage::Underway) {
            d.step();
            took += DT;
            fastest = fastest.max(d.ships[0].vel.length());
            assert!(took < 400.0, "the departure never ended");
        }
        // Up off the bay's wall to the axis, down the axis out through the throat, and on clear of the pair: the
        // berth's legs (Tamsin Station's in the system file), never faster than the corridor's 40 m/s.
        let out = d.ships[0].rot.inverse() * d.ships[0].pos;
        assert!((out - DVec3::new(0.0, 200.0, 3000.0)).length() < 5.0, "at the departure's end: {out}");
        assert!(fastest <= 40.5, "the corridor's 40 m/s: {fastest}");
        assert!((240.0..=300.0).contains(&took), "about 4.4 minutes from the clamps (patrol-mission 4.2): {took}");
        assert!(d.patrol.as_ref().unwrap().undocked);
    }

    #[test]
    fn the_station_locks_the_jump_until_the_tern_is_clear_of_it() {
        let (mut d, s) = patrolling(&[Station::Helm, Station::Engineering]);
        let g = d.drive_group().unwrap() as u8;
        d.command(s[1], Command::Power(PowerCmd::Breaker(g))).unwrap();
        d.command(s[1], Command::Shore(false)).unwrap();
        run_until(&mut d, 30.0, |d| d.drive_ready());
        d.command(s[0], Command::Undock).unwrap();
        // In the throat on the axis: still inside the station's lock.
        run_until(&mut d, 300.0, |d| d.patrol.as_ref().is_some_and(|p| p.leg == 3));
        assert_eq!(d.mass_lock().as_deref(), Some("Tamsin Station"));
        run_until(&mut d, 120.0, |d| d.patrol.as_ref().is_some_and(|p| p.stage == patrol::Stage::Underway));
        assert_eq!(d.mass_lock(), None, "the departure ends clear of the lock");
    }

    #[test]
    fn a_crew_of_automation_undocks_jumps_sweeps_and_meets_the_hound() {
        // One person at Tactical; every other seat is automation playing the patrol by the bots' rule (design 9).
        let (mut d, _) = patrolling(&[Station::Tactical]);
        let out = run_until(&mut d, 360.0, |d| d.patrol.as_ref().is_some_and(|p| p.undocked));
        let jump = run_until(&mut d, 180.0, |d| d.patrol.as_ref().is_some_and(|p| p.jumps >= 1));
        println!("automation: clear of the berth after {out:.1} s, the first jump {jump:.1} s later");
        let p = d.patrol.as_ref().unwrap();
        assert_eq!(p.at, 1, "at the Pell beacon");
        let t = &d.ships[0];
        assert!(t.pos.length() < 1.0, "the drill's origin moved with the Tern");
        let beacon = &d.data.patrol.as_ref().unwrap().places[1];
        assert!((p.origin - beacon.arrival).length() <= 1_500.0 + 1e-6, "within the arrival scatter");
        run_until(&mut d, 120.0, |d| d.patrol.as_ref().unwrap().swept[1]);
        run_until(&mut d, 240.0, |d| d.phase == Phase::Engage);
        let p = d.patrol.as_ref().unwrap();
        assert_eq!((p.at, p.met), (2, true), "the fight at the relay");
        let ahead = d.ships[0].rot.inverse() * (d.ships[1].pos - d.ships[0].pos);
        let want = DVec3::from_array(d.data.mission.enemy_start_m);
        assert!((ahead - want).length() < 1.0, "the Hound placed from the arrival as the drill places it: {ahead}");
    }

    #[test]
    fn after_the_fight_the_patrol_goes_on_and_ends_home() {
        let (mut d, s) = patrolling(&[Station::Tactical]);
        run_until(&mut d, 600.0, |d| d.phase == Phase::Engage);
        let _ = s;
        // The Hound destroyed: the patrol goes on, the outcome waits for home.
        d.ships[1].hull = 0.0;
        d.ships[1].alive = false;
        run_until(&mut d, 5.0, |d| d.phase == Phase::Patrol);
        assert_eq!(d.outcome, Outcome::None);
        run_until(&mut d, 300.0, |d| d.phase == Phase::Debrief);
        assert_eq!(d.outcome, Outcome::Victory, "home after a victory: MISSION COMPLETE");
        assert_eq!(d.patrol.as_ref().unwrap().stage, patrol::Stage::Home);
    }

    #[test]
    fn the_patrols_objectives_are_measured_by_the_undock_the_sweep_and_the_return() {
        use data::{ObjectiveFacts, ObjectiveKind, ObjectiveState};
        let d = DrillData::shipped_patrol();
        let o = &d.mission.objectives;
        let swept = vec!["pell_beacon".to_owned()];
        let f = ObjectiveFacts {
            enemy_hull: 1.0,
            tern_hull: 1.0,
            fighters_down: 0,
            undocked: true,
            swept: &swept,
            home: false,
        };
        let state = |k: ObjectiveKind| o.iter().find(|x| x.kind == Some(k)).unwrap().measure(&f).0;
        assert_eq!(state(ObjectiveKind::Undock), ObjectiveState::Done);
        assert_eq!(state(ObjectiveKind::Sweep), ObjectiveState::Done);
        assert_eq!(state(ObjectiveKind::Return), ObjectiveState::Progress);
    }

    #[test]
    fn the_drill_without_a_patrol_refuses_the_patrols_commands() {
        let (mut d, s) = engaged_at(&[Station::Helm]);
        d.command(s[0], Command::Undock).unwrap();
        let refused = {
            d.step();
            d.drain_events().iter().any(|e| matches!(e, DrillEvent::Refused { reason: Refusal::NoSuch, .. }))
        };
        assert!(refused);
    }

    #[test]
    fn the_shipped_drill_data_loads() {
        let d = DrillData::shipped();
        assert_eq!(d.mission.enemy, "hound");
        assert_eq!(d.tern_combat.turrets.len(), 4);
    }

    #[test]
    fn each_objective_kind_is_measured_by_its_own_fact() {
        use data::{ObjectiveFacts, ObjectiveState::*};
        let obj = &DrillData::shipped().mission.objectives;
        let f = ObjectiveFacts {
            enemy_hull: 0.4,
            tern_hull: 0.9,
            fighters_down: 1,
            undocked: false,
            swept: &[],
            home: false,
        };
        let m: Vec<_> = obj.iter().map(|o| o.measure(&f)).collect();
        assert_eq!(m[0], (Progress, 0.6), "the Hound at 40 % is 60 % of the way to destroyed");
        assert_eq!(m[1], (Progress, 0.9), "survive reads the Tern's hull");
        assert_eq!(m[2], (Progress, 0.5), "one fighter of two");
        let end = ObjectiveFacts {
            enemy_hull: 0.0,
            tern_hull: 0.0,
            fighters_down: 2,
            undocked: false,
            swept: &[],
            home: false,
        };
        let m: Vec<_> = obj.iter().map(|o| o.measure(&end).0).collect();
        assert_eq!(m, vec![Done, Failed, Done]);
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
    fn the_captain_starts_the_mission_with_nobody_else_ready() {
        let mut d = drill();
        let cap = d.join("Capt. A", false).unwrap();
        let helm = d.join("Lt. B", false).unwrap();
        d.claim(cap, Station::Captain).unwrap();
        d.claim(helm, Station::Helm).unwrap();
        d.step();
        assert_eq!(d.phase, Phase::Muster, "nobody is ready");
        d.start_mission(cap).expect("the captain's call");
        assert_eq!(d.phase, Phase::Countdown, "START MISSION begins the countdown at once");
        run_to_engage(&mut d);
    }

    #[test]
    fn another_station_cannot_start_the_mission_while_a_person_is_captain() {
        let mut d = drill();
        let cap = d.join("Capt. A", false).unwrap();
        let helm = d.join("Lt. B", false).unwrap();
        let walker = d.join("Lt. C", false).unwrap();
        d.claim(cap, Station::Captain).unwrap();
        d.claim(helm, Station::Helm).unwrap();
        assert_eq!(d.start_mission(helm), Err(Refusal::CaptainsCall));
        assert_eq!(d.start_mission(walker), Err(Refusal::NotYourStation), "not seated: the control is on a console");
        d.step();
        assert_eq!(d.phase, Phase::Muster, "a refused start changes nothing");
    }

    #[test]
    fn with_no_person_in_the_captains_seat_a_seated_person_starts_and_a_bot_never_does() {
        let mut d = drill();
        let bot = d.join("Lt. Bot", true).unwrap();
        let me = d.join("Lt. Me", false).unwrap();
        d.claim(bot, Station::Captain).unwrap();
        assert_eq!(d.start_mission(bot), Err(Refusal::NotYourStation), "a bot is ready once seated, it never starts");
        d.claim(me, Station::Helm).unwrap();
        d.start_mission(me).expect("single player: the player starts from any seat");
        assert_eq!(d.phase, Phase::Countdown);
        assert_eq!(d.start_mission(me), Err(Refusal::WrongPhase), "the start is already under way");
    }

    #[test]
    fn the_round_still_starts_by_itself_when_everyone_is_ready() {
        let mut d = drill();
        let cap = d.join("Capt. A", false).unwrap();
        let helm = d.join("Lt. B", false).unwrap();
        d.claim(cap, Station::Captain).unwrap();
        d.claim(helm, Station::Helm).unwrap();
        d.set_ready(cap, true).unwrap();
        d.set_ready(helm, true).unwrap();
        d.step();
        assert_eq!(d.phase, Phase::Countdown, "nobody pressed START MISSION");
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
        let face = d.hit_ship(0, bow_point, -DVec3::Z, 50.0, WeaponKind::Cannon, None, 1.0);
        assert_eq!(face, 0, "a hit ahead strikes the bow face");
        assert_eq!(d.ships[0].faces[0], 0.0);
        // 40 MJ held by the shield; of the 10 that pass, the bow's armour holds 4 and the hull takes 6.
        assert!((d.ships[0].hull - (d.data.tern_combat.hull_mj - 6.0)).abs() < 1e-9, "6 MJ reach the hull");
        let worn = d.damage[0].armour.iter().copied().fold(f64::INFINITY, f64::min);
        assert!((worn - 90.0).abs() < 1e-9, "a bolt's 4 MJ take 10 points of the bow section's armour: {worn}");
    }

    /// A drill played to its end: each seat held by a bot client (the `bot` profile through the same commands a
    /// console sends) or, when false, left to the server's automation.
    /// The worst of a fight for the Tern: the most fires at once, and the most systems below nominal at once.
    #[derive(Default)]
    struct Worst {
        fires: usize,
        hurt: usize,
        /// Integrity points the Tern's systems lost over the fight, and won back.
        lost: f64,
        mended: f64,
        /// Each system's lowest integrity.
        lowest: Vec<f64>,
    }

    fn crew_drill(helm_bot: bool, tactical_bot: bool, seed: u64) -> Drill {
        crew_drill_worst(helm_bot, tactical_bot, seed).0
    }

    fn crew_drill_worst(helm_bot: bool, tactical_bot: bool, seed: u64) -> (Drill, Worst) {
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
        let mut worst = Worst::default();
        let nominal = d.data.damage.systems.nominal_from_pct;
        let mut before = d.damage[0].integrity.clone();
        worst.lowest = before.clone();
        while d.phase == Phase::Engage {
            for (b, v) in before.iter_mut().zip(&d.damage[0].integrity) {
                if *v < *b {
                    worst.lost += *b - *v;
                } else {
                    worst.mended += *v - *b;
                }
                *b = *v;
            }
            worst.fires = worst.fires.max(d.damage[0].fires.len());
            for (lo, v) in worst.lowest.iter_mut().zip(&d.damage[0].integrity) {
                *lo = lo.min(*v);
            }
            worst.hurt = worst.hurt.max(d.damage[0].integrity.iter().filter(|v| **v < nominal).count());
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
        (d, worst)
    }

    #[test]
    fn two_bots_at_helm_and_tactical_take_the_hound_to_the_wire_with_engineering_on_automation() {
        // With the power grid (power-grid 12a) the combat preset wants more than the reactor gives (question P1:
        // engineering manages the crunch); on automation the drive is shed when the guns and shields draw together, so
        // a crew with no engineer wins some and runs others to the limit. A player at Engineering is what turns it.
        let (mut damaged, mut broken, mut won) = (0, 0, 0);
        let hound_hull = DrillData::shipped().enemy.combat.hull_mj;
        for seed in 1..=6 {
            let (d, worst) = crew_drill_worst(true, true, seed);
            won += usize::from(d.outcome == Outcome::Victory);
            // The owner, 2026-10-10: "this battle show go on much longer"; armour-and-missiles 6: 5-10 minutes.
            assert!(
                (240.0..=d.data.mission.engage_limit_s + 0.5).contains(&d.stats.engage_s),
                "seed {seed}: a fight runs 4-10 minutes, took {:.0} s",
                d.stats.engage_s
            );
            // Runs bundle at their panels, so one hit there can cut a room's every way in (power-routing 4, question
            // R2): the Hound is hurt in every fight, not always taken down.
            assert!(d.ships[1].hull < 0.9 * hound_hull, "seed {seed}: the crew damages the Hound: {:?}", d.stats);
            damaged += usize::from(d.ships[0].hull < d.data.tern_combat.hull_mj);
            broken += usize::from(worst.hurt > 0);
        }
        // With the conduit boxes (power-routing 4) a hit in the forward switchboard room cuts the bow off: on
        // automation the bots win fewer; the drill stays winnable without an engineer.
        assert!(won >= 1, "the bots can still win with Engineering on automation: {won} of 6");
        // The owner, 2026-10-11: "the enemy should be damaging your ship", "people fixing the ship along the way".
        assert_eq!(damaged, 6, "the Hound damages the Tern's hull in every fight");
        assert!(broken >= 5, "and puts a system below nominal in nearly every fight: {broken} of 6");
    }

    /// The fight's numbers over seeds, for tuning (armour-and-missiles 6): `cargo test -p sc-core tuning -- --ignored
    /// --nocapture`.
    #[test]
    #[ignore]
    fn tuning_report() {
        for (label, helm, tac) in [("bots", true, true), ("automation", false, false)] {
            for seed in 1..=8 {
                let (d, worst) = crew_drill_worst(helm, tac, seed);
                let s = d.stats;
                println!(
                    "{label} seed {seed}: {:?} in {:.0} s; Tern hull {:.0}, Hound hull {:.0}; lasers {}/{} vs {}/{}; \
                     missiles {}/{} vs {}/{}; decoys {} fooled {}; armour stripped T {} H {}; fighters down {}; Tern systems below 25 % {}, \
                     below 75 % {}; at worst {} fires, {} systems below nominal; points lost {:.0}, mended {:.0}",
                    d.outcome,
                    s.engage_s,
                    d.ships[0].hull,
                    d.ships[1].hull,
                    s.tern_laser_hits,
                    s.tern_laser_shots,
                    s.enemy_laser_hits,
                    s.enemy_laser_shots,
                    s.missile_hits,
                    s.missiles_fired,
                    s.enemy_missile_hits,
                    s.enemy_missiles_fired,
                    s.decoys_thrown,
                    s.missiles_fooled,
                    d.damage[0].armour.iter().filter(|a| **a < 1.0).count(),
                    d.damage[1].armour.iter().filter(|a| **a < 1.0).count(),
                    s.fighters_down,
                    d.damage[0].integrity.iter().filter(|v| **v < 25.0).count(),
                    d.damage[0].integrity.iter().filter(|v| **v < 75.0).count(),
                    worst.fires,
                    worst.hurt,
                    worst.lost,
                    worst.mended,
                );
                let hurt: Vec<String> = d
                    .data
                    .tern_layout
                    .loads
                    .iter()
                    .zip(&worst.lowest)
                    .filter(|(_, v)| **v < 95.0)
                    .map(|(l, v)| format!("{} {v:.0}", l.id))
                    .collect();
                println!("    hurt: {hurt:?}");
            }
        }
    }

    /// The Tern's grid through a fight with bots at Helm and Tactical, every 30 s: `cargo test -p sc-core --release
    /// power_in_the_fight -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn power_in_the_fight() {
        let seeds: Vec<u64> = std::env::var("SEEDS")
            .ok()
            .map_or(vec![1, 2, 3, 4], |v| v.split(',').filter_map(|x| x.parse().ok()).collect());
        for seed in seeds {
            let mut d = Drill::new(DrillData::shipped(), seed);
            let h = d.join("Helm", true).unwrap();
            let t = d.join("Tactical", true).unwrap();
            d.claim(h, Station::Helm).unwrap();
            d.claim(t, Station::Tactical).unwrap();
            d.set_ready(h, true).unwrap();
            d.set_ready(t, true).unwrap();
            run_to_engage(&mut d);
            let bot = d.data.profile("bot").clone();
            let mut mem = automation::Memory::default();
            let mut next = 0.0;
            let mut was = d.power.reactor.state;
            while d.phase == Phase::Engage {
                if d.power.reactor.state != was {
                    was = d.power.reactor.state;
                    let aux: Vec<String> = ["reactor_aux_p", "reactor_aux_s"]
                        .iter()
                        .filter_map(|id| d.data.power.load(id))
                        .map(|k| {
                            format!("{:.0}% {:.2}/{:.2}", d.damage[0].integrity[k], d.power.alloc[k], d.power.demand[k])
                        })
                        .collect();
                    println!(
                        "seed {seed} t {:>5.1} reactor {:?} cause {:?} aux {aux:?} soc {:.0} loop {:.0} blanket {:.0}",
                        d.phase_s,
                        was,
                        d.power.reactor.scram_cause,
                        d.power.battery.soc_mj,
                        d.power.lp.t_k,
                        d.power.reactor.t_k
                    );
                }
                let pic = d.picture();
                let now = d.phase_s;
                for c in automation::helm(&pic, &bot.helm, &d.data.tern_flight, &mut mem.helm, now) {
                    let _ = d.command(h, c);
                }
                for c in automation::tactical(&pic, &bot.tactical, &d.data, &mut mem.tactical, now) {
                    let _ = d.command(t, c);
                }
                d.step();
                if d.phase_s >= next {
                    next += 30.0;
                    let p = &d.power;
                    let (want, got) = p.totals();
                    let dark: Vec<&str> = d
                        .data
                        .power
                        .loads
                        .iter()
                        .enumerate()
                        .filter(|(i, l)| l.file.is_some() && p.demand[*i] > 1e-6 && p.alloc[*i] < 1e-6)
                        .map(|(_, l)| l.id.as_str())
                        .collect();
                    let cut: Vec<&str> = d
                        .data
                        .power
                        .file
                        .conduits
                        .iter()
                        .zip(&d.damage[0].conduits)
                        .filter(|(_, c)| c.severed)
                        .map(|(k, _)| k.id.as_str())
                        .collect();
                    println!(
                        "    dark {dark:?} cut {cut:?} live {} trips {}",
                        {
                            let s0 = &d.damage[0];
                            d.power
                                .reached(&d.data.power, &s0.node_health, &s0.conduits)
                                .iter()
                                .filter(|r| !**r)
                                .count()
                        },
                        p.trips
                    );
                    let short: Vec<String> = d
                        .data
                        .power
                        .file
                        .groups
                        .iter()
                        .enumerate()
                        .filter_map(|(g, gr)| {
                            let (w, x) = d.data.power.group_loads[g]
                                .iter()
                                .fold((0.0, 0.0), |(w, x), &k| (w + p.want[k], x + p.alloc[k]));
                            (w > 0.05).then(|| format!("{} {x:.1}/{w:.1}", gr.id))
                        })
                        .collect();
                    println!(
                        "seed {seed} t {:>4.0} {:?} {:?} want {want:5.1} got {got:5.1} rx {:.2} bat {:4.1} soc {:4.0} loop {:.0}/{:.0} drive {:.2} shields {:.2} | {}",
                        d.phase_s,
                        p.preset,
                        p.reactor.state,
                        p.reactor.throttle,
                        p.bat_out,
                        p.battery.soc_mj,
                        p.lp.t_k,
                        p.lp.hot_k,
                        d.capability(0, "impulse_drive"),
                        d.capability(0, "shield_generator"),
                        short.join(", ")
                    );
                }
            }
            println!("seed {seed}: {:?} in {:.0} s", d.outcome, d.stats.engage_s);
        }
    }

    /// Where the Hound's missiles end, over the tuning seeds with bots at Helm and Tactical: hit, fooled by a decoy,
    /// shot down near the Tern, or lost far from it.
    #[test]
    #[ignore]
    fn missile_fates() {
        for seed in 1..=8u64 {
            let mut d = Drill::new(DrillData::shipped(), seed);
            let h = d.join("Helm", true).unwrap();
            let t = d.join("Tactical", true).unwrap();
            d.claim(h, Station::Helm).unwrap();
            d.claim(t, Station::Tactical).unwrap();
            d.set_ready(h, true).unwrap();
            d.set_ready(t, true).unwrap();
            run_to_engage(&mut d);
            let bot = d.data.profile("bot").clone();
            let mut mem = automation::Memory::default();
            let mut ours = std::collections::BTreeMap::new();
            let (mut hit, mut fooled, mut near, mut far) = (0, 0, 0, 0);
            while d.phase == Phase::Engage {
                let pic = d.picture();
                let now = d.phase_s;
                for c in automation::helm(&pic, &bot.helm, &d.data.tern_flight, &mut mem.helm, now) {
                    let _ = d.command(h, c);
                }
                for c in automation::tactical(&pic, &bot.tactical, &d.data, &mut mem.tactical, now) {
                    let _ = d.command(t, c);
                }
                d.step();
                for m in &d.missiles {
                    if m.owner == ENEMY_ID {
                        let r = (m.pos - d.ships[0].pos).length();
                        let e = ours.entry(m.id).or_insert((f64::MAX, false));
                        e.0 = e.0.min(r);
                    }
                }
                for e in d.drain_events() {
                    match e {
                        DrillEvent::Fooled { missile, .. } if ours.contains_key(&missile) => {
                            ours.get_mut(&missile).unwrap().1 = true;
                            fooled += 1;
                        }
                        DrillEvent::Detonated { missile, damage_mj, .. } if ours.contains_key(&missile) => {
                            let (closest, was_fooled) = ours[&missile];
                            if damage_mj > 0.0 {
                                hit += 1;
                            } else if !was_fooled && closest < 1500.0 {
                                near += 1;
                                println!(
                                    "  seed {seed} missile {missile} ended {closest:.0} m from the Tern at its closest"
                                );
                            } else if !was_fooled {
                                far += 1;
                            }
                        }
                        _ => {}
                    }
                }
            }
            println!(
                "seed {seed}: {} launched; hit {hit}, fooled {fooled}, shot down near {near}, lost far {far}",
                ours.len()
            );
        }
    }

    #[test]
    #[ignore]
    fn debug_enemy_missiles() {
        let mut d = Drill::new(DrillData::shipped(), 1);
        let h = d.join("Helm", false).unwrap();
        d.claim(h, Station::Helm).unwrap();
        d.set_ready(h, true).unwrap();
        run_to_engage(&mut d);
        let mut shot_down = 0;
        let mut expired = 0;
        let mut closest = std::collections::HashMap::new();
        let mut lowest = vec![100.0f64; d.data.tern_layout.loads.len()];
        let mut fires = 0;
        for _ in 0..(TICK_HZ as usize * 300) {
            d.step();
            for (lo, v) in lowest.iter_mut().zip(&d.damage[0].integrity) {
                *lo = lo.min(*v);
            }
            fires = fires.max(d.damage[0].fires.len());
            for m in &d.missiles {
                if m.owner == ENEMY_ID {
                    let r = (m.pos - d.ships[0].pos).length();
                    let e = closest.entry(m.id).or_insert(f64::MAX);
                    *e = e.min(r);
                }
            }
            for e in d.drain_events() {
                if let DrillEvent::Detonated { missile, damage_mj, .. } = e {
                    if closest.contains_key(&missile) && damage_mj == 0.0 {
                        if closest[&missile] < 1500.0 {
                            shot_down += 1
                        } else {
                            expired += 1
                        }
                    }
                }
            }
        }
        let low: Vec<String> = d
            .data
            .tern_layout
            .loads
            .iter()
            .zip(&lowest)
            .filter(|(_, v)| **v < 75.0)
            .map(|(l, v)| format!("{} {:.0}", l.id, v))
            .collect();
        println!("Tern systems that fell below 75 %: {low:?}; fires seen {fires}; hull {:.0}", d.ships[0].hull);
        let mut c: Vec<_> = closest.values().map(|v| *v as i64).collect();
        c.sort();
        println!("enemy missiles {} closest approaches {:?}; ended near {shot_down}, far {expired}", c.len(), c);
    }

    #[test]
    fn the_captains_repair_order_raises_the_worst_damaged_system() {
        let (mut d, _, _) = engaged();
        let c = d.join("Captain", false).unwrap();
        d.claim(c, Station::Captain).unwrap();
        let l = d.data.tern_layout.clone();
        let k = l.loads.iter().position(|x| x.id == "impulse_drive").unwrap();
        d.damage[0].integrity[k] = 20.0;
        let repair = d.data.order_verbs(Station::Engineering).iter().position(|v| v[0] == "REPAIR").unwrap();
        d.command(c, Command::Order(Station::Engineering, repair as u8)).unwrap();
        d.step();
        assert_eq!(d.repair_priority.as_deref(), Some("impulse_drive"), "the teams go to the drive first");
    }

    #[test]
    #[ignore]
    fn debug_hit_at_the_drive() {
        let mut d = drill();
        let l = d.data.tern_layout.clone();
        let k = l.loads.iter().position(|x| x.id == "impulse_drive").unwrap();
        let c = l.loads[k].centre;
        println!("drive centre {c:?}");
        // A missile's blast below the drive, aimed at the ship's middle.
        let below = d.ships[0].pos + d.ships[0].rot * (c + DVec3::new(0.0, -12.0, 0.0));
        let dir = (d.ships[0].pos - below).normalize();
        d.ships[0].faces = [0.0; 6];
        let before = d.damage[0].integrity.clone();
        d.hit_ship(0, below, dir, 50.0, WeaponKind::Missile, None, 0.5);
        let hit: Vec<String> = l
            .loads
            .iter()
            .zip(before.iter().zip(&d.damage[0].integrity))
            .filter(|(_, (a, b))| (*a - *b).abs() > 1e-9)
            .map(|(x, (a, b))| format!("{} {a:.0}->{b:.0}", x.id))
            .collect();
        let hull = d.ships[0].hull;
        println!("hull {hull:.1}; systems hit {hit:?}; events {:?}", d.drain_events());
    }

    #[test]
    fn left_to_automation_the_drill_is_not_won() {
        // The drill is a co-op test: automation alone (bridge-stations' lower competence) must not win it. It never
        // fires a missile, so the Hound's armour is never stripped and its lasers never reach its hull.
        for seed in [1, 2, 3] {
            let d = crew_drill(false, false, seed);
            assert_ne!(d.outcome, Outcome::Victory, "seed {seed}: {:?}", d.stats);
            assert!(d.ships[0].hull < d.data.tern_combat.hull_mj, "seed {seed}: the Hound damages the Tern");
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
        // The debrief waits for the teams to put the ship right, at most its longest.
        for _ in 0..(TICK_HZ * (d.data.mission.debrief_max_s + 1.0)) as usize {
            if d.phase != Phase::Debrief {
                break;
            }
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

    /// Walk a player's body to `to` as its client would: walk messages of at most 2 m, a tick apart.
    fn walk_body(d: &mut Drill, slot: u8, to: [f32; 3]) {
        for _ in 0..400 {
            let at = d.body(slot).expect("a body").pos;
            let v = [to[0] - at[0], to[1] - at[1], to[2] - at[2]];
            let len = v.iter().map(|x| x * x).sum::<f32>().sqrt();
            if len < 1e-4 {
                return;
            }
            let k = (2.0 / len).min(1.0);
            d.walked(slot, [at[0] + v[0] * k, at[1] + v[1] * k, at[2] + v[2] * k], 0.0).expect("a step of the walk");
            d.step();
        }
        panic!("the walk never arrived");
    }

    /// The law (CLAUDE.md): a player walks to a station and sits there; nothing sits them from across the ship.
    #[test]
    fn a_player_walks_to_a_seat_sits_within_reach_and_stands_up_on_their_feet() {
        let mut d = on_foot();
        let a = d.join("Ens. Holt", false).expect("joins");
        assert!(d.body(a).is_some_and(|b| b.free()), "a player joins on foot, theirs to walk");
        assert_eq!(d.claim(a, Station::Helm), Err(Refusal::TooFar), "the muster place is metres from the helm seat");
        assert_eq!(d.operator(Station::Helm), Operator::Auto);
        let helm = [1.8, 3.5, 28.2];
        walk_body(&mut d, a, [helm[0], helm[1], helm[2] - 1.2]);
        d.claim(a, Station::Helm).expect("standing 1.2 m behind the seat, in reach");
        let t = walk_until(&mut d, |d| d.operator(Station::Helm) == Operator::Player(a));
        assert!(t < 1.5, "a step and the 0.4 s sit: {t}");
        assert_eq!(d.body(a).map(|b| b.posture), Some(bodies::Posture::Seated));
        assert_eq!(d.walked(a, helm, 0.0), Err(Refusal::Walking), "a seated body is not walked");
        // Up: the station to automation at once, the body on its feet behind the seat.
        d.stand(a).expect("stands");
        assert_eq!(d.operator(Station::Helm), Operator::Auto, "the seat left goes to automation at once");
        walk_until(&mut d, |d| d.body(a).is_some_and(|b| b.free()));
        let p = d.body(a).unwrap().pos;
        assert!((p[2] - (helm[2] - 0.7)).abs() < 1e-4 && (p[0] - helm[0]).abs() < 1e-4, "behind the seat: {p:?}");
        // Across the bridge to tactical: too far from where it stood up, in reach once walked there.
        assert_eq!(d.claim(a, Station::Tactical), Err(Refusal::TooFar));
        walk_body(&mut d, a, [-1.8, 3.5, 27.4]);
        d.claim(a, Station::Tactical).expect("in reach");
        walk_until(&mut d, |d| d.operator(Station::Tactical) == Operator::Player(a));
    }

    /// All hands (all-hands 4): a player takes an extinguisher at the damage-control locker, which never empties, and
    /// sprays a fire in its room out; alone it takes three extinguishers (40 hand-seconds at 15 s of agent each).
    #[test]
    fn a_player_takes_extinguishers_at_the_locker_and_sprays_a_fire_out() {
        let mut d = on_foot();
        let a = d.join("Ens. Holt", false).expect("joins");
        assert_eq!(d.take_extinguisher(a), Err(Refusal::TooFar), "the locker is on deck B, the body on the bridge");
        assert_eq!(d.spray(a, true), Err(Refusal::NoSuch), "nothing to spray");
        let l = d.data.tern_layout.lockers[0];
        walk_body(&mut d, a, [l.x as f32 + 0.8, l.y as f32, l.z as f32]);
        d.take_extinguisher(a).expect("at the locker");
        let room = d.data.tern_layout.room("damage_control").expect("the damage control room");
        d.damage[0].teams.clear();
        d.damage[0].fires.push(damage::Fire::new(room));
        let mut taken = 1;
        let mut t = 0.0;
        while d.damage[0].fires.iter().any(|f| f.room == room) && t < 120.0 {
            if d.kits[0].agent_s <= 0.0 {
                d.take_extinguisher(a).expect("another, at the locker");
                taken += 1;
            }
            if !d.kits[0].spraying {
                d.spray(a, true).expect("sprays");
            }
            d.damage_tick();
            t += DT;
        }
        assert!(!d.damage[0].fires.iter().any(|f| f.room == room), "out within two minutes ({t:.1} s)");
        assert!((t - 40.0).abs() < 0.5, "one player puts a fire out in 40 s of spraying: {t:.1} s");
        assert_eq!(taken, 3, "15 s of agent an extinguisher");
    }

    /// Safety points (safety-points 3a and 5a; the owner, 2026-10-12: "there should be an extinguisher in every room"):
    /// a player takes an extinguisher from a full bracket they walked to; the bracket is then empty, nothing more comes
    /// from it, and it is full again 15 s later.
    #[test]
    fn a_player_takes_an_extinguisher_from_a_bracket_and_it_is_back_15_s_later() {
        let mut d = on_foot();
        let a = d.join("Ens. Holt", false).expect("joins");
        let l = &d.data.tern_layout;
        assert!(l.brackets.len() >= l.rooms.len() / 2, "brackets round the ship: {}", l.brackets.len());
        // A bracket with no other, and no locker, within 3 m.
        let lone = |k: usize| {
            let w = l.brackets[k].wall;
            l.brackets.iter().enumerate().all(|(j, o)| j == k || (o.wall - w).length() > 3.0)
                && l.lockers.iter().all(|o| (*o - w).length() > 3.0)
        };
        let k = (0..l.brackets.len()).find(|k| lone(*k)).expect("a lone bracket");
        let at = l.brackets[k].wall + l.brackets[k].into * 0.6;
        let restock = d.data.damage.fire.bracket_restock_s;
        walk_body(&mut d, a, [at.x as f32, at.y as f32, at.z as f32]);
        d.take_extinguisher(a).expect("at a full bracket");
        assert!(d.kits.iter().any(|kit| kit.slot == a && kit.agent_s > 0.0), "in hand");
        assert!(!d.damage[0].bracket_full(k), "the bracket is empty");
        assert_eq!(d.take_extinguisher(a), Err(Refusal::NoSuch), "nothing more from an empty bracket");
        let mut t = 0.0;
        while !d.damage[0].bracket_full(k) {
            d.damage_tick();
            t += DT;
            assert!(t < 60.0, "never filled");
        }
        assert!((t - restock).abs() <= 2.0 * DT, "full again {restock} s after: {t:.2} s");
        d.take_extinguisher(a).expect("full again");
    }

    /// Repairs on deck (all-hands 5): a damaged machine is repaired by a player who walked to it, round by round,
    /// each round checked by the server; from across the ship nothing docks, and a round faster than any hand lands
    /// nothing.
    #[test]
    fn a_player_repairs_a_machine_they_walked_to_round_by_round() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let data = crate::repair::data::RepairData::load_for_ship(|rel| {
            std::fs::read_to_string(root.join(rel)).map_err(|e| format!("{rel}: {e}"))
        })
        .expect("the repair data");
        let mut d = on_foot();
        d.set_repairs(data);
        let a = d.join("Ens. Holt", false).expect("joins");
        let li = d.data.tern_layout.loads.iter().position(|l| l.id == "coolant_pump_a").expect("pump A");
        d.damage[0].integrity[li] = 50.0;
        let rd = d.repairs.as_ref().unwrap();
        let job =
            rd.data.ship.jobs.iter().position(|j| j.loads.iter().any(|l| l == "coolant_pump_a")).expect("its job");
        assert_eq!(d.dock(a, job), Err(Refusal::TooFar), "the pump is on deck C, the body on the bridge");
        let c = d.data.tern_layout.loads[li].centre;
        walk_body(&mut d, a, [c.x as f32 + 1.0, c.y as f32 - 0.5, c.z as f32]);
        let first = d.dock(a, job).expect("docked at the pump");
        assert_eq!(first.round, 0);
        assert_eq!(d.round_done(a, job, 0, 0.01), Err(Refusal::BadValue), "faster than any hand: nothing lands");
        let mut round = first.round;
        let mut rounds = 0;
        loop {
            let before = d.damage[0].integrity[li];
            let next = d.round_done(a, job, round, 60.0).expect("a round landed");
            rounds += 1;
            assert!(d.damage[0].integrity[li] > before, "a landed round raises the pump");
            match next {
                Some(o) => round = o.round,
                None => break,
            }
            assert!(rounds < 20, "a job has a few rounds");
        }
        let nominal = d.data.damage.systems.nominal_from_pct;
        assert!(d.damage[0].integrity[li] >= nominal - 1e-9, "repaired: {}", d.damage[0].integrity[li]);
        assert!(d.repairs.as_ref().unwrap().book.jobs.is_empty(), "the job closes once the pump is back");
    }

    #[test]
    fn a_walk_message_cannot_carry_a_body_across_the_ship() {
        let mut d = on_foot();
        let a = d.join("Ens. Holt", false).expect("joins");
        let at = d.body(a).unwrap().pos;
        assert_eq!(d.walked(a, [at[0], at[1], at[2] + 20.0], 0.0), Err(Refusal::BadValue), "20 m in one message");
        assert_eq!(d.walked(a, [f32::NAN, 0.0, 0.0], 0.0), Err(Refusal::BadValue));
        assert_eq!(d.body(a).unwrap().pos, at, "nothing moved");
    }

    #[test]
    fn a_player_relieves_a_bot_and_nobody_walks_into_a_players_seat() {
        let mut d = on_foot();
        let bot = d.join("Lt. Venn (bot)", true).expect("joins");
        let me = d.join("Ens. Holt", false).expect("joins");
        let other = d.join("Ens. Rook", false).expect("joins");
        d.claim(bot, Station::Helm).expect("walks");
        walk_until(&mut d, |d| d.operator(Station::Helm) == Operator::Player(bot));
        walk_body(&mut d, me, [1.8, 3.5, 27.0]);
        walk_body(&mut d, other, [1.2, 3.5, 27.2]);
        d.claim(me, Station::Helm).expect("a player relieves a bot");
        assert!(d.drain_events().iter().any(|e| matches!(e, DrillEvent::Relieved { slot, .. } if *slot == bot)));
        assert_eq!(d.player(bot).and_then(|p| p.station), None, "the bot is up");
        assert_eq!(d.claim(other, Station::Helm), Err(Refusal::StationTaken), "someone is on the way");
        walk_until(&mut d, |d| d.operator(Station::Helm) == Operator::Player(me));
        assert_eq!(d.claim(other, Station::Helm), Err(Refusal::StationTaken), "a player's seat");
        walk_until(&mut d, |d| d.body(bot).is_some_and(|b| !b.busy()));
        assert_eq!(d.claim(bot, Station::Helm), Err(Refusal::StationTaken), "a bot never relieves a player");
    }

    /// A bridge with the helm, tactical and the captain's chair, straight lines between them.
    fn command_bridge() -> bodies::Bridge {
        bodies::Bridge::straight(
            vec![
                (Station::Helm, [1.8, 3.5, 28.2], 0.0),
                (Station::Tactical, [-1.8, 3.5, 28.2], 0.0),
                (Station::Captain, [0.0, 3.8, 24.2], 0.0),
            ],
            vec![[0.0, 3.5, 21.2]],
            0.9,
            2.4,
            0.4,
            0.4,
        )
        .with_reach(1.5, 1.0, 0.7)
    }

    #[test]
    fn on_foot_the_round_waits_for_the_person_to_sit_and_sitting_is_their_ready() {
        // Regression (browser-page-fit design 6): in the browser's single player the round began while the player was
        // still walking to the bridge, and a player who sat first waited on a READY that is not on screen on foot.
        let mut d = drill();
        d.set_bridge(command_bridge());
        let bot = d.join("Lt. Xa'lensha", true).expect("joins");
        let me = d.join("Lt. Holt", false).expect("joins");
        d.claim(bot, Station::Captain).expect("the bot walks to the chair");
        walk_until(&mut d, |d| d.operator(Station::Captain) == Operator::Player(bot));
        d.set_ready(bot, true).unwrap();
        for _ in 0..60 {
            d.step();
        }
        assert_eq!(d.phase, Phase::Muster, "the bot is seated and ready, but the person is still on their feet");
        walk_body(&mut d, me, [1.8, 3.5, 27.4]);
        d.claim(me, Station::Helm).expect("sits at the helm");
        walk_until(&mut d, |d| d.operator(Station::Helm) == Operator::Player(me));
        assert!(d.player(me).is_some_and(|p| p.ready), "sitting in Muster is the person's READY");
        walk_until(&mut d, |d| d.phase == Phase::Countdown);
    }

    #[test]
    fn the_helmsman_takes_the_captains_chair_from_its_bot_mid_fight_and_the_bot_takes_the_helm() {
        // The owner, 2026-10-15: "for example I'm helmsman, I should be able to sit in captains seat whenever". The
        // person's title is the lobby's rank ("Capt."), the NPC a bot (relieve-bots 1).
        let mut d = drill();
        d.set_bridge(command_bridge());
        let bot = d.join("Lt. Xa'lensha", true).expect("joins");
        let me = d.join("Capt. Raggra of House Da", false).expect("joins");
        d.claim(bot, Station::Captain).expect("the bot walks to the chair");
        walk_until(&mut d, |d| d.operator(Station::Captain) == Operator::Player(bot));
        walk_body(&mut d, me, [1.8, 3.5, 27.4]);
        d.claim(me, Station::Helm).expect("sits at the helm");
        walk_until(&mut d, |d| d.operator(Station::Helm) == Operator::Player(me));
        d.set_ready(bot, true).unwrap();
        d.set_ready(me, true).unwrap();
        run_to_engage(&mut d);
        d.stand(me).expect("up from the helm in the fight");
        assert_eq!(d.operator(Station::Helm), Operator::Auto, "the helm goes to automation at once");
        walk_until(&mut d, |d| d.body(me).is_some_and(bodies::CrewBody::free));
        walk_body(&mut d, me, [0.0, 3.8, 23.5]);
        d.claim(me, Station::Captain).expect("a person takes the captain's chair from a bot mid-fight");
        walk_until(&mut d, |d| {
            d.operator(Station::Captain) == Operator::Player(me) && d.operator(Station::Helm) == Operator::Player(bot)
        });
        assert_eq!(d.phase, Phase::Engage, "all in the fight");
    }

    #[test]
    fn a_person_takes_a_seat_a_bot_is_walking_to_and_the_bot_goes_elsewhere() {
        let mut d = on_foot();
        let bot = d.join("Lt. Venn", true).expect("joins");
        let me = d.join("Ens. Holt", false).expect("joins");
        walk_body(&mut d, me, [1.8, 3.5, 27.4]);
        d.claim(bot, Station::Helm).expect("the bot sets off");
        d.step();
        assert_eq!(d.body(bot).and_then(bodies::CrewBody::walking_to), Some(Station::Helm));
        d.claim(me, Station::Helm).expect("the person wins over a bot on its way");
        assert_ne!(d.body(bot).and_then(bodies::CrewBody::walking_to), Some(Station::Helm), "the bot turned away");
        walk_until(&mut d, |d| d.operator(Station::Helm) == Operator::Player(me));
        walk_until(&mut d, |d| d.operator(Station::Tactical) == Operator::Player(bot));
    }

    #[test]
    fn a_person_never_takes_a_seat_another_person_walks_to() {
        let mut d = on_foot();
        let a = d.join("Ens. Holt", false).expect("joins");
        let b = d.join("Ens. Rook", false).expect("joins");
        walk_body(&mut d, a, [1.8, 3.5, 27.4]);
        walk_body(&mut d, b, [1.6, 3.5, 27.4]);
        d.claim(a, Station::Helm).expect("first");
        assert_eq!(d.claim(b, Station::Helm), Err(Refusal::StationTaken));
    }

    #[test]
    fn leaving_takes_the_body_off_the_bridge() {
        let mut d = on_foot();
        let a = d.join("Ens. Holt", false).expect("joins");
        assert!(d.body(a).is_some());
        d.leave(a);
        assert!(d.bodies.is_empty());
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
    fn a_turret_out_of_rounds_holds_fire_for_its_reload_then_has_them_all_again() {
        // The owner, 2026-10-11: "limited to 100 shots with a 30 second cooldown".
        let (mut d, _, t) = engaged();
        d.command(t, Command::Lock(Some(ENEMY_ID))).unwrap();
        let gun = d.data.gun(&d.data.tern_combat.gun).clone();
        assert_eq!((gun.rounds, gun.reload_s), (100, 30.0));
        assert!(d.ships[0].turrets.iter().all(|t| t.rounds == 100), "a turret starts with every round");
        // The dorsal turret on its last round, the Hound close and bearing: it fires it and reloads.
        let k = 0;
        d.ships[0].turrets[k].rounds = 1;
        d.ships[0].turrets[k].mode = TurretMode::Target;
        d.ships[1].pos = d.ships[0].pos + d.ships[0].rot * DVec3::new(0.0, 600.0, 400.0);
        d.ships[1].vel = d.ships[0].vel;
        let mut fired = 0;
        for _ in 0..(TICK_HZ as usize * 20) {
            d.ships[1].pos = d.ships[0].pos + d.ships[0].rot * DVec3::new(0.0, 600.0, 400.0);
            d.ships[1].vel = d.ships[0].vel;
            d.step();
            fired += d.drain_events().iter().filter(|e| matches!(e, DrillEvent::Fired { owner: TERN_ID, .. })).count();
            if d.ships[0].turrets[k].rounds == 0 {
                break;
            }
        }
        assert_eq!(d.ships[0].turrets[k].rounds, 0, "the last round went");
        assert!((d.ships[0].turrets[k].reload_s - 30.0).abs() < 0.1, "and it reloads for 30 s");
        assert!(fired >= 1);
        let before = d.ships[0].turrets[k].since_fire_s;
        steps(&mut d, 29.0);
        assert!(d.ships[0].turrets[k].since_fire_s >= before + 28.9, "no bolt while it reloads");
        steps(&mut d, 1.5);
        assert_eq!(d.ships[0].turrets[k].rounds, 100, "reloaded: a hundred rounds again");
    }

    #[test]
    fn the_next_drill_waits_for_the_teams_to_put_the_ship_right() {
        let (mut d, _, _) = engaged();
        let k = d.data.tern_layout.loads.iter().position(|x| x.id == "impulse_drive").unwrap();
        d.damage[0].integrity[k] = 30.0;
        d.ships[1].hull = 0.0;
        d.ships[1].alive = false;
        while d.phase == Phase::Engage {
            d.step();
        }
        assert_eq!(d.phase, Phase::Debrief);
        assert!(!d.put_right(), "the drive is still broken when the fight ends");
        let debrief_s = d.data.mission.debrief_s;
        steps(&mut d, debrief_s + 1.0);
        assert_eq!(d.phase, Phase::Debrief, "the debrief waits while the teams work");
        let mut t = debrief_s + 1.0;
        while d.phase == Phase::Debrief && t < d.data.mission.debrief_max_s + 1.0 {
            d.step();
            t += DT;
        }
        assert_eq!(d.phase, Phase::Muster, "then the next drill");
        assert!(t < d.data.mission.debrief_max_s, "the teams fixed the drive before the longest debrief: {t:.0} s");
    }

    #[test]
    fn engineering_automation_sets_the_combat_preset_on_red_alert_and_cruise_after() {
        // A player holds the captain's chair, so the condition is the test's alone.
        let (mut d, _) = engaged_at(&[Station::Helm, Station::Captain]);
        let reaction = d.data.profile("automation").engineering.reaction_s;
        d.bridge.red_alert = true;
        steps(&mut d, reaction + 0.5);
        assert_eq!(d.power.preset.as_deref(), Some("combat"), "red alert: the combat preset after {reaction} s");
        let shields = d.data.power.file.groups.iter().position(|g| g.id == "shields").unwrap();
        let k = d.data.power.group_loads[shields][0];
        assert_eq!(d.power.setpoint[k], d.data.power.preset_setpoint("combat", "shields").unwrap());
        d.bridge.red_alert = false;
        steps(&mut d, reaction + 0.5);
        assert_eq!(d.power.preset.as_deref(), Some("cruise"), "and cruise when it ends");
    }

    #[test]
    fn a_player_at_engineering_runs_the_grid_and_the_automation_lets_it_be() {
        let (mut d, s) = engaged_at(&[Station::Engineering]);
        let e = s[0];
        let drive = d.data.power.file.groups.iter().position(|g| g.id == "drive").unwrap() as u8;
        d.command(e, Command::Power(PowerCmd::Preview(Some((drive, 0.5))))).unwrap();
        steps(&mut d, 0.5);
        let (g, sp, want, got) = d.power_preview.expect("the server answers a dragged fader");
        assert_eq!((g, sp), (drive, 0.5));
        assert!(got <= want + 1e-9 && want > 0.0, "{got} of {want} MW");
        d.command(e, Command::Power(PowerCmd::Group(drive, 0.5))).unwrap();
        d.bridge.red_alert = true;
        steps(&mut d, 5.0);
        let k = d.data.power.group_loads[usize::from(drive)][0];
        assert_eq!(d.power.setpoint[k], 0.5, "the player's fader holds: no automation on a manned station");
        assert_eq!(d.power.preset, None, "a hand on a fader breaks the preset");
        assert!(d.power_preview.is_none(), "letting go ends the preview");
        assert_eq!(
            d.command(e, Command::Power(PowerCmd::Group(drive, f64::NAN))),
            Err(Refusal::BadValue),
            "a setpoint that is not a number stops at the edge"
        );
    }

    #[test]
    fn engineering_automation_closes_a_spare_round_a_cut_feed() {
        // power-routing 3: the dorsal turret's room loses its feed; the automation brings power round the aft passage.
        let (mut d, _) = engaged_at(&[Station::Helm, Station::Captain]);
        let pd = d.data.power.clone();
        let (cut, spare) = (
            pd.file.conduits.iter().position(|k| k.id == "r_dorsal_fwd").unwrap(),
            pd.file.conduits.iter().position(|k| k.id == "r_dorsal_aft").unwrap(),
        );
        d.damage[0].conduits[cut] = damage::ConduitState { health: 0.0, severed: true, open: false };
        steps(&mut d, 1.0);
        assert!(!d.power.conduit_breaker[spare], "not before its reaction time");
        // First the comms array, whose panel the cut darkens too, swaps to its other feed (no live run spent); a
        // reaction later the spare goes in for the turret, which has no lit panel to swap to.
        let reaction = d.data.profile("automation").engineering.reaction_s;
        steps(&mut d, 2.0 * reaction + 0.5);
        assert!(d.power.on_alt[pd.load("comms").unwrap()], "the comms array is on its other feed");
        assert!(d.power.conduit_breaker[spare], "the spare through the aft passage is closed");
        assert!(!d.damage[0].conduits[spare].open, "and the damage state knows it carries power");
        steps(&mut d, 0.5);
        let turret = pd.load("turret_dorsal").unwrap();
        assert!(d.power.supply(turret) > 0.99, "the dorsal turret has its power back");
    }

    #[test]
    fn engineering_automation_swaps_a_dark_system_to_its_other_feed() {
        let (mut d, _) = engaged_at(&[Station::Helm, Station::Captain]);
        let pd = d.data.power.clone();
        let cut = pd.file.conduits.iter().position(|k| k.id == "r_shield").unwrap();
        d.damage[0].conduits[cut] = damage::ConduitState { health: 0.0, severed: true, open: false };
        let reaction = d.data.profile("automation").engineering.reaction_s;
        steps(&mut d, reaction + 1.0);
        let shields = pd.load("shield_generator").unwrap();
        assert!(d.power.on_alt[shields], "the shield generator is swapped to its other panel");
        assert!(d.power.supply(shields) > 0.99, "and has its power");
    }

    #[test]
    fn a_ship_with_no_power_cannot_push_or_raise_her_shields() {
        let (mut d, _) = engaged_at(&[Station::Helm]);
        steps(&mut d, 1.0);
        assert!(d.capability(0, "impulse_drive") > 0.99, "a powered drive works");
        // The reactor scrammed and the battery flat: nothing feeds the grid.
        d.power.scram(&d.data.power, power::ScramCause::Manual);
        d.power.battery.soc_mj = 0.0;
        d.power.lp.t_k = d.data.power.file.reactor.reset.loop_below_k + 20.0;
        steps(&mut d, 1.0);
        assert!(d.capability(0, "impulse_drive") < 0.01, "a starved drive: {}", d.capability(0, "impulse_drive"));
        assert!(d.capability(0, "shield_generator") < 0.01, "and no shields");
    }

    #[test]
    fn a_laser_bank_charges_from_empty_in_thirty_seconds() {
        let (mut d, _, _) = engaged();
        let w = d.data.laser(&d.data.tern_combat.lasers.as_ref().unwrap().laser).clone();
        assert_eq!(d.ships[0].lasers[0].charge_mj, 0.0);
        steps(&mut d, 29.0);
        assert!(d.ships[0].lasers[0].charge_mj < w.capacitor_mj, "not full before 30 s");
        steps(&mut d, 1.5);
        assert!((d.ships[0].lasers[0].charge_mj - w.capacitor_mj).abs() < 1e-6, "full at 30 s");
    }

    #[test]
    fn armour_stops_a_laser_until_it_is_stripped() {
        let mut d = drill();
        let (pos, hull) = (d.ships[1].pos, d.ships[1].hull);
        let at = pos + DVec3::X * 30.0;
        d.ships[1].faces = [0.0; 6];
        d.hit_ship(1, at, -DVec3::X, 120.0, WeaponKind::Laser, None, 1.0);
        assert_eq!(d.ships[1].hull, hull, "fresh armour holds the whole beam");
        d.damage[1].armour.fill(0.0);
        d.ships[1].faces = [0.0; 6];
        d.hit_ship(1, at, -DVec3::X, 120.0, WeaponKind::Laser, None, 1.0);
        assert!(d.ships[1].hull < hull, "with the armour gone the beam reaches the hull");
    }

    #[test]
    fn a_tube_is_armed_sixty_three_seconds_after_loading() {
        let (mut d, _, t) = engaged();
        d.command(t, Command::Load(0)).unwrap();
        d.step();
        steps(&mut d, 62.0);
        assert_ne!(d.ships[0].tubes[0].state, TubeState::Armed);
        steps(&mut d, 1.5);
        assert_eq!(d.ships[0].tubes[0].state, TubeState::Armed);
        assert_eq!(d.ships[0].magazine, d.data.tern_combat.tubes.as_ref().unwrap().magazine - 1);
    }

    #[test]
    fn a_decoy_in_a_missiles_last_seconds_fools_it_more_often() {
        let d = DrillData::shipped();
        let w = d.decoy("decoy_burst");
        let late = w.chances(2.0);
        let early = w.chances(8.0);
        assert!(late.0 + late.1 > early.0 + early.1, "{late:?} against {early:?}");
        assert!(late.0 + late.1 <= 1.0);
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

    #[test]
    fn the_hounds_shield_holds_its_hull_and_mounts_with_a_metre_to_spare() {
        let d = DrillData::shipped();
        let e = d.enemy.shields.ellipsoid;
        let (axes, c) = (DVec3::from_array(e.axes_m) - DVec3::ONE, DVec3::from_array(e.centre_m));
        let mut pts: Vec<DVec3> = Vec::new();
        for q in &d.hound_layout.sections {
            let (hb, t, b, ch) = (q.half_beam_m, q.top_m, q.bottom_m, q.chamfer_m);
            for (x, y) in [(hb, t - ch), (hb - ch, t), (hb, b + ch), (hb - ch, b)] {
                pts.extend([DVec3::new(x, y, q.z_m), DVec3::new(-x, y, q.z_m)]);
            }
        }
        let cb = &d.enemy.combat;
        pts.extend(cb.turrets.iter().map(|t| DVec3::from_array(t.position_m)));
        pts.extend(cb.lasers.iter().flat_map(|l| l.banks.iter().map(|b| DVec3::from_array(b.position_m))));
        pts.extend(cb.tubes.iter().flat_map(|t| t.muzzle_m.iter().map(|m| DVec3::from_array(*m))));
        for p in pts {
            assert!(((p - c) / axes).length() <= 1.0, "{p} is outside the Hound's shield less a metre");
        }
        assert!(e.axes_m[2] > 2.0 * e.axes_m[0], "a long ship's shield is long, not the sphere it was");
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
            d.hit_ship(0, d.ships[0].pos + DVec3::Z * 60.0, -DVec3::Z, 10.0, WeaponKind::Cannon, Some(hound_band), 1.0);
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
    fn only_the_captains_station_may_change_the_condition() {
        // The owner, 2026-10-15: "also lighting can only be changed by the captain" (bridge-stations design 7). The
        // condition is the lighting, so a red alert from any other seat is refused and changes nothing.
        let others: Vec<Station> = Station::ALL.iter().copied().filter(|s| *s != Station::Captain).collect();
        let (mut d, s) = engaged_at(&others);
        let (red, queued) = (d.bridge.red_alert, d.queue.len());
        for (slot, st) in s.iter().zip(&others) {
            assert_eq!(
                d.command(*slot, Command::Alert(!red)),
                Err(Refusal::NotYourStation),
                "{st:?} set the condition"
            );
        }
        assert_eq!(d.queue.len(), queued, "a refused condition command was queued, so it would change the lighting");
        assert!(Command::Alert(true).may(Station::Captain), "the captain's chair sets the condition");
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

    #[test]
    fn the_bridges_main_viewscreen_shows_the_captains_camera() {
        let (mut d, s) = engaged_at(&[Station::Captain, Station::Helm]);
        assert_eq!(d.bridge.main_view, 1, "FWD until the captain picks a camera");
        d.command(s[0], Command::MainView { feed: 0, chase: true, zoom: 3, orbit: None }).unwrap();
        d.step();
        assert_eq!(
            (d.bridge.main_view, d.bridge.main_chase, d.bridge.main_zoom),
            (0, true, 3),
            "the captain's TARGET, chasing, at 10x, is on it"
        );
        assert_eq!(
            d.command(s[0], Command::MainView { feed: 9, chase: false, zoom: 0, orbit: None }),
            Err(Refusal::BadValue)
        );
        assert_eq!(
            d.command(s[0], Command::MainView { feed: 0, chase: false, zoom: 5, orbit: None }),
            Err(Refusal::BadValue),
            "no zoom past 20x"
        );
        assert!(
            d.command(s[1], Command::MainView { feed: 1, chase: false, zoom: 0, orbit: None }).is_err(),
            "only the captain picks the main viewscreen's camera"
        );
        assert_eq!((d.bridge.main_view, d.bridge.main_chase, d.bridge.main_zoom), (0, true, 3));
    }
}
