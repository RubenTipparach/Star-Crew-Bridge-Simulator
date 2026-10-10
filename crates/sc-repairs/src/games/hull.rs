//! Hull plating, a damaged wall section repaired from inside (repair-minigames design 2 and 6e, hull-repair 3), from
//! `docs/mockups/repairs/hull.js`.
//!
//! One wall bay between two ribs, in the crew finish (dark grey steel, rivets), its middle panel buckled, scorched and
//! torn. A round is a whole new plate, in three phases:
//! - **Cut out**: trace the torch round the buckled panel's outline, either way round; what the torch passes over is
//!   cut. Drifting off the line only pauses. Too fast scores a thin line instead of cutting through: go over it again.
//! - **Fit**: drag the new plate from the trolley onto the opening (it snaps in), then turn it, dragging round its rim,
//!   until its bolt holes meet the frame's.
//! - **Weld, then bolt**: run the bead along each of the plate's four seams. The torch's heat gauge shows the speed:
//!   too slow and the heat climbs into the red and burns through (the fumble "Burned through: a hiss of air", and a
//!   hole in the seam to go over again); too fast leaves a cold bead to go over again. Then drive the frame's bolts in
//!   star order, the next one lit.
//!
//! Each level narrows the good weld band and brings the burn-through sooner (design 1a). A disabled or destroyed
//! section's first step fetches a plate from the stack onto the trolley.
//!
//! Keys: hold Right or Left (or D, A) to run the torch along the cut; the arrows carry the plate and Space drops it;
//! Q and E (or Left and Right) turn it; Space runs the weld along the current seam (Tab picks the next); arrows pick a
//! bolt and Space drives it.

use std::f32::consts::{FRAC_PI_2, FRAC_PI_4, PI, TAU};

use egui::{Color32, Key};

use crate::kit::{nearest, star_order, Ctx, Game, Input, BAR_H, H, TOUCH_R, W};
use crate::pen::{c, hex, mix, rgba, Align, Pen};

/// In the engine.
pub const PORTED: bool = true;

/// A fresh game.
pub fn new() -> Box<dyn Game> {
    Box::new(Hull::default())
}

// ------------------------------------------------------------------------ the wall (canvas px)
/// The opening's centre.
const OC: [f32; 2] = [450.0, 396.0];
/// The buckled panel's half size.
const PANEL_H: f32 = 210.0;
/// The cut line: a rounded square inside the panel's seams.
const CUT_H: f32 = 196.0;
const CUT_R: f32 = 22.0;
/// The new plate's half size (it laps the frame).
const PLATE_H: f32 = 238.0;
/// The frame's bolt holes, from the centre.
const HOLE_H: f32 = 218.0;
/// The bay's ribs (left edge of each, 44 wide).
const RIBS: [f32; 2] = [70.0, 786.0];
const TROLLEY: [f32; 4] = [940.0, 470.0, 290.0, 214.0];
const STACK: [f32; 4] = [940.0, 128.0, 290.0, 250.0];
/// The plate's scale on the trolley.
const SMALL: f32 = 0.36;

// ------------------------------------------------------------------------ feel
/// Px off the line a torch still works.
const BAND: f32 = 40.0;
/// Px of line a coverage bin holds.
const BIN: f32 = 8.0;
/// Px either side of the tip a pass covers.
const TIP: f32 = 7.0;
/// Px/s: faster than this scores a thin line.
const CUT_FAST: f32 = 560.0;
/// Share of the line cut through to free the panel.
const CUT_DONE: f32 = 0.98;
/// Px/s at which the torch's heat would fall to nothing.
const WELD_REF: f32 = 420.0;
/// S: the heat follows the speed with this lag.
const HEAT_TAU: f32 = 0.25;
/// S the torch is off after a burn-through.
const TORCH_COOL_S: f32 = 0.8;
/// Share of a seam that must be good.
const WELD_DONE: f32 = 0.97;
/// Px/s the keys run the torch along the cut, and along a seam.
const KEY_SPEED: f32 = 300.0;
const KEY_WELD: f32 = 150.0;
/// Rad: the plate's holes meet inside this.
const SNAP: f32 = 0.06;
/// S: the speed's smoothing.
const SPEED_EMA: f32 = 0.12;
const BURNED: &str = "Burned through: a hiss of air";

// ------------------------------------------------------------------------ the steady hand
/// Px/s the hand runs the cut (under `CUT_FAST`).
const HAND_CUT_PX_S: f32 = 420.0;
/// Px/s the hand runs a weld: heat 0.6, the middle of the good band at every level.
const HAND_WELD_PX_S: f32 = 168.0;
/// Px a frame the hand carries a plate.
const HAND_CARRY_PX: f32 = 10.0;
/// Frames the hand rests at the start of each step, looking before it reaches (and letting go of anything held).
const HAND_REST_FRAMES: u32 = 18;
/// Rad a frame the hand turns the plate.
const HAND_TURN_RAD: f32 = 0.03;

const PAINT: Color32 = hex(0x4d4a46);
const PAINT_HI: Color32 = hex(0x625e58);
const PAINT_LO: Color32 = hex(0x36332f);
const RIVET: Color32 = hex(0x8b867d);

fn wrap_q(a: f32) -> f32 {
    a - FRAC_PI_2 * (a / FRAC_PI_2).round()
}

/// The frame's bolt holes round the rim, clockwise from the top left (so `star_order(8)` works across).
const HOLES: [[f32; 2]; 8] =
    [[-1.0, -1.0], [0.0, -1.0], [1.0, -1.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0], [-1.0, 1.0], [-1.0, 0.0]];

fn hole_xy(i: usize, a: f32) -> [f32; 2] {
    let (u, v) = (HOLES[i][0] * HOLE_H, HOLES[i][1] * HOLE_H);
    [OC[0] + u * a.cos() - v * a.sin(), OC[1] + u * a.sin() + v * a.cos()]
}

/// A line for the torch: points, arc lengths, bins (0 untouched, 1 thin or cold, 2 done, 3 burned).
#[derive(Clone, Debug, Default)]
struct Line {
    pts: Vec<[f32; 2]>,
    cum: Vec<f32>,
    len: f32,
    closed: bool,
    n: usize,
    bins: Vec<u8>,
    last_s: Option<f32>,
    speed: f32,
}

impl Line {
    fn new(pts: &[[f32; 2]], closed: bool) -> Self {
        let mut p = pts.to_vec();
        if closed {
            p.push(pts[0]);
        }
        let mut cum = vec![0.0];
        for i in 1..p.len() {
            cum.push(cum[i - 1] + (p[i][0] - p[i - 1][0]).hypot(p[i][1] - p[i - 1][1]));
        }
        let len = cum[cum.len() - 1];
        let n = (len / BIN).ceil() as usize;
        Self { pts: p, cum, len, closed, n, bins: vec![0; n], last_s: None, speed: 0.0 }
    }

    fn at(&self, sv: f32) -> [f32; 2] {
        let sv = if self.closed { sv.rem_euclid(self.len) } else { sv.clamp(0.0, self.len) };
        let mut i = 1;
        while i < self.cum.len() - 1 && self.cum[i] < sv {
            i += 1;
        }
        let (a, b) = (self.pts[i - 1], self.pts[i]);
        let k = (sv - self.cum[i - 1]) / (self.cum[i] - self.cum[i - 1]).max(1e-6);
        [a[0] + (b[0] - a[0]) * k, a[1] + (b[1] - a[1]) * k]
    }

    /// The nearest point of the line to (x, y): its distance and arc length.
    fn project(&self, x: f32, y: f32) -> (f32, f32) {
        let mut best = (1e9f32, 0.0f32);
        for i in 1..self.pts.len() {
            let ([ax, ay], [bx, by]) = (self.pts[i - 1], self.pts[i]);
            let (dx, dy) = (bx - ax, by - ay);
            let l2 = (dx * dx + dy * dy).max(1e-9);
            let u = (((x - ax) * dx + (y - ay) * dy) / l2).clamp(0.0, 1.0);
            let d = (x - (ax + dx * u)).hypot(y - (ay + dy * u));
            if d < best.0 {
                best = (d, self.cum[i - 1] + u * l2.sqrt());
            }
        }
        best
    }

    /// The torch on the line this frame at arc length `sv` (or off it: None): the bins it passed over since the last
    /// frame, and its speed along the line updated, px/s. The one rule both the cut and the weld use.
    fn sweep(&mut self, sv: Option<f32>, dt: f32) -> Option<Vec<usize>> {
        let Some(sv) = sv else {
            self.last_s = None;
            self.speed *= (1.0 - dt / SPEED_EMA).max(0.0);
            return None;
        };
        let (mut a, mut b, mut ds) = (sv, sv, 0.0);
        if let Some(l) = self.last_s {
            ds = sv - l;
            if self.closed {
                if ds > self.len / 2.0 {
                    ds -= self.len;
                }
                if ds < -self.len / 2.0 {
                    ds += self.len;
                }
            }
            if ds.abs() < 160.0 {
                a = l.min(l + ds);
                b = l.max(l + ds);
            } else {
                ds = 0.0;
            }
        }
        self.last_s = Some(sv);
        self.speed += (ds.abs() / dt.max(1e-3) - self.speed) * (dt / SPEED_EMA).min(1.0);
        let mut out = Vec::new();
        let (i0, i1) = (((a - TIP) / BIN).floor() as i64, ((b + TIP) / BIN).floor() as i64);
        let n = self.n as i64;
        for i in i0..=i1 {
            let k = if self.closed { i.rem_euclid(n) } else { i };
            if (0..n).contains(&k) && !out.contains(&(k as usize)) {
                out.push(k as usize);
            }
        }
        Some(out)
    }

    fn count(&self, v: u8) -> usize {
        self.bins.iter().filter(|b| **b == v).count()
    }

    fn welded(&self) -> bool {
        self.count(2) as f32 >= self.n as f32 * WELD_DONE
    }
}

/// The cut line: a rounded square round the opening's centre.
fn cut_line() -> Line {
    let (h, r) = (CUT_H, CUT_R);
    let mut pts = Vec::new();
    for (cx, cy, a0) in
        [(h - r, -h + r, -FRAC_PI_2), (h - r, h - r, 0.0), (-h + r, h - r, FRAC_PI_2), (-h + r, -h + r, PI)]
    {
        for k in 0..=6 {
            let a = a0 + (k as f32 / 6.0) * FRAC_PI_2;
            pts.push([OC[0] + cx + a.cos() * r, OC[1] + cy + a.sin() * r]);
        }
    }
    Line::new(&pts, true)
}

fn seam_lines() -> Vec<Line> {
    let h = PLATE_H;
    let (x0, x1, y0, y1) = (OC[0] - h, OC[0] + h, OC[1] - h, OC[1] + h);
    vec![
        Line::new(&[[x0, y0], [x1, y0]], false),
        Line::new(&[[x1, y0], [x1, y1]], false),
        Line::new(&[[x1, y1], [x0, y1]], false),
        Line::new(&[[x0, y1], [x0, y0]], false),
    ]
}

#[derive(Clone, Debug, Default)]
struct Plate {
    x: f32,
    y: f32,
    a: f32,
    scale: f32,
    placed: bool,
    fitted: bool,
    on_trolley: bool,
}

#[derive(Clone, Copy, Debug, Default)]
struct Bolt {
    on: bool,
    t: f32,
}

/// The wall bay: kept over a round's phases, new each round.
#[derive(Clone, Debug, Default)]
struct Wall {
    round: u32,
    cut: bool,
    fall: f32,
    plate: Plate,
    welded: bool,
    seams: Vec<Line>,
    bolts: [Bolt; 8],
    scorch: [f32; 2],
    creases: Vec<[f32; 4]>,
    stacked: i32,
}

impl Wall {
    fn fresh(r: &mut crate::kit::Dice, round: u32) -> Self {
        let sign = if r.f() < 0.5 { -1.0 } else { 1.0 };
        let a = sign * (0.4 + 0.35 * r.f());
        let scorch = [0.3 + 0.4 * r.f(), 0.25 + 0.3 * r.f()];
        let creases = (0..7).map(|_| [r.f(), r.f(), r.f(), r.f()]).collect();
        Self {
            round,
            cut: false,
            fall: 1.0,
            plate: Plate {
                x: TROLLEY[0] + TROLLEY[2] / 2.0,
                y: TROLLEY[1] + 92.0,
                a,
                scale: SMALL,
                placed: false,
                fitted: false,
                on_trolley: true,
            },
            welded: false,
            seams: Vec::new(),
            bolts: [Bolt::default(); 8],
            scorch,
            creases,
            stacked: 6,
        }
    }

    fn next_bolt(&self) -> Option<usize> {
        let done = self.bolts.iter().filter(|b| b.on).count();
        star_order(8).get(done).copied()
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Kind {
    #[default]
    Part,
    Cut,
    Fit,
    WeldBolt,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Phase {
    #[default]
    Part,
    Cut,
    Fit,
    Weld,
    Bolt,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Grab {
    Carry,
    Turn,
}

#[derive(Clone, Debug)]
struct Spark {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    life: f32,
    col: Color32,
}

#[derive(Clone, Debug, Default)]
struct Carry {
    x: f32,
    y: f32,
    held: bool,
    dx: f32,
    dy: f32,
}

/// One phase's state.
#[derive(Clone, Debug, Default)]
struct St {
    kind: Kind,
    phase: Phase,
    heat: f32,
    burn_t: f32,
    cool: f32,
    hiss: Vec<[f32; 3]>,
    sparks: Vec<Spark>,
    cur: usize,
    keys: bool,
    key_s: f32,
    grab: Option<Grab>,
    prev_a: f32,
    dx: f32,
    dy: f32,
    wrong: Option<(usize, f32)>,
    played: bool,
    carry: Option<Carry>,
    sel: usize,
    line: Option<Line>,
    tip: Option<[f32; 2]>,
    /// The round's level: the cold edge, the burn edge (heat, 0-1) and the burn's time, s.
    cold: f32,
    burn: f32,
    burn_s: f32,
    /// The sparks' seeded stream.
    seed: u64,
}

impl St {
    fn rnd(&mut self) -> f32 {
        // xorshift64*: the sparks' scatter, the same every run (the mockup's Math.random).
        self.seed ^= self.seed >> 12;
        self.seed ^= self.seed << 25;
        self.seed ^= self.seed >> 27;
        ((self.seed.wrapping_mul(0x2545_f491_4f6c_dd1d) >> 40) as f32) / (1u64 << 24) as f32
    }

    fn spark(&mut self, x: f32, y: f32, n: usize, col: Color32, speed: f32) {
        for _ in 0..n {
            if self.sparks.len() >= 300 {
                break;
            }
            let a = self.rnd() * TAU;
            let v = speed * (0.3 + 0.7 * self.rnd());
            let life = 0.4 + 0.3 * self.rnd();
            self.sparks.push(Spark { x, y, vx: a.cos() * v, vy: a.sin() * v - 60.0, life, col });
        }
    }
}

/// The steady hand's memory.
#[derive(Clone, Debug, Default)]
struct HandMem {
    down: bool,
    s: f32,
    dir: f32,
    seam: usize,
    wait: u32,
    a: f32,
    n: u32,
    rest: u32,
}

/// The game's state.
#[derive(Default)]
pub struct Hull {
    wall: Option<Wall>,
    st: St,
    hm: HandMem,
}

fn torch_pos(input: &Input, l: &Line) -> Option<f32> {
    if input.down {
        let (d, s) = l.project(input.x, input.y);
        return (d < BAND).then_some(s);
    }
    None
}

fn cut_update(st: &mut St, wall: &mut Wall, cx: &mut Ctx, dt: f32, input: &Input) {
    if wall.cut {
        return;
    }
    let Some(mut l) = st.line.take() else { return };
    let dir = input.stick[0].signum() * f32::from(u8::from(input.stick[0] != 0.0));
    let sv = if dir != 0.0 {
        st.keys = true;
        st.key_s += dir * KEY_SPEED * dt;
        Some(st.key_s)
    } else {
        torch_pos(input, &l)
    };
    if let Some(hit) = l.sweep(sv, dt) {
        let fast = l.speed > CUT_FAST;
        for k in hit {
            if fast {
                if l.bins[k] == 0 {
                    l.bins[k] = 1;
                }
            } else {
                l.bins[k] = 2;
            }
        }
        let [x, y] = l.at(sv.unwrap_or(0.0));
        st.tip = Some([x, y]);
        if fast {
            st.spark(x, y, 1, hex(0xffd27a), 160.0);
        } else {
            st.spark(x, y, 3, hex(0xffb347), 320.0);
        }
        if l.count(2) as f32 >= l.n as f32 * CUT_DONE {
            l.bins.fill(2);
            wall.cut = true;
            wall.fall = 0.0;
            st.tip = None;
            cx.step_done();
        }
    }
    st.line = Some(l);
}

fn part_update(st: &mut St, wall: &mut Wall, cx: &mut Ctx, dt: f32, input: &Input) {
    let Some(p) = st.carry.as_mut() else { return };
    if input.stick != [0.0, 0.0] {
        p.x += input.stick[0] * 480.0 * dt;
        p.y += input.stick[1] * 480.0 * dt;
        st.keys = true;
    }
    if input.pressed && (input.x - p.x).abs() < 120.0 && (input.y - p.y).abs() < 100.0 {
        p.held = true;
        p.dx = p.x - input.x;
        p.dy = p.y - input.y;
        st.keys = false;
    }
    if p.held && input.down {
        p.x = input.x + p.dx;
        p.y = input.y + p.dy;
    }
    let [tx, ty, tw, th] = TROLLEY;
    let on_trolley = p.x > tx - 20.0 && p.x < tx + tw + 20.0 && p.y > ty - 30.0 && p.y < ty + th;
    if (p.held && !input.down) || (st.keys && input.action_pressed) {
        p.held = false;
        if on_trolley {
            st.carry = None;
            wall.stacked -= 1;
            wall.plate.on_trolley = true;
            cx.step_done();
            return;
        }
    }
    if !p.held && !st.keys {
        let (hx, hy) = (STACK[0] + STACK[2] / 2.0, STACK[1] + STACK[3] / 2.0 + 10.0);
        p.x += (hx - p.x) * (dt * 10.0).min(1.0);
        p.y += (hy - p.y) * (dt * 10.0).min(1.0);
    }
    p.x = p.x.clamp(40.0, W - 40.0);
    p.y = p.y.clamp(BAR_H + 40.0, H - 40.0);
}

fn fit_update(st: &mut St, wall: &mut Wall, cx: &mut Ctx, dt: f32, input: &Input) {
    let p = &mut wall.plate;
    if p.fitted {
        return;
    }
    let home = [TROLLEY[0] + TROLLEY[2] / 2.0, TROLLEY[1] + 92.0];
    // Keys: carry with the arrows and drop with Space; once in, turn with Q and E (or Left and Right).
    if !p.placed {
        if input.stick != [0.0, 0.0] {
            p.x += input.stick[0] * 520.0 * dt;
            p.y += input.stick[1] * 520.0 * dt;
            st.keys = true;
            p.on_trolley = false;
        }
        if st.keys && input.action_pressed && (p.x - OC[0]).hypot(p.y - OC[1]) < 120.0 {
            p.placed = true;
        }
    } else {
        let k = f32::from(u8::from(input.held(Key::E) || input.held(Key::ArrowRight)))
            - f32::from(u8::from(input.held(Key::Q) || input.held(Key::ArrowLeft)));
        if k != 0.0 {
            p.a += k * 0.8 * dt;
            st.keys = true;
        }
    }
    // Pointer: a press near the plate's middle carries it; a press on its rim, once it is in, turns it.
    if input.pressed {
        let d = (input.x - p.x).hypot(input.y - p.y);
        let r = PLATE_H * p.scale;
        if p.placed && d > 70.0 && d < r + 40.0 {
            st.grab = Some(Grab::Turn);
            st.prev_a = (input.y - p.y).atan2(input.x - p.x);
            st.keys = false;
        } else if d < r + 20.0 {
            st.grab = Some(Grab::Carry);
            st.dx = p.x - input.x;
            st.dy = p.y - input.y;
            p.placed = false;
            p.on_trolley = false;
            st.keys = false;
        }
    }
    if st.grab == Some(Grab::Carry) && input.down {
        p.x = input.x + st.dx;
        p.y = input.y + st.dy;
    }
    if st.grab == Some(Grab::Turn) && input.down {
        let a = (input.y - p.y).atan2(input.x - p.x);
        let d = a - st.prev_a;
        p.a += d.sin().atan2(d.cos());
        st.prev_a = a;
    }
    if !input.down {
        if st.grab == Some(Grab::Carry) {
            if (p.x - OC[0]).hypot(p.y - OC[1]) < 90.0 {
                p.placed = true;
            } else {
                p.on_trolley = true;
            }
        }
        st.grab = None;
    }
    // Where it goes: snapped into the opening, back on the trolley, or in the hand; full size over the wall.
    if p.placed {
        p.x += (OC[0] - p.x) * (dt * 12.0).min(1.0);
        p.y += (OC[1] - p.y) * (dt * 12.0).min(1.0);
    } else if p.on_trolley && st.grab.is_none() && !st.keys {
        p.x += (home[0] - p.x) * (dt * 10.0).min(1.0);
        p.y += (home[1] - p.y) * (dt * 10.0).min(1.0);
    }
    p.scale += ((if p.x < 880.0 { 1.0 } else { SMALL }) - p.scale) * (dt * 8.0).min(1.0);
    if p.placed && (p.x - OC[0]).hypot(p.y - OC[1]) < 3.0 && wrap_q(p.a).abs() < SNAP {
        p.a -= wrap_q(p.a);
        p.x = OC[0];
        p.y = OC[1];
        p.scale = 1.0;
        p.fitted = true;
        st.grab = None;
        cx.step_done();
    }
}

fn weld_update(st: &mut St, wall: &mut Wall, cx: &mut Ctx, dt: f32, input: &Input) {
    st.cool = (st.cool - dt).max(0.0);
    for h in &mut st.hiss {
        h[2] += dt;
    }
    st.hiss.retain(|h| h[2] < 2.5);
    if wall.welded || wall.seams.len() != 4 {
        return;
    }
    let seams = &mut wall.seams;
    // Which seam: the one the torch is on (sticky while it stays near), or the keys' current one.
    if input.hit(Key::Tab) {
        st.cur = (st.cur + 1) % 4;
        st.key_s = 0.0;
    }
    if seams[st.cur].welded() {
        if let Some(i) = seams.iter().position(|q| !q.welded()) {
            if st.keys {
                st.cur = i;
                st.key_s = 0.0;
            }
        }
    }
    let mut on: Option<(usize, f32)> = None;
    if input.held(Key::Space) {
        // Keys: Space runs the torch along the current seam at a good speed.
        st.keys = true;
        st.key_s = (st.key_s + KEY_WELD * dt).min(seams[st.cur].len);
        on = Some((st.cur, st.key_s));
    } else if input.down {
        // The seam under the torch; the one already in hand wins a near tie (a corner), so a pass does not jump seams.
        let mut best: Option<(usize, f32, f32)> = None;
        for (i, q) in seams.iter().enumerate() {
            let (d, s) = q.project(input.x, input.y);
            let k = d - if i == st.cur { 12.0 } else { 0.0 };
            if d < BAND && best.is_none_or(|b| k < b.1) {
                best = Some((i, k, s));
            }
        }
        if let Some((i, _, s)) = best {
            st.cur = i;
            on = Some((i, s));
            st.keys = false;
        }
    }
    for (i, q) in seams.iter_mut().enumerate() {
        if on.map(|o| o.0) != Some(i) {
            q.sweep(None, dt);
        }
    }
    if st.cool > 0.0 {
        if let Some((i, _)) = on {
            seams[i].sweep(None, dt);
        }
        st.heat = (st.heat - dt / 0.6).max(0.0);
        st.tip = None;
        return;
    }
    let hit = on.and_then(|(i, sv)| seams[i].sweep(Some(sv), dt));
    let (Some(hit), Some((i, sv))) = (hit, on) else {
        st.heat = (st.heat - dt / 0.6).max(0.0);
        st.burn_t = 0.0;
        st.tip = None;
        return;
    };
    let l = &mut seams[i];
    // The heat follows the speed: slow is hot, fast is cold.
    let target = (1.0 - l.speed / WELD_REF).clamp(0.0, 1.0);
    st.heat += (target - st.heat) * (dt / HEAT_TAU).min(1.0);
    let [x, y] = l.at(sv);
    st.tip = Some([x, y]);
    if st.heat > st.burn {
        st.burn_t += dt;
        if st.burn_t > st.burn_s {
            // Burned through: a hole in the seam, air hissing out of it, the torch off a moment.
            let k0 = (sv / BIN).floor() as i64;
            for k in k0 - 2..=k0 + 2 {
                if (0..l.n as i64).contains(&k) {
                    l.bins[k as usize] = 3;
                }
            }
            st.hiss.push([x, y, 0.0]);
            st.heat = 0.45;
            st.burn_t = 0.0;
            st.cool = TORCH_COOL_S;
            l.last_s = None;
            cx.fumble(BURNED);
            return;
        }
    } else {
        st.burn_t = 0.0;
    }
    let v = if st.heat < st.cold { 1 } else { 2 };
    for k in hit {
        if !(v == 1 && l.bins[k] == 2) {
            l.bins[k] = v;
        }
    }
    let col = if st.heat > st.burn { hex(0xffffff) } else { hex(0xffd27a) };
    st.spark(x, y, 2, col, 200.0);
    if seams.iter().all(Line::welded) {
        for q in seams.iter_mut() {
            q.bins.fill(2);
        }
        wall.welded = true;
        st.tip = None;
        if st.kind == Kind::WeldBolt {
            st.phase = Phase::Bolt;
            wall.bolts = [Bolt::default(); 8];
        } else {
            cx.step_done();
        }
    }
}

fn bolt_update(st: &mut St, wall: &mut Wall, cx: &mut Ctx, dt: f32, input: &Input) {
    for b in &mut wall.bolts {
        if b.on {
            b.t = (b.t + dt / 0.35).min(1.0);
        }
    }
    if let Some((_, t)) = &mut st.wrong {
        *t -= dt;
        if *t <= 0.0 {
            st.wrong = None;
        }
    }
    if st.played {
        return;
    }
    let mut drive = |st: &mut St, wall: &mut Wall, i: usize| {
        if wall.bolts[i].on {
            return;
        }
        if Some(i) != wall.next_bolt() {
            st.wrong = Some((i, 0.7));
            cx.say("Out of order");
            return;
        }
        wall.bolts[i].on = true;
        if wall.bolts.iter().all(|b| b.on) {
            st.played = true;
            cx.step_done();
        }
    };
    for (k, d) in [
        (Key::ArrowLeft, 7),
        (Key::A, 7),
        (Key::ArrowUp, 7),
        (Key::ArrowRight, 1),
        (Key::D, 1),
        (Key::ArrowDown, 1),
        (Key::Tab, 1),
    ] {
        if input.hit(k) {
            st.keys = true;
            st.sel = (st.sel + d) % 8;
        }
    }
    if input.action_pressed {
        st.keys = true;
        let s = st.sel;
        drive(st, wall, s);
    }
    if input.pressed {
        let pts: Vec<Option<[f32; 2]>> = (0..8).map(|i| Some(hole_xy(i, 0.0))).collect();
        if let Some(i) = nearest(&pts, input.x, input.y, TOUCH_R + 10.0) {
            st.keys = false;
            drive(st, wall, i);
        }
    }
}

// ------------------------------------------------------------------------ drawing helpers (private)

/// A colour along gradient stops at `t` (0-1).
fn stop_at(stops: &[(f32, Color32)], t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    for w in stops.windows(2) {
        let ((t0, c0), (t1, c1)) = (w[0], w[1]);
        if t <= t1 {
            return mix(c0, c1, (t - t0) / (t1 - t0).max(1e-6));
        }
    }
    stops.last().map_or(Color32::TRANSPARENT, |s| s.1)
}

/// A rectangle filled with a linear gradient of several stops from `p0` to `p1` (canvas's `createLinearGradient`),
/// drawn as a grid of quads coloured at their corners.
#[allow(clippy::too_many_arguments)]
fn lin_grad(g: &Pen, x: f32, y: f32, w: f32, h: f32, p0: [f32; 2], p1: [f32; 2], stops: &[(f32, Color32)]) {
    let (dx, dy) = (p1[0] - p0[0], p1[1] - p0[1]);
    let l2 = (dx * dx + dy * dy).max(1e-6);
    let col = |px: f32, py: f32| stop_at(stops, ((px - p0[0]) * dx + (py - p0[1]) * dy) / l2);
    let (nx, ny) = (if dx.abs() > 1e-3 { 6 } else { 1 }, if dy.abs() > 1e-3 { 6 } else { 1 });
    for i in 0..nx {
        for j in 0..ny {
            let (xa, xb) = (x + w * i as f32 / nx as f32, x + w * (i + 1) as f32 / nx as f32);
            let (ya, yb) = (y + h * j as f32 / ny as f32, y + h * (j + 1) as f32 / ny as f32);
            g.quad([[xa, ya], [xb, ya], [xb, yb], [xa, yb]], [col(xa, ya), col(xb, ya), col(xb, yb), col(xa, yb)]);
        }
    }
}

/// A two-circle radial gradient (canvas's `createRadialGradient`): stop t lies on the circle between (x0, y0, r0)
/// and (x1, y1, r1); inside the first circle is the first stop, beyond the last nothing (every use here ends clear).
fn radial2(g: &Pen, a: [f32; 3], b: [f32; 3], stops: &[(f32, Color32)]) {
    const SEG: usize = 40;
    let circ = |t: f32| [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t];
    if let Some(&(t0, c0)) = stops.first() {
        let k = circ(t0);
        if k[2] > 0.0 {
            // A mesh fan, not a feathered disc, so no rim shows where it meets the rings.
            g.radial(k[0], k[1], k[2], c0, c0);
        }
    }
    for w in stops.windows(2) {
        let ((ta, ca), (tb, cb)) = (w[0], w[1]);
        let (p, q) = (circ(ta), circ(tb));
        for s in 0..SEG {
            let (u, v) = (TAU * s as f32 / SEG as f32, TAU * (s + 1) as f32 / SEG as f32);
            let on = |k: [f32; 3], a: f32| [k[0] + a.cos() * k[2], k[1] + a.sin() * k[2]];
            g.quad([on(p, u), on(p, v), on(q, v), on(q, u)], [ca, ca, cb, cb]);
        }
    }
}

fn rivets(g: &Pen, x0: f32, y0: f32, x1: f32, y1: f32, step: f32) {
    let n = ((x1 - x0).hypot(y1 - y0) / step).round().max(1.0) as i32;
    for i in 0..=n {
        let k = i as f32 / n as f32;
        let (x, y) = (x0 + (x1 - x0) * k, y0 + (y1 - y0) * k);
        g.disc(x + 1.0, y + 1.5, 3.4, rgba(0, 0, 0, 0.5));
        g.disc(x, y, 3.2, RIVET);
        g.disc(x - 1.0, y - 1.0, 1.2, hex(0xcfc8bb));
    }
}

fn steel_panel(g: &Pen, x: f32, y: f32, w: f32, h: f32, base: Color32) {
    lin_grad(g, x, y, w, h, [x, y], [x + w * 0.4, y + h], &[(0.0, PAINT_HI), (0.5, base), (1.0, PAINT_LO)]);
    let mut yy = y + 4.0;
    while yy < y + h {
        g.line(x + 2.0, yy, x + w - 2.0, yy, 1.0, rgba(255, 255, 255, 0.05));
        yy += 6.0;
    }
    g.rect_stroke(x + 1.5, y + 1.5, w - 3.0, h - 3.0, 3.0, hex(0x1c1a18));
    g.rect_stroke(x + 4.0, y + 4.0, w - 8.0, h - 8.0, 1.0, rgba(255, 255, 255, 0.08));
}

/// A dashed circle.
#[allow(clippy::too_many_arguments)]
fn dashed_ring(g: &Pen, x: f32, y: f32, r: f32, width: f32, col: Color32, dash: f32, gap: f32) {
    g.dashed(&Pen::arc_points(x, y, r, 0.0, TAU), width, col, dash, gap);
}

impl Hull {
    fn draw_wall(&self, g: &Pen) {
        let g = g.clip(16.0, 84.0, 868.0, 628.0);
        g.rect(16.0, 84.0, 868.0, 628.0, hex(0x2a2826));
        // Neighbouring panels round the bay: above, below and beside the buckled one.
        let (x0, x1, y0, y1) = (OC[0] - PANEL_H, OC[0] + PANEL_H, OC[1] - PANEL_H, OC[1] + PANEL_H);
        steel_panel(&g, RIBS[0] + 44.0, 124.0, x0 - RIBS[0] - 44.0, 548.0, PAINT);
        steel_panel(&g, x1, 124.0, RIBS[1] - x1, 548.0, PAINT);
        steel_panel(&g, x0, 124.0, x1 - x0, y0 - 124.0, PAINT);
        steel_panel(&g, x0, y1, x1 - x0, 672.0 - y1, PAINT);
        // A vent grille on the left neighbour and a stencil on the right: the crew finish's panels are not all alike.
        for k in 0..6 {
            g.round(RIBS[0] + 64.0, 300.0 + k as f32 * 18.0, 70.0, 8.0, 4.0, Some(hex(0x1c1a18)), None);
        }
        g.hatch(x1 + 20.0, 520.0, RIBS[1] - x1 - 40.0, 14.0, rgba(209, 160, 36, 0.55));
        // The ribs: a pillar each side with its base and capital, and the trims top and bottom.
        for rx in RIBS {
            lin_grad(
                &g,
                rx,
                84.0,
                44.0,
                628.0,
                [rx, 0.0],
                [rx + 44.0, 0.0],
                &[(0.0, hex(0x2f2c29)), (0.5, hex(0x6a655d)), (1.0, hex(0x2a2724))],
            );
            g.rect(rx - 6.0, 112.0, 56.0, 16.0, hex(0x57534c));
            g.rect(rx - 6.0, 664.0, 56.0, 18.0, hex(0x57534c));
            rivets(&g, rx + 22.0, 150.0, rx + 22.0, 640.0, 40.0);
        }
        g.rect(16.0, 96.0, 868.0, 24.0, hex(0x3a3733));
        g.rect(16.0, 676.0, 868.0, 30.0, hex(0x3a3733));
        g.rect(RIBS[0] + 60.0, 100.0, RIBS[1] - RIBS[0] - 76.0, 6.0, rgba(255, 240, 210, 0.5));
        rivets(&g, x0 + 10.0, 136.0, x1 - 10.0, 136.0, 30.0);
        rivets(&g, x0 + 10.0, 660.0, x1 - 10.0, 660.0, 30.0);
    }

    /// Behind the panel: frame, insulation and a cable tray, seen once the panel is out.
    fn draw_cavity(&self, g: &Pen) {
        let h = CUT_H;
        let (x0, y0) = (OC[0] - h, OC[1] - h);
        let p = g.clip(x0, y0, 2.0 * h, 2.0 * h);
        p.round(x0, y0, 2.0 * h, 2.0 * h, CUT_R, Some(hex(0x121110)), None);
        // Quilted insulation.
        let mut y = y0;
        while y < OC[1] + h {
            let mut x = x0;
            while x < OC[0] + h {
                p.round(x + 3.0, y + 3.0, 30.0, 30.0, 8.0, Some(hex(0x4a4436)), None);
                p.disc(x + 18.0, y + 18.0, 3.0, hex(0x2b271f));
                x += 36.0;
            }
            y += 36.0;
        }
        // Two frame members and a cable tray across.
        p.rect(OC[0] - 110.0, y0, 26.0, 2.0 * h, hex(0x24282e));
        p.rect(OC[0] + 84.0, y0, 26.0, 2.0 * h, hex(0x24282e));
        p.rect(x0, OC[1] + 40.0, 2.0 * h, 34.0, hex(0x30353d));
        for (col, dy) in [(hex(0xb0473f), 48.0), (hex(0xd1a024), 56.0), (hex(0x4f8fd0), 64.0)] {
            p.rect(x0, OC[1] + dy, 2.0 * h, 4.0, col);
        }
        g.round(x0, y0, 2.0 * h, 2.0 * h, CUT_R, None, Some((4.0, hex(0x0b0a09))));
    }

    /// The buckled panel: dented, creased, scorched, a corner torn back to the frame.
    fn draw_buckled(&self, g: &Pen, wall: &Wall, t: f32) {
        let h = PANEL_H;
        let g = if wall.cut {
            let f = wall.fall;
            g.translate(OC[0] + f * 60.0, OC[1] + f * f * 700.0)
                .rotate(f * 0.5)
                .alpha((1.0 - f * 1.2).max(0.0))
                .translate(-OC[0], -OC[1])
                .clip(OC[0] - CUT_H, OC[1] - CUT_H, 2.0 * CUT_H, 2.0 * CUT_H)
        } else {
            g.clone()
        };
        steel_panel(&g, OC[0] - h, OC[1] - h, 2.0 * h, 2.0 * h, hex(0x47443f));
        let pc = g.clip(OC[0] - h, OC[1] - h, 2.0 * h, 2.0 * h);
        // The buckle: a dent off centre, lit from the upper left.
        let (bx, by) = (OC[0] - 40.0, OC[1] + 20.0);
        radial2(
            &pc,
            [bx + 30.0, by + 34.0, 10.0],
            [bx, by, 190.0],
            &[
                (0.0, rgba(0, 0, 0, 0.55)),
                (0.45, rgba(0, 0, 0, 0.25)),
                (0.7, rgba(255, 255, 255, 0.07)),
                (1.0, rgba(0, 0, 0, 0.0)),
            ],
        );
        // Creases from the dent outwards: dark folds with a lit edge.
        for [a, b, cc, d] in &wall.creases {
            let ang = a * TAU;
            let (r0, r1, bend) = (30.0 + 40.0 * b, 130.0 + 80.0 * cc, (d - 0.5) * 0.6);
            let p0 = [bx + ang.cos() * r0, by + ang.sin() * r0];
            let p1 = [bx + (ang + bend).cos() * r1, by + (ang + bend).sin() * r1];
            let pm = [
                bx + (ang + bend / 2.0).cos() * (r0 + r1) / 2.0 + 10.0,
                by + (ang + bend / 2.0).sin() * (r0 + r1) / 2.0,
            ];
            g.path_round(&[p0, pm, p1], 4.0, rgba(15, 13, 12, 0.75));
            g.path_round(
                &[[p0[0] - 2.0, p0[1] - 3.0], [pm[0] - 2.0, pm[1] - 3.0], [p1[0] - 2.0, p1[1] - 3.0]],
                2.0,
                rgba(200, 190, 175, 0.25),
            );
        }
        // Scorch: soot round where the blast hit, the paint blistered brown at its edge.
        let (sx, sy) = (OC[0] - h + wall.scorch[0] * 2.0 * h, OC[1] - h + wall.scorch[1] * 2.0 * h);
        radial2(
            &pc,
            [sx, sy, 6.0],
            [sx, sy, 150.0],
            &[
                (0.0, rgba(8, 6, 5, 0.92)),
                (0.35, rgba(30, 20, 12, 0.7)),
                (0.6, rgba(90, 52, 24, 0.35)),
                (1.0, rgba(0, 0, 0, 0.0)),
            ],
        );
        // Soot carried up from it in soft streaks.
        for k in 0..5 {
            let x = sx - 48.0 + k as f32 * 24.0;
            let top = sy - 170.0 + ((k * 37) % 50) as f32;
            pc.gradient(x - 7.0, top, 14.0, sy - 30.0 - top, rgba(10, 8, 6, 0.0), rgba(10, 8, 6, 0.4), true);
        }
        // A corner torn back: jagged, the frame dark behind it, the lip bent up.
        let (cx, cy) = (OC[0] + h, OC[1] - h);
        let torn = [
            [cx, cy + 4.0],
            [cx - 92.0, cy + 4.0],
            [cx - 70.0, cy + 22.0],
            [cx - 82.0, cy + 40.0],
            [cx - 48.0, cy + 52.0],
            [cx - 40.0, cy + 78.0],
            [cx - 14.0, cy + 70.0],
            [cx - 4.0, cy + 104.0],
            [cx, cy + 104.0],
        ];
        g.poly(&torn, hex(0x0d0c0b));
        g.path(&torn, true, 2.0, hex(0xa59d90));
        g.poly(&[[cx - 92.0, cy + 4.0], [cx - 70.0, cy + 22.0], [cx - 104.0, cy + 30.0]], hex(0x6a655d));
        // The panel's rivets, some sprung.
        let (l, r, tp, bt) = (OC[0] - h + 14.0, OC[0] + h - 14.0, OC[1] - h + 14.0, OC[1] + h - 14.0);
        rivets(&g, l, tp, OC[0] + h - 110.0, tp, 30.0);
        rivets(&g, l, bt, r, bt, 30.0);
        rivets(&g, l, tp, l, bt, 30.0);
        rivets(&g, r, OC[1] - h + 110.0, r, bt, 30.0);
        // A cracked lamp cover hanging loose (design 2's looks for 25-49%).
        let lamp = g.translate(OC[0] + 100.0, OC[1] - 150.0).rotate(0.35 + 0.03 * (t * 2.0).sin());
        lamp.round(-50.0, 0.0, 100.0, 22.0, 6.0, Some(hex(0xc8c0a8)), Some((2.0, hex(0x3a3733))));
        lamp.path(&[[-20.0, 0.0], [-6.0, 12.0], [10.0, 4.0], [22.0, 22.0]], false, 2.0, hex(0x3a3733));
    }

    /// The new plate: clean steel in the crew finish, its eight bolt holes, a stencil band.
    fn draw_plate(&self, g: &Pen, p: &Plate, bolts: &[Bolt; 8]) {
        let h = PLATE_H;
        let g = g.translate(p.x, p.y).rotate(p.a).scale(p.scale, p.scale);
        g.rect(-h + 10.0, -h + 14.0, 2.0 * h, 2.0 * h, rgba(0, 0, 0, 0.45));
        steel_panel(&g, -h, -h, 2.0 * h, 2.0 * h, hex(0x57534d));
        g.hatch(-h + 40.0, h - 60.0, 140.0, 16.0, rgba(209, 160, 36, 0.6));
        g.rect(h - 150.0, -h + 34.0, 110.0, 10.0, rgba(230, 224, 210, 0.35));
        g.rect(h - 150.0, -h + 52.0, 70.0, 10.0, rgba(230, 224, 210, 0.35));
        for (i, ho) in HOLES.iter().enumerate() {
            let (u, v) = (ho[0] * HOLE_H, ho[1] * HOLE_H);
            g.disc(u, v, 11.0, hex(0x141210));
            g.ring(u, v, 11.0, hex(0x8b867d), 2.0);
            let b = bolts[i];
            if b.on {
                // A driven bolt: a hex head that spins in as it seats.
                let hexa: Vec<[f32; 2]> = (0..6)
                    .map(|k| {
                        let a = PI / 6.0 + k as f32 * PI / 3.0 + b.t * 6.0;
                        [u + a.cos() * 14.0, v + a.sin() * 14.0]
                    })
                    .collect();
                g.poly(&hexa, hex(0xb7b0a4));
                g.path(&hexa, true, 2.0, hex(0x4a4640));
            }
        }
    }

    fn draw_seams(&self, g: &Pen, wall: &Wall) {
        for l in &wall.seams {
            for k in 0..l.n {
                let a = l.at(k as f32 * BIN);
                let b = l.at(((k + 1) as f32 * BIN + 0.5).min(l.len));
                match l.bins[k] {
                    0 => g.dashed(&[a, b], 2.0, rgba(242, 160, 70, 0.45), 5.0, 5.0),
                    1 => g.dashed(&[a, b], 6.0, hex(0x7d8796), 3.0, 5.0),
                    2 => {
                        g.line(a[0], a[1], b[0], b[1], 9.0, hex(0xd9cfbf));
                        // A good bead's ripples, so done reads by shape as well as tone.
                        g.ring((a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0, 3.5, rgba(120, 100, 70, 0.8), 1.5);
                    }
                    _ => g.line(a[0], a[1], b[0], b[1], 11.0, hex(0x050404)),
                }
            }
        }
    }

    fn draw_torch(&self, g: &Pen, tip: Option<[f32; 2]>, hot: bool, t: f32) {
        let Some([x, y]) = tip else { return };
        let g = g.translate(x, y);
        g.path_round(&[[14.0, 14.0], [80.0, 70.0]], 14.0, hex(0x3a4658));
        g.line(8.0, 8.0, 20.0, 20.0, 8.0, hex(0xc08a3e));
        let mid = if hot { rgba(255, 120, 90, 0.9) } else { rgba(150, 210, 255, 0.85) };
        let r = 30.0 + 3.0 * (t * 40.0).sin();
        radial2(
            &g,
            [0.0, 0.0, 0.0],
            [0.0, 0.0, r],
            &[(0.0, rgba(255, 255, 255, 1.0)), (0.3, mid), (1.0, rgba(79, 195, 247, 0.0))],
        );
    }

    /// The torch's gauge at the right: speed for the cut (too fast hatched), heat for the weld (cold and burn hatched).
    fn draw_gauge(&self, g: &Pen, heat: bool, v: f32) {
        let (gx, gy, gw, gh) = (1050.0, 150.0, 64.0, 420.0);
        g.panel(gx - 70.0, gy - 60.0, gw + 140.0, gh + 120.0, 18.0, hex(0x0c121a), c::LINE);
        g.round(gx, gy, gw, gh, 14.0, Some(hex(0x121a25)), None);
        let y_at = |u: f32| gy + gh - u * gh;
        let (cold, burn) = (self.st.cold, self.st.burn);
        if heat {
            g.hatch(gx, y_at(cold), gw, cold * gh, rgba(79, 168, 247, 0.55));
            g.rect(gx, y_at(burn), gw, (burn - cold) * gh, rgba(61, 220, 132, 0.25));
            g.hatch(gx, gy, gw, (1.0 - burn) * gh, rgba(255, 71, 87, 0.7));
        } else {
            let f = 1.0 / 1.5;
            g.rect(gx, y_at(f), gw, f * gh, rgba(61, 220, 132, 0.25));
            g.hatch(gx, gy, gw, (1.0 - f) * gh, rgba(242, 160, 70, 0.6));
        }
        let y = y_at(v.clamp(0.0, 1.0));
        g.poly(&[[gx - 18.0, y - 12.0], [gx - 2.0, y], [gx - 18.0, y + 12.0]], c::FG);
        g.rect(gx, y - 2.0, gw, 4.0, c::FG);
        // The torch icon over it and one word under it.
        let ic = g.translate(gx + gw / 2.0, gy - 30.0).rotate(-0.7);
        ic.round(-22.0, -6.0, 36.0, 12.0, 5.0, Some(hex(0xc9d3e0)), None);
        ic.poly(&[[14.0, -5.0], [28.0, 0.0], [14.0, 5.0]], hex(0xffb347));
        g.text(if heat { "HEAT" } else { "SPEED" }, gx + gw / 2.0, gy + gh + 30.0, 20.0, c::DIM, Align::Center);
    }

    fn draw_trolley(&self, g: &Pen) {
        let [x, y, w, h] = TROLLEY;
        g.panel(x, y, w, h, 16.0, hex(0x151b25), c::LINE);
        g.rect(x + 20.0, y + h - 40.0, w - 40.0, 12.0, hex(0x3a4658));
        for wx in [x + 46.0, x + w - 46.0] {
            g.disc(wx, y + h - 18.0, 14.0, hex(0x0b0f15));
            g.ring(wx, y + h - 18.0, 14.0, hex(0x5a6577), 3.0);
        }
    }

    fn draw_stack(&self, g: &Pen, wall: &Wall) {
        let [x, y, w, h] = STACK;
        g.panel(x, y, w, h, 16.0, hex(0x151b25), c::LINE);
        g.hatch(x + 14.0, y + 14.0, w - 28.0, 12.0, rgba(242, 160, 70, 0.5));
        let n = wall.stacked - i32::from(self.st.carry.is_some());
        for k in 0..n.max(0) {
            let yy = y + h - 40.0 - k as f32 * 9.0;
            g.rect(x + 60.0, yy, w - 120.0, 8.0, if k % 2 == 1 { hex(0x57534d) } else { hex(0x4d4a46) });
            g.rect(x + 60.0, yy + 7.0, w - 120.0, 1.0, hex(0x1c1a18));
        }
    }
}

impl Game for Hull {
    fn id(&self) -> &'static str {
        "hull"
    }

    fn knobs(&self) -> &'static [&'static str] {
        &["cold_frac", "burn_frac", "burn_s"]
    }

    fn phases(&self, _rounds: u32) -> &'static [&'static str] {
        &["cut", "fit", "weldbolt"]
    }

    fn step(&mut self, cx: &mut Ctx) {
        let mut r = cx.dice();
        let kind = if cx.part {
            Kind::Part
        } else {
            match cx.phase_name {
                "fit" => Kind::Fit,
                "weldbolt" => Kind::WeldBolt,
                _ => Kind::Cut,
            }
        };
        // Each round is a whole plate (design 1a): a new round's cut starts a new wall, and so does a round restarted
        // after three fumbles (its wall is already cut).
        let stale = self.wall.as_ref().is_none_or(|w| kind == Kind::Cut && (w.round != cx.round || w.cut));
        if stale {
            self.wall = Some(Wall::fresh(&mut r, cx.round));
        }
        let seed = (u64::from((r.f() * 16_777_216.0) as u32) << 20) | 0x9e37_79b9;
        self.st = St {
            kind,
            phase: match kind {
                Kind::Part => Phase::Part,
                Kind::Cut => Phase::Cut,
                Kind::Fit => Phase::Fit,
                Kind::WeldBolt => Phase::Weld,
            },
            cold: cx.knob("cold_frac"),
            burn: cx.knob("burn_frac"),
            burn_s: cx.knob("burn_s"),
            seed,
            ..St::default()
        };
        self.hm = HandMem::default();
        let Some(wall) = self.wall.as_mut() else { return };
        match kind {
            Kind::Part => {
                wall.plate.on_trolley = false;
                self.st.carry = Some(Carry {
                    x: STACK[0] + STACK[2] / 2.0,
                    y: STACK[1] + STACK[3] / 2.0 + 10.0,
                    ..Carry::default()
                });
            }
            Kind::Cut => self.st.line = Some(cut_line()),
            Kind::Fit => {
                if !wall.cut {
                    wall.cut = true;
                    wall.fall = 1.0;
                }
            }
            Kind::WeldBolt => {
                wall.cut = true;
                let p = &mut wall.plate;
                p.placed = true;
                p.fitted = true;
                p.a = 0.0;
                p.x = OC[0];
                p.y = OC[1];
                p.scale = 1.0;
                wall.seams = seam_lines();
            }
        }
    }

    fn update(&mut self, cx: &mut Ctx, dt: f32, input: &Input) {
        let Some(wall) = self.wall.as_mut() else { return };
        let st = &mut self.st;
        if wall.cut && wall.fall < 1.0 {
            wall.fall = (wall.fall + dt * 1.4).min(1.0);
        }
        for p in &mut st.sparks {
            p.vy += 700.0 * dt;
            p.x += p.vx * dt;
            p.y += p.vy * dt;
            p.life -= dt;
        }
        st.sparks.retain(|p| p.life > 0.0);
        match st.phase {
            Phase::Part => part_update(st, wall, cx, dt, input),
            Phase::Cut => cut_update(st, wall, cx, dt, input),
            Phase::Fit => fit_update(st, wall, cx, dt, input),
            Phase::Weld => weld_update(st, wall, cx, dt, input),
            Phase::Bolt => bolt_update(st, wall, cx, dt, input),
        }
        if st.phase == Phase::Cut && !input.down && !st.keys {
            st.tip = None;
        }
    }

    fn draw(&self, g: &Pen, _cx: &Ctx, t: f32, _input: &Input) {
        g.rect(0.0, BAR_H, W, H, hex(0x070b12));
        let Some(wall) = self.wall.as_ref() else { return };
        let st = &self.st;
        self.draw_wall(g);
        let p = &wall.plate;
        if wall.cut {
            self.draw_cavity(g);
        }
        if !wall.cut || wall.fall < 1.0 {
            self.draw_buckled(g, wall, t);
        }
        // The frame's bolt holes round the opening, ringed while the plate is to be fitted.
        if wall.cut && !p.fitted {
            for i in 0..8 {
                let [x, y] = hole_xy(i, 0.0);
                g.disc(x, y, 11.0, hex(0x0b0a09));
                g.ring(x, y, 15.0, rgba(242, 160, 70, 0.8), 3.0);
            }
        }
        if st.phase == Phase::Cut && !wall.cut {
            if let Some(l) = &st.line {
                // The cut line: dashed where it is still whole, a thin scored line where it went too fast, open where
                // cut.
                for k in 0..l.n {
                    let a = l.at(k as f32 * BIN);
                    let b = l.at((k + 1) as f32 * BIN + 0.5);
                    match l.bins[k] {
                        0 => g.dashed(&[a, b], 3.0, rgba(242, 160, 70, 0.85), 8.0, 6.0),
                        1 => g.line(a[0], a[1], b[0], b[1], 2.0, hex(0xffd27a)),
                        _ => {
                            g.line(a[0], a[1], b[0], b[1], 8.0, hex(0x050404));
                            g.line(a[0], a[1], b[0], b[1], 2.0, rgba(255, 140, 60, 0.5));
                        }
                    }
                }
            }
        }
        // The plate: on the trolley, in the hand, or in the opening.
        if matches!(st.phase, Phase::Fit | Phase::Weld | Phase::Bolt) {
            if st.phase == Phase::Fit {
                self.draw_trolley(g);
            }
            self.draw_plate(g, p, &wall.bolts);
            // The holes meeting: green rings where the plate's holes sit over the frame's.
            if st.phase == Phase::Fit && p.placed {
                let near = wrap_q(p.a).abs() < 0.12;
                for i in 0..8 {
                    let [x, y] = hole_xy(i, p.a);
                    g.ring(x, y, 16.0, if near { c::OK } else { rgba(232, 238, 246, 0.5) }, 3.0);
                }
                // The turn: an arrow round the rim the way that closes the gap.
                if !p.fitted {
                    let k = if wrap_q(p.a) > 0.0 { -1.0 } else { 1.0 };
                    let a0 = -FRAC_PI_4;
                    g.turn_arrow(OC[0], OC[1], PLATE_H + 26.0, a0, a0 + k * 0.5, rgba(232, 238, 246, 0.75));
                }
            }
        }
        if st.phase == Phase::Part {
            // The stores: the stack of plates, and the trolley to carry one to the wall.
            self.draw_stack(g, wall);
            self.draw_trolley(g);
            if p.on_trolley {
                self.draw_plate(g, p, &wall.bolts);
            }
            if let Some(cr) = &st.carry {
                let pl = Plate { x: cr.x, y: cr.y, a: 0.0, scale: SMALL, ..Plate::default() };
                self.draw_plate(g, &pl, &wall.bolts);
                let col = if cr.held { c::AMBER } else { rgba(232, 238, 246, 0.35) };
                g.ring(cr.x, cr.y, 70.0 + 3.0 * (t * 5.0).sin(), col, 3.0);
            }
        }
        if st.phase == Phase::Weld || (st.phase == Phase::Bolt && !wall.seams.is_empty()) {
            self.draw_seams(g, wall);
        }
        if st.phase == Phase::Weld {
            for h in &st.hiss {
                for j in 0..8 {
                    let ph = (h[2] * 1.3 + j as f32 / 8.0) % 1.0;
                    let a = 0.45 * (1.0 - ph) * (1.0 - h[2] / 2.5).max(0.0);
                    g.disc(
                        h[0] + (j as f32 * 2.1).sin() * 8.0 + ph * 40.0,
                        h[1] - ph * 50.0,
                        4.0 + ph * 12.0,
                        rgba(225, 235, 245, a),
                    );
                }
            }
            self.draw_gauge(g, true, st.heat);
            self.draw_torch(g, st.tip, st.heat > st.burn, t);
        }
        if st.phase == Phase::Cut && !wall.cut {
            let speed = st.line.as_ref().map_or(0.0, |l| l.speed);
            self.draw_gauge(g, false, speed / (CUT_FAST * 1.5));
            self.draw_torch(g, st.tip, false, t);
        }
        if st.phase == Phase::Bolt {
            let nx = wall.next_bolt();
            let order = star_order(8);
            for (i, b) in wall.bolts.iter().enumerate() {
                let [x, y] = hole_xy(i, 0.0);
                if !b.on {
                    let n = order.iter().position(|&o| o == i).unwrap_or(0) + 1;
                    g.order_badge(x, y, 13.0, n, Some(i) == nx, t);
                }
                if st.keys && st.sel == i {
                    dashed_ring(g, x, y, 30.0, 3.0, c::AMBER, 7.0, 5.0);
                }
                if st.wrong.is_some_and(|w| w.0 == i) {
                    g.cross(x, y, 16.0, 5.0, c::DANGER);
                }
            }
        }
        for p in &st.sparks {
            g.disc(p.x, p.y, 2.2, p.col);
        }
    }

    fn hand(&mut self, _cx: &Ctx, _t: f32) -> Input {
        let dt = 1.0 / 60.0;
        let Some(wall) = self.wall.as_ref() else { return Input::default() };
        let (st, hm) = (&self.st, &mut self.hm);
        if hm.rest < HAND_REST_FRAMES {
            hm.rest += 1;
            return Input::default();
        }
        let toward = |from: [f32; 2], to: [f32; 2]| {
            let d = (to[0] - from[0]).hypot(to[1] - from[1]);
            let k = (HAND_CARRY_PX / d.max(1e-6)).min(1.0);
            ([from[0] + (to[0] - from[0]) * k, from[1] + (to[1] - from[1]) * k], d)
        };
        match st.phase {
            Phase::Part => {
                // Pick the plate off the stack and carry it down onto the trolley.
                let Some(p) = &st.carry else { return Input::default() };
                if !p.held {
                    return Input::hold(p.x, p.y, true);
                }
                let to = [TROLLEY[0] + TROLLEY[2] / 2.0, TROLLEY[1] + 90.0];
                let (next, d) = toward([p.x, p.y], to);
                if d < 1.0 {
                    return Input::release(to[0], to[1]);
                }
                Input::hold(next[0], next[1], false)
            }
            Phase::Cut => {
                // Trace the line once round, steadily, under the scoring speed.
                let Some(l) = &st.line else { return Input::default() };
                if wall.cut {
                    return Input::default();
                }
                let first = !hm.down;
                hm.down = true;
                if !first {
                    hm.s += HAND_CUT_PX_S * dt;
                }
                let [x, y] = l.at(hm.s);
                Input::hold(x, y, first)
            }
            Phase::Fit => {
                let p = &wall.plate;
                if p.fitted {
                    return Input::default();
                }
                if !p.placed {
                    // Carry the plate from the trolley into the opening and let go there.
                    if st.grab != Some(Grab::Carry) {
                        return Input::hold(p.x, p.y, true);
                    }
                    let (next, d) = toward([p.x, p.y], OC);
                    if d < 1.0 {
                        return Input::release(OC[0], OC[1]);
                    }
                    return Input::hold(next[0], next[1], false);
                }
                // In: wait for it to settle, then turn it by its rim until the holes meet.
                if st.grab != Some(Grab::Turn) {
                    if p.scale < 0.97 || (p.x - OC[0]).hypot(p.y - OC[1]) > 2.0 {
                        return Input::default();
                    }
                    hm.a = -FRAC_PI_4;
                    return Input::hold(OC[0] + hm.a.cos() * 200.0, OC[1] + hm.a.sin() * 200.0, true);
                }
                hm.a += (-wrap_q(p.a)).clamp(-HAND_TURN_RAD, HAND_TURN_RAD);
                Input::hold(OC[0] + hm.a.cos() * 200.0, OC[1] + hm.a.sin() * 200.0, false)
            }
            Phase::Weld => {
                // Each seam in turn: press a little in from its corner, run to the far end and back at a steady speed
                // (the back pass re-lays the cold start), lift, rest a moment, the next.
                let Some(i) = wall.seams.iter().position(|q| !q.welded()) else { return Input::default() };
                if hm.down && i != hm.seam {
                    hm.down = false;
                    hm.wait = 15;
                    let [x, y] = wall.seams[hm.seam].at(hm.s);
                    return Input::release(x, y);
                }
                if hm.wait > 0 {
                    hm.wait -= 1;
                    return Input::default();
                }
                let l = &wall.seams[i];
                if !hm.down {
                    hm.down = true;
                    hm.seam = i;
                    hm.s = 20.0;
                    hm.dir = 1.0;
                    let [x, y] = l.at(hm.s);
                    return Input::hold(x, y, true);
                }
                hm.s += hm.dir * HAND_WELD_PX_S * dt;
                if hm.s >= l.len {
                    hm.s = l.len;
                    hm.dir = -1.0;
                } else if hm.s <= 0.0 {
                    hm.s = 0.0;
                    hm.dir = 1.0;
                }
                let [x, y] = l.at(hm.s);
                Input::hold(x, y, false)
            }
            Phase::Bolt => {
                // Tap the lit bolt, a beat apart.
                if hm.down {
                    hm.down = false;
                    return Input::release(-1.0, -1.0);
                }
                let Some(b) = wall.next_bolt() else { return Input::default() };
                hm.n += 1;
                if hm.n % 12 != 0 {
                    return Input::default();
                }
                hm.down = true;
                let [x, y] = hole_xy(b, 0.0);
                Input::hold(x, y, true)
            }
        }
    }

    fn guide_now(&self) -> Option<usize> {
        match self.st.phase {
            Phase::Part => None,
            Phase::Cut => Some(0),
            Phase::Fit => Some(1),
            Phase::Weld => Some(2),
            Phase::Bolt => Some(3),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::games::tests::{fumble_check, open, plays_to_end};
    use crate::kit::Input;
    use sc_core::repair::State;

    #[test]
    fn a_steady_hand_repairs_it_damaged_disabled_and_destroyed() {
        plays_to_end("hull");
    }

    #[test]
    fn holding_the_torch_still_on_a_seam_burns_through() {
        // A shadow of the same job, played by the steady hand in step with the real one through the cut and the fit;
        // once the weld opens, the torch is pressed on the top seam and held still until it burns through.
        let mut shadow = open("hull", State::Damaged);
        let mut pressed = false;
        fumble_check("hull", move |r| {
            if r.phase < 2 {
                let i = shadow.hand();
                shadow.update(1.0 / 60.0, &i);
                return i;
            }
            let first = !pressed;
            pressed = true;
            Input::hold(super::OC[0], super::OC[1] - super::PLATE_H, first)
        });
    }
}
