//! `sc-client`: the game client (engine-stack design sections 3 and 6).
//!
//! Today it loads the ship's compiled decks (`compiled/tern.deck`, from `sc-tools deckc`), every compartment lit
//! by the bake in the three lighting states, blended by the state weights as the decks will be (keys 1, 2 and 3:
//! normal, red alert, emergency power), and puts you on your feet on the bridge (deck-pipeline 13a): a body
//! walking the deck plan's walk world with `sc-core::walk` and the numbers of `data/crew/walk.json`. Click to look
//! around with the mouse; W A S D walk, Shift runs, Space jumps, E climbs a ladder (up where a trunk goes both ways; Q down) or goes through a hatch; F flies
//! instead (Space and C rise and sink) and lands you where you are; Tab lets the mouse go; F12 writes a capture;
//! Escape quits. In the lift's car E sends it up a deck and Q down; at a lift door E calls it (deck-pipeline 13b,
//! `sc-core::lift`). Doors stand open (13a); there is no portal culling yet. Without a compiled deck it shows first light's test room and says how to build the deck.
//!
//! Usage: sc-client [--window] [--deck compiled/tern.deck] [--headless --shots DIR] [--headless --walk-test DIR]
//!
//! `--walk-test DIR` walks a scripted route from the bridge down both ladders to deck C, captures along it, and
//! fails if the body does not arrive where the route ends.

mod first_light;

use glam::{Mat4, Vec3};
use sc_client::platform::{self, keys, App, Event, Flow, Frame, WindowConfig};
use sc_core::deck::{DeckView, WalkDoor};
use sc_core::exterior::{linear, normalized, ExteriorData};
use sc_core::lift::Lift;
use sc_core::walk::{self, Body, Input, WalkData, WalkEntities, WalkWorld};
use sc_render::{DeckParams, DeckProgram, Indices, Mesh, Renderer, SkyParams, Target, TextureArray};
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
    ("bridge-viewscreen", [0.0, 3.5, 24.5], [0.0, 1.0], 9.0),
    ("bridge-port-window", [3.4, 3.5, 28.9], [0.68, 0.73], -14.0),
    ("corridor-B", [0.0, 0.0, 15.0], [0.0, -1.0], -4.0),
    ("eng-mezzanine", [7.0, 0.0, -18.9], [-0.55, -1.0], 6.0),
    ("eng-lower", [0.0, -3.5, -18.9], [0.12, -1.0], 10.0),
];

/// The scripted walk of `--walk-test`: from the bridge's start aft through its door, down the command passage to
/// the ladder trunk, down to deck B and on down to deck C, with a shot at each stage. Plan positions `[x, z]`.
enum Leg {
    /// Walk to a point.
    To([f32; 2]),
    /// Use (climb the ladder in reach, down if `true`) and wait until off it.
    Use(bool),
    /// Face `[x, z]` at a pitch (degrees), settle, and capture.
    Shot(&'static str, [f32; 2], f32),
    /// Press the lift's E (true) or Q, wait until the car stands open again, and check the feet rode it.
    Lift(bool),
}
/// The lift's landings are at x -1.25, z 18.05, facing +x; its car is x -3.3 to -1.25 (deck-access).
const ROUTE: &[Leg] = &[
    Leg::Shot("walk-1-bridge", [0.0, 1.0], -8.0),
    Leg::To([0.0, 20.6]),
    Leg::To([0.0, 18.05]),
    Leg::Shot("lift-1-landing-deck-A", [-1.0, 0.0], -6.0),
    Leg::To([-2.3, 18.05]),
    Leg::Shot("lift-2-in-the-car-at-A", [1.0, 0.0], -6.0),
    Leg::Lift(false),
    Leg::Shot("lift-3-car-at-B", [1.0, 0.0], -6.0),
    Leg::Lift(false),
    Leg::Shot("lift-4-car-at-C", [1.0, 0.0], -6.0),
    Leg::Lift(true),
    Leg::Lift(true),
    Leg::Shot("lift-5-back-at-A", [1.0, 0.0], -6.0),
    Leg::To([0.0, 18.05]),
    Leg::To([0.0, 15.0]),
    Leg::Shot("walk-2-command-passage", [0.0, -1.0], -6.0),
    Leg::To([0.0, 11.3]),
    Leg::Use(true),
    Leg::Shot("walk-3-deck-B-at-the-ladder", [0.0, 1.0], -4.0),
    Leg::Use(true),
    Leg::Shot("walk-4-deck-C-at-the-ladder", [0.0, -1.0], -4.0),
    // The ladder's rails and rungs stand just aft of where the climb ends: step round them.
    Leg::To([0.85, 11.3]),
    Leg::To([0.85, 9.5]),
    Leg::To([0.0, 6.0]),
    Leg::Shot("walk-5-deck-C-corridor", [0.0, -1.0], -6.0),
];
/// Where the route must end, ship coordinates (deck C's spine aft of the trunk), and how near.
const ROUTE_END: ([f32; 3], f32) = ([0.0, -3.5, 6.0], 0.3);

/// A body on the decks and the eye that follows it.
struct Walker {
    world: WalkWorld,
    data: WalkData,
    body: Body,
    eye_y: f32,
    /// Each lift's car, and the floor height its room was drawn at in the deck.
    lifts: Vec<(Lift, f32)>,
    /// The doors, for the lift landings a call reaches.
    doors: Vec<WalkDoor>,
}

impl Walker {
    /// E and Q for the lifts: in a car, up and down a deck; at a landing, E calls the car. True when a key was taken.
    fn lift_keys(&mut self, up: bool, down: bool) -> bool {
        if !up && !down {
            return false;
        }
        let f = self.body.feet;
        if let Some(i) = self.world.lift_index_at(f[0], f[2]) {
            let l = &mut self.lifts[i].0;
            if up {
                l.up();
            } else {
                l.down();
            }
            return true;
        }
        let reach = self.data.lift_call_m as f32;
        if let (true, Some((i, stop))) = (up, self.world.landing_near(f[0], f[1], f[2], reach, &self.doors)) {
            let l = &mut self.lifts[i].0;
            let (s, _) = l.nearest_stop(stop);
            l.send(s);
            return true;
        }
        false
    }

    /// Run the cars `dt` seconds and tell the walk world where they are.
    fn step_lifts(&mut self, dt: f32) {
        for (i, (l, _)) in self.lifts.iter_mut().enumerate() {
            l.step(dt);
            let open = (l.doors_open() > 0.9).then(|| l.stops()[l.stop()]);
            self.world.set_lift(i, l.car_y, open);
        }
    }
}

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

/// A compartment as the client draws it.
struct Room {
    mesh: Mesh,
    /// Its frame's origin, ship coordinates, metres.
    origin: [f64; 3],
    /// The lift whose car it is (a mover), if any.
    car: Option<usize>,
    /// Outside the hull (the space dock): drawn in the bow camera's view too.
    outside: bool,
}

/// The outside (deck-pipeline 13b): the sky's numbers, the bow camera's target and where the viewscreens are.
struct Outside {
    data: ExteriorData,
    bow: Target,
    views: Vec<DeckView>,
}

impl Outside {
    /// The sky's parameters seen through `proj * view` (a view with no translation), `px_rad` a pixel's angle.
    fn sky(&self, proj: Mat4, view: Mat4, px_rad: f32) -> SkyParams {
        let e = &self.data;
        let sun = normalized(e.sun.dir);
        let pl = normalized(e.planet.dir);
        let v4 = |c: [f32; 3], w: f64| [c[0], c[1], c[2], w as f32];
        SkyParams {
            inv_vp: (proj * view).inverse(),
            sun: v4(sun, (e.sun.disc_deg / 2.0).to_radians().cos()),
            sun_colour: v4(linear(e.sun.colour_srgb), e.sun.glow),
            planet: v4(pl, e.planet.radius_deg.to_radians().sin()),
            ocean: v4(linear(e.planet.ocean_srgb), e.planet.land_frac),
            land: v4(linear(e.planet.land_srgb), e.planet.cloud_frac),
            cloud: v4(linear(e.planet.cloud_srgb), e.planet.night),
            atmosphere: v4(linear(e.planet.atmosphere_srgb), e.planet.atmosphere_frac),
            stars: [e.stars.density as f32, e.stars.brightness as f32, px_rad, 0.0],
        }
    }
}

enum Scene {
    Ship {
        rooms: Vec<Room>,
        outside: Option<Box<Outside>>,
        tex: TextureArray,
        panel_first: u32,
        panel_glow: [f32; 3],
        triangles: usize,
        start: sc_core::deck::WalkStart,
    },
    TestRoom {
        room: Mesh,
        tex: TextureArray,
    },
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
    walker: Option<Walker>,
    flying: bool,
    /// One-shot keys this frame (jump, use), taken by the walk's next step.
    pressed: HashSet<u32>,
    /// `--walk-test`: the leg on now, its seconds so far, and the frames a shot has settled.
    walk_test: Option<(usize, f64, u32)>,
    /// The walk test: the feet's height when a Use began.
    use_from: f32,
}

fn load_ship(r: &mut Renderer, path: &std::path::Path) -> Result<(Scene, Option<Walker>), String> {
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
        let car = if c.mover == 0 {
            None
        } else {
            d.index.walk.lifts.iter().position(|l| l.car_room.as_deref() == Some(c.id.as_str()))
        };
        rooms.push(Room {
            mesh: r.make_mesh(d.vertices_of(c), Indices::U32(&idx)),
            origin: c.origin_m,
            car,
            outside: c.deck == "outside",
        });
    }
    println!(
        "sc-client: {} with {} compartments, {} triangles, {} texture layers",
        path.display(),
        rooms.len(),
        triangles,
        t.layers
    );
    // The walk world (deck-pipeline 13a) and the walk's numbers.
    let text = std::fs::read_to_string("data/crew/walk.json").map_err(|e| format!("data/crew/walk.json: {e}"))?;
    let data: WalkData = sc_core::data::parse("data/crew/walk.json", &text).map_err(|e| e.to_string())?;
    let w = &d.index.walk;
    let ents = WalkEntities {
        ladders: w.ladders.clone(),
        hatches: w.hatches.clone(),
        doors: w.doors.clone(),
        lifts: w.lifts.clone(),
    };
    let t0 = std::time::Instant::now();
    let world = WalkWorld::new(&d.walk_triangles(), &ents, &data)?;
    println!(
        "sc-client: walk world of {} triangles built in {:.0} ms; {} ladders, {} hatches, {} doors",
        world.triangles,
        t0.elapsed().as_secs_f64() * 1000.0,
        w.ladders.len(),
        w.hatches.len(),
        w.doors.len()
    );
    let body = Body::at_start(&w.start, &data);
    let eye_y = w.start.at_m[1];
    let lifts = w.lifts.iter().map(|l| (Lift::new(&l.stops_m, l.speed_m_s, l.door_s, l.car_m), l.car_m)).collect();
    let walker = Walker { world, data, body, eye_y, lifts, doors: w.doors.clone() };
    // The outside: its data, and the bow camera's target (no MSAA: it is shown a fifth of the screen's size).
    let text =
        std::fs::read_to_string("data/space/exterior.json").map_err(|e| format!("data/space/exterior.json: {e}"))?;
    let ext: ExteriorData = sc_core::data::parse("data/space/exterior.json", &text).map_err(|e| e.to_string())?;
    let [bw, bh] = ext.viewscreen.target_px;
    let outside = Outside { bow: r.make_target(bw, bh, 1), views: d.index.views.clone(), data: ext };
    let scene = Scene::Ship {
        rooms,
        outside: Some(Box::new(outside)),
        tex,
        panel_first: t.panel_first,
        panel_glow: t.panel_glow,
        triangles,
        start: w.start.clone(),
    };
    Ok((scene, Some(walker)))
}

impl Client {
    fn new(deck: PathBuf, shots: Option<PathBuf>, walk_test: bool) -> Result<Self, String> {
        let text =
            std::fs::read_to_string("data/engine/render.json").map_err(|e| format!("data/engine/render.json: {e}"))?;
        let cfg = sc_core::data::parse("data/engine/render.json", &text).map_err(|e| e.to_string())?;
        let mut r = Renderer::new(&cfg);
        let target = r.make_target(RENDER_3D.0, RENDER_3D.1, 4);
        let (scene, walker) = match load_ship(&mut r, &deck) {
            Ok(s) => s,
            Err(e) if walk_test => return Err(format!("--walk-test needs the ship: {e}")),
            Err(e) => {
                eprintln!("sc-client: no ship ({e}); showing first light's test room.");
                eprintln!("sc-client: build the deck with: node tools/deck/export_deck.mjs && cargo run --release -p sc-tools -- deckc");
                let (v, i) = first_light::room();
                let (size, layers) = first_light::layers();
                (
                    Scene::TestRoom {
                        room: r.make_mesh(&v, Indices::U16(&i)),
                        tex: r.make_texture_array(size, &layers),
                    },
                    None,
                )
            }
        };
        let cam = match &scene {
            Scene::Ship { start, .. } => Camera {
                pos: [f64::from(start.at_m[0]), f64::from(start.at_m[1]) + EYE_M, f64::from(start.at_m[2])],
                yaw: start.face[0].atan2(start.face[1]),
                pitch: 0.0,
            },
            Scene::TestRoom { .. } => Camera { pos: [0.0, 1.65, -2.6], yaw: 0.0, pitch: -0.1 },
        };
        // Headless shots: every pose (the ship) or one view (the test room), in each state; a walk test instead walks.
        let poses = if matches!(scene, Scene::Ship { .. }) { POSES.len() } else { 1 };
        let shot_plan = if shots.is_some() && !walk_test {
            (0..poses).flat_map(|p| (0..3).map(move |s| (p, s))).collect()
        } else {
            Vec::new()
        };
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
            flying: walker.is_none(),
            walker,
            pressed: HashSet::new(),
            walk_test: walk_test.then_some((0, 0.0, 0)),
            use_from: 0.0,
        })
    }

    /// One step of the body from the keys (or the walk test's steering), and the camera on its eye.
    fn walk(&mut self, dt: f64, steer: Option<Input>) {
        let Some(w) = self.walker.as_mut() else { return };
        let k = |c: u32| if self.held.contains(&c) { 1.0 } else { 0.0 };
        let input = steer.unwrap_or(Input {
            forward: k(keys::W) - k(keys::S),
            right: k(keys::D) - k(keys::A),
            yaw: self.cam.yaw,
            run: self.held.contains(&keys::LSHIFT),
            jump: self.pressed.contains(&keys::SPACE),
            use_: self.pressed.contains(&keys::E) || self.pressed.contains(&keys::Q),
            down: self.pressed.contains(&keys::Q),
        });
        // E and Q work the lift first (in its car, or at a landing); otherwise they climb.
        let lift = steer.is_none() && w.lift_keys(self.pressed.contains(&keys::E), self.pressed.contains(&keys::Q));
        let input = if lift { Input { use_: false, down: false, ..input } } else { input };
        self.pressed.clear();
        w.step_lifts(dt as f32);
        w.body.step(&w.world, &w.data, &input, dt);
        w.eye_y = walk::eye_follow(w.eye_y, w.body.feet[1], dt as f32, &w.data, w.body.climbing());
        let f = w.body.feet;
        self.cam.pos = [f64::from(f[0]), f64::from(w.eye_y) + w.data.body.eye_m, f64::from(f[2])];
    }

    /// The walk test's next move: steering toward a leg's point, a use, or a shot. Returns the input, the shot
    /// file name when one is due, and whether the route is done (Err: it failed).
    fn walk_test_step(&mut self, dt: f64) -> Result<(Option<Input>, Option<String>, bool), String> {
        let Some((leg, t, settle)) = self.walk_test else { return Ok((None, None, false)) };
        let Some(w) = self.walker.as_ref() else { return Err("no walker".into()) };
        let feet = w.body.feet;
        let next = |s: &mut Self| s.walk_test = Some((leg + 1, 0.0, 0));
        let Some(l) = ROUTE.get(leg) else {
            let (end, tol) = ROUTE_END;
            let off = ((feet[0] - end[0]).powi(2) + (feet[1] - end[1]).powi(2) + (feet[2] - end[2]).powi(2)).sqrt();
            return if off <= tol {
                println!("sc-client: walk test: arrived at {feet:?}, {off:.2} m from the route's end");
                Ok((None, None, true))
            } else {
                Err(format!("walk test: the body ended at {feet:?}, {off:.2} m from {end:?}"))
            };
        };
        self.walk_test = Some((leg, t + dt, settle));
        match l {
            Leg::To(p) => {
                let (dx, dz) = (p[0] - feet[0], p[1] - feet[2]);
                if dx.hypot(dz) < 0.12 {
                    next(self);
                    return Ok((Some(Input { yaw: self.cam.yaw, ..Input::default() }), None, false));
                }
                if t > 20.0 {
                    return Err(format!("walk test: leg {leg} did not reach {p:?} in 20 s; the body is at {feet:?}"));
                }
                self.cam.yaw = dx.atan2(dz);
                Ok((Some(Input { forward: 1.0, yaw: self.cam.yaw, ..Input::default() }), None, false))
            }
            Leg::Use(down) => {
                if t == 0.0 {
                    let y0 = feet[1];
                    self.use_from = y0;
                    return Ok((
                        Some(Input { use_: true, down: *down, yaw: self.cam.yaw, ..Input::default() }),
                        None,
                        false,
                    ));
                }
                if !w.body.climbing() && t > 0.1 {
                    if !w.body.grounded {
                        return Ok((Some(Input { yaw: self.cam.yaw, ..Input::default() }), None, false));
                    }
                    if (feet[1] - self.use_from).abs() < 1.0 {
                        return Err(format!("walk test: leg {leg}: Use climbed nothing; the body is at {feet:?}"));
                    }
                    next(self);
                }
                if t > 10.0 {
                    return Err(format!("walk test: leg {leg}: Use climbed nothing; the body is at {feet:?}"));
                }
                Ok((Some(Input { yaw: self.cam.yaw, ..Input::default() }), None, false))
            }
            Leg::Lift(up) => {
                let w = self.walker.as_mut().ok_or("no walker")?;
                if t == 0.0 {
                    self.use_from = feet[1];
                    if !w.lift_keys(*up, !*up) {
                        return Err(format!("walk test: leg {leg}: the lift key did nothing at {feet:?}"));
                    }
                    return Ok((Some(Input { yaw: self.cam.yaw, ..Input::default() }), None, false));
                }
                let (l, _) = &w.lifts[0];
                if l.phase == sc_core::lift::Phase::Idle && t > 0.5 {
                    if (feet[1] - l.car_y).abs() > 0.02 || (feet[1] - self.use_from).abs() < 3.0 {
                        return Err(format!(
                            "walk test: leg {leg}: the car is at {} m and the feet at {feet:?}, from {}",
                            l.car_y, self.use_from
                        ));
                    }
                    next(self);
                } else if t > 20.0 {
                    return Err(format!("walk test: leg {leg}: the car did not arrive in 20 s"));
                }
                Ok((Some(Input { yaw: self.cam.yaw, ..Input::default() }), None, false))
            }
            Leg::Shot(name, face, pitch) => {
                self.cam.yaw = face[0].atan2(face[1]);
                self.cam.pitch = pitch.to_radians();
                let shot = (settle == 30).then(|| format!("{name}.png"));
                self.walk_test = Some((leg, t + dt, settle + 1));
                if shot.is_some() {
                    next(self);
                }
                Ok((Some(Input { yaw: self.cam.yaw, ..Input::default() }), shot, false))
            }
        }
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
        if self.walk_test.is_some() {
            match self.walk_test_step(dt) {
                Ok((input, shot, done)) => {
                    if done {
                        return Flow::Done;
                    }
                    self.walk(dt, input);
                    shot_name = shot;
                }
                Err(e) => {
                    eprintln!("sc-client: {e}");
                    return Flow::Fail;
                }
            }
        } else if headless {
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
        } else if self.flying {
            self.fly(dt);
        } else {
            self.walk(dt, None);
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
        // The bow camera's view first, in its own pass: the sky and the dock, for the viewscreens (13b).
        if let Scene::Ship { rooms, outside: Some(o), tex, panel_first, panel_glow, .. } = &self.scene {
            let v = &o.data.viewscreen;
            let (tw, th) = o.bow.size();
            let bproj = glam::camera::rh::proj::opengl::perspective(
                (v.fov_deg as f32).to_radians(),
                tw as f32 / th as f32,
                0.5,
                400.0,
            );
            let look = normalized(v.look);
            let bview = glam::camera::rh::view::look_to_mat4(Vec3::ZERO, Vec3::from(look), Vec3::Y);
            self.r.begin_3d(&o.bow, [0.0, 0.0, 0.0, 1.0]);
            self.r.draw_sky(&o.bow, &o.sky(bproj, bview, (v.fov_deg as f32).to_radians() / th as f32));
            let glow = w[0] * panel_glow[0] + w[1] * panel_glow[1] + w[2] * panel_glow[2];
            for room in rooms.iter().filter(|r| r.outside) {
                let rel = Vec3::new(
                    (room.origin[0] - v.camera_m[0]) as f32,
                    (room.origin[1] - v.camera_m[1]) as f32,
                    (room.origin[2] - v.camera_m[2]) as f32,
                );
                let p = DeckParams {
                    mvp: bproj * bview * Mat4::from_translation(rel),
                    state_weights: w,
                    flash_dir: Vec3::Z,
                    flash: 0.0,
                    panel_first: *panel_first,
                    panel_glow: glow,
                };
                self.r.draw_deck(&o.bow, &room.mesh, DeckProgram::Textured, &p, Some(tex));
            }
            self.r.end_pass();
        }
        self.r.begin_3d(&self.target, [0.004, 0.005, 0.012, 1.0]);
        match &self.scene {
            Scene::Ship { rooms, outside, tex, panel_first, panel_glow, .. } => {
                if let Some(o) = outside {
                    let px = 70f32.to_radians() / RENDER_3D.1 as f32;
                    self.r.draw_sky(&self.target, &o.sky(proj, view, px));
                }
                let glow = w[0] * panel_glow[0] + w[1] * panel_glow[1] + w[2] * panel_glow[2];
                for Room { mesh, origin, car, .. } in rooms {
                    // A lift's car is drawn where the car is: its room was exported at the car's floor in the deck.
                    let lift_dy = match (car, self.walker.as_ref()) {
                        (Some(i), Some(w)) => w.lifts.get(*i).map_or(0.0, |(l, base)| f64::from(l.car_y - base)),
                        _ => 0.0,
                    };
                    // The frames rule: subtract the camera in f64, then narrow to f32.
                    let rel = Vec3::new(
                        (origin[0] - self.cam.pos[0]) as f32,
                        (origin[1] + lift_dy - self.cam.pos[1]) as f32,
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
                // The viewscreens: the bow camera's picture on a quad 3 cm in front of each.
                if let Some(o) = outside {
                    for vw in &o.views {
                        let f = vw.facing_yaw_deg.to_radians();
                        let n = Vec3::new(f.sin(), 0.0, f.cos());
                        let right = Vec3::new(n.z, 0.0, -n.x);
                        let c = Vec3::new(
                            (f64::from(vw.center_m[0]) - self.cam.pos[0]) as f32,
                            (f64::from(vw.center_m[1]) - self.cam.pos[1]) as f32,
                            (f64::from(vw.center_m[2]) - self.cam.pos[2]) as f32,
                        ) + n * 0.03;
                        let model = Mat4::from_cols(
                            (right * vw.size_m[0] / 2.0).extend(0.0),
                            (Vec3::Y * vw.size_m[1] / 2.0).extend(0.0),
                            n.extend(0.0),
                            c.extend(1.0),
                        );
                        let look = [o.data.viewscreen.scanlines as f32, o.bow.size().1 as f32, 1.0, 0.0];
                        self.r.draw_screen(&self.target, proj * view * model, &o.bow, look);
                    }
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
            Event::KeyDown(keys::F) if self.walker.is_some() => {
                // Flying, then landing where the camera is: the body stands on the floor under the eye.
                self.flying = !self.flying;
                if let (false, Some(w)) = (self.flying, self.walker.as_mut()) {
                    let (x, z) = (self.cam.pos[0] as f32, self.cam.pos[2] as f32);
                    let y = w
                        .world
                        .floor_below(x, self.cam.pos[1] as f32, z)
                        .unwrap_or(self.cam.pos[1] as f32 - EYE_M as f32);
                    w.body = Body::new([x, y, z], &w.data);
                    w.eye_y = y;
                }
            }
            Event::KeyDown(k) => {
                if !self.held.contains(&k) {
                    self.pressed.insert(k);
                }
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
    let walk_dir = value("--walk-test");
    let walk_test = walk_dir.is_some();
    let shots = shots.or(walk_dir);
    if headless && shots.is_none() {
        eprintln!("sc-client: --headless needs --shots DIR (nothing would be seen)");
        return ExitCode::from(2);
    }
    let cfg = WindowConfig {
        title: "Star Crew".into(),
        // In the browser the window is the page's canvas, sized by the page (engine-stack design 10a).
        width: if cfg!(target_os = "emscripten") { 1280 } else { 1920 },
        height: if cfg!(target_os = "emscripten") { 720 } else { 1080 },
        fullscreen: !headless && !args.iter().any(|a| a == "--window") && !cfg!(target_os = "emscripten"),
        headless,
        vsync: !headless,
    };
    platform::run(cfg, move |gl| {
        println!("sc-client: {} on {} ({})", gl.version, gl.renderer, gl.video_driver);
        Client::new(deck, shots, walk_test).map(|c| Box::new(c) as Box<dyn App>)
    })
}
