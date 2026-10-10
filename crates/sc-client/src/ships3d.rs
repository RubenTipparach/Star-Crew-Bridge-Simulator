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
