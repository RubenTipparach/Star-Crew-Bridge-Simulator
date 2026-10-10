//! The shuttle's repair (repair-minigames design 2 and 6f), from `docs/mockups/repairs/shuttle.js`.
//!
//! The Petrel from above, nose right, its fuel lines drawn on the hull as pipes full of fuel flowing out of the tank
//! amidships to the two main engines aft and the four RCS quads. One line leaks somewhere. A round is four stages:
//! find (drag the leak sniffer along the lines; its ring pulses faster, bigger and warmer the nearer the leak, and
//! held on the leak a moment it marks it), isolate (tap the valve on the tank's side of the leak; the lines past it
//! empty), patch (drag the patch from the kit onto the leak), and the pressure test (tap the shut valve to open it,
//! then hold the pump and keep the needle in the green band for 3 s; leaving the band restarts the 3 s). A valve that
//! leaves the leak still fed is the fumble: fuel mist in the bay, and the valve springs back open. Level 1 has the
//! simple valve set, later levels the full network with the leak deeper in it, a narrower band and a faster decay. A
//! disabled shuttle's first step fits the new isolation valve into the gap in the main line. Keys: the arrows move the
//! sniffer along the lines, then pick a valve; Space closes it, fits the patch, opens it and pumps while held.

use std::f32::consts::{FRAC_PI_2, PI, TAU};

use egui::{Color32, Key};

use crate::kit::{nearest, poly_at, poly_len, Ctx, Game, Input, BAR_H, H, TOUCH_R, W};
use crate::pen::{c, hex, rgba, Align, Pen};

/// In the engine.
pub const PORTED: bool = true;

/// A fresh game.
pub fn new() -> Box<dyn Game> {
    Box::new(Shuttle::default())
}

type Pt = [f32; 2];

const AFT: Pt = [450.0, 410.0];
const FWD: Pt = [630.0, 410.0];
const HULL: [Pt; 13] = [
    [968.0, 410.0],
    [946.0, 322.0],
    [892.0, 262.0],
    [760.0, 222.0],
    [210.0, 210.0],
    [92.0, 222.0],
    [60.0, 272.0],
    [60.0, 548.0],
    [92.0, 598.0],
    [210.0, 610.0],
    [760.0, 598.0],
    [892.0, 558.0],
    [946.0, 498.0],
];
const RCS: [Pt; 4] = [[250.0, 230.0], [250.0, 590.0], [840.0, 262.0], [840.0, 558.0]];
/// The gauge: centre and radius.
const G: [f32; 3] = [1128.0, 250.0, 96.0];
/// The sniffer's signal bars.
const SIG: Pt = [1128.0, 396.0];
/// The pump: centre and radius.
const PUMP: [f32; 3] = [1128.0, 524.0, 58.0];
/// The patch kit (and the part step's crate).
const KITBOX: Pt = [1128.0, 640.0];
/// The hold band's centre, of the gauge's full scale.
const BAND: f32 = 0.6;
/// The needle held in band this long passes the test.
const HOLD_S: f32 = 3.0;
const SNIFF_HOME: Pt = [425.0, 410.0];
/// How far off the sniffer still warms, px.
const SNIFF_HEAT_PX: f32 = 360.0;
/// How near the mist shows, px.
const SNIFF_MIST_PX: f32 = 90.0;
/// How near counts as on the leak, px.
const SNIFF_LOCK_PX: f32 = 22.0;
/// How long on it marks it.
const SNIFF_LOCK_S: f32 = 0.45;
/// The sniffer's speed on the keys.
const SNIFF_KEYS_PX_S: f32 = 260.0;
const FUEL: Color32 = hex(0xb9822f);
const FUEL_LIT: Color32 = hex(0xffd27a);
const EMPTY: Color32 = hex(0x070b12);
const PIPE: Color32 = hex(0x56657c);

/// A valve: its parent (None: the tank), the line from the parent to it, and (with no children) the lines on to what
/// it feeds.
#[derive(Clone, Debug)]
struct Valve {
    parent: Option<usize>,
    route: Vec<Pt>,
    out: Vec<Vec<Pt>>,
}

fn v(parent: Option<usize>, route: Vec<Pt>, out: Vec<Vec<Pt>>) -> Valve {
    Valve { parent, route, out }
}

fn to_v2() -> Vec<Pt> {
    vec![[395.0, 410.0], [340.0, 410.0], [340.0, 320.0], [290.0, 320.0]]
}
fn to_v3() -> Vec<Pt> {
    vec![[395.0, 410.0], [340.0, 410.0], [340.0, 500.0], [290.0, 500.0]]
}
fn fwd_p() -> Vec<Pt> {
    vec![[690.0, 410.0], [740.0, 410.0], [740.0, 300.0], [800.0, 300.0]]
}
fn fwd_s() -> Vec<Pt> {
    vec![[690.0, 410.0], [740.0, 410.0], [740.0, 520.0], [800.0, 520.0]]
}

/// The full network, from level 2.
fn full() -> Vec<Valve> {
    vec![
        v(None, vec![AFT, [395.0, 410.0]], vec![]),
        v(Some(0), to_v2(), vec![]),
        v(Some(0), to_v3(), vec![]),
        v(Some(1), vec![[290.0, 320.0], [180.0, 320.0]], vec![vec![[180.0, 320.0], [72.0, 320.0]]]),
        v(Some(2), vec![[290.0, 500.0], [180.0, 500.0]], vec![vec![[180.0, 500.0], [72.0, 500.0]]]),
        v(Some(1), vec![[290.0, 320.0], [250.0, 320.0], [250.0, 275.0]], vec![vec![[250.0, 275.0], [250.0, 238.0]]]),
        v(Some(2), vec![[290.0, 500.0], [250.0, 500.0], [250.0, 545.0]], vec![vec![[250.0, 545.0], [250.0, 582.0]]]),
        v(None, vec![FWD, [690.0, 410.0]], vec![]),
        v(Some(7), fwd_p(), vec![vec![[800.0, 300.0], [840.0, 300.0], [840.0, 270.0]]]),
        v(Some(7), fwd_s(), vec![vec![[800.0, 520.0], [840.0, 520.0], [840.0, 550.0]]]),
    ]
}

/// The simple valve set, at level 1 and on the part step.
fn simple() -> Vec<Valve> {
    let ext = |mut a: Vec<Pt>, b: &[Pt]| {
        a.extend_from_slice(b);
        a
    };
    vec![
        v(None, vec![AFT, [395.0, 410.0]], vec![]),
        v(
            Some(0),
            to_v2(),
            vec![vec![[290.0, 320.0], [72.0, 320.0]], vec![[290.0, 320.0], [250.0, 320.0], [250.0, 238.0]]],
        ),
        v(
            Some(0),
            to_v3(),
            vec![vec![[290.0, 500.0], [72.0, 500.0]], vec![[290.0, 500.0], [250.0, 500.0], [250.0, 582.0]]],
        ),
        v(
            None,
            vec![FWD, [690.0, 410.0]],
            vec![ext(fwd_p(), &[[840.0, 300.0], [840.0, 270.0]]), ext(fwd_s(), &[[840.0, 520.0], [840.0, 550.0]])],
        ),
    ]
}

fn pos(v: &Valve) -> Pt {
    v.route[v.route.len() - 1]
}

fn kids(vs: &[Valve], i: Option<usize>) -> Vec<usize> {
    (0..vs.len()).filter(|&k| vs[k].parent == i).collect()
}

/// Is region `owner` (a valve's outflow, or None the tank's) past valve `v`?
fn past(vs: &[Valve], owner: Option<usize>, v: usize) -> bool {
    let mut o = owner;
    while let Some(k) = o {
        if k == v {
            return true;
        }
        o = vs[k].parent;
    }
    false
}

/// The lines a valve's outflow holds: its children's lines and what it feeds.
fn region(vs: &[Valve], o: Option<usize>) -> Vec<Vec<Pt>> {
    let mut r: Vec<Vec<Pt>> = kids(vs, o).iter().map(|&k| vs[k].route.clone()).collect();
    if let Some(o) = o {
        r.extend(vs[o].out.iter().cloned());
    }
    r
}

/// Every line on the ship, the sniffer's track (across the tank too, so keys can carry it fore and aft).
fn all_lines(vs: &[Valve]) -> Vec<Vec<Pt>> {
    let mut l = vec![vec![AFT, FWD]];
    for v in vs {
        l.push(v.route.clone());
        l.extend(v.out.iter().cloned());
    }
    l
}

fn depth(vs: &[Valve], o: usize) -> usize {
    let mut n = 0;
    let mut k = Some(o);
    while let Some(i) = k {
        n += 1;
        k = vs[i].parent;
    }
    n
}

/// Where the leak is and whose outflow it is in.
#[derive(Clone, Copy, Debug, Default)]
struct Leak {
    owner: usize,
    x: f32,
    y: f32,
    vert: bool,
}

/// The round's leak: in a valve's outflow (so some valve can cut it off), at least `deep` valves down (or as deep as
/// the network goes), on a straight stretch clear of the bends (a patch lies flat on a straight pipe). `f` is the
/// round's stream, in [0, 1).
fn place_leak(vs: &[Valve], deep: f32, mut f: impl FnMut() -> f32) -> Leak {
    let mut int = |n: usize| ((f() * n as f32) as usize).min(n.saturating_sub(1));
    let owners: Vec<usize> = (0..vs.len()).filter(|&k| !region(vs, Some(k)).is_empty()).collect();
    let max_d = owners.iter().map(|&o| depth(vs, o)).max().unwrap_or(1);
    let need = (deep.round().max(0.0) as usize).min(max_d);
    let pool: Vec<usize> = owners.into_iter().filter(|&o| depth(vs, o) >= need).collect();
    let owner = pool[int(pool.len())];
    let runs: Vec<(Pt, Pt)> = region(vs, Some(owner))
        .iter()
        .flat_map(|l| l.windows(2).map(|w| (w[0], w[1])).collect::<Vec<_>>())
        .filter(|(a, b)| (b[0] - a[0]).hypot(b[1] - a[1]) >= 36.0)
        .collect();
    let (a, b) = runs[int(runs.len())];
    let k = 0.35 + 0.3 * f();
    Leak { owner, x: a[0] + (b[0] - a[0]) * k, y: a[1] + (b[1] - a[1]) * k, vert: (b[0] - a[0]).abs() < 1.0 }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Phase {
    Part,
    #[default]
    Find,
    Isolate,
    Drain,
    Patch,
    Open,
    Hold,
    Done,
}

#[derive(Clone, Copy, Debug, Default)]
struct Carry {
    x: f32,
    y: f32,
    held: bool,
    set: bool,
    gx: f32,
    gy: f32,
}

#[derive(Clone, Copy, Debug, Default)]
struct Sniffer {
    x: f32,
    y: f32,
    held: bool,
}

#[derive(Clone, Copy, Debug, Default)]
struct Puff {
    x: f32,
    y: f32,
    t: f32,
}

/// The game's state.
#[derive(Default)]
pub struct Shuttle {
    valves: Vec<Valve>,
    phase: Phase,
    part_step: bool,
    closed: Option<usize>,
    spring: f32,
    drain: f32,
    refill: f32,
    leak: Leak,
    p: f32,
    flow: f32,
    hold: f32,
    hw: f32,
    decay: f32,
    clock: f32,
    patch: Carry,
    puffs: Vec<Puff>,
    kf: bool,
    fi: usize,
    sn: Sniffer,
    lock: f32,
    /// The hand: the way along the lines to the leak, how far along it, and whether it is pressing.
    hand_path: Vec<Pt>,
    hand_s: f32,
    hand_down: bool,
}

impl Shuttle {
    /// The nearest point on any line to (x, y): the sniffer rides the lines.
    fn snap(&self, x: f32, y: f32) -> Pt {
        let mut best = [x, y];
        let mut bd = f32::INFINITY;
        for line in all_lines(&self.valves) {
            for w in line.windows(2) {
                let ([ax, ay], [bx, by]) = (w[0], w[1]);
                let (vx, vy) = (bx - ax, by - ay);
                let l = vx * vx + vy * vy;
                let l = if l == 0.0 { 1.0 } else { l };
                let k = (((x - ax) * vx + (y - ay) * vy) / l).clamp(0.0, 1.0);
                let (qx, qy) = (ax + vx * k, ay + vy * k);
                let d = (x - qx).hypot(y - qy);
                if d < bd {
                    bd = d;
                    best = [qx, qy];
                }
            }
        }
        best
    }

    /// How near the sniffer is to the leak, 0 (cold) to 1 (on it).
    fn warmth(&self) -> f32 {
        (1.0 - (self.sn.x - self.leak.x).hypot(self.sn.y - self.leak.y) / SNIFF_HEAT_PX).max(0.0)
    }

    /// Lines past the shut valve, and how empty they are (0 full, 1 empty).
    fn emptied(&self, owner: Option<usize>) -> f32 {
        match self.closed {
            Some(cl) if self.spring <= 0.0 && past(&self.valves, owner, cl) => self.drain * (1.0 - self.refill),
            _ => 0.0,
        }
    }

    fn puff(&mut self, x: f32, y: f32) {
        self.puffs.push(Puff { x, y, t: 0.0 });
    }

    /// Close valve i: on the tank's side of the leak it isolates it; anywhere else the leak is still fed.
    fn close(&mut self, cx: &mut Ctx, i: usize) {
        if self.spring > 0.0 {
            return;
        }
        let [x, y] = pos(&self.valves[i]);
        if !past(&self.valves, Some(self.leak.owner), i) {
            self.closed = Some(i);
            self.spring = 0.45;
            self.puff(self.leak.x, self.leak.y);
            self.puff(x, y);
            cx.fumble("Fuel mist: the bay's fire risk rises");
            return;
        }
        self.closed = Some(i);
        self.drain = 0.0;
        self.phase = Phase::Drain;
    }

    /// The patch (or the part) in the hand: true when it is dropped on (tx, ty), or fitted from the keys.
    fn drag_patch(&mut self, input: &Input, ease: f32, tx: f32, ty: f32) -> bool {
        let p = &mut self.patch;
        if input.pressed && (input.x - p.x).abs() < 44.0 && (input.y - p.y).abs() < 30.0 {
            p.held = true;
            p.gx = input.x - p.x;
            p.gy = input.y - p.y;
        }
        if p.held && input.down {
            p.x = input.x - p.gx;
            p.y = input.y - p.gy;
        }
        if p.held && input.released {
            p.held = false;
            if (p.x - tx).hypot(p.y - ty) < 44.0 {
                return true;
            }
        }
        if !p.held {
            p.x += (KITBOX[0] - p.x) * ease;
            p.y += (KITBOX[1] - p.y) * ease;
        }
        self.kf && input.action_pressed
    }

    /// The hand's way to the leak: from the sniffer's rest along the valves' routes from the tank, then the leak's
    /// own line to it.
    fn route_to_leak(&self) -> Vec<Pt> {
        let vs = &self.valves;
        let mut chain = Vec::new();
        let mut o = Some(self.leak.owner);
        while let Some(k) = o {
            chain.push(k);
            o = vs[k].parent;
        }
        chain.reverse();
        let mut path = vec![SNIFF_HOME];
        if vs[chain[0]].route[0] == FWD {
            path.push(FWD);
        }
        for &k in &chain {
            path.extend(vs[k].route.iter().skip(1));
        }
        let (lx, ly) = (self.leak.x, self.leak.y);
        'lines: for line in region(vs, Some(self.leak.owner)) {
            for (i, w) in line.windows(2).enumerate() {
                let ([ax, ay], [bx, by]) = (w[0], w[1]);
                let on = (lx - ax).hypot(ly - ay) + (bx - lx).hypot(by - ly) - (bx - ax).hypot(by - ay);
                if on.abs() < 0.5 {
                    path.extend_from_slice(&line[1..=i]);
                    break 'lines;
                }
            }
        }
        path.push([lx, ly]);
        path
    }

    /// The hand taps (x, y): a press one frame, a release the next.
    fn hand_tap(&mut self, [x, y]: Pt) -> Input {
        if self.hand_down {
            self.hand_down = false;
            return Input::release(x, y);
        }
        self.hand_down = true;
        Input::hold(x, y, true)
    }

    /// The hand carries the patch (or the part) to `to` and lets go there.
    fn hand_drag(&mut self, to: Pt) -> Input {
        let p = self.patch;
        if !p.held {
            if self.hand_down {
                self.hand_down = false;
                return Input::release(p.x, p.y);
            }
            self.hand_down = true;
            return Input::hold(p.x, p.y, true);
        }
        let d = (to[0] - p.x).hypot(to[1] - p.y);
        if d > 2.0 {
            let k = (20.0 / d).min(1.0);
            return Input::hold(p.x + (to[0] - p.x) * k, p.y + (to[1] - p.y) * k, false);
        }
        self.hand_down = false;
        Input::release(to[0], to[1])
    }
}

impl Game for Shuttle {
    fn id(&self) -> &'static str {
        "shuttle"
    }

    fn knobs(&self) -> &'static [&'static str] {
        &["valves_count", "leak_depth_count", "band_half_frac", "decay_frac_s"]
    }

    fn step(&mut self, cx: &mut Ctx) {
        let mut r = cx.dice();
        let net = full();
        self.valves = if !cx.part && cx.knob("valves_count") >= net.len() as f32 { net } else { simple() };
        self.part_step = cx.part;
        self.clock = 0.0;
        self.closed = None;
        self.spring = 0.0;
        self.drain = 0.0;
        self.refill = 0.0;
        self.p = 0.7;
        self.flow = 0.0;
        self.hold = 0.0;
        self.hw = cx.knob("band_half_frac");
        self.decay = cx.knob("decay_frac_s");
        self.puffs.clear();
        self.kf = false;
        self.fi = 0;
        self.lock = 0.0;
        self.sn = Sniffer { x: SNIFF_HOME[0], y: SNIFF_HOME[1], held: false };
        self.patch = Carry { x: KITBOX[0], y: KITBOX[1], ..Carry::default() };
        self.hand_path.clear();
        self.hand_s = 0.0;
        self.hand_down = false;
        if cx.part {
            self.phase = Phase::Part;
            self.p = 0.0;
            self.leak = Leak { owner: 0, x: -999.0, y: -999.0, vert: false };
            return;
        }
        self.phase = Phase::Find;
        self.leak = place_leak(&self.valves, cx.knob("leak_depth_count"), || r.f());
    }

    fn update(&mut self, cx: &mut Ctx, dt: f32, input: &Input) {
        let ease = (dt * 12.0).min(1.0);
        let hit = |a: Key, b: Key| input.hit(a) || input.hit(b);
        let dir = i32::from(hit(Key::ArrowRight, Key::D) || hit(Key::ArrowDown, Key::S))
            - i32::from(hit(Key::ArrowLeft, Key::A) || hit(Key::ArrowUp, Key::W));
        if dir != 0 || input.action_pressed {
            self.kf = true;
        }
        if input.pressed {
            self.kf = false;
        }
        self.clock += dt;
        for q in &mut self.puffs {
            q.t += dt;
        }
        self.puffs.retain(|q| q.t < 1.4);
        if self.spring > 0.0 {
            self.spring -= dt;
            if self.spring <= 0.0 {
                self.spring = 0.0;
                self.closed = None;
            }
        }
        // The leak bleeds the line down while it is fed; the section past a shut valve empties.
        if matches!(self.phase, Phase::Find | Phase::Isolate) {
            self.p = (self.p - 0.04 * dt).max(0.3);
        }
        match self.phase {
            Phase::Part => {
                let [x, y] = pos(&self.valves[0]);
                if self.drag_patch(input, ease, x, y) {
                    self.patch = Carry { x, y, set: true, ..Carry::default() };
                    self.phase = Phase::Done;
                    cx.step_done();
                }
            }
            Phase::Find => {
                // Pick the sniffer up anywhere on the ship; it rides the line nearest the finger.
                if input.pressed && input.x < 990.0 {
                    self.sn.held = true;
                }
                if !input.down {
                    self.sn.held = false;
                }
                if self.sn.held {
                    let [x, y] = self.snap(input.x, input.y);
                    self.sn.x = x;
                    self.sn.y = y;
                }
                let [sx, sy] = input.stick;
                if sx != 0.0 || sy != 0.0 {
                    self.kf = true;
                    let [x, y] =
                        self.snap(self.sn.x + sx * SNIFF_KEYS_PX_S * dt, self.sn.y + sy * SNIFF_KEYS_PX_S * dt);
                    self.sn.x = x;
                    self.sn.y = y;
                }
                if (self.sn.x - self.leak.x).hypot(self.sn.y - self.leak.y) < SNIFF_LOCK_PX {
                    self.lock += dt;
                } else {
                    self.lock = (self.lock - 2.0 * dt).max(0.0);
                }
                if self.lock >= SNIFF_LOCK_S {
                    self.lock = SNIFF_LOCK_S;
                    self.sn.held = false;
                    self.phase = Phase::Isolate;
                    self.fi = 0;
                }
            }
            Phase::Isolate => {
                if self.kf {
                    let n = self.valves.len() as i32;
                    self.fi = (self.fi as i32 + dir).rem_euclid(n) as usize;
                    if input.action_pressed {
                        self.close(cx, self.fi);
                    }
                } else if input.pressed {
                    // The nearest valve within a fingertip's reach; no two valves are closer than 60 px.
                    let pts: Vec<Option<Pt>> = self.valves.iter().map(|v| Some(pos(v))).collect();
                    if let Some(i) = nearest(&pts, input.x, input.y, TOUCH_R + 6.0) {
                        self.close(cx, i);
                    }
                }
            }
            Phase::Drain => {
                self.drain = (self.drain + dt / 0.7).min(1.0);
                self.p += (0.05 - self.p) * dt * 4.0;
                if self.drain >= 1.0 {
                    self.phase = Phase::Patch;
                }
            }
            Phase::Patch => {
                self.p += (0.05 - self.p) * dt * 4.0;
                let (lx, ly) = (self.leak.x, self.leak.y);
                if self.drag_patch(input, ease, lx, ly) {
                    self.patch = Carry { x: lx, y: ly, set: true, ..Carry::default() };
                    self.phase = Phase::Open;
                }
            }
            Phase::Open => {
                // Open the shut valve again: the line refills behind the patch, and the test begins.
                if let Some(cl) = self.closed {
                    let at = pos(&self.valves[cl]);
                    let tapped = input.pressed && nearest(&[Some(at)], input.x, input.y, TOUCH_R + 6.0) == Some(0);
                    if (self.kf && input.action_pressed) || tapped {
                        self.phase = Phase::Hold;
                        self.p = 0.2;
                    }
                }
            }
            Phase::Hold => {
                self.refill = (self.refill + dt / 0.5).min(1.0);
                let pumping = input.held(Key::Space)
                    || input.held(Key::Enter)
                    || input.held(Key::ArrowUp)
                    || input.held(Key::W)
                    || (input.down && input.dist(PUMP[0], PUMP[1]) < PUMP[2]);
                let want = if pumping { 0.36 } else { 0.0 };
                self.flow += (want - self.flow) * (dt * 4.0).min(1.0);
                self.p = (self.p + (self.flow - self.decay - 0.03 * (self.clock * 1.1).sin()) * dt).clamp(0.0, 1.0);
                if (self.p - BAND).abs() <= self.hw {
                    self.hold += dt;
                } else {
                    self.hold = 0.0;
                }
                if self.hold >= HOLD_S {
                    self.hold = HOLD_S;
                    self.phase = Phase::Done;
                    cx.step_done();
                }
            }
            Phase::Done => {}
        }
    }

    fn draw(&self, g: &Pen, _cx: &Ctx, t: f32, _input: &Input) {
        g.rect(0.0, BAR_H, W, H, hex(0x070b12));
        draw_hull(g);
        // The lines as pipes: full of fuel flowing out of the tank, or hollow past a shut valve.
        for (i, v) in self.valves.iter().enumerate() {
            // The part step: a gap in the main line where the new valve goes.
            let route: Vec<Pt> = if !self.part_step || self.patch.set {
                v.route.clone()
            } else if i == 0 {
                vec![AFT, [417.0, 410.0]]
            } else if v.parent == Some(0) {
                let mut r = vec![[373.0, 410.0]];
                r.extend_from_slice(&v.route[1..]);
                r
            } else {
                v.route.clone()
            };
            pipe(g, &route, self.emptied(v.parent), t);
            for o in &v.out {
                pipe(g, o, self.emptied(Some(i)), t);
            }
        }
        // The tank.
        g.round(450.0, 372.0, 180.0, 76.0, 38.0, Some(hex(0x1d2738)), Some((3.0, c::STEEL)));
        g.round(458.0, 380.0, 164.0, 60.0, 30.0, Some(rgba(185, 130, 47, 0.55)), None);
        g.text("FUEL", 540.0, 410.0, 20.0, c::FG, Align::Center);
        // The valves: a bow tie, hollow open, solid shut.
        for (i, v) in self.valves.iter().enumerate() {
            let [x, y] = pos(v);
            let prev = v.route[v.route.len() - 2];
            let vert = (prev[0] - x).abs() < 1.0;
            if self.part_step && i == 0 && !self.patch.set {
                bowtie(g, x, y, vert, hex(0x0b111b), None);
                let mut pts = bowtie_pts(x, y, vert).to_vec();
                pts.push(pts[0]);
                g.dashed(&pts, 3.0, c::AMBER, 6.0, 5.0);
                continue;
            }
            let shut = self.closed == Some(i) && !matches!(self.phase, Phase::Hold | Phase::Done);
            let wrong = shut && self.spring > 0.0;
            let fill = if wrong {
                c::DANGER
            } else if shut {
                hex(0xc9d3df)
            } else {
                hex(0x0b111b)
            };
            bowtie(g, x, y, vert, fill, Some(if wrong { c::DANGER } else { hex(0xc9d3df) }));
            if self.phase == Phase::Isolate && self.kf && self.fi == i {
                dashed_ring(g, x, y, 26.0, c::AMBER, 3.0, 6.0, 5.0);
            }
            if self.phase == Phase::Open && self.closed == Some(i) {
                g.disc(x, y, 30.0, rgba(79, 195, 247, 0.18));
                g.ring(x, y, 34.0 + 3.0 * (t * 6.0).sin(), c::ACCENT, 3.0);
            }
        }
        // The leak: a faint mist once the sniffer is within reach; marked and spraying once found, until cut off.
        if !self.part_step {
            let lk = self.leak;
            let d = (self.sn.x - lk.x).hypot(self.sn.y - lk.y);
            let fed =
                matches!(self.phase, Phase::Find | Phase::Isolate) || (self.phase == Phase::Drain && self.drain < 1.0);
            let mist = if self.phase == Phase::Find {
                ((1.0 - d / SNIFF_MIST_PX).max(0.0) * 1.1).min(0.7)
            } else if fed {
                1.0 - if self.phase == Phase::Drain { self.drain } else { 0.0 }
            } else {
                0.0
            };
            if mist > 0.0 {
                for k in 0..7 {
                    let a = t * 1.3 + k as f32 * 0.9;
                    let rr = 10.0 + (t * 30.0 + k as f32 * 13.0) % 40.0;
                    g.disc(
                        lk.x + a.cos() * rr * 0.8,
                        lk.y + a.sin() * rr * 0.6 - rr * 0.4,
                        7.0 + rr * 0.25,
                        rgba(255, 232, 170, 0.6 * mist * (1.0 - rr / 50.0)),
                    );
                }
            }
            if self.phase != Phase::Find && !self.patch.set {
                dashed_ring(g, lk.x, lk.y, 20.0, c::DANGER, 3.0, 7.0, 6.0);
                let q = g.translate(lk.x, lk.y);
                let q = if lk.vert { q.rotate(FRAC_PI_2) } else { q };
                q.path(&[[-9.0, -6.0], [-3.0, 2.0], [2.0, -3.0], [9.0, 6.0]], false, 3.0, c::DANGER);
            }
            if self.phase == Phase::Patch {
                g.ring(lk.x, lk.y, 30.0 + 3.0 * (t * 6.0).sin(), c::ACCENT, 3.0);
            }
        }
        for q in &self.puffs {
            for k in 0..6 {
                let (a, rr) = (k as f32 * 1.05, 8.0 + q.t * 50.0);
                g.disc(
                    q.x + a.cos() * rr,
                    q.y + a.sin() * rr,
                    6.0 + q.t * 10.0,
                    rgba(255, 228, 160, 0.5 * (1.0 - q.t / 1.4)),
                );
            }
        }
        if self.phase == Phase::Find {
            self.draw_sniffer(g, t);
        }
        self.draw_panel(g);
        if self.part_step {
            if !self.patch.set {
                let (x, y) = (self.patch.x, self.patch.y);
                g.rect(x - 24.0, y - 16.0, 6.0, 32.0, c::STEEL);
                g.rect(x + 18.0, y - 16.0, 6.0, 32.0, c::STEEL);
                bowtie(g, x, y, false, hex(0xc9d3df), Some(c::ACCENT));
            }
        } else {
            let q = g.translate(self.patch.x, self.patch.y);
            let q = if self.patch.set && self.leak.vert { q.rotate(FRAC_PI_2) } else { q };
            q.round(-30.0, -13.0, 60.0, 26.0, 8.0, Some(c::COPPER), Some((2.0, hex(0xf0c08a))));
            q.rect(-20.0, -13.0, 6.0, 26.0, hex(0x7a4a22));
            q.rect(14.0, -13.0, 6.0, 26.0, hex(0x7a4a22));
        }
    }

    fn hand(&mut self, _cx: &Ctx, _t: f32) -> Input {
        match self.phase {
            Phase::Part => {
                let to = pos(&self.valves[0]);
                self.hand_drag(to)
            }
            Phase::Find => {
                // Run the sniffer along the lines to the leak at 300 px/s, and hold it there.
                if self.hand_path.is_empty() {
                    self.hand_path = self.route_to_leak();
                    self.hand_s = 0.0;
                }
                let len = poly_len(&self.hand_path).max(1.0);
                let [x, y, _] = poly_at(&self.hand_path, (self.hand_s / len).min(1.0));
                self.hand_s += 5.0;
                let pressed = !self.hand_down;
                self.hand_down = true;
                Input::hold(x, y, pressed)
            }
            // The leak's own valve is always on the tank's side of it.
            Phase::Isolate => {
                let at = pos(&self.valves[self.leak.owner]);
                self.hand_tap(at)
            }
            Phase::Patch => {
                let to = [self.leak.x, self.leak.y];
                self.hand_drag(to)
            }
            Phase::Open => match self.closed {
                Some(cl) => {
                    let at = pos(&self.valves[cl]);
                    self.hand_tap(at)
                }
                None => Input::default(),
            },
            Phase::Hold => {
                // Pump to the flow that holds the needle on the band's centre: the decay, its wobble, and a pull
                // towards the centre; the pump's own lag smooths the presses.
                let want = self.decay + 0.03 * (self.clock * 1.1).sin() + 2.0 * (BAND - self.p);
                if self.flow < want.clamp(0.0, 0.36) {
                    let pressed = !self.hand_down;
                    self.hand_down = true;
                    Input::hold(PUMP[0], PUMP[1], pressed)
                } else if self.hand_down {
                    self.hand_down = false;
                    Input::release(PUMP[0], PUMP[1])
                } else {
                    Input::at(PUMP[0], PUMP[1])
                }
            }
            Phase::Drain | Phase::Done => {
                if self.hand_down {
                    self.hand_down = false;
                    Input::release(self.sn.x, self.sn.y)
                } else {
                    Input::default()
                }
            }
        }
    }

    fn guide_now(&self) -> Option<usize> {
        match self.phase {
            Phase::Find => Some(0),
            Phase::Isolate | Phase::Drain => Some(1),
            Phase::Patch => Some(2),
            Phase::Open | Phase::Hold => Some(3),
            Phase::Part | Phase::Done => None,
        }
    }
}

impl Shuttle {
    /// The leak sniffer: a probe head on the line, its ring pulsing faster, bigger and warmer near the leak.
    fn draw_sniffer(&self, g: &Pen, t: f32) {
        let w = self.warmth();
        let col = heat(w);
        let hz = 0.6 + 5.4 * w * w;
        let (x, y) = (self.sn.x, self.sn.y);
        // The pulse: rings run out from the head; nearer the leak they come faster, reach further and burn brighter.
        for k in 0..2 {
            let f = (t * hz + k as f32 / 2.0) % 1.0;
            let r = 18.0 + f * (26.0 + 44.0 * w);
            g.ring(x, y, r, col((0.35 + 0.65 * w) * (1.0 - f)), 3.0 + 3.0 * w);
        }
        // The wand down to the hand, then the head.
        g.path_round(&[[x + 10.0, y + 10.0], [x + 34.0, y + 34.0]], 6.0, hex(0x9aa6b6));
        g.disc(x, y, 15.0, hex(0x1b2433));
        g.ring(x, y, 15.0, hex(0xc9d3df), 3.0);
        g.disc(x, y, 7.0, col(0.5 + 0.5 * w));
        if self.lock > 0.0 {
            g.arc(x, y, 22.0, -FRAC_PI_2, -FRAC_PI_2 + TAU * self.lock / SNIFF_LOCK_S, 5.0, c::DANGER);
        }
        if self.kf {
            dashed_ring(g, x, y, 28.0, c::AMBER, 2.0, 6.0, 5.0);
        }
    }

    /// The test panel: the gauge, the sniffer's signal, the pump, the patch kit.
    fn draw_panel(&self, g: &Pen) {
        g.panel(1000.0, 92.0, 256.0, 608.0, 18.0, hex(0x0a0f17), c::LINE);
        let ang = |v: f32| (150.0 + 240.0 * v) * PI / 180.0;
        let [gx, gy, gr] = G;
        g.disc(gx, gy, gr, hex(0x0b111b));
        g.ring(gx, gy, gr, c::STEEL, 3.0);
        for k in 0..=10 {
            let a = ang(k as f32 / 10.0);
            let r0 = gr - if k % 5 != 0 { 10.0 } else { 18.0 };
            let (ca, sa) = (a.cos(), a.sin());
            g.line(gx + ca * r0, gy + sa * r0, gx + ca * (gr - 4.0), gy + sa * (gr - 4.0), 2.0, hex(0x4a5a70));
        }
        g.dashed(&Pen::arc_points(gx, gy, gr - 12.0, ang(0.9), ang(1.0)), 8.0, c::DANGER, 5.0, 4.0);
        if matches!(self.phase, Phase::Hold | Phase::Done) {
            g.arc(gx, gy, gr - 14.0, ang(BAND - self.hw), ang(BAND + self.hw), 14.0, c::OK);
            if self.hold > 0.0 {
                g.arc(gx, gy, gr + 10.0, -FRAC_PI_2, -FRAC_PI_2 + TAU * self.hold / HOLD_S, 5.0, c::OK);
            }
        }
        let a = ang(self.p);
        g.path_round(&[[gx, gy], [gx + a.cos() * (gr - 16.0), gy + a.sin() * (gr - 16.0)]], 4.0, c::FG);
        g.disc(gx, gy, 9.0, c::STEEL);
        // The sniffer's signal: five bars, more and warmer the nearer the leak (readable under a finger on the probe).
        let live = self.phase == Phase::Find;
        let w = if live { self.warmth() } else { 0.0 };
        let col = heat(w);
        let n = if live { (w * 5.0).ceil() as usize } else { 0 };
        let [sx, sy] = SIG;
        g.panel(
            sx - 106.0,
            sy - 30.0,
            212.0,
            60.0,
            12.0,
            if live { hex(0x111826) } else { hex(0x0b111b) },
            if live && n > 0 { col(0.8) } else { c::LINE },
        );
        g.disc(sx - 76.0, sy, 12.0, hex(0x1b2433));
        g.ring(sx - 76.0, sy, 12.0, if live { hex(0xc9d3df) } else { hex(0x3a4558) }, 3.0);
        g.disc(sx - 76.0, sy, 5.0, if live { col(0.9) } else { hex(0x2a3446) });
        for k in 0..5 {
            let bh = 10.0 + k as f32 * 8.0;
            let bx = sx - 46.0 + k as f32 * 30.0;
            g.rect(bx, sy + 22.0 - bh, 20.0, bh, if k < n { col(1.0) } else { hex(0x222b3a) });
        }
        // The pump: the round's one action once the valve is open again.
        let on = self.phase == Phase::Hold;
        let [px, py, pr] = PUMP;
        g.disc(px, py, pr, if on { hex(0x262e42) } else { hex(0x121822) });
        let rim = if on && self.flow > 0.18 {
            c::AMBER
        } else if on {
            c::STEEL
        } else {
            c::LINE
        };
        g.ring(px, py, pr, rim, 4.0);
        g.text("PUMP", px, py, 24.0, if on { c::FG } else { hex(0x3a4558) }, Align::Center);
        let lit = matches!(self.phase, Phase::Patch | Phase::Part);
        g.panel(
            KITBOX[0] - 80.0,
            KITBOX[1] - 36.0,
            160.0,
            72.0,
            12.0,
            hex(0x141b27),
            if lit { c::ACCENT } else { c::LINE },
        );
    }
}

/// The sniffer's colour: cold blue far off, yellow nearer, red on the leak; returns the colour at an alpha.
fn heat(w: f32) -> impl Fn(f32) -> Color32 {
    let mix = |a: [f32; 3], b: [f32; 3], k: f32| -> [u8; 3] {
        [0, 1, 2].map(|i| (a[i] + (b[i] - a[i]) * k).round().clamp(0.0, 255.0) as u8)
    };
    let (blue, yellow, red) = ([79.0, 195.0, 247.0], [255.0, 197.0, 66.0], [255.0, 71.0, 87.0]);
    let col = if w < 0.5 { mix(blue, yellow, w * 2.0) } else { mix(yellow, red, (w - 0.5) * 2.0) };
    move |a| rgba(col[0], col[1], col[2], a)
}

/// The Petrel from above: hull, cockpit, engines, RCS quads, side hatch.
fn draw_hull(g: &Pen) {
    g.poly(&HULL, hex(0x0f1622));
    g.path(&HULL, true, 3.0, hex(0x2c3a52));
    for x in [330.0, 700.0] {
        g.line(x, 214.0, x, 606.0, 2.0, hex(0x151e2c));
    }
    for y in [-1.0f32, 1.0] {
        let pts =
            [[60.0, 410.0 + y * 60.0], [40.0, 410.0 + y * 40.0], [40.0, 410.0 + y * 140.0], [60.0, 410.0 + y * 120.0]];
        g.poly(&pts, hex(0x1b2433));
        g.path(&pts, true, 2.0, c::STEEL);
    }
    for (y0, y1) in [(350.0, 392.0), (428.0, 470.0)] {
        g.poly(&[[912.0, y0], [944.0, y0 + 12.0], [944.0, y1 - 12.0], [912.0, y1]], hex(0x1d3b52));
    }
    for [x, y] in RCS {
        g.rect(x - 9.0, y - 9.0, 18.0, 18.0, hex(0x1b2433));
        g.rect_stroke(x - 9.0, y - 9.0, 18.0, 18.0, 2.0, c::STEEL);
    }
    g.rect(600.0, 224.0, 64.0, 8.0, hex(0x080c12));
    g.text("PETREL", 520.0, 650.0, 18.0, c::DIM, Align::Center);
}

/// A fuel pipe: a steel wall round fuel that flows outward (moving dashes), or hollow when `empty` reaches 1.
fn pipe(g: &Pen, pts: &[Pt], empty: f32, t: f32) {
    g.path_round(pts, 13.0, PIPE);
    g.path_round(pts, 7.0, EMPTY);
    if empty < 1.0 {
        let a = g.alpha(1.0 - empty);
        a.path_round(pts, 7.0, FUEL);
        flow_dashes(&a, pts, t * 36.0, 5.0, 15.0, 3.0, FUEL_LIT);
    }
}

/// Dashes `dash` on, `gap` off along a polyline, the pattern moved `off` px along it (canvas's `lineDashOffset` of
/// minus `off`), with round caps.
#[allow(clippy::too_many_arguments)]
fn flow_dashes(g: &Pen, pts: &[Pt], off: f32, dash: f32, gap: f32, width: f32, col: Color32) {
    let total = poly_len(pts);
    let period = dash + gap;
    let mut a = off.rem_euclid(period) - period;
    while a < total {
        let (s0, s1) = (a.max(0.0), (a + dash).min(total));
        if s1 > s0 {
            g.path_round(&sub_path(pts, s0, s1), width, col);
        }
        a += period;
    }
}

/// The stretch of a polyline from length `s0` to `s1`.
fn sub_path(pts: &[Pt], s0: f32, s1: f32) -> Vec<Pt> {
    let mut out: Vec<Pt> = Vec::new();
    let mut d = 0.0;
    for w in pts.windows(2) {
        let ([x0, y0], [x1, y1]) = (w[0], w[1]);
        let l = (x1 - x0).hypot(y1 - y0);
        if l > 0.0 && d + l >= s0 && d <= s1 {
            let (k0, k1) = (((s0 - d) / l).clamp(0.0, 1.0), ((s1 - d) / l).clamp(0.0, 1.0));
            if out.is_empty() {
                out.push([x0 + (x1 - x0) * k0, y0 + (y1 - y0) * k0]);
            }
            out.push([x0 + (x1 - x0) * k1, y0 + (y1 - y0) * k1]);
        }
        d += l;
    }
    out
}

/// A valve's bow tie corners, in stroke order.
fn bowtie_pts(x: f32, y: f32, vert: bool) -> [Pt; 4] {
    if vert {
        [[x - 12.0, y - 14.0], [x + 12.0, y - 14.0], [x - 12.0, y + 14.0], [x + 12.0, y + 14.0]]
    } else {
        [[x - 14.0, y - 12.0], [x - 14.0, y + 12.0], [x + 14.0, y - 12.0], [x + 14.0, y + 12.0]]
    }
}

/// A valve: a bow tie, two triangles meeting at its centre, filled and outlined.
fn bowtie(g: &Pen, x: f32, y: f32, vert: bool, fill: Color32, stroke: Option<Color32>) {
    let p = bowtie_pts(x, y, vert);
    g.poly(&[p[0], p[1], [x, y]], fill);
    g.poly(&[p[2], p[3], [x, y]], fill);
    if let Some(s) = stroke {
        g.path(&p, true, 3.0, s);
    }
}

/// A dashed circle.
#[allow(clippy::too_many_arguments)]
fn dashed_ring(g: &Pen, x: f32, y: f32, r: f32, col: Color32, width: f32, dash: f32, gap: f32) {
    g.dashed(&Pen::arc_points(x, y, r, 0.0, TAU), width, col, dash, gap);
}

#[cfg(test)]
mod tests {
    use super::{past, place_leak, pos, simple};
    use crate::games::tests::{data, fumble_check, plays_to_end};
    use crate::kit::Input;

    #[test]
    fn a_steady_hand_repairs_it_damaged_disabled_and_destroyed() {
        plays_to_end("shuttle");
    }

    #[test]
    fn shutting_a_valve_that_leaves_the_leak_fed_mists_the_bay() {
        // The mistake sees only the runner, so it works the leak out as the round placed it: the same seeded stream
        // (the tests' seed 1, the shuttle, round 0, phase 0, no part) through the same placement, at level 1 (the
        // simple valve set). A valve that is neither the leak's own nor upstream of it leaves the leak fed.
        let d = data();
        let file = d.games.get("shuttle").expect("the shuttle's file");
        assert!(file.knob("valves_count", 1) < 10.0, "level 1 plays the simple valve set");
        let mut rng = sc_core::repair::round_rng(1, "shuttle", 0, 0, false);
        let valves = simple();
        let leak = place_leak(&valves, file.knob("leak_depth_count", 1) as f32, || rng.next_f64() as f32);
        let wrong = (0..valves.len()).find(|&v| !past(&valves, Some(leak.owner), v)).expect("a valve off the leak");
        let [wx, wy] = pos(&valves[wrong]);
        let mut n = 0;
        // Hold the sniffer on the leak until it is marked, then tap the wrong valve.
        fumble_check("shuttle", move |_r| {
            n += 1;
            if n < 45 {
                Input::hold(leak.x, leak.y, n == 1)
            } else if n % 2 == 0 {
                Input::release(wx, wy)
            } else {
                Input::hold(wx, wy, true)
            }
        });
    }
}
