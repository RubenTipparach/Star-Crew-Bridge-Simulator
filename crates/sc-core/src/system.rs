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

/// A station: one of the spacedock versions (openspec/changes/spacedock design 11a), in orbit about a body or near a
/// point of interest.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Station {
    /// A stable id.
    pub id: String,
    /// Its name.
    pub name: String,
    /// Which spacedock version it is (`A`-`E`, `data/space/spacedock.json`): its shape, its bay, its model.
    pub version: String,
    /// The body it orbits, by id; with `orbit_radius_m` and `orbit_dir`.
    #[serde(default)]
    pub body: Option<String>,
    /// Its orbit's radius about the body's centre, metres.
    #[serde(default)]
    pub orbit_radius_m: Option<f64>,
    /// Where on the orbit it stands at the mission's epoch: the way from the body's centre, any length.
    #[serde(default)]
    pub orbit_dir: Option<[f64; 3]>,
    /// The point of interest it stands near, by id; with `offset_m`.
    #[serde(default)]
    pub near: Option<String>,
    /// Its offset from that point, system axes, metres.
    #[serde(default)]
    pub offset_m: Option<[f64; 3]>,
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

    /// A station's centre (its frame's origin), system frame: its body's centre and its orbit, or its point and its
    /// offset. Validated at load: the body or point exists.
    pub fn station_pos(&self, s: &Station) -> DVec3 {
        if let (Some(b), Some(r), Some(d)) = (&s.body, s.orbit_radius_m, s.orbit_dir) {
            let body = self.bodies.iter().find(|x| &x.id == b).map_or(DVec3::ZERO, |x| v(x.pos_m));
            return body + v(d).normalize_or_zero() * r;
        }
        let point = s.near.as_ref().and_then(|n| self.points.iter().find(|p| &p.id == n));
        point.map_or(DVec3::ZERO, |p| v(p.pos_m)) + s.offset_m.map_or(DVec3::ZERO, v)
    }

    /// A berth's pose in the system frame: where the docked Tern's origin is and its attitude.
    pub fn berth_pose(&self, s: &Station, b: &Berth) -> (DVec3, DQuat) {
        let rot = self.station_rot(s);
        (self.station_pos(s) + rot * v(b.pos_m), (rot * hpr(b.hpr_deg)).normalize())
    }

    /// A station's or a point's place, by id.
    pub fn place(&self, id: &str) -> Option<Place> {
        if let Some(s) = self.station(id) {
            let pos = self.station_pos(s);
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
        self.stations.iter().find(|s| (pos - self.station_pos(s)).length() < s.mass_lock_m).map(|s| s.name.clone())
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
            if !["A", "B", "C", "D", "E"].contains(&s.version.as_str()) {
                c.equals(&format!("{f}.version"), &s.version, "A, B, C, D or E");
            }
            match (&s.body, s.orbit_radius_m, s.orbit_dir, &s.near, s.offset_m) {
                (Some(b), Some(r), Some(_), None, None) => match self.bodies.iter().find(|x| &x.id == b) {
                    // In orbit above its body's surface (the jump's mass lock is checked against jump.json in a test).
                    Some(body) => c.number(&format!("{f}.orbit_radius_m"), r, body.radius_m * 1.01, SYSTEM_LIM_M),
                    None => c.equals(&format!("{f}.body"), b, "a body of this file"),
                },
                (None, None, None, Some(n), Some(o)) => {
                    if !self.points.iter().any(|p| &p.id == n) {
                        c.equals(&format!("{f}.near"), n, "a point of this file");
                    }
                    finite(c, &format!("{f}.offset_m"), o, 1e9);
                }
                _ => c.equals(&format!("{f}.body"), "(both or neither)", "body, orbit_radius_m and orbit_dir, or near and offset_m"),
            }
            if let Some(d) = s.orbit_dir {
                finite(c, &format!("{f}.orbit_dir"), d, 1e6);
                c.number(&format!("{f}.orbit_dir (length)"), v(d).length(), 0.5, 2.0);
            }
            finite(c, &format!("{f}.hpr_deg"), s.hpr_deg, 360.0);
            c.number(&format!("{f}.mass_lock_m"), s.mass_lock_m, 0.0, 1e6);
            finite(c, &format!("{f}.arrival_m"), s.arrival_m, 1e6);
            let a = DVec3::from_array(s.arrival_m).length();
            c.number(&format!("{f}.arrival_m (distance)"), a, ARRIVAL_CLEAR_M, 1e6);
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
    fn tamsin_stations_axis_points_at_the_star_and_tamsin_lies_below_it() {
        // Version E's axes are on the star (spacedock design 5a); its frame keeps Tamsin under the bay's floor.
        let s = shoals();
        let st = s.station("tamsin_station").unwrap();
        let (pos, rot) = (s.station_pos(st), s.station_rot(st));
        let sky = s.sky_from(pos);
        let axis = rot * DVec3::Z;
        assert!(axis.angle_between(sky.sun_dir).to_degrees() < 0.5, "the axis is off the star");
        let (_, planet_dir, _) = sky.planet.expect("Tamsin is the near planet at its station");
        assert!((rot.inverse() * planet_dir).y < -0.5, "Tamsin below the station");
    }

    #[test]
    fn every_station_stands_at_its_body_or_point_and_orbits_outside_the_jumps_lock() {
        // spacedock design 11a: each at 3.33 of its body's radii, outside the jump's planet lock (jump.json).
        let s = shoals();
        let jump: serde_json::Value =
            serde_json::from_str(include_str!("../../../data/ships/tern/jump.json")).unwrap();
        let radii = jump["planet_radii"].as_f64().unwrap();
        assert_eq!(s.stations.len(), 5, "the five versions are the five stations");
        for st in &s.stations {
            match &st.body {
                Some(b) => {
                    let body = s.bodies.iter().find(|x| &x.id == b).unwrap();
                    let r = (s.station_pos(st) - DVec3::from_array(body.pos_m)).length();
                    assert!(r > body.radius_m * radii, "{} orbits inside {}'s lock", st.id, body.name);
                    assert_eq!(s.mass_lock(s.station_pos(st) + DVec3::Y * st.mass_lock_m * 1.5, radii), None);
                }
                None => assert!(s.points.iter().any(|p| Some(&p.id) == st.near.as_ref()), "{} is near a point", st.id),
            }
        }
    }

    #[test]
    fn every_berth_sits_inside_its_versions_bay() {
        let (s, dock) = (shoals(), crate::spacedock::Spacedock::shipped());
        for st in &s.stations {
            let bay = dock.bay(&st.version).unwrap();
            for b in &st.berths {
                assert!(bay.holds(DVec3::from_array(b.pos_m), 30.0), "{}'s {} is outside its bay", st.id, b.id);
            }
        }
    }

    #[test]
    fn every_departure_keeps_to_the_bays_and_the_corridors_limits() {
        let (s, dock) = (shoals(), crate::spacedock::Spacedock::shipped());
        for st in &s.stations {
            let bay = dock.bay(&st.version).unwrap();
            for b in &st.berths {
                let rot = from_hpr(b.hpr_deg[0], b.hpr_deg[1], b.hpr_deg[2]);
                for (k, l) in b.departure.iter().enumerate() {
                    let end = DVec3::from_array(b.pos_m) + rot * DVec3::from_array(l.to_m);
                    let limit = if bay.holds(end, 0.0) { dock.traffic.bay_mps } else { dock.traffic.corridor_mps };
                    assert!(l.max_mps <= limit, "{} leg {k} at {} m/s over {limit}", st.id, l.max_mps);
                }
            }
        }
    }

    #[test]
    fn the_tern_leaves_tamsin_station_down_its_axis_and_clear_of_its_lock() {
        let (s, dock) = (shoals(), crate::spacedock::Spacedock::shipped());
        let st = s.station("tamsin_station").unwrap();
        let b = &st.berths[0];
        let ends: Vec<DVec3> =
            b.departure.iter().map(|l| DVec3::from_array(b.pos_m) + DVec3::from_array(l.to_m)).collect();
        let mouth = dock.oneill.port.mouth_z();
        let on_axis = ends.iter().skip(1).all(|e| e.x.hypot(e.y) < 1.0);
        assert!(on_axis, "up off the wall to the axis, then along it: {ends:?}");
        let last = *ends.last().unwrap();
        assert!(last.z > mouth && last.length() > st.mass_lock_m, "out through the mouth and past the lock");
    }

    #[test]
    fn the_spacedock_mockups_station_list_agrees_with_the_system_file() {
        // spacedock tasks 2.1: the system file is the one source; until the mockup reads it, its own list must agree
        // (CLAUDE.md 6.6: the real artifacts compared, not two typed copies trusted).
        let s = shoals();
        let dock: serde_json::Value =
            serde_json::from_str(include_str!("../../../data/space/spacedock.json")).unwrap();
        let list = dock["stations"]["list"].as_array().unwrap();
        assert_eq!(list.len(), s.stations.len());
        for m in list {
            let st = s.station(m["id"].as_str().unwrap()).expect("every mockup station is in the system file");
            assert_eq!(m["version"].as_str(), Some(st.version.as_str()), "{}", st.id);
            assert_eq!(m["body"].as_str(), st.body.as_deref(), "{}", st.id);
            assert_eq!(m["near"].as_str(), st.near.as_deref(), "{}", st.id);
            assert_eq!(m["orbit_radius_m"].as_f64(), st.orbit_radius_m, "{}", st.id);
        }
    }

    #[test]
    fn a_station_with_neither_a_body_nor_a_point_is_refused() {
        let mut v: serde_json::Value = serde_json::from_str(FILE).unwrap();
        v["stations"][0]["body"] = serde_json::Value::Null;
        let err = parse::<SystemFile>("x", &v.to_string()).expect_err("no body and no point");
        assert_eq!(err.field, "stations[0].body");
    }

    #[test]
    fn tamsin_from_the_station_spans_seventeen_and_a_half_degrees() {
        let s = shoals();
        let sky = s.sky_from(s.station_pos(s.station("tamsin_station").unwrap()));
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
