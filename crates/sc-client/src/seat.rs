//! A seat at a drill station: the console's own view state, the [`ConsoleView`] built from each snapshot, and what a
//! press, a drag, a hold, a scroll or a key on the console sends (openspec/changes/console-parity, design 5).
//!
//! The console is the mockup's (`console`); this is the drill's side of it. What the console shows of the ship comes
//! from the snapshot and from `sc-core`'s own rules (the slew a helm order flies, where the helm is told to point,
//! the lock, the seeker's cone), so a preview is the outcome (CLAUDE.md 6.1). What is the seat's alone (the
//! viewscreen's feed, the scanners' range and camera, the thumbwheels before GO) stays here. A control maps to the
//! mockup's action name and from there to a `Command`: the same commands a bot or automation sends.

use crate::console::kit::Hit;
use crate::console::{self, Act, Cam, CapView, ComposerView, ConsoleView, ContactView, CrewAt, CrewRow, FlightView};
use crate::console::{HitView, HoldView, Hpr, OrderView, PlanView, ScanCam, ScanCams, SciView, TubeView, TurretView};
use egui::{Pos2, Rect};
use glam::{DQuat, DVec3};
use sc_client::platform::keys;
use sc_core::combat::attitude::{self, from_hpr, hpr_of, whole_deg};
use sc_core::combat::data::DrillData;
use sc_core::combat::data::BANDS;
use sc_core::combat::{self as combat, Command, HelmMode, Station, TubeState, ENEMY_ID, FEEDS};
use sc_net::msg::{ShipSnap, Snapshot};
use std::collections::HashSet;

/// The scanners' range ladder, km: 5 km steps from 5 to 100 (bridge-stations 8.0).
const RANGE_KM: [f64; 20] =
    [5., 10., 15., 20., 25., 30., 35., 40., 45., 50., 55., 60., 65., 70., 75., 80., 85., 90., 95., 100.];
/// How long a guarded control is held before it acts (bridge-stations 8.5).
pub const HOLD_S: f64 = 0.6;
/// How long a hit on the Tern's shield is drawn (weapons-and-shields 11: the last 8 s of hits).
const HIT_SHOW_S: f64 = 8.0;
/// The id the console's first hostile track takes.
const TRACK: &str = "T1";
/// The shield view's usual camera (the mockup's).
const SHIELD_CAM: Cam = Cam { yaw: 62.0, el: 24.0 };
/// The captain's CREW rows: the bridge's stations in the mockup's order.
const CREW_ROLES: [&str; 6] = ["helm", "tactical", "engineering", "science", "comms", "flight_ops"];

/// A drag in progress.
struct Dragging {
    what: console::kit::Drag,
    p0: Pos2,
    track: Rect,
    cam0: (f64, f64),
    v0: f64,
    side: f64,
    moved: bool,
}

/// A recent hit on the Tern, for the shield view's arrows.
struct RecentHit {
    t: f64,
    d_local: DVec3,
    face: u8,
    through: bool,
}

/// The seat's own state.
pub struct Seat {
    feed: String,
    zoom: f64,
    nav_km: f64,
    tac_km: f64,
    sci_km: f64,
    /// Science's picked contact, and its shield view's camera.
    sci_sel: Option<String>,
    shield_cam: Cam,
    /// The captain's tab (CMD or SHIP), the room picked on the SHIP tab, and the order being written.
    cap_tab: String,
    ship_sel: String,
    composer: (String, Option<String>),
    scan_cam: ScanCams,
    feed_cam: Cam,
    oe: Hpr,
    /// The helm's speed set point and strafe as this seat sets them (sent as commands).
    pub speed_set: f64,
    strafe: [f64; 2],
    synced_round: Option<u32>,
    /// The stick from the drag controls, [yaw to starboard, pitch up, roll right].
    stick_drag: [f64; 3],
    /// The stick now, the keys and the drag together.
    stick_now: [f64; 3],
    /// The strafe last sent (the pad's, or the keys' while held).
    strafe_sent: [f64; 2],
    /// A guarded control being held: its key, since when, and whether a key (not the pointer) holds it.
    hold: Option<(String, f64, bool)>,
    drag: Option<Dragging>,
    last_send: f64,
    hits: Vec<RecentHit>,
    face_hit: [f64; 6],
}

/// Things the console asks the drill to do besides commands.
pub enum SeatAsk {
    /// Send this.
    Command(Command),
    /// Take the other station.
    Claim(Station),
}

impl Default for Seat {
    fn default() -> Self {
        Seat {
            feed: "FWD".into(),
            zoom: 1.0,
            nav_km: 20.0,
            tac_km: 5.0,
            sci_km: 20.0,
            sci_sel: Some(TRACK.into()),
            shield_cam: SHIELD_CAM,
            cap_tab: "CMD".into(),
            ship_sel: "engineering".into(),
            composer: ("helm".into(), None),
            scan_cam: ScanCams::default(),
            feed_cam: Cam { yaw: 40.0, el: 30.0 },
            oe: Hpr { h: 45.0, p: 10.0, r: -30.0 },
            speed_set: 0.0,
            strafe: [0.0; 2],
            synced_round: None,
            stick_drag: [0.0; 3],
            stick_now: [0.0; 3],
            strafe_sent: [0.0; 2],
            hold: None,
            drag: None,
            last_send: 0.0,
            hits: Vec::new(),
            face_hit: [-1e9; 6],
        }
    }
}

fn mode_word(m: HelmMode) -> &'static str {
    match m {
        HelmMode::Hold => "HOLD",
        HelmMode::Course => "COURSE",
        HelmMode::Chase => "CHASE",
        HelmMode::Match => "MATCH",
        HelmMode::Evade => "EVADE",
    }
}

fn mode_of(w: &str) -> HelmMode {
    match w {
        "COURSE" => HelmMode::Course,
        "CHASE" => HelmMode::Chase,
        "MATCH" => HelmMode::Match,
        "EVADE" => HelmMode::Evade,
        _ => HelmMode::Hold,
    }
}

fn arr(v: DVec3) -> [f64; 3] {
    v.to_array()
}

impl Seat {
    /// A hit on the Tern's face `face` by a bolt flying `vel`, at local time `now`.
    pub fn hit(&mut self, tern: &ShipSnap, face: u8, vel: DVec3, now: f64) {
        let d_local = (tern.rot.inverse() * -vel).normalize_or(DVec3::Z);
        let f = usize::from(face.min(5));
        self.face_hit[f] = now;
        let through = tern.faces[f] < 0.5;
        self.hits.push(RecentHit { t: now, d_local, face, through });
        self.hits.retain(|h| now - h.t < HIT_SHOW_S);
    }

    /// The console's view of the drill now. `ops` is who sits at each station (a bot shows as "Bot"); the rest are
    /// held by automation, and this seat can swap to them.
    #[allow(clippy::too_many_arguments)]
    pub fn view(
        &mut self,
        station: Station,
        tern: &ShipSnap,
        hound: &ShipSnap,
        snap: &Snapshot,
        data: &DrillData,
        ops: &[(Station, String)],
        now: f64,
    ) -> ConsoleView {
        let op = ops.iter().find(|(s, _)| *s == station).map(|(_, n)| n.clone());
        let open = Station::ALL
            .iter()
            .filter(|o| **o != station && !ops.iter().any(|(s, _)| s == *o))
            .map(|o| o.id().to_owned())
            .collect();
        let br = &snap.bridge;
        if self.synced_round != Some(snap.round) {
            self.synced_round = Some(snap.round);
            self.speed_set = tern.speed_set;
            self.strafe = tern.strafe;
        }
        let fl = &data.tern_flight;
        let lim = fl.rate_limit_deg_s.map(f64::to_radians);
        let inv = tern.rot.inverse();
        let hound_up = hound.active && hound.alive;
        let waypoint = DVec3::from_array(data.mission.waypoint_m);
        let goal = combat::helm_goal(
            tern.helm_mode,
            tern.order,
            tern.pos,
            waypoint,
            hound_up.then_some((hound.pos, hound.vel)),
        );
        let is_helm = station == Station::Helm;
        // The rates the helm asks for: the slew to where it is told to point, else this seat's stick.
        let wset = match goal {
            Some(g) => attitude::slew(tern.rot, tern.rates, g, fl).w,
            None if is_helm => {
                let s = self.stick_now;
                DVec3::new(-s[1] * lim[1], -s[0] * lim[0], s[2] * lim[2])
            }
            None => DVec3::ZERO,
        };
        let plan = tern.order.map(|o| {
            let p = attitude::slew(tern.rot, tern.rates, o, fl);
            PlanView { time: p.time_s, angle: p.angle_deg, held: p.held }
        });
        let pv = attitude::slew(tern.rot, tern.rates, from_hpr(self.oe.h, self.oe.p, self.oe.r), fl);
        let contacts: Vec<ContactView> = if hound_up {
            vec![ContactView {
                id: TRACK.into(),
                cls: "corvette".into(),
                iff: "hostile".into(),
                rel: arr(inv * (hound.pos - tern.pos)),
                dir: arr(inv * (hound.rot * DVec3::Z)),
                hull: hound.hull / data.enemy.combat.hull_mj.max(1e-9) * 100.0,
                scan: br.scan,
            }]
        } else {
            Vec::new()
        };
        let locked_on = tern.lock_target == Some(ENEMY_ID) && hound_up;
        let tubes_cfg = data.tern_combat.tubes.as_ref();
        let (load_s, arm_s) = tubes_cfg.map_or((1.0, 0.0), |t| (t.load_s, t.arm_s));
        let seeker = tubes_cfg.map_or(30.0, |t| data.missile(&t.missile).seeker_half_angle_deg);
        let off_bow = combat::off_bow_deg(tern.pos, tern.rot, hound.pos);
        let in_cone = hound_up && off_bow <= seeker;
        let tubes: Vec<TubeView> = tern
            .tubes
            .iter()
            .map(|(st, left)| match st {
                TubeState::Empty => TubeView { state: "EMPTY".into(), t: 0.0 },
                TubeState::Loading => TubeView { state: "LOADING".into(), t: load_s - left },
                TubeState::Arming => TubeView { state: "LOADING".into(), t: load_s + arm_s - left },
                TubeState::Armed => TubeView { state: "READY".into(), t: 0.0 },
            })
            .collect();
        let (fire_note, fire_ok) = if !locked_on {
            ("NO LOCK".to_owned(), false)
        } else if tern.lock_frac < 1.0 {
            (format!("LOCK {}%", (tern.lock_frac * 100.0).floor() as i64), false)
        } else if !in_cone {
            ("OUT OF CONE".to_owned(), false)
        } else {
            ("LOCKED".to_owned(), true)
        };
        let gun_shorts =
            data.tern_combat.turrets.iter().map(|t| t.id.chars().next().unwrap_or('?').to_ascii_uppercase());
        let turrets: Vec<TurretView> = tern
            .turrets
            .iter()
            .zip(gun_shorts)
            .map(|(t, short)| TurretView {
                short: short.to_string(),
                mode: t.mode.word().into(),
                op: None,
                heat: t.heat,
                aim_deg: console::brg_of(inv * t.aim),
                firing: t.firing,
                cooling: t.cooling,
                ok: 1.0,
            })
            .collect();
        let caps =
            data.tern_shields.presets[usize::from(tern.preset).min(data.tern_shields.presets.len() - 1)].faces_mj;
        let preset_id =
            &data.tern_shields.presets[usize::from(tern.preset).min(data.tern_shields.presets.len() - 1)].id;
        let preset = match preset_id.as_str() {
            "bow" => 0,
            "stern" => 1,
            "port" => 2,
            "starboard" => 3,
            _ => -1,
        };
        self.hits.retain(|h| now - h.t < HIT_SHOW_S);
        let stick = if is_helm { self.stick_now } else { [0.0; 3] };
        let verb_of = |to: Station, i: u8| data.order_verbs(to).get(usize::from(i)).map(|v| v[0].clone());
        let order =
            br.orders.iter().find(|(to, _, done)| *to == station && !done).and_then(|(to, v, _)| verb_of(*to, *v));
        // The captain's camera on every viewscreen while it is taken.
        let feed = br.view.and_then(|i| FEEDS.get(usize::from(i))).map_or(self.feed.clone(), |f| (*f).to_owned());
        let sci = SciView {
            sel: self.sci_sel.clone(),
            scanning: br.scanning,
            ping: br.ping_s,
            cam: self.shield_cam,
            freq: BANDS[usize::from(br.band.min(3))].into(),
            by: br.shields_by.unwrap_or(Station::Science).name().into(),
        };
        let captain = CapView {
            tab: self.cap_tab.clone(),
            ship_sel: self.ship_sel.clone(),
            braced: br.braced,
            view_override: br.view.is_some(),
            composer: ComposerView { to: self.composer.0.clone(), verb: self.composer.1.clone() },
            orders: br
                .orders
                .iter()
                .filter_map(|(to, v, done)| {
                    let verb = verb_of(*to, *v)?;
                    Some(OrderView { to: to.id().into(), verb, state: if *done { "done" } else { "sent" }.into() })
                })
                .collect(),
            crew: CREW_ROLES
                .iter()
                .map(|r| {
                    let st = Station::from_id(r);
                    CrewRow {
                        role: (*r).into(),
                        op: st.and_then(|st| ops.iter().find(|(s, _)| *s == st).map(|(_, n)| n.clone())),
                        order: st.is_some_and(|st| br.orders.iter().any(|(to, _, done)| *to == st && !done)),
                    }
                })
                .collect(),
            // No compartment is simulated in the drill: every room reads OK, and the crew are seated on the bridge.
            rooms: Vec::new(),
            crew_at: ops.iter().map(|_| CrewAt { comp: Some("bridge".into()), ok: true }).collect(),
            room: None,
        };
        let role = if station == Station::Captain { "command" } else { station.id() };
        ConsoleView {
            station: station.id().into(),
            word: station.id().to_uppercase(),
            role: role.into(),
            op,
            open,
            order,
            alert: if br.red_alert { "red_alert" } else { "normal" }.into(),
            red_alert: br.red_alert,
            clock_s: snap.phase_s,
            hull_pct: tern.hull / data.tern_combat.hull_mj.max(1e-9) * 100.0,
            shield_ratio: std::array::from_fn(|f| tern.faces[f] / caps[f].max(1e-9)),
            power_mw: None,
            reactor_up: true,
            speed_mps: tern.vel.length(),
            fires: 0,
            breaches: 0,
            q: console::quat_arr(tern.rot),
            w: arr(tern.rates),
            wset: arr(wset),
            v_body: arr(inv * tern.vel),
            speed_set: if is_helm { self.speed_set } else { tern.speed_set },
            strafe: if is_helm { self.strafe } else { tern.strafe },
            stick,
            oe: self.oe,
            order_active: tern.order.is_some(),
            plan,
            preview: PlanView { time: pv.time_s, angle: pv.angle_deg, held: false },
            goal: goal.map(console::quat_arr),
            ap: mode_word(tern.helm_mode).into(),
            flight: FlightView {
                rate_dps: [fl.rate_limit_deg_s[1], fl.rate_limit_deg_s[0], fl.rate_limit_deg_s[2]],
                speed_min_mps: fl.speed_min_mps,
                speed_max_mps: fl.speed_max_mps,
                strafe_max_mps: fl.strafe_max_mps,
            },
            contacts,
            waypoint: Some(arr(inv * (waypoint - tern.pos))),
            target: locked_on.then(|| TRACK.into()),
            tubes_bear: in_cone,
            missiles: snap.missiles.iter().map(|m| arr(inv * (m.pos - tern.pos))).collect(),
            inbound: Vec::new(),
            hits: self
                .hits
                .iter()
                .map(|h| HitView { d: arr(h.d_local), face: h.face, age_s: now - h.t, through: h.through })
                .collect(),
            flash_age_s: std::array::from_fn(|f| now - self.face_hit[f]),
            feed,
            zoom: self.zoom,
            nav_km: self.nav_km,
            tac_km: self.tac_km,
            sci_km: self.sci_km,
            scan_cam: self.scan_cam,
            feed_cam: self.feed_cam,
            turrets,
            tubes,
            tube_load_s: load_s + arm_s,
            magazine: u32::from(tern.magazine),
            magazine_slots: tubes_cfg.map_or(0, |t| t.magazine),
            fire_ring: if locked_on { tern.lock_frac } else { 0.0 },
            fire_note,
            fire_ok,
            tube_cone_deg: seeker,
            charge: tern.faces,
            cap: caps,
            preset,
            hold: self
                .hold
                .as_ref()
                .map(|(k, t0, _)| HoldView { key: k.clone(), k: ((now - t0) / HOLD_S).clamp(0.0, 1.0) }),
            t: now,
            sci: Some(sci),
            captain: Some(captain),
            // No power grid in the drill yet: Engineering's console is unavailable (console-parity 10.2).
            eng: None,
        }
    }

    /// The viewscreen's camera: forward and up in the system frame, and the vertical field of view in radians for an
    /// image `aspect` wide over high (the mockup's feed: 70 degrees across at zoom 1, centred on the screen).
    pub fn feed_camera(&self, v: &ConsoleView, tern_rot: DQuat) -> (DVec3, DVec3, f64) {
        let b = console::feed_basis(v);
        let foc = 307.0 / 35f64.to_radians().tan() * v.zoom;
        (tern_rot * b.f, tern_rot * b.u, 2.0 * (97.0 / foc).atan())
    }

    fn act(
        &mut self,
        a: Act,
        v: &ConsoleView,
        tern: &ShipSnap,
        hound: &ShipSnap,
        data: &DrillData,
        out: &mut Vec<SeatAsk>,
    ) {
        let cmd = |c| SeatAsk::Command(c);
        let presets = &data.tern_shields.presets;
        match a {
            Act::Swap(id) => {
                if let Some(st) = Station::from_id(&id) {
                    out.push(SeatAsk::Claim(st));
                }
            }
            Act::Ack => {
                if let Some(st) = Station::from_id(&v.station) {
                    out.push(cmd(Command::Ack(st)));
                }
            }
            Act::AllStop => {
                self.speed_set = 0.0;
                self.strafe = [0.0; 2];
                self.strafe_sent = [0.0; 2];
                out.push(cmd(Command::AllStop));
            }
            Act::Ap(k) => {
                // As the mockup: leaving EVADE takes its jinks off the strafe pad.
                if v.ap == "EVADE" && k != "EVADE" {
                    self.strafe = [0.0; 2];
                }
                out.push(cmd(Command::Helm(mode_of(&k))));
            }
            Act::OGo => {
                if v.order_active {
                    out.push(cmd(Command::Orient(None)));
                } else {
                    out.push(cmd(Command::Orient(Some([self.oe.h, self.oe.p, self.oe.r]))));
                }
            }
            Act::OLevel | Act::OFlip | Act::OTarget => {
                let (h, p, r, _) = hpr_of(tern.rot);
                let hpr = match a {
                    Act::OLevel => Some([h, 0.0, 0.0]),
                    Act::OFlip => Some([h + 180.0, p, r]),
                    _ => (hound.active && hound.alive).then(|| {
                        let (th, tp, _, _) = hpr_of(attitude::bow_on(hound.pos - tern.pos));
                        [th, tp, 0.0]
                    }),
                };
                if let Some([h, p, r]) = hpr {
                    self.oe = Hpr { h: whole_deg('h', h), p: whole_deg('p', p), r: whole_deg('r', r) };
                    out.push(cmd(Command::Orient(Some([self.oe.h, self.oe.p, self.oe.r]))));
                }
            }
            Act::Target(id) => {
                if id == TRACK {
                    out.push(cmd(Command::Lock(Some(ENEMY_ID))));
                }
            }
            Act::Range(key, d) => {
                let km = match key {
                    "nav" => &mut self.nav_km,
                    "tac" => &mut self.tac_km,
                    _ => &mut self.sci_km,
                };
                let i = RANGE_KM.iter().position(|r| (r - *km).abs() < 1e-9).unwrap_or(3) as i64;
                *km = RANGE_KM[(i + i64::from(d)).clamp(0, RANGE_KM.len() as i64 - 1) as usize];
            }
            Act::ScanReset(key) => match key {
                "nav" => self.scan_cam.nav = ScanCam::default(),
                "tac" => self.scan_cam.tac = ScanCam::default(),
                _ => self.scan_cam.sci = ScanCam::default(),
            },
            Act::TMode(i) => {
                if let Some(t) = tern.turrets.get(i) {
                    out.push(cmd(Command::TurretMode(i as u8, t.mode.next())));
                }
            }
            Act::Load(i) => out.push(cmd(Command::Load(i as u8))),
            Act::Favour(f) => {
                let id = match f {
                    0 => "bow",
                    1 => "stern",
                    2 => "port",
                    3 => "starboard",
                    _ => "balanced",
                };
                if let Some(i) = presets.iter().position(|p| p.id == id) {
                    out.push(cmd(Command::Preset(i as u8)));
                }
            }
            Act::Feed(k) => {
                // The captain's camera is everyone's while the viewscreen is taken.
                if v.station == "captain" && v.captain.as_ref().is_some_and(|c| c.view_override) {
                    if let Some(i) = FEEDS.iter().position(|f| *f == k) {
                        out.push(cmd(Command::Viewscreen(Some(i as u8))));
                    }
                }
                self.feed = k;
            }
            Act::SciSel(id) => self.sci_sel = Some(id),
            Act::Scan => {
                let scanning = v.sci.as_ref().is_some_and(|q| q.scanning);
                out.push(cmd(Command::Scan(!scanning)));
            }
            Act::Freq => {
                let band = v.sci.as_ref().and_then(|q| BANDS.iter().position(|b| *b == q.freq)).unwrap_or(0);
                out.push(cmd(Command::Freq(((band + 1) % BANDS.len()) as u8)));
            }
            Act::ShieldView => self.shield_cam = SHIELD_CAM,
            Act::Zoom(d) => self.zoom = (self.zoom * if d > 0 { 2.0 } else { 0.5 }).clamp(1.0, 8.0),
            Act::Compose(r) => self.composer = (r, None),
            Act::Verb(w) => self.composer.1 = Some(w),
            Act::Send => {
                if let (Some(to), Some(w)) = (Station::from_id(&self.composer.0), self.composer.1.take()) {
                    if let Some(i) = data.order_verbs(to).iter().position(|x| x[0] == w) {
                        out.push(cmd(Command::Order(to, i as u8)));
                    }
                }
            }
            Act::Brace => {
                let braced = v.captain.as_ref().is_some_and(|c| c.braced);
                out.push(cmd(Command::Brace(!braced)));
            }
            Act::ViewTake => {
                let taken = v.captain.as_ref().is_some_and(|c| c.view_override);
                let feed = FEEDS.iter().position(|f| *f == self.feed).unwrap_or(1) as u8;
                out.push(cmd(Command::Viewscreen((!taken).then_some(feed))));
            }
            Act::Room(id) => self.ship_sel = id,
            Act::Tab(t) => self.cap_tab = t,
            // Engineering's and the captain's damage control are not simulated in the drill: drawn unavailable.
            Act::Preset(_)
            | Act::Prio(_)
            | Act::GLock(_)
            | Act::Scram
            | Act::RxMode
            | Act::CMode
            | Act::OrderRepair => {}
        }
    }

    fn step_wheel(&mut self, k: char, d: f64) {
        match k {
            'h' => self.oe.h = whole_deg('h', self.oe.h + d),
            'p' => self.oe.p = whole_deg('p', self.oe.p + d),
            _ => self.oe.r = whole_deg('r', self.oe.r + d),
        }
    }

    fn drag_to(&mut self, p: Pos2, s: f32, fl: &sc_core::combat::data::FlightBlock) {
        use console::kit::Drag;
        let Some(d) = self.drag.as_mut() else { return };
        let r = d.track;
        let ky = f64::from((r.max.y - p.y) / r.height().max(1e-3));
        let kx = f64::from((p.x - r.min.x) / r.width().max(1e-3));
        let (dx, dy) = (p.x - d.p0.x, p.y - d.p0.y);
        match &d.what {
            Drag::Throttle => {
                let v = fl.speed_min_mps + ky * (fl.speed_max_mps - fl.speed_min_mps);
                self.speed_set = ((v / 10.0).round() * 10.0).clamp(fl.speed_min_mps, fl.speed_max_mps);
            }
            Drag::Strafe => {
                let m = fl.strafe_max_mps;
                let q = |k: f64| (((k * 2.0 - 1.0) * m / 5.0).round() * 5.0).clamp(-m, m);
                self.strafe = [q(kx), q(ky)];
            }
            Drag::Joy => {
                self.stick_drag[0] = f64::from(dx / (40.0 * s)).clamp(-1.0, 1.0);
                self.stick_drag[1] = f64::from(dy / (24.0 * s)).clamp(-1.0, 1.0);
            }
            Drag::Flip => self.stick_drag[2] = (d.side + f64::from(dx / (24.0 * s))).clamp(-1.0, 1.0),
            Drag::Wheel(k) => {
                let n = (-dy / (5.0 * s)).round();
                d.moved |= n != 0.0;
                let v = whole_deg(*k, d.v0 + f64::from(n));
                match k {
                    'h' => self.oe.h = v,
                    'p' => self.oe.p = v,
                    _ => self.oe.r = v,
                }
            }
            Drag::ScanCam(key) => {
                let cam = ScanCam {
                    yaw: d.cam0.0 + f64::from(dx / r.width().max(1.0)) * 180.0,
                    d_el: (d.cam0.1 + f64::from(dy / r.height().max(1.0)) * 90.0).clamp(-60.0, 80.0),
                };
                match *key {
                    "nav" => self.scan_cam.nav = cam,
                    "tac" => self.scan_cam.tac = cam,
                    _ => self.scan_cam.sci = cam,
                }
            }
            Drag::FeedView => {
                self.feed_cam = Cam {
                    yaw: d.cam0.0 + f64::from(dx / r.width().max(1.0)) * 180.0,
                    el: (d.cam0.1 + f64::from(dy / r.height().max(1.0)) * 120.0).clamp(-85.0, 85.0),
                };
            }
            Drag::ShieldCam => {
                // As the mockup: a drag across turns the view a full turn, up and down 120 degrees, held to +/-80.
                self.shield_cam = Cam {
                    yaw: d.cam0.0 + f64::from(dx / r.width().max(1.0)) * 360.0,
                    el: (d.cam0.1 + f64::from(dy / r.height().max(1.0)) * 120.0).clamp(-80.0, 80.0),
                };
            }
            // Engineering's faders and levers: no power grid in the drill (drawn unavailable, with no hits).
            Drag::Fader(_) | Drag::Lever(_) => {}
        }
    }

    /// Turn this frame's pointer, scroll and keys on the console (whose controls are `hits`, at `s` points a layout
    /// point) into the seat's changes and the commands it sends. Call only in Engage, from the seated client.
    #[allow(clippy::too_many_arguments)]
    pub fn input(
        &mut self,
        ctx_input: &Input,
        hits: &console::kit::Hits,
        s: f32,
        v: &ConsoleView,
        tern: &ShipSnap,
        hound: &ShipSnap,
        data: &DrillData,
        station: Station,
        now: f64,
    ) -> Vec<SeatAsk> {
        use console::kit::Drag;
        let mut out = Vec::new();
        let fl = &data.tern_flight;
        for e in &ctx_input.events {
            match *e {
                PointerEv::Press(p) => {
                    let Some(region) = hits.at(p) else { continue };
                    match region.hit.clone() {
                        Some(Hit::Act(a)) => self.act(a, v, tern, hound, data, &mut out),
                        Some(Hit::Hold(k)) => self.hold = Some((k, now, false)),
                        Some(Hit::Drag(what)) => {
                            let cam0 = match &what {
                                Drag::ScanCam(key) => {
                                    let c = self.scan_cam.of(key);
                                    (c.yaw, c.d_el)
                                }
                                Drag::FeedView => (self.feed_cam.yaw, self.feed_cam.el),
                                Drag::ShieldCam => (self.shield_cam.yaw, self.shield_cam.el),
                                _ => (0.0, 0.0),
                            };
                            let v0 = match &what {
                                Drag::Wheel('h') => self.oe.h,
                                Drag::Wheel('p') => self.oe.p,
                                Drag::Wheel(_) => self.oe.r,
                                _ => 0.0,
                            };
                            let side = if p.x < region.rect.center().x { -1.0 } else { 1.0 };
                            let immediate = matches!(what, Drag::Throttle | Drag::Strafe | Drag::Joy | Drag::Flip);
                            self.drag =
                                Some(Dragging { what, p0: p, track: region.track, cam0, v0, side, moved: false });
                            if immediate {
                                self.drag_to(p, s, fl);
                            }
                        }
                        None => {}
                    }
                }
                PointerEv::Move(p) => self.drag_to(p, s, fl),
                PointerEv::Release(p) => {
                    if let Some(d) = self.drag.take() {
                        match d.what {
                            Drag::Joy => {
                                self.stick_drag[0] = 0.0;
                                self.stick_drag[1] = 0.0;
                            }
                            Drag::Flip => self.stick_drag[2] = 0.0,
                            // A thumbwheel tapped, not turned: its top half turns it up a degree, its bottom down.
                            Drag::Wheel(k) if !d.moved => {
                                self.step_wheel(k, if p.y < d.track.center().y { 1.0 } else { -1.0 })
                            }
                            _ => {}
                        }
                    }
                    // A hold let go early does nothing (bridge-stations 8.5: guarded controls).
                    if self.hold.as_ref().is_some_and(|(_, t0, key)| !key && now - t0 < HOLD_S) {
                        self.hold = None;
                    }
                }
                PointerEv::Scroll(p, up) => {
                    if let Some(Hit::Drag(what)) =
                        hits.regions.iter().rev().find(|r| r.rect.contains(p)).and_then(|r| r.hit.clone())
                    {
                        match what {
                            Drag::Wheel(k) => self.step_wheel(k, if up { 1.0 } else { -1.0 }),
                            Drag::ScanCam(key) => {
                                self.act(Act::Range(key, if up { -1 } else { 1 }), v, tern, hound, data, &mut out)
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
        // A guarded hold done: FIRE from the first ready tube.
        if let Some((k, t0, _)) = self.hold.clone() {
            if now - t0 >= HOLD_S {
                self.hold = None;
                match k.as_str() {
                    "fire" => {
                        if let Some(i) = tern.tubes.iter().position(|t| t.0 == TubeState::Armed) {
                            out.push(SeatAsk::Command(Command::Fire(i as u8)));
                        }
                    }
                    "ping" => out.push(SeatAsk::Command(Command::Ping)),
                    "redalert" => out.push(SeatAsk::Command(Command::Alert(!v.red_alert))),
                    _ => {}
                }
            }
        }
        let held = &ctx_input.held;
        let pressed = &ctx_input.pressed;
        match station {
            Station::Helm => {
                if pressed.contains(&keys::W) {
                    self.speed_set = (self.speed_set + 10.0).min(fl.speed_max_mps);
                }
                if pressed.contains(&keys::S) {
                    self.speed_set = (self.speed_set - 10.0).max(fl.speed_min_mps);
                }
                if pressed.contains(&keys::X) {
                    self.act(Act::AllStop, v, tern, hound, data, &mut out);
                }
                let axis =
                    |p: u32, n: u32| f64::from(u8::from(held.contains(&p))) - f64::from(u8::from(held.contains(&n)));
                // The mockup's keys: A and D yaw to port and starboard, R and F pitch up and down, Q and E roll.
                let stick_keys = [axis(keys::D, keys::A), axis(keys::R, keys::F), axis(keys::E, keys::Q)];
                let stick: [f64; 3] = std::array::from_fn(|i| (stick_keys[i] + self.stick_drag[i]).clamp(-1.0, 1.0));
                // Z and C strafe to port and starboard, Space and Ctrl up and down, while held; the pad otherwise.
                let (lat, ver) = (axis(keys::C, keys::Z), axis(keys::SPACE, keys::LCTRL));
                let m = fl.strafe_max_mps;
                let want = [
                    if lat != 0.0 { lat * m } else { self.strafe[0] },
                    if ver != 0.0 { ver * m } else { self.strafe[1] },
                ];
                if want != self.strafe_sent {
                    self.strafe_sent = want;
                    out.push(SeatAsk::Command(Command::Strafe { lat_mps: want[0], vert_mps: want[1] }));
                }
                if now - self.last_send >= 0.05 {
                    self.last_send = now;
                    // The engine's stick yaws to port: the console's yaw is to starboard.
                    out.push(SeatAsk::Command(Command::Stick {
                        speed_set_mps: self.speed_set,
                        yaw: -stick[0],
                        pitch: stick[1],
                        roll: stick[2],
                    }));
                }
                self.stick_now = stick;
            }
            Station::Tactical => {
                if pressed.contains(&keys::T) {
                    out.push(SeatAsk::Command(Command::Lock(Some(ENEMY_ID))));
                }
                if pressed.contains(&keys::L) {
                    if let Some(i) = tern.tubes.iter().position(|t| t.0 == TubeState::Empty) {
                        out.push(SeatAsk::Command(Command::Load(i as u8)));
                    }
                }
                if pressed.contains(&keys::G) && self.hold.is_none() {
                    self.hold = Some(("fire".into(), now, true));
                }
                if !held.contains(&keys::G) && self.hold.as_ref().is_some_and(|(_, _, key)| *key) {
                    self.hold = None;
                }
            }
            // The mockup's Q: scan.
            Station::Science => {
                if pressed.contains(&keys::Q) {
                    self.act(Act::Scan, v, tern, hound, data, &mut out);
                }
            }
            Station::Engineering | Station::Captain => {}
        }
        // Y acknowledges the captain's order, on any console.
        if pressed.contains(&keys::Y) {
            self.act(Act::Ack, v, tern, hound, data, &mut out);
        }
        out
    }
}

/// A pointer event on the console, in egui points.
#[derive(Clone, Copy, Debug)]
pub enum PointerEv {
    Press(Pos2),
    Move(Pos2),
    Release(Pos2),
    /// A scroll notch: up or down.
    Scroll(Pos2, bool),
}

/// This frame's input for the seat.
#[derive(Default)]
pub struct Input {
    pub events: Vec<PointerEv>,
    pub held: HashSet<u32>,
    pub pressed: HashSet<u32>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::console::kit::{Drag, Hits};
    use sc_core::combat::{Drill, Phase, TurretMode, TICK_HZ};

    /// A drill in Engage with both seats taken, its snapshot, and its data.
    fn engaged() -> (Snapshot, DrillData) {
        let data = DrillData::shipped();
        let mut d = Drill::new(data.clone(), 3);
        let h = d.join("Helm", false).unwrap();
        let t = d.join("Tac", false).unwrap();
        d.claim(h, Station::Helm).unwrap();
        d.claim(t, Station::Tactical).unwrap();
        d.set_ready(h, true).unwrap();
        d.set_ready(t, true).unwrap();
        for _ in 0..(TICK_HZ as usize * 10) {
            d.step();
            if d.phase == Phase::Engage {
                break;
            }
        }
        d.command(t, Command::Lock(Some(ENEMY_ID))).unwrap();
        for _ in 0..(TICK_HZ as usize * 3) {
            d.step();
        }
        (Snapshot::of(&d), data)
    }

    /// The console painted headless at 1280 x 720 points: its controls.
    fn paint(v: &ConsoleView) -> Hits {
        let ctx = egui::Context::default();
        crate::vg::install_fonts(&ctx);
        let mut hits = None;
        let raw = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(1280.0, 720.0))),
            ..Default::default()
        };
        let _ = ctx.run(raw, |ctx| hits = Some(console::paint(ctx, v, false, None).0));
        hits.unwrap()
    }

    fn centre(hits: &Hits, want: &Hit) -> Pos2 {
        hits.regions
            .iter()
            .find(|r| r.hit.as_ref() == Some(want))
            .unwrap_or_else(|| panic!("no {want:?} on the console"))
            .rect
            .center()
    }

    struct Bench {
        seat: Seat,
        snap: Snapshot,
        data: DrillData,
        station: Station,
    }

    impl Bench {
        fn new(station: Station) -> Self {
            let (snap, data) = engaged();
            Bench { seat: Seat::default(), snap, data, station }
        }
        fn view(&mut self, now: f64) -> ConsoleView {
            let (t, h) = (self.snap.ships[0].clone(), self.snap.ships[1].clone());
            self.seat.view(self.station, &t, &h, &self.snap, &self.data, &[], now)
        }
        /// Feed pointer events and return the commands sent.
        fn input(&mut self, events: Vec<PointerEv>, now: f64) -> Vec<Command> {
            let v = self.view(now);
            let hits = paint(&v);
            let (t, h) = (self.snap.ships[0].clone(), self.snap.ships[1].clone());
            let input = Input { events, ..Default::default() };
            let asks = self.seat.input(&input, &hits, 1.0, &v, &t, &h, &self.data, self.station, now);
            asks.into_iter()
                .filter_map(|a| match a {
                    SeatAsk::Command(c) => Some(c),
                    SeatAsk::Claim(_) => None,
                })
                .filter(|c| !matches!(c, Command::Stick { .. }))
                .collect()
        }
        fn press(&mut self, hit: Hit, now: f64) -> Vec<Command> {
            let p = centre(&paint(&self.view(now)), &hit);
            self.input(vec![PointerEv::Press(p), PointerEv::Release(p)], now)
        }
    }

    #[test]
    fn the_autopilot_buttons_send_their_modes() {
        let mut b = Bench::new(Station::Helm);
        assert_eq!(b.press(Hit::Act(Act::Ap("COURSE".into())), 1.0), vec![Command::Helm(HelmMode::Course)]);
        assert_eq!(b.press(Hit::Act(Act::Ap("EVADE".into())), 1.1), vec![Command::Helm(HelmMode::Evade)]);
    }

    #[test]
    fn go_sends_the_thumbwheels_order_and_level_the_level_attitude() {
        let mut b = Bench::new(Station::Helm);
        assert_eq!(b.press(Hit::Act(Act::OGo), 1.0), vec![Command::Orient(Some([45.0, 10.0, -30.0]))]);
        let c = b.press(Hit::Act(Act::OLevel), 1.1);
        let Some(Command::Orient(Some([_, p, r]))) = c.first() else { panic!("{c:?}") };
        assert_eq!((*p, *r), (0.0, 0.0), "LEVEL keeps the heading and zeroes pitch and roll");
    }

    #[test]
    fn dragging_the_lever_to_its_top_sets_full_speed() {
        let mut b = Bench::new(Station::Helm);
        let hits = paint(&b.view(1.0));
        let lever = hits.regions.iter().find(|r| r.hit == Some(Hit::Drag(Drag::Throttle))).unwrap();
        let (bottom, top) = (lever.track.center_bottom(), lever.track.center_top() - egui::vec2(0.0, 20.0));
        b.input(vec![PointerEv::Press(bottom), PointerEv::Move(top), PointerEv::Release(top)], 1.0);
        assert_eq!(b.seat.speed_set, b.data.tern_flight.speed_max_mps);
    }

    #[test]
    fn the_strafe_pad_sends_its_set_points() {
        let mut b = Bench::new(Station::Helm);
        let hits = paint(&b.view(1.0));
        let pad = hits.regions.iter().find(|r| r.hit == Some(Hit::Drag(Drag::Strafe))).unwrap();
        let corner = pad.track.right_top();
        let sent = b.input(vec![PointerEv::Press(corner), PointerEv::Release(corner)], 1.0);
        let m = b.data.tern_flight.strafe_max_mps;
        assert!(
            sent.contains(&Command::Strafe { lat_mps: m, vert_mps: m }),
            "the top right corner is full starboard and up: {sent:?}"
        );
    }

    #[test]
    fn science_scans_on_its_button_and_tunes_the_band_up_one() {
        let mut b = Bench::new(Station::Science);
        assert_eq!(b.press(Hit::Act(Act::Scan), 1.0), vec![Command::Scan(true)]);
        assert_eq!(b.press(Hit::Act(Act::Freq), 1.1), vec![Command::Freq(1)], "A steps to B");
        let bow = b.data.tern_shields.presets.iter().position(|p| p.id == "bow").unwrap() as u8;
        assert_eq!(b.press(Hit::Act(Act::Favour(0)), 1.2), vec![Command::Preset(bow)], "Science sets the shields too");
    }

    #[test]
    fn the_captain_sends_the_composed_order_and_takes_the_viewscreen() {
        let mut b = Bench::new(Station::Captain);
        b.press(Hit::Act(Act::Compose("tactical".into())), 1.0);
        b.press(Hit::Act(Act::Verb("HOLD FIRE".into())), 1.1);
        let verb = b.data.order_verbs(Station::Tactical).iter().position(|v| v[0] == "HOLD FIRE").unwrap() as u8;
        assert_eq!(b.press(Hit::Act(Act::Send), 1.2), vec![Command::Order(Station::Tactical, verb)]);
        assert_eq!(b.press(Hit::Act(Act::ViewTake), 1.3), vec![Command::Viewscreen(Some(1))], "the seat's FWD camera");
        assert_eq!(b.press(Hit::Act(Act::Brace), 1.4), vec![Command::Brace(true)]);
    }

    #[test]
    fn engineering_draws_unavailable_with_nothing_to_press() {
        let mut b = Bench::new(Station::Engineering);
        let v = b.view(1.0);
        assert!(v.eng.is_none(), "no power grid is simulated in the drill");
        let hits = paint(&v);
        let live = hits.regions.iter().filter(|r| r.rect.min.y > 280.0 && r.rect.max.y < 688.0 && r.hit.is_some());
        assert_eq!(live.count(), 0, "every engineering control is inert");
    }

    #[test]
    fn a_tap_on_a_turret_steps_its_mode() {
        let mut b = Bench::new(Station::Tactical);
        assert_eq!(b.press(Hit::Act(Act::TMode(2)), 1.0), vec![Command::TurretMode(2, TurretMode::Hold.next())]);
    }

    #[test]
    fn the_bow_preset_and_a_target_card_send_their_commands() {
        let mut b = Bench::new(Station::Tactical);
        let bow = b.data.tern_shields.presets.iter().position(|p| p.id == "bow").unwrap() as u8;
        assert_eq!(b.press(Hit::Act(Act::Favour(0)), 1.0), vec![Command::Preset(bow)]);
        assert_eq!(b.press(Hit::Act(Act::Target(TRACK.into())), 1.1), vec![Command::Lock(Some(ENEMY_ID))]);
    }

    #[test]
    fn fire_acts_only_after_a_full_hold() {
        let mut b = Bench::new(Station::Tactical);
        b.snap.ships[0].tubes[0] = (TubeState::Armed, 0.0);
        let p = centre(&paint(&b.view(1.0)), &Hit::Hold("fire".into()));
        assert!(b.input(vec![PointerEv::Press(p)], 1.0).is_empty());
        assert!(b.input(vec![PointerEv::Release(p)], 1.3).is_empty(), "let go at 0.3 s: nothing fires");
        assert!(b.input(vec![PointerEv::Press(p)], 2.0).is_empty());
        assert_eq!(b.input(vec![], 2.0 + HOLD_S), vec![Command::Fire(0)], "held 0.6 s: the ready tube fires");
    }
}
