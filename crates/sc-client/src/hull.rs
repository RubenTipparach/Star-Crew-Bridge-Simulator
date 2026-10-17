//! The ship's hull as the client draws it (openspec/changes/tern-exterior, design 4): the exterior model `deckc` packed
//! into the deck file, one mesh in the ship's frame, drawn part by part after the rooms with the deck's own programs.
//!
//! It lives in the client because it is what the walk and the drill's bridge both draw; one loader and one draw, so the
//! two views cannot show the hull differently (CLAUDE.md 6.1). The model's vertex colours carry the exterior build's
//! baked sun; the draw moves that light to the frame's sun (patrol-mission design 11.3), so the hull seen from a window
//! is lit from where the sky's sun is, however the Tern has turned. Inside a station's bay the hull is lit at each
//! vertex from the bay's light probes instead (spacedock design 8b): a copy of its mesh whose colours carry that light,
//! made for each place it stands and kept while it stands there.

use std::cell::RefCell;
use std::rc::Rc;

use glam::{DVec3, Mat4, Quat, Vec3};
use sc_core::deck::Deck;
use sc_render::{
    DeckParams, DeckProgram, Indices, Magnify, Mesh, Relight, Renderer, Target, TextureArray, NO_RELIGHT, NO_TINT,
};
use serde::Deserialize;

use crate::outside::HullBake;

/// The hull, ready to draw.
pub struct HullView {
    mesh: Mesh,
    /// The mesh frame's origin, ship coordinates, metres.
    origin: [f64; 3],
    /// What a stored position is multiplied by to give metres.
    scale: f32,
    /// Each part: its first index, its index count and the array it samples (None: it glows, its colour its light).
    parts: Vec<(usize, usize, Option<usize>)>,
    arrays: Vec<TextureArray>,
    /// The light baked into its colours.
    bake: HullBake,
    /// Its triangles.
    pub triangles: usize,
    /// Its vertices and indices as packed, for the probe-lit copies.
    vertices: Vec<u8>,
    indices: Vec<u32>,
    /// For each vertex, whether a glowing part uses it (its own light, never relit).
    glowing: Vec<bool>,
    /// The probe-lit copies, one for each place a hull stands in a bay, the most recently drawn last.
    lit: RefCell<Vec<(LitKey, Mesh)>>,
}

/// Where a probe-lit copy was lit: the probe set (its address), the hull's origin to the metre, its turn to a hundredth
/// and the star's share of its light to a twentieth.
type LitKey = (usize, [i64; 3], [i32; 4], i32);

/// Probe-lit copies kept at once: the Tern and the docked ships of a bay, with room to spare.
const LIT_COPIES: usize = 12;

impl HullView {
    /// The hull of the deck `d`, or None when the ship has none; an error when its bake's data does not load.
    pub fn load(r: &Renderer, d: &Deck<'_>) -> Result<Option<Self>, String> {
        let Some(h) = d.index.hull.as_ref() else { return Ok(None) };
        let bake = HullBake::load()?;
        let idx = d.hull_indices();
        let mesh = r.make_mesh(d.hull_vertices(), Indices::U32(&idx));
        // A painted UV atlas is magnified bilinear: from the bridge's windows the bow's paint (about 12 px a metre) is
        // a metre or two off, and nearest drew each texel as a block (tern-exterior design 2).
        let smooth = h.smooth_arrays(&smooth_materials(&d.index.ship)?)?;
        let arrays = h
            .arrays
            .iter()
            .zip(smooth)
            .map(|(a, smooth)| {
                let t = r.make_texture_array_mips(a.size_px, a.layers, &d.hull_array_levels(a));
                if smooth {
                    t.magnified(Magnify::Linear)
                } else {
                    t
                }
            })
            .collect();
        let parts: Vec<(usize, usize, Option<usize>)> =
            h.parts.iter().map(|p| (p.first as usize, p.count as usize, p.array.map(|a| a as usize))).collect();
        let vertices = d.hull_vertices().to_vec();
        let mut glowing = vec![false; vertices.len() / sc_core::vertex::DECK_VERTEX_BYTES];
        for &(first, count, array) in &parts {
            if array.is_none() {
                for &i in &idx[first..first + count] {
                    glowing[i as usize] = true;
                }
            }
        }
        Ok(Some(Self {
            mesh,
            origin: h.origin_m,
            scale: h.scale,
            parts,
            arrays,
            bake,
            triangles: idx.len() / 3,
            vertices,
            indices: idx,
            glowing,
            lit: RefCell::new(Vec::new()),
        }))
    }

    /// The probe-lit copy for `bay`, made now if this place has none (the least recently drawn copy giving way).
    fn lit_mesh<'a>(
        &self,
        r: &Renderer,
        light: &HullLight,
        bay: &ProbeLight,
        lit: &'a mut Vec<(LitKey, Mesh)>,
    ) -> &'a Mesh {
        let q = bay.turn.normalize();
        let key: LitKey = (
            Rc::as_ptr(&bay.probes) as usize,
            bay.at.to_array().map(|x| x.round() as i64),
            q.to_array().map(|x| (x * 100.0).round() as i32),
            (light.star_share * 20.0).round() as i32,
        );
        if let Some(k) = lit.iter().position(|(k, _)| *k == key) {
            let e = lit.remove(k);
            lit.push(e);
        } else {
            if lit.len() >= LIT_COPIES {
                lit.remove(0);
            }
            let v = probe_lit(&self.vertices, &self.glowing, (self.origin, self.scale), self.bake, bay, q, light);
            lit.push((key, r.make_mesh(&v, Indices::U32(&self.indices))));
        }
        &lit.last().expect("a copy was just placed").1
    }

    /// Draw the hull into the current 3D pass of `t`, seen through `proj_view` (a view at the origin) from `eye`, ship
    /// coordinates, lit by the sun at `sun` (the ship's frame, unit length: `outside::sun_in_ship`). The outside keeps its
    /// own light: the interior's alert states and the panels' glow do not reach it. The ship's axes are the sky's here
    /// (the walk's dock), so the sky's tint reaches it unturned.
    pub fn draw(&self, r: &mut Renderer, t: &Target, proj_view: Mat4, eye: [f64; 3], sun: Vec3) {
        let light = HullLight::star(r, sun, Some(Quat::IDENTITY));
        self.draw_lit(r, t, proj_view, eye, &light);
    }

    /// As `draw`, lit by `light`: the star's, or under a station's roof its bay's (patrol-mission design 12b, spacedock
    /// design 8b).
    pub fn draw_lit(&self, r: &mut Renderer, t: &Target, proj_view: Mat4, eye: [f64; 3], light: &HullLight) {
        // The frames rule: subtract the eye in f64, then narrow to f32.
        let rel = Vec3::new(
            (self.origin[0] - eye[0]) as f32,
            (self.origin[1] - eye[1]) as f32,
            (self.origin[2] - eye[2]) as f32,
        );
        self.draw_at(r, t, proj_view * Mat4::from_translation(rel), light);
    }

    /// Another ship of the Tern's model (a sister ship at a berth, patrol-mission design 12b): its frame placed by `model`
    /// (from the ship's frame to the frame `vp` is in, relative to the eye), lit by `light` in its own frame.
    pub fn draw_posed(&self, r: &mut Renderer, t: &Target, vp: Mat4, model: Mat4, light: &HullLight) {
        let o = Vec3::new(self.origin[0] as f32, self.origin[1] as f32, self.origin[2] as f32);
        self.draw_at(r, t, vp * model * Mat4::from_translation(o), light);
    }

    /// The parts, the mesh's origin placed by `at` (projection, view and placement), lit by `light`.
    fn draw_at(&self, r: &mut Renderer, t: &Target, at: Mat4, light: &HullLight) {
        // In a bay or its throat, the probe-lit copy, drawn as its colours are.
        if let Some(bay) = light.bay.as_ref() {
            let mut lit = self.lit.borrow_mut();
            let mesh = self.lit_mesh(r, light, bay, &mut lit);
            let p = DeckParams {
                mvp: at * Mat4::from_scale(Vec3::splat(self.scale)),
                state_weights: [1.0, 0.0, 0.0],
                flash_dir: Vec3::Z,
                flash: 0.0,
                panel_first: u32::MAX,
                panel_glow: 0.0,
                clip_y: f32::MAX,
                relight: NO_RELIGHT,
            };
            for &(first, count, array) in &self.parts {
                match array {
                    Some(a) => {
                        r.draw_deck_range(t, mesh, DeckProgram::Textured, &p, Some(&self.arrays[a]), first, count)
                    }
                    None => r.draw_deck_range(t, mesh, DeckProgram::Flat, &p, None, first, count),
                }
            }
            return;
        }
        let (tint, strength) = light.terms(self.bake.ambient);
        let p = DeckParams {
            mvp: at * Mat4::from_scale(Vec3::splat(self.scale)),
            state_weights: [1.0, 0.0, 0.0],
            flash_dir: light.sun,
            flash: strength,
            panel_first: u32::MAX,
            panel_glow: 0.0,
            clip_y: f32::MAX,
            relight: Relight { baked: self.bake.relight(), tint },
        };
        // The glowing parts (lights, screens) are their own light: not relit, no light added.
        let glowing = DeckParams { flash: 0.0, relight: NO_RELIGHT, ..p };
        for &(first, count, array) in &self.parts {
            match array {
                Some(a) => {
                    r.draw_deck_range(t, &self.mesh, DeckProgram::Textured, &p, Some(&self.arrays[a]), first, count)
                }
                None => r.draw_deck_range(t, &self.mesh, DeckProgram::Flat, &glowing, None, first, count),
            }
        }
    }
}

/// `vertices`' colours (a hull's packed vertices, its mesh frame at `frame`: origin in the ship's frame and scale) lit
/// from `bay`'s probes at each vertex, the hull's frame turned by `turn` into the probes': each colour's exterior bake
/// (`bake`) divided out and the probes' cube there multiplied in, each axis held under `bay.max`, blended with `star`'s
/// light by its `star_share` (down a throat); the `glowing` ones left as they are. A vertex with no valid probe round it
/// takes the cube at the hull's middle; none there, its bake as it is.
fn probe_lit(
    vertices: &[u8],
    glowing: &[bool],
    frame: ([f64; 3], f32),
    bake: HullBake,
    bay: &ProbeLight,
    turn: Quat,
    star: &HullLight,
) -> Vec<u8> {
    let f = star.star_share.clamp(0.0, 1.0);
    let grids = bay.probes.grids();
    let mid = sc_core::probes::sample_ship(&grids, bay.at.to_array(), bay.state);
    let (a, s) = (bake.ambient, bake.sun_ship);
    let o = DVec3::from_array(frame.0);
    let mut out = vertices.to_vec();
    for (i, v) in out.chunks_exact_mut(sc_core::vertex::DECK_VERTEX_BYTES).enumerate() {
        if glowing.get(i).copied().unwrap_or(false) {
            continue;
        }
        let (p, n) = sc_core::vertex::unpack_position_normal(v);
        let n = Vec3::from(n).normalize_or_zero();
        let p_bay = bay.at + turn.as_dquat() * (o + Vec3::from(p).as_dvec3() * f64::from(frame.1));
        let Some(cube) = sc_core::probes::sample_ship(&grids, p_bay.to_array(), bay.state).or(mid) else { continue };
        let probe = crate::outside::cube_at(&cube, turn * n).map(|c| c.min(bay.max));
        // The star's, as the shader's relight gives it: the ambient share in the sky's tint, the rest from the star.
        let sky = crate::outside::cube_at(&star.tint, n);
        let lit_by_star = (1.0 - a) * n.dot(star.sun).max(0.0) * star.strength;
        let light: [f32; 3] = std::array::from_fn(|c| (1.0 - f) * probe[c] + f * (a * sky[c] + lit_by_star));
        let baked = a + (1.0 - a) * n.dot(s).max(0.0);
        for set in 0..3 {
            for c in 0..3 {
                let b = &mut v[12 + set * 4 + c];
                *b = (f32::from(*b) * light[c] / baked).round().clamp(0.0, 255.0) as u8;
            }
        }
    }
    out
}

/// How a hull is lit (patrol-mission design 11.3 and 12b; spacedock design 8b). The deck program's relight divides the
/// exterior bake's light out of the hull's colours and multiplies the frame's in: `a sky(n) + (1 - a) max(0, n . sun)
/// strength`, the bake's ambient share `a` taking an ambient cube (`relight`'s tint). A bay's probes ride in that cube
/// with the star off, so the hull takes `cube(n)`; in a station's throat the two are blended, `(1 - f) cube(n) + f
/// star(n)`, which the shader's light term carries exactly since it is linear in the tint and the sun's strength. With
/// `bay`, the hull is lit at each vertex from the bay's probes instead, the star's share blended in the same way
/// (`HullView`'s probe-lit copies).
#[derive(Debug, Clone)]
pub struct HullLight {
    /// The way to the star (or the stand-in's light), the hull's frame, unit length.
    pub sun: Vec3,
    /// Its strength (1 the star's).
    pub strength: f32,
    /// The sky's tint on the bake's ambient share (`sc_render::NO_TINT` for none).
    pub tint: [[f32; 3]; 6],
    /// A bay's probe cube in the hull's frame (display multipliers), when the bay lights it.
    pub cube: Option<[[f32; 3]; 6]>,
    /// With a cube, the star's share of the light, 0-1 (0 in the bay, rising to 1 down the throat).
    pub star_share: f32,
    /// In a bay, its probes and where the hull stands among them: it is lit at each vertex (the cube is then the one
    /// at its middle).
    pub bay: Option<ProbeLight>,
}

/// A bay's light probes and a hull's place among them (spacedock design 8b).
#[derive(Debug, Clone)]
pub struct ProbeLight {
    /// The bay's probes, the station's frame.
    pub probes: Rc<sc_core::probes::ProbeSet>,
    /// The hull's frame's origin, the station's frame, metres.
    pub at: DVec3,
    /// The hull's frame turned into the station's.
    pub turn: Quat,
    /// The most light a probe gives a hull, each axis (display multiplier).
    pub max: f32,
    /// The station's lighting state's weights.
    pub state: [f32; 3],
}

impl HullLight {
    /// The star at `sun` (the hull's frame), its shade tinted by the sky when `sky` gives the hull's attitude.
    pub fn star(r: &Renderer, sun: Vec3, sky: Option<Quat>) -> Self {
        let tint = sky.map_or(NO_TINT, |q| crate::outside::sky_tint(r, q));
        Self { sun, strength: 1.0, tint, cube: None, star_share: 1.0, bay: None }
    }

    /// The stand-in under a roof with no probes: light from `way` (the hull's frame) at `strength`, no sky.
    pub fn from_way(way: Vec3, strength: f32) -> Self {
        Self { sun: way, strength, tint: NO_TINT, cube: None, star_share: 1.0, bay: None }
    }

    /// The probes' `cube` (the hull's frame), the star's light `star` taking `star_share` of it (0-1).
    pub fn probes(cube: [[f32; 3]; 6], star: Self, star_share: f32) -> Self {
        Self { cube: Some(cube), star_share: star_share.clamp(0.0, 1.0), ..star }
    }

    /// As `probes`, the hull lit at each vertex from `bay`.
    pub fn with_bay(self, bay: ProbeLight) -> Self {
        Self { bay: Some(bay), ..self }
    }

    /// The relight's tint and the sun's strength for a bake whose ambient share is `ambient` (above 0).
    pub fn terms(&self, ambient: f32) -> ([[f32; 3]; 6], f32) {
        let Some(cube) = self.cube else { return (self.tint, self.strength) };
        let f = self.star_share;
        let tint =
            std::array::from_fn(|k| std::array::from_fn(|c| (1.0 - f) * cube[k][c] / ambient + f * self.tint[k][c]));
        (tint, f * self.strength)
    }
}

/// The field of a ship in `data/ships/exteriors.json` the hull's sampling needs; the rest is the exterior build's.
#[derive(Deserialize)]
struct ShipFields {
    id: String,
    /// The materials painted on a UV atlas, magnified bilinear; none when absent.
    #[serde(default)]
    smooth_magnify_materials: Vec<String>,
}

#[derive(Deserialize)]
struct ExteriorShips {
    ships: Vec<ShipFields>,
}

/// Ship `ship`'s materials magnified bilinear, from the text of `data/ships/exteriors.json` (`path` names it in an
/// error); none for a ship the file does not list.
fn parse_smooth_materials(path: &str, text: &str, ship: &str) -> Result<Vec<String>, String> {
    let f: ExteriorShips = serde_json::from_str(text).map_err(|e| format!("{path}: {e}"))?;
    Ok(f.ships.into_iter().find(|s| s.id == ship).map(|s| s.smooth_magnify_materials).unwrap_or_default())
}

/// As [`parse_smooth_materials`], from `data/ships/exteriors.json` in the working directory.
fn smooth_materials(ship: &str) -> Result<Vec<String>, String> {
    let path = "data/ships/exteriors.json";
    let text = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
    parse_smooth_materials(path, &text, ship)
}

#[cfg(test)]
mod tests {
    use super::{parse_smooth_materials, probe_lit, HullLight, ProbeLight};
    use crate::outside::HullBake;
    use glam::{DVec3, Quat, Vec3};

    #[test]
    fn in_a_bay_each_vertex_of_a_hull_takes_the_probes_light_where_it_stands() {
        // Two probes 20 m apart along x: the first lit from above (its +Y face 1.6), the second dim (0.2 on every face).
        let mut data = vec![0u8; 2 * sc_core::probes::PROBE_BYTES];
        for (k, probe) in data.chunks_exact_mut(sc_core::probes::PROBE_BYTES).enumerate() {
            for axis in 0..6 {
                let v: f32 = if k == 0 && axis == 2 { 1.6 } else { 0.2 };
                for i in 0..9 {
                    probe[axis * 9 + i] = (v / 2.0 * 255.0).round() as u8;
                }
            }
            probe[54] = 1;
        }
        let grid = sc_core::probes::ProbeGrid { origin_m: [0.0; 3], spacing_m: 20.0, dims: [2, 1, 1], data: &data };
        let probes = std::rc::Rc::new(sc_core::probes::ProbeSet::new(&[grid]));
        let bay = ProbeLight { probes, at: DVec3::ZERO, turn: Quat::IDENTITY, max: 1.24, state: [1.0, 0.0, 0.0] };
        // Three vertices facing up, baked at byte 100 under a bake whose light on an upward face is 1: at the lit probe,
        // at the dim one, and a glowing one.
        let vertex = |x: f32| {
            let v = sc_core::vertex::DeckVertexIn {
                position_m: [x, 0.0, 0.0],
                mover: 0,
                layer: 0,
                normal: [0.0, 1.0, 0.0],
                colors: [[100, 100, 100, 255]; 3],
                uv: [0.0, 0.0],
            };
            sc_core::vertex::pack_deck_vertex(&v).expect("packs")
        };
        let vs: Vec<u8> = [vertex(0.0), vertex(20.0), vertex(0.0)].concat();
        let bake = HullBake { sun_ship: Vec3::Y, ambient: 0.3 };
        let star = HullLight::from_way(Vec3::Y, 1.0);
        let out =
            probe_lit(&vs, &[false, false, true], ([0.0; 3], 1.0), bake, &bay, Quat::IDENTITY, &star_share(&star, 0.0));
        let byte = |i: usize| out[i * sc_core::vertex::DECK_VERTEX_BYTES + 12];
        assert_eq!(byte(0), 124, "under the lit probe: 100 times its light, held at 1.24");
        assert_eq!(byte(1), 20, "under the dim probe: 100 times 0.2");
        assert_eq!(byte(2), 100, "a glowing vertex keeps its own light");
        // Halfway down a throat, half the probes' light and half the star's (straight overhead: the bake's 1 again).
        let half =
            probe_lit(&vs, &[false, false, true], ([0.0; 3], 1.0), bake, &bay, Quat::IDENTITY, &star_share(&star, 0.5));
        assert_eq!(half[sc_core::vertex::DECK_VERTEX_BYTES + 12], 60, "the dim probe's 20 and the star's 100, halved");
    }

    fn star_share(l: &HullLight, f: f32) -> HullLight {
        HullLight { star_share: f, ..l.clone() }
    }

    /// The deck shader's relight factor for a face with normal `n` (deck.glsl's deck_vs, one channel): the frame's
    /// light over the bake's.
    fn relit(l: &HullLight, ambient: f32, baked_sun: Vec3, n: Vec3) -> f32 {
        let (tint, strength) = l.terms(ambient);
        let n2 = n * n;
        let pick = |v: f32, k: usize| if v >= 0.0 { tint[k][0] } else { tint[k + 1][0] };
        let sky = n2.x * pick(n.x, 0) + n2.y * pick(n.y, 2) + n2.z * pick(n.z, 4);
        let sun_now = n.dot(l.sun).max(0.0) * strength;
        let baked = ambient + (1.0 - ambient) * n.dot(baked_sun).max(0.0);
        (ambient * sky + (1.0 - ambient) * sun_now) / baked
    }

    #[test]
    fn a_hull_in_the_bay_takes_the_probes_cube_and_down_the_throat_the_star_takes_over() {
        let (a, baked_sun) = (0.3, Vec3::new(0.3, 0.8, 0.5).normalize());
        let mut cube = [[0.4f32; 3]; 6];
        cube[2] = [1.2; 3]; // a pool of light from above
        let star = HullLight::from_way(Vec3::X, 1.0);
        for n in [Vec3::Y, -Vec3::Y, Vec3::X, Vec3::new(0.6, 0.8, 0.0)] {
            let baked = a + (1.0 - a) * n.dot(baked_sun).max(0.0);
            let n2 = n * n;
            let want_cube = n2.x * if n.x >= 0.0 { cube[0][0] } else { cube[1][0] }
                + n2.y * if n.y >= 0.0 { cube[2][0] } else { cube[3][0] };
            let want_star = a + (1.0 - a) * n.dot(Vec3::X).max(0.0);
            let in_bay = relit(&HullLight::probes(cube, star.clone(), 0.0), a, baked_sun, n) * baked;
            assert!((in_bay - want_cube).abs() < 1e-5, "in the bay a face takes the cube's light alone: {n}");
            let half = relit(&HullLight::probes(cube, star.clone(), 0.5), a, baked_sun, n) * baked;
            assert!((half - (want_cube + want_star) / 2.0).abs() < 1e-5, "halfway down the throat, half of each: {n}");
            let out = relit(&HullLight::probes(cube, star.clone(), 1.0), a, baked_sun, n) * baked;
            assert!((out - want_star).abs() < 1e-5, "at the mouth, the star's alone: {n}");
        }
    }

    #[test]
    fn the_terns_painted_atlases_are_the_smooth_ones() {
        // The hull from the bridge's windows drew its paint as big blocks: the atlas's texels, sampled nearest.
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../data/ships/exteriors.json");
        let text = std::fs::read_to_string(path).expect("the exterior data reads");
        let smooth = parse_smooth_materials(path, &text, "tern").expect("it parses");
        for m in ["tern_hull", "tern_rim", "tern_nacelle"] {
            assert!(smooth.iter().any(|s| s == m), "{m} is painted on an atlas and is magnified bilinear");
        }
        for m in ["hazard", "accent"] {
            assert!(!smooth.iter().any(|s| s == m), "{m} is a 128 px tiling layer and keeps the texel look");
        }
        assert!(parse_smooth_materials(path, &text, "no-such-ship").expect("it parses").is_empty());
    }
}
