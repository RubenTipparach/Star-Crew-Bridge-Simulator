//! The consoles' vector drawing: the console mockup's SVG primitives, drawn with egui shapes.
//!
//! The approved console mockup (`docs/mockups/consoles.html`) is the consoles' specification (CLAUDE.md 10): every
//! panel is SVG drawn at 1280 x 720 layout points with a handful of primitives (`rect`, `circle`, `ellipse`, `path`,
//! `text`, an icon set on a 24-unit grid, the Barlow Semi Condensed face). This module is those primitives for egui,
//! so a console here is written as the mockup's drawing code is, call for call, and draws the same picture
//! (openspec/changes/console-parity, design 3). It lives in the client because egui is the client's UI; the colours
//! and icons are data (`data/ui/console_style.json`), read out of the mockup by `tools/ui/console_style.py`.
//!
//! A [`Canvas`] is a painter with the SVG's user space: an origin and a scale from layout points to egui points.
//! Shapes keep SVG's meaning: a stroke is centred on the edge, a path is flattened from its own commands (lines,
//! cubic and quadratic curves, elliptical arcs), a fill may be any simple polygon, text sits on its alphabetic
//! baseline with SVG's anchors and letter spacing, and digits are tabular (baked into the font by `tools/ui/fonts.py`).

use egui::epaint::{Mesh, PathShape, PathStroke, RectShape, Vertex, WHITE_UV};
use egui::{Color32, CornerRadius, FontFamily, FontId, Painter, Pos2, Rect, Shape, Stroke, StrokeKind, Vec2};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::OnceLock;

/// The two faces the consoles use, as egui font families.
pub const SEMIBOLD: &str = "barlow-600";
/// Bold (the mockup's `font-weight 700`).
pub const BOLD: &str = "barlow-700";

/// Give egui the consoles' typeface, Barlow Semi Condensed 600 and 700, with egui's own fonts behind it for any
/// glyph Barlow lacks (as the mockup's font stack falls back to the system's).
pub fn install_fonts(ctx: &egui::Context) {
    let mut defs = egui::FontDefinitions::default();
    for (name, bytes) in [
        (SEMIBOLD, include_bytes!("../../../assets/fonts/barlow-semi-condensed/semibold-600.ttf").as_slice()),
        (BOLD, include_bytes!("../../../assets/fonts/barlow-semi-condensed/bold-700.ttf").as_slice()),
    ] {
        defs.font_data.insert(name.to_owned(), egui::FontData::from_static(bytes).into());
        let mut chain = vec![name.to_owned()];
        chain.extend(defs.families.get(&FontFamily::Proportional).cloned().unwrap_or_default());
        defs.families.insert(FontFamily::Name(name.into()), chain);
    }
    ctx.set_fonts(defs);
}

/// The mockup's colour roles (`const C` in consoles.html).
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Palette {
    pub bg: Hex,
    pub panel: Hex,
    pub panel2: Hex,
    pub line: Hex,
    pub line2: Hex,
    pub text: Hex,
    pub dim: Hex,
    pub faint: Hex,
    pub ok: Hex,
    pub warn: Hex,
    pub danger: Hex,
    pub alert: Hex,
    pub emergency: Hex,
    pub hostile: Hex,
    pub neutral: Hex,
    pub friendly: Hex,
    pub unknown: Hex,
}

/// The stations' colours (`PALETTE.role` in shipkit.js).
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Roles {
    pub command: Hex,
    pub helm: Hex,
    pub tactical: Hex,
    pub engineering: Hex,
    pub science: Hex,
    pub comms: Hex,
    pub flight_ops: Hex,
    pub gunner: Hex,
}

/// A colour written `#rrggbb` in the data, opaque.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Hex(pub Color32);

impl<'de> Deserialize<'de> for Hex {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        parse_hex(&s).map(Hex).ok_or_else(|| serde::de::Error::custom(format!("not a #rrggbb colour: {s}")))
    }
}

fn parse_hex(s: &str) -> Option<Color32> {
    let h = s.strip_prefix('#')?;
    if h.len() != 6 {
        return None;
    }
    let v = u32::from_str_radix(h, 16).ok()?;
    Some(Color32::from_rgb((v >> 16) as u8, (v >> 8) as u8, v as u8))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StyleFile {
    #[allow(dead_code)]
    schema: String,
    #[serde(rename = "_doc")]
    #[allow(dead_code)]
    doc: String,
    colours: Palette,
    roles: Roles,
    icons: HashMap<String, String>,
    order_verbs: std::collections::BTreeMap<String, Vec<[String; 2]>>,
}

/// One stroked piece of an icon, flattened in its 24-unit grid.
#[derive(Clone, Debug)]
struct IconPart {
    subpaths: Vec<SubPath>,
    dash: Option<[f32; 2]>,
    width: Option<f32>,
}

/// The consoles' look: colours, role colours and the flattened icons.
pub struct Style {
    pub c: Palette,
    pub role: Roles,
    icons: HashMap<String, Vec<IconPart>>,
    /// The captain's orders to each station: [verb, icon], in the mockup's order.
    pub order_verbs: std::collections::BTreeMap<String, Vec<[String; 2]>>,
}

/// The style, parsed once from the compiled-in `data/ui/console_style.json`.
pub fn style() -> &'static Style {
    static STYLE: OnceLock<Style> = OnceLock::new();
    STYLE.get_or_init(|| {
        let f: StyleFile = serde_json::from_str(include_str!("../../../data/ui/console_style.json"))
            .expect("data/ui/console_style.json is valid (tools/ui/console_style.py writes it)");
        let icons = f.icons.iter().map(|(k, v)| (k.clone(), parse_icon(v))).collect();
        Style { c: f.colours, role: f.roles, icons, order_verbs: f.order_verbs }
    })
}

impl Roles {
    /// A role's colour by its id in the layout's station data.
    pub fn of(&self, id: &str) -> Color32 {
        match id {
            "command" => self.command.0,
            "helm" => self.helm.0,
            "tactical" => self.tactical.0,
            "engineering" => self.engineering.0,
            "science" => self.science.0,
            "comms" => self.comms.0,
            "flight_ops" => self.flight_ops.0,
            _ => self.gunner.0,
        }
    }
}

/// A colour at an opacity, as SVG's `opacity` and `fill-opacity` blend it (egui's colours are premultiplied).
pub fn alpha(c: Color32, a: f32) -> Color32 {
    let a = a.clamp(0.0, 1.0);
    let [r, g, b, a0] = c.to_srgba_unmultiplied();
    Color32::from_rgba_unmultiplied(r, g, b, (f32::from(a0) * a).round() as u8)
}

/// A colour from its `#rrggbb` and an opacity: the mockup's `"#rrggbb" + "1f"` suffixes and literal colours.
pub fn rgba(rgb: u32, a: f32) -> Color32 {
    alpha(Color32::from_rgb((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8), a)
}

// ------------------------------------------------------------------------------------------------ paths

/// A flattened subpath: points in user units, closed or open.
#[derive(Clone, Debug, Default)]
pub struct SubPath {
    pub pts: Vec<[f32; 2]>,
    pub closed: bool,
}

/// A 2D affine transform, SVG's `matrix(a b c d e f)`: x' = a x + c y + e, y' = b x + d y + f.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Xf([f32; 6]);

impl Default for Xf {
    fn default() -> Self {
        Self::ID
    }
}

impl Xf {
    /// The identity.
    pub const ID: Xf = Xf([1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);
    /// `translate(x, y)`.
    pub fn tr(x: f32, y: f32) -> Self {
        Xf([1.0, 0.0, 0.0, 1.0, x, y])
    }
    /// This, then a rotation by `deg` (clockwise on screen, as SVG's `rotate`).
    pub fn rot(self, deg: f32) -> Self {
        let (s, c) = deg.to_radians().sin_cos();
        self.then(Xf([c, s, -s, c, 0.0, 0.0]))
    }
    /// This, then a scale.
    pub fn sc(self, k: f32) -> Self {
        self.then(Xf([k, 0.0, 0.0, k, 0.0, 0.0]))
    }
    /// SVG's order: `self` is the outer transform, `inner` is applied to the point first.
    pub fn then(self, inner: Xf) -> Self {
        let [a, b, c, d, e, f] = self.0;
        let [a2, b2, c2, d2, e2, f2] = inner.0;
        Xf([
            a * a2 + c * b2,
            b * a2 + d * b2,
            a * c2 + c * d2,
            b * c2 + d * d2,
            a * e2 + c * f2 + e,
            b * e2 + d * f2 + f,
        ])
    }
    /// A point through the transform.
    pub fn ap(&self, p: [f32; 2]) -> [f32; 2] {
        let [a, b, c, d, e, f] = self.0;
        [a * p[0] + c * p[1] + e, b * p[0] + d * p[1] + f]
    }
    /// How much the transform scales a length (the larger axis).
    pub fn scale(&self) -> f32 {
        let [a, b, c, d, _, _] = self.0;
        a.hypot(b).max(c.hypot(d))
    }
}

struct Tokens<'a> {
    s: &'a [u8],
    i: usize,
}

impl Tokens<'_> {
    fn skip(&mut self) {
        while self.i < self.s.len() && (self.s[self.i].is_ascii_whitespace() || self.s[self.i] == b',') {
            self.i += 1;
        }
    }
    fn cmd(&mut self) -> Option<u8> {
        self.skip();
        let c = *self.s.get(self.i)?;
        if c.is_ascii_alphabetic() {
            self.i += 1;
            Some(c)
        } else {
            None
        }
    }
    fn at_number(&mut self) -> bool {
        self.skip();
        self.s.get(self.i).is_some_and(|c| c.is_ascii_digit() || matches!(c, b'-' | b'+' | b'.'))
    }
    fn num(&mut self) -> f32 {
        self.skip();
        let st = self.i;
        let mut seen_dot = false;
        let mut seen_e = false;
        if matches!(self.s.get(self.i), Some(b'-' | b'+')) {
            self.i += 1;
        }
        while let Some(&c) = self.s.get(self.i) {
            if c.is_ascii_digit() {
                self.i += 1;
            } else if c == b'.' && !seen_dot && !seen_e {
                seen_dot = true;
                self.i += 1;
            } else if (c == b'e' || c == b'E') && !seen_e {
                seen_e = true;
                self.i += 1;
                if matches!(self.s.get(self.i), Some(b'-' | b'+')) {
                    self.i += 1;
                }
            } else {
                break;
            }
        }
        std::str::from_utf8(&self.s[st..self.i]).ok().and_then(|t| t.parse().ok()).unwrap_or(0.0)
    }
    fn flag(&mut self) -> bool {
        self.skip();
        let c = self.s.get(self.i).copied();
        self.i += 1;
        c == Some(b'1')
    }
}

/// Points along an elliptical arc in SVG's endpoint form (SVG 1.1 appendix F.6.5), excluding the start.
#[allow(clippy::too_many_arguments)]
fn arc_points(
    p0: [f32; 2],
    rx: f32,
    ry: f32,
    rot_deg: f32,
    large: bool,
    sweep: bool,
    p1: [f32; 2],
    seg: f32,
) -> Vec<[f32; 2]> {
    let (mut rx, mut ry) = (rx.abs(), ry.abs());
    if rx < 1e-6 || ry < 1e-6 || (p0[0] - p1[0]).hypot(p0[1] - p1[1]) < 1e-6 {
        return vec![p1];
    }
    let (sp, cp) = rot_deg.to_radians().sin_cos();
    let dx = (p0[0] - p1[0]) / 2.0;
    let dy = (p0[1] - p1[1]) / 2.0;
    let x1 = cp * dx + sp * dy;
    let y1 = -sp * dx + cp * dy;
    let lam = (x1 * x1) / (rx * rx) + (y1 * y1) / (ry * ry);
    if lam > 1.0 {
        rx *= lam.sqrt();
        ry *= lam.sqrt();
    }
    let num = (rx * rx * ry * ry - rx * rx * y1 * y1 - ry * ry * x1 * x1).max(0.0);
    let den = rx * rx * y1 * y1 + ry * ry * x1 * x1;
    let mut k = (num / den.max(1e-12)).sqrt();
    if large == sweep {
        k = -k;
    }
    let cx1 = k * rx * y1 / ry;
    let cy1 = -k * ry * x1 / rx;
    let cx = cp * cx1 - sp * cy1 + (p0[0] + p1[0]) / 2.0;
    let cy = sp * cx1 + cp * cy1 + (p0[1] + p1[1]) / 2.0;
    let ang = |ux: f32, uy: f32, vx: f32, vy: f32| (ux * vy - uy * vx).atan2(ux * vx + uy * vy);
    let t1 = ang(1.0, 0.0, (x1 - cx1) / rx, (y1 - cy1) / ry);
    let mut dt = ang((x1 - cx1) / rx, (y1 - cy1) / ry, (-x1 - cx1) / rx, (-y1 - cy1) / ry);
    if !sweep && dt > 0.0 {
        dt -= std::f32::consts::TAU;
    } else if sweep && dt < 0.0 {
        dt += std::f32::consts::TAU;
    }
    let n = ((dt.abs() * rx.max(ry)) / seg).ceil().clamp(2.0, 256.0) as usize;
    (1..=n)
        .map(|i| {
            if i == n {
                return p1;
            }
            let t = t1 + dt * i as f32 / n as f32;
            let (st, ct) = t.sin_cos();
            [cp * rx * ct - sp * ry * st + cx, sp * rx * ct + cp * ry * st + cy]
        })
        .collect()
}

fn cubic(p0: [f32; 2], c1: [f32; 2], c2: [f32; 2], p1: [f32; 2], seg: f32, out: &mut Vec<[f32; 2]>) {
    let d = |a: [f32; 2], b: [f32; 2]| (a[0] - b[0]).hypot(a[1] - b[1]);
    let len = d(p0, c1) + d(c1, c2) + d(c2, p1);
    let n = (len / seg).ceil().clamp(2.0, 128.0) as usize;
    for i in 1..=n {
        let t = i as f32 / n as f32;
        let u = 1.0 - t;
        let (a, b, c, e) = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
        out.push([a * p0[0] + b * c1[0] + c * c2[0] + e * p1[0], a * p0[1] + b * c1[1] + c * c2[1] + e * p1[1]]);
    }
}

/// An SVG path's `d`, flattened into subpaths in its own units; `seg` is the longest a flattened piece may be.
pub fn parse_path(d: &str, seg: f32) -> Vec<SubPath> {
    let mut t = Tokens { s: d.as_bytes(), i: 0 };
    let mut out: Vec<SubPath> = Vec::new();
    let mut cur = SubPath::default();
    let (mut x, mut y) = (0.0f32, 0.0f32);
    let (mut sx, mut sy) = (0.0f32, 0.0f32);
    let mut last_ctrl: Option<[f32; 2]> = None;
    let mut last_q: Option<[f32; 2]> = None;
    let mut cmd = b'M';
    loop {
        if let Some(c) = t.cmd() {
            cmd = c;
        } else if !t.at_number() {
            break;
        }
        let rel = cmd.is_ascii_lowercase();
        let (ox, oy) = if rel { (x, y) } else { (0.0, 0.0) };
        let up = cmd.to_ascii_uppercase();
        let (mut ctrl, mut qctrl) = (None, None);
        match up {
            b'M' => {
                // A lone point (a moveto, or the point a closepath left) draws nothing.
                if cur.pts.len() > 1 {
                    out.push(std::mem::take(&mut cur));
                }
                cur = SubPath::default();
                x = ox + t.num();
                y = oy + t.num();
                sx = x;
                sy = y;
                cur.pts.push([x, y]);
                // Further pairs after a moveto are linetos.
                cmd = if rel { b'l' } else { b'L' };
            }
            b'L' => {
                x = ox + t.num();
                y = oy + t.num();
                cur.pts.push([x, y]);
            }
            b'H' => {
                x = ox + t.num();
                cur.pts.push([x, y]);
            }
            b'V' => {
                y = oy + t.num();
                cur.pts.push([x, y]);
            }
            b'C' | b'S' => {
                let c1 = if up == b'C' {
                    [ox + t.num(), oy + t.num()]
                } else {
                    last_ctrl.map(|c| [2.0 * x - c[0], 2.0 * y - c[1]]).unwrap_or([x, y])
                };
                let c2 = [ox + t.num(), oy + t.num()];
                let p1 = [ox + t.num(), oy + t.num()];
                cubic([x, y], c1, c2, p1, seg, &mut cur.pts);
                ctrl = Some(c2);
                x = p1[0];
                y = p1[1];
            }
            b'Q' | b'T' => {
                let q = if up == b'Q' {
                    [ox + t.num(), oy + t.num()]
                } else {
                    last_q.map(|c| [2.0 * x - c[0], 2.0 * y - c[1]]).unwrap_or([x, y])
                };
                let p1 = [ox + t.num(), oy + t.num()];
                let c1 = [x + 2.0 / 3.0 * (q[0] - x), y + 2.0 / 3.0 * (q[1] - y)];
                let c2 = [p1[0] + 2.0 / 3.0 * (q[0] - p1[0]), p1[1] + 2.0 / 3.0 * (q[1] - p1[1])];
                cubic([x, y], c1, c2, p1, seg, &mut cur.pts);
                qctrl = Some(q);
                x = p1[0];
                y = p1[1];
            }
            b'A' => {
                let (rx, ry, rot) = (t.num(), t.num(), t.num());
                let (large, sweep) = (t.flag(), t.flag());
                let p1 = [ox + t.num(), oy + t.num()];
                if cur.pts.is_empty() {
                    cur.pts.push([x, y]);
                }
                cur.pts.extend(arc_points([x, y], rx, ry, rot, large, sweep, p1, seg));
                x = p1[0];
                y = p1[1];
            }
            b'Z' => {
                cur.closed = true;
                out.push(std::mem::take(&mut cur));
                x = sx;
                y = sy;
                cur.pts.push([x, y]);
            }
            _ => break,
        }
        last_ctrl = ctrl;
        last_q = qctrl;
    }
    if cur.pts.len() > 1 {
        out.push(cur);
    }
    // A closed subpath's repeated start point is implied by `closed`.
    for s in &mut out {
        if s.closed && s.pts.len() > 2 {
            let (a, b) = (s.pts[0], s.pts[s.pts.len() - 1]);
            if (a[0] - b[0]).abs() < 1e-4 && (a[1] - b[1]).abs() < 1e-4 {
                s.pts.pop();
            }
        }
    }
    out.retain(|s| !s.pts.is_empty());
    out
}

fn attr(el: &str, name: &str) -> Option<String> {
    let key = format!(" {name}=\"");
    let i = el.find(&key)? + key.len();
    let j = el[i..].find('"')? + i;
    Some(el[i..j].to_owned())
}

fn attr_f(el: &str, name: &str) -> Option<f32> {
    attr(el, name).and_then(|v| v.trim().parse().ok())
}

/// Circle as a closed subpath.
fn circle_path(cx: f32, cy: f32, r: f32, seg: f32) -> SubPath {
    let n = ((std::f32::consts::TAU * r) / seg).ceil().clamp(12.0, 128.0) as usize;
    SubPath {
        pts: (0..n)
            .map(|i| {
                let a = std::f32::consts::TAU * i as f32 / n as f32;
                [cx + r * a.cos(), cy + r * a.sin()]
            })
            .collect(),
        closed: true,
    }
}

/// A rounded rectangle as a closed subpath.
fn rrect_path(x: f32, y: f32, w: f32, h: f32, rx: f32, seg: f32) -> SubPath {
    let r = rx.min(w / 2.0).min(h / 2.0).max(0.0);
    if r < 1e-4 {
        return SubPath { pts: vec![[x, y], [x + w, y], [x + w, y + h], [x, y + h]], closed: true };
    }
    let n = ((std::f32::consts::FRAC_PI_2 * r) / seg).ceil().clamp(2.0, 32.0) as usize;
    let mut pts = Vec::new();
    for (cx, cy, a0) in
        [(x + w - r, y + r, -90.0f32), (x + w - r, y + h - r, 0.0), (x + r, y + h - r, 90.0), (x + r, y + r, 180.0)]
    {
        for i in 0..=n {
            let a = (a0 + 90.0 * i as f32 / n as f32).to_radians();
            pts.push([cx + r * a.cos(), cy + r * a.sin()]);
        }
    }
    SubPath { pts, closed: true }
}

/// An icon's SVG fragment (paths, circles and rects with their dash and width overrides), flattened.
fn parse_icon(frag: &str) -> Vec<IconPart> {
    const SEG: f32 = 0.35;
    let mut parts = Vec::new();
    for el in frag.split('<').filter(|e| !e.trim().is_empty()) {
        let el = format!(" {}", el.trim_end_matches(['/', '>', ' ']));
        let tag = el.split_whitespace().next().unwrap_or("");
        let subpaths = match tag {
            "path" => parse_path(&attr(&el, "d").unwrap_or_default(), SEG),
            "circle" => vec![circle_path(
                attr_f(&el, "cx").unwrap_or(0.0),
                attr_f(&el, "cy").unwrap_or(0.0),
                attr_f(&el, "r").unwrap_or(0.0),
                SEG,
            )],
            "rect" => vec![rrect_path(
                attr_f(&el, "x").unwrap_or(0.0),
                attr_f(&el, "y").unwrap_or(0.0),
                attr_f(&el, "width").unwrap_or(0.0),
                attr_f(&el, "height").unwrap_or(0.0),
                attr_f(&el, "rx").unwrap_or(0.0),
                SEG,
            )],
            _ => continue,
        };
        let dash = attr(&el, "stroke-dasharray").and_then(|v| {
            let n: Vec<f32> = v.split_whitespace().filter_map(|x| x.parse().ok()).collect();
            (n.len() == 2).then(|| [n[0], n[1]])
        });
        parts.push(IconPart { subpaths, dash, width: attr_f(&el, "stroke-width") });
    }
    parts
}

// ------------------------------------------------------------------------------------------------ fills

fn signed_area(p: &[Pos2]) -> f32 {
    let mut a = 0.0;
    for i in 0..p.len() {
        let (u, v) = (p[i], p[(i + 1) % p.len()]);
        a += u.x * v.y - v.x * u.y;
    }
    a / 2.0
}

fn is_convex(p: &[Pos2]) -> bool {
    let n = p.len();
    if n < 4 {
        return true;
    }
    let mut sign = 0.0f32;
    for i in 0..n {
        let (a, b, c) = (p[i], p[(i + 1) % n], p[(i + 2) % n]);
        let z = (b.x - a.x) * (c.y - b.y) - (b.y - a.y) * (c.x - b.x);
        if z.abs() < 1e-6 {
            continue;
        }
        if sign == 0.0 {
            sign = z.signum();
        } else if z.signum() != sign {
            return false;
        }
    }
    true
}

/// A closed outline. egui mitres every corner of a closed path, so a sharp corner spikes; such an outline is drawn
/// as an open path back to its start, whose sharp corners egui cuts off.
fn outline(mut pts: Vec<Pos2>, stroke: PathStroke) -> Shape {
    if has_sharp_corner(&pts) {
        pts.push(pts[0]);
        Shape::Path(PathShape::line(pts, stroke))
    } else {
        Shape::Path(PathShape::closed_line(pts, stroke))
    }
}

/// Whether a polygon has a corner sharper than about 15 degrees.
fn has_sharp_corner(p: &[Pos2]) -> bool {
    let n = p.len();
    (0..n).any(|i| {
        let (a, b, c) = (p[(i + n - 1) % n], p[i], p[(i + 1) % n]);
        let (u, v) = ((a - b).normalized(), (c - b).normalized());
        u.dot(v) > 0.966
    })
}

/// Triangles of a simple polygon (ear clipping), as indices into `p`.
fn triangulate(p: &[Pos2]) -> Vec<u32> {
    let n = p.len();
    let mut idx: Vec<usize> = (0..n).collect();
    if signed_area(p) < 0.0 {
        idx.reverse();
    }
    let cross = |a: Pos2, b: Pos2, c: Pos2| (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x);
    let inside =
        |a: Pos2, b: Pos2, c: Pos2, q: Pos2| cross(a, b, q) >= 0.0 && cross(b, c, q) >= 0.0 && cross(c, a, q) >= 0.0;
    let mut tris = Vec::with_capacity(3 * n);
    let mut guard = 0;
    while idx.len() > 3 && guard < 4 * n * n {
        guard += 1;
        let m = idx.len();
        let mut clipped = false;
        for i in 0..m {
            let (ia, ib, ic) = (idx[(i + m - 1) % m], idx[i], idx[(i + 1) % m]);
            let (a, b, c) = (p[ia], p[ib], p[ic]);
            if cross(a, b, c) <= 0.0 {
                continue;
            }
            if idx.iter().any(|&j| j != ia && j != ib && j != ic && inside(a, b, c, p[j])) {
                continue;
            }
            tris.extend([ia as u32, ib as u32, ic as u32]);
            idx.remove(i);
            clipped = true;
            break;
        }
        if !clipped {
            break;
        }
    }
    if idx.len() == 3 {
        tris.extend(idx.iter().map(|&i| i as u32));
    }
    tris
}

// ------------------------------------------------------------------------------------------------ text

/// Where text sits on its x (SVG's `text-anchor`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Anchor {
    #[default]
    Start,
    Middle,
    End,
}

/// Which line sits on its y (SVG's `dominant-baseline`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Base {
    #[default]
    Alphabetic,
    Middle,
}

/// Text options: the mockup's `text(x, y, size, col, str, { a, w, ls, b })`.
#[derive(Clone, Copy, Debug)]
pub struct T {
    pub a: Anchor,
    /// 600 or 700.
    pub w: u16,
    /// Letter spacing, layout points (the mockup's default is 0.5).
    pub ls: f32,
    pub b: Base,
}

impl Default for T {
    fn default() -> Self {
        T { a: Anchor::Start, w: 600, ls: 0.5, b: Base::Alphabetic }
    }
}

impl T {
    /// Anchored in the middle.
    pub fn mid() -> Self {
        T { a: Anchor::Middle, ..T::default() }
    }
    /// Anchored at its end.
    pub fn end() -> Self {
        T { a: Anchor::End, ..T::default() }
    }
    /// Bold.
    pub fn bold(self) -> Self {
        T { w: 700, ..self }
    }
    /// With this letter spacing.
    pub fn ls(self, ls: f32) -> Self {
        T { ls, ..self }
    }
    /// Centred on the x-height's middle.
    pub fn vmid(self) -> Self {
        T { b: Base::Middle, ..self }
    }
}

/// Barlow's x-height over its em (OS/2 sxHeight 514 of 1000): SVG's `dominant-baseline: middle` centres on half of it.
const X_HEIGHT: f32 = 0.514;

// ------------------------------------------------------------------------------------------------ the canvas

/// A stroke: width in user units, colour, and SVG's round caps and joins or its butt caps and miters.
#[derive(Clone, Copy, Debug)]
pub struct Pen {
    pub w: f32,
    pub col: Color32,
    pub round: bool,
    /// A dash pattern [on, off] and its offset, in user units.
    pub dash: Option<[f32; 3]>,
}

impl Pen {
    /// A plain stroke.
    pub fn new(w: f32, col: Color32) -> Self {
        Pen { w, col, round: false, dash: None }
    }
    /// With round caps and joins.
    pub fn round(self) -> Self {
        Pen { round: true, ..self }
    }
    /// Dashed: `on` then `off`, starting `offset` into the pattern (SVG's `stroke-dashoffset`).
    pub fn dash(self, on: f32, off: f32, offset: f32) -> Self {
        Pen { dash: Some([on, off, offset]), ..self }
    }
}

/// A painter in the mockup's user space: an origin and a scale from layout points to egui points.
#[derive(Clone)]
pub struct Canvas {
    pub painter: Painter,
    /// Where user (0, 0) is, in egui points.
    pub o: Vec2,
    /// Egui points per user unit.
    pub s: f32,
}

impl Canvas {
    /// A canvas on `painter` with its origin at `o` and `s` points per unit.
    pub fn new(painter: Painter, o: Vec2, s: f32) -> Self {
        Canvas { painter, o, s }
    }

    /// A user point on screen.
    pub fn p(&self, x: f32, y: f32) -> Pos2 {
        Pos2::new(self.o.x + x * self.s, self.o.y + y * self.s)
    }

    /// A user rectangle on screen.
    pub fn r(&self, x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect::from_min_max(self.p(x, y), self.p(x + w, y + h))
    }

    /// The same canvas with its origin moved to user (x, y): SVG's `translate`, or a nested `<svg x y>`.
    pub fn at(&self, x: f32, y: f32) -> Canvas {
        Canvas { painter: self.painter.clone(), o: self.p(x, y).to_vec2(), s: self.s }
    }

    /// The same canvas, drawing only inside a user rectangle (a nested `<svg>` with `overflow: hidden`).
    pub fn clip(&self, x: f32, y: f32, w: f32, h: f32) -> Canvas {
        let r = self.r(x, y, w, h).intersect(self.painter.clip_rect());
        Canvas { painter: self.painter.with_clip_rect(r), o: self.o, s: self.s }
    }

    fn add(&self, s: Shape) {
        self.painter.add(s);
    }

    /// `<rect x y width height rx fill stroke stroke-width>`.
    #[allow(clippy::too_many_arguments)]
    pub fn rect(&self, x: f32, y: f32, w: f32, h: f32, rx: f32, fill: Color32, stroke: Option<Pen>) {
        if w <= 0.0 || h <= 0.0 {
            return;
        }
        let r = self.r(x, y, w, h);
        let rad = (rx * self.s).min(r.width() / 2.0).min(r.height() / 2.0).clamp(0.0, 255.0);
        let st = stroke.filter(|p| p.dash.is_none()).map(|p| Stroke::new(p.w * self.s, p.col)).unwrap_or(Stroke::NONE);
        self.add(Shape::Rect(RectShape::new(r, CornerRadius::same(rad.round() as u8), fill, st, StrokeKind::Middle)));
        if let Some(p) = stroke.filter(|p| p.dash.is_some()) {
            self.stroke(&[rrect_path(x, y, w, h, rx, 1.0)], &Xf::ID, p);
        }
    }

    /// `<circle cx cy r fill stroke>`.
    pub fn circle(&self, cx: f32, cy: f32, r: f32, fill: Color32, stroke: Option<Pen>) {
        let st = stroke.filter(|p| p.dash.is_none()).map(|p| Stroke::new(p.w * self.s, p.col)).unwrap_or(Stroke::NONE);
        self.add(Shape::circle_filled(self.p(cx, cy), r * self.s, fill));
        if st.width > 0.0 {
            self.add(Shape::circle_stroke(self.p(cx, cy), r * self.s, st));
        }
        if let Some(p) = stroke.filter(|p| p.dash.is_some()) {
            self.stroke(&[circle_path(cx, cy, r, 1.0)], &Xf::ID, p);
        }
    }

    /// `<ellipse cx cy rx ry fill stroke>`.
    pub fn ellipse(&self, cx: f32, cy: f32, rx: f32, ry: f32, fill: Color32, stroke: Option<Pen>) {
        let st = stroke.map(|p| Stroke::new(p.w * self.s, p.col)).unwrap_or(Stroke::NONE);
        self.add(Shape::ellipse_filled(self.p(cx, cy), Vec2::new(rx, ry) * self.s, fill));
        if st.width > 0.0 {
            self.add(Shape::ellipse_stroke(self.p(cx, cy), Vec2::new(rx, ry) * self.s, st));
        }
    }

    /// A straight line from (x0, y0) to (x1, y1).
    pub fn seg(&self, x0: f32, y0: f32, x1: f32, y1: f32, pen: Pen) {
        self.stroke(&[SubPath { pts: vec![[x0, y0], [x1, y1]], closed: false }], &Xf::ID, pen);
    }

    /// An open polyline through user points.
    pub fn polyline(&self, pts: &[[f32; 2]], pen: Pen) {
        self.stroke(&[SubPath { pts: pts.to_vec(), closed: false }], &Xf::ID, pen);
    }

    /// A filled polygon through user points (any simple polygon).
    pub fn poly(&self, pts: &[[f32; 2]], fill: Color32) {
        self.fill(&[SubPath { pts: pts.to_vec(), closed: true }], &Xf::ID, fill);
    }

    /// `<path d transform fill stroke>`: `xf` maps the path's units to user units.
    pub fn path(&self, d: &str, xf: Xf, fill: Option<Color32>, stroke: Option<Pen>) {
        let seg = 1.5 / (xf.scale() * self.s).max(1e-3);
        let sub = parse_path(d, seg);
        if let Some(f) = fill {
            self.fill(&sub, &xf, f);
        }
        if let Some(p) = stroke {
            self.stroke(&sub, &xf, p);
        }
    }

    /// Fill flattened subpaths, each as its own polygon (they overlap rather than cut: nonzero, same winding).
    pub fn fill(&self, sub: &[SubPath], xf: &Xf, col: Color32) {
        if col.a() == 0 {
            return;
        }
        for s in sub {
            if s.pts.len() < 3 {
                continue;
            }
            let mut pts: Vec<Pos2> = Vec::with_capacity(s.pts.len());
            for q in s.pts.iter().map(|q| xf.ap(*q)).map(|q| self.p(q[0], q[1])) {
                // A repeated point (a pole of a sphere's patch) has no edge normal: egui's fill would spike there.
                if pts.last().is_none_or(|l: &Pos2| l.distance(q) > 1e-3) {
                    pts.push(q);
                }
            }
            while pts.len() > 2 && pts[0].distance(pts[pts.len() - 1]) <= 1e-3 {
                pts.pop();
            }
            if pts.len() < 3 || signed_area(&pts).abs() < 1e-4 {
                continue;
            }
            // egui feathers a convex fill's edge outward when it winds clockwise on screen.
            if signed_area(&pts) < 0.0 {
                pts.reverse();
            }
            // egui feathers a convex fill by mitring its corners outward; a sliver's sharp corner mitres into a long
            // spike, so a polygon with any corner under about 15 degrees is filled without feathering.
            if is_convex(&pts) && !has_sharp_corner(&pts) {
                self.add(Shape::convex_polygon(pts, col, Stroke::NONE));
            } else {
                let tris = triangulate(&pts);
                let mut mesh = Mesh::default();
                for q in &pts {
                    mesh.vertices.push(Vertex { pos: *q, uv: WHITE_UV, color: col });
                }
                mesh.indices = tris;
                self.add(Shape::mesh(mesh));
                // A thin outline of the same colour stands in for the edge's anti-aliasing.
                self.add(outline(pts, PathStroke::new(0.6_f32, col)));
            }
        }
    }

    /// Stroke flattened subpaths with a pen.
    pub fn stroke(&self, sub: &[SubPath], xf: &Xf, pen: Pen) {
        let k = xf.scale() * self.s;
        let w = pen.w * k;
        if w <= 0.0 || pen.col.a() == 0 {
            return;
        }
        for s in sub {
            let pts: Vec<[f32; 2]> = s.pts.iter().map(|q| xf.ap(*q)).collect();
            let pieces = match pen.dash {
                Some([on, off, offset]) => {
                    dashes(&pts, s.closed, on * xf.scale(), off * xf.scale(), offset * xf.scale())
                }
                None => vec![(pts, s.closed)],
            };
            for (pts, closed) in pieces {
                let mut scr: Vec<Pos2> = Vec::with_capacity(pts.len());
                for q in pts.iter().map(|q| self.p(q[0], q[1])) {
                    if scr.last().is_none_or(|l: &Pos2| l.distance(q) > 1e-3) {
                        scr.push(q);
                    }
                }
                if scr.len() == 1 {
                    scr.push(scr[0]);
                }
                let degenerate = scr.windows(2).all(|w2| w2[0].distance(w2[1]) < 1e-3);
                if degenerate {
                    if pen.round && !scr.is_empty() {
                        self.add(Shape::circle_filled(scr[0], w / 2.0, pen.col));
                    }
                    continue;
                }
                if pen.round && w >= 1.2 {
                    // Round caps and joins: a disc at the ends and the corners.
                    let n = scr.len();
                    for i in 0..n {
                        let end = !closed && (i == 0 || i == n - 1);
                        let corner = i > 0 && i + 1 < n || closed;
                        let sharp = corner && {
                            let (a, b, c) = (scr[(i + n - 1) % n], scr[i], scr[(i + 1) % n]);
                            let (u, v) = (b - a, c - b);
                            let cos = u.normalized().dot(v.normalized());
                            cos < 0.9
                        };
                        if end || sharp {
                            self.add(Shape::circle_filled(scr[i], w / 2.0, pen.col));
                        }
                    }
                }
                let stroke = PathStroke::new(w, pen.col);
                if closed {
                    self.add(outline(scr, stroke));
                } else {
                    self.add(Shape::Path(PathShape::line(scr, stroke)));
                }
            }
        }
    }

    /// The mockup's `icon(name, x, y, s, col, { sw })`: a 24-unit icon centred on (x, y), `size` across.
    pub fn icon(&self, name: &str, x: f32, y: f32, size: f32, col: Color32) {
        self.icon_w(name, x, y, size, col, 2.0);
    }

    /// An icon with its stroke width in icon units.
    pub fn icon_w(&self, name: &str, x: f32, y: f32, size: f32, col: Color32, sw: f32) {
        let Some(parts) = style().icons.get(name) else { return };
        let xf = Xf::tr(x - size / 2.0, y - size / 2.0).sc(size / 24.0);
        for part in parts {
            let mut pen = Pen::new(part.width.unwrap_or(sw), col).round();
            if let Some([on, off]) = part.dash {
                pen = pen.dash(on, off, 0.0);
            }
            self.stroke(&part.subpaths, &xf, pen);
        }
    }

    /// Lay text out: its galley, its advance in egui points (SVG counts the letter spacing after the last character
    /// too) and the galley's baseline below its top.
    fn layout(&self, size: f32, col: Color32, s: &str, t: T) -> (std::sync::Arc<egui::Galley>, f32, f32) {
        let family = FontFamily::Name(if t.w >= 700 { BOLD } else { SEMIBOLD }.into());
        let mut job = egui::text::LayoutJob::default();
        job.append(
            s,
            0.0,
            egui::TextFormat {
                font_id: FontId::new(size * self.s, family),
                color: col,
                extra_letter_spacing: t.ls * self.s,
                ..Default::default()
            },
        );
        let galley = self.painter.layout_job(job);
        let base =
            galley.rows.first().and_then(|r| r.row.glyphs.first().map(|g| r.pos.y + g.pos.y)).unwrap_or(size * self.s);
        let adv = galley.size().x + t.ls * self.s;
        (galley, adv, base)
    }

    /// Where a laid-out text's top-left goes, in user units, for its anchor and baseline at (x, y).
    fn text_corner(&self, x: f32, y: f32, size: f32, adv: f32, base: f32, t: T) -> [f32; 2] {
        let left = match t.a {
            Anchor::Start => 0.0,
            Anchor::Middle => adv / 2.0,
            Anchor::End => adv,
        };
        let base_y = match t.b {
            Base::Alphabetic => y,
            Base::Middle => y + X_HEIGHT * size / 2.0,
        };
        [x - left / self.s, base_y - base / self.s]
    }

    /// The mockup's `text(x, y, size, col, str, opts)`; returns its advance in user units.
    pub fn text(&self, x: f32, y: f32, size: f32, col: Color32, s: &str, t: T) -> f32 {
        if s.is_empty() || col.a() == 0 {
            return 0.0;
        }
        let (galley, adv, base) = self.layout(size, col, s, t);
        let [lx, ly] = self.text_corner(x, y, size, adv, base, t);
        self.painter.galley(self.p(lx, ly), galley, col);
        adv / self.s
    }

    /// Text laid out at (x, y) inside a transformed group (`xf`, which turns by `deg`): the navball's numbers.
    #[allow(clippy::too_many_arguments)]
    pub fn text_turned(&self, x: f32, y: f32, size: f32, col: Color32, s: &str, t: T, xf: &Xf, deg: f32) {
        if s.is_empty() || col.a() == 0 {
            return;
        }
        let (galley, adv, base) = self.layout(size, col, s, t);
        let corner = xf.ap(self.text_corner(x, y, size, adv, base, t));
        let mut shape = egui::epaint::TextShape::new(self.p(corner[0], corner[1]), galley, col);
        shape.angle = deg.to_radians();
        self.painter.add(shape);
    }

    /// A rectangle filled with a vertical gradient: `stops` are (offset 0-1, colour), top to bottom.
    pub fn vgrad(&self, x: f32, y: f32, w: f32, h: f32, stops: &[(f32, Color32)]) {
        let mut mesh = Mesh::default();
        for (i, (k, c)) in stops.iter().enumerate() {
            let yy = y + h * k;
            mesh.vertices.push(Vertex { pos: self.p(x, yy), uv: WHITE_UV, color: *c });
            mesh.vertices.push(Vertex { pos: self.p(x + w, yy), uv: WHITE_UV, color: *c });
            if i > 0 {
                let b = (2 * i) as u32;
                mesh.indices.extend([b - 2, b - 1, b + 1, b - 2, b + 1, b]);
            }
        }
        self.add(Shape::mesh(mesh));
    }

    /// Raw triangles in user units with a colour per vertex.
    pub fn mesh(&self, verts: &[([f32; 2], Color32)], idx: Vec<u32>) {
        let mut mesh = Mesh::default();
        for (q, c) in verts {
            mesh.vertices.push(Vertex { pos: self.p(q[0], q[1]), uv: WHITE_UV, color: *c });
        }
        mesh.indices = idx;
        self.add(Shape::mesh(mesh));
    }
}

/// A polyline cut into dashes: [on, off] repeating from `offset` into the pattern, as SVG's stroke-dasharray.
fn dashes(pts: &[[f32; 2]], closed: bool, on: f32, off: f32, offset: f32) -> Vec<(Vec<[f32; 2]>, bool)> {
    let mut pts = pts.to_vec();
    if closed && !pts.is_empty() {
        pts.push(pts[0]);
    }
    let period = (on + off).max(1e-3);
    let mut out = Vec::new();
    let mut phase = offset.rem_euclid(period);
    let mut cur: Vec<[f32; 2]> = Vec::new();
    let mut drawing = phase < on;
    if drawing && !pts.is_empty() {
        cur.push(pts[0]);
    }
    for w in pts.windows(2) {
        let (a, b) = (w[0], w[1]);
        let len = (b[0] - a[0]).hypot(b[1] - a[1]);
        let mut t = 0.0;
        while t < len {
            let left = if drawing { on - phase } else { period - phase };
            let step = left.min(len - t);
            t += step;
            phase += step;
            let q = [a[0] + (b[0] - a[0]) * t / len.max(1e-9), a[1] + (b[1] - a[1]) * t / len.max(1e-9)];
            if drawing {
                cur.push(q);
            }
            if drawing && phase >= on - 1e-6 {
                if cur.len() > 1 {
                    out.push((std::mem::take(&mut cur), false));
                }
                cur.clear();
                drawing = false;
            } else if !drawing && phase >= period - 1e-6 {
                phase = 0.0;
                drawing = true;
                cur.push(q);
            }
        }
    }
    if drawing && cur.len() > 1 {
        out.push((cur, false));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_icon_in_the_mockup_parses_to_something_drawable() {
        let st = style();
        assert!(st.icons.len() >= 60, "the mockup has 64 icons");
        for (name, parts) in &st.icons {
            assert!(!parts.is_empty(), "{name} has no parts");
            for p in parts {
                for s in &p.subpaths {
                    assert!(
                        s.pts.iter().all(|q| q[0] > -2.0 && q[0] < 26.0 && q[1] > -2.0 && q[1] < 26.0),
                        "{name} leaves its grid"
                    );
                }
            }
        }
    }

    #[test]
    fn an_arc_ends_where_svg_says_and_bulges_the_right_way() {
        // The "speed" icon's dial: from (4, 18) round over the top to (20, 18).
        let s = parse_path("M4 18a8 8 0 1 1 16 0", 0.2);
        let pts = &s[0].pts;
        let last = pts[pts.len() - 1];
        assert!((last[0] - 20.0).abs() < 1e-3 && (last[1] - 18.0).abs() < 1e-3);
        let top = pts.iter().map(|q| q[1]).fold(f32::INFINITY, f32::min);
        assert!((top - 10.0).abs() < 0.05, "the arc's top is 8 above its chord, at y 10, not {top}");
    }

    #[test]
    fn relative_commands_and_closepath_follow_the_spec() {
        let s = parse_path("M3 9h12l5 3-5 3H3zM7 9V7", 1.0);
        assert_eq!(s.len(), 2);
        assert!(s[0].closed);
        assert_eq!(s[0].pts, vec![[3.0, 9.0], [15.0, 9.0], [20.0, 12.0], [15.0, 15.0], [3.0, 15.0]]);
        assert_eq!(s[1].pts, vec![[7.0, 9.0], [7.0, 7.0]]);
    }

    #[test]
    fn a_concave_glyph_triangulates_into_its_own_area() {
        // The corvette: an arrowhead with a notch at its stern.
        let p: Vec<Pos2> =
            [[0.0, -12.0], [7.0, 8.0], [0.0, 4.0], [-7.0, 8.0]].iter().map(|q| Pos2::new(q[0], q[1])).collect();
        assert!(!is_convex(&p));
        let tris = triangulate(&p);
        assert_eq!(tris.len(), 6);
        let area: f32 =
            tris.chunks(3).map(|t| signed_area(&[p[t[0] as usize], p[t[1] as usize], p[t[2] as usize]]).abs()).sum();
        assert!((area - signed_area(&p).abs()).abs() < 1e-3);
    }

    #[test]
    fn dashes_keep_the_pattern_and_its_offset() {
        let d = dashes(&[[0.0, 0.0], [20.0, 0.0]], false, 4.0, 4.0, 0.0);
        assert_eq!(d.len(), 3);
        assert_eq!(d[1].0.first(), Some(&[8.0, 0.0]));
        let d = dashes(&[[0.0, 0.0], [20.0, 0.0]], false, 4.0, 4.0, 2.0);
        assert_eq!(d[0].0, vec![[0.0, 0.0], [2.0, 0.0]], "an offset of 2 starts half way through the first dash");
    }

    #[test]
    fn transforms_compose_as_svg_writes_them() {
        // translate(10, 0) rotate(90): the point (1, 0) turns to (0, 1), then moves.
        let xf = Xf::tr(10.0, 0.0).rot(90.0);
        let q = xf.ap([1.0, 0.0]);
        assert!((q[0] - 10.0).abs() < 1e-5 && (q[1] - 1.0).abs() < 1e-5);
    }
}
