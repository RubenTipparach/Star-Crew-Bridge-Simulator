//! `sc-tools safety-points [ship]`: where the ship's extinguishers hang (openspec/changes/safety-points design 2 and 5a;
//! the owner, 2026-10-12: "there should be an extinguisher in every room and spaced out evenly in corridors").
//!
//! It reads the layout (rooms, doors, seats, fixtures) and the compiled deck's walk world, and places a point inside
//! every room's main door, more in a big room, evenly along every corridor, and wherever a deck's floor is still far
//! from one. Each place is checked against the deck as built (a wall right behind it, nothing standing proud of the
//! wall in front of it, floor to stand on) and against the layout (clear of every door's frame, of the seats and the
//! fixtures), and the first that passes is taken. It writes `data/ships/<ship>/kit.json` and a report.
//!
//! It lives in the tools because only the tools place things from a compiled deck; the game reads the file
//! (`sc-core::combat::damage::ShipLayout::brackets`), and a test fails when the committed file is stale.

use sc_core::walk::{WalkData, WalkEntities, WalkWorld};
use serde_json::{json, Value};
use std::path::Path;

/// A corridor's points are at most this far apart, metres (design 2, rule 3).
const CORRIDOR_SPACING_M: f64 = 8.0;
/// How far along a corridor a point may move from its even place to clear a door, metres (rule 3): a frame bay and a
/// half.
const SHIFT_M: f64 = 3.0;
/// A room takes another point for every this much floor beyond the first, square metres (rule 2).
const BIG_ROOM_M2: f64 = 80.0;
/// No floor point is farther than this from a point, metres (rule 4).
const FAR_M: f64 = 10.0;
/// A point keeps this far from a door's frame, metres (rule 5), and its own half width beyond that.
const DOOR_CLEAR_M: f64 = 0.4;
const HALF_WIDTH_M: f64 = 0.13;
/// A point keeps this far from a station's seat, metres (rule 5).
const SEAT_CLEAR_M: f64 = 1.2;
/// A point keeps this far from a fixture's edge, metres (a fixture with no size counts as a metre across).
const FIXTURE_CLEAR_M: f64 = 0.6;
/// How far into the room a probe looks for the deck's wall, from the layout's wall line, and how far the wall may stand
/// in from that line (a fitted room, the bridge's suite, stands inside its brush), metres.
const FIND_M: f32 = 1.5;
const INSET_MAX_M: f32 = 0.9;
/// How far from the wall the flatness probes start, and how far proud of it anything may stand (panels, frames, the
/// baseboard), metres: anything nearer the probe than that is in the way, and a wall farther back is a recess.
const PROBE_M: f32 = 0.6;
const WALL_PROUD_M: f32 = 0.08;
const WALL_BACK_M: f32 = 0.15;
/// Heights over the floor a point's wall is probed at, metres: the bracket's foot, its handle, the sign.
const PROBE_HEIGHTS_M: [f32; 3] = [0.6, 1.1, 1.8];
/// How far up a point may look for floor in front of it, metres.
const FLOOR_SLOP_M: f32 = 0.3;

/// Where a ship's kit file is, from the repository root.
pub(crate) fn path(ship: &str) -> String {
    format!("data/ships/{ship}/kit.json")
}

/// A place for a point: its wall point (on the room's wall, at its floor) and the way into the room (`[x, z]`).
type Place = ([f64; 3], [f64; 2]);

/// A placed point: its id, room, rule, wall point (on the room's wall, at its floor) and the way into the room.
#[derive(Debug, Clone, PartialEq)]
struct Point {
    id: String,
    room: String,
    rule: &'static str,
    wall: [f64; 3],
    into: [f64; 2],
}

/// The layout's parts the placing reads.
struct Layout {
    rooms: Vec<Room>,
    portals: Vec<Portal>,
    seats: Vec<[f64; 3]>,
    fixtures: Vec<(String, [f64; 3], f64)>,
    decks: Vec<f64>,
    frames: (f64, f64),
    jamb: (f64, f64),
}

struct Room {
    id: String,
    kind: String,
    brushes: Vec<([f64; 2], Vec<[f64; 2]>)>,
}

struct Portal {
    kind: String,
    between: [String; 2],
    centre: [f64; 3],
    normal: [f64; 3],
    size: [f64; 2],
}

fn num(v: &Value, what: &str) -> Result<f64, String> {
    v.as_f64().filter(|x| x.is_finite()).ok_or_else(|| format!("{what}: not a number"))
}

fn vec3(v: &Value, what: &str) -> Result<[f64; 3], String> {
    let a = v.as_array().filter(|a| a.len() == 3).ok_or_else(|| format!("{what}: not three numbers"))?;
    Ok([num(&a[0], what)?, num(&a[1], what)?, num(&a[2], what)?])
}

fn read_layout(root: &Path, ship: &str) -> Result<Layout, String> {
    let rel = format!("data/ships/{ship}/layout.json");
    let text = std::fs::read_to_string(root.join(&rel)).map_err(|e| format!("{rel}: {e}"))?;
    let l: Value = serde_json::from_str(&text).map_err(|e| format!("{rel}: {e}"))?;
    let mut rooms = Vec::new();
    for c in l["compartments"].as_array().ok_or(format!("{rel}: compartments"))? {
        let mut brushes = Vec::new();
        for b in c["brushes"].as_array().into_iter().flatten() {
            let y = b["y"].as_array().ok_or(format!("{rel}: brush y"))?;
            let poly = b["poly"].as_array().ok_or(format!("{rel}: brush poly"))?;
            let poly = poly
                .iter()
                .map(|p| Ok([num(&p[0], &rel)?, num(&p[1], &rel)?]))
                .collect::<Result<Vec<[f64; 2]>, String>>()?;
            brushes.push(([num(&y[0], &rel)?, num(&y[1], &rel)?], poly));
        }
        let id = c["id"].as_str().ok_or(format!("{rel}: compartment id"))?.to_owned();
        rooms.push(Room { id, kind: c["kind"].as_str().unwrap_or("").to_owned(), brushes });
    }
    let mut portals = Vec::new();
    for p in l["portals"].as_array().ok_or(format!("{rel}: portals"))? {
        let b = p["between"].as_array().filter(|b| b.len() == 2).ok_or(format!("{rel}: portal between"))?;
        let s = p["size_m"].as_array().filter(|s| s.len() == 2).ok_or(format!("{rel}: portal size_m"))?;
        portals.push(Portal {
            kind: p["kind"].as_str().unwrap_or("").to_owned(),
            between: [b[0].as_str().unwrap_or("").to_owned(), b[1].as_str().unwrap_or("").to_owned()],
            centre: vec3(&p["center_m"], &rel)?,
            normal: vec3(&p["normal"], &rel)?,
            size: [num(&s[0], &rel)?, num(&s[1], &rel)?],
        });
    }
    let seats =
        l["stations"].as_array().into_iter().flatten().map(|s| vec3(&s["seat_m"], &rel)).collect::<Result<_, _>>()?;
    let mut fixtures = Vec::new();
    for f in l["fixtures"].as_array().into_iter().flatten() {
        let size = f["size_m"].as_array().map_or(1.0, |s| s.iter().filter_map(Value::as_f64).fold(0.0, f64::max));
        fixtures.push((f["compartment"].as_str().unwrap_or("").to_owned(), vec3(&f["center_m"], &rel)?, size));
    }
    let decks =
        l["decks"].as_array().into_iter().flatten().map(|d| num(&d["floor_y_m"], &rel)).collect::<Result<_, _>>()?;
    let drel = format!("data/ships/{ship}/detailing.json");
    let dtext = std::fs::read_to_string(root.join(&drel)).map_err(|e| format!("{drel}: {e}"))?;
    let d: Value = serde_json::from_str(&dtext).map_err(|e| format!("{drel}: {e}"))?;
    let frames = (num(&d["frames"]["spacing_m"], &drel)?, num(&d["frames"]["origin_z_m"], &drel)?);
    let jamb = (num(&d["door_frame"]["jamb_m"], &drel)?, num(&d["door_frame"]["pressure_jamb_m"], &drel)?);
    Ok(Layout { rooms, portals, seats, fixtures, decks, frames, jamb })
}

/// Whether `(x, z)` is inside a convex footprint (counter-clockwise), with a margin `m` inward.
fn inside(poly: &[[f64; 2]], x: f64, z: f64, m: f64) -> bool {
    (0..poly.len()).all(|i| {
        let (a, b) = (poly[i], poly[(i + 1) % poly.len()]);
        let (ex, ez) = (b[0] - a[0], b[1] - a[1]);
        let l = ex.hypot(ez).max(1e-9);
        // Positive area: the inside is to the left of each edge.
        (ex * (z - a[1]) - ez * (x - a[0])) / l >= m
    })
}

fn area(poly: &[[f64; 2]]) -> f64 {
    (0..poly.len())
        .map(|i| {
            let (a, b) = (poly[i], poly[(i + 1) % poly.len()]);
            a[0] * b[1] - b[0] * a[1]
        })
        .sum::<f64>()
        / 2.0
}

impl Layout {
    fn room(&self, id: &str) -> Option<&Room> {
        self.rooms.iter().find(|r| r.id == id)
    }

    /// The nearest bay middle along a wall running along `t` (unit, horizontal) to `p`: fore and aft walls carry ribs at
    /// the frames, athwartship walls stiffeners half a frame off them (detailing.json `frames`); a slanted wall has none.
    fn bay(&self, p: [f64; 3], t: [f64; 2]) -> [f64; 3] {
        let (s, o) = self.frames;
        if t[1].abs() > 0.95 {
            [p[0], p[1], o + s * ((p[2] - o) / s - 0.5).round() + s * 0.5]
        } else if t[0].abs() > 0.95 {
            [s * (p[0] / s).round(), p[1], p[2]]
        } else {
            p
        }
    }

    /// Why a point at `wall` facing `into` cannot be, if it cannot: the layout's part (doors, seats, fixtures).
    fn refused(&self, room: &str, wall: [f64; 3], into: [f64; 2]) -> Option<&'static str> {
        let t = [-into[1], into[0]];
        for p in self.portals.iter().filter(|p| p.normal[1].abs() < 0.5) {
            // A portal in this wall's plane, within its frame's reach along it.
            let d = [wall[0] - p.centre[0], wall[2] - p.centre[2]];
            let off = d[0] * p.normal[0] + d[1] * p.normal[2];
            let along = d[0] * t[0] + d[1] * t[1];
            let jamb = if p.kind == "pressure_door" { self.jamb.1 } else { self.jamb.0 };
            let level = (wall[1] - (p.centre[1] - p.size[1] / 2.0)).abs() < 2.0;
            if off.abs() < 0.3 && level && along.abs() < p.size[0] / 2.0 + jamb + DOOR_CLEAR_M + HALF_WIDTH_M {
                return Some("a door's clearance");
            }
        }
        if self
            .seats
            .iter()
            .any(|s| (s[0] - wall[0]).hypot(s[2] - wall[2]) < SEAT_CLEAR_M && (s[1] - wall[1]).abs() < 2.0)
        {
            return Some("a seat");
        }
        if self.fixtures.iter().any(|(c, f, size)| {
            c == room
                && (f[0] - wall[0]).hypot(f[2] - wall[2]) < size / 2.0 + FIXTURE_CLEAR_M
                && (f[1] - wall[1]).abs() < 2.5
        }) {
            return Some("a fixture");
        }
        let r = self.room(room)?;
        let probe = [wall[0] + into[0] * 0.35, wall[2] + into[1] * 0.35];
        if !r
            .brushes
            .iter()
            .any(|(y, poly)| wall[1] >= y[0] - 0.05 && wall[1] < y[1] && inside(poly, probe[0], probe[1], 0.0))
        {
            return Some("outside the room");
        }
        None
    }
}

/// The deck's wall at a layout wall point, as built: found by a probe from inside the room at handle height, then
/// checked flat (the wall right behind the bracket at every probe, nothing proud of it in front) with floor to stand on
/// before it. The wall point on the deck's wall, or why there is none.
fn built(world: &WalkWorld, wall: [f64; 3], into: [f64; 2]) -> Result<[f64; 3], &'static str> {
    let (w, n) = (wall.map(|x| x as f32), into.map(|x| x as f32));
    let t = [-n[1], n[0]];
    let back = [-n[0], 0.0, -n[1]];
    let from = [w[0] + n[0] * FIND_M, w[1] + 1.1, w[2] + n[1] * FIND_M];
    let d = world.cast(from, back, FIND_M + 0.5).ok_or("no wall behind it")?;
    if d < FIND_M - INSET_MAX_M {
        return Err("something in front of the wall");
    }
    let w = [w[0] + n[0] * (FIND_M - d), w[1], w[2] + n[1] * (FIND_M - d)];
    for h in PROBE_HEIGHTS_M {
        for o in [-HALF_WIDTH_M as f32, 0.0, HALF_WIDTH_M as f32] {
            let from = [w[0] + n[0] * PROBE_M + t[0] * o, w[1] + h, w[2] + n[1] * PROBE_M + t[1] * o];
            match world.cast(from, back, PROBE_M + WALL_BACK_M) {
                None => return Err("no wall behind it"),
                Some(d) if d < PROBE_M - WALL_PROUD_M => return Err("something in front of the wall"),
                _ => {}
            }
        }
    }
    let (fx, fz) = (w[0] + n[0] * 0.45, w[2] + n[1] * 0.45);
    match world.floor_below(fx, w[1] + 1.0, fz) {
        Some(y) if (y - w[1]).abs() <= FLOOR_SLOP_M => Ok(w.map(f64::from)),
        _ => Err("no floor before it"),
    }
}

/// Places, and why each place was refused (for the report).
struct Placer<'a> {
    l: &'a Layout,
    world: &'a WalkWorld,
    points: Vec<Point>,
    refusals: std::collections::BTreeMap<&'static str, usize>,
}

impl Placer<'_> {
    /// The first of `cands` (wall point, into) that passes, placed as `rule` in `room`.
    fn take_first(&mut self, room: &str, rule: &'static str, cands: &[Place]) -> bool {
        for &(wall, into) in cands {
            if self
                .points
                .iter()
                .any(|p| (p.wall[0] - wall[0]).hypot(p.wall[2] - wall[2]) < 1.0 && (p.wall[1] - wall[1]).abs() < 1.5)
            {
                continue;
            }
            let wall = match built(self.world, wall, into) {
                Ok(w) => w,
                Err(why) => {
                    *self.refusals.entry(why).or_default() += 1;
                    continue;
                }
            };
            if let Some(why) = self.l.refused(room, wall, into) {
                *self.refusals.entry(why).or_default() += 1;
                continue;
            }
            if self
                .points
                .iter()
                .any(|p| (p.wall[0] - wall[0]).hypot(p.wall[2] - wall[2]) < 1.0 && (p.wall[1] - wall[1]).abs() < 1.5)
            {
                continue;
            }
            let n = self.points.iter().filter(|p| p.room == room).count() + 1;
            let mm = |x: f64| (x * 1000.0).round() / 1000.0;
            let unit = |x: f64| (x * 10_000.0).round() / 10_000.0;
            self.points.push(Point {
                id: format!("sp_{room}_{n}"),
                room: room.to_owned(),
                rule,
                wall: wall.map(mm),
                into: into.map(unit),
            });
            return true;
        }
        false
    }

    /// Places along a wall from `c` (on the wall, at the floor) both ways along it, from `from` metres out to `to`,
    /// each at its bay's middle, nearest first.
    fn beside(&self, c: [f64; 3], into: [f64; 2], from: f64, to: f64) -> Vec<Place> {
        let t = [-into[1], into[0]];
        let mut out: Vec<(f64, [f64; 3])> = Vec::new();
        let mut d = from;
        while d <= to {
            for s in [1.0, -1.0] {
                let p = [c[0] + t[0] * d * s, c[1], c[2] + t[1] * d * s];
                let q = self.l.bay(p, t);
                let dist = (q[0] - c[0]).hypot(q[2] - c[2]);
                if dist >= from - 1e-6 && !out.iter().any(|(_, o)| (o[0] - q[0]).hypot(o[2] - q[2]) < 0.05) {
                    out.push((dist, q));
                }
            }
            d += 0.25;
        }
        out.sort_by(|a, b| a.0.total_cmp(&b.0));
        out.into_iter().map(|(_, q)| (q, into)).collect()
    }

    /// Every wall bay of a room: along each brush edge that is a wall (not open to another of its brushes), at its
    /// floor.
    fn walls(&self, r: &Room) -> Vec<Place> {
        let mut out = Vec::new();
        for (bi, (y, poly)) in r.brushes.iter().enumerate() {
            for i in 0..poly.len() {
                let (a, b) = (poly[i], poly[(i + 1) % poly.len()]);
                let len = (b[0] - a[0]).hypot(b[1] - a[1]);
                if len < 1.2 {
                    continue;
                }
                let t = [(b[0] - a[0]) / len, (b[1] - a[1]) / len];
                // The inside is to the left of the edge.
                let into = [-t[1], t[0]];
                let mid = [(a[0] + b[0]) / 2.0 - into[0] * 0.05, (a[1] + b[1]) / 2.0 - into[1] * 0.05];
                let open =
                    r.brushes.iter().enumerate().any(|(bj, (yj, pj))| {
                        bj != bi && yj[0] < y[1] && yj[1] > y[0] && inside(pj, mid[0], mid[1], 0.0)
                    });
                if open {
                    continue;
                }
                let mut s = 0.6;
                while s <= len - 0.6 {
                    let p = [a[0] + t[0] * s, y[0], a[1] + t[1] * s];
                    let q = self.l.bay(p, t);
                    if !out.iter().any(|(o, _): &Place| (o[0] - q[0]).hypot(o[2] - q[2]) < 0.05) {
                        out.push((q, into));
                    }
                    s += 1.0;
                }
            }
        }
        out
    }
}

/// The rooms that take a point at their main door (rule 1): every room and bay, not an airlock, a pod or a corridor.
fn takes_a_point(kind: &str) -> bool {
    matches!(kind, "room" | "bay" | "crawlspace")
}

/// Place the points for `ship`: the layout's rooms and corridors over the compiled deck's walk world.
fn place(l: &Layout, world: &WalkWorld) -> (Vec<Point>, Vec<String>) {
    let mut pl = Placer { l, world, points: Vec::new(), refusals: Default::default() };
    let mut notes = Vec::new();
    // 1. Inside every room's main door: its widest door to a corridor, else its widest door; then its other doors.
    for r in l.rooms.iter().filter(|r| takes_a_point(&r.kind)) {
        let mut doors: Vec<&Portal> = l
            .portals
            .iter()
            .filter(|p| matches!(p.kind.as_str(), "door" | "pressure_door") && p.normal[1].abs() < 0.5)
            .filter(|p| p.between.contains(&r.id) && !p.between.iter().any(|b| b == "space"))
            .collect();
        let to_corridor = |p: &Portal| p.between.iter().any(|b| l.room(b).is_some_and(|o| o.kind == "corridor"));
        doors.sort_by(|a, b| to_corridor(b).cmp(&to_corridor(a)).then(b.size[0].total_cmp(&a.size[0])));
        let mut placed = false;
        let first_door = doors.first().map(|p| [p.centre[0], p.centre[1] - p.size[1] / 2.0, p.centre[2]]);
        for p in doors {
            let into = if p.between[0] == r.id { [-p.normal[0], -p.normal[2]] } else { [p.normal[0], p.normal[2]] };
            let c = [p.centre[0], p.centre[1] - p.size[1] / 2.0, p.centre[2]];
            let jamb = if p.kind == "pressure_door" { l.jamb.1 } else { l.jamb.0 };
            let from = p.size[0] / 2.0 + jamb + DOOR_CLEAR_M + HALF_WIDTH_M;
            let cands = pl.beside(c, into, from, from + 3.0);
            if pl.take_first(&r.id, "main_door", &cands) {
                placed = true;
                break;
            }
        }
        if !placed {
            // No door wall with room: the room's own walls, nearest its main door first (else in the layout's order).
            let mut cands = pl.walls(r);
            if let Some(p) = first_door {
                let dist = |q: &[f64; 3]| (q[0] - p[0]).hypot(q[2] - p[2]) + (q[1] - p[1]).abs();
                cands.sort_by(|a, b| dist(&a.0).total_cmp(&dist(&b.0)));
            }
            if !pl.take_first(&r.id, "main_door", &cands) {
                notes.push(format!("{}: no wall takes a point", r.id));
            }
        }
    }
    // 2. Big rooms: another for every BIG_ROOM_M2 beyond the first, on the wall bay farthest from its points.
    for r in l.rooms.iter().filter(|r| takes_a_point(&r.kind)) {
        let floor: f64 = r.brushes.iter().map(|(_, p)| area(p)).sum();
        let extra = ((floor - BIG_ROOM_M2) / BIG_ROOM_M2).floor().max(0.0) as usize;
        for _ in 0..extra {
            let mine: Vec<[f64; 3]> = pl.points.iter().filter(|p| p.room == r.id).map(|p| p.wall).collect();
            let far = |q: &[f64; 3]| {
                mine.iter()
                    .map(|m| (m[0] - q[0]).hypot(m[2] - q[2]) + (m[1] - q[1]).abs() * 2.0)
                    .fold(f64::MAX, f64::min)
            };
            let mut cands = pl.walls(r);
            cands.sort_by(|a, b| far(&b.0).total_cmp(&far(&a.0)));
            if !pl.take_first(&r.id, "big_room", &cands) {
                notes.push(format!("{}: no wall takes a big room's point", r.id));
            }
        }
    }
    // 3. Corridors, evenly: ceil(L / 8) points at L (i + 1/2) / n along, port wall then starboard in turn.
    for r in l.rooms.iter().filter(|r| r.kind == "corridor") {
        for (y, poly) in &r.brushes {
            let (lo, hi) = poly.iter().fold(([f64::MAX; 2], [f64::MIN; 2]), |(lo, hi), p| {
                ([lo[0].min(p[0]), lo[1].min(p[1])], [hi[0].max(p[0]), hi[1].max(p[1])])
            });
            let along_z = hi[1] - lo[1] >= hi[0] - lo[0];
            let (len, k) = if along_z { (hi[1] - lo[1], 1) } else { (hi[0] - lo[0], 0) };
            let n = (len / CORRIDOR_SPACING_M).ceil().max(1.0) as usize;
            for i in 0..n {
                let s = lo[k] + len * (i as f64 + 0.5) / n as f64;
                // The walls: port (+X) then starboard for a fore and aft corridor; bow (+Z) then stern athwartships.
                let walls: [Place; 2] = if along_z {
                    [([hi[0], y[0], s], [-1.0, 0.0]), ([lo[0], y[0], s], [1.0, 0.0])]
                } else {
                    [([s, y[0], hi[1]], [0.0, -1.0]), ([s, y[0], lo[1]], [0.0, 1.0])]
                };
                let (first, second) = if i % 2 == 0 { (walls[0], walls[1]) } else { (walls[1], walls[0]) };
                // The bays within SHIFT_M of its place on either wall, nearest the place first, its own wall first.
                let mut cands: Vec<(f64, usize, Place)> = Vec::new();
                for (k, (w, into)) in [first, second].into_iter().enumerate() {
                    let t = [-into[1], into[0]];
                    let mut step = -SHIFT_M;
                    while step <= SHIFT_M {
                        let q = l.bay([w[0] + t[0] * step, w[1], w[2] + t[1] * step], t);
                        let off = (q[0] - w[0]).hypot(q[2] - w[2]);
                        if off <= SHIFT_M
                            && !cands.iter().any(|c| c.1 == k && (c.2 .0[0] - q[0]).hypot(c.2 .0[2] - q[2]) < 0.05)
                        {
                            cands.push((off, k, (q, into)));
                        }
                        step += 0.5;
                    }
                }
                cands.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
                let cands: Vec<Place> = cands.into_iter().map(|c| c.2).collect();
                if !pl.take_first(&r.id, "corridor", &cands) {
                    notes.push(format!("{}: no wall takes point {} of {n}", r.id, i + 1));
                }
            }
        }
    }
    // 4. Far floor: while some floor is over FAR_M from every point on its deck, the wall bay nearest it takes one; a
    // room with no wall to take one is said, and left out of the search.
    let mut walled: Vec<String> = Vec::new();
    for _ in 0..16 {
        let Some((d, at, room)) = farthest(l, &pl.points, None, &walled) else { break };
        if d <= FAR_M {
            break;
        }
        let Some(r) = l.room(&room) else { break };
        let mut cands = pl.walls(r);
        cands.sort_by(|a, b| {
            let dist = |q: &[f64; 3]| (q[0] - at[0]).hypot(q[2] - at[2]);
            dist(&a.0).total_cmp(&dist(&b.0))
        });
        if !pl.take_first(&room, "reach", &cands) {
            notes.push(format!("{room}: floor {d:.1} m from a point, and no wall of it takes one"));
            walled.push(room);
        }
    }
    let refused: Vec<String> = pl.refusals.iter().map(|(why, n)| format!("{n} for {why}")).collect();
    notes.push(format!("places refused: {}", refused.join(", ")));
    (pl.points, notes)
}

/// The deck a floor height is on: the nearest deck floor at or below it.
fn deck_of(l: &Layout, y: f64) -> usize {
    let mut best = 0;
    for (i, d) in l.decks.iter().enumerate() {
        if (y - d).abs() < (y - l.decks[best]).abs() {
            best = i;
        }
    }
    best
}

/// The floor point farthest from a point on its own deck, over every room and corridor that takes one (on `deck`
/// alone, if given; not the rooms in `skip`), sampled every half metre: (distance, where, room).
fn farthest(l: &Layout, points: &[Point], deck: Option<usize>, skip: &[String]) -> Option<(f64, [f64; 3], String)> {
    let mut worst: Option<(f64, [f64; 3], String)> = None;
    for r in l.rooms.iter().filter(|r| (takes_a_point(&r.kind) || r.kind == "corridor") && !skip.contains(&r.id)) {
        for (y, poly) in r.brushes.iter().filter(|(y, _)| deck.is_none_or(|d| deck_of(l, y[0]) == d)) {
            let (lo, hi) = poly.iter().fold(([f64::MAX; 2], [f64::MIN; 2]), |(lo, hi), p| {
                ([lo[0].min(p[0]), lo[1].min(p[1])], [hi[0].max(p[0]), hi[1].max(p[1])])
            });
            let deck = deck_of(l, y[0]);
            let mut x = lo[0] + 0.25;
            while x < hi[0] {
                let mut z = lo[1] + 0.25;
                while z < hi[1] {
                    if inside(poly, x, z, 0.3) {
                        let d = points
                            .iter()
                            .filter(|p| deck_of(l, p.wall[1]) == deck)
                            .map(|p| (p.wall[0] - x).hypot(p.wall[2] - z))
                            .fold(f64::MAX, f64::min);
                        if worst.as_ref().is_none_or(|w| d > w.0) {
                            worst = Some((d, [x, y[0], z], r.id.clone()));
                        }
                    }
                    z += 0.5;
                }
                x += 0.5;
            }
        }
    }
    worst
}

/// The kit file's text for the points.
fn to_json(points: &[Point]) -> String {
    let pts: Vec<Value> = points
        .iter()
        .map(|p| {
            json!({
                "id": p.id,
                "compartment": p.room,
                "rule": p.rule,
                "wall_point_m": p.wall,
                "normal": [p.into[0], 0.0, p.into[1]],
                "holds": ["extinguisher"],
            })
        })
        .collect();
    let v = json!({
        "schema": "starcrew.kit/1",
        "note": "Written by `sc-tools safety-points` from layout.json, detailing.json's frames and the compiled deck's walk world (openspec/changes/safety-points design 2, 4 and 5a); never edited by hand. A point is an extinguisher's bracket on a room's wall: wall_point_m on the wall at the room's floor, normal the way into the room. Rules: main_door (inside every room's main door), big_room (another for every 80 m^2 beyond the first 80), corridor (ceil(L / 8) along every corridor, evenly, its walls in turn), reach (floor still over 10 m from one).",
        "points": pts,
    });
    let mut text = serde_json::to_string_pretty(&v).expect("a kit file serializes");
    text.push('\n');
    text
}

fn world(root: &Path, ship: &str) -> Result<WalkWorld, String> {
    let rel = format!("compiled/{ship}.deck");
    let bytes = std::fs::read(root.join(&rel)).map_err(|e| format!("{rel}: {e} (build it: sc-tools deckc {ship})"))?;
    let deck = sc_core::deck::read(&bytes).map_err(|e| format!("{rel}: {e}"))?;
    let text =
        std::fs::read_to_string(root.join("data/crew/walk.json")).map_err(|e| format!("data/crew/walk.json: {e}"))?;
    let data: WalkData = sc_core::data::parse("data/crew/walk.json", &text).map_err(|e| e.to_string())?;
    let w = &deck.index.walk;
    let ents = WalkEntities {
        ladders: w.ladders.clone(),
        hatches: w.hatches.clone(),
        doors: w.doors.clone(),
        lifts: w.lifts.clone(),
    };
    WalkWorld::new(&deck.walk_triangles(), &ents, &data)
}

/// The kit file `ship`'s layout and compiled deck give, and the report's lines.
fn build(root: &Path, ship: &str) -> Result<(String, Vec<String>), String> {
    let l = read_layout(root, ship)?;
    let w = world(root, ship)?;
    let (points, mut notes) = place(&l, &w);
    for (i, d) in l.decks.iter().enumerate() {
        let on: Vec<&Point> = points.iter().filter(|p| deck_of(&l, p.wall[1]) == i).collect();
        let far = farthest(&l, &points, Some(i), &[]);
        let far =
            far.map_or("no floor".to_owned(), |(d, _, r)| format!("farthest floor {d:.1} m from a point (in {r})"));
        notes.push(format!("deck at {d:+.1} m: {} points; {far}", on.len()));
    }
    let text = to_json(&points);
    let rel = path(ship);
    sc_core::data::parse::<sc_core::combat::damage::KitFile>(&rel, &text).map_err(|e| e.to_string())?;
    notes.insert(
        0,
        format!(
            "{} points: {} main door, {} big room, {} corridor, {} reach",
            points.len(),
            points.iter().filter(|p| p.rule == "main_door").count(),
            points.iter().filter(|p| p.rule == "big_room").count(),
            points.iter().filter(|p| p.rule == "corridor").count(),
            points.iter().filter(|p| p.rule == "reach").count()
        ),
    );
    Ok((text, notes))
}

/// Write `ship`'s kit file; the report.
pub(crate) fn run(root: &Path, ship: &str) -> Result<String, String> {
    let (text, notes) = build(root, ship)?;
    let rel = path(ship);
    std::fs::write(root.join(&rel), &text).map_err(|e| format!("{rel}: {e}"))?;
    Ok(format!("{rel}\n  {}", notes.join("\n  ")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root() -> &'static Path {
        Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
    }

    /// The committed kit file is the one the layout and the compiled deck give. Needs the compiled deck; skipped
    /// without one.
    #[test]
    fn the_committed_kit_file_is_the_one_the_layout_places() {
        if !root().join("compiled/tern.deck").exists() {
            eprintln!("no compiled deck: skipped");
            return;
        }
        let rel = path("tern");
        let Ok((want, _)) = build(root(), "tern") else {
            eprintln!("a compiled deck of another version: skipped");
            return;
        };
        let have = std::fs::read_to_string(root().join(&rel)).unwrap();
        assert!(have == want, "{rel} is stale: run `cargo run --release -p sc-tools -- safety-points tern`");
    }

    /// The owner's rule (2026-10-12): every room has an extinguisher, and every corridor has `ceil(L / 8)` of them,
    /// spaced evenly (within a frame bay of their places), on alternate walls.
    #[test]
    fn every_room_has_one_and_the_corridors_are_spaced_evenly() {
        let text = std::fs::read_to_string(root().join(path("tern"))).unwrap();
        let kit: Value = serde_json::from_str(&text).unwrap();
        let pts = kit["points"].as_array().unwrap();
        let l = read_layout(root(), "tern").unwrap();
        for r in l.rooms.iter().filter(|r| takes_a_point(&r.kind)) {
            assert!(pts.iter().any(|p| p["compartment"] == r.id.as_str()), "{} has no extinguisher", r.id);
        }
        for r in l.rooms.iter().filter(|r| r.kind == "corridor") {
            let (_, poly) = &r.brushes[0];
            let (z0, z1) = poly.iter().fold((f64::MAX, f64::MIN), |(a, b), p| (a.min(p[1]), b.max(p[1])));
            let n = ((z1 - z0) / CORRIDOR_SPACING_M).ceil() as usize;
            let mut mine: Vec<(f64, f64)> = pts
                .iter()
                .filter(|p| p["compartment"] == r.id.as_str() && p["rule"] == "corridor")
                .map(|p| (p["wall_point_m"][2].as_f64().unwrap(), p["wall_point_m"][0].as_f64().unwrap()))
                .collect();
            mine.sort_by(|a, b| a.0.total_cmp(&b.0));
            assert_eq!(mine.len(), n, "{}: {} m long takes {n}", r.id, z1 - z0);
            for (i, (z, _)) in mine.iter().enumerate() {
                let want = z0 + (z1 - z0) * (i as f64 + 0.5) / n as f64;
                assert!((z - want).abs() <= SHIFT_M + 1e-6, "{}: point {i} at z {z}, its place {want}", r.id);
            }
        }
    }
}
