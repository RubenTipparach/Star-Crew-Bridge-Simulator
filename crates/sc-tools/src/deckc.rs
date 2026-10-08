//! `deckc`: compiles a ship's decks into `compiled/<ship>.deck` (openspec/changes/deck-pipeline, design
//! section 13: the first deck in the engine).
//!
//! Its input today is the kit's own output, exported from the deck plan by `tools/deck/export_deck.mjs`
//! into `build/deck/<ship>/`: every compartment's triangles in ship coordinates, normals, texture
//! coordinates in layer spans, layers, and the three state colours as linear multipliers (the baked
//! light). This step packs them with `sc-core`'s one vertex packer, so the 28 bytes are written exactly
//! as the renderer reads them:
//!
//! - positions relative to the compartment's centre (the frames rule: subtract the origin, then narrow);
//! - texture coordinates shifted by whole spans per compartment, which repeat, into the format's +/-32;
//! - a colour byte is the display multiplier `c^(1/2.2) / 2` (light-baking design 5, 0-2x);
//! - the bigger texture arrays' layers resampled into the one array after the main layers;
//! - identical vertices merged, in first-seen order (stable), behind 32-bit indices;
//! - a full mip chain for the texture array (box filter), sampled nearest up close;
//! - since 13b: a room's mover (the lift's car is a room of its own, mover 1), each room's console faces appended to
//!   its mesh in the screens' atlas, which is cut into square layers after every other layer, and the viewscreens.

use sc_core::deck::{
    self, DeckCompartment, DeckIndex, DeckTextures, DeckView, DeckWalk, WalkDoor, WalkHatch, WalkLadder, WalkLift,
    WalkStart,
};
use sc_core::vertex::{pack_deck_vertex, DeckVertexIn};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Export {
    #[allow(dead_code)]
    schema: String,
    ship: String,
    panel_first: u32,
    panel_glow: Glow,
    textures: ExportTextures,
    rooms: Vec<Room>,
    walk: ExportWalk,
    views: Vec<DeckView>,
}
/// The walk world as the deck plan exported it (deck-pipeline 13a): the triangles' file and the entities.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ExportWalk {
    triangles: String,
    triangle_count: u64,
    start: WalkStart,
    ladders: Vec<WalkLadder>,
    hatches: Vec<WalkHatch>,
    doors: Vec<WalkDoor>,
    lifts: Vec<WalkLift>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Glow {
    normal: f32,
    red_alert: f32,
    emergency: f32,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ExportTextures {
    main: TexArray,
    big: Vec<BigArray>,
    screens: Option<Screens>,
}
/// The console screens' atlas (bridge-stations 11.6): one image `width_px` wide, rows of screens down it; a face's
/// v runs down the image from its top.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Screens {
    width_px: u32,
    height_px: u32,
    rgba: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TexArray {
    size_px: u32,
    layers: u32,
    rgba: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BigArray {
    base: u32,
    size_px: u32,
    layers: u32,
    rgba: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Room {
    id: String,
    name: String,
    deck: String,
    mover: u8,
    vertices: u32,
    files: RoomFiles,
    faces: Option<Faces>,
}
/// A room's console faces: triangles textured from the screens' atlas, lit by the room's bake.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Faces {
    vertices: u32,
    files: FaceFiles,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FaceFiles {
    position: String,
    normal: String,
    uv: String,
    c0: String,
    c1: String,
    c2: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RoomFiles {
    position: String,
    normal: String,
    uv: String,
    layer: String,
    c0: String,
    c1: String,
    c2: String,
}

fn floats(dir: &Path, name: &str, want: usize) -> Result<Vec<f32>, String> {
    let b = std::fs::read(dir.join(name)).map_err(|e| format!("{name}: {e}"))?;
    if b.len() != want * 4 {
        return Err(format!("{name}: {} bytes, expected {}", b.len(), want * 4));
    }
    let v: Vec<f32> = b.chunks_exact(4).map(|c| f32::from_le_bytes(c.try_into().unwrap())).collect();
    if let Some(i) = v.iter().position(|x| !x.is_finite()) {
        return Err(format!("{name}: value {i} is not finite"));
    }
    Ok(v)
}

/// A colour multiplier as the byte the deck stores (a display multiplier, 0-2x).
fn color_byte(c: f32) -> u8 {
    ((c.max(0.0).powf(1.0 / 2.2) / 2.0).clamp(0.0, 1.0) * 255.0).round() as u8
}

/// Box-filter `src` (square RGBA8 layers of `from` px) to `to` px, `to` dividing `from`.
fn shrink(src: &[u8], from: u32, to: u32) -> Vec<u8> {
    let k = (from / to) as usize;
    let (f, t) = (from as usize, to as usize);
    let mut out = vec![0u8; t * t * 4];
    for y in 0..t {
        for x in 0..t {
            let mut acc = [0u32; 4];
            for dy in 0..k {
                for dx in 0..k {
                    let i = ((y * k + dy) * f + x * k + dx) * 4;
                    for (c, a) in acc.iter_mut().enumerate() {
                        *a += u32::from(src[i + c]);
                    }
                }
            }
            for c in 0..4 {
                out[(y * t + x) * 4 + c] = (acc[c] / (k * k) as u32) as u8;
            }
        }
    }
    out
}

/// Pack a compartment's vertices: identical ones merged in first-seen order (stable), its indices local to it. Returns
/// its vertex offset (bytes), vertex count, index offset and index count.
fn pack(
    vertices: &mut Vec<u8>,
    indices: &mut Vec<u32>,
    ins: &[DeckVertexIn],
    id: &str,
) -> Result<(u64, u32, u64, u32), String> {
    let mut seen: HashMap<[u8; 28], u32> = HashMap::new();
    let first_vertex = vertices.len();
    let first_index = indices.len();
    let mut local_count = 0u32;
    for (i, v) in ins.iter().enumerate() {
        let b = pack_deck_vertex(v).map_err(|e| format!("{id}: vertex {i}: {}", e.0))?;
        let k = *seen.entry(b).or_insert_with(|| {
            vertices.extend_from_slice(&b);
            local_count += 1;
            local_count - 1
        });
        indices.push(k);
    }
    Ok((first_vertex as u64, local_count, first_index as u64, (indices.len() - first_index) as u32))
}

/// Compile `build/deck/<ship>/export.json` under `root` into `compiled/<ship>.deck`; returns a summary.
pub fn run(root: &Path, ship: &str) -> Result<String, String> {
    let dir = root.join("build/deck").join(ship);
    let text = std::fs::read_to_string(dir.join("export.json"))
        .map_err(|e| format!("build/deck/{ship}/export.json: {e} (run node tools/deck/export_deck.mjs first)"))?;
    let ex: Export = serde_json::from_str(&text).map_err(|e| format!("export.json: {e}"))?;

    // The texture array: the main layers, then each bigger array's layers resampled to the main size.
    let size = ex.textures.main.size_px;
    let layer_bytes = (size * size * 4) as usize;
    let mut layers: Vec<u8> = std::fs::read(dir.join(&ex.textures.main.rgba)).map_err(|e| e.to_string())?;
    if layers.len() != layer_bytes * ex.textures.main.layers as usize {
        return Err("the main texture array is not the size its index says".into());
    }
    let mut remap: Vec<(u32, u32, u32)> = Vec::new(); // (base, layers, first index in the one array)
    let mut next = ex.textures.main.layers;
    for b in &ex.textures.big {
        let data = std::fs::read(dir.join(&b.rgba)).map_err(|e| e.to_string())?;
        let bl = (b.size_px * b.size_px * 4) as usize;
        if data.len() != bl * b.layers as usize || b.size_px % size != 0 {
            return Err(format!("texture array at base {} is inconsistent", b.base));
        }
        for i in 0..b.layers as usize {
            layers.extend_from_slice(&shrink(&data[i * bl..(i + 1) * bl], b.size_px, size));
        }
        remap.push((b.base, b.layers, next));
        next += b.layers;
    }
    // The screens' atlas: cut into square layers of its width, down the image, each shrunk to the array's size.
    let screens_first = next;
    let mut screens_rows = 0u32;
    let mut screens_w = size;
    if let Some(sc) = &ex.textures.screens {
        let data = std::fs::read(dir.join(&sc.rgba)).map_err(|e| e.to_string())?;
        if data.len() != (sc.width_px * sc.height_px * 4) as usize || sc.width_px % size != 0 {
            return Err(format!("the screens' atlas ({} x {} px) is inconsistent", sc.width_px, sc.height_px));
        }
        let w = sc.width_px as usize;
        let n = sc.height_px.div_ceil(sc.width_px) as usize;
        for i in 0..n {
            let mut sq = vec![0u8; w * w * 4];
            let rows = (sc.height_px as usize - i * w).min(w);
            sq[..rows * w * 4].copy_from_slice(&data[i * w * w * 4..(i * w + rows) * w * 4]);
            layers.extend_from_slice(&shrink(&sq, sc.width_px, size));
        }
        next += n as u32;
        screens_rows = sc.height_px;
        screens_w = sc.width_px;
    }
    let total_layers = next;
    if total_layers > 256 {
        return Err(format!("{total_layers} texture layers; the deck vertex addresses 256"));
    }
    let map_layer = |l: f32| -> Result<u8, String> {
        let l = l.round() as u32;
        if l < ex.textures.main.layers {
            return Ok(l as u8);
        }
        for &(base, n, first) in &remap {
            if l >= base && l < base + n {
                return Ok((first + l - base) as u8);
            }
        }
        Err(format!("layer {l} is in no texture array"))
    };
    // Mips: level k holds every layer at size >> k.
    let mut mip_offsets = Vec::new();
    let mut tex_blob = Vec::new();
    let mut level = layers;
    let mut px = size;
    loop {
        mip_offsets.push(tex_blob.len() as u64);
        tex_blob.extend_from_slice(&level);
        if px == 1 {
            break;
        }
        let lb = (px * px * 4) as usize;
        level = (0..total_layers as usize).flat_map(|i| shrink(&level[i * lb..(i + 1) * lb], px, px / 2)).collect();
        px /= 2;
    }

    let mut vertices: Vec<u8> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();
    let mut comps = Vec::new();
    let mut tris = 0usize;
    for r in &ex.rooms {
        let n = r.vertices as usize;
        let f = &r.files;
        let (pos, nor, uv, lay) = (
            floats(&dir, &f.position, n * 3)?,
            floats(&dir, &f.normal, n * 3)?,
            floats(&dir, &f.uv, n * 2)?,
            floats(&dir, &f.layer, n)?,
        );
        let cs = [floats(&dir, &f.c0, n * 3)?, floats(&dir, &f.c1, n * 3)?, floats(&dir, &f.c2, n * 3)?];
        // The compartment frame: the centre of its bounds, to the centimetre.
        let mut lo = [f32::MAX; 3];
        let mut hi = [f32::MIN; 3];
        for p in pos.chunks_exact(3) {
            for k in 0..3 {
                lo[k] = lo[k].min(p[k]);
                hi[k] = hi[k].max(p[k]);
            }
        }
        let origin: [f64; 3] =
            std::array::from_fn(|k| ((f64::from(lo[k]) + f64::from(hi[k])) / 2.0 * 100.0).round() / 100.0);
        // Whole spans to take off the texture coordinates (they repeat), so they fit +/-32.
        let mean =
            |a: usize| (uv.chunks_exact(2).map(|q| f64::from(q[a])).sum::<f64>() / n.max(1) as f64).floor() as f32;
        let shift = [mean(0), mean(1)];
        let vertex = |i: usize, p: &[f32], nr: &[f32], c: &[Vec<f32>; 3], layer: u8, uv: [f32; 2]| DeckVertexIn {
            position_m: std::array::from_fn(|k| (f64::from(p[i * 3 + k]) - origin[k]) as f32),
            mover: r.mover,
            layer,
            normal: [nr[i * 3], nr[i * 3 + 1], nr[i * 3 + 2]],
            colors: std::array::from_fn(|s| {
                [color_byte(c[s][i * 3]), color_byte(c[s][i * 3 + 1]), color_byte(c[s][i * 3 + 2]), 255]
            }),
            uv,
        };
        let mut ins = Vec::with_capacity(n);
        for i in 0..n {
            ins.push(vertex(i, &pos, &nor, &cs, map_layer(lay[i])?, [uv[i * 2] - shift[0], uv[i * 2 + 1] - shift[1]]));
        }
        // The console faces, after the room's own triangles: a triangle's layer is the atlas square its centre lies
        // in, and its v is measured down that square (a face never crosses one: the images are whole squares' halves).
        if let Some(fc) = &r.faces {
            if screens_rows == 0 {
                return Err(format!("{}: console faces but no screens' atlas", r.id));
            }
            let m = fc.vertices as usize;
            let g = &fc.files;
            let (fp, fnr, fuv) =
                (floats(&dir, &g.position, m * 3)?, floats(&dir, &g.normal, m * 3)?, floats(&dir, &g.uv, m * 2)?);
            let fcs = [floats(&dir, &g.c0, m * 3)?, floats(&dir, &g.c1, m * 3)?, floats(&dir, &g.c2, m * 3)?];
            let (rows, w) = (screens_rows as f32, screens_w as f32);
            for t in 0..m / 3 {
                let mid = (0..3).map(|k| fuv[(t * 3 + k) * 2 + 1]).sum::<f32>() / 3.0 * rows;
                let sq = (mid / w).floor().max(0.0);
                let layer =
                    u8::try_from(screens_first + sq as u32).map_err(|_| format!("{}: screens layer past 255", r.id))?;
                for k in 0..3 {
                    let i = t * 3 + k;
                    let v = (fuv[i * 2 + 1] * rows - sq * w) / w;
                    ins.push(vertex(i, &fp, &fnr, &fcs, layer, [fuv[i * 2], v]));
                }
            }
            tris += m / 3;
        }
        let (vertex_offset, vertex_count, index_offset, index_count) = pack(&mut vertices, &mut indices, &ins, &r.id)?;
        tris += n / 3;
        comps.push(DeckCompartment {
            id: r.id.clone(),
            name: r.name.clone(),
            deck: r.deck.clone(),
            origin_m: origin,
            vertex_offset,
            vertex_count,
            index_offset,
            index_count,
            mover: r.mover,
        });
    }
    // The space dock's frame (13b): one compartment a bay, in the deck `outside`, lit by the sun here.
    let ext_text = std::fs::read_to_string(root.join("data/space/exterior.json"))
        .map_err(|e| format!("data/space/exterior.json: {e}"))?;
    let ext: sc_core::exterior::ExteriorData =
        sc_core::data::parse("data/space/exterior.json", &ext_text).map_err(|e| e.to_string())?;
    let mats: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(root.join("data/materials/materials.json")).map_err(|e| e.to_string())?,
    )
    .map_err(|e| format!("data/materials/materials.json: {e}"))?;
    let material = |id: &str| -> Result<(u8, f32), String> {
        let m = &mats["materials"][id];
        match (m["layer"].as_u64(), m["span_m"].as_f64()) {
            (Some(l), Some(s)) => Ok((map_layer(l as f32)?, s as f32)),
            _ => Err(format!("data/materials/materials.json: no material {id:?} with a layer and a span")),
        }
    };
    let (steel_layer, span) = material(&ext.dock.material)?;
    let (light_layer, _) = material("light_panel")?;
    for bay in crate::dock::build(&ext.dock, ext.sun.dir, ext.sun.colour_srgb, span) {
        let mut lo = [f32::MAX; 3];
        let mut hi = [f32::MIN; 3];
        for v in &bay.vertices {
            for k in 0..3 {
                lo[k] = lo[k].min(v.position_m[k]);
                hi[k] = hi[k].max(v.position_m[k]);
            }
        }
        let origin: [f64; 3] =
            std::array::from_fn(|k| ((f64::from(lo[k]) + f64::from(hi[k])) / 2.0 * 100.0).round() / 100.0);
        let ins: Vec<DeckVertexIn> = bay
            .vertices
            .iter()
            .map(|v| {
                let c = [color_byte(v.colour[0]), color_byte(v.colour[1]), color_byte(v.colour[2]), 255];
                DeckVertexIn {
                    position_m: std::array::from_fn(|k| (f64::from(v.position_m[k]) - origin[k]) as f32),
                    mover: 0,
                    layer: if v.light { light_layer } else { steel_layer },
                    normal: v.normal,
                    colors: [c, c, c],
                    uv: v.uv,
                }
            })
            .collect();
        let (vertex_offset, vertex_count, index_offset, index_count) =
            pack(&mut vertices, &mut indices, &ins, &bay.id)?;
        tris += ins.len() / 3;
        comps.push(DeckCompartment {
            id: bay.id.clone(),
            name: "Space dock".into(),
            deck: "outside".into(),
            origin_m: origin,
            vertex_offset,
            vertex_count,
            index_offset,
            index_count,
            mover: 0,
        });
    }
    // The walk world: its triangles carried as they are, checked finite and whole.
    let w = &ex.walk;
    let walk_tris = floats(&dir, &w.triangles, w.triangle_count as usize * 9)?;
    let index = DeckIndex {
        ship: ex.ship.clone(),
        source: "the deck plan's kit (docs/mockups/lib/shipkit.js) via tools/deck/export_deck.mjs".into(),
        compartments: comps,
        textures: DeckTextures {
            size_px: size,
            layers: total_layers,
            mips: mip_offsets.len() as u32,
            mip_offsets,
            panel_first: ex.panel_first,
            panel_glow: [ex.panel_glow.normal, ex.panel_glow.red_alert, ex.panel_glow.emergency],
            screens_first,
        },
        vertex_bytes: vertices.len() as u64,
        index_count: indices.len() as u64,
        texture_bytes: tex_blob.len() as u64,
        walk: DeckWalk {
            triangle_count: w.triangle_count,
            start: w.start.clone(),
            ladders: w.ladders.clone(),
            hatches: w.hatches.clone(),
            doors: w.doors.clone(),
            lifts: w.lifts.clone(),
        },
        views: ex.views.clone(),
    };
    let bytes = deck::write(&index, &vertices, &indices, &tex_blob, &walk_tris);
    deck::read(&bytes).map_err(|e| format!("the deck just written does not read back: {e}"))?;
    let out = root.join("compiled").join(format!("{ship}.deck"));
    std::fs::create_dir_all(out.parent().unwrap()).map_err(|e| e.to_string())?;
    std::fs::write(&out, &bytes).map_err(|e| format!("{}: {e}", out.display()))?;
    Ok(format!(
        "{}: {} compartments, {} triangles, {} vertices ({:.1} MB), {} texture layers of {} px with {} mips ({:.1} MB), a walk world of {} triangles, {:.1} MB in all",
        out.display(),
        index.compartments.len(),
        tris,
        vertices.len() / 28,
        vertices.len() as f64 / 1e6,
        total_layers,
        size,
        index.textures.mips,
        tex_blob.len() as f64 / 1e6,
        w.triangle_count,
        bytes.len() as f64 / 1e6
    ))
}
