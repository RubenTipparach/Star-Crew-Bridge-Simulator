//! An electrical conduit's repair (repair-minigames design 2), from `docs/mockups/repairs/conduits.js`.
//!
//! Two phases. A splice: the conduit is open where it burnt, its wires charred along the middle. Strip the burnt
//! length (drag along it, or hold Space or Right), then join the cut ends colour to colour, each colour with its own tip
//! shape (drag a left end onto a right end, or Up and Down to choose and Space to pick and drop) before the clamp's jaws
//! close; a clamp that shuts first pulls the joins apart. A reroute, every round of a job of four rounds or more
//! (design 1a): the conduit map with the run's junction box burnt out; draw a new run from the switchboard to the load
//! round it and the bulkheads (drag through the cells, or the arrows). Each level has more pairs, a quicker clamp and a
//! busier map. A crossed pair is the fumble: a spark, 5 HP, the breaker trips. A disabled conduit's first step fits a
//! new length: drag it from the crate into the gap.

use egui::{Color32, Key};

use crate::kit::{nearest, Ctx, Dice, Game, Input, BAR_H, H, TOUCH_R, W};
use crate::pen::{c, hex, mix, rgba, round_rect_points, Align, Pen};

/// In the engine.
pub const PORTED: bool = true;

/// A fresh game.
pub fn new() -> Box<dyn Game> {
    Box::new(Conduits::default())
}

// The splice's picture: the conduit, open between PIPE_L and PIPE_R.
const PIPE_Y: f32 = 330.0;
const PIPE_H: f32 = 120.0;
const PIPE_L: f32 = 345.0;
const PIPE_R: f32 = 935.0;
const MID: f32 = PIPE_Y + PIPE_H / 2.0;
/// The burnt length.
const BURN_X0: f32 = 470.0;
const BURN_X1: f32 = 810.0;
const BINS: usize = 34;
/// How fast Space or Right strips, px/s.
const STRIP_PX_S: f32 = 260.0;
const LX: f32 = BURN_X0 + 4.0;
const RX: f32 = BURN_X1 - 4.0;
/// Where the spare length waits in its crate.
const LEN_HOME: [f32; 2] = [1130.0, 620.0];

// The reroute's map.
const COLS: usize = 11;
const ROWS: usize = 6;
const CELL: f32 = 80.0;
const GX: f32 = 200.0;
const GY: f32 = 150.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Tip {
    Circle,
    Square,
    Triangle,
    Diamond,
    Bar,
    Ring,
}

const WIRES: [(Color32, Tip); 6] = [
    (c::DANGER, Tip::Circle),
    (c::ACCENT, Tip::Square),
    (c::OK, Tip::Triangle),
    (c::WARN, Tip::Diamond),
    (c::LILAC, Tip::Bar),
    (hex(0xe8eef6), Tip::Ring),
];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Mode {
    Part,
    #[default]
    Splice,
    Reroute,
}

#[derive(Clone, Copy, Debug, Default)]
struct Spark {
    x: f32,
    y: f32,
    t: f32,
}

#[derive(Clone, Copy, Debug, Default)]
struct Deny {
    c: usize,
    r: usize,
    t: f32,
}

type Cell = (usize, usize);

fn key((c, r): Cell) -> usize {
    r * COLS + c
}
fn cx(c: usize) -> f32 {
    GX + c as f32 * CELL + CELL / 2.0
}
fn cy(r: usize) -> f32 {
    GY + r as f32 * CELL + CELL / 2.0
}
fn cell_of(x: f32, y: f32) -> Option<Cell> {
    let (c, r) = (((x - GX) / CELL).floor(), ((y - GY) / CELL).floor());
    (c >= 0.0 && c < COLS as f32 && r >= 0.0 && r < ROWS as f32).then_some((c as usize, r as usize))
}

/// The shortest run of open cells from `src` to `dst` (the map's check, and the hand's way), or None.
fn bfs(grid: &[u8], src: Cell, dst: Cell) -> Option<Vec<Cell>> {
    let mut from: Vec<Option<Cell>> = vec![None; COLS * ROWS];
    let mut seen = [false; COLS * ROWS];
    seen[key(src)] = true;
    let mut q = std::collections::VecDeque::from([src]);
    while let Some((c, r)) = q.pop_front() {
        if (c, r) == dst {
            let mut path = vec![dst];
            let mut at = dst;
            while let Some(p) = from[key(at)] {
                path.push(p);
                at = p;
            }
            path.reverse();
            return Some(path);
        }
        for (dc, dr) in [(1i32, 0i32), (-1, 0), (0, 1), (0, -1)] {
            let (nc, nr) = (c as i32 + dc, r as i32 + dr);
            if nc < 0 || nc >= COLS as i32 || nr < 0 || nr >= ROWS as i32 {
                continue;
            }
            let n = (nc as usize, nr as usize);
            if grid[key(n)] != 0 || seen[key(n)] {
                continue;
            }
            seen[key(n)] = true;
            from[key(n)] = Some((c, r));
            q.push_back(n);
        }
    }
    None
}

/// The points of a cubic Bezier (canvas's `bezierCurveTo`), for the joined wires.
fn bezier(p0: [f32; 2], p1: [f32; 2], p2: [f32; 2], p3: [f32; 2]) -> Vec<[f32; 2]> {
    (0..=24)
        .map(|i| {
            let t = i as f32 / 24.0;
            let u = 1.0 - t;
            let (a, b, c2, d) = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
            [a * p0[0] + b * p1[0] + c2 * p2[0] + d * p3[0], a * p0[1] + b * p1[1] + c2 * p2[1] + d * p3[1]]
        })
        .collect()
}

/// The game's state.
#[derive(Default)]
pub struct Conduits {
    mode: Mode,
    n: usize,
    right_order: Vec<usize>,
    stripped: Vec<bool>,
    head: f32,
    joins: Vec<Option<usize>>,
    held: Option<usize>,
    sel_l: usize,
    sel_r: usize,
    clamp: f32,
    clamp_t: f32,
    spark: Option<Spark>,
    done: bool,
    len: [f32; 2],
    len_held: bool,
    len_set: bool,
    grid: Vec<u8>,
    src: Cell,
    load: Cell,
    burnt: Cell,
    old_run: Vec<Cell>,
    run: Vec<Cell>,
    deny: Option<Deny>,
    time: f32,
    /// The hand: frames into its current gesture.
    hand_f: u32,
    /// The hand: the wire it is carrying, from and to.
    hand_drag: Option<([f32; 2], [f32; 2])>,
    /// The hand: carrying the spare length.
    hand_carry: bool,
    /// The hand: its way round the map.
    hand_path: Vec<Cell>,
    /// The hand: frames it looks before the next join.
    hand_wait: u32,
}

impl Conduits {
    fn row_y(&self, i: usize) -> f32 {
        MID + (i as f32 - (self.n as f32 - 1.0) / 2.0) * 26.0
    }

    fn strip_done(&self) -> bool {
        self.stripped.iter().all(|s| *s)
    }

    fn build_map(&mut self, r: &mut Dice, want: usize) {
        for _ in 0..200 {
            let (r0, r1, bc) = (r.int(ROWS), r.int(ROWS), 4 + r.int(3));
            self.src = (0, r0);
            self.load = (COLS - 1, r1);
            self.burnt = (bc, r0);
            self.old_run = (0..=7).map(|c| (c, r0)).collect();
            let mut rr = r0 as i32;
            while rr != r1 as i32 {
                let s = (r1 as i32 - rr).signum();
                self.old_run.push((7, (rr + s) as usize));
                rr += s;
            }
            self.old_run.extend((8..COLS).map(|c| (c, r1)));
            self.grid = vec![0; COLS * ROWS];
            self.grid[key(self.burnt)] = 2;
            // Bulkheads: never on the old run (it is still there, but for its box), always beside the burnt box.
            let on_old = |old: &[Cell], c: usize, rr: usize| old.contains(&(c, rr));
            for dr in [-1i32, 1] {
                let rr = r0 as i32 + dr;
                if rr >= 0 && rr < ROWS as i32 && !on_old(&self.old_run, bc, rr as usize) {
                    self.grid[key((bc, rr as usize))] = 1;
                }
            }
            let (mut placed, mut guard) = (0, 0);
            while placed < want && guard < 500 {
                guard += 1;
                let (c, rr) = (1 + r.int(COLS - 2), r.int(ROWS));
                if self.grid[key((c, rr))] == 0 && !on_old(&self.old_run, c, rr) {
                    self.grid[key((c, rr))] = 1;
                    placed += 1;
                }
            }
            // A run must exist from the switchboard to the load.
            if bfs(&self.grid, self.src, self.load).is_some() {
                return;
            }
        }
        self.grid = vec![0; COLS * ROWS];
        self.grid[key(self.burnt)] = 2;
    }

    fn try_move(&mut self, cx: &mut Ctx, c: usize, r: usize) {
        let Some(&last) = self.run.last() else { return };
        if self.run.len() >= 2 && self.run[self.run.len() - 2] == (c, r) {
            self.run.pop();
            return;
        }
        if let Some(at) = self.run.iter().position(|&p| p == (c, r)) {
            self.run.truncate(at + 1);
            return;
        }
        if c.abs_diff(last.0) + r.abs_diff(last.1) != 1 {
            return;
        }
        if self.grid[key((c, r))] != 0 {
            self.deny = Some(Deny { c, r, t: 0.5 });
            return;
        }
        self.run.push((c, r));
        if (c, r) == self.load {
            self.done = true;
            cx.step_done();
        }
    }

    fn splice(&mut self, cx: &mut Ctx, dt: f32, input: &Input) {
        // Strip first.
        if !self.strip_done() {
            let mut mark = |x: f32| {
                let b = ((x - BURN_X0) / (BURN_X1 - BURN_X0) * BINS as f32).floor();
                if b >= 0.0 && b < BINS as f32 {
                    self.stripped[b as usize] = true;
                }
            };
            if input.down {
                for p in input.path.iter().chain(std::iter::once(&[input.x, input.y])) {
                    if (p[1] - MID).abs() < 90.0 {
                        mark(p[0]);
                    }
                }
            }
            if input.action || input.stick[0] > 0.0 {
                self.head = (self.head + STRIP_PX_S * dt).min(BURN_X1);
                let mut x = BURN_X0;
                while x <= self.head {
                    mark(x);
                    x += 5.0;
                }
            }
            return;
        }
        // Then join, before the clamp closes.
        self.clamp_t += dt;
        if self.clamp_t >= self.clamp {
            self.joins.iter_mut().for_each(|j| *j = None);
            self.held = None;
            self.clamp_t = 0.0;
            cx.say("Clamp shut on loose ends");
            return;
        }
        let n = self.n;
        let up = input.hit(Key::ArrowUp) || input.hit(Key::W);
        let dn = input.hit(Key::ArrowDown) || input.hit(Key::S);
        let Some(held) = self.held else {
            if up {
                self.sel_l = (self.sel_l + n - 1) % n;
            }
            if dn {
                self.sel_l = (self.sel_l + 1) % n;
            }
            // The ends are 26 px apart: the nearest free one within a fingertip's reach.
            if input.pressed {
                let ends: Vec<Option<[f32; 2]>> =
                    (0..n).map(|k| self.joins[k].is_none().then_some([LX, self.row_y(k)])).collect();
                if let Some(i) = nearest(&ends, input.x, input.y, TOUCH_R + 10.0) {
                    self.held = Some(i);
                    self.sel_l = i;
                }
            }
            if input.action_pressed && self.joins[self.sel_l].is_none() {
                self.held = Some(self.sel_l);
                self.sel_r = (0..n).find(|i| !self.joins.contains(&Some(*i))).unwrap_or(0);
            }
            return;
        };
        if up {
            self.sel_r = (self.sel_r + n - 1) % n;
        }
        if dn {
            self.sel_r = (self.sel_r + 1) % n;
        }
        // Dragged onto a right end, or tapped there after tapping a left end; let go elsewhere, the wire drops back.
        let mut drop = None;
        if input.pressed || input.released {
            let ends: Vec<Option<[f32; 2]>> = (0..n).map(|k| Some([RX, self.row_y(k)])).collect();
            drop = nearest(&ends, input.x, input.y, TOUCH_R + 10.0);
            if drop.is_none() && input.released && input.dist(LX, self.row_y(held)) > 30.0 {
                self.held = None;
                return;
            }
        }
        if input.action_pressed {
            drop = Some(self.sel_r);
        }
        let Some(drop) = drop else { return };
        if self.joins.contains(&Some(drop)) {
            self.held = None;
            return;
        }
        if self.right_order[drop] == held {
            self.joins[held] = Some(drop);
            self.held = None;
            self.sel_l = self.joins.iter().position(|j| j.is_none()).unwrap_or(0);
            if self.joins.iter().all(|j| j.is_some()) {
                self.done = true;
                cx.step_done();
            }
        } else {
            self.spark = Some(Spark { x: RX, y: self.row_y(drop), t: 0.6 });
            self.held = None;
            cx.fumble("Crossed pair: spark, 5 HP, breaker tripped");
        }
    }
}

impl Game for Conduits {
    fn id(&self) -> &'static str {
        "conduits"
    }

    fn knobs(&self) -> &'static [&'static str] {
        &["pairs_count", "clamp_s", "bulkheads_count"]
    }

    fn phases(&self, rounds: u32) -> &'static [&'static str] {
        // A round splices; a disabled job has a burnt junction box too, rerouted every round (design 1a).
        if rounds >= 4 {
            &["splice", "reroute"]
        } else {
            &["splice"]
        }
    }

    fn step(&mut self, cx: &mut Ctx) {
        let mut r = cx.dice();
        self.time = 0.0;
        self.spark = None;
        self.done = false;
        self.deny = None;
        self.len = LEN_HOME;
        self.len_held = false;
        self.len_set = false;
        self.hand_f = 0;
        self.hand_drag = None;
        self.hand_carry = false;
        self.hand_path.clear();
        self.hand_wait = 0;
        self.mode = if cx.part {
            Mode::Part
        } else if cx.phase_name == "reroute" {
            Mode::Reroute
        } else {
            Mode::Splice
        };
        // The splice: the right ends in any order but the left's.
        self.n = (cx.knob("pairs_count").round() as usize).clamp(2, WIRES.len());
        self.right_order = (0..self.n).collect();
        loop {
            r.shuffle(&mut self.right_order);
            if self.right_order.iter().enumerate().any(|(i, w)| *w != i) {
                break;
            }
        }
        self.stripped = vec![false; BINS];
        self.head = BURN_X0;
        self.joins = vec![None; self.n];
        self.held = None;
        self.sel_l = 0;
        self.sel_r = 0;
        self.clamp = cx.knob("clamp_s");
        self.clamp_t = 0.0;
        // The reroute.
        if self.mode == Mode::Reroute {
            let want = cx.knob("bulkheads_count").round().max(0.0) as usize;
            self.build_map(&mut r, want);
            self.run = vec![self.src];
            self.hand_path = bfs(&self.grid, self.src, self.load).unwrap_or_default();
        }
    }

    fn update(&mut self, cx: &mut Ctx, dt: f32, input: &Input) {
        self.time += dt;
        if let Some(s) = &mut self.spark {
            s.t -= dt;
            if s.t <= 0.0 {
                self.spark = None;
            }
        }
        if let Some(d) = &mut self.deny {
            d.t -= dt;
            if d.t <= 0.0 {
                self.deny = None;
            }
        }
        match self.mode {
            Mode::Part => {
                // The part step: carry the new length into the gap.
                if self.len_set {
                    return;
                }
                if input.pressed && (input.x - self.len[0]).abs() < 90.0 && (input.y - self.len[1]).abs() < 40.0 {
                    self.len_held = true;
                }
                if self.len_held && input.down {
                    self.len = [input.x, input.y];
                }
                if self.len_held && input.released {
                    self.len_held = false;
                    if self.len[0] > PIPE_L && self.len[0] < PIPE_R && (self.len[1] - MID).abs() < 70.0 {
                        self.len_set = true;
                        cx.step_done();
                    }
                }
            }
            _ if self.done => {}
            Mode::Reroute => {
                if input.down {
                    if let Some((c, r)) = cell_of(input.x, input.y) {
                        self.try_move(cx, c, r);
                    }
                }
                let Some(&last) = self.run.last() else { return };
                let mv = if input.hit(Key::ArrowRight) || input.hit(Key::D) {
                    Some((1, 0))
                } else if input.hit(Key::ArrowLeft) || input.hit(Key::A) {
                    Some((-1, 0))
                } else if input.hit(Key::ArrowDown) || input.hit(Key::S) {
                    Some((0, 1))
                } else if input.hit(Key::ArrowUp) || input.hit(Key::W) {
                    Some((0, -1))
                } else {
                    None
                };
                if let Some((dc, dr)) = mv {
                    let (c, r) = (last.0 as i32 + dc, last.1 as i32 + dr);
                    if c >= 0 && c < COLS as i32 && r >= 0 && r < ROWS as i32 {
                        self.try_move(cx, c as usize, r as usize);
                    }
                }
            }
            Mode::Splice => self.splice(cx, dt, input),
        }
    }

    fn draw(&self, g: &Pen, _cx: &Ctx, t: f32, input: &Input) {
        g.rect(0.0, BAR_H, W, H, hex(0x070b12));
        if self.mode == Mode::Reroute {
            self.draw_map(g, t);
        } else {
            self.draw_splice(g, t, input);
        }
    }

    fn hand(&mut self, _cx: &Ctx, _t: f32) -> Input {
        match self.mode {
            Mode::Part => {
                if self.len_set {
                    return Input::default();
                }
                // Pick the length up, carry it into the gap, let go there.
                let goal = [640.0, MID];
                if !self.hand_carry {
                    self.hand_carry = true;
                    return Input::hold(self.len[0], self.len[1], true);
                }
                let (dx, dy) = (goal[0] - self.len[0], goal[1] - self.len[1]);
                let d = dx.hypot(dy);
                if d > 2.0 {
                    let k = (12.0 / d).min(1.0);
                    return Input::hold(self.len[0] + dx * k, self.len[1] + dy * k, false);
                }
                self.hand_carry = false;
                Input::release(goal[0], goal[1])
            }
            _ if self.done => Input::default(),
            Mode::Reroute => {
                // Drag through the way's cells, a few frames on each.
                let Some(&last) = self.hand_path.last() else { return Input::default() };
                let k = (self.hand_f / 10) as usize;
                self.hand_f += 1;
                let (c, r) = self.hand_path.get(k).copied().unwrap_or(last);
                Input::hold(cx(c), cy(r), self.hand_f == 1)
            }
            Mode::Splice => {
                if !self.strip_done() {
                    // Drag along the burn at 180 px/s.
                    let x = BURN_X0 - 10.0 + 3.0 * self.hand_f as f32;
                    self.hand_f += 1;
                    if x > BURN_X1 + 10.0 {
                        self.hand_f = 0;
                        self.hand_wait = 24;
                        return Input::release(x, MID);
                    }
                    return Input::hold(x, MID, self.hand_f == 1);
                }
                // Each left end dragged to the right end of its colour, at 480 px/s, after a look to find it.
                if self.hand_drag.is_none() && self.hand_wait > 0 {
                    self.hand_wait -= 1;
                    return Input::default();
                }
                let Some((from, to)) = self.hand_drag else {
                    let Some(i) = self.joins.iter().position(|j| j.is_none()) else { return Input::default() };
                    let k = self.right_order.iter().position(|&w| w == i).unwrap_or(0);
                    let (from, to) = ([LX, self.row_y(i)], [RX, self.row_y(k)]);
                    self.hand_drag = Some((from, to));
                    self.hand_f = 0;
                    return Input::hold(from[0], from[1], true);
                };
                self.hand_f += 1;
                let d = (to[0] - from[0]).hypot(to[1] - from[1]);
                let u = self.hand_f as f32 * 8.0 / d.max(1.0);
                if u >= 1.0 {
                    self.hand_drag = None;
                    self.hand_wait = 24;
                    return Input::release(to[0], to[1]);
                }
                Input::hold(from[0] + (to[0] - from[0]) * u, from[1] + (to[1] - from[1]) * u, false)
            }
        }
    }

    fn guide_now(&self) -> Option<usize> {
        match self.mode {
            Mode::Part => None,
            Mode::Reroute => Some(2),
            Mode::Splice => Some(if self.strip_done() { 1 } else { 0 }),
        }
    }
}

impl Conduits {
    fn draw_splice(&self, g: &Pen, t: f32, input: &Input) {
        // The conduit's two good ends and the open span between them, with the tray under it.
        g.rect(PIPE_L, PIPE_Y + 10.0, PIPE_R - PIPE_L, PIPE_H - 20.0, hex(0x0e151f));
        let mut x = PIPE_L + 10.0;
        while x < PIPE_R {
            g.line(x, PIPE_Y + 10.0, x, PIPE_Y + PIPE_H - 10.0, 2.0, hex(0x1f2a37));
            x += 24.0;
        }
        pipe(g, 20.0, PIPE_L);
        pipe(g, PIPE_R, 1260.0);
        if self.mode == Mode::Part {
            if self.len_set {
                pipe(g, PIPE_L, PIPE_R);
            } else {
                let mut gap = round_rect_points(PIPE_L + 8.0, PIPE_Y, PIPE_R - PIPE_L - 16.0, PIPE_H, 14.0);
                gap.push(gap[0]);
                g.dashed(&gap, 2.0, c::AMBER, 10.0, 8.0);
                g.panel(1020.0, 570.0, 220.0, 110.0, 14.0, hex(0x141b27), c::LINE);
                let s = g.translate(self.len[0], self.len[1]).scale(0.32, 0.32).translate(-640.0, -MID);
                pipe(&s, 640.0 - 260.0, 640.0 + 260.0);
            }
            flange(g, PIPE_L);
            flange(g, PIPE_R);
            return;
        }
        let n = self.n;
        let strip_done = self.strip_done();
        // The wires: whole from the flanges to the burnt length; charred inside it until stripped.
        for i in 0..n {
            let y = self.row_y(i);
            g.line(PIPE_L, y, BURN_X0, y, 9.0, WIRES[i].0);
            g.line(BURN_X1, y, PIPE_R, y, 9.0, WIRES[self.right_order[i]].0);
        }
        if !strip_done {
            let bw = (BURN_X1 - BURN_X0) / BINS as f32;
            let (y0, yn) = (self.row_y(0), self.row_y(n - 1));
            for b in 0..BINS {
                if self.stripped[b] {
                    continue;
                }
                let x = BURN_X0 + b as f32 * bw;
                g.rect(x, y0 - 18.0, bw + 0.5, yn - y0 + 36.0, rgba(20, 16, 14, 0.9));
                for i in 0..n {
                    let col = if (b + i) % 3 != 0 { hex(0x2e2620) } else { hex(0x3a2f27) };
                    g.rect(x, self.row_y(i) - 5.0 + ((b * 5 + i) % 3) as f32 - 1.0, bw + 0.5, 10.0, col);
                }
                if (b * 7) % 5 == 0 {
                    let bf = b as f32;
                    g.disc(
                        x + bw / 2.0,
                        self.row_y((b * 3) % n) + 4.0 * (t * 3.0 + bf).sin(),
                        3.0,
                        rgba(255, 120, 60, 0.5 + 0.4 * (t * 5.0 + bf).sin()),
                    );
                }
            }
            if input.down && (input.y - MID).abs() < 90.0 && input.x > BURN_X0 - 40.0 && input.x < BURN_X1 + 40.0 {
                self.stripper(g, input.x);
            } else {
                self.stripper(g, self.head);
            }
        }
        // The clamp's jaws above and below the span, closing as the time runs out.
        let k = if strip_done {
            if self.done {
                1.0
            } else {
                self.clamp_t / self.clamp
            }
        } else {
            0.0
        };
        let top = 120.0 + k * (self.row_y(0) - 30.0 - 120.0 - 24.0);
        let bot = 640.0 - k * (640.0 - (self.row_y(n - 1) + 30.0));
        for (y, dir) in [(top, 1.0f32), (bot, -1.0)] {
            let fill = if self.done {
                hex(0x2f6e4c)
            } else if k > 0.75 {
                hex(0x7a2f38)
            } else {
                hex(0x566273)
            };
            g.round(
                BURN_X0 - 40.0,
                y - if dir > 0.0 { 24.0 } else { 0.0 },
                BURN_X1 - BURN_X0 + 80.0,
                24.0,
                6.0,
                Some(fill),
                None,
            );
            let mut x = BURN_X0 - 30.0;
            while x < BURN_X1 + 30.0 {
                g.poly(&[[x, y], [x + 11.0, y + dir * 12.0], [x + 22.0, y]], hex(0x3a4656));
                x += 22.0;
            }
            let mx = (BURN_X0 + BURN_X1) / 2.0 - 10.0;
            if dir > 0.0 {
                g.rect(mx, BAR_H, 20.0, y - 24.0 - BAR_H, hex(0x2b3646));
            } else {
                g.rect(mx, y, 20.0, H - y, hex(0x2b3646));
            }
        }
        if !strip_done {
            return;
        }
        // The ends: a tip shape and colour each; joins as curves with a sleeve.
        for (i, join) in self.joins.iter().enumerate() {
            if let Some(j) = *join {
                let (y0, y1) = (self.row_y(i), self.row_y(j));
                g.path(&bezier([LX, y0], [LX + 120.0, y0], [RX - 120.0, y1], [RX, y1]), false, 8.0, WIRES[i].0);
                let (mx, my) = ((LX + RX) / 2.0, (y0 + y1) / 2.0);
                g.round(mx - 22.0, my - 9.0, 44.0, 18.0, 6.0, Some(hex(0xb8c2d0)), None);
            }
        }
        for i in 0..n {
            tip(g, WIRES[i], LX, self.row_y(i), 11.0);
            tip(g, WIRES[self.right_order[i]], RX, self.row_y(i), 11.0);
        }
        // The keyboard's choice, and the wire in the hand.
        if self.held.is_none() && self.joins[self.sel_l].is_none() {
            g.ring(LX, self.row_y(self.sel_l), 18.0, rgba(242, 160, 70, 0.6), 2.0);
        }
        if let Some(h) = self.held {
            let y0 = self.row_y(h);
            let (ex, ey) = if input.down { (input.x, input.y) } else { (RX, self.row_y(self.sel_r)) };
            g.alpha(0.8).path(&bezier([LX, y0], [LX + 100.0, y0], [ex - 100.0, ey], [ex, ey]), false, 8.0, WIRES[h].0);
            tip(g, WIRES[h], ex, ey, 13.0);
            if !input.down {
                g.ring(RX, self.row_y(self.sel_r), 20.0, c::AMBER, 3.0);
            }
        }
        if let Some(s) = self.spark {
            let a = s.t / 0.6;
            for k in 0..10 {
                let b = k as f32 * 0.63 + t * 20.0;
                g.line(s.x, s.y, s.x + b.cos() * 50.0, s.y + b.sin() * 50.0, 3.0, rgba(255, 240, 180, a));
            }
            g.disc(s.x, s.y, 16.0, rgba(255, 255, 255, a));
        }
        flange(g, PIPE_L);
        flange(g, PIPE_R);
    }

    /// The wire stripper: two jaws closing on the bundle.
    fn stripper(&self, g: &Pen, x: f32) {
        let (y0, y1) = (self.row_y(0) - 26.0, self.row_y(self.n - 1) + 26.0);
        g.round(x - 10.0, y0 - 60.0, 20.0, 60.0, 6.0, Some(c::DANGER), None);
        g.round(x - 10.0, y1, 20.0, 60.0, 6.0, Some(c::DANGER), None);
        g.rect(x - 6.0, y0 - 4.0, 12.0, 10.0, hex(0xb8c2d0));
        g.rect(x - 6.0, y1 - 6.0, 12.0, 10.0, hex(0xb8c2d0));
    }

    fn draw_map(&self, g: &Pen, t: f32) {
        // The conduit map: the deck's cells, bulkheads hatched, the old run dashed, the burnt box, the new run.
        g.panel(
            GX - 30.0,
            GY - 30.0,
            COLS as f32 * CELL + 60.0,
            ROWS as f32 * CELL + 60.0,
            18.0,
            hex(0x0b1119),
            c::LINE,
        );
        for r in 0..ROWS {
            for cc in 0..COLS {
                let (x, y) = (GX + cc as f32 * CELL, GY + r as f32 * CELL);
                g.rect(x + 2.0, y + 2.0, CELL - 4.0, CELL - 4.0, hex(0x101824));
                if self.grid.get(key((cc, r))) == Some(&1) {
                    let s = g.clip(x + 2.0, y + 2.0, CELL - 4.0, CELL - 4.0);
                    s.rect(x, y, CELL, CELL, hex(0x1f2733));
                    let mut k = -CELL;
                    while k < CELL {
                        s.line(x + k, y + CELL, x + k + CELL, y, 3.0, hex(0x384354));
                        k += 14.0;
                    }
                }
                if let Some(d) = self.deny.filter(|d| d.c == cc && d.r == r) {
                    g.rect(x + 2.0, y + 2.0, CELL - 4.0, CELL - 4.0, rgba(255, 71, 87, d.t));
                }
            }
        }
        let old: Vec<[f32; 2]> = self.old_run.iter().map(|&(cc, r)| [cx(cc), cy(r)]).collect();
        g.dashed(&old, 6.0, rgba(111, 127, 148, 0.45), 8.0, 8.0);
        // The burnt junction box: red, crossed, sooted.
        let (bx, by) = (cx(self.burnt.0), cy(self.burnt.1));
        g.disc(bx, by, 36.0, rgba(30, 24, 22, 0.9));
        g.round(bx - 24.0, by - 24.0, 48.0, 48.0, 6.0, Some(hex(0x3a1a1e)), Some((3.0, c::DANGER)));
        g.cross(bx, by, 14.0, 5.0, c::DANGER);
        // The new run.
        let run: Vec<[f32; 2]> = self.run.iter().map(|&(cc, r)| [cx(cc), cy(r)]).collect();
        if run.len() >= 2 {
            g.path_round(&run, 12.0, if self.done { c::OK } else { c::AMBER });
        }
        // The switchboard and the load.
        let (sx, sy) = (cx(self.src.0), cy(self.src.1));
        g.round(sx - 28.0, sy - 28.0, 56.0, 56.0, 8.0, Some(hex(0x2b3646)), Some((3.0, c::AMBER)));
        g.poly(
            &[
                [sx + 4.0, sy - 18.0],
                [sx - 10.0, sy + 3.0],
                [sx, sy + 3.0],
                [sx - 4.0, sy + 18.0],
                [sx + 10.0, sy - 3.0],
                [sx, sy - 3.0],
            ],
            c::AMBER,
        );
        let (lx, ly) = (cx(self.load.0), cy(self.load.1));
        g.disc(lx, ly, 28.0, hex(0x2b3646));
        g.ring(lx, ly, 28.0, if self.done { c::OK } else { hex(0x566273) }, 3.0);
        g.disc(lx, ly, 13.0, if self.done { rgba(255, 214, 120, 0.8 + 0.2 * (t * 6.0).sin()) } else { hex(0x3a4656) });
        let ty = GY + ROWS as f32 * CELL + 14.0;
        g.text("SWBD", sx, ty, 16.0, c::DIM, Align::Center);
        g.text("LOAD", lx, ty, 16.0, c::DIM, Align::Center);
        if let (Some(&(hc, hr)), false) = (self.run.last(), self.done) {
            if self.run.len() > 1 {
                g.disc(cx(hc), cy(hr), 14.0, c::AMBER);
            }
            g.ring(cx(hc), cy(hr), 36.0, rgba(242, 160, 70, 0.5), 3.0);
        }
    }
}

/// A wire's cut end: its colour and its tip shape.
fn tip(g: &Pen, (col, shape): (Color32, Tip), x: f32, y: f32, s: f32) {
    match shape {
        Tip::Circle => g.disc(x, y, s, col),
        Tip::Square => g.rect(x - s, y - s, s * 2.0, s * 2.0, col),
        Tip::Triangle => g.poly(&[[x, y - s - 2.0], [x + s + 2.0, y + s], [x - s - 2.0, y + s]], col),
        Tip::Diamond => g.poly(&[[x, y - s - 3.0], [x + s + 3.0, y], [x, y + s + 3.0], [x - s - 3.0, y]], col),
        Tip::Bar => g.rect(x - 4.0, y - s - 2.0, 8.0, s * 2.0 + 4.0, col),
        Tip::Ring => g.ring(x, y, s - 2.0, col, 5.0),
    }
}

/// The conduit's casing from x0 to x1: a rounded tube shaded top to bottom (the mockup's three-stop gradient, drawn
/// as horizontal slices so the rounded ends keep their shape), with its dark bands.
fn pipe(g: &Pen, x0: f32, x1: f32) {
    let (r, h) = (14.0f32, PIPE_H);
    let stops = [(0.0, hex(0x3a4656)), (0.45, hex(0x566273)), (1.0, hex(0x1b2433))];
    let col = |f: f32| {
        if f <= stops[1].0 {
            mix(stops[0].1, stops[1].1, f / stops[1].0)
        } else {
            mix(stops[1].1, stops[2].1, (f - stops[1].0) / (1.0 - stops[1].0))
        }
    };
    let inset = |yy: f32| {
        if yy < r {
            r - (r * r - (r - yy).powi(2)).max(0.0).sqrt()
        } else if yy > h - r {
            r - (r * r - (yy - (h - r)).powi(2)).max(0.0).sqrt()
        } else {
            0.0
        }
    };
    let mut ys: Vec<f32> =
        vec![0.0, 1.0, 2.5, 4.5, 7.0, 10.0, r, 0.45 * h, h - r, h - 10.0, h - 7.0, h - 4.5, h - 2.5, h - 1.0, h];
    ys.dedup();
    for w in ys.windows(2) {
        let (ya, yb) = (w[0], w[1]);
        let (ia, ib) = (inset(ya), inset(yb));
        let (ca, cb) = (col(ya / h), col(yb / h));
        g.quad(
            [[x0 + ia, PIPE_Y + ya], [x1 - ia, PIPE_Y + ya], [x1 - ib, PIPE_Y + yb], [x0 + ib, PIPE_Y + yb]],
            [ca, ca, cb, cb],
        );
    }
    let mut x = x0 + 30.0;
    while x < x1 - 10.0 {
        g.rect(x, PIPE_Y + 4.0, 6.0, PIPE_H - 8.0, rgba(0, 0, 0, 0.25));
        x += 70.0;
    }
}

/// A flange at x, bolted top and bottom.
fn flange(g: &Pen, x: f32) {
    g.round(x - 14.0, PIPE_Y - 22.0, 28.0, PIPE_H + 44.0, 6.0, Some(hex(0x7d8796)), None);
    for y in [PIPE_Y - 10.0, PIPE_Y + PIPE_H + 10.0] {
        g.disc(x, y, 5.0, hex(0x3a4656));
    }
}

#[cfg(test)]
mod tests {
    use crate::games::tests::{fumble_check, plays_to_end};
    use crate::kit::Input;
    use egui::Key;

    #[test]
    fn a_steady_hand_repairs_it_damaged_disabled_and_destroyed() {
        plays_to_end("conduits");
    }

    #[test]
    fn a_crossed_pair_sparks() {
        // Hold Space two seconds to strip, then Space every ten frames: pick the chosen left end, drop it on the first
        // free right end. That pairs left k with right k, which only works if the right ends are in the left's order,
        // and the round never deals them so: a crossed pair comes before the last join.
        let mut f = 0;
        fumble_check("conduits", move |_r| {
            f += 1;
            if f <= 120 {
                Input::default().key(Key::Space, f == 1)
            } else if (f - 120) % 10 == 0 {
                Input::default().key(Key::Space, true)
            } else {
                Input::default()
            }
        });
    }
}
