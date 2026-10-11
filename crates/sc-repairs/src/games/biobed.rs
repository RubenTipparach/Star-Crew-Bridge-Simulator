//! A medbay biobed's repair (repair-minigames design 2), from `docs/mockups/repairs/biobed.js`.
//!
//! Sensor calibration. The bed's monitor shows a channel's reference trace (wide, dim) and the bed's live trace over
//! it. Turn the two knobs, GAIN and PHASE (drag round the knob, the wheel over it, or Left and Right to pick one and
//! Up and Down to turn it), until the live trace lies on the reference and turns green, and hold it there 2 s. A round
//! tunes both channels, ECG then SpO2 (its two phases); each level tightens the match, adds trace noise and lets the
//! patient's signal wander. A knob turned into its red zone over-drives the sensor: a fumble, the bed alarms, the knob
//! falls back. A disabled bed's first step fits the new sensor module: drag it from the crate into its socket on the
//! arch. The knobs sit behind the control box's cover (the kit's screws, from the file).

use std::f32::consts::{PI, TAU};

use egui::Key;

use crate::kit::{Ctx, Game, Input, BAR_H, H, W};
use crate::pen::{c, hex, rgba, Align, Pen};

/// In the engine.
pub const PORTED: bool = true;

/// A fresh game.
pub fn new() -> Box<dyn Game> {
    Box::new(Biobed::default())
}

const ALARM: &str = "The bed alarms";
/// The monitor: x, y, w, h.
const MON: [f32; 4] = [600.0, 96.0, 640.0, 380.0];
/// Its trace area: left, right, the mid line, px per unit.
const SX0: f32 = 630.0;
const SX1: f32 = 1210.0;
const SY: f32 = 300.0;
const SCALE: f32 = 100.0;
/// One beat across the screen, px.
const PERIOD: f32 = 190.0;
const KX: [f32; 2] = [760.0, 980.0];
const KY: f32 = 588.0;
const KR: f32 = 56.0;
const NAMES: [&str; 2] = ["GAIN", "PHASE"];
/// A knob past this is over-driven.
const RED: f32 = 0.9;
/// The sensor arch over the bed: centre and radius.
const ARCH: [f32; 3] = [310.0, 450.0, 200.0];
/// The spare module's crate (part step): x, y, w, h.
const CRATE: [f32; 4] = [1100.0, 560.0, 135.0, 115.0];
/// A round's channels, one a phase.
const CHANNELS: [&str; 2] = ["ECG", "SpO2"];
/// The arch's pad lit for each channel.
const PAD_OF: [usize; 2] = [0, 2];

fn socket() -> [f32; 2] {
    [ARCH[0] + ARCH[2] * (-2.09f32).cos(), ARCH[1] + ARCH[2] * (-2.09f32).sin()]
}

fn g2(p: f32, m: f32, w: f32) -> f32 {
    (-((p - m) / w).powi(2)).exp()
}

/// A channel's wave over one beat, `p` 0-1.
fn wave(ch: usize, p: f32) -> f32 {
    if ch == 0 {
        0.12 * g2(p, 0.12, 0.03) - 0.15 * g2(p, 0.27, 0.01) + g2(p, 0.3, 0.014) - 0.25 * g2(p, 0.33, 0.012)
            + 0.3 * g2(p, 0.55, 0.05)
    } else {
        0.9 * g2(p, 0.25, 0.08) + 0.35 * g2(p, 0.5, 0.07) - 0.3
    }
}

fn wrap(a: f32) -> f32 {
    a.sin().atan2(a.cos())
}

/// A knob's setting (0-1) as its angle.
fn ang(k: f32) -> f32 {
    0.75 * PI + 1.5 * PI * k
}

fn fract(x: f32) -> f32 {
    x - x.floor()
}

/// The points of a cubic Bezier from `a` to `d`.
fn bezier(a: [f32; 2], b: [f32; 2], c2: [f32; 2], d: [f32; 2], n: usize) -> Vec<[f32; 2]> {
    (0..=n)
        .map(|i| {
            let t = i as f32 / n as f32;
            let u = 1.0 - t;
            let k = [u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t];
            [
                k[0] * a[0] + k[1] * b[0] + k[2] * c2[0] + k[3] * d[0],
                k[0] * a[1] + k[1] * b[1] + k[2] * c2[1] + k[3] * d[1],
            ]
        })
        .collect()
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Phase {
    #[default]
    Part,
    Tune,
    Done,
}

#[derive(Clone, Debug, Default)]
struct Module {
    x: f32,
    y: f32,
    held: bool,
    pad: bool,
    set: bool,
}

/// The game's state.
#[derive(Default)]
pub struct Biobed {
    t: f32,
    phase: Phase,
    ch: usize,
    base: [f32; 2],
    k: [f32; 2],
    ph: f32,
    tol: f32,
    drift: f32,
    noise: f32,
    hold_s: f32,
    hold: f32,
    sel: usize,
    drag: Option<usize>,
    last_a: f32,
    keys: bool,
    alarm: f32,
    module: Module,
    /// The hand: the knob it is turning and the angle its finger is at, or None.
    hand_knob: Option<(usize, f32)>,
    /// The hand: carrying the module.
    hand_carry: bool,
}

impl Biobed {
    /// Where the patient's signal wants each knob now.
    fn targets_at(&self, t: f32) -> [f32; 2] {
        [
            self.base[0] + self.drift * 0.5 * (t * 0.45 + self.ph).sin(),
            self.base[1] + self.drift * (t * 0.6 + self.ph * 2.0).sin(),
        ]
    }

    fn matched(&self) -> bool {
        let tg = self.targets_at(self.t);
        (0..2).all(|i| (self.k[i] - tg[i]).abs() <= self.tol)
    }

    fn part_step(&mut self, cx: &mut Ctx, dt: f32, input: &Input) {
        let p = &mut self.module;
        if input.stick != [0.0, 0.0] {
            p.x += input.stick[0] * 480.0 * dt;
            p.y += input.stick[1] * 480.0 * dt;
            p.pad = true;
        }
        if input.pressed && (input.x - p.x).hypot(input.y - p.y) < 50.0 {
            p.held = true;
        }
        if p.held && input.down {
            p.x = input.x;
            p.y = input.y;
        }
        if (p.held && input.released) || (p.pad && input.action_pressed) {
            p.held = false;
            p.pad = false;
            let [sx, sy] = socket();
            if (p.x - sx).hypot(p.y - sy) < 40.0 {
                p.set = true;
                p.x = sx;
                p.y = sy;
                cx.step_done();
            }
        }
    }

    fn tune(&mut self, cx: &mut Ctx, dt: f32, input: &Input) {
        if input.hit(Key::ArrowLeft) || input.hit(Key::A) {
            self.sel = 0;
            self.keys = true;
        }
        if input.hit(Key::ArrowRight) || input.hit(Key::D) {
            self.sel = 1;
            self.keys = true;
        }
        if input.stick[1] != 0.0 {
            self.k[self.sel] = (self.k[self.sel] - input.stick[1] * 0.25 * dt).max(0.0);
            self.keys = true;
        }
        let over = KX.iter().position(|&x| (input.x - x).hypot(input.y - KY) < KR + 22.0);
        if let (true, Some(o)) = (input.pressed, over) {
            self.drag = Some(o);
            self.sel = o;
            self.last_a = (input.y - KY).atan2(input.x - KX[o]);
        }
        if let Some(d) = self.drag {
            if !input.down {
                self.drag = None;
            } else {
                let a = (input.y - KY).atan2(input.x - KX[d]);
                if (input.x - KX[d]).hypot(input.y - KY) > 10.0 {
                    self.k[d] = (self.k[d] + wrap(a - self.last_a) / (1.5 * PI)).max(0.0);
                }
                self.last_a = a;
            }
        }
        if let (true, Some(o)) = (input.wheel != 0.0, over) {
            self.k[o] = (self.k[o] - input.wheel * 0.01).max(0.0);
        }
        if let Some(hot) = self.k.iter().position(|&k| k > RED) {
            self.k[hot] = 0.78;
            self.drag = None;
            self.alarm = 1.6;
            self.hold = 0.0;
            cx.fumble(ALARM);
            return;
        }
        if self.matched() {
            self.hold += dt;
            if self.hold >= self.hold_s {
                self.phase = Phase::Done;
                cx.step_done();
            }
        } else {
            self.hold = 0.0;
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn trace(&self, g: &Pen, k0: f32, k1: f32, col: egui::Color32, w: f32, noise: f32, t: f32) {
        let mut pts = Vec::with_capacity(200);
        let mut x = SX0;
        while x <= SX1 {
            let p = (x - SX0) / PERIOD - t * 0.5 + k1;
            let n = if noise != 0.0 { noise * (x * 0.9 + t * 23.0).sin() * (x * 0.37 - t * 17.0).sin() } else { 0.0 };
            pts.push([x, SY - SCALE * (0.3 + 1.2 * k0) * wave(self.ch, fract(p)) + n]);
            x += 3.0;
        }
        g.path(&pts, false, w, col);
    }

    fn knob(&self, g: &Pen, i: usize) {
        let (x, k, live) = (KX[i], self.k[i], self.phase == Phase::Tune);
        // The scale: the working sweep, then the red zone, ticked across so it reads without its colour.
        g.arc(x, KY, KR + 18.0, ang(0.0), ang(RED), 10.0, hex(0x1f2a37));
        g.arc(x, KY, KR + 18.0, ang(RED), ang(1.0), 10.0, c::DANGER);
        for j in 1..4 {
            let a = ang(RED + (j as f32 * (1.0 - RED)) / 4.0);
            g.line(
                x + a.cos() * (KR + 12.0),
                KY + a.sin() * (KR + 12.0),
                x + a.cos() * (KR + 24.0),
                KY + a.sin() * (KR + 24.0),
                3.0,
                hex(0x070b12),
            );
        }
        for j in 0..=9 {
            let a = ang(j as f32 / 10.0);
            g.disc(x + a.cos() * (KR + 32.0), KY + a.sin() * (KR + 32.0), 2.5, hex(0x3a4658));
        }
        g.disc(x, KY, KR, hex(0x1d2738));
        g.ring(x, KY, KR, if live { hex(0x9aa6b6) } else { hex(0x4a5566) }, 4.0);
        for j in 0..12 {
            let a = j as f32 * PI / 6.0 + ang(k);
            g.disc(x + a.cos() * (KR - 8.0), KY + a.sin() * (KR - 8.0), 4.0, hex(0x2a3446));
        }
        let a = ang(k);
        g.path_round(
            &[[x + a.cos() * 12.0, KY + a.sin() * 12.0], [x + a.cos() * (KR - 6.0), KY + a.sin() * (KR - 6.0)]],
            6.0,
            if k > RED - 0.06 { c::DANGER } else { c::FG },
        );
        if self.keys && self.sel == i && live {
            g.ring(x, KY, KR + 42.0, c::AMBER, 3.0);
        }
        g.text(NAMES[i], x, KY + 98.0, 18.0, c::DIM, Align::Center);
    }

    fn module_glyph(g: &Pen, x: f32, y: f32) {
        g.round(x - 26.0, y - 20.0, 52.0, 40.0, 8.0, Some(c::STEEL), None);
        g.disc(x, y, 12.0, hex(0x0b111b));
        g.ring(x, y, 12.0, c::ACCENT, 3.0);
        g.rect(x - 20.0, y + 20.0, 40.0, 6.0, hex(0x4a5566));
    }
}

impl Game for Biobed {
    fn id(&self) -> &'static str {
        "biobed"
    }

    fn knobs(&self) -> &'static [&'static str] {
        &["tolerance_frac", "drift_frac", "noise_px", "hold_s"]
    }

    fn phases(&self, _rounds: u32) -> &'static [&'static str] {
        &["ch1", "ch2"]
    }

    fn step(&mut self, cx: &mut Ctx) {
        let mut r = cx.dice();
        let base = [0.18 + 0.5 * r.f(), 0.18 + 0.55 * r.f()];
        let k = base.map(|b| {
            let off = 0.18 + 0.12 * r.f();
            // The JS draws its coin only when the first test passes: `b + off < 0.82 && (r() < 0.5 || ...)`.
            if b + off < 0.82 && (r.f() < 0.5 || b - off < 0.02) {
                b + off
            } else {
                b - off
            }
        });
        self.t = 0.0;
        self.phase = if cx.part { Phase::Part } else { Phase::Tune };
        self.ch = if cx.phase_name == "ch2" { 1 } else { 0 };
        self.base = base;
        self.k = k;
        self.ph = r.f() * TAU;
        self.tol = cx.knob("tolerance_frac");
        self.drift = cx.knob("drift_frac");
        self.noise = cx.knob("noise_px");
        self.hold_s = cx.knob("hold_s");
        self.hold = 0.0;
        self.sel = 0;
        self.drag = None;
        self.last_a = 0.0;
        self.keys = false;
        self.alarm = 0.0;
        self.module = Module {
            x: CRATE[0] + CRATE[2] / 2.0,
            y: CRATE[1] + CRATE[3] / 2.0,
            held: false,
            pad: false,
            set: !cx.part,
        };
        self.hand_knob = None;
        self.hand_carry = false;
    }

    fn update(&mut self, cx: &mut Ctx, dt: f32, input: &Input) {
        self.t += dt;
        self.alarm = (self.alarm - dt).max(0.0);
        match self.phase {
            Phase::Part => self.part_step(cx, dt, input),
            Phase::Tune => self.tune(cx, dt, input),
            Phase::Done => {}
        }
    }

    fn draw(&self, g: &Pen, _cx: &Ctx, t: f32, _input: &Input) {
        g.rect(0.0, BAR_H, W, H, hex(0x070b12));
        g.rect(0.0, 690.0, W, H - 690.0, hex(0x0a0f17));
        let live = matches!(self.phase, Phase::Tune | Phase::Done);
        let [ax, ay, ar] = ARCH;
        // The cable from the arch to the monitor.
        g.path(
            &bezier([ax + ar, ay - 10.0], [580.0, 470.0], [560.0, 330.0], [MON[0], 330.0], 24),
            false,
            8.0,
            hex(0x2a3446),
        );
        // The bed: pedestal, frame, mattress, the patient under a blanket.
        g.rect(250.0, 520.0, 120.0, 170.0, hex(0x1d2738));
        g.rect(200.0, 676.0, 220.0, 14.0, hex(0x2a3446));
        g.round(60.0, 492.0, 500.0, 34.0, 8.0, Some(hex(0x3a4658)), None);
        g.round(70.0, 452.0, 480.0, 44.0, 14.0, Some(hex(0xc9d2dc)), None);
        g.round(80.0, 430.0, 82.0, 30.0, 12.0, Some(hex(0xe8eef6)), None);
        g.disc(126.0, 418.0, 25.0, hex(0x8a6a58));
        let mut blanket = bezier([150.0, 456.0], [170.0, 400.0], [300.0, 404.0], [380.0, 420.0], 16);
        blanket.extend(bezier([380.0, 420.0], [450.0, 430.0], [500.0, 426.0], [540.0, 456.0], 16).into_iter().skip(1));
        g.poly(&blanket, hex(0x3d6f8f));
        // The sensor arch, its pads (the channel's lit), the module socket, the alarm lamp.
        g.arc(ax, ay, ar, PI, TAU, 34.0, hex(0x141b27));
        g.arc(ax, ay, ar, PI, TAU, 24.0, hex(0x2a3446));
        for j in 0..5 {
            if j == 1 {
                continue;
            }
            let a = PI + PI * (j as f32 + 0.5) / 5.0;
            let lit = live && j == PAD_OF[self.ch];
            g.disc(ax + ar * a.cos(), ay + ar * a.sin(), 9.0, if lit { c::ACCENT } else { hex(0x3d4a5c) });
        }
        let [sx, sy] = socket();
        if self.module.set {
            Self::module_glyph(g, sx, sy);
        } else {
            g.disc(sx, sy, 28.0, hex(0x120d08));
            g.ring(sx, sy, 28.0, c::AMBER, 4.0);
        }
        let lamp = self.alarm > 0.0 && (t * 18.0).sin() > 0.0;
        g.poly(
            &[[ax, ay - ar - 40.0], [ax - 20.0, ay - ar - 6.0], [ax + 20.0, ay - ar - 6.0]],
            if lamp { c::DANGER } else { hex(0x3a2226) },
        );
        if self.alarm > 0.0 {
            g.rect(40.0, 220.0, 540.0, 470.0, rgba(255, 71, 87, 0.12 * if lamp { 1.0 } else { 0.4 }));
        }
        // The monitor: channel tabs, the grid, the reference and the live trace, the hold.
        let [mx, my, mw, mh] = MON;
        g.panel(mx, my, mw, mh, 16.0, hex(0x0d131c), c::LINE);
        let n = CHANNELS.len();
        let tw = 110.0f32.min((mw - 40.0) / n as f32 - 8.0);
        for (i, name) in CHANNELS.iter().enumerate() {
            let x = mx + 20.0 + i as f32 * (tw + 8.0);
            let done = i < self.ch || (i == self.ch && self.phase == Phase::Done);
            let cur = i == self.ch && self.phase != Phase::Part;
            g.round(
                x,
                my + 10.0,
                tw,
                28.0,
                8.0,
                Some(if cur { hex(0x1d2a3c) } else { hex(0x121a25) }),
                cur.then_some((2.0, c::ACCENT)),
            );
            g.text(
                name,
                x + 12.0,
                my + 24.0,
                16.0,
                if done {
                    c::OK
                } else if cur {
                    c::FG
                } else {
                    c::DIM
                },
                Align::Left,
            );
            if done {
                g.path(
                    &[[x + tw - 26.0, my + 24.0], [x + tw - 20.0, my + 30.0], [x + tw - 10.0, my + 18.0]],
                    false,
                    3.0,
                    c::OK,
                );
            }
        }
        g.rect(SX0 - 10.0, my + 48.0, SX1 - SX0 + 20.0, mh - 70.0, hex(0x04080c));
        let mut x = SX0;
        while x <= SX1 {
            g.line(x, my + 50.0, x, my + mh - 24.0, 1.0, hex(0x0f1a24));
            x += 40.0;
        }
        let mut y = my + 60.0;
        while y < my + mh - 24.0 {
            g.line(SX0 - 8.0, y, SX1 + 8.0, y, 1.0, hex(0x0f1a24));
            y += 40.0;
        }
        let s = g.clip(SX0 - 10.0, my + 48.0, SX1 - SX0 + 20.0, mh - 70.0);
        let ok = self.phase == Phase::Done || (self.phase == Phase::Tune && self.matched());
        if self.phase == Phase::Part {
            s.rect(SX0, SY - 1.0, SX1 - SX0, 3.0, hex(0x1b2433));
        } else {
            let tg = self.targets_at(self.t);
            self.trace(&s, tg[0], tg[1], hex(0x24405a), 14.0, 0.0, self.t);
            let noise = if self.phase == Phase::Done { 0.0 } else { self.noise };
            self.trace(&s, self.k[0], self.k[1], if ok { c::OK } else { c::ACCENT }, 3.0, noise, self.t);
        }
        // The hold: a bar along the screen's foot.
        let hf = if self.phase == Phase::Done { 1.0 } else { self.hold / self.hold_s.max(1e-6) };
        g.rect(SX0, my + mh - 18.0, SX1 - SX0, 8.0, hex(0x121a25));
        g.rect(SX0, my + mh - 18.0, (SX1 - SX0) * hf.min(1.0), 8.0, c::OK);
        // The knobs.
        g.panel(640.0, 490.0, 460.0, 210.0, 18.0, hex(0x0d131c), c::LINE);
        self.knob(g, 0);
        self.knob(g, 1);
        if !self.module.set {
            let [cx0, cy0, cw, ch] = CRATE;
            g.panel(cx0, cy0, cw, ch, 14.0, hex(0x141b27), c::LINE);
            Self::module_glyph(g, self.module.x, self.module.y);
            g.ring(self.module.x, self.module.y, 40.0, hex(0xf0c08a), 3.0);
        }
    }

    fn hand(&mut self, _cx: &Ctx, _t: f32) -> Input {
        match self.phase {
            Phase::Part => {
                // Pick the module up, carry it to the socket a few px a frame, let go there.
                let [sx, sy] = socket();
                let (mx, my) = (self.module.x, self.module.y);
                if !self.module.held && !self.hand_carry {
                    self.hand_carry = true;
                    return Input::hold(mx, my, true);
                }
                let d = (sx - mx).hypot(sy - my);
                if d > 2.0 {
                    let k = (24.0 / d).min(1.0);
                    return Input::hold(mx + (sx - mx) * k, my + (sy - my) * k, false);
                }
                self.hand_carry = false;
                Input::release(sx, sy)
            }
            Phase::Done => Input::default(),
            Phase::Tune => {
                // Turn whichever knob is further off by dragging a finger round it, one knob at a time; let go to
                // change knobs. Aim at where the signal will be next frame.
                let tg = self.targets_at(self.t + 1.0 / 60.0);
                let err = [tg[0] - self.k[0], tg[1] - self.k[1]];
                let want = if err[1].abs() >= err[0].abs() * 1.5 || err[0].abs() < self.tol * 0.3 { 1 } else { 0 };
                let rad = KR * 0.7;
                match self.hand_knob {
                    Some((i, a)) if self.drag == Some(i) => {
                        if i != want && err[i].abs() < self.tol * 0.3 {
                            self.hand_knob = None;
                            return Input::release(KX[i] + a.cos() * rad, KY + a.sin() * rad);
                        }
                        let na = a + (err[i] * 1.5 * PI).clamp(-0.08, 0.08);
                        self.hand_knob = Some((i, na));
                        Input::hold(KX[i] + na.cos() * rad, KY + na.sin() * rad, false)
                    }
                    Some(_) => {
                        // The game let go of the knob (a fall back): lift the finger and start again.
                        self.hand_knob = None;
                        Input::default()
                    }
                    None => {
                        if err[want].abs() < self.tol * 0.3 {
                            return Input::default();
                        }
                        let a = -PI / 2.0;
                        self.hand_knob = Some((want, a));
                        Input::hold(KX[want] + a.cos() * rad, KY + a.sin() * rad, true)
                    }
                }
            }
        }
    }

    fn guide_now(&self) -> Option<usize> {
        (self.phase == Phase::Tune).then_some(2)
    }
}

#[cfg(test)]
mod tests {
    use crate::games::tests::{fumble_check, plays_to_end};
    use crate::kit::Input;

    #[test]
    fn a_steady_hand_repairs_it_damaged_disabled_and_destroyed() {
        plays_to_end("biobed");
    }

    #[test]
    fn a_knob_wound_into_its_red_zone_sets_the_bed_alarming() {
        // The wheel over GAIN, turned up hard, every frame.
        fumble_check("biobed", |_r| Input { wheel: -10.0, ..Input::at(super::KX[0], super::KY) });
    }
}
