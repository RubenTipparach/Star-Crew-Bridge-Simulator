//! Validated data: the one loader every data file goes through, and the schemas the game and the
//! tools share (CLAUDE.md 6.5; engine-stack design section 9).
//!
//! It lives in the core because "one schema" means the server, the client and the tools read the
//! same structs. The core never reads a file: the caller hands it the file's path (for messages)
//! and its text. Every schema struct denies unknown keys, so a misspelt knob is an error, and the
//! load stops naming the file and the field. Keys starting with `_` are notes (`_doc`, `_rules`) and
//! are dropped before the schema sees them, as the repository's data files and tools treat them.

use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::Value;

/// Why a data file was refused: the file, the field (a dotted path) and what was wrong.
#[derive(Debug, Clone, PartialEq)]
pub struct DataError {
    /// The file as the caller named it.
    pub file: String,
    /// The field, as a dotted path (`pools.buffers`), or `.` for the whole file.
    pub field: String,
    /// What was wrong with it.
    pub message: String,
}

impl std::fmt::Display for DataError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}: {}", self.file, self.field, self.message)
    }
}
impl std::error::Error for DataError {}

/// The checks a schema runs after it parsed: ranges and finiteness, each naming its field.
#[derive(Default)]
pub struct Checks {
    first: Option<(String, String)>,
}

impl Checks {
    fn fail(&mut self, field: &str, message: String) {
        if self.first.is_none() {
            self.first = Some((field.to_owned(), message));
        }
    }
    /// A float must be finite and inside `[lo, hi]`.
    pub fn number(&mut self, field: &str, v: f64, lo: f64, hi: f64) {
        if !v.is_finite() {
            self.fail(field, format!("must be a finite number, got {v}"));
        } else if v < lo || v > hi {
            self.fail(field, format!("must be in {lo}-{hi}, got {v}"));
        }
    }
    /// An integer must be inside `[lo, hi]`.
    pub fn count(&mut self, field: &str, v: i64, lo: i64, hi: i64) {
        if v < lo || v > hi {
            self.fail(field, format!("must be in {lo}-{hi}, got {v}"));
        }
    }
    /// A string must equal `want` (a schema tag).
    pub fn equals(&mut self, field: &str, v: &str, want: &str) {
        if v != want {
            self.fail(field, format!("must be {want:?}, got {v:?}"));
        }
    }
}

/// A schema that checks itself once parsed.
pub trait Validate {
    /// Record every problem in `c`; the first one is reported.
    fn validate(&self, c: &mut Checks);
}

fn strip_notes(v: &mut Value) {
    match v {
        Value::Object(m) => {
            m.retain(|k, _| !k.starts_with('_'));
            m.values_mut().for_each(strip_notes);
        }
        Value::Array(a) => a.iter_mut().for_each(strip_notes),
        _ => {}
    }
}

/// Parse and validate `text`, the contents of `file`, as `T`; the first problem stops it.
pub fn parse<T: DeserializeOwned + Validate>(file: &str, text: &str) -> Result<T, DataError> {
    let err = |field: String, message: String| DataError { file: file.to_owned(), field, message };
    let mut v: Value = serde_json::from_str(text).map_err(|e| err(".".into(), e.to_string()))?;
    strip_notes(&mut v);
    let t: T = serde_path_to_error::deserialize(v).map_err(|e| err(e.path().to_string(), e.inner().to_string()))?;
    let mut c = Checks::default();
    t.validate(&mut c);
    match c.first {
        Some((field, message)) => Err(err(field, message)),
        None => Ok(t),
    }
}

/// `data/engine/render.json`: what the renderer allocates at startup (engine-stack design 5 and 8).
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RenderConfig {
    /// `starcrew.render/1`.
    pub schema: String,
    /// sokol_gfx's resource pools, fixed at setup (CLAUDE.md 2: allocate up front).
    pub pools: RenderPools,
    /// The most uniform data drawn in one frame, in bytes (sokol_gfx's `uniform_buffer_size`).
    pub uniform_buffer_bytes: i64,
}

/// sokol_gfx's pool sizes: how many of each resource can exist at once.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RenderPools {
    /// Vertex and index buffers.
    pub buffers: i64,
    /// Images: textures and render targets.
    pub images: i64,
    /// Samplers.
    pub samplers: i64,
    /// Shader programs.
    pub shaders: i64,
    /// Pipelines (a shader with its vertex layout and state).
    pub pipelines: i64,
    /// Views (an image bound as a texture or an attachment).
    pub views: i64,
}

impl Validate for RenderConfig {
    fn validate(&self, c: &mut Checks) {
        c.equals("schema", &self.schema, "starcrew.render/1");
        let p = &self.pools;
        for (k, v) in [("buffers", p.buffers), ("images", p.images), ("samplers", p.samplers), ("shaders", p.shaders)] {
            c.count(&format!("pools.{k}"), v, 1, 65_536);
        }
        c.count("pools.pipelines", p.pipelines, 1, 65_536);
        c.count("pools.views", p.views, 1, 65_536);
        c.count("uniform_buffer_bytes", self.uniform_buffer_bytes, 1024, 64 << 20);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOOD: &str = r#"{ "schema": "starcrew.render/1", "_doc": "a note",
        "pools": { "buffers": 256, "images": 64, "samplers": 16, "shaders": 16, "pipelines": 64, "views": 128 },
        "uniform_buffer_bytes": 4194304 }"#;

    #[test]
    fn the_shipped_render_config_loads() {
        let text = include_str!("../../../data/engine/render.json");
        let c: RenderConfig = parse("data/engine/render.json", text).expect("the shipped file is valid");
        assert!(c.pools.pipelines > 0);
    }

    #[test]
    fn notes_are_dropped_and_a_good_file_loads() {
        let c: RenderConfig = parse("render.json", GOOD).unwrap();
        assert_eq!(c.pools.buffers, 256);
    }

    #[test]
    fn a_misspelt_key_stops_the_load_naming_the_field() {
        let bad = GOOD.replace("\"samplers\"", "\"samplerz\"");
        let e = parse::<RenderConfig>("render.json", &bad).unwrap_err();
        assert_eq!(e.file, "render.json");
        assert!(e.field.starts_with("pools"), "the error names where the bad key is: {e}");
        assert!(e.message.contains("samplerz"), "and the key itself: {e}");
    }

    #[test]
    fn a_number_out_of_range_stops_the_load_naming_the_field() {
        let bad = GOOD.replace("\"pipelines\": 64", "\"pipelines\": 0");
        let e = parse::<RenderConfig>("render.json", &bad).unwrap_err();
        assert_eq!(e.field, "pools.pipelines", "{e}");
    }

    #[test]
    fn a_non_finite_value_stops_the_load() {
        let mut c = Checks::default();
        c.number("power.capacity_mj", f64::INFINITY, 0.0, 1e9);
        assert_eq!(c.first.map(|(f, _)| f).as_deref(), Some("power.capacity_mj"));
        let bad = GOOD.replace("4194304", "1e999");
        let e = parse::<RenderConfig>("render.json", &bad).unwrap_err();
        assert_eq!(e.file, "render.json", "an unrepresentable number is refused at the edge: {e}");
    }
}
