//! The patrol (openspec/changes/patrol-mission): the Tern docked at a starbase, the drive started and the clamps let
//! go, the berth's departure path flown, the jump drive between the points of a route, the sweep at each, the drill's
//! fight at the encounter's point, and the jump home.
//!
//! It lives in the core beside the drill because the drill runs it (`Drill::step`), its rules are the server's, and
//! the same decisions are made by the drill's automation for an empty seat and by a bot client from its snapshot
//! ([`decide`], CLAUDE.md 6.1). Positions are the drill's local frame: a floating origin (`PatrolState::origin`) in the
//! system frame, moved to the arrival point at each jump, so combat and the wire keep small numbers (CLAUDE.md 7).
//! Tuning is `data/ships/tern/jump.json`, the mission's `patrol` block and the system file; nothing here is a number a
//! designer would change.

use glam::{DQuat, DVec3};
use serde::Deserialize;

use super::{Command, PowerCmd, Station};
use crate::data::{Checks, Validate};
use crate::system::{Leg, Place, SystemFile};

/// The jump drive (`data/ships/tern/jump.json`, flight-and-navigation 7).
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct JumpData {
    /// `starcrew.jump/1`.
    pub schema: String,
    /// The energy a spool takes, MJ.
    pub spool_mj: f64,
    /// The spool's draw on a whole, powered drive, MW.
    pub spool_mw: f64,
    /// The bow within this of the jump vector, degrees.
    pub align_deg: f64,
    /// Turning slower than this, degrees a second.
    pub align_rate_dps: f64,
    /// For the spool's last this many seconds.
    pub align_last_s: f64,
    /// No spool within this many radii of a body's centre.
    pub planet_radii: f64,
    /// Nor within this of a hostile ship, metres.
    pub ship_m: f64,
    /// The Tern arrives within this of the arrival point, metres.
    pub arrival_scatter_m: f64,
    /// No spool for this long after a jump, seconds.
    pub cooldown_s: f64,
}

impl Validate for JumpData {
    fn validate(&self, c: &mut Checks) {
        c.equals("schema", &self.schema, "starcrew.jump/1");
        c.number("spool_mj", self.spool_mj, 1.0, 1e5);
        c.number("spool_mw", self.spool_mw, 0.1, 1e3);
        c.number("align_deg", self.align_deg, 0.1, 90.0);
        c.number("align_rate_dps", self.align_rate_dps, 0.01, 90.0);
        c.number("align_last_s", self.align_last_s, 0.0, 60.0);
        c.number("planet_radii", self.planet_radii, 1.0, 100.0);
        c.number("ship_m", self.ship_m, 0.0, 1e6);
        c.number("arrival_scatter_m", self.arrival_scatter_m, 0.0, 1e5);
        c.number("cooldown_s", self.cooldown_s, 0.0, 3600.0);
    }
}

/// The mission's `patrol` block.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PatrolBlock {
    /// The system file's id (`data/space/systems/<id>.json`).
    pub system: String,
    /// The station the Tern starts at and returns to.
    pub station: String,
    /// Its berth.
    pub berth: String,
    /// The points of interest in order.
    pub route: Vec<String>,
    /// The point where the drill's fight runs.
    pub encounter: String,
    /// How long the clamps take to open, seconds.
    pub clamps_s: f64,
    /// The impulse drive's capability the berth asks for before it opens the clamps, 0-1.
    pub drive_ready_min: f64,
    /// Science's sweep, seconds.
    pub sweep_s: f64,
    /// How far a sweep sees, metres.
    pub sweep_range_m: f64,
    /// A crew's helm (a bot, automation) clears a mass lock at this speed, and loiters at a point at this, m/s.
    pub clear_mps: f64,
    pub loiter_mps: f64,
    /// A crew's patrol command is given this long to land before it is sent again, seconds.
    pub resend_s: f64,
}

impl Validate for PatrolBlock {
    fn validate(&self, c: &mut Checks) {
        c.count("patrol.route", self.route.len() as i64, 1, 8);
        c.number("patrol.clamps_s", self.clamps_s, 0.0, 60.0);
        c.number("patrol.drive_ready_min", self.drive_ready_min, 0.0, 1.0);
        c.number("patrol.sweep_s", self.sweep_s, 1.0, 600.0);
        c.number("patrol.sweep_range_m", self.sweep_range_m, 1e3, 1e7);
        c.number("patrol.clear_mps", self.clear_mps, 1.0, 400.0);
        c.number("patrol.loiter_mps", self.loiter_mps, 0.0, 400.0);
        c.number("patrol.resend_s", self.resend_s, 0.1, 30.0);
    }
}

/// A patrol's data: its block, its system and the jump drive, and the places it visits resolved.
#[derive(Debug, Clone, PartialEq)]
pub struct PatrolData {
    /// The mission's block.
    pub block: PatrolBlock,
    /// The system.
    pub system: SystemFile,
    /// The jump drive.
    pub jump: JumpData,
    /// The places, home (the station) first, then the route in order.
    pub places: Vec<Place>,
    /// The index in `places` of the encounter's point.
    pub encounter: usize,
    /// The berth: where the docked Tern's origin is, its attitude, and its departure legs.
    pub berth_pos: DVec3,
    pub berth_rot: DQuat,
    pub departure: Vec<Leg>,
}

impl PatrolData {
    /// Resolve a block against its system file and the jump drive: every id it names must exist.
    pub fn resolve(block: PatrolBlock, system: SystemFile, jump: JumpData) -> Result<Self, String> {
        let st =
            system.station(&block.station).ok_or_else(|| format!("patrol.station: no station {}", block.station))?;
        let berth = st
            .berths
            .iter()
            .find(|b| b.id == block.berth)
            .ok_or_else(|| format!("patrol.berth: {} has no berth {}", st.id, block.berth))?;
        let (berth_pos, berth_rot) = system.berth_pose(st, berth);
        let departure = berth.departure.clone();
        let mut places = vec![system.place(&block.station).expect("the station was found")];
        for id in &block.route {
            places.push(system.place(id).ok_or_else(|| format!("patrol.route: no point or station {id}"))?);
        }
        let encounter = places
            .iter()
            .position(|p| p.id == block.encounter)
            .filter(|i| *i > 0)
            .ok_or_else(|| format!("patrol.encounter: {} is not on the route", block.encounter))?;
        Ok(Self { block, system, jump, places, encounter, berth_pos, berth_rot, departure })
    }
}

/// The power group that feeds the impulse drive (Engineering's breaker for it): the one rule the drill and a bot use.
pub fn drive_group(power: &crate::power::PowerData) -> Option<usize> {
    power.file.groups.iter().position(|g| g.loads.iter().any(|l| l == "impulse_drive"))
}

/// Where the patrol is (design 1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    /// Clamped in the berth.
    Docked = 0,
    /// The clamps opening.
    Releasing = 1,
    /// Flying the berth's departure path.
    Departing = 2,
    /// Free: in transit, at a point, or fighting.
    Underway = 3,
    /// Arrived home: the mission is over.
    Home = 4,
}

impl Stage {
    /// From its wire byte.
    pub fn from_u8(v: u8) -> Option<Self> {
        [Self::Docked, Self::Releasing, Self::Departing, Self::Underway, Self::Home].get(usize::from(v)).copied()
    }
}

/// The captain's standing order to the helm in the patrol (design 9); a console's control for it awaits its mockup.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PatrolOrder {
    /// Take us out of the berth.
    Undock = 0,
    /// Jump to the next place when ready.
    Jump = 1,
}

impl PatrolOrder {
    /// From its wire byte.
    pub fn from_u8(v: u8) -> Option<Self> {
        [Self::Undock, Self::Jump].get(usize::from(v)).copied()
    }
}

/// The patrol's state, server side (the snapshot carries it to every client).
#[derive(Clone, Debug, PartialEq)]
pub struct PatrolState {
    /// Where it is.
    pub stage: Stage,
    /// The umbilical is connected.
    pub shore: bool,
    /// The drill's local frame's origin in the system frame, metres.
    pub origin: DVec3,
    /// The place the Tern last arrived at (an index in `PatrolData::places`): home at the start.
    pub at: usize,
    /// The place the next jump goes to; none once home.
    pub next: Option<usize>,
    /// The departure leg being flown.
    pub leg: usize,
    /// Seconds left on the clamps.
    pub clamps_s: f64,
    /// Energy in the spool, MJ, and whether it is spooling.
    pub spool_mj: f64,
    pub spooling: bool,
    /// Seconds the bow has been aligned and still.
    pub aligned_s: f64,
    /// Seconds before the drive may spool again.
    pub cooldown_s: f64,
    /// The sweep's progress (0-1) and whether it runs.
    pub sweep: f64,
    pub sweeping: bool,
    /// Each place swept (home counts as swept).
    pub swept: Vec<bool>,
    /// The captain's standing order.
    pub ordered: Option<PatrolOrder>,
    /// The bow has cleared the berth.
    pub undocked: bool,
    /// The drill's fight has been met.
    pub met: bool,
    /// Jumps made.
    pub jumps: u32,
    /// The drive was ready when last looked (for its report, once).
    pub drive_said: bool,
}

impl PatrolState {
    /// Docked in the berth, on shore power: the drill's local origin is the berth.
    pub fn new(d: &PatrolData) -> Self {
        let mut swept = vec![false; d.places.len()];
        swept[0] = true;
        Self {
            stage: Stage::Docked,
            shore: true,
            origin: d.berth_pos,
            at: 0,
            next: Some(1),
            leg: 0,
            clamps_s: d.block.clamps_s,
            spool_mj: 0.0,
            spooling: false,
            aligned_s: 0.0,
            cooldown_s: 0.0,
            sweep: 0.0,
            sweeping: false,
            swept,
            ordered: None,
            undocked: false,
            met: false,
            jumps: 0,
            drive_said: false,
        }
    }

    /// A place's position in the local frame.
    pub fn local(&self, p: DVec3) -> DVec3 {
        p - self.origin
    }

    /// The point the helm's COURSE flies at: the next place, else the one it is at; local frame.
    pub fn course_point(&self, d: &PatrolData) -> DVec3 {
        let i = self.next.unwrap_or(self.at);
        self.local(d.places[i].pos)
    }
}

/// The berth's departure, flown by the assist (design 4.2): the set points that carry the Tern from `pos` along the
/// path's leg `leg`, never faster than the leg allows, the attitude held. Returns the forward speed set point, the
/// strafe set points [to starboard, up] (m/s, the drill's `Ship`), and whether this leg is done. `berth_pos` and
/// `berth_rot` are the berth in the local frame, `rot` the Tern's attitude.
pub fn departure_setpoints(
    pos: DVec3,
    rot: DQuat,
    berth_pos: DVec3,
    berth_rot: DQuat,
    legs: &[Leg],
    leg: usize,
) -> (f64, [f64; 2], bool) {
    let Some(l) = legs.get(leg) else { return (0.0, [0.0; 2], true) };
    let from = if leg == 0 { berth_pos } else { berth_pos + berth_rot * DVec3::from_array(legs[leg - 1].to_m) };
    let to = berth_pos + berth_rot * DVec3::from_array(l.to_m);
    let along = (to - from).normalize_or_zero();
    // Done when the Tern has passed the leg's end along the leg.
    let done = (to - pos).dot(along) <= 0.05;
    // Toward the end, at the leg's speed, steering back onto the leg's line.
    let off = (pos - from) - along * (pos - from).dot(along);
    let want = along * l.max_mps - off * 0.5;
    let want = if want.length() > l.max_mps { want.normalize() * l.max_mps } else { want };
    let ship = rot.inverse() * want;
    (ship.z, [-ship.x, ship.y], done)
}

/// Whether the bow is on the jump vector within the drive's tolerance and turning slowly enough (FN 7).
pub fn aligned(rot: DQuat, rates: DVec3, jump_dir: DVec3, j: &JumpData) -> bool {
    let bow = rot * DVec3::Z;
    bow.angle_between(jump_dir.normalize_or(DVec3::Z)).to_degrees() <= j.align_deg
        && rates.length().to_degrees() <= j.align_rate_dps
}

/// What a station sees of the patrol to decide on (design 9): the drill's automation builds it from the drill, a bot
/// from its snapshot.
#[derive(Clone, Debug, PartialEq)]
pub struct PatrolPicture {
    /// The stage.
    pub stage: Stage,
    /// On shore power.
    pub shore: bool,
    /// The impulse drive's feed breaker is open.
    pub drive_open: bool,
    /// The drive is ready for the berth (shore off and its capability at least `drive_ready_min`).
    pub drive_ready: bool,
    /// The drive group's index (Engineering's breaker).
    pub drive_group: u8,
    /// Spooling.
    pub spooling: bool,
    /// Seconds of cooldown left.
    pub cooldown_s: f64,
    /// Something locks the jump here.
    pub locked: bool,
    /// The place the Tern is at has been swept (true at home).
    pub swept_here: bool,
    /// A sweep runs.
    pub sweeping: bool,
    /// The patrol is home, or has no next place.
    pub done: bool,
    /// The captain's standing order.
    pub ordered: Option<PatrolOrder>,
    /// A person or a bot holds the captain's seat.
    pub captain_manned: bool,
    /// A hostile ship is in the fight.
    pub hostile: bool,
    /// The way to the next place from the Tern, local frame (zero when none).
    pub next_dir: DVec3,
    /// The Tern's attitude and body rates.
    pub rot: DQuat,
    pub rates: DVec3,
    /// The helm is on COURSE already.
    pub on_course: bool,
}

/// A station's memory between decisions: when it last did each thing, so it does not repeat one while the snapshot
/// catches up.
#[derive(Clone, Debug, PartialEq)]
pub struct PatrolMemory {
    /// When the station may next decide (its reaction time).
    pub next_s: f64,
    /// When it last sent each patrol command, by [`act_index`].
    pub sent_s: [f64; 8],
}

impl Default for PatrolMemory {
    /// Nothing sent yet: every command may go at once.
    fn default() -> Self {
        Self { next_s: f64::NEG_INFINITY, sent_s: [f64::NEG_INFINITY; 8] }
    }
}

/// The slot in `PatrolMemory::sent_s` of a command.
fn act_index(c: &Command) -> usize {
    match c {
        Command::Shore(_) => 0,
        Command::Power(_) => 1,
        Command::Undock => 2,
        Command::Jump(_) => 3,
        Command::Sweep => 4,
        Command::Patrol(_) => 5,
        Command::Helm(_) => 6,
        _ => 7,
    }
}

/// A station's patrol decisions (design 9), the one rule for the drill's automation and a bot: what it sends now,
/// at most once a `reaction_s`. The captain gives the standing orders; with no captain aboard, helm acts on its own.
pub fn decide(
    st: Station,
    p: &PatrolPicture,
    b: &PatrolBlock,
    mem: &mut PatrolMemory,
    now_s: f64,
    reaction_s: f64,
) -> Vec<Command> {
    if now_s < mem.next_s || p.done {
        return Vec::new();
    }
    mem.next_s = now_s + reaction_s;
    let mut out = Vec::new();
    let go = |o: PatrolOrder| p.ordered == Some(o) || !p.captain_manned;
    match st {
        Station::Engineering if p.stage == Stage::Docked => {
            if p.drive_open {
                out.push(Command::Power(PowerCmd::Breaker(p.drive_group)));
            } else if p.shore {
                out.push(Command::Shore(false));
            }
        }
        Station::Helm => match p.stage {
            Stage::Docked if p.drive_ready && go(PatrolOrder::Undock) => out.push(Command::Undock),
            Stage::Underway if !p.hostile => {
                if !p.on_course {
                    out.push(Command::Helm(super::HelmMode::Course));
                }
                let speed = if p.locked { b.clear_mps } else { b.loiter_mps };
                out.push(Command::Stick { speed_set_mps: speed, yaw: 0.0, pitch: 0.0, roll: 0.0 });
                if !p.spooling && !p.locked && p.cooldown_s <= 0.0 && p.swept_here && go(PatrolOrder::Jump) {
                    out.push(Command::Jump(true));
                }
            }
            _ => {}
        },
        Station::Science if p.stage == Stage::Underway && !p.swept_here && !p.sweeping && !p.hostile => {
            out.push(Command::Sweep);
        }
        Station::Captain => match p.stage {
            Stage::Docked if p.drive_ready && p.ordered != Some(PatrolOrder::Undock) => {
                out.push(Command::Patrol(PatrolOrder::Undock));
            }
            Stage::Underway
                if !p.hostile
                    && p.swept_here
                    && !p.spooling
                    && p.cooldown_s <= 0.0
                    && p.ordered != Some(PatrolOrder::Jump) =>
            {
                out.push(Command::Patrol(PatrolOrder::Jump));
            }
            _ => {}
        },
        _ => {}
    }
    // A command already sent is given time to land; the helm's flying is sent every time.
    out.retain(|c| {
        let k = act_index(c);
        if k == 6 || matches!(c, Command::Stick { .. }) {
            return true;
        }
        if now_s - mem.sent_s[k] < b.resend_s {
            return false;
        }
        mem.sent_s[k] = now_s;
        true
    });
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::parse;

    fn data() -> PatrolData {
        let sys: SystemFile =
            parse("shoals", include_str!("../../../../data/space/systems/shoals.json")).expect("the system");
        let jump: JumpData = parse("jump", include_str!("../../../../data/ships/tern/jump.json")).expect("the drive");
        let block = PatrolBlock {
            system: "shoals".into(),
            station: "tamsin_station".into(),
            berth: "berth_a".into(),
            route: vec!["pell_beacon".into(), "shoals_relay".into()],
            encounter: "shoals_relay".into(),
            clamps_s: 3.0,
            drive_ready_min: 0.5,
            sweep_s: 20.0,
            sweep_range_m: 50_000.0,
            clear_mps: 150.0,
            loiter_mps: 40.0,
            resend_s: 2.0,
        };
        PatrolData::resolve(block, sys, jump).expect("resolves")
    }

    fn picture() -> PatrolPicture {
        PatrolPicture {
            stage: Stage::Docked,
            shore: true,
            drive_open: true,
            drive_ready: false,
            drive_group: 0,
            spooling: false,
            cooldown_s: 0.0,
            locked: true,
            swept_here: true,
            sweeping: false,
            done: false,
            ordered: None,
            captain_manned: true,
            hostile: false,
            next_dir: DVec3::Z,
            rot: DQuat::IDENTITY,
            rates: DVec3::ZERO,
            on_course: false,
        }
    }

    #[test]
    fn a_patrol_starts_in_the_berth_with_the_route_resolved() {
        let d = data();
        let s = PatrolState::new(&d);
        assert_eq!((s.stage, s.shore, s.next), (Stage::Docked, true, Some(1)));
        assert_eq!(
            d.places.iter().map(|p| p.id.as_str()).collect::<Vec<_>>(),
            ["tamsin_station", "pell_beacon", "shoals_relay"]
        );
        assert_eq!(d.encounter, 2);
        assert_eq!(s.local(d.berth_pos), DVec3::ZERO, "the drill's origin is the berth");
    }

    #[test]
    fn a_route_naming_an_unknown_point_is_refused() {
        let d = data();
        let mut b = d.block.clone();
        b.route.push("nowhere".into());
        assert!(PatrolData::resolve(b, d.system.clone(), d.jump.clone()).unwrap_err().contains("nowhere"));
    }

    #[test]
    fn the_departure_backs_the_tern_out_at_the_first_legs_speed() {
        let d = data();
        // Docked, local frame: the berth at the origin, the Tern in it.
        let (speed, strafe, done) =
            departure_setpoints(DVec3::ZERO, d.berth_rot, DVec3::ZERO, d.berth_rot, &d.departure, 0);
        assert!(!done);
        assert!((speed + 1.0).abs() < 1e-6, "astern at the first leg's 1 m/s, got {speed}");
        assert!(strafe[0].abs() < 1e-6 && strafe[1].abs() < 1e-6, "straight back: {strafe:?}");
    }

    #[test]
    fn a_leg_is_done_past_its_end_and_the_next_is_faster() {
        let d = data();
        let past = d.berth_rot * DVec3::new(0.0, 0.0, -10.2);
        let (_, _, done) = departure_setpoints(past, d.berth_rot, DVec3::ZERO, d.berth_rot, &d.departure, 0);
        assert!(done);
        let (speed, _, done) = departure_setpoints(past, d.berth_rot, DVec3::ZERO, d.berth_rot, &d.departure, 1);
        assert!(!done && (speed + 4.0).abs() < 1e-6, "the second leg at 4 m/s: {speed}");
    }

    #[test]
    fn a_tern_off_the_legs_line_is_steered_back_onto_it() {
        let d = data();
        let off = d.berth_rot * DVec3::new(2.0, 0.0, -5.0);
        let (_, strafe, _) = departure_setpoints(off, d.berth_rot, DVec3::ZERO, d.berth_rot, &d.departure, 0);
        assert!(strafe[0] > 0.0, "2 m to port is steered to starboard: {strafe:?}");
    }

    #[test]
    fn alignment_needs_the_bow_on_and_the_ship_still() {
        let j = data().jump;
        assert!(aligned(DQuat::IDENTITY, DVec3::ZERO, DVec3::Z, &j));
        let off = DQuat::from_rotation_y(10f64.to_radians());
        assert!(!aligned(off, DVec3::ZERO, DVec3::Z, &j), "10 degrees off the jump vector");
        assert!(!aligned(DQuat::IDENTITY, DVec3::Y * 5f64.to_radians(), DVec3::Z, &j), "turning at 5 deg/s");
    }

    #[test]
    fn engineering_closes_the_drive_then_drops_shore_power() {
        let mut m = PatrolMemory::default();
        let mut p = picture();
        assert_eq!(
            decide(Station::Engineering, &p, &data().block, &mut m, 0.0, 0.5),
            [Command::Power(PowerCmd::Breaker(0))]
        );
        p.drive_open = false;
        assert_eq!(decide(Station::Engineering, &p, &data().block, &mut m, 1.0, 0.5), [Command::Shore(false)]);
    }

    #[test]
    fn the_helm_waits_for_the_captains_order_to_undock() {
        let mut m = PatrolMemory::default();
        let mut p = picture();
        p.drive_ready = true;
        p.shore = false;
        assert!(decide(Station::Helm, &p, &data().block, &mut m, 0.0, 0.3).is_empty(), "no order yet");
        p.ordered = Some(PatrolOrder::Undock);
        assert_eq!(decide(Station::Helm, &p, &data().block, &mut m, 1.0, 0.3), [Command::Undock]);
    }

    #[test]
    fn with_no_captain_aboard_the_helm_undocks_on_its_own() {
        let mut m = PatrolMemory::default();
        let mut p = picture();
        p.drive_ready = true;
        p.captain_manned = false;
        assert_eq!(decide(Station::Helm, &p, &data().block, &mut m, 0.0, 0.3), [Command::Undock]);
    }

    #[test]
    fn a_command_sent_is_not_sent_again_before_it_can_land() {
        let mut m = PatrolMemory::default();
        let p = picture();
        assert_eq!(decide(Station::Engineering, &p, &data().block, &mut m, 0.0, 0.1).len(), 1);
        assert!(
            decide(Station::Engineering, &p, &data().block, &mut m, 0.5, 0.1).is_empty(),
            "the breaker is not toggled twice"
        );
        assert_eq!(decide(Station::Engineering, &p, &data().block, &mut m, 2.5, 0.1).len(), 1);
    }

    #[test]
    fn the_helm_clears_the_lock_before_it_spools() {
        let mut m = PatrolMemory::default();
        let mut p = picture();
        p.stage = Stage::Underway;
        p.ordered = Some(PatrolOrder::Jump);
        let c = decide(Station::Helm, &p, &data().block, &mut m, 0.0, 0.3);
        assert!(c.contains(&Command::Helm(super::super::HelmMode::Course)));
        assert!(!c.contains(&Command::Jump(true)), "still inside the station's lock");
        p.locked = false;
        assert!(decide(Station::Helm, &p, &data().block, &mut m, 1.0, 0.3).contains(&Command::Jump(true)));
    }

    #[test]
    fn science_sweeps_a_new_point_and_the_captain_orders_the_jump_after() {
        let mut m = PatrolMemory::default();
        let mut p = picture();
        p.stage = Stage::Underway;
        p.swept_here = false;
        assert_eq!(decide(Station::Science, &p, &data().block, &mut m, 0.0, 0.5), [Command::Sweep]);
        assert!(decide(Station::Captain, &p, &data().block, &mut PatrolMemory::default(), 0.0, 0.5).is_empty());
        p.swept_here = true;
        assert_eq!(
            decide(Station::Captain, &p, &data().block, &mut PatrolMemory::default(), 0.0, 0.5),
            [Command::Patrol(PatrolOrder::Jump)]
        );
    }
}
