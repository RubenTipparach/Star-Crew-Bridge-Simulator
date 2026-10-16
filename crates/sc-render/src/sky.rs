//! The sky layer's buffers and the streak's per-frame rule (openspec/changes/nebula-skies, design 3, 5 and 7).
//!
//! The sky is one mesh, a closed sphere of directions with a colour each (the system's nebula, or with none loaded an
//! octahedron of black, the old sky's look), and the vector stars over it as GL lines and points. They are drawn by
//! `Renderer::draw_sky` with the sky layer's depth (`sky_layer_depth`), so nothing in the sky ever covers a solid. This
//! module builds their vertices from `sc-core`'s decoded sky and decides, each frame of each view, how long the stars'
//! tails are: the vertex shader (`shaders/sky.glsl`, `star_vs`) draws each star from that, FreeSpace 2's rule.

use glam::{Mat3, Mat4};
use sc_core::sky::{NebulaSky, Starfield, Streak};

/// A sky mesh vertex: a direction (unit length) and its colour, sRGB, with the share of stars hidden in the fourth byte.
#[repr(C)]
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub(crate) struct SkyVertex {
    pub dir: [f32; 3],
    pub col: [u8; 4],
}

/// A vector star's vertex: its direction, its colour (sRGB) with its brightness in the fourth byte, and which end it is
/// (0 the head, 1 the tail end) with the head's size at rest in pixels.
#[repr(C)]
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub(crate) struct StarVertex {
    pub dir: [f32; 3],
    pub col: [u8; 4],
    pub end: [f32; 2],
}

/// The sky mesh's vertices from a decoded sky.
pub(crate) fn mesh_vertices(sky: &NebulaSky) -> Vec<SkyVertex> {
    sky.directions.iter().zip(&sky.rgba).map(|(d, c)| SkyVertex { dir: *d, col: *c }).collect()
}

/// The sky with no nebula: an octahedron of black that hides no star, so the program draws the faint stars, the sun
/// and the planet over black as the full-screen sky did before the nebula.
pub(crate) fn octahedron() -> (Vec<SkyVertex>, Vec<u16>) {
    let axes =
        [[1.0, 0.0, 0.0], [-1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, -1.0, 0.0], [0.0, 0.0, 1.0], [0.0, 0.0, -1.0]];
    let v = axes.map(|dir| SkyVertex { dir, col: [0, 0, 0, 0] }).to_vec();
    let mut idx = Vec::new();
    for x in [0u16, 1] {
        for y in [2u16, 3] {
            for z in [4u16, 5] {
                idx.extend_from_slice(&[x, y, z]);
            }
        }
    }
    (v, idx)
}

/// The vector stars' vertices: two a star for the lines (head, tail end), and one a star of a class whose head is 2 px
/// or more for the round heads.
pub(crate) fn star_vertices(sky: &NebulaSky, starfield: &Starfield) -> (Vec<StarVertex>, Vec<StarVertex>) {
    let byte = |x: f64| (x.clamp(0.0, 1.0) * 255.0).round() as u8;
    let mut lines = Vec::with_capacity(2 * sky.stars.len());
    let mut heads = Vec::new();
    for s in &sky.stars {
        let class = &starfield.classes[usize::from(s.class)];
        let c = starfield.colours[usize::from(s.colour)].srgb;
        let col = [byte(c[0]), byte(c[1]), byte(c[2]), byte(f64::from(s.brightness))];
        let size = class.size_px as f32;
        lines.push(StarVertex { dir: s.dir, col, end: [0.0, size] });
        lines.push(StarVertex { dir: s.dir, col, end: [1.0, size] });
        if class.size_px >= 2.0 {
            heads.push(StarVertex { dir: s.dir, col, end: [0.0, size] });
        }
    }
    (lines, heads)
}

/// How one frame of one view draws the stars' tails.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct StreakFrame {
    /// The tail's scale: a star that moved `a` radians this frame has a tail of `min(a * tail_scale, max)`. 0 draws
    /// every star as a point.
    pub tail_scale: f32,
    /// The tail at the fastest-moving point of the view (the view's own turn this frame, scaled and capped), radians.
    pub tail_rad: f32,
    /// The faint hashed stars' brightness this frame, 0-1 (design 5.5): dimmed with the tail as a streak is, and faded
    /// toward `faint_floor` as the tail reaches its cap.
    pub faint: f32,
    /// Whether this frame is a cut (the first frame of a view, a turn faster than `cut_deg_s`, or no time passed): last
    /// frame's rotation is forgotten and no tail is drawn.
    pub cut: bool,
}

/// The view's turn between two rotations (radians): the angle of the rotation that takes `prev` to `now`.
pub fn turn_rad(now: Mat3, prev: Mat3) -> f32 {
    let d = now * prev.transpose();
    let c = ((d.x_axis.x + d.y_axis.y + d.z_axis.z - 1.0) * 0.5).clamp(-1.0, 1.0);
    c.acos()
}

/// The streak for a view whose rotation is `now` at `time_s`, last drawn at `prev` (its rotation and time; none if it
/// was never drawn), by the rule `s` (`data/space/starfield.json`). The tail is scaled to `tail_s` of turning, so it
/// does not depend on the frame rate (design 5.2); a cut draws no tail (FreeSpace's `stars_camera_cut`).
pub fn streak_frame(now: Mat3, prev: Option<(Mat3, f64)>, time_s: f64, s: &Streak) -> StreakFrame {
    let max_rad = s.max_deg.to_radians() as f32;
    let rest = StreakFrame { tail_scale: 0.0, tail_rad: 0.0, faint: 1.0, cut: true };
    let Some((prev, t0)) = prev else { return rest };
    let dt = time_s - t0;
    if !dt.is_finite() || dt <= 1e-6 {
        return rest;
    }
    let turned = turn_rad(now, prev);
    if f64::from(turned) / dt > s.cut_deg_s.to_radians() {
        return rest;
    }
    let tail_scale = (s.tail_s / dt) as f32;
    let tail_rad = (turned * tail_scale).min(max_rad);
    let dim = (1.0 - s.dim_per_deg.to_degrees() as f32 * tail_rad).max(s.dim_floor as f32);
    let share = if max_rad > 0.0 { (tail_rad / max_rad).min(1.0) } else { 0.0 };
    let faint = dim * (1.0 - share * (1.0 - s.faint_floor as f32));
    StreakFrame { tail_scale, tail_rad, faint, cut: false }
}

/// The 3 x 3 rotation of a view matrix (its translation, if any, dropped).
pub(crate) fn rotation_of(view: Mat4) -> Mat3 {
    Mat3::from_mat4(view)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn streak() -> Streak {
        let text = include_str!("../../../data/space/starfield.json");
        sc_core::sky::Starfield::load("data/space/starfield.json", text).expect("the shipped starfield").streak
    }

    fn turned(deg_s: f32, dt: f64) -> (Mat3, Option<(Mat3, f64)>) {
        let now = Mat3::from_rotation_y(0.4);
        let prev = Mat3::from_rotation_y(0.4 - (deg_s.to_radians() * dt as f32));
        (now, Some((prev, 10.0)))
    }

    #[test]
    fn a_view_at_rest_draws_no_tail() {
        let (now, _) = turned(0.0, 1.0 / 60.0);
        let f = streak_frame(now, Some((now, 10.0)), 10.0 + 1.0 / 60.0, &streak());
        assert_eq!(f.tail_rad, 0.0);
        assert_eq!(f.faint, 1.0, "the faint stars stay whole at rest");
        assert!(!f.cut);
    }

    #[test]
    fn the_tail_grows_with_the_turn_rate_and_stops_at_the_cap() {
        let s = streak();
        let at = |deg_s: f32| {
            let (now, prev) = turned(deg_s, 1.0 / 60.0);
            streak_frame(now, prev, 10.0 + 1.0 / 60.0, &s).tail_rad.to_degrees()
        };
        // 10 degrees a second: 0.25 degrees of tail (design 5.2, "the Tern turning at 10 degrees a second").
        assert!((at(10.0) - 0.25).abs() < 0.01, "{}", at(10.0));
        assert!(at(30.0) > at(10.0) && at(60.0) > at(30.0), "longer as the turn is faster");
        // The spec's fast turn: 70 degrees a second draws the 1.7 degree cap, and faster turns no more.
        assert!((at(70.0) - 1.7).abs() < 0.01, "{}", at(70.0));
        assert!((at(400.0) - 1.7).abs() < 1e-4, "capped: {}", at(400.0));
    }

    #[test]
    fn the_tail_is_the_same_at_thirty_and_sixty_frames_a_second() {
        // tail_s, not one frame's motion (design 5.2): FreeSpace's tail changed with the frame rate.
        let s = streak();
        let (n60, p60) = turned(20.0, 1.0 / 60.0);
        let (n30, p30) = turned(20.0, 1.0 / 30.0);
        let a = streak_frame(n60, p60, 10.0 + 1.0 / 60.0, &s).tail_rad;
        let b = streak_frame(n30, p30, 10.0 + 1.0 / 30.0, &s).tail_rad;
        assert!((a - b).abs() < 1e-5, "{a} against {b}");
    }

    #[test]
    fn the_faint_stars_never_fall_under_their_floor() {
        let s = streak();
        let (now, prev) = turned(400.0, 1.0 / 60.0);
        let f = streak_frame(now, prev, 10.0 + 1.0 / 60.0, &s);
        // At the cap: dimmed to dim_floor's 0.5 or more and faded to faint_floor of that.
        let least = s.dim_floor as f32 * s.faint_floor as f32;
        assert!(f.faint >= least - 1e-6 && f.faint < 0.5, "{}", f.faint);
    }

    #[test]
    fn a_cut_draws_no_tail() {
        let s = streak();
        let now = Mat3::from_rotation_y(0.4);
        assert!(streak_frame(now, None, 1.0, &s).cut, "the first frame of a view");
        // Sitting down or a feed switch: the camera jumps 30 degrees in a frame, 1,800 degrees a second.
        let (n, p) = turned(1_800.0, 1.0 / 60.0);
        let f = streak_frame(n, p, 10.0 + 1.0 / 60.0, &s);
        assert!(f.cut && f.tail_scale == 0.0 && f.tail_rad == 0.0);
        assert!(streak_frame(now, Some((now, 2.0)), 2.0, &s).cut, "no time passed");
    }

    #[test]
    fn the_turn_is_the_angle_between_the_rotations() {
        let a = Mat3::from_rotation_x(0.3) * Mat3::from_rotation_y(1.0);
        let b = Mat3::from_axis_angle(glam::Vec3::new(1.0, 2.0, 3.0).normalize(), 0.2) * a;
        assert!((turn_rad(b, a) - 0.2).abs() < 1e-5);
    }

    #[test]
    fn the_octahedron_is_closed() {
        let (v, i) = octahedron();
        assert_eq!((v.len(), i.len() / 3), (6, 8));
    }
}
