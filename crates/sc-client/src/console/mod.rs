//! The bridge consoles, drawn as their approved mockup draws them (CLAUDE.md 10; openspec/changes/console-parity).
//!
//! `docs/mockups/consoles.html` is the consoles' specification. This module is its five consoles (helm, tactical,
//! engineering, science and the captain's), ported function for function onto the [`vg`](crate::vg) canvas: the
//! title band, the look band (the viewscreen, its camera widget and the navball), the four panels of each console on
//! the 104 x 102 grid, and the status strip.
//! Everything they show comes from one [`ConsoleView`], which is the mockup's own `window.consoleState()` in shape:
//! the drill fills it from its snapshot every frame, and the parity check (`sc-client --console-fixture`) reads it
//! from the file the mockup wrote, so the engine draws the mockup's exact moment beside the mockup's picture.
//!
//! Drawing records where each control is ([`Hits`]); the caller turns a press, a drag or a hold on one into an
//! [`Act`], and the drill turns that into a command or a change to the console's own view.

mod captain;
mod engineering;
mod gunner;
mod helm;
pub mod kit;
mod laser;
mod objectives;
pub mod rig;
pub mod schematic;
mod science;
mod tactical;

use crate::vg::{self, alpha, rgba, Canvas, Pen, Xf, T};
use egui::{Color32, Pos2, Rect};
use glam::{DQuat, DVec3};
use serde::Deserialize;
use std::sync::OnceLock;

pub use kit::Act;

/// One heading, pitch and roll, degrees: an attitude order's three thumbwheels.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Hpr {
    pub h: f64,
    pub p: f64,
    pub r: f64,
}

/// A slew's plan (flight-and-navigation 6a): seconds, degrees, and whether the order is held.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PlanView {
    pub time: f64,
    pub angle: f64,
    #[serde(default)]
    pub held: bool,
}

/// The ship's flight limits the controls are drawn against.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct FlightView {
    /// Rate limits [pitch (x), yaw (y), roll (z)], deg/s.
    pub rate_dps: [f64; 3],
    pub speed_min_mps: f64,
    pub speed_max_mps: f64,
    pub strafe_max_mps: f64,
}

/// A contact, in ship axes.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ContactView {
    pub id: String,
    /// corvette, fighter or freighter.
    pub cls: String,
    /// hostile, neutral, friendly or unknown.
    pub iff: String,
    /// Where it is from the ship, ship axes, metres.
    pub rel: [f64; 3],
    /// Its heading, a unit vector in ship axes.
    pub dir: [f64; 3],
    /// Hull, percent.
    pub hull: f64,
    /// How far Science's scan has gone, 0-1 (the hull shows from 0.3).
    pub scan: f64,
    /// Its top, a unit vector in ship axes (target-bracket 1); upright when a saved state has none.
    #[serde(default = "up_y")]
    pub up: [f64; 3],
    /// Its hull box in its own axes (+z its bow, +y its top), least then greatest corner, metres; none when its class
    /// has no model (target-bracket 1).
    #[serde(default)]
    pub box_m: Option<[[f64; 3]; 2]>,
}

fn up_y() -> [f64; 3] {
    [0.0, 1.0, 0.0]
}

/// An inbound missile: from direction `d` (ship axes, toward the shooter), `t` seconds out.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct InboundView {
    pub d: [f64; 3],
    pub t: f64,
}

/// A recent hit on the shield.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct HitView {
    /// Toward the attacker, ship axes.
    pub d: [f64; 3],
    pub face: u8,
    pub age_s: f64,
    /// Some of it reached the hull.
    pub through: bool,
}

/// A scanner's camera: yaw about the ship's dorsal axis and tilt from the panel's own, degrees.
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ScanCam {
    pub yaw: f64,
    #[serde(rename = "dEl")]
    pub d_el: f64,
}

/// The three scanners' cameras: helm's, tactical's and science's.
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ScanCams {
    pub nav: ScanCam,
    pub tac: ScanCam,
    #[serde(default)]
    pub sci: ScanCam,
}

impl ScanCams {
    /// The camera of scanner `key` (nav, tac or sci).
    pub fn of(&self, key: &str) -> ScanCam {
        match key {
            "nav" => self.nav,
            "tac" => self.tac,
            _ => self.sci,
        }
    }
}

/// A camera turned round the ship: yaw and elevation, degrees.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Cam {
    pub yaw: f64,
    pub el: f64,
}

/// A turret's dial.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct TurretView {
    /// Its letter.
    pub short: String,
    /// AUTO, TARGET, PD or HOLD.
    pub mode: String,
    /// Its gunner, if one is in the pod.
    pub op: Option<String>,
    /// Heat, 0-1 of lockout.
    pub heat: f64,
    /// Where it points: bearing, degrees to starboard of the bow.
    pub aim_deg: f64,
    pub firing: bool,
    pub cooling: bool,
    /// How well it works, 0-1 (supply and integrity).
    pub ok: f64,
}

/// A missile tube.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct TubeView {
    /// EMPTY, LOADING or READY.
    pub state: String,
    /// Seconds into loading.
    pub t: f64,
}

/// A guarded control being held: its key and how far the hold has gone, 0-1.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct HoldView {
    pub key: String,
    pub k: f64,
}

/// Science's own state: the contact it has picked, its scan and ping, and its shield view.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SciView {
    /// The contact picked for the CONTACT panel and its scan.
    pub sel: Option<String>,
    pub scanning: bool,
    /// Seconds left of an active ping's ring (0: none).
    pub ping: f64,
    /// The SHIELDS panel's camera.
    pub cam: Cam,
    /// The shield's frequency band, A-D.
    pub freq: String,
    /// Who set the shields last: Science or Tactical.
    pub by: String,
}

/// An order from the captain: to whom, what, and whether it is done (sent or done).
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct OrderView {
    pub to: String,
    pub verb: String,
    pub state: String,
}

/// The order being written: to whom, and the verb picked.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ComposerView {
    pub to: String,
    pub verb: Option<String>,
}

/// A row of the CREW panel: the station's role, who holds it (none: automation) and whether an order waits on it.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CrewRow {
    pub role: String,
    pub op: Option<String>,
    pub order: bool,
}

/// A compartment's state on the ship plan, by its layout id: OK, NO POWER, DAMAGED, SMOKE, FIRE or OPEN TO SPACE.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RoomView {
    pub id: String,
    pub word: String,
}

/// Where a crew member is (a compartment's layout id) and whether they are well.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CrewAt {
    pub comp: Option<String>,
    pub ok: bool,
}

/// The chosen compartment's air and crew, for the ROOM panel.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RoomReadout {
    pub p_kpa: f64,
    pub po2_kpa: f64,
    pub t_k: f64,
    pub crew: u32,
}

/// The captain's console: the CMD and SHIP tabs, the orders, the crew and every compartment.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CapView {
    /// CMD or SHIP.
    pub tab: String,
    /// The compartment picked on the SHIP tab.
    pub ship_sel: String,
    pub braced: bool,
    /// The captain has taken the viewscreen.
    pub view_override: bool,
    pub composer: ComposerView,
    /// The last orders, newest first.
    pub orders: Vec<OrderView>,
    pub crew: Vec<CrewRow>,
    /// Every compartment's state, in the layout's order (none: nothing aboard simulates them).
    pub rooms: Vec<RoomView>,
    pub crew_at: Vec<CrewAt>,
    /// The picked compartment's readings (none: no atmosphere is simulated).
    pub room: Option<RoomReadout>,
}

/// A load group's fader (power-grid 5): MW wanted and delivered, its setpoint and priority, and its feed breaker.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GroupView {
    /// Its id, which is also its icon.
    pub id: String,
    pub name: String,
    /// MW wanted and delivered.
    pub want: f64,
    pub got: f64,
    /// Its setpoint, 0-max.
    pub sp: f64,
    /// 0 (never shed) to 3.
    pub prio: u8,
    /// The highest setpoint.
    pub max: f64,
    /// What it wants and gets on the setpoint's scale.
    pub want_k: f64,
    pub got_k: f64,
    /// Its feed breaker, if it has one: closed, open or locked.
    pub breaker: Option<String>,
}

/// A fader being dragged: the solve's answer at that setpoint, on the setpoint's scale.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PreviewView {
    pub gid: String,
    pub sp: f64,
    pub k: f64,
}

/// The reactor (power-grid 4).
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ReactorView {
    pub up: bool,
    /// Throttle and its target, 0-throttle_max.
    pub throttle: f64,
    pub target: f64,
    pub p_e_mw: f64,
    /// auto or manual.
    pub mode: String,
    pub scram_cause: Option<String>,
    /// The blanket's temperature, its warning and scram, K.
    pub t_k: f64,
    pub warn_k: f64,
    pub scram_k: f64,
    pub throttle_max: f64,
}

/// A pump: speed, what it can do, its power and setpoint.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PumpView {
    pub name: String,
    pub speed: f64,
    pub capability: f64,
    pub alloc_mw: f64,
    pub setpoint: f64,
}

/// A coolant tank.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct TankView {
    pub name: String,
    pub kg: f64,
    pub frac: f64,
}

/// The reactor's cooling loop (reactor-cooling 5 and 6).
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CoolView {
    /// auto or manual.
    pub mode: String,
    /// Flow, a share of design, and in kg/s.
    pub flow: f64,
    pub flow_kg_s: f64,
    /// The share of the hot leg through the chiller.
    pub chiller: f64,
    pub hot_k: f64,
    pub cold_k: f64,
    pub pumps: Vec<PumpView>,
    pub radiator_pumps: Vec<PumpView>,
    pub makeup_valve: f64,
    pub makeup_kg_s: f64,
    pub tanks: Vec<TankView>,
    /// The loop's fill, a share of its inventory.
    pub inventory: f64,
    /// The exchanger's capability.
    pub exchanger: f64,
    pub rad_mw: f64,
    pub rad_cap_mw: f64,
    pub rad_flow: f64,
    /// The hot leg's band and its warning, K.
    pub hot_band_k: [f64; 2],
    pub hot_warn_k: f64,
    /// The loop's scram, K.
    pub limit_k: f64,
    /// Heat in and what the flow carries, MW.
    pub heat_mw: f64,
    pub carry_mw: f64,
    /// The pumps' and radiator pumps' highest setpoints, and the makeup's most, kg/s.
    pub pump_max: f64,
    pub rad_max: f64,
    pub makeup_max_kg_s: f64,
}

/// The one-line diagram's breakers and buses, and the battery.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct BusesView {
    pub gen_p: bool,
    pub gen_s: bool,
    pub tie: bool,
    pub eb_p: bool,
    pub eb_s: bool,
    /// The two main buses are whole.
    pub np: bool,
    pub ns: bool,
    /// The battery's charge, 0-1.
    pub soc: f64,
    /// Cables cut.
    pub cut: u32,
    /// The runs live, the main protection's limit, and the seconds left dark after a trip (power-routing 2): the
    /// SCHEMATIC panel's count (0: a fixture from before the routing).
    #[serde(default)]
    pub live: u32,
    #[serde(default)]
    pub limit: u32,
    #[serde(default)]
    pub trip_s: f64,
    /// The board in small (power-routing 6.2): each edge's and node's state (none: a fixture from before it).
    #[serde(default)]
    pub thumb: Option<schematic::ThumbView>,
}

/// The AIR panel: oxygen, carbon dioxide and temperature lamps (0 fine, 1 warning, 2 danger) and the worst room.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct AirView {
    pub status: [u8; 3],
    pub worst: Option<String>,
}

/// Engineering's console: the power solve, the reactor and its loop, the buses and the air.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct EngView {
    /// The reactor's most, MW.
    pub rated_mw: f64,
    pub rx_out_mw: f64,
    pub bat_out_mw: f64,
    pub want_mw: f64,
    /// cruise, combat, silent or emergency, or none after a hand on a fader.
    pub preset: Option<String>,
    pub groups: Vec<GroupView>,
    pub preview: Option<PreviewView>,
    pub reactor: ReactorView,
    /// SCRAM pressed once: the next press confirms.
    pub armed_scram: bool,
    pub cool: CoolView,
    pub buses: BusesView,
    /// The air (none: no atmosphere is simulated, and the panel is drawn unavailable).
    pub air: Option<AirView>,
}

/// A crewed station in single player: its NPC, its mode and whether the NPC is in the seat.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CrewTab {
    /// The station's id.
    pub station: String,
    /// The NPC's name, "Lt. Dal Varen".
    pub op: String,
    /// AUTO: the NPC plays it. MANUAL: it does what the player does on its console.
    pub auto: bool,
    /// The NPC is in the seat (a station works only then).
    pub seated: bool,
}

/// Single player (openspec/changes/single-player design 5): the player's own seat and the crew at the others.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SoloView {
    /// The player's own station's id.
    pub seat: String,
    pub crew: Vec<CrewTab>,
}

/// A laser bank's console (laser-banks 3; the mockup's `laserView`).
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct LaserView {
    /// port or stbd.
    pub bank: String,
    /// The capacitor's share of full.
    pub charge: f64,
    /// Seconds to full at its charging rate.
    pub secs_to_full: f64,
    /// The least share it fires from.
    pub min_charge: f64,
    /// Its arc holds the target, in range.
    pub bearing: bool,
    /// The chance a shot now hits.
    pub hit_chance: f64,
    /// The target's face a beam meets (bow, stern, port, starboard, dorsal or ventral; empty: no target), and that
    /// face's shield and armour as shares of full.
    pub spot_face: String,
    pub spot_shield: f64,
    pub spot_armour: f64,
    /// How well the bank works (0-1).
    pub capability: f64,
}

/// Both banks.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct LasersView {
    pub port: LaserView,
    pub stbd: LaserView,
}

/// Seconds since each bank last fired.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct BeamAges {
    pub port: f64,
    pub stbd: f64,
}

/// A target in the gunner's sight.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GunTarget {
    pub id: String,
    /// fighter or corvette.
    pub kind: String,
    /// Where it is and where to aim (one step of lead for the bolt's speed), ship axes, metres.
    pub rel: [f64; 3],
    pub lead: [f64; 3],
    /// Hull and shield, shares of full.
    pub hull: f64,
    pub shield: f64,
}

/// The gunner's sight (enemy-fighters 3; the mockup's `gunnerView`).
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GunnerView {
    /// The barrel: yaw to starboard and pitch up, degrees, ship axes.
    pub aim: [f64; 2],
    /// The chosen target's index in `targets`.
    pub chosen: Option<usize>,
    /// The turret's capacitor and heat, shares of full.
    pub capacitor: f64,
    pub heat: f64,
    pub trigger: bool,
    /// Hostiles within 8 km, fighters first, then the nearest.
    pub targets: Vec<GunTarget>,
}

/// One row of the OBJECTIVES panel.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ObjectiveRow {
    pub words: String,
    /// progress, done or failed.
    pub state: String,
    /// How far it has gone, 0-1.
    pub k: f64,
    /// Its one number (none: words only).
    pub number: Option<String>,
}

/// The OBJECTIVES panel (objectives-tab 2).
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ObjectivesView {
    pub title: String,
    /// The fight's clock, seconds.
    pub clock_s: f64,
    pub rows: Vec<ObjectiveRow>,
}

/// Everything a console draws: the mockup's `window.consoleState()`, field for field.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ConsoleView {
    /// helm or tactical.
    pub station: String,
    /// The station's word on its chip (HELM).
    pub word: String,
    /// The station's role, for its colour.
    pub role: String,
    /// Who sits there.
    pub op: Option<String>,
    /// Stations with nobody at them, which this seat can swap to.
    pub open: Vec<String>,
    /// Single player's crew (none: a multiplayer seat). `station` is then the console shown, any crewed station's.
    pub solo: Option<SoloView>,
    /// The captain's newest order to this station.
    pub order: Option<String>,
    /// normal, red_alert or emergency: the title band's condition (emergency power over the alert).
    pub alert: String,
    /// The captain's red alert, whatever the power: the look band's red glow.
    #[serde(default)]
    pub red_alert: bool,
    /// The clock in the title band, seconds.
    pub clock_s: f64,
    pub hull_pct: f64,
    /// Each shield face's charge over its capacity.
    pub shield_ratio: [f64; 6],
    /// Power in use, MW (none: no power grid is simulated).
    pub power_mw: Option<f64>,
    pub reactor_up: bool,
    pub speed_mps: f64,
    pub fires: u32,
    pub breaches: u32,
    /// Attitude, ship axes to reference axes, [w, x, y, z].
    pub q: [f64; 4],
    /// Body rates, rad/s, ship axes.
    pub w: [f64; 3],
    /// The rates the helm is asking for, rad/s.
    pub wset: [f64; 3],
    /// Velocity in ship axes, m/s.
    pub v_body: [f64; 3],
    pub speed_set: f64,
    /// Lateral (to starboard) and vertical set points, m/s.
    pub strafe: [f64; 2],
    /// The stick [yaw to starboard, pitch up, roll right], -1..1.
    pub stick: [f64; 3],
    /// The order on the thumbwheels.
    pub oe: Hpr,
    pub order_active: bool,
    /// The active order's slew.
    pub plan: Option<PlanView>,
    /// The thumbwheels' order's slew, before GO.
    pub preview: PlanView,
    /// Where the ship is told to point, if anywhere.
    pub goal: Option<[f64; 4]>,
    /// HOLD, COURSE, CHASE, MATCH or EVADE.
    pub ap: String,
    pub flight: FlightView,
    pub contacts: Vec<ContactView>,
    /// The waypoint, ship axes (none: the mission has none).
    pub waypoint: Option<[f64; 3]>,
    pub target: Option<String>,
    pub tubes_bear: bool,
    /// Our missiles in flight, ship axes.
    pub missiles: Vec<[f64; 3]>,
    pub inbound: Vec<InboundView>,
    pub hits: Vec<HitView>,
    /// Seconds since each face last flashed.
    pub flash_age_s: [f64; 6],
    /// What the viewscreen shows: FWD, AFT, PORT, STBD or TARGET.
    pub feed: String,
    /// The chase toggle: the camera seen from behind the ship (chase-camera 1). The engine's own; the mockup has none.
    #[serde(default)]
    pub chase: bool,
    pub zoom: f64,
    /// ORBIT's goal, [heading, elevation, distance] (degrees, degrees, metres), while it is on (chase-orbit design 5).
    /// The engine's own; the mockup's state has only `orbit_on`.
    #[serde(skip)]
    pub orbit: Option<[f64; 3]>,
    /// ORBIT is on: the camera widget's button lit (the mockup's `G.orbit`).
    #[serde(default)]
    pub orbit_on: bool,
    /// The viewscreen's camera this frame, from the seat's rig (chase-orbit design 7): the feed is drawn from it and
    /// the bracket projected through it. None: a fixture, drawn with the settled camera.
    #[serde(skip)]
    pub cam: Option<rig::Shot>,
    pub nav_km: f64,
    pub tac_km: f64,
    pub sci_km: f64,
    pub scan_cam: ScanCams,
    pub feed_cam: Cam,
    pub turrets: Vec<TurretView>,
    pub tubes: Vec<TubeView>,
    pub tube_load_s: f64,
    pub magazine: u32,
    pub magazine_slots: u32,
    /// The ring round FIRE, 0-1: what the fire rule waits on (the drill: the lock; the mockup: its hit chance).
    pub fire_ring: f64,
    /// The words under FIRE.
    pub fire_note: String,
    /// The ring and its words in the ready colour, not the waiting one.
    pub fire_ok: bool,
    /// The tubes' cone off the bow either way, degrees.
    pub tube_cone_deg: f64,
    /// Shield charge and capacity per face, MJ.
    pub charge: [f64; 6],
    pub cap: [f64; 6],
    /// The favoured face, or -1 balanced.
    pub preset: i32,
    pub hold: Option<HoldView>,
    /// The clock the moving pictures are drawn at (the sweep, the pumps, the pipes' dashes), seconds.
    #[serde(default)]
    pub t: f64,
    /// Science's state (none: the parity fixtures of a console that has none).
    #[serde(default)]
    pub sci: Option<SciView>,
    /// The captain's state.
    #[serde(default)]
    pub captain: Option<CapView>,
    /// Engineering's state (none: no power grid is simulated, and engineering's controls are unavailable).
    #[serde(default)]
    pub eng: Option<EngView>,
    /// Countermeasures left and carried, seconds since the last was thrown, and the nearest inbound missile's seconds
    /// to impact (armour-and-missiles 3: Tactical's DECOY).
    #[serde(default)]
    pub decoys: u32,
    #[serde(default)]
    pub decoys_max: u32,
    #[serde(default = "long_ago")]
    pub decoy_age_s: f64,
    #[serde(default)]
    pub next_inbound_s: Option<f64>,
    /// The laser banks (none: a fixture from before them).
    #[serde(default)]
    pub lasers: Option<LasersView>,
    #[serde(default)]
    pub laser_beam_age_s: Option<BeamAges>,
    /// The gunner's sight.
    #[serde(default)]
    pub gunner: Option<GunnerView>,
    /// The gunner's turret is locked out by heat.
    #[serde(default)]
    pub gun_locked: bool,
    /// OBJECTIVES is open over the panels.
    #[serde(default)]
    pub objectives_open: bool,
    /// Engineering's schematic, open full screen (power-routing 6.3).
    #[serde(default)]
    pub schem: Option<schematic::SchemView>,
    #[serde(default)]
    pub objectives: Option<ObjectivesView>,
    /// LEAVE STATION in the title band, while the player sits (leave-station 4).
    #[serde(default)]
    pub leave: Option<LeaveView>,
    /// START MISSION in the title band, while the round waits and this player may start it (start-mission 4).
    #[serde(default)]
    pub start: Option<StartView>,
}

/// START MISSION (start-mission 4): the start key's name on its keycap, and how many of those aboard are ready.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct StartView {
    /// The start key, as `data/input/bindings.json` names it ("Return").
    pub key: String,
    /// Seated and ready.
    pub ready: u32,
    /// Everyone aboard, people and NPCs.
    pub aboard: u32,
}

/// LEAVE STATION (leave-station 4): the stand key's name on its keycap, and how much of its hold has passed (0..1).
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct LeaveView {
    /// The stand key, as `data/input/bindings.json` names it ("E").
    pub key: String,
    /// The hold so far, 0..1: the keycap's ring.
    pub share: f64,
}

/// A time long past, seconds: nothing thrown yet.
fn long_ago() -> f64 {
    1e9
}

// ------------------------------------------------------------------------------------------------ math

/// The mockup's quaternion [w, x, y, z] as glam's.
pub fn quat(q: [f64; 4]) -> DQuat {
    DQuat::from_xyzw(q[1], q[2], q[3], q[0])
}

/// glam's quaternion as the mockup's [w, x, y, z], with w >= 0 (the form the consoles show).
pub fn quat_arr(q: DQuat) -> [f64; 4] {
    let s = if q.w < 0.0 { -1.0 } else { 1.0 };
    [q.w * s, q.x * s, q.y * s, q.z * s]
}

pub fn v3(a: [f64; 3]) -> DVec3 {
    DVec3::from_array(a)
}

/// Degrees, wrapped to 0..360.
pub fn wrap360(a: f64) -> f64 {
    a.rem_euclid(360.0)
}

/// Degrees, wrapped to -180..180.
pub fn wrap180(a: f64) -> f64 {
    let a = wrap360(a);
    if a > 180.0 {
        a - 360.0
    } else {
        a
    }
}

/// A rotation of `deg` about a unit axis.
fn axis(ax: DVec3, deg: f64) -> DQuat {
    DQuat::from_axis_angle(ax, deg.to_radians())
}

pub use sc_core::combat::attitude::{from_hpr as from_euler, hpr_of as to_euler, whole_deg};

/// Bearing of a ship-axes direction, degrees to starboard of the bow, -180..180.
pub fn brg_of(d: DVec3) -> f64 {
    wrap180((-d.x).atan2(d.z).to_degrees())
}

/// Elevation of a ship-axes direction above the ship's plane, degrees.
pub fn elev_of(d: DVec3) -> f64 {
    d.y.atan2(d.x.hypot(d.z)).to_degrees()
}

/// A camera turned round a model: forward, right and up for yaw and elevation in degrees (the mockup's `camBasis`).
pub struct Basis {
    pub f: DVec3,
    pub r: DVec3,
    pub u: DVec3,
}

pub fn cam_basis(yaw: f64, el: f64) -> Basis {
    let (ce, se) = (el.to_radians().cos(), el.to_radians().sin());
    let d = DVec3::new(yaw.to_radians().sin() * ce, se, -yaw.to_radians().cos() * ce);
    let f = -d;
    let r = f.cross(DVec3::Y).normalize_or_zero();
    Basis { f, r, u: r.cross(f) }
}

/// The mockup's `fmtAng`: a heading as three digits, a signed angle with its sign, and a degree sign.
pub fn fmt_ang(x: f64, signed: bool) -> String {
    if signed {
        let r = x.round();
        format!("{}{}°", if r >= 0.0 { "+" } else { "" }, r as i64)
    } else {
        format!("{:03}°", (wrap360(x).round() as i64) % 360)
    }
}

/// The mockup's `fmtQ`: each component to four places, a space where a minus would be.
pub fn fmt_q(q: [f64; 4]) -> String {
    let s = if q[0] < 0.0 { -1.0 } else { 1.0 };
    q.iter()
        .map(|x| {
            let v = x * s;
            format!("{}{:.4}", if v < 0.0 { "" } else { " " }, v)
        })
        .collect::<Vec<_>>()
        .join("  ")
}

// ------------------------------------------------------------------------------------------------ ship data

/// The shield ellipsoid: semi-axes and centre, ship axes, metres (weapons-and-shields 11).
#[derive(Clone, Copy, Debug)]
pub struct Ellipsoid {
    pub axes: DVec3,
    pub centre: DVec3,
}

/// The Tern's shape for the consoles' 3D pictures: the shield ellipsoid. The hull drawn inside it is the low poly
/// model (`kit::hull_polys`, console-ship-model). Read once from the compiled-in data.
pub struct ShipShape {
    pub shield: Ellipsoid,
}

/// The Tern's shape.
pub fn tern() -> &'static ShipShape {
    static SHAPE: OnceLock<ShipShape> = OnceLock::new();
    SHAPE.get_or_init(|| {
        let data = sc_core::combat::data::DrillData::shipped();
        let e = data.tern_shields.ellipsoid;
        ShipShape { shield: Ellipsoid { axes: DVec3::from_array(e.axes_m), centre: DVec3::from_array(e.centre_m) } }
    })
}

// ------------------------------------------------------------------------------------------------ drawing

/// What one frame's drawing needs besides the view.
pub struct Ctx<'a> {
    pub v: &'a ConsoleView,
    pub hits: &'a mut kit::Hits,
    /// The viewscreen's feed is drawn under the console by the renderer (`Renderer::present_with_ui_at` into
    /// [`viewscreen_px`]), so the console leaves the viewscreen open; false (a parity fixture): the screen is black.
    pub feed: bool,
    /// The pointer, in canvas units, for hover tips.
    pub pointer: Option<Pos2>,
}

/// The colour of a contact's IFF.
pub fn iff_col(iff: &str) -> Color32 {
    let c = &vg::style().c;
    match iff {
        "hostile" => c.hostile.0,
        "neutral" => c.neutral.0,
        "friendly" => c.friendly.0,
        _ => c.unknown.0,
    }
}

/// A shield ratio's colour: green over half, amber over a fifth, red below.
pub fn ratio_col(k: f64) -> Color32 {
    let c = &vg::style().c;
    if k > 0.5 {
        c.ok.0
    } else if k > 0.2 {
        c.warn.0
    } else {
        c.danger.0
    }
}

/// Draw a console on egui's background, the 1280 x 720 canvas on the reference frame (`ui::frame_in`); returns where its
/// controls are and its scale (egui points a layout point). `feed` is the viewscreen's camera, `pointer` the pointer in
/// egui points (for the hover tip).
pub fn paint(ctx: &egui::Context, v: &ConsoleView, feed: bool, pointer: Option<Pos2>) -> (kit::Hits, f32) {
    let (o, s) = crate::ui::frame_in(ctx.content_rect());
    let o = o.to_vec2();
    let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Background, egui::Id::new("console")));
    let cv = Canvas::new(painter, o, s);
    let mut hits = kit::Hits::default();
    draw(&cv, &mut Ctx { v, hits: &mut hits, feed, pointer });
    (hits, s)
}

/// The viewscreen's window in output pixels (x, y from the top left, width, height), for a screen `points` big drawn
/// at `ppp` pixels a point: where the renderer draws the feed under the console.
pub fn viewscreen_px(points: [f32; 2], ppp: f32) -> [i32; 4] {
    let (o, s) = crate::ui::frame_in(Rect::from_min_size(Pos2::ZERO, egui::vec2(points[0], points[1])));
    let [x, y, w, h] = VIEWSCREEN;
    let px = |v: f32| (v * ppp).round() as i32;
    [px(o.x + x * s), px(o.y + (32.0 + y) * s), px(w * s), px(h * s)]
}

/// Draw a whole console (1280 x 720 layout points) on `cv`: the bands, the station's panels and the status strip.
pub fn draw(cv: &Canvas, cx: &mut Ctx) {
    let c = &vg::style().c;
    if cx.feed {
        // The background round the viewscreen's window, which the feed shows through.
        let [x, y, w, h] = VIEWSCREEN;
        let (y, x2, y2) = (y + 32.0, x + w, y + 32.0 + h);
        cv.rect(0.0, 0.0, 1280.0, y, 0.0, c.bg.0, None);
        cv.rect(0.0, y2, 1280.0, 720.0 - y2, 0.0, c.bg.0, None);
        cv.rect(0.0, y, x, h, 0.0, c.bg.0, None);
        cv.rect(x2, y, 1280.0 - x2, h, 0.0, c.bg.0, None);
    } else {
        cv.rect(0.0, 0.0, 1280.0, 720.0, 0.0, c.bg.0, None);
    }
    title_band(&cv.at(0.0, 0.0), cx);
    // The gunner's console is a sight, not panels, with no look band (enemy-fighters 3). OBJECTIVES opens over the
    // four panels, or over the sight (objectives-tab 1).
    let sight = cx.v.station == "gunner";
    if !sight {
        // Nothing in the look band reaches past it, so it takes no clip rectangle (each costs a UI draw call).
        look_band(&cv.at(0.0, 32.0), cx);
    }
    // Engineering's schematic opens over the look band and the panels (power-routing 6.3).
    if let Some(sv) = cx.v.schem.as_ref().filter(|_| cx.v.station == "engineering" && !cx.v.objectives_open) {
        schematic::draw(cv, cx, sv);
        status_strip(&cv.at(0.0, 688.0), cx);
        kit::tooltip(cv, cx);
        return;
    }
    if !cx.v.objectives_open || sight {
        match cx.v.station.as_str() {
            "helm" => helm::draw(cv, cx),
            "engineering" => engineering::draw(cv, cx),
            "science" => science::draw(cv, cx),
            "captain" => captain::draw(cv, cx),
            "laser_port" | "laser_stbd" => laser::draw(cv, cx),
            "gunner" => gunner::draw(cv, cx),
            _ => tactical::draw(cv, cx),
        }
    }
    if cx.v.objectives_open {
        objectives::draw(cv, cx);
    }
    status_strip(&cv.at(0.0, 688.0), cx);
    kit::tooltip(cv, cx);
}

/// The station's accent colour (its role's).
pub fn accent(v: &ConsoleView) -> Color32 {
    vg::style().role.of(&v.role)
}

/// A station's word on its chip and its role (for its colour and icon).
pub fn station_word(id: &str) -> (&'static str, &'static str) {
    match id {
        "helm" => ("HELM", "helm"),
        "tactical" => ("TACTICAL", "tactical"),
        "engineering" => ("ENGINEERING", "engineering"),
        "science" => ("SCIENCE", "science"),
        "laser_port" => ("LASER PORT", "laser"),
        "laser_stbd" => ("LASER STBD", "laser"),
        "gunner" => ("GUNNER", "gunner"),
        _ => ("CAPTAIN", "command"),
    }
}

/// The title band: the station's chip, the open stations to swap to, the captain's order, the condition and the clock.
fn title_band(cv: &Canvas, cx: &mut Ctx) {
    let st = vg::style();
    let (c, role) = (&st.c, &st.role);
    let v = cx.v;
    let (red, emerg) = (v.alert == "red_alert", v.alert == "emergency");
    let bg = if emerg {
        rgba(0x2b1a08, 1.0)
    } else if red {
        rgba(0x3a0c10, 1.0)
    } else {
        rgba(0x0a0f16, 1.0)
    };
    cv.rect(0.0, 0.0, 1280.0, 32.0, 0.0, bg, None);
    let line = if emerg {
        c.emergency.0
    } else if red {
        c.alert.0
    } else {
        c.line.0
    };
    cv.rect(0.0, 31.0, 1280.0, 1.0, 0.0, line, None);
    let mut x = 8.0;
    // The station's own chip; the captain's two pages in its place. In single player the own chip is the player's
    // seat, lit while its console is shown, and a press on it comes back to it.
    let solo = v.solo.as_ref();
    let home = solo.is_none_or(|q| q.seat == v.station);
    let (own_word, own_role) = match solo {
        Some(q) => station_word(&q.seat),
        None => (v.word.as_str(), v.role.as_str()),
    };
    let own_captain = solo.map_or(v.station == "captain", |q| q.seat == "captain");
    let tabs: Vec<(String, String, bool, Option<Act>)> = if own_captain {
        let tab = v.captain.as_ref().map(|q| q.tab.as_str()).unwrap_or("CMD");
        vec![
            ("COMMAND".into(), "command".into(), home && tab == "CMD", Some(Act::Tab("CMD".into()))),
            ("SHIP".into(), "command".into(), home && tab == "SHIP", Some(Act::Tab("SHIP".into()))),
        ]
    } else {
        let back = solo.filter(|_| !home).map(|q| Act::View(q.seat.clone()));
        vec![(own_word.to_owned(), own_role.to_owned(), home, back)]
    };
    for (w, r, on0, act0) in tabs {
        // With OBJECTIVES open, the station's own chip is the way back (objectives-tab 1).
        let on = on0 && !v.objectives_open;
        let act = act0.or_else(|| v.objectives_open.then_some(Act::ObjClose));
        let tw = 34.0 + w.chars().count() as f32 * 9.4;
        let rc = role.of(&r);
        let (fill, stroke) = if on { (alpha(rc, 0x26 as f32 / 255.0), rc) } else { (Color32::TRANSPARENT, c.line2.0) };
        cv.rect(x, 4.0, tw, 24.0, 5.0, fill, Some(Pen::new(1.0, stroke)));
        cv.icon(&r, x + 13.0, 16.0, 14.0, if on { rc } else { c.dim.0 });
        cv.text(x + 25.0, 21.0, 13.0, if on { c.text.0 } else { c.dim.0 }, &w, T::default().ls(1.5).bold());
        if let Some(a) = act {
            let tip = (solo.is_some() && !home && a != Act::ObjClose).then(|| "Your console (Tab)".to_owned());
            cx.hits.add(cv, x, 4.0, tw, 24.0, Some(kit::Hit::Act(a)), tip);
        }
        x += tw + 6.0;
    }
    if let Some(q) = solo {
        // A tab for each crewed station: its icon and its mode, a robot (AUTO) or a hand (MANUAL), dim while its NPC
        // walks to the seat. Press one to show its console; the tab of the console shown is filled.
        for t in &q.crew {
            let (w, r) = station_word(&t.station);
            let (on, rc) = (v.station == t.station, role.of(r));
            let (fill, stroke) =
                if on { (alpha(rc, 0x26 as f32 / 255.0), rc) } else { (Color32::TRANSPARENT, c.line2.0) };
            cv.rect(x, 4.0, 52.0, 24.0, 5.0, fill, Some(Pen::new(1.0, stroke)));
            cv.icon(r, x + 15.0, 16.0, 14.0, if t.seated { rc } else { c.faint.0 });
            let mark = if t.auto { "auto" } else { "hand" };
            cv.icon(mark, x + 37.0, 16.0, 14.0, if t.seated { c.text.0 } else { c.faint.0 });
            let mut name = w.to_lowercase();
            name[..1].make_ascii_uppercase();
            let tip = format!(
                "{name}: {}, on {}{}",
                t.op,
                if t.auto { "AUTO" } else { "MANUAL" },
                if t.seated { "" } else { ", walking to the seat" }
            );
            cx.hits.add(cv, x, 4.0, 52.0, 24.0, Some(kit::Hit::Act(Act::View(t.station.clone()))), Some(tip));
            x += 58.0;
        }
        // The mode switch of the console shown, when it is a crewed station's: its word (the captain's two pages, for
        // the captain's console), then AUTO and MANUAL.
        if let Some(t) = q.crew.iter().find(|t| t.station == v.station) {
            if v.station == "captain" {
                let tab = v.captain.as_ref().map(|q| q.tab.as_str()).unwrap_or("CMD");
                for (w, page) in [("COMMAND", "CMD"), ("SHIP", "SHIP")] {
                    let tw = 34.0 + w.chars().count() as f32 * 9.4;
                    let (on, rc) = (tab == page, role.of("command"));
                    let (fill, stroke) =
                        if on { (alpha(rc, 0x26 as f32 / 255.0), rc) } else { (Color32::TRANSPARENT, c.line2.0) };
                    cv.rect(x, 4.0, tw, 24.0, 5.0, fill, Some(Pen::new(1.0, stroke)));
                    cv.icon("command", x + 13.0, 16.0, 14.0, if on { rc } else { c.dim.0 });
                    cv.text(x + 25.0, 21.0, 13.0, if on { c.text.0 } else { c.dim.0 }, w, T::default().ls(1.5).bold());
                    cx.hits.add(cv, x, 4.0, tw, 24.0, Some(kit::Hit::Act(Act::Tab(page.into()))), None);
                    x += tw + 6.0;
                }
            } else {
                cv.text(x + 4.0, 21.0, 13.0, role.of(&v.role), &v.word, T::default().ls(1.5).bold());
                x += 14.0 + v.word.chars().count() as f32 * 9.4;
            }
            for (auto, w, ic, tw) in [(true, "AUTO", "auto", 76.0), (false, "MANUAL", "hand", 94.0)] {
                let on = t.auto == auto;
                let (fill, stroke) = if on {
                    (alpha(c.text.0, 0x1f as f32 / 255.0), c.text.0)
                } else {
                    (Color32::TRANSPARENT, c.line2.0)
                };
                cv.rect(x, 4.0, tw, 24.0, 5.0, fill, Some(Pen::new(1.0, stroke)));
                let col = if on { c.text.0 } else { c.dim.0 };
                cv.icon(ic, x + 14.0, 16.0, 14.0, col);
                cv.text(x + 26.0, 21.0, 13.0, col, w, T::default().ls(1.5).bold());
                let tip =
                    if auto { format!("{} runs it (M)", t.op) } else { format!("You run it, through {} (M)", t.op) };
                cx.hits.add(cv, x, 4.0, tw, 24.0, Some(kit::Hit::Act(Act::Mode(auto))), Some(tip));
                x += tw + 4.0;
            }
            x += 2.0;
        }
    }
    // OBJECTIVES, after the station's own tabs and before the swap chips, on every station (objectives-tab 1).
    {
        let w = "OBJECTIVES";
        let tw = 34.0 + w.len() as f32 * 9.4;
        let on = v.objectives_open;
        let (fill, stroke) =
            if on { (alpha(c.text.0, 0x1f as f32 / 255.0), c.text.0) } else { (Color32::TRANSPARENT, c.line2.0) };
        cv.rect(x, 4.0, tw, 24.0, 5.0, fill, Some(Pen::new(1.0, stroke)));
        let col = if on { c.text.0 } else { c.dim.0 };
        cv.icon("objectives", x + 13.0, 16.0, 14.0, col);
        cv.text(x + 25.0, 21.0, 13.0, col, w, T::default().ls(1.5).bold());
        cx.hits.add(cv, x, 4.0, tw, 24.0, Some(kit::Hit::Act(Act::Obj)), Some("Objectives".into()));
        x += tw + 6.0;
    }
    // The laser banks took the comms and flight-ops seats (laser-banks 3), so only the open stations are left to swap
    // to; single player crews every station, so it has none.
    let cond = if emerg {
        "EMERGENCY POWER"
    } else if red {
        "RED ALERT"
    } else {
        "NORMAL"
    };
    let cond_x = 1180.0 - cond.len() as f32 * 9.4 - 22.0;
    let leave_w = 34.0 + 13.0 * 9.4;
    let leave_x = cond_x - 8.0 - leave_w;
    let order_end = if v.leave.is_some() { leave_x } else { cond_x };
    // START MISSION's place (start-mission 4), right against LEAVE STATION: the swap chips stop short of it.
    let start_w = 34.0 + 13.0 * 9.4 + 46.0;
    let start_x = order_end - 8.0 - start_w;
    let open: Vec<(String, String, Option<String>)> = if solo.is_some() {
        Vec::new()
    } else {
        v.open
            .iter()
            .map(|id| {
                let (w, r) = station_word(id);
                (w.to_owned(), r.to_owned(), Some(id.clone()))
            })
            .collect()
    };
    for (w, r, id) in open {
        let tw = 44.0 + w.chars().count() as f32 * 8.6;
        if v.start.is_some() && x + tw > start_x - 6.0 {
            break;
        }
        let on = id.is_some();
        cv.rect(x, 4.0, tw, 24.0, 5.0, Color32::TRANSPARENT, Some(Pen::new(1.0, c.line2.0).dash(4.0, 3.0, 0.0)));
        cv.icon(&r, x + 13.0, 16.0, 13.0, if on { role.of(&r) } else { c.faint.0 });
        cv.text(x + 24.0, 21.0, 12.0, if on { c.text.0 } else { c.faint.0 }, &w, T::default().ls(1.5).bold());
        let ac = if on { c.text.0 } else { c.faint.0 };
        let (ax, bx) = (x + tw - 15.0, x + tw - 7.0);
        cv.path(&format!("M{ax} 12h8l-3 -3M{bx} 20h-8l3 3"), Xf::ID, None, Some(Pen::new(1.5, ac)));
        let tip = match &id {
            Some(_) => format!("{} is open: swap to it", w.to_lowercase()),
            None => format!("{} is open (its console is not in this drill yet)", w.to_lowercase()),
        };
        let hit = id.map(Act::Swap);
        cx.hits.add(cv, x, 4.0, tw, 24.0, hit.map(kit::Hit::Act), Some(tip));
        x += tw + 6.0;
    }
    // LEAVE STATION (leave-station 4): left of the condition, the same place at every station; the stand key on its
    // keycap, whose ring fills while the key is held. A click or a tap stands.
    if let Some(l) = &v.leave {
        let held = l.share > 0.0;
        cv.rect(leave_x, 4.0, leave_w, 24.0, 5.0, Color32::TRANSPARENT, Some(Pen::new(1.0, c.line2.0)));
        let (kx, ky) = (leave_x + 15.0, 16.0);
        cv.circle(kx, ky, 8.0, Color32::TRANSPARENT, Some(Pen::new(1.0, c.dim.0)));
        cv.text(kx, ky + 4.0, 11.0, c.text.0, &l.key, T::mid().bold());
        if held {
            let arc = kit::arc_path(kx, ky, 8.0, 0.0, l.share.min(1.0) * 360.0);
            cv.path(&arc, Xf::ID, None, Some(Pen::new(2.0, c.warn.0)));
        }
        let col = if held { c.text.0 } else { c.dim.0 };
        cv.text(leave_x + 29.0, 21.0, 13.0, col, "LEAVE STATION", T::default().ls(1.5).bold());
        let tip = format!("Leave the station (hold {})", l.key);
        cx.hits.add(cv, leave_x, 4.0, leave_w, 24.0, Some(kit::Hit::Act(Act::Leave)), Some(tip));
    }
    // START MISSION (start-mission 4): while the round waits, on the console of a player who may start it, in the
    // order's place right against LEAVE STATION; the start key on its keycap, then how many of those aboard are
    // ready. Not drawn when the station's own tabs leave it no room (its key still works).
    let sx = start_x;
    let start = v.start.as_ref().filter(|_| x <= sx);
    if let Some(sv) = start {
        cv.rect(sx, 4.0, start_w, 24.0, 5.0, alpha(c.ok.0, 0x26 as f32 / 255.0), Some(Pen::new(1.0, c.ok.0)));
        let kw = 10.0 + sv.key.chars().count() as f32 * 6.5;
        let kx = sx + 6.0;
        cv.rect(kx, 8.0, kw, 16.0, 4.0, Color32::TRANSPARENT, Some(Pen::new(1.0, c.dim.0)));
        cv.text(kx + kw / 2.0, 20.0, 10.0, c.text.0, &sv.key, T::mid().bold());
        cv.text(kx + kw + 7.0, 21.0, 13.0, c.text.0, "START MISSION", T::default().ls(1.5).bold());
        let count = format!("{}/{}", sv.ready, sv.aboard);
        cv.text(sx + start_w - 10.0, 21.0, 13.0, c.dim.0, &count, T::end());
        let tip = format!("Start now: {} of {} ready", sv.ready, sv.aboard);
        cx.hits.add(cv, sx, 4.0, start_w, 24.0, Some(kit::Hit::Act(Act::Start)), Some(tip));
    }
    // The newest order to the station shown: at 470, or after the tabs when they reach past it, short of LEAVE STATION
    // (or the condition); left out when the tabs leave it no room, or START MISSION has its place.
    if let Some(o) = v.order.as_ref().filter(|_| start.is_none() && order_end - 8.0 - x.max(470.0) >= 60.0) {
        let ox = x.max(470.0);
        let ow = 300f32.min(order_end - 8.0 - ox);
        cv.rect(ox, 4.0, ow, 24.0, 5.0, alpha(c.warn.0, 0x1f as f32 / 255.0), Some(Pen::new(1.0, c.warn.0)));
        cv.icon("command", ox + 16.0, 16.0, 14.0, c.warn.0);
        cv.text(ox + 30.0, 21.0, 13.0, c.text.0, &format!("CAPTAIN: {o}"), T::default().ls(1.5).bold());
        cv.icon("check", ox + ow - 16.0, 16.0, 14.0, c.warn.0);
        cx.hits.add(cv, ox, 4.0, ow, 24.0, Some(kit::Hit::Act(Act::Ack)), Some("Acknowledge (Y)".into()));
    }
    let cc = if emerg {
        c.emergency.0
    } else if red {
        c.alert.0
    } else {
        c.ok.0
    };
    let cw = cond.len() as f32 * 9.4;
    cv.rect(1180.0 - cw - 22.0, 4.0, cw + 22.0, 24.0, 5.0, alpha(cc, 0x22 as f32 / 255.0), Some(Pen::new(1.0, cc)));
    cv.text(1170.0, 21.0, 13.0, cc, cond, T::end().ls(1.5).bold());
    let t = v.clock_s.max(0.0).floor() as i64;
    cv.text(1268.0, 21.0, 14.0, c.dim.0, &format!("{:02}:{:02}", t / 60, t % 60), T::end());
}

/// The viewscreen's camera, in ship axes: forward, right and up (the mockup's `feedBasis`).
/// Where the bow camera stands, ship frame, metres: ahead of the hull's bow and a little up. The feed is drawn from
/// here and the target bracket projected from here, so they agree however far the zoom goes (target-bracket 1).
pub const BOW_EYE: DVec3 = DVec3::new(0.0, 3.0, 32.0);

/// What a viewscreen is set to: its camera (`combat::FEEDS`), CHASE, zoom (times) and ORBIT's goal, [heading,
/// elevation, distance] (degrees, degrees, metres; none: ORBIT off).
#[derive(Clone, Debug, PartialEq)]
pub struct ScreenSet {
    pub feed: String,
    pub chase: bool,
    pub zoom: f64,
    pub orbit: Option<[f64; 3]>,
}

impl ScreenSet {
    /// The bridge's main viewscreen's settings as the snapshot carries them, the captain's (chase-camera 2,
    /// chase-orbit 6): camera and zoom by their indices, the orbit in its three bytes.
    pub fn main(view: u8, chase: bool, zoom: u8, orbit: Option<[u8; 3]>) -> Self {
        let feed = sc_core::combat::FEEDS.get(usize::from(view)).copied().unwrap_or("FWD");
        Self {
            feed: feed.to_owned(),
            chase,
            zoom: sc_core::combat::ZOOMS.get(usize::from(zoom)).copied().unwrap_or(1.0),
            orbit: orbit.filter(|_| chase).map(|q| {
                let (y, p, d) = chase_cfg().orbit.unquantize(q);
                [y, p, d]
            }),
        }
    }
}

/// What a viewscreen shows (captains-viewscreen 1, chase-orbit 6): while the captain holds the viewscreen (`view`, an
/// index in `FEEDS`), his camera with his CHASE, zoom and orbit (`main`, the snapshot's); otherwise `own`, the
/// console's own (or the captain's, for the bridge's big screen). One rule for the consoles' panels and the main
/// viewscreen.
pub fn on_screen(view: Option<u8>, main: (bool, u8, Option<[u8; 3]>), own: ScreenSet) -> ScreenSet {
    match view.filter(|i| usize::from(*i) < sc_core::combat::FEEDS.len()) {
        Some(i) => ScreenSet::main(i, main.0, main.1, main.2),
        None => own,
    }
}

pub fn feed_basis(v: &ConsoleView) -> Basis {
    let tgt = v.target.as_ref().and_then(|id| v.contacts.iter().find(|c| &c.id == id));
    feed_basis_for(&v.feed, tgt.map(|c| v3(c.rel)))
}

/// Where a camera of the viewscreen (`combat::FEEDS`) looks, in the ship's frame; `target` is the locked contact
/// relative to the Tern in the ship's frame, which TARGET looks at (forward without one). The consoles' feed
/// and the bridge's viewscreens both look this way; with CHASE on, from behind the ship ([`feed_view`]).
pub fn feed_basis_for(feed: &str, target: Option<DVec3>) -> Basis {
    let f = match feed {
        // CHASE is a toggle now (chase-camera 1); a held CHASE from an older client is FWD.
        "FWD" | "CHASE" => DVec3::Z,
        "AFT" => DVec3::NEG_Z,
        "PORT" => DVec3::X,
        "STBD" => DVec3::NEG_X,
        _ => target.map(|t| t.normalize_or_zero()).filter(|t| *t != DVec3::ZERO).unwrap_or(DVec3::Z),
    };
    let mut r = f.cross(DVec3::Y);
    if r.length() < 1e-3 {
        r = DVec3::NEG_X;
    }
    let r = r.normalize();
    Basis { f, r, u: r.cross(f) }
}

/// The chase camera's numbers, set once from `data/space/exterior.json` when the drill starts ([`set_chase`]): the
/// console draws its target bracket through the chase camera's lens.
static CHASE: OnceLock<sc_core::exterior::Chase> = OnceLock::new();

/// Set the chase camera's numbers for the consoles (once; later calls keep the first).
pub fn set_chase(c: &sc_core::exterior::Chase) {
    let _ = CHASE.set(c.clone());
}

/// The chase camera's numbers: the drill's, or the shipped file's for a console drawn with no drill (a fixture, a test).
pub fn chase_cfg() -> &'static sc_core::exterior::Chase {
    CHASE.get_or_init(|| sc_core::exterior::ExteriorData::shipped().viewscreen.chase)
}

/// The chase camera's vertical field of view, radians, at `zoom` (the feed is 614 x 194, as every camera's).
pub fn chase_vfov(c: &sc_core::exterior::Chase, zoom: f64) -> f64 {
    let across = (c.fov_across_deg / 2.0).to_radians().tan() / zoom.max(1.0);
    2.0 * (across * 97.0 / 307.0).atan()
}

/// The chase camera (openspec/changes/chase-camera design 1): a viewscreen camera looking along `look` (ship frame),
/// seen from behind the ship. It stands `distance_m` (the chase camera's, or an orbit's: chase-orbit design 5) back
/// from the ship's centre along that line, raised square to its
/// look until the ship's centre sits `ship_low` of the half height below the middle of a picture `half_vfov` radians
/// high, the ship in the middle across; it looks along `look`, or at `target` (TARGET), so the target is in the middle.
/// Returns where it stands relative to the ship's centre and the way it looks, both in the ship's frame.
pub fn chase_camera(
    look: DVec3,
    target: Option<DVec3>,
    c: &sc_core::exterior::Chase,
    half_vfov: f64,
    distance_m: f64,
) -> (DVec3, Basis) {
    let line = look.normalize_or_zero();
    let line = if line == DVec3::ZERO { DVec3::Z } else { line };
    let look_from = |eye: DVec3| match target {
        Some(t) if (t - eye).length() > 1.0 => (t - eye).normalize(),
        _ => line,
    };
    let frame = |f: DVec3| {
        let mut r = f.cross(DVec3::Y);
        if r.length() < 1e-3 {
            r = DVec3::NEG_X;
        }
        let r = r.normalize();
        Basis { f, r, u: r.cross(f) }
    };
    // Raised until the ship sits low in the picture it finally takes. Looking at a near target turns the camera as it
    // rises, so the lift is settled on the last aim rather than the first (a few steps: each turn is small).
    let low = c.ship_low * half_vfov.tan();
    let mut eye = -line * distance_m;
    for _ in 0..12 {
        let b = frame(look_from(eye));
        let depth = (-eye).dot(b.f).max(1.0);
        let off = (-eye).dot(b.u) + depth * low;
        if off.abs() < 1e-4 {
            break;
        }
        eye += b.u * off;
    }
    (eye, frame(look_from(eye)))
}

/// A viewscreen's camera, the one sum the consoles and the bridge's main viewscreen share: camera `feed`
/// (`combat::FEEDS`), chasing or not, with `target` the locked contact relative to the Tern (ship frame), at `zoom`.
/// Returns where it stands relative to the ship's centre (None: on the bow), the way it looks (ship frame) and its
/// vertical field of view, radians.
pub fn feed_view(
    feed: &str,
    chase: bool,
    target: Option<DVec3>,
    c: &sc_core::exterior::Chase,
    zoom: f64,
) -> (Option<DVec3>, Basis, f64) {
    if chase {
        // A zoom is a lens (viewscreen-zoom 2; owner, 2026-10-14: "4x looks like 1x is what I meant by weird"): the
        // chase camera stands and looks where it does at 1x, and its picture is magnified, so the target grows and the
        // Tern, low in the 1x picture, leaves by the bottom edge.
        let b = feed_basis_for(feed, target);
        let (eye, b) =
            chase_camera(b.f, target.filter(|_| feed == "TARGET"), c, chase_vfov(c, 1.0) / 2.0, c.distance_m);
        return (Some(eye), b, chase_vfov(c, zoom));
    }
    // From the bow camera: TARGET looks from where it stands at the target.
    let b = feed_basis_for(feed, target.map(|t| t - BOW_EYE));
    let foc = 307.0 / 35f64.to_radians().tan() * zoom;
    (None, b, 2.0 * (97.0 / foc).atan())
}

/// What a console's viewscreen asks of its camera rig: its camera, CHASE, zoom, the locked target and ORBIT's goal.
pub fn feed_want(v: &ConsoleView) -> rig::Want {
    let target = v.target.as_ref().and_then(|id| v.contacts.iter().find(|k| &k.id == id)).map(|k| v3(k.rel));
    rig::Want { feed: v.feed.clone(), chase: v.chase, zoom: v.zoom, target, orbit: v.orbit.filter(|_| v.chase) }
}

/// The camera a console's viewscreen shows this frame: its rig's (`ConsoleView::cam`), or with no rig (a fixture) the
/// settled one. The 3D feed is drawn from it and the bracket projected through it (chase-orbit design 7).
pub fn feed_shot(v: &ConsoleView) -> rig::Shot {
    v.cam.unwrap_or_else(|| rig::Shot::settled(&feed_want(v), chase_cfg()))
}

/// The viewscreen's frame on screen (look band coordinates): where the 3D feed is drawn.
pub const VIEWSCREEN: [f32; 4] = [333.0, 21.0, 614.0, 194.0];

/// The look band: the room from the seat, here the viewscreen and its frame, the camera widget and the navball.
fn look_band(cv: &Canvas, cx: &mut Ctx) {
    let c = &vg::style().c;
    let v = cx.v;
    let [sx, sy, sw, sh] = VIEWSCREEN;
    let (g0, g1) = (rgba(0x0b1018, 1.0), rgba(0x05070b, 1.0));
    if cx.feed {
        // The band's gradient round the viewscreen's window.
        let at = |y: f32| lerp_col(g0, g1, y / 240.0);
        cv.vgrad(0.0, 0.0, 1280.0, sy, &[(0.0, g0), (1.0, at(sy))]);
        cv.vgrad(0.0, sy + sh, 1280.0, 240.0 - sy - sh, &[(0.0, at(sy + sh)), (1.0, g1)]);
        cv.vgrad(0.0, sy, sx, sh, &[(0.0, at(sy)), (1.0, at(sy + sh))]);
        cv.vgrad(sx + sw, sy, 1280.0 - sx - sw, sh, &[(0.0, at(sy)), (1.0, at(sy + sh))]);
    } else {
        cv.vgrad(0.0, 0.0, 1280.0, 240.0, &[(0.0, g0), (1.0, g1)]);
    }
    let side = Pen::new(1.0, rgba(0x18212d, 1.0));
    cv.path(
        "M0 240V150l180-30h120l40 120zM1280 240V150l-180-30H980l-40 120z",
        Xf::ID,
        Some(rgba(0x0d131c, 1.0)),
        Some(side),
    );
    let fill = if cx.feed { Color32::TRANSPARENT } else { rgba(0x020306, 1.0) };
    cv.rect(330.0, 18.0, 620.0, 200.0, 6.0, fill, Some(Pen::new(3.0, rgba(0x2a3442, 1.0))));
    // The target's bracket on the feed, where the feed's camera sees it: through the one camera the feed is drawn
    // from, the rig's smoothed one (chase-orbit design 7), or with no rig (a fixture) its settled pose.
    let scr = cv.at(sx, sy).clip(0.0, 0.0, sw, sh);
    let shot = feed_shot(v);
    let (eye, b, foc) = (shot.eye, shot.basis(), 97.0 / (shot.vfov / 2.0).tan());
    for k in &v.contacts {
        if k.iff == "unknown" || Some(&k.id) != v.target.as_ref() {
            continue;
        }
        let Some(r) = bracket_rect(k, eye, &b, foc, v.zoom) else { continue };
        let col = match k.iff.as_str() {
            "friendly" => c.friendly.0,
            "neutral" => c.neutral.0,
            _ => c.hostile.0,
        };
        draw_bracket(&scr, r, k, col, (v3(k.rel) - eye).length(), c.line.0);
    }
    // ORBIT on: the viewscreen itself is the handle that turns the camera round the ship (chase-orbit design 5).
    if v.orbit_on {
        let tip = "Drag to orbit the ship; scroll or pinch to move in or out";
        cx.hits.add(cv, sx, sy, sw, sh, Some(kit::Hit::Drag(kit::Drag::Orbit)), Some(tip.into()));
    }
    cv.text(342.0, 38.0, 11.0, c.dim.0, &format!("VIEWSCREEN · {}", v.feed), T::default().ls(2.0));
    kit::feed_widget(cv, cx, 196.0, 10.0);
    kit::navball(cv, cx, 1046.0, 116.0, 76.0);
    if v.red_alert {
        kit::vignette(cv, 1280.0, 240.0, c.alert.0, 0.35);
    }
}

/// A colour `k` of the way from `a` to `b`, channel by channel (as an SVG gradient between two stops).
fn lerp_col(a: Color32, b: Color32, k: f32) -> Color32 {
    let k = k.clamp(0.0, 1.0);
    let ch = |x: u8, y: u8| (f32::from(x) + (f32::from(y) - f32::from(x)) * k).round() as u8;
    Color32::from_rgb(ch(a.r(), b.r()), ch(a.g(), b.g()), ch(a.b(), b.b()))
}

/// The viewscreen's middle, in its 614 x 194 picture.
const SCREEN_MID: [f64; 2] = [307.0, 97.0];

/// The target bracket's rectangle on the viewscreen, [left, top, right, bottom] (target-bracket 1): the contact's hull
/// box turned as it lies and projected through the camera at `eye` with axes `b` and focal length `foc` pixels, padded
/// 4 px and at least 16 px a side about its middle. None when a corner is behind the camera. A class with no box is
/// framed by the old range rule's square (60 px at 1 km, 5 to 70 px), times `zoom`.
pub fn bracket_rect(k: &ContactView, eye: DVec3, b: &Basis, foc: f64, zoom: f64) -> Option<[f64; 4]> {
    let at = |p: DVec3| {
        let d = p - eye;
        let cz = d.dot(b.f);
        (cz > 0.01).then(|| [SCREEN_MID[0] + d.dot(b.r) / cz * foc, SCREEN_MID[1] - d.dot(b.u) / cz * foc])
    };
    let rel = v3(k.rel);
    let (mut x0, mut y0, mut x1, mut y1) = (f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
    match k.box_m {
        Some([lo, hi]) => {
            let fwd = v3(k.dir).normalize_or(DVec3::Z);
            let up = v3(k.up).normalize_or(DVec3::Y);
            let right = up.cross(fwd).normalize_or(DVec3::X);
            for i in 0..8 {
                let pick = |a: usize| if i >> a & 1 == 0 { lo[a] } else { hi[a] };
                let p = at(rel + right * pick(0) + up * pick(1) + fwd * pick(2))?;
                (x0, y0, x1, y1) = (x0.min(p[0]), y0.min(p[1]), x1.max(p[0]), y1.max(p[1]));
            }
        }
        None => {
            let p = at(rel)?;
            let half = (60.0 / ((rel - eye).length() / 1000.0)).clamp(5.0, 70.0) * zoom * 0.9;
            (x0, y0, x1, y1) = (p[0] - half, p[1] - half, p[0] + half, p[1] + half);
        }
    }
    let pad = |a: f64, z: f64| {
        let (m, h) = ((a + z) / 2.0, ((z - a) / 2.0 + 4.0).max(8.0));
        (m - h, m + h)
    };
    let ((x0, x1), (y0, y1)) = (pad(x0, x1), pad(y0, y1));
    Some([x0, y0, x1, y1])
}

/// A range as the bracket reads it: whole metres under a kilometre, kilometres to one place from there.
pub fn range_text(m: f64) -> String {
    if m < 1000.0 {
        format!("{:.0} m", m)
    } else {
        format!("{:.1} km", m / 1000.0)
    }
}

/// The target bracket (target-bracket 1-3): four L corners round `r` in the IFF colour, the IFF word and the hull bar
/// above it, the contact's id and range below, the texts kept inside the viewscreen.
fn draw_bracket(cv: &Canvas, r: [f64; 4], k: &ContactView, col: Color32, range_m: f64, track: Color32) {
    let [x0, y0, x1, y1] = r.map(|a| a as f32);
    let arm = ((x1 - x0).min(y1 - y0) / 4.0).clamp(5.0, 14.0);
    let pen = Pen::new(1.5, col);
    cv.polyline(&[[x0, y0 + arm], [x0, y0], [x0 + arm, y0]], pen);
    cv.polyline(&[[x1 - arm, y0], [x1, y0], [x1, y0 + arm]], pen);
    cv.polyline(&[[x1, y1 - arm], [x1, y1], [x1 - arm, y1]], pen);
    cv.polyline(&[[x0 + arm, y1], [x0, y1], [x0, y1 - arm]], pen);
    let tx = x0.clamp(2.0, 614.0 - 72.0);
    let top = (y0 - 8.0).max(12.0);
    cv.text(tx, top - 4.0, 10.0, col, &k.iff.to_uppercase(), T::default().bold().ls(1.0));
    if k.scan >= 0.3 {
        let w = x1 - x0;
        cv.rect(x0, y0 - 6.0, w, 3.0, 0.0, track, None);
        cv.rect(x0, y0 - 6.0, w * (k.hull / 100.0).clamp(0.0, 1.0) as f32, 3.0, 0.0, col, None);
    }
    let below = (y1 + 13.0).min(192.0);
    cv.text(tx, below, 10.0, col, &format!("{}  {}", k.id, range_text(range_m)), T::default());
}

/// The status strip: the same on every console, pictures and numbers only.
fn status_strip(cv: &Canvas, cx: &mut Ctx) {
    let st = vg::style();
    let c = &st.c;
    let v = cx.v;
    cv.rect(0.0, 0.0, 1280.0, 32.0, 0.0, rgba(0x0a0f16, 1.0), None);
    cv.rect(0.0, 0.0, 1280.0, 1.0, 0.0, c.line.0, None);
    let hp = v.hull_pct;
    let hc = if hp > 75.0 {
        c.ok.0
    } else if hp > 40.0 {
        c.warn.0
    } else {
        c.danger.0
    };
    cv.icon("hull", 22.0, 16.0, 16.0, hc);
    kit::bar(cv, 36.0, 11.0, 90.0, 10.0, hp / 100.0, hc, false);
    cv.text(132.0, 21.0, 14.0, c.text.0, &format!("{:.0}%", hp), T::default());
    cx.hits.add(cv, 14.0, 4.0, 160.0, 24.0, None, Some("Hull".into()));
    cv.icon("shields", 196.0, 16.0, 16.0, c.text.0);
    for f in 0..6 {
        let k = v.shield_ratio[f];
        kit::bar(cv, 212.0 + f as f32 * 16.0, 8.0, 11.0, 16.0, k, ratio_col(k), true);
    }
    cx.hits.add(
        cv,
        188.0,
        4.0,
        120.0,
        24.0,
        None,
        Some("Shields: bow, stern, port, starboard, dorsal, ventral".into()),
    );
    match v.power_mw {
        Some(pw) => {
            cv.icon("bolt", 330.0, 16.0, 16.0, if v.reactor_up { c.text.0 } else { c.emergency.0 });
            cv.text(344.0, 21.0, 14.0, c.text.0, &format!("{pw:.1} MW"), T::default());
            // The mockup's tip: the battery's share when it gives any.
            let tip = match v.eng.as_ref().filter(|e| e.bat_out_mw > 0.05) {
                Some(e) => format!("Reactor {:.1} MW, battery {:.1} MW", e.rx_out_mw, e.bat_out_mw),
                None => "Power in use".to_owned(),
            };
            cx.hits.add(cv, 322.0, 4.0, 90.0, 24.0, None, Some(tip));
        }
        None => {
            // Unavailable: the drill simulates no power grid yet (CLAUDE.md 10, a control's unavailable state).
            cv.icon("bolt", 330.0, 16.0, 16.0, c.faint.0);
            cv.text(344.0, 21.0, 14.0, c.faint.0, "-- MW", T::default());
            cx.hits.add(cv, 322.0, 4.0, 90.0, 24.0, None, Some("Power: not simulated in this drill".into()));
        }
    }
    cv.icon("speed", 432.0, 16.0, 16.0, c.text.0);
    cv.text(446.0, 21.0, 14.0, c.text.0, &format!("{:.0} m/s", v.speed_mps), T::default());
    cx.hits.add(cv, 424.0, 4.0, 100.0, 24.0, None, Some("Speed".into()));
    let (nf, nb) = (v.fires, v.breaches);
    let fc = if nf > 0 { c.danger.0 } else { c.faint.0 };
    cv.icon("flame", 540.0, 16.0, 16.0, fc);
    cv.text(554.0, 21.0, 14.0, fc, &nf.to_string(), T::default());
    cx.hits.add(cv, 532.0, 4.0, 40.0, 24.0, None, Some("Fires".into()));
    let bc = if nb > 0 { c.danger.0 } else { c.faint.0 };
    cv.icon("breach", 590.0, 16.0, 16.0, bc);
    cv.text(604.0, 21.0, 14.0, bc, &nb.to_string(), T::default());
    cx.hits.add(cv, 582.0, 4.0, 40.0, 24.0, None, Some("Compartments open to space".into()));
    cv.icon("person", 1090.0, 16.0, 16.0, accent(v));
    cv.text(1106.0, 21.0, 14.0, c.text.0, &format!("{} · {}", v.op.as_deref().unwrap_or("You"), v.word), T::default());
}

/// The grid cell of a panel: column, row, width and height in cells (the mockup's `grid`).
pub fn grid(col: f32, row: f32, w: f32, h: f32) -> [f32; 4] {
    [20.0 + 104.0 * col, 280.0 + 102.0 * row, 104.0 * w - 8.0, 102.0 * h - 8.0]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chase_cfg() -> sc_core::exterior::Chase {
        sc_core::exterior::ExteriorData::shipped().viewscreen.chase
    }

    /// Where `p` (ship frame) lands on the picture of a camera at `eye` looking along `b`: right and up as a share of
    /// the half height, and its depth.
    fn on_screen(p: DVec3, eye: DVec3, b: &Basis, half: f64) -> (f64, f64, f64) {
        let d = p - eye;
        let z = d.dot(b.f);
        (d.dot(b.r) / z / half.tan(), d.dot(b.u) / z / half.tan(), z)
    }

    #[test]
    fn chase_puts_the_ship_low_and_in_the_middle_across_on_every_camera() {
        let target = DVec3::new(900.0, 300.0, 2500.0);
        for feed in ["TARGET", "FWD", "PORT", "AFT", "STBD"] {
            let (eye, b, vfov) = feed_view(feed, true, Some(target), &chase_cfg(), 1.0);
            let eye = eye.expect("chase stands off the ship");
            let (sx, sy, sz) = on_screen(DVec3::ZERO, eye, &b, vfov / 2.0);
            assert!(sz > 100.0, "{feed}: the ship is ahead of the camera, {sz} m");
            assert!(sx.abs() < 0.05, "{feed}: the ship is in the middle across, at {sx}");
            assert!((sy + 0.45).abs() < 0.03, "{feed}: the ship's centre sits low, at {sy}");
            let plain = feed_basis_for(feed, Some(target)).f;
            if feed != "TARGET" {
                assert!(b.f.dot(plain) > 0.999, "{feed}: it looks the camera's way");
            }
        }
    }

    #[test]
    fn chase_on_target_keeps_the_target_in_the_middle() {
        let target = DVec3::new(900.0, 300.0, 2500.0);
        let (eye, b, vfov) = feed_view("TARGET", true, Some(target), &chase_cfg(), 1.0);
        let (tx, ty, _) = on_screen(target, eye.unwrap(), &b, vfov / 2.0);
        assert!(tx.abs() < 1e-6 && ty.abs() < 1e-6, "the target is in the middle, at {tx} {ty}");
    }

    #[test]
    fn chase_on_a_near_target_still_puts_the_ship_low() {
        // A contact closer than the chase distance turns the camera hardest as it rises: the ship must stay low.
        for target in [DVec3::new(250.0, 0.0, 120.0), DVec3::new(-90.0, 60.0, 200.0), DVec3::new(0.0, -40.0, -300.0)] {
            let c = chase_cfg();
            let (eye, b, vfov) = feed_view("TARGET", true, Some(target), &c, 1.0);
            let eye = eye.unwrap();
            let (tx, ty, _) = on_screen(target, eye, &b, vfov / 2.0);
            assert!(tx.abs() < 1e-6 && ty.abs() < 1e-6, "the target is in the middle, at {tx} {ty}");
            let (sx, sy, _) = on_screen(DVec3::ZERO, eye, &b, vfov / 2.0);
            assert!(sx.abs() < 1e-3, "the ship is in the middle across, at {sx}");
            assert!((sy + c.ship_low).abs() < 1e-3, "the ship sits {} below the middle, at {sy}", c.ship_low);
        }
    }

    fn hound_contact(rel: [f64; 3]) -> ContactView {
        ContactView {
            id: "T1".into(),
            cls: "corvette".into(),
            iff: "hostile".into(),
            rel,
            dir: [0.6, 0.0, 0.8],
            hull: 80.0,
            scan: 1.0,
            up: [0.0, 1.0, 0.0],
            box_m: crate::ships3d::class_box("corvette").map(|(lo, hi)| [lo.map(f64::from), hi.map(f64::from)]),
        }
    }

    /// Every corner of the contact's box, projected as the bracket projects them.
    fn box_on_screen(k: &ContactView, eye: DVec3, b: &Basis, foc: f64) -> Vec<[f64; 2]> {
        let [lo, hi] = k.box_m.unwrap();
        let (fwd, up) = (v3(k.dir).normalize(), v3(k.up).normalize());
        let right = up.cross(fwd).normalize();
        (0..8)
            .map(|i| {
                let pick = |a: usize| if i >> a & 1 == 0 { lo[a] } else { hi[a] };
                let d = v3(k.rel) + right * pick(0) + up * pick(1) + fwd * pick(2) - eye;
                let cz = d.dot(b.f);
                [SCREEN_MID[0] + d.dot(b.r) / cz * foc, SCREEN_MID[1] - d.dot(b.u) / cz * foc]
            })
            .collect()
    }

    #[test]
    fn the_bracket_closes_round_the_hound_near_and_far() {
        let bow = Basis { f: DVec3::Z, r: DVec3::NEG_X, u: DVec3::Y };
        let foc = 307.0 / 35f64.to_radians().tan();
        for rel in [[30.0, 10.0, 300.0], [-200.0, 40.0, 3000.0]] {
            let k = hound_contact(rel);
            let r = bracket_rect(&k, DVec3::ZERO, &bow, foc, 1.0).expect("ahead of the camera");
            let pts = box_on_screen(&k, DVec3::ZERO, &bow, foc);
            let (x0, x1) = pts.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |a, p| (a.0.min(p[0]), a.1.max(p[0])));
            let (y0, y1) = pts.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |a, p| (a.0.min(p[1]), a.1.max(p[1])));
            assert!(r[0] <= x0 && r[1] <= y0 && r[2] >= x1 && r[3] >= y1, "the corners enclose the hull at {rel:?}");
            assert!(r[2] - r[0] >= 16.0 - 1e-9 && r[3] - r[1] >= 16.0 - 1e-9, "at least 16 px a side");
            if x1 - x0 > 8.0 {
                assert!((r[0] - (x0 - 4.0)).abs() < 1e-6 && (r[2] - (x1 + 4.0)).abs() < 1e-6, "4 px to spare across");
            }
        }
    }

    #[test]
    fn the_bracket_follows_the_chase_camera() {
        let target = DVec3::new(250.0, 0.0, 600.0);
        let k = hound_contact(target.to_array());
        let (eye, b, vfov) = feed_view("TARGET", true, Some(target), &chase_cfg(), 1.0);
        let (eye, foc) = (eye.unwrap(), 97.0 / (vfov / 2.0).tan());
        let r = bracket_rect(&k, eye, &b, foc, 1.0).unwrap();
        let mid = [(r[0] + r[2]) / 2.0, (r[1] + r[3]) / 2.0];
        assert!(
            (mid[0] - SCREEN_MID[0]).abs() < 12.0 && (mid[1] - SCREEN_MID[1]).abs() < 12.0,
            "on the Hound in the middle: {mid:?}"
        );
    }

    #[test]
    fn the_bracket_stays_on_the_hound_at_20x_from_the_bow() {
        // The feed is drawn from the bow camera: the bracket must be projected from there too, or the zoom magnifies
        // the 32 m between the bow and the ship's centre into a bracket beside the ship.
        let target = DVec3::new(-700.0, 120.0, 1300.0);
        let k = hound_contact(target.to_array());
        let (eye, b, vfov) = feed_view("TARGET", false, Some(target), &chase_cfg(), 20.0);
        assert!(eye.is_none(), "on the bow");
        let foc = 97.0 / (vfov / 2.0).tan();
        let r = bracket_rect(&k, BOW_EYE, &b, foc, 20.0).unwrap();
        let mid = [(r[0] + r[2]) / 2.0, (r[1] + r[3]) / 2.0];
        assert!(
            (mid[0] - SCREEN_MID[0]).abs() < 12.0 && (mid[1] - SCREEN_MID[1]).abs() < 12.0,
            "on the Hound: {mid:?}"
        );
    }

    #[test]
    fn zoomed_chase_magnifies_from_where_the_camera_stands() {
        let target = DVec3::new(250.0, 0.0, 1600.0);
        let (e1, b1, v1) = feed_view("TARGET", true, Some(target), &chase_cfg(), 1.0);
        let (e4, b4, v4) = feed_view("TARGET", true, Some(target), &chase_cfg(), 4.0);
        assert_eq!(e1, e4, "the camera does not move");
        assert!((b1.f - b4.f).length() < 1e-12, "nor turn");
        let ratio = (v1 / 2.0).tan() / (v4 / 2.0).tan();
        assert!((ratio - 4.0).abs() < 1e-9, "4x magnifies four times: {ratio}");
        let (_, sy, _) = on_screen(DVec3::ZERO, e4.unwrap(), &b4, v4 / 2.0);
        assert!(sy < -1.0, "the Tern, low at 1x, is below the 4x picture: {sy}");
    }

    #[test]
    fn a_target_behind_the_camera_has_no_bracket() {
        let bow = Basis { f: DVec3::Z, r: DVec3::NEG_X, u: DVec3::Y };
        assert!(bracket_rect(&hound_contact([0.0, 0.0, -500.0]), DVec3::ZERO, &bow, 400.0, 1.0).is_none());
    }

    #[test]
    fn the_bracket_reads_its_range_in_metres_then_kilometres() {
        assert_eq!(range_text(640.4), "640 m");
        assert_eq!(range_text(3240.0), "3.2 km");
    }

    #[test]
    fn chase_off_is_the_bow_camera() {
        let (eye, b, _) = feed_view("PORT", false, None, &chase_cfg(), 1.0);
        assert!(eye.is_none(), "on the bow");
        assert_eq!(b.f, DVec3::X);
    }

    #[test]
    fn every_viewscreen_camera_looks_its_own_way() {
        let t = DVec3::new(300.0, 0.0, 400.0);
        assert!((feed_basis_for("TARGET", Some(t)).f - t.normalize()).length() < 1e-9, "TARGET looks at the target");
        assert_eq!(feed_basis_for("TARGET", None).f, DVec3::Z, "TARGET with nothing locked looks forward");
        assert_eq!(feed_basis_for("AFT", Some(t)).f, DVec3::NEG_Z);
        assert_eq!(feed_basis_for("PORT", None).f, DVec3::X);
        assert_eq!(feed_basis_for("STBD", None).f, DVec3::NEG_X);
        let b = feed_basis_for("FWD", None);
        assert!(b.u.y > 0.99, "the picture is upright");
    }

    #[test]
    fn euler_round_trips_through_the_quaternion() {
        for (h, p, r) in [(20.0, 0.0, 0.0), (45.0, 10.0, -30.0), (300.0, -60.0, 170.0)] {
            let (h2, p2, r2, g) = to_euler(from_euler(h, p, r));
            assert!(!g);
            assert!((wrap180(h2 - h)).abs() < 1e-6 && (p2 - p).abs() < 1e-6 && (wrap180(r2 - r)).abs() < 1e-6);
        }
    }

    #[test]
    fn the_mockups_starting_attitude_reads_as_its_quaternion() {
        // consoles.html starts the ship at heading 020: [0.9848, 0, -0.1736, 0].
        let q = quat_arr(from_euler(20.0, 0.0, 0.0));
        assert!((q[0] - 0.984_807_753).abs() < 1e-9 && (q[2] + 0.173_648_178).abs() < 1e-9);
        assert_eq!(fmt_q(q), " 0.9848   0.0000  -0.1736   0.0000");
    }

    #[test]
    fn every_mockup_fixture_parses() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/screenshots/parity");
        let Ok(rd) = std::fs::read_dir(&dir) else { return };
        for e in rd.flatten() {
            let p = e.path();
            // A state is a JSON beside its mockup picture; the comparison's report is not one.
            let shot = p.file_stem().and_then(|s| s.to_str()).unwrap_or("").to_owned();
            if p.extension().is_some_and(|x| x == "json") && dir.join(format!("{shot}-mockup.png")).exists() {
                let t = std::fs::read_to_string(&p).unwrap();
                serde_json::from_str::<ConsoleView>(&t).unwrap_or_else(|err| panic!("{}: {err}", p.display()));
            }
        }
    }
}
