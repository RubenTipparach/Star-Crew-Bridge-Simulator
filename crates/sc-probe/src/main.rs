//! `sc-probe`: the measurement instrument that comes before the engine (engine-stack design
//! section 11; CLAUDE.md section 4's exception). It draws synthetic scenes through `sc-render`,
//! full screen at the output size with 3D at 1280 x 720, and writes a JSON report and a Markdown
//! summary of frame times (p50, p95, p99), the CPU time spent submitting, and what sokol_gfx counted.
//!
//! Scenes built so far: 1 (the triangle curve), 2 (the draw-call curve), 3 (fill rate: overdraw,
//! the texture array fetch, MSAA 4x), 4 (render to texture) and 8 (sokol_gfx's own cost: CPU per
//! draw from scenes 1 and 2, and the memory its pools took). Scene 5 (UI) waits for the egui
//! painter, scene 6 (memory with the Tern's decks) for `deckc`, scene 7 for the ship systems.
//!
//! Each frame waits for the GPU before it is shown (`sc_render::finish`), so a frame's time is its
//! whole cost; the CPU submit time is measured before that wait.
//!
//! Its numbers mean something only on a Raspberry Pi 5 (CLAUDE.md 2). Anywhere else it proves that
//! every scene draws, and the report says so in its first line.
//!
//! Usage: sc-probe [--headless] [--quick] [--frames 120] [--warmup 30] [--repeats 3]
//!                 [--width 1920 --height 1080] [--out docs/benchmarks/<date>-<name>] [--captures DIR]

mod geometry;
mod report;

use glam::Mat4;
use sc_client::platform::{self, App, Event, Flow, Frame, GlInfo, WindowConfig};
use sc_core::data::{parse, RenderConfig};
use sc_render::{DeckParams, DeckProgram, Indices, Mesh, Renderer, Target, TextureArray};
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Instant;

/// The 3D target's size (engine-stack design section 5: 3D at 1280 x 720, scaled up to the output).
const RENDER_3D: (u32, u32) = (1280, 720);
/// Grid meshes the triangle and call curves draw (scene 1: "in 50 calls"), one a tile of the view.
const GRID_MESHES: u32 = 50;
/// The view: orthographic, 32 m by 18 m, depth -20 to 20 m.
const VIEW: (f32, f32) = (16.0, 9.0);

#[derive(Clone, Debug)]
struct Options {
    headless: bool,
    quick: bool,
    frames: u32,
    warmup: u32,
    repeats: u32,
    width: u32,
    height: u32,
    out: Option<PathBuf>,
    captures: Option<PathBuf>,
    root: PathBuf,
}

fn options() -> Result<Options, String> {
    let mut o = Options {
        headless: false,
        quick: false,
        frames: 120,
        warmup: 30,
        repeats: 3,
        width: 1920,
        height: 1080,
        out: None,
        captures: None,
        root: PathBuf::from("."),
    };
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        let mut val = || args.next().ok_or(format!("{a} needs a value"));
        match a.as_str() {
            "--headless" => o.headless = true,
            "--quick" => o.quick = true,
            "--frames" => o.frames = val()?.parse().map_err(|e| format!("--frames: {e}"))?,
            "--warmup" => o.warmup = val()?.parse().map_err(|e| format!("--warmup: {e}"))?,
            "--repeats" => o.repeats = val()?.parse().map_err(|e| format!("--repeats: {e}"))?,
            "--width" => o.width = val()?.parse().map_err(|e| format!("--width: {e}"))?,
            "--height" => o.height = val()?.parse().map_err(|e| format!("--height: {e}"))?,
            "--out" => o.out = Some(PathBuf::from(val()?)),
            "--captures" => o.captures = Some(PathBuf::from(val()?)),
            "--root" => o.root = PathBuf::from(val()?),
            _ => return Err(format!("unknown option {a}")),
        }
    }
    if o.frames == 0 || o.repeats == 0 {
        return Err("--frames and --repeats must be at least 1".into());
    }
    Ok(o)
}

/// One measured configuration of a scene.
#[derive(Clone, Debug)]
pub enum Step {
    /// Scene 1: `tris` triangles in 50 calls.
    Triangles { tris: u32 },
    /// Scene 2: 100,000 triangles in `calls` calls.
    Calls { calls: u32 },
    /// Scene 3: full-view quads at `layers` overdraw.
    Fill { layers: u32, textured: bool, msaa: bool },
    /// Scene 4: 30,000 triangles drawn into a `w` x `h` target, which is then shown.
    Rtt { w: u32, h: u32 },
}

impl Step {
    fn scene(&self) -> u8 {
        match self {
            Step::Triangles { .. } => 1,
            Step::Calls { .. } => 2,
            Step::Fill { .. } => 3,
            Step::Rtt { .. } => 4,
        }
    }
    fn label(&self) -> String {
        match self {
            Step::Triangles { tris } => format!("triangles-{tris}"),
            Step::Calls { calls } => format!("calls-{calls}"),
            Step::Fill { layers, textured, msaa } => {
                format!(
                    "fill-{layers}x-{}{}",
                    if *textured { "textured" } else { "flat" },
                    if *msaa { "-msaa4" } else { "" }
                )
            }
            Step::Rtt { w, h } => format!("rtt-{w}x{h}"),
        }
    }
}

fn steps(quick: bool) -> Vec<Step> {
    let tris: &[u32] = if quick {
        &[50_000, 200_000, 500_000, 1_000_000]
    } else {
        &[50_000, 100_000, 200_000, 300_000, 400_000, 500_000, 600_000, 800_000, 1_000_000]
    };
    let calls: &[u32] = if quick { &[50, 200, 1000, 2000] } else { &[50, 100, 200, 500, 1000, 2000] };
    let mut s: Vec<Step> = tris.iter().map(|&t| Step::Triangles { tris: t }).collect();
    s.extend(calls.iter().map(|&c| Step::Calls { calls: c }));
    for layers in [1, 2, 4, 8] {
        for textured in [false, true] {
            s.push(Step::Fill { layers, textured, msaa: false });
        }
    }
    s.push(Step::Fill { layers: 4, textured: true, msaa: true });
    s.extend([(512, 256), (1024, 512), (2048, 1024)].map(|(w, h)| Step::Rtt { w, h }));
    s
}

/// What one step measured in one repeat.
#[derive(Clone, Debug, Default)]
pub struct Sample {
    /// Milliseconds between frame starts (the whole frame, swap included).
    pub frame_ms: Vec<f64>,
    /// Milliseconds the CPU spent issuing the frame's sokol_gfx calls.
    pub submit_ms: Vec<f64>,
    /// sokol_gfx's counts for the step's last frame.
    pub draws: u32,
    /// Triangles the step submitted a frame.
    pub triangles: u64,
}

// Fields drop in order: the meshes and textures before the renderer they belong to.
struct Probe {
    o: Options,
    grids: Vec<Mesh>,
    grid_cells: u32,
    quads: Mesh,
    rtt_scene: Vec<Mesh>,
    tex: TextureArray,
    target: Target,
    target_msaa: Target,
    rtt: Vec<((u32, u32), Target)>,
    steps: Vec<Step>,
    step: usize,
    repeat: u32,
    frame_in_step: u32,
    samples: Vec<Vec<Sample>>,
    info: report::RunInfo,
    started: Instant,
    r: Renderer,
}

fn ortho() -> Mat4 {
    glam::camera::rh::proj::opengl::orthographic(-VIEW.0, VIEW.0, -VIEW.1, VIEW.1, -20.0, 20.0)
}

fn params() -> DeckParams {
    DeckParams {
        mvp: ortho(),
        state_weights: [1.0, 0.0, 0.0],
        flash_dir: glam::Vec3::Z,
        flash: 0.0,
        panel_first: u32::MAX,
        panel_glow: 0.0,
    }
}

impl Probe {
    /// The 50 grid tiles at `cells` a side, each covering its whole tile (10 columns by 5 rows, no
    /// overlap), made when a step needs a different density.
    fn grids_at(&mut self, cells: u32) {
        if self.grid_cells == cells {
            return;
        }
        self.grids.clear();
        let (cols, rows) = (10, 5);
        let (tw, th) = (2.0 * VIEW.0 / cols as f32, 2.0 * VIEW.1 / rows as f32);
        for k in 0..GRID_MESHES {
            let (c, rw) = ((k % cols) as f32, (k / cols) as f32);
            let x0 = -VIEW.0 + c * tw;
            let y0 = -VIEW.1 + rw * th;
            let g = geometry::grid(cells, x0, y0, x0 + tw, y0 + th, 0.0, k);
            self.grids.push(self.r.make_mesh(&g.vertices, Indices::U16(&g.indices)));
        }
        self.grid_cells = cells;
    }

    fn new(o: Options, gl: &GlInfo) -> Result<Self, String> {
        let path = o.root.join("data/engine/render.json");
        let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let cfg: RenderConfig = parse("data/engine/render.json", &text).map_err(|e| e.to_string())?;
        let rss_before = report::rss_bytes();
        let mut r = Renderer::new(&cfg);
        let rss_after_setup = report::rss_bytes();
        let q = geometry::quads(8, -VIEW.0, -VIEW.1, VIEW.0, VIEW.1, -10.0, 10.0, 2.0);
        let quads = r.make_mesh(&q.vertices, Indices::U16(&q.indices));
        // Scene 4's view: 30,000 triangles (a viewscreen's space scene, design section 5).
        let rtt_scene = (0..3)
            .map(|k| {
                let g = geometry::grid(70, -12.0 + 8.0 * k as f32, -6.0, -5.0 + 8.0 * k as f32, 6.0, 0.0, 99 + k);
                r.make_mesh(&g.vertices, Indices::U16(&g.indices))
            })
            .collect();
        let (size, layers) = geometry::pattern_layers();
        let tex = r.make_texture_array(size, &layers);
        let target = r.make_target(RENDER_3D.0, RENDER_3D.1, 1);
        let target_msaa = r.make_target(RENDER_3D.0, RENDER_3D.1, 4);
        let rtt =
            [(512, 256), (1024, 512), (2048, 1024)].map(|s| (s, r.make_target(s.0, s.1, 1))).into_iter().collect();
        let steps = steps(o.quick);
        let samples = vec![Vec::new(); steps.len()];
        let info = report::RunInfo::collect(gl, &o, rss_before, rss_after_setup, report::rss_bytes());
        println!(
            "sc-probe: {} on {} ({}), {} steps x {} repeats",
            info.gl_version,
            info.gl_renderer,
            info.video_driver,
            steps.len(),
            o.repeats
        );
        Ok(Self {
            o,
            grids: Vec::new(),
            grid_cells: 0,
            quads,
            rtt_scene,
            tex,
            target,
            target_msaa,
            rtt,
            steps,
            step: 0,
            repeat: 0,
            frame_in_step: 0,
            samples,
            info,
            started: Instant::now(),
            r,
        })
    }

    /// Draw one frame of `step`; returns the triangles it submitted.
    fn draw(&mut self, step: &Step, out_w: u32, out_h: u32) -> u64 {
        let p = params();
        let clear = [0.02, 0.025, 0.04, 1.0];
        match *step {
            Step::Triangles { tris } => {
                // Cells a side so 50 tiles hold about `tris` triangles: 2 c^2 each.
                self.grids_at(((f64::from(tris) / f64::from(GRID_MESHES) / 2.0).sqrt().round() as u32).max(1));
                let t = &self.target;
                self.r.begin_3d(t, clear);
                for m in &self.grids {
                    self.r.draw_deck(t, m, DeckProgram::Flat, &p, None);
                }
                self.r.end_pass();
                self.r.present(t, out_w, out_h, true);
                self.grids.iter().map(|m| m.triangles() as u64).sum()
            }
            Step::Calls { calls } => {
                // About 100,000 triangles (50 tiles of 32 x 32 cells: 102,400), split into `calls`
                // draws of consecutive index ranges.
                self.grids_at(32);
                let per_mesh_calls = (calls / GRID_MESHES).max(1) as usize;
                let t = &self.target;
                self.r.begin_3d(t, clear);
                let mut drawn = 0u64;
                for m in &self.grids {
                    let tris = m.triangles();
                    for c in 0..per_mesh_calls {
                        let (a, b) = (tris * c / per_mesh_calls, tris * (c + 1) / per_mesh_calls);
                        self.r.draw_deck_range(t, m, DeckProgram::Flat, &p, None, a * 3, (b - a) * 3);
                        drawn += (b - a) as u64;
                    }
                }
                self.r.end_pass();
                self.r.present(t, out_w, out_h, true);
                drawn
            }
            Step::Fill { layers, textured, msaa } => {
                let t = if msaa { &self.target_msaa } else { &self.target };
                let prog = if textured { DeckProgram::Textured } else { DeckProgram::Flat };
                self.r.begin_3d(t, clear);
                self.r.draw_deck_range(t, &self.quads, prog, &p, Some(&self.tex), 0, layers as usize * 6);
                self.r.end_pass();
                self.r.present(t, out_w, out_h, true);
                u64::from(layers) * 2
            }
            Step::Rtt { w, h } => {
                let rt = &self.rtt.iter().find(|(s, _)| *s == (w, h)).expect("every rtt size has a target").1;
                self.r.begin_3d(rt, [0.0, 0.0, 0.02, 1.0]);
                let mut tris = 0;
                for m in &self.rtt_scene {
                    self.r.draw_deck(rt, m, DeckProgram::Flat, &p, None);
                    tris += m.triangles() as u64;
                }
                self.r.end_pass();
                self.r.present(rt, out_w, out_h, true);
                tris
            }
        }
    }
}

impl App for Probe {
    fn frame(&mut self, f: &Frame) -> Flow {
        if self.step >= self.steps.len() {
            return self.finish();
        }
        let step = self.steps[self.step].clone();
        let t0 = Instant::now();
        let tris = self.draw(&step, f.width, f.height);
        let measuring = self.frame_in_step >= self.o.warmup;
        let last = self.frame_in_step + 1 == self.o.warmup + self.o.frames;
        if last && self.repeat == 0 {
            if let Some(dir) = &self.o.captures {
                let px = sc_render::read_output(f.width, f.height);
                let path = dir.join(format!("probe-scene{}-{}.png", step.scene(), step.label()));
                if let Err(e) = sc_client::capture::write_png(&path, f.width, f.height, &px) {
                    eprintln!("sc-probe: {}: {e}", path.display());
                    return Flow::Fail;
                }
            }
        }
        self.r.commit();
        let submit_ms = t0.elapsed().as_secs_f64() * 1e3;
        // Wait for the GPU, so the next frame's start marks this one's whole cost.
        sc_render::finish();
        // The frame time lands one frame late: elapsed_s is the time since the previous frame began.
        if self.repeat < self.o.repeats && self.frame_in_step > self.o.warmup {
            let s = self.samples[self.step].last_mut().expect("a sample per repeat");
            s.frame_ms.push(f.elapsed_s * 1e3);
        }
        if self.frame_in_step == 0 {
            self.samples[self.step].push(Sample::default());
        }
        if measuring {
            let counts = self.r.last_frame_counts();
            let s = self.samples[self.step].last_mut().expect("a sample per repeat");
            s.submit_ms.push(submit_ms);
            s.draws = counts.draws;
            s.triangles = tris;
        }
        self.frame_in_step += 1;
        if last {
            self.frame_in_step = 0;
            self.step += 1;
            if self.step == self.steps.len() {
                self.repeat += 1;
                if self.repeat < self.o.repeats {
                    self.step = 0;
                }
                println!("sc-probe: repeat {} done after {:.0} s", self.repeat, self.started.elapsed().as_secs_f64());
            }
        }
        Flow::Continue
    }

    fn event(&mut self, e: Event) -> Flow {
        match e {
            Event::Quit => Flow::Done,
            Event::KeyDown(platform::keys::ESCAPE) => Flow::Done,
            _ => Flow::Continue,
        }
    }
}

impl Probe {
    fn finish(&mut self) -> Flow {
        let rep = report::Report::build(&self.info, &self.steps, &self.samples);
        match &self.o.out {
            Some(dir) => match rep.write(dir) {
                Ok(()) => println!("sc-probe: report in {}", dir.display()),
                Err(e) => {
                    eprintln!("sc-probe: {}: {e}", dir.display());
                    return Flow::Fail;
                }
            },
            None => print!("{}", rep.markdown()),
        }
        Flow::Done
    }
}

fn main() -> ExitCode {
    let o = match options() {
        Ok(o) => o,
        Err(e) => {
            eprintln!("sc-probe: {e}");
            return ExitCode::from(2);
        }
    };
    let cfg = WindowConfig {
        title: "Star Crew probe".into(),
        width: o.width,
        height: o.height,
        fullscreen: !o.headless,
        headless: o.headless,
        vsync: false,
    };
    platform::run(cfg, move |gl| Probe::new(o, gl).map(|p| Box::new(p) as Box<dyn App>))
}
