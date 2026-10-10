//! The co-op drill in the client (openspec/changes/coop-drill design 5): connect to a drill server, read the
//! briefing, sit at Helm or Tactical, then the debrief. The station's console is its approved mockup's
//! (`console`, openspec/changes/console-parity), its viewscreen showing the 3D feed in which the Hound is a
//! placeholder model. `--bot` lets the station's automation play the seat at a player's competence, through the
//! same commands the console sends.
//!
//! It is its own `App` beside the walkable ship because the drill is a separate game mode: nothing here walks a
//! deck. Every rule it shows comes from `sc-core` (the hit chance, the lock, the tube timings), every number from
//! the snapshot; the client decides nothing a console could not.

use std::collections::HashSet;
use std::net::{SocketAddr, ToSocketAddrs};
use std::path::PathBuf;

use egui::{Align2, Color32, CornerRadius, Pos2, Rect, RichText, Stroke, Vec2};
use glam::{DQuat, DVec3, Mat4, Quat, Vec3};
use sc_client::platform::{keys, App, Event, Flow, Frame};
use sc_core::combat::data::{DrillData, DRILL_FILES};
use sc_core::combat::{Command, Outcome, Phase, Station, ENEMY_ID, TERN_ID};
use sc_core::exterior::{normalized, ExteriorData};
use sc_net::bot::Bot;
use sc_net::msg::{ShipSnap, Snapshot};
use sc_net::session::{Session, Stage, Visual, INTERP_DELAY_S};
use sc_net::transport::Impair;
use sc_render::{DeckParams, DeckProgram, Indices, Mesh, Renderer, Target};

use crate::console;
use crate::seat::{Input, PointerEv, Seat, SeatAsk};
use crate::ships3d;
use crate::ui::Ui;

/// The viewscreen's feed, pixels: twice its 614 x 194 layout points, so it stays sharp up to 1440p.
const FEED: (u32, u32) = (1228, 388);
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

/// The drill client.
pub struct DrillApp {
    r: Renderer,
    /// A black backdrop under the console (the output pass blits one).
    blank: Target,
    /// The viewscreen's feed.
    feed: Target,
    ui: Ui,
    exterior: ExteriorData,
    data: DrillData,
    meshes: Meshes,
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
    seat: Seat,
    /// The pointer's events since the last frame, for the console.
    pointer: Vec<PointerEv>,
    /// F3: the link's round trip and loss in a corner (debug numbers are not console content).
    show_net: bool,
    shots_taken: HashSet<String>,
    phase_seen: Option<(u32, Phase, f64)>,
    shot_round: Option<u32>,
    done: bool,
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
        let blank = r.make_target(16, 16, 1);
        let feed = r.make_target(FEED.0, FEED.1, 4);
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
            blank,
            feed,
            ui: Ui::new(),
            exterior,
            data,
            meshes,
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
            seat: Seat::default(),
            pointer: Vec::new(),
            show_net: false,
            shots_taken: HashSet::new(),
            phase_seen: None,
            shot_round: None,
            done: false,
        };
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
        let mut tern_hits = Vec::new();
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
                        let vel = self.bolts.iter().find(|b| b.id == bolt).map_or(DVec3::Z, |b| b.vel);
                        tern_hits.push((face, vel));
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
        if let Some(tern) = self.session.as_ref().and_then(|s| s.latest()).and_then(|s| s.ships.first().cloned()) {
            for (face, vel) in tern_hits {
                self.seat.hit(&tern, face, vel, now);
            }
        }
        let render_t = now - INTERP_DELAY_S;
        self.bolts.retain(|b| render_t - b.t0 < b.life_s && b.hit_at.is_none_or(|h| render_t < h));
        self.booms.retain(|b| render_t - b.t0 < b.dur);
    }

    /// The camera on the Tern's bow: its eye in the system frame and its attitude.
    fn bow_eye(ships: Option<&(ShipSnap, ShipSnap, Snapshot)>, time_s: f64) -> (DVec3, DQuat) {
        match ships {
            Some((t, _, _)) => (t.pos + t.rot * DVec3::new(0.0, 3.0, 32.0), t.rot),
            None => (DVec3::ZERO, DQuat::from_rotation_y(time_s * 0.02)),
        }
    }

    /// The viewscreen's feed: from the bow, looking where the console's feed points (the mockup's feed camera:
    /// 70 degrees across at zoom 1), into its own target, which the console draws in the viewscreen's frame.
    fn render_feed(&mut self, ships: Option<&(ShipSnap, ShipSnap, Snapshot)>, view: Option<&console::ConsoleView>) {
        let (eye, rot) = Self::bow_eye(ships, self.time_s);
        let (fwd, up, vfov) = match view {
            Some(v) => self.seat.feed_camera(v, rot),
            None => (rot * DVec3::Z, rot * DVec3::Y, 2.0 * (97.0f64 / (307.0 / 35f64.to_radians().tan())).atan()),
        };
        let proj =
            glam::camera::rh::proj::opengl::perspective(vfov as f32, FEED.0 as f32 / FEED.1 as f32, 1.0, 80_000.0);
        let view_m = glam::camera::rh::view::look_to_mat4(Vec3::ZERO, fwd.as_vec3(), up.as_vec3());
        let world = World {
            meshes: &self.meshes,
            exterior: &self.exterior,
            bolts: &self.bolts,
            booms: &self.booms,
            time_s: self.time_s,
        };
        self.r.begin_3d(&self.feed, [0.0, 0.0, 0.0, 1.0]);
        world.draw(&mut self.r, &self.feed, proj, view_m, vfov as f32 / FEED.1 as f32, eye, ships);
        self.r.end_pass();
        self.r.begin_3d(&self.blank, [0.0, 0.0, 0.0, 1.0]);
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
            Event::KeyDown(keys::F3) => self.show_net = !self.show_net,
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
        let crew = self.session.as_ref().map(|s| s.crew.clone()).unwrap_or_default();
        // The console's view: the station this client holds (or wants), from the newest interpolated snapshot.
        let console_view = ships.as_ref().map(|(t, h, snap)| {
            let st = station.unwrap_or(self.args.station);
            let op =
                crew.iter()
                    .find(|c| c.station == Some(st))
                    .map(|c| if c.bot { "Bot".to_owned() } else { c.name.clone() });
            let open = Station::ALL
                .iter()
                .filter(|o| **o != st && !crew.iter().any(|c| c.station == Some(**o)))
                .map(|o| o.id().to_owned())
                .collect();
            self.seat.view(st, t, h, snap, &self.data, op, open, self.time_s)
        });
        self.render_feed(ships.as_ref(), console_view.as_ref());
        let mut ask: Vec<UiAsk> = Vec::new();
        let view = UiView::gather(self, ships.as_ref());
        let mut painted: Option<(console::kit::Hits, f32)> = None;
        let mut pointer: Vec<PointerEv> = Vec::new();
        let show_net = self.show_net;
        let frame = self.ui.run(&mut self.r, f, self.time_s, |ctx| {
            if let Some(v) = console_view.as_ref() {
                let hover = ctx.input(|i| i.pointer.hover_pos());
                painted = Some(console::paint(ctx, v, true, hover));
                ctx.input(|i| {
                    for e in &i.events {
                        match e {
                            egui::Event::PointerButton {
                                pos, button: egui::PointerButton::Primary, pressed, ..
                            } => pointer.push(if *pressed { PointerEv::Press(*pos) } else { PointerEv::Release(*pos) }),
                            egui::Event::PointerMoved(p) => pointer.push(PointerEv::Move(*p)),
                            egui::Event::MouseWheel { delta, .. } if delta.y != 0.0 => {
                                if let Some(p) = i.pointer.hover_pos() {
                                    pointer.push(PointerEv::Scroll(p, delta.y > 0.0));
                                }
                            }
                            _ => {}
                        }
                    }
                });
            }
            draw_ui(ctx, &view, &mut ask, show_net);
        });
        self.pointer = pointer;
        // The seated player's hands on the console, in Engage only (the briefing and debrief sit over it otherwise).
        let engage = ships.as_ref().is_some_and(|x| x.2.phase == Phase::Engage);
        let pressed = std::mem::take(&mut self.pressed);
        if let (true, Some(st), None, Some((hits, scale)), Some(v), Some((t, h, _))) =
            (engage, station, self.bot.as_ref(), painted.as_ref(), console_view.as_ref(), ships.as_ref())
        {
            let input = Input { events: std::mem::take(&mut self.pointer), held: self.held.clone(), pressed };
            for a in self.seat.input(&input, hits, *scale, v, t, h, &self.data, st, self.time_s) {
                match a {
                    SeatAsk::Command(c) => ask.push(UiAsk::Command(c)),
                    SeatAsk::Claim(s) => ask.push(UiAsk::Claim(s)),
                }
            }
        }
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
        // The viewscreen's feed in its window under the console, or black when there is no console yet.
        let ppp = f.height as f32 / frame.points[1].max(1.0);
        match console_view {
            Some(_) => {
                let at = console::viewscreen_px(frame.points, ppp);
                self.r.present_with_ui_at(&self.feed, Some(at), f.width, f.height, true, &frame.meshes(), frame.points)
            }
            None => self.r.present_with_ui(&self.blank, f.width, f.height, true, &frame.meshes(), frame.points),
        }
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

/// Everything the briefing, the debrief and the F3 corner draw from, gathered before the UI closure borrows the
/// renderer.
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
    rtt_ms: Option<f64>,
    loss: f64,
    refused: Option<(String, f64)>,
    mission: sc_core::combat::data::MissionFile,
    tern_hull: f64,
}

impl UiView {
    fn gather(app: &DrillApp, ships: Option<&(ShipSnap, ShipSnap, Snapshot)>) -> Self {
        let s = app.session.as_ref();
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
            rtt_ms: s.and_then(|s| s.rtt_ms),
            loss: s.map_or(0.0, |s| s.snapshot_loss()),
            refused: s.and_then(|s| s.refused.map(|(r, t)| (r.text().to_owned(), s.now_s() - t))),
            mission: app.data.mission.clone(),
            tern_hull: app.data.tern_combat.hull_mj,
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

fn mmss(s: f64) -> String {
    let s = s.max(0.0) as u64;
    format!("{:02}:{:02}", s / 60, s % 60)
}

fn draw_ui(ctx: &egui::Context, v: &UiView, ask: &mut Vec<UiAsk>, show_net: bool) {
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
    match snap.phase {
        Phase::Muster | Phase::Countdown => briefing(ctx, v, snap, ask),
        Phase::Engage => {
            // A refused command says why, over the status strip, for a few seconds.
            if let Some((why, age)) = v.refused.as_ref().filter(|r| r.1 < 3.0) {
                let _ = age;
                egui::Area::new(egui::Id::new("refused")).anchor(Align2::CENTER_BOTTOM, Vec2::new(0.0, -40.0)).show(
                    ctx,
                    |ui| {
                        label(ui, why, 16.0, AMBER);
                    },
                );
            }
        }
        Phase::Debrief => debrief(ctx, v, snap),
    }
    if show_net {
        egui::Area::new(egui::Id::new("net")).anchor(Align2::RIGHT_TOP, Vec2::new(-10.0, 40.0)).show(ctx, |ui| {
            let rtt = v.rtt_ms.map_or("--".to_owned(), |r| format!("{r:.0} ms"));
            label(ui, &format!("RTT {rtt}  LOSS {:.1}%", v.loss * 100.0), 13.0, DIM);
        });
    }
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
