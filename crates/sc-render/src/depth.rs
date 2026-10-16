//! How a view of the outside shares the depth buffer (openspec/changes/sky-behind-solids, design 1 and 2).
//!
//! Three layers, back to front: the sky layer (stars, the sun, a planet, and any later star streaks or nebula) at the
//! far plane itself, where it is depth tested and never writes, so it shows only where nothing solid was drawn,
//! whatever the order; the outside's solids (ships, stations, missiles, bolts, debris) through [`outside_lens`]; and,
//! seen through the bridge's windows, the bridge in the front half of the buffer ([`in_front_half`]) with the outside
//! squeezed behind it ([`behind_everything`]). The viewscreen feeds, the chase camera and the bridge's windows all take
//! their numbers from here, so they cannot disagree (CLAUDE.md 6.1).

use glam::{Mat4, Vec4};

/// The outside's near plane, metres.
pub const OUTSIDE_NEAR_M: f32 = 1.0;

/// The outside's far plane, metres. Well past [`SOLID_REACH_M`]: a solid at the far plane would tie with the sky layer
/// in a 24-bit buffer (the last 0.6 % of the range rounds to the sky's depth), so nothing solid is let near it.
pub const OUTSIDE_FAR_M: f32 = 160_000.0;

/// The farthest a solid outside may lie from the eye, metres: a station's draw distance plus its reach plus where the
/// eye stands off the Tern must stay inside it (`sc-client`'s stations check it at load). At this distance a solid is
/// still many depth steps in front of the sky layer through every lens here.
pub const SOLID_REACH_M: f32 = 120_000.0;

// A solid's reach is inside the far plane, with the headroom `SOLID_REACH_M` describes.
const _: () = assert!(SOLID_REACH_M < OUTSIDE_FAR_M * 0.8);

/// The share of the depth buffer the outside is squeezed into behind the bridge, through its windows: half, so a
/// station 2 km off resolves to under a metre (patrol-mission design 12b; it was 0.0001, about 2 km at 2 km).
pub const OUTSIDE_DEPTH: f32 = 0.5;

/// One step of the 24-bit depth buffer (`DEPTH_FORMAT`), as a share of its range.
pub const DEPTH_STEP: f64 = 1.0 / 16_777_215.0;

/// The outside's projection for a vertical field of view `vfov` (radians) and `aspect` (width over height): a
/// viewscreen's feed or the chase camera, and (behind [`behind_everything`]) the outside through the bridge's windows.
pub fn outside_lens(vfov: f32, aspect: f32) -> Mat4 {
    glam::camera::rh::proj::opengl::perspective(vfov, aspect, OUTSIDE_NEAR_M, OUTSIDE_FAR_M)
}

/// The clip-space map that puts a projection's depth into the front `1 - OUTSIDE_DEPTH` of the buffer (OpenGL's -1..1
/// to -1..1 - 2 * OUTSIDE_DEPTH), keeping x, y and w: the bridge's own, ahead of everything outside.
pub fn in_front_half() -> Mat4 {
    let k = 1.0 - OUTSIDE_DEPTH;
    Mat4::from_cols(Vec4::X, Vec4::Y, Vec4::new(0.0, 0.0, k, 0.0), Vec4::new(0.0, 0.0, k - 1.0, 1.0))
}

/// The clip-space map that puts a projection's depth into the last `OUTSIDE_DEPTH` of the buffer (OpenGL's -1..1 to
/// 1 - 2 * OUTSIDE_DEPTH..1), keeping x, y and w.
pub fn behind_everything() -> Mat4 {
    let k = 2.0 * OUTSIDE_DEPTH;
    Mat4::from_cols(Vec4::X, Vec4::Y, Vec4::new(0.0, 0.0, k / 2.0, 0.0), Vec4::new(0.0, 0.0, 1.0 - k / 2.0, 1.0))
}

/// The window depth (0 at the near plane, 1 at the far plane, where the sky layer is) of a point `d_m` metres straight
/// ahead through the projection `clip`.
pub fn window_depth(clip: Mat4, d_m: f32) -> f64 {
    let c = clip * Vec4::new(0.0, 0.0, -d_m, 1.0);
    f64::from(c.z / c.w) * 0.5 + 0.5
}

/// Whether a solid `d_m` metres ahead through `clip` lands in front of the sky layer with room to spare: at least four
/// depth steps short of the far plane, so neither rounding nor the order of the draws lets the sky win.
pub fn in_front_of_sky(clip: Mat4, d_m: f32) -> bool {
    window_depth(clip, d_m) <= 1.0 - 4.0 * DEPTH_STEP
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lenses() -> [(&'static str, Mat4); 2] {
        let feed = outside_lens(70f32.to_radians(), 1228.0 / 388.0);
        [("a feed", feed), ("the bridge's windows", behind_everything() * feed)]
    }

    #[test]
    fn a_solid_at_the_farthest_reach_stands_in_front_of_the_sky_through_every_lens() {
        // Regression (sky-behind-solids 2): the far plane was the stations' draw distance, 80 km, so Tamsin Station,
        // 12.9 km in reach, was cut open by it from 67 km and the stars showed through its hull.
        for (name, m) in lenses() {
            assert!(
                in_front_of_sky(m, SOLID_REACH_M),
                "{name}: {} at {SOLID_REACH_M} m",
                window_depth(m, SOLID_REACH_M)
            );
            assert!(window_depth(m, SOLID_REACH_M) < window_depth(m, SOLID_REACH_M * 1.01), "{name} keeps its order");
        }
    }

    #[test]
    fn a_solid_near_the_far_plane_would_tie_with_the_sky() {
        // Why the far plane has headroom: within its last half percent a solid rounds to the sky's depth.
        let feed = outside_lens(1.0, 1.0);
        assert!(!in_front_of_sky(feed, OUTSIDE_FAR_M * 0.999), "the last 0.1 % of the range is the sky's");
    }

    #[test]
    fn the_outside_keeps_its_resolution_behind_the_bridge() {
        // A metre at 2 km and 5 cm at 200 m are a step or more apart behind the bridge (a station's own parts).
        let far = behind_everything() * outside_lens(70f32.to_radians(), 16.0 / 9.0);
        assert!(window_depth(far, 2_001.0) - window_depth(far, 2_000.0) > DEPTH_STEP, "a metre at 2 km");
        assert!(window_depth(far, 200.05) - window_depth(far, 200.0) > DEPTH_STEP, "5 cm at 200 m");
        assert!(window_depth(far, OUTSIDE_NEAR_M) >= f64::from(1.0 - OUTSIDE_DEPTH) - 1e-6, "all behind the bridge");
    }
}
