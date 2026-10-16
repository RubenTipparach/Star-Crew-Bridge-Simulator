//! The viewscreen's camera rig (openspec/changes/chase-orbit design 3-7): the chase camera's critically damped follow
//! of the ship's turn in the system frame, the ease between cameras, and ORBIT round the ship. One rig is the one camera
//! state a viewscreen has: the 3D feed is drawn from what it gives and the console's bracket is projected through it.
//!
//! It lives with the console because where the camera stands is `console::feed_view`'s and `chase_camera`'s; it holds
//! only what makes the picture move smoothly. The numbers are `data/space/exterior.json`'s `viewscreen.chase`.

use glam::{DMat3, DQuat, DVec3};
use sc_core::exterior::Chase;

use super::{chase_camera, chase_vfov, feed_basis_for, feed_view, Basis, BOW_EYE};

/// One step of a critically damped follow toward zero (chase-orbit design 3): `y` the offset from the target, `v`
/// its rate, over `dt` seconds with time constant `tau_s`. The exact solution of `y'' = -2w y' - w^2 y` (w = 2 / tau)
/// over the step, so any frames of any length that add up to the same time land on the same answer, and from rest it
/// falls to zero without crossing it.
pub fn damp(y: DVec3, v: DVec3, dt: f64, tau_s: f64) -> (DVec3, DVec3) {
    let w = 2.0 / tau_s.max(1e-6);
    let e = (-w * dt).exp();
    let temp = (v + y * w) * dt;
    ((y + temp) * e, (v - temp * w) * e)
}

/// One step of the follow of a target that moved by `moved` over the step, taken as moving evenly through it (the
/// exact solution for a target turning at a steady rate, so the lag behind a steady turn is the same at any frame
/// rate): `y` the offset from where the target was, `v` the rate. Returns the offset from where it is now, and the rate.
pub fn follow(y: DVec3, v: DVec3, moved: DVec3, dt: f64, tau_s: f64) -> (DVec3, DVec3) {
    if dt <= 0.0 {
        return (y - moved, v);
    }
    // Pulled toward a target moving at u, the offset y obeys y'' = -2w (y' + u) - w^2 y; z = y + 2u/w obeys the still
    // target's equation, with rate y'. So the still target's exact step, shifted back.
    let (u, w) = (moved / dt, 2.0 / tau_s.max(1e-6));
    let (z, vz) = damp(y + u * (2.0 / w), v - u, dt, tau_s);
    (z - u * (2.0 / w), vz + u)
}

/// A camera's pose about the ship: where it stands and its orientation (camera axes to ship axes: its right is `-X`,
/// up `Y`, forward `Z`), ship frame, and its vertical field of view, radians.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pose {
    pub eye: DVec3,
    pub rot: DQuat,
    pub vfov: f64,
}

impl Pose {
    fn of(eye: DVec3, b: &Basis, vfov: f64) -> Self {
        let rot = DQuat::from_mat3(&DMat3::from_cols(-b.r, b.u, b.f)).normalize();
        Pose { eye, rot, vfov }
    }

    /// The pose's axes.
    pub fn basis(&self) -> Basis {
        Basis { f: self.rot * DVec3::Z, r: self.rot * DVec3::NEG_X, u: self.rot * DVec3::Y }
    }

    fn mix(&self, to: &Pose, k: f64) -> Pose {
        Pose {
            eye: self.eye.lerp(to.eye, k),
            rot: self.rot.slerp(to.rot, k),
            vfov: self.vfov + (to.vfov - self.vfov) * k,
        }
    }
}

/// What a viewscreen asks of its rig this frame: its camera (`combat::FEEDS`), CHASE, zoom, the locked target relative
/// to the ship (ship frame), and ORBIT's goal, the hand's [heading, elevation, distance] (degrees, degrees, metres).
#[derive(Clone, Debug, PartialEq)]
pub struct Want {
    pub feed: String,
    pub chase: bool,
    pub zoom: f64,
    pub target: Option<DVec3>,
    pub orbit: Option<[f64; 3]>,
}

/// The camera a viewscreen shows this frame (chase-orbit design 7): where it stands relative to the ship's centre and
/// its axes, both in the ship's frame (the lag of the follow already in them), its vertical field of view (radians),
/// and whether the Tern's hull is in the picture (the camera is not wholly the bow camera).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shot {
    pub eye: DVec3,
    pub rot: DQuat,
    pub vfov: f64,
    pub hull: bool,
}

impl Shot {
    /// The shot's axes, ship frame.
    pub fn basis(&self) -> Basis {
        Pose { eye: self.eye, rot: self.rot, vfov: self.vfov }.basis()
    }

    /// The camera a viewscreen shows with no rig to smooth it (a console fixture): `feed_view`'s pose, the rig's
    /// settled one.
    pub fn settled(want: &Want, c: &Chase) -> Self {
        let p = want_pose(want, c, want.orbit);
        Shot { eye: p.eye, rot: p.rot, vfov: p.vfov, hull: want.chase }
    }
}

/// An orbit's look line in ship axes: heading `yaw_deg` from the bow, positive to starboard, and elevation `pitch_deg`.
pub fn orbit_line(yaw_deg: f64, pitch_deg: f64) -> DVec3 {
    let (y, p) = (yaw_deg.to_radians(), pitch_deg.to_radians());
    DVec3::new(-y.sin() * p.cos(), p.sin(), y.cos() * p.cos())
}

/// The heading and elevation of a look line, degrees (the inverse of [`orbit_line`]).
pub fn line_angles(f: DVec3) -> (f64, f64) {
    let f = f.normalize_or(DVec3::Z);
    ((-f.x).atan2(f.z).to_degrees(), f.y.clamp(-1.0, 1.0).asin().to_degrees())
}

/// Where ORBIT starts (chase-orbit design 5): the chase camera's own line and distance, so nothing moves until the
/// player drags. Held within the orbit's limits.
pub fn orbit_start(feed: &str, target: Option<DVec3>, c: &Chase) -> [f64; 3] {
    let (yaw, pitch) = line_angles(feed_basis_for(feed, target).f);
    clamp_orbit([yaw, pitch, c.distance_m], c)
}

/// An orbit held to its limits: elevation within `pitch_limit_deg`, distance within `distance_m`; any heading.
pub fn clamp_orbit(o: [f64; 3], c: &Chase) -> [f64; 3] {
    let lim = c.orbit.pitch_limit_deg;
    let [lo, hi] = c.orbit.distance_m;
    [o[0], o[1].clamp(-lim, lim), o[2].clamp(lo, hi)]
}

/// The pose a viewscreen wants: the orbit `orbit` (heading, elevation, distance) round the ship when given with CHASE
/// on, the chase camera on its line at that distance (design 5); otherwise `feed_view`'s.
fn want_pose(want: &Want, c: &Chase, orbit: Option<[f64; 3]>) -> Pose {
    match orbit.filter(|_| want.chase) {
        Some([yaw, pitch, dist]) => {
            let (eye, b) = chase_camera(orbit_line(yaw, pitch), None, c, chase_vfov(c, 1.0) / 2.0, dist);
            Pose::of(eye, &b, chase_vfov(c, want.zoom))
        }
        None => {
            let (eye, b, vfov) = feed_view(&want.feed, want.chase, want.target, c, want.zoom);
            Pose::of(eye.unwrap_or(BOW_EYE), &b, vfov)
        }
    }
}

/// A rig left this long (a console shown again) starts afresh, seconds.
const STALE_S: f64 = 1.0;

/// A viewscreen's camera rig (chase-orbit design 7). Step it once a frame.
#[derive(Clone, Debug, Default)]
pub struct FeedRig {
    last_s: Option<f64>,
    /// The follow: the rig's rotation in the system frame and its angular rate (system frame, rad/s).
    lag: DQuat,
    lag_v: DVec3,
    /// The ship's rotation at the last step.
    ship: DQuat,
    /// The orbit drawn, [heading, elevation, distance], and its rates.
    orbit: Option<([f64; 3], [f64; 3])>,
    /// What the viewscreen last asked for: camera, CHASE, ORBIT.
    key: Option<(String, bool, bool)>,
    /// An ease under way: the pose and CHASE weight it started from, and when.
    blend: Option<(Pose, f64, f64)>,
    /// The pose last shown (before the follow's lag) and its CHASE weight.
    shown: Option<(Pose, f64)>,
}

impl FeedRig {
    /// The camera at `now_s` (the drill's clock) for a ship turned `ship_rot` (system frame), as `want` asks.
    pub fn step(&mut self, now_s: f64, ship_rot: DQuat, want: &Want, c: &Chase) -> Shot {
        let dt = self.last_s.map_or(f64::INFINITY, |t| now_s - t);
        if !(0.0..=STALE_S).contains(&dt) {
            *self = FeedRig { lag: ship_rot, ship: ship_rot, ..Default::default() };
        }
        let dt = dt.clamp(0.0, STALE_S);
        self.last_s = Some(now_s);
        // The orbit drawn follows the hand's with the follow's damper (design 5); the heading is taken the nearest
        // way round from where it is.
        let orbit = want.orbit.filter(|_| want.chase).map(|goal| {
            let (mut x, mut v) = self.orbit.unwrap_or((goal, [0.0; 3]));
            let mut g = goal;
            g[0] = x[0] + (g[0] - x[0] + 180.0).rem_euclid(360.0) - 180.0;
            let (y, nv) =
                damp(DVec3::from_array(x) - DVec3::from_array(g), DVec3::from_array(v), dt, c.orbit.smooth_tau_s);
            x = (DVec3::from_array(g) + y).to_array();
            v = nv.to_array();
            self.orbit = Some((x, v));
            x
        });
        if orbit.is_none() {
            self.orbit = None;
        }
        let target = want_pose(want, c, orbit);
        // A new camera, CHASE or ORBIT eases from the picture last shown (design 4).
        let key = (want.feed.clone(), want.chase, orbit.is_some());
        let chase_w = if want.chase { 1.0 } else { 0.0 };
        if self.key.as_ref().is_some_and(|k| *k != key) {
            if let Some((p, w)) = self.shown {
                self.blend = Some((p, w, now_s));
            }
        }
        self.key = Some(key);
        let (pose, w) = match self.blend {
            Some((from, w0, t0)) if c.blend_s > 0.0 && now_s - t0 < c.blend_s => {
                let s = ((now_s - t0) / c.blend_s).clamp(0.0, 1.0);
                let k = s * s * (3.0 - 2.0 * s);
                (from.mix(&target, k), w0 + (chase_w - w0) * k)
            }
            _ => {
                self.blend = None;
                (target, chase_w)
            }
        };
        self.shown = Some((pose, w));
        // The follow, in the system frame (design 3): the rig's offset from the ship damped toward none, the ship's
        // turn over the frame taken as even.
        let rotvec = |q: DQuat| if q.w < 0.0 { (-q).to_scaled_axis() } else { q.to_scaled_axis() };
        let off = rotvec(self.lag * self.ship.inverse());
        let moved = rotvec(ship_rot * self.ship.inverse());
        let (y, v) = follow(off, self.lag_v, moved, dt, c.follow_tau_s);
        self.lag = (DQuat::from_scaled_axis(y) * ship_rot).normalize();
        self.lag_v = v;
        self.ship = ship_rot;
        // Seen from the ship, the lag is the rig's turn from the ship's, weighted by how far CHASE is on.
        let rel = if w > 0.0 {
            DQuat::IDENTITY.slerp((ship_rot.inverse() * self.lag).normalize(), w)
        } else {
            DQuat::IDENTITY
        };
        Shot { eye: rel * pose.eye, rot: (rel * pose.rot).normalize(), vfov: pose.vfov, hull: w > 1e-3 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sc_core::exterior::ExteriorData;

    fn cfg() -> Chase {
        ExteriorData::shipped().viewscreen.chase
    }

    fn want(feed: &str, chase: bool) -> Want {
        Want { feed: feed.into(), chase, zoom: 1.0, target: None, orbit: None }
    }

    fn angle_deg(a: DQuat, b: DQuat) -> f64 {
        a.angle_between(b).to_degrees()
    }

    /// The rig's turn in the system frame after following a ship that yaws at `rate_dps` for `secs` at `fps`.
    fn follow(fps: f64, secs: f64, rate_dps: f64) -> (DQuat, DQuat) {
        let c = cfg();
        let mut rig = FeedRig::default();
        let w = want("FWD", true);
        let n = (secs * fps).round() as usize;
        let mut ship = DQuat::IDENTITY;
        let mut shot = rig.step(0.0, ship, &w, &c);
        for i in 1..=n {
            let t = i as f64 / fps;
            ship = DQuat::from_rotation_y((rate_dps * t).to_radians());
            shot = rig.step(t, ship, &w, &c);
        }
        // The camera's orientation in the system frame.
        (ship, ship * shot.rot)
    }

    #[test]
    fn the_follow_settles_the_time_constant_of_the_turn_behind() {
        let c = cfg();
        let (ship, cam) = follow(60.0, 6.0, 18.0);
        let still = ship * want_pose(&want("FWD", true), &c, None).rot;
        let lag = angle_deg(cam, still);
        let expect = c.follow_tau_s * 18.0;
        assert!((lag - expect).abs() < 0.3, "at 18 degrees a second it trails {expect} degrees: {lag}");
    }

    #[test]
    fn the_follow_is_the_same_at_any_frame_rate() {
        for fps in [20.0, 30.0, 144.0] {
            let (_, a) = follow(fps, 2.0, 18.0);
            let (_, b) = follow(240.0, 2.0, 18.0);
            assert!(
                angle_deg(a, b) < 0.05,
                "{fps} frames a second lands within 0.05 degrees of 240: {}",
                angle_deg(a, b)
            );
        }
        // A still target: the exact solution, frame for frame.
        let (y0, v0) = (DVec3::new(0.3, -0.1, 0.2), DVec3::ZERO);
        let one = damp(y0, v0, 0.5, 0.4);
        let mut many = (y0, v0);
        for _ in 0..37 {
            many = damp(many.0, many.1, 0.5 / 37.0, 0.4);
        }
        assert!((one.0 - many.0).length() < 1e-12 && (one.1 - many.1).length() < 1e-12, "exact over any steps");
    }

    #[test]
    fn the_follow_converges_without_overshooting() {
        // The ship turns 40 degrees at once and holds: the camera swings after it and stops on it, never past.
        let c = cfg();
        let mut rig = FeedRig::default();
        let w = want("FWD", true);
        rig.step(0.0, DQuat::IDENTITY, &w, &c);
        let ship = DQuat::from_rotation_y(40f64.to_radians());
        let mut last = f64::INFINITY;
        for i in 1..=180 {
            let shot = rig.step(i as f64 / 60.0, ship, &w, &c);
            // Seen from the ship, the camera's turn back toward where it was: falls to zero, never changes sign.
            let behind = (shot.rot * want_pose(&w, &c, None).rot.inverse()).to_scaled_axis().y.to_degrees();
            assert!(behind <= 1e-9, "never past the ship's turn: {behind} at frame {i}");
            assert!(-behind <= last + 1e-9, "never swinging back out: {behind}");
            last = -behind;
        }
        assert!(last < 0.05, "settled after 3 s: {last} degrees");
    }

    #[test]
    fn chase_off_is_the_bow_camera_however_the_ship_turns() {
        let c = cfg();
        let mut rig = FeedRig::default();
        let w = want("FWD", false);
        for i in 0..60 {
            let ship = DQuat::from_rotation_y(i as f64 * 0.05);
            let shot = rig.step(i as f64 / 30.0, ship, &w, &c);
            assert!((shot.eye - BOW_EYE).length() < 1e-9, "on the bow");
            assert!(!shot.hull, "no hull from the bow");
            assert!(angle_deg(shot.rot, DQuat::IDENTITY) < 1e-9, "the bow camera does not lag");
        }
    }

    #[test]
    fn chase_pressed_eases_from_the_bow_to_behind_the_ship() {
        let c = cfg();
        let mut rig = FeedRig::default();
        let bow = rig.step(0.0, DQuat::IDENTITY, &want("FWD", false), &c);
        let end = want_pose(&want("FWD", true), &c, None);
        // CHASE pressed at 0.1 s.
        rig.step(0.1, DQuat::IDENTITY, &want("FWD", true), &c);
        let mid = rig.step(0.1 + c.blend_s / 2.0, DQuat::IDENTITY, &want("FWD", true), &c);
        assert!(mid.hull, "the hull comes into the picture as it eases out");
        let (d0, d1) = ((mid.eye - bow.eye).length(), (mid.eye - end.eye).length());
        assert!(d0 > 1.0 && d1 > 1.0, "half way it is between the two: {d0} m from the bow, {d1} m from chase");
        let done = rig.step(0.1 + c.blend_s + 0.01, DQuat::IDENTITY, &want("FWD", true), &c);
        assert!((done.eye - end.eye).length() < 1e-9, "and then behind the ship");
    }

    #[test]
    fn the_orbit_goes_all_the_way_round_and_holds_short_of_the_poles() {
        let c = cfg();
        let o = clamp_orbit([540.0, 120.0, 5.0], &c);
        assert_eq!(
            o,
            [540.0, c.orbit.pitch_limit_deg, c.orbit.distance_m[0]],
            "any heading; elevation and distance held"
        );
        let o = clamp_orbit([0.0, -120.0, 1e5], &c);
        assert_eq!((o[1], o[2]), (-c.orbit.pitch_limit_deg, c.orbit.distance_m[1]));
        // Round the ship: from every heading the camera looks at it, from the orbit's distance.
        for yaw in (0..360).step_by(30) {
            let mut w = want("FWD", true);
            w.orbit = Some([f64::from(yaw), -20.0, 200.0]);
            let p = want_pose(&w, &c, w.orbit);
            let b = p.basis();
            assert!(b.f.dot(-p.eye.normalize()) > 0.9, "heading {yaw}: it looks at the ship");
            let along = -p.eye.dot(orbit_line(f64::from(yaw), -20.0));
            assert!((along - 200.0).abs() < 1.0, "heading {yaw}: 200 m back along its line, {along}");
        }
    }

    #[test]
    fn orbit_starts_where_the_chase_camera_is() {
        let c = cfg();
        let t = Some(DVec3::new(400.0, 80.0, 1500.0));
        for feed in ["FWD", "PORT", "AFT", "STBD"] {
            let mut w = want(feed, true);
            w.target = t;
            let chase = want_pose(&w, &c, None);
            let start = orbit_start(feed, t, &c);
            let orbit = want_pose(&w, &c, Some(start));
            assert!((orbit.eye - chase.eye).length() < 1e-6, "{feed}: ORBIT on moves nothing");
            assert!(angle_deg(orbit.rot, chase.rot) < 1e-6, "{feed}: nor turns");
        }
    }

    #[test]
    fn the_orbit_glides_to_the_hands_goal() {
        let c = cfg();
        let mut rig = FeedRig::default();
        let mut w = want("FWD", true);
        w.orbit = Some([0.0, 0.0, 160.0]);
        rig.step(0.0, DQuat::IDENTITY, &w, &c);
        w.orbit = Some([350.0, 0.0, 160.0]);
        let s1 = rig.step(1.0 / 60.0, DQuat::IDENTITY, &w, &c);
        let (yaw, _) = line_angles(s1.basis().f);
        assert!(yaw < 0.0 && yaw > -2.0, "350 is 10 degrees the short way round, a step at a time: {yaw}");
        for i in 2..120 {
            rig.step(i as f64 / 60.0, DQuat::IDENTITY, &w, &c);
        }
        let s = rig.step(2.0, DQuat::IDENTITY, &w, &c);
        let goal = want_pose(&w, &c, w.orbit);
        assert!((s.eye - goal.eye).length() < 0.1, "there in two seconds");
    }
}
