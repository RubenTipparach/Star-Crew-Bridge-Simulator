//! The crew figure (openspec/changes/crew-npcs, design 7): a low-poly standing body 1.8 m tall, its tunic in a
//! department's colour, built by `deckc` into the deck file (a generator, CLAUDE.md 9) as a compartment of the deck
//! `figure`, one a department. The client draws one a bot with the flat program, its feet at the origin and its
//! front facing +z.
//!
//! Seven boxes, 84 triangles: legs, torso, arms, neck, head, and a badge on the chest so the front reads. Light is
//! baked from above and the front, so it reads anywhere on the decks.

/// One triangle corner: position (metres, feet at the origin), normal, and the baked colour (a linear multiplier).
pub struct FigureVertex {
    /// Position, metres.
    pub position_m: [f32; 3],
    /// Unit normal.
    pub normal: [f32; 3],
    /// Baked colour, linear, 0-2.
    pub colour: [f32; 3],
}

const TROUSERS: [f32; 3] = [0.03, 0.035, 0.045];
const SKIN: [f32; 3] = [0.55, 0.36, 0.24];
const HAIR: [f32; 3] = [0.03, 0.02, 0.015];
const BADGE: [f32; 3] = [0.9, 0.7, 0.25];

/// An axis-aligned box from `lo` to `hi`, wound counter-clockwise seen from outside, lit from above and the front.
fn cuboid(out: &mut Vec<FigureVertex>, lo: [f32; 3], hi: [f32; 3], colour: [f32; 3]) {
    let light = |n: [f32; 3]| {
        let l = [0.3f32, 0.8, 0.52];
        let k = 0.45 + 0.65 * (n[0] * l[0] + n[1] * l[1] + n[2] * l[2]).max(0.0);
        colour.map(|c| (c * k * 2.0).min(2.0))
    };
    let (x0, y0, z0, x1, y1, z1) = (lo[0], lo[1], lo[2], hi[0], hi[1], hi[2]);
    // Each face: normal and four corners counter-clockwise seen from outside.
    let faces: [([f32; 3], [[f32; 3]; 4]); 6] = [
        ([1.0, 0.0, 0.0], [[x1, y0, z1], [x1, y0, z0], [x1, y1, z0], [x1, y1, z1]]),
        ([-1.0, 0.0, 0.0], [[x0, y0, z0], [x0, y0, z1], [x0, y1, z1], [x0, y1, z0]]),
        ([0.0, 1.0, 0.0], [[x0, y1, z1], [x1, y1, z1], [x1, y1, z0], [x0, y1, z0]]),
        ([0.0, -1.0, 0.0], [[x0, y0, z0], [x1, y0, z0], [x1, y0, z1], [x0, y0, z1]]),
        ([0.0, 0.0, 1.0], [[x0, y0, z1], [x1, y0, z1], [x1, y1, z1], [x0, y1, z1]]),
        ([0.0, 0.0, -1.0], [[x1, y0, z0], [x0, y0, z0], [x0, y1, z0], [x1, y1, z0]]),
    ];
    for (n, q) in faces {
        let c = light(n);
        for i in [0, 1, 2, 0, 2, 3] {
            out.push(FigureVertex { position_m: q[i], normal: n, colour: c });
        }
    }
}

/// A figure in `tunic` (linear colour).
pub fn build(tunic: [f32; 3]) -> Vec<FigureVertex> {
    let mut v = Vec::with_capacity(7 * 36);
    cuboid(&mut v, [-0.19, 0.0, -0.1], [-0.02, 0.86, 0.1], TROUSERS);
    cuboid(&mut v, [0.02, 0.0, -0.1], [0.19, 0.86, 0.1], TROUSERS);
    cuboid(&mut v, [-0.22, 0.86, -0.12], [0.22, 1.45, 0.12], tunic);
    cuboid(&mut v, [-0.32, 0.8, -0.07], [-0.23, 1.42, 0.07], tunic);
    cuboid(&mut v, [0.23, 0.8, -0.07], [0.32, 1.42, 0.07], tunic);
    cuboid(&mut v, [-0.11, 1.45, -0.11], [0.11, 1.78, 0.12], SKIN);
    // Hair over the crown and back; the badge stands 1 cm proud of the tunic (CLAUDE.md 8).
    cuboid(&mut v, [-0.12, 1.66, -0.125], [0.12, 1.8, 0.06], HAIR);
    cuboid(&mut v, [0.07, 1.3, 0.12], [0.13, 1.36, 0.13], BADGE);
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_figure_is_about_a_hundred_triangles_wound_outward_and_stands_on_its_feet() {
        let v = build([0.5, 0.1, 0.1]);
        assert!(v.len() / 3 <= 100, "crew-npcs 7: about 100 triangles: {}", v.len() / 3);
        let min_y = v.iter().map(|p| p.position_m[1]).fold(f32::MAX, f32::min);
        assert!(min_y.abs() < 1e-6, "feet at the origin");
        for t in v.chunks_exact(3) {
            let (a, b, c) = (t[0].position_m, t[1].position_m, t[2].position_m);
            let e1 = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
            let e2 = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
            let n = [e1[1] * e2[2] - e1[2] * e2[1], e1[2] * e2[0] - e1[0] * e2[2], e1[0] * e2[1] - e1[1] * e2[0]];
            let d = n[0] * t[0].normal[0] + n[1] * t[0].normal[1] + n[2] * t[0].normal[2];
            assert!(d > 0.0, "a triangle wound against its normal is culled");
        }
    }
}
