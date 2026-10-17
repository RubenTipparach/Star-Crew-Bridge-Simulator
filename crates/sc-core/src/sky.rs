//! A system's nebula sky and the vector stars drawn over it (openspec/changes/nebula-skies, design 3, 5 and 9):
//! `data/space/skies/<id>.json`, written by `tools/space/nebula_skies.py`, and `data/space/starfield.json`, the stars'
//! classes, colours and streak rule.
//!
//! It lives in the core because the data loader is one (CLAUDE.md 6.5): `sc-tools check-data` refuses a bad sky here
//! before the client would, and the client hands the decoded sky to the renderer. Nothing here draws. A sky's mesh is a
//! closed sphere of directions in the system's frame (+y north, the planets in the xz plane), each with an sRGB colour
//! and a byte for how much of the stars behind it the nebula hides; its stars are directions with a brightness, a class
//! and a colour that index `starfield.json`.

use serde::Deserialize;

use crate::data::{parse, Checks, DataError, Validate};

/// The stars' classes, colours and streak rule.
pub const STARFIELD_FILE: &str = "data/space/starfield.json";

/// The most vertices a sky may hold (the nebula-skies spec: "at most 8,000 vertices", NS5).
pub const MAX_SKY_VERTICES: usize = 8_000;

/// The most vector stars a sky may hold (design 5.1: 1,400 to 2,400 a sky, with room).
pub const MAX_SKY_STARS: usize = 4_000;

/// The file of the sky `id`.
pub fn sky_file(id: &str) -> String {
    format!("data/space/skies/{id}.json")
}

/// `data/space/starfield.json`.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Starfield {
    /// `starcrew.starfield/1`.
    pub schema: String,
    /// The brightness classes, by the index a sky's star names.
    pub classes: Vec<StarClass>,
    /// The colours, by the index a sky's star names.
    pub colours: Vec<StarColour>,
    /// How a star streaks when the view turns.
    pub streak: Streak,
    /// The motes round the eye (patrol-mission 11.4; read, not yet drawn).
    pub motes: Motes,
}

/// A brightness class of vector stars.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct StarClass {
    /// Its name.
    pub name: String,
    /// Its share of a sky's stars (the generator's).
    pub share: f64,
    /// Its brightness range, 0-1 (the generator's).
    pub brightness: [f64; 2],
    /// Its head's size at rest, pixels: 1 is the line alone, 2 or more draws a round head over it.
    pub size_px: f64,
}

/// A star colour.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct StarColour {
    /// Its name.
    pub name: String,
    /// Its share of a sky's stars (the generator's).
    pub share: f64,
    /// The colour, sRGB 0-1.
    pub srgb: [f64; 3],
}

/// The streak (design 5.2, after FreeSpace 2's `StarField.cpp`): a star is drawn from where it is on screen back toward
/// where it was `tail_s` ago, the tail capped and the star dimmed as it grows.
#[derive(Debug, Clone, Copy, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Streak {
    /// The seconds of turning a tail shows.
    pub tail_s: f64,
    /// The longest tail, degrees.
    pub max_deg: f64,
    /// The least line drawn, pixels: a star at rest is a point this long.
    pub min_px: f64,
    /// The brightness lost per degree of tail.
    pub dim_per_deg: f64,
    /// The least brightness a streaking star keeps, 0-1.
    pub dim_floor: f64,
    /// The tail end's share of the head's opacity.
    pub tail_fade: f64,
    /// A streaking star's least opacity, before the dim.
    pub alpha_min: f64,
    /// How far a streaking star's colour goes toward white, 0-1.
    pub whiten: f64,
    /// A view turning faster than this is a camera cut: no tail that frame, degrees a second.
    pub cut_deg_s: f64,
    /// The faint hashed stars' brightness while the tail is at its cap, 0-1 (design 5.5: they fade during a fast turn).
    pub faint_floor: f64,
}

/// The motes round the eye (patrol-mission 11.4, design 5.3).
#[derive(Debug, Clone, Copy, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Motes {
    /// How many.
    pub count: u32,
    /// The cube they wrap in, metres a side.
    pub cube_m: f64,
    /// The seconds of motion a mote's streak shows.
    pub tail_s: f64,
    /// They fade beyond this, metres.
    pub fade_from_m: f64,
    /// Their brightness, 0-1.
    pub brightness: f64,
}

fn unit_colour(c: &mut Checks, field: &str, a: [f64; 3]) {
    for (k, x) in a.iter().enumerate() {
        c.number(&format!("{field}[{k}]"), *x, 0.0, 1.0);
    }
}

impl Validate for Starfield {
    fn validate(&self, c: &mut Checks) {
        c.equals("schema", &self.schema, "starcrew.starfield/1");
        c.count("classes", self.classes.len() as i64, 1, 255);
        c.count("colours", self.colours.len() as i64, 1, 255);
        for (i, k) in self.classes.iter().enumerate() {
            c.number(&format!("classes[{i}].share"), k.share, 0.0, 1.0);
            c.number(&format!("classes[{i}].brightness[0]"), k.brightness[0], 0.0, 1.0);
            c.number(&format!("classes[{i}].brightness[1]"), k.brightness[1], k.brightness[0], 1.0);
            c.number(&format!("classes[{i}].size_px"), k.size_px, 1.0, 8.0);
        }
        for (i, k) in self.colours.iter().enumerate() {
            c.number(&format!("colours[{i}].share"), k.share, 0.0, 1.0);
            unit_colour(c, &format!("colours[{i}].srgb"), k.srgb);
        }
        let s = &self.streak;
        c.number("streak.tail_s", s.tail_s, 0.0, 0.5);
        c.number("streak.max_deg", s.max_deg, 0.0, 20.0);
        c.number("streak.min_px", s.min_px, 0.0, 4.0);
        c.number("streak.dim_per_deg", s.dim_per_deg, 0.0, 10.0);
        c.number("streak.dim_floor", s.dim_floor, 0.0, 1.0);
        c.number("streak.tail_fade", s.tail_fade, 0.0, 1.0);
        c.number("streak.alpha_min", s.alpha_min, 0.0, 1.0);
        c.number("streak.whiten", s.whiten, 0.0, 1.0);
        c.number("streak.cut_deg_s", s.cut_deg_s, 1.0, 100_000.0);
        c.number("streak.faint_floor", s.faint_floor, 0.0, 1.0);
        let m = &self.motes;
        c.count("motes.count", i64::from(m.count), 0, 10_000);
        c.number("motes.cube_m", m.cube_m, 1.0, 10_000.0);
        c.number("motes.tail_s", m.tail_s, 0.0, 1.0);
        c.number("motes.fade_from_m", m.fade_from_m, 0.0, m.cube_m);
        c.number("motes.brightness", m.brightness, 0.0, 1.0);
    }
}

impl Starfield {
    /// Parse and validate `data/space/starfield.json`'s text (`file` names it in an error).
    pub fn load(file: &str, text: &str) -> Result<Self, DataError> {
        parse(file, text)
    }
}

/// A sky file as written (design 9). Every key is named so an unknown one is refused; the generator's notes (its
/// label, name, look, refinement settings and the ambient's coefficients) are kept for that and not read.
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct SkyFile {
    schema: String,
    id: String,
    label: String,
    name: String,
    #[serde(rename = "for")]
    for_: String,
    look: String,
    mesh: MeshFile,
    dust_stars: DustStars,
    stars: StarsFile,
    ambient: AmbientFile,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct MeshFile {
    vertices: usize,
    triangles: usize,
    base_subdivisions: u32,
    vertex_budget: usize,
    error_bytes: f64,
    vertices_b64: Vec<String>,
    triangles_b64: Vec<String>,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct DustStars {
    density: f64,
    brightness: f64,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct StarsFile {
    count: usize,
    list: Vec<[f64; 6]>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct AmbientFile {
    strength: f64,
    cube_linear: [[f64; 3]; 6],
    sh9_linear: [[f64; 3]; 9],
    dominant_srgb: [f64; 3],
}

impl Validate for SkyFile {
    fn validate(&self, c: &mut Checks) {
        c.equals("schema", &self.schema, "starcrew.sky/1");
        let m = &self.mesh;
        c.count("mesh.vertices", m.vertices as i64, 4, MAX_SKY_VERTICES.min(m.vertex_budget) as i64);
        c.count("mesh.triangles", m.triangles as i64, 4, 2 * MAX_SKY_VERTICES as i64);
        c.number("mesh.error_bytes", m.error_bytes, 0.0, 255.0);
        c.number("dust_stars.density", self.dust_stars.density, 0.0, 1.0);
        c.number("dust_stars.brightness", self.dust_stars.brightness, 0.0, 4.0);
        c.count("stars.count", self.stars.count as i64, self.stars.list.len() as i64, self.stars.list.len() as i64);
        c.count("stars.list", self.stars.list.len() as i64, 0, MAX_SKY_STARS as i64);
        // Homeworld's lighting files never went over 40% ambient: 0.13 linear (design 6).
        c.number("ambient.strength", self.ambient.strength, 0.0, 0.13);
        for (i, f) in self.ambient.cube_linear.iter().enumerate() {
            for (k, x) in f.iter().enumerate() {
                c.number(&format!("ambient.cube_linear[{i}][{k}]"), *x, 0.0, 1.0);
            }
        }
    }
}

/// One vector star.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SkyStar {
    /// Its direction, system frame, unit length.
    pub dir: [f32; 3],
    /// Its brightness, 0-1, already dimmed by the dust in front of it.
    pub brightness: f32,
    /// Its class, an index of `Starfield::classes`.
    pub class: u8,
    /// Its colour, an index of `Starfield::colours`.
    pub colour: u8,
}

/// A sky, decoded and checked: what the renderer uploads and the outside light reads.
#[derive(Debug, Clone, PartialEq)]
pub struct NebulaSky {
    /// Its id.
    pub id: String,
    /// Each vertex's direction, system frame, unit length.
    pub directions: Vec<[f32; 3]>,
    /// Each vertex's colour as sRGB bytes, and in the fourth byte the share of the stars behind it the nebula hides
    /// (0-255).
    pub rgba: Vec<[u8; 4]>,
    /// Three indices a triangle, counter-clockwise seen from the centre.
    pub indices: Vec<u16>,
    /// The vector stars, brightest first.
    pub stars: Vec<SkyStar>,
    /// The faint hashed stars' density and brightness (`sky.glsl`'s grids, design 5.5).
    pub dust_stars: [f32; 2],
    /// The ambient's strength: the brightest face's linear luminance (design 6).
    pub ambient_strength: f32,
    /// The linear RGB a white surface takes facing +x, -x, +y, -y, +z and -z, system frame (design 6).
    pub ambient_cube: [[f32; 3]; 6],
}

/// The standard base64 alphabet's value of `b`, or none.
fn b64_value(b: u8) -> Option<u32> {
    Some(match b {
        b'A'..=b'Z' => b - b'A',
        b'a'..=b'z' => b - b'a' + 26,
        b'0'..=b'9' => b - b'0' + 52,
        b'+' => 62,
        b'/' => 63,
        _ => return None,
    } as u32)
}

/// Decode rows of standard base64 (each row padded on its own, as the generator writes them) into one byte string.
fn decode_rows(rows: &[String]) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    for (r, row) in rows.iter().enumerate() {
        let s = row.as_bytes();
        if s.len() % 4 != 0 {
            return Err(format!("row {r} is not whole base64 quads"));
        }
        for (q, quad) in s.chunks(4).enumerate() {
            let last = (q + 1) * 4 == s.len();
            let pad = quad.iter().rev().take_while(|b| **b == b'=').count();
            if pad > 2 || (pad > 0 && !last) {
                return Err(format!("row {r} has padding inside it"));
            }
            let mut n = 0u32;
            for &b in &quad[..4 - pad] {
                n = (n << 6) | b64_value(b).ok_or_else(|| format!("row {r} holds {:?}, not base64", b as char))?;
            }
            n <<= 6 * pad as u32;
            let bytes = [(n >> 16) as u8, (n >> 8) as u8, n as u8];
            out.extend_from_slice(&bytes[..3 - pad]);
        }
    }
    Ok(out)
}

/// A direction from two 16-bit octahedral coordinates (the generator's `oct_decode`), unit length.
pub fn oct_decode(u: u16, v: u16) -> [f32; 3] {
    let px = f64::from(u) / 65535.0 * 2.0 - 1.0;
    let py = f64::from(v) / 65535.0 * 2.0 - 1.0;
    let z = 1.0 - px.abs() - py.abs();
    let t = (-z).max(0.0);
    let x = px + if px >= 0.0 { -t } else { t };
    let y = py + if py >= 0.0 { -t } else { t };
    let l = (x * x + y * y + z * z).sqrt();
    [(x / l) as f32, (y / l) as f32, (z / l) as f32]
}

impl NebulaSky {
    /// Parse, decode and check the sky file `file` (its text `text`) against the stars' `starfield`: the counts match
    /// the rows, every index is a vertex, every direction is finite and unit, every star's class and colour exist. The
    /// first problem stops it, naming the file and the field.
    pub fn load(file: &str, text: &str, starfield: &Starfield) -> Result<Self, DataError> {
        let f: SkyFile = parse(file, text)?;
        let err = |field: &str, message: String| DataError { file: file.to_owned(), field: field.to_owned(), message };
        let m = &f.mesh;
        let vb = decode_rows(&m.vertices_b64).map_err(|e| err("mesh.vertices_b64", e))?;
        if vb.len() != 8 * m.vertices {
            return Err(err(
                "mesh.vertices_b64",
                format!("{} bytes, not 8 for each of {} vertices", vb.len(), m.vertices),
            ));
        }
        let tb = decode_rows(&m.triangles_b64).map_err(|e| err("mesh.triangles_b64", e))?;
        if tb.len() != 6 * m.triangles {
            return Err(err(
                "mesh.triangles_b64",
                format!("{} bytes, not 6 for each of {} triangles", tb.len(), m.triangles),
            ));
        }
        let le = |b: &[u8]| u16::from_le_bytes([b[0], b[1]]);
        let mut directions = Vec::with_capacity(m.vertices);
        let mut rgba = Vec::with_capacity(m.vertices);
        for v in vb.chunks_exact(8) {
            directions.push(oct_decode(le(&v[0..2]), le(&v[2..4])));
            rgba.push([v[4], v[5], v[6], v[7]]);
        }
        let indices: Vec<u16> = tb.chunks_exact(2).map(le).collect();
        if let Some((k, i)) = indices.iter().enumerate().find(|(_, i)| usize::from(**i) >= m.vertices) {
            return Err(err(
                "mesh.triangles_b64",
                format!("index {i} (triangle {}) is past the {} vertices", k / 3, m.vertices),
            ));
        }
        let mut stars = Vec::with_capacity(f.stars.list.len());
        for (i, s) in f.stars.list.iter().enumerate() {
            let field = format!("stars.list[{i}]");
            let len = (s[0] * s[0] + s[1] * s[1] + s[2] * s[2]).sqrt();
            if !len.is_finite() || (len - 1.0).abs() > 1e-3 {
                return Err(err(&field, format!("its direction must be unit length, is {len}")));
            }
            if !(0.0..=1.0).contains(&s[3]) {
                return Err(err(&field, format!("its brightness must be in 0-1, is {}", s[3])));
            }
            let class = s[4];
            let colour = s[5];
            if class.fract() != 0.0 || class < 0.0 || class as usize >= starfield.classes.len() {
                return Err(err(&field, format!("class {class} is not one of {STARFIELD_FILE}'s classes")));
            }
            if colour.fract() != 0.0 || colour < 0.0 || colour as usize >= starfield.colours.len() {
                return Err(err(&field, format!("colour {colour} is not one of {STARFIELD_FILE}'s colours")));
            }
            let d = [(s[0] / len) as f32, (s[1] / len) as f32, (s[2] / len) as f32];
            stars.push(SkyStar { dir: d, brightness: s[3] as f32, class: class as u8, colour: colour as u8 });
        }
        let a = &f.ambient;
        Ok(Self {
            id: f.id,
            directions,
            rgba,
            indices,
            stars,
            dust_stars: [f.dust_stars.density as f32, f.dust_stars.brightness as f32],
            ambient_strength: a.strength as f32,
            ambient_cube: a.cube_linear.map(|c| c.map(|x| x as f32)),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    const STARFIELD: &str = include_str!("../../../data/space/starfield.json");
    const SKIES: [(&str, &str); 4] = [
        ("bloom", include_str!("../../../data/space/skies/bloom.json")),
        ("quiet", include_str!("../../../data/space/skies/quiet.json")),
        ("ember", include_str!("../../../data/space/skies/ember.json")),
        ("verdigris", include_str!("../../../data/space/skies/verdigris.json")),
    ];

    fn starfield() -> Starfield {
        Starfield::load(STARFIELD_FILE, STARFIELD).expect("the shipped starfield loads")
    }

    fn sky(id: &str, text: &str) -> NebulaSky {
        NebulaSky::load(&sky_file(id), text, &starfield()).expect("the shipped sky loads")
    }

    #[test]
    fn every_shipped_sky_decodes_within_the_budget() {
        for (id, text) in SKIES {
            let s = sky(id, text);
            assert_eq!(s.id, id);
            assert!(s.directions.len() <= MAX_SKY_VERTICES, "{id}: {} vertices", s.directions.len());
            assert!(s.directions.len() > 7_000, "{id}: the refinement spent its budget (design 3)");
            assert_eq!(s.indices.len() % 3, 0);
            assert!((1_400..=2_400).contains(&s.stars.len()), "{id}: {} stars (design 5.1)", s.stars.len());
            for d in &s.directions {
                let l = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
                assert!((l - 1.0).abs() < 1e-5, "{id}: a direction of length {l}");
            }
        }
    }

    #[test]
    fn every_shipped_sky_is_a_closed_sphere_wound_toward_its_centre() {
        // A hole would show the clear colour through the sky; a triangle wound outward would be culled by a renderer
        // that culls, and says the generator's order changed.
        for (id, text) in SKIES {
            let s = sky(id, text);
            let mut edges: HashMap<(u16, u16), u32> = HashMap::new();
            for t in s.indices.chunks_exact(3) {
                for (a, b) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
                    *edges.entry((a.min(b), a.max(b))).or_default() += 1;
                }
                let p = [t[0], t[1], t[2]].map(|i| glam::Vec3::from(s.directions[usize::from(i)]));
                let n = (p[1] - p[0]).cross(p[2] - p[0]);
                assert!(n.dot(p[0] + p[1] + p[2]) < 0.0, "{id}: triangle {t:?} faces away from the centre");
            }
            let open = edges.values().filter(|n| **n != 2).count();
            assert_eq!(open, 0, "{id}: {open} edges not shared by exactly two triangles");
            // Euler's formula for a sphere: V - E + F = 2.
            let (v, e, f) = (s.directions.len() as i64, edges.len() as i64, (s.indices.len() / 3) as i64);
            assert_eq!(v - e + f, 2, "{id}: not one closed sphere");
        }
    }

    #[test]
    fn the_octahedral_mapping_round_trips_the_axes() {
        let near = |a: [f32; 3], b: [f32; 3]| a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-4);
        assert!(near(oct_decode(32768, 32768), [0.0, 0.0, 1.0]), "the centre is +z");
        assert!(near(oct_decode(65535, 32768), [1.0, 0.0, 0.0]), "the right edge is +x");
        assert!(near(oct_decode(0, 0), [0.0, 0.0, -1.0]), "a corner is -z");
    }

    #[test]
    fn base64_rows_decode_with_their_own_padding() {
        assert_eq!(decode_rows(&["TWFu".into(), "TWE=".into(), "TQ==".into()]).unwrap(), b"ManMaM".to_vec());
        assert!(decode_rows(&["TQ==TWFu".into()]).is_err(), "padding mid row");
        assert!(decode_rows(&["TW!u".into()]).is_err(), "a byte outside the alphabet");
    }

    #[test]
    fn an_index_past_the_vertices_stops_the_load_naming_the_field() {
        // The spec's scenario "A bad sky file".
        let mut v: serde_json::Value = serde_json::from_str(SKIES[0].1).unwrap();
        // Four triangles, the first (0, 1, 65535): "AAABAP//" is 00 00 01 00 ff ff, "AAABAAIA" is (0, 1, 2).
        v["mesh"]["triangles_b64"] = serde_json::json!(["AAABAP//", "AAABAAIA", "AAABAAIA", "AAABAAIA"]);
        v["mesh"]["triangles"] = serde_json::json!(4);
        let e = NebulaSky::load("data/space/skies/bad.json", &v.to_string(), &starfield()).unwrap_err();
        assert_eq!(e.file, "data/space/skies/bad.json");
        assert_eq!(e.field, "mesh.triangles_b64");
        assert!(e.message.contains("65535"), "{e}");
    }

    #[test]
    fn a_star_of_a_class_the_starfield_lacks_is_refused() {
        let mut v: serde_json::Value = serde_json::from_str(SKIES[0].1).unwrap();
        v["stars"]["list"][0][4] = serde_json::json!(9);
        let e = NebulaSky::load("x.json", &v.to_string(), &starfield()).unwrap_err();
        assert_eq!(e.field, "stars.list[0]");
    }

    #[test]
    fn a_count_that_disagrees_with_its_rows_is_refused() {
        let mut v: serde_json::Value = serde_json::from_str(SKIES[0].1).unwrap();
        let n = v["mesh"]["vertices"].as_u64().unwrap();
        v["mesh"]["vertices"] = serde_json::json!(n - 1);
        let e = NebulaSky::load("x.json", &v.to_string(), &starfield()).unwrap_err();
        assert_eq!(e.field, "mesh.vertices_b64");
    }

    #[test]
    fn a_streak_floor_above_one_is_refused() {
        let mut v: serde_json::Value = serde_json::from_str(STARFIELD).unwrap();
        v["streak"]["dim_floor"] = serde_json::json!(1.5);
        let e = Starfield::load("x.json", &v.to_string()).unwrap_err();
        assert_eq!(e.field, "streak.dim_floor");
    }

    #[test]
    fn the_ambient_cube_peaks_at_the_skys_strength() {
        // Design 6 and task 5.2: the brightest face's linear luminance is the recipe's strength.
        for (id, text) in SKIES {
            let s = sky(id, text);
            let lum = |c: [f32; 3]| 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2];
            let peak = s.ambient_cube.iter().map(|c| lum(*c)).fold(0.0, f32::max);
            assert!((peak - s.ambient_strength).abs() < 0.002, "{id}: peak {peak} against {}", s.ambient_strength);
        }
    }
}
