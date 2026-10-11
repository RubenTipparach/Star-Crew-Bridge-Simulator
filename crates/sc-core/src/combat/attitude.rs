//! Attitude: heading, pitch and roll against the quaternion, and the slew that turns a ship to an attitude
//! (flight-and-navigation section 6a).
//!
//! It lives in the core because the helm's attitude orders and pointing modes resolve with it on the server, and
//! the console's preview of an order (its time, the GO button's seconds) is computed by the same function
//! (CLAUDE.md 6.1). The console mockup (`docs/mockups/consoles.html`, `quat.fromEuler`, `quat.toEuler`, `bowOn`,
//! `slewPlan`) is where these were first written; this is that code, in the engine.
//!
//! Axes are the layout's: ship +X port, +Y dorsal, +Z bow; an attitude maps ship axes to the reference axes. Heading
//! is to starboard, pitch nose up, roll starboard down, in degrees.

use super::data::FlightBlock;
use glam::{DQuat, DVec3};

/// A rotation of `deg` about a unit axis.
fn axis(ax: DVec3, deg: f64) -> DQuat {
    DQuat::from_axis_angle(ax, deg.to_radians())
}

/// Degrees wrapped to 0..360.
pub fn wrap360(a: f64) -> f64 {
    a.rem_euclid(360.0)
}

/// The attitude of a heading, pitch and roll: Ry(-h) Rx(-p) Rz(r).
pub fn from_hpr(h: f64, p: f64, r: f64) -> DQuat {
    (axis(DVec3::Y, -h) * axis(DVec3::X, -p) * axis(DVec3::Z, r)).normalize()
}

/// Heading, pitch and roll of an attitude in degrees, and whether the pitch is at +/-90 (the roll then folds into
/// the heading and is reported as 0).
pub fn hpr_of(q: DQuat) -> (f64, f64, f64, bool) {
    let f = q * DVec3::Z;
    let u = q * DVec3::Y;
    let s = q * DVec3::NEG_X;
    let p = f.y.clamp(-1.0, 1.0).asin().to_degrees();
    if f.y.abs() > 0.99999 {
        let h = if f.y > 0.0 { u.x.atan2(-u.z) } else { (-u.x).atan2(u.z) };
        return (wrap360(h.to_degrees()), p, 0.0, true);
    }
    (wrap360((-f.x).atan2(f.z).to_degrees()), p, (-s.y).atan2(u.y).to_degrees(), false)
}

/// The attitude whose bow points along `d` (reference axes) with its wings level.
pub fn bow_on(d: DVec3) -> DQuat {
    let f = d.normalize_or(DVec3::Z);
    from_hpr(wrap360((-f.x).atan2(f.z).to_degrees()), f.y.clamp(-1.0, 1.0).asin().to_degrees(), 0.0)
}

/// An order's angle in whole degrees and its range: heading 0-359 (`'h'`), pitch -90..+90 (`'p'`), roll -179..+180.
pub fn whole_deg(k: char, v: f64) -> f64 {
    let v = (v + 0.5).floor();
    match k {
        'h' => v.rem_euclid(360.0),
        'p' => v.clamp(-90.0, 90.0),
        _ => (v + 179.0).rem_euclid(360.0) - 179.0,
    }
}

/// A slew to an attitude: the body rates to ask for now, the angle left, the time it takes, and whether the ship is
/// there and still (held).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Slew {
    /// Body rates to ask for, rad/s, ship axes.
    pub w: DVec3,
    /// The angle left, degrees.
    pub angle_deg: f64,
    /// Seconds to get there from rest at the rate and acceleration limits.
    pub time_s: f64,
    /// Inside the hold tolerance and turning slower than its rate tolerance.
    pub held: bool,
}

/// The rate and acceleration limits as body axes [x (pitch), y (yaw), z (roll)], rad/s and rad/s^2. The data writes
/// them [yaw, pitch, roll].
fn body_limits(flight: &FlightBlock) -> (DVec3, DVec3) {
    let r = flight.rate_limit_deg_s.map(f64::to_radians);
    let a = flight.ang_accel_deg_s2.map(f64::to_radians);
    (DVec3::new(r[1], r[0], r[2]), DVec3::new(a[1], a[0], a[2]))
}

/// The slew from `rot` (turning at `rates`) to `goal`: the shortest single-axis turn, its rate set point and its time
/// (flight-and-navigation 6a). The helm's orders and pointing modes fly it; the console's GO shows its time.
pub fn slew(rot: DQuat, rates: DVec3, goal: DQuat, flight: &FlightBlock) -> Slew {
    let mut e = rot.conjugate() * goal;
    if e.w < 0.0 {
        e = -e;
    }
    let ang = 2.0 * e.w.clamp(-1.0, 1.0).acos();
    let s = DVec3::new(e.x, e.y, e.z).length();
    let held_rate = flight.held_dps.to_radians();
    if s < 1e-9 {
        return Slew { w: DVec3::ZERO, angle_deg: 0.0, time_s: 0.0, held: rates.length() < held_rate };
    }
    let u = DVec3::new(e.x, e.y, e.z) / s;
    let (rl, al) = body_limits(flight);
    let (mut rmax, mut amax) = (f64::INFINITY, f64::INFINITY);
    for i in 0..3 {
        if u[i].abs() > 1e-6 {
            rmax = rmax.min(rl[i] / u[i].abs());
            amax = amax.min(al[i] / u[i].abs());
        }
    }
    let time_s = if ang > rmax * rmax / amax { ang / rmax + rmax / amax } else { 2.0 * (ang / amax).sqrt() };
    let inside = ang < flight.held_deg.to_radians();
    let w = if inside { DVec3::ZERO } else { u * rmax.min((1.6 * amax * ang).sqrt()) };
    Slew { w, angle_deg: ang.to_degrees(), time_s, held: inside && rates.length() < held_rate }
}

/// The stick [yaw to port, pitch up, roll port up] in -1..1 that asks the full assist for body rates `w`.
pub fn stick_for(w: DVec3, flight: &FlightBlock) -> [f64; 3] {
    let (rl, _) = body_limits(flight);
    [(w.y / rl.y).clamp(-1.0, 1.0), (-w.x / rl.x).clamp(-1.0, 1.0), (w.z / rl.z).clamp(-1.0, 1.0)]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat::data::DrillData;

    #[test]
    fn heading_pitch_and_roll_round_trip_through_the_quaternion() {
        for (h, p, r) in [(20.0, 0.0, 0.0), (45.0, 10.0, -30.0), (300.0, -60.0, 170.0)] {
            let (h2, p2, r2, g) = hpr_of(from_hpr(h, p, r));
            assert!(!g);
            assert!((h2 - h).abs() < 1e-6 && (p2 - p).abs() < 1e-6 && (r2 - r).abs() < 1e-6);
        }
    }

    #[test]
    fn a_heading_turns_the_bow_to_starboard() {
        // Starboard is -X: heading 090 puts the bow along -X.
        let f = from_hpr(90.0, 0.0, 0.0) * DVec3::Z;
        assert!((f - DVec3::NEG_X).length() < 1e-9);
    }

    #[test]
    fn a_slew_takes_longer_for_a_wider_turn_and_is_held_on_arrival() {
        let flight = DrillData::shipped().tern_flight;
        let a = slew(DQuat::IDENTITY, DVec3::ZERO, from_hpr(30.0, 0.0, 0.0), &flight);
        let b = slew(DQuat::IDENTITY, DVec3::ZERO, from_hpr(120.0, 0.0, 0.0), &flight);
        assert!((a.angle_deg - 30.0).abs() < 1e-6 && b.time_s > a.time_s);
        // A yaw of 120 deg at 18 deg/s and 12 deg/s^2: 120/18 + 18/12 = 8.17 s.
        assert!((b.time_s - (120.0 / 18.0 + 18.0 / 12.0)).abs() < 1e-6, "{}", b.time_s);
        let here = slew(from_hpr(30.0, 0.0, 0.0), DVec3::ZERO, from_hpr(30.0, 0.0, 0.0), &flight);
        assert!(here.held && here.w == DVec3::ZERO);
    }

    #[test]
    fn whole_degrees_wrap_as_the_thumbwheels_read() {
        assert_eq!(whole_deg('h', -1.0), 359.0);
        assert_eq!(whole_deg('p', 95.0), 90.0);
        assert_eq!(whole_deg('r', 181.0), -179.0);
        assert_eq!(whole_deg('r', 180.0), 180.0);
    }
}
