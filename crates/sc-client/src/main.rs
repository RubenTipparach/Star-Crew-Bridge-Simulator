//! `sc-client`: the game client (engine-stack design sections 3 and 6).
//!
//! Today it loads the ship's compiled decks (`compiled/tern.deck`, from `sc-tools deckc`) and lets you fly
//! through every compartment, lit by the bake in the three lighting states, blended by the state weights as
//! the decks will be (keys 1, 2 and 3: normal, red alert, emergency power). Click to look around with the
//! mouse; W A S D move, Space and C rise and sink, Shift is faster; Tab lets the mouse go; F12 writes a
//! capture; Escape quits. There is no walking, collision, portal culling or simulation yet: it is a fly-through
//! of the level. Without a compiled deck it shows first light's test room and says how to build the deck.
//!
//! Usage: sc-client [--window] [--deck compiled/tern.deck] [--headless --shots DIR]

mod first_light;

use glam::{Mat4, Vec3};
use sc_client::platform::{self, keys, App, Event, Flow, Frame, WindowConfig};
use sc_render::{DeckParams, DeckProgram, Indices, Mesh, Renderer, Target, TextureArray};
use std::collections::HashSet;
use std::path::PathBuf;
use std::process::ExitCode;

/// The 3D pass's size (engine-stack design section 5).
const RENDER_3D: (u32, u32) = (1280, 720);
/// Seconds a change of lighting state takes to blend.
const STATE_BLEND_S: f32 = 0.5;
/// The eye above the floor, metres (crew-on-deck's body).
const EYE_M: f64 = 1.65;
/// Flying speed, metres a second, and with Shift.
const SPEED_M_S: (f64, f64) = (3.0, 9.0);
/// Radians of turn a pixel of mouse motion.
const LOOK_RAD_PX: f32 = 0.0025;

/// Named views for headless shots and the start: floor position (metres), facing in the plan, pitch (degrees).
/// The same viewpoints as the deck plan's walk shots (docs/mockups/deck-plan.html), so the two compare.
const POSES: &[(&str, [f64; 3], [f32; 2], f32)] = &[
    ("bridge-forward", [0.0, 3.5, 21.2], [0.0, 1.0], -10.0),
    ("bridge-captain", [1.2, 3.95, 27.0], [-0.25, -1.0], -14.0),
    ("bridge-helm-chairs", [3.8, 3.95, 25.0], [-0.75, 1.0], -22.0),
    ("corridor-B", [0.0, 0.0, 15.0], [0.0, -1.0], -4.0),
    ("eng-mezzanine", [7.0, 0.0, -18.9], [-0.55, -1.0], 6.0),
    ("eng-lower", [0.0, -3.5, -18.9], [0.12, -1.0], 10.0),
];

struct Camera {
    pos: [f64; 3],
    yaw: f32,
    pitch: f32,
}

impl Camera {
    fn at(pose: &(&str, [f64; 3], [f32; 2], f32)) -> Self {
        let (_, p, f, pitch) = pose;
        Self { pos: [p[0], p[1] + EYE_M, p[2]], yaw: f[0].atan2(f[1]), pitch: pitch.to_radians() }
    }
    fn dir(&self) -> Vec3 {
        Vec3::new(self.yaw.sin() * self.pitch.cos(), self.pitch.sin(), self.yaw.cos() * self.pitch.cos())
    }
}

enum Scene {
    Ship { rooms: Vec<(Mesh, [f64; 3])>, tex: TextureArray, panel_first: u32, panel_glow: [f32; 3], triangles: usize },
    TestRoom { room: Mesh, tex: TextureArray },
}

struct Client {
    target: Target,
    scene: Scene,
    cam: Camera,
    weights: [f32; 3],
    want: usize,
    held: HashSet<u32>,
    captured: bool,
    time_s: f64,
    shots: Option<PathBuf>,
    shot_plan: Vec<(usize, usize)>,
    shot_frame: u64,
    capture_next: bool,
    r: Renderer,
}

fn load_ship(r: &Renderer, path: &std::path::Path) -> Result<Scene, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let d = sc_core::deck::read(&bytes).map_err(|e| format!("{}: {e}", path.display()))?;
    let t = &d.index.textures;
    let levels: Vec<&[u8]> = (0..t.mips as usize)
        .map(|k| {
            let a = t.mip_offsets[k] as usize;
            let b = t.mip_offsets.get(k + 1).map_or(d.textures.len(), |o| *o as usize);
            &d.textures[a..b]
        })
        .collect();
    let tex = r.make_texture_array_mips(t.size_px, t.layers, &levels);
    let mut rooms = Vec::new();
    let mut triangles = 0;
    for c in &d.index.compartments {
        let idx = d.indices_of(c);
        triangles += idx.len() / 3;
        rooms.push((r.make_mesh(d.vertices_of(c), Indices::U32(&idx)), c.origin_m));
    }
    println!(
        "sc-client: {} with {} compartments, {} triangles, {} texture layers",
        path.display(),
        rooms.len(),
        triangles,
        t.layers
    );
    Ok(Scene::Ship { rooms, tex, panel_first: t.panel_first, panel_glow: t.panel_glow, triangles })
}

impl Client {
    fn new(deck: PathBuf, shots: Option<PathBuf>) -> Result<Self, String> {
        let text =
            std::fs::read_to_string("data/engine/render.json").map_err(|e| format!("data/engine/render.json: {e}"))?;
        let cfg = sc_core::data::parse("data/engine/render.json", &text).map_err(|e| e.to_string())?;
        let mut r = Renderer::new(&cfg);
        let target = r.make_target(RENDER_3D.0, RENDER_3D.1, 4);
        let scene = match load_ship(&r, &deck) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("sc-client: no ship ({e}); showing first light's test room.");
                eprintln!("sc-client: build the deck with: node tools/deck/export_deck.mjs && cargo run --release -p sc-tools -- deckc");
                let (v, i) = first_light::room();
                let (size, layers) = first_light::layers();
                Scene::TestRoom { room: r.make_mesh(&v, Indices::U16(&i)), tex: r.make_texture_array(size, &layers) }
            }
        };
        let cam = match scene {
            Scene::Ship { .. } => Camera::at(&POSES[0]),
            Scene::TestRoom { .. } => Camera { pos: [0.0, 1.65, -2.6], yaw: 0.0, pitch: -0.1 },
        };
        // Headless shots: every pose (the ship) or one view (the test room), in each state.
        let poses = if matches!(scene, Scene::Ship { .. }) { POSES.len() } else { 1 };
        let shot_plan =
            if shots.is_some() { (0..poses).flat_map(|p| (0..3).map(move |s| (p, s))).collect() } else { Vec::new() };
        Ok(Self {
            target,
            scene,
            cam,
            weights: [1.0, 0.0, 0.0],
            want: 0,
            held: HashSet::new(),
            captured: false,
            time_s: 0.0,
            shots,
            shot_plan,
            shot_frame: 0,
            capture_next: false,
            r,
        })
    }

    fn fly(&mut self, dt: f64) {
        let fwd = [f64::from(self.cam.yaw.sin()), 0.0, f64::from(self.cam.yaw.cos())];
        let right = [-fwd[2], 0.0, fwd[0]];
        let k = |c: u32| if self.held.contains(&c) { 1.0 } else { 0.0 };
        let speed = if self.held.contains(&keys::LSHIFT) { SPEED_M_S.1 } else { SPEED_M_S.0 };
        let (f, s, u) =
            (k(keys::W) - k(keys::S), k(keys::D) - k(keys::A), k(keys::SPACE) + k(keys::E) - k(keys::C) - k(keys::Q));
        for i in 0..3 {
            self.cam.pos[i] += (fwd[i] * f + right[i] * s) * speed * dt;
        }
        self.cam.pos[1] += u * speed * dt;
    }
}

impl App for Client {
    fn frame(&mut self, f: &Frame) -> Flow {
        let headless = self.shots.is_some();
        // Headless runs step a fixed 1/30 s a frame, so their shots do not depend on the machine.
        let dt = if headless { 1.0 / 30.0 } else { f.elapsed_s.min(0.1) };
        self.time_s += dt;
        let mut shot_name = None;
        if headless {
            // Each shot: set the pose and state, settle 40 frames (the blend), then capture.
            let Some(&(p, s)) = self.shot_plan.first() else { return Flow::Done };
            self.want = s;
            if matches!(self.scene, Scene::Ship { .. }) {
                self.cam = Camera::at(&POSES[p]);
            }
            self.shot_frame += 1;
            if self.shot_frame == 40 {
                let pose = if matches!(self.scene, Scene::Ship { .. }) { POSES[p].0 } else { "first-light" };
                shot_name = Some(format!("{pose}-{}.png", ["normal", "red-alert", "emergency"][s]));
                self.shot_plan.remove(0);
                self.shot_frame = 0;
            }
        } else {
            self.fly(dt);
        }
        let step = (dt as f32 / STATE_BLEND_S).min(1.0);
        for (k, w) in self.weights.iter_mut().enumerate() {
            let goal = if k == self.want { 1.0 } else { 0.0 };
            *w += (goal - *w) * step;
            if (goal - *w).abs() < 0.002 {
                *w = goal;
            }
        }
        let view = glam::camera::rh::view::look_to_mat4(Vec3::ZERO, self.cam.dir(), Vec3::Y);
        let proj = glam::camera::rh::proj::opengl::perspective(
            70f32.to_radians(),
            RENDER_3D.0 as f32 / RENDER_3D.1 as f32,
            0.05,
            300.0,
        );
        let w = self.weights;
        self.r.begin_3d(&self.target, [0.004, 0.005, 0.012, 1.0]);
        match &self.scene {
            Scene::Ship { rooms, tex, panel_first, panel_glow, .. } => {
                let glow = w[0] * panel_glow[0] + w[1] * panel_glow[1] + w[2] * panel_glow[2];
                for (mesh, origin) in rooms {
                    // The frames rule: subtract the camera in f64, then narrow to f32.
                    let rel = Vec3::new(
                        (origin[0] - self.cam.pos[0]) as f32,
                        (origin[1] - self.cam.pos[1]) as f32,
                        (origin[2] - self.cam.pos[2]) as f32,
                    );
                    let p = DeckParams {
                        mvp: proj * view * Mat4::from_translation(rel),
                        state_weights: w,
                        flash_dir: Vec3::Z,
                        flash: 0.0,
                        panel_first: *panel_first,
                        panel_glow: glow,
                    };
                    self.r.draw_deck(&self.target, mesh, DeckProgram::Textured, &p, Some(tex));
                }
            }
            Scene::TestRoom { room, tex } => {
                let rel = Vec3::new(-self.cam.pos[0] as f32, -self.cam.pos[1] as f32, -self.cam.pos[2] as f32);
                let p = DeckParams {
                    mvp: proj * view * Mat4::from_translation(rel),
                    state_weights: w,
                    flash_dir: Vec3::Z,
                    flash: 0.0,
                    panel_first: u32::MAX,
                    panel_glow: 0.0,
                };
                self.r.draw_deck(&self.target, room, DeckProgram::Textured, &p, Some(tex));
            }
        }
        self.r.end_pass();
        self.r.present(&self.target, f.width, f.height, true);
        if self.capture_next {
            shot_name = Some(format!("capture-{}.png", f.index));
            self.capture_next = false;
        }
        if let Some(n) = shot_name {
            let dir = self.shots.clone().unwrap_or_else(|| PathBuf::from("captures"));
            let px = sc_render::read_output(f.width, f.height);
            if let Err(e) = sc_client::capture::write_png(&dir.join(&n), f.width, f.height, &px) {
                eprintln!("sc-client: {e}");
                return Flow::Fail;
            }
            println!("sc-client: wrote {}", dir.join(n).display());
        }
        self.r.commit();
        if f.index == 1 {
            if let Scene::Ship { triangles, .. } = &self.scene {
                println!(
                    "sc-client: drawing {triangles} triangles a frame, {} draws (no culling yet)",
                    self.r.last_frame_counts().draws
                );
            }
        }
        Flow::Continue
    }

    fn event(&mut self, e: Event) -> Flow {
        match e {
            Event::Quit => return Flow::Done,
            Event::KeyDown(keys::ESCAPE) => {
                if !self.captured {
                    return Flow::Done;
                }
                self.captured = false;
                platform::capture_mouse(false);
            }
            Event::KeyDown(keys::TAB) => {
                self.captured = false;
                platform::capture_mouse(false);
            }
            Event::MouseDown => {
                self.captured = true;
                platform::capture_mouse(true);
            }
            Event::MouseMotion(dx, dy) if self.captured => {
                self.cam.yaw -= dx * LOOK_RAD_PX;
                self.cam.pitch = (self.cam.pitch - dy * LOOK_RAD_PX).clamp(-1.5, 1.5);
            }
            Event::KeyDown(keys::N1) => self.want = 0,
            Event::KeyDown(keys::N2) => self.want = 1,
            Event::KeyDown(keys::N3) => self.want = 2,
            Event::KeyDown(keys::F12) => self.capture_next = true,
            Event::KeyDown(k) => {
                self.held.insert(k);
            }
            Event::KeyUp(k) => {
                self.held.remove(&k);
            }
            Event::MouseMotion(..) => {}
        }
        Flow::Continue
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let value = |name: &str| args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).map(PathBuf::from);
    let headless = args.iter().any(|a| a == "--headless");
    let shots = value("--shots");
    let deck = value("--deck").unwrap_or_else(|| PathBuf::from("compiled/tern.deck"));
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
        Client::new(deck, shots).map(|c| Box::new(c) as Box<dyn App>)
    })
}
