//! The ship's hull as the client draws it (openspec/changes/tern-exterior, design 4): the exterior model `deckc` packed
//! into the deck file, one mesh in the ship's frame, drawn part by part after the rooms with the deck's own programs.
//!
//! It lives in the client because it is what the walk and the drill's bridge both draw; one loader and one draw, so the
//! two views cannot show the hull differently (CLAUDE.md 6.1).

use glam::{Mat4, Vec3};
use sc_core::deck::Deck;
use sc_render::{DeckParams, DeckProgram, Indices, Mesh, Renderer, Target, TextureArray};

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
    /// Its triangles.
    pub triangles: usize,
}

impl HullView {
    /// The hull of the deck `d`, or None when the ship has none.
    pub fn load(r: &Renderer, d: &Deck<'_>) -> Option<Self> {
        let h = d.index.hull.as_ref()?;
        let idx = d.hull_indices();
        let mesh = r.make_mesh(d.hull_vertices(), Indices::U32(&idx));
        let arrays =
            h.arrays.iter().map(|a| r.make_texture_array_mips(a.size_px, a.layers, &d.hull_array_levels(a))).collect();
        let parts = h.parts.iter().map(|p| (p.first as usize, p.count as usize, p.array.map(|a| a as usize))).collect();
        Some(Self { mesh, origin: h.origin_m, scale: h.scale, parts, arrays, triangles: idx.len() / 3 })
    }

    /// Draw the hull into the current 3D pass of `t`, seen through `proj_view` (a view at the origin) from `eye`, ship
    /// coordinates. The outside keeps its own light: the interior's alert states and the panels' glow do not reach it.
    pub fn draw(&self, r: &mut Renderer, t: &Target, proj_view: Mat4, eye: [f64; 3]) {
        // The frames rule: subtract the eye in f64, then narrow to f32.
        let rel = Vec3::new(
            (self.origin[0] - eye[0]) as f32,
            (self.origin[1] - eye[1]) as f32,
            (self.origin[2] - eye[2]) as f32,
        );
        let p = DeckParams {
            mvp: proj_view * Mat4::from_translation(rel) * Mat4::from_scale(Vec3::splat(self.scale)),
            state_weights: [1.0, 0.0, 0.0],
            flash_dir: Vec3::Z,
            flash: 0.0,
            panel_first: u32::MAX,
            panel_glow: 0.0,
            clip_y: f32::MAX,
        };
        for &(first, count, array) in &self.parts {
            match array {
                Some(a) => {
                    r.draw_deck_range(t, &self.mesh, DeckProgram::Textured, &p, Some(&self.arrays[a]), first, count)
                }
                None => r.draw_deck_range(t, &self.mesh, DeckProgram::Flat, &p, None, first, count),
            }
        }
    }
}
