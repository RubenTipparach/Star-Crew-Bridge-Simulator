//! The ship's hull as the client draws it (openspec/changes/tern-exterior, design 4): the exterior model `deckc` packed
//! into the deck file, one mesh in the ship's frame, drawn part by part after the rooms with the deck's own programs.
//!
//! It lives in the client because it is what the walk and the drill's bridge both draw; one loader and one draw, so the
//! two views cannot show the hull differently (CLAUDE.md 6.1). The model's vertex colours carry the exterior build's
//! baked sun; the draw moves that light to the frame's sun (patrol-mission design 11.3), so the hull seen from a window
//! is lit from where the sky's sun is, however the Tern has turned.

use glam::{Mat4, Quat, Vec3};
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
}

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
        let parts = h.parts.iter().map(|p| (p.first as usize, p.count as usize, p.array.map(|a| a as usize))).collect();
        Ok(Some(Self { mesh, origin: h.origin_m, scale: h.scale, parts, arrays, bake, triangles: idx.len() / 3 }))
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

/// How a hull is lit (patrol-mission design 11.3 and 12b; spacedock design 8b). The deck program's relight divides the
/// exterior bake's light out of the hull's colours and multiplies the frame's in: `a sky(n) + (1 - a) max(0, n . sun)
/// strength`, the bake's ambient share `a` taking an ambient cube (`relight`'s tint). A bay's probes ride in that cube
/// with the star off, so the hull takes `cube(n)`; in a station's throat the two are blended, `(1 - f) cube(n) + f
/// star(n)`, which the shader's light term carries exactly since it is linear in the tint and the sun's strength.
#[derive(Debug, Clone, Copy, PartialEq)]
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
}

impl HullLight {
    /// The star at `sun` (the hull's frame), its shade tinted by the sky when `sky` gives the hull's attitude.
    pub fn star(r: &Renderer, sun: Vec3, sky: Option<Quat>) -> Self {
        let tint = sky.map_or(NO_TINT, |q| crate::outside::sky_tint(r, q));
        Self { sun, strength: 1.0, tint, cube: None, star_share: 1.0 }
    }

    /// The stand-in under a roof with no probes: light from `way` (the hull's frame) at `strength`, no sky.
    pub fn from_way(way: Vec3, strength: f32) -> Self {
        Self { sun: way, strength, tint: NO_TINT, cube: None, star_share: 1.0 }
    }

    /// The probes' `cube` (the hull's frame), the star's light `star` taking `star_share` of it (0-1).
    pub fn probes(cube: [[f32; 3]; 6], star: Self, star_share: f32) -> Self {
        Self { cube: Some(cube), star_share: star_share.clamp(0.0, 1.0), ..star }
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
    use super::{parse_smooth_materials, HullLight};
    use glam::Vec3;

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
            let in_bay = relit(&HullLight::probes(cube, star, 0.0), a, baked_sun, n) * baked;
            assert!((in_bay - want_cube).abs() < 1e-5, "in the bay a face takes the cube's light alone: {n}");
            let half = relit(&HullLight::probes(cube, star, 0.5), a, baked_sun, n) * baked;
            assert!((half - (want_cube + want_star) / 2.0).abs() < 1e-5, "halfway down the throat, half of each: {n}");
            let out = relit(&HullLight::probes(cube, star, 1.0), a, baked_sun, n) * baked;
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
