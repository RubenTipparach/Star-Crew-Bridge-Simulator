//! A breaker's repair at the main switchboard (repair-minigames design 2), from `docs/mockups/repairs/breakers.js`.
//!
//! The switchboard's cubicle with its draw-out breaker, the bus's rating plate above it, the synchroscope and the
//! close button on the right, the fuse tray below them. Rack the breaker out (drag its handle down, or hold the down
//! arrow), and its fuse cover opens on the blown cartridge. Fit the cartridge whose rating matches the bus (drag it
//! from the tray, or up and down to choose and Space to fit it), rack the breaker back in (drag up, or hold up), and
//! close it (the button, or Space) while the synchroscope's needle sits in its green wedge and the lamp shows green.
//! A round is one breaker; each level has more cartridges, a faster needle and a narrower wedge. Closing on red is a
//! fumble: an arc flash, 10 HP. A wrong cartridge is a fumble too: it blows as it seats. A disabled board's first
//! step fits the new breaker: drag it off the trolley into the empty cubicle.
//!
//! This file also holds [`Hand`], the steady hand's pointer moves that this game and the turret, tubes, scrubbers,
//! sensors and shields share, until the kit takes it (design 6a, task 1.5: "a drag helper").

use egui::Color32;

use crate::kit::{Ctx, Game, Input, BAR_H, W};
use crate::pen::{c, hex, rgba, round_rect_points, Align, Pen};

/// In the engine.
pub const PORTED: bool = true;

/// A fresh game.
pub fn new() -> Box<dyn Game> {
    Box::new(Breakers::default())
}

// ------------------------------------------------------------------------------------------------ the shared hand

/// The steady hand's pointer, for the scripted hands of several games: press, carry a thing at a steady pace and let
/// go, hold, tap. Each move is a task; starting a new task lets go of the old one first, so a press never lands
/// while the pointer is still down. It belongs in the kit beside [`crate::kit::nearest`] and lives here until then.
#[derive(Clone, Debug, Default)]
pub(crate) struct Hand {
    p: Option<[f32; 2]>,
    task: u32,
    settled: bool,
}

/// The pace a part is carried from its crate, px a frame (360 px/s).
pub(crate) const PART_STEP: f32 = 6.0;

impl Hand {
    /// True on the step's first frame, when the hand rests (no keys, nothing pressed) before it starts: a person's
    /// beat, and the kit's guide hold lets go only on such a frame.
    pub(crate) fn resting(&mut self) -> bool {
        !std::mem::replace(&mut self.settled, true)
    }

    /// Let go of an old task when `task` is new; the release to send this frame, if any.
    fn switch(&mut self, task: u32) -> Option<Input> {
        if self.task == task {
            return None;
        }
        self.task = task;
        self.p.take().map(|p| Input::release(p[0], p[1]))
    }

    /// Carry from `from` to `to` at `step` px a frame as task `task`: press at `from`, move, let go at `to`.
    pub(crate) fn carry(&mut self, task: u32, from: [f32; 2], to: [f32; 2], step: f32) -> Input {
        if let Some(i) = self.switch(task) {
            return i;
        }
        let Some(p) = self.p else {
            self.p = Some(from);
            return Input::hold(from[0], from[1], true);
        };
        let d = (to[0] - p[0]).hypot(to[1] - p[1]);
        if d > 0.01 {
            let k = (step / d).min(1.0);
            let q = [p[0] + (to[0] - p[0]) * k, p[1] + (to[1] - p[1]) * k];
            self.p = Some(q);
            return Input::hold(q[0], q[1], false);
        }
        self.p = None;
        Input::release(to[0], to[1])
    }

    /// Hold the pointer down at `at` as task `task`: pressed on its first frame, then held there.
    pub(crate) fn hold(&mut self, task: u32, at: [f32; 2]) -> Input {
        if let Some(i) = self.switch(task) {
            return i;
        }
        let first = self.p.is_none();
        self.p = Some(at);
        Input::hold(at[0], at[1], first)
    }

    /// Tap at `at` as task `task`: a press one frame, the release the next.
    pub(crate) fn tap(&mut self, task: u32, at: [f32; 2]) -> Input {
        if let Some(i) = self.switch(task) {
            return i;
        }
        match self.p.take() {
            Some(p) => Input::release(p[0], p[1]),
            None => {
                self.p = Some(at);
                Input::hold(at[0], at[1], true)
            }
        }
    }

    /// Nothing in hand: let go if the pointer is down, else no input.
    pub(crate) fn idle(&mut self) -> Input {
        self.switch(u32::MAX).unwrap_or_default()
    }
}

/// An angle wrapped into -pi to pi.
pub(crate) fn wrap(a: f32) -> f32 {
    a.sin().atan2(a.cos())
}

/// The outline of an ellipse round (x, y), radii rx and ry (canvas's `ellipse`).
pub(crate) fn ellipse(x: f32, y: f32, rx: f32, ry: f32) -> Vec<[f32; 2]> {
    (0..40)
        .map(|i| {
            let a = std::f32::consts::TAU * i as f32 / 40.0;
            [x + a.cos() * rx, y + a.sin() * ry]
        })
        .collect()
}

/// A closed outline as an open polyline back to its first point (for a dashed or stroked loop).
pub(crate) fn closed(mut pts: Vec<[f32; 2]>) -> Vec<[f32; 2]> {
    if let Some(p) = pts.first().copied() {
        pts.push(p);
    }
    pts
}

/// A pie slice's outline: the centre, then the arc from `a0` to `a1` (filled as a polygon; `Pen::sector` with an
/// inner radius of 0 gives its inner arc fewer points than its outer one and draws stray triangles).
pub(crate) fn pie(x: f32, y: f32, r: f32, a0: f32, a1: f32) -> Vec<[f32; 2]> {
    let mut pts = vec![[x, y]];
    pts.extend(Pen::arc_points(x, y, r, a0, a1));
    pts
}

/// A dashed circle (canvas's `setLineDash` round an arc).
#[allow(clippy::too_many_arguments)]
pub(crate) fn dashed_ring(g: &Pen, x: f32, y: f32, r: f32, col: Color32, width: f32, dash: f32, gap: f32) {
    g.dashed(&Pen::arc_points(x, y, r, 0.0, std::f32::consts::TAU), width, col, dash, gap);
}

// ------------------------------------------------------------------------------------------------ the game

const CAB: [f32; 4] = [120.0, 92.0, 660.0, 610.0];
const CUB: [f32; 4] = [200.0, 196.0, 500.0, 460.0];
const UNIT_W: f32 = 440.0;
const UNIT_H: f32 = 236.0;
const UX: f32 = CUB[0] + (CUB[2] - UNIT_W) / 2.0;
const UY0: f32 = CUB[1] + 12.0;
const TRAVEL: f32 = 196.0;
/// The fuse holder on the breaker's face: dx, dy, w, h.
const FUSE: [f32; 4] = [140.0, 64.0, 160.0, 70.0];
const SYN: [f32; 3] = [965.0, 250.0, 112.0];
const LAMP: [f32; 3] = [1168.0, 170.0, 32.0];
/// The close button: the biggest control.
const BTN: [f32; 3] = [1168.0, 318.0, 56.0];
const TRAY: [f32; 4] = [820.0, 432.0, 430.0, 268.0];
const RATINGS: [u32; 5] = [16, 25, 40, 63, 100];
/// The hand's frame, seconds (the kit's tests and captures run at 60 a second).
const HAND_DT: f32 = 1.0 / 60.0;

fn band_col(a: u32) -> Color32 {
    match a {
        16 => c::LILAC,
        25 => c::ACCENT,
        40 => c::WARN,
        63 => c::OK,
        _ => c::DANGER,
    }
}

fn in_box(b: [f32; 4], x: f32, y: f32, m: f32) -> bool {
    x >= b[0] - m && x <= b[0] + b[2] + m && y >= b[1] - m && y <= b[1] + b[3] + m
}

fn cart_box(i: usize) -> [f32; 4] {
    [TRAY[0] + 60.0, TRAY[1] + 26.0 + i as f32 * 60.0, 310.0, 44.0]
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Phase {
    #[default]
    Part,
    Out,
    Fuse,
    In,
    Close,
    Done,
}

#[derive(Clone, Copy, Debug, Default)]
struct Cart {
    a: u32,
    gone: bool,
}

/// The game's state.
#[derive(Default)]
pub struct Breakers {
    unit_p: [f32; 2],
    unit_held: bool,
    unit_set: bool,
    phase: Phase,
    rack: f32,
    grab: bool,
    grab0: f32,
    carts: Vec<Cart>,
    sel: usize,
    held: Option<usize>,
    bus: u32,
    bus_name: char,
    fuse_new: bool,
    theta: f32,
    omega: f32,
    win: f32,
    closed: bool,
    flash: f32,
    time: f32,
    hand: Hand,
}

impl Breakers {
    fn unit_y(&self) -> f32 {
        UY0 + self.rack * TRAVEL
    }
    fn handle(&self) -> [f32; 4] {
        [UX + UNIT_W / 2.0 - 70.0, self.unit_y() + UNIT_H - 46.0, 140.0, 30.0]
    }
    fn holder(&self) -> [f32; 4] {
        [UX + FUSE[0], self.unit_y() + FUSE[1], FUSE[2], FUSE[3]]
    }
    fn green(&self) -> bool {
        wrap(self.theta).abs() < self.win
    }

    /// The draw-out breaker at (x, y), at a scale, with its face: state window, fuse holder, racking handle.
    fn unit(&self, g: &Pen, x: f32, y: f32, s: f32) {
        let g = g.translate(x, y).scale(s, s);
        g.panel(0.0, 0.0, UNIT_W, UNIT_H, 10.0, hex(0x2b3646), hex(0x566273));
        g.rect(10.0, 10.0, UNIT_W - 20.0, 40.0, hex(0x222c3c));
        // The state window: O open (green), I closed (red).
        g.round(24.0, 16.0, 64.0, 28.0, 6.0, Some(hex(0x0c121a)), None);
        if self.closed {
            g.rect(52.0, 20.0, 8.0, 20.0, c::DANGER);
        } else {
            g.ring(56.0, 30.0, 9.0, c::OK, 4.0);
        }
        for k in 0..5 {
            g.disc(300.0 + k as f32 * 24.0, 30.0, 6.0, hex(0x3a4656));
        }
        // The fuse holder: covered by the interlock until the breaker is out.
        let [fx, fy, fw, fh] = FUSE;
        g.round(fx - 6.0, fy - 6.0, fw + 12.0, fh + 12.0, 8.0, Some(hex(0x121a25)), None);
        let p = self.phase;
        let open = p == Phase::Fuse || (p == Phase::In && self.rack > 0.02) || (p == Phase::Out && self.rack >= 0.98);
        if open || p == Phase::Part {
            if p != Phase::Part {
                let b = [fx + 8.0, fy + 13.0, fw - 16.0, fh - 26.0];
                if self.fuse_new {
                    cartridge(&g, b, self.bus, false, false);
                } else {
                    cartridge(&g, b, 0, false, true);
                }
            }
        } else {
            let q = g.clip(fx, fy, fw, fh);
            q.rect(fx, fy, fw, fh, hex(0x3a4656));
            let mut k = -fh;
            while k < fw {
                q.line(fx + k, fy + fh, fx + k + fh, fy, 6.0, hex(0x2b3646));
                k += 18.0;
            }
        }
        if p == Phase::Fuse {
            g.round(fx - 6.0, fy - 6.0, fw + 12.0, fh + 12.0, 8.0, None, Some((3.0, c::AMBER)));
        }
        // Vents.
        for k in 0..6 {
            g.rect(24.0, 70.0 + k as f32 * 14.0, 80.0, 6.0, hex(0x1b2433));
            g.rect(UNIT_W - 104.0, 70.0 + k as f32 * 14.0, 80.0, 6.0, hex(0x1b2433));
        }
        // The racking handle.
        let (hx, hy) = (UNIT_W / 2.0 - 70.0, UNIT_H - 46.0);
        let racking = p == Phase::Out || p == Phase::In;
        g.rect(UNIT_W / 2.0 - 6.0, hy - 10.0, 12.0, 14.0, hex(0x566273));
        let hc = if racking {
            if self.grab {
                c::AMBER
            } else {
                hex(0xc9d3e0)
            }
        } else {
            hex(0x7d8796)
        };
        g.round(hx, hy, 140.0, 30.0, 10.0, Some(hc), None);
        g.rect(hx + 20.0, hy + 12.0, 100.0, 6.0, hex(0x3a4656));
        if racking {
            let dir = if p == Phase::Out { 1.0 } else { -1.0 };
            let ay = if dir > 0.0 { hy + 44.0 } else { hy - 26.0 };
            g.poly(
                &[
                    [UNIT_W / 2.0 - 16.0, ay - dir * 8.0],
                    [UNIT_W / 2.0 + 16.0, ay - dir * 8.0],
                    [UNIT_W / 2.0, ay + dir * 8.0],
                ],
                c::AMBER,
            );
        }
    }

    /// The red hatching over the synchroscope's dial outside its green wedge.
    fn dial_hatch(&self, g: &Pen) {
        let [sx, sy, r] = SYN;
        // The wedge's two edges, and their inward normals (towards the wedge's middle, straight up).
        let mid = [0.0f32, -1.0];
        let normal = |a: f32| {
            let e = [a.cos(), a.sin()];
            let n = [-e[1], e[0]];
            if n[0] * mid[0] + n[1] * mid[1] >= 0.0 {
                n
            } else {
                [-n[0], -n[1]]
            }
        };
        let ns = [normal(-std::f32::consts::FRAC_PI_2 - self.win), normal(-std::f32::consts::FRAC_PI_2 + self.win)];
        let col = rgba(255, 71, 87, 0.18);
        let mut x = -r * 2.0;
        while x < r * 2.0 {
            let (a, b) = ([sx + x, sy + r], [sx + x + r, sy - r]);
            let d = [b[0] - a[0], b[1] - a[1]];
            // The chord inside the dial.
            let f = [a[0] - sx, a[1] - sy];
            let (qa, qb, qc) =
                (d[0] * d[0] + d[1] * d[1], 2.0 * (f[0] * d[0] + f[1] * d[1]), f[0] * f[0] + f[1] * f[1] - r * r);
            let disc = qb * qb - 4.0 * qa * qc;
            if disc > 0.0 {
                let (u0, u1) =
                    (((-qb - disc.sqrt()) / (2.0 * qa)).max(0.0), ((-qb + disc.sqrt()) / (2.0 * qa)).min(1.0));
                // The part of it inside the wedge.
                let (mut w0, mut w1) = (f32::NEG_INFINITY, f32::INFINITY);
                for n in ns {
                    let (k0, k1) = (n[0] * f[0] + n[1] * f[1], n[0] * d[0] + n[1] * d[1]);
                    if k1.abs() < 1e-9 {
                        if k0 < 0.0 {
                            w0 = f32::INFINITY;
                        }
                    } else if k1 > 0.0 {
                        w0 = w0.max(-k0 / k1);
                    } else {
                        w1 = w1.min(-k0 / k1);
                    }
                }
                let at = |u: f32| [a[0] + d[0] * u, a[1] + d[1] * u];
                let seg = |s0: f32, s1: f32| {
                    if s1 > s0 {
                        let (p, q) = (at(s0), at(s1));
                        g.line(p[0], p[1], q[0], q[1], 3.0, col);
                    }
                };
                if w0 >= w1 {
                    seg(u0, u1);
                } else {
                    seg(u0, u1.min(w0));
                    seg(u0.max(w1), u1);
                }
            }
            x += 14.0;
        }
    }
}

/// A fuse cartridge: brass caps, a ceramic body, the rating's band and label; blown is black and cracked.
fn cartridge(g: &Pen, b: [f32; 4], amps: u32, on: bool, blown: bool) {
    let [x, y, w, h] = b;
    let cap = 30.0f32.min(w * 0.14);
    let capc = if blown { hex(0x4a3a24) } else { hex(0xc9a45a) };
    g.round(x, y, cap, h, 4.0, Some(capc), None);
    g.round(x + w - cap, y, cap, h, 4.0, Some(capc), None);
    g.rect(x + cap, y + 3.0, w - cap * 2.0, h - 6.0, if blown { hex(0x1a1512) } else { hex(0xe7e2d6) });
    if blown {
        g.path(
            &[
                [x + w * 0.42, y + 3.0],
                [x + w * 0.5, y + h * 0.45],
                [x + w * 0.45, y + h * 0.6],
                [x + w * 0.55, y + h - 3.0],
            ],
            false,
            3.0,
            c::DANGER,
        );
    } else {
        g.rect(x + cap + 6.0, y + 3.0, 12.0, h - 6.0, band_col(amps));
        g.text(
            &format!("{amps} A"),
            x + w / 2.0 + 8.0,
            y + h / 2.0 + 1.0,
            26.0f32.min(h * 0.6),
            hex(0x111821),
            Align::Center,
        );
    }
    if on {
        g.round(x - 6.0, y - 5.0, w + 12.0, h + 10.0, 8.0, None, Some((3.0, c::AMBER)));
    }
}

impl Game for Breakers {
    fn id(&self) -> &'static str {
        "breakers"
    }

    fn knobs(&self) -> &'static [&'static str] {
        &["cartridges_count", "slip_rad_s", "window_rad"]
    }

    fn step(&mut self, cx: &mut Ctx) {
        let mut r = cx.dice();
        self.unit_p = [1035.0, 590.0];
        self.unit_held = false;
        self.unit_set = false;
        self.phase = if cx.part { Phase::Part } else { Phase::Out };
        self.rack = 0.0;
        self.grab = false;
        self.held = None;
        self.sel = 0;
        self.closed = false;
        self.flash = 0.0;
        self.time = 0.0;
        self.fuse_new = false;
        let k = (cx.knob("cartridges_count").round() as usize).clamp(1, RATINGS.len());
        let mut pool = RATINGS;
        r.shuffle(&mut pool);
        self.carts = pool[..k].iter().map(|&a| Cart { a, gone: false }).collect();
        self.bus = self.carts[r.int(k)].a;
        self.bus_name = ['A', 'B', 'C', 'D'][r.int(4)];
        self.theta = r.f() * std::f32::consts::TAU;
        self.omega = cx.knob("slip_rad_s");
        self.win = cx.knob("window_rad");
        self.hand = Hand::default();
    }

    fn update(&mut self, cx: &mut Ctx, dt: f32, input: &Input) {
        self.time += dt;
        self.flash = (self.flash - dt * 1.6).max(0.0);
        self.theta += self.omega * (1.0 + 0.3 * (self.time * 0.5).sin()) * dt;
        match self.phase {
            Phase::Part => {
                // The part step: wheel the new breaker into the cubicle.
                if self.unit_set {
                    return;
                }
                let [ux, uy] = self.unit_p;
                if input.pressed && (input.x - ux).abs() < UNIT_W * 0.25 && (input.y - uy).abs() < UNIT_H * 0.25 {
                    self.unit_held = true;
                }
                if self.unit_held && input.down {
                    self.unit_p = [input.x, input.y];
                }
                if self.unit_held && input.released {
                    self.unit_held = false;
                    if in_box(CUB, self.unit_p[0], self.unit_p[1], 0.0) {
                        self.unit_set = true;
                        cx.step_done();
                    }
                }
            }
            Phase::Out | Phase::In => {
                // Racking: drag the handle, or hold the arrow, the way this phase goes.
                let dir = if self.phase == Phase::Out { 1.0 } else { -1.0 };
                let h = self.handle();
                if input.pressed && in_box(h, input.x, input.y, 16.0) {
                    self.grab = true;
                    self.grab0 = input.y - self.rack * TRAVEL;
                }
                if self.grab && input.down {
                    self.rack = ((input.y - self.grab0) / TRAVEL).clamp(0.0, 1.0);
                }
                if !input.down {
                    self.grab = false;
                }
                if input.stick[1] * dir > 0.0 {
                    self.rack = (self.rack + dir * 0.7 * dt).clamp(0.0, 1.0);
                }
                if self.phase == Phase::Out && self.rack >= 0.98 {
                    self.rack = 1.0;
                    self.grab = false;
                    self.phase = Phase::Fuse;
                }
                if self.phase == Phase::In && self.rack <= 0.02 {
                    self.rack = 0.0;
                    self.grab = false;
                    self.phase = Phase::Close;
                }
            }
            Phase::Fuse => {
                use egui::Key;
                let live: Vec<usize> = (0..self.carts.len()).filter(|&i| !self.carts[i].gone).collect();
                let n = live.len() as isize;
                let at = live.iter().position(|&i| i == self.sel).map_or(-1, |p| p as isize);
                if input.hit(Key::ArrowUp) || input.hit(Key::W) {
                    self.sel = live[(at + n - 1).rem_euclid(n) as usize];
                }
                if input.hit(Key::ArrowDown) || input.hit(Key::S) {
                    self.sel = live[(at + 1).rem_euclid(n) as usize];
                }
                if self.carts[self.sel].gone {
                    self.sel = live[0];
                }
                // A cartridge is 44 px tall in a 60 px pitch: its zone reaches 8 px into each gap (kit TOUCH_R).
                if input.pressed {
                    for &i in &live {
                        if in_box(cart_box(i), input.x, input.y, 8.0) {
                            self.held = Some(i);
                            self.sel = i;
                        }
                    }
                }
                let mut fit = None;
                if let (Some(h), true) = (self.held, input.released) {
                    if in_box(self.holder(), input.x, input.y, 40.0) {
                        fit = Some(h);
                    }
                    self.held = None;
                }
                if input.action_pressed {
                    fit = Some(self.sel);
                }
                let Some(f) = fit else { return };
                self.carts[f].gone = true;
                if self.carts[f].a == self.bus {
                    self.fuse_new = true;
                    self.phase = Phase::In;
                } else {
                    self.flash = 0.5;
                    cx.fumble("Wrong rating: the fuse blows");
                }
            }
            Phase::Close => {
                let press = input.action_pressed || (input.pressed && input.dist(BTN[0], BTN[1]) < BTN[2] + 8.0);
                if !press {
                    return;
                }
                if self.green() {
                    self.closed = true;
                    self.phase = Phase::Done;
                    cx.step_done();
                } else {
                    self.flash = 1.0;
                    cx.fumble("Arc flash: 10 HP");
                }
            }
            Phase::Done => {}
        }
    }

    fn draw(&self, g: &Pen, _cx: &Ctx, t: f32, input: &Input) {
        use std::f32::consts::{FRAC_PI_2, TAU};
        g.rect(0.0, BAR_H, W, 720.0, hex(0x070b12));
        let p = self.phase;

        // The cabinet: steel, its bolts, the rating plate, the cubicle and its rails.
        let [kx, ky, kw, kh] = CAB;
        g.panel(kx, ky, kw, kh, 10.0, hex(0x1b2433), hex(0x3a4656));
        for [x, y] in [
            [kx + 22.0, ky + 22.0],
            [kx + kw - 22.0, ky + 22.0],
            [kx + 22.0, ky + kh - 22.0],
            [kx + kw - 22.0, ky + kh - 22.0],
        ] {
            g.disc(x, y, 7.0, hex(0x3a4656));
        }
        g.panel(300.0, 112.0, 300.0, 62.0, 8.0, hex(0x0c121a), hex(0x566273));
        g.text(&format!("BUS {}", self.bus_name), 330.0, 143.0, 26.0, c::DIM, Align::Left);
        g.text(&format!("{} A", self.bus), 570.0, 143.0, 34.0, c::AMBER, Align::Right);
        let [bx, by, bw, bh] = CUB;
        g.round(bx, by, bw, bh, 6.0, Some(hex(0x06090e)), Some((3.0, hex(0x2b3646))));
        for x in [bx + 8.0, bx + bw - 20.0] {
            g.rect(x, by + 8.0, 12.0, bh - 16.0, hex(0x3a4656));
        }
        let mut y = by + 24.0;
        while y < by + 120.0 {
            g.rect(bx + 120.0, y, 260.0, 8.0, hex(0x2b2014));
            g.rect(bx + 120.0, y, 260.0, 3.0, c::COPPER);
            y += 22.0;
        }
        // The racked position: a slot on the cabinet's side with a marker.
        g.rect(bx + bw + 26.0, UY0 + 40.0, 10.0, TRAVEL, hex(0x0c121a));
        let my = UY0 + 40.0 + self.rack * TRAVEL;
        let racking = p == Phase::Out || p == Phase::In;
        g.poly(
            &[[bx + bw + 22.0, my], [bx + bw + 40.0, my - 10.0], [bx + bw + 40.0, my + 10.0]],
            if racking { c::AMBER } else { c::STEEL },
        );
        g.text("IN", bx + bw + 48.0, UY0 + 40.0, 16.0, c::DIM, Align::Left);
        g.text("OUT", bx + bw + 48.0, UY0 + 40.0 + TRAVEL, 16.0, c::DIM, Align::Left);

        if p != Phase::Part {
            self.unit(g, UX, self.unit_y(), 1.0);
        } else if self.unit_set {
            self.unit(g, UX, UY0, 1.0);
        } else {
            g.dashed(&closed(round_rect_points(UX, UY0, UNIT_W, UNIT_H, 10.0)), 3.0, c::AMBER, 12.0, 9.0);
        }

        // The synchroscope: a dial, a green wedge at the top, the needle turning with the slip.
        let [sx, sy, sr] = SYN;
        g.panel(820.0, 96.0, 430.0, 314.0, 18.0, hex(0x0c121a), c::LINE);
        g.disc(sx, sy, sr + 10.0, hex(0x121a25));
        g.ring(sx, sy, sr + 10.0, hex(0x3a4656), 3.0);
        g.poly(&pie(sx, sy, sr, -FRAC_PI_2 - self.win, -FRAC_PI_2 + self.win), rgba(61, 220, 132, 0.4));
        self.dial_hatch(g);
        for k in 0..36 {
            let a = k as f32 / 36.0 * TAU;
            g.line(
                sx + a.cos() * (sr - 10.0),
                sy + a.sin() * (sr - 10.0),
                sx + a.cos() * sr,
                sy + a.sin() * sr,
                2.0,
                hex(0x566273),
            );
        }
        let live = p != Phase::Part;
        let ok = live && self.green();
        let na = self.theta - FRAC_PI_2;
        g.path_round(
            &[[sx - na.cos() * 20.0, sy - na.sin() * 20.0], [sx + na.cos() * (sr - 8.0), sy + na.sin() * (sr - 8.0)]],
            6.0,
            if live { c::FG } else { hex(0x3a4656) },
        );
        g.disc(sx, sy, 12.0, hex(0x566273));
        // The lamp: green with a tick, red with a cross.
        let [lx, ly, lr] = LAMP;
        g.disc(lx, ly, lr + 6.0, hex(0x121a25));
        if !live {
            g.disc(lx, ly, lr, hex(0x1d2738));
        } else if ok {
            g.disc(lx, ly, lr, c::OK);
            g.tick(lx, ly + 2.0, 30.0, hex(0x04140a));
        } else {
            g.disc(lx, ly, lr, c::DANGER);
            g.cross(lx, ly, 12.0, 6.0, hex(0x2a0a0e));
        }
        // The close button: a mushroom head, lit when the breaker is ready to close.
        let ready = p == Phase::Close;
        let [nx, ny, nr] = BTN;
        g.disc(nx, ny, nr + 8.0, hex(0x121a25));
        g.ring(nx, ny, nr + 8.0, if ready { c::AMBER } else { hex(0x2b3646) }, 4.0);
        g.disc(nx, ny + 4.0, nr, if ready { hex(0x7a2f38) } else { hex(0x262e42) });
        let head = if self.closed {
            c::OK
        } else if ready {
            c::DANGER
        } else {
            hex(0x2b3646)
        };
        g.disc(nx, ny, nr - 2.0, head);
        g.text("CLOSE", nx, ny, 22.0, if ready || self.closed { hex(0xffffff) } else { c::DIM }, Align::Center);

        // The fuse tray: one cartridge a rating, each with its colour band and label.
        let [tx, ty, tw, th] = TRAY;
        g.panel(tx, ty, tw, th, 18.0, hex(0x0c121a), c::LINE);
        if p == Phase::Part {
            // The trolley with the new breaker on it.
            g.rect(tx + 40.0, ty + th - 50.0, tw - 80.0, 14.0, hex(0x2b3646));
            for x in [tx + 80.0, tx + tw - 80.0] {
                g.disc(x, ty + th - 26.0, 14.0, hex(0x566273));
            }
            if !self.unit_set {
                self.unit(g, self.unit_p[0] - UNIT_W * 0.25, self.unit_p[1] - UNIT_H * 0.25, 0.5);
            }
        } else {
            for (i, cart) in self.carts.iter().enumerate() {
                if cart.gone || Some(i) == self.held {
                    continue;
                }
                cartridge(g, cart_box(i), cart.a, p == Phase::Fuse && i == self.sel, false);
            }
            if let Some(h) = self.held {
                let b = cart_box(h);
                cartridge(
                    g,
                    [input.x - b[2] * 0.25, input.y - b[3] * 0.5, b[2] * 0.5, b[3]],
                    self.carts[h].a,
                    true,
                    false,
                );
            }
        }

        // The arc flash.
        if self.flash > 0.0 {
            let h = self.holder();
            let (x0, y0) = (h[0] + h[2] / 2.0, h[1] + h[3] / 2.0);
            let col = rgba(200, 230, 255, self.flash);
            for k in 0..8 {
                let kf = k as f32;
                let mut pts = vec![[x0, y0]];
                for j in 1..6 {
                    let jf = j as f32;
                    pts.push([
                        x0 + (kf * 0.8 + t * 17.0).cos() * jf * 30.0 + (t * 50.0 + jf).sin() * 12.0,
                        y0 + (kf * 0.8 + t * 13.0).sin() * jf * 26.0,
                    ]);
                }
                g.path(&pts, false, 4.0, col);
            }
            g.rect(0.0, BAR_H, W, 720.0, rgba(220, 235, 255, 0.35 * self.flash));
        }
    }

    fn hand(&mut self, _cx: &Ctx, _t: f32) -> Input {
        if self.hand.resting() {
            return Input::default();
        }
        let h = self.handle();
        let grip = [h[0] + h[2] / 2.0, h[1] + h[3] / 2.0];
        match self.phase {
            // Wheel the breaker off the trolley into the cubicle.
            Phase::Part if !self.unit_set => self.hand.carry(1, self.unit_p, [450.0, 330.0], PART_STEP),
            // Draw the handle down, then up, a steady pull.
            Phase::Out => self.hand.carry(2, grip, [grip[0], grip[1] + TRAVEL + 10.0], 8.0),
            Phase::In => self.hand.carry(4, grip, [grip[0], grip[1] - TRAVEL - 10.0], 8.0),
            // The cartridge whose rating is the bus's, into the holder.
            Phase::Fuse => {
                let Some(i) = self.carts.iter().position(|k| !k.gone && k.a == self.bus) else {
                    return self.hand.idle();
                };
                let b = cart_box(i);
                let hd = self.holder();
                self.hand.carry(
                    3,
                    [b[0] + b[2] / 2.0, b[1] + b[3] / 2.0],
                    [hd[0] + hd[2] / 2.0, hd[1] + hd[3] / 2.0],
                    24.0,
                )
            }
            // Close when the needle will sit well inside the wedge.
            Phase::Close => {
                let i = self.hand.idle();
                if i != Input::default() {
                    return i;
                }
                let next = self.theta + self.omega * (1.0 + 0.3 * ((self.time + HAND_DT) * 0.5).sin()) * HAND_DT;
                Input { action_pressed: wrap(next).abs() < self.win * 0.6, ..Input::default() }
            }
            _ => self.hand.idle(),
        }
    }

    fn guide_now(&self) -> Option<usize> {
        match self.phase {
            Phase::Out => Some(1),
            Phase::Fuse => Some(2),
            Phase::In | Phase::Close => Some(3),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::games::tests::{fumble_check, plays_to_end};
    use crate::kit::Input;
    use egui::Key;

    #[test]
    fn a_steady_hand_repairs_it_damaged_disabled_and_destroyed() {
        plays_to_end("breakers");
    }

    #[test]
    fn a_wrong_fuse_or_closing_on_red_is_an_arc_flash() {
        // Rack out on the arrow, fit whichever cartridge is chosen, rack in, then close at once: either the fuse is
        // the wrong rating, or the close lands on red.
        let mut n = 0;
        fumble_check("breakers", move |_r| mistake(&mut n));
    }

    /// Frame `n` of the mistake (counted here).
    fn mistake(n: &mut u32) -> Input {
        *n += 1;
        match *n {
            0..=150 => Input::default().key(Key::ArrowDown, false),
            151 => Input::default().key(Key::Space, true),
            152..=300 => Input::default().key(Key::ArrowUp, false),
            k => Input::default().key(Key::Space, k % 2 == 0),
        }
    }
}
