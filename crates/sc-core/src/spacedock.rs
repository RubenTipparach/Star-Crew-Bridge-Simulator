//! The spacedock versions as the engine reads them (`data/space/spacedock.json`; openspec/changes/spacedock): each
//! version's bay, the traffic limits, and the O'Neill pair's sizes (version E) for its generator.
//!
//! It lives in the core because the system file's checks hold every berth to its version's bay and every departure
//! leg to the traffic limits, and the client builds E's model from the same sizes: one file, one reading (CLAUDE.md
//! 6.5). The file is the spacedock mockup's own (schema `starcrew.spacedock-options/1`), richer than the engine needs;
//! this reads the fields named here and checks them, and leaves the rest to the mockup, so a reading is partial on
//! purpose (no `deny_unknown_fields`).

use std::collections::BTreeMap;

use glam::DVec3;
use serde::Deserialize;

use crate::data::{Checks, DataError, Validate};

/// The file, relative to the repository's root.
pub const SPACEDOCK_FILE: &str = "data/space/spacedock.json";

/// The shipped file's text, compiled in: the core's reading and the client's station models read the same bytes.
pub const SHIPPED: &str = include_str!("../../../data/space/spacedock.json");

/// The traffic limits (design 6).
#[derive(Debug, Clone, Copy, Deserialize, PartialEq)]
pub struct Traffic {
    /// The approach corridor's speed, m/s.
    pub corridor_mps: f64,
    /// At the doors, m/s.
    pub door_mps: f64,
    /// Inside the bay, m/s.
    pub bay_mps: f64,
}

/// E's cylinders (design 5a).
#[derive(Debug, Clone, Copy, Deserialize, PartialEq)]
pub struct Cylinders {
    /// Radius, length, the shell's thickness and a dome's depth, metres.
    pub radius_m: f64,
    pub length_m: f64,
    pub shell_m: f64,
    pub dome_m: f64,
    /// The pair's centres apart, metres, and the second's bearing round the first's axis, degrees.
    pub spacing_m: f64,
    pub pair_angle_deg: f64,
}

/// E's port: the hub, its throat and iris, and the round bay behind (design 5a).
#[derive(Debug, Clone, Copy, Deserialize, PartialEq)]
pub struct Port {
    /// The hub's outer radius (the dome's hole), the throat's and the bay's radius, metres.
    pub hub_radius_m: f64,
    pub throat_radius_m: f64,
    /// Where the iris stands on the axis, how deep the throat is in front of it, how long the bay is behind it,
    /// metres in the station's frame.
    pub iris_z_m: f64,
    pub recess_m: f64,
    pub bay_length_m: f64,
}

impl Port {
    /// The mouth's station along the axis (the dome's apex), metres.
    pub fn mouth_z(&self) -> f64 {
        self.iris_z_m + self.recess_m
    }

    /// The bay's back wall's station, metres.
    pub fn back_z(&self) -> f64 {
        self.iris_z_m - self.bay_length_m
    }
}

/// E's mirrors (design 5a).
#[derive(Debug, Clone, Copy, Deserialize, PartialEq)]
pub struct Mirrors {
    /// Opened toward the star by day, degrees.
    pub open_deg: f64,
    /// Thickness, metres, and length as a share of a cylinder's.
    pub thickness_m: f64,
    pub length_frac: f64,
}

/// E's farm ring (design 5a).
#[derive(Debug, Clone, Copy, Deserialize, PartialEq)]
pub struct Ring {
    /// The ring's radius, its pods, a pod's radius and length, the tube's thickness, its spokes, and its inset from
    /// the sunward dome's foot, metres.
    pub radius_m: f64,
    pub pods: u32,
    pub pod_radius_m: f64,
    pub pod_length_m: f64,
    pub tube_m: f64,
    pub spokes: u32,
    pub inset_m: f64,
}

/// E's berths round the bay's wall: [row, angle round the axis from the bottom in degrees, the ship or none].
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct EBerths {
    /// The ships' origins' distance from the axis, metres.
    pub radius_m: f64,
    /// Each row's station along the axis, metres.
    pub rows_z_m: Vec<f64>,
    /// The slots.
    pub slots: Vec<(usize, f64, Option<String>)>,
}

/// Version E, the O'Neill pair: what its generator needs.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct OneillPair {
    /// The star's way in the station's frame (along the axes).
    pub sun_dir: [f64; 3],
    pub cylinders: Cylinders,
    pub port: Port,
    pub mirrors: Mirrors,
    pub ring: Ring,
    pub berths: EBerths,
}

/// A bay's room, in its station's frame: a round hall about an axis, or a box.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Bay {
    /// A cylinder about the y axis (`y` false: about z): radius, and its ends along the axis, metres.
    Round { about_y: bool, radius_m: f64, from_m: f64, to_m: f64 },
    /// A box: half width (x), half height (y), its ends along z, metres.
    Box { half_x_m: f64, half_y_m: f64, from_m: f64, to_m: f64 },
}

impl Bay {
    /// Whether `p` (station frame) is inside with `clear_m` to spare on every side.
    pub fn holds(&self, p: DVec3, clear_m: f64) -> bool {
        let within = |x: f64, lo: f64, hi: f64| x >= lo + clear_m && x <= hi - clear_m;
        match *self {
            Bay::Round { about_y, radius_m, from_m, to_m } => {
                let (along, r) = if about_y { (p.y, p.x.hypot(p.z)) } else { (p.z, p.x.hypot(p.y)) };
                within(along, from_m, to_m) && r <= radius_m - clear_m
            }
            Bay::Box { half_x_m, half_y_m, from_m, to_m } => {
                within(p.z, from_m, to_m) && p.x.abs() <= half_x_m - clear_m && p.y.abs() <= half_y_m - clear_m
            }
        }
    }
}

/// What the engine reads of the file.
#[derive(Debug, Clone, PartialEq)]
pub struct Spacedock {
    /// The traffic limits.
    pub traffic: Traffic,
    /// Each version's bay, by its letter.
    pub bays: BTreeMap<String, Bay>,
    /// Version E.
    pub oneill: OneillPair,
}

impl Validate for OneillPair {
    fn validate(&self, c: &mut Checks) {
        let y = &self.cylinders;
        c.number("versions.E.cylinders.radius_m", y.radius_m, 100.0, 10_000.0);
        c.number("versions.E.cylinders.length_m", y.length_m, y.radius_m, 50_000.0);
        c.number("versions.E.cylinders.shell_m", y.shell_m, 1.0, y.radius_m / 2.0);
        c.number("versions.E.cylinders.dome_m", y.dome_m, 10.0, y.radius_m);
        c.number("versions.E.cylinders.spacing_m", y.spacing_m, 2.0 * y.radius_m, 100_000.0);
        let p = &self.port;
        c.number("versions.E.port.hub_radius_m", p.hub_radius_m, 50.0, y.radius_m);
        c.number("versions.E.port.throat_radius_m", p.throat_radius_m, 30.0, p.hub_radius_m);
        c.number("versions.E.port.bay_length_m", p.bay_length_m, 100.0, y.length_m);
        c.number("versions.E.port.recess_m", p.recess_m, 10.0, y.dome_m + 1000.0);
        c.number("versions.E.mirrors.open_deg", self.mirrors.open_deg, 0.0, 90.0);
        c.number("versions.E.mirrors.length_frac", self.mirrors.length_frac, 0.1, 1.0);
        c.count("versions.E.ring.pods", i64::from(self.ring.pods), 1, 64);
        c.number("versions.E.berths.radius_m", self.berths.radius_m, 0.0, p.throat_radius_m);
        for (i, s) in self.berths.slots.iter().enumerate() {
            c.count(
                &format!("versions.E.berths.slots[{i}] (row)"),
                s.0 as i64,
                0,
                self.berths.rows_z_m.len() as i64 - 1,
            );
        }
    }
}

/// A number at a JSON pointer, or the error naming it.
fn num(v: &serde_json::Value, ptr: &str) -> Result<f64, DataError> {
    v.pointer(ptr).and_then(serde_json::Value::as_f64).ok_or_else(|| DataError {
        file: SPACEDOCK_FILE.into(),
        field: ptr.trim_start_matches('/').replace('/', "."),
        message: "must be a number".into(),
    })
}

impl Spacedock {
    /// Read the file's text: the traffic limits, each version's bay and version E, checked.
    pub fn parse(text: &str) -> Result<Self, DataError> {
        let err =
            |field: &str, message: String| DataError { file: SPACEDOCK_FILE.into(), field: field.into(), message };
        let v: serde_json::Value = serde_json::from_str(text).map_err(|e| err(".", e.to_string()))?;
        let traffic: Traffic =
            serde_json::from_value(v["traffic"].clone()).map_err(|e| err("traffic", e.to_string()))?;
        let oneill: OneillPair =
            serde_json::from_value(v["versions"]["E"].clone()).map_err(|e| err("versions.E", e.to_string()))?;
        let mut c = Checks::default();
        oneill.validate(&mut c);
        for (k, x) in
            [("corridor_mps", traffic.corridor_mps), ("door_mps", traffic.door_mps), ("bay_mps", traffic.bay_mps)]
        {
            c.number(&format!("traffic.{k}"), x, 0.1, 400.0);
        }
        c.result(SPACEDOCK_FILE)?;
        // Each version's bay from its own sizes (design 2-5a).
        let mut bays = BTreeMap::new();
        bays.insert(
            "A".to_owned(),
            Bay::Round {
                about_y: true,
                radius_m: num(&v, "/versions/A/hall/radius_m")?,
                from_m: num(&v, "/versions/A/hall/floor_y_m")?,
                to_m: num(&v, "/versions/A/hall/ceiling_y_m")?,
            },
        );
        for k in ["B", "D"] {
            bays.insert(
                k.to_owned(),
                Bay::Round {
                    about_y: false,
                    radius_m: num(&v, &format!("/versions/{k}/hall/radius_m"))?,
                    from_m: num(&v, &format!("/versions/{k}/hall/z_m/0"))?,
                    to_m: num(&v, &format!("/versions/{k}/hall/z_m/1"))?,
                },
            );
        }
        bays.insert(
            "C".to_owned(),
            Bay::Box {
                half_x_m: num(&v, "/versions/C/hall/width_m")? / 2.0,
                half_y_m: num(&v, "/versions/C/hall/height_m")? / 2.0,
                from_m: num(&v, "/versions/C/hall/z_m/0")?,
                to_m: num(&v, "/versions/C/hall/z_m/1")?,
            },
        );
        let p = &oneill.port;
        bays.insert(
            "E".to_owned(),
            Bay::Round { about_y: false, radius_m: p.throat_radius_m, from_m: p.back_z(), to_m: p.iris_z_m },
        );
        Ok(Self { traffic, bays, oneill })
    }

    /// The shipped file, compiled in.
    pub fn shipped() -> Self {
        Self::parse(SHIPPED).expect("the shipped spacedock file is valid")
    }

    /// A version's bay.
    pub fn bay(&self, version: &str) -> Option<&Bay> {
        self.bays.get(version)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_version_has_a_bay_and_e_has_its_port() {
        let s = Spacedock::shipped();
        for k in ["A", "B", "C", "D", "E"] {
            assert!(s.bay(k).is_some(), "version {k}");
        }
        assert_eq!(s.oneill.port.mouth_z(), 780.0, "the iris at 460 m and the throat 320 m deep");
    }

    #[test]
    fn a_point_on_e_s_axis_is_in_its_bay_and_one_through_its_wall_is_not() {
        let s = Spacedock::shipped();
        let bay = s.bay("E").unwrap();
        assert!(bay.holds(DVec3::new(0.0, 0.0, 100.0), 30.0));
        assert!(!bay.holds(DVec3::new(0.0, -260.0, 100.0), 30.0), "10 m from the wall is too close");
        assert!(!bay.holds(DVec3::new(0.0, 0.0, 600.0), 30.0), "in the throat, past the iris");
    }
}
