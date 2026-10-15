//! The ship's hull as the client draws it (openspec/changes/tern-exterior, design 4): the exterior model `deckc` packed
//! into the deck file, one mesh in the ship's frame, drawn part by part after the rooms with the deck's own programs.
//!
//! It lives in the client because it is what the walk and the drill's bridge both draw; one loader and one draw, so the
//! two views cannot show the hull differently (CLAUDE.md 6.1). The model's vertex colours carry the exterior build's
//! baked sun; the draw moves that light to the frame's sun (patrol-mission design 11.3), so the hull seen from a window
//! is lit from where the sky's sun is, however the Tern has turned.

use glam::{Mat4, Vec3};
use sc_core::deck::Deck;
use sc_render::{DeckParams, DeckProgram, Indices, Mesh, Renderer, Target, TextureArray, NO_RELIGHT};

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
        let arrays =
            h.arrays.iter().map(|a| r.make_texture_array_mips(a.size_px, a.layers, &d.hull_array_levels(a))).collect();
        let parts = h.parts.iter().map(|p| (p.first as usize, p.count as usize, p.array.map(|a| a as usize))).collect();
        Ok(Some(Self { mesh, origin: h.origin_m, scale: h.scale, parts, arrays, bake, triangles: idx.len() / 3 }))
    }

    /// Draw the hull into the current 3D pass of `t`, seen through `proj_view` (a view at the origin) from `eye`, ship
    /// coordinates, lit by the sun at `sun` (the ship's frame, unit length: `outside::sun_in_ship`). The outside keeps its
    /// own light: the interior's alert states and the panels' glow do not reach it.
    pub fn draw(&self, r: &mut Renderer, t: &Target, proj_view: Mat4, eye: [f64; 3], sun: Vec3) {
        self.draw_lit(r, t, proj_view, eye, sun, 1.0);
    }

    /// As `draw`, lit from `light` (the ship's frame, unit length) at `strength` (1 the sun's): a hull under a
    /// station's roof is lit by its floods from above, not the star (patrol-mission design 12b).
    pub fn draw_lit(&self, r: &mut Renderer, t: &Target, proj_view: Mat4, eye: [f64; 3], light: Vec3, strength: f32) {
        // The frames rule: subtract the eye in f64, then narrow to f32.
        let rel = Vec3::new(
            (self.origin[0] - eye[0]) as f32,
            (self.origin[1] - eye[1]) as f32,
            (self.origin[2] - eye[2]) as f32,
        );
        self.draw_at(r, t, proj_view * Mat4::from_translation(rel), light, strength);
    }

    /// Another ship of the Tern's model (a sister ship at a berth, patrol-mission design 12b): its frame placed by `model`
    /// (from the ship's frame to the frame `vp` is in, relative to the eye), lit from `light` in its own frame at
    /// `strength`.
    pub fn draw_posed(&self, r: &mut Renderer, t: &Target, vp: Mat4, model: Mat4, light: Vec3, strength: f32) {
        let o = Vec3::new(self.origin[0] as f32, self.origin[1] as f32, self.origin[2] as f32);
        self.draw_at(r, t, vp * model * Mat4::from_translation(o), light, strength);
    }

    /// The parts, the mesh's origin placed by `at` (projection, view and placement).
    fn draw_at(&self, r: &mut Renderer, t: &Target, at: Mat4, light: Vec3, strength: f32) {
        let p = DeckParams {
            mvp: at * Mat4::from_scale(Vec3::splat(self.scale)),
            state_weights: [1.0, 0.0, 0.0],
            flash_dir: light,
            flash: strength,
            panel_first: u32::MAX,
            panel_glow: 0.0,
            clip_y: f32::MAX,
            relight: self.bake.relight(),
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
