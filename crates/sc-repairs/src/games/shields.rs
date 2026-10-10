//! The shield generator's repair (repair-minigames design 2), from `docs/mockups/repairs/shields.js`.
//!
//! The generator from above: the core, six emitter segments on a ring, one for each shield face, and the field's six
//! arcs outside them. A detuned segment hums out of step with the master wave. Its wave and the master's are laid over
//! each other on the scope; turn the phase dial and the gain dial (drag them, or the arrows: left and right for phase,
//! up and down for gain, or the wheel for gain) until the two lie together, and hold them there while the lock fills.
//! A round is one pair of segments matched; each level the segments wander faster and the match is tighter. Gain
//! driven into the hatched red zone is a fumble: the emitter crackles and its face drains. A disabled generator's
//! first step fits a new emitter: drag it from the crate into the empty seat on the ring.

use egui::Key;

use crate::games::breakers::{wrap, Hand, PART_STEP};
use crate::kit::{Ctx, Game, Input, BAR_H, W};
use crate::pen::{c, hex, rgba, Align, Pen};

/// In the engine.
pub const PORTED: bool = true;

/// A fresh game.
pub fn new() -> Box<dyn Game> {
    Box::new(Shields::default())
}

const CX: f32 = 330.0;
const CY: f32 = 405.0;
const POD_R: f32 = 168.0;
const FIELD_R: f32 = 238.0;
const FACES: [&str; 6] = ["FWD", "STBD", "DOR", "AFT", "VEN", "PORT"];
/// The scope: x, y, w, h.
const SC: [f32; 4] = [660.0, 100.0, 580.0, 310.0];
/// The phase dial and the gain dial: x, y, r.
const PH: [f32; 3] = [820.0, 575.0, 82.0];
const GA: [f32; 3] = [1090.0, 575.0, 82.0];
/// Gain: the dial's top, the over-drive line (master = 1).
const G_MAX: f32 = 2.0;
const G_RED: f32 = 1.5;
/// The gain dial's sweep, radians from +x.
const G_A0: f32 = std::f32::consts::PI * 0.75;
const G_SWEEP: f32 = std::f32::consts::PI * 1.5;
/// Seconds matched to lock a segment.
const HOLD: f32 = 1.2;

fn ang(i: usize) -> f32 {
    -std::f32::consts::FRAC_PI_2 + i as f32 * std::f32::consts::PI / 3.0
}

fn pod_at(i: usize) -> [f32; 2] {
    [CX + ang(i).cos() * POD_R, CY + ang(i).sin() * POD_R]
}

#[derive(Clone, Copy, Debug, Default)]
struct Seg {
    ph: f32,
    amp: f32,
    drift: f32,
    hold: f32,
    locked: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Dial {
    Phase,
    Gain,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Look {
    Empty,
    Ok,
    Pair,
    Bad,
}

/// The game's state.
#[derive(Default)]
pub struct Shields {
    part: bool,
    pod: [f32; 2],
    pod_held: bool,
    pod_set: bool,
    segs: [Seg; 6],
    pair: [usize; 2],
    active: usize,
    ok: [bool; 6],
    tol_p: f32,
    tol_a: f32,
    grab: Option<Dial>,
    last_a: f32,
    crackle: f32,
    time: f32,
    hand: Hand,
}

impl Shields {
    fn matched(&self, s: &Seg) -> bool {
        wrap(s.ph).abs() < self.tol_p && (s.amp - 1.0).abs() < self.tol_a
    }

    fn look(&self, i: usize) -> Look {
        if self.part {
            if i == 0 && !self.pod_set {
                Look::Empty
            } else {
                Look::Ok
            }
        } else if self.segs[i].locked || self.ok[i] {
            Look::Ok
        } else if self.pair.contains(&i) {
            Look::Pair
        } else {
            Look::Bad
        }
    }

    /// One dial: its ring, ticks, knurled face and pointer at angle `a`.
    fn dial(g: &Pen, d: [f32; 3], a: f32, on: bool) {
        use std::f32::consts::TAU;
        let [x, y, r] = d;
        g.disc(x, y, r + 14.0, hex(0x0c121a));
        g.ring(x, y, r + 14.0, if on { c::AMBER } else { c::LINE }, 3.0);
        for k in 0..24 {
            let b = k as f32 / 24.0 * TAU;
            g.line(
                x + b.cos() * (r + 4.0),
                y + b.sin() * (r + 4.0),
                x + b.cos() * (r + 10.0),
                y + b.sin() * (r + 10.0),
                2.0,
                hex(0x3b4a5e),
            );
        }
        g.disc(x, y, r - 10.0, hex(0x1b2433));
        g.ring(x, y, r - 10.0, hex(0x566273), 3.0);
        for k in 0..12 {
            let b = k as f32 / 12.0 * TAU + a;
            g.disc(x + b.cos() * (r - 18.0), y + b.sin() * (r - 18.0), 4.0, hex(0x2b3646));
        }
        g.path_round(&[[x, y], [x + a.cos() * (r - 16.0), y + a.sin() * (r - 16.0)]], 6.0, c::FG);
        g.disc(x, y, 12.0, hex(0x3a4656));
    }
}

impl Game for Shields {
    fn id(&self) -> &'static str {
        "shields"
    }

    fn knobs(&self) -> &'static [&'static str] {
        &["phase_tol_rad", "gain_tol_frac", "wander_rad_s"]
    }

    fn step(&mut self, cx: &mut Ctx) {
        let mut r = cx.dice();
        self.part = cx.part;
        self.pod = [575.0, 648.0];
        self.pod_held = false;
        self.pod_set = false;
        self.tol_p = cx.knob("phase_tol_rad");
        self.tol_a = cx.knob("gain_tol_frac");
        let wander = cx.knob("wander_rad_s");
        // Which pair this round tunes is the round's place in the job, not its difficulty.
        let k = if self.part { 0 } else { cx.round as usize };
        self.pair = [(2 * k) % 6, (2 * k + 1) % 6];
        // Faces tuned earlier in the job hold; this round's pair and the later ones are out.
        for i in 0..6 {
            self.ok[i] = if k >= 3 { !self.pair.contains(&i) } else { i < 2 * k };
        }
        for s in &mut self.segs {
            let ph = if r.f() < 0.5 { -1.0 } else { 1.0 } * (1.0 + r.f() * 1.8);
            let mut amp = 0.35 + r.f() * 0.95;
            if (amp - 1.0).abs() < 0.3 {
                amp = if r.f() < 0.5 { 0.5 } else { 1.35 };
            }
            let drift = if r.f() < 0.5 { -1.0 } else { 1.0 } * wander;
            *s = Seg { ph, amp, drift, hold: 0.0, locked: false };
        }
        self.active = self.pair[0];
        self.grab = None;
        self.crackle = 0.0;
        self.time = 0.0;
        self.hand = Hand::default();
    }

    fn update(&mut self, cx: &mut Ctx, dt: f32, input: &Input) {
        self.crackle = (self.crackle - dt * 1.5).max(0.0);
        if self.part && !self.pod_set {
            // The part step: carry the emitter to its seat.
            let [sx, sy] = pod_at(0);
            if input.pressed && input.dist(self.pod[0], self.pod[1]) < 50.0 {
                self.pod_held = true;
            }
            if self.pod_held && input.down {
                self.pod = [input.x, input.y];
            }
            if self.pod_held && input.released {
                self.pod_held = false;
                if (self.pod[0] - sx).hypot(self.pod[1] - sy) < 50.0 {
                    self.pod_set = true;
                    self.pod = [sx, sy];
                    cx.step_done();
                }
            }
            return;
        }
        if self.part {
            return;
        }
        self.time += dt;
        if self.segs[self.active].locked {
            return;
        }
        // Clicking the other segment of the pair takes it instead.
        if input.pressed {
            for i in self.pair {
                let [x, y] = pod_at(i);
                if !self.segs[i].locked && input.dist(x, y) < 50.0 {
                    self.active = i;
                    return;
                }
            }
            if input.dist(PH[0], PH[1]) < PH[2] + 20.0 {
                self.grab = Some(Dial::Phase);
                self.last_a = (input.y - PH[1]).atan2(input.x - PH[0]);
            } else if input.dist(GA[0], GA[1]) < GA[2] + 20.0 {
                self.grab = Some(Dial::Gain);
                self.last_a = (input.y - GA[1]).atan2(input.x - GA[0]);
            }
        }
        let time = self.time;
        let s = &mut self.segs[self.active];
        if let (Some(d), true) = (self.grab, input.down) {
            // The dials turn with the pointer's angle round them, from where they were grabbed.
            let at = if d == Dial::Phase { PH } else { GA };
            let a = (input.y - at[1]).atan2(input.x - at[0]);
            let dd = wrap(a - self.last_a);
            self.last_a = a;
            if d == Dial::Phase {
                s.ph += dd;
            } else {
                s.amp += dd / G_SWEEP * G_MAX;
            }
        }
        if !input.down {
            self.grab = None;
        }
        s.ph += input.stick[0] * 1.4 * dt;
        s.amp += -input.stick[1] * 0.5 * dt - input.wheel * 0.05;
        // The segment wanders on its own.
        s.ph += s.drift * dt + 0.05 * (time * 1.7).sin() * dt;
        s.amp = (s.amp + 0.04 * (time * 1.3).sin() * dt).clamp(0.0, G_MAX);
        if s.amp > G_RED {
            self.crackle = 1.0;
            s.amp = 0.4;
            s.hold = 0.0;
            self.grab = None;
            cx.fumble("Emitter crackle: the face drains");
            return;
        }
        let seg = *s;
        let m = self.matched(&seg);
        let s = &mut self.segs[self.active];
        s.hold = if m { s.hold + dt } else { (s.hold - dt * 2.0).max(0.0) };
        if s.hold >= HOLD {
            s.locked = true;
            s.ph = 0.0;
            s.amp = 1.0;
            match self.pair.iter().copied().find(|&i| !self.segs[i].locked) {
                Some(rest) => self.active = rest,
                None => cx.step_done(),
            }
        }
    }

    fn draw(&self, g: &Pen, _cx: &Ctx, t: f32, _input: &Input) {
        use std::f32::consts::{FRAC_PI_2, PI, TAU};
        g.rect(0.0, BAR_H, W, 720.0, hex(0x070b12));
        let live = !self.part;

        // The field: six arcs, solid when the face holds, broken when it is out.
        for i in 0..6 {
            let (st, a) = (self.look(i), ang(i));
            let pts = Pen::arc_points(CX, CY, FIELD_R, a - 0.46, a + 0.46);
            match st {
                Look::Ok => g.path(&pts, false, 8.0, rgba(79, 195, 247, 0.75)),
                Look::Pair => g.dashed(&pts, 8.0, rgba(242, 160, 70, 0.6), 12.0, 10.0),
                _ => g.dashed(&pts, 8.0, rgba(255, 71, 87, 0.55), 12.0, 10.0),
            }
        }
        // The ring, the busbars and the core.
        g.ring(CX, CY, POD_R, hex(0x1b2433), 30.0);
        g.ring(CX, CY, POD_R, hex(0x2b3646), 2.0);
        for i in 0..6 {
            let a = ang(i);
            let (x0, y0, x1, y1) = (
                CX + a.cos() * 56.0,
                CY + a.sin() * 56.0,
                CX + a.cos() * (POD_R - 40.0),
                CY + a.sin() * (POD_R - 40.0),
            );
            g.line(x0, y0, x1, y1, 10.0, hex(0x3a4656));
            g.line(x0, y0, x1, y1, 3.0, c::COPPER);
        }
        g.disc(CX, CY, 56.0, hex(0x141c28));
        g.ring(CX, CY, 56.0, hex(0x3a4656), 4.0);
        let locked_n = if live { (0..6).filter(|&i| self.look(i) == Look::Ok).count() } else { 5 };
        g.radial(CX, CY, 44.0, rgba(160, 220, 255, 0.5 + 0.08 * locked_n as f32), rgba(79, 195, 247, 0.0));

        // The emitter pods: a block, a dish, a lamp whose shape says its state.
        for (i, name) in FACES.iter().enumerate() {
            let (st, a) = (self.look(i), ang(i));
            let [x, y] = pod_at(i);
            let (tx, ty) = (CX + a.cos() * (POD_R + 104.0), CY + a.sin() * (POD_R + 104.0));
            if st == Look::Empty {
                let r = crate::pen::round_rect_points(x - 40.0, y - 34.0, 80.0, 68.0, 12.0);
                g.dashed(&crate::games::breakers::closed(r), 2.0, c::AMBER, 7.0, 6.0);
                g.text(name, tx, ty, 18.0, c::DANGER, Align::Center);
                continue;
            }
            let is_active = live && i == self.active && !self.segs[i].locked;
            g.panel(
                x - 40.0,
                y - 34.0,
                80.0,
                68.0,
                12.0,
                hex(0x1b2433),
                if is_active { c::AMBER } else { hex(0x3a4656) },
            );
            g.disc(x, y, 22.0, hex(0x0e151f));
            g.ring(x, y, 22.0, hex(0x566273), 3.0);
            g.disc(x, y, 8.0, if st == Look::Ok { c::ACCENT } else { hex(0x33404f) });
            let (lx, ly) = (x + 28.0, y - 22.0);
            match st {
                Look::Ok => g.disc(lx, ly, 7.0, c::OK),
                Look::Pair => g.poly(&[[lx, ly - 8.0], [lx + 8.0, ly + 6.0], [lx - 8.0, ly + 6.0]], c::AMBER),
                _ => {
                    let q = g.translate(lx, ly).rotate(PI / 4.0);
                    q.rect(-7.0, -2.0, 14.0, 4.0, c::DANGER);
                    q.rect(-2.0, -7.0, 4.0, 14.0, c::DANGER);
                }
            }
            if is_active {
                let h = self.segs[i].hold / HOLD;
                g.ring(x, y, 52.0, hex(0x1d2636), 6.0);
                if h > 0.0 {
                    g.arc(x, y, 52.0, -FRAC_PI_2, -FRAC_PI_2 + TAU * h, 6.0, c::OK);
                }
            }
            if is_active && self.crackle > 0.0 {
                for k in 0..6 {
                    let b = t * 40.0 + k as f32;
                    let mut pts = vec![[x, y]];
                    for j in 1..=4 {
                        let jf = j as f32;
                        pts.push([
                            x + b.cos() * jf * 16.0 + (b * 3.0 + jf).sin() * 8.0,
                            y + b.sin() * jf * 16.0 + (b * 2.0 + jf).cos() * 8.0,
                        ]);
                    }
                    g.path(&pts, false, 2.0, rgba(255, 255, 255, self.crackle));
                }
            }
            let col = match st {
                Look::Ok => c::DIM,
                Look::Pair => c::AMBER,
                _ => c::DANGER,
            };
            g.text(name, tx, ty, 18.0, col, Align::Center);
        }

        // The scope: the master wave as a broad band, the segment's wave as a line over it.
        let [sx, sy, sw, sh] = SC;
        g.panel(sx, sy, sw, sh, 18.0, hex(0x060c10), hex(0x1f3a33));
        let q = g.clip(sx, sy, sw, sh);
        let mut x = sx;
        while x < sx + sw {
            q.line(x, sy, x, sy + sh, 1.0, rgba(61, 220, 132, 0.08));
            x += 40.0;
        }
        let mut y = sy + 15.0;
        while y < sy + sh {
            q.line(sx, y, sx + sw, y, 1.0, rgba(61, 220, 132, 0.08));
            y += 40.0;
        }
        let (mid, amp_px) = (sy + sh / 2.0, 72.0);
        q.rect(sx, sy, sw, mid - amp_px * G_RED - sy, rgba(255, 71, 87, 0.14));
        q.rect(sx, mid + amp_px * G_RED, sw, sy + sh - mid - amp_px * G_RED, rgba(255, 71, 87, 0.14));
        let wave = |amp: f32, ph: f32, w: f32| -> Vec<[f32; 2]> {
            (0..=(sw as i32 / 4))
                .map(|k| {
                    let xx = k as f32 * 4.0;
                    [sx + xx, mid - amp_px * amp * ((xx / sw) * TAU * 2.0 - t * w + ph).sin()]
                })
                .collect()
        };
        q.path(&wave(1.0, 0.0, 3.0), false, 16.0, rgba(79, 195, 247, 0.35));
        if live {
            let s = self.segs[self.active];
            q.path(&wave(s.amp, s.ph, 3.0), false, 4.0, if s.locked || self.matched(&s) { c::OK } else { c::AMBER });
        }

        // The dials: phase turns freely; gain sweeps to a hatched red zone at its top.
        let s = if live { self.segs[self.active] } else { Seg { amp: 1.0, ..Seg::default() } };
        let red_a = G_A0 + G_RED / G_MAX * G_SWEEP;
        let end_a = G_A0 + G_SWEEP;
        g.hatch_arc(GA[0], GA[1], GA[2] + 16.0, GA[2] + 28.0, red_a, end_a, c::DANGER);
        g.arc(GA[0], GA[1], GA[2] + 22.0, G_A0, red_a, 12.0, hex(0x2b3646));
        Self::dial(g, PH, -FRAC_PI_2 + s.ph, self.grab == Some(Dial::Phase));
        Self::dial(g, GA, G_A0 + s.amp / G_MAX * G_SWEEP, self.grab == Some(Dial::Gain));
        g.text("PHASE", PH[0], PH[1] + PH[2] + 42.0, 20.0, c::DIM, Align::Center);
        g.text("GAIN", GA[0], GA[1] + GA[2] + 42.0, 20.0, c::DIM, Align::Center);

        // The part: the new emitter in its crate, or in the hand.
        if self.part && !self.pod_set {
            let [px, py] = self.pod;
            g.panel(505.0, 595.0, 140.0, 106.0, 14.0, hex(0x141b27), c::LINE);
            g.panel(px - 40.0, py - 34.0, 80.0, 68.0, 12.0, hex(0x2b3646), hex(0x7d8796));
            g.disc(px, py, 22.0, hex(0x0e151f));
            g.ring(px, py, 22.0, hex(0xb8c2d0), 3.0);
            g.disc(px, py, 8.0, c::ACCENT);
        }
    }

    fn hand(&mut self, _cx: &Ctx, _t: f32) -> Input {
        if self.hand.resting() {
            return Input::default();
        }
        if self.part && !self.pod_set {
            return self.hand.carry(1, self.pod, pod_at(0), PART_STEP);
        }
        if self.part {
            return self.hand.idle();
        }
        // The arrows: steer the phase to zero and the gain to one, and keep them there against the wander.
        let s = self.segs[self.active];
        let mut i = Input::default();
        let pe = wrap(s.ph);
        if pe > 0.015 {
            i = i.key(Key::ArrowLeft, false);
        } else if pe < -0.015 {
            i = i.key(Key::ArrowRight, false);
        }
        let ae = s.amp - 1.0;
        if ae > 0.008 {
            i = i.key(Key::ArrowDown, false);
        } else if ae < -0.008 {
            i = i.key(Key::ArrowUp, false);
        }
        i
    }
}

#[cfg(test)]
mod tests {
    use crate::games::tests::{fumble_check, plays_to_end};
    use crate::kit::Input;
    use egui::Key;

    #[test]
    fn a_steady_hand_repairs_it_damaged_disabled_and_destroyed() {
        plays_to_end("shields");
    }

    #[test]
    fn gain_driven_into_the_red_crackles() {
        fumble_check("shields", |_r| Input::default().key(Key::ArrowUp, false));
    }
}
