//! An impulse engine's repair (repair-minigames design 2), from `docs/mockups/repairs/impulse.js`.
//!
//! The unit from the side: the combustion chamber, its manifold and eight fuel injectors above it, the nozzle bell
//! behind. Below it the timing scope: each injector's pulse scrolls along the trace towards the firing line, in the
//! rhythm of the unit's tune. Fire (click, tap, or Space) as a pulse crosses the line and that injector is timed. A
//! round is the eight injectors timed; each level scrolls faster with a tighter line. A pulse let past the line comes
//! round again. Firing off the line is a fumble: a misfire, soot and heat. A disabled unit's first step fits the new
//! injector: drag it from the crate into its empty seat on the manifold.

use crate::kit::{Ctx, Game, Input, BAR_H, W};
use crate::pen::{c, hex, rgba, Align, Pen};

/// In the engine.
pub const PORTED: bool = true;

/// A fresh game.
pub fn new() -> Box<dyn Game> {
    Box::new(Impulse::default())
}

const N: usize = 8;
const MAN_Y: f32 = 156.0;
const CH: [f32; 4] = [130.0, 196.0, 790.0, 120.0];
const SC: [f32; 4] = [60.0, 392.0, 1160.0, 290.0];
const BASE: f32 = SC[1] + SC[3] - 70.0;
const LINE: f32 = 300.0;
const SPIKE: f32 = 120.0;
const MISSING: usize = 3;

fn inj_x(i: usize) -> f32 {
    210.0 + i as f32 * 88.0
}

#[derive(Clone, Debug, Default)]
struct Pulse {
    inj: usize,
    at: f32,
    done: bool,
}

#[derive(Clone, Debug, Default)]
struct Soot {
    x: f32,
    y: f32,
    t: f32,
}

/// The game's state.
#[derive(Default)]
pub struct Impulse {
    part: bool,
    inj: [f32; 2],
    held: bool,
    set: bool,
    pulses: Vec<Pulse>,
    speed: f32,
    tol: f32,
    time: f32,
    heat: f32,
    soot: Vec<Soot>,
    kick: f32,
    fired: usize,
    /// The hand: whether it is carrying the injector.
    hand_carry: bool,
}

impl Impulse {
    fn px(&self, p: &Pulse) -> f32 {
        LINE + (p.at - self.time) * self.speed
    }
}

impl Game for Impulse {
    fn id(&self) -> &'static str {
        "impulse"
    }

    fn knobs(&self) -> &'static [&'static str] {
        &["pulse_speed_px_s", "window_px", "beat_s"]
    }

    fn step(&mut self, cx: &mut Ctx) {
        let mut r = cx.dice();
        self.part = cx.part;
        self.inj = [1110.0, 214.0];
        self.held = false;
        self.set = false;
        self.speed = cx.knob("pulse_speed_px_s");
        self.tol = cx.knob("window_px");
        self.time = 0.0;
        self.heat = 0.0;
        self.soot.clear();
        self.kick = 0.0;
        self.fired = 0;
        self.hand_carry = false;
        // The unit's tune: a short rhythm of long and short gaps, repeated with a little swing.
        let beat = cx.knob("beat_s");
        let tune: Vec<f32> = [1.0, 1.0, 0.5, 1.5, 1.0, 0.5, 0.5, 1.5].iter().map(|b| b * beat).collect();
        let mut order: Vec<usize> = (0..N).collect();
        r.shuffle(&mut order);
        let mut at = 2.4;
        self.pulses = order
            .iter()
            .enumerate()
            .map(|(i, &k)| {
                let p = Pulse { inj: k, at, done: false };
                at += tune[(i + r.int(3)) % tune.len()];
                p
            })
            .collect();
    }

    fn update(&mut self, cx: &mut Ctx, dt: f32, input: &Input) {
        self.heat = (self.heat - dt * 0.5).max(0.0);
        self.kick = (self.kick - dt * 3.0).max(0.0);
        for s in &mut self.soot {
            s.t += dt;
        }
        self.soot.retain(|s| s.t < 1.6);
        if self.part && !self.set {
            // The part step: carry the injector to its seat.
            let (sx, sy) = (inj_x(MISSING), MAN_Y + 40.0);
            if input.pressed && (input.x - self.inj[0]).abs() < 40.0 && (input.y - self.inj[1]).abs() < 60.0 {
                self.held = true;
            }
            if self.held && input.down {
                self.inj = [input.x, input.y];
            }
            if self.held && input.released {
                self.held = false;
                if (self.inj[0] - sx).hypot(self.inj[1] - sy) < 45.0 {
                    self.set = true;
                    self.inj = [sx, sy];
                    cx.step_done();
                }
            }
            return;
        }
        if self.part {
            return;
        }
        self.time += dt;
        if self.fired < N {
            // A pulse let past the line comes round again at the back of the trace.
            let mut last = self.pulses.iter().map(|p| p.at).fold(self.time, f32::max);
            for i in 0..self.pulses.len() {
                if !self.pulses[i].done && self.px(&self.pulses[i]) < LINE - self.tol - 40.0 {
                    last += 1.2;
                    self.pulses[i].at = last;
                }
            }
            let fire = input.action_pressed || (input.pressed && input.y > BAR_H);
            if fire {
                let best = (0..self.pulses.len()).filter(|&i| !self.pulses[i].done).min_by(|&a, &b| {
                    let (da, db) = ((self.px(&self.pulses[a]) - LINE).abs(), (self.px(&self.pulses[b]) - LINE).abs());
                    da.total_cmp(&db)
                });
                let bd = best.map_or(f32::INFINITY, |i| (self.px(&self.pulses[i]) - LINE).abs());
                match best {
                    Some(i) if bd <= self.tol => {
                        self.pulses[i].done = true;
                        self.fired += 1;
                        self.kick = 1.0;
                        if self.fired == N {
                            cx.step_done();
                        }
                    }
                    _ => {
                        self.heat = 1.0;
                        let k = best.map_or(0, |i| self.pulses[i].inj);
                        self.soot.push(Soot { x: inj_x(k), y: CH[1] + 20.0, t: 0.0 });
                        cx.fumble("Misfire: soot and heat");
                    }
                }
            }
        }
    }

    fn draw(&self, g: &Pen, _cx: &Ctx, t: f32, _input: &Input) {
        g.rect(0.0, BAR_H, W, 720.0, hex(0x070b12));
        let live = !self.part;
        let next = if live {
            self.pulses
                .iter()
                .filter(|p| !p.done && self.px(p) > LINE - self.tol)
                .min_by(|a, b| self.px(a).total_cmp(&self.px(b)))
                .map(|p| p.inj)
        } else {
            None
        };
        let fired_set: Vec<usize> =
            if live { self.pulses.iter().filter(|p| p.done).map(|p| p.inj).collect() } else { Vec::new() };
        let [chx, chy, chw, chh] = CH;

        // The nozzle bell and its plume: brighter with every injector timed.
        let thrust = if live { self.fired as f32 / N as f32 } else { 0.0 };
        let plume = rgba(79, 195, 247, 0.15 + 0.6 * thrust + 0.25 * self.kick);
        let clear = rgba(79, 195, 247, 0.0);
        g.quad(
            [
                [1040.0, 170.0],
                [1260.0, 196.0 + 10.0 * (t * 9.0).sin()],
                [1260.0, 316.0 - 10.0 * (t * 7.0).sin()],
                [1040.0, 342.0],
            ],
            [plume, clear, clear, plume],
        );
        let bell = [[chx + chw, chy + 14.0], [1040.0, 160.0], [1040.0, 352.0], [chx + chw, chy + chh - 14.0]];
        g.poly(&bell, hex(0x1b2433));
        g.path(&bell, true, 3.0, hex(0x3a4656));
        for k in 1..=3 {
            let x = chx + chw + k as f32 * 30.0;
            let f = (x - chx - chw) / (1040.0 - chx - chw);
            g.line(x, chy + 14.0 - f * 50.0, x, chy + chh - 14.0 + f * 50.0, 2.0, hex(0x2a3444));
        }
        g.rect(1034.0, 162.0, 6.0, 188.0, rgba(160, 220, 255, 0.3 + 0.6 * thrust));

        // The chamber: a ribbed steel cylinder, hot when it misfires.
        g.panel(chx, chy, chw, chh, 26.0, hex(0x1b2433), hex(0x3a4656));
        let mut x = chx + 40.0;
        while x < chx + chw - 20.0 {
            g.rect(x, chy + 4.0, 10.0, chh - 8.0, hex(0x222c3c));
            x += 44.0;
        }
        if self.heat > 0.0 {
            g.round(chx, chy, chw, chh, 26.0, Some(rgba(255, 71, 87, 0.35 * self.heat)), None);
        }
        let mid = rgba(79, 195, 247, 0.1 + 0.45 * thrust);
        g.gradient(chx + 30.0, chy + 40.0, chw - 40.0, 20.0, rgba(79, 195, 247, 0.0), mid, true);
        g.gradient(chx + 30.0, chy + 60.0, chw - 40.0, 20.0, mid, rgba(79, 195, 247, 0.0), true);
        g.panel(chx - 50.0, chy + 18.0, 70.0, chh - 36.0, 14.0, hex(0x141c28), hex(0x3a4656));
        g.text("UNIT 1", chx + 30.0, chy + chh + 26.0, 20.0, c::DIM, Align::Left);

        // The manifold and the eight injectors, each with its lamp: hollow waiting, amber ring next, green timed.
        g.round(170.0, MAN_Y - 12.0, 650.0, 24.0, 12.0, Some(hex(0x2b3646)), None);
        g.round(120.0, MAN_Y - 8.0, 60.0, 16.0, 8.0, Some(hex(0x2b3646)), None);
        for i in 0..N {
            let x = inj_x(i);
            if self.part && i == MISSING && !self.set {
                let r = crate::pen::round_rect_points(x - 15.0, MAN_Y + 8.0, 30.0, chy - MAN_Y - 6.0, 6.0);
                let mut closed = r.clone();
                closed.push(r[0]);
                g.dashed(&closed, 2.0, c::AMBER, 6.0, 5.0);
                continue;
            }
            g.round(x - 13.0, MAN_Y + 8.0, 26.0, chy - MAN_Y - 4.0, 6.0, Some(hex(0x566273)), None);
            g.rect(x - 17.0, MAN_Y + 18.0, 34.0, 6.0, hex(0x3a4656));
            let ly = MAN_Y - 44.0;
            if fired_set.contains(&i) {
                g.disc(x, ly, 14.0, c::OK);
                g.tick(x, ly + 1.0, 16.0, hex(0x04140a));
            } else if next == Some(i) {
                g.disc(x, ly, 14.0, hex(0x1d2738));
                g.ring(x, ly, 14.0, c::AMBER, 4.0);
            } else {
                g.disc(x, ly, 14.0, hex(0x121a25));
                g.ring(x, ly, 14.0, hex(0x3b4a5e), 2.0);
            }
        }
        for s in &self.soot {
            let k = s.t / 1.6;
            for j in 0..5 {
                let (jx, jy) = ((j as f32 - 2.0) * 14.0 * (1.0 + k), 30.0 * k + (j % 2) as f32 * 10.0);
                g.disc(s.x + jx, s.y - jy, 10.0 + 22.0 * k, rgba(40, 40, 44, 0.75 * (1.0 - k)));
            }
        }

        // The timing scope: a grid, the trace, the firing line, the pulses.
        let [sx, sy, sw, sh] = SC;
        g.panel(sx, sy, sw, sh, 18.0, hex(0x060c10), hex(0x1f3a33));
        let s = g.clip(sx, sy, sw, sh);
        let mut gx = sx;
        while gx < sx + sw {
            s.line(gx, sy, gx, sy + sh, 1.0, rgba(61, 220, 132, 0.08));
            gx += 40.0;
        }
        let mut gy = sy + 10.0;
        while gy < sy + sh {
            s.line(sx, gy, sx + sw, gy, 1.0, rgba(61, 220, 132, 0.08));
            gy += 40.0;
        }
        s.rect(LINE - self.tol, sy, self.tol * 2.0, sh, rgba(242, 160, 70, 0.14));
        s.rect(LINE - 2.0, sy, 4.0, sh, c::AMBER);
        s.poly(&[[LINE - 14.0, sy], [LINE + 14.0, sy], [LINE, sy + 18.0]], c::AMBER);
        s.poly(&[[LINE - 14.0, sy + sh], [LINE + 14.0, sy + sh], [LINE, sy + sh - 18.0]], c::AMBER);
        // The trace: flat between pulses, a sharp spike at each.
        let mut shown: Vec<(&Pulse, f32)> = if live {
            self.pulses.iter().map(|p| (p, self.px(p))).filter(|(_, x)| *x > sx - 40.0 && *x < sx + sw + 40.0).collect()
        } else {
            Vec::new()
        };
        shown.sort_by(|a, b| a.1.total_cmp(&b.1));
        let mut trace = vec![[sx, BASE]];
        for (_, x) in &shown {
            trace.extend_from_slice(&[
                [x - 22.0, BASE],
                [x - 8.0, BASE - SPIKE],
                [x + 8.0, BASE - SPIKE],
                [x + 22.0, BASE],
            ]);
        }
        trace.push([sx + sw, BASE]);
        for (p, x) in &shown {
            let near = !p.done && (x - LINE).abs() <= self.tol;
            let fill = if p.done {
                rgba(61, 220, 132, 0.35)
            } else if near {
                rgba(242, 160, 70, 0.45)
            } else {
                rgba(61, 220, 132, 0.12)
            };
            s.poly(&[[x - 22.0, BASE], [x - 8.0, BASE - SPIKE], [x + 8.0, BASE - SPIKE], [x + 22.0, BASE]], fill);
        }
        s.path(&trace, false, 3.0, if live { rgba(61, 220, 132, 0.75) } else { rgba(61, 220, 132, 0.2) });
        for (p, x) in &shown {
            let near = !p.done && (x - LINE).abs() <= self.tol;
            if p.done {
                s.disc(*x, BASE - SPIKE - 26.0, 14.0, c::OK);
                s.tick(*x, BASE - SPIKE - 25.0, 16.0, hex(0x04140a));
            } else {
                s.text(
                    &(p.inj + 1).to_string(),
                    *x,
                    BASE - SPIKE - 24.0,
                    22.0,
                    if near { c::AMBER } else { c::FG },
                    Align::Center,
                );
            }
        }

        // The part: the new injector in its crate, or in the hand.
        if self.part && !self.set {
            g.panel(1040.0, 120.0, 140.0, 190.0, 14.0, hex(0x141b27), c::LINE);
            let [ix, iy] = self.inj;
            g.round(ix - 13.0, iy - 40.0, 26.0, 80.0, 6.0, Some(hex(0x8a96a6)), None);
            g.rect(ix - 17.0, iy - 30.0, 34.0, 6.0, c::COPPER);
            g.poly(&[[ix - 8.0, iy + 40.0], [ix + 8.0, iy + 40.0], [ix, iy + 54.0]], hex(0xb8c2d0));
        }
    }

    fn hand(&mut self, _cx: &Ctx, _t: f32) -> Input {
        if self.part && !self.set {
            // Pick the injector up, carry it to its seat in a few frames, let go there.
            let (sx, sy) = (inj_x(MISSING), MAN_Y + 40.0);
            if !self.held && !self.hand_carry {
                self.hand_carry = true;
                return Input::hold(self.inj[0], self.inj[1], true);
            }
            let d = (sx - self.inj[0]).hypot(sy - self.inj[1]);
            if d > 2.0 {
                let k = (30.0 / d).min(1.0);
                return Input::hold(self.inj[0] + (sx - self.inj[0]) * k, self.inj[1] + (sy - self.inj[1]) * k, false);
            }
            self.hand_carry = false;
            return Input::release(sx, sy);
        }
        // Fire when the nearest pulse is well inside the window.
        let near =
            self.pulses.iter().filter(|p| !p.done).map(|p| (self.px(p) - LINE).abs()).fold(f32::INFINITY, f32::min);
        Input { action_pressed: near <= self.tol * 0.4, ..Input::default() }
    }

    fn guide_now(&self) -> Option<usize> {
        Some(if self.fired == 0 { 0 } else { 2 })
    }
}

#[cfg(test)]
mod tests {
    use crate::games::tests::{fumble_check, plays_to_end};

    #[test]
    fn a_steady_hand_repairs_it_damaged_disabled_and_destroyed() {
        plays_to_end("impulse");
    }

    #[test]
    fn firing_off_the_line_is_a_misfire() {
        // A press with no pulse near the line, the moment the round opens.
        fumble_check("impulse", |_r| crate::kit::Input { action_pressed: true, ..crate::kit::Input::default() });
    }
}
