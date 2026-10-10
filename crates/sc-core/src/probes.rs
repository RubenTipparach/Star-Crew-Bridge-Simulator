//! Light probes for things that move (openspec/changes/light-baking, design sections 6 and 16): ambient cubes on a
//! grid in each compartment's air, baked with the walls' light, and the rule that lights a body from the ones around
//! it. It lives in the core because which probes light a body, and how they blend, is a rule the client applies every
//! frame and the tests check without a GPU; the deck file (`crate::deck`) carries the probes.
//!
//! A probe is `PROBE_BYTES` bytes: six axes (+X, -X, +Y, -Y, +Z, -Z), each three lighting states (normal, red
//! alert, emergency), each RGB, as display multipliers 0-2 in a byte (the deck's vertex colours' encoding,
//! light-baking design 5); then a byte that is 1 when the probe is valid (not inside a prop, not outside the air).

/// Bytes a probe takes in the deck file: 54 of colour and one valid flag.
pub const PROBE_BYTES: usize = 55;

/// An ambient cube for one lighting blend: the display multiplier (0-2) of a surface facing +X, -X, +Y, -Y, +Z and
/// -Z, in that order.
pub type Cube = [[f32; 3]; 6];

/// One compartment's probes, as the deck file holds them.
#[derive(Debug, Clone, Copy)]
pub struct ProbeGrid<'a> {
    /// The first grid point, ship coordinates, metres.
    pub origin_m: [f64; 3],
    /// The distance between neighbouring points, metres.
    pub spacing_m: f32,
    /// Points on the x, y and z axes.
    pub dims: [u32; 3],
    /// The probes, `PROBE_BYTES` each, x fastest, then y, then z.
    pub data: &'a [u8],
}

impl ProbeGrid<'_> {
    /// True when `p` (ship coordinates, metres) is inside the grid's box grown by half a spacing on every side.
    pub fn holds(&self, p: [f64; 3]) -> bool {
        let half = f64::from(self.spacing_m) * 0.5;
        (0..3).all(|k| {
            let hi = self.origin_m[k] + f64::from(self.dims[k].saturating_sub(1)) * f64::from(self.spacing_m);
            p[k] >= self.origin_m[k] - half && p[k] <= hi + half
        })
    }

    /// The cube at `p`, blended over the eight probes around it with trilinear weights, invalid probes skipped and
    /// the weights renormalised, and over the three lighting states by `state_weights` (normal, red alert,
    /// emergency). Returns the cube and the share of the trilinear weight that was valid (0-1), or None when no
    /// probe around `p` is valid or `p` is outside the grid.
    pub fn sample(&self, p: [f64; 3], state_weights: [f32; 3]) -> Option<(Cube, f32)> {
        if !self.holds(p) || self.dims.contains(&0) {
            return None;
        }
        let n = self.dims.iter().map(|&d| d as usize).product::<usize>();
        if self.data.len() < n * PROBE_BYTES {
            return None;
        }
        let mut i0 = [0usize; 3];
        let mut f = [0f32; 3];
        for k in 0..3 {
            let t = ((p[k] - self.origin_m[k]) / f64::from(self.spacing_m)).clamp(0.0, f64::from(self.dims[k] - 1));
            i0[k] = (t.floor() as usize).min(self.dims[k] as usize - 1);
            f[k] = (t - i0[k] as f64) as f32;
        }
        let (dx, dy) = (self.dims[0] as usize, self.dims[1] as usize);
        let mut cube = [[0f32; 3]; 6];
        let mut total = 0f32;
        for corner in 0..8 {
            let o = [corner & 1, (corner >> 1) & 1, (corner >> 2) & 1];
            let mut w = 1f32;
            let mut idx = [0usize; 3];
            for k in 0..3 {
                idx[k] = (i0[k] + o[k]).min(self.dims[k] as usize - 1);
                w *= if o[k] == 1 { f[k] } else { 1.0 - f[k] };
            }
            if w <= 0.0 {
                continue;
            }
            let at = (idx[0] + dx * (idx[1] + dy * idx[2])) * PROBE_BYTES;
            let probe = &self.data[at..at + PROBE_BYTES];
            if probe[54] != 1 {
                continue;
            }
            for (axis, out) in cube.iter_mut().enumerate() {
                for (c, v) in out.iter_mut().enumerate() {
                    let s: f32 = (0..3).map(|st| f32::from(probe[axis * 9 + st * 3 + c]) * state_weights[st]).sum();
                    *v += w * s * (2.0 / 255.0);
                }
            }
            total += w;
        }
        if total <= 1e-6 {
            return None;
        }
        for v in cube.iter_mut().flatten() {
            *v /= total;
        }
        Some((cube, total))
    }
}

/// The cube at `p` from whichever grid holds it with the most valid weight around it (rooms' boxes overlap at
/// their walls), or None when no grid has a valid probe there.
pub fn sample_ship(grids: &[ProbeGrid<'_>], p: [f64; 3], state_weights: [f32; 3]) -> Option<Cube> {
    grids
        .iter()
        .filter_map(|g| g.sample(p, state_weights))
        .fold(None, |best: Option<(Cube, f32)>, s| match best {
            Some(b) if b.1 >= s.1 => Some(b),
            _ => Some(s),
        })
        .map(|(c, _)| c)
}

/// Every compartment's probes, owned: what a client keeps after the deck file's bytes are gone.
#[derive(Debug, Clone, Default)]
pub struct ProbeSet {
    grids: Vec<OwnedGrid>,
    data: Vec<u8>,
}

/// A grid in a `ProbeSet`: its origin (ship coordinates, metres), spacing (metres), dims, and its bytes in the set.
#[derive(Debug, Clone)]
struct OwnedGrid {
    origin_m: [f64; 3],
    spacing_m: f32,
    dims: [u32; 3],
    bytes: std::ops::Range<usize>,
}

impl ProbeSet {
    /// The probe sets of `grids`, copied.
    pub fn new(grids: &[ProbeGrid<'_>]) -> Self {
        let mut set = Self::default();
        for g in grids {
            let at = set.data.len();
            set.data.extend_from_slice(g.data);
            set.grids.push(OwnedGrid {
                origin_m: g.origin_m,
                spacing_m: g.spacing_m,
                dims: g.dims,
                bytes: at..set.data.len(),
            });
        }
        set
    }
    /// Grids held.
    pub fn len(&self) -> usize {
        self.grids.len()
    }
    /// True when it holds no grid.
    pub fn is_empty(&self) -> bool {
        self.grids.is_empty()
    }
    /// The cube at `p` (`sample_ship` over every grid).
    pub fn sample(&self, p: [f64; 3], state_weights: [f32; 3]) -> Option<Cube> {
        let grids: Vec<ProbeGrid<'_>> = self
            .grids
            .iter()
            .map(|g| ProbeGrid {
                origin_m: g.origin_m,
                spacing_m: g.spacing_m,
                dims: g.dims,
                data: &self.data[g.bytes.clone()],
            })
            .collect();
        sample_ship(&grids, p, state_weights)
    }
}

/// The light the figures had before probes (crew-npcs 7), as a cube: a key from above and the front,
/// `0.45 + 0.65 max(n . l, 0)` along each axis with `l = (0.3, 0.8, 0.52)`, times two, as a display multiplier. A
/// body with no valid probe around it, and the ship map's figures, take it.
pub fn fallback_cube() -> Cube {
    const L: [f32; 3] = [0.3, 0.8, 0.52];
    let axes: [[f32; 3]; 6] =
        [[1.0, 0.0, 0.0], [-1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, -1.0, 0.0], [0.0, 0.0, 1.0], [0.0, 0.0, -1.0]];
    axes.map(|n| {
        let k = 0.45 + 0.65 * (n[0] * L[0] + n[1] * L[1] + n[2] * L[2]).max(0.0);
        let d = (2.0 * k).powf(1.0 / 2.2).min(2.0);
        [d; 3]
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A probe of one grey level (byte) in every axis and state, valid or not.
    fn probe(level: u8, valid: bool) -> Vec<u8> {
        let mut p = vec![level; 54];
        p.push(u8::from(valid));
        p
    }

    fn grid(data: &[u8], dims: [u32; 3]) -> ProbeGrid<'_> {
        ProbeGrid { origin_m: [0.0, 0.0, 0.0], spacing_m: 1.0, dims, data }
    }

    #[test]
    fn halfway_between_two_probes_takes_their_mean() {
        let data = [probe(0, true), probe(255, true)].concat();
        let (cube, valid) = grid(&data, [2, 1, 1]).sample([0.5, 0.0, 0.0], [1.0, 0.0, 0.0]).unwrap();
        assert!((cube[0][0] - 1.0).abs() < 1e-4, "byte 0 is 0x and byte 255 is 2x: halfway is 1x, got {}", cube[0][0]);
        assert!((valid - 1.0).abs() < 1e-6);
    }

    #[test]
    fn an_invalid_probe_is_skipped_and_its_neighbour_fills_in() {
        let data = [probe(255, false), probe(51, true)].concat();
        let (cube, valid) = grid(&data, [2, 1, 1]).sample([0.25, 0.0, 0.0], [1.0, 0.0, 0.0]).unwrap();
        assert!((cube[2][1] - 0.4).abs() < 1e-4, "only the valid probe (byte 51 = 0.4x) lights it: {}", cube[2][1]);
        assert!((valid - 0.25).abs() < 1e-6, "a quarter of the weight was valid");
    }

    #[test]
    fn no_valid_probe_or_outside_the_grid_gives_none() {
        let data = probe(255, false);
        assert!(grid(&data, [1, 1, 1]).sample([0.0, 0.0, 0.0], [1.0, 0.0, 0.0]).is_none());
        let ok = probe(255, true);
        assert!(
            grid(&ok, [1, 1, 1]).sample([0.6, 0.0, 0.0], [1.0, 0.0, 0.0]).is_none(),
            "past half a spacing is outside"
        );
        assert!(grid(&ok, [1, 1, 1]).sample([0.4, 0.0, 0.0], [1.0, 0.0, 0.0]).is_some(), "within half a spacing is in");
    }

    #[test]
    fn the_states_blend_by_their_weights() {
        let mut p = vec![0u8; 55];
        p[54] = 1;
        for axis in 0..6 {
            p[axis * 9 + 3] = 255; // red alert's red channel only
        }
        let (cube, _) = grid(&p, [1, 1, 1]).sample([0.0, 0.0, 0.0], [0.5, 0.5, 0.0]).unwrap();
        assert!((cube[4][0] - 1.0).abs() < 1e-4, "half red alert at 2x is 1x red: {}", cube[4][0]);
        assert_eq!(cube[4][1], 0.0);
    }

    #[test]
    fn the_ship_takes_the_grid_with_more_valid_light_around_the_point() {
        let a = [probe(0, false), probe(255, true)].concat();
        let b = [probe(102, true), probe(102, true)].concat();
        let ga = grid(&a, [2, 1, 1]);
        let gb = grid(&b, [2, 1, 1]);
        let c = sample_ship(&[ga, gb], [0.25, 0.0, 0.0], [1.0, 0.0, 0.0]).unwrap();
        assert!((c[0][0] - 0.8).abs() < 1e-4, "grid b is all valid there and wins: {}", c[0][0]);
    }

    #[test]
    fn the_fallback_lights_the_top_more_than_the_bottom() {
        let c = fallback_cube();
        assert!(c[2][0] > c[3][0], "a key from above: +Y brighter than -Y");
        assert!(c.iter().flatten().all(|v| (0.0..=2.0).contains(v)));
    }
}
