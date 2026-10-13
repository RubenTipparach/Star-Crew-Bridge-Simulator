//! What a fire and its fighting look like inside the ship (all-hands 4; the owner, 2026-10-12: "fire is also super bare
//! bones", "we had a fire extinguisher model, please use it from the JS mockups"). After docs/mockups/lib/firefight.js:
//! flames standing across a burning room's open floor, embers under them and on the floor after, smoke rising to pool
//! under the ceiling, a haze over the eye in a burning room, and an extinguisher's agent in puffs. All are
//! camera-facing quads for sc-render's effects program (`shaders/fx.glsl`), one draw a frame.
//!
//! It lives in the client because it is only a picture: the fire is the server's (`sc-core::combat::damage`) and nothing
//! here decides an outcome. Where flames stand comes from a fixed hash of the room and the cell, so every client draws a
//! fire alike; the agent's puffs are a picture of each extinguisher the snapshot says is spraying.

use glam::{DVec3, Mat4, Vec3, Vec4};
use sc_core::combat::damage::Room;
use sc_render::FxVertex;
use std::collections::BTreeMap;

/// How far from a room's floor point (its largest brush's middle) its flames stand, metres, and how many at most: the
/// mockup's flames stand on burning half-metre cells; until the engine has fire-spread's cells, a fire is a room's.
const SPREAD_M: f32 = 3.5;
const MAX_FLAMES: usize = 18;
/// A flame's height at the fire's seat, metres (fire-spread's `flame_height_m` at peak), and at the spread's edge.
const FLAME_SEAT_M: f32 = 1.6;
const FLAME_EDGE_M: f32 = 0.7;
/// Seconds a fire takes to grow to its full flames once lit, and its flames to drop once out.
const GROW_S: f64 = 4.0;
const DROP_S: f64 = 0.6;
/// Seconds embers glow, and smoke hangs, after a fire is out.
const EMBERS_S: f64 = 25.0;
const SMOKE_AFTER_S: f64 = 12.0;
/// Smoke puffs a burning room sends up, and the seconds each takes from the flames to the ceiling and away along it.
const SMOKE_PUFFS: usize = 16;
const SMOKE_RISE_S: f64 = 7.0;
/// The wall extinguishers' yellow breath while a fire burns aboard: seconds a breath, how far toward the yellow at its
/// height (0-1), and how fast it starts and stops (per second), so it is a soft pulse and never a blink.
const BREATH_S: f64 = 1.6;
const BREATH_MAX: f32 = 0.85;
const ALARM_RATE: f32 = 1.0;
/// A burning room's haze over the eye at its thickest (0-1), and how fast it comes and goes (per second).
const HAZE: f32 = 0.34;
const HAZE_RATE: f32 = 0.6;

/// The agent: the mockup's 220 puffs, 260 a second from a nozzle at 6.5 m/s, each living 0.55 s, in a 30 degree cone
/// (atmosphere.json `fire.extinguisher.cone_deg`; the picture's, not the rule's: the server counts hands, not puffs).
const PUFFS: usize = 220;
const PUFFS_PER_S: f32 = 260.0;
const PUFF_SPEED_M_S: f32 = 6.5;
const PUFF_LIFE_S: f32 = 0.55;
const PUFF_CONE_DEG: f32 = 30.0;
/// How high over the floor the cone aims (fire-spread's `aim_height_m`, the flames' base), metres.
const AIM_HEIGHT_M: f32 = 0.2;

/// The extinguisher in a player's hand, in the eye's frame: the mockup's `handExt` (at 0.34, -0.6, -0.78, turned 0.25
/// about x and -0.35 about y, scaled 0.75).
pub fn held_model() -> Mat4 {
    Mat4::from_translation(Vec3::new(0.34, -0.6, -0.78))
        * Mat4::from_rotation_x(0.25)
        * Mat4::from_rotation_y(-0.35)
        * Mat4::from_scale(Vec3::splat(0.75))
}

/// The eye's view turned: the camera's frame to the ship's (the bridge's `look_to` at the eye, as `drill_bridge` draws).
fn eye_to_ship(dir: Vec3) -> Mat4 {
    glam::camera::rh::view::look_to_mat4(Vec3::ZERO, dir, Vec3::Y).inverse()
}

/// The direction an eye at `yaw` and `pitch` (radians) looks, as the bridge draws it.
pub fn look_dir(yaw: f32, pitch: f32) -> Vec3 {
    Vec3::new(yaw.sin() * pitch.cos(), pitch.sin(), yaw.cos() * pitch.cos())
}

/// A nozzle spraying: where the agent leaves and which way it goes (ship frame, metres), and the floor under it.
#[derive(Debug, Clone, Copy)]
pub struct Nozzle {
    /// Where the agent leaves.
    pub from: [f32; 3],
    /// Which way (any length).
    pub dir: [f32; 3],
    /// The floor's height there.
    pub floor_y: f32,
}

/// This player's nozzle, from the extinguisher in hand (`held_model`) toward where the crosshair meets the flames' base
/// (`AIM_HEIGHT_M` over the feet), else 4 m straight on: the mockup's `nozzleAim`.
pub fn held_nozzle(eye: [f64; 3], yaw: f32, pitch: f32, feet_y: f32) -> Nozzle {
    let dir = look_dir(yaw, pitch);
    let tip = crate::ships3d::EXTINGUISHER_NOZZLE_M;
    let at = eye_to_ship(dir) * held_model() * Vec4::new(tip[0], tip[1], tip[2], 1.0);
    let e = Vec3::new(eye[0] as f32, eye[1] as f32, eye[2] as f32);
    let from = e + at.truncate();
    let base = feet_y + AIM_HEIGHT_M;
    let to = if dir.y < -0.02 { e + dir * ((base - e.y) / dir.y) } else { e + dir * 4.0 };
    Nozzle { from: from.to_array(), dir: (to - from).to_array(), floor_y: feet_y }
}

/// Another body's nozzle (a crewmate spraying, seen by this player): from its hands, ahead and down at the floor 2 m
/// before it.
pub fn body_nozzle(feet: [f32; 3], yaw: f32) -> Nozzle {
    let fwd = Vec3::new(yaw.sin(), 0.0, yaw.cos());
    let from = Vec3::from_array(feet) + Vec3::Y * 1.05 + fwd * 0.45;
    let to = Vec3::from_array(feet) + Vec3::Y * AIM_HEIGHT_M + fwd * 2.4;
    Nozzle { from: from.to_array(), dir: (to - from).to_array(), floor_y: feet[1] }
}

/// One flame: where it stands (ship frame), its height and width (metres), its noise phase and heat (0-1).
#[derive(Debug, Clone, Copy, PartialEq)]
struct Flame {
    at: [f32; 3],
    h: f32,
    w: f32,
    phase: f32,
    heat: f32,
}

/// A room's fire as drawn.
#[derive(Debug, Clone)]
struct RoomFire {
    flames: Vec<Flame>,
    /// The room's ceiling, metres: where its smoke pools.
    ceiling: f32,
    /// When it was lit, and when it went out, local seconds.
    lit_at: f64,
    out_at: Option<f64>,
}

/// A puff of agent in the air.
#[derive(Debug, Clone, Copy, Default)]
struct Puff {
    at: [f32; 3],
    v: [f32; 3],
    /// 0 when it leaves the nozzle, 1 gone.
    age: f32,
    floor_y: f32,
}

/// Every room's fire as drawn, the haze over the eye, and the agent in the air.
pub struct FireFx {
    /// Each room's open floor, where its flames stand (`open_floors`), by the layout's `rooms`.
    floors: Vec<Vec<[f32; 3]>>,
    rooms: BTreeMap<usize, RoomFire>,
    haze: f32,
    /// 0-1: how much the wall extinguishers breathe yellow, rising while a fire burns aboard and falling after.
    alarm: f32,
    puffs: Vec<Puff>,
    next: usize,
    owed: f32,
    rng: u32,
}

impl Default for FireFx {
    fn default() -> Self {
        Self {
            floors: Vec::new(),
            rooms: BTreeMap::new(),
            haze: 0.0,
            alarm: 0.0,
            puffs: vec![Puff { age: 1.0, ..Puff::default() }; PUFFS],
            next: 0,
            owed: 0.0,
            rng: 0x9e37_79b9,
        }
    }
}

/// A number in 0-1 from a room, a cell and a purpose: the same on every client.
fn hash(room: usize, i: usize, purpose: u32) -> f32 {
    let mut x = (room as u32).wrapping_mul(0x9e37_79b9)
        ^ (i as u32).wrapping_mul(0x85eb_ca6b)
        ^ purpose.wrapping_mul(0xc2b2_ae35);
    x ^= x >> 16;
    x = x.wrapping_mul(0x7feb_352d);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846c_a68b);
    x ^= x >> 16;
    (x >> 8) as f32 / (1u32 << 24) as f32
}

/// Where room `k`'s flames stand: on its open floor (`open_floors`), about the open floor point nearest its floor
/// point (a room's middle may be under a machine), nearest first with a hash breaking the lattice's rows, tallest at
/// the seat; with no open floor known, round its floor point.
fn flames_of(k: usize, room: &Room, floor: &[[f32; 3]]) -> Vec<Flame> {
    let mid = room.floor.as_vec3().to_array();
    let inside = |p: &[f32; 3]| room.contains(DVec3::new(f64::from(p[0]), f64::from(p[1]) + 0.3, f64::from(p[2])));
    let near_mid = |p: &&[f32; 3]| (p[0] - mid[0]).hypot(p[2] - mid[2]) + (p[1] - mid[1]).abs() * 2.0;
    let seat = floor.iter().min_by(|a, b| near_mid(a).total_cmp(&near_mid(b))).copied().unwrap_or(mid);
    let mut spots: Vec<[f32; 3]> = floor
        .iter()
        .filter(|p| (p[0] - seat[0]).hypot(p[2] - seat[2]) <= SPREAD_M && (p[1] - seat[1]).abs() < 1.5)
        .copied()
        .collect();
    if spots.is_empty() {
        spots.push(seat);
    }
    let dist = |p: &[f32; 3]| (p[0] - seat[0]).hypot(p[2] - seat[2]);
    let mut keyed: Vec<(f32, [f32; 3])> =
        spots.iter().enumerate().map(|(i, p)| (dist(p) + hash(k, i, 1) * 1.4, *p)).collect();
    keyed.sort_by(|a, b| a.0.total_cmp(&b.0));
    keyed
        .into_iter()
        .take(MAX_FLAMES)
        .enumerate()
        .map(|(i, (_, p))| {
            let edge = (dist(&p) / SPREAD_M).min(1.0);
            let heat = (1.0 - edge) * 0.6 + 0.4 * hash(k, i, 2);
            // A little off its cell, unless that is out of the room.
            let off = [p[0] + (hash(k, i, 3) - 0.5) * 0.24, p[1], p[2] + (hash(k, i, 4) - 0.5) * 0.24];
            let at = if inside(&off) { off } else { p };
            Flame {
                at: [at[0], at[1] + 0.01, at[2]],
                h: (FLAME_SEAT_M + (FLAME_EDGE_M - FLAME_SEAT_M) * edge) * (0.75 + 0.5 * hash(k, i, 5)),
                w: 0.55 + 0.35 * heat,
                phase: hash(k, i, 6),
                heat,
            }
        })
        .collect()
}

/// How high over a room's floor something must stand for a flame to stand on it rather than under it (a desk, a bunk,
/// a bench), metres (fire-spread's `flame_on_props_below_m`), and how far over the floor a cell is looked down on from.
const ON_PROPS_BELOW_M: f32 = 0.9;
const LOOK_DOWN_FROM_M: f32 = 2.2;
/// The open floor's lattice, metres.
const FLOOR_CELL_M: f32 = 0.5;

/// Each room's floor where its flames stand (by the layout's `rooms`): the walk grid's floor in it that a walk from
/// `start` reaches (`NavGrid::joined`), where a person can come to the fire; never a pocket walled off from the ship,
/// such as the floor inside the reactor's ring, where flames would burn unseen inside its housing. A room the walk
/// hardly covers (a launch bay, where nobody walks) keeps its open floor (`open_floors`).
pub fn fire_floors(
    open: Vec<Vec<[f32; 3]>>,
    grid: &sc_core::nav::NavGrid,
    start: [f32; 3],
    rooms: &[Room],
) -> Vec<Vec<[f32; 3]>> {
    let joined = grid.joined(start);
    rooms
        .iter()
        .zip(open)
        .map(|(room, open)| {
            let mine: Vec<[f32; 3]> = joined
                .iter()
                .filter(|p| room.contains(DVec3::new(f64::from(p[0]), f64::from(p[1]) + 0.3, f64::from(p[2]))))
                .copied()
                .collect();
            if mine.len() >= MAX_FLAMES / 3 {
                mine
            } else {
                open
            }
        })
        .collect()
}

/// Each room's open floor (by the layout's `rooms`): a point every half metre over its brushes where a look down from
/// head height meets its floor, or the top of something low on it (a desk, a bunk), and never a machine taller than
/// that, read from the deck's drawn geometry. Where the walk reaches, `fire_floors` takes its floor instead.
pub fn open_floors(d: &sc_core::deck::Deck, rooms: &[Room]) -> Vec<Vec<[f32; 3]>> {
    use sc_core::vertex::{DECK_VERTEX_BYTES, FIXED_SCALE};
    // Every drawn surface that is not a wall (a wall is never met by a look straight down), ship frame, bucketed by the
    // lattice cells their plan's bounds cover: every compartment's, since a room's floor and what stands on it may be
    // drawn as another's (deck A is the command suite's, not the layout's rooms).
    let mut tris: Vec<[[f32; 3]; 3]> = Vec::new();
    for c in d.index.compartments.iter().filter(|c| !c.id.starts_with("figure_") && !c.id.starts_with("dock_")) {
        let v = d.vertices_of(c);
        let at = |i: u32| {
            let b = &v[i as usize * DECK_VERTEX_BYTES..];
            let q = |k: usize| f32::from(i16::from_le_bytes([b[k * 2], b[k * 2 + 1]])) / FIXED_SCALE;
            [c.origin_m[0] as f32 + q(0), c.origin_m[1] as f32 + q(1), c.origin_m[2] as f32 + q(2)]
        };
        for t in d.indices_of(c).chunks_exact(3) {
            let (a, b, cc) = (at(t[0]), at(t[1]), at(t[2]));
            let plan = (b[0] - a[0]) * (cc[2] - a[2]) - (b[2] - a[2]) * (cc[0] - a[0]);
            if plan.abs() > 1e-6 {
                tris.push([a, b, cc]);
            }
        }
    }
    let cell = |x: f32| (x / FLOOR_CELL_M).floor() as i32;
    let mut buckets: std::collections::HashMap<(i32, i32), Vec<u32>> = std::collections::HashMap::new();
    for (i, t) in tris.iter().enumerate() {
        let (x0, x1) =
            (t.iter().map(|p| p[0]).fold(f32::MAX, f32::min), t.iter().map(|p| p[0]).fold(f32::MIN, f32::max));
        let (z0, z1) =
            (t.iter().map(|p| p[2]).fold(f32::MAX, f32::min), t.iter().map(|p| p[2]).fold(f32::MIN, f32::max));
        for cx in cell(x0)..=cell(x1) {
            for cz in cell(z0)..=cell(z1) {
                buckets.entry((cx, cz)).or_default().push(i as u32);
            }
        }
    }
    rooms
        .iter()
        .map(|room| {
            // The highest surface under (x, z) between `lo` and `hi`.
            let down = |x: f32, z: f32, lo: f32, hi: f32| -> Option<f32> {
                let mut best: Option<f32> = None;
                for &i in buckets.get(&(cell(x), cell(z))).into_iter().flatten() {
                    let [a, b, c] = tris[i as usize];
                    let det = (b[2] - c[2]) * (a[0] - c[0]) + (c[0] - b[0]) * (a[2] - c[2]);
                    if det.abs() < 1e-9 {
                        continue;
                    }
                    let l1 = ((b[2] - c[2]) * (x - c[0]) + (c[0] - b[0]) * (z - c[2])) / det;
                    let l2 = ((c[2] - a[2]) * (x - c[0]) + (a[0] - c[0]) * (z - c[2])) / det;
                    let l3 = 1.0 - l1 - l2;
                    if l1 < -1e-4 || l2 < -1e-4 || l3 < -1e-4 {
                        continue;
                    }
                    let y = l1 * a[1] + l2 * b[1] + l3 * c[1];
                    if y >= lo && y <= hi && best.is_none_or(|h| y > h) {
                        best = Some(y);
                    }
                }
                best
            };
            let mut out = Vec::new();
            for b in &room.brushes {
                let floor = b.y[0] as f32;
                let (lo, hi) = b.poly.iter().fold(([f64::MAX; 2], [f64::MIN; 2]), |(lo, hi), p| {
                    ([lo[0].min(p[0]), lo[1].min(p[1])], [hi[0].max(p[0]), hi[1].max(p[1])])
                });
                let mut x = (lo[0] as f32 / FLOOR_CELL_M).floor() * FLOOR_CELL_M + FLOOR_CELL_M / 2.0;
                while x < hi[0] as f32 {
                    let mut z = (lo[1] as f32 / FLOOR_CELL_M).floor() * FLOOR_CELL_M + FLOOR_CELL_M / 2.0;
                    while z < hi[1] as f32 {
                        let inside = room.contains(DVec3::new(f64::from(x), f64::from(floor) + 0.3, f64::from(z)));
                        let top = if inside { down(x, z, floor - 0.1, floor + LOOK_DOWN_FROM_M) } else { None };
                        if let Some(y) = top.filter(|y| *y <= floor + ON_PROPS_BELOW_M) {
                            out.push([x, y, z]);
                        }
                        z += FLOOR_CELL_M;
                    }
                    x += FLOOR_CELL_M;
                }
            }
            out
        })
        .collect()
}

/// A quad being sorted: its distance from the eye, and its six vertices.
type Quad = (f32, [FxVertex; 6]);

/// A quad from its four corners (relative to the eye; bottom left, bottom right, top right, top left) and parameters.
fn quad(c: [Vec3; 4], p: [f32; 4]) -> Quad {
    let v = |k: usize, uv: [f32; 2]| FxVertex { pos: c[k].to_array(), uv, p };
    let mid = (c[0] + c[2]) * 0.5;
    (
        mid.length(),
        [v(0, [0.0, 0.0]), v(1, [1.0, 0.0]), v(2, [1.0, 1.0]), v(0, [0.0, 0.0]), v(2, [1.0, 1.0]), v(3, [0.0, 1.0])],
    )
}

/// A quad standing up at `base` (relative to the eye), turned about the vertical to face the eye: a flame or embers.
/// `sway` leans its top sideways, metres.
fn standing(base: Vec3, w: f32, h: f32, sway: f32, p: [f32; 4]) -> Quad {
    let to_eye = Vec3::new(-base.x, 0.0, -base.z);
    let l = to_eye.length();
    let right = if l > 1e-4 { Vec3::new(to_eye.z, 0.0, -to_eye.x) / l } else { Vec3::X };
    let (hw, up) = (right * (w * 0.5), Vec3::Y * h);
    quad([base - hw, base + hw, base + hw + up + right * sway, base - hw + up + right * sway], p)
}

/// A quad facing the eye square on, `s` metres from its middle to an edge: a puff.
fn facing(mid: Vec3, s: f32, right: Vec3, up: Vec3, p: [f32; 4]) -> Quad {
    let (r, u) = (right * s, up * s);
    quad([mid - r - u, mid + r - u, mid + r + u, mid - r + u], p)
}

fn smooth(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

impl FireFx {
    /// Hand it the rooms' open floor (`open_floors`), where their flames stand.
    pub fn set_floors(&mut self, floors: Vec<Vec<[f32; 3]>>) {
        self.floors = floors;
    }

    /// Stand the flames on the walk's floor a walk from `start` reaches, where a room has it (`fire_floors`).
    pub fn join_floors(&mut self, grid: &sc_core::nav::NavGrid, start: [f32; 3], rooms: &[Room]) {
        let open = std::mem::take(&mut self.floors);
        let open = if open.len() == rooms.len() { open } else { vec![Vec::new(); rooms.len()] };
        self.floors = fire_floors(open, grid, start, rooms);
    }

    /// This frame, `dt` seconds at local time `now`: `burning` the rooms on fire (the layout's `rooms`, by index),
    /// `eye` this player's eye (the haze), `nozzles` every extinguisher
    /// spraying.
    #[allow(clippy::too_many_arguments)]
    pub fn step(&mut self, burning: &[usize], rooms: &[Room], now: f64, eye: [f64; 3], nozzles: &[Nozzle], dt: f32) {
        for &k in burning {
            let Some(room) = rooms.get(k) else { continue };
            match self.rooms.get_mut(&k) {
                Some(f) if f.out_at.is_some() => {
                    // Lit again where it burnt.
                    f.out_at = None;
                    f.lit_at = now;
                }
                Some(_) => {}
                None => {
                    let flames = flames_of(k, room, self.floors.get(k).map_or(&[], Vec::as_slice));
                    let rf = RoomFire { flames, ceiling: room.hi.y as f32, lit_at: now, out_at: None };
                    self.rooms.insert(k, rf);
                }
            }
        }
        for (k, f) in self.rooms.iter_mut() {
            if f.out_at.is_none() && !burning.contains(k) {
                f.out_at = Some(now);
            }
        }
        self.rooms.retain(|_, f| f.out_at.is_none_or(|t| now - t < EMBERS_S));
        // The haze: thick in a burning room's air, thinning as its smoke clears.
        let e = DVec3::from_array(eye);
        let want = self
            .rooms
            .iter()
            .filter(|(k, _)| rooms.get(**k).is_some_and(|r| r.contains(e)))
            .map(|(_, f)| HAZE * Self::smoke_left(f, now))
            .fold(0.0f32, f32::max);
        self.haze += (want - self.haze).clamp(-HAZE_RATE * dt, HAZE_RATE * dt);
        let on = if burning.is_empty() { 0.0 } else { 1.0 };
        self.alarm += (on - self.alarm).clamp(-ALARM_RATE * dt, ALARM_RATE * dt);
        self.spray(nozzles, dt);
    }

    /// How much of a room's smoke is left, 0-1: all of it while it burns, clearing after.
    fn smoke_left(f: &RoomFire, now: f64) -> f32 {
        f.out_at.map_or(1.0, |t| (1.0 - (now - t) / SMOKE_AFTER_S).max(0.0) as f32)
    }

    /// A fresh number in 0-1 (xorshift; the agent's puffs only).
    fn rand(&mut self) -> f32 {
        let mut x = self.rng;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.rng = x;
        (x >> 8) as f32 / (1u32 << 24) as f32
    }

    /// The agent: new puffs from each nozzle in its cone, and every puff on, slowing, falling and lying on the floor.
    fn spray(&mut self, nozzles: &[Nozzle], dt: f32) {
        self.owed += PUFFS_PER_S * dt * nozzles.len() as f32;
        let half = (PUFF_CONE_DEG * 0.5).to_radians().tan();
        let mut k = 0;
        while self.owed >= 1.0 && !nozzles.is_empty() {
            self.owed -= 1.0;
            let n = nozzles[k % nozzles.len()];
            k += 1;
            let u = Vec3::from_array(n.dir).normalize_or(Vec3::NEG_Z);
            let o = if u.y.abs() < 0.9 { Vec3::Y } else { Vec3::X };
            let e1 = u.cross(o).normalize();
            let e2 = u.cross(e1);
            let (a, s) = (self.rand() * std::f32::consts::TAU, self.rand().sqrt() * half);
            let v = (u + (e1 * a.cos() + e2 * a.sin()) * s) * PUFF_SPEED_M_S;
            let i = self.next;
            self.next = (self.next + 1) % PUFFS;
            self.puffs[i] = Puff { at: n.from, v: v.to_array(), age: 0.0, floor_y: n.floor_y };
        }
        if nozzles.is_empty() {
            self.owed = 0.0;
        }
        let drag = 0.93f32.powf(dt * 60.0);
        for p in self.puffs.iter_mut().filter(|p| p.age < 1.0) {
            p.age = (p.age + dt / PUFF_LIFE_S).min(1.0);
            for j in 0..3 {
                p.at[j] += p.v[j] * dt;
            }
            p.v = [p.v[0] * drag, p.v[1] * drag - 0.6 * dt, p.v[2] * drag];
            if p.at[1] < p.floor_y + 0.03 {
                p.at[1] = p.floor_y + 0.03;
                p.v[1] = p.v[1].abs() * 0.2;
            }
        }
    }

    /// How far toward their yellow the wall extinguishers are at local time `now`, 0-1: a soft breath while a fire
    /// burns aboard (the owner, 2026-10-12: "fire extenguisher should flash yellow (a soft transition to highlight
    /// them)", "when there is a fire"), nothing otherwise.
    pub fn highlight(&self, now: f64) -> f32 {
        let breath = 0.5 - 0.5 * (now * std::f64::consts::TAU / BREATH_S).cos();
        self.alarm * BREATH_MAX * breath as f32
    }

    /// Whether anything is to be drawn.
    pub fn is_empty(&self) -> bool {
        self.rooms.is_empty() && self.haze <= 0.0 && self.puffs.iter().all(|p| p.age >= 1.0)
    }

    /// The quads to draw for an eye at `eye` looking along `dir`, at local time `now`: farthest first, the haze last.
    pub fn quads(&self, eye: [f64; 3], dir: Vec3, now: f64) -> Vec<FxVertex> {
        let rel = |p: [f32; 3]| {
            Vec3::new(
                (f64::from(p[0]) - eye[0]) as f32,
                (f64::from(p[1]) - eye[1]) as f32,
                (f64::from(p[2]) - eye[2]) as f32,
            )
        };
        let right = dir.cross(Vec3::Y).normalize_or(Vec3::X);
        let up = right.cross(dir);
        let t = now as f32;
        let mut qs: Vec<Quad> = Vec::new();
        for f in self.rooms.values() {
            let grown = ((now - f.lit_at) / GROW_S).clamp(0.15, 1.0) as f32;
            let flame_k = f.out_at.map_or(grown, |o| (1.0 - (now - o) / DROP_S).max(0.0) as f32 * grown);
            let ember_k = f.out_at.map_or(grown, |o| (1.0 - (now - o) / EMBERS_S).max(0.0) as f32);
            let smoke_k = Self::smoke_left(f, now) * grown;
            for fl in &f.flames {
                let base = rel(fl.at);
                // Embers: a low crust wider than the flame.
                if ember_k > 0.01 {
                    qs.push(standing(base + Vec3::Y * 0.01, fl.w * 1.5, 0.16, 0.0, [fl.phase, 0.3, 1.0, ember_k]));
                }
                if flame_k > 0.01 {
                    let breathe = 0.88 + 0.12 * (t * 2.1 + fl.phase * std::f32::consts::TAU).sin();
                    let h = fl.h * flame_k * breathe;
                    let sway = (t * 3.3 + fl.phase * std::f32::consts::TAU).sin() * 0.07 * h;
                    qs.push(standing(base, fl.w, h, sway, [fl.phase, fl.heat, 0.0, 1.0]));
                }
            }
            // Smoke: puffs leaving the flames' tops, rising to the ceiling and spreading under it.
            if smoke_k > 0.01 && !f.flames.is_empty() {
                for i in 0..SMOKE_PUFFS {
                    let ph = f64::from(hash(i, f.flames.len(), 7));
                    let age = (((now - f.lit_at) / SMOKE_RISE_S + ph) % 1.0) as f32;
                    let src = f.flames[i % f.flames.len()];
                    let top = src.at[1] + src.h * 0.8;
                    let roof = f.ceiling - 0.35;
                    let y = top + (roof - top).max(0.0) * smooth(0.0, 0.45, age);
                    let ang = hash(i, 3, 8) * std::f32::consts::TAU;
                    let out = (age - 0.3).max(0.0) * 3.2;
                    let s = 0.3 + 1.1 * age;
                    let at = [src.at[0] + ang.sin() * out, y.min(roof - s * 0.3), src.at[2] + ang.cos() * out];
                    let k = smooth(0.0, 0.12, age) * (1.0 - age) * smoke_k;
                    qs.push(facing(rel(at), s, right, up, [hash(i, 9, 9), (1.0 - age) * 0.8, 2.0, k]));
                }
            }
        }
        for p in self.puffs.iter().filter(|p| p.age < 1.0) {
            let s = 0.03 + 0.12 * p.age;
            qs.push(facing(rel(p.at), s, right, up, [p.age, 0.0, 3.0, 1.0 - p.age]));
        }
        qs.sort_by(|a, b| b.0.total_cmp(&a.0));
        let mut out: Vec<FxVertex> = qs.into_iter().flat_map(|(_, v)| v).collect();
        if self.haze > 0.005 {
            let mid = dir * 0.3;
            out.extend(facing(mid, 1.0, right, up, [0.0, 0.6, 4.0, self.haze]).1);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sc_core::combat::data::DrillData;

    /// A burning room shows flames across its floor, smoke and embers; once out, its flames drop at once and its embers
    /// and smoke fade; and the same fire stands alike on every client.
    #[test]
    fn a_fire_stands_across_its_room_and_dies_down_when_put_out() {
        let data = DrillData::shipped();
        let rooms = &data.tern_layout.rooms;
        let k = data.tern_layout.room("engineering").expect("engineering");
        let mut fx = FireFx::default();
        // An eye outside the ship, looking at the room: no haze.
        let eye = rooms[k].floor.as_vec3().to_array().map(f64::from);
        let eye = [eye[0] + 40.0, eye[1] + 1.6, eye[2]];
        let dir = Vec3::new(-1.0, 0.0, 0.0);
        fx.step(&[k], rooms, 10.0, eye, &[], 1.0 / 30.0);
        let count = |fx: &FireFx, now: f64, kind: f32| {
            fx.quads(eye, dir, now).chunks(6).filter(|q| q[0].p[2] == kind && q[0].p[3] > 0.01).count()
        };
        assert_eq!(count(&fx, 20.0, 0.0), 1, "no open floor known: one flame at its floor point");
        assert!(count(&fx, 20.0, 2.0) > 4, "smoke rising");
        fx.step(&[], rooms, 30.0, eye, &[], 1.0 / 30.0);
        assert_eq!(count(&fx, 31.0, 0.0), 0, "put out, its flames are gone");
        assert_eq!(count(&fx, 31.0, 1.0), 1, "its embers glow on");
        fx.step(&[], rooms, 30.0 + EMBERS_S + 1.0, eye, &[], 1.0 / 30.0);
        assert!(fx.quads(eye, dir, 60.0).is_empty(), "and in the end nothing is left");
        let floor = [[1.0, 0.0, 2.0], [1.5, 0.0, 2.0], [1.0, 0.0, 2.5]];
        assert_eq!(flames_of(k, &rooms[k], &floor), flames_of(k, &rooms[k], &floor), "the same fire on every client");
    }

    /// Every room's flames stand on its own floor, a good few of them, where a person can walk to them: what a player
    /// who walks into a burning room sees, never a flame inside a machine (engineering's reactor ring) or on something
    /// taller than a desk. Needs the compiled deck; skipped without one.
    #[test]
    fn every_rooms_flames_stand_on_its_floor_where_a_walk_comes() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let Ok(bytes) = std::fs::read(root.join("compiled/tern.deck")) else {
            eprintln!("no compiled deck: skipped");
            return;
        };
        let Ok(d) = sc_core::deck::read(&bytes) else {
            eprintln!("a compiled deck of another version: skipped");
            return;
        };
        std::env::set_current_dir(&root).expect("the repository");
        let foot = crate::onfoot::OnFoot::from_deck(&d).expect("the walk");
        let data = DrillData::shipped();
        let rooms = &data.tern_layout.rooms;
        let t0 = std::time::Instant::now();
        let start = d.index.walk.start.at_m;
        let floors = fire_floors(open_floors(&d, rooms), &foot.grid, start, rooms);
        eprintln!("fire floors in {:.0} ms", t0.elapsed().as_secs_f64() * 1000.0);
        let joined = foot.grid.joined(start);
        let mut few = Vec::new();
        for (k, r) in rooms.iter().enumerate() {
            let flames = flames_of(k, r, &floors[k]);
            for fl in &flames {
                let p = DVec3::new(f64::from(fl.at[0]), f64::from(fl.at[1]) + 0.3, f64::from(fl.at[2]));
                assert!(r.contains(p), "{}: a flame outside the room at {:?}", r.id, fl.at);
                let low = fl.at[1] - r.brushes.iter().map(|b| b.y[0] as f32).fold(f32::MAX, f32::min);
                assert!(low <= ON_PROPS_BELOW_M + 0.02, "{}: a flame {low} m up, on something tall", r.id);
            }
            if r.floor_m2() > 20.0 && flames.len() < 6 {
                few.push(format!("{} ({})", r.id, flames.len()));
            }
        }
        assert!(few.is_empty(), "rooms of more than 20 m^2 with fewer than six flames: {}", few.join(", "));
        // Engineering's flames stand where a person can walk to them, never inside the reactor's ring.
        let k = data.tern_layout.room("engineering").expect("engineering");
        for f in flames_of(k, &rooms[k], &floors[k]) {
            let on = |p: &[f32; 3]| (p[0] - f.at[0]).hypot(p[2] - f.at[2]) < 0.4 && (p[1] - f.at[1]).abs() < 0.1;
            assert!(joined.iter().any(on), "a flame where no walk comes: {:?}", f.at);
        }
    }

    /// The owner, 2026-10-12: "fire extenguisher should flash yellow (a soft transition to highlight them)", "when there
    /// is a fire". No fire, no yellow; a fire aboard, a breath to the yellow and back that never jumps; out, it fades.
    #[test]
    fn the_extinguishers_breathe_yellow_while_a_fire_burns_and_softly() {
        let data = DrillData::shipped();
        let rooms = &data.tern_layout.rooms;
        let k = data.tern_layout.room("mess").expect("the mess");
        let mut fx = FireFx::default();
        let dt = 1.0 / 30.0;
        let mut now = 0.0;
        let run = |fx: &mut FireFx, burning: &[usize], secs: f64, now: &mut f64| {
            let mut seen = Vec::new();
            while secs > 0.0 && seen.len() < (secs / dt) as usize {
                fx.step(burning, rooms, *now, [0.0, 100.0, 0.0], &[], dt as f32);
                *now += dt;
                seen.push(fx.highlight(*now));
            }
            seen
        };
        assert!(run(&mut fx, &[], 3.0, &mut now).iter().all(|h| *h == 0.0), "no fire, no yellow");
        let burning = run(&mut fx, &[k], 6.0, &mut now);
        let top = burning.iter().copied().fold(0.0f32, f32::max);
        assert!(top > 0.8, "a fire aboard: the extinguishers go most of the way to yellow ({top})");
        assert!(burning.iter().skip(60).any(|h| *h < 0.05), "and back, a breath and not a glow");
        assert!(burning.windows(2).all(|w| (w[1] - w[0]).abs() < 0.12), "soft: no jump between frames");
        let out = run(&mut fx, &[], 2.0, &mut now);
        assert!(out.windows(2).all(|w| (w[1] - w[0]).abs() < 0.12) && out.last() == Some(&0.0), "out: it fades away");
    }

    /// A nozzle sends its agent along its aim, and the puffs lie down on the floor rather than falling through it.
    #[test]
    fn the_agent_goes_where_the_nozzle_points_and_lies_on_the_floor() {
        let mut fx = FireFx::default();
        let n = Nozzle { from: [0.0, 1.2, 0.0], dir: [0.0, -0.4, -1.0], floor_y: 0.0 };
        for _ in 0..15 {
            fx.step(&[], &[], 0.0, [0.0, 1.6, 0.5], &[n], 1.0 / 30.0);
        }
        let live: Vec<&Puff> = fx.puffs.iter().filter(|p| p.age < 1.0).collect();
        assert!(live.len() > 100, "about 260 a second: {}", live.len());
        assert!(live.iter().all(|p| p.at[1] >= 0.0), "none below the floor");
        let mean_z = live.iter().map(|p| p.at[2]).sum::<f32>() / live.len() as f32;
        assert!(mean_z < -0.5, "ahead along the aim: {mean_z}");
        for _ in 0..30 {
            fx.step(&[], &[], 0.0, [0.0, 1.6, 0.5], &[], 1.0 / 30.0);
        }
        assert!(fx.is_empty(), "stopped spraying, the puffs are gone within their life");
    }
}
