//! The crew figure (openspec/changes/crew-npcs, design 7): a low-poly standing body 1.8 m tall, its tunic in a
//! department's colour, built by `deckc` into the deck file (a generator, CLAUDE.md 9) as a compartment of the deck
//! `figure`, one a department. The client draws one a bot with the flat program, its feet at the origin and its
//! front facing +z.
//!
//! Nine boxes, 96 triangles: legs, torso, arms, head, hair in two pieces, and a badge on the chest so the front
//! reads. Faces no eye can reach (buried in the torso, under the hair, behind the badge) are left out. Its colours
//! are unlit albedo: the client lights it from the light probes around it (openspec/changes/light-baking, design
//! section 16), or from `sc_core::probes::fallback_cube` where there are none.

/// One triangle corner: position (metres, feet at the origin), normal, and the albedo (linear).
pub struct FigureVertex {
    /// Position, metres.
    pub position_m: [f32; 3],
    /// Unit normal.
    pub normal: [f32; 3],
    /// Albedo, linear, 0-1.
    pub colour: [f32; 3],
}

const TROUSERS: [f32; 3] = [0.03, 0.035, 0.045];
const SKIN: [f32; 3] = [0.55, 0.36, 0.24];
const HAIR: [f32; 3] = [0.03, 0.02, 0.015];
const BADGE: [f32; 3] = [0.9, 0.7, 0.25];

/// A box's faces, as `cuboid` emits them: +x, -x, top, bottom, front (+z), back.
const TOP: usize = 2;
const BOTTOM: usize = 3;
const BACK: usize = 5;

/// An axis-aligned box from `lo` to `hi`, wound counter-clockwise seen from outside, in one albedo, without the faces
/// in `hidden` (indices as `TOP`, `BOTTOM`, `BACK`).
fn cuboid(out: &mut Vec<FigureVertex>, lo: [f32; 3], hi: [f32; 3], colour: [f32; 3], hidden: &[usize]) {
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
    for (k, (n, q)) in faces.into_iter().enumerate() {
        if hidden.contains(&k) {
            continue;
        }
        for i in [0, 1, 2, 0, 2, 3] {
            out.push(FigureVertex { position_m: q[i], normal: n, colour });
        }
    }
}

/// A figure in `tunic` (linear colour).
pub fn build(tunic: [f32; 3]) -> Vec<FigureVertex> {
    let mut v = Vec::with_capacity(7 * 36);
    cuboid(&mut v, [-0.19, 0.0, -0.1], [-0.02, 0.86, 0.1], TROUSERS, &[TOP]);
    cuboid(&mut v, [0.02, 0.0, -0.1], [0.19, 0.86, 0.1], TROUSERS, &[TOP]);
    cuboid(&mut v, [-0.22, 0.86, -0.12], [0.22, 1.45, 0.12], tunic, &[]);
    cuboid(&mut v, [-0.32, 0.8, -0.07], [-0.23, 1.42, 0.07], tunic, &[]);
    cuboid(&mut v, [0.23, 0.8, -0.07], [0.32, 1.42, 0.07], tunic, &[]);
    cuboid(&mut v, [-0.11, 1.45, -0.11], [0.11, 1.78, 0.12], SKIN, &[TOP, BOTTOM]);
    // Hair (owner, 2026-10-10: "what is wrong with crew hair?"; it stopped 6 cm short of the face, so from the front
    // the crew looked bald): a crown with a fringe to the forehead, and a cap down the sides to the ears and the back
    // to the nape. Every face stands 1 cm proud of the head, and the two pieces overlap by 1 cm rather than meet in
    // one plane (CLAUDE.md 8). The badge stands 1 cm proud of the tunic.
    cuboid(&mut v, [-0.12, 1.71, -0.125], [0.12, 1.8, 0.13], HAIR, &[]);
    cuboid(&mut v, [-0.12, 1.56, -0.125], [0.12, 1.72, 0.03], HAIR, &[TOP]);
    cuboid(&mut v, [0.07, 1.3, 0.12], [0.13, 1.36, 0.13], BADGE, &[BACK]);
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

    #[test]
    fn the_hair_reaches_the_forehead_and_stands_clear_of_the_head() {
        // Regression: the hair ended 6 cm behind the face, so from the front the crew looked bald.
        let v = build([0.5, 0.1, 0.1]);
        let hair: Vec<_> = v.iter().filter(|p| p.colour == HAIR).collect();
        let front = hair.iter().map(|p| p.position_m[2]).fold(f32::MIN, f32::max);
        assert!(front > 0.12, "the fringe comes past the face (z 0.12 m): {front}");
        let low = hair.iter().map(|p| p.position_m[1]).fold(f32::MAX, f32::min);
        assert!(low < 1.6, "the sides come down to the ears: {low}");
        for p in &hair {
            let [x, y, z] = p.position_m;
            let on_head_face = (x.abs() - 0.11).abs() < 1e-6 || (y - 1.78).abs() < 1e-6 || (z - 0.12).abs() < 1e-6;
            assert!(!on_head_face, "no hair corner lies in a face of the head (CLAUDE.md 8): {:?}", p.position_m);
        }
    }
}
