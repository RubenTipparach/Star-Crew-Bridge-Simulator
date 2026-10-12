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
//!   its mesh in the screens' atlas, which is cut into square layers after every other layer, and the viewscreens;
//! - since tern-exterior 3: the ship's hull from its exterior model (`assets/models/exteriors/props.json`'s default
//!   livery), read by `crate::glb` and packed with the same packer, colour byte and mip chain as the rooms, its
//!   positions scaled to fit the vertex's +/-32 m, its textures in arrays of their own after the rooms'.

use sc_core::deck::{
    self, DeckCompartment, DeckHull, DeckIndex, DeckProbes, DeckTextures, DeckView, DeckWalk, HullArray, HullPart,
    WalkDoor, WalkHatch, WalkLadder, WalkLift, WalkStart,
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
    probes: Option<RoomProbes>,
}
/// A room's light probes (light-baking 16): the grid, then its cubes (54 bytes a probe) and valid flags (a byte a
/// probe) in their files.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RoomProbes {
    origin_m: [f64; 3],
    spacing_m: f32,
    dims: [u32; 3],
    cubes: String,
    valid: String,
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

/// A texture array's mip chain: level 0 is `level0` (`layers` square RGBA8 layers of `size` px), each next level every
/// layer box-filtered to half the size, down to 1 px.
fn mip_chain(level0: Vec<u8>, size: u32, layers: u32) -> Vec<Vec<u8>> {
    let mut levels = vec![level0];
    let mut px = size;
    while px > 1 {
        let lb = (px * px * 4) as usize;
        let last = levels.last().unwrap();
        let next = (0..layers as usize).flat_map(|i| shrink(&last[i * lb..(i + 1) * lb], px, px / 2)).collect();
        levels.push(next);
        px /= 2;
    }
    levels
}

/// The middle of compartment `id`'s floor on its lowest deck: the centroid of the largest of its lowest brushes (each is
/// a convex prism, so the centroid is inside it).
fn floor_middle(layout: &serde_json::Value, id: &str) -> Option<[f32; 3]> {
    let c = layout["compartments"].as_array()?.iter().find(|c| c["id"] == id)?;
    let brushes = c["brushes"].as_array()?;
    let lo = brushes.iter().filter_map(|b| b["y"][0].as_f64()).fold(f64::MAX, f64::min);
    let mut best: Option<(f64, [f64; 2])> = None;
    for b in brushes.iter().filter(|b| b["y"][0].as_f64() == Some(lo)) {
        let p: Vec<[f64; 2]> =
            b["poly"].as_array()?.iter().filter_map(|q| Some([q[0].as_f64()?, q[1].as_f64()?])).collect();
        let (mut a, mut cx, mut cz) = (0.0, 0.0, 0.0);
        for i in 0..p.len() {
            let (u, v) = (p[i], p[(i + 1) % p.len()]);
            let k = u[0] * v[1] - v[0] * u[1];
            a += k;
            cx += (u[0] + v[0]) * k;
            cz += (u[1] + v[1]) * k;
        }
        if a.abs() < 1e-9 {
            continue;
        }
        let area = a.abs() / 2.0;
        let centre = [cx / (3.0 * a), cz / (3.0 * a)];
        if best.is_none_or(|(ba, _)| area > ba) {
            best = Some((area, centre));
        }
    }
    best.map(|(_, c)| [c[0] as f32, lo as f32, c[1] as f32])
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
    for level in mip_chain(layers, size, total_layers) {
        mip_offsets.push(tex_blob.len() as u64);
        tex_blob.extend_from_slice(&level);
    }

    let mut vertices: Vec<u8> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();
    let mut comps = Vec::new();
    let mut tris = 0usize;
    let mut probe_blob: Vec<u8> = Vec::new();
    let mut probes_valid = 0usize;
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
        // Its light probes, each its 54 colour bytes then its valid flag (sc-core::probes).
        let probes = match &r.probes {
            None => None,
            Some(p) => {
                let count = p.dims.iter().map(|&d| d as usize).product::<usize>();
                let cubes = std::fs::read(dir.join(&p.cubes)).map_err(|e| format!("{}: {e}", p.cubes))?;
                let valid = std::fs::read(dir.join(&p.valid)).map_err(|e| format!("{}: {e}", p.valid))?;
                if count == 0 || cubes.len() != count * 54 || valid.len() != count || valid.iter().any(|&v| v > 1) {
                    return Err(format!("{}: its probes do not match their grid", r.id));
                }
                if !(p.spacing_m > 0.0 && p.spacing_m.is_finite()) || p.origin_m.iter().any(|x| !x.is_finite()) {
                    return Err(format!("{}: its probe grid is not finite", r.id));
                }
                let first = (probe_blob.len() / sc_core::probes::PROBE_BYTES) as u64;
                for (cube, v) in cubes.chunks_exact(54).zip(&valid) {
                    probe_blob.extend_from_slice(cube);
                    probe_blob.push(*v);
                }
                probes_valid += valid.iter().filter(|&&v| v == 1).count();
                Some(DeckProbes { origin_m: p.origin_m, spacing_m: p.spacing_m, dims: p.dims, first })
            }
        };
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
            floor_m: None,
            probes,
        });
    }
    // Bots' places (crew-npcs 7): each layout compartment's floor middle, set on the rooms above.
    let layout: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(root.join("data/ships").join(ship).join("layout.json")).map_err(|e| e.to_string())?,
    )
    .map_err(|e| format!("layout.json: {e}"))?;
    for c in comps.iter_mut() {
        c.floor_m = floor_middle(&layout, &c.id);
    }
    // The crew figures (crew-npcs 7): one a department, their colours unlit albedo for the probes to light
    // (light-baking 16), feet at the origin.
    let company: sc_core::crew::CompanyData = sc_core::data::parse(
        "data/crew/company.json",
        &std::fs::read_to_string(root.join("data/crew/company.json")).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    // The player's own figure on the map (ship-plan-view 5): white and amber, set apart from every department.
    let player = sc_core::crew::Department {
        id: "player".into(),
        name: "Player".into(),
        count: 0,
        colour_srgb: [1.0, 0.72, 0.3],
        rooms: Vec::new(),
    };
    for d in company.departments.iter().chain(std::iter::once(&player)) {
        let tunic = sc_core::exterior::linear(d.colour_srgb);
        let ins: Vec<DeckVertexIn> = crate::figure::build(tunic)
            .iter()
            .map(|v| {
                let c = [color_byte(v.colour[0]), color_byte(v.colour[1]), color_byte(v.colour[2]), 255];
                DeckVertexIn {
                    position_m: v.position_m,
                    mover: 0,
                    layer: 0,
                    normal: v.normal,
                    colors: [c, c, c],
                    uv: [0.0, 0.0],
                }
            })
            .collect();
        let id = format!("figure_{}", d.id);
        let (vertex_offset, vertex_count, index_offset, index_count) = pack(&mut vertices, &mut indices, &ins, &id)?;
        comps.push(DeckCompartment {
            id,
            name: format!("{} crew", d.name),
            deck: "figure".into(),
            origin_m: [0.0; 3],
            vertex_offset,
            vertex_count,
            index_offset,
            index_count,
            mover: 0,
            floor_m: None,
            probes: None,
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
            floor_m: None,
            probes: None,
        });
    }
    // The hull from the ship's exterior model (tern-exterior 3), when it has one.
    let hull = hull(root, ship, &mut vertices, &mut indices, &mut tex_blob)?;
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
        probe_count: (probe_blob.len() / sc_core::probes::PROBE_BYTES) as u64,
        hull,
    };
    let bytes = deck::write(&index, &vertices, &indices, &tex_blob, &walk_tris, &probe_blob);
    deck::read(&bytes).map_err(|e| format!("the deck just written does not read back: {e}"))?;
    let out = root.join("compiled").join(format!("{ship}.deck"));
    std::fs::create_dir_all(out.parent().unwrap()).map_err(|e| e.to_string())?;
    std::fs::write(&out, &bytes).map_err(|e| format!("{}: {e}", out.display()))?;
    let hull_note = match &index.hull {
        Some(h) => format!(
            ", a hull of {} triangles in {} parts from {}",
            h.parts.iter().map(|p| p.count as usize / 3).sum::<usize>(),
            h.parts.len(),
            h.source
        ),
        None => String::new(),
    };
    Ok(format!(
        "{}: {} compartments, {} triangles, {} vertices ({:.1} MB), {} texture layers of {} px with {} mips ({:.1} MB), a walk world of {} triangles, {} light probes ({} valid, {:.2} MB){hull_note}, {:.1} MB in all",
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
        index.probe_count,
        probes_valid,
        probe_blob.len() as f64 / 1e6,
        bytes.len() as f64 / 1e6
    ))
}

/// The ship's hull from its exterior model (tern-exterior 3), packed into the shared blobs; None when
/// `assets/models/exteriors/props.json` has no model for `ship`. The model is the manifest's default livery's full
/// detail level; its triangle count and bounds must be the manifest's, and its triangles within the ship's exterior
/// budget (`data/ships/exteriors.json`, the Pi 5 table's "Ship exterior" row).
pub(crate) fn hull(
    root: &Path,
    ship: &str,
    vertices: &mut Vec<u8>,
    indices: &mut Vec<u32>,
    tex_blob: &mut Vec<u8>,
) -> Result<Option<DeckHull>, String> {
    let read_json = |rel: &str| -> Result<serde_json::Value, String> {
        let text = std::fs::read_to_string(root.join(rel)).map_err(|e| format!("{rel}: {e}"))?;
        serde_json::from_str(&text).map_err(|e| format!("{rel}: {e}"))
    };
    let props = read_json("assets/models/exteriors/props.json")?;
    if props["props"].get(ship).is_none() {
        return Ok(None);
    }
    let livery = props["default_livery"].as_str().ok_or("props.json: no default_livery")?;
    let dir = props["liveries"]
        .as_array()
        .and_then(|l| l.iter().find(|x| x["id"] == livery))
        .and_then(|x| x["directory"].as_str())
        .ok_or_else(|| format!("props.json: no directory for the livery {livery:?}"))?;
    let base = format!("assets/models/exteriors/{dir}");
    let manifest = read_json(&format!("{base}/props.json"))?;
    let entry = &manifest["props"][ship];
    let file = entry["file"].as_str().ok_or_else(|| format!("{base}/props.json: no file for {ship}"))?;
    let source = format!("{base}/{file}");
    let model = crate::glb::read(&root.join(&source))?;

    // The checks: the manifest's triangles and bounds (rounded to the centimetre there), and the budget.
    let want = entry["triangles"].as_u64().ok_or_else(|| format!("{base}/props.json: no triangles"))? as usize;
    if model.triangles() != want {
        return Err(format!("{source}: {} triangles, its manifest says {want}", model.triangles()));
    }
    let (lo, hi) = model.bounds();
    for (k, corner) in [lo, hi].iter().enumerate() {
        let m = &entry["bounds_m"][k];
        for a in 0..3 {
            let v = m[a].as_f64().ok_or_else(|| format!("{base}/props.json: bounds_m"))?;
            if (f64::from(corner[a]) - v).abs() > 0.006 {
                return Err(format!("{source}: bounds {lo:?} to {hi:?} differ from its manifest's"));
            }
        }
    }
    let ext = read_json("data/ships/exteriors.json")?;
    let budget = ext["ships"]
        .as_array()
        .and_then(|s| s.iter().find(|x| x["id"] == ship))
        .and_then(|x| x["budget_triangles"][0].as_u64())
        .ok_or_else(|| format!("data/ships/exteriors.json: no budget_triangles for {ship}"))?;
    if model.triangles() as u64 > budget {
        return Err(format!("{source}: {} triangles, over the exterior budget of {budget}", model.triangles()));
    }

    // The frame: the centre of the bounds to the centimetre, and the smallest power of two that brings the farthest
    // corner inside the vertex's +/-32 m.
    let origin: [f64; 3] =
        std::array::from_fn(|k| ((f64::from(lo[k]) + f64::from(hi[k])) / 2.0 * 100.0).round() / 100.0);
    let reach = (0..3).map(|k| (f64::from(hi[k]) - origin[k]).max(origin[k] - f64::from(lo[k]))).fold(0.0, f64::max);
    let mut scale = 1.0f32;
    while reach / f64::from(scale) > 31.9 {
        scale *= 2.0;
    }

    // The arrays: each image the model samples, in an array of its size, in the order first used.
    let mut arrays: Vec<(u32, Vec<usize>)> = Vec::new(); // (size, images)
    let mut place: HashMap<usize, (u32, u8)> = HashMap::new(); // image -> (array, layer)
    for p in &model.primitives {
        let Some(im) = p.image else { continue };
        if place.contains_key(&im) {
            continue;
        }
        let img = &model.images[im];
        if !img.width.is_power_of_two() || !img.height.is_power_of_two() {
            return Err(format!(
                "{source}: image {} is {} x {} px, dimensions must be powers of two",
                img.name, img.width, img.height
            ));
        }
        let square_size = img.width.max(img.height);
        let a = match arrays.iter().position(|(size, _)| *size == square_size) {
            Some(a) => a,
            None => {
                arrays.push((square_size, Vec::new()));
                arrays.len() - 1
            }
        };
        arrays[a].1.push(im);
        place.insert(im, (a as u32, (arrays[a].1.len() - 1) as u8));
    }

    // The mesh: every primitive's triangles in order, one vertex an index, merged by the packer.
    let mut ins = Vec::new();
    let mut parts = Vec::new();
    for p in &model.primitives {
        let (array, layer) = match p.image {
            Some(im) => {
                let (a, l) = place[&im];
                (Some(a), l)
            }
            None => (None, 0),
        };
        parts.push(HullPart {
            material: p.material.clone(),
            first: ins.len() as u32,
            count: p.indices.len() as u32,
            array,
        });
        for &i in &p.indices {
            let i = i as usize;
            let c = p.colours[i];
            let c = [color_byte(c[0]), color_byte(c[1]), color_byte(c[2]), 255];
            ins.push(DeckVertexIn {
                position_m: std::array::from_fn(|k| {
                    ((f64::from(p.positions[i][k]) - origin[k]) / f64::from(scale)) as f32
                }),
                mover: 0,
                layer,
                normal: p.normals[i],
                colors: [c, c, c],
                uv: p.uvs[i],
            });
        }
    }
    let (vertex_offset, vertex_count, index_offset, index_count) = pack(vertices, indices, &ins, "the hull")?;

    // Each array's layers and mips, after everything already in the texture blob.
    let mut out_arrays = Vec::new();
    for (size, images) in &arrays {
        let level0: Vec<u8> = images.iter().flat_map(|&im| square_hull_image(&model.images[im])).collect();
        let mut mip_offsets = Vec::new();
        for level in mip_chain(level0, *size, images.len() as u32) {
            mip_offsets.push(tex_blob.len() as u64);
            tex_blob.extend_from_slice(&level);
        }
        out_arrays.push(HullArray { size_px: *size, layers: images.len() as u32, mip_offsets });
    }
    Ok(Some(DeckHull {
        source,
        origin_m: origin,
        scale,
        vertex_offset,
        vertex_count,
        index_offset,
        index_count,
        parts,
        arrays: out_arrays,
    }))
}

/// Expand a rectangular power-of-two image into the renderer's square layer without changing normalized UVs.
/// Exact pixel replication retains every source texel; the shared mip builder filters only after expansion.
fn square_hull_image(image: &crate::glb::Image) -> Vec<u8> {
    let size = image.width.max(image.height) as usize;
    let width = image.width as usize;
    let height = image.height as usize;
    if width == height {
        return image.rgba.clone();
    }
    let mut square = Vec::with_capacity(size * size * 4);
    for y in 0..size {
        for x in 0..size {
            let source = ((y * height / size) * width + x * width / size) * 4;
            square.extend_from_slice(&image.rgba[source..source + 4]);
        }
    }
    square
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root() -> &'static Path {
        Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
    }

    #[test]
    fn the_tern_hull_compiles_within_two_millimetres_of_its_model() {
        let (mut v, mut x, mut t) = (Vec::new(), Vec::new(), Vec::new());
        let h = hull(root(), "tern", &mut v, &mut x, &mut t).unwrap().expect("the Tern has an exterior model");
        assert_eq!(h.parts.len(), 7);
        assert_eq!(h.parts.iter().map(|p| p.count as usize / 3).sum::<usize>(), 4440);
        assert_eq!(h.scale, 2.0, "an 81.5 m hull needs half-size positions to fit +/-32 m");
        let sizes: Vec<(u32, u32)> = h.arrays.iter().map(|a| (a.size_px, a.layers)).collect();
        assert!(sizes.contains(&(1024, 1)) && sizes.contains(&(128, 2)), "the hull atlas and the two tiles: {sizes:?}");
        assert!(sizes.contains(&(2048, 1)), "the rectangular nacelle paint expands into one square layer: {sizes:?}");
        println!("Compiled hull texture allocation: {} bytes ({:.4} MiB)", t.len(), t.len() as f64 / 1048576.0);
        assert!(h.parts.iter().filter(|p| p.array.is_none()).count() == 2, "the screens and light panels glow");
        // Every corner, unpacked, against the model's own.
        let model = crate::glb::read(&root().join(&h.source)).unwrap();
        let mut k = 0usize;
        for p in &model.primitives {
            for &i in &p.indices {
                let vi = x[h.index_offset as usize + k] as usize;
                let b = &v[h.vertex_offset as usize + vi * 28..][..28];
                for a in 0..3 {
                    let q = i16::from_le_bytes([b[a * 2], b[a * 2 + 1]]);
                    let m = f64::from(q) / 1024.0 * f64::from(h.scale) + h.origin_m[a];
                    let d = (m - f64::from(p.positions[i as usize][a])).abs();
                    assert!(d <= 0.002, "corner {k} axis {a} is {d} m off");
                }
                k += 1;
            }
        }
    }

    #[test]
    fn a_ship_without_an_exterior_model_compiles_without_a_hull() {
        let (mut v, mut x, mut t) = (Vec::new(), Vec::new(), Vec::new());
        assert!(hull(root(), "hound", &mut v, &mut x, &mut t).unwrap().is_none());
        assert!(v.is_empty() && x.is_empty() && t.is_empty());
    }

    #[test]
    fn rectangular_hull_paint_preserves_each_normalized_texel() {
        for (width, height, expected) in [
            (2, 4, vec![0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7]),
            (4, 2, vec![0, 1, 2, 3, 0, 1, 2, 3, 4, 5, 6, 7, 4, 5, 6, 7]),
            (4, 4, (0..16).collect()),
        ] {
            let rgba = (0..width * height).flat_map(|i| [i as u8, 20, 30, 0]).collect();
            let image = crate::glb::Image { name: "paint".into(), width, height, rgba };
            let square = square_hull_image(&image);
            let size = width.max(height);
            assert_eq!(square.len(), (size * size * 4) as usize);
            assert_eq!(square.chunks_exact(4).map(|p| p[0]).collect::<Vec<_>>(), expected);
            assert!(square.chunks_exact(4).all(|p| p[1..] == [20, 30, 0]), "RGBA channels stay intact");
        }
    }
}
