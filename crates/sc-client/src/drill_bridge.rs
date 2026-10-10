//! The bridge in the drill client (openspec/changes/coop-drill design 9): the compiled deck's command deck and the
//! crew's figures, drawn while this player's body walks between seats, and from the back wall for the overview (V).
//!
//! It lives beside the drill because it is the drill's view of the ship; it draws with the ship walk's renderer
//! calls and the deck's own meshes (`deckc`), so the bridge looks as it does on foot in the walkable ship. The outside
//! is the fight's: the sky turns with the Tern, the Hound, bolts and bursts show through the windows, the bridge's
//! viewscreens carry the feed the drill renders (the captain's camera while he holds the viewscreen), and red alert
//! lights the room in its red state (`bridge-stations` 1 and 7).

use std::path::Path;

use glam::{Mat4, Quat, Vec3, Vec4};
use sc_core::combat::bodies::Posture;
use sc_core::deck::DeckView;
use sc_core::exterior::ExteriorData;
use sc_net::msg::BodySnap;
use sc_render::{DeckParams, DeckProgram, Indices, Mesh, Renderer, Target, TextureArray};

/// A body's eye above its feet standing, and seated (crew-on-deck), metres.
const EYE_STAND_M: f32 = 1.65;
const EYE_SEAT_M: f32 = 1.2;

/// The outside's near and far planes, metres: the Hound at a few kilometres, the farthest bolt and burst well inside.
const OUTSIDE_NEAR_M: f32 = 1.0;
const OUTSIDE_FAR_M: f32 = 80_000.0;

/// The depth the outside is squeezed into, at the back of the depth buffer (window depth 1 - this to 1): drawn after
/// the bridge, it shows only where no part of the bridge is in front, which is through the windows.
const OUTSIDE_DEPTH: f32 = 0.0001;

/// The command deck and the crew's figures.
pub struct BridgeView {
    rooms: Vec<(Mesh, [f64; 3])>,
    /// The viewscreens on the command deck.
    views: Vec<DeckView>,
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
        let floors: Vec<f64> = d
            .index
            .compartments
            .iter()
            .filter(|c| c.deck == "A")
            .filter_map(|c| Some(f64::from(c.floor_m?[1])))
            .collect();
        let views = views_on(&d.index.views, &floors);
        Ok(Self { rooms, views, player, crew, tex, panel_first: t.panel_first, panel_glow: t.panel_glow[0], triangles })
    }

    /// Draw the bridge from `eye` (ship frame) looking along `yaw` (0 to the bow) and `pitch` into `t`, with the
    /// bodies; `me` is this player's slot (its own body is not drawn from its own eye). `out` is the outside: the
    /// Tern's attitude, the lighting state, the viewscreens' picture, and the fight drawn through the windows.
    /// Returns the projection and view in the ship's frame, for the names over the bodies.
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
        out: Outside<'_>,
    ) -> Mat4 {
        let fov = 70f32.to_radians();
        let proj = glam::camera::rh::proj::opengl::perspective(fov, aspect, 0.05, 400.0);
        let dir = Vec3::new(yaw.sin() * pitch.cos(), pitch.sin(), yaw.cos() * pitch.cos());
        let view = glam::camera::rh::view::look_to_mat4(Vec3::ZERO, dir, Vec3::Y);
        // The outside is in the system's frame: a direction there reaches the eye turned into the ship's.
        let view_out = view * Mat4::from_quat(out.attitude.inverse());
        r.begin_3d(t, [0.004, 0.005, 0.012, 1.0]);
        r.draw_sky(t, &crate::sky_params(ext, proj, view_out, fov / 720.0));
        let w = out.lighting;
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
        // The viewscreens: the feed's picture on a quad 3 cm in front of each, as the walk draws them.
        if let Some((feed, scanlines)) = out.screen {
            for vw in self.views.iter().filter(|vw| faces(vw, eye)) {
                let look = [scanlines, feed.size().1 as f32, 1.0, 0.0];
                r.draw_screen(t, proj * view * screen_model(vw, eye), feed, look);
            }
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
        // Last, the fight outside, at the back of the depth buffer: only the windows let it through.
        let far = glam::camera::rh::proj::opengl::perspective(fov, aspect, OUTSIDE_NEAR_M, OUTSIDE_FAR_M);
        (out.objects)(r, t, behind_everything() * far * view_out);
        r.end_pass();
        proj * view
    }
}

/// What the bridge shows of the outside.
pub struct Outside<'a> {
    /// The Tern's attitude, ship frame to the system's.
    pub attitude: Quat,
    /// The lighting state's weights (normal, red alert, emergency).
    pub lighting: [f32; 3],
    /// The viewscreens' picture and its scan lines' depth, once rendered this frame.
    pub screen: Option<(&'a Target, f32)>,
    /// Draws the Hound, missiles, bolts and bursts into the pass, given the view-projection from the bridge's eye in
    /// the system's frame (directions only: positions are taken relative to the eye).
    pub objects: DrawOutside<'a>,
}

/// Draws the fight outside into a pass, given the view-projection from the bridge's eye in the system's frame.
pub type DrawOutside<'a> = Box<dyn FnOnce(&mut Renderer, &Target, Mat4) + 'a>;

/// The clip-space map that puts a projection's depth into the last `OUTSIDE_DEPTH` of the buffer (OpenGL's -1..1 to
/// 1 - 2 * OUTSIDE_DEPTH..1), keeping x, y and w.
pub fn behind_everything() -> Mat4 {
    let k = 2.0 * OUTSIDE_DEPTH;
    Mat4::from_cols(Vec4::X, Vec4::Y, Vec4::new(0.0, 0.0, k / 2.0, 0.0), Vec4::new(0.0, 0.0, 1.0 - k / 2.0, 1.0))
}

/// A viewscreen's quad seen from `eye` (ship frame): its corners (-1..1) on the screen's face, 3 cm out from the wall.
pub fn screen_model(vw: &DeckView, eye: [f64; 3]) -> Mat4 {
    let f = vw.facing_yaw_deg.to_radians();
    let n = Vec3::new(f.sin(), 0.0, f.cos());
    let right = Vec3::new(n.z, 0.0, -n.x);
    let c = Vec3::new(
        (f64::from(vw.center_m[0]) - eye[0]) as f32,
        (f64::from(vw.center_m[1]) - eye[1]) as f32,
        (f64::from(vw.center_m[2]) - eye[2]) as f32,
    ) + n * 0.03;
    Mat4::from_cols(
        (right * vw.size_m[0] / 2.0).extend(0.0),
        (Vec3::Y * vw.size_m[1] / 2.0).extend(0.0),
        n.extend(0.0),
        c.extend(1.0),
    )
}

/// Whether `eye` (ship frame) is in front of a viewscreen: a screen is seen from its face only. The overview's eye is
/// in the bow wall behind the bridge's viewscreen, looking aft through it.
pub fn faces(vw: &DeckView, eye: [f64; 3]) -> bool {
    let f = f64::from(vw.facing_yaw_deg).to_radians();
    let (dx, dz) = (eye[0] - f64::from(vw.center_m[0]), eye[2] - f64::from(vw.center_m[2]));
    dx * f.sin() + dz * f.cos() > 0.0
}

/// The viewscreens on a deck whose rooms stand on `floors` (metres): those from 1 m under the lowest floor to 6 m over
/// the highest, so a screen on another deck is never drawn floating outside the bridge.
pub fn views_on(views: &[DeckView], floors: &[f64]) -> Vec<DeckView> {
    let lo = floors.iter().copied().fold(f64::INFINITY, f64::min) - 1.0;
    let hi = floors.iter().copied().fold(f64::NEG_INFINITY, f64::max) + 6.0;
    views.iter().filter(|v| (lo..=hi).contains(&f64::from(v.center_m[1]))).cloned().collect()
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

#[cfg(test)]
mod tests {
    use super::*;

    /// The Tern's bridge viewscreen as the compiled deck has it (layout fixture `viewscreen`).
    fn bridge_screen() -> DeckView {
        DeckView { id: "viewscreen".into(), center_m: [0.0, 5.2, 31.5], size_m: [6.0, 2.4], facing_yaw_deg: 180.0 }
    }

    #[test]
    fn the_viewscreen_is_drawn_from_the_bridge_and_not_from_the_overview_behind_it() {
        // Regression: the overview's eye sits in the bow wall behind the screen, and a quad drawn from behind 0.3 m
        // away filled the whole view with the feed's stars.
        assert!(!faces(&bridge_screen(), overview().0), "the overview looks aft through the screen's back");
        assert!(faces(&bridge_screen(), [0.0, 5.15, 26.0]), "the captain's chair faces the screen");
    }

    #[test]
    fn only_the_screens_on_the_command_deck_are_kept() {
        let mut below = bridge_screen();
        below.id = "deck_b_screen".into();
        below.center_m[1] = 0.5;
        let kept = views_on(&[bridge_screen(), below], &[3.5, 3.5, 6.5]);
        assert_eq!(kept.iter().map(|v| v.id.as_str()).collect::<Vec<_>>(), ["viewscreen"]);
    }

    #[test]
    fn the_outside_lands_behind_every_part_of_the_bridge() {
        let aspect = 16.0 / 9.0;
        let fov = 70f32.to_radians();
        let far = behind_everything()
            * glam::camera::rh::proj::opengl::perspective(fov, aspect, OUTSIDE_NEAR_M, OUTSIDE_FAR_M);
        let near = glam::camera::rh::proj::opengl::perspective(fov, aspect, 0.05, 400.0);
        let depth = |m: Mat4, d: f32| {
            let c = m * Vec4::new(0.0, 0.0, -d, 1.0);
            (c.z / c.w) * 0.5 + 0.5
        };
        // The farthest wall of the bridge, 40 m away, against the nearest thing outside, the near plane.
        assert!(depth(near, 40.0) < depth(far, OUTSIDE_NEAR_M), "the bridge's back wall must hide the outside");
        for d in [OUTSIDE_NEAR_M, 2_000.0, OUTSIDE_FAR_M * 0.99] {
            let z = depth(far, d);
            assert!((1.0 - OUTSIDE_DEPTH - 1e-6..=1.0).contains(&z), "{d} m lands at depth {z}");
        }
        assert!(depth(far, 1_500.0) < depth(far, 3_000.0), "the outside keeps its own order");
    }
}
