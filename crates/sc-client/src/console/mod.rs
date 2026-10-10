//! The bridge consoles, drawn as their approved mockup draws them (CLAUDE.md 10; openspec/changes/console-parity).
//!
//! `docs/mockups/consoles.html` is the consoles' specification. This module is its helm and tactical consoles,
//! ported function for function onto the [`vg`](crate::vg) canvas: the title band, the look band (the viewscreen,
//! its camera widget and the navball), the four panels of each console on the 104 x 102 grid, and the status strip.
//! Everything they show comes from one [`ConsoleView`], which is the mockup's own `window.consoleState()` in shape:
//! the drill fills it from its snapshot every frame, and the parity check (`sc-client --console-fixture`) reads it
//! from the file the mockup wrote, so the engine draws the mockup's exact moment beside the mockup's picture.
//!
//! Drawing records where each control is ([`Hits`]); the caller turns a press, a drag or a hold on one into an
//! [`Act`], and the drill turns that into a command or a change to the console's own view.

mod helm;
pub mod kit;
mod tactical;

use crate::vg::{self, alpha, rgba, Canvas, Pen, Xf, T};
use egui::{Color32, Pos2, Rect};
use glam::{DQuat, DVec3};
use serde::Deserialize;
use std::sync::OnceLock;

pub use kit::Act;

/// One heading, pitch and roll, degrees: an attitude order's three thumbwheels.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Hpr {
    pub h: f64,
    pub p: f64,
    pub r: f64,
}

/// A slew's plan (flight-and-navigation 6a): seconds, degrees, and whether the order is held.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PlanView {
    pub time: f64,
    pub angle: f64,
    #[serde(default)]
    pub held: bool,
}

/// The ship's flight limits the controls are drawn against.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct FlightView {
    /// Rate limits [pitch (x), yaw (y), roll (z)], deg/s.
    pub rate_dps: [f64; 3],
    pub speed_min_mps: f64,
    pub speed_max_mps: f64,
    pub strafe_max_mps: f64,
}

/// A contact, in ship axes.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ContactView {
    pub id: String,
    /// corvette, fighter or freighter.
    pub cls: String,
    /// hostile, neutral, friendly or unknown.
    pub iff: String,
    /// Where it is from the ship, ship axes, metres.
    pub rel: [f64; 3],
    /// Its heading, a unit vector in ship axes.
    pub dir: [f64; 3],
    /// Hull, percent.
    pub hull: f64,
    /// How far Science's scan has gone, 0-1 (the hull shows from 0.3).
    pub scan: f64,
}

/// An inbound missile: from direction `d` (ship axes, toward the shooter), `t` seconds out.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct InboundView {
    pub d: [f64; 3],
    pub t: f64,
}

/// A recent hit on the shield.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct HitView {
    /// Toward the attacker, ship axes.
    pub d: [f64; 3],
    pub face: u8,
    pub age_s: f64,
    /// Some of it reached the hull.
    pub through: bool,
}

/// A scanner's camera: yaw about the ship's dorsal axis and tilt from the panel's own, degrees.
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ScanCam {
    pub yaw: f64,
    #[serde(rename = "dEl")]
    pub d_el: f64,
}

/// The two scanners' cameras.
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ScanCams {
    pub nav: ScanCam,
    pub tac: ScanCam,
}

/// A camera turned round the ship: yaw and elevation, degrees.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Cam {
    pub yaw: f64,
    pub el: f64,
}

/// A turret's dial.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct TurretView {
    /// Its letter.
    pub short: String,
    /// AUTO, TARGET, PD or HOLD.
    pub mode: String,
    /// Its gunner, if one is in the pod.
    pub op: Option<String>,
    /// Heat, 0-1 of lockout.
    pub heat: f64,
    /// Where it points: bearing, degrees to starboard of the bow.
    pub aim_deg: f64,
    pub firing: bool,
    pub cooling: bool,
    /// How well it works, 0-1 (supply and integrity).
    pub ok: f64,
}

/// A missile tube.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct TubeView {
    /// EMPTY, LOADING or READY.
    pub state: String,
    /// Seconds into loading.
    pub t: f64,
}

/// A guarded control being held: its key and how far the hold has gone, 0-1.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct HoldView {
    pub key: String,
    pub k: f64,
}

/// Everything the helm and tactical consoles draw: the mockup's `window.consoleState()`, field for field.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ConsoleView {
    /// helm or tactical.
    pub station: String,
    /// The station's word on its chip (HELM).
    pub word: String,
    /// The station's role, for its colour.
    pub role: String,
    /// Who sits there.
    pub op: Option<String>,
    /// Stations with nobody at them, which this seat can swap to.
    pub open: Vec<String>,
    /// The captain's newest order to this station.
    pub order: Option<String>,
    /// normal, red_alert or emergency.
    pub alert: String,
    /// The clock in the title band, seconds.
    pub clock_s: f64,
    pub hull_pct: f64,
    /// Each shield face's charge over its capacity.
    pub shield_ratio: [f64; 6],
    /// Power in use, MW (none: no power grid is simulated).
    pub power_mw: Option<f64>,
    pub reactor_up: bool,
    pub speed_mps: f64,
    pub fires: u32,
    pub breaches: u32,
    /// Attitude, ship axes to reference axes, [w, x, y, z].
    pub q: [f64; 4],
    /// Body rates, rad/s, ship axes.
    pub w: [f64; 3],
    /// The rates the helm is asking for, rad/s.
    pub wset: [f64; 3],
    /// Velocity in ship axes, m/s.
    pub v_body: [f64; 3],
    pub speed_set: f64,
    /// Lateral (to starboard) and vertical set points, m/s.
    pub strafe: [f64; 2],
    /// The stick [yaw to starboard, pitch up, roll right], -1..1.
    pub stick: [f64; 3],
    /// The order on the thumbwheels.
    pub oe: Hpr,
    pub order_active: bool,
    /// The active order's slew.
    pub plan: Option<PlanView>,
    /// The thumbwheels' order's slew, before GO.
    pub preview: PlanView,
    /// Where the ship is told to point, if anywhere.
    pub goal: Option<[f64; 4]>,
    /// HOLD, COURSE, CHASE, MATCH or EVADE.
    pub ap: String,
    pub flight: FlightView,
    pub contacts: Vec<ContactView>,
    /// The waypoint, ship axes (none: the mission has none).
    pub waypoint: Option<[f64; 3]>,
    pub target: Option<String>,
    pub tubes_bear: bool,
    /// Our missiles in flight, ship axes.
    pub missiles: Vec<[f64; 3]>,
    pub inbound: Vec<InboundView>,
    pub hits: Vec<HitView>,
    /// Seconds since each face last flashed.
    pub flash_age_s: [f64; 6],
    /// What the viewscreen shows: FWD, AFT, PORT, STBD, CHASE or TARGET.
    pub feed: String,
    pub zoom: f64,
    pub nav_km: f64,
    pub tac_km: f64,
    pub sci_km: f64,
    pub scan_cam: ScanCams,
    pub feed_cam: Cam,
    pub turrets: Vec<TurretView>,
    pub tubes: Vec<TubeView>,
    pub tube_load_s: f64,
    pub magazine: u32,
    pub magazine_slots: u32,
    /// The ring round FIRE, 0-1: what the fire rule waits on (the drill: the lock; the mockup: its hit chance).
    pub fire_ring: f64,
    /// The words under FIRE.
    pub fire_note: String,
    /// The ring and its words in the ready colour, not the waiting one.
    pub fire_ok: bool,
    /// The tubes' cone off the bow either way, degrees.
    pub tube_cone_deg: f64,
    /// Shield charge and capacity per face, MJ.
    pub charge: [f64; 6],
    pub cap: [f64; 6],
    /// The favoured face, or -1 balanced.
    pub preset: i32,
    pub hold: Option<HoldView>,
}

// ------------------------------------------------------------------------------------------------ math

/// The mockup's quaternion [w, x, y, z] as glam's.
pub fn quat(q: [f64; 4]) -> DQuat {
    DQuat::from_xyzw(q[1], q[2], q[3], q[0])
}

/// glam's quaternion as the mockup's [w, x, y, z], with w >= 0 (the form the consoles show).
pub fn quat_arr(q: DQuat) -> [f64; 4] {
    let s = if q.w < 0.0 { -1.0 } else { 1.0 };
    [q.w * s, q.x * s, q.y * s, q.z * s]
}

pub fn v3(a: [f64; 3]) -> DVec3 {
    DVec3::from_array(a)
}

/// Degrees, wrapped to 0..360.
pub fn wrap360(a: f64) -> f64 {
    a.rem_euclid(360.0)
}

/// Degrees, wrapped to -180..180.
pub fn wrap180(a: f64) -> f64 {
    let a = wrap360(a);
    if a > 180.0 {
        a - 360.0
    } else {
        a
    }
}

/// A rotation of `deg` about a unit axis.
fn axis(ax: DVec3, deg: f64) -> DQuat {
    DQuat::from_axis_angle(ax, deg.to_radians())
}

pub use sc_core::combat::attitude::{from_hpr as from_euler, hpr_of as to_euler, whole_deg};

/// Bearing of a ship-axes direction, degrees to starboard of the bow, -180..180.
pub fn brg_of(d: DVec3) -> f64 {
    wrap180((-d.x).atan2(d.z).to_degrees())
}

/// Elevation of a ship-axes direction above the ship's plane, degrees.
pub fn elev_of(d: DVec3) -> f64 {
    d.y.atan2(d.x.hypot(d.z)).to_degrees()
}

/// A camera turned round a model: forward, right and up for yaw and elevation in degrees (the mockup's `camBasis`).
pub struct Basis {
    pub f: DVec3,
    pub r: DVec3,
    pub u: DVec3,
}

pub fn cam_basis(yaw: f64, el: f64) -> Basis {
    let (ce, se) = (el.to_radians().cos(), el.to_radians().sin());
    let d = DVec3::new(yaw.to_radians().sin() * ce, se, -yaw.to_radians().cos() * ce);
    let f = -d;
    let r = f.cross(DVec3::Y).normalize_or_zero();
    Basis { f, r, u: r.cross(f) }
}

/// The mockup's `fmtAng`: a heading as three digits, a signed angle with its sign, and a degree sign.
pub fn fmt_ang(x: f64, signed: bool) -> String {
    if signed {
        let r = x.round();
        format!("{}{}°", if r >= 0.0 { "+" } else { "" }, r as i64)
    } else {
        format!("{:03}°", (wrap360(x).round() as i64) % 360)
    }
}

/// The mockup's `fmtQ`: each component to four places, a space where a minus would be.
pub fn fmt_q(q: [f64; 4]) -> String {
    let s = if q[0] < 0.0 { -1.0 } else { 1.0 };
    q.iter()
        .map(|x| {
            let v = x * s;
            format!("{}{:.4}", if v < 0.0 { "" } else { " " }, v)
        })
        .collect::<Vec<_>>()
        .join("  ")
}

// ------------------------------------------------------------------------------------------------ ship data

/// The shield ellipsoid: semi-axes and centre, ship axes, metres (weapons-and-shields 11).
#[derive(Clone, Copy, Debug)]
pub struct Ellipsoid {
    pub axes: DVec3,
    pub centre: DVec3,
}

/// The Tern's shape for the consoles' 3D pictures: the hull loft's octagonal sections, each eight corners in ship
/// axes relative to the shield's centre, and the shield ellipsoid. Read once from the compiled-in data.
pub struct ShipShape {
    pub hull: Vec<[DVec3; 8]>,
    pub shield: Ellipsoid,
}

/// The Tern's shape.
pub fn tern() -> &'static ShipShape {
    static SHAPE: OnceLock<ShipShape> = OnceLock::new();
    SHAPE.get_or_init(|| {
        let data = sc_core::combat::data::DrillData::shipped();
        let e = data.tern_shields.ellipsoid;
        let shield = Ellipsoid { axes: DVec3::from_array(e.axes_m), centre: DVec3::from_array(e.centre_m) };
        let layout: serde_json::Value = serde_json::from_str(include_str!("../../../../data/ships/tern/layout.json"))
            .expect("the layout is valid JSON (tools/layout_check.py)");
        let hull = layout["hull"]["sections"]
            .as_array()
            .expect("the layout has hull sections")
            .iter()
            .map(|q| {
                let f = |k: &str| q[k].as_f64().expect("a hull section's numbers");
                let (hb, t, b, c, z) = (f("half_beam_m"), f("top_m"), f("bottom_m"), f("chamfer_m"), f("z_m"));
                let pts = [
                    [hb, t - c],
                    [hb - c, t],
                    [-(hb - c), t],
                    [-hb, t - c],
                    [-hb, b + c],
                    [-(hb - c), b],
                    [hb - c, b],
                    [hb, b + c],
                ];
                pts.map(|[x, y]| DVec3::new(x, y, z) - shield.centre)
            })
            .collect();
        ShipShape { hull, shield }
    })
}

// ------------------------------------------------------------------------------------------------ drawing

/// What one frame's drawing needs besides the view.
pub struct Ctx<'a> {
    pub v: &'a ConsoleView,
    pub hits: &'a mut kit::Hits,
    /// The viewscreen's feed is drawn under the console by the renderer (`Renderer::present_with_ui_at` into
    /// [`viewscreen_px`]), so the console leaves the viewscreen open; false (a parity fixture): the screen is black.
    pub feed: bool,
    /// The pointer, in canvas units, for hover tips.
    pub pointer: Option<Pos2>,
}

/// The colour of a contact's IFF.
pub fn iff_col(iff: &str) -> Color32 {
    let c = &vg::style().c;
    match iff {
        "hostile" => c.hostile.0,
        "neutral" => c.neutral.0,
        "friendly" => c.friendly.0,
        _ => c.unknown.0,
    }
}

/// A shield ratio's colour: green over half, amber over a fifth, red below.
pub fn ratio_col(k: f64) -> Color32 {
    let c = &vg::style().c;
    if k > 0.5 {
        c.ok.0
    } else if k > 0.2 {
        c.warn.0
    } else {
        c.danger.0
    }
}

/// Draw a console on egui's background, the 1280 x 720 canvas fitted and centred in the screen; returns where its
/// controls are and its scale (egui points a layout point). `feed` is the viewscreen's camera, `pointer` the pointer in
/// egui points (for the hover tip).
pub fn paint(ctx: &egui::Context, v: &ConsoleView, feed: bool, pointer: Option<Pos2>) -> (kit::Hits, f32) {
    let (o, s) = fit(ctx.content_rect());
    let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Background, egui::Id::new("console")));
    let cv = Canvas::new(painter, o, s);
    let mut hits = kit::Hits::default();
    draw(&cv, &mut Ctx { v, hits: &mut hits, feed, pointer });
    (hits, s)
}

/// Where the 1280 x 720 canvas sits on a screen of `screen` points: its origin and its scale, fitted and centred.
pub fn fit(screen: Rect) -> (egui::Vec2, f32) {
    let s = (screen.width() / 1280.0).min(screen.height() / 720.0);
    (egui::Vec2::new((screen.width() - 1280.0 * s) / 2.0, (screen.height() - 720.0 * s) / 2.0), s)
}

/// The viewscreen's window in output pixels (x, y from the top left, width, height), for a screen `points` big drawn
/// at `ppp` pixels a point: where the renderer draws the feed under the console.
pub fn viewscreen_px(points: [f32; 2], ppp: f32) -> [i32; 4] {
    let (o, s) = fit(Rect::from_min_size(Pos2::ZERO, egui::vec2(points[0], points[1])));
    let [x, y, w, h] = VIEWSCREEN;
    let px = |v: f32| (v * ppp).round() as i32;
    [px(o.x + x * s), px(o.y + (32.0 + y) * s), px(w * s), px(h * s)]
}

/// Draw a whole console (1280 x 720 layout points) on `cv`: the bands, the station's panels and the status strip.
pub fn draw(cv: &Canvas, cx: &mut Ctx) {
    let c = &vg::style().c;
    if cx.feed {
        // The background round the viewscreen's window, which the feed shows through.
        let [x, y, w, h] = VIEWSCREEN;
        let (y, x2, y2) = (y + 32.0, x + w, y + 32.0 + h);
        cv.rect(0.0, 0.0, 1280.0, y, 0.0, c.bg.0, None);
        cv.rect(0.0, y2, 1280.0, 720.0 - y2, 0.0, c.bg.0, None);
        cv.rect(0.0, y, x, h, 0.0, c.bg.0, None);
        cv.rect(x2, y, 1280.0 - x2, h, 0.0, c.bg.0, None);
    } else {
        cv.rect(0.0, 0.0, 1280.0, 720.0, 0.0, c.bg.0, None);
    }
    title_band(&cv.at(0.0, 0.0), cx);
    // Nothing in the look band reaches past it, so it takes no clip rectangle (each costs a UI draw call).
    look_band(&cv.at(0.0, 32.0), cx);
    match cx.v.station.as_str() {
        "helm" => helm::draw(cv, cx),
        _ => tactical::draw(cv, cx),
    }
    status_strip(&cv.at(0.0, 688.0), cx);
    kit::tooltip(cv, cx);
}

/// The station's accent colour (its role's).
pub fn accent(v: &ConsoleView) -> Color32 {
    vg::style().role.of(&v.role)
}

fn station_word(id: &str) -> (&'static str, &'static str) {
    match id {
        "helm" => ("HELM", "helm"),
        "tactical" => ("TACTICAL", "tactical"),
        "engineering" => ("ENGINEERING", "engineering"),
        "science" => ("SCIENCE", "science"),
        _ => ("CAPTAIN", "command"),
    }
}

/// The title band: the station's chip, the open stations to swap to, the captain's order, the condition and the clock.
fn title_band(cv: &Canvas, cx: &mut Ctx) {
    let st = vg::style();
    let (c, role) = (&st.c, &st.role);
    let v = cx.v;
    let (red, emerg) = (v.alert == "red_alert", v.alert == "emergency");
    let bg = if emerg {
        rgba(0x2b1a08, 1.0)
    } else if red {
        rgba(0x3a0c10, 1.0)
    } else {
        rgba(0x0a0f16, 1.0)
    };
    cv.rect(0.0, 0.0, 1280.0, 32.0, 0.0, bg, None);
    let line = if emerg {
        c.emergency.0
    } else if red {
        c.alert.0
    } else {
        c.line.0
    };
    cv.rect(0.0, 31.0, 1280.0, 1.0, 0.0, line, None);
    let mut x = 8.0;
    {
        let (w, r) = (v.word.as_str(), v.role.as_str());
        let tw = 34.0 + w.chars().count() as f32 * 9.4;
        let rc = role.of(r);
        cv.rect(x, 4.0, tw, 24.0, 5.0, alpha(rc, 0x26 as f32 / 255.0), Some(Pen::new(1.0, rc)));
        cv.icon(r, x + 13.0, 16.0, 14.0, rc);
        cv.text(x + 25.0, 21.0, 13.0, c.text.0, w, T::default().ls(1.5).bold());
        x += tw + 6.0;
    }
    let mut open: Vec<(String, String, Option<String>)> = v
        .open
        .iter()
        .map(|id| {
            let (w, r) = station_word(id);
            (w.to_owned(), r.to_owned(), Some(id.clone()))
        })
        .collect();
    open.push(("COMMS".into(), "comms".into(), None));
    open.push(("FLIGHT".into(), "flight_ops".into(), None));
    for (w, r, id) in open {
        let tw = 44.0 + w.chars().count() as f32 * 8.6;
        let on = id.is_some();
        cv.rect(x, 4.0, tw, 24.0, 5.0, Color32::TRANSPARENT, Some(Pen::new(1.0, c.line2.0).dash(4.0, 3.0, 0.0)));
        cv.icon(&r, x + 13.0, 16.0, 13.0, if on { role.of(&r) } else { c.faint.0 });
        cv.text(x + 24.0, 21.0, 12.0, if on { c.text.0 } else { c.faint.0 }, &w, T::default().ls(1.5).bold());
        let ac = if on { c.text.0 } else { c.faint.0 };
        let (ax, bx) = (x + tw - 15.0, x + tw - 7.0);
        cv.path(&format!("M{ax} 12h8l-3 -3M{bx} 20h-8l3 3"), Xf::ID, None, Some(Pen::new(1.5, ac)));
        let tip = match &id {
            Some(_) => format!("{} is open: swap to it", w.to_lowercase()),
            None => format!("{} is open (its console is not in this drill yet)", w.to_lowercase()),
        };
        let hit = id.map(Act::Swap);
        cx.hits.add(cv, x, 4.0, tw, 24.0, hit.map(kit::Hit::Act), Some(tip));
        x += tw + 6.0;
    }
    if let Some(o) = &v.order {
        cv.rect(470.0, 4.0, 300.0, 24.0, 5.0, alpha(c.warn.0, 0x1f as f32 / 255.0), Some(Pen::new(1.0, c.warn.0)));
        cv.icon("command", 486.0, 16.0, 14.0, c.warn.0);
        cv.text(500.0, 21.0, 13.0, c.text.0, &format!("CAPTAIN: {o}"), T::default().ls(1.5).bold());
        cv.icon("check", 754.0, 16.0, 14.0, c.warn.0);
        cx.hits.add(cv, 470.0, 4.0, 300.0, 24.0, Some(kit::Hit::Act(Act::Ack)), Some("Acknowledge (Y)".into()));
    }
    let cond = if emerg {
        "EMERGENCY POWER"
    } else if red {
        "RED ALERT"
    } else {
        "NORMAL"
    };
    let cc = if emerg {
        c.emergency.0
    } else if red {
        c.alert.0
    } else {
        c.ok.0
    };
    let cw = cond.len() as f32 * 9.4;
    cv.rect(1180.0 - cw - 22.0, 4.0, cw + 22.0, 24.0, 5.0, alpha(cc, 0x22 as f32 / 255.0), Some(Pen::new(1.0, cc)));
    cv.text(1170.0, 21.0, 13.0, cc, cond, T::end().ls(1.5).bold());
    let t = v.clock_s.max(0.0).floor() as i64;
    cv.text(1268.0, 21.0, 14.0, c.dim.0, &format!("{:02}:{:02}", t / 60, t % 60), T::end());
}

/// The viewscreen's camera, in ship axes: forward, right and up (the mockup's `feedBasis`).
pub fn feed_basis(v: &ConsoleView) -> Basis {
    let tgt = v.target.as_ref().and_then(|id| v.contacts.iter().find(|c| &c.id == id));
    let f = match v.feed.as_str() {
        "FWD" | "CHASE" => DVec3::Z,
        "AFT" => DVec3::NEG_Z,
        "PORT" => DVec3::X,
        "STBD" => DVec3::NEG_X,
        _ => tgt.map(|c| v3(c.rel).normalize_or_zero()).unwrap_or(DVec3::Z),
    };
    let mut r = f.cross(DVec3::Y);
    if r.length() < 1e-3 {
        r = DVec3::NEG_X;
    }
    let r = r.normalize();
    Basis { f, r, u: r.cross(f) }
}

/// The viewscreen's frame on screen (look band coordinates): where the 3D feed is drawn.
pub const VIEWSCREEN: [f32; 4] = [333.0, 21.0, 614.0, 194.0];

/// The look band: the room from the seat, here the viewscreen and its frame, the camera widget and the navball.
fn look_band(cv: &Canvas, cx: &mut Ctx) {
    let c = &vg::style().c;
    let v = cx.v;
    let [sx, sy, sw, sh] = VIEWSCREEN;
    let (g0, g1) = (rgba(0x0b1018, 1.0), rgba(0x05070b, 1.0));
    if cx.feed {
        // The band's gradient round the viewscreen's window.
        let at = |y: f32| lerp_col(g0, g1, y / 240.0);
        cv.vgrad(0.0, 0.0, 1280.0, sy, &[(0.0, g0), (1.0, at(sy))]);
        cv.vgrad(0.0, sy + sh, 1280.0, 240.0 - sy - sh, &[(0.0, at(sy + sh)), (1.0, g1)]);
        cv.vgrad(0.0, sy, sx, sh, &[(0.0, at(sy)), (1.0, at(sy + sh))]);
        cv.vgrad(sx + sw, sy, 1280.0 - sx - sw, sh, &[(0.0, at(sy)), (1.0, at(sy + sh))]);
    } else {
        cv.vgrad(0.0, 0.0, 1280.0, 240.0, &[(0.0, g0), (1.0, g1)]);
    }
    let side = Pen::new(1.0, rgba(0x18212d, 1.0));
    cv.path(
        "M0 240V150l180-30h120l40 120zM1280 240V150l-180-30H980l-40 120z",
        Xf::ID,
        Some(rgba(0x0d131c, 1.0)),
        Some(side),
    );
    let fill = if cx.feed { Color32::TRANSPARENT } else { rgba(0x020306, 1.0) };
    cv.rect(330.0, 18.0, 620.0, 200.0, 6.0, fill, Some(Pen::new(3.0, rgba(0x2a3442, 1.0))));
    // The target's bracket on the feed, where the feed's camera sees it.
    let scr = cv.at(sx, sy).clip(0.0, 0.0, sw, sh);
    let b = feed_basis(v);
    let foc = (307.0 / 35f64.to_radians().tan()) * v.zoom;
    for k in &v.contacts {
        if k.iff == "unknown" || Some(&k.id) != v.target.as_ref() {
            continue;
        }
        let d = v3(k.rel);
        let cz = d.dot(b.f);
        if cz <= 0.01 {
            continue;
        }
        let p = [307.0 + d.dot(b.r) / cz * foc, 97.0 - d.dot(b.u) / cz * foc];
        let sz = ((60.0 / (d.length() / 1000.0)).clamp(5.0, 70.0) * v.zoom) as f32;
        let (px, py) = (p[0] as f32, p[1] as f32);
        let side = sz * 1.8;
        cv_dashed_rect(&scr, px - sz * 0.9, py - sz * 0.9, side, side, c.hostile.0, side - 12.0);
    }
    cv.text(342.0, 38.0, 11.0, c.dim.0, &format!("VIEWSCREEN · {}", v.feed), T::default().ls(2.0));
    kit::feed_widget(cv, cx, 196.0, 10.0);
    kit::navball(cv, cx, 1046.0, 116.0, 76.0);
    if v.alert == "red_alert" {
        kit::vignette(cv, 1280.0, 240.0, c.alert.0, 0.35);
    }
}

/// A colour `k` of the way from `a` to `b`, channel by channel (as an SVG gradient between two stops).
fn lerp_col(a: Color32, b: Color32, k: f32) -> Color32 {
    let k = k.clamp(0.0, 1.0);
    let ch = |x: u8, y: u8| (f32::from(x) + (f32::from(y) - f32::from(x)) * k).round() as u8;
    Color32::from_rgb(ch(a.r(), b.r()), ch(a.g(), b.g()), ch(a.b(), b.b()))
}

/// The mockup's target bracket: a rectangle dashed `6 gap` from 3 in, so only its corners show.
fn cv_dashed_rect(cv: &Canvas, x: f32, y: f32, w: f32, h: f32, col: Color32, gap: f32) {
    let pts = [[x, y], [x + w, y], [x + w, y + h], [x, y + h], [x, y]];
    cv.polyline(&pts, Pen::new(1.0, col).dash(6.0, gap.max(0.0), 3.0));
}

/// The status strip: the same on every console, pictures and numbers only.
fn status_strip(cv: &Canvas, cx: &mut Ctx) {
    let st = vg::style();
    let c = &st.c;
    let v = cx.v;
    cv.rect(0.0, 0.0, 1280.0, 32.0, 0.0, rgba(0x0a0f16, 1.0), None);
    cv.rect(0.0, 0.0, 1280.0, 1.0, 0.0, c.line.0, None);
    let hp = v.hull_pct;
    let hc = if hp > 75.0 {
        c.ok.0
    } else if hp > 40.0 {
        c.warn.0
    } else {
        c.danger.0
    };
    cv.icon("hull", 22.0, 16.0, 16.0, hc);
    kit::bar(cv, 36.0, 11.0, 90.0, 10.0, hp / 100.0, hc, false);
    cv.text(132.0, 21.0, 14.0, c.text.0, &format!("{:.0}%", hp), T::default());
    cx.hits.add(cv, 14.0, 4.0, 160.0, 24.0, None, Some("Hull".into()));
    cv.icon("shields", 196.0, 16.0, 16.0, c.text.0);
    for f in 0..6 {
        let k = v.shield_ratio[f];
        kit::bar(cv, 212.0 + f as f32 * 16.0, 8.0, 11.0, 16.0, k, ratio_col(k), true);
    }
    cx.hits.add(
        cv,
        188.0,
        4.0,
        120.0,
        24.0,
        None,
        Some("Shields: bow, stern, port, starboard, dorsal, ventral".into()),
    );
    match v.power_mw {
        Some(pw) => {
            cv.icon("bolt", 330.0, 16.0, 16.0, if v.reactor_up { c.text.0 } else { c.emergency.0 });
            cv.text(344.0, 21.0, 14.0, c.text.0, &format!("{pw:.1} MW"), T::default());
            cx.hits.add(cv, 322.0, 4.0, 90.0, 24.0, None, Some("Power in use".into()));
        }
        None => {
            // Unavailable: the drill simulates no power grid yet (CLAUDE.md 10, a control's unavailable state).
            cv.icon("bolt", 330.0, 16.0, 16.0, c.faint.0);
            cv.text(344.0, 21.0, 14.0, c.faint.0, "-- MW", T::default());
            cx.hits.add(cv, 322.0, 4.0, 90.0, 24.0, None, Some("Power: not simulated in this drill".into()));
        }
    }
    cv.icon("speed", 432.0, 16.0, 16.0, c.text.0);
    cv.text(446.0, 21.0, 14.0, c.text.0, &format!("{:.0} m/s", v.speed_mps), T::default());
    cx.hits.add(cv, 424.0, 4.0, 100.0, 24.0, None, Some("Speed".into()));
    let (nf, nb) = (v.fires, v.breaches);
    let fc = if nf > 0 { c.danger.0 } else { c.faint.0 };
    cv.icon("flame", 540.0, 16.0, 16.0, fc);
    cv.text(554.0, 21.0, 14.0, fc, &nf.to_string(), T::default());
    cx.hits.add(cv, 532.0, 4.0, 40.0, 24.0, None, Some("Fires".into()));
    let bc = if nb > 0 { c.danger.0 } else { c.faint.0 };
    cv.icon("breach", 590.0, 16.0, 16.0, bc);
    cv.text(604.0, 21.0, 14.0, bc, &nb.to_string(), T::default());
    cx.hits.add(cv, 582.0, 4.0, 40.0, 24.0, None, Some("Compartments open to space".into()));
    cv.icon("person", 1090.0, 16.0, 16.0, accent(v));
    cv.text(1106.0, 21.0, 14.0, c.text.0, &format!("{} · {}", v.op.as_deref().unwrap_or("You"), v.word), T::default());
}

/// The grid cell of a panel: column, row, width and height in cells (the mockup's `grid`).
pub fn grid(col: f32, row: f32, w: f32, h: f32) -> [f32; 4] {
    [20.0 + 104.0 * col, 280.0 + 102.0 * row, 104.0 * w - 8.0, 102.0 * h - 8.0]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn euler_round_trips_through_the_quaternion() {
        for (h, p, r) in [(20.0, 0.0, 0.0), (45.0, 10.0, -30.0), (300.0, -60.0, 170.0)] {
            let (h2, p2, r2, g) = to_euler(from_euler(h, p, r));
            assert!(!g);
            assert!((wrap180(h2 - h)).abs() < 1e-6 && (p2 - p).abs() < 1e-6 && (wrap180(r2 - r)).abs() < 1e-6);
        }
    }

    #[test]
    fn the_mockups_starting_attitude_reads_as_its_quaternion() {
        // consoles.html starts the ship at heading 020: [0.9848, 0, -0.1736, 0].
        let q = quat_arr(from_euler(20.0, 0.0, 0.0));
        assert!((q[0] - 0.984_807_753).abs() < 1e-9 && (q[2] + 0.173_648_178).abs() < 1e-9);
        assert_eq!(fmt_q(q), " 0.9848   0.0000  -0.1736   0.0000");
    }

    #[test]
    fn every_mockup_fixture_parses() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/screenshots/parity");
        let Ok(rd) = std::fs::read_dir(&dir) else { return };
        for e in rd.flatten() {
            let p = e.path();
            // A state is a JSON beside its mockup picture; the comparison's report is not one.
            let shot = p.file_stem().and_then(|s| s.to_str()).unwrap_or("").to_owned();
            if p.extension().is_some_and(|x| x == "json") && dir.join(format!("{shot}-mockup.png")).exists() {
                let t = std::fs::read_to_string(&p).unwrap();
                serde_json::from_str::<ConsoleView>(&t).unwrap_or_else(|err| panic!("{}: {err}", p.display()));
            }
        }
    }
}
