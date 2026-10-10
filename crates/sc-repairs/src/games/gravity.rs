//! The gravity generator's repair, "field alignment" (repair-minigames design 2 and 6d), from
//! `docs/mockups/repairs/gravity.js`.
//!
//! The field over the deck is a 3D wave surface, a 24 x 24 wireframe in perspective seen from a fixed three-quarter
//! view above, its height the field's ripple, projected here by hand and drawn as flat quads and lines. The reference
//! shape (the field the generator should make) is a faint dashed ghost in the same space. Each live line is coloured
//! by how far it lies from the ghost there: green on it, amber off it, red and thick far off it. Three controls set
//! the live wave: Turn (a geared dial, the crests' direction), Shift (a horizontal slider, the phase) and Stretch (a
//! vertical slider, the wavelength, locked at level 1). Dragging on the surface turns it (sideways) and shifts it (up
//! and down). Keys and pad: the stick turns (x) and shifts (y), Q and E stretch, Tab or the action picks the ripple.
//!
//! One number drives the round: the RMS difference between the live surface and the ghost over the grid, over the
//! ghost's own RMS. Under `HOLD_ERR` the field holds and the hold ring on the deck fills; the round is played when it
//! fills. Above `SURGE_ERR` the field surges: the fumble, a jolt, loose bolts float, and the drift is kicked to a new
//! heading. All three settings drift, faster each level and in combat. From level 3 a second, smaller ripple joins
//! both the ghost and the live field, and two tabs pick which ripple the controls hold. A disabled generator's first
//! step fits the new field coil: carry it from the crate into the hub socket.

use std::collections::BTreeMap;
use std::f32::consts::{FRAC_PI_2, PI, TAU};

use egui::{Color32, Key};

use crate::kit::{Ctx, Dice, Game, Input, BAR_H, H, W};
use crate::pen::{c, hex, rgba, Align, Pen};

/// In the engine.
pub const PORTED: bool = true;

/// A fresh game.
pub fn new() -> Box<dyn Game> {
    Box::new(Gravity::new())
}

const SURGE_TEXT: &str = "Field surge: everyone near floats";

// ------------------------------------------------------------------------------------------------ the match
/// Grid cells a side (N + 1 vertices).
const N: usize = 24;
/// Vertices in the grid.
const NV: usize = (N + 1) * (N + 1);
/// Design 6d: under this the field holds.
const HOLD_ERR: f32 = 0.15;
/// Design 6d: above this the field surges.
const SURGE_ERR: f32 = 0.85;
/// The field's envelope over the deck, exp(-r^2 / ENV): a ripple round the hub.
const ENV: f32 = 1.4;
/// Radians of phase at the Shift slider's end stop.
const PH: f32 = 1.1;
/// Octaves of wavelength at the Stretch slider's end stop.
const OCT: f32 = 0.6;
/// The Turn dial turns 4 times for the crests' once: a fine dial.
const GEAR: f32 = 4.0;
/// Seconds after a surge before another can happen.
const COOL_S: f32 = 1.5;
/// Seconds of hold a surge knocks back.
const SURGE_HOLD_S: f32 = 1.0;
/// The ripples, big then small: relative height, and the wavelength at the Stretch slider's centre, deck units.
const RIPPLES: [(f32, f32); 2] = [(1.0, 1.15), (0.45, 0.56)];
/// Drift of turn (rad/s), shift and stretch (slider units/s) at level 1, and each one's bound.
const DRIFT_T: (f32, f32) = (0.045, 0.45);
const DRIFT_S: (f32, f32) = (0.09, 0.45);
const DRIFT_L: (f32, f32) = (0.07, 0.4);
/// Drift in combat, times.
const COMBAT_SPEEDUP: f32 = 1.5;
/// The small ripple drifts slower, and not in stretch: two to nurse, not two to chase.
const SMALL_DRIFT: f32 = 0.35;
/// Key rates, per second at a full stick.
const KEY_TURN: f32 = 0.35;
const KEY_SHIFT: f32 = 0.7;
const KEY_STRETCH: f32 = 0.6;
/// Per canvas px dragged on the surface.
const DRAG_TURN: f32 = 0.0025;
const DRAG_SHIFT: f32 = 0.006 / PH;

// ------------------------------------------------------------------------------------------------ the screen
const GRAPH: [f32; 4] = [16.0, 84.0, 880.0, 548.0];
const DIAL: (f32, f32, f32) = (1036.0, 252.0, 104.0);
const SHIFT: [f32; 4] = [120.0, 650.0, 700.0, 56.0];
const STRETCH: [f32; 4] = [1188.0, 112.0, 60.0, 470.0];
const TABS: [[f32; 4]; 2] = [[924.0, 452.0, 108.0, 84.0], [1044.0, 452.0, 108.0, 84.0]];
const CRATE: [f32; 4] = [940.0, 548.0, 200.0, 150.0];
/// Extra px round every control, so a finger finds it.
const HIT: f32 = 34.0;

// ------------------------------------------------------------------------------------------------ the view
const YAW: f32 = -0.66;
const ELEV: f32 = 0.7;
const CAM: f32 = 4.3;
const FOCAL: f32 = 840.0;
const VX: f32 = 456.0;
const VY: f32 = 292.0;
/// Deck units of height per unit of ripple.
const ZS: f32 = 0.32;
const DECK_Z: f32 = -0.4;
const RING_R: f32 = 1.44;
const PLATE_R: f32 = 1.56;
/// Loose bolts on the deck plate: they float in a surge.
const BOLTS: [[f32; 2]; 6] = [[1.22, -0.35], [1.2, 0.45], [-0.5, -1.22], [0.35, -1.22], [-1.22, 0.1], [0.2, 1.24]];

/// Deck coordinates (u right, v away, z up) to canvas px, and the depth.
fn proj(u: f32, v: f32, z: f32) -> [f32; 3] {
    let (sy, cy) = YAW.sin_cos();
    let (se, ce) = ELEV.sin_cos();
    let (x1, y1) = (u * cy - v * sy, u * sy + v * cy);
    let depth = CAM + y1 * ce - z * se;
    let up = z * ce + y1 * se;
    [VX + FOCAL * x1 / depth, VY - FOCAL * up / depth, depth]
}

fn xy(p: [f32; 3]) -> [f32; 2] {
    [p[0], p[1]]
}

fn wrap(a: f32) -> f32 {
    a.sin().atan2(a.cos())
}

/// JavaScript's `Math.sign`: zero for zero.
fn sign(x: f32) -> f32 {
    if x > 0.0 {
        1.0
    } else if x < 0.0 {
        -1.0
    } else {
        0.0
    }
}

fn in_rect(r: [f32; 4], x: f32, y: f32, pad: f32) -> bool {
    x >= r[0] - pad && x <= r[0] + r[2] + pad && y >= r[1] - pad && y <= r[1] + r[3] + pad
}

fn shift_at(x: f32) -> f32 {
    ((x - (SHIFT[0] + SHIFT[2] / 2.0)) / (SHIFT[2] / 2.0 - 24.0)).clamp(-1.0, 1.0)
}

fn stretch_at(y: f32) -> f32 {
    (-(y - (STRETCH[1] + STRETCH[3] / 2.0)) / (STRETCH[3] / 2.0 - 24.0)).clamp(-1.0, 1.0)
}

/// An ellipse's outline round (x, y).
fn ellipse(x: f32, y: f32, rx: f32, ry: f32) -> Vec<[f32; 2]> {
    (0..48).map(|k| k as f32 / 48.0 * TAU).map(|a| [x + a.cos() * rx, y + a.sin() * ry]).collect()
}

/// One drifting setting: a smoothed heading that changes now and then and turns back at its bound.
#[derive(Clone, Copy, Debug, Default)]
struct Drifter {
    x: f32,
    v: f32,
    h: f32,
    timer: f32,
    speed: f32,
    bound: f32,
}

impl Drifter {
    fn new((speed, bound): (f32, f32), mult: f32) -> Self {
        Self { speed: speed * mult, bound, ..Self::default() }
    }

    fn tick(&mut self, dt: f32, r: &mut Dice) {
        self.timer -= dt;
        if self.timer <= 0.0 {
            let s = if self.x.abs() > 0.5 * self.bound {
                -sign(self.x)
            } else if r.f() < 0.5 {
                -1.0
            } else {
                1.0
            };
            self.h = s * self.speed * (0.55 + 0.45 * r.f());
            self.timer = 1.4 + 2.2 * r.f();
        }
        if self.x.abs() > self.bound && sign(self.h) == sign(self.x) {
            self.h = -self.h;
        }
        self.v += (self.h - self.v) * (dt / 0.7).min(1.0);
        self.x += self.v * dt;
    }

    /// A surge kicks the drift to a new heading, back the way it came and harder.
    fn kick(&mut self, r: &mut Dice) {
        let s = if self.v == 0.0 { 1.0 } else { sign(self.v) };
        self.h = -s * self.speed * 1.4;
        self.v = self.h * 0.5;
        self.timer = 1.6 + r.f();
    }
}

/// Turn, shift and stretch.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Set {
    t: f32,
    s: f32,
    l: f32,
}

/// One ripple: the ghost's settings, the drift, and the hand's controls.
#[derive(Clone, Debug)]
struct Ripple {
    gt: f32,
    gs: f32,
    gl: f32,
    dt: Drifter,
    ds: Drifter,
    dl: Drifter,
    ct: f32,
    cs: f32,
    cl: f32,
}

impl Ripple {
    fn new(k: usize, stretch: bool, mult: f32, r: &mut Dice, gt: f32) -> Self {
        let small = if k > 0 { SMALL_DRIFT } else { 1.0 };
        let gs = (r.f() * 2.0 - 1.0) * 0.35;
        let gl = if stretch { (r.f() * 2.0 - 1.0) * 0.3 } else { 0.0 };
        let mut rp = Self {
            gt,
            gs,
            gl,
            dt: Drifter::new(DRIFT_T, mult * small),
            ds: Drifter::new(DRIFT_S, mult * small),
            dl: Drifter::new(DRIFT_L, if stretch && k == 0 { mult } else { 0.0 }),
            ct: 0.0,
            cs: 0.0,
            cl: 0.0,
        };
        // The hand starts off the mark, by enough to see, never into a surge.
        let sg = |r: &mut Dice| if r.f() < 0.5 { -1.0 } else { 1.0 };
        let (kt, ks, kl) = if k > 0 { (0.16, 0.45, 0.3) } else { (0.12, 0.36, 0.24) };
        let s = sg(r);
        rp.ct = rp.gt + s * kt * (0.8 + 0.4 * r.f());
        let s = sg(r);
        rp.cs = (rp.gs + s * ks * (0.85 + 0.3 * r.f())).clamp(-1.0, 1.0);
        rp.cl = if stretch {
            let s = sg(r);
            (rp.gl + s * kl * (0.85 + 0.3 * r.f())).clamp(-1.0, 1.0)
        } else {
            0.0
        };
        rp
    }

    /// The live settings: the hand's control plus the drift.
    fn live(&self) -> Set {
        Set { t: self.ct + self.dt.x, s: self.cs + self.ds.x, l: self.cl + self.dl.x }
    }

    fn ghost(&self) -> Set {
        Set { t: self.gt, s: self.gs, l: self.gl }
    }

    /// The control values that would lay it exactly on the ghost now.
    fn need(&self) -> Set {
        Set { t: self.ct + wrap(self.gt - (self.ct + self.dt.x)), s: self.gs - self.ds.x, l: self.gl - self.dl.x }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Phase {
    /// The part step: the coil to the hub.
    #[default]
    Part,
    /// The coil is in.
    Fitted,
    /// The round.
    Play,
    /// Held: the field settles onto the ghost.
    Held,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Grab {
    Tab(usize),
    Stretch,
    Shift,
    Dial,
    Surface,
}

#[derive(Clone, Copy, Debug, Default)]
struct Coil {
    x: f32,
    y: f32,
    held: bool,
    pad: bool,
    set: bool,
    dx: f32,
    dy: f32,
}

/// One quad of the grid: its corners, and which of its edges it draws (the nearest quad on an edge draws it, so a
/// shared edge is drawn once and still on top, as the mockup's painter's order leaves it).
#[derive(Clone, Copy, Debug)]
struct Quad {
    v: [usize; 4],
    own: [bool; 4],
}

/// The game's state.
pub struct Gravity {
    gu: Vec<[f32; 2]>,
    envw: Vec<f32>,
    quads: Vec<Quad>,
    phase: Phase,
    stretch: bool,
    rip: Vec<Ripple>,
    sel: usize,
    err: f32,
    hold: f32,
    hold_need: f32,
    cool: f32,
    surge: f32,
    grab: Option<Grab>,
    last: [f32; 2],
    last_a: f32,
    acc: f32,
    dice: Option<Dice>,
    coil: Coil,
    hl: Vec<f32>,
    hg: Vec<f32>,
    dv: Vec<f32>,
    /// The hand: carrying the coil.
    hand_carry: bool,
    /// The hand: frames until it may pick the other ripple again.
    hand_wait: u32,
    /// The hand: it has rested its first frame of the step (hands off for a frame lets the kit's guide hold go).
    hand_rested: bool,
}

impl Gravity {
    fn new() -> Self {
        let mut gu = Vec::with_capacity(NV);
        let mut envw = Vec::with_capacity(NV);
        for j in 0..=N {
            for i in 0..=N {
                let (u, v) = (-1.0 + 2.0 * i as f32 / N as f32, -1.0 + 2.0 * j as f32 / N as f32);
                gu.push([u, v]);
                envw.push((-(u * u + v * v) / ENV).exp());
            }
        }
        // The quads far to near, sorted once (heights are small beside the depth).
        let mut q: Vec<([usize; 4], f32)> = Vec::with_capacity(N * N);
        for j in 0..N {
            for i in 0..N {
                let a = j * (N + 1) + i;
                let d = proj(-1.0 + (2 * i + 1) as f32 / N as f32, -1.0 + (2 * j + 1) as f32 / N as f32, 0.0)[2];
                q.push(([a, a + 1, a + N + 2, a + N + 1], d));
            }
        }
        q.sort_by(|a, b| b.1.total_cmp(&a.1));
        let edge = |v: [usize; 4], e: usize| {
            let (p, r) = (v[e], v[(e + 1) % 4]);
            (p.min(r), p.max(r))
        };
        let mut last: BTreeMap<(usize, usize), usize> = BTreeMap::new();
        for (qi, (v, _)) in q.iter().enumerate() {
            for e in 0..4 {
                last.insert(edge(*v, e), qi);
            }
        }
        let quads = q
            .iter()
            .enumerate()
            .map(|(qi, (v, _))| Quad { v: *v, own: std::array::from_fn(|e| last.get(&edge(*v, e)) == Some(&qi)) })
            .collect();
        Self {
            gu,
            envw,
            quads,
            phase: Phase::Part,
            stretch: false,
            rip: Vec::new(),
            sel: 0,
            err: 1.0,
            hold: 0.0,
            hold_need: 0.0,
            cool: 0.0,
            surge: 0.0,
            grab: None,
            last: [0.0, 0.0],
            last_a: 0.0,
            acc: 0.0,
            dice: None,
            coil: Coil::default(),
            hl: vec![0.0; NV],
            hg: vec![0.0; NV],
            dv: vec![0.0; NV],
            hand_carry: false,
            hand_wait: 0,
            hand_rested: false,
        }
    }

    fn heights(&self, ghost: bool, out: &mut [f32]) {
        out.fill(0.0);
        for (k, rp) in self.rip.iter().enumerate() {
            let p = if ghost { rp.ghost() } else { rp.live() };
            let (amp, lam0) = RIPPLES[k];
            let kk = TAU / (lam0 * 2f32.powf(OCT * p.l));
            let (st, ct) = p.t.sin_cos();
            let ph = PH * p.s;
            for (o, [u, v]) in out.iter_mut().zip(&self.gu) {
                *o += amp * (kk * (u * ct + v * st) - ph).cos();
            }
        }
        for (o, e) in out.iter_mut().zip(&self.envw) {
            *o *= e;
        }
    }

    /// The match: RMS(live - ghost) / RMS(ghost) over the grid; also each vertex's local miss, over the local scale.
    fn measure(&mut self) -> f32 {
        let (mut hl, mut hg) = (std::mem::take(&mut self.hl), std::mem::take(&mut self.hg));
        self.heights(false, &mut hl);
        self.heights(true, &mut hg);
        let scale: f32 = (0..self.rip.len()).map(|k| RIPPLES[k].0).sum();
        let (mut num, mut den) = (0.0, 0.0);
        for i in 0..NV {
            let d = hl[i] - hg[i];
            num += d * d;
            den += hg[i] * hg[i];
            self.dv[i] = d.abs() / (self.envw[i] * scale);
        }
        self.hl = hl;
        self.hg = hg;
        (num / f32::max(1e-9, den)).sqrt()
    }

    fn hit_test(&self, x: f32, y: f32) -> Option<Grab> {
        if self.rip.len() > 1 {
            if let Some(k) = TABS.iter().position(|t| in_rect(*t, x, y, 10.0)) {
                return Some(Grab::Tab(k));
            }
        }
        if self.stretch && x >= STRETCH[0] - HIT && in_rect(STRETCH, x, y, HIT) {
            return Some(Grab::Stretch);
        }
        if in_rect(SHIFT, x, y, HIT) {
            return Some(Grab::Shift);
        }
        if (x - DIAL.0).hypot(y - DIAL.1) < DIAL.2 + HIT + 6.0 {
            return Some(Grab::Dial);
        }
        if in_rect(GRAPH, x, y, 0.0) {
            return Some(Grab::Surface);
        }
        None
    }

    fn part_step(&mut self, cx: &mut Ctx, dt: f32, input: &Input) {
        let hub = proj(0.0, 0.0, DECK_Z);
        let p = &mut self.coil;
        if input.stick != [0.0, 0.0] {
            p.x += input.stick[0] * 480.0 * dt;
            p.y += input.stick[1] * 480.0 * dt;
            p.pad = true;
        }
        if input.pressed && (input.x - p.x).hypot(input.y - p.y) < 64.0 {
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
            if (p.x - hub[0]).hypot(p.y - hub[1]) < 56.0 {
                p.set = true;
                p.x = hub[0];
                p.y = hub[1];
                self.phase = Phase::Fitted;
                cx.step_done();
                return;
            }
        }
        p.x = p.x.clamp(30.0, W - 30.0);
        p.y = p.y.clamp(BAR_H + 30.0, H - 30.0);
    }

    fn play(&mut self, cx: &mut Ctx, dt: f32, input: &Input) {
        // Picking the ripple: a tab, Tab, or the action (level 3 and on).
        if self.rip.len() > 1 && (input.hit(Key::Tab) || input.action_pressed) {
            self.sel = 1 - self.sel;
        }
        if input.pressed {
            self.grab = self.hit_test(input.x, input.y);
            self.last = [input.x, input.y];
            if self.grab == Some(Grab::Dial) {
                self.last_a = (input.y - DIAL.1).atan2(input.x - DIAL.0);
            }
            if let Some(Grab::Tab(k)) = self.grab {
                self.sel = k;
                self.grab = None;
            }
        }
        if !input.down {
            self.grab = None;
        }
        let stretch = self.stretch;
        let r = &mut self.rip[self.sel];
        match self.grab {
            Some(Grab::Shift) => r.cs = shift_at(input.x),
            Some(Grab::Stretch) => r.cl = stretch_at(input.y),
            Some(Grab::Dial) => {
                if (input.x - DIAL.0).hypot(input.y - DIAL.1) > 14.0 {
                    let a = (input.y - DIAL.1).atan2(input.x - DIAL.0);
                    r.ct += wrap(a - self.last_a) / GEAR;
                    self.last_a = a;
                }
            }
            Some(Grab::Surface) => {
                let (dx, dy) = (input.x - self.last[0], input.y - self.last[1]);
                r.ct += dx * DRAG_TURN;
                r.cs = (r.cs - dy * DRAG_SHIFT).clamp(-1.0, 1.0);
            }
            _ => {}
        }
        self.last = [input.x, input.y];
        // Keys and pad.
        r.ct += input.stick[0] * KEY_TURN * dt;
        if input.stick[1] != 0.0 {
            r.cs = (r.cs - input.stick[1] * KEY_SHIFT * dt).clamp(-1.0, 1.0);
        }
        let q = f32::from(u8::from(input.held(Key::E))) - f32::from(u8::from(input.held(Key::Q)));
        if q != 0.0 && stretch {
            r.cl = (r.cl + q * KEY_STRETCH * dt).clamp(-1.0, 1.0);
        }

        // The drift, on a fixed tick so a run is the same whatever the frame rate.
        self.acc += dt;
        let tick = 1.0 / 60.0;
        if let Some(d) = self.dice.as_mut() {
            while self.acc >= tick - 1e-6 {
                self.acc -= tick;
                for p in &mut self.rip {
                    p.dt.tick(tick, d);
                    p.ds.tick(tick, d);
                    if stretch {
                        p.dl.tick(tick, d);
                    }
                }
            }
        }
        self.err = self.measure();
        if self.err > SURGE_ERR && self.cool <= 0.0 {
            // The surge: the fumble, the field dumps half its error, the drift turns.
            self.cool = COOL_S;
            self.surge = 2.0;
            self.hold = (self.hold - SURGE_HOLD_S).max(0.0);
            self.grab = None;
            if let Some(d) = self.dice.as_mut() {
                for p in &mut self.rip {
                    let l = p.live();
                    p.dt.x -= 0.5 * wrap(l.t - p.gt);
                    p.ds.x -= 0.5 * (l.s - p.gs);
                    p.dl.x -= 0.5 * (l.l - p.gl);
                    p.dt.kick(d);
                    p.ds.kick(d);
                    if stretch {
                        p.dl.kick(d);
                    }
                }
            }
            self.err = self.measure();
            cx.fumble(SURGE_TEXT);
            return;
        }
        if self.err < HOLD_ERR {
            self.hold += dt;
            if self.hold >= self.hold_need {
                self.hold = self.hold_need;
                self.phase = Phase::Held;
                self.grab = None;
                cx.step_done();
            }
        }
    }

    // -------------------------------------------------------------------------------------------- drawing

    fn draw_deck(&self, g: &Pen) {
        // The deck plate under the field: a round plate, its near rim's thickness, its own faint grid.
        let n = 72;
        let pt = |k: usize, z: f32| {
            let a = k as f32 / n as f32 * TAU;
            proj(PLATE_R * a.cos(), PLATE_R * a.sin(), z)
        };
        let top: Vec<[f32; 3]> = (0..n).map(|k| pt(k, DECK_Z)).collect();
        let low: Vec<[f32; 3]> = (0..n).map(|k| pt(k, DECK_Z - 0.1)).collect();
        for k in 0..n {
            let k2 = (k + 1) % n;
            if top[k][2] > CAM && top[k2][2] > CAM {
                continue;
            }
            let q = [xy(top[k]), xy(top[k2]), xy(low[k2]), xy(low[k])];
            g.poly(&q, hex(0x121a26));
            g.path(&q, true, 1.0, hex(0x121a26));
        }
        let top2: Vec<[f32; 2]> = top.iter().map(|p| xy(*p)).collect();
        g.poly(&top2, hex(0x0b1119));
        g.path(&top2, true, 2.0, hex(0x2a3648));
        // The grid, cut to the plate exactly: each line's ends where it meets the rim.
        let col = rgba(60, 78, 100, 0.35);
        for k in -6..=6 {
            let w = k as f32 / 6.0 * PLATE_R;
            let half = (PLATE_R * PLATE_R - w * w).max(0.0).sqrt();
            if half <= 0.0 {
                continue;
            }
            let (a, b) = (proj(w, -half, DECK_Z), proj(w, half, DECK_Z));
            let (cc, d) = (proj(-half, w, DECK_Z), proj(half, w, DECK_Z));
            g.line(a[0], a[1], b[0], b[1], 1.0, col);
            g.line(cc[0], cc[1], d[0], d[1], 1.0, col);
        }
    }

    /// The generator's hub on the deck: on the part step, its open socket.
    fn draw_hub(&self, g: &Pen, t: f32) {
        let hub = proj(0.0, 0.0, DECK_Z);
        let open = self.phase == Phase::Part && !self.coil.set;
        let e = ellipse(hub[0], hub[1], 64.0, 64.0 * 0.55);
        g.poly(&e, hex(0x121a26));
        g.path(&e, true, 3.0, if open { c::AMBER } else { hex(0x3a4a60) });
        if open {
            let mut e = ellipse(hub[0], hub[1], 40.0, 22.0);
            g.poly(&e, hex(0x120d08));
            e.push(e[0]);
            g.dashed(&e, 3.0, c::AMBER, 8.0, 6.0);
            g.ring(hub[0], hub[1] - 2.0, 54.0 + 3.0 * (t * 5.0).sin(), rgba(242, 160, 70, 0.35), 2.0);
        }
    }

    /// The hold ring on the deck round the graph: the track, the hold in green, the surge zone ticked. `near` draws
    /// the half over the surface, else the half under it.
    fn draw_ring(&self, g: &Pen, t: f32, near: bool) {
        let segs = 96;
        let a0 = YAW.cos().atan2(YAW.sin());
        let all: Vec<[f32; 3]> = (0..=segs)
            .map(|i| {
                let a = a0 + TAU * i as f32 / segs as f32;
                proj(RING_R * a.cos(), RING_R * a.sin(), DECK_Z)
            })
            .collect();
        let fill = match self.phase {
            Phase::Held | Phase::Fitted => 1.0,
            _ if self.hold_need > 0.0 => self.hold / self.hold_need,
            _ => 0.0,
        };
        let play = self.phase == Phase::Play;
        let holding = play && self.err < HOLD_ERR;
        let danger = play && self.err > 0.6;
        for i in 0..segs {
            let (a, b) = (all[i], all[i + 1]);
            let far = (a[2] + b[2]) / 2.0 > CAM;
            if near == far {
                continue;
            }
            let f = (i as f32 + 0.5) / segs as f32;
            if f <= fill {
                g.line(a[0], a[1], b[0], b[1], 13.0, if holding || fill >= 1.0 { c::OK } else { hex(0x2f9e64) });
            } else {
                let col = if danger { rgba(255, 71, 87, 0.35 + 0.3 * (t * 9.0).sin()) } else { hex(0x1d2738) };
                g.line(a[0], a[1], b[0], b[1], 11.0, col);
            }
            // The surge zone has a shape as well as a colour: cross ticks on the track.
            if f > fill && danger && i % 3 == 0 {
                let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
                let l = dx.hypot(dy).max(1e-6);
                g.line(
                    a[0] - dy / l * 10.0,
                    a[1] + dx / l * 10.0,
                    a[0] + dy / l * 10.0,
                    a[1] - dx / l * 10.0,
                    3.0,
                    c::DANGER,
                );
            }
        }
        if near && fill > 0.0 && fill < 1.0 {
            // The head of the hold, a bright bead.
            let p = all[((fill * segs as f32).floor() as usize).min(segs)];
            g.disc(p[0], p[1], 9.0, if holding { hex(0xc9ffe0) } else { hex(0x2f9e64) });
        }
    }

    fn draw_surface(&self, g: &Pen, t: f32) {
        let jolt = if self.surge > 1.0 { self.surge - 1.0 } else { 0.0 };
        let zk = 1.0 + 1.4 * jolt * (t * 38.0).sin();
        let pts: Vec<[f32; 2]> = (0..NV)
            .map(|i| {
                let wob = if jolt > 0.0 { 0.04 * (t * 51.0 + i as f32).sin() * jolt } else { 0.0 };
                xy(proj(self.gu[i][0], self.gu[i][1], self.hl[i] * ZS * zk + wob))
            })
            .collect();
        let held = self.phase == Phase::Held;
        let flash = jolt > 0.2;
        let flash_col = css(mix(BAD, [255.0, 214.0, 220.0], 0.5 + 0.5 * (t * 30.0).sin()), 1.0);
        // Quads far to near: a shaded fill tinted by the miss, then its edges coloured by the miss.
        for q in &self.quads {
            let [a, b, cc, d] = q.v;
            let hl = &self.hl;
            let dzu = (hl[b] - hl[a] + hl[cc] - hl[d]) / 2.0;
            let dzv = (hl[d] - hl[a] + hl[cc] - hl[b]) / 2.0;
            let shade = (0.55 + 2.2 * (-dzu * 0.7 + dzv * 0.5) * ZS).clamp(0.2, 1.0);
            let miss = if held { 0.0 } else { (self.dv[a] + self.dv[b] + self.dv[cc] + self.dv[d]) / 4.0 };
            let base = mix([16.0, 30.0, 48.0], miss_style(miss).0, 0.16);
            let k = 0.6 + 0.9 * shade;
            g.poly(&[pts[a], pts[b], pts[cc], pts[d]], css([base[0] * k, base[1] * k, base[2] * k], 0.9));
            for e in 0..4 {
                if !q.own[e] {
                    continue;
                }
                let (p, r) = (q.v[e], q.v[(e + 1) % 4]);
                let (ec, w) = miss_style(if held { 0.0 } else { (self.dv[p] + self.dv[r]) / 2.0 });
                let (col, w) = if flash { (flash_col, w.max(2.6)) } else { (css(ec, 0.95), w) };
                g.line(pts[p][0], pts[p][1], pts[r][0], pts[r][1], w, col);
            }
        }
        // The ghost: the shape the field should hold, a faint dashed wire over the same space.
        if !held {
            let gp = |i: usize| xy(proj(self.gu[i][0], self.gu[i][1], self.hg[i] * ZS));
            let col = rgba(210, 228, 255, 0.5);
            for j in (0..=N).step_by(2) {
                let row: Vec<[f32; 2]> = (0..=N).map(|i| gp(j * (N + 1) + i)).collect();
                g.dashed(&row, 1.3, col, 5.0, 5.0);
            }
            for i in (0..=N).step_by(2) {
                let col_pts: Vec<[f32; 2]> = (0..=N).map(|j| gp(j * (N + 1) + i)).collect();
                g.dashed(&col_pts, 1.3, col, 5.0, 5.0);
            }
        }
    }

    fn draw_bolts(&self, g: &Pen, t: f32, near: bool) {
        let k = if self.surge > 0.0 { (PI * ((2.0 - self.surge) / 2.0).min(1.0)).sin() } else { 0.0 };
        for (i, [u, v]) in BOLTS.iter().enumerate() {
            let z = DECK_Z + 0.03 + 0.55 * k * (0.7 + 0.3 * (i as f32 * 2.1).sin());
            let [x, y, dep] = proj(*u, *v, z);
            if near == (dep > CAM) {
                continue;
            }
            if k > 0.0 {
                let s = proj(*u, *v, DECK_Z);
                g.poly(&ellipse(s[0], s[1], 10.0, 5.0), rgba(0, 0, 0, 0.5));
            }
            let p = g.translate(x, y).rotate(k * (t * 3.0 + i as f32));
            p.rect(-9.0, -4.0, 18.0, 8.0, if i % 2 == 1 { c::STEEL } else { hex(0xa0aab8) });
            p.rect(-4.0, -4.0, 3.0, 8.0, hex(0x4a5566));
        }
    }

    fn draw_dial(&self, g: &Pen) {
        let on = self.phase == Phase::Play;
        let ct = self.rip.get(self.sel).map_or(0.0, |r| r.ct);
        let grab = self.grab == Some(Grab::Dial);
        let (x, y, r) = DIAL;
        g.disc(x, y, r + 22.0, hex(0x0c121a));
        g.ring(x, y, r + 22.0, if grab { c::AMBER } else { c::LINE }, 2.0);
        // The knob's grip turns with the hand (geared).
        g.disc(x, y, r, hex(0x1d2738));
        g.ring(x, y, r, if on { hex(0x9aa6b6) } else { hex(0x4a5566) }, 4.0);
        for j in 0..24 {
            let a = j as f32 / 24.0 * TAU + ct * GEAR;
            let col = match (j, on) {
                (0, true) => c::FG,
                (0, false) => hex(0x6f7f94),
                _ => hex(0x2a3446),
            };
            g.disc(x + a.cos() * (r - 12.0), y + a.sin() * (r - 12.0), 5.0, col);
        }
        // The face: the crests' direction, three lines across, cut to the face.
        g.disc(x, y, r - 30.0, hex(0x0a1018));
        let face = g.translate(x, y).rotate(-(ct + YAW) + FRAC_PI_2);
        let rr = r - 32.0;
        for o in [-26.0f32, 0.0, 26.0] {
            let pts: Vec<[f32; 2]> = (0..=80)
                .map(|k| -80.0 + 2.0 * k as f32)
                .map(|xx| [xx, o + 5.0 * (xx * 0.08).sin()])
                .filter(|p| p[0].hypot(p[1]) <= rr)
                .collect();
            face.path(&pts, false, 4.0, if on { c::ACCENT } else { hex(0x3d5a72) });
        }
        // Which ways it turns.
        g.turn_arrow(x, y, r + 11.0, -2.45, -1.85, rgba(232, 238, 246, 0.45));
        g.turn_arrow(x, y, r + 11.0, -0.7, -1.3, rgba(232, 238, 246, 0.45));
        g.text("TURN", x, y + r + 44.0, 20.0, if on { c::DIM } else { hex(0x3a4658) }, Align::Center);
    }

    fn draw_tabs(&self, g: &Pen) {
        if self.rip.len() < 2 {
            return;
        }
        for (k, t) in TABS.iter().enumerate() {
            let on = self.sel == k;
            g.round(
                t[0],
                t[1],
                t[2],
                t[3],
                14.0,
                Some(if on { hex(0x1d2a3c) } else { hex(0x101721) }),
                Some((if on { 4.0 } else { 2.0 }, if on { c::AMBER } else { c::LINE })),
            );
            // The ripple's glyph: a big slow wave, or a small quick one.
            let (a, f) = if k > 0 { (7.0, 0.42) } else { (16.0, 0.16) };
            let (cx, cy) = (t[0] + t[2] / 2.0, t[1] + 32.0);
            let pts: Vec<[f32; 2]> =
                (0..=38).map(|i| -38.0 + 2.0 * i as f32).map(|x| [cx + x, cy + a * (x * f).sin()]).collect();
            g.path(&pts, false, if k > 0 { 3.0 } else { 4.0 }, if on { c::FG } else { c::DIM });
            let label = if k > 0 { "SMALL" } else { "BIG" };
            g.text(label, cx, t[1] + t[3] - 17.0, 18.0, if on { c::AMBER } else { c::DIM }, Align::Center);
        }
    }
}

/// A slider's look: on (live), locked (hatched with a padlock), held.
struct SliderLook {
    on: bool,
    locked: bool,
    grab: bool,
}

fn slider(g: &Pen, s: [f32; 4], vertical: bool, val: f32, o: &SliderLook) {
    let [x, y, w, h] = s;
    g.round(x, y, w, h, 14.0, Some(hex(0x121a25)), Some((2.0, if o.grab { c::AMBER } else { c::LINE })));
    if o.locked {
        g.hatch(x + 4.0, y + 4.0, w - 8.0, h - 8.0, rgba(111, 127, 148, 0.18));
        // A padlock in the middle.
        let (lx, ly) = (x + w / 2.0, y + h / 2.0);
        g.arc(lx, ly - 10.0, 10.0, PI, TAU, 4.0, hex(0x6f7f94));
        g.round(lx - 15.0, ly - 10.0, 30.0, 24.0, 5.0, Some(hex(0x6f7f94)), None);
        g.disc(lx, ly + 1.0, 3.5, hex(0x121a25));
        return;
    }
    let len = if vertical { h } else { w };
    let mid = len / 2.0;
    let pos = mid + if vertical { -val } else { val } * (mid - 24.0);
    // Ticks along the track, the centre mark brighter.
    for k in -4..=4 {
        let at = mid + k as f32 / 4.0 * (mid - 24.0);
        let th = if k == 0 { 3.0 } else { 2.0 };
        if vertical {
            g.rect(x + 10.0, y + at - 1.0, w - 20.0, th, hex(0x2a3446));
        } else {
            g.rect(x + at - 1.0, y + 10.0, th, h - 20.0, hex(0x2a3446));
        }
    }
    let fill = if o.on { rgba(79, 195, 247, 0.35) } else { rgba(79, 195, 247, 0.12) };
    if vertical {
        g.rect(x + 14.0, y + mid.min(pos), w - 28.0, (pos - mid).abs(), fill);
    } else {
        g.rect(x + mid.min(pos), y + 14.0, (pos - mid).abs(), h - 28.0, fill);
    }
    // The handle.
    let hc = if o.grab {
        c::AMBER
    } else if o.on {
        hex(0xc9d3e0)
    } else {
        hex(0x4a5566)
    };
    if vertical {
        g.round(x - 8.0, y + pos - 16.0, w + 16.0, 32.0, 9.0, Some(hc), None);
        g.rect(x + 4.0, y + pos - 2.0, w - 8.0, 4.0, hex(0x1b2433));
    } else {
        g.round(x + pos - 16.0, y - 8.0, 32.0, h + 16.0, 9.0, Some(hc), None);
        g.rect(x + pos - 2.0, y + 4.0, 4.0, h - 8.0, hex(0x1b2433));
    }
}

/// Crests sliding sideways: a wave by the Shift slider.
fn shift_glyph(g: &Pen, x: f32, y: f32, on: bool) {
    let pts: Vec<[f32; 2]> = (0..=36).map(|k| [x - 18.0 + k as f32, y - 6.0 + 6.0 * (k as f32 * 0.35).sin()]).collect();
    g.path(&pts, false, 3.0, if on { c::ACCENT } else { hex(0x3d5a72) });
}

fn draw_coil(g: &Pen, x: f32, y: f32, r: f32) {
    g.disc(x + 4.0, y + 6.0, r, rgba(0, 0, 0, 0.45));
    g.disc(x, y, r, c::COPPER);
    g.ring(x, y, r, hex(0xf0c08a), 3.0);
    for k in 0..3 {
        g.ring(x, y, r - 8.0 - k as f32 * 6.0, hex(0x7a4a22), 3.0);
    }
    g.disc(x, y, r - 28.0, hex(0x141b27));
}

const OK_RGB: [f32; 3] = [61.0, 220.0, 132.0];
const WARN_RGB: [f32; 3] = [255.0, 197.0, 66.0];
const BAD: [f32; 3] = [255.0, 71.0, 87.0];

fn mix(a: [f32; 3], b: [f32; 3], k: f32) -> [f32; 3] {
    std::array::from_fn(|i| a[i] + (b[i] - a[i]) * k)
}

fn css(c: [f32; 3], a: f32) -> Color32 {
    let ch = |v: f32| v.clamp(0.0, 255.0) as u8;
    rgba(ch(c[0]), ch(c[1]), ch(c[2]), a)
}

/// A local miss to a line colour and width: green on, amber off, red and thick far off.
fn miss_style(d: f32) -> ([f32; 3], f32) {
    if d < 0.18 {
        (OK_RGB, 1.6)
    } else if d < 0.5 {
        (mix(OK_RGB, WARN_RGB, (d - 0.18) / 0.32), 2.2)
    } else if d < 0.8 {
        (mix(WARN_RGB, BAD, (d - 0.5) / 0.3), 3.0)
    } else {
        (BAD, 4.2)
    }
}

/// The hand's dead bands: how near each control it calls on the mark (a step of the keys is about half of each).
const HAND_DB: Set = Set { t: 0.008, s: 0.015, l: 0.012 };

impl Game for Gravity {
    fn id(&self) -> &'static str {
        "gravity"
    }

    fn knobs(&self) -> &'static [&'static str] {
        &["drift_scale_frac", "ripples_count", "stretch_count", "hold_s"]
    }

    fn step(&mut self, cx: &mut Ctx) {
        let mut r = cx.dice();
        self.phase = if cx.part { Phase::Part } else { Phase::Play };
        self.stretch = cx.knob("stretch_count") >= 1.0;
        let mult = cx.knob("drift_scale_frac") * if cx.combat { COMBAT_SPEEDUP } else { 1.0 };
        self.rip.clear();
        self.sel = 0;
        self.err = 1.0;
        self.hold = 0.0;
        self.hold_need = 0.0;
        self.cool = 0.0;
        self.surge = 0.0;
        self.grab = None;
        self.acc = 0.0;
        self.hand_carry = false;
        self.hand_wait = 0;
        self.hand_rested = false;
        self.coil =
            Coil { x: CRATE[0] + CRATE[2] / 2.0, y: CRATE[1] + CRATE[3] / 2.0 + 8.0, set: !cx.part, ..Coil::default() };
        if !cx.part {
            let gt = r.f() * TAU;
            self.rip.push(Ripple::new(0, self.stretch, mult, &mut r, gt));
            // The small ripple runs across the big one.
            if cx.knob("ripples_count") >= 2.0 {
                let g2 = gt + FRAC_PI_2 + (r.f() * 2.0 - 1.0) * 0.5;
                self.rip.push(Ripple::new(1, self.stretch, mult, &mut r, g2));
            }
            self.hold_need = cx.knob("hold_s");
            self.dice = Some(r);
            self.err = self.measure();
        } else {
            self.dice = Some(r);
        }
    }

    fn update(&mut self, cx: &mut Ctx, dt: f32, input: &Input) {
        self.cool = (self.cool - dt).max(0.0);
        self.surge = (self.surge - dt).max(0.0);
        match self.phase {
            Phase::Part => self.part_step(cx, dt, input),
            Phase::Play => self.play(cx, dt, input),
            Phase::Held => {
                // Played: the field settles onto the ghost.
                let k = (dt * 3.0).min(1.0);
                for p in &mut self.rip {
                    let l = p.live();
                    p.dt.x -= k * wrap(l.t - p.gt);
                    p.ds.x -= k * (l.s - p.gs);
                    p.dl.x -= k * (l.l - p.gl);
                }
                self.err = self.measure();
            }
            Phase::Fitted => {}
        }
    }

    fn draw(&self, g: &Pen, _cx: &Ctx, t: f32, _input: &Input) {
        g.rect(0.0, BAR_H, W, H, hex(0x070b12));
        let [gx, gy, gw, gh] = GRAPH;
        let calm = self.phase == Phase::Play && self.err < HOLD_ERR;
        g.panel(gx, gy, gw, gh, 18.0, hex(0x080d15), if calm { hex(0x245a3e) } else { c::LINE });
        let (jx, jy) = if self.surge > 1.0 {
            ((t * 47.0).sin() * 10.0 * (self.surge - 1.0), (t * 41.0).cos() * 7.0 * (self.surge - 1.0))
        } else {
            (0.0, 0.0)
        };
        let gp = g.clip(gx + 2.0, gy + 2.0, gw - 4.0, gh - 4.0).translate(jx, jy);
        self.draw_deck(&gp);
        let part = matches!(self.phase, Phase::Part | Phase::Fitted);
        if !part {
            self.draw_hub(&gp, t);
            self.draw_ring(&gp, t, false);
            self.draw_bolts(&gp, t, false);
            self.draw_surface(&gp, t);
            self.draw_ring(&gp, t, true);
            self.draw_bolts(&gp, t, true);
        } else {
            // The field is off: the bare deck, the hub open for its coil.
            self.draw_hub(&gp, t);
            self.draw_bolts(&gp, t, false);
            self.draw_bolts(&gp, t, true);
            if self.coil.set {
                let h = proj(0.0, 0.0, DECK_Z);
                draw_coil(&gp, h[0], h[1], 34.0);
                gp.ring(h[0], h[1], 46.0, c::OK, 4.0);
            }
        }
        if self.surge > 0.0 {
            gp.rect(gx - 20.0, gy - 20.0, gw + 40.0, gh + 40.0, rgba(255, 71, 87, 0.12 * self.surge.min(1.0)));
        }

        // The controls, the tabs and, on the part step, the crate.
        if part {
            let [x, y, w, h] = CRATE;
            g.panel(x, y, w, h, 14.0, hex(0x141b27), c::LINE);
            g.hatch(x + 12.0, y + 12.0, w - 24.0, 12.0, rgba(242, 160, 70, 0.5));
            if !self.coil.set {
                let p = &self.coil;
                draw_coil(g, p.x, p.y, 40.0);
                let col = if p.held || p.pad { c::AMBER } else { rgba(232, 238, 246, 0.35) };
                g.ring(p.x, p.y, 50.0 + 3.0 * (t * 5.0).sin(), col, 3.0);
            }
            return;
        }
        let on = self.phase == Phase::Play;
        let Some(r) = self.rip.get(self.sel) else { return };
        self.draw_dial(g);
        slider(g, SHIFT, false, r.cs, &SliderLook { on, locked: false, grab: self.grab == Some(Grab::Shift) });
        shift_glyph(g, SHIFT[0] - 58.0, SHIFT[1] + 24.0, on);
        g.text("SHIFT", SHIFT[0] - 58.0, SHIFT[1] + 46.0, 16.0, if on { c::DIM } else { hex(0x3a4658) }, Align::Center);
        let grab = self.grab == Some(Grab::Stretch);
        slider(g, STRETCH, true, r.cl, &SliderLook { on, locked: !self.stretch, grab });
        let sc = if self.stretch && on { c::DIM } else { hex(0x3a4658) };
        g.text("STRETCH", STRETCH[0] + STRETCH[2] / 2.0, STRETCH[1] + STRETCH[3] + 28.0, 17.0, sc, Align::Center);
        self.draw_tabs(g);
    }

    fn hand(&mut self, _cx: &Ctx, _t: f32) -> Input {
        if !self.hand_rested {
            self.hand_rested = true;
            return Input::default();
        }
        match self.phase {
            Phase::Part => {
                // Pick the coil up, carry it to the hub at a hand's pace, let go there.
                let hub = proj(0.0, 0.0, DECK_Z);
                let p = self.coil;
                if !p.held && !self.hand_carry {
                    self.hand_carry = true;
                    return Input::hold(p.x, p.y, true);
                }
                let d = (hub[0] - p.x).hypot(hub[1] - p.y);
                if d > 2.0 {
                    let k = (12.0 / d).min(1.0);
                    return Input::hold(p.x + (hub[0] - p.x) * k, p.y + (hub[1] - p.y) * k, false);
                }
                self.hand_carry = false;
                Input::release(hub[0], hub[1])
            }
            Phase::Play => {
                // A pad player nursing the field: the stick and Q and E steer the held ripple onto its ghost, and Tab
                // picks the other ripple when this one is on the mark and the other has wandered.
                self.hand_wait = self.hand_wait.saturating_sub(1);
                let off = |r: &Ripple| {
                    let n = r.need();
                    ((n.t - r.ct).abs() / HAND_DB.t)
                        .max((n.s - r.cs).abs() / HAND_DB.s)
                        .max((n.l - r.cl).abs() / HAND_DB.l)
                };
                if self.rip.len() > 1 && self.hand_wait == 0 {
                    let other = 1 - self.sel;
                    if off(&self.rip[self.sel]) <= 1.0 && off(&self.rip[other]) > 1.5 {
                        self.hand_wait = 10;
                        return Input::default().key(Key::Tab, true);
                    }
                }
                let r = &self.rip[self.sel];
                let n = r.need();
                let mut i = Input::default();
                let mut press = |k: Key| i = std::mem::take(&mut i).key(k, false);
                if n.t - r.ct > HAND_DB.t {
                    press(Key::ArrowRight);
                } else if n.t - r.ct < -HAND_DB.t {
                    press(Key::ArrowLeft);
                }
                if n.s - r.cs > HAND_DB.s {
                    press(Key::ArrowUp);
                } else if n.s - r.cs < -HAND_DB.s {
                    press(Key::ArrowDown);
                }
                if self.stretch {
                    if n.l - r.cl > HAND_DB.l {
                        press(Key::E);
                    } else if n.l - r.cl < -HAND_DB.l {
                        press(Key::Q);
                    }
                }
                i
            }
            _ => Input::default(),
        }
    }

    fn guide_now(&self) -> Option<usize> {
        match self.phase {
            Phase::Part | Phase::Fitted => None,
            _ => Some(if self.hold > 0.0 { 3 } else { 0 }),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::games::tests::{fumble_check, plays_to_end};

    #[test]
    fn a_steady_hand_repairs_it_damaged_disabled_and_destroyed() {
        plays_to_end("gravity");
    }

    #[test]
    fn turning_the_field_far_off_the_ghost_surges_it() {
        // The stick held right turns the crests away until the field surges.
        fumble_check("gravity", |_r| crate::kit::Input::default().key(egui::Key::ArrowRight, false));
    }
}
