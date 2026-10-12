//! A ship's power and its cooling (openspec/changes/power-grid, reactor-cooling): the reactor, the battery, the buses,
//! ties and conduits between the switchboards, every load's demand, the one flow solve that gives each load its power,
//! and where the heat goes: the loads' thermal nodes, the reactor's blanket, the coolant loop and the radiators.
//!
//! It lives in the core because what each system gets of its power is a gameplay rule the server, the consoles'
//! previews and the tests must agree on (CLAUDE.md 6.1, 6.2): a console's ghost is [`PowerState::preview_group`], the
//! same solve the step runs. It is a port of the mockups' library (`docs/mockups/lib/shipsystems.js`, `gridSnapshot`,
//! `solveGrid`, `stepPower`, `stepCooling`, `stepHeat`, `checkScram`), which the approved Engineering console was
//! designed against. The air is not simulated in the drill yet, so heat a load puts into a room is counted but warms
//! nothing, and the life-support plant's loads draw by their activity like any other.

use crate::combat::damage::{ConduitState, CoolantLeakRules, DamageData, SystemRules};
use crate::data::{parse, Checks, DataError, Validate};
use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeMap;

/// `data/ships/<id>/power.json` (`starcrew.ship-power/1`).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PowerFile {
    pub schema: String,
    pub status: String,
    pub ship: String,
    pub units: BTreeMap<String, String>,
    pub solve: SolveRules,
    pub reactor: ReactorRules,
    pub battery: BatteryRules,
    pub nodes: Vec<PowerNode>,
    pub generators: Vec<Generator>,
    pub ties: Vec<Tie>,
    pub conduits: Vec<PowerConduit>,
    pub loads: Vec<PowerLoad>,
    pub groups: Vec<Group>,
    /// Each preset's setpoint by group id, and a `note`.
    pub presets: BTreeMap<String, Value>,
    pub lighting: Lighting,
    pub coolant: Coolant,
    pub radiators: Radiators,
    pub overdrive: Overdrive,
    /// The conduit boxes' generator's rules (`tools/power/boxes.py`; power-routing 1).
    pub distribution: Distribution,
    /// The main protection's limit on live runs and the trip (power-routing 2).
    pub routing: Routing,
}

/// The rules `tools/power/boxes.py` places the conduit boxes, runs and feeders by. The engine reads the result, not
/// these; they are typed so that a misspelt key stops startup.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Distribution {
    pub box_height_m: f64,
    pub box_beside_door_m: f64,
    pub box_into_room_m: f64,
    pub run_below_ceiling_m: f64,
    pub ratings_mw: Vec<f64>,
    pub rating_margin: f64,
    pub spare_mw: f64,
    /// Every box joined to at least this many others: runs through walls and decks make up the rest.
    pub min_runs_per_box: u32,
    /// Panels each system is wired to: its node (the nearest) and, with 2, its `alt_node` (the next nearest).
    pub feeds_per_system: u32,
    /// Two rooms touch where their walls run within this of each other for at least `min_shared_wall_m`, metres.
    pub adjacent_gap_m: f64,
    pub min_shared_wall_m: f64,
    pub note: String,
}

/// The main protection (power-routing 2): how many runs may be live at once, how long a trip keeps the grid dark,
/// and which node kinds stay on through it.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Routing {
    pub max_live_runs: u32,
    pub trip_dark_s: f64,
    pub kept_on_trip: Vec<String>,
    pub note: String,
}

impl Validate for PowerFile {
    fn validate(&self, c: &mut Checks) {
        c.equals("schema", &self.schema, "starcrew.ship-power/1");
        c.number("solve.substep_hz", self.solve.substep_hz, 1.0, 120.0);
        c.number("reactor.thermal_rated_mw", self.reactor.thermal_rated_mw, 1.0, 1e5);
        c.number("battery.capacity_mj", self.battery.capacity_mj, 0.0, 1e6);
        c.number("battery.capacity_mj", self.battery.capacity_mj, 0.0, 1e6);
        c.number("battery.restart_reserve_mj", self.battery.restart_reserve_mj, 0.0, self.battery.capacity_mj);
        c.number("coolant.inventory_kg", self.coolant.inventory_kg, 1.0, 1e7);
        c.number("routing.trip_dark_s", self.routing.trip_dark_s, 0.0, 600.0);
        for (i, g) in self.generators.iter().enumerate() {
            c.number(&format!("generators.{i}.capacity_mw"), g.capacity_mw, 0.0, 1e4);
        }
        for (i, t) in self.ties.iter().enumerate() {
            c.number(&format!("ties.{i}.capacity_mw"), t.capacity_mw, 0.0, 1e4);
        }
        for (i, k) in self.conduits.iter().enumerate() {
            c.number(&format!("conduits.{i}.capacity_mw"), k.capacity_mw, 0.0, 1e4);
        }
        for (i, l) in self.loads.iter().enumerate() {
            c.number(&format!("loads.{i}.nominal_mw"), l.nominal_mw, 0.0, 1e4);
            c.number(&format!("loads.{i}.standby_mw"), l.standby_mw, 0.0, 1e4);
            c.number(&format!("loads.{i}.setpoint_max"), l.setpoint_max, 0.0, 2.0);
            c.number(&format!("loads.{i}.min_ratio"), l.min_ratio, 0.0, 1.0);
            c.count(&format!("loads.{i}.priority"), i64::from(l.priority), 0, 3);
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SolveRules {
    pub substep_hz: f64,
    pub quanta_rounds: u32,
    pub epsilon_mw: f64,
    pub dropout_resolves: u32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReactorRules {
    pub system: String,
    pub fuel: String,
    pub thermal_rated_mw: f64,
    pub conversion_efficiency: f64,
    pub throttle_min: f64,
    pub throttle_max: f64,
    pub throttle_default: f64,
    pub ramp_up_per_s: f64,
    pub ramp_down_per_s: f64,
    pub fuel_energy_mj_per_kg: f64,
    pub fuel_load_kg: f64,
    pub overdrive_wear_pct_per_min_at_max: f64,
    pub heat: ReactorHeat,
    pub scram: ScramRules,
    pub reset: ResetRules,
    pub restart: RestartRules,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReactorHeat {
    pub capacity_mj_per_k: f64,
    pub loop_kw_per_k: f64,
    pub room_w_per_k: f64,
    pub warn_k: f64,
    pub scram_k: f64,
    pub afterheat_share: f64,
    pub afterheat_tau_s: f64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScramRules {
    pub loop_over_k: f64,
    pub loop_over_hold_s: f64,
    pub aux_supply_below: f64,
    pub aux_rule: String,
    pub aux_supply_hold_s: f64,
    pub integrity_below_pct: f64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResetRules {
    pub fixture: String,
    pub hold_s: f64,
    pub loop_below_k: f64,
    pub blanket_below_k: f64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RestartRules {
    pub ignition_mw: f64,
    pub ignition_s: f64,
    pub start_throttle: f64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BatteryRules {
    pub system: String,
    pub node: String,
    /// Where its link to its node runs, along the ship's axes (power-routing 3a; written by tools/power/boxes.py).
    pub path_m: Option<Vec<[f64; 3]>>,
    pub capacity_mj: f64,
    pub initial_soc: f64,
    pub discharge_max_mw: f64,
    pub charge_max_mw: f64,
    pub charge_efficiency: f64,
    pub discharge_efficiency: f64,
    pub restart_reserve_mj: f64,
    pub heat_to: String,
}

/// A switchboard section, a panel or a bus.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PowerNode {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub compartment: String,
    pub center_m: [f64; 3],
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Generator {
    pub id: String,
    pub name: String,
    pub node: String,
    pub capacity_mw: f64,
    /// Where it runs, from the reactor to its node along the ship's axes (power-routing 3a; written by
    /// tools/power/boxes.py). The schematic draws it; the solve does not read it.
    pub path_m: Option<Vec<[f64; 3]>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Tie {
    pub id: String,
    pub name: String,
    pub between: [String; 2],
    pub capacity_mw: f64,
    pub closed: bool,
    /// Where it runs, end to end along the ship's axes (power-routing 3a; written by tools/power/boxes.py).
    pub path_m: Option<Vec<[f64; 3]>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PowerConduit {
    pub id: String,
    pub name: String,
    pub between: [String; 2],
    pub capacity_mw: f64,
    pub route: Vec<String>,
    pub path_m: Vec<[f64; 3]>,
    /// Closed in the factory configuration (power-routing 1; none: closed). A spare route is open.
    pub factory_closed: Option<bool>,
}

/// A load: what it draws and where its heat goes.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PowerLoad {
    pub id: String,
    pub name: String,
    pub system: Option<String>,
    pub mount: Option<String>,
    pub node: String,
    pub priority: u8,
    pub nominal_mw: f64,
    pub standby_mw: f64,
    pub setpoint_max: f64,
    pub min_ratio: f64,
    /// A pump: its setpoint is its speed, and it draws nominal x speed^exponent.
    pub power_exponent: Option<f64>,
    /// What it is doing, 0-1, names an activity the ship sets (the drive's thrust, a turret's firing).
    pub activity: Option<String>,
    pub center_m: Option<[f64; 3]>,
    pub heat: LoadHeat,
    pub under: String,
    pub over: String,
    /// Its feeder: the run from its node's box to it, ship frame, metres (power-routing 1).
    pub feeder_m: Option<Vec<[f64; 3]>>,
    /// Its second feed (power-routing 1): the next nearest panel, and the feeder from it.
    pub alt_node: Option<String>,
    pub alt_feeder_m: Option<Vec<[f64; 3]>>,
}

/// Where a load's heat goes: the loop, a room, the duct, the lights' rooms, its own thermal node, or nowhere.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LoadHeat {
    pub to: String,
    pub fraction: f64,
    pub capacity_mj_per_k: Option<f64>,
    pub loop_kw_per_k: Option<f64>,
    pub room_w_per_k: Option<f64>,
    pub warn_k: Option<f64>,
    pub damage_k: Option<f64>,
    pub room: Option<String>,
}

/// A group of loads with one fader on Engineering's POWER panel.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Group {
    pub id: String,
    pub name: String,
    pub loads: Vec<String>,
    pub priority_min: Option<u8>,
    pub feed_breaker: Option<bool>,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Lighting {
    pub priority: u8,
    pub normal_w_per_m2: f64,
    pub emergency_w_per_m2: f64,
    pub normal_below_ratio: f64,
    /// Each lighting panel's node and the compartments it lights.
    pub panels: BTreeMap<String, Vec<String>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Coolant {
    pub system: String,
    pub fluid: String,
    pub capacity_mj_per_k: f64,
    pub nominal_k: f64,
    pub initial_k: f64,
    pub warn_k: f64,
    pub pumps: Vec<String>,
    pub flow_per_pump: f64,
    pub design_flow_kg_s: f64,
    pub design_flow_note: String,
    pub specific_heat_kj_per_kg_k: f64,
    pub radiator_pumps: Vec<String>,
    pub flow_per_radiator_pump: f64,
    pub natural_circulation_flow: f64,
    pub flow_alarm_below: f64,
    pub inventory_kg: f64,
    pub tanks: Vec<CoolantTank>,
    pub segments: Vec<CoolantSegment>,
    pub segments_note: String,
    pub isolated_leg_flow: f64,
    pub exchanger: Named,
    pub makeup: Makeup,
    pub cavitation: Cavitation,
    pub bands: Bands,
    pub automation: CoolingAutomation,
    pub cooling_note: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CoolantTank {
    pub id: String,
    pub name: String,
    pub capacity_kg: f64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CoolantSegment {
    pub id: String,
    pub name: String,
    pub leg: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Named {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Makeup {
    pub pump: String,
    pub max_kg_s: f64,
    pub auto_open_below: f64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Cavitation {
    pub full_flow_from: f64,
    pub no_flow_below: f64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bands {
    pub hot_leg_k: [f64; 2],
    pub cold_leg_k: [f64; 2],
    pub hot_leg_warning_k: f64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CoolingAutomation {
    pub period_s: f64,
    pub design_rise_k: f64,
    pub pump_speed_min: f64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Radiators {
    pub system: String,
    pub rated_mw: f64,
    pub rated_at_k: f64,
    pub background_k: f64,
    pub bypass_band_k: f64,
    pub law: String,
    pub note: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Overdrive {
    pub heat_factor: f64,
    pub wear_pct_per_min_at_150: f64,
    /// A load over its damage temperature loses this much integrity a second for each kelvin over, percent.
    pub heat_wear_pct_per_s_per_k: f64,
}

/// A load as the solve sees it: the file's, or a lighting panel's.
#[derive(Debug, Clone)]
pub struct Load {
    pub id: String,
    pub name: String,
    pub system: Option<String>,
    /// Its node's index.
    pub node: usize,
    pub priority: u8,
    pub nominal_mw: f64,
    pub standby_mw: f64,
    pub setpoint_max: f64,
    pub min_ratio: f64,
    pub power_exponent: Option<f64>,
    pub activity: Option<String>,
    pub heat: LoadHeat,
    /// Its index in the file's loads (none: a lighting panel, which damage does not break).
    pub file: Option<usize>,
    /// Its second feed's node's index, if it has one.
    pub alt: Option<usize>,
}

/// The ship's power, loaded and indexed once.
#[derive(Debug, Clone)]
pub struct PowerData {
    pub file: PowerFile,
    /// The file's loads, then a lighting load a panel.
    pub loads: Vec<Load>,
    /// Each group's loads, by index.
    pub group_loads: Vec<Vec<usize>>,
    /// The nodes' ids, in order.
    pub nodes: Vec<String>,
    /// What a part does at an integrity (damage.json `systems`).
    pub systems: SystemRules,
    /// A pipe segment's or tank's leak (damage.json `coolant`).
    pub leak: CoolantLeakRules,
    /// A room's air temperature while the air is not simulated (atmosphere.json `thermal.initial_k`), K.
    pub room_k: f64,
}

/// A compartment's floor area, by its layout id (for the lighting loads).
pub type FloorAreas = BTreeMap<String, f64>;

impl PowerData {
    /// The power data from `power.json`'s text, the damage rules, a room's air temperature (K) and the
    /// compartments' floor areas.
    pub fn parse(text: &str, rules: &DamageData, room_k: f64, floors: &FloorAreas) -> Result<Self, DataError> {
        let path = crate::combat::damage::TERN_FILES[1];
        let file: PowerFile = parse(path, text)?;
        let err = |field: String, message: String| DataError { file: path.into(), field, message };
        let nodes: Vec<String> = file.nodes.iter().map(|n| n.id.clone()).collect();
        let node_of = |id: &str, field: String| {
            nodes.iter().position(|n| n == id).ok_or_else(|| err(field, format!("names {id:?}, which no node is")))
        };
        let mut loads = Vec::new();
        for (i, l) in file.loads.iter().enumerate() {
            loads.push(Load {
                id: l.id.clone(),
                name: l.name.clone(),
                system: l.system.clone(),
                node: node_of(&l.node, format!("loads.{i}.node"))?,
                priority: l.priority,
                nominal_mw: l.nominal_mw,
                standby_mw: l.standby_mw,
                setpoint_max: l.setpoint_max,
                min_ratio: l.min_ratio,
                power_exponent: l.power_exponent,
                activity: l.activity.clone(),
                heat: l.heat.clone(),
                file: Some(i),
                alt: l.alt_node.as_deref().map(|n| node_of(n, format!("loads.{i}.alt_node"))).transpose()?,
            });
        }
        // Lighting, one load a panel, from the floor area it lights (normal lighting).
        let lg = &file.lighting;
        for (panel, rooms) in &lg.panels {
            let area: f64 = rooms.iter().map(|r| floors.get(r).copied().unwrap_or(0.0)).sum();
            let mw = area * lg.normal_w_per_m2 / 1e6;
            loads.push(Load {
                id: format!("lights_{panel}"),
                name: format!("Lighting, {panel}"),
                system: None,
                node: node_of(panel, format!("lighting.panels.{panel}"))?,
                priority: lg.priority,
                nominal_mw: mw,
                standby_mw: mw,
                setpoint_max: 1.0,
                min_ratio: 0.0,
                power_exponent: None,
                activity: None,
                heat: LoadHeat {
                    to: "lights".into(),
                    fraction: 1.0,
                    capacity_mj_per_k: None,
                    loop_kw_per_k: None,
                    room_w_per_k: None,
                    warn_k: None,
                    damage_k: None,
                    room: None,
                },
                file: None,
                alt: None,
            });
        }
        let mut group_loads = Vec::new();
        for (gi, g) in file.groups.iter().enumerate() {
            let mut v = Vec::new();
            for id in &g.loads {
                let k = loads
                    .iter()
                    .position(|l| &l.id == id)
                    .ok_or_else(|| err(format!("groups.{gi}.loads"), format!("names {id:?}, which no load is")))?;
                v.push(k);
            }
            group_loads.push(v);
        }
        for (k, g) in &file.generators.iter().enumerate().map(|(k, g)| (k, g.node.clone())).collect::<Vec<_>>() {
            node_of(g, format!("generators.{k}.node"))?;
        }
        node_of(&file.battery.node, "battery.node".into())?;
        for id in file
            .coolant
            .pumps
            .iter()
            .chain(&file.coolant.radiator_pumps)
            .chain(std::iter::once(&file.coolant.makeup.pump))
        {
            if !loads.iter().any(|l| &l.id == id) {
                return Err(err("coolant".into(), format!("names the pump {id:?}, which no load is")));
            }
        }
        let factory = file.conduits.iter().filter(|k| k.factory_closed.unwrap_or(true)).count()
            + file.ties.iter().filter(|t| t.closed).count();
        if factory > file.routing.max_live_runs as usize {
            return Err(err(
                "routing.max_live_runs".into(),
                format!("is {} but the factory configuration has {factory} runs live", file.routing.max_live_runs),
            ));
        }
        Ok(Self {
            file,
            loads,
            group_loads,
            nodes,
            systems: rules.systems.clone(),
            leak: rules.coolant.clone(),
            room_k,
        })
    }

    /// The index of load `id`.
    pub fn load(&self, id: &str) -> Option<usize> {
        self.loads.iter().position(|l| l.id == id)
    }

    /// The index of node `id`.
    pub fn node(&self, id: &str) -> Option<usize> {
        self.nodes.iter().position(|n| n == id)
    }

    /// The sub-step, seconds.
    pub fn dt(&self) -> f64 {
        1.0 / self.file.solve.substep_hz
    }

    /// A preset's setpoint for group `g` (1 where it names none).
    pub fn preset_setpoint(&self, preset: &str, g: &str) -> Option<f64> {
        let p = self.file.presets.get(preset)?.as_object()?;
        Some(p.get(g).and_then(Value::as_f64).unwrap_or(1.0))
    }

    /// The presets' names, in the file's order of keys (sorted), without the note.
    pub fn presets(&self) -> Vec<&str> {
        self.file.presets.iter().filter(|(_, v)| v.is_object()).map(|(k, _)| k.as_str()).collect()
    }

    fn cap(&self, g: f64) -> f64 {
        self.systems.capability(g)
    }

    /// The heat the loop's flow (a share of design) carries at the design rise, MW.
    pub fn carry_mw(&self, flow: f64) -> f64 {
        let c = &self.file.coolant;
        flow * c.design_flow_kg_s * c.specific_heat_kj_per_kg_k * c.automation.design_rise_k / 1000.0
    }

    /// A group's highest setpoint (its loads' most): 1.5 for most, 1.2 for the cooling pumps.
    pub fn group_max(&self, group: usize) -> f64 {
        self.group_loads[group].iter().map(|&k| self.loads[k].setpoint_max).fold(0.0, f64::max)
    }

    /// A group's MW on its fader's setpoint scale: a share of its nominal, and for pumps the speed that power buys
    /// (they draw nominal x speed^exponent; the console mockup's `groupK`).
    pub fn group_k(&self, group: usize, mw: f64) -> f64 {
        let ls = &self.group_loads[group];
        let nominal: f64 = ls.iter().map(|&k| self.loads[k].nominal_mw).sum();
        let e = ls.first().and_then(|&k| self.loads[k].power_exponent).unwrap_or(1.0);
        if nominal <= 0.0 {
            return 0.0;
        }
        (mw / nominal).max(0.0).powf(1.0 / e)
    }
}

/// The reactor's state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReactorState {
    Running,
    Scrammed,
    Igniting,
}

/// Why the reactor scrammed (power-grid 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScramCause {
    /// SCRAM pressed.
    Manual = 1,
    /// The coolant's hot leg over its limit for its hold.
    LoopHot = 2,
    /// The blanket over its scram temperature.
    BlanketHot = 3,
    /// Neither auxiliary train had its supply for its hold.
    AuxSupply = 4,
    /// The reactor's integrity below its limit.
    Damaged = 5,
}

impl ScramCause {
    /// From its number on the wire.
    pub fn from_u8(v: u8) -> Option<Self> {
        [Self::Manual, Self::LoopHot, Self::BlanketHot, Self::AuxSupply, Self::Damaged]
            .into_iter()
            .find(|c| *c as u8 == v)
    }

    /// What the console says, with the limit from the data.
    pub fn words(self, d: &PowerData) -> String {
        let r = &d.file.reactor;
        match self {
            Self::Manual => "manual".into(),
            Self::LoopHot => format!("coolant hot leg over {:.0} K", r.scram.loop_over_k),
            Self::BlanketHot => format!("blanket over {:.0} K", r.heat.scram_k),
            Self::AuxSupply => format!("auxiliaries below {:.0}% supply", r.scram.aux_supply_below * 100.0),
            Self::Damaged => format!("reactor integrity below {:.0}%", r.scram.integrity_below_pct),
        }
    }
}

/// Who holds a control: the automation, or a hand.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Auto,
    Manual,
}

/// A group's feed breaker (power-grid 5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Breaker {
    Closed,
    Open,
    /// Open and locked: it refuses to close until it is unlocked (opened).
    Locked,
}

/// The reactor.
#[derive(Debug, Clone)]
pub struct Reactor {
    pub mode: Mode,
    pub state: ReactorState,
    pub throttle: f64,
    pub target: f64,
    /// Thermal and electric power, W.
    pub p_th_w: f64,
    pub p_e_w: f64,
    /// The blanket's temperature, K.
    pub t_k: f64,
    pub integrity: f64,
    pub fuel_kg: f64,
    pub timer_loop_s: f64,
    pub timer_aux_s: f64,
    pub scram_cause: Option<ScramCause>,
    /// Seconds of ignition done.
    pub ignition: f64,
    pub release_reserve: bool,
    pub afterheat_w: f64,
}

/// The battery bank.
#[derive(Debug, Clone)]
pub struct Battery {
    pub soc_mj: f64,
    pub out_mw: f64,
    pub in_mw: f64,
    pub health: f64,
}

/// The coolant loop.
#[derive(Debug, Clone)]
pub struct CoolantLoop {
    /// The cold leg, K.
    pub t_k: f64,
    pub hot_k: f64,
    /// Flow, a share of design.
    pub flow: f64,
    pub rad_mw: f64,
    pub rad_cap_mw: f64,
    /// Heat into the loop, MW.
    pub in_mw: f64,
    pub radiator_health: f64,
    /// Each load's branch valve.
    pub branch_open: Vec<bool>,
    pub m_kg: f64,
    pub tanks_kg: Vec<f64>,
    pub rad_flow: f64,
    pub exchanger: f64,
    /// The share of the hot leg through the chiller (the radiators).
    pub chiller: f64,
    pub makeup_valve: f64,
    pub makeup_kg_s: f64,
    pub seg_leak_kg_s: f64,
    pub tank_leak_kg_s: f64,
    pub legs: [f64; 2],
}

/// The engineer's hand on the loop (reactor-cooling design 5).
#[derive(Debug, Clone)]
pub struct Cooling {
    pub mode: Mode,
    pub chiller: f64,
    pub makeup: f64,
    pub timer_s: f64,
    pub auto_makeup: f64,
}

/// What the power step reads from the rest of the ship each sub-step.
pub struct Ship<'a> {
    /// Each file load's integrity, percent (the damage state's); overdrive and heat wear it.
    pub integrity: &'a mut [f64],
    /// Each node's health, 0-1.
    pub node_health: &'a [f64],
    /// Each conduit's carrying share and whether it is severed (the damage state's).
    pub conduits: &'a [ConduitState],
    /// Each activity by name, 0-1.
    pub activity: &'a BTreeMap<String, f64>,
}

/// The ship's power now.
#[derive(Debug, Clone)]
pub struct PowerState {
    pub setpoint: Vec<f64>,
    pub priority: Vec<u8>,
    pub breaker_open: Vec<bool>,
    pub group_breaker: Vec<Breaker>,
    pub tie_closed: Vec<bool>,
    pub gen_closed: Vec<bool>,
    pub conduit_breaker: Vec<bool>,
    /// Each node's box switched on (power-routing 1): off, nothing passes it and nothing it feeds is supplied.
    pub box_on: Vec<bool>,
    /// Each load swapped to its second feed (power-routing 1; the factory feeds every load from its node).
    pub on_alt: Vec<bool>,
    /// Seconds the grid has left dark after a trip (power-routing 2); 0 when it is not tripped.
    pub trip_s: f64,
    /// Trips since the ship started.
    pub trips: u32,
    pub battery_breaker: bool,
    pub reactor: Reactor,
    pub battery: Battery,
    pub lp: CoolantLoop,
    /// Each coolant segment's, then tank's, then the exchanger's integrity and whether it is isolated.
    pub coolant_parts: Vec<(f64, bool)>,
    pub cooling: Cooling,
    /// The makeup pump is running: its valve open, the loop short and the tanks not empty (its activity).
    pub makeup_on: bool,
    /// The last preset applied (none: a hand on a fader since).
    pub preset: Option<String>,
    pub demand: Vec<f64>,
    pub want: Vec<f64>,
    pub alloc: Vec<f64>,
    pub dropped: Vec<bool>,
    pub rx_out: f64,
    pub bat_out: f64,
    pub charge: f64,
    /// Each load's thermal node, K (the loads whose heat goes to a node).
    pub thermal: Vec<f64>,
    pub battery_loss_w: f64,
    /// Heat that went into rooms and the duct, W (the air is not simulated: counted, not warming).
    pub room_heat_w: f64,
    /// Seconds simulated.
    pub t_s: f64,
}

/// One allocation: what each load gets and what the sources give.
#[derive(Debug, Clone)]
pub struct Solved {
    pub demand: Vec<f64>,
    pub want: Vec<f64>,
    pub alloc: Vec<f64>,
    pub dropped: Vec<bool>,
    pub rx_out: f64,
    pub bat_out: f64,
    pub charge: f64,
}

/// The activities that are megawatts wanted, not a 0-1 share: the oxygen plant's, the heater's, the bay's and the
/// airlock's pumps (life support and the bays compute them; the mockup's `computeDemand`).
pub const PLANT_MW: [&str; 4] = ["o2_output", "heater", "bay_pumps", "airlock_pump"];

const S: usize = 0;
const RX: usize = 1;
const BAT: usize = 2;
const NOFF: usize = 3;

#[derive(Debug, Clone)]
struct Edge {
    u: usize,
    v: usize,
    c: f64,
    f: f64,
    directed: bool,
}

/// A snapshot of the grid's edges for one solve (the mockup's `gridSnapshot`).
#[derive(Debug, Clone)]
struct Grid {
    edges: Vec<Edge>,
    adj: Vec<Vec<(usize, i8)>>,
    e_rx: usize,
    e_bat: usize,
    bat_avail: f64,
    bat_avail_all: f64,
    charge_want: f64,
}

impl Grid {
    fn residual(&self, ei: usize, dir: i8) -> f64 {
        let e = &self.edges[ei];
        if dir > 0 {
            e.c - e.f
        } else if e.directed {
            e.f
        } else {
            e.c + e.f
        }
    }

    /// Push up to `amount` MW from the source to node `t` along shortest residual paths (breadth first, stable).
    fn push_to(&mut self, t: usize, amount: f64, eps: f64) -> f64 {
        let n = self.adj.len();
        let mut got = 0.0;
        let mut prev_e = vec![0usize; n];
        let mut prev_d = vec![0i8; n];
        for _ in 0..16 {
            if amount - got <= eps {
                break;
            }
            let mut seen = vec![false; n];
            let mut queue = vec![S];
            seen[S] = true;
            let mut qh = 0;
            while qh < queue.len() && !seen[t] {
                let u = queue[qh];
                qh += 1;
                for &(ei, dir) in &self.adj[u] {
                    let e = &self.edges[ei];
                    let v = if dir > 0 { e.v } else { e.u };
                    if seen[v] || self.residual(ei, dir) <= eps {
                        continue;
                    }
                    seen[v] = true;
                    prev_e[v] = ei;
                    prev_d[v] = dir;
                    queue.push(v);
                }
            }
            if !seen[t] {
                break;
            }
            let mut b = amount - got;
            let mut v = t;
            while v != S {
                let ei = prev_e[v];
                b = b.min(self.residual(ei, prev_d[v]));
                let e = &self.edges[ei];
                v = if prev_d[v] > 0 { e.u } else { e.v };
            }
            let mut v = t;
            while v != S {
                let ei = prev_e[v];
                let d = prev_d[v];
                self.edges[ei].f += f64::from(d) * b;
                let e = &self.edges[ei];
                v = if d > 0 { e.u } else { e.v };
            }
            got += b;
        }
        got
    }
}

impl PowerState {
    /// A ship at cruise: the reactor running at the default throttle's 60 %, the battery at its initial charge, the
    /// loop at its initial temperature, every setpoint 100 % and every breaker closed.
    pub fn new(d: &PowerData) -> Self {
        let f = &d.file;
        let n = d.loads.len();
        let thermal = d.loads.iter().map(|_| f.coolant.initial_k + 5.0).collect();
        let parts = f.coolant.segments.len() + f.coolant.tanks.len() + 1;
        let mut s = Self {
            setpoint: vec![1.0; n],
            priority: d.loads.iter().map(|l| l.priority).collect(),
            breaker_open: vec![false; n],
            group_breaker: vec![Breaker::Closed; f.groups.len()],
            tie_closed: f.ties.iter().map(|t| t.closed).collect(),
            gen_closed: vec![true; f.generators.len()],
            conduit_breaker: f.conduits.iter().map(|k| k.factory_closed.unwrap_or(true)).collect(),
            box_on: vec![true; f.nodes.len()],
            on_alt: vec![false; d.loads.len()],
            trip_s: 0.0,
            trips: 0,
            battery_breaker: true,
            reactor: Reactor {
                mode: Mode::Auto,
                state: ReactorState::Running,
                throttle: f.reactor.throttle_default * 0.6,
                target: f.reactor.throttle_default,
                p_th_w: 0.0,
                p_e_w: 0.0,
                t_k: 600.0,
                integrity: 100.0,
                fuel_kg: f.reactor.fuel_load_kg,
                timer_loop_s: 0.0,
                timer_aux_s: 0.0,
                scram_cause: None,
                ignition: 0.0,
                release_reserve: false,
                afterheat_w: 0.0,
            },
            battery: Battery {
                soc_mj: f.battery.capacity_mj * f.battery.initial_soc,
                out_mw: 0.0,
                in_mw: 0.0,
                health: 1.0,
            },
            lp: CoolantLoop {
                t_k: f.coolant.initial_k,
                hot_k: f.coolant.initial_k,
                flow: 1.0,
                rad_mw: 0.0,
                rad_cap_mw: 0.0,
                in_mw: 0.0,
                radiator_health: 1.0,
                branch_open: vec![true; n],
                m_kg: f.coolant.inventory_kg,
                tanks_kg: f.coolant.tanks.iter().map(|t| t.capacity_kg).collect(),
                rad_flow: 1.0,
                exchanger: 1.0,
                chiller: 1.0,
                makeup_valve: 0.0,
                makeup_kg_s: 0.0,
                seg_leak_kg_s: 0.0,
                tank_leak_kg_s: 0.0,
                legs: [1.0, 1.0],
            },
            coolant_parts: vec![(100.0, false); parts],
            cooling: Cooling { mode: Mode::Auto, chiller: 1.0, makeup: 0.0, timer_s: 0.0, auto_makeup: 0.0 },
            makeup_on: false,
            preset: None,
            demand: vec![0.0; n],
            want: vec![0.0; n],
            alloc: vec![0.0; n],
            dropped: vec![false; n],
            rx_out: 0.0,
            bat_out: 0.0,
            charge: 0.0,
            thermal,
            battery_loss_w: 0.0,
            room_heat_w: 0.0,
            t_s: 0.0,
        };
        let _ = s.apply_preset(d, "cruise");
        s
    }

    fn integrity_of(&self, d: &PowerData, ship_integrity: &[f64], i: usize) -> f64 {
        d.loads[i].file.and_then(|k| ship_integrity.get(k).copied()).unwrap_or(100.0)
    }

    /// The grid's edges now (MW), for one solve.
    fn grid(&self, d: &PowerData, ship: &Ship, dt: f64) -> Grid {
        let f = &d.file;
        let gn = NOFF + d.nodes.len();
        let mut edges: Vec<Edge> = Vec::new();
        let mut adj: Vec<Vec<(usize, i8)>> = vec![Vec::new(); gn];
        let mut add = |u: usize, v: usize, c: f64, directed: bool| {
            adj[u].push((edges.len(), 1));
            adj[v].push((edges.len(), -1));
            edges.push(Edge { u, v, c: c.max(0.0), f: 0.0, directed });
            edges.len() - 1
        };
        let rx = &self.reactor;
        let rx_avail =
            if rx.state == ReactorState::Running { rx.p_th_w * f.reactor.conversion_efficiency / 1e6 } else { 0.0 };
        let b = &f.battery;
        let reserve = if rx.release_reserve { 0.0 } else { b.restart_reserve_mj };
        let lim = |e: f64| (b.discharge_max_mw * self.battery.health).min(e * b.discharge_efficiency / dt).max(0.0);
        let bat_avail = lim(self.battery.soc_mj - reserve);
        let bat_avail_all = lim(self.battery.soc_mj);
        let nh = |id: &str| {
            d.node(id)
                .map_or(1.0, |k| if self.box_on[k] { ship.node_health.get(k).copied().unwrap_or(1.0) } else { 0.0 })
        };
        let p = |id: &str| NOFF + d.node(id).unwrap_or(0);
        let e_rx = add(S, RX, rx_avail, true);
        let e_bat = add(S, BAT, 0.0, true);
        let bat_out = add(BAT, p(&b.node), if self.battery_breaker { 1e9 } else { 0.0 }, true);
        for (k, g) in f.generators.iter().enumerate() {
            let ok = self.gen_closed[k] && nh(&g.node) > 0.0;
            add(RX, p(&g.node), if ok { g.capacity_mw } else { 0.0 }, true);
        }
        for (k, t) in f.ties.iter().enumerate() {
            let ok = self.tie_closed[k] && nh(&t.between[0]) > 0.0 && nh(&t.between[1]) > 0.0;
            add(p(&t.between[0]), p(&t.between[1]), if ok { t.capacity_mw } else { 0.0 }, false);
        }
        for (k, c) in f.conduits.iter().enumerate() {
            let (health, severed) = ship.conduits.get(k).map_or((1.0, false), |c| (c.health, c.severed));
            let ok = !severed && self.conduit_breaker[k] && nh(&c.between[0]) > 0.0 && nh(&c.between[1]) > 0.0;
            add(p(&c.between[0]), p(&c.between[1]), if ok { c.capacity_mw * health } else { 0.0 }, false);
        }
        if nh(&b.node) <= 0.0 {
            edges[bat_out].c = 0.0;
        }
        let charge_want = (b.charge_max_mw * self.battery.health)
            .min((b.capacity_mj - self.battery.soc_mj) / (dt * b.charge_efficiency))
            .max(0.0);
        Grid { edges, adj, e_rx, e_bat, bat_avail, bat_avail_all, charge_want }
    }

    fn may_use_reserve(&self, d: &PowerData, i: usize) -> bool {
        let id = d.loads[i].id.as_str();
        let c = &d.file.coolant;
        id == "emergency_lighting"
            || (id.starts_with("reactor_aux") && self.reactor.state == ReactorState::Igniting)
            || ((c.pumps.iter().any(|p| p == id) || c.radiator_pumps.iter().any(|p| p == id))
                && self.reactor.state != ReactorState::Running
                && self.priority[i] == 0)
    }

    /// Allocate `demand` over a grid: priority classes in order; within a class the reactor first, then the battery;
    /// within a phase a proportional share of what the sources can still give, routed by augmenting paths; a load cut
    /// off by its path keeps what it got and the rest is offered again (the mockup's `solveGrid`).
    fn solve_grid(&self, d: &PowerData, grid: &mut Grid, demand: &[f64]) -> (Vec<f64>, f64, f64, f64) {
        let eps = d.file.solve.epsilon_mw;
        let mut alloc = vec![0.0; d.loads.len()];
        let passes: [(u8, bool); 5] = [(0, true), (0, false), (1, false), (2, false), (3, false)];
        for (p, reserve) in passes {
            let members: Vec<usize> = (0..d.loads.len())
                .filter(|&i| self.priority[i] == p && demand[i] > eps && reserve == self.may_use_reserve(d, i))
                .collect();
            if members.is_empty() {
                continue;
            }
            for phase in [0, 1] {
                let eb = grid.e_bat;
                grid.edges[eb].c = if phase == 1 {
                    grid.edges[eb].f.max(if reserve { grid.bat_avail_all } else { grid.bat_avail })
                } else {
                    grid.edges[eb].c.min(grid.edges[eb].f)
                };
                let mut blocked = vec![false; d.loads.len()];
                for _ in 0..d.file.solve.quanta_rounds {
                    let total: f64 = members.iter().filter(|&&i| !blocked[i]).map(|&i| demand[i] - alloc[i]).sum();
                    let src = (grid.edges[grid.e_rx].c - grid.edges[grid.e_rx].f)
                        + (grid.edges[grid.e_bat].c - grid.edges[grid.e_bat].f);
                    if total <= eps || src <= eps {
                        break;
                    }
                    let lam = (src / total).min(1.0);
                    let mut short = false;
                    for &i in &members {
                        if blocked[i] {
                            continue;
                        }
                        let want = lam * (demand[i] - alloc[i]);
                        if want <= eps {
                            continue;
                        }
                        let got = grid.push_to(NOFF + self.node_of(d, i), want, eps);
                        alloc[i] += got;
                        if got < want - eps {
                            blocked[i] = true;
                            short = true;
                        }
                    }
                    if !short && lam >= 1.0 {
                        break;
                    }
                }
            }
        }
        // The battery charges from the reactor's surplus only, never while it is discharging.
        let eb = grid.e_bat;
        grid.edges[eb].c = grid.edges[eb].f;
        let mut charge = 0.0;
        if grid.edges[eb].f <= eps && grid.charge_want > eps {
            let bn = NOFF + d.node(&d.file.battery.node).unwrap_or(0);
            charge = grid.push_to(bn, grid.charge_want, eps);
        }
        (alloc, grid.edges[grid.e_rx].f, grid.edges[eb].f, charge)
    }

    /// Each load's demand from the setpoints, its activity, its damage and its breaker, MW (the mockup's
    /// `computeDemand`).
    pub fn demand(&self, d: &PowerData, ship: &Ship, setpoints: &[f64]) -> Vec<f64> {
        let mut out = vec![0.0; d.loads.len()];
        for (i, l) in d.loads.iter().enumerate() {
            let sp = setpoints[i].min(l.setpoint_max);
            let fed = self.node_of(d, i);
            let node_dead = !self.box_on[fed] || ship.node_health.get(fed).is_some_and(|h| *h <= 0.0);
            if sp <= 0.0 || self.breaker_open[i] || node_dead {
                continue;
            }
            let cap = d.cap(self.integrity_of(d, ship.integrity, i));
            if cap <= 0.0 {
                continue;
            }
            let aux = l.id.starts_with("reactor_aux");
            let x = if aux && self.reactor.state == ReactorState::Igniting {
                d.file.reactor.restart.ignition_mw / 2.0
            } else if aux && self.reactor.state == ReactorState::Scrammed {
                l.standby_mw
            } else if let Some(e) = l.power_exponent {
                l.nominal_mw * sp.powf(e)
            } else if let Some(mw) = l.activity.as_deref().filter(|k| PLANT_MW.contains(k)) {
                // The plants ask for megawatts (the heater on top of its standby), up to the setpoint's share.
                let want = ship.activity.get(mw).copied().unwrap_or(0.0);
                let base = if mw == "heater" { l.standby_mw } else { 0.0 };
                (base + want).min(l.nominal_mw * sp)
            } else {
                let a = match l.activity.as_deref() {
                    None => 1.0,
                    Some("makeup") => f64::from(u8::from(self.makeup_on)),
                    Some(k) => ship.activity.get(k).copied().unwrap_or(0.0),
                };
                l.standby_mw + (l.nominal_mw * sp - l.standby_mw).max(0.0) * a
            };
            out[i] = x * cap;
        }
        out
    }

    /// The full solve with drop-out: a load below its least ratio switches off and the rest solve again (the
    /// mockup's `solveWithDropout`). The step and every preview call this one function (CLAUDE.md 6.1).
    pub fn solve(&self, d: &PowerData, ship: &Ship, setpoints: &[f64]) -> Solved {
        let dt = d.dt();
        let eps = d.file.solve.epsilon_mw;
        let mut demand = self.demand(d, ship, setpoints);
        let want = demand.clone();
        let mut grid = self.grid(d, ship, dt);
        let mut res = self.solve_grid(d, &mut grid, &demand);
        let mut dropped = vec![false; d.loads.len()];
        for _ in 0..d.file.solve.dropout_resolves {
            let mut any = false;
            for (i, l) in d.loads.iter().enumerate() {
                if demand[i] > eps && l.min_ratio > 0.0 && res.0[i] < l.min_ratio * demand[i] - eps {
                    dropped[i] = true;
                    demand[i] = 0.0;
                    any = true;
                }
            }
            if !any {
                break;
            }
            let mut grid = self.grid(d, ship, dt);
            res = self.solve_grid(d, &mut grid, &demand);
        }
        Solved { demand, want, alloc: res.0, dropped, rx_out: res.1, bat_out: res.2, charge: res.3 }
    }

    /// A group's fader dragged to `sp`: what its loads would want and get (the console's ghost), MW.
    pub fn preview_group(&self, d: &PowerData, ship: &Ship, group: usize, sp: f64) -> (f64, f64) {
        let mut s = self.setpoint.clone();
        for &k in &d.group_loads[group] {
            s[k] = sp;
        }
        let r = self.solve(d, ship, &s);
        d.group_loads[group].iter().fold((0.0, 0.0), |(w, g), &k| (w + r.want[k], g + r.alloc[k]))
    }

    /// The share of its demand a load gets now (1 when it wants nothing): what the system can do of its job.
    pub fn supply(&self, i: usize) -> f64 {
        if self.demand[i] > 1e-9 {
            (self.alloc[i] / self.demand[i]).clamp(0.0, 1.0)
        } else if self.dropped[i] {
            0.0
        } else {
            1.0
        }
    }

    /// One sub-step of `d.dt()` seconds: the loop's controls, the power solve, the heat, and the scram checks (the
    /// mockup's `stepCooling`, `stepPower`, `stepHeat`, `checkScram`, in its order).
    pub fn step(&mut self, d: &PowerData, ship: &mut Ship) {
        let dt = d.dt();
        self.step_routing(d, ship, dt);
        self.step_cooling(d, dt);
        self.step_power(d, ship, dt);
        self.step_heat(d, ship, dt);
        self.check_scram(d, dt);
        self.t_s += dt;
    }

    /// The main protection (power-routing 2): a trip's dark time running out restores the factory configuration;
    /// more live runs than it allows trips the grid.
    fn step_routing(&mut self, d: &PowerData, ship: &Ship, dt: f64) {
        if self.trip_s > 0.0 {
            self.trip_s -= dt;
            if self.trip_s <= 1e-9 {
                self.trip_s = 0.0;
                self.factory_reset(d);
            }
            return;
        }
        if self.live_runs(d, ship) > d.file.routing.max_live_runs as usize {
            self.trip_s = d.file.routing.trip_dark_s;
            self.trips += 1;
            for (k, n) in d.file.nodes.iter().enumerate() {
                self.box_on[k] = d.file.routing.kept_on_trip.contains(&n.kind);
            }
            self.conduit_breaker.iter_mut().for_each(|b| *b = false);
            self.tie_closed.iter_mut().for_each(|b| *b = false);
        }
    }

    /// Every box on, every run at its factory setting.
    pub fn factory_reset(&mut self, d: &PowerData) {
        self.on_alt.iter_mut().for_each(|a| *a = false);
        self.box_on.iter_mut().for_each(|b| *b = true);
        for (k, c) in d.file.conduits.iter().enumerate() {
            self.conduit_breaker[k] = c.factory_closed.unwrap_or(true);
        }
        for (k, t) in d.file.ties.iter().enumerate() {
            self.tie_closed[k] = t.closed;
        }
    }

    /// The runs live now (power-routing 2): conduits and ties closed, whole, with both their boxes on.
    pub fn live_runs(&self, d: &PowerData, ship: &Ship) -> usize {
        let on = |id: &str| d.node(id).is_some_and(|k| self.box_on[k]);
        let conduits = d.file.conduits.iter().enumerate().filter(|(k, c)| {
            self.conduit_breaker[*k]
                && !ship.conduits.get(*k).is_some_and(|s| s.severed)
                && on(&c.between[0])
                && on(&c.between[1])
        });
        let ties = d
            .file
            .ties
            .iter()
            .enumerate()
            .filter(|(k, t)| self.tie_closed[*k] && on(&t.between[0]) && on(&t.between[1]));
        conduits.count() + ties.count()
    }

    /// The nodes power reaches from the generators' and the battery's nodes over live runs, given each node's health
    /// and each conduit's state.
    pub fn reached(&self, d: &PowerData, node_health: &[f64], conduits: &[ConduitState]) -> Vec<bool> {
        let n = d.nodes.len();
        let up = |k: usize| self.box_on[k] && node_health.get(k).is_none_or(|h| *h > 0.0);
        let mut edges: Vec<(usize, usize)> = Vec::new();
        for (k, c) in d.file.conduits.iter().enumerate() {
            let whole = conduits.get(k).is_none_or(|s| !s.severed && s.health > 0.0);
            if self.conduit_breaker[k] && whole {
                if let (Some(a), Some(b)) = (d.node(&c.between[0]), d.node(&c.between[1])) {
                    edges.push((a, b));
                }
            }
        }
        for (k, t) in d.file.ties.iter().enumerate() {
            if self.tie_closed[k] {
                if let (Some(a), Some(b)) = (d.node(&t.between[0]), d.node(&t.between[1])) {
                    edges.push((a, b));
                }
            }
        }
        let mut seen = vec![false; n];
        let mut stack: Vec<usize> = d
            .file
            .generators
            .iter()
            .map(|g| g.node.as_str())
            .chain(std::iter::once(d.file.battery.node.as_str()))
            .filter_map(|id| d.node(id))
            .filter(|&k| up(k))
            .collect();
        for &k in &stack {
            seen[k] = true;
        }
        while let Some(u) = stack.pop() {
            for &(a, b) in &edges {
                let v = if a == u {
                    b
                } else if b == u {
                    a
                } else {
                    continue;
                };
                if !seen[v] && up(v) {
                    seen[v] = true;
                    stack.push(v);
                }
            }
        }
        seen
    }

    /// A spare run to close (power-routing 3, Engineering's automation): the first open, whole conduit, in the
    /// data's order, from a node power reaches to a switched-on node it does not that feeds a load, while one more
    /// live run stays within the protection's limit.
    pub fn reroute(&self, d: &PowerData, node_health: &[f64], conduits: &[ConduitState]) -> Option<usize> {
        let reached = self.reached(d, node_health, conduits);
        let n = d.nodes.len();
        let up = |k: usize| self.box_on[k] && node_health.get(k).is_none_or(|h| *h > 0.0);
        let whole = |k: usize| conduits.get(k).is_none_or(|s| !s.severed && s.health > 0.0);
        // The dark islands: switched-on nodes power does not reach, joined by closed whole runs; one wants power
        // when a load on any of its nodes does.
        let mut links: Vec<(usize, usize)> = Vec::new();
        for (k, c) in d.file.conduits.iter().enumerate() {
            if self.conduit_breaker[k] && whole(k) {
                if let (Some(a), Some(b)) = (d.node(&c.between[0]), d.node(&c.between[1])) {
                    links.push((a, b));
                }
            }
        }
        let mut island = vec![usize::MAX; n];
        for start in 0..n {
            if reached[start] || !up(start) || island[start] != usize::MAX {
                continue;
            }
            island[start] = start;
            let mut stack = vec![start];
            while let Some(u) = stack.pop() {
                for &(a, b) in &links {
                    let v = if a == u {
                        b
                    } else if b == u {
                        a
                    } else {
                        continue;
                    };
                    if !reached[v] && up(v) && island[v] == usize::MAX {
                        island[v] = start;
                        stack.push(v);
                    }
                }
            }
        }
        let wants = |k: usize| {
            island[k] != usize::MAX
                && (0..d.loads.len())
                    .any(|i| d.loads[i].nominal_mw > 0.0 && island.get(self.node_of(d, i)) == Some(&island[k]))
        };
        if !(0..n).any(wants) {
            return None;
        }
        let on = |id: &str| d.node(id).is_some_and(|k| self.box_on[k]);
        let live = d
            .file
            .conduits
            .iter()
            .enumerate()
            .filter(|(k, c)| {
                self.conduit_breaker[*k]
                    && !conduits.get(*k).is_some_and(|s| s.severed)
                    && on(&c.between[0])
                    && on(&c.between[1])
            })
            .count()
            + d.file
                .ties
                .iter()
                .enumerate()
                .filter(|(k, t)| self.tie_closed[*k] && on(&t.between[0]) && on(&t.between[1]))
                .count();
        if live + 1 > d.file.routing.max_live_runs as usize {
            return None;
        }
        d.file.conduits.iter().enumerate().find_map(|(k, c)| {
            let (a, b) = (d.node(&c.between[0])?, d.node(&c.between[1])?);
            let bridges = (reached[a] && wants(b)) || (reached[b] && wants(a));
            (!self.conduit_breaker[k] && whole(k) && bridges).then_some(k)
        })
    }

    /// Switch node `node`'s box on or off (none while the grid is tripped dark).
    pub fn set_box(&mut self, node: usize, on: bool) -> bool {
        if self.trip_s > 0.0 || node >= self.box_on.len() {
            return false;
        }
        self.box_on[node] = on;
        true
    }

    /// The node load `i` draws from now: its own, or its second feed's once swapped.
    pub fn node_of(&self, d: &PowerData, i: usize) -> usize {
        match (self.on_alt[i], d.loads[i].alt) {
            (true, Some(a)) => a,
            _ => d.loads[i].node,
        }
    }

    /// Swap load `i` to its second feed (true) or back (none while dark, or for a load with one feed).
    pub fn set_feed(&mut self, d: &PowerData, i: usize, alt: bool) -> bool {
        if self.trip_s > 0.0 || d.loads.get(i).and_then(|l| l.alt).is_none() {
            return false;
        }
        self.on_alt[i] = alt;
        true
    }

    /// A system to swap to its other feed (power-routing 3, Engineering's automation): the first, in the data's
    /// order, whose panel now gets no power while its other panel does.
    pub fn transfer(&self, d: &PowerData, node_health: &[f64], conduits: &[ConduitState]) -> Option<usize> {
        let reached = self.reached(d, node_health, conduits);
        (0..d.loads.len()).find(|&i| {
            let Some(alt) = d.loads[i].alt else { return false };
            let (now, other) = if self.on_alt[i] { (alt, d.loads[i].node) } else { (d.loads[i].node, alt) };
            !reached[now] && reached[other]
        })
    }

    /// Close or open conduit `k` at its box (none while the grid is tripped dark).
    pub fn set_conduit(&mut self, k: usize, closed: bool) -> bool {
        if self.trip_s > 0.0 || k >= self.conduit_breaker.len() {
            return false;
        }
        self.conduit_breaker[k] = closed;
        true
    }

    fn step_power(&mut self, d: &PowerData, ship: &Ship, dt: f64) {
        let rc = &d.file.reactor;
        let rated_e = rc.thermal_rated_mw * rc.conversion_efficiency * (self.reactor.integrity / 100.0);
        // Automation follows the load the grid can take from the reactor: a probe solve with the reactor at its rated
        // output and the battery held back. A hand sets the throttle; power the grid cannot take heats the blanket.
        if self.reactor.mode == Mode::Auto && self.reactor.state == ReactorState::Running {
            let mut g0 = self.grid(d, ship, dt);
            g0.edges[g0.e_rx].c = rated_e;
            g0.bat_avail = 0.0;
            g0.bat_avail_all = 0.0;
            let dem = self.demand(d, ship, &self.setpoint);
            let (_, rx_out, _, charge) = self.solve_grid(d, &mut g0, &dem);
            self.reactor.target = ((rx_out + charge) / rated_e.max(1e-6)).clamp(rc.throttle_min, 1.0);
        }
        let rx = &mut self.reactor;
        if rx.state == ReactorState::Running {
            let dd = rx.target - rx.throttle;
            rx.throttle += if dd > 0.0 { dd.min(rc.ramp_up_per_s * dt) } else { dd.max(-rc.ramp_down_per_s * dt) };
            rx.throttle = rx.throttle.clamp(rc.throttle_min, rc.throttle_max);
            rx.p_th_w = rx.throttle * rc.thermal_rated_mw * 1e6 * (rx.integrity / 100.0);
        } else {
            rx.p_th_w = 0.0;
        }
        let res = self.solve(d, ship, &self.setpoint);
        self.reactor.p_e_w = res.rx_out * 1e6;
        let b = &d.file.battery;
        let bat = &mut self.battery;
        bat.soc_mj = (bat.soc_mj - res.bat_out * dt / b.discharge_efficiency + res.charge * dt * b.charge_efficiency)
            .clamp(0.0, b.capacity_mj);
        bat.out_mw = res.bat_out;
        bat.in_mw = res.charge;
        self.battery_loss_w =
            (res.bat_out * (1.0 / b.discharge_efficiency - 1.0) + res.charge * (1.0 - b.charge_efficiency)) * 1e6;
        self.rx_out = res.rx_out;
        self.bat_out = res.bat_out;
        self.charge = res.charge;
        self.demand = res.demand;
        self.want = res.want;
        self.alloc = res.alloc;
        self.dropped = res.dropped;
        let rx = &mut self.reactor;
        rx.fuel_kg = (rx.fuel_kg - rx.p_th_w * dt / (rc.fuel_energy_mj_per_kg * 1e6)).max(0.0);
        if rx.state == ReactorState::Running && rx.throttle > 1.0 {
            let over = (rx.throttle - 1.0) / (rc.throttle_max - 1.0);
            rx.integrity = (rx.integrity - rc.overdrive_wear_pct_per_min_at_max / 60.0 * dt * over).max(0.0);
        }
        if self.reactor.state == ReactorState::Igniting {
            let rate = self.ignition_rate(d);
            let rx = &mut self.reactor;
            rx.ignition += dt * rate;
            if rx.ignition >= rc.restart.ignition_s {
                rx.state = ReactorState::Running;
                rx.throttle = rc.restart.start_throttle;
            }
        }
    }

    fn aux_need(&self, l: &Load, d: &PowerData) -> f64 {
        match self.reactor.state {
            ReactorState::Igniting => d.file.reactor.restart.ignition_mw / 2.0,
            ReactorState::Running => l.nominal_mw,
            ReactorState::Scrammed => l.standby_mw,
        }
    }

    /// The reactor's auxiliaries are two whole trains: it runs while either has its supply (the better train's ratio).
    fn aux_ratio(&self, d: &PowerData) -> f64 {
        ["reactor_aux_p", "reactor_aux_s"]
            .iter()
            .filter_map(|id| d.load(id))
            .map(|i| self.alloc[i] / self.aux_need(&d.loads[i], d).max(1e-9))
            .fold(0.0, f64::max)
            .min(1.0)
    }

    fn ignition_rate(&self, d: &PowerData) -> f64 {
        let a: f64 = ["reactor_aux_p", "reactor_aux_s"].iter().filter_map(|id| d.load(id)).map(|i| self.alloc[i]).sum();
        (a / d.file.reactor.restart.ignition_mw).clamp(0.0, 1.0)
    }

    /// A pump's speed (0 to its setpoint's most): the speed its delivered power allows.
    pub fn pump_speed(&self, d: &PowerData, integrity: &[f64], i: usize) -> f64 {
        let l = &d.loads[i];
        let cap = d.cap(self.integrity_of(d, integrity, i));
        if l.nominal_mw <= 0.0 || cap <= 0.0 {
            return 0.0;
        }
        (self.alloc[i] / (l.nominal_mw * cap)).max(0.0).powf(1.0 / l.power_exponent.unwrap_or(1.0))
    }

    fn pumps_flow(&self, d: &PowerData, integrity: &[f64], ids: &[String], share: f64) -> f64 {
        ids.iter()
            .filter_map(|id| d.load(id))
            .map(|i| self.pump_speed(d, integrity, i) * d.cap(self.integrity_of(d, integrity, i)) * share)
            .sum()
    }

    fn cavitation(&self, d: &PowerData) -> f64 {
        let c = &d.file.coolant;
        ((self.lp.m_kg / c.inventory_kg - c.cavitation.no_flow_below)
            / (c.cavitation.full_flow_from - c.cavitation.no_flow_below))
            .clamp(0.0, 1.0)
    }

    fn leg_factor(&self, d: &PowerData, leg: &str) -> f64 {
        let c = &d.file.coolant;
        let (mut worst, mut isolated) = (1.0f64, false);
        for (k, sg) in c.segments.iter().enumerate() {
            if sg.leg != leg {
                continue;
            }
            let (g, iso) = self.coolant_parts[k];
            if iso {
                isolated = true;
            } else {
                worst = worst.min(d.cap(g));
            }
        }
        if isolated {
            c.isolated_leg_flow * worst
        } else {
            worst
        }
    }

    fn loop_flow(&mut self, d: &PowerData, integrity: &[f64]) -> f64 {
        let c = &d.file.coolant;
        self.lp.legs = [self.leg_factor(d, "hot"), self.leg_factor(d, "cold")];
        let pumps = self.pumps_flow(d, integrity, &c.pumps, c.flow_per_pump);
        c.natural_circulation_flow.max(pumps * self.cavitation(d) * self.lp.legs[0].min(self.lp.legs[1]))
    }

    fn part_leak(&self, d: &PowerData, k: usize) -> f64 {
        let (g, isolated) = self.coolant_parts[k];
        let exchanger = k == self.coolant_parts.len() - 1;
        let lk = &d.leak;
        if isolated || exchanger || g >= lk.leak_below_pct {
            0.0
        } else {
            lk.leak_kg_s_at_zero * (lk.leak_below_pct - g) / lk.leak_below_pct
        }
    }

    /// The loop's controls before the solve (reactor-cooling design 5): the makeup valve, its pump's activity, and in
    /// AUTO each period the core pumps at the flow the heat needs, the radiator pumps at full.
    fn step_cooling(&mut self, d: &PowerData, dt: f64) {
        let c = &d.file.coolant;
        let frac = self.lp.m_kg / c.inventory_kg;
        if self.cooling.mode == Mode::Auto {
            if frac < c.makeup.auto_open_below {
                self.cooling.auto_makeup = 1.0;
            } else if frac >= 1.0 {
                self.cooling.auto_makeup = 0.0;
            }
            self.lp.makeup_valve = self.cooling.auto_makeup;
        } else {
            self.lp.makeup_valve = self.cooling.makeup;
        }
        let tanks: f64 = self.lp.tanks_kg.iter().sum();
        self.makeup_on = self.lp.makeup_valve > 0.0 && frac < 1.0 && tanks > 0.0;
        if self.cooling.mode != Mode::Auto {
            return;
        }
        self.cooling.timer_s -= dt;
        if self.cooling.timer_s > 0.0 {
            return;
        }
        self.cooling.timer_s = c.automation.period_s;
        let need = self.lp.in_mw / d.carry_mw(1.0).max(1e-9);
        for id in &c.pumps {
            if let Some(i) = d.load(id) {
                self.setpoint[i] = need.max(c.automation.pump_speed_min).clamp(0.0, d.loads[i].setpoint_max);
            }
        }
        for id in &c.radiator_pumps {
            if let Some(i) = d.load(id) {
                self.setpoint[i] = 1.0;
            }
        }
    }

    fn step_heat(&mut self, d: &PowerData, ship: &mut Ship, dt: f64) {
        let f = &d.file;
        let c = &f.coolant;
        let rd = &f.radiators;
        self.lp.flow = self.loop_flow(d, ship.integrity);
        self.lp.rad_flow = self.pumps_flow(d, ship.integrity, &c.radiator_pumps, c.flow_per_radiator_pump);
        let ex = self.coolant_parts.len() - 1;
        self.lp.exchanger = d.cap(self.coolant_parts[ex].0);
        let mut to_loop = 0.0;
        let mut to_rooms = 0.0;
        for (i, l) in d.loads.iter().enumerate() {
            let h = &l.heat;
            let aw = self.alloc[i] * 1e6;
            let over = if l.nominal_mw > 0.0 { (self.alloc[i] / l.nominal_mw - 1.0).max(0.0) } else { 0.0 };
            let q = h.fraction * aw * (1.0 + f.overdrive.heat_factor * over);
            // Overdrive wears any load: 2 % a minute at 150 %, in proportion.
            if over > 0.0 {
                if let Some(k) = l.file {
                    let wear = dt * (f.overdrive.wear_pct_per_min_at_150 / 60.0) * (over / 0.5);
                    ship.integrity[k] = (ship.integrity[k] - wear).max(0.0);
                }
            }
            match h.to.as_str() {
                "loop" => to_loop += q,
                "room" | "duct" | "lights" => to_rooms += q,
                "node" => {
                    let branch = if self.lp.branch_open[i] { 1.0 } else { 0.0 };
                    let t = self.thermal[i];
                    let q_loop = h.loop_kw_per_k.unwrap_or(0.0) * 1000.0 * self.lp.flow * branch * (t - self.lp.t_k);
                    let q_room = h.room_w_per_k.unwrap_or(0.0) * (t - d.room_k);
                    let cap = h.capacity_mj_per_k.unwrap_or(1.0) * 1e6;
                    self.thermal[i] = t + dt * (q - q_loop - q_room) / cap;
                    to_loop += q_loop;
                    to_rooms += q_room;
                    // Over its damage temperature it wears.
                    if let (Some(dk), Some(k)) = (h.damage_k, l.file) {
                        if self.thermal[i] > dk {
                            let wear = dt * (self.thermal[i] - dk) * f.overdrive.heat_wear_pct_per_s_per_k;
                            ship.integrity[k] = (ship.integrity[k] - wear).max(0.0);
                        }
                    }
                }
                _ => {}
            }
        }
        // The reactor's blanket, with its afterheat after a scram.
        let rh = &f.reactor.heat;
        let rx = &mut self.reactor;
        rx.afterheat_w =
            if rx.state == ReactorState::Running { 0.0 } else { rx.afterheat_w * (-dt / rh.afterheat_tau_s).exp() };
        let waste = (rx.p_th_w - rx.p_e_w).max(0.0) + rx.afterheat_w;
        let q_rx_loop = rh.loop_kw_per_k * 1000.0 * self.lp.flow * (rx.t_k - self.lp.t_k);
        let q_rx_room = rh.room_w_per_k * (rx.t_k - d.room_k);
        rx.t_k += dt * (waste - q_rx_loop - q_rx_room) / (rh.capacity_mj_per_k * 1e6);
        to_loop += q_rx_loop;
        to_rooms += q_rx_room;
        if f.battery.heat_to == "loop" {
            to_loop += self.battery_loss_w;
        } else {
            to_rooms += self.battery_loss_w;
        }
        self.room_heat_w = to_rooms;
        // The radiators, by the fourth-power law; the bypass holds the loop near nominal when the heat is low.
        let lp = &mut self.lp;
        let tb4 = rd.background_k.powi(4);
        lp.rad_cap_mw =
            rd.rated_mw * lp.radiator_health * lp.flow * lp.exchanger * lp.rad_flow * (lp.t_k.powi(4) - tb4)
                / (rd.rated_at_k.powi(4) - tb4);
        let open = if self.cooling.mode == Mode::Manual {
            self.cooling.chiller
        } else if rd.bypass_band_k > 0.0 {
            ((lp.t_k - (c.nominal_k - rd.bypass_band_k)) / rd.bypass_band_k).clamp(0.0, 1.0)
        } else {
            1.0
        };
        lp.chiller = open;
        lp.rad_mw = lp.rad_cap_mw.max(0.0) * open;
        lp.in_mw = to_loop / 1e6;
        // The inventory: the makeup pump feeds the loop from the tanks while its valve is open.
        let mi = d.load(&c.makeup.pump).unwrap_or(0);
        let tanks: f64 = self.lp.tanks_kg.iter().sum();
        let mk_supply = if self.demand[mi] > 1e-9 { (self.alloc[mi] / self.demand[mi]).clamp(0.0, 1.0) } else { 0.0 };
        let mk = (c.makeup.max_kg_s * self.lp.makeup_valve * mk_supply * dt)
            .min(tanks)
            .min(c.inventory_kg - self.lp.m_kg)
            .max(0.0);
        if mk > 0.0 {
            for t in self.lp.tanks_kg.iter_mut() {
                *t -= mk * (*t / tanks);
            }
        }
        self.lp.makeup_kg_s = mk / dt;
        let nseg = c.segments.len();
        let seg_leak: f64 = (0..nseg).map(|k| self.part_leak(d, k)).sum();
        let mut tank_leak = 0.0;
        for k in 0..c.tanks.len() {
            let q = self.lp.tanks_kg[k].min(self.part_leak(d, nseg + k) * dt);
            self.lp.tanks_kg[k] -= q;
            tank_leak += q / dt;
        }
        let lp = &mut self.lp;
        lp.seg_leak_kg_s = seg_leak.min((lp.m_kg + mk) / dt);
        lp.tank_leak_kg_s = tank_leak;
        lp.m_kg = (lp.m_kg + mk - seg_leak * dt).clamp(0.0, c.inventory_kg);
        let c_loop = c.capacity_mj_per_k * 1e6 * (lp.m_kg / c.inventory_kg).max(0.05);
        lp.t_k += dt * (to_loop - lp.rad_mw * 1e6) / c_loop;
        lp.hot_k = lp.t_k + to_loop / (lp.flow * c.design_flow_kg_s * c.specific_heat_kj_per_kg_k * 1000.0).max(1e-9);
        let _ = rd;
    }

    fn check_scram(&mut self, d: &PowerData, dt: f64) {
        if self.reactor.state != ReactorState::Running {
            return;
        }
        let sc = &d.file.reactor.scram;
        let aux = self.aux_ratio(d);
        let rx = &mut self.reactor;
        rx.timer_loop_s = if self.lp.hot_k > sc.loop_over_k { rx.timer_loop_s + dt } else { 0.0 };
        rx.timer_aux_s = if aux < sc.aux_supply_below { rx.timer_aux_s + dt } else { 0.0 };
        let cause = if rx.timer_loop_s >= sc.loop_over_hold_s {
            Some(ScramCause::LoopHot)
        } else if rx.t_k > d.file.reactor.heat.scram_k {
            Some(ScramCause::BlanketHot)
        } else if rx.timer_aux_s >= sc.aux_supply_hold_s {
            Some(ScramCause::AuxSupply)
        } else if rx.integrity < sc.integrity_below_pct {
            Some(ScramCause::Damaged)
        } else {
            None
        };
        if let Some(c) = cause {
            self.scram(d, c);
        }
    }

    /// Scram the reactor: it stops, its afterheat runs on.
    pub fn scram(&mut self, d: &PowerData, cause: ScramCause) {
        let rx = &mut self.reactor;
        rx.state = ReactorState::Scrammed;
        rx.scram_cause = Some(cause);
        rx.timer_loop_s = 0.0;
        rx.timer_aux_s = 0.0;
        rx.afterheat_w = d.file.reactor.heat.afterheat_share * rx.p_th_w;
        rx.throttle = 0.0;
        rx.p_th_w = 0.0;
    }

    /// The hands-on reset at the reactor panel: refused while a cause persists. `Ok` starts the ignition.
    pub fn scram_reset(&mut self, d: &PowerData) -> Result<(), String> {
        if let Some(why) = self.reset_refusal(d) {
            return Err(why);
        }
        let rx = &mut self.reactor;
        rx.state = ReactorState::Igniting;
        rx.ignition = 0.0;
        rx.scram_cause = None;
        Ok(())
    }

    /// Whether either auxiliary train, at the integrities `integrity` (the file's loads'), can carry a running
    /// reactor: its capability at least the supply the scram rule asks (power-grid 3, the best-train rule). A crew
    /// does not reset into a fault that would scram the reactor again.
    pub fn trains_hold(&self, d: &PowerData, integrity: &[f64]) -> bool {
        ["reactor_aux_p", "reactor_aux_s"]
            .iter()
            .filter_map(|id| d.load(id))
            .any(|i| d.cap(self.integrity_of(d, integrity, i)) >= d.file.reactor.scram.aux_supply_below)
    }

    /// Why the reactor panel's reset would be refused now (none: it would be taken).
    pub fn reset_refusal(&self, d: &PowerData) -> Option<String> {
        let rs = &d.file.reactor.reset;
        let rx = &self.reactor;
        if rx.state != ReactorState::Scrammed {
            Some("not scrammed".into())
        } else if self.lp.t_k >= rs.loop_below_k {
            Some(format!("loop too hot ({:.0} K)", self.lp.t_k))
        } else if rx.t_k >= rs.blanket_below_k {
            Some(format!("blanket too hot ({:.0} K)", rx.t_k))
        } else if rx.integrity < d.file.reactor.scram.integrity_below_pct {
            Some("reactor damaged".into())
        } else {
            None
        }
    }

    /// Apply a named preset: every group's setpoint (power-grid 8).
    pub fn apply_preset(&mut self, d: &PowerData, name: &str) -> bool {
        if !d.presets().contains(&name) {
            return false;
        }
        for (gi, g) in d.file.groups.iter().enumerate() {
            let v = d.preset_setpoint(name, &g.id).unwrap_or(1.0);
            for &k in &d.group_loads[gi] {
                self.setpoint[k] = v;
            }
        }
        self.preset = Some(name.to_owned());
        true
    }

    /// A group's fader set by hand: its loads' setpoints (held to each load's most); the preset is no longer whole.
    pub fn set_group(&mut self, d: &PowerData, group: usize, sp: f64) {
        for &k in &d.group_loads[group] {
            self.setpoint[k] = sp.clamp(0.0, d.loads[k].setpoint_max);
        }
        self.preset = None;
        // A hand on the Cooling fader takes the loop from its automation, as a lever does (reactor-cooling design 5).
        if d.file.coolant.pumps.iter().any(|id| d.load(id).is_some_and(|k| d.group_loads[group].contains(&k))) {
            self.set_cooling(d, Some(Mode::Manual), None, None, None, None);
        }
    }

    /// A tap on a group's priority: toward the back of the queue and round (1, 3, 2, 1; a group that may be vital,
    /// Cooling, 0, 3, 2, 1, 0), the console mockup's rule.
    pub fn cycle_priority(&mut self, d: &PowerData, group: usize) -> bool {
        let lo = d.file.groups[group].priority_min.unwrap_or(1);
        let p = self.group_priority(d, group);
        self.set_priority(d, group, if p == lo { 3 } else { p - 1 })
    }

    /// Toggle a switch on the one-line diagram: a generator's breaker (`0..generators`), then a tie's.
    pub fn toggle_switch(&mut self, d: &PowerData, k: usize) -> bool {
        if self.trip_s > 0.0 {
            return false;
        }
        let g = d.file.generators.len();
        if k < g {
            self.gen_closed[k] = !self.gen_closed[k];
        } else if k - g < self.tie_closed.len() {
            self.tie_closed[k - g] = !self.tie_closed[k - g];
        } else {
            return false;
        }
        true
    }

    /// How well file load `i` works now: its capability at `integrity` times the share of its demand it gets (the
    /// console mockup's `works`). The fight reads every system's power through this.
    pub fn works(&self, d: &PowerData, integrity: f64, i: usize) -> f64 {
        d.cap(integrity) * self.supply(i)
    }

    /// A group's priority (0-3; never below the group's least, 1 for most).
    pub fn set_priority(&mut self, d: &PowerData, group: usize, p: u8) -> bool {
        let lo = d.file.groups[group].priority_min.unwrap_or(1);
        if d.group_loads[group].iter().any(|&k| d.loads[k].priority < lo) {
            return false;
        }
        for &k in &d.group_loads[group] {
            self.priority[k] = p.clamp(lo, 3);
        }
        true
    }

    /// A group's priority now: the least of its loads' (the console mockup's `groupNow`).
    pub fn group_priority(&self, d: &PowerData, group: usize) -> u8 {
        d.group_loads[group].iter().map(|&k| self.priority[k]).min().unwrap_or(3)
    }

    /// Open, close or lock open a group's feed breaker; a locked breaker refuses to close.
    pub fn set_group_breaker(&mut self, d: &PowerData, group: usize, state: Breaker) -> bool {
        if d.file.groups[group].feed_breaker != Some(true) {
            return false;
        }
        if state == Breaker::Closed && self.group_breaker[group] == Breaker::Locked {
            return false;
        }
        self.group_breaker[group] = state;
        for &k in &d.group_loads[group] {
            self.breaker_open[k] = state != Breaker::Closed;
        }
        true
    }

    /// The engineer's hand on the reactor: AUTO, or MANUAL at `throttle`.
    pub fn set_reactor(&mut self, d: &PowerData, mode: Mode, throttle: Option<f64>) {
        self.reactor.mode = mode;
        if let (Mode::Manual, Some(t)) = (mode, throttle) {
            self.reactor.target = t.clamp(d.file.reactor.throttle_min, d.file.reactor.throttle_max);
        }
    }

    /// The engineer's hand on the loop: its mode, the pumps' and radiator pumps' speeds, the chiller's share, the
    /// makeup valve.
    pub fn set_cooling(
        &mut self,
        d: &PowerData,
        mode: Option<Mode>,
        pumps: Option<f64>,
        radiators: Option<f64>,
        chiller: Option<f64>,
        makeup: Option<bool>,
    ) {
        let c = &d.file.coolant;
        if mode == Some(Mode::Manual) && self.cooling.mode != Mode::Manual {
            self.cooling.chiller = self.lp.chiller;
            self.cooling.makeup = self.lp.makeup_valve;
        }
        if mode == Some(Mode::Auto) && self.cooling.mode != Mode::Auto {
            self.cooling.timer_s = 0.0;
        }
        if let Some(m) = mode {
            self.cooling.mode = m;
        }
        let mut speed = |ids: &[String], v: f64| {
            for id in ids {
                if let Some(i) = d.load(id) {
                    self.setpoint[i] = v.clamp(0.0, d.loads[i].setpoint_max);
                }
            }
        };
        if let Some(v) = pumps {
            speed(&c.pumps, v);
        }
        if let Some(v) = radiators {
            speed(&c.radiator_pumps, v);
        }
        if let Some(v) = chiller {
            self.cooling.chiller = v.clamp(0.0, 1.0);
        }
        if let Some(on) = makeup {
            self.cooling.makeup = f64::from(u8::from(on));
        }
    }

    /// What every load wants and gets, MW, summed.
    pub fn totals(&self) -> (f64, f64) {
        (self.want.iter().sum(), self.alloc.iter().sum())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data() -> PowerData {
        crate::combat::data::DrillData::shipped().power
    }

    /// Cruise's activities (the mockup's defaults): the drive at 30 %, the dampers at 20 %, RCS at 10 %, comms at 20 %,
    /// the rest idle.
    fn cruise() -> BTreeMap<String, f64> {
        [("drive", 0.3), ("dampers", 0.2), ("rcs", 0.1), ("comms", 0.2)]
            .into_iter()
            .map(|(k, v)| (k.to_owned(), v))
            .collect()
    }

    fn run(d: &PowerData, s: &mut PowerState, act: &BTreeMap<String, f64>, secs: f64) {
        let file_loads = d.file.loads.len();
        let mut integrity = vec![100.0; file_loads];
        let nodes = vec![1.0; d.nodes.len()];
        let conduits = vec![ConduitState { health: 1.0, severed: false, open: false }; d.file.conduits.len()];
        for _ in 0..(secs / d.dt()) as usize {
            let mut ship = Ship { integrity: &mut integrity, node_health: &nodes, conduits: &conduits, activity: act };
            s.step(d, &mut ship);
        }
    }

    #[test]
    fn cruise_needs_less_than_half_the_reactor() {
        // power-grid design 12: 21.2 MW wanted and delivered with the reactor at 44 %, plus the radiator pumps'
        // 1.2 MW that reactor-cooling added since: 22.4 MW at 47 %.
        let d = data();
        let mut s = PowerState::new(&d);
        run(&d, &mut s, &cruise(), 300.0);
        let (want, got) = s.totals();
        assert!((18.0..24.0).contains(&want), "cruise wants {want:.1} MW");
        assert!((want - got).abs() < 0.05, "and gets it all: {got:.1} of {want:.1} MW");
        assert!((0.35..0.55).contains(&s.reactor.throttle), "the reactor near 44 %: {:.2}", s.reactor.throttle);
        assert!(s.battery.soc_mj > d.file.battery.capacity_mj * 0.99, "the battery full: {:.0} MJ", s.battery.soc_mj);
        assert!((310.0..345.0).contains(&s.lp.t_k), "the loop settles near 328 K: {:.0}", s.lp.t_k);
    }

    #[test]
    fn a_preview_equals_the_next_solve() {
        let d = data();
        let mut s = PowerState::new(&d);
        let act = cruise();
        run(&d, &mut s, &act, 30.0);
        let shields = d.file.groups.iter().position(|g| g.id == "shields").unwrap();
        let mut integrity = vec![100.0; d.file.loads.len()];
        let nodes = vec![1.0; d.nodes.len()];
        let conduits = vec![ConduitState { health: 1.0, severed: false, open: false }; d.file.conduits.len()];
        let ship = Ship { integrity: &mut integrity, node_health: &nodes, conduits: &conduits, activity: &act };
        let (want, got) = s.preview_group(&d, &ship, shields, 1.5);
        let mut s2 = s.clone();
        s2.set_group(&d, shields, 1.5);
        let r = s2.solve(&d, &ship, &s2.setpoint);
        let real: (f64, f64) =
            d.group_loads[shields].iter().fold((0.0, 0.0), |(w, g), &k| (w + r.want[k], g + r.alloc[k]));
        assert_eq!((want, got), real, "the fader's ghost is the solve's own answer (CLAUDE.md 6.1)");
    }

    #[test]
    fn the_solve_is_deterministic() {
        let d = data();
        let act = cruise();
        let mut a = PowerState::new(&d);
        let mut b = PowerState::new(&d);
        run(&d, &mut a, &act, 20.0);
        run(&d, &mut b, &act, 20.0);
        assert_eq!(a.alloc, b.alloc);
        assert_eq!(a.lp.t_k.to_bits(), b.lp.t_k.to_bits());
    }

    #[test]
    fn a_cut_port_trunk_loses_nothing() {
        let d = data();
        let mut s = PowerState::new(&d);
        let act = cruise();
        run(&d, &mut s, &act, 60.0);
        let k = d.file.conduits.iter().position(|c| c.id == "k_mid_p").expect("the port trunk midships");
        let mut integrity = vec![100.0; d.file.loads.len()];
        let nodes = vec![1.0; d.nodes.len()];
        let mut conduits = vec![ConduitState { health: 1.0, severed: false, open: false }; d.file.conduits.len()];
        conduits[k] = ConduitState { health: 0.0, severed: true, open: false };
        let ship = Ship { integrity: &mut integrity, node_health: &nodes, conduits: &conduits, activity: &act };
        let r = s.solve(&d, &ship, &s.setpoint);
        for (i, l) in d.loads.iter().enumerate() {
            assert!(r.alloc[i] >= r.demand[i] - 1e-3, "{} keeps its supply through the other trunk", l.id);
        }
    }

    #[test]
    fn a_scram_stops_the_reactor_and_the_battery_carries_the_vital_loads() {
        let d = data();
        let mut s = PowerState::new(&d);
        let act = cruise();
        run(&d, &mut s, &act, 60.0);
        s.scram(&d, ScramCause::Manual);
        run(&d, &mut s, &act, 5.0);
        assert_eq!(s.reactor.state, ReactorState::Scrammed);
        assert!(s.rx_out < 1e-6, "no reactor power");
        assert!(s.bat_out > 1.0, "the battery carries the ship: {:.1} MW", s.bat_out);
        let aux = d.load("coolant_pump_a").unwrap();
        assert!(s.supply(aux) > 0.99, "the coolant pumps keep running");
        assert!(
            s.scram_reset(&d).is_ok() || s.lp.t_k >= d.file.reactor.reset.loop_below_k,
            "reset when the loop is cool"
        );
    }

    #[test]
    fn the_combat_preset_raises_the_shields_and_holds_the_drive_back() {
        let d = data();
        let mut s = PowerState::new(&d);
        assert!(s.apply_preset(&d, "combat"));
        let shields = d.load("shield_generator").unwrap();
        let drive = d.load("impulse_drive").unwrap();
        assert_eq!(s.setpoint[shields], d.preset_setpoint("combat", "shields").unwrap());
        assert_eq!(s.setpoint[drive], d.preset_setpoint("combat", "drive").unwrap());
        assert!(!s.apply_preset(&d, "warp speed"), "an unknown preset changes nothing");
    }

    #[test]
    fn a_preview_equals_the_next_solve_for_every_group_and_setpoint() {
        // power-grid 3.8: the fader's ghost is the solve's own answer, across every group and its whole travel.
        let d = data();
        let act = cruise();
        let mut s = PowerState::new(&d);
        run(&d, &mut s, &act, 10.0);
        let mut integrity = vec![100.0; d.file.loads.len()];
        let nodes = vec![1.0; d.nodes.len()];
        let conduits = vec![ConduitState { health: 1.0, severed: false, open: false }; d.file.conduits.len()];
        for g in 0..d.file.groups.len() {
            for k in 0..=6 {
                let sp = d.group_max(g) * f64::from(k) / 6.0;
                let ship = Ship { integrity: &mut integrity, node_health: &nodes, conduits: &conduits, activity: &act };
                let ghost = s.preview_group(&d, &ship, g, sp);
                let mut after = s.clone();
                after.set_group(&d, g, sp);
                let r = after.solve(&d, &ship, &after.setpoint);
                let real = d.group_loads[g].iter().fold((0.0, 0.0), |(w, x), &k| (w + r.want[k], x + r.alloc[k]));
                assert_eq!(ghost, real, "{} at {sp:.2}", d.file.groups[g].id);
            }
        }
    }

    #[test]
    fn losing_one_main_switchboard_section_does_not_scram_the_reactor() {
        // power-grid 3.5, the best-train rule: the starboard train carries the reactor alone.
        let d = data();
        let act = cruise();
        let mut s = PowerState::new(&d);
        run(&d, &mut s, &act, 30.0);
        let mut integrity = vec![100.0; d.file.loads.len()];
        let mut nodes = vec![1.0; d.nodes.len()];
        nodes[d.node("msb_p").unwrap()] = 0.0;
        let conduits = vec![ConduitState { health: 1.0, severed: false, open: false }; d.file.conduits.len()];
        for _ in 0..(20.0 / d.dt()) as usize {
            let mut ship = Ship { integrity: &mut integrity, node_health: &nodes, conduits: &conduits, activity: &act };
            s.step(&d, &mut ship);
        }
        assert_eq!(s.reactor.state, ReactorState::Running, "scrammed: {:?}", s.reactor.scram_cause);
        assert_eq!(s.alloc[d.load("reactor_aux_p").unwrap()], 0.0, "the port train is dark");
    }

    /// A whole ship: every load, node and conduit sound.
    struct Whole {
        integrity: Vec<f64>,
        nodes: Vec<f64>,
        conduits: Vec<ConduitState>,
    }

    impl Whole {
        fn new(d: &PowerData) -> Self {
            Self {
                integrity: vec![100.0; d.file.loads.len()],
                nodes: vec![1.0; d.nodes.len()],
                conduits: vec![ConduitState { health: 1.0, severed: false, open: false }; d.file.conduits.len()],
            }
        }
        fn step(&mut self, d: &PowerData, s: &mut PowerState, act: &BTreeMap<String, f64>, secs: f64) {
            for _ in 0..(secs / d.dt()).round() as usize {
                let mut ship = Ship {
                    integrity: &mut self.integrity,
                    node_health: &self.nodes,
                    conduits: &self.conduits,
                    activity: act,
                };
                s.step(d, &mut ship);
            }
        }
        fn live(&mut self, d: &PowerData, s: &PowerState, act: &BTreeMap<String, f64>) -> usize {
            let ship = Ship {
                integrity: &mut self.integrity,
                node_health: &self.nodes,
                conduits: &self.conduits,
                activity: act,
            };
            s.live_runs(d, &ship)
        }
    }

    fn conduit(d: &PowerData, id: &str) -> usize {
        d.file.conduits.iter().position(|k| k.id == id).unwrap_or_else(|| panic!("no conduit {id}"))
    }

    #[test]
    fn every_room_has_a_box_and_every_load_a_feeder_from_it() {
        // power-routing 1: "a power grid needs to connect everything together".
        let dd = crate::combat::data::DrillData::shipped();
        let d = &dd.power;
        for r in &dd.tern_layout.rooms {
            assert!(d.file.nodes.iter().any(|n| n.compartment == r.id), "{} has no box", r.id);
        }
        for l in &d.file.loads {
            let f = l.feeder_m.as_ref().unwrap_or_else(|| panic!("{} has no feeder", l.id));
            let n = &d.file.nodes[d.node(&l.node).unwrap()];
            assert_eq!(f[0], n.center_m, "{}'s feeder starts at its box", l.id);
        }
    }

    #[test]
    fn every_room_is_lit_at_cruise() {
        // power-routing 1: each box lights its own room, down to a pod's 24 W; none is under the solve's tolerance.
        let d = data();
        let mut s = PowerState::new(&d);
        run(&d, &mut s, &cruise(), 10.0);
        for (i, l) in d.loads.iter().enumerate().filter(|(_, l)| l.id.starts_with("lights_")) {
            assert!(s.alloc[i] > 0.0 && s.supply(i) > 0.99, "{} gets {} of {} MW", l.id, s.alloc[i], s.demand[i]);
        }
    }

    #[test]
    fn every_panel_connects_to_at_least_two_others() {
        // The owner, 2026-10-11: "there should be a panel in every room, panels should connect to at least 2 other
        // panels", so a room always has a second way for its power to come.
        let d = data();
        let mut joined: Vec<std::collections::BTreeSet<&str>> = vec![Default::default(); d.nodes.len()];
        for (a, b) in d
            .file
            .conduits
            .iter()
            .map(|k| (&k.between[0], &k.between[1]))
            .chain(d.file.ties.iter().map(|t| (&t.between[0], &t.between[1])))
        {
            joined[d.node(a).unwrap()].insert(b);
            joined[d.node(b).unwrap()].insert(a);
        }
        for (k, j) in joined.iter().enumerate() {
            assert!(j.len() >= 2, "{} is joined to {:?}", d.nodes[k], j);
        }
    }

    #[test]
    fn every_system_has_two_feeds_the_nearest_first() {
        // The owner, 2026-10-11: "systems should be connected to 2 or more panels, so we can swap with a different
        // route, it should generally connect to the nearest panel".
        let d = data();
        for l in &d.file.loads {
            let alt = l.alt_node.as_ref().unwrap_or_else(|| panic!("{} has one feed", l.id));
            assert_ne!(alt, &l.node, "{}'s two feeds are two panels", l.id);
            assert!(l.alt_feeder_m.is_some(), "{} has a feeder from its second panel", l.id);
        }
        let shield = d.file.loads.iter().find(|l| l.id == "shield_generator").unwrap();
        assert_eq!(shield.node, "bx_shield_room", "the shield generator is fed from its own room's panel");
    }

    #[test]
    fn the_reactor_feeds_five_panels() {
        // The owner, 2026-10-11: "reactor should connect to 4 -5 panels, since it is a critical resource".
        let d = data();
        let fed: std::collections::BTreeSet<&str> = d.file.generators.iter().map(|g| g.node.as_str()).collect();
        assert_eq!(fed.len(), 5, "{fed:?}");
        let mut s = PowerState::new(&d);
        run(&d, &mut s, &cruise(), 30.0);
        assert_eq!(s.reactor.state, ReactorState::Running);
        let k = d.file.generators.iter().position(|g| g.id == "gen_p").unwrap();
        s.gen_closed[k] = false;
        let k = d.file.generators.iter().position(|g| g.id == "gen_s").unwrap();
        s.gen_closed[k] = false;
        run(&d, &mut s, &cruise(), 30.0);
        let (want, got) = s.totals();
        assert!(
            (want - got).abs() < 0.05,
            "both generators open, the other three feeds carry the ship: {got:.1} of {want:.1} MW"
        );
    }

    #[test]
    fn a_system_swapped_to_its_second_feed_draws_from_that_panel() {
        let d = data();
        let act = cruise();
        let mut s = PowerState::new(&d);
        let mut w = Whole::new(&d);
        let shields = d.load("shield_generator").unwrap();
        w.conduits[conduit(&d, "r_shield")] = ConduitState { health: 0.0, severed: true, open: false };
        w.step(&d, &mut s, &act, 2.0);
        assert!(s.alloc[shields] < 1e-9, "the shield room's feed is cut");
        assert_eq!(s.transfer(&d, &w.nodes, &w.conduits), Some(shields), "its other panel has power: swap it");
        assert!(s.set_feed(&d, shields, true));
        assert_eq!(d.nodes[s.node_of(&d, shields)], "fsb_p", "fed from the forward switchboard now");
        w.step(&d, &mut s, &act, 2.0);
        assert!(s.supply(shields) > 0.99, "and it has its power back");
    }

    #[test]
    fn the_factory_configuration_is_under_the_protections_limit() {
        let d = data();
        let s = PowerState::new(&d);
        let mut w = Whole::new(&d);
        let live = w.live(&d, &s, &cruise());
        assert_eq!(live, 39, "the trunks, the ties and one feed into every room");
        assert!(live < d.file.routing.max_live_runs as usize);
    }

    #[test]
    fn a_box_switched_off_darkens_what_it_feeds() {
        let d = data();
        let act = cruise();
        let mut s = PowerState::new(&d);
        let mut w = Whole::new(&d);
        w.step(&d, &mut s, &act, 10.0);
        let shields = d.load("shield_generator").unwrap();
        assert!(s.supply(shields) > 0.99);
        assert!(s.set_box(d.node("bx_shield_room").unwrap(), false));
        w.step(&d, &mut s, &act, 1.0);
        assert!(s.alloc[shields] < 1e-9, "the shield room's box is off: its generator has nothing");
        s.set_box(d.node("bx_shield_room").unwrap(), true);
        w.step(&d, &mut s, &act, 1.0);
        assert!(s.supply(shields) > 0.99, "and on again it has its power back");
    }

    #[test]
    fn a_spare_run_closed_routes_power_round_a_cut_feed() {
        let d = data();
        let act = cruise();
        let mut s = PowerState::new(&d);
        let mut w = Whole::new(&d);
        let turret = d.load("turret_dorsal").unwrap();
        w.conduits[conduit(&d, "r_dorsal_fwd")] = ConduitState { health: 0.0, severed: true, open: false };
        w.step(&d, &mut s, &act, 2.0);
        assert!(s.alloc[turret] < 1e-9, "the dorsal turret's only feed is cut");
        assert!(s.set_conduit(conduit(&d, "r_dorsal_aft"), true), "close the spare through the aft passage");
        w.step(&d, &mut s, &act, 2.0);
        assert!(s.supply(turret) > 0.99, "the turret is fed from engineering the long way round");
    }

    #[test]
    fn the_reroute_closes_the_spare_that_reaches_a_cut_off_box() {
        let d = data();
        let s = PowerState::new(&d);
        let mut w = Whole::new(&d);
        assert_eq!(s.reroute(&d, &w.nodes, &w.conduits), None, "nothing is cut off");
        w.conduits[conduit(&d, "r_dorsal_fwd")] = ConduitState { health: 0.0, severed: true, open: false };
        let k = s.reroute(&d, &w.nodes, &w.conduits).expect("the dorsal turret's room is cut off");
        assert_eq!(d.file.conduits[k].id, "r_dorsal_aft", "the spare through the aft passage reaches it");
    }

    #[test]
    fn too_many_live_runs_trip_the_grid_dark_then_it_comes_back_at_its_factory_settings() {
        // The owner, 2026-10-11: "Too many conduits open can cause the ship to shut down temporarily and reset the
        // configuration to factory settings".
        let d = data();
        let act = cruise();
        let mut s = PowerState::new(&d);
        let mut w = Whole::new(&d);
        w.step(&d, &mut s, &act, 10.0);
        let spares: Vec<usize> =
            (0..d.file.conduits.len()).filter(|&k| d.file.conduits[k].factory_closed == Some(false)).collect();
        let room = d.file.routing.max_live_runs as usize - w.live(&d, &s, &act);
        for &k in &spares[..room] {
            s.set_conduit(k, true);
        }
        w.step(&d, &mut s, &act, 1.0);
        assert_eq!(s.trips, 0, "at the limit the grid holds");
        s.set_conduit(spares[room], true);
        w.step(&d, &mut s, &act, 0.2);
        assert_eq!(s.trips, 1, "one run more trips it");
        let (shields, aux) = (d.load("shield_generator").unwrap(), d.load("reactor_aux_p").unwrap());
        assert!(s.alloc[shields] < 1e-9, "dark past the switchboards");
        assert!(s.supply(aux) > 0.99, "the switchboards' own loads keep their power");
        assert_eq!(s.reactor.state, ReactorState::Running, "so the reactor runs on");
        assert!(!s.set_conduit(spares[0], false), "nothing can be switched while it is dark");
        w.step(&d, &mut s, &act, d.file.routing.trip_dark_s + 0.5);
        assert_eq!(s.trip_s, 0.0);
        assert!(spares.iter().all(|&k| !s.conduit_breaker[k]), "every spare open again: the factory settings");
        assert!(s.supply(shields) > 0.99, "and the ship has its power back");
    }

    #[test]
    fn the_reset_is_refused_while_the_loop_is_too_hot() {
        let d = data();
        let mut s = PowerState::new(&d);
        s.scram(&d, ScramCause::LoopHot);
        s.lp.t_k = d.file.reactor.reset.loop_below_k + 5.0;
        let why = s.scram_reset(&d).expect_err("a hot loop refuses the reset");
        assert!(why.contains("loop too hot"), "{why}");
        assert_eq!(s.reactor.state, ReactorState::Scrammed);
        s.lp.t_k = d.file.reactor.reset.loop_below_k - 5.0;
        assert!(s.scram_reset(&d).is_ok(), "a cool loop takes it");
        assert_eq!(s.reactor.state, ReactorState::Igniting, "the reset starts the ignition");
    }

    #[test]
    fn a_reset_reactor_ignites_on_its_auxiliaries_and_runs_again() {
        let d = data();
        let act = cruise();
        let mut s = PowerState::new(&d);
        run(&d, &mut s, &act, 30.0);
        s.scram(&d, ScramCause::Manual);
        run(&d, &mut s, &act, 1.0);
        s.scram_reset(&d).expect("a cool loop takes the reset");
        run(&d, &mut s, &act, d.file.reactor.restart.ignition_s + 2.0);
        assert_eq!(
            s.reactor.state,
            ReactorState::Running,
            "ignition done in its {} s",
            d.file.reactor.restart.ignition_s
        );
        run(&d, &mut s, &act, 60.0);
        let (want, got) = s.totals();
        assert!((want - got).abs() < 0.05, "and the ship has its power again: {got:.1} of {want:.1} MW");
    }

    #[test]
    fn the_damage_layout_and_the_grid_index_the_same_loads_nodes_and_conduits() {
        // The damage state's integrity, node health and conduits are read by the grid by index (Ship).
        let dd = crate::combat::data::DrillData::shipped();
        let (l, d) = (&dd.tern_layout, &dd.power);
        let ids = |v: Vec<&str>| v.into_iter().map(str::to_owned).collect::<Vec<_>>();
        assert_eq!(
            ids(l.loads.iter().map(|x| x.id.as_str()).collect()),
            ids(d.file.loads.iter().map(|x| x.id.as_str()).collect())
        );
        assert_eq!(
            ids(l.nodes.iter().map(|x| x.id.as_str()).collect()),
            ids(d.file.nodes.iter().map(|x| x.id.as_str()).collect())
        );
        assert_eq!(
            ids(l.conduits.iter().map(|x| x.id.as_str()).collect()),
            ids(d.file.conduits.iter().map(|x| x.id.as_str()).collect())
        );
    }

    #[test]
    fn the_lights_draw_twelve_watts_a_square_metre_of_floor() {
        // power-grid design 6: 29.3 kW over 2,445.5 m^2.
        let d = data();
        let lights: f64 = d.loads.iter().filter(|l| l.id.starts_with("lights_")).map(|l| l.nominal_mw).sum();
        assert!((lights * 1000.0 - 29.3).abs() < 0.2, "lighting {:.1} kW", lights * 1000.0);
    }

    /// The numbers behind the tests, for tuning: `cargo test -p sc-core power_report -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn power_report() {
        let d = data();
        let mut s = PowerState::new(&d);
        let act = cruise();
        for t in [0.0, 10.0, 60.0, 300.0] {
            let more = t - s.t_s.min(t);
            run(&d, &mut s, &act, more);
            let (want, got) = s.totals();
            println!(
                "t {:>5.0} s  want {want:5.2} got {got:5.2} MW  throttle {:.3}  rx {:5.2} bat {:5.2} charge {:5.2}  soc {:5.0} MJ  loop {:5.1} K hot {:5.1}  flow {:.2}  blanket {:5.0} K  rad {:5.2}/{:5.2} MW  in {:5.2}",
                s.t_s, s.reactor.throttle, s.rx_out, s.bat_out, s.charge, s.battery.soc_mj, s.lp.t_k, s.lp.hot_k,
                s.lp.flow, s.reactor.t_k, s.lp.rad_mw, s.lp.rad_cap_mw, s.lp.in_mw
            );
        }
        for (i, l) in d.loads.iter().enumerate() {
            println!(
                "  {:<22} p{} want {:6.3} got {:6.3}  T {:5.1}",
                l.id, s.priority[i], s.want[i], s.alloc[i], s.thermal[i]
            );
        }
        s.scram(&d, ScramCause::Manual);
        for t in [1.0, 10.0, 60.0, 300.0, 900.0] {
            run(&d, &mut s, &act, t);
            println!(
                "scram +{t:>4.0}s  bat {:5.2} soc {:5.0}  loop {:5.1} blanket {:5.0}  reset {:?}",
                s.bat_out,
                s.battery.soc_mj,
                s.lp.t_k,
                s.reactor.t_k,
                s.clone().scram_reset(&d)
            );
        }
    }
}
