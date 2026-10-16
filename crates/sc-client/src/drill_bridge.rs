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
// The outside is drawn after the bridge at the back of the depth buffer (the bridge in the front half, the outside
// behind it, the sky layer at the far plane: sky-behind-solids), so it shows only through the windows.
use sc_render::depth::{behind_everything, in_front_half, outside_lens};

/// A body's eye above its feet standing, and seated (crew-on-deck), metres.
const EYE_STAND_M: f32 = 1.65;
const EYE_SEAT_M: f32 = 1.2;
/// A seated eye's lean toward the desk, from the seat point, metres.
const EYE_SEAT_FWD_M: f32 = 0.1;

/// A room's bounds, ship frame, metres: least and greatest corner.
type Bounds = ([f64; 3], [f64; 3]);
/// The eye above the floor it stands on, metres (a standing person's, near enough for the overview's eye too).
const EYE_FLOOR_M: f64 = 1.6;

/// The ship's decks and the crew's figures: the bridge, and the rest of the ship the crew walk up from.
pub struct BridgeView {
    /// Every room of every deck: its mesh, its origin and its bounds (ship frame, metres). All are drawn but those
    /// wholly outside the view (the owner, 2026-10-12: "there are holes in the ship", "the lower floors or distant rooms
    /// or DOORs are not loaded in": a floor hatch, a ladder's well or a doorway looks into the room beyond, whatever
    /// its deck).
    rooms: Vec<(Mesh, [f64; 3], Bounds)>,
    /// The walk's collision, for whether one person sees another (crew-nameplates 2).
    world: Option<sc_core::walk::WalkWorld>,
    /// The viewscreens on the command deck.
    views: Vec<DeckView>,
    /// The crew figures, one a division, by the division's id (crew-uniforms 3).
    figures: Vec<(String, Mesh)>,
    tex: TextureArray,
    panel_first: u32,
    panel_glow: f32,
    /// The hull (tern-exterior 4): the bow's rim and the forward hull through the bridge's windows.
    hull: Option<crate::hull::HullView>,
    /// Triangles drawn for the rooms.
    pub triangles: usize,
    /// The eye's vertical field of view, degrees (Options, game-menu 3).
    pub fov_deg: f32,
    /// The extinguishers' brackets round the ship (safety-points 5a), in groups about an origin each (the vertex
    /// format's reach): the brackets' fixed parts, their extinguishers, and which brackets (by the layout's index) a
    /// group holds, in its order.
    points: Vec<PointGroup>,
}

/// A group of safety points drawn about one origin.
struct PointGroup {
    origin: [f64; 3],
    fixed: Mesh,
    hung: Mesh,
    brackets: Vec<usize>,
}

/// Safety points within this distance along the keel of a group's origin share its meshes, metres (inside the vertex
/// format's 32 m).
const POINT_GROUP_M: f64 = 25.0;

impl BridgeView {
    /// The hull, when the deck has one: the CHASE camera draws it from outside (chase-camera 1).
    pub fn hull(&self) -> Option<&crate::hull::HullView> {
        self.hull.as_ref()
    }

    /// Load the decks' rooms, the figures and the walk's collision from the compiled deck at `path`.
    pub fn load(r: &mut Renderer, path: &Path) -> Result<Self, String> {
        let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let d = sc_core::deck::read(&bytes).map_err(|e| format!("{}: {e}", path.display()))?;
        let t = &d.index.textures;
        let tex = r.make_texture_array_mips(t.size_px, t.layers, &d.texture_levels());
        let (mut rooms, mut figures, mut triangles) = (Vec::new(), Vec::new(), 0);
        for c in &d.index.compartments {
            let idx = d.indices_of(c);
            match (c.deck.as_str(), c.id.as_str()) {
                ("figure", id) => figures.push((
                    id.trim_start_matches("figure_").to_owned(),
                    r.make_mesh(d.vertices_of(c), Indices::U32(&idx)),
                )),
                // The walk's dock (the tour's girder frame) is not the patrol's: Tamsin Station is `stations` (12b).
                ("outside", _) => {}
                (deck, _) => {
                    if deck == "A" {
                        triangles += idx.len() / 3;
                    }
                    let v = d.vertices_of(c);
                    rooms.push((r.make_mesh(v, Indices::U32(&idx)), c.origin_m, bounds(v, c.origin_m)));
                }
            }
        }
        let walk_text =
            std::fs::read_to_string("data/crew/walk.json").map_err(|e| format!("data/crew/walk.json: {e}"))?;
        let walk: sc_core::walk::WalkData =
            sc_core::data::parse("data/crew/walk.json", &walk_text).map_err(|e| e.to_string())?;
        let w = &d.index.walk;
        let ents = sc_core::walk::WalkEntities {
            ladders: w.ladders.clone(),
            hatches: w.hatches.clone(),
            doors: w.doors.clone(),
            lifts: w.lifts.clone(),
        };
        let world = sc_core::walk::WalkWorld::new(&d.walk_triangles(), &ents, &walk).ok();
        let floors: Vec<f64> = d
            .index
            .compartments
            .iter()
            .filter(|c| c.deck == "A")
            .filter_map(|c| Some(f64::from(c.floor_m?[1])))
            .collect();
        let views = views_on(&d.index.views, &floors);
        let fov_deg = crate::options::FOV_DEG[2] as f32;
        let panel_glow = t.panel_glow[0];
        let hull = crate::hull::HullView::load(r, &d)?;
        Ok(Self {
            rooms,
            world,
            views,
            figures,
            tex,
            panel_first: t.panel_first,
            panel_glow,
            hull,
            triangles,
            fov_deg,
            points: Vec::new(),
        })
    }

    /// The extinguishers' brackets to draw (the layout's, `ShipLayout::brackets`): grouped along the keel, each group
    /// two meshes. Returns the triangles they add.
    pub fn set_brackets(&mut self, r: &Renderer, brackets: &[sc_core::combat::damage::Bracket]) -> usize {
        self.points.clear();
        // Aft to fore: each group takes the brackets within twice POINT_GROUP_M of its aftmost, about their middle.
        let mut left: Vec<usize> = (0..brackets.len()).collect();
        left.sort_by(|a, b| brackets[*a].wall.z.total_cmp(&brackets[*b].wall.z).then(a.cmp(b)));
        let mut triangles = 0;
        while let Some(&k0) = left.first() {
            let z0 = brackets[k0].wall.z;
            let (mine, rest): (Vec<usize>, Vec<usize>) =
                left.iter().partition(|k| brackets[**k].wall.z - z0 <= POINT_GROUP_M * 2.0);
            left = rest;
            let zs = mine.iter().map(|k| brackets[*k].wall.z);
            let mid = (zs.clone().fold(f64::MAX, f64::min) + zs.fold(f64::MIN, f64::max)) / 2.0;
            let origin = [0.0, 0.0, mid];
            let pts: Vec<([f32; 3], [f32; 3])> = mine
                .iter()
                .map(|k| (brackets[*k].wall.as_vec3().to_array(), brackets[*k].into.as_vec3().to_array()))
                .collect();
            let (fixed, hung) = crate::ships3d::safety_points(&pts, origin.map(|x| x as f32));
            triangles += fixed.triangles() + hung.triangles();
            let mesh = |m: &crate::ships3d::MeshBuilder| r.make_mesh(&m.vertices, Indices::U16(&m.indices));
            self.points.push(PointGroup { origin, fixed: mesh(&fixed), hung: mesh(&hung), brackets: mine });
        }
        triangles
    }

    /// Whether a person whose feet are at `to` is in sight from the eye at `eye` (ship frame, metres): a ray at chest
    /// height through the walk's collision (`WalkWorld::sees`); true when no collision was loaded.
    pub fn sees(&self, eye: [f64; 3], to: [f32; 3]) -> bool {
        let from = [eye[0] as f32, (eye[1] - EYE_FLOOR_M) as f32, eye[2] as f32];
        self.world.as_ref().is_none_or(|w| w.sees(from, to))
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
        bodies: &[(BodySnap, String)],
        me: Option<u8>,
        own_eye: bool,
        inside: &Inside<'_>,
        out: Outside<'_>,
    ) -> Mat4 {
        let fov = self.fov_deg.to_radians();
        let lens = glam::camera::rh::proj::opengl::perspective(fov, aspect, 0.05, 400.0);
        // The bridge in the front half of the depth buffer, the outside behind it (`OUTSIDE_DEPTH`).
        let proj = in_front_half() * lens;
        let dir = Vec3::new(yaw.sin() * pitch.cos(), pitch.sin(), yaw.cos() * pitch.cos());
        let view = glam::camera::rh::view::look_to_mat4(Vec3::ZERO, dir, Vec3::Y);
        // The outside is in the system's frame: a direction there reaches the eye turned into the ship's.
        let view_out = view * Mat4::from_quat(out.attitude.inverse());
        r.begin_3d(t, [0.004, 0.005, 0.012, 1.0]);
        r.draw_sky(t, &crate::sky_params(ext, lens, view_out, fov / 720.0, inside.time_s));
        let w = out.lighting;
        // Every room in view, whatever its deck: the depth buffer hides what walls and floors cover.
        let vp = proj * view;
        for (mesh, o, _) in self.rooms.iter().filter(|(_, _, b)| in_view(vp, eye, b)) {
            let rel = Vec3::new((o[0] - eye[0]) as f32, (o[1] - eye[1]) as f32, (o[2] - eye[2]) as f32);
            let p = DeckParams {
                mvp: proj * view * Mat4::from_translation(rel),
                state_weights: w,
                flash_dir: Vec3::Z,
                flash: 0.0,
                panel_first: self.panel_first,
                panel_glow: self.panel_glow,
                clip_y: f32::MAX,
                relight: sc_render::NO_RELIGHT,
            };
            r.draw_deck(t, mesh, DeckProgram::Textured, &p, Some(&self.tex));
        }
        // The hull (tern-exterior 4): after the rooms, so their walls hide most of it before it is shaded.
        // Lit by the sky's sun, turned into the ship's frame by the Tern's attitude (patrol-mission design 11.3).
        if let Some(h) = &self.hull {
            let (light, k) = out.hull_light.unwrap_or((crate::outside::sun_in_ship(ext, out.attitude), 1.0));
            h.draw_lit(r, t, proj * view, eye, light, k, out.hull_light.is_none().then_some(out.attitude));
        }
        // The viewscreens: the feed's picture on a quad 3 cm in front of each, as the walk draws them.
        if let Some((feed, scanlines)) = out.screen {
            for vw in self.views.iter().filter(|vw| faces(vw, eye)) {
                let look = [scanlines, feed.size().1 as f32, 1.0, 0.0];
                r.draw_screen(t, proj * view * screen_model(vw, eye), feed, look);
            }
        }
        let cube = sc_core::probes::fallback_cube();
        for (b, division) in bodies {
            if own_eye && Some(b.slot) == me {
                continue;
            }
            let Some(mesh) = self.figures.iter().find(|(d, _)| d == division).or(self.figures.first()).map(|(_, m)| m)
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
        // The extinguishers' brackets round the ship, and the extinguishers in the full ones (safety-points 5a): a
        // group's fixed parts in one draw, its extinguishers in runs of full brackets.
        let per = crate::ships3d::extinguisher().indices.len();
        for g in &self.points {
            let rel =
                Vec3::new((g.origin[0] - eye[0]) as f32, (g.origin[1] - eye[1]) as f32, (g.origin[2] - eye[2]) as f32);
            let p = flat_params(proj * view * Mat4::from_translation(rel));
            r.draw_deck(t, &g.fixed, DeckProgram::Flat, &p, None);
            // While a fire burns aboard the extinguishers breathe toward their yellow (their third colour set).
            let k = inside.fx.highlight(inside.time_s);
            let p = DeckParams { state_weights: [1.0 - k, 0.0, k], ..p };
            let full = |i: usize| !inside.empty_brackets.contains(&g.brackets[i]);
            let mut i = 0;
            while i < g.brackets.len() {
                if !full(i) {
                    i += 1;
                    continue;
                }
                let start = i;
                while i < g.brackets.len() && full(i) {
                    i += 1;
                }
                r.draw_deck_range(t, &g.hung, DeckProgram::Flat, &p, None, start * per, (i - start) * per);
            }
        }
        // Fires across their rooms' floors, their smoke, the agent in the air and a smoky room's haze (all-hands 4),
        // one draw, farthest first.
        if !inside.fx.is_empty() {
            let quads = inside.fx.quads(eye, dir, inside.time_s);
            r.draw_fx(t, proj * view, inside.time_s as f32, &quads);
        }
        // This player's extinguisher in hand, at the lower right of the eye, as the mockup holds it.
        if own_eye && inside.carrying {
            let held = crate::firefx::held_model();
            r.draw_deck(t, inside.held, DeckProgram::Flat, &flat_params(proj * held), None);
        }
        // Last, the fight outside, at the back of the depth buffer: only the windows let it through.
        (out.objects)(r, t, behind_everything() * outside_lens(fov, aspect) * view_out);
        r.end_pass();
        proj * view
    }
}

/// A room's bounds from its packed vertices (positions in 1/1024 m from its origin, `sc-core::vertex`), ship frame.
fn bounds(v: &[u8], origin: [f64; 3]) -> Bounds {
    let (mut lo, mut hi) = ([f64::MAX; 3], [f64::MIN; 3]);
    for vx in v.chunks_exact(sc_core::vertex::DECK_VERTEX_BYTES) {
        for k in 0..3 {
            let p = f64::from(i16::from_le_bytes([vx[k * 2], vx[k * 2 + 1]])) / f64::from(sc_core::vertex::FIXED_SCALE);
            lo[k] = lo[k].min(origin[k] + p);
            hi[k] = hi[k].max(origin[k] + p);
        }
    }
    (lo, hi)
}

/// Whether any of a box (ship frame) is in the view of `vp` from `eye` (the box taken relative to the eye, as the rooms
/// are drawn): false only when all eight corners lie outside one clip plane.
fn in_view(vp: Mat4, eye: [f64; 3], b: &Bounds) -> bool {
    let (lo, hi) = b;
    let corners: Vec<Vec4> = (0..8)
        .map(|i| {
            let c = |k: usize| if i & (1 << k) == 0 { lo[k] } else { hi[k] };
            vp * Vec4::new((c(0) - eye[0]) as f32, (c(1) - eye[1]) as f32, (c(2) - eye[2]) as f32, 1.0)
        })
        .collect();
    let all = |f: &dyn Fn(&Vec4) -> bool| corners.iter().all(f);
    !(all(&|p| p.x < -p.w)
        || all(&|p| p.x > p.w)
        || all(&|p| p.y < -p.w)
        || all(&|p| p.y > p.w)
        || all(&|p| p.z < -p.w)
        || all(&|p| p.z > p.w))
}

/// What the ship shows inside besides its rooms and crew: fires burning, and this player's extinguisher.
pub struct Inside<'a> {
    /// The fires, their smoke and the agent in the air, as drawn (`firefx`).
    pub fx: &'a crate::firefx::FireFx,
    /// The extinguisher in hand.
    pub held: &'a Mesh,
    /// The brackets standing empty (the layout's indices), from the snapshot.
    pub empty_brackets: &'a [usize],
    /// This player carries an extinguisher.
    pub carrying: bool,
    /// The local clock, seconds: the flames move with it.
    pub time_s: f64,
}

/// A mesh's own colours, unlit and unshaded (bolts, bursts, flames): the deck program's flat parameters.
pub fn flat_params(mvp: Mat4) -> DeckParams {
    DeckParams {
        mvp,
        state_weights: [1.0, 0.0, 0.0],
        flash_dir: Vec3::ZERO,
        flash: 0.0,
        panel_first: u32::MAX,
        panel_glow: 1.0,
        clip_y: f32::MAX,
        relight: sc_render::NO_RELIGHT,
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
    /// The light on the hull when it is not the sky's sun: under a station's roof, its floods' way (the ship's frame)
    /// and strength (patrol-mission design 12b).
    pub hull_light: Option<(Vec3, f32)>,
    /// Draws the Hound, missiles, bolts and bursts into the pass, given the view-projection from the bridge's eye in
    /// the system's frame (directions only: positions are taken relative to the eye).
    pub objects: DrawOutside<'a>,
}

/// Draws the fight outside into a pass, given the view-projection from the bridge's eye in the system's frame.
pub type DrawOutside<'a> = Box<dyn FnOnce(&mut Renderer, &Target, Mat4) + 'a>;

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
    // Seated, the head is forward of the chair's back, over the desk's edge, not in its headrest.
    let (up, fwd) = if b.posture == Posture::Seated { (EYE_SEAT_M, EYE_SEAT_FWD_M) } else { (EYE_STAND_M, 0.0) };
    [f64::from(b.pos[0] + b.yaw.sin() * fwd), f64::from(b.pos[1] + up), f64::from(b.pos[2] + b.yaw.cos() * fwd)]
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
        use sc_render::depth::{window_depth, OUTSIDE_DEPTH, OUTSIDE_NEAR_M, SOLID_REACH_M};
        let (aspect, fov) = (16.0 / 9.0, 70f32.to_radians());
        let far = behind_everything() * outside_lens(fov, aspect);
        let near = in_front_half() * glam::camera::rh::proj::opengl::perspective(fov, aspect, 0.05, 400.0);
        // The farthest wall of the bridge, 40 m away, against the nearest thing outside, the near plane.
        assert!(
            window_depth(near, 40.0) < window_depth(far, OUTSIDE_NEAR_M),
            "the bridge's back wall hides the outside"
        );
        for d in [OUTSIDE_NEAR_M, 2_000.0, SOLID_REACH_M] {
            let z = window_depth(far, d);
            assert!((f64::from(1.0 - OUTSIDE_DEPTH) - 1e-6..1.0).contains(&z), "{d} m lands at depth {z}");
        }
        assert!(window_depth(far, 1_500.0) < window_depth(far, 3_000.0), "the outside keeps its own order");
        assert!(window_depth(near, 399.0) <= f64::from(1.0 - OUTSIDE_DEPTH), "the whole bridge is in front of it");
    }
}
