//! The reactor's coolant pipes (reactor-cooling design 1, 2 and 4), from `docs/mockups/repairs/pipes.js`.
//!
//! A damaged loop segment. A round is the whole job on a new segment, three phases in order:
//!
//! - **isolate**: the engineering pipe run as a one-line plan, the core at the top, the chiller at the bottom, the hot
//!   leg down the right (red, chevrons), the cold leg up the left (blue, dots), each with its valves and, round its
//!   middle run, a bypass with its own valve, shut in normal running. Open the leg's bypass, then shut the valve on
//!   each side of the cracked segment (tap a valve; tap again to open it). Shutting the last open way from the core to
//!   the chiller (a boxed main valve, or the line with the bypass still shut) is the fumble "Core starved: the
//!   blanket heats", and that valve springs back open.
//! - **rebuild**: the segment's run as a tile grid with two pairs of ends: hot left to right, cold top to bottom. Tap a
//!   tile to turn it a quarter. Join each pair with no open end and without the legs meeting; crossover tiles let one
//!   leg pass over the other. Cracked tiles drip: drag a new piece from the tray onto each. Then FILL: coolant enters
//!   at both inlets at once and fills the connected tiles tile by tile (the kit's [`flood`]). The first fault it
//!   reaches (an open end, a cracked tile carrying, the legs meeting) is the fumble, and the coolant drains back.
//! - **bleed**: the new segment between its two valves, over a high point with a bleed valve on top. Open the
//!   downstream valve first, then the upstream one, then hold the bleed valve until the gauge's needle settles.
//!   Upstream first is the fumble "Water hammer: the joint jumps".
//!
//! Each level turns up the rebuild: a 5 x 4 grid at level 1, 7 x 5 from level 2, with more crossings and cracked
//! tiles. A disabled segment's first step fits the new spool from the stores rack, the one for the right leg.
//!
//! Colour is never alone: hot pipe carries chevrons, cold pipe dots, a shut valve is solid with a bar across.
//! Keys: arrows pick (a valve, a tile), Space works it (hold it on the bleed valve), Q and E pick a tray piece (Space
//! on a cracked tile fits it), Enter fills.

use std::f32::consts::{PI, TAU};

use egui::{Color32, Key};

use crate::kit::{flood, nearest, poly_at, Ctx, Dice, Game, Input, BAR_H, TOUCH_R, W};
use crate::pen::{c, hex, rgba, Align, Pen, PipeStyle};

/// In the engine.
pub const PORTED: bool = true;

/// A fresh game.
pub fn new() -> Box<dyn Game> {
    Box::new(Pipes::default())
}

const STARVED: &str = "Core starved: the blanket heats";
const HAMMER: &str = "Water hammer: the joint jumps";
const CRACKED: &str = "Cracked pipe: a scalding spray";
const MIXED: &str = "Legs mixed: hot into cold";
const OPEN_END: &str = "Open joint: a scalding spray";
const HOT: Color32 = hex(0xff6a4d);
const COLD: Color32 = hex(0x4fa8f7);
/// A valve's tap reach, px.
const REACH: f32 = TOUCH_R + 8.0;
/// Coolant advances this many tiles a second once FILL is pressed.
const FLOW_TILES_S: f32 = 4.0;
/// Seconds the coolant keeps running after it meets a fault, before it drains.
const FAULT_SHOW_S: f32 = 1.6;
/// Seconds the spilt coolant takes to drain back out of the run.
const DRAIN_S: f32 = 0.8;
/// The bore's share of a pipe's width here: wide enough to read the leg's colour and its pattern in it.
const BORE: f32 = 0.55;

/// A leg of the loop.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Leg {
    #[default]
    Hot,
    Cold,
}

impl Leg {
    fn other(self) -> Leg {
        match self {
            Leg::Hot => Leg::Cold,
            Leg::Cold => Leg::Hot,
        }
    }
    fn col(self) -> Color32 {
        match self {
            Leg::Hot => HOT,
            Leg::Cold => COLD,
        }
    }
    fn i(self) -> usize {
        usize::from(self == Leg::Cold)
    }
}

/// The damaged segment this round: its leg, and which middle segment (2 or 3).
#[derive(Clone, Copy, Debug, Default)]
struct Job {
    leg: Leg,
    seg: u32,
}

// ================================================================ shared drawing

/// A pipe along a polyline, the kit's: the bore dry when `dim` or legless, else holding the leg's coolant `fill` of
/// the way, with its pattern (chevrons hot, dots cold) every 30 px moving with the flow.
#[allow(clippy::too_many_arguments)]
fn pipe(g: &Pen, pts: &[[f32; 2]], leg: Option<Leg>, w: f32, flow: f32, t: f32, dim: bool, fill: f32) {
    let wet = if dim || fill <= 0.0 { None } else { leg };
    let style = PipeStyle { w, bore: BORE, fluid: wet.map(Leg::col), ..PipeStyle::default() };
    g.pipe(&[(pts.to_vec(), fill)], &style);
    let Some(leg) = wet else { return };
    let segs: Vec<f32> = pts.windows(2).map(|p| (p[1][0] - p[0][0]).hypot(p[1][1] - p[0][1])).collect();
    let total: f32 = segs.iter().sum();
    let gap = 30.0;
    let mut d = (t * flow * 60.0).rem_euclid(gap);
    while d < total * fill {
        let (mut k, mut acc) = (0, 0.0);
        while k < segs.len() - 1 && acc + segs[k] < d {
            acc += segs[k];
            k += 1;
        }
        let u = (d - acc) / segs[k].max(1e-6);
        let ([x0, y0], [x1, y1]) = (pts[k], pts[k + 1]);
        let p = g.translate(x0 + (x1 - x0) * u, y0 + (y1 - y0) * u).rotate((y1 - y0).atan2(x1 - x0));
        if leg == Leg::Hot {
            p.path_round(&[[-5.0, -7.0], [4.0, 0.0], [-5.0, 7.0]], 3.0, hex(0xfff3e0));
        } else {
            p.disc(0.0, 0.0, 3.5, hex(0xe8f6ff));
        }
        d += gap;
    }
}

/// A one-line valve: a bowtie on the pipe, hollow when open, solid with a bar across when shut.
#[allow(clippy::too_many_arguments)]
fn bowtie(g: &Pen, x: f32, y: f32, vertical: bool, shut: bool, main: bool, r: f32, focus: bool) {
    let p = g.translate(x, y);
    let p = if vertical { p.rotate(PI / 2.0) } else { p };
    if main {
        p.round(
            -r - 10.0,
            -r - 10.0,
            2.0 * r + 20.0,
            2.0 * r + 20.0,
            6.0,
            Some(hex(0x0b1119)),
            Some((2.5, hex(0x8796aa))),
        );
    }
    let fill = if shut { hex(0xc0392b) } else { hex(0x0b1119) };
    p.poly(&[[-r, -r * 0.8], [0.0, 0.0], [-r, r * 0.8]], fill);
    p.poly(&[[r, r * 0.8], [0.0, 0.0], [r, -r * 0.8]], fill);
    let outline = [[-r, -r * 0.8], [r, r * 0.8], [r, -r * 0.8], [-r, r * 0.8]];
    p.path(&outline, true, 3.0, if shut { hex(0xff8a80) } else { c::OK });
    // The stem and handle: along the pipe when open, across it when shut.
    let stem = if shut { hex(0xffd2cc) } else { hex(0xc9d3e0) };
    if shut {
        p.path_round(&[[0.0, -r - 6.0], [0.0, r + 6.0]], 5.0, stem);
    } else {
        p.path_round(&[[-r * 0.6, -r - 4.0], [r * 0.6, -r - 4.0]], 5.0, stem);
    }
    if focus {
        dashed_ring(&p, 0.0, 0.0, r + 18.0, c::AMBER, 3.0);
    }
}

fn dashed_ring(g: &Pen, x: f32, y: f32, r: f32, col: Color32, w: f32) {
    g.dashed(&Pen::arc_points(x, y, r, 0.0, TAU), w, col, 7.0, 5.0);
}

fn drips(g: &Pen, x: f32, y: f32, t: f32, n: usize) {
    for j in 0..n {
        let ph = (t * 1.5 + j as f32 / n as f32).rem_euclid(1.0);
        let dx = (((j * 7) % 5) as f32 - 2.0) * 3.0;
        g.disc(x + dx, y + ph * 60.0, 4.5 - 2.5 * ph, rgba(255, 170, 120, 0.95 - 0.7 * ph));
    }
    for j in 0..3 {
        let ph = (t * 0.45 + j as f32 / 3.0).rem_euclid(1.0);
        g.disc(
            x + 16.0 + (t + j as f32).sin() * 8.0,
            y - 10.0 - ph * 50.0,
            7.0 + ph * 12.0,
            rgba(220, 230, 240, 0.13 * (1.0 - ph)),
        );
    }
}

fn core_glyph(g: &Pen, x: f32, y: f32, r: f32, hot: f32) {
    let inner = if hot > 0.0 { rgba(255, 140, 110, 0.5 + 0.4 * hot) } else { rgba(240, 230, 255, 0.75) };
    g.radial(x, y, r * 1.7, inner, rgba(190, 159, 230, 0.0));
    g.disc(x, y, r, hex(0x141c28));
    g.ring(x, y, r, if hot > 0.0 { c::DANGER } else { hex(0x4a5568) }, 4.0);
    g.disc(x, y, r * 0.5, if hot > 0.0 { hex(0xffb4a0) } else { hex(0xe6dcff) });
}

fn chill_glyph(g: &Pen, x: f32, y: f32, w: f32, h: f32) {
    g.panel(x - w / 2.0, y - h / 2.0, w, h, 10.0, hex(0x152030), hex(0x4a5a70));
    for k in 0..7 {
        let xx = x - w / 2.0 + 14.0 + k as f32 * ((w - 28.0) / 6.0);
        g.path(
            &[[xx - 3.0, y - h / 2.0 + 10.0], [xx + 3.0, y - 4.0], [xx - 3.0, y + 6.0], [xx + 3.0, y + h / 2.0 - 10.0]],
            false,
            4.0,
            rgba(79, 168, 247, 0.8),
        );
    }
}

/// Diagonal hatching inside a disc (the meeting glyph's core): the kit's hatch, clipped to a circle.
fn hatch_disc(g: &Pen, x: f32, y: f32, r: f32, col: Color32) {
    let s = std::f32::consts::FRAC_1_SQRT_2;
    let mut m = -r * 2.0;
    while m < r * 2.0 {
        let h2 = r * r - m * m / 2.0;
        if h2 > 0.0 {
            let h = h2.sqrt();
            let (mx, my) = (x + m / 2.0, y + m / 2.0);
            g.line(mx - h * s, my + h * s, mx + h * s, my - h * s, 3.0, col);
        }
        m += 12.0;
    }
}

// ================================================================ isolate
const CORE: [f32; 3] = [640.0, 150.0, 40.0];
const CHILL: [f32; 4] = [640.0, 650.0, 130.0, 54.0];
const XR: f32 = 1010.0;
const XL: f32 = 270.0;
const YT: f32 = 150.0;
const YB: f32 = 650.0;
const BYR: f32 = 1140.0;
const BYL: f32 = 140.0;
/// The inner valves' heights, top to bottom.
const VY: [f32; 3] = [272.0, 400.0, 528.0];
/// Where each bypass leaves and rejoins its leg.
const TEE: [f32; 2] = [205.0, 595.0];
/// K the blanket heats when the core is starved (design 4), shown on the plan.
const BLANKET_JUMP_K: u32 = 20;

/// A valve on the plan: its leg, where, main (at the core or the chiller), its place on the leg's line (1-3, 0 off
/// it), a bypass's own.
#[derive(Clone, Copy, Debug)]
struct Valve {
    leg: Leg,
    x: f32,
    y: f32,
    main: bool,
    i: u32,
    bypass: bool,
}

const fn v(leg: Leg, x: f32, y: f32, main: bool, i: u32, bypass: bool) -> Valve {
    Valve { leg, x, y, main, i, bypass }
}

const VALVES: [Valve; 12] = [
    v(Leg::Cold, 500.0, YT, true, 0, false),
    v(Leg::Hot, 780.0, YT, true, 0, false),
    v(Leg::Hot, XR, VY[0], false, 1, false),
    v(Leg::Hot, XR, VY[1], false, 2, false),
    v(Leg::Hot, XR, VY[2], false, 3, false),
    v(Leg::Hot, 780.0, YB, true, 0, false),
    v(Leg::Cold, 500.0, YB, true, 0, false),
    v(Leg::Cold, XL, VY[2], false, 1, false),
    v(Leg::Cold, XL, VY[1], false, 2, false),
    v(Leg::Cold, XL, VY[0], false, 3, false),
    // The bypasses' own valves, shut in normal running.
    v(Leg::Hot, BYR, 400.0, false, 0, true),
    v(Leg::Cold, BYL, 400.0, false, 0, true),
];

/// Segment n (1-4) of a leg, as its pipe points: the hot leg runs core to chiller, the cold chiller to core.
fn seg_pts(leg: Leg, n: u32) -> Vec<[f32; 2]> {
    let ya = [YT, VY[0], VY[1], VY[2]];
    let yb = [VY[0], VY[1], VY[2], YB];
    let n = n.clamp(1, 4) as usize;
    match leg {
        Leg::Hot => match n {
            1 => vec![[780.0, YT], [XR, YT], [XR, VY[0]]],
            4 => vec![[XR, VY[2]], [XR, YB], [780.0, YB]],
            _ => vec![[XR, ya[n - 1]], [XR, yb[n - 1]]],
        },
        Leg::Cold => match n {
            1 => vec![[500.0, YB], [XL, YB], [XL, VY[2]]],
            4 => vec![[XL, VY[0]], [XL, YT], [500.0, YT]],
            _ => vec![[XL, yb[4 - n]], [XL, ya[4 - n]]],
        },
    }
}

fn seg_mid(leg: Leg, n: u32) -> [f32; 2] {
    let p = seg_pts(leg, n);
    let (a, b) = (p[0], p[p.len() - 1]);
    [(a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0]
}

/// Which valves are shut, one bit a valve.
type Shut = u16;

fn is_shut(shut: Shut, i: usize) -> bool {
    shut & (1 << i) != 0
}

/// A leg's middle run carries coolant when its three line valves are open.
fn line_open(leg: Leg, shut: Shut) -> bool {
    VALVES.iter().enumerate().all(|(i, v)| v.leg != leg || v.i == 0 || !is_shut(shut, i))
}

/// Its bypass carries when the bypass valve is open.
fn bypass_open(leg: Leg, shut: Shut) -> bool {
    VALVES.iter().enumerate().all(|(i, v)| v.leg != leg || !v.bypass || !is_shut(shut, i))
}

/// The leg flows when either does.
fn leg_flows(leg: Leg, shut: Shut) -> bool {
    line_open(leg, shut) || bypass_open(leg, shut)
}

/// A line segment n (2 or 3, between line valves n-1 and n) is cut off when a valve on each side of it is shut.
fn seg_cut(leg: Leg, n: u32, shut: Shut) -> bool {
    let shut_at = |k: u32| VALVES.iter().enumerate().any(|(i, v)| v.leg == leg && v.i == k && is_shut(shut, i));
    (1..=3).any(|k| k < n && shut_at(k)) && (1..=3).any(|k| k >= n && shut_at(k))
}

// ================================================================ rebuild (the tile grid)
const NP: u8 = 0;
const EP: u8 = 1;
const SP: u8 = 2;
const WP: u8 = 3;
const DC: [i32; 4] = [0, 1, 0, -1];
const DR: [i32; 4] = [-1, 0, 1, 0];

fn opp(p: u8) -> u8 {
    (p + 2) % 4
}

/// A tile: a straight, an elbow, a tee or a crossover.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Kind {
    #[default]
    S,
    E,
    T,
    X,
}

impl Kind {
    fn groups(self) -> &'static [&'static [u8]] {
        match self {
            Kind::S => &[&[0, 2]],
            Kind::E => &[&[0, 1]],
            Kind::T => &[&[0, 1, 2]],
            Kind::X => &[&[0, 2], &[1, 3]],
        }
    }
}

/// The tray's three new pieces.
const TRAY: [Kind; 3] = [Kind::S, Kind::E, Kind::X];
const TRAY_BOX: [f32; 4] = [1020.0, 116.0, 220.0, 400.0];
const FILL_BTN: [f32; 4] = [1020.0, 556.0, 220.0, 116.0];

fn slot_xy(i: usize) -> [f32; 2] {
    [TRAY_BOX[0] + TRAY_BOX[2] / 2.0, TRAY_BOX[1] + 70.0 + i as f32 * 130.0]
}

#[derive(Clone, Debug, Default)]
struct Tile {
    kind: Kind,
    k: u32,
    ang: f32,
    sol: bool,
    /// The port pairs the solution runs through it (two on a crossover).
    need: Vec<[u8; 2]>,
    /// The piece the solution lays here.
    need_kind: Kind,
    cracked: bool,
    fresh: f32,
}

impl Tile {
    /// Its port groups, turned k quarters clockwise.
    fn groups(&self) -> Vec<Vec<u8>> {
        self.kind.groups().iter().map(|gp| gp.iter().map(|p| (p + self.k as u8) % 4).collect()).collect()
    }
    /// Whether it lies as the solution needs.
    fn right(&self) -> bool {
        let gs = self.groups();
        self.need.iter().all(|pair| gs.iter().any(|gp| pair.iter().all(|p| gp.contains(p))))
    }
}

/// One cell of a laid run: where, the port it is entered by and the one it is left by.
#[derive(Clone, Copy, Debug)]
struct Step {
    c: usize,
    r: usize,
    pin: u8,
    pout: u8,
}

/// How a walk may move: freely, or (the cold leg) only straight across the hot run's straight cells.
enum Rule<'a> {
    Free,
    Crossing(&'a [Option<Step>], usize),
}

impl Rule<'_> {
    fn forced(&self, c: usize, r: usize, pin: u8) -> Vec<u8> {
        if let Rule::Crossing(hot, cols) = self {
            if hot[r * cols + c].is_some() {
                return vec![opp(pin)];
            }
        }
        (0..4).filter(|&p| p != pin).collect()
    }
    fn enter(&self, c: usize, r: usize, pin: u8) -> bool {
        match self {
            Rule::Free => true,
            Rule::Crossing(hot, cols) => match hot[r * cols + c] {
                None => true,
                Some(h) => (h.pin == EP || h.pin == WP) && h.pout == opp(h.pin) && (pin == NP || pin == SP),
            },
        }
    }
}

/// A random walk from a cell entered by port `pin` to the goal cell left by port `pout`.
struct Walk<'a, 'b> {
    r: &'a mut Dice,
    cols: usize,
    rows: usize,
    goal: (usize, usize),
    pout: u8,
    rule: Rule<'b>,
    bias: [f32; 4],
    budget: i32,
    seen: Vec<bool>,
    path: Vec<Step>,
}

impl Walk<'_, '_> {
    fn go(&mut self, c: usize, rr: usize, pin: u8) -> bool {
        self.budget -= 1;
        if self.budget < 0 {
            return false;
        }
        self.seen[rr * self.cols + c] = true;
        self.path.push(Step { c, r: rr, pin, pout: 9 });
        let outs = self.rule.forced(c, rr, pin);
        let mut order: Vec<(u8, f32)> = outs.iter().map(|&p| (p, self.r.f() + self.bias[p as usize])).collect();
        order.sort_by(|a, b| b.1.total_cmp(&a.1));
        for (p, _) in order {
            if (c, rr) == self.goal && p == self.pout {
                if let Some(s) = self.path.last_mut() {
                    s.pout = p;
                }
                return true;
            }
            let (nc, nr) = (c as i32 + DC[p as usize], rr as i32 + DR[p as usize]);
            if nc < 0 || nr < 0 || nc >= self.cols as i32 || nr >= self.rows as i32 {
                continue;
            }
            let (nc, nr) = (nc as usize, nr as usize);
            if self.seen[nr * self.cols + nc] || !self.rule.enter(nc, nr, opp(p)) {
                continue;
            }
            if let Some(s) = self.path.last_mut() {
                s.pout = p;
            }
            if self.go(nc, nr, opp(p)) {
                return true;
            }
        }
        self.seen[rr * self.cols + c] = false;
        self.path.pop();
        false
    }
}

#[allow(clippy::too_many_arguments)]
fn walk(
    r: &mut Dice,
    cols: usize,
    rows: usize,
    start: (usize, usize),
    pin: u8,
    goal: (usize, usize),
    pout: u8,
    rule: Rule,
    bias: [f32; 4],
) -> Option<Vec<Step>> {
    let mut w =
        Walk { r, cols, rows, goal, pout, rule, bias, budget: 4000, seen: vec![false; cols * rows], path: Vec::new() };
    w.go(start.0, start.1, pin).then_some(w.path)
}

/// The legs' ends: hot enters at row a and leaves at row b; cold enters at column c and leaves at column d.
#[derive(Clone, Copy, Debug, Default)]
struct Ends {
    a: usize,
    b: usize,
    c: usize,
    d: usize,
}

/// The two legs' runs: hot left to right, cold top to bottom, crossing `want` times on crossover tiles.
fn lay_runs(r: &mut Dice, cols: usize, rows: usize, want: usize) -> Option<(Vec<Step>, Vec<Step>, Ends)> {
    for _ in 0..400 {
        let e = Ends { a: r.int(rows), b: r.int(rows), c: 1 + r.int(cols - 2), d: 1 + r.int(cols - 2) };
        let bias = if want > 1 { [0.1, 0.0, 0.1, 0.0] } else { [0.0, 0.4, 0.0, 0.0] };
        let Some(hot) = walk(r, cols, rows, (0, e.a), WP, (cols - 1, e.b), EP, Rule::Free, bias) else { continue };
        let mut hot_at: Vec<Option<Step>> = vec![None; cols * rows];
        for h in &hot {
            hot_at[h.r * cols + h.c] = Some(*h);
        }
        if !Rule::Crossing(&hot_at, cols).enter(e.c, 0, NP) {
            continue;
        }
        for _ in 0..25 {
            let rule = Rule::Crossing(&hot_at, cols);
            let Some(cold) = walk(r, cols, rows, (e.c, 0), NP, (e.d, rows - 1), SP, rule, [0.0, 0.0, 0.15, 0.0]) else {
                break;
            };
            let x = cold.iter().filter(|q| hot_at[q.r * cols + q.c].is_some()).count();
            if x == want {
                return Some((hot, cold, e));
            }
        }
    }
    None
}

/// What the coolant meets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FaultKind {
    Leak,
    Crack,
    Meet,
}

#[derive(Clone, Copy, Debug)]
struct Fault {
    kind: FaultKind,
    leg: Leg,
    at: [f32; 2],
    dir: u8,
    d: f32,
}

/// The coolant's reach through the tiles as they lie: which leg wets each tile's group and at what distance, the port
/// it enters by, where each leg reaches its outlet, and the faults it meets, each at the distance it gets there.
#[derive(Clone, Debug, Default)]
struct Net {
    dist: Vec<Option<u32>>,
    leg_of: Vec<Option<Leg>>,
    from: Vec<Option<u8>>,
    faults: Vec<Fault>,
    reach: [Option<f32>; 2],
    end: f32,
    ok: bool,
}

/// A wet piece: a tile's group, the port it is entered by, its leg.
#[derive(Clone, Copy, Debug)]
struct Node {
    c: usize,
    r: usize,
    gp: usize,
    pin: u8,
    leg: Leg,
}

impl PartialEq for Node {
    fn eq(&self, o: &Self) -> bool {
        (self.c, self.r, self.gp) == (o.c, o.r, o.gp)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Flow {
    #[default]
    Play,
    Flowing,
    Running,
    Drain,
}

#[derive(Clone, Debug, Default)]
struct Grid {
    cols: usize,
    rows: usize,
    tt: f32,
    gx: f32,
    gy: f32,
    tiles: Vec<Tile>,
    ends: Ends,
    cur: (usize, usize),
    sel: usize,
    held: Option<(Kind, f32, f32)>,
    refuse: Option<(usize, usize, f32)>,
    phase: Flow,
    front: f32,
    fumbled: bool,
    after_fault: f32,
    drain_t: f32,
    net: Option<Net>,
}

impl Grid {
    fn tile(&self, c: i32, r: i32) -> Option<&Tile> {
        (c >= 0 && r >= 0 && (c as usize) < self.cols && (r as usize) < self.rows)
            .then(|| &self.tiles[r as usize * self.cols + c as usize])
    }
    fn cell_xy(&self, c: usize, r: usize) -> [f32; 2] {
        [self.gx + c as f32 * self.tt + self.tt / 2.0, self.gy + r as f32 * self.tt + self.tt / 2.0]
    }
    fn key(&self, c: usize, r: usize, gp: usize) -> usize {
        (r * self.cols + c) * 2 + gp
    }
    /// Each leg's entry cell and port, and its exit cell and port.
    fn leg_ends(&self, leg: Leg) -> ((usize, usize), u8, (usize, usize), u8) {
        let e = self.ends;
        match leg {
            Leg::Hot => ((0, e.a), WP, (self.cols - 1, e.b), EP),
            Leg::Cold => ((e.c, 0), NP, (e.d, self.rows - 1), SP),
        }
    }
    fn edge(&self, c: usize, r: usize, p: u8) -> [f32; 2] {
        let [x, y] = self.cell_xy(c, r);
        [x + DC[p as usize] as f32 * self.tt / 2.0, y + DR[p as usize] as f32 * self.tt / 2.0]
    }
    fn group_at(&self, c: usize, r: usize, pin: u8) -> Option<usize> {
        self.tiles[r * self.cols + c].groups().iter().position(|g| g.contains(&pin))
    }

    /// The coolant's reach, both legs filling at once from their inlets through the kit's fill.
    fn network(&self) -> Net {
        let n = self.cols * self.rows * 2;
        let mut net = Net { dist: vec![None; n], leg_of: vec![None; n], from: vec![None; n], ..Net::default() };
        let mut starts = Vec::new();
        for leg in [Leg::Hot, Leg::Cold] {
            let (st, pin, _, _) = self.leg_ends(leg);
            let Some(gp) = self.group_at(st.0, st.1, pin) else {
                let at = self.edge(st.0, st.1, pin);
                net.faults.push(Fault { kind: FaultKind::Leak, leg, at, dir: opp(pin), d: 0.0 });
                continue;
            };
            let k = self.key(st.0, st.1, gp);
            if net.leg_of[k].is_none() {
                net.leg_of[k] = Some(leg);
                net.from[k] = Some(pin);
                net.dist[k] = Some(0);
                starts.push(Node { c: st.0, r: st.1, gp, pin, leg });
            }
        }
        let mut far = 0u32;
        let mut faults = std::mem::take(&mut net.faults);
        let mut reach = [None; 2];
        let reached = flood(&starts, |nd| {
            let k = self.key(nd.c, nd.r, nd.gp);
            let d = net.dist[k].unwrap_or(0);
            far = far.max(d);
            let df = d as f32;
            let tile = &self.tiles[nd.r * self.cols + nd.c];
            let (_, _, goal, pout) = self.leg_ends(nd.leg);
            let (ost, opin, ogoal, opout) = self.leg_ends(nd.leg.other());
            if tile.cracked {
                faults.push(Fault {
                    kind: FaultKind::Crack,
                    leg: nd.leg,
                    at: self.cell_xy(nd.c, nd.r),
                    dir: 0,
                    d: df + 0.5,
                });
            }
            let mut out = Vec::new();
            for &p in &tile.groups()[nd.gp] {
                if p == nd.pin {
                    continue;
                }
                let here = self.edge(nd.c, nd.r, p);
                if (nd.c, nd.r) == goal && p == pout {
                    if reach[nd.leg.i()].is_none() {
                        reach[nd.leg.i()] = Some(df + 1.0);
                    }
                    continue;
                }
                if ((nd.c, nd.r) == ogoal && p == opout) || ((nd.c, nd.r) == ost && p == opin) {
                    faults.push(Fault { kind: FaultKind::Meet, leg: nd.leg, at: here, dir: p, d: df + 1.0 });
                    continue;
                }
                let (nc, nr) = (nd.c as i32 + DC[p as usize], nd.r as i32 + DR[p as usize]);
                let ngp = self.tile(nc, nr).and_then(|_| self.group_at(nc as usize, nr as usize, opp(p)));
                let Some(ngp) = ngp else {
                    faults.push(Fault { kind: FaultKind::Leak, leg: nd.leg, at: here, dir: p, d: df + 1.0 });
                    continue;
                };
                let (nc, nr) = (nc as usize, nr as usize);
                let nk = self.key(nc, nr, ngp);
                match net.leg_of[nk] {
                    None => {
                        net.leg_of[nk] = Some(nd.leg);
                        net.from[nk] = Some(opp(p));
                        net.dist[nk] = Some(d + 1);
                        out.push(Node { c: nc, r: nr, gp: ngp, pin: opp(p), leg: nd.leg });
                    }
                    Some(l) if l != nd.leg => {
                        faults.push(Fault { kind: FaultKind::Meet, leg: nd.leg, at: here, dir: p, d: df + 1.0 });
                    }
                    Some(_) => {}
                }
            }
            out
        });
        debug_assert!(reached.len() >= starts.len());
        faults.sort_by(|a, b| a.d.total_cmp(&b.d));
        let mut end = (far + 1) as f32;
        for r in reach.iter().flatten() {
            end = end.max(*r);
        }
        for f in &faults {
            end = end.max(f.d);
        }
        net.ok = reach.iter().all(Option::is_some) && faults.is_empty();
        net.faults = faults;
        net.reach = reach;
        net.end = end;
        net
    }

    fn fit_part(&mut self, c: i32, r: i32, kind: Kind) -> bool {
        let ok = self.tile(c, r).is_some_and(|t| t.cracked);
        if !ok {
            if c >= 0 && r >= 0 {
                self.refuse = Some((c as usize, r as usize, 0.6));
            }
            return false;
        }
        let t = &mut self.tiles[r as usize * self.cols + c as usize];
        t.kind = kind;
        t.cracked = false;
        t.k = 0;
        t.ang = 0.0;
        t.fresh = 1.0;
        true
    }

    fn cell_of(&self, x: f32, y: f32) -> (i32, i32) {
        (((x - self.gx) / self.tt).floor() as i32, ((y - self.gy) / self.tt).floor() as i32)
    }
}

// ================================================================ refill and bleed
const RUN: [[f32; 2]; 6] =
    [[40.0, 500.0], [420.0, 500.0], [500.0, 330.0], [780.0, 330.0], [860.0, 500.0], [1240.0, 500.0]];
/// The upstream (left) and downstream (right) valves.
const UPV: [f32; 2] = [250.0, 500.0];
const DNV: [f32; 2] = [1040.0, 500.0];
/// The bleed valve on the high point: x, the pipe's y, its top.
const BLEED: [f32; 3] = [640.0, 330.0, 236.0];
const GAUGE: [f32; 3] = [1040.0, 236.0, 76.0];
/// A handwheel's radius, px.
const WHEEL: f32 = 40.0;

fn targets() -> [[f32; 2]; 3] {
    [[UPV[0], UPV[1] - 64.0], [BLEED[0], BLEED[2]], [DNV[0], DNV[1] - 64.0]]
}

#[derive(Clone, Debug, Default)]
struct Bleed {
    up: bool,
    dn: bool,
    up_a: f32,
    dn_a: f32,
    fill: f32,
    air: f32,
    holding: bool,
    settled: f32,
    played: bool,
    jolt: f32,
    focus: usize,
    need: f32,
    spit: f32,
}

// ================================================================ the part step: the new spool
const RACK: [f32; 4] = [860.0, 120.0, 380.0, 560.0];
/// The burst section cut out of the run: x, y, w.
const GAP: [f32; 3] = [420.0, 400.0, 240.0];

fn spool_home(i: usize) -> [f32; 2] {
    [RACK[0] + RACK[2] / 2.0, RACK[1] + 90.0 + i as f32 * 128.0]
}

#[derive(Clone, Copy, Debug, Default)]
struct Spool {
    leg: Leg,
    x: f32,
    y: f32,
    held: bool,
    fitted: bool,
}

#[derive(Clone, Debug, Default)]
struct Part {
    spools: Vec<Spool>,
    held: Option<usize>,
    focus: usize,
    set: bool,
    refuse: f32,
    dx: f32,
    dy: f32,
}

// ================================================================ the game
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Ph {
    #[default]
    Isolate,
    Rebuild,
    Bleed,
    Part,
}

/// The game's state.
#[derive(Default)]
pub struct Pipes {
    job: Option<Job>,
    phase: Ph,
    keys: bool,
    // isolate
    shut: Shut,
    spring: Vec<(usize, f32)>,
    focus: usize,
    blanket: f32,
    done: bool,
    done_t: f32,
    bypass: [f32; 2],
    // rebuild, bleed, part
    grid: Grid,
    bleed: Bleed,
    part: Part,
    after_part: bool,
    /// The hand: frames since it last acted, and a drag under way (from, to).
    hand_n: u32,
    hand_drag: Option<([f32; 2], [f32; 2])>,
}

impl Pipes {
    fn job(&self) -> Job {
        self.job.unwrap_or(Job { leg: Leg::Hot, seg: 2 })
    }

    fn iso_step(&mut self) {
        self.phase = Ph::Isolate;
        self.shut = VALVES.iter().enumerate().filter(|(_, v)| v.bypass).fold(0, |m, (i, _)| m | (1 << i));
        self.spring.clear();
        self.focus = 0;
        self.keys = false;
        self.blanket = 0.0;
        self.done = false;
        self.done_t = 0.0;
        self.bypass = [0.0, 0.0];
    }

    /// The two valves either side of the cracked segment.
    fn iso_need(&self) -> Vec<usize> {
        let j = self.job();
        (0..VALVES.len())
            .filter(|&i| VALVES[i].leg == j.leg && (VALVES[i].i == j.seg - 1 || VALVES[i].i == j.seg))
            .collect()
    }

    fn iso_toggle(&mut self, cx: &mut Ctx, i: usize) {
        let v = VALVES[i];
        if v.main {
            // The core's feed: it slams shut, the blanket heats, and it springs back open.
            self.spring.push((i, 0.7));
            self.blanket = 1.0;
            cx.fumble(STARVED);
            return;
        }
        let next = self.shut ^ (1 << i);
        if !leg_flows(v.leg, next) {
            // That was the leg's last open way: the core's feed stops, the valve springs back open.
            self.spring.push((i, 0.7));
            self.blanket = 1.0;
            cx.fumble(STARVED);
            return;
        }
        self.shut = next;
        // Played: the cracked segment shut off on both sides, and its leg still flowing.
        let j = self.job();
        if seg_cut(j.leg, j.seg, self.shut) && leg_flows(j.leg, self.shut) {
            self.done = true;
            cx.step_done();
        }
    }

    fn iso_update(&mut self, cx: &mut Ctx, dt: f32, input: &Input) {
        self.blanket = (self.blanket - dt * 0.6).max(0.0);
        for s in &mut self.spring {
            s.1 -= dt;
        }
        self.spring.retain(|s| s.1 > 0.0);
        for leg in [Leg::Hot, Leg::Cold] {
            let want = if bypass_open(leg, self.shut) { 1.0 } else { 0.0 };
            self.bypass[leg.i()] += (want - self.bypass[leg.i()]) * (dt * 4.0).min(1.0);
        }
        if self.done {
            self.done_t += dt;
            return;
        }
        let n = VALVES.len();
        for (k, d) in [
            (Key::ArrowLeft, n - 1),
            (Key::A, n - 1),
            (Key::ArrowUp, n - 1),
            (Key::W, n - 1),
            (Key::ArrowRight, 1),
            (Key::D, 1),
            (Key::ArrowDown, 1),
            (Key::S, 1),
            (Key::Tab, 1),
        ] {
            if input.hit(k) {
                self.keys = true;
                self.focus = (self.focus + d) % n;
            }
        }
        if input.action_pressed {
            self.keys = true;
            self.iso_toggle(cx, self.focus);
            return;
        }
        if input.pressed {
            let pts: Vec<Option<[f32; 2]>> = VALVES.iter().map(|v| Some([v.x, v.y])).collect();
            if let Some(i) = nearest(&pts, input.x, input.y, REACH) {
                self.keys = false;
                self.focus = i;
                self.iso_toggle(cx, i);
            }
        }
    }

    fn iso_draw(&self, g: &Pen, t: f32) {
        g.panel(40.0, 92.0, 1200.0, 616.0, 18.0, hex(0x0a0f17), c::LINE);
        let j = self.job();
        let crack_iso = seg_cut(j.leg, j.seg, self.shut);
        // The bypasses: dashed and dry while their valve is shut, running once it is open.
        for leg in [Leg::Hot, Leg::Cold] {
            let (x, b) = if leg == Leg::Hot { (XR, BYR) } else { (XL, BYL) };
            let k = self.bypass[leg.i()];
            let mut pts = vec![[x, TEE[0]], [b, TEE[0]], [b, TEE[1]], [x, TEE[1]]];
            if leg == Leg::Cold {
                pts.reverse();
            }
            pipe(g, &pts, (k > 0.5).then_some(leg), 14.0, 1.5, t, k < 0.5, 1.0);
            if k < 0.5 {
                g.dashed(&pts, 3.0, hex(0x3a4658), 10.0, 8.0);
            }
        }
        // The legs, segment by segment: the middle run flows only while its line is open, stands still when a valve
        // on it is shut, and is drawn dry where it is shut off both sides.
        for leg in [Leg::Hot, Leg::Cold] {
            for n in 1..=4 {
                let mid = n == 2 || n == 3;
                let (line, by) = (line_open(leg, self.shut), bypass_open(leg, self.shut));
                let flow = if !mid {
                    1.5
                } else if line {
                    if by {
                        0.8
                    } else {
                        1.5
                    }
                } else {
                    0.0
                };
                pipe(g, &seg_pts(leg, n), Some(leg), 24.0, flow, t, mid && seg_cut(leg, n, self.shut), 1.0);
            }
        }
        // Main runs: core to the main valves, the main valves to the chiller.
        let [kx, _, kr] = CORE;
        let [chx, _, chw, _] = CHILL;
        pipe(g, &[[kx + kr, YT], [780.0, YT]], Some(Leg::Hot), 24.0, 1.5, t, false, 1.0);
        pipe(g, &[[780.0, YB], [chx + chw / 2.0, YB]], Some(Leg::Hot), 24.0, 1.5, t, false, 1.0);
        pipe(g, &[[chx - chw / 2.0, YB], [500.0, YB]], Some(Leg::Cold), 24.0, 1.5, t, false, 1.0);
        pipe(g, &[[500.0, YT], [kx - kr, YT]], Some(Leg::Cold), 24.0, 1.5, t, false, 1.0);
        // The crack: a split mark and a drip, until it is cut off.
        let [mx, my] = seg_mid(j.leg, j.seg);
        let side = if j.leg == Leg::Hot { 1.0 } else { -1.0 };
        g.path(
            &[[mx - 8.0, my - 18.0], [mx + 6.0, my - 6.0], [mx - 6.0, my + 4.0], [mx + 8.0, my + 18.0]],
            false,
            3.0,
            c::DANGER,
        );
        if !crack_iso {
            drips(g, mx + side * 24.0, my + 8.0, t, 6);
        } else if self.done_t < 1.0 {
            drips(g, mx + side * 24.0, my + 8.0, t, 2);
        }
        core_glyph(g, kx, CORE[1], kr, self.blanket);
        chill_glyph(g, chx, CHILL[1], chw, CHILL[3]);
        for (i, v) in VALVES.iter().enumerate() {
            let shut = is_shut(self.shut, i) || self.spring.iter().any(|s| s.0 == i);
            let vertical = v.x == XR || v.x == XL || v.bypass;
            let r = if v.bypass { 14.0 } else { 18.0 };
            bowtie(g, v.x, v.y, vertical, shut, v.main, r, self.keys && self.focus == i);
        }
        if self.blanket > 0.0 {
            g.text(&format!("BLANKET +{BLANKET_JUMP_K} K"), kx, CORE[1] + 74.0, 20.0, c::DANGER, Align::Center);
        }
    }

    fn rebuild_step(&mut self, r: &mut Dice, cols: usize, rows: usize, tt: f32, want: usize, cracks: usize) {
        let runs = lay_runs(r, cols, rows, want).or_else(|| lay_runs(r, cols, rows, 1));
        let (hot, cold, ends) = runs.unwrap_or_default();
        let mut gr = Grid {
            cols,
            rows,
            tt,
            gx: 540.0 - (cols as f32 * tt) / 2.0,
            gy: 410.0 - (rows as f32 * tt) / 2.0,
            ends,
            ..Grid::default()
        };
        // Decoys first; the runs laid over them; crossings where they meet; then every tile turned at random.
        for _ in 0..cols * rows {
            let q = r.f();
            let kind = if q < 0.36 {
                Kind::S
            } else if q < 0.8 {
                Kind::E
            } else if q < 0.92 {
                Kind::T
            } else {
                Kind::X
            };
            gr.tiles.push(Tile { kind, ..Tile::default() });
        }
        for run in [&hot, &cold] {
            for q in run {
                let tile = &mut gr.tiles[q.r * cols + q.c];
                tile.need.push([q.pin, q.pout]);
                if tile.sol {
                    tile.kind = Kind::X;
                    tile.need_kind = Kind::X;
                    continue;
                }
                tile.sol = true;
                tile.kind = if q.pout == opp(q.pin) { Kind::S } else { Kind::E };
                tile.need_kind = tile.kind;
            }
        }
        for tile in &mut gr.tiles {
            tile.k = r.int(4) as u32;
            tile.ang = tile.k as f32 * PI / 2.0;
        }
        let mut cand: Vec<usize> =
            (0..gr.tiles.len()).filter(|&i| gr.tiles[i].sol && gr.tiles[i].kind != Kind::X).collect();
        for _ in 0..cracks {
            if cand.is_empty() {
                break;
            }
            let i = cand.remove(r.int(cand.len()));
            gr.tiles[i].cracked = true;
        }
        if gr.network().ok {
            let i = cand.first().copied().unwrap_or(0);
            gr.tiles[i].k += 1;
            gr.tiles[i].ang = gr.tiles[i].k as f32 * PI / 2.0;
        }
        self.grid = gr;
        self.phase = Ph::Rebuild;
        self.keys = false;
    }

    /// FILL: the coolant runs through what is built; what it meets is judged as it gets there.
    fn fill(&mut self) {
        let s = &mut self.grid;
        s.net = Some(s.network());
        s.phase = Flow::Flowing;
        s.front = 0.0;
        s.fumbled = false;
        s.after_fault = 0.0;
        s.drain_t = 0.0;
    }

    fn rebuild_update(&mut self, cx: &mut Ctx, dt: f32, input: &Input) {
        let s = &mut self.grid;
        if let Some(rf) = &mut s.refuse {
            rf.2 -= dt;
            if rf.2 <= 0.0 {
                s.refuse = None;
            }
        }
        for tile in &mut s.tiles {
            tile.ang += (tile.k as f32 * PI / 2.0 - tile.ang) * (dt * 16.0).min(1.0);
            tile.fresh = (tile.fresh - dt).max(0.0);
        }
        match s.phase {
            Flow::Flowing => {
                // The front advances tile by tile; the first fault it reaches is the fumble, and it keeps running a
                // moment so every spray it reaches shows, then drains back to the build.
                s.front += dt * FLOW_TILES_S;
                let Some(net) = &s.net else { return };
                let end = net.end;
                if let Some(first) = net.faults.first().copied() {
                    if !s.fumbled && s.front >= first.d {
                        s.fumbled = true;
                        cx.fumble(match first.kind {
                            FaultKind::Crack => CRACKED,
                            FaultKind::Meet => MIXED,
                            FaultKind::Leak => OPEN_END,
                        });
                        return;
                    }
                }
                if s.fumbled {
                    s.after_fault += dt;
                    if s.after_fault >= FAULT_SHOW_S || s.front >= end + 0.5 {
                        s.phase = Flow::Drain;
                        s.drain_t = 0.0;
                    }
                } else if s.front >= end + 0.6 {
                    s.phase = Flow::Running;
                    cx.step_done();
                }
                return;
            }
            Flow::Running => {
                s.front += dt * FLOW_TILES_S;
                return;
            }
            Flow::Drain => {
                s.drain_t += dt;
                if s.drain_t >= DRAIN_S {
                    s.phase = Flow::Play;
                    s.net = None;
                    s.front = 0.0;
                }
                return;
            }
            Flow::Play => {}
        }
        // Keys: a cursor; Space turns (or fits the picked piece on a cracked tile); Q and E pick a piece; Enter fills.
        for (k, dc, dr) in [
            (Key::ArrowLeft, -1, 0),
            (Key::A, -1, 0),
            (Key::ArrowRight, 1, 0),
            (Key::D, 1, 0),
            (Key::ArrowUp, 0, -1),
            (Key::W, 0, -1),
            (Key::ArrowDown, 0, 1),
            (Key::S, 0, 1),
        ] {
            if input.hit(k) {
                self.keys = true;
                s.cur.0 = (s.cur.0 as i32 + dc).clamp(0, s.cols as i32 - 1) as usize;
                s.cur.1 = (s.cur.1 as i32 + dr).clamp(0, s.rows as i32 - 1) as usize;
            }
        }
        if input.hit(Key::Q) {
            self.keys = true;
            s.sel = (s.sel + TRAY.len() - 1) % TRAY.len();
        }
        if input.hit(Key::E) {
            self.keys = true;
            s.sel = (s.sel + 1) % TRAY.len();
        }
        if input.hit(Key::Space) {
            self.keys = true;
            let (c, r) = s.cur;
            if s.tiles[r * s.cols + c].cracked {
                let kind = TRAY[s.sel];
                s.fit_part(c as i32, r as i32, kind);
            } else {
                s.tiles[r * s.cols + c].k += 1;
            }
        }
        if input.hit(Key::Enter) {
            self.fill();
            return;
        }
        // Pointer: a press on the tray picks a piece up; a drag carries it; a tap on a tile turns it; FILL fills.
        if input.pressed {
            let slots: Vec<Option<[f32; 2]>> = (0..TRAY.len()).map(|k| Some(slot_xy(k))).collect();
            if let Some(i) = nearest(&slots, input.x, input.y, 62.0) {
                s.held = Some((TRAY[i], input.x, input.y));
                s.sel = i;
                self.keys = false;
                return;
            }
            let [fx, fy, fw, fh] = FILL_BTN;
            if input.over(fx, fy, fw, fh) {
                self.fill();
                return;
            }
            let (c, r) = s.cell_of(input.x, input.y);
            if s.tile(c, r).is_some() {
                s.tiles[r as usize * s.cols + c as usize].k += 1;
                s.cur = (c as usize, r as usize);
                self.keys = false;
            }
        }
        if let Some(h) = &mut s.held {
            if input.down {
                h.1 = input.x;
                h.2 = input.y;
            } else {
                let (kind, x, y) = *h;
                s.held = None;
                let (c, r) = s.cell_of(x, y);
                if s.tile(c, r).is_some() {
                    s.fit_part(c, r, kind);
                }
            }
        }
    }

    fn rebuild_draw(&self, g: &Pen, t: f32) {
        let s = &self.grid;
        let tt = s.tt;
        let live_net;
        let net = match &s.net {
            Some(n) => n,
            None => {
                live_net = s.network();
                &live_net
            }
        };
        let flowing = s.phase != Flow::Play;
        let front = s.front;
        let fade = if s.phase == Flow::Drain { (1.0 - s.drain_t / DRAIN_S).max(0.0) } else { 1.0 };
        let (cols, rows) = (s.cols as f32, s.rows as f32);
        g.panel(s.gx - 14.0, s.gy - 14.0, cols * tt + 28.0, rows * tt + 28.0, 16.0, hex(0x0b1018), c::LINE);
        // The ends: stubs in from outside. The inlets stand full behind the grid; an outlet fills once its leg's
        // coolant reaches it.
        let (hs, _, hg, _) = s.leg_ends(Leg::Hot);
        let (cs, _, cg, _) = s.leg_ends(Leg::Cold);
        let hy0 = s.cell_xy(hs.0, hs.1)[1];
        let hy1 = s.cell_xy(hg.0, hg.1)[1];
        let cx0 = s.cell_xy(cs.0, cs.1)[0];
        let cx1 = s.cell_xy(cg.0, cg.1)[0];
        let outlet = |leg: Leg| match net.reach[leg.i()] {
            Some(r) if flowing && front >= r => ((front - r) / 1.2).clamp(0.05, 1.0) * fade,
            _ => 0.0,
        };
        let inflow = if flowing { 1.0 } else { 0.3 };
        pipe(g, &[[20.0, hy0], [s.gx, hy0]], Some(Leg::Hot), 24.0, inflow, t, false, 1.0);
        let oh = outlet(Leg::Hot);
        pipe(
            g,
            &[[s.gx + cols * tt, hy1], [s.gx + cols * tt + 90.0, hy1]],
            Some(Leg::Hot),
            24.0,
            1.0,
            t,
            oh <= 0.0,
            oh,
        );
        pipe(g, &[[cx0, 84.0], [cx0, s.gy]], Some(Leg::Cold), 24.0, inflow, t, false, 1.0);
        let oc = outlet(Leg::Cold);
        pipe(g, &[[cx1, s.gy + rows * tt], [cx1, 712.0]], Some(Leg::Cold), 24.0, 1.0, t, oc <= 0.0, oc);
        for (x, y, vert) in [
            (s.gx - 4.0, hy0, true),
            (s.gx + cols * tt + 4.0, hy1, true),
            (cx0, s.gy - 4.0, false),
            (cx1, s.gy + rows * tt + 4.0, false),
        ] {
            if vert {
                g.rect(x - 4.0, y - 22.0, 8.0, 44.0, hex(0x8796aa));
            } else {
                g.rect(x - 22.0, y - 4.0, 44.0, 8.0, hex(0x8796aa));
            }
        }
        // The tiles: the groups a leg reaches carry its tint while building, and fill in order of distance once
        // flowing.
        for r in 0..s.rows {
            for cc in 0..s.cols {
                let tile = &s.tiles[r * s.cols + cc];
                let [x, y] = s.cell_xy(cc, r);
                let bg = if tile.cracked {
                    hex(0x1d1410)
                } else if tile.fresh > 0.0 {
                    rgba(61, 220, 132, 0.25 * tile.fresh)
                } else {
                    hex(0x0f1520)
                };
                g.round(x - tt / 2.0 + 3.0, y - tt / 2.0 + 3.0, tt - 6.0, tt - 6.0, 10.0, Some(bg), None);
                if tile.cracked {
                    let o =
                        crate::pen::round_rect_points(x - tt / 2.0 + 3.0, y - tt / 2.0 + 3.0, tt - 6.0, tt - 6.0, 10.0);
                    let mut closed = o.clone();
                    closed.push(o[0]);
                    g.dashed(&closed, 2.0, rgba(255, 71, 87, 0.6), 6.0, 5.0);
                }
                let mut legs = [None; 2];
                let mut wets = [0.0; 2];
                let mut froms = [None; 2];
                for gi in 0..2 {
                    let k = s.key(cc, r, gi);
                    legs[gi] = net.leg_of[k];
                    wets[gi] = match net.dist[k] {
                        Some(d) if flowing => (front - d as f32).clamp(0.0, 1.0),
                        _ => 0.0,
                    };
                    froms[gi] = net.from[k].map(|f| ((f as i32 - tile.k as i32).rem_euclid(4)) as u8);
                }
                tile_pipes(&g.translate(x, y), tile.kind, tile.ang, tt, &legs, &wets, &froms, t, fade);
                if tile.cracked {
                    g.path(
                        &[[x - 14.0, y - 20.0], [x - 2.0, y - 6.0], [x - 12.0, y + 4.0], [x + 4.0, y + 20.0]],
                        false,
                        3.0,
                        c::DANGER,
                    );
                    drips(g, x + 18.0, y + 10.0, t + cc as f32 * 0.3, 4);
                }
                if s.refuse.is_some_and(|rf| rf.0 == cc && rf.1 == r) {
                    g.cross(x, y, 18.0, 5.0, c::DANGER);
                }
            }
        }
        // What the coolant met, each once it gets there: a jet at an open end, a spray from a cracked tile, steam
        // where the legs meet.
        if flowing {
            for f in &net.faults {
                if front < f.d {
                    continue;
                }
                let [fx, fy] = f.at;
                match f.kind {
                    FaultKind::Leak => jet(g, fx, fy, f.dir, t, f.leg, fade),
                    FaultKind::Crack => {
                        jet(g, fx + 6.0, fy, EP, t, f.leg, fade * 0.9);
                        jet(g, fx - 6.0, fy, WP, t + 0.37, f.leg, fade * 0.9);
                    }
                    FaultKind::Meet => meet_glyph(g, fx, fy, t, fade),
                }
            }
        }
        if self.keys && s.phase == Flow::Play {
            let [x, y] = s.cell_xy(s.cur.0, s.cur.1);
            g.round(x - tt / 2.0 + 1.0, y - tt / 2.0 + 1.0, tt - 2.0, tt - 2.0, 10.0, None, Some((4.0, c::AMBER)));
        }
        // The tray: three new pieces, picked up by a press and dropped on a cracked tile.
        let [bx, by, bw, bh] = TRAY_BOX;
        g.panel(bx, by, bw, bh, 16.0, hex(0x0c121a), c::LINE);
        for (i, kind) in TRAY.iter().enumerate() {
            let [x, y] = slot_xy(i);
            let edge = if self.keys && s.sel == i { c::AMBER } else { hex(0x2a3446) };
            g.round(x - 52.0, y - 52.0, 104.0, 104.0, 12.0, Some(hex(0x141c28)), Some((2.0, edge)));
            tile_pipes(&g.translate(x, y), *kind, 0.0, 84.0, &[None; 2], &[0.0; 2], &[None; 2], 0.0, 1.0);
        }
        // FILL: the round's main action, the biggest control.
        let ready = s.phase == Flow::Play;
        let [fx, fy, fw, fh] = FILL_BTN;
        g.round(
            fx,
            fy,
            fw,
            fh,
            22.0,
            Some(if ready { hex(0x1f6b45) } else { hex(0x1a2230) }),
            Some((3.0, if ready { c::OK } else { c::LINE })),
        );
        g.text("FILL", fx + fw / 2.0, fy + fh / 2.0, 40.0, if ready { c::FG } else { c::DIM }, Align::Center);
        if let Some((kind, hx, hy)) = s.held {
            let p = g.translate(hx, hy);
            p.alpha(0.9).round(-44.0, -44.0, 88.0, 88.0, 12.0, Some(rgba(20, 28, 40, 0.85)), None);
            tile_pipes(&p.alpha(0.9), kind, 0.0, 84.0, &[None; 2], &[0.0; 2], &[None; 2], 0.0, 1.0);
        }
    }

    fn bleed_update(&mut self, cx: &mut Ctx, dt: f32, input: &Input) {
        let s = &mut self.bleed;
        s.jolt = (s.jolt - dt * 2.0).max(0.0);
        let k = (dt * 6.0).min(1.0);
        s.up_a += ((if s.up { PI * 1.5 } else { 0.0 }) - s.up_a) * k;
        s.dn_a += ((if s.dn { PI * 1.5 } else { 0.0 }) - s.dn_a) * k;
        if s.up && s.dn {
            s.fill = (s.fill + dt / 1.2).min(1.0);
        }
        for (key, d) in [(Key::ArrowLeft, 2), (Key::A, 2), (Key::ArrowRight, 1), (Key::D, 1), (Key::Tab, 1)] {
            if input.hit(key) {
                self.keys = true;
                s.focus = (s.focus + d) % 3;
            }
        }
        let operate = |s: &mut Bleed, which: usize, cx: &mut Ctx| {
            if which == 0 && !s.up {
                if !s.dn {
                    // Upstream first: the slug of coolant hits a shut valve.
                    s.jolt = 1.0;
                    cx.fumble(HAMMER);
                    return;
                }
                s.up = true;
            } else if which == 2 && !s.dn {
                s.dn = true;
            }
        };
        if input.action_pressed && s.focus != 1 {
            self.keys = true;
            let f = s.focus;
            operate(s, f, cx);
        }
        let mut hold = self.keys && s.focus == 1 && input.action;
        if input.pressed {
            let pts: Vec<Option<[f32; 2]>> = targets().iter().map(|p| Some(*p)).collect();
            if let Some(i) = nearest(&pts, input.x, input.y, WHEEL + 14.0) {
                self.keys = false;
                s.focus = i;
                if i != 1 {
                    operate(s, i, cx);
                }
            }
        }
        if !self.keys && s.focus == 1 && input.down && input.dist(BLEED[0], BLEED[2]) < WHEEL + 30.0 {
            hold = true;
        }
        s.holding = hold;
        if hold && s.fill >= 1.0 && s.air > 0.0 {
            s.air = (s.air - dt / s.need.max(0.1)).max(0.0);
        }
        if hold && s.air <= 0.0 {
            s.spit = (s.spit + dt * 2.0).min(1.0);
        } else {
            s.spit = (s.spit - dt).max(0.0);
        }
        if s.air <= 0.0 && s.fill >= 1.0 {
            s.settled += dt;
            if s.settled > 0.4 && !s.played {
                s.played = true;
                cx.step_done();
            }
        }
    }

    fn bleed_draw(&self, g: &Pen, t: f32) {
        let s = &self.bleed;
        let leg = self.job().leg;
        let jx = if s.jolt > 0.0 { (t * 60.0).sin() * 10.0 * s.jolt } else { 0.0 };
        g.panel(20.0, 92.0, 1240.0, 616.0, 18.0, hex(0x0a0f17), c::LINE);
        // Where the run comes from and goes to: the core or the chiller at each end, the flow left to right.
        let both = s.up && s.dn;
        pipe(g, &[[70.0, 600.0], [70.0, 500.0]], Some(leg), 22.0, 1.0, t, false, 1.0);
        pipe(g, &[[1210.0, 500.0], [1210.0, 600.0]], Some(leg), 22.0, 1.0, t, !both, 1.0);
        let src_core = leg == Leg::Hot;
        for (x, core) in [(70.0, src_core), (1210.0, !src_core)] {
            if core {
                core_glyph(g, x, 620.0, 34.0, 0.0);
            } else {
                chill_glyph(g, x, 620.0, 96.0, 52.0);
            }
        }
        // The run: live up to the upstream valve; the new segment fills as the valves open; live again past
        // downstream.
        pipe(g, &[RUN[0], UPV], Some(leg), 24.0, 1.0, t, false, 1.0);
        let m = g.translate(jx, 0.0);
        let mid = [UPV, RUN[1], RUN[2], RUN[3], RUN[4], DNV];
        pipe(&m, &mid, Some(leg), 24.0, if both { 1.0 } else { 0.0 }, t, s.fill <= 0.0, s.fill);
        // The air pocket at the high point: what the bleed lets out.
        let [bx, by, btop] = BLEED;
        if s.fill > 0.6 && s.air > 0.0 {
            let w = 200.0 * s.air;
            m.round(bx - w / 2.0, by - 11.0, w, 22.0, 11.0, Some(rgba(230, 240, 250, 0.85)), None);
            for k in 0..6 {
                let ox = (k as f32 * 37.0 + t * 40.0).rem_euclid((w - 24.0).max(10.0));
                m.disc(bx - w / 2.0 + 12.0 + ox, by + (t * 5.0 + k as f32).sin() * 3.0, 4.0, rgba(120, 140, 170, 0.8));
            }
        }
        // The joints at the new segment's ends: flanges that jump in a water hammer.
        for x in [UPV[0] + 70.0, DNV[0] - 70.0] {
            m.rect(x - 5.0, 470.0, 10.0, 60.0, if s.jolt > 0.0 { c::DANGER } else { hex(0x9aa6b6) });
        }
        pipe(g, &[DNV, RUN[5]], Some(leg), 24.0, if both { 1.0 } else { 0.0 }, t, !both, 1.0);
        // Flow arrows, so up and downstream read without words.
        for x in [140.0, 1140.0] {
            g.poly(&[[x + 16.0, 548.0], [x - 10.0, 534.0], [x - 10.0, 562.0]], hex(0x6f7f94));
        }
        wheel(g, UPV, s.up_a, s.up, self.keys && s.focus == 0);
        wheel(g, DNV, s.dn_a, s.dn, self.keys && s.focus == 2);
        // The bleed valve: a petcock on a stem over the high point; held, it hisses air, then spits coolant.
        g.line(bx, by - 12.0, bx, btop, 8.0, hex(0x8796aa));
        let on = s.holding;
        g.disc(bx, btop, WHEEL - 4.0, if on { hex(0x3a2a12) } else { hex(0x1b2433) });
        g.ring(bx, btop, WHEEL - 4.0, if on { c::AMBER } else { hex(0x8796aa) }, 5.0);
        let h = g.translate(bx, btop).rotate(if on { PI / 2.0 } else { 0.0 });
        h.round(-26.0, -8.0, 52.0, 16.0, 8.0, Some(if on { c::AMBER } else { hex(0xc9d3e0) }), None);
        if self.keys && s.focus == 1 {
            dashed_ring(g, bx, btop, WHEEL + 12.0, c::AMBER, 3.0);
        }
        if !s.played && both && s.fill >= 1.0 && !on {
            g.ring(bx, btop, WHEEL + 8.0 + 3.0 * (t * 6.0).sin(), rgba(79, 195, 247, 0.8), 3.0);
        }
        if on && s.fill > 0.0 {
            for j in 0..12 {
                let ph = (t * 2.5 + j as f32 / 12.0).rem_euclid(1.0);
                let side = if j % 2 == 1 { 1.0 } else { -1.0 };
                let a = -PI / 2.0 + side * (0.3 + 0.5 * (j as f32 * 0.37).rem_euclid(1.0));
                let (x, y) = (bx + 28.0 + a.cos() * ph * 70.0, btop - 6.0 + a.sin() * ph * 70.0);
                let col = if s.air > 0.0 {
                    rgba(235, 242, 250, 0.75 * (1.0 - ph))
                } else if leg == Leg::Hot {
                    rgba(255, 150, 120, 0.9 * (1.0 - ph))
                } else {
                    rgba(120, 190, 250, 0.9 * (1.0 - ph))
                };
                g.disc(x, y, 5.0 + 8.0 * ph, col);
            }
        }
        // The gauge: the needle wanders while there is air in the run and settles in its band once it is out.
        let [gx, gy, gr] = GAUGE;
        g.disc(gx, gy, gr + 10.0, hex(0x0b1119));
        g.ring(gx, gy, gr + 10.0, hex(0x3a4658), 3.0);
        let au = |u: f32| PI * 0.8 + (PI * 2.2 - PI * 0.8) * u;
        g.arc(gx, gy, gr - 6.0, au(0.55), au(0.7), 12.0, rgba(61, 220, 132, 0.8));
        for k in 0..=10 {
            let a = au(k as f32 / 10.0);
            g.line(
                gx + a.cos() * (gr - 22.0),
                gy + a.sin() * (gr - 22.0),
                gx + a.cos() * (gr - 14.0),
                gy + a.sin() * (gr - 14.0),
                2.0,
                hex(0x4a5568),
            );
        }
        let base = s.fill * 0.62;
        let wob = if s.fill >= 1.0 { s.air * 0.22 * (t * 9.0).sin() + s.air * 0.1 * (t * 23.0).sin() } else { 0.0 };
        let na = au((base + wob).clamp(0.0, 1.0));
        let settled = s.air <= 0.0 && s.fill >= 1.0;
        g.path_round(
            &[[gx, gy], [gx + na.cos() * (gr - 18.0), gy + na.sin() * (gr - 18.0)]],
            5.0,
            if settled { c::OK } else { c::FG },
        );
        g.disc(gx, gy, 8.0, hex(0x8796aa));
        if settled {
            g.path(&[[gx - 14.0, gy + 40.0], [gx - 3.0, gy + 51.0], [gx + 16.0, gy + 30.0]], false, 5.0, c::OK);
        }
        // The order, as a badge on each valve: 1 downstream, 2 upstream, 3 the bleed.
        if !s.dn {
            g.order_badge(DNV[0], DNV[1] - 64.0, WHEEL - 14.0, 1, true, t);
        }
        if !s.up {
            g.order_badge(UPV[0], UPV[1] - 64.0, WHEEL - 14.0, 2, s.dn, t);
        }
        if !s.played {
            g.order_badge(bx, btop, WHEEL - 14.0, 3, both, t);
        }
    }

    fn part_step(&mut self, r: &mut Dice) {
        // Two of each leg's spool, in a seeded order on the rack.
        let mut legs = [Leg::Hot, Leg::Cold, Leg::Hot, Leg::Cold];
        r.shuffle(&mut legs);
        self.part = Part {
            spools: legs
                .iter()
                .enumerate()
                .map(|(i, &leg)| {
                    let [x, y] = spool_home(i);
                    Spool { leg, x, y, held: false, fitted: false }
                })
                .collect(),
            ..Part::default()
        };
        self.phase = Ph::Part;
        self.keys = false;
    }

    fn part_update(&mut self, cx: &mut Ctx, dt: f32, input: &Input) {
        let leg = self.job().leg;
        let s = &mut self.part;
        s.refuse = (s.refuse - dt).max(0.0);
        let set = s.set;
        for (i, sp) in s.spools.iter_mut().enumerate() {
            if !sp.held && !(set && sp.fitted) {
                let [hx, hy] = spool_home(i);
                let k = (dt * 10.0).min(1.0);
                sp.x += (hx - sp.x) * k;
                sp.y += (hy - sp.y) * k;
            }
        }
        if set {
            return;
        }
        let drop = |s: &mut Part, i: usize, cx: &mut Ctx| {
            if s.spools[i].leg != leg {
                s.refuse = 0.8;
                cx.say("Wrong leg");
                return;
            }
            let sp = &mut s.spools[i];
            sp.fitted = true;
            sp.x = GAP[0] + GAP[2] / 2.0;
            sp.y = GAP[1];
            s.set = true;
            cx.step_done();
        };
        let n = s.spools.len();
        for (k, d) in [(Key::ArrowUp, n - 1), (Key::W, n - 1), (Key::ArrowDown, 1), (Key::S, 1), (Key::Tab, 1)] {
            if input.hit(k) {
                self.keys = true;
                s.focus = (s.focus + d) % n;
            }
        }
        if input.action_pressed {
            self.keys = true;
            let f = s.focus;
            drop(s, f, cx);
            return;
        }
        if input.pressed {
            let pts: Vec<Option<[f32; 2]>> = s.spools.iter().map(|sp| Some([sp.x, sp.y])).collect();
            if let Some(i) = nearest(&pts, input.x, input.y, 70.0) {
                s.held = Some(i);
                s.spools[i].held = true;
                s.dx = s.spools[i].x - input.x;
                s.dy = s.spools[i].y - input.y;
                self.keys = false;
            }
        }
        if let Some(i) = s.held {
            if input.down {
                s.spools[i].x = input.x + s.dx;
                s.spools[i].y = input.y + s.dy;
            } else {
                s.spools[i].held = false;
                s.held = None;
                let sp = s.spools[i];
                if (sp.x - (GAP[0] + GAP[2] / 2.0)).abs() < 110.0 && (sp.y - GAP[1]).abs() < 70.0 {
                    drop(s, i, cx);
                }
            }
        }
    }

    fn part_draw(&self, g: &Pen) {
        let s = &self.part;
        let leg = self.job().leg;
        g.panel(20.0, 92.0, 820.0, 616.0, 18.0, hex(0x0a0f17), c::LINE);
        let [gx, gy, gw] = GAP;
        // The run with its burst section cut out: the flanges either side, the drip pan under it.
        pipe(g, &[[60.0, gy], [gx, gy]], None, 24.0, 0.0, 0.0, true, 1.0);
        pipe(g, &[[gx + gw, gy], [800.0, gy]], None, 24.0, 0.0, 0.0, true, 1.0);
        g.rect(gx - 6.0, gy - 30.0, 12.0, 60.0, hex(0x9aa6b6));
        g.rect(gx + gw - 6.0, gy - 30.0, 12.0, 60.0, hex(0x9aa6b6));
        if !s.set {
            let o = crate::pen::round_rect_points(gx + 6.0, gy - 26.0, gw - 12.0, 52.0, 8.0);
            let mut closed = o.clone();
            closed.push(o[0]);
            g.dashed(&closed, 3.0, rgba(232, 238, 246, 0.5), 10.0, 8.0);
        }
        // Which leg: the run's own stripes at both ends (hot chevrons or cold dots).
        for x in [140.0, 700.0] {
            for k in -1..=1 {
                stripe(g, x + k as f32 * 26.0, gy, leg);
            }
        }
        g.rect(gx - 30.0, 560.0, gw + 60.0, 18.0, hex(0x141b27));
        let [rx, ry, rw, rh] = RACK;
        g.panel(rx, ry, rw, rh, 16.0, hex(0x0c121a), c::LINE);
        for i in 0..4 {
            g.rect(rx + 20.0, spool_home(i)[1] + 38.0, rw - 40.0, 8.0, hex(0x1d2636));
        }
        for (i, sp) in s.spools.iter().enumerate() {
            if !sp.held {
                spool_glyph(g, sp.x, sp.y, sp.leg);
            }
            if self.keys && s.focus == i && !s.set {
                let o = crate::pen::round_rect_points(sp.x - 130.0, sp.y - 44.0, 260.0, 88.0, 12.0);
                let mut closed = o.clone();
                closed.push(o[0]);
                g.dashed(&closed, 3.0, c::AMBER, 7.0, 5.0);
            }
        }
        for sp in s.spools.iter().filter(|sp| sp.held) {
            spool_glyph(g, sp.x, sp.y, sp.leg);
        }
        if s.refuse > 0.0 {
            g.cross(gx + gw / 2.0, gy, 24.0, 6.0, c::DANGER);
        }
    }
}

/// A leg's mark on a pipe: a hot chevron or a cold dot.
fn stripe(g: &Pen, x: f32, y: f32, leg: Leg) {
    if leg == Leg::Hot {
        g.path(&[[x - 6.0, y - 9.0], [x + 5.0, y], [x - 6.0, y + 9.0]], false, 4.0, HOT);
    } else {
        g.disc(x, y, 5.0, COLD);
    }
}

fn spool_glyph(g: &Pen, x: f32, y: f32, leg: Leg) {
    let w = 220.0;
    g.round(x - w / 2.0, y - 20.0, w, 40.0, 8.0, Some(hex(0x566375)), Some((2.0, hex(0x8796aa))));
    g.rect(x - w / 2.0 - 6.0, y - 30.0, 12.0, 60.0, hex(0x9aa6b6));
    g.rect(x + w / 2.0 - 6.0, y - 30.0, 12.0, 60.0, hex(0x9aa6b6));
    for bx in [x - w / 2.0 + 24.0, x + w / 2.0 - 44.0] {
        g.rect(bx, y - 20.0, 20.0, 40.0, leg.col());
    }
    for k in -2..=2 {
        stripe(g, x + k as f32 * 26.0, y, leg);
    }
}

/// A handwheel valve: the bowtie on the pipe, a stem, the wheel on top.
fn wheel(g: &Pen, at: [f32; 2], a: f32, open: bool, focus: bool) {
    let [x, y] = at;
    g.line(x, y, x, y - 64.0, 6.0, hex(0x8796aa));
    bowtie(g, x, y, false, !open, false, 20.0, false);
    let wy = y - 64.0;
    g.ring(x, wy, WHEEL - 6.0, if open { c::OK } else { hex(0xc0392b) }, 9.0);
    for k in 0..3 {
        let b = a + k as f32 * TAU / 3.0;
        g.line(x, wy, x + b.cos() * (WHEEL - 10.0), wy + b.sin() * (WHEEL - 10.0), 6.0, hex(0xc9d3e0));
    }
    g.disc(x, wy, 9.0, hex(0x2a3446));
    if focus {
        dashed_ring(g, x, wy, WHEEL + 12.0, c::AMBER, 3.0);
    }
}

/// One tile's pipes at the pen's origin, its groups turned by `ang`. A group a leg reaches is tinted with its pattern
/// while building (the preview); once FILL runs, the coolant fills it from its inlet port, `wets[gi]` of the way.
#[allow(clippy::too_many_arguments)]
fn tile_pipes(
    g: &Pen,
    kind: Kind,
    ang: f32,
    tt: f32,
    legs: &[Option<Leg>; 2],
    wets: &[f32; 2],
    froms: &[Option<u8>; 2],
    t: f32,
    fade: f32,
) {
    let w = tt * 0.24;
    let g = g.rotate(ang);
    let at2 = |p: u8| [DC[p as usize] as f32 * tt / 2.0, DR[p as usize] as f32 * tt / 2.0];
    for (gi, gp) in kind.groups().iter().enumerate() {
        let leg = legs[gi];
        let wf = if leg.is_some() { wets[gi] } else { 0.0 };
        // A crossover's second group (east-west) bridges over the first: drawn last, with a gap cut under it.
        if kind == Kind::X && gi == 1 {
            g.line(-tt / 2.0 + w, 0.0, tt / 2.0 - w, 0.0, w + 16.0, hex(0x0d131c));
        }
        let f = froms[gi].filter(|f| gp.contains(f)).unwrap_or(gp[0]);
        let br: Vec<Vec<[f32; 2]>> =
            gp.iter().filter(|&&q| q != f).map(|&q| vec![at2(f), [0.0, 0.0], at2(q)]).collect();
        let style = PipeStyle {
            w,
            bore: BORE,
            body: if leg.is_some() { hex(0x5a6a82) } else { hex(0x3a4658) },
            dry: match leg {
                Some(Leg::Hot) => hex(0x3a1f1c),
                Some(Leg::Cold) => hex(0x14263a),
                None => hex(0x141b27),
            },
            fluid: leg.filter(|_| wf > 0.0).map(Leg::col),
            fluid_alpha: fade,
            ..PipeStyle::default()
        };
        let runs: Vec<(Vec<[f32; 2]>, f32)> = br.iter().map(|p| (p.clone(), wf)).collect();
        g.pipe(&runs, &style);
        if kind != Kind::X && gi == 0 {
            let body = if leg.is_some() { hex(0x6b7c95) } else { hex(0x4a5566) };
            let fluid = leg.filter(|_| wf >= 0.5).map(Leg::col);
            g.pipe_hub(0.0, 0.0, w * 0.62, body, hex(0x141b27), fluid, fade);
        }
        // The leg's pattern, never colour alone: bright and running where the coolant is, faint where it will go.
        if let Some(leg) = leg {
            for pts in &br {
                for u0 in [1.0 / 6.0, 0.5, 5.0 / 6.0] {
                    let u = if wf >= 1.0 { (u0 + (t * 0.9 * FLOW_TILES_S) / 6.0).rem_euclid(1.0) } else { u0 };
                    let wet = u <= wf;
                    let [x, y, a] = poly_at(pts, u);
                    let p = g.translate(x, y).rotate(a);
                    let p = if wet { p.alpha(fade) } else { p };
                    let ink = match (wet, leg) {
                        (true, Leg::Hot) => hex(0xfff3e0),
                        (true, Leg::Cold) => hex(0xe8f6ff),
                        (false, Leg::Hot) => rgba(255, 210, 190, 0.45),
                        (false, Leg::Cold) => rgba(210, 235, 255, 0.45),
                    };
                    if leg == Leg::Hot {
                        p.path_round(&[[-3.0, -4.5], [2.5, 0.0], [-3.0, 4.5]], 2.2, ink);
                    } else {
                        p.disc(0.0, 0.0, 2.6, ink);
                    }
                }
            }
        }
    }
}

/// Coolant jetting out of an open joint, the way it was running (`dir`, a port): a spray, and steam off the hot leg.
#[allow(clippy::too_many_arguments)]
fn jet(g: &Pen, x: f32, y: f32, dir: u8, t: f32, leg: Leg, k: f32) {
    let (dx, dy) = (DC[dir as usize] as f32, DR[dir as usize] as f32);
    let (r, gg, b) = if leg == Leg::Hot { (255, 150, 110) } else { (130, 195, 250) };
    for j in 0..36 {
        let ph = (t * 2.2 + j as f32 / 36.0).rem_euclid(1.0);
        let side = ((j * 7) % 11) as f32 / 10.0 - 0.5;
        let px = x + dx * ph * 120.0 + if dx != 0.0 { 0.0 } else { side * ph * 70.0 };
        let py = y + dy * ph * 120.0 + if dy != 0.0 { 0.0 } else { side * ph * 70.0 } + ph * ph * 50.0;
        g.disc(px, py, 9.0 - 6.0 * ph, rgba(r, gg, b, 0.95 * (1.0 - ph) * k));
    }
    g.disc(x, y, 13.0, rgba(r, gg, b, 0.9 * k));
    if leg == Leg::Hot {
        for j in 0..4 {
            let ph = (t * 0.7 + j as f32 / 4.0).rem_euclid(1.0);
            g.disc(
                x + dx * 40.0 + (t * 2.0 + j as f32).sin() * 10.0,
                y + dy * 40.0 - ph * 70.0,
                10.0 + ph * 18.0,
                rgba(230, 236, 244, 0.22 * (1.0 - ph) * k),
            );
        }
    }
}

/// The two legs meeting: hot into cold, a burst of steam over a ring half red, half blue, and a hatched core.
fn meet_glyph(g: &Pen, x: f32, y: f32, t: f32, k: f32) {
    let g = g.alpha(k);
    for j in 0..7 {
        let ph = (t * 0.8 + j as f32 / 7.0).rem_euclid(1.0);
        g.disc(
            x + (j as f32 * 2.1 + t).sin() * 22.0 * ph,
            y - ph * 80.0,
            10.0 + ph * 24.0,
            rgba(235, 240, 248, 0.5 * (1.0 - ph)),
        );
    }
    let r = 24.0 + 3.0 * (t * 8.0).sin();
    g.arc(x, y, r, PI / 2.0, PI * 1.5, 6.0, HOT);
    g.arc(x, y, r, -PI / 2.0, PI / 2.0, 6.0, COLD);
    hatch_disc(&g, x, y, r - 6.0, rgba(190, 159, 230, 0.8));
}

impl Game for Pipes {
    fn id(&self) -> &'static str {
        "pipes"
    }

    fn knobs(&self) -> &'static [&'static str] {
        &["grid_cols_count", "grid_rows_count", "tile_px", "crossings_count", "cracked_count", "bleed_hold_s"]
    }

    fn phases(&self, _rounds: u32) -> &'static [&'static str] {
        &["isolate", "rebuild", "bleed"]
    }

    fn step(&mut self, cx: &mut Ctx) {
        let mut r = cx.dice();
        self.hand_n = 0;
        self.hand_drag = None;
        // Each round is the whole job on a new segment (repair-minigames 1a): isolate, rebuild, refill and bleed. The
        // part step picks the segment its spool is for, and the round it opens keeps it.
        let fresh = cx.part || (cx.phase_name == "isolate" && !self.after_part) || self.job.is_none();
        if fresh {
            let leg = if r.f() < 0.5 { Leg::Hot } else { Leg::Cold };
            let seg = if r.f() < 0.5 { 2 } else { 3 };
            self.job = Some(Job { leg, seg });
        }
        self.after_part = cx.part;
        if cx.part {
            self.part_step(&mut r);
            return;
        }
        match cx.phase_name {
            "rebuild" => {
                let cols = cx.knob("grid_cols_count").round().max(3.0) as usize;
                let rows = cx.knob("grid_rows_count").round().max(3.0) as usize;
                let tt = cx.knob("tile_px");
                let want = cx.knob("crossings_count").round().max(1.0) as usize;
                let cracks = cx.knob("cracked_count").round().max(0.0) as usize;
                self.rebuild_step(&mut r, cols, rows, tt, want, cracks);
            }
            "bleed" => {
                self.phase = Ph::Bleed;
                self.keys = false;
                self.bleed = Bleed { air: 1.0, focus: 2, need: cx.knob("bleed_hold_s"), ..Bleed::default() };
            }
            _ => self.iso_step(),
        }
    }

    fn update(&mut self, cx: &mut Ctx, dt: f32, input: &Input) {
        match self.phase {
            Ph::Isolate => self.iso_update(cx, dt, input),
            Ph::Rebuild => self.rebuild_update(cx, dt, input),
            Ph::Bleed => self.bleed_update(cx, dt, input),
            Ph::Part => self.part_update(cx, dt, input),
        }
    }

    fn draw(&self, g: &Pen, _cx: &Ctx, t: f32, _input: &Input) {
        g.rect(0.0, BAR_H, W, 720.0, hex(0x070b12));
        match self.phase {
            Ph::Isolate => self.iso_draw(g, t),
            Ph::Rebuild => self.rebuild_draw(g, t),
            Ph::Bleed => self.bleed_draw(g, t),
            Ph::Part => self.part_draw(g),
        }
    }

    fn hand(&mut self, _cx: &Ctx, _t: f32) -> Input {
        self.hand_n += 1;
        // A drag under way: carry it 10 px a frame, let go on its mark.
        if let Some((at, to)) = self.hand_drag {
            let d = (to[0] - at[0]).hypot(to[1] - at[1]);
            if d > 1.0 {
                let k = (10.0 / d).min(1.0);
                let next = [at[0] + (to[0] - at[0]) * k, at[1] + (to[1] - at[1]) * k];
                self.hand_drag = Some((next, to));
                return Input::hold(next[0], next[1], false);
            }
            self.hand_drag = None;
            self.hand_n = 0;
            return Input::release(to[0], to[1]);
        }
        // A tap every fifth of a second, a press on one frame and nothing on the next.
        let tap_now = self.hand_n >= 12;
        let tap = |this: &mut Self, x: f32, y: f32| {
            this.hand_n = 0;
            Input::hold(x, y, true)
        };
        match self.phase {
            Ph::Isolate => {
                if self.done || !tap_now {
                    return Input::default();
                }
                // The leg's bypass open first, then the valves either side of the crack.
                let j = self.job();
                let by = (0..VALVES.len()).find(|&i| VALVES[i].bypass && VALVES[i].leg == j.leg).unwrap_or(0);
                if is_shut(self.shut, by) {
                    return tap(self, VALVES[by].x, VALVES[by].y);
                }
                match self.iso_need().into_iter().find(|&i| !is_shut(self.shut, i)) {
                    Some(i) => tap(self, VALVES[i].x, VALVES[i].y),
                    None => Input::default(),
                }
            }
            Ph::Rebuild => {
                let s = &self.grid;
                if s.phase != Flow::Play || !tap_now || s.held.is_some() {
                    return Input::default();
                }
                // A new piece onto each cracked tile, the one the run needs there.
                if let Some(i) = s.tiles.iter().position(|t| t.cracked) {
                    let slot = TRAY.iter().position(|&k| k == s.tiles[i].need_kind).unwrap_or(0);
                    let from = slot_xy(slot);
                    let to = s.cell_xy(i % s.cols, i / s.cols);
                    self.hand_drag = Some((from, to));
                    return Input::hold(from[0], from[1], true);
                }
                // Each tile of the runs turned to lie as the run needs it, then FILL.
                if let Some(i) = s.tiles.iter().position(|t| t.sol && !t.right()) {
                    let [x, y] = s.cell_xy(i % s.cols, i / s.cols);
                    return tap(self, x, y);
                }
                let [fx, fy, fw, fh] = FILL_BTN;
                tap(self, fx + fw / 2.0, fy + fh / 2.0)
            }
            Ph::Bleed => {
                let s = &self.bleed;
                if s.played {
                    return Input::default();
                }
                let [up, bleed, dn] = targets();
                if !s.dn {
                    return if tap_now { tap(self, dn[0], dn[1]) } else { Input::default() };
                }
                if !s.up {
                    return if tap_now { tap(self, up[0], up[1]) } else { Input::default() };
                }
                if s.fill < 1.0 {
                    return Input::default();
                }
                // Hold the bleed valve open until the needle settles.
                let pressed = !s.holding;
                Input::hold(bleed[0], bleed[1], pressed)
            }
            Ph::Part => {
                let s = &self.part;
                if s.set || !tap_now {
                    return Input::default();
                }
                let leg = self.job().leg;
                let Some(sp) = s.spools.iter().find(|sp| sp.leg == leg) else { return Input::default() };
                let from = [sp.x, sp.y];
                self.hand_drag = Some((from, [GAP[0] + GAP[2] / 2.0, GAP[1]]));
                Input::hold(from[0], from[1], true)
            }
        }
    }

    fn guide_now(&self) -> Option<usize> {
        match self.phase {
            Ph::Isolate => Some(0),
            Ph::Rebuild => Some(if self.grid.phase == Flow::Play { 1 } else { 2 }),
            Ph::Bleed => Some(3),
            Ph::Part => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::games::tests::{fumble_check, plays_to_end};
    use crate::kit::Input;

    #[test]
    fn a_steady_hand_repairs_it_damaged_disabled_and_destroyed() {
        plays_to_end("pipes");
    }

    #[test]
    fn shutting_a_main_valve_starves_the_core() {
        // A tap on the cold leg's main valve at the core.
        fumble_check("pipes", |_r| Input::hold(500.0, 150.0, true));
    }
}
