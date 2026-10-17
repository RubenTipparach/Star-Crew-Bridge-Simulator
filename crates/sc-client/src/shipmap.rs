//! The ship map (openspec/changes/ship-plan-view 5): an orbit round the ship, the decks pulled apart and each cut above
//! its floor, a figure for every body aboard with its name over it, and the map's title and deck at the top left.
//!
//! It lives in the client because both views on foot open it with M (or MAP by thumb): the ship tour and the game. One
//! camera, one draw and one overlay, so the two cannot show the ship differently (CLAUDE.md 6.1). Each view hands it
//! its own rooms and bodies.

use glam::{Mat4, Vec3};
use sc_render::{DeckParams, DeckProgram, Mesh, Renderer, Target, TextureArray};

use crate::look::{self, Align, Role};

/// The decks pulled apart in the map, metres.
const MAP_EXPLODE_M: f32 = 12.0;
/// Each deck is cut this high above its floor in the map, metres.
const MAP_CUT_M: f32 = 1.6;
/// Figures are drawn this much larger on the map, so a body reads from across the ship.
const MAP_FIGURE_SCALE: f32 = 3.0;

/// The map's camera: an orbit round the ship, and the deck shown.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MapView {
    pub yaw: f32,
    pub pitch: f32,
    pub dist: f32,
    /// One deck only (its index, 0 the lowest), or all.
    pub deck: Option<usize>,
}

impl MapView {
    /// The map as it opens: every deck, from aft and above on the port quarter.
    pub fn open() -> Self {
        Self { yaw: 2.1, pitch: 0.55, dist: 100.0, deck: None }
    }

    /// Tab: all decks, then each from the top, of `n` decks.
    pub fn next_deck(&mut self, n: usize) {
        self.deck = match self.deck {
            None => Some(n.saturating_sub(1)),
            Some(0) => None,
            Some(k) => Some(k - 1),
        };
    }

    /// A drag of `dx`, `dy` (pixels at `k` radians a pixel) turns the orbit.
    pub fn turn(&mut self, dx: f32, dy: f32, k: f32) {
        self.yaw -= dx * k;
        self.pitch = (self.pitch + dy * k).clamp(0.1, 1.5);
    }

    /// The wheel's `y` notches (or a pinch) closes in or backs off.
    pub fn zoom(&mut self, y: f32) {
        self.dist = (self.dist * 1.12f32.powf(-y)).clamp(25.0, 260.0);
    }

    /// The deck's name in the title: "All decks", or "Deck B" of `n` decks.
    pub fn deck_name(&self, n: usize) -> String {
        match self.deck {
            None => "All decks".to_owned(),
            Some(k) => format!("Deck {}", (b'A' + (n.saturating_sub(1) - k.min(n.saturating_sub(1))) as u8) as char),
        }
    }
}

/// A room on the map: its mesh, its frame's origin (ship frame, metres) and its deck's letter (`A` the top).
pub struct MapRoom<'a> {
    pub mesh: &'a Mesh,
    pub origin: [f64; 3],
    pub deck: char,
}

/// A body on the map: its feet (ship frame, metres), its facing, its figure, its name and colour, and whether it is
/// this player's own.
pub struct MapBody<'a> {
    pub feet: [f32; 3],
    pub yaw: f32,
    pub figure: Option<&'a Mesh>,
    pub name: String,
    pub colour: egui::Color32,
    pub own: bool,
}

/// How the rooms are lit and textured: the deck's texture array, its first panel layer and glow, the lighting state.
pub struct MapLook<'a> {
    pub tex: &'a TextureArray,
    pub panel_first: u32,
    pub panel_glow: f32,
    pub lighting: [f32; 3],
}

/// A name over a marker on the map: where (0-1 across and down the screen), what, its colour.
pub struct MapLabel {
    pub at: [f32; 2],
    pub text: String,
    pub colour: egui::Color32,
    pub own: bool,
}

/// Draw the map into the open 3D pass on `t` (`aspect` its width over height): every room of the shown decks cut
/// above its floor and lifted by its deck, then the bodies large. `decks` are the floor heights, lowest first.
/// Returns the names to draw over the markers.
#[allow(clippy::too_many_arguments)]
pub fn draw(
    r: &mut Renderer,
    t: &Target,
    m: &MapView,
    aspect: f32,
    decks: &[f32],
    rooms: &[MapRoom<'_>],
    lk: &MapLook<'_>,
    bodies: &[MapBody<'_>],
) -> Vec<MapLabel> {
    let n = decks.len().max(1);
    let deck_index = |c: char| (n - 1).saturating_sub((c as u8).saturating_sub(b'A') as usize).min(n - 1);
    let deck_of_y = |y: f32| {
        (0..decks.len()).min_by(|&a, &b| (decks[a] - y + 0.3).abs().total_cmp(&(decks[b] - y + 0.3).abs())).unwrap_or(0)
    };
    let lift = |k: usize| k as f32 * MAP_EXPLODE_M;
    let floor = |k: usize| decks.get(k).copied().unwrap_or(0.0);
    // The camera: an orbit round the ship's middle, the exploded stack's middle height.
    let target = Vec3::new(0.0, decks.first().copied().unwrap_or(0.0) + lift(n - 1) / 2.0, 0.0);
    let dir = Vec3::new(m.pitch.cos() * m.yaw.sin(), m.pitch.sin(), m.pitch.cos() * m.yaw.cos());
    let eye = target + dir * m.dist;
    let view = glam::camera::rh::view::look_to_mat4(Vec3::ZERO, -dir, Vec3::Y);
    let proj = glam::camera::rh::proj::opengl::perspective(45f32.to_radians(), aspect, 1.0, 600.0);
    let shown = |k: usize| m.deck.is_none_or(|d| d == k);
    for room in rooms {
        let k = deck_index(room.deck);
        if !shown(k) {
            continue;
        }
        let o = Vec3::new(room.origin[0] as f32, room.origin[1] as f32 + lift(k), room.origin[2] as f32);
        let p = DeckParams {
            mvp: proj * view * Mat4::from_translation(o - eye),
            state_weights: lk.lighting,
            flash_dir: Vec3::Z,
            flash: 0.0,
            panel_first: lk.panel_first,
            panel_glow: lk.panel_glow,
            clip_y: floor(k) + MAP_CUT_M - room.origin[1] as f32,
            relight: sc_render::NO_RELIGHT,
        };
        r.draw_deck(t, room.mesh, DeckProgram::Textured, &p, Some(lk.tex));
    }
    let vp = proj * view;
    let mut labels = Vec::new();
    for b in bodies {
        let k = deck_of_y(b.feet[1]);
        if !shown(k) {
            continue;
        }
        let at = Vec3::new(b.feet[0], b.feet[1] + lift(k), b.feet[2]);
        if let Some(mesh) = b.figure {
            // The map is a diagram: its figures take the fixed light, not a room's (light-baking 16).
            let turn = Mat4::from_rotation_y(b.yaw);
            let mvp = vp * Mat4::from_translation(at - eye) * turn * Mat4::from_scale(Vec3::splat(MAP_FIGURE_SCALE));
            r.draw_deck_probe(t, mesh, mvp, turn, &sc_core::probes::fallback_cube(), [1.0, 0.0, 0.0]);
        }
        let head = vp * (at - eye + Vec3::Y * (MAP_FIGURE_SCALE * 1.9 + 0.6)).extend(1.0);
        if head.w > 0.0 {
            let ndc = head.truncate() / head.w;
            if ndc.x.abs() < 1.1 && ndc.y.abs() < 1.1 {
                labels.push(MapLabel {
                    at: [(ndc.x + 1.0) / 2.0, (1.0 - ndc.y) / 2.0],
                    text: b.name.clone(),
                    colour: b.colour,
                    own: b.own,
                });
            }
        }
    }
    labels
}

/// The map's title and deck at the top left of the safe area (ui-art-direction design 11), and each marker's name on a
/// square plate (the player's edged). By thumb (`touch`) the title moves right of the MAP button in that corner.
pub fn overlay(ctx: &egui::Context, deck: &str, labels: &[MapLabel], touch: bool) {
    let screen = ctx.content_rect();
    let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Background, egui::Id::new("names")));
    let c = look::look().c();
    let k = look::kit(&painter, screen, crate::ui::Anchor::TopLeft, egui::Vec2::new(800.0, 200.0));
    // MAP is 48 px of the 390 px reference a phone is laid out on, 12 px in: about 190 of the 1080 the kit draws in.
    let left = if touch { 230.0 } else { 96.0 };
    k.text(Role::Label, left, 54.0, c.accent.0, "Ship map", Align::Start);
    k.text(Role::Title, left, 54.0 + k.line(Role::Label) + 8.0, c.text.0, deck, Align::Start);
    let (_, s) = crate::ui::anchored(screen, crate::ui::Anchor::Centre, egui::Vec2::ZERO);
    for l in labels {
        let p = egui::pos2(screen.min.x + l.at[0] * screen.width(), screen.min.y + l.at[1] * screen.height());
        let m = look::Kit::new(painter.clone(), p, s);
        let role = if l.own { Role::Body } else { Role::Label };
        let w = m.width(role, &l.text) + 20.0;
        let h = m.line(role) + 8.0;
        m.fill(-w / 2.0, -h / 2.0, w, h, look::alpha(c.panel.0, 0.9));
        if l.own {
            m.edge(-w / 2.0, -h / 2.0, w, h, l.colour);
        }
        m.text_mid(role, 0.0, 0.0, l.colour, &l.text, Align::Middle);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tab_steps_the_map_through_all_decks_then_each_from_the_top() {
        let mut m = MapView::open();
        let mut names = vec![m.deck_name(3)];
        for _ in 0..4 {
            m.next_deck(3);
            names.push(m.deck_name(3));
        }
        assert_eq!(names, ["All decks", "Deck A", "Deck B", "Deck C", "All decks"]);
    }
}
