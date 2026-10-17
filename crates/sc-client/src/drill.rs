//! The co-op drill in the client (openspec/changes/coop-drill design 5): connect to a drill server, read the
//! briefing, sit at Helm or Tactical, then the debrief. The station's console is its approved mockup's
//! (`console`, openspec/changes/console-parity), its viewscreen showing the 3D feed in which the Hound is a
//! placeholder model. `--bot` lets the station's automation play the seat at a player's competence, through the
//! same commands the console sends.
//!
//! It is its own `App` beside the walkable ship because the drill is a separate game mode: nothing here walks a
//! deck. Every rule it shows comes from `sc-core` (the hit chance, the lock, the tube timings), every number from
//! the snapshot; the client decides nothing a console could not.
//!
//! Single player (openspec/changes/single-player) is the same drill with its server inside the client and an NPC at
//! every other station ([`crate::solo`]): the player can show any crewed station's console from their seat, and play
//! one on MANUAL through its NPC's session.

use std::collections::HashSet;
use std::net::{SocketAddr, ToSocketAddrs};
use std::path::PathBuf;

use egui::{Align2, Color32, Pos2, Rect, RichText, Vec2};
use glam::{DQuat, DVec3, Mat4, Quat, Vec3};
use sc_client::platform::{keys, App, Event, Flow, Frame, TouchPhase};
use sc_core::combat::data::{DrillData, DRILL_FILES};
use sc_core::combat::seats::{claim_rule, ClaimRule, Occupant};
use sc_core::combat::{Command, Outcome, Phase, Station, ENEMY_ID, TERN_ID};
use sc_core::exterior::{normalized, ExteriorData};
use sc_net::bot::Bot;
use sc_net::msg::{ShipSnap, Snapshot};
use sc_net::session::{Session, Stage, Visual, INTERP_DELAY_S};
use sc_net::transport::{Dial, Impair};
use sc_render::{DeckProgram, Indices, Mesh, Renderer, Target};

use crate::console;
use crate::hull::HullLight;
use crate::seat::{Input, PointerEv, Seat, SeatAsk};
use crate::ships3d;
use crate::ui::Ui;

/// The viewscreen's feed, pixels: twice its 614 x 194 layout points, so it stays sharp up to 1440p.
const FEED: (u32, u32) = (1228, 388);

const DIM: Color32 = Color32::from_rgb(128, 146, 164);
const AMBER: Color32 = Color32::from_rgb(242, 160, 70);

/// What `--connect` was given.
pub struct DrillArgs {
    /// `host` or `host:port`.
    pub server: String,
    /// The station to take.
    pub station: Station,
    /// The player asked for Any: the first of the drill's `bot_posts` (Helm) is taken instead of `station`.
    pub any: bool,
    /// The officer's name.
    pub name: String,
    /// Whether the station's automation plays it.
    pub bot: bool,
    /// Where captures go, for the scripted shots.
    pub shots: Option<PathBuf>,
    /// Join with no station and watch the bridge from its overview (`--watch`).
    pub watch: bool,
    /// Host the drill here with an NPC crew (single player, `--solo`); `server` is then unused.
    pub solo: bool,
    /// Esc goes back to the front menu (a game started from it) rather than closing the program.
    pub menu: bool,
    /// The session's seed: the NPCs' names.
    pub seed: u64,
    /// The hand-over test's folder: once the briefing shows, capture it and hand back to the menu.
    pub hand_test: Option<PathBuf>,
    /// Single player: a scripted player walks the ship, sits at stations and works them, for a recording of the game
    /// as it is played (`--playtest`, `crate::playtest`).
    pub playtest: bool,
    /// Single player with `--shots`: write every frame of a scripted run through the viewscreen's cameras, FWD,
    /// TARGET, CHASE round the ship and the bridge's main viewscreen, for a video (`--film`, chase-camera 3).
    pub film: bool,
    /// With `film`: the chase camera following the ship's turn and ORBIT round it instead (`--film-orbit`,
    /// chase-orbit).
    pub film_orbit: bool,
    /// Single player: All hands, no damage-control teams aboard; the crew put the ship right (`--all-hands`,
    /// all-hands 1).
    pub all_hands: bool,
    /// The mission to load, `data/missions/<id>.json` (`--mission`; a server's welcome names its own).
    pub mission: String,
    /// Single player: the UI's capture run (`--ui-shots DIR`, `--demo DIR`; ui-art-direction tasks 3 and 5).
    pub tour: Option<TourArgs>,
}

/// The UI's capture run (CLAUDE.md 4: a capture instrument, it decides nothing a player could not): a scripted player
/// walks to its station with the first-time hint up, stands a moment in reach, sits, reads the briefing, opens the
/// Esc menu and its pages, and watches the console into the fight. `--ui-shots` writes a named capture of each screen
/// (ui-art-direction task 5.2); `--demo` writes every frame at 30 a second for the video, each screen held a while.
#[derive(Clone, Debug)]
pub struct TourArgs {
    /// Where the captures or frames go.
    pub dir: PathBuf,
    /// Every frame, for a video (`--demo`), rather than one capture a screen.
    pub video: bool,
    /// The video's next frame number (the front menu's frames come first).
    pub frame: u64,
}

/// Where the UI's capture run is: its step, when the step began, its frames.
#[derive(Debug)]
struct Tour {
    args: TourArgs,
    step: usize,
    since: f64,
    frames: u64,
}

struct Meshes {
    hound: Mesh,
    jackal: Mesh,
    debris: Mesh,
    bolt_tern: Mesh,
    bolt_enemy: Mesh,
    missile: Mesh,
    burst: Mesh,
    beam: Mesh,
    /// The extinguisher in hand (all-hands 4; the mockups' own).
    held: Mesh,
    /// The relay at the Shoals, where the helm's COURSE flies (patrol-mission design 3): a placeholder.
    relay: crate::outside::StationView,
    /// The system's stations in a patrol (patrol-mission design 12b): Tamsin Station and the four placeholders.
    stations: Option<crate::stations::StationsView>,
}

/// A scripted shot's eye: where it stands in the ship's frame (metres), its yaw and its pitch (radians).
type ShotEye = ([f64; 3], f32, f32);

/// A thing outside drawn as a station: the relay, or a system file's station by its index there.
#[derive(Clone, Copy, Debug)]
enum Shown {
    Relay,
    Station(usize),
}

/// The stations of `data`'s patrol on the GPU, or none for a drill.
fn station_views(r: &Renderer, data: &DrillData) -> Result<Option<crate::stations::StationsView>, String> {
    let Some(p) = data.patrol.as_ref() else { return Ok(None) };
    let look = crate::stations::StationLook::load()?;
    let v = crate::stations::StationsView::new(r, &p.system, look, std::path::Path::new("."))?;
    println!("sc-client: the system's stations are {} triangles", v.triangles);
    Ok(Some(v))
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

/// A laser beam as drawn: a stretched bolt from the emitter to where it struck or passed (laser-banks 2).
struct VisBeam {
    from: DVec3,
    to: DVec3,
    /// Local time it fired.
    t0: f64,
}

/// How long a beam shows, seconds.
const BEAM_SHOW_S: f64 = 0.4;

/// A piece thrown off a burst aboard a ship (ship-damage 6): it tumbles away and fades.
struct Debris {
    pos: DVec3,
    vel: DVec3,
    spin: Vec3,
    t0: f64,
    dur: f64,
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
    /// The bridge, drawn full screen while this player's body walks or the overview is up (design 9).
    bridge_target: Target,
    ui: Ui,
    exterior: ExteriorData,
    data: DrillData,
    meshes: Meshes,
    /// The fires, smoke and agent as drawn inside (all-hands 4).
    fire_fx: crate::firefx::FireFx,
    args: DrillArgs,
    addr: Option<Dial>,
    session: Option<Session>,
    error: Option<String>,
    retry_at: f64,
    bot: Option<Bot>,
    time_s: f64,
    bolts: Vec<VisBolt>,
    beams: Vec<VisBeam>,
    booms: Vec<Boom>,
    debris: Vec<Debris>,
    held: HashSet<u32>,
    pressed: HashSet<u32>,
    /// A console's own state for each station (`Station as usize`): the player's seat, and in single player every
    /// crewed station's console they show.
    seats: Vec<Seat>,
    /// Single player: the server and the NPC crew.
    solo: Option<crate::solo::SoloHost>,
    /// Single player: the crewed station whose console is shown, or none for the player's own.
    shown: Option<Station>,
    /// Single player: READY pressed while the crew still walk to their seats; sent once they sit (design 4).
    ready_held: bool,
    /// START MISSION on this frame's console, and the drill clock (`phase_s`) it was first seen at (start-mission 4):
    /// the scripted shots capture it and press it.
    start_seen: Option<f64>,
    /// The pointer's events since the last frame, for the console.
    pointer: Vec<PointerEv>,
    /// F3: the link's round trip and loss in a corner (debug numbers are not console content).
    show_net: bool,
    shots_taken: HashSet<String>,
    phase_seen: Option<(u32, Phase, f64)>,
    shot_round: Option<u32>,
    done: bool,
    /// The film's frames written so far, and whether it is looking at the bridge's main viewscreen (chase-camera 3).
    film_frame: u32,
    film_screen: bool,
    /// The patrol's scripted shots (patrol-mission design 12b): the eye they stand at (ship frame, yaw, pitch), and the
    /// shot it is for, taken the frame after the eye is set.
    shot_eye: Option<ShotEye>,
    shot_pending: Option<String>,
    /// `--playtest` in a patrol: the scripted player's memory of what it last sent at its console (`patrol::decide`).
    playtest_patrol: sc_core::combat::patrol::PatrolMemory,
    /// The cue written last, for the video's captions (chase-camera 3).
    film_cue: String,
    /// The orbit reel's starting heading, degrees (`--film-orbit`).
    film_orbit0: Option<f64>,
    /// The bridge's main viewscreen's camera rig (chase-orbit design 7).
    main_rig: console::rig::FeedRig,
    /// The bridge, when the compiled deck is there (design 9).
    bridge: Option<crate::drill_bridge::BridgeView>,
    /// The overview of the bridge is up (V).
    overview: bool,
    /// The scripted shots: since when this player's body, and another's, has been seen walking.
    walk_seen: (Option<f64>, Option<f64>),
    /// The game menu on Esc (game-menu), and the options it sets.
    menu: crate::game_menu::GameMenu,
    options: crate::options::Options,
    /// The player's own body on foot (CLAUDE.md, the law: one game, played on foot): walked here, sent to the server.
    foot: Option<crate::onfoot::OnFoot>,
    /// The body stood free last frame: the player's to walk.
    free: bool,
    /// The eye's facing and tilt on foot, radians.
    look: (f32, f32),
    /// The mouse is captured for looking around on foot.
    captured: bool,
    /// When the walk was last sent, local seconds.
    walk_sent: f64,
    /// The stand key's hold while seated (leave-station 2).
    stand: crate::stand::StandHold,
    /// The keys that are data (`data/input/bindings.json`).
    bindings: crate::bindings::Bindings,
    /// Each station's seat (layout.json), for the seat in reach.
    seat_points: Vec<(Station, [f32; 3], f32)>,
    /// `--playtest`: the scripted player.
    playtest: Option<crate::playtest::Playtest>,
    /// The script holds the stand key, and a console control it presses next frame, where a player's pointer would.
    playtest_stand: bool,
    playtest_press: Option<crate::playtest::Control>,
    /// When the body last sat down at a console, local seconds: the console rises from the desk from then.
    seated_at: Option<f64>,
    /// The eye the player last saw the ship from on their feet (where, facing, tilt), and the one the sit started from:
    /// the camera sits down with the body from there.
    last_view: Option<([f64; 3], f32, f32)>,
    rise_from: Option<([f64; 3], f32, f32)>,
    /// The round's phase on the last snapshot seen, and when the briefing began lowering into the desk (design 18a).
    seen_phase: Option<Phase>,
    lowered_at: Option<f64>,
    /// The left button is down (looking round on foot, it sprays an extinguisher), and what was last sent of spraying.
    mouse_left: bool,
    spray_sent: bool,
    /// Repairs on deck (all-hands 5): the games' data, the repair being played at a machine, and a dock asked for (the
    /// job, and when, local seconds).
    repair_data: Option<sc_core::repair::data::RepairData>,
    deck: Option<crate::deckrepair::DeckRun>,
    dock_asked: Option<(u8, f64)>,
    /// Playing by thumb (touch-controls): the layer on foot and seated, and what it is doing this frame.
    touch: crate::touch::Layer,
    touch_mode: crate::touch::Mode,
    /// A finger on the console in Engage (the console takes fingers this frame), and the first finger as a repair
    /// game's pointer.
    console_finger: crate::touch::ConsoleFinger,
    console_touch: bool,
    touch_mouse: sc_client::platform::TouchMouse,
    /// The keys the layer held last frame (let go of before this frame's), and the stick's walk this frame.
    touch_keys: Vec<u32>,
    touch_stick: Option<(f32, f32, bool)>,
    /// The last on-foot prompt had something below to go down to: DOWN shows.
    touch_down: bool,
    /// The on-foot prompt last said (logged when it changes), and when the last refusal logged was made.
    prompt_said: Option<String>,
    refused_said: Option<f64>,
    /// The prompt plate on screen and how much of it shows: it fades in over `prompt_in` and out over `prompt_out`
    /// (ui-art-direction design 8).
    plate: Option<(crate::hud::Prompt, f32)>,
    /// The loading screen's and the briefing's art and the loading screen's tip, loaded on the first frame and kept
    /// while the game runs (design 15: the briefing shows it again each round).
    art: Option<(Option<egui::TextureHandle>, Option<crate::hud::Tip>)>,
    /// The game's clock at the first snapshot: the loading screen fades out over `load_out` from it.
    boarded_at: Option<f64>,
    /// `--ui-shots` and `--demo`: the UI's capture run.
    tour: Option<Tour>,
    /// The last frame's UI meshes, one draw call each (the capture run reports them).
    ui_meshes: usize,
}

/// The console comes up from the desk over this long once the body sits, seconds (the law: maximum immersion): the
/// style's `motion_ms.console_rise` (ui-art-direction design 8 and 18a).
fn console_rise_s() -> f64 {
    f64::from(crate::look::look().f.motion_ms.console_rise) / 1000.0
}

/// How far the console has come up `since` seconds after the body sat, on the style's curve: 0 down, 1 up.
fn console_risen(since: f64) -> f32 {
    let l = crate::look::look();
    crate::motion::eased(since, l.f.motion_ms.console_rise, l.f.motion_ease.console_rise)
}

/// The walk goes to the server this often, seconds (20 a second: other players see a body walk, not hop).
const WALK_SEND_S: f64 = 0.05;

/// The drill's data for mission `id` (`data/missions/<id>.json`).
fn load_data(id: &str) -> Result<DrillData, String> {
    let read = |rel: &str| std::fs::read_to_string(rel).map_err(|e| format!("{rel}: {e}"));
    let t: Vec<String> = DRILL_FILES.iter().map(|f| read(f)).collect::<Result<_, _>>()?;
    let arr: [&str; sc_core::combat::data::DRILL_FILE_COUNT] = std::array::from_fn(|i| t[i].as_str());
    let m = format!("data/missions/{id}.json");
    DrillData::parse(arr, &m, &read(&m)?).map_err(|e| e.to_string())
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
        let bridge_target = r.make_target(1280, 720, 4);
        let text = std::fs::read_to_string("data/space/exterior.json")
            .map_err(|e| format!("data/space/exterior.json: {e}"))?;
        let exterior: ExteriorData =
            sc_core::data::parse("data/space/exterior.json", &text).map_err(|e| e.to_string())?;
        console::set_chase(&exterior.viewscreen.chase);
        let mut data = load_data(&args.mission)?;
        // The nebula behind every view: the patrol's system's, or the home system's (openspec/changes/nebula-skies).
        match data.patrol.as_ref() {
            Some(p) => crate::outside::set_system_sky(&mut r, &p.system)?,
            None => crate::outside::set_home_sky(&mut r)?,
        }
        if args.all_hands && args.solo {
            data.mission.damage_control = "crew".into();
        }
        let hound = ships3d::hound();
        println!("sc-client: the Hound placeholder is {} triangles", hound.triangles());
        let relay = crate::outside::StationView::new(&r, ships3d::StationKind::Relay);
        println!("sc-client: the relay placeholder is {} triangles", relay.triangles);
        let meshes = Meshes {
            hound: mesh(&r, &hound),
            jackal: mesh(&r, &ships3d::jackal()),
            debris: mesh(&r, &ships3d::bolt([0.35, 0.33, 0.33])),
            bolt_tern: mesh(&r, &ships3d::bolt([0.6, 1.8, 2.0])),
            bolt_enemy: mesh(&r, &ships3d::bolt([2.0, 0.8, 0.4])),
            missile: mesh(&r, &ships3d::missile()),
            burst: mesh(&r, &ships3d::burst()),
            beam: mesh(&r, &ships3d::bolt([2.0, 0.6, 1.3])),
            held: mesh(&r, &ships3d::extinguisher()),
            relay,
            stations: station_views(&r, &data)?,
        };
        let mut args = args;
        if args.any {
            args.station = data.stations.bot_posts.stations().first().copied().unwrap_or(Station::Helm);
        }
        let solo = if args.solo {
            let names: sc_core::names::NameData = std::fs::read_to_string("data/crew/names.json")
                .map_err(|e| format!("data/crew/names.json: {e}"))
                .and_then(|t| sc_core::data::parse("data/crew/names.json", &t).map_err(|e| e.to_string()))?;
            let (_, seat) = crate::solo::roster(&data, args.station);
            args.station = seat;
            let host = crate::solo::SoloHost::start(&data, seat, &names, args.seed, crate::solo::bridge(&data))?;
            Some(host)
        } else {
            None
        };
        // Single player joins its own server over the local link; a multiplayer game dials the server's address.
        let addr = match solo.as_ref() {
            Some(h) => Some(h.dial()),
            None => {
                let host =
                    if args.server.contains(':') { args.server.clone() } else { format!("{}:7700", args.server) };
                host.to_socket_addrs().ok().and_then(|mut a| a.find(SocketAddr::is_ipv4)).map(Dial::Addr)
            }
        };
        let bot = args.bot.then(|| Bot::new(&data, args.station));
        let args_watch = args.watch;
        let touch = crate::touch::load()?;
        let bindings = crate::bindings::load()?;
        let mut app = Self {
            r,
            blank,
            feed,
            bridge_target,
            ui: Ui::new(),
            exterior,
            data,
            meshes,
            fire_fx: crate::firefx::FireFx::default(),
            args,
            addr,
            session: None,
            error: None,
            retry_at: 0.0,
            bot,
            time_s: 0.0,
            bolts: Vec::new(),
            beams: Vec::new(),
            booms: Vec::new(),
            debris: Vec::new(),
            held: HashSet::new(),
            pressed: HashSet::new(),
            seats: Station::ALL.iter().map(|_| Seat::default()).collect(),
            solo,
            shown: None,
            ready_held: false,
            start_seen: None,
            pointer: Vec::new(),
            show_net: false,
            shots_taken: HashSet::new(),
            phase_seen: None,
            shot_round: None,
            done: false,
            film_frame: 0,
            film_screen: false,
            shot_eye: None,
            shot_pending: None,
            playtest_patrol: Default::default(),
            film_cue: String::new(),
            film_orbit0: None,
            main_rig: console::rig::FeedRig::default(),
            bridge: None,
            overview: args_watch,
            walk_seen: (None, None),
            menu: Default::default(),
            options: crate::options::Options::load(),
            foot: None,
            free: false,
            look: (0.0, 0.0),
            captured: false,
            walk_sent: 0.0,
            stand: crate::stand::StandHold::default(),
            bindings,
            seat_points: Vec::new(),
            playtest: None,
            playtest_stand: false,
            playtest_press: None,
            seated_at: None,
            last_view: None,
            rise_from: None,
            seen_phase: None,
            lowered_at: None,
            mouse_left: false,
            spray_sent: false,
            repair_data: None,
            deck: None,
            dock_asked: None,
            console_finger: crate::touch::ConsoleFinger::new(touch.console.clone()),
            touch: crate::touch::Layer::new(touch),
            touch_mode: crate::touch::Mode::Off,
            console_touch: false,
            touch_mouse: Default::default(),
            touch_keys: Vec::new(),
            touch_stick: None,
            touch_down: false,
            prompt_said: None,
            plate: None,
            art: None,
            boarded_at: None,
            tour: None,
            ui_meshes: 0,
            refused_said: None,
        };
        match sc_core::repair::data::RepairData::load_for_ship(|rel| {
            std::fs::read_to_string(rel).map_err(|e| format!("{rel}: {e}"))
        }) {
            Ok(r) => app.repair_data = Some(r),
            Err(e) => println!("sc-client: no repairs on deck ({e})"),
        }
        if let (Some(t), true) = (app.args.tour.clone(), app.solo.is_some()) {
            app.playtest = Some(crate::playtest::Playtest::ui_tour(app.args.station));
            // The run shows the first-time hint whatever this machine's settings say, and keeps nothing.
            app.options.first_steps_done = false;
            app.tour = Some(Tour { frames: t.frame, args: t, step: 0, since: 0.0 });
        } else if app.args.film && app.solo.is_some() {
            app.playtest = Some(crate::playtest::Playtest::film(app.args.station));
        } else if app.args.playtest && app.solo.is_some() {
            // A patrol's script looks out of the windows as the patrol goes on (patrol-mission design 11).
            app.playtest = Some(if app.data.patrol.is_some() {
                crate::playtest::Playtest::patrol(app.args.station)
            } else {
                crate::playtest::Playtest::new(app.args.station)
            });
        }
        // Where a fire's flames stand in each room: its open floor, from the deck's own geometry.
        if let Ok(bytes) = std::fs::read("compiled/tern.deck") {
            if let Ok(d) = sc_core::deck::read(&bytes) {
                let t0 = std::time::Instant::now();
                app.fire_fx.set_floors(crate::firefx::open_floors(&d, &app.data.tern_layout.rooms));
                println!("sc-client: the rooms' open floor for fires, {:.0} ms", t0.elapsed().as_secs_f64() * 1000.0);
            }
        }
        match crate::drill_bridge::BridgeView::load(&mut app.r, std::path::Path::new("compiled/tern.deck")) {
            Ok(mut b) => {
                println!("sc-client: the bridge for the crew on foot, {} triangles", b.triangles);
                let n = b.set_brackets(&app.r, &app.data.tern_layout.brackets);
                println!(
                    "sc-client: {} extinguisher brackets round the ship, {n} triangles",
                    app.data.tern_layout.brackets.len()
                );
                b.fov_deg = app.options.fov_deg as f32;
                app.bridge = Some(b);
            }
            Err(e) => println!("sc-client: no bridge to walk ({e}); the drill shows the consoles only"),
        }
        // The player's own walk, on the same deck (a watcher has no body).
        if !app.args.watch && app.bridge.is_some() {
            let foot = std::fs::read("compiled/tern.deck").map_err(|e| e.to_string()).and_then(|b| {
                let d = sc_core::deck::read(&b).map_err(|e| e.to_string())?;
                crate::onfoot::OnFoot::from_deck(&d).map(|f| (f, d.index.walk.start.at_m))
            });
            match foot {
                Ok((f, start)) => {
                    // A fire's flames stand where a person can come to them: the walk's joined floor in each room.
                    app.fire_fx.join_floors(&f.grid, start, &app.data.tern_layout.rooms);
                    app.foot = Some(f)
                }
                Err(e) => println!("sc-client: no walk on foot ({e})"),
            }
        }
        let stations: Vec<Station> = app.data.mission.stations.iter().filter_map(|s| Station::from_id(s)).collect();
        app.seat_points = sc_server::bridge::seats(std::path::Path::new("."), &app.data, &stations).unwrap_or_default();
        if let Some(f) = app.foot.as_ref() {
            sc_server::bridge::on_floor(&mut app.seat_points, &f.world);
        }
        app.try_connect();
        Ok(app)
    }

    fn try_connect(&mut self) {
        let Some(addr) = self.addr.clone() else {
            self.error = Some(format!("Cannot find {}", self.args.server));
            return;
        };
        match Session::connect(
            addr.clone(),
            &self.args.name,
            self.args.bot,
            (!self.args.watch).then_some(self.args.station),
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

    /// The station whose console is on screen: the one held (or wanted), or in single player the crewed one shown.
    fn console_station(&self) -> Station {
        let own = self.session.as_ref().and_then(Session::station).unwrap_or(self.args.station);
        if self.solo.is_some() {
            self.shown.unwrap_or(own)
        } else {
            own
        }
    }

    /// One frame on foot (the law: one game, played on foot): the body put where the server has it when it becomes
    /// the player's, the keys' walk, E to sit at a seat in reach (else E and Q climb and work the lifts), the walk sent
    /// to the server. `empty` are the brackets standing empty. Returns what E does here, for the screen.
    fn step_foot(
        &mut self,
        dt: f64,
        me: sc_net::msg::BodySnap,
        burning: &[usize],
        jobs: &[sc_core::combat::DeckJob],
        empty: &[usize],
    ) -> Option<crate::hud::Prompt> {
        let seats = &self.data.stations.seats;
        // The use key sits (the same key stands, leave-station 2), takes and repairs.
        let use_key = self.bindings.use_key;
        let (reach, reach_dy) = (seats.reach_m as f32, seats.reach_dy_m as f32);
        // Where the server put the body as it became the player's: the newest snapshot's, not the interpolated one (which
        // trails it, and would stand the body in the chair it just got up from).
        let latest = self.session.as_ref().and_then(|s| {
            let slot = s.slot?;
            s.latest()?.bodies.iter().find(|b| b.slot == slot).copied()
        });
        let foot = self.foot.as_mut()?;
        if !self.free {
            let at = latest.unwrap_or(me);
            foot.place(at.pos);
            self.look = (at.yaw, -0.05);
        }
        let seat = sc_core::combat::bodies::seat_in_reach(&self.seat_points, foot.body.feet, reach, reach_dy);
        let menu = self.menu.open();
        let (held, mut pressed) =
            if menu { (HashSet::new(), HashSet::new()) } else { (self.held.clone(), self.pressed.clone()) };
        let mut sit = None;
        if let (Some(st), true) = (seat, pressed.contains(&use_key)) {
            pressed.remove(&use_key);
            sit = Some(st);
        }
        // The damage-control locker or a full bracket in reach: E takes an extinguisher (all-hands 4, safety-points
        // 5a; the server's rule, `Drill::take_extinguisher`). F, or the left button while looking round, held, sprays it.
        let feet = foot.body.feet;
        let layout = &self.data.tern_layout;
        let near = |l: &glam::DVec3| {
            (l.x - f64::from(feet[0])).hypot(l.z - f64::from(feet[2])) <= f64::from(reach)
                && (l.y - f64::from(feet[1])).abs() < 1.5
        };
        let bracket_near = |full: bool| {
            (0..layout.brackets.len()).any(|k| near(&layout.brackets[k].wall) && empty.contains(&k) != full)
        };
        let at_locker = layout.lockers.iter().any(near) || bracket_near(true);
        let emptied = !at_locker && bracket_near(false);
        let mut take = false;
        if at_locker && seat.is_none() && pressed.contains(&use_key) {
            pressed.remove(&use_key);
            take = true;
        }
        // A damaged machine in reach (all-hands 5): E docks at it and its repair game opens.
        let reach_job = self.repair_data.as_ref().and_then(|rd| {
            let (r, ry) = (rd.ship.reach_m, rd.ship.reach_dy_m);
            jobs.iter()
                .filter(|j| {
                    (j.at.x - f64::from(feet[0])).hypot(j.at.z - f64::from(feet[2])) <= r
                        && (j.at.y - f64::from(feet[1])).abs() <= ry
                })
                .min_by(|a, b| {
                    let d = |j: &&sc_core::combat::DeckJob| {
                        (j.at.x - f64::from(feet[0])).hypot(j.at.z - f64::from(feet[2]))
                    };
                    d(a).total_cmp(&d(b))
                })
                .map(|j| {
                    (j.job, rd.games.get(&rd.ship.jobs[j.job].game).map_or("a machine".to_owned(), |g| g.title.clone()))
                })
        });
        let mut dock = None;
        if let (Some((k, _)), true, None) = (&reach_job, pressed.contains(&use_key), seat) {
            pressed.remove(&use_key);
            dock = Some(*k as u8);
        }
        let repairing = self.deck.is_some();
        let mut spray = held.contains(&self.bindings.spray_key) || self.mouse_left;
        let chest = glam::DVec3::new(f64::from(feet[0]), f64::from(feet[1]) + 1.2, f64::from(feet[2]));
        let rooms = &self.data.tern_layout.rooms;
        let in_fire = rooms.iter().position(|r| r.contains(chest)).is_some_and(|k| burning.contains(&k));
        let (input, lift_keys) = match self.playtest.as_mut() {
            Some(p) => {
                let layout = &self.data.tern_layout;
                let room = |id: &str| layout.room(id).map(|k| layout.rooms[k].floor.as_vec3().to_array());
                let fires: Vec<[f32; 3]> =
                    burning.iter().filter_map(|k| layout.rooms.get(*k)).map(|r| r.floor.as_vec3().to_array()).collect();
                // The nearest place to take one: a full bracket (before it, on the floor) or the locker.
                let feet_d = glam::DVec3::from_array(feet.map(f64::from));
                let far = |p: &glam::DVec3| (p.x - feet_d.x).hypot(p.z - feet_d.z) + (p.y - feet_d.y).abs() * 4.0;
                let locker = layout
                    .brackets
                    .iter()
                    .enumerate()
                    .filter(|(k, _)| !empty.contains(k))
                    .map(|(_, b)| b.wall + b.into * 0.6)
                    .chain(layout.lockers.iter().copied())
                    .min_by(|a, b| far(a).total_cmp(&far(b)))
                    .map(|l| l.as_vec3().to_array());
                let job_at: Vec<[f32; 3]> = jobs.iter().map(|j| j.at.as_vec3().to_array()).collect();
                let hands = crate::playtest::Hands {
                    at_locker,
                    carrying: me.agent > 0.0,
                    in_fire,
                    fires: &fires,
                    locker,
                    jobs: &job_at,
                    at_job: reach_job.is_some(),
                    repairing,
                };
                let (input, e, h) = p.on_foot(foot, &self.seat_points, seat, room, &hands, &mut self.look, dt);
                sit = e;
                take = h.take;
                spray = h.spray;
                if h.repair {
                    dock = reach_job.as_ref().map(|j| j.0 as u8);
                }
                (input, false)
            }
            // At a machine the body stays: the game has the hands.
            None if repairing => (sc_core::walk::Input { yaw: self.look.0, ..Default::default() }, false),
            None => {
                let mut i = crate::onfoot::OnFoot::keys_input(&held, &pressed, self.look.0, &self.bindings);
                // The touch layer's stick walks the body as the keys do, by how far it is pushed (touch-controls 3).
                if let Some((forward, right, run)) = self.touch_stick.filter(|s| s.0 != 0.0 || s.1 != 0.0) {
                    i.forward = forward;
                    i.right = right;
                    i.run |= run;
                }
                (i, true)
            }
        };
        if let (Some(j), Some(s), false) = (dock, self.session.as_ref(), repairing) {
            if self.dock_asked.is_none_or(|(k, t)| k != j || self.time_s - t > 1.0) {
                s.dock(j);
                self.dock_asked = Some((j, self.time_s));
            }
        }
        if let (Some(st), Some(s)) = (sit, self.session.as_ref()) {
            s.claim(st);
        }
        let spray = spray && me.agent > 0.0;
        if let Some(s) = self.session.as_ref() {
            if take {
                s.take_extinguisher();
            }
            if spray != self.spray_sent {
                s.spray(spray);
                self.spray_sent = spray;
            }
        }
        foot.step(input, lift_keys, dt);
        if self.time_s - self.walk_sent >= WALK_SEND_S {
            if let Some(s) = self.session.as_ref() {
                s.walked(foot.body.feet, self.look.0);
            }
            self.walk_sent = self.time_s;
        }
        // What the use and spray keys do here (ui-art-direction design 7): sit (taking the seat from an NPC in it), take
        // an extinguisher, spray the fire, or the lift's and ladder's keys; the keys are the bindings', as read above.
        use crate::hud::Prompt;
        use crate::look::KeyTok;
        let crew = self.session.as_ref().map(|s| s.crew.clone()).unwrap_or_default();
        let others = self.session.as_ref().and_then(|s| s.latest()).map(|s| s.bodies.clone()).unwrap_or_default();
        let b = &self.bindings;
        let use_key = || vec![KeyTok::Key(b.use_label.clone())];
        match seat {
            Some(st) => Some(seat_prompt(st, me.slot, &crew, &others, &b.use_label)),
            None if repairing => None,
            None if reach_job.is_some() => reach_job.as_ref().map(|(_, title)| Prompt::act(title, use_key(), "Repair")),
            None if at_locker && me.agent < 1.0 => Some(Prompt::act("Extinguisher", use_key(), "Take")),
            None if emptied && me.agent < 1.0 => Some(Prompt::note("Extinguisher", "Empty")),
            None if in_fire && me.agent > 0.0 => {
                let mut p = Prompt::act("Fire", vec![KeyTok::Hold, KeyTok::Key(b.spray_label.clone())], "Spray");
                p.bar = Some(me.agent);
                Some(p)
            }
            None if in_fire => Some(Prompt::note("Fire", "Find an extinguisher")),
            None => foot.prompt(b),
        }
    }

    /// Seated, hold the use key (E) to get up, at every station (leave-station 2): `stand_hold_s`, counted from a
    /// press made in the seat (a tap does nothing).
    fn stand_up(&mut self, dt: f64, station: Option<Station>, posture: Option<sc_core::combat::bodies::Posture>) {
        let down = self.held.contains(&self.bindings.use_key);
        if self.stand.step(self.can_stand(station, posture), down, dt, self.data.stations.seats.stand_hold_s) {
            self.stand_now();
        }
    }

    /// Seated in a station of this player's own, the menu shut: LEAVE STATION and the stand key work.
    /// Whether this player may press START MISSION now (start-mission 2): `combat::may_start` on the crew the server
    /// sent, the rule the server applies to the request.
    fn may_start(&self) -> bool {
        let Some(s) = self.session.as_ref() else { return false };
        let Some(me) = s.slot else { return false };
        let crew: Vec<(u8, Option<Station>, bool)> = s.crew.iter().map(|c| (c.slot, c.station, c.bot)).collect();
        self.bot.is_none() && sc_core::combat::may_start(&crew, me).is_ok()
    }

    fn can_stand(&self, station: Option<Station>, posture: Option<sc_core::combat::bodies::Posture>) -> bool {
        posture == Some(sc_core::combat::bodies::Posture::Seated)
            && station.is_some()
            && self.bot.is_none()
            && !self.menu.open()
    }

    /// Up from the seat now: the server stands the body behind it and the station passes to automation.
    fn stand_now(&mut self) {
        if let Some(s) = self.session.as_ref() {
            s.stand();
        }
    }

    /// `--playtest`, seated: the script works the console (Engineering's schematic, the controls its beats press),
    /// marks READY in the briefing, and gets up when its beat says, holding the stand key as a player does.
    fn playtest_seated(
        &mut self,
        dt: f64,
        station: Option<Station>,
        posture: Option<sc_core::combat::bodies::Posture>,
    ) {
        if self.playtest_stand {
            self.held.remove(&self.bindings.use_key);
            self.playtest_stand = false;
        }
        let (Some(st), Some(sc_core::combat::bodies::Posture::Seated)) = (station, posture) else { return };
        let latest = self.session.as_ref().and_then(|s| s.latest());
        let fire = latest.is_some_and(|s| !s.damage.fires.is_empty());
        let engaged = latest.is_some_and(|s| s.phase == Phase::Engage);
        let Some(p) = self.playtest.as_mut() else { return };
        let at = p.at_console(st, fire, engaged, dt);
        if at.press.is_some() {
            self.playtest_press = at.press;
        }
        if st == Station::Engineering {
            match at.schematic {
                Some((three_d, turn)) => self.seats[st as usize].show_schematic(true, three_d, turn),
                None => self.seats[st as usize].show_schematic(false, false, 0.0),
            }
        }
        // In a patrol the script works its console's patrol controls as a bot reads them (`patrol::decide`, at the bot
        // profile's reaction): at the helm, UNDOCK once the drive is green, then the jumps. The controls' mockup awaits
        // approval (patrol-consoles.html), so the script sends the commands those controls will send.
        if let (Some(sess), Some(pd)) = (self.session.as_mut(), self.data.patrol.as_ref()) {
            let captain_manned = sess.crew.iter().any(|c| c.station == Some(Station::Captain));
            let fight = sess.latest().is_some_and(|snap| snap.phase == Phase::Engage);
            let picture =
                sess.latest().and_then(|snap| snap.patrol.as_ref()?.picture(&self.data, &snap.ships, captain_manned));
            if let Some(pic) = picture {
                let react = self.data.profile("bot").reaction_s(st);
                let mem = &mut self.playtest_patrol;
                for c in sc_core::combat::patrol::decide(st, &pic, &pd.block, mem, self.time_s, react) {
                    // In a fight the helm flies the fight (as a bot does); the patrol's decisions are the jump's.
                    if !(fight && matches!(c, Command::Helm(_) | Command::Stick { .. })) {
                        sess.command(c);
                    }
                }
            }
        }
        let muster = self.session.as_ref().and_then(|s| s.latest()).is_some_and(|s| s.phase == Phase::Muster);
        if muster && self.session.as_ref().is_some_and(|s| !s.ready()) {
            self.ready_held = true;
        }
        if at.stand {
            if st == Station::Engineering {
                self.seats[st as usize].show_schematic(false, false, 0.0);
            }
            self.held.insert(self.bindings.use_key);
            self.playtest_stand = true;
        }
    }

    /// The Tern's repair jobs that need a hand now, from the snapshot's integrity (all-hands 5: the drill's own rule).
    fn deck_jobs(&self, snap: Option<&Snapshot>) -> Vec<sc_core::combat::DeckJob> {
        let (Some(rd), Some(snap)) = (self.repair_data.as_ref(), snap) else { return Vec::new() };
        let integrity: Vec<f64> = snap.damage.integrity.iter().map(|v| f64::from(*v)).collect();
        sc_core::combat::deck_jobs(rd, &self.data.tern_layout, &integrity, self.data.damage.systems.nominal_from_pct)
    }

    /// Repairs on deck, a frame (all-hands 5): the server's orders open the game at the machine, give its next round, or
    /// close it when the job is done; the game plays; each round landed goes to the server; the eye looks at the
    /// machine while the hands work it.
    fn deck_frame(&mut self, f: &Frame, dt: f64, snap: Option<&Snapshot>, free: bool) {
        let orders = self.session.as_mut().map(|s| std::mem::take(&mut s.repairs)).unwrap_or_default();
        let jobs = self.deck_jobs(snap);
        for (job, next) in orders {
            match next {
                Some((round, _, _)) if self.deck.as_ref().is_some_and(|d| d.job == job) => {
                    if let Some(d) = self.deck.as_mut() {
                        d.next_round(round, self.time_s);
                    }
                }
                Some((round, _, _)) => {
                    let (Some(rd), Some(j)) =
                        (self.repair_data.as_ref(), jobs.iter().find(|j| j.job == usize::from(job)))
                    else {
                        continue;
                    };
                    let game = rd.ship.jobs[j.job].game.clone();
                    match crate::deckrepair::DeckRun::open(rd, job, &game, j.state, self.args.seed, round, self.time_s)
                    {
                        Ok(d) => {
                            println!("sc-client: repairing {} ({:?})", d.title(), j.state);
                            self.deck = Some(d);
                        }
                        Err(e) => eprintln!("sc-client: {e}"),
                    }
                }
                None => {
                    if self.deck.as_ref().is_some_and(|d| d.job == job) {
                        println!("sc-client: repaired");
                        self.deck = None;
                    }
                }
            }
        }
        // Up from the machine (sat down, or walked off): the game closes.
        if !free && self.deck.is_some() {
            self.deck = None;
            if let Some(s) = self.session.as_ref() {
                s.undock();
            }
        }
        let scripted = self.playtest.is_some();
        if self.deck.is_none() {
            return;
        }
        if !scripted {
            self.release_mouse();
        }
        let Some(d) = self.deck.as_mut() else { return };
        if let Some(j) = jobs.iter().find(|j| j.job == usize::from(d.job)) {
            if let Some(foot) = self.foot.as_ref() {
                let e = foot.body.feet;
                let want = (j.at.x as f32 - e[0]).atan2(j.at.z as f32 - e[2]);
                let turn = (want - self.look.0 + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU)
                    - std::f32::consts::PI;
                self.look.0 += turn * (dt as f32 * 4.0).min(1.0);
                self.look.1 += (-0.3 - self.look.1) * (dt as f32 * 3.0).min(1.0);
            }
        }
        if let Some(play_s) = d.update(f, dt, self.time_s, scripted) {
            if let Some(s) = self.session.as_ref() {
                s.round_done(d.job, d.round, play_s);
            }
        }
        if d.done() {
            self.deck = None;
            if let Some(s) = self.session.as_ref() {
                s.undock();
            }
        }
    }

    /// A finger (touch-controls design 3 and 4): the layer's when it takes it (a button, the stick, the look); else at a
    /// machine the repair game's pointer, seated in Engage the console's, and otherwise the UI's (the menus, the
    /// briefing, the debrief).
    fn touch_event(&mut self, id: u64, phase: TouchPhase, x: f32, y: f32, e: &Event) {
        // The finger in layout points, by the one mapping the drawing used (browser-page-fit design 5).
        let at = self.ui.fit().to_points(x, y);
        if self.touch.touch(id, phase, at.x, at.y) {
            return;
        }
        match self.touch_mode {
            crate::touch::Mode::Repair => {
                if let Some(d) = self.deck.as_mut() {
                    for m in self.touch_mouse.convert(e) {
                        d.event(&m);
                    }
                }
            }
            crate::touch::Mode::Seated if self.console_touch => {
                self.console_finger.take(id, phase, at, self.time_s);
            }
            _ => self.ui.event(e),
        }
    }

    /// This frame's touch layer: what it is doing, and its hands as the keyboard's keys, the stick's walk and the
    /// look's turn (touch-controls design 3: the same intents, one walk and one seat).
    fn touch_frame(&mut self, f: &Frame, dt: f64, free: bool, seated: bool) {
        use crate::touch::{Btn, Mode};
        let off = self.menu.open() || self.bot.is_some() || self.args.watch || self.playtest.is_some();
        let mode = match () {
            _ if off => Mode::Off,
            _ if self.deck.is_some() => Mode::Repair,
            _ if free => Mode::Foot { down: self.touch_down },
            _ if seated => Mode::Seated,
            _ => Mode::Off,
        };
        self.touch.set(&crate::ui::Fit::of(f), mode);
        self.touch_mode = mode;
        let hands = self.touch.frame(dt);
        for k in self.touch_keys.drain(..) {
            self.held.remove(&k);
        }
        let (pressed, held) = hands.keys();
        for k in pressed {
            if !self.held.contains(&k) {
                self.pressed.insert(k);
            }
        }
        for k in held {
            if self.held.insert(k) {
                self.touch_keys.push(k);
            }
        }
        self.touch_stick = Some((hands.forward, hands.right, hands.run));
        let k = self.options.look_rad_px() / crate::options::LOOK_RAD_PX;
        self.look.0 -= hands.look.0 * k;
        self.look.1 = (self.look.1 - hands.look.1 * k).clamp(-1.5, 1.5);
        for b in hands.taps {
            match b {
                Btn::Map => self.overview = !self.overview,
                Btn::Menu => {
                    self.menu.escape();
                    self.held.clear();
                    self.touch_keys.clear();
                    self.release_mouse();
                }
                _ => {}
            }
        }
    }

    /// Give the mouse back (the menu, a console). The playtest keeps it: its pointer is hidden from the recording.
    fn release_mouse(&mut self) {
        if self.captured && self.playtest.is_none() {
            self.captured = false;
            sc_client::platform::capture_mouse(false);
        }
    }

    /// Single player: show `st`'s console (the player's own for their seat). Leaving the helm's console lets its
    /// stick go, unless its NPC flies it: a key held while switching would otherwise hold the turn.
    fn show(&mut self, st: Station) {
        let Some(host) = self.solo.as_mut() else { return };
        let next = if st == host.seat || host.npc(st).is_none() { None } else { Some(st) };
        let was = self.shown.unwrap_or(host.seat);
        if next == self.shown {
            return;
        }
        if was == Station::Helm {
            let speed = self.seats[Station::Helm as usize].speed_set;
            let neutral = Command::Stick { speed_set_mps: speed, yaw: 0.0, pitch: 0.0, roll: 0.0 };
            match host.npc(Station::Helm).map(|n| n.auto) {
                None => {
                    if let Some(s) = self.session.as_mut() {
                        s.command(neutral);
                    }
                }
                Some(false) => host.command_as(Station::Helm, neutral),
                Some(true) => {}
            }
        }
        self.shown = next;
    }

    /// Single player: Tab and Shift+Tab step through the consoles, the player's own first; M changes the shown one's
    /// mode.
    fn solo_keys(&mut self, pressed: &HashSet<u32>) {
        let Some(host) = self.solo.as_mut() else { return };
        if pressed.contains(&keys::M) {
            if let Some(n) = self.shown.and_then(|st| host.npc_mut(st)) {
                n.auto = !n.auto;
            }
        }
        // On the gunner's sight Tab steps the target instead (the mockup's key); Shift+Tab still moves on.
        let shift = self.held.contains(&keys::LSHIFT) || self.held.contains(&keys::RSHIFT);
        let sight = self.shown.unwrap_or(host.seat) == Station::Gunner;
        if pressed.contains(&keys::TAB) && (!sight || shift) {
            let ids: Vec<Station> = std::iter::once(host.seat).chain(host.crew.iter().map(|n| n.station)).collect();
            let at = ids.iter().position(|s| Some(*s) == self.shown).unwrap_or(0);
            let back = shift;
            let next = ids[(at + if back { ids.len() - 1 } else { 1 }) % ids.len()];
            self.show(next);
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
        // Crew chat, printed so playtests show the crew's talk until its screens are approved (crew-chat design 5).
        for l in s.drain_chat() {
            println!("sc-client: chat: {}", l.plain());
        }
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
                Visual::Beam { from, to, hit, .. } => {
                    let t0 = now - rtt / 2.0;
                    self.beams.push(VisBeam { from, to, t0 });
                    if hit {
                        self.booms.push(Boom { pos: to, t0, dur: 0.6, radius: 15.0 });
                    }
                }
                Visual::Burst { pos, size } => {
                    // A burst aboard: sized by the energy past the armour or the integrity a part lost.
                    let t0 = now - rtt / 2.0;
                    let radius = 3.0 + size.max(0.0).sqrt() * 1.5;
                    self.booms.push(Boom { pos, t0, dur: 0.9, radius });
                    // Debris spun off the point: 6-20 pieces, out from the ship and tumbling.
                    let out = self
                        .ships_now()
                        .map(|(t, h, _)| if (t.pos - pos).length() < (h.pos - pos).length() { t.pos } else { h.pos })
                        .map_or(DVec3::Y, |c| (pos - c).normalize_or(DVec3::Y));
                    let n = 6 + (size.max(0.0) / 4.0).min(14.0) as usize;
                    for k in 0..n {
                        let a = (k as f64 * 2.399 + t0 * 7.0).sin();
                        let b = (k as f64 * 1.731 + t0 * 3.0).cos();
                        let c = (k as f64 * 0.917 + t0 * 5.0).sin();
                        let spray = (out + DVec3::new(a, b, c) * 0.8).normalize_or(out);
                        let speed = 15.0 + 25.0 * ((k * 7 % 5) as f64 / 4.0);
                        self.debris.push(Debris {
                            pos,
                            vel: spray * speed,
                            spin: Vec3::new(a as f32, b as f32, c as f32) * 4.0,
                            t0,
                            dur: 2.0 + (k % 3) as f64 * 0.5,
                        });
                    }
                }
                Visual::Decoy(id) => {
                    // A ring of flares round the ship that threw it.
                    if let Some((t, h, _)) = self.ships_now() {
                        let ship = if id == TERN_ID { t } else { h };
                        for k in 0..6 {
                            let a = k as f64 * std::f64::consts::TAU / 6.0;
                            let off =
                                ship.rot * DVec3::new(a.cos() * 40.0, (k % 2) as f64 * 15.0 - 7.0, a.sin() * 40.0);
                            self.booms.push(Boom {
                                pos: ship.pos + off,
                                t0: now + k as f64 * 0.15,
                                dur: 3.0,
                                radius: 6.0,
                            });
                        }
                    }
                }
                Visual::Destroyed(id) => {
                    // A fighter goes up where it was.
                    let fighter =
                        self.ships_now().and_then(|(_, _, s)| s.fighters.iter().find(|f| f.id == id).map(|f| f.pos));
                    if let Some(pos) = fighter {
                        self.booms.push(Boom { pos, t0: now, dur: 1.2, radius: 20.0 });
                    }
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
                for seat in &mut self.seats {
                    seat.hit(&tern, face, vel, now);
                }
            }
        }
        let render_t = now - INTERP_DELAY_S;
        self.bolts.retain(|b| render_t - b.t0 < b.life_s && b.hit_at.is_none_or(|h| render_t < h));
        self.booms.retain(|b| render_t - b.t0 < b.dur);
    }

    /// The viewscreen's feed: through the console's camera (its rig's, the one its bracket is projected through:
    /// chase-orbit design 7), into its own target, which the console draws in the viewscreen's frame; off the bow it
    /// draws the Tern too (chase-camera 1). With no console (on foot, the overview) it is the bridge's main
    /// viewscreen's picture through its own rig: the captain's camera while he holds the viewscreen, his console's
    /// otherwise (chase-camera 2, chase-orbit 6).
    fn render_feed(&mut self, ships: Option<&(ShipSnap, ShipSnap, Snapshot)>, view: Option<&console::ConsoleView>) {
        let rot = ships.map_or(DQuat::from_rotation_y(self.time_s * 0.02), |(t, _, _)| t.rot);
        let chase_cfg = &self.exterior.viewscreen.chase;
        let shot = match view {
            Some(v) => console::feed_shot(v),
            None => {
                // The captain's camera, CHASE, zoom and orbit, by the rule every console's panel uses.
                let br = ships.map(|(_, _, s)| &s.bridge);
                let main = br.map_or(console::ScreenSet::main(1, false, 0, None), |b| {
                    console::ScreenSet::main(b.main_view, b.main_chase, b.main_zoom, b.main_orbit)
                });
                let set = console::on_screen(
                    br.and_then(|b| b.view),
                    br.map_or((false, 0, None), |b| (b.main_chase, b.main_zoom, b.main_orbit)),
                    main,
                );
                let target =
                    ships.filter(|(_, h, _)| h.active && h.alive).map(|(t, h, _)| t.rot.inverse() * (h.pos - t.pos));
                let want =
                    console::rig::Want { feed: set.feed, chase: set.chase, zoom: set.zoom, target, orbit: set.orbit };
                self.main_rig.step(self.time_s, rot, &want, chase_cfg)
            }
        };
        let centre = ships.map_or(DVec3::ZERO, |(t, _, _)| t.pos);
        let eye = centre + rot * shot.eye;
        let b = shot.basis();
        let (fwd, up, vfov) = (rot * b.f, rot * b.u, shot.vfov);
        let proj = sc_render::depth::outside_lens(vfov as f32, FEED.0 as f32 / FEED.1 as f32);
        let view_m = glam::camera::rh::view::look_to_mat4(Vec3::ZERO, fwd.as_vec3(), up.as_vec3());
        let (ext_now, stations, roof) = self.outside_now(ships.map(|x| &x.2));
        let world = World {
            meshes: &self.meshes,
            exterior: &ext_now,
            bolts: &self.bolts,
            beams: &self.beams,
            booms: &self.booms,
            debris: &self.debris,
            time_s: self.time_s,
            stations,
            hull: self.bridge.as_ref().and_then(|b| b.hull()),
        };
        self.r.begin_3d(&self.feed, [0.0, 0.0, 0.0, 1.0]);
        world.draw(&mut self.r, &self.feed, proj, view_m, vfov as f32 / FEED.1 as f32, eye, ships);
        // CHASE sees the Tern from outside: its hull, drawn in the ship's frame from where the camera stands in it.
        if let (true, Some(h)) = (shot.hull, self.bridge.as_ref().and_then(|b| b.hull())) {
            let q = rot.as_quat();
            let in_ship = shot.eye;
            // The sky's sun from where the Tern is (one sun outside, design 11.3), or the bay's light under a roof.
            let light = self.hull_light(roof, &ext_now, q);
            h.draw_lit(&mut self.r, &self.feed, proj * view_m * Mat4::from_quat(q), in_ship.to_array(), &light);
        }
        self.r.end_pass();
        self.r.begin_3d(&self.blank, [0.0, 0.0, 0.0, 1.0]);
        self.r.end_pass();
    }
}

impl DrillApp {
    /// The outside of this frame (patrol-mission design 11): in a patrol the sky from the system file as seen from
    /// the Tern, and the stations within reach (`StationsView::draw_m`); in the drill the exterior's sky and the relay
    /// at the mission's waypoint. Also where the Tern is when it is under a station's roof (in Tamsin Station's bay or
    /// throat), where its hull is lit by the bay instead of the star (design 12b; spacedock design 8b).
    fn outside_now(&self, snap: Option<&Snapshot>) -> (ExteriorData, Vec<(Shown, DVec3, Quat)>, Option<Roof>) {
        let (Some(p), Some(d)) = (snap.and_then(|s| s.patrol.as_ref()), self.data.patrol.as_ref()) else {
            let relay = (Shown::Relay, DVec3::from_array(self.data.mission.waypoint_m), Quat::IDENTITY);
            return (self.exterior.clone(), vec![relay], None);
        };
        let tern = snap.and_then(|s| s.ships.first()).map_or(DVec3::ZERO, |t| t.pos);
        let ext = crate::outside::exterior_at(&self.exterior, &d.system, p.origin + tern);
        let mut stations = Vec::new();
        let mut roof = None;
        if let Some(v) = self.meshes.stations.as_ref() {
            for (i, st) in d.system.stations.iter().enumerate() {
                let at = d.system.station_pos(st) - p.origin;
                if (at - tern).length() <= v.draw_m() {
                    let rot = d.system.station_rot(st);
                    let here = rot.inverse() * (tern - at);
                    if v.under_roof(i, here) {
                        roof = Some(Roof { station: i, at: here, rot: rot.as_quat() });
                    }
                    stations.push((Shown::Station(i), at, rot.as_quat()));
                }
            }
        }
        let relay = d.places[d.encounter].pos - p.origin;
        if (relay - tern).length() <= RELAY_DRAW_M {
            stations.push((Shown::Relay, relay, Quat::IDENTITY));
        }
        (ext, stations, roof)
    }

    /// The light on the Tern's hull, its attitude `q` (ship frame to the system's): the sky's sun from where it is
    /// (`ext`, design 11.3), or under a station's roof the bay's (`StationsView::hull_light`: its probes, or the
    /// stand-in from its own up; down the throat, giving way to the star).
    fn hull_light(&self, roof: Option<Roof>, ext: &ExteriorData, q: Quat) -> HullLight {
        let star = HullLight::star(&self.r, crate::outside::sun_in_ship(ext, q), Some(q));
        let (Some(roof), Some(v)) = (roof, self.meshes.stations.as_ref()) else { return star };
        v.hull_light(roof.station, roof.at, roof.rot.inverse() * q, star).unwrap_or(star)
    }
}

/// Where the Tern is when it is under a station's roof: the station (the system file's order), the Tern's place in the
/// station's frame (metres) and the station's attitude (its frame to the system's).
#[derive(Debug, Clone, Copy)]
struct Roof {
    station: usize,
    at: DVec3,
    rot: Quat,
}

/// The relay is drawn within this of the Tern, metres (design 11.2: beyond, a point; later): well inside
/// `sc_render::depth::SOLID_REACH_M`, so it never meets the far plane.
const RELAY_DRAW_M: f64 = 80_000.0;

/// What the 3D views draw, borrowed from the app so the renderer can be borrowed beside it.
struct World<'a> {
    meshes: &'a Meshes,
    exterior: &'a ExteriorData,
    bolts: &'a [VisBolt],
    beams: &'a [VisBeam],
    booms: &'a [Boom],
    debris: &'a [Debris],
    time_s: f64,
    /// The stations outside: which, where in the drill's frame, and their attitude.
    stations: Vec<(Shown, DVec3, Quat)>,
    /// The Tern's hull model, for the ships docked at a station.
    hull: Option<&'a crate::hull::HullView>,
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
        let sky = crate::sky_params(self.exterior, proj, view, px_rad, self.time_s);
        r.draw_sky(t, &sky);
        self.draw_objects(r, t, proj * view, eye, ships);
    }

    /// The Hound, missiles, bolts and bursts alone (no sky), seen from `eye` through `vp` into `t`'s pass: the feed
    /// draws them over its sky, the bridge through its windows.
    fn draw_objects(
        &self,
        r: &mut Renderer,
        t: &Target,
        vp: Mat4,
        eye: DVec3,
        ships: Option<&(ShipSnap, ShipSnap, Snapshot)>,
    ) {
        let rel = |p: DVec3| (p - eye).as_vec3();
        let flat = crate::drill_bridge::flat_params;
        // One sun lights everything outside (patrol-mission design 11.3).
        let sun = Vec3::from_array(normalized(self.exterior.sun.dir));
        let cube = crate::outside::sun_cube(sun, &crate::outside::sky_tint(r, Quat::IDENTITY));
        // The stations: the relay where the helm's COURSE flies, the system's spacedocks (design 12b), their beacons
        // blinking.
        for (shown, at, rot) in &self.stations {
            let b = &self.exterior.beacons;
            match (shown, self.meshes.stations.as_ref()) {
                (Shown::Relay, _) => self.meshes.relay.draw(r, t, vp, rel(*at), *rot, sun, b, self.time_s),
                (Shown::Station(i), Some(v)) => v.draw(r, t, vp, *i, rel(*at), *rot, b, self.time_s, self.hull),
                (Shown::Station(_), None) => {}
            }
        }
        if let Some((_, hound, snap)) = ships {
            if hound.active && hound.alive {
                let q = Quat::from_xyzw(hound.rot.x as f32, hound.rot.y as f32, hound.rot.z as f32, hound.rot.w as f32);
                let model = Mat4::from_rotation_translation(q, rel(hound.pos));
                r.draw_deck_probe(t, &self.meshes.hound, vp * model, Mat4::from_quat(q), &cube, [1.0, 0.0, 0.0]);
            }
            // The enemy's fighters (enemy-fighters 4), lit like the Hound.
            for f in &snap.fighters {
                let q = Quat::from_xyzw(f.rot.x as f32, f.rot.y as f32, f.rot.z as f32, f.rot.w as f32);
                let model = Mat4::from_rotation_translation(q, rel(f.pos));
                r.draw_deck_probe(t, &self.meshes.jackal, vp * model, Mat4::from_quat(q), &cube, [1.0, 0.0, 0.0]);
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
        for b in self.beams {
            if render_t - b.t0 > BEAM_SHOW_S || render_t < b.t0 {
                continue;
            }
            let d = b.to - b.from;
            let len = d.length().max(1.0) as f32;
            let model = Mat4::from_rotation_translation(
                Quat::from_rotation_arc(Vec3::Z, d.normalize_or(DVec3::Z).as_vec3()),
                rel((b.from + b.to) * 0.5),
            ) * Mat4::from_scale(Vec3::new(40.0, 40.0, len));
            r.draw_deck(t, &self.meshes.beam, DeckProgram::Flat, &flat(vp * model), None);
        }
        for p in self.debris {
            let age = render_t - p.t0;
            if !(0.0..p.dur).contains(&age) {
                continue;
            }
            let at = p.pos + p.vel * age;
            let turn = Quat::from_scaled_axis(p.spin * age as f32);
            let model = Mat4::from_rotation_translation(turn, rel(at)) * Mat4::from_scale(Vec3::new(9.0, 9.0, 2.5));
            r.draw_deck(t, &self.meshes.debris, DeckProgram::Flat, &flat(vp * model), None);
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
    /// The crew's bodies 100 ms behind the newest snapshot, as the ships are drawn, each with the division of the
    /// station it holds (or walks to): what it wears (crew-uniforms 2).
    fn bodies_now(&self) -> Vec<(sc_net::msg::BodySnap, String)> {
        let Some(s) = self.session.as_ref() else { return Vec::new() };
        let Some((a, b, f)) = s.interpolation() else { return Vec::new() };
        crate::drill_bridge::blend(&a.bodies, &b.bodies, f)
            .into_iter()
            .map(|x| {
                let held = s.crew.iter().find(|c| c.slot == x.slot).and_then(|c| c.station).or(x.going);
                let division = held.map_or("operations", |st| self.data.division_of(st)).to_owned();
                (x, division)
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

    /// One frame of the UI's capture run; false when it is over. `seated`: the body sits at its console;
    /// `loading`: how much of the loading screen shows.
    fn tour_step(&mut self, f: &Frame, seated: bool, loading: f32) -> bool {
        let Some(mut t) = self.tour.take() else { return true };
        let video = t.args.video;
        if video {
            let name = format!("frame-{:05}.png", t.frames);
            let px = sc_render::read_output(f.width, f.height);
            if let Err(e) = sc_client::capture::write_png(&t.args.dir.join(name), f.width, f.height, &px) {
                eprintln!("sc-client: {e}");
            }
        }
        t.frames += 1;
        let now = self.time_s;
        // How long a screen is held: long enough to read in the video, long enough for a fade in a capture; the console
        // a while into the fight.
        let hold = match t.step {
            0 => 0.0,
            11 => 3.0,
            _ if video => 2.5,
            _ => 0.5,
        };
        let phase = self.session.as_ref().and_then(|s| s.latest()).map(|s| s.phase);
        let boarded = self.boarded_at.map_or(0.0, |b| now - b);
        let risen = self.seated_at.is_some_and(|s| now - s > console_rise_s() + 0.3);
        let own = self.args.station.name();
        let plate = self.plate.as_ref().is_some_and(|(p, a)| p.label == own && !p.acts.is_empty() && *a >= 0.99);
        // Each step: is its screen up, the menu's page for it, and its capture's name.
        let (ready, page, name): (bool, &str, &str) = match t.step {
            0 => (loading > 0.9, "closed", "1-loading"),
            1 => (self.free && boarded > 1.5, "closed", "2-hud-first-steps"),
            2 => (true, "main", "3-pause-walking"),
            3 => (plate, "closed", "4-prompt"),
            4 => (seated && risen && matches!(phase, Some(Phase::Muster | Phase::Countdown)), "closed", "5-briefing"),
            5 => (true, "main", "6-pause-seated"),
            6 => (true, "leave", "7-pause-leave"),
            7 => (true, "display", "8-options"),
            8 => (true, "controls-foot", "9-controls-foot"),
            9 => (true, "controls-console", "10-controls-console"),
            10 => (true, "controls-touch", "11-controls-touch"),
            11 => (phase.is_some_and(Phase::live), "closed", "12-console"),
            _ => {
                println!("sc-client: the UI run is done ({} frames)", t.frames);
                return false;
            }
        };
        // A moment that went by on its own is passed: the loading screen once aboard, the prompt once seated, the
        // briefing once the fight is on.
        let gone = match t.step {
            0 => boarded > 1.0,
            3 => seated,
            4 => phase.is_some_and(Phase::live),
            _ => false,
        };
        if !ready && gone {
            println!("sc-client: ui run: step {} passed", t.step);
            t.step += 1;
            t.since = now;
        }
        if !ready {
            t.since = now;
        } else {
            self.menu.tour_page(page);
            // The briefing is read: READY goes out once the crew sit, as a player's would.
            if t.step >= 5 {
                self.ready_held = !self.session.as_ref().is_some_and(|s| s.ready());
            }
            if now - t.since >= hold {
                if !video {
                    let px = sc_render::read_output(f.width, f.height);
                    let path = t.args.dir.join(format!("{name}.png"));
                    match sc_client::capture::write_png(&path, f.width, f.height, &px) {
                        Ok(()) => println!("sc-client: wrote {}", path.display()),
                        Err(e) => eprintln!("sc-client: {e}"),
                    }
                }
                // ui-art-direction task 4.1: the UI's draw calls (egui's meshes, one draw each) and its textures.
                println!(
                    "sc-client: ui run: {name}: {} UI draw calls, {:.1} MB of UI textures",
                    self.ui_meshes,
                    self.ui.texture_bytes() as f64 / 1e6
                );
                t.step += 1;
                t.since = now;
                self.menu.tour_page("closed");
            }
        }
        self.tour = Some(t);
        true
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
        // A scripted single-player run has no hands on READY: it presses it once the briefing is shot, as a bot does
        // after reading it, and it goes out when the crew are seated.
        if self.solo.is_some() && snap.phase == Phase::Muster && t >= 2.0 {
            self.ready_held = true;
        }
        // START MISSION (start-mission 4): a scripted run seated while the round waits shoots the band with the
        // control, then presses it a second later, as the player would (again each second while it is still up: a
        // press lost to a reconnect), and shoots the countdown.
        if let Some(seen) = self.start_seen {
            self.capture(f, &format!("1-start-mission-{st}.png"));
            if t - seen >= 1.0 {
                self.shots_taken.insert("start-pressed".into());
                self.start_seen = Some(t);
                if let Some(s) = self.session.as_ref() {
                    println!("sc-client: START MISSION (scripted)");
                    s.start_mission();
                }
            }
        }
        if snap.phase == Phase::Countdown && self.shots_taken.contains("start-pressed") && t >= 0.5 {
            self.capture(f, &format!("2-countdown-started-{st}.png"));
        }
        if self.args.film {
            self.film(f, &snap);
            return;
        }
        if snap.patrol.is_some() {
            self.patrol_shots(f, &snap);
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
        // The game menu over the console, opened and closed by the script (game-menu 2.5).
        let menu_shot = format!("7-menu-{st}.png");
        if snap.phase == Phase::Engage && (30.0..32.0).contains(&t) && !self.shots_taken.contains(&menu_shot) {
            if !self.menu.open() {
                self.menu.escape();
            } else if t >= 30.5 {
                self.capture(f, &menu_shot);
                self.menu.escape();
            }
        }
        // Single player: the crew's tabs on the player's own console, then Tactical's console (or the first NPC's) on
        // AUTO, then taken MANUAL, then back to AUTO and the player's own.
        let other = self.solo.as_ref().and_then(|h| h.npc(Station::Tactical).or(h.crew.first())).map(|n| n.station);
        if let (Some(other), Phase::Engage) = (other, snap.phase) {
            let own = self.args.station;
            let o = other.id();
            let set = |app: &mut Self, auto: bool| {
                if let Some(n) = app.solo.as_mut().and_then(|h| h.npc_mut(other)) {
                    n.auto = auto;
                }
            };
            if (8.0..10.0).contains(&t) {
                self.show(own);
                if t >= 8.5 {
                    self.capture(f, &format!("s1-solo-{st}.png"));
                }
            } else if (10.0..12.0).contains(&t) {
                self.show(other);
                if t >= 11.0 {
                    self.capture(f, &format!("s2-solo-{o}-auto.png"));
                }
            } else if (12.0..14.5).contains(&t) {
                set(self, false);
                if t >= 13.5 {
                    self.capture(f, &format!("s3-solo-{o}-manual.png"));
                }
            } else if (14.5..15.0).contains(&t) {
                set(self, true);
                self.show(own);
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

impl DrillApp {
    /// The patrol's scripted shots (patrol-mission design 12b, spacedock tasks 2.9): berthed in Tamsin Station's bay,
    /// out of the bridge's port and starboard windows, ahead to the throat, and from out in the bay at the nearest docked ship and back at the Tern; in the throat on the way out; then
    /// Tamsin Station seen from 14 km off once the Tern is clear, and along the same line from 76 km, near the edge of
    /// its draw distance, where the whole station must still stand in front of the sky (sky-behind-solids 2). Each
    /// stands its eye one frame, then is taken.
    fn patrol_shots(&mut self, f: &Frame, snap: &Snapshot) {
        if let Some(name) = self.shot_pending.take() {
            self.capture(f, &name);
            self.shot_eye = None;
            if name.starts_with("p6-") {
                self.done = true;
            }
            return;
        }
        let (Some(p), Some(d), Some(t)) = (snap.patrol.as_ref(), self.data.patrol.as_ref(), snap.ships.first()) else {
            return;
        };
        let Some(st) = d.system.stations.iter().find(|s| s.berths.iter().any(|b| b.departure.len() > 1)) else {
            return;
        };
        let (at, rot) = (d.system.station_pos(st), d.system.station_rot(st));
        let here = rot.inverse() * (p.origin + t.pos - at);
        let window = |w: crate::playtest::Window, pitch: f32| {
            let (feet, yaw) = w.stand();
            ([f64::from(feet[0]), f64::from(feet[1]) + 1.6, f64::from(feet[2])], yaw, pitch)
        };
        use crate::playtest::Window;
        let berthed = snap.phase == Phase::Patrol && snap.phase_s >= 4.0 && here.z < 100.0;
        let ahead = |mut e: ShotEye| {
            e.1 = 0.25;
            e
        };
        let want: Option<(&str, ShotEye)> = if berthed {
            // From the port window toward the nearest docked ship, lit by the bay's probes (spacedock design 8b).
            let si = d.system.stations.iter().position(|s| std::ptr::eq(s, st));
            let heron = self.meshes.stations.as_ref().zip(si).and_then(|(v, i)| {
                v.docked(i)
                    .iter()
                    .map(|m| m.w_axis.truncate())
                    .min_by(|a, b| (*a - here).length().total_cmp(&(*b - here).length()))
            });
            // An eye out in the bay between the two, 40 m up toward the axis, on the docked ship; then the Tern's own hull.
            let to_ship = |v: DVec3| t.rot.inverse() * (rot * v);
            let look_from = |eye_st: DVec3, at_st: DVec3| -> ShotEye {
                let dir = to_ship(at_st - eye_st).normalize();
                (to_ship(eye_st - here).to_array(), dir.x.atan2(dir.z) as f32, dir.y.asin() as f32)
            };
            let toward_axis = -DVec3::new(here.x, here.y, 0.0).normalize_or_zero();
            let bay_eye = heron.map(|h| here + (h - here) * 0.45 + toward_axis * 40.0);
            [
                ("p1-berth-port", Some(window(Window::BridgePort, 0.12))),
                ("p2-berth-starboard", Some(window(Window::BridgeStbd, 0.12))),
                ("p3-berth-ahead", Some(ahead(window(Window::BridgePort, 0.12)))),
                ("p3b-bay-docked-ship", bay_eye.zip(heron).map(|(e, h)| look_from(e, h))),
                // Above and aft of the Tern, so the bay's spine on the axis stays behind the eye.
                ("p3c-bay-tern-hull", Some(look_from(here + toward_axis * 70.0 - DVec3::Z * 90.0, here))),
            ]
            .into_iter()
            .filter_map(|(n, e)| e.map(|e| (n, e)))
            .find(|(n, _)| !self.shots_taken.contains(&format!("{n}.png")))
        } else if here.z > 480.0 && here.z < 600.0 && here.x.hypot(here.y) < 60.0 {
            Some(("p4-throat", window(Window::BridgePort, 0.3)))
        } else if here.z > 2_900.0 {
            // From the star's side, below and to port of the pair, about 14 km out, looking at its middle.
            let near_st = DVec3::new(-5_000.0, -4_500.0, 6_000.0);
            let look_st = DVec3::new(4_300.0, 1_500.0, -2_000.0);
            let (name, eye_st) = if self.shots_taken.contains("p5-pair.png") {
                ("p6-pair-far", look_st + (near_st - look_st).normalize() * 76_000.0)
            } else {
                ("p5-pair", near_st)
            };
            let to_ship = |v: DVec3| t.rot.inverse() * (rot * v);
            let eye = to_ship(eye_st - here);
            let dir = to_ship(look_st - eye_st).normalize();
            let (yaw, pitch) = (dir.x.atan2(dir.z) as f32, dir.y.asin() as f32);
            Some((name, (eye.to_array(), yaw, pitch)))
        } else {
            None
        };
        if let Some((name, eye)) = want {
            let name = format!("{name}.png");
            if !self.shots_taken.contains(&name) {
                self.shot_eye = Some(eye);
                self.shot_pending = Some(name);
            }
        }
    }

    /// The film of the viewscreen's cameras (chase-camera 3). It follows the body: the walk to the captain's chair,
    /// sitting down and the countdown, then, once the round is engaged, the console's FWD with chase off and on,
    /// TARGET, PORT, AFT and STBD with chase on, TARGET at 1x, 4x and 20x, TARGET with chase on at 4x, and the
    /// bridge's main viewscreen showing the captain's TARGET with chase on. With --shots every frame is written, at
    /// the video's 30 a second, so the video plays in real time; frames go to the shots folder as `film-NNNNN.png`,
    /// and each shot's first frame and caption to `film-cues.txt`; `tools/video/film.sh` makes the video.
    fn film(&mut self, f: &Frame, snap: &Snapshot) {
        let seated = self.seated_at.is_some();
        let t = snap.phase_s;
        let engaged = snap.phase == Phase::Engage;
        let st = self.console_station() as usize;
        let cue = if !seated && !engaged {
            "Walking to the captain's chair, on foot"
        } else if !engaged || t < 2.0 {
            "Seated at Captain: the console comes up, the round counts down"
        } else if self.args.film_orbit {
            match self.orbit_reel(t, snap) {
                Some(cue) => cue,
                None => {
                    self.film_screen = false;
                    self.done = true;
                    return;
                }
            }
        } else {
            let seat = &mut self.seats[st];
            // Each shot: the camera, chase on or off, the zoom, and its caption.
            let shot: Option<(&str, bool, f64, &str)> = match t {
                t if t < 4.0 => {
                    Some(("FWD", false, 1.0, "FWD, chase off: the captain's viewscreen from the bow, as before"))
                }
                t if t < 6.5 => {
                    Some(("FWD", true, 1.0, "FWD, chase on: from behind the Tern, the ship low and centred"))
                }
                t if t < 10.0 => Some((
                    "TARGET",
                    true,
                    1.0,
                    "TARGET, chase on: past the Tern at the Hound, the bracket round the Hound",
                )),
                t if t < 12.0 => Some(("PORT", true, 1.0, "PORT, chase on: looking to port past the ship")),
                t if t < 14.0 => Some(("AFT", true, 1.0, "AFT, chase on: looking aft past the ship")),
                t if t < 16.0 => Some(("STBD", true, 1.0, "STBD, chase on: looking to starboard past the ship")),
                t if t < 18.0 => Some(("TARGET", false, 1.0, "TARGET, chase off, 1x: the bracket round the Hound")),
                t if t < 20.0 => Some(("TARGET", false, 4.0, "TARGET, 4x: zoomed, the bracket closes round the hull")),
                t if t < 22.0 => Some(("TARGET", false, 20.0, "TARGET, 20x")),
                t if t < 24.0 => Some(("TARGET", true, 4.0, "TARGET, chase on, 4x: magnified past the ship")),
                t if t < 28.0 => None,
                _ => {
                    self.film_screen = false;
                    self.done = true;
                    return;
                }
            };
            match shot {
                Some((feed, chase, zoom, cue)) => {
                    seat.set_feed(feed);
                    seat.set_chase(chase);
                    seat.set_zoom(zoom);
                    cue
                }
                None => {
                    seat.set_feed("TARGET");
                    seat.set_chase(true);
                    seat.set_zoom(1.0);
                    if !self.film_screen {
                        // The captain's camera on the main viewscreen, as his console sends it when he presses.
                        if let Some(s) = self.session.as_mut() {
                            s.command(sc_core::combat::Command::MainView {
                                feed: 0,
                                chase: true,
                                zoom: 0,
                                orbit: None,
                            });
                        }
                        self.film_screen = true;
                    }
                    "The bridge's main viewscreen shows the captain's camera: TARGET, chase on"
                }
            }
        };
        self.film_frame += 1;
        if self.film_cue != cue {
            self.film_cue = cue.to_owned();
            if let Some(dir) = self.args.shots.as_ref() {
                use std::io::Write;
                let line = format!("{}\t{cue}\n", self.film_frame);
                let file = std::fs::OpenOptions::new().create(true).append(true).open(dir.join("film-cues.txt"));
                if let Err(e) = file.and_then(|mut w| w.write_all(line.as_bytes())) {
                    eprintln!("sc-client: film cues: {e}");
                }
            }
        }
        let name = format!("film-{:05}.png", self.film_frame);
        self.capture(f, &name);
    }
}

impl DrillApp {
    /// The orbit reel (`--film-orbit`, chase-orbit): from 2 s into Engage, the captain's viewscreen from the bow, then
    /// CHASE easing out behind the Tern and following its turns, TARGET easing round, ORBIT circling the ship and moving
    /// in and out as a drag and the wheel would, ORBIT off easing back, and the bridge's main viewscreen showing his
    /// orbit. None when it is over.
    fn orbit_reel(&mut self, t: f64, snap: &Snapshot) -> Option<&'static str> {
        let st = self.console_station() as usize;
        let target = match (snap.ships.first(), snap.ships.get(1)) {
            (Some(tn), Some(h)) if h.active && h.alive => Some(tn.rot.inverse() * (h.pos - tn.pos)),
            _ => None,
        };
        // Each orbit starts afresh where the chase camera is.
        if (23.0..26.0).contains(&t) {
            self.film_orbit0 = None;
        }
        let cfg = &self.exterior.viewscreen.chase;
        // A circle round the ship from where ORBIT starts, `turns` of it, its elevation and distance swinging.
        let mut orbit = |from: f64, secs: f64, turns: f64| {
            let k = ((t - from) / secs).clamp(0.0, 1.0);
            let yaw0 = *self.film_orbit0.get_or_insert_with(|| console::rig::orbit_start("TARGET", target, cfg)[0]);
            let tau = std::f64::consts::TAU;
            [yaw0 - 360.0 * turns * k, -30.0 * (tau * k).sin() - 10.0, 160.0 + 120.0 * (tau * k * 1.5).sin().max(-0.5)]
        };
        let (feed, chase, goal, cue): (&str, bool, Option<[f64; 3]>, &'static str) = match t {
            t if t < 3.0 => ("FWD", false, None, "FWD, chase off: the captain's viewscreen from the bow"),
            t if t < 9.0 => {
                ("FWD", true, None, "CHASE on: the camera eases out behind the Tern and follows its turns with a lag")
            }
            t if t < 13.0 => {
                ("TARGET", true, None, "TARGET, chase on: the camera eases round to look past the Tern at the Hound")
            }
            t if t < 23.0 => (
                "TARGET",
                true,
                Some(orbit(13.0, 10.0, 1.0)),
                "ORBIT on: dragging turns the camera round the ship, the wheel moves it in and out",
            ),
            t if t < 26.0 => ("TARGET", true, None, "ORBIT off: the camera eases back to the chase position"),
            t if t < 34.0 => {
                let g = orbit(26.0, 8.0, 0.75);
                if !self.film_screen {
                    self.film_screen = true;
                }
                ("TARGET", true, Some(g), "The bridge's main viewscreen shows the captain's orbit")
            }
            _ => return None,
        };
        let seat = &mut self.seats[st];
        seat.set_feed(feed);
        seat.set_chase(chase);
        seat.set_zoom(1.0);
        seat.set_orbit(goal);
        Some(cue)
    }
}

impl App for DrillApp {
    fn event(&mut self, e: Event) -> Flow {
        if let Event::Touch { id, phase, x, y } = e {
            self.touch_event(id, phase, x, y, &e);
            return Flow::Continue;
        }
        // A key or the mouse: the touch layer hides until the next finger.
        if matches!(e, Event::KeyDown(_) | Event::MouseButton(..) | Event::MouseMotion(..)) {
            self.touch.other_input();
        }
        self.ui.event(&e);
        // At a machine the game takes the pointer and the keys; Esc steps away from it.
        if let Some(d) = self.deck.as_mut() {
            if e == Event::KeyDown(keys::ESCAPE) {
                self.deck = None;
                if let Some(s) = self.session.as_ref() {
                    s.undock();
                }
                return Flow::Continue;
            }
            d.event(&e);
        }
        match e {
            Event::Quit => return Flow::Done,
            // Esc first closes OBJECTIVES over a console (objectives-tab 1), then opens or steps back through the
            // game menu (game-menu 2.4).
            Event::KeyDown(keys::ESCAPE)
                if !self.menu.open() && self.seats[self.console_station() as usize].objectives_open() =>
            {
                let st = self.console_station() as usize;
                self.seats[st].close_objectives();
            }
            Event::KeyDown(keys::ESCAPE) => {
                self.menu.escape();
                self.held.clear();
                self.release_mouse();
            }
            // On foot a click takes the mouse for looking around; its motion turns the eye.
            Event::MouseDown if self.free && !self.captured && !self.menu.open() => {
                self.captured = true;
                sc_client::platform::capture_mouse(true);
            }
            Event::MouseButton(1, down) => self.mouse_left = down && self.captured,
            Event::MouseMotion(dx, dy) if self.captured => {
                let k = self.options.look_rad_px();
                self.look.0 -= dx * k;
                self.look.1 = (self.look.1 - dy * k).clamp(-1.5, 1.5);
            }
            // While the menu is up, its keys are its own (the game underneath runs on, design 2).
            Event::KeyDown(_) if self.menu.open() => {}
            Event::KeyDown(keys::V) => self.overview = !self.overview,
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
        // The film written to frames (`--film --shots`) steps the video's own 30 frames a second from its first frame,
        // whatever a frame takes to draw, and keeps every frame: the video plays in real time, the walk and the
        // countdown too (chase-camera 3; owner, 2026-10-14: "normal time please, I dont like the speedup"). Without
        // --shots the film runs on the clock, for a recorder on a machine with a GPU (obs-record); everything else
        // steps by the frame, as a player's game does.
        let filming = (self.args.film && self.args.shots.is_some()) || self.tour.as_ref().is_some_and(|t| t.args.video);
        let dt = if filming { 1.0 / 30.0 } else { f.elapsed_s.clamp(0.0, 0.1) };
        self.time_s += dt;
        if let Some(h) = self.solo.as_mut() {
            h.step(dt, &self.data);
        }
        if self.session.is_none() && self.time_s >= self.retry_at {
            self.try_connect();
        }
        // READY waits for the crew to sit, so a round starts with every seat filled (single-player design 4).
        if self.ready_held && self.solo.as_ref().is_some_and(crate::solo::SoloHost::all_seated) {
            if let Some(s) = self.session.as_ref() {
                s.set_ready(true);
                self.ready_held = false;
            }
        }
        if let Some(s) = self.session.as_mut() {
            s.update();
            // The ships are drawn on the server's clock, at this frame's own step (chase-orbit design 2).
            s.advance(dt);
            // The server's mission is the one played: a client started with another loads it (patrol-mission).
            if !s.mission.is_empty() && s.mission != self.data.mission.id {
                match load_data(&s.mission).and_then(|d| Ok((station_views(&self.r, &d)?, d))) {
                    Ok((v, d)) => {
                        self.data = d;
                        self.meshes.stations = v;
                    }
                    Err(e) => self.error = Some(e),
                }
            }
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
                    self.beams.clear();
                    self.booms.clear();
                    self.debris.clear();
                }
                println!("sc-client: round {round} {p:?}");
            }
        }
        let station = self.session.as_ref().and_then(|s| s.station());
        // Single player: the player's own chip follows the seat they sit at (relieve-bots 5), and a crewed console shown
        // that has become theirs is simply theirs.
        if let Some(h) = self.solo.as_mut() {
            h.follow(station);
            if self.shown == Some(h.seat) {
                self.shown = None;
            }
        }
        let aspect = f.width as f32 / f.height.max(1) as f32;
        // On foot (design 9): while this player's body is up and walking, or with the overview (V), the bridge.
        let bodies = self.bodies_now();
        let me = self.session.as_ref().and_then(|s| s.slot);
        let mine = bodies.iter().find(|(b, _)| Some(b.slot) == me).map(|x| x.0);
        let walking = mine.is_some_and(|b| b.posture != sc_core::combat::bodies::Posture::Seated);
        // On foot (the law): a player's body standing free is theirs to walk, with W A S D and the mouse.
        use sc_core::combat::bodies::Posture;
        let free = self.bot.is_none()
            && !self.args.watch
            && station.is_none()
            && self.foot.is_some()
            && mine.is_some_and(|b| b.going.is_none() && matches!(b.posture, Posture::Standing | Posture::Walking));
        let seated_body = station.is_some() && mine.is_some_and(|b| b.posture == Posture::Seated);
        self.touch_frame(f, dt, free, seated_body);
        let foot_prompt = match (free, mine) {
            (true, Some(me)) => {
                let burning: Vec<usize> = ships
                    .as_ref()
                    .map(|(_, _, snap)| snap.damage.fires.iter().map(|r| usize::from(*r)).collect())
                    .unwrap_or_default();
                let jobs = self.deck_jobs(ships.as_ref().map(|x| &x.2));
                let empty: Vec<usize> = ships
                    .as_ref()
                    .map(|(_, _, snap)| snap.damage.empty_brackets.iter().map(|k| usize::from(*k)).collect())
                    .unwrap_or_default();
                self.step_foot(dt, me, &burning, &jobs, &empty)
            }
            _ => None,
        };
        // A refusal from the server, said once (the screen shows it over a console only).
        let refused = self.session.as_ref().and_then(|s| s.refused);
        if refused.map(|r| r.1) != self.refused_said {
            if let Some((r, _)) = refused {
                println!("sc-client: refused: {}", r.text());
            }
            self.refused_said = refused.map(|r| r.1);
        }
        // What the keys do here, said once when it changes: a playtest's and the browser smoke test's view of the walk.
        let line = foot_prompt.as_ref().map(crate::hud::Prompt::line);
        if line != self.prompt_said {
            if let Some(p) = &line {
                println!("sc-client: prompt: {p}");
            }
            self.prompt_said = line;
        }
        // By thumb the plate shows no keys and USE carries the verb (touch-controls 3).
        let touching = self.touch.active();
        let b = &self.bindings;
        self.touch_down = foot_prompt.as_ref().is_some_and(|p| p.has_key(&b.down_label));
        let use_word =
            foot_prompt.as_ref().filter(|_| touching).and_then(|p| p.touch_word(&b.use_label, &b.spray_label));
        // The plate fades in and out (design 8); walking to a station picked from a tab says where.
        let want = foot_prompt
            .clone()
            .or_else(|| mine.and_then(|b| b.going).map(|g| crate::hud::Prompt::note(g.name(), "On your way")));
        let m = &crate::look::look().f.motion_ms;
        // Linear fades by the frame's share of each (a prompt can turn back mid-fade, so it steps rather than starts).
        let (fade_in, fade_out) = (crate::motion::progress(dt, m.prompt_in), crate::motion::progress(dt, m.prompt_out));
        self.plate = match (want, self.plate.take()) {
            (Some(p), was) => Some((p, (was.map_or(0.0, |w| w.1) + fade_in).min(1.0))),
            (None, Some((p, a))) => Some((p, a - fade_out)).filter(|x| x.1 > 0.0),
            (None, None) => None,
        };
        if !free {
            self.release_mouse();
        }
        self.deck_frame(f, dt, ships.as_ref().map(|x| &x.2), free);
        if self.playtest.is_some() && !self.captured {
            self.captured = true;
            sc_client::platform::capture_mouse(true);
        }
        self.free = free;
        // Seated: hold Stand to get up (E, or Backspace; at Helm, where E rolls the ship, Backspace).
        self.stand_up(dt, station, mine.map(|b| b.posture));
        self.playtest_seated(dt, station, mine.map(|b| b.posture));
        let shot_overview =
            self.args.shots.is_some() && (self.shot_overview() || self.film_screen || self.shot_eye.is_some());
        let on_bridge = self.bridge.is_some() && !bodies.is_empty() && (walking || self.overview || shot_overview);
        // Sat down: the console rises from the desk, the bridge seen from the chair behind it until it is up.
        let seated = !walking && station.is_some() && mine.is_some();
        if seated && self.seated_at.is_none() {
            self.rise_from = self.last_view;
            // The first-time hint has done its work once the player sits (ui-art-direction design 11).
            if !self.options.first_steps_done && self.bot.is_none() && self.args.shots.is_none() && self.tour.is_none()
            {
                self.options.first_steps_done = true;
                self.options.keep();
            }
            println!("sc-client: sat at {}", station.map_or("a seat", |s| s.name()));
        }
        if !seated && self.seated_at.is_some() {
            println!("sc-client: stood up");
        }
        self.seated_at = if seated { self.seated_at.or(Some(self.time_s)) } else { None };
        let rise = self.seated_at.map_or(1.0, |t| console_risen(self.time_s - t));
        let rising = seated && rise < 1.0 && self.bridge.is_some() && !self.overview;
        let mut bridge_ui = None;
        if on_bridge || rising {
            let own = (walking || rising) && !self.overview && !shot_overview;
            // Its own body is not drawn from its own eye, nor a watcher's anywhere (it only watches).
            let hide_me = own || self.args.watch;
            let (eye, yaw, pitch) = match (own, mine, self.foot.as_ref().filter(|_| free)) {
                (true, _, Some(f)) => (f.eye(), self.look.0, self.look.1),
                // Sitting down: from where the eye last was on foot into the chair, eased, as the console comes up.
                (true, Some(b), None) if rising => {
                    let to = (crate::drill_bridge::eye_of(&b), b.yaw, -0.18f32);
                    let from = self.rise_from.unwrap_or(to);
                    let k = rise;
                    let turn =
                        (to.1 - from.1 + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
                    let e: [f64; 3] = std::array::from_fn(|i| from.0[i] + (to.0[i] - from.0[i]) * f64::from(k));
                    (e, from.1 + turn * k, from.2 + (to.2 - from.2) * k)
                }
                (true, Some(b), None) => (crate::drill_bridge::eye_of(&b), b.yaw, -0.12),
                // The film's look at the bridge's main viewscreen: from behind the captain's chair, over his head,
                // square to the screen so its picture reads as the console's does.
                _ if self.film_screen => ([0.0, 6.0, 23.4], 0.0, -0.06),
                // The patrol's shots: out of a window, or the station seen from afar (design 12b).
                _ if self.shot_eye.is_some() => self.shot_eye.unwrap_or_else(crate::drill_bridge::overview),
                _ => crate::drill_bridge::overview(),
            };
            if own && !rising {
                self.last_view = Some((eye, yaw, pitch));
            }
            // The bridge's viewscreens show the feed: rendered first, in its own pass.
            self.render_feed(ships.as_ref(), None);
            if let Some(bv) = &self.bridge {
                let attitude = ships.as_ref().map_or(Quat::IDENTITY, |(t, _, _)| t.rot.as_quat());
                let red = ships.as_ref().is_some_and(|(_, _, s)| s.bridge.red_alert);
                // The bridge's eye in the system's frame, where the Hound, bolts and bursts are.
                let eye_out = ships.as_ref().map(|(t, _, _)| t.pos + t.rot * DVec3::from_array(eye));
                let (ext_now, stations, roof) = self.outside_now(ships.as_ref().map(|x| &x.2));
                let hull_light = self.hull_light(roof, &ext_now, attitude);
                let world = World {
                    meshes: &self.meshes,
                    exterior: &ext_now,
                    bolts: &self.bolts,
                    beams: &self.beams,
                    booms: &self.booms,
                    debris: &self.debris,
                    time_s: self.time_s,
                    stations,
                    hull: bv.hull(),
                };
                let seen = ships.as_ref();
                let out = crate::drill_bridge::Outside {
                    attitude,
                    lighting: if red { [0.0, 1.0, 0.0] } else { [1.0, 0.0, 0.0] },
                    screen: Some((&self.feed, self.exterior.viewscreen.scanlines as f32)),
                    hull_light: Some(hull_light),
                    objects: Box::new(move |r, t, vp| {
                        if let Some(e) = eye_out {
                            world.draw_objects(r, t, vp, e, seen);
                        }
                    }),
                };
                // The fires burning in the Tern's rooms, and the extinguishers spraying (all-hands 4): this player's from
                // the one in hand, a crewmate's from their hands.
                let burning: Vec<usize> = ships
                    .as_ref()
                    .map(|(_, _, snap)| snap.damage.fires.iter().map(|r| usize::from(*r)).collect())
                    .unwrap_or_default();
                let mut nozzles = Vec::new();
                for (b, _) in bodies.iter().filter(|(b, _)| b.spraying && b.agent > 0.0) {
                    if hide_me && Some(b.slot) == me {
                        let feet = self.foot.as_ref().map_or(b.pos[1], |f| f.body.feet[1]);
                        nozzles.push(crate::firefx::held_nozzle(eye, yaw, pitch, feet));
                    } else {
                        nozzles.push(crate::firefx::body_nozzle(b.pos, b.yaw));
                    }
                }
                let rooms = &self.data.tern_layout.rooms;
                self.fire_fx.step(&burning, rooms, self.time_s, eye, &nozzles, dt as f32);
                let empty_brackets: Vec<usize> = ships
                    .as_ref()
                    .map(|(_, _, snap)| snap.damage.empty_brackets.iter().map(|k| usize::from(*k)).collect())
                    .unwrap_or_default();
                let inside = crate::drill_bridge::Inside {
                    fx: &self.fire_fx,
                    held: &self.meshes.held,
                    empty_brackets: &empty_brackets,
                    carrying: mine.is_some_and(|b| b.agent > 0.0),
                    time_s: self.time_s,
                };
                let vp = bv.draw(
                    &mut self.r,
                    &self.bridge_target,
                    &ext_now,
                    eye,
                    yaw,
                    pitch,
                    aspect,
                    &bodies,
                    me,
                    hide_me,
                    &inside,
                    out,
                );
                // The picture in the UI's points, as the screen is laid out (ui::Fit).
                let pts = self.ui.fit().points;
                let (w, h) = (pts.x, pts.y);
                let crew = self.session.as_ref().map(|s| s.crew.clone()).unwrap_or_default();
                // A name over every body (crew-nameplates 2): on screen, near and in sight (the crew start about the
                // ship, so walls and decks hide some). Off when Options says so.
                let names = self.options.names;
                let labels = bodies
                    .iter()
                    .filter(|_| names)
                    .filter(|(b, _)| !(hide_me && Some(b.slot) == me))
                    .filter_map(|(b, _)| {
                        let head = [b.pos[0], b.pos[1] + 2.05, b.pos[2]];
                        let dist = head.iter().zip(eye).map(|(a, e)| (f64::from(*a) - e).powi(2)).sum::<f64>().sqrt();
                        let k = crate::nameplate::fade(dist as f32)?;
                        if !bv.sees(eye, b.pos) {
                            return None;
                        }
                        let at = crate::drill_bridge::project(vp, eye, head, w, h)?;
                        let who = crew.iter().find(|c| c.slot == b.slot);
                        let name = who.map(|c| c.name.clone()).unwrap_or_default();
                        let going = b.going.map(|g| format!("to {}", g.name()));
                        let npc = who.is_some_and(|c| c.bot);
                        Some(Label { at: Pos2::new(at[0], at[1]), name, npc, going, k, dist })
                    })
                    .collect::<Vec<Label>>();
                // The farthest first, so a nearer plate is drawn over one behind it.
                let mut labels = labels;
                labels.sort_by(|a, b| b.dist.total_cmp(&a.dist));
                // The HUD (ui-art-direction design 11): the objective, the alert at alert only, the extinguisher in
                // hand, and the first time aboard the hint with a marker over the player's station.
                let my_station = self.solo.as_ref().map_or(self.args.station, |h| h.seat);
                let objective = match ships.as_ref().map(|x| x.2.phase) {
                    Some(Phase::Muster | Phase::Countdown) => {
                        Some(format!("Report to the {}", my_station.name().to_lowercase()))
                    }
                    Some(_) => ships
                        .as_ref()
                        .and_then(|(t, h, s)| crate::seat::objectives_view(s, t, h, &self.data))
                        .and_then(|ov| ov.rows.iter().find(|r| r.state != "done").map(|r| r.words.clone())),
                    None => None,
                };
                let hint = (own && free && !self.options.first_steps_done && self.bot.is_none()).then(|| {
                    let at = self.seat_points.iter().find(|p| p.0 == my_station).and_then(|(_, p, _)| {
                        let top = [p[0], p[1] + 1.6, p[2]];
                        let d = top.iter().zip(eye).map(|(a, e)| (f64::from(*a) - e).powi(2)).sum::<f64>().sqrt();
                        crate::drill_bridge::project(vp, eye, top, w, h).map(|q| (Pos2::new(q[0], q[1]), d as f32))
                    });
                    (my_station.name().to_owned(), at)
                });
                bridge_ui = (!rising).then_some(BridgeUi {
                    labels,
                    plate: self.plate.clone(),
                    touch: touching,
                    objective: objective.filter(|_| own),
                    red_alert: red,
                    agent: mine.filter(|b| own && b.agent > 0.0 && b.posture != Posture::Seated).map(|b| b.agent),
                    hint,
                });
            }
        }
        let crew = self.session.as_ref().map(|s| s.crew.clone()).unwrap_or_default();
        // The console's view: the station this client holds (or wants), from the newest interpolated snapshot; in
        // single player the crewed station shown, with the crew's tabs.
        let st = self.console_station();
        let solo = self.solo.is_some();
        let mut console_view = ships.as_ref().filter(|_| !on_bridge).map(|(t, h, snap)| {
            let ops: Vec<(Station, String)> = crew.iter().filter_map(|c| Some((c.station?, c.name.clone()))).collect();
            let mut v = self.seats[st as usize].view(st, t, h, snap, &self.data, &ops, self.time_s);
            // LEAVE STATION while this player sits (leave-station 4): the stand key on its keycap, the hold's ring.
            v.leave = seated_body.then(|| console::LeaveView {
                key: self.bindings.use_label.clone(),
                share: self.stand.share(self.data.stations.seats.stand_hold_s),
            });
            // START MISSION while the round waits and this player may start it (start-mission 4): the server's own
            // rule, asked of the crew list it sends.
            v.start = (snap.phase == Phase::Muster && self.may_start()).then(|| console::StartView {
                key: self.bindings.start_label.clone(),
                ready: crew.iter().filter(|c| c.ready && c.station.is_some()).count() as u32,
                aboard: crew.len() as u32,
            });
            v
        });
        if let (Some(v), Some(host)) = (console_view.as_mut(), self.solo.as_ref()) {
            v.open.clear();
            v.solo = Some(console::SoloView {
                seat: host.seat.id().into(),
                crew: host
                    .crew
                    .iter()
                    .map(|n| console::CrewTab {
                        station: n.station.id().into(),
                        op: n.name.clone(),
                        auto: n.auto,
                        seated: n.seated(),
                    })
                    .collect(),
            });
        }
        // The gunner's sight has no viewscreen (enemy-fighters 3): its console covers the screen.
        let sight = console_view.as_ref().is_some_and(|v| v.station == "gunner");
        if !on_bridge && !sight {
            self.render_feed(ships.as_ref(), console_view.as_ref());
        }
        let mut ask: Vec<UiAsk> = Vec::new();
        // The round starting under a briefing: it lowers into the desk (design 18a).
        let phase = ships.as_ref().map(|x| x.2.phase);
        if matches!(self.seen_phase, Some(Phase::Muster | Phase::Countdown))
            && matches!(phase, Some(Phase::Engage | Phase::Patrol))
        {
            self.lowered_at = Some(self.time_s);
        }
        self.seen_phase = phase.or(self.seen_phase);
        let mut view = UiView::gather(self, ships.as_ref());
        view.bridge = bridge_ui;
        let mut painted: Option<(console::kit::Hits, f32)> = None;
        let mut pointer: Vec<PointerEv> = Vec::new();
        let show_net = self.show_net;
        if ships.is_some() && self.boarded_at.is_none() {
            self.boarded_at = Some(self.time_s);
        }
        let menu_view = menu_view(self, ships.as_ref(), seated_body);
        let art = self.art.get_or_insert_with(|| (None, crate::hud::tip(&self.bindings, self.args.seed)));
        let need_art = art.0.is_none();
        let controls = self.bindings.controls.clone();
        let loading_a = self.boarded_at.map_or(1.0, |t| {
            let l = crate::look::look();
            1.0 - crate::motion::eased(self.time_s - t, l.f.motion_ms.load_out, l.f.motion_ease.load_out)
        });
        let mut menu_ask = None;
        let (menu, options) = (&mut self.menu, &mut self.options);
        let deck = self.deck.as_ref();
        let touch = &self.touch;
        let finger_hover = self.console_finger.hover();
        let stand_share = self.stand.share(self.data.stations.seats.stand_hold_s);
        let frame = self.ui.run(&mut self.r, f, self.time_s, |ctx| {
            if let Some(v) = console_view.as_ref() {
                // A long press's finger is the hover on a touch screen (touch-controls 4): the tip shows there.
                let hover = finger_hover.or_else(|| ctx.input(|i| i.pointer.hover_pos()));
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
            if need_art {
                art.0 = crate::look::load_art(ctx, crate::look::Art::Loading);
            }
            draw_ui(ctx, &view, &mut ask, show_net, (art.0.as_ref(), art.1.as_ref()), loading_a);
            if let Some(d) = deck {
                let painter =
                    ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::new("deck-repair")));
                d.draw(&painter, ctx.content_rect().size());
            }
            let layer = ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::new("touch")));
            touch.paint(&layer, use_word.as_deref());
            touch.paint_stand(&layer, stand_share);
            menu_ask = menu.show(ctx, &menu_view, options, &controls);
        });
        self.pointer = pointer;
        // A finger on the console in Engage, against this frame's controls (touch-controls 4).
        let engage_now = ships.as_ref().is_some_and(|x| x.2.phase == Phase::Engage);
        self.console_touch =
            self.touch_mode == crate::touch::Mode::Seated && engage_now && !self.menu.open() && painted.is_some();
        match painted.as_ref().filter(|_| self.console_touch) {
            Some((hits, _)) => {
                let evs = self.console_finger.resolve(hits, self.touch.k(), self.time_s);
                self.pointer.extend(evs);
            }
            None => self.console_finger.clear(),
        }
        // LEAVE STATION pressed, by the mouse or a finger (leave-station 4): up at once, in any phase. The press is the
        // button's, so it is not a touch that takes a crewed station MANUAL.
        if let Some((hits, _)) = painted.as_ref() {
            if crate::seat::leave_pressed(hits, &self.pointer) {
                self.pointer.clear();
                if self.can_stand(station, mine.map(|b| b.posture)) {
                    println!("sc-client: LEAVE STATION");
                    self.stand_now();
                }
            }
        }
        // START MISSION, by the mouse, a finger or its key (start-mission 4): the request goes to the server, which
        // decides; the press is the button's, not a touch that takes a crewed station MANUAL.
        let start_shown = console_view.as_ref().is_some_and(|v| v.start.is_some());
        let phase_s = ships.as_ref().map_or(0.0, |x| x.2.phase_s);
        self.start_seen = if start_shown { self.start_seen.or(Some(phase_s)) } else { None };
        let mut start_now = start_shown && !self.menu.open() && self.pressed.contains(&self.bindings.start_key);
        if let Some((hits, _)) = painted.as_ref().filter(|_| start_shown) {
            if crate::seat::band_pressed(hits, &self.pointer, &crate::console::kit::Act::Start) {
                self.pointer.clear();
                start_now = true;
            }
        }
        if start_now {
            if let Some(s) = self.session.as_ref() {
                println!("sc-client: START MISSION");
                s.start_mission();
            }
        }
        // The script's press: the middle of the control as drawn, pressed and let go, as a player's pointer would.
        if let (Some(c), Some((hits, _))) = (self.playtest_press.take(), painted.as_ref()) {
            use crate::console::kit::{Act, Hit};
            let want = Hit::Act(match c {
                crate::playtest::Control::Screen => Act::ViewTake,
                crate::playtest::Control::Target => Act::Feed("TARGET".into()),
                crate::playtest::Control::Chase => Act::Feed("CHASE".into()),
            });
            match hits.regions.iter().rev().find(|r| r.hit.as_ref() == Some(&want)) {
                Some(r) => {
                    let p = r.rect.center();
                    println!("sc-client: playtest: pressed {c:?}");
                    self.pointer.extend([PointerEv::Press(p), PointerEv::Release(p)]);
                }
                None => println!("sc-client: playtest: no {c:?} on the console"),
            }
        }
        if let Some(b) = self.bridge.as_mut() {
            b.fov_deg = self.options.fov_deg as f32;
        }
        // Return to menu leaves the crew for the front menu (the station passes to automation, design 2); Exit closes
        // the game. Each was asked Yes or No first.
        match menu_ask {
            Some(crate::game_menu::MenuAsk::LeaveStation) => {
                if self.can_stand(station, mine.map(|b| b.posture)) {
                    println!("sc-client: LEAVE STATION");
                    self.stand_now();
                }
            }
            Some(crate::game_menu::MenuAsk::Leave) if self.args.menu => crate::back_to_menu(None),
            Some(crate::game_menu::MenuAsk::Leave | crate::game_menu::MenuAsk::Exit) => return Flow::Done,
            _ => {}
        }
        // The seated player's hands on the console, in Engage only (the briefing and debrief sit over it otherwise).
        let engage = ships.as_ref().is_some_and(|x| x.2.phase.live());
        let pressed = std::mem::take(&mut self.pressed);
        let engage = engage && !self.menu.open();
        if let (true, Some(_), None, Some((hits, scale)), Some(v), Some((t, h, _))) =
            (engage, station, self.bot.as_ref(), painted.as_ref(), console_view.as_ref(), ships.as_ref())
        {
            self.solo_keys(&pressed);
            let input = Input {
                events: std::mem::take(&mut self.pointer),
                held: self.held.clone(),
                pressed,
                keys: self.bindings.clone(),
            };
            let seat = &mut self.seats[st as usize];
            let asks = seat.input(&input, hits, *scale, v, t, h, &self.data, st, self.time_s);
            // A crewed station's console in single player: on AUTO its NPC plays it, and a touch takes it MANUAL
            // (design 4); on MANUAL what the player does goes out as its NPC's.
            let npc_auto = self.solo.as_mut().and_then(|host| host.npc_mut(st)).map(|n| {
                if n.auto && seat.touched() {
                    n.auto = false;
                }
                n.auto
            });
            if npc_auto == Some(true) {
                seat.follow(t);
            }
            for a in asks {
                match a {
                    SeatAsk::Command(c) => match npc_auto {
                        None => ask.push(UiAsk::Command(c)),
                        Some(false) => ask.push(UiAsk::As(st, c)),
                        Some(true) => {}
                    },
                    SeatAsk::Claim(s) if !solo => ask.push(UiAsk::Claim(s)),
                    SeatAsk::Claim(_) => {}
                    // The law: a console shows only to whoever sits at it; another station's tab says who holds it.
                    SeatAsk::View(s) if s == st => self.show(s),
                    SeatAsk::View(_) => {}
                    SeatAsk::Mode(auto) => {
                        if let Some(n) = self.solo.as_mut().and_then(|host| host.npc_mut(st)) {
                            n.auto = auto;
                        }
                    }
                    SeatAsk::Tab => {
                        if let Some(seat) = self.solo.as_ref().map(|host| host.seat) {
                            self.show(seat);
                        }
                    }
                }
            }
        }
        for a in ask {
            match a {
                UiAsk::As(st, c) => {
                    if let Some(host) = self.solo.as_mut() {
                        host.command_as(st, c);
                    }
                }
                UiAsk::Ready(true) if self.solo.as_ref().is_some_and(|h| !h.all_seated()) => self.ready_held = true,
                a => {
                    let Some(s) = self.session.as_mut() else { continue };
                    match a {
                        UiAsk::Claim(st) => s.claim(st),
                        UiAsk::Ready(r) => {
                            self.ready_held = false;
                            s.set_ready(r);
                        }
                        UiAsk::Command(c) if self.bot.is_none() => s.command(c),
                        UiAsk::Command(_) | UiAsk::As(..) => {}
                    }
                }
            }
        }
        self.ui_meshes = frame.meshes().len();
        // The viewscreen's feed in its window under the console, or black when there is no console yet.
        let ppp = frame.ppp;
        match console_view {
            None if on_bridge => {
                self.r.present_with_ui(&self.bridge_target, f.width, f.height, true, &frame.meshes(), frame.points)
            }
            // The console coming up from the desk: the chair's view behind it, the console eased up from below.
            Some(_) if rising => {
                let dy = (1.0 - rise) * frame.points[1];
                let up: Vec<(Vec<sc_render::UiVertex>, [i32; 4])> = frame
                    .meshes()
                    .iter()
                    .map(|m| {
                        let v = m.vertices.iter().map(|v| sc_render::UiVertex { pos: [v.pos[0], v.pos[1] + dy], ..*v });
                        let c = m.clip_px;
                        (v.collect(), [c[0], c[1] + (dy * ppp) as i32, c[2], c[3]])
                    })
                    .collect();
                let meshes: Vec<sc_render::UiMesh<'_>> = frame
                    .meshes()
                    .iter()
                    .zip(&up)
                    .map(|(m, (v, clip))| sc_render::UiMesh {
                        clip_px: *clip,
                        texture: m.texture,
                        vertices: v,
                        indices: m.indices,
                    })
                    .collect();
                self.r.present_with_ui(&self.bridge_target, f.width, f.height, true, &meshes, frame.points)
            }
            Some(_) if sight => {
                self.r.present_with_ui(&self.blank, f.width, f.height, true, &frame.meshes(), frame.points)
            }
            Some(_) => {
                let at = console::viewscreen_px(frame.points, ppp);
                self.r.present_with_ui_at(&self.feed, Some(at), f.width, f.height, true, &frame.meshes(), frame.points)
            }
            None => self.r.present_with_ui(&self.blank, f.width, f.height, true, &frame.meshes(), frame.points),
        }
        self.scripted_shots(f);
        if !self.tour_step(f, seated, loading_a) {
            return Flow::Done;
        }
        // The hand-over test: the game up with its briefing, then back to the menu.
        let briefing = self.session.as_ref().and_then(|s| s.latest()).is_some_and(|s| s.phase_s >= 1.0);
        if let (Some(dir), true) = (self.args.hand_test.clone(), briefing) {
            self.args.hand_test = None;
            if !crate::capture_to(f, &dir, "hand-over-2-game.png") {
                return Flow::Fail;
            }
            crate::back_to_menu(Some(dir));
        }
        self.r.commit();
        if self.done {
            Flow::Done
        } else {
            Flow::Continue
        }
    }
}

/// Who is in the way of seat `st` for the body in `slot`, and what the one seat rule makes of it (relieve-bots 2,
/// `sc-core`'s `seats::claim_rule`, which the server's claim also runs): its holder and the body walking to it.
fn seat_rule(
    st: Station,
    slot: Option<u8>,
    crew: &[sc_net::msg::CrewEntry],
    bodies: &[sc_net::msg::BodySnap],
) -> (ClaimRule, Option<Occupant>, Option<Occupant>) {
    let who = |s: u8| Occupant { slot: s, bot: crew.iter().any(|c| c.slot == s && c.bot) };
    let holder = crew.iter().find(|c| Some(c.slot) != slot && c.station == Some(st)).map(|c| who(c.slot));
    let walker = bodies.iter().find(|b| Some(b.slot) != slot && b.going == Some(st)).map(|b| who(b.slot));
    (claim_rule(false, holder, walker), holder, walker)
}

/// What the use key offers at seat `st` for the body in `slot`, by the server's rule: a free seat is sat in, a bot's (in
/// it or walking to it) is taken from it, a person's is theirs. The prompt never promises a seat the server would refuse
/// (CLAUDE.md 6.1: the preview is the outcome). Drawn as ui-art-direction design 7's plate: the station over its key and
/// verb, or over the name of whoever has it, with nothing to press.
fn seat_prompt(
    st: Station,
    slot: u8,
    crew: &[sc_net::msg::CrewEntry],
    bodies: &[sc_net::msg::BodySnap],
    use_label: &str,
) -> crate::hud::Prompt {
    use crate::hud::Prompt;
    let name = |s: u8| crew.iter().find(|c| c.slot == s).map_or("Someone", |c| c.name.as_str()).to_owned();
    let key = vec![crate::look::KeyTok::Key(use_label.to_owned())];
    match seat_rule(st, Some(slot), crew, bodies) {
        (ClaimRule::Sit, _, _) => Prompt::act(st.name(), key, "Sit down"),
        (ClaimRule::Relieve, holder, walker) => {
            let who = holder.or(walker).map_or("Someone".to_owned(), |o| name(o.slot));
            Prompt::act(st.name(), key, &format!("Relieve {who}"))
        }
        (ClaimRule::Taken(s), holder, _) if holder.is_some_and(|h| h.slot == s) => {
            Prompt { log_extra: Some(format!("holds {}", st.name())), ..Prompt::note(st.name(), &name(s)) }
        }
        (ClaimRule::Taken(s), _, _) => {
            Prompt { log_extra: Some(format!("is on the way to {}", st.name())), ..Prompt::note(st.name(), &name(s)) }
        }
    }
}

/// What the UI asks the session to do.
enum UiAsk {
    Claim(Station),
    Ready(bool),
    Command(Command),
    /// Single player: a command for a MANUAL station, sent through its NPC's session.
    As(Station, Command),
}

/// Everything the briefing, the debrief and the F3 corner draw from, gathered before the UI closure borrows the
/// renderer.
struct UiView {
    error: Option<String>,
    stage: Option<Stage>,
    station: Option<Station>,
    /// This player's slot.
    slot: Option<u8>,
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
    /// A system at or above this integrity is at nominal (ship-damage 9: the debrief waits for it).
    nominal_pct: f64,
    /// The bridge on screen instead of the consoles (design 9).
    bridge: Option<BridgeUi>,
    /// Single player: READY pressed, waiting for the crew to sit.
    ready_held: bool,
    /// The briefing lowering into the desk as the round starts, how far down (0..1), while it goes (design 18a).
    lowering: Option<f32>,
}

/// What the bridge view's UI shows: a name over each body (and where it is going), and this player's own walk.
/// A name over a body: where its foot is on screen, the name, whether it is an NPC, where it walks to, and how much
/// of it shows.
struct Label {
    at: Pos2,
    name: String,
    npc: bool,
    going: Option<String>,
    k: f32,
    /// From the eye, metres.
    dist: f64,
}

/// The walking HUD's state this frame (ui-art-direction design 11).
struct BridgeUi {
    labels: Vec<Label>,
    /// The prompt plate and how much of it shows.
    plate: Option<(crate::hud::Prompt, f32)>,
    /// By thumb: the plate shows no keys.
    touch: bool,
    /// The objective card's line.
    objective: Option<String>,
    /// The alert chip.
    red_alert: bool,
    /// The extinguisher in hand, its agent left.
    agent: Option<f32>,
    /// The first-time hint: the player's station, and its marker on screen and distance.
    hint: Option<(String, Option<(Pos2, f32)>)>,
}

impl UiView {
    fn gather(app: &DrillApp, ships: Option<&(ShipSnap, ShipSnap, Snapshot)>) -> Self {
        let s = app.session.as_ref();
        Self {
            error: app.error.clone(),
            stage: s.map(|s| s.stage),
            station: s.and_then(|s| s.station()),
            slot: s.and_then(|s| s.slot),
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
            nominal_pct: app.data.damage.systems.nominal_from_pct,
            bridge: None,
            ready_held: app.ready_held,
            lowering: app.lowered_at.map(|t| briefing_lowered(app.time_s - t)).filter(|k| *k < 1.0),
        }
    }
}

fn label(ui: &mut egui::Ui, text: &str, size: f32, colour: Color32) {
    ui.label(RichText::new(text).size(size).color(colour));
}

/// How far the briefing has lowered into the desk `since` seconds after the round started, on the style's curve.
fn briefing_lowered(since: f64) -> f32 {
    let l = crate::look::look();
    crate::motion::eased(since, l.f.motion_ms.briefing_lower, l.f.motion_ease.briefing_lower)
}

fn mmss(s: f64) -> String {
    let s = s.max(0.0) as u64;
    format!("{:02}:{:02}", s / 60, s % 60)
}

/// The game's screens around the consoles (ui-art-direction design 11): the loading screen until the ship is up (and
/// fading out over it), the walking HUD on the bridge, the briefing over the console before the round, the debrief.
/// `art`: the loading screen's and the briefing's picture and the tip; `loading`: how much of the loading screen still
/// shows over the game.
fn draw_ui(
    ctx: &egui::Context,
    v: &UiView,
    ask: &mut Vec<UiAsk>,
    show_net: bool,
    art: (Option<&egui::TextureHandle>, Option<&crate::hud::Tip>),
    loading: f32,
) {
    let mlabel = mission_label(&v.mission);
    let loading_screen = |progress: f32, a: f32| {
        let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::new("loading")));
        let mut painter = painter;
        painter.multiply_opacity(a);
        crate::hud::loading(&painter, ctx.content_rect(), art.0, &mlabel, &v.mission.title, art.1, progress);
        if let Some(e) = &v.error {
            let k =
                crate::look::kit(&painter, ctx.content_rect(), crate::ui::Anchor::BottomRight, Vec2::new(900.0, 200.0));
            let c = crate::look::look().c();
            k.text(crate::look::Role::Body, 900.0 - 96.0, 40.0, c.warn.0, e, crate::look::Align::End);
        }
    };
    let Some(snap) = v.snap.as_ref() else {
        let progress = match v.stage {
            Some(Stage::Joined) => 0.8,
            Some(_) => 0.5,
            None => 0.2,
        };
        loading_screen(progress, 1.0);
        return;
    };
    if let Some(b) = &v.bridge {
        bridge_overlay(ctx, b);
    } else {
        match snap.phase {
            Phase::Muster | Phase::Countdown => briefing(ctx, v, snap.phase, snap.phase_s, ask, art.0, 0.0),
            Phase::Engage | Phase::Patrol => {
                // The briefing goes down into the desk at its last count, taking no presses (design 18a).
                if let Some(k) = v.lowering {
                    briefing(ctx, v, Phase::Countdown, v.mission.countdown_s, &mut Vec::new(), art.0, k);
                }
                // A refused command says why, over the status strip, for a few seconds.
                if let Some((why, _)) = v.refused.as_ref().filter(|r| r.1 < 3.0) {
                    egui::Area::new(egui::Id::new("refused"))
                        .anchor(Align2::CENTER_BOTTOM, Vec2::new(0.0, -40.0))
                        .show(ctx, |ui| {
                            label(ui, why, 16.0, AMBER);
                        });
                }
            }
            Phase::Debrief => debrief(ctx, v, snap),
        }
    }
    // F3's round trip and loss: a debug overlay, off in release builds (ui-art-direction Q11).
    if show_net && cfg!(debug_assertions) {
        egui::Area::new(egui::Id::new("net")).anchor(Align2::RIGHT_TOP, Vec2::new(-10.0, 40.0)).show(ctx, |ui| {
            let rtt = v.rtt_ms.map_or("--".to_owned(), |r| format!("{r:.0} ms"));
            label(ui, &format!("RTT {rtt}  LOSS {:.1}%", v.loss * 100.0), 13.0, DIM);
        });
    }
    if loading > 0.0 {
        loading_screen(1.0, loading);
    }
}

/// What the Esc menu shows of the game (ui-art-direction design 11, shot pause-after): the mission, whether this
/// player sits, their name and station, the objectives as the OBJECTIVES tab measures them, and the crew.
fn menu_view(
    app: &DrillApp,
    ships: Option<&(ShipSnap, ShipSnap, Snapshot)>,
    seated: bool,
) -> crate::game_menu::MenuView {
    let s = app.session.as_ref();
    let crew = s.map(|s| s.crew.clone()).unwrap_or_default();
    let mine = s.and_then(|s| s.station());
    let slot = s.and_then(|s| s.slot);
    let objectives = match ships.and_then(|(t, h, snap)| crate::seat::objectives_view(snap, t, h, &app.data)) {
        Some(ov) => ov.rows.iter().map(|r| (r.words.clone(), r.state == "done")).collect(),
        None => app.data.mission.objectives.iter().map(|o| (o.words.clone(), false)).collect(),
    };
    let rows = Station::ALL
        .iter()
        .map(|st| {
            let who = crew.iter().find(|c| c.station == Some(*st));
            crate::hud::CrewRow {
                station: *st,
                name: who.map(|c| c.name.clone()),
                you: mine == Some(*st),
                ready: who.is_some(),
                npc: who.is_some_and(|c| c.bot),
            }
        })
        .collect();
    let me = crew
        .iter()
        .find(|c| Some(c.slot) == slot)
        .map(|c| (c.name.clone(), mine.map_or("person", crate::hud::role_of).to_owned()));
    crate::game_menu::MenuView {
        label: mission_label(&app.data.mission),
        title: app.data.mission.title.clone(),
        seated,
        me,
        objectives,
        crew: rows,
    }
}

/// The small line over a mission's name: its `label`, or the briefing's word for one without.
fn mission_label(m: &sc_core::combat::data::MissionFile) -> String {
    if m.label.is_empty() {
        "Mission".to_owned()
    } else {
        m.label.clone()
    }
}

/// The walking HUD (ui-art-direction design 11, shots hud-after, prompt-after, hint-after): a name over every body,
/// the objective card top left, the centre dot, the alert chip at alert only, the prompt plate below the dot, the
/// extinguisher's bar beside it, and the first time aboard the hint with its marker. No roster, phase or clock.
fn bridge_overlay(ctx: &egui::Context, b: &BridgeUi) {
    let screen = ctx.content_rect();
    let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Middle, egui::Id::new("bridge-names")));
    for l in &b.labels {
        crate::nameplate::draw(&painter, l.at, &l.name, l.npc, l.going.as_deref(), l.k);
    }
    if let Some(o) = &b.objective {
        crate::hud::objective_card(&painter, screen, o);
    }
    if b.red_alert {
        crate::hud::alert_chip(&painter, screen, true);
    }
    crate::hud::reticle(&painter, screen);
    if let Some(a) = b.agent {
        crate::hud::carried_bar(&painter, screen, a);
    }
    if let Some((station, marker)) = &b.hint {
        crate::hud::first_steps(&painter, screen, station, &[crate::look::KeyTok::Key("E".into())], *marker);
    }
    if let Some((p, a)) = &b.plate {
        crate::hud::prompt_plate(&painter, screen, p, *a, b.touch);
    }
}

/// The briefing on the console before the round (ui-art-direction design 11, Q7): a solid screen under the console's
/// band (LEAVE STATION and START MISSION stay in reach there), the mission, your orders, the crew, then READY or Launch
/// in. A crew row pressed takes that station (relieve-bots 6: never a person's). `lower`: how far it has gone down
/// into the desk as the round starts, 0 up to 1 gone (design 18a).
fn briefing(
    ctx: &egui::Context,
    v: &UiView,
    phase: Phase,
    phase_s: f64,
    ask: &mut Vec<UiAsk>,
    art: Option<&egui::TextureHandle>,
    lower: f32,
) {
    use crate::hud::{BriefingAsk, CrewRow, ReadyState};
    let mine = v.station.unwrap_or(v.want);
    let crew: Vec<CrewRow> = Station::ALL
        .iter()
        .map(|s| {
            let who = v.crew.iter().find(|c| c.station == Some(*s));
            CrewRow {
                station: *s,
                name: who.map(|c| c.name.clone()),
                you: v.station == Some(*s),
                ready: who.is_some_and(|c| c.ready),
                npc: who.is_some_and(|c| c.bot),
            }
        })
        .collect();
    let objectives: Vec<(String, bool)> = v.mission.objectives.iter().map(|o| (o.words.clone(), false)).collect();
    let launch = (phase == Phase::Countdown).then(|| {
        let total = v.mission.countdown_s.max(1e-3);
        ((total - phase_s).max(0.0), (phase_s / total).clamp(0.0, 1.0) as f32)
    });
    let ready = (v.station.is_some() && !v.bot).then_some(if v.ready {
        ReadyState::Ready
    } else if v.ready_held {
        ReadyState::Waiting
    } else {
        ReadyState::Open
    });
    let label = mission_label(&v.mission);
    let b = crate::hud::Briefing {
        label: &label,
        title: &v.mission.title,
        situation: &v.mission.situation,
        objectives,
        orders: (mine, v.mission.orders.of(mine)),
        crew,
        launch,
        ready,
    };
    let screen = ctx.content_rect();
    // The console's band (its top 32 layout points) stays: the solid screen is everything under it.
    let (o, k) = crate::ui::frame_in(screen);
    let band = Rect::from_min_size(o, Vec2::new(crate::ui::layout_size().x * k, BRIEF_BAND_PT * k));
    let got = crate::look::screen_area(ctx, "briefing", egui::Order::Middle, |ui, screen| {
        ui.set_clip_rect(screen);
        let below = Rect::from_min_max(egui::pos2(screen.min.x, band.max.y), screen.max);
        let above = Rect::from_min_max(screen.min, egui::pos2(screen.max.x, band.min.y));
        let sides = [
            Rect::from_min_max(egui::pos2(screen.min.x, band.min.y), egui::pos2(band.min.x, band.max.y)),
            Rect::from_min_max(egui::pos2(band.max.x, band.min.y), egui::pos2(screen.max.x, band.max.y)),
        ];
        let ground = crate::look::look().c().ground.0;
        for r in std::iter::once(above).chain(sides) {
            ui.painter().rect_filled(r, 0.0, ground);
        }
        // Lowering, the solid screen and everything on it go down under the band, uncovering the console.
        let drop = Vec2::new(0.0, lower.clamp(0.0, 1.0) * below.height());
        let mut sub = ui.new_child(egui::UiBuilder::new().max_rect(below.translate(drop)));
        sub.set_clip_rect(below);
        crate::hud::briefing(&mut sub, screen.translate(drop), art, &b)
    });
    match got {
        Some(BriefingAsk::Ready(r)) => ask.push(UiAsk::Ready(r)),
        Some(BriefingAsk::Claim(s)) if !v.bot && v.station != Some(s) => {
            // Off for a person's seat only: a bot's is taken from it (relieve-bots 6).
            let bodies = v.snap.as_ref().map_or(&[][..], |x| x.bodies.as_slice());
            if !matches!(seat_rule(s, v.slot, &v.crew, bodies).0, ClaimRule::Taken(_)) {
                ask.push(UiAsk::Claim(s));
            }
        }
        _ => {}
    }
}

/// The console's band, layout points: the briefing leaves it showing.
const BRIEF_BAND_PT: f32 = 32.0;

/// An "s" for more than one.
fn plural(n: usize) -> &'static str {
    if n == 1 {
        ""
    } else {
        "s"
    }
}

/// The debrief between rounds, in the screens' style (ui-art-direction design 6 and 12): an opaque panel in the
/// middle, the outcome in its state's colour with its shape, the round's numbers, the damage control, the next round.
fn debrief(ctx: &egui::Context, v: &UiView, snap: &Snapshot) {
    use crate::look::{kit, look, Align, Role};
    let c = look().c();
    let (head, colour, icon) = match snap.outcome {
        Outcome::Victory => ("Victory", c.ok.0, "check"),
        Outcome::Defeat => ("The Tern is lost", c.danger.0, "bang"),
        Outcome::Withdrew => ("The Hound withdrew", c.warn.0, "bang"),
        Outcome::None => ("Debrief", c.text.0, "objectives"),
    };
    let st = &snap.stats;
    let rows = [
        ("Time", mmss(st.engage_s)),
        ("Turret hits", format!("{} of {}", st.tern_hits, st.tern_shots)),
        ("Missile hits", format!("{} of {}", st.missile_hits, st.missiles_fired)),
        ("Damage dealt", format!("{:.0} MJ", st.damage_dealt_mj)),
        ("Damage taken", format!("{:.0} MJ", st.damage_taken_mj)),
        ("Hull left", format!("{:.0} of {:.0} MJ", v.tern.as_ref().map_or(0.0, |t| t.hull), v.tern_hull)),
    ];
    // The teams put the ship right before the next round (ship-damage 9).
    let hurt = snap.damage.integrity.iter().filter(|i| f64::from(**i) < v.nominal_pct).count();
    let fires = snap.damage.fires.len();
    let repairs = match (hurt, fires) {
        (0, 0) => "Ship put right".to_owned(),
        (h, 0) => format!("Repairing {h} system{}", plural(h)),
        (0, f) => format!("Fighting {f} fire{}", plural(f)),
        (h, f) => format!("Repairing {h} system{}, fighting {f} fire{}", plural(h), plural(f)),
    };
    let next = if hurt + fires == 0 {
        (v.mission.debrief_s - snap.phase_s).max(0.0)
    } else {
        (v.mission.debrief_max_s - snap.phase_s).max(0.0)
    };
    let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Middle, egui::Id::new("debrief")));
    let screen = ctx.content_rect();
    let k = kit(&painter, screen, crate::ui::Anchor::Centre, Vec2::new(1040.0, 760.0));
    k.panel(0.0, 0.0, 1040.0, 760.0);
    k.bar(0.0, 0.0, 760.0);
    k.text(Role::Label, 64.0, 48.0, c.accent.0, &mission_label(&v.mission), Align::Start);
    let ty = 48.0 + k.line(Role::Label) + 12.0;
    k.icon(icon, 64.0 + 24.0, ty + k.line(Role::Display) / 2.0, 48.0, colour);
    k.text(Role::Display, 64.0 + 64.0, ty, colour, head, Align::Start);
    let mut y = ty + k.line(Role::Display) + 32.0;
    for (name, val) in
        rows.iter().map(|(a, b)| (*a, b.clone())).chain(std::iter::once(("Damage control", repairs.clone())))
    {
        k.text(Role::Body, 64.0, y, c.dim.0, name, Align::Start);
        let col = if name == "Damage control" && hurt + fires > 0 { c.warn.0 } else { c.text.0 };
        k.text(Role::Body, 400.0, y, col, &val, Align::Start);
        y += 52.0;
    }
    let when = if hurt + fires == 0 { "Next round in" } else { "Next round at the latest in" };
    k.text(
        Role::Label,
        64.0,
        760.0 - 56.0 - k.line(Role::Label),
        c.dim.0,
        &format!("{when} {next:.0} s"),
        Align::Start,
    );
}

#[cfg(test)]
mod tests {
    use super::{bridge_overlay, briefing, seat_prompt, BridgeUi, UiView};
    use crate::release::{check, texts_drawn};
    use egui::{Pos2, Rect};
    use sc_core::combat::bodies::Posture;
    use sc_core::combat::data::{DrillData, DRILL_FILES, DRILL_FILE_COUNT};
    use sc_core::combat::{Phase, Station};
    use sc_net::msg::{BodySnap, CrewEntry};

    fn crew(slot: u8, name: &str, station: Option<Station>, bot: bool) -> CrewEntry {
        CrewEntry { slot, name: name.into(), station, ready: false, bot }
    }

    fn body(slot: u8, going: Option<Station>) -> BodySnap {
        BodySnap { slot, pos: [0.0; 3], yaw: 0.0, posture: Posture::Walking, going, agent: 0.0, spraying: false }
    }

    #[test]
    fn a_seat_a_person_walks_to_is_not_offered_and_one_a_bot_walks_to_is() {
        // The server refuses a claim on a seat a person walks to (Station taken): the prompt must not promise it. A bot
        // on its way loses the seat to a person (relieve-bots 2), so the prompt offers it.
        let crew = [crew(1, "Me", None, false), crew(2, "Lt. Vor", None, true), crew(3, "Ana", None, false)];
        let bodies = [body(1, None), body(2, Some(Station::Captain)), body(3, Some(Station::Helm))];
        assert_eq!(seat_prompt(Station::Captain, 1, &crew, &bodies, "E").line(), "Captain [E] Relieve Lt. Vor");
        let p = seat_prompt(Station::Helm, 1, &crew, &bodies, "E");
        assert_eq!(p.line(), "Helm Ana is on the way to Helm");
        assert!(p.acts.is_empty(), "nothing for E to do, so USE stays dark");
    }

    #[test]
    fn a_free_seat_is_sat_in_and_an_npcs_is_taken_from_it() {
        let crew = [crew(1, "Me", None, false), crew(2, "Lt. Vor", Some(Station::Helm), true)];
        let bodies = [body(1, None), body(2, None)];
        assert_eq!(seat_prompt(Station::Captain, 1, &crew, &bodies, "E").line(), "Captain [E] Sit down");
        assert_eq!(seat_prompt(Station::Helm, 1, &crew, &bodies, "E").line(), "Helm [E] Relieve Lt. Vor");
    }

    #[test]
    fn a_players_seat_is_held_and_my_own_walk_does_not_block_me() {
        let crew = [crew(1, "Me", None, false), crew(3, "Ana", Some(Station::Tactical), false)];
        let bodies = [body(1, Some(Station::Captain))];
        assert_eq!(seat_prompt(Station::Tactical, 1, &crew, &bodies, "E").line(), "Tactical Ana holds Tactical");
        assert_eq!(seat_prompt(Station::Captain, 1, &crew, &bodies, "E").line(), "Captain [E] Sit down");
    }

    /// The mission `id` as the client loads it, from the repository's data.
    fn mission(id: &str) -> sc_core::combat::data::MissionFile {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let read = |rel: &str| std::fs::read_to_string(root.join(rel)).expect("the repository's data reads");
        let t: Vec<String> = DRILL_FILES.iter().map(|f| read(f)).collect();
        let arr: [&str; DRILL_FILE_COUNT] = std::array::from_fn(|i| t[i].as_str());
        let m = format!("data/missions/{id}.json");
        DrillData::parse(arr, &m, &read(&m)).expect("the mission parses").mission
    }

    /// All eight stations crewed, by the names in the Tamsin undock captures: the player at Helm, NPCs elsewhere.
    fn eight_crew() -> Vec<CrewEntry> {
        let names = [
            "Lt. Hurath of House Zarlen",
            "Lt. Sil Saren",
            "Lt. Jaya Strandraari",
            "Lt. Elaya Corregan",
            "Lt. Xa'lensha",
            "Lt. Kiya Ashberg",
            "Lt. Ni'lenru",
            "Lt. Haro Lundivan",
        ];
        Station::ALL
            .iter()
            .zip(names)
            .enumerate()
            .map(|(i, (s, n))| CrewEntry {
                slot: i as u8 + 1,
                name: n.into(),
                station: Some(*s),
                ready: i % 2 == 1,
                bot: i > 0,
            })
            .collect()
    }

    fn view(id: &str, me: Station) -> UiView {
        UiView {
            error: None,
            stage: None,
            station: Some(me),
            slot: Some(0),
            want: me,
            bot: false,
            ready: false,
            crew: eight_crew(),
            snap: None,
            tern: None,
            rtt_ms: None,
            loss: 0.0,
            refused: None,
            mission: mission(id),
            tern_hull: 0.0,
            nominal_pct: 0.0,
            bridge: None,
            ready_held: false,
            lowering: None,
        }
    }

    /// The output sizes the screens must fit (browser-page-fit design 5): 1080p, the Pi's 720p, a laptop, 1440p, 4:3,
    /// an ultrawide, itch's embed, a phone held sideways, and the undock video's 1280 x 678 window.
    const OUTPUTS: [(u32, u32); 9] = [
        (1920, 1080),
        (1280, 720),
        (1366, 768),
        (2560, 1440),
        (1024, 768),
        (2560, 1080),
        (985, 455),
        (844, 390),
        (1280, 678),
    ];

    #[test]
    fn the_walking_hud_is_one_objective_and_a_dot_with_no_roster_phase_or_clock() {
        // Before (ui-art-direction design 1): "Patrol: the Shoals   PATROL   00:03" and eight crew across the top, over
        // one another in the browser, and BRIDGE (V) along the bottom.
        let b = BridgeUi {
            labels: Vec::new(),
            plate: Some((crate::hud::Prompt::act("Helm", vec![crate::look::KeyTok::Key("E".into())], "Sit down"), 1.0)),
            touch: false,
            objective: Some("Report to the helm".into()),
            red_alert: false,
            agent: None,
            hint: Some(("Helm".into(), None)),
        };
        let bad = check("walking HUD", |ctx| bridge_overlay(ctx, &b));
        assert!(bad.is_empty(), "{bad:#?}");
        for px in OUTPUTS {
            let (drawn, points) = texts_drawn(px, |ctx| bridge_overlay(ctx, &b));
            let screen = Rect::from_min_size(Pos2::ZERO, points).expand(0.5);
            for d in &drawn {
                assert!(screen.contains_rect(d.rect), "{:?} at {:?} leaves the screen at {px:?}", d.text, d.rect);
            }
            let texts: Vec<&str> = drawn.iter().map(|d| d.text.as_str()).collect();
            assert!(texts.contains(&"OBJECTIVE") && texts.contains(&"Report to the helm"), "{texts:?} at {px:?}");
            assert!(texts.contains(&"Sit down") && texts.contains(&"E"), "the plate: key and verb at {px:?}");
            assert!(!texts.iter().any(|t| t.contains(':')), "no clock on the walking HUD at {px:?}: {texts:?}");
        }
    }

    #[test]
    fn the_briefing_lowers_into_the_desk_over_350_ms_and_the_console_rises_over_600() {
        // design 18a: the durations are the style's, the curves ease both ends.
        assert_eq!(super::briefing_lowered(0.0), 0.0);
        assert!((super::briefing_lowered(0.175) - 0.5).abs() < 1e-3, "half way down at half time");
        assert_eq!(super::briefing_lowered(0.35), 1.0, "gone at 350 ms");
        assert_eq!(super::console_risen(0.6), 1.0, "up at 600 ms");
        assert!(super::console_risen(0.06) < 0.01, "the console starts gently");
    }

    #[test]
    fn a_lowering_briefing_goes_down_whole_and_takes_no_presses() {
        // Half way down every word has dropped by the same amount, about half the screen under the band.
        let v = view("drill-hound", Station::Helm);
        let mut ask = Vec::new();
        let (up, size) = texts_drawn((1920, 1080), |ctx| briefing(ctx, &v, Phase::Countdown, 5.0, &mut ask, None, 0.0));
        let (down, _) = texts_drawn((1920, 1080), |ctx| briefing(ctx, &v, Phase::Countdown, 5.0, &mut ask, None, 0.5));
        assert!(!up.is_empty() && up.len() == down.len());
        let drop = down[0].rect.min.y - up[0].rect.min.y;
        assert!(drop > size.y * 0.4 && drop < size.y * 0.5, "half of the screen under the band: {drop} of {}", size.y);
        for (a, b) in up.iter().zip(&down) {
            assert!((b.rect.min.y - a.rect.min.y - drop).abs() < 0.5, "{} moves with the rest", a.text);
        }
        assert!(ask.is_empty(), "nothing was pressed");
    }

    #[test]
    fn the_briefing_fits_eight_crew_and_every_stations_orders() {
        // The undock video's briefing drew Laser port, Laser stbd and Gunner over one another, the claim buttons below;
        // the art direction's is a solid screen with one hierarchy and no (bot), automation or MARK READY.
        for id in ["patrol-shoals", "drill-hound"] {
            for me in Station::ALL {
                let v = view(id, me);
                let bad = check(&format!("briefing {id} {me:?}"), |ctx| {
                    briefing(ctx, &v, Phase::Muster, 1.0, &mut Vec::new(), None, 0.0)
                });
                assert!(bad.is_empty(), "{bad:#?}");
                let (drawn, _) =
                    texts_drawn((1920, 1080), |ctx| briefing(ctx, &v, Phase::Muster, 1.0, &mut Vec::new(), None, 0.0));
                for name in
                    ["Lt. Kiya Ashberg", "Lt. Haro Lundivan", "Gunner", "Starboard laser", "READY", "YOUR ORDERS"]
                {
                    assert!(drawn.iter().any(|d| d.text == name), "{name:?} is drawn ({id}, {me:?})");
                }
            }
        }
        let v = view("drill-hound", Station::Helm);
        let bad =
            check("briefing countdown", |ctx| briefing(ctx, &v, Phase::Countdown, 2.0, &mut Vec::new(), None, 0.0));
        assert!(bad.is_empty(), "{bad:#?}");
    }
}
