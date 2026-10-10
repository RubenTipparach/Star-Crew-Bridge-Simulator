//! The heat exchanger's repair, the chiller between the coolant loop and the radiators (repair-minigames design 6h),
//! from `docs/mockups/repairs/chiller.js`.
//!
//! A plate exchanger: a pack of seven plates between a fixed frame and a pressure plate, held by a top and a bottom tie
//! bolt. A round is two phases, in order:
//!
//! - **scrub**: the fouled plate on the bench, face on, grey scale over its chevrons, a black rubber gasket round its
//!   edge and its two ports. Drag the brush over it (or move it with the arrows and hold Space): the scale under it
//!   comes off. The brush held on the rubber 0.3 s is the fumble "Torn gasket: coolant weeps". 95% off: clean.
//! - **restack**: the pack's plates must alternate, chevrons up and down: tap a plate hung the wrong way to turn it
//!   over. Then close the pack to its mark: tap a tie bolt's nut to turn it a quarter, which moves its end of the
//!   pressure plate in. The two ends more than two quarters apart is the fumble "Skewed pack: a plate cracks", and that
//!   nut backs off. The nuts do not turn until the plates alternate.
//!
//! Each level puts more scale on the plate, and from level 3 two plates hang the wrong way. A disabled chiller's first
//! step fits a new plate: drag it from the crate into the gap in the pack.

use std::collections::BTreeSet;
use std::f32::consts::TAU;

use egui::Key;

use crate::kit::{nearest, Ctx, Dice, Game, Input, BAR_H, TOUCH_R, W};
use crate::pen::{c, hex, rgba, Align, Pen, PipeStyle};

/// In the engine.
pub const PORTED: bool = true;

/// A fresh game.
pub fn new() -> Box<dyn Game> {
    Box::new(Chiller::default())
}

const TORN: &str = "Torn gasket: coolant weeps";
const SKEW: &str = "Skewed pack: a plate cracks";

// ---- scrub: the plate on the bench (canvas px).
/// The plate: x, y, w, h.
const PL: [f32; 4] = [470.0, 102.0, 340.0, 586.0];
/// The gasket's line, inset from the plate's edge.
const GASKET_IN: f32 = 18.0;
/// The brush is on the gasket within this of its line.
const GASKET_HIT: f32 = 10.0;
/// The two ports.
const PORTS: [[f32; 2]; 2] = [[PL[0] + 78.0, PL[1] + 80.0], [PL[0] + PL[2] - 78.0, PL[1] + PL[3] - 80.0]];
const PORT_R: f32 = 34.0;
/// The gasket ring round each port.
const PORT_GASKET: f32 = 48.0;
/// The scale's grid.
const CELL: f32 = 12.0;
const BRUSH_R: f32 = 36.0;
/// The share of the scale off that counts as clean.
const CLEAN: f32 = 0.95;
/// Seconds on the rubber that tear it.
const TEAR_S: f32 = 0.3;

// ---- restack: the pack from the side.
const FRAME_X: f32 = 200.0;
const BOLT_Y: [f32; 2] = [168.0, 572.0];
const PLATE_Y: [f32; 2] = [214.0, 526.0];
/// Plates in the pack.
const N: usize = 7;
/// Px a quarter turn moves an end of the pressure plate.
const Q: f32 = 8.0;
/// Quarters each end has to close.
const Q0: i32 = 6;
/// The most the two ends may differ, quarters.
const SKEW_MAX: i32 = 2;
/// The pressure plate's mark when the pack is closed.
const MARK: f32 = 760.0;
/// A plate's width, side on.
const PW: f32 = 40.0;
/// The part step's crate: x, y, w, h.
const CRATE: [f32; 4] = [1020.0, 190.0, 170.0, 400.0];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Ph {
    #[default]
    Scrub,
    Restack,
    Part,
}

#[derive(Clone, Copy, Debug)]
struct Cell {
    x: f32,
    y: f32,
    on: bool,
    shade: f32,
}

#[derive(Clone, Copy, Debug, Default)]
struct Plate {
    up: bool,
    flip: f32,
}

/// The game's state.
#[derive(Default)]
pub struct Chiller {
    phase: Ph,
    done: bool,
    keys: bool,
    // scrub
    cells: Vec<Cell>,
    total: usize,
    left: usize,
    cur: [f32; 2],
    brushing: bool,
    on_gasket: bool,
    tear_t: f32,
    tears: Vec<[f32; 2]>,
    lock: bool,
    shine: f32,
    // restack and part
    plates: Vec<Plate>,
    q: [i32; 2],
    focus: usize,
    kick: f32,
    turn: [f32; 2],
    gap: Option<usize>,
    new_plate: [f32; 2],
    held: bool,
    off: [f32; 2],
    /// The hand: frames since its last tap, and whether it is carrying the new plate.
    hand_n: u32,
    hand_carry: bool,
}

/// How far a point is from the gasket's line (its edge run and its two port rings), px.
fn gasket_dist(x: f32, y: f32) -> f32 {
    let (x0, x1, y0, y1) = (PL[0] + GASKET_IN, PL[0] + PL[2] - GASKET_IN, PL[1] + GASKET_IN, PL[1] + PL[3] - GASKET_IN);
    let inside = x > x0 && x < x1 && y > y0 && y < y1;
    let edge = if inside {
        (x - x0).min(x1 - x).min(y - y0).min(y1 - y)
    } else {
        (x0 - x).max(0.0).max(x - x1).hypot((y0 - y).max(0.0).max(y - y1))
    };
    PORTS.iter().fold(edge, |m, [px, py]| m.min(((x - px).hypot(y - py) - PORT_GASKET).abs()))
}

impl Chiller {
    fn scrub_step(&mut self, r: &mut Dice, patches: usize) {
        // Scale: blobs over the plate's field, kept clear of the gasket by a brush's reach.
        let [px, py, pw, ph] = PL;
        let cols = ((pw - 2.0 * 36.0) / CELL).floor() as usize;
        let rows = ((ph - 2.0 * 36.0) / CELL).floor() as usize;
        let blobs: Vec<[f32; 3]> = (0..patches)
            .map(|_| [px + 40.0 + r.f() * (pw - 80.0), py + 40.0 + r.f() * (ph - 80.0), 56.0 + r.f() * 50.0])
            .collect();
        self.cells.clear();
        for j in 0..rows {
            for i in 0..cols {
                let (x, y) = (px + 36.0 + (i as f32 + 0.5) * CELL, py + 36.0 + (j as f32 + 0.5) * CELL);
                if PORTS.iter().any(|[qx, qy]| (x - qx).hypot(y - qy) < PORT_GASKET + 14.0) {
                    continue;
                }
                if gasket_dist(x, y) < 18.0 {
                    continue;
                }
                let k = blobs.iter().fold(0.0f32, |m, b| m.max(1.0 - (x - b[0]).hypot(y - b[1]) / b[2]));
                if k > 0.0 {
                    self.cells.push(Cell { x, y, on: true, shade: 0.55 + 0.45 * (k * 2.0).min(1.0) });
                }
            }
        }
        self.phase = Ph::Scrub;
        self.total = self.cells.len();
        self.left = self.total;
        self.cur = [px + pw / 2.0, py + ph / 2.0];
        self.keys = false;
        self.brushing = false;
        self.on_gasket = false;
        self.tear_t = 0.0;
        self.tears.clear();
        self.lock = false;
        self.done = false;
        self.shine = 0.0;
    }

    fn scrub_at(&mut self, x: f32, y: f32) {
        for c in &mut self.cells {
            if c.on && (c.x - x).hypot(c.y - y) <= BRUSH_R {
                c.on = false;
                self.left -= 1;
            }
        }
    }

    fn scrub_update(&mut self, cx: &mut Ctx, dt: f32, input: &Input) {
        if self.done {
            self.shine = (self.shine + dt * 2.0).min(1.0);
            return;
        }
        let [px, py, pw, ph] = PL;
        let mut pts: Vec<[f32; 2]> = Vec::new();
        if input.stick != [0.0, 0.0] {
            self.keys = true;
            self.cur[0] = (self.cur[0] + input.stick[0] * 420.0 * dt).clamp(px, px + pw);
            self.cur[1] = (self.cur[1] + input.stick[1] * 420.0 * dt).clamp(py, py + ph);
        }
        if self.keys && input.action {
            pts = vec![self.cur];
        }
        if input.down {
            self.keys = false;
            self.cur = [input.x, input.y];
            pts = if input.path.is_empty() { vec![[input.x, input.y]] } else { input.path.clone() };
        }
        if !input.down && !input.action {
            self.lock = false;
        }
        self.brushing = !pts.is_empty() && !self.lock;
        if !self.brushing {
            self.on_gasket = false;
            self.tear_t = (self.tear_t - dt).max(0.0);
            return;
        }
        for [x, y] in &pts {
            self.scrub_at(*x, *y);
        }
        // The brush on the rubber: a moment is a scuff, held there it tears.
        let [lx, ly] = pts[pts.len() - 1];
        self.on_gasket = gasket_dist(lx, ly) < GASKET_HIT;
        self.tear_t = if self.on_gasket { self.tear_t + dt } else { (self.tear_t - dt).max(0.0) };
        if self.tear_t >= TEAR_S {
            self.tear_t = 0.0;
            self.lock = true;
            self.tears.push([lx, ly]);
            cx.fumble(TORN);
            return;
        }
        if self.left as f32 <= self.total as f32 * (1.0 - CLEAN) {
            for c in &mut self.cells {
                c.on = false;
            }
            self.left = 0;
            self.done = true;
            cx.step_done();
        }
    }

    fn alternate(&self) -> bool {
        self.plates.windows(2).all(|w| w[0].up != w[1].up)
    }

    /// The pressure plate's top (0) and bottom (1) faces.
    fn end_x(&self, k: usize) -> f32 {
        MARK + Q * self.q[k] as f32
    }

    fn plate_x(&self, i: usize) -> f32 {
        let right = self.end_x(0).min(self.end_x(1)) - 34.0;
        FRAME_X + 70.0 + (i as f32 * (right - FRAME_X - 70.0 - PW)) / (N - 1) as f32
    }

    fn nut_xy(&self, k: usize) -> [f32; 2] {
        [self.end_x(0).max(self.end_x(1)) + 52.0, BOLT_Y[k]]
    }

    fn restack_step(&mut self, r: &mut Dice, part: bool, wrong: usize) {
        let start = r.f() < 0.5;
        self.plates = (0..N).map(|i| Plate { up: (i % 2 == 0) == start, flip: 0.0 }).collect();
        self.phase = if part { Ph::Part } else { Ph::Restack };
        self.q = if part { [0, 0] } else { [Q0, Q0] };
        self.focus = 0;
        self.keys = false;
        self.kick = 0.0;
        self.done = false;
        self.turn = [0.0, 0.0];
        self.gap = None;
        self.held = false;
        if part {
            self.gap = Some(1 + r.int(N - 2));
            self.new_plate = [CRATE[0] + CRATE[2] / 2.0, CRATE[1] + CRATE[3] / 2.0];
            return;
        }
        // One or two plates hung the wrong way.
        let mut picked = BTreeSet::new();
        while picked.len() < wrong.min(N) {
            picked.insert(r.int(N));
        }
        for i in picked {
            self.plates[i].up = !self.plates[i].up;
        }
        if self.alternate() {
            self.plates[0].up = !self.plates[0].up;
        }
    }

    fn tap_plate(&mut self, i: usize) {
        self.plates[i].up = !self.plates[i].up;
        self.plates[i].flip = 1.0;
    }

    fn tap_nut(&mut self, cx: &mut Ctx, k: usize) {
        if !self.alternate() {
            self.kick = 1.0;
            cx.say("Plates first: up, down, up");
            return;
        }
        if self.q[k] <= 0 {
            return;
        }
        self.q[k] -= 1;
        self.turn[k] += TAU / 4.0;
        if (self.q[0] - self.q[1]).abs() > SKEW_MAX {
            self.q[k] += 1;
            self.kick = 1.0;
            cx.fumble(SKEW);
            return;
        }
        if self.q == [0, 0] {
            self.done = true;
            cx.step_done();
        }
    }

    fn restack_update(&mut self, cx: &mut Ctx, dt: f32, input: &Input) {
        self.kick = (self.kick - dt * 3.0).max(0.0);
        for p in &mut self.plates {
            p.flip = (p.flip - dt * 4.0).max(0.0);
        }
        if self.done {
            return;
        }
        let items = N + 2;
        for (k, d) in
            [(Key::ArrowLeft, items - 1), (Key::A, items - 1), (Key::ArrowRight, 1), (Key::D, 1), (Key::Tab, 1)]
        {
            if input.hit(k) {
                self.keys = true;
                self.focus = (self.focus + d) % items;
            }
        }
        if input.action_pressed {
            self.keys = true;
            if self.focus < N {
                self.tap_plate(self.focus);
            } else {
                self.tap_nut(cx, self.focus - N);
            }
            return;
        }
        if !input.pressed {
            return;
        }
        let nuts = [Some(self.nut_xy(0)), Some(self.nut_xy(1))];
        if let Some(k) = nearest(&nuts, input.x, input.y, TOUCH_R + 14.0) {
            self.keys = false;
            self.focus = N + k;
            self.tap_nut(cx, k);
            return;
        }
        for i in 0..N {
            let x = self.plate_x(i);
            if input.x >= x - 10.0
                && input.x <= x + PW + 10.0
                && input.y >= PLATE_Y[0] - 10.0
                && input.y <= PLATE_Y[1] + 10.0
            {
                self.keys = false;
                self.focus = i;
                self.tap_plate(i);
                return;
            }
        }
    }

    fn gap_xy(&self) -> Option<[f32; 2]> {
        self.gap.map(|g| [self.plate_x(g) + PW / 2.0, (PLATE_Y[0] + PLATE_Y[1]) / 2.0])
    }

    fn part_update(&mut self, cx: &mut Ctx, dt: f32, input: &Input) {
        if self.done {
            return;
        }
        let m = &mut self.new_plate;
        if input.stick != [0.0, 0.0] {
            self.keys = true;
            m[0] += input.stick[0] * 480.0 * dt;
            m[1] += input.stick[1] * 480.0 * dt;
        }
        if input.pressed && (input.x - m[0]).abs() < PW && (input.y - m[1]).abs() < 120.0 {
            self.held = true;
            self.off = [input.x - m[0], input.y - m[1]];
        }
        if self.held && input.down {
            *m = [input.x - self.off[0], input.y - self.off[1]];
        }
        if !((self.held && input.released) || (self.keys && input.action_pressed)) {
            return;
        }
        self.held = false;
        let Some([gx, gy]) = self.gap_xy() else { return };
        let m = self.new_plate;
        if (m[0] - gx).abs() < 50.0 && (m[1] - gy).abs() < 140.0 {
            self.gap = None;
            self.done = true;
            cx.step_done();
            return;
        }
        self.new_plate = [CRATE[0] + CRATE[2] / 2.0, CRATE[1] + CRATE[3] / 2.0];
    }

    fn scrub_draw(&self, g: &Pen, t: f32) {
        let [px, py, pw, ph] = PL;
        // The plate: steel, its chevron pressing, the gasket, the ports.
        g.round(px, py, pw, ph, 18.0, Some(hex(0x5d6b80)), Some((3.0, hex(0x8796aa))));
        let field = g.clip(px + 30.0, py + 30.0, pw - 60.0, ph - 60.0);
        let mut y = py - 200.0;
        while y < py + ph + 200.0 {
            field.path(&[[px, y], [px + pw / 2.0, y + 60.0], [px + pw, y]], false, 5.0, hex(0x71809a));
            y += 26.0;
        }
        // The scale: grey crust over the steel, left where the brush has not been.
        for c in self.cells.iter().filter(|c| c.on) {
            g.disc(c.x, c.y, CELL * 0.78, rgba(186, 180, 164, c.shade));
        }
        for [qx, qy] in PORTS {
            g.disc(qx, qy, PORT_R, hex(0x0b1018));
        }
        // The gasket: black rubber, red where it is being scrubbed.
        let hot = self.on_gasket && self.brushing;
        let rubber = if hot { c::DANGER } else { hex(0x111418) };
        g.round(
            px + GASKET_IN,
            py + GASKET_IN,
            pw - 2.0 * GASKET_IN,
            ph - 2.0 * GASKET_IN,
            10.0,
            None,
            Some((12.0, rubber)),
        );
        for [qx, qy] in PORTS {
            g.ring(qx, qy, PORT_GASKET, rubber, 12.0);
        }
        for [x, y] in &self.tears {
            g.cross(*x, *y, 8.0, 3.0, c::DANGER);
        }
        if self.shine > 0.0 {
            let a = 0.25 * (1.0 - (self.shine * 2.0 - 1.0).abs());
            g.round(px, py, pw, ph, 18.0, Some(rgba(232, 246, 255, a)), None);
        }
        // How much is left: a bar beside the plate, the one number a player reads.
        let k = if self.total > 0 { 1.0 - self.left as f32 / self.total as f32 } else { 1.0 };
        g.round(px + pw + 60.0, py + 40.0, 36.0, ph - 80.0, 18.0, Some(hex(0x141c28)), None);
        let hh = (ph - 88.0) * (k / CLEAN).min(1.0);
        g.round(
            px + pw + 64.0,
            py + ph - 44.0 - hh,
            28.0,
            hh,
            14.0,
            Some(if k >= CLEAN { c::OK } else { c::ACCENT }),
            None,
        );
        g.text("CLEAN", px + pw + 78.0, py + 18.0, 18.0, c::DIM, Align::Center);
        // The brush.
        if self.brushing || self.keys {
            let [x, y] = self.cur;
            g.ring(x, y, BRUSH_R, if self.on_gasket { c::DANGER } else { rgba(232, 238, 246, 0.7) }, 3.0);
            for k2 in 0..10 {
                let a = (k2 as f32 / 10.0) * TAU + t * if self.brushing { 8.0 } else { 0.0 };
                let r = BRUSH_R - 6.0;
                g.line(x, y, x + a.cos() * r, y + a.sin() * r, 2.0, rgba(242, 160, 70, 0.8));
            }
        }
    }

    fn pack_draw(&self, g0: &Pen, t: f32) {
        let shake = if self.kick > 0.0 { (t * 60.0).sin() * 5.0 * self.kick } else { 0.0 };
        let g = g0.translate(shake, 0.0);
        // The fixed frame on the left, the tie bolts along the top and bottom.
        g.round(FRAME_X - 40.0, 120.0, 70.0, 500.0, 10.0, Some(hex(0x2a3446)), Some((3.0, hex(0x4a586d))));
        let bolt_end = self.end_x(0).max(self.end_x(1)) + 110.0;
        for y in BOLT_Y {
            g.rect(FRAME_X, y - 6.0, bolt_end - FRAME_X, 12.0, hex(0x9aa6b6));
        }
        // The ports' pipes into the frame: hot in, cold out.
        let style = |col| PipeStyle { w: 30.0, fluid: Some(col), ..PipeStyle::default() };
        g.pipe(&[(vec![[40.0, 260.0], [FRAME_X - 40.0, 260.0]], 1.0)], &style(hex(0xff6a4d)));
        g.pipe(&[(vec![[40.0, 480.0], [FRAME_X - 40.0, 480.0]], 1.0)], &style(hex(0x4fa8f7)));
        // The mark the pressure plate closes to.
        g.dashed(&[[MARK, 110.0], [MARK, 630.0]], 3.0, c::OK, 10.0, 8.0);
        // The plates.
        let alt = self.alternate();
        for (i, p) in self.plates.iter().enumerate() {
            if self.phase == Ph::Part && self.gap == Some(i) {
                continue;
            }
            plate_glyph(&g, self.plate_x(i), p.up, self.keys && self.focus == i, p.flip);
        }
        if let (Ph::Part, Some(gp)) = (self.phase, self.gap) {
            let pts = crate::pen::round_rect_points(self.plate_x(gp), PLATE_Y[0], PW, PLATE_Y[1] - PLATE_Y[0], 6.0);
            let mut closed = pts.clone();
            closed.push(pts[0]);
            g.dashed(&closed, 3.0, c::AMBER, 8.0, 6.0);
        }
        // The pressure plate: its top and bottom ends where their nuts have brought them.
        let (xt, xb) = (self.end_x(0), self.end_x(1));
        let pp = [[xt, 130.0], [xt + 40.0, 130.0], [xb + 40.0, 610.0], [xb, 610.0]];
        g.poly(&pp, hex(0x33405a));
        let skewed = (self.q[0] - self.q[1]).abs() == SKEW_MAX;
        g.path(&pp, true, 3.0, if skewed { c::WARN } else { hex(0x5a6a82) });
        // The nuts: a hexagon each, locked (dim) until the plates alternate.
        for k in 0..2 {
            let [nx, ny] = self.nut_xy(k);
            let sel = self.keys && self.focus == N + k;
            let on = alt && self.phase == Ph::Restack;
            let hexa: Vec<[f32; 2]> = (0..6)
                .map(|j| {
                    let a = (j as f32 / 6.0) * TAU + self.turn[k];
                    [nx + a.cos() * 30.0, ny + a.sin() * 30.0]
                })
                .collect();
            g.poly(&hexa, if on { hex(0xb7c0cc) } else { hex(0x4a5566) });
            g.path(&hexa, true, if sel { 4.0 } else { 2.0 }, if sel { c::AMBER } else { hex(0x2a313c) });
            g.disc(nx, ny, 10.0, hex(0x2a313c));
            if on && self.q[k] > 0 {
                g.turn_arrow(nx, ny, 44.0, -2.2, -0.9, rgba(232, 238, 246, 0.6));
            }
            // How many quarters this end still has to go: a short row of dots by the nut.
            for j in 0..self.q[k].max(0) {
                let dy = if k == 1 { 46.0 } else { -46.0 };
                g.disc(nx - 30.0 + j as f32 * 12.0, ny + dy, 4.0, if on { c::FG } else { hex(0x4a5566) });
            }
        }
        if self.phase == Ph::Part {
            let [x, y, w, h] = CRATE;
            g0.panel(x, y, w, h, 12.0, hex(0x141c28), hex(0x3a4658));
            if self.gap.is_some() {
                let [mx, my] = self.new_plate;
                let p = g0.translate(mx - PW / 2.0, my - (PLATE_Y[1] - PLATE_Y[0]) / 2.0 - PLATE_Y[0]);
                plate_glyph(&p, 0.0, true, self.held, 0.0);
            }
        }
    }
}

/// A plate side on, with its chevron: up (a peak) or down (a valley), and its colour so the alternation reads.
fn plate_glyph(g: &Pen, x: f32, up: bool, sel: bool, flip: f32) {
    let (y0, y1, cx, sq) = (PLATE_Y[0], PLATE_Y[1], x + PW / 2.0, 1.0 - 0.8 * flip);
    g.round(
        x + (PW * (1.0 - sq)) / 2.0,
        y0,
        PW * sq,
        y1 - y0,
        6.0,
        Some(if up { hex(0x5d6b80) } else { hex(0x4a586d) }),
        Some((if sel { 4.0 } else { 2.0 }, if sel { c::AMBER } else { hex(0x8796aa) })),
    );
    let col = if up { c::ACCENT } else { c::AMBER };
    let d = if up { 10.0 } else { -10.0 };
    for yy in [y0 + 70.0, (y0 + y1) / 2.0, y1 - 70.0] {
        g.path_round(&[[cx - 12.0, yy + d], [cx, yy - d], [cx + 12.0, yy + d]], 5.0, col);
    }
}

impl Game for Chiller {
    fn id(&self) -> &'static str {
        "chiller"
    }

    fn knobs(&self) -> &'static [&'static str] {
        &["scale_patches_count", "wrong_plates_count"]
    }

    fn phases(&self, _rounds: u32) -> &'static [&'static str] {
        &["scrub", "restack"]
    }

    fn step(&mut self, cx: &mut Ctx) {
        let mut r = cx.dice();
        self.hand_n = 0;
        self.hand_carry = false;
        // A round scrubs the plate, then restacks the pack (repair-minigames 1a); the part opens the first round.
        if cx.part || cx.phase_name == "restack" {
            let wrong = cx.knob("wrong_plates_count").round().max(0.0) as usize;
            self.restack_step(&mut r, cx.part, wrong);
        } else {
            let patches = cx.knob("scale_patches_count").round().max(1.0) as usize;
            self.scrub_step(&mut r, patches);
        }
    }

    fn update(&mut self, cx: &mut Ctx, dt: f32, input: &Input) {
        match self.phase {
            Ph::Scrub => self.scrub_update(cx, dt, input),
            Ph::Part => self.part_update(cx, dt, input),
            Ph::Restack => self.restack_update(cx, dt, input),
        }
    }

    fn draw(&self, g: &Pen, _cx: &Ctx, t: f32, _input: &Input) {
        g.rect(0.0, BAR_H, W, 720.0, hex(0x070b12));
        if self.phase == Ph::Scrub {
            self.scrub_draw(g, t);
        } else {
            self.pack_draw(g, t);
        }
    }

    fn hand(&mut self, _cx: &Ctx, _t: f32) -> Input {
        self.hand_n += 1;
        match self.phase {
            Ph::Scrub => {
                if self.done {
                    return Input::default();
                }
                // Brush towards the nearest scale left, 20 px a frame, on the field only (the cells keep clear of
                // the rubber).
                let [x, y] = self.cur;
                let Some(c) = self
                    .cells
                    .iter()
                    .filter(|c| c.on)
                    .min_by(|a, b| (a.x - x).hypot(a.y - y).total_cmp(&(b.x - x).hypot(b.y - y)))
                else {
                    return Input::default();
                };
                let d = (c.x - x).hypot(c.y - y);
                let k = if d > 20.0 { 20.0 / d } else { 1.0 };
                let (nx, ny) = (x + (c.x - x) * k, y + (c.y - y) * k);
                Input::hold(nx, ny, self.hand_n == 1)
            }
            Ph::Restack => {
                if self.done {
                    return Input::default();
                }
                // A tap every quarter second: press on one frame, let go on the next.
                if self.hand_n % 15 != 0 {
                    return Input::default();
                }
                // The plates first: flip whichever of the two alternating patterns needs fewer turns.
                let want = |first: bool, i: usize| (i % 2 == 0) == first;
                let wrong = |first: bool| (0..N).filter(|&i| self.plates[i].up != want(first, i)).count();
                let first = wrong(true) <= wrong(false);
                if let Some(i) = (0..N).find(|&i| self.plates[i].up != want(first, i)) {
                    let x = self.plate_x(i) + PW / 2.0;
                    return Input::hold(x, (PLATE_Y[0] + PLATE_Y[1]) / 2.0, true);
                }
                // Then the nuts in turn, the end further out first.
                let k = usize::from(self.q[1] > self.q[0]);
                let [nx, ny] = self.nut_xy(k);
                Input::hold(nx, ny, true)
            }
            Ph::Part => {
                if self.done {
                    return Input::default();
                }
                let Some([gx, gy]) = self.gap_xy() else { return Input::default() };
                let [mx, my] = self.new_plate;
                if !self.held && !self.hand_carry {
                    self.hand_carry = true;
                    return Input::hold(mx, my, true);
                }
                let d = (gx - mx).hypot(gy - my);
                if d > 2.0 {
                    let k = (10.0 / d).min(1.0);
                    return Input::hold(mx + (gx - mx) * k, my + (gy - my) * k, false);
                }
                self.hand_carry = false;
                Input::release(gx, gy)
            }
        }
    }

    fn guide_now(&self) -> Option<usize> {
        match self.phase {
            Ph::Scrub => Some(usize::from(self.on_gasket)),
            Ph::Restack => Some(if self.alternate() { 3 } else { 2 }),
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
        plays_to_end("chiller");
    }

    #[test]
    fn the_brush_held_on_the_rubber_tears_the_gasket() {
        // The brush parked on the gasket's line down the plate's left edge (x 470 + 18).
        fumble_check("chiller", |_r| Input::hold(488.0, 400.0, false));
    }
}
