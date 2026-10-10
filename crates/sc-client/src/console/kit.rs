//! The consoles' shared pieces, each a port of the mockup function it names: `button`, `bar`, `shipGlyph`, the 3D
//! `scanner` and its contacts, the viewscreen's `feedCone` and `feedWidget`, `rangeCtl`, the `navball`, the
//! `shieldView`, and the small 3D kit the helm's stick and flipper are drawn with (`lathe`, `drawMesh`). Coordinates,
//! sizes and colours are the mockup's; where a number looks arbitrary, it is the mockup's number.

use super::{
    accent, brg_of, cam_basis, elev_of, feed_basis, iff_col, ratio_col, tern, to_euler, v3, wrap180, wrap360, Basis,
    ConsoleView, Ctx,
};
use crate::vg::{self, alpha, rgba, Canvas, Pen, Xf, T};
use egui::{Color32, Pos2, Rect};
use glam::{DQuat, DVec3};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

/// What a press on a control does (the mockup's `data-act`).
#[derive(Clone, Debug, PartialEq)]
pub enum Act {
    Swap(String),
    Ack,
    AllStop,
    Ap(String),
    OGo,
    OLevel,
    OFlip,
    OTarget,
    Target(String),
    Range(&'static str, i32),
    ScanReset(&'static str),
    TMode(usize),
    Load(usize),
    Favour(i32),
    Feed(String),
    /// Engineering: a power preset, a group's priority, a group's breaker lock, SCRAM, the reactor's and the loop's
    /// modes.
    Preset(String),
    Prio(String),
    GLock(String),
    Scram,
    RxMode,
    CMode,
    /// Science: pick a contact, scan it, step the shield's frequency, put the shield view back, zoom the viewscreen.
    SciSel(String),
    Scan,
    Freq,
    ShieldView,
    Zoom(i32),
    /// The captain: a station to write to, a verb, send; brace; take the viewscreen; a room on the plan; a tab.
    Compose(String),
    Verb(String),
    Send,
    Brace,
    ViewTake,
    Room(String),
    Tab(String),
    OrderRepair,
}

/// What a drag on a control turns (the mockup's `data-drag`).
#[derive(Clone, Debug, PartialEq)]
pub enum Drag {
    Throttle,
    Strafe,
    Joy,
    Flip,
    Wheel(char),
    ScanCam(&'static str),
    FeedView,
    /// Engineering: a load group's fader, one of the cooling loop's levers.
    Fader(String),
    Lever(String),
    /// Science: the shield view's camera.
    ShieldCam,
}

/// A control: pressed, dragged or held (the mockup's `data-hold`, 0.6 s).
#[derive(Clone, Debug, PartialEq)]
pub enum Hit {
    Act(Act),
    Drag(Drag),
    Hold(String),
}

/// A control's place on screen this frame, what it does, and its tip.
#[derive(Clone, Debug)]
pub struct Region {
    pub rect: Rect,
    pub hit: Option<Hit>,
    pub tip: Option<String>,
    /// What a drag is measured against (a lever's track, a pad), screen points: the region itself unless given.
    pub track: Rect,
}

/// Every control drawn this frame, in drawing order: the last one under the pointer is the one on top.
#[derive(Default)]
pub struct Hits {
    pub regions: Vec<Region>,
}

impl Hits {
    /// Record a control at user (x, y, w, h) on `cv`, clipped as `cv` draws.
    #[allow(clippy::too_many_arguments)]
    pub fn add(&mut self, cv: &Canvas, x: f32, y: f32, w: f32, h: f32, hit: Option<Hit>, tip: Option<String>) {
        let rect = cv.r(x, y, w, h).intersect(cv.painter.clip_rect());
        if rect.width() > 0.0 && rect.height() > 0.0 {
            self.regions.push(Region { rect, hit, tip, track: cv.r(x, y, w, h) });
        }
    }

    /// A drag control whose drag is measured against `track` (user x, y, w, h), not its whole area.
    #[allow(clippy::too_many_arguments)]
    pub fn add_track(&mut self, cv: &Canvas, x: f32, y: f32, w: f32, h: f32, track: [f32; 4], hit: Hit, tip: &str) {
        let rect = cv.r(x, y, w, h).intersect(cv.painter.clip_rect());
        if rect.width() > 0.0 && rect.height() > 0.0 {
            let [tx, ty, tw, th] = track;
            self.regions.push(Region { rect, hit: Some(hit), tip: Some(tip.to_owned()), track: cv.r(tx, ty, tw, th) });
        }
    }

    /// The topmost control under a point (screen points).
    pub fn at(&self, p: Pos2) -> Option<&Region> {
        self.regions.iter().rev().find(|r| r.rect.contains(p) && r.hit.is_some())
    }

    /// The topmost tip under a point.
    pub fn tip_at(&self, p: Pos2) -> Option<&str> {
        self.regions.iter().rev().find(|r| r.rect.contains(p) && r.tip.is_some()).and_then(|r| r.tip.as_deref())
    }
}

/// The hover tip: the mockup's `#tip`, beside the pointer.
pub fn tooltip(cv: &Canvas, cx: &mut Ctx) {
    let Some(p) = cx.pointer else { return };
    let Some(tip) = cx.hits.tip_at(p).map(str::to_owned) else { return };
    let c = &vg::style().c;
    // Measured off screen, then drawn where it fits.
    let (x, y) = ((p.x - cv.o.x) / cv.s + 14.0, (p.y - cv.o.y) / cv.s + 18.0);
    let probe = Canvas::new(cv.painter.with_clip_rect(Rect::NOTHING), cv.o, cv.s);
    let w = probe.text(0.0, 0.0, 13.0, c.text.0, &tip, T::default().ls(0.39)) + 16.0;
    let x = x.min(1280.0 - w - 4.0);
    let y = if y + 26.0 > 720.0 { y - 46.0 } else { y };
    cv.rect(x, y, w, 24.0, 5.0, rgba(0x182231, 1.0), Some(Pen::new(1.0, rgba(0x2c3a4d, 1.0))));
    cv.text(x + 8.0, y + 17.0, 13.0, c.text.0, &tip, T::default().ls(0.39));
}

/// A button (the mockup's `button(b)`): a rounded box with an icon, a word, or both.
#[derive(Clone, Debug, Default)]
pub struct Btn<'a> {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub icon: Option<&'a str>,
    pub label: Option<&'a str>,
    pub act: Option<Act>,
    pub hold: Option<&'a str>,
    /// Lit.
    pub on: bool,
    /// Cannot act now: dimmed.
    pub off: bool,
    /// The colour when lit (default: the station's accent).
    pub col: Option<Color32>,
    /// The colour at rest (default: text).
    pub col2: Option<Color32>,
    /// The stroke at rest (default: line2).
    pub stroke: Option<Color32>,
    /// The fill when lit (default: the helm's accent at 16%).
    pub fill: Option<Color32>,
    /// Icon size.
    pub is: Option<f32>,
    /// Label size.
    pub fs: Option<f32>,
    /// Icon over label.
    pub stack: bool,
    pub tip: Option<String>,
}

pub fn button(cv: &Canvas, cx: &mut Ctx, b: Btn) {
    let c = &vg::style().c;
    let acc = accent(cx.v);
    let col = if b.off {
        c.faint.0
    } else if b.on {
        b.col.unwrap_or(acc)
    } else {
        b.col2.unwrap_or(c.text.0)
    };
    let fill = if b.on { b.fill.unwrap_or(rgba(0x4fc3f7, 0.16)) } else { c.panel2.0 };
    let stroke = if b.on { Pen::new(2.0, col) } else { Pen::new(1.2, b.stroke.unwrap_or(c.line2.0)) };
    cv.rect(b.x, b.y, b.w, b.h, 7.0, fill, Some(stroke));
    if let (Some(h), Some(hv)) = (b.hold, cx.v.hold.as_ref()) {
        if hv.key == h {
            let p = hv.k.clamp(0.0, 1.0) as f32;
            cv.rect(b.x, b.y, b.w * p, b.h, 7.0, alpha(b.col.unwrap_or(c.danger.0), 0.35), None);
        }
    }
    let cy = b.y + b.h / 2.0;
    match (b.icon, b.label) {
        (Some(ic), Some(lb)) => {
            let is = b.is.unwrap_or((b.h * 0.5).min(26.0));
            if b.stack {
                cv.icon(ic, b.x + b.w / 2.0, b.y + b.h * 0.4, is, col);
                cv.text(b.x + b.w / 2.0, b.y + b.h - 9.0, b.fs.unwrap_or(13.0), col, lb, T::mid().ls(1.0));
            } else {
                cv.icon(ic, b.x + 12.0 + is / 2.0, cy, is, col);
                cv.text(b.x + 20.0 + is, cy + 1.0, b.fs.unwrap_or(15.0), col, lb, T::default().ls(1.0).vmid());
            }
        }
        (Some(ic), None) => cv.icon(ic, b.x + b.w / 2.0, cy, b.is.unwrap_or((b.h * 0.6).min(28.0)), col),
        (None, Some(lb)) => {
            cv.text(b.x + b.w / 2.0, cy + 1.0, b.fs.unwrap_or(15.0), col, lb, T::mid().ls(1.2).vmid());
        }
        (None, None) => {}
    }
    let hit = if b.off {
        None
    } else if let Some(h) = b.hold {
        Some(Hit::Hold(h.to_owned()))
    } else {
        b.act.map(Hit::Act)
    };
    if hit.is_some() || b.tip.is_some() {
        cx.hits.add(cv, b.x, b.y, b.w, b.h, hit, b.tip);
    }
}

/// A bar: `k` of its length filled, horizontal or (from the bottom) vertical.
#[allow(clippy::too_many_arguments)]
pub fn bar(cv: &Canvas, x: f32, y: f32, w: f32, h: f32, k: f64, col: Color32, vertical: bool) {
    let k = k.clamp(0.0, 1.0) as f32;
    cv.rect(x, y, w, h, 2.0, rgba(0x16202c, 1.0), None);
    if vertical {
        cv.rect(x, y + h * (1.0 - k), w, h * k, 2.0, col, None);
    } else {
        cv.rect(x, y, w * k, h, 2.0, col, None);
    }
}

/// A ship seen from above, pointing along `hdg` (degrees, on screen), `s` across, in its IFF colour; `sel` rings it.
#[allow(clippy::too_many_arguments)]
pub fn ship_glyph(cv: &Canvas, cls: &str, iff: &str, x: f32, y: f32, s: f32, hdg: f32, col: Color32, sel: bool) {
    let xf = Xf::tr(x, y).rot(hdg).sc(s / 24.0);
    if iff == "unknown" {
        cv.path("M0 -10L10 0L0 10L-10 0Z", xf, None, Some(Pen::new(2.5 * 24.0 / s, col)));
    } else {
        let d = match cls {
            "fighter" => "M0 -10L4 2L10 6L0 4L-10 6L-4 2Z",
            "freighter" => "M-4 -11H4V11H-4ZM-7 -4H7V4H-7Z",
            _ => "M0 -12L7 8L0 4L-7 8Z",
        };
        cv.path(d, xf, Some(col), None);
    }
    if sel {
        cv.circle(x, y, s * 0.75, Color32::TRANSPARENT, Some(Pen::new(1.5, col).dash(4.0, 3.0, 0.0)));
    }
}

/// A 3D scanner (the mockup's `scanner`): the ship's plane seen from aft and above as an ellipse, bow up.
pub struct Scan {
    pub cx: f64,
    pub cy: f64,
    pub r: f64,
    pub k: f64,
    pub el: f64,
    pub yaw: f64,
    pub h: f64,
    pub hdg: f64,
    pub range_km: f64,
    b: Basis,
}

/// A point on a scanner: on the plane (by) and above or below it (y).
pub struct At {
    pub x: f64,
    pub by: f64,
    pub y: f64,
    pub inside: bool,
}

impl Scan {
    pub fn new(v: &ConsoleView, w: f64, h: f64, key: &str, cy0: Option<f64>) -> Scan {
        let range_km = match key {
            "nav" => v.nav_km,
            "tac" => v.tac_km,
            _ => v.sci_km,
        };
        let vw = v.scan_cam.of(key);
        let cx = w / 2.0;
        let cy = cy0.unwrap_or(h / 2.0 + 4.0);
        let r0 = w / 2.0 - 16.0;
        let k0 = ((h / 2.0 - 22.0) / r0).clamp(0.42, 0.62);
        let el = (k0.asin().to_degrees() + vw.d_el).clamp(12.0, 90.0);
        let k = el.to_radians().sin();
        let r = r0.min((h / 2.0 - 22.0) / k);
        let hh = el.to_radians().cos() * 0.4;
        let b = cam_basis(vw.yaw, el);
        let mut s = Scan { cx, cy, r, k, el, yaw: vw.yaw, h: hh, hdg: 0.0, range_km, b };
        let [ax, ay] = s.dir(0.0, 1.0);
        s.hdg = (ax - cx).atan2(-(ay - cy)).to_degrees();
        s
    }
    fn proj(&self, v: DVec3) -> At {
        let fl = DVec3::new(v.x, 0.0, v.z);
        let x = self.cx + fl.dot(self.b.r) * self.r;
        let by = self.cy - fl.dot(self.b.u) * self.r;
        At { x, by, y: by - v.y * self.r * self.h, inside: true }
    }
    /// A ship-axes point (metres from the ship) on the scanner.
    pub fn at(&self, rel: DVec3) -> At {
        let r = rel / (self.range_km * 1000.0);
        At { inside: r.length() <= 1.0, ..self.proj(r) }
    }
    /// The point on the plane at bearing `a` (degrees to starboard) and `k` of the range.
    pub fn dir(&self, a: f64, k: f64) -> [f64; 2] {
        let a = a.to_radians();
        let q = self.proj(DVec3::new(-a.sin() * k, 0.0, a.cos() * k));
        [q.x, q.by]
    }
    /// The plane: its rim and half-range ellipses, the bow-stern and beam lines, a dot every 30 degrees.
    pub fn draw_base(&self, cv: &Canvas) {
        let c = &vg::style().c;
        let (cx, cy, r, k) = (self.cx as f32, self.cy as f32, self.r as f32, self.k as f32);
        cv.ellipse(cx, cy, r, r * k, rgba(0x0a1017, 1.0), Some(Pen::new(1.0, c.line2.0)));
        cv.ellipse(cx, cy, r / 2.0, r * k / 2.0, Color32::TRANSPARENT, Some(Pen::new(1.0, c.line.0)));
        let [ax, ay] = self.dir(0.0, 1.0);
        let [bx, by] = self.dir(180.0, 1.0);
        let [px, py] = self.dir(270.0, 1.0);
        let [qx, qy] = self.dir(90.0, 1.0);
        let pen = Pen::new(1.0, c.line.0).dash(2.0, 6.0, 0.0);
        cv.seg(ax as f32, ay as f32, bx as f32, by as f32, pen);
        cv.seg(px as f32, py as f32, qx as f32, qy as f32, pen);
        let mut a = 30.0;
        while a < 360.0 {
            let [x, y] = self.dir(a, 1.0);
            cv.circle(x as f32, y as f32, 1.4, c.faint.0, None);
            a += 30.0;
        }
    }
}

/// A scanner's drag area: the wheel turns under a drag, and a scroll steps its range.
pub fn scan_wrap(cv: &Canvas, cx: &mut Ctx, key: &'static str, w: f32, h: f32) {
    cx.hits.add(
        cv,
        0.0,
        0.0,
        w,
        h,
        Some(Hit::Drag(Drag::ScanCam(key))),
        Some("Drag to turn the plot; scroll for range".into()),
    );
}

/// The scanner's reset: back to bow up.
pub fn scan_reset(cv: &Canvas, cx: &mut Ctx, x: f32, y: f32, key: &'static str) {
    let v = cx.v.scan_cam.of(key);
    let off = v.yaw == 0.0 && v.d_el == 0.0;
    button(
        cv,
        cx,
        Btn {
            x,
            y,
            w: 34.0,
            h: 30.0,
            icon: Some("reset"),
            is: Some(18.0),
            act: Some(Act::ScanReset(key)),
            off,
            tip: Some("Plot back to bow up".into()),
            ..Default::default()
        },
    );
}

/// A contact's range in km.
pub fn range_km(rel: [f64; 3]) -> f64 {
    v3(rel).length() / 1000.0
}

/// Every live contact on a scanner, on its stalk, in its IFF colour; a press on one targets it, or on Science's
/// scanner (`sci`) picks it to scan.
pub fn scanner_contacts(cv: &Canvas, cx: &mut Ctx, s: &Scan, sci: bool) {
    let v = cx.v;
    let mut list: Vec<(&super::ContactView, At)> =
        v.contacts.iter().filter(|k| k.hull > 0.0).map(|k| (k, s.at(v3(k.rel)))).filter(|(_, p)| p.inside).collect();
    list.sort_by(|a, b| a.1.by.total_cmp(&b.1.by));
    for (k, p) in list {
        let ahead = s.at(v3(k.rel) + v3(k.dir) * 1500.0);
        let rot = (ahead.x - p.x).atan2(-(ahead.y - p.y)).to_degrees() as f32;
        let el = elev_of(v3(k.rel)).round() as i64;
        let op = if el < 0 { 0.5 } else { 1.0 };
        let col = alpha(iff_col(&k.iff), op);
        let (px, pby, py) = (p.x as f32, p.by as f32, p.y as f32);
        cv.ellipse(px, pby, 8.0, 3.2, alpha(col, 0.22), Some(Pen::new(1.5, col)));
        let mut stalk = Pen::new(2.0, alpha(col, 0.85));
        if el < 0 {
            stalk = stalk.dash(4.0, 3.0, 0.0);
        }
        cv.seg(px, pby, px, py, stalk);
        let (lx, ly) = (px + 13.0, py + 1.0);
        let km = range_km(k.rel);
        if el != 0 {
            let d =
                if el > 0 { format!("M{lx} {}l4 -7l4 7z", ly + 3.0) } else { format!("M{lx} {}l4 7l4 -7z", ly - 4.0) };
            cv.path(&d, Xf::ID, Some(col), None);
        }
        let t = if km < 10.0 { format!("{km:.1} km") } else { format!("{} km", js_round(km)) };
        cv.text(lx + if el != 0 { 11.0 } else { 0.0 }, ly + 4.0, 12.0, col, &t, T::default().bold().ls(0.0));
        let sci_sel = v.sci.as_ref().and_then(|s| s.sel.as_ref());
        let sel = Some(&k.id) == v.target.as_ref() || (v.station == "science" && Some(&k.id) == sci_sel);
        ship_glyph(cv, &k.cls, &k.iff, px, py, 18.0, rot, col, sel);
        let elev = js_round(elev_of(v3(k.rel)));
        let (tip, act) = if sci {
            let name = if k.iff == "unknown" { "Unknown" } else { k.id.as_str() };
            (format!("{name} · {km:.1} km · elevation {elev}°"), Act::SciSel(k.id.clone()))
        } else {
            (format!("{} · {:.1} km · elevation {}°", k.id, km, elev), Act::Target(k.id.clone()))
        };
        cx.hits.add(cv, px - 11.0, py - 11.0, 22.0, 22.0, Some(Hit::Act(act)), Some(tip));
    }
}

/// The mockup's `pol`: the point `r` from (cx, cy) at bearing `a` degrees (0 up, clockwise).
pub fn pol(cx: f32, cy: f32, r: f32, a: f64) -> [f32; 2] {
    let a = a.to_radians();
    [cx + r * a.sin() as f32, cy - r * a.cos() as f32]
}

/// The mockup's `arcPath`: an SVG path along a circle from bearing a0 to a1 (degrees, 0 up, clockwise); a whole
/// circle from 359.9 degrees on.
pub fn arc_path(cx: f32, cy: f32, r: f32, a0: f64, a1: f64) -> String {
    if a1 - a0 >= 359.9 {
        return format!("M{} {cy}a{r} {r} 0 1 0 {} 0a{r} {r} 0 1 0 {} 0", cx - r, 2.0 * r, -2.0 * r);
    }
    let [x0, y0] = pol(cx, cy, r, a0);
    let [x1, y1] = pol(cx, cy, r, a1);
    format!("M{x0:.1} {y0:.1}A{r} {r} 0 {} 1 {x1:.1} {y1:.1}", if a1 - a0 > 180.0 { 1 } else { 0 })
}

/// JavaScript's `Math.round`: halves go up.
pub fn js_round(x: f64) -> i64 {
    (x + 0.5).floor() as i64
}

/// Where the viewscreen looks: its forward axis, bearing and elevation in degrees.
pub fn feed_dir(v: &ConsoleView) -> (DVec3, f64, i64) {
    let f = feed_basis(v).f;
    (f, brg_of(f), js_round(elev_of(f)))
}

/// The viewscreen's camera on a scanner: an icon at the ship and its field of view as a cone on the plane.
pub fn feed_cone(cv: &Canvas, cx: &mut Ctx, s: &Scan) {
    let v = cx.v;
    let (_, brg, el) = feed_dir(v);
    let half = 35.0 / v.zoom;
    let len = 0.34;
    let [x0, y0] = s.dir(brg - half, len);
    let [x1, y1] = s.dir(brg + half, len);
    let [ix, iy] = s.dir(brg, len * 0.32);
    let rot = (ix - s.cx).atan2(-(iy - s.cy)).to_degrees() as f32;
    let fc = rgba(0xcfe3f5, 1.0);
    let tri = [[s.cx as f32, s.cy as f32], [x0 as f32, y0 as f32], [x1 as f32, y1 as f32]];
    cv.poly(&tri, alpha(fc, 0.09));
    let mut closed = tri.to_vec();
    closed.push(tri[0]);
    cv.polyline(&closed, Pen::new(1.0, alpha(fc, 0.35)).dash(3.0, 3.0, 0.0));
    let xf = Xf::tr(ix as f32, iy as f32).rot(rot);
    cv.path("M-3 -2H3A2 2 0 0 1 5 0V5A2 2 0 0 1 3 7H-3A2 2 0 0 1 -5 5V0A2 2 0 0 1 -3 -2Z", xf, Some(fc), None);
    cv.path("M-4 -2L-7 -8H7L4 -2Z", xf, Some(fc), None);
    let (ixf, iyf) = (ix as f32, iy as f32);
    if el.abs() >= 3 {
        let (lx, ly) = (ixf + 10.0, iyf - 8.0);
        let d = if el > 0 { format!("M{lx} {}l4 -7l4 7z", ly + 3.0) } else { format!("M{lx} {}l4 7l4 -7z", ly - 4.0) };
        cv.path(&d, Xf::ID, Some(fc), None);
        let t = if el > 0 { format!("+{el}") } else { el.to_string() };
        cv.text(lx + 11.0, ly + 4.0, 11.0, fc, &t, T::default().bold().ls(0.0));
    }
    let tip = format!(
        "Viewscreen: {}, bearing {:03}, elevation {}°",
        v.feed.to_lowercase(),
        js_round(wrap360(brg)).rem_euclid(360),
        el
    );
    cx.hits.add(cv, ixf - 9.0, iyf - 9.0, 18.0, 18.0, None, Some(tip));
}

/// The range of a scanner: its number between - and +.
pub fn range_ctl(cv: &Canvas, cx: &mut Ctx, x: f32, y: f32, key: &'static str) {
    let c = &vg::style().c;
    let km = match key {
        "nav" => cx.v.nav_km,
        "tac" => cx.v.tac_km,
        _ => cx.v.sci_km,
    };
    button(
        cv,
        cx,
        Btn {
            x,
            y,
            w: 32.0,
            h: 30.0,
            label: Some("\u{2212}"),
            act: Some(Act::Range(key, -1)),
            fs: Some(20.0),
            off: km <= 5.0,
            tip: Some("Closer (scroll)".into()),
            ..Default::default()
        },
    );
    cv.rect(x + 36.0, y, 64.0, 30.0, 6.0, c.panel2.0, Some(Pen::new(1.0, c.line2.0)));
    cv.text(x + 68.0, y + 20.0, 15.0, c.text.0, &format!("{} km", js_round(km)), T::mid().bold());
    button(
        cv,
        cx,
        Btn {
            x: x + 104.0,
            y,
            w: 32.0,
            h: 30.0,
            label: Some("+"),
            act: Some(Act::Range(key, 1)),
            fs: Some(20.0),
            off: km >= 100.0,
            tip: Some("Further (scroll)".into()),
            ..Default::default()
        },
    );
}

// ------------------------------------------------------------------------------------------------ the 3D kit

/// A flat polygon of a model: corners, outward normal, colour (0-255).
#[derive(Clone, Debug)]
pub struct Poly {
    pub v: Vec<DVec3>,
    pub n: DVec3,
    pub col: [f64; 3],
}

/// A surface of revolution about +Y from a profile [(r, y, sharp)], bottom to top (the mockup's `lathe`).
pub fn lathe(profile: &[(f64, f64, bool)], col: [f64; 3], n: usize, smooth: bool) -> Vec<Poly> {
    let seg: Vec<[f64; 2]> = profile
        .windows(2)
        .map(|w| {
            let (nr, ny) = (w[1].1 - w[0].1, -(w[1].0 - w[0].0));
            let l = nr.hypot(ny).max(1e-12);
            [nr / l, ny / l]
        })
        .collect();
    let vn: Vec<Option<[f64; 2]>> = (0..profile.len())
        .map(|i| {
            if i == 0 || i >= seg.len() || profile[i].2 {
                return None;
            }
            let (a, b) = (seg[i - 1], seg[i]);
            let (x, y) = (a[0] + b[0], a[1] + b[1]);
            let l = x.hypot(y).max(1e-12);
            Some([x / l, y / l])
        })
        .collect();
    let mut polys = Vec::new();
    for i in 0..seg.len() {
        let (r0, y0) = (profile[i].0, profile[i].1);
        let (r1, y1) = (profile[i + 1].0, profile[i + 1].1);
        let [mut nr, mut ny] = seg[i];
        if smooth {
            let e0 = vn[i].unwrap_or(seg[i]);
            let e1 = vn[i + 1].unwrap_or(seg[i]);
            let (x, y) = (e0[0] + e1[0], e0[1] + e1[1]);
            let l = x.hypot(y).max(1e-12);
            nr = x / l;
            ny = y / l;
        }
        for j in 0..n {
            let a0 = j as f64 / n as f64 * std::f64::consts::TAU;
            let a1 = (j + 1) as f64 / n as f64 * std::f64::consts::TAU;
            let am = (a0 + a1) / 2.0;
            let v = vec![
                DVec3::new(r0 * a0.cos(), y0, r0 * a0.sin()),
                DVec3::new(r1 * a0.cos(), y1, r1 * a0.sin()),
                DVec3::new(r1 * a1.cos(), y1, r1 * a1.sin()),
                DVec3::new(r0 * a1.cos(), y0, r0 * a1.sin()),
            ];
            polys.push(Poly { v, n: DVec3::new(nr * am.cos(), ny, nr * am.sin()), col });
        }
    }
    polys
}

/// A smooth profile through control points (Catmull-Rom), `k` samples a span, ends kept.
pub fn smooth_profile(pts: &[(f64, f64)], k: usize) -> Vec<(f64, f64, bool)> {
    let mut out = Vec::new();
    for i in 0..pts.len() - 1 {
        let p0 = pts[i.saturating_sub(1)];
        let (p1, p2) = (pts[i], pts[i + 1]);
        let p3 = pts[(i + 2).min(pts.len() - 1)];
        for t in 0..k {
            let u = t as f64 / k as f64;
            let (u2, u3) = (u * u, u * u * u);
            let f = |a: f64, b: f64, c: f64, d: f64| {
                0.5 * (2.0 * b
                    + (-a + c) * u
                    + (2.0 * a - 5.0 * b + 4.0 * c - d) * u2
                    + (-a + 3.0 * b - 3.0 * c + d) * u3)
            };
            out.push((f(p0.0, p1.0, p2.0, p3.0).max(0.0), f(p0.1, p1.1, p2.1, p3.1), false));
        }
    }
    let last = pts[pts.len() - 1];
    out.push((last.0, last.1, false));
    out
}

/// A box centred at `c` with half sizes `h`.
pub fn box_mesh(c: DVec3, h: DVec3, col: [f64; 3]) -> Vec<Poly> {
    let p = |x: f64, y: f64, z: f64| DVec3::new(c.x + x * h.x, c.y + y * h.y, c.z + z * h.z);
    let q = |v: [DVec3; 4], n: [f64; 3]| Poly { v: v.to_vec(), n: DVec3::from_array(n), col };
    vec![
        q([p(1., -1., -1.), p(1., 1., -1.), p(1., 1., 1.), p(1., -1., 1.)], [1., 0., 0.]),
        q([p(-1., -1., 1.), p(-1., 1., 1.), p(-1., 1., -1.), p(-1., -1., -1.)], [-1., 0., 0.]),
        q([p(-1., 1., -1.), p(-1., 1., 1.), p(1., 1., 1.), p(1., 1., -1.)], [0., 1., 0.]),
        q([p(-1., -1., 1.), p(-1., -1., -1.), p(1., -1., -1.), p(1., -1., 1.)], [0., -1., 0.]),
        q([p(-1., -1., 1.), p(1., -1., 1.), p(1., 1., 1.), p(-1., 1., 1.)], [0., 0., 1.]),
        q([p(1., -1., -1.), p(-1., -1., -1.), p(-1., 1., -1.), p(1., 1., -1.)], [0., 0., -1.]),
    ]
}

/// Polygons turned about a pivot by a rotation, normals with them.
pub fn turn_mesh(polys: &[Poly], q: DQuat, pivot: DVec3) -> Vec<Poly> {
    polys
        .iter()
        .map(|p| Poly { v: p.v.iter().map(|x| q * (*x - pivot) + pivot).collect(), n: q * p.n, col: p.col })
        .collect()
}

type Drawn = Vec<(Vec<[f32; 2]>, Color32)>;

/// An edge between two points snapped to 1/64 of a unit, smaller first: shared by the polygons on both its sides.
type EdgeKey = ((i32, i32), (i32, i32));

/// Polygons seen from a camera: back faces dropped, the rest lit and sorted far to near (the mockup's `drawMesh`).
pub fn project_mesh(polys: &[Poly], yaw: f64, el: f64, cx: f64, cy: f64, sc: f64) -> Drawn {
    let b = cam_basis(yaw, el);
    let light = (-b.f + DVec3::new(0.5, 0.9, 0.2)).normalize();
    let half = (light - b.f).normalize();
    let mut vis: Vec<(f64, Vec<[f32; 2]>, Color32)> = Vec::with_capacity(polys.len());
    for p in polys {
        if p.n.dot(b.f) >= 0.0 {
            continue;
        }
        let mid = p.v.iter().fold(DVec3::ZERO, |a, x| a + *x) / p.v.len() as f64;
        let d = p.n.dot(light).max(0.0);
        let k = 0.28 + 0.72 * d;
        let spec = p.n.dot(half).max(0.0).powi(24) * 70.0;
        let ch = |c: f64| (c * k + spec).round().min(255.0) as u8;
        let col = Color32::from_rgb(ch(p.col[0]), ch(p.col[1]), ch(p.col[2]));
        let pts = p.v.iter().map(|x| [(cx + x.dot(b.r) * sc) as f32, (cy - x.dot(b.u) * sc) as f32]).collect();
        vis.push((mid.dot(b.f), pts, col));
    }
    vis.sort_by(|a, b| b.0.total_cmp(&a.0));
    vis.into_iter().map(|(_, p, c)| (p, c)).collect()
}

/// Draw projected polygons as one mesh, far to near, and stroke the silhouette: the mockup strokes every polygon 0.6
/// wide in its own colour so neighbours meet without a seam; one mesh has no seams to hide, so only the edges no other
/// visible polygon shares (the outline, where anti-aliasing shows) take the stroke. Thousands of polygons stay a few
/// thousand vertices.
pub fn draw_projected(cv: &Canvas, drawn: &Drawn) {
    let mut verts: Vec<([f32; 2], Color32)> = Vec::new();
    let mut idx: Vec<u32> = Vec::new();
    let q = |p: [f32; 2]| ((p[0] * 64.0).round() as i32, (p[1] * 64.0).round() as i32);
    let mut edge_at: HashMap<EdgeKey, usize> = HashMap::new();
    let mut edges: Vec<(u32, [f32; 2], [f32; 2], Color32)> = Vec::new();
    for (pts, col) in drawn {
        let n = pts.len();
        if n < 3 {
            continue;
        }
        let base = verts.len() as u32;
        verts.extend(pts.iter().map(|p| (*p, *col)));
        for i in 1..n - 1 {
            idx.extend([base, base + i as u32, base + i as u32 + 1]);
        }
        for i in 0..n {
            let (a, b) = (pts[i], pts[(i + 1) % n]);
            let (ka, kb) = (q(a), q(b));
            if ka == kb {
                continue;
            }
            let key = if ka < kb { (ka, kb) } else { (kb, ka) };
            match edge_at.get(&key) {
                Some(&j) => {
                    edges[j].0 += 1;
                    edges[j].3 = *col;
                }
                None => {
                    edge_at.insert(key, edges.len());
                    edges.push((1, a, b, *col));
                }
            }
        }
    }
    cv.mesh(&verts, idx);
    for (count, a, b, col) in edges {
        if count == 1 {
            cv.seg(a[0], a[1], b[0], b[1], Pen::new(0.6, col).round());
        }
    }
}

/// A small memo for meshes whose picture depends only on a few numbers (the mockup's `JOY_MEMO`).
fn memo(key: String, make: impl FnOnce() -> Drawn) -> Drawn {
    static MEMO: OnceLock<Mutex<HashMap<String, Drawn>>> = OnceLock::new();
    let m = MEMO.get_or_init(|| Mutex::new(HashMap::new()));
    let mut g = m.lock().expect("the mesh memo is never poisoned");
    if let Some(d) = g.get(&key) {
        return d.clone();
    }
    let d = make();
    if g.len() > 32 {
        g.clear();
    }
    g.insert(key, d.clone());
    d
}

struct Models {
    base: Vec<Poly>,
    boot: Vec<Poly>,
    stick: Vec<Poly>,
    housing: Vec<Poly>,
    boss: Vec<Poly>,
    bat: Vec<Poly>,
    hull: Vec<Poly>,
}

fn sharp(p: &[(f64, f64)], at: &[usize]) -> Vec<(f64, f64, bool)> {
    p.iter().enumerate().map(|(i, q)| (q.0, q.1, at.contains(&i))).collect()
}

fn models() -> &'static Models {
    static M: OnceLock<Models> = OnceLock::new();
    M.get_or_init(|| {
        let base = lathe(
            &sharp(
                &[(0.0, -0.3), (1.0, -0.3), (1.06, -0.12), (1.04, -0.02), (0.96, 0.02), (0.42, 0.02), (0.0, 0.02)],
                &[1, 4],
            ),
            [34.0, 43.0, 54.0],
            40,
            true,
        );
        let boot = lathe(
            &smooth_profile(&[(0.0, 0.0), (0.4, 0.02), (0.34, 0.12), (0.22, 0.28), (0.12, 0.4), (0.0, 0.44)], 4),
            [26.0, 31.0, 38.0],
            32,
            true,
        );
        let mut stick = lathe(
            &sharp(&[(0.0, 0.3), (0.075, 0.3), (0.075, 1.22), (0.0, 1.22)], &[1, 2]),
            [150.0, 160.0, 172.0],
            24,
            true,
        );
        stick.extend(lathe(
            &smooth_profile(
                &[
                    (0.0, 1.1),
                    (0.15, 1.13),
                    (0.21, 1.28),
                    (0.235, 1.5),
                    (0.22, 1.7),
                    (0.16, 1.86),
                    (0.07, 1.93),
                    (0.0, 1.95),
                ],
                4,
            ),
            [62.0, 72.0, 86.0],
            36,
            true,
        ));
        let housing = box_mesh(DVec3::new(0.0, -0.2, 0.0), DVec3::new(0.62, 0.2, 0.5), [30.0, 38.0, 48.0]);
        let boss = lathe(
            &sharp(&[(0.0, 0.0), (0.3, 0.0), (0.3, 0.1), (0.22, 0.16), (0.0, 0.16)], &[]),
            [44.0, 54.0, 66.0],
            16,
            false,
        );
        let mut bat = box_mesh(DVec3::new(0.0, 0.62, 0.0), DVec3::new(0.11, 0.5, 0.08), [150.0, 160.0, 172.0]);
        bat.extend(box_mesh(DVec3::new(0.0, 1.12, 0.0), DVec3::new(0.15, 0.06, 0.1), [62.0, 72.0, 86.0]));
        Models { base, boot, stick, housing, boss, bat, hull: hull_polys() }
    })
}

/// The hull loft as polygons with outward normals (the mockup's `HULL_POLYS`).
fn hull_polys() -> Vec<Poly> {
    let hull = &tern().hull;
    let col = [96.0, 110.0, 128.0];
    let mut polys = Vec::new();
    for s in 0..hull.len() - 1 {
        for k in 0..8 {
            let (a, b) = (hull[s][k], hull[s][(k + 1) % 8]);
            let (c, d) = (hull[s + 1][(k + 1) % 8], hull[s + 1][k]);
            let mut n = (b - a).cross(d - a).normalize_or_zero();
            let mid = (a + b + c + d) * 0.25;
            if n.dot(DVec3::new(mid.x, mid.y - (hull[s][0].y + hull[s][4].y) / 2.0, 0.0)) < 0.0 {
                n = -n;
            }
            polys.push(Poly { v: vec![a, b, c, d], n, col });
        }
    }
    polys.push(Poly { v: hull[0].to_vec(), n: DVec3::Z, col });
    polys.push(Poly { v: hull[hull.len() - 1].to_vec(), n: DVec3::NEG_Z, col });
    polys
}

// ------------------------------------------------------------------------------------------------ the look band

/// Left of the viewscreen: the ship in 3D with the camera's cone and every contact's direction, and the feed buttons.
pub fn feed_widget(cv: &Canvas, cx: &mut Ctx, x: f32, y: f32) {
    let v = cx.v;
    let (wx, wy) = (x as f64 + 60.0, y as f64 + 66.0);
    let sc = 1.05;
    let cam = v.feed_cam;
    let b = cam_basis(cam.yaw, cam.el);
    let pr = |p: DVec3| [(wx + p.dot(b.r) * sc) as f32, (wy - p.dot(b.u) * sc) as f32, p.dot(b.f) as f32];
    cv.rect(x, y, 120.0, 134.0, 8.0, rgba(0x070a0f, 1.0), Some(Pen::new(1.0, rgba(0x26313e, 1.0))));
    let inner = cv.clip(x, y, 120.0, 134.0);
    let key = format!("feed|{:.2}|{:.2}|{x}|{y}", cam.yaw, cam.el);
    let hull = memo(key, || project_mesh(&models().hull, cam.yaw, cam.el, wx, wy, sc));
    draw_projected(&inner, &hull);
    let (f, _, _) = feed_dir(v);
    let half = (35.0 / v.zoom).to_radians();
    let lc = 42.0;
    let a = if f.y.abs() > 0.9 { f.cross(DVec3::X) } else { f.cross(DVec3::Y) }.normalize();
    let bb = f.cross(a);
    let ring: Vec<[f32; 3]> = (0..12)
        .map(|t| {
            let th = t as f64 / 12.0 * std::f64::consts::TAU;
            pr(f * lc + (a * th.cos() + bb * th.sin()) * lc * half.tan())
        })
        .collect();
    let o = pr(DVec3::ZERO);
    let fc = rgba(0xcfe3f5, 1.0);
    for t in 0..12 {
        let (q, q2) = (ring[t], ring[(t + 1) % 12]);
        inner.poly(&[[o[0], o[1]], [q[0], q[1]], [q2[0], q2[1]]], alpha(fc, 0.07));
    }
    let mut loop_pts: Vec<[f32; 2]> = ring.iter().map(|q| [q[0], q[1]]).collect();
    loop_pts.push(loop_pts[0]);
    inner.polyline(&loop_pts, Pen::new(1.0, alpha(fc, 0.7)));
    inner.circle(o[0], o[1], 3.0, rgba(0xe8eef6, 1.0), None);
    let mut marks: Vec<(&super::ContactView, [f32; 3])> =
        v.contacts.iter().filter(|k| k.hull > 0.0).map(|k| (k, pr(v3(k.rel).normalize_or_zero() * 50.0))).collect();
    marks.sort_by(|m1, m2| m2.1[2].total_cmp(&m1.1[2]));
    for (k, q) in marks {
        let col = iff_col(&k.iff);
        let near = q[2] < 0.0;
        let r0 = if near { 4.0 } else { 3.0 };
        inner.seg(o[0], o[1], q[0], q[1], Pen::new(1.0, alpha(col, if near { 0.7 } else { 0.35 })).dash(2.0, 3.0, 0.0));
        let op = if near { 1.0 } else { 0.55 };
        if k.iff == "unknown" {
            let d = format!("M0 {}L{r0} 0L0 {r0}L{} 0Z", -r0, -r0);
            inner.path(
                &d,
                Xf::tr(q[0], q[1]).sc(std::f32::consts::SQRT_2),
                None,
                Some(Pen::new(1.5 / std::f32::consts::SQRT_2, alpha(col, op))),
            );
        } else {
            inner.circle(q[0], q[1], r0, alpha(col, op), None);
        }
        if Some(&k.id) == v.target.as_ref() {
            inner.circle(q[0], q[1], r0 + 4.0, Color32::TRANSPARENT, Some(Pen::new(1.5, col).dash(3.0, 2.0, 0.0)));
        }
    }
    cx.hits.add(
        cv,
        x,
        y,
        120.0,
        134.0,
        Some(Hit::Drag(Drag::FeedView)),
        Some("Drag to look round the ship; the cone is where the viewscreen looks".into()),
    );
    const PAD: [(&str, &str); 6] = [
        ("TARGET", "target"),
        ("FWD", "fwd"),
        ("CHASE", "chasecam"),
        ("PORT", "port"),
        ("AFT", "aft"),
        ("STBD", "stbd"),
    ];
    for (i, (k, ic)) in PAD.iter().enumerate() {
        button(
            cv,
            cx,
            Btn {
                x: x + (i % 3) as f32 * 41.0,
                y: y + 140.0 + (i / 3) as f32 * 28.0,
                w: 38.0,
                h: 25.0,
                icon: Some(ic),
                is: Some(15.0),
                act: Some(Act::Feed((*k).to_owned())),
                on: v.feed == *k,
                tip: Some(format!("Viewscreen: {}", k.to_lowercase())),
                ..Default::default()
            },
        );
    }
}

/// A convex polygon clipped to the half-plane where `side(p) >= 0` (one Sutherland-Hodgman pass).
fn clip_half(poly: &[[f32; 2]], side: impl Fn([f32; 2]) -> f32) -> Vec<[f32; 2]> {
    let mut out = Vec::new();
    for i in 0..poly.len() {
        let (a, b) = (poly[i], poly[(i + 1) % poly.len()]);
        let (sa, sb) = (side(a), side(b));
        if sa >= 0.0 {
            out.push(a);
        }
        if (sa >= 0.0) != (sb >= 0.0) {
            let t = sa / (sa - sb);
            out.push([a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]);
        }
    }
    out
}

/// A segment clipped to a circle, or none.
fn clip_seg(a: [f32; 2], b: [f32; 2], c: [f32; 2], r: f32) -> Option<([f32; 2], [f32; 2])> {
    let d = [b[0] - a[0], b[1] - a[1]];
    let f = [a[0] - c[0], a[1] - c[1]];
    let (qa, qb, qc) =
        (d[0] * d[0] + d[1] * d[1], 2.0 * (f[0] * d[0] + f[1] * d[1]), f[0] * f[0] + f[1] * f[1] - r * r);
    let disc = qb * qb - 4.0 * qa * qc;
    if disc <= 0.0 || qa < 1e-9 {
        return None;
    }
    let s = disc.sqrt();
    let t0 = ((-qb - s) / (2.0 * qa)).max(0.0);
    let t1 = ((-qb + s) / (2.0 * qa)).min(1.0);
    (t1 > t0).then(|| ([a[0] + d[0] * t0, a[1] + d[1] * t0], [a[0] + d[0] * t1, a[1] + d[1] * t1]))
}

/// The navball: the reference plane as a horizon, pitched and rolled with the ship, heading marks along it, the
/// order's horizon as a ghost, the roll ring and the roll in whole degrees (flight-and-navigation 6a).
pub fn navball(cv: &Canvas, cx: &mut Ctx, bx: f32, by: f32, r: f32) {
    let st = vg::style();
    let c = &st.c;
    let v = cx.v;
    let (h, p, roll, _) = to_euler(super::quat(v.q));
    let ppd = r / 60.0;
    let lc = rgba(0xd9e6f2, 1.0);
    // The ball's frame: translate by the pitch, then rotate by minus the roll about the centre.
    let ball = |pitch: f64, rl: f64| {
        Xf::tr(bx, by).rot(-rl as f32).then(Xf::tr(-bx, -by)).then(Xf::tr(0.0, pitch as f32 * ppd))
    };
    let bxf = ball(p, roll);
    let circle: Vec<[f32; 2]> = (0..96)
        .map(|i| {
            let a = i as f32 / 96.0 * std::f32::consts::TAU;
            [bx + r * a.cos(), by + r * a.sin()]
        })
        .collect();
    // Which side of the ball's horizon a screen point is: invert the ball's transform's y.
    let horizon_side = |xf: Xf| {
        let p0 = xf.ap([bx, by]);
        let p1 = xf.ap([bx + 1.0, by]);
        let (dx, dy) = (p1[0] - p0[0], p1[1] - p0[1]);
        move |q: [f32; 2]| -((q[0] - p0[0]) * dy - (q[1] - p0[1]) * dx)
    };
    let side = horizon_side(bxf);
    let sky = clip_half(&circle, |q| -side(q));
    let ground = clip_half(&circle, side);
    cv.poly(&sky, rgba(0x14304a, 1.0));
    cv.poly(&ground, rgba(0x3a2a16, 1.0));
    let seg_in = |a: [f32; 2], b: [f32; 2], pen: Pen| {
        if let Some((a, b)) = clip_seg(a, b, [bx, by], r) {
            cv.seg(a[0], a[1], b[0], b[1], pen);
        }
    };
    let (ha, hb) = (bxf.ap([bx - 3.0 * r, by]), bxf.ap([bx + 3.0 * r, by]));
    seg_in(ha, hb, Pen::new(2.0, lc));
    let inside = |q: [f32; 2]| (q[0] - bx).hypot(q[1] - by) < r - 2.0;
    let mut a = -90;
    while a <= 90 {
        if a != 0 {
            let y = by - a as f32 * ppd;
            let wd = if a % 30 != 0 { 9.0 } else { 18.0 };
            seg_in(bxf.ap([bx - wd, y]), bxf.ap([bx + wd, y]), Pen::new(1.0, alpha(lc, 0.7)));
            if a % 30 == 0 {
                let lab = format!("{}{a}", if a > 0 { "+" } else { "" });
                for (lx, t) in [(bx - wd - 4.0, T::end().ls(0.0)), (bx + wd + 4.0, T::default().ls(0.0))] {
                    let q = bxf.ap([lx, y + 3.0]);
                    if inside(q) {
                        text_rot(cv, lx, y + 3.0, 9.0, lc, &lab, t, &bxf, -roll as f32);
                    }
                }
            }
        }
        a += 15;
    }
    let mut hd = 0;
    while hd < 360 {
        let off = wrap180(hd as f64 - h);
        if off.abs() <= 75.0 {
            let x = bx + off as f32 * ppd;
            seg_in(bxf.ap([x, by]), bxf.ap([x, by - 6.0]), Pen::new(1.5, lc));
            if inside(bxf.ap([x, by - 8.0])) {
                text_rot(cv, x, by - 8.0, 9.0, lc, &format!("{hd:03}"), T::mid().ls(0.0), &bxf, -roll as f32);
            }
        }
        hd += 30;
    }
    let goal = v.goal.map(super::quat);
    if let Some(g) = goal {
        let (_, gp, gr, _) = to_euler(g);
        let gx = ball(gp, gr);
        let (a0, a1) = (gx.ap([bx - 3.0 * r, by]), gx.ap([bx + 3.0 * r, by]));
        seg_in(a0, a1, Pen::new(2.0, accent(v)).dash(6.0, 4.0, 0.0));
    }
    cv.circle(bx, by, r, Color32::TRANSPARENT, Some(Pen::new(2.0, c.line2.0)));
    cv.path(
        &format!(
            "M{} {by}H{}L{} {}L{bx} {by}L{} {}L{} {by}H{}",
            bx - 20.0,
            bx - 7.0,
            bx - 3.0,
            by + 5.0,
            bx + 3.0,
            by + 5.0,
            bx + 7.0,
            bx + 20.0
        ),
        Xf::ID,
        None,
        Some(Pen::new(2.5, c.warn.0).round()),
    );
    let mut a = -170;
    while a <= 180 {
        let th = (a as f64 - roll).to_radians();
        let (sx, sy) = (th.sin() as f32, -th.cos() as f32);
        let big = a % 30 == 0;
        let (r1, r2) = (r + 3.0, r + if big { 10.0 } else { 6.0 });
        cv.seg(
            bx + sx * r1,
            by + sy * r1,
            bx + sx * r2,
            by + sy * r2,
            Pen::new(if big { 1.5 } else { 1.0 }, alpha(lc, if big { 0.9 } else { 0.5 })),
        );
        if big {
            let lab = if a > 0 && a < 180 { format!("+{a}") } else { a.to_string() };
            let col = if a != 0 { rgba(0xa9b6c4, 1.0) } else { lc };
            let t = if a != 0 { T::mid().ls(0.0) } else { T::mid().ls(0.0).bold() };
            cv.text(bx + sx * (r + 19.0), by + sy * (r + 19.0) + 3.0, 9.0, col, &lab, t);
        }
        a += 10;
    }
    cv.path(&format!("M{bx} {}l-5 -9h10z", by - r - 2.0), Xf::ID, Some(c.warn.0), None);
    let rr = super::whole_deg('r', roll) as i64;
    let rx = bx + r + 30.0;
    cv.rect(rx, by - 26.0, 54.0, 52.0, 6.0, c.panel2.0, Some(Pen::new(1.0, c.line2.0)));
    cv.text(rx + 27.0, by - 9.0, 10.0, c.dim.0, "ROLL", T::mid().ls(2.0));
    cv.text(
        rx + 27.0,
        by + 15.0,
        18.0,
        c.text.0,
        &format!("{}{rr}°", if rr > 0 { "+" } else { "" }),
        T::mid().bold().ls(0.0),
    );
    let tip = format!(
        "Heading {}, pitch {}, roll {}. Now {}",
        super::fmt_ang(h, false),
        super::fmt_ang(p, true),
        super::fmt_ang(roll, true),
        super::fmt_q(v.q)
    );
    cx.hits.add(cv, bx - r, by - r, 2.0 * r, 2.0 * r, None, Some(tip));
}

/// Text in a rotated frame: laid out at user (x, y) under `xf` and turned by `deg` (egui turns text about its corner).
#[allow(clippy::too_many_arguments)]
fn text_rot(cv: &Canvas, x: f32, y: f32, size: f32, col: Color32, s: &str, t: T, xf: &Xf, deg: f32) {
    if deg.abs() < 0.05 {
        let q = xf.ap([x, y]);
        cv.text(q[0], q[1], size, col, s, t);
        return;
    }
    cv.text_turned(x, y, size, col, s, t, xf, deg);
}

/// The red alert's vignette: the band's edges washed in the alert colour (a radial gradient from 55% out).
pub fn vignette(cv: &Canvas, w: f32, h: f32, col: Color32, a: f32) {
    let (nx, ny) = (40, 12);
    let mut verts = Vec::new();
    let mut idx = Vec::new();
    for j in 0..=ny {
        for i in 0..=nx {
            let (u, vv) = (i as f32 / nx as f32, j as f32 / ny as f32);
            let t = ((u - 0.5) / 0.75).hypot((vv - 0.5) / 0.75);
            let k = ((t - 0.55) / 0.45).clamp(0.0, 1.0) * a;
            verts.push(([u * w, vv * h], alpha(col, k)));
        }
    }
    for j in 0..ny {
        for i in 0..nx {
            let q = (j * (nx + 1) + i) as u32;
            let n = (nx + 1) as u32;
            idx.extend([q, q + 1, q + n + 1, q, q + n + 1, q + n]);
        }
    }
    cv.mesh(&verts, idx);
}

// ------------------------------------------------------------------------------------------------ shields

/// The faces' names in the hit rule's order, for tips.
const FACE_NAMES: [&str; 6] = ["Bow", "Stern", "Port", "Starboard", "Dorsal", "Ventral"];

/// The shield's bubble cut into patches along the faces' borders (the mockup's `makeBubble`): each patch keeps the face
/// the hit rule gives its centre, and the borders between faces.
pub struct Bubble {
    pub patches: Vec<([DVec3; 4], DVec3, u8)>,
    pub edges: Vec<(DVec3, DVec3)>,
}

pub fn make_bubble(nc: usize, nm: usize, np: usize) -> Bubble {
    let e = tern().shield;
    let ax = e.axes;
    let tc = sc_core::combat::SHIELD_CAP_COS.acos();
    let nt = 2 * nc + nm;
    let pi = std::f64::consts::PI;
    let theta = |i: usize| {
        if i <= nc {
            i as f64 / nc as f64 * tc
        } else if i <= nc + nm {
            tc + (i - nc) as f64 / nm as f64 * (pi - 2.0 * tc)
        } else {
            pi - tc + (i - nc - nm) as f64 / nc as f64 * tc
        }
    };
    let at3 = |th: f64, ph: f64| DVec3::new(ax.x * th.sin() * ph.cos(), ax.y * th.sin() * ph.sin(), ax.z * th.cos());
    let pt = |i: usize, j: usize| at3(theta(i), j as f64 / np as f64 * std::f64::consts::TAU);
    let mut patches = Vec::new();
    for i in 0..nt {
        for j in 0..np {
            let v = [pt(i, j), pt(i + 1, j), pt(i + 1, j + 1), pt(i, j + 1)];
            let th = (theta(i) + theta(i + 1)) / 2.0;
            let ph = (j as f64 + 0.5) / np as f64 * std::f64::consts::TAU;
            let mid = at3(th, ph);
            patches.push((v, mid, sc_core::combat::shield_face(mid, ax)));
        }
    }
    let at = |i: usize, j: usize| &patches[i * np + j % np];
    let mut edges = Vec::new();
    for i in 0..nt {
        for j in 0..np {
            let p = at(i, j);
            if i + 1 < nt && at(i + 1, j).2 != p.2 {
                edges.push((p.0[1], p.0[2]));
            }
            if at(i, j + 1).2 != p.2 {
                edges.push((p.0[2], p.0[3]));
            }
        }
    }
    Bubble { patches, edges }
}

fn mini_bubble() -> &'static Bubble {
    static B: OnceLock<Bubble> = OnceLock::new();
    B.get_or_init(|| make_bubble(2, 4, 16))
}

fn big_bubble() -> &'static Bubble {
    static B: OnceLock<Bubble> = OnceLock::new();
    B.get_or_init(|| make_bubble(4, 8, 32))
}

/// The point of the shield ellipsoid in direction `d` from its centre.
pub fn ellipsoid_point(d: DVec3) -> DVec3 {
    let a = tern().shield.axes;
    d / (d.x / a.x).hypot(d.y / a.y).hypot(d.z / a.z)
}

/// The ellipsoid's outward normal at its point `o`.
fn ellipsoid_normal(o: DVec3) -> DVec3 {
    let a = tern().shield.axes;
    DVec3::new(o.x / (a.x * a.x), o.y / (a.y * a.y), o.z / (a.z * a.z)).normalize()
}

/// Hits closer together than this, in degrees of bearing, are drawn as one arrow (the newest): the mockup's
/// `HIT_MERGE_DEG`.
pub const HIT_MERGE_DEG: f64 = 10.0;

/// The hits drawn, oldest first: one a bearing, a hit within [`HIT_MERGE_DEG`] of a newer one left out, so a fight's
/// dozen hits from one side read as the one arrow they are, in the newest hit's colour and with its label (the
/// mockup's `shown`).
fn shown_hits(hits: &[super::HitView]) -> Vec<&super::HitView> {
    let merge = HIT_MERGE_DEG.to_radians().cos();
    let mut shown: Vec<&super::HitView> = Vec::new();
    for h in hits.iter().rev() {
        let d = v3(h.d);
        if !shown.iter().any(|q| v3(q.d).dot(d) > merge) {
            shown.push(h);
        }
    }
    shown.reverse();
    shown
}

/// The part of the segment `a`-`b` inside `[0, w] x [0, h]` (Liang and Barsky), with how far along it from `a` the
/// kept part starts, so a dash pattern keeps its phase.
fn clip_to_rect(a: [f32; 2], b: [f32; 2], [w, h]: [f32; 2]) -> Option<([f32; 2], [f32; 2], f32)> {
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let (mut t0, mut t1) = (0.0_f32, 1.0_f32);
    for (p, q) in [(-dx, a[0]), (dx, w - a[0]), (-dy, a[1]), (dy, h - a[1])] {
        if p == 0.0 {
            if q < 0.0 {
                return None;
            }
        } else {
            let r = q / p;
            if p < 0.0 {
                t0 = t0.max(r);
            } else {
                t1 = t1.min(r);
            }
        }
    }
    (t0 < t1).then(|| {
        let at = |t: f32| [a[0] + dx * t, a[1] + dy * t];
        (at(t0), at(t1), t0 * dx.hypot(dy))
    })
}

/// The ship inside its shield, in 3D (the mockup's `shieldView`): hull panels lit and depth sorted between the
/// bubble's far and near halves, each patch filled by its face's charge, the faces' borders, hostiles' directions,
/// inbound missiles and the last 8 s of hits, one arrow a bearing ([`HIT_MERGE_DEG`]). `big` (Science's, given the
/// view's width and height) uses the finer bubble, names every face with its charge, marks the hits' angles and lets a
/// face be pressed to favour it. The mockup draws the big view in an `<svg>` clipped to that size; here what reaches
/// past it is cut to it by hand, since a clip rectangle is a UI draw call.
#[allow(clippy::too_many_arguments)]
pub fn shield_view(cv: &Canvas, cx: &mut Ctx, ox: f64, oy: f64, sc: f64, yaw: f64, el: f64, big: Option<[f32; 2]>) {
    let v = cx.v;
    let c = &vg::style().c;
    let b = cam_basis(yaw, el);
    let pr = |p: DVec3| [(ox + p.dot(b.r) * sc) as f32, (oy - p.dot(b.u) * sc) as f32];
    let bub = if big.is_some() { big_bubble() } else { mini_bubble() };
    let light = (-b.f + DVec3::new(0.3, 0.8, 0.2)).normalize();
    let hull = &tern().hull;
    let mut hull_polys: Vec<(f64, Vec<[f32; 2]>, Color32, bool)> = Vec::new();
    for s in 0..hull.len() - 1 {
        for k in 0..8 {
            let (a0, b0) = (hull[s][k], hull[s][(k + 1) % 8]);
            let (c0, d0) = (hull[s + 1][(k + 1) % 8], hull[s + 1][k]);
            let mut n = (b0 - a0).cross(d0 - a0).normalize_or_zero();
            let mid = (a0 + b0 + c0 + d0) * 0.25;
            if n.dot(DVec3::new(mid.x, mid.y - (hull[s][0].y + hull[s][4].y) / 2.0, 0.0)) < 0.0 {
                n = -n;
            }
            if n.dot(b.f) > 0.0 {
                continue;
            }
            let lum = 0.35 + 0.65 * n.dot(light).max(0.0);
            let col = Color32::from_rgb(
                (70.0 * lum + 20.0).round() as u8,
                (84.0 * lum + 22.0).round() as u8,
                (102.0 * lum + 26.0).round() as u8,
            );
            hull_polys.push((mid.dot(b.f), [a0, b0, c0, d0].iter().map(|q| pr(*q)).collect(), col, true));
        }
    }
    for end in [&hull[0], &hull[hull.len() - 1]] {
        let mid = end.iter().fold(DVec3::ZERO, |a, p| a + *p) / 8.0;
        hull_polys.push((mid.dot(b.f), end.iter().map(|q| pr(*q)).collect(), rgba(0x3a4656, 1.0), true));
    }
    hull_polys.sort_by(|a, b| b.0.total_cmp(&a.0));
    let shell: Vec<(f64, Vec<[f32; 2]>, u8, bool)> = bub
        .patches
        .iter()
        .map(|(vs, mid, face)| {
            (mid.dot(b.f), vs.iter().map(|q| pr(*q)).collect(), *face, ellipsoid_normal(*mid).dot(b.f) < 0.0)
        })
        .collect();
    let mut far: Vec<&(f64, Vec<[f32; 2]>, u8, bool)> = shell.iter().filter(|p| !p.3).collect();
    let mut near: Vec<&(f64, Vec<[f32; 2]>, u8, bool)> = shell.iter().filter(|p| p.3).collect();
    far.sort_by(|a, b| b.0.total_cmp(&a.0));
    near.sort_by(|a, b| b.0.total_cmp(&a.0));
    let patch = |p: &(f64, Vec<[f32; 2]>, u8, bool)| {
        let f = usize::from(p.2);
        let k = if v.cap[f] > 1e-6 { v.charge[f] / v.cap[f] } else { 0.0 };
        let fl = (1.0 - v.flash_age_s[f] / 0.4).max(0.0);
        let op = if p.3 { 0.07 + 0.2 * k } else { 0.04 + 0.1 * k } + 0.5 * fl;
        cv.poly(&p.1, alpha(ratio_col(k), op as f32));
    };
    let ec = rgba(0xcfe3f5, 1.0);
    for p in &far {
        patch(p);
    }
    let mut near_edges = Vec::new();
    for (a, e) in &bub.edges {
        let front = ellipsoid_normal((*a + *e) * 0.5).dot(b.f) < 0.0;
        let (pa, pb) = (pr(*a), pr(*e));
        if front {
            near_edges.push((pa, pb));
        } else {
            cv.seg(pa[0], pa[1], pb[0], pb[1], Pen::new(0.6, alpha(ec, 0.25)));
        }
    }
    for (_, pts, col, _) in &hull_polys {
        cv.poly(pts, *col);
        let mut closed = pts.clone();
        closed.push(pts[0]);
        cv.polyline(&closed, Pen::new(0.6, rgba(0x0d141d, 1.0)));
    }
    for p in &near {
        patch(p);
    }
    if big.is_some() {
        // A press on a face favours it; the patches' boxes, nearest last, stand in for the polygons.
        for p in far.iter().chain(near.iter()) {
            let f = usize::from(p.2);
            let (mut x0, mut y0, mut x1, mut y1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
            for q in &p.1 {
                (x0, y0, x1, y1) = (x0.min(q[0]), y0.min(q[1]), x1.max(q[0]), y1.max(q[1]));
            }
            let tip = format!("{}: {:.0} of {:.0} MJ", FACE_NAMES[f], v.charge[f], v.cap[f]);
            cx.hits.add(cv, x0, y0, x1 - x0, y1 - y0, Some(Hit::Act(Act::Favour(f as i32))), Some(tip));
        }
    }
    for (pa, pb) in near_edges {
        cv.seg(pa[0], pa[1], pb[0], pb[1], Pen::new(1.2, alpha(ec, 0.55)));
    }
    let shown = shown_hits(&v.hits);
    // A hit's label, held inside the view as the faces' labels are (the mockup's `hitLabel`).
    let hit_label = |p: [f32; 2], [w, h]: [f32; 2]| [p[0].clamp(30.0, w - 30.0), (p[1] - 6.0).clamp(16.0, h - 4.0)];
    // What the mockup's clip keeps: a segment cut to the view, a mark only while it is inside it.
    let inside = |p: [f32; 2]| big.is_none_or(|[w, h]| (0.0..=w).contains(&p[0]) && (0.0..=h).contains(&p[1]));
    let cut = |a: [f32; 2], b: [f32; 2]| match big {
        Some(wh) => clip_to_rect(a, b, wh),
        None => Some((a, b, 0.0)),
    };
    // Labels already down, and where a new one goes: up 16 lp at a time, at most 8 times, while it sits on one (the
    // mockup's `placed` and `place`).
    let mut placed: Vec<[f32; 2]> = Vec::new();
    let place = |placed: &mut Vec<[f32; 2]>, x: f32, mut y: f32| {
        for _ in 0..8 {
            if !placed.iter().any(|q| (q[0] - x).abs() < 52.0 && (q[1] - y).abs() < 15.0) {
                break;
            }
            y -= 16.0;
        }
        placed.push([x, y]);
        y
    };
    if let Some([pw, _]) = big {
        // Every face named on the model with its charge under the name; a face on the far side darker.
        let axes = tern().shield.axes;
        let dirs = [DVec3::Z, DVec3::NEG_Z, DVec3::X, DVec3::NEG_X, DVec3::Y, DVec3::NEG_Y];
        let names = ["BOW", "STERN", "PORT", "STBD", "TOP", "BELOW"];
        for (f, d) in dirs.iter().enumerate() {
            let ax = [axes.z, axes.z, axes.x, axes.x, axes.y, axes.y][f];
            let far_side = d.dot(b.f) > 0.05;
            let k = if v.cap[f] > 1e-6 { v.charge[f] / v.cap[f] } else { 0.0 };
            let (c0, p0) = (pr(DVec3::ZERO), pr(*d * ax));
            let (sx, sy) = (p0[0] - c0[0], p0[1] - c0[1]);
            let sl = sx.hypot(sy);
            let mut p = if sl > 4.0 { [p0[0] + sx / sl * 14.0, p0[1] + sy / sl * 14.0] } else { p0 };
            p[0] = p[0].clamp(30.0, pw - 30.0);
            let ly = p[1] + 4.0;
            placed.push([p[0], ly]);
            let op = if far_side { 0.42 } else { 1.0 };
            let col = alpha(ratio_col(k), op);
            cv.text(p[0], ly, 12.0, col, names[f], T::mid().bold().ls(1.5));
            cv.rect(p[0] - 16.0, ly + 4.0, 32.0, 4.0, 2.0, alpha(rgba(0x16202c, 1.0), op), None);
            cv.rect(p[0] - 16.0, ly + 4.0, 32.0 * k.clamp(0.0, 1.0) as f32, 4.0, 2.0, col, None);
            let tip = format!(
                "{}: {:.0} of {:.0} MJ{}",
                FACE_NAMES[f],
                v.charge[f],
                v.cap[f],
                if far_side { " (far side)" } else { "" }
            );
            cx.hits.add(cv, p[0] - 24.0, ly - 12.0, 48.0, 22.0, None, Some(tip));
        }
    }
    let bigf = big.is_some();
    for k in &v.contacts {
        if k.hull <= 0.0 || k.iff != "hostile" || range_km(k.rel) > v.sci_km {
            continue;
        }
        let d = v3(k.rel).normalize_or_zero();
        let e = ellipsoid_point(d);
        let (a, bb) = (pr(e), pr(e + d * if bigf { 48.0 } else { 34.0 }));
        if let Some((p0, p1, off)) = cut(bb, a) {
            cv.seg(p0[0], p0[1], p1[0], p1[1], Pen::new(1.5, alpha(c.hostile.0, 0.8)).dash(4.0, 4.0, off));
        }
        if !inside(bb) {
            continue;
        }
        cv.circle(bb[0], bb[1], if bigf { 4.0 } else { 2.5 }, c.hostile.0, None);
        let tip = format!(
            "{}: {:03} {}{}",
            k.id,
            js_round(wrap360(brg_of(d))).rem_euclid(360),
            if elev_of(d) >= 0.0 { "+" } else { "" },
            js_round(elev_of(d))
        );
        cx.hits.add(cv, bb[0] - 4.0, bb[1] - 4.0, 8.0, 8.0, None, Some(tip));
    }
    for m in &v.inbound {
        let d = v3(m.d);
        let e = ellipsoid_point(d);
        let (q, q2) = (pr(e + d * 20.0 * m.t), pr(e + d * (20.0 * m.t - 4.0)));
        let ang = (q2[0] - q[0]).atan2(-(q2[1] - q[1])).to_degrees();
        let arrow = if bigf { "M0 -8L6 6L0 2L-6 6Z" } else { "M0 -5L4 4L0 1L-4 4Z" };
        if inside(q) {
            cv.path(arrow, Xf::tr(q[0], q[1]).rot(ang), Some(c.danger.0), None);
        }
    }
    for hit in shown {
        let age = (hit.age_s / 8.0) as f32;
        let d = v3(hit.d);
        let e = ellipsoid_point(d);
        let (a, bb) = (pr(e), pr(e + d * if bigf { 38.0 } else { 26.0 }));
        let col = alpha(if hit.through { c.danger.0 } else { c.warn.0 }, 1.0 - age);
        let ang = (a[0] - bb[0]).atan2(-(a[1] - bb[1])).to_degrees();
        if let Some((p0, p1, _)) = cut(bb, a) {
            cv.seg(p0[0], p0[1], p1[0], p1[1], Pen::new(if bigf { 3.0 } else { 2.0 }, col));
        }
        let head = if bigf { "M0 0L-6 -10H6Z" } else { "M0 0L-4 -7H4Z" };
        cv.path(head, Xf::tr(a[0], a[1]).rot(ang), Some(col), None);
        if let Some(wh) = big {
            let el = js_round(elev_of(d));
            let t =
                format!("{:03} {}{}", js_round(wrap360(brg_of(d))).rem_euclid(360), if el >= 0 { "+" } else { "" }, el);
            // Pinned face labels stay; a hit's label steps up clear of them and of the hits' before it.
            let [lx, ly0] = hit_label(bb, wh);
            let ly = place(&mut placed, lx, ly0).max(16.0);
            cv.text(lx, ly, 14.0, col, &t, T::mid().bold());
        }
    }
}

// ------------------------------------------------------------------------------------------------ helm's controls

/// A joystick in perspective (the mockup's `joystick`): base and boot fixed, shaft and grip leaning as pushed, the
/// four directions on the base, and the way the nose is moving.
pub fn joystick(cv: &Canvas, cx: &mut Ctx, x: f32, h: f32, yaw: f64, pitch: f64) {
    let v = cx.v;
    let c = &vg::style().c;
    let t = super::axis(DVec3::Z, -yaw * 22.0) * super::axis(DVec3::X, pitch * 22.0);
    let key = format!("joy|{yaw:.3}|{pitch:.3}|{x}|{h}");
    let (xd, hd) = (x as f64, h as f64);
    let drawn = memo(key, || {
        let m = models();
        let mut parts = m.base.clone();
        parts.extend(m.boot.iter().cloned());
        parts.extend(turn_mesh(&m.stick, t, DVec3::ZERO));
        project_mesh(&parts, 180.0, 80.0, xd, hd / 2.0 + 6.0, 60.0)
    });
    draw_projected(cv, &drawn);
    let (bcx, bcy) = (x, h / 2.0 + 6.0);
    let rr = 47.0;
    let dom = if yaw.abs().max(pitch.abs()) < 0.15 {
        None
    } else if yaw.abs() >= pitch.abs() {
        Some(if yaw > 0.0 { 'r' } else { 'l' })
    } else {
        Some(if pitch > 0.0 { 'd' } else { 'u' })
    };
    let acc = accent(v);
    for (k, dx, dy) in [('u', 0.0f32, -1.0f32), ('d', 0.0, 1.0), ('l', -1.0, 0.0), ('r', 1.0, 0.0)] {
        let on = dom == Some(k);
        let (ax, ay) = (bcx + dx * rr, bcy + dy * rr * 0.985);
        let ang = dx.atan2(-dy).to_degrees();
        let fill = if on { acc } else { alpha(rgba(0x7d8a99, 1.0), 0.55) };
        let pen = on.then(|| Pen::new(0.8, Color32::WHITE));
        cv.path("M0 -7L7 3H2V7H-2V3H-7Z", Xf::tr(ax, ay).rot(ang), Some(fill), pen);
    }
    let ry = -v.w[1] / v.flight.rate_dps[1].to_radians();
    let rp = -v.w[0] / v.flight.rate_dps[0].to_radians();
    let mag = ry.hypot(rp).min(1.0);
    if mag > 0.03 {
        let (ux, uy) = (ry / ry.hypot(rp), rp / ry.hypot(rp));
        let l = 14.0 + 26.0 * mag;
        let (ex, ey) = (bcx + (ux * l) as f32, bcy + (uy * l) as f32);
        cv.seg(bcx, bcy, ex, ey, Pen::new(3.0, c.warn.0).round());
        cv.path("M0 -6L5 3H-5Z", Xf::tr(ex, ey).rot(ux.atan2(-uy).to_degrees() as f32), Some(c.warn.0), None);
    }
    cx.hits.add(
        cv,
        x - 74.0,
        0.0,
        148.0,
        h,
        Some(Hit::Drag(Drag::Joy)),
        Some("Stick: yaw and pitch (A, D, R, F). Pull back for nose up".into()),
    );
}

/// A spring-return flipper switch for roll (the mockup's `flipper`).
pub fn flipper(cv: &Canvas, cx: &mut Ctx, x: f32, h: f32, roll: f64) {
    let c = &vg::style().c;
    let acc = accent(cx.v);
    let y0 = h - 140.0;
    let lc = if roll < -0.05 { acc } else { c.dim.0 };
    let rc = if roll > 0.05 { acc } else { c.dim.0 };
    cv.path(
        &format!("M{} {y0}Q{} {} {} {}", x - 6.0, x - 24.0, y0 - 4.0, x - 30.0, y0 + 10.0),
        Xf::ID,
        None,
        Some(Pen::new(2.5, lc)),
    );
    cv.path(&format!("M{} {}l5 9 5 -7z", x - 35.0, y0 + 6.0), Xf::ID, Some(lc), None);
    cv.path(
        &format!("M{} {y0}Q{} {} {} {}", x + 6.0, x + 24.0, y0 - 4.0, x + 30.0, y0 + 10.0),
        Xf::ID,
        None,
        Some(Pen::new(2.5, rc)),
    );
    cv.path(&format!("M{} {}l-5 9 -5 -7z", x + 35.0, y0 + 6.0), Xf::ID, Some(rc), None);
    let key = format!("flip|{roll:.3}|{x}|{h}");
    let (xd, hd) = (x as f64, h as f64);
    let drawn = memo(key, || {
        let m = models();
        let mut parts = m.housing.clone();
        parts.extend(m.boss.iter().cloned());
        parts.extend(turn_mesh(&m.bat, super::axis(DVec3::Z, -roll * 30.0), DVec3::new(0.0, 0.1, 0.0)));
        project_mesh(&parts, 180.0, 26.0, xd, hd - 46.0, 54.0)
    });
    draw_projected(cv, &drawn);
    cx.hits.add(
        cv,
        x - 40.0,
        0.0,
        80.0,
        h,
        Some(Hit::Drag(Drag::Flip)),
        Some("Flipper: roll left or right (Q, E)".into()),
    );
}

/// A thumbwheel: a ridged drum in its housing that rolls a whole degree a notch (the mockup's `thumbwheel`).
#[allow(clippy::too_many_arguments)]
pub fn thumbwheel(cv: &Canvas, cx: &mut Ctx, x: f32, top: f32, hh: f32, value: f64, k: char, word: &str) {
    let c = &vg::style().c;
    let cy = top + hh / 2.0;
    let r = hh / 2.0 - 8.0;
    let phase = (value * 15.0).rem_euclid(15.0);
    cv.rect(x - 26.0, top, 52.0, hh, 7.0, rgba(0x10161e, 1.0), Some(Pen::new(1.0, c.line2.0)));
    let (d0, d1) = (rgba(0x07090c, 1.0), rgba(0x3b4654, 1.0));
    let (dt, db) = (top + 6.0, top + hh - 6.0);
    cv.vgrad(x - 17.0, dt, 34.0, hh - 12.0, &[(0.0, d0), (0.5, d1), (1.0, d0)]);
    // The ridges, cut to the drum by hand: a clip rectangle would cost a UI draw call per wheel.
    let band = |y0: f32, h: f32, col: Color32| {
        let (a, b) = (y0.max(dt), (y0 + h).min(db));
        if b > a {
            cv.rect(x - 17.0, a, 34.0, b - a, 0.0, col, None);
        }
    };
    let mut a = -90.0;
    while a <= 90.0 {
        let ang = a + phase;
        if (-89.0..=89.0).contains(&ang) {
            let y = cy - r * (ang as f32).to_radians().sin();
            let t = (ang as f32).to_radians().cos();
            band(y - 1.6 * t, 3.2 * t + 0.4, rgba(0x0a0d11, 1.0));
            band(y - 1.6 * t - 1.0, 1.0, alpha(rgba(0x8a96a4, 1.0), 0.5 * t));
        }
        a += 15.0;
    }
    let d = format!("M{} {}l3 5h-6z M{} {}l3 -5h-6z", x + 21.0, top + 12.0, x + 21.0, top + hh - 12.0);
    cv.path(&d, Xf::ID, Some(c.dim.0), None);
    cx.hits.add(
        cv,
        x - 26.0,
        top,
        52.0,
        hh,
        Some(Hit::Drag(Drag::Wheel(k))),
        Some(format!("{}: drag, scroll or tap, 1° a notch", word.to_lowercase())),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hit(brg_deg: f64, age_s: f64) -> super::super::HitView {
        let r = brg_deg.to_radians();
        super::super::HitView { d: [r.sin(), 0.0, r.cos()], face: 0, age_s, through: false }
    }

    #[test]
    fn a_volley_from_one_bearing_is_drawn_as_its_newest_hit() {
        // The five-seat drill landed about 13 hits in 8 s from the Hound's side, and their labels stacked into a smear
        // past the panel's edge (2026-10-10).
        let hits = [hit(10.0, 7.0), hit(14.0, 5.0), hit(90.0, 4.0), hit(12.0, 1.0), hit(17.0, 0.5)];
        let shown = shown_hits(&hits);
        let ages: Vec<f64> = shown.iter().map(|h| h.age_s).collect();
        assert_eq!(ages, vec![4.0, 0.5], "one arrow for the bow volley, the newest, and one for the beam hit");
    }

    #[test]
    fn a_segment_is_cut_to_the_view_and_keeps_its_dash_phase() {
        let (a, b, off) = clip_to_rect([-20.0, 50.0], [80.0, 50.0], [400.0, 300.0]).expect("crosses the view");
        assert_eq!((a, b), ([0.0, 50.0], [80.0, 50.0]));
        assert!((off - 20.0).abs() < 1e-4, "the dash starts where the mockup's clipped one would, 20 lp in");
        assert!(clip_to_rect([-20.0, 50.0], [-5.0, 60.0], [400.0, 300.0]).is_none(), "wholly outside: nothing drawn");
        let (a, b, off) = clip_to_rect([10.0, 10.0], [20.0, 20.0], [400.0, 300.0]).expect("inside");
        assert_eq!((a, b, off), ([10.0, 10.0], [20.0, 20.0], 0.0));
    }
}
