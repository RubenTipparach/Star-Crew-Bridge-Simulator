//! A Swift fighter's avionics repair on its hangar cradle (repair-minigames design 2), from
//! `docs/mockups/repairs/fighter.js`.
//!
//! The Swift sits on its cradle at the left with its dorsal avionics bay marked; the bay fills the rest of the screen.
//! Lift the access panel off (drag it away), plug each avionics lead into the socket of the same shape and stripe, then
//! torque the panel's fasteners back in a star order: each one across from the last, working round. Every fastener
//! shows its number in the order and the next one is lit, so the order is known before the first is picked. A plug
//! forced into the wrong socket sparks (5 HP) and comes back bent: the fumble. A fastener out of order is refused with
//! a red cross, never a fumble. Each level has more leads, fewer shapes to tell them apart by (the stripe decides), and
//! more fasteners. A disabled fighter's first step fits the new avionics unit: drag it from the crate into the empty
//! rack slot. Keys: arrows choose, Space lifts, picks up, places and turns.

use std::f32::consts::{FRAC_PI_2, PI, TAU};

use egui::{Color32, Key};

use crate::kit::{nearest, star_order, Ctx, Game, Input, BAR_H, H, TOUCH_R, W};
use crate::pen::{c, hex, rgba, Align, Pen};

/// In the engine.
pub const PORTED: bool = true;

/// A fresh game.
pub fn new() -> Box<dyn Game> {
    Box::new(Fighter::default())
}

/// The avionics bay's opening, close up: x, y, w, h.
const BAY: [f32; 4] = [520.0, 140.0, 560.0, 290.0];
/// The access panel over it.
const PLATE: [f32; 4] = [490.0, 112.0, 620.0, 346.0];
/// The close-up's frame.
const FRAME: [f32; 4] = [420.0, 92.0, 836.0, 608.0];
/// How far aside the lifted panel sits, px.
const REST: f32 = 610.0;
/// Where the leads come up out of the bay floor.
const GROMMET: [f32; 2] = [800.0, 418.0];
/// The part step's empty rack slot.
const SLOT: [f32; 2] = [800.0, 290.0];
/// The spare unit's crate.
const CRATE: [f32; 2] = [1170.0, 590.0];
/// The fighter's picture, nose up.
const SWIFT: [f32; 2] = [214.0, 380.0];
/// The panel's lift: dragged further than this, px, it comes off.
const LIFT_PX: f32 = 110.0;
/// The steady hand's pointer speed, px a frame.
const HAND_PX: f32 = 14.0;

/// A lead's and a socket's keyed shape.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Shape {
    Circle,
    Square,
    Triangle,
    Diamond,
}

const SHAPES: [Shape; 4] = [Shape::Circle, Shape::Square, Shape::Triangle, Shape::Diamond];
/// Stripe i is drawn as i + 1 bars, so it reads without colour.
const STRIPES: [Color32; 3] = [c::ACCENT, c::AMBER, c::LILAC];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Phase {
    /// The part step: the new unit into its slot.
    Part,
    /// The access panel on.
    #[default]
    Panel,
    /// The leads to their sockets.
    Plugs,
    /// The panel going back on.
    Close,
    /// The fasteners in the star order.
    Torque,
    /// Played.
    Done,
}

#[derive(Clone, Debug, Default)]
struct Plate {
    dx: f32,
    dy: f32,
    tx: f32,
    ty: f32,
    held: bool,
    gx: f32,
    gy: f32,
}

#[derive(Clone, Debug)]
struct Plug {
    shape: Shape,
    stripe: usize,
    hx: f32,
    hy: f32,
    x: f32,
    y: f32,
    held: bool,
    seat: Option<usize>,
    bent: bool,
    gx: f32,
    gy: f32,
}

#[derive(Clone, Debug)]
struct Socket {
    shape: Shape,
    stripe: usize,
    x: f32,
    y: f32,
    plug: Option<usize>,
}

#[derive(Clone, Debug)]
struct Fastener {
    x: f32,
    y: f32,
    on: bool,
    t: f32,
}

#[derive(Clone, Debug, Default)]
struct Unit {
    x: f32,
    y: f32,
    held: bool,
    set: bool,
    gx: f32,
    gy: f32,
}

/// The game's state.
#[derive(Default)]
pub struct Fighter {
    phase: Phase,
    panel: Plate,
    plugs: Vec<Plug>,
    sockets: Vec<Socket>,
    fast: Vec<Fastener>,
    order: Vec<usize>,
    /// A fastener refused, and how long its cross shows.
    wrong: Option<(usize, f32)>,
    unit: Unit,
    /// The keys are in use (the focus ring shows).
    kf: bool,
    /// The keys' choice.
    fi: usize,
    /// The plug carried by the keys.
    carry: Option<usize>,
    /// The plug held by the pointer.
    held: Option<usize>,
    /// A spark: where, and how long it has left.
    spark: Option<(f32, f32, f32)>,
    /// The hand: pressing.
    hand_down: bool,
    /// The hand: where its pointer is.
    hand_at: [f32; 2],
    /// The hand: where it is carrying to.
    hand_to: [f32; 2],
    /// The hand: frames to wait.
    hand_wait: u32,
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

/// The fastener seats round the panel's rim, in ring order (clockwise from the top left).
fn ring_pts(n: usize) -> Vec<[f32; 2]> {
    let i = 22.0;
    let (l, r, t, b) = (PLATE[0] + i, PLATE[0] + PLATE[2] - i, PLATE[1] + i, PLATE[1] + PLATE[3] - i);
    let (m, v) = (PLATE[0] + PLATE[2] / 2.0, PLATE[1] + PLATE[3] / 2.0);
    if n == 6 {
        vec![[l, t], [m, t], [r, t], [r, b], [m, b], [l, b]]
    } else {
        vec![[l, t], [m, t], [r, t], [r, v], [r, b], [m, b], [l, b], [l, v]]
    }
}

/// The fastener that comes next: the kit's star order from the first (one order, shown at rest).
fn next_in_star(done: &[usize], n: usize) -> Option<usize> {
    star_order(n).get(done.len()).copied()
}

/// Fastener i's number in the order, from 1.
fn order_no(i: usize, n: usize) -> usize {
    star_order(n).iter().position(|&k| k == i).map_or(0, |p| p + 1)
}

/// A keyed shape's outline.
fn shape_pts(kind: Shape, x: f32, y: f32, r: f32) -> Vec<[f32; 2]> {
    match kind {
        Shape::Circle => Pen::arc_points(x, y, r, 0.0, TAU),
        Shape::Square => {
            let s = r * 0.85;
            vec![[x - s, y - s], [x + s, y - s], [x + s, y + s], [x - s, y + s]]
        }
        Shape::Triangle => vec![[x, y - r], [x + r, y + r * 0.8], [x - r, y + r * 0.8]],
        Shape::Diamond => vec![[x, y - r * 1.1], [x + r, y], [x, y + r * 1.1], [x - r, y]],
    }
}

/// A hexagon's outline (a fastener's head).
fn hex_pts(x: f32, y: f32, r: f32) -> Vec<[f32; 2]> {
    (0..6)
        .map(|k| {
            let a = PI / 6.0 + k as f32 * PI / 3.0;
            [x + r * a.cos(), y + r * a.sin()]
        })
        .collect()
}

/// An ellipse's outline (the canopy).
fn ellipse_pts(x: f32, y: f32, rx: f32, ry: f32) -> Vec<[f32; 2]> {
    (0..40)
        .map(|k| {
            let a = TAU * k as f32 / 40.0;
            [x + rx * a.cos(), y + ry * a.sin()]
        })
        .collect()
}

/// A quadratic curve's points (canvas's `quadraticCurveTo`).
fn quad_pts(p0: [f32; 2], c1: [f32; 2], p1: [f32; 2]) -> Vec<[f32; 2]> {
    (0..=20)
        .map(|k| {
            let t = k as f32 / 20.0;
            let u = 1.0 - t;
            [u * u * p0[0] + 2.0 * u * t * c1[0] + t * t * p1[0], u * u * p0[1] + 2.0 * u * t * c1[1] + t * t * p1[1]]
        })
        .collect()
}

/// A ring drawn dashed 8 on, 6 off (canvas's `setLineDash([8, 6])` on an arc): the keys' focus.
fn dashed_ring(g: &Pen, x: f32, y: f32, r: f32, col: Color32, w: f32) {
    g.dashed(&Pen::arc_points(x, y, r, 0.0, TAU), w, col, 8.0, 6.0);
}

/// A filled outline with a stroke round it.
fn fill_stroke(g: &Pen, pts: &[[f32; 2]], fill: Color32, stroke: Option<(f32, Color32)>) {
    g.poly(pts, fill);
    if let Some((w, col)) = stroke {
        g.path(pts, true, w, col);
    }
}

impl Fighter {
    fn free(&self) -> Vec<usize> {
        (0..self.plugs.len()).filter(|&i| self.plugs[i].seat.is_none()).collect()
    }

    fn open(&self) -> Vec<usize> {
        (0..self.sockets.len()).filter(|&i| self.sockets[i].plug.is_none()).collect()
    }

    fn lift(&mut self) {
        self.phase = Phase::Plugs;
        self.panel.tx = REST;
        self.panel.ty = 0.0;
        self.fi = 0;
    }

    fn place(&mut self, cx: &mut Ctx, p: usize, s: usize) {
        let (pl, so) = (&self.plugs[p], &self.sockets[s]);
        if pl.shape == so.shape && pl.stripe == so.stripe {
            self.plugs[p].seat = Some(s);
            self.sockets[s].plug = Some(p);
            if self.plugs.iter().all(|q| q.seat.is_some()) {
                self.phase = Phase::Close;
                self.panel.tx = 0.0;
                self.panel.ty = 0.0;
            }
            return;
        }
        self.plugs[p].bent = true;
        self.spark = Some((so.x, so.y, 0.5));
        cx.fumble("Spark: 5 HP, a bent plug");
    }

    fn turn(&mut self, cx: &mut Ctx, i: usize) {
        if self.fast[i].on {
            return;
        }
        if next_in_star(&self.order, self.fast.len()) == Some(i) {
            self.fast[i].on = true;
            self.order.push(i);
            if self.order.len() == self.fast.len() {
                self.phase = Phase::Done;
                cx.step_done();
            }
        } else {
            self.wrong = Some((i, 0.7));
            cx.say("Out of order");
        }
    }

    fn seat_unit(&mut self, cx: &mut Ctx) {
        self.unit.set = true;
        self.unit.held = false;
        self.unit.x = SLOT[0];
        self.unit.y = SLOT[1];
        self.phase = Phase::Done;
        cx.step_done();
    }

    /// The hand carries something from `from` to `to`: press on it, move at the hand's speed, let go there.
    fn carry_to(&mut self, from: [f32; 2], to: [f32; 2]) -> Input {
        if !self.hand_down {
            self.hand_down = true;
            self.hand_at = from;
            self.hand_to = to;
            return Input::hold(from[0], from[1], true);
        }
        let [tx, ty] = self.hand_to;
        let d = (tx - self.hand_at[0]).hypot(ty - self.hand_at[1]);
        if d > 1.0 {
            let k = (HAND_PX / d).min(1.0);
            self.hand_at = [self.hand_at[0] + (tx - self.hand_at[0]) * k, self.hand_at[1] + (ty - self.hand_at[1]) * k];
            return Input::hold(self.hand_at[0], self.hand_at[1], false);
        }
        self.hand_down = false;
        self.hand_wait = 12;
        Input::release(tx, ty)
    }

    fn draw_hangar(&self, g: &Pen) {
        g.panel(24.0, 92.0, 380.0, 608.0, 18.0, hex(0x0a0f17), c::LINE);
        let mut y = 132.0;
        while y < 700.0 {
            g.line(26.0, y, 402.0, y, 2.0, hex(0x111925));
            y += 60.0;
        }
        let mut x = 64.0;
        while x < 404.0 {
            g.line(x, 94.0, x, 698.0, 2.0, hex(0x111925));
            x += 60.0;
        }
        let s = g.translate(SWIFT[0], SWIFT[1]);
        // The cradle: a centre rail and two arms with their pads.
        s.rect(-8.0, -236.0, 16.0, 480.0, hex(0x1e2533));
        for (ay, aw) in [(-104.0f32, 84.0f32), (178.0, 96.0)] {
            s.round(-aw, ay - 9.0, aw * 2.0, 18.0, 6.0, Some(hex(0x3a4456)), None);
            for k in [-1.0f32, 1.0] {
                s.rect(k * aw - if k > 0.0 { 14.0 } else { 0.0 }, ay - 13.0, 14.0, 26.0, c::AMBER);
            }
        }
        for k in [-1.0f32, 1.0] {
            let wing = [[k * 30.0, -40.0], [k * 150.0, 112.0], [k * 150.0, 142.0], [k * 34.0, 150.0]];
            fill_stroke(&s, &wing, hex(0x222b3a), Some((2.0, c::STEEL)));
            let canard = [[k * 22.0, -150.0], [k * 62.0, -118.0], [k * 62.0, -106.0], [k * 26.0, -110.0]];
            fill_stroke(&s, &canard, hex(0x222b3a), Some((2.0, c::STEEL)));
            s.rect(if k > 0.0 { 136.0 } else { -150.0 }, 116.0, 14.0, 22.0, c::AMBER);
        }
        let body = [
            [0.0, -250.0],
            [14.0, -214.0],
            [24.0, -150.0],
            [30.0, -60.0],
            [34.0, 60.0],
            [36.0, 150.0],
            [30.0, 200.0],
            [-30.0, 200.0],
            [-36.0, 150.0],
            [-34.0, 60.0],
            [-30.0, -60.0],
            [-24.0, -150.0],
            [-14.0, -214.0],
        ];
        fill_stroke(&s, &body, hex(0x2a3446), Some((2.0, c::STEEL)));
        s.rect(-26.0, 200.0, 20.0, 18.0, hex(0x121821));
        s.rect(6.0, 200.0, 20.0, 18.0, hex(0x121821));
        s.line(0.0, -105.0, 0.0, 190.0, 2.0, hex(0x3a4558));
        fill_stroke(&s, &ellipse_pts(0.0, -150.0, 11.0, 36.0), hex(0x1d3b52), Some((2.0, c::ACCENT)));
        s.rect(-18.0, -70.0, 36.0, 80.0, rgba(242, 160, 70, 0.25));
        s.rect_stroke(-18.0, -70.0, 36.0, 80.0, 3.0, c::AMBER);
        g.text("SWIFT 1", SWIFT[0], 668.0, 18.0, c::DIM, Align::Center);
        // The call out from the bay to the close up.
        let call = rgba(242, 160, 70, 0.3);
        g.line(SWIFT[0] + 18.0, SWIFT[1] - 70.0, FRAME[0], FRAME[1] + 20.0, 2.0, call);
        g.line(SWIFT[0] + 18.0, SWIFT[1] + 10.0, FRAME[0], FRAME[1] + FRAME[3] - 20.0, 2.0, call);
    }

    /// The bay's recess, its rails and (on a normal step) the avionics units with their sockets.
    fn draw_bay(&self, g: &Pen) {
        let [bx, by, bw, bh] = BAY;
        g.rect(bx, by, bw, bh, hex(0x05080d));
        g.rect_stroke(bx + 4.0, by + 4.0, bw - 8.0, bh - 8.0, 8.0, hex(0x000000));
        g.rect_stroke(bx, by, bw, bh, 2.0, hex(0x2a3446));
        g.rect(bx + 8.0, by + 18.0, bw - 16.0, 8.0, hex(0x1a212d));
        g.rect(bx + 8.0, by + bh - 26.0, bw - 16.0, 8.0, hex(0x1a212d));
        if self.phase != Phase::Part && !self.sockets.is_empty() {
            let uw = bw / self.sockets.len() as f32 - 18.0;
            for s in &self.sockets {
                g.round(s.x - uw / 2.0, 166.0, uw, 236.0, 6.0, Some(hex(0x1b2433)), Some((2.0, hex(0x2c3a52))));
                for k in 0..4 {
                    g.rect(s.x - uw / 2.0 + 12.0, 366.0 + k as f32 * 8.0, uw - 24.0, 3.0, hex(0x121821));
                }
                fill_stroke(g, &shape_pts(s.shape, s.x, s.y, 20.0), hex(0x020306), Some((2.0, hex(0x8a96a8))));
                for k in 0..=s.stripe {
                    g.rect(s.x - 22.0, 320.0 + k as f32 * 9.0, 44.0, 5.0, STRIPES[s.stripe]);
                }
            }
        }
        g.disc(GROMMET[0], GROMMET[1], 14.0, hex(0x0e1219));
        g.ring(GROMMET[0], GROMMET[1], 14.0, hex(0x3a4456), 3.0);
    }

    fn draw_cable(&self, g: &Pen, p: &Plug) {
        let (ex, ey) = (p.x, p.y + 38.0);
        let ctrl = [(GROMMET[0] + ex) / 2.0, GROMMET[1].max(ey) + if p.seat.is_some() { 24.0 } else { 50.0 }];
        let pts = quad_pts(GROMMET, ctrl, [ex, ey]);
        g.path(&pts, false, 11.0, hex(0x0e1219));
        g.path(&pts, false, 5.0, hex(0x3a4456));
    }

    fn draw_plug(&self, g: &Pen, p: &Plug) {
        let (x, y) = (p.x, p.y);
        g.round(x - 30.0, y - 38.0, 60.0, 76.0, 10.0, Some(hex(0x2a3446)), Some((2.0, c::STEEL)));
        g.poly(&shape_pts(p.shape, x, y - 14.0, 15.0), hex(0xc9d3df));
        for k in 0..=p.stripe {
            g.rect(x - 22.0, y + 10.0 + k as f32 * 8.0, 44.0, 5.0, STRIPES[p.stripe]);
        }
        if p.bent {
            g.path(&[[x - 10.0, y - 38.0], [x - 16.0, y - 48.0], [x - 6.0, y - 56.0]], false, 3.0, c::DANGER);
            g.line(x + 8.0, y - 38.0, x + 16.0, y - 46.0, 3.0, c::DANGER);
        }
        if p.seat.is_some() {
            g.rect(x - 14.0, y - 46.0, 28.0, 7.0, c::OK);
        }
    }

    /// The access panel, wherever it is: seated, held, or set aside at the frame's edge.
    fn draw_plate(&self, g: &Pen, t: f32) {
        let [px, py, pw, ph] = PLATE;
        let (ox, oy) = (self.panel.dx, self.panel.dy);
        let (x, y) = (px + ox, py + oy);
        g.round(x + 8.0, y + 10.0, pw, ph, 14.0, Some(rgba(0, 0, 0, 0.35)), None);
        g.round(x, y, pw, ph, 14.0, Some(hex(0x2b3546)), Some((3.0, c::STEEL)));
        g.rect_stroke(x + 44.0, y + 44.0, pw - 88.0, ph - 88.0, 2.0, hex(0x3a4558));
        g.round(
            x + pw / 2.0 - 70.0,
            y + ph / 2.0 - 14.0,
            140.0,
            28.0,
            14.0,
            Some(hex(0x1a212d)),
            Some((2.0, hex(0x4a5568))),
        );
        g.text("AV 2", x + 70.0, y + 70.0, 22.0, hex(0x4a5568), Align::Left);
        let loose = matches!(self.phase, Phase::Torque | Phase::Done | Phase::Close);
        let n = self.fast.len();
        let next = next_in_star(&self.order, n);
        for (i, f) in self.fast.iter().enumerate() {
            let (fx, fy) = (f.x + ox, f.y + oy);
            if !loose {
                g.disc(fx, fy, 10.0, hex(0x0b0f15));
                g.ring(fx, fy, 10.0, hex(0x4a5568), 2.0);
                continue;
            }
            let head = hex_pts(fx, fy, 14.0);
            if f.on {
                g.poly(&head, hex(0xc9d3df));
                g.arc(fx, fy, 21.0, -FRAC_PI_2, -FRAC_PI_2 + TAU * f.t, 4.0, c::OK);
            } else {
                fill_stroke(g, &head, hex(0x0b0f15), Some((2.0, hex(0xc9d3df))));
            }
            g.line(fx - 6.0, fy, fx + 6.0, fy, 3.0, if f.on { hex(0x2a3446) } else { hex(0xc9d3df) });
            // Every fastener's number in the star, the next one lit (owner: tell me the next screw before I pick one).
            if self.phase == Phase::Torque && !f.on {
                g.order_badge(fx, fy, 14.0, order_no(i, n), next == Some(i), t);
            }
            if self.wrong.is_some_and(|(w, _)| w == i) {
                g.cross(fx, fy, 18.0, 5.0, c::DANGER);
            }
        }
    }

    /// The part step: the rack with one unit missing.
    fn draw_rack(&self, g: &Pen) {
        for i in 0..5 {
            let cx = BAY[0] + (i as f32 + 0.5) * BAY[2] / 5.0;
            if i == 2 {
                if !self.unit.set {
                    let r = [
                        [cx - 46.0, 200.0],
                        [cx + 46.0, 200.0],
                        [cx + 46.0, 380.0],
                        [cx - 46.0, 380.0],
                        [cx - 46.0, 200.0],
                    ];
                    g.dashed(&r, 3.0, c::AMBER, 10.0, 7.0);
                }
                continue;
            }
            g.round(cx - 46.0, 200.0, 92.0, 180.0, 6.0, Some(hex(0x1b2433)), Some((2.0, hex(0x2c3a52))));
            for k in 0..4 {
                g.rect(cx - 34.0, 340.0 + k as f32 * 8.0, 68.0, 3.0, hex(0x121821));
            }
            g.disc(cx, 230.0, 5.0, hex(0x2c3a52));
        }
    }

    fn draw_crate(&self, g: &Pen) {
        g.panel(CRATE[0] - 62.0, CRATE[1] - 104.0, 124.0, 208.0, 12.0, hex(0x141b27), c::LINE);
        let (x, y) = (self.unit.x, self.unit.y);
        g.round(x - 46.0, y - 90.0, 92.0, 180.0, 6.0, Some(hex(0x24314a)), Some((2.0, c::ACCENT)));
        g.rect(x - 30.0, y - 82.0, 60.0, 8.0, c::COPPER);
        for k in 0..4 {
            g.rect(x - 34.0, y + 50.0 + k as f32 * 8.0, 68.0, 3.0, hex(0x121821));
        }
        g.disc(x, y - 60.0, 5.0, if self.unit.set { c::OK } else { hex(0x2c3a52) });
    }
}

impl Game for Fighter {
    fn id(&self) -> &'static str {
        "fighter"
    }

    fn knobs(&self) -> &'static [&'static str] {
        &["leads_count", "shapes_count", "fasteners_count"]
    }

    fn step(&mut self, cx: &mut Ctx) {
        let mut r = cx.dice();
        self.kf = false;
        self.fi = 0;
        self.carry = None;
        self.held = None;
        self.wrong = None;
        self.spark = None;
        self.hand_down = false;
        self.hand_wait = 0;
        self.panel = Plate::default();
        self.plugs.clear();
        self.sockets.clear();
        self.order.clear();
        let nf = cx.knob("fasteners_count").round() as usize;
        self.fast = ring_pts(nf).into_iter().map(|[x, y]| Fastener { x, y, on: false, t: 0.0 }).collect();
        if cx.part {
            self.phase = Phase::Part;
            self.panel.dx = REST;
            self.panel.tx = REST;
            self.unit = Unit { x: CRATE[0], y: CRATE[1], ..Unit::default() };
            return;
        }
        self.phase = Phase::Panel;
        // More leads each level, told apart by fewer shapes, so the stripe has to be read.
        let n = (cx.knob("leads_count").round() as usize).clamp(2, 12);
        let kinds = (cx.knob("shapes_count").round() as usize).clamp(1, SHAPES.len());
        let mut shapes = SHAPES.to_vec();
        r.shuffle(&mut shapes);
        shapes.truncate(kinds);
        let combos: Vec<(Shape, usize)> = if n <= kinds {
            shapes.iter().take(n).map(|&s| (s, r.int(3))).collect()
        } else {
            let mut all: Vec<(Shape, usize)> = shapes.iter().flat_map(|&s| (0..3).map(move |k| (s, k))).collect();
            r.shuffle(&mut all);
            all.truncate(n);
            all
        };
        let n = combos.len();
        self.sockets = combos
            .iter()
            .enumerate()
            .map(|(i, &(shape, stripe))| Socket {
                shape,
                stripe,
                x: BAY[0] + (i as f32 + 0.5) * BAY[2] / n as f32,
                y: 250.0,
                plug: None,
            })
            .collect();
        let mut homes: Vec<usize> = (0..n).collect();
        r.shuffle(&mut homes);
        self.plugs = combos
            .iter()
            .enumerate()
            .map(|(i, &(shape, stripe))| Plug {
                shape,
                stripe,
                hx: 590.0 + homes[i] as f32 * (420.0 / (n as f32 - 1.0).max(1.0)),
                hy: 590.0,
                x: GROMMET[0],
                y: GROMMET[1] + 40.0,
                held: false,
                seat: None,
                bent: false,
                gx: 0.0,
                gy: 0.0,
            })
            .collect();
    }

    fn update(&mut self, cx: &mut Ctx, dt: f32, input: &Input) {
        let ease = (dt * 12.0).min(1.0);
        if let Some((_, t)) = &mut self.wrong {
            *t -= dt;
        }
        if self.wrong.is_some_and(|(_, t)| t <= 0.0) {
            self.wrong = None;
        }
        if let Some((_, _, t)) = &mut self.spark {
            *t -= dt;
        }
        if self.spark.is_some_and(|(_, _, t)| t <= 0.0) {
            self.spark = None;
        }
        let fwd = [Key::ArrowRight, Key::D, Key::ArrowDown, Key::S].iter().any(|k| input.hit(*k));
        let back = [Key::ArrowLeft, Key::A, Key::ArrowUp, Key::W].iter().any(|k| input.hit(*k));
        let dir = i32::from(fwd) - i32::from(back);
        if dir != 0 || input.action_pressed {
            self.kf = true;
        }
        if input.pressed {
            self.kf = false;
            if let Some(p) = self.carry.take() {
                self.plugs[p].held = false;
            }
        }
        if !self.panel.held {
            self.panel.dx += (self.panel.tx - self.panel.dx) * ease;
            self.panel.dy += (self.panel.ty - self.panel.dy) * ease;
        }
        if self.phase != Phase::Panel {
            for i in 0..self.plugs.len() {
                if self.plugs[i].held {
                    continue;
                }
                let (tx, ty) = match self.plugs[i].seat {
                    Some(s) => (self.sockets[s].x, self.sockets[s].y + 14.0),
                    None => (self.plugs[i].hx, self.plugs[i].hy),
                };
                let p = &mut self.plugs[i];
                p.x += (tx - p.x) * ease;
                p.y += (ty - p.y) * ease;
            }
        }
        for f in &mut self.fast {
            if f.on {
                f.t = (f.t + dt / 0.3).min(1.0);
            }
        }

        match self.phase {
            Phase::Part => {
                let u = &mut self.unit;
                if input.pressed && (input.x - u.x).abs() < 52.0 && (input.y - u.y).abs() < 92.0 {
                    u.held = true;
                    u.gx = input.x - u.x;
                    u.gy = input.y - u.y;
                }
                if u.held && input.down {
                    u.x = input.x - u.gx;
                    u.y = input.y - u.gy;
                }
                if u.held && input.released {
                    u.held = false;
                    if near([u.x, u.y], SLOT, 50.0) {
                        self.seat_unit(cx);
                        return;
                    }
                }
                let u = &mut self.unit;
                if !u.held {
                    u.x += (CRATE[0] - u.x) * ease;
                    u.y += (CRATE[1] - u.y) * ease;
                }
                if self.kf && input.action_pressed {
                    self.seat_unit(cx);
                }
            }
            Phase::Panel => {
                let (x, y) = (input.x - PLATE[0], input.y - PLATE[1]);
                if input.pressed && x > 0.0 && x < PLATE[2] && y > 0.0 && y < PLATE[3] {
                    self.panel.held = true;
                    self.panel.gx = input.x;
                    self.panel.gy = input.y;
                }
                if self.panel.held && input.down {
                    self.panel.dx = input.x - self.panel.gx;
                    self.panel.dy = input.y - self.panel.gy;
                }
                if self.panel.held && input.released {
                    self.panel.held = false;
                    if self.panel.dx.hypot(self.panel.dy) > LIFT_PX {
                        self.lift();
                    }
                }
                if self.kf && input.action_pressed {
                    self.lift();
                }
            }
            Phase::Plugs => {
                let (free, open) = (self.free(), self.open());
                if self.kf && self.carry.is_none() {
                    self.fi = wrap_i(self.fi as i32 + dir, free.len());
                    if input.action_pressed {
                        if let Some(&p) = free.get(self.fi) {
                            self.carry = Some(p);
                            self.plugs[p].held = true;
                            self.fi = 0;
                        }
                    }
                } else if let (true, Some(p)) = (self.kf, self.carry) {
                    self.fi = wrap_i(self.fi as i32 + dir, open.len());
                    let Some(&s) = open.get(self.fi) else { return };
                    let (sx, sy) = (self.sockets[s].x, self.sockets[s].y);
                    let pl = &mut self.plugs[p];
                    pl.x += (sx - pl.x) * ease;
                    pl.y += (sy + 84.0 - pl.y) * ease;
                    if input.action_pressed {
                        self.carry = None;
                        self.plugs[p].held = false;
                        self.fi = 0;
                        self.place(cx, p, s);
                    }
                } else {
                    if input.pressed {
                        let hit = free.iter().rev().copied().find(|&q| {
                            (input.x - self.plugs[q].x).abs() < 32.0 && (input.y - self.plugs[q].y).abs() < 40.0
                        });
                        if let Some(p) = hit {
                            self.held = Some(p);
                            let pl = &mut self.plugs[p];
                            pl.held = true;
                            pl.gx = input.x - pl.x;
                            pl.gy = input.y - pl.y;
                        }
                    }
                    if let (Some(p), true) = (self.held, input.down) {
                        let pl = &mut self.plugs[p];
                        pl.x = input.x - pl.gx;
                        pl.y = input.y - pl.gy;
                    }
                    if let (Some(p), true) = (self.held, input.released) {
                        self.held = None;
                        self.plugs[p].held = false;
                        let at = [self.plugs[p].x, self.plugs[p].y - 14.0];
                        let s = open.iter().copied().find(|&q| near(at, [self.sockets[q].x, self.sockets[q].y], 46.0));
                        if let Some(s) = s {
                            self.place(cx, p, s);
                        }
                    }
                }
            }
            Phase::Close => {
                if self.panel.dx.abs() < 2.0 {
                    self.phase = Phase::Torque;
                    self.panel.dx = 0.0;
                    self.panel.dy = 0.0;
                    self.fi = 0;
                }
            }
            Phase::Torque => {
                if self.kf {
                    self.fi = wrap_i(self.fi as i32 + dir, self.fast.len());
                    if input.action_pressed {
                        self.turn(cx, self.fi);
                    }
                } else if input.pressed {
                    let pts: Vec<Option<[f32; 2]>> = self.fast.iter().map(|f| Some([f.x, f.y])).collect();
                    if let Some(i) = nearest(&pts, input.x, input.y, TOUCH_R + 10.0) {
                        self.turn(cx, i);
                    }
                }
            }
            Phase::Done => {}
        }
    }

    fn draw(&self, g: &Pen, _cx: &Ctx, t: f32, _input: &Input) {
        g.rect(0.0, BAR_H, W, H, hex(0x070b12));
        self.draw_hangar(g);
        // The close up: the hull skin round the bay.
        let [fx, fy, fw, fh] = FRAME;
        g.round(fx, fy, fw, fh, 18.0, Some(hex(0x151d2a)), None);
        let f = g.clip(fx, fy, fw, fh);
        for x in [462.0, 1150.0] {
            f.line(x, fy, x, fy + fh, 3.0, hex(0x0d131c));
        }
        f.line(fx, 492.0, fx + fw, 492.0, 3.0, hex(0x0d131c));
        let mut x = 440.0;
        while x < 1256.0 {
            f.disc(x, 482.0, 2.5, hex(0x2a3446));
            f.disc(x, 502.0, 2.5, hex(0x2a3446));
            x += 28.0;
        }
        self.draw_bay(&f);
        if self.phase == Phase::Part {
            self.draw_rack(&f);
        } else if self.phase != Phase::Panel {
            for p in self.plugs.iter().filter(|p| !p.held) {
                self.draw_cable(&f, p);
            }
            for p in self.plugs.iter().filter(|p| !p.held) {
                self.draw_plug(&f, p);
            }
        }
        self.draw_plate(&f, t);
        if self.phase == Phase::Part {
            self.draw_crate(&f);
        }
        for p in self.plugs.iter().filter(|p| p.held) {
            self.draw_cable(&f, p);
            self.draw_plug(&f, p);
        }
        // Keys: the chosen item, dashed (only once a key has been used).
        if self.kf && self.phase != Phase::Done {
            let focus = match self.phase {
                Phase::Panel => Some((PLATE[0] + PLATE[2] / 2.0, PLATE[1] + PLATE[3] / 2.0, 70.0)),
                Phase::Plugs if self.carry.is_none() => {
                    self.free().get(self.fi).map(|&p| (self.plugs[p].x, self.plugs[p].y, 48.0))
                }
                Phase::Plugs => self.open().get(self.fi).map(|&s| (self.sockets[s].x, self.sockets[s].y, 32.0)),
                Phase::Torque => self.fast.get(self.fi).map(|q| (q.x, q.y, 28.0)),
                Phase::Part => Some((SLOT[0], SLOT[1], 70.0)),
                _ => None,
            };
            if let Some((x, y, r)) = focus {
                dashed_ring(&f, x, y, r, c::AMBER, 3.0);
            }
        }
        if let Some((sx, sy, st)) = self.spark {
            let k = st / 0.5;
            for i in 0..9 {
                let a = i as f32 * 0.7 + t * 9.0;
                let (r0, r1) = (10.0, 20.0 + 40.0 * (1.0 - k));
                f.line(sx + r0 * a.cos(), sy + r0 * a.sin(), sx + r1 * a.cos(), sy + r1 * a.sin(), 3.0, c::WARN);
            }
        }
        g.round(fx, fy, fw, fh, 18.0, None, Some((2.0, c::LINE)));
    }

    fn hand(&mut self, _cx: &Ctx, _t: f32) -> Input {
        if self.hand_wait > 0 {
            self.hand_wait -= 1;
            return Input::default();
        }
        match self.phase {
            // Carry the new unit from the crate into its slot.
            Phase::Part => self.carry_to([self.unit.x, self.unit.y], SLOT),
            // Drag the panel well clear of the bay.
            Phase::Panel => {
                let mid = [PLATE[0] + PLATE[2] / 2.0, PLATE[1] + PLATE[3] / 2.0];
                self.carry_to(mid, [mid[0] + 200.0, mid[1]])
            }
            // Each lead to the socket of its shape and stripe, once the leads have come up to their places.
            Phase::Plugs => {
                if !self.hand_down {
                    let settled = self.plugs.iter().all(|p| p.seat.is_some() || near([p.x, p.y], [p.hx, p.hy], 2.0));
                    if !settled {
                        return Input::default();
                    }
                }
                let Some(p) = self.free().first().copied() else { return Input::default() };
                let pl = &self.plugs[p];
                let s = (0..self.sockets.len())
                    .find(|&s| self.sockets[s].shape == pl.shape && self.sockets[s].stripe == pl.stripe)
                    .unwrap_or(p);
                let (from, to) = ([pl.x, pl.y], [self.sockets[s].x, self.sockets[s].y + 14.0]);
                self.carry_to(from, to)
            }
            // Tap the lit fastener, then the next.
            Phase::Torque => {
                let Some(i) = next_in_star(&self.order, self.fast.len()) else { return Input::default() };
                let (x, y) = (self.fast[i].x, self.fast[i].y);
                if !self.hand_down {
                    self.hand_down = true;
                    return Input::hold(x, y, true);
                }
                self.hand_down = false;
                self.hand_wait = 14;
                Input::release(x, y)
            }
            Phase::Close | Phase::Done => Input::default(),
        }
    }

    fn guide_now(&self) -> Option<usize> {
        match self.phase {
            Phase::Panel => Some(0),
            Phase::Plugs => Some(1),
            Phase::Close | Phase::Torque => Some(2),
            Phase::Part | Phase::Done => None,
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
        plays_to_end("fighter");
    }

    #[test]
    fn a_plug_forced_into_the_wrong_socket_sparks() {
        // By keys: Space lifts the panel, Space picks up the first free lead, Right moves to the second open socket,
        // Space forces it in. Lead i belongs in socket i, so the second socket is always the wrong one.
        let mut f = 0;
        fumble_check("fighter", move |_r| {
            f += 1;
            match f % 80 {
                1 | 40 | 60 => Input::default().key(Key::Space, true),
                50 => Input::default().key(Key::ArrowRight, true),
                _ => Input::default(),
            }
        });
    }
}
