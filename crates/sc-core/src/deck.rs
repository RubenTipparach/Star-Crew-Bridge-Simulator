//! The compiled deck file (`compiled/<ship>.deck`): every compartment's render mesh in the 28-byte deck
//! vertex, its indices, the ship's texture array with its mipmaps, and the walk world: the triangles a body
//! collides with and what it uses (openspec/changes/deck-pipeline, design sections 9, 13 and 13a).
//!
//! It lives in the core because the writer (`deckc` in `sc-tools`) and the readers (the client, later the
//! server's compartment data) must agree on one format, read in one sequential pass. The core never reads
//! the file: callers hand it bytes.
//!
//! Layout, little-endian: `"SCDK"`, the version (u32), the index's length in bytes (u32), the index as
//! JSON (`DeckIndex`), then the blobs the index points into: vertices, indices (u32), texture mip levels, the
//! walk's triangles (nine f32 each: three corners in ship coordinates, metres), and the light probes
//! (`crate::probes::PROBE_BYTES` each).
//!
//! Version 3 (deck-pipeline 13b) adds a compartment's mover (the lift's car is a compartment of its own that moves),
//! a lift's speed, door time and car, the viewscreens, and the first layer of the console screens' atlas. Version 4
//! (crew-npcs 7) adds a compartment's floor centre, where a bot goes to work in it, and the crew figures (the deck
//! `figure`). Version 5 (light-baking 16) adds each compartment's light probe grid and the probes' blob. Version 6
//! (tern-exterior 3) adds the ship's hull from its exterior model: its mesh in the vertex and index blobs, scaled to fit
//! the vertex's range, its parts, and its own texture arrays after the rooms' in the texture blob.

use serde::{Deserialize, Serialize};

/// The file's magic number.
pub const MAGIC: &[u8; 4] = b"SCDK";
/// The format version this build reads and writes.
pub const VERSION: u32 = 6;

/// One compartment's mesh in the file.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct DeckCompartment {
    /// The layout's compartment id.
    pub id: String,
    /// Its name.
    pub name: String,
    /// The deck it belongs to (its lowest).
    pub deck: String,
    /// The compartment frame's origin in ship coordinates, metres; vertex positions are relative to it.
    pub origin_m: [f64; 3],
    /// Byte offset of its vertices in the vertex blob (a multiple of 28).
    pub vertex_offset: u64,
    /// Its vertex count.
    pub vertex_count: u32,
    /// Offset, in indices, of its indices in the index blob.
    pub index_offset: u64,
    /// Its index count (three a triangle).
    pub index_count: u32,
    /// The mover every vertex of it rides (0: static; the lift's car is 1), also in each vertex (`sc-core::vertex`).
    pub mover: u8,
    /// The middle of its floor on its lowest deck, ship coordinates, metres: where a bot goes to work in it. None for
    /// what is not a room (the dock, the figures, the lift's car).
    pub floor_m: Option<[f32; 3]>,
    /// Its light probes (light-baking 16): None for what has none (the dock, the figures, the lift's car).
    pub probes: Option<DeckProbes>,
}

/// A compartment's light probe grid: points `spacing_m` apart on each axis from `origin_m`, `dims` on each axis,
/// x fastest, then y, then z; its probes are `dims[0] * dims[1] * dims[2]` in the probe blob from `first`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct DeckProbes {
    /// The first grid point, ship coordinates, metres.
    pub origin_m: [f64; 3],
    /// The distance between neighbouring points, metres.
    pub spacing_m: f32,
    /// Points on the x, y and z axes.
    pub dims: [u32; 3],
    /// Index of its first probe in the probe blob.
    pub first: u64,
}

/// The texture array: square RGBA8 layers with a full mip chain, largest first.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct DeckTextures {
    /// Width and height of layer 0's top level, pixels.
    pub size_px: u32,
    /// Layers in the array.
    pub layers: u32,
    /// Mip levels, the top one included.
    pub mips: u32,
    /// Byte offset of each mip level (all layers of it) in the texture blob.
    pub mip_offsets: Vec<u64>,
    /// The first layer whose alpha is an emission mask (the panel layers, wall-panels).
    pub panel_first: u32,
    /// How bright the masks glow in the normal, red alert and emergency states.
    pub panel_glow: [f32; 3],
    /// The first layer of the console screens' atlas (bridge-stations 11.6), after every other layer; `layers` when none.
    pub screens_first: u32,
}

/// Where a walk starts: a floor point and the way the body faces in the plan.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct WalkStart {
    /// The feet, ship coordinates, metres.
    pub at_m: [f32; 3],
    /// The facing, `[x, z]`, any length.
    pub face: [f32; 2],
}

/// A ladder or floor hatch: climbed between its lower and upper floor at its centre.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct WalkLadder {
    /// Centre, metres.
    pub x_m: f32,
    /// Centre, metres.
    pub z_m: f32,
    /// The lower floor's height, metres.
    pub lo_m: f32,
    /// The upper floor's height, metres.
    pub hi_m: f32,
}

/// A hatch in a wall whose sill is too high to step over: gone through with Use.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct WalkHatch {
    /// Centre, metres.
    pub x_m: f32,
    /// Centre, metres.
    pub z_m: f32,
    /// The wall's normal, `[x, z]`.
    pub normal: [f32; 2],
    /// The sill's height, metres.
    pub sill_m: f32,
}

/// A door's opening: a door, a pressure door or a lift's landing door.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct WalkDoor {
    /// The layout's portal id.
    pub id: String,
    /// `door`, `pressure_door` or `lift`.
    pub kind: String,
    /// The opening's centre, metres.
    pub center_m: [f32; 3],
    /// The opening's normal, `[x, z]`.
    pub normal: [f32; 2],
    /// Clear width, metres.
    pub width_m: f32,
    /// Clear height, metres.
    pub height_m: f32,
}

/// A lift: its shaft's footprint, its stops and where its car's floor is.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct WalkLift {
    /// The shaft's footprint, `[x, z]` corners, metres.
    pub poly: Vec<[f32; 2]>,
    /// The floors it stops at, metres.
    pub stops_m: Vec<f32>,
    /// The car's floor where the deck draws it, metres.
    pub car_m: f32,
    /// The car's speed between stops, metres a second.
    pub speed_m_s: f32,
    /// Seconds its doors take to open, and to close.
    pub door_s: f32,
    /// The compartment that is its car (a mover), if the deck draws one.
    pub car_room: Option<String>,
}

/// A viewscreen: a screen on a wall showing the outside (deck-pipeline 13b).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct DeckView {
    /// The layout's fixture id.
    pub id: String,
    /// The screen's centre, ship coordinates, metres.
    pub center_m: [f32; 3],
    /// Width and height, metres.
    pub size_m: [f32; 2],
    /// The way it faces in the plan, degrees about +Y (0: +z, the bow).
    pub facing_yaw_deg: f32,
}

/// The walk world's index: the triangle count (the triangles are the last blob) and the entities.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct DeckWalk {
    /// Triangles in the walk blob.
    pub triangle_count: u64,
    /// Where a walk starts.
    pub start: WalkStart,
    /// Ladders and floor hatches.
    pub ladders: Vec<WalkLadder>,
    /// Wall hatches gone through with Use.
    pub hatches: Vec<WalkHatch>,
    /// Every door's opening.
    pub doors: Vec<WalkDoor>,
    /// The lifts.
    pub lifts: Vec<WalkLift>,
}

/// The ship's hull, from its exterior model (tern-exterior 3): one mesh in the shared blobs, drawn in parts.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct DeckHull {
    /// The model it was compiled from, for the record.
    pub source: String,
    /// The mesh frame's origin in ship coordinates, metres: a vertex position times `scale` is relative to it.
    pub origin_m: [f64; 3],
    /// What a stored position is multiplied by to give metres (the hull is longer than the vertex's +/-32 m).
    pub scale: f32,
    /// Byte offset of its vertices in the vertex blob.
    pub vertex_offset: u64,
    /// Its vertex count.
    pub vertex_count: u32,
    /// Offset, in indices, of its indices in the index blob (relative to its own first vertex).
    pub index_offset: u64,
    /// Its index count.
    pub index_count: u32,
    /// Its parts, one a material, in the model's order.
    pub parts: Vec<HullPart>,
    /// Its texture arrays.
    pub arrays: Vec<HullArray>,
}

impl DeckHull {
    /// Which of its arrays are magnified bilinear rather than nearest: those sampled by the `smooth` materials, the
    /// ones painted on a UV atlas (`data/ships/exteriors.json`, a ship's `smooth_magnify_materials`; tern-exterior
    /// design 2). A tiling material layer keeps the texel look. An error names a listed material the hull does not
    /// have, or an array that a listed and an unlisted material both sample, since one array takes one sampler.
    pub fn smooth_arrays(&self, smooth: &[String]) -> Result<Vec<bool>, String> {
        if let Some(m) = smooth.iter().find(|m| !self.parts.iter().any(|p| &p.material == *m)) {
            return Err(format!("smooth_magnify_materials: the hull has no material {m}"));
        }
        let mut kind: Vec<Option<bool>> = vec![None; self.arrays.len()];
        for p in &self.parts {
            let Some(a) = p.array else { continue };
            let slot =
                kind.get_mut(a as usize).ok_or_else(|| format!("hull part {} samples no array {a}", p.material))?;
            let s = smooth.contains(&p.material);
            if slot.is_some_and(|k| k != s) {
                return Err(format!(
                    "hull array {a} is sampled by painted and tiling materials alike ({})",
                    p.material
                ));
            }
            *slot = Some(s);
        }
        Ok(kind.into_iter().map(|k| k.unwrap_or(false)).collect())
    }
}

/// One material's triangles of the hull.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct HullPart {
    /// The model's material name.
    pub material: String,
    /// Its first index, within the hull's indices.
    pub first: u32,
    /// Its index count (three a triangle).
    pub count: u32,
    /// The texture array it samples (an index into `DeckHull::arrays`); None for a glowing part, whose colour is its
    /// light.
    pub array: Option<u32>,
}

/// One of the hull's texture arrays: square RGBA8 layers with a full mip chain, largest first.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct HullArray {
    /// Width and height of a layer's top level, pixels.
    pub size_px: u32,
    /// Layers.
    pub layers: u32,
    /// Byte offset of each mip level (all layers of it) in the texture blob.
    pub mip_offsets: Vec<u64>,
}

/// The file's index.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct DeckIndex {
    /// The ship's id.
    pub ship: String,
    /// What made it, for the record.
    pub source: String,
    /// Every compartment, in layout order.
    pub compartments: Vec<DeckCompartment>,
    /// The texture array.
    pub textures: DeckTextures,
    /// Bytes in the vertex blob.
    pub vertex_bytes: u64,
    /// Indices in the index blob.
    pub index_count: u64,
    /// Bytes in the texture blob.
    pub texture_bytes: u64,
    /// The walk world.
    pub walk: DeckWalk,
    /// The viewscreens.
    pub views: Vec<DeckView>,
    /// Probes in the probe blob.
    pub probe_count: u64,
    /// The ship's hull from its exterior model, when it has one.
    pub hull: Option<DeckHull>,
}

/// A deck file opened from its bytes.
pub struct Deck<'a> {
    /// The index.
    pub index: DeckIndex,
    /// All vertices, 28 bytes each.
    pub vertices: &'a [u8],
    /// All indices, little-endian u32.
    pub indices: &'a [u8],
    /// All texture mip levels.
    pub textures: &'a [u8],
    /// The walk's triangles, nine little-endian f32 each.
    pub walk: &'a [u8],
    /// The light probes, `crate::probes::PROBE_BYTES` each.
    pub probes: &'a [u8],
}

/// The file could not be read: what was wrong.
#[derive(Debug, PartialEq)]
pub struct DeckError(pub String);

impl std::fmt::Display for DeckError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for DeckError {}

/// Write a deck file from its parts.
pub fn write(
    index: &DeckIndex,
    vertices: &[u8],
    indices: &[u32],
    textures: &[u8],
    walk: &[f32],
    probes: &[u8],
) -> Vec<u8> {
    let json = serde_json::to_vec(index).expect("the index serialises");
    let mut out = Vec::with_capacity(12 + json.len() + vertices.len() + indices.len() * 4 + textures.len());
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&VERSION.to_le_bytes());
    out.extend_from_slice(&(json.len() as u32).to_le_bytes());
    out.extend_from_slice(&json);
    out.extend_from_slice(vertices);
    for i in indices {
        out.extend_from_slice(&i.to_le_bytes());
    }
    out.extend_from_slice(textures);
    for f in walk {
        out.extend_from_slice(&f.to_le_bytes());
    }
    out.extend_from_slice(probes);
    out
}

/// Open a deck file's bytes, checking every range the index names (CLAUDE.md 6.6: guard the edges).
pub fn read(bytes: &[u8]) -> Result<Deck<'_>, DeckError> {
    let e = |m: &str| DeckError(m.to_owned());
    if bytes.len() < 12 || &bytes[0..4] != MAGIC {
        return Err(e("not a deck file (no SCDK header)"));
    }
    let version = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
    if version != VERSION {
        return Err(DeckError(format!(
            "deck version {version}; this build reads {VERSION}: rebuild it with sc-tools deckc"
        )));
    }
    let n = u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize;
    let json = bytes.get(12..12 + n).ok_or_else(|| e("the index runs past the end"))?;
    let index: DeckIndex = serde_json::from_slice(json).map_err(|x| DeckError(format!("the index: {x}")))?;
    let mut at = 12 + n;
    let mut take = |len: u64, what: &str| -> Result<&[u8], DeckError> {
        let len = usize::try_from(len).map_err(|_| DeckError(format!("{what} too large")))?;
        let s = bytes.get(at..at + len).ok_or_else(|| DeckError(format!("{what} runs past the end")))?;
        at += len;
        Ok(s)
    };
    let vertices = take(index.vertex_bytes, "the vertex blob")?;
    let indices = take(index.index_count * 4, "the index blob")?;
    let textures = take(index.texture_bytes, "the texture blob")?;
    let walk = take(index.walk.triangle_count * 36, "the walk's triangles")?;
    let probes = take(index.probe_count * crate::probes::PROBE_BYTES as u64, "the light probes")?;
    if walk.chunks_exact(4).any(|b| !f32::from_le_bytes(b.try_into().unwrap()).is_finite()) {
        return Err(e("the walk's triangles hold a non-finite number"));
    }
    if vertices.len() % crate::vertex::DECK_VERTEX_BYTES != 0 {
        return Err(e("the vertex blob is not whole 28-byte vertices"));
    }
    for c in &index.compartments {
        let v_end = c.vertex_offset + u64::from(c.vertex_count) * crate::vertex::DECK_VERTEX_BYTES as u64;
        if v_end > index.vertex_bytes || c.index_offset + u64::from(c.index_count) > index.index_count {
            return Err(DeckError(format!("compartment {} points outside the file", c.id)));
        }
        if c.origin_m.iter().any(|x| !x.is_finite()) {
            return Err(DeckError(format!("compartment {} has a non-finite origin", c.id)));
        }
        if let Some(p) = &c.probes {
            let n = p.dims.iter().map(|&d| u64::from(d)).product::<u64>();
            if n == 0
                || p.first + n > index.probe_count
                || !(p.spacing_m > 0.0 && p.spacing_m.is_finite())
                || p.origin_m.iter().any(|x| !x.is_finite())
            {
                return Err(DeckError(format!("compartment {}'s probe grid is not usable", c.id)));
            }
        }
    }
    for v in &index.views {
        if v.center_m.iter().chain(&v.size_m).chain([&v.facing_yaw_deg]).any(|x| !x.is_finite()) {
            return Err(DeckError(format!("viewscreen {} is not finite", v.id)));
        }
    }
    for l in &index.walk.lifts {
        if !(l.speed_m_s > 0.0 && l.speed_m_s.is_finite() && l.door_s >= 0.0 && l.door_s.is_finite())
            || l.stops_m.is_empty()
        {
            return Err(e("a lift's speed, door time or stops are not usable"));
        }
    }
    let t = &index.textures;
    if t.mip_offsets.len() != t.mips as usize || t.layers == 0 || t.layers > 256 {
        return Err(e("the texture index is inconsistent"));
    }
    for (k, &off) in t.mip_offsets.iter().enumerate() {
        let px = u64::from((t.size_px >> k).max(1));
        if off + px * px * 4 * u64::from(t.layers) > index.texture_bytes {
            return Err(e("a mip level of the rooms' texture array runs past the texture blob"));
        }
    }
    if let Some(h) = &index.hull {
        check_hull(h, &index)?;
    }
    Ok(Deck { index, vertices, indices, textures, walk, probes })
}

/// Every range the hull names lies inside the file, and its numbers are usable.
fn check_hull(h: &DeckHull, index: &DeckIndex) -> Result<(), DeckError> {
    let e = |m: &str| DeckError(format!("the hull: {m}"));
    let v_end = h.vertex_offset + u64::from(h.vertex_count) * crate::vertex::DECK_VERTEX_BYTES as u64;
    if v_end > index.vertex_bytes || h.index_offset + u64::from(h.index_count) > index.index_count {
        return Err(e("its mesh points outside the file"));
    }
    if !(h.scale > 0.0 && h.scale.is_finite()) || h.origin_m.iter().any(|x| !x.is_finite()) {
        return Err(e("its scale or origin is not usable"));
    }
    for p in &h.parts {
        if p.count % 3 != 0 || u64::from(p.first) + u64::from(p.count) > u64::from(h.index_count) {
            return Err(DeckError(format!("the hull: part {} lies outside its indices", p.material)));
        }
        if p.array.is_some_and(|a| a as usize >= h.arrays.len()) {
            return Err(DeckError(format!("the hull: part {} samples a missing array", p.material)));
        }
    }
    for a in &h.arrays {
        if a.layers == 0 || a.layers > 256 || !a.size_px.is_power_of_two() {
            return Err(e("a texture array's size is not usable"));
        }
        if a.mip_offsets.len() != a.size_px.trailing_zeros() as usize + 1 {
            return Err(e("a texture array's mip chain is not whole"));
        }
        for (k, &off) in a.mip_offsets.iter().enumerate() {
            let px = u64::from((a.size_px >> k).max(1));
            if off + px * px * 4 * u64::from(a.layers) > index.texture_bytes {
                return Err(e("a texture array runs past the texture blob"));
            }
        }
    }
    Ok(())
}

impl Deck<'_> {
    /// The rooms' texture array as its mip levels, largest first, each exactly its layers' bytes (the hull's arrays
    /// follow it in the blob).
    pub fn texture_levels(&self) -> Vec<&[u8]> {
        let t = &self.index.textures;
        t.mip_offsets
            .iter()
            .enumerate()
            .map(|(k, &off)| {
                let px = (t.size_px >> k).max(1) as usize;
                &self.textures[off as usize..off as usize + px * px * 4 * t.layers as usize]
            })
            .collect()
    }
    /// The hull's indices, as u32 relative to its own first vertex (empty when the ship has no hull).
    pub fn hull_indices(&self) -> Vec<u32> {
        let Some(h) = &self.index.hull else { return Vec::new() };
        let s = &self.indices[h.index_offset as usize * 4..(h.index_offset as usize + h.index_count as usize) * 4];
        s.chunks_exact(4).map(|b| u32::from_le_bytes(b.try_into().unwrap())).collect()
    }
    /// The hull's vertex bytes (empty when the ship has no hull).
    pub fn hull_vertices(&self) -> &[u8] {
        let Some(h) = &self.index.hull else { return &[] };
        &self.vertices[h.vertex_offset as usize
            ..h.vertex_offset as usize + h.vertex_count as usize * crate::vertex::DECK_VERTEX_BYTES]
    }
    /// One of the hull's texture arrays as its mip levels, largest first.
    pub fn hull_array_levels(&self, a: &HullArray) -> Vec<&[u8]> {
        a.mip_offsets
            .iter()
            .enumerate()
            .map(|(k, &off)| {
                let px = (a.size_px >> k).max(1) as usize;
                &self.textures[off as usize..off as usize + px * px * 4 * a.layers as usize]
            })
            .collect()
    }
    /// A compartment's indices, as u32 relative to its own first vertex.
    pub fn indices_of(&self, c: &DeckCompartment) -> Vec<u32> {
        let s = &self.indices[c.index_offset as usize * 4..(c.index_offset as usize + c.index_count as usize) * 4];
        s.chunks_exact(4).map(|b| u32::from_le_bytes(b.try_into().unwrap())).collect()
    }
    /// The walk's triangles: three corners, ship coordinates, metres.
    pub fn walk_triangles(&self) -> Vec<[f32; 9]> {
        self.walk
            .chunks_exact(36)
            .map(|t| std::array::from_fn(|k| f32::from_le_bytes(t[k * 4..k * 4 + 4].try_into().unwrap())))
            .collect()
    }
    /// A compartment's probe grid, ready to sample, or None when it has none.
    pub fn probe_grid(&self, c: &DeckCompartment) -> Option<crate::probes::ProbeGrid<'_>> {
        let p = c.probes.as_ref()?;
        let n = p.dims.iter().map(|&d| d as usize).product::<usize>();
        let at = p.first as usize * crate::probes::PROBE_BYTES;
        Some(crate::probes::ProbeGrid {
            origin_m: p.origin_m,
            spacing_m: p.spacing_m,
            dims: p.dims,
            data: &self.probes[at..at + n * crate::probes::PROBE_BYTES],
        })
    }
    /// A compartment's vertex bytes.
    pub fn vertices_of(&self, c: &DeckCompartment) -> &[u8] {
        &self.vertices[c.vertex_offset as usize
            ..c.vertex_offset as usize + c.vertex_count as usize * crate::vertex::DECK_VERTEX_BYTES]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tiny() -> (DeckIndex, Vec<u8>, Vec<u32>, Vec<u8>) {
        let index = DeckIndex {
            ship: "test".into(),
            source: "unit test".into(),
            compartments: vec![DeckCompartment {
                id: "a".into(),
                name: "A".into(),
                deck: "A".into(),
                origin_m: [1.0, 2.0, 3.0],
                vertex_offset: 0,
                vertex_count: 3,
                index_offset: 0,
                index_count: 3,
                mover: 0,
                floor_m: None,
                probes: Some(DeckProbes { origin_m: [0.5, 0.5, 0.5], spacing_m: 1.0, dims: [1, 1, 1], first: 0 }),
            }],
            textures: DeckTextures {
                size_px: 1,
                layers: 1,
                mips: 1,
                mip_offsets: vec![0],
                panel_first: 0,
                panel_glow: [1.0, 1.0, 0.3],
                screens_first: 1,
            },
            vertex_bytes: 84,
            index_count: 3,
            texture_bytes: 4,
            walk: DeckWalk {
                triangle_count: 1,
                start: WalkStart { at_m: [0.0, 0.0, 0.0], face: [0.0, 1.0] },
                ladders: vec![],
                hatches: vec![],
                doors: vec![],
                lifts: vec![],
            },
            views: vec![],
            probe_count: 1,
            hull: None,
        };
        (index, vec![7; 84], vec![0, 1, 2], vec![255; 4])
    }
    const TRI: [f32; 9] = [0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0];
    const PROBE: [u8; crate::probes::PROBE_BYTES] = [128; crate::probes::PROBE_BYTES];

    #[test]
    fn a_deck_reads_back_as_written() {
        let (i, v, x, t) = tiny();
        let bytes = write(&i, &v, &x, &t, &TRI, &PROBE);
        let d = read(&bytes).unwrap();
        assert_eq!(d.index, i);
        assert_eq!(d.walk_triangles(), vec![TRI]);
        assert_eq!(d.indices_of(&i.compartments[0]), x);
        assert_eq!(d.vertices_of(&i.compartments[0]), &v[..]);
        assert_eq!(d.probe_grid(&i.compartments[0]).unwrap().data, &PROBE[..]);
    }

    /// The tiny deck with a hull: its one triangle again, a glowing part and a textured part on a 2 px array.
    fn with_hull() -> (DeckIndex, Vec<u8>, Vec<u32>, Vec<u8>) {
        let (mut i, v, mut x, mut t) = tiny();
        x.extend([0, 1, 2, 2, 1, 0]);
        i.index_count = 9;
        let first = t.len() as u64;
        t.extend(vec![200; 2 * 2 * 4 + 4]);
        i.texture_bytes = t.len() as u64;
        i.hull = Some(DeckHull {
            source: "test.glb".into(),
            origin_m: [0.0, 1.0, -3.5],
            scale: 2.0,
            vertex_offset: 0,
            vertex_count: 3,
            index_offset: 3,
            index_count: 6,
            parts: vec![
                HullPart { material: "glow".into(), first: 0, count: 3, array: None },
                HullPart { material: "skin".into(), first: 3, count: 3, array: Some(0) },
            ],
            arrays: vec![HullArray { size_px: 2, layers: 1, mip_offsets: vec![first, first + 16] }],
        });
        (i, v, x, t)
    }

    #[test]
    fn a_deck_with_a_hull_reads_back_as_written() {
        let (i, v, x, t) = with_hull();
        let bytes = write(&i, &v, &x, &t, &TRI, &PROBE);
        let d = read(&bytes).unwrap();
        assert_eq!(d.index, i);
        assert_eq!(d.hull_indices(), vec![0, 1, 2, 2, 1, 0]);
        assert_eq!(d.hull_vertices(), &v[..]);
        let levels = d.hull_array_levels(&i.hull.as_ref().unwrap().arrays[0]);
        assert_eq!(levels.iter().map(|l| l.len()).collect::<Vec<_>>(), vec![16, 4], "2 px then 1 px, RGBA8");
    }

    #[test]
    fn a_hull_pointing_outside_the_file_is_refused() {
        let (mut i, v, x, t) = with_hull();
        i.hull.as_mut().unwrap().parts[1].count = 6;
        assert!(read(&write(&i, &v, &x, &t, &TRI, &PROBE)).is_err(), "a part past the hull's indices");
        let (mut i, v, x, t) = with_hull();
        i.hull.as_mut().unwrap().arrays[0].mip_offsets[1] = 1_000;
        assert!(read(&write(&i, &v, &x, &t, &TRI, &PROBE)).is_err(), "a mip level past the texture blob");
        let (mut i, v, x, t) = with_hull();
        i.hull.as_mut().unwrap().scale = 0.0;
        assert!(read(&write(&i, &v, &x, &t, &TRI, &PROBE)).is_err(), "a scale of zero");
    }

    #[test]
    fn a_probe_grid_running_past_the_blob_is_refused() {
        let (mut i, v, x, t) = tiny();
        i.compartments[0].probes.as_mut().unwrap().dims = [2, 1, 1];
        assert!(read(&write(&i, &v, &x, &t, &TRI, &PROBE)).is_err(), "a grid must not read probes it does not have");
    }

    #[test]
    fn a_truncated_deck_is_refused() {
        let (i, v, x, t) = tiny();
        let bytes = write(&i, &v, &x, &t, &TRI, &PROBE);
        assert!(read(&bytes[..bytes.len() - 1]).is_err(), "a short file must not be read past its end");
    }

    #[test]
    fn a_compartment_pointing_outside_the_file_is_refused() {
        let (mut i, v, x, t) = tiny();
        i.compartments[0].vertex_count = 4;
        assert!(read(&write(&i, &v, &x, &t, &TRI, &PROBE)).is_err());
    }

    #[test]
    fn a_walk_triangle_that_is_not_finite_is_refused() {
        let (i, v, x, t) = tiny();
        let mut bad = TRI;
        bad[4] = f32::NAN;
        assert!(read(&write(&i, &v, &x, &t, &bad, &PROBE)).is_err(), "a NaN would poison the collision world");
    }
}
