//! The drill's data: the schemas of the flight, shield, ship-combat, weapon, enemy, station and mission files
//! (openspec/changes/coop-drill design sections 1-3), each validated with its units.
//!
//! They live in the core with the rules that read them, so the server, the client's previews and the tools
//! load the same structs (CLAUDE.md 6.5). The caller reads the files and hands their text to [`DrillData::parse`].

use crate::data::{parse, Checks, DataError, Validate};
use serde::Deserialize;

/// How a ship flies under full assist (flight-and-navigation section 4).
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct FlightBlock {
    /// The lowest forward speed set point, in m/s (negative is astern).
    pub speed_min_mps: f64,
    /// The highest forward speed set point, in m/s.
    pub speed_max_mps: f64,
    /// The most forward speed can rise in a second, in m/s^2.
    pub accel_fwd_mps2: f64,
    /// The most forward speed can fall in a second, in m/s^2.
    pub accel_rev_mps2: f64,
    /// The assist's time constant on velocity, in seconds.
    pub tau_v_s: f64,
    /// The assist's time constant on body rates, in seconds.
    pub tau_w_s: f64,
    /// Rate limits [yaw, pitch, roll], in deg/s.
    pub rate_limit_deg_s: [f64; 3],
    /// Angular accelerations [yaw, pitch, roll], in deg/s^2.
    pub ang_accel_deg_s2: [f64; 3],
    /// The lateral and vertical speed set points' limit, in m/s (the strafe pad's edge).
    pub strafe_max_mps: f64,
    /// An attitude order is held inside this angle, in degrees.
    pub held_deg: f64,
    /// ... and turning slower than this, in deg/s.
    pub held_dps: f64,
    /// EVADE throws new lateral and vertical set points this often, in seconds.
    pub jink_period_s: f64,
    /// ... each up to this far either way, in m/s (held to the strafe limit).
    pub jink_mps: f64,
}

impl FlightBlock {
    fn check(&self, c: &mut Checks, at: &str) {
        c.number(&format!("{at}.speed_min_mps"), self.speed_min_mps, -1000.0, 0.0);
        c.number(&format!("{at}.speed_max_mps"), self.speed_max_mps, 1.0, 5000.0);
        c.number(&format!("{at}.accel_fwd_mps2"), self.accel_fwd_mps2, 0.1, 500.0);
        c.number(&format!("{at}.accel_rev_mps2"), self.accel_rev_mps2, 0.1, 500.0);
        c.number(&format!("{at}.tau_v_s"), self.tau_v_s, 0.05, 30.0);
        c.number(&format!("{at}.tau_w_s"), self.tau_w_s, 0.05, 30.0);
        c.number(&format!("{at}.strafe_max_mps"), self.strafe_max_mps, 0.0, 1000.0);
        c.number(&format!("{at}.held_deg"), self.held_deg, 0.01, 10.0);
        c.number(&format!("{at}.held_dps"), self.held_dps, 0.01, 10.0);
        c.number(&format!("{at}.jink_period_s"), self.jink_period_s, 0.5, 60.0);
        c.number(&format!("{at}.jink_mps"), self.jink_mps, 0.0, 1000.0);
        for i in 0..3 {
            c.number(&format!("{at}.rate_limit_deg_s[{i}]"), self.rate_limit_deg_s[i], 0.1, 360.0);
            c.number(&format!("{at}.ang_accel_deg_s2[{i}]"), self.ang_accel_deg_s2[i], 0.1, 720.0);
        }
    }
}

/// `data/ships/<id>/flight.json`.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct FlightFile {
    /// `starcrew.flight/1`.
    pub schema: String,
    /// The flight block.
    pub flight: FlightBlock,
}

impl Validate for FlightFile {
    fn validate(&self, c: &mut Checks) {
        c.equals("schema", &self.schema, "starcrew.flight/1");
        self.flight.check(c, "flight");
    }
}

/// A shield preset: the cap of each face, in the order bow, stern, port, starboard, dorsal, ventral.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ShieldPreset {
    /// A stable id (`bow`).
    pub id: String,
    /// What a console calls it.
    pub name: String,
    /// The six faces' caps, in MJ.
    pub faces_mj: [f64; 6],
}

/// The shield's shape: an ellipsoid in ship axes (weapons-and-shields section 11).
#[derive(Debug, Clone, Copy, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Ellipsoid {
    /// Semi-axes along ship x (port), y (dorsal) and z (bow), metres.
    pub axes_m: [f64; 3],
    /// Its centre, ship axes, metres.
    pub centre_m: [f64; 3],
}

/// A ship's shields (weapons-and-shields section 9).
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ShieldBlock {
    /// The shield's shape, which decides the face a hit strikes.
    pub ellipsoid: Ellipsoid,
    /// Regeneration of all faces together, in MJ/s, shared by the preset's caps.
    pub regen_mj_s: f64,
    /// The presets; the first is the one a ship starts with.
    pub presets: Vec<ShieldPreset>,
}

impl ShieldBlock {
    fn check(&self, c: &mut Checks, at: &str) {
        for (i, a) in self.ellipsoid.axes_m.iter().enumerate() {
            c.number(&format!("{at}.ellipsoid.axes_m[{i}]"), *a, 0.5, 1000.0);
        }
        for (i, a) in self.ellipsoid.centre_m.iter().enumerate() {
            c.number(&format!("{at}.ellipsoid.centre_m[{i}]"), *a, -1000.0, 1000.0);
        }
        c.number(&format!("{at}.regen_mj_s"), self.regen_mj_s, 0.0, 1000.0);
        c.count(&format!("{at}.presets"), self.presets.len() as i64, 1, 16);
        for (i, p) in self.presets.iter().enumerate() {
            for (f, v) in p.faces_mj.iter().enumerate() {
                c.number(&format!("{at}.presets[{i}].faces_mj[{f}]"), *v, 0.0, 10_000.0);
            }
        }
    }
}

/// `data/ships/<id>/shields.json`.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ShieldsFile {
    /// `starcrew.shields/1`.
    pub schema: String,
    /// The shield block.
    pub shields: ShieldBlock,
}

impl Validate for ShieldsFile {
    fn validate(&self, c: &mut Checks) {
        c.equals("schema", &self.schema, "starcrew.shields/1");
        self.shields.check(c, "shields");
        let caps = &self.shields.presets[0].faces_mj;
        let total: f64 = caps.iter().sum();
        for (i, p) in self.shields.presets.iter().enumerate() {
            let sum: f64 = p.faces_mj.iter().sum();
            c.number(&format!("shields.presets[{i}] (sum of faces_mj)"), sum, total - 0.5, total + 0.5);
        }
    }
}

/// Where a turret bears (the placeholder for weapons-and-shields' hull mask): the half of the sky its mount faces,
/// from `-arc_overlap_deg` below the mount's plane to its zenith.
#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Arc {
    /// Facing +Y.
    Dorsal,
    /// Facing -Y.
    Ventral,
    /// Facing +X.
    Port,
    /// Facing -X.
    Starboard,
    /// Everywhere.
    All,
}

impl Arc {
    /// The mount's facing, ship axes (none for a turret that bears everywhere).
    pub fn facing(self) -> Option<glam::DVec3> {
        match self {
            Arc::Dorsal => Some(glam::DVec3::Y),
            Arc::Ventral => Some(glam::DVec3::NEG_Y),
            Arc::Port => Some(glam::DVec3::X),
            Arc::Starboard => Some(glam::DVec3::NEG_X),
            Arc::All => None,
        }
    }
}

/// A turret's mount.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct TurretMount {
    /// A stable id.
    pub id: String,
    /// Where it bears.
    pub arc: Arc,
    /// Its muzzle, ship-local metres.
    pub position_m: [f64; 3],
}

/// A ship's missile tubes.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct TubesBlock {
    /// Tubes.
    pub count: u32,
    /// Missiles aboard at the start, not counting what is in the tubes.
    pub magazine: u32,
    /// Seconds to load a tube.
    pub load_s: f64,
    /// Seconds to arm a loaded tube.
    pub arm_s: f64,
    /// The missile's id in `data/weapons.json`.
    pub missile: String,
    /// Each tube's muzzle, ship-local metres.
    pub muzzle_m: Vec<[f64; 3]>,
}

/// How a ship locks a target.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct LockBlock {
    /// Seconds from designation to lock.
    pub time_s: f64,
    /// The range a lock holds to, in metres.
    pub range_m: f64,
}

/// What a ship fights with.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CombatBlock {
    /// Hull, in MJ of damage it takes before it is lost.
    pub hull_mj: f64,
    /// The hit sphere's radius, in metres (placeholder for the shield ellipsoid).
    pub hit_radius_m: f64,
    /// The turrets' gun, an id in `data/weapons.json`.
    pub gun: String,
    /// The turrets' aim error with the target still, in degrees (1 sigma).
    pub aim_sigma_base_deg: f64,
    /// The turrets.
    pub turrets: Vec<TurretMount>,
    /// How far past the ship's plane a dorsal or ventral turret still bears, in degrees.
    pub arc_overlap_deg: f64,
    /// Missile tubes, if the ship has any.
    #[serde(default)]
    pub tubes: Option<TubesBlock>,
    /// Locking.
    pub lock: LockBlock,
}

impl CombatBlock {
    fn check(&self, c: &mut Checks, at: &str) {
        c.number(&format!("{at}.hull_mj"), self.hull_mj, 1.0, 1e6);
        c.number(&format!("{at}.hit_radius_m"), self.hit_radius_m, 0.5, 500.0);
        c.number(&format!("{at}.aim_sigma_base_deg"), self.aim_sigma_base_deg, 0.0, 30.0);
        c.count(&format!("{at}.turrets"), self.turrets.len() as i64, 0, 16);
        c.number(&format!("{at}.arc_overlap_deg"), self.arc_overlap_deg, 0.0, 90.0);
        for (i, t) in self.turrets.iter().enumerate() {
            for (k, v) in t.position_m.iter().enumerate() {
                c.number(&format!("{at}.turrets[{i}].position_m[{k}]"), *v, -500.0, 500.0);
            }
        }
        if let Some(t) = &self.tubes {
            c.count(&format!("{at}.tubes.count"), i64::from(t.count), 1, 8);
            c.count(&format!("{at}.tubes.magazine"), i64::from(t.magazine), 0, 255);
            c.number(&format!("{at}.tubes.load_s"), t.load_s, 0.0, 600.0);
            c.number(&format!("{at}.tubes.arm_s"), t.arm_s, 0.0, 600.0);
            c.count(&format!("{at}.tubes.muzzle_m"), t.muzzle_m.len() as i64, i64::from(t.count), i64::from(t.count));
        }
        c.number(&format!("{at}.lock.time_s"), self.lock.time_s, 0.0, 60.0);
        c.number(&format!("{at}.lock.range_m"), self.lock.range_m, 100.0, 1e6);
    }
}

/// `data/ships/<id>/combat.json`.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CombatFile {
    /// `starcrew.ship-combat/1`.
    pub schema: String,
    /// The combat block.
    pub combat: CombatBlock,
}

impl Validate for CombatFile {
    fn validate(&self, c: &mut Checks) {
        c.equals("schema", &self.schema, "starcrew.ship-combat/1");
        self.combat.check(c, "combat");
    }
}

/// A gun (weapons-and-shields section 1).
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Gun {
    /// A stable id.
    pub id: String,
    /// What a console calls it.
    pub name: String,
    /// Bolts a second from one turret.
    pub rate_hz: f64,
    /// Bolt speed relative to the ship, in m/s.
    pub speed_mps: f64,
    /// Bolt life, in seconds.
    pub life_s: f64,
    /// Damage a bolt delivers, in MJ.
    pub damage_mj: f64,
    /// A turret's capacitor, in MJ.
    pub capacitor_mj: f64,
    /// The capacitor's charge rate, in MW (MJ/s).
    pub charge_mw: f64,
    /// Energy one bolt draws, in MJ.
    pub draw_mj: f64,
    /// Aim error per angular rate of the target: degrees of sigma per deg/s.
    pub sigma_rate_k: f64,
    /// A turret fires only at or above this hit chance (0-1).
    pub min_hit_chance: f64,
    /// Waste heat one bolt puts into the turret's sink, in MJ.
    pub heat_per_bolt_mj: f64,
    /// The sink: at this much heat the turret locks out, in MJ.
    pub heat_sink_mj: f64,
    /// What the sink sheds into the coolant, in MW.
    pub heat_shed_mw: f64,
    /// A locked-out turret fires again below this share of its sink (0-1).
    pub heat_resume_frac: f64,
}

/// A missile (weapons-and-shields section 6).
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MissileData {
    /// A stable id.
    pub id: String,
    /// What a console calls it.
    pub name: String,
    /// Speed out of the tube, relative to the ship, in m/s.
    pub eject_mps: f64,
    /// Boost acceleration, in m/s^2.
    pub boost_mps2: f64,
    /// Boost time, in seconds.
    pub boost_s: f64,
    /// Sustain acceleration, in m/s^2.
    pub sustain_mps2: f64,
    /// Sustain time, in seconds.
    pub sustain_s: f64,
    /// Proportional navigation's gain.
    pub nav_gain: f64,
    /// Seconds before it destroys itself.
    pub life_s: f64,
    /// The seeker's half-angle, in degrees.
    pub seeker_half_angle_deg: f64,
    /// The warhead at zero distance, in MJ.
    pub warhead_mj: f64,
    /// The distance at which the warhead's damage reaches zero, in metres.
    pub blast_radius_m: f64,
}

/// `data/weapons.json`.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct WeaponsFile {
    /// `starcrew.weapons/1`.
    pub schema: String,
    /// Guns.
    pub guns: Vec<Gun>,
    /// Missiles.
    pub missiles: Vec<MissileData>,
}

impl Validate for WeaponsFile {
    fn validate(&self, c: &mut Checks) {
        c.equals("schema", &self.schema, "starcrew.weapons/1");
        for (i, g) in self.guns.iter().enumerate() {
            let at = format!("guns[{i}]");
            c.number(&format!("{at}.rate_hz"), g.rate_hz, 0.01, 100.0);
            c.number(&format!("{at}.speed_mps"), g.speed_mps, 10.0, 1e5);
            c.number(&format!("{at}.life_s"), g.life_s, 0.05, 60.0);
            c.number(&format!("{at}.damage_mj"), g.damage_mj, 0.0, 1e4);
            c.number(&format!("{at}.capacitor_mj"), g.capacitor_mj, 0.0, 1e5);
            c.number(&format!("{at}.charge_mw"), g.charge_mw, 0.0, 1e5);
            c.number(&format!("{at}.draw_mj"), g.draw_mj, 0.0, 1e4);
            c.number(&format!("{at}.sigma_rate_k"), g.sigma_rate_k, 0.0, 10.0);
            c.number(&format!("{at}.min_hit_chance"), g.min_hit_chance, 0.0, 1.0);
            c.number(&format!("{at}.heat_per_bolt_mj"), g.heat_per_bolt_mj, 0.0, 1e3);
            c.number(&format!("{at}.heat_sink_mj"), g.heat_sink_mj, 0.01, 1e4);
            c.number(&format!("{at}.heat_shed_mw"), g.heat_shed_mw, 0.0, 1e3);
            c.number(&format!("{at}.heat_resume_frac"), g.heat_resume_frac, 0.0, 1.0);
        }
        for (i, m) in self.missiles.iter().enumerate() {
            let at = format!("missiles[{i}]");
            c.number(&format!("{at}.eject_mps"), m.eject_mps, 0.0, 1000.0);
            c.number(&format!("{at}.boost_mps2"), m.boost_mps2, 0.0, 5000.0);
            c.number(&format!("{at}.boost_s"), m.boost_s, 0.0, 600.0);
            c.number(&format!("{at}.sustain_mps2"), m.sustain_mps2, 0.0, 5000.0);
            c.number(&format!("{at}.sustain_s"), m.sustain_s, 0.0, 600.0);
            c.number(&format!("{at}.nav_gain"), m.nav_gain, 0.0, 20.0);
            c.number(&format!("{at}.life_s"), m.life_s, 1.0, 3600.0);
            c.number(&format!("{at}.seeker_half_angle_deg"), m.seeker_half_angle_deg, 1.0, 180.0);
            c.number(&format!("{at}.warhead_mj"), m.warhead_mj, 0.0, 1e5);
            c.number(&format!("{at}.blast_radius_m"), m.blast_radius_m, 0.1, 1e4);
        }
    }
}

/// How an enemy flies at the player's ship.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct AiBlock {
    /// The orbit radius it keeps, in metres.
    pub orbit_m: f64,
    /// Its speed on the orbit, in m/s.
    pub orbit_speed_mps: f64,
    /// Its speed closing to the orbit, in m/s.
    pub approach_speed_mps: f64,
    /// How far the orbit's axis leans from the drill's up, in degrees.
    pub orbit_tilt_deg: f64,
}

/// An enemy class.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct EnemyClass {
    /// A stable id (`hound`).
    pub id: String,
    /// What a console calls it.
    pub name: String,
    /// Flight.
    pub flight: FlightBlock,
    /// Shields.
    pub shields: ShieldBlock,
    /// Weapons and hull.
    pub combat: CombatBlock,
    /// Behaviour.
    pub ai: AiBlock,
}

/// `data/enemies.json`.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct EnemiesFile {
    /// `starcrew.enemies/1`.
    pub schema: String,
    /// Classes.
    pub classes: Vec<EnemyClass>,
}

impl Validate for EnemiesFile {
    fn validate(&self, c: &mut Checks) {
        c.equals("schema", &self.schema, "starcrew.enemies/1");
        for (i, e) in self.classes.iter().enumerate() {
            e.flight.check(c, &format!("classes[{i}].flight"));
            e.shields.check(c, &format!("classes[{i}].shields"));
            e.combat.check(c, &format!("classes[{i}].combat"));
            c.number(&format!("classes[{i}].ai.orbit_m"), e.ai.orbit_m, 50.0, 1e5);
            c.number(&format!("classes[{i}].ai.orbit_speed_mps"), e.ai.orbit_speed_mps, 0.0, 5000.0);
            c.number(&format!("classes[{i}].ai.approach_speed_mps"), e.ai.approach_speed_mps, 0.0, 5000.0);
            c.number(&format!("classes[{i}].ai.orbit_tilt_deg"), e.ai.orbit_tilt_deg, 0.0, 90.0);
        }
    }
}

/// How a helm plays (automation or a bot).
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct HelmProfile {
    /// Seconds between decisions.
    pub reaction_s: f64,
    /// Share of the rate limit it turns at (0-1).
    pub rate_frac: f64,
    /// Whether it hunts the enemy (bow on target, holding a standoff) or only holds heading and speed.
    pub hunt: bool,
    /// The range a hunting helm keeps, in metres.
    pub standoff_m: f64,
    /// How far off the bow line it weaves, in degrees.
    pub weave_deg: f64,
    /// One full weave, in seconds.
    pub weave_period_s: f64,
}

/// How a tactical officer plays.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct TacticalProfile {
    /// Seconds between decisions.
    pub reaction_s: f64,
    /// Weapons free inside this range, in metres.
    pub weapons_range_m: f64,
    /// Whether it loads and fires missiles.
    pub missiles: bool,
    /// Whether it turns the strongest shield face toward the threat.
    pub face_threat: bool,
}

/// A profile: both stations.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    /// `automation` or `bot`.
    pub id: String,
    /// Helm.
    pub helm: HelmProfile,
    /// Tactical.
    pub tactical: TacticalProfile,
}

/// `data/stations.json`.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct StationsFile {
    /// `starcrew.stations/1`.
    pub schema: String,
    /// Profiles.
    pub profiles: Vec<Profile>,
}

impl Validate for StationsFile {
    fn validate(&self, c: &mut Checks) {
        c.equals("schema", &self.schema, "starcrew.stations/1");
        for (i, p) in self.profiles.iter().enumerate() {
            c.number(&format!("profiles[{i}].helm.reaction_s"), p.helm.reaction_s, 0.0, 60.0);
            c.number(&format!("profiles[{i}].helm.rate_frac"), p.helm.rate_frac, 0.0, 1.0);
            c.number(&format!("profiles[{i}].helm.standoff_m"), p.helm.standoff_m, 50.0, 1e5);
            c.number(&format!("profiles[{i}].helm.weave_deg"), p.helm.weave_deg, 0.0, 90.0);
            c.number(&format!("profiles[{i}].helm.weave_period_s"), p.helm.weave_period_s, 1.0, 600.0);
            c.number(&format!("profiles[{i}].tactical.reaction_s"), p.tactical.reaction_s, 0.0, 60.0);
            c.number(&format!("profiles[{i}].tactical.weapons_range_m"), p.tactical.weapons_range_m, 0.0, 1e5);
        }
        for want in ["automation", "bot"] {
            let n = self.profiles.iter().filter(|p| p.id == want).count() as i64;
            c.count(&format!("profiles (with id {want:?})"), n, 1, 1);
        }
    }
}

/// What a station is told in the briefing.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Orders {
    /// Helm's orders.
    pub helm: String,
    /// Tactical's orders.
    pub tactical: String,
}

/// `data/missions/<id>.json`.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MissionFile {
    /// `starcrew.mission/1`.
    pub schema: String,
    /// A stable id.
    pub id: String,
    /// The title.
    pub title: String,
    /// The situation, as the briefing reads it.
    pub situation: String,
    /// The objectives.
    pub objectives: Vec<String>,
    /// Each station's orders.
    pub orders: Orders,
    /// The stations the drill uses, by id.
    pub stations: Vec<String>,
    /// The enemy's class id.
    pub enemy: String,
    /// Where the enemy starts, relative to the Tern in its frame, in metres.
    pub enemy_start_m: [f64; 3],
    /// The waypoint the helm's COURSE flies to, relative to the Tern's start in its frame, in metres.
    pub waypoint_m: [f64; 3],
    /// The Tern's forward speed at the start, in m/s.
    pub tern_start_speed_mps: f64,
    /// Countdown, in seconds.
    pub countdown_s: f64,
    /// Engage ends with the enemy withdrawn after this many seconds.
    pub engage_limit_s: f64,
    /// Debrief, in seconds.
    pub debrief_s: f64,
    /// How long a bot client reads the briefing, in seconds.
    pub bot_ready_s: f64,
}

impl Validate for MissionFile {
    fn validate(&self, c: &mut Checks) {
        c.equals("schema", &self.schema, "starcrew.mission/1");
        c.count("title (characters)", self.title.chars().count() as i64, 1, 60);
        c.count("objectives", self.objectives.len() as i64, 1, 6);
        c.count("stations", self.stations.len() as i64, 1, 2);
        for (i, s) in self.stations.iter().enumerate() {
            if crate::combat::Station::from_id(s).is_none() {
                c.equals(&format!("stations[{i}]"), s, "helm or tactical");
            }
        }
        for (k, v) in self.enemy_start_m.iter().enumerate() {
            c.number(&format!("enemy_start_m[{k}]"), *v, -50_000.0, 50_000.0);
        }
        for (k, v) in self.waypoint_m.iter().enumerate() {
            c.number(&format!("waypoint_m[{k}]"), *v, -100_000.0, 100_000.0);
        }
        c.number("tern_start_speed_mps", self.tern_start_speed_mps, -100.0, 400.0);
        c.number("countdown_s", self.countdown_s, 0.0, 120.0);
        c.number("engage_limit_s", self.engage_limit_s, 10.0, 7200.0);
        c.number("debrief_s", self.debrief_s, 1.0, 600.0);
        c.number("bot_ready_s", self.bot_ready_s, 0.0, 120.0);
    }
}

/// Everything a drill reads, loaded and cross-checked.
#[derive(Debug, Clone)]
pub struct DrillData {
    /// The Tern's flight.
    pub tern_flight: FlightBlock,
    /// The Tern's shields.
    pub tern_shields: ShieldBlock,
    /// The Tern's weapons and hull.
    pub tern_combat: CombatBlock,
    /// Guns and missiles.
    pub weapons: WeaponsFile,
    /// The enemy class the mission names.
    pub enemy: EnemyClass,
    /// Automation and bot profiles.
    pub stations: StationsFile,
    /// The mission.
    pub mission: MissionFile,
}

/// The files a drill reads, relative to the repository's root, in the order [`DrillData::parse`] takes them.
pub const DRILL_FILES: [&str; 6] = [
    "data/ships/tern/flight.json",
    "data/ships/tern/shields.json",
    "data/ships/tern/combat.json",
    "data/weapons.json",
    "data/enemies.json",
    "data/stations.json",
];

impl DrillData {
    /// Parse the six files of [`DRILL_FILES`] (their texts, in that order) and a mission file, and check that
    /// every id one names exists in another.
    pub fn parse(texts: [&str; 6], mission_file: &str, mission_text: &str) -> Result<Self, DataError> {
        let flight: FlightFile = parse(DRILL_FILES[0], texts[0])?;
        let shields: ShieldsFile = parse(DRILL_FILES[1], texts[1])?;
        let combat: CombatFile = parse(DRILL_FILES[2], texts[2])?;
        let weapons: WeaponsFile = parse(DRILL_FILES[3], texts[3])?;
        let enemies: EnemiesFile = parse(DRILL_FILES[4], texts[4])?;
        let stations: StationsFile = parse(DRILL_FILES[5], texts[5])?;
        let mission: MissionFile = parse(mission_file, mission_text)?;
        let missing = |file: &str, field: &str, id: &str| DataError {
            file: file.to_owned(),
            field: field.to_owned(),
            message: format!("names {id:?}, which no file defines"),
        };
        let enemy = enemies
            .classes
            .iter()
            .find(|e| e.id == mission.enemy)
            .cloned()
            .ok_or_else(|| missing(mission_file, "enemy", &mission.enemy))?;
        for (file, gun) in [(DRILL_FILES[2], &combat.combat.gun), (DRILL_FILES[4], &enemy.combat.gun)] {
            if !weapons.guns.iter().any(|g| &g.id == gun) {
                return Err(missing(file, "combat.gun", gun));
            }
        }
        if let Some(t) = &combat.combat.tubes {
            if !weapons.missiles.iter().any(|m| m.id == t.missile) {
                return Err(missing(DRILL_FILES[2], "combat.tubes.missile", &t.missile));
            }
        }
        Ok(Self {
            tern_flight: flight.flight,
            tern_shields: shields.shields,
            tern_combat: combat.combat,
            weapons,
            enemy,
            stations,
            mission,
        })
    }

    /// The shipped files, compiled in: for tests and for tools that run without the repository beside them.
    pub fn shipped() -> Self {
        Self::parse(
            [
                include_str!("../../../../data/ships/tern/flight.json"),
                include_str!("../../../../data/ships/tern/shields.json"),
                include_str!("../../../../data/ships/tern/combat.json"),
                include_str!("../../../../data/weapons.json"),
                include_str!("../../../../data/enemies.json"),
                include_str!("../../../../data/stations.json"),
            ],
            "data/missions/drill-hound.json",
            include_str!("../../../../data/missions/drill-hound.json"),
        )
        .expect("the shipped drill data is valid")
    }

    /// A gun by id (the ids were checked at load).
    pub fn gun(&self, id: &str) -> &Gun {
        self.weapons.guns.iter().find(|g| g.id == id).expect("checked at load")
    }

    /// A missile by id (checked at load).
    pub fn missile(&self, id: &str) -> &MissileData {
        self.weapons.missiles.iter().find(|m| m.id == id).expect("checked at load")
    }

    /// A profile by id (`automation` and `bot` are checked at load).
    pub fn profile(&self, id: &str) -> &Profile {
        self.stations.profiles.iter().find(|p| p.id == id).expect("checked at load")
    }
}
