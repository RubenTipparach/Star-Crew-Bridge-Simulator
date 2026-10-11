//! The twin pulse cannon's repair (repair-minigames design 2; the cannon is weapons-and-shields section 2), from
//! `docs/mockups/repairs/turret.js`.
//!
//! The turret's access room: one barrel's emitter on its optical bench, a test beam through two lenses onto a target
//! plate (seen face on, on the right), and the capacitor bank below with its coupling. A round is two phases. The
//! lens phase: slide the two lenses along the rail (drag them, or keys: Up and Down pick a lens, Left and Right move
//! it) until the spot on the plate is smallest and on the crosshair, and hold it there. The lenses work together: each
//! one moves both the focus and the aim, so neither can be set alone. The coupling phase: drag the bank's coupling
//! home along its guide (or push it with Right) and arrive inside the speed band. Too fast slams it: a fumble, the
//! bank arcs and loses its charge. Too slow and it does not latch and rolls back. Each level adds shimmer to the beam,
//! shrinks the target and narrows the band. A disabled turret's first step fits a new focus lens: drag it from the
//! crate into the empty carrier.

use egui::Key;

use crate::games::breakers::{closed, ellipse, Hand, PART_STEP};
use crate::kit::{nearest, Ctx, Game, Input, BAR_H, W};
use crate::pen::{c, hex, rgba, Align, Pen};

/// In the engine.
pub const PORTED: bool = true;

/// A fresh game.
pub fn new() -> Box<dyn Game> {
    Box::new(Turret::default())
}

/// The beam's axis on the bench, y.
const AXIS: f32 = 250.0;
/// The emitter's muzzle, x.
const EMIT_X: f32 = 250.0;
/// Each lens's travel on the rail: x0, x1.
const LENS: [[f32; 2]; 2] = [[320.0, 500.0], [560.0, 740.0]];
/// The target plate, side on.
const PLATE_X: f32 = 830.0;
/// The plate seen face on: x, y, r.
const VIEW: [f32; 3] = [1060.0, 250.0, 150.0];
/// The coupling's guide: its y, where the plug starts, and home (plug in socket).
const GUIDE_Y: f32 = 600.0;
const GUIDE_X0: f32 = 330.0;
const GUIDE_HOME: f32 = 862.0;
/// The speed gauge's full scale, px/s.
const VMAX_GAUGE: f32 = 800.0;
/// Seconds the spot must stay on to finish the lens phase.
const HOLD_S: f32 = 1.0;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Kind {
    #[default]
    Lens,
    Coupling,
}

#[derive(Clone, Copy, Debug, Default)]
struct Plug {
    x: f32,
    v: f32,
    held: bool,
    seated: bool,
    back: f32,
}

#[derive(Clone, Copy, Debug, Default)]
struct Spot {
    r: f32,
    x: f32,
    y: f32,
}

/// The game's state.
#[derive(Default)]
pub struct Turret {
    kind: Kind,
    part: bool,
    lens_part: [f32; 2],
    lens_held: bool,
    lens_set: bool,
    lens: [f32; 2],
    sel: usize,
    grab: Option<usize>,
    target: [f32; 2],
    dir: f32,
    shimmer: f32,
    tol_c: f32,
    tol_r: f32,
    hold: f32,
    plug: Plug,
    band: [f32; 2],
    charge: f32,
    arc: f32,
    latch_t: f32,
    played: bool,
    clock: f32,
    hand: Hand,
}

impl Turret {
    fn lens_x(&self, i: usize) -> f32 {
        LENS[i][0] + (LENS[i][1] - LENS[i][0]) * self.lens[i]
    }

    /// The spot on the plate from the two lens positions: radius and centre offset, px in the face-on view.
    fn spot(&self, t: f32) -> Spot {
        let (ea, eb) = (self.lens[0] - self.target[0], self.lens[1] - self.target[1]);
        let (focus, aim) = (ea + 0.6 * eb, 0.7 * ea - eb);
        let sh = self.shimmer * (t * 2.3).sin() + self.shimmer * 0.6 * (t * 5.1 + 1.0).sin();
        let shy = self.shimmer * (t * 1.9 + 2.0).cos() * 0.8;
        let r = (7.0 + 260.0 * focus.abs() + sh.abs() * 0.3).clamp(7.0, 125.0);
        let off = (300.0 * aim).clamp(-140.0, 140.0);
        Spot { r, x: self.dir.cos() * off + sh, y: self.dir.sin() * off + shy }
    }

    fn on(&self, s: Spot) -> bool {
        s.x.hypot(s.y) < self.tol_c && s.r < self.tol_r
    }
}

/// A cubic Bezier's points (canvas's `bezierCurveTo`).
fn bezier(p0: [f32; 2], p1: [f32; 2], p2: [f32; 2], p3: [f32; 2]) -> Vec<[f32; 2]> {
    (0..=24)
        .map(|i| {
            let u = i as f32 / 24.0;
            let v = 1.0 - u;
            let (a, b, cc, d) = (v * v * v, 3.0 * v * v * u, 3.0 * v * u * u, u * u * u);
            [a * p0[0] + b * p1[0] + cc * p2[0] + d * p3[0], a * p0[1] + b * p1[1] + cc * p2[1] + d * p3[1]]
        })
        .collect()
}

/// A disc shading through `stops` (fraction of the radius, colour) from its centre outwards (a radial gradient).
fn radial_stops(g: &Pen, x: f32, y: f32, r: f32, stops: &[(f32, egui::Color32)]) {
    if let Some(&(_, c0)) = stops.first() {
        g.radial(x, y, r * stops.get(1).map_or(1.0, |s| s.0), c0, stops.get(1).map_or(c0, |s| s.1));
    }
    for w in stops.windows(2).skip(1) {
        let ((f0, c0), (f1, c1)) = (w[0], w[1]);
        let n = 40;
        for i in 0..n {
            let (a0, a1) =
                (std::f32::consts::TAU * i as f32 / n as f32, std::f32::consts::TAU * (i + 1) as f32 / n as f32);
            let (r0, r1) = (r * f0, r * f1);
            g.quad(
                [
                    [x + a0.cos() * r0, y + a0.sin() * r0],
                    [x + a0.cos() * r1, y + a0.sin() * r1],
                    [x + a1.cos() * r1, y + a1.sin() * r1],
                    [x + a1.cos() * r0, y + a1.sin() * r0],
                ],
                [c0, c1, c1, c0],
            );
        }
    }
}

/// A small stable noise in 0-1 from three numbers (the arc's jag; the mockup's `Math.random`).
fn jitter(a: f32, b: f32, k: f32) -> f32 {
    let h = ((a * 12.9898 + b * 78.233 + k * 37.719) as f64).sin() * 43758.5453;
    (h - h.floor()) as f32
}

impl Game for Turret {
    fn id(&self) -> &'static str {
        "turret"
    }

    fn knobs(&self) -> &'static [&'static str] {
        &["shimmer_px", "spot_tol_px", "size_tol_px", "band_lo_px_s", "band_hi_px_s", "band_min_width_px_s"]
    }

    fn phases(&self, _rounds: u32) -> &'static [&'static str] {
        &["lens", "coupling"]
    }

    fn step(&mut self, cx: &mut Ctx) {
        let mut r = cx.dice();
        self.part = cx.part;
        self.kind = if cx.phase_name == "coupling" { Kind::Coupling } else { Kind::Lens };
        self.lens_part = [190.0, 475.0];
        self.lens_held = false;
        self.lens_set = false;
        self.target = [0.2 + 0.6 * r.f(), 0.2 + 0.6 * r.f()];
        // Start the lenses well off their marks.
        for i in 0..2 {
            let v = self.target[i];
            self.lens[i] = if v > 0.5 {
                (v - 0.3 - 0.15 * r.f()).clamp(0.0, 1.0)
            } else {
                (v + 0.3 + 0.15 * r.f()).clamp(0.0, 1.0)
            };
        }
        self.dir = r.f() * std::f32::consts::TAU;
        self.shimmer = cx.knob("shimmer_px");
        self.tol_c = cx.knob("spot_tol_px");
        self.tol_r = cx.knob("size_tol_px");
        self.sel = 0;
        self.grab = None;
        self.hold = 0.0;
        // The coupling: out at the start of its guide on a coupling phase, home otherwise.
        let coupling = self.kind == Kind::Coupling && !self.part;
        self.plug = Plug { x: if coupling { GUIDE_X0 } else { GUIDE_HOME }, seated: !coupling, ..Plug::default() };
        let lo = cx.knob("band_lo_px_s");
        self.band = [lo, (lo + cx.knob("band_min_width_px_s")).max(cx.knob("band_hi_px_s"))];
        self.charge = if coupling { 0.0 } else { 1.0 };
        self.arc = 0.0;
        self.latch_t = 0.0;
        self.played = false;
        self.hand = Hand::default();
    }

    fn update(&mut self, cx: &mut Ctx, dt: f32, input: &Input) {
        self.clock += dt;
        self.arc = (self.arc - dt).max(0.0);
        self.latch_t = (self.latch_t - dt * 1.5).max(0.0);
        if self.plug.seated {
            self.charge = (self.charge + dt * 0.8).min(1.0);
        }
        if self.part && !self.lens_set {
            // The part step: carry the new focus lens to its carrier.
            let [lx, ly] = self.lens_part;
            if input.pressed && (input.x - lx).hypot(input.y - ly) < 60.0 {
                self.lens_held = true;
            }
            if self.lens_held && input.down {
                self.lens_part = [input.x, input.y];
            }
            if self.lens_held && input.released {
                self.lens_held = false;
                let [lx, ly] = self.lens_part;
                if (lx - self.lens_x(0)).hypot(ly - AXIS) < 45.0 {
                    self.lens_set = true;
                    self.lens_part = [self.lens_x(0), AXIS];
                    cx.step_done();
                }
            }
            return;
        }
        if self.played {
            return;
        }
        if self.kind == Kind::Lens {
            // Pick a lens by pointer or keys, slide it along its travel.
            if input.pressed && input.y > AXIS - 90.0 && input.y < AXIS + 130.0 {
                // The nearer lens within 40 px across (two at the ends of their travels are 60 px apart).
                let pts = [Some([self.lens_x(0), input.y]), Some([self.lens_x(1), input.y])];
                if let Some(i) = nearest(&pts, input.x, input.y, 40.0) {
                    self.grab = Some(i);
                    self.sel = i;
                }
            }
            if let (Some(g), true) = (self.grab, input.down) {
                self.lens[g] = ((input.x - LENS[g][0]) / (LENS[g][1] - LENS[g][0])).clamp(0.0, 1.0);
            }
            if input.released {
                self.grab = None;
            }
            if [Key::ArrowUp, Key::W, Key::ArrowDown, Key::S].iter().any(|&k| input.hit(k)) {
                self.sel = 1 - self.sel;
            }
            if input.stick[0] != 0.0 {
                self.lens[self.sel] = (self.lens[self.sel] + input.stick[0] * 0.22 * dt).clamp(0.0, 1.0);
            }
            let on = self.on(self.spot(self.clock));
            self.hold = if on { self.hold + dt } else { (self.hold - 2.0 * dt).max(0.0) };
            if self.hold >= HOLD_S {
                self.hold = HOLD_S;
                self.played = true;
                cx.step_done();
            }
            return;
        }
        // The coupling phase.
        let p = &mut self.plug;
        if p.seated {
            return;
        }
        if p.back > 0.0 {
            // Rolled back from the socket: it eases out a short way and stops.
            p.back = (p.back - dt).max(0.0);
            p.x = GUIDE_X0.max(p.x - 260.0 * dt * (p.back / 0.5));
            p.v = 0.0;
            return;
        }
        if input.pressed && (input.x - p.x).abs() < 60.0 && (input.y - GUIDE_Y).abs() < 50.0 {
            p.held = true;
        }
        if p.held && input.down {
            let nx = input.x.clamp(GUIDE_X0, GUIDE_HOME);
            let raw = (nx - p.x) / dt.max(1e-3);
            p.v += (raw - p.v) * (dt / 0.08).min(1.0);
            p.x = nx;
        } else {
            if input.released {
                p.held = false;
            }
            if input.stick[0] > 0.0 {
                p.v += 520.0 * dt;
            } else if input.stick[0] < 0.0 {
                p.v -= 520.0 * dt;
            } else {
                p.v *= (-1.6 * dt).exp();
            }
            p.x += p.v * dt;
            if p.x < GUIDE_X0 {
                p.x = GUIDE_X0;
                p.v = 0.0;
            }
        }
        if p.x >= GUIDE_HOME - 0.5 {
            let v = p.v;
            p.held = false;
            if v > self.band[1] {
                // Slammed: the bank arcs and dumps its charge; the coupling kicks back out.
                self.arc = 0.9;
                self.charge = 0.0;
                *p = Plug { x: GUIDE_HOME - 6.0, back: 0.8, ..*p };
                p.v = 0.0;
                cx.fumble("The bank arcs: 10 HP, charge lost");
            } else if v < self.band[0] {
                // Too soft: the latch does not catch.
                p.x = GUIDE_HOME - 4.0;
                p.back = 0.5;
                p.v = 0.0;
            } else {
                p.x = GUIDE_HOME;
                p.v = 0.0;
                p.seated = true;
                self.latch_t = 1.0;
                self.played = true;
                cx.step_done();
            }
        }
    }

    fn draw(&self, g: &Pen, _cx: &Ctx, t: f32, _input: &Input) {
        use std::f32::consts::{FRAC_PI_2, TAU};
        g.rect(0.0, BAR_H, W, 720.0, hex(0x070b12));
        let lens_round = self.kind == Kind::Lens || self.part;
        let bench_dim = !lens_round;
        // ---- The optical bench: the emitter, the rail, the lenses, the beam.
        let b = g.alpha(if bench_dim { 0.45 } else { 1.0 });
        b.panel(40.0, 110.0, 840.0, 290.0, 18.0, hex(0x0b111b), c::LINE);
        // The emitter's housing: two barrels' breeches, the upper one on test.
        b.round(70.0, 160.0, 180.0, 180.0, 14.0, Some(hex(0x1a2333)), Some((3.0, c::STEEL)));
        for yy in [AXIS - 40.0, AXIS + 40.0] {
            b.round(210.0, yy - 22.0, 50.0, 44.0, 8.0, Some(hex(0x222d40)), Some((2.0, c::STEEL)));
        }
        for i in 0..5 {
            b.rect(90.0, 180.0 + i as f32 * 30.0, 90.0, 12.0, hex(0x2a3446));
        }
        b.disc(245.0, AXIS - 40.0, 9.0, if self.part { hex(0x2a3446) } else { c::ACCENT });
        b.text("EMITTER", 160.0, 360.0, 15.0, c::DIM, Align::Center);
        // The rail under the lenses, and each lens's travel.
        b.rect(EMIT_X + 20.0, AXIS + 80.0, PLATE_X - EMIT_X - 10.0, 12.0, hex(0x283246));
        for [x0, x1] in LENS {
            b.round(x0 - 8.0, AXIS + 100.0, x1 - x0 + 16.0, 14.0, 7.0, Some(hex(0x121a26)), Some((2.0, c::LINE)));
        }
        // The plate, side on.
        b.rect(PLATE_X, AXIS - 120.0, 12.0, 240.0, c::STEEL);
        // The beam: collimated from the emitter, bent by each lens, onto the plate.
        let s = self.spot(self.clock);
        if !self.part {
            let (xa, xb) = (self.lens_x(0), self.lens_x(1));
            let hb = 30.0 * (0.45 + 0.6 * (self.lens[0] - self.target[0]).abs()) + 6.0;
            let (hp, cp) = (s.r * 0.45, AXIS - 40.0 + s.y * 0.35);
            b.poly(
                &[
                    [EMIT_X, AXIS - 40.0 - 14.0],
                    [xa, AXIS - 40.0 - 26.0],
                    [xb, AXIS - 40.0 - hb + s.y * 0.15],
                    [PLATE_X, cp - hp],
                    [PLATE_X, cp + hp],
                    [xb, AXIS - 40.0 + hb + s.y * 0.15],
                    [xa, AXIS - 40.0 + 26.0],
                    [EMIT_X, AXIS - 40.0 + 14.0],
                ],
                if bench_dim { rgba(79, 195, 247, 0.08) } else { rgba(79, 195, 247, 0.22) },
            );
        }
        // The lenses on their carriers (the beam runs through the upper barrel's line).
        for i in 0..2 {
            let x = self.lens_x(i);
            let empty = self.part && i == 0 && !self.lens_set;
            b.rect(x - 10.0, AXIS + 20.0, 20.0, 60.0, hex(0x30394c));
            b.round(x - 26.0, AXIS + 70.0, 52.0, 18.0, 6.0, Some(hex(0x3a4558)), None);
            if empty {
                b.dashed(&closed(ellipse(x, AXIS - 40.0, 16.0, 62.0)), 3.0, c::AMBER, 8.0, 6.0);
            } else {
                let e = ellipse(x, AXIS - 40.0, 14.0, 60.0);
                b.poly(&e, rgba(160, 220, 255, 0.35));
                let on = lens_round && !self.part && self.sel == i;
                b.path(&e, true, 4.0, if on { c::AMBER } else { c::STEEL });
            }
        }
        // ---- The plate face on: crosshair, the target ring, the spot, the hold.
        let dim_view = !lens_round || self.part;
        let [vx, vy, vr] = VIEW;
        g.disc(vx, vy, vr, hex(0x0b1018));
        g.ring(vx, vy, vr, c::LINE, 4.0);
        g.line(vx - vr + 10.0, vy, vx + vr - 10.0, vy, 2.0, hex(0x2b3a50));
        g.line(vx, vy - vr + 10.0, vx, vy + vr - 10.0, 2.0, hex(0x2b3a50));
        for rr in [50.0, 100.0] {
            g.ring(vx, vy, rr, hex(0x18222f), 2.0);
        }
        if !self.part {
            let on = self.on(s);
            let (sx, sy) = (vx + s.x, vy + s.y);
            let a = if dim_view { 0.25 } else { 1.0 };
            radial_stops(
                g,
                sx,
                sy,
                s.r,
                &[(0.0, rgba(230, 250, 255, a)), (0.4, rgba(79, 195, 247, 0.8 * a)), (1.0, rgba(79, 195, 247, 0.0))],
            );
            // The target ring: dashed while off, solid green while on.
            let col = if on || self.played { c::OK } else { c::AMBER };
            if on {
                g.ring(vx, vy, self.tol_c + 6.0, col, 3.0);
            } else {
                crate::games::breakers::dashed_ring(g, vx, vy, self.tol_c + 6.0, col, 3.0, 6.0, 6.0);
            }
            if self.kind == Kind::Lens && self.hold > 0.0 {
                g.arc(vx, vy, vr + 12.0, -FRAC_PI_2, -FRAC_PI_2 + TAU * (self.hold / HOLD_S), 8.0, c::OK);
            }
        }
        // ---- The capacitor bank and the coupling's guide.
        let coupling_round = self.kind == Kind::Coupling && !self.part;
        let k = g.alpha(if coupling_round { 1.0 } else { 0.45 });
        k.panel(900.0, 440.0, 340.0, 250.0, 16.0, hex(0x111926), c::LINE);
        k.text("BANK", 1070.0, 466.0, 16.0, c::DIM, Align::Center);
        for i in 0..8 {
            let (x, y0, h) = (935.0 + i as f32 * 36.0, 490.0, 170.0);
            k.round(x, y0, 26.0, h, 5.0, Some(hex(0x1a2232)), None);
            let lit = (self.charge * 8.0 - i as f32).clamp(0.0, 1.0);
            if lit > 0.0 {
                k.round(x, y0 + h * (1.0 - lit), 26.0, h * lit, 5.0, Some(c::ACCENT), None);
            }
        }
        let p = self.plug;
        // The socket on the bank's face.
        k.round(
            880.0,
            GUIDE_Y - 34.0,
            30.0,
            68.0,
            6.0,
            Some(if p.seated { hex(0x1f3a2a) } else { hex(0x2a1c10) }),
            Some((3.0, if p.seated { c::OK } else { c::AMBER })),
        );
        // The guide rails.
        k.rect(GUIDE_X0 - 40.0, GUIDE_Y - 42.0, GUIDE_HOME - GUIDE_X0 + 60.0, 6.0, hex(0x283246));
        k.rect(GUIDE_X0 - 40.0, GUIDE_Y + 36.0, GUIDE_HOME - GUIDE_X0 + 60.0, 6.0, hex(0x283246));
        // The cable from the bulkhead to the plug.
        k.round(40.0, GUIDE_Y - 60.0, 60.0, 120.0, 10.0, Some(hex(0x1a2333)), Some((2.0, c::STEEL)));
        k.path(
            &bezier([100.0, GUIDE_Y], [180.0, GUIDE_Y + 30.0], [p.x - 160.0, GUIDE_Y + 20.0], [p.x - 40.0, GUIDE_Y]),
            false,
            12.0,
            c::COPPER,
        );
        // The plug.
        k.round(
            p.x - 50.0,
            GUIDE_Y - 30.0,
            70.0,
            60.0,
            8.0,
            Some(hex(0x3b4558)),
            Some((3.0, if p.held { c::AMBER } else { c::STEEL })),
        );
        for yy in [-14.0, 0.0, 14.0] {
            k.rect(p.x + 20.0, GUIDE_Y + yy - 3.0, 14.0, 6.0, hex(0xc9a26b));
        }
        if self.latch_t > 0.0 {
            k.ring(895.0, GUIDE_Y, 40.0 + 40.0 * (1.0 - self.latch_t), rgba(61, 220, 132, self.latch_t), 4.0);
        }
        // The speed gauge: the band bracketed, past it hatched red, a marker for now.
        let (gx0, gx1, gy) = (GUIDE_X0 - 40.0, 860.0, 668.0);
        let vxp = |v: f32| gx0 + (gx1 - gx0) * (v / VMAX_GAUGE).clamp(0.0, 1.0);
        k.round(gx0, gy - 10.0, gx1 - gx0, 20.0, 10.0, Some(hex(0x151d2a)), None);
        let [lo, hi] = self.band;
        k.rect(vxp(lo), gy - 10.0, vxp(hi) - vxp(lo), 20.0, rgba(61, 220, 132, 0.35));
        k.hatch(vxp(hi), gy - 10.0, gx1 - vxp(hi), 20.0, rgba(255, 71, 87, 0.7));
        for (bx, d) in [(vxp(lo), 1.0), (vxp(hi), -1.0)] {
            k.path(
                &[[bx + 8.0 * d, gy - 16.0], [bx, gy - 16.0], [bx, gy + 16.0], [bx + 8.0 * d, gy + 16.0]],
                false,
                3.0,
                c::OK,
            );
        }
        let mv = p.v.max(0.0);
        let mc = if mv > hi {
            c::DANGER
        } else if mv >= lo {
            c::OK
        } else {
            c::FG
        };
        k.poly(&[[vxp(mv), gy - 12.0], [vxp(mv) - 9.0, gy - 26.0], [vxp(mv) + 9.0, gy - 26.0]], mc);
        // The arc: a jagged discharge from socket to bank.
        if self.arc > 0.0 {
            let col = rgba(190, 159, 230, (self.arc * 1.5).min(1.0));
            let frame = (t * 30.0).floor();
            for kk in 0..4 {
                let (mut x, mut pts) = (880.0, vec![[880.0, GUIDE_Y]]);
                for j in 0..7 {
                    x += 30.0 + jitter(frame, kk as f32, j as f32) * 20.0;
                    let y = GUIDE_Y - 120.0 + jitter(frame, kk as f32 + 9.0, j as f32) * 240.0;
                    pts.push([x, y.clamp(450.0, 690.0)]);
                }
                g.path(&pts, false, 4.0, col);
            }
        }
        // The part step: the crate with the new lens.
        if self.part && !self.lens_set {
            g.panel(120.0, 408.0, 140.0, 134.0, 14.0, hex(0x141b27), c::LINE);
            let e = ellipse(self.lens_part[0], self.lens_part[1], 14.0, 60.0);
            g.poly(&e, rgba(160, 220, 255, 0.5));
            g.path(&e, true, 4.0, hex(0xcfe9ff));
        }
    }

    fn hand(&mut self, _cx: &Ctx, _t: f32) -> Input {
        if self.hand.resting() {
            return Input::default();
        }
        if self.part && !self.lens_set {
            return self.hand.carry(1, self.lens_part, [self.lens_x(0), AXIS], PART_STEP);
        }
        if self.played {
            return self.hand.idle();
        }
        if self.kind == Kind::Lens {
            // Slide each lens onto its mark in turn, then keep still while the hold fills.
            for (i, [x0, x1]) in LENS.into_iter().enumerate() {
                if (self.lens[i] - self.target[i]).abs() > 1e-4 {
                    let to = x0 + (x1 - x0) * self.target[i];
                    return self.hand.carry(10 + i as u32, [self.lens_x(i), AXIS + 100.0], [to, AXIS + 100.0], 8.0);
                }
            }
            return self.hand.idle();
        }
        // The coupling: a steady push at the middle of the band, in equal steps that land exactly home.
        if self.plug.seated || self.plug.back > 0.0 {
            return self.hand.idle();
        }
        let v = (self.band[0] + self.band[1]) / 2.0;
        let n = ((GUIDE_HOME - GUIDE_X0) / (v / 60.0)).round().max(1.0);
        let step = (GUIDE_HOME - GUIDE_X0) / n;
        self.hand.carry(20, [self.plug.x, GUIDE_Y], [GUIDE_HOME, GUIDE_Y], step)
    }

    fn guide_now(&self) -> Option<usize> {
        if self.part {
            None
        } else if self.kind == Kind::Coupling {
            Some(3)
        } else {
            Some(0)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::games::tests::{fumble_check, open, plays_to_end};
    use crate::kit::Input;
    use egui::Key;
    use sc_core::repair::State;

    #[test]
    fn a_steady_hand_repairs_it_damaged_disabled_and_destroyed() {
        plays_to_end("turret");
    }

    #[test]
    fn slamming_the_coupling_home_arcs_the_bank() {
        // A twin of the job plays the lenses with the steady hand; then the coupling is shoved home on the arrow.
        let mut twin = open("turret", State::Damaged);
        fumble_check("turret", move |r| {
            if r.phase == 0 {
                let i = twin.hand();
                twin.update(1.0 / 60.0, &i);
                return i;
            }
            Input::default().key(Key::ArrowRight, false)
        });
    }
}
