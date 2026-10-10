//! The coolant balance, the reactor operator's duty (reactor-cooling design 2, 3 and 5; the loop's numbers are
//! power-grid 10), from `docs/mockups/repairs/coolant.js`.
//!
//! Played here as a job so it can be tried alone, one reactor state a round, each harder than the last: cruise, a
//! combat surge to 100%, a core pump lost (a damaged job); a leak, overdrive at 120%, a radiator pump down, a surge (a
//! longer job, after its part step fits core pump B's new cartridge).
//!
//! The picture: the loop as a ring, the core at the top and the chiller at the bottom, the cold leg up the left side
//! (blue, dots) and the hot leg down the right (red to orange with its temperature, chevrons), the two tanks and the
//! loop's inventory at the left, the radiators beyond the chiller. Two gauges: the legs' temperatures (two needles
//! against their bands, the warning and the scram), and the core's heat against what the flow carries at the design's
//! 15 K rise. Four levers: core pump speed (the biggest), the chiller valve (how much of the hot leg goes through the
//! exchanger), the radiator pumps' speed, and the makeup valve from the tanks. Exact values while a lever is held, on
//! hover or on the keys' focus.
//!
//! The goal, whatever the reactor does: the hot leg 335-350 K, the cold leg 320-335 K and the loop over 80% full. Held
//! in band, the hold ring round the core fills. The hot leg past 370 K is the fumble "Loop over-temperature warning";
//! past 380 K for 2 s the reactor scrams (a second fumble) and restarts.
//!
//! The physics (one lumped loop, as the mockup's): heat in is 40 MW at full throttle; the hot leg follows the cold leg
//! plus heat / (flow x 3.6 kJ/(kg K)); the radiators reject 40 MW x chiller valve x radiator pumps x
//! (T_hot^4 - 4^4) / (330^4 - 4^4); the cold leg moves by (heat in - rejected) / (3.6 kJ/(kg K) x the loop's kg). The
//! loop runs `TIME_X` ship seconds a played second so a round's drift shows; the console's own panel runs at 1. These
//! constants are the loop's physics (reactor-cooling 2-3), the same at every level, so they live here beside the
//! picture; what a level changes is the reactor's state and the hold, which are knobs.
//!
//! Keys: Left and Right pick a lever, Up and Down move it.

use std::f32::consts::{PI, TAU};

use egui::{Color32, Key};

use crate::kit::{nearest, Ctx, Game, Input, BAR_H, W};
use crate::pen::{c, hex, rgba, Align, Pen};

/// In the engine.
pub const PORTED: bool = true;

/// A fresh game.
pub fn new() -> Box<dyn Game> {
    Box::new(Coolant::default())
}

const WARN_TEXT: &str = "Loop over-temperature warning";
const SCRAM_TEXT: &str = "Scram: the loop tripped the reactor";

// ---------------------------------------------------------------- the loop (reactor-cooling 2-3, power-grid 2, 10)
/// MJ/(kg K): water and glycol.
const CP: f64 = 3.6e-3;
/// Kg in the loop when full (40 MJ/K over CP).
const M_FULL: f64 = 11100.0;
/// Kg of reserve in each tank.
const TANK_KG: f64 = 2000.0;
/// Kg/s at full flow: two core pumps of half the flow each.
const FLOW_FULL: f64 = 740.0;
/// MW into the loop at 100% throttle: a 15 K rise at full flow.
const HEAT_FULL_MW: f64 = 40.0;
/// MW the radiators reject at `T_RAD`, full health and radiator pumps.
const RAD_MW: f64 = 40.0;
const T_RAD_K: f64 = 330.0;
const T_SPACE_K: f64 = 4.0;
/// Kg/s from a segment at 0% integrity.
const LEAK_KG_S: f64 = 20.0;
/// % integrity under which a segment leaks.
const LEAK_FROM_PCT: f64 = 75.0;
/// Kg/s through the makeup valve, full open.
const MAKEUP_KG_S: f64 = 10.0;
/// Inventory: no flow at 60%, full flow at 80% and over (cavitation).
const CAV_LO: f64 = 0.6;
const CAV_HI: f64 = 0.8;
/// Natural circulation with the pumps stopped (reactor-cooling 6).
const NATURAL: f64 = 0.05;
/// Pump speed at overdrive (reactor-cooling 6: 0-120%).
const SPEED_MAX: f64 = 1.2;
/// Throttle a ship second, up and down (power-grid 2).
const RAMP_UP: f64 = 0.02;
const RAMP_DOWN: f64 = 0.10;
/// Ship s: the hot leg follows the core outlet with this lag.
const HOT_TAU_S: f64 = 6.0;
/// The rise at full flow and full heat: the "carries" needle's scale.
const RISE_DESIGN_K: f64 = 15.0;
const HOT_BAND: [f64; 2] = [335.0, 350.0];
const COLD_BAND: [f64; 2] = [320.0, 335.0];
const INV_MIN: f64 = 0.8;
const WARN_K: f64 = 370.0;
const WARN_REARM_K: f64 = 362.0;
const SCRAM_K: f64 = 380.0;
/// Ship s over the scram line that trips it.
const SCRAM_HOLD_S: f64 = 2.0;
/// Played s the reactor sits at 10% after a scram before it ramps back.
const SCRAM_RESTART_S: f64 = 3.0;
/// Ship s a played s.
const TIME_X: f64 = 6.0;
/// Played s: the loop steps on a fixed tick.
const TICK: f64 = 1.0 / 60.0;
/// What a fumble knocks off the hold, s.
const FUMBLE_HOLD_S: f32 = 1.5;
/// A lever's travel a second from the keys.
const LEVER_RATE: f32 = 0.6;

/// A reactor state: its word, its throttle (0-1.2), pump B's and radiator pump B's capability, the leaking segment's
/// integrity (%).
#[derive(Clone, Copy, Debug)]
struct St {
    name: &'static str,
    throttle: f64,
    pump_b: f64,
    rad_b: f64,
    seg: Option<f64>,
}

/// The reactor's states, by the index the knobs name: cruise, surge, overdrive, pump B lost, a leak, a radiator pump
/// down.
const STATES: [St; 6] = [
    St { name: "CRUISE", throttle: 0.6, pump_b: 1.0, rad_b: 1.0, seg: None },
    St { name: "SURGE", throttle: 1.0, pump_b: 1.0, rad_b: 1.0, seg: None },
    St { name: "OVERDRIVE", throttle: 1.2, pump_b: 1.0, rad_b: 1.0, seg: None },
    St { name: "PUMP B LOST", throttle: 0.85, pump_b: 0.0, rad_b: 1.0, seg: None },
    St { name: "LEAK", throttle: 0.6, pump_b: 1.0, rad_b: 1.0, seg: Some(50.0) },
    St { name: "RAD PUMP DOWN", throttle: 0.7, pump_b: 1.0, rad_b: 0.0, seg: None },
];
const CRUISE: usize = 0;
const LEAK: usize = 4;

// ---------------------------------------------------------------- the screen (canvas px)
const RING: [f32; 3] = [380.0, 400.0, 186.0];
const CORE: [f32; 3] = [RING[0], RING[1] - RING[2], 48.0];
const CHILL: [f32; 4] = [RING[0], RING[1] + RING[2], 132.0, 58.0];
const RADS: [f32; 4] = [236.0, 654.0, 288.0, 50.0];
const TANKS: [[f32; 4]; 2] = [[34.0, 300.0, 40.0, 190.0], [84.0, 300.0, 40.0, 190.0]];
const LOOPBAR: [f32; 4] = [34.0, 528.0, 90.0, 26.0];
/// The core pumps on the cold leg, upper left, degrees round the ring.
const PUMPS_DEG: [f32; 2] = [205.0, 228.0];
/// The leaking segment, on the hot leg.
const LEAK_A: f32 = 25.0 * PI / 180.0;
const TDIAL: [f32; 3] = [768.0, 256.0, 104.0];
const HDIAL: [f32; 3] = [768.0, 548.0, 92.0];
/// The temperature dial's scale, K.
const T_LO: f64 = 300.0;
const T_HI: f64 = 390.0;
/// The heat dial's scale, MW.
const H_HI: f64 = 60.0;
const DIAL_A0: f32 = PI * 0.8;
const DIAL_A1: f32 = PI * 2.2;

/// A lever: what it sets, its word, its rectangle, its top.
#[derive(Clone, Copy, Debug)]
struct Lever {
    key: usize,
    word: &'static str,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    max: f32,
}

const PUMP: usize = 0;
const CHL: usize = 1;
const RAD: usize = 2;
const MAKEUP: usize = 3;
const LEVERS: [Lever; 4] = [
    Lever { key: PUMP, word: "PUMPS", x: 916.0, y: 168.0, w: 88.0, h: 392.0, max: SPEED_MAX as f32 },
    Lever { key: CHL, word: "CHILLER", x: 1030.0, y: 168.0, w: 60.0, h: 392.0, max: 1.0 },
    Lever { key: RAD, word: "RADS", x: 1112.0, y: 168.0, w: 60.0, h: 392.0, max: SPEED_MAX as f32 },
    Lever { key: MAKEUP, word: "MAKEUP", x: 1194.0, y: 168.0, w: 60.0, h: 392.0, max: 1.0 },
];
/// Px round each lever a finger still finds.
const HIT: f32 = 26.0;
/// The part step's crate.
const CRATE: [f32; 4] = [30.0, 596.0, 170.0, 108.0];
const COLD: Color32 = hex(0x4fa8f7);

/// The loop's state.
#[derive(Clone, Debug, Default)]
struct Loop {
    tc: f64,
    th: f64,
    inv: f64,
    tanks: [f64; 2],
    throttle: f64,
    /// Pump, chiller, radiators, makeup.
    ctrl: [f64; 4],
    scram_t: f64,
    restart_t: f64,
    armed: bool,
    acc: f64,
}

/// The loop's numbers now.
#[derive(Clone, Copy, Debug, Default)]
struct Derived {
    q: f64,
    flow: f64,
    rej: f64,
    leak: f64,
    carry: f64,
    rise: f64,
}

fn fresh_loop(kind: usize) -> Loop {
    let leak = kind == LEAK;
    Loop {
        tc: 338.0,
        th: 347.0,
        inv: if leak { 0.84 } else { 0.97 },
        tanks: if leak { [1500.0, 1400.0] } else { [TANK_KG, TANK_KG] },
        throttle: STATES[kind].throttle,
        ctrl: [1.0, 0.45, 1.0, 0.0],
        scram_t: 0.0,
        restart_t: 0.0,
        armed: true,
        acc: 0.0,
    }
}

/// The radiators' rejection per unit of chiller and radiator pumps at `th`, MW.
fn rej_per(th: f64) -> f64 {
    RAD_MW * (th.powi(4) - T_SPACE_K.powi(4)) / (T_RAD_K.powi(4) - T_SPACE_K.powi(4))
}

fn derive(l: &Loop, st: &St) -> Derived {
    let q = HEAT_FULL_MW * l.throttle;
    let pumps = l.ctrl[PUMP] * (1.0 + st.pump_b) / 2.0;
    let cav = ((l.inv - CAV_LO) / (CAV_HI - CAV_LO)).clamp(0.0, 1.0);
    let flow = (pumps * cav).max(NATURAL);
    let rad_p = l.ctrl[RAD] * (1.0 + st.rad_b) / 2.0;
    let rej = l.ctrl[CHL] * rad_p * rej_per(l.th);
    let leak = match st.seg {
        Some(seg) if seg < LEAK_FROM_PCT => (LEAK_KG_S * (LEAK_FROM_PCT - seg)) / LEAK_FROM_PCT,
        _ => 0.0,
    };
    let carry = flow * FLOW_FULL * CP * RISE_DESIGN_K;
    Derived { q, flow, rej, leak, carry, rise: q / (flow * FLOW_FULL * CP) }
}

fn in_band(l: &Loop) -> bool {
    l.th >= HOT_BAND[0] && l.th <= HOT_BAND[1] && l.tc >= COLD_BAND[0] && l.tc <= COLD_BAND[1] && l.inv >= INV_MIN
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Ph {
    #[default]
    Play,
    Held,
    Part,
    Fitted,
}

/// The new pump cartridge.
#[derive(Clone, Copy, Debug, Default)]
struct Cart {
    x: f32,
    y: f32,
    held: bool,
    pad: bool,
    set: bool,
    dx: f32,
    dy: f32,
}

/// The game's state.
#[derive(Default)]
pub struct Coolant {
    sim: Option<Loop>,
    kind: usize,
    part: bool,
    phase: Ph,
    hold: f32,
    hold_need: f32,
    grab: Option<usize>,
    focus: usize,
    keys: bool,
    flash: f32,
    cart: Cart,
    last_round: Option<u32>,
    last_part: bool,
    /// The hand: frames played, and whether it carries the cartridge.
    hand_n: u32,
    hand_carry: bool,
}

fn ring_pt(a: f32, r: f32) -> [f32; 2] {
    [RING[0] + a.cos() * r, RING[1] + a.sin() * r]
}

fn pump_at(i: usize) -> [f32; 2] {
    ring_pt(PUMPS_DEG[i] * PI / 180.0, RING[2])
}

fn lerp(a: f32, b: f32, u: f32) -> f32 {
    a + (b - a) * u
}

/// The hot leg's colour at a temperature: red at its band's top and over, orange through it, amber below.
fn hot_col(t: f64) -> Color32 {
    let u = ((t as f32 - 330.0) / 30.0).clamp(0.0, 1.0);
    Color32::from_rgb(
        lerp(242.0, 255.0, u).round() as u8,
        lerp(160.0, 71.0, u).round() as u8,
        lerp(70.0, 87.0, u).round() as u8,
    )
}

fn lever_y(l: &Lever, v: f32) -> f32 {
    l.y + l.h - 20.0 - (v / l.max) * (l.h - 40.0)
}

fn value_at(l: &Lever, y: f32) -> f32 {
    (((l.y + l.h - 20.0 - y) / (l.h - 40.0)) * l.max).clamp(0.0, l.max)
}

fn lever_at(x: f32, y: f32) -> Option<usize> {
    let pts: Vec<Option<[f32; 2]>> = LEVERS
        .iter()
        .map(|l| {
            (x >= l.x - HIT && x <= l.x + l.w + HIT && y >= l.y - HIT && y <= l.y + l.h + HIT)
                .then_some([l.x + l.w / 2.0, y.clamp(l.y, l.y + l.h)])
        })
        .collect();
    nearest(&pts, x, y, 200.0)
}

/// A quadratic Bezier's points, `n` steps.
fn quad_pts(p0: [f32; 2], c1: [f32; 2], p1: [f32; 2], n: usize) -> Vec<[f32; 2]> {
    (0..=n)
        .map(|i| {
            let u = i as f32 / n as f32;
            let v = 1.0 - u;
            [v * v * p0[0] + 2.0 * v * u * c1[0] + u * u * p1[0], v * v * p0[1] + 2.0 * v * u * c1[1] + u * u * p1[1]]
        })
        .collect()
}

/// An arc with round ends (canvas's `lineCap = "round"`).
#[allow(clippy::too_many_arguments)]
fn arc_round(g: &Pen, x: f32, y: f32, r: f32, a0: f32, a1: f32, width: f32, col: Color32) {
    if (a1 - a0).abs() < 1e-4 {
        return;
    }
    g.arc(x, y, r, a0, a1, width, col);
    for a in [a0, a1] {
        g.disc(x + a.cos() * r, y + a.sin() * r, width / 2.0, col);
    }
}

fn dial_a(u: f32) -> f32 {
    lerp(DIAL_A0, DIAL_A1, u.clamp(0.0, 1.0))
}

impl Coolant {
    fn st(&self) -> &'static St {
        &STATES[self.kind.min(STATES.len() - 1)]
    }

    fn start(&mut self, cx: &Ctx) {
        let round = cx.round;
        // The same balance each round against a harder state (repair-minigames 1a); a longer job's are harder still.
        let knob = if cx.rounds >= 4 { "disabled_state_index" } else { "damaged_state_index" };
        self.kind = (cx.knob(knob).round().max(0.0) as usize).min(STATES.len() - 1);
        // A restarted round (three fumbles) restarts the loop too: the reactor came back from a scram. The part move
        // hands on to its round at the same index: that is not a restart.
        let restart = self.last_round == Some(round) && !cx.part && !self.last_part;
        if self.sim.is_none() || restart {
            self.sim = Some(fresh_loop(self.kind));
        }
        if restart {
            if let Some(l) = &mut self.sim {
                l.tc = 330.0;
                l.th = 338.0;
                l.throttle = 0.1;
                l.restart_t = SCRAM_RESTART_S;
            }
        }
        self.last_round = Some(round);
        self.last_part = cx.part;
        self.part = cx.part;
        self.phase = if cx.part { Ph::Part } else { Ph::Play };
        self.hold = 0.0;
        self.hold_need = if cx.part { 0.0 } else { cx.knob("hold_s") };
        self.grab = None;
        self.focus = 0;
        self.keys = false;
        self.flash = 0.0;
        self.cart =
            Cart { x: CRATE[0] + CRATE[2] / 2.0, y: CRATE[1] + CRATE[3] / 2.0 + 4.0, set: !cx.part, ..Cart::default() };
        self.hand_n = 0;
        self.hand_carry = false;
    }

    fn physics(&mut self, cx: &mut Ctx, dt: f64) {
        let st = *self.st();
        let Some(l) = &mut self.sim else { return };
        let ds = dt * TIME_X;
        // The reactor: its throttle ramps to the state's (power-grid 2), or sits at 10% while it restarts.
        l.restart_t = (l.restart_t - dt).max(0.0);
        let want = if l.restart_t > 0.0 { 0.1 } else { st.throttle };
        l.throttle =
            if want > l.throttle { want.min(l.throttle + RAMP_UP * ds) } else { want.max(l.throttle - RAMP_DOWN * ds) };
        let d = derive(l, &st);
        // The hot leg follows the core outlet; the cold leg is the loop's heat balance over its heat capacity.
        l.th += (l.tc + d.rise - l.th) * (ds / HOT_TAU_S).min(1.0);
        l.tc += ((d.q - d.rej) / (CP * l.inv * M_FULL)) * ds;
        // Inventory: the leak out, the makeup in from the tanks (both evenly), never over full.
        let mut m = l.inv * M_FULL - d.leak * ds;
        let room = (M_FULL - m).max(0.0);
        let have = l.tanks[0] + l.tanks[1];
        let add = (MAKEUP_KG_S * l.ctrl[MAKEUP] * ds).min(room).min(have);
        if add > 0.0 {
            let k = add / have;
            for t in &mut l.tanks {
                *t -= *t * k;
            }
            m += add;
        }
        l.inv = (m / M_FULL).clamp(0.0, 1.0);
        // The warning (the fumble) and the scram.
        if l.th < WARN_REARM_K {
            l.armed = true;
        }
        let mut said = None;
        if l.th > WARN_K && l.armed {
            l.armed = false;
            said = Some(WARN_TEXT);
        }
        l.scram_t = if l.th > SCRAM_K { l.scram_t + ds } else { 0.0 };
        let scram = l.scram_t > SCRAM_HOLD_S && l.restart_t <= 0.0;
        if scram {
            l.scram_t = 0.0;
            l.restart_t = SCRAM_RESTART_S;
            l.throttle = 0.1;
        }
        if let Some(w) = said {
            self.hold = (self.hold - FUMBLE_HOLD_S).max(0.0);
            self.flash = 1.0;
            cx.fumble(w);
        }
        if scram {
            self.hold = (self.hold - FUMBLE_HOLD_S).max(0.0);
            cx.fumble(SCRAM_TEXT);
        }
    }

    fn play(&mut self, cx: &mut Ctx, dt: f32, input: &Input) {
        // The levers: a press picks the nearest, a drag sets it; the keys pick and move the focused one.
        let n = LEVERS.len();
        for (k, d) in [(Key::ArrowLeft, n - 1), (Key::A, n - 1), (Key::ArrowRight, 1), (Key::D, 1), (Key::Tab, 1)] {
            if input.hit(k) {
                self.keys = true;
                self.focus = (self.focus + d) % n;
            }
        }
        let Some(l) = &mut self.sim else { return };
        if input.stick[1] != 0.0 {
            self.keys = true;
            let lv = &LEVERS[self.focus];
            let v = l.ctrl[lv.key] as f32 - input.stick[1] * LEVER_RATE * lv.max * dt;
            l.ctrl[lv.key] = f64::from(v.clamp(0.0, lv.max));
        }
        if input.pressed {
            if let Some(i) = lever_at(input.x, input.y) {
                self.grab = Some(i);
                self.focus = i;
                self.keys = false;
            }
        }
        if let (Some(i), true) = (self.grab, input.down) {
            let lv = &LEVERS[i];
            l.ctrl[lv.key] = f64::from(value_at(lv, input.y));
        }
        if !input.down {
            self.grab = None;
        }
        // The loop, on a fixed tick.
        l.acc += f64::from(dt);
        while self.sim.as_ref().is_some_and(|l| l.acc >= TICK) {
            if let Some(l) = &mut self.sim {
                l.acc -= TICK;
            }
            self.physics(cx, TICK);
            if self.phase != Ph::Play && self.phase != Ph::Held {
                return;
            }
        }
        if self.phase == Ph::Play && self.sim.as_ref().is_some_and(in_band) {
            self.hold = (self.hold + dt).min(self.hold_need);
            if self.hold >= self.hold_need {
                self.phase = Ph::Held;
                cx.step_done();
            }
        }
    }

    fn part_step(&mut self, cx: &mut Ctx, dt: f32, input: &Input) {
        let slot = pump_at(1);
        let p = &mut self.cart;
        if input.stick != [0.0, 0.0] {
            p.x += input.stick[0] * 480.0 * dt;
            p.y += input.stick[1] * 480.0 * dt;
            p.pad = true;
        }
        if input.pressed && input.dist(p.x, p.y) < 60.0 {
            p.held = true;
            p.dx = p.x - input.x;
            p.dy = p.y - input.y;
        }
        if p.held && input.down {
            p.x = input.x + p.dx;
            p.y = input.y + p.dy;
        }
        if (p.held && input.released) || (p.pad && input.action_pressed) {
            p.held = false;
            p.pad = false;
            if (p.x - slot[0]).hypot(p.y - slot[1]) < 60.0 {
                p.set = true;
                p.x = slot[0];
                p.y = slot[1];
                self.phase = Ph::Fitted;
                cx.step_done();
            }
        }
        p.x = p.x.clamp(30.0, W - 30.0);
        p.y = p.y.clamp(BAR_H + 30.0, 720.0 - 30.0);
    }

    fn draw_loop(&self, g: &Pen, t: f32, d: &Derived, st: &St) {
        let Some(l) = &self.sim else { return };
        let live = self.phase != Ph::Part;
        let [rx, _, rr] = RING;
        // The chiller's bypass: a chord over the exchanger; its width is the share that skips it.
        let by = CHILL[1] - 46.0;
        let (bx0, bx1) = (rx + 112.0, rx - 112.0);
        let chord = [ring_pt(PI / 2.0 - 0.62, rr), [bx0 - 30.0, by], [bx1 + 30.0, by], ring_pt(PI / 2.0 + 0.62, rr)];
        g.path_round(&chord, 14.0, hex(0x263142));
        let bysh = if live { 1.0 - l.ctrl[CHL] as f32 } else { 0.0 };
        if bysh > 0.02 {
            g.alpha(0.6).path_round(&chord, 3.0 + 8.0 * bysh, hot_col(l.th));
        }
        // The legs: hot down the right (core to chiller), cold up the left (chiller to core).
        let fl = if live { d.flow as f32 * 10.0 } else { 0.0 };
        leg_arc(g, -PI / 2.0 + 0.26, PI / 2.0 - 0.36, hot_col(l.th), true, fl, t, !live);
        leg_arc(g, PI / 2.0 + 0.36, PI * 1.5 - 0.26, COLD, false, fl, t, !live);
        // The leak: a cracked joint on the hot leg, dripping and steaming.
        if d.leak > 0.0 && live {
            let [x, y] = ring_pt(LEAK_A, rr + 14.0);
            g.path(
                &[[x - 10.0, y - 12.0], [x - 2.0, y - 2.0], [x - 8.0, y + 6.0], [x + 2.0, y + 14.0]],
                false,
                3.0,
                c::DANGER,
            );
            for j in 0..6 {
                let ph = (t * 1.6 + j as f32 / 6.0).rem_euclid(1.0);
                g.disc(
                    x + 12.0 + j as f32 * 3.0,
                    y + 10.0 + ph * 70.0,
                    4.0 - 2.0 * ph,
                    rgba(255, 170, 120, 0.9 - 0.7 * ph),
                );
            }
            for j in 0..4 {
                let ph = (t * 0.5 + j as f32 / 4.0).rem_euclid(1.0);
                g.disc(
                    x + 30.0 + (t + j as f32).sin() * 8.0,
                    y - ph * 60.0,
                    8.0 + ph * 14.0,
                    rgba(220, 230, 240, 0.12 * (1.0 - ph)),
                );
            }
        }
        // The makeup line: the tanks into the cold leg.
        let [mx, my] = ring_pt(PI, rr);
        let x0 = TANKS[1][0] + TANKS[1][2];
        g.line(x0, my, mx - 16.0, my, 12.0, hex(0x263142));
        if live && l.ctrl[MAKEUP] > 0.02 && l.inv < 1.0 && l.tanks[0] + l.tanks[1] > 1.0 {
            g.line(x0, my, mx - 16.0, my, 2.0 + 5.0 * l.ctrl[MAKEUP] as f32, COLD);
            for j in 0..3 {
                let ph = (t * 1.2 + j as f32 / 3.0).rem_euclid(1.0);
                g.disc(lerp(x0, mx - 16.0, ph), my, 3.5, hex(0xe8f6ff));
            }
        }
        // The core: its vessel, its heat as a glow, the throttle as an arc, the hold ring round it.
        let [kx, ky, kr] = CORE;
        let thr = (l.throttle / 1.2).clamp(0.0, 1.0) as f32;
        let glow = if live { 0.35 + 0.55 * thr } else { 0.1 };
        g.radial(kx, ky, kr * 1.8, rgba(190, 159, 230, glow * 0.6), rgba(190, 159, 230, 0.0));
        g.radial(kx, ky, kr * 1.8 * 0.4, rgba(255, 240, 220, glow), rgba(190, 159, 230, glow * 0.6));
        g.disc(kx, ky, kr, hex(0x141c28));
        g.ring(kx, ky, kr, hex(0x4a5568), 4.0);
        g.disc(kx, ky, kr * 0.55, rgba(240, 230, 255, 0.25 + glow * 0.7));
        if live {
            let col = if l.throttle > 1.001 { c::AMBER } else { hex(0xbe9fe6) };
            g.arc(kx, ky, kr - 9.0, PI * 0.75, PI * 0.75 + PI * 1.5 * thr, 5.0, col);
            // The hold: a ring round the core, green while every needle is in its band.
            let held = in_band(l);
            g.ring(kx, ky, kr + 20.0, hex(0x1d2636), 9.0);
            let k = if self.hold_need > 0.0 { self.hold / self.hold_need } else { 0.0 };
            let col = if self.phase == Ph::Held || held { c::OK } else { c::AMBER };
            arc_round(g, kx, ky, kr + 20.0, -PI / 2.0, -PI / 2.0 + TAU * k, 9.0, col);
        }
        // The pumps on the cold leg (B may be lost; on the part step its housing is open).
        for i in 0..2 {
            let [x, y] = pump_at(i);
            if self.phase == Ph::Part && i == 1 && !self.cart.set {
                g.disc(x, y, 30.0, hex(0x120d08));
                g.ring(x, y, 30.0, c::AMBER, 3.0);
                g.dashed(&Pen::arc_points(x, y, 40.0 + 3.0 * (t * 5.0).sin(), 0.0, TAU), 2.0, c::AMBER, 6.0, 5.0);
                continue;
            }
            let cap = if i == 0 { 1.0 } else { st.pump_b };
            pump_glyph(g, x, y, live, live && cap <= 0.0, t, l.ctrl[PUMP] as f32);
        }
        // The chiller: plates of the exchanger, cooler where the valve lets more through.
        let [chx, chy, chw, chh] = CHILL;
        g.panel(chx - chw / 2.0, chy - chh / 2.0, chw, chh, 10.0, hex(0x152030), hex(0x4a5a70));
        let plate = if live { rgba(79, 168, 247, 0.3 + 0.6 * l.ctrl[CHL] as f32) } else { hex(0x2a3446) };
        for k in 0..9 {
            let x = chx - chw / 2.0 + 14.0 + k as f32 * 13.0;
            g.path(&[[x, chy - 19.0], [x + 5.0, chy - 8.0], [x, chy + 3.0], [x + 5.0, chy + 14.0]], false, 4.0, plate);
        }
        // The radiator loop: down from the chiller to the fins; the fins glow with what they reject.
        let [rdx, rdy, _, rdh] = RADS;
        g.line(chx - 40.0, chy + chh / 2.0, chx - 40.0, rdy, 10.0, hex(0x263142));
        g.line(chx + 40.0, chy + chh / 2.0, chx + 40.0, rdy, 10.0, hex(0x263142));
        let rk = if live { (d.rej as f32 / 50.0).clamp(0.0, 1.0) } else { 0.0 };
        for k in 0..18 {
            let x = rdx + 6.0 + k as f32 * 16.0;
            let dead = st.rad_b <= 0.0 && k >= 9 && live;
            let fill = if dead {
                hex(0x1b2230)
            } else {
                Color32::from_rgb(
                    lerp(60.0, 255.0, rk).round() as u8,
                    lerp(70.0, 120.0, rk).round() as u8,
                    lerp(90.0, 80.0, rk).round() as u8,
                )
            };
            g.rect(x, rdy, 9.0, rdh, fill);
            if dead {
                g.hatch(x, rdy, 9.0, rdh, rgba(255, 71, 87, 0.45));
            }
        }
        for i in 0..2 {
            let (x, y) = (chx + if i == 1 { 40.0 } else { -40.0 }, rdy - 26.0);
            let dead = live && i == 1 && st.rad_b <= 0.0;
            g.disc(x, y, 15.0, hex(0x1b2433));
            g.ring(x, y, 15.0, if dead { c::DANGER } else { hex(0x8796aa) }, 2.5);
            let rot = if live && !dead { t * 6.0 * l.ctrl[RAD] as f32 } else { 0.3 };
            let blade = if dead { hex(0x5a3036) } else { hex(0xc9d3e0) };
            for k in 0..3 {
                let a = rot + k as f32 * TAU / 3.0;
                let (s, co) = a.sin_cos();
                g.line(x, y, x + 10.0 * co + 3.0 * s, y + 10.0 * s - 3.0 * co, 3.0, blade);
            }
            if dead {
                g.cross(x, y, 9.0, 3.0, c::DANGER);
            }
        }
        // The tanks and the loop's inventory: levels, the loop's 80% line, hatched under it.
        for (i, [tx, ty, tw, th]) in TANKS.iter().copied().enumerate() {
            g.round(tx, ty, tw, th, 14.0, Some(hex(0x121a25)), Some((2.0, hex(0x3a4658))));
            let k = (l.tanks[i] / TANK_KG) as f32;
            let h = (th - 8.0) * k;
            g.round(
                tx + 4.0,
                ty + th - 4.0 - h,
                tw - 8.0,
                h,
                10.0f32.min(h / 2.0),
                Some(rgba(79, 168, 247, 0.7)),
                None,
            );
        }
        let [lx, ly, lw, lh] = LOOPBAR;
        g.round(lx, ly, lw, lh, 8.0, Some(hex(0x121a25)), None);
        g.hatch(lx, ly, lw * INV_MIN as f32, lh, rgba(255, 71, 87, 0.28));
        let low = l.inv < INV_MIN;
        g.round(
            lx + 3.0,
            ly + 3.0,
            ((lw - 6.0) * l.inv as f32).max(6.0),
            lh - 6.0,
            6.0,
            Some(if low { c::DANGER } else { COLD }),
            None,
        );
        g.rect(lx + lw * INV_MIN as f32 - 1.0, ly - 5.0, 3.0, lh + 10.0, c::FG);
        g.text("LOOP", lx + lw / 2.0, ly + lh + 18.0, 16.0, c::DIM, Align::Center);
        g.text("TANKS", TANKS[0][0] + 45.0, TANKS[0][1] - 16.0, 16.0, c::DIM, Align::Center);
        // The state: one word or two.
        let word = if self.part { "PUMP B" } else { st.name };
        let col = if self.kind == CRUISE && !self.part { c::DIM } else { c::AMBER };
        g.text(word, 34.0, 112.0, 30.0, col, Align::Left);
    }

    fn draw_dials(&self, g: &Pen, d: &Derived) {
        let Some(l) = &self.sim else { return };
        let live = self.phase != Ph::Part;
        // The legs' temperatures: two needles, two bands, the warning and the scram hatched.
        let [tx, ty, tr] = TDIAL;
        let tu = |k: f64| ((k - T_LO) / (T_HI - T_LO)) as f32;
        g.panel(tx - tr - 26.0, ty - tr - 30.0, (tr + 26.0) * 2.0, tr * 2.0 + 74.0, 18.0, hex(0x0b1119), c::LINE);
        g.ring(tx, ty, tr, hex(0x1d2636), 14.0);
        band_arc(g, tx, ty, tr, tu(COLD_BAND[0]), tu(COLD_BAND[1]), rgba(79, 168, 247, 0.75), Band::Dots);
        band_arc(g, tx, ty, tr, tu(HOT_BAND[0]), tu(HOT_BAND[1]), rgba(255, 120, 90, 0.8), Band::Chev);
        band_arc(g, tx, ty, tr, tu(WARN_K), tu(SCRAM_K), rgba(242, 160, 70, 0.8), Band::Hatch);
        band_arc(g, tx, ty, tr, tu(SCRAM_K), 1.0, rgba(255, 71, 87, 0.95), Band::Hatch);
        let mut k = T_LO;
        while k <= T_HI {
            let a = dial_a(tu(k));
            let (r0, r1) = (tr - 22.0, tr - 14.0);
            g.line(tx + a.cos() * r0, ty + a.sin() * r0, tx + a.cos() * r1, ty + a.sin() * r1, 2.0, hex(0x4a5568));
            k += 10.0;
        }
        if live {
            needle(g, tx, ty, tr - 26.0, dial_a(tu(l.tc)), COLD, Tip::Dot);
            needle(g, tx, ty, tr - 18.0, dial_a(tu(l.th)), hot_col(l.th), Tip::Chev);
        }
        g.disc(tx, ty, 10.0, hex(0x8796aa));
        g.text("LEGS", tx, ty + tr + 26.0, 18.0, c::DIM, Align::Center);
        // The core's heat against what the flow carries at the design rise: two needles; the gap between hatched red
        // when the heat is ahead.
        let [hx, hy, hr] = HDIAL;
        let hu = |mw: f64| (mw / H_HI) as f32;
        g.panel(hx - hr - 38.0, hy - hr - 26.0, (hr + 38.0) * 2.0, hr * 2.0 + 68.0, 18.0, hex(0x0b1119), c::LINE);
        g.ring(hx, hy, hr, hex(0x1d2636), 14.0);
        if live {
            if d.q > d.carry {
                band_arc(g, hx, hy, hr, hu(d.carry), hu(d.q), rgba(255, 71, 87, 0.85), Band::Hatch);
            } else {
                g.arc(hx, hy, hr, dial_a(hu(d.q)), dial_a(hu(H_HI.min(d.carry))), 12.0, rgba(61, 220, 132, 0.35));
            }
            needle(g, hx, hy, hr - 18.0, dial_a(hu(H_HI.min(d.carry))), COLD, Tip::Dot);
            needle(g, hx, hy, hr - 12.0, dial_a(hu(d.q)), c::AMBER, Tip::Flame);
        }
        g.disc(hx, hy, 10.0, hex(0x8796aa));
        g.text("HEAT", hx - 30.0, hy + hr + 22.0, 18.0, c::AMBER, Align::Center);
        g.text("FLOW", hx + 30.0, hy + hr + 22.0, 18.0, COLD, Align::Center);
    }

    fn draw_levers(&self, g: &Pen, input: &Input) {
        let Some(l) = &self.sim else { return };
        let live = self.phase == Ph::Play || self.phase == Ph::Held;
        for (i, lv) in LEVERS.iter().enumerate() {
            let v = l.ctrl[lv.key] as f32;
            let on = self.grab == Some(i);
            let foc = self.keys && self.focus == i;
            g.round(
                lv.x,
                lv.y,
                lv.w,
                lv.h,
                14.0,
                Some(hex(0x121a25)),
                Some((2.0, if on || foc { c::AMBER } else { c::LINE })),
            );
            // Over 100%: the overdrive zone, hatched amber.
            if lv.max > 1.0 {
                let y1 = lever_y(lv, 1.0);
                g.hatch(lv.x + 4.0, lv.y + 4.0, lv.w - 8.0, y1 - lv.y - 4.0, rgba(242, 160, 70, 0.35));
                g.rect(lv.x + 4.0, y1 - 1.0, lv.w - 8.0, 3.0, rgba(232, 238, 246, 0.5));
            }
            let y = lever_y(lv, v);
            let bot = lv.y + lv.h - 20.0;
            let fill =
                if lv.key == MAKEUP || lv.key == CHL { rgba(79, 168, 247, 0.35) } else { rgba(190, 159, 230, 0.3) };
            g.rect(lv.x + 12.0, y, lv.w - 24.0, bot - y, fill);
            let handle = if live {
                if on {
                    c::AMBER
                } else {
                    hex(0xc9d3e0)
                }
            } else {
                hex(0x4a5568)
            };
            g.round(lv.x - 8.0, y - 16.0, lv.w + 16.0, 32.0, 10.0, Some(handle), None);
            g.rect(lv.x + 4.0, y - 2.0, lv.w - 8.0, 4.0, hex(0x1b2433));
            glyph(g, lv.key, lv.x + lv.w / 2.0, lv.y - 30.0, if live { c::FG } else { c::DIM });
            let size = if lv.w > 70.0 { 20.0 } else { 16.0 };
            g.text(
                lv.word,
                lv.x + lv.w / 2.0,
                lv.y + lv.h + 26.0,
                size,
                if on || foc { c::AMBER } else { c::DIM },
                Align::Center,
            );
        }
        // The exact values: while a lever is held, on the keys' focus, or on hover. One tooltip.
        let tip = if self.grab.is_some() {
            self.grab
        } else if self.keys {
            Some(self.focus)
        } else if !input.down && input.x >= 0.0 {
            lever_at(input.x, input.y)
        } else {
            None
        };
        let d = derive(l, self.st());
        if let (Some(i), true) = (tip, live) {
            let lv = &LEVERS[i];
            let v = l.ctrl[lv.key];
            let pct = (v * 100.0).round();
            let txt = match lv.key {
                PUMP => format!("{pct}%  {} kg/s", (d.flow * FLOW_FULL).round()),
                CHL => format!("{pct}% through  {:.1} MW", d.rej),
                RAD => format!("{pct}%  {:.1} MW", d.rej),
                _ => format!("{pct}%  {:.1} kg/s", MAKEUP_KG_S * v),
            };
            tooltip(g, &txt, lv.x + lv.w / 2.0, lever_y(lv, v as f32) - 30.0);
        }
        // Hover on the dials and the loop: the numbers behind the needles.
        if !input.down && self.grab.is_none() && !self.keys && live && input.x >= 0.0 {
            let (x, y) = (input.x, input.y);
            if (x - TDIAL[0]).hypot(y - TDIAL[1]) < TDIAL[2] {
                tooltip(g, &format!("Hot {:.1} K  Cold {:.1} K", l.th, l.tc), TDIAL[0], TDIAL[1] - TDIAL[2] - 6.0);
            } else if (x - HDIAL[0]).hypot(y - HDIAL[1]) < HDIAL[2] {
                let s = format!("Heat {:.1} MW  Flow carries {:.1} MW", d.q, d.carry);
                tooltip(g, &s, HDIAL[0], HDIAL[1] - HDIAL[2] - 6.0);
            } else if x < 150.0 && y > 280.0 && y < 580.0 {
                let s = format!(
                    "Loop {:.1}%  Tanks {} kg  Leak {:.1} kg/s",
                    l.inv * 100.0,
                    (l.tanks[0] + l.tanks[1]).round(),
                    d.leak
                );
                tooltip(g, &s, 300.0, 290.0);
            }
        }
    }
}

/// An arc of pipe: steel casing, the coolant, and a pattern running the flow's way (hot: chevrons; cold: dots).
#[allow(clippy::too_many_arguments)]
fn leg_arc(g: &Pen, a0: f32, a1: f32, col: Color32, hot: bool, flow: f32, t: f32, dim: bool) {
    let [rx, ry, rr] = RING;
    g.arc(rx, ry, rr, a0, a1, 34.0, hex(0x263142));
    if dim {
        g.arc(rx, ry, rr, a0, a1, 22.0, hex(0x1b2433));
        return;
    }
    g.alpha(0.55).arc(rx, ry, rr, a0, a1, 22.0, col);
    let dir = if a1 > a0 { 1.0 } else { -1.0 };
    let span = (a1 - a0).abs();
    let step = 0.16;
    let off = (t * flow * 0.5).rem_euclid(step);
    let mut a = off;
    while a < span {
        let th = a0 + dir * a;
        let [x, y] = ring_pt(th, rr);
        let p = g.translate(x, y).rotate(th + dir * PI / 2.0);
        if hot {
            p.path_round(&[[-5.0, -8.0], [4.0, 0.0], [-5.0, 8.0]], 3.5, hex(0xfff3e0));
        } else {
            p.disc(0.0, 0.0, 4.0, hex(0xe8f6ff));
        }
        a += step;
    }
}

#[allow(clippy::too_many_arguments)]
fn pump_glyph(g: &Pen, x: f32, y: f32, on: bool, dead: bool, t: f32, speed: f32) {
    g.disc(x, y, 25.0, hex(0x1b2433));
    g.ring(x, y, 25.0, if dead { c::DANGER } else { hex(0x8796aa) }, 3.0);
    let p = g.translate(x, y).rotate(if on && !dead { t * 8.0 * speed } else { 0.4 });
    let blade = quad_pts([0.0, 0.0], [7.0, -6.0], [15.0, -4.0], 8);
    for k in 0..4 {
        p.rotate(k as f32 * PI / 2.0).path(&blade, false, 4.0, if dead { hex(0x5a3036) } else { hex(0xc9d3e0) });
    }
    g.disc(x, y, 5.0, hex(0x8796aa));
    if dead {
        g.hatch(x - 18.0, y - 18.0, 36.0, 36.0, rgba(255, 71, 87, 0.5));
        g.cross(x, y, 12.0, 4.0, c::DANGER);
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Band {
    Hatch,
    Chev,
    Dots,
}

#[allow(clippy::too_many_arguments)]
fn band_arc(g: &Pen, cx: f32, cy: f32, r: f32, u0: f32, u1: f32, col: Color32, kind: Band) {
    let (a0, a1) = (dial_a(u0), dial_a(u1));
    if kind == Band::Hatch {
        g.hatch_arc(cx, cy, r - 10.0, r + 6.0, a0, a1, col);
        return;
    }
    g.arc(cx, cy, r, a0, a1, 12.0, col);
    let mut a = a0 + 0.06;
    while a < a1 - 0.03 {
        let (x, y) = (cx + a.cos() * r, cy + a.sin() * r);
        if kind == Band::Dots {
            g.disc(x, y, 2.5, hex(0x04131c));
        } else {
            g.translate(x, y).rotate(a + PI / 2.0).path(
                &[[-3.0, -4.0], [1.0, 0.0], [-3.0, 4.0]],
                false,
                2.0,
                hex(0x2a0b0e),
            );
        }
        a += 0.09;
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tip {
    Chev,
    Dot,
    Flame,
}

/// A needle from a dial's centre at angle a.
#[allow(clippy::too_many_arguments)]
fn needle(g: &Pen, cx: f32, cy: f32, r: f32, a: f32, col: Color32, tip: Tip) {
    let (x, y) = (cx + a.cos() * r, cy + a.sin() * r);
    g.path_round(&[[cx, cy], [x, y]], 5.0, col);
    let p = g.translate(x, y).rotate(a);
    match tip {
        Tip::Chev => p.path_round(&[[-14.0, -11.0], [2.0, 0.0], [-14.0, 11.0]], 5.0, col),
        Tip::Dot => p.disc(0.0, 0.0, 9.0, col),
        Tip::Flame => {
            let mut pts = quad_pts([4.0, 0.0], [-6.0, -12.0], [-16.0, 0.0], 8);
            pts.extend(quad_pts([-16.0, 0.0], [-6.0, 12.0], [4.0, 0.0], 8).into_iter().skip(1));
            pts.pop();
            p.poly(&pts, col);
        }
    }
}

/// A lever's picture above it.
fn glyph(g: &Pen, key: usize, x: f32, y: f32, col: Color32) {
    match key {
        PUMP => {
            g.ring(x, y, 14.0, col, 3.0);
            for k in 0..3 {
                let a = k as f32 * TAU / 3.0;
                g.line(x, y, x + a.cos() * 10.0, y + a.sin() * 10.0, 3.0, col);
            }
        }
        CHL => {
            for k in 0..4 {
                let dx = k as f32 * 8.0;
                g.path(
                    &[[x - 12.0 + dx, y - 12.0], [x - 8.0 + dx, y - 2.0], [x - 12.0 + dx, y + 8.0]],
                    false,
                    3.0,
                    col,
                );
            }
        }
        RAD => {
            for k in 0..4 {
                g.rect(x - 13.0 + k as f32 * 7.0, y - 12.0, 4.0, 24.0, col);
            }
        }
        _ => {
            // A drop: a point on top, round below.
            let mut pts = quad_pts([x, y - 14.0], [x + 11.0, y], [x + 8.0, y + 6.0], 6);
            pts.extend(Pen::arc_points(x, y + 5.0, 8.0, 0.1, PI - 0.1));
            pts.extend(quad_pts([x - 8.0, y + 6.0], [x - 11.0, y], [x, y - 14.0], 6).into_iter().skip(1));
            pts.pop();
            g.poly(&pts, col);
        }
    }
}

fn tooltip(g: &Pen, txt: &str, x: f32, y: f32) {
    let w = g.text_width(txt, 18.0) + 24.0;
    let bx = (x - w / 2.0).clamp(8.0, W - w - 8.0);
    g.panel(bx, y - 34.0, w, 32.0, 10.0, hex(0x1d2738), hex(0x4a5568));
    g.text(txt, bx + w / 2.0, y - 18.0, 18.0, c::FG, Align::Center);
}

impl Game for Coolant {
    fn id(&self) -> &'static str {
        "coolant"
    }

    fn knobs(&self) -> &'static [&'static str] {
        &["damaged_state_index", "disabled_state_index", "hold_s"]
    }

    fn step(&mut self, cx: &mut Ctx) {
        self.start(cx);
    }

    fn update(&mut self, cx: &mut Ctx, dt: f32, input: &Input) {
        self.flash = (self.flash - dt * 2.0).max(0.0);
        match self.phase {
            Ph::Part => self.part_step(cx, dt, input),
            Ph::Play | Ph::Held => self.play(cx, dt, input),
            Ph::Fitted => {}
        }
    }

    fn draw(&self, g: &Pen, _cx: &Ctx, t: f32, input: &Input) {
        g.rect(0.0, BAR_H, W, 720.0, hex(0x070b12));
        let Some(l) = &self.sim else { return };
        let st = self.st();
        let d = derive(l, st);
        self.draw_loop(g, t, &d, st);
        self.draw_dials(g, &d);
        self.draw_levers(g, input);
        if self.phase == Ph::Part || self.phase == Ph::Fitted {
            // The new pump cartridge in its crate, or in the hand.
            let [x, y, w, h] = CRATE;
            g.panel(x, y, w, h, 14.0, hex(0x141b27), c::LINE);
            g.hatch(x + 10.0, y + 10.0, w - 20.0, 10.0, rgba(242, 160, 70, 0.5));
            let p = self.cart;
            if !p.set {
                g.disc(p.x + 4.0, p.y + 6.0, 30.0, rgba(0, 0, 0, 0.45));
                pump_glyph(g, p.x, p.y, false, false, t, 0.0);
                let ring = if p.held || p.pad { c::AMBER } else { rgba(232, 238, 246, 0.35) };
                g.ring(p.x, p.y, 38.0 + 3.0 * (t * 5.0).sin(), ring, 3.0);
            }
        }
    }

    fn hand(&mut self, _cx: &Ctx, _t: f32) -> Input {
        self.hand_n += 1;
        if self.phase == Ph::Part {
            let slot = pump_at(1);
            let p = self.cart;
            if !p.held && !self.hand_carry {
                self.hand_carry = true;
                return Input::hold(p.x, p.y, true);
            }
            let d = (slot[0] - p.x).hypot(slot[1] - p.y);
            if d > 2.0 {
                let k = (10.0 / d).min(1.0);
                return Input::hold(p.x + (slot[0] - p.x) * k, p.y + (slot[1] - p.y) * k, false);
            }
            self.hand_carry = false;
            return Input::release(slot[0], slot[1]);
        }
        let Some(l) = &self.sim else { return Input::default() };
        if self.phase != Ph::Play {
            return Input::default();
        }
        // An engineer's hand on the loop, one lever at a time, round and round. The pumps set the rise across the
        // core to 15 K (the bands' centres apart), as far as they can; the hot leg's aim sits at the bands' centre, or
        // higher when the radiators can only shed the heat hotter; the chiller and the radiators shed the core's heat
        // plus a pull towards the aim; the makeup keeps the loop over 90% full.
        let st = self.st();
        let d = derive(l, st);
        let q = d.q.max(1.0);
        let cav = ((l.inv - CAV_LO) / (CAV_HI - CAV_LO)).clamp(0.05, 1.0);
        let pump_k = (1.0 + st.pump_b) / 2.0;
        let pump = (q / (RISE_DESIGN_K * FLOW_FULL * CP) / cav / pump_k).clamp(0.2, SPEED_MAX);
        let flow = (pump * pump_k * cav).max(NATURAL);
        let rise = q / (flow * FLOW_FULL * CP);
        let rad_k = (1.0 + st.rad_b) / 2.0;
        let most = SPEED_MAX * rad_k;
        let th_min = T_RAD_K * (q / (RAD_MW * most)).powf(0.25);
        let th_aim = (327.5 + rise).max(th_min + 2.0).clamp(338.0, 348.0);
        let tc_aim = th_aim - rise;
        let want = (q + 3.0 * (l.tc - tc_aim) + 2.0 * (l.th - th_aim)).max(0.0);
        let p = want / rej_per(l.th);
        let (mut chill, mut rad) = (p / rad_k, 1.0);
        if chill > 1.0 {
            chill = 1.0;
            rad = (p / rad_k).min(SPEED_MAX);
        }
        let makeup = (d.leak / MAKEUP_KG_S + (0.92 - l.inv) * 30.0).clamp(0.0, 1.0);
        let vals = [pump, chill.clamp(0.02, 1.0), rad, makeup];
        let i = (self.hand_n as usize) % LEVERS.len();
        let lv = &LEVERS[i];
        Input::hold(lv.x + lv.w / 2.0, lever_y(lv, vals[i] as f32), true)
    }
}

#[cfg(test)]
mod tests {
    use crate::games::tests::{fumble_check, plays_to_end};
    use crate::kit::Input;

    #[test]
    fn a_steady_hand_repairs_it_damaged_disabled_and_destroyed() {
        plays_to_end("coolant");
    }

    #[test]
    fn stopping_the_pumps_overheats_the_hot_leg() {
        // The pump lever held at its foot: natural circulation only, and the hot leg runs away past 370 K.
        fumble_check("coolant", |_r| Input::hold(960.0, 560.0, true));
    }
}
