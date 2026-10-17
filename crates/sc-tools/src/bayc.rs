//! `bayc`: compiles the spacedock bays into `compiled/stations.bays` (openspec/changes/spacedock design 8b).
//!
//! Its input is the mockup's own output, exported from `docs/mockups/spacedock.html` by
//! `tools/stations/export_bays.mjs` into `build/stations/`: each version's baked inside and its fixtures' faces in the
//! station's frame, the inside's light probes, and the texture layers they sample. It packs them with `deckc`'s packer,
//! colour byte and float reader and `sc-core`'s one vertex packer, so the bays are stored exactly as a deck's rooms:
//!
//! - positions about each mesh's centre, divided by the smallest power of two that fits them in the vertex's +/-32;
//! - texture coordinates shifted by whole spans (they repeat) so they fit the vertex's +/-32: the mesh's mean, or a
//!   triangle's own where the mean leaves it outside, a triangle spanning more than 30 halved until it fits;
//! - identical vertices merged, in first-seen order (stable), behind 32-bit indices;
//! - each probe its 54 colour bytes then its valid flag (`sc-core::probes`);
//! - the texture layers with a full mip chain (box filter).

use sc_core::bay::{self, Bay, BayMesh, BayTextures, BaysIndex};
use sc_core::deck::DeckProbes;
use sc_core::mips::mip_chain;
use sc_core::vertex::DeckVertexIn;
use serde::Deserialize;
use std::path::Path;

use crate::deckc::{color_byte, floats, pack};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Export {
    #[allow(dead_code)]
    schema: String,
    textures: Textures,
    bays: Vec<ExportBay>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Textures {
    size_px: u32,
    layers: u32,
    rgba: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ExportBay {
    version: String,
    digest: String,
    inside: ExportMesh,
    glow: ExportMesh,
    grids: Vec<ExportGrid>,
    /// A-D: the outside with its doors shut; none for E (the engine builds its own).
    #[serde(default)]
    outside: Option<ExportMesh>,
    /// The docked ships: fleet key and pose.
    #[serde(default)]
    docked: Vec<sc_core::bay::BayDocked>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ExportMesh {
    vertices: u64,
    /// Indices (0: the mesh is a triangle soup, three vertices a triangle).
    indices: u64,
    files: MeshFiles,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MeshFiles {
    position: String,
    normal: String,
    uv: String,
    layer: String,
    #[serde(default)]
    index: Option<String>,
    c0: String,
    c1: String,
    c2: String,
    /// A float a vertex (the outside's): 1 where the part is its own light (alpha 0), 0.5 where its face looks into the
    /// bay's air (alpha 128), 0 for the outside proper.
    #[serde(default)]
    glow: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ExportGrid {
    origin_m: [f64; 3],
    spacing_m: f32,
    dims: [u32; 3],
    cubes: String,
    valid: String,
}

/// The smallest power of two that brings `half_extent_m` inside the vertex's +/-32 m with a metre to spare.
fn scale_for(half_extent_m: f64) -> f32 {
    let mut s = 1.0f32;
    while half_extent_m / f64::from(s) > 31.0 {
        s *= 2.0;
    }
    s
}

/// A triangle's corner before packing: the station's frame (metres), its normal, texture coordinates (layer spans), layer
/// and the three states' colours (linear multipliers).
#[derive(Clone, Copy)]
struct Corner {
    p: [f32; 3],
    n: [f32; 3],
    uv: [f32; 2],
    layer: u8,
    c: [[f32; 3]; 3],
    /// The colour's alpha byte: 255, 0 where the part is its own light, 128 where it faces into the bay (the outside's).
    a: u8,
}

/// The most texture repeats a triangle may span on either axis: the vertex holds +/-32 after a whole-span shift.
const MAX_SPANS: f32 = 30.0;

/// Push `t` onto `out`, halved across its longest edge (every attribute taken at the edge's middle, so the picture is
/// the same) until it spans at most `MAX_SPANS` repeats: a long lit panel, a window tier 400 m across tiled every 7 m.
fn split(t: [Corner; 3], out: &mut Vec<[Corner; 3]>) {
    let span = |a: usize| {
        let (lo, hi) = t.iter().fold((f32::MAX, f32::MIN), |(lo, hi), q| (lo.min(q.uv[a]), hi.max(q.uv[a])));
        hi - lo
    };
    if span(0) <= MAX_SPANS && span(1) <= MAX_SPANS {
        out.push(t);
        return;
    }
    let len = |i: usize| {
        let (a, b) = (t[i].uv, t[(i + 1) % 3].uv);
        (a[0] - b[0]).abs().max((a[1] - b[1]).abs())
    };
    let e = (0..3).max_by(|&i, &j| len(i).total_cmp(&len(j))).unwrap_or(0);
    let (a, b, c) = (t[e], t[(e + 1) % 3], t[(e + 2) % 3]);
    let mid = |x: f32, y: f32| (x + y) / 2.0;
    let m = Corner {
        p: std::array::from_fn(|k| mid(a.p[k], b.p[k])),
        n: a.n,
        uv: std::array::from_fn(|k| mid(a.uv[k], b.uv[k])),
        layer: a.layer,
        c: std::array::from_fn(|s| std::array::from_fn(|k| mid(a.c[s][k], b.c[s][k]))),
        a: a.a.min(b.a),
    };
    // The same winding: (a, m, c) and (m, b, c).
    split([a, m, c], out);
    split([m, b, c], out);
}

/// Pack one exported mesh into the blobs; `layers` is the texture array's layer count (a layer past it is an error).
fn mesh(
    dir: &Path,
    what: &str,
    m: &ExportMesh,
    layers: u32,
    vertices: &mut Vec<u8>,
    indices: &mut Vec<u32>,
) -> Result<(BayMesh, usize), String> {
    let n = m.vertices as usize;
    let f = &m.files;
    let (pos, nor, uv, lay) = (
        floats(dir, &f.position, n * 3)?,
        floats(dir, &f.normal, n * 3)?,
        floats(dir, &f.uv, n * 2)?,
        floats(dir, &f.layer, n)?,
    );
    let cs = [floats(dir, &f.c0, n * 3)?, floats(dir, &f.c1, n * 3)?, floats(dir, &f.c2, n * 3)?];
    let glows = match &f.glow {
        Some(g) => floats(dir, g, n)?,
        None => vec![0.0; n],
    };
    // The triangles' corners: from the index file, or the soup's own order.
    let corners: Vec<usize> = match (&f.index, m.indices) {
        (None, 0) => (0..n).collect(),
        (Some(name), k) if k > 0 => {
            let b = std::fs::read(dir.join(name)).map_err(|e| format!("{name}: {e}"))?;
            if b.len() != k as usize * 4 {
                return Err(format!("{name}: {} bytes, expected {}", b.len(), k * 4));
            }
            let ix: Vec<usize> =
                b.chunks_exact(4).map(|q| u32::from_le_bytes(q.try_into().unwrap()) as usize).collect();
            if let Some(bad) = ix.iter().find(|&&i| i >= n) {
                return Err(format!("{what}: index {bad} past its {n} vertices"));
            }
            ix
        }
        _ => return Err(format!("{what}: an index count without an index file, or the other way round")),
    };
    if corners.len() % 3 != 0 || corners.is_empty() {
        return Err(format!("{what}: {} corners is not whole triangles", corners.len()));
    }
    // Its frame: the centre of its bounds, to the centimetre, and the scale that fits it.
    let mut lo = [f64::MAX; 3];
    let mut hi = [f64::MIN; 3];
    for p in pos.chunks_exact(3) {
        for k in 0..3 {
            lo[k] = lo[k].min(f64::from(p[k]));
            hi[k] = hi[k].max(f64::from(p[k]));
        }
    }
    let origin: [f64; 3] = std::array::from_fn(|k| ((lo[k] + hi[k]) / 2.0 * 100.0).round() / 100.0);
    let half = (0..3).map(|k| (hi[k] - origin[k]).max(origin[k] - lo[k])).fold(0.0, f64::max);
    let scale = scale_for(half);
    let corner = |i: usize| -> Result<Corner, String> {
        let l = lay[i].round();
        if !(0.0..layers as f32).contains(&l) {
            return Err(format!("{what}: vertex {i} samples layer {l}, past the array's {layers}"));
        }
        Ok(Corner {
            p: [pos[i * 3], pos[i * 3 + 1], pos[i * 3 + 2]],
            n: [nor[i * 3], nor[i * 3 + 1], nor[i * 3 + 2]],
            uv: [uv[i * 2], uv[i * 2 + 1]],
            layer: l as u8,
            c: std::array::from_fn(|s| [cs[s][i * 3], cs[s][i * 3 + 1], cs[s][i * 3 + 2]]),
            // The outside's flag: its own light (alpha 0), or a face looking into the bay's air (alpha 128).
            a: if glows[i] > 0.75 {
                0
            } else if glows[i] > 0.25 {
                128
            } else {
                255
            },
        })
    };
    let mut tris = Vec::with_capacity(corners.len() / 3);
    for t in corners.chunks_exact(3) {
        split([corner(t[0])?, corner(t[1])?, corner(t[2])?], &mut tris);
    }
    // Whole spans off the texture coordinates (they repeat): the mesh's mean, so neighbours keep sharing a vertex, and a
    // triangle's own where the mean leaves it outside +/-31.
    let n_c = (tris.len() * 3).max(1) as f64;
    let mean: [f32; 2] =
        std::array::from_fn(|a| (tris.iter().flatten().map(|q| f64::from(q.uv[a])).sum::<f64>() / n_c).floor() as f32);
    let mut ins = Vec::with_capacity(tris.len() * 3);
    for t in &tris {
        let fits = t.iter().all(|q| (0..2).all(|a| (q.uv[a] - mean[a]).abs() <= 31.0));
        let own = |a: usize| t.iter().map(|q| q.uv[a]).fold(f32::MAX, f32::min).floor();
        let (du, dv) = if fits { (mean[0], mean[1]) } else { (own(0), own(1)) };
        for q in t {
            ins.push(DeckVertexIn {
                position_m: std::array::from_fn(|k| ((f64::from(q.p[k]) - origin[k]) / f64::from(scale)) as f32),
                mover: 0,
                layer: q.layer,
                normal: q.n,
                colors: std::array::from_fn(|s| {
                    [color_byte(q.c[s][0]), color_byte(q.c[s][1]), color_byte(q.c[s][2]), q.a]
                }),
                uv: [q.uv[0] - du, q.uv[1] - dv],
            });
        }
    }
    let (vertex_offset, vertex_count, index_offset, index_count) = pack(vertices, indices, &ins, what)?;
    Ok((BayMesh { origin_m: origin, scale, vertex_offset, vertex_count, index_offset, index_count }, tris.len()))
}

/// Compile `build/stations/export.json` under `root` into `compiled/stations.bays`; returns a summary.
pub fn run(root: &Path) -> Result<String, String> {
    let dir = root.join("build/stations");
    let text = std::fs::read_to_string(dir.join("export.json"))
        .map_err(|e| format!("build/stations/export.json: {e} (run node tools/stations/export_bays.mjs first)"))?;
    let ex: Export = serde_json::from_str(&text).map_err(|e| format!("export.json: {e}"))?;
    let t = &ex.textures;
    if !t.size_px.is_power_of_two() || t.layers == 0 || t.layers > 256 {
        return Err(format!("the texture array ({} px, {} layers) is not usable", t.size_px, t.layers));
    }
    let level0 = std::fs::read(dir.join(&t.rgba)).map_err(|e| format!("{}: {e}", t.rgba))?;
    if level0.len() != (t.size_px * t.size_px * 4 * t.layers) as usize {
        return Err("the texture array is not the size its index says".into());
    }
    let mut mip_offsets = Vec::new();
    let mut tex_blob = Vec::new();
    for level in mip_chain(level0, t.size_px, t.layers) {
        mip_offsets.push(tex_blob.len() as u64);
        tex_blob.extend_from_slice(&level);
    }
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let mut probe_blob = Vec::new();
    let mut bays = Vec::new();
    let mut said = Vec::new();
    for b in &ex.bays {
        if !matches!(b.version.as_str(), "A" | "B" | "C" | "D" | "E")
            || bays.iter().any(|x: &Bay| x.version == b.version)
        {
            return Err(format!("bay {}: not a version A-E, or listed twice", b.version));
        }
        let (inside, tris) =
            mesh(&dir, &format!("{} inside", b.version), &b.inside, t.layers, &mut vertices, &mut indices)?;
        let (glow, glow_tris) =
            mesh(&dir, &format!("{} glow", b.version), &b.glow, t.layers, &mut vertices, &mut indices)?;
        let outside = match &b.outside {
            Some(m) => Some(mesh(&dir, &format!("{} outside", b.version), m, t.layers, &mut vertices, &mut indices)?.0),
            None => None,
        };
        let mut grids = Vec::new();
        let mut valid_count = 0;
        for (k, g) in b.grids.iter().enumerate() {
            let count = g.dims.iter().map(|&d| d as usize).product::<usize>();
            let cubes = std::fs::read(dir.join(&g.cubes)).map_err(|e| format!("{}: {e}", g.cubes))?;
            let valid = std::fs::read(dir.join(&g.valid)).map_err(|e| format!("{}: {e}", g.valid))?;
            if count == 0 || cubes.len() != count * 54 || valid.len() != count || valid.iter().any(|&v| v > 1) {
                return Err(format!("bay {}: grid {k}'s probes do not match its dims", b.version));
            }
            if !(g.spacing_m > 0.0 && g.spacing_m.is_finite()) || g.origin_m.iter().any(|x| !x.is_finite()) {
                return Err(format!("bay {}: grid {k} is not finite", b.version));
            }
            let first = (probe_blob.len() / sc_core::probes::PROBE_BYTES) as u64;
            for (cube, v) in cubes.chunks_exact(54).zip(&valid) {
                probe_blob.extend_from_slice(cube);
                probe_blob.push(*v);
            }
            valid_count += valid.iter().filter(|&&v| v == 1).count();
            grids.push(DeckProbes { origin_m: g.origin_m, spacing_m: g.spacing_m, dims: g.dims, first });
        }
        said.push(format!(
            "{}: {tris} triangles inside ({} vertices, at 1/{} m steps x {}), {glow_tris} of glow, {} probes ({valid_count} valid)",
            b.version,
            inside.vertex_count,
            sc_core::vertex::FIXED_SCALE,
            inside.scale,
            grids.iter().map(|g| g.dims.iter().product::<u32>()).sum::<u32>()
        ));
        bays.push(Bay {
            version: b.version.clone(),
            digest: b.digest.clone(),
            inside,
            glow,
            grids,
            outside,
            docked: b.docked.clone(),
        });
    }
    let index = BaysIndex {
        bays,
        textures: BayTextures { size_px: t.size_px, layers: t.layers, mip_offsets },
        vertex_bytes: vertices.len() as u64,
        index_count: indices.len() as u64,
        texture_bytes: tex_blob.len() as u64,
        probe_count: (probe_blob.len() / sc_core::probes::PROBE_BYTES) as u64,
    };
    let bytes = bay::write(&index, &vertices, &indices, &tex_blob, &probe_blob);
    // Read it back through the reader the client uses, so a file that will not load is never written.
    bay::read(&bytes).map_err(|e| format!("the packed file does not read back: {e}"))?;
    std::fs::create_dir_all(root.join("compiled")).map_err(|e| e.to_string())?;
    std::fs::write(root.join("compiled/stations.bays"), &bytes).map_err(|e| e.to_string())?;
    Ok(format!(
        "compiled/stations.bays, {:.1} MB, {} texture layers of {} px: {}",
        bytes.len() as f64 / 1e6,
        t.layers,
        t.size_px,
        said.join("; ")
    ))
}

#[cfg(test)]
mod tests {
    use super::{scale_for, split, Corner, MAX_SPANS};

    #[test]
    fn a_mesh_takes_the_smallest_power_of_two_that_fits_it() {
        assert_eq!(scale_for(20.0), 1.0);
        assert_eq!(scale_for(31.0), 1.0);
        assert_eq!(scale_for(32.0), 2.0);
        assert_eq!(scale_for(310.0), 16.0, "E's bay, 620 m long, at 1.6 cm steps");
    }

    #[test]
    fn a_panel_wider_than_the_vertex_holds_is_halved_until_it_fits_and_keeps_its_picture() {
        // A panel 400 m along x and 10 m up y, its texture 57 repeats along x and one up y: u = 57 x / 400 + y / 10.
        let uv = |x: f32, y: f32| x * 57.0 / 400.0 + y / 10.0;
        let q = |x: f32, y: f32| Corner {
            p: [x, y, 0.0],
            n: [0.0, 0.0, 1.0],
            uv: [uv(x, y), 0.0],
            layer: 2,
            c: [[x / 100.0; 3]; 3],
            a: 255,
        };
        let whole = [q(0.0, 0.0), q(400.0, 0.0), q(0.0, 10.0)];
        let mut out = Vec::new();
        split(whole, &mut out);
        assert!(out.len() > 1, "57 repeats do not fit in one triangle");
        for t in &out {
            let (lo, hi) = t.iter().fold((f32::MAX, f32::MIN), |(lo, hi), c| (lo.min(c.uv[0]), hi.max(c.uv[0])));
            assert!(hi - lo <= MAX_SPANS, "each piece spans at most {MAX_SPANS} repeats");
            for c in t {
                assert!((c.uv[0] - uv(c.p[0], c.p[1])).abs() < 1e-3, "the texture lies where it lay: {:?}", c.p);
                assert!((c.c[0][0] - c.p[0] / 100.0).abs() < 1e-4, "the colour is the one at that point");
                assert_eq!(c.layer, 2);
            }
        }
        let area = |t: &[Corner; 3]| {
            let (a, b, c) = (t[0].p, t[1].p, t[2].p);
            ((b[0] - a[0]) * (c[1] - a[1]) - (c[0] - a[0]) * (b[1] - a[1])) / 2.0
        };
        assert!(out.iter().all(|t| area(t) > 0.0), "every piece is wound as the panel was");
        assert!((out.iter().map(area).sum::<f32>() - area(&whole)).abs() < 1e-2, "the pieces cover the panel once");
    }
}
