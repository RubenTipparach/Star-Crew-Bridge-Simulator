//! The medic's treatment (repair-minigames design 3; rates from medical-officer 2), from `docs/mockups/repairs/medic.js`.
//!
//! One body on one screen. The vitals strip runs along the top (heart rate, blood oxygen, HP, the medkit's doses); the
//! body chart on the left marks each wound by kind and size (a cut, a burn, a fracture, a bleed, smoke in the lungs);
//! the close-up on the right is the chosen wound, big, with its target drawn on it; the tool tray is along the bottom
//! (click a slot, or keys 1 to 5, Q and E). Each wound takes its own tool:
//!
//! - sealer, a cut: press on or near the cut and sweep along it, either way; what the tip passes over seals, drifting
//!   off only pauses;
//! - gel, a burn: paint it with a wide brush until it is covered; gel off the burn is wasted, not a slip;
//! - knitter, a fracture: grab the loose bone, drag it to its socket (it snaps in), hold while it knits; letting go
//!   pauses; yanking it well away once knitting has started is the slip;
//! - clamp then sealer, a bleed: the ring round the bleed shows the spurt (hatched red) and the gap (green); clamp in
//!   the gap, then seal the tear like a cut; a clamp into a spurt is the slip;
//! - inhaler, smoke: puff while the chest rises (the gauge climbs, green); a puff on the out-breath is the slip.
//!
//! The tool in hand shows at the cursor: greyed with a red cross when it does nothing here, and a press says "Wrong
//! tool". A press only treats when it starts inside the close-up, so a click on the tray or the chart never touches the
//! wound. A wound is one round of the file's HP job (no part step); a slip is a fumble, the job's 2 HP. The arrows or
//! W A S D move a cursor and Space presses, for a pad. The patient is made once, on the first round; three slips
//! restart the round, which reopens the wounds not yet treated. Nothing turns up with the level (design 1a: the
//! treatment is not rounds of one game), so the knobs are the mockup's feel, as data.
//!
//! The sparks and pulses are cosmetic and drawn from their own fixed stream (`Rng::for_purpose(7, 0, "medic-fx")`, the
//! mockup's `KIT.rng(7)`), so a shot is the same every time. The burn's gel is kept as covered cells, not a painted
//! layer: the burn's own 14 px cells, and a 14 px film grid over the close-up for gel off the burn.

use std::f32::consts::{FRAC_PI_2, PI, TAU};

use egui::{Color32, Key};
use sc_core::rng::Rng;

use crate::kit::{poly_at, poly_len, Ctx, Dice, Game, Input, BAR_H, H, W};
use crate::pen::{c, hex, mix, rgba, Align, Pen};

/// In the engine.
pub const PORTED: bool = true;

/// A fresh game.
pub fn new() -> Box<dyn Game> {
    Box::new(Medic::new())
}

/// The medkit's doses and the HP a dose gives (medical-officer 2).
const DOSES: usize = 20;
const HP_PER_DOSE: f64 = 25.0;
/// The vitals strip, the body chart, the close-up (x, y, w, h).
const VIT: [f32; 4] = [16.0, 82.0, 1248.0, 58.0];
const CHART: [f32; 4] = [16.0, 150.0, 300.0, 558.0];
const WORK: [f32; 4] = [330.0, 150.0, 934.0, 446.0];
/// The figure on the chart: its top centre, px per body unit (100 tall).
const BX: f32 = 166.0;
const BY: f32 = 172.0;
const S: f32 = 5.0;
const CX: f32 = WORK[0] + WORK[2] / 2.0;
const CY: f32 = WORK[1] + WORK[3] / 2.0;
/// The tool tray.
const TRAY_X: f32 = 330.0;
const TRAY_Y: f32 = 606.0;
const TRAY_H: f32 = 102.0;
const SLOT: f32 = 178.0;
const GAP: f32 = 11.0;
/// The sealer keeps its coverage in bins this long along the cut, px.
const BIN: f32 = 6.0;
/// The loose bone's length and width, and the grab reach off its axis, px.
const BONE: f32 = 300.0;
const BONE_W: f32 = 40.0;
const GRAB: f32 = 46.0;
/// The gel's film grid over the close-up: its cell, px, and its size.
const FILM: f32 = 14.0;
const FILM_C: usize = 67;
const FILM_R: usize = 32;
/// The smoke close-up's chest centre, and the mouth.
const BUST: [f32; 2] = [CX - 90.0, CY + 60.0];
const MOUTH: [f32; 2] = [BUST[0], BUST[1] - 222.0];
const TOOLS: [&str; 5] = ["Sealer", "Gel", "Knitter", "Clamp", "Inhaler"];
const TOOL_KEYS: [Key; 5] = [Key::Num1, Key::Num2, Key::Num3, Key::Num4, Key::Num5];
/// The hand's frame, s (the tests and the captures step at 60 a second).
const HAND_DT: f32 = 1.0 / 60.0;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Kind {
    #[default]
    Cut,
    Burn,
    Fracture,
    Bleed,
    Smoke,
}

const KINDS: [Kind; 5] = [Kind::Cut, Kind::Burn, Kind::Fracture, Kind::Bleed, Kind::Smoke];

impl Kind {
    fn name(self) -> &'static str {
        match self {
            Kind::Cut => "Cut",
            Kind::Burn => "Burn",
            Kind::Fracture => "Fracture",
            Kind::Bleed => "Bleed",
            Kind::Smoke => "Smoke",
        }
    }

    /// Where it can be on the body: [u, v, limb angle], body units from the top of the head.
    fn anchors(self) -> &'static [[f32; 3]] {
        match self {
            Kind::Cut => &[
                [-18.0, 40.0, 1.45],
                [18.0, 40.0, 1.7],
                [-6.0, 62.0, 1.53],
                [6.0, 62.0, 1.61],
                [7.0, 36.0, 1.2],
                [-15.5, 24.0, 1.38],
                [15.5, 24.0, 1.76],
            ],
            Kind::Burn => &[
                [-5.0, 24.0, 0.0],
                [5.0, 27.0, 0.0],
                [-11.0, 18.0, 0.0],
                [11.0, 18.0, 0.0],
                [-6.0, 66.0, 0.0],
                [6.0, 66.0, 0.0],
                [0.0, 40.0, 0.0],
            ],
            Kind::Fracture => &[
                [-18.0, 41.0, 1.45],
                [18.0, 41.0, 1.7],
                [-6.8, 85.0, 1.55],
                [6.8, 85.0, 1.59],
                [-15.5, 24.0, 1.38],
                [15.5, 24.0, 1.76],
            ],
            Kind::Bleed => &[
                [-6.0, 58.0, 0.0],
                [6.0, 58.0, 0.0],
                [-15.5, 26.0, 0.0],
                [15.5, 26.0, 0.0],
                [-18.0, 38.0, 0.0],
                [18.0, 38.0, 0.0],
            ],
            Kind::Smoke => &[[0.0, 27.0, 0.0]],
        }
    }
}

/// The mockup's feel, from the file (generous on purpose; owner, 2026-10-08: "the tools dont respond well").
#[derive(Clone, Copy, Debug, Default)]
struct Feel {
    band: f32,
    tip: f32,
    seal_done: f32,
    brush: f32,
    gel_done: f32,
    snap_in: f32,
    snap_out: f32,
    yank: f32,
    spurt: f32,
    inhale: f32,
}

/// A line across the close-up (a cut, a bleed's tear): its points, arc lengths, and its sealed bins.
#[derive(Clone, Debug, Default)]
struct Trail {
    pts: Vec<[f32; 2]>,
    cum: Vec<f32>,
    len: f32,
    n: usize,
    /// When each bin was sealed, s, or below zero.
    seal: Vec<f32>,
    count: usize,
    last_s: Option<f32>,
}

#[derive(Clone, Debug, Default)]
struct Wound {
    kind: Kind,
    u: f32,
    v: f32,
    la: f32,
    size: usize,
    done: bool,
    tr: Option<Trail>,
    // A burn.
    r: f32,
    a: f32,
    b: f32,
    cells: Vec<[f32; 2]>,
    cov: Vec<bool>,
    cov_n: usize,
    lp: Option<[f32; 2]>,
    film: Vec<bool>,
    // A fracture.
    bs: [f32; 2],
    e0: [f32; 2],
    tilt: f32,
    knit_t: f32,
    e: [f32; 2],
    held: bool,
    snap: bool,
    seated: bool,
    knit: f32,
    gx: f32,
    gy: f32,
    // A bleed.
    period: f32,
    ph: f32,
    th: f32,
    clamped: bool,
    // Smoke.
    need: usize,
    breath_s: f32,
    puffs: usize,
    last: i64,
    mist: f32,
}

#[derive(Clone, Copy, Debug)]
struct Fx {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    life: f32,
    max: f32,
    col: Color32,
    r: f32,
    g: f32,
}

/// A burst's shape: its direction and spread, a spark's radius, its gravity.
#[derive(Clone, Copy, Debug)]
struct Spray {
    dir: f32,
    spread: f32,
    r: f32,
    grav: f32,
}

impl Default for Spray {
    fn default() -> Self {
        Self { dir: 0.0, spread: PI, r: 3.0, grav: 0.0 }
    }
}

#[derive(Clone, Copy, Debug)]
struct Ping {
    x: f32,
    y: f32,
    col: Color32,
    r: f32,
    t: f32,
}

/// The pointer this frame: the mouse's or touch's, or the pad's cursor.
#[derive(Clone, Copy, Debug)]
struct P {
    x: f32,
    y: f32,
    down: bool,
    pressed: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Out {
    Busy,
    Done,
    Slip,
}

/// The game's state.
pub struct Medic {
    k: Feel,
    start_hp: f64,
    wounds: Vec<Wound>,
    t: f32,
    tool: usize,
    cur: usize,
    flinch: f32,
    pad: bool,
    vc: [f32; 2],
    mx: f32,
    my: f32,
    index: i64,
    waiting: bool,
    /// A press that started in the close-up: whether its tool was the one this wound takes.
    gest: Option<bool>,
    hint: f32,
    cross: Option<([f32; 2], f32)>,
    tip: Option<([f32; 2], f32)>,
    fx: Vec<Fx>,
    pings: Vec<Ping>,
    fxr: Rng,
    /// The hand: the wound it is working, its route and how far along it is, whether it is pressing, where.
    hand_wi: usize,
    hand_route: Vec<[f32; 2]>,
    hand_u: f32,
    hand_down: bool,
    hand_pos: [f32; 2],
    /// The hand pressed a tool's key last frame: this frame it lets every key go.
    hand_keyed: bool,
}

impl Medic {
    fn new() -> Self {
        Self {
            k: Feel::default(),
            start_hp: 30.0,
            wounds: Vec::new(),
            t: 0.0,
            tool: 0,
            cur: 0,
            flinch: 0.0,
            pad: false,
            vc: [CX, CY],
            mx: -1.0,
            my: -1.0,
            index: -1,
            waiting: false,
            gest: None,
            hint: 0.0,
            cross: None,
            tip: None,
            fx: Vec::new(),
            pings: Vec::new(),
            fxr: Rng::for_purpose(7, 0, "medic-fx"),
            hand_wi: usize::MAX,
            hand_route: Vec::new(),
            hand_u: 0.0,
            hand_down: false,
            hand_pos: [CX, CY],
            hand_keyed: false,
        }
    }

    fn fr(&mut self) -> f32 {
        self.fxr.next_f64() as f32
    }

    fn burst(&mut self, at: [f32; 2], n: usize, col: Color32, speed: f32, life: f32, o: Spray) {
        for _ in 0..n {
            if self.fx.len() >= 500 {
                break;
            }
            let a = o.dir + o.spread * (self.fr() * 2.0 - 1.0);
            let v = speed * (0.4 + 0.6 * self.fr());
            let l = life * (0.6 + 0.4 * self.fr());
            self.fx.push(Fx {
                x: at[0],
                y: at[1],
                vx: a.cos() * v,
                vy: a.sin() * v,
                life: l,
                max: life,
                col,
                r: o.r,
                g: o.grav,
            });
        }
    }

    fn ping(&mut self, x: f32, y: f32, col: Color32, r: f32) {
        self.pings.push(Ping { x, y, col, r, t: 0.0 });
    }

    fn spurting(&self, w: &Wound) -> bool {
        !w.clamped && (self.t + w.ph) % w.period < self.k.spurt
    }

    fn breath(&self, w: &Wound) -> f32 {
        frac((self.t + w.ph) / w.breath_s)
    }

    /// How full the breath is, 0 to 1: up over the in-breath, down over the out-breath.
    fn lungs(&self, w: &Wound) -> f32 {
        let f = self.breath(w);
        let i = self.k.inhale;
        if f < i {
            (f / i * FRAC_PI_2).sin()
        } else {
            ((f - i) / (1.0 - i) * FRAC_PI_2).cos()
        }
    }

    fn next_open(&self) -> usize {
        self.wounds.iter().position(|w| !w.done).unwrap_or(self.cur)
    }

    /// How far along a wound is, 0 to 1.
    fn progress(&self, w: &Wound) -> f32 {
        if w.done {
            return 1.0;
        }
        let sealed = |tr: &Trail| (tr.count as f32 / (tr.n as f32 * self.k.seal_done)).min(1.0);
        match w.kind {
            Kind::Cut => w.tr.as_ref().map_or(0.0, sealed),
            Kind::Burn => (w.cov_n as f32 / (w.cells.len() as f32 * self.k.gel_done)).min(1.0),
            Kind::Fracture => w.knit.min(1.0),
            Kind::Bleed => {
                if w.clamped {
                    0.3 + 0.7 * w.tr.as_ref().map_or(0.0, sealed)
                } else {
                    0.0
                }
            }
            Kind::Smoke => w.puffs as f32 / w.need as f32,
        }
    }

    fn sealed_enough(&self, tr: &Trail) -> bool {
        tr.count as f32 >= tr.n as f32 * self.k.seal_done
    }

    /// Mouse, touch, or the pad's cursor (the stick moves it, Space presses).
    fn pointer(&mut self, dt: f32, input: &Input) -> P {
        if input.stick[0] != 0.0 || input.stick[1] != 0.0 {
            if !self.pad {
                self.pad = true;
                self.vc = [if self.mx >= 0.0 { self.mx } else { CX }, if self.my >= 0.0 { self.my } else { CY }];
            }
            self.vc = [
                (self.vc[0] + input.stick[0] * 420.0 * dt).clamp(0.0, W),
                (self.vc[1] + input.stick[1] * 420.0 * dt).clamp(BAR_H, H),
            ];
        }
        if input.pressed || input.x != self.mx || input.y != self.my {
            self.pad = false;
        }
        self.mx = input.x;
        self.my = input.y;
        if self.pad {
            P { x: self.vc[0], y: self.vc[1], down: input.action, pressed: input.action_pressed }
        } else {
            P { x: input.x, y: input.y, down: input.down, pressed: input.pressed }
        }
    }

    /// Where the cursor is (the pad's or the mouse's), or None before the pointer has been seen.
    fn cursor(&self) -> Option<[f32; 2]> {
        if self.pad {
            Some(self.vc)
        } else if self.mx >= 0.0 {
            Some([self.mx, self.my])
        } else {
            None
        }
    }

    // ------------------------------------------------------------------------------------------------ treatment

    /// The sealer on a line this frame: seals what the tip passes over; off the band it only pauses.
    fn seal_along(&mut self, tr: &mut Trail, p: &P, on: bool) {
        if !on {
            tr.last_s = None;
            return;
        }
        let (d, sv) = project(tr, p.x, p.y);
        if d > self.k.band {
            tr.last_s = None;
            if p.pressed {
                self.hint = 1.0;
            }
            return;
        }
        let (mut a, mut b) = (sv, sv);
        if let Some(l) = tr.last_s {
            if (l - sv).abs() < 240.0 {
                a = a.min(l);
                b = b.max(l);
            }
        }
        tr.last_s = Some(sv);
        let i0 = ((a - self.k.tip) / BIN).floor().max(0.0) as usize;
        let i1 = (((b + self.k.tip) / BIN).floor().max(0.0) as usize).min(tr.n.saturating_sub(1));
        for i in i0..=i1 {
            if tr.seal[i] < 0.0 {
                tr.seal[i] = self.t;
                tr.count += 1;
            }
        }
        let at = trail_at(tr, sv);
        let col = if self.fr() < 0.5 { hex(0xbfefff) } else { c::ACCENT };
        self.burst(at, if p.pressed { 14 } else { 4 }, col, 220.0, 0.35, Spray { r: 2.5, ..Spray::default() });
        self.tip = Some((at, 0.12));
        if p.pressed {
            self.ping(at[0], at[1], c::ACCENT, 30.0);
        }
    }

    fn paint_gel(&self, w: &mut Wound, a: [f32; 2], b: [f32; 2]) {
        let r = self.k.brush;
        for i in 0..w.cells.len() {
            if !w.cov[i] && seg_dist(w.cells[i], a, b) < r {
                w.cov[i] = true;
                w.cov_n += 1;
            }
        }
        let (x0, x1) = (a[0].min(b[0]) - r, a[0].max(b[0]) + r);
        let (y0, y1) = (a[1].min(b[1]) - r, a[1].max(b[1]) + r);
        let ci = |v: f32, o: f32, n: usize| (((v - o) / FILM).floor().max(0.0) as usize).min(n - 1);
        for j in ci(y0, WORK[1], FILM_R)..=ci(y1, WORK[1], FILM_R) {
            for i in ci(x0, WORK[0], FILM_C)..=ci(x1, WORK[0], FILM_C) {
                let q = [WORK[0] + (i as f32 + 0.5) * FILM, WORK[1] + (j as f32 + 0.5) * FILM];
                if seg_dist(q, a, b) < r {
                    w.film[j * FILM_C + i] = true;
                }
            }
        }
    }

    fn treat(&mut self, w: &mut Wound, p: &P, dt: f32, on: bool) -> Out {
        match w.kind {
            Kind::Cut => {
                let mut tr = w.tr.take().unwrap_or_default();
                self.seal_along(&mut tr, p, on);
                let done = self.sealed_enough(&tr);
                w.tr = Some(tr);
                if done {
                    Out::Done
                } else {
                    Out::Busy
                }
            }
            Kind::Burn => {
                if !on {
                    w.lp = None;
                    return Out::Busy;
                }
                let from = w.lp.unwrap_or([p.x, p.y]);
                self.paint_gel(w, from, [p.x, p.y]);
                w.lp = Some([p.x, p.y]);
                let on_burn = in_burn(w, p.x, p.y, 0.0);
                let col = if on_burn { hex(0xa8f5e6) } else { hex(0x5f8f88) };
                self.burst(
                    [p.x, p.y],
                    if p.pressed { 12 } else { 3 },
                    col,
                    120.0,
                    0.4,
                    Spray { r: 4.0, ..Spray::default() },
                );
                if p.pressed {
                    self.ping(p.x, p.y, if on_burn { hex(0x7fe6d2) } else { c::DIM }, self.k.brush);
                }
                if p.pressed && !in_burn(w, p.x, p.y, self.k.brush) {
                    self.hint = 1.0;
                }
                if w.cov_n as f32 >= w.cells.len() as f32 * self.k.gel_done {
                    Out::Done
                } else {
                    Out::Busy
                }
            }
            Kind::Fracture => {
                if on && p.pressed {
                    let far = bone_far(w);
                    if seg_dist([p.x, p.y], w.e, far) < GRAB {
                        w.held = true;
                        w.seated = false;
                        w.gx = w.e[0] - p.x;
                        w.gy = w.e[1] - p.y;
                        self.burst([p.x, p.y], 12, c::LILAC, 160.0, 0.4, Spray::default());
                        self.ping(p.x, p.y, c::LILAC, 36.0);
                    } else {
                        self.hint = 1.0;
                    }
                }
                if !on {
                    w.held = false;
                }
                if !w.held {
                    return Out::Busy;
                }
                let (tx, ty) = (p.x + w.gx, p.y + w.gy);
                let d = (tx - w.bs[0]).hypot(ty - w.bs[1]);
                // The slip: pulled well out of the socket in the same hold that seated it, once knitting has started.
                if w.seated && w.knit > 0.0 && d > self.k.yank {
                    w.held = false;
                    w.snap = false;
                    w.seated = false;
                    w.knit = (w.knit - 0.25).max(0.0);
                    w.e = [tx, ty];
                    self.burst(w.bs, 18, c::DANGER, 260.0, 0.5, Spray::default());
                    return Out::Slip;
                }
                let was = w.snap;
                w.snap = d < self.k.snap_in || (w.snap && d < self.k.snap_out);
                w.e = if w.snap { w.bs } else { [tx, ty] };
                if w.snap {
                    w.seated = true;
                }
                if w.snap && !was {
                    self.burst(w.bs, 16, hex(0xf3eaff), 240.0, 0.35, Spray::default());
                    self.ping(w.bs[0], w.bs[1], c::LILAC, 50.0);
                }
                if w.snap {
                    w.knit += dt / w.knit_t;
                    let y = w.bs[1] + (self.fr() - 0.5) * 40.0;
                    self.burst([w.bs[0], y], 2, c::LILAC, 90.0, 0.5, Spray { r: 3.5, ..Spray::default() });
                }
                if w.knit >= 1.0 {
                    Out::Done
                } else {
                    Out::Busy
                }
            }
            Kind::Bleed => {
                if !w.clamped {
                    if !(on && p.pressed) {
                        return Out::Busy;
                    }
                    if self.spurting(w) {
                        self.burst([CX, CY], 24, c::DANGER, 300.0, 0.6, Spray { grav: 300.0, ..Spray::default() });
                        return Out::Slip;
                    }
                    w.clamped = true;
                    let k = clamp_xy(w);
                    self.burst(k, 20, hex(0xe8eef6), 260.0, 0.35, Spray::default());
                    self.ping(k[0], k[1], c::OK, 60.0);
                    return Out::Busy;
                }
                let mut tr = w.tr.take().unwrap_or_default();
                self.seal_along(&mut tr, p, on);
                let done = self.sealed_enough(&tr);
                w.tr = Some(tr);
                if done {
                    Out::Done
                } else {
                    Out::Busy
                }
            }
            Kind::Smoke => {
                if !(on && p.pressed) {
                    return Out::Busy;
                }
                let n = ((self.t + w.ph) / w.breath_s).floor() as i64;
                if self.breath(w) >= self.k.inhale {
                    self.burst(MOUTH, 18, c::DANGER, 200.0, 0.5, Spray::default());
                    return Out::Slip;
                }
                if w.last == n {
                    // One puff a breath: this one is taken.
                    self.hint = 1.0;
                    return Out::Busy;
                }
                w.last = n;
                w.puffs += 1;
                w.mist = 1.0;
                let o = Spray { dir: FRAC_PI_2, spread: 0.9, r: 6.0, grav: 0.0 };
                self.burst([MOUTH[0], MOUTH[1] - 30.0], 26, hex(0x9fe0ff), 160.0, 0.7, o);
                self.ping(MOUTH[0], MOUTH[1], c::ACCENT, 50.0);
                if w.puffs >= w.need {
                    Out::Done
                } else {
                    Out::Busy
                }
            }
        }
    }

    // ------------------------------------------------------------------------------------------------ drawing

    fn marker(&self, g: &Pen, w: &Wound, i: usize) {
        let [x, y] = chart_xy(w);
        let k = 1.5 + 0.4 * w.size as f32;
        if i == self.cur && !w.done {
            g.disc(x, y, 30.0, rgba(242, 160, 70, 0.18));
            g.ring(x, y, 28.0 + 3.0 * (self.t * 5.0).sin(), c::AMBER, 3.0);
        }
        let m = g.translate(x, y);
        let m = if w.done { m.alpha(0.45) } else { m };
        match w.kind {
            Kind::Cut => {
                let q = m.rotate(w.la + FRAC_PI_2 + 0.5);
                q.line(-10.0 * k, 0.0, 10.0 * k, 0.0, 5.0, c::DANGER);
                for j in -1..=1 {
                    let jx = j as f32 * 6.0 * k;
                    q.line(jx, -5.0, jx, 5.0, 2.0, c::DANGER);
                }
            }
            Kind::Burn => {
                let pts: Vec<[f32; 2]> = (0..9)
                    .map(|j| {
                        let a = j as f32 / 9.0 * TAU;
                        let rr = 9.0 * k * (1.0 + 0.25 * (j as f32 * 2.3).sin());
                        [a.cos() * rr, a.sin() * rr]
                    })
                    .collect();
                m.poly(&pts, c::AMBER);
                for j in -1..=1 {
                    let o = j as f32 * 5.0;
                    m.line(-6.0 * k + o, 6.0 * k, 6.0 * k + o, -6.0 * k, 2.0, hex(0x5a2f0c));
                }
            }
            Kind::Fracture => {
                let q = m.rotate(w.la + FRAC_PI_2);
                q.path(
                    &[[-11.0 * k, 0.0], [-4.0 * k, -5.0], [0.0, 5.0], [4.0 * k, -5.0], [11.0 * k, 0.0]],
                    false,
                    4.0,
                    c::FG,
                );
            }
            Kind::Bleed => {
                let mut pts = qbez([0.0, -12.0 * k], [9.0 * k, 0.0], [0.0, 8.0 * k], 12);
                pts.extend(qbez([0.0, 8.0 * k], [-9.0 * k, 0.0], [0.0, -12.0 * k], 12).into_iter().skip(1));
                pts.pop();
                m.poly(&pts, c::DANGER);
                m.path(&pts, true, 2.0, hex(0xffd0d4));
            }
            Kind::Smoke => {
                for [dx, dy, rr] in
                    [[-12.0, 2.0, 13.0], [0.0, -6.0, 15.0], [12.0, 3.0, 12.0], [-4.0, 10.0, 11.0], [8.0, 12.0, 10.0]]
                {
                    m.disc(dx * k * 0.8, dy * k * 0.8, rr * k * 0.8, rgba(160, 170, 184, 0.75));
                }
            }
        }
        if w.done {
            check(g, x + 12.0, y - 12.0, 12.0);
        }
    }

    fn vitals(&self, g: &Pen, hp: f64) {
        let smoke = self.wounds.iter().find(|w| w.kind == Kind::Smoke && !w.done);
        let hr = (128.0 - (hp - self.start_hp) as f32 * 0.8 + 30.0 * self.flinch).round();
        let spo2 = smoke.map_or(97.0, |w| (86.0 + 8.0 * w.puffs as f32 / w.need as f32).round());
        let [vx, vy, vw, vh] = VIT;
        g.panel(vx, vy, vw, vh, 12.0, hex(0x0b1018), c::LINE);
        let y = vy + vh / 2.0;
        g.text("HR", vx + 20.0, y, 16.0, c::DIM, Align::Left);
        g.text(&format!("{hr}"), vx + 56.0, y, 30.0, if hr > 110.0 { c::WARN } else { c::OK }, Align::Left);
        let e = g.clip(130.0, vy + 6.0, 420.0, vh - 12.0);
        let pts: Vec<[f32; 2]> = (0..=210)
            .map(|i| {
                let x = 130.0 + i as f32 * 2.0;
                let p = frac((self.t - (550.0 - x) / 140.0) * (hr / 60.0));
                [x, y + 8.0 - 22.0 * ecg(p)]
            })
            .collect();
        e.path(&pts, false, 2.0, c::OK);
        g.text("SpO2", 590.0, y, 16.0, c::DIM, Align::Left);
        g.text(&format!("{spo2}%"), 640.0, y, 30.0, if spo2 < 92.0 { c::WARN } else { c::OK }, Align::Left);
        if spo2 < 92.0 {
            g.poly(&[[718.0, y - 12.0], [706.0, y + 10.0], [730.0, y + 10.0]], c::WARN);
        }
        g.text("HP", 770.0, y, 16.0, c::DIM, Align::Left);
        let hpc = if hp >= 75.0 {
            c::OK
        } else if hp >= 25.0 {
            c::WARN
        } else {
            c::DANGER
        };
        g.text(&format!("{}", hp.round()), 800.0, y, 30.0, hpc, Align::Left);
        g.round(860.0, y - 7.0, 160.0, 14.0, 7.0, Some(hex(0x1a2230)), None);
        let bar = if hp >= 75.0 { c::OK } else { c::WARN };
        g.round(860.0, y - 7.0, (1.6 * hp as f32).max(14.0), 14.0, 7.0, Some(bar), None);
        g.rect(860.0 + 1.6 * 75.0 - 1.0, y - 11.0, 3.0, 22.0, c::FG);
        let used = ((hp - self.start_hp - 1e-6).max(0.0) / HP_PER_DOSE).ceil() as usize;
        let left = DOSES.saturating_sub(used);
        g.text("Doses", 1050.0, y, 16.0, c::DIM, Align::Left);
        for i in 0..DOSES {
            let (x, yy) = (1102.0 + (i % 10) as f32 * 15.0, y - 9.0 + (i / 10) as f32 * 18.0);
            if i < left {
                g.disc(x, yy, 5.0, c::LILAC);
            } else {
                g.ring(x, yy, 4.0, hex(0x3a4658), 2.0);
            }
        }
    }

    fn tray(&self, g: &Pen) {
        for (i, name) in TOOLS.iter().enumerate() {
            let on = self.tool == i;
            let x = TRAY_X + i as f32 * (SLOT + GAP);
            let y = TRAY_Y - if on { 4.0 } else { 0.0 };
            g.panel(
                x,
                y,
                SLOT,
                TRAY_H,
                14.0,
                if on { hex(0x1d2738) } else { hex(0x0d131c) },
                if on { c::AMBER } else { c::LINE },
            );
            if on {
                g.round(x, y, SLOT, TRAY_H, 14.0, None, Some((4.0, c::AMBER)));
            }
            tool_icon(g, i, x + SLOT / 2.0, y + 42.0, if on { hex(0xd5dde6) } else { c::STEEL }, c::ACCENT);
            g.text(name, x + SLOT / 2.0, y + 84.0, 19.0, if on { c::FG } else { c::DIM }, Align::Center);
            // The hotkey, as a keycap in the corner.
            g.round(
                x + 10.0,
                y + 10.0,
                26.0,
                26.0,
                6.0,
                Some(if on { c::AMBER } else { hex(0x1a2230) }),
                Some((1.5, if on { c::AMBER } else { hex(0x3a4658) })),
            );
            g.text(
                &(i + 1).to_string(),
                x + 23.0,
                y + 24.0,
                17.0,
                if on { hex(0x0b111b) } else { c::DIM },
                Align::Center,
            );
        }
    }

    fn skin(&self, g: &Pen) {
        let [wx, wy, ww, wh] = WORK;
        g.rect(wx, wy, ww, wh, hex(0x4f3a31));
        g.radial(CX, CY, ww * 0.62, hex(0x8a6553), hex(0x4f3a31));
        for j in 0..9 {
            let pts =
                ellipse(wx + 70.0 + j as f32 * 105.0, wy + 60.0 + (j % 3) as f32 * 150.0, 60.0, 22.0, j as f32 * 0.7);
            g.poly(&pts, rgba(0, 0, 0, 0.08));
        }
    }

    /// A line to seal: its working band, the raw wound, then what is sealed, glowing as it cools.
    fn draw_seal(&self, g: &Pen, tr: &Trail, done: bool, raw: Color32, edge: Color32, live: bool) {
        if !done {
            let a = if live { 0.10 + 0.22 * self.hint } else { 0.05 };
            cap_line(g, &tr.pts, self.k.band * 2.0, rgba(79, 195, 247, a));
            if live {
                g.dashed(&tr.pts, 2.0, rgba(79, 195, 247, 0.25 + 0.5 * self.hint), 8.0, 10.0);
            }
            g.path_round(&tr.pts, 14.0, raw);
            g.path_round(&tr.pts, 4.0, edge);
        }
        for i in 0..tr.n {
            if !done && tr.seal[i] < 0.0 {
                continue;
            }
            let p0 = trail_at(tr, i as f32 * BIN);
            let p1 = trail_at(tr, tr.len.min((i + 1) as f32 * BIN + 0.5));
            g.path_round(&[p0, p1], 10.0, hex(0xd9a294));
            let age = if done { 9.0 } else { self.t - tr.seal[i] };
            if age < 0.8 {
                g.line(p0[0], p0[1], p1[0], p1[1], 16.0, rgba(160, 235, 255, 0.9 * (1.0 - age / 0.8)));
            }
        }
        // Stitches across the sealed length, every 22 px.
        let mut sv = 11.0;
        while sv < tr.len {
            let bin = ((sv / BIN).floor() as usize).min(tr.n.saturating_sub(1));
            if done || tr.seal[bin] >= 0.0 {
                let [x, y] = trail_at(tr, sv);
                let [x2, y2] = trail_at(tr, tr.len.min(sv + 2.0));
                let a = (y2 - y).atan2(x2 - x) + FRAC_PI_2;
                g.line(x - a.cos() * 9.0, y - a.sin() * 9.0, x + a.cos() * 9.0, y + a.sin() * 9.0, 3.0, c::OK);
            }
            sv += 22.0;
        }
    }

    fn draw_burn(&self, g: &Pen, w: &Wound, live: bool) {
        self.skin(g);
        let outline: Vec<[f32; 2]> = (0..72)
            .map(|j| {
                let th = j as f32 / 72.0 * TAU;
                let rr = burn_r(w, th);
                [CX + th.cos() * rr, CY + th.sin() * rr]
            })
            .collect();
        let (c0, c1) = if w.done { (hex(0x9b6a58), hex(0x86604f)) } else { (hex(0xd2583a), hex(0x8e3424)) };
        radial_poly(g, [CX, CY], &outline, c0, c1, w.r * 1.2);
        if !w.done {
            for j in 0..14 {
                let a = j as f32 * 2.4;
                let rr = w.r * (0.15 + 0.65 * frac(j as f32 * 0.37));
                let (x, y, br) = (CX + a.cos() * rr, CY + a.sin() * rr * 0.9, 6.0 + (j % 3) as f32 * 3.0);
                if in_burn(w, x, y, -br) {
                    g.disc(x, y, br, hex(0xec9a7f));
                    g.ring(x, y, br, hex(0x7a2a1c), 1.5);
                }
            }
        }
        // Gel off the burn is a thin, wasted film; gel on it reads strong (it is working).
        let gel = hex(0x7fe6d2);
        let film = g.alpha(0.2);
        for j in 0..FILM_R {
            let mut i = 0;
            while i < FILM_C {
                if !w.film.get(j * FILM_C + i).copied().unwrap_or(false) {
                    i += 1;
                    continue;
                }
                let s = i;
                while i < FILM_C && w.film[j * FILM_C + i] {
                    i += 1;
                }
                film.rect(WORK[0] + s as f32 * FILM, WORK[1] + j as f32 * FILM, (i - s) as f32 * FILM, FILM, gel);
            }
        }
        let strong = g.alpha(0.45);
        let mut k = 0;
        while k < w.cells.len() {
            if !w.cov[k] {
                k += 1;
                continue;
            }
            let s = k;
            while k + 1 < w.cells.len()
                && w.cov[k + 1]
                && w.cells[k + 1][1] == w.cells[s][1]
                && (w.cells[k + 1][0] - w.cells[k][0] - 14.0).abs() < 0.5
            {
                k += 1;
            }
            let ([x0, y0], x1) = (w.cells[s], w.cells[k][0]);
            strong.rect(x0 - 7.0, y0 - 7.0, x1 - x0 + 14.0, 14.0, gel);
            k += 1;
        }
        // The target: a dashed outline round the burn, on top of the gel, brighter when a press missed it.
        let col = if w.done {
            rgba(61, 220, 132, 0.7)
        } else if self.hint > 0.0 {
            c::WARN
        } else {
            c::AMBER
        };
        let mut closed = outline.clone();
        closed.push(outline[0]);
        g.dashed(&closed, 3.0 + 3.0 * self.hint, col, 12.0, 9.0);
        if live && !w.done {
            if let Some([px, py]) = self.cursor().filter(|p| in_work(p[0], p[1])) {
                g.dashed(&circle(px, py, self.k.brush), 2.0, rgba(168, 245, 230, 0.8), 6.0, 6.0);
            }
        }
    }

    fn draw_fracture(&self, g: &Pen, w: &Wound, live: bool) {
        let [wx, wy, ww, wh] = WORK;
        g.rect(wx, wy, ww, wh, hex(0x081420));
        g.round(wx + 20.0, CY - 130.0, ww - 40.0, 260.0, 130.0, Some(rgba(107, 140, 170, 0.13)), None);
        let [bx, by] = w.bs;
        bone(g, [bx - 380.0, by], [bx, by]);
        if w.done {
            bone(g, [bx + BONE, by], [bx, by]);
            g.line(bx, by - 22.0, bx, by + 22.0, 4.0, c::LILAC);
            return;
        }
        // The target: where the loose bone belongs (a dashed ghost) and its socket, the snap zone.
        let p = self.cursor();
        let near = w.held && dist(w.e, w.bs) < 90.0;
        let si = self.k.snap_in;
        if !w.snap {
            let mut ghost = crate::pen::round_rect_points(bx, by - BONE_W / 2.0, BONE, BONE_W, BONE_W / 2.0);
            ghost.push(ghost[0]);
            g.dashed(&ghost, 2.0, rgba(190, 159, 230, 0.55), 10.0, 8.0);
            g.disc(bx, by, si, if near { rgba(190, 159, 230, 0.4) } else { rgba(190, 159, 230, 0.16) });
            g.dashed(&circle(bx, by, si), if near { 3.0 } else { 2.0 }, c::LILAC, 5.0, 5.0);
            if !w.held {
                let f = frac(self.t * 0.8);
                g.ring(bx, by, si + 6.0 + 14.0 * f, rgba(190, 159, 230, 0.6 * (1.0 - f)), 2.0);
            }
            if self.hint > 0.0 {
                g.ring(bx, by, si + 10.0 + 20.0 * (1.0 - self.hint), rgba(190, 159, 230, self.hint), 3.0);
            }
        }
        let far = bone_far(w);
        let tremor = if w.snap && w.held { 1.2 } else { 0.0 };
        let (ex, ey) = (w.e[0] + tremor * (self.t * 23.0).sin(), w.e[1] + tremor * (self.t * 19.0).cos());
        let hover = live && !w.held && p.is_some_and(|q| seg_dist(q, w.e, far) < GRAB);
        if w.held || hover {
            let col = if w.held { rgba(190, 159, 230, 0.55) } else { rgba(190, 159, 230, 0.3) };
            cap_line(g, &[far, [ex, ey]], BONE_W + 22.0, col);
        }
        bone(g, [far[0] + (ex - w.e[0]), far[1] + (ey - w.e[1])], [ex, ey]);
        // The knit: a ring round the join that fills while it is held in the socket.
        if w.knit > 0.0 || w.snap {
            g.ring(bx, by, 52.0, rgba(26, 34, 48, 0.9), 9.0);
            if w.knit > 0.0 {
                g.arc(bx, by, 52.0, -FRAC_PI_2, -FRAC_PI_2 + TAU * w.knit.min(1.0), 9.0, c::LILAC);
            }
            if w.snap && w.held {
                g.ring(bx, by, 30.0 + 6.0 * (self.t * 14.0).sin(), rgba(243, 234, 255, 0.8), 3.0);
            }
        }
    }

    fn draw_bleed(&self, g: &Pen, w: &Wound, live: bool) {
        self.skin(g);
        let v = g.translate(CX, CY).rotate(w.th);
        v.line(-620.0, 0.0, 620.0, 0.0, 54.0, hex(0x4a121b));
        v.line(-620.0, 0.0, 620.0, 0.0, 42.0, hex(0x7a1f2a));
        v.line(-620.0, -11.0, 620.0, -11.0, 6.0, rgba(255, 120, 130, 0.25));
        if !w.done {
            v.poly(&ellipse(30.0, 60.0, 170.0, 46.0, 0.0), rgba(122, 20, 32, 0.55));
        }
        if w.clamped || w.done {
            // The clamp across the vessel, upstream of the tear.
            let steel = hex(0xc9d2dc);
            v.rect(-176.0, -40.0, 14.0, 80.0, steel);
            v.line(-169.0, -40.0, -220.0, -110.0, 7.0, steel);
            v.line(-169.0, 40.0, -232.0, -92.0, 7.0, steel);
            v.ring(-226.0, -118.0, 11.0, steel, 5.0);
            v.ring(-242.0, -96.0, 11.0, steel, 5.0);
        }
        if !w.clamped && !w.done {
            // The rhythm: a hand goes round once a beat; the spurt is the hatched red arc, the gap is green.
            let r = 92.0;
            let ph = ((self.t + w.ph) % w.period) / w.period;
            let a0 = -FRAC_PI_2;
            let a_s = a0 + TAU * (self.k.spurt / w.period);
            let sp = self.spurting(w);
            g.disc(CX, CY, r + 14.0, rgba(4, 6, 10, 0.35));
            let (gc, gw) = if sp { (rgba(61, 220, 132, 0.45), 10.0) } else { (c::OK, 16.0) };
            g.arc(CX, CY, r, a_s + 0.04, a0 + TAU - 0.04, gw, gc);
            g.arc(CX, CY, r, a0, a_s, 18.0, rgba(255, 71, 87, 0.35));
            g.hatch_arc(CX, CY, r - 9.0, r + 9.0, a0, a_s, c::DANGER);
            let ah = a0 + TAU * ph;
            g.path_round(
                &[
                    [CX + ah.cos() * (r - 22.0), CY + ah.sin() * (r - 22.0)],
                    [CX + ah.cos() * (r + 22.0), CY + ah.sin() * (r + 22.0)],
                ],
                6.0,
                c::FG,
            );
            let (hx, hy) = (CX + ah.cos() * r, CY + ah.sin() * r);
            g.disc(hx, hy, 8.0, if sp { c::DANGER } else { c::OK });
            g.ring(hx, hy, 8.0, c::FG, 2.0);
            // The bleed point, and the spurt itself.
            g.disc(CX, CY, if sp { 22.0 } else { 16.0 }, if sp { c::DANGER } else { hex(0xa01c2a) });
            if sp {
                let q0 = ((self.t + w.ph) % w.period) / self.k.spurt;
                for j in 0..22 {
                    let q = (q0 + j as f32 / 22.0) % 1.0;
                    let a = -FRAC_PI_2 + w.th + ((j % 7) as f32 - 3.0) * 0.1;
                    g.disc(
                        CX + a.cos() * q * 200.0,
                        CY + a.sin() * q * 200.0 + q * q * 120.0,
                        8.0 - 5.0 * q,
                        c::DANGER,
                    );
                }
            }
        }
        if w.clamped || w.done {
            if let Some(tr) = &w.tr {
                self.draw_seal(g, tr, w.done, hex(0x5a0f18), hex(0xff6b78), live);
            }
        }
    }

    fn draw_smoke(&self, g: &Pen, w: &Wound) {
        let [wx, wy, ww, wh] = WORK;
        g.rect(wx, wy, ww, wh, hex(0x081420));
        let lv = if w.done { 0.5 } else { self.lungs(w) };
        let up = !w.done && self.breath(w) < self.k.inhale;
        let k = 1.0 + 0.07 * lv;
        let b = g.translate(BUST[0], BUST[1]).scale(k, 1.0 + 0.03 * lv);
        let (body, rim) = (hex(0x1b293b), hex(0x4d6683));
        b.disc(0.0, -222.0, 56.0, body);
        b.ring(0.0, -222.0, 56.0, rim, 3.0);
        b.rect(-30.0, -172.0, 60.0, 40.0, body);
        b.rect_stroke(-30.0, -172.0, 60.0, 40.0, 3.0, rim);
        let mut torso = qbez([-230.0, -132.0], [0.0, -180.0], [230.0, -132.0], 16);
        torso.extend_from_slice(&[[200.0, 200.0], [-200.0, 200.0]]);
        b.poly(&torso, body);
        b.path(&torso, true, if up { 4.0 } else { 3.0 }, if up { c::OK } else { rim });
        let left = if w.done { 0.0 } else { 1.0 - w.puffs as f32 / w.need as f32 };
        for sx in [-1.0f32, 1.0] {
            let lung = ellipse(sx * 82.0, 20.0, 68.0, 130.0, sx * 0.1);
            b.poly(&lung, hex(0x3a2a30));
            b.path(&lung, true, 2.0, hex(0x8a6a70));
            for j in 0..6 {
                b.disc(
                    sx * (66.0 + (j % 3) as f32 * 18.0),
                    -50.0 + j as f32 * 34.0,
                    26.0,
                    rgba(150, 160, 172, 0.85 * left),
                );
            }
        }
        for j in 0..4 {
            let jf = j as f32;
            let rib = ellipse_arc(0.0, -70.0 + jf * 52.0, 170.0 - jf * 6.0, 24.0, 0.15, PI - 0.15);
            b.path(&rib, false, 4.0, hex(0x2c3f57));
        }
        // The breath gauge: it climbs green with up chevrons on the in-breath (the moment), falls hatched grey on the out.
        let (gx, gy, gw, gh) = (wx + ww - 150.0, wy + 96.0, 70.0, 270.0);
        g.round(gx, gy, gw, gh, 14.0, Some(hex(0x101824)), Some((2.0, c::LINE)));
        let fh = (gh - 8.0) * lv;
        if !w.done {
            let (fx, fy) = (gx + 4.0, gy + gh - 4.0 - fh);
            g.round(fx, fy, gw - 8.0, fh, 10.0, Some(if up { c::OK } else { hex(0x2a3446) }), None);
            if !up && fh > 2.0 {
                g.hatch(fx, fy, gw - 8.0, fh, hex(0x4a5468));
            }
            let cy = gy + gh - 4.0 - fh;
            let mx = gx + gw / 2.0;
            if up {
                for j in 0..2 {
                    let yy = cy - 22.0 - j as f32 * 22.0;
                    g.path_round(&[[mx - 18.0, yy + 10.0], [mx, yy - 6.0], [mx + 18.0, yy + 10.0]], 6.0, c::OK);
                }
            } else {
                let yy = cy + 26.0;
                g.path_round(&[[mx - 18.0, yy - 10.0], [mx, yy + 6.0], [mx + 18.0, yy - 10.0]], 6.0, c::DIM);
            }
            if self.hint > 0.0 {
                g.round(
                    gx - 4.0,
                    gy - 4.0,
                    gw + 8.0,
                    gh + 8.0,
                    16.0,
                    None,
                    Some((4.0, rgba(232, 238, 246, self.hint))),
                );
            }
        }
        // A pip per puff needed.
        for j in 0..w.need {
            let px = gx + gw / 2.0 - (w.need as f32 - 1.0) * 24.0 / 2.0 + j as f32 * 24.0;
            let py = gy + gh + 30.0;
            if j < w.puffs || w.done {
                g.disc(px, py, 9.0, c::ACCENT);
            } else {
                g.ring(px, py, 8.0, hex(0x3a4658), 3.0);
            }
        }
        if w.mist > 0.0 {
            for j in 0..16 {
                let jf = j as f32;
                g.disc(
                    MOUTH[0] + (jf * 1.7).sin() * 30.0 * (1.0 - w.mist),
                    MOUTH[1] - 50.0 + (1.0 - w.mist) * 140.0 + jf * 7.0,
                    10.0,
                    rgba(159, 224, 255, 0.55 * w.mist),
                );
            }
        }
    }

    fn close_up(&self, g: &Pen, w: &Wound) {
        let [wx, wy, ww, wh] = WORK;
        let live = !w.done && !self.waiting && self.tool == need(w);
        let k = g.clip(wx, wy, ww, wh);
        match w.kind {
            Kind::Cut => {
                self.skin(&k);
                if let Some(tr) = &w.tr {
                    self.draw_seal(&k, tr, w.done, hex(0x5a0f18), hex(0xff6b78), live);
                }
            }
            Kind::Burn => self.draw_burn(&k, w, live),
            Kind::Fracture => self.draw_fracture(&k, w, live),
            Kind::Bleed => self.draw_bleed(&k, w, live),
            Kind::Smoke => self.draw_smoke(&k, w),
        }
        // Effects: the tool working, the press landing.
        for q in &self.pings {
            k.ring(q.x, q.y, q.r * (0.6 + q.t * 2.0), q.col, (4.0 * (1.0 - q.t / 0.35)).max(0.1));
        }
        for f in &self.fx {
            k.alpha((f.life / f.max).max(0.0)).disc(f.x, f.y, f.r, f.col);
        }
        if let Some(([x, y], tt)) = self.tip {
            if tt > 0.0 {
                k.disc(x, y, 16.0, rgba(191, 239, 255, 0.35));
                k.disc(x, y, 7.0, hex(0xf2fbff));
            }
        }
        if let Some(([x, y], tt)) = self.cross {
            if tt > 0.0 {
                cross_mark(&k, x, y, 22.0, (tt * 2.0).min(1.0));
            }
        }
        corner_masks(g, WORK, 16.0, hex(0x070b12));
        g.round(wx, wy, ww, wh, 16.0, None, Some((2.0, c::LINE)));
        // The wound's name and size, top left; how far along, top right.
        g.round(wx + 14.0, wy + 14.0, 168.0, 40.0, 20.0, Some(rgba(4, 6, 10, 0.78)), None);
        g.text(w.kind.name(), wx + 34.0, wy + 34.0, 22.0, if w.done { c::OK } else { c::FG }, Align::Left);
        for j in 0..=w.size {
            g.disc(wx + 138.0 + j as f32 * 13.0, wy + 34.0, 4.5, if w.done { c::OK } else { c::AMBER });
        }
        let (rx, ry) = (wx + ww - 48.0, wy + 48.0);
        g.disc(rx, ry, 32.0, rgba(4, 6, 10, 0.78));
        if w.done {
            check(g, rx, ry, 24.0);
        } else {
            g.ring(rx, ry, 22.0, hex(0x2a3446), 8.0);
            let f = self.progress(w);
            if f > 0.0 {
                g.arc(rx, ry, 22.0, -FRAC_PI_2, -FRAC_PI_2 + TAU * f, 8.0, c::OK);
            }
        }
    }
}

impl Game for Medic {
    fn id(&self) -> &'static str {
        "medic"
    }

    fn knobs(&self) -> &'static [&'static str] {
        &[
            "band_px",
            "tip_px",
            "seal_done_frac",
            "brush_px",
            "gel_done_frac",
            "snap_in_px",
            "snap_out_px",
            "yank_px",
            "spurt_s",
            "inhale_frac",
        ]
    }

    fn step(&mut self, cx: &mut Ctx) {
        self.k = Feel {
            band: cx.knob("band_px"),
            tip: cx.knob("tip_px"),
            seal_done: cx.knob("seal_done_frac"),
            brush: cx.knob("brush_px"),
            gel_done: cx.knob("gel_done_frac"),
            snap_in: cx.knob("snap_in_px"),
            snap_out: cx.knob("snap_out_px"),
            yank: cx.knob("yank_px"),
            spurt: cx.knob("spurt_s"),
            inhale: cx.knob("inhale_frac"),
        };
        self.start_hp = cx.data.job.as_ref().map_or(30.0, |j| j.start);
        if self.wounds.is_empty() {
            let mut r = cx.dice();
            self.wounds = patient(&mut r);
        }
        let index = i64::from(cx.round);
        if index == self.index {
            // Three slips: the round restarts, and the wounds not yet treated open again.
            for w in &mut self.wounds {
                if !w.done {
                    reset(w);
                }
            }
        }
        self.index = index;
        self.waiting = false;
        self.gest = None;
        if self.wounds[self.cur].done {
            self.cur = self.next_open();
        }
    }

    fn update(&mut self, cx: &mut Ctx, dt: f32, input: &Input) {
        self.t += dt;
        self.flinch = (self.flinch - dt * 1.5).max(0.0);
        self.hint = (self.hint - dt * 2.0).max(0.0);
        if let Some(c) = &mut self.cross {
            c.1 -= dt;
        }
        if let Some(t) = &mut self.tip {
            t.1 -= dt;
        }
        for w in &mut self.wounds {
            w.mist = (w.mist - dt * 1.2).max(0.0);
        }
        for f in &mut self.fx {
            f.x += f.vx * dt;
            f.y += f.vy * dt;
            f.vy += f.g * dt;
            f.vx *= 0.96;
            f.vy *= 0.96;
            f.life -= dt;
        }
        self.fx.retain(|f| f.life > 0.0);
        for q in &mut self.pings {
            q.t += dt;
        }
        self.pings.retain(|q| q.t < 0.35);
        let p = self.pointer(dt, input);
        for (i, k) in TOOL_KEYS.iter().enumerate() {
            if input.hit(*k) {
                self.tool = i;
            }
        }
        if input.hit(Key::Q) {
            self.tool = (self.tool + 4) % 5;
        }
        if input.hit(Key::E) {
            self.tool = (self.tool + 1) % 5;
        }
        let wi = self.cur;
        if p.pressed {
            self.gest = None;
            // The tray and the chart take their press; neither ever reaches the wound.
            if p.y >= TRAY_Y - 4.0 && p.y <= TRAY_Y + TRAY_H {
                // The slot under the finger, or the nearer one in the 11 px gap between two.
                let i = ((p.x - TRAY_X + GAP / 2.0) / (SLOT + GAP)).floor();
                if p.x >= TRAY_X - GAP && (0.0..5.0).contains(&i) {
                    self.tool = i as usize;
                }
            } else if p.x < CHART[0] + CHART[2] + 8.0 {
                let pts: Vec<Option<[f32; 2]>> = self.wounds.iter().map(|q| Some(chart_xy(q))).collect();
                if let Some(best) = crate::kit::nearest(&pts, p.x, p.y, 40.0) {
                    if !self.wounds[best].done && !self.waiting {
                        self.cur = best;
                        self.gest = None;
                    }
                }
            } else if in_work(p.x, p.y) && !self.wounds[wi].done && !self.waiting {
                let ok = self.tool == need(&self.wounds[wi]);
                self.gest = Some(ok);
                if !ok {
                    cx.say("Wrong tool");
                    self.cross = Some(([p.x, p.y], 0.8));
                }
            }
        }
        if !p.down {
            self.gest = None;
        }
        if self.waiting || self.wounds[wi].done {
            return;
        }
        let on = self.gest == Some(true) && self.tool == need(&self.wounds[wi]) && p.down;
        let mut w = std::mem::take(&mut self.wounds[wi]);
        let out = self.treat(&mut w, &p, dt, on);
        self.wounds[wi] = w;
        match out {
            Out::Slip => {
                self.flinch = 1.0;
                self.gest = None;
                cx.fumble("The patient flinches: 2 HP");
            }
            Out::Done => {
                self.wounds[wi].done = true;
                self.waiting = true;
                self.gest = None;
                cx.step_done();
                self.burst([CX, CY], 30, c::OK, 320.0, 0.6, Spray::default());
                self.ping(CX, CY, c::OK, 120.0);
            }
            Out::Busy => {}
        }
    }

    fn draw(&self, g: &Pen, cx: &Ctx, _t: f32, _input: &Input) {
        g.rect(0.0, BAR_H, W, H - BAR_H, hex(0x070b12));
        if self.wounds.is_empty() {
            return;
        }
        self.vitals(g, cx.value);
        let shake = if self.flinch > 0.0 { (self.t * 50.0).sin() * 5.0 * self.flinch } else { 0.0 };
        let [chx, chy, chw, chh] = CHART;
        g.panel(chx, chy, chw, chh, 16.0, hex(0x0b1018), c::LINE);
        let f = g.translate(shake, 0.0);
        figure(&f);
        for (i, w) in self.wounds.iter().enumerate() {
            self.marker(&f, w, i);
        }
        let w = &self.wounds[self.cur];
        // A leader from the chosen wound on the chart to its close-up.
        let [mx, my] = chart_xy(w);
        g.dashed(&[[mx + 30.0, my], [WORK[0], WORK[1] + 34.0]], 2.0, rgba(242, 160, 70, 0.45), 6.0, 6.0);
        self.close_up(&g.translate(shake * 0.5, 0.0), w);
        self.tray(g);
        // The tool in hand at the cursor over the close-up: bright when it works here, greyed with a red cross when not.
        let p = self.cursor();
        if let Some([px, py]) = p {
            if in_work(px, py) && !w.done && !self.waiting {
                let ok = self.tool == need(w);
                let q = g.translate(px + 30.0, py - 30.0).scale(0.85, 0.85);
                q.disc(0.0, 0.0, 34.0, if ok { rgba(4, 6, 10, 0.35) } else { rgba(4, 6, 10, 0.6) });
                let (col, tip) = if ok { (hex(0xe8eef6), c::ACCENT) } else { (hex(0x4a5466), hex(0x4a5466)) };
                tool_icon(&q, self.tool, 0.0, 0.0, col, tip);
                if ok {
                    g.ring(px, py, 6.0, c::FG, 2.0);
                } else {
                    cross_mark(g, px, py, 13.0, 1.0);
                }
            }
            if self.pad {
                g.line(px - 14.0, py, px + 14.0, py, 2.0, c::FG);
                g.line(px, py - 14.0, px, py + 14.0, 2.0, c::FG);
            }
        }
    }

    fn hand(&mut self, _cx: &Ctx, _t: f32) -> Input {
        if self.waiting || self.wounds.is_empty() {
            return Input::default();
        }
        let wi = self.cur;
        if self.hand_wi != wi {
            self.hand_wi = wi;
            self.hand_route.clear();
            self.hand_down = false;
        }
        let w = &self.wounds[wi];
        if w.done {
            return Input::default();
        }
        let tool = need(w);
        if self.tool != tool {
            // Pick the tool this wound takes now, letting go of anything held; a key is pressed, then every key is
            // let go for a frame, as a hand would.
            self.hand_route.clear();
            self.hand_down = false;
            self.hand_keyed = !self.hand_keyed;
            if !self.hand_keyed {
                return Input::default();
            }
            return Input::default().key(TOOL_KEYS[tool], true);
        }
        self.hand_keyed = false;
        // What the clock will read when this frame is played.
        let t = self.t + HAND_DT;
        let sweep = |me: &mut Self, route: Vec<[f32; 2]>, speed: f32| -> Input {
            if me.hand_route.is_empty() {
                me.hand_route = route;
                me.hand_u = 0.0;
                me.hand_down = true;
                let [x, y] = me.hand_route[0];
                return Input::hold(x, y, true);
            }
            me.hand_u += speed;
            let len = poly_len(&me.hand_route);
            if me.hand_u > len {
                // The route is run and the wound still open: let go, and go again.
                let [x, y] = me.hand_route[me.hand_route.len() - 1];
                me.hand_route.clear();
                me.hand_down = false;
                return Input::release(x, y);
            }
            let [x, y, _] = poly_at(&me.hand_route, me.hand_u / len.max(1e-6));
            Input::hold(x, y, false)
        };
        let tap = |me: &mut Self, ready: bool| -> Input {
            if me.hand_down {
                me.hand_down = false;
                return Input::at(CX, CY);
            }
            if ready {
                me.hand_down = true;
                return Input::hold(CX, CY, true);
            }
            Input::at(CX, CY)
        };
        match w.kind {
            Kind::Cut => {
                let route = w.tr.as_ref().map(|tr| tr.pts.clone()).unwrap_or_default();
                sweep(self, route, 6.0)
            }
            Kind::Bleed if w.clamped => {
                let route = w.tr.as_ref().map(|tr| tr.pts.clone()).unwrap_or_default();
                sweep(self, route, 6.0)
            }
            Kind::Bleed => {
                // Clamp in the gap, well clear of the next spurt.
                let ph = (t + w.ph) % w.period;
                let ready = ph > self.k.spurt + 0.06 && ph < w.period - 0.08;
                tap(self, ready)
            }
            Kind::Smoke => {
                // Puff early in a fresh in-breath.
                let f = frac((t + w.ph) / w.breath_s);
                let n = ((t + w.ph) / w.breath_s).floor() as i64;
                let ready = f > 0.05 && f < self.k.inhale - 0.08 && n != w.last;
                tap(self, ready)
            }
            Kind::Burn => {
                // Paint the burn in rows 60 px apart, the brush reaching 50 px either side.
                let span = w.r * 1.3;
                let (x0, x1) = (CX - span, CX + span);
                let mut route = Vec::new();
                let mut y = CY - span + 25.0;
                let mut right = true;
                while y <= CY + span {
                    if right {
                        route.extend_from_slice(&[[x0, y], [x1, y]]);
                    } else {
                        route.extend_from_slice(&[[x1, y], [x0, y]]);
                    }
                    right = !right;
                    y += 60.0;
                }
                sweep(self, route, 8.0)
            }
            Kind::Fracture => {
                let (e, b, held) = (w.e, w.bs, w.held);
                if !self.hand_down {
                    // Grab the loose end.
                    self.hand_down = true;
                    self.hand_pos = e;
                    return Input::hold(e[0], e[1], true);
                }
                if !held {
                    self.hand_down = false;
                    return Input::release(self.hand_pos[0], self.hand_pos[1]);
                }
                // Carry it to the socket, then hold it there while it knits.
                let d = dist(self.hand_pos, b);
                let k = if d > 0.0 { (8.0 / d).min(1.0) } else { 0.0 };
                self.hand_pos = [
                    self.hand_pos[0] + (b[0] - self.hand_pos[0]) * k,
                    self.hand_pos[1] + (b[1] - self.hand_pos[1]) * k,
                ];
                Input::hold(self.hand_pos[0], self.hand_pos[1], false)
            }
        }
    }

    fn guide_now(&self) -> Option<usize> {
        let w = self.wounds.get(self.cur)?;
        if w.done {
            return Some(0);
        }
        let n = need(w);
        Some(if self.tool != n {
            1
        } else if n >= 3 {
            3
        } else {
            2
        })
    }
}

// ---------------------------------------------------------------------------------------------------- the patient

fn frac(x: f32) -> f32 {
    x - x.floor()
}

fn dist(a: [f32; 2], b: [f32; 2]) -> f32 {
    (a[0] - b[0]).hypot(a[1] - b[1])
}

fn ecg(p: f32) -> f32 {
    let q = |m: f32, w: f32| (-((p - m) / w).powi(2)).exp();
    0.12 * q(0.12, 0.03) - 0.15 * q(0.27, 0.01) + q(0.3, 0.014) - 0.25 * q(0.33, 0.012) + 0.3 * q(0.55, 0.05)
}

fn in_work(x: f32, y: f32) -> bool {
    (WORK[0]..=WORK[0] + WORK[2]).contains(&x) && (WORK[1]..=WORK[1] + WORK[3]).contains(&y)
}

fn chart_xy(w: &Wound) -> [f32; 2] {
    [BX + w.u * S, BY + w.v * S]
}

/// The tool a wound takes now (a bleed: the clamp, then the sealer).
fn need(w: &Wound) -> usize {
    match w.kind {
        Kind::Cut => 0,
        Kind::Burn => 1,
        Kind::Fracture => 2,
        Kind::Bleed => {
            if w.clamped {
                0
            } else {
                3
            }
        }
        Kind::Smoke => 4,
    }
}

fn trail(r: &mut Dice, len: f32, ang: f32, amp0: f32) -> Trail {
    let amp = amp0 * (0.5 + 0.5 * r.f());
    let k = 1.0 + r.f();
    let ph = r.f() * TAU;
    let pts: Vec<[f32; 2]> = (0..=32)
        .map(|i| {
            let fi = i as f32;
            let a = -len / 2.0 + len * fi / 32.0;
            let o = amp * (PI * k * fi / 32.0 + ph).sin();
            [CX + a * ang.cos() - o * ang.sin(), CY + a * ang.sin() + o * ang.cos()]
        })
        .collect();
    let mut cum = vec![0.0];
    for i in 1..pts.len() {
        cum.push(cum[i - 1] + dist(pts[i - 1], pts[i]));
    }
    let total = cum[cum.len() - 1];
    let n = (total / BIN).ceil() as usize;
    Trail { pts, cum, len: total, n, seal: vec![-1.0; n], count: 0, last_s: None }
}

/// The nearest point on a line: its distance off, and its arc length along.
fn project(tr: &Trail, x: f32, y: f32) -> (f32, f32) {
    let mut best = (1e9f32, 0.0f32);
    for i in 1..tr.pts.len() {
        let ([ax, ay], [bx, by]) = (tr.pts[i - 1], tr.pts[i]);
        let l = tr.cum[i] - tr.cum[i - 1];
        let t = if l > 0.0 { (((x - ax) * (bx - ax) + (y - ay) * (by - ay)) / (l * l)).clamp(0.0, 1.0) } else { 0.0 };
        let d = (x - (ax + (bx - ax) * t)).hypot(y - (ay + (by - ay) * t));
        if d < best.0 {
            best = (d, tr.cum[i - 1] + l * t);
        }
    }
    best
}

fn trail_at(tr: &Trail, sv: f32) -> [f32; 2] {
    for i in 1..tr.pts.len() {
        if tr.cum[i] >= sv {
            let l = tr.cum[i] - tr.cum[i - 1];
            let t = (sv - tr.cum[i - 1]) / if l > 0.0 { l } else { 1.0 };
            let ([ax, ay], [bx, by]) = (tr.pts[i - 1], tr.pts[i]);
            return [ax + (bx - ax) * t, ay + (by - ay) * t];
        }
    }
    tr.pts[tr.pts.len() - 1]
}

fn seg_dist(p: [f32; 2], a: [f32; 2], b: [f32; 2]) -> f32 {
    let l2 = (b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2);
    let t = if l2 > 0.0 {
        (((p[0] - a[0]) * (b[0] - a[0]) + (p[1] - a[1]) * (b[1] - a[1])) / l2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    dist(p, [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t])
}

fn burn_r(w: &Wound, th: f32) -> f32 {
    w.r * (1.0 + 0.16 * (3.0 * th + w.a).sin() + 0.08 * (5.0 * th + w.b).sin())
}

fn in_burn(w: &Wound, x: f32, y: f32, pad: f32) -> bool {
    let th = (y - CY).atan2(x - CX);
    (x - CX).hypot(y - CY) < burn_r(w, th) + pad
}

/// The loose bone's angle: it tilts the further it is from its socket, and lies straight in it.
fn bone_ang(w: &Wound) -> f32 {
    if w.snap || w.done {
        0.0
    } else {
        w.tilt * (dist(w.e, w.bs) / 80.0).min(1.0)
    }
}

fn bone_far(w: &Wound) -> [f32; 2] {
    let a = bone_ang(w);
    [w.e[0] + a.cos() * BONE, w.e[1] + a.sin() * BONE]
}

fn clamp_xy(w: &Wound) -> [f32; 2] {
    [CX - w.th.cos() * 170.0, CY - w.th.sin() * 170.0]
}

fn wound(kind: Kind, size: usize, r: &mut Dice, u: f32, v: f32, la: f32) -> Wound {
    let mut w = Wound { kind, u, v, la, size, ..Wound::default() };
    let sz = size as f32;
    match kind {
        Kind::Cut => {
            let ang = (r.f() - 0.5) * 0.5;
            w.tr = Some(trail(r, 420.0 + 110.0 * sz, ang, 34.0));
        }
        Kind::Burn => {
            w.r = 100.0 + 22.0 * sz;
            w.a = r.f() * TAU;
            w.b = r.f() * TAU;
            let span = w.r * 1.3;
            let mut y = CY - span;
            while y < CY + span {
                let mut x = CX - span;
                while x < CX + span {
                    if in_burn(&w, x, y, 0.0) {
                        w.cells.push([x, y]);
                    }
                    x += 14.0;
                }
                y += 14.0;
            }
        }
        Kind::Fracture => {
            w.bs = [CX - 80.0, CY];
            // The loose end sits above or below the socket, and leans back across it.
            let side = if r.f() < 0.5 { -1.0 } else { 1.0 };
            let ex = w.bs[0] + 110.0 + 50.0 * r.f();
            let ey = w.bs[1] + side * (70.0 + 40.0 * r.f());
            w.e0 = [ex, ey];
            w.tilt = -side * (0.2 + 0.15 * r.f());
            // Seconds held in the socket to knit.
            w.knit_t = 1.4 + 0.3 * sz;
        }
        Kind::Bleed => {
            w.period = 1.6 - 0.15 * sz;
            w.ph = r.f() * 2.0;
            w.th = (r.f() - 0.5) * 0.5;
            w.tr = Some(trail(r, 320.0, w.th, 8.0));
        }
        Kind::Smoke => {
            w.need = 2 + size;
            w.breath_s = 3.0 - 0.25 * sz;
            w.ph = r.f() * 3.0;
        }
    }
    reset(&mut w);
    w
}

/// Four wounds of four different kinds, each at a spot of its kind clear of the others.
fn patient(r: &mut Dice) -> Vec<Wound> {
    let mut kinds = KINDS;
    r.shuffle(&mut kinds);
    let mut used: Vec<[f32; 2]> = Vec::new();
    kinds[..4]
        .iter()
        .map(|&kind| {
            let all = kind.anchors();
            let spots: Vec<[f32; 3]> =
                all.iter().copied().filter(|s| used.iter().all(|u| (s[0] - u[0]).hypot(s[1] - u[1]) > 7.0)).collect();
            let k = (r.f() * spots.len() as f32) as usize;
            let [u, v, la] = spots.get(k).copied().unwrap_or(all[0]);
            used.push([u, v]);
            let size = ((r.f() * 3.0) as usize).min(2);
            wound(kind, size, r, u, v, la)
        })
        .collect()
}

fn reset(w: &mut Wound) {
    if let Some(tr) = &mut w.tr {
        tr.seal.iter_mut().for_each(|s| *s = -1.0);
        tr.count = 0;
        tr.last_s = None;
    }
    match w.kind {
        Kind::Burn => {
            w.cov = vec![false; w.cells.len()];
            w.cov_n = 0;
            w.lp = None;
            w.film = vec![false; FILM_C * FILM_R];
        }
        Kind::Fracture => {
            w.e = w.e0;
            w.held = false;
            w.snap = false;
            w.seated = false;
            w.knit = 0.0;
        }
        Kind::Bleed => w.clamped = false,
        Kind::Smoke => {
            w.puffs = 0;
            w.last = -1;
            w.mist = 0.0;
        }
        Kind::Cut => {}
    }
}

// ---------------------------------------------------------------------------------------------------- pictures

/// An ellipse's outline round (x, y), turned by `rot`.
fn ellipse(x: f32, y: f32, rx: f32, ry: f32, rot: f32) -> Vec<[f32; 2]> {
    let mut v = ellipse_arc(x, y, rx, ry, 0.0, TAU);
    v.pop();
    if rot != 0.0 {
        let (s, c) = rot.sin_cos();
        for p in &mut v {
            let (dx, dy) = (p[0] - x, p[1] - y);
            *p = [x + dx * c - dy * s, y + dx * s + dy * c];
        }
    }
    v
}

/// Part of an axis-aligned ellipse from `a0` to `a1`.
fn ellipse_arc(x: f32, y: f32, rx: f32, ry: f32, a0: f32, a1: f32) -> Vec<[f32; 2]> {
    let n = 40;
    (0..=n)
        .map(|i| {
            let a = a0 + (a1 - a0) * i as f32 / n as f32;
            [x + a.cos() * rx, y + a.sin() * ry]
        })
        .collect()
}

/// A circle's outline, closed (for a dashed ring).
fn circle(x: f32, y: f32, r: f32) -> Vec<[f32; 2]> {
    Pen::arc_points(x, y, r, 0.0, TAU)
}

/// A quadratic Bezier from p0 through control p1 to p2, n pieces.
fn qbez(p0: [f32; 2], p1: [f32; 2], p2: [f32; 2], n: usize) -> Vec<[f32; 2]> {
    (0..=n)
        .map(|i| {
            let t = i as f32 / n as f32;
            let u = 1.0 - t;
            [u * u * p0[0] + 2.0 * u * t * p1[0] + t * t * p2[0], u * u * p0[1] + 2.0 * u * t * p1[1] + t * t * p2[1]]
        })
        .collect()
}

/// A wide see-through line with round ends that do not double up over it: the line, and a half disc at each end.
fn cap_line(g: &Pen, pts: &[[f32; 2]], w: f32, col: Color32) {
    if pts.len() < 2 {
        return;
    }
    g.path(pts, false, w, col);
    let r = w / 2.0;
    let (a, b) = (pts[0], pts[1]);
    let ang = (a[1] - b[1]).atan2(a[0] - b[0]);
    g.sector(a[0], a[1], 0.0, r, ang - FRAC_PI_2, ang + FRAC_PI_2, col);
    let (a, b) = (pts[pts.len() - 1], pts[pts.len() - 2]);
    let ang = (a[1] - b[1]).atan2(a[0] - b[0]);
    g.sector(a[0], a[1], 0.0, r, ang - FRAC_PI_2, ang + FRAC_PI_2, col);
}

/// A polygon shaded from `inner` at `centre` to `outer` at radius `r` (a radial gradient clipped to it).
fn radial_poly(g: &Pen, centre: [f32; 2], pts: &[[f32; 2]], inner: Color32, outer: Color32, r: f32) {
    let at = |p: [f32; 2]| mix(inner, outer, (dist(p, centre) / r).clamp(0.0, 1.0));
    for i in 0..pts.len() {
        let (p0, p1) = (pts[i], pts[(i + 1) % pts.len()]);
        g.quad([centre, centre, p0, p1], [inner, inner, at(p0), at(p1)]);
    }
}

/// Round off a clipped rectangle's corners by painting the ground over them.
fn corner_masks(g: &Pen, [x, y, w, h]: [f32; 4], r: f32, col: Color32) {
    for (corner, centre, a0) in [
        ([x, y], [x + r, y + r], PI),
        ([x + w, y], [x + w - r, y + r], 1.5 * PI),
        ([x + w, y + h], [x + w - r, y + h - r], 0.0),
        ([x, y + h], [x + r, y + h - r], FRAC_PI_2),
    ] {
        let mut pts = vec![corner];
        pts.extend(Pen::arc_points(centre[0], centre[1], r, a0, a0 + FRAC_PI_2));
        g.poly(&pts, col);
    }
}

fn figure(g: &Pen) {
    let p = |u: f32, v: f32| [BX + u * S, BY + v * S];
    let limbs: [([f32; 2], [f32; 2], f32); 9] = [
        ([-14.0, 16.0], [-17.0, 32.0], 5.5),
        ([-17.0, 32.0], [-19.0, 47.0], 4.5),
        ([14.0, 16.0], [17.0, 32.0], 5.5),
        ([17.0, 32.0], [19.0, 47.0], 4.5),
        ([-5.5, 51.0], [-6.5, 74.0], 7.5),
        ([-6.5, 74.0], [-7.0, 95.0], 5.5),
        ([5.5, 51.0], [6.5, 74.0], 7.5),
        ([6.5, 74.0], [7.0, 95.0], 5.5),
        ([0.0, 11.0], [0.0, 16.0], 5.0),
    ];
    let torso: Vec<[f32; 2]> = [
        [-13.5, 15.0],
        [13.5, 15.0],
        [12.0, 30.0],
        [10.5, 46.0],
        [11.0, 52.0],
        [-11.0, 52.0],
        [-10.5, 46.0],
        [-12.0, 30.0],
    ]
    .iter()
    .map(|q| p(q[0], q[1]))
    .collect();
    for (col, o) in [(hex(0x4d6683), 4.0), (hex(0x1b293b), 0.0)] {
        for (a, b, w) in limbs {
            g.path_round(&[p(a[0], a[1]), p(b[0], b[1])], w * S + o, col);
        }
        g.poly(&torso, col);
        if o > 0.0 {
            g.path(&torso, true, o, col);
        }
        let [hx, hy] = p(0.0, 6.0);
        g.disc(hx, hy, 6.0 * S + o / 2.0, col);
        for sx in [-1.0f32, 1.0] {
            let [x, y] = p(sx * 19.6, 50.0);
            g.disc(x, y, 2.6 * S + o / 2.0, col);
            let [x, y] = p(sx * 8.2, 97.0);
            g.poly(&ellipse(x, y, 3.4 * S + o / 2.0, 1.6 * S + o / 2.0, 0.0), col);
        }
    }
}

/// A green disc with a tick: treated.
fn check(g: &Pen, x: f32, y: f32, r: f32) {
    g.disc(x, y, r, c::OK);
    g.path_round(
        &[[x - r * 0.5, y], [x - r * 0.12, y + r * 0.42], [x + r * 0.55, y - r * 0.42]],
        (r * 0.25).max(2.5),
        hex(0x0b111b),
    );
}

/// A red cross with a dark rim: this tool does nothing here.
fn cross_mark(g: &Pen, x: f32, y: f32, r: f32, a: f32) {
    let q = g.alpha(a);
    for (w, col) in [(10.0, rgba(4, 6, 10, 0.8)), (5.0, c::DANGER)] {
        q.path_round(&[[x - r, y - r], [x + r, y + r]], w, col);
        q.path_round(&[[x + r, y - r], [x - r, y + r]], w, col);
    }
}

fn tool_icon(g: &Pen, i: usize, x: f32, y: f32, col: Color32, tip: Color32) {
    let q = g.translate(x, y);
    match i {
        0 => {
            let r = q.rotate(-0.6);
            r.round(-30.0, -8.0, 48.0, 16.0, 6.0, Some(col), None);
            r.rect(-20.0, -3.0, 26.0, 6.0, hex(0x1d2738));
            r.poly(&[[18.0, -6.0], [32.0, 0.0], [18.0, 6.0]], tip);
        }
        1 => {
            q.poly(&[[-26.0, -12.0], [14.0, -9.0], [14.0, 9.0], [-26.0, 12.0]], col);
            q.round(14.0, -7.0, 14.0, 14.0, 3.0, Some(if tip == c::ACCENT { hex(0x7fe6d2) } else { tip }), None);
            q.rect(-28.0, -13.0, 5.0, 26.0, hex(0x3a4658));
        }
        2 => {
            q.path_round(&[[-20.0, -18.0], [-20.0, 12.0], [20.0, 12.0], [20.0, -18.0]], 7.0, col);
            q.path_round(&[[-14.0, -16.0], [14.0, -16.0]], 3.0, if tip == c::ACCENT { c::LILAC } else { tip });
        }
        3 => {
            q.ring(-22.0, 10.0, 7.0, col, 4.0);
            q.ring(-8.0, 16.0, 7.0, col, 4.0);
            q.path_round(&[[-17.0, 5.0], [24.0, -16.0]], 4.0, col);
            q.path_round(&[[-3.0, 11.0], [26.0, -10.0]], 4.0, col);
        }
        _ => {
            q.round(-10.0, -22.0, 20.0, 34.0, 6.0, Some(col), None);
            q.round(-4.0, 6.0, 30.0, 14.0, 5.0, Some(col), None);
            q.rect(-7.0, -28.0, 14.0, 7.0, tip);
        }
    }
}

/// A bone from its knuckle end (x0, y0) to its break (x1, y1).
fn bone(g: &Pen, [x0, y0]: [f32; 2], [x1, y1]: [f32; 2]) {
    let (col, rim) = (hex(0xd9d2bf), hex(0x8a8170));
    g.path_round(&[[x0, y0], [x1, y1]], BONE_W + 6.0, rim);
    g.path_round(&[[x0, y0], [x1, y1]], BONE_W, col);
    let a = (y1 - y0).atan2(x1 - x0);
    let (nx, ny) = (-a.sin(), a.cos());
    g.disc(x0 + nx * 16.0, y0 + ny * 16.0, 27.0, rim);
    g.disc(x0 - nx * 16.0, y0 - ny * 16.0, 27.0, rim);
    g.disc(x0 + nx * 16.0, y0 + ny * 16.0, 24.0, col);
    g.disc(x0 - nx * 16.0, y0 - ny * 16.0, 24.0, col);
    // The break: a jagged end at (x1, y1).
    let (ca, sa) = (a.cos(), a.sin());
    g.path(
        &[
            [x1 + nx * 20.0, y1 + ny * 20.0],
            [x1 + nx * 7.0 + ca * 6.0, y1 + ny * 7.0 + sa * 6.0],
            [x1 - nx * 4.0 - ca * 4.0, y1 - ny * 4.0 - sa * 4.0],
            [x1 - nx * 20.0 + ca * 5.0, y1 - ny * 20.0 + sa * 5.0],
        ],
        false,
        3.0,
        hex(0x5a5446),
    );
}

#[cfg(test)]
mod tests {
    use crate::games::tests::{fumble_check, plays_to_end};
    use crate::kit::Input;
    use egui::Key;

    #[test]
    fn a_steady_hand_treats_every_wound_with_no_slip() {
        plays_to_end("medic");
    }

    #[test]
    fn a_puff_on_the_out_breath_makes_the_patient_flinch() {
        // Tap the smoke on the chart (always at the chest, u 0, v 27), take the inhaler, then puff at every frame:
        // the first breath's out-breath comes, and a puff then is the slip.
        let mut n = 0;
        fumble_check("medic", move |_r| {
            n += 1;
            match n {
                1 => Input::hold(super::BX, super::BY + 27.0 * super::S, true),
                2 => Input::release(super::BX, super::BY + 27.0 * super::S),
                3 => Input::default().key(Key::Num5, true),
                _ => Input::hold(super::CX, super::CY, true),
            }
        });
    }
}
