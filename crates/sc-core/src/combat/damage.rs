//! Hits that damage parts of a ship (openspec/changes/ship-damage): the hull test, armour by weapon, the march into the
//! ship, the cascade through the power wiring, capability, fires and the damage-control teams. Pure: no drill, no
//! network. Workstream B builds it (docs/design/combat-2-work-plan.md); the seam is in that plan.
//!
//! It lives in the core because what a hit breaks is a rule the server, a replay and a test must agree on, and the
//! clients only draw what it says. Like every core module it never reads a file (crate::data): the caller reads
//! [`TERN_FILES`], [`HOUND_FILE`] and [`DAMAGE_FILE`] and hands their texts in, as it does `DrillData`'s.
//!
//! Everything here is in a ship's own frame: metres, +X port, +Y dorsal, +Z bow (the layout's convention). The rules
//! are `damage-control`'s model as the systems mockup runs it (`resolveHit` in `docs/mockups/lib/shipsystems.js`),
//! with `armour-and-missiles` 1 for armour by weapon, and `ship-damage` 3 and 5 for the cascade, fires and teams.
//! Every number is `damage.json`'s.

use crate::data::{parse, Checks, DataError, Validate};
use crate::repair::data::DamageRepair;
use crate::rng::Rng;
use glam::DVec3;
use serde::Deserialize;
use serde_json::Value;
use std::collections::VecDeque;

/// The Tern's files, relative to the repository's root, in the order [`ShipLayout::tern`] takes their texts.
pub const TERN_FILES: [&str; 3] =
    ["data/ships/tern/layout.json", "data/ships/tern/power.json", "data/ships/tern/damage.json"];

/// The Hound's insides (`ship-damage` 7), which [`ShipLayout::hound`] takes.
pub const HOUND_FILE: &str = "data/ships/hound/layout.json";

/// The damage rules, the same for every ship, which [`DamageData::parse`] takes.
pub const DAMAGE_FILE: &str = "data/ships/tern/damage.json";

/// The six faces of an armour section, in `damage.json`'s order: the second number of [`HitOutcome::section`].
pub const FACES: [&str; 6] = ["dorsal", "ventral", "port", "starboard", "bow", "stern"];

/// The hull test's step along a shot's path, metres (`ship-damage` 1.2); the entry is then found by bisection.
const HULL_STEP_M: f64 = 0.25;
/// The march stops when less energy than this is left, MJ (as the mockup's `resolveHit`).
const MARCH_FLOOR_MJ: f64 = 0.05;
/// Points below this from the march are not applied (as the mockup's).
const POINTS_FLOOR: f64 = 0.05;
/// A room the march left less than this in cannot catch fire, MJ (as the mockup's).
const ROOM_FLOOR_MJ: f64 = 0.01;
/// Slack for a point on a brush's face, metres.
const EPS: f64 = 1e-9;

// ------------------------------------------------------------------------------------------------------- the rules

/// `damage.json`: the rules a hit, a fire and a team follow (`damage-control`, `ship-damage`). This module reads its
/// hull, breach, propagation, systems, nodes, cascade, conduits, fire, repair and teams blocks; the rest belongs to
/// `damage-control`'s later work and is carried as it is.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct DamageData {
    /// `starcrew.ship-damage/1`.
    pub schema: String,
    /// Who reads it.
    pub status: String,
    /// The ship the file was written for; the rules hold for every ship.
    pub ship: String,
    /// The units, as words.
    pub units: Value,
    /// Armour sections.
    pub hull: ArmourRules,
    /// The hole a hit opens.
    pub breach: BreachRules,
    /// The march into the ship.
    pub propagation: MarchRules,
    /// Integrity and capability.
    pub systems: SystemRules,
    /// Power nodes.
    pub nodes: NodeRules,
    /// `reactor-cooling`'s leaks (not read here).
    pub coolant: Value,
    /// The owner's cascade (`ship-damage` 3).
    pub cascade: CascadeRules,
    /// Conduits.
    pub conduits: ConduitRules,
    /// Fires.
    pub fire: FireRules,
    /// Harm to crew (not read here: crew positions are `crew-on-deck`'s).
    pub crew: Value,
    /// Repair rates, shared with the repair rules (`crate::repair`).
    pub repair: DamageRepair,
    /// Patching breaches (not read here).
    pub patch: Value,
    /// Jammed doors (not read here).
    pub doors: Value,
    /// The magazine's cook-off (not read here: the missiles aboard are the drill's).
    pub cook_off: Value,
    /// Remote control through the computer (not read here).
    pub remote_control: Value,
    /// Damage-control teams.
    pub teams: TeamRules,
}

/// How armour takes a hit (`damage-control` 1, `armour-and-missiles` 1).
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ArmourRules {
    /// What a fresh section absorbs of a hit, MJ.
    pub armour_mj: f64,
    /// The energy that strips a section from 100 to 0, MJ.
    pub section_mj: f64,
    /// The faces, in [`FACES`]' order.
    pub faces: Vec<String>,
    /// Words.
    pub note: Option<String>,
}

/// The hole a hit opens in the first room it enters.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct BreachRules {
    /// Square metres per MJ past the armour.
    pub m2_per_mj: f64,
    /// The smallest hole, m^2.
    pub min_m2: f64,
    /// The largest, m^2.
    pub max_m2: f64,
    /// Words.
    pub note: Option<String>,
}

/// The march: the energy past the armour going on along the shot's line (`damage-control` 2).
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MarchRules {
    /// Step, metres.
    pub march_step_m: f64,
    /// How far it goes at most, metres.
    pub march_max_m: f64,
    /// The length over which it loses 1 - 1/e of what is left, metres.
    pub decay_m: f64,
    /// What a bulkhead crossed costs, MJ.
    pub bulkhead_mj: f64,
    /// The reach of a step, metres, plus `radius_per_sqrt_mj` times the square root of the energy left.
    pub radius_m: f64,
    /// Metres per square root of an MJ.
    pub radius_per_sqrt_mj: f64,
    /// Words.
    pub note: Option<String>,
}

/// Integrity and what it lets a system do.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SystemRules {
    /// Integrity points per MJ deposited.
    pub points_per_mj: f64,
    /// At or above this integrity a system does all it can, percent.
    pub nominal_from_pct: f64,
    /// Below this it does nothing, percent.
    pub disabled_below_pct: f64,
    /// Words.
    pub note: Option<String>,
}

/// Power nodes.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct NodeRules {
    /// Below this health (0-1) a node is destroyed.
    pub destroyed_below: f64,
    /// Words.
    pub note: Option<String>,
}

/// The owner's cascade (`ship-damage` 3).
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CascadeRules {
    /// The share of a hit's points on an already damaged system that surges into its node.
    pub surge_share: f64,
    /// What the surge keeps at each step.
    pub step_factor: f64,
    /// The most steps.
    pub max_steps: u32,
    /// It stops below this many points.
    pub min_points: f64,
    /// Words.
    pub note: Option<String>,
}

/// Conduits.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ConduitRules {
    /// A conduit taking this much is severed, MJ.
    pub sever_mj: f64,
    /// This much damages it, MJ.
    pub damage_mj: f64,
    /// What a damaged conduit carries, of its rating.
    pub damaged_capacity: f64,
}

/// Fires (`damage-control` 3, `ship-damage` 5).
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct FireRules {
    /// Ignition chance per MJ a hit leaves in a room.
    pub chance_per_mj: f64,
    /// The highest ignition chance.
    pub chance_max: f64,
    /// A new fire's size per MJ, kW (for `fire-spread`; not read here).
    pub seed_kw_per_mj: f64,
    /// What a fire takes from every system in its room, integrity points a second.
    pub burn_pct_per_s: f64,
    /// What it takes from the ship's structural hull, MJ a second.
    pub burn_hull_mj_per_s: f64,
    /// A fire no team has reached in this long spreads to a neighbouring room, seconds.
    pub spread_after_s: f64,
    /// A team puts a fire out in this long, seconds.
    pub put_out_s: f64,
    /// Words.
    pub note: Option<String>,
}

/// Damage-control teams (`damage-control` 6, `ship-damage` 5).
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct TeamRules {
    /// Teams aboard the Tern.
    pub count: u32,
    /// People a team.
    pub members: u32,
    /// The Tern's room they wait in.
    pub home: String,
    /// Walking, m/s.
    pub walk_m_s: f64,
    /// Running, m/s (`crew-on-deck`; not read here).
    pub run_m_s: f64,
    /// Suited, m/s (not read here).
    pub suited_m_s: f64,
    /// Up a ladder, m/s (not read here).
    pub ladder_up_m_s: f64,
    /// Down a ladder, m/s (not read here).
    pub ladder_down_m_s: f64,
    /// Putting a suit on, seconds (not read here).
    pub suit_up_s: f64,
    /// From the order to leaving, seconds.
    pub dispatch_delay_s: f64,
    /// A walk is the straight distance times this (`ship-damage` 5), until teams walk the deck (`crew-on-deck`).
    pub path_factor: f64,
    /// Below this pressure a team suits up first, kPa (not read here: air is not simulated in the drill).
    pub enter_below_kpa_needs_suit: f64,
    /// A fire this big needs suits, kW (not read here).
    pub refuse_fire_above_kw_without_suit: f64,
    /// Words.
    pub note: Option<String>,
}

impl DamageData {
    /// Parse and check `damage.json`'s text.
    pub fn parse(text: &str) -> Result<Self, DataError> {
        parse(DAMAGE_FILE, text)
    }

    /// Integrity points a team restores a second: its members, each at a rating's kit rate, two hands at the job
    /// (`ship-damage` 5: 2 x 0.6 x 1.6).
    pub fn repair_pct_per_s(&self) -> f64 {
        f64::from(self.teams.members) * self.repair.kit_pct_per_s.rating * self.repair.two_hands_factor
    }
}

impl Validate for DamageData {
    fn validate(&self, c: &mut Checks) {
        c.equals("schema", &self.schema, "starcrew.ship-damage/1");
        if self.hull.faces != FACES {
            c.equals("hull.faces", &self.hull.faces.join(","), &FACES.join(","));
        }
        let h = &self.hull;
        c.number("hull.armour_mj", h.armour_mj, 0.0, 1e4);
        c.number("hull.section_mj", h.section_mj, 1e-3, 1e5);
        let b = &self.breach;
        c.number("breach.m2_per_mj", b.m2_per_mj, 0.0, 100.0);
        c.number("breach.min_m2", b.min_m2, 0.0, b.max_m2);
        c.number("breach.max_m2", b.max_m2, 0.0, 1e3);
        let p = &self.propagation;
        c.number("propagation.march_step_m", p.march_step_m, 0.01, 5.0);
        c.number("propagation.march_max_m", p.march_max_m, 0.0, 500.0);
        c.number("propagation.decay_m", p.decay_m, 0.01, 1e3);
        c.number("propagation.bulkhead_mj", p.bulkhead_mj, 0.0, 1e4);
        c.number("propagation.radius_m", p.radius_m, 0.0, 100.0);
        c.number("propagation.radius_per_sqrt_mj", p.radius_per_sqrt_mj, 0.0, 100.0);
        let s = &self.systems;
        c.number("systems.points_per_mj", s.points_per_mj, 0.0, 1e4);
        c.number("systems.nominal_from_pct", s.nominal_from_pct, 1.0, 100.0);
        c.number("systems.disabled_below_pct", s.disabled_below_pct, 0.0, s.nominal_from_pct);
        c.number("nodes.destroyed_below", self.nodes.destroyed_below, 0.0, 1.0);
        let k = &self.cascade;
        c.number("cascade.surge_share", k.surge_share, 0.0, 1.0);
        c.number("cascade.step_factor", k.step_factor, 0.0, 1.0);
        c.count("cascade.max_steps", i64::from(k.max_steps), 0, 16);
        c.number("cascade.min_points", k.min_points, 1e-4, 1e4);
        let d = &self.conduits;
        c.number("conduits.damage_mj", d.damage_mj, 0.0, d.sever_mj);
        c.number("conduits.sever_mj", d.sever_mj, 0.0, 1e4);
        c.number("conduits.damaged_capacity", d.damaged_capacity, 0.0, 1.0);
        let f = &self.fire;
        c.number("fire.chance_per_mj", f.chance_per_mj, 0.0, 1.0);
        c.number("fire.chance_max", f.chance_max, 0.0, 1.0);
        c.number("fire.seed_kw_per_mj", f.seed_kw_per_mj, 0.0, 1e5);
        c.number("fire.burn_pct_per_s", f.burn_pct_per_s, 0.0, 100.0);
        c.number("fire.burn_hull_mj_per_s", f.burn_hull_mj_per_s, 0.0, 100.0);
        c.number("fire.spread_after_s", f.spread_after_s, 0.0, 1e4);
        c.number("fire.put_out_s", f.put_out_s, 0.01, 1e4);
        let r = &self.repair;
        c.number("repair.kit_pct_per_s.rating", r.kit_pct_per_s.rating, 0.01, 100.0);
        c.number("repair.two_hands_factor", r.two_hands_factor, 1.0, 2.0);
        let t = &self.teams;
        c.count("teams.count", i64::from(t.count), 0, 16);
        c.count("teams.members", i64::from(t.members), 1, 8);
        c.number("teams.walk_m_s", t.walk_m_s, 0.1, 20.0);
        c.number("teams.dispatch_delay_s", t.dispatch_delay_s, 0.0, 600.0);
        c.number("teams.path_factor", t.path_factor, 1.0, 5.0);
    }
}

// ------------------------------------------------------------------------------------------------------ the layout

/// One of a hull's octagonal cross-sections (the layout's `hull.sections`): lofted linearly to the next along Z.
#[derive(Debug, Clone, Copy, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct HullSection {
    /// Where along the ship, metres.
    pub z_m: f64,
    /// Half its width, metres.
    pub half_beam_m: f64,
    /// Its top, metres.
    pub top_m: f64,
    /// Its bottom, metres.
    pub bottom_m: f64,
    /// Each corner is cut at 45 degrees by this much, metres.
    pub chamfer_m: f64,
}

/// A convex prism of a room's air: its footprint (`[x, z]`, positive signed area) between a floor and a ceiling.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Brush {
    /// Floor and ceiling, metres.
    pub y: [f64; 2],
    /// The footprint's corners, `[x, z]`.
    pub poly: Vec<[f64; 2]>,
}

/// A room (a compartment), as the damage rules see it.
#[derive(Debug, Clone, PartialEq)]
pub struct Room {
    /// Its id in the layout.
    pub id: String,
    /// Its air.
    pub brushes: Vec<Brush>,
    /// The corner of its bounds with the least x, y and z.
    pub lo: DVec3,
    /// The corner with the most.
    pub hi: DVec3,
    /// A point well inside it: its largest brush's centroid at mid height (shipkit's `center`); where a team goes.
    pub centre: DVec3,
    /// The rooms a portal joins it to, in the layout's order: where a fire spreads.
    pub neighbours: Vec<usize>,
    /// The loads inside it: what a fire there burns.
    pub loads: Vec<usize>,
}

impl Room {
    /// Whether `p` is in this room's air.
    pub fn contains(&self, p: DVec3) -> bool {
        p.cmpge(self.lo - DVec3::splat(EPS)).all()
            && p.cmple(self.hi + DVec3::splat(EPS)).all()
            && self
                .brushes
                .iter()
                .any(|b| p.y >= b.y[0] - EPS && p.y <= b.y[1] + EPS && inside_convex(&b.poly, p.x, p.z))
    }
}

/// A power load: a system the damage rules break (`power.json`'s `loads`).
#[derive(Debug, Clone, PartialEq)]
pub struct Load {
    /// Its id.
    pub id: String,
    /// What [`capability`] calls it: its system's id, else its mount's, else its own.
    pub system: String,
    /// Its power node.
    pub node: usize,
    /// Where it is.
    pub centre: DVec3,
    /// The room it is in, if any (a turret's mount is outside).
    pub room: Option<usize>,
}

/// A power node: a switchboard section, a panel or a bus.
#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    /// Its id.
    pub id: String,
    /// Its room: only a march through that room reaches it.
    pub room: Option<usize>,
    /// Where it is.
    pub centre: DVec3,
    /// The loads it feeds.
    pub loads: Vec<usize>,
    /// The conduits that end at it.
    pub conduits: Vec<usize>,
}

/// A conduit between two nodes.
#[derive(Debug, Clone, PartialEq)]
pub struct Conduit {
    /// Its id.
    pub id: String,
    /// The nodes at its ends.
    pub ends: [usize; 2],
    /// The rooms it runs through: only a march through one of them reaches it.
    pub route: Vec<usize>,
    /// Its run, corner to corner.
    pub path: Vec<DVec3>,
}

/// A ship's insides, loaded once: hull sections, rooms (brush prisms), loads (system, centre, node), nodes,
/// conduits, and its damage-control teams.
#[derive(Debug, Clone, PartialEq)]
pub struct ShipLayout {
    /// Which ship.
    pub ship: String,
    /// The hull's sections, from the stern (least z) to the bow.
    pub sections: Vec<HullSection>,
    /// Its rooms.
    pub rooms: Vec<Room>,
    /// Its loads.
    pub loads: Vec<Load>,
    /// Its power nodes.
    pub nodes: Vec<Node>,
    /// Its conduits.
    pub conduits: Vec<Conduit>,
    /// How many damage-control teams it has.
    pub teams: usize,
    /// Where they wait.
    pub home: DVec3,
    /// The hull's bounds: least corner.
    pub lo: DVec3,
    /// The hull's bounds: greatest corner.
    pub hi: DVec3,
}

impl ShipLayout {
    /// The Tern, from the texts of [`TERN_FILES`] in order: its layout (hull, compartments, portals, systems,
    /// mounts), its power (loads, nodes, conduits) and its damage rules (the teams).
    pub fn tern(texts: [&str; 3]) -> Result<Self, DataError> {
        let layout: TernLayoutFile = parse(TERN_FILES[0], texts[0])?;
        let power: PowerFile = parse(TERN_FILES[1], texts[1])?;
        let rules = DamageData::parse(texts[2])?;
        let sys = |id: &str| layout.systems.iter().find(|s| s.id == id).map(|s| s.center_m);
        let mount = |id: &str| layout.mounts.iter().find(|m| m.id == id).map(|m| m.center_m);
        let mut loads = Vec::with_capacity(power.loads.len());
        for (i, l) in power.loads.iter().enumerate() {
            let missing = |what: &str, id: &str| err(TERN_FILES[1], &format!("loads.{i}.{what}"), names(id));
            let centre = match (&l.center_m, &l.mount, &l.system) {
                (Some(c), _, _) => *c,
                (None, Some(m), _) => mount(m).ok_or_else(|| missing("mount", m))?,
                (None, None, Some(s)) => sys(s).ok_or_else(|| missing("system", s))?,
                (None, None, None) => power
                    .nodes
                    .iter()
                    .find(|n| n.id == l.node)
                    .map(|n| n.center_m)
                    .ok_or_else(|| missing("node", &l.node))?,
            };
            let key = l.system.clone().or_else(|| l.mount.clone()).unwrap_or_else(|| l.id.clone());
            loads.push(LoadIn { id: l.id.clone(), system: key, node: l.node.clone(), centre });
        }
        let parts = Parts {
            ship: "tern".to_owned(),
            files: [TERN_FILES[0], TERN_FILES[1], TERN_FILES[2]],
            sections: layout.hull.sections.clone(),
            rooms: layout.compartments.iter().map(|c| (c.id.clone(), c.brushes.clone())).collect(),
            portals: layout
                .portals
                .iter()
                .filter(|p| p.kind != "window" && p.between.iter().all(|r| r != "space"))
                .map(|p| p.between.clone())
                .collect(),
            nodes: power.nodes.iter().map(|n| (n.id.clone(), Some(n.compartment.clone()), n.center_m)).collect(),
            loads,
            conduits: power
                .conduits
                .iter()
                .map(|k| ConduitIn {
                    id: k.id.clone(),
                    between: k.between.clone(),
                    route: k.route.clone(),
                    path: k.path_m.clone(),
                })
                .collect(),
            teams: rules.teams.count,
            home: rules.teams.home.clone(),
        };
        build(parts)
    }

    /// The Hound, from the text of [`HOUND_FILE`].
    pub fn hound(text: &str) -> Result<Self, DataError> {
        let f: CombatLayoutFile = parse(HOUND_FILE, text)?;
        build(Parts {
            ship: f.ship.clone(),
            files: [HOUND_FILE; 3],
            sections: f.hull.sections.clone(),
            rooms: f.rooms.iter().map(|r| (r.id.clone(), r.brushes.clone())).collect(),
            portals: f.portals.iter().map(|p| p.between.clone()).collect(),
            nodes: f.nodes.iter().map(|n| (n.id.clone(), Some(n.room.clone()), n.center_m)).collect(),
            loads: f
                .loads
                .iter()
                .map(|l| LoadIn {
                    id: l.id.clone(),
                    system: l.system.clone().unwrap_or_else(|| l.id.clone()),
                    node: l.node.clone(),
                    centre: l.center_m,
                })
                .collect(),
            conduits: f
                .conduits
                .iter()
                .map(|k| ConduitIn {
                    id: k.id.clone(),
                    between: k.between.clone(),
                    route: k.route.clone(),
                    path: k.path_m.clone(),
                })
                .collect(),
            teams: f.teams.count,
            home: f.teams.home.clone(),
        })
    }

    /// The number of spans between hull sections; a ship has six times as many armour sections.
    pub fn spans(&self) -> usize {
        self.sections.len() - 1
    }

    /// Whether any load answers to `system` (its system, mount or own id): [`capability`] of anything else is 1.
    pub fn has_system(&self, system: &str) -> bool {
        self.loads.iter().any(|l| l.system == system || l.id == system)
    }

    /// The index of the room `id`.
    pub fn room(&self, id: &str) -> Option<usize> {
        self.rooms.iter().position(|r| r.id == id)
    }

    /// The index of the load `id`.
    pub fn load(&self, id: &str) -> Option<usize> {
        self.loads.iter().position(|l| l.id == id)
    }

    /// The index of the node `id`.
    pub fn node(&self, id: &str) -> Option<usize> {
        self.nodes.iter().position(|n| n.id == id)
    }

    /// The hull's cross-section at `z`, lofted between its two neighbours; `None` beyond the bow or the stern.
    pub fn hull_at(&self, z: f64) -> Option<HullSection> {
        let s = &self.sections;
        let i = s.windows(2).position(|w| z >= w[0].z_m && z <= w[1].z_m)?;
        let (a, b) = (s[i], s[i + 1]);
        let f = if b.z_m > a.z_m { (z - a.z_m) / (b.z_m - a.z_m) } else { 0.0 };
        let mix = |x: f64, y: f64| x + (y - x) * f;
        Some(HullSection {
            z_m: z,
            half_beam_m: mix(a.half_beam_m, b.half_beam_m),
            top_m: mix(a.top_m, b.top_m),
            bottom_m: mix(a.bottom_m, b.bottom_m),
            chamfer_m: mix(a.chamfer_m, b.chamfer_m),
        })
    }

    /// Whether `p` is inside the hull's lofted octagons.
    pub fn inside_hull(&self, p: DVec3) -> bool {
        let Some(h) = self.hull_at(p.z) else { return false };
        let side = h.half_beam_m - p.x.abs();
        side >= 0.0
            && p.y <= h.top_m
            && p.y >= h.bottom_m
            && side + (h.top_m - p.y) >= h.chamfer_m
            && side + (p.y - h.bottom_m) >= h.chamfer_m
    }
}

// ------------------------------------------------------------------------------------------------------- the state

/// What a team is doing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Job {
    /// Putting out the fire in this room.
    Fire(usize),
    /// Bringing this load back to nominal.
    Repair(usize),
}

/// A fire burning in a room.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fire {
    /// The room.
    pub room: usize,
    /// How long it has burned, seconds.
    pub burning_s: f64,
    /// How long it has burned since it last spread with no team at it, seconds.
    pub unreached_s: f64,
}

impl Fire {
    /// A fire starting now in `room`.
    pub fn new(room: usize) -> Self {
        Self { room, burning_s: 0.0, unreached_s: 0.0 }
    }
}

/// A damage-control team.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Team {
    /// Where it is (where it last worked, or home).
    pub at: DVec3,
    /// Its job, if it has one.
    pub job: Option<Job>,
    /// Seconds until it reaches its job.
    pub eta_s: f64,
    /// Whether it is at its job and working.
    pub working: bool,
    /// Seconds it has worked on a fire.
    pub work_s: f64,
}

/// A conduit's state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConduitState {
    /// What it carries, of its rating: 1, `damaged_capacity`, or 0 when severed.
    pub health: f64,
    /// Cut through: it carries nothing, and no surge.
    pub severed: bool,
}

/// A hole a hit opened.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Breach {
    /// The room.
    pub room: usize,
    /// Its area, m^2 (air is not simulated in the drill yet).
    pub area_m2: f64,
}

/// The live state of a ship's insides: its armour, each load's integrity, node health, conduits, fires, breaches and
/// teams.
#[derive(Debug, Clone, PartialEq)]
pub struct DamageState {
    /// Each armour section's integrity, 0-100, at `span * 6 + face` ([`FACES`]).
    pub armour: Vec<f64>,
    /// Each load's integrity, 0-100.
    pub integrity: Vec<f64>,
    /// Each node's health, 0-1.
    pub node_health: Vec<f64>,
    /// Each conduit's state.
    pub conduits: Vec<ConduitState>,
    /// The fires, in the order they started.
    pub fires: Vec<Fire>,
    /// The holes, in the order they were made.
    pub breaches: Vec<Breach>,
    /// The damage-control teams.
    pub teams: Vec<Team>,
}

impl DamageState {
    /// A ship unhurt, its teams at home.
    pub fn new(l: &ShipLayout) -> Self {
        Self {
            armour: vec![100.0; l.spans() * FACES.len()],
            integrity: vec![100.0; l.loads.len()],
            node_health: vec![1.0; l.nodes.len()],
            conduits: vec![ConduitState { health: 1.0, severed: false }; l.conduits.len()],
            fires: Vec::new(),
            breaches: Vec::new(),
            teams: vec![Team { at: l.home, job: None, eta_s: 0.0, working: false, work_s: 0.0 }; l.teams],
        }
    }

    /// Whether `room` is burning.
    pub fn burning(&self, room: usize) -> bool {
        self.fires.iter().any(|f| f.room == room)
    }
}

// --------------------------------------------------------------------------------------------------------- the hit

/// What kind of weapon a hit came from: armour meets each its own way (`armour-and-missiles` 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Weapon {
    /// A cannon bolt: armour absorbs up to its rating and wears by `damage-control`'s rule.
    Cannon,
    /// A laser beam: any armour left holds all of it and loses nothing.
    Laser,
    /// A missile's blast: armour absorbs up to its rating and is stripped by the whole blast.
    Missile,
}

/// What a hit did: for events (`ship-damage` 6) and the structural hull.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct HitOutcome {
    /// The armour section struck: (span, face in [`FACES`]).
    pub section: (u8, u8),
    /// What the armour absorbed, MJ.
    pub absorbed_mj: f64,
    /// What passed into the ship, MJ: it also comes off the ship's structural hull.
    pub passed_mj: f64,
    /// Each load that lost integrity (by index), with the points it lost, the march's and the cascade's.
    pub systems: Vec<(usize, f64)>,
    /// Where something burst, with its size in integrity points: a system destroyed, a component the cascade hit.
    pub bursts: Vec<(DVec3, f64)>,
    /// The rooms the hit set on fire.
    pub fires: Vec<usize>,
}

/// Where the path `p0 -> p1` first goes inside the hull, or `None` when it misses (`ship-damage` 1.2). Ship frame;
/// `p0 -> p1` is a bolt's step, a beam's ray or a blast point and the ship's centre. The path is first cut to the
/// hull's bounds, so a beam from kilometres away costs what a short one does.
pub fn hull_entry(l: &ShipLayout, p0: DVec3, p1: DVec3) -> Option<DVec3> {
    if !(p0.is_finite() && p1.is_finite()) {
        return None;
    }
    if l.inside_hull(p0) {
        return Some(p0);
    }
    let d = p1 - p0;
    let len = d.length();
    if len <= EPS {
        return None;
    }
    let (t0, t1) = clip_box(p0, d, l.lo, l.hi)?;
    let step = HULL_STEP_M / len;
    let (mut prev, mut t) = (t0, t0);
    loop {
        if l.inside_hull(p0 + d * t) {
            if t <= prev {
                return Some(p0 + d * t);
            }
            let (mut out, mut inn) = (prev, t);
            for _ in 0..40 {
                let m = 0.5 * (out + inn);
                if l.inside_hull(p0 + d * m) {
                    inn = m;
                } else {
                    out = m;
                }
            }
            return Some(p0 + d * inn);
        }
        if t >= t1 {
            return None;
        }
        prev = t;
        t = (t + step).min(t1);
    }
}

/// The armour section a shot travelling along `dir` meets at `p`: (span, face). The span is the one `p` is in along
/// the ship; the face is the side the shot's main axis comes from (the mockup's `hullSection`).
pub fn section_of(l: &ShipLayout, p: DVec3, dir: DVec3) -> (usize, usize) {
    let mut span = 0;
    while span + 1 < l.spans() && p.z > l.sections[span + 1].z_m {
        span += 1;
    }
    let a = dir.abs();
    let face = if a.y >= a.x && a.y >= a.z {
        if dir.y < 0.0 {
            0
        } else {
            1
        }
    } else if a.x >= a.z {
        if dir.x < 0.0 {
            2
        } else {
            3
        }
    } else if dir.z < 0.0 {
        4
    } else {
        5
    };
    (span, face)
}

/// What passes the shield arrives here (`ship-damage` 2, `armour-and-missiles` 1): `energy_mj` at the hull at `entry`
/// going along `dir`. The armour section takes its share by `weapon`; what passes marches into the ship, damaging the
/// loads, nodes and conduits within reach, breaching the first room and maybe setting rooms on fire (decided by the
/// stream named by `seed`, `hit_id` and the room, so the same hit in the same session does the same); a hit on a
/// system already below nominal cascades (`ship-damage` 3).
#[allow(clippy::too_many_arguments)]
pub fn resolve_hit(
    l: &ShipLayout,
    s: &mut DamageState,
    cfg: &DamageData,
    entry: DVec3,
    dir: DVec3,
    energy_mj: f64,
    weapon: Weapon,
    hit_id: u64,
    seed: u64,
) -> HitOutcome {
    let mut out = HitOutcome::default();
    let dir = dir.normalize_or_zero();
    if !entry.is_finite() || dir == DVec3::ZERO || !(energy_mj.is_finite() && energy_mj > 0.0) {
        return out;
    }
    let (span, face) = section_of(l, entry, dir);
    out.section = (span as u8, face as u8);
    let k = span * FACES.len() + face;
    let armour = s.armour[k];
    let a = &cfg.hull;
    let (absorbed, loss) = match weapon {
        Weapon::Laser => (if armour > 0.0 { energy_mj } else { 0.0 }, 0.0),
        Weapon::Missile => (energy_mj.min(a.armour_mj * armour / 100.0), energy_mj / a.section_mj * 100.0),
        Weapon::Cannon => {
            (energy_mj.min(a.armour_mj * armour / 100.0), energy_mj.min(a.armour_mj) / a.section_mj * 100.0)
        }
    };
    s.armour[k] = (armour - loss).max(0.0);
    out.absorbed_mj = absorbed;
    out.passed_mj = energy_mj - absorbed;
    if out.passed_mj <= 0.0 {
        return out;
    }

    // The march, as the mockup's: each step in a room deposits dE = E (1 - exp(-step / decay)) at its point; a load,
    // a node or a conduit within r = radius + per_sqrt sqrt(E) takes points_per_mj dE (1 - d / r); a bulkhead costs
    // bulkhead_mj; the first room entered is breached.
    let pg = &cfg.propagation;
    let mut e = out.passed_mj;
    let fall = 1.0 - (-pg.march_step_m / pg.decay_m).exp();
    let end = entry + dir * pg.march_max_m;
    let r_max = pg.radius_m + pg.radius_per_sqrt_mj * e.sqrt();
    let cand_loads: Vec<usize> = (0..l.loads.len())
        .filter(|&i| closest_on_seg(l.loads[i].centre, entry, end).distance(l.loads[i].centre) < r_max)
        .collect();
    let cand_nodes: Vec<usize> = (0..l.nodes.len())
        .filter(|&i| closest_on_seg(l.nodes[i].centre, entry, end).distance(l.nodes[i].centre) < r_max)
        .collect();
    let cand_conduits: Vec<usize> = (0..l.conduits.len())
        .filter(|&i| l.conduits[i].path.windows(2).any(|w| seg_seg_dist(entry, end, w[0], w[1]) < r_max))
        .collect();
    let (lo, hi) = (entry.min(end), entry.max(end));
    let cand_rooms: Vec<usize> =
        (0..l.rooms.len()).filter(|&i| l.rooms[i].lo.cmple(hi).all() && l.rooms[i].hi.cmpge(lo).all()).collect();
    let mut load_pts = vec![0.0; l.loads.len()];
    let mut node_pts = vec![0.0; l.nodes.len()];
    let mut cond_e = vec![0.0; l.conduits.len()];
    let mut room_e = vec![0.0; l.rooms.len()];
    let (mut cur, mut entered) = (None, false);
    let steps = (pg.march_max_m / pg.march_step_m).floor() as usize;
    for i in 0..=steps {
        if e <= MARCH_FLOOR_MJ {
            break;
        }
        let p = entry + dir * (i as f64 * pg.march_step_m);
        let c = cand_rooms.iter().copied().find(|&r| l.rooms[r].contains(p));
        if let Some(room) = c.filter(|_| c != cur) {
            if entered {
                e = (e - pg.bulkhead_mj).max(0.0);
            } else {
                entered = true;
                let b = &cfg.breach;
                s.breaches.push(Breach { room, area_m2: (b.m2_per_mj * e).clamp(b.min_m2, b.max_m2) });
            }
        }
        cur = c;
        let de = e * fall;
        e -= de;
        let Some(c) = c else { continue };
        if de <= 0.0 {
            continue;
        }
        let r = pg.radius_m + pg.radius_per_sqrt_mj * (e + de).sqrt();
        room_e[c] += de;
        for &li in &cand_loads {
            let d = p.distance(l.loads[li].centre);
            if d < r {
                load_pts[li] += cfg.systems.points_per_mj * de * (1.0 - d / r);
            }
        }
        for &ni in &cand_nodes {
            let d = p.distance(l.nodes[ni].centre);
            if l.nodes[ni].room == Some(c) && d < r {
                node_pts[ni] += cfg.systems.points_per_mj * de * (1.0 - d / r);
            }
        }
        for &ci in &cand_conduits {
            let k = &l.conduits[ci];
            if !k.route.contains(&c) {
                continue;
            }
            let d = k.path.windows(2).map(|w| closest_on_seg(p, w[0], w[1]).distance(p)).fold(f64::INFINITY, f64::min);
            if d < r {
                cond_e[ci] += de * (1.0 - d / r);
            }
        }
    }

    // What the march did.
    let nominal = cfg.systems.nominal_from_pct;
    let mut taken = vec![0.0; l.loads.len()];
    let mut sources = Vec::new();
    for (li, &pts) in load_pts.iter().enumerate() {
        if pts <= POINTS_FLOOR {
            continue;
        }
        let before = s.integrity[li];
        s.integrity[li] = (before - pts).max(0.0);
        taken[li] += pts;
        if before > 0.0 && s.integrity[li] == 0.0 {
            out.bursts.push((l.loads[li].centre, pts));
        }
        if before < nominal {
            sources.push((li, pts));
        }
    }
    for (ni, &pts) in node_pts.iter().enumerate() {
        if pts > POINTS_FLOOR {
            damage_node(s, cfg, ni, pts);
        }
    }
    for (ci, &ce) in cond_e.iter().enumerate() {
        let k = &mut s.conduits[ci];
        if ce >= cfg.conduits.sever_mj {
            *k = ConduitState { health: 0.0, severed: true };
        } else if ce >= cfg.conduits.damage_mj {
            k.health = k.health.min(cfg.conduits.damaged_capacity);
        }
    }
    for (room, &re) in room_e.iter().enumerate() {
        if re <= ROOM_FLOOR_MJ || s.burning(room) {
            continue;
        }
        let chance = (cfg.fire.chance_per_mj * re).min(cfg.fire.chance_max);
        if Rng::for_purpose(seed, hit_id, &format!("hit_fire:{}", l.rooms[room].id)).next_f64() < chance {
            s.fires.push(Fire::new(room));
            out.fires.push(room);
        }
    }
    for (src, pts) in sources {
        cascade(l, s, cfg, src, pts, &mut taken, &mut out.bursts);
    }
    out.systems = taken.iter().enumerate().filter_map(|(i, &p)| (p > 0.0).then_some((i, p))).collect();
    out
}

/// A node takes `pts` integrity points as a hundredth of its health each; below `destroyed_below` it is destroyed.
fn damage_node(s: &mut DamageState, cfg: &DamageData, node: usize, pts: f64) {
    let left = (s.node_health[node] - pts / 100.0).max(0.0);
    s.node_health[node] = if left < cfg.nodes.destroyed_below { 0.0 } else { left };
}

/// A component the surge can reach.
#[derive(Clone, Copy)]
enum Part {
    Node(usize),
    Load(usize),
}

/// The owner's cascade (`ship-damage` 3) from `src`, a system already below nominal that just lost `points`: a surge of
/// `surge_share` of them goes into its node; from a node to every other load on it and, through each unsevered
/// conduit, to the node at its far end, times `step_factor` a step. Each component it reaches takes its share once,
/// and is a burst. The hit system's node passes it on; any other component only if it was below nominal before the
/// surge, so the surge runs on through damaged wiring and stops at healthy wiring. A system's only wire is the node
/// the surge came through, which has passed it on already, so a surge reaching a system ends there. It stops after
/// `max_steps` or below `min_points`.
fn cascade(
    l: &ShipLayout,
    s: &mut DamageState,
    cfg: &DamageData,
    src: usize,
    points: f64,
    taken: &mut [f64],
    bursts: &mut Vec<(DVec3, f64)>,
) {
    let k = &cfg.cascade;
    let nominal = cfg.systems.nominal_from_pct;
    let mut load_seen = vec![false; l.loads.len()];
    let mut node_seen = vec![false; l.nodes.len()];
    load_seen[src] = true;
    let mut queue = VecDeque::from([(Part::Node(l.loads[src].node), k.surge_share * points, 1u32)]);
    while let Some((part, share, step)) = queue.pop_front() {
        if share < k.min_points || step > k.max_steps {
            continue;
        }
        match part {
            Part::Node(n) => {
                if node_seen[n] {
                    continue;
                }
                node_seen[n] = true;
                let before = s.node_health[n];
                damage_node(s, cfg, n, share);
                bursts.push((l.nodes[n].centre, share));
                if step > 1 && before * 100.0 >= nominal {
                    continue;
                }
                let next = share * k.step_factor;
                for &li in &l.nodes[n].loads {
                    if !load_seen[li] {
                        queue.push_back((Part::Load(li), next, step + 1));
                    }
                }
                for &ci in &l.nodes[n].conduits {
                    let far = if l.conduits[ci].ends[0] == n { l.conduits[ci].ends[1] } else { l.conduits[ci].ends[0] };
                    if !s.conduits[ci].severed && !node_seen[far] {
                        queue.push_back((Part::Node(far), next, step + 1));
                    }
                }
            }
            Part::Load(li) => {
                if load_seen[li] {
                    continue;
                }
                load_seen[li] = true;
                s.integrity[li] = (s.integrity[li] - share).max(0.0);
                taken[li] += share;
                bursts.push((l.loads[li].centre, share));
            }
        }
    }
}

// ----------------------------------------------------------------------------------------------------- capability

/// What a load at `integrity` can do, 0-1 (`damage-control` 2): 1 at nominal or above, integrity / nominal between,
/// 0 when disabled.
pub fn load_capability(cfg: &DamageData, integrity: f64) -> f64 {
    let r = &cfg.systems;
    if integrity < r.disabled_below_pct {
        0.0
    } else if integrity < r.nominal_from_pct {
        integrity / r.nominal_from_pct
    } else {
        1.0
    }
}

/// Capability 0-1 of a system by its layout id (the drill reads "impulse_drive", "shield_generator", "laser_port",
/// "turret_dorsal" ...): the mean of its loads' (the reactor has two trains). A system with no load aboard is 1;
/// [`ShipLayout::has_system`] tells the two apart.
pub fn capability(l: &ShipLayout, s: &DamageState, cfg: &DamageData, system: &str) -> f64 {
    let (sum, n) = l
        .loads
        .iter()
        .enumerate()
        .filter(|(_, ld)| ld.system == system || ld.id == system)
        .fold((0.0, 0u32), |(sum, n), (i, _)| (sum + load_capability(cfg, s.integrity[i]), n + 1));
    if n == 0 {
        1.0
    } else {
        sum / f64::from(n)
    }
}

// ----------------------------------------------------------------------------------------------- fires and teams

/// What happened in a [`tick`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DamageEvent {
    /// A fire no team reached spread from one room to another.
    FireSpread {
        /// Where it was.
        from: usize,
        /// Where it went.
        to: usize,
    },
    /// A team put a fire out.
    FireOut {
        /// The room.
        room: usize,
    },
    /// A team set off for a job.
    Dispatched {
        /// Which team.
        team: usize,
        /// The job.
        job: Job,
    },
    /// A team reached its job.
    Arrived {
        /// Which team.
        team: usize,
        /// The job.
        job: Job,
    },
    /// A team brought a load back to nominal.
    Repaired {
        /// The load.
        load: usize,
    },
    /// The fires took this much off the ship's structural hull, MJ.
    HullBurned {
        /// MJ.
        mj: f64,
    },
}

/// Where a job is.
fn job_at(l: &ShipLayout, job: Job) -> DVec3 {
    match job {
        Job::Fire(room) => l.rooms[room].centre,
        Job::Repair(load) => l.loads[load].centre,
    }
}

/// The most urgent job no team has (`ship-damage` 5): a fire, in the order they started; then the captain's raised
/// system, its worst load first; then the load with the least integrity below nominal (the first of equals).
fn next_job(l: &ShipLayout, s: &DamageState, cfg: &DamageData, priority: Option<&str>, taken: &[Job]) -> Option<Job> {
    if let Some(f) = s.fires.iter().find(|f| !taken.contains(&Job::Fire(f.room))) {
        return Some(Job::Fire(f.room));
    }
    let nominal = cfg.systems.nominal_from_pct;
    let worst = |want: &dyn Fn(&Load) -> bool| {
        (0..l.loads.len())
            .filter(|&i| want(&l.loads[i]) && s.integrity[i] < nominal && !taken.contains(&Job::Repair(i)))
            .min_by(|&a, &b| s.integrity[a].total_cmp(&s.integrity[b]).then(a.cmp(&b)))
    };
    priority
        .and_then(|p| worst(&|ld: &Load| ld.system == p || ld.id == p))
        .or_else(|| worst(&|_: &Load| true))
        .map(Job::Repair)
}

/// Fires burn, spread and go out; teams travel and work (`ship-damage` 5). `priority` is the captain's raised
/// system, if any. Returns what happened, in order.
pub fn tick(
    l: &ShipLayout,
    s: &mut DamageState,
    cfg: &DamageData,
    dt: f64,
    priority: Option<&str>,
) -> Vec<DamageEvent> {
    let mut ev = Vec::new();
    if !(dt.is_finite() && dt > 0.0) {
        return ev;
    }
    let f = &cfg.fire;

    // Fires burn every system in their room and the structural hull.
    let mut hull = 0.0;
    for fire in &mut s.fires {
        fire.burning_s += dt;
        if !s.teams.iter().any(|t| t.working && t.job == Some(Job::Fire(fire.room))) {
            fire.unreached_s += dt;
        }
        for &li in &l.rooms[fire.room].loads {
            s.integrity[li] = (s.integrity[li] - f.burn_pct_per_s * dt).max(0.0);
        }
        hull += f.burn_hull_mj_per_s * dt;
    }
    // A fire no team has reached spreads to its first neighbour not already burning.
    for i in 0..s.fires.len() {
        if s.fires[i].unreached_s < f.spread_after_s {
            continue;
        }
        s.fires[i].unreached_s = 0.0;
        let from = s.fires[i].room;
        if let Some(&to) = l.rooms[from].neighbours.iter().find(|&&r| !s.burning(r)) {
            s.fires.push(Fire::new(to));
            ev.push(DamageEvent::FireSpread { from, to });
        }
    }

    // Teams on the way arrive; teams at work work.
    let nominal = cfg.systems.nominal_from_pct;
    let rate = cfg.repair_pct_per_s();
    for (ti, t) in s.teams.iter_mut().enumerate() {
        let Some(job) = t.job else { continue };
        if !t.working {
            t.eta_s -= dt;
            if t.eta_s <= 0.0 {
                t.working = true;
                t.at = job_at(l, job);
                ev.push(DamageEvent::Arrived { team: ti, job });
            }
            continue;
        }
        let done = match job {
            Job::Fire(room) => {
                if s.fires.iter().any(|x| x.room == room) {
                    t.work_s += dt;
                    let out = t.work_s >= f.put_out_s;
                    if out {
                        s.fires.retain(|x| x.room != room);
                        ev.push(DamageEvent::FireOut { room });
                    }
                    out
                } else {
                    true
                }
            }
            Job::Repair(li) => {
                if s.integrity[li] < nominal {
                    s.integrity[li] = (s.integrity[li] + rate * dt).min(nominal);
                }
                let back = s.integrity[li] >= nominal;
                if back {
                    ev.push(DamageEvent::Repaired { load: li });
                }
                back
            }
        };
        if done {
            *t = Team { at: t.at, job: None, eta_s: 0.0, working: false, work_s: 0.0 };
        }
    }

    // Idle teams take the most urgent job left, in team order.
    for ti in 0..s.teams.len() {
        if s.teams[ti].job.is_some() {
            continue;
        }
        let taken: Vec<Job> = s.teams.iter().filter_map(|t| t.job).collect();
        if let Some(job) = next_job(l, s, cfg, priority, &taken) {
            let t = &mut s.teams[ti];
            let walk = t.at.distance(job_at(l, job)) * cfg.teams.path_factor / cfg.teams.walk_m_s;
            *t = Team {
                at: t.at,
                job: Some(job),
                eta_s: cfg.teams.dispatch_delay_s + walk,
                working: false,
                work_s: 0.0,
            };
            ev.push(DamageEvent::Dispatched { team: ti, job });
        }
    }
    if hull > 0.0 {
        ev.push(DamageEvent::HullBurned { mj: hull });
    }
    ev
}

// -------------------------------------------------------------------------------------------------------- geometry

/// Whether `(x, z)` is inside the convex footprint `poly` (positive signed area), its edges included.
fn inside_convex(poly: &[[f64; 2]], x: f64, z: f64) -> bool {
    let n = poly.len();
    (0..n).all(|i| {
        let (a, b) = (poly[i], poly[(i + 1) % n]);
        (b[0] - a[0]) * (z - a[1]) - (b[1] - a[1]) * (x - a[0]) >= -EPS
    })
}

/// The signed area of a footprint (positive when its corners run the layout's way).
fn signed_area(poly: &[[f64; 2]]) -> f64 {
    let n = poly.len();
    0.5 * (0..n).map(|i| poly[i][0] * poly[(i + 1) % n][1] - poly[(i + 1) % n][0] * poly[i][1]).sum::<f64>()
}

/// The point of segment `ab` nearest `p`.
fn closest_on_seg(p: DVec3, a: DVec3, b: DVec3) -> DVec3 {
    let d = b - a;
    let l2 = d.length_squared();
    let t = if l2 > 1e-12 { ((p - a).dot(d) / l2).clamp(0.0, 1.0) } else { 0.0 };
    a + d * t
}

/// The least distance between segments `p1q1` and `p2q2` (Ericson, Real-Time Collision Detection 5.1.9; the mockup's
/// `segSegDist`).
fn seg_seg_dist(p1: DVec3, q1: DVec3, p2: DVec3, q2: DVec3) -> f64 {
    let (d1, d2, r) = (q1 - p1, q2 - p2, p1 - p2);
    let (a, e, f) = (d1.dot(d1), d2.dot(d2), d2.dot(r));
    let (s, t) = if a <= 1e-12 && e <= 1e-12 {
        (0.0, 0.0)
    } else if a <= 1e-12 {
        (0.0, (f / e).clamp(0.0, 1.0))
    } else {
        let c = d1.dot(r);
        if e <= 1e-12 {
            ((-c / a).clamp(0.0, 1.0), 0.0)
        } else {
            let b = d1.dot(d2);
            let den = a * e - b * b;
            let s = if den != 0.0 { ((b * f - c * e) / den).clamp(0.0, 1.0) } else { 0.0 };
            let t = (b * s + f) / e;
            if t < 0.0 {
                ((-c / a).clamp(0.0, 1.0), 0.0)
            } else if t > 1.0 {
                (((b - c) / a).clamp(0.0, 1.0), 1.0)
            } else {
                (s, t)
            }
        }
    };
    (p1 + d1 * s).distance(p2 + d2 * t)
}

/// The part `[t0, t1]` of `p + d t`, `t` in 0-1, inside the box `lo`-`hi`, if any (the slab test).
fn clip_box(p: DVec3, d: DVec3, lo: DVec3, hi: DVec3) -> Option<(f64, f64)> {
    let (mut t0, mut t1) = (0.0f64, 1.0f64);
    for i in 0..3 {
        if d[i].abs() < 1e-12 {
            if p[i] < lo[i] || p[i] > hi[i] {
                return None;
            }
        } else {
            let (a, b) = ((lo[i] - p[i]) / d[i], (hi[i] - p[i]) / d[i]);
            t0 = t0.max(a.min(b));
            t1 = t1.min(a.max(b));
        }
    }
    (t0 <= t1).then_some((t0, t1))
}

// ---------------------------------------------------------------------------------------------- the files' schemas

/// A load as both ships' files give it.
struct LoadIn {
    id: String,
    system: String,
    node: String,
    centre: [f64; 3],
}

/// A conduit as both ships' files give it.
struct ConduitIn {
    id: String,
    between: [String; 2],
    route: Vec<String>,
    path: Vec<[f64; 3]>,
}

/// A ship's insides as read, before ids become indices.
struct Parts {
    ship: String,
    /// Where the layout, the power and the teams came from, for messages.
    files: [&'static str; 3],
    sections: Vec<HullSection>,
    rooms: Vec<(String, Vec<Brush>)>,
    portals: Vec<[String; 2]>,
    nodes: Vec<(String, Option<String>, [f64; 3])>,
    loads: Vec<LoadIn>,
    conduits: Vec<ConduitIn>,
    teams: u32,
    home: String,
}

fn err(file: &str, field: &str, message: String) -> DataError {
    DataError { file: file.to_owned(), field: field.to_owned(), message }
}

fn names(id: &str) -> String {
    format!("names {id:?}, which no file defines")
}

fn finite3(file: &str, field: &str, v: [f64; 3]) -> Result<DVec3, DataError> {
    let p = DVec3::from_array(v);
    if p.is_finite() {
        Ok(p)
    } else {
        Err(err(file, field, format!("must be finite numbers, got {v:?}")))
    }
}

/// One builder for every ship: check the cross references and the geometry, then index everything.
fn build(p: Parts) -> Result<ShipLayout, DataError> {
    let [lf, pf, tf] = p.files;
    let mut sections = p.sections;
    sections.sort_by(|a, b| a.z_m.total_cmp(&b.z_m));
    if sections.len() < 2 {
        return Err(err(lf, "hull.sections", "needs at least two sections".into()));
    }
    for (i, s) in sections.iter().enumerate() {
        let ok = [s.z_m, s.half_beam_m, s.top_m, s.bottom_m, s.chamfer_m].iter().all(|v| v.is_finite())
            && s.half_beam_m > 0.0
            && s.top_m > s.bottom_m
            && s.chamfer_m >= 0.0;
        if !ok || (i > 0 && s.z_m <= sections[i - 1].z_m) {
            return Err(err(
                lf,
                &format!("hull.sections.{i}"),
                "must be finite, with width, height and a z of its own".into(),
            ));
        }
    }
    let lo = DVec3::new(
        -sections.iter().map(|s| s.half_beam_m).fold(0.0, f64::max),
        sections.iter().map(|s| s.bottom_m).fold(f64::INFINITY, f64::min),
        sections[0].z_m,
    );
    let hi = DVec3::new(
        -lo.x,
        sections.iter().map(|s| s.top_m).fold(f64::NEG_INFINITY, f64::max),
        sections[sections.len() - 1].z_m,
    );

    let mut rooms = Vec::with_capacity(p.rooms.len());
    for (ri, (id, brushes)) in p.rooms.into_iter().enumerate() {
        if brushes.is_empty() {
            return Err(err(lf, &format!("rooms.{ri}.brushes"), "a room needs a brush".into()));
        }
        let (mut blo, mut bhi) = (DVec3::splat(f64::INFINITY), DVec3::splat(f64::NEG_INFINITY));
        let mut best: Option<(f64, &Brush)> = None;
        for (bi, b) in brushes.iter().enumerate() {
            let a = signed_area(&b.poly);
            let ok = b.poly.len() >= 3
                && b.poly.iter().flatten().chain(b.y.iter()).all(|v| v.is_finite())
                && b.y[1] > b.y[0]
                && a > 0.0;
            if !ok {
                return Err(err(
                    lf,
                    &format!("{id}.brushes.{bi}"),
                    "must be finite, with height and positive area".into(),
                ));
            }
            for q in &b.poly {
                blo = blo.min(DVec3::new(q[0], b.y[0], q[1]));
                bhi = bhi.max(DVec3::new(q[0], b.y[1], q[1]));
            }
            if best.is_none_or(|(ba, _)| a > ba) {
                best = Some((a, b));
            }
        }
        let (area, b) = best.ok_or_else(|| err(lf, &id, "a room needs a brush".into()))?;
        let n = b.poly.len();
        let (mut cx, mut cz) = (0.0, 0.0);
        for i in 0..n {
            let (u, v) = (b.poly[i], b.poly[(i + 1) % n]);
            let w = u[0] * v[1] - v[0] * u[1];
            cx += (u[0] + v[0]) * w;
            cz += (u[1] + v[1]) * w;
        }
        let centre = DVec3::new(cx / (6.0 * area), 0.5 * (b.y[0] + b.y[1]), cz / (6.0 * area));
        rooms.push(Room { id, brushes, lo: blo, hi: bhi, centre, neighbours: Vec::new(), loads: Vec::new() });
    }
    let room_of = |file: &str, field: &str, id: &str, rooms: &[Room]| {
        rooms.iter().position(|r| r.id == id).ok_or_else(|| err(file, field, names(id)))
    };
    for (i, [a, b]) in p.portals.iter().enumerate() {
        let (ra, rb) =
            (room_of(lf, &format!("portals.{i}"), a, &rooms)?, room_of(lf, &format!("portals.{i}"), b, &rooms)?);
        for (x, y) in [(ra, rb), (rb, ra)] {
            if x != y && !rooms[x].neighbours.contains(&y) {
                rooms[x].neighbours.push(y);
            }
        }
    }

    let mut nodes = Vec::with_capacity(p.nodes.len());
    for (i, (id, room, c)) in p.nodes.into_iter().enumerate() {
        let field = format!("nodes.{i}");
        let room = room.map(|r| room_of(pf, &format!("{field}.room"), &r, &rooms)).transpose()?;
        let centre = finite3(pf, &format!("{field}.center_m"), c)?;
        nodes.push(Node { id, room, centre, loads: Vec::new(), conduits: Vec::new() });
    }
    let node_of = |field: &str, id: &str, nodes: &[Node]| {
        nodes.iter().position(|n| n.id == id).ok_or_else(|| err(pf, field, names(id)))
    };
    let mut loads = Vec::with_capacity(p.loads.len());
    for (i, l) in p.loads.into_iter().enumerate() {
        let node = node_of(&format!("loads.{i}.node"), &l.node, &nodes)?;
        let centre = finite3(pf, &format!("loads.{i}.center_m"), l.centre)?;
        let room = rooms.iter().position(|r| r.contains(centre));
        nodes[node].loads.push(i);
        if let Some(r) = room {
            rooms[r].loads.push(i);
        }
        loads.push(Load { id: l.id, system: l.system, node, centre, room });
    }
    let mut conduits = Vec::with_capacity(p.conduits.len());
    for (i, ConduitIn { id, between: [a, b], route, path }) in p.conduits.into_iter().enumerate() {
        let field = format!("conduits.{i}");
        let ends =
            [node_of(&format!("{field}.between"), &a, &nodes)?, node_of(&format!("{field}.between"), &b, &nodes)?];
        let route =
            route.iter().map(|r| room_of(pf, &format!("{field}.route"), r, &rooms)).collect::<Result<Vec<_>, _>>()?;
        let path = path
            .iter()
            .enumerate()
            .map(|(j, q)| finite3(pf, &format!("{field}.path_m.{j}"), *q))
            .collect::<Result<Vec<_>, _>>()?;
        if path.len() < 2 {
            return Err(err(pf, &format!("{field}.path_m"), "a conduit's run needs two points".into()));
        }
        for e in ends {
            nodes[e].conduits.push(i);
        }
        conduits.push(Conduit { id, ends, route, path });
    }
    let home = rooms[room_of(tf, "teams.home", &p.home, &rooms)?].centre;
    Ok(ShipLayout { ship: p.ship, sections, rooms, loads, nodes, conduits, teams: p.teams as usize, home, lo, hi })
}

/// `layout.json`: the parts this module reads are typed; the rest belongs to other readers.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)] // typed so that an unknown or misspelt key is refused; not every field is read here
struct TernLayoutFile {
    schema: String,
    ship: Value,
    conventions: Value,
    decks: Value,
    hull: HullFile,
    compartments: Vec<CompartmentFile>,
    portals: Vec<PortalFile>,
    stations: Value,
    fixtures: Value,
    systems: Vec<SystemFile>,
    mounts: Vec<MountFile>,
    craft: Value,
}

impl Validate for TernLayoutFile {
    fn validate(&self, c: &mut Checks) {
        c.equals("schema", &self.schema, "starcrew.ship-layout/2");
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)] // typed so that an unknown or misspelt key is refused; not every field is read here
struct HullFile {
    note: Option<String>,
    clearance_m: Option<f64>,
    sections: Vec<HullSection>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)] // typed so that an unknown or misspelt key is refused; not every field is read here
struct CompartmentFile {
    id: String,
    poi: u32,
    name: String,
    decks: Vec<String>,
    kind: String,
    finish: String,
    purpose: String,
    brushes: Vec<Brush>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)] // typed so that an unknown or misspelt key is refused; not every field is read here
struct PortalFile {
    id: String,
    kind: String,
    between: [String; 2],
    center_m: [f64; 3],
    normal: [f64; 3],
    size_m: [f64; 2],
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)] // typed so that an unknown or misspelt key is refused; not every field is read here
struct SystemFile {
    id: String,
    name: String,
    kind: String,
    compartment: String,
    center_m: [f64; 3],
    radius_m: Option<f64>,
    height_m: Option<f64>,
    units_m: Option<Vec<[f64; 3]>>,
    units_note: Option<String>,
    mount: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)] // typed so that an unknown or misspelt key is refused; not every field is read here
struct MountFile {
    id: String,
    kind: String,
    center_m: [f64; 3],
    facing: [f64; 3],
    weapon: Option<String>,
    crew_seat: Option<String>,
}

/// `power.json`: its nodes, conduits and loads are typed; the rest is `power-grid`'s.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)] // typed so that an unknown or misspelt key is refused; not every field is read here
struct PowerFile {
    schema: String,
    status: String,
    ship: String,
    units: Value,
    solve: Value,
    reactor: Value,
    battery: Value,
    nodes: Vec<PowerNodeFile>,
    generators: Value,
    ties: Value,
    conduits: Vec<PowerConduitFile>,
    loads: Vec<PowerLoadFile>,
    groups: Value,
    presets: Value,
    lighting: Value,
    coolant: Value,
    radiators: Value,
    overdrive: Value,
}

impl Validate for PowerFile {
    fn validate(&self, c: &mut Checks) {
        c.equals("schema", &self.schema, "starcrew.ship-power/1");
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)] // typed so that an unknown or misspelt key is refused; not every field is read here
struct PowerNodeFile {
    id: String,
    name: String,
    kind: String,
    compartment: String,
    center_m: [f64; 3],
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)] // typed so that an unknown or misspelt key is refused; not every field is read here
struct PowerConduitFile {
    id: String,
    name: String,
    between: [String; 2],
    capacity_mw: f64,
    route: Vec<String>,
    path_m: Vec<[f64; 3]>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)] // typed so that an unknown or misspelt key is refused; not every field is read here
struct PowerLoadFile {
    id: String,
    name: String,
    system: Option<String>,
    mount: Option<String>,
    node: String,
    priority: u32,
    nominal_mw: f64,
    standby_mw: f64,
    setpoint_max: f64,
    min_ratio: f64,
    power_exponent: Option<f64>,
    activity: Option<String>,
    center_m: Option<[f64; 3]>,
    heat: Value,
    under: String,
    over: String,
}

/// `data/ships/hound/layout.json` (`starcrew.combat-layout/1`): a ship's insides for combat damage, for a ship with
/// no deck plan.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)] // typed so that an unknown or misspelt key is refused; not every field is read here
struct CombatLayoutFile {
    schema: String,
    ship: String,
    hull: CombatHullFile,
    rooms: Vec<CombatRoomFile>,
    portals: Vec<CombatPortalFile>,
    nodes: Vec<CombatNodeFile>,
    loads: Vec<CombatLoadFile>,
    conduits: Vec<CombatConduitFile>,
    teams: CombatTeamsFile,
}

impl Validate for CombatLayoutFile {
    fn validate(&self, c: &mut Checks) {
        c.equals("schema", &self.schema, "starcrew.combat-layout/1");
        c.count("teams.count", i64::from(self.teams.count), 0, 16);
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)] // typed so that an unknown or misspelt key is refused; not every field is read here
struct CombatHullFile {
    sections: Vec<HullSection>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)] // typed so that an unknown or misspelt key is refused; not every field is read here
struct CombatRoomFile {
    id: String,
    name: String,
    brushes: Vec<Brush>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)] // typed so that an unknown or misspelt key is refused; not every field is read here
struct CombatPortalFile {
    id: String,
    between: [String; 2],
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)] // typed so that an unknown or misspelt key is refused; not every field is read here
struct CombatNodeFile {
    id: String,
    room: String,
    center_m: [f64; 3],
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)] // typed so that an unknown or misspelt key is refused; not every field is read here
struct CombatLoadFile {
    id: String,
    system: Option<String>,
    node: String,
    center_m: [f64; 3],
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)] // typed so that an unknown or misspelt key is refused; not every field is read here
struct CombatConduitFile {
    id: String,
    between: [String; 2],
    route: Vec<String>,
    path_m: Vec<[f64; 3]>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)] // typed so that an unknown or misspelt key is refused; not every field is read here
struct CombatTeamsFile {
    count: u32,
    home: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    const TERN: [&str; 3] = [
        include_str!("../../../../data/ships/tern/layout.json"),
        include_str!("../../../../data/ships/tern/power.json"),
        include_str!("../../../../data/ships/tern/damage.json"),
    ];

    fn tern() -> (ShipLayout, DamageData) {
        (ShipLayout::tern(TERN).expect("the Tern's files load"), DamageData::parse(TERN[2]).expect("the rules load"))
    }

    fn hound() -> ShipLayout {
        ShipLayout::hound(include_str!("../../../../data/ships/hound/layout.json")).expect("the Hound's file loads")
    }

    fn load(l: &ShipLayout, id: &str) -> usize {
        l.load(id).unwrap_or_else(|| panic!("the ship has a load {id}"))
    }

    /// Bare the armour section a shot along `dir` meets at `entry`, so everything passes into the ship.
    fn bare(l: &ShipLayout, s: &mut DamageState, entry: DVec3, dir: DVec3) {
        let (span, face) = section_of(l, entry, dir);
        s.armour[span * FACES.len() + face] = 0.0;
    }

    #[test]
    fn the_terns_armour_is_nine_spans_by_six_faces_and_the_hounds_four() {
        let (l, _) = tern();
        let s = DamageState::new(&l);
        assert_eq!(s.armour.len(), 54);
        assert!(s.armour.iter().all(|&a| a == 100.0), "a ship starts with every section whole");
        assert_eq!(DamageState::new(&hound()).armour.len(), 24);
    }

    #[test]
    fn every_room_but_the_pods_is_inside_the_hull() {
        for l in [tern().0, hound()] {
            for r in l.rooms.iter().filter(|r| !r.id.starts_with("pod_")) {
                for b in &r.brushes {
                    for q in &b.poly {
                        for y in b.y {
                            let p = DVec3::new(q[0], y, q[1]);
                            assert!(l.inside_hull(p), "{}'s {} has a corner {p} outside the hull", l.ship, r.id);
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn a_shot_from_dead_ahead_enters_at_the_bow() {
        let (l, _) = tern();
        let e = hull_entry(&l, DVec3::new(0.0, 1.0, 200.0), DVec3::new(0.0, 1.0, 0.0)).expect("it hits");
        assert!((e.z - 42.0).abs() < 1e-3, "the bow's tip is at z 42 m, entered at {e}");
        assert!(l.inside_hull(e));
    }

    #[test]
    fn a_beam_from_eight_kilometres_finds_the_same_point() {
        let (l, _) = tern();
        let near = hull_entry(&l, DVec3::new(0.0, 1.0, 200.0), DVec3::ZERO.with_y(1.0)).unwrap();
        let far = hull_entry(&l, DVec3::new(0.0, 1.0, 8000.0), DVec3::ZERO.with_y(1.0)).unwrap();
        assert!(near.distance(far) < 1e-3, "the path is cut to the hull's bounds before it is stepped");
    }

    #[test]
    fn a_shot_that_passes_over_the_ship_has_no_entry() {
        let (l, _) = tern();
        assert_eq!(hull_entry(&l, DVec3::new(0.0, 20.0, 100.0), DVec3::new(0.0, 20.0, -100.0)), None);
    }

    #[test]
    fn the_face_is_the_side_the_shot_comes_from() {
        let (l, _) = tern();
        let p = DVec3::new(0.0, 0.0, 10.0);
        assert_eq!(section_of(&l, p, DVec3::NEG_Y).1, 0, "a shot going down hits the dorsal face");
        assert_eq!(section_of(&l, p, DVec3::NEG_X).1, 2, "a shot going to starboard (-X) hits the port face");
        assert_eq!(section_of(&l, p, DVec3::NEG_Z).1, 4, "a shot going aft hits the bow face");
        assert_eq!(section_of(&l, DVec3::new(0.0, 0.0, -41.0), DVec3::Z).0, 0, "the stern's span is the first");
        assert_eq!(section_of(&l, DVec3::new(0.0, 0.0, 41.0), DVec3::Z).0, 8, "the bow's span is the ninth");
    }

    #[test]
    fn fresh_armour_holds_a_laser_and_loses_nothing() {
        let (l, cfg) = tern();
        let mut s = DamageState::new(&l);
        let o = resolve_hit(&l, &mut s, &cfg, DVec3::new(0.0, 1.0, 42.0), DVec3::NEG_Z, 40.0, Weapon::Laser, 1, 7);
        assert_eq!((o.absorbed_mj, o.passed_mj), (40.0, 0.0));
        assert!(s.armour.iter().all(|&a| a == 100.0), "a laser does not strip armour");
        assert!(s.integrity.iter().all(|&i| i == 100.0));
    }

    #[test]
    fn a_missile_strips_armour_and_then_a_laser_goes_through() {
        let (l, cfg) = tern();
        let mut s = DamageState::new(&l);
        let (at, dir) = (DVec3::new(0.0, 1.0, 42.0), DVec3::NEG_Z);
        let k = 8 * FACES.len() + 4;
        let o = resolve_hit(&l, &mut s, &cfg, at, dir, 30.0, Weapon::Missile, 1, 7);
        assert_eq!((o.absorbed_mj, o.passed_mj), (4.0, 26.0), "fresh armour absorbs its 4 MJ");
        assert_eq!(s.armour[k], 25.0, "a 30 MJ blast strips 75 points");
        resolve_hit(&l, &mut s, &cfg, at, dir, 30.0, Weapon::Missile, 2, 7);
        assert_eq!(s.armour[k], 0.0);
        let o = resolve_hit(&l, &mut s, &cfg, at, dir, 10.0, Weapon::Laser, 3, 7);
        assert_eq!(o.passed_mj, 10.0, "with no armour left the beam goes in whole");
    }

    #[test]
    fn a_cannon_bolt_wears_armour_by_damage_controls_rule() {
        let (l, cfg) = tern();
        let mut s = DamageState::new(&l);
        let o = resolve_hit(&l, &mut s, &cfg, DVec3::new(0.0, 1.0, 42.0), DVec3::NEG_Z, 6.0, Weapon::Cannon, 1, 7);
        assert_eq!((o.absorbed_mj, o.passed_mj), (4.0, 2.0));
        assert_eq!(s.armour[8 * FACES.len() + 4], 90.0, "min(6, 4) / 40 x 100 = 10 points");
    }

    #[test]
    fn capability_is_one_at_nominal_proportional_between_and_nothing_when_disabled() {
        let (_, cfg) = tern();
        assert_eq!(load_capability(&cfg, 100.0), 1.0);
        assert_eq!(load_capability(&cfg, 75.0), 1.0);
        assert_eq!(load_capability(&cfg, 60.0), 0.8);
        assert_eq!(load_capability(&cfg, 24.9), 0.0);
    }

    #[test]
    fn a_beam_through_the_drive_room_breaks_the_impulse_drive() {
        // The spec's scenario: energy past the armour along a line through the impulse drive.
        let (l, cfg) = tern();
        let mut s = DamageState::new(&l);
        let entry = hull_entry(&l, DVec3::new(0.0, 1.5, -100.0), DVec3::new(0.0, 1.5, 0.0)).expect("it hits the stern");
        bare(&l, &mut s, entry, DVec3::Z);
        let o = resolve_hit(&l, &mut s, &cfg, entry, DVec3::Z, 40.0, Weapon::Laser, 1, 7);
        let drive = load(&l, "impulse_drive");
        assert!(o.systems.iter().any(|&(i, _)| i == drive), "the drive is reported");
        assert!(s.integrity[drive] < 75.0, "the drive is below nominal: {}", s.integrity[drive]);
        assert!(capability(&l, &s, &cfg, "impulse_drive") < 1.0, "and the ship's thrust falls with it");
        assert_eq!(
            s.breaches.first().map(|b| l.rooms[b.room].id.as_str()),
            Some("drive"),
            "the drive room is breached"
        );
    }

    #[test]
    fn a_surge_from_a_damaged_system_reaches_the_systems_on_its_panel() {
        // The spec's scenario: two damaged systems on one panel (the life support panel feeds both).
        let (l, cfg) = tern();
        let mut s = DamageState::new(&l);
        let (o2, co2, air) = (load(&l, "o2_generator"), load(&l, "co2_scrubbers"), load(&l, "air_handler"));
        s.integrity[o2] = 50.0;
        s.integrity[co2] = 50.0;
        let (mut taken, mut bursts) = (vec![0.0; l.loads.len()], Vec::new());
        cascade(&l, &mut s, &cfg, o2, 20.0, &mut taken, &mut bursts);
        let panel = l.node("dp_ls").unwrap();
        assert!((s.node_health[panel] - 0.88).abs() < 1e-9, "the panel takes 0.6 x 20 = 12 points");
        assert_eq!(s.integrity[co2], 44.0, "the other damaged system takes half of that");
        assert_eq!(s.integrity[air], 94.0, "a healthy system on the panel takes its share too");
        assert_eq!(s.integrity[o2], 50.0, "the hit system does not take its own surge");
        let (fsb, beyond) = (l.node("fsb_p").unwrap(), l.node("dp_a").unwrap());
        assert!((s.node_health[fsb] - 0.94).abs() < 1e-9, "the switchboard at the conduit's far end takes 6");
        assert_eq!(s.node_health[beyond], 1.0, "a healthy switchboard stops the surge");
        assert_eq!(bursts.len(), 1 + 3 + 2, "the panel, its three other systems and the two far ends burst");
    }

    #[test]
    fn a_damaged_switchboard_passes_the_surge_on() {
        let (l, cfg) = tern();
        let mut s = DamageState::new(&l);
        let o2 = load(&l, "o2_generator");
        s.integrity[o2] = 50.0;
        let fsb = l.node("fsb_p").unwrap();
        s.node_health[fsb] = 0.5;
        let (mut taken, mut bursts) = (vec![0.0; l.loads.len()], Vec::new());
        cascade(&l, &mut s, &cfg, o2, 20.0, &mut taken, &mut bursts);
        let dp_a = l.node("dp_a").unwrap();
        assert!((s.node_health[dp_a] - 0.97).abs() < 1e-9, "the panel past it takes 3 points (step 3)");
        let computer = load(&l, "computer");
        assert_eq!(s.integrity[computer], 100.0, "the panel past it was healthy, so its systems are spared");
        assert_eq!(s.node_health[fsb], 0.5 - 0.06, "the damaged switchboard took its 6 points");
    }

    #[test]
    fn the_surge_stops_after_four_steps_and_below_a_point() {
        let (l, cfg) = tern();
        let mut s = DamageState::new(&l);
        for h in &mut s.node_health {
            *h = 0.5;
        }
        let o2 = load(&l, "o2_generator");
        s.integrity[o2] = 50.0;
        let (mut taken, mut bursts) = (vec![0.0; l.loads.len()], Vec::new());
        cascade(&l, &mut s, &cfg, o2, 20.0, &mut taken, &mut bursts);
        let least = bursts.iter().map(|b| b.1).fold(f64::INFINITY, f64::min);
        assert!(least >= cfg.cascade.min_points, "nothing below a point: {least}");
        assert!(least >= 12.0 / 8.0 - 1e-9, "four steps from 12 points end at 1.5");
    }

    #[test]
    fn a_hit_on_a_damaged_system_cascades_to_its_neighbours() {
        let (l, cfg) = tern();
        let mut s = DamageState::new(&l);
        let (o2, co2) = (load(&l, "o2_generator"), load(&l, "co2_scrubbers"));
        s.integrity[o2] = 50.0;
        s.integrity[co2] = 50.0;
        // A beam from port along y -3, z 17, through the life support room and the oxygen generator.
        let entry = hull_entry(&l, DVec3::new(40.0, -3.0, 17.0), DVec3::new(0.0, -3.0, 17.0)).expect("it hits");
        bare(&l, &mut s, entry, DVec3::NEG_X);
        let o = resolve_hit(&l, &mut s, &cfg, entry, DVec3::NEG_X, 20.0, Weapon::Laser, 1, 7);
        assert!(s.integrity[o2] < 50.0, "the beam hits the oxygen generator");
        assert!(s.integrity[co2] < 50.0, "6 m off the beam, the scrubbers lose integrity through the panel");
        assert!(o.systems.iter().any(|&(i, _)| i == co2), "and are reported");
        assert!(!o.bursts.is_empty(), "the cascade bursts");
    }

    #[test]
    fn the_same_hit_in_the_same_session_does_the_same() {
        let (l, cfg) = tern();
        let entry = hull_entry(&l, DVec3::new(0.0, 1.5, -100.0), DVec3::new(0.0, 1.5, 0.0)).unwrap();
        let run = |hit: u64| {
            let mut s = DamageState::new(&l);
            bare(&l, &mut s, entry, DVec3::Z);
            let o = resolve_hit(&l, &mut s, &cfg, entry, DVec3::Z, 60.0, Weapon::Missile, hit, 7);
            (o, s)
        };
        assert_eq!(run(5), run(5), "fires are rolled from the session, the hit and the room, nothing else");
        let fires = (0..40).map(|h| run(h).0.fires.len()).sum::<usize>();
        assert!(fires > 0, "60 MJ through the drive starts fires on some hits");
    }

    #[test]
    fn a_fire_burns_its_room_until_a_team_puts_it_out() {
        let (l, cfg) = tern();
        let mut s = DamageState::new(&l);
        let room = l.room("drive").unwrap();
        s.fires.push(Fire::new(room));
        let drive = load(&l, "impulse_drive");
        let (mut out_at, mut t, mut burned) = (None, 0.0, 0.0);
        while t < 300.0 && out_at.is_none() {
            for e in tick(&l, &mut s, &cfg, 0.1, None) {
                match e {
                    DamageEvent::FireOut { room: r } if r == room => out_at = Some(t),
                    DamageEvent::HullBurned { mj } => burned += mj,
                    _ => {}
                }
            }
            t += 0.1;
        }
        let out_at = out_at.expect("a team puts it out");
        assert!(!s.burning(room));
        assert!(out_at < cfg.fire.spread_after_s, "reached in time, it did not spread ({out_at} s)");
        assert_eq!(s.fires.len(), 0);
        let lost = 100.0 - s.integrity[drive];
        assert!((lost - cfg.fire.burn_pct_per_s * out_at).abs() < 1.0, "the drive burned 0.4 a second: lost {lost}");
        assert!((burned - cfg.fire.burn_hull_mj_per_s * out_at).abs() < 0.1, "and the hull 0.05 MJ a second");
    }

    #[test]
    fn an_unreached_fire_spreads_after_ninety_seconds() {
        let (l, cfg) = tern();
        let mut s = DamageState::new(&l);
        s.teams.clear();
        let room = l.room("drive").unwrap();
        s.fires.push(Fire::new(room));
        let mut spread = Vec::new();
        for i in 0..1000 {
            for e in tick(&l, &mut s, &cfg, 0.1, None) {
                if let DamageEvent::FireSpread { from, to } = e {
                    spread.push((i, from, to));
                }
            }
        }
        let (i, from, to) = spread[0];
        assert!((899..=901).contains(&i), "it spread at 90 s, step {i}");
        assert_eq!(from, room);
        assert!(l.rooms[room].neighbours.contains(&to), "into a room a portal joins it to");
    }

    #[test]
    fn a_team_brings_a_disabled_system_back_to_nominal() {
        // The spec's scenario.
        let (l, cfg) = tern();
        let mut s = DamageState::new(&l);
        let sensors = load(&l, "sensors");
        s.integrity[sensors] = 10.0;
        assert_eq!(capability(&l, &s, &cfg, "sensor_array"), 0.0, "disabled");
        let mut repaired = None;
        for i in 0..2000 {
            if tick(&l, &mut s, &cfg, 0.1, None).contains(&DamageEvent::Repaired { load: sensors }) {
                repaired = Some(i);
                break;
            }
        }
        assert!(repaired.is_some(), "a team reached it and brought it back");
        assert_eq!(s.integrity[sensors], cfg.systems.nominal_from_pct, "back at nominal, no further");
        assert_eq!(capability(&l, &s, &cfg, "sensor_array"), 1.0);
        assert!(s.teams.iter().all(|t| t.job.is_none()), "with nothing else to do the teams wait where they are");
    }

    #[test]
    fn the_captains_raised_system_is_repaired_first() {
        let (l, cfg) = tern();
        let first = |priority: Option<&str>| {
            let mut s = DamageState::new(&l);
            s.integrity[load(&l, "o2_generator")] = 60.0;
            s.integrity[load(&l, "computer")] = 30.0;
            tick(&l, &mut s, &cfg, 0.1, priority);
            s.teams[0].job
        };
        assert_eq!(first(None), Some(Job::Repair(load(&l, "computer"))), "the worst system first");
        assert_eq!(first(Some("o2_generator")), Some(Job::Repair(load(&l, "o2_generator"))), "the captain's first");
    }

    #[test]
    fn a_fire_comes_before_any_repair() {
        let (l, cfg) = tern();
        let mut s = DamageState::new(&l);
        s.integrity[load(&l, "computer")] = 30.0;
        s.fires.push(Fire::new(l.room("mess").unwrap()));
        tick(&l, &mut s, &cfg, 0.1, Some("computer"));
        assert_eq!(s.teams[0].job, Some(Job::Fire(l.room("mess").unwrap())));
        assert_eq!(s.teams[1].job, Some(Job::Repair(load(&l, "computer"))), "the second team takes the next job");
    }

    #[test]
    fn the_tern_has_everything_the_drill_reads() {
        let (l, _) = tern();
        for id in [
            "impulse_drive",
            "rcs_aft",
            "rcs_fwd",
            "shield_generator",
            "sensor_array",
            "tube_1",
            "tube_2",
            "laser_port",
            "laser_stbd",
            "turret_dorsal",
            "turret_ventral",
            "turret_port",
            "turret_stbd",
            "reactor",
            "computer",
            "magazine_racks",
        ] {
            assert!(l.has_system(id), "the Tern can break its {id}");
        }
        assert!(!l.has_system("warp_core"), "and nothing it does not have");
        let banks: Vec<usize> = ["laser_port", "laser_stbd"].iter().map(|b| load(&l, b)).collect();
        assert!(banks.iter().all(|&b| l.loads[b].room.is_some()), "the banks are in the broadside rooms");
    }

    #[test]
    fn the_hound_has_its_rooms_loads_wiring_and_a_drive_that_breaks() {
        let l = hound();
        assert_eq!((l.rooms.len(), l.loads.len(), l.nodes.len(), l.conduits.len()), (7, 11, 2, 4));
        for id in ["impulse_drive", "reactor", "shield_generator", "sensor_array", "laser_port", "laser_stbd"] {
            assert!(l.has_system(id), "the Hound can lose its {id}");
        }
        let (_, cfg) = tern();
        let mut s = DamageState::new(&l);
        let entry = hull_entry(&l, DVec3::new(0.0, 0.0, -100.0), DVec3::ZERO).expect("it hits the stern");
        assert!((entry.z + 15.0).abs() < 1e-3, "the Hound's stern is at z -15 m");
        bare(&l, &mut s, entry, DVec3::Z);
        resolve_hit(&l, &mut s, &cfg, entry, DVec3::Z, 30.0, Weapon::Laser, 1, 7);
        assert!(capability(&l, &s, &cfg, "impulse_drive") < 1.0, "a beam up its stern breaks its drive");
    }

    #[test]
    fn a_misspelt_rule_is_refused_with_its_field() {
        let text = TERN[2].replace("\"burn_pct_per_s\"", "\"burn_pct_per_sec\"");
        let e = DamageData::parse(&text).expect_err("an unknown key is an error");
        assert!(e.field.starts_with("fire"), "the error names the block: {e}");
    }
}
