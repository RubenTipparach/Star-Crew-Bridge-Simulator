//! `sc-tools console-model [ship]`: writes `data/ships/<ship>/console_model.json` (`sc_core::console_model`), the ship
//! as the consoles draw it in 3D, from the default livery's distant detail level (`<ship>_lod2`, read and checked by
//! `deckc::exterior_model`, the same reader and the same livery as the deck's hull).
//!
//! It keeps the model's geometry and nothing else: positions welded to the millimetre (the model's corners are split
//! where its texture coordinates are), its triangles as the model winds them, and each edge where the surface turns by
//! more than [`CREASE_DEG`] marked as a crease, the line a console outlines. No normals, texture coordinates or
//! textures. It lives in the tools because only the tools read a glb (CLAUDE.md 9: meshes are files built by committed
//! generators); the same model writes the same bytes, and a test fails when the committed file is stale.

use sc_core::console_model::{ConsoleModel, MAX_TRIANGLES, SCHEMA};
use std::collections::HashMap;
use std::path::Path;

/// The level of detail the consoles draw: the distant one, a few hundred triangles.
const LOD: &str = "lod2";

/// Two triangles meeting at more than this, degrees, meet at a crease: the triangles of one flat panel are one face.
const CREASE_DEG: f64 = 1.0;

/// Where a ship's console model is written, from the repository's root.
pub(crate) fn path(ship: &str) -> String {
    format!("data/ships/{ship}/console_model.json")
}

/// The console model's file for `ship`, as text, checked through the one loader.
pub(crate) fn build(root: &Path, ship: &str) -> Result<String, String> {
    let prop = format!("{ship}_{LOD}");
    let (source, model) = crate::deckc::exterior_model(root, &prop)?
        .ok_or_else(|| format!("assets/models/exteriors/props.json: no model {prop}"))?;
    let (corners, indices): (Vec<[f32; 3]>, Vec<u32>) = model.primitives.iter().fold((vec![], vec![]), |mut a, p| {
        let base = a.0.len() as u32;
        a.0.extend_from_slice(&p.positions);
        a.1.extend(p.indices.iter().map(|i| i + base));
        a
    });
    let (vertices, triangles) = weld(&corners, &indices);
    if triangles.len() > MAX_TRIANGLES {
        return Err(format!("{source}: {} triangles, over the console model's {MAX_TRIANGLES}", triangles.len()));
    }
    let creases = creases(&vertices, &triangles, CREASE_DEG);
    let text = to_json(&source, &vertices, &triangles, &creases);
    let rel = path(ship);
    sc_core::data::parse::<ConsoleModel>(&rel, &text).map_err(|e| e.to_string())?;
    Ok(text)
}

/// Write `ship`'s console model; what was written, in a line.
pub(crate) fn run(root: &Path, ship: &str) -> Result<String, String> {
    let text = build(root, ship)?;
    let rel = path(ship);
    std::fs::write(root.join(&rel), &text).map_err(|e| format!("{rel}: {e}"))?;
    let m: ConsoleModel = sc_core::data::parse(&rel, &text).map_err(|e| e.to_string())?;
    let creased = m.creases.iter().map(|c| c.count_ones()).sum::<u32>();
    Ok(format!(
        "{rel}: {} vertices, {} triangles, {creased} crease edges (each counted from both sides) from {}",
        m.vertices_m.len(),
        m.triangles.len(),
        m.source
    ))
}

/// Corners welded where they round to the same millimetre, in first-seen order (stable), and the triangles over the
/// welded corners; a triangle that welds to fewer than three corners is dropped.
fn weld(corners: &[[f32; 3]], indices: &[u32]) -> (Vec<[f64; 3]>, Vec<[u32; 3]>) {
    let mm = |x: f32| (f64::from(x) * 1000.0).round() as i64;
    let mut at: HashMap<[i64; 3], u32> = HashMap::new();
    let mut vertices: Vec<[f64; 3]> = Vec::new();
    let remap: Vec<u32> = corners
        .iter()
        .map(|c| {
            let key = c.map(mm);
            *at.entry(key).or_insert_with(|| {
                vertices.push(key.map(|k| k as f64 / 1000.0));
                vertices.len() as u32 - 1
            })
        })
        .collect();
    let triangles = indices
        .chunks_exact(3)
        .map(|t| [remap[t[0] as usize], remap[t[1] as usize], remap[t[2] as usize]])
        .filter(|t| t[0] != t[1] && t[1] != t[2] && t[2] != t[0])
        .collect();
    (vertices, triangles)
}

/// A triangle's unit normal (counter-clockwise seen from its front).
fn normal(v: &[[f64; 3]], t: [u32; 3]) -> [f64; 3] {
    let [a, b, c] = t.map(|i| v[i as usize]);
    let (u, w) = ([b[0] - a[0], b[1] - a[1], b[2] - a[2]], [c[0] - a[0], c[1] - a[1], c[2] - a[2]]);
    let n = [u[1] * w[2] - u[2] * w[1], u[2] * w[0] - u[0] * w[2], u[0] * w[1] - u[1] * w[0]];
    let l = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt().max(1e-12);
    n.map(|x| x / l)
}

/// Each triangle's crease mask (`ConsoleModel::creases`): an edge is a crease unless exactly one other triangle shares
/// it, wound the other way, facing within `deg` degrees of this one.
fn creases(v: &[[f64; 3]], tris: &[[u32; 3]], deg: f64) -> Vec<u8> {
    let normals: Vec<[f64; 3]> = tris.iter().map(|t| normal(v, *t)).collect();
    let mut edges: HashMap<(u32, u32), Vec<usize>> = HashMap::new();
    for (i, t) in tris.iter().enumerate() {
        for k in 0..3 {
            edges.entry((t[k], t[(k + 1) % 3])).or_default().push(i);
        }
    }
    let flat = deg.to_radians().cos();
    tris.iter()
        .enumerate()
        .map(|(i, t)| {
            (0..3).fold(0u8, |mask, k| {
                let (a, b) = (t[k], t[(k + 1) % 3]);
                let same_way = edges.get(&(a, b)).map_or(0, Vec::len);
                let back = edges.get(&(b, a)).map(Vec::as_slice).unwrap_or(&[]);
                let smooth = same_way == 1 && back.len() == 1 && {
                    let (n, m) = (normals[i], normals[back[0]]);
                    n[0] * m[0] + n[1] * m[1] + n[2] * m[2] >= flat
                };
                if smooth {
                    mask
                } else {
                    mask | 1 << k
                }
            })
        })
        .collect()
}

/// A coordinate in metres to the millimetre, without trailing zeros (and never "-0").
fn metres(x: f64) -> String {
    let s = format!("{x:.3}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" {
        "0".into()
    } else {
        s.into()
    }
}

/// The file's text: one corner and one triangle a line, so a change to the model is a readable diff.
fn to_json(source: &str, v: &[[f64; 3]], tris: &[[u32; 3]], creases: &[u8]) -> String {
    let doc = "The ship as the consoles draw it in 3D (the camera widget, Tactical's plot and Science's shields, \
weapons-and-shields 11): the geometry of the exterior model's distant detail level, without its texture. Written by \
`sc-tools console-model` (crates/sc-tools/src/console_model.rs) from source; never edit it by hand, a test fails when it \
is stale. Corners welded to the millimetre, metres, ship axes (+X port, +Y up, +Z bow) from the layout's origin; \
triangles counter-clockwise seen from outside; creases: per triangle, bit k set when its edge from corner k to corner \
k + 1 is outlined (the surface turns there by more than crease_deg degrees). The engine reads it in \
crates/sc-client/src/console/mod.rs (tern), the console mockup inlines it (data:tern/console_model).";
    let mut s = String::new();
    s.push_str(&format!("{{\n  \"schema\": \"{SCHEMA}\",\n  \"_doc\": \"{doc}\",\n"));
    s.push_str(&format!("  \"source\": \"{source}\",\n  \"crease_deg\": {CREASE_DEG},\n  \"vertices_m\": [\n"));
    let rows: Vec<String> =
        v.iter().map(|p| format!("    [{}, {}, {}]", metres(p[0]), metres(p[1]), metres(p[2]))).collect();
    s.push_str(&rows.join(",\n"));
    s.push_str("\n  ],\n  \"triangles\": [\n");
    let rows: Vec<String> = tris.iter().map(|t| format!("    [{}, {}, {}]", t[0], t[1], t[2])).collect();
    s.push_str(&rows.join(",\n"));
    s.push_str("\n  ],\n  \"creases\": [\n");
    let rows: Vec<String> = creases
        .chunks(32)
        .map(|c| format!("    {}", c.iter().map(u8::to_string).collect::<Vec<_>>().join(", ")))
        .collect();
    s.push_str(&rows.join(",\n"));
    s.push_str("\n  ]\n}\n");
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root() -> &'static Path {
        Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
    }

    fn committed() -> ConsoleModel {
        let rel = path("tern");
        let text = std::fs::read_to_string(root().join(&rel)).unwrap();
        sc_core::data::parse(&rel, &text).unwrap()
    }

    #[test]
    fn the_committed_console_model_is_the_one_the_exterior_model_writes() {
        let rel = path("tern");
        let want = build(root(), "tern").unwrap();
        let have = std::fs::read_to_string(root().join(&rel)).unwrap();
        assert!(have == want, "{rel} is stale: run `cargo run --release -p sc-tools -- console-model tern`");
    }

    #[test]
    fn the_console_model_is_the_low_poly_tern_closed_and_wound_outward() {
        let m = committed();
        assert!(m.source.ends_with("tern_lod2.glb"), "the distant detail level, {}", m.source);
        assert_eq!(m.triangles.len(), 496, "every triangle of tern_lod2.glb");
        // Closed: every edge is walked once each way, so no gap shows the background through the ship.
        let mut walked: HashMap<(u32, u32), u32> = HashMap::new();
        for t in &m.triangles {
            for k in 0..3 {
                *walked.entry((t[k], t[(k + 1) % 3])).or_default() += 1;
            }
        }
        for (&(a, b), &n) in &walked {
            assert!(n == 1 && walked.get(&(b, a)) == Some(&1), "edge {a}-{b} is not shared by exactly two faces");
        }
        // Outward: a closed surface wound counter-clockwise from outside has a positive volume, so the consoles' back
        // faces are the ones turned away.
        let v = |i: u32| m.vertices_m[i as usize];
        let vol: f64 = m
            .triangles
            .iter()
            .map(|t| {
                let (a, b, c) = (v(t[0]), v(t[1]), v(t[2]));
                a[0] * (b[1] * c[2] - b[2] * c[1]) - a[1] * (b[0] * c[2] - b[2] * c[0])
                    + a[2] * (b[0] * c[1] - b[1] * c[0])
            })
            .sum::<f64>()
            / 6.0;
        assert!(vol > 10_000.0, "a frigate's volume, wound outward: {vol} m3");
    }

    /// The consoles drew the layout's hull loft, sitting in the shield ellipsoid; the model must stand in the same
    /// frame, so its hull's widest beam, roof, keel and stern are the loft's.
    #[test]
    fn the_console_model_stands_in_the_layouts_frame() {
        let m = committed();
        let layout: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(root().join("data/ships/tern/layout.json")).unwrap())
                .unwrap();
        let secs = layout["hull"]["sections"].as_array().unwrap();
        let of = |k: &str| secs.iter().map(|s| s[k].as_f64().unwrap()).collect::<Vec<f64>>();
        let max = |v: Vec<f64>| v.into_iter().fold(f64::MIN, f64::max);
        let min = |v: Vec<f64>| v.into_iter().fold(f64::MAX, f64::min);
        let (beam, top, bottom, stern) =
            (max(of("half_beam_m")), max(of("top_m")), min(of("bottom_m")), min(of("z_m")));
        // The hull alone: the pylons and nacelles stand out past its beam.
        let hull: Vec<[f64; 3]> = m.vertices_m.iter().copied().filter(|p| p[0].abs() <= beam + 0.01).collect();
        let near = |a: f64, b: f64, what: &str| assert!((a - b).abs() < 0.011, "{what}: model {a} m, loft {b} m");
        near(max(hull.iter().map(|p| p[0].abs()).collect()), beam, "half beam");
        near(max(hull.iter().map(|p| p[1]).collect()), top, "roof");
        near(min(hull.iter().map(|p| p[1]).collect()), bottom, "keel");
        near(min(hull.iter().map(|p| p[2]).collect()), stern, "stern");
        assert!(max(hull.iter().map(|p| p[2]).collect()) <= max(of("z_m")), "the bow within the loft's");
    }

    #[test]
    fn welding_merges_corners_a_millimetre_apart_and_drops_what_collapses() {
        let corners = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [1.0002, 0.0, 0.0], [0.0004, 0.0, 0.0]];
        let (v, t) = weld(&corners, &[0, 1, 2, 3, 2, 0, 0, 4, 2]);
        assert_eq!(v.len(), 3, "the split corners weld");
        assert_eq!(t, vec![[0, 1, 2], [1, 2, 0]], "a triangle welded to two corners is dropped");
    }

    #[test]
    fn a_flat_panels_diagonal_is_no_crease_and_a_fold_is_one() {
        // A unit square in two triangles, folded along x = 1 into a second square standing up.
        let v = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 0.0, 1.0], [0.0, 0.0, 1.0], [1.0, 1.0, 0.0], [1.0, 1.0, 1.0]];
        let tris = [[0, 2, 1], [0, 3, 2], [1, 2, 5], [1, 5, 4]];
        let c = creases(&v, &tris, CREASE_DEG);
        // [0, 2, 1]: 0-2 the diagonal (smooth), 2-1 the fold, 1-0 an open edge.
        assert_eq!(c[0], 0b110);
        // [0, 3, 2]: 0-3 and 3-2 open edges, 2-0 the diagonal.
        assert_eq!(c[1], 0b011);
        // [1, 2, 5]: 1-2 the fold, 2-5 open, 5-1 the standing square's diagonal.
        assert_eq!(c[2], 0b011);
    }
}
