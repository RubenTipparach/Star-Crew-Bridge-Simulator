//! The co-op drill in the client (openspec/changes/coop-drill design 5): connect to a drill server, read the
//! briefing, sit at Helm or Tactical (the `bridge-stations` 8.0 layouts) over a 3D bow view in which the Hound is a
//! placeholder model, then the debrief. `--bot` lets the station's automation play the seat at a player's
//! competence, through the same commands the console sends.
//!
//! It is its own `App` beside the walkable ship because the drill is a separate game mode: nothing here walks a
//! deck. Every rule it shows comes from `sc-core` (the hit chance, the lock, the tube timings), every number from
//! the snapshot; the client decides nothing a console could not.

use std::collections::HashSet;
use std::net::{SocketAddr, ToSocketAddrs};
use std::path::PathBuf;

use egui::{Align2, Color32, CornerRadius, FontId, Pos2, Rect, RichText, Sense, Stroke, StrokeKind, Vec2};
use glam::{DQuat, DVec3, Mat4, Quat, Vec3};
use sc_client::platform::{keys, App, Event, Flow, Frame};
use sc_core::combat::data::{DrillData, DRILL_FILES};
use sc_core::combat::{Command, HelmMode, Outcome, Phase, Station, TubeState, ENEMY_ID, TERN_ID};
use sc_core::exterior::{normalized, ExteriorData};
use sc_net::bot::Bot;
use sc_net::msg::{ShipSnap, Snapshot};
use sc_net::session::{Session, Stage, Visual, INTERP_DELAY_S};
use sc_net::transport::Impair;
use sc_render::{DeckParams, DeckProgram, Indices, Mesh, Renderer, Target};

use crate::ships3d;
use crate::ui::Ui;

/// The 3D target (engine-stack design 5).
const RENDER_3D: (u32, u32) = (1280, 720);
/// The target camera's inset (a camera feed, bridge-stations 8.0), pixels.
const INSET: (u32, u32) = (448, 252);
/// The console canvas, logical points (bridge-stations 8.0).
const CANVAS: (f32, f32) = (1280.0, 720.0);

const PANEL: Color32 = Color32::from_rgba_premultiplied(10, 16, 26, 232);
const EDGE: Color32 = Color32::from_rgb(44, 62, 82);
const TEXT: Color32 = Color32::from_rgb(222, 230, 238);
const DIM: Color32 = Color32::from_rgb(128, 146, 164);
const CYAN: Color32 = Color32::from_rgb(88, 200, 232);
const AMBER: Color32 = Color32::from_rgb(242, 160, 70);
const RED: Color32 = Color32::from_rgb(232, 82, 70);
const GREEN: Color32 = Color32::from_rgb(112, 204, 122);

/// What `--connect` was given.
pub struct DrillArgs {
    /// `host` or `host:port`.
    pub server: String,
    /// The station to take.
    pub station: Station,
    /// The officer's name.
    pub name: String,
    /// Whether the station's automation plays it.
    pub bot: bool,
    /// Where captures go, for the scripted shots.
    pub shots: Option<PathBuf>,
}

struct Meshes {
    hound: Mesh,
    bolt_tern: Mesh,
    bolt_enemy: Mesh,
    missile: Mesh,
    burst: Mesh,
}

struct VisBolt {
    id: u32,
    owner: u16,
    pos: DVec3,
    vel: DVec3,
    /// Local time it left the muzzle.
    t0: f64,
    life_s: f64,
    /// Local time it struck, if it did.
    hit_at: Option<f64>,
}

struct Boom {
    pos: DVec3,
    t0: f64,
    dur: f64,
    radius: f64,
}

#[derive(Default)]
struct HelmControls {
    speed_set: f64,
    synced_round: Option<u32>,
    pad: [f64; 2],
    pad_held: bool,
    last_send: f64,
}

/// The drill client.
pub struct DrillApp {
    r: Renderer,
    target: Target,
    ui: Ui,
    exterior: ExteriorData,
    data: DrillData,
    meshes: Meshes,
    inset: Option<Target>,
    args: DrillArgs,
    addr: Option<SocketAddr>,
    session: Option<Session>,
    error: Option<String>,
    retry_at: f64,
    bot: Option<Bot>,
    time_s: f64,
    bolts: Vec<VisBolt>,
    booms: Vec<Boom>,
    held: HashSet<u32>,
    pressed: HashSet<u32>,
    helm: HelmControls,
    fire_hold: Option<(u8, f64)>,
    face_hit: [f64; 6],
    shots_taken: HashSet<String>,
    phase_seen: Option<(u32, Phase, f64)>,
    shot_round: Option<u32>,
    done: bool,
    /// The bridge, when the compiled deck is there (design 9).
    bridge: Option<crate::drill_bridge::BridgeView>,
    /// The overview of the bridge is up (V).
    overview: bool,
    /// The scripted shots: since when this player's body, and another's, has been seen walking.
    walk_seen: (Option<f64>, Option<f64>),
}

fn load_data() -> Result<DrillData, String> {
    let read = |rel: &str| std::fs::read_to_string(rel).map_err(|e| format!("{rel}: {e}"));
    let t: Vec<String> = DRILL_FILES.iter().map(|f| read(f)).collect::<Result<_, _>>()?;
    let arr: [&str; 6] = std::array::from_fn(|i| t[i].as_str());
    let m = "data/missions/drill-hound.json";
    DrillData::parse(arr, m, &read(m)?).map_err(|e| e.to_string())
}

fn mesh(r: &Renderer, m: &ships3d::MeshBuilder) -> Mesh {
    r.make_mesh(&m.vertices, Indices::U16(&m.indices))
}

impl DrillApp {
    /// Set up the renderer and start connecting.
    pub fn new(args: DrillArgs) -> Result<Self, String> {
        let text =
            std::fs::read_to_string("data/engine/render.json").map_err(|e| format!("data/engine/render.json: {e}"))?;
        let cfg = sc_core::data::parse("data/engine/render.json", &text).map_err(|e| e.to_string())?;
        let mut r = Renderer::new(&cfg);
        let target = r.make_target(RENDER_3D.0, RENDER_3D.1, 4);
        let inset = r.make_target(INSET.0, INSET.1, 1);
        let text = std::fs::read_to_string("data/space/exterior.json")
            .map_err(|e| format!("data/space/exterior.json: {e}"))?;
        let exterior: ExteriorData =
            sc_core::data::parse("data/space/exterior.json", &text).map_err(|e| e.to_string())?;
        let data = load_data()?;
        let hound = ships3d::hound();
        println!("sc-client: the Hound placeholder is {} triangles", hound.triangles());
        let meshes = Meshes {
            hound: mesh(&r, &hound),
            bolt_tern: mesh(&r, &ships3d::bolt([0.6, 1.8, 2.0])),
            bolt_enemy: mesh(&r, &ships3d::bolt([2.0, 0.8, 0.4])),
            missile: mesh(&r, &ships3d::missile()),
            burst: mesh(&r, &ships3d::burst()),
        };
        let host = if args.server.contains(':') { args.server.clone() } else { format!("{}:7700", args.server) };
        let addr = host.to_socket_addrs().ok().and_then(|mut a| a.find(SocketAddr::is_ipv4));
        let bot = args.bot.then(|| Bot::new(&data, args.station));
        let mut app = Self {
            r,
            target,
            ui: Ui::new(),
            exterior,
            data,
            meshes,
            inset: Some(inset),
            args,
            addr,
            session: None,
            error: None,
            retry_at: 0.0,
            bot,
            time_s: 0.0,
            bolts: Vec::new(),
            booms: Vec::new(),
            held: HashSet::new(),
            pressed: HashSet::new(),
            helm: HelmControls::default(),
            fire_hold: None,
            face_hit: [-10.0; 6],
            shots_taken: HashSet::new(),
            phase_seen: None,
            shot_round: None,
            done: false,
            bridge: None,
            overview: false,
            walk_seen: (None, None),
        };
        match crate::drill_bridge::BridgeView::load(&mut app.r, std::path::Path::new("compiled/tern.deck")) {
            Ok(b) => {
                println!("sc-client: the bridge for the crew on foot, {} triangles", b.triangles);
                app.bridge = Some(b);
            }
            Err(e) => println!("sc-client: no bridge to walk ({e}); the drill shows the consoles only"),
        }
        app.try_connect();
        Ok(app)
    }

    fn try_connect(&mut self) {
        let Some(addr) = self.addr else {
            self.error = Some(format!("Cannot find {}", self.args.server));
            return;
        };
        match Session::connect(
            addr,
            &self.args.name,
            self.args.bot,
            Some(self.args.station),
            Impair::default(),
            self.time_s.to_bits(),
        ) {
            Ok(s) => {
                println!("sc-client: connecting to {addr} as {} for {}", self.args.name, self.args.station.name());
                self.session = Some(s);
                self.error = None;
            }
            Err(e) => {
                self.error = Some(e);
                self.retry_at = self.time_s + 3.0;
            }
        }
    }

    /// The Tern and the Hound as drawn now: interpolated between snapshots, `INTERP_DELAY_S` behind.
    fn ships_now(&self) -> Option<(ShipSnap, ShipSnap, Snapshot)> {
        let s = self.session.as_ref()?;
        let (a, b, f) = s.interpolation()?;
        let lerp = |x: &ShipSnap, y: &ShipSnap| {
            let mut out = y.clone();
            if x.id == y.id && x.active == y.active {
                out.pos = x.pos.lerp(y.pos, f);
                out.rot = x.rot.slerp(y.rot, f);
                out.vel = x.vel.lerp(y.vel, f);
            }
            out
        };
        if a.ships.len() < 2 || b.ships.len() < 2 {
            return None;
        }
        Some((lerp(&a.ships[0], &b.ships[0]), lerp(&a.ships[1], &b.ships[1]), b.clone()))
    }

    fn take_visuals(&mut self) {
        let Some(s) = self.session.as_mut() else { return };
        let rtt = s.rtt_ms.unwrap_or(20.0) / 1000.0;
        let now = self.time_s;
        let tern_gun = self.data.gun(&self.data.tern_combat.gun).life_s;
        let enemy_gun = self.data.gun(&self.data.enemy.combat.gun).life_s;
        for v in s.drain_visuals() {
            match v {
                Visual::Fired { bolt, owner, pos, vel } => self.bolts.push(VisBolt {
                    id: bolt,
                    owner,
                    pos,
                    vel,
                    t0: now - rtt / 2.0,
                    life_s: if owner == TERN_ID { tern_gun } else { enemy_gun },
                    hit_at: None,
                }),
                Visual::Hit { bolt, target, face } => {
                    if let Some(b) = self.bolts.iter_mut().find(|b| b.id == bolt) {
                        b.hit_at = Some(now - rtt / 2.0);
                        let at = b.pos + b.vel * (now - rtt / 2.0 - b.t0);
                        self.booms.push(Boom { pos: at, t0: now - rtt / 2.0, dur: 0.35, radius: 4.0 });
                    }
                    if target == TERN_ID {
                        self.face_hit[usize::from(face.min(5))] = now;
                    }
                }
                Visual::Detonated { pos, damage_mj } => {
                    let radius = if damage_mj > 0.0 { 30.0 } else { 12.0 };
                    self.booms.push(Boom { pos, t0: now - rtt / 2.0, dur: 1.2, radius });
                }
                Visual::Destroyed(id) => {
                    if id == ENEMY_ID {
                        if let Some((_, h, _)) = self.ships_now() {
                            self.booms.push(Boom { pos: h.pos, t0: now, dur: 2.5, radius: 70.0 });
                        }
                    }
                }
            }
        }
        let render_t = now - INTERP_DELAY_S;
        self.bolts.retain(|b| render_t - b.t0 < b.life_s && b.hit_at.is_none_or(|h| render_t < h));
        self.booms.retain(|b| render_t - b.t0 < b.dur);
    }

    fn console_input(&mut self, snap: &Snapshot, station: Station) {
        let Some(s) = self.session.as_mut() else { return };
        let tern = &snap.ships[0];
        let pressed = std::mem::take(&mut self.pressed);
        let now = self.time_s;
        match station {
            Station::Helm => {
                if self.helm.synced_round != Some(snap.round) {
                    self.helm.synced_round = Some(snap.round);
                    self.helm.speed_set = tern.speed_set;
                }
                let f = &self.data.tern_flight;
                if pressed.contains(&keys::W) {
                    self.helm.speed_set = (self.helm.speed_set + 25.0).min(f.speed_max_mps);
                }
                if pressed.contains(&keys::S) {
                    self.helm.speed_set = (self.helm.speed_set - 25.0).max(f.speed_min_mps);
                }
                if pressed.contains(&keys::BACKSPACE) {
                    self.helm.speed_set = 0.0;
                    s.command(Command::AllStop);
                }
                if pressed.contains(&keys::T) {
                    s.command(Command::Helm(HelmMode::Target));
                }
                if pressed.contains(&keys::L) {
                    s.command(Command::Helm(HelmMode::Level));
                }
                let axis = |p: u32, n: u32, held: &HashSet<u32>| {
                    f64::from(u8::from(held.contains(&p))) - f64::from(u8::from(held.contains(&n)))
                };
                let (mut yaw, mut pitch) = (axis(keys::A, keys::D, &self.held), axis(keys::R, keys::F, &self.held));
                let roll = axis(keys::E, keys::Q, &self.held);
                if self.helm.pad_held {
                    yaw = self.helm.pad[0];
                    pitch = self.helm.pad[1];
                }
                if now - self.helm.last_send >= 0.05 {
                    self.helm.last_send = now;
                    s.command(Command::Stick { speed_set_mps: self.helm.speed_set, yaw, pitch, roll });
                }
            }
            Station::Tactical => {
                if pressed.contains(&keys::T) {
                    s.command(Command::Lock(Some(ENEMY_ID)));
                }
                if pressed.contains(&keys::K) {
                    s.command(Command::WeaponsFree(!tern.weapons_free));
                }
                if pressed.contains(&keys::S) {
                    let n = self.data.tern_shields.presets.len() as u8;
                    s.command(Command::Preset((tern.preset + 1) % n.max(1)));
                }
                if pressed.contains(&keys::L) {
                    if let Some(i) = tern.tubes.iter().position(|t| t.0 == TubeState::Empty) {
                        s.command(Command::Load(i as u8));
                    }
                }
                if self.held.contains(&keys::G) {
                    let armed = tern.tubes.iter().position(|t| t.0 == TubeState::Armed);
                    match (self.fire_hold, armed) {
                        (None, Some(i)) => self.fire_hold = Some((i as u8, now)),
                        (Some((i, t0)), _) if now - t0 >= 0.6 => {
                            s.command(Command::Fire(i));
                            self.fire_hold = None;
                            self.held.remove(&keys::G);
                        }
                        _ => {}
                    }
                } else if self.fire_hold.is_some_and(|(_, t0)| now - t0 < 0.6) && !self.held.contains(&keys::G) {
                    // A key hold released early: nothing fires (bridge-stations: guarded controls).
                    // The mouse hold on FIRE is handled in the console.
                }
            }
        }
    }

    /// The camera on the Tern's bow: its eye in the system frame and its attitude.
    fn bow_eye(ships: Option<&(ShipSnap, ShipSnap, Snapshot)>, time_s: f64) -> (DVec3, DQuat) {
        match ships {
            Some((t, _, _)) => (t.pos + t.rot * DVec3::new(0.0, 3.0, 32.0), t.rot),
            None => (DVec3::ZERO, DQuat::from_rotation_y(time_s * 0.02)),
        }
    }

    /// The bow view, then the target camera's inset in the look band. `aspect` is the window's, so the 16:9 target
    /// stretched to any window keeps true proportions.
    fn render_3d(&mut self, ships: Option<&(ShipSnap, ShipSnap, Snapshot)>, aspect: f32) {
        let (eye, rot) = Self::bow_eye(ships, self.time_s);
        // The view's centre lifted into the look band (y 32-272 of 720): shift NDC y by (360 - 152) / 360.
        let shift = Mat4::from_translation(Vec3::new(0.0, (360.0 - 152.0) / 360.0, 0.0));
        let fov = 62f32.to_radians();
        let proj = shift * glam::camera::rh::proj::opengl::perspective(fov, aspect, 1.0, 80_000.0);
        let view =
            glam::camera::rh::view::look_to_mat4(Vec3::ZERO, (rot * DVec3::Z).as_vec3(), (rot * DVec3::Y).as_vec3());
        // The target camera: from the same eye, a narrow view that keeps the Hound about half the inset tall.
        let inset = ships.and_then(|(t, h, _)| {
            (h.active && h.alive).then(|| {
                let to = h.pos - eye;
                let d = to.length().max(1.0);
                let fov = (2.0 * (40.0 / d).atan()).clamp(0.5f64.to_radians(), 50f64.to_radians()) as f32;
                let proj = glam::camera::rh::proj::opengl::perspective(fov, 16.0 / 9.0, 1.0, 80_000.0);
                let view = glam::camera::rh::view::look_to_mat4(
                    Vec3::ZERO,
                    to.as_vec3().normalize(),
                    (t.rot * DVec3::Y).as_vec3(),
                );
                (proj, view, fov)
            })
        });
        let world = World {
            meshes: &self.meshes,
            exterior: &self.exterior,
            bolts: &self.bolts,
            booms: &self.booms,
            time_s: self.time_s,
        };
        if let (Some((p, v, fov)), Some(it)) = (inset, self.inset.as_ref()) {
            self.r.begin_3d(it, [0.0, 0.0, 0.0, 1.0]);
            world.draw(&mut self.r, it, p, v, fov / INSET.1 as f32, eye, ships);
            self.r.end_pass();
        }
        self.r.begin_3d(&self.target, [0.0, 0.0, 0.0, 1.0]);
        world.draw(&mut self.r, &self.target, proj, view, fov / RENDER_3D.1 as f32, eye, ships);
        if let (Some(_), Some(it)) = (inset, self.inset.as_ref()) {
            // The inset's quad in the look band, left of centre: 384 x 216 lp at (16, 44) on the canvas.
            let screen_w = 720.0 * aspect;
            let x0 = (screen_w - CANVAS.0) / 2.0 + 16.0;
            let (w, h, y0) = (384.0, 216.0, 44.0);
            let cx = (x0 + w / 2.0) / screen_w * 2.0 - 1.0;
            let cy = 1.0 - (y0 + h / 2.0) / 720.0 * 2.0;
            let mvp = Mat4::from_translation(Vec3::new(cx, cy, 0.0))
                * Mat4::from_scale(Vec3::new(w / screen_w, h / 720.0, 1.0));
            self.r.draw_screen(&self.target, mvp, it, [0.0, INSET.1 as f32, 1.0, 0.0]);
        }
        self.r.end_pass();
    }
}

/// What the 3D views draw, borrowed from the app so the renderer can be borrowed beside it.
struct World<'a> {
    meshes: &'a Meshes,
    exterior: &'a ExteriorData,
    bolts: &'a [VisBolt],
    booms: &'a [Boom],
    time_s: f64,
}

impl World<'_> {
    /// The sky, the Hound, missiles, bolts and bursts, seen from `eye` through `proj * view` into `t`'s pass.
    #[allow(clippy::too_many_arguments)]
    fn draw(
        &self,
        r: &mut Renderer,
        t: &Target,
        proj: Mat4,
        view: Mat4,
        px_rad: f32,
        eye: DVec3,
        ships: Option<&(ShipSnap, ShipSnap, Snapshot)>,
    ) {
        let sky = crate::sky_params(self.exterior, proj, view, px_rad);
        r.draw_sky(t, &sky);
        let vp = proj * view;
        let rel = |p: DVec3| (p - eye).as_vec3();
        let flat = |mvp: Mat4| DeckParams {
            mvp,
            state_weights: [1.0, 0.0, 0.0],
            flash_dir: Vec3::ZERO,
            flash: 0.0,
            panel_first: u32::MAX,
            panel_glow: 1.0,
            clip_y: f32::MAX,
        };
        if let Some((_, hound, snap)) = ships {
            if hound.active && hound.alive {
                let sun = Vec3::from_array(normalized(self.exterior.sun.dir));
                let axes = [Vec3::X, -Vec3::X, Vec3::Y, -Vec3::Y, Vec3::Z, -Vec3::Z];
                let cube = axes.map(|a| {
                    let s = a.dot(sun).max(0.0);
                    let fill = (-a.dot(sun)).max(0.0) * 0.12;
                    [0.2 + 1.3 * s + fill * 0.6, 0.2 + 1.25 * s + fill * 0.7, 0.22 + 1.15 * s + fill]
                });
                let q = Quat::from_xyzw(hound.rot.x as f32, hound.rot.y as f32, hound.rot.z as f32, hound.rot.w as f32);
                let model = Mat4::from_rotation_translation(q, rel(hound.pos));
                r.draw_deck_probe(t, &self.meshes.hound, vp * model, Mat4::from_quat(q), &cube, [1.0, 0.0, 0.0]);
            }
            for m in &snap.missiles {
                let dir = m.vel.normalize_or(DVec3::Z).as_vec3();
                let model = Mat4::from_rotation_translation(Quat::from_rotation_arc(Vec3::Z, dir), rel(m.pos))
                    * Mat4::from_scale(Vec3::splat(1.5));
                r.draw_deck(t, &self.meshes.missile, DeckProgram::Flat, &flat(vp * model), None);
            }
        }
        let render_t = self.time_s - INTERP_DELAY_S;
        for b in self.bolts {
            let age = render_t - b.t0;
            if age < 0.0 {
                continue;
            }
            let p = b.pos + b.vel * age;
            let dir = b.vel.normalize_or(DVec3::Z).as_vec3();
            let model = Mat4::from_rotation_translation(Quat::from_rotation_arc(Vec3::Z, dir), rel(p))
                * Mat4::from_scale(Vec3::new(6.0, 6.0, 20.0));
            let m = if b.owner == TERN_ID { &self.meshes.bolt_tern } else { &self.meshes.bolt_enemy };
            r.draw_deck(t, m, DeckProgram::Flat, &flat(vp * model), None);
        }
        for b in self.booms {
            let k = ((render_t - b.t0) / b.dur).clamp(0.0, 1.0);
            let radius = (b.radius * k.sqrt()).max(0.5) as f32;
            let model = Mat4::from_translation(rel(b.pos)) * Mat4::from_scale(Vec3::splat(radius));
            r.draw_deck(t, &self.meshes.burst, DeckProgram::Flat, &flat(vp * model), None);
        }
    }
}

impl DrillApp {
    /// The crew's bodies 100 ms behind the newest snapshot, as the ships are drawn, each with whether it is a bot's.
    fn bodies_now(&self) -> Vec<(sc_net::msg::BodySnap, bool)> {
        let Some(s) = self.session.as_ref() else { return Vec::new() };
        let Some((a, b, f)) = s.interpolation() else { return Vec::new() };
        crate::drill_bridge::blend(&a.bodies, &b.bodies, f)
            .into_iter()
            .map(|x| {
                let bot = s.crew.iter().any(|c| c.slot == x.slot && c.bot);
                (x, bot)
            })
            .collect()
    }

    /// The scripted shots' overview frames: the first half second after another crew member is seen walking.
    fn shot_overview(&self) -> bool {
        self.walk_seen.1.is_some_and(|t| self.time_s - t < 0.6)
            && !self.shots_taken.iter().any(|n| n.starts_with("0-bridge"))
    }

    fn capture(&mut self, f: &Frame, name: &str) {
        if !self.shots_taken.insert(name.to_owned()) {
            return;
        }
        let Some(dir) = self.args.shots.clone() else { return };
        let px = sc_render::read_output(f.width, f.height);
        match sc_client::capture::write_png(&dir.join(name), f.width, f.height, &px) {
            Ok(()) => println!("sc-client: wrote {}", dir.join(name).display()),
            Err(e) => eprintln!("sc-client: {e}"),
        }
    }

    fn scripted_shots(&mut self, f: &Frame) {
        if self.args.shots.is_none() {
            return;
        }
        let Some(snap) = self.session.as_ref().and_then(|s| s.latest()).cloned() else { return };
        let st = self.args.station.id();
        let t = snap.phase_s;
        // Shoot only a round seen from its Muster, so every shot is of a fight watched from the start.
        if snap.phase == Phase::Muster {
            self.shot_round = Some(snap.round);
        }
        if self.shot_round != Some(snap.round) {
            return;
        }
        // On foot: the overview while the crew walk, then this player's own walk if it has one.
        if !snap.bodies.is_empty() && self.bridge.is_some() {
            if snap.phase == Phase::Muster && (1.0..1.4).contains(&t) {
                self.capture(f, &format!("0-bridge-{st}.png"));
            }
            let me = self.session.as_ref().and_then(|s| s.slot);
            if snap.bodies.iter().any(|b| Some(b.slot) == me && b.posture == sc_core::combat::bodies::Posture::Walking)
                && t > 1.5
            {
                self.capture(f, &format!("0-walking-{st}.png"));
            }
        }
        let plan: &[(Phase, f64, &str)] = &[
            (Phase::Muster, 1.5, "1-briefing"),
            (Phase::Countdown, 2.0, "2-countdown"),
            (Phase::Engage, 6.0, "3-engage-a"),
            (Phase::Engage, 22.0, "4-engage-b"),
            (Phase::Engage, 40.0, "5-engage-c"),
            (Phase::Debrief, 2.0, "6-debrief"),
        ];
        for (p, at, name) in plan {
            if snap.phase == *p && t >= *at {
                self.capture(f, &format!("{name}-{st}.png"));
            }
        }
        // Done once a whole fight has been shot: the last engage shot, then its debrief.
        if self.shots_taken.iter().any(|n| n.starts_with("5-engage")) && snap.phase == Phase::Debrief && t >= 2.0 {
            self.shots_taken.remove(&format!("6-debrief-{st}.png"));
            self.capture(f, &format!("6-debrief-{st}.png"));
            self.done = true;
        }
    }
}

impl App for DrillApp {
    fn event(&mut self, e: Event) -> Flow {
        self.ui.event(&e);
        match e {
            Event::Quit => return Flow::Done,
            Event::KeyDown(keys::ESCAPE) => return Flow::Done,
            Event::KeyDown(keys::V) => self.overview = !self.overview,
            Event::KeyDown(k) => {
                if !self.held.contains(&k) {
                    self.pressed.insert(k);
                }
                self.held.insert(k);
            }
            Event::KeyUp(k) => {
                self.held.remove(&k);
            }
            _ => {}
        }
        Flow::Continue
    }

    fn frame(&mut self, f: &Frame) -> Flow {
        let dt = f.elapsed_s.clamp(0.0, 0.1);
        self.time_s += dt;
        if self.session.is_none() && self.time_s >= self.retry_at {
            self.try_connect();
        }
        if let Some(s) = self.session.as_mut() {
            s.update();
            if s.stage == Stage::Closed {
                self.error = Some("Connection lost; reconnecting".into());
                self.session = None;
                self.retry_at = self.time_s + 2.0;
            }
        }
        if let (Some(b), Some(s)) = (self.bot.as_mut(), self.session.as_mut()) {
            b.drive(s, &self.data);
        }
        self.take_visuals();
        let ships = self.ships_now();
        let phase = self.session.as_ref().and_then(|s| s.latest()).map(|s| (s.round, s.phase));
        if let Some((round, p)) = phase {
            if self.phase_seen.map(|x| (x.0, x.1)) != Some((round, p)) {
                self.phase_seen = Some((round, p, self.time_s));
                if p == Phase::Muster {
                    self.bolts.clear();
                    self.booms.clear();
                }
                println!("sc-client: round {round} {p:?}");
            }
        }
        let station = self.session.as_ref().and_then(|s| s.station());
        if let (Some(snap), Some(st), None) = (ships.as_ref().map(|x| x.2.clone()), station, self.bot.as_ref()) {
            if snap.phase == Phase::Engage {
                self.console_input(&snap, st);
            } else {
                self.pressed.clear();
            }
        } else {
            self.pressed.clear();
        }
        let aspect = f.width as f32 / f.height.max(1) as f32;
        // On foot (design 9): while this player's body is up and walking, or with the overview (V), the bridge.
        let bodies = self.bodies_now();
        let me = self.session.as_ref().and_then(|s| s.slot);
        let mine = bodies.iter().find(|(b, _)| Some(b.slot) == me).map(|x| x.0);
        let walking = mine.is_some_and(|b| b.posture != sc_core::combat::bodies::Posture::Seated);
        let shot_overview = self.args.shots.is_some() && self.shot_overview();
        let on_bridge = self.bridge.is_some() && !bodies.is_empty() && (walking || self.overview || shot_overview);
        let mut bridge_ui = None;
        if on_bridge {
            let own = walking && !self.overview && !shot_overview;
            let (eye, yaw, pitch) = match (own, mine) {
                (true, Some(b)) => (crate::drill_bridge::eye_of(&b), b.yaw, -0.12),
                _ => crate::drill_bridge::overview(),
            };
            if let Some(bv) = &self.bridge {
                let vp = bv.draw(&mut self.r, &self.target, &self.exterior, eye, yaw, pitch, aspect, &bodies, me, own);
                let (w, h) = (720.0 * aspect, 720.0);
                let crew = self.session.as_ref().map(|s| s.crew.clone()).unwrap_or_default();
                let labels = bodies
                    .iter()
                    .filter(|(b, _)| !(own && Some(b.slot) == me))
                    .filter_map(|(b, _)| {
                        let head = [b.pos[0], b.pos[1] + 2.05, b.pos[2]];
                        let at = crate::drill_bridge::project(vp, eye, head, w, h)?;
                        let name = crew.iter().find(|c| c.slot == b.slot).map(|c| c.name.clone()).unwrap_or_default();
                        let going = b.going.map(|g| format!("to {}", g.name()));
                        Some((Pos2::new(at[0], at[1]), name, going))
                    })
                    .collect();
                let banner = mine.and_then(|b| b.going).map(|g| format!("Walking to {}", g.name()));
                bridge_ui = Some(BridgeUi { labels, banner, overview: !own });
            }
        } else {
            self.render_3d(ships.as_ref(), aspect);
        }
        // The UI: build it from a snapshot of what it needs, then send what it asks for.
        let mut ask: Vec<UiAsk> = Vec::new();
        let mut view = UiView::gather(self, ships.as_ref());
        view.bridge = bridge_ui;
        let mut helm_pad = (self.helm.pad, self.helm.pad_held, self.helm.speed_set);
        let mut fire_hold = self.fire_hold;
        let now = self.time_s;
        let frame = self.ui.run(&mut self.r, f, self.time_s, |ctx| {
            draw_ui(ctx, &view, &mut ask, &mut helm_pad, &mut fire_hold, now);
        });
        self.helm.pad = helm_pad.0;
        self.helm.pad_held = helm_pad.1;
        self.helm.speed_set = helm_pad.2;
        self.fire_hold = fire_hold;
        if let Some(s) = self.session.as_mut() {
            for a in ask {
                match a {
                    UiAsk::Claim(st) => s.claim(st),
                    UiAsk::Ready(r) => s.set_ready(r),
                    UiAsk::Command(c) if self.bot.is_none() => s.command(c),
                    UiAsk::Command(_) => {}
                }
            }
        }
        self.r.present_with_ui(&self.target, f.width, f.height, true, &frame.meshes(), frame.points);
        self.scripted_shots(f);
        self.r.commit();
        if self.done {
            Flow::Done
        } else {
            Flow::Continue
        }
    }
}

/// What the UI asks the session to do.
enum UiAsk {
    Claim(Station),
    Ready(bool),
    Command(Command),
}

/// Everything the UI draws from, gathered before the UI closure borrows the renderer.
struct UiView {
    error: Option<String>,
    stage: Option<Stage>,
    server: String,
    station: Option<Station>,
    want: Station,
    bot: bool,
    ready: bool,
    crew: Vec<sc_net::msg::CrewEntry>,
    snap: Option<Snapshot>,
    tern: Option<ShipSnap>,
    hound: Option<ShipSnap>,
    rtt_ms: Option<f64>,
    loss: f64,
    refused: Option<(String, f64)>,
    face_hit: [f64; 6],
    mission: sc_core::combat::data::MissionFile,
    presets: Vec<String>,
    preset_caps: Vec<[f64; 6]>,
    load_s: f64,
    arm_s: f64,
    seeker_deg: f64,
    lock_s: f64,
    enemy_name: String,
    enemy_hull: f64,
    enemy_faces: [f64; 6],
    tern_hull: f64,
    session_now: f64,
    /// The bridge on screen instead of the consoles (design 9).
    bridge: Option<BridgeUi>,
}

/// What the bridge view's UI shows: a name over each body (and where it is going), and this player's own walk.
struct BridgeUi {
    labels: Vec<(Pos2, String, Option<String>)>,
    banner: Option<String>,
    overview: bool,
}

impl UiView {
    fn gather(app: &DrillApp, ships: Option<&(ShipSnap, ShipSnap, Snapshot)>) -> Self {
        let s = app.session.as_ref();
        let tubes = app.data.tern_combat.tubes.as_ref();
        Self {
            error: app.error.clone(),
            stage: s.map(|s| s.stage),
            server: app.args.server.clone(),
            station: s.and_then(|s| s.station()),
            want: app.args.station,
            bot: app.bot.is_some(),
            ready: s.is_some_and(|s| s.ready()),
            crew: s.map(|s| s.crew.clone()).unwrap_or_default(),
            snap: ships.map(|x| x.2.clone()),
            tern: ships.map(|x| x.0.clone()),
            hound: ships.map(|x| x.1.clone()),
            rtt_ms: s.and_then(|s| s.rtt_ms),
            loss: s.map_or(0.0, |s| s.snapshot_loss()),
            refused: s.and_then(|s| s.refused.map(|(r, t)| (r.text().to_owned(), s.now_s() - t))),
            face_hit: app.face_hit.map(|t| app.time_s - t),
            mission: app.data.mission.clone(),
            presets: app.data.tern_shields.presets.iter().map(|p| p.name.clone()).collect(),
            preset_caps: app.data.tern_shields.presets.iter().map(|p| p.faces_mj).collect(),
            load_s: tubes.map_or(1.0, |t| t.load_s),
            arm_s: tubes.map_or(1.0, |t| t.arm_s),
            seeker_deg: tubes.map_or(30.0, |t| app.data.missile(&t.missile).seeker_half_angle_deg),
            lock_s: app.data.tern_combat.lock.time_s,
            enemy_name: app.data.enemy.name.clone(),
            enemy_hull: app.data.enemy.combat.hull_mj,
            enemy_faces: app.data.enemy.shields.presets[0].faces_mj,
            tern_hull: app.data.tern_combat.hull_mj,
            session_now: s.map_or(0.0, |s| s.now_s()),
            bridge: None,
        }
    }
}

fn canvas_origin(ctx: &egui::Context) -> Pos2 {
    let w = ctx.content_rect().width();
    Pos2::new(((w - CANVAS.0) / 2.0).max(0.0), 0.0)
}

fn label(ui: &mut egui::Ui, text: &str, size: f32, colour: Color32) {
    ui.label(RichText::new(text).size(size).color(colour));
}

fn big_button(text: &str, size: Vec2, fill: Color32) -> egui::Button<'static> {
    egui::Button::new(RichText::new(text.to_owned()).size(20.0).strong().color(Color32::WHITE))
        .min_size(size)
        .corner_radius(CornerRadius::same(6))
        .fill(fill)
}

/// A fixed-size panel at `rect` (canvas points) with a title, holding `add`.
fn panel(ctx: &egui::Context, id: &str, rect: Rect, title: &str, add: impl FnOnce(&mut egui::Ui)) {
    let o = canvas_origin(ctx);
    egui::Area::new(egui::Id::new(id)).fixed_pos(rect.min + o.to_vec2()).order(egui::Order::Middle).show(ctx, |ui| {
        egui::Frame::new()
            .fill(PANEL)
            .stroke(Stroke::new(1.0_f32, EDGE))
            .corner_radius(CornerRadius::same(6))
            .inner_margin(egui::Margin::same(10))
            .show(ui, |ui| {
                let inner = rect.size() - Vec2::splat(20.0);
                ui.set_min_size(inner);
                ui.set_max_size(inner);
                if !title.is_empty() {
                    label(ui, title, 13.0, DIM);
                }
                add(ui);
            });
    });
}

/// A grid cell rectangle (bridge-stations 8.0: 12 columns of 96 lp, 4 rows of 94 lp, 8 lp gutters, from y 280).
fn cell(col: f32, row: f32, w: f32, h: f32) -> Rect {
    let (cw, rh, g) = (96.0, 94.0, 8.0);
    let x0 = (CANVAS.0 - (12.0 * cw + 11.0 * g)) / 2.0;
    Rect::from_min_size(
        Pos2::new(x0 + col * (cw + g), 280.0 + row * (rh + g)),
        Vec2::new(w * cw + (w - 1.0) * g, h * rh + (h - 1.0) * g),
    )
}

fn arc(p: &egui::Painter, c: Pos2, r: f32, a0: f32, a1: f32, stroke: Stroke) {
    let n = 24;
    let pts: Vec<Pos2> = (0..=n)
        .map(|i| {
            let a = a0 + (a1 - a0) * i as f32 / n as f32;
            c + Vec2::new(a.sin(), -a.cos()) * r
        })
        .collect();
    p.add(egui::Shape::line(pts, stroke));
}

fn bar(p: &egui::Painter, r: Rect, frac: f64, colour: Color32) {
    p.rect_filled(r, CornerRadius::same(3), Color32::from_rgb(28, 38, 50));
    let w = r.width() * frac.clamp(0.0, 1.0) as f32;
    p.rect_filled(Rect::from_min_size(r.min, Vec2::new(w, r.height())), CornerRadius::same(3), colour);
}

fn mmss(s: f64) -> String {
    let s = s.max(0.0) as u64;
    format!("{:02}:{:02}", s / 60, s % 60)
}

fn draw_ui(
    ctx: &egui::Context,
    v: &UiView,
    ask: &mut Vec<UiAsk>,
    helm: &mut ([f64; 2], bool, f64),
    fire_hold: &mut Option<(u8, f64)>,
    now: f64,
) {
    let Some(snap) = v.snap.as_ref() else {
        let text = match (&v.error, v.stage) {
            (Some(e), _) => e.to_string(),
            (None, Some(Stage::Joined)) => "Waiting for the drill".into(),
            _ => format!("Connecting to {}", v.server),
        };
        egui::Area::new(egui::Id::new("connecting")).anchor(Align2::CENTER_CENTER, Vec2::ZERO).show(ctx, |ui| {
            label(ui, "STAR CREW", 34.0, AMBER);
            label(ui, &text, 20.0, TEXT);
        });
        return;
    };
    if let Some(b) = &v.bridge {
        bridge_overlay(ctx, v, snap, b, ask);
        return;
    }
    match snap.phase {
        Phase::Muster | Phase::Countdown => briefing(ctx, v, snap, ask),
        Phase::Engage => {
            title_band(ctx, v, snap, ask);
            match v.station {
                Some(Station::Helm) => helm_console(ctx, v, ask, helm),
                Some(Station::Tactical) => tactical_console(ctx, v, ask, fire_hold, now),
                None => {
                    egui::Area::new(egui::Id::new("nostation"))
                        .anchor(Align2::CENTER_BOTTOM, Vec2::new(0.0, -80.0))
                        .show(ctx, |ui| {
                            label(ui, "Your station is held; choose one above", 20.0, AMBER);
                        });
                }
            }
            status_strip(ctx, v);
            target_bracket(ctx, v);
            if v.hound.as_ref().is_some_and(|h| h.active && h.alive) {
                let o = canvas_origin(ctx);
                let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::new("inset")));
                let r = Rect::from_min_size(o + Vec2::new(16.0, 44.0), Vec2::new(384.0, 216.0));
                painter.rect_stroke(r, CornerRadius::same(4), Stroke::new(1.0_f32, EDGE), StrokeKind::Outside);
                painter.text(
                    r.min + Vec2::new(8.0, 6.0),
                    Align2::LEFT_TOP,
                    "TARGET CAM",
                    FontId::proportional(12.0),
                    DIM,
                );
            }
        }
        Phase::Debrief => debrief(ctx, v, snap),
    }
}

/// The bridge on screen (design 9): the title band, a name over every body with where it is walking, this player's
/// own walk said once in the middle, the status strip.
fn bridge_overlay(ctx: &egui::Context, v: &UiView, snap: &Snapshot, b: &BridgeUi, ask: &mut Vec<UiAsk>) {
    title_band(ctx, v, snap, ask);
    let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Middle, egui::Id::new("bridge-names")));
    for (at, name, going) in &b.labels {
        painter.text(*at, Align2::CENTER_BOTTOM, name, FontId::proportional(17.0), TEXT);
        if let Some(g) = going {
            painter.text(*at + Vec2::new(0.0, 18.0), Align2::CENTER_BOTTOM, g, FontId::proportional(14.0), AMBER);
        }
    }
    let w = ctx.content_rect().width();
    if let Some(t) = &b.banner {
        painter.text(Pos2::new(w / 2.0, 560.0), Align2::CENTER_CENTER, t, FontId::proportional(30.0), AMBER);
    }
    if b.overview {
        painter.text(Pos2::new(w / 2.0, 640.0), Align2::CENTER_CENTER, "BRIDGE (V)", FontId::proportional(14.0), DIM);
    }
    status_strip(ctx, v);
}

fn briefing(ctx: &egui::Context, v: &UiView, snap: &Snapshot, ask: &mut Vec<UiAsk>) {
    let rect = Rect::from_min_size(Pos2::new(190.0, 70.0), Vec2::new(900.0, 580.0));
    panel(ctx, "briefing", rect, "BRIEFING", |ui| {
        label(ui, &v.mission.title, 30.0, AMBER);
        ui.add_space(6.0);
        ui.label(RichText::new(&v.mission.situation).size(17.0).color(TEXT));
        ui.add_space(8.0);
        for o in &v.mission.objectives {
            label(ui, &format!("- {o}"), 17.0, TEXT);
        }
        ui.add_space(10.0);
        let mine = v.station.unwrap_or(v.want);
        let orders = match mine {
            Station::Helm => &v.mission.orders.helm,
            Station::Tactical => &v.mission.orders.tactical,
        };
        label(ui, &format!("{} ORDERS", mine.name().to_uppercase()), 13.0, DIM);
        ui.label(RichText::new(orders).size(18.0).color(CYAN));
        ui.add_space(12.0);
        label(ui, "CREW", 13.0, DIM);
        for s in Station::ALL {
            let who = v.crew.iter().find(|c| c.station == Some(s));
            let (name, ready) = who.map_or(("automation".to_owned(), true), |c| {
                (format!("{}{}", c.name, if c.bot { " (bot)" } else { "" }), c.ready)
            });
            ui.horizontal(|ui| {
                label(ui, &format!("{:<9}", s.name()), 18.0, DIM);
                label(ui, &name, 18.0, TEXT);
                if who.is_some() {
                    label(ui, if ready { "READY" } else { "reading" }, 16.0, if ready { GREEN } else { AMBER });
                }
            });
        }
        ui.add_space(14.0);
        ui.horizontal(|ui| {
            for s in Station::ALL {
                let held_by_other = v.crew.iter().any(|c| c.station == Some(s)) && v.station != Some(s);
                let fill =
                    if v.station == Some(s) { Color32::from_rgb(30, 90, 120) } else { Color32::from_rgb(34, 44, 58) };
                if ui
                    .add_enabled(!held_by_other && !v.bot, big_button(s.name(), Vec2::new(150.0, 44.0), fill))
                    .clicked()
                {
                    ask.push(UiAsk::Claim(s));
                }
            }
            ui.add_space(40.0);
            if snap.phase == Phase::Countdown {
                let left = (v.mission.countdown_s - snap.phase_s).ceil().max(0.0);
                label(ui, &format!("ENGAGING IN {left:.0}"), 30.0, AMBER);
            } else {
                let text = if v.ready { "READY" } else { "MARK READY" };
                let fill = if v.ready { Color32::from_rgb(40, 120, 60) } else { Color32::from_rgb(170, 100, 30) };
                if ui
                    .add_enabled(v.station.is_some() && !v.bot, big_button(text, Vec2::new(220.0, 52.0), fill))
                    .clicked()
                {
                    ask.push(UiAsk::Ready(!v.ready));
                }
            }
        });
        if v.bot {
            label(ui, "A bot plays this station", 14.0, DIM);
        }
    });
}

fn debrief(ctx: &egui::Context, v: &UiView, snap: &Snapshot) {
    let rect = Rect::from_min_size(Pos2::new(290.0, 120.0), Vec2::new(700.0, 460.0));
    let (head, colour) = match snap.outcome {
        Outcome::Victory => ("VICTORY", GREEN),
        Outcome::Defeat => ("THE TERN IS LOST", RED),
        Outcome::Withdrew => ("THE HOUND WITHDREW", AMBER),
        Outcome::None => ("DEBRIEF", TEXT),
    };
    panel(ctx, "debrief", rect, "DEBRIEF", |ui| {
        label(ui, head, 40.0, colour);
        label(ui, &v.mission.title, 18.0, DIM);
        ui.add_space(12.0);
        let st = &snap.stats;
        let pct = |h: u32, s: u32| if s == 0 { 0.0 } else { 100.0 * f64::from(h) / f64::from(s) };
        let rows = [
            ("Time", mmss(st.engage_s)),
            (
                "Turret hits",
                format!("{} of {} ({:.0} %)", st.tern_hits, st.tern_shots, pct(st.tern_hits, st.tern_shots)),
            ),
            ("Missile hits", format!("{} of {}", st.missile_hits, st.missiles_fired)),
            ("Damage dealt", format!("{:.0} MJ", st.damage_dealt_mj)),
            ("Damage taken", format!("{:.0} MJ", st.damage_taken_mj)),
            ("Hull left", format!("{:.0} of {:.0} MJ", v.tern.as_ref().map_or(0.0, |t| t.hull), v.tern_hull)),
        ];
        for (k, val) in rows {
            ui.horizontal(|ui| {
                ui.add_sized(Vec2::new(200.0, 26.0), egui::Label::new(RichText::new(k).size(18.0).color(DIM)));
                label(ui, &val, 20.0, TEXT);
            });
        }
        ui.add_space(14.0);
        label(ui, &format!("Next drill in {:.0} s", (v.mission.debrief_s - snap.phase_s).max(0.0)), 18.0, AMBER);
    });
}

fn title_band(ctx: &egui::Context, v: &UiView, snap: &Snapshot, ask: &mut Vec<UiAsk>) {
    let o = canvas_origin(ctx);
    egui::Area::new(egui::Id::new("title")).fixed_pos(o).order(egui::Order::Middle).show(ctx, |ui| {
        egui::Frame::new().fill(PANEL).inner_margin(egui::Margin::symmetric(14, 4)).show(ui, |ui| {
            ui.set_min_size(Vec2::new(CANVAS.0 - 28.0, 24.0));
            ui.set_max_size(Vec2::new(CANVAS.0 - 28.0, 24.0));
            ui.horizontal(|ui| {
                label(ui, &v.station.map_or("NO STATION".into(), |s| s.name().to_uppercase()), 20.0, AMBER);
                label(ui, &v.mission.title, 15.0, DIM);
                label(ui, &mmss(snap.phase_s), 18.0, TEXT);
                if v.bot {
                    label(ui, "BOT", 15.0, CYAN);
                }
                for s in Station::ALL {
                    if Some(s) == v.station {
                        continue;
                    }
                    let open = !v.crew.iter().any(|c| c.station == Some(s));
                    let text = format!("{} {}", s.name(), if open { "AUTO" } else { "manned" });
                    if ui
                        .add_enabled(
                            open && !v.bot,
                            egui::Button::new(RichText::new(text).size(14.0)).corner_radius(CornerRadius::same(10)),
                        )
                        .clicked()
                    {
                        ask.push(UiAsk::Claim(s));
                    }
                }
            });
        });
    });
}

fn status_strip(ctx: &egui::Context, v: &UiView) {
    let o = canvas_origin(ctx);
    let Some(t) = v.tern.as_ref() else { return };
    egui::Area::new(egui::Id::new("status")).fixed_pos(o + Vec2::new(0.0, 690.0)).order(egui::Order::Middle).show(
        ctx,
        |ui| {
            egui::Frame::new().fill(PANEL).inner_margin(egui::Margin::symmetric(14, 3)).show(ui, |ui| {
                ui.set_min_size(Vec2::new(CANVAS.0 - 28.0, 24.0));
                ui.set_max_size(Vec2::new(CANVAS.0 - 28.0, 24.0));
                ui.horizontal(|ui| {
                    label(ui, "HULL", 13.0, DIM);
                    let (r, _) = ui.allocate_exact_size(Vec2::new(160.0, 12.0), Sense::hover());
                    let frac = t.hull / v.tern_hull;
                    bar(
                        ui.painter(),
                        r,
                        frac,
                        if frac > 0.5 {
                            GREEN
                        } else if frac > 0.25 {
                            AMBER
                        } else {
                            RED
                        },
                    );
                    label(ui, "SHIELDS", 13.0, DIM);
                    let (r, _) = ui.allocate_exact_size(Vec2::new(160.0, 12.0), Sense::hover());
                    let cap: f64 = v.preset_caps.get(usize::from(t.preset)).map_or(240.0, |c| c.iter().sum());
                    bar(ui.painter(), r, t.faces.iter().sum::<f64>() / cap, CYAN);
                    ui.add_space(20.0);
                    let net = format!(
                        "{} ms  {:.1} % lost",
                        v.rtt_ms.map_or("?".into(), |r| format!("{r:.0}")),
                        v.loss * 100.0
                    );
                    label(ui, &net, 14.0, if v.loss > 0.05 { AMBER } else { DIM });
                    ui.add_space(20.0);
                    for c in &v.crew {
                        let st = c.station.map_or("-", |s| s.name());
                        label(ui, &format!("{st}: {}", c.name), 14.0, DIM);
                    }
                    if let Some((r, age)) = &v.refused {
                        if *age < 2.5 {
                            label(ui, r, 15.0, RED);
                        }
                    }
                });
            });
        },
    );
}

/// A bracket on the Hound in the look band, so a ship 6 km out is found at a glance.
fn target_bracket(ctx: &egui::Context, v: &UiView) {
    let (Some(t), Some(h)) = (v.tern.as_ref(), v.hound.as_ref()) else { return };
    if !h.active || !h.alive {
        return;
    }
    let eye = t.pos + t.rot * DVec3::new(0.0, 3.0, 32.0);
    let l = t.rot.inverse() * (h.pos - eye);
    if l.z <= 1.0 {
        return;
    }
    // The same projection as the 3D view: 62 deg vertical at the window's aspect, centre at y 152 lp.
    let screen_w = ctx.content_rect().width();
    let aspect = f64::from(screen_w / 720.0);
    let f = 1.0 / (31f64.to_radians()).tan();
    let ndc_x = -l.x / l.z * f / aspect;
    let ndc_y = l.y / l.z * f;
    let p = Pos2::new((screen_w / 2.0) * (1.0 + ndc_x as f32), 152.0 - 360.0 * ndc_y as f32);
    if p.y < 32.0 || p.y > 272.0 || p.x < 0.0 || p.x > screen_w {
        return;
    }
    let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::new("bracket")));
    let colour = if t.lock_target == Some(h.id) && t.lock_frac >= 1.0 { RED } else { AMBER };
    let s = 14.0;
    for (dx, dy) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
        let c = p + Vec2::new(dx * s, dy * s);
        p_line(&painter, c, c + Vec2::new(-dx * 6.0, 0.0), colour);
        p_line(&painter, c, c + Vec2::new(0.0, -dy * 6.0), colour);
    }
    painter.text(
        p + Vec2::new(s + 6.0, -s),
        Align2::LEFT_TOP,
        format!("{:.1} km", (h.pos - t.pos).length() / 1000.0),
        FontId::proportional(14.0),
        colour,
    );
}

fn p_line(p: &egui::Painter, a: Pos2, b: Pos2, c: Color32) {
    p.line_segment([a, b], Stroke::new(2.0_f32, c));
}

/// Euler angles for the ORIENT panel: heading (deg, 0 along +Z, positive to port), pitch and roll.
fn euler(q: DQuat) -> (f64, f64, f64) {
    let f = q * DVec3::Z;
    let port = q * DVec3::X;
    let dorsal = q * DVec3::Y;
    (f.x.atan2(f.z).to_degrees(), f.y.clamp(-1.0, 1.0).asin().to_degrees(), port.y.atan2(dorsal.y).to_degrees())
}

/// A contact's place on a scanner: top-down, bow up, starboard right.
fn scanner_point(c: Pos2, scale: f32, local: DVec3) -> (Pos2, Pos2) {
    let base = c + Vec2::new(-local.x as f32, -local.z as f32) * scale;
    (base, base + Vec2::new(0.0, -local.y as f32 * scale * 0.6))
}

fn helm_console(ctx: &egui::Context, v: &UiView, ask: &mut Vec<UiAsk>, helm: &mut ([f64; 2], bool, f64)) {
    let Some(t) = v.tern.as_ref() else { return };
    let speed = t.vel.dot(t.rot * DVec3::Z);
    panel(ctx, "thrust", cell(0.0, 0.0, 3.0, 4.0), "THRUST", |ui| {
        // The set point the ship holds (the server's); the lever's drag changes what this console sends.
        label(ui, &format!("{:.0} m/s", t.speed_set), 30.0, AMBER);
        label(ui, &format!("now {speed:.0}"), 15.0, DIM);
        let (r, resp) = ui
            .allocate_exact_size(Vec2::new(ui.available_width(), ui.available_height() - 8.0), Sense::click_and_drag());
        let p = ui.painter();
        let track = Rect::from_center_size(r.center(), Vec2::new(46.0, r.height() - 16.0));
        p.rect_filled(track, CornerRadius::same(6), Color32::from_rgb(22, 30, 42));
        let y_of = |s: f64| track.bottom() - ((s + 100.0) / 500.0) as f32 * track.height();
        for m in [-100.0, 0.0, 100.0, 200.0, 300.0, 400.0] {
            let y = y_of(m);
            p.line_segment([Pos2::new(track.left() - 8.0, y), Pos2::new(track.left(), y)], Stroke::new(1.0_f32, DIM));
            p.text(
                Pos2::new(track.left() - 12.0, y),
                Align2::RIGHT_CENTER,
                format!("{m:.0}"),
                FontId::proportional(12.0),
                DIM,
            );
        }
        let set_y = y_of(t.speed_set);
        p.rect_filled(
            Rect::from_min_max(
                Pos2::new(track.left(), set_y.min(y_of(0.0))),
                Pos2::new(track.right(), set_y.max(y_of(0.0))),
            ),
            CornerRadius::same(4),
            Color32::from_rgba_unmultiplied(242, 160, 70, 90),
        );
        p.rect_filled(
            Rect::from_center_size(Pos2::new(track.center().x, set_y), Vec2::new(64.0, 10.0)),
            CornerRadius::same(3),
            AMBER,
        );
        let now_y = y_of(speed);
        p.add(egui::Shape::convex_polygon(
            vec![
                Pos2::new(track.right() + 4.0, now_y),
                Pos2::new(track.right() + 16.0, now_y - 7.0),
                Pos2::new(track.right() + 16.0, now_y + 7.0),
            ],
            CYAN,
            Stroke::NONE,
        ));
        if resp.dragged() || resp.clicked() {
            if let Some(pos) = resp.interact_pointer_pos() {
                let s = (track.bottom() - pos.y) / track.height() * 500.0 - 100.0;
                helm.2 = ((f64::from(s) / 5.0).round() * 5.0).clamp(-100.0, 400.0);
            }
        }
    });
    panel(ctx, "scanner", cell(3.0, 0.0, 5.0, 4.0), "SCANNER", |ui| {
        let (r, _) = ui.allocate_exact_size(ui.available_size(), Sense::hover());
        let p = ui.painter();
        let c = r.center();
        let rad = r.height().min(r.width()) / 2.0 - 8.0;
        let range_m = 5000.0;
        let scale = rad / range_m as f32;
        for k in [1000.0, 2500.0, 5000.0] {
            p.circle_stroke(c, k as f32 * scale, Stroke::new(1.0_f32, EDGE));
        }
        // The seeker's cone off the bow (the tubes' ghost).
        let half = (v.seeker_deg as f32).to_radians();
        for s in [-1.0f32, 1.0] {
            p.line_segment(
                [c, c + Vec2::new((half * s).sin(), -(half * s).cos()) * rad],
                Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(88, 200, 232, 110)),
            );
        }
        p.add(egui::Shape::convex_polygon(
            vec![c + Vec2::new(0.0, -10.0), c + Vec2::new(7.0, 8.0), c + Vec2::new(-7.0, 8.0)],
            CYAN,
            Stroke::NONE,
        ));
        if let Some(h) = v.hound.as_ref().filter(|h| h.active && h.alive) {
            let l = t.rot.inverse() * (h.pos - t.pos);
            let d = l.length();
            let l = if d > range_m { l * (range_m / d) } else { l };
            let (base, dot) = scanner_point(c, scale, l);
            p.line_segment(
                [base, dot],
                Stroke::new(1.5_f32, if l.y >= 0.0 { RED } else { Color32::from_rgb(150, 60, 50) }),
            );
            p.circle_filled(dot, 6.0, RED);
            p.text(
                dot + Vec2::new(10.0, -8.0),
                Align2::LEFT_BOTTOM,
                format!("{:.1} km {}", d / 1000.0, if l.y >= 0.0 { "above" } else { "below" }),
                FontId::proportional(15.0),
                TEXT,
            );
        }
    });
    panel(ctx, "attitude", cell(8.0, 0.0, 4.0, 2.0), "ATTITUDE", |ui| {
        let (r, resp) =
            ui.allocate_exact_size(Vec2::new(ui.available_width(), ui.available_height()), Sense::click_and_drag());
        let p = ui.painter();
        let pad = Rect::from_center_size(r.center(), Vec2::splat(r.height().min(r.width()) - 4.0));
        p.rect_stroke(pad, CornerRadius::same(6), Stroke::new(1.0_f32, EDGE), StrokeKind::Inside);
        p.line_segment([pad.center_top(), pad.center_bottom()], Stroke::new(1.0_f32, EDGE));
        p.line_segment([pad.left_center(), pad.right_center()], Stroke::new(1.0_f32, EDGE));
        if resp.is_pointer_button_down_on() {
            if let Some(pos) = resp.interact_pointer_pos() {
                let d = (pos - pad.center()) / (pad.width() / 2.0);
                helm.0 = [f64::from(-d.x).clamp(-1.0, 1.0), f64::from(-d.y).clamp(-1.0, 1.0)];
                helm.1 = true;
            }
        } else {
            helm.0 = [0.0, 0.0];
            helm.1 = false;
        }
        let lim = 18f64.to_radians();
        let rates = Vec2::new((-t.rates.y / lim) as f32, (t.rates.x / lim) as f32);
        p.circle_filled(pad.center() + rates * (pad.width() / 2.0), 7.0, CYAN);
        if helm.1 {
            p.circle_stroke(
                pad.center() + Vec2::new(-helm.0[0] as f32, -helm.0[1] as f32) * (pad.width() / 2.0),
                10.0,
                Stroke::new(2.0_f32, AMBER),
            );
        }
    });
    panel(ctx, "orient", cell(8.0, 2.0, 4.0, 2.0), "ORIENT", |ui| {
        let (hd, pt, rl) = euler(t.rot);
        label(ui, &format!("HDG {hd:+04.0}  PIT {pt:+03.0}  ROL {rl:+04.0}"), 18.0, TEXT);
        label(ui, &format!("q {:+.3} {:+.3} {:+.3} {:+.3}", t.rot.w, t.rot.x, t.rot.y, t.rot.z), 13.0, DIM);
        ui.horizontal(|ui| {
            let on = |m: HelmMode| {
                if t.helm_mode == m {
                    Color32::from_rgb(30, 90, 120)
                } else {
                    Color32::from_rgb(34, 44, 58)
                }
            };
            if ui.add(big_button("TARGET", Vec2::new(110.0, 44.0), on(HelmMode::Target))).clicked() {
                ask.push(UiAsk::Command(Command::Helm(HelmMode::Target)));
            }
            if ui.add(big_button("LEVEL", Vec2::new(100.0, 44.0), on(HelmMode::Level))).clicked() {
                ask.push(UiAsk::Command(Command::Helm(HelmMode::Level)));
            }
            if ui.add(big_button("STOP", Vec2::new(120.0, 52.0), Color32::from_rgb(150, 50, 40))).clicked() {
                helm.2 = 0.0;
                ask.push(UiAsk::Command(Command::AllStop));
            }
        });
    });
}

fn tactical_console(
    ctx: &egui::Context,
    v: &UiView,
    ask: &mut Vec<UiAsk>,
    fire_hold: &mut Option<(u8, f64)>,
    now: f64,
) {
    let Some(t) = v.tern.as_ref() else { return };
    let hound = v.hound.as_ref().filter(|h| h.active && h.alive);
    let locked = t.lock_target == Some(ENEMY_ID) && t.lock_frac >= 1.0;
    panel(ctx, "targets", cell(0.0, 0.0, 3.0, 4.0), "TARGETS", |ui| {
        let Some(h) = hound else {
            label(ui, "No contact", 18.0, DIM);
            return;
        };
        label(ui, &v.enemy_name, 18.0, TEXT);
        label(ui, &format!("{:.2} km", (h.pos - t.pos).length() / 1000.0), 26.0, AMBER);
        let (r, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 120.0), Sense::hover());
        let p = ui.painter();
        let c = r.center();
        p.circle_stroke(c, 46.0, Stroke::new(6.0_f32, Color32::from_rgb(28, 38, 50)));
        arc(
            p,
            c,
            46.0,
            0.0,
            std::f32::consts::TAU * t.lock_frac as f32,
            Stroke::new(6.0_f32, if locked { RED } else { AMBER }),
        );
        p.text(
            c,
            Align2::CENTER_CENTER,
            if locked {
                "LOCK"
            } else if t.lock_target.is_some() {
                "..."
            } else {
                ""
            },
            FontId::proportional(18.0),
            TEXT,
        );
        label(ui, "HULL", 12.0, DIM);
        let (r, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 12.0), Sense::hover());
        bar(ui.painter(), r, h.hull / v.enemy_hull, RED);
        label(ui, "SHIELDS", 12.0, DIM);
        let (r, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 12.0), Sense::hover());
        bar(ui.painter(), r, h.faces.iter().sum::<f64>() / v.enemy_faces.iter().sum::<f64>().max(1.0), CYAN);
        ui.add_space(6.0);
        if ui
            .add(big_button(
                if t.lock_target.is_some() { "RELOCK" } else { "LOCK" },
                Vec2::new(ui.available_width(), 48.0),
                Color32::from_rgb(150, 70, 30),
            ))
            .clicked()
        {
            ask.push(UiAsk::Command(Command::Lock(Some(ENEMY_ID))));
        }
    });
    panel(ctx, "plot", cell(3.0, 0.0, 5.0, 4.0), "SHIELDS", |ui| {
        let (r, _) =
            ui.allocate_exact_size(Vec2::new(ui.available_width(), ui.available_height() - 52.0), Sense::hover());
        let p = ui.painter();
        let c = r.center();
        let rad = r.height().min(r.width()) / 2.0 - 14.0;
        let caps = v.preset_caps.get(usize::from(t.preset)).copied().unwrap_or([40.0; 6]);
        // Faces around the ship seen from above, bow up: bow, starboard (right), stern, port (left).
        let quads = [
            (0usize, 0.0f32),
            (3, std::f32::consts::FRAC_PI_2),
            (1, std::f32::consts::PI),
            (2, -std::f32::consts::FRAC_PI_2),
        ];
        for (face, centre) in quads {
            let frac = (t.faces[face] / caps[face].max(1.0)) as f32;
            let hit = v.face_hit[face] < 0.4;
            let colour = if hit {
                Color32::WHITE
            } else if frac < 0.25 {
                RED
            } else {
                CYAN
            };
            arc(p, c, rad, centre - 0.7, centre + 0.7, Stroke::new(3.0_f32, EDGE));
            arc(p, c, rad, centre - 0.7, centre - 0.7 + 1.4 * frac, Stroke::new(9.0_f32, colour));
        }
        for (k, face) in [(0.0f32, 4usize), (1.0, 5)] {
            let frac = t.faces[face] / caps[face].max(1.0);
            let br = Rect::from_min_size(
                Pos2::new(r.right() - 16.0, r.top() + 10.0 + k * (r.height() / 2.0)),
                Vec2::new(10.0, r.height() / 2.0 - 20.0),
            );
            p.rect_filled(br, CornerRadius::same(3), Color32::from_rgb(28, 38, 50));
            let h = br.height() * frac.clamp(0.0, 1.0) as f32;
            p.rect_filled(
                Rect::from_min_max(Pos2::new(br.left(), br.bottom() - h), br.max),
                CornerRadius::same(3),
                if v.face_hit[face] < 0.4 { Color32::WHITE } else { CYAN },
            );
        }
        p.add(egui::Shape::convex_polygon(
            vec![c + Vec2::new(0.0, -16.0), c + Vec2::new(10.0, 14.0), c + Vec2::new(-10.0, 14.0)],
            TEXT,
            Stroke::NONE,
        ));
        if let Some(h) = hound {
            let l = t.rot.inverse() * (h.pos - t.pos);
            let dir = Vec2::new(-l.x as f32, -l.z as f32).normalized();
            p.circle_filled(c + dir * (rad + 12.0), 7.0, RED);
        }
        ui.horizontal(|ui| {
            for (i, name) in v.presets.iter().enumerate() {
                let on = usize::from(t.preset) == i;
                let fill = if on { Color32::from_rgb(30, 90, 120) } else { Color32::from_rgb(34, 44, 58) };
                if ui
                    .add(egui::Button::new(RichText::new(name).size(15.0)).min_size(Vec2::new(78.0, 34.0)).fill(fill))
                    .clicked()
                {
                    ask.push(UiAsk::Command(Command::Preset(i as u8)));
                }
            }
        });
    });
    panel(ctx, "turrets", cell(8.0, 0.0, 4.0, 2.0), "TURRETS", |ui| {
        ui.horizontal(|ui| {
            for tu in &t.turrets {
                let (r, resp) = ui.allocate_exact_size(Vec2::new(54.0, 54.0), Sense::hover());
                let p = ui.painter();
                let c = r.center();
                p.circle_stroke(c, 22.0, Stroke::new(3.0_f32, Color32::from_rgb(28, 38, 50)));
                arc(
                    p,
                    c,
                    22.0,
                    0.0,
                    std::f32::consts::TAU * tu.charge as f32,
                    Stroke::new(3.0_f32, if tu.charge < 0.15 { AMBER } else { CYAN }),
                );
                let l = t.rot.inverse() * tu.aim;
                let d = Vec2::new(-l.x as f32, -l.z as f32);
                let d = if d.length() > 0.01 { d.normalized() } else { Vec2::new(0.0, -1.0) };
                let firing = t.weapons_free && locked && tu.bearing;
                p.line_segment(
                    [c, c + d * 18.0],
                    Stroke::new(
                        3.0_f32,
                        if firing {
                            GREEN
                        } else if tu.bearing {
                            TEXT
                        } else {
                            DIM
                        },
                    ),
                );
                resp.on_hover_text(format!(
                    "hit {:.0} %, charge {:.0} %, {}",
                    tu.hit_chance * 100.0,
                    tu.charge * 100.0,
                    if tu.bearing { "bearing" } else { "blind" }
                ));
            }
        });
        let text = if t.weapons_free { "WEAPONS FREE" } else { "HOLD FIRE" };
        let fill = if t.weapons_free { Color32::from_rgb(40, 120, 60) } else { Color32::from_rgb(110, 60, 40) };
        if ui.add(big_button(text, Vec2::new(ui.available_width(), 44.0), fill)).clicked() {
            ask.push(UiAsk::Command(Command::WeaponsFree(!t.weapons_free)));
        }
    });
    panel(ctx, "tubes", cell(8.0, 2.0, 4.0, 2.0), "TUBES", |ui| {
        ui.horizontal(|ui| {
            for (state, timer) in &t.tubes {
                let (r, _) = ui.allocate_exact_size(Vec2::new(86.0, 26.0), Sense::hover());
                let (frac, colour, text) = match state {
                    TubeState::Empty => (0.0, DIM, "EMPTY"),
                    TubeState::Loading => (1.0 - timer / v.load_s.max(0.1), AMBER, "LOAD"),
                    TubeState::Arming => (1.0 - timer / v.arm_s.max(0.1), AMBER, "ARM"),
                    TubeState::Armed => (1.0, GREEN, "ARMED"),
                };
                bar(ui.painter(), r, frac, colour);
                ui.painter().text(r.center(), Align2::CENTER_CENTER, text, FontId::proportional(13.0), Color32::BLACK);
            }
            label(ui, &format!("{} left", t.magazine), 14.0, DIM);
        });
        let off_bow = hound.map(|h| sc_core::combat::off_bow_deg(t.pos, t.rot, h.pos));
        let in_cone = off_bow.is_some_and(|d| d <= v.seeker_deg);
        let armed = t.tubes.iter().position(|x| x.0 == TubeState::Armed);
        ui.horizontal(|ui| {
            let empty = t.tubes.iter().position(|x| x.0 == TubeState::Empty);
            if ui
                .add_enabled(
                    empty.is_some() && t.magazine > 0,
                    big_button("LOAD", Vec2::new(90.0, 44.0), Color32::from_rgb(34, 44, 58)),
                )
                .clicked()
            {
                if let Some(i) = empty {
                    ask.push(UiAsk::Command(Command::Load(i as u8)));
                }
            }
            let ready = armed.is_some() && locked && in_cone;
            let fill = if ready { Color32::from_rgb(180, 40, 30) } else { Color32::from_rgb(70, 40, 40) };
            let resp = ui.add_enabled(armed.is_some(), big_button("FIRE", Vec2::new(150.0, 52.0), fill));
            // Guarded: FIRE is held 0.6 s (bridge-stations 8.0).
            if resp.is_pointer_button_down_on() {
                match *fire_hold {
                    None => *fire_hold = armed.map(|i| (i as u8, now)),
                    Some((i, t0)) if now - t0 >= 0.6 => {
                        ask.push(UiAsk::Command(Command::Fire(i)));
                        *fire_hold = Some((i, f64::INFINITY));
                    }
                    _ => {}
                }
            } else if fire_hold.is_some_and(|(_, t0)| t0.is_infinite() || now - t0 < 0.6) {
                *fire_hold = None;
            }
            let why = if armed.is_none() {
                ""
            } else if !locked {
                "no lock"
            } else if !in_cone {
                "off bow"
            } else {
                "ready"
            };
            label(ui, why, 14.0, if ready { GREEN } else { AMBER });
        });
    });
    let _ = v.lock_s;
    let _ = v.session_now;
}
