//! The deck vertex: 28 bytes, packed by `deckc` and read by the deck pipeline
//! (openspec/changes/deck-pipeline section 5; engine-stack design section 7).
//!
//! It lives in the core because the writer (`deckc` in `sc-tools`) and the reader (`sc-render`'s
//! pipeline) must agree byte for byte, and one table named once is how they agree (CLAUDE.md 6.6:
//! validate the real artifact). `sc-render` builds its vertex layout from `DECK_ATTRIBUTES` and a
//! render test draws a vertex packed here.

/// Bytes in one deck vertex.
pub const DECK_VERTEX_BYTES: usize = 28;

/// Position and texture coordinates are stored in 1/1024 of their unit (a metre, a layer's span).
pub const FIXED_SCALE: f32 = 1024.0;

/// How the GPU reads one attribute (named for OpenGL ES 3.0; `sc-render` maps them to sokol_gfx).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum AttrFormat {
    /// Four signed 16-bit integers, read by the shader as integers (`ivec4`).
    Short4,
    /// Two signed 16-bit integers, read by the shader as integers (`ivec2`).
    Short2,
    /// Signed 2_10_10_10 (x in the low bits), normalized to -1..1.
    Int10N2,
    /// Four unsigned bytes, normalized to 0..1.
    UByte4N,
}

/// One attribute of the deck vertex: its shader name, byte offset and format.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Attribute {
    /// The name the deck shader gives the input.
    pub name: &'static str,
    /// Byte offset in the vertex.
    pub offset: usize,
    /// How it is read.
    pub format: AttrFormat,
}

/// The deck vertex's attributes, in shader input order.
pub const DECK_ATTRIBUTES: [Attribute; 6] = [
    // Position x, y, z in 1/1024 m, then the mover (byte 6) and texture layer (byte 7) as one lane.
    Attribute { name: "pos_mover_layer", offset: 0, format: AttrFormat::Short4 },
    Attribute { name: "normal", offset: 8, format: AttrFormat::Int10N2 },
    Attribute { name: "color_normal", offset: 12, format: AttrFormat::UByte4N },
    Attribute { name: "color_red_alert", offset: 16, format: AttrFormat::UByte4N },
    Attribute { name: "color_emergency", offset: 20, format: AttrFormat::UByte4N },
    Attribute { name: "uv", offset: 24, format: AttrFormat::Short2 },
];

/// A vertex before packing, in metres, a unit normal, colours as bytes and texture coordinates in
/// layer spans.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct DeckVertexIn {
    /// Compartment-local position in metres, within +/-32 m.
    pub position_m: [f32; 3],
    /// The mover this vertex rides (0: static).
    pub mover: u8,
    /// The texture array layer.
    pub layer: u8,
    /// A unit normal.
    pub normal: [f32; 3],
    /// The baked colour in the normal, red alert and emergency states, RGBA8.
    pub colors: [[u8; 4]; 3],
    /// The texture coordinate in layer spans, within +/-32.
    pub uv: [f32; 2],
}

/// The vertex could not be packed: a value outside what 28 bytes can hold, or not finite.
#[derive(Debug, PartialEq)]
pub struct PackError(pub String);

fn fixed(v: f32, what: &str) -> Result<i16, PackError> {
    let x = (v * FIXED_SCALE).round();
    if !x.is_finite() || x < f32::from(i16::MIN) || x > f32::from(i16::MAX) {
        return Err(PackError(format!("{what} {v} is outside +/-32 at 1/1024")));
    }
    Ok(x as i16)
}

fn snorm10(v: f32) -> u32 {
    let x = (v.clamp(-1.0, 1.0) * 511.0).round() as i32;
    (x as u32) & 0x3ff
}

/// Pack one vertex into its 28 bytes, little-endian, as the GPU reads them.
pub fn pack_deck_vertex(v: &DeckVertexIn) -> Result<[u8; DECK_VERTEX_BYTES], PackError> {
    if v.normal.iter().any(|c| !c.is_finite()) {
        return Err(PackError("normal is not finite".into()));
    }
    let mut out = [0u8; DECK_VERTEX_BYTES];
    for (i, &p) in v.position_m.iter().enumerate() {
        out[i * 2..i * 2 + 2].copy_from_slice(&fixed(p, "position")?.to_le_bytes());
    }
    out[6] = v.mover;
    out[7] = v.layer;
    let n = snorm10(v.normal[0]) | (snorm10(v.normal[1]) << 10) | (snorm10(v.normal[2]) << 20);
    out[8..12].copy_from_slice(&n.to_le_bytes());
    for (k, c) in v.colors.iter().enumerate() {
        out[12 + k * 4..16 + k * 4].copy_from_slice(c);
    }
    out[24..26].copy_from_slice(&fixed(v.uv[0], "texture u")?.to_le_bytes());
    out[26..28].copy_from_slice(&fixed(v.uv[1], "texture v")?.to_le_bytes());
    Ok(out)
}

/// The mover and layer from the position's fourth lane as the shader reads it: a sign-extended
/// integer whose low 16 bits hold the mover (low byte) and the layer (high byte), decoded with the
/// same mask and shift as `deck.glsl` (engine-stack design 7).
pub fn decode_mover_layer(w: i32) -> (u8, u8) {
    let u = w & 0xffff;
    ((u & 0xff) as u8, (u >> 8) as u8)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v() -> DeckVertexIn {
        DeckVertexIn {
            position_m: [1.5, -0.25, 31.0],
            mover: 3,
            layer: 200,
            normal: [0.0, 1.0, 0.0],
            colors: [[255, 128, 0, 255], [10, 20, 30, 40], [1, 2, 3, 4]],
            uv: [0.5, -2.0],
        }
    }

    #[test]
    fn the_vertex_is_twenty_eight_bytes_and_its_attributes_tile_it() {
        let mut end = 0;
        for a in DECK_ATTRIBUTES {
            assert_eq!(a.offset, end, "{} starts where the last ended", a.name);
            end += match a.format {
                AttrFormat::Short4 => 8,
                AttrFormat::Short2 | AttrFormat::Int10N2 | AttrFormat::UByte4N => 4,
            };
        }
        assert_eq!(end, DECK_VERTEX_BYTES);
    }

    #[test]
    fn mover_and_layer_survive_the_signed_lane_the_shader_reads() {
        for (mover, layer) in [(0u8, 0u8), (3, 200), (255, 255), (0, 128), (17, 127)] {
            let mut x = v();
            x.mover = mover;
            x.layer = layer;
            let b = pack_deck_vertex(&x).unwrap();
            let w = i32::from(i16::from_le_bytes([b[6], b[7]]));
            assert_eq!(decode_mover_layer(w), (mover, layer), "layer {layer} with the sign bit set decodes exactly");
        }
    }

    #[test]
    fn positions_are_kept_to_a_millimetre() {
        let b = pack_deck_vertex(&v()).unwrap();
        let x = f32::from(i16::from_le_bytes([b[0], b[1]])) / FIXED_SCALE;
        let z = f32::from(i16::from_le_bytes([b[4], b[5]])) / FIXED_SCALE;
        assert!((x - 1.5).abs() < 0.001 && (z - 31.0).abs() < 0.001);
    }

    #[test]
    fn a_position_beyond_thirty_two_metres_is_refused() {
        let mut x = v();
        x.position_m[0] = 32.5;
        assert!(pack_deck_vertex(&x).is_err(), "a compartment wider than the format is a deckc error, not a wrap");
    }

    #[test]
    fn an_up_normal_packs_to_the_top_of_ten_bits() {
        let b = pack_deck_vertex(&v()).unwrap();
        let n = u32::from_le_bytes([b[8], b[9], b[10], b[11]]);
        assert_eq!((n >> 10) & 0x3ff, 511, "y = +1 is 511 in signed 10 bits");
        assert_eq!(n & 0x3ff, 0);
    }
}
