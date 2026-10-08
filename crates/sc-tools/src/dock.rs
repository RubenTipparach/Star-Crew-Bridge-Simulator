//! The space dock's frame (openspec/changes/deck-pipeline, design section 13b): girders round the hull, built from
//! `data/space/exterior.json` `dock` for `deckc`, which packs them into the deck file as compartments of the deck
//! `outside`.
//!
//! It is a generator in the tools, not in the client, because meshes are files built by committed generators
//! (CLAUDE.md 9): the deck file carries the dock like any room, and the client only draws it.
//!
//! The frame: a ring of four girders every `ring_spacing_m` along z, six longitudinal girders between rings (the four
//! corners and the middle of the top and the bottom), an X of braces on each side of a bay (the two diagonals 1.1
//! braces apart in x, so their faces never share a plane, CLAUDE.md 8), and a work light at each ring's four inner
//! corners. Each bay is its own compartment, its ring and girders within half a bay of its centre, so its vertices fit
//! the deck vertex's +/-32 m. Light is baked here: the sun's Lambert term over an ambient floor, tinted by the steel's
//! colour; the work lights are full bright in every state.

use sc_core::exterior::{linear, normalized, Dock};

/// One triangle corner, in ship coordinates.
pub struct DockVertex {
    /// Position, metres.
    pub position_m: [f32; 3],
    /// Unit normal.
    pub normal: [f32; 3],
    /// Texture coordinates in layer spans.
    pub uv: [f32; 2],
    /// True for a work light (the light panel's layer), false for steel.
    pub light: bool,
    /// The baked light, a linear multiplier (0-2).
    pub colour: [f32; 3],
}

/// One bay of the frame: an id and its triangles (three vertices each).
pub struct DockBay {
    /// `dock_<k>`, from the stern.
    pub id: String,
    /// Its triangles.
    pub vertices: Vec<DockVertex>,
}

type V3 = [f32; 3];

fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn add(a: V3, b: V3) -> V3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
fn mul(a: V3, k: f32) -> V3 {
    [a[0] * k, a[1] * k, a[2] * k]
}
fn dot(a: V3, b: V3) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn cross(a: V3, b: V3) -> V3 {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}
fn unit(a: V3) -> V3 {
    mul(a, 1.0 / dot(a, a).sqrt())
}

/// A box from `a` to `b` (its axis) with a square section of `s` metres, wound counter-clockwise seen from outside.
fn beam(out: &mut Vec<DockVertex>, a: V3, b: V3, s: f32, light: bool, shade: &dyn Fn(V3, bool) -> V3, span: f32) {
    let ax = unit(sub(b, a));
    // A side axis: perpendicular to the beam, preferring the horizontal.
    let up = if ax[1].abs() > 0.9 { [1.0, 0.0, 0.0] } else { [0.0, 1.0, 0.0] };
    let v = unit(cross(up, ax));
    let w = cross(ax, v);
    let h = s / 2.0;
    let len = dot(sub(b, a), ax);
    // Six faces: (normal, two in-plane axes whose cross is the normal, half sizes, centre).
    let mid = mul(add(a, b), 0.5);
    let faces: [(V3, V3, V3, f32, f32); 6] = [
        (v, w, ax, h, len / 2.0),
        (mul(v, -1.0), ax, w, len / 2.0, h),
        (w, ax, v, len / 2.0, h),
        (mul(w, -1.0), v, ax, h, len / 2.0),
        (ax, v, w, h, h),
        (mul(ax, -1.0), w, v, h, h),
    ];
    for (n, e1, e2, h1, h2) in faces {
        let off = if dot(n, ax).abs() > 0.5 { len / 2.0 } else { h };
        let c = add(mid, mul(n, off));
        let colour = shade(n, light);
        let corner = |s1: f32, s2: f32| add(add(c, mul(e1, s1 * h1)), mul(e2, s2 * h2));
        let quad = [corner(-1.0, -1.0), corner(1.0, -1.0), corner(1.0, 1.0), corner(-1.0, 1.0)];
        let uv = |p: V3| [(dot(sub(p, quad[0]), e1)) / span, (dot(sub(p, quad[0]), e2)) / span];
        for i in [0, 1, 2, 0, 2, 3] {
            out.push(DockVertex { position_m: quad[i], normal: n, uv: uv(quad[i]), light, colour });
        }
    }
}

/// The frame's bays from the dock's data, `sun_dir` and `sun_srgb` the sun's (data/space/exterior.json), `span_m` the
/// material's layer span.
pub fn build(d: &Dock, sun_dir: [f64; 3], sun_srgb: [f64; 3], span_m: f32) -> Vec<DockBay> {
    let sun = normalized(sun_dir);
    let sun_c = linear(sun_srgb);
    let steel = linear(d.steel_srgb);
    let lamp = linear(d.light_srgb);
    let amb = d.ambient as f32;
    let shade = move |n: V3, light: bool| -> V3 {
        if light {
            return [2.0 * lamp[0].min(1.0), 2.0 * lamp[1].min(1.0), 2.0 * lamp[2].min(1.0)];
        }
        let k = amb + (1.0 - amb) * dot(n, sun).max(0.0);
        std::array::from_fn(|i| (1.6 * steel[i] * sun_c[i] * k).min(2.0))
    };
    let c = d.center_m.map(|x| x as f32);
    let (hw, hh, g, br, l) =
        ((d.width_m / 2.0) as f32, (d.height_m / 2.0) as f32, d.girder_m as f32, d.brace_m as f32, d.light_m as f32);
    let n = (d.length_m / d.ring_spacing_m).round().max(1.0) as usize;
    let sp = (d.length_m / n as f64) as f32;
    let z0 = c[2] - (d.length_m / 2.0) as f32;
    let (x0, x1, y0, y1) = (c[0] - hw, c[0] + hw, c[1] - hh, c[1] + hh);
    let mut bays = Vec::new();
    for k in 0..n {
        let mut out = Vec::new();
        let za = z0 + sp * k as f32;
        let zb = za + sp;
        let ring = |z: f32, out: &mut Vec<DockVertex>| {
            beam(out, [x0 - g / 2.0, y1, z], [x1 + g / 2.0, y1, z], g, false, &shade, span_m);
            beam(out, [x0 - g / 2.0, y0, z], [x1 + g / 2.0, y0, z], g, false, &shade, span_m);
            beam(out, [x0, y0 + g / 2.0, z], [x0, y1 - g / 2.0, z], g, false, &shade, span_m);
            beam(out, [x1, y0 + g / 2.0, z], [x1, y1 - g / 2.0, z], g, false, &shade, span_m);
            // Work lights in the inner corners, 1 cm clear of the girders (CLAUDE.md 8).
            let e = g / 2.0 + l / 2.0 + 0.01;
            for (x, y) in [(x0 + e, y1 - e), (x1 - e, y1 - e), (x0 + e, y0 + e), (x1 - e, y0 + e)] {
                beam(out, [x, y, z - l / 2.0], [x, y, z + l / 2.0], l, true, &shade, span_m);
            }
        };
        ring(za, &mut out);
        if k + 1 == n {
            ring(zb, &mut out);
        }
        // Longitudinals between the rings' faces.
        let (zs, ze) = (za + g / 2.0, zb - g / 2.0);
        for (x, y) in [(x0, y0), (x0, y1), (x1, y0), (x1, y1), (c[0], y0), (c[0], y1)] {
            beam(&mut out, [x, y, zs], [x, y, ze], g, false, &shade, span_m);
        }
        // An X of braces on each side; the second diagonal 1.1 braces further in.
        for (x, inward) in [(x0, 1.0f32), (x1, -1.0)] {
            let x2 = x + inward * br * 1.1;
            beam(&mut out, [x, y0 + g / 2.0, zs], [x, y1 - g / 2.0, ze], br, false, &shade, span_m);
            beam(&mut out, [x2, y1 - g / 2.0, zs], [x2, y0 + g / 2.0, ze], br, false, &shade, span_m);
        }
        bays.push(DockBay { id: format!("dock_{k}"), vertices: out });
    }
    bays
}

#[cfg(test)]
mod tests {
    use super::*;
    use sc_core::exterior::ExteriorData;

    fn data() -> ExteriorData {
        let text = include_str!("../../../data/space/exterior.json");
        sc_core::data::parse("data/space/exterior.json", text).expect("the shipped exterior loads")
    }

    #[test]
    fn every_bay_fits_the_deck_vertex_about_its_centre() {
        let e = data();
        for b in build(&e.dock, e.sun.dir, e.sun.colour_srgb, 4.0) {
            let mut lo = [f32::MAX; 3];
            let mut hi = [f32::MIN; 3];
            for v in &b.vertices {
                for k in 0..3 {
                    lo[k] = lo[k].min(v.position_m[k]);
                    hi[k] = hi[k].max(v.position_m[k]);
                }
            }
            for k in 0..3 {
                assert!(
                    hi[k] - lo[k] < 63.0,
                    "{} spans {} m on axis {k}: the vertex holds +/-32 m",
                    b.id,
                    hi[k] - lo[k]
                );
            }
        }
    }

    #[test]
    fn the_frame_costs_about_fifteen_hundred_triangles_and_faces_outward() {
        let e = data();
        let bays = build(&e.dock, e.sun.dir, e.sun.colour_srgb, 4.0);
        let tris: usize = bays.iter().map(|b| b.vertices.len() / 3).sum();
        assert!((1000..=2000).contains(&tris), "13b's budget is about 1,500 triangles: {tris}");
        for b in &bays {
            for t in b.vertices.chunks_exact(3) {
                let n = cross(sub(t[1].position_m, t[0].position_m), sub(t[2].position_m, t[0].position_m));
                assert!(
                    dot(n, t[0].normal) > 0.0,
                    "{}: a triangle wound against its normal is culled from outside",
                    b.id
                );
            }
        }
    }
}
