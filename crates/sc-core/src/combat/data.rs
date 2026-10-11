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
    /// Frequency tuning, if the shield has it (weapons-and-shields 11).
    #[serde(default)]
    pub frequency: Option<FrequencyBlock>,
}

/// A shield's frequency bands (weapons-and-shields 11): a face loses `match_factor` of a hit whose weapon band
/// matches the shield's, and `retune_factor` of every hit while it retunes for `retune_s`.
#[derive(Debug, Clone, Copy, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct FrequencyBlock {
    pub match_factor: f64,
    pub retune_s: f64,
    pub retune_factor: f64,
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
        if let Some(f) = &self.frequency {
            c.number(&format!("{at}.frequency.match_factor"), f.match_factor, 0.0, 1.0);
            c.number(&format!("{at}.frequency.retune_s"), f.retune_s, 0.0, 60.0);
            c.number(&format!("{at}.frequency.retune_factor"), f.retune_factor, 1.0, 4.0);
        }
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

/// A laser bank's mount (laser-banks 1).
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct LaserMount {
    /// A stable id: its system's id in the ship's power data (`laser_port`).
    pub id: String,
    /// Where it bears.
    pub arc: Arc,
    /// Its emitter, ship-local metres.
    pub position_m: [f64; 3],
}

/// A ship's laser banks.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct LasersBlock {
    /// The weapon's id in `data/weapons.json` (`lasers`).
    pub laser: String,
    /// The banks, port first.
    pub banks: Vec<LaserMount>,
}

/// A ship's countermeasures.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct DecoysBlock {
    /// The countermeasure's id in `data/weapons.json` (`decoys`).
    pub decoy: String,
    /// How many it carries.
    pub count: u32,
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
    /// Point defence takes inbound missiles within this range, in metres (armour-and-missiles 4).
    pub pd_range_m: f64,
    /// A turret's aim error against a missile, in degrees (1 sigma): a missile is small and fast, so point defence
    /// misses far more than it does at a ship (weapons-and-shields 9).
    pub pd_aim_sigma_deg: f64,
    /// Turrets take fighters within this range after missiles, in metres (enemy-fighters 2).
    pub fighter_range_m: f64,
    /// Missile tubes, if the ship has any.
    #[serde(default)]
    pub tubes: Option<TubesBlock>,
    /// Locking.
    pub lock: LockBlock,
    /// Sensors, if the ship has a science station to run them.
    #[serde(default)]
    pub sensors: Option<SensorsBlock>,
    /// Laser banks, if the ship has any (laser-banks).
    #[serde(default)]
    pub lasers: Option<LasersBlock>,
    /// Countermeasures, if the ship has any (armour-and-missiles 3).
    #[serde(default)]
    pub decoys: Option<DecoysBlock>,
}

/// A ship's sensors: a full scan of a contact takes `scan_s` with Science manned; an active ping's ring lasts
/// `ping_s`.
#[derive(Debug, Clone, Copy, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SensorsBlock {
    pub scan_s: f64,
    pub ping_s: f64,
}

impl CombatBlock {
    fn check(&self, c: &mut Checks, at: &str) {
        c.number(&format!("{at}.hull_mj"), self.hull_mj, 1.0, 1e6);
        c.number(&format!("{at}.hit_radius_m"), self.hit_radius_m, 0.5, 500.0);
        c.number(&format!("{at}.aim_sigma_base_deg"), self.aim_sigma_base_deg, 0.0, 30.0);
        c.count(&format!("{at}.turrets"), self.turrets.len() as i64, 0, 16);
        c.number(&format!("{at}.arc_overlap_deg"), self.arc_overlap_deg, 0.0, 90.0);
        c.number(&format!("{at}.pd_range_m"), self.pd_range_m, 0.0, 1e5);
        c.number(&format!("{at}.pd_aim_sigma_deg"), self.pd_aim_sigma_deg, 0.0, 30.0);
        c.number(&format!("{at}.fighter_range_m"), self.fighter_range_m, 0.0, 1e5);
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
        if let Some(s) = &self.sensors {
            c.number(&format!("{at}.sensors.scan_s"), s.scan_s, 0.1, 600.0);
            c.number(&format!("{at}.sensors.ping_s"), s.ping_s, 0.1, 60.0);
        }
        if let Some(l) = &self.lasers {
            c.count(&format!("{at}.lasers.banks"), l.banks.len() as i64, 1, 4);
            for (i, b) in l.banks.iter().enumerate() {
                for (k, v) in b.position_m.iter().enumerate() {
                    c.number(&format!("{at}.lasers.banks[{i}].position_m[{k}]"), *v, -500.0, 500.0);
                }
            }
        }
        if let Some(d) = &self.decoys {
            c.count(&format!("{at}.decoys.count"), i64::from(d.count), 0, 50);
        }
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
    /// Its frequency band, A-D (weapons-and-shields 11).
    pub band: String,
    /// The share of a bolt's damage that counts against a ship (enemy-fighters 2: cannons wear shields slowly); a
    /// fighter or a missile takes it in full.
    pub capital_factor: f64,
    /// Bolts a turret fires before it must reload, and the reload, seconds (owner, 2026-10-11: "limited to 100 shots
    /// with a 30 second cooldown").
    pub rounds: u32,
    pub reload_s: f64,
}

impl Gun {
    /// Its band as 0-3 (A-D).
    pub fn band_index(&self) -> u8 {
        BANDS.iter().position(|b| *b == self.band).unwrap_or(0) as u8
    }
}

/// The shield frequency bands, by index.
pub const BANDS: [&str; 4] = ["A", "B", "C", "D"];

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
    /// Its radius for a point-defence bolt, in metres (armour-and-missiles 4).
    pub hit_radius_m: f64,
    /// The share of its blast a shield face meets; the rest goes on (armour-and-missiles 1).
    pub shield_share: f64,
    /// How fast its body turns toward the target while its seeker sees, degrees a second.
    pub turn_deg_s: f64,
}

/// A laser bank's weapon (laser-banks 1-2).
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct LaserData {
    /// A stable id.
    pub id: String,
    /// What a console calls it.
    pub name: String,
    /// The bank's capacitor, in MJ: a shot spends all of it.
    pub capacitor_mj: f64,
    /// Its charge rate, in MW (MJ/s).
    pub charge_mw: f64,
    /// The least share of a full capacitor it fires with (0-1).
    pub min_charge_frac: f64,
    /// Its reach, in metres.
    pub range_m: f64,
    /// Aim error with the target still, in degrees (1 sigma).
    pub sigma_base_deg: f64,
    /// Aim error per angular rate of the target: degrees of sigma per deg/s.
    pub sigma_rate_k: f64,
    /// How long a client draws the beam, in seconds.
    pub beam_s: f64,
}

/// One window of a countermeasure's effect, by the missile's time to impact (armour-and-missiles 3).
#[derive(Debug, Clone, Copy, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct DecoyWindow {
    /// The window's start, seconds to impact.
    pub from_s: f64,
    /// Its end.
    pub to_s: f64,
    /// The chance the missile is pulled off (0-1).
    pub pull_off: f64,
    /// The chance it is set off early (0-1).
    pub early: f64,
}

/// A countermeasure (armour-and-missiles 3).
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct DecoyData {
    /// A stable id.
    pub id: String,
    /// What a console calls it.
    pub name: String,
    /// How long the decoy burst lasts, in seconds.
    pub life_s: f64,
    /// Missiles within this range of the ship see it, in metres.
    pub range_m: f64,
    /// The windows, by time to impact.
    pub windows: Vec<DecoyWindow>,
    /// Outside every window: the chance of a pull-off...
    pub pull_off_else: f64,
    /// ...and of an early burst.
    pub early_else: f64,
}

impl DecoyData {
    /// The chances (pulled off, set off early) for a missile `tti_s` seconds from impact.
    pub fn chances(&self, tti_s: f64) -> (f64, f64) {
        self.windows
            .iter()
            .find(|w| tti_s >= w.from_s && tti_s < w.to_s)
            .map_or((self.pull_off_else, self.early_else), |w| (w.pull_off, w.early))
    }
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
    /// Laser banks' weapons.
    pub lasers: Vec<LaserData>,
    /// Countermeasures.
    pub decoys: Vec<DecoyData>,
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
            if !BANDS.contains(&g.band.as_str()) {
                c.equals(&format!("{at}.band"), &g.band, "A, B, C or D");
            }
            c.number(&format!("{at}.capital_factor"), g.capital_factor, 0.0, 1.0);
            c.count(&format!("{at}.rounds"), i64::from(g.rounds), 1, 100_000);
            c.number(&format!("{at}.reload_s"), g.reload_s, 0.0, 600.0);
        }
        for (i, l) in self.lasers.iter().enumerate() {
            let at = format!("lasers[{i}]");
            c.number(&format!("{at}.capacitor_mj"), l.capacitor_mj, 1.0, 1e5);
            c.number(&format!("{at}.charge_mw"), l.charge_mw, 0.01, 1e4);
            c.number(&format!("{at}.min_charge_frac"), l.min_charge_frac, 0.0, 1.0);
            c.number(&format!("{at}.range_m"), l.range_m, 100.0, 1e6);
            c.number(&format!("{at}.sigma_base_deg"), l.sigma_base_deg, 0.0, 10.0);
            c.number(&format!("{at}.sigma_rate_k"), l.sigma_rate_k, 0.0, 10.0);
            c.number(&format!("{at}.beam_s"), l.beam_s, 0.01, 10.0);
        }
        for (i, d) in self.decoys.iter().enumerate() {
            let at = format!("decoys[{i}]");
            c.number(&format!("{at}.life_s"), d.life_s, 0.1, 60.0);
            c.number(&format!("{at}.range_m"), d.range_m, 10.0, 1e5);
            c.number(&format!("{at}.pull_off_else"), d.pull_off_else, 0.0, 1.0);
            c.number(&format!("{at}.early_else"), d.early_else, 0.0, 1.0);
            for (k, w) in d.windows.iter().enumerate() {
                c.number(&format!("{at}.windows[{k}].from_s"), w.from_s, 0.0, 600.0);
                c.number(&format!("{at}.windows[{k}].to_s"), w.to_s, w.from_s, 600.0);
                c.number(&format!("{at}.windows[{k}].pull_off"), w.pull_off, 0.0, 1.0);
                c.number(&format!("{at}.windows[{k}].early"), w.early, 0.0, 1.0 - w.pull_off);
            }
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
            c.number(&format!("{at}.hit_radius_m"), m.hit_radius_m, 0.1, 100.0);
            c.number(&format!("{at}.shield_share"), m.shield_share, 0.0, 1.0);
            c.number(&format!("{at}.turn_deg_s"), m.turn_deg_s, 0.0, 3600.0);
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

/// How an engineer plays: with no power grid in the drill, it only acknowledges orders.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct EngineeringProfile {
    /// Seconds between decisions.
    pub reaction_s: f64,
}

/// How a science officer plays.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ScienceProfile {
    /// Seconds between decisions.
    pub reaction_s: f64,
    /// A scan takes this many times a manned scan's time.
    pub scan_factor: f64,
}

/// How a captain plays: auto-condition (bridge-stations 3).
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CaptainProfile {
    /// Seconds between decisions.
    pub reaction_s: f64,
    /// Red alert when a hostile comes within this range, in metres.
    pub red_within_m: f64,
    /// Normal again once every hostile has been beyond this range, in metres...
    pub normal_beyond_m: f64,
    /// ...for this long, in seconds.
    pub normal_after_s: f64,
}

/// A laser station's automation (laser-banks 3).
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct LaserProfile {
    /// Seconds between decisions.
    pub reaction_s: f64,
    /// Fires a full bank at or above this hit chance (0-1).
    pub full_min_hit_chance: f64,
    /// Fires at the least charge, when the spot it bears on has no armour, at or above this chance (0-1).
    pub bare_min_hit_chance: f64,
}

/// The gunner's automation: the turret on AUTO (enemy-fighters 3).
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GunnerProfile {
    /// Seconds between decisions.
    pub reaction_s: f64,
}

/// A profile: every station.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    /// `automation` or `bot`.
    pub id: String,
    /// Helm.
    pub helm: HelmProfile,
    /// Tactical.
    pub tactical: TacticalProfile,
    /// Engineering.
    pub engineering: EngineeringProfile,
    /// Science.
    pub science: ScienceProfile,
    /// The captain.
    pub captain: CaptainProfile,
    /// The laser stations.
    pub lasers: LaserProfile,
    /// The gunner.
    pub gunner: GunnerProfile,
}

/// `data/stations.json`.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct StationsFile {
    /// `starcrew.stations/1`.
    pub schema: String,
    /// Profiles.
    pub profiles: Vec<Profile>,
    /// The verbs the captain can send each station, [verb, icon], by station id (bridge-stations 5).
    pub orders: std::collections::BTreeMap<String, Vec<[String; 2]>>,
    /// Getting in and out of a seat (bridge-stations 6).
    pub seats: SeatTimes,
    /// A station's seat in the ship's layout where its id differs from the station's (laser-banks 3: the laser
    /// stations take the comms and flight-ops seats; enemy-fighters 3: the gunner the dorsal pod's).
    pub seat_ids: std::collections::BTreeMap<String, String>,
    /// When a bot gets up for another station (coop-drill design 9).
    pub bot_posts: BotPosts,
    /// Each station's division, by station id: what the person holding it wears (crew-uniforms 2; the divisions are
    /// `data/crew/company.json`'s).
    pub divisions: std::collections::BTreeMap<String, String>,
}

/// Getting in and out of a seat, seconds (bridge-stations 6: a 0.4 s seat snap either way).
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SeatTimes {
    /// Getting up.
    pub stand_s: f64,
    /// Sitting down.
    pub sit_s: f64,
}

/// A bot's posts (coop-drill design 9): the stations it fills, most important first, and how long a more important
/// one stands empty before it gets up for it.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct BotPosts {
    /// Station ids, most important first.
    pub posts: Vec<String>,
    /// Seconds a more important station is empty, nobody on the way, before the bot goes.
    pub reassign_s: f64,
}

impl BotPosts {
    /// The posts as stations, most important first.
    pub fn stations(&self) -> Vec<crate::combat::Station> {
        self.posts.iter().filter_map(|p| crate::combat::Station::from_id(p)).collect()
    }
}

impl Validate for StationsFile {
    fn validate(&self, c: &mut Checks) {
        c.equals("schema", &self.schema, "starcrew.stations/1");
        c.number("seats.stand_s", self.seats.stand_s, 0.0, 10.0);
        c.number("seats.sit_s", self.seats.sit_s, 0.0, 10.0);
        c.number("bot_posts.reassign_s", self.bot_posts.reassign_s, 0.0, 600.0);
        for s in crate::combat::Station::ALL {
            if !self.divisions.contains_key(s.id()) {
                c.equals(&format!("divisions.{}", s.id()), "", "a division's id for every station");
            }
        }
        for (i, p) in self.bot_posts.posts.iter().enumerate() {
            if crate::combat::Station::from_id(p).is_none() {
                c.equals(&format!("bot_posts.posts[{i}]"), p, "helm or tactical");
            }
        }
        for (i, p) in self.profiles.iter().enumerate() {
            c.number(&format!("profiles[{i}].helm.reaction_s"), p.helm.reaction_s, 0.0, 60.0);
            c.number(&format!("profiles[{i}].helm.rate_frac"), p.helm.rate_frac, 0.0, 1.0);
            c.number(&format!("profiles[{i}].helm.standoff_m"), p.helm.standoff_m, 50.0, 1e5);
            c.number(&format!("profiles[{i}].helm.weave_deg"), p.helm.weave_deg, 0.0, 90.0);
            c.number(&format!("profiles[{i}].helm.weave_period_s"), p.helm.weave_period_s, 1.0, 600.0);
            c.number(&format!("profiles[{i}].tactical.reaction_s"), p.tactical.reaction_s, 0.0, 60.0);
            c.number(&format!("profiles[{i}].tactical.weapons_range_m"), p.tactical.weapons_range_m, 0.0, 1e5);
            c.number(&format!("profiles[{i}].engineering.reaction_s"), p.engineering.reaction_s, 0.0, 60.0);
            c.number(&format!("profiles[{i}].science.reaction_s"), p.science.reaction_s, 0.0, 60.0);
            c.number(&format!("profiles[{i}].science.scan_factor"), p.science.scan_factor, 1.0, 10.0);
            c.number(&format!("profiles[{i}].captain.reaction_s"), p.captain.reaction_s, 0.0, 60.0);
            c.number(&format!("profiles[{i}].captain.red_within_m"), p.captain.red_within_m, 0.0, 1e6);
            c.number(
                &format!("profiles[{i}].captain.normal_beyond_m"),
                p.captain.normal_beyond_m,
                p.captain.red_within_m,
                1e6,
            );
            c.number(&format!("profiles[{i}].captain.normal_after_s"), p.captain.normal_after_s, 0.0, 3600.0);
        }
        for (k, verbs) in &self.orders {
            if !matches!(crate::combat::Station::from_id(k), Some(s) if s != crate::combat::Station::Captain) {
                c.equals(&format!("orders.{k}"), k, "a station the captain orders");
            }
            c.count(&format!("orders.{k}"), verbs.len() as i64, 1, 6);
            for (i, [verb, icon]) in verbs.iter().enumerate() {
                c.count(&format!("orders.{k}[{i}] (verb characters)"), verb.chars().count() as i64, 1, 12);
                c.count(&format!("orders.{k}[{i}] (icon characters)"), icon.chars().count() as i64, 1, 24);
            }
        }
        for want in ["automation", "bot"] {
            let n = self.profiles.iter().filter(|p| p.id == want).count() as i64;
            c.count(&format!("profiles (with id {want:?})"), n, 1, 1);
        }
    }
}

/// What each station is told in the briefing.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Orders {
    pub helm: String,
    pub tactical: String,
    pub engineering: String,
    pub science: String,
    pub captain: String,
    pub laser_port: String,
    pub laser_stbd: String,
    pub gunner: String,
}

impl Orders {
    /// A station's orders.
    pub fn of(&self, s: crate::combat::Station) -> &str {
        use crate::combat::Station::*;
        match s {
            Helm => &self.helm,
            Tactical => &self.tactical,
            Engineering => &self.engineering,
            Science => &self.science,
            Captain => &self.captain,
            LaserPort => &self.laser_port,
            LaserStbd => &self.laser_stbd,
            Gunner => &self.gunner,
        }
    }
}

/// What measures an objective (objectives-tab 2).
#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ObjectiveKind {
    /// The ship named by `ship` destroyed.
    Destroy,
    /// The Tern's hull above zero to the end.
    Survive,
    /// `count` of the enemy's fighters shot down.
    DestroyFighters,
}

/// One of a mission's objectives: its words, as the briefing and the OBJECTIVES tab read them, and what measures it
/// (none: the words only).
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Objective {
    /// What the crew read.
    pub words: String,
    /// What measures it.
    #[serde(default)]
    pub kind: Option<ObjectiveKind>,
    /// A destroy objective's ship: the mission's enemy class id.
    #[serde(default)]
    pub ship: Option<String>,
    /// A destroy-fighters objective's number of fighters.
    #[serde(default)]
    pub count: Option<u32>,
}

/// Where an objective stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectiveState {
    Progress,
    Done,
    Failed,
}

/// What an objective is measured against: the enemy's and the Tern's hull as shares of full, and the fighters down.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ObjectiveFacts {
    pub enemy_hull: f64,
    pub tern_hull: f64,
    pub fighters_down: u32,
}

impl Objective {
    /// Where it stands and how far it has gone (0-1), the one rule the OBJECTIVES tab draws (objectives-tab 2).
    pub fn measure(&self, f: &ObjectiveFacts) -> (ObjectiveState, f64) {
        match self.kind {
            Some(ObjectiveKind::Destroy) => {
                let h = f.enemy_hull.clamp(0.0, 1.0);
                (if h <= 0.0 { ObjectiveState::Done } else { ObjectiveState::Progress }, 1.0 - h)
            }
            Some(ObjectiveKind::Survive) => {
                let h = f.tern_hull.clamp(0.0, 1.0);
                (if h <= 0.0 { ObjectiveState::Failed } else { ObjectiveState::Progress }, h)
            }
            Some(ObjectiveKind::DestroyFighters) => {
                let n = self.count.unwrap_or(1).max(1);
                let k = (f64::from(f.fighters_down) / f64::from(n)).min(1.0);
                (if f.fighters_down >= n { ObjectiveState::Done } else { ObjectiveState::Progress }, k)
            }
            None => (ObjectiveState::Progress, 0.0),
        }
    }
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
    /// The objectives (objectives-tab 2).
    pub objectives: Vec<Objective>,
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
    /// Where the crew start and stand with no station: compartments of the Tern's layout, one a crew member in turn,
    /// so they start spread through the ship and walk to their seats (owner, 2026-10-11: "spread them throughout the
    /// ship not all in one spot"; coop-drill design 9).
    pub muster_rooms: Vec<String>,
    /// Between bodies sharing a muster room, metres.
    pub muster_spacing_m: f64,
}

impl Validate for MissionFile {
    fn validate(&self, c: &mut Checks) {
        c.equals("schema", &self.schema, "starcrew.mission/1");
        c.count("muster_rooms", self.muster_rooms.len() as i64, 1, 16);
        c.number("muster_spacing_m", self.muster_spacing_m, 0.3, 3.0);
        c.count("title (characters)", self.title.chars().count() as i64, 1, 60);
        c.count("objectives", self.objectives.len() as i64, 1, 6);
        for (i, o) in self.objectives.iter().enumerate() {
            c.count(&format!("objectives[{i}].words (characters)"), o.words.chars().count() as i64, 1, 60);
            match o.kind {
                Some(ObjectiveKind::Destroy) => {
                    c.equals(&format!("objectives[{i}].ship"), o.ship.as_deref().unwrap_or(""), &self.enemy)
                }
                Some(ObjectiveKind::DestroyFighters) => c.count(
                    &format!("objectives[{i}].count"),
                    i64::from(o.count.unwrap_or(0)),
                    1,
                    crate::combat::FIGHTER_IDS.len() as i64,
                ),
                Some(ObjectiveKind::Survive) | None => {}
            }
        }
        c.count("stations", self.stations.len() as i64, 1, crate::combat::Station::ALL.len() as i64);
        for (i, s) in self.stations.iter().enumerate() {
            if crate::combat::Station::from_id(s).is_none() {
                c.equals(
                    &format!("stations[{i}]"),
                    s,
                    "helm, tactical, engineering, science, captain, laser_port, laser_stbd or gunner",
                );
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
    /// The enemy's fighters (enemy-fighters 1).
    pub fighter: crate::combat::fighters::FighterData,
    /// The Tern's insides, for hits that break parts of it (ship-damage).
    pub tern_layout: crate::combat::damage::ShipLayout,
    /// The Hound's insides.
    pub hound_layout: crate::combat::damage::ShipLayout,
    /// The damage rules.
    pub damage: crate::combat::damage::DamageData,
}

/// How many files [`DRILL_FILES`] names: the size of the array [`DrillData::parse`] takes.
pub const DRILL_FILE_COUNT: usize = 11;

/// The files a drill reads, relative to the repository's root, in the order [`DrillData::parse`] takes them.
pub const DRILL_FILES: [&str; DRILL_FILE_COUNT] = [
    "data/ships/tern/flight.json",
    "data/ships/tern/shields.json",
    "data/ships/tern/combat.json",
    "data/weapons.json",
    "data/enemies.json",
    "data/stations.json",
    "data/fighters.json",
    crate::combat::damage::TERN_FILES[0],
    crate::combat::damage::TERN_FILES[1],
    crate::combat::damage::TERN_FILES[2],
    crate::combat::damage::HOUND_FILE,
];

impl DrillData {
    /// Parse the files of [`DRILL_FILES`] (their texts, in that order) and a mission file, and check that
    /// every id one names exists in another.
    pub fn parse(texts: [&str; DRILL_FILE_COUNT], mission_file: &str, mission_text: &str) -> Result<Self, DataError> {
        let flight: FlightFile = parse(DRILL_FILES[0], texts[0])?;
        let shields: ShieldsFile = parse(DRILL_FILES[1], texts[1])?;
        let combat: CombatFile = parse(DRILL_FILES[2], texts[2])?;
        let weapons: WeaponsFile = parse(DRILL_FILES[3], texts[3])?;
        let enemies: EnemiesFile = parse(DRILL_FILES[4], texts[4])?;
        let stations: StationsFile = parse(DRILL_FILES[5], texts[5])?;
        let fighter = crate::combat::fighters::FighterData::parse(DRILL_FILES[6], texts[6])?;
        let tern_layout = crate::combat::damage::ShipLayout::tern([texts[7], texts[8], texts[9]])?;
        let damage = crate::combat::damage::DamageData::parse(texts[9])?;
        let hound_layout = crate::combat::damage::ShipLayout::hound(texts[10])?;
        let mission: MissionFile = parse(mission_file, mission_text)?;
        let missing = |file: &str, field: &str, id: &str| DataError {
            file: file.to_owned(),
            field: field.to_owned(),
            message: format!("names {id:?}, which no file defines"),
        };
        for (i, r) in mission.muster_rooms.iter().enumerate() {
            if tern_layout.room(r).is_none() {
                return Err(missing(mission_file, &format!("muster_rooms[{i}]"), r));
            }
        }
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
        for (file, block) in [(DRILL_FILES[2], &combat.combat), (DRILL_FILES[4], &enemy.combat)] {
            if let Some(t) = &block.tubes {
                if !weapons.missiles.iter().any(|m| m.id == t.missile) {
                    return Err(missing(file, "combat.tubes.missile", &t.missile));
                }
            }
            if let Some(l) = &block.lasers {
                if !weapons.lasers.iter().any(|w| w.id == l.laser) {
                    return Err(missing(file, "combat.lasers.laser", &l.laser));
                }
            }
            if let Some(d) = &block.decoys {
                if !weapons.decoys.iter().any(|w| w.id == d.decoy) {
                    return Err(missing(file, "combat.decoys.decoy", &d.decoy));
                }
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
            fighter,
            tern_layout,
            hound_layout,
            damage,
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
                include_str!("../../../../data/fighters.json"),
                include_str!("../../../../data/ships/tern/layout.json"),
                include_str!("../../../../data/ships/tern/power.json"),
                include_str!("../../../../data/ships/tern/damage.json"),
                include_str!("../../../../data/ships/hound/layout.json"),
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

    /// A laser bank's weapon by id (checked at load).
    pub fn laser(&self, id: &str) -> &LaserData {
        self.weapons.lasers.iter().find(|l| l.id == id).expect("checked at load")
    }

    /// A countermeasure by id (checked at load).
    pub fn decoy(&self, id: &str) -> &DecoyData {
        self.weapons.decoys.iter().find(|d| d.id == id).expect("checked at load")
    }

    /// A ship's combat block: the Tern's (0) or the enemy's (1).
    pub fn combat_of(&self, ship: usize) -> &CombatBlock {
        if ship == 0 {
            &self.tern_combat
        } else {
            &self.enemy.combat
        }
    }

    /// A profile by id (`automation` and `bot` are checked at load).
    pub fn profile(&self, id: &str) -> &Profile {
        self.stations.profiles.iter().find(|p| p.id == id).expect("checked at load")
    }

    /// Where each muster room's people stand: its floor's middle, ship frame, metres (in the mission's order).
    pub fn muster_places(&self) -> Vec<[f32; 3]> {
        self.mission
            .muster_rooms
            .iter()
            .filter_map(|r| self.tern_layout.room(r))
            .map(|k| self.tern_layout.rooms[k].floor.as_vec3().to_array())
            .collect()
    }

    /// The division of whoever holds station `s` (crew-uniforms 2).
    pub fn division_of(&self, s: crate::combat::Station) -> &str {
        self.stations.divisions.get(s.id()).map_or("operations", String::as_str)
    }

    /// The layout id of a station's seat.
    pub fn seat_id(&self, s: crate::combat::Station) -> &str {
        self.stations.seat_ids.get(s.id()).map_or(s.id(), String::as_str)
    }

    /// The verbs the captain can send a station, [verb, icon] (none for a station the captain does not order).
    pub fn order_verbs(&self, s: crate::combat::Station) -> &[[String; 2]] {
        self.stations.orders.get(s.id()).map(Vec::as_slice).unwrap_or(&[])
    }
}
