//! Render tests (engine-stack design section 12, task 3.5): draw known deck vertices through the
//! real deck pipeline, headless (SDL's offscreen driver; Mesa's llvmpipe over EGL, OpenGL ES 3.0, in
//! a cloud session), read the output back and check what reached the shader.
//!
//! It proves the passes compose and the 28-byte vertex is read as `sc-core` packs it (the
//! engine-platform scenario "the deck vertex against its pipeline"). It proves nothing about the
//! Pi's speed. The whole frame is also compared with `tests/reference/deck_vertex.png`; a missing
//! reference is written for review, and SC_UPDATE_REFERENCES=1 rewrites it.
//!
//! The mover byte is checked by `sc-core`'s unit tests; the shader does not read it until movers
//! are drawn.

use glam::Vec3;
use sc_client::platform::{self, App, Flow, Frame, WindowConfig};
use sc_core::vertex::{pack_deck_vertex, DeckVertexIn};
use sc_render::{DeckParams, DeckProgram, Indices, Renderer, Target, TextureArray};
use std::path::PathBuf;
use std::process::ExitCode;

const SIZE: u32 = 256;
/// The view: 32 m across 256 px, so a metre is 8 px.
const HALF_M: f32 = 16.0;

fn quad(x0: f32, y0: f32, x1: f32, y1: f32, layer: u8, colors: [[u8; 4]; 3], uv: [[f32; 2]; 4]) -> (Vec<u8>, Vec<u16>) {
    let mut v = Vec::new();
    for (k, (x, y)) in [(x0, y0), (x1, y0), (x1, y1), (x0, y1)].into_iter().enumerate() {
        let d = DeckVertexIn { position_m: [x, y, 0.0], mover: 9, layer, normal: [0.0, 0.0, 1.0], colors, uv: uv[k] };
        v.extend_from_slice(&pack_deck_vertex(&d).expect("test vertices fit the format"));
    }
    (v, vec![0, 1, 2, 0, 2, 3])
}

/// 256 layers of 2 x 2 px: texel (0, 0) of layer L is (L, 255 - L, 7), texel (1, 0) is (L / 2, 0, 255),
/// the bottom row grey.
fn layers() -> Vec<u8> {
    let mut out = Vec::new();
    for l in 0..=255u8 {
        out.extend_from_slice(&[l, 255 - l, 7, 255, l / 2, 0, 255, 255, 90, 90, 90, 255, 90, 90, 90, 255]);
    }
    out
}

struct Check {
    name: &'static str,
    /// Where to look, in metres.
    at: (f32, f32),
    want: [u8; 3],
}

const CHECKS: &[Check] = &[
    Check { name: "layer 200 (the sign bit of the packed lane) and its texel", at: (-12.0, 12.0), want: [200, 55, 7] },
    Check { name: "the red alert colour set alone", at: (-4.0, 12.0), want: [200, 0, 0] },
    Check { name: "the emergency colour set alone", at: (4.0, 12.0), want: [0, 0, 200] },
    Check { name: "half normal, half red alert", at: (12.0, 12.0), want: [132, 32, 32] },
    Check { name: "texture u in the first texel", at: (-14.0, 4.0), want: [5, 250, 7] },
    Check { name: "texture u in the second texel", at: (-2.0, 4.0), want: [2, 0, 255] },
    Check { name: "the normal lit by the dynamic light term", at: (4.0, 4.0), want: [128, 128, 128] },
    Check { name: "nothing drawn outside the quads", at: (12.0, -12.0), want: [0, 0, 0] },
];

struct Test {
    target: Target,
    tex: TextureArray,
    draws: Vec<(sc_render::Mesh, DeckProgram, [f32; 3], f32)>,
    failures: Vec<String>,
    r: Renderer,
}

impl Test {
    fn new() -> Result<Self, String> {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let text = std::fs::read_to_string(root.join("data/engine/render.json")).map_err(|e| e.to_string())?;
        let cfg = sc_core::data::parse("data/engine/render.json", &text).map_err(|e| e.to_string())?;
        let mut r = Renderer::new(&cfg);
        let target = r.make_target(SIZE, SIZE, 1);
        let tex = r.make_texture_array(2, &layers());
        let grey = [[128, 128, 128, 255]; 3];
        let sets = [[32, 32, 32, 255], [100, 0, 0, 255], [0, 0, 100, 255]];
        let still = [[0.25, 0.25]; 4];
        let across = [[0.0, 0.25], [1.0, 0.25], [1.0, 0.25], [0.0, 0.25]];
        let mut draws = Vec::new();
        let mut add = |m: (Vec<u8>, Vec<u16>), p, w, f| draws.push((r.make_mesh(&m.0, Indices::U16(&m.1)), p, w, f));
        add(quad(-16.0, 8.0, -8.0, 16.0, 200, grey, still), DeckProgram::Textured, [1.0, 0.0, 0.0], 0.0);
        add(quad(-8.0, 8.0, 0.0, 16.0, 0, sets, still), DeckProgram::Flat, [0.0, 1.0, 0.0], 0.0);
        add(quad(0.0, 8.0, 8.0, 16.0, 0, sets, still), DeckProgram::Flat, [0.0, 0.0, 1.0], 0.0);
        add(quad(8.0, 8.0, 16.0, 16.0, 0, sets, still), DeckProgram::Flat, [0.5, 0.5, 0.0], 0.0);
        add(quad(-16.0, 0.0, 0.0, 8.0, 5, grey, across), DeckProgram::Textured, [1.0, 0.0, 0.0], 0.0);
        add(quad(0.0, 0.0, 8.0, 8.0, 0, sets, still), DeckProgram::Flat, [1.0, 0.0, 0.0], 0.25);
        Ok(Self { target, tex, draws, failures: Vec::new(), r })
    }
}

impl App for Test {
    fn frame(&mut self, f: &Frame) -> Flow {
        let mvp = glam::camera::rh::proj::opengl::orthographic(-HALF_M, HALF_M, -HALF_M, HALF_M, -1.0, 1.0);
        self.r.begin_3d(&self.target, [0.0, 0.0, 0.0, 1.0]);
        for (mesh, prog, w, flash) in &self.draws {
            let p = DeckParams {
                mvp,
                state_weights: *w,
                flash_dir: Vec3::Z,
                flash: *flash,
                panel_first: u32::MAX,
                panel_glow: 0.0,
            };
            self.r.draw_deck(&self.target, mesh, *prog, &p, Some(&self.tex));
        }
        self.r.end_pass();
        self.r.present(&self.target, f.width, f.height, false);
        let px = sc_render::read_output(f.width, f.height);
        self.r.commit();
        assert_eq!((f.width, f.height), (SIZE, SIZE), "the offscreen window is the size asked for");
        for c in CHECKS {
            let x = ((c.at.0 + HALF_M) / (2.0 * HALF_M) * SIZE as f32) as usize;
            let y = ((HALF_M - c.at.1) / (2.0 * HALF_M) * SIZE as f32) as usize;
            let i = (y * SIZE as usize + x) * 4;
            let got = [px[i], px[i + 1], px[i + 2]];
            let ok = got.iter().zip(c.want).all(|(g, w)| (i32::from(*g) - i32::from(w)).abs() <= 2);
            println!("{} {}: got {got:?}, want {:?}", if ok { "ok  " } else { "FAIL" }, c.name, c.want);
            if !ok {
                self.failures.push(c.name.to_owned());
            }
        }
        let refp = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/reference/deck_vertex.png");
        if let Err(e) = compare_reference(&refp, &px) {
            println!("FAIL reference image: {e}");
            self.failures.push("reference image".into());
        }
        if self.failures.is_empty() {
            Flow::Done
        } else {
            Flow::Fail
        }
    }
}

/// The frame against the committed reference: every channel of every pixel within 3 of it.
fn compare_reference(path: &std::path::Path, px: &[u8]) -> Result<(), String> {
    let update = std::env::var("SC_UPDATE_REFERENCES").is_ok_and(|v| v == "1");
    if update || !path.exists() {
        sc_client::capture::write_png(path, SIZE, SIZE, px).map_err(|e| e.to_string())?;
        println!("note reference image written to {}; look at it and commit it", path.display());
        return Ok(());
    }
    let dec = png::Decoder::new(std::io::BufReader::new(std::fs::File::open(path).map_err(|e| e.to_string())?));
    let mut reader = dec.read_info().map_err(|e| e.to_string())?;
    let mut buf = vec![0; reader.output_buffer_size().ok_or("reference too large")?];
    let info = reader.next_frame(&mut buf).map_err(|e| e.to_string())?;
    if (info.width, info.height) != (SIZE, SIZE) || info.color_type != png::ColorType::Rgba {
        return Err(format!("{} is not a {SIZE} px RGBA image", path.display()));
    }
    let bad = buf.iter().zip(px).filter(|(a, b)| (i32::from(**a) - i32::from(**b)).abs() > 3).count();
    if bad > 0 {
        let actual = path.with_extension("actual.png");
        let _ = sc_client::capture::write_png(&actual, SIZE, SIZE, px);
        return Err(format!("{bad} channel values differ by more than 3; this frame is in {}", actual.display()));
    }
    Ok(())
}

fn main() -> ExitCode {
    let cfg = WindowConfig {
        title: "render test".into(),
        width: SIZE,
        height: SIZE,
        fullscreen: false,
        headless: true,
        vsync: false,
    };
    let code = platform::run(cfg, |_| Test::new().map(|t| Box::new(t) as Box<dyn App>));
    println!("render tests: {}", if code == ExitCode::SUCCESS { "ok" } else { "FAILED" });
    code
}
