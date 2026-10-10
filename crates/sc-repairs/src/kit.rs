//! The repair games' kit (openspec/changes/repair-minigames design 1, 6 and 8), the engine's `kit.js`: what every
//! game shares, so no game re-implements it (CLAUDE.md 6.1).
//!
//! - **The job** is `sc-core::repair::Job`: the kit runs a game's phases in order, lands the round when the last is
//!   played, fits the part first on a disabled job, takes a fumble's cost and restarts the round on the third; a
//!   rating has no game and fills at its rate.
//! - **Input** in canvas pixels: the pointer (mouse or touch, every sample of a drag), keys, a stick from the arrows
//!   or W A S D, one action (Space or Enter), the wheel.
//! - **The frame**: 1280 x 720, the job's bar along the top 72 px, the combat shake, a fumble's flash.
//! - **The cover**: a game whose file names one has its machine behind a plate, unscrewed before the work and screwed
//!   back after it.
//! - **The guide card** (design 6g), from the game's file, with the kit's icons.
//! - **A seeded stream** per game, round and phase (`sc_core::repair::round_rng`).
//! - **The hand**: every game scripts a steady hand that plays it, so a test plays it to the end with no person.

use std::collections::HashSet;

use egui::{Color32, Key, Rect};
use sc_core::repair::data::{Cover as CoverData, GameFile, RepairData};
use sc_core::repair::{round_rng, Fumbled, Job, State, Who};
use sc_core::rng::Rng;

use crate::pen::{alpha, c, hex, rgba, Align, Pen};

/// The canvas's width, pixels.
pub const W: f32 = 1280.0;
/// Its height.
pub const H: f32 = 720.0;
/// The job's bar along the top; a game draws below it.
pub const BAR_H: f32 = 72.0;
/// The smallest reach a touch target gets, canvas pixels (`kit.js`'s `TOUCH_R`).
pub const TOUCH_R: f32 = 30.0;

/// One frame's input, canvas pixels (`kit.js`'s `input`).
#[derive(Clone, Debug, PartialEq)]
pub struct Input {
    /// The pointer, or (-1, -1) with none.
    pub x: f32,
    /// As `x`.
    pub y: f32,
    /// Every sample of the pointer while pressed since the last frame, for games that follow a stroke's shape.
    pub path: Vec<[f32; 2]>,
    /// Held.
    pub down: bool,
    /// Pressed this frame.
    pub pressed: bool,
    /// Let go this frame.
    pub released: bool,
    /// Keys held.
    pub keys: HashSet<Key>,
    /// Keys pressed this frame.
    pub hit: HashSet<Key>,
    /// Wheel notches this frame, down positive.
    pub wheel: f32,
    /// The stick, -1 to 1 each way (+y down), from the arrows or W A S D.
    pub stick: [f32; 2],
    /// The action held: Space or Enter.
    pub action: bool,
    /// The action pressed this frame.
    pub action_pressed: bool,
}

impl Default for Input {
    fn default() -> Self {
        Self {
            x: -1.0,
            y: -1.0,
            path: Vec::new(),
            down: false,
            pressed: false,
            released: false,
            keys: HashSet::new(),
            hit: HashSet::new(),
            wheel: 0.0,
            stick: [0.0, 0.0],
            action: false,
            action_pressed: false,
        }
    }
}

impl Input {
    /// A pointer at (x, y), not pressed.
    pub fn at(x: f32, y: f32) -> Self {
        Self { x, y, ..Self::default() }
    }

    /// Held at (x, y); `pressed` on the frame the press starts.
    pub fn hold(x: f32, y: f32, pressed: bool) -> Self {
        Self { x, y, down: true, pressed, path: vec![[x, y]], ..Self::default() }
    }

    /// Let go at (x, y).
    pub fn release(x: f32, y: f32) -> Self {
        Self { x, y, released: true, ..Self::default() }
    }

    /// With `k` held, and pressed this frame if `pressed`.
    pub fn key(mut self, k: Key, pressed: bool) -> Self {
        self.keys.insert(k);
        if pressed {
            self.hit.insert(k);
        }
        self.derive();
        self
    }

    /// Fill the stick and the action from the keys (the kit does this every frame).
    pub fn derive(&mut self) {
        let k = |a: Key, b: Key| self.keys.contains(&a) || self.keys.contains(&b);
        self.stick = [
            f32::from(u8::from(k(Key::ArrowRight, Key::D))) - f32::from(u8::from(k(Key::ArrowLeft, Key::A))),
            f32::from(u8::from(k(Key::ArrowDown, Key::S))) - f32::from(u8::from(k(Key::ArrowUp, Key::W))),
        ];
        self.action = self.keys.contains(&Key::Space) || self.keys.contains(&Key::Enter);
        self.action_pressed = self.action_pressed || self.hit.contains(&Key::Space) || self.hit.contains(&Key::Enter);
    }

    /// Whether the pointer is in the rectangle.
    pub fn over(&self, x: f32, y: f32, w: f32, h: f32) -> bool {
        self.x >= x && self.x <= x + w && self.y >= y && self.y <= y + h
    }

    /// Whether it was pressed in the rectangle this frame (a button).
    pub fn pressed_in(&self, x: f32, y: f32, w: f32, h: f32) -> bool {
        self.pressed && self.over(x, y, w, h)
    }

    /// Whether `k` was pressed this frame.
    pub fn hit(&self, k: Key) -> bool {
        self.hit.contains(&k)
    }

    /// Whether `k` is held.
    pub fn held(&self, k: Key) -> bool {
        self.keys.contains(&k)
    }

    /// Distance from the pointer to (x, y).
    pub fn dist(&self, x: f32, y: f32) -> f32 {
        (self.x - x).hypot(self.y - y)
    }
}

/// The nearest of `pts` to (x, y) within `r`, or None; a `None` entry (done or hidden) is skipped.
pub fn nearest(pts: &[Option<[f32; 2]>], x: f32, y: f32, r: f32) -> Option<usize> {
    let mut best = None;
    let mut bd = r;
    for (i, p) in pts.iter().enumerate() {
        if let Some(p) = p {
            let d = (x - p[0]).hypot(y - p[1]);
            if d < bd {
                bd = d;
                best = Some(i);
            }
        }
    }
    best
}

/// A polyline's length.
pub fn poly_len(pts: &[[f32; 2]]) -> f32 {
    pts.windows(2).map(|w| (w[1][0] - w[0][0]).hypot(w[1][1] - w[0][1])).sum()
}

/// The point `u` (0-1) of the way along a polyline by length, and the direction it runs there: [x, y, angle].
pub fn poly_at(pts: &[[f32; 2]], u: f32) -> [f32; 3] {
    let mut d = u.clamp(0.0, 1.0) * poly_len(pts);
    for i in 1..pts.len() {
        let ([x0, y0], [x1, y1]) = (pts[i - 1], pts[i]);
        let l = (x1 - x0).hypot(y1 - y0);
        if d <= l || i == pts.len() - 1 {
            let k = if l > 0.0 { (d / l).min(1.0) } else { 0.0 };
            return [x0 + (x1 - x0) * k, y0 + (y1 - y0) * k, (y1 - y0).atan2(x1 - x0)];
        }
        d -= l;
    }
    pts.first().map_or([0.0; 3], |p| [p[0], p[1], 0.0])
}

/// The first `u` (0-1) of a polyline by length.
pub fn poly_cut(pts: &[[f32; 2]], u: f32) -> Vec<[f32; 2]> {
    let mut d = u.clamp(0.0, 1.0) * poly_len(pts);
    let mut out = pts.first().map(|p| vec![*p]).unwrap_or_default();
    for i in 1..pts.len() {
        let ([x0, y0], [x1, y1]) = (pts[i - 1], pts[i]);
        let l = (x1 - x0).hypot(y1 - y0);
        if d >= l {
            out.push(pts[i]);
            d -= l;
            continue;
        }
        let k = if l > 0.0 { d / l } else { 0.0 };
        out.push([x0 + (x1 - x0) * k, y0 + (y1 - y0) * k]);
        break;
    }
    out
}

/// The star (cross) order to work n fasteners round a rim, from the first: across, then round.
pub fn star_order(n: usize) -> Vec<usize> {
    match n {
        4 => vec![0, 2, 1, 3],
        6 => vec![0, 3, 1, 4, 2, 5],
        8 => vec![0, 4, 2, 6, 1, 5, 3, 7],
        _ => (0..n).collect(),
    }
}

/// A fluid's reach through connected pieces (`kit.js`'s `flood`): a breadth-first fill from `starts`; `next(key)`
/// lists the pieces a piece passes fluid on to. Returns each reached piece's distance, in the order reached.
pub fn flood<K: Copy + PartialEq>(starts: &[K], mut next: impl FnMut(K) -> Vec<K>) -> Vec<(K, u32)> {
    let mut out: Vec<(K, u32)> = Vec::new();
    for s in starts {
        if !out.iter().any(|(k, _)| k == s) {
            out.push((*s, 0));
        }
    }
    let mut qi = 0;
    while qi < out.len() {
        let (k, d) = out[qi];
        for n in next(k) {
            if !out.iter().any(|(o, _)| *o == n) {
                out.push((n, d + 1));
            }
        }
        qi += 1;
    }
    out
}

/// A round's seeded numbers (`kit.js`'s `api.rand()`).
pub struct Dice(Rng);

impl Dice {
    /// In [0, 1).
    pub fn f(&mut self) -> f32 {
        self.0.next_f64() as f32
    }
    /// In [a, b).
    pub fn range(&mut self, a: f32, b: f32) -> f32 {
        a + (b - a) * self.f()
    }
    /// In 0..n.
    pub fn int(&mut self, n: usize) -> usize {
        ((self.f() * n as f32) as usize).min(n.saturating_sub(1))
    }
    /// `v` shuffled (Fisher-Yates, as the mockups do it).
    pub fn shuffle<T>(&mut self, v: &mut [T]) {
        for i in (1..v.len()).rev() {
            let j = self.int(i + 1);
            v.swap(i, j);
        }
    }
}

/// What a game tells the kit.
#[derive(Clone, Debug, PartialEq)]
pub enum Said {
    /// The part move or a phase is played.
    StepDone,
    /// A mistake, and what happened.
    Fumble(String),
    /// A short note on the bar.
    Say(String),
}

/// What a game knows of its round, and how it speaks to the kit (`kit.js`'s `api`).
pub struct Ctx<'a> {
    /// The game's file.
    pub data: &'a GameFile,
    /// The round being played, 0 up.
    pub round: u32,
    /// Its level, 1 to 6.
    pub level: u32,
    /// The phase within the round, 0 up.
    pub phase: usize,
    /// The phase's name, or "" for a game of one.
    pub phase_name: &'static str,
    /// This step fits the spare part.
    pub part: bool,
    /// Rounds in the job.
    pub rounds: u32,
    /// The job's value now.
    pub value: f64,
    /// The ship is under fire (the shake).
    pub combat: bool,
    /// The job's id, for the seeded stream.
    pub job_id: &'a str,
    /// The session's seed.
    pub seed: u64,
    said: Vec<Said>,
}

impl Ctx<'_> {
    /// This round's, phase's and part's seeded numbers.
    pub fn dice(&self) -> Dice {
        Dice(round_rng(self.seed, self.job_id, self.round, self.phase as u32, self.part))
    }
    /// Knob `name` at this round's level (data/repairs/<game>.json).
    pub fn knob(&self, name: &str) -> f32 {
        self.data.knob(name, self.level) as f32
    }
    /// The part move or this phase is played.
    pub fn step_done(&mut self) {
        self.said.push(Said::StepDone);
    }
    /// A mistake: the job's cost and the hazard, said on the bar.
    pub fn fumble(&mut self, what: &str) {
        self.said.push(Said::Fumble(what.to_owned()));
    }
    /// A short note on the bar (an event, never instructions).
    pub fn say(&mut self, what: &str) {
        self.said.push(Said::Say(what.to_owned()));
    }
}

/// A repair game: one machine, one move (design 2), ported from `docs/mockups/repairs/<id>.js`.
pub trait Game {
    /// Its id: its data file's name.
    fn id(&self) -> &'static str;
    /// The knobs it reads from its file; the load refuses a file with any other set.
    fn knobs(&self) -> &'static [&'static str];
    /// The phases of a round, in order (design 1a); empty for a game of one. `rounds` is the job's.
    fn phases(&self, _rounds: u32) -> &'static [&'static str] {
        &[]
    }
    /// Set up the step `cx` names: the part move when `cx.part`, else phase `cx.phase` of round `cx.round`.
    fn step(&mut self, cx: &mut Ctx);
    /// Play it for `dt` seconds.
    fn update(&mut self, cx: &mut Ctx, dt: f32, input: &Input);
    /// Draw it below the bar (the kit clips it there). `t` is the job's clock, seconds.
    fn draw(&self, pen: &Pen, cx: &Ctx, t: f32, input: &Input);
    /// The steady hand: what a careful player does this frame (tests and captures). It may keep its own memory.
    fn hand(&mut self, cx: &Ctx, t: f32) -> Input;
    /// The guide card's step it is in (0 up), or None.
    fn guide_now(&self) -> Option<usize> {
        None
    }
}

/// Where the cover is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CoverPhase {
    /// Screwed on; the screws turn out.
    Open,
    /// Coming off.
    Lifting,
    /// Off: the game is played.
    Work,
    /// Going back on.
    Lowering,
    /// On; the screws drive home.
    Close,
}

/// One screw of the cover.
#[derive(Clone, Debug, PartialEq)]
pub struct Screw {
    /// Where.
    pub x: f32,
    /// As x.
    pub y: f32,
    /// Radians turned the right way.
    pub turn: f32,
    /// Its picture's rotation.
    pub rot: f32,
}

/// Turns to free a screw or drive it home (owner, 2026-10-09: one loop).
pub const SCREW_TURNS: f32 = 1.0;
/// A screw head's radius.
pub const SCREW_R: f32 = 15.0;

/// The cover plate over a machine (kit: access panels).
#[derive(Clone, Debug, PartialEq)]
pub struct Cover {
    /// The plate.
    pub rect: [f32; 4],
    /// Where it is.
    pub phase: CoverPhase,
    /// How far it has lifted, 0-1.
    pub lift: f32,
    /// The selected screw.
    pub sel: usize,
    grab: Option<usize>,
    prev_a: Option<f32>,
    wrong_acc: f32,
    wrong_t: f32,
    /// The screws.
    pub screws: Vec<Screw>,
}

impl Cover {
    fn new(d: &CoverData) -> Self {
        let (x, y, w, h, m) = (d.x as f32, d.y as f32, d.w as f32, d.h as f32, 26.0);
        let spots: Vec<[f32; 2]> = if d.screws >= 6 {
            vec![
                [x + m, y + m],
                [x + w / 2.0, y + m],
                [x + w - m, y + m],
                [x + w - m, y + h - m],
                [x + w / 2.0, y + h - m],
                [x + m, y + h - m],
            ]
        } else {
            vec![[x + m, y + m], [x + w - m, y + m], [x + w - m, y + h - m], [x + m, y + h - m]]
        };
        Self {
            rect: [x, y, w, h],
            phase: CoverPhase::Open,
            lift: 0.0,
            sel: 0,
            grab: None,
            prev_a: None,
            wrong_acc: 0.0,
            wrong_t: 0.0,
            screws: spots
                .into_iter()
                .take(d.screws as usize)
                .map(|[x, y]| Screw { x, y, turn: 0.0, rot: 0.0 })
                .collect(),
        }
    }

    fn goal() -> f32 {
        SCREW_TURNS * std::f32::consts::TAU
    }

    /// Work the screws a frame: `dir` -1 unscrews (anticlockwise), +1 drives home. True when every one is done; a
    /// note when they were turned the wrong way.
    fn turn(&mut self, input: &Input, dt: f32, dir: f32) -> (bool, bool) {
        let goal = Self::goal();
        let left: Vec<usize> = (0..self.screws.len()).filter(|&i| self.screws[i].turn < goal).collect();
        if left.is_empty() {
            return (true, false);
        }
        if input.hit(Key::Tab) {
            self.sel = left.iter().copied().find(|&i| i > self.sel).unwrap_or(left[0]);
        }
        if self.screws[self.sel].turn >= goal {
            self.sel = left[0];
        }
        if input.pressed {
            let pts: Vec<Option<[f32; 2]>> =
                self.screws.iter().map(|s| (s.turn < goal).then_some([s.x, s.y])).collect();
            if let Some(i) = nearest(&pts, input.x, input.y, SCREW_R + 22.0) {
                self.grab = Some(i);
                self.sel = i;
                self.prev_a = None;
            }
        }
        if !input.down {
            self.grab = None;
        }
        let turn = |s: &mut Screw, d: f32| {
            s.turn = (s.turn + (d * dir).max(0.0)).min(goal);
            s.rot += d;
        };
        if let Some(g) = self.grab {
            let pts = if input.path.is_empty() { vec![[input.x, input.y]] } else { input.path.clone() };
            let mut wrong = 0.0;
            for [px, py] in pts {
                let (dx, dy) = (px - self.screws[g].x, py - self.screws[g].y);
                if dx.hypot(dy) <= 6.0 {
                    continue;
                }
                let a = dy.atan2(dx);
                if let Some(p) = self.prev_a {
                    let d = (a - p).sin().atan2((a - p).cos());
                    turn(&mut self.screws[g], d);
                    if d * dir < 0.0 {
                        wrong -= d * dir;
                    }
                }
                self.prev_a = Some(a);
            }
            self.wrong_acc = self.wrong_acc * (1.0 - dt).max(0.0) + wrong;
        }
        let mut say = false;
        if self.wrong_acc > 2.0 {
            self.wrong_acc = 0.0;
            self.wrong_t = 1.2;
            say = true;
        }
        self.wrong_t = (self.wrong_t - dt).max(0.0);
        if input.wheel != 0.0 {
            let i = (0..self.screws.len())
                .find(|&i| {
                    self.screws[i].turn < goal && input.dist(self.screws[i].x, self.screws[i].y) < SCREW_R + 22.0
                })
                .unwrap_or(self.sel);
            turn(&mut self.screws[i], -input.wheel * 0.7);
        }
        let key = input.stick[0];
        if key != 0.0 {
            let s = self.sel;
            turn(&mut self.screws[s], key * 9.0 * dt);
        }
        (self.screws.iter().all(|s| s.turn >= goal), say)
    }

    /// The steady hand on the cover: circle the next screw the right way.
    fn hand(&self, t: f32, dir: f32) -> Input {
        let goal = Self::goal();
        let Some(s) = self.screws.iter().find(|s| s.turn < goal) else { return Input::default() };
        let a0 = dir * t * 7.0;
        let mut path = Vec::new();
        for k in 0..4 {
            let a = a0 + dir * k as f32 * 0.03;
            path.push([s.x + a.cos() * 24.0, s.y + a.sin() * 24.0]);
        }
        let last = path[path.len() - 1];
        let pressed = self.grab.is_none();
        Input { x: last[0], y: last[1], path, down: true, pressed, ..Input::default() }
    }

    fn draw(&self, pen: &Pen, t: f32) {
        use std::f32::consts::{FRAC_PI_2, TAU};
        if self.lift >= 1.0 {
            return;
        }
        let [x, y, w, h] = self.rect;
        let goal = Self::goal();
        let p = pen.translate(0.0, -self.lift * (h + 120.0)).alpha(1.0 - self.lift * 0.6);
        p.round(x + 8.0, y + 10.0, w, h, 12.0, Some(rgba(0, 0, 0, 0.45)), None);
        p.clip(x, y, w, h).gradient(x, y, w, h, hex(0x3a4556), hex(0x262f3d), true);
        p.round(x, y, w, h, 12.0, None, Some((3.0, hex(0x556275))));
        let mut yy = y + 6.0;
        while yy < y + h {
            p.line(x + 6.0, yy, x + w - 6.0, yy, 1.0, rgba(255, 255, 255, 0.04));
            yy += 5.0;
        }
        let vs = ((w - 120.0) / 40.0).floor().clamp(0.0, 6.0) as i32;
        for i in 0..vs {
            p.round(
                x + w / 2.0 - (vs as f32 * 40.0) / 2.0 + i as f32 * 40.0 + 8.0,
                y + h - 64.0,
                24.0,
                30.0,
                6.0,
                Some(hex(0x151b25)),
                None,
            );
        }
        p.hatch(x + 60.0, y + 50.0, (w - 120.0).min(140.0), 14.0, rgba(242, 160, 70, 0.55));
        for (i, s) in self.screws.iter().enumerate() {
            let k = (s.turn / goal).min(1.0);
            let out = if self.phase == CoverPhase::Open { k } else { 1.0 - k };
            p.disc(s.x + 3.0 + 5.0 * out, s.y + 4.0 + 5.0 * out, SCREW_R + 1.0, rgba(0, 0, 0, 0.5));
            let head = if out > 0.98 && self.phase == CoverPhase::Open { hex(0x6d7686) } else { hex(0xb7c0cc) };
            p.disc(s.x, s.y, SCREW_R + 2.0 * out, head);
            p.ring(s.x, s.y, SCREW_R + 2.0 * out, hex(0x7a8494), 2.0);
            let q = p.translate(s.x, s.y).rotate(s.rot);
            q.line(-8.0, 0.0, 8.0, 0.0, 4.0, hex(0x2a313c));
            q.line(0.0, -8.0, 0.0, 8.0, 4.0, hex(0x2a313c));
            if s.turn < goal {
                let pdir = if self.phase == CoverPhase::Open { -1.0 } else { 1.0 };
                p.arc(s.x, s.y, SCREW_R + 9.0, -FRAC_PI_2, -FRAC_PI_2 + pdir * TAU * s.turn / goal, 4.0, c::AMBER);
                if i == self.sel {
                    p.ring(s.x, s.y, SCREW_R + 15.0 + (t * 5.0).sin() * 2.0, rgba(232, 238, 246, 0.35), 2.0);
                }
            } else {
                p.ring(s.x, s.y, SCREW_R + 9.0, c::OK, 3.0);
            }
        }
        if let Some(s) = self.screws.get(self.sel).filter(|s| s.turn < goal) {
            let open = self.phase == CoverPhase::Open;
            let wrong = self.wrong_t > 0.0 && (t * 18.0).sin() > -0.3;
            let col = if wrong { c::DANGER } else { rgba(232, 238, 246, 0.6) };
            let (a0, a1) = if open { (0.2, -1.6) } else { (-1.6, 0.2) };
            p.turn_arrow(s.x, s.y, SCREW_R + 26.0, a0, a1, col);
        }
    }
}

/// The guide's ? button in the bar.
const GUIDE_BTN: (f32, f32, f32) = (1234.0, 31.0, 24.0);
/// The card's frame.
const CARD: [f32; 4] = [120.0, 176.0, 1040.0, 400.0];

/// What the menu chose: who repairs, from what state, under fire or not.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Options {
    /// A player or a rating.
    pub who: Who,
    /// The state the job starts from.
    pub state: State,
    /// The ship under fire: the frame shakes.
    pub combat: bool,
    /// The session's seed.
    pub seed: u64,
}

impl Default for Options {
    fn default() -> Self {
        Self { who: Who::Officer, state: State::Damaged, combat: false, seed: 1 }
    }
}

/// One open repair: a game, its job and the frame round it (`kit.js`'s `run`).
pub struct Runner {
    game: Box<dyn Game>,
    /// The game's file.
    pub data: GameFile,
    /// The job.
    pub job: Job,
    /// The phase of the round being played.
    pub phase: usize,
    /// The cover, if the machine has one.
    pub cover: Option<Cover>,
    /// The options.
    pub opts: Options,
    rate: f64,
    /// The job is repaired and the cover is back on.
    pub done: bool,
    /// The job's clock.
    pub t: f32,
    shown: f64,
    flash: f32,
    shake: f32,
    note: String,
    note_t: f32,
    /// The guide card is up.
    pub guide_open: bool,
    guide_hold: bool,
    guide_t: f32,
    /// Seconds each landed round took to play, in order (for `min_round_s`, design 8).
    pub round_s: Vec<f32>,
    round_start: f32,
    job_id: String,
}

impl Runner {
    /// Open `game` with `opts`, its file from `data`; `guide_seen` when the player has closed its card before.
    pub fn new(game: Box<dyn Game>, data: &RepairData, opts: Options, guide_seen: bool) -> Result<Self, String> {
        let file = data.games.get(game.id()).ok_or_else(|| format!("no data/repairs/{}.json", game.id()))?.clone();
        file.needs(&sc_core::repair::data::game_file(game.id()), game.knobs()).map_err(|e| e.to_string())?;
        let job = Job::for_game(&file, &data.rules, &data.damage, opts.state);
        let rate = match (&file.job, opts.who) {
            (Some(j), _) => j.rate_per_s,
            (None, Who::Officer) => data.damage.kit_pct_per_s.officer,
            (None, Who::Rating) => data.damage.kit_pct_per_s.rating,
        };
        let auto = opts.who == Who::Rating && file.job.is_none();
        let cover = file.cover.as_ref().map(Cover::new);
        let mut r = Self {
            job_id: game.id().to_owned(),
            game,
            shown: job.value,
            data: file,
            job,
            phase: 0,
            cover,
            opts,
            rate,
            done: false,
            t: 0.0,
            flash: 0.0,
            shake: 0.0,
            note: String::new(),
            note_t: 0.0,
            guide_open: false,
            guide_hold: false,
            guide_t: 0.0,
            round_s: Vec::new(),
            round_start: 0.0,
        };
        r.guide_open = !auto && !guide_seen;
        r.guide_hold = r.guide_open;
        r.start_step();
        Ok(r)
    }

    /// The game's id.
    pub fn id(&self) -> &'static str {
        self.game.id()
    }

    /// A rating at work: no game.
    pub fn auto(&self) -> bool {
        self.opts.who == Who::Rating && self.data.job.is_none()
    }

    fn phases(&self) -> &'static [&'static str] {
        self.game.phases(self.job.rounds)
    }

    fn start_step(&mut self) {
        let ph = self.phases();
        let mut cx = ctx_of(&self.job, self.phase, ph, &self.opts, &self.data, &self.job_id);
        self.game.step(&mut cx);
    }

    fn say(&mut self, s: &str, t: f32) {
        self.note = s.to_owned();
        self.note_t = t;
    }

    /// The guide card: close it and let the game go at once (scripted play and captures, where nothing lets go of a
    /// press first).
    pub fn close_guide(&mut self) {
        self.guide_open = false;
        self.guide_hold = false;
    }

    /// The guide card: open it.
    pub fn show_guide(&mut self) {
        self.guide_open = true;
        self.guide_hold = true;
    }

    /// One frame of `dt` seconds with `input`. Returns true when the player closed the guide card (to remember it).
    pub fn update(&mut self, dt: f32, input: &Input) -> bool {
        let mut input = input.clone();
        input.derive();
        let mut closed = false;
        let on_btn = input.pressed && input.dist(GUIDE_BTN.0, GUIDE_BTN.1) <= GUIDE_BTN.2 + 10.0;
        let toggle = input.hit(Key::F1) || on_btn;
        if self.guide_open {
            self.guide_t += dt;
            if toggle || input.pressed || input.hit(Key::Escape) || input.action_pressed {
                self.guide_open = false;
                closed = true;
            }
        } else if toggle && !self.auto() {
            self.show_guide();
        }
        if self.guide_hold {
            if self.guide_open || input.down || !input.keys.is_empty() {
                return closed;
            }
            self.guide_hold = false;
        }
        self.t += dt;
        if !self.done {
            self.step(dt, &input);
        }
        self.shown += (self.job.value - self.shown) * f64::from((dt * 18.0).min(1.0));
        if (self.job.value - self.shown).abs() < 0.05 {
            self.shown = self.job.value;
        }
        self.flash = (self.flash - dt * 2.5).max(0.0);
        self.note_t -= dt;
        self.shake = (self.shake - dt * 1.5).max(0.0);
        closed
    }

    fn step(&mut self, dt: f32, input: &Input) {
        if self.auto() {
            self.job.work(f64::from(dt), self.rate);
            if let Some(c) = &mut self.cover {
                c.lift = 1.0;
                c.phase = CoverPhase::Work;
            }
            if self.job.done() {
                self.finish();
            }
            return;
        }
        if let Some(c) = &mut self.cover {
            match c.phase {
                CoverPhase::Open => {
                    let (all, wrong) = c.turn(input, dt, -1.0);
                    if all {
                        c.phase = CoverPhase::Lifting;
                    }
                    if wrong {
                        self.say("Other way", 1.2);
                    }
                    return;
                }
                CoverPhase::Lifting => {
                    c.lift = (c.lift + dt * 2.5).min(1.0);
                    if c.lift >= 1.0 {
                        c.phase = CoverPhase::Work;
                        self.round_start = self.t;
                    }
                    return;
                }
                CoverPhase::Lowering => {
                    c.lift = (c.lift - dt * 2.5).max(0.0);
                    if c.lift <= 0.0 {
                        c.phase = CoverPhase::Close;
                        c.sel = 0;
                    }
                    return;
                }
                CoverPhase::Close => {
                    let (all, wrong) = c.turn(input, dt, 1.0);
                    if wrong {
                        self.say("Other way", 1.2);
                    }
                    if all {
                        self.finish();
                    }
                    return;
                }
                CoverPhase::Work => {}
            }
        }
        let ph = self.phases();
        let mut cx = ctx_of(&self.job, self.phase, ph, &self.opts, &self.data, &self.job_id);
        self.game.update(&mut cx, dt, input);
        let said = std::mem::take(&mut cx.said);
        for s in said {
            match s {
                Said::Say(w) => self.say(&w, 2.0),
                Said::Fumble(w) => {
                    self.flash = 1.0;
                    self.shake = self.shake.max(0.6);
                    let what = if w.is_empty() { self.data.hazard.text.clone() } else { w };
                    self.say(&what, 2.5);
                    if self.job.fumble() == Fumbled::Restart {
                        self.phase = 0;
                        self.say("Round restarted", 2.0);
                        self.start_step();
                    }
                }
                Said::StepDone => {
                    if self.job.part_pending {
                        self.job.fit_part();
                        self.say("Part fitted", 1.2);
                        self.start_step();
                    } else if self.phase + 1 < self.phases().len().max(1) {
                        self.phase += 1;
                        self.start_step();
                    } else {
                        self.land();
                    }
                    // A game says one step at a time; anything after it was for the step it ended.
                    break;
                }
            }
        }
    }

    fn land(&mut self) {
        self.job.land();
        self.round_s.push(self.t - self.round_start);
        self.round_start = self.t;
        self.phase = 0;
        if !self.job.done() {
            let n = self.job.landed;
            self.say(&format!("Round {n} done"), 1.4);
            self.start_step();
            return;
        }
        match &mut self.cover {
            Some(c) => {
                c.phase = CoverPhase::Lowering;
                for s in &mut c.screws {
                    s.turn = 0.0;
                }
            }
            None => self.finish(),
        }
    }

    fn finish(&mut self) {
        self.done = true;
        let w = self.data.done_word.clone().unwrap_or_else(|| "Repaired".into());
        self.say(&w, 99.0);
    }

    /// The steady hand for this frame: the cover's, or the game's.
    pub fn hand(&mut self) -> Input {
        if self.guide_open {
            return Input::default();
        }
        if let Some(c) = &self.cover {
            match c.phase {
                CoverPhase::Open => return c.hand(self.t, -1.0),
                CoverPhase::Close => return c.hand(self.t, 1.0),
                CoverPhase::Lifting | CoverPhase::Lowering => return Input::default(),
                CoverPhase::Work => {}
            }
        }
        let ph = self.phases();
        let cx = ctx_of(&self.job, self.phase, ph, &self.opts, &self.data, &self.job_id);
        self.game.hand(&cx, self.t)
    }

    /// Draw the frame into `screen` (points) on `painter`.
    pub fn draw(&self, painter: &egui::Painter, screen: Rect, input: &Input) {
        let pen = Pen::new(painter.clone(), screen);
        pen.rect(0.0, 0.0, W, H, c::BG);
        let s = if self.opts.combat { 5.0 + 3.0 * (self.t * 1.7).sin() } else { 0.0 } + self.shake * 10.0;
        let shaken = if s > 0.0 {
            let k = (self.t * 1000.0) as u64;
            let mut r = Rng::for_purpose(self.opts.seed, k, "shake");
            pen.translate((r.next_f64() as f32 - 0.5) * s, (r.next_f64() as f32 - 0.5) * s)
        } else {
            pen.clone()
        };
        let below = shaken.clip(0.0, BAR_H, W, H - BAR_H);
        let idle = Input::default();
        let inp = if self.guide_hold { &idle } else { input };
        if self.auto() {
            below.text("RATING AT WORK", W / 2.0, H / 2.0, 40.0, c::DIM, Align::Center);
        } else {
            let cx = ctx_of(&self.job, self.phase, self.phases(), &self.opts, &self.data, &self.job_id);
            self.game.draw(&below, &cx, self.t, inp);
        }
        if let (Some(c), false) = (&self.cover, self.auto()) {
            c.draw(&below, self.t);
        }
        if self.done {
            below.rect(0.0, BAR_H, W, H - BAR_H, rgba(4, 6, 10, 0.55));
            let w = self.data.done_word.clone().unwrap_or_else(|| "Repaired".into()).to_uppercase();
            below.text(&w, W / 2.0, H / 2.0, 64.0, c::OK, Align::Center);
        }
        if self.flash > 0.0 {
            below.rect(0.0, BAR_H, W, H - BAR_H, rgba(255, 71, 87, 0.25 * self.flash));
        }
        self.draw_bar(&shaken);
        if self.guide_open {
            self.draw_guide(&pen);
        }
    }

    fn draw_bar(&self, g: &Pen) {
        let job = &self.job;
        g.rect(0.0, 0.0, W, BAR_H, c::PANEL2);
        g.rect(0.0, BAR_H - 2.0, W, 2.0, c::LINE);
        g.text(&self.data.title.to_uppercase(), 24.0, 26.0, 26.0, c::AMBER, Align::Left);
        g.text(&self.data.place, 24.0, 52.0, 16.0, c::DIM, Align::Left);
        let (x0, x1, y, h) = (380.0f32, 1000.0f32, 22.0f32, 22.0f32);
        let f = |v: f64| x0 + (x1 - x0) * (v / job.target.max(100.0)).clamp(0.0, 1.0) as f32;
        g.round(x0, y, x1 - x0, h, 11.0, Some(hex(0x1a2230)), None);
        g.round(x0, y, f(job.cap()) - x0, h, 11.0, Some(rgba(79, 195, 247, 0.18)), None);
        let v = self.shown;
        let good = if v >= 75.0 {
            c::OK
        } else if v >= 25.0 {
            c::WARN
        } else {
            c::DANGER
        };
        g.round(x0, y, (f(v) - x0).max(h), h, 11.0, Some(if self.flash > 0.0 { c::DANGER } else { good }), None);
        let unit = if self.data.job.as_ref().is_some_and(|j| j.unit == "HP") { " HP" } else { "%" };
        g.text(&format!("{}{unit}", v.round()), x1 + 14.0, y + h / 2.0, 22.0, c::FG, Align::Left);
        let nph = self.phases().len();
        for i in 0..job.rounds {
            let (cx, cy) = (x0 + 10.0 + i as f32 * 26.0, 56.0);
            let col = if i < job.landed {
                c::OK
            } else if i == job.landed && !self.done {
                c::AMBER
            } else {
                hex(0x2a3446)
            };
            g.disc(cx, cy, 7.0, col);
            if i == job.landed && !self.done && nph > 1 {
                for k in 0..nph {
                    let col = if k < self.phase {
                        c::OK
                    } else if k == self.phase {
                        c::AMBER
                    } else {
                        hex(0x2a3446)
                    };
                    g.rect(cx - (nph as f32 * 7.0) / 2.0 + k as f32 * 7.0, cy + 10.0, 5.0, 3.0, col);
                }
            }
        }
        let rounds = job.rounds_left();
        let left = if self.auto() {
            format!("{:.0} s", job.time_left(self.rate))
        } else {
            format!("{rounds} round{} left, level {}", if rounds == 1 { "" } else { "s" }, job.level())
        };
        if !self.done && self.note_t <= 0.0 {
            g.text(&left, x1 + 14.0, 58.0, 16.0, c::DIM, Align::Left);
        }
        for i in 0..3u32 {
            g.rect(1140.0 + i as f32 * 20.0, 18.0, 12.0, 26.0, if i < job.fumbles { c::DANGER } else { hex(0x2a3446) });
        }
        let (bx, by, br) = GUIDE_BTN;
        let open = self.guide_open;
        g.disc(bx, by, br, if open { c::ACCENT } else { hex(0x262e42) });
        g.ring(bx, by, br, if open { hex(0xe8f7ff) } else { hex(0x4a5a70) }, 2.0);
        g.text("?", bx, by + 2.0, 30.0, if open { hex(0x04131c) } else { c::FG }, Align::Center);
        if self.note_t > 0.0 {
            g.text(&self.note, 1240.0, 62.0, 16.0, if self.done { c::OK } else { c::DANGER }, Align::Right);
        }
    }

    fn guide_now(&self) -> Option<usize> {
        let gd = &self.data.guide;
        if let Some(c) = &self.cover {
            if !self.auto() && c.phase != CoverPhase::Work {
                return gd.steps.iter().position(|s| s.cover);
            }
        }
        self.game.guide_now()
    }

    fn draw_guide(&self, g: &Pen) {
        let gd = &self.data.guide;
        let now = self.guide_now();
        let t = self.guide_t;
        let [x, y, w, h] = CARD;
        g.rect(0.0, BAR_H, W, H - BAR_H, rgba(4, 6, 10, 0.72));
        g.panel(x, y, w, h, 22.0, hex(0x0e151f), hex(0x33445a));
        g.cross(x + w - 34.0, y + 34.0, 10.0, 4.0, c::DIM);
        let n = gd.steps.len().min(4);
        let col_w = ((w - 80.0) / n.max(1) as f32).min(240.0);
        let (x0, tile, ty) = (x + (w - col_w * n as f32) / 2.0, 150.0, y + 52.0);
        for (i, st) in gd.steps.iter().take(4).enumerate() {
            let cx = x0 + i as f32 * col_w + col_w / 2.0;
            let lit = now == Some(i);
            g.round(
                cx - tile / 2.0,
                ty,
                tile,
                tile,
                18.0,
                Some(if lit { hex(0x1d2a3c) } else { hex(0x121a25) }),
                Some((if lit { 4.0 } else { 2.0 }, if lit { c::AMBER } else { hex(0x2a3446) })),
            );
            let ip = g.clip(cx - 72.0, ty + 3.0, 144.0, 144.0).translate(cx, ty + tile / 2.0).scale(1.25, 1.25);
            crate::icons::draw(&ip, &st.icon, t);
            g.disc(cx - tile / 2.0 + 4.0, ty + 4.0, 18.0, if lit { c::AMBER } else { hex(0x26324a) });
            g.ring(cx - tile / 2.0 + 4.0, ty + 4.0, 18.0, if lit { hex(0xffe2bd) } else { hex(0x4a5568) }, 2.0);
            g.text(
                &(i + 1).to_string(),
                cx - tile / 2.0 + 4.0,
                ty + 5.0,
                24.0,
                if lit { hex(0x1a0f02) } else { c::FG },
                Align::Center,
            );
            if i + 1 < n {
                let ax = x0 + (i + 1) as f32 * col_w;
                g.path(
                    &[
                        [ax - 6.0, ty + tile / 2.0 - 12.0],
                        [ax + 4.0, ty + tile / 2.0],
                        [ax - 6.0, ty + tile / 2.0 + 12.0],
                    ],
                    false,
                    4.0,
                    hex(0x4a5568),
                );
            }
            for (k, ln) in wrap(g, &st.text, col_w - 22.0, 24.0).iter().take(3).enumerate() {
                g.text(
                    ln,
                    cx,
                    ty + tile + 30.0 + k as f32 * 28.0,
                    24.0,
                    if lit { c::FG } else { hex(0xc4cfdc) },
                    Align::Center,
                );
            }
        }
        if !gd.mistake.is_empty() {
            let my = y + h - 52.0;
            g.round(
                x + 30.0,
                my - 28.0,
                w - 60.0,
                56.0,
                14.0,
                Some(hex(0x1c0f14)),
                Some((2.0, rgba(255, 71, 87, 0.55))),
            );
            warn_glyph(g, x + 66.0, my + 1.0, 16.0);
            let max = w - 150.0;
            let mut s = gd.mistake.clone();
            if g.text_width(&s, 24.0) > max {
                while s.len() > 4 && g.text_width(&format!("{s}..."), 24.0) > max {
                    s.pop();
                }
                s = format!("{}...", s.trim_end());
            }
            g.text(&s, x + 96.0, my + 1.0, 24.0, hex(0xffd2d6), Align::Left);
        }
    }
}

fn ctx_of<'a>(
    job: &Job,
    phase: usize,
    phases: &'static [&'static str],
    opts: &Options,
    data: &'a GameFile,
    job_id: &'a str,
) -> Ctx<'a> {
    Ctx {
        data,
        round: job.landed,
        level: job.level(),
        phase,
        phase_name: phases.get(phase).copied().unwrap_or(""),
        part: job.part_pending,
        rounds: job.rounds,
        value: job.value,
        combat: opts.combat,
        job_id,
        seed: opts.seed,
        said: Vec::new(),
    }
}

/// A warning triangle: the mistake's line.
fn warn_glyph(g: &Pen, x: f32, y: f32, s: f32) {
    g.poly(&[[x, y - s], [x + s * 1.1, y + s * 0.85], [x - s * 1.1, y + s * 0.85]], c::DANGER);
    g.rect(x - 2.5, y - s * 0.45, 5.0, s * 0.75, hex(0x1a0508));
    g.rect(x - 2.5, y + s * 0.45, 5.0, 5.0, hex(0x1a0508));
}

/// Words wrapped to lines no wider than `w`; two lines are balanced so a step never leaves one word alone.
fn wrap(g: &Pen, s: &str, w: f32, size: f32) -> Vec<String> {
    let words: Vec<&str> = s.split_whitespace().collect();
    let mut out: Vec<String> = Vec::new();
    let mut cur = String::new();
    for word in &words {
        let next = if cur.is_empty() { (*word).to_owned() } else { format!("{cur} {word}") };
        if !cur.is_empty() && g.text_width(&next, size) > w {
            out.push(std::mem::replace(&mut cur, (*word).to_owned()));
        } else {
            cur = next;
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    if out.len() != 2 {
        return out;
    }
    let mut best = out;
    let mut bw = f32::INFINITY;
    for i in 1..words.len() {
        let (a, b) = (words[..i].join(" "), words[i..].join(" "));
        let m = g.text_width(&a, size).max(g.text_width(&b, size));
        if m <= w && m < bw {
            bw = m;
            best = vec![a, b];
        }
    }
    best
}

/// A dim colour for things not in play (shared by games).
pub fn quiet(col: Color32) -> Color32 {
    alpha(col, 0.35)
}

/// What a played job came to (the tests' and the captures' report).
#[derive(Clone, Debug, PartialEq)]
pub struct Played {
    /// Repaired, the cover back on.
    pub done: bool,
    /// Seconds on the job's clock.
    pub t_s: f32,
    /// Fumbles over the job.
    pub fumbles: u32,
    /// Seconds each round took.
    pub round_s: Vec<f32>,
}

/// Play `r` with its steady hand at 60 frames a second for up to `max_s` seconds (`each` sees every frame).
pub fn play(r: &mut Runner, max_s: f32, mut each: impl FnMut(&Runner, &Input)) -> Played {
    let dt = 1.0 / 60.0;
    r.close_guide();
    let mut n = 0;
    while !r.done && (n as f32) * dt < max_s {
        let i = r.hand();
        r.update(dt, &i);
        each(r, &i);
        n += 1;
    }
    Played { done: r.done, t_s: r.t, fumbles: r.job.fumbles_total, round_s: r.round_s.clone() }
}

/// A game not in the engine yet: the menu lists it, and opening it says so.
pub struct Unported(pub &'static str);

impl Game for Unported {
    fn id(&self) -> &'static str {
        self.0
    }
    fn knobs(&self) -> &'static [&'static str] {
        &[]
    }
    fn step(&mut self, _cx: &mut Ctx) {}
    fn update(&mut self, _cx: &mut Ctx, _dt: f32, _input: &Input) {}
    fn draw(&self, pen: &Pen, _cx: &Ctx, _t: f32, _input: &Input) {
        pen.text("NOT IN THE ENGINE YET", W / 2.0, H / 2.0, 40.0, c::DIM, Align::Center);
        pen.text("Its mockup is docs/mockups/repairs.html", W / 2.0, H / 2.0 + 44.0, 20.0, c::DIM, Align::Center);
    }
    fn hand(&mut self, _cx: &Ctx, _t: f32) -> Input {
        Input::default()
    }
}
