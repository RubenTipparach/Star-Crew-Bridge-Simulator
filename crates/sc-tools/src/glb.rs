//! A glTF binary (`.glb`) reader for the exterior models (openspec/changes/tern-exterior, design 3): the header, the
//! JSON and binary chunks, the accessors the models use, and their embedded PNG images.
//!
//! It lives in the tools because only `deckc` reads a model; the client reads what `deckc` writes. It reads what the
//! exterior generator (`tools/blender/build_ship_exteriors.py`, Blender's glTF exporter) writes and nothing more:
//! anything else it meets (a second buffer, a sparse accessor, a node transform, a primitive that is not triangles) is
//! an error naming the file, never a silent skip (CLAUDE.md 6.5).

use serde_json::Value;
use std::path::Path;

/// One primitive of the model's mesh: a material's triangles.
pub struct Primitive {
    /// Its material's name.
    pub material: String,
    /// Positions, metres, in the model's frame.
    pub positions: Vec<[f32; 3]>,
    /// Unit normals.
    pub normals: Vec<[f32; 3]>,
    /// Texture coordinates (zeros where the primitive has none).
    pub uvs: Vec<[f32; 2]>,
    /// Vertex colours times the material's base colour factor, linear RGB.
    pub colours: Vec<[f32; 3]>,
    /// Three indices a triangle.
    pub indices: Vec<u32>,
    /// The image its base colour samples (an index into `Glb::images`), or None for an untextured material.
    pub image: Option<usize>,
}

/// An embedded image, decoded.
pub struct Image {
    /// Its name.
    pub name: String,
    /// Width and height, pixels.
    pub width: u32,
    /// Height, pixels.
    pub height: u32,
    /// RGBA8, rows from the top.
    pub rgba: Vec<u8>,
}

/// A model read from a glb.
pub struct Glb {
    /// Its mesh's primitives, in file order.
    pub primitives: Vec<Primitive>,
    /// Its embedded images, in file order.
    pub images: Vec<Image>,
}

impl Glb {
    /// Triangles in all primitives.
    pub fn triangles(&self) -> usize {
        self.primitives.iter().map(|p| p.indices.len() / 3).sum()
    }

    /// The bounds of every position: the lowest and the highest corner.
    pub fn bounds(&self) -> ([f32; 3], [f32; 3]) {
        let mut lo = [f32::MAX; 3];
        let mut hi = [f32::MIN; 3];
        for p in self.primitives.iter().flat_map(|p| &p.positions) {
            for k in 0..3 {
                lo[k] = lo[k].min(p[k]);
                hi[k] = hi[k].max(p[k]);
            }
        }
        (lo, hi)
    }
}

/// Read the model at `path`.
pub fn read(path: &Path) -> Result<Glb, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    parse(&bytes).map_err(|e| format!("{}: {e}", path.display()))
}

fn u32_at(b: &[u8], at: usize) -> Result<u32, String> {
    b.get(at..at + 4).map(|s| u32::from_le_bytes(s.try_into().unwrap())).ok_or_else(|| "truncated".to_owned())
}

/// Parse a glb's bytes.
pub fn parse(b: &[u8]) -> Result<Glb, String> {
    if b.len() < 20 || &b[0..4] != b"glTF" {
        return Err("not a glTF binary (no glTF header)".into());
    }
    if u32_at(b, 4)? != 2 {
        return Err("not glTF version 2".into());
    }
    let json_len = u32_at(b, 12)? as usize;
    if &b[16..20] != b"JSON" {
        return Err("the first chunk is not JSON".into());
    }
    let json = b.get(20..20 + json_len).ok_or("the JSON chunk runs past the end")?;
    let js: Value = serde_json::from_slice(json).map_err(|e| format!("the JSON chunk: {e}"))?;
    let at = 20 + json_len;
    let bin_len = u32_at(b, at)? as usize;
    if b.get(at + 4..at + 8) != Some(b"BIN\0".as_slice()) {
        return Err("the second chunk is not BIN".into());
    }
    let bin = b.get(at + 8..at + 8 + bin_len).ok_or("the BIN chunk runs past the end")?;
    let model = Model { js: &js, bin };
    model.check_shape()?;

    let mut images = Vec::new();
    for (i, im) in arr(&js, "images").iter().enumerate() {
        let data = model.view(int(im, "bufferView")?)?;
        if im["mimeType"].as_str() != Some("image/png") {
            return Err(format!("image {i} is not a PNG"));
        }
        let (width, height, rgba) = decode_png(data).map_err(|e| format!("image {i}: {e}"))?;
        images.push(Image { name: im["name"].as_str().unwrap_or("").to_owned(), width, height, rgba });
    }

    let mut primitives = Vec::new();
    for (k, p) in arr(&js["meshes"][0], "primitives").iter().enumerate() {
        if p.get("mode").is_some_and(|m| m.as_u64() != Some(4)) {
            return Err(format!("primitive {k} is not triangles"));
        }
        let a = &p["attributes"];
        let positions = model.floats::<3>(int(a, "POSITION")?)?;
        let n = positions.len();
        let normals = model.floats::<3>(int(a, "NORMAL")?)?;
        let uvs = match a.get("TEXCOORD_0") {
            Some(i) => model.floats::<2>(i.as_u64().ok_or("TEXCOORD_0 is not an index")? as usize)?,
            None => vec![[0.0; 2]; n],
        };
        let raw_colours = match a.get("COLOR_0") {
            Some(i) => model.floats::<4>(i.as_u64().ok_or("COLOR_0 is not an index")? as usize)?,
            None => vec![[1.0; 4]; n],
        };
        let indices = model.indices(int(p, "indices")?)?;
        if normals.len() != n || uvs.len() != n || raw_colours.len() != n {
            return Err(format!("primitive {k}: its attributes differ in length"));
        }
        if indices.len() % 3 != 0 || indices.iter().any(|&i| i as usize >= n) {
            return Err(format!("primitive {k}: its indices are not whole triangles of its vertices"));
        }
        let m = &js["materials"][int(p, "material")?];
        let pbr = &m["pbrMetallicRoughness"];
        let factor: [f32; 4] = match pbr.get("baseColorFactor").and_then(Value::as_array) {
            Some(f) if f.len() == 4 => std::array::from_fn(|c| f[c].as_f64().unwrap_or(1.0) as f32),
            Some(_) => return Err(format!("primitive {k}: a base colour factor of the wrong length")),
            None => [1.0; 4],
        };
        let image = match pbr.get("baseColorTexture") {
            Some(t) => {
                let tex = &js["textures"][int(t, "index")?];
                Some(int(tex, "source")?)
            }
            None => None,
        };
        if image.is_some_and(|i| i >= images.len()) {
            return Err(format!("primitive {k}: its texture's image is missing"));
        }
        let colours = raw_colours.iter().map(|c| [c[0] * factor[0], c[1] * factor[1], c[2] * factor[2]]).collect();
        let all_finite = positions.iter().flatten().chain(normals.iter().flatten()).all(|v| v.is_finite());
        if !all_finite {
            return Err(format!("primitive {k}: a position or normal is not finite"));
        }
        primitives.push(Primitive {
            material: m["name"].as_str().unwrap_or("").to_owned(),
            positions,
            normals,
            uvs,
            colours,
            indices,
            image,
        });
    }
    Ok(Glb { primitives, images })
}

fn arr<'a>(v: &'a Value, key: &str) -> &'a [Value] {
    v.get(key).and_then(Value::as_array).map(Vec::as_slice).unwrap_or(&[])
}

fn int(v: &Value, key: &str) -> Result<usize, String> {
    v.get(key).and_then(Value::as_u64).map(|x| x as usize).ok_or_else(|| format!("no `{key}`"))
}

struct Model<'a> {
    js: &'a Value,
    bin: &'a [u8],
}

/// An accessor's elements as the reader walks them.
struct Elements<'a> {
    /// Its buffer view's bytes from the accessor's offset on.
    data: &'a [u8],
    /// Bytes from one element to the next.
    stride: usize,
    /// How many elements.
    count: usize,
    /// The glTF component type (5121 u8, 5123 u16, 5125 u32, 5126 f32).
    ct: u64,
    /// Components an element (1 to 4).
    comps: usize,
    /// Whether integer components are normalized to 0-1.
    normalized: bool,
}
impl Model<'_> {
    /// The shape this reader handles: one buffer (the BIN chunk), one mesh on one node with no transform.
    fn check_shape(&self) -> Result<(), String> {
        let buffers = arr(self.js, "buffers");
        if buffers.len() != 1 || buffers[0].get("uri").is_some() {
            return Err("the model needs exactly one buffer, the BIN chunk".into());
        }
        if arr(self.js, "meshes").len() != 1 {
            return Err("the model needs exactly one mesh".into());
        }
        for n in arr(self.js, "nodes") {
            if ["matrix", "translation", "rotation", "scale"].iter().any(|k| n.get(k).is_some()) {
                return Err("a node carries a transform; the model must be in the ship's frame".into());
            }
        }
        Ok(())
    }

    fn view(&self, i: usize) -> Result<&[u8], String> {
        let v = &self.js["bufferViews"][i];
        let off = v.get("byteOffset").and_then(Value::as_u64).unwrap_or(0) as usize;
        let len = int(v, "byteLength")?;
        self.bin.get(off..off + len).ok_or_else(|| format!("buffer view {i} runs past the BIN chunk"))
    }

    /// An accessor's elements, checked to lie inside its buffer view.
    fn elements(&self, i: usize) -> Result<Elements<'_>, String> {
        let a = &self.js["accessors"][i];
        if a.get("sparse").is_some() {
            return Err(format!("accessor {i} is sparse"));
        }
        let view_i = int(a, "bufferView")?;
        let view = self.view(view_i)?;
        let off = a.get("byteOffset").and_then(Value::as_u64).unwrap_or(0) as usize;
        let ct = a["componentType"].as_u64().ok_or_else(|| format!("accessor {i}: no component type"))?;
        let comps = match a["type"].as_str() {
            Some("SCALAR") => 1,
            Some("VEC2") => 2,
            Some("VEC3") => 3,
            Some("VEC4") => 4,
            t => return Err(format!("accessor {i}: type {t:?} is not read")),
        };
        let size = match ct {
            5121 => 1,
            5123 => 2,
            5125 | 5126 => 4,
            t => return Err(format!("accessor {i}: component type {t} is not read")),
        };
        let stride = self.js["bufferViews"][view_i].get("byteStride").and_then(Value::as_u64).unwrap_or(0) as usize;
        let stride = if stride == 0 { size * comps } else { stride };
        let count = int(a, "count")?;
        let need = if count == 0 { 0 } else { off + (count - 1) * stride + size * comps };
        if need > view.len() {
            return Err(format!("accessor {i} runs past its buffer view"));
        }
        let normalized = a.get("normalized").and_then(Value::as_bool).unwrap_or(false);
        Ok(Elements { data: &view[off..], stride, count, ct, comps, normalized })
    }

    /// An accessor of N float lanes: floats as they are, normalized integers scaled to 0-1.
    fn floats<const N: usize>(&self, i: usize) -> Result<Vec<[f32; N]>, String> {
        let Elements { data: d, stride, count, ct, comps, normalized } = self.elements(i)?;
        if comps != N {
            return Err(format!("accessor {i} has {comps} components, {N} expected"));
        }
        if ct == 5125 || (ct != 5126 && !normalized) {
            return Err(format!("accessor {i}: integers that are not normalized are not read as floats"));
        }
        let lane = |e: usize, c: usize| -> f32 {
            let at = e * stride;
            match ct {
                5126 => f32::from_le_bytes(d[at + c * 4..at + c * 4 + 4].try_into().unwrap()),
                5123 => f32::from(u16::from_le_bytes(d[at + c * 2..at + c * 2 + 2].try_into().unwrap())) / 65535.0,
                _ => f32::from(d[at + c]) / 255.0,
            }
        };
        Ok((0..count).map(|e| std::array::from_fn(|c| lane(e, c))).collect())
    }

    /// An index accessor: unsigned bytes, shorts or ints.
    fn indices(&self, i: usize) -> Result<Vec<u32>, String> {
        let Elements { data: d, stride, count, ct, comps, .. } = self.elements(i)?;
        if comps != 1 {
            return Err(format!("accessor {i}: indices are scalars"));
        }
        Ok((0..count)
            .map(|e| {
                let at = e * stride;
                match ct {
                    5121 => u32::from(d[at]),
                    5123 => u32::from(u16::from_le_bytes(d[at..at + 2].try_into().unwrap())),
                    _ => u32::from_le_bytes(d[at..at + 4].try_into().unwrap()),
                }
            })
            .collect())
    }
}

/// Decode a PNG to RGBA8.
fn decode_png(data: &[u8]) -> Result<(u32, u32, Vec<u8>), String> {
    let mut dec = png::Decoder::new(std::io::Cursor::new(data));
    dec.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = dec.read_info().map_err(|e| e.to_string())?;
    let mut buf = vec![0u8; reader.output_buffer_size().ok_or("the image is too large")?];
    let info = reader.next_frame(&mut buf).map_err(|e| e.to_string())?;
    let (w, h) = (info.width, info.height);
    let px = (w * h) as usize;
    let rgba = match info.color_type {
        png::ColorType::Rgba => buf[..px * 4].to_vec(),
        png::ColorType::Rgb => buf[..px * 3].chunks_exact(3).flat_map(|c| [c[0], c[1], c[2], 255]).collect(),
        png::ColorType::GrayscaleAlpha => {
            buf[..px * 2].chunks_exact(2).flat_map(|c| [c[0], c[0], c[0], c[1]]).collect()
        }
        png::ColorType::Grayscale => buf[..px].iter().flat_map(|&g| [g, g, g, 255]).collect(),
        t => return Err(format!("colour type {t:?} is not read")),
    };
    Ok((w, h, rgba))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tern() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/models/exteriors/liveries/cobalt/tern.glb")
    }

    #[test]
    fn the_tern_reads_as_its_manifest_says() {
        let g = read(&tern()).expect("the committed model reads");
        assert_eq!(g.primitives.len(), 6, "six materials, six primitives");
        assert_eq!(g.triangles(), 4440, "props.json: 4,440 triangles");
        let (lo, hi) = g.bounds();
        let want = ([-22.5, -8.02, -44.2], [22.5, 10.02, 37.27]);
        for k in 0..3 {
            assert!((lo[k] - want.0[k]).abs() < 0.006 && (hi[k] - want.1[k]).abs() < 0.006, "bounds {lo:?} {hi:?}");
        }
        assert_eq!(g.images.len(), 3);
        let hull = g.primitives.iter().find(|p| p.material == "tern_hull").expect("the hull's primitive");
        let im = &g.images[hull.image.expect("the hull is textured")];
        assert_eq!((im.width, im.height, im.rgba.len()), (1024, 1024, 1024 * 1024 * 4));
        assert!(g.primitives.iter().any(|p| p.image.is_none()), "the glowing materials have no texture");
    }

    #[test]
    fn a_truncated_model_is_refused() {
        let b = std::fs::read(tern()).unwrap();
        assert!(parse(&b[..b.len() / 2]).is_err(), "half a file is an error, not half a model");
        assert!(parse(b"not a model at all").is_err());
    }
}
