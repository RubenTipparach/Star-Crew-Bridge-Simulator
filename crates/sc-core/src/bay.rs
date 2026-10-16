//! The compiled bays file (`compiled/stations.bays`): each spacedock version's baked inside and its fixtures' faces in
//! the 28-byte deck vertex, the inside's light probes, and the small texture array they sample with its mipmaps
//! (openspec/changes/spacedock design 8b).
//!
//! It lives in the core because the writer (`bayc` in `sc-tools`) and the reader (the client's stations) must agree on
//! one format, read in one sequential pass, as the deck file's do (`crate::deck`). The core never reads the file:
//! callers hand it bytes.
//!
//! Layout, little-endian: `"SCBY"`, the version (u32), the index's length in bytes (u32), the index as JSON
//! (`BaysIndex`), then the blobs the index points into: vertices, indices (u32), texture mip levels and the light probes
//! (`crate::probes::PROBE_BYTES` each). A mesh's stored positions are its metres about its `origin_m` divided by its
//! `scale`, so a bay hundreds of metres long fits the vertex's +/-32; its probes are in the station's frame, metres.

use serde::{Deserialize, Serialize};

use crate::deck::DeckProbes;
use crate::probes::{ProbeGrid, PROBE_BYTES};
use crate::vertex::DECK_VERTEX_BYTES;

/// The file's magic number.
pub const MAGIC: &[u8; 4] = b"SCBY";
/// The format version this build reads and writes.
pub const VERSION: u32 = 1;

/// One mesh in the file.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct BayMesh {
    /// Where its stored positions are measured from, the station's frame, metres.
    pub origin_m: [f64; 3],
    /// What a stored position is multiplied by to give metres.
    pub scale: f32,
    /// Byte offset of its first vertex in the vertex blob.
    pub vertex_offset: u64,
    /// Its vertices.
    pub vertex_count: u32,
    /// Index of its first index in the index blob (indices are local to the mesh).
    pub index_offset: u64,
    /// Its indices, three a triangle.
    pub index_count: u32,
}

/// One version's bay.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Bay {
    /// The spacedock version, "A" to "E".
    pub version: String,
    /// The mockup's digest of the bake it came from (its three colour sets and its probes).
    pub digest: String,
    /// The inside, its light baked into the three colour sets.
    pub inside: BayMesh,
    /// The fixtures' faces, their own light in the three colour sets.
    pub glow: BayMesh,
    /// Its light probe grids, the finest first (`crate::probes::sample_ship` keeps the first of equal weight).
    pub grids: Vec<DeckProbes>,
}

/// The texture array: square RGBA8 layers with a full mip chain, largest first.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct BayTextures {
    /// Width and height of a layer's top level, pixels (a power of two).
    pub size_px: u32,
    /// Layers in the array.
    pub layers: u32,
    /// Byte offset of each mip level in the texture blob, the top one first.
    pub mip_offsets: Vec<u64>,
}

/// The file's index.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct BaysIndex {
    /// The bays, one a version at most.
    pub bays: Vec<Bay>,
    /// The texture array every bay samples.
    pub textures: BayTextures,
    /// Bytes in the vertex blob.
    pub vertex_bytes: u64,
    /// Indices in the index blob.
    pub index_count: u64,
    /// Bytes in the texture blob.
    pub texture_bytes: u64,
    /// Probes in the probe blob.
    pub probe_count: u64,
}

/// An open bays file: its index and the blobs, borrowed from the bytes.
#[derive(Debug)]
pub struct Bays<'a> {
    /// The index.
    pub index: BaysIndex,
    vertices: &'a [u8],
    indices: &'a [u8],
    textures: &'a [u8],
    probes: &'a [u8],
}

/// The file could not be read: what was wrong.
#[derive(Debug, PartialEq)]
pub struct BaysError(pub String);

impl std::fmt::Display for BaysError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for BaysError {}

/// Write a bays file from its parts.
pub fn write(index: &BaysIndex, vertices: &[u8], indices: &[u32], textures: &[u8], probes: &[u8]) -> Vec<u8> {
    let json = serde_json::to_vec(index).expect("the index serialises");
    let mut out =
        Vec::with_capacity(12 + json.len() + vertices.len() + indices.len() * 4 + textures.len() + probes.len());
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&VERSION.to_le_bytes());
    out.extend_from_slice(&(json.len() as u32).to_le_bytes());
    out.extend_from_slice(&json);
    out.extend_from_slice(vertices);
    for i in indices {
        out.extend_from_slice(&i.to_le_bytes());
    }
    out.extend_from_slice(textures);
    out.extend_from_slice(probes);
    out
}

/// Open a bays file's bytes, checking every range the index names (CLAUDE.md 6.6: guard the edges).
pub fn read(bytes: &[u8]) -> Result<Bays<'_>, BaysError> {
    let e = |m: String| BaysError(m);
    if bytes.len() < 12 || &bytes[0..4] != MAGIC {
        return Err(e("not a bays file (no SCBY header)".into()));
    }
    let version = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
    if version != VERSION {
        return Err(e(format!("bays version {version}; this build reads {VERSION}: rebuild it with sc-tools bayc")));
    }
    let n = u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize;
    let json = bytes.get(12..12 + n).ok_or_else(|| e("the index runs past the end".into()))?;
    let index: BaysIndex = serde_json::from_slice(json).map_err(|x| e(format!("the index: {x}")))?;
    let mut at = 12 + n;
    let mut take = |len: u64, what: &str| -> Result<&[u8], BaysError> {
        let len = usize::try_from(len).map_err(|_| BaysError(format!("{what} too large")))?;
        let s = bytes.get(at..at + len).ok_or_else(|| BaysError(format!("{what} runs past the end")))?;
        at += len;
        Ok(s)
    };
    let vertices = take(index.vertex_bytes, "the vertex blob")?;
    let indices = take(index.index_count * 4, "the index blob")?;
    let textures = take(index.texture_bytes, "the texture blob")?;
    let probes = take(index.probe_count * PROBE_BYTES as u64, "the light probes")?;
    if vertices.len() % DECK_VERTEX_BYTES != 0 {
        return Err(e("the vertex blob is not whole 28-byte vertices".into()));
    }
    for b in &index.bays {
        for (what, m) in [("inside", &b.inside), ("glow", &b.glow)] {
            let v_end = m.vertex_offset + u64::from(m.vertex_count) * DECK_VERTEX_BYTES as u64;
            if v_end > index.vertex_bytes || m.index_offset + u64::from(m.index_count) > index.index_count {
                return Err(e(format!("bay {}: its {what} points outside the file", b.version)));
            }
            if m.index_count % 3 != 0
                || !(m.scale > 0.0 && m.scale.is_finite())
                || m.origin_m.iter().any(|x| !x.is_finite())
            {
                return Err(e(format!("bay {}: its {what}'s triangles, scale or origin are not usable", b.version)));
            }
            let first = m.index_offset as usize * 4;
            let local = &indices[first..first + m.index_count as usize * 4];
            if local.chunks_exact(4).any(|q| u32::from_le_bytes(q.try_into().unwrap()) >= m.vertex_count) {
                return Err(e(format!("bay {}: its {what} indexes a vertex it does not have", b.version)));
            }
        }
        for p in &b.grids {
            let n = p.dims.iter().map(|&d| u64::from(d)).product::<u64>();
            if n == 0
                || p.first + n > index.probe_count
                || !(p.spacing_m > 0.0 && p.spacing_m.is_finite())
                || p.origin_m.iter().any(|x| !x.is_finite())
            {
                return Err(e(format!("bay {}: a probe grid is not usable", b.version)));
            }
        }
    }
    let t = &index.textures;
    if t.layers == 0 || t.layers > 256 || !t.size_px.is_power_of_two() {
        return Err(e("the texture array's size is not usable".into()));
    }
    if t.mip_offsets.len() != t.size_px.trailing_zeros() as usize + 1 {
        return Err(e("the texture array's mip chain is not whole".into()));
    }
    for (k, &off) in t.mip_offsets.iter().enumerate() {
        let px = u64::from((t.size_px >> k).max(1));
        if off + px * px * 4 * u64::from(t.layers) > index.texture_bytes {
            return Err(e("a mip level runs past the texture blob".into()));
        }
    }
    Ok(Bays { index, vertices, indices, textures, probes })
}

impl<'a> Bays<'a> {
    /// The bay of `version` ("A" to "E"), if the file holds it.
    pub fn bay(&self, version: &str) -> Option<&Bay> {
        self.index.bays.iter().find(|b| b.version == version)
    }

    /// A mesh's vertices, 28 bytes each.
    pub fn vertices_of(&self, m: &BayMesh) -> &'a [u8] {
        let at = m.vertex_offset as usize;
        &self.vertices[at..at + m.vertex_count as usize * DECK_VERTEX_BYTES]
    }

    /// A mesh's indices, local to it.
    pub fn indices_of(&self, m: &BayMesh) -> Vec<u32> {
        let at = m.index_offset as usize * 4;
        self.indices[at..at + m.index_count as usize * 4]
            .chunks_exact(4)
            .map(|q| u32::from_le_bytes(q.try_into().unwrap()))
            .collect()
    }

    /// The texture array's mip levels, largest first, each exactly its layers' bytes.
    pub fn texture_levels(&self) -> Vec<&'a [u8]> {
        let t = &self.index.textures;
        t.mip_offsets
            .iter()
            .enumerate()
            .map(|(k, &off)| {
                let px = ((t.size_px >> k).max(1)) as usize;
                &self.textures[off as usize..off as usize + px * px * 4 * t.layers as usize]
            })
            .collect()
    }

    /// A bay's probe grids, in the station's frame, the finest first.
    pub fn grids(&self, b: &Bay) -> Vec<ProbeGrid<'a>> {
        b.grids
            .iter()
            .map(|p| {
                let n = p.dims.iter().map(|&d| d as usize).product::<usize>();
                let at = p.first as usize * PROBE_BYTES;
                ProbeGrid {
                    origin_m: p.origin_m,
                    spacing_m: p.spacing_m,
                    dims: p.dims,
                    data: &self.probes[at..at + n * PROBE_BYTES],
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mesh(vertex_offset: u64, vertex_count: u32, index_offset: u64, index_count: u32) -> BayMesh {
        BayMesh { origin_m: [0.0, 0.0, 100.0], scale: 16.0, vertex_offset, vertex_count, index_offset, index_count }
    }

    /// A file of one bay: a triangle inside, a triangle of glow, one 2 x 1 x 1 probe grid and a 2 px texture.
    fn sample() -> (BaysIndex, Vec<u8>, Vec<u32>, Vec<u8>, Vec<u8>) {
        let vertices = vec![7u8; DECK_VERTEX_BYTES * 6];
        let indices = vec![0, 1, 2, 0, 1, 2];
        let textures = vec![9u8; 2 * 2 * 4 + 4];
        let mut probes = vec![100u8; PROBE_BYTES * 2];
        probes[PROBE_BYTES - 1] = 1;
        probes[2 * PROBE_BYTES - 1] = 1;
        let index = BaysIndex {
            bays: vec![Bay {
                version: "E".into(),
                digest: "d".into(),
                inside: mesh(0, 3, 0, 3),
                glow: mesh(DECK_VERTEX_BYTES as u64 * 3, 3, 3, 3),
                grids: vec![DeckProbes { origin_m: [0.0, 0.0, 0.0], spacing_m: 20.0, dims: [2, 1, 1], first: 0 }],
            }],
            textures: BayTextures { size_px: 2, layers: 1, mip_offsets: vec![0, 16] },
            vertex_bytes: vertices.len() as u64,
            index_count: indices.len() as u64,
            texture_bytes: textures.len() as u64,
            probe_count: 2,
        };
        (index, vertices, indices, textures, probes)
    }

    #[test]
    fn a_bays_file_reads_back_what_was_written() {
        let (index, v, i, t, p) = sample();
        let bytes = write(&index, &v, &i, &t, &p);
        let b = read(&bytes).expect("it reads");
        assert_eq!(b.index, index);
        let e = b.bay("E").expect("E is in it");
        assert_eq!(b.vertices_of(&e.glow).len(), DECK_VERTEX_BYTES * 3);
        assert_eq!(b.indices_of(&e.inside), vec![0, 1, 2]);
        assert_eq!(b.texture_levels().iter().map(|l| l.len()).collect::<Vec<_>>(), vec![16, 4]);
        let g = b.grids(e);
        let cube = crate::probes::sample_ship(&g, [10.0, 0.0, 0.0], [1.0, 0.0, 0.0]).expect("a valid probe");
        assert!((cube[0][0] - 100.0 * 2.0 / 255.0).abs() < 1e-5, "a probe byte is a display multiplier, 0-2");
        assert!(b.bay("A").is_none());
    }

    #[test]
    fn a_bays_file_that_points_past_its_blobs_is_refused() {
        let (mut index, v, i, t, p) = sample();
        index.bays[0].glow.index_count = 6;
        assert!(read(&write(&index, &v, &i, &t, &p)).is_err(), "the glow's indices run past the blob");
        let (mut index, v, i, t, p) = sample();
        index.bays[0].inside.vertex_count = 2;
        assert!(read(&write(&index, &v, &i, &t, &p)).is_err(), "an index names a vertex the mesh does not have");
        let (mut index, v, i, t, p) = sample();
        index.bays[0].grids[0].dims = [3, 1, 1];
        assert!(read(&write(&index, &v, &i, &t, &p)).is_err(), "the grid needs more probes than the file holds");
        let (index, v, i, t, p) = sample();
        let mut bytes = write(&index, &v, &i, &t, &p);
        bytes[4] = 9;
        assert!(read(&bytes).is_err(), "another version is refused, never misread");
    }
}
