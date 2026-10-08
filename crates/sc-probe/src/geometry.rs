//! The probe's synthetic geometry, in the deck vertex format (sc-core::vertex), so every scene
//! draws through the same pipeline the decks will.

use sc_core::vertex::{pack_deck_vertex, DeckVertexIn, DECK_VERTEX_BYTES};

/// A packed mesh before upload.
pub struct MeshData {
    /// 28-byte vertices.
    pub vertices: Vec<u8>,
    /// 16-bit indices.
    pub indices: Vec<u16>,
}

/// A colour byte is a display multiplier from 0 to 2x (light-baking design 5), so the probe's
/// colours stay at or under 1x (byte 128) to show as themselves.
fn hue(i: u32) -> [u8; 4] {
    let h = i.wrapping_mul(2_654_435_761) >> 8;
    [(40 + (h & 0x57)) as u8, (40 + ((h >> 7) & 0x57)) as u8, (40 + ((h >> 14) & 0x57)) as u8, 255]
}

fn push(out: &mut Vec<u8>, p: [f32; 3], c: [u8; 4], uv: [f32; 2], layer: u8) {
    let red = [c[0].saturating_add(30), c[1] / 3, c[2] / 3, 255];
    let amber = [c[0] / 2 + 60, c[1] / 3 + 30, c[2] / 8, 255];
    let v = DeckVertexIn { position_m: p, mover: 0, layer, normal: [0.0, 0.0, 1.0], colors: [c, red, amber], uv };
    out.extend_from_slice(&pack_deck_vertex(&v).expect("probe geometry stays inside the format"));
}

/// A grid of `cells` x `cells` squares (2 triangles each) over the rectangle `x0..x1`, `y0..y1` at
/// depth `z`, every vertex its own colour: `2 * cells^2` triangles, `(cells + 1)^2` vertices.
pub fn grid(cells: u32, x0: f32, y0: f32, x1: f32, y1: f32, z: f32, seed: u32) -> MeshData {
    let n = cells + 1;
    let mut vertices = Vec::with_capacity((n * n) as usize * DECK_VERTEX_BYTES);
    for j in 0..n {
        for i in 0..n {
            let (u, v) = (i as f32 / cells as f32, j as f32 / cells as f32);
            push(&mut vertices, [x0 + (x1 - x0) * u, y0 + (y1 - y0) * v, z], hue(seed ^ (j * n + i)), [u, v], 0);
        }
    }
    let mut indices = Vec::with_capacity((cells * cells * 6) as usize);
    for j in 0..cells {
        for i in 0..cells {
            let a = (j * n + i) as u16;
            let b = a + 1;
            let c = a + n as u16;
            let d = c + 1;
            indices.extend_from_slice(&[a, b, d, a, d, c]);
        }
    }
    MeshData { vertices, indices }
}

/// `layers` full-view quads over `x0..x1`, `y0..y1`, back to front from depth `z_far` to `z_near`,
/// so each one passes the depth test and every pixel is shaded `layers` times. Texture
/// coordinates tile a layer every `tile_m` metres; quad k samples layer k % 4.
#[allow(clippy::too_many_arguments)]
pub fn quads(layers: u32, x0: f32, y0: f32, x1: f32, y1: f32, z_far: f32, z_near: f32, tile_m: f32) -> MeshData {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    for k in 0..layers {
        let z = z_far + (z_near - z_far) * k as f32 / (layers.max(2) - 1) as f32;
        let c = hue(k * 977 + 3);
        let base = (k * 4) as u16;
        for (x, y) in [(x0, y0), (x1, y0), (x1, y1), (x0, y1)] {
            push(&mut vertices, [x, y, z], c, [x / tile_m, y / tile_m], (k % 4) as u8);
        }
        indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    MeshData { vertices, indices }
}

/// Four 128 px layers of a test pattern (checks and bands), for the textured fill scenes.
pub fn pattern_layers() -> (u32, Vec<u8>) {
    let size = 128u32;
    let mut out = Vec::with_capacity((size * size * 4 * 4) as usize);
    for l in 0..4u32 {
        for y in 0..size {
            for x in 0..size {
                let check = ((x / 16) + (y / 16) + l) % 2 == 0;
                let band = (y % 32) < 3;
                let g: u8 = if band {
                    60
                } else if check {
                    230
                } else {
                    170
                };
                out.extend_from_slice(&[g, (g as u32 * (3 + l) / 6) as u8, (g as u32 * (6 - l) / 6) as u8, 255]);
            }
        }
    }
    (size, out)
}
