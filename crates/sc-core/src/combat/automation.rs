//! Station automation (openspec/changes/coop-drill design 3; bridge-stations section 3): each station's decisions
//! as the same console commands a player sends.
//!
//! It lives in the core because the server runs it for an empty seat and a bot client runs it for the seat it
//! holds, and the two must be one implementation that differs only by profile (CLAUDE.md 6.1). It reads a
//! [`Picture`], which the server builds from the drill and a client builds from a snapshot, so it never needs more
//! than a console shows.

use super::data::{
    CaptainOrders, CaptainProfile, DrillData, EngineeringProfile, FlightBlock, HelmProfile, TacticalProfile,
};
use super::{
    off_bow_deg, steer_toward, Bridge, Command, Drill, HelmMode, PowerCmd, Station, TubeState, TurretMode, ENEMY_ID,
};
use glam::{DQuat, DVec3};

/// The Tern as a console sees her.
#[derive(Clone, Debug, PartialEq)]
pub struct Own {
    /// Position, system frame.
    pub pos: DVec3,
    /// Attitude.
    pub rot: DQuat,
    /// Velocity.
    pub vel: DVec3,
    /// Body rates, rad/s.
    pub rates: DVec3,
    /// The speed set point, m/s.
    pub speed_set: f64,
    /// The helm's mode.
    pub helm_mode: HelmMode,
    /// The shield preset.
    pub preset: u8,
    /// The designated target.
    pub lock_target: Option<u16>,
    /// Whether the lock is complete.
    pub locked: bool,
    /// Each turret's mode.
    pub turret_modes: Vec<TurretMode>,
    /// Each tube's state.
    pub tubes: Vec<TubeState>,
    /// Missiles aboard, not counting the tubes.
    pub magazine: u32,
    /// Hull left, MJ.
    pub hull_mj: f64,
}

/// A hostile as a console sees it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hostile {
    /// Its id.
    pub id: u16,
    /// Position, system frame.
    pub pos: DVec3,
    /// Velocity.
    pub vel: DVec3,
}

/// What automation decides from.
#[derive(Clone, Debug, PartialEq)]
pub struct Picture {
    /// The Tern.
    pub own: Own,
    /// The hostile, when one is in the fight.
    pub hostile: Option<Hostile>,
}

impl Picture {
    /// The picture from the server's drill.
    pub fn from_drill(d: &Drill) -> Self {
        let t = &d.ships[0];
        let e = &d.ships[1];
        Self {
            own: Own {
                pos: t.pos,
                rot: t.rot,
                vel: t.vel,
                rates: t.rates,
                speed_set: t.speed_set,
                helm_mode: t.helm_mode,
                preset: t.preset,
                lock_target: t.lock_target,
                locked: t.lock_target.is_some() && t.lock_s >= d.data.tern_combat.lock.time_s,
                turret_modes: t.turrets.iter().map(|x| x.mode).collect(),
                tubes: t.tubes.iter().map(|x| x.state).collect(),
                magazine: t.magazine,
                hull_mj: t.hull,
            },
            hostile: (e.active && e.alive).then_some(Hostile { id: ENEMY_ID, pos: e.pos, vel: e.vel }),
        }
    }
}

/// When a helm decides next.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct HelmMemory {
    next_s: f64,
}

/// When a tactical officer decides next.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TacticalMemory {
    next_s: f64,
}

/// Both stations' memory.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Memory {
    /// Helm.
    pub helm: HelmMemory,
    /// Tactical.
    pub tactical: TacticalMemory,
    /// The captain.
    pub captain: CaptainMemory,
    /// Engineering.
    pub engineering: EngineeringMemory,
}

/// What Engineering's automation last answered: the condition it set the grid for, and since when the bridge has
/// been at another.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct EngineeringMemory {
    /// The condition its preset is for (red alert: true), none before its first.
    pub alert: Option<bool>,
    /// When the condition last differed from it, seconds into Engage.
    pub since_s: Option<f64>,
    /// Since when a spare run would bring power back to a dark room (power-routing 3).
    pub reroute_since: Option<f64>,
}

/// Engineering's automation (bridge-stations 3, `power.json` presets' note): after its reaction time, the combat
/// preset on red alert and cruise when it ends, and the preset a COMBAT or CRUISE order names. Shedding by priority
/// is the solve's, and the reactor follows the load on its own AUTO. It never scrams, vents, closes a locked breaker
/// or sheds life support or the computer core.
pub fn engineering(
    bridge: &Bridge,
    data: &DrillData,
    p: &EngineeringProfile,
    mem: &mut EngineeringMemory,
    now_s: f64,
) -> Vec<Command> {
    let presets = data.power.presets();
    let preset = |name: &str| presets.iter().position(|n| n.eq_ignore_ascii_case(name)).map(|i| i as u8);
    let mut out = Vec::new();
    if mem.alert != Some(bridge.red_alert) {
        let since = *mem.since_s.get_or_insert(now_s);
        if now_s - since >= p.reaction_s {
            mem.alert = Some(bridge.red_alert);
            mem.since_s = None;
            out.extend(
                preset(if bridge.red_alert { "combat" } else { "cruise" }).map(|i| Command::Power(PowerCmd::Preset(i))),
            );
        }
    }
    // The captain's order to it, once it has had its reaction time (its acknowledgement is the drill's rule).
    let verbs = data.order_verbs(Station::Engineering);
    if let Some(o) =
        bridge.orders.iter().find(|o| o.to == Station::Engineering && !o.done && now_s - o.at_s >= p.reaction_s)
    {
        if let Some(i) = verbs.get(usize::from(o.verb)).and_then(|v| preset(&v[0])) {
            out.push(Command::Power(PowerCmd::Preset(i)));
        }
    }
    out
}

/// When the captain's automation next decides, and the orders a bot captain has given this round.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CaptainMemory {
    /// Seconds into Engage.
    pub next_s: f64,
    /// The orders given this round, a bit each in [`captain_orders`]'s list.
    pub given: u8,
    /// When the last order went, seconds into Engage.
    pub ordered_s: Option<f64>,
}

/// The captain's automation (bridge-stations 3): auto-condition, nothing else. Red alert when a hostile comes within
/// `red_within_m`; normal once every hostile has been beyond `normal_beyond_m` for `normal_after_s` (the
/// hysteresis keeps it from flickering).
pub fn captain(
    pic: &Picture,
    bridge: &Bridge,
    p: &CaptainProfile,
    mem: &mut CaptainMemory,
    now_s: f64,
) -> Vec<Command> {
    if now_s < mem.next_s {
        return Vec::new();
    }
    mem.next_s = now_s + p.reaction_s;
    let range = pic.hostile.as_ref().map(|h| (h.pos - pic.own.pos).length());
    if !bridge.red_alert && range.is_some_and(|r| r <= p.red_within_m) {
        return vec![Command::Alert(true)];
    }
    let clear = range.is_none_or(|r| r > p.normal_beyond_m);
    if bridge.red_alert && clear && now_s - bridge.hostile_near_s >= p.normal_after_s {
        return vec![Command::Alert(false)];
    }
    Vec::new()
}

/// A bot captain's orders (crew-chat design 3): at most one each `gap_s`, each once a round, the first that applies
/// of: Tactical ENGAGE, then Helm CHASE, at red alert with a hostile in the fight; Science SCAN while the hostile is not
/// scanned; Engineering REPAIR with the hull under `repair_below_frac` of full; Engineering CRUISE when red alert ends
/// after the fight. A verb the station does not have (`data/stations.json`) is skipped.
pub fn captain_orders(
    pic: &Picture,
    bridge: &Bridge,
    data: &DrillData,
    p: &CaptainOrders,
    mem: &mut CaptainMemory,
    now_s: f64,
) -> Vec<Command> {
    if mem.ordered_s.is_some_and(|t| now_s - t < p.gap_s) {
        return Vec::new();
    }
    let fight = bridge.red_alert && pic.hostile.is_some();
    let wants = [
        (Station::Tactical, "ENGAGE", fight),
        (Station::Helm, "CHASE", fight),
        (Station::Science, "SCAN", pic.hostile.is_some() && bridge.scan < 1.0),
        (Station::Engineering, "REPAIR", pic.own.hull_mj < data.tern_combat.hull_mj * p.repair_below_frac),
        (Station::Engineering, "CRUISE", !bridge.red_alert && mem.given & 1 != 0),
    ];
    for (bit, (to, verb, applies)) in wants.into_iter().enumerate() {
        if !applies || mem.given & (1 << bit) != 0 {
            continue;
        }
        let Some(i) = data.order_verbs(to).iter().position(|v| v[0] == verb) else { continue };
        mem.given |= 1 << bit;
        mem.ordered_s = Some(now_s);
        return vec![Command::Order(to, i as u8)];
    }
    Vec::new()
}

/// The helm's decisions at `now_s` (seconds into Engage). An `automation` profile holds heading and speed and
/// never flies evasive (bridge-stations section 3), so it issues nothing; a hunting one flies bow on the hostile,
/// weaving across its line of sight, and holds its standoff range.
pub fn helm(pic: &Picture, p: &HelmProfile, flight: &FlightBlock, mem: &mut HelmMemory, now_s: f64) -> Vec<Command> {
    if now_s < mem.next_s {
        return Vec::new();
    }
    mem.next_s = now_s + p.reaction_s;
    let Some(h) = pic.hostile else { return Vec::new() };
    if !p.hunt {
        return Vec::new();
    }
    let o = &pic.own;
    let to = h.pos - o.pos;
    let range = to.length();
    let up = o.rot * DVec3::Y;
    let weave = (p.weave_deg.to_radians()) * (std::f64::consts::TAU * now_s / p.weave_period_s).sin();
    let desired = DQuat::from_axis_angle(up, weave) * to;
    let stick = steer_toward(o.rot, o.rates, flight, desired, p.rate_frac);
    let speed =
        if range < p.standoff_m - 400.0 { 30.0 } else { (60.0 + (range - p.standoff_m) * 0.15).clamp(30.0, 300.0) };
    vec![Command::Stick { speed_set_mps: speed, yaw: stick[0], pitch: stick[1], roll: stick[2] }]
}

/// The preset whose strongest face points at a hostile in direction `to` (system frame): bow, stern, port or
/// starboard by the nearest horizontal axis; balanced when it is above or below.
pub fn preset_facing(data: &DrillData, rot: DQuat, to: DVec3) -> u8 {
    let l = rot.inverse() * to;
    let a = l.abs();
    let id = if a.y > a.x && a.y > a.z {
        "balanced"
    } else if a.z >= a.x {
        if l.z >= 0.0 {
            "bow"
        } else {
            "stern"
        }
    } else if l.x >= 0.0 {
        "port"
    } else {
        "starboard"
    };
    data.tern_shields.presets.iter().position(|p| p.id == id).unwrap_or(0) as u8
}

/// Tactical's decisions at `now_s`: lock the hostile, set the turrets to AUTO in range and HOLD out of it, set the
/// shields, and (when the profile allows missiles) keep the tubes loaded and fire when armed, locked and in the
/// seeker's cone.
pub fn tactical(
    pic: &Picture,
    p: &TacticalProfile,
    data: &DrillData,
    mem: &mut TacticalMemory,
    now_s: f64,
) -> Vec<Command> {
    if now_s < mem.next_s {
        return Vec::new();
    }
    mem.next_s = now_s + p.reaction_s;
    let mut out = Vec::new();
    let Some(h) = pic.hostile else { return out };
    let o = &pic.own;
    if o.lock_target != Some(h.id) {
        out.push(Command::Lock(Some(h.id)));
    }
    let to = h.pos - o.pos;
    let range = to.length();
    let want_mode = if range <= p.weapons_range_m { TurretMode::Auto } else { TurretMode::Hold };
    for (i, m) in o.turret_modes.iter().enumerate() {
        if *m != want_mode {
            out.push(Command::TurretMode(i as u8, want_mode));
        }
    }
    let want = if p.face_threat { preset_facing(data, o.rot, to) } else { 0 };
    if o.preset != want {
        out.push(Command::Preset(want));
    }
    if p.missiles {
        if let Some(tubes) = &data.tern_combat.tubes {
            let seeker = data.missile(&tubes.missile).seeker_half_angle_deg;
            let mut magazine = o.magazine;
            let mut fired = false;
            for (i, t) in o.tubes.iter().enumerate() {
                match t {
                    TubeState::Empty if magazine > 0 => {
                        magazine -= 1;
                        out.push(Command::Load(i as u8));
                    }
                    TubeState::Armed
                        if !fired && o.locked && range < 8000.0 && off_bow_deg(o.pos, o.rot, h.pos) <= seeker * 0.9 =>
                    {
                        fired = true;
                        out.push(Command::Fire(i as u8));
                    }
                    _ => {}
                }
            }
        }
    }
    out
}
