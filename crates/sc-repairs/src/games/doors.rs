//! A jammed door's repair (repair-minigames design 2), from `docs/mockups/repairs/doors.js`.
//!
//! Any door on the ship, seen from the corridor: the leaf in its frame, hung from a track over the opening, driven by a
//! chain from the hand crank on the right; above the crank, the strain gauge with its slip limit hatched red. Turn the
//! crank clockwise (drag round in circles, or alternate Left and Right) and the leaf slides open along its track into
//! the wall's pocket. The leaf fights back in waves: it shudders as a wave builds, and turning hard into a wave drives
//! the gauge up. Past the slip limit the crank slips: a fumble, the leaf drops back part of the way. Ease off through
//! each wave and wind on between them. The lamp over the door shows red and crossed while it is stuck, green and ticked
//! once it is open. Each level needs more turns and the waves come harder and closer. A disabled door's first step fits
//! a new drive pinion: drag it from the crate onto the crank's shaft.

use std::f32::consts::{PI, TAU};

use egui::{Color32, Key};

use crate::kit::{Ctx, Game, Input, BAR_H, W};
use crate::pen::{c, hex, rgba, Pen};

/// In the engine.
pub const PORTED: bool = true;

/// A fresh game.
pub fn new() -> Box<dyn Game> {
    Box::new(Doors::default())
}

/// The doorway: x0, x1, y0, y1.
const OPEN: [f32; 4] = [250.0, 650.0, 170.0, 680.0];
/// How far the leaf slides into its pocket, px.
const TRAVEL: f32 = 380.0;
/// The crank's hub.
const HUB: [f32; 2] = [1060.0, 520.0];
/// The crank wheel's radius, px.
const WHEEL: f32 = 130.0;
/// The strain gauge's centre and radius.
const GAUGE: [f32; 3] = [1060.0, 250.0, 118.0];
/// The gauge's full scale: the slip is at 1.
const GAUGE_MAX: f32 = 1.3;
/// The crank speed that strains it to its limit at rest, rad/s.
const W_REF: f32 = 8.0;
/// Where the new pinion waits in its crate.
const PINION_HOME: [f32; 2] = [835.0, 635.0];
/// How many waves a round lays out ahead (more than any round lasts).
const WAVES: usize = 40;

#[derive(Clone, Copy, Debug, Default)]
struct Wave {
    t: f32,
    d: f32,
}

/// The game's state.
#[derive(Default)]
pub struct Doors {
    part: bool,
    pinion: [f32; 2],
    pinion_held: bool,
    pinion_set: bool,
    crank: f32,
    omega: f32,
    strain: f32,
    prog: f32,
    shown: f32,
    turns: f32,
    waves: Vec<Wave>,
    amp: f32,
    rt: f32,
    lock: f32,
    drag_a: Option<f32>,
    last_key: u8,
    played: bool,
    wobble: f32,
    /// The hand: whether it is carrying the pinion.
    hand_carry: bool,
    /// The hand: its angle round the hub, and whether it has the crank.
    hand_a: f32,
    hand_on: bool,
}

fn wrap(a: f32) -> f32 {
    a.sin().atan2(a.cos())
}

impl Doors {
    /// The leaf's resistance at `time`: 1 at rest, rising through each wave.
    fn resistance(&self, time: f32) -> f32 {
        let mut b: f32 = 0.0;
        for w in &self.waves {
            if time >= w.t && time < w.t + w.d {
                b = b.max((PI * (time - w.t) / w.d).sin().powi(2));
            }
        }
        1.0 + self.amp * b
    }
}

/// A toothed wheel: the pinion, and the track's gear.
#[allow(clippy::too_many_arguments)]
fn gear(g: &Pen, x: f32, y: f32, r: f32, a: f32, teeth: usize, fill: Color32, stroke: Color32) {
    let n = teeth * 2;
    let mut pts = Vec::with_capacity(n * 2);
    for i in 0..n {
        let ang = a + (i as f32 / n as f32) * TAU;
        let rr = if i % 2 == 1 { r } else { r + 7.0 };
        let a2 = ang + PI / n as f32;
        pts.push([x + ang.cos() * rr, y + ang.sin() * rr]);
        pts.push([x + a2.cos() * rr, y + a2.sin() * rr]);
    }
    g.poly(&pts, fill);
    g.path(&pts, true, 2.0, stroke);
}

/// A dashed polyline whose dashes start `offset` px into the pattern (canvas's `lineDashOffset`), so a chain can run.
#[allow(clippy::too_many_arguments)]
fn chain(g: &Pen, pts: &[[f32; 2]], width: f32, col: Color32, dash: f32, gap: f32, offset: f32) {
    let period = dash + gap;
    let mut d0 = 0.0;
    for w in pts.windows(2) {
        let ([x0, y0], [x1, y1]) = (w[0], w[1]);
        let l = (x1 - x0).hypot(y1 - y0);
        if l <= 0.0 {
            continue;
        }
        // The first dash that ends past this segment's start.
        let ph = (d0 + offset).rem_euclid(period);
        let mut s = -ph;
        while s < l {
            let (a, b) = (s.max(0.0), (s + dash).min(l));
            if b > a {
                let (ka, kb) = (a / l, b / l);
                g.line(x0 + (x1 - x0) * ka, y0 + (y1 - y0) * ka, x0 + (x1 - x0) * kb, y0 + (y1 - y0) * kb, width, col);
            }
            s += period;
        }
        d0 += l;
    }
}

impl Game for Doors {
    fn id(&self) -> &'static str {
        "doors"
    }

    fn knobs(&self) -> &'static [&'static str] {
        &["turns_count", "wave_gain_x", "wave_gap_s"]
    }

    fn step(&mut self, cx: &mut Ctx) {
        let mut r = cx.dice();
        self.part = cx.part;
        self.pinion = PINION_HOME;
        self.pinion_held = false;
        self.pinion_set = false;
        self.turns = TAU * cx.knob("turns_count");
        self.amp = cx.knob("wave_gain_x");
        let gap = cx.knob("wave_gap_s");
        self.waves.clear();
        let mut tt = 1.0 + r.f();
        for _ in 0..WAVES {
            let d = 1.2 + 0.4 * r.f();
            self.waves.push(Wave { t: tt, d });
            tt += d + gap + r.f() * 1.2;
        }
        self.crank = 0.0;
        self.omega = 0.0;
        self.strain = 0.0;
        self.prog = 0.0;
        self.shown = 0.0;
        self.rt = 0.0;
        self.lock = 0.0;
        self.drag_a = None;
        self.last_key = 0;
        self.played = false;
        self.wobble = 0.0;
        self.hand_carry = false;
        self.hand_on = false;
    }

    fn update(&mut self, cx: &mut Ctx, dt: f32, input: &Input) {
        if self.part && !self.pinion_set {
            if input.pressed && input.dist(self.pinion[0], self.pinion[1]) < 50.0 {
                self.pinion_held = true;
            }
            if self.pinion_held && input.down {
                self.pinion = [input.x, input.y];
            }
            if self.pinion_held && input.released {
                self.pinion_held = false;
                if (self.pinion[0] - HUB[0]).hypot(self.pinion[1] - HUB[1]) < 45.0 {
                    self.pinion_set = true;
                    self.pinion = HUB;
                    cx.step_done();
                }
            }
            return;
        }
        self.shown += (self.prog - self.shown) * (dt * 6.0).min(1.0);
        if self.played || self.part {
            self.strain = (self.strain - dt * 2.0).max(0.0);
            return;
        }
        self.rt += dt;
        let res = self.resistance(self.rt);
        self.wobble = res - 1.0;
        if self.lock > 0.0 {
            self.lock -= dt;
            self.omega = 0.0;
            self.strain = (self.strain - dt * 2.0).max(0.0);
            return;
        }
        // The hands: a drag round the hub (clockwise only: the ratchet holds the other way), or alternate keys. A press
        // takes the crank, and so does a hand still on it once a slip's lock or a step change has let it go.
        let turned;
        if (input.pressed || (self.drag_a.is_none() && input.down)) && input.dist(HUB[0], HUB[1]) < 220.0 {
            self.drag_a = Some((input.y - HUB[1]).atan2(input.x - HUB[0]));
        }
        match self.drag_a {
            Some(prev) if input.down => {
                let a = (input.y - HUB[1]).atan2(input.x - HUB[0]);
                turned = wrap(a - prev).max(0.0);
                self.drag_a = Some(a);
                self.omega += (turned / dt.max(1e-3) - self.omega) * (dt / 0.1).min(1.0);
            }
            _ => {
                self.drag_a = None;
                for (k, side) in [(Key::ArrowLeft, 1u8), (Key::A, 1), (Key::ArrowRight, 2), (Key::D, 2)] {
                    if input.hit(k) && side != self.last_key {
                        self.omega += 1.8;
                        self.last_key = side;
                    }
                }
                self.omega *= (-2.5 * dt).exp();
                turned = self.omega * dt;
            }
        }
        self.crank += turned;
        self.prog = (self.prog + turned / self.turns).min(1.0);
        // The strain: how hard the hands drive the leaf against its resistance now.
        self.strain += (res * self.omega / W_REF - self.strain) * (dt / 0.12).min(1.0);
        if self.strain > 1.0 {
            // Slipped: the leaf drops back and the crank spins free for a moment.
            self.prog = (self.prog - 0.2).max(0.0);
            self.lock = 0.8;
            self.omega = 0.0;
            self.drag_a = None;
            self.strain = 1.15;
            cx.fumble("The leaf drops back");
            return;
        }
        if self.prog >= 1.0 {
            self.played = true;
            self.omega = 0.0;
            cx.step_done();
        }
    }

    fn draw(&self, g: &Pen, _cx: &Ctx, t: f32, _input: &Input) {
        let [x0, x1, y0, y1] = OPEN;
        g.rect(0.0, BAR_H, W, 720.0, hex(0x070b12));
        // Beyond the door: the next corridor, lit.
        g.gradient(x0, y0, x1 - x0, y1 - y0, hex(0x1c2a3c), hex(0x0a1018), true);
        g.rect(x0 + 120.0, y0 + 30.0, 160.0, 10.0, rgba(255, 226, 170, 0.55));
        g.rect(x0, y1 - 120.0, x1 - x0, 120.0, hex(0x18222f));
        // The leaf, slid along by the progress, shuddering through a wave.
        let sh = if !self.played && !self.part { (t * 47.0).sin() * 2.5 * (self.wobble / 2.0).min(1.0) } else { 0.0 };
        let lx = x0 - TRAVEL * self.shown + sh;
        g.rect(lx, y0, x1 - x0, y1 - y0, hex(0x4a5568));
        for yy in [260.0, 420.0, 560.0] {
            g.rect(lx + 20.0, yy, x1 - x0 - 70.0, 16.0, hex(0x56637a));
        }
        g.round(lx + 120.0, 210.0, 150.0, 34.0, 10.0, Some(hex(0x0e1622)), Some((3.0, c::STEEL)));
        // Hazard chevrons on the leading edge.
        let hz = g.clip(lx + 360.0, y0, 40.0, y1 - y0);
        hz.rect(lx + 360.0, y0, 40.0, y1 - y0, hex(0x1a1408));
        let mut y = y0 - 40.0;
        while y < y1 {
            hz.poly(
                &[[lx + 360.0, y], [lx + 400.0, y + 20.0], [lx + 400.0, y + 40.0], [lx + 360.0, y + 20.0]],
                c::AMBER,
            );
            y += 40.0;
        }
        // The wall around it, covering the pocket the leaf slides into.
        let wall = hex(0x141b27);
        g.rect(0.0, BAR_H, x0, 720.0, wall);
        g.rect(x1, BAR_H, 800.0 - x1, 720.0, wall);
        g.rect(0.0, BAR_H, 800.0, y0 - BAR_H, wall);
        g.rect(0.0, y1, 800.0, 720.0 - y1, wall);
        let mut x = 20.0;
        while x < 800.0 {
            if x < x0 - 30.0 || x > x1 + 10.0 {
                g.rect(x, 200.0, 6.0, 470.0, hex(0x1a2232));
            }
            x += 120.0;
        }
        // The frame: jambs and a lintel standing proud of the wall.
        g.rect_stroke(x0 - 6.0, y0 - 6.0, x1 - x0 + 12.0, y1 - y0 + 12.0, 6.0, c::STEEL);
        // The track over the opening with its hangers and the gear that drives it.
        g.rect(0.0, 128.0, 720.0, 16.0, hex(0x2a3446));
        for hx in [lx + 60.0, lx + 340.0] {
            if hx > 0.0 {
                g.disc(hx, 136.0, 10.0, hex(0x8792a3));
                g.rect(hx - 3.0, 136.0, 6.0, y0 - 136.0, hex(0x56637a));
            }
        }
        gear(g, 700.0, 136.0, 20.0, self.crank * 0.6, 10, hex(0x3a4558), c::STEEL);
        // The lamp over the door: red and crossed while stuck, green and ticked once open.
        let open = self.prog >= 1.0;
        g.disc(450.0, 108.0, 16.0, if open { c::OK } else { c::DANGER });
        let ink = hex(0x0b0f15);
        if open {
            g.path(&[[442.0, 108.0], [448.0, 115.0], [459.0, 101.0]], false, 4.0, ink);
        } else {
            g.line(443.0, 101.0, 457.0, 115.0, 4.0, ink);
            g.line(457.0, 101.0, 443.0, 115.0, 4.0, ink);
        }
        // The drive: the chain from the track's gear down the wall and across to the crank.
        chain(
            g,
            &[[720.0, 136.0], [760.0, 136.0], [760.0, HUB[1]], [HUB[0], HUB[1]]],
            8.0,
            hex(0x56637a),
            10.0,
            6.0,
            -self.crank * 20.0,
        );
        // The crank.
        g.panel(900.0, 380.0, 320.0, 300.0, 18.0, hex(0x0f1620), c::LINE);
        let [hx, hy] = HUB;
        if !self.part || self.pinion_set {
            g.ring(hx, hy, WHEEL, if self.lock > 0.0 { c::DANGER } else { hex(0x8792a3) }, 12.0);
            for i in 0..3 {
                let a = self.crank + (i as f32 / 3.0) * TAU;
                g.line(hx, hy, hx + a.cos() * WHEEL, hy + a.sin() * WHEEL, 10.0, hex(0x56637a));
            }
            gear(g, hx, hy, 30.0, self.crank, 12, hex(0x3a4558), c::STEEL);
            let ka = self.crank;
            let knob = if self.drag_a.is_some() { c::AMBER } else { hex(0xc9d1dc) };
            g.disc(hx + ka.cos() * (WHEEL - 22.0), hy + ka.sin() * (WHEEL - 22.0), 22.0, knob);
            // The way it turns: an arrow on the rim.
            g.turn_arrow(hx, hy, WHEEL + 22.0, -PI / 2.0 - 0.5, -PI / 2.0 + 0.4, c::DIM);
        } else {
            g.dashed(&Pen::arc_points(hx, hy, 40.0, 0.0, TAU), 4.0, c::AMBER, 8.0, 6.0);
            g.disc(hx, hy, 12.0, hex(0x3a4558));
        }
        // The strain gauge: green, amber near the limit, hatched red past it, the needle.
        let [gx, gy, gr] = GAUGE;
        let ang = |v: f32| PI + PI * (v / GAUGE_MAX).clamp(0.0, 1.0);
        g.panel(920.0, 110.0, 280.0, 170.0, 18.0, hex(0x0f1620), c::LINE);
        g.arc(gx, gy, gr - 20.0, ang(0.0), ang(0.75), 22.0, rgba(61, 220, 132, 0.45));
        g.arc(gx, gy, gr - 20.0, ang(0.75), ang(1.0), 22.0, rgba(242, 160, 70, 0.7));
        g.hatch_arc(gx, gy, gr - 31.0, gr - 9.0, ang(1.0), ang(GAUGE_MAX), rgba(255, 71, 87, 0.9));
        let la = ang(1.0);
        g.line(
            gx + la.cos() * (gr - 36.0),
            gy + la.sin() * (gr - 36.0),
            gx + la.cos() * (gr + 2.0),
            gy + la.sin() * (gr + 2.0),
            5.0,
            c::DANGER,
        );
        let na = ang(self.strain);
        let ncol = if self.strain > 1.0 { c::DANGER } else { c::FG };
        g.path_round(&[[gx, gy], [gx + na.cos() * (gr - 14.0), gy + na.sin() * (gr - 14.0)]], 5.0, ncol);
        g.disc(gx, gy, 10.0, hex(0x3a4558));
        // The part step: the crate with the new pinion.
        if self.part && !self.pinion_set {
            g.panel(775.0, 575.0, 120.0, 120.0, 14.0, hex(0x141b27), c::LINE);
            let [px, py] = self.pinion;
            gear(g, px, py, 30.0, 0.0, 12, c::COPPER, hex(0xf0c08a));
            g.disc(px, py, 9.0, hex(0x7a4a22));
        }
    }

    fn hand(&mut self, _cx: &Ctx, _t: f32) -> Input {
        if self.part && !self.pinion_set {
            // Pick the pinion up, carry it to the shaft in a few frames, let go there.
            if !self.pinion_held && !self.hand_carry {
                self.hand_carry = true;
                return Input::hold(self.pinion[0], self.pinion[1], true);
            }
            let d = (HUB[0] - self.pinion[0]).hypot(HUB[1] - self.pinion[1]);
            if d > 2.0 {
                let k = (6.0 / d).min(1.0);
                let (x, y) =
                    (self.pinion[0] + (HUB[0] - self.pinion[0]) * k, self.pinion[1] + (HUB[1] - self.pinion[1]) * k);
                return Input::hold(x, y, false);
            }
            self.hand_carry = false;
            return Input::release(HUB[0], HUB[1]);
        }
        if self.played || self.part {
            self.hand_on = false;
            return Input::default();
        }
        // Wind on while the leaf is easy, ease off as a wave builds: the speed that keeps the strain at 70% of the
        // slip, judged on the worst resistance of the next 0.3 s (the crank and the gauge both lag).
        let worst = (0..=6).map(|k| self.resistance(self.rt + k as f32 * 0.05)).fold(1.0f32, f32::max);
        let speed = if self.lock > 0.0 { 0.0 } else { 0.7 * W_REF / worst };
        let pressed = !self.hand_on;
        if self.hand_on {
            self.hand_a += speed / 60.0;
        } else {
            self.hand_a = self.crank;
            self.hand_on = true;
        }
        let r = WHEEL - 22.0;
        Input::hold(HUB[0] + self.hand_a.cos() * r, HUB[1] + self.hand_a.sin() * r, pressed)
    }
}

#[cfg(test)]
mod tests {
    use crate::games::tests::{fumble_check, plays_to_end};
    use crate::kit::Input;

    #[test]
    fn a_steady_hand_repairs_it_damaged_disabled_and_destroyed() {
        plays_to_end("doors");
    }

    #[test]
    fn cranking_hard_into_the_leaf_slips_it() {
        // Spin the crank at 30 rad/s: the strain is far past the slip at once.
        fumble_check("doors", |r| {
            let a = r.t * 30.0;
            Input::hold(1060.0 + a.cos() * 100.0, 520.0 + a.sin() * 100.0, false)
        });
    }
}
