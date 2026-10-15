//! Placeholder 3D models for the co-op drill (openspec/changes/coop-drill design 5; owner, 2026-10-10: "make enemy
//! ship 3d we'll just use that as placeholder for now"): the Hound corvette, a pulse bolt, a Gannet missile and a
//! burst, as flat-shaded low-poly meshes in the 28-byte deck vertex; and the extinguisher in a player's hand, the
//! mockups' own (docs/mockups/lib/firefight.js).
//!
//! They are generated here, deterministically, until the Hound is a modelled asset with its own triangle budget
//! (CLAUDE.md 9: meshes are files built by committed generators; this is the generator, and a placeholder). The
//! Hound's colours are albedo for the probe program (lit by an ambient cube from the sun); the bolt, missile and
//! burst are emissive colours for the flat program. The placeholder stations of the patrol (openspec/changes/patrol-mission
//! design 3: the relay at the Shoals) is generated here the same way; the spacedocks are `stations`.

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
        self.tri_sets(a, b, c, [rgb; 3]);
    }

    /// One triangle with everything the vertex holds: its corners (stored units), its normal, its texture coordinates
    /// and layer, and each corner's colour (display multipliers), the same in all three lighting states.
    pub fn tri_full(&mut self, p: [[f32; 3]; 3], n: [f32; 3], uv: [[f32; 2]; 3], layer: u8, rgb: [[f32; 3]; 3]) {
        let base = (self.vertices.len() / 28) as u16;
        for k in 0..3 {
            let c = [byte(rgb[k][0]), byte(rgb[k][1]), byte(rgb[k][2]), 255];
            let v = DeckVertexIn { position_m: p[k], mover: 0, layer, normal: n, colors: [c; 3], uv: uv[k] };
            self.vertices.extend_from_slice(&pack_deck_vertex(&v).expect("a generated model fits the format"));
        }
        self.indices.extend_from_slice(&[base, base + 1, base + 2]);
    }

    /// One triangle with its three colour sets (the deck vertex's normal, red alert and emergency; an unlit mesh
    /// blends them by its draw's state weights as it likes).
    pub fn tri_sets(&mut self, a: [f32; 3], b: [f32; 3], c: [f32; 3], sets: [[f32; 3]; 3]) {
        let raw = cross(sub(b, a), sub(c, a));
        // A triangle with no area (a sphere's pole, a ring's collapsed corner) draws nothing: skip it.
        if raw.iter().map(|x| x * x).sum::<f32>() < 1e-12 {
            return;
        }
        let n = norm(raw);
        let colors = sets.map(|rgb| [byte(rgb[0]), byte(rgb[1]), byte(rgb[2]), 255]);
        let base = (self.vertices.len() / 28) as u16;
        for p in [a, b, c] {
            let v = DeckVertexIn { position_m: p, mover: 0, layer: 0, normal: n, colors, uv: [0.0, 0.0] };
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

/// A colour of the mockups' `paint` (docs/mockups/lib/firefight.js): `hex` (sRGB) darker on a face that looks away from
/// a light above and in front, `k = 0.62 + 0.3 max(0, n.y) + 0.12 max(0, n.z)`, applied in linear light as three.js
/// does, back to a display colour.
fn paint(hex: u32, n: [f32; 3]) -> [f32; 3] {
    let k = 0.62 + 0.3 * n[1].max(0.0) + 0.12 * n[2].max(0.0);
    let lin = |c: f32| if c <= 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) };
    let srgb = |c: f32| if c <= 0.003_130_8 { c * 12.92 } else { 1.055 * c.powf(1.0 / 2.4) - 0.055 };
    let ch = |shift: u32| srgb(lin(((hex >> shift) & 0xff) as f32 / 255.0) * k);
    [ch(16), ch(8), ch(0)]
}

/// Painted parts gathered in their own frame (the mockups' `paint` shades by a part's own facing, before it is turned
/// onto its wall), then laid into a mesh at a place: each triangle, its colour and whether it glows (unshaded).
#[derive(Default)]
struct Parts(Vec<([[f32; 3]; 3], u32, bool)>);

impl Parts {
    fn tri(&mut self, a: [f32; 3], b: [f32; 3], c: [f32; 3], hex: u32) {
        self.0.push(([a, b, c], hex, false));
    }

    /// A box as three.js's `BoxGeometry(size)` moved to `c`, painted `hex`, or glowing in it.
    fn cuboid(&mut self, c: [f32; 3], size: [f32; 3], hex: u32, glow: bool) {
        let e = size.map(|x| x / 2.0);
        let v = |s: [f32; 3]| [c[0] + s[0] * e[0], c[1] + s[1] * e[1], c[2] + s[2] * e[2]];
        let faces: [[[f32; 3]; 4]; 6] = [
            [[1., -1., -1.], [1., 1., -1.], [1., 1., 1.], [1., -1., 1.]],
            [[-1., -1., 1.], [-1., 1., 1.], [-1., 1., -1.], [-1., -1., -1.]],
            [[-1., 1., -1.], [-1., 1., 1.], [1., 1., 1.], [1., 1., -1.]],
            [[-1., -1., 1.], [-1., -1., -1.], [1., -1., -1.], [1., -1., 1.]],
            [[-1., -1., 1.], [1., -1., 1.], [1., 1., 1.], [-1., 1., 1.]],
            [[1., -1., -1.], [-1., -1., -1.], [-1., 1., -1.], [1., 1., -1.]],
        ];
        for f in faces {
            let q = f.map(v);
            self.0.push(([q[0], q[1], q[2]], hex, glow));
            self.0.push(([q[0], q[2], q[3]], hex, glow));
        }
    }

    /// Lay the parts into `m`, turned about the vertical so their +Z is `into` (horizontal, unit) and moved to `at`;
    /// `highlight`, a colour their third colour set takes instead of their own, painted alike.
    fn lay(&self, m: &mut MeshBuilder, at: [f32; 3], into: [f32; 3], highlight: Option<u32>) {
        // three.js's rotateY(atan2(into.x, into.z)): +Z to `into`, +X to (into.z, 0, -into.x).
        let place = |p: [f32; 3]| {
            [at[0] + into[2] * p[0] + into[0] * p[2], at[1] + p[1], at[2] - into[0] * p[0] + into[2] * p[2]]
        };
        for ([a, b, c], hex, glow) in &self.0 {
            let n = norm(cross(sub(*b, *a), sub(*c, *a)));
            // A glowing part is its colour whatever its facing (the mockups' `paint(g, hex, true)`).
            let colour =
                |hex: u32| if *glow { [16, 8, 0].map(|s| ((hex >> s) & 0xff) as f32 / 255.0) } else { paint(hex, n) };
            let rgb = colour(*hex);
            m.tri_sets(place(*a), place(*b), place(*c), [rgb, rgb, highlight.map_or(rgb, colour)]);
        }
    }
}

/// The extinguisher's red and its valve's near black (the mockup's `RED` and `DARK`), and the safety sign's glow
/// (`SIGN`).
const EXT_RED: u32 = 0xb8231b;
const EXT_DARK: u32 = 0x1b1f24;
const SIGN_GLOW: u32 = 0xa8ffd0;

/// The extinguisher's parts, upright in its own frame (`extinguisher`).
fn extinguisher_parts() -> Parts {
    let mut m = Parts::default();
    let tau = std::f32::consts::TAU;
    let (r, h, segs) = (0.075f32, 0.42f32, 10);
    let at = |a: f32, y: f32, r: f32| [a.sin() * r, y, a.cos() * r];
    // The body, and its foot (three.js's cylinder, capped; the top cap is under the dome).
    for i in 0..segs {
        let (a0, a1) = (tau * i as f32 / segs as f32, tau * (i + 1) as f32 / segs as f32);
        let (b0, b1, t0, t1) = (at(a0, 0.0, r), at(a1, 0.0, r), at(a0, h, r), at(a1, h, r));
        m.tri(b0, b1, t1, EXT_RED);
        m.tri(b0, t1, t0, EXT_RED);
        m.tri([0.0, 0.0, 0.0], b1, b0, EXT_RED);
    }
    // The dome: a half sphere of four rows from the body's top to its crown.
    let rows = 4;
    let ring = |k: usize, a: f32| {
        let th = std::f32::consts::FRAC_PI_2 * k as f32 / rows as f32;
        at(a, h + r * th.cos(), r * th.sin())
    };
    for k in 0..rows {
        for i in 0..segs {
            let (a0, a1) = (tau * i as f32 / segs as f32, tau * (i + 1) as f32 / segs as f32);
            // Row k is nearer the crown than row k + 1.
            let (u0, u1, d0, d1) = (ring(k, a0), ring(k, a1), ring(k + 1, a0), ring(k + 1, a1));
            m.tri(d0, d1, u1, EXT_RED);
            m.tri(d0, u1, u0, EXT_RED);
        }
    }
    // The valve block, 0.04 x 0.07 x 0.1 m about (0, 0.5, 0.02).
    m.cuboid([0.0, 0.5, 0.02], [0.04, 0.07, 0.1], EXT_DARK, false);
    // The nozzle: a cone 0.16 m long, 0.035 m in radius at its base, on eight sides, its tip at (0, 0.48, 0.22).
    let (tip, base_z, nr) = (EXTINGUISHER_NOZZLE_M, 0.06f32, 0.035f32);
    let base = |a: f32| [a.sin() * nr, tip[1] + a.cos() * nr, base_z];
    for i in 0..8 {
        let (a0, a1) = (tau * i as f32 / 8.0, tau * (i + 1) as f32 / 8.0);
        m.tri(base(a1), base(a0), tip, EXT_DARK);
        m.tri([0.0, tip[1], base_z], base(a0), base(a1), EXT_DARK);
    }
    m
}

/// The extinguisher (all-hands 4; the owner, 2026-10-12: "we had a fire extinguisher model, please use it from the JS
/// mockups"): docs/mockups/lib/firefight.js's `extinguisherGeo`, part for part. A red cylinder 0.42 m tall and 0.075 m
/// in radius on ten sides, a dome on top, a dark valve block and a dark cone nozzle pointing along +Z, painted by the
/// mockup's `paint`; upright in its own frame, its foot at the origin. 128 triangles.
pub fn extinguisher() -> MeshBuilder {
    let mut m = MeshBuilder::default();
    extinguisher_parts().lay(&mut m, [0.0; 3], [0.0, 0.0, 1.0], None);
    m
}

/// The yellow a wall's extinguisher breathes toward while a fire burns aboard (the owner, 2026-10-12: "fire extenguisher
/// should flash yellow (a soft transition to highlight them)", "when there is a fire"): its third colour set, painted
/// as the rest of it is.
const HIGHLIGHT: u32 = 0xffd23f;

/// Where a bracket holds its extinguisher in the safety point's frame (the wall point at the origin, +Z into the
/// room): the mockup's 0.58 m up and 0.1 m off the wall.
const HUNG_M: [f32; 3] = [0.0, 0.58, 0.1];

/// The safety points round the ship (safety-points 1, 5a; the owner, 2026-10-12: "there should be an extinguisher in
/// every room and spaced out evenly in corridors"), as the mockups draw them (`firefight.js`): each bracket's back
/// plate and strap and the glowing sign above, in one mesh; their extinguishers, in another, one after another in
/// `points`' order (`extinguisher().indices.len()` indices each), their third colour set the `HIGHLIGHT` yellow. `points` are each one's wall point and the way into
/// its room, ship frame; the meshes are about `origin`, and every point must be within 30 m of it (the vertex
/// format's reach).
pub fn safety_points(points: &[([f32; 3], [f32; 3])], origin: [f32; 3]) -> (MeshBuilder, MeshBuilder) {
    let mut fixed = Parts::default();
    fixed.cuboid([0.0, 0.8, 0.01], [0.2, 0.5, 0.02], EXT_DARK, false);
    fixed.cuboid([0.0, 0.78, 0.105], [0.19, 0.04, 0.17], EXT_DARK, false);
    fixed.cuboid([0.0, 1.9, 0.006], [0.30, 0.15, 0.012], SIGN_GLOW, true);
    let ext = extinguisher_parts();
    let (mut a, mut b) = (MeshBuilder::default(), MeshBuilder::default());
    for (wall, into) in points {
        let at = [wall[0] - origin[0], wall[1] - origin[1], wall[2] - origin[2]];
        fixed.lay(&mut a, at, *into, None);
        let hung = [at[0] + into[0] * HUNG_M[2], at[1] + HUNG_M[1], at[2] + into[2] * HUNG_M[2]];
        ext.lay(&mut b, hung, *into, Some(HIGHLIGHT));
    }
    (a, b)
}

/// Where the extinguisher's nozzle ends, in its own frame (`extinguisher`), metres: the spray leaves from here.
pub const EXTINGUISHER_NOZZLE_M: [f32; 3] = [0.0, 0.48, 0.22];

/// Which placeholder station (openspec/changes/patrol-mission design 3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StationKind {
    /// The relay at the Shoals: a 120 m truss with three dishes, a solar wing and a beacon.
    Relay,
}

/// A placeholder station: its albedo parts (lit by the sun's ambient cube), its glowing parts (flat) and its beacons
/// (drawn only while they blink on, each kept at least a few pixels across however far it is, as a lamp seen at night). Positions are stored at `1 / scale` of their size so a kilometre fits the
/// vertex's +/-32 m; draw them scaled by `scale`. Axes: the station's axis along +X, +Y up.
pub struct StationMesh {
    /// What a stored position is multiplied by to give metres.
    pub scale: f32,
    /// The structure, albedo.
    pub body: MeshBuilder,
    /// Lit windows and strips, their own light.
    pub lights: MeshBuilder,
    /// The navigation beacons, their own light.
    pub beacons: Vec<Beacon>,
}

/// A navigation beacon on a station: a glowing octahedron.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Beacon {
    /// Where it is in the station's frame, metres.
    pub at_m: [f32; 3],
    /// Its radius up close, metres.
    pub radius_m: f32,
    /// Its colour, a display multiplier (its own light).
    pub rgb: [f32; 3],
}

/// A beacon's lamp: an octahedron of radius 1 about the origin in `rgb`, scaled and placed when drawn.
pub fn beacon_lamp(rgb: [f32; 3]) -> MeshBuilder {
    let mut m = MeshBuilder::default();
    Shaper { k: 1.0 }.beacon(&mut m, glam::Vec3::ZERO, 1.0, rgb);
    m
}

impl StationMesh {
    /// The triangles of all three parts.
    pub fn triangles(&self) -> usize {
        self.body.triangles() + self.lights.triangles() + self.beacons.len() * beacon_lamp([1.0; 3]).triangles()
    }
}

/// Lays parts into a mesh at `1 / scale`, each face wound outward from the part's own centre.
struct Shaper {
    k: f32,
}

impl Shaper {
    fn at(&self, p: glam::Vec3) -> [f32; 3] {
        (p * self.k).to_array()
    }

    /// A quad whose outside is away from `inside`.
    fn face(&self, m: &mut MeshBuilder, q: [glam::Vec3; 4], inside: glam::Vec3, rgb: [f32; 3]) {
        let n = (q[1] - q[0]).cross(q[2] - q[0]);
        let mid = (q[0] + q[1] + q[2] + q[3]) / 4.0;
        let q = if n.dot(mid - inside) >= 0.0 { q } else { [q[3], q[2], q[1], q[0]] };
        m.quad(self.at(q[0]), self.at(q[1]), self.at(q[2]), self.at(q[3]), rgb);
    }

    /// A box of half extents `half` turned by `rot` about its centre `c`, metres.
    fn block(&self, m: &mut MeshBuilder, c: glam::Vec3, half: glam::Vec3, rot: glam::Quat, rgb: [f32; 3]) {
        let v = |x: f32, y: f32, z: f32| c + rot * (half * glam::Vec3::new(x, y, z));
        for (a, b) in [(0usize, 1usize), (1, 2), (0, 2)] {
            for s in [-1.0f32, 1.0] {
                let corner = |u: f32, w: f32| {
                    let mut e = [0.0f32; 3];
                    e[3 - a - b] = s;
                    e[a] = u;
                    e[b] = w;
                    v(e[0], e[1], e[2])
                };
                self.face(m, [corner(-1.0, -1.0), corner(1.0, -1.0), corner(1.0, 1.0), corner(-1.0, 1.0)], c, rgb);
            }
        }
    }

    /// An octahedron of radius `r` at `c`: a beacon.
    fn beacon(&self, m: &mut MeshBuilder, c: glam::Vec3, r: f32, rgb: [f32; 3]) {
        let d = [glam::Vec3::X, glam::Vec3::Z, -glam::Vec3::X, -glam::Vec3::Z];
        for i in 0..4 {
            let (a, b) = (c + d[i] * r, c + d[(i + 1) % 4] * r);
            m.tri(self.at(c + glam::Vec3::Y * r), self.at(b), self.at(a), rgb);
            m.tri(self.at(c - glam::Vec3::Y * r), self.at(a), self.at(b), rgb);
        }
    }

    /// A dish: a shallow cone opening upward, its rim of radius `r` at `c`, `depth` deep.
    fn dish(&self, m: &mut MeshBuilder, c: glam::Vec3, r: f32, depth: f32, rgb: [f32; 3]) {
        let rim = |i: usize| {
            let a = std::f32::consts::TAU * i as f32 / 8.0;
            c + glam::Vec3::new(r * a.cos(), 0.0, r * a.sin())
        };
        let low = c - glam::Vec3::Y * depth;
        for i in 0..8 {
            let (a, b) = (rim(i), rim((i + 1) % 8));
            // Both sides, so the dish reads from above and below.
            m.tri(self.at(low), self.at(b), self.at(a), rgb);
            m.tri(self.at(low), self.at(a), self.at(b), rgb);
        }
    }
}

/// The placeholder station of `kind` (patrol-mission design 3), generated as the Hound is until its model exists.
pub fn station(kind: StationKind) -> StationMesh {
    use glam::{Quat, Vec3};
    let (hull, dark, pale) = ([0.62, 0.64, 0.68], [0.30, 0.32, 0.36], [0.78, 0.80, 0.83]);
    let (red, green) = ([2.0, 0.35, 0.25], [0.4, 2.0, 0.7]);
    let mut out =
        StationMesh { scale: 1.0, body: MeshBuilder::default(), lights: MeshBuilder::default(), beacons: Vec::new() };
    match kind {
        StationKind::Relay => {
            out.scale = 4.0;
            let sh = Shaper { k: 1.0 / out.scale };
            sh.block(&mut out.body, Vec3::ZERO, Vec3::new(60.0, 3.0, 3.0), Quat::IDENTITY, hull);
            for x in [-40.0, 0.0, 40.0] {
                sh.block(&mut out.body, Vec3::new(x, 6.0, 0.0), Vec3::new(1.0, 3.0, 1.0), Quat::IDENTITY, dark);
                sh.dish(&mut out.body, Vec3::new(x, 16.0, 0.0), 14.0, 6.0, pale);
            }
            sh.block(
                &mut out.body,
                Vec3::new(0.0, -10.0, 0.0),
                Vec3::new(30.0, 0.25, 13.0),
                Quat::IDENTITY,
                [0.16, 0.22, 0.42],
            );
            sh.block(
                &mut out.lights,
                Vec3::new(0.0, 0.0, 3.1),
                Vec3::new(40.0, 0.4, 0.1),
                Quat::IDENTITY,
                [0.5, 1.8, 1.0],
            );
            out.beacons.push(Beacon { at_m: [0.0, 24.0, 0.0], radius_m: 3.0, rgb: red });
            out.beacons.push(Beacon { at_m: [62.0, 0.0, 0.0], radius_m: 2.0, rgb: green });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_relay_fits_its_budget_and_the_vertex_format() {
        // patrol-mission design 3 and 13: the relay about 300 triangles; generating it packs every position, so a part
        // outside the vertex's +/-32 m at its scale would panic here.
        let relay = station(StationKind::Relay);
        assert!(relay.triangles() <= 300, "the relay: {} triangles", relay.triangles());
        assert!(!relay.beacons.is_empty() && !relay.lights.indices.is_empty());
    }

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
            // Its origin is its foot: the foot's cap, the valve's underside and the nozzle's back face it.
            ("extinguisher", extinguisher(), 0.7),
        ] {
            let share = outward_share(&m);
            assert!(share >= floor, "{name}: {share:.2} of its faces wind outward");
        }
    }

    /// A wall's extinguisher carries its paint in its first colour set and the highlight's yellow in its third (what its
    /// draw breathes toward while a fire burns); its bracket and sign have no yellow.
    #[test]
    fn the_wall_extinguishers_carry_their_yellow() {
        let (fixed, hung) = safety_points(&[([0.0, 0.0, 0.0], [0.0, 0.0, 1.0])], [0.0; 3]);
        let set = |v: &[u8], k: usize| [v[12 + k * 4], v[13 + k * 4], v[14 + k * 4]];
        for v in hung.vertices.chunks(28) {
            let (paint, yellow) = (set(v, 0), set(v, 2));
            assert!(yellow[0] > 60 && yellow[1] > yellow[0] / 2 && yellow[2] < yellow[1] / 2, "yellow: {yellow:?}");
            assert!(paint[1] < paint[0] || paint[0] < 30, "the paint is red or near black: {paint:?}");
        }
        assert!(fixed.vertices.chunks(28).all(|v| set(v, 0) == set(v, 2)), "the bracket and sign do not change");
    }

    /// The extinguisher is the mockup's (docs/mockups/lib/firefight.js `extinguisherGeo`): 0.075 m in radius and from its
    /// foot to its valve's top 0.535 m, the nozzle reaching 0.22 m forward, in its red and near black.
    #[test]
    fn the_extinguisher_is_the_mockups() {
        let m = extinguisher();
        let mut lo = [f32::MAX; 3];
        let mut hi = [f32::MIN; 3];
        let mut radius = 0.0f32;
        let mut colours = std::collections::BTreeSet::new();
        for v in m.vertices.chunks(28) {
            let p: Vec<f32> =
                (0..3).map(|k| f32::from(i16::from_le_bytes([v[k * 2], v[k * 2 + 1]])) / 1024.0).collect();
            for k in 0..3 {
                lo[k] = lo[k].min(p[k]);
                hi[k] = hi[k].max(p[k]);
            }
            if p[1] < 0.42 {
                radius = radius.max(p[0].hypot(p[2]));
            }
            colours.insert(v[12] > v[13] * 2);
        }
        let near = |a: f32, b: f32| (a - b).abs() < 0.003;
        assert!(near(radius, 0.075), "radius: {radius}");
        assert!(near(lo[1], 0.0) && near(hi[1], 0.535), "height: {lo:?} {hi:?}");
        assert!(near(hi[2], 0.22), "the nozzle's reach: {hi:?}");
        assert_eq!(colours.len(), 2, "red and near black");
        assert!(m.triangles() < 140, "{} triangles", m.triangles());
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
