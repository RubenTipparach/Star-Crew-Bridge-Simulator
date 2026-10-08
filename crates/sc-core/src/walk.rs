//! Walking a ship's decks: a crew member's body on Rapier's kinematic character controller (openspec/changes/
//! crew-on-deck sections 2-4 and 3a; deck-pipeline section 13a: the first walk in the engine).
//!
//! It lives in the core because where a body can stand and go is a gameplay rule the server, the client's
//! prediction and the tests must agree on (CLAUDE.md 6.2, 6.3). It takes the walk world as triangles and
//! entities (from `deck::Deck`), the walk's numbers as data (`data/crew/walk.json`, `WalkData`) and a body's
//! input a fixed step at a time; it reads no file and no clock.
//!
//! The rules are the deck plan's walk (`docs/mockups/lib/shipwalk.js`, its Rapier path), which reads the same
//! data file, so the mockup and the engine walk alike:
//! - the body is a standing capsule; it speeds up and brakes at their rates, walks, runs, and moves slower
//!   backwards; it steps up ledges, follows ramps and stairs (stairs collide as ramps), slides along walls,
//!   falls, and jumps;
//! - Use climbs a ladder or floor hatch from either end and goes through a wall hatch whose sill is too high;
//! - inside a lift's shaft the feet stand on its car; every landing the car is not standing at is walled.
//!
//! What it leaves out for now (deck-pipeline 13a): doors stand open, the car does not move, one body.

use crate::data::{Checks, Validate};
use crate::deck::{WalkDoor, WalkHatch, WalkLadder, WalkLift, WalkStart};
use rapier3d::control::{CharacterLength, KinematicCharacterController};
use rapier3d::parry::query::ShapeCastOptions;
use rapier3d::prelude::*;
use serde::Deserialize;

/// `data/crew/walk.json` (`starcrew.walk/1`): the walk's numbers, units in the keys.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct WalkData {
    /// `starcrew.walk/1`.
    pub schema: String,
    /// Proposal and provenance.
    pub status: String,
    /// The standing capsule.
    pub body: BodyData,
    /// Speeds and rates.
    pub r#move: MoveData,
    /// Ladders and floor hatches.
    pub ladder: LadderData,
    /// Seconds to go through a wall hatch.
    pub side_hatch_s: f64,
    /// Doors (read by the mockup today; the engine's doors come with their leaves).
    pub door: DoorData,
    /// Rapier's character controller.
    pub controller: ControllerData,
    /// How the eye follows the feet.
    pub eye: EyeData,
    /// Gravity on the decks, m/s^2.
    pub gravity_m_s2: f64,
    /// The fastest fall, m/s.
    pub fall_max_m_s: f64,
    /// A jump.
    pub jump: JumpData,
    /// How far below the feet a walking body still finds its floor, metres (the mockup's other controller).
    pub drop_m: f64,
    /// How near a lift's door calls the car, metres.
    pub lift_call_m: f64,
    /// The walk's substep, seconds.
    pub substep_s: f64,
}

/// The standing capsule (crew-on-deck section 2).
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct BodyData {
    /// Radius, metres.
    pub radius_m: f64,
    /// Height, metres.
    pub height_m: f64,
    /// The eye above the feet, metres.
    pub eye_m: f64,
    /// The highest step climbed without a jump, metres.
    pub step_m: f64,
}

/// Speeds and rates (crew-on-deck section 3).
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MoveData {
    /// Walking, m/s.
    pub walk_m_s: f64,
    /// Running, m/s.
    pub run_m_s: f64,
    /// The share of the speed kept moving backwards.
    pub back_scale: f64,
    /// The share kept on stairs (the mockup's other controller; Rapier slows a body on a slope itself).
    pub stair_scale: f64,
    /// Speeding up, m/s^2.
    pub accel_m_s2: f64,
    /// Braking, m/s^2.
    pub stop_m_s2: f64,
}

/// Ladders and floor hatches (crew-on-deck section 4).
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct LadderData {
    /// Climbing up, m/s.
    pub up_m_s: f64,
    /// Climbing down, m/s.
    pub down_m_s: f64,
    /// Getting on, seconds.
    pub mount_s: f64,
    /// Getting off, seconds.
    pub dismount_s: f64,
    /// How near the centre a body stands to use it, metres.
    pub reach_m: f64,
}

/// Doors (crew-on-deck section 5).
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct DoorData {
    /// A door opens for a body this near its plane, metres.
    pub zone_m: f64,
    /// ...and this far past its sides, metres.
    pub zone_side_m: f64,
    /// It closes this long after its zone is empty, seconds.
    pub close_after_s: f64,
    /// Opening, seconds.
    pub open_s: f64,
    /// Closing, seconds.
    pub close_s: f64,
    /// A pressure door opening, seconds.
    pub pressure_open_s: f64,
    /// A pressure door closing, seconds.
    pub pressure_close_s: f64,
    /// The opening (0 shut, 1 open) from which a body passes.
    pub passable: f64,
    /// A leaf's thickness, metres.
    pub thick_m: f64,
}

/// Rapier's character controller (crew-on-deck section 3a).
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ControllerData {
    /// The gap kept to every surface, metres.
    pub offset_m: f64,
    /// A ledge stepped onto must be at least this deep, metres.
    pub step_depth_m: f64,
    /// How far below the feet the floor is kept, metres.
    pub snap_m: f64,
    /// The steepest slope climbed, degrees.
    pub climb_deg: f64,
    /// Slopes from this steepness are slid down, degrees.
    pub slide_deg: f64,
}

/// How the eye follows the feet's height.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct EyeData {
    /// The time constant, seconds.
    pub tau_s: f64,
    /// It is never further behind than this, metres.
    pub lag_m: f64,
}

/// A jump.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct JumpData {
    /// Take-off speed, m/s.
    pub speed_m_s: f64,
    /// For this long after take-off the floor does not count as landing, seconds.
    pub lift_s: f64,
}

impl Validate for WalkData {
    fn validate(&self, c: &mut Checks) {
        c.equals("schema", &self.schema, "starcrew.walk/1");
        let b = &self.body;
        c.number("body.radius_m", b.radius_m, 0.1, 0.6);
        c.number("body.height_m", b.height_m, 2.0 * b.radius_m + 0.1, 2.5);
        c.number("body.eye_m", b.eye_m, 0.5, b.height_m);
        c.number("body.step_m", b.step_m, 0.0, 0.6);
        let m = &self.r#move;
        c.number("move.walk_m_s", m.walk_m_s, 0.1, 10.0);
        c.number("move.run_m_s", m.run_m_s, m.walk_m_s, 15.0);
        c.number("move.back_scale", m.back_scale, 0.0, 1.0);
        c.number("move.stair_scale", m.stair_scale, 0.0, 1.0);
        c.number("move.accel_m_s2", m.accel_m_s2, 0.1, 200.0);
        c.number("move.stop_m_s2", m.stop_m_s2, 0.1, 200.0);
        let l = &self.ladder;
        for (k, v) in [("up_m_s", l.up_m_s), ("down_m_s", l.down_m_s)] {
            c.number(&format!("ladder.{k}"), v, 0.1, 10.0);
        }
        for (k, v) in [("mount_s", l.mount_s), ("dismount_s", l.dismount_s)] {
            c.number(&format!("ladder.{k}"), v, 0.0, 5.0);
        }
        c.number("ladder.reach_m", l.reach_m, 0.1, 3.0);
        c.number("side_hatch_s", self.side_hatch_s, 0.0, 10.0);
        let d = &self.door;
        for (k, v) in [
            ("zone_m", d.zone_m),
            ("zone_side_m", d.zone_side_m),
            ("close_after_s", d.close_after_s),
            ("open_s", d.open_s),
            ("close_s", d.close_s),
            ("pressure_open_s", d.pressure_open_s),
            ("pressure_close_s", d.pressure_close_s),
        ] {
            c.number(&format!("door.{k}"), v, 0.0, 30.0);
        }
        c.number("door.passable", d.passable, 0.0, 1.0);
        c.number("door.thick_m", d.thick_m, 0.001, 0.5);
        let k = &self.controller;
        c.number("controller.offset_m", k.offset_m, 0.001, 0.2);
        c.number("controller.step_depth_m", k.step_depth_m, 0.0, 1.0);
        c.number("controller.snap_m", k.snap_m, 0.0, 1.0);
        c.number("controller.climb_deg", k.climb_deg, 0.0, 89.0);
        c.number("controller.slide_deg", k.slide_deg, 0.0, 89.0);
        c.number("eye.tau_s", self.eye.tau_s, 0.0, 1.0);
        c.number("eye.lag_m", self.eye.lag_m, 0.0, 2.0);
        c.number("gravity_m_s2", self.gravity_m_s2, 0.0, 50.0);
        c.number("fall_max_m_s", self.fall_max_m_s, 0.1, 100.0);
        c.number("jump.speed_m_s", self.jump.speed_m_s, 0.0, 10.0);
        c.number("jump.lift_s", self.jump.lift_s, 0.0, 1.0);
        c.number("drop_m", self.drop_m, 0.0, 2.0);
        c.number("lift_call_m", self.lift_call_m, 0.0, 10.0);
        c.number("substep_s", self.substep_s, 1e-4, 0.05);
    }
}

/// The entities a body uses, as `deck::DeckWalk` carries them.
#[derive(Debug, Clone, Default)]
pub struct WalkEntities {
    /// Ladders and floor hatches.
    pub ladders: Vec<WalkLadder>,
    /// Wall hatches.
    pub hatches: Vec<WalkHatch>,
    /// Every door's opening.
    pub doors: Vec<WalkDoor>,
    /// The lifts.
    pub lifts: Vec<WalkLift>,
}

/// A wall hatch with its two sides' floor points, found once: `(x, floor, z)` on each side.
struct Hatch {
    at: [f32; 2],
    normal: [f32; 2],
    sides: [[f32; 3]; 2],
}

/// The walk world: one static triangle mesh, the walls across the lift landings the car is not at, and the
/// entities. Built once; bodies only query it.
pub struct WalkWorld {
    colliders: ColliderSet,
    bodies: RigidBodySet,
    broad_phase: BroadPhaseBvh,
    narrow_phase: NarrowPhase,
    ladders: Vec<WalkLadder>,
    hatches: Vec<Hatch>,
    lifts: Vec<WalkLift>,
    /// Triangles in the static mesh.
    pub triangles: usize,
}

impl WalkWorld {
    /// Build the world from triangles (three corners, ship coordinates, metres) and the entities.
    pub fn new(triangles: &[[f32; 9]], e: &WalkEntities, d: &WalkData) -> Result<Self, String> {
        let mut colliders = ColliderSet::new();
        let mut bodies = RigidBodySet::new();
        // The soup's corners as they come: a triangle soup, not welded (the controller does not need it).
        let mut verts = Vec::with_capacity(triangles.len() * 3);
        let mut idx = Vec::with_capacity(triangles.len());
        for (i, t) in triangles.iter().enumerate() {
            if t.iter().any(|v| !v.is_finite()) {
                return Err(format!("walk triangle {i} is not finite"));
            }
            for k in 0..3 {
                verts.push(Vector::new(t[k * 3], t[k * 3 + 1], t[k * 3 + 2]));
            }
            // Degenerate triangles (zero area) are dropped: they hold nothing up and Rapier refuses them.
            let (a, b, c) = (verts[i * 3], verts[i * 3 + 1], verts[i * 3 + 2]);
            if (b - a).cross(c - a).length_squared() > 1e-12 {
                idx.push([i as u32 * 3, i as u32 * 3 + 1, i as u32 * 3 + 2]);
            }
        }
        if !idx.is_empty() {
            let co = ColliderBuilder::trimesh(verts, idx.clone()).map_err(|e| format!("the walk mesh: {e:?}"))?;
            colliders.insert(co);
        }
        // A lift's landing door is a wall where the car is not standing (the car does not move yet).
        for dr in e.doors.iter().filter(|dr| dr.kind == "lift") {
            let stop = dr.center_m[1] - dr.height_m / 2.0;
            if e.lifts.iter().any(|l| (l.car_m - stop).abs() < 0.05) {
                continue;
            }
            let th = dr.normal[0].atan2(dr.normal[1]);
            let thick = d.door.thick_m as f32;
            colliders.insert(
                ColliderBuilder::cuboid(dr.width_m / 2.0, dr.height_m / 2.0, thick / 2.0)
                    .translation(Vector::new(dr.center_m[0], dr.center_m[1], dr.center_m[2]))
                    .rotation(Vector::Y * th),
            );
        }
        let mut w = Self {
            colliders,
            bodies: RigidBodySet::new(),
            broad_phase: BroadPhaseBvh::new(),
            narrow_phase: NarrowPhase::new(),
            ladders: e.ladders.clone(),
            hatches: Vec::new(),
            lifts: e.lifts.clone(),
            triangles: idx.len(),
        };
        // One step of an empty physics pipeline fills the broad phase's tree; nothing moves in it afterwards.
        let mut pipeline = PhysicsPipeline::new();
        pipeline.step(
            Vector::ZERO,
            &IntegrationParameters::default(),
            &mut IslandManager::new(),
            &mut w.broad_phase,
            &mut w.narrow_phase,
            &mut bodies,
            &mut w.colliders,
            &mut ImpulseJointSet::new(),
            &mut MultibodyJointSet::new(),
            &mut SoftBodySet::new(),
            &mut CCDSolver::new(),
            &(),
            &(),
        );
        w.bodies = bodies;
        // A wall hatch's two sides: 0.7 m out from its centre each way, and the floor there.
        for h in &e.hatches {
            let side = |s: f32| {
                let (x, z) = (h.x_m + h.normal[0] * 0.7 * s, h.z_m + h.normal[1] * 0.7 * s);
                w.floor_below(x, h.sill_m + 0.3, z).map(|y| [x, y, z])
            };
            if let (Some(a), Some(b)) = (side(-1.0), side(1.0)) {
                if h.sill_m - a[1] > d.body.step_m as f32 || h.sill_m - b[1] > d.body.step_m as f32 {
                    w.hatches.push(Hatch { at: [h.x_m, h.z_m], normal: h.normal, sides: [a, b] });
                }
            }
        }
        Ok(w)
    }

    fn queries(&self) -> QueryPipeline<'_> {
        self.broad_phase.as_query_pipeline(
            self.narrow_phase.query_dispatcher(),
            &self.bodies,
            &self.colliders,
            QueryFilter::default(),
        )
    }

    /// The first floor straight down from `(x, from, z)`, or `None`.
    pub fn floor_below(&self, x: f32, from: f32, z: f32) -> Option<f32> {
        let ray = Ray::new(Vector::new(x, from, z), Vector::new(0.0, -1.0, 0.0));
        self.queries().cast_ray(&ray, 40.0, true).map(|(_, toi)| from - toi)
    }

    /// The lift whose shaft holds `(x, z)`.
    fn lift_at(&self, x: f32, z: f32) -> Option<&WalkLift> {
        self.lifts.iter().find(|l| in_poly(&l.poly, x, z))
    }
}

/// `(x, z)` inside a polygon of `[x, z]` corners.
fn in_poly(poly: &[[f32; 2]], x: f32, z: f32) -> bool {
    let mut inside = false;
    let n = poly.len();
    for i in 0..n {
        let (a, b) = (poly[i], poly[(i + 1) % n]);
        if (a[1] > z) != (b[1] > z) && x < (b[0] - a[0]) * (z - a[1]) / (b[1] - a[1]) + a[0] {
            inside = !inside;
        }
    }
    inside
}

/// What a body is told to do this step.
#[derive(Debug, Clone, Copy, Default)]
pub struct Input {
    /// Forward (+1) or back (-1).
    pub forward: f32,
    /// Right (+1) or left (-1).
    pub right: f32,
    /// The facing in the plan, radians: forward is `(sin yaw, cos yaw)` in `(x, z)`.
    pub yaw: f32,
    /// Run.
    pub run: bool,
    /// Jump (taken once).
    pub jump: bool,
    /// Use: climb, go through (taken once).
    pub use_: bool,
    /// Use going down: where a ladder up and a ladder down both start (a trunk through three decks), take the one
    /// down (Q, as the deck plan's "lift down"); plain Use takes the one up.
    pub down: bool,
}

/// A body moving along set points (a ladder, a hatch): each leg's end and its seconds.
struct Action {
    legs: Vec<([f32; 3], f32)>,
    leg: usize,
    t: f32,
    from: [f32; 3],
}

/// A crew member's body on the decks.
pub struct Body {
    /// The feet, ship coordinates, metres.
    pub feet: [f32; 3],
    vel: [f32; 3],
    /// On a floor.
    pub grounded: bool,
    jump_t: f32,
    action: Option<Action>,
    kcc: KinematicCharacterController,
    shape: SharedShape,
    lift: f32,
    step_m: f32,
    step_depth_m: f32,
}

impl Body {
    /// A body standing at `at` (its feet).
    pub fn new(at: [f32; 3], d: &WalkData) -> Self {
        let b = &d.body;
        let k = &d.controller;
        let (r, h) = (b.radius_m as f32, b.height_m as f32);
        let kcc = KinematicCharacterController {
            up: Vector::Y,
            offset: CharacterLength::Absolute(k.offset_m as f32),
            slide: true,
            // Rapier's own autostep is off: it first casts the body straight up with the skin gap as its tolerance,
            // and a body standing against a riser at that gap always counts as blocked, so it never steps (0.36).
            // `step_up` below does it instead.
            autostep: None,
            max_slope_climb_angle: (k.climb_deg as f32).to_radians(),
            min_slope_slide_angle: (k.slide_deg as f32).to_radians(),
            snap_to_ground: Some(CharacterLength::Absolute(k.snap_m as f32)),
            ..KinematicCharacterController::default()
        };
        Self {
            feet: at,
            vel: [0.0; 3],
            grounded: true,
            jump_t: 0.0,
            action: None,
            kcc,
            shape: SharedShape::capsule_y(h / 2.0 - r, r),
            lift: h / 2.0 + k.offset_m as f32,
            step_m: b.step_m as f32,
            step_depth_m: k.step_depth_m as f32,
        }
    }

    /// A body standing where a deck's walk starts.
    pub fn at_start(s: &WalkStart, d: &WalkData) -> Self {
        Self::new(s.at_m, d)
    }

    /// The horizontal speed, m/s.
    pub fn speed(&self) -> f32 {
        self.vel[0].hypot(self.vel[2])
    }

    /// On a ladder or going through a hatch.
    pub fn climbing(&self) -> bool {
        self.action.is_some()
    }

    /// Move the body by `dt_s` seconds of `input`, in substeps of `substep_s`.
    pub fn step(&mut self, w: &WalkWorld, d: &WalkData, input: &Input, dt_s: f64) {
        let n = (dt_s / d.substep_s).ceil().max(1.0) as usize;
        let h = (dt_s / n as f64) as f32;
        let mut once = *input;
        for _ in 0..n {
            self.substep(w, d, &once, h);
            once.jump = false;
            once.use_ = false;
        }
    }

    fn substep(&mut self, w: &WalkWorld, d: &WalkData, i: &Input, h: f32) {
        self.jump_t = (self.jump_t - h).max(0.0);
        if self.action.is_some() {
            self.act(h);
            return;
        }
        if i.use_ && self.use_(w, d, i.down) {
            return;
        }
        let in_lift = w.lift_at(self.feet[0], self.feet[2]).map(|l| l.car_m);
        if i.jump && self.grounded && in_lift.is_none() {
            self.vel[1] = d.jump.speed_m_s as f32;
            self.grounded = false;
            self.jump_t = d.jump.lift_s as f32;
        }
        // The wanted horizontal velocity: forward and right in the plan, at walking or running speed.
        let (mut f, mut s) = (i.forward.clamp(-1.0, 1.0), i.right.clamp(-1.0, 1.0));
        let mag = f.hypot(s);
        if mag > 1.0 {
            f /= mag;
            s /= mag;
        }
        let m = &d.r#move;
        let mut speed = if i.run { m.run_m_s } else { m.walk_m_s } as f32;
        if f < -0.1 {
            speed *= m.back_scale as f32;
        }
        let (fx, fz) = (i.yaw.sin(), i.yaw.cos());
        let (rx, rz) = (-fz, fx);
        let (tx, tz) = ((fx * f + rx * s) * speed, (fz * f + rz * s) * speed);
        if self.grounded {
            let (mut dx, mut dz) = (tx - self.vel[0], tz - self.vel[2]);
            // Braking (slowing, or turning against the way the body moves) takes the stopping rate.
            let braking = tx * tx + tz * tz < self.vel[0].powi(2) + self.vel[2].powi(2)
                || tx * self.vel[0] + tz * self.vel[2] < 0.0;
            let cap = (if braking { m.stop_m_s2 } else { m.accel_m_s2 }) as f32 * h;
            let dl = dx.hypot(dz);
            if dl > cap {
                dx *= cap / dl;
                dz *= cap / dl;
            }
            self.vel[0] += dx;
            self.vel[2] += dz;
        }
        // In a lift's shaft nothing is under the feet but the car: no pull down while standing.
        let g = d.gravity_m_s2 as f32;
        self.vel[1] = if self.grounded {
            if in_lift.is_some() {
                0.0
            } else {
                -0.5
            }
        } else {
            (self.vel[1] - g * h).max(-(d.fall_max_m_s as f32))
        };
        let desired = Vector::new(self.vel[0] * h, self.vel[1] * h, self.vel[2] * h);
        let pose = Pose::from_translation(Vector::new(self.feet[0], self.feet[1] + self.lift, self.feet[2]));
        let mut walls = Vec::new();
        let q = w.queries();
        let mut mv = self.kcc.move_shape(h, &q, &*self.shape, &pose, desired, |c| walls.push(c.hit.normal1));
        // Stopped by a wall (a near-vertical contact): perhaps a ledge it can step onto.
        let walled = walls.iter().any(|n| n.y.abs() < 0.3);
        if self.grounded && walled && in_lift.is_none() {
            if let Some(up) = self.step_up(&q, &pose, desired, mv.translation) {
                mv.translation = up;
                mv.grounded = true;
            }
        }
        let mut c = pose.translation + mv.translation;
        let on_car = in_lift.is_some_and(|car| c.y - self.lift <= car + 1e-4);
        if let Some(car) = in_lift.filter(|_| on_car) {
            c.y = car + self.lift;
        }
        self.feet = [c.x, c.y - self.lift, c.z];
        self.grounded = (on_car || mv.grounded) && self.jump_t <= 0.0;
        if self.grounded {
            self.vel[1] = 0.0;
        } else if self.vel[1] > 0.0 && mv.translation.y < self.vel[1] * h * 0.5 {
            self.vel[1] = 0.0; // the head met a ceiling
        }
        // A wall (a near-vertical contact): drop the part of the velocity into it, keep the part along it.
        for n in walls {
            let hl = n.x.hypot(n.z);
            if n.y.abs() > 0.3 || hl < 1e-6 {
                continue;
            }
            let (ux, uz) = (n.x / hl, n.z / hl);
            let into = self.vel[0] * ux + self.vel[2] * uz;
            if into < 0.0 {
                self.vel[0] -= into * ux;
                self.vel[2] -= into * uz;
            }
        }
    }

    /// A grounded body stopped short by a ledge no higher than a step: lifted by the step, moved again, and set back
    /// down on what it reached. The translation that got there, or `None` (no ledge, too high, or no room on top).
    fn step_up(&self, q: &QueryPipeline<'_>, pose: &Pose, desired: Vector, got: Vector) -> Option<Vector> {
        let want = Vector::new(desired.x, 0.0, desired.z);
        let (wl, gl) = (want.length(), Vector::new(got.x, 0.0, got.z).length());
        if wl < 1e-5 || gl > wl * 0.6 {
            return None; // not stopped short
        }
        let opts = |max: f32| ShapeCastOptions {
            max_time_of_impact: max,
            target_distance: 0.0,
            stop_at_penetration: false,
            compute_impact_geometry_on_penetration: false,
        };
        // Room overhead to rise by the step.
        if q.cast_shape(pose, Vector::Y, &*self.shape, opts(self.step_m)).is_some() {
            return None;
        }
        let raised = Pose::from_translation(pose.translation + Vector::Y * self.step_m);
        // Across, at the raised height, by at least the step's depth: a ledge narrower than that is not a step.
        let across = want * (self.step_depth_m / wl).max(1.0);
        let free = q
            .cast_shape(&raised, across.normalize(), &*self.shape, opts(across.length()))
            .map_or(across.length(), |(_, hit)| hit.time_of_impact);
        if free < self.step_depth_m.min(across.length()) {
            return None;
        }
        // A step's depth ahead, down onto what is there, no further than the step: the body is carried that far onto it
        // in the one substep (a few centimetres more than it would have walked, hidden as the eye follows the rise).
        let onto = across.normalize() * self.step_depth_m.min(free);
        let (_, hit) = q.cast_shape(
            &Pose::from_translation(raised.translation + onto),
            -Vector::Y,
            &*self.shape,
            opts(self.step_m + 0.05),
        )?;
        let rise = self.step_m - hit.time_of_impact;
        if !(0.005..=self.step_m).contains(&rise) || hit.normal1.y < 0.7 {
            return None; // no higher floor ahead, or one too steep to stand on
        }
        // Standing there must overlap nothing (a raised body cast down can start inside a slope or a wall).
        let there = Pose::from_translation(pose.translation + Vector::new(onto.x, rise + 0.01, onto.z));
        if q.intersect_shape(there, &*self.shape).next().is_some() {
            return None;
        }
        Some(Vector::new(onto.x, rise, onto.z))
    }

    /// Use: climb a ladder from either floor, or go through a wall hatch. True when it took.
    fn use_(&mut self, w: &WalkWorld, d: &WalkData, down: bool) -> bool {
        let [x, y, z] = self.feet;
        let reach = d.ladder.reach_m as f32;
        // The ladders in reach whose end is this floor: going up from their foot first, or down from their top with `down`.
        let mut order: Vec<&WalkLadder> = w.ladders.iter().collect();
        order.sort_by_key(|l| ((y - l.hi_m).abs() < 0.4) != down);
        for l in order {
            if (l.x_m - x).hypot(l.z_m - z) > reach {
                continue;
            }
            let up = (y - l.lo_m).abs() < 0.4;
            if !up && (y - l.hi_m).abs() >= 0.4 {
                continue;
            }
            let (a, b) = if up { (l.lo_m, l.hi_m) } else { (l.hi_m, l.lo_m) };
            let speed = if up { d.ladder.up_m_s } else { d.ladder.down_m_s } as f32;
            self.start(vec![
                ([l.x_m, a, l.z_m], d.ladder.mount_s as f32),
                ([l.x_m, b, l.z_m], (b - a).abs() / speed),
                ([l.x_m, b, l.z_m], d.ladder.dismount_s as f32),
            ]);
            return true;
        }
        for hch in &w.hatches {
            if (hch.at[0] - x).hypot(hch.at[1] - z) > reach + 0.2 {
                continue;
            }
            let s = usize::from((x - hch.at[0]) * hch.normal[0] + (z - hch.at[1]) * hch.normal[1] >= 0.0);
            if (y - hch.sides[s][1]).abs() < 0.5 {
                self.start(vec![(hch.sides[1 - s], d.side_hatch_s as f32)]);
                return true;
            }
        }
        false
    }

    fn start(&mut self, legs: Vec<([f32; 3], f32)>) {
        self.action = Some(Action { legs, leg: 0, t: 0.0, from: self.feet });
        self.vel = [0.0; 3];
    }

    fn act(&mut self, h: f32) {
        let Some(a) = self.action.as_mut() else { return };
        let (to, dur) = a.legs[a.leg];
        a.t += h;
        let k = if dur > 0.0 { (a.t / dur).min(1.0) } else { 1.0 };
        self.feet = std::array::from_fn(|i| a.from[i] + (to[i] - a.from[i]) * k);
        if k < 1.0 {
            return;
        }
        a.leg += 1;
        a.t = 0.0;
        a.from = self.feet;
        if a.leg >= a.legs.len() {
            self.action = None;
            self.grounded = true;
            self.vel = [0.0; 3];
        }
    }
}

/// The eye's height over a step: it follows the feet with a time constant and never lags more than `lag_m`.
pub fn eye_follow(eye_y: f32, feet_y: f32, dt_s: f32, d: &WalkData, straight: bool) -> f32 {
    if straight {
        return feet_y;
    }
    let e = eye_y + (feet_y - eye_y) * (1.0 - (-dt_s / d.eye.tau_s.max(1e-6) as f32).exp());
    let lag = d.eye.lag_m as f32;
    e.clamp(feet_y - lag, feet_y + lag)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data() -> WalkData {
        crate::data::parse("data/crew/walk.json", include_str!("../../../data/crew/walk.json"))
            .expect("the shipped walk data is valid")
    }

    /// A quad as two triangles: corners `a b c d` in order.
    fn quad(out: &mut Vec<[f32; 9]>, a: [f32; 3], b: [f32; 3], c: [f32; 3], d: [f32; 3]) {
        out.push([a[0], a[1], a[2], b[0], b[1], b[2], c[0], c[1], c[2]]);
        out.push([a[0], a[1], a[2], c[0], c[1], c[2], d[0], d[1], d[2]]);
    }

    /// A floor 40 x 40 m at y 0, with whatever else the test adds.
    fn floor() -> Vec<[f32; 9]> {
        let mut t = Vec::new();
        quad(&mut t, [-20.0, 0.0, -20.0], [-20.0, 0.0, 20.0], [20.0, 0.0, 20.0], [20.0, 0.0, -20.0]);
        t
    }

    fn world(t: &[[f32; 9]], e: &WalkEntities) -> WalkWorld {
        WalkWorld::new(t, e, &data()).expect("the test world builds")
    }

    fn run(b: &mut Body, w: &WalkWorld, d: &WalkData, i: Input, s: f64) {
        let mut t = 0.0;
        while t < s {
            b.step(w, d, &i, 1.0 / 60.0);
            t += 1.0 / 60.0;
        }
    }

    const NORTH: Input =
        Input { forward: 1.0, right: 0.0, yaw: 0.0, run: false, jump: false, use_: false, down: false };

    #[test]
    fn the_shipped_walk_data_loads_and_a_misspelt_key_is_refused() {
        let d = data();
        assert_eq!(d.body.radius_m, 0.25);
        let bad = include_str!("../../../data/crew/walk.json").replace("\"walk_m_s\"", "\"walk_ms\"");
        assert!(crate::data::parse::<WalkData>("walk.json", &bad).is_err(), "a misspelt knob must stop the load");
    }

    #[test]
    fn a_body_reaches_walking_speed_in_about_a_tenth_of_a_second_and_stops_as_quickly() {
        let d = data();
        let w = world(&floor(), &WalkEntities::default());
        let mut b = Body::new([0.0, 0.0, 0.0], &d);
        run(&mut b, &w, &d, NORTH, 0.12);
        assert!((b.speed() - 1.8).abs() < 0.05, "1.8 m/s at 20 m/s^2 takes 0.09 s, not ice skating: {}", b.speed());
        run(&mut b, &w, &d, Input::default(), 0.08);
        assert!(b.speed() < 0.01, "braking at 30 m/s^2 stops it in 0.06 s: {}", b.speed());
        assert!(b.grounded && b.feet[1].abs() < 0.02, "it stays on the floor: {:?}", b.feet);
    }

    #[test]
    fn a_wall_stops_a_body_and_it_slides_along_it() {
        let d = data();
        let mut t = floor();
        quad(&mut t, [-5.0, 0.0, 2.0], [5.0, 0.0, 2.0], [5.0, 3.0, 2.0], [-5.0, 3.0, 2.0]);
        let w = world(&t, &WalkEntities::default());
        let mut b = Body::new([0.0, 0.0, 0.0], &d);
        run(&mut b, &w, &d, NORTH, 2.0);
        assert!(b.feet[2] < 2.0 - 0.24 && b.feet[2] > 1.6, "it stands against the wall, not through it: {:?}", b.feet);
        // Walking into it at 45 degrees, it slides along: x keeps moving.
        let x0 = b.feet[0];
        run(&mut b, &w, &d, Input { yaw: std::f32::consts::FRAC_PI_4, ..NORTH }, 1.0);
        assert!(b.feet[0] - x0 > 0.6, "it slides along the wall: {:?}", b.feet);
    }

    #[test]
    fn a_body_walks_up_a_stair_ramp_and_a_step() {
        let d = data();
        let mut t = floor();
        // A 41 degree ramp from z 2 to z 4.3, rising 2 m, and a landing at 2 m.
        quad(&mut t, [-2.0, 0.0, 2.0], [-2.0, 2.0, 4.3], [2.0, 2.0, 4.3], [2.0, 0.0, 2.0]);
        quad(&mut t, [-2.0, 2.0, 4.3], [-2.0, 2.0, 9.0], [2.0, 2.0, 9.0], [2.0, 2.0, 4.3]);
        // A 0.3 m step to the east.
        quad(&mut t, [5.0, 0.3, -3.0], [5.0, 0.3, 3.0], [9.0, 0.3, 3.0], [9.0, 0.3, -3.0]);
        quad(&mut t, [5.0, 0.0, -3.0], [5.0, 0.3, -3.0], [5.0, 0.3, 3.0], [5.0, 0.0, 3.0]);
        let w = world(&t, &WalkEntities::default());
        let mut b = Body::new([0.0, 0.0, 0.0], &d);
        // On the slope the controller keeps the speed along the surface, so the plan sees about 1.3 m/s.
        run(&mut b, &w, &d, NORTH, 6.0);
        assert!((b.feet[1] - 2.0).abs() < 0.05 && b.feet[2] > 4.6, "up the ramp to the landing: {:?}", b.feet);
        let mut b = Body::new([3.0, 0.0, 0.0], &d);
        run(&mut b, &w, &d, Input { yaw: std::f32::consts::FRAC_PI_2, ..NORTH }, 2.0);
        assert!((b.feet[1] - 0.3).abs() < 0.03 && b.feet[0] > 5.3, "onto the step: {:?}", b.feet);
    }

    #[test]
    fn a_body_falls_off_a_ledge_and_lands() {
        let d = data();
        let mut t = Vec::new();
        quad(&mut t, [-5.0, 2.0, -5.0], [-5.0, 2.0, 1.0], [5.0, 2.0, 1.0], [5.0, 2.0, -5.0]);
        quad(&mut t, [-20.0, 0.0, -20.0], [-20.0, 0.0, 20.0], [20.0, 0.0, 20.0], [20.0, 0.0, -20.0]);
        let w = world(&t, &WalkEntities::default());
        let mut b = Body::new([0.0, 2.0, 0.0], &d);
        run(&mut b, &w, &d, NORTH, 2.5);
        assert!(b.feet[1].abs() < 0.03 && b.grounded, "it came down to the floor below: {:?}", b.feet);
    }

    #[test]
    fn a_jump_rises_about_half_a_metre() {
        let d = data();
        let w = world(&floor(), &WalkEntities::default());
        let mut b = Body::new([0.0, 0.0, 0.0], &d);
        run(&mut b, &w, &d, Input::default(), 0.1);
        let mut top = 0f32;
        b.step(&w, &d, &Input { jump: true, ..Input::default() }, 1.0 / 60.0);
        for _ in 0..90 {
            b.step(&w, &d, &Input::default(), 1.0 / 60.0);
            top = top.max(b.feet[1]);
        }
        assert!((top - 0.46).abs() < 0.06, "3.0 m/s up at 9.81 m/s^2 is 0.46 m: {top}");
        assert!(b.grounded && b.feet[1].abs() < 0.03, "and it lands: {:?}", b.feet);
    }

    #[test]
    fn a_ladder_takes_a_body_to_the_floor_above() {
        let d = data();
        let mut t = floor();
        // An upper floor at 3.5 m over the ladder, its hatch covered as the deck plan's walk covers every opening.
        quad(&mut t, [-6.0, 3.5, -1.0], [-6.0, 3.5, 6.0], [6.0, 3.5, 6.0], [6.0, 3.5, -1.0]);
        let e = WalkEntities {
            ladders: vec![WalkLadder { x_m: 0.0, z_m: 0.5, lo_m: 0.0, hi_m: 3.5 }],
            ..WalkEntities::default()
        };
        let w = world(&t, &e);
        let mut b = Body::new([0.0, 0.0, 0.0], &d);
        b.step(&w, &d, &Input { use_: true, ..Input::default() }, 1.0 / 60.0);
        assert!(b.climbing(), "Use at the ladder's foot starts the climb");
        run(&mut b, &w, &d, Input::default(), 2.5);
        assert!(!b.climbing() && (b.feet[1] - 3.5).abs() < 0.01, "at the top after 0.15 + 1.75 + 0.15 s: {:?}", b.feet);
    }

    #[test]
    fn where_two_ladders_meet_use_climbs_up_and_use_down_climbs_down() {
        let d = data();
        let mut t = floor();
        quad(&mut t, [-6.0, 3.5, -1.0], [-6.0, 3.5, 6.0], [6.0, 3.5, 6.0], [6.0, 3.5, -1.0]);
        quad(&mut t, [-6.0, -3.5, -6.0], [-6.0, -3.5, 6.0], [6.0, -3.5, 6.0], [6.0, -3.5, -6.0]);
        let e = WalkEntities {
            ladders: vec![
                WalkLadder { x_m: 0.0, z_m: 0.5, lo_m: 0.0, hi_m: 3.5 },
                WalkLadder { x_m: 0.0, z_m: 0.5, lo_m: -3.5, hi_m: 0.0 },
            ],
            ..WalkEntities::default()
        };
        let w = world(&t, &e);
        for (down, want) in [(false, 3.5), (true, -3.5)] {
            let mut b = Body::new([0.0, 0.0, 0.0], &d);
            b.step(&w, &d, &Input { use_: true, down, ..Input::default() }, 1.0 / 60.0);
            run(&mut b, &w, &d, Input::default(), 2.5);
            assert!(
                (b.feet[1] - want).abs() < 0.01,
                "a trunk through three decks: down {down} ends at {want}: {:?}",
                b.feet
            );
        }
    }

    #[test]
    fn a_lift_landing_without_the_car_is_a_wall_and_the_car_holds_the_feet() {
        let d = data();
        let mut t = floor();
        // An upper floor at 3.5 m beside a shaft (x -3..-1, z -1..1) with no floor in it.
        quad(&mut t, [-1.0, 3.5, -5.0], [-1.0, 3.5, 5.0], [6.0, 3.5, 5.0], [6.0, 3.5, -5.0]);
        let lift = WalkLift {
            poly: vec![[-3.0, -1.0], [-1.0, -1.0], [-1.0, 1.0], [-3.0, 1.0]],
            stops_m: vec![0.0, 3.5],
            car_m: 3.5,
        };
        let door = |y: f32| WalkDoor {
            id: format!("p_lift_{y}"),
            kind: "lift".into(),
            center_m: [-1.0, y + 1.1, 0.0],
            normal: [1.0, 0.0],
            width_m: 1.2,
            height_m: 2.2,
        };
        let e = WalkEntities { doors: vec![door(0.0), door(3.5)], lifts: vec![lift], ..WalkEntities::default() };
        let w = world(&t, &e);
        let west = Input { yaw: -std::f32::consts::FRAC_PI_2, ..NORTH };
        // Upstairs the car stands: the body walks in and stands on it.
        let mut b = Body::new([1.0, 3.5, 0.0], &d);
        run(&mut b, &w, &d, west, 2.0);
        assert!(b.feet[0] < -1.5 && (b.feet[1] - 3.5).abs() < 0.01, "into the car, on its floor: {:?}", b.feet);
        // Downstairs it does not: the landing is shut.
        let mut b = Body::new([1.0, 0.0, 0.0], &d);
        run(&mut b, &w, &d, west, 2.0);
        assert!(b.feet[0] > -1.0 && b.feet[1].abs() < 0.01, "the shaft is walled where the car is not: {:?}", b.feet);
    }
}
