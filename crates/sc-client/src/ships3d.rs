//! Placeholder 3D models for the co-op drill (openspec/changes/coop-drill design 5; owner, 2026-10-10: "make enemy
//! ship 3d we'll just use that as placeholder for now"): the Hound corvette, a pulse bolt, a Gannet missile and a
//! burst, as flat-shaded low-poly meshes in the 28-byte deck vertex.
//!
//! They are generated here, deterministically, until the Hound is a modelled asset with its own triangle budget
//! (CLAUDE.md 9: meshes are files built by committed generators; this is the generator, and a placeholder). The
//! Hound's colours are albedo for the probe program (lit by an ambient cube from the sun); the bolt, missile and
//! burst are emissive colours for the flat program.

use sc_core::vertex::{pack_deck_vertex, DeckVertexIn};

/// A mesh being built: 28-byte vertices and 16-bit indices, one flat-shaded triangle at a time.
#[derive(Default)]
pub struct MeshBuilder {
    /// Packed vertices.
    pub vertices: Vec<u8>,
    /// Indices.
    pub indices: Vec<u16>,
}

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

fn norm(a: [f32; 3]) -> [f32; 3] {
    let l = (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt().max(1e-9);
    [a[0] / l, a[1] / l, a[2] / l]
}

/// An sRGB colour as the vertex's display multiplier byte (128 is 1x).
fn byte(c: f32) -> u8 {
    (c * 128.0).round().clamp(0.0, 255.0) as u8
}

impl MeshBuilder {
    /// One triangle, counter-clockwise seen from outside, in colour `rgb` (display multipliers, 1 = as is).
    pub fn tri(&mut self, a: [f32; 3], b: [f32; 3], c: [f32; 3], rgb: [f32; 3]) {
        let raw = cross(sub(b, a), sub(c, a));
        // A triangle with no area (a sphere's pole, a ring's collapsed corner) draws nothing: skip it.
        if raw.iter().map(|x| x * x).sum::<f32>() < 1e-12 {
            return;
        }
        let n = norm(raw);
        let col = [byte(rgb[0]), byte(rgb[1]), byte(rgb[2]), 255];
        let base = (self.vertices.len() / 28) as u16;
        for p in [a, b, c] {
            let v = DeckVertexIn { position_m: p, mover: 0, layer: 0, normal: n, colors: [col; 3], uv: [0.0, 0.0] };
            self.vertices.extend_from_slice(&pack_deck_vertex(&v).expect("a placeholder model fits the format"));
        }
        self.indices.extend_from_slice(&[base, base + 1, base + 2]);
    }

    /// A quad as two triangles, corners counter-clockwise from outside.
    pub fn quad(&mut self, a: [f32; 3], b: [f32; 3], c: [f32; 3], d: [f32; 3], rgb: [f32; 3]) {
        self.tri(a, b, c, rgb);
        self.tri(a, c, d, rgb);
    }

    /// A closed loft between two rings of equal length (each counter-clockwise looking down +Z), with end caps.
    fn loft(&mut self, back: &[[f32; 3]], front: &[[f32; 3]], rgb: [f32; 3], cap_back: [f32; 3], cap_front: [f32; 3]) {
        let n = back.len();
        for i in 0..n {
            let j = (i + 1) % n;
            self.quad(back[i], back[j], front[j], front[i], rgb);
        }
        let cb =
            back.iter().fold([0.0; 3], |s, p| [s[0] + p[0] / n as f32, s[1] + p[1] / n as f32, s[2] + p[2] / n as f32]);
        let cf = front
            .iter()
            .fold([0.0; 3], |s, p| [s[0] + p[0] / n as f32, s[1] + p[1] / n as f32, s[2] + p[2] / n as f32]);
        for i in 0..n {
            let j = (i + 1) % n;
            self.tri(cb, back[j], back[i], cap_back);
            self.tri(cf, front[i], front[j], cap_front);
        }
    }

    /// The triangle count.
    pub fn triangles(&self) -> usize {
        self.indices.len() / 3
    }
}

fn ring(z: f32, w: f32, h: f32, y: f32) -> Vec<[f32; 3]> {
    // A flattened hexagon: port, dorsal pair, starboard, ventral pair; counter-clockwise looking down +Z.
    vec![
        [w, y, z],
        [w * 0.5, y + h, z],
        [-w * 0.5, y + h, z],
        [-w, y, z],
        [-w * 0.5, y - h * 0.7, z],
        [w * 0.5, y - h * 0.7, z],
    ]
}

/// The Hound corvette, about 38 m long: a hexagonal hull tapering to a beak, swept fins, a dorsal tower, twin
/// engines. Albedo colours; ship-local axes +X port, +Y dorsal, +Z bow.
/// A mesh's box in its own axes: the least and greatest of its vertices' positions, metres (target-bracket 1). Read
/// back from the packed vertices, so it is the box of the model the engine draws.
pub fn bounds(m: &MeshBuilder) -> ([f32; 3], [f32; 3]) {
    let (mut lo, mut hi) = ([f32::INFINITY; 3], [f32::NEG_INFINITY; 3]);
    for v in m.vertices.chunks_exact(sc_core::vertex::DECK_VERTEX_BYTES) {
        for (i, (l, h)) in lo.iter_mut().zip(hi.iter_mut()).enumerate() {
            let p = f32::from(i16::from_le_bytes([v[i * 2], v[i * 2 + 1]])) / sc_core::vertex::FIXED_SCALE;
            *l = l.min(p);
            *h = h.max(p);
        }
    }
    (lo, hi)
}

/// The hull box of a contact class (`corvette` the Hound, `fighter` the Jackal), measured once off its model.
pub fn class_box(cls: &str) -> Option<([f32; 3], [f32; 3])> {
    use std::sync::OnceLock;
    static HOUND: OnceLock<([f32; 3], [f32; 3])> = OnceLock::new();
    static JACKAL: OnceLock<([f32; 3], [f32; 3])> = OnceLock::new();
    match cls {
        "corvette" => Some(*HOUND.get_or_init(|| bounds(&hound()))),
        "fighter" => Some(*JACKAL.get_or_init(|| bounds(&jackal()))),
        _ => None,
    }
}

/// The Jackal raider (enemy-fighters 1): a delta wing 8 m long, its albedo lit like the Hound's.
pub fn jackal() -> MeshBuilder {
    let mut m = MeshBuilder::default();
    let (hull, dark, glow) = ([0.40, 0.38, 0.36], [0.2, 0.19, 0.2], [1.9, 0.8, 0.3]);
    let nose = [0.0, 0.0, 4.0];
    let (top, bot) = ([0.0, 0.8, -2.0], [0.0, -0.6, -2.0]);
    let (lw, rw) = ([4.0, 0.0, -3.5], [-4.0, 0.0, -3.5]);
    let tail = [0.0, 0.0, -3.0];
    // Upper and lower faces of the wing and body.
    m.tri(nose, lw, top, hull);
    m.tri(nose, top, rw, hull);
    m.tri(nose, bot, lw, dark);
    m.tri(nose, rw, bot, dark);
    m.tri(top, lw, tail, hull);
    m.tri(top, tail, rw, hull);
    m.tri(bot, tail, lw, dark);
    m.tri(bot, rw, tail, dark);
    // A fin and the engine's glow.
    m.tri([0.0, 0.8, -2.0], [0.0, 1.8, -3.6], [0.0, 0.4, -3.2], hull);
    m.tri([0.0, 0.8, -2.0], [0.0, 0.4, -3.2], [0.0, 1.8, -3.6], hull);
    m.quad([0.5, 0.4, -3.05], [-0.5, 0.4, -3.05], [-0.5, -0.3, -3.05], [0.5, -0.3, -3.05], glow);
    m
}

pub fn hound() -> MeshBuilder {
    let mut m = MeshBuilder::default();
    let hull = [0.42, 0.40, 0.38];
    let dark = [0.22, 0.21, 0.22];
    let red = [0.62, 0.16, 0.12];
    let glow = [1.9, 0.9, 0.35];
    // Hull: stern, mid, shoulder and beak rings.
    let rings =
        [ring(-16.0, 4.2, 2.6, 0.0), ring(-4.0, 5.0, 3.0, 0.0), ring(8.0, 3.6, 2.2, 0.2), ring(17.0, 0.9, 0.6, 0.4)];
    for k in 0..rings.len() - 1 {
        let colour = if k == 1 { red } else { hull };
        let (a, b) = (&rings[k], &rings[k + 1]);
        for i in 0..6 {
            let j = (i + 1) % 6;
            let c = if i == 1 || i == 2 { colour } else { hull };
            m.quad(a[i], a[j], b[j], b[i], c);
        }
    }
    for i in 0..6 {
        let j = (i + 1) % 6;
        let c = [0.0, 0.0, -16.0];
        m.tri(c, rings[0][j], rings[0][i], dark);
        let tip = [0.0, 0.4, 19.5];
        m.tri(tip, rings[3][i], rings[3][j], dark);
    }
    // Swept fins, port and starboard, with thickness.
    for s in [1.0f32, -1.0] {
        let root_f = [4.6 * s, 0.0, 2.0];
        let root_b = [4.0 * s, 0.0, -12.0];
        let tip_b = [13.0 * s, -1.2, -15.0];
        let tip_f = [12.0 * s, -1.0, -9.0];
        let up = [0.0, 0.35, 0.0];
        let add = |p: [f32; 3], d: [f32; 3]| [p[0] + d[0], p[1] + d[1], p[2] + d[2]];
        let dn = [0.0, -0.35, 0.0];
        let (a, b, c, d) = (add(root_f, up), add(root_b, up), add(tip_b, up), add(tip_f, up));
        let (e, f, g, h) = (add(root_f, dn), add(root_b, dn), add(tip_b, dn), add(tip_f, dn));
        if s > 0.0 {
            m.quad(a, d, c, b, hull);
            m.quad(e, f, g, h, dark);
            m.quad(d, h, g, c, red);
            m.quad(a, e, h, d, hull);
            m.quad(b, c, g, f, dark);
        } else {
            m.quad(a, b, c, d, hull);
            m.quad(e, h, g, f, dark);
            m.quad(d, c, g, h, red);
            m.quad(a, d, h, e, hull);
            m.quad(b, f, g, c, dark);
        }
    }
    // Dorsal tower.
    let tower_b = ring(-9.0, 1.4, 1.6, 3.0);
    let tower_f = ring(-1.0, 1.0, 1.2, 3.2);
    m.loft(&tower_b, &tower_f, hull, dark, [0.30, 0.55, 0.70]);
    // Twin engines with glowing nozzles.
    for s in [1.0f32, -1.0] {
        let back = ring(-18.5, 1.3, 1.0, -0.8).into_iter().map(|p| [p[0] + 2.6 * s, p[1], p[2]]).collect::<Vec<_>>();
        let front = ring(-10.0, 1.3, 1.0, -0.8).into_iter().map(|p| [p[0] + 2.6 * s, p[1], p[2]]).collect::<Vec<_>>();
        m.loft(&back, &front, dark, glow, dark);
    }
    m
}

/// A pulse bolt: a long octahedron along +Z, 1 m long and 0.1 m across (scaled when drawn), emissive.
pub fn bolt(rgb: [f32; 3]) -> MeshBuilder {
    let mut m = MeshBuilder::default();
    let (f, b) = ([0.0, 0.0, 0.5], [0.0, 0.0, -0.5]);
    let r = 0.05;
    let ring = [[r, 0.0, 0.1], [0.0, r, 0.1], [-r, 0.0, 0.1], [0.0, -r, 0.1]];
    for i in 0..4 {
        let j = (i + 1) % 4;
        m.tri(f, ring[i], ring[j], rgb);
        m.tri(b, ring[j], ring[i], rgb);
    }
    m
}

/// A Gannet missile, 4 m along +Z, with a bright motor.
pub fn missile() -> MeshBuilder {
    let mut m = MeshBuilder::default();
    let body = [0.85, 0.85, 0.80];
    let back = ring(-2.0, 0.22, 0.22, 0.0);
    let front = ring(1.4, 0.22, 0.22, 0.0);
    m.loft(&back, &front, body, [2.0, 1.6, 0.9], body);
    let tip = [0.0, 0.0, 2.2];
    for i in 0..6 {
        let j = (i + 1) % 6;
        m.tri(tip, front[i], front[j], [0.9, 0.3, 0.2]);
    }
    m
}

/// A burst: a low-poly sphere of radius 1 (scaled when drawn), emissive orange.
pub fn burst() -> MeshBuilder {
    let mut m = MeshBuilder::default();
    let (rings, segs) = (5, 8);
    let p = |i: usize, j: usize| {
        let th = std::f32::consts::PI * i as f32 / rings as f32;
        let ph = std::f32::consts::TAU * j as f32 / segs as f32;
        [th.sin() * ph.cos(), th.cos(), th.sin() * ph.sin()]
    };
    for i in 0..rings {
        for j in 0..segs {
            let c = if (i + j) % 2 == 0 { [2.0, 1.2, 0.4] } else { [1.8, 0.7, 0.2] };
            let (a, b, cc, d) = (p(i, j), p(i, j + 1), p(i + 1, j + 1), p(i + 1, j));
            m.tri(a, b, cc, c);
            m.tri(a, cc, d, c);
        }
    }
    m
}

/// Flames on a burning room's floor (all-hands 4): five tongues about the middle, each two crossed blades seen from
/// both sides, bright enough to glow (colours over 1), the tallest 1.4 m; the draw flickers their height.
pub fn flames() -> MeshBuilder {
    let mut m = MeshBuilder::default();
    let tongues: [(f32, f32, f32); 5] =
        [(0.0, 0.0, 1.0), (0.35, 0.2, 0.75), (-0.3, 0.25, 0.8), (0.2, -0.35, 0.7), (-0.25, -0.3, 0.65)];
    for (k, (x, z, h)) in tongues.into_iter().enumerate() {
        let w = 0.28;
        let c = if k % 2 == 0 { [2.4, 1.0, 0.25] } else { [2.6, 1.7, 0.5] };
        let tip = [x, h * 1.4, z];
        for (dx, dz) in [(w, 0.0), (0.0, w)] {
            let (a, b) = ([x - dx, 0.0, z - dz], [x + dx, 0.0, z + dz]);
            m.tri(a, b, tip, c);
            m.tri(b, a, tip, c);
        }
    }
    m
}

/// A puff of extinguishing agent: a white ball (the burst's shape), drawn along a spray.
pub fn puff() -> MeshBuilder {
    let mut m = MeshBuilder::default();
    let (rings, segs) = (4, 6);
    let p = |i: usize, j: usize| {
        let th = std::f32::consts::PI * i as f32 / rings as f32;
        let ph = std::f32::consts::TAU * j as f32 / segs as f32;
        [th.sin() * ph.cos(), th.cos(), th.sin() * ph.sin()]
    };
    for i in 0..rings {
        for j in 0..segs {
            let c = if (i + j) % 2 == 0 { [1.3, 1.35, 1.4] } else { [1.1, 1.15, 1.2] };
            let (a, b, cc, d) = (p(i, j), p(i, j + 1), p(i + 1, j + 1), p(i + 1, j));
            m.tri(a, b, cc, c);
            m.tri(a, cc, d, c);
        }
    }
    m
}

/// The extinguisher in a player's hand: a red cylinder (an eight-sided prism) 0.4 m long, its black hose and nozzle
/// at the top, upright in its own frame.
pub fn extinguisher() -> MeshBuilder {
    let mut m = MeshBuilder::default();
    let (r, h) = (0.07f32, 0.4f32);
    let ring = |y: f32, r: f32| -> Vec<[f32; 3]> {
        (0..8)
            .map(|i| {
                let a = std::f32::consts::TAU * i as f32 / 8.0;
                [a.cos() * r, y, a.sin() * r]
            })
            .collect()
    };
    let (lo, hi) = (ring(0.0, r), ring(h, r));
    for i in 0..8 {
        let j = (i + 1) % 8;
        m.quad(lo[i], lo[j], hi[j], hi[i], if i % 2 == 0 { [0.75, 0.08, 0.08] } else { [0.6, 0.05, 0.05] });
        m.tri(hi[i], hi[j], [0.0, h + 0.03, 0.0], [0.5, 0.05, 0.05]);
    }
    let nozzle = [[-0.015, h, -0.015], [0.015, h, -0.015], [0.015, h + 0.06, -0.12], [-0.015, h + 0.06, -0.12]];
    m.quad(nozzle[0], nozzle[1], nozzle[2], nozzle[3], [0.08, 0.08, 0.08]);
    m.quad(nozzle[1], nozzle[0], nozzle[3], nozzle[2], [0.08, 0.08, 0.08]);
    m
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_hound_is_low_poly_and_inside_the_vertex_format() {
        let h = hound();
        assert!(h.triangles() > 60 && h.triangles() <= 400, "a placeholder corvette of {} triangles", h.triangles());
        assert_eq!(h.vertices.len() / 28, h.indices.len(), "one vertex per corner: flat shading");
    }

    fn outward_share(m: &MeshBuilder) -> f64 {
        // Positions back from the packed vertices (1/1024 m), each triangle's normal against its centroid's
        // direction from the model's centre: back faces are culled, so faces must wind outward.
        let pos = |v: usize| {
            let b = &m.vertices[v * 28..v * 28 + 6];
            let c = |k: usize| f32::from(i16::from_le_bytes([b[k], b[k + 1]])) / 1024.0;
            [c(0), c(2), c(4)]
        };
        let mut out = 0;
        let tris = m.indices.chunks(3).collect::<Vec<_>>();
        for t in &tris {
            let (a, b, c) = (pos(t[0] as usize), pos(t[1] as usize), pos(t[2] as usize));
            let n = cross(sub(b, a), sub(c, a));
            let mid = [(a[0] + b[0] + c[0]) / 3.0, (a[1] + b[1] + c[1]) / 3.0, (a[2] + b[2] + c[2]) / 3.0];
            if n[0] * mid[0] + n[1] * mid[1] + n[2] * mid[2] > 0.0 {
                out += 1;
            }
        }
        f64::from(out) / tris.len() as f64
    }

    #[test]
    fn the_models_wind_outward() {
        // Against the model's centre, a convex model winds all outward; the Hound's engines and tower sit off its
        // centre line, so their inner faces point at the centre and the honest floor is lower. An inside-out model
        // would show close to 0.
        for (name, m, floor) in [
            ("hound", hound(), 0.7),
            ("missile", missile(), 0.95),
            ("burst", burst(), 0.95),
            ("bolt", bolt([1.0; 3]), 0.95),
        ] {
            let share = outward_share(&m);
            assert!(share >= floor, "{name}: {share:.2} of its faces wind outward");
        }
    }
}

#[cfg(test)]
mod mockup_tests {
    use super::*;

    /// The mockup's `SHIP_BOX` (docs/mockups/consoles.html), read out of the page, as `{class: [[lo], [hi]]}`.
    fn mockup_boxes() -> serde_json::Value {
        let page = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/mockups/consoles.html"))
            .expect("the console mockup");
        let at = page.find("const SHIP_BOX = ").expect("the mockup's SHIP_BOX") + "const SHIP_BOX = ".len();
        let end = at + page[at..].find(';').expect("the end of SHIP_BOX");
        serde_json::from_str(&page[at..end]).expect("SHIP_BOX is JSON")
    }

    #[test]
    fn the_mockups_ship_boxes_are_the_models_boxes() {
        let table = mockup_boxes();
        for cls in ["corvette", "fighter"] {
            let (lo, hi) = class_box(cls).unwrap();
            let want =
                serde_json::json!([lo.map(|v| (v * 100.0).round() / 100.0), hi.map(|v| (v * 100.0).round() / 100.0)]);
            let got = &table[cls];
            let close = (0..2).all(|c| {
                (0..3).all(|a| (got[c][a].as_f64().unwrap_or(f64::NAN) - want[c][a].as_f64().unwrap()).abs() <= 0.01)
            });
            assert!(close, "consoles.html SHIP_BOX.{cls} should be {want} (target-bracket 4), is {got}");
        }
    }
}
