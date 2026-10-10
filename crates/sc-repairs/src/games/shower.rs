//! A shower's repair (repair-minigames design 2 and 1a), from the shower half of `docs/mockups/repairs/heads.js`.
//!
//! A pipe puzzle in the deckhead. Water comes down the main on the left; the shower waits on the right. Click a tile
//! to turn it a quarter (or move the cursor with the arrows and turn it with Space) until one run joins the main to
//! the shower with no open end anywhere along it, then open the valve on the main (click it, or Enter): the water runs
//! through and the shower runs. Each level is a bigger grid. Opening the valve on an unfinished run is the fumble
//! "Loose joint: you are soaked". A shower has no part to fit: a disabled shower's first round starts at once. The
//! water's reach is the kit's `flood`, shared with the coolant pipes; the duck and the ellipse helpers are the
//! toilet's (the two were one script in the mockup).

use std::f32::consts::{FRAC_PI_2, PI};

use egui::Key;

use super::toilet::{duck, oval};
use crate::kit::{flood, Ctx, Dice, Game, Input, BAR_H, H, W};
use crate::pen::{c, hex, rgba, Pen, PipeStyle};

/// In the engine.
pub const PORTED: bool = true;

/// A fresh game.
pub fn new() -> Box<dyn Game> {
    Box::new(Shower::default())
}

const SOAKED: &str = "Loose joint: you are soaked";
/// A tile, px.
const T: f32 = 84.0;
/// The four ways out of a tile, N E S W: its bit, and the column and row it leads to.
const DIRS: [(u8, i32, i32); 4] = [(1, 0, -1), (2, 1, 0), (4, 0, 1), (8, -1, 0)];
/// The main's riser, x, and its valve: x and radius.
const RISER: f32 = 100.0;
const VX: f32 = 196.0;
const VR: f32 = 36.0;
/// The shower stall: x, y, w, h.
const FIX: [f32; 4] = [990.0, 100.0, 250.0, 540.0];
/// The shower head's face.
const HEAD: [f32; 2] = [1140.0, 190.0];
/// Water runs this many tiles a second.
const FLOW: f32 = 6.0;

fn opp(b: u8) -> u8 {
    match b {
        1 => 4,
        2 => 8,
        4 => 1,
        _ => 2,
    }
}

/// A tile's arms turned a quarter `k` times clockwise.
fn turn(m: u8, k: u32) -> u8 {
    let mut m = m;
    for _ in 0..k % 4 {
        m = ((m << 1) | (m >> 3)) & 15;
    }
    m
}

#[derive(Clone, Debug, Default)]
struct Tile {
    base: u8,
    k: u32,
    ang: f32,
    run: bool,
    /// On the run: the arms it must show.
    want: u8,
}

impl Tile {
    fn open(&self) -> u8 {
        turn(self.base, self.k)
    }
}

/// The water's reach from the main: each tile it fills and its distance, the open ends (x, y, the way out), whether
/// it reaches the shower whole, and the furthest distance.
#[derive(Clone, Debug, Default)]
struct Net {
    cells: Vec<(usize, u32)>,
    leaks: Vec<(f32, f32, u8)>,
    whole: bool,
    far: u32,
}

impl Net {
    fn dist(&self, key: usize) -> Option<u32> {
        self.cells.iter().find(|(k, _)| *k == key).map(|(_, d)| *d)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Phase {
    #[default]
    Play,
    Flow,
}

/// The game's state.
#[derive(Default)]
pub struct Shower {
    skip_part: bool,
    cols: usize,
    rows: usize,
    gx: f32,
    gy: f32,
    r0: usize,
    r1: usize,
    y_main: f32,
    tiles: Vec<Tile>,
    cur: (usize, usize),
    keys: bool,
    phase: Phase,
    flow: f32,
    run_t: f32,
    spray: f32,
    net: Net,
    va: f32,
    running: bool,
    /// The hand: when it may act next (the job's clock), and whether its finger is down.
    hand_next: f32,
    hand_down: bool,
}

impl Shower {
    fn at(&self, c: i32, r: i32) -> Option<&Tile> {
        (c >= 0 && r >= 0 && (c as usize) < self.cols && (r as usize) < self.rows)
            .then(|| &self.tiles[r as usize * self.cols + c as usize])
    }

    fn centre(&self, c: usize, r: usize) -> [f32; 2] {
        [self.gx + c as f32 * T + T / 2.0, self.gy + r as f32 * T + T / 2.0]
    }

    /// A random self-avoiding run from the main's tile to the shower's, leaning right.
    fn route(&self, r: &mut Dice) -> Vec<(usize, usize)> {
        struct Walk<'a> {
            cols: usize,
            rows: usize,
            r1: usize,
            seen: Vec<bool>,
            path: Vec<(usize, usize)>,
            budget: i32,
            r: &'a mut Dice,
        }
        fn go(w: &mut Walk, c: usize, rr: usize) -> bool {
            w.budget -= 1;
            if w.budget < 0 {
                return false;
            }
            w.seen[rr * w.cols + c] = true;
            w.path.push((c, rr));
            if c == w.cols - 1 && rr == w.r1 {
                return true;
            }
            let mut opts: Vec<((u8, i32, i32), f32)> =
                DIRS.iter().map(|d| (*d, w.r.f() + if d.1 > 0 { 0.35 } else { 0.0 })).collect();
            opts.sort_by(|a, b| b.1.total_cmp(&a.1));
            for ((_, dc, dr), _) in opts {
                let (nc, nr) = (c as i32 + dc, rr as i32 + dr);
                if nc < 0 || nr < 0 || nc >= w.cols as i32 || nr >= w.rows as i32 {
                    continue;
                }
                let (nc, nr) = (nc as usize, nr as usize);
                if w.seen[nr * w.cols + nc] {
                    continue;
                }
                if go(w, nc, nr) {
                    return true;
                }
            }
            w.path.pop();
            false
        }
        let mut w = Walk {
            cols: self.cols,
            rows: self.rows,
            r1: self.r1,
            seen: vec![false; self.cols * self.rows],
            path: Vec::new(),
            budget: 4000,
            r,
        };
        if go(&mut w, 0, self.r0) {
            return w.path;
        }
        // The budget ran out: straight along, then down or up.
        let mut p: Vec<(usize, usize)> = (0..self.cols).map(|c| (c, self.r0)).collect();
        let mut rr = self.r0 as i32;
        let dir = (self.r1 as i32 - self.r0 as i32).signum();
        while rr != self.r1 as i32 {
            p.push((self.cols - 1, (rr + dir) as usize));
            rr += dir;
        }
        p
    }

    /// The water's reach from the main through the tiles as they are turned now (the kit's `flood`).
    fn network(&self) -> Net {
        let yc = self.centre(0, self.r0)[1];
        let first = self.r0 * self.cols;
        if self.tiles[first].open() & 8 == 0 {
            return Net { leaks: vec![(self.gx, yc, 2)], ..Net::default() };
        }
        let mut leaks = Vec::new();
        let mut reach = false;
        let cols = self.cols;
        let cells = flood(&[first], |key| {
            let (c, r) = (key % cols, key / cols);
            let m = self.tiles[key].open();
            let mut out = Vec::new();
            for &(b, dc, dr) in &DIRS {
                if m & b == 0 {
                    continue;
                }
                if c == 0 && r == self.r0 && b == 8 {
                    continue;
                }
                if c == cols - 1 && r == self.r1 && b == 2 {
                    reach = true;
                    continue;
                }
                let (nc, nr) = (c as i32 + dc, r as i32 + dr);
                let [x, y] = self.centre(c, r);
                match self.at(nc, nr) {
                    Some(n) if n.open() & opp(b) != 0 => out.push(nr as usize * cols + nc as usize),
                    _ => leaks.push((x + dc as f32 * T / 2.0, y + dr as f32 * T / 2.0, b)),
                }
            }
            out
        });
        let far = cells.iter().map(|(_, d)| *d).max().unwrap_or(0);
        let whole = reach && leaks.is_empty();
        Net { cells, leaks, whole, far }
    }

    fn open_valve(&mut self, cx: &mut Ctx) {
        self.net = self.network();
        if self.net.whole {
            self.phase = Phase::Flow;
            self.flow = 0.0;
            return;
        }
        self.spray = 1.4;
        cx.fumble(SOAKED);
    }

    fn pipe(g: &Pen, pts: Vec<[f32; 2]>, w: f32, wet: bool) {
        g.pipe(&[(pts, 1.0)], &PipeStyle { w, fluid: wet.then_some(c::ACCENT), ..PipeStyle::default() });
    }

    fn valve_glyph(g: &Pen, x: f32, y: f32, a: f32, on: bool) {
        g.disc(x, y, VR, hex(0x141b27));
        g.ring(x, y, VR - 5.0, if on { c::OK } else { hex(0xb0473f) }, 10.0);
        for k in 0..3 {
            let b = a + k as f32 * PI * 2.0 / 3.0;
            g.line(x, y, x + b.cos() * (VR - 10.0), y + b.sin() * (VR - 10.0), 7.0, hex(0x9aa6b6));
        }
        g.disc(x, y, 10.0, hex(0x2a3446));
        g.ring(x, y, 10.0, hex(0x9aa6b6), 2.0);
    }

    fn spray_at(g: &Pen, x: f32, y: f32, b: u8, t: f32) {
        let d = DIRS.iter().find(|q| q.0 == b).copied().unwrap_or(DIRS[1]);
        let (dc, dr) = (d.1 as f32, d.2 as f32);
        for j in 0..16 {
            let ph = (t * 2.2 + j as f32 / 16.0).rem_euclid(1.0);
            let side = (j % 8) as f32 - 3.5;
            let px = x + dc * ph * 80.0 + if dc != 0.0 { 0.0 } else { side * ph * 9.0 };
            let py = y + dr * ph * 80.0 + if dc != 0.0 { side * ph * 9.0 } else { 0.0 } + ph * ph * 60.0;
            g.disc(px, py, 5.0 - 3.0 * ph, rgba(79, 195, 247, 0.9));
        }
    }

    fn stall(&self, g: &Pen, t: f32, running: bool) {
        // The stall: tiled walls, a curtain bunched on its rail, the head, the tray, and a duck on the tray.
        let [fx, fy, fw, fh] = FIX;
        g.panel(fx, fy, fw, fh, 14.0, hex(0x101a24), c::LINE);
        let mut y = fy + 34.0;
        while y < fy + fh - 30.0 {
            g.rect(fx + 6.0, y, fw - 12.0, 2.0, hex(0x16222f));
            y += 34.0;
        }
        let mut x = fx + 40.0;
        while x < fx + fw {
            g.rect(x, fy + 6.0, 2.0, fh - 40.0, hex(0x16222f));
            x += 40.0;
        }
        let y1 = self.centre(self.cols - 1, self.r1)[1];
        Self::pipe(
            g,
            vec![[fx - 20.0, y1], [fx + 34.0, y1], [fx + 34.0, 150.0], [HEAD[0], 150.0], [HEAD[0], 166.0]],
            16.0,
            running,
        );
        g.rect(fx + 10.0, 116.0, fw - 20.0, 6.0, hex(0x9aa6b6));
        for k in 0..4 {
            g.round(fx + fw - 52.0 + k as f32 * 10.0, 122.0, 12.0, 440.0, 6.0, Some(c::LILAC), None);
        }
        for k in 0..4 {
            g.rect(fx + fw - 44.0 + k as f32 * 10.0, 126.0, 3.0, 430.0, rgba(0, 0, 0, 0.25));
        }
        // The head: a rose on its arm, angled down.
        g.poly(
            &[[HEAD[0] - 34.0, 166.0], [HEAD[0] + 34.0, 166.0], [HEAD[0] + 46.0, HEAD[1]], [HEAD[0] - 46.0, HEAD[1]]],
            c::STEEL,
        );
        for k in 0..7 {
            g.rect(HEAD[0] - 39.0 + k as f32 * 12.0, HEAD[1], 5.0, 4.0, hex(0x4a5566));
        }
        // The tray and its drain.
        g.rect(fx + 14.0, 604.0, fw - 28.0, 20.0, hex(0x3a4658));
        g.poly(&oval(HEAD[0] - 20.0, 606.0, 18.0, 4.0), hex(0x151b23));
        if running {
            let k = (self.run_t / 0.5).min(1.0);
            g.rect(fx + 20.0, 598.0, fw - 40.0, 8.0, rgba(79, 195, 247, 0.45 * k));
            // The fall: a cone of streaks from the rose, splashes on the tray, a little steam.
            for j in 0..60 {
                let ph = (t * 1.7 + (j as f32 * 0.618).rem_euclid(1.0)).rem_euclid(1.0);
                let lane = ((j * 7) % 23) as f32 / 22.0 - 0.5;
                let y = HEAD[1] + 6.0 + ph * 400.0 * k;
                let x = HEAD[0] - 6.0 + lane * (84.0 + (y - HEAD[1]) * 0.2);
                g.rect(x - 1.5, y, 3.0, 18.0, rgba(120, 205, 250, 0.5 + 0.4 * (1.0 - ph)));
            }
            for j in 0..12 {
                let ph = (t * 2.4 + j as f32 / 12.0).rem_euclid(1.0);
                let sx = HEAD[0] - 10.0 + ((j as f32 * 0.7).rem_euclid(1.0) - 0.5) * 150.0;
                let side = if j % 2 == 1 { 1.0 } else { -1.0 };
                g.disc(sx + side * ph * 30.0, 598.0 - (ph * PI).sin() * 26.0, 3.0, rgba(160, 220, 250, 0.8));
            }
            for j in 0..5 {
                let ph = (t * 0.25 + j as f32 / 5.0).rem_euclid(1.0);
                g.disc(
                    fx + 50.0 + j as f32 * 36.0 + (t + j as f32).sin() * 10.0,
                    560.0 - ph * 380.0,
                    18.0 + ph * 26.0,
                    rgba(220, 230, 240, 0.04 * (1.0 - ph)),
                );
            }
        }
        let bob = if running { (t * 5.0).sin() * 3.0 } else { 0.0 };
        duck(g, fx + 56.0, 588.0 + bob, 0.7, if running { (t * 4.0).sin() * 0.15 } else { 0.0 });
    }
}

impl Game for Shower {
    fn id(&self) -> &'static str {
        "shower"
    }

    fn knobs(&self) -> &'static [&'static str] {
        &["cols_count", "rows_count"]
    }

    fn step(&mut self, cx: &mut Ctx) {
        let mut r = cx.dice();
        let (cols, rows) =
            (cx.knob("cols_count").round().max(2.0) as usize, cx.knob("rows_count").round().max(1.0) as usize);
        *self = Shower { skip_part: cx.part, cols, rows, ..Shower::default() };
        self.gx = 600.0 - cols as f32 * T / 2.0;
        self.gy = 410.0 - rows as f32 * T / 2.0;
        self.r0 = (r.f() * rows as f32) as usize;
        self.r1 = (r.f() * rows as f32) as usize;
        self.y_main = self.centre(0, self.r0)[1];
        // Decoys first, then the run laid over them; then every tile turned at random.
        for _ in 0..cols * rows {
            let q = r.f();
            self.tiles.push(Tile {
                base: if q < 0.45 {
                    3
                } else if q < 0.8 {
                    5
                } else {
                    7
                },
                ..Tile::default()
            });
        }
        let path = self.route(&mut r);
        let dir_to = |a: (usize, usize), b: (usize, usize)| {
            DIRS.iter().find(|d| a.0 as i32 + d.1 == b.0 as i32 && a.1 as i32 + d.2 == b.1 as i32).map_or(0, |d| d.0)
        };
        for (i, &(c, rr)) in path.iter().enumerate() {
            let prev = if i > 0 { dir_to(path[i], path[i - 1]) } else { 8 };
            let next = if i + 1 < path.len() { dir_to(path[i], path[i + 1]) } else { 2 };
            let m = prev | next;
            let tile = &mut self.tiles[rr * cols + c];
            tile.base = if m == 5 || m == 10 { 5 } else { 3 };
            tile.run = true;
            tile.want = m;
        }
        for tile in &mut self.tiles {
            tile.k = (r.f() * 4.0) as u32;
            tile.ang = tile.k as f32 * FRAC_PI_2;
        }
        if self.network().whole {
            let tile = &mut self.tiles[self.r0 * cols];
            tile.k += 1;
            tile.ang = tile.k as f32 * FRAC_PI_2;
        }
    }

    fn update(&mut self, cx: &mut Ctx, dt: f32, input: &Input) {
        if self.skip_part {
            // A shower has no part to fit: its round starts at once.
            self.skip_part = false;
            cx.step_done();
            return;
        }
        self.spray = (self.spray - dt).max(0.0);
        for tile in &mut self.tiles {
            tile.ang += (tile.k as f32 * FRAC_PI_2 - tile.ang) * (dt * 16.0).min(1.0);
        }
        if self.phase == Phase::Flow {
            self.flow += FLOW * dt;
            self.va = (self.va + dt * 6.0).min(PI * 1.5);
            if self.flow > self.net.far as f32 + 1.5 {
                if !self.running {
                    self.running = true;
                    cx.step_done();
                }
                self.run_t += dt;
            }
            return;
        }
        // Keys: a cursor, Space turns, Enter opens the valve.
        for (k, dc, dr) in [
            (Key::ArrowLeft, -1, 0),
            (Key::A, -1, 0),
            (Key::ArrowRight, 1, 0),
            (Key::D, 1, 0),
            (Key::ArrowUp, 0, -1),
            (Key::W, 0, -1),
            (Key::ArrowDown, 0, 1),
            (Key::S, 0, 1),
        ] {
            if input.hit(k) {
                self.keys = true;
                self.cur.0 = (self.cur.0 as i32 + dc).clamp(0, self.cols as i32 - 1) as usize;
                self.cur.1 = (self.cur.1 as i32 + dr).clamp(0, self.rows as i32 - 1) as usize;
            }
        }
        if input.hit(Key::Space) {
            self.keys = true;
            let (c, r) = self.cur;
            self.tiles[r * self.cols + c].k += 1;
        }
        if input.hit(Key::Enter) && self.spray == 0.0 {
            self.open_valve(cx);
            return;
        }
        if input.pressed {
            if (input.x - VX).hypot(input.y - self.y_main) < VR + 8.0 {
                if self.spray == 0.0 {
                    self.open_valve(cx);
                }
                return;
            }
            let (c, r) = (((input.x - self.gx) / T).floor() as i32, ((input.y - self.gy) / T).floor() as i32);
            if self.at(c, r).is_some() {
                self.tiles[r as usize * self.cols + c as usize].k += 1;
                self.cur = (c as usize, r as usize);
            }
        }
    }

    fn draw(&self, g: &Pen, _cx: &Ctx, t: f32, _input: &Input) {
        g.rect(0.0, BAR_H, W, H, hex(0x070b12));
        let flowing = self.phase == Phase::Flow;
        let wet_all = self.spray > 0.0;
        let live;
        let net = if flowing || wet_all {
            &self.net
        } else {
            live = self.network();
            &live
        };
        let (cols, rows) = (self.cols as f32, self.rows as f32);
        g.panel(self.gx - 14.0, self.gy - 14.0, cols * T + 28.0, rows * T + 28.0, 16.0, hex(0x0b1018), c::LINE);
        // The main: down from the deckhead, along to the grid, through its valve.
        Self::pipe(g, vec![[RISER, BAR_H], [RISER, self.y_main], [self.gx, self.y_main]], 22.0, flowing || wet_all);
        Self::valve_glyph(g, VX, self.y_main, self.va, flowing);
        let y_end = self.centre(self.cols - 1, self.r1)[1];
        Self::pipe(g, vec![[self.gx + cols * T, y_end], [FIX[0] - 20.0, y_end]], 22.0, self.running);
        self.stall(g, t, self.running);
        for rr in 0..self.rows {
            for cc in 0..self.cols {
                let key = rr * self.cols + cc;
                let tile = &self.tiles[key];
                let [x, y] = self.centre(cc, rr);
                let d = net.dist(key);
                let in_net = d.is_some();
                let wet = (flowing && d.is_some_and(|d| (d as f32) < self.flow)) || (wet_all && in_net);
                let fill = if in_net && self.phase == Phase::Play { hex(0x121c2a) } else { hex(0x0d131c) };
                g.round(x - T / 2.0 + 3.0, y - T / 2.0 + 3.0, T - 6.0, T - 6.0, 10.0, Some(fill), None);
                let p = g.translate(x, y).rotate(tile.ang);
                let arms: Vec<(Vec<[f32; 2]>, f32)> = DIRS
                    .iter()
                    .filter(|d| tile.base & d.0 != 0)
                    .map(|d| (vec![[0.0, 0.0], [d.1 as f32 * T / 2.0, d.2 as f32 * T / 2.0]], 1.0))
                    .collect();
                let fluid = wet.then_some(c::ACCENT);
                let body = if in_net { hex(0x5a6a82) } else { hex(0x3a4658) };
                p.pipe(&arms, &PipeStyle { body, fluid, ..PipeStyle::default() });
                let hub = if in_net { hex(0x6b7c95) } else { hex(0x4a5566) };
                p.pipe_hub(0.0, 0.0, 14.0, hub, hex(0x141b27), fluid, 1.0);
            }
        }
        if self.keys && self.phase == Phase::Play {
            let [x, y] = self.centre(self.cur.0, self.cur.1);
            g.round(x - T / 2.0 + 2.0, y - T / 2.0 + 2.0, T - 4.0, T - 4.0, 10.0, None, Some((4.0, c::AMBER)));
        }
        if wet_all {
            for &(x, y, b) in &self.net.leaks {
                Self::spray_at(g, x, y, b, t);
            }
        }
    }

    fn hand(&mut self, _cx: &Ctx, t: f32) -> Input {
        if self.phase != Phase::Play || self.skip_part {
            return Input::default();
        }
        if self.hand_down {
            // Lift the finger between presses.
            self.hand_down = false;
            return Input::at(VX, self.y_main);
        }
        if self.hand_next == 0.0 {
            // A look at the grid before the first press.
            self.hand_next = t + 1.0;
        }
        if t < self.hand_next {
            return Input::default();
        }
        self.hand_next = t + 0.3;
        // Turn the run's tiles one press at a time until each shows its arms, then open the valve.
        let wrong = self.tiles.iter().position(|tile| tile.run && tile.open() != tile.want);
        let [x, y] = match wrong {
            Some(i) => self.centre(i % self.cols, i / self.cols),
            None => [VX, self.y_main],
        };
        self.hand_down = true;
        Input::hold(x, y, true)
    }

    fn guide_now(&self) -> Option<usize> {
        Some(0)
    }
}

#[cfg(test)]
mod tests {
    use crate::games::tests::{fumble_check, plays_to_end};
    use crate::kit::Input;
    use egui::Key;

    #[test]
    fn a_steady_hand_repairs_it_damaged_disabled_and_destroyed() {
        plays_to_end("shower");
    }

    #[test]
    fn opening_the_valve_on_an_open_run_soaks_you() {
        // Enter (the valve) the moment the round opens: the tiles are turned at random, never a whole run.
        fumble_check("shower", |_r| Input::default().key(Key::Enter, true));
    }
}
