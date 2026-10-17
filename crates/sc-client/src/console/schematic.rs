//! Engineering's schematic, full screen (openspec/changes/power-routing design 6.3-6.5). The approved mockup is
//! `docs/mockups/power-grid.html` (version 13): this draws what it draws, from the snapshot's routing and grid, on the
//! console's canvas between the title band and the status strip.
//!
//! - 2D, the board (design 3c): `power_board.json`'s parts and traces, every corner cut at 45, the spares on the
//!   bottom layer, a hop through two vias where two traces cross, the chips with their switches, lamps and system
//!   lights in their families' colours (`subsystems.json`, design 3e).
//! - 3D, the ship (design 3a): the decks pulled apart, the rooms' floors, every run and feeder in its own lane with
//!   its corners cut, drawn as round-joined strokes on a dark outline, the markers; a drag orbits, the wheel zooms.
//!
//! Every tap is the mockup's: a box's switch, a run, a system's light swaps its feed. A tap acts on release, so a drag
//! that starts on a run turns the view instead. The client works out no rule of the grid's: what is live, reached,
//! cut or dropped is the snapshot's (`RouteSnap`); this file only decides how it looks.

use super::kit::{Act, Drag, Hit};
use super::Ctx;
use crate::vg::{self, alpha, Canvas, Pen, Xf, T};
use egui::{Color32, Pos2};
use sc_core::combat::data::DrillData;
use sc_core::power::PowerData;
use sc_core::schematic::{Families, Family, PowerBoard, BATTERY_LINK};
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

/// The view's area on the console: between the title band and the status strip.
const AREA: [f32; 4] = [0.0, 32.0, 1280.0, 656.0];
/// The left column (the status box, the legend in 3D), as the mockup's `leftCol()`.
const LEFT: f32 = 344.0;
/// How fast every trace's dashes run, points a second (design 3a: one speed).
const FLOW_PX_S: f32 = 36.0;

/// The schematic as the seat shows it (`ConsoleView::schem`): how it is viewed, and the routing and the grid now.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SchemView {
    /// 3D (true), or the board.
    pub three_d: bool,
    /// The 3D camera: yaw and pitch, degrees, and its distance from the ship's middle, metres.
    pub cam: [f64; 3],
    /// The board's zoom (1: fitted) and its pan, console points.
    pub zoom: f64,
    pub pan: [f64; 2],
    /// Each node's box on, reached by power, and its health, 0-1.
    pub box_on: Vec<bool>,
    pub reached: Vec<bool>,
    pub node_health: Vec<f64>,
    /// Each conduit closed, cut and damaged.
    pub closed: Vec<bool>,
    pub severed: Vec<bool>,
    pub damaged: Vec<bool>,
    /// The generators' breakers, then the ties', closed.
    pub switches: Vec<bool>,
    /// Each load on its second feed, dropped, and its supply (none: it wants nothing).
    pub on_alt: Vec<bool>,
    pub dropped: Vec<bool>,
    pub supply: Vec<Option<f64>>,
    /// Each load's integrity, percent (100 where damage holds none: the lights).
    pub integrity: Vec<f64>,
    /// The flow on every edge, MW (`PowerData::edge_count`'s order).
    pub flows: Vec<f64>,
    /// The live runs, the limit, and the trip's seconds left.
    pub live: u32,
    pub limit: u32,
    pub trip_s: f64,
    /// The reactor running and its throttle; the battery's charge and what it gives, MW; delivered and wanted, MW.
    pub rx_up: bool,
    pub throttle: f64,
    pub soc: f64,
    pub bat_out_mw: f64,
    pub got_mw: f64,
    pub want_mw: f64,
}

/// The board in small on the SCHEMATIC panel (power-routing 6.2): each edge's state in `PowerData::edge_count`'s order
/// (live 0, dead 1, open 2, cut 3) and each node's (lit 0, nothing reaching it 1, off 2, destroyed 3, plus 8 when
/// damaged). The mockup writes the same codes, so the two draw the same picture.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ThumbView {
    pub edges: Vec<u8>,
    pub nodes: Vec<u8>,
}

/// The board in small's codes from the schematic's state.
pub fn thumb(v: &SchemView) -> ThumbView {
    let s = data();
    let edges = (0..s.edges.len())
        .map(|k| match edge_state(s, v, k) {
            St::Live => 0,
            St::Dead => 1,
            St::Open => 2,
            St::Cut => 3,
        })
        .collect();
    let nodes = (0..s.pd.nodes.len())
        .map(|n| {
            let h = v.node_health.get(n).copied().unwrap_or(1.0);
            let base = if h <= 0.0 {
                3
            } else if !v.box_on.get(n).copied().unwrap_or(true) {
                2
            } else if !v.reached.get(n).copied().unwrap_or(true) {
                1
            } else {
                0
            };
            base + if h > 0.0 && h < 1.0 - 1e-6 { 8 } else { 0 }
        })
        .collect();
    ThumbView { edges, nodes }
}

/// The schematic's board in small in the box (x, y, w, h) on `b` (the mockup's `boardThumb`): the boards, every trace in
/// its state (live yellow, dead copper, an open spare thin blue, cut red), every part (lit, nothing reaching it faint,
/// off dim, destroyed red, a damaged one with a red rim), the reactor and the battery.
pub fn board_thumb(b: &Canvas, [x, y, w, h]: [f32; 4], t: &ThumbView, rx_up: bool, bat_out: bool) {
    let c = &vg::style().c;
    let sc = &vg::style().schem;
    let pcb = &sc.pcb;
    let s = data();
    let fr = &s.board.board;
    let (bw, bh) = (fr.w as f32, fr.h as f32);
    let k = (w / bw).min(h / bh);
    let (ox, oy) = (x + (w - bw * k) / 2.0, y + (h - bh * k) / 2.0);
    let (bx0, bx1) = (fr.x[0] as f32, fr.x[1] as f32);
    for l in &s.board.layers {
        let (y0, y1) = (l.y[0] as f32, l.y[1] as f32);
        b.rect(
            ox + bx0 * k,
            oy + y0 * k,
            (bx1 - bx0) * k,
            (y1 - y0) * k,
            2.0,
            pcb.mask.0,
            Some(Pen::new(1.0, pcb.mask_edge.0)),
        );
    }
    // The open ones first (the bottom layer), then the copper.
    for pass in [true, false] {
        for (i, e) in s.edges.iter().enumerate() {
            let code = t.edges.get(i).copied().unwrap_or(0);
            let open = code == 2;
            if open != pass {
                continue;
            }
            let (col, sw) = match code {
                3 => (c.danger.0, 1.4),
                2 => (alpha(pcb.bottom.0, 0.7), 0.8),
                1 => (pcb.copper_off.0, 0.0),
                _ => (sc.power.0, 0.0),
            };
            let sw = if sw > 0.0 {
                sw
            } else {
                ((2.4 + 4.6 * (e.rated / 40.0).clamp(0.0, 1.0)) as f32 * k * 2.0).clamp(0.9, 1.6)
            };
            let pts: Vec<[f32; 2]> = e.trace.iter().map(|p| [ox + p[0] * k, oy + p[1] * k]).collect();
            b.polyline(&pts, Pen::new(sw, col).round());
        }
    }
    for (n, id) in s.pd.nodes.iter().enumerate() {
        let Some(p) = s.board.parts.get(id) else { continue };
        let code = t.nodes.get(n).copied().unwrap_or(0);
        let (st, hurt) = (code & 7, code >= 8);
        let fill = match st {
            3 => c.danger.0,
            2 => pcb.hole.0,
            _ => pcb.body.0,
        };
        let edge = if hurt {
            c.danger.0
        } else {
            match st {
                0 => pcb.silk.0,
                1 => c.faint.0,
                _ => c.dim.0,
            }
        };
        let (pw, ph) = (p[2] as f32 * k, p[3] as f32 * k);
        let (px, py) = (ox + p[0] as f32 * k - pw / 2.0, oy + p[1] as f32 * k - ph / 2.0);
        b.rect(px, py, pw, ph, 0.8, fill, Some(Pen::new(if hurt { 1.2 } else { 0.8 }, edge)));
    }
    if let Some(r) = s.board.parts.get("reactor") {
        let col = if rx_up { sc.power.0 } else { c.danger.0 };
        b.circle(
            ox + r[0] as f32 * k,
            oy + r[1] as f32 * k,
            r[2] as f32 / 2.0 * k,
            pcb.body.0,
            Some(Pen::new(1.2, col)),
        );
    }
    if let Some(bt) = s.board.parts.get("battery") {
        let (pw, ph) = (bt[2] as f32 * k, bt[3] as f32 * k);
        let col = if bat_out { c.emergency.0 } else { sc.power.0 };
        b.rect(ox + bt[0] as f32 * k - pw / 2.0, oy + bt[1] as f32 * k - ph / 2.0, pw, ph, 0.8, col, None);
    }
}

// ------------------------------------------------------------------------------------------------ the static data

/// An edge's kind and its index among its kind in `power.json`.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Kind {
    Gen(usize),
    Tie(usize),
    Battery,
    Conduit(usize),
}

/// One edge: what it is, its board trace and its 3D run.
struct Edge {
    name: String,
    kind: Kind,
    rated: f64,
    spare: bool,
    /// Its two nodes (a generator's first end is the reactor: none).
    ends: [Option<usize>; 2],
    /// The board: the trace with its corners cut, where it hops under heavier traces (segment, share along), the vias
    /// where it leaves its board.
    trace: Vec<[f32; 2]>,
    hops: Vec<(usize, f32)>,
    vias: Vec<[f32; 2]>,
    /// 3D: its lane, ship points, and each segment's width, metres.
    lane: Vec<[f64; 3]>,
    ws: Vec<f64>,
}

/// A load's feeder in 3D (its second feed's when `alt`).
struct Feeder {
    load: usize,
    alt: bool,
    lane: Vec<[f64; 3]>,
    ws: Vec<f64>,
}

/// Everything the schematic draws that does not change: built once from the shipped data.
struct Static {
    pd: PowerData,
    board: PowerBoard,
    fam: Families,
    /// Each load's family, by index.
    fam_of: Vec<usize>,
    edges: Vec<Edge>,
    feeders: Vec<Feeder>,
    /// Each node's loads, the vital first and its room's lights last (the mockup's `ON`).
    on: Vec<Vec<usize>>,
    /// Each node's room (layout index), and each load's.
    node_room: Vec<Option<usize>>,
    load_room: Vec<Option<usize>>,
    /// Each node's centre, and each file load's (none for the lights).
    node_at: Vec<[f64; 3]>,
    load_at: Vec<Option<[f64; 3]>>,
    reactor_at: [f64; 3],
    battery_at: [f64; 3],
    /// Each room's brushes: floor and footprint.
    rooms: Vec<Vec<(f64, Vec<[f64; 2]>)>>,
    /// Each room's lights (the lighting load of the panel that lights it), and the emergency lighting's load.
    room_lights: Vec<Option<usize>>,
    emergency_lights: Option<usize>,
    /// The decks, highest first: id, name, floor.
    decks: Vec<(String, String, f64)>,
    /// The ship's bounds in plan: x and z.
    bounds: [f64; 4],
}

fn data() -> &'static Static {
    static S: OnceLock<Static> = OnceLock::new();
    S.get_or_init(|| build(&DrillData::shipped()))
}

/// The polyline with every corner sharper than about 60 degrees cut at 45 (the mockup's `chamfer`): a corner loses
/// `c` each way, or half its shorter side.
fn chamfer2(pts: &[[f32; 2]], c: f32) -> Vec<[f32; 2]> {
    let mut out = vec![pts[0]];
    for w in pts.windows(3) {
        let (a, b, d) = (w[0], w[1], w[2]);
        let (u, v) = ([b[0] - a[0], b[1] - a[1]], [d[0] - b[0], d[1] - b[1]]);
        let (lu, lv) = (u[0].hypot(u[1]), v[0].hypot(v[1]));
        if lu < 1e-6 || lv < 1e-6 || ((u[0] * v[0] + u[1] * v[1]) / (lu * lv)).abs() > 0.5 {
            out.push(b);
            continue;
        }
        let k = c.min(lu / 2.0).min(lv / 2.0);
        out.push([b[0] - u[0] / lu * k, b[1] - u[1] / lu * k]);
        out.push([b[0] + v[0] / lv * k, b[1] + v[1] / lv * k]);
    }
    out.push(pts[pts.len() - 1]);
    out
}

/// A 3D run with every corner sharper than 20 degrees cut (the mockup's `chamfer3`): the points and each segment's
/// width (a cut takes the narrower of its two).
fn chamfer3(pts: &[[f64; 3]], ws: &[f64], c: f64) -> (Vec<[f64; 3]>, Vec<f64>) {
    let (mut out, mut wo) = (vec![pts[0]], Vec::new());
    for i in 1..pts.len().saturating_sub(1) {
        let (a, b, d) = (pts[i - 1], pts[i], pts[i + 1]);
        let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        let v = [d[0] - b[0], d[1] - b[1], d[2] - b[2]];
        let (lu, lv) = (len3(u), len3(v));
        if lu < 1e-6 || lv < 1e-6 || dot3(u, v) / (lu * lv) > 0.94 {
            out.push(b);
            wo.push(ws[i - 1]);
            continue;
        }
        let k = c.min(lu / 2.0).min(lv / 2.0);
        out.push([b[0] - u[0] / lu * k, b[1] - u[1] / lu * k, b[2] - u[2] / lu * k]);
        wo.push(ws[i - 1]);
        out.push([b[0] + v[0] / lv * k, b[1] + v[1] / lv * k, b[2] + v[2] / lv * k]);
        wo.push(ws[i - 1].min(ws[i]));
    }
    out.push(pts[pts.len() - 1]);
    wo.push(ws[pts.len().saturating_sub(2)]);
    (out, wo)
}

fn len3(v: [f64; 3]) -> f64 {
    (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()
}
fn dot3(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn sub3(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn add3(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
fn scale3(a: [f64; 3], k: f64) -> [f64; 3] {
    [a[0] * k, a[1] * k, a[2] * k]
}

/// Where two segments cross, not at an end (the mockup's `cross`): the point and the share along the first.
fn cross(p: [f32; 2], q: [f32; 2], r: [f32; 2], s: [f32; 2]) -> Option<([f32; 2], f32)> {
    let d = (q[0] - p[0]) * (s[1] - r[1]) - (q[1] - p[1]) * (s[0] - r[0]);
    if d.abs() < 1e-9 {
        return None;
    }
    let t = ((r[0] - p[0]) * (s[1] - r[1]) - (r[1] - p[1]) * (s[0] - r[0])) / d;
    let u = ((r[0] - p[0]) * (q[1] - p[1]) - (r[1] - p[1]) * (q[0] - p[0])) / d;
    (t > 0.02 && t < 0.98 && u > 0.02 && u < 0.98).then(|| ([p[0] + (q[0] - p[0]) * t, p[1] + (q[1] - p[1]) * t], t))
}

/// Runs that share a route side by side (the mockup's `lanes`, design 3a): every straight piece of every ribbon grouped
/// with the pieces lying along it, each ribbon in a group in its own lane across it, then each ribbon rebuilt with a
/// fan out of its box into its lane and its corners where its two lanes meet, and cut at 45.
fn lanes(ribbons: &[(Vec<[f64; 3]>, f64)]) -> Vec<(Vec<[f64; 3]>, Vec<f64>)> {
    const LANE_M: f64 = 0.7;
    const BUNDLE_M: f64 = 5.0;
    const LANE_TOL_M: f64 = 0.2;
    const LANE_MIN_M: f64 = 0.3;
    const FAN_M: f64 = 0.8;
    const CHAMFER_M: f64 = 0.6;
    struct Seg {
        ri: usize,
        a: [f64; 3],
        b: [f64; 3],
        len: f64,
        d: [f64; 3],
        off: Option<[f64; 3]>,
        w: f64,
    }
    let mut segs: Vec<Vec<Seg>> = Vec::new();
    for (ri, (pts, w)) in ribbons.iter().enumerate() {
        let mut v = Vec::new();
        for p in pts.windows(2) {
            let d = sub3(p[1], p[0]);
            let len = len3(d);
            let d = if len > 1e-6 { scale3(d, 1.0 / len) } else { [1.0, 0.0, 0.0] };
            v.push(Seg { ri, a: p[0], b: p[1], len, d, off: None, w: *w });
        }
        segs.push(v);
    }
    let flat: Vec<(usize, usize)> = segs
        .iter()
        .enumerate()
        .flat_map(|(r, v)| v.iter().enumerate().filter(|(_, s)| s.len >= 0.05).map(move |(k, _)| (r, k)))
        .collect();
    let mut par: Vec<usize> = (0..flat.len()).collect();
    fn find(par: &mut [usize], k: usize) -> usize {
        let mut r = k;
        while par[r] != r {
            r = par[r];
        }
        let mut k = k;
        while par[k] != r {
            let n = par[k];
            par[k] = r;
            k = n;
        }
        r
    }
    for i in 0..flat.len() {
        for j in i + 1..flat.len() {
            let (a, b) = (&segs[flat[i].0][flat[i].1], &segs[flat[j].0][flat[j].1]);
            if a.ri == b.ri || dot3(a.d, b.d).abs() < 0.995 {
                continue;
            }
            let t = sub3(b.a, a.a);
            let t0 = dot3(t, a.d);
            if len3(sub3(t, scale3(a.d, t0))) > LANE_TOL_M {
                continue;
            }
            let t1 = t0 + b.len * dot3(b.d, a.d);
            if a.len.min(t0.max(t1)) - 0f64.max(t0.min(t1)) < LANE_MIN_M {
                continue;
            }
            let (ri, rj) = (find(&mut par, i), find(&mut par, j));
            par[rj] = ri;
        }
    }
    let mut groups: std::collections::BTreeMap<usize, Vec<usize>> = std::collections::BTreeMap::new();
    for k in 0..flat.len() {
        let r = find(&mut par, k);
        groups.entry(r).or_default().push(k);
    }
    for g in groups.values() {
        let mut runs: Vec<usize> = g.iter().map(|&k| flat[k].0).collect();
        runs.sort_unstable();
        runs.dedup();
        let n = runs.len();
        if n < 2 {
            continue;
        }
        let sp = LANE_M.min(BUNDLE_M / (n - 1) as f64);
        let d = segs[flat[g[0]].0][flat[g[0]].1].d;
        let across = if d[1].abs() > 0.7 {
            [1.0, 0.0, 0.0]
        } else {
            let c = [-d[2], 0.0, d[0]];
            scale3(c, 1.0 / len3(c).max(1e-9))
        };
        for &k in g {
            let s = &mut segs[flat[k].0][flat[k].1];
            let lane = runs.iter().position(|&r| r == s.ri).unwrap_or(0) as f64;
            s.off = Some(scale3(across, (lane - (n - 1) as f64 / 2.0) * sp));
            s.w = s.w.min(sp * 0.8);
        }
    }
    let mut out = Vec::with_capacity(segs.len());
    for (ri, s) in segs.iter_mut().enumerate() {
        if s.is_empty() {
            let p = ribbons[ri].0.first().copied().unwrap_or([0.0; 3]);
            out.push((vec![p, p], vec![ribbons[ri].1]));
            continue;
        }
        for i in 0..s.len() {
            if s[i].off.is_none() {
                s[i].off = Some(if i > 0 { s[i - 1].off.unwrap_or([0.0; 3]) } else { [0.0; 3] });
            }
        }
        for i in (0..s.len().saturating_sub(1)).rev() {
            if s[i].len < 0.05 && len3(s[i + 1].off.unwrap_or([0.0; 3])) > 0.0 {
                s[i].off = s[i + 1].off;
            }
        }
        let off = |k: usize| s[k].off.unwrap_or([0.0; 3]);
        let (mut pts, mut ws): (Vec<[f64; 3]>, Vec<f64>) = (vec![s[0].a], Vec::new());
        let mut push = |p: [f64; 3], w: f64| {
            pts.push(p);
            ws.push(w);
        };
        if len3(off(0)) > 0.0 {
            push(add3(add3(s[0].a, scale3(s[0].d, FAN_M.min(s[0].len * 0.3))), off(0)), s[0].w);
        }
        for j in 1..s.len() {
            let (a, b, p) = (&s[j - 1], &s[j], s[j].a);
            let (oa, ob) = (off(j - 1), off(j));
            if len3(sub3(oa, ob)) < 1e-6 {
                push(add3(p, oa), a.w);
                continue;
            }
            let bb = dot3(a.d, b.d);
            if bb.abs() > 0.995 {
                push(add3(p, oa), a.w);
                push(add3(p, ob), b.w);
                continue;
            }
            // The corner where the two lanes meet: the middle of the closest points of their lines.
            let (p1, p2) = (add3(p, oa), add3(p, ob));
            let w0 = sub3(p1, p2);
            let (dd, ee, den) = (dot3(a.d, w0), dot3(b.d, w0), 1.0 - bb * bb);
            let (t, u) = ((bb * ee - dd) / den, (ee - bb * dd) / den);
            push(scale3(add3(add3(p1, scale3(a.d, t)), add3(p2, scale3(b.d, u))), 0.5), a.w);
        }
        let l = s.len() - 1;
        if len3(off(l)) > 0.0 {
            push(add3(add3(s[l].b, scale3(s[l].d, -FAN_M.min(s[l].len * 0.3))), off(l)), s[l].w);
        }
        push(s[l].b, s[l].w);
        out.push(chamfer3(&pts, &ws, CHAMFER_M));
    }
    out
}

fn build(d: &DrillData) -> Static {
    let pd = d.power.clone();
    let board = PowerBoard::load(include_str!("../../../../data/ships/tern/power_board.json"), &pd)
        .expect("power_board.json is valid (sc-tools check-data holds it)");
    let fam = Families::load(include_str!("../../../../data/ships/tern/subsystems.json"), &pd)
        .expect("subsystems.json is valid (sc-tools check-data holds it)");
    let fam_of = fam.of_loads(&pd);
    let layout: serde_json::Value =
        serde_json::from_str(include_str!("../../../../data/ships/tern/layout.json")).expect("layout.json is JSON");
    let sys_at = |id: &str| -> [f64; 3] {
        layout["systems"]
            .as_array()
            .and_then(|a| a.iter().find(|s| s["id"] == id))
            .and_then(|s| serde_json::from_value(s["center_m"].clone()).ok())
            .unwrap_or([0.0; 3])
    };
    let f = &pd.file;
    let node_at: Vec<[f64; 3]> = f.nodes.iter().map(|n| n.center_m).collect();
    let reactor_at = sys_at(&f.reactor.system);
    let battery_at = sys_at(&f.battery.system);
    let l = &d.tern_layout;
    let node_room: Vec<Option<usize>> =
        pd.nodes.iter().map(|id| l.nodes.iter().find(|n| n.id == *id).and_then(|n| n.room)).collect();
    let file_load = |i: usize| pd.loads[i].file.and_then(|k| f.loads.get(k));
    let layout_load = |i: usize| file_load(i).and_then(|fl| l.loads.iter().find(|x| x.id == fl.id));
    let load_room = (0..pd.loads.len()).map(|i| layout_load(i).and_then(|x| x.room)).collect();
    let load_at = (0..pd.loads.len()).map(|i| layout_load(i).map(|x| x.centre.to_array())).collect();
    // The edges, in the flows' order: generators, ties, the battery's link, conduits.
    let mut edges = Vec::new();
    let tr = |id: &str| -> Vec<[f32; 2]> {
        let pts: Vec<[f32; 2]> =
            board.traces.get(id).map(|t| t.iter().map(|p| [p[0] as f32, p[1] as f32]).collect()).unwrap_or_default();
        if pts.len() < 2 {
            vec![[0.0, 0.0], [0.0, 0.0]]
        } else {
            chamfer2(&pts, board.board.chamfer as f32)
        }
    };
    let node = |id: &str| pd.node(id);
    for (k, g) in f.generators.iter().enumerate() {
        let path = g.path_m.clone().unwrap_or_else(|| vec![reactor_at, node_at[node(&g.node).unwrap_or(0)]]);
        edges.push((g.id.clone(), g.name.clone(), Kind::Gen(k), g.capacity_mw, false, [None, node(&g.node)], path));
    }
    for (k, t) in f.ties.iter().enumerate() {
        let ends = [node(&t.between[0]), node(&t.between[1])];
        let path = t.path_m.clone().unwrap_or_else(|| ends.iter().map(|e| node_at[e.unwrap_or(0)]).collect());
        edges.push((t.id.clone(), t.name.clone(), Kind::Tie(k), t.capacity_mw, false, ends, path));
    }
    let bn = node(&f.battery.node);
    let bpath = f.battery.path_m.clone().unwrap_or_else(|| vec![battery_at, node_at[bn.unwrap_or(0)]]);
    edges.push((
        BATTERY_LINK.into(),
        "Battery to the emergency bus".into(),
        Kind::Battery,
        f.battery.discharge_max_mw,
        false,
        [None, bn],
        bpath,
    ));
    for (k, c) in f.conduits.iter().enumerate() {
        let ends = [node(&c.between[0]), node(&c.between[1])];
        let spare = c.factory_closed == Some(false);
        edges.push((c.id.clone(), c.name.clone(), Kind::Conduit(k), c.capacity_mw, spare, ends, c.path_m.clone()));
    }
    // The 3D lanes: every edge, then every load's feeder and second feeder.
    let mut ribbons: Vec<(Vec<[f64; 3]>, f64)> =
        edges.iter().map(|e| (e.6.clone(), 0.2 + 0.4 * (e.3 / 40.0).clamp(0.0, 1.0))).collect();
    let mut feeders = Vec::new();
    for i in 0..pd.loads.len() {
        if let Some(fl) = file_load(i) {
            if let Some(p) = fl.feeder_m.clone().filter(|p| p.len() >= 2) {
                ribbons.push((p, 0.11));
                feeders.push((i, false));
            }
        }
    }
    for i in 0..pd.loads.len() {
        if let Some(fl) = file_load(i) {
            if let Some(p) = fl.alt_feeder_m.clone().filter(|p| p.len() >= 2) {
                ribbons.push((p, 0.09));
                feeders.push((i, true));
            }
        }
    }
    let laned = lanes(&ribbons);
    let mut edges: Vec<Edge> = edges
        .into_iter()
        .zip(&laned)
        .map(|((id, name, kind, rated, spare, ends, _), (lane, ws))| Edge {
            trace: tr(&id),
            name,
            kind,
            rated,
            spare,
            ends,
            hops: Vec::new(),
            vias: Vec::new(),
            lane: lane.clone(),
            ws: ws.clone(),
        })
        .collect();
    let ne = edges.len();
    let feeders = feeders
        .into_iter()
        .zip(&laned[ne..])
        .map(|((load, alt), (lane, ws))| Feeder { load, alt, lane: lane.clone(), ws: ws.clone() })
        .collect();
    // The hops: every crossing of two top-layer traces; the one rated lower drops under the other.
    for i in 0..ne {
        for j in i + 1..ne {
            if edges[i].spare || edges[j].spare {
                continue;
            }
            for a in 0..edges[i].trace.len() - 1 {
                for b in 0..edges[j].trace.len() - 1 {
                    let (p, q) = (edges[i].trace[a], edges[i].trace[a + 1]);
                    let (r, s) = (edges[j].trace[b], edges[j].trace[b + 1]);
                    if let Some((x, _)) = cross(p, q, r, s) {
                        let (hop, seg) = if edges[i].rated <= edges[j].rated { (i, a) } else { (j, b) };
                        let (p, q) = (edges[hop].trace[seg], edges[hop].trace[seg + 1]);
                        let l2 = (q[0] - p[0]).powi(2) + (q[1] - p[1]).powi(2);
                        let t = ((x[0] - p[0]) * (q[0] - p[0]) + (x[1] - p[1]) * (q[1] - p[1])) / l2.max(1e-9);
                        edges[hop].hops.push((seg, t));
                    }
                }
            }
        }
    }
    for e in &mut edges {
        e.hops.sort_by(|a, b| (a.0, a.1).partial_cmp(&(b.0, b.1)).unwrap_or(std::cmp::Ordering::Equal));
        for l in &board.layers {
            for y in l.y {
                let y = y as f32;
                for w in e.trace.windows(2) {
                    if (w[0][1] - y) * (w[1][1] - y) < 0.0 {
                        let t = (y - w[0][1]) / (w[1][1] - w[0][1]);
                        e.vias.push([w[0][0] + (w[1][0] - w[0][0]) * t, y]);
                    }
                }
            }
        }
    }
    let mut on: Vec<Vec<usize>> = vec![Vec::new(); pd.nodes.len()];
    for (i, ld) in pd.loads.iter().enumerate() {
        on[ld.node].push(i);
    }
    for v in &mut on {
        v.sort_by_key(|&i| (pd.loads[i].id.starts_with("lights_"), pd.loads[i].priority, i));
    }
    let rooms = l.rooms.iter().map(|r| r.brushes.iter().map(|b| (b.y[0], b.poly.clone())).collect()).collect();
    let room_lights = l
        .rooms
        .iter()
        .map(|r| {
            f.lighting
                .panels
                .iter()
                .find(|(_, rs)| rs.contains(&r.id))
                .and_then(|(panel, _)| pd.load(&format!("lights_{panel}")))
        })
        .collect();
    let emergency_lights = pd.load("emergency_lighting");
    let mut decks: Vec<(String, String, f64)> = layout["decks"]
        .as_array()
        .map(|a| {
            a.iter()
                .map(|d| {
                    (
                        d["id"].as_str().unwrap_or("").to_owned(),
                        d["name"].as_str().unwrap_or("").to_owned(),
                        d["floor_y_m"].as_f64().unwrap_or(0.0),
                    )
                })
                .collect()
        })
        .unwrap_or_default();
    decks.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
    let mut bounds = [f64::MAX, f64::MIN, f64::MAX, f64::MIN];
    for r in &l.rooms {
        for b in &r.brushes {
            for p in &b.poly {
                bounds = [bounds[0].min(p[0]), bounds[1].max(p[0]), bounds[2].min(p[1]), bounds[3].max(p[1])];
            }
        }
    }
    Static {
        pd,
        board,
        fam,
        fam_of,
        edges,
        feeders,
        on,
        node_room,
        load_room,
        node_at,
        load_at,
        reactor_at,
        battery_at,
        rooms,
        room_lights,
        emergency_lights,
        decks,
        bounds,
    }
}

/// A room's floor in 3D, by its light (the mockup's `lighting`): its own panel's lights with their power, red at red
/// alert; else the emergency lighting's, amber; else dark.
fn room_tint(s: &Static, v: &SchemView, room: usize, red: bool) -> Color32 {
    let t = &vg::style().schem.room_tint;
    let got = |i: Option<usize>| i.and_then(|i| v.supply.get(i).copied().flatten());
    let normal = s.pd.file.lighting.normal_below_ratio;
    if got(s.room_lights.get(room).copied().flatten()).is_some_and(|x| x >= normal) {
        if red {
            t.red_alert.0
        } else {
            t.normal.0
        }
    } else if got(s.emergency_lights).is_some_and(|x| x >= 0.5) {
        t.emergency.0
    } else {
        t.dark.0
    }
}

// ------------------------------------------------------------------------------------------------ the state, as drawn

/// An edge's state (the mockup's `edgeNow`): live (carrying, or energized and idle), dead, open at its box, or cut.
#[derive(Clone, Copy, PartialEq, Debug)]
enum St {
    Live,
    Dead,
    Open,
    Cut,
}

/// `a` toward `b` by `k`.
fn mix(a: Color32, b: Color32, k: f32) -> Color32 {
    let ch = |x: u8, y: u8| (f32::from(x) * (1.0 - k) + f32::from(y) * k).round() as u8;
    Color32::from_rgba_unmultiplied(ch(a.r(), b.r()), ch(a.g(), b.g()), ch(a.b(), b.b()), a.a())
}

fn red_alert(cx: &Ctx) -> bool {
    cx.v.red_alert
}

fn colour(hex: &str) -> Color32 {
    let [r, g, b] = Family::rgb(hex).unwrap_or([255, 255, 255]);
    Color32::from_rgb(r, g, b)
}

fn edge_state(s: &Static, v: &SchemView, k: usize) -> St {
    let e = &s.edges[k];
    let flow = v.flows.get(k).copied().unwrap_or(0.0);
    let ok = |n: Option<usize>| {
        n.is_none_or(|n| v.box_on.get(n).copied().unwrap_or(true) && v.node_health.get(n).copied().unwrap_or(1.0) > 0.0)
    };
    let reached = |n: Option<usize>| n.is_none_or(|n| v.reached.get(n).copied().unwrap_or(true));
    match e.kind {
        Kind::Conduit(c) => {
            if v.severed.get(c).copied().unwrap_or(false) {
                St::Cut
            } else if !v.closed.get(c).copied().unwrap_or(true) {
                St::Open
            } else if !ok(e.ends[0])
                || !ok(e.ends[1])
                || (flow.abs() <= 1e-6 && !(reached(e.ends[0]) && reached(e.ends[1])))
            {
                St::Dead
            } else {
                St::Live
            }
        }
        Kind::Tie(t) => {
            let ng = s.pd.file.generators.len();
            if !v.switches.get(ng + t).copied().unwrap_or(true) {
                St::Open
            } else if ok(e.ends[0]) && ok(e.ends[1]) {
                St::Live
            } else {
                St::Dead
            }
        }
        Kind::Gen(g) => {
            if v.switches.get(g).copied().unwrap_or(true) && (flow.abs() > 1e-4 || v.rx_up) {
                St::Live
            } else {
                St::Dead
            }
        }
        Kind::Battery => {
            if ok(e.ends[1]) {
                St::Live
            } else {
                St::Dead
            }
        }
    }
}

/// A system's supply as its light shows it: (its rim's colour, filled, its fill share 0-1).
fn supply_look(v: &SchemView, i: usize, fam: Color32) -> (Color32, bool, f64) {
    let c = &vg::style().c;
    match v.supply.get(i).copied().flatten() {
        None if !v.dropped.get(i).copied().unwrap_or(false) => (c.faint.0, false, 0.0),
        Some(x) if x >= 0.98 => (fam, true, 1.0),
        Some(x) if x >= 0.005 => (c.emergency.0, true, x),
        _ => (c.danger.0, false, 0.0),
    }
}

/// A system's damage: nominal, damaged, disabled (offline) or destroyed (damage.json's systems).
fn damage_of(s: &Static, v: &SchemView, i: usize) -> u8 {
    let g = v.integrity.get(i).copied().unwrap_or(100.0);
    let r = &s.pd.systems;
    if g <= 0.0 {
        3
    } else if g < r.disabled_below_pct {
        2
    } else if g < r.nominal_from_pct {
        1
    } else {
        0
    }
}

/// The edge an action switches, if a tap can: a conduit opened or closed, a tie's breaker.
fn edge_act(e: &Edge) -> Option<Act> {
    match e.kind {
        Kind::Conduit(k) => Some(Act::SchemRun(k)),
        Kind::Tie(k) => Some(Act::SchemTie(k)),
        _ => None,
    }
}

fn edge_tip(e: &Edge, st: St, flow: f64, damaged: bool) -> String {
    let word = match st {
        St::Live => "live",
        St::Dead => "dead",
        St::Open => "open",
        St::Cut => "severed",
    };
    let mut t = format!("{}: {:.1} of {:.0} MW, {word}", e.name, flow.abs(), e.rated);
    if e.spare {
        t += " (a spare route)";
    }
    if damaged && st != St::Cut {
        t += &format!(", damaged: at most {:.1} MW", e.rated * 0.5);
    }
    if edge_act(e).is_some() {
        t += if st == St::Open { "; tap to close it" } else { "; tap to open it" };
    } else if matches!(e.kind, Kind::Gen(_)) {
        t += if st == St::Live { "; hold to open its breaker" } else { "; hold to close its breaker" };
    }
    t
}

// ------------------------------------------------------------------------------------------------ the frame

/// Draw the schematic over the console (the look band and the panels): its view, then the status box, the 2D / 3D
/// switch and CONSOLE, the legend in 3D, and the trip's banner.
pub fn draw(cv: &Canvas, cx: &mut Ctx, v: &SchemView) {
    let c = &vg::style().c;
    let s = data();
    let [x, y, w, h] = AREA;
    cv.rect(x, y, w, h, 0.0, c.bg.0, None);
    if v.three_d {
        ship3d(cv, cx, s, v);
        legend(cv, s);
    } else {
        board2d(cv, cx, s, v);
    }
    status(cv, cx, s, v);
    switches(cv, cx, v);
    if v.trip_s > 0.0 {
        let b = cv.at(640.0, 44.0);
        b.rect(-150.0, 0.0, 300.0, 52.0, 8.0, vg::rgba(0x28060a, 0.92), Some(Pen::new(2.0, c.danger.0)));
        b.text(0.0, 26.0, 22.0, Color32::WHITE, "TRIPPED", T::mid().bold().ls(2.6));
        let msg = format!("resets in {} s", v.trip_s.ceil());
        b.text(0.0, 44.0, 13.0, vg::rgba(0xffb3ba, 1.0), &msg, T::mid());
    }
}

/// The top right: the 2D / 3D switch and CONSOLE (back to the console's panels).
fn switches(cv: &Canvas, cx: &mut Ctx, v: &SchemView) {
    let c = &vg::style().c;
    let (y, bw) = (42.0, 54.0);
    let x0 = 1280.0 - 12.0 - 118.0 - 10.0 - 2.0 * bw;
    cv.rect(x0, y, 2.0 * bw, 34.0, 6.0, c.panel.0, Some(Pen::new(1.0, c.line2.0)));
    for (k, (label, three)) in [("3D", true), ("2D", false)].into_iter().enumerate() {
        let on = v.three_d == three;
        let bx = x0 + k as f32 * bw;
        if on {
            cv.rect(bx + 2.0, y + 2.0, bw - 4.0, 30.0, 5.0, vg::rgba(0x123a5c, 1.0), None);
        }
        cv.text(
            bx + bw / 2.0,
            y + 22.0,
            15.0,
            if on { vg::rgba(0x48d6ff, 1.0) } else { c.dim.0 },
            label,
            T::mid().bold(),
        );
        let tip = if three { "The ship in 3D" } else { "The power plan, one board a deck" };
        cx.hits.add(cv, bx, y, bw, 34.0, Some(Hit::Act(Act::SchemDim(three))), Some(tip.into()));
    }
    let (bx, bw) = (1280.0 - 12.0 - 118.0, 118.0);
    cv.rect(bx, y, bw, 34.0, 6.0, c.panel.0, Some(Pen::new(1.0, c.line2.0)));
    let pen = Pen::new(2.0, c.text.0).round();
    cv.path("M4 14h6v6M20 10h-6V4M14 10l7-7M3 21l7-7", Xf::tr(bx + 12.0, y + 8.0).sc(0.75), None, Some(pen));
    cv.text(bx + 36.0, y + 22.0, 13.0, c.text.0, "CONSOLE", T::default().bold().ls(1.0));
    let tip = "Back to the Engineering console";
    cx.hits.add(cv, bx, y, bw, 34.0, Some(Hit::Act(Act::Schem(false))), Some(tip.into()));
}

/// The status box, top left: the reactor, the battery, delivered of wanted, the live runs, the systems offline.
fn status(cv: &Canvas, cx: &mut Ctx, s: &Static, v: &SchemView) {
    let c = &vg::style().c;
    let power = vg::style().schem.power.0;
    let (x, y, w) = (12.0, 42.0, 316.0);
    let offline: Vec<String> = (0..s.pd.loads.len())
        .filter(|&i| !s.pd.loads[i].id.starts_with("lights_"))
        .filter_map(|i| {
            let d = damage_of(s, v, i);
            let none = v.supply.get(i).copied().flatten().is_some_and(|x| x < 0.005)
                || v.dropped.get(i).copied() == Some(true);
            if d >= 2 {
                Some(format!("{} ({})", s.pd.loads[i].name, if d == 3 { "destroyed" } else { "disabled" }))
            } else if none {
                Some(format!("{} (no power)", s.pd.loads[i].name))
            } else {
                None
            }
        })
        .collect();
    let h = if offline.is_empty() { 62.0 } else { 90.0 };
    cv.rect(x, y, w, h, 8.0, c.panel.0, Some(Pen::new(1.0, c.line.0)));
    if red_alert(cx) {
        cv.rect(x, y - 1.0, w, 4.0, 2.0, c.danger.0, None);
    }
    let item = |cx: &mut Ctx, x0: f32, y0: f32, icon: &str, col: Color32, text: &str, tip: String| {
        cv.icon(icon, x0 + 10.0, y0, 18.0, col);
        cv.text(x0 + 24.0, y0 + 5.0, 15.0, if col == c.danger.0 { col } else { c.text.0 }, text, T::default().bold());
        cx.hits.add(cv, x0, y0 - 11.0, 96.0, 22.0, None, Some(tip));
    };
    let rx = if v.rx_up { format!("{}%", (v.throttle * 100.0).round()) } else { "OFF".into() };
    item(
        cx,
        x + 8.0,
        y + 20.0,
        "core",
        if v.rx_up { power } else { c.danger.0 },
        &rx,
        format!("Reactor: throttle {rx}"),
    );
    let bat = format!("{}%", (v.soc * 100.0).round());
    let bcol = if v.bat_out_mw > 0.05 { c.emergency.0 } else { power };
    item(cx, x + 108.0, y + 20.0, "battery", bcol, &bat, format!("Battery {bat}"));
    let mw = format!("{:.1} / {:.1} MW", v.got_mw, v.want_mw);
    let mcol = if v.got_mw < v.want_mw - 0.05 { c.danger.0 } else { power };
    item(cx, x + 196.0, y + 20.0, "bolt", mcol, &mw, "Delivered of wanted".into());
    let rcol = if v.trip_s > 0.0 {
        c.danger.0
    } else if v.live >= v.limit {
        c.emergency.0
    } else {
        c.text.0
    };
    let runs = if v.trip_s > 0.0 { "TRIPPED".to_owned() } else { format!("{} / {}", v.live, v.limit) };
    cv.icon("runs", x + 18.0, y + 46.0, 18.0, rcol);
    cv.text(x + 32.0, y + 51.0, 15.0, rcol, &runs, T::default().bold());
    cx.hits.add(
        cv,
        x + 8.0,
        y + 35.0,
        120.0,
        22.0,
        None,
        Some("Live cables of the most the grid carries: one more trips it".into()),
    );
    if !offline.is_empty() {
        let (ox, oy) = (x + 18.0, y + 74.0);
        cv.path(&format!("M{ox} {}l8 8-8 8-8-8z", oy - 8.0), Xf::ID, None, Some(Pen::new(2.0, c.danger.0).round()));
        cv.text(ox + 14.0, oy + 5.0, 15.0, c.danger.0, &format!("{} offline", offline.len()), T::default().bold());
        cx.hits.add(cv, x + 8.0, oy - 11.0, 140.0, 22.0, None, Some(format!("Offline: {}", offline.join(", "))));
    }
}

/// The 3D legend under the status box: what each mark says (the mockup's legend, its rows that fit the console).
fn legend(cv: &Canvas, s: &Static) {
    let c = &vg::style().c;
    let sc = &vg::style().schem;
    let (x, mut y, w) = (12.0, 146.0, 316.0);
    cv.rect(x, y, w, 530.0, 8.0, c.panel.0, Some(Pen::new(1.0, c.line.0)));
    y += 18.0;
    let head = |y: f32, t: &str| cv.text(x + 12.0, y, 12.0, c.dim.0, t, T::default().ls(0.7));
    let row = |y: f32, t: &str| cv.text(x + 56.0, y + 5.0, 13.0, c.text.0, t, T::default());
    head(y, "POWER BOXES");
    let rows: [(&str, Color32, u8); 4] = [
        ("On, carrying power", sc.power.0, 0),
        ("On, nothing reaches it", c.faint.0, 0),
        ("Switched off", c.dim.0, 2),
        ("Destroyed", c.danger.0, 3),
    ];
    for (k, (t, col, mode)) in rows.iter().enumerate() {
        let yy = y + 18.0 + k as f32 * 22.0;
        let fill = if *mode == 0 { *col } else { c.bg.0 };
        cv.rect(x + 26.0, yy - 6.0, 12.0, 12.0, 1.5, fill, Some(Pen::new(2.0, *col)));
        if *mode == 3 {
            cv.seg(x + 27.0, yy - 5.0, x + 37.0, yy + 5.0, Pen::new(1.6, *col));
            cv.seg(x + 37.0, yy - 5.0, x + 27.0, yy + 5.0, Pen::new(1.6, *col));
        }
        row(yy, t);
    }
    y += 18.0 + 4.0 * 22.0 + 8.0;
    head(y, "CABLES");
    let lines: [(&str, Pen); 5] = [
        ("Power flowing: wider the more it carries", Pen::new(4.0, sc.power.0).dash(8.0, 3.0, 0.0)),
        ("Open: a spare route, or switched off", Pen::new(2.5, sc.spare.0).dash(3.0, 6.0, 0.0)),
        ("Dead: nothing reaches it", Pen::new(2.5, c.faint.0).dash(3.0, 3.0, 0.0)),
        ("Severed", Pen::new(3.5, c.danger.0)),
        ("Feeder: a panel to one system", Pen::new(2.0, sc.power.0).dash(6.0, 3.0, 0.0)),
    ];
    for (k, (t, pen)) in lines.iter().enumerate() {
        let yy = y + 18.0 + k as f32 * 22.0;
        cv.seg(x + 14.0, yy, x + 48.0, yy, *pen);
        row(yy, t);
    }
    y += 18.0 + 5.0 * 22.0 + 8.0;
    head(y, "SUBSYSTEMS (each system in its colour)");
    for (k, f) in s.fam.families.iter().enumerate() {
        let (xx, yy) = (x + 26.0 + (k % 2) as f32 * 150.0, y + 18.0 + (k / 2) as f32 * 22.0);
        cv.circle(xx + 5.0, yy, 7.0, colour(&f.colour), None);
        cv.text(xx + 24.0, yy + 5.0, 13.0, c.text.0, &f.name, T::default());
    }
    y += 18.0 + 2.0 * 22.0 + 8.0;
    head(y, "SUPPLY (a system's rim)");
    let misc = s.fam.families.iter().find(|f| f.default).map_or(c.text.0, |f| colour(&f.colour));
    let sup: [(&str, Color32, bool); 3] = [
        ("Short: filled to its share", c.emergency.0, true),
        ("Dropped or cut off", c.danger.0, false),
        ("Off: nothing wanted now", c.faint.0, false),
    ];
    for (k, (t, rim, filled)) in sup.iter().enumerate() {
        let yy = y + 18.0 + k as f32 * 22.0;
        cv.circle(x + 31.0, yy, 7.0, c.bg.0, Some(Pen::new(2.5, *rim)));
        cv.circle(x + 31.0, yy, if *filled { 4.5 } else { 2.4 }, misc, None);
        row(yy, t);
    }
    y += 18.0 + 3.0 * 22.0 + 8.0;
    head(y, "DAMAGE");
    let yy = y + 18.0;
    cv.rect(x + 25.0, yy - 7.0, 14.0, 14.0, 2.0, sc.power.0, Some(Pen::new(3.0, c.danger.0)));
    row(yy, "Box or system damaged: a red rim");
    let yy = yy + 22.0;
    cv.path(&format!("M{} {}l7 7-7 7-7-7z", x + 32.0, yy - 7.0), Xf::ID, Some(c.bg.0), Some(Pen::new(2.0, c.danger.0)));
    row(yy, "System offline, crossed: destroyed");
}

// ------------------------------------------------------------------------------------------------ 2D: the board

/// The board's frame on the console: where board (0, 0) is and console points per board unit.
fn board_frame(s: &Static, v: &SchemView) -> ([f32; 4], f32, [f32; 2]) {
    let area = [LEFT + 8.0, 84.0, 1280.0 - LEFT - 16.0, 688.0 - 84.0 - 8.0];
    let (bw, bh) = (s.board.board.w as f32, s.board.board.h as f32);
    let fit = (area[2] / bw).min(area[3] / bh);
    let k = fit * v.zoom as f32;
    let o = [area[0] + (area[2] - bw * fit) / 2.0 + v.pan[0] as f32, area[1] + v.pan[1] as f32];
    (area, k, o)
}

/// The point at a share of a polyline's length, and its segment.
fn along(path: &[[f32; 2]], share: f32) -> ([f32; 2], usize) {
    let total: f32 = path.windows(2).map(|w| (w[1][0] - w[0][0]).hypot(w[1][1] - w[0][1])).sum();
    let mut left = total * share;
    for (i, w) in path.windows(2).enumerate() {
        let l = (w[1][0] - w[0][0]).hypot(w[1][1] - w[0][1]);
        if left <= l || i + 2 == path.len() {
            let t = if l > 0.0 { (left / l).min(1.0) } else { 0.0 };
            return ([w[0][0] + (w[1][0] - w[0][0]) * t, w[0][1] + (w[1][1] - w[0][1]) * t], i);
        }
        left -= l;
    }
    (path[0], 0)
}

/// A trace cut into its top-layer pieces and the bottom-layer spans of its hops (the mockup's `pieces`).
fn pieces(e: &Edge) -> Vec<(Vec<[f32; 2]>, bool)> {
    const R: f32 = 11.0;
    if e.hops.is_empty() {
        return vec![(e.trace.clone(), false)];
    }
    let (mut out, mut cur, mut h) = (Vec::new(), vec![e.trace[0]], 0);
    for i in 0..e.trace.len() - 1 {
        let (p, q) = (e.trace[i], e.trace[i + 1]);
        let len = (q[0] - p[0]).hypot(q[1] - p[1]).max(1e-6);
        while h < e.hops.len() && e.hops[h].0 == i {
            let (t, d) = (e.hops[h].1, R / len);
            let a = [p[0] + (q[0] - p[0]) * (t - d), p[1] + (q[1] - p[1]) * (t - d)];
            let b = [p[0] + (q[0] - p[0]) * (t + d), p[1] + (q[1] - p[1]) * (t + d)];
            cur.push(a);
            out.push((std::mem::take(&mut cur), false));
            out.push((vec![a, b], true));
            cur = vec![b];
            h += 1;
        }
        cur.push(q);
    }
    out.push((cur, false));
    out
}

/// A board canvas: `cv`'s painter, clipped to `area`, with board units at `k` points from `o`.
fn board_canvas(cv: &Canvas, area: [f32; 4], k: f32, o: [f32; 2]) -> Canvas {
    let clip = cv.clip(area[0], area[1], area[2], area[3]);
    Canvas::new(clip.painter.clone(), cv.p(o[0], o[1]).to_vec2(), cv.s * k)
}

fn board2d(cv: &Canvas, cx: &mut Ctx, s: &Static, v: &SchemView) {
    let c = &vg::style().c;
    let sc = &vg::style().schem;
    let pcb = &sc.pcb;
    let (area, k, o) = board_frame(s, v);
    // Under everything: a drag moves the board, the wheel zooms it.
    cx.hits.add(cv, area[0], area[1], area[2], area[3], Some(Hit::Drag(Drag::SchemPan)), None);
    supply_strip(cv, v);
    let b = board_canvas(cv, area, k, o);
    let fr = &s.board.board;
    let (bx0, bx1) = (fr.x[0] as f32, fr.x[1] as f32);
    // The boards, a deck each: the mask, mounting holes, the deck in silkscreen.
    for l in &s.board.layers {
        let (y0, y1) = (l.y[0] as f32, l.y[1] as f32);
        b.rect(bx0, y0, bx1 - bx0, y1 - y0, 14.0, pcb.mask.0, Some(Pen::new(2.0, pcb.mask_edge.0)));
        for (hx, hy) in
            [(bx0 + 14.0, y0 + 14.0), (bx1 - 14.0, y0 + 14.0), (bx0 + 14.0, y1 - 14.0), (bx1 - 14.0, y1 - 14.0)]
        {
            b.circle(hx, hy, 6.0, pcb.hole.0, Some(Pen::new(1.5, pcb.silk_dim.0)));
        }
        let name = s.decks.iter().find(|d| d.0 == l.deck).map_or(String::new(), |d| d.1.to_uppercase());
        b.text(62.0, (y0 + y1) / 2.0 + 6.0, 44.0, vg::rgba(0xffb84d, 1.0), &l.deck, T::mid().bold());
        b.text(62.0, (y0 + y1) / 2.0 + 26.0, 10.0, vg::rgba(0xa8946e, 1.0), &name, T::mid().ls(1.0));
    }
    // Between two boards: how many traces join them and how many carry power.
    let states: Vec<St> = (0..s.edges.len()).map(|k| edge_state(s, v, k)).collect();
    for w in s.board.layers.windows(2) {
        let mid = ((w[0].y[1] + w[1].y[0]) / 2.0) as f32;
        let links: Vec<usize> = (0..s.edges.len())
            .filter(|&k| s.edges[k].trace.windows(2).any(|p| (p[0][1] - mid) * (p[1][1] - mid) < 0.0))
            .collect();
        let live = links.iter().filter(|&&k| states[k] == St::Live).count();
        let col = if live > 0 { sc.power.0 } else { c.danger.0 };
        b.text(62.0, mid + 1.0, 13.0, col, &format!("{} to {}", w[0].deck, w[1].deck), T::mid().bold());
        b.text(62.0, mid + 14.0, 10.0, c.dim.0, &format!("{live} of {} live", links.len()), T::mid());
    }
    key(&b, s);
    burns(&b, s, v);
    // The pointer, on the board: the trace under it is lit.
    let ptr = cx.pointer.map(|p| [(p.x - b.o.x) / b.s, (p.y - b.o.y) / b.s]);
    let hover = ptr
        .filter(|_| cx.pointer.is_some_and(|p| cv.r(area[0], area[1], area[2], area[3]).contains(p)))
        .and_then(|p| {
            let near = (0..s.edges.len())
                .flat_map(|k| s.edges[k].trace.windows(2).map(move |w| (k, w[0], w[1])))
                .map(|(k, a, q)| (k, kit_dist(p, a, q)))
                .filter(|(_, d)| *d < 7.0f32.max(6.0 / (k_of(cv, &b))))
                .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
            near.map(|n| n.0)
        });
    // The bottom layer first (the spares), then the copper, the heavier last, the lit trace over all.
    let mut order: Vec<usize> = (0..s.edges.len()).collect();
    let bottom = |k: usize| matches!(states[k], St::Open) || (states[k] == St::Cut && s.edges[k].spare);
    order.sort_by(|&a, &bb| {
        (bottom(bb), s.edges[a].rated).partial_cmp(&(bottom(a), s.edges[bb].rated)).unwrap_or(std::cmp::Ordering::Equal)
    });
    if let Some(hk) = hover {
        order.retain(|&k| k != hk);
        order.push(hk);
    }
    for &k in &order {
        trace(&b, cx, s, v, k, states[k], bottom(k), hover == Some(k));
    }
    for n in 0..s.pd.nodes.len() {
        part(&b, cx, s, v, n);
    }
    reactor(&b, v, s);
    battery(&b, v, s);
}

/// Console points per board unit on `b` against `cv`.
fn k_of(cv: &Canvas, b: &Canvas) -> f32 {
    b.s / cv.s
}

fn kit_dist(p: [f32; 2], a: [f32; 2], b: [f32; 2]) -> f32 {
    super::kit::seg_dist(Pos2::new(p[0], p[1]), Pos2::new(a[0], a[1]), Pos2::new(b[0], b[1]))
}

/// The strip along the top: power delivered against what the loads want.
fn supply_strip(cv: &Canvas, v: &SchemView) {
    let c = &vg::style().c;
    let sc = &vg::style().schem;
    let (x0, x1, y) = (470.0, 900.0, 52.0);
    let max = v.want_mw.max(v.got_mw).max(1.0) * 1.05;
    let k = |mw: f64| x0 + (x1 - x0) * (mw / max) as f32;
    cv.rect(x0, y, x1 - x0, 14.0, 4.0, vg::rgba(0x16202c, 1.0), None);
    let rx = (v.got_mw - v.bat_out_mw.max(0.0)).max(0.0);
    cv.rect(x0, y, k(rx) - x0, 14.0, 4.0, sc.power.0, None);
    if v.bat_out_mw > 0.05 {
        cv.rect(k(rx), y, k(v.got_mw) - k(rx), 14.0, 0.0, c.emergency.0, None);
    }
    if v.want_mw > v.got_mw + 0.05 {
        cv.rect(k(v.got_mw), y, k(v.want_mw) - k(v.got_mw), 14.0, 0.0, alpha(c.danger.0, 0.45), None);
    }
    cv.text(x0 - 10.0, y + 12.0, 14.0, c.text.0, &format!("{:.1}", v.got_mw), T::end().bold());
    let col = if v.want_mw > v.got_mw + 0.05 { c.danger.0 } else { c.dim.0 };
    cv.text(x1 + 10.0, y + 12.0, 14.0, col, &format!("of {:.1} MW", v.want_mw), T::default().bold());
}

/// The subsystems' key in silkscreen, in deck C's empty aft corner (design 3e).
fn key(b: &Canvas, s: &Static) {
    let c = &vg::style().c;
    let pcb = &vg::style().schem.pcb;
    let Some(l) = s.board.layers.iter().find(|l| l.deck == "C") else { return };
    let (x0, y0) = (s.board.board.x[0] as f32 + 52.0, l.y[0] as f32 + 46.0);
    b.rect(
        x0 - 16.0,
        y0 - 24.0,
        380.0,
        132.0,
        4.0,
        Color32::TRANSPARENT,
        Some(Pen::new(1.2, pcb.silk_dim.0).dash(4.0, 3.0, 0.0)),
    );
    b.text(x0, y0, 13.0, pcb.silk.0, "SUBSYSTEMS", T::default().bold().ls(1.6));
    for (k, f) in s.fam.families.iter().enumerate() {
        let (x, y) = (x0 + (k % 2) as f32 * 180.0, y0 + 30.0 + (k / 2) as f32 * 28.0);
        b.circle(x + 7.0, y - 4.0, 7.0, colour(&f.colour), None);
        b.text(x + 22.0, y + 1.0, 13.0, pcb.silk.0, &f.name.to_uppercase(), T::default().bold().ls(1.0));
    }
    let y = y0 + 96.0;
    b.circle(x0 + 7.0, y - 4.0, 6.0, pcb.hole.0, Some(Pen::new(2.0, c.emergency.0)));
    b.circle(x0 + 7.0, y - 4.0, 2.2, pcb.silk.0, None);
    b.text(x0 + 20.0, y + 1.0, 11.0, pcb.silk_dim.0, "SHORT", T::default().bold().ls(1.0));
    b.circle(x0 + 90.0, y - 4.0, 6.0, pcb.hole.0, Some(Pen::new(2.0, c.danger.0)));
    b.circle(x0 + 90.0, y - 4.0, 2.2, pcb.silk.0, None);
    b.text(x0 + 103.0, y + 1.0, 11.0, pcb.silk_dim.0, "NO POWER", T::default().bold().ls(1.0));
    b.path(
        &format!("M{} {}l6 6-6 6-6-6z", x0 + 192.0, y - 10.0),
        Xf::ID,
        Some(pcb.hole.0),
        Some(Pen::new(1.6, c.danger.0)),
    );
    b.text(x0 + 206.0, y + 1.0, 11.0, pcb.silk_dim.0, "DAMAGED", T::default().bold().ls(1.0));
}

/// Each room's place on the board: the box round its parts.
fn room_box(s: &Static, room: usize) -> Option<[f32; 4]> {
    let mut r: Option<[f32; 4]> = None;
    let mut put = |p: &[f64; 4]| {
        let q = [
            (p[0] - p[2] / 2.0) as f32,
            (p[1] - p[3] / 2.0) as f32,
            (p[0] + p[2] / 2.0) as f32,
            (p[1] + p[3] / 2.0) as f32,
        ];
        r = Some(r.map_or(q, |r| [r[0].min(q[0]), r[1].min(q[1]), r[2].max(q[2]), r[3].max(q[3])]));
    };
    for (n, id) in s.pd.nodes.iter().enumerate() {
        if s.node_room[n] == Some(room) {
            if let Some(p) = s.board.parts.get(id) {
                put(p);
            }
        }
    }
    r
}

/// The board burnt under every room whose boxes, systems or runs are damaged (design 6.4: the engine keeps no hit
/// history, so the burn follows the damage, darker the worse): a radial fan, black at its middle.
fn burns(b: &Canvas, s: &Static, v: &SchemView) {
    let mut worst = vec![0.0f32; s.rooms.len()];
    for (n, r) in s.node_room.iter().enumerate() {
        if let Some(r) = r {
            worst[*r] = worst[*r].max(1.0 - v.node_health.get(n).copied().unwrap_or(1.0) as f32);
        }
    }
    for (i, r) in s.load_room.iter().enumerate() {
        if let Some(r) = r {
            worst[*r] = worst[*r].max(1.0 - (v.integrity.get(i).copied().unwrap_or(100.0) / 100.0) as f32);
        }
    }
    for (room, &wv) in worst.iter().enumerate() {
        if wv < 0.05 {
            continue;
        }
        let Some([x0, y0, x1, y1]) = room_box(s, room) else { continue };
        let (cxp, cyp) = ((x0 + x1) / 2.0, (y0 + y1) / 2.0);
        let g = 14.0 + 40.0 * wv;
        let (rx, ry) = ((x1 - x0) / 2.0 + g, (y1 - y0) / 2.0 + g);
        let op = (0.6 + wv * 0.4).min(1.0);
        let n = 28;
        let mut verts = vec![([cxp, cyp], Color32::from_rgba_unmultiplied(0, 0, 0, (255.0 * op) as u8))];
        let mut idx = Vec::new();
        let ring = |k: f32, col: [u8; 3], a: f32, verts: &mut Vec<([f32; 2], Color32)>| {
            for j in 0..n {
                let t = j as f32 / n as f32 * std::f32::consts::TAU;
                verts.push((
                    [cxp + rx * k * t.cos(), cyp + ry * k * t.sin()],
                    Color32::from_rgba_unmultiplied(col[0], col[1], col[2], (255.0 * a * op) as u8),
                ));
            }
        };
        ring(0.5, [0x1a, 0x0d, 0x05], 0.92, &mut verts);
        ring(0.78, [0x4a, 0x2e, 0x0c], 0.55, &mut verts);
        ring(1.0, [0x4a, 0x2e, 0x0c], 0.0, &mut verts);
        for j in 0..n as u32 {
            let nx = (j + 1) % n as u32;
            idx.extend([0, 1 + j, 1 + nx]);
            for ringi in 0..2u32 {
                let (a0, b0) = (1 + ringi * n as u32, 1 + (ringi + 1) * n as u32);
                idx.extend([a0 + j, b0 + j, b0 + nx, a0 + j, b0 + nx, a0 + nx]);
            }
        }
        b.mesh(&verts, idx);
    }
}

fn via(b: &Canvas, p: [f32; 2], col: Color32) {
    b.circle(p[0], p[1], 5.2, col, None);
    b.circle(p[0], p[1], 2.2, vg::style().schem.pcb.hole.0, None);
}

/// A pad where a trace meets a part: a small square at the end, turned with the trace.
fn pad(b: &Canvas, p: [f32; 2], q: [f32; 2], col: Color32) {
    let (vx, vy) = (q[0] - p[0], q[1] - p[1]);
    let l = vx.hypot(vy).max(1e-6);
    let (ux, uy) = (vx / l, vy / l);
    let c = [p[0] + ux * 3.0, p[1] + uy * 3.0];
    let (w, h) = (6.0, 4.0);
    b.poly(
        &[
            [c[0] - ux * w + uy * h, c[1] - uy * w - ux * h],
            [c[0] + ux * w + uy * h, c[1] + uy * w - ux * h],
            [c[0] + ux * w - uy * h, c[1] + uy * w + ux * h],
            [c[0] - ux * w - uy * h, c[1] - uy * w + ux * h],
        ],
        col,
    );
}

#[allow(clippy::too_many_arguments)]
fn trace(b: &Canvas, cx: &mut Ctx, s: &Static, v: &SchemView, k: usize, st: St, bottom: bool, hi: bool) {
    let c = &vg::style().c;
    let sc = &vg::style().schem;
    let e = &s.edges[k];
    let flow = v.flows.get(k).copied().unwrap_or(0.0);
    let damaged = matches!(e.kind, Kind::Conduit(cn) if v.damaged.get(cn).copied().unwrap_or(false));
    let col = match st {
        St::Cut => c.danger.0,
        _ if bottom => sc.pcb.bottom.0,
        St::Dead => sc.pcb.copper_off.0,
        _ => sc.power.0,
    };
    let w = if bottom { 1.8 } else { 2.4 + 4.6 * (e.rated / 40.0).clamp(0.0, 1.0) as f32 };
    let lit = |col: Color32| if hi { mix(col, Color32::WHITE, 0.35) } else { col };
    if hi {
        b.polyline(&e.trace, Pen::new(w + 10.0, alpha(Color32::WHITE, 0.42)).round());
    }
    if bottom {
        b.polyline(
            &e.trace,
            Pen::new(w + if hi { 0.8 } else { 0.0 }, alpha(lit(col), if hi { 1.0 } else { 0.8 })).dash(6.0, 5.0, 0.0),
        );
    } else {
        for (pts, under) in pieces(e) {
            if under {
                b.polyline(&pts, Pen::new(1.6, alpha(col, 0.7)).dash(2.0, 3.0, 0.0));
                via(b, pts[0], col);
                via(b, pts[pts.len() - 1], col);
                continue;
            }
            b.polyline(&pts, Pen::new(w, alpha(lit(col), if st == St::Live && !hi { 0.55 } else { 0.95 })).round());
            if st == St::Live && flow.abs() > 1e-3 {
                // Every flowing trace at one speed (design 3a): the dashes run the way the power flows.
                let off = (-(cx.v.t as f32) * FLOW_PX_S * flow.signum() as f32).rem_euclid(19.0);
                b.polyline(&pts, Pen::new(w, lit(col)).dash(12.0, 7.0, off));
            }
        }
    }
    for p in &e.vias {
        via(b, *p, col);
    }
    if !bottom {
        let n = e.trace.len();
        pad(b, e.trace[0], e.trace[1], col);
        pad(b, e.trace[n - 1], e.trace[n - 2], col);
    }
    let (mid, seg) = along(&e.trace, 0.5);
    if damaged && st != St::Cut {
        // A damaged run (half its rating left): a red hatched sleeve where the run is halfway (the snapshot does not
        // say where a hit took it).
        let (p, q) = (e.trace[seg], e.trace[seg + 1]);
        let l = (q[0] - p[0]).hypot(q[1] - p[1]).max(1e-6);
        let (ux, uy) = ((q[0] - p[0]) / l, (q[1] - p[1]) / l);
        let (a, z) = ([mid[0] - ux * 10.0, mid[1] - uy * 10.0], [mid[0] + ux * 10.0, mid[1] + uy * 10.0]);
        b.seg(a[0], a[1], z[0], z[1], Pen::new(w + 8.0, alpha(c.danger.0, 0.45)));
        for t in [-6.0f32, -2.0, 2.0, 6.0] {
            let (mx, my) = (mid[0] + ux * t, mid[1] + uy * t);
            let (nx, ny) = (-uy * (w / 2.0 + 4.0), ux * (w / 2.0 + 4.0));
            b.seg(
                mx - nx - ux * 2.0,
                my - ny - uy * 2.0,
                mx + nx + ux * 2.0,
                my + ny + uy * 2.0,
                Pen::new(1.4, c.danger.0),
            );
        }
    }
    if st == St::Cut {
        b.circle(mid[0], mid[1], 9.0, sc.pcb.mask.0, Some(Pen::new(2.0, c.danger.0)));
        b.seg(mid[0] - 5.0, mid[1] - 5.0, mid[0] + 5.0, mid[1] + 5.0, Pen::new(2.0, Color32::WHITE));
        b.seg(mid[0] + 5.0, mid[1] - 5.0, mid[0] - 5.0, mid[1] + 5.0, Pen::new(2.0, Color32::WHITE));
    }
    let tip = edge_tip(e, st, flow, damaged);
    let hit = edge_hit(s, e);
    if let Some(Hit::Hold(key)) = &hit {
        if let Some(hv) = cx.v.hold.as_ref().filter(|h| h.key == *key) {
            b.circle(mid[0], mid[1], 4.0 + 10.0 * hv.k.clamp(0.0, 1.0) as f32, alpha(c.warn.0, 0.5), None);
        }
    }
    for wseg in e.trace.windows(2) {
        cx.hits.add_seg(b, wseg[0], wseg[1], 7.0, hit.clone(), Some(tip.clone()));
    }
}

/// What a press on an edge does: a run or a tie switches on a tap; a generator's breaker is held (power-routing 3a: the
/// one-line diagram's breakers, guarded as they were, the seat's `brk:` hold).
fn edge_hit(s: &Static, e: &Edge) -> Option<Hit> {
    match e.kind {
        Kind::Gen(g) => Some(Hit::Hold(format!("brk:{}", s.pd.file.generators[g].id))),
        _ => edge_act(e).map(Hit::Tap),
    }
}

/// A part as a chip: its body, its pins, its name in silkscreen, its box's switch, its room's lamp, a light a system
/// it feeds (tap one to swap its feed); a damaged one cracked, its health in red.
fn part(b: &Canvas, cx: &mut Ctx, s: &Static, v: &SchemView, n: usize) {
    let c = &vg::style().c;
    let sc = &vg::style().schem;
    let pcb = &sc.pcb;
    let id = &s.pd.nodes[n];
    let Some(p) = s.board.parts.get(id) else { return };
    let (w, h) = (p[2] as f32, p[3] as f32);
    let (x0, y0) = (p[0] as f32 - w / 2.0, p[1] as f32 - h / 2.0);
    let (x1, y1) = (x0 + w, y0 + h);
    let on = v.box_on.get(n).copied().unwrap_or(true);
    let live = v.reached.get(n).copied().unwrap_or(true);
    let health = v.node_health.get(n).copied().unwrap_or(1.0);
    let hurt = health < 1.0 - 1e-6;
    let destroyed = health <= 0.0;
    let box_kind = s.pd.file.nodes.get(n).is_some_and(|x| x.kind == "box");
    let name = s.pd.file.nodes.get(n).map_or(id.as_str(), |x| x.name.as_str());
    let label = s.board.labels.get(id).map_or(id.as_str(), String::as_str);
    let look = if destroyed {
        c.danger.0
    } else if !on {
        c.dim.0
    } else if live {
        sc.power.0
    } else {
        c.faint.0
    };
    let stroke = if hurt {
        c.danger.0
    } else if on && live {
        pcb.silk.0
    } else {
        look
    };
    let body = if destroyed { vg::rgba(0x0b0606, 1.0) } else { pcb.body.0 };
    b.rect(
        x0,
        y0,
        w,
        h,
        if box_kind { 3.0 } else { 4.0 },
        body,
        Some(Pen::new(if !hurt && on { 1.1 } else { 2.0 }, stroke)),
    );
    if hurt {
        b.polyline(
            &[
                [x0 + w * 0.6, y0],
                [x0 + w * 0.6 - 6.0, y0 + h * 0.35],
                [x0 + w * 0.6 + 2.0, y0 + h * 0.55],
                [x0 + w * 0.6 - 5.0, y1],
            ],
            Pen::new(1.2, alpha(c.danger.0, 0.8)),
        );
        b.text(x1 - 5.0, y1 - 5.0, 10.0, c.danger.0, &format!("{}%", (health * 100.0).round()), T::end().bold());
    }
    b.circle(x0 + 6.0, y0 + 6.0, 1.8, pcb.silk_dim.0, None);
    let mut x = x0 + 10.0;
    while x < x1 - 6.0 {
        b.rect(x, y0 - 3.0, 4.0, 3.0, 0.0, pcb.silk_dim.0, None);
        b.rect(x, y1, 4.0, 3.0, 0.0, pcb.silk_dim.0, None);
        x += 10.0;
    }
    b.text(
        x0 + 24.0,
        y0 + 15.0,
        12.0,
        if on && live { pcb.silk.0 } else { pcb.silk_dim.0 },
        label,
        T::default().bold().ls(0.6),
    );
    let state = format!(
        "{name}: {}{}{}",
        if on { "on" } else { "switched off" },
        if hurt {
            format!(", {} ({}%)", if destroyed { "destroyed" } else { "damaged" }, (health * 100.0).round())
        } else {
            String::new()
        },
        if on {
            if live {
                ", live"
            } else {
                ", nothing reaches it"
            }
        } else {
            ""
        }
    );
    cx.hits.add(b, x0, y0, w, h, None, Some(state));
    // The box's own switch: filled on, hollow off, crossed destroyed.
    let (sx, sy) = (x0 + 12.0, y0 + 11.0);
    let filled = on && !destroyed;
    b.rect(sx - 5.0, sy - 5.0, 10.0, 10.0, 1.5, if filled { look } else { pcb.hole.0 }, Some(Pen::new(1.6, look)));
    if destroyed {
        b.seg(sx - 4.0, sy - 4.0, sx + 4.0, sy + 4.0, Pen::new(1.6, c.danger.0));
        b.seg(sx + 4.0, sy - 4.0, sx - 4.0, sy + 4.0, Pen::new(1.6, c.danger.0));
    }
    let tip = format!(
        "{name}: {}, tap to switch it {}",
        if on { "on" } else { "switched off" },
        if on { "off" } else { "on" }
    );
    cx.hits.add(b, sx - 10.0, sy - 10.0, 20.0, 20.0, Some(Hit::Tap(Act::SchemBox(n))), Some(tip));
    // The room's lamp: lit by its box (red at red alert), else the emergency bus's light.
    let lamp = if on && live {
        if red_alert(cx) {
            c.danger.0
        } else {
            vg::rgba(0xa9bccf, 1.0)
        }
    } else {
        vg::rgba(0xe3922e, 1.0)
    };
    b.circle(x1 - 8.0, y0 + 9.0, 3.6, lamp, Some(Pen::new(1.0, lamp)));
    // A light a system.
    for (j, &i) in s.on[n].iter().enumerate() {
        let ld = &s.pd.loads[i];
        let (lx, ly) = (x0 + 12.0 + j as f32 * 10.0, y1 - 9.0);
        if lx > x1 - 4.0 {
            break;
        }
        let f = &s.fam.families[s.fam_of[i]];
        let fam = colour(&f.colour);
        let (rim, filled, _) = supply_look(v, i, fam);
        let dmg = damage_of(s, v, i);
        if dmg >= 2 {
            b.path(
                &format!("M{lx} {}l4.6 4.6-4.6 4.6-4.6-4.6z", ly - 4.6),
                Xf::ID,
                Some(pcb.hole.0),
                Some(Pen::new(1.4, c.danger.0)),
            );
            if dmg == 3 {
                b.seg(lx - 2.2, ly - 2.2, lx + 2.2, ly + 2.2, Pen::new(1.2, c.danger.0));
                b.seg(lx + 2.2, ly - 2.2, lx - 2.2, ly + 2.2, Pen::new(1.2, c.danger.0));
            }
        } else {
            let ring = if dmg == 1 { c.danger.0 } else { rim };
            let wr = if dmg == 1 || ring != fam { 1.8 } else { 1.2 };
            b.circle(lx, ly, 3.4, if filled { fam } else { pcb.hole.0 }, Some(Pen::new(wr, ring)));
            if !filled {
                b.circle(lx, ly, 1.3, fam, None);
            }
        }
        let alt = v.on_alt.get(i).copied().unwrap_or(false);
        if alt {
            b.circle(lx, ly, 5.4, Color32::TRANSPARENT, Some(Pen::new(1.2, pcb.bottom.0)));
        }
        let other = ld.alt.map(|a| if alt { ld.node } else { a });
        let word = match dmg {
            0 => "nominal".to_owned(),
            1 => "damaged".to_owned(),
            2 => "disabled: offline".to_owned(),
            _ => "destroyed: offline".to_owned(),
        };
        let sup = match v.supply.get(i).copied().flatten() {
            None => "off".to_owned(),
            Some(x) => format!("{}% of what it wants", (x * 100.0).round()),
        };
        let from = s
            .board
            .labels
            .get(&s.pd.nodes[if alt { ld.alt.unwrap_or(ld.node) } else { ld.node }])
            .cloned()
            .unwrap_or_default();
        let mut tip = format!("{} ({}): {sup}, fed from {from}, {word}", ld.name, f.name);
        if let Some(o) = other {
            tip += &format!("; tap to swap to {}", s.board.labels.get(&s.pd.nodes[o]).cloned().unwrap_or_default());
        }
        let hit = ld.alt.map(|_| Hit::Tap(Act::SchemFeed(i)));
        cx.hits.add(b, lx - 5.0, ly - 5.0, 10.0, 10.0, hit, Some(tip));
    }
}

fn reactor(b: &Canvas, v: &SchemView, s: &Static) {
    let c = &vg::style().c;
    let sc = &vg::style().schem;
    let Some(p) = s.board.parts.get("reactor") else { return };
    let (x, y, r) = (p[0] as f32, p[1] as f32, p[2] as f32 / 2.0);
    let col = if v.rx_up { sc.power.0 } else { c.danger.0 };
    b.circle(x, y, r, sc.pcb.body.0, Some(Pen::new(2.4, col)));
    let arc = |a0: f32, a1: f32| -> Vec<[f32; 2]> {
        (0..=24)
            .map(|k| {
                let a = (a0 + (a1 - a0) * k as f32 / 24.0).to_radians();
                [x + (r - 9.0) * a.cos(), y + (r - 9.0) * a.sin()]
            })
            .collect()
    };
    b.polyline(&arc(-220.0, 40.0), Pen::new(7.0, vg::rgba(0x1c2734, 1.0)).round());
    if v.rx_up {
        let k = (v.throttle / 1.2).clamp(0.0, 1.0) as f32;
        b.polyline(&arc(-220.0, -220.0 + 260.0 * k), Pen::new(7.0, col).round());
    }
    let t = if v.rx_up { format!("{}%", (v.throttle * 100.0).round()) } else { "OFF".into() };
    b.text(x, y + 5.0, 15.0, if v.rx_up { c.text.0 } else { col }, &t, T::mid().bold());
    b.text(x, y + 26.0, 8.5, sc.pcb.silk.0, "REACTOR", T::mid().bold().ls(1.0));
}

fn battery(b: &Canvas, v: &SchemView, s: &Static) {
    let c = &vg::style().c;
    let sc = &vg::style().schem;
    let Some(p) = s.board.parts.get("battery") else { return };
    let (w, h) = (p[2] as f32, p[3] as f32);
    let (x0, y0) = (p[0] as f32 - w / 2.0, p[1] as f32 - h / 2.0);
    let out = v.bat_out_mw > 0.05;
    let col = if out { c.emergency.0 } else { sc.power.0 };
    b.rect(x0, y0, w, h, 4.0, sc.pcb.body.0, Some(Pen::new(2.0, col)));
    let cw = (w - 16.0) / 5.0;
    for i in 0..5 {
        let fill = if (i as f64) / 5.0 < v.soc { col } else { sc.pcb.hole.0 };
        b.rect(x0 + 8.0 + i as f32 * cw, y0 + 8.0, cw - 3.0, h - 16.0, 1.5, fill, None);
    }
    b.text(
        p[0] as f32,
        y0 + h + 13.0,
        11.0,
        sc.pcb.silk.0,
        &format!("BATTERY {}%", (v.soc * 100.0).round()),
        T::mid().bold().ls(1.2),
    );
}

// ------------------------------------------------------------------------------------------------ 3D: the ship

/// The 3D view's projection: the camera round the ship's middle, the decks pulled apart.
struct View3 {
    eye: [f64; 3],
    f: [f64; 3],
    r: [f64; 3],
    u: [f64; 3],
    focal: f64,
    centre: [f32; 2],
    spread: f64,
    floors: Vec<f64>,
}

impl View3 {
    fn new(s: &Static, v: &SchemView) -> Self {
        let spread = 5.0;
        let floors: Vec<f64> = s.decks.iter().map(|d| d.2).collect();
        let mut me = View3 {
            eye: [0.0; 3],
            f: [0.0; 3],
            r: [0.0; 3],
            u: [0.0; 3],
            focal: 1.0,
            centre: [0.0; 2],
            spread,
            floors,
        };
        let mid_y = me.y(s.decks.iter().map(|d| d.2).sum::<f64>() / s.decks.len().max(1) as f64 + 1.5);
        let target = [0.0, mid_y, (s.bounds[2] + s.bounds[3]) / 2.0];
        let (yaw, pitch) = (v.cam[0].to_radians(), v.cam[1].to_radians());
        let dir = [pitch.cos() * yaw.sin(), pitch.sin(), pitch.cos() * yaw.cos()];
        me.eye = add3(target, scale3(dir, v.cam[2]));
        me.f = scale3(dir, -1.0);
        let r = [me.f[2], 0.0, -me.f[0]];
        let rl = len3(r).max(1e-9);
        me.r = scale3(r, -1.0 / rl);
        me.u = [
            me.r[1] * me.f[2] - me.r[2] * me.f[1],
            me.r[2] * me.f[0] - me.r[0] * me.f[2],
            me.r[0] * me.f[1] - me.r[1] * me.f[0],
        ];
        me.focal = (656.0 / 2.0) / (22.5f64).to_radians().tan();
        me.centre = [(LEFT + 1280.0) / 2.0, 32.0 + 656.0 / 2.0];
        me
    }
    /// A ship height where the view draws it: each deck keeps its own heights, only the gaps between decks stretch.
    fn y(&self, y: f64) -> f64 {
        let f = self.floors.iter().copied().find(|f| *f <= y + 0.3).unwrap_or(*self.floors.last().unwrap_or(&0.0));
        f * self.spread + (y - f)
    }
    /// A ship point on the console, and its depth (none behind the camera).
    fn p(&self, p: [f64; 3]) -> Option<([f32; 2], f64)> {
        let q = sub3([p[0], self.y(p[1]), p[2]], self.eye);
        let z = dot3(q, self.f);
        if z < 0.5 {
            return None;
        }
        let (x, y) = (dot3(q, self.r), dot3(q, self.u));
        Some(([self.centre[0] + (x * self.focal / z) as f32, self.centre[1] - (y * self.focal / z) as f32], z))
    }
    fn px_per_m(&self, z: f64) -> f32 {
        (self.focal / z.max(0.5)) as f32
    }
}

fn ship3d(cv: &Canvas, cx: &mut Ctx, s: &Static, v: &SchemView) {
    let c = &vg::style().c;
    let sc = &vg::style().schem;
    let [ax, ay, aw, ah] = AREA;
    cx.hits.add(cv, ax, ay, aw, ah, Some(Hit::Drag(Drag::SchemCam)), None);
    let view = View3::new(s, v);
    let g = cv.clip(ax, ay, aw, ah);
    // The rooms' floors, the lowest deck first (the farthest from a camera above), and their walls.
    let mut floors: Vec<(f64, Vec<[f32; 2]>, Color32)> = Vec::new();
    for (ri, r) in s.rooms.iter().enumerate() {
        let tint = room_tint(s, v, ri, cx.v.red_alert);
        for (y, poly) in r {
            let pts: Option<Vec<([f32; 2], f64)>> = poly.iter().map(|q| view.p([q[0], *y, q[1]])).collect();
            if let Some(pts) = pts {
                floors.push((*y, pts.iter().map(|p| p.0).collect(), tint));
            }
        }
    }
    floors.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    for (_, pts, tint) in &floors {
        g.poly(pts, alpha(*tint, 0.55));
    }
    for (_, pts, _) in &floors {
        let mut p = pts.clone();
        p.push(pts[0]);
        g.polyline(&p, Pen::new(1.0, alpha(vg::rgba(0x7b8796, 1.0), 0.45)));
    }
    // Every run and feeder, projected: its points, its width in points, its colour and how it is dashed.
    struct Line {
        pts: Vec<[f32; 2]>,
        w: f32,
        col: Color32,
        st: St,
        flow: f64,
        hit: Option<Hit>,
        tip: String,
    }
    let mut lines: Vec<Line> = Vec::new();
    let project = |lane: &[[f64; 3]], ws: &[f64]| -> Option<(Vec<[f32; 2]>, f32)> {
        let pts: Option<Vec<([f32; 2], f64)>> = lane.iter().map(|p| view.p(*p)).collect();
        let pts = pts?;
        let z = pts.iter().map(|p| p.1).sum::<f64>() / pts.len().max(1) as f64;
        let wm = ws.iter().copied().fold(0.0, f64::max);
        Some((pts.into_iter().map(|p| p.0).collect(), (wm as f32 * view.px_per_m(z)).clamp(1.4, 9.0)))
    };
    for (k, e) in s.edges.iter().enumerate() {
        let Some((pts, w)) = project(&e.lane, &e.ws) else { continue };
        let st = edge_state(s, v, k);
        let col = match st {
            St::Cut => c.danger.0,
            St::Open => sc.spare.0,
            St::Dead => c.faint.0,
            St::Live => sc.power.0,
        };
        let flow = v.flows.get(k).copied().unwrap_or(0.0);
        let damaged = matches!(e.kind, Kind::Conduit(cn) if v.damaged.get(cn).copied().unwrap_or(false));
        lines.push(Line { pts, w, col, st, flow, hit: edge_hit(s, e), tip: edge_tip(e, st, flow, damaged) });
    }
    for fd in &s.feeders {
        let Some((pts, w)) = project(&fd.lane, &fd.ws) else { continue };
        let i = fd.load;
        let ld = &s.pd.loads[i];
        let on_alt = v.on_alt.get(i).copied().unwrap_or(false);
        let node = if fd.alt { ld.alt.unwrap_or(ld.node) } else { ld.node };
        let fam = colour(&s.fam.families[s.fam_of[i]].colour);
        let (rim, _, _) = supply_look(v, i, fam);
        let wants = v.supply.get(i).copied().flatten();
        let st = if fd.alt != on_alt {
            St::Open
        } else if wants.is_some_and(|x| x > 0.005)
            || (v.box_on.get(node).copied().unwrap_or(true)
                && v.reached.get(node).copied().unwrap_or(true)
                && wants.is_none())
        {
            St::Live
        } else {
            St::Dead
        };
        let col = match st {
            St::Open => sc.spare.0,
            St::Dead => c.faint.0,
            // A feeder is electricity, yellow; only its system takes its family's colour (design 3e).
            _ if rim == fam || wants.is_none() => sc.power.0,
            _ => rim,
        };
        let from = s.board.labels.get(&s.pd.nodes[node]).cloned().unwrap_or_default();
        let tip =
            format!("{}: feeder from {from}{}", ld.name, if ld.alt.is_some() { "; tap to swap its feed" } else { "" });
        let flow = if st == St::Live && wants.is_some_and(|x| x > 0.005) { 1.0 } else { 0.0 };
        lines.push(Line { pts, w: w.min(2.4), col, st, flow, hit: ld.alt.map(|_| Hit::Tap(Act::SchemFeed(i))), tip });
    }
    // The line under the pointer is lit.
    let ptr = cx.pointer.map(|p| [(p.x - cv.o.x) / cv.s, (p.y - cv.o.y) / cv.s]);
    let hover = ptr.and_then(|p| {
        lines
            .iter()
            .enumerate()
            .flat_map(|(k, l)| l.pts.windows(2).map(move |w| (k, kit_dist(p, w[0], w[1]))))
            .filter(|(_, d)| *d < 8.0)
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|x| x.0)
    });
    // Outlines first: dark under every working line (white and wider under the lit one), then the lines.
    for (k, l) in lines.iter().enumerate() {
        let lit = hover == Some(k);
        if l.st == St::Live || lit {
            let (w, col) = if lit {
                (l.w + 2.0 * (1.3 + 2.8), vg::rgba(0xeef3f8, 1.0))
            } else {
                (l.w + 2.6, vg::rgba(0x05080b, 1.0))
            };
            g.polyline(&l.pts, Pen::new(w, col).round());
        }
    }
    let t = cx.v.t as f32;
    for (k, l) in lines.iter().enumerate() {
        let lit = hover == Some(k);
        let col = if lit { mix(l.col, Color32::WHITE, 0.25) } else { l.col };
        match l.st {
            St::Live => {
                if l.flow.abs() > 1e-3 {
                    g.polyline(&l.pts, Pen::new(l.w, mix(col, Color32::BLACK, 0.38)).round());
                    let off = (-t * FLOW_PX_S * l.flow.signum() as f32).rem_euclid(22.0);
                    g.polyline(&l.pts, Pen::new(l.w, col).dash(13.0, 9.0, off));
                } else {
                    g.polyline(&l.pts, Pen::new(l.w, col).round());
                }
            }
            St::Dead => g.polyline(&l.pts, Pen::new(l.w, col).dash(4.0, 4.0, 0.0)),
            St::Open => g.polyline(&l.pts, Pen::new(l.w, col).dash(5.0, 9.0, 0.0)),
            St::Cut => g.polyline(&l.pts, Pen::new(l.w, col).round()),
        }
        for w in l.pts.windows(2) {
            cx.hits.add_seg(cv, w[0], w[1], 6.0, l.hit.clone(), Some(l.tip.clone()));
        }
    }
    // The cuts and the damaged runs: a mark halfway along each (the snapshot does not say where a hit took it).
    for (k, e) in s.edges.iter().enumerate() {
        let Kind::Conduit(cn) = e.kind else { continue };
        let (cut, dmg) = (v.severed.get(cn).copied().unwrap_or(false), v.damaged.get(cn).copied().unwrap_or(false));
        if !(cut || dmg) {
            continue;
        }
        let mid = e.lane[e.lane.len() / 2];
        let Some((p, z)) = view.p(mid) else { continue };
        let r = (0.6 * view.px_per_m(z)).clamp(4.0, 10.0);
        if cut {
            g.rect(p[0] - r, p[1] - r, 2.0 * r, 2.0 * r, 0.0, c.bg.0, Some(Pen::new(2.0, c.danger.0)));
            g.seg(p[0] - r * 0.7, p[1] - r * 0.7, p[0] + r * 0.7, p[1] + r * 0.7, Pen::new(2.0, c.danger.0));
            g.seg(p[0] + r * 0.7, p[1] - r * 0.7, p[0] - r * 0.7, p[1] + r * 0.7, Pen::new(2.0, c.danger.0));
        } else {
            g.path(
                &format!("M{} {}l{r} {r}-{r} {r}-{r}-{r}z", p[0], p[1] - r),
                Xf::ID,
                Some(c.bg.0),
                Some(Pen::new(2.0, c.danger.0)),
            );
        }
        let _ = k;
    }
    // The markers: the reactor, the battery, every node, every system.
    let mark =
        |at: [f64; 3], size_m: f64| view.p(at).map(|(p, z)| (p, (size_m as f32 * view.px_per_m(z)).clamp(5.0, 34.0)));
    if let Some((p, r)) = mark(s.reactor_at, 3.2) {
        let col = if v.rx_up { sc.power.0 } else { c.danger.0 };
        g.circle(p[0], p[1], r / 2.0, c.bg.0, Some(Pen::new(2.5, col)));
        if v.rx_up {
            g.circle(p[0], p[1], r / 2.0 - 3.0, alpha(col, (v.throttle as f32).clamp(0.2, 1.0)), None);
        }
    }
    if let Some((p, r)) = mark(s.battery_at, 1.6) {
        let col = if v.bat_out_mw > 0.05 { c.emergency.0 } else { sc.power.0 };
        g.rect(p[0] - r / 2.0, p[1] - r / 2.0, r, r, 2.0, col, None);
    }
    for (n, at) in s.node_at.iter().enumerate() {
        let kind = s.pd.file.nodes.get(n).map_or("box", |x| x.kind.as_str());
        let size = match kind {
            "switchboard" => 1.6,
            "emergency_bus" => 1.4,
            "box" => 0.95,
            _ => 1.15,
        };
        let Some((p, r)) = mark(*at, size) else { continue };
        let on = v.box_on.get(n).copied().unwrap_or(true);
        let live = v.reached.get(n).copied().unwrap_or(true);
        let health = v.node_health.get(n).copied().unwrap_or(1.0);
        let col = if health <= 0.0 {
            c.danger.0
        } else if !on {
            c.dim.0
        } else if live {
            sc.power.0
        } else {
            c.faint.0
        };
        let rim = if health < 1.0 - 1e-6 { c.danger.0 } else { col };
        let fill = if on && health > 0.0 { col } else { c.bg.0 };
        g.rect(
            p[0] - r / 2.0,
            p[1] - r / 2.0,
            r,
            r,
            1.5,
            fill,
            Some(Pen::new(if rim == col { 1.5 } else { 3.0 }, rim)),
        );
        if health <= 0.0 {
            g.seg(p[0] - r / 2.0, p[1] - r / 2.0, p[0] + r / 2.0, p[1] + r / 2.0, Pen::new(1.6, c.danger.0));
            g.seg(p[0] + r / 2.0, p[1] - r / 2.0, p[0] - r / 2.0, p[1] + r / 2.0, Pen::new(1.6, c.danger.0));
        }
        let name = s.pd.file.nodes.get(n).map_or("", |x| x.name.as_str());
        let tip = format!(
            "{name}: {}, tap to switch it {}",
            if on { "on" } else { "switched off" },
            if on { "off" } else { "on" }
        );
        let hr = r.max(14.0);
        cx.hits.add(cv, p[0] - hr / 2.0, p[1] - hr / 2.0, hr, hr, Some(Hit::Tap(Act::SchemBox(n))), Some(tip));
        if kind != "box" || v.cam[2] < 55.0 {
            let label = s.board.labels.get(&s.pd.nodes[n]).cloned().unwrap_or_default();
            g.text(p[0], p[1] + r / 2.0 + 12.0, 11.0, vg::rgba(0xffe7b0, 1.0), &label, T::mid().bold().ls(0.5));
        }
    }
    for (i, at) in s.load_at.iter().enumerate() {
        let Some(at) = at else { continue };
        if s.pd.loads[i].id.starts_with("lights_") {
            continue;
        }
        let Some((p, r)) = mark(*at, 0.95) else { continue };
        let f = &s.fam.families[s.fam_of[i]];
        let fam = colour(&f.colour);
        let (rim, filled, share) = supply_look(v, i, fam);
        let dmg = damage_of(s, v, i);
        let rr = r / 2.0;
        if dmg >= 2 {
            g.path(
                &format!("M{} {}l{rr} {rr}-{rr} {rr}-{rr}-{rr}z", p[0], p[1] - rr),
                Xf::ID,
                Some(c.bg.0),
                Some(Pen::new(2.0, c.danger.0)),
            );
            if dmg == 3 {
                g.seg(p[0] - rr / 2.0, p[1] - rr / 2.0, p[0] + rr / 2.0, p[1] + rr / 2.0, Pen::new(1.6, c.danger.0));
                g.seg(p[0] + rr / 2.0, p[1] - rr / 2.0, p[0] - rr / 2.0, p[1] + rr / 2.0, Pen::new(1.6, c.danger.0));
            }
        } else {
            g.circle(
                p[0],
                p[1],
                rr,
                if filled && share >= 0.98 { fam } else { c.bg.0 },
                Some(Pen::new(if dmg == 1 { 3.0 } else { 2.0 }, if dmg == 1 { c.danger.0 } else { rim })),
            );
            if filled && share < 0.98 {
                g.circle(p[0], p[1], rr * share.sqrt() as f32 * 0.8, fam, None);
            }
        }
        let ld = &s.pd.loads[i];
        let tip = format!("{} ({}){}", ld.name, f.name, if ld.alt.is_some() { ": tap to swap its feed" } else { "" });
        let hr = r.max(12.0);
        cx.hits.add(
            cv,
            p[0] - hr / 2.0,
            p[1] - hr / 2.0,
            hr,
            hr,
            ld.alt.map(|_| Hit::Tap(Act::SchemFeed(i))),
            Some(tip),
        );
    }
    // Each deck's letter, past the bow (the stern is under the legend).
    for d in &s.decks {
        if let Some((p, _)) = view.p([0.0, d.2 + 0.1, s.bounds[3] + 8.0]) {
            g.rect(p[0] - 30.0, p[1] - 22.0, 60.0, 44.0, 6.0, alpha(c.panel.0, 0.85), Some(Pen::new(1.0, c.line2.0)));
            g.text(p[0], p[1] + 4.0, 22.0, vg::rgba(0xffb84d, 1.0), &d.0, T::mid().bold());
            g.text(p[0], p[1] + 17.0, 9.0, vg::rgba(0xa8946e, 1.0), &d.1.to_uppercase(), T::mid().ls(0.8));
        }
    }
}
