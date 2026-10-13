//! `data/ships/<ship>/console_model.json`: the ship as the consoles draw it in 3D (the look band's camera widget,
//! Tactical's plot and Science's shields, weapons-and-shields 11): the geometry of the exterior model's distant detail
//! level alone, its corners welded and its triangles with the edges to outline, no texture.
//!
//! It lives in the core because two programs read one schema (CLAUDE.md 6.5): `sc-tools console-model` writes the file
//! from the exterior model (`assets/models/exteriors`) and checks what it wrote, and the client draws it. The console
//! mockup inlines the same file (`tools/mockups/inline.py`, `data:<ship>/console_model`), so the engine and the
//! mockup draw one model.

use crate::data::{Checks, Validate};
use serde::Deserialize;

/// The schema tag.
pub const SCHEMA: &str = "starcrew.console-model/1";

/// The most triangles a console model may hold: what the consoles' UI buffers spare for the two 3D pictures a console
/// draws at once (the camera widget and a shield view), a few tens of vertices a triangle each.
pub const MAX_TRIANGLES: usize = 1024;

/// The whole file.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ConsoleModel {
    /// [`SCHEMA`].
    pub schema: String,
    /// The model it was made from, a path from the repository's root.
    pub source: String,
    /// Two triangles that meet at more than this angle, degrees, meet at a crease, which is outlined.
    pub crease_deg: f64,
    /// The corners: metres, ship axes (+X port, +Y up, +Z bow), from the layout's origin.
    pub vertices_m: Vec<[f64; 3]>,
    /// Three corners a triangle, counter-clockwise seen from outside.
    pub triangles: Vec<[u32; 3]>,
    /// Each triangle's creases: bit `k` set when its edge from corner `k` to corner `k + 1` (mod 3) is one.
    pub creases: Vec<u8>,
}

impl Validate for ConsoleModel {
    fn validate(&self, c: &mut Checks) {
        c.equals("schema", &self.schema, SCHEMA);
        c.number("crease_deg", self.crease_deg, 0.0, 90.0);
        for (i, v) in self.vertices_m.iter().enumerate() {
            for (k, x) in v.iter().enumerate() {
                c.number(&format!("vertices_m.{i}.{k}"), *x, -1000.0, 1000.0);
            }
        }
        let n = self.triangles.len() as i64;
        c.count("triangles", n, 1, MAX_TRIANGLES as i64);
        let last = self.vertices_m.len() as i64 - 1;
        for (i, t) in self.triangles.iter().enumerate() {
            for (k, x) in t.iter().enumerate() {
                c.count(&format!("triangles.{i}.{k}"), i64::from(*x), 0, last);
            }
            if t[0] == t[1] || t[1] == t[2] || t[2] == t[0] {
                c.count(&format!("triangles.{i}"), 0, 1, 1);
            }
        }
        c.count("creases", self.creases.len() as i64, n, n);
        for (i, m) in self.creases.iter().enumerate() {
            c.count(&format!("creases.{i}"), i64::from(*m), 0, 7);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::parse;

    const TETRA: &str = r#"{ "schema": "starcrew.console-model/1", "_doc": "a note", "source": "x.glb",
        "crease_deg": 1.0, "vertices_m": [[0, 0, 0], [1, 0, 0], [0, 1, 0], [0, 0, 1]],
        "triangles": [[0, 2, 1], [0, 1, 3], [0, 3, 2], [1, 2, 3]], "creases": [7, 7, 7, 7] }"#;

    #[test]
    fn a_closed_model_loads() {
        let m: ConsoleModel = parse("t.json", TETRA).unwrap();
        assert_eq!(m.triangles.len(), 4);
    }

    #[test]
    fn a_corner_past_the_last_vertex_is_refused_naming_it() {
        let bad = TETRA.replace("[1, 2, 3]]", "[1, 2, 4]]");
        let e = parse::<ConsoleModel>("t.json", &bad).unwrap_err();
        assert_eq!(e.field, "triangles.3.2");
    }

    #[test]
    fn a_triangle_with_a_repeated_corner_is_refused() {
        let bad = TETRA.replace("[1, 2, 3]]", "[1, 2, 2]]");
        assert_eq!(parse::<ConsoleModel>("t.json", &bad).unwrap_err().field, "triangles.3");
    }

    #[test]
    fn creases_must_name_every_triangle_and_only_its_three_edges() {
        let short = TETRA.replace("[7, 7, 7, 7]", "[7, 7, 7]");
        assert_eq!(parse::<ConsoleModel>("t.json", &short).unwrap_err().field, "creases");
        let wide = TETRA.replace("[7, 7, 7, 7]", "[7, 7, 7, 8]");
        assert_eq!(parse::<ConsoleModel>("t.json", &wide).unwrap_err().field, "creases.3");
    }
}
