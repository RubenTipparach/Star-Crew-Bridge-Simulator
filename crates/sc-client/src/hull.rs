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
        self.draw_lit(r, t, proj_view, eye, sun, 1.0, Some(Quat::IDENTITY));
    }

    /// As `draw`, lit from `light` (the ship's frame, unit length) at `strength` (1 the sun's): a hull under a
    /// station's roof is lit by its floods from above, not the star (patrol-mission design 12b). `sky`: the ship's
    /// attitude (ship frame to the sky's) when the sky's colour tints its shade (nebula-skies design 6), none under a
    /// roof.
    #[allow(clippy::too_many_arguments)]
    pub fn draw_lit(
        &self,
        r: &mut Renderer,
        t: &Target,
        proj_view: Mat4,
        eye: [f64; 3],
        light: Vec3,
        strength: f32,
        sky: Option<Quat>,
    ) {
        // The frames rule: subtract the eye in f64, then narrow to f32.
        let rel = Vec3::new(
            (self.origin[0] - eye[0]) as f32,
            (self.origin[1] - eye[1]) as f32,
            (self.origin[2] - eye[2]) as f32,
        );
        let tint = sky.map_or(NO_TINT, |q| crate::outside::sky_tint(r, q));
        self.draw_at(r, t, proj_view * Mat4::from_translation(rel), light, strength, &tint);
    }

    /// Another ship of the Tern's model (a sister ship at a berth, patrol-mission design 12b): its frame placed by `model`
    /// (from the ship's frame to the frame `vp` is in, relative to the eye), lit from `light` in its own frame at
    /// `strength`. A berth is under the station's roof: no sky's tint.
    pub fn draw_posed(&self, r: &mut Renderer, t: &Target, vp: Mat4, model: Mat4, light: Vec3, strength: f32) {
        let o = Vec3::new(self.origin[0] as f32, self.origin[1] as f32, self.origin[2] as f32);
        self.draw_at(r, t, vp * model * Mat4::from_translation(o), light, strength, &NO_TINT);
    }

    /// The parts, the mesh's origin placed by `at` (projection, view and placement), the bake's ambient tinted by `tint`.
    fn draw_at(&self, r: &mut Renderer, t: &Target, at: Mat4, light: Vec3, strength: f32, tint: &[[f32; 3]; 6]) {
        let p = DeckParams {
            mvp: at * Mat4::from_scale(Vec3::splat(self.scale)),
            state_weights: [1.0, 0.0, 0.0],
            flash_dir: light,
            flash: strength,
            panel_first: u32::MAX,
            panel_glow: 0.0,
            clip_y: f32::MAX,
            relight: Relight { baked: self.bake.relight(), tint: *tint },
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
    use super::parse_smooth_materials;

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
