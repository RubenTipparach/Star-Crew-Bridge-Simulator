//! A star system (`data/space/systems/<id>.json`; openspec/changes/patrol-mission design 2, flight-and-navigation 13):
//! the star, the bodies, the stations with their berths, and the points of interest a patrol visits, in the system
//! frame (origin at the star's centre, metres, `f64`).
//!
//! It lives in the core because the server's patrol jumps and checks mass locks against it and the client draws the
//! sky from it: one file, one loader (CLAUDE.md 6.5). A berth is a pose in its station's frame and a departure path in
//! its own frame, so the berth's rules never depend on what the station looks like.

use glam::{DQuat, DVec3};
use serde::Deserialize;

use crate::combat::attitude::from_hpr;
use crate::data::{Checks, Validate};

/// The least distance an arrival point keeps from every station and point of interest, metres (flight-and-navigation
/// 7: "authored at least 5 km from anything").
pub const ARRIVAL_CLEAR_M: f64 = 5_000.0;

/// The whole file.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SystemFile {
    /// `starcrew.system/1`.
    pub schema: String,
    /// A stable id.
    pub id: String,
    /// Its name in a sentence ("the Shoals").
    pub name: String,
    /// The star at the origin.
    pub star: Star,
    /// Planets and moons.
    pub bodies: Vec<Body>,
    /// Stations.
    pub stations: Vec<Station>,
    /// Points of interest.
    pub points: Vec<Point>,
}

/// The star, at the system's origin.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Star {
    /// Its name.
    pub name: String,
    /// Its radius, metres.
    pub radius_m: f64,
    /// Its colour, sRGB 0-1.
    pub colour_srgb: [f64; 3],
    /// How bright the glow round its disc is, 0-1.
    pub glow: f64,
    /// The distance at which its light is 1, metres.
    pub light_ref_m: f64,
    /// Its light falls as `(light_ref_m / d) ^ light_exponent` (a softened inverse square, design 11.2).
    pub light_exponent: f64,
    /// The least and the most its light is held to.
    pub light_min: f64,
    pub light_max: f64,
}

/// How a body looks: the sky program's planet (`data/space/exterior.json`'s fields).
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Look {
    /// Sea, sRGB 0-1.
    pub ocean_srgb: [f64; 3],
    /// Land, sRGB 0-1.
    pub land_srgb: [f64; 3],
    /// Cloud, sRGB 0-1.
    pub cloud_srgb: [f64; 3],
    /// The atmosphere's rim, sRGB 0-1.
    pub atmosphere_srgb: [f64; 3],
    /// The atmosphere's thickness as a share of the radius.
    pub atmosphere_frac: f64,
    /// The share of land, 0-1.
    pub land_frac: f64,
    /// The share of cloud, 0-1.
    pub cloud_frac: f64,
    /// The night side's brightness, 0-1.
    pub night: f64,
}

/// A planet or a moon.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Body {
    /// A stable id.
    pub id: String,
    /// Its name.
    pub name: String,
    /// Its centre, system frame, metres.
    pub pos_m: [f64; 3],
    /// Its radius, metres.
    pub radius_m: f64,
    /// Its look.
    pub look: Look,
}

/// A station.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Station {
    /// A stable id.
    pub id: String,
    /// Its name.
    pub name: String,
    /// What it is (`starbase`, `relay`): which model the client draws.
    pub kind: String,
    /// Its centre, system frame, metres.
    pub pos_m: [f64; 3],
    /// Its attitude [heading, pitch, roll], degrees (`attitude::from_hpr`): its frame into the system's.
    pub hpr_deg: [f64; 3],
    /// No jump spools within this of its centre, metres (flight-and-navigation 7).
    pub mass_lock_m: f64,
    /// Where a jump to it ends, in its own frame from its centre, metres.
    pub arrival_m: [f64; 3],
    /// Its berths.
    pub berths: Vec<Berth>,
}

/// A berth: where the docked Tern's origin sits and its attitude, in the station's frame, and the path out.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Berth {
    /// A stable id.
    pub id: String,
    /// The docked Tern's origin, station frame, metres.
    pub pos_m: [f64; 3],
    /// The docked Tern's attitude in the station's frame [heading, pitch, roll], degrees.
    pub hpr_deg: [f64; 3],
    /// The way out, in the berth's frame (the docked Tern's), leg by leg.
    pub departure: Vec<Leg>,
}

/// A leg of a departure path.
#[derive(Debug, Clone, Copy, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Leg {
    /// Where it ends, the berth's frame, metres.
    pub to_m: [f64; 3],
    /// The most it is flown at, m/s.
    pub max_mps: f64,
}

/// A point of interest.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Point {
    /// A stable id.
    pub id: String,
    /// Its name in a sentence ("the Pell beacon").
    pub name: String,
    /// Where it is, system frame, metres.
    pub pos_m: [f64; 3],
    /// Where a jump to it ends, an offset from it in the system's axes, metres.
    pub arrival_m: [f64; 3],
}

/// A place a jump can go: a station or a point, by what it is called and where.
#[derive(Debug, Clone, PartialEq)]
pub struct Place {
    /// Its id.
    pub id: String,
    /// Its name in a sentence.
    pub name: String,
    /// Where it is, system frame.
    pub pos: DVec3,
    /// Where a jump to it ends, system frame.
    pub arrival: DVec3,
}

/// The star and the near planet as seen from a point (design 11.2): what the sky program draws.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SkyView {
    /// The way to the star, unit, system frame.
    pub sun_dir: DVec3,
    /// The sine of the star's angular radius.
    pub sun_sin_r: f64,
    /// The star's light, a multiplier on its colour.
    pub light: f64,
    /// The body with the largest angular radius: its index, the way to it (unit) and the sine of its angular radius.
    pub planet: Option<(usize, DVec3, f64)>,
}

fn v(a: [f64; 3]) -> DVec3 {
    DVec3::from_array(a)
}

fn hpr(a: [f64; 3]) -> DQuat {
    from_hpr(a[0], a[1], a[2])
}

impl SystemFile {
    /// A station by id.
    pub fn station(&self, id: &str) -> Option<&Station> {
        self.stations.iter().find(|s| s.id == id)
    }

    /// A station's attitude, its frame into the system's.
    pub fn station_rot(&self, s: &Station) -> DQuat {
        hpr(s.hpr_deg)
    }

    /// A berth's pose in the system frame: where the docked Tern's origin is and its attitude.
    pub fn berth_pose(&self, s: &Station, b: &Berth) -> (DVec3, DQuat) {
        let rot = self.station_rot(s);
        (v(s.pos_m) + rot * v(b.pos_m), (rot * hpr(b.hpr_deg)).normalize())
    }

    /// A station's or a point's place, by id.
    pub fn place(&self, id: &str) -> Option<Place> {
        if let Some(s) = self.station(id) {
            let pos = v(s.pos_m);
            return Some(Place {
                id: s.id.clone(),
                name: s.name.clone(),
                pos,
                arrival: pos + self.station_rot(s) * v(s.arrival_m),
            });
        }
        self.points.iter().find(|p| p.id == id).map(|p| Place {
            id: p.id.clone(),
            name: p.name.clone(),
            pos: v(p.pos_m),
            arrival: v(p.pos_m) + v(p.arrival_m),
        })
    }

    /// What locks a jump at `pos` (system frame): the name of the first body within `planet_radii` of its centre or
    /// station within its own lock, in the file's order; none when the way is clear (flight-and-navigation 7).
    pub fn mass_lock(&self, pos: DVec3, planet_radii: f64) -> Option<String> {
        for b in &self.bodies {
            if (pos - v(b.pos_m)).length() < b.radius_m * planet_radii {
                return Some(b.name.clone());
            }
        }
        self.stations.iter().find(|s| (pos - v(s.pos_m)).length() < s.mass_lock_m).map(|s| s.name.clone())
    }

    /// The star and the near planet seen from `eye` (system frame), design 11.2.
    pub fn sky_from(&self, eye: DVec3) -> SkyView {
        let st = &self.star;
        let d = eye.length().max(st.radius_m * 1.01);
        let light = (st.light_ref_m / d).powf(st.light_exponent).clamp(st.light_min, st.light_max);
        let mut planet: Option<(usize, DVec3, f64)> = None;
        for (i, b) in self.bodies.iter().enumerate() {
            let to = v(b.pos_m) - eye;
            let dist = to.length().max(b.radius_m * 1.0001);
            let sin_r = (b.radius_m / dist).min(0.9999);
            if planet.is_none_or(|(_, _, s)| sin_r > s) {
                planet = Some((i, to / dist, sin_r));
            }
        }
        SkyView { sun_dir: (-eye).normalize_or(DVec3::Y), sun_sin_r: (st.radius_m / d).min(0.9999), light, planet }
    }
}

fn finite(c: &mut Checks, field: &str, a: [f64; 3], lim: f64) {
    for (k, x) in a.iter().enumerate() {
        c.number(&format!("{field}[{k}]"), *x, -lim, lim);
    }
}

fn colour(c: &mut Checks, field: &str, a: [f64; 3]) {
    for (k, x) in a.iter().enumerate() {
        c.number(&format!("{field}[{k}]"), *x, 0.0, 1.0);
    }
}

/// The system's extent, metres: 100 AU.
const SYSTEM_LIM_M: f64 = 1.5e13;

impl Validate for SystemFile {
    fn validate(&self, c: &mut Checks) {
        c.equals("schema", &self.schema, "starcrew.system/1");
        let s = &self.star;
        c.number("star.radius_m", s.radius_m, 1e6, 1e11);
        colour(c, "star.colour_srgb", s.colour_srgb);
        c.number("star.glow", s.glow, 0.0, 1.0);
        c.number("star.light_ref_m", s.light_ref_m, 1e8, SYSTEM_LIM_M);
        c.number("star.light_exponent", s.light_exponent, 0.0, 2.0);
        c.number("star.light_min", s.light_min, 0.0, 2.0);
        c.number("star.light_max", s.light_max, s.light_min, 4.0);
        for (i, b) in self.bodies.iter().enumerate() {
            let f = format!("bodies[{i}]");
            finite(c, &format!("{f}.pos_m"), b.pos_m, SYSTEM_LIM_M);
            c.number(&format!("{f}.radius_m"), b.radius_m, 1e3, 1e9);
            let l = &b.look;
            for (k, x) in [("ocean_srgb", l.ocean_srgb), ("land_srgb", l.land_srgb), ("cloud_srgb", l.cloud_srgb)] {
                colour(c, &format!("{f}.look.{k}"), x);
            }
            colour(c, &format!("{f}.look.atmosphere_srgb"), l.atmosphere_srgb);
            c.number(&format!("{f}.look.atmosphere_frac"), l.atmosphere_frac, 0.0, 0.5);
            for (k, x) in [("land_frac", l.land_frac), ("cloud_frac", l.cloud_frac), ("night", l.night)] {
                c.number(&format!("{f}.look.{k}"), x, 0.0, 1.0);
            }
        }
        for (i, s) in self.stations.iter().enumerate() {
            let f = format!("stations[{i}]");
            finite(c, &format!("{f}.pos_m"), s.pos_m, SYSTEM_LIM_M);
            finite(c, &format!("{f}.hpr_deg"), s.hpr_deg, 360.0);
            c.number(&format!("{f}.mass_lock_m"), s.mass_lock_m, 0.0, 1e6);
            finite(c, &format!("{f}.arrival_m"), s.arrival_m, 1e6);
            let a = DVec3::from_array(s.arrival_m).length();
            c.number(&format!("{f}.arrival_m (distance)"), a, ARRIVAL_CLEAR_M, 1e6);
            if !["starbase", "relay"].contains(&s.kind.as_str()) {
                c.equals(&format!("{f}.kind"), &s.kind, "starbase or relay");
            }
            c.count(&format!("{f}.berths"), s.berths.len() as i64, 0, 16);
            for (j, b) in s.berths.iter().enumerate() {
                let g = format!("{f}.berths[{j}]");
                finite(c, &format!("{g}.pos_m"), b.pos_m, 1e5);
                finite(c, &format!("{g}.hpr_deg"), b.hpr_deg, 360.0);
                c.count(&format!("{g}.departure"), b.departure.len() as i64, 1, 32);
                for (k, l) in b.departure.iter().enumerate() {
                    finite(c, &format!("{g}.departure[{k}].to_m"), l.to_m, 1e5);
                    c.number(&format!("{g}.departure[{k}].max_mps"), l.max_mps, 0.1, 50.0);
                }
            }
        }
        for (i, p) in self.points.iter().enumerate() {
            let f = format!("points[{i}]");
            finite(c, &format!("{f}.pos_m"), p.pos_m, SYSTEM_LIM_M);
            finite(c, &format!("{f}.arrival_m"), p.arrival_m, 1e6);
            c.number(&format!("{f}.arrival_m (distance)"), v(p.arrival_m).length(), ARRIVAL_CLEAR_M, 1e6);
        }
        // Ids are unique across stations and points: a route names either.
        let mut ids: Vec<&str> = self.stations.iter().map(|s| s.id.as_str()).collect();
        ids.extend(self.points.iter().map(|p| p.id.as_str()));
        ids.extend(self.bodies.iter().map(|b| b.id.as_str()));
        let mut sorted = ids.clone();
        sorted.sort_unstable();
        sorted.dedup();
        c.count("ids (unique)", sorted.len() as i64, ids.len() as i64, ids.len() as i64);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::parse;
    use crate::exterior::ExteriorData;

    const FILE: &str = include_str!("../../../data/space/systems/shoals.json");

    fn shoals() -> SystemFile {
        parse("data/space/systems/shoals.json", FILE).expect("the shipped system loads")
    }

    #[test]
    fn the_shipped_system_is_valid() {
        let s = shoals();
        assert!(s.station("tamsin_station").is_some() && s.place("shoals_relay").is_some());
    }

    #[test]
    fn the_berth_shows_the_docks_sun_and_planet_where_the_dock_view_has_them() {
        // The berth's attitude is the one that keeps the star and Tamsin where data/space/exterior.json's dock view has
        // them (design 2), checked here on both files rather than typed twice.
        let s = shoals();
        let st = s.station("tamsin_station").unwrap();
        let (pos, rot) = s.berth_pose(st, &st.berths[0]);
        let e: ExteriorData =
            parse("data/space/exterior.json", include_str!("../../../data/space/exterior.json")).unwrap();
        let sky = s.sky_from(pos);
        let sun_ship = rot.inverse() * sky.sun_dir;
        let (_, planet_dir, _) = sky.planet.expect("Tamsin is the near planet at the berth");
        let planet_ship = rot.inverse() * planet_dir;
        let want = |d: [f64; 3]| DVec3::from_array(d).normalize();
        let deg = |a: DVec3, b: DVec3| a.angle_between(b).to_degrees();
        assert!(deg(sun_ship, want(e.sun.dir)) < 1.0, "the sun is {} degrees off", deg(sun_ship, want(e.sun.dir)));
        assert!(
            deg(planet_ship, want(e.planet.dir)) < 1.0,
            "Tamsin is off by {}",
            deg(planet_ship, want(e.planet.dir))
        );
    }

    #[test]
    fn tamsin_from_the_station_spans_seventeen_and_a_half_degrees() {
        let s = shoals();
        let sky = s.sky_from(DVec3::from_array(s.station("tamsin_station").unwrap().pos_m));
        let r = sky.planet.unwrap().2.asin().to_degrees();
        assert!((r - 17.5).abs() < 0.2, "{r} degrees: the station sits 3.33 radii out (design 2)");
    }

    #[test]
    fn the_station_locks_a_jump_and_its_arrival_point_does_not() {
        let s = shoals();
        let home = s.place("tamsin_station").unwrap();
        assert_eq!(s.mass_lock(home.pos + DVec3::X * 1_000.0, 3.0).as_deref(), Some("Tamsin Station"));
        for id in ["tamsin_station", "pell_beacon", "shoals_relay"] {
            let p = s.place(id).unwrap();
            assert_eq!(s.mass_lock(p.arrival, 3.0), None, "{id}'s arrival point must be clear of every lock");
        }
    }

    #[test]
    fn the_star_is_dimmer_and_smaller_at_the_shoals() {
        let s = shoals();
        let at = |id: &str| s.sky_from(s.place(id).unwrap().arrival);
        let (home, far) = (at("tamsin_station"), at("shoals_relay"));
        assert!(far.light < home.light && far.sun_sin_r < home.sun_sin_r);
        assert!(far.light >= s.star.light_min, "the softened inverse square keeps a far point lit");
    }

    #[test]
    fn orrin_fills_the_sky_at_the_pell_beacon() {
        let s = shoals();
        let sky = s.sky_from(s.place("pell_beacon").unwrap().arrival);
        let (i, _, sin_r) = sky.planet.unwrap();
        assert_eq!(s.bodies[i].id, "orrin");
        assert!((sin_r.asin().to_degrees() - 16.9).abs() < 0.3);
    }

    #[test]
    fn an_arrival_point_too_close_to_its_point_is_refused() {
        let mut v: serde_json::Value = serde_json::from_str(FILE).unwrap();
        v["points"][0]["arrival_m"] = serde_json::json!([100.0, 0.0, 0.0]);
        let err = parse::<SystemFile>("x", &v.to_string()).expect_err("an arrival 100 m from its point");
        assert_eq!(err.field, "points[0].arrival_m (distance)");
    }
}
