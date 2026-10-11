//! The bridge in the drill client (openspec/changes/coop-drill design 9): the compiled deck's command deck and the
//! crew's figures, drawn while this player's body walks between seats, and from the back wall for the overview (V).
//!
//! It lives beside the drill because it is the drill's view of the ship; it draws with the ship walk's renderer
//! calls and the deck's own meshes (`deckc`), so the bridge looks as it does on foot in the walkable ship.

use std::path::Path;

use glam::{Mat4, Vec3, Vec4};
use sc_core::combat::bodies::Posture;
use sc_core::exterior::ExteriorData;
use sc_net::msg::BodySnap;
use sc_render::{DeckParams, DeckProgram, Indices, Mesh, Renderer, Target, TextureArray};

/// A body's eye above its feet standing, and seated (crew-on-deck), metres.
const EYE_STAND_M: f32 = 1.65;
const EYE_SEAT_M: f32 = 1.2;

/// The command deck and the crew's figures.
pub struct BridgeView {
    rooms: Vec<(Mesh, [f64; 3])>,
    /// The figure players wear, and the one bots wear.
    player: Option<Mesh>,
    crew: Option<Mesh>,
    tex: TextureArray,
    panel_first: u32,
    panel_glow: f32,
    /// Triangles drawn for the rooms.
    pub triangles: usize,
}

impl BridgeView {
    /// Load deck A's rooms and the figures from the compiled deck at `path`.
    pub fn load(r: &mut Renderer, path: &Path) -> Result<Self, String> {
        let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let d = sc_core::deck::read(&bytes).map_err(|e| format!("{}: {e}", path.display()))?;
        let t = &d.index.textures;
        let levels: Vec<&[u8]> = (0..t.mips as usize)
            .map(|k| {
                let a = t.mip_offsets[k] as usize;
                let b = t.mip_offsets.get(k + 1).map_or(d.textures.len(), |o| *o as usize);
                &d.textures[a..b]
            })
            .collect();
        let tex = r.make_texture_array_mips(t.size_px, t.layers, &levels);
        let (mut rooms, mut player, mut crew, mut triangles) = (Vec::new(), None, None, 0);
        for c in &d.index.compartments {
            let idx = d.indices_of(c);
            match (c.deck.as_str(), c.id.as_str()) {
                ("figure", "figure_player") => player = Some(r.make_mesh(d.vertices_of(c), Indices::U32(&idx))),
                ("figure", "figure_command") => crew = Some(r.make_mesh(d.vertices_of(c), Indices::U32(&idx))),
                ("A", _) => {
                    triangles += idx.len() / 3;
                    rooms.push((r.make_mesh(d.vertices_of(c), Indices::U32(&idx)), c.origin_m));
                }
                _ => {}
            }
        }
        Ok(Self { rooms, player, crew, tex, panel_first: t.panel_first, panel_glow: t.panel_glow[0], triangles })
    }

    /// Draw the bridge from `eye` (ship frame) looking along `yaw` (0 to the bow) and `pitch` into `t`, with the
    /// bodies; `me` is this player's slot (its own body is not drawn from its own eye). Returns the projection and
    /// view, for the names over the bodies.
    #[allow(clippy::too_many_arguments)]
    pub fn draw(
        &self,
        r: &mut Renderer,
        t: &Target,
        ext: &ExteriorData,
        eye: [f64; 3],
        yaw: f32,
        pitch: f32,
        aspect: f32,
        bodies: &[(BodySnap, bool)],
        me: Option<u8>,
        own_eye: bool,
    ) -> Mat4 {
        let fov = 70f32.to_radians();
        let proj = glam::camera::rh::proj::opengl::perspective(fov, aspect, 0.05, 400.0);
        let dir = Vec3::new(yaw.sin() * pitch.cos(), pitch.sin(), yaw.cos() * pitch.cos());
        let view = glam::camera::rh::view::look_to_mat4(Vec3::ZERO, dir, Vec3::Y);
        r.begin_3d(t, [0.004, 0.005, 0.012, 1.0]);
        r.draw_sky(t, &crate::sky_params(ext, proj, view, fov / 720.0));
        let w = [1.0, 0.0, 0.0];
        for (mesh, o) in &self.rooms {
            let rel = Vec3::new((o[0] - eye[0]) as f32, (o[1] - eye[1]) as f32, (o[2] - eye[2]) as f32);
            let p = DeckParams {
                mvp: proj * view * Mat4::from_translation(rel),
                state_weights: w,
                flash_dir: Vec3::Z,
                flash: 0.0,
                panel_first: self.panel_first,
                panel_glow: self.panel_glow,
                clip_y: f32::MAX,
            };
            r.draw_deck(t, mesh, DeckProgram::Textured, &p, Some(&self.tex));
        }
        let cube = sc_core::probes::fallback_cube();
        for (b, bot) in bodies {
            if own_eye && Some(b.slot) == me {
                continue;
            }
            let Some(mesh) = (if *bot { self.crew.as_ref() } else { self.player.as_ref() }).or(self.player.as_ref())
            else {
                continue;
            };
            // Seated, the figure sinks to the chair (the blocky figure has no sitting pose yet).
            let sink = if b.posture == Posture::Seated { -0.45 } else { 0.0 };
            let rel = Vec3::new(
                (f64::from(b.pos[0]) - eye[0]) as f32,
                (f64::from(b.pos[1]) + sink - eye[1]) as f32,
                (f64::from(b.pos[2]) - eye[2]) as f32,
            );
            let turn = Mat4::from_rotation_y(b.yaw);
            r.draw_deck_probe(t, mesh, proj * view * Mat4::from_translation(rel) * turn, turn, &cube, w);
        }
        r.end_pass();
        proj * view
    }
}

/// The eye of a body (standing or seated), ship frame.
pub fn eye_of(b: &BodySnap) -> [f64; 3] {
    let up = if b.posture == Posture::Seated { EYE_SEAT_M } else { EYE_STAND_M };
    [f64::from(b.pos[0]), f64::from(b.pos[1] + up), f64::from(b.pos[2])]
}

/// The overview: from high at the bow end of the bridge, under the viewscreen, looking aft and down over the seats to
/// the muster point at the back, so every body is in view from the moment it joins.
pub fn overview() -> ([f64; 3], f32, f32) {
    ([0.0, 6.2, 31.8], std::f32::consts::PI, -0.40)
}

/// Bodies between two snapshots (the 100 ms behind, as the ships are): positions and headings blended by `f`.
pub fn blend(a: &[BodySnap], b: &[BodySnap], f: f64) -> Vec<BodySnap> {
    let f = f as f32;
    b.iter()
        .map(|nb| match a.iter().find(|x| x.slot == nb.slot) {
            Some(oa) => {
                let mut o = *nb;
                for i in 0..3 {
                    o.pos[i] = oa.pos[i] + (nb.pos[i] - oa.pos[i]) * f;
                }
                let d =
                    (nb.yaw - oa.yaw + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
                o.yaw = oa.yaw + d * f;
                o
            }
            None => *nb,
        })
        .collect()
}

/// Where a ship-frame point `p` (seen from `eye`) lands on a screen `w` x `h` points, or None behind the eye.
pub fn project(vp: Mat4, eye: [f64; 3], p: [f32; 3], w: f32, h: f32) -> Option<[f32; 2]> {
    let rel = Vec4::new(
        (f64::from(p[0]) - eye[0]) as f32,
        (f64::from(p[1]) - eye[1]) as f32,
        (f64::from(p[2]) - eye[2]) as f32,
        1.0,
    );
    let c = vp * rel;
    if c.w <= 0.05 {
        return None;
    }
    Some([(c.x / c.w * 0.5 + 0.5) * w, (0.5 - c.y / c.w * 0.5) * h])
}
