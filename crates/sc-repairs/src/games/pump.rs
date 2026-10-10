//! A coolant pump's repair (repair-minigames design 6h; reactor-cooling 6a opens it for the core pumps, the radiator
//! pumps and the makeup pump), from `docs/mockups/repairs/pump.js`.
//!
//! The pump from the side: the volute (the snail-shell casing) on the left with its impeller behind a window, the
//! coupling, the motor on two feet on the right. A guard over the coupling is screwed on (the kit's cover). A round is
//! two phases, in order:
//! - **align**: a laser on the pump's shaft throws two dots on two targets on the motor, near and far. Drag each
//!   foot's shim handle up or down (W and S on the selected foot, Tab or A and D to the other): the front foot moves
//!   the near dot most, the rear foot the far one, and each pulls the other's dot a little. Both dots held in their
//!   rings: aligned. A dot left off its target 1.2 s is the fumble "Coupling knocks: the bearings heat".
//! - **start**: raise the speed lever (drag it, or W and S) to the band at 100% and hold it there, keeping the suction
//!   gauge's needle out of the red. Speed lowers suction, and a fast ramp drops it further while it ramps. The needle
//!   in the red is the fumble "Cavitation: the impeller pits", bubbles in the window and the speed knocked back.
//!
//! Each level shrinks the dots' rings and raises the red band. A disabled pump's first step fits a new impeller: drag
//! it from the crate onto the shaft, its vanes curving away from the casing's rotation arrow; tap it (or F) to turn it
//! over. Fitted the wrong way round is the fumble "Impeller backwards: no flow".

use std::f32::consts::{FRAC_PI_2, PI, TAU};

use egui::{Color32, Key};

use crate::kit::{nearest, Ctx, Game, Input, BAR_H, H, TOUCH_R, W};
use crate::pen::{c, hex, rgba, Align, Pen};

/// In the engine.
pub const PORTED: bool = true;

/// A fresh game.
pub fn new() -> Box<dyn Game> {
    Box::new(Pump::default())
}

const KNOCK: &str = "Coupling knocks: the bearings heat";
const CAVITATE: &str = "Cavitation: the impeller pits";
const BACKWARDS: &str = "Impeller backwards: no flow";

/// The volute's centre and radius; its window shows the impeller.
const VOL: [f32; 3] = [290.0, 275.0, 108.0];
const WIN_R: f32 = 64.0;
/// The pump's shaft line.
const SHAFT_Y: f32 = VOL[1];
/// The coupling's face.
const CPL_X: f32 = 520.0;
/// The motor's body along the shaft: x0, x1, height.
const MOTOR: [f32; 3] = [560.0, 900.0, 150.0];
/// The front and rear feet, x.
const FOOT: [f32; 2] = [640.0, 840.0];
const SPAN: f32 = FOOT[1] - FOOT[0];
/// The skid's top.
const BASE_Y: f32 = 420.0;
/// The shim handles' tracks under each foot: centre y and half length.
const TRACK_Y: f32 = 560.0;
const TRACK_HALF: f32 = 78.0;
/// Px of handle per px of foot height.
const KNOB: f32 = 1.6;
/// A foot's travel either way, px.
const H_MAX: f32 = 46.0;
/// The near and far targets.
const TGT: [[f32; 2]; 2] = [[1010.0, 270.0], [1160.0, 270.0]];
const PLATE_R: f32 = 62.0;
/// Px of dot per px of motor error.
const DOT_K: f32 = 1.5;
/// How much each foot pulls the other's dot.
const PULL: f32 = 0.35;
/// Seconds a dot may sit off its plate.
const OFF_S: f32 = 1.2;
/// Seconds both dots must sit in their rings.
const HOLD_S: f32 = 0.6;
/// The speed lever: x, the y of 0 speed, the y of full speed, width.
const LEVER: [f32; 4] = [1190.0, 640.0, 140.0, 100.0];
const V_MAX: f32 = 1.2;
const BAND: [f32; 2] = [0.95, 1.05];
const HOLD_RUN_S: f32 = 1.0;
/// The suction gauge: centre and radius.
const GAUGE: [f32; 3] = [1000.0, 540.0, 96.0];
const A0: f32 = 0.8 * PI;
const SWEEP: f32 = 1.4 * PI;
/// Seconds of lag on suction, and how much a ramp drops it.
const LAG_S: f32 = 0.8;
const DROP: f32 = 0.6;
/// The part step's crate.
const CRATE: [f32; 4] = [700.0, 480.0, 200.0, 190.0];
const IMP_R: f32 = 52.0;

fn crate_mid() -> [f32; 2] {
    [CRATE[0] + CRATE[2] / 2.0, CRATE[1] + CRATE[3] / 2.0]
}

fn lever_y(v: f32) -> f32 {
    LEVER[1] + (LEVER[2] - LEVER[1]) * v / V_MAX
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Phase {
    #[default]
    Part,
    Align,
    Start,
}

#[derive(Clone, Debug, Default)]
struct Imp {
    x: f32,
    y: f32,
    /// -1 backward-curved (right for the clockwise casing), +1 the wrong way.
    sweep: f32,
    held: bool,
    moved: f32,
    ox: f32,
    oy: f32,
    px: f32,
    py: f32,
    seated: bool,
}

/// What the hand is doing in the part step.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum PartHand {
    #[default]
    Idle,
    Tapped,
    Carrying,
}

/// The game's state.
#[derive(Default)]
pub struct Pump {
    phase: Phase,
    t: f32,
    sel: usize,
    keys: bool,
    spin: f32,
    bubbles: f32,
    knock: f32,
    done: bool,
    /// The feet's heights, where they belong, and each dot's time off its plate.
    h: [f32; 2],
    want: [f32; 2],
    off: [f32; 2],
    /// The foot in the hand, or the lever (start).
    grab: Option<usize>,
    gy: f32,
    hold: f32,
    imp: Imp,
    // The start.
    v: f32,
    v_prev: f32,
    rate: f32,
    p_lag: f32,
    p: f32,
    // Knobs.
    ring_px: f32,
    red_frac: f32,
    // The hand's memory.
    hand_part: PartHand,
    hand_v: f32,
}

impl Pump {
    /// The dots' readings: the motor's error at each target, from the feet's heights against where they belong.
    fn dots(&self) -> [f32; 2] {
        let (ef, er) = (self.h[0] - self.want[0], self.h[1] - self.want[1]);
        [(ef + PULL * er) * DOT_K, (er + PULL * ef) * DOT_K]
    }

    /// Where the motor's shaft line is at x (it tilts with the feet): canvas y.
    fn motor_y(&self, x: f32) -> f32 {
        let (ef, er) = (self.h[0] - self.want[0], self.h[1] - self.want[1]);
        SHAFT_Y - (ef + (er - ef) * (x - FOOT[0]) / SPAN)
    }

    fn handle(&self, k: usize) -> [f32; 2] {
        [FOOT[k], TRACK_Y - self.h[k] * KNOB]
    }

    fn part_update(&mut self, cx: &mut Ctx, dt: f32, input: &Input) {
        let m = &mut self.imp;
        if m.seated {
            return;
        }
        if input.hit(Key::F) {
            m.sweep = -m.sweep;
        }
        if input.stick != [0.0, 0.0] {
            self.keys = true;
            m.x += input.stick[0] * 480.0 * dt;
            m.y += input.stick[1] * 480.0 * dt;
        }
        if input.pressed && input.dist(m.x, m.y) < IMP_R + 14.0 {
            m.held = true;
            m.moved = 0.0;
            m.ox = input.x - m.x;
            m.oy = input.y - m.y;
            m.px = input.x;
            m.py = input.y;
        }
        if m.held && input.down {
            m.moved += (input.x - m.px).hypot(input.y - m.py);
            m.px = input.x;
            m.py = input.y;
            m.x = input.x - m.ox;
            m.y = input.y - m.oy;
        }
        let drop = (m.held && input.released) || (self.keys && input.action_pressed);
        if !drop {
            return;
        }
        let was_tap = m.held && m.moved < 12.0;
        m.held = false;
        let [hx, hy] = crate_mid();
        if was_tap {
            m.sweep = -m.sweep;
            m.x = hx;
            m.y = hy;
            return;
        }
        if (m.x - VOL[0]).hypot(m.y - VOL[1]) < 60.0 {
            // Backward-curved vanes (sweep -1) suit the casing's clockwise turn; the other way round pumps nothing.
            if m.sweep < 0.0 {
                m.seated = true;
                m.x = VOL[0];
                m.y = VOL[1];
                cx.step_done();
                return;
            }
            cx.fumble(BACKWARDS);
        }
        m.x = hx;
        m.y = hy;
    }

    fn align_update(&mut self, cx: &mut Ctx, dt: f32, input: &Input) {
        if self.done {
            return;
        }
        // Keys: the selected foot up and down, Tab or A and D to the other.
        for k in [Key::Tab, Key::A, Key::D, Key::ArrowLeft, Key::ArrowRight] {
            if input.hit(k) {
                self.keys = true;
                self.sel = 1 - self.sel;
            }
        }
        let up = |a: Key, b: Key| f32::from(u8::from(input.held(a) || input.held(b)));
        let ky = up(Key::W, Key::ArrowUp) - up(Key::S, Key::ArrowDown);
        if ky != 0.0 {
            self.keys = true;
            self.h[self.sel] = (self.h[self.sel] + ky * 30.0 * dt).clamp(-H_MAX, H_MAX);
        }
        if input.pressed {
            let pts = [Some(self.handle(0)), Some(self.handle(1))];
            if let Some(i) = nearest(&pts, input.x, input.y, TOUCH_R + 20.0) {
                self.grab = Some(i);
                self.sel = i;
                self.keys = false;
                self.gy = input.y + self.h[i] * KNOB;
            }
        }
        if let (Some(g), true) = (self.grab, input.down) {
            self.h[g] = ((self.gy - input.y) / KNOB).clamp(-H_MAX, H_MAX);
        }
        if self.grab.is_some() && input.released {
            self.grab = None;
        }
        let d = self.dots();
        // A dot off its plate too long: the coupling knocks.
        for (off, dot) in self.off.iter_mut().zip(d) {
            *off = if dot.abs() > PLATE_R { *off + dt } else { 0.0 };
        }
        if self.off.iter().any(|&o| o > OFF_S) {
            self.off = [0.0, 0.0];
            self.knock = 1.0;
            cx.fumble(KNOCK);
            return;
        }
        let r = self.ring_px;
        self.hold = if d.iter().all(|v| v.abs() <= r) && self.grab.is_none() { self.hold + dt } else { 0.0 };
        if self.hold >= HOLD_S {
            self.done = true;
            cx.step_done();
        }
    }

    fn start_update(&mut self, cx: &mut Ctx, dt: f32, input: &Input) {
        if !self.done {
            let up = |a: Key, b: Key| f32::from(u8::from(input.held(a) || input.held(b)));
            let ky = up(Key::W, Key::ArrowUp) - up(Key::S, Key::ArrowDown);
            if ky != 0.0 {
                self.keys = true;
                self.v = (self.v + ky * 0.3 * dt).clamp(0.0, V_MAX);
            }
            if input.pressed
                && (input.x - LEVER[0]).abs() < LEVER[3] / 2.0 + 20.0
                && (input.y - lever_y(self.v)).abs() < 50.0
            {
                self.grab = Some(0);
                self.keys = false;
                self.gy = input.y - lever_y(self.v);
            }
            if self.grab.is_some() && input.down {
                self.v = ((input.y - self.gy - LEVER[1]) * V_MAX / (LEVER[2] - LEVER[1])).clamp(0.0, V_MAX);
            }
            if self.grab.is_some() && input.released {
                self.grab = None;
            }
        }
        // Suction: lower with speed, followed with a lag, and lower still while the speed rises.
        let rate = ((self.v - self.v_prev) / dt.max(1e-3)).max(0.0);
        self.v_prev = self.v;
        self.rate += (rate - self.rate) * (dt * 8.0).min(1.0);
        self.p_lag += (1.0 - 0.55 * self.v * self.v - self.p_lag) * (dt / LAG_S).min(1.0);
        self.p = self.p_lag - DROP * self.rate;
        self.spin += dt * self.v * 14.0;
        self.bubbles = (self.bubbles - dt).max(0.0);
        if self.done {
            return;
        }
        let red = self.red_frac;
        if self.p < red && self.v > 0.2 {
            self.v = 0.4;
            self.v_prev = 0.4;
            self.rate = 0.0;
            self.grab = None;
            self.bubbles = 1.4;
            self.hold = 0.0;
            cx.fumble(CAVITATE);
            return;
        }
        self.hold = if self.v >= BAND[0] && self.v <= BAND[1] && self.p >= red { self.hold + dt } else { 0.0 };
        if self.hold >= HOLD_RUN_S {
            self.done = true;
            cx.step_done();
        }
    }
}

impl Game for Pump {
    fn id(&self) -> &'static str {
        "pump"
    }

    fn knobs(&self) -> &'static [&'static str] {
        &["ring_px", "red_frac"]
    }

    fn phases(&self, _rounds: u32) -> &'static [&'static str] {
        &["align", "start"]
    }

    fn step(&mut self, cx: &mut Ctx) {
        let mut r = cx.dice();
        self.ring_px = cx.knob("ring_px");
        self.red_frac = cx.knob("red_frac");
        self.phase = if cx.part {
            Phase::Part
        } else if cx.phase_name == "start" {
            Phase::Start
        } else {
            Phase::Align
        };
        // The motor sits true (feet where they belong) except while it is being aligned.
        self.t = 0.0;
        self.sel = 0;
        self.keys = false;
        self.spin = 0.0;
        self.bubbles = 0.0;
        self.knock = 0.0;
        self.done = false;
        self.h = [0.0, 0.0];
        self.want = [0.0, 0.0];
        self.off = [0.0, 0.0];
        self.grab = None;
        self.hold = 0.0;
        self.hand_part = PartHand::Idle;
        match self.phase {
            Phase::Part => {
                // The new impeller waits in the crate, the right way round or not, at random.
                let [x, y] = crate_mid();
                let sweep = if r.f() < 0.5 { 1.0 } else { -1.0 };
                self.imp = Imp { x, y, sweep, ..Imp::default() };
            }
            Phase::Align => {
                // Where the feet belong, and where they start: both dots on their plates, outside their rings.
                self.want = [(r.f() - 0.5) * 30.0, (r.f() - 0.5) * 30.0];
                for _ in 0..50 {
                    let mut pick = |w: f32| {
                        let s = if r.f() < 0.5 { -1.0 } else { 1.0 };
                        (w + s * (16.0 + r.f() * 16.0)).clamp(-H_MAX, H_MAX)
                    };
                    let h0 = pick(self.want[0]);
                    let h1 = pick(self.want[1]);
                    self.h = [h0, h1];
                    let rr = self.ring_px;
                    if self.dots().iter().all(|v| v.abs() > rr + 6.0 && v.abs() < PLATE_R - 12.0) {
                        break;
                    }
                }
            }
            Phase::Start => {
                self.v = 0.0;
                self.v_prev = 0.0;
                self.rate = 0.0;
                self.p_lag = 1.0;
                self.p = 1.0;
                self.hand_v = 0.0;
            }
        }
    }

    fn update(&mut self, cx: &mut Ctx, dt: f32, input: &Input) {
        self.t += dt;
        self.knock = (self.knock - dt * 2.0).max(0.0);
        match self.phase {
            Phase::Part => self.part_update(cx, dt, input),
            Phase::Align => self.align_update(cx, dt, input),
            Phase::Start => self.start_update(cx, dt, input),
        }
    }

    fn draw(&self, g: &Pen, _cx: &Ctx, t: f32, _input: &Input) {
        g.rect(0.0, BAR_H, W, H, hex(0x070b12));
        let shake = if self.knock > 0.0 { (t * 60.0).sin() * 4.0 * self.knock } else { 0.0 };
        self.machine(&g.translate(shake, 0.0), t);
        match self.phase {
            Phase::Align => self.targets(g),
            Phase::Start => self.start_panel(g),
            Phase::Part => self.part_panel(g),
        }
    }

    fn hand(&mut self, _cx: &Ctx, _t: f32) -> Input {
        match self.phase {
            Phase::Part => self.hand_part(),
            Phase::Align => self.hand_align(),
            Phase::Start => self.hand_start(),
        }
    }

    fn guide_now(&self) -> Option<usize> {
        Some(match self.phase {
            Phase::Part => 1,
            Phase::Align => 2,
            Phase::Start => 3,
        })
    }
}

// ------------------------------------------------------------------------------------------------ the hand

impl Pump {
    fn hand_part(&mut self) -> Input {
        let m = &self.imp;
        if m.seated {
            return Input::default();
        }
        match self.hand_part {
            PartHand::Idle if !m.held => {
                // The wrong way round: tap it over first (press and let go where it lies).
                self.hand_part = if m.sweep > 0.0 { PartHand::Tapped } else { PartHand::Carrying };
                Input::hold(m.x, m.y, true)
            }
            PartHand::Tapped => {
                self.hand_part = PartHand::Idle;
                Input::release(m.x, m.y)
            }
            _ => {
                let d = (VOL[0] - m.x).hypot(VOL[1] - m.y);
                if d > 2.0 {
                    let k = (8.0 / d).min(1.0);
                    return Input::hold(m.x + (VOL[0] - m.x) * k, m.y + (VOL[1] - m.y) * k, false);
                }
                self.hand_part = PartHand::Idle;
                Input::release(VOL[0], VOL[1])
            }
        }
    }

    fn hand_align(&mut self) -> Input {
        if self.done {
            return Input::default();
        }
        // One foot at a time: press its handle, slide it to where the foot belongs, let go.
        let k = match (0..2).find(|&k| (self.h[k] - self.want[k]).abs() > 0.05) {
            Some(k) => k,
            None => {
                return match self.grab {
                    Some(g) => Input::release(FOOT[g], TRACK_Y - self.want[g] * KNOB),
                    None => Input::default(),
                };
            }
        };
        if let Some(g) = self.grab.filter(|&g| g != k) {
            return Input::release(FOOT[g], TRACK_Y - self.h[g] * KNOB);
        }
        let [hx, hy] = self.handle(k);
        if self.grab.is_none() {
            return Input::hold(hx, hy, true);
        }
        let goal = TRACK_Y - self.want[k] * KNOB;
        let y = hy + (goal - hy).clamp(-6.0, 6.0);
        Input::hold(hx, y, false)
    }

    fn hand_start(&mut self) -> Input {
        if self.done {
            return match self.grab {
                Some(_) => Input::release(LEVER[0], lever_y(self.v)),
                None => Input::default(),
            };
        }
        // Raise the speed only while the needle has room over the red; ease off towards the band.
        let room = self.p - (self.red_frac + 0.08);
        let up = (room * 1.5).clamp(0.0, 0.35) * (1.0 / 60.0);
        self.hand_v = (self.v + up).min(1.0);
        let y = lever_y(self.hand_v);
        if self.grab.is_none() {
            return Input::hold(LEVER[0], lever_y(self.v), true);
        }
        Input::hold(LEVER[0], y, false)
    }
}

// ------------------------------------------------------------------------------------------------ drawing

/// A straight pipe with square ends (the mockups' `cap: "butt"`): casing, body, bore and the fluid in it.
fn butt_pipe(g: &Pen, a: [f32; 2], b: [f32; 2], w: f32, fluid: Color32) {
    for (lw, col) in [(w + 6.0, hex(0x232c3b)), (w, hex(0x3a4658)), (w * 0.6, hex(0x141b27)), (w * 0.6, fluid)] {
        g.line(a[0], a[1], b[0], b[1], lw, col);
    }
}

/// An impeller: a hub and six vanes curving `sweep` (-1 backward for the clockwise casing), turned `a`.
fn impeller(g: &Pen, x: f32, y: f32, r: f32, a: f32, sweep: f32) {
    g.disc(x, y, r, hex(0x1b2433));
    g.ring(x, y, r, hex(0x4a5566), 3.0);
    let w = (r * 0.09).max(3.0);
    for k in 0..6 {
        let b = a + k as f32 * PI / 3.0;
        let pts: Vec<[f32; 2]> = (0..=8)
            .map(|j| {
                let u = j as f32 / 8.0;
                let (rr, th) = (r * (0.28 + 0.66 * u), b + sweep * 1.1 * u);
                [x + th.cos() * rr, y + th.sin() * rr]
            })
            .collect();
        g.path_round(&pts, w, hex(0xc9d3e0));
    }
    g.disc(x, y, r * 0.24, hex(0x7d8796));
    g.disc(x, y, r * 0.09, hex(0x2a313c));
}

impl Pump {
    fn volute(&self, g: &Pen, open: bool) {
        // The suction pipe in from the left, the discharge up from the top of the scroll; coolant stands in them,
        // brighter while the pump moves it; a flange where each meets the casing.
        let [vx, vy, vr] = VOL;
        let water = if self.phase == Phase::Start && self.v > 0.05 { hex(0x4fa8f7) } else { hex(0x2a5a86) };
        butt_pipe(g, [40.0, vy + 30.0], [vx - vr + 10.0, vy + 30.0], 40.0, water);
        butt_pipe(g, [vx + 20.0, vy - vr + 10.0], [vx + 20.0, 90.0], 40.0, water);
        g.rect(vx - vr - 6.0, vy + 30.0 - 32.0, 14.0, 64.0, hex(0x5a6a82));
        g.rect(vx + 20.0 - 32.0, vy - vr - 6.0, 64.0, 14.0, hex(0x5a6a82));
        // The scroll: a spiral, wider towards the discharge.
        let scroll: Vec<[f32; 2]> = (0..=40)
            .map(|j| {
                let u = j as f32 / 40.0;
                let (th, rr) = (-FRAC_PI_2 + u * TAU, vr * (0.82 + 0.18 * u));
                [vx + th.cos() * rr, vy + th.sin() * rr]
            })
            .collect();
        g.poly(&scroll, hex(0x2a3446));
        g.path(&scroll, true, 4.0, hex(0x4a586d));
        // The rotation arrow on the casing: clockwise.
        g.turn_arrow(vx, vy, vr - 14.0, -2.4, -1.0, c::AMBER);
        g.disc(vx, vy, WIN_R + 6.0, hex(0x3a4658));
        g.disc(vx, vy, WIN_R, if open { hex(0x070b12) } else { hex(0x0d1520) });
    }

    fn machine(&self, g: &Pen, t: f32) {
        let [vx, vy, vr] = VOL;
        let live = self.phase != Phase::Part;
        self.volute(g, self.phase == Phase::Part);
        if live {
            impeller(g, vx, vy, WIN_R - 6.0, self.spin, -1.0);
        } else if self.imp.seated {
            impeller(g, vx, vy, WIN_R - 6.0, 0.0, -1.0);
        } else {
            g.dashed(&Pen::arc_points(vx, vy, WIN_R - 6.0, 0.0, TAU), 3.0, c::AMBER, 8.0, 6.0);
            g.disc(vx, vy, 12.0, hex(0x7d8796));
        }
        if self.phase == Phase::Start && self.bubbles > 0.0 {
            for j in 0..14 {
                let a = j as f32 * 2.4 + t * 3.0;
                let rr = 14.0 + ((j * 7) % 40) as f32;
                g.ring(
                    vx + a.cos() * rr,
                    vy + a.sin() * rr,
                    3.0 + (j % 3) as f32,
                    rgba(232, 246, 255, 0.8 * self.bubbles),
                    2.0,
                );
            }
        }
        // The skid and the pump's pedestal.
        g.round(90.0, BASE_Y, 870.0, 26.0, 6.0, Some(hex(0x2a3446)), None);
        g.round(vx - 70.0, vy + vr - 8.0, 140.0, BASE_Y - vy - vr + 10.0, 6.0, Some(hex(0x222c3a)), None);
        // The pump's shaft to its coupling half.
        g.rect(vx + WIN_R + 6.0, SHAFT_Y - 9.0, CPL_X - vx - WIN_R - 26.0, 18.0, hex(0x9aa6b6));
        g.round(CPL_X - 34.0, SHAFT_Y - 34.0, 34.0, 68.0, 6.0, Some(hex(0x5a6a82)), None);
        // The motor, tilted and lifted by its feet.
        let [mx0, mx1, mh] = MOTOR;
        let (e0, e1) = (self.motor_y(mx0), self.motor_y(mx1));
        let a = (e1 - e0).atan2(mx1 - mx0);
        let m = g.translate(mx0, e0).rotate(a);
        m.round(-36.0, -34.0, 34.0, 68.0, 6.0, Some(hex(0x5a6a82)), None);
        m.rect(-6.0, -9.0, 12.0, 18.0, hex(0x9aa6b6));
        m.round(0.0, -mh / 2.0, mx1 - mx0, mh, 16.0, Some(hex(0x33405a)), Some((3.0, hex(0x5a6a82))));
        let mut x = 30.0;
        while x < mx1 - mx0 - 20.0 {
            m.rect(x, -mh / 2.0 + 8.0, 10.0, mh - 16.0, hex(0x283247));
            x += 22.0;
        }
        // The feet, the shims under them, and the handles that move them.
        for (k, fx) in FOOT.iter().copied().enumerate() {
            let fy = self.motor_y(fx) + mh / 2.0;
            let sel = self.phase == Phase::Align && if self.keys { self.sel == k } else { self.grab == Some(k) };
            g.rect(fx - 40.0, fy - 4.0, 80.0, 18.0, hex(0x4a586d));
            let shim = BASE_Y - (fy + 14.0);
            if shim > 0.0 {
                g.rect(fx - 34.0, fy + 14.0, 68.0, shim, c::COPPER);
            }
            if self.phase != Phase::Align {
                continue;
            }
            let ky = TRACK_Y - self.h[k] * KNOB;
            g.round(fx - 9.0, TRACK_Y - TRACK_HALF, 18.0, TRACK_HALF * 2.0, 9.0, Some(hex(0x141c28)), None);
            g.round(
                fx - 46.0,
                ky - 22.0,
                92.0,
                44.0,
                12.0,
                Some(if sel { hex(0x2a3a52) } else { hex(0x1f2a3a) }),
                Some((3.0, if sel { c::AMBER } else { hex(0x5a6a82) })),
            );
            g.path(&[[fx - 10.0, ky - 6.0], [fx, ky - 14.0], [fx + 10.0, ky - 6.0]], false, 3.0, c::FG);
            g.path(&[[fx - 10.0, ky + 6.0], [fx, ky + 14.0], [fx + 10.0, ky + 6.0]], false, 3.0, c::FG);
        }
    }

    fn targets(&self, g: &Pen) {
        let d = self.dots();
        let rr = self.ring_px;
        // The beam from the pump's coupling to the targets.
        g.dashed(&[[CPL_X, SHAFT_Y], TGT[1]], 2.0, rgba(255, 71, 87, 0.35), 6.0, 6.0);
        for (k, [tx, ty]) in TGT.iter().copied().enumerate() {
            g.disc(tx, ty, PLATE_R + 6.0, c::PANEL);
            g.disc(tx, ty, PLATE_R, hex(0x1a2230));
            g.ring(tx, ty, PLATE_R, if self.off[k] > 0.0 { c::DANGER } else { hex(0x3a4658) }, 3.0);
            g.line(tx - PLATE_R, ty, tx + PLATE_R, ty, 1.5, hex(0x3a4658));
            g.line(tx, ty - PLATE_R, tx, ty + PLATE_R, 1.5, hex(0x3a4658));
            let in_ring = d[k].abs() <= rr;
            g.ring(tx, ty, rr, if in_ring { c::OK } else { rgba(232, 238, 246, 0.6) }, 3.0);
            let dy = d[k].clamp(-PLATE_R - 14.0, PLATE_R + 14.0);
            g.disc(tx, ty - dy, 11.0, rgba(255, 71, 87, 0.35));
            g.disc(tx, ty - dy, 6.0, c::DANGER);
            g.text(if k == 1 { "FAR" } else { "NEAR" }, tx, ty + PLATE_R + 26.0, 18.0, c::DIM, Align::Center);
        }
        if self.hold > 0.0 {
            g.arc(1085.0, 120.0, 16.0, -FRAC_PI_2, -FRAC_PI_2 + TAU * self.hold / HOLD_S, 5.0, c::OK);
        }
    }

    fn start_panel(&self, g: &Pen) {
        // The suction gauge: green above the red band, red (hatched) at the bottom of its sweep.
        let [gx, gy, gr] = GAUGE;
        let red = self.red_frac;
        let ang = |v: f32| A0 + SWEEP * v.clamp(0.0, 1.0);
        g.disc(gx, gy, gr + 10.0, c::PANEL);
        g.disc(gx, gy, gr, hex(0x141c28));
        g.arc(gx, gy, gr - 16.0, ang(red), ang(1.0), 16.0, rgba(61, 220, 132, 0.45));
        g.hatch_arc(gx, gy, gr - 24.0, gr - 8.0, ang(0.0), ang(red), c::DANGER);
        let na = ang(self.p);
        let needle = if self.p < red { c::DANGER } else { c::FG };
        g.path_round(&[[gx, gy], [gx + na.cos() * (gr - 22.0), gy + na.sin() * (gr - 22.0)]], 5.0, needle);
        g.disc(gx, gy, 10.0, hex(0x5a6a82));
        g.text("SUCTION", gx, gy + gr + 30.0, 20.0, c::DIM, Align::Center);
        // The speed lever: the biggest control. The band at 100% is green.
        let [lx, ly0, ly1, lw] = LEVER;
        g.round(lx - 18.0, ly1 - 10.0, 36.0, ly0 - ly1 + 20.0, 18.0, Some(hex(0x141c28)), None);
        let (b0, b1) = (lever_y(BAND[1]), lever_y(BAND[0]));
        g.rect(lx - lw / 2.0 - 14.0, b0, lw + 28.0, b1 - b0, rgba(61, 220, 132, 0.35));
        g.rect_stroke(lx - lw / 2.0 - 14.0, b0, lw + 28.0, b1 - b0, 2.0, c::OK);
        let ky = lever_y(self.v);
        let on = self.grab.is_some() || self.keys;
        g.round(
            lx - lw / 2.0,
            ky - 30.0,
            lw,
            60.0,
            14.0,
            Some(if on { hex(0x2a3a52) } else { hex(0x243045) }),
            Some((3.0, if on { c::AMBER } else { hex(0x7d8796) })),
        );
        g.line(lx - 26.0, ky, lx + 26.0, ky, 4.0, c::FG);
        g.text("SPEED", lx, ly0 + 40.0, 20.0, c::DIM, Align::Center);
        if self.hold > 0.0 {
            g.arc(lx, ly1 - 40.0, 16.0, -FRAC_PI_2, -FRAC_PI_2 + TAU * self.hold / HOLD_RUN_S, 5.0, c::OK);
        }
    }

    fn part_panel(&self, g: &Pen) {
        let [x, y, w, h] = CRATE;
        g.panel(x, y, w, h, 12.0, hex(0x141c28), hex(0x3a4658));
        let m = &self.imp;
        if m.seated {
            return;
        }
        impeller(g, m.x, m.y, IMP_R, 0.0, m.sweep);
        // Tap to turn it over: a mirror mark under it (a line with a head at each end), not a turning arrow.
        if !m.held {
            let (cx, yy) = (x + w / 2.0, y + h - 16.0);
            let col = rgba(232, 238, 246, 0.6);
            g.line(cx - 34.0, yy, cx + 34.0, yy, 3.0, col);
            for d in [-1.0f32, 1.0] {
                g.poly(&[[cx + d * 44.0, yy], [cx + d * 32.0, yy - 7.0], [cx + d * 32.0, yy + 7.0]], col);
            }
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
        plays_to_end("pump");
    }

    #[test]
    fn both_feet_run_to_their_stops_knock_the_coupling() {
        // W held on the front foot, Tab, W held on the rear one: the motor lifts until a dot leaves its plate and
        // stays off. If both feet belong high, the same run downwards (S) does it.
        let mut n = 0u32;
        fumble_check("pump", move |_r| {
            n += 1;
            let k = n % 600;
            let key = if n < 600 { Key::W } else { Key::S };
            if k == 150 || k == 300 {
                Input::default().key(Key::Tab, true)
            } else {
                Input::default().key(key, false)
            }
        });
    }
}
