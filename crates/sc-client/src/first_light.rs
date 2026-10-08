//! First light's placeholder room: a 6 x 8 m box, 3 m high, with a panel band and floor tiles in a
//! test texture, and colours standing in for a bake in each lighting state (a pool under a lamp in
//! normal light, red on red alert, amber low light on emergency power). It is made here only until
//! `deckc` compiles real decks (openspec/changes/deck-pipeline); nothing about it is a deck rule.

use sc_core::vertex::{pack_deck_vertex, DeckVertexIn};

/// A colour byte is a display multiplier, 0-2x (light-baking design 5): 128 shows the texture as is.
fn shade(p: [f32; 3]) -> [[u8; 4]; 3] {
    // Light from a lamp at the centre of the ceiling, and a low red and amber fill.
    let d = ((p[0]).powi(2) + (p[1] - 3.0).powi(2) + (p[2] - 2.0).powi(2)).sqrt();
    let lamp = (1.6 / (1.0 + 0.18 * d * d)).clamp(0.12, 1.6);
    let b = |x: f32| (x * 128.0).round().clamp(0.0, 255.0) as u8;
    let normal = [b(lamp), b(lamp * 0.96), b(lamp * 0.9), 255];
    let red = [b(lamp * 0.9 + 0.1), b(lamp * 0.12), b(lamp * 0.1), 255];
    let low = (lamp * 0.45).max(0.05);
    let amber = [b(low), b(low * 0.55), b(low * 0.2), 255];
    [normal, red, amber]
}

fn face(v: &mut Vec<u8>, idx: &mut Vec<u16>, corners: [[f32; 3]; 4], normal: [f32; 3], layer: u8, cells: u32) {
    // Split into cells so the stand-in light varies across the face, as a vertex bake would.
    let lerp = |a: [f32; 3], b: [f32; 3], t: f32| {
        [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t]
    };
    let base = (v.len() / 28) as u16;
    let n = cells + 1;
    for j in 0..n {
        for i in 0..n {
            let (s, t) = (i as f32 / cells as f32, j as f32 / cells as f32);
            let p = lerp(lerp(corners[0], corners[1], s), lerp(corners[3], corners[2], s), t);
            let len_u = ((corners[1][0] - corners[0][0]).powi(2)
                + (corners[1][1] - corners[0][1]).powi(2)
                + (corners[1][2] - corners[0][2]).powi(2))
            .sqrt();
            let len_v = ((corners[3][0] - corners[0][0]).powi(2)
                + (corners[3][1] - corners[0][1]).powi(2)
                + (corners[3][2] - corners[0][2]).powi(2))
            .sqrt();
            let d = DeckVertexIn {
                position_m: p,
                mover: 0,
                layer,
                normal,
                colors: shade(p),
                uv: [s * len_u / 2.0, t * len_v / 2.0],
            };
            v.extend_from_slice(&pack_deck_vertex(&d).expect("the room fits the format"));
        }
    }
    for j in 0..cells {
        for i in 0..cells {
            let a = base + (j * n + i) as u16;
            idx.extend_from_slice(&[a, a + 1, a + 1 + n as u16, a, a + 1 + n as u16, a + n as u16]);
        }
    }
}

/// The room's vertices (28 bytes each) and 16-bit indices, faces wound to face inward.
pub fn room() -> (Vec<u8>, Vec<u16>) {
    let (x, z0, z1, h) = (3.0f32, -4.0f32, 4.0f32, 3.0f32);
    let (mut v, mut i) = (Vec::new(), Vec::new());
    face(&mut v, &mut i, [[-x, 0.0, z1], [x, 0.0, z1], [x, 0.0, z0], [-x, 0.0, z0]], [0.0, 1.0, 0.0], 0, 12);
    face(&mut v, &mut i, [[-x, h, z0], [x, h, z0], [x, h, z1], [-x, h, z1]], [0.0, -1.0, 0.0], 1, 12);
    face(&mut v, &mut i, [[-x, 0.0, z1], [-x, 0.0, z0], [-x, h, z0], [-x, h, z1]], [1.0, 0.0, 0.0], 2, 12);
    face(&mut v, &mut i, [[x, 0.0, z0], [x, 0.0, z1], [x, h, z1], [x, h, z0]], [-1.0, 0.0, 0.0], 2, 12);
    face(&mut v, &mut i, [[x, 0.0, z1], [-x, 0.0, z1], [-x, h, z1], [x, h, z1]], [0.0, 0.0, -1.0], 3, 12);
    face(&mut v, &mut i, [[-x, 0.0, z0], [x, 0.0, z0], [x, h, z0], [-x, h, z0]], [0.0, 0.0, 1.0], 2, 12);
    (v, i)
}

/// Four 64 px test layers: floor plates, ceiling panels, wall panels with a band, a fore wall.
pub fn layers() -> (u32, Vec<u8>) {
    let s = 64u32;
    let mut out = Vec::new();
    for l in 0..4u32 {
        for y in 0..s {
            for x in 0..s {
                let edge = x % 32 == 0 || y % 32 == 0;
                let c: [u8; 3] = match l {
                    0 => {
                        if edge {
                            [40, 44, 50]
                        } else if (x / 4 + y / 4) % 2 == 0 {
                            [120, 124, 130]
                        } else {
                            [110, 114, 120]
                        }
                    }
                    1 => {
                        if edge {
                            [60, 62, 66]
                        } else {
                            [150, 152, 156]
                        }
                    }
                    2 => {
                        if y == 20 || y == 21 {
                            [200, 140, 40]
                        } else if x % 16 == 0 {
                            [70, 74, 80]
                        } else {
                            [130, 134, 140]
                        }
                    }
                    _ => {
                        if y > 22 && y < 42 && x > 8 && x < 56 {
                            [30, 60, 90]
                        } else {
                            [125, 128, 134]
                        }
                    }
                };
                out.extend_from_slice(&[c[0], c[1], c[2], 255]);
            }
        }
    }
    (s, out)
}
