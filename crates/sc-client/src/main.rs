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
//! It opens on the lobby (openspec/changes/lobby): name your officer, pick a station, Beam aboard; the ship loads
//! then. A HUD names the officer and what E and Q do at a lift.
//!
//! Usage: sc-client [--window] [--deck compiled/tern.deck] [--headless --shots DIR] [--headless --walk-test DIR]
//! [--headless --lobby-test DIR] [--console-fixture STATE.json --shots DIR]
//!
//! `--lobby-test DIR` shoots the lobby, rolls a name, shoots it again, beams aboard and shoots the bridge.
//!
//! `--walk-test DIR` walks a scripted route from the bridge down both ladders to deck C, captures along it, and
//! fails if the body does not arrive where the route ends.

mod console;
mod drill;
mod first_light;
mod fixture;
mod lobby;
mod seat;
mod ships3d;
mod ui;
mod vg;

use glam::{Mat4, Vec3};
use sc_client::platform::{self, keys, App, Event, Flow, Frame, WindowConfig};
use sc_core::bots::Crew;
use sc_core::crew::CompanyData;
use sc_core::deck::{DeckView, WalkDoor};
use sc_core::exterior::{linear, normalized, ExteriorData};
use sc_core::lift::Lift;
use sc_core::nav::NavGrid;
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
    /// The walk grid the bots path on (crew-npcs 7).
    grid: NavGrid,
    /// The bot crew.
    crew: Crew,
    /// Their departments.
    company: CompanyData,
    /// The decks' floor heights, lowest first (C, B, A on the Tern), metres.
    decks: Vec<f32>,
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

    /// What E and Q do where the body stands: in a lift's car, or at a landing.
    fn prompt(&self) -> Option<&'static str> {
        let f = self.body.feet;
        if self.world.lift_index_at(f[0], f[2]).is_some() {
            return Some("E  Up        Q  Down");
        }
        let reach = self.data.lift_call_m as f32;
        self.world.landing_near(f[0], f[1], f[2], reach, &self.doors).map(|_| "E  Call lift")
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
    /// Its lowest deck's letter (`A` the top), for the map.
    deck: char,
}

/// The ship map (ship-plan-view 5): an orbit round the ship, the decks pulled apart and cut.
struct MapView {
    yaw: f32,
    pitch: f32,
    dist: f32,
    /// One deck only (its index, 0 the lowest), or all.
    deck: Option<usize>,
}

/// The decks pulled apart in the map, metres.
const MAP_EXPLODE_M: f32 = 12.0;
/// Each deck is cut this high above its floor in the map, metres.
const MAP_CUT_M: f32 = 1.6;
/// Figures are drawn this much larger on the map, so a body reads from across the ship.
const MAP_FIGURE_SCALE: f32 = 3.0;
/// `--shots` frames this many bots close up after the ship's poses (light-baking 16).
const BOT_SHOTS: usize = 4;
/// A bot close-up's camera stands this far in front of the bot, metres.
const BOT_SHOT_M: f32 = 2.2;
/// Where a figure takes its light probes: its chest, this far above its feet, metres (light-baking 16).
const FIGURE_PROBE_HEIGHT_M: f64 = 1.0;

/// A name over a marker on the map: where (0-1 across and down the screen), what, its colour.
struct MapLabel {
    at: [f32; 2],
    text: String,
    colour: egui::Color32,
    own: bool,
}

/// The map's title and deck, and each marker's name in a dark pill (the player's ringed).
fn map_overlay(ctx: &egui::Context, deck: &str, labels: &[MapLabel]) {
    let screen = ctx.content_rect();
    egui::Area::new(egui::Id::new("map")).anchor(egui::Align2::CENTER_TOP, egui::Vec2::new(0.0, 14.0)).show(
        ctx,
        |ui| {
            ui.vertical_centered(|ui| {
                ui.label(
                    egui::RichText::new("SHIP MAP").size(24.0).strong().color(egui::Color32::from_rgb(242, 160, 70)),
                );
                ui.label(egui::RichText::new(deck).size(15.0).color(egui::Color32::from_rgb(190, 160, 230)));
            });
        },
    );
    let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Background, egui::Id::new("names")));
    for l in labels {
        let p = egui::pos2(screen.min.x + l.at[0] * screen.width(), screen.min.y + l.at[1] * screen.height());
        let size = if l.own { 17.0 } else { 13.0 };
        let galley = painter.layout_no_wrap(l.text.clone(), egui::FontId::proportional(size), l.colour);
        let r = egui::Rect::from_center_size(p, galley.size() + egui::vec2(10.0, 4.0));
        painter.rect_filled(r, 6.0, egui::Color32::from_rgba_premultiplied(6, 9, 14, 200));
        if l.own {
            painter.rect_stroke(r, 6.0, egui::Stroke::new(1.5_f32, l.colour), egui::StrokeKind::Outside);
        }
        painter.galley(r.min + egui::vec2(5.0, 2.0), galley, l.colour);
    }
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
        sky_params(&self.data, proj, view, px_rad)
    }
}

/// The sky's parameters from the exterior's data, seen through `proj * view`, `px_rad` a pixel's angle.
fn sky_params(e: &ExteriorData, proj: Mat4, view: Mat4, px_rad: f32) -> SkyParams {
    {
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
        /// A crew figure mesh for each department, by its index in the company.
        figures: Vec<(usize, Mesh)>,
        /// Every compartment's light probes, which light the figures (light-baking 16).
        probes: sc_core::probes::ProbeSet,
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
    /// Nothing loaded yet: the lobby.
    Empty,
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
    /// The UI layer.
    ui: ui::Ui,
    /// The lobby while it is up; `None` aboard.
    lobby: Option<lobby::Lobby>,
    /// Frames since Beam aboard was pressed (the first shows the beaming, the second loads).
    beaming: u32,
    /// The officer aboard: "Lt. Maren Holloway" and the station asked for.
    officer: Option<(String, String)>,
    /// The deck file, loaded on beaming aboard.
    deck: PathBuf,
    /// The outside's data (the lobby's sky; the ship's outside).
    exterior: ExteriorData,
    /// `--lobby-test`: frames the lobby has run.
    lobby_test: Option<u32>,
    /// The ship map while it is open (M).
    map: Option<MapView>,
    /// The names over the map's markers this frame.
    map_labels: Vec<MapLabel>,
}

/// Load the deck at `deck`, or first light's test room when there is none (refused for a test that needs the ship),
/// and the camera on the start.
fn load_scene(
    r: &mut Renderer,
    deck: &std::path::Path,
    ext: &ExteriorData,
    need_ship: bool,
    session_seed: u64,
) -> Result<(Scene, Option<Walker>, Camera), String> {
    let (scene, walker) = match load_ship(r, deck, ext, session_seed) {
        Ok(s) => s,
        Err(e) if need_ship => return Err(format!("this test needs the ship: {e}")),
        Err(e) => {
            eprintln!("sc-client: no ship ({e}); showing first light's test room.");
            eprintln!("sc-client: build the deck with: node tools/deck/export_deck.mjs && cargo run --release -p sc-tools -- deckc");
            let (v, i) = first_light::room();
            let (size, layers) = first_light::layers();
            (
                Scene::TestRoom { room: r.make_mesh(&v, Indices::U16(&i)), tex: r.make_texture_array(size, &layers) },
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
        _ => Camera { pos: [0.0, 1.65, -2.6], yaw: 0.0, pitch: -0.1 },
    };
    Ok((scene, walker, cam))
}

fn load_ship(
    r: &mut Renderer,
    path: &std::path::Path,
    ext: &ExteriorData,
    session_seed: u64,
) -> Result<(Scene, Option<Walker>), String> {
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
    let mut figures = Vec::new();
    let mut triangles = 0;
    for c in &d.index.compartments {
        let idx = d.indices_of(c);
        if c.deck == "figure" {
            figures.push((
                c.id.trim_start_matches("figure_").to_owned(),
                r.make_mesh(d.vertices_of(c), Indices::U32(&idx)),
            ));
            continue;
        }
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
            deck: c.deck.chars().next().unwrap_or('A'),
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
    // The bot crew (crew-npcs 7): the walk grid on the decks the lift stops at, the company from the session seed.
    let mut decks: Vec<f32> = w.lifts.iter().flat_map(|l| l.stops_m.iter().copied()).collect();
    if decks.is_empty() {
        decks = w.ladders.iter().flat_map(|l| [l.lo_m, l.hi_m]).collect();
    }
    decks.sort_by(f32::total_cmp);
    decks.dedup_by(|a, b| (*a - *b).abs() < 0.1);
    let tris = d.walk_triangles();
    let mut bounds = [f32::MAX, f32::MAX, f32::MIN, f32::MIN];
    for t in &tris {
        for k in 0..3 {
            bounds = [
                bounds[0].min(t[k * 3]),
                bounds[1].min(t[k * 3 + 2]),
                bounds[2].max(t[k * 3]),
                bounds[3].max(t[k * 3 + 2]),
            ];
        }
    }
    let t0 = std::time::Instant::now();
    let shafts: Vec<Vec<[f32; 2]>> = w.lifts.iter().map(|l| l.poly.clone()).collect();
    // `decks` stays with the walker: the map stacks them.
    let grid = NavGrid::build(&world, &decks, bounds, &w.ladders, &shafts);
    println!(
        "sc-client: walk grid of {} walkable cells on {} decks built in {:.0} ms",
        grid.walkable(),
        decks.len(),
        t0.elapsed().as_secs_f64() * 1000.0
    );
    let read = |p: &str| std::fs::read_to_string(p).map_err(|e| format!("{p}: {e}"));
    let company: CompanyData =
        sc_core::data::parse("data/crew/company.json", &read("data/crew/company.json")?).map_err(|e| e.to_string())?;
    let names: sc_core::names::NameData =
        sc_core::data::parse("data/crew/names.json", &read("data/crew/names.json")?).map_err(|e| e.to_string())?;
    let rooms_at: Vec<(String, [f32; 3])> =
        d.index.compartments.iter().filter_map(|c| c.floor_m.map(|f| (c.id.clone(), f))).collect();
    let crew = Crew::new(&company, company.bots(&names, session_seed, 1), &rooms_at, &grid, session_seed, &data);
    println!("sc-client: {} bot crew aboard", crew.bots.len());
    // The player's figure is index `departments.len()` (deckc's "player"), the map's own marker.
    let figures: Vec<(usize, Mesh)> = figures
        .into_iter()
        .filter_map(|(id, m)| {
            let i = if id == "player" {
                Some(company.departments.len())
            } else {
                company.departments.iter().position(|dep| dep.id == id)
            };
            i.map(|i| (i, m))
        })
        .collect();
    let walker = Walker { world, data, body, eye_y, lifts, doors: w.doors.clone(), grid, crew, company, decks };
    // The outside: its data, and the bow camera's target (no MSAA: it is shown a fifth of the screen's size).
    let [bw, bh] = ext.viewscreen.target_px;
    let outside = Outside { bow: r.make_target(bw, bh, 1), views: d.index.views.clone(), data: ext.clone() };
    let grids: Vec<_> = d.index.compartments.iter().filter_map(|c| d.probe_grid(c)).collect();
    let probes = sc_core::probes::ProbeSet::new(&grids);
    println!(
        "sc-client: {} light probes in {} rooms ({:.2} MB)",
        d.index.probe_count,
        probes.len(),
        d.probes.len() as f64 / 1e6
    );
    let scene = Scene::Ship {
        rooms,
        figures,
        probes,
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
    fn new(deck: PathBuf, shots: Option<PathBuf>, walk_test: bool, lobby_test: bool) -> Result<Self, String> {
        let text =
            std::fs::read_to_string("data/engine/render.json").map_err(|e| format!("data/engine/render.json: {e}"))?;
        let cfg = sc_core::data::parse("data/engine/render.json", &text).map_err(|e| e.to_string())?;
        let mut r = Renderer::new(&cfg);
        let target = r.make_target(RENDER_3D.0, RENDER_3D.1, 4);
        let text = std::fs::read_to_string("data/space/exterior.json")
            .map_err(|e| format!("data/space/exterior.json: {e}"))?;
        let exterior: ExteriorData =
            sc_core::data::parse("data/space/exterior.json", &text).map_err(|e| e.to_string())?;
        // The lobby comes first, except for the headless shots and the walk test, which go straight aboard.
        let with_lobby = lobby_test || shots.is_none();
        let (scene, walker, cam, lobby) = if with_lobby {
            let text =
                std::fs::read_to_string("data/crew/names.json").map_err(|e| format!("data/crew/names.json: {e}"))?;
            let names = sc_core::data::parse("data/crew/names.json", &text).map_err(|e| e.to_string())?;
            // A fresh crew each start (the clock is the client's to read, never the core's); a test is fixed.
            let seed = if lobby_test {
                7
            } else {
                std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(1, |d| d.as_nanos() as u64)
            };
            let cam = Camera { pos: [0.0; 3], yaw: 0.0, pitch: 0.0 };
            (Scene::Empty, None, cam, Some(lobby::Lobby::new(names, seed)))
        } else {
            let (scene, walker, cam) = load_scene(&mut r, &deck, &exterior, walk_test, 7)?;
            (scene, walker, cam, None)
        };
        // Headless shots: every pose (the ship) or one view (the test room), in each state; a walk test instead walks.
        // After the ship's poses, a close-up of each of the first BOT_SHOTS bots (light-baking 16: probe-lit figures).
        let bot_shots = walker.as_ref().map_or(0, |w| w.crew.bots.len().min(BOT_SHOTS));
        let poses = if matches!(scene, Scene::Ship { .. }) { POSES.len() + bot_shots } else { 1 };
        let shot_plan = if shots.is_some() && !walk_test && !lobby_test {
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
            ui: ui::Ui::new(),
            lobby,
            beaming: 0,
            officer: None,
            deck,
            exterior,
            lobby_test: lobby_test.then_some(0),
            map: None,
            map_labels: Vec::new(),
        })
    }

    /// A frame of the lobby: the sky turning slowly behind the panel; Beam aboard loads the ship.
    fn lobby_frame(&mut self, f: &Frame) -> Flow {
        let dt = if self.shots.is_some() { 1.0 / 30.0 } else { f.elapsed_s.min(0.1) };
        self.time_s += dt;
        let mut shot = None;
        let mut beam = false;
        if let (Some(n), Some(l)) = (self.lobby_test.as_mut(), self.lobby.as_mut()) {
            *n += 1;
            match *n {
                20 => shot = Some("lobby-1.png"),
                21 => l.roll(),
                40 => shot = Some("lobby-2-rolled.png"),
                41 => beam = true,
                _ => {}
            }
        }
        if self.beaming >= 1 {
            // The frame after the beaming showed: load the ship and go aboard.
            let Some(l) = self.lobby.take() else { return Flow::Fail };
            l.keep();
            self.officer = Some((l.title(), lobby::STATIONS[l.officer.station].to_owned()));
            match load_scene(&mut self.r, &self.deck, &self.exterior, false, l.seed()) {
                Ok((scene, walker, cam)) => {
                    self.scene = scene;
                    self.flying = walker.is_none();
                    self.walker = walker;
                    self.cam = cam;
                }
                Err(e) => {
                    eprintln!("sc-client: {e}");
                    return Flow::Fail;
                }
            }
            self.r.commit();
            return Flow::Continue;
        }
        let yaw = (self.time_s * 0.02) as f32 + 1.2;
        let dir = Vec3::new(yaw.sin() * 0.95, -0.3, yaw.cos() * 0.95).normalize();
        let view = glam::camera::rh::view::look_to_mat4(Vec3::ZERO, dir, Vec3::Y);
        let aspect = RENDER_3D.0 as f32 / RENDER_3D.1 as f32;
        let proj = glam::camera::rh::proj::opengl::perspective(60f32.to_radians(), aspect, 0.05, 300.0);
        self.r.begin_3d(&self.target, [0.0, 0.0, 0.0, 1.0]);
        let px = 60f32.to_radians() / RENDER_3D.1 as f32;
        self.r.draw_sky(&self.target, &sky_params(&self.exterior, proj, view, px));
        self.r.end_pass();
        let Some(l) = self.lobby.as_mut() else { return Flow::Fail };
        let mut action = lobby::LobbyAction::None;
        let beaming = self.beaming;
        let frame = self.ui.run(&mut self.r, f, self.time_s, |ctx| {
            if beaming == 0 {
                action = l.ui(ctx);
            }
        });
        if beam || action == lobby::LobbyAction::BeamAboard {
            self.beaming = 1;
        }
        // The beaming frame: the panel gives way to one line.
        let frame = if self.beaming == 1 {
            self.ui.run(&mut self.r, f, self.time_s, |ctx| {
                egui::Area::new(egui::Id::new("beam")).anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO).show(
                    ctx,
                    |ui| {
                        ui.label(
                            egui::RichText::new("ENERGIZING")
                                .size(40.0)
                                .strong()
                                .color(egui::Color32::from_rgb(242, 160, 70)),
                        );
                    },
                );
            })
        } else {
            frame
        };
        self.r.present_with_ui(&self.target, f.width, f.height, true, &frame.meshes(), frame.points);
        if let Some(n) = shot {
            if let Some(fl) = self.capture(f, n) {
                return fl;
            }
        }
        self.r.commit();
        Flow::Continue
    }

    /// Write the output to `name` in the shots folder; `Some(Fail)` when it cannot be written.
    fn capture(&self, f: &Frame, name: &str) -> Option<Flow> {
        let dir = self.shots.clone().unwrap_or_else(|| PathBuf::from("captures"));
        let px = sc_render::read_output(f.width, f.height);
        if let Err(e) = sc_client::capture::write_png(&dir.join(name), f.width, f.height, &px) {
            eprintln!("sc-client: {e}");
            return Some(Flow::Fail);
        }
        println!("sc-client: wrote {}", dir.join(name).display());
        None
    }

    /// The ship map (ship-plan-view 5): every room cut above its deck's floor, the decks pulled apart, a figure for
    /// every body aboard and its name. Fills `map_labels` for the HUD.
    fn draw_map(&mut self, w: [f32; 3]) {
        self.map_labels.clear();
        let (Some(m), Some(wk), Scene::Ship { rooms, figures, tex, panel_first, panel_glow, .. }) =
            (self.map.as_ref(), self.walker.as_ref(), &self.scene)
        else {
            return;
        };
        let n = wk.decks.len().max(1);
        let deck_index = |c: char| (n - 1).saturating_sub((c as u8).saturating_sub(b'A') as usize).min(n - 1);
        let deck_of_y = |y: f32| {
            (0..wk.decks.len())
                .min_by(|&a, &b| (wk.decks[a] - y + 0.3).abs().total_cmp(&(wk.decks[b] - y + 0.3).abs()))
                .unwrap_or(0)
        };
        let lift = |k: usize| k as f32 * MAP_EXPLODE_M;
        // The camera: an orbit round the ship's middle, the exploded stack's middle height.
        let target = Vec3::new(0.0, wk.decks.first().copied().unwrap_or(0.0) + lift(n - 1) / 2.0, 0.0);
        let dir = Vec3::new(m.pitch.cos() * m.yaw.sin(), m.pitch.sin(), m.pitch.cos() * m.yaw.cos());
        let eye = target + dir * m.dist;
        let view = glam::camera::rh::view::look_to_mat4(Vec3::ZERO, -dir, Vec3::Y);
        let proj = glam::camera::rh::proj::opengl::perspective(
            45f32.to_radians(),
            RENDER_3D.0 as f32 / RENDER_3D.1 as f32,
            1.0,
            600.0,
        );
        let glow = w[0] * panel_glow[0] + w[1] * panel_glow[1] + w[2] * panel_glow[2];
        let shown = |k: usize| m.deck.is_none_or(|d| d == k);
        for room in rooms.iter().filter(|r| !r.outside && r.car.is_none()) {
            let k = deck_index(room.deck);
            if !shown(k) {
                continue;
            }
            let o = Vec3::new(room.origin[0] as f32, room.origin[1] as f32 + lift(k), room.origin[2] as f32);
            let p = DeckParams {
                mvp: proj * view * Mat4::from_translation(o - eye),
                state_weights: w,
                flash_dir: Vec3::Z,
                flash: 0.0,
                panel_first: *panel_first,
                panel_glow: glow,
                clip_y: wk.decks[k] + MAP_CUT_M - room.origin[1] as f32,
            };
            self.r.draw_deck(&self.target, &room.mesh, DeckProgram::Textured, &p, Some(tex));
        }
        // Bodies: every bot in its department's figure, the player in theirs, drawn large; names over them.
        let deps = &wk.company.departments;
        let mut bodies: Vec<([f32; 3], f32, usize, String, egui::Color32, bool)> = wk
            .crew
            .bots
            .iter()
            .map(|b| {
                let c = deps[b.member.department].colour_srgb;
                let col = egui::Color32::from_rgb((c[0] * 255.0) as u8, (c[1] * 255.0) as u8, (c[2] * 255.0) as u8);
                (b.body.feet, b.yaw, b.member.department, b.member.name.clone(), col, false)
            })
            .collect();
        let me = self.officer.as_ref().map_or_else(|| "You".to_owned(), |(t, _)| t.clone());
        bodies.push((wk.body.feet, self.cam.yaw, deps.len(), me, egui::Color32::from_rgb(255, 184, 77), true));
        let vp = proj * view;
        for (feet, yaw, fig, name, colour, own) in bodies {
            let k = deck_of_y(feet[1]);
            if !shown(k) {
                continue;
            }
            let at = Vec3::new(feet[0], feet[1] + lift(k), feet[2]);
            if let Some((_, mesh)) = figures.iter().find(|(d, _)| *d == fig) {
                // The map is a diagram: its figures take the fixed light, not a room's (light-baking 16).
                let turn = Mat4::from_rotation_y(yaw);
                let mvp =
                    vp * Mat4::from_translation(at - eye) * turn * Mat4::from_scale(Vec3::splat(MAP_FIGURE_SCALE));
                self.r.draw_deck_probe(
                    &self.target,
                    mesh,
                    mvp,
                    turn,
                    &sc_core::probes::fallback_cube(),
                    [1.0, 0.0, 0.0],
                );
            }
            let head = vp * (at - eye + Vec3::Y * (MAP_FIGURE_SCALE * 1.9 + 0.6)).extend(1.0);
            if head.w > 0.0 {
                let ndc = head.truncate() / head.w;
                if ndc.x.abs() < 1.1 && ndc.y.abs() < 1.1 {
                    self.map_labels.push(MapLabel {
                        at: [(ndc.x + 1.0) / 2.0, (1.0 - ndc.y) / 2.0],
                        text: name,
                        colour,
                        own,
                    });
                }
            }
        }
    }

    /// The HUD aboard: who you are, and what E and Q do where you stand; on the map, its title and the names over the
    /// markers.
    fn hud(&mut self, f: &Frame) -> ui::UiFrame {
        let officer = self.officer.clone();
        let prompt = self.walker.as_ref().filter(|_| !self.flying && self.map.is_none()).and_then(Walker::prompt);
        let labels = std::mem::take(&mut self.map_labels);
        let map_deck = self.map.as_ref().map(|m| match m.deck {
            None => "ALL DECKS".to_owned(),
            Some(k) => {
                let n = self.walker.as_ref().map_or(3, |w| w.decks.len());
                format!("DECK {}", (b'A' + (n - 1 - k) as u8) as char)
            }
        });
        let frame = self.ui.run(&mut self.r, f, self.time_s, |ctx| {
            if let Some(deck) = &map_deck {
                map_overlay(ctx, deck, &labels);
            }

            if let Some((title, station)) = &officer {
                egui::Area::new(egui::Id::new("officer"))
                    .anchor(egui::Align2::LEFT_TOP, egui::Vec2::new(18.0, 14.0))
                    .show(ctx, |ui| {
                        ui.label(
                            egui::RichText::new(title).size(20.0).strong().color(egui::Color32::from_rgb(242, 160, 70)),
                        );
                        ui.label(egui::RichText::new(station).size(15.0).color(egui::Color32::from_rgb(190, 160, 230)));
                    });
            }
            if let Some(p) = prompt {
                egui::Area::new(egui::Id::new("prompt"))
                    .anchor(egui::Align2::CENTER_BOTTOM, egui::Vec2::new(0.0, -40.0))
                    .show(ctx, |ui| {
                        egui::Frame::new()
                            .fill(egui::Color32::from_rgba_premultiplied(8, 12, 20, 210))
                            .corner_radius(16.0)
                            .inner_margin(egui::Margin::symmetric(18, 8))
                            .show(ui, |ui| {
                                ui.label(
                                    egui::RichText::new(p).size(20.0).color(egui::Color32::from_rgb(225, 232, 245)),
                                );
                            });
                    });
            }
        });
        self.map_labels = labels;
        frame
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
        if self.lobby.is_some() {
            return self.lobby_frame(f);
        }
        if let Some(n) = self.lobby_test.as_mut() {
            // Aboard after the lobby test: settle, shoot the bridge, done.
            *n += 1;
            let n = *n;
            // Aboard: shoot the bridge; open the map and shoot it whole; then deck B alone (ship-plan-view 4.4).
            match n {
                120 | 200 | 240 => self.capture_next = true,
                121 => {
                    let _ = self.event(Event::KeyDown(keys::M));
                }
                201 => {
                    let _ = self.event(Event::KeyDown(keys::TAB));
                    let _ = self.event(Event::KeyDown(keys::TAB));
                }
                241.. => return Flow::Done,
                _ => {}
            }
        }
        let headless = self.shots.is_some() && self.lobby_test.is_none();
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
            let bot = p.checked_sub(POSES.len());
            if let (Some(k), Some(w)) = (bot, self.walker.as_ref()) {
                // A bot close-up: the camera in front of it, facing it, following it as it walks.
                let b = &w.crew.bots[k];
                let (f, (sy, cy)) = (b.body.feet, b.yaw.sin_cos());
                let pos = [f64::from(f[0] + sy * BOT_SHOT_M), f64::from(f[1]), f64::from(f[2] + cy * BOT_SHOT_M)];
                self.cam = Camera::at(&("bot", pos, [-sy, -cy], -14.0));
            } else if matches!(self.scene, Scene::Ship { .. }) {
                self.cam = Camera::at(&POSES[p]);
            }
            self.shot_frame += 1;
            if self.shot_frame == 40 {
                let bot_name = bot.map(|k| format!("bot-{k}"));
                let pose = match (&bot_name, matches!(self.scene, Scene::Ship { .. })) {
                    (Some(n), _) => n.as_str(),
                    (None, true) => POSES[p].0,
                    (None, false) => "first-light",
                };
                shot_name = Some(format!("{pose}-{}.png", ["normal", "red-alert", "emergency"][s]));
                self.shot_plan.remove(0);
                self.shot_frame = 0;
            }
        } else if self.map.is_some() {
            // The body stands where it was; the lift runs on.
            if let Some(w) = self.walker.as_mut() {
                w.step_lifts(dt as f32);
            }
        } else if self.flying {
            self.fly(dt);
        } else {
            self.walk(dt, None);
        }
        // The bot crew walk whatever the player is doing (walking, flying, a test's route).
        if let Some(w) = self.walker.as_mut() {
            w.crew.step(&w.world, &w.grid, &w.data, dt as f32);
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
                    clip_y: f32::MAX,
                };
                self.r.draw_deck(&o.bow, &room.mesh, DeckProgram::Textured, &p, Some(tex));
            }
            self.r.end_pass();
        }
        if self.map.is_some() {
            self.r.begin_3d(&self.target, [0.012, 0.016, 0.026, 1.0]);
            self.draw_map(w);
            self.r.end_pass();
            let hud = self.hud(f);
            self.r.present_with_ui(&self.target, f.width, f.height, true, &hud.meshes(), hud.points);
            if self.capture_next {
                self.capture_next = false;
                let n = match (self.lobby_test, self.map.as_ref().and_then(|m| m.deck)) {
                    (Some(_), None) => "map-1-all-decks.png".to_owned(),
                    (Some(_), Some(_)) => "map-2-deck-B.png".to_owned(),
                    _ => format!("capture-{}.png", f.index),
                };
                if let Some(fl) = self.capture(f, &n) {
                    return fl;
                }
            }
            self.r.commit();
            return Flow::Continue;
        }
        self.r.begin_3d(&self.target, [0.004, 0.005, 0.012, 1.0]);
        match &self.scene {
            Scene::Ship { rooms, figures, probes, outside, tex, panel_first, panel_glow, .. } => {
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
                        clip_y: f32::MAX,
                    };
                    self.r.draw_deck(&self.target, mesh, DeckProgram::Textured, &p, Some(tex));
                }
                // The bot crew: a figure a bot, in its department's tunic (crew-npcs 7), lit by the light probes at
                // its chest, 1.0 m above its feet (light-baking 16); where none is valid, by the figures' old light.
                if let Some(wk) = self.walker.as_ref() {
                    for b in &wk.crew.bots {
                        let Some((_, mesh)) = figures.iter().find(|(d, _)| *d == b.member.department) else { continue };
                        let f = b.body.feet;
                        let chest = [f64::from(f[0]), f64::from(f[1]) + FIGURE_PROBE_HEIGHT_M, f64::from(f[2])];
                        let cube = probes.sample(chest, w).unwrap_or_else(sc_core::probes::fallback_cube);
                        let rel = Vec3::new(
                            (f64::from(f[0]) - self.cam.pos[0]) as f32,
                            (f64::from(f[1]) - self.cam.pos[1]) as f32,
                            (f64::from(f[2]) - self.cam.pos[2]) as f32,
                        );
                        let turn = Mat4::from_rotation_y(b.yaw);
                        self.r.draw_deck_probe(
                            &self.target,
                            mesh,
                            proj * view * Mat4::from_translation(rel) * turn,
                            turn,
                            &cube,
                            w,
                        );
                    }
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
                    clip_y: f32::MAX,
                };
                self.r.draw_deck(&self.target, room, DeckProgram::Textured, &p, Some(tex));
            }
            Scene::Empty => {}
        }
        self.r.end_pass();
        let hud = self.hud(f);
        self.r.present_with_ui(&self.target, f.width, f.height, true, &hud.meshes(), hud.points);
        if self.capture_next {
            shot_name = Some(if self.lobby_test.is_some() {
                "lobby-3-aboard.png".to_owned()
            } else {
                format!("capture-{}.png", f.index)
            });
            self.capture_next = false;
        }
        if let Some(n) = shot_name {
            if let Some(fl) = self.capture(f, &n) {
                return fl;
            }
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
        if self.lobby.is_some() {
            // The lobby's input is the UI's.
            if e == Event::Quit {
                return Flow::Done;
            }
            self.ui.event(&e);
            return Flow::Continue;
        }
        match e {
            Event::Quit => return Flow::Done,
            Event::KeyDown(keys::ESCAPE) => {
                if !self.captured {
                    return Flow::Done;
                }
                self.captured = false;
                platform::capture_mouse(false);
            }
            // On the map, Tab steps through the decks: all, then each from the top.
            Event::KeyDown(keys::TAB) if self.map.is_some() => {
                let n = self.walker.as_ref().map_or(3, |w| w.decks.len());
                if let Some(m) = self.map.as_mut() {
                    m.deck = match m.deck {
                        None => Some(n - 1),
                        Some(0) => None,
                        Some(k) => Some(k - 1),
                    };
                }
            }
            Event::KeyDown(keys::TAB) => {
                self.captured = false;
                platform::capture_mouse(false);
            }
            // M opens and closes the ship map, wherever the player is (ship-plan-view 5).
            Event::KeyDown(keys::M) if self.walker.is_some() => {
                self.map = match self.map {
                    Some(_) => None,
                    None => Some(MapView { yaw: 2.1, pitch: 0.55, dist: 100.0, deck: None }),
                };
                self.held.clear();
            }
            Event::Wheel(_, y) if self.map.is_some() => {
                if let Some(m) = self.map.as_mut() {
                    m.dist = (m.dist * 1.12f32.powf(-y)).clamp(25.0, 260.0);
                }
            }
            Event::MouseMotion(dx, dy) if self.captured && self.map.is_some() => {
                if let Some(m) = self.map.as_mut() {
                    m.yaw -= dx * LOOK_RAD_PX;
                    m.pitch = (m.pitch + dy * LOOK_RAD_PX).clamp(0.1, 1.5);
                }
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
            Event::MouseMotion(..)
            | Event::Pointer(..)
            | Event::MouseButton(..)
            | Event::Wheel(..)
            | Event::Text(_)
            | Event::KeyRepeat(_) => {}
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
    let lobby_dir = value("--lobby-test");
    let lobby_test = lobby_dir.is_some();
    let shots = shots.or(walk_dir).or(lobby_dir);
    // The console parity check's engine side (openspec/changes/console-parity design 4): one state, one capture.
    if let Some(state) = value("--console-fixture") {
        let Some(dir) = shots.clone() else {
            eprintln!("sc-client: --console-fixture needs --shots DIR");
            return ExitCode::from(2);
        };
        let cfg = WindowConfig {
            title: "Star Crew: console fixture".into(),
            width: 1280,
            height: 720,
            fullscreen: false,
            headless: true,
            vsync: false,
        };
        return platform::run(cfg, move |gl| {
            println!("sc-client: {} on {} ({})", gl.version, gl.renderer, gl.video_driver);
            fixture::FixtureApp::new(state, dir).map(|c| Box::new(c) as Box<dyn App>)
        });
    }
    // The co-op drill (openspec/changes/coop-drill design 5): its own mode, joined by address.
    if let Some(server) = args.iter().position(|a| a == "--connect").and_then(|i| args.get(i + 1)).cloned() {
        let station = args.iter().position(|a| a == "--station").and_then(|i| args.get(i + 1));
        let Some(station) = station.map(String::as_str).and_then(sc_core::combat::Station::from_id) else {
            eprintln!("sc-client: --connect needs --station helm or tactical");
            return ExitCode::from(2);
        };
        let bot = args.iter().any(|a| a == "--bot");
        let name = args
            .iter()
            .position(|a| a == "--name")
            .and_then(|i| args.get(i + 1))
            .cloned()
            .unwrap_or_else(|| format!("{} officer{}", station.name(), if bot { " (bot)" } else { "" }));
        let cfg = WindowConfig {
            title: "Star Crew: drill".into(),
            width: 1920,
            height: 1080,
            fullscreen: !headless && !args.iter().any(|a| a == "--window"),
            headless,
            vsync: !headless,
        };
        let dargs = drill::DrillArgs { server, station, name, bot, shots };
        return platform::run(cfg, move |gl| {
            println!("sc-client: {} on {} ({})", gl.version, gl.renderer, gl.video_driver);
            drill::DrillApp::new(dargs).map(|c| Box::new(c) as Box<dyn App>)
        });
    }
    if headless && shots.is_none() && !args.iter().any(|a| a == "--connect") {
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
        Client::new(deck, shots, walk_test, lobby_test).map(|c| Box::new(c) as Box<dyn App>)
    })
}
