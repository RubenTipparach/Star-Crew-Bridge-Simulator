//! The reactor core's repair, "Reactor: magnetic core" (repair-minigames design 2 and 6b), from
//! `docs/mockups/repairs/reactor.js`.
//!
//! The containment vessel from the front: the plasma ball floats inside the ring, held by four magnet coils (N, E, S,
//! W), while a fifth coil in the upper right slot is swapped out. The core's load pushes the plasma off centre and the
//! push wanders. Two controls hold it (owner, 2026-10-08): the horizontal slider pulls the field left or right, the
//! vertical one up or down, the way its handle is pushed (drag them, or the arrows or W A S D); the coils glow with the
//! pull. A round has two stages on the same controls: hold the core in its centre band until the swap's first half
//! fills, then the plasma ring round it, which the load pulls out of round (the horizontal slider sets its width, the
//! vertical its height), held round in its band. The plasma, or the ring bulging into the wall or collapsing onto the
//! core, is the fumble: a heat spike, and both sliders spring back to centre and let go of the hand (design 6d). Each
//! level the load is heavier and wanders faster, and the holds are longer. A disabled core's first step fits the new
//! coil: drag it from the crate into the open slot, with the core cold.

use std::f32::consts::{FRAC_PI_2, FRAC_PI_4, PI, TAU};

use egui::{Color32, Key};

use crate::kit::{Ctx, Game, Input, BAR_H, H, W};
use crate::pen::{c, hex, rgba, Align, Pen};

/// In the engine.
pub const PORTED: bool = true;

/// A fresh game.
pub fn new() -> Box<dyn Game> {
    Box::new(Reactor::default())
}

const CX: f32 = 520.0;
const CY: f32 = 400.0;
/// The vessel's outer radius, px.
const VESSEL: f32 = 226.0;
/// The chamber's inner wall, px.
const WALL: f32 = 178.0;
/// The plasma's radius, px.
const BALL: f32 = 24.0;
/// The centre band the swap needs, px.
const BAND: f32 = 46.0;
/// Where the coil housings sit, px from the centre.
const COIL_R: f32 = 258.0;
/// Px/s of push at a control's end stop.
const PULL: f32 = 125.0;
/// A control's travel a second from the stick (end to end in 1.3 s).
const TRIM_RATE: f32 = 1.5;
/// Seconds: the plasma's lag behind the field.
const LAG: f32 = 0.35;
/// The swapped coil's slot: upper right.
const SLOT: f32 = -FRAC_PI_4;
/// The ring's round radius, px.
const RING: f32 = 112.0;
/// How far out of round still counts as held.
const RING_BAND: f32 = 0.22;
/// The radius change per unit of shape error.
const RING_GAIN: f32 = 0.4;
/// The coils: name, angle, whether it sits on the x axis.
const COILS: [(&str, f32, bool); 4] =
    [("N", -FRAC_PI_2, false), ("E", 0.0, true), ("S", FRAC_PI_2, false), ("W", PI, true)];
/// The vertical slider (x, y, w, h).
const VS: [f32; 4] = [1010.0, 120.0, 64.0, 400.0];
/// The horizontal slider.
const HS: [f32; 4] = [892.0, 590.0, 300.0, 64.0];
/// The crate the new coil waits in.
const CRATE_COIL: [f32; 2] = [1080.0, 420.0];

fn slot_xy() -> [f32; 2] {
    [CX + SLOT.cos() * COIL_R, CY + SLOT.sin() * COIL_R]
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Grab {
    #[default]
    None,
    V,
    H,
}

#[derive(Clone, Copy, Debug, Default)]
struct Drift {
    a0: f32,
    w: f32,
    m: f32,
    p1: f32,
    p2: f32,
}

#[derive(Clone, Copy, Debug, Default)]
struct RingLoad {
    m: f32,
    w1: f32,
    w2: f32,
    p1: f32,
    p2: f32,
}

/// The game's state.
#[derive(Default)]
pub struct Reactor {
    part: bool,
    coil: [f32; 2],
    coil_held: bool,
    coil_set: bool,
    /// The two controls, -1 to 1: x the horizontal slider, y the vertical.
    ax: [f32; 2],
    /// The plasma's offset from the centre, px, and its velocity, px/s.
    p: [f32; 2],
    v: [f32; 2],
    drift: Drift,
    hold: f32,
    hold_need: f32,
    done: bool,
    heat: f32,
    cool: f32,
    grab: Grab,
    trail: Vec<[f32; 2]>,
    time: f32,
    /// 1 the core, 2 the plasma ring.
    stage: u8,
    ring: [f32; 2],
    ring_load: RingLoad,
    ring_hold: f32,
    ring_need: f32,
    /// The hand: carrying the coil.
    hand_carry: bool,
}

impl Reactor {
    /// The core's load at time `t`: the push on the plasma, px/s.
    fn load(&self, t: f32) -> [f32; 2] {
        let d = &self.drift;
        let m = d.m * (1.0 + 0.35 * (1.3 * t + d.p1).sin());
        let a = d.a0 + d.w * t + 0.6 * (0.7 * t + d.p2).sin();
        [a.cos() * m, a.sin() * m]
    }

    /// The ring's pull out of round from the load alone, at time `t`.
    fn ring_load_at(&self, t: f32) -> [f32; 2] {
        let l = &self.ring_load;
        [l.m * (l.w1 * t + l.p1).sin(), l.m * (l.w2 * t + l.p2).cos()]
    }

    /// Each coil's pull from the two controls (N, E, S, W).
    fn coil_pull(&self) -> [f32; 4] {
        let [x, y] = self.ax;
        [(1.0 - y) / 2.0, (1.0 + x) / 2.0, (1.0 + y) / 2.0, (1.0 - x) / 2.0]
    }

    fn reset_controls(&mut self) {
        self.ax = [0.0, 0.0];
        self.grab = Grab::None;
    }

    fn ring_radii(&self) -> (f32, f32) {
        (RING * (1.0 + RING_GAIN * self.ring[0]), RING * (1.0 + RING_GAIN * self.ring[1]))
    }
}

fn slider_at(x: f32, y: f32) -> Grab {
    let [vx, vy, vw, vh] = VS;
    let [hx, hy, hw, hh] = HS;
    if x >= vx - 16.0 && x <= vx + vw + 16.0 && y >= vy - 20.0 && y <= vy + vh + 20.0 {
        return Grab::V;
    }
    if x >= hx - 20.0 && x <= hx + hw + 20.0 && y >= hy - 16.0 && y <= hy + hh + 16.0 {
        return Grab::H;
    }
    Grab::None
}

/// A disc shading through `stops` (0 at the centre, 1 at the rim), canvas's radial gradient with several stops.
fn radial_stops(g: &Pen, x: f32, y: f32, r: f32, stops: &[(f32, Color32)]) {
    let n = 32;
    for w in stops.windows(2) {
        let ((t0, c0), (t1, c1)) = (w[0], w[1]);
        let (r0, r1) = (r * t0, r * t1);
        for i in 0..n {
            let (a0, a1) = (TAU * i as f32 / n as f32, TAU * (i + 1) as f32 / n as f32);
            g.quad(
                [
                    [x + a0.cos() * r0, y + a0.sin() * r0],
                    [x + a1.cos() * r0, y + a1.sin() * r0],
                    [x + a1.cos() * r1, y + a1.sin() * r1],
                    [x + a0.cos() * r1, y + a0.sin() * r1],
                ],
                [c0, c0, c1, c1],
            );
        }
    }
}

/// An ellipse's outline points round (x, y).
fn ellipse_pts(x: f32, y: f32, rx: f32, ry: f32) -> Vec<[f32; 2]> {
    (0..=72).map(|i| TAU * i as f32 / 72.0).map(|a| [x + a.cos() * rx, y + a.sin() * ry]).collect()
}

/// A dashed circle.
#[allow(clippy::too_many_arguments)]
fn dashed_ring(g: &Pen, x: f32, y: f32, r: f32, col: Color32, width: f32, dash: f32, gap: f32) {
    g.dashed(&Pen::arc_points(x, y, r, 0.0, TAU), width, col, dash, gap);
}

impl Game for Reactor {
    fn id(&self) -> &'static str {
        "reactor"
    }

    fn knobs(&self) -> &'static [&'static str] {
        &["drift_rad_s", "drift_px_s", "hold_s", "ring_hold_s", "ring_load_frac", "ring_w1_rad_s", "ring_w2_rad_s"]
    }

    fn step(&mut self, cx: &mut Ctx) {
        let mut r = cx.dice();
        self.part = cx.part;
        self.coil = CRATE_COIL;
        self.coil_held = false;
        self.coil_set = false;
        self.ax = [0.0, 0.0];
        self.p = [0.0, 0.0];
        self.v = [0.0, 0.0];
        let a0 = r.f() * TAU;
        let sign = if r.f() < 0.5 { -1.0 } else { 1.0 };
        self.drift =
            Drift { a0, w: cx.knob("drift_rad_s") * sign, m: cx.knob("drift_px_s"), p1: r.f() * 6.3, p2: r.f() * 6.3 };
        self.hold = 0.0;
        self.hold_need = cx.knob("hold_s");
        self.done = false;
        self.stage = 1;
        self.ring = [0.0, 0.0];
        self.ring_hold = 0.0;
        self.ring_need = cx.knob("ring_hold_s");
        self.ring_load = RingLoad {
            m: cx.knob("ring_load_frac"),
            w1: cx.knob("ring_w1_rad_s"),
            w2: cx.knob("ring_w2_rad_s"),
            p1: r.f() * 6.3,
            p2: r.f() * 6.3,
        };
        self.heat = 0.0;
        self.cool = 0.0;
        self.grab = Grab::None;
        self.trail.clear();
        self.time = 0.0;
        self.hand_carry = false;
    }

    fn update(&mut self, cx: &mut Ctx, dt: f32, input: &Input) {
        self.heat = (self.heat - dt * 0.6).max(0.0);
        self.cool = (self.cool - dt).max(0.0);
        if self.part && !self.coil_set {
            // The part step: carry the coil to the open slot.
            let [sx, sy] = slot_xy();
            if input.pressed && input.dist(self.coil[0], self.coil[1]) < 50.0 {
                self.coil_held = true;
            }
            if self.coil_held && input.down {
                self.coil = [input.x, input.y];
            }
            if self.coil_held && input.released {
                self.coil_held = false;
                if (self.coil[0] - sx).hypot(self.coil[1] - sy) < 50.0 {
                    self.coil_set = true;
                    self.coil = [sx, sy];
                    cx.step_done();
                }
            }
            return;
        }
        if self.part {
            return;
        }
        self.time += dt;
        // The two controls: the stick moves them, a drag sets the one it holds.
        let cl = |x: f32| x.clamp(-1.0, 1.0);
        if input.stick[0] != 0.0 {
            self.ax[0] = cl(self.ax[0] + input.stick[0] * TRIM_RATE * dt);
        }
        if input.stick[1] != 0.0 {
            self.ax[1] = cl(self.ax[1] + input.stick[1] * TRIM_RATE * dt);
        }
        if input.pressed {
            self.grab = slider_at(input.x, input.y);
        }
        if self.grab == Grab::V && input.down {
            self.ax[1] = cl((input.y - (VS[1] + VS[3] / 2.0)) / (VS[3] / 2.0));
        }
        if self.grab == Grab::H && input.down {
            self.ax[0] = cl((input.x - (HS[0] + HS[2] / 2.0)) / (HS[2] / 2.0));
        }
        if input.released || !input.down {
            self.grab = Grab::None;
        }
        if self.done {
            // Swapped: the field settles the plasma home.
            let k = 1.0 - (dt * 2.0).min(1.0);
            self.p = [self.p[0] * k, self.p[1] * k];
            return;
        }
        // The plasma follows the field (the load plus the coils) with a short lag; in stage 2 the core is held.
        let [lx, ly] = if self.stage == 1 {
            self.load(self.time)
        } else {
            [-self.ax[0] * PULL - self.p[0] * 3.0, -self.ax[1] * PULL - self.p[1] * 3.0]
        };
        let (fx, fy) = (lx + self.ax[0] * PULL, ly + self.ax[1] * PULL);
        let k = (dt / LAG).min(1.0);
        self.v[0] += (fx - self.v[0]) * k;
        self.v[1] += (fy - self.v[1]) * k;
        self.p[0] += self.v[0] * dt;
        self.p[1] += self.v[1] * dt;
        self.trail.push(self.p);
        if self.trail.len() > 14 {
            self.trail.remove(0);
        }
        let off = self.p[0].hypot(self.p[1]);
        if off + BALL >= WALL && self.cool <= 0.0 {
            self.heat = 1.0;
            self.cool = 1.0;
            self.p = [self.p[0] * 0.5, self.p[1] * 0.5];
            self.v = [0.0, 0.0];
            self.trail.clear();
            self.reset_controls();
            cx.fumble("Plasma touched the wall: heat spike");
            return;
        }
        if off + BALL > WALL {
            let k = (WALL - BALL) / off;
            self.p = [self.p[0] * k, self.p[1] * k];
        }
        if self.stage == 1 && off <= BAND {
            self.hold += dt;
            // The core is held: on to the ring, the controls back to centre.
            if self.hold >= self.hold_need {
                self.hold = self.hold_need;
                self.stage = 2;
                self.ax = [0.0, 0.0];
                self.time = 0.0;
            }
        }
        if self.stage == 2 {
            let [ex, ey] = self.ring_load_at(self.time);
            let (ex, ey) = (ex + self.ax[0], ey + self.ax[1]);
            self.ring[0] += (ex - self.ring[0]) * k;
            self.ring[1] += (ey - self.ring[1]) * k;
            let (rx, ry) = self.ring_radii();
            let bulge = rx.max(ry) >= WALL - 6.0;
            if (bulge || rx.min(ry) <= BALL * 2.0) && self.cool <= 0.0 {
                self.heat = 1.0;
                self.cool = 1.0;
                self.ring = [self.ring[0] * 0.4, self.ring[1] * 0.4];
                self.reset_controls();
                cx.fumble(if bulge {
                    "Plasma ring touched the wall: heat spike"
                } else {
                    "Plasma ring collapsed on the core: heat spike"
                });
                return;
            }
            if self.ring[0].abs() < RING_BAND && self.ring[1].abs() < RING_BAND {
                self.ring_hold += dt;
                if self.ring_hold >= self.ring_need {
                    self.ring_hold = self.ring_need;
                    self.done = true;
                    cx.step_done();
                }
            }
        }
    }

    fn draw(&self, g: &Pen, _cx: &Ctx, t: f32, _input: &Input) {
        g.rect(0.0, BAR_H, W, H, hex(0x070b12));
        let live = !self.part;
        let in_band = live && self.p[0].hypot(self.p[1]) <= BAND;

        // The core's temperature: a column on the left, its top band hatched red.
        let (tx, ty0, th) = (96.0, 150.0, 470.0);
        g.panel(tx - 26.0, ty0 - 16.0, 52.0, th + 32.0, 14.0, c::PANEL, c::LINE);
        let col = g.clip(tx - 12.0, ty0, 24.0, th);
        col.round(tx - 12.0, ty0, 24.0, th, 12.0, Some(hex(0x1a2230)), None);
        let mut y = ty0 - 24.0;
        while y < ty0 + th * 0.2 {
            col.line(tx - 12.0, y + 24.0, tx + 12.0, y, 3.0, rgba(255, 71, 87, 0.55));
            y += 12.0;
        }
        let temp = if live { 0.42 + 0.06 * (t * 0.8).sin() + 0.5 * self.heat } else { 0.08 };
        let ty = ty0 + th * (1.0 - temp);
        let tcol = if temp > 0.8 {
            c::DANGER
        } else if temp > 0.6 {
            c::WARN
        } else {
            c::AMBER
        };
        col.round(tx - 12.0, ty, 24.0, ty0 + th - ty, 12.0, Some(tcol), None);
        col.rect(tx - 12.0, ty, 24.0, (ty0 + th - ty - 12.0).max(0.0), tcol);
        g.disc(tx, ty0 + th + 2.0, 20.0, if temp > 0.8 { c::DANGER } else { c::AMBER });

        // The vessel: a thick steel ring with its bolts, the dark chamber inside.
        g.disc(CX, CY, VESSEL, hex(0x141c28));
        g.ring(CX, CY, VESSEL, hex(0x2b3646), 4.0);
        for i in 0..24 {
            let a = i as f32 / 24.0 * TAU + PI / 24.0;
            g.disc(CX + a.cos() * (VESSEL - 18.0), CY + a.sin() * (VESSEL - 18.0), 5.0, hex(0x3a4656));
        }
        g.disc(CX, CY, WALL + 14.0, hex(0x0e151f));
        g.disc(CX, CY, WALL, hex(0x05080d));
        let wall_hot = if self.heat > 0.0 {
            rgba(255, 71, 87, 0.35 + 0.65 * self.heat)
        } else if live {
            hex(0x2f4a66)
        } else {
            hex(0x1d2633)
        };
        g.ring(CX, CY, WALL, wall_hot, 5.0);

        // The coils: housings outside the vessel, windings glowing with their trim, the field on the inner wall.
        let pull = self.coil_pull();
        for (i, (name, a, on_x)) in COILS.iter().enumerate() {
            let (hx, hy) = (CX + a.cos() * COIL_R, CY + a.sin() * COIL_R);
            let k = if live { pull[i] } else { 0.0 };
            let q = g.translate(hx, hy).rotate(a + FRAC_PI_2);
            q.panel(-60.0, -28.0, 120.0, 56.0, 10.0, hex(0x1b2433), hex(0x34404f));
            let mut s = -48.0;
            while s <= 44.0 {
                q.rect(s, -18.0, 5.0, 36.0, rgba(208, 138, 74, 0.35 + 0.65 * k));
                s += 8.0;
            }
            g.arc(CX, CY, WALL - 6.0, a - 0.45, a + 0.45, 6.0 + 10.0 * k, rgba(79, 195, 247, 0.08 + 0.55 * k));
            if *on_x {
                let (lx, ly) = (CX + a.cos() * (COIL_R + 52.0), CY + a.sin() * (COIL_R + 52.0));
                g.text(name, lx, ly, 22.0, c::DIM, Align::Center);
            } else {
                g.text(name, hx - 82.0, hy, 22.0, c::DIM, Align::Center);
            }
        }

        // The fifth coil's slot: the swap's ring fills while the plasma is held centred.
        let [sx, sy] = slot_xy();
        let open = self.part && !self.coil_set;
        let q = g.translate(sx, sy).rotate(SLOT + FRAC_PI_2);
        q.panel(
            -60.0,
            -28.0,
            120.0,
            56.0,
            10.0,
            if open { hex(0x120d08) } else { hex(0x1b2433) },
            if open { c::AMBER } else { hex(0x34404f) },
        );
        let hold_k = if self.hold_need > 0.0 { self.hold / self.hold_need } else { 0.0 };
        let ring_k = if self.ring_need > 0.0 { self.ring_hold / self.ring_need } else { 0.0 };
        if !open {
            let fill = if self.part { 1.0 } else { 0.25 + 0.75 * ((hold_k + ring_k) / 2.0) };
            let mut s = -48.0;
            while s <= 44.0 {
                q.rect(s, -18.0, 5.0, 36.0, if (s + 48.0) / 96.0 <= fill { c::COPPER } else { hex(0x3a2a1c) });
                s += 8.0;
            }
        }
        if !self.part {
            g.ring(sx, sy, 74.0, hex(0x1d2636), 8.0);
            // Two halves: the core's hold, then the ring's.
            let sweep = PI * (hold_k + ring_k);
            if sweep > 0.0 {
                let col = if self.done { c::OK } else { c::AMBER };
                let (a0, a1) = (-FRAC_PI_2, -FRAC_PI_2 + sweep);
                g.arc(sx, sy, 74.0, a0, a1, 8.0, col);
                g.disc(sx + a0.cos() * 74.0, sy + a0.sin() * 74.0, 4.0, col);
                g.disc(sx + a1.cos() * 74.0, sy + a1.sin() * 74.0, 4.0, col);
            }
        }

        // The centre band and the plasma.
        dashed_ring(g, CX, CY, BAND, if in_band { c::OK } else { hex(0x3b4a5e) }, 3.0, 10.0, 8.0);
        g.line(CX - WALL, CY, CX + WALL, CY, 1.0, rgba(111, 127, 148, 0.25));
        g.line(CX, CY - WALL, CX, CY + WALL, 1.0, rgba(111, 127, 148, 0.25));
        if live {
            let n = self.trail.len() as f32;
            for (i, [x, y]) in self.trail.iter().enumerate() {
                let f = i as f32 / n;
                g.disc(CX + x, CY + y, BALL * f * 0.8, rgba(190, 159, 230, 0.05 + 0.12 * f));
            }
            // Stage 2: the plasma ring, its target band dashed round it (green while held), a pip for each stage.
            if self.stage == 2 {
                let held = self.ring[0].abs() < RING_BAND && self.ring[1].abs() < RING_BAND;
                for k in [1.0 - RING_GAIN * RING_BAND, 1.0 + RING_GAIN * RING_BAND] {
                    dashed_ring(g, CX, CY, RING * k, if held { c::OK } else { hex(0x3b4a5e) }, 2.0, 8.0, 7.0);
                }
                let (rx, ry) = self.ring_radii();
                let wob = 1.0 + 0.02 * (t * 17.0).sin();
                let pts = ellipse_pts(CX, CY, rx * wob, ry * wob);
                for (w, col) in [
                    (22.0, rgba(190, 159, 230, 0.18)),
                    (12.0, rgba(190, 159, 230, 0.55)),
                    (4.0, rgba(240, 230, 255, 0.95)),
                ] {
                    let col = if self.heat > 0.0 { rgba(255, 120, 120, 0.4 + 0.5 * self.heat) } else { col };
                    g.path(&pts, true, w, col);
                }
            }
            for k in 0..2u8 {
                let on = self.stage > k + 1 || self.done;
                let cur = self.stage == k + 1 && !self.done;
                let col = if on {
                    c::OK
                } else if cur {
                    c::AMBER
                } else {
                    hex(0x2a3446)
                };
                g.disc(sx - 14.0 + f32::from(k) * 28.0, sy + 100.0, 8.0, col);
            }
            let (bx, by) = (CX + self.p[0], CY + self.p[1]);
            let flick = 1.0 + 0.06 * (t * 23.0).sin() + 0.04 * (t * 37.0).sin();
            let mid = if self.heat > 0.0 { rgba(255, 120, 120, 0.6) } else { rgba(190, 159, 230, 0.55) };
            radial_stops(
                g,
                bx,
                by,
                BALL * 2.6 * flick,
                &[
                    (0.0, rgba(255, 255, 255, 1.0)),
                    (0.25, rgba(225, 205, 255, 0.95)),
                    (0.45, mid),
                    (1.0, rgba(79, 195, 247, 0.0)),
                ],
            );
        }

        // The two controls: a vertical and a horizontal slider, a centre mark, the pull as a fill from the centre to
        // the handle, arrows at the ends for the way the field pulls.
        if live {
            slider(g, VS, true, self.ax[1], self.grab == Grab::V);
            slider(g, HS, false, self.ax[0], self.grab == Grab::H);
        }

        // The part: the new coil in its crate, or in the hand.
        if open {
            g.panel(1000.0, 350.0, 160.0, 140.0, 14.0, hex(0x141b27), c::LINE);
            let [x, y] = self.coil;
            g.disc(x, y, 38.0, c::COPPER);
            g.ring(x, y, 38.0, hex(0xf0c08a), 3.0);
            g.disc(x, y, 16.0, hex(0x141b27));
            g.ring(x, y, 26.0, hex(0x7a4a22), 5.0);
        }
    }

    fn hand(&mut self, _cx: &Ctx, _t: f32) -> Input {
        if self.part && !self.coil_set {
            // Pick the coil up, carry it to the slot at 480 px/s, let go there.
            let [sx, sy] = slot_xy();
            if !self.coil_held && !self.hand_carry {
                self.hand_carry = true;
                return Input::hold(self.coil[0], self.coil[1], true);
            }
            let d = (sx - self.coil[0]).hypot(sy - self.coil[1]);
            if d > 2.0 {
                let k = (8.0 / d).min(1.0);
                return Input::hold(
                    self.coil[0] + (sx - self.coil[0]) * k,
                    self.coil[1] + (sy - self.coil[1]) * k,
                    false,
                );
            }
            self.hand_carry = false;
            return Input::release(sx, sy);
        }
        if self.part || self.done {
            return Input::default();
        }
        // A steady hand: where the controls should be this frame. Stage 1 cancels the load and steers the plasma home
        // (a damped pull on its offset and speed); stage 2 cancels the ring's pull out of round.
        let want = if self.stage == 1 {
            let l = self.load(self.time);
            [
                ((-2.0 * self.p[0] - 0.5 * self.v[0] - l[0]) / PULL).clamp(-1.0, 1.0),
                ((-2.0 * self.p[1] - 0.5 * self.v[1] - l[1]) / PULL).clamp(-1.0, 1.0),
            ]
        } else {
            let l = self.ring_load_at(self.time);
            [(-l[0]).clamp(-1.0, 1.0), (-l[1]).clamp(-1.0, 1.0)]
        };
        // The pointer holds the horizontal slider's handle at its mark; a finger on W or S nudges the vertical one.
        let hx = HS[0] + HS[2] / 2.0 + want[0] * HS[2] / 2.0;
        let hy = HS[1] + HS[3] / 2.0;
        let mut i = Input::hold(hx, hy, self.grab != Grab::H);
        let dy = want[1] - self.ax[1];
        if dy > 0.02 {
            i = i.key(Key::S, false);
        } else if dy < -0.02 {
            i = i.key(Key::W, false);
        }
        i
    }

    fn guide_now(&self) -> Option<usize> {
        if self.part {
            None
        } else if self.stage == 2 {
            Some(2)
        } else {
            Some(1)
        }
    }
}

/// One of the two controls: its track, the pull filled from the centre to the handle, the centre mark, an arrow at
/// each end for the way the field pulls, and the handle.
fn slider(g: &Pen, s: [f32; 4], vertical: bool, val: f32, on: bool) {
    let [x, y, w, h] = s;
    g.round(x, y, w, h, 14.0, Some(c::PANEL2), Some((2.0, if on { c::AMBER } else { c::LINE })));
    let len = if vertical { h } else { w };
    let mid = len / 2.0;
    let pos = mid + val * (mid - 22.0);
    let fill = rgba(79, 195, 247, 0.35);
    if vertical {
        g.rect(x + 12.0, y + mid.min(pos), w - 24.0, (pos - mid).abs(), fill);
        g.rect(x + 6.0, y + mid - 1.0, w - 12.0, 3.0, rgba(232, 238, 246, 0.5));
    } else {
        g.rect(x + mid.min(pos), y + 12.0, (pos - mid).abs(), h - 24.0, fill);
        g.rect(x + mid - 1.0, y + 6.0, 3.0, h - 12.0, rgba(232, 238, 246, 0.5));
    }
    let arrow = |ax: f32, ay: f32, a: f32| {
        let q = g.translate(ax, ay).rotate(a);
        q.poly(&[[10.0, 0.0], [-6.0, -9.0], [-6.0, 9.0]], hex(0x4a5568));
    };
    let handle = if on { c::AMBER } else { hex(0xc9d3e0) };
    if vertical {
        arrow(x + w / 2.0, y - 18.0, -FRAC_PI_2);
        arrow(x + w / 2.0, y + h + 18.0, FRAC_PI_2);
        g.round(x - 8.0, y + pos - 14.0, w + 16.0, 28.0, 9.0, Some(handle), None);
        g.rect(x + 6.0, y + pos - 2.0, w - 12.0, 4.0, hex(0x1b2433));
    } else {
        arrow(x - 18.0, y + h / 2.0, PI);
        arrow(x + w + 18.0, y + h / 2.0, 0.0);
        g.round(x + pos - 14.0, y - 8.0, 28.0, h + 16.0, 9.0, Some(handle), None);
        g.rect(x + pos - 2.0, y + 6.0, 4.0, h - 12.0, hex(0x1b2433));
    }
}

#[cfg(test)]
mod tests {
    use crate::games::tests::{fumble_check, plays_to_end};
    use crate::kit::Input;

    #[test]
    fn a_steady_hand_repairs_it_damaged_disabled_and_destroyed() {
        plays_to_end("reactor");
    }

    #[test]
    fn pushing_the_field_hard_over_drives_the_plasma_into_the_wall() {
        // The horizontal slider held at its end stop: the field pulls the plasma into the wall.
        fumble_check("reactor", |_r| Input::hold(1192.0, 622.0, true));
    }
}
