//! The Gannet tubes' repair (repair-minigames design 2; the missile and its tube states are weapons-and-shields
//! section 10), from `docs/mockups/repairs/tubes.js`.
//!
//! The magazine's breech: above, the tube side on with a Gannet on its ready tray; below left, the breech face; below
//! right, the load panel's three big switches. First clear the jam: turn the locking ring (drag around it, or Left and
//! Right) feeling for each hidden notch. The pawl at the top rises as the ring nears a notch and drops into it once
//! the ring rests there; turning fast runs past it. The lamps beside the face count the notches. With the last notch
//! found the ring lets go and the breech door swings open. Then load and arm, in order: RAIL (the guide rail lowers
//! and the missile runs in), LATCH (only once the missile sits home against its stop; it bounces before it settles),
//! INTERLOCK (the tube arms). Click a switch, or Left and Right to pick and Space to throw. A switch out of order is a
//! fumble: the interlock trips and the latch and interlock fall back off. Each level hides more notches in narrower
//! windows and bounces the missile longer, and from level 3 the switches are shuffled on the panel. A disabled tube's
//! first step fits a new breech seal: drag it from the crate onto the breech face.

use egui::Key;

use crate::games::breakers::{dashed_ring, wrap, Hand, PART_STEP};
use crate::kit::{Ctx, Game, Input, BAR_H, W};
use crate::pen::{c, hex, rgba, Align, Pen};

/// In the engine.
pub const PORTED: bool = true;

/// A fresh game.
pub fn new() -> Box<dyn Game> {
    Box::new(Tubes::default())
}

/// The missile's axis, side view.
const MY: f32 = 220.0;
/// The tube: x0, x1, top, bottom.
const TUBE: [f32; 4] = [600.0, 1240.0, 180.0, 260.0];
const M_LEN: f32 = 380.0;
/// The missile's tail x on the tray, and home in the tube.
const M_START: f32 = 120.0;
const M_HOME: f32 = 650.0;
/// The breech face: x, y, r.
const FACE: [f32; 3] = [300.0, 545.0, 150.0];
/// The switches: top, bottom, pivot, width.
const SW_Y0: f32 = 400.0;
const SW_Y1: f32 = 640.0;
const SW_PIVOT: f32 = 520.0;
const SW_W: f32 = 140.0;
/// Seconds at rest in a notch's window for the pawl to drop.
const DWELL_S: f32 = 0.4;

fn slot_x(s: usize) -> f32 {
    730.0 + s as f32 * 200.0
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Phase {
    #[default]
    Part,
    Sealed,
    Jam,
    Open,
    Load,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Sw {
    Rail,
    Latch,
    Arm,
}

impl Sw {
    fn name(self) -> &'static str {
        match self {
            Sw::Rail => "RAIL",
            Sw::Latch => "LATCH",
            Sw::Arm => "INTERLOCK",
        }
    }
}

/// The game's state.
#[derive(Default)]
pub struct Tubes {
    part: bool,
    seal: [f32; 2],
    seal_held: bool,
    seal_set: bool,
    phase: Phase,
    notches: Vec<f32>,
    found: usize,
    ring_a: f32,
    ring_lo: f32,
    dwell: f32,
    drag_a: Option<f32>,
    ring_shake: f32,
    open_t: f32,
    door_t: f32,
    window: f32,
    push: f32,
    bounce: f32,
    slots: Vec<Sw>,
    focus: usize,
    on: [bool; 3],
    rail_t: f32,
    latch_t: f32,
    mx: f32,
    mv: f32,
    trip_t: f32,
    trip: Option<Sw>,
    played: bool,
    arm_t: f32,
    hand: Hand,
}

impl Tubes {
    fn is_on(&self, s: Sw) -> bool {
        self.on[s as usize]
    }
    fn home(&self) -> bool {
        self.mx >= M_HOME - 3.0
    }

    /// Throw switch `s`: in order it acts, out of order it trips the interlock.
    fn throw(&mut self, cx: &mut Ctx, s: Sw) {
        if self.is_on(s) || self.played {
            return;
        }
        let ok = match s {
            Sw::Rail => true,
            Sw::Latch => self.is_on(Sw::Rail) && self.rail_t >= 1.0 && self.home(),
            Sw::Arm => self.is_on(Sw::Latch),
        };
        if !ok {
            self.on[Sw::Latch as usize] = false;
            self.on[Sw::Arm as usize] = false;
            self.latch_t = self.latch_t.min(1.0);
            self.trip = Some(s);
            self.trip_t = 0.9;
            cx.fumble("Interlock tripped: back to the latch");
            return;
        }
        self.on[s as usize] = true;
        if s == Sw::Latch {
            self.mv = 0.0;
        }
        if s == Sw::Arm {
            self.played = true;
            self.arm_t = 0.0;
            cx.step_done();
        }
    }
}

impl Game for Tubes {
    fn id(&self) -> &'static str {
        "tubes"
    }

    fn knobs(&self) -> &'static [&'static str] {
        &["notches_count", "notch_window_rad", "shuffle_flag", "push_px_s2", "bounce_frac"]
    }

    fn step(&mut self, cx: &mut Ctx) {
        let mut r = cx.dice();
        self.part = cx.part;
        self.seal = [1080.0, 540.0];
        self.seal_held = false;
        self.seal_set = false;
        self.phase = if self.part { Phase::Part } else { Phase::Jam };
        // Hidden notches, clockwise from the ring's rest.
        let k = cx.knob("notches_count").round().max(1.0) as usize;
        let mut a = 0.0;
        self.notches = (0..k)
            .map(|_| {
                a += 0.6 + r.f() * 0.9;
                a
            })
            .collect();
        self.found = 0;
        self.ring_a = 0.0;
        self.ring_lo = -0.3;
        self.dwell = 0.0;
        self.drag_a = None;
        self.ring_shake = 0.0;
        self.open_t = 0.0;
        self.door_t = 0.0;
        self.window = cx.knob("notch_window_rad");
        self.push = cx.knob("push_px_s2");
        self.bounce = cx.knob("bounce_frac");
        // The panel: in order at first, shuffled once the hands know it.
        self.slots = vec![Sw::Rail, Sw::Latch, Sw::Arm];
        if cx.knob("shuffle_flag") >= 0.5 {
            r.shuffle(&mut self.slots);
        }
        self.focus = 0;
        self.on = [false; 3];
        self.rail_t = 0.0;
        self.latch_t = 0.0;
        self.mx = M_START;
        self.mv = 0.0;
        self.trip_t = 0.0;
        self.trip = None;
        self.played = false;
        self.arm_t = 0.0;
        self.hand = Hand::default();
    }

    fn update(&mut self, cx: &mut Ctx, dt: f32, input: &Input) {
        self.ring_shake = (self.ring_shake - dt).max(0.0);
        self.trip_t = (self.trip_t - dt).max(0.0);
        let [fx, fy, _] = FACE;
        match self.phase {
            Phase::Part => {
                if input.pressed && input.dist(self.seal[0], self.seal[1]) < 110.0 {
                    self.seal_held = true;
                }
                if self.seal_held && input.down {
                    self.seal = [input.x, input.y];
                }
                if self.seal_held && input.released {
                    self.seal_held = false;
                    if (self.seal[0] - fx).hypot(self.seal[1] - fy) < 50.0 {
                        self.seal_set = true;
                        self.seal = [fx, fy];
                        self.phase = Phase::Sealed;
                        cx.step_done();
                    }
                }
            }
            Phase::Sealed => {}
            Phase::Jam => {
                let last = self.notches[self.notches.len() - 1];
                if input.pressed {
                    let d = input.dist(fx, fy);
                    if d > 40.0 && d < 210.0 {
                        self.drag_a = Some((input.y - fy).atan2(input.x - fx));
                    }
                }
                let mut turn = 0.0;
                match (self.drag_a, input.down) {
                    (Some(d), true) => {
                        let a = (input.y - fy).atan2(input.x - fx);
                        turn = wrap(a - d);
                        self.drag_a = Some(a);
                    }
                    _ => self.drag_a = None,
                }
                turn += input.stick[0] * 0.9 * dt;
                self.ring_a = (self.ring_a + turn).clamp(self.ring_lo, last + 0.35);
                let speed = turn.abs() / dt.max(1e-3);
                let in_win = (self.ring_a - self.notches[self.found]).abs() < self.window;
                // The pawl drops only while the ring rests in the window.
                if in_win && speed < 1.2 {
                    self.dwell += dt;
                } else {
                    self.dwell = (self.dwell - 2.0 * dt).max(0.0);
                }
                if self.dwell >= DWELL_S {
                    self.ring_a = self.notches[self.found];
                    self.ring_lo = self.ring_a;
                    self.found += 1;
                    self.dwell = 0.0;
                    self.ring_shake = 0.3;
                    if self.found == self.notches.len() {
                        self.phase = Phase::Open;
                        self.open_t = 0.0;
                    }
                }
            }
            Phase::Open => {
                // The ring lets go and turns to its stop; the breech door swings up.
                self.open_t += dt;
                let last = self.notches[self.notches.len() - 1];
                self.ring_a = (last + 0.35).min(self.ring_a + dt * 0.8);
                self.door_t = ((self.open_t - 0.4) / 0.6).clamp(0.0, 1.0);
                if self.door_t >= 1.0 {
                    self.phase = Phase::Load;
                }
            }
            Phase::Load => {
                // The panel.
                if input.hit(Key::ArrowLeft) || input.hit(Key::A) {
                    self.focus = (self.focus + 2) % 3;
                }
                if input.hit(Key::ArrowRight) || input.hit(Key::D) {
                    self.focus = (self.focus + 1) % 3;
                }
                if input.action_pressed {
                    let s = self.slots[self.focus];
                    self.throw(cx, s);
                    return;
                }
                if input.pressed && input.y > SW_Y0 && input.y < SW_Y1 + 30.0 {
                    if let Some(s) = (0..3).find(|&s| (input.x - slot_x(s)).abs() < SW_W / 2.0) {
                        self.focus = s;
                        let sw = self.slots[s];
                        self.throw(cx, sw);
                        return;
                    }
                }
                // The movers: the rail lowers, the missile runs in and bounces at its stop, the latch swings down.
                if self.is_on(Sw::Rail) {
                    self.rail_t = (self.rail_t + dt / 0.6).min(1.0);
                }
                self.latch_t = if self.is_on(Sw::Latch) {
                    (self.latch_t + dt / 0.35).min(1.0)
                } else {
                    (self.latch_t - dt / 0.35).max(0.0)
                };
                if self.rail_t >= 1.0 && !self.is_on(Sw::Latch) {
                    if self.mx < M_HOME {
                        self.mv = 520.0f32.min(self.mv + self.push * dt);
                    }
                    self.mx += self.mv * dt;
                    if self.mx >= M_HOME {
                        self.mx = M_HOME;
                        self.mv = if self.mv > 50.0 { -self.mv * self.bounce } else { 0.0 };
                    }
                }
                if self.played {
                    self.arm_t += dt;
                    self.door_t = (1.0 - self.arm_t / 0.6).max(0.0);
                }
            }
        }
    }

    fn draw(&self, g: &Pen, _cx: &Ctx, t: f32, _input: &Input) {
        use std::f32::consts::{FRAC_PI_2, TAU};
        g.rect(0.0, BAR_H, W, 720.0, hex(0x070b12));
        // ---- Side view: the tray, the rail, the tube cut away, the missile, the latch, the breech door.
        g.panel(30.0, 100.0, 1220.0, 240.0, 18.0, hex(0x0b111b), c::LINE);
        // The ready tray and its legs.
        g.rect(80.0, 262.0, 500.0, 14.0, hex(0x283246));
        for x in [120.0, 300.0, 480.0] {
            g.rect(x, 276.0, 16.0, 50.0, hex(0x283246));
        }
        let mut x = 100.0;
        while x < 580.0 {
            g.disc(x, 258.0, 6.0, hex(0x3a4558));
            x += 40.0;
        }
        // The tube's bore, dark, then the rail inside it.
        let [tx0, tx1, top, bot] = TUBE;
        g.rect(tx0, top, tx1 - tx0, bot - top, hex(0x05080d));
        let rail_y = 252.0;
        g.rect(680.0, rail_y, 540.0, 6.0, if self.is_on(Sw::Rail) { hex(0x55617a) } else { hex(0x3a4558) });
        // The bridging section: tilted up until thrown, level once down.
        g.translate(560.0, rail_y + 3.0).rotate(-0.35 * (1.0 - self.rail_t)).rect(
            0.0,
            -3.0,
            120.0,
            6.0,
            if self.rail_t >= 1.0 { hex(0x55617a) } else { c::AMBER },
        );
        // The missile.
        let shake = if self.ring_shake > 0.0 { (t * 80.0).sin() * 2.0 } else { 0.0 };
        let x = self.mx;
        g.rect(x, MY - 26.0, M_LEN - 50.0, 52.0, hex(0xaab3c0));
        g.poly(&[[x + M_LEN - 50.0, MY - 26.0], [x + M_LEN, MY], [x + M_LEN - 50.0, MY + 26.0]], hex(0xaab3c0));
        g.rect(x + M_LEN - 90.0, MY - 26.0, 16.0, 52.0, c::AMBER);
        for sg in [-1.0, 1.0] {
            g.poly(
                &[[x, MY + sg * 26.0], [x + 50.0, MY + sg * 26.0], [x + 30.0, MY + sg * 38.0], [x, MY + sg * 38.0]],
                hex(0x6f7a8a),
            );
        }
        g.rect(x - 10.0, MY - 14.0, 10.0, 28.0, hex(0x3a4558));
        // The tube's walls drawn over the bore's edges.
        g.rect(tx0, top - 16.0, tx1 - tx0, 16.0, hex(0x2a3446));
        g.rect(tx0, bot, tx1 - tx0, 16.0, hex(0x2a3446));
        let mut xx = tx0 + 60.0;
        while xx < tx1 {
            g.rect(xx, top - 16.0, 8.0, 16.0, hex(0x364257));
            g.rect(xx, bot, 8.0, 16.0, hex(0x364257));
            xx += 120.0;
        }
        // Interlock lamps on the tube: hollow while safe, filled green when armed.
        for i in 0..3 {
            let (lx, ly) = (1100.0 + i as f32 * 40.0, 140.0);
            if self.is_on(Sw::Arm) {
                g.disc(lx, ly, 11.0, c::OK);
            } else {
                g.ring(lx, ly, 10.0, hex(0x3a4558), 4.0);
            }
        }
        // The breech flange and its door (hinged at the top, swings up and aft, clear of the latch).
        g.rect(588.0, 150.0, 24.0, 140.0, c::STEEL);
        let door = if self.door_t > 0.0 && self.door_t < 1.0 { hex(0x8c96a6) } else { hex(0x5d6879) };
        g.translate(600.0, 160.0).rotate(self.door_t * 1.9).rect(-10.0, 0.0, 20.0, 100.0, door);
        // The latch: an arm from a pivot above the breech, swinging down behind the tail.
        let la = -0.6 + (FRAC_PI_2 + 0.6) * self.latch_t;
        let (px, py) = (628.0, 150.0);
        let (lx, ly) = (px + la.cos() * 70.0, py + la.sin() * 70.0);
        g.path_round(&[[px, py], [lx, ly]], 10.0, if self.is_on(Sw::Latch) { c::OK } else { hex(0x8792a3) });
        g.disc(px, py, 9.0, hex(0x3a4558));
        // ---- The breech face: the housing, the locking ring, the pawl, the notch lamps.
        let [fx, fy, fr] = FACE;
        let dim_face = self.phase == Phase::Load || self.phase == Phase::Sealed;
        let f = g.alpha(if dim_face { 0.55 } else { 1.0 }).translate(shake, 0.0);
        f.disc(fx, fy, fr, hex(0x141b27));
        f.ring(fx, fy, fr, c::STEEL, 4.0);
        for i in 0..12 {
            let a = i as f32 / 12.0 * TAU;
            f.disc(fx + a.cos() * (fr - 12.0), fy + a.sin() * (fr - 12.0), 5.0, hex(0x4a5568));
        }
        // The locking ring and its lugs, turned by its angle.
        f.ring(fx, fy, 112.0, hex(0x2f3a4d), 40.0);
        for i in 0..8 {
            let a = self.ring_a + i as f32 / 8.0 * TAU;
            f.translate(fx + a.cos() * 112.0, fy + a.sin() * 112.0).rotate(a).rect(
                -14.0,
                -16.0,
                28.0,
                32.0,
                if i == 0 { c::ACCENT } else { hex(0x56637a) },
            );
        }
        // The bore: the missile's tail when one is home, the new seal's groove on the part step.
        f.disc(fx, fy, 80.0, hex(0x05080d));
        if self.part && !self.seal_set {
            dashed_ring(&f, fx, fy, 86.0, c::AMBER, 4.0, 10.0, 8.0);
        }
        if self.seal_set {
            f.ring(fx, fy, 86.0, hex(0x2b2f38), 12.0);
        }
        if self.home() {
            f.disc(fx, fy, 56.0, hex(0x6f7a8a));
            f.disc(fx, fy, 20.0, hex(0x3a4558));
        }
        // The pawl at the top: it rises as the ring nears a notch and drops in at rest.
        if self.phase == Phase::Jam {
            let near = (1.0 - (self.ring_a - self.notches[self.found]).abs() / 0.6).clamp(0.0, 1.0);
            let lift = near * near * 22.0 * (1.0 - self.dwell / DWELL_S);
            let top = fy - 136.0 - lift;
            let pc = if self.dwell > 0.0 {
                c::AMBER
            } else if near > 0.5 {
                hex(0xe8c28a)
            } else {
                c::STEEL
            };
            f.poly(&[[fx - 16.0, top - 26.0], [fx + 16.0, top - 26.0], [fx, top]], pc);
            // The feel: an arc beside the pawl, its length how near the notch is.
            if near > 0.0 {
                f.arc(
                    fx,
                    fy,
                    fr + 18.0,
                    -FRAC_PI_2 - 0.5 * near,
                    -FRAC_PI_2 + 0.5 * near,
                    8.0,
                    rgba(242, 160, 70, 0.3 + 0.7 * near),
                );
            }
            // The way it turns: an arrow on the rim.
            f.turn_arrow(fx, fy, fr + 18.0, -FRAC_PI_2 + 0.7, -FRAC_PI_2 + 1.3, c::DIM);
        }
        // The notch lamps: hollow for one still to find, filled green once found.
        if !self.part {
            for i in 0..self.notches.len() {
                let (lx, ly) = (500.0, 440.0 + i as f32 * 46.0);
                if i < self.found {
                    g.disc(lx, ly, 14.0, c::OK);
                } else {
                    let col = if i == self.found && self.phase == Phase::Jam { c::AMBER } else { hex(0x3a4558) };
                    g.ring(lx, ly, 12.0, col, 4.0);
                }
            }
        }
        // ---- The load panel, or the crate on the part step.
        if self.phase == Phase::Part || (self.part && self.seal_set) {
            if !self.seal_set {
                g.panel(960.0, 420.0, 240.0, 240.0, 16.0, hex(0x141b27), c::LINE);
                g.ring(self.seal[0], self.seal[1], 86.0, hex(0x2b2f38), 16.0);
                g.ring(self.seal[0], self.seal[1], 86.0, c::AMBER, 2.0);
            }
            return;
        }
        let live = self.phase == Phase::Load;
        let p = g.alpha(if live { 1.0 } else { 0.4 });
        p.panel(620.0, 380.0, 620.0, 320.0, 18.0, hex(0x0f1620), c::LINE);
        for (s, &sw) in self.slots.iter().enumerate() {
            let cx = slot_x(s);
            let tripped = self.trip_t > 0.0 && self.trip == Some(sw);
            let on = self.is_on(sw);
            p.round(
                cx - SW_W / 2.0,
                SW_Y0,
                SW_W,
                SW_Y1 - SW_Y0,
                14.0,
                Some(if tripped { hex(0x3a1218) } else { hex(0x161f2c) }),
                Some((3.0, if live && self.focus == s { c::AMBER } else { c::LINE })),
            );
            // The lever: up when off, down when thrown.
            let ey = SW_PIVOT + if on { 85.0 } else { -85.0 };
            p.path_round(&[[cx, SW_PIVOT], [cx, ey]], 16.0, hex(0x8792a3));
            p.disc(cx, SW_PIVOT, 20.0, hex(0x3a4558));
            let knob = if on {
                c::OK
            } else if tripped {
                c::DANGER
            } else {
                hex(0xc9d1dc)
            };
            p.disc(cx, ey, 26.0, knob);
            if on {
                p.path(&[[cx - 11.0, ey], [cx - 3.0, ey + 9.0], [cx + 12.0, ey - 9.0]], false, 5.0, hex(0x0b2a18));
            } else if tripped {
                p.cross(cx, ey, 9.0, 5.0, hex(0x2a0a0e));
            }
            p.text(sw.name(), cx, 670.0, 20.0, if on { c::OK } else { c::FG }, Align::Center);
        }
    }

    fn hand(&mut self, _cx: &Ctx, _t: f32) -> Input {
        if self.hand.resting() {
            return Input::default();
        }
        match self.phase {
            Phase::Part => self.hand.carry(1, self.seal, [FACE[0], FACE[1]], PART_STEP),
            Phase::Jam => {
                // Turn the ring on the arrows towards the next notch and rest there while the pawl drops.
                let d = self.notches[self.found] - self.ring_a;
                let tol = self.window * 0.3;
                if d > tol {
                    Input::default().key(Key::ArrowRight, false)
                } else if d < -tol {
                    Input::default().key(Key::ArrowLeft, false)
                } else {
                    Input::default()
                }
            }
            Phase::Load => {
                // RAIL, then LATCH once the missile has settled home, then INTERLOCK.
                let next = if !self.is_on(Sw::Rail) {
                    Some(Sw::Rail)
                } else if !self.is_on(Sw::Latch) {
                    (self.rail_t >= 1.0 && self.mx >= M_HOME && self.mv == 0.0).then_some(Sw::Latch)
                } else if !self.is_on(Sw::Arm) {
                    Some(Sw::Arm)
                } else {
                    None
                };
                match next.and_then(|n| self.slots.iter().position(|&s| s == n)) {
                    Some(s) => self.hand.tap(2 + s as u32, [slot_x(s), SW_PIVOT]),
                    None => self.hand.idle(),
                }
            }
            _ => self.hand.idle(),
        }
    }

    fn guide_now(&self) -> Option<usize> {
        match self.phase {
            _ if self.part => None,
            Phase::Jam => Some(0),
            Phase::Load => Some(2),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::games::tests::{fumble_check, open, plays_to_end};
    use sc_core::repair::State;

    #[test]
    fn a_steady_hand_repairs_it_damaged_disabled_and_destroyed() {
        plays_to_end("tubes");
    }

    #[test]
    fn a_switch_out_of_order_trips_the_interlock() {
        // A twin of the job clears the jam with the steady hand (on the arrows); all the while the pointer presses
        // INTERLOCK (the third switch at level 1), which trips the moment the panel is live.
        let mut twin = open("tubes", State::Damaged);
        fumble_check("tubes", move |_r| {
            let mut i = twin.hand();
            twin.update(1.0 / 60.0, &i);
            i.x = 1130.0;
            i.y = 520.0;
            i.down = true;
            i.pressed = true;
            i
        });
    }
}
