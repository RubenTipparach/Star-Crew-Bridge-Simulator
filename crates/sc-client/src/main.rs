//! `sc-client`: the game client (engine-stack design sections 3 and 6).
//!
//! Today it is first light: the window, the GL context and the renderer, drawing a placeholder room
//! through the deck pipeline in the three lighting states, blended by the compartment's state
//! weights as the decks will be (keys 1, 2 and 3: normal, red alert, emergency power; F12 writes a
//! capture; Escape quits). The room is a test shape made here, not a deck: decks come from `deckc`
//! (openspec/changes/deck-pipeline), which is not built yet.
//!
//! Usage: sc-client [--window] [--headless --shots DIR]

mod first_light;

use glam::Vec3;
use sc_client::platform::{self, keys, App, Event, Flow, Frame, WindowConfig};
use sc_render::{DeckParams, DeckProgram, Indices, Mesh, Renderer, Target, TextureArray};
use std::path::PathBuf;
use std::process::ExitCode;

/// The 3D pass's size (engine-stack design section 5).
const RENDER_3D: (u32, u32) = (1280, 720);
/// Seconds a change of lighting state takes to blend.
const STATE_BLEND_S: f32 = 0.5;

struct Client {
    target: Target,
    room: Mesh,
    tex: TextureArray,
    weights: [f32; 3],
    want: usize,
    time_s: f64,
    shots: Option<PathBuf>,
    shot_plan: Vec<(u64, usize, &'static str)>,
    capture_next: bool,
    r: Renderer,
}

impl Client {
    fn new(shots: Option<PathBuf>) -> Result<Self, String> {
        let root = PathBuf::from(".");
        let text = std::fs::read_to_string(root.join("data/engine/render.json"))
            .map_err(|e| format!("data/engine/render.json: {e}"))?;
        let cfg = sc_core::data::parse("data/engine/render.json", &text).map_err(|e| e.to_string())?;
        let mut r = Renderer::new(&cfg);
        let target = r.make_target(RENDER_3D.0, RENDER_3D.1, 4);
        let (v, i) = first_light::room();
        let room = r.make_mesh(&v, Indices::U16(&i));
        let (size, layers) = first_light::layers();
        let tex = r.make_texture_array(size, &layers);
        // Headless shots: each state once it has settled, then done.
        let shot_plan = if shots.is_some() {
            vec![(40, 0, "normal"), (90, 1, "red-alert"), (140, 2, "emergency")]
        } else {
            Vec::new()
        };
        Ok(Self {
            target,
            room,
            tex,
            weights: [1.0, 0.0, 0.0],
            want: 0,
            time_s: 0.0,
            shots,
            shot_plan,
            capture_next: false,
            r,
        })
    }
}

impl App for Client {
    fn frame(&mut self, f: &Frame) -> Flow {
        // Headless runs step a fixed 1/30 s a frame, so their shots do not depend on the machine.
        let dt = if self.shots.is_some() { 1.0 / 30.0 } else { f.elapsed_s.min(0.1) };
        self.time_s += dt;
        if let Some(&(at, state, _)) = self.shot_plan.first() {
            if f.index + 30 == at {
                self.want = state;
            }
        }
        let step = (dt as f32 / STATE_BLEND_S).min(1.0);
        for (k, w) in self.weights.iter_mut().enumerate() {
            let goal = if k == self.want { 1.0 } else { 0.0 };
            *w += (goal - *w) * step.max(0.0);
            if (goal - *w).abs() < 0.002 {
                *w = goal;
            }
        }
        let t = self.time_s as f32 * 0.15;
        let eye = Vec3::new(t.sin() * 2.2, 1.65, -2.6 + t.cos() * 0.6);
        let view = glam::camera::rh::view::look_at_mat4(eye, Vec3::new(0.0, 1.2, 2.0), Vec3::Y);
        let proj = glam::camera::rh::proj::opengl::perspective(
            70f32.to_radians(),
            RENDER_3D.0 as f32 / RENDER_3D.1 as f32,
            0.05,
            60.0,
        );
        let p = DeckParams { mvp: proj * view, state_weights: self.weights, flash_dir: Vec3::Z, flash: 0.0 };
        self.r.begin_3d(&self.target, [0.0, 0.0, 0.0, 1.0]);
        self.r.draw_deck(&self.target, &self.room, DeckProgram::Textured, &p, Some(&self.tex));
        self.r.end_pass();
        self.r.present(&self.target, f.width, f.height, true);
        let mut name = None;
        if let Some(&(at, _, n)) = self.shot_plan.first() {
            if f.index == at {
                name = Some(format!("first-light-{n}.png"));
                self.shot_plan.remove(0);
            }
        }
        if self.capture_next {
            name = Some(format!("capture-{}.png", f.index));
            self.capture_next = false;
        }
        if let Some(n) = name {
            let dir = self.shots.clone().unwrap_or_else(|| PathBuf::from("captures"));
            let px = sc_render::read_output(f.width, f.height);
            if let Err(e) = sc_client::capture::write_png(&dir.join(&n), f.width, f.height, &px) {
                eprintln!("sc-client: {e}");
                return Flow::Fail;
            }
            println!("sc-client: wrote {}", dir.join(n).display());
        }
        self.r.commit();
        if self.shots.is_some() && self.shot_plan.is_empty() {
            return Flow::Done;
        }
        Flow::Continue
    }

    fn event(&mut self, e: Event) -> Flow {
        match e {
            Event::Quit | Event::KeyDown(keys::ESCAPE) => return Flow::Done,
            Event::KeyDown(keys::N1) => self.want = 0,
            Event::KeyDown(keys::N2) => self.want = 1,
            Event::KeyDown(keys::N3) => self.want = 2,
            Event::KeyDown(keys::F12) => self.capture_next = true,
            Event::KeyDown(_) => {}
        }
        Flow::Continue
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let headless = args.iter().any(|a| a == "--headless");
    let shots = args.iter().position(|a| a == "--shots").and_then(|i| args.get(i + 1)).map(PathBuf::from);
    if headless && shots.is_none() {
        eprintln!("sc-client: --headless needs --shots DIR (nothing would be seen)");
        return ExitCode::from(2);
    }
    let cfg = WindowConfig {
        title: "Star Crew".into(),
        width: 1920,
        height: 1080,
        fullscreen: !headless && !args.iter().any(|a| a == "--window"),
        headless,
        vsync: !headless,
    };
    platform::run(cfg, move |gl| {
        println!("sc-client: {} on {} ({})", gl.version, gl.renderer, gl.video_driver);
        Client::new(shots).map(|c| Box::new(c) as Box<dyn App>)
    })
}
