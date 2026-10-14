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

use egui::{Align2, Color32, CornerRadius, Pos2, Rect, RichText, Stroke, Vec2};
use glam::{DQuat, DVec3, Mat4, Quat, Vec3};
use sc_client::platform::{keys, App, Event, Flow, Frame, TouchPhase};
use sc_core::combat::data::{DrillData, DRILL_FILES};
use sc_core::combat::{Command, Outcome, Phase, Station, ENEMY_ID, FEEDS, TERN_ID};
use sc_core::exterior::{normalized, ExteriorData};
use sc_net::bot::Bot;
use sc_net::msg::{ShipSnap, Snapshot};
use sc_net::session::{Session, Stage, Visual, INTERP_DELAY_S};
use sc_net::transport::{Dial, Impair};
use sc_render::{DeckProgram, Indices, Mesh, Renderer, Target};

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
    /// Single player: All hands, no damage-control teams aboard; the crew put the ship right (`--all-hands`,
    /// all-hands 1).
    pub all_hands: bool,
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
    /// Fire on a room's floor, a puff of agent, the extinguisher in hand (all-hands 4).
    flame: Mesh,
    puff: Mesh,
    held: Mesh,
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
    /// Frames the film has seen, and the cue written last for the video's captions (chase-camera 3).
    film_tick: u64,
    film_cue: String,
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
    /// How long Stand has been held while seated, seconds.
    stand_hold: f64,
    /// Each station's seat (layout.json), for the seat in reach.
    seat_points: Vec<(Station, [f32; 3], f32)>,
    /// `--playtest`: the scripted player.
    playtest: Option<crate::playtest::Playtest>,
    /// When the body last sat down at a console, local seconds: the console rises from the desk from then.
    seated_at: Option<f64>,
    /// The eye the player last saw the ship from on their feet (where, facing, tilt), and the one the sit started from:
    /// the camera sits down with the body from there.
    last_view: Option<([f64; 3], f32, f32)>,
    rise_from: Option<([f64; 3], f32, f32)>,
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
}

/// The console comes up from the desk over this long once the body sits, seconds (the law: maximum immersion).
const CONSOLE_RISE_S: f64 = 0.6;

/// The walk goes to the server this often, seconds (20 a second: other players see a body walk, not hop).
const WALK_SEND_S: f64 = 0.05;

fn load_data() -> Result<DrillData, String> {
    let read = |rel: &str| std::fs::read_to_string(rel).map_err(|e| format!("{rel}: {e}"));
    let t: Vec<String> = DRILL_FILES.iter().map(|f| read(f)).collect::<Result<_, _>>()?;
    let arr: [&str; sc_core::combat::data::DRILL_FILE_COUNT] = std::array::from_fn(|i| t[i].as_str());
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
        let bridge_target = r.make_target(1280, 720, 4);
        let text = std::fs::read_to_string("data/space/exterior.json")
            .map_err(|e| format!("data/space/exterior.json: {e}"))?;
        let exterior: ExteriorData =
            sc_core::data::parse("data/space/exterior.json", &text).map_err(|e| e.to_string())?;
        let mut data = load_data()?;
        if args.all_hands && args.solo {
            data.mission.damage_control = "crew".into();
        }
        let hound = ships3d::hound();
        println!("sc-client: the Hound placeholder is {} triangles", hound.triangles());
        let meshes = Meshes {
            hound: mesh(&r, &hound),
            jackal: mesh(&r, &ships3d::jackal()),
            debris: mesh(&r, &ships3d::bolt([0.35, 0.33, 0.33])),
            bolt_tern: mesh(&r, &ships3d::bolt([0.6, 1.8, 2.0])),
            bolt_enemy: mesh(&r, &ships3d::bolt([2.0, 0.8, 0.4])),
            missile: mesh(&r, &ships3d::missile()),
            burst: mesh(&r, &ships3d::burst()),
            beam: mesh(&r, &ships3d::bolt([2.0, 0.6, 1.3])),
            flame: mesh(&r, &ships3d::flames()),
            puff: mesh(&r, &ships3d::puff()),
            held: mesh(&r, &ships3d::extinguisher()),
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
        let mut app = Self {
            r,
            blank,
            feed,
            bridge_target,
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
            beams: Vec::new(),
            booms: Vec::new(),
            debris: Vec::new(),
            held: HashSet::new(),
            pressed: HashSet::new(),
            seats: Station::ALL.iter().map(|_| Seat::default()).collect(),
            solo,
            shown: None,
            ready_held: false,
            pointer: Vec::new(),
            show_net: false,
            shots_taken: HashSet::new(),
            phase_seen: None,
            shot_round: None,
            done: false,
            film_frame: 0,
            film_screen: false,
            film_tick: 0,
            film_cue: String::new(),
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
            stand_hold: 0.0,
            seat_points: Vec::new(),
            playtest: None,
            seated_at: None,
            last_view: None,
            rise_from: None,
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
            refused_said: None,
        };
        match sc_core::repair::data::RepairData::load_for_ship(|rel| {
            std::fs::read_to_string(rel).map_err(|e| format!("{rel}: {e}"))
        }) {
            Ok(r) => app.repair_data = Some(r),
            Err(e) => println!("sc-client: no repairs on deck ({e})"),
        }
        if app.args.film && app.solo.is_some() {
            app.playtest = Some(crate::playtest::Playtest::film(app.args.station));
        } else if app.args.playtest && app.solo.is_some() {
            app.playtest = Some(crate::playtest::Playtest::new(app.args.station));
        }
        match crate::drill_bridge::BridgeView::load(&mut app.r, std::path::Path::new("compiled/tern.deck")) {
            Ok(mut b) => {
                println!("sc-client: the bridge for the crew on foot, {} triangles", b.triangles);
                b.fov_deg = app.options.fov_deg as f32;
                app.bridge = Some(b);
            }
            Err(e) => println!("sc-client: no bridge to walk ({e}); the drill shows the consoles only"),
        }
        // The player's own walk, on the same deck (a watcher has no body).
        if !app.args.watch && app.bridge.is_some() {
            let foot = std::fs::read("compiled/tern.deck").map_err(|e| e.to_string()).and_then(|b| {
                let d = sc_core::deck::read(&b).map_err(|e| e.to_string())?;
                crate::onfoot::OnFoot::from_deck(&d)
            });
            match foot {
                Ok(f) => app.foot = Some(f),
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
    /// to the server. Returns what E does here, for the screen.
    fn step_foot(
        &mut self,
        dt: f64,
        me: sc_net::msg::BodySnap,
        burning: &[usize],
        jobs: &[sc_core::combat::DeckJob],
    ) -> Option<String> {
        let seats = &self.data.stations.seats;
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
        if let (Some(st), true) = (seat, pressed.contains(&keys::E)) {
            pressed.remove(&keys::E);
            sit = Some(st);
        }
        // The damage-control locker in reach: E takes an extinguisher (all-hands 4). F, or the left button while looking
        // round, held, sprays it.
        let feet = foot.body.feet;
        let at_locker = self.data.tern_layout.lockers.iter().any(|l| {
            (l.x - f64::from(feet[0])).hypot(l.z - f64::from(feet[2])) <= f64::from(reach)
                && (l.y - f64::from(feet[1])).abs() < 1.5
        });
        let mut take = false;
        if at_locker && seat.is_none() && pressed.contains(&keys::E) {
            pressed.remove(&keys::E);
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
        if let (Some((k, _)), true, None) = (&reach_job, pressed.contains(&keys::E), seat) {
            pressed.remove(&keys::E);
            dock = Some(*k as u8);
        }
        let repairing = self.deck.is_some();
        let mut spray = held.contains(&keys::F) || self.mouse_left;
        let chest = glam::DVec3::new(f64::from(feet[0]), f64::from(feet[1]) + 1.2, f64::from(feet[2]));
        let rooms = &self.data.tern_layout.rooms;
        let in_fire = rooms.iter().position(|r| r.contains(chest)).is_some_and(|k| burning.contains(&k));
        let (input, lift_keys) = match self.playtest.as_mut() {
            Some(p) => {
                let layout = &self.data.tern_layout;
                let room = |id: &str| layout.room(id).map(|k| layout.rooms[k].floor.as_vec3().to_array());
                let fires: Vec<[f32; 3]> =
                    burning.iter().filter_map(|k| layout.rooms.get(*k)).map(|r| r.floor.as_vec3().to_array()).collect();
                let locker = layout.lockers.first().map(|l| l.as_vec3().to_array());
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
                let mut i = crate::onfoot::OnFoot::keys_input(&held, &pressed, self.look.0);
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
        // What E and F do here: sit (taking the seat from an NPC in it), take an extinguisher, spray the fire, or the
        // lift's and ladder's keys.
        let crew = self.session.as_ref().map(|s| s.crew.clone()).unwrap_or_default();
        let others = self.session.as_ref().and_then(|s| s.latest()).map(|s| s.bodies.clone()).unwrap_or_default();
        let agent = (me.agent * 100.0).round();
        match seat {
            Some(st) => Some(seat_prompt(st, me.slot, &crew, &others)),
            None if repairing => None,
            None if reach_job.is_some() => reach_job.as_ref().map(|(_, title)| format!("E  Repair: {title}")),
            None if at_locker && me.agent < 1.0 => Some("E  Take an extinguisher".to_owned()),
            None if in_fire && me.agent > 0.0 => Some(format!("FIRE   F  Spray it   (agent {agent:.0}%)")),
            None if in_fire => Some("FIRE   An extinguisher is in the damage control locker".to_owned()),
            None => foot
                .prompt()
                .map(str::to_owned)
                .or_else(|| (me.agent > 0.0).then(|| format!("Extinguisher {agent:.0}%"))),
        }
    }

    /// Seated, hold Stand to get up: E or Backspace, held `stand_hold_s` (a tap does nothing); at Helm, whose console
    /// rolls on E, Backspace.
    fn stand_up(&mut self, dt: f64, station: Option<Station>, posture: Option<sc_core::combat::bodies::Posture>) {
        let seated = posture == Some(sc_core::combat::bodies::Posture::Seated);
        let keys_held =
            self.held.contains(&keys::BACKSPACE) || (station != Some(Station::Helm) && self.held.contains(&keys::E));
        if !(seated && station.is_some() && self.bot.is_none() && !self.menu.open() && keys_held) {
            self.stand_hold = 0.0;
            return;
        }
        self.stand_hold += dt;
        if self.stand_hold >= self.data.stations.seats.stand_hold_s {
            if let Some(s) = self.session.as_ref() {
                s.stand();
            }
            self.stand_hold = f64::NEG_INFINITY;
        }
    }

    /// `--playtest`, seated: the script works the console (Engineering's schematic), marks READY in the briefing, and
    /// gets up when its beat says.
    fn playtest_seated(
        &mut self,
        dt: f64,
        station: Option<Station>,
        posture: Option<sc_core::combat::bodies::Posture>,
    ) {
        let (Some(st), Some(sc_core::combat::bodies::Posture::Seated)) = (station, posture) else { return };
        let fire = self.session.as_ref().and_then(|s| s.latest()).is_some_and(|s| !s.damage.fires.is_empty());
        let Some(p) = self.playtest.as_mut() else { return };
        let at = p.at_console(st, fire, dt);
        if st == Station::Engineering {
            match at.schematic {
                Some((three_d, turn)) => self.seats[st as usize].show_schematic(true, three_d, turn),
                None => self.seats[st as usize].show_schematic(false, false, 0.0),
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
            if let Some(s) = self.session.as_ref() {
                s.stand();
            }
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
        if self.touch.touch(id, phase, x, y) {
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
                let pt = self.ui.win_to_pt();
                self.console_finger.take(id, phase, Pos2::new(x * pt, y * pt), self.time_s);
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
        self.touch.set([f.window_w, f.window_h], mode);
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

    /// The camera on the Tern's bow: its eye in the system frame and its attitude.
    fn bow_eye(ships: Option<&(ShipSnap, ShipSnap, Snapshot)>, time_s: f64) -> (DVec3, DQuat) {
        match ships {
            Some((t, _, _)) => (t.pos + t.rot * DVec3::new(0.0, 3.0, 32.0), t.rot),
            None => (DVec3::ZERO, DQuat::from_rotation_y(time_s * 0.02)),
        }
    }

    /// The viewscreen's feed: from the bow, looking where the console's feed points (the mockup's feed camera:
    /// 70 degrees across at zoom 1), into its own target, which the console draws in the viewscreen's frame; CHASE
    /// stands off the ship behind it and draws the Tern too (chase-camera 1). With no console (on foot, the overview)
    /// it is the bridge's main viewscreen's picture: the captain's camera while he holds the viewscreen, his console's
    /// otherwise (chase-camera 2).
    fn render_feed(&mut self, ships: Option<&(ShipSnap, ShipSnap, Snapshot)>, view: Option<&console::ConsoleView>) {
        let (bow, rot) = Self::bow_eye(ships, self.time_s);
        let chase_cfg = &self.exterior.viewscreen.chase;
        let (stand, fwd, up, vfov) = match view {
            Some(v) => self.seats[self.console_station() as usize].feed_camera(v, rot, chase_cfg),
            None => {
                let feed = ships
                    .and_then(|(_, _, s)| s.bridge.view.or(Some(s.bridge.main_view)))
                    .and_then(|i| FEEDS.get(usize::from(i)))
                    .copied()
                    .unwrap_or("FWD");
                let target =
                    ships.filter(|(_, h, _)| h.active && h.alive).map(|(t, h, _)| t.rot.inverse() * (h.pos - t.pos));
                let vfov = 2.0 * (97.0f64 / (307.0 / 35f64.to_radians().tan())).atan();
                if feed == "CHASE" {
                    // The captain's own angles when this is his client; behind the ship otherwise.
                    let angles = self.seats[Station::Captain as usize].chase();
                    let vfov = console::chase_vfov(chase_cfg, 1.0);
                    let (e, b) = console::chase_camera(target, angles, chase_cfg, vfov / 2.0);
                    (Some(rot * e), rot * b.f, rot * b.u, vfov)
                } else {
                    let b = console::feed_basis_for(feed, target);
                    (None, rot * b.f, rot * b.u, vfov)
                }
            }
        };
        let centre = ships.map_or(DVec3::ZERO, |(t, _, _)| t.pos);
        let eye = stand.map_or(bow, |s| centre + s);
        let proj =
            glam::camera::rh::proj::opengl::perspective(vfov as f32, FEED.0 as f32 / FEED.1 as f32, 1.0, 80_000.0);
        let view_m = glam::camera::rh::view::look_to_mat4(Vec3::ZERO, fwd.as_vec3(), up.as_vec3());
        let world = World {
            meshes: &self.meshes,
            exterior: &self.exterior,
            bolts: &self.bolts,
            beams: &self.beams,
            booms: &self.booms,
            debris: &self.debris,
            time_s: self.time_s,
        };
        self.r.begin_3d(&self.feed, [0.0, 0.0, 0.0, 1.0]);
        world.draw(&mut self.r, &self.feed, proj, view_m, vfov as f32 / FEED.1 as f32, eye, ships);
        // CHASE sees the Tern from outside: its hull, drawn in the ship's frame from where the camera stands in it.
        if let (Some(s), Some(h)) = (stand, self.bridge.as_ref().and_then(|b| b.hull())) {
            let q = rot.as_quat();
            let in_ship = rot.inverse() * s;
            h.draw(&mut self.r, &self.feed, proj * view_m * Mat4::from_quat(q), in_ship.to_array());
        }
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
    beams: &'a [VisBeam],
    booms: &'a [Boom],
    debris: &'a [Debris],
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
            // The enemy's fighters (enemy-fighters 4), lit like the Hound.
            for f in &snap.fighters {
                let sun = Vec3::from_array(normalized(self.exterior.sun.dir));
                let axes = [Vec3::X, -Vec3::X, Vec3::Y, -Vec3::Y, Vec3::Z, -Vec3::Z];
                let cube = axes.map(|a| {
                    let s = a.dot(sun).max(0.0);
                    [0.2 + 1.3 * s, 0.2 + 1.25 * s, 0.22 + 1.15 * s]
                });
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
        if self.args.film {
            self.film(f, &snap);
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
    /// The film of the viewscreen's cameras (chase-camera 3). It follows the body: the walk to the captain's chair (every
    /// second frame), sitting down and the countdown (every fourth), both sped up; then, once the round is engaged, every
    /// frame at the video's 30 a second (the frame's step is held to 1/30 s from 1.5 s into Engage): the
    /// console's FWD, TARGET (from the bow, on the target), CHASE behind the ship, CHASE swung round it while it keeps
    /// the target in the middle and the ship low, and the bridge's main viewscreen showing the captain's CHASE. Frames
    /// go to the shots folder as `film-NNNNN.png`, and each shot's first frame and caption to `film-cues.txt`;
    /// `tools/video/film.sh` makes the video.
    fn film(&mut self, f: &Frame, snap: &Snapshot) {
        self.film_tick += 1;
        let seated = self.seated_at.is_some();
        let t = snap.phase_s;
        let engaged = snap.phase == Phase::Engage;
        let st = self.console_station() as usize;
        let (cue, every) = if !seated && !engaged {
            ("Walking to the captain's chair, on foot (sped up)", 2)
        } else if !engaged || t < 2.0 {
            ("Seated at Captain: the console comes up, the round counts down (sped up)", 4)
        } else {
            let seat = &mut self.seats[st];
            match t {
                t if t < 4.5 => {
                    seat.set_feed("FWD");
                    ("FWD: the captain's viewscreen from the bow, as before", 1)
                }
                t if t < 7.0 => {
                    seat.set_feed("TARGET");
                    ("TARGET: first person from the bow, the Hound in the middle", 1)
                }
                t if t < 10.0 => {
                    seat.set_feed("CHASE");
                    seat.set_chase([0.0, 0.0]);
                    ("CHASE: behind the Tern, looking at the Hound, the ship low on the screen", 1)
                }
                t if t < 18.0 => {
                    // Round the ship: out to port, back past the stern to starboard, rising and falling.
                    let a = std::f64::consts::TAU * (t - 10.0) / 8.0;
                    seat.set_chase([0.85 * a.sin(), 0.6 * (a * 0.5).sin() - 0.15 * (a * 1.5).sin()]);
                    ("CHASE swung round the ship (drag the camera widget): still on the Hound, ship low", 1)
                }
                t if t < 22.0 => {
                    seat.set_chase([0.35 * ((t - 18.0) * 0.6).sin(), 0.15]);
                    if !self.film_screen {
                        // The captain's camera on the main viewscreen, as pressing CHASE on his console sends it.
                        if let Some(s) = self.session.as_mut() {
                            s.command(sc_core::combat::Command::MainView(2));
                        }
                        self.film_screen = true;
                    }
                    ("The bridge's main viewscreen shows the captain's camera: CHASE", 1)
                }
                _ => {
                    self.film_screen = false;
                    self.done = true;
                    return;
                }
            }
        };
        if self.film_tick % every != 0 {
            return;
        }
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
        // The film plays its camera shots at the video's own 30 frames a second, whatever a frame takes to draw
        // (chase-camera 3); everything else steps by the frame, as a player's game does.
        let filming = self.args.film
            && self
                .session
                .as_ref()
                .and_then(|s| s.latest())
                .is_some_and(|s| s.phase == Phase::Engage && s.phase_s >= 1.5);
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
                self.step_foot(dt, me, &burning, &jobs)
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
        // What E does here, said once when it changes: a playtest's and the browser smoke test's view of the walk.
        if foot_prompt != self.prompt_said {
            if let Some(p) = foot_prompt.as_deref() {
                println!("sc-client: prompt: {}", p.split_whitespace().collect::<Vec<_>>().join(" "));
            }
            self.prompt_said = foot_prompt.clone();
        }
        // By thumb the prompt loses its key letters and USE carries its verb (touch-controls 3).
        let touching = self.touch.active();
        self.touch_down = foot_prompt.as_deref().is_some_and(|p| p.contains("Q  Down"));
        let (use_word, foot_prompt) = match foot_prompt {
            Some(p) if touching => {
                let (w, clean) = crate::touch::touch_prompt(&p);
                (w, Some(clean))
            }
            p => (None, p),
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
        let shot_overview = self.args.shots.is_some() && (self.shot_overview() || self.film_screen);
        let on_bridge = self.bridge.is_some() && !bodies.is_empty() && (walking || self.overview || shot_overview);
        // Sat down: the console rises from the desk, the bridge seen from the chair behind it until it is up.
        let seated = !walking && station.is_some() && mine.is_some();
        if seated && self.seated_at.is_none() {
            self.rise_from = self.last_view;
            println!("sc-client: sat at {}", station.map_or("a seat", |s| s.name()));
        }
        if !seated && self.seated_at.is_some() {
            println!("sc-client: stood up");
        }
        self.seated_at = if seated { self.seated_at.or(Some(self.time_s)) } else { None };
        let rise = self.seated_at.map_or(1.0, |t| ((self.time_s - t) / CONSOLE_RISE_S).min(1.0));
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
                    let k = (rise * rise * (3.0 - 2.0 * rise)) as f32;
                    let turn =
                        (to.1 - from.1 + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
                    let e: [f64; 3] = std::array::from_fn(|i| from.0[i] + (to.0[i] - from.0[i]) * f64::from(k));
                    (e, from.1 + turn * k, from.2 + (to.2 - from.2) * k)
                }
                (true, Some(b), None) => (crate::drill_bridge::eye_of(&b), b.yaw, -0.12),
                // The film's look at the bridge's main viewscreen: from behind the helm, up at the screen.
                _ if self.film_screen => ([0.0, 4.4, 24.5], 0.0, 0.08),
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
                let world = World {
                    meshes: &self.meshes,
                    exterior: &self.exterior,
                    bolts: &self.bolts,
                    beams: &self.beams,
                    booms: &self.booms,
                    debris: &self.debris,
                    time_s: self.time_s,
                };
                let seen = ships.as_ref();
                let out = crate::drill_bridge::Outside {
                    attitude,
                    lighting: if red { [0.0, 1.0, 0.0] } else { [1.0, 0.0, 0.0] },
                    screen: Some((&self.feed, self.exterior.viewscreen.scanlines as f32)),
                    objects: Box::new(move |r, t, vp| {
                        if let Some(e) = eye_out {
                            world.draw_objects(r, t, vp, e, seen);
                        }
                    }),
                };
                // The fires burning in the Tern's rooms, and this player's extinguisher (all-hands 4).
                let rooms = &self.data.tern_layout.rooms;
                let fires: Vec<[f32; 3]> = ships
                    .as_ref()
                    .map(|(_, _, snap)| {
                        snap.damage
                            .fires
                            .iter()
                            .filter_map(|r| rooms.get(usize::from(*r)))
                            .map(|r| r.floor.as_vec3().to_array())
                            .collect()
                    })
                    .unwrap_or_default();
                let inside = crate::drill_bridge::Inside {
                    fires: &fires,
                    flame: &self.meshes.flame,
                    puff: &self.meshes.puff,
                    held: &self.meshes.held,
                    carrying: mine.is_some_and(|b| b.agent > 0.0),
                    spraying: mine.is_some_and(|b| b.spraying),
                    time_s: self.time_s,
                };
                let vp = bv.draw(
                    &mut self.r,
                    &self.bridge_target,
                    &self.exterior,
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
                let (w, h) = (720.0 * aspect, 720.0);
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
                let banner =
                    foot_prompt.clone().or_else(|| mine.and_then(|b| b.going).map(|g| format!("To {}", g.name())));
                bridge_ui = (!rising).then_some(BridgeUi { labels, banner, overview: !own });
            }
        }
        let crew = self.session.as_ref().map(|s| s.crew.clone()).unwrap_or_default();
        // The console's view: the station this client holds (or wants), from the newest interpolated snapshot; in
        // single player the crewed station shown, with the crew's tabs.
        let st = self.console_station();
        let solo = self.solo.is_some();
        let mut console_view = ships.as_ref().filter(|_| !on_bridge).map(|(t, h, snap)| {
            let ops: Vec<(Station, String)> = crew
                .iter()
                .filter_map(|c| Some((c.station?, if c.bot && !solo { "Bot".to_owned() } else { c.name.clone() })))
                .collect();
            self.seats[st as usize].view(st, t, h, snap, &self.data, &ops, self.time_s)
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
        let mut view = UiView::gather(self, ships.as_ref());
        view.bridge = bridge_ui;
        let mut painted: Option<(console::kit::Hits, f32)> = None;
        let mut pointer: Vec<PointerEv> = Vec::new();
        let show_net = self.show_net;
        let place = format!("SCS Tern, {}", station.map_or("Bridge", |s| s.name()));
        let mut menu_ask = None;
        let (menu, options) = (&mut self.menu, &mut self.options);
        let deck = self.deck.as_ref();
        let (touch, pt) = (&self.touch, self.ui.win_to_pt());
        let finger_hover = self.console_finger.hover();
        let stand_share = (self.stand_hold / self.data.stations.seats.stand_hold_s).clamp(0.0, 1.0);
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
            draw_ui(ctx, &view, &mut ask, show_net);
            if let Some(d) = deck {
                let painter =
                    ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::new("deck-repair")));
                d.draw(&painter, ctx.content_rect().size());
            }
            let layer = ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::new("touch")));
            touch.paint(&layer, pt, use_word.as_deref());
            touch.paint_stand(&layer, pt, stand_share);
            menu_ask = menu.show(ctx, &place, options);
        });
        self.pointer = pointer;
        // A finger on the console in Engage, against this frame's controls (touch-controls 4).
        let engage_now = ships.as_ref().is_some_and(|x| x.2.phase == Phase::Engage);
        self.console_touch =
            self.touch_mode == crate::touch::Mode::Seated && engage_now && !self.menu.open() && painted.is_some();
        match painted.as_ref().filter(|_| self.console_touch) {
            Some((hits, _)) => {
                let evs = self.console_finger.resolve(hits, self.ui.win_to_pt(), self.time_s);
                self.pointer.extend(evs);
            }
            None => self.console_finger.clear(),
        }
        if let Some(b) = self.bridge.as_mut() {
            b.fov_deg = self.options.fov_deg as f32;
        }
        // Return to menu leaves the crew for the front menu (the station passes to automation, design 2); Exit closes
        // the game. Each was asked Yes or No first.
        match menu_ask {
            Some(crate::game_menu::MenuAsk::Leave) if self.args.menu => crate::back_to_menu(None),
            Some(crate::game_menu::MenuAsk::Leave | crate::game_menu::MenuAsk::Exit) => return Flow::Done,
            _ => {}
        }
        // The seated player's hands on the console, in Engage only (the briefing and debrief sit over it otherwise).
        let engage = ships.as_ref().is_some_and(|x| x.2.phase == Phase::Engage);
        let pressed = std::mem::take(&mut self.pressed);
        let engage = engage && !self.menu.open();
        if let (true, Some(_), None, Some((hits, scale)), Some(v), Some((t, h, _))) =
            (engage, station, self.bot.as_ref(), painted.as_ref(), console_view.as_ref(), ships.as_ref())
        {
            self.solo_keys(&pressed);
            let input = Input { events: std::mem::take(&mut self.pointer), held: self.held.clone(), pressed };
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
        // The viewscreen's feed in its window under the console, or black when there is no console yet.
        let ppp = f.height as f32 / frame.points[1].max(1.0);
        match console_view {
            None if on_bridge => {
                self.r.present_with_ui(&self.bridge_target, f.width, f.height, true, &frame.meshes(), frame.points)
            }
            // The console coming up from the desk: the chair's view behind it, the console eased up from below.
            Some(_) if rising => {
                let dy = ((1.0 - rise) * (1.0 - rise)) as f32 * frame.points[1];
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

/// What E offers at seat `st` for the body in `slot`, by the server's claim rule (sc-core's `claim_on_foot`): a seat
/// another body walks to is theirs, so it is not offered; an NPC's is taken from it; a player's is held; a free one is
/// sat in. The prompt never promises a seat the server would refuse (CLAUDE.md 6.1: the preview is the outcome).
fn seat_prompt(st: Station, slot: u8, crew: &[sc_net::msg::CrewEntry], bodies: &[sc_net::msg::BodySnap]) -> String {
    if let Some(b) = bodies.iter().find(|b| b.slot != slot && b.going == Some(st)) {
        let name = crew.iter().find(|c| c.slot == b.slot).map_or("Someone", |c| c.name.as_str());
        return format!("{name} is on the way to {}", st.name());
    }
    match crew.iter().find(|c| c.station == Some(st)) {
        Some(c) if c.bot => format!("E  Take {} from {}", st.name(), c.name),
        Some(c) => format!("{} holds {}", c.name, st.name()),
        None => format!("E  Sit at {}", st.name()),
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
    /// A system at or above this integrity is at nominal (ship-damage 9: the debrief waits for it).
    nominal_pct: f64,
    /// The bridge on screen instead of the consoles (design 9).
    bridge: Option<BridgeUi>,
    /// Single player: READY pressed, waiting for the crew to sit.
    ready_held: bool,
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

struct BridgeUi {
    labels: Vec<Label>,
    banner: Option<String>,
    overview: bool,
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
            nominal_pct: app.data.damage.systems.nominal_from_pct,
            bridge: None,
            ready_held: app.ready_held,
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
    if let Some(b) = &v.bridge {
        bridge_overlay(ctx, v, snap, b, ask);
        return;
    }
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

/// The bridge on screen (design 9): a band along the top with the drill, its phase and clock and who holds each
/// station, a name over every body with where it is walking, and this player's own walk said once in the middle.
fn bridge_overlay(ctx: &egui::Context, v: &UiView, snap: &Snapshot, b: &BridgeUi, _ask: &mut Vec<UiAsk>) {
    use egui::FontId;
    let w = ctx.content_rect().width();
    let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Middle, egui::Id::new("bridge-names")));
    painter.rect_filled(Rect::from_min_size(Pos2::ZERO, Vec2::new(w, 34.0)), CornerRadius::ZERO, PANEL);
    let phase = format!("{:?}", snap.phase).to_uppercase();
    let head = format!("{}   {}   {}", v.mission.title, phase, mmss(snap.phase_s));
    painter.text(Pos2::new(16.0, 17.0), Align2::LEFT_CENTER, head, FontId::proportional(17.0), TEXT);
    let held: Vec<String> = Station::ALL
        .iter()
        .map(|s| {
            let who = v.crew.iter().find(|c| c.station == Some(*s)).map_or("auto".to_owned(), |c| c.name.clone());
            format!("{} {}", s.name(), who)
        })
        .collect();
    painter.text(Pos2::new(w - 16.0, 17.0), Align2::RIGHT_CENTER, held.join("   "), FontId::proportional(14.0), DIM);
    for l in &b.labels {
        crate::nameplate::draw(&painter, l.at, &l.name, l.npc, l.going.as_deref(), l.k);
    }
    if let Some(t) = &b.banner {
        painter.text(Pos2::new(w / 2.0, 560.0), Align2::CENTER_CENTER, t, FontId::proportional(30.0), AMBER);
    }
    if b.overview {
        painter.text(Pos2::new(w / 2.0, 700.0), Align2::CENTER_CENTER, "BRIDGE (V)", FontId::proportional(14.0), DIM);
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
            label(ui, &format!("- {}", o.words), 17.0, TEXT);
        }
        ui.add_space(10.0);
        let mine = v.station.unwrap_or(v.want);
        let orders = v.mission.orders.of(mine);
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
                let text = if v.ready {
                    "READY"
                } else if v.ready_held {
                    "CREW SITTING"
                } else {
                    "MARK READY"
                };
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

/// An "s" for more than one.
fn plural(n: usize) -> &'static str {
    if n == 1 {
        ""
    } else {
        "s"
    }
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
        // The teams put the ship right before the next drill (ship-damage 9).
        let hurt = snap.damage.integrity.iter().filter(|i| f64::from(**i) < v.nominal_pct).count();
        let fires = snap.damage.fires.len();
        let repairs = match (hurt, fires) {
            (0, 0) => "Ship put right".to_owned(),
            (h, 0) => format!("Repairing {h} system{}", plural(h)),
            (0, f) => format!("Fighting {f} fire{}", plural(f)),
            (h, f) => format!("Repairing {h} system{}, fighting {f} fire{}", plural(h), plural(f)),
        };
        ui.horizontal(|ui| {
            ui.add_sized(
                Vec2::new(200.0, 26.0),
                egui::Label::new(RichText::new("Damage control").size(18.0).color(DIM)),
            );
            label(ui, &repairs, 20.0, if hurt + fires == 0 { GREEN } else { AMBER });
        });
        ui.add_space(14.0);
        let next = if hurt + fires == 0 {
            (v.mission.debrief_s - snap.phase_s).max(0.0)
        } else {
            (v.mission.debrief_max_s - snap.phase_s).max(0.0)
        };
        let when = if hurt + fires == 0 { "Next drill in" } else { "Next drill at the latest in" };
        label(ui, &format!("{when} {next:.0} s"), 18.0, AMBER);
    });
}

#[cfg(test)]
mod tests {
    use super::seat_prompt;
    use sc_core::combat::bodies::Posture;
    use sc_core::combat::Station;
    use sc_net::msg::{BodySnap, CrewEntry};

    fn crew(slot: u8, name: &str, station: Option<Station>, bot: bool) -> CrewEntry {
        CrewEntry { slot, name: name.into(), station, ready: false, bot }
    }

    fn body(slot: u8, going: Option<Station>) -> BodySnap {
        BodySnap { slot, pos: [0.0; 3], yaw: 0.0, posture: Posture::Walking, going, agent: 0.0, spraying: false }
    }

    #[test]
    fn a_seat_someone_walks_to_is_not_offered() {
        // The server refuses a claim on a seat another body walks to (Station taken): the prompt must not promise it.
        let crew = [crew(1, "Me", None, false), crew(2, "Lt. Vor", None, true)];
        let bodies = [body(1, None), body(2, Some(Station::Captain))];
        let p = seat_prompt(Station::Captain, 1, &crew, &bodies);
        assert_eq!(p, "Lt. Vor is on the way to Captain");
        assert!(!p.starts_with("E  "), "nothing for E to do, so USE stays dark");
    }

    #[test]
    fn a_free_seat_is_sat_in_and_an_npcs_is_taken_from_it() {
        let crew = [crew(1, "Me", None, false), crew(2, "Lt. Vor", Some(Station::Helm), true)];
        let bodies = [body(1, None), body(2, None)];
        assert_eq!(seat_prompt(Station::Captain, 1, &crew, &bodies), "E  Sit at Captain");
        assert_eq!(seat_prompt(Station::Helm, 1, &crew, &bodies), "E  Take Helm from Lt. Vor");
    }

    #[test]
    fn a_players_seat_is_held_and_my_own_walk_does_not_block_me() {
        let crew = [crew(1, "Me", None, false), crew(3, "Ana", Some(Station::Tactical), false)];
        let bodies = [body(1, Some(Station::Captain))];
        assert_eq!(seat_prompt(Station::Tactical, 1, &crew, &bodies), "Ana holds Tactical");
        assert_eq!(seat_prompt(Station::Captain, 1, &crew, &bodies), "E  Sit at Captain");
    }
}
