//! A warp pylon's coil repair, outside on EVA (repair-minigames design 2 and 5), from `docs/mockups/repairs/pylons.js`.
//!
//! The hull and a pylon side on against the stars, the warp coil along the pylon's top with one segment dark and
//! cracked. You are the suited figure at the dorsal airlock on two tether clips. Tap a handhold within reach and you
//! swing to it with one clip open; its hook swings past the rail, and you clip it home as it crosses (tap, or Space).
//! Only one clip is ever open, and an open clip has a few seconds: miss them and the tether snaps you back to the last
//! hold (the fumble). At the damaged segment the view closes in: unbolt its plate in a star order (every bolt shows its
//! number and the next is lit; a wrong bolt slips and is lost, a fumble too), let the old segment go, and push the new
//! one up into the gap against the drift until it seats square. Each level has shorter clip times, a segment further
//! out, more bolts and a stronger drift. A disabled pylon's first step fits the new coil driver into its socket at the
//! pylon's root. In combat an EVA is refused (design 5): the game shows it and takes no input.
//! Keys: arrows choose a handhold or a bolt and push the segment, Space moves, clips and unbolts.

use std::f32::consts::{FRAC_PI_2, PI, TAU};

use crate::kit::{nearest, star_order, Ctx, Game, Input, BAR_H, H, TOUCH_R, W};
use crate::pen::{c, hex, rgba, Align, Pen};
use egui::{Color32, Key};

/// In the engine.
pub const PORTED: bool = true;

/// A fresh game.
pub fn new() -> Box<dyn Game> {
    Box::new(Pylons { stars: stars(), ..Pylons::default() })
}

/// The hull's curve, side on: centre and radius.
const HULL: [f32; 3] = [600.0, 3000.0, 2400.0];
/// The coil: its left end, top, height, a segment's length, segments.
const COIL_X0: f32 = 380.0;
const COIL_Y0: f32 = 172.0;
const COIL_H: f32 = 88.0;
const COIL_SEG: f32 = 120.0;
const COIL_N: usize = 7;
/// How far a suited arm and lanyard reach, px.
const REACH: f32 = 175.0;
/// The coil driver's socket, low on the pylon.
const SOCKET: [f32; 2] = [512.0, 548.0];
/// The zoomed view's scale at the damaged segment.
const ZOOM: f32 = 2.3;

fn hull_y(x: f32) -> f32 {
    HULL[1] - (HULL[2] * HULL[2] - (x - HULL[0]) * (x - HULL[0])).sqrt()
}

fn hatch() -> [f32; 2] {
    [232.0, hull_y(232.0)]
}

fn crate_at() -> [f32; 2] {
    [120.0, hull_y(120.0) - 26.0]
}

fn seg_x(k: usize) -> f32 {
    COIL_X0 + k as f32 * COIL_SEG
}

fn near(a: [f32; 2], b: [f32; 2], r: f32) -> bool {
    (a[0] - b[0]).hypot(a[1] - b[1]) < r
}

fn wrap_i(i: i32, n: usize) -> usize {
    if n == 0 {
        0
    } else {
        i.rem_euclid(n as i32) as usize
    }
}

/// The mockup kit's seeded stream (mulberry32), so the starfield is the mockup's.
fn mulberry(a: &mut u32) -> f32 {
    *a = a.wrapping_add(0x6d2b_79f5);
    let mut t = *a;
    t = (t ^ (t >> 15)).wrapping_mul(t | 1);
    t ^= t.wrapping_add((t ^ (t >> 7)).wrapping_mul(t | 61));
    ((t ^ (t >> 14)) as f64 / 4_294_967_296.0) as f32
}

/// The mockup kit's string hash (FNV-1a, 32 bits).
fn fnv32(s: &str) -> u32 {
    s.bytes().fold(2_166_136_261u32, |h, b| (h ^ u32::from(b)).wrapping_mul(16_777_619))
}

/// The stars behind the hull: x, y, radius, twinkle phase (the mockup's, from "pylons:stars").
fn stars() -> Vec<[f32; 4]> {
    let mut a = fnv32("pylons:stars");
    (0..170)
        .map(|_| {
            let x = mulberry(&mut a) * 1280.0;
            let y = 80.0 + mulberry(&mut a) * 640.0;
            let r = 0.6 + mulberry(&mut a) * 1.6;
            let p = mulberry(&mut a) * 6.0;
            [x, y, r, p]
        })
        .collect()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
enum Kind {
    #[default]
    Hull,
    Over,
    Pylon,
    Under,
}

#[derive(Clone, Copy, Debug, Default)]
struct Hold {
    x: f32,
    y: f32,
    kind: Kind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
enum Phase {
    #[default]
    Climb,
    Part,
    Bolts,
    Release,
    Swap,
    Done,
}

#[derive(Clone, Copy, Debug, Default)]
struct Bolt {
    x: f32,
    y: f32,
    out: bool,
    t: f32,
}

#[derive(Clone, Copy, Debug, Default)]
struct Lost {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    a: f32,
    t: f32,
}

/// The new segment: its offset from its seat, velocity, drift phase and bias, and a bump's flash.
#[derive(Clone, Copy, Debug, Default)]
struct Seg {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    ph: f32,
    bias: f32,
    bump: f32,
}

#[derive(Clone, Copy, Debug, Default)]
struct Drv {
    x: f32,
    y: f32,
    held: bool,
    set: bool,
    gx: f32,
    gy: f32,
}

/// The view: scale and the world point at the screen's (640, 400).
#[derive(Clone, Copy, Debug)]
struct Cam {
    s: f32,
    x: f32,
    y: f32,
}

impl Default for Cam {
    fn default() -> Self {
        Self { s: 1.0, x: 640.0, y: 400.0 }
    }
}

impl Cam {
    fn world(&self, x: f32, y: f32) -> [f32; 2] {
        [(x - 640.0) / self.s + self.x, (y - 400.0) / self.s + self.y]
    }
    fn screen(&self, x: f32, y: f32) -> [f32; 2] {
        [(x - self.x) * self.s + 640.0, (y - self.y) * self.s + 400.0]
    }
}

/// The game's state.
#[derive(Default)]
pub struct Pylons {
    stars: Vec<[f32; 4]>,
    phase: Phase,
    holds: Vec<Hold>,
    /// Holds on the route before the extras (the target is the last hold).
    route_n: usize,
    at: usize,
    open: bool,
    from: usize,
    to: usize,
    open_t: f32,
    open_max: f32,
    omega: f32,
    swing_t: f32,
    miss: f32,
    fig: [f32; 3],
    cam: Cam,
    dmg: usize,
    bolts: Vec<Bolt>,
    order: Vec<usize>,
    wrong: Option<(usize, f32)>,
    lost: Vec<Lost>,
    rel: f32,
    seg: Seg,
    drift: f32,
    tol: f32,
    kf: bool,
    fi: usize,
    drv: Drv,
    /// The hand: frames counted between bolt taps.
    hand_n: u32,
}

impl Pylons {
    fn seat(&self) -> [f32; 2] {
        [seg_x(self.dmg) + COIL_SEG / 2.0, COIL_Y0 + COIL_H / 2.0]
    }

    fn hold(&self, i: usize) -> [f32; 2] {
        [self.holds[i].x, self.holds[i].y]
    }

    fn reachable(&self) -> Vec<usize> {
        let a = self.hold(self.at);
        let mut r: Vec<usize> =
            (0..self.holds.len()).filter(|&i| i != self.at && near(self.hold(i), a, REACH)).collect();
        r.sort_by(|&a, &b| self.holds[a].x.total_cmp(&self.holds[b].x));
        r
    }

    /// The open clip's hook `swing` seconds into its swing: across its hold with the suit's drift.
    fn hook_at(&self, swing: f32) -> [f32; 2] {
        let ph = self.omega * swing + FRAC_PI_2;
        let h = self.hold(self.to);
        [h[0] + 32.0 * ph.sin(), h[1] + 9.0 * (2.0 * ph).sin()]
    }

    fn next_bolt(&self) -> Option<usize> {
        star_order(self.bolts.len()).get(self.order.len()).copied()
    }

    fn order_no(&self, i: usize) -> usize {
        star_order(self.bolts.len()).iter().position(|&k| k == i).map_or(0, |p| p + 1)
    }

    fn zoomed(&self) -> bool {
        matches!(self.phase, Phase::Bolts | Phase::Release | Phase::Swap)
            || (self.phase == Phase::Done && !self.drv.set)
    }

    /// The camera after one more frame's ease of `dt`.
    fn cam_after(&self, dt: f32) -> Cam {
        let tc = if self.zoomed() { Cam { s: ZOOM, x: self.seat()[0], y: 296.0 } } else { Cam::default() };
        let k = (dt * 4.0).min(1.0);
        Cam {
            s: self.cam.s + (tc.s - self.cam.s) * k,
            x: self.cam.x + (tc.x - self.cam.x) * k,
            y: self.cam.y + (tc.y - self.cam.y) * k,
        }
    }

    fn arrive(&mut self) {
        self.open = false;
        if self.at == self.holds.len() - 1 && self.phase == Phase::Climb {
            self.phase = Phase::Bolts;
            self.fi = 0;
        }
    }

    fn unbolt(&mut self, cx: &mut Ctx, i: usize) {
        if self.bolts[i].out {
            return;
        }
        if self.next_bolt() == Some(i) {
            self.bolts[i].out = true;
            self.order.push(i);
            if self.order.len() == self.bolts.len() {
                self.phase = Phase::Release;
                self.rel = 0.0;
            }
        } else {
            self.wrong = Some((i, 0.7));
            let n = self.order.len() as f32;
            self.lost.push(Lost {
                x: self.fig[0] + 10.0,
                y: self.fig[1] + 20.0,
                vx: 30.0 + 20.0 * n.sin(),
                vy: 46.0,
                a: 0.0,
                t: 0.0,
            });
            cx.fumble("A dropped bolt is lost");
        }
    }

    fn fit(&mut self, cx: &mut Ctx) {
        self.drv.set = true;
        self.drv.held = false;
        self.drv.x = SOCKET[0];
        self.drv.y = SOCKET[1];
        self.phase = Phase::Done;
        cx.step_done();
    }

    // ------------------------------------------------------------------------------------------------ drawing

    fn draw_hull(&self, g: &Pen, combat: bool) {
        let arc = Pen::arc_points(HULL[0], HULL[1], HULL[2], PI * 1.2, PI * 1.8);
        g.poly(&arc, hex(0x151d2a));
        g.path(&arc, false, 3.0, c::STEEL);
        let mut x = -40.0;
        while x < 1340.0 {
            let y = hull_y(x);
            let (nx, ny) = ((x - HULL[0]) / HULL[2], (y - HULL[1]) / HULL[2]);
            g.line(x - nx * 2.0, y - ny * 2.0, x - nx * 140.0, y - ny * 140.0, 2.0, hex(0x0d131c));
            x += 150.0;
        }
        let h = hatch();
        g.round(
            h[0] - 40.0,
            h[1] - 7.0,
            80.0,
            16.0,
            8.0,
            Some(hex(0x0b0f15)),
            Some((3.0, if combat { c::DANGER } else { c::AMBER })),
        );
        // The pylon: a swept strut from the hull to the coil, with its ribs and the warp feed.
        let top = COIL_Y0 + COIL_H;
        let pts = [[398.0, hull_y(398.0) + 6.0], [548.0, hull_y(548.0) + 6.0], [702.0, top], [584.0, top]];
        g.poly(&pts, hex(0x222b3a));
        g.path(&pts, true, 2.0, c::STEEL);
        for k in 0..5 {
            let f = 0.15 + 0.17 * k as f32;
            let (ax, ay) = (pts[0][0] + (pts[3][0] - pts[0][0]) * f, pts[0][1] + (pts[3][1] - pts[0][1]) * f);
            let (bx, by) = (pts[1][0] + (pts[2][0] - pts[1][0]) * f, pts[1][1] + (pts[2][1] - pts[1][1]) * f);
            g.line(ax, ay, bx, by, 2.0, hex(0x2c3a52));
        }
        g.line(500.0, hull_y(500.0), 660.0, top, 4.0, rgba(190, 159, 230, 0.35));
        if self.phase == Phase::Part || self.drv.set {
            let [cxx, cyy] = crate_at();
            if !self.drv.set {
                let [sx, sy] = SOCKET;
                let r =
                    [[sx - 30.0, sy - 20.0], [sx + 30.0, sy - 20.0], [sx + 30.0, sy + 20.0], [sx - 30.0, sy + 20.0]];
                let mut closed = r.to_vec();
                closed.push(r[0]);
                g.dashed(&closed, 3.0, c::AMBER, 6.0, 5.0);
                g.rect(cxx - 44.0, cyy - 26.0, 88.0, 52.0, hex(0x141b27));
                g.rect_stroke(cxx - 44.0, cyy - 26.0, 88.0, 52.0, 2.0, c::LINE);
            }
            let d = self.drv;
            g.round(d.x - 28.0, d.y - 18.0, 56.0, 36.0, 6.0, Some(hex(0x24314a)), Some((2.0, c::ACCENT)));
            g.rect(d.x - 18.0, d.y - 4.0, 36.0, 8.0, c::COPPER);
        }
    }

    fn draw_seg(g: &Pen, x: f32, y: f32, dmg: bool, t: f32) {
        g.round(
            x + 3.0,
            y,
            COIL_SEG - 6.0,
            COIL_H,
            10.0,
            Some(if dmg { hex(0x121720) } else { hex(0x1b2433) }),
            Some((2.0, hex(0x2c3a52))),
        );
        for k in 0..6 {
            let col = if dmg {
                hex(0x241d2a)
            } else {
                rgba(190, 159, 230, 0.5 + 0.25 * (t * 2.0 + k as f32 + x * 0.02).sin())
            };
            g.rect(x + 14.0 + k as f32 * 17.6, y + 8.0, 8.0, COIL_H - 16.0, col);
        }
        if dmg {
            g.path(
                &[
                    [x + 34.0, y + 4.0],
                    [x + 52.0, y + 30.0],
                    [x + 44.0, y + 46.0],
                    [x + 70.0, y + 64.0],
                    [x + 62.0, y + 84.0],
                ],
                false,
                3.0,
                c::DANGER,
            );
            if (t * 7.0).sin() > 0.3 {
                for k in 0..4 {
                    let a = k as f32 * 1.6 + t * 3.0;
                    g.line(x + 48.0, y + 36.0, x + 48.0 + 14.0 * a.cos(), y + 36.0 + 14.0 * a.sin(), 2.0, c::WARN);
                }
            }
        }
    }

    fn draw_coil(&self, g: &Pen, t: f32) {
        let (x0, y0, h, w) = (COIL_X0, COIL_Y0, COIL_H, COIL_SEG);
        let xe = x0 + COIL_N as f32 * w;
        g.round(x0 - 40.0, y0 + 6.0, 46.0, h - 12.0, 12.0, Some(hex(0x2a3446)), Some((2.0, c::STEEL)));
        let cap = [[xe, y0], [xe + 54.0, y0 + 30.0], [xe + 54.0, y0 + h - 30.0], [xe, y0 + h]];
        g.poly(&cap, hex(0x2a3446));
        g.path(&cap, true, 2.0, c::STEEL);
        for k in 0..COIL_N {
            if k != self.dmg {
                Self::draw_seg(g, seg_x(k), y0, false, t);
                continue;
            }
            let shown = matches!(self.phase, Phase::Climb | Phase::Bolts | Phase::Part | Phase::Release)
                || (self.phase == Phase::Done && self.drv.set);
            if shown {
                let (mx, my) = (seg_x(k) + w / 2.0, y0 + h / 2.0);
                let p = if self.phase == Phase::Release {
                    g.alpha(1.0 - self.rel)
                        .translate(mx + self.rel * 90.0, my - self.rel * 160.0)
                        .rotate(self.rel * 0.5)
                        .translate(-mx, -my)
                } else {
                    g.clone()
                };
                Self::draw_seg(&p, seg_x(k), y0, true, t);
                if self.phase != Phase::Release {
                    self.draw_bolts(&p, t);
                }
            }
        }
        for k in 0..=COIL_N {
            g.rect(x0 + k as f32 * w - 5.0, y0 - 8.0, 10.0, h + 16.0, c::STEEL);
        }
        if self.phase == Phase::Swap || (self.phase == Phase::Done && !self.drv.set) {
            let s = self.seat();
            let (x, y) = (s[0] + self.seg.x - w / 2.0, s[1] + self.seg.y - h / 2.0);
            let done = self.phase == Phase::Done;
            let square = self.seg.x.abs() <= self.tol;
            // The gap's mouth: brackets, solid and green when the segment is square to it.
            let col = if done || square {
                c::OK
            } else if self.seg.bump > 0.0 {
                c::DANGER
            } else {
                c::AMBER
            };
            for sx in [-1.0f32, 1.0] {
                let (bx, by) = (s[0] + sx * (w / 2.0 - 3.0), y0 + h + 4.0);
                let pts = [[bx - sx * 14.0, by + 10.0], [bx, by + 10.0], [bx, by - 8.0]];
                if square || done {
                    g.path(&pts, false, 3.0, col);
                } else {
                    g.dashed(&pts, 3.0, col, 5.0, 4.0);
                }
            }
            g.line(self.fig[0], self.fig[1] + 30.0, x + w / 2.0, y + h, 1.5, rgba(201, 211, 223, 0.5));
            Self::draw_seg(g, x, y, false, t);
        }
    }

    fn draw_bolts(&self, g: &Pen, t: f32) {
        let loose = self.phase == Phase::Bolts;
        let next = self.next_bolt();
        for (i, b) in self.bolts.iter().enumerate() {
            if b.out {
                g.disc(b.x, b.y, 5.0, hex(0x05070b));
                g.ring(b.x, b.y, 6.0, hex(0x4a5568), 1.5);
                continue;
            }
            let hx = hexagon(b.x, b.y, 6.0);
            g.poly(&hx, hex(0xc9d3df));
            g.path(&hx, true, 1.5, hex(0x2a3446));
            // Every bolt's number in the star, the next one lit (the owner's note on the fighter, the same here).
            if loose {
                g.order_badge(b.x, b.y, 6.0, self.order_no(i), next == Some(i), t);
            }
            if loose && self.kf && self.fi == i {
                dash_ring(g, b.x, b.y, 11.0, c::AMBER, 1.5, 3.0, 3.0);
            }
            if self.wrong.is_some_and(|(w, _)| w == i) {
                g.cross(b.x, b.y, 9.0, 2.5, c::DANGER);
            }
        }
        // A bolt on its way out to the pouch.
        for b in self.bolts.iter().filter(|b| b.out && b.t < 1.0) {
            let (x, y) = (b.x + (self.fig[0] - b.x) * b.t, b.y + (self.fig[1] + 20.0 - b.y) * b.t);
            g.alpha(1.0 - b.t).poly(&hexagon(x, y, 6.0), hex(0xc9d3df));
        }
    }

    fn draw_holds(&self, g: &Pen) {
        let climbing = self.phase == Phase::Climb && !self.open;
        let reach = if climbing { self.reachable() } else { Vec::new() };
        if climbing {
            let a = self.hold(self.at);
            dash_ring(g, a[0], a[1], REACH, rgba(79, 195, 247, 0.18), 2.0, 6.0, 8.0);
        }
        for (i, hd) in self.holds.iter().enumerate() {
            let top = match hd.kind {
                Kind::Under => Some(COIL_Y0 + COIL_H),
                Kind::Over => Some(COIL_Y0 - 8.0),
                _ => None,
            };
            if let Some(top) = top {
                g.line(hd.x - 9.0, hd.y, hd.x - 9.0, top, 3.0, c::STEEL);
                g.line(hd.x + 9.0, hd.y, hd.x + 9.0, top, 3.0, c::STEEL);
            }
            if hd.kind == Kind::Hull {
                g.line(hd.x - 9.0, hd.y, hd.x - 9.0, hd.y + 18.0, 3.0, c::STEEL);
                g.line(hd.x + 9.0, hd.y, hd.x + 9.0, hd.y + 18.0, 3.0, c::STEEL);
            }
            g.round(hd.x - 13.0, hd.y - 4.0, 26.0, 8.0, 4.0, Some(hex(0xc9a640)), None);
            if reach.contains(&i) {
                g.ring(hd.x, hd.y, 17.0, c::ACCENT, 2.5);
                if self.kf && reach.get(self.fi) == Some(&i) {
                    dash_ring(g, hd.x, hd.y, 25.0, c::AMBER, 2.5, 5.0, 4.0);
                }
            }
        }
    }

    fn draw_suit(&self, g: &Pen, t: f32) {
        let [x, y, fa] = self.fig;
        let open = self.open && self.phase == Phase::Climb;
        let sway = fa + 0.1 * (t * 0.9).sin() + if open { 0.18 * (self.omega * self.swing_t).sin() } else { 0.0 };
        let waist = [x - sway.sin() * 44.0, y + sway.cos() * 44.0];
        let anchor = self.hold(if open { self.from } else { self.at });
        let k = self.hook_at(self.swing_t);
        // The lanyards.
        let lan = hex(0xc9d3df);
        g.line(waist[0], waist[1], anchor[0] - 4.0, anchor[1] + 2.0, 2.0, lan);
        let end = if open { k } else { [anchor[0] + 4.0, anchor[1] + 2.0] };
        g.line(waist[0], waist[1], end[0], end[1], 2.0, lan);
        g.ring(anchor[0] - 4.0, anchor[1] + 2.0, 5.0, c::OK, 3.0);
        if !open {
            g.ring(anchor[0] + 4.0, anchor[1] + 2.0, 5.0, c::OK, 3.0);
        }
        // Side on: backpack behind, sleeves up to the gloves on the rail, the helmet's gold visor to the right.
        let p = g.translate(x, y).rotate(sway);
        let suit = hex(0xd9dee6);
        p.round(-17.0, 20.0, 12.0, 28.0, 4.0, Some(hex(0x3a4456)), None);
        for seg in [
            [[-6.0, 26.0], [-3.0, 6.0]],
            [[6.0, 26.0], [4.0, 6.0]],
            [[-4.0, 50.0], [-7.0, 72.0]],
            [[5.0, 50.0], [10.0, 70.0]],
        ] {
            p.path_round(&seg, 8.0, suit);
        }
        p.disc(-3.0, 3.0, 4.5, hex(0x4a5568));
        p.disc(4.0, 3.0, 4.5, hex(0x4a5568));
        p.round(-10.0, 20.0, 22.0, 34.0, 8.0, Some(suit), None);
        p.disc(2.0, 17.0, 11.0, suit);
        p.poly(&ellipse(7.0, 17.0, 5.0, 7.0), hex(0xd8a23a));
        p.rect(-4.0, 32.0, 12.0, 4.0, c::ACCENT);
        if open {
            // The open hook: a C, amber, ringed by the time it has left.
            let left = 1.0 - self.open_t / self.open_max;
            g.arc(k[0], k[1], 6.0, 0.6, TAU - 0.6, 3.0, if self.miss > 0.0 { c::DANGER } else { c::AMBER });
            if left > 0.0 {
                g.arc(
                    k[0],
                    k[1],
                    12.0,
                    -FRAC_PI_2,
                    -FRAC_PI_2 + TAU * left,
                    3.0,
                    if left < 0.3 { c::DANGER } else { c::AMBER },
                );
            }
        }
    }
}

/// A hexagon's corners (a bolt head).
fn hexagon(x: f32, y: f32, r: f32) -> Vec<[f32; 2]> {
    (0..6)
        .map(|k| {
            let a = PI / 6.0 + k as f32 * PI / 3.0;
            [x + r * a.cos(), y + r * a.sin()]
        })
        .collect()
}

/// An ellipse's outline (the visor).
fn ellipse(x: f32, y: f32, rx: f32, ry: f32) -> Vec<[f32; 2]> {
    (0..24)
        .map(|k| {
            let a = TAU * k as f32 / 24.0;
            [x + rx * a.cos(), y + ry * a.sin()]
        })
        .collect()
}

/// A dashed circle (the mockup's `setLineDash` ring).
#[allow(clippy::too_many_arguments)]
fn dash_ring(g: &Pen, x: f32, y: f32, r: f32, col: Color32, width: f32, dash: f32, gap: f32) {
    g.dashed(&Pen::arc_points(x, y, r, 0.0, TAU), width, col, dash, gap);
}

impl Game for Pylons {
    fn id(&self) -> &'static str {
        "pylons"
    }

    fn knobs(&self) -> &'static [&'static str] {
        &[
            "clip_window_s",
            "swing_period_s",
            "segment_out_count",
            "bolts_count",
            "drift_px_s2",
            "drift_bias_px_s2",
            "seat_tol_px",
        ]
    }

    fn step(&mut self, cx: &mut Ctx) {
        let mut r = cx.dice();
        self.open = false;
        self.at = 0;
        self.open_t = 0.0;
        self.swing_t = 0.0;
        self.miss = 0.0;
        self.kf = false;
        self.fi = 0;
        self.order.clear();
        self.wrong = None;
        self.lost.clear();
        self.rel = 0.0;
        self.cam = Cam::default();
        self.hand_n = 0;
        self.open_max = cx.knob("clip_window_s");
        self.omega = TAU / cx.knob("swing_period_s");
        self.tol = cx.knob("seat_tol_px");
        self.drift = cx.knob("drift_px_s2");
        self.dmg = (cx.knob("segment_out_count") as usize + r.int(2)).min(6);
        // The route: along the hull from the hatch, up the pylon, along the coil's underside to the damaged segment,
        // a hold every 100-125 px (always within reach), and a few holds off the route.
        let h0 = hatch();
        let way = [
            [h0[0], h0[1] - 18.0],
            [472.0, hull_y(472.0) - 20.0],
            [636.0, 290.0],
            [seg_x(self.dmg) + COIL_SEG / 2.0, 290.0],
        ];
        let mut holds = vec![Hold { x: way[0][0], y: way[0][1], kind: Kind::Hull }];
        for w in 1..way.len() {
            let ([ax, ay], [bx, by]) = (way[w - 1], way[w]);
            let len = (bx - ax).hypot(by - ay);
            let n = (len / (100.0 + 25.0 * r.f())).ceil().max(1.0) as usize;
            for k in 1..=n {
                let f = k as f32 / n as f32;
                let last = w == way.len() - 1 && k == n;
                let kind = match w {
                    1 => Kind::Hull,
                    2 => Kind::Pylon,
                    _ => Kind::Under,
                };
                let jx = if last { 0.0 } else { (r.f() - 0.5) * 18.0 };
                let jy = if last { 0.0 } else { (r.f() - 0.5) * 14.0 };
                let x = ax + (bx - ax) * f + jx;
                let mut y = ay + (by - ay) * f + jy;
                if kind == Kind::Hull {
                    y = hull_y(x) - 20.0;
                }
                if kind == Kind::Under && !last {
                    y = 290.0 + jy;
                }
                holds.push(Hold { x, y, kind });
            }
        }
        let target = holds.pop().unwrap_or_default();
        self.route_n = holds.len();
        for k in 0..4 {
            let over = k < 2;
            let x = if over { seg_x(1) + r.f() * (seg_x(self.dmg) - seg_x(1) + 60.0) } else { 560.0 + r.f() * 260.0 };
            let h = Hold {
                x,
                y: if over { COIL_Y0 - 22.0 } else { hull_y(x) - 20.0 },
                kind: if over { Kind::Over } else { Kind::Hull },
            };
            let clear = |q: &Hold| !near([q.x, q.y], [h.x, h.y], 70.0);
            if holds.iter().all(clear) && clear(&target) {
                holds.push(h);
            }
        }
        holds.push(target);
        self.holds = holds;
        self.fig = [self.holds[0].x, self.holds[0].y, PI * 0.66];
        let n = cx.knob("bolts_count") as usize;
        let (x0, y0) = (seg_x(self.dmg), COIL_Y0);
        let (l, rr, t, b) = (x0 + 16.0, x0 + COIL_SEG - 16.0, y0 + 14.0, y0 + COIL_H - 14.0);
        let (m, v) = (x0 + COIL_SEG / 2.0, y0 + COIL_H / 2.0);
        let pts: Vec<[f32; 2]> = if n == 6 {
            vec![[l, t], [m, t], [rr, t], [rr, b], [m, b], [l, b]]
        } else {
            vec![[l, t], [m, t], [rr, t], [rr, v], [rr, b], [m, b], [l, b], [l, v]]
        };
        self.bolts = pts.into_iter().map(|[x, y]| Bolt { x, y, out: false, t: 0.0 }).collect();
        let side = if r.f() < 0.5 { -1.0 } else { 1.0 };
        self.seg = Seg {
            x: side * (30.0 + 30.0 * r.f()),
            y: 150.0,
            vx: 0.0,
            vy: 0.0,
            ph: r.f() * 6.0,
            bias: side * -cx.knob("drift_bias_px_s2"),
            bump: 0.0,
        };
        let [crx, cry] = crate_at();
        self.drv = Drv { x: crx, y: cry, ..Drv::default() };
        self.phase = if cx.part { Phase::Part } else { Phase::Climb };
    }

    fn update(&mut self, cx: &mut Ctx, dt: f32, input: &Input) {
        if cx.combat {
            // Design 5: no EVA in combat.
            return;
        }
        let ease = (dt * 10.0).min(1.0);
        let fwd = [Key::ArrowRight, Key::D, Key::ArrowDown, Key::S].iter().any(|k| input.hit(*k));
        let back = [Key::ArrowLeft, Key::A, Key::ArrowUp, Key::W].iter().any(|k| input.hit(*k));
        let dir = i32::from(fwd) - i32::from(back);
        if dir != 0 || input.action_pressed {
            self.kf = true;
        }
        if input.pressed {
            self.kf = false;
        }
        if let Some((_, w)) = &mut self.wrong {
            *w -= dt;
            if *w <= 0.0 {
                self.wrong = None;
            }
        }
        self.miss = (self.miss - dt).max(0.0);
        for b in &mut self.lost {
            b.t += dt;
            b.x += b.vx * dt;
            b.y += b.vy * dt;
            b.a += dt * 5.0;
        }
        self.lost.retain(|b| b.t < 3.0);
        for b in &mut self.bolts {
            if b.out {
                b.t = (b.t + dt / 0.35).min(1.0);
            }
        }
        self.cam = self.cam_after(dt);
        let goal = self.holds[if self.open { self.to } else { self.at }];
        self.fig[0] += (goal.x - self.fig[0]) * ease;
        self.fig[1] += (goal.y - self.fig[1]) * ease;
        // Floating off the surface it holds.
        let lean = match goal.kind {
            Kind::Hull => 0.66,
            Kind::Over => 1.0,
            Kind::Pylon => 0.6,
            Kind::Under => 0.0,
        } * PI;
        self.fig[2] += (lean - self.fig[2]) * (dt * 5.0).min(1.0);

        match self.phase {
            Phase::Part => {
                let [wx, wy] = self.cam.world(input.x, input.y);
                let d = &mut self.drv;
                if input.pressed && (wx - d.x).abs() < 34.0 && (wy - d.y).abs() < TOUCH_R + 4.0 {
                    d.held = true;
                    d.gx = wx - d.x;
                    d.gy = wy - d.y;
                }
                if d.held && input.down {
                    d.x = wx - d.gx;
                    d.y = wy - d.gy;
                }
                if d.held && input.released {
                    d.held = false;
                    if near([d.x, d.y], SOCKET, 40.0) {
                        self.fit(cx);
                        return;
                    }
                }
                if !self.drv.held {
                    let [crx, cry] = crate_at();
                    self.drv.x += (crx - self.drv.x) * ease;
                    self.drv.y += (cry - self.drv.y) * ease;
                }
                if self.kf && input.action_pressed {
                    self.fit(cx);
                }
            }
            Phase::Climb if !self.open => {
                let reach = self.reachable();
                let mut go = None;
                if self.kf {
                    self.fi = wrap_i(self.fi as i32 + dir, reach.len());
                    if input.action_pressed {
                        go = reach.get(self.fi).copied();
                    }
                } else if input.pressed {
                    let [wx, wy] = self.cam.world(input.x, input.y);
                    // The nearest reachable hold within a fingertip's reach (holds are 70 px or more apart).
                    let pts: Vec<Option<[f32; 2]>> = reach.iter().map(|&i| Some(self.hold(i))).collect();
                    go = nearest(&pts, wx, wy, TOUCH_R + 10.0).map(|k| reach[k]);
                }
                if let Some(go) = go {
                    self.open = true;
                    self.from = self.at;
                    self.to = go;
                    self.open_t = 0.0;
                    self.swing_t = 0.0;
                    self.fi = 0;
                }
            }
            Phase::Climb => {
                // One clip open: clip it as its hook crosses the hold, before the tether takes up.
                self.open_t += dt;
                self.swing_t += dt;
                if input.pressed || input.action_pressed {
                    if near(self.hook_at(self.swing_t), self.hold(self.to), 13.0) {
                        self.at = self.to;
                        self.arrive();
                        return;
                    }
                    self.miss = 0.25;
                }
                if self.open_t >= self.open_max {
                    self.open = false;
                    self.at = self.from;
                    cx.fumble("The tether snaps you back");
                }
            }
            Phase::Bolts => {
                if self.kf {
                    self.fi = wrap_i(self.fi as i32 + dir, self.bolts.len());
                    if input.action_pressed {
                        self.unbolt(cx, self.fi);
                    }
                } else if input.pressed {
                    let [wx, wy] = self.cam.world(input.x, input.y);
                    // The bolts are 30 px or more apart in the world, seen at 2.3x: the nearest within 20 world px.
                    let pts: Vec<Option<[f32; 2]>> =
                        self.bolts.iter().map(|b| (!b.out).then_some([b.x, b.y])).collect();
                    if let Some(i) = nearest(&pts, wx, wy, 20.0) {
                        self.unbolt(cx, i);
                    }
                }
            }
            Phase::Release => {
                self.rel += dt / 1.2;
                if self.rel >= 1.0 {
                    self.rel = 1.0;
                    self.phase = Phase::Swap;
                }
            }
            Phase::Swap => {
                // The new segment: offset from its seat, pushed by the drift, the pointer's pull or the stick.
                let tol = self.tol;
                let sg = &mut self.seg;
                let mut ax = self.drift * sg.ph.sin() + sg.bias;
                let mut ay = 0.0;
                sg.ph += dt * 0.8;
                if input.down && !self.kf {
                    let [wx, wy] = self.cam.world(input.x, input.y);
                    let s = [seg_x(self.dmg) + COIL_SEG / 2.0, COIL_Y0 + COIL_H / 2.0];
                    ax += (wx - (s[0] + sg.x)) * 6.0 - sg.vx * 3.0;
                    ay += (wy - (s[1] + sg.y)) * 6.0 - sg.vy * 3.0;
                } else if input.stick != [0.0, 0.0] {
                    ax += input.stick[0] * 70.0 - sg.vx * 1.5;
                    ay += input.stick[1] * 70.0 - sg.vy * 1.5;
                }
                sg.vx += ax * dt;
                sg.vy += ay * dt;
                sg.vx *= 1.0 - 0.3 * dt;
                sg.vy *= 1.0 - 0.3 * dt;
                let prev_y = sg.y;
                sg.x += sg.vx * dt;
                sg.y += sg.vy * dt;
                sg.bump = (sg.bump - dt).max(0.0);
                if sg.y < COIL_H && prev_y >= COIL_H && sg.x.abs() > tol {
                    // Caught on the flange.
                    sg.y = COIL_H;
                    sg.vy = sg.vy.abs() * 0.4 + 25.0;
                    sg.vx *= 0.5;
                    sg.bump = 0.4;
                }
                if sg.y < COIL_H {
                    sg.x = sg.x.clamp(-tol, tol);
                }
                sg.y = sg.y.min(260.0);
                if sg.y <= 0.0 {
                    sg.x = 0.0;
                    sg.y = 0.0;
                    self.phase = Phase::Done;
                    cx.step_done();
                }
            }
            Phase::Done => {}
        }
    }

    fn draw(&self, g: &Pen, cx: &Ctx, t: f32, _input: &Input) {
        g.rect(0.0, BAR_H, W, H, hex(0x020409));
        for [x, y, r, p] in &self.stars {
            g.disc(*x, *y, *r, rgba(232, 238, 246, 0.35 + 0.3 * (t * 0.8 + p).sin()));
        }
        let w = g.translate(640.0, 400.0).scale(self.cam.s, self.cam.s).translate(-self.cam.x, -self.cam.y);
        self.draw_hull(&w, cx.combat);
        self.draw_coil(&w, t);
        self.draw_holds(&w);
        self.draw_suit(&w, t);
        for b in &self.lost {
            w.translate(b.x, b.y).rotate(b.a).alpha(1.0 - b.t / 3.0).poly(&hexagon(0.0, 0.0, 6.0), hex(0xc9d3df));
        }
        if cx.combat {
            g.rect(0.0, BAR_H, W, H - BAR_H, rgba(4, 6, 10, 0.6));
            g.panel(250.0, 330.0, 780.0, 120.0, 20.0, hex(0x2a0c10), c::DANGER);
            g.ring(330.0, 390.0, 34.0, c::DANGER, 7.0);
            g.line(306.0, 366.0, 354.0, 414.0, 7.0, c::DANGER);
            g.text("EVA REFUSED: IN COMBAT", 680.0, 392.0, 46.0, c::DANGER, Align::Center);
        }
    }

    fn hand(&mut self, cx: &Ctx, _t: f32) -> Input {
        if cx.combat {
            return Input::default();
        }
        // The kit plays at 60 frames a second; the camera the update will use is one more ease on.
        let dt = 1.0 / 60.0;
        let cam = self.cam_after(dt);
        match self.phase {
            Phase::Part => {
                // Take the driver from the crate and carry it to the socket in steady steps, then let go.
                let d = self.drv;
                let [dx, dy] = cam.screen(d.x, d.y);
                if !d.held {
                    return Input::hold(dx, dy, true);
                }
                let [sx, sy] = cam.screen(SOCKET[0], SOCKET[1]);
                let dist = (sx - dx).hypot(sy - dy);
                if dist > 2.0 {
                    let k = (12.0 / dist).min(1.0);
                    return Input::hold(dx + (sx - dx) * k, dy + (sy - dy) * k, false);
                }
                Input::release(sx, sy)
            }
            Phase::Climb if !self.open => {
                // The hold furthest along the route in reach, or the segment's hold itself.
                let reach = self.reachable();
                let last = self.holds.len() - 1;
                let pick = if reach.contains(&last) {
                    Some(last)
                } else {
                    reach.iter().copied().filter(|&i| i < self.route_n).max()
                };
                match pick {
                    Some(i) => {
                        let [x, y] = cam.screen(self.holds[i].x, self.holds[i].y);
                        Input::hold(x, y, true)
                    }
                    None => Input::default(),
                }
            }
            Phase::Climb => {
                // Clip as the hook crosses the hold (where it will be after this frame).
                let k = self.hook_at(self.swing_t + dt);
                if near(k, self.hold(self.to), 5.0) {
                    let [x, y] = cam.screen(k[0], k[1]);
                    Input::hold(x, y, true)
                } else {
                    Input::default()
                }
            }
            Phase::Bolts => {
                // Once the view has closed in, tap the lit bolt every few frames.
                self.hand_n += 1;
                let settled = (cam.s - ZOOM).abs() < 0.05;
                match self.next_bolt() {
                    Some(i) if settled && self.hand_n % 8 == 0 => {
                        let [x, y] = cam.screen(self.bolts[i].x, self.bolts[i].y);
                        Input::hold(x, y, true)
                    }
                    _ => Input::default(),
                }
            }
            Phase::Swap => {
                // Hold the pointer where its pull cancels the drift and steers the segment square, then up home.
                let sg = self.seg;
                let s = self.seat();
                let drift = self.drift * sg.ph.sin() + sg.bias;
                let ax = -8.0 * sg.x - 5.0 * sg.vx;
                let square = sg.x.abs() < 1.0 && sg.vx.abs() < 3.0;
                let goal = if square || sg.y < COIL_H + 2.0 { -30.0 } else { 120.0 };
                let ay = (-6.0 * (sg.y - goal) - 5.0 * sg.vy).clamp(-300.0, 300.0);
                let wx = s[0] + sg.x + (ax - drift + sg.vx * 3.0) / 6.0;
                let wy = s[1] + sg.y + (ay + sg.vy * 3.0) / 6.0;
                let [x, y] = cam.screen(wx, wy);
                Input::hold(x, y, false)
            }
            Phase::Release | Phase::Done => Input::default(),
        }
    }

    fn guide_now(&self) -> Option<usize> {
        match self.phase {
            Phase::Climb => Some(if self.open { 1 } else { 0 }),
            Phase::Bolts => Some(2),
            Phase::Release | Phase::Swap => Some(3),
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
        plays_to_end("pylons");
    }

    #[test]
    fn a_clip_left_open_snaps_the_tether_back() {
        // Space once opens a clip to the first hold in reach; leaving it open runs its time out.
        let mut n = 0;
        fumble_check("pylons", move |_r| {
            n += 1;
            if n == 1 {
                Input::default().key(egui::Key::Space, true)
            } else {
                Input::default()
            }
        });
    }
}
