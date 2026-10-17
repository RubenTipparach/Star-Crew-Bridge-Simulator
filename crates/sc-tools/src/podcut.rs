//! The turret pods' cut in the hull (tern-exterior 3; the owner, 2026-10-17: "hull is clipping in the gunner pod,
//! there's no hole in the geometry carved for that"). A pod is a room that stands up (or down, or out) through the
//! hull's skin into its turret: the skin, its hazard plating and the turret base's floor crossed the dorsal pod at
//! chest height, so a gunner standing or seated there looked at the hull from inside it and could not see the floor or
//! the ladder down. `deckc` cuts every pod's prism out of the hull's triangles as it packs them, so the opening follows
//! the pods in `data/ships/<id>/layout.json` wherever they are moved.
//!
//! The cut keeps every part of a triangle outside the prism and drops the part inside, splitting a triangle that
//! crosses a wall into pieces; a piece's corners take the position, normal, texture coordinate and colour at that
//! point of the triangle (linear across it, so the paint on the atlas lands exactly where it did: no texture changes).
//! The turret's base on the model encloses each pod's footprint, so the opening is never seen from outside.

use crate::glb::Primitive;

/// How far the cut reaches past a pod's floor and ceiling, metres: a face of the model lying in a pod's ceiling (the
/// dorsal turret's floor plate at 9.0 m) is inside the cut, not left to fight the ceiling's own face.
const CAP_M: f32 = 0.01;

/// A pod's air: a convex outline in the plan (x, z, metres) between two heights.
#[derive(Clone, Debug, PartialEq)]
pub struct Prism {
    pub y: [f32; 2],
    pub poly: Vec<[f32; 2]>,
}

/// Every pod's prisms in a layout (`compartments` of kind `pod`, one a brush).
pub fn pods(layout: &serde_json::Value) -> Result<Vec<Prism>, String> {
    let mut out = Vec::new();
    for c in layout["compartments"].as_array().ok_or("the layout has no compartments")? {
        if c["kind"] != "pod" {
            continue;
        }
        for b in c["brushes"].as_array().ok_or_else(|| format!("pod {} has no brushes", c["id"]))? {
            let num = |v: &serde_json::Value| v.as_f64().map(|x| x as f32);
            let y = b["y"].as_array().and_then(|y| Some([num(y.first()?)?, num(y.get(1)?)?]));
            let poly: Option<Vec<[f32; 2]>> =
                b["poly"].as_array().and_then(|p| p.iter().map(|q| Some([num(q.get(0)?)?, num(q.get(1)?)?])).collect());
            match (y, poly) {
                (Some(y), Some(poly)) if poly.len() >= 3 && y[0] < y[1] => out.push(Prism { y, poly }),
                _ => return Err(format!("pod {}: a brush without a y range and an outline", c["id"])),
            }
        }
    }
    Ok(out)
}

/// A triangle's corner with everything that is drawn with it.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Corner {
    p: [f32; 3],
    n: [f32; 3],
    uv: [f32; 2],
    c: [f32; 3],
}

fn lerp(a: &Corner, b: &Corner, t: f32) -> Corner {
    let l3 = |x: [f32; 3], y: [f32; 3]| std::array::from_fn(|k| x[k] + (y[k] - x[k]) * t);
    let n = l3(a.n, b.n);
    let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt().max(1e-6);
    Corner {
        p: l3(a.p, b.p),
        n: n.map(|v| v / len),
        uv: [a.uv[0] + (b.uv[0] - a.uv[0]) * t, a.uv[1] + (b.uv[1] - a.uv[1]) * t],
        c: l3(a.c, b.c),
    }
}

/// A prism as planes `(normal, offset)`, each with `normal . p + offset > 0` inside.
fn planes(pr: &Prism) -> Vec<([f32; 3], f32)> {
    let n = pr.poly.len() as f32;
    let mid = pr.poly.iter().fold([0.0, 0.0], |m, q| [m[0] + q[0] / n, m[1] + q[1] / n]);
    let mut out = vec![([0.0, 1.0, 0.0], -(pr.y[0] - CAP_M)), ([0.0, -1.0, 0.0], pr.y[1] + CAP_M)];
    for (i, a) in pr.poly.iter().enumerate() {
        let b = pr.poly[(i + 1) % pr.poly.len()];
        // The edge's normal in the plan, turned toward the middle.
        let (mut nx, mut nz) = (-(b[1] - a[1]), b[0] - a[0]);
        if nx * (mid[0] - a[0]) + nz * (mid[1] - a[1]) < 0.0 {
            (nx, nz) = (-nx, -nz);
        }
        let len = nx.hypot(nz).max(1e-9);
        let (nx, nz) = (nx / len, nz / len);
        out.push(([nx, 0.0, nz], -(nx * a[0] + nz * a[1])));
    }
    out
}

fn side(pl: &([f32; 3], f32), p: [f32; 3]) -> f32 {
    pl.0[0] * p[0] + pl.0[1] * p[1] + pl.0[2] * p[2] + pl.1
}

/// The part of convex polygon `poly` where `f >= 0` (Sutherland-Hodgman, order kept).
fn clip(poly: &[Corner], f: impl Fn(&Corner) -> f32) -> Vec<Corner> {
    let mut out = Vec::new();
    for i in 0..poly.len() {
        let (a, b) = (&poly[i], &poly[(i + 1) % poly.len()]);
        let (fa, fb) = (f(a), f(b));
        if fa >= 0.0 {
            out.push(*a);
        }
        if (fa >= 0.0) != (fb >= 0.0) {
            out.push(lerp(a, b, fa / (fa - fb)));
        }
    }
    out
}

/// The pieces of triangle `t` outside prism `pr`, as convex polygons; None when the prism does not touch it (keep it
/// as it is). Empty when the triangle lies wholly inside.
fn outside(t: &[Corner; 3], pr: &Prism) -> Option<Vec<Vec<Corner>>> {
    let pls = planes(pr);
    // Wholly outside one plane: untouched.
    if pls.iter().any(|pl| t.iter().all(|c| side(pl, c.p) <= 0.0)) {
        return None;
    }
    let mut pieces = Vec::new();
    let mut rest: Vec<Corner> = t.to_vec();
    for pl in &pls {
        let out = clip(&rest, |c| -side(pl, c.p));
        if out.len() >= 3 {
            pieces.push(out);
        }
        rest = clip(&rest, |c| side(pl, c.p));
        if rest.len() < 3 {
            return Some(pieces);
        }
    }
    Some(pieces)
}

/// Twice a polygon's area, metres squared (slivers from a corner on a wall are dropped).
fn area2(poly: &[Corner]) -> f32 {
    let mut s = [0.0f32; 3];
    for i in 1..poly.len().saturating_sub(1) {
        let (a, b, c) = (poly[0].p, poly[i].p, poly[i + 1].p);
        let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        let v = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
        let x = [u[1] * v[2] - u[2] * v[1], u[2] * v[0] - u[0] * v[2], u[0] * v[1] - u[1] * v[0]];
        s = [s[0] + x[0], s[1] + x[1], s[2] + x[2]];
    }
    (s[0] * s[0] + s[1] * s[1] + s[2] * s[2]).sqrt()
}

/// Cut every prism out of a primitive's triangles. Triangles no prism touches keep their corners and order; the rest
/// are replaced, in place, by their pieces outside (fanned from each piece's first corner, so their winding is kept).
/// Returns how many of its triangles were cut.
pub fn cut(p: &mut Primitive, prisms: &[Prism]) -> usize {
    let corner = |p: &Primitive, i: u32| {
        let i = i as usize;
        Corner { p: p.positions[i], n: p.normals[i], uv: p.uvs[i], c: p.colours[i] }
    };
    let mut indices = Vec::with_capacity(p.indices.len());
    let mut cut = 0;
    let tris: Vec<[u32; 3]> = p.indices.chunks_exact(3).map(|t| [t[0], t[1], t[2]]).collect();
    for t in tris {
        let mut pieces: Vec<Vec<Corner>> = vec![t.iter().map(|&i| corner(p, i)).collect()];
        let mut touched = false;
        for pr in prisms {
            let mut next = Vec::new();
            for piece in pieces {
                // A piece is a convex polygon: fan it into triangles for the next prism.
                for k in 1..piece.len() - 1 {
                    let tri = [piece[0], piece[k], piece[k + 1]];
                    match outside(&tri, pr) {
                        None => next.push(tri.to_vec()),
                        Some(out) => {
                            touched = true;
                            next.extend(out);
                        }
                    }
                }
            }
            pieces = next;
        }
        if !touched {
            indices.extend_from_slice(&t);
            continue;
        }
        cut += 1;
        for piece in pieces.iter().filter(|q| q.len() >= 3 && area2(q) > 1e-6) {
            let first = p.positions.len() as u32;
            for c in piece {
                p.positions.push(c.p);
                p.normals.push(c.n);
                p.uvs.push(c.uv);
                p.colours.push(c.c);
            }
            // A corner on a wall can leave three corners in a line: no triangle of nothing is drawn.
            for k in 1..piece.len() - 1 {
                if area2(&[piece[0], piece[k], piece[k + 1]]) > 1e-6 {
                    let k = k as u32;
                    indices.extend_from_slice(&[first, first + k, first + k + 1]);
                }
            }
        }
    }
    p.indices = indices;
    cut
}

/// Whether point `q` lies inside prism `pr`, `inset` metres in from every face (the checks' question).
#[cfg(test)]
pub fn inside(pr: &Prism, q: [f32; 3], inset: f32) -> bool {
    planes(pr).iter().all(|pl| side(pl, q) > inset + CAP_M)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A square floor plate 4 m on a side at height `y`, two triangles facing up.
    fn plate(y: f32) -> Primitive {
        Primitive {
            material: "plate".into(),
            positions: vec![[-2.0, y, -2.0], [2.0, y, -2.0], [2.0, y, 2.0], [-2.0, y, 2.0]],
            normals: vec![[0.0, 1.0, 0.0]; 4],
            uvs: vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
            colours: vec![[0.5; 3]; 4],
            indices: vec![0, 2, 1, 0, 3, 2],
            image: None,
        }
    }

    fn pod() -> Prism {
        Prism { y: [0.0, 2.0], poly: vec![[-1.0, -1.0], [1.0, -1.0], [1.0, 1.0], [-1.0, 1.0]] }
    }

    fn area(p: &Primitive) -> f32 {
        p.indices
            .chunks_exact(3)
            .map(|t| {
                let c = |i: u32| {
                    let k = i as usize;
                    Corner { p: p.positions[k], n: p.normals[k], uv: p.uvs[k], c: p.colours[k] }
                };
                area2(&[c(t[0]), c(t[1]), c(t[2])]) / 2.0
            })
            .sum()
    }

    #[test]
    fn a_plate_through_a_pod_keeps_everything_but_the_pods_footprint() {
        let mut p = plate(1.0);
        assert_eq!(cut(&mut p, &[pod()]), 2, "both triangles cross the pod");
        assert!((area(&p) - (16.0 - 4.0)).abs() < 1e-4, "16 m2 less the pod's 4: {}", area(&p));
        for t in p.indices.chunks_exact(3) {
            let m: [f32; 3] = std::array::from_fn(|k| t.iter().map(|&i| p.positions[i as usize][k]).sum::<f32>() / 3.0);
            assert!(!inside(&pod(), m, -0.001), "a piece left inside the pod at {m:?}");
            let n = {
                let (a, b, c) = (p.positions[t[0] as usize], p.positions[t[1] as usize], p.positions[t[2] as usize]);
                let (u, v) = ([b[0] - a[0], b[2] - a[2]], [c[0] - a[0], c[2] - a[2]]);
                u[1] * v[0] - u[0] * v[1]
            };
            assert!(n > 0.0, "a piece keeps the plate's winding (facing up)");
        }
    }

    #[test]
    fn a_cut_corner_takes_the_paint_where_it_lies_on_the_plate() {
        let mut p = plate(1.0);
        cut(&mut p, &[pod()]);
        for (q, uv) in p.positions.iter().zip(&p.uvs) {
            let want = [(q[0] + 2.0) / 4.0, (q[2] + 2.0) / 4.0];
            assert!((uv[0] - want[0]).abs() < 1e-5 && (uv[1] - want[1]).abs() < 1e-5, "{q:?} has {uv:?}");
        }
    }

    #[test]
    fn a_plate_clear_of_the_pod_or_in_its_ceiling_is_kept_or_cut_as_it_should_be() {
        let mut above = plate(2.5);
        assert_eq!(cut(&mut above, &[pod()]), 0, "above the pod: untouched");
        assert_eq!(above.indices, plate(2.5).indices);
        let mut ceiling = plate(2.0);
        assert_eq!(cut(&mut ceiling, &[pod()]), 2, "a face lying in the pod's ceiling is cut, not left to fight it");
        assert!((area(&ceiling) - 12.0).abs() < 1e-4);
    }
}
