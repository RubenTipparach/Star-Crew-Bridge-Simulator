//! A toilet's repair (repair-minigames design 2 and 1a), from the toilet half of `docs/mockups/repairs/heads.js`.
//!
//! A clogged toilet seen from the front, lid up, a plunger standing in the bowl. Plunge it: drag the plunger down and
//! up in the bowl (or Down then Up on the keys, or tap Space for a whole stroke). Each full stroke pushes the clog
//! along the trap, drawn in the cutaway beside the toilet; the ring on the plunger's knob shows the safe rhythm
//! (dashed and filling: too soon; solid: go). A stroke too soon sloshes the water up the bowl, and water over the rim
//! is the fumble "Overflow: the floor is wet" (a puddle, a wet floor sign); the level drains back slowly. Pressing the
//! flush on a clog fills the bowl to overflowing too. Once the clog goes the culprit pops up (a rubber duck), and the
//! flush handle glows: press it (click it, Enter or Space) and the water swirls away. Each level is a tougher clog
//! (more strokes) and a narrower safe rhythm (a longer wait between strokes, a bigger slosh, a slower drain, suction
//! lost sooner if you dawdle). A disabled toilet's first step fits the new flush valve (the flapper): the cistern is
//! open, drag the flapper from the crate onto its seat (or move it with the arrows and drop it with Space).
//!
//! The drawing helpers the shower shares (an ellipse's points, Bezier curves, a shaded fan, the rubber duck) live here
//! as `pub(crate)` items, as the two games were one script in the mockup.

use egui::{Color32, Key};

use crate::kit::{poly_at, poly_cut, Ctx, Dice, Game, Input, BAR_H, H, W};
use crate::pen::{c, hex, mix, rgba, Pen};

/// In the engine.
pub const PORTED: bool = true;

/// A fresh game.
pub fn new() -> Box<dyn Game> {
    Box::new(Toilet::default())
}

const OVERFLOW: &str = "Overflow: the floor is wet";
/// The toilet's centre line and the floor, px.
const TX: f32 = 590.0;
const FLOOR: f32 = 652.0;
/// The bowl's rim: y, rx, ry.
const RIM: [f32; 3] = [388.0, 186.0, 66.0];
/// The seat's opening: y, rx, ry.
const HOLE: [f32; 3] = [392.0, 136.0, 44.0];
/// The cistern: x, y, w, h.
const CIS: [f32; 4] = [TX - 136.0, 112.0, 272.0, 150.0];
/// The flush handle's pivot, and its lever's length.
const PIV: [f32; 2] = [TX + 124.0, 150.0];
const LEVER: f32 = 66.0;
/// The handle's angle at rest and pressed, rad.
const HANDLE_REST: f32 = 0.08;
const HANDLE_DOWN: f32 = 0.62;
/// Pointer travel for a whole stroke, px.
const TRAVEL: f32 = 120.0;
/// The flapper's seat in the open cistern (part step).
const SEAT: [f32; 2] = [TX - 34.0, 244.0];
/// The spare flapper's crate (part step): x, y, w, h.
const CRATE: [f32; 4] = [960.0, 520.0, 190.0, 120.0];
/// The bowl's level column: x, y, w, h.
const COL: [f32; 4] = [290.0, 300.0, 34.0, 250.0];
/// The bowl's resting level clogged and clear (1 is the rim).
const CLOGGED: f32 = 0.58;
const CLEAR: f32 = 0.45;
/// The trap in cutaway (the clog gauge): from the bowl's outlet, under and over the weir, down to the floor.
const TRAP_PTS: [[f32; 2]; 11] = [
    [946.0, 296.0],
    [944.0, 370.0],
    [958.0, 432.0],
    [996.0, 466.0],
    [1036.0, 450.0],
    [1058.0, 404.0],
    [1090.0, 374.0],
    [1128.0, 386.0],
    [1146.0, 436.0],
    [1150.0, 520.0],
    [1150.0, 588.0],
];
/// The lump sits this far along the trap when the last stroke lands.
const LUMP_END: f32 = 0.86;

// ------------------------------------------------------------------------------------- shared drawing helpers

/// Smoothstep of `u` clamped to 0-1.
pub(crate) fn ease(u: f32) -> f32 {
    let u = u.clamp(0.0, 1.0);
    u * u * (3.0 - 2.0 * u)
}

/// Between `a` and `b` by `u`.
pub(crate) fn lerp(a: f32, b: f32, u: f32) -> f32 {
    a + (b - a) * u
}

/// The points of an ellipse round (cx, cy), radii rx and ry, turned by `rot`, from angle a0 to a1 (canvas's
/// `ellipse`), `n` segments.
#[allow(clippy::too_many_arguments)]
pub(crate) fn ellipse(cx: f32, cy: f32, rx: f32, ry: f32, rot: f32, a0: f32, a1: f32, n: usize) -> Vec<[f32; 2]> {
    let (s, c) = rot.sin_cos();
    (0..=n)
        .map(|i| {
            let a = a0 + (a1 - a0) * i as f32 / n as f32;
            let (x, y) = (a.cos() * rx, a.sin() * ry);
            [cx + x * c - y * s, cy + x * s + y * c]
        })
        .collect()
}

/// A whole ellipse's outline, unturned.
pub(crate) fn oval(cx: f32, cy: f32, rx: f32, ry: f32) -> Vec<[f32; 2]> {
    let mut p = ellipse(cx, cy, rx, ry, 0.0, 0.0, std::f32::consts::TAU, 56);
    p.pop();
    p
}

/// The points of a cubic Bezier from `a` to `d` (canvas's `bezierCurveTo`), `n` segments, `a` included.
pub(crate) fn cubic(a: [f32; 2], b: [f32; 2], c2: [f32; 2], d: [f32; 2], n: usize) -> Vec<[f32; 2]> {
    (0..=n)
        .map(|i| {
            let t = i as f32 / n as f32;
            let u = 1.0 - t;
            let k = [u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t];
            [
                k[0] * a[0] + k[1] * b[0] + k[2] * c2[0] + k[3] * d[0],
                k[0] * a[1] + k[1] * b[1] + k[2] * c2[1] + k[3] * d[1],
            ]
        })
        .collect()
}

/// The points of a quadratic Bezier from `a` to `c2` (canvas's `quadraticCurveTo`), `n` segments, `a` included.
pub(crate) fn quadratic(a: [f32; 2], b: [f32; 2], c2: [f32; 2], n: usize) -> Vec<[f32; 2]> {
    (0..=n)
        .map(|i| {
            let t = i as f32 / n as f32;
            let u = 1.0 - t;
            [u * u * a[0] + 2.0 * u * t * b[0] + t * t * c2[0], u * u * a[1] + 2.0 * u * t * b[1] + t * t * c2[1]]
        })
        .collect()
}

/// A star-shaped polygon filled from `centre` out with a colour a point (a canvas gradient's fill, for shapes the
/// pen's rectangle gradient cannot cover): two rings of triangles, each vertex coloured by `col`.
pub(crate) fn fan(g: &Pen, pts: &[[f32; 2]], centre: [f32; 2], col: impl Fn([f32; 2]) -> Color32) {
    let n = pts.len();
    if n < 3 {
        return;
    }
    let mid: Vec<[f32; 2]> = pts.iter().map(|p| [(centre[0] + p[0]) / 2.0, (centre[1] + p[1]) / 2.0]).collect();
    let cc = col(centre);
    for i in 0..n {
        let j = (i + 1) % n;
        g.quad([centre, centre, mid[i], mid[j]], [cc, cc, col(mid[i]), col(mid[j])]);
        g.quad([mid[i], mid[j], pts[j], pts[i]], [col(mid[i]), col(mid[j]), col(pts[j]), col(pts[i])]);
    }
}

/// The culprit: a rubber duck at (x, y), scaled and turned.
pub(crate) fn duck(g: &Pen, x: f32, y: f32, sc: f32, rot: f32) {
    let p = g.translate(x, y).rotate(rot).scale(sc, sc);
    let yellow = hex(0xffd23f);
    p.poly(&oval(0.0, 0.0, 26.0, 15.0), yellow);
    p.poly(&[[-24.0, -2.0], [-36.0, -12.0], [-22.0, -10.0]], yellow);
    p.disc(14.0, -16.0, 12.0, yellow);
    p.poly(&[[24.0, -18.0], [36.0, -14.0], [24.0, -11.0]], hex(0xf2803a));
    p.disc(17.0, -19.0, 2.6, hex(0x1a1a1a));
    let mut wing = ellipse(-4.0, 2.0, 12.0, 6.0, -0.3, 0.0, std::f32::consts::TAU, 24);
    wing.pop();
    p.poly(&wing, hex(0xf0bd2a));
}

/// The parts of a polyline that lie where `inside` holds (a stroke clipped to a shape the pen cannot clip to).
fn path_where(g: &Pen, pts: &[[f32; 2]], w: f32, col: Color32, inside: &impl Fn(f32, f32) -> bool) {
    let mut run: Vec<[f32; 2]> = Vec::new();
    for p in pts {
        if inside(p[0], p[1]) {
            run.push(*p);
        } else {
            if run.len() > 1 {
                g.path(&run, false, w, col);
            }
            run.clear();
        }
    }
    if run.len() > 1 {
        g.path(&run, false, w, col);
    }
}

/// The kit's diagonal hatching over a rectangle, kept to where `inside` holds.
#[allow(clippy::too_many_arguments)]
fn hatch_where(g: &Pen, x: f32, y: f32, w: f32, h: f32, col: Color32, inside: &impl Fn(f32, f32) -> bool) {
    let mut i = -h;
    while i < w {
        let pts: Vec<[f32; 2]> = (0..=40).map(|k| k as f32 / 40.0).map(|u| [x + i + h * u, y + h - h * u]).collect();
        path_where(g, &pts, 3.0, col, inside);
        i += 12.0;
    }
}

/// The porcelain's sheen across x0..x1 (the mockup's linear gradient).
fn porcelain(x0: f32, x1: f32) -> impl Fn([f32; 2]) -> Color32 {
    move |p: [f32; 2]| {
        let t = ((p[0] - x0) / (x1 - x0).max(1e-6)).clamp(0.0, 1.0);
        if t < 0.55 {
            mix(hex(0xf2f5f8), hex(0xd3dae3), t / 0.55)
        } else {
            mix(hex(0xd3dae3), hex(0x97a3b2), (t - 0.55) / 0.45)
        }
    }
}

/// A rounded rectangle in porcelain, outlined.
#[allow(clippy::too_many_arguments)]
fn porcelain_box(g: &Pen, x: f32, y: f32, w: f32, h: f32, r: f32, line: f32) {
    let pts = crate::pen::round_rect_points(x, y, w, h, r);
    fan(g, &pts, [x + w / 2.0, y + h / 2.0], porcelain(x, x + w));
    g.path(&pts, true, line, c::STEEL);
}

/// The two-circle radial gradient's parameter at `p`: from the circle (c0, r0) at 0 to (c1, r1) at 1.
fn radial_t(p: [f32; 2], c0: [f32; 2], r0: f32, c1: [f32; 2], r1: f32) -> f32 {
    let (qx, qy) = (p[0] - c0[0], p[1] - c0[1]);
    let (dx, dy, dr) = (c1[0] - c0[0], c1[1] - c0[1], r1 - r0);
    let a = dx * dx + dy * dy - dr * dr;
    let b = -2.0 * (qx * dx + qy * dy + r0 * dr);
    let cc = qx * qx + qy * qy - r0 * r0;
    let disc = (b * b - 4.0 * a * cc).max(0.0).sqrt();
    let t = if a.abs() < 1e-6 { -cc / b.min(-1e-6) } else { ((-b - disc) / (2.0 * a)).max((-b + disc) / (2.0 * a)) };
    t.clamp(0.0, 1.0)
}

/// Inside the seat's opening.
fn in_hole(x: f32, y: f32) -> bool {
    ((x - TX) / HOLE[1]).powi(2) + ((y - HOLE[0]) / HOLE[2]).powi(2) <= 1.0
}

/// A point pulled inside the seat's opening, towards its centre.
fn into_hole(p: [f32; 2]) -> [f32; 2] {
    let (dx, dy) = ((p[0] - TX) / HOLE[1], (p[1] - HOLE[0]) / HOLE[2]);
    let r = dx.hypot(dy);
    if r <= 1.0 {
        p
    } else {
        [TX + (p[0] - TX) / r, HOLE[0] + (p[1] - HOLE[0]) / r]
    }
}

/// The trap's centre line: a Catmull-Rom curve through its points, 12 samples a span.
fn trap() -> Vec<[f32; 2]> {
    let n = TRAP_PTS.len();
    let mut pts = Vec::with_capacity(n * 12);
    for i in 0..n - 1 {
        let (p0, p1, p2, p3) =
            (TRAP_PTS[i.saturating_sub(1)], TRAP_PTS[i], TRAP_PTS[i + 1], TRAP_PTS[(i + 2).min(n - 1)]);
        for j in 0..12 {
            let u = j as f32 / 12.0;
            let (u2, u3) = (u * u, u * u * u);
            let f = |k: usize| {
                0.5 * (2.0 * p1[k]
                    + (-p0[k] + p2[k]) * u
                    + (2.0 * p0[k] - 5.0 * p1[k] + 4.0 * p2[k] - p3[k]) * u2
                    + (-p0[k] + 3.0 * p1[k] - 3.0 * p2[k] + p3[k]) * u3)
            };
            pts.push([f(0), f(1)]);
        }
    }
    pts.push(TRAP_PTS[n - 1]);
    pts
}

/// A thick stroke with round ends.
fn stroke_round(g: &Pen, pts: &[[f32; 2]], w: f32, col: Color32) {
    g.path(pts, false, w, col);
    if let (Some(a), Some(b)) = (pts.first(), pts.last()) {
        g.disc(a[0], a[1], w / 2.0, col);
        g.disc(b[0], b[1], w / 2.0, col);
    }
}

/// A dashed polyline whose dashes run along it by `offset` px (canvas's `lineDashOffset`).
#[allow(clippy::too_many_arguments)]
fn dashes(g: &Pen, pts: &[[f32; 2]], w: f32, col: Color32, dash: f32, gap: f32, offset: f32) {
    let len = crate::kit::poly_len(pts);
    if len <= 0.0 {
        return;
    }
    let period = dash + gap;
    let mut s = offset.rem_euclid(period) - period;
    while s < len {
        let (a, b) = (s.max(0.0), (s + dash).min(len));
        if b > a {
            let seg: Vec<[f32; 2]> = (0..=6)
                .map(|k| {
                    let p = poly_at(pts, (a + (b - a) * k as f32 / 6.0) / len);
                    [p[0], p[1]]
                })
                .collect();
            g.path(&seg, false, w, col);
        }
        s += period;
    }
}

// ------------------------------------------------------------------------------------------------- the game

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Phase {
    #[default]
    Part,
    Plunge,
    Ready,
    Flush,
    Fresh,
}

#[derive(Clone, Copy, Debug, Default)]
struct Tune {
    need: f32,
    min_gap: f32,
    max_gap: f32,
    slosh: f32,
    bump: f32,
    drain: f32,
}

#[derive(Clone, Copy, Debug, Default)]
struct Plunger {
    d: f32,
    target: f32,
    grab: bool,
    gy: f32,
    gd: f32,
    auto: bool,
    bottom: bool,
}

#[derive(Clone, Copy, Debug)]
struct Drop {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    life: f32,
}

#[derive(Clone, Copy, Debug, Default)]
struct Flapper {
    x: f32,
    y: f32,
    held: bool,
    pad: bool,
    set: bool,
}

/// The hand's place in a stroke: pressing down, or coming back up, at a pointer height.
#[derive(Clone, Copy, Debug, PartialEq)]
enum HandStroke {
    Down(f32),
    Up(f32),
}

/// The game's state.
#[derive(Default)]
pub struct Toilet {
    r: Option<Dice>,
    part: bool,
    tune: Tune,
    phase: Phase,
    level: f32,
    rest: f32,
    wob: f32,
    rising: bool,
    spill: bool,
    spill_t: f32,
    puddle: u32,
    puddle_t: f32,
    prog: f32,
    strokes: u32,
    last: f32,
    clock: f32,
    clear_t: f32,
    out: f32,
    ft: f32,
    from: f32,
    swirl: f32,
    flushed: bool,
    press: f32,
    h_ang: f32,
    plunger: Plunger,
    drops: Vec<Drop>,
    wad: [f32; 11],
    flapper: Flapper,
    /// The hand: carrying the flapper.
    hand_carry: bool,
    /// The hand: mid stroke.
    hand_stroke: Option<HandStroke>,
    /// The hand: pressed the handle.
    hand_flushed: bool,
}

fn handle_tip(a: f32) -> [f32; 2] {
    [PIV[0] + a.cos() * LEVER, PIV[1] + a.sin() * LEVER]
}

/// The water's surface for a level: y, rx, ry.
fn water_shape(l: f32) -> [f32; 3] {
    [HOLE[0] + 22.0 - 26.0 * l, 30.0 + 104.0 * l, 10.0 + 33.0 * l]
}

impl Toilet {
    fn rnd(&mut self) -> f32 {
        self.r.as_mut().map_or(0.5, Dice::f)
    }

    fn splash(&mut self, n: usize, x: f32, y: f32) {
        for _ in 0..n {
            let (a, b, c2, d) = (self.rnd(), self.rnd(), self.rnd(), self.rnd());
            self.drops.push(Drop {
                x: x + (a - 0.5) * 120.0,
                y,
                vx: (b - 0.5) * 260.0,
                vy: -(160.0 + c2 * 260.0),
                life: 0.7 + d * 0.4,
            });
        }
    }

    fn on_handle(&self, x: f32, y: f32) -> bool {
        let [hx, hy] = handle_tip(self.h_ang);
        (x - hx).hypot(y - hy) < 46.0 || (x - (PIV[0] + hx) / 2.0).hypot(y - (PIV[1] + hy) / 2.0) < 30.0
    }

    fn stroke(&mut self) {
        let k = self.tune;
        let gap = self.clock - self.last;
        let fast = self.strokes > 0 && gap < k.min_gap;
        self.last = self.clock;
        self.strokes += 1;
        self.wob = 1.0;
        if fast {
            self.level += k.slosh;
            self.prog += 0.5;
            self.splash(7, TX, HOLE[0] - 10.0);
        } else {
            self.level += k.bump;
            self.prog += 1.0;
        }
        if self.prog >= k.need {
            self.prog = k.need;
            self.phase = Phase::Ready;
            self.rest = CLEAR;
            self.clear_t = 0.0;
            self.plunger.grab = false;
        }
    }

    fn part_update(&mut self, cx: &mut Ctx, dt: f32, input: &Input) {
        let f = &mut self.flapper;
        if f.set {
            return;
        }
        if input.stick != [0.0, 0.0] {
            f.x += input.stick[0] * 480.0 * dt;
            f.y += input.stick[1] * 480.0 * dt;
            f.pad = true;
        }
        if input.pressed && (input.x - f.x).hypot(input.y - f.y) < 56.0 {
            f.held = true;
        }
        if f.held && input.down {
            f.x = input.x;
            f.y = input.y;
        }
        if (f.held && input.released) || (f.pad && input.action_pressed) {
            f.held = false;
            f.pad = false;
            if (f.x - SEAT[0]).hypot(f.y - SEAT[1]) < 42.0 {
                f.set = true;
                f.x = SEAT[0];
                f.y = SEAT[1];
                cx.step_done();
            }
        }
    }

    // ------------------------------------------------------------------------------------------- drawing

    fn room(g: &Pen) {
        g.rect(0.0, BAR_H, W, FLOOR - BAR_H, hex(0x0b1119));
        let mut y = BAR_H + 44.0;
        while y < FLOOR {
            g.rect(0.0, y, W, 2.0, hex(0x111a26));
            y += 48.0;
        }
        let mut x = 24.0;
        while x < W {
            g.rect(x, BAR_H, 2.0, FLOOR - BAR_H, hex(0x111a26));
            x += 48.0;
        }
        g.rect(0.0, FLOOR - 16.0, W, 16.0, hex(0x1a2433));
        g.rect(0.0, FLOOR, W, H - FLOOR, hex(0x0e141d));
        let mut x = 0.0;
        while x < W {
            g.rect(x, FLOOR, 2.0, H - FLOOR, hex(0x121a25));
            x += 64.0;
        }
    }

    fn cistern(&self, g: &Pen, open: bool) {
        let [x, y, w, h] = CIS;
        g.round(x + 8.0, y + 8.0, w, h, 14.0, Some(rgba(0, 0, 0, 0.35)), None);
        porcelain_box(g, x, y, w, h, 14.0, 3.0);
        if open {
            // The front cut away: inside, the water, the overflow tube, the fill valve and the empty flapper seat.
            g.round(x + 14.0, y + 10.0, w - 28.0, h - 22.0, 8.0, Some(hex(0x16202c)), None);
            g.rect(x + 14.0, y + 52.0, w - 28.0, h - 64.0, rgba(79, 170, 230, 0.35));
            g.rect(x + 14.0, y + 52.0, w - 28.0, 3.0, rgba(160, 215, 245, 0.5));
            g.rect(TX + 14.0, y + 26.0, 14.0, h - 50.0, hex(0x8d98a8));
            g.rect(TX + 86.0, y + 30.0, 18.0, h - 54.0, hex(0x6b7686));
            g.round(TX + 76.0, y + 64.0, 38.0, 26.0, 6.0, Some(hex(0x4a5566)), None);
            g.line(PIV[0] - 4.0, PIV[1], SEAT[0] + 4.0, PIV[1] + 6.0, 5.0, hex(0xb7c0cc));
            // The chain, hanging to the seat (taut once the flapper is on it).
            let from = [SEAT[0] + 4.0, PIV[1] + 6.0];
            let chain = if self.flapper.set {
                vec![from, [SEAT[0] + 2.0, SEAT[1] - 12.0]]
            } else {
                quadratic(from, [SEAT[0] + 26.0, SEAT[1] - 30.0], [SEAT[0] + 18.0, SEAT[1] - 14.0], 12)
            };
            g.dashed(&chain, 2.0, hex(0x9aa6b6), 4.0, 3.0);
            let seat = oval(SEAT[0], SEAT[1] + 2.0, 30.0, 9.0);
            g.poly(&seat, hex(0x0a0e14));
            g.path(&seat, true, 3.0, hex(0x5a6577));
        } else {
            porcelain_box(g, x - 8.0, y - 14.0, w + 16.0, 20.0, 8.0, 2.0);
        }
    }

    fn handle(&self, g: &Pen, t: f32, ready: bool) {
        let [hx, hy] = handle_tip(self.h_ang);
        if ready {
            // The main action: the handle glows and shows which way it goes.
            let pulse = 0.5 + 0.5 * (t * 5.0).sin();
            g.disc(hx, hy, 44.0 + 6.0 * pulse, rgba(242, 160, 70, 0.12 + 0.12 * pulse));
            g.ring(hx, hy, 44.0 + 6.0 * pulse, c::AMBER, 4.0);
            g.turn_arrow(PIV[0], PIV[1], LEVER + 30.0, 0.05, 0.75, c::AMBER);
        }
        g.path_round(&[PIV, [hx, hy]], 16.0, hex(0x5b6574));
        g.path_round(&[PIV, [hx, hy]], 10.0, hex(0xd5dce5));
        g.disc(hx, hy, 11.0, hex(0xe8eef6));
        g.ring(hx, hy, 11.0, c::STEEL, 2.0);
        g.disc(PIV[0], PIV[1], 15.0, hex(0xb7c0cc));
        g.ring(PIV[0], PIV[1], 15.0, hex(0x6b7686), 3.0);
    }

    fn bowl_body(g: &Pen) {
        let [ry, rx, _] = RIM;
        let mut pts = cubic([TX - rx, ry], [TX - rx, ry + 110.0], [TX - 92.0, ry + 148.0], [TX - 80.0, ry + 176.0], 20);
        pts.push([TX - 100.0, FLOOR]);
        pts.push([TX + 100.0, FLOOR]);
        pts.extend(cubic([TX + 80.0, ry + 176.0], [TX + 92.0, ry + 148.0], [TX + rx, ry + 110.0], [TX + rx, ry], 20));
        fan(g, &pts, [TX, ry + 120.0], porcelain(TX - rx, TX + rx));
        g.path(&pts, true, 3.0, c::STEEL);
        g.rect(TX - 100.0, FLOOR - 10.0, 200.0, 10.0, rgba(0, 0, 0, 0.12));
    }

    fn wad(&self, g: &Pen, x: f32, y: f32, sc: f32) {
        let n = self.wad.len();
        let pts: Vec<[f32; 2]> = self
            .wad
            .iter()
            .enumerate()
            .map(|(i, k)| {
                let a = i as f32 / n as f32 * std::f32::consts::TAU;
                [x + a.cos() * 30.0 * sc * k, y + a.sin() * 18.0 * sc * k]
            })
            .collect();
        g.poly(&pts, hex(0xe9e4d6));
        g.path(&pts, true, 2.0, hex(0xb3ab97));
        for i in 0..3 {
            let o = i as f32 * 12.0 * sc;
            g.line(x - 18.0 * sc + o, y - 8.0 * sc, x - 10.0 * sc + o, y + 6.0 * sc, 1.5, hex(0xc9c2ae));
        }
    }

    fn bowl(&self, g: &Pen, t: f32) {
        let [ry, rx, rr] = RIM;
        let [hy, hrx, hry] = HOLE;
        let rim = oval(TX, ry, rx, rr);
        g.poly(&rim, hex(0xe6ebf0));
        g.path(&rim, true, 3.0, c::STEEL);
        if self.part {
            let lid = oval(TX, ry - 4.0, rx - 6.0, rr - 4.0);
            g.poly(&lid, hex(0xc6d0dc));
            g.path(&lid, true, 3.0, hex(0x8a96a6));
            let glint = ellipse(
                TX,
                ry - 10.0,
                rx - 30.0,
                rr - 18.0,
                0.0,
                std::f32::consts::PI * 1.1,
                std::f32::consts::PI * 1.9,
                24,
            );
            g.path(&glint, false, 3.0, rgba(255, 255, 255, 0.5));
            return;
        }
        let seat = oval(TX, ry + 2.0, rx - 6.0, rr - 6.0);
        g.poly(&seat, hex(0xc6d0dc));
        g.path(&seat, true, 2.0, hex(0x8a96a6));
        // The opening: shaded down into the drain.
        let c0 = [TX, hy + 22.0];
        fan(g, &oval(TX, hy, hrx, hry), c0, |p| mix(hex(0x4d5763), hex(0xd6dce3), radial_t(p, c0, 8.0, [TX, hy], hrx)));
        g.poly(&oval(TX, hy + 24.0, 26.0, 8.0), hex(0x151b23));
        let clogged = self.phase == Phase::Plunge;
        // The clog under the water, smaller as it goes, the duck jammed in it.
        if clogged {
            let sc = 1.0 - 0.55 * (self.prog / self.tune.need.max(1.0));
            self.wad(g, TX + 4.0, hy + 16.0, sc);
            duck(g, TX + 70.0, hy + 10.0, 0.78, 2.3);
        }
        let l = self.level.min(1.02);
        let [wy, wrx, wry] = water_shape(l);
        let wob = self.wob * (t * 15.0).sin();
        let clean = if self.phase == Phase::Flush { self.ft > 0.4 } else { !clogged };
        let water: Vec<[f32; 2]> = oval(TX, wy + wob * 3.0, wrx * (1.0 + 0.05 * wob), wry * (1.0 - 0.05 * wob))
            .into_iter()
            .map(into_hole)
            .collect();
        g.poly(&water, if clean { rgba(79, 170, 230, 0.72) } else { rgba(112, 132, 104, 0.82) });
        let mut sheen = oval(TX - wrx * 0.2, wy - wry * 0.3 + wob * 3.0, wrx * 0.5, wry * 0.25);
        sheen.push(sheen[0]);
        g.path(&sheen, false, 2.0, if clean { rgba(200, 235, 255, 0.55) } else { rgba(190, 200, 170, 0.4) });
        if self.phase == Phase::Flush {
            // The swirl: spiral arms turning down into the drain.
            for arm in 0..4 {
                let pts: Vec<[f32; 2]> = (0..=18)
                    .map(|j| {
                        let f = j as f32 / 18.0;
                        let a = self.swirl + arm as f32 * std::f32::consts::FRAC_PI_2 + f * 3.4;
                        let k = (1.0 - f) * 0.92;
                        [TX + a.cos() * wrx * k, wy + a.sin() * wry * k]
                    })
                    .collect();
                g.path(&pts, false, 3.0, rgba(230, 245, 255, 0.75));
            }
            g.disc(TX, wy + 2.0, 8.0 + 10.0 * self.ft.min(1.0), rgba(20, 40, 60, 0.6));
        }
        // The duck: up from the drain when the clog goes, afloat, then away down the swirl.
        if self.phase == Phase::Ready {
            let up = ease(self.clear_t / 0.6);
            let bob = (t * 3.0).sin() * 3.0;
            duck(g, TX + 10.0, lerp(hy + 22.0, wy - 10.0, up) + bob, lerp(0.4, 0.9, up), (t * 2.0).sin() * 0.12);
            if self.clear_t < 0.9 {
                let r = 20.0 + self.clear_t * 90.0;
                let ring = ellipse(TX, wy, r, r, 0.0, 0.0, std::f32::consts::TAU, 72);
                path_where(g, &ring, 3.0, rgba(220, 240, 255, 0.8 - self.clear_t * 0.85), &in_hole);
            }
        }
        if self.phase == Phase::Flush && self.ft < 1.3 {
            let f = self.ft / 1.3;
            let a = self.swirl * 1.1;
            let k = (1.0 - f) * wrx * 0.6;
            duck(g, TX + a.cos() * k, wy - 10.0 + a.sin() * k * 0.32 + f * 14.0, 0.9 * (1.0 - 0.75 * f), a * 0.8);
        }
        if self.level > 0.82 && clogged {
            // Near the rim, the rim is hatched red: the water is about to go over.
            let band = |x: f32, y: f32| {
                ((x - TX) / (rx - 4.0)).powi(2) + ((y - ry - 2.0) / (rr - 4.0)).powi(2) <= 1.0
                    && ((x - TX) / (hrx + 6.0)).powi(2) + ((y - hy) / (hry + 4.0)).powi(2) > 1.0
            };
            let a = 0.35 + 0.5 * ((self.level - 0.82) / 0.18).clamp(0.0, 1.0);
            hatch_where(g, TX - rx, ry - rr, rx * 2.0, rr * 2.0 + 6.0, rgba(255, 71, 87, a), &band);
        }
        // Fresh: a squeak-clean glint or two.
        if self.phase == Phase::Fresh || (self.phase == Phase::Flush && self.ft > 2.2) {
            for (gx, gy, ph) in [
                (TX - 120.0, ry - 30.0, 0.0),
                (TX + 140.0, ry + 10.0, 1.7),
                (TX - 40.0, ry + 120.0, 3.1),
                (TX + 60.0, CIS[1] + 30.0, 4.4),
            ] {
                let k = 0.5 + 0.5 * (t * 4.0 + ph).sin();
                let r = 6.0 + 10.0 * k;
                let q = r * 0.25;
                g.poly(
                    &[
                        [gx, gy - r],
                        [gx + q, gy - q],
                        [gx + r, gy],
                        [gx + q, gy + q],
                        [gx, gy + r],
                        [gx - q, gy + q],
                        [gx - r, gy],
                        [gx - q, gy - q],
                    ],
                    rgba(255, 255, 255, 0.4 + 0.5 * k),
                );
            }
        }
    }

    fn lid_up(g: &Pen) {
        let lid = oval(TX, 262.0, 116.0, 98.0);
        g.poly(&lid, hex(0xbcc6d2));
        g.path(&lid, true, 3.0, hex(0x8a96a6));
        g.path(&oval(TX, 262.0, 96.0, 80.0), true, 2.0, rgba(255, 255, 255, 0.35));
        g.rect(TX - 60.0, 352.0, 24.0, 10.0, hex(0x8a96a6));
        g.rect(TX + 36.0, 352.0, 24.0, 10.0, hex(0x8a96a6));
    }

    fn spillover(&self, g: &Pen, t: f32) {
        if self.spill_t <= 0.0 {
            return;
        }
        let a = self.spill_t.min(1.0);
        let ry = RIM[0];
        for (x0, sway) in [(-150.0f32, -1.0f32), (-80.0, -0.5), (10.0, 0.2), (96.0, 0.6), (160.0, 1.0)] {
            let mut pts = vec![[TX + x0, ry + 40.0]];
            let mut y = ry + 40.0;
            while y <= FLOOR {
                pts.push([TX + x0 * (1.0 - (y - ry) / 520.0) + sway * 18.0 + (t * 9.0 + y * 0.05).sin() * 3.0, y]);
                y += 12.0;
            }
            g.path_round(&pts, 6.0, rgba(79, 195, 247, 0.75 * a));
        }
    }

    /// The plunger in the bowl (or set aside once the clog is gone); returns its knob's place.
    fn plunger(&self, g: &Pen, t: f32) -> [f32; 2] {
        let p = self.plunger;
        let sq = ((p.d - 0.65) / 0.35).clamp(0.0, 1.0);
        let e = ease(self.out);
        let cx = lerp(TX, TX + 236.0, e);
        let cup_y = lerp(HOLE[0] + 8.0 + p.d * 24.0, FLOOR - 4.0, e);
        let ang = lerp(0.0, 0.12, e);
        let (w, h, stick) = (96.0 * (1.0 + 0.22 * sq), 42.0 * (1.0 - 0.3 * sq), 205.0);
        if e < 0.2 {
            // Ripples where the cup meets the water.
            let wy = water_shape(self.level.min(1.0))[0];
            g.path(
                &oval(TX, wy + 6.0, w / 2.0 + 12.0 + 6.0 * (t * 6.0).sin(), 10.0),
                true,
                2.0,
                rgba(200, 235, 255, 0.35),
            );
        }
        let q = g.translate(cx, cup_y).rotate(ang);
        q.rect(-7.0, -h - stick, 14.0, stick, hex(0xa4733f));
        q.rect(2.0, -h - stick, 5.0, stick, rgba(0, 0, 0, 0.18));
        q.disc(0.0, -h - stick - 6.0, 14.0, hex(0x7a4f28));
        let cup = cubic([-w / 2.0, 0.0], [-w / 2.0, -h * 1.1], [w / 2.0, -h * 1.1], [w / 2.0, 0.0], 20);
        q.poly(&cup, hex(0xd23c3c));
        q.path(&cup, true, 2.0, hex(0x8e2626));
        q.poly(&oval(0.0, 0.0, w / 2.0, 8.0), hex(0x9e2a2a));
        let mut hi = ellipse(-w * 0.18, -h * 0.55, w * 0.12, h * 0.16, -0.5, 0.0, std::f32::consts::TAU, 20);
        hi.pop();
        q.poly(&hi, rgba(255, 255, 255, 0.25));
        [cx, cup_y - h - stick - 6.0]
    }

    fn rhythm(&self, g: &Pen, t: f32, kx: f32, ky: f32) {
        // The safe rhythm on the knob: dashed and filling, too soon; solid, go; grey dashed, the suction is going.
        use std::f32::consts::{FRAC_PI_2, TAU};
        let k = self.tune;
        let gap = self.clock - self.last;
        if self.strokes == 0 || (gap >= k.min_gap && gap <= k.max_gap) {
            g.ring(kx, ky, 28.0, c::OK, 5.0);
        } else if gap < k.min_gap {
            g.ring(kx, ky, 28.0, hex(0x2a3446), 5.0);
            let arc = Pen::arc_points(kx, ky, 28.0, -FRAC_PI_2, -FRAC_PI_2 + TAU * gap / k.min_gap);
            g.dashed(&arc, 5.0, c::AMBER, 7.0, 5.0);
        } else {
            let r = 28.0 + 2.0 * (t * 8.0).sin();
            g.dashed(&Pen::arc_points(kx, ky, r, 0.0, TAU), 4.0, c::STEEL, 4.0, 6.0);
        }
        if self.strokes == 0 {
            // Up and down, once, beside the handle (a shape, not words), until the first stroke.
            let a = 0.35 + 0.35 * (t * 4.0).sin();
            let (x, y) = (kx - 58.0, ky + 80.0);
            for (dy, dir) in [(-26.0f32, -1.0f32), (26.0, 1.0)] {
                g.poly(
                    &[[x - 14.0, y + dy - dir * 7.0], [x, y + dy + dir * 9.0], [x + 14.0, y + dy - dir * 7.0]],
                    rgba(232, 238, 246, a),
                );
            }
        }
    }

    fn level_column(&self, g: &Pen) {
        let [x, y, w, h] = COL;
        let top = 1.12;
        let y_at = |v: f32| y + h - (h * v) / top;
        g.round(x - 6.0, y - 6.0, w + 12.0, h + 12.0, 10.0, Some(hex(0x0c121a)), Some((2.0, c::LINE)));
        g.hatch(x, y_at(1.12), w, y_at(0.85) - y_at(1.12), rgba(255, 71, 87, 0.5));
        let v = self.level.min(top);
        let col = if self.phase == Phase::Plunge { rgba(112, 132, 104, 0.95) } else { rgba(79, 170, 230, 0.9) };
        g.rect(x, y_at(v), w, y + h - y_at(v), col);
        g.rect(x - 10.0, y_at(1.0) - 2.0, w + 20.0, 4.0, c::DANGER);
        // A bowl glyph at the foot, so the column reads as the bowl's water.
        let mut glyph = vec![[x - 6.0, y + h + 18.0]];
        glyph.extend(quadratic([x + w + 6.0, y + h + 18.0], [x + w / 2.0, y + h + 46.0], [x - 6.0, y + h + 18.0], 16));
        g.poly(&glyph, hex(0xc6d0dc));
    }

    fn trap_gauge(&self, g: &Pen, t: f32) {
        g.panel(868.0, 196.0, 352.0, 428.0, 18.0, hex(0x0b1018), c::LINE);
        let plunging = self.phase == Phase::Plunge;
        // The bowl in section, feeding the trap.
        let mut outer = vec![[888.0, 226.0]];
        outer.extend(quadratic([1006.0, 226.0], [1000.0, 300.0], [958.0, 300.0], 12));
        outer.extend(quadratic([958.0, 300.0], [900.0, 296.0], [888.0, 226.0], 12).into_iter().skip(1));
        g.poly(&outer, hex(0xc6d0dc));
        let mut inner = vec![[900.0, 238.0]];
        inner.extend(quadratic([994.0, 238.0], [988.0, 290.0], [954.0, 290.0], 12));
        inner.extend(quadratic([954.0, 290.0], [910.0, 288.0], [900.0, 238.0], 12).into_iter().skip(1));
        g.poly(&inner, if plunging { rgba(112, 132, 104, 0.9) } else { rgba(79, 170, 230, 0.8) });
        let tr = trap();
        stroke_round(g, &tr, 40.0, hex(0xc6d0dc));
        stroke_round(g, &tr, 26.0, hex(0x0d131c));
        g.rect(1112.0, 588.0, 76.0, 12.0, hex(0x3a4658));
        let need = self.tune.need.max(1.0);
        let lu = if plunging {
            (self.prog / need) * LUMP_END
        } else {
            LUMP_END + (1.0 - LUMP_END) * (self.clear_t / 0.7).clamp(0.0, 1.0)
        };
        if plunging {
            if lu > 0.0 {
                stroke_round(g, &poly_cut(&tr, lu), 18.0, rgba(112, 132, 104, 0.95));
            }
        } else {
            stroke_round(g, &tr, 18.0, rgba(79, 170, 230, 0.85));
            dashes(g, &tr, 4.0, rgba(220, 240, 255, 0.7), 10.0, 14.0, t * 90.0);
        }
        // A mark a stroke along the trap: hollow ahead, filled behind the clog.
        let n = need.round() as usize;
        for i in 0..n {
            let p = poly_at(&tr, ((i + 1) as f32 / need) * LUMP_END);
            if (i as f32) < self.prog.floor() || !plunging {
                g.disc(p[0], p[1], 6.0, c::OK);
            } else {
                g.ring(p[0], p[1], 6.0, c::DIM, 2.0);
            }
        }
        if plunging || self.clear_t < 0.7 {
            let p = poly_at(&tr, lu);
            let a = if plunging { 1.0 } else { 1.0 - (self.clear_t / 0.7).clamp(0.0, 1.0) };
            self.wad(&g.alpha(a), p[0], p[1], 0.8);
        }
    }

    fn puddle_and_sign(&self, g: &Pen, t: f32) {
        if self.puddle == 0 {
            return;
        }
        let k = self.puddle_t;
        g.poly(&oval(TX, FLOOR + 26.0, 150.0 + 70.0 * k, 20.0 + 6.0 * k), rgba(79, 195, 247, 0.3));
        g.poly(&oval(TX - 60.0, FLOOR + 22.0, 60.0 + 20.0 * k, 4.0), rgba(220, 240, 255, 0.25));
        // The wet floor sign: an A-frame with a figure slipping, no words.
        let (x, y) = (150.0, FLOOR + 30.0);
        let frame = [[x - 48.0, y], [x - 18.0, y - 150.0], [x + 18.0, y - 150.0], [x + 48.0, y]];
        g.poly(&frame, hex(0xe0b100));
        g.path(&frame, true, 3.0, hex(0x7a6000));
        g.poly(&[[x - 30.0, y - 70.0], [x, y - 122.0], [x + 30.0, y - 70.0]], hex(0x1a1a1a));
        g.poly(&[[x - 24.0, y - 74.0], [x, y - 115.0], [x + 24.0, y - 74.0]], hex(0xe0b100));
        let f = g.translate(x, y - 88.0).rotate(-0.5 + 0.05 * (t * 3.0).sin());
        let ink = hex(0x1a1a1a);
        f.disc(4.0, -16.0, 4.0, ink);
        for (a, b) in [
            ([2.0, -11.0], [-2.0, 2.0]),
            ([-2.0, 2.0], [-10.0, 10.0]),
            ([-2.0, 2.0], [9.0, 7.0]),
            ([1.0, -7.0], [-9.0, -12.0]),
            ([1.0, -7.0], [11.0, -4.0]),
        ] {
            f.path_round(&[a, b], 3.0, ink);
        }
        g.rect(x - 22.0, y - 22.0, 44.0, 5.0, ink);
    }

    fn flapper_glyph(g: &Pen, x: f32, y: f32, seated: bool) {
        use std::f32::consts::{PI, TAU};
        g.poly(&oval(x, y, 32.0, 11.0), hex(0xb23232));
        g.poly(&ellipse(x, y - 4.0, 22.0, 13.0, 0.0, PI, TAU, 20), hex(0xd23c3c));
        g.rect(x - 40.0, y - 4.0, 12.0, 7.0, hex(0x8e2626));
        g.rect(x + 28.0, y - 4.0, 12.0, 7.0, hex(0x8e2626));
        g.ring(x + 2.0, y - 18.0, 5.0, hex(0x9aa6b6), 2.0);
        if !seated {
            let mut hi = ellipse(x - 8.0, y - 9.0, 7.0, 3.0, -0.3, 0.0, TAU, 16);
            hi.pop();
            g.poly(&hi, rgba(255, 255, 255, 0.3));
        }
    }

    fn crate_and_flapper(&self, g: &Pen, t: f32) {
        let [x, y, w, h] = CRATE;
        g.panel(x, y, w, h, 14.0, hex(0x141b27), c::LINE);
        g.hatch(x + 14.0, y + h - 22.0, w - 28.0, 10.0, rgba(242, 160, 70, 0.5));
        let f = self.flapper;
        if !f.set {
            // The empty seat waits, ringed amber.
            g.ring(SEAT[0], SEAT[1] + 2.0, 40.0 + 3.0 * (t * 5.0).sin(), c::AMBER, 3.0);
        }
        Self::flapper_glyph(g, f.x, f.y, f.set);
        if f.set {
            g.ring(SEAT[0], SEAT[1] + 2.0, 40.0, c::OK, 4.0);
        } else if !f.held {
            g.ring(f.x, f.y, 52.0, hex(0xf0c08a), 3.0);
        }
    }

    fn leaning_lid(g: &Pen) {
        // The cistern's lid, set down on the floor out of the way.
        porcelain_box(g, 120.0, FLOOR - 4.0, 288.0, 26.0, 8.0, 2.0);
        g.rect(132.0, FLOOR, 264.0, 3.0, rgba(255, 255, 255, 0.45));
    }
}

impl Game for Toilet {
    fn id(&self) -> &'static str {
        "toilet"
    }

    fn knobs(&self) -> &'static [&'static str] {
        &["strokes_count", "min_gap_s", "max_gap_s", "slosh_frac", "bump_frac", "drain_frac_s"]
    }

    fn step(&mut self, cx: &mut Ctx) {
        let mut r = cx.dice();
        self.part = cx.part;
        self.tune = Tune {
            need: cx.knob("strokes_count"),
            min_gap: cx.knob("min_gap_s"),
            max_gap: cx.knob("max_gap_s"),
            slosh: cx.knob("slosh_frac"),
            bump: cx.knob("bump_frac"),
            drain: cx.knob("drain_frac_s"),
        };
        self.phase = if cx.part { Phase::Part } else { Phase::Plunge };
        self.level = if cx.part { CLEAR } else { CLOGGED };
        self.rest = self.level;
        self.wob = 0.0;
        self.rising = false;
        self.spill = false;
        self.spill_t = 0.0;
        self.puddle = 0;
        self.puddle_t = 0.0;
        self.prog = 0.0;
        self.strokes = 0;
        self.last = -99.0;
        self.clock = 0.0;
        self.clear_t = 0.0;
        self.out = 0.0;
        self.ft = 0.0;
        self.from = 0.0;
        self.swirl = 0.0;
        self.flushed = false;
        self.press = 0.0;
        self.h_ang = HANDLE_REST;
        self.plunger = Plunger::default();
        self.drops.clear();
        // The wad's outline, jittered once a round so it is the same in every shot.
        for k in &mut self.wad {
            *k = 0.75 + 0.45 * r.f();
        }
        self.flapper =
            Flapper { x: CRATE[0] + CRATE[2] / 2.0, y: CRATE[1] + CRATE[3] / 2.0 + 6.0, ..Flapper::default() };
        self.r = Some(r);
        self.hand_carry = false;
        self.hand_stroke = None;
        self.hand_flushed = false;
    }

    fn update(&mut self, cx: &mut Ctx, dt: f32, input: &Input) {
        let k = self.tune;
        self.clock += dt;
        self.wob = (self.wob - dt * 1.4).max(0.0);
        self.press = (self.press - dt).max(0.0);
        self.h_ang += ((if self.press > 0.0 { HANDLE_DOWN } else { HANDLE_REST }) - self.h_ang) * (dt * 14.0).min(1.0);
        self.puddle_t = (self.puddle as f32).min(self.puddle_t + dt * 1.5);
        self.spill_t = (self.spill_t - dt).max(0.0);
        for p in &mut self.drops {
            p.vy += 900.0 * dt;
            p.x += p.vx * dt;
            p.y += p.vy * dt;
            p.life -= dt;
        }
        self.drops.retain(|p| p.life > 0.0 && p.y < FLOOR + 30.0);
        if self.phase != Phase::Plunge {
            self.out = (self.out + dt * 2.0).min(1.0);
        }
        if self.phase == Phase::Ready {
            self.clear_t += dt;
        }
        if self.phase == Phase::Part {
            self.part_update(cx, dt, input);
            return;
        }
        // The water: it rises while a clogged flush fills it, drains back to rest, and is the flush's own in a flush.
        if self.phase == Phase::Flush {
            self.ft += dt;
            self.swirl += dt * (3.0 + self.ft * 5.0);
            self.level = if self.ft < 1.5 {
                lerp(self.from, 0.03, ease(self.ft / 1.5))
            } else {
                lerp(0.03, CLEAR, ease((self.ft - 1.6) / 1.3))
            };
            if self.ft >= 1.5 && !self.flushed {
                self.flushed = true;
                cx.step_done();
            }
            if self.ft >= 3.0 {
                self.phase = Phase::Fresh;
                self.level = CLEAR;
            }
            return;
        }
        if self.phase == Phase::Fresh {
            return;
        }
        if self.rising {
            self.level += 0.55 * dt;
        } else if self.level > self.rest {
            let drain = if self.phase == Phase::Ready { 0.6 } else { k.drain };
            self.level = self.rest.max(self.level - drain * dt);
        } else {
            self.level = self.rest.min(self.level + 0.6 * dt);
        }
        if self.spill && self.level < 0.9 {
            self.spill = false;
        }
        if self.level > 1.0 && !self.spill {
            // Over the rim: the fumble. The level holds at the rim and drains back slowly.
            self.spill = true;
            self.rising = false;
            self.level = 1.0;
            self.puddle += 1;
            self.spill_t = 1.6;
            self.splash(10, TX, RIM[0]);
            cx.fumble(OVERFLOW);
            // A third fumble restarts the round: the kit sets up the next one after this frame.
            return;
        }
        let enter = input.hit(Key::Enter);
        let flush_it = (input.pressed && self.on_handle(input.x, input.y))
            || enter
            || (self.phase == Phase::Ready && input.hit(Key::Space));
        if flush_it {
            self.press = 0.45;
            if self.phase == Phase::Ready {
                self.phase = Phase::Flush;
                self.ft = 0.0;
                self.from = self.level;
                return;
            }
            if !self.spill {
                // Flushing a clog: it only fills the bowl.
                self.rising = true;
            }
            if !enter {
                return;
            }
        }
        if self.phase != Phase::Plunge {
            return;
        }
        // The plunger: the pointer drags it, the keys and Space drive it.
        let p = &mut self.plunger;
        let zone = input.x > TX - 160.0 && input.x < TX + 160.0 && input.y > 110.0 && input.y < HOLE[0] + 80.0;
        if input.pressed && zone {
            p.grab = true;
            p.gy = input.y;
            p.gd = p.d;
            p.auto = false;
        }
        if !input.down {
            p.grab = false;
        }
        if p.grab {
            p.d = (p.gd + (input.y - p.gy) / TRAVEL).clamp(0.0, 1.0);
        } else {
            if input.hit(Key::Space) {
                p.auto = true;
                p.target = 1.0;
            }
            if input.stick[1] > 0.0 {
                p.target = 1.0;
                p.auto = false;
            }
            if input.stick[1] < 0.0 {
                p.target = 0.0;
                p.auto = false;
            }
            if p.auto && p.d >= 0.97 {
                p.target = 0.0;
                p.auto = false;
            }
            p.d += (p.target - p.d).clamp(-8.0 * dt, 8.0 * dt);
        }
        if p.d > 0.8 {
            p.bottom = true;
        }
        if p.bottom && p.d < 0.25 {
            p.bottom = false;
            self.stroke();
        }
        // Dawdle past the safe gap and the suction goes: the clog creeps back.
        if self.phase == Phase::Plunge && self.strokes > 0 && self.clock - self.last > k.max_gap {
            self.prog = (self.prog - 0.4 * dt).max(0.0);
        }
    }

    fn draw(&self, g: &Pen, _cx: &Ctx, t: f32, _input: &Input) {
        Self::room(g);
        let part = self.part;
        if part {
            Self::leaning_lid(g);
        }
        self.cistern(g, part);
        self.handle(g, t, self.phase == Phase::Ready);
        if !part {
            Self::lid_up(g);
        }
        Self::bowl_body(g);
        self.bowl(g, t);
        self.spillover(g, t);
        self.puddle_and_sign(g, t);
        if part {
            self.crate_and_flapper(g, t);
            return;
        }
        let [kx, ky] = self.plunger(g, t);
        if self.phase == Phase::Plunge {
            self.rhythm(g, t, kx, ky);
        }
        for p in &self.drops {
            g.disc(p.x, p.y, 4.0, rgba(79, 195, 247, 0.85));
        }
        self.level_column(g);
        self.trap_gauge(g, t);
    }

    fn hand(&mut self, _cx: &Ctx, _t: f32) -> Input {
        const Y0: f32 = 200.0;
        const STEP_PX: f32 = 12.0;
        match self.phase {
            Phase::Part => {
                // Pick the flapper up, carry it to its seat a few px a frame, let go there.
                let (fx, fy) = (self.flapper.x, self.flapper.y);
                if self.flapper.set {
                    return Input::default();
                }
                if !self.flapper.held && !self.hand_carry {
                    self.hand_carry = true;
                    return Input::hold(fx, fy, true);
                }
                let d = (SEAT[0] - fx).hypot(SEAT[1] - fy);
                if d > 2.0 {
                    let k = (10.0 / d).min(1.0);
                    return Input::hold(fx + (SEAT[0] - fx) * k, fy + (SEAT[1] - fy) * k, false);
                }
                self.hand_carry = false;
                Input::release(SEAT[0], SEAT[1])
            }
            Phase::Plunge => match self.hand_stroke {
                None => {
                    // Wait for the rhythm: a stroke takes about a third of a second from the press to landing, so
                    // start one just before the safe gap opens and it lands inside it.
                    let gap = self.clock - self.last;
                    let go = if self.strokes == 0 { self.clock > 0.4 } else { gap >= self.tune.min_gap - 0.15 };
                    if !go {
                        return Input::at(TX, Y0);
                    }
                    self.hand_stroke = Some(HandStroke::Down(Y0));
                    Input::hold(TX, Y0, true)
                }
                Some(HandStroke::Down(y)) => {
                    let y = y + STEP_PX;
                    self.hand_stroke =
                        Some(if y >= Y0 + TRAVEL + 10.0 { HandStroke::Up(y) } else { HandStroke::Down(y) });
                    Input::hold(TX, y, false)
                }
                Some(HandStroke::Up(y)) => {
                    let y = y - STEP_PX;
                    if y <= Y0 - 4.0 {
                        self.hand_stroke = None;
                        return Input::release(TX, y);
                    }
                    self.hand_stroke = Some(HandStroke::Up(y));
                    Input::hold(TX, y, false)
                }
            },
            Phase::Ready => {
                if self.hand_stroke.take().is_some() {
                    return Input::release(TX, Y0);
                }
                let [hx, hy] = handle_tip(self.h_ang);
                if self.clear_t > 0.6 && !self.hand_flushed {
                    self.hand_flushed = true;
                    return Input::hold(hx, hy, true);
                }
                Input::at(hx, hy)
            }
            Phase::Flush | Phase::Fresh => Input::default(),
        }
    }

    fn guide_now(&self) -> Option<usize> {
        match self.phase {
            Phase::Part => None,
            Phase::Ready | Phase::Flush => Some(2),
            _ => Some(0),
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
        plays_to_end("toilet");
    }

    #[test]
    fn flushing_a_clog_overflows_the_bowl() {
        // Enter (the flush) held from the moment the round opens, the clog still in.
        fumble_check("toilet", |_r| Input::default().key(Key::Enter, true));
    }
}
