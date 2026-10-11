//! The life support scrubbers' repair (repair-minigames design 2), from `docs/mockups/repairs/scrubbers.js`.
//!
//! The scrubber cabinet holds one CO2 cartridge. Drag the spent one (dark, hatched) out of its slot into the bin, and
//! a fresh one from the rack into the slot; the blower spins up. Then trim the three gas valves (turn a wheel round
//! its hub, the wheel over it, or the arrows) until each gauge's needle, O2, N2 and CO2, sits in its green band and
//! holds there a moment. A round is one cartridge and one bank trimmed; each level narrows the bands, the mix wanders
//! more and each valve pulls on the next gauge. A cartridge let go short of the slot or the bin, or a needle left in
//! the red: a fumble, a hiss, the room's CO2 rises. Space moves the next cartridge. A disabled scrubber's first step
//! fits the new blower fan: drag it from the crate into its housing. The cabinet's cover is the kit's.

use egui::Key;

use crate::games::breakers::{pie, wrap, Hand, PART_STEP};
use crate::kit::{Ctx, Game, Input, BAR_H, H, W};
use crate::pen::{c, hex, Align, Pen};

/// In the engine.
pub const PORTED: bool = true;

/// A fresh game.
pub fn new() -> Box<dyn Game> {
    Box::new(Scrubbers::default())
}

const HISS: &str = "Hiss: the room's CO2 rises";
/// The scrubber cabinet: x, y, w, h.
const CAB: [f32; 4] = [90.0, 100.0, 420.0, 590.0];
/// Its cartridge slot.
const SLOT: [f32; 4] = [225.0, 270.0, 150.0, 320.0];
/// Fresh cartridges wait here.
const RACK: [f32; 4] = [545.0, 110.0, 150.0, 360.0];
/// Spent ones go down here.
const BIN: [f32; 4] = [545.0, 500.0, 150.0, 190.0];
/// A cartridge, px.
const CW: f32 = 120.0;
const CH: f32 = 290.0;
const SEATED: [f32; 2] = [300.0, 430.0];
const RACKED: [f32; 2] = [620.0, 290.0];
const BINNED: [f32; 2] = [620.0, 610.0];
/// The blower's housing: x, y, r.
const FAN: [f32; 3] = [300.0, 185.0, 58.0];
/// The spare fan's crate (part step).
const CRATE: [f32; 4] = [1070.0, 565.0, 150.0, 115.0];
const GX: [f32; 3] = [830.0, 990.0, 1150.0];
const NAMES: [&str; 3] = ["O2", "N2", "CO2"];
const GY: f32 = 235.0;
const GR: f32 = 70.0;
const VY: f32 = 420.0;
const VR: f32 = 50.0;
const HEADER: f32 = 600.0;
/// A gauge's sweep: lower left, over the top, lower right.
const A0: f32 = 0.8 * std::f32::consts::PI;
const SWEEP: f32 = 1.4 * std::f32::consts::PI;
/// Each end of a gauge in the red, as a fraction of the sweep.
const RED: f32 = 0.08;
/// A needle left in the red this long, s: a fumble.
const RED_S: f32 = 1.2;
/// A needle held in its band this long, s: that gas is set.
const HOLD_S: f32 = 1.0;
/// The steady hand's pointer on a valve wheel, px from its hub.
const HAND_R: f32 = 40.0;

fn in_rect(r: [f32; 4], x: f32, y: f32) -> bool {
    x >= r[0] && x <= r[0] + r[2] && y >= r[1] && y <= r[1] + r[3]
}

fn ang(v: f32) -> f32 {
    A0 + SWEEP * v
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Phase {
    #[default]
    Part,
    Fitted,
    Out,
    In,
    Trim,
    Done,
}

#[derive(Clone, Copy, Debug, Default)]
struct Cart {
    x: f32,
    y: f32,
    home: [f32; 2],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Which {
    Spent,
    Fresh,
}

#[derive(Clone, Copy, Debug, Default)]
struct Valve {
    c: f32,
    w: f32,
    bias: f32,
    u: f32,
    u0: f32,
    v: f32,
    hold: f32,
    red: f32,
    locked: bool,
    ph: f32,
    fr: f32,
}

/// The game's state.
#[derive(Default)]
pub struct Scrubbers {
    t: f32,
    phase: Phase,
    spent: Cart,
    fresh: Cart,
    fan: [f32; 2],
    fan_held: bool,
    fan_pad: bool,
    fan_set: bool,
    held: Option<Which>,
    off: [f32; 2],
    sel: usize,
    drag: Option<usize>,
    last_a: f32,
    keys: bool,
    spin: f32,
    hiss: f32,
    valves: [Valve; 3],
    amp: f32,
    pull: f32,
    hand: Hand,
    /// The hand on a valve wheel: which, and the angle its pointer is at round the hub.
    wheel: Option<(usize, f32)>,
}

impl Scrubbers {
    fn cart(&mut self, w: Which) -> &mut Cart {
        match w {
            Which::Spent => &mut self.spent,
            Which::Fresh => &mut self.fresh,
        }
    }

    fn part_step(&mut self, cx: &mut Ctx, dt: f32, input: &Input) {
        if input.stick != [0.0, 0.0] {
            self.fan[0] += input.stick[0] * 480.0 * dt;
            self.fan[1] += input.stick[1] * 480.0 * dt;
            self.fan_pad = true;
        }
        if input.pressed && input.dist(self.fan[0], self.fan[1]) < 50.0 {
            self.fan_held = true;
        }
        if self.fan_held && input.down {
            self.fan = [input.x, input.y];
        }
        if (self.fan_held && input.released) || (self.fan_pad && input.action_pressed) {
            self.fan_held = false;
            self.fan_pad = false;
            if (self.fan[0] - FAN[0]).hypot(self.fan[1] - FAN[1]) < 44.0 {
                self.fan_set = true;
                self.fan = [FAN[0], FAN[1]];
                self.phase = Phase::Fitted;
                cx.step_done();
            }
        }
    }

    fn swap(&mut self, cx: &mut Ctx, input: &Input) {
        let spent = self.phase == Phase::Out;
        let which = if spent { Which::Spent } else { Which::Fresh };
        let seat = |s: &mut Self| {
            if spent {
                s.spent.home = BINNED;
                s.phase = Phase::In;
            } else {
                s.fresh.home = SEATED;
                s.phase = Phase::Trim;
            }
        };
        if input.action_pressed && self.held.is_none() {
            seat(self);
            return;
        }
        let k = *self.cart(which);
        if input.pressed && (input.x - k.x).abs() < CW / 2.0 && (input.y - k.y).abs() < CH / 2.0 {
            self.held = Some(which);
            self.off = [k.x - input.x, k.y - input.y];
        }
        if self.held == Some(which) && input.down {
            let o = self.off;
            let k = self.cart(which);
            k.x = input.x + o[0];
            k.y = input.y + o[1];
        }
        if self.held == Some(which) && input.released {
            self.held = None;
            if in_rect(if spent { BIN } else { SLOT }, input.x, input.y) {
                seat(self);
            } else if !in_rect(if spent { SLOT } else { RACK }, input.x, input.y) {
                self.hiss = 1.2;
                cx.fumble(HISS);
            }
        }
    }

    fn trim(&mut self, cx: &mut Ctx, dt: f32, input: &Input) {
        if input.hit(Key::ArrowLeft) || input.hit(Key::A) {
            self.sel = (self.sel + 2) % 3;
            self.keys = true;
        }
        if input.hit(Key::ArrowRight) || input.hit(Key::D) {
            self.sel = (self.sel + 1) % 3;
            self.keys = true;
        }
        let sel = self.sel;
        if input.stick[1] != 0.0 && !self.valves[sel].locked {
            self.valves[sel].u = (self.valves[sel].u - input.stick[1] * 0.3 * dt).clamp(0.0, 1.0);
            self.keys = true;
        }
        // The pointer turns a wheel round its hub: a turn and a half from shut to open.
        let over = GX.iter().position(|&x| input.dist(x, VY) < VR + 14.0);
        if let (true, Some(o)) = (input.pressed, over) {
            if !self.valves[o].locked {
                self.drag = Some(o);
                self.sel = o;
                self.last_a = (input.y - VY).atan2(input.x - GX[o]);
            }
        }
        if let Some(d) = self.drag {
            if !input.down || self.valves[d].locked {
                self.drag = None;
            } else {
                let a = (input.y - VY).atan2(input.x - GX[d]);
                if input.dist(GX[d], VY) > 10.0 {
                    let u = self.valves[d].u + wrap(a - self.last_a) / (1.5 * std::f32::consts::PI);
                    self.valves[d].u = u.clamp(0.0, 1.0);
                }
                self.last_a = a;
            }
        }
        if let (true, Some(o)) = (input.wheel != 0.0, over) {
            if !self.valves[o].locked {
                self.valves[o].u = (self.valves[o].u - input.wheel * 0.02).clamp(0.0, 1.0);
            }
        }
        // The gauges: each needle lags its valve, wanders with the mix, and (later levels) feels the valve before it.
        for i in 0..3 {
            let prev = self.valves[(i + 2) % 3];
            let g = &mut self.valves[i];
            if g.locked {
                g.v += (g.c - g.v) * (dt * 4.0).min(1.0);
                continue;
            }
            let raw = (g.u + g.bias + self.pull * (prev.u - prev.u0) + self.amp * (self.t * g.fr + g.ph).sin())
                .clamp(0.0, 1.0);
            g.v += (raw - g.v) * (dt * 3.0).min(1.0);
            if (g.v - g.c).abs() <= g.w {
                g.hold += dt;
                if g.hold >= HOLD_S {
                    g.locked = true;
                }
            } else {
                g.hold = (g.hold - dt * 2.0).max(0.0);
            }
            g.red = if g.v < RED || g.v > 1.0 - RED { g.red + dt } else { 0.0 };
        }
        if let Some(b) = self.valves.iter().position(|g| g.red >= RED_S) {
            let g = &mut self.valves[b];
            g.u = g.u0;
            g.red = 0.0;
            self.drag = None;
            self.hiss = 1.2;
            cx.fumble(HISS);
            return;
        }
        if self.valves.iter().all(|g| g.locked) {
            self.phase = Phase::Done;
            cx.step_done();
        }
    }

    fn gauge(&self, g: &Pen, i: usize, live: bool) {
        use std::f32::consts::{FRAC_PI_2, TAU};
        let (v, x) = (self.valves[i], GX[i]);
        g.disc(x, GY, GR + 8.0, hex(0x0d131c));
        g.ring(x, GY, GR + 8.0, c::STEEL, 4.0);
        g.disc(x, GY, GR, hex(0x070b12));
        let arc = |a: f32, b: f32, col, w| g.arc(x, GY, GR - 14.0, ang(a), ang(b), w, col);
        arc(0.0, 1.0, hex(0x1b2433), 14.0);
        arc(0.0, RED, c::DANGER, 14.0);
        arc(1.0 - RED, 1.0, c::DANGER, 14.0);
        // The red ends carry ticks across them, so the red is a shape as well as a colour.
        for r0 in [0.0, 1.0 - RED] {
            for k in 1..4 {
                let a = ang(r0 + RED * k as f32 / 4.0);
                g.line(
                    x + a.cos() * (GR - 22.0),
                    GY + a.sin() * (GR - 22.0),
                    x + a.cos() * (GR - 6.0),
                    GY + a.sin() * (GR - 6.0),
                    3.0,
                    hex(0x070b12),
                );
            }
        }
        arc(v.c - v.w, v.c + v.w, if live { c::OK } else { hex(0x245c3c) }, 20.0);
        // The hold: a ring filling round the dial; set: a check under the hub.
        if v.hold > 0.0 && !v.locked {
            g.arc(x, GY, GR + 16.0, -FRAC_PI_2, -FRAC_PI_2 + TAU * (v.hold / HOLD_S).min(1.0), 5.0, c::OK);
        }
        if v.locked {
            g.ring(x, GY, GR + 16.0, c::OK, 5.0);
        }
        let a = ang(v.v);
        g.path_round(
            &[[x, GY], [x + a.cos() * (GR - 8.0), GY + a.sin() * (GR - 8.0)]],
            4.0,
            if v.red > 0.0 { c::DANGER } else { c::FG },
        );
        g.disc(x, GY, 9.0, hex(0x9aa6b6));
        g.text(NAMES[i], x, GY + 40.0, 22.0, if v.locked { c::OK } else { c::FG }, Align::Center);
        if v.locked {
            g.path(&[[x - 9.0, GY + 58.0], [x - 2.0, GY + 65.0], [x + 11.0, GY + 51.0]], false, 4.0, c::OK);
        }
    }

    fn valve(&self, g: &Pen, i: usize, live: bool) {
        let (v, x) = (self.valves[i], GX[i]);
        g.disc(x, VY, VR, hex(0x141b27));
        let rim = if v.locked {
            c::OK
        } else if live {
            c::STEEL
        } else {
            hex(0x3a4658)
        };
        g.ring(x, VY, VR - 4.0, rim, 10.0);
        let a = v.u * 3.0 * std::f32::consts::PI;
        for k in 0..3 {
            let b = a + k as f32 * std::f32::consts::TAU / 3.0;
            g.line(
                x,
                VY,
                x + b.cos() * (VR - 8.0),
                VY + b.sin() * (VR - 8.0),
                7.0,
                if live { hex(0x9aa6b6) } else { hex(0x4a5566) },
            );
        }
        g.disc(x, VY, 11.0, hex(0x2a3446));
        g.ring(x, VY, 11.0, hex(0x9aa6b6), 2.0);
        if live && self.keys && i == self.sel && !v.locked {
            g.ring(x, VY, VR + 9.0, c::AMBER, 3.0);
        }
    }
}

/// A pipe: a dark casing, the metal, a highlight down the middle.
fn pipe(g: &Pen, pts: &[[f32; 2]], w: f32) {
    g.path_round(pts, w + 6.0, hex(0x232c3b));
    g.path_round(pts, w, hex(0x3a4658));
    g.path(pts, false, 3.0, hex(0x56637a));
}

fn fan_glyph(g: &Pen, x: f32, y: f32, r: f32, spin: f32, live: bool) {
    g.disc(x, y, r, hex(0x1d2738));
    for i in 0..5 {
        let a = spin + i as f32 * std::f32::consts::TAU / 5.0;
        g.poly(&pie(x, y, r - 6.0, a, a + 0.75), if live { hex(0x7d9ab8) } else { c::STEEL });
    }
    g.disc(x, y, r * 0.25, hex(0x2a3446));
    g.ring(x, y, r * 0.25, hex(0x9aa6b6), 2.0);
}

fn cartridge(g: &Pen, k: Cart, spent: bool) {
    let (x, y) = (k.x - CW / 2.0, k.y - CH / 2.0);
    g.round(x, y, CW, CH, 16.0, Some(if spent { hex(0x3b342d) } else { hex(0x8fa3b8) }), None);
    let q = g.clip(x, y, CW, CH);
    if spent {
        let mut i = -CH;
        while i < CW + CH {
            q.line(x + i, y, x + i - CH, y + CH, 7.0, hex(0x28221d));
            i += 20.0;
        }
    } else {
        q.rect(x + 12.0, y, 16.0, CH, hex(0xa9bccf));
    }
    g.round(x, y, CW, CH, 16.0, None, Some((2.0, hex(0x0b111b))));
    g.round(x + 6.0, y - 6.0, CW - 12.0, 24.0, 6.0, Some(c::STEEL), None);
    g.round(x + 6.0, y + CH - 18.0, CW - 12.0, 24.0, 6.0, Some(c::STEEL), None);
    g.arc(k.x, y - 6.0, 22.0, std::f32::consts::PI, std::f32::consts::TAU, 5.0, hex(0x9aa6b6));
    // The saturation window: green and clear when fresh, a dark red cross when spent.
    let wy = y + CH * 0.36;
    g.round(x + 22.0, wy, CW - 44.0, 44.0, 8.0, Some(if spent { hex(0x4a1c24) } else { hex(0x0f2a1c) }), None);
    if spent {
        g.cross(k.x, wy + 22.0, 12.0, 5.0, c::DANGER);
    } else {
        g.disc(k.x, wy + 22.0, 13.0, c::OK);
    }
    g.text("CO2", k.x, y + CH * 0.74, 24.0, if spent { hex(0x8a7d6e) } else { hex(0x0b111b) }, Align::Center);
}

impl Game for Scrubbers {
    fn id(&self) -> &'static str {
        "scrubbers"
    }

    fn knobs(&self) -> &'static [&'static str] {
        &["band_frac", "wander_frac", "pull_frac"]
    }

    fn step(&mut self, cx: &mut Ctx) {
        let mut r = cx.dice();
        let w = cx.knob("band_frac");
        self.t = 0.0;
        self.phase = if cx.part { Phase::Part } else { Phase::Out };
        self.spent = Cart { x: SEATED[0], y: SEATED[1], home: SEATED };
        self.fresh = Cart { x: RACKED[0], y: RACKED[1], home: RACKED };
        self.fan = [CRATE[0] + CRATE[2] / 2.0, CRATE[1] + CRATE[3] / 2.0];
        self.fan_held = false;
        self.fan_pad = false;
        self.fan_set = !cx.part;
        self.held = None;
        self.off = [0.0, 0.0];
        self.sel = 0;
        self.drag = None;
        self.last_a = 0.0;
        self.keys = false;
        self.spin = 0.0;
        self.hiss = 0.0;
        self.amp = cx.knob("wander_frac");
        self.pull = cx.knob("pull_frac");
        for v in &mut self.valves {
            let c = 0.3 + 0.4 * r.f();
            let bias = (r.f() - 0.5) * 0.2;
            let off = w + 0.08 + 0.08 * r.f();
            let mut v0 = c + if r.f() < 0.5 { -off } else { off };
            if !(0.15..=0.85).contains(&v0) {
                v0 = 2.0 * c - v0;
            }
            v0 = v0.clamp(0.15, 0.85);
            *v = Valve {
                c,
                w,
                bias,
                u: v0 - bias,
                u0: v0 - bias,
                v: v0,
                hold: 0.0,
                red: 0.0,
                locked: false,
                ph: r.f() * std::f32::consts::TAU,
                fr: 0.6 + 0.6 * r.f(),
            };
        }
        self.hand = Hand::default();
        self.wheel = None;
    }

    fn update(&mut self, cx: &mut Ctx, dt: f32, input: &Input) {
        self.t += dt;
        self.hiss = (self.hiss - dt).max(0.0);
        if matches!(self.phase, Phase::Trim | Phase::Done) {
            self.spin += dt * if self.phase == Phase::Done { 12.0 } else { 7.0 };
        }
        let k = (dt * 12.0).min(1.0);
        for w in [Which::Spent, Which::Fresh] {
            if self.held != Some(w) {
                let c = self.cart(w);
                c.x += (c.home[0] - c.x) * k;
                c.y += (c.home[1] - c.y) * k;
            }
        }
        match self.phase {
            Phase::Part => self.part_step(cx, dt, input),
            Phase::Out | Phase::In => self.swap(cx, input),
            Phase::Trim => self.trim(cx, dt, input),
            Phase::Fitted | Phase::Done => {}
        }
    }

    fn draw(&self, g0: &Pen, _cx: &Ctx, t: f32, _input: &Input) {
        g0.rect(0.0, BAR_H, W, H, hex(0x070b12));
        let live = matches!(self.phase, Phase::Trim | Phase::Done);
        let jolt = if self.hiss > 0.0 { (t * 40.0).sin() * 4.0 * self.hiss } else { 0.0 };
        let g = &g0.translate(jolt, 0.0);
        let [kx, ky, kw, kh] = CAB;
        // Pipes: the cabinet's outlet behind the bin to the valve bank, each valve up to its gauge.
        pipe(g, &[[kx + kw - 10.0, HEADER], [GX[2], HEADER]], 18.0);
        for x in GX {
            pipe(g, &[[x, HEADER], [x, VY]], 14.0);
            pipe(g, &[[x, VY], [x, GY]], 10.0);
        }
        // The cabinet: blower on top, the cartridge slot, the intake grille below.
        g.panel(kx, ky, kw, kh, 16.0, hex(0x101722), c::LINE);
        let [fx, fy, fr] = FAN;
        g.disc(fx, fy, fr + 10.0, hex(0x0b111b));
        g.ring(fx, fy, fr + 10.0, c::STEEL, 3.0);
        if self.fan_set {
            fan_glyph(g, fx, fy, fr, self.spin, live);
        } else {
            g.disc(fx, fy, fr, hex(0x120d08));
            g.ring(fx, fy, fr, c::AMBER, 4.0);
        }
        g.ring(fx, fy, fr + 4.0, hex(0x2a3446), 2.0);
        let into = self.phase == Phase::In;
        g.round(
            SLOT[0],
            SLOT[1],
            SLOT[2],
            SLOT[3],
            12.0,
            Some(hex(0x05080d)),
            Some(if into { (4.0, c::AMBER) } else { (2.0, c::STEEL) }),
        );
        for k in 0..4 {
            g.rect(kx + 60.0, 612.0 + k as f32 * 18.0, kw - 120.0, 8.0, hex(0x1a2230));
        }
        // The run lamp: a green disc running, a red triangle stopped.
        if live {
            g.disc(kx + kw - 40.0, ky + 34.0, 13.0, c::OK);
        } else {
            g.poly(&[[kx + kw - 40.0, ky + 20.0], [kx + kw - 55.0, ky + 46.0], [kx + kw - 25.0, ky + 46.0]], c::DANGER);
        }
        // The rack and the bin.
        let [rx, ry, rw, rh] = RACK;
        g.panel(rx, ry, rw, rh, 12.0, hex(0x0d131c), if self.held == Some(Which::Fresh) { c::AMBER } else { c::LINE });
        let [bx, by, bw, bh] = BIN;
        g.round(bx + 8.0, by, bw - 16.0, 30.0, 6.0, Some(hex(0x05080d)), None);
        let out = self.phase == Phase::Out;
        if !out || self.held == Some(Which::Spent) {
            cartridge(g, self.spent, true);
        }
        g.panel(
            bx,
            by + 22.0,
            bw,
            bh - 22.0,
            12.0,
            hex(0x141b27),
            if self.held == Some(Which::Spent) { c::AMBER } else { c::LINE },
        );
        g.path(
            &[[bx + 45.0, by + 80.0], [bx + bw / 2.0, by + 120.0], [bx + bw - 45.0, by + 80.0]],
            false,
            10.0,
            hex(0x3a4658),
        );
        if out && self.held != Some(Which::Spent) {
            cartridge(g, self.spent, true);
        }
        cartridge(g, self.fresh, false);
        for y in [ry + 50.0, ry + rh - 50.0] {
            g.rect(rx - 6.0, y - 6.0, 16.0, 12.0, c::STEEL);
            g.rect(rx + rw - 10.0, y - 6.0, 16.0, 12.0, c::STEEL);
        }
        // The valve bank and its gauges.
        for i in 0..3 {
            self.valve(g, i, live);
            self.gauge(g, i, live);
        }
        g.rect(0.0, 690.0, W, H - 690.0, hex(0x0a0f17));
        if matches!(self.phase, Phase::Part | Phase::Fitted) && !self.fan_set {
            let [cx0, cy0, cw, ch] = CRATE;
            g.panel(cx0, cy0, cw, ch, 14.0, hex(0x141b27), c::LINE);
            fan_glyph(g, self.fan[0], self.fan[1], 44.0, 0.0, false);
            g.ring(self.fan[0], self.fan[1], 46.0, hex(0xf0c08a), 3.0);
        }
    }

    fn hand(&mut self, _cx: &Ctx, _t: f32) -> Input {
        if self.hand.resting() {
            return Input::default();
        }
        match self.phase {
            Phase::Part => self.hand.carry(1, self.fan, [FAN[0], FAN[1]], PART_STEP),
            Phase::Out => self.hand.carry(2, [self.spent.x, self.spent.y], [620.0, 600.0], 20.0),
            Phase::In => self.hand.carry(3, [self.fresh.x, self.fresh.y], SEATED, 20.0),
            Phase::Trim => {
                // Turn each valve's wheel in turn to where its needle sits centred in green, then keep still.
                let Some(i) = self.valves.iter().position(|v| !v.locked) else { return self.hand.idle() };
                let (v, prev) = (self.valves[i], self.valves[(i + 2) % 3]);
                let want = (v.c - v.bias - self.pull * (prev.u - prev.u0)).clamp(0.0, 1.0);
                let a = match self.wheel {
                    Some((w, a)) if w == i => {
                        let du = (want - v.u).clamp(-0.6 / 60.0, 0.6 / 60.0);
                        a + du * 1.5 * std::f32::consts::PI
                    }
                    _ => -std::f32::consts::FRAC_PI_2,
                };
                self.wheel = Some((i, a));
                self.hand.hold(10 + i as u32, [GX[i] + a.cos() * HAND_R, VY + a.sin() * HAND_R])
            }
            _ => self.hand.idle(),
        }
    }

    fn guide_now(&self) -> Option<usize> {
        match self.phase {
            Phase::Out => Some(1),
            Phase::In => Some(2),
            Phase::Trim => Some(3),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::games::tests::{fumble_check, plays_to_end};
    use crate::kit::Input;

    #[test]
    fn a_steady_hand_repairs_it_damaged_disabled_and_destroyed() {
        plays_to_end("scrubbers");
    }

    #[test]
    fn a_cartridge_let_go_short_of_the_bin_hisses() {
        // Pick the spent cartridge out of its slot and let it go over open floor, again and again.
        let mut n = 0;
        fumble_check("scrubbers", move |_r| mistake(&mut n));
    }

    /// Frame `n` of the mistake: pick the spent cartridge up and let it go over open floor.
    fn mistake(n: &mut u32) -> Input {
        *n += 1;
        match *n % 3 {
            1 => Input::hold(300.0, 430.0, true),
            2 => Input::hold(450.0, 200.0, false),
            _ => Input::release(450.0, 200.0),
        }
    }
}
