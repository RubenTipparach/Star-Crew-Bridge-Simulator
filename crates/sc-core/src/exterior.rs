//! `data/space/exterior.json`: the outside of the ship while it lies in the space dock (openspec/changes/deck-pipeline,
//! design section 13b): the sun, the planet below to port, the stars, the dock's frame and the bow camera that feeds the
//! viewscreens.
//!
//! It lives in the core because two programs read it: `deckc` builds the dock's girders from it into the deck file, and
//! the client draws the sky and the viewscreens from it. One schema, one loader (CLAUDE.md 6.5).

use crate::data::{Checks, Validate};
use serde::Deserialize;

/// The whole file.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ExteriorData {
    /// `starcrew.exterior/1`.
    pub schema: String,
    /// The sun.
    pub sun: Sun,
    /// The planet the dock orbits.
    pub planet: Planet,
    /// The star field.
    pub stars: Stars,
    /// The dock's frame round the hull.
    pub dock: Dock,
    /// The bow camera the viewscreens show.
    pub viewscreen: Viewscreen,
}

/// The sun: a disc and a glow round it.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Sun {
    /// The way to it, ship coordinates, any length.
    pub dir: [f64; 3],
    /// Its colour, sRGB 0-1.
    pub colour_srgb: [f64; 3],
    /// The disc's angular diameter, degrees.
    pub disc_deg: f64,
    /// How bright the glow round the disc is, 0-1.
    pub glow: f64,
}

/// The planet: a lit sphere with oceans, land, cloud and an atmosphere's rim.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Planet {
    /// The way to its centre, ship coordinates, any length.
    pub dir: [f64; 3],
    /// The angle from its centre to its limb, degrees.
    pub radius_deg: f64,
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
    /// The share of the surface that is land, 0-1.
    pub land_frac: f64,
    /// The share covered by cloud, 0-1.
    pub cloud_frac: f64,
    /// How bright the night side is, 0-1.
    pub night: f64,
}

/// The star field.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Stars {
    /// The share of grid cells holding a star, 0-1.
    pub density: f64,
    /// The brightest star's value, 0-2.
    pub brightness: f64,
}

/// The dock: a box frame of girders round the hull.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Dock {
    /// Its centre, ship coordinates, metres.
    pub center_m: [f64; 3],
    /// Along z, metres.
    pub length_m: f64,
    /// Along x, metres.
    pub width_m: f64,
    /// Along y, metres.
    pub height_m: f64,
    /// Between ring frames, metres.
    pub ring_spacing_m: f64,
    /// A girder's square section, metres.
    pub girder_m: f64,
    /// A brace's square section, metres.
    pub brace_m: f64,
    /// A work light's cube, metres.
    pub light_m: f64,
    /// The material the steel samples (`data/materials/materials.json`).
    pub material: String,
    /// The steel's colour, sRGB 0-1.
    pub steel_srgb: [f64; 3],
    /// The work lights' colour, sRGB 0-1.
    pub light_srgb: [f64; 3],
    /// The shade side's light as a share of the sun's, 0-1.
    pub ambient: f64,
}

/// The bow camera the viewscreens show.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Viewscreen {
    /// Where it sits, ship coordinates, metres.
    pub camera_m: [f64; 3],
    /// The way it looks, ship coordinates, any length.
    pub look: [f64; 3],
    /// Its vertical field of view, degrees.
    pub fov_deg: f64,
    /// The target it renders into, pixels.
    pub target_px: [u32; 2],
    /// How deep the scan lines are, 0-1.
    pub scanlines: f64,
}

fn unit(c: &mut Checks, field: &str, v: [f64; 3]) {
    for (k, x) in v.iter().enumerate() {
        c.number(&format!("{field}[{k}]"), *x, -1e6, 1e6);
    }
    if v.iter().map(|x| x * x).sum::<f64>() < 1e-12 {
        c.number(field, 0.0, 1.0, 1.0);
    }
}

fn colour(c: &mut Checks, field: &str, v: [f64; 3]) {
    for (k, x) in v.iter().enumerate() {
        c.number(&format!("{field}[{k}]"), *x, 0.0, 1.0);
    }
}

impl Validate for ExteriorData {
    fn validate(&self, c: &mut Checks) {
        c.equals("schema", &self.schema, "starcrew.exterior/1");
        let s = &self.sun;
        unit(c, "sun.dir", s.dir);
        colour(c, "sun.colour_srgb", s.colour_srgb);
        c.number("sun.disc_deg", s.disc_deg, 0.05, 10.0);
        c.number("sun.glow", s.glow, 0.0, 1.0);
        let p = &self.planet;
        unit(c, "planet.dir", p.dir);
        c.number("planet.radius_deg", p.radius_deg, 0.5, 80.0);
        for (k, v) in [("ocean_srgb", p.ocean_srgb), ("land_srgb", p.land_srgb), ("cloud_srgb", p.cloud_srgb)] {
            colour(c, &format!("planet.{k}"), v);
        }
        colour(c, "planet.atmosphere_srgb", p.atmosphere_srgb);
        c.number("planet.atmosphere_frac", p.atmosphere_frac, 0.0, 0.5);
        for (k, v) in [("land_frac", p.land_frac), ("cloud_frac", p.cloud_frac), ("night", p.night)] {
            c.number(&format!("planet.{k}"), v, 0.0, 1.0);
        }
        c.number("stars.density", self.stars.density, 0.0, 1.0);
        c.number("stars.brightness", self.stars.brightness, 0.0, 2.0);
        let d = &self.dock;
        for (k, v) in d.center_m.iter().enumerate() {
            c.number(&format!("dock.center_m[{k}]"), *v, -1000.0, 1000.0);
        }
        c.number("dock.length_m", d.length_m, 10.0, 1000.0);
        // The deck vertex holds +/-32 m about a compartment's centre: a bay must fit, girders and all.
        c.number("dock.width_m", d.width_m, 4.0, 60.0);
        c.number("dock.height_m", d.height_m, 4.0, 60.0);
        c.number("dock.ring_spacing_m", d.ring_spacing_m, 2.0, 40.0);
        c.number("dock.girder_m", d.girder_m, 0.05, 5.0);
        c.number("dock.brace_m", d.brace_m, 0.05, d.girder_m);
        c.number("dock.light_m", d.light_m, 0.05, 5.0);
        colour(c, "dock.steel_srgb", d.steel_srgb);
        colour(c, "dock.light_srgb", d.light_srgb);
        c.number("dock.ambient", d.ambient, 0.0, 1.0);
        let v = &self.viewscreen;
        for (k, x) in v.camera_m.iter().enumerate() {
            c.number(&format!("viewscreen.camera_m[{k}]"), *x, -1000.0, 1000.0);
        }
        unit(c, "viewscreen.look", v.look);
        c.number("viewscreen.fov_deg", v.fov_deg, 5.0, 120.0);
        c.count("viewscreen.target_px[0]", i64::from(v.target_px[0]), 16, 2048);
        c.count("viewscreen.target_px[1]", i64::from(v.target_px[1]), 16, 2048);
        c.number("viewscreen.scanlines", v.scanlines, 0.0, 1.0);
    }
}

/// A direction made unit length, as `f32`.
pub fn normalized(v: [f64; 3]) -> [f32; 3] {
    let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    [(v[0] / l) as f32, (v[1] / l) as f32, (v[2] / l) as f32]
}

/// An sRGB 0-1 colour as linear light.
pub fn linear(c: [f64; 3]) -> [f32; 3] {
    c.map(|x| if x <= 0.04045 { x / 12.92 } else { ((x + 0.055) / 1.055).powf(2.4) } as f32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::parse;

    const FILE: &str = include_str!("../../../data/space/exterior.json");

    #[test]
    fn the_shipped_exterior_is_valid() {
        let e: ExteriorData = parse("data/space/exterior.json", FILE).expect("the shipped file loads");
        assert!(e.dock.width_m > 24.4 && e.dock.height_m > 13.8, "the dock clears the Tern's hull");
    }

    #[test]
    fn a_dock_too_wide_for_the_deck_vertex_is_refused() {
        let bad = FILE.replace("\"width_m\": 52.0", "\"width_m\": 80.0");
        let err = parse::<ExteriorData>("x", &bad).expect_err("a bay wider than 64 m does not fit the vertex");
        assert_eq!(err.field, "dock.width_m");
    }
}
