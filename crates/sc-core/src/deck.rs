//! The compiled deck file (`compiled/<ship>.deck`): every compartment's render mesh in the 28-byte deck
//! vertex, its indices, and the ship's texture array with its mipmaps (openspec/changes/deck-pipeline,
//! design sections 9 and 13).
//!
//! It lives in the core because the writer (`deckc` in `sc-tools`) and the readers (the client, later the
//! server's compartment data) must agree on one format, read in one sequential pass. The core never reads
//! the file: callers hand it bytes.
//!
//! Layout, little-endian: `"SCDK"`, the version (u32), the index's length in bytes (u32), the index as
//! JSON (`DeckIndex`), then the blobs the index points into: vertices, indices (u32), texture mip levels.

use serde::{Deserialize, Serialize};

/// The file's magic number.
pub const MAGIC: &[u8; 4] = b"SCDK";
/// The format version this build reads and writes.
pub const VERSION: u32 = 1;

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
pub fn write(index: &DeckIndex, vertices: &[u8], indices: &[u32], textures: &[u8]) -> Vec<u8> {
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
    }
    let t = &index.textures;
    if t.mip_offsets.len() != t.mips as usize || t.layers == 0 || t.layers > 256 {
        return Err(e("the texture index is inconsistent"));
    }
    Ok(Deck { index, vertices, indices, textures })
}

impl Deck<'_> {
    /// A compartment's indices, as u32 relative to its own first vertex.
    pub fn indices_of(&self, c: &DeckCompartment) -> Vec<u32> {
        let s = &self.indices[c.index_offset as usize * 4..(c.index_offset as usize + c.index_count as usize) * 4];
        s.chunks_exact(4).map(|b| u32::from_le_bytes(b.try_into().unwrap())).collect()
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
            }],
            textures: DeckTextures {
                size_px: 1,
                layers: 1,
                mips: 1,
                mip_offsets: vec![0],
                panel_first: 0,
                panel_glow: [1.0, 1.0, 0.3],
            },
            vertex_bytes: 84,
            index_count: 3,
            texture_bytes: 4,
        };
        (index, vec![7; 84], vec![0, 1, 2], vec![255; 4])
    }

    #[test]
    fn a_deck_reads_back_as_written() {
        let (i, v, x, t) = tiny();
        let bytes = write(&i, &v, &x, &t);
        let d = read(&bytes).unwrap();
        assert_eq!(d.index, i);
        assert_eq!(d.indices_of(&i.compartments[0]), x);
        assert_eq!(d.vertices_of(&i.compartments[0]), &v[..]);
    }

    #[test]
    fn a_truncated_deck_is_refused() {
        let (i, v, x, t) = tiny();
        let bytes = write(&i, &v, &x, &t);
        assert!(read(&bytes[..bytes.len() - 1]).is_err(), "a short file must not be read past its end");
    }

    #[test]
    fn a_compartment_pointing_outside_the_file_is_refused() {
        let (mut i, v, x, t) = tiny();
        i.compartments[0].vertex_count = 4;
        assert!(read(&write(&i, &v, &x, &t)).is_err());
    }
}
