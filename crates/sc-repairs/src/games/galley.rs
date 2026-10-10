//! The galley synthesizer's repair (repair-minigames design 2), from `docs/mockups/repairs/galley.js`.
//!
//! The recipe card shows four nutrient columns and the band each must reach; the same bands are bracketed on the
//! synthesizer's four glass columns. Hold a column's valve to fill it (the pointer, keys 1 to 4, or the arrows and
//! Space); the feed runs on briefly after the valve is let go, so let go early. All four in their bands lights PURGE:
//! press it (or Space) and the round is done. Each level fills faster, runs on longer and narrows the bands. Past a
//! band's top is a fumble: paste everywhere, that column drains. A disabled synthesizer's first step fits the new
//! dispenser nozzle: drag it from the crate onto the dispenser.

use egui::{Color32, Key};

use crate::kit::{Ctx, Game, Input, BAR_H, H, W};
use crate::pen::{alpha, c, hex, rgba, round_rect_points, Align, Pen};

/// In the engine.
pub const PORTED: bool = true;

/// A fresh game.
pub fn new() -> Box<dyn Game> {
    Box::new(Galley::default())
}

const SPILL: &str = "Paste everywhere: the column drains";
const MACH: [f32; 4] = [320.0, 100.0, 620.0, 590.0];
const COLX: [f32; 4] = [440.0, 580.0, 720.0, 860.0];
const CW: f32 = 92.0;
const TOP: f32 = 140.0;
const BOT: f32 = 500.0;
const CH: f32 = BOT - TOP;
const VY: f32 = 572.0;
const VR: f32 = 42.0;
const MANI: f32 = 650.0;
const NAMES: [&str; 4] = ["PRO", "CARB", "FAT", "VIT"];
const COLS: [Color32; 4] = [c::COPPER, hex(0xe6d6a8), c::WARN, c::LILAC];
const DIGITS: [Key; 4] = [Key::Num1, Key::Num2, Key::Num3, Key::Num4];
const CARD: [f32; 4] = [40.0, 112.0, 240.0, 290.0];
const DISP: [f32; 4] = [40.0, 425.0, 240.0, 265.0];
const NOZ: [f32; 2] = [160.0, 470.0];
const PURGE: [f32; 3] = [1160.0, 330.0, 72.0];
const CRATE: [f32; 4] = [1090.0, 560.0, 140.0, 115.0];
/// A spilled column drains at this, column heights a second.
const DRAIN: f32 = 0.7;
/// A feed below this, column heights a second, is still.
const STILL: f32 = 0.004;
/// The feed's rise to its rate once a valve opens, seconds.
const OPEN_S: f32 = 0.08;
/// The floor the paste lands on.
const FLOOR: f32 = 640.0;

fn level(l: f32) -> f32 {
    BOT - l * CH
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Phase {
    Part,
    Fitted,
    #[default]
    Fill,
    Purge,
}

#[derive(Clone, Copy, Debug, Default)]
struct Col {
    /// The band's centre, of the column's height.
    c: f32,
    /// The band's half width.
    w: f32,
    /// The paste's level.
    l: f32,
    /// The feed running in, heights a second.
    f: f32,
    open: bool,
    drain: bool,
}

impl Col {
    fn in_band(&self) -> bool {
        (self.l - self.c).abs() <= self.w
    }
    fn settled(&self) -> bool {
        self.in_band() && self.f < STILL && !self.drain
    }
}

#[derive(Clone, Debug)]
struct Splat {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    c: Color32,
    life: f32,
    land: bool,
}

#[derive(Clone, Copy, Debug, Default)]
struct Noz {
    x: f32,
    y: f32,
    held: bool,
    pad: bool,
    set: bool,
}

/// The game's state.
#[derive(Default)]
pub struct Galley {
    phase: Phase,
    sel: usize,
    keys: bool,
    purge_t: f32,
    splats: Vec<Splat>,
    noz: Noz,
    cols: [Col; 4],
    rate: f32,
    tau: f32,
    /// The hand: whether it is carrying the nozzle.
    hand_carry: bool,
}

impl Galley {
    fn settled(&self) -> bool {
        self.cols.iter().all(Col::settled)
    }

    fn part_step(&mut self, cx: &mut Ctx, dt: f32, input: &Input) {
        let p = &mut self.noz;
        let [sx, sy] = input.stick;
        if sx != 0.0 || sy != 0.0 {
            p.x += sx * 480.0 * dt;
            p.y += sy * 480.0 * dt;
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
            if (p.x - NOZ[0]).hypot(p.y - NOZ[1]) < 40.0 {
                p.set = true;
                p.x = NOZ[0];
                p.y = NOZ[1];
                self.phase = Phase::Fitted;
                cx.step_done();
            }
        }
    }

    fn fill(&mut self, cx: &mut Ctx, dt: f32, input: &Input) {
        if input.hit(Key::ArrowLeft) || input.hit(Key::A) {
            self.sel = (self.sel + 3) % 4;
            self.keys = true;
        }
        if input.hit(Key::ArrowRight) || input.hit(Key::D) {
            self.sel = (self.sel + 1) % 4;
            self.keys = true;
        }
        let ready = self.settled();
        let purge = (input.pressed && (input.x - PURGE[0]).hypot(input.y - PURGE[1]) < PURGE[2])
            || (ready && input.action_pressed);
        if purge && ready {
            self.phase = Phase::Purge;
            self.purge_t = 0.0;
            cx.step_done();
            return;
        }
        for i in 0..4 {
            let held = (input.down && (input.x - COLX[i]).hypot(input.y - VY) < VR + 10.0)
                || input.held(DIGITS[i])
                || (input.action && !ready && self.sel == i);
            if input.action {
                self.keys = true;
            }
            let col = &mut self.cols[i];
            col.open = held && !col.drain;
            if col.drain {
                col.l = (col.l - DRAIN * dt).max(0.0);
                col.f = 0.0;
                if col.l <= 0.0 {
                    col.drain = false;
                }
                continue;
            }
            // The feed: up to the rate while held, running on and dying away after.
            col.f = if col.open {
                col.f + (self.rate - col.f) * (dt / OPEN_S).min(1.0)
            } else {
                col.f * (-dt / self.tau).exp()
            };
            col.l += col.f * dt;
            if col.l > col.c + col.w {
                col.drain = true;
                col.f = 0.0;
                let y = level(col.l);
                for k in 0..16 {
                    let kf = k as f32;
                    self.splats.push(Splat {
                        x: COLX[i] + (kf - 8.0) * 4.0,
                        y,
                        vx: ((kf * 7.1).sin() * 0.5 + (kf - 8.0) / 8.0) * 260.0,
                        vy: -150.0 - 160.0 * (kf * 3.3).cos().abs(),
                        c: COLS[i],
                        life: 2.2,
                        land: false,
                    });
                }
                cx.fumble(SPILL);
                return;
            }
        }
    }
}

/// A plain pipe: a dark casing round a metal body, round ends.
fn pipe(g: &Pen, pts: &[[f32; 2]], w: f32) {
    g.path_round(pts, w + 6.0, hex(0x232c3b));
    g.path_round(pts, w, hex(0x3a4658));
}

/// The dispenser nozzle.
fn nozzle(g: &Pen, x: f32, y: f32) {
    g.poly(&[[x - 26.0, y - 22.0], [x + 26.0, y - 22.0], [x + 12.0, y + 18.0], [x - 12.0, y + 18.0]], c::STEEL);
    g.rect(x - 30.0, y - 30.0, 60.0, 10.0, hex(0x4a5566));
    g.rect(x - 6.0, y + 12.0, 12.0, 8.0, hex(0x1a2230));
}

/// A band's bracket beside a column, its arms pointing `side` (+1 right, -1 left).
fn bracket(g: &Pen, x: f32, y0: f32, y1: f32, side: f32, col: Color32) {
    g.path(&[[x + side * 14.0, y0], [x, y0], [x, y1], [x + side * 14.0, y1]], false, 5.0, col);
}

/// The points of an ellipse round (x, y) from `a0` to `a1` radians (the pen has no ellipse).
fn ellipse(x: f32, y: f32, rx: f32, ry: f32, a0: f32, a1: f32) -> Vec<[f32; 2]> {
    let n = 32;
    (0..=n)
        .map(|i| {
            let a = a0 + (a1 - a0) * i as f32 / n as f32;
            [x + a.cos() * rx, y + a.sin() * ry]
        })
        .collect()
}

/// A convex polygon cut at the line y = `edge`, keeping the part below it on screen (`under`) or above it.
fn half(pts: &[[f32; 2]], edge: f32, under: bool) -> Vec<[f32; 2]> {
    let inside = |p: [f32; 2]| if under { p[1] >= edge } else { p[1] <= edge };
    let mut out = Vec::new();
    for i in 0..pts.len() {
        let (a, b) = (pts[i], pts[(i + 1) % pts.len()]);
        if inside(a) {
            out.push(a);
        }
        if inside(a) != inside(b) {
            let k = (edge - a[1]) / (b[1] - a[1]);
            out.push([a[0] + (b[0] - a[0]) * k, edge]);
        }
    }
    out
}

/// A convex polygon's band between y0 and y1 (the glass's rounded shape holding the paste, as the mockup's clip).
fn slab(pts: &[[f32; 2]], y0: f32, y1: f32) -> Vec<[f32; 2]> {
    half(&half(pts, y0, true), y1, false)
}

impl Game for Galley {
    fn id(&self) -> &'static str {
        "galley"
    }

    fn knobs(&self) -> &'static [&'static str] {
        &["band_half_frac", "feed_rate_frac_s", "run_on_s"]
    }

    fn step(&mut self, cx: &mut Ctx) {
        let mut r = cx.dice();
        let w = cx.knob("band_half_frac");
        self.rate = cx.knob("feed_rate_frac_s");
        self.tau = cx.knob("run_on_s");
        self.phase = if cx.part { Phase::Part } else { Phase::Fill };
        self.sel = 0;
        self.keys = false;
        self.purge_t = 0.0;
        self.splats.clear();
        self.noz = Noz {
            x: CRATE[0] + CRATE[2] / 2.0,
            y: CRATE[1] + CRATE[3] / 2.0 + 8.0,
            held: false,
            pad: false,
            set: !cx.part,
        };
        for col in &mut self.cols {
            let c = 0.35 + 0.5 * r.f();
            let l = 0.02 + 0.1 * r.f();
            *col = Col { c, w, l, f: 0.0, open: false, drain: false };
        }
        self.hand_carry = false;
    }

    fn update(&mut self, cx: &mut Ctx, dt: f32, input: &Input) {
        for p in &mut self.splats {
            p.life -= dt;
            if !p.land {
                p.vy += 900.0 * dt;
                p.x += p.vx * dt;
                p.y += p.vy * dt;
                if p.y > FLOOR && p.vy > 0.0 {
                    p.land = true;
                    p.y = FLOOR + p.x % 30.0;
                }
            }
        }
        self.splats.retain(|p| p.life > 0.0);
        match self.phase {
            Phase::Part => self.part_step(cx, dt, input),
            Phase::Fill => self.fill(cx, dt, input),
            Phase::Purge => {
                self.purge_t += dt;
                for col in &mut self.cols {
                    col.f = 0.0;
                    col.open = false;
                    col.l = (col.l - 0.8 * dt).max(0.0);
                }
            }
            Phase::Fitted => {}
        }
    }

    fn draw(&self, g: &Pen, _cx: &Ctx, _t: f32, _input: &Input) {
        g.rect(0.0, BAR_H, W, H, hex(0x070b12));
        let ready = self.phase == Phase::Fill && self.settled();
        let purging = self.phase == Phase::Purge;
        // The manifold from the columns' feet to the dispenser.
        pipe(g, &[[COLX[3], MANI], [DISP[0] + DISP[2] - 10.0, MANI]], 16.0);
        // The synthesizer's body.
        let [mx, my, mw, mh] = MACH;
        g.panel(mx, my, mw, mh, 18.0, hex(0x101722), c::LINE);
        g.rect(mx + 20.0, my + 16.0, mw - 40.0, 10.0, hex(0x0d131c));
        for (i, &x) in COLX.iter().enumerate() {
            let col = &self.cols[i];
            let (good, over) = (col.in_band(), col.drain);
            let x0 = x - CW / 2.0;
            // Glass, graduations, the band, the paste.
            g.round(x0, TOP, CW, CH, 14.0, Some(hex(0x060a10)), None);
            g.rect(x0 + 4.0, level(col.c + col.w), CW - 8.0, col.w * 2.0 * CH, rgba(61, 220, 132, 0.10));
            let glass = round_rect_points(x0, TOP, CW, CH, 14.0);
            let y = level(col.l);
            g.poly(&slab(&glass, y, BOT), alpha(COLS[i], 0.9));
            g.poly(&slab(&glass, y, y + 4.0), rgba(255, 255, 255, 0.25));
            if col.open || col.f > 0.01 {
                g.clip(x0, TOP, CW, CH).rect(x - 5.0, y, 10.0, BOT - y, COLS[i]);
            }
            for k in 1..10 {
                let len = if k % 5 != 0 { 10.0 } else { 18.0 };
                g.rect(x0, BOT - k as f32 * CH / 10.0, len, 2.0, hex(0x2a3446));
            }
            g.round(x0, TOP, CW, CH, 14.0, None, Some((3.0, if over { c::DANGER } else { hex(0x3a4658) })));
            let bc = if over {
                c::DANGER
            } else if good {
                c::OK
            } else {
                hex(0x2f7a50)
            };
            let (b0, b1) = (level(col.c + col.w), level(col.c - col.w));
            bracket(g, x0 - 8.0, b0, b1, 1.0, bc);
            bracket(g, x + CW / 2.0 + 8.0, b0, b1, -1.0, bc);
            // In band and still: a check on the cap.
            g.round(x0 - 6.0, TOP - 26.0, CW + 12.0, 26.0, 8.0, Some(c::STEEL), None);
            g.text(NAMES[i], x - 10.0, TOP - 13.0, 18.0, hex(0x0b111b), Align::Center);
            if good && col.f < STILL && !over {
                g.disc(x + 32.0, TOP - 13.0, 10.0, c::OK);
                g.path(
                    &[[x + 27.0, TOP - 13.0], [x + 31.0, TOP - 9.0], [x + 37.0, TOP - 17.0]],
                    false,
                    3.0,
                    hex(0x0b111b),
                );
            }
            // The valve under it.
            pipe(g, &[[x, BOT], [x, MANI]], 12.0);
            g.disc(x, VY, VR, if col.open { COLS[i] } else { hex(0x1d2738) });
            g.ring(x, VY, VR, if col.open { hex(0xfff3d6) } else { c::STEEL }, 4.0);
            g.disc(x, VY, VR - 16.0, if col.open { hex(0xfff3d6) } else { hex(0x2a3446) });
            if self.keys && self.sel == i && self.phase == Phase::Fill {
                g.ring(x, VY, VR + 9.0, c::AMBER, 3.0);
            }
        }
        // PURGE: the main action, lit once all four sit in their bands.
        let [px, py, pr] = PURGE;
        g.disc(px, py, pr + 10.0, hex(0x0d131c));
        let fill = if ready {
            c::AMBER
        } else if purging {
            hex(0x5a3a1a)
        } else {
            hex(0x1d2738)
        };
        g.disc(px, py, pr, fill);
        g.ring(px, py, pr, if ready { hex(0xffd9a8) } else { c::STEEL }, 4.0);
        g.poly(
            &[[px - 24.0, py - 26.0], [px + 24.0, py - 26.0], [px, py + 4.0]],
            if ready { hex(0x1a0f04) } else { hex(0x3a4658) },
        );
        g.text("PURGE", px, py + 30.0, 22.0, if ready { hex(0x1a0f04) } else { c::DIM }, Align::Center);
        // The recipe card: the four bands at a glance.
        let [kx, ky, kw, kh] = CARD;
        g.round(kx, ky, kw, kh, 10.0, Some(hex(0xd9d2bf)), None);
        g.rect(kx + 16.0, ky + 18.0, kw - 32.0, 6.0, hex(0x8a8170));
        for (i, col) in self.cols.iter().enumerate() {
            let (x, y0, h) = (kx + 30.0 + i as f32 * 52.0, ky + 50.0, kh - 80.0);
            g.rect(x, y0, 32.0, h, hex(0xbdb5a0));
            let by = y0 + h * (1.0 - col.c - col.w);
            g.rect(x, by, 32.0, h * col.w * 2.0, COLS[i]);
            g.rect_stroke(x, by, 32.0, h * col.w * 2.0, 2.0, hex(0x3b352a));
            g.rect_stroke(x, y0, 32.0, h, 2.0, hex(0x3b352a));
        }
        // The dispenser: the nozzle and the bowl.
        let [dx, dy, dw, dh] = DISP;
        g.panel(dx, dy, dw, dh, 14.0, hex(0x0d131c), c::LINE);
        g.round(dx + 24.0, dy + 70.0, dw - 48.0, dh - 90.0, 10.0, Some(hex(0x05080d)), None);
        let [nx, ny] = NOZ;
        if purging {
            if self.purge_t < 1.4 {
                g.rect(nx - 6.0, ny + 18.0, 12.0, FLOOR - ny - 18.0, hex(0xc9a36a));
            }
            let m = (self.purge_t / 1.4).min(1.0);
            let pi = std::f32::consts::PI;
            g.poly(&ellipse(nx, FLOOR, 50.0 * m + 1.0, 22.0 * m + 1.0, pi, 2.0 * pi), hex(0xc9a36a));
        }
        g.poly(&[[nx - 70.0, 636.0], [nx + 70.0, 636.0], [nx + 52.0, 670.0], [nx - 52.0, 670.0]], hex(0x7d8796));
        if self.noz.set {
            nozzle(g, nx, ny);
        } else {
            g.ring(nx, ny, 30.0, c::AMBER, 4.0);
            g.disc(nx, ny, 26.0, hex(0x120d08));
        }
        // Paste on everything.
        for p in &self.splats {
            let a = g.alpha(p.life.min(1.0));
            if p.land {
                a.poly(&ellipse(p.x, p.y, 12.0, 5.0, 0.0, std::f32::consts::TAU), p.c);
            } else {
                a.disc(p.x, p.y, 7.0, p.c);
            }
        }
        if !self.noz.set {
            let [cx0, cy0, cw, chh] = CRATE;
            g.panel(cx0, cy0, cw, chh, 14.0, hex(0x141b27), c::LINE);
            nozzle(g, self.noz.x, self.noz.y);
            g.ring(self.noz.x, self.noz.y - 4.0, 40.0, hex(0xf0c08a), 3.0);
        }
    }

    fn hand(&mut self, _cx: &Ctx, _t: f32) -> Input {
        match self.phase {
            Phase::Part => {
                // Pick the nozzle up, carry it onto the dispenser in a few frames, let go there.
                let (tx, ty) = (NOZ[0], NOZ[1]);
                let (x, y) = (self.noz.x, self.noz.y);
                if !self.noz.held && !self.hand_carry {
                    self.hand_carry = true;
                    return Input::hold(x, y, true);
                }
                let d = (tx - x).hypot(ty - y);
                if d > 2.0 {
                    let k = (30.0 / d).min(1.0);
                    return Input::hold(x + (tx - x) * k, y + (ty - y) * k, false);
                }
                self.hand_carry = false;
                Input::release(tx, ty)
            }
            Phase::Fill => {
                if self.settled() {
                    return Input::hold(PURGE[0], PURGE[1], true);
                }
                // The first column not settled: hold its valve until what runs on after letting go lands it on the
                // band's centre (the feed dies as f exp(-t / tau), so about f tau more comes in), then wait.
                let Some(i) = (0..4).find(|&i| !self.cols[i].settled()) else { return Input::default() };
                let col = &self.cols[i];
                if !col.drain && !col.in_band() && col.l + col.f * self.tau < col.c {
                    Input::hold(COLX[i], VY, false)
                } else {
                    Input::default()
                }
            }
            Phase::Fitted | Phase::Purge => Input::default(),
        }
    }

    fn guide_now(&self) -> Option<usize> {
        match self.phase {
            Phase::Fill => Some(0),
            Phase::Purge => Some(2),
            Phase::Part | Phase::Fitted => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::games::tests::{fumble_check, plays_to_end};

    #[test]
    fn a_steady_hand_repairs_it_damaged_disabled_and_destroyed() {
        plays_to_end("galley");
    }

    #[test]
    fn holding_a_valve_past_its_band_spills_the_paste() {
        // The first column's valve held open until it runs over its band.
        fumble_check("galley", |_r| crate::kit::Input::hold(440.0, 572.0, false));
    }
}
