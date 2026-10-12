//! Engineering's schematic data (openspec/changes/power-routing design 3c, 3e and 6.4): the circuit board its 2D view
//! draws, every part and trace placed by hand (`power_board.json`), and the systems' families its lights are coloured
//! by (`subsystems.json`). The approved mockup, `docs/mockups/power-grid.html`, reads the same files.
//!
//! It lives in the core so `sc-tools check-data` holds the files to the power data (a trace for every edge, a part and
//! a label for every node, a family for every system), as the client draws them.

use crate::data::{Checks, DataError, Validate};
use crate::power::PowerData;
use serde::Deserialize;
use std::collections::BTreeMap;

/// The board's file, relative to the repository's root.
pub const BOARD_FILE: &str = "data/ships/tern/power_board.json";
/// The families' file.
pub const FAMILIES_FILE: &str = "data/ships/tern/subsystems.json";
/// The parts on the board that are not nodes.
pub const OTHER_PARTS: [&str; 2] = ["reactor", "battery"];
/// The battery's link's id on the board (the mockup's edge id).
pub const BATTERY_LINK: &str = "battery_out";

/// `power_board.json` (`starcrew.power-board/1`).
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PowerBoard {
    pub schema: String,
    pub status: String,
    pub ship: String,
    pub units: String,
    pub board: BoardFrame,
    /// A board a deck, top to bottom.
    pub layers: Vec<BoardLayer>,
    /// Each part: centre x, centre y, width, height.
    pub parts: BTreeMap<String, [f64; 4]>,
    /// Each part's silkscreen name.
    pub labels: BTreeMap<String, String>,
    /// Each edge's trace: its points from the pad on its first end to the pad on its other.
    pub traces: BTreeMap<String, Vec<[f64; 2]>>,
}

/// The whole drawing's size, the boards' left and right, and how far a trace's corner is cut.
#[derive(Debug, Clone, Copy, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct BoardFrame {
    pub w: f64,
    pub h: f64,
    pub x: [f64; 2],
    pub chamfer: f64,
}

/// One deck's board: its top and bottom.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct BoardLayer {
    pub deck: String,
    pub y: [f64; 2],
}

impl Validate for PowerBoard {
    fn validate(&self, c: &mut Checks) {
        c.equals("schema", &self.schema, "starcrew.power-board/1");
        let b = &self.board;
        c.number("board.w", b.w, 100.0, 10000.0);
        c.number("board.h", b.h, 100.0, 10000.0);
        c.number("board.x[0]", b.x[0], 0.0, b.w);
        c.number("board.x[1]", b.x[1], b.x[0], b.w);
        c.number("board.chamfer", b.chamfer, 0.0, 100.0);
        for (k, l) in self.layers.iter().enumerate() {
            c.number(&format!("layers[{k}].y[0]"), l.y[0], 0.0, b.h);
            c.number(&format!("layers[{k}].y[1]"), l.y[1], l.y[0], b.h);
        }
        for (id, p) in &self.parts {
            c.number(&format!("parts.{id}[0]"), p[0], 0.0, b.w);
            c.number(&format!("parts.{id}[1]"), p[1], 0.0, b.h);
            c.number(&format!("parts.{id}[2]"), p[2], 1.0, 400.0);
            c.number(&format!("parts.{id}[3]"), p[3], 1.0, 400.0);
        }
        for (id, pts) in &self.traces {
            c.count(&format!("traces.{id}"), pts.len() as i64, 2, 64);
            for (k, p) in pts.iter().enumerate() {
                c.number(&format!("traces.{id}[{k}][0]"), p[0], 0.0, b.w);
                c.number(&format!("traces.{id}[{k}][1]"), p[1], 0.0, b.h);
            }
            // Every segment at 0, 45 or 90 degrees (design 3c).
            for (k, w) in pts.windows(2).enumerate() {
                let (dx, dy) = ((w[1][0] - w[0][0]).abs(), (w[1][1] - w[0][1]).abs());
                if dx > 0.5 && dy > 0.5 && (dx - dy).abs() > 0.5 {
                    c.equals(&format!("traces.{id}[{k}]"), "a slanted segment", "a segment at 0, 45 or 90 degrees");
                }
            }
        }
    }
}

impl PowerBoard {
    /// The board's ids against the power data's: a trace for every edge and nothing else, a part and a label for
    /// every node and the reactor and the battery, and nothing else.
    pub fn check(&self, pd: &PowerData) -> Result<(), DataError> {
        let err = |field: String, message: String| DataError { file: BOARD_FILE.into(), field, message };
        let f = &pd.file;
        let edges: Vec<&str> = f
            .generators
            .iter()
            .map(|g| g.id.as_str())
            .chain(f.ties.iter().map(|t| t.id.as_str()))
            .chain([BATTERY_LINK])
            .chain(f.conduits.iter().map(|k| k.id.as_str()))
            .collect();
        for e in &edges {
            if !self.traces.contains_key(*e) {
                return Err(err(format!("traces.{e}"), "is missing: every edge of power.json has a trace".into()));
            }
        }
        if let Some(t) = self.traces.keys().find(|t| !edges.contains(&t.as_str())) {
            return Err(err(format!("traces.{t}"), "is no edge of power.json".into()));
        }
        let parts: Vec<&str> = pd.nodes.iter().map(String::as_str).chain(OTHER_PARTS).collect();
        for (name, map) in [("parts", self.parts.keys().collect::<Vec<_>>()), ("labels", self.labels.keys().collect())]
        {
            for p in &parts {
                if !map.iter().any(|k| k.as_str() == *p) {
                    return Err(err(format!("{name}.{p}"), "is missing: every node has a part and a label".into()));
                }
            }
            if let Some(k) = map.iter().find(|k| !parts.contains(&k.as_str())) {
                return Err(err(format!("{name}.{k}"), "is no node of power.json".into()));
            }
        }
        Ok(())
    }

    /// Parse, validate and check `text` against the power data.
    pub fn load(text: &str, pd: &PowerData) -> Result<Self, DataError> {
        let b: Self = crate::data::parse(BOARD_FILE, text)?;
        b.check(pd)?;
        Ok(b)
    }
}

/// `subsystems.json` (`starcrew.subsystems/1`): the systems' families.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Families {
    pub schema: String,
    pub status: String,
    pub ship: String,
    pub units: String,
    pub families: Vec<Family>,
}

/// One family: its colour (`#rrggbb`), the `power.json` groups whose loads it takes, and loads of its own.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Family {
    pub id: String,
    pub name: String,
    pub colour: String,
    pub groups: Vec<String>,
    pub loads: Vec<String>,
    /// The family of every load in no other.
    #[serde(default)]
    pub default: bool,
}

impl Validate for Families {
    fn validate(&self, c: &mut Checks) {
        c.equals("schema", &self.schema, "starcrew.subsystems/1");
        c.count("families", self.families.len() as i64, 1, 16);
        c.count("families (default)", self.families.iter().filter(|f| f.default).count() as i64, 1, 1);
        for (k, f) in self.families.iter().enumerate() {
            if Family::rgb(&f.colour).is_none() {
                c.equals(&format!("families[{k}].colour"), &f.colour, "#rrggbb");
            }
        }
    }
}

impl Family {
    /// `#rrggbb` as its three bytes.
    pub fn rgb(hex: &str) -> Option<[u8; 3]> {
        let h = hex.strip_prefix('#').filter(|h| h.len() == 6)?;
        let v = u32::from_str_radix(h, 16).ok()?;
        Some([(v >> 16) as u8, (v >> 8) as u8, v as u8])
    }
}

impl Families {
    /// Each load's family, by index in `pd.loads`: the last family naming it (by group or by itself), else the default.
    pub fn of_loads(&self, pd: &PowerData) -> Vec<usize> {
        let def = self.families.iter().position(|f| f.default).unwrap_or(0);
        let mut out = vec![def; pd.loads.len()];
        for (k, f) in self.families.iter().enumerate() {
            for g in &f.groups {
                if let Some(gi) = pd.file.groups.iter().position(|x| x.id == *g) {
                    for &i in &pd.group_loads[gi] {
                        out[i] = k;
                    }
                }
            }
            for id in &f.loads {
                if let Some(i) = pd.load(id) {
                    out[i] = k;
                }
            }
        }
        out
    }

    /// Parse, validate and check `text` against the power data: every group and load a family names exists.
    pub fn load(text: &str, pd: &PowerData) -> Result<Self, DataError> {
        let f: Self = crate::data::parse(FAMILIES_FILE, text)?;
        let err = |field: String, message: String| DataError { file: FAMILIES_FILE.into(), field, message };
        for (k, fam) in f.families.iter().enumerate() {
            if let Some(g) = fam.groups.iter().find(|g| !pd.file.groups.iter().any(|x| x.id == **g)) {
                return Err(err(format!("families[{k}].groups"), format!("{g} is no group of power.json")));
            }
            if let Some(l) = fam.loads.iter().find(|l| pd.load(l).is_none()) {
                return Err(err(format!("families[{k}].loads"), format!("{l} is no load of power.json")));
            }
        }
        Ok(f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_shipped_board_and_families_hold_to_the_power_data() {
        let d = crate::combat::data::DrillData::shipped();
        let b = PowerBoard::load(include_str!("../../../data/ships/tern/power_board.json"), &d.power).unwrap();
        assert_eq!(b.traces.len(), d.power.edge_count(), "a trace for every edge");
        let f = Families::load(include_str!("../../../data/ships/tern/subsystems.json"), &d.power).unwrap();
        let of = f.of_loads(&d.power);
        let fam = |id: &str| f.families[of[d.power.load(id).unwrap()]].id.as_str();
        assert_eq!(fam("shield_generator"), "shields");
        assert_eq!(fam("laser_port"), "weapons", "a load in no group, named by its family");
        assert_eq!(fam("turret_dorsal"), "weapons");
        assert_eq!(fam("o2_generator"), "life_support");
        assert_eq!(fam("impulse_drive"), "misc", "the default");
    }

    #[test]
    fn a_board_missing_an_edge_or_naming_a_stranger_is_refused() {
        let d = crate::combat::data::DrillData::shipped();
        let text = include_str!("../../../data/ships/tern/power_board.json");
        let missing = text.replace("\"r_cargo\":", "\"r_cargo_gone\":");
        let e = PowerBoard::load(&missing, &d.power).unwrap_err();
        assert!(e.field.starts_with("traces.r_cargo"), "{e}");
        let slanted = text.replace("\"rf_drive\": [[372, 560], [331, 560]]", "\"rf_drive\": [[372, 560], [331, 590]]");
        assert_ne!(slanted, text);
        assert!(PowerBoard::load(&slanted, &d.power).unwrap_err().field.starts_with("traces.rf_drive"));
    }
}
