//! The sensor array's repair (repair-minigames design 2), from `docs/mockups/repairs/sensors.js`.
//!
//! The sensor bay with the dish. Left, a small sky map: the dish's aim is the reticle, and the hatched border is its
//! mechanical stops. Beside it the return meter, cold blue to hot amber. Right, the dish itself on its turntable.
//! Below, the scope's trace and three band filters. First sweep: drag on the sky map (the heavy dish slews toward the
//! pointer) or steer with the arrows, read the meter, and rest the reticle on the strongest return until it locks.
//! Pushing the dish into a stop strains it; held there, it jams: a fumble, and the dish will not move for a moment.
//! Then filter: three sliders, LO, MID and HI (drag them, or Left and Right to pick and Up and Down to move), each
//! cancels one band of the noise on the trace (the slow swell, the ripple, the hiss); set all three until the
//! contact's blip stands clean and hold it. Each level narrows the return, tightens the filters and makes the sky
//! noisier, and from level 2 a weaker false return lies elsewhere on the map. A disabled array's first step fits a new
//! feed horn: drag it from the crate to the dish's focus.

use egui::{Color32, Key};

use crate::games::breakers::{dashed_ring, ellipse, Hand, PART_STEP};
use crate::kit::{Ctx, Game, Input, BAR_H, W};
use crate::pen::{c, hex, mix, rgba, Align, Pen};

/// In the engine.
pub const PORTED: bool = true;

/// A fresh game.
pub fn new() -> Box<dyn Game> {
    Box::new(Sensors::default())
}

/// The sky map's panel: x, y, w, h.
const MAP: [f32; 4] = [40.0, 100.0, 580.0, 370.0];
/// The dish's travel inside its stops: x0, x1, y0, y1.
const R: [f32; 4] = [75.0, 585.0, 135.0, 435.0];
const METER: [f32; 4] = [648.0, 112.0, 34.0, 346.0];
const SCOPE: [f32; 4] = [40.0, 495.0, 820.0, 205.0];
/// The band filters: each one's x, and the travel's top and bottom.
const SL_X: [f32; 3] = [950.0, 1065.0, 1180.0];
const SL_Y0: f32 = 520.0;
const SL_Y1: f32 = 660.0;
/// The dish's elevation pivot on its yoke.
const PIVOT: [f32; 2] = [930.0, 300.0];
/// The dish's slew, px of map a second.
const SLEW: f32 = 260.0;
/// The return that locks, the seconds it takes, the seconds the trace must stay clean, the seconds against a stop that
/// jam the dish, and how long it stays jammed.
const LOCK: f32 = 0.92;
const LOCK_S: f32 = 0.8;
const CLEAN_S: f32 = 0.8;
const JAM_AT: f32 = 0.6;
const JAM_S: f32 = 1.5;

fn gauss(dx: f32, dy: f32, s: f32) -> f32 {
    (-(dx * dx + dy * dy) / (2.0 * s * s)).exp()
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Phase {
    #[default]
    Part,
    Fitted,
    Sweep,
    Filter,
}

#[derive(Clone, Copy, Debug, Default)]
struct Band {
    c: f32,
    v: f32,
    p: f32,
}

#[derive(Clone, Copy, Debug, Default)]
struct Speck {
    x: f32,
    y: f32,
    p: f32,
    s: f32,
}

/// The dish's facing from the reticle.
struct Pose {
    az: f32,
    dir: [f32; 2],
    squash: f32,
    focus: [f32; 2],
}

/// The game's state.
#[derive(Default)]
pub struct Sensors {
    part: bool,
    horn: [f32; 2],
    horn_held: bool,
    horn_set: bool,
    phase: Phase,
    dish: [f32; 2],
    steering: bool,
    src: [f32; 2],
    decoy: Option<[f32; 2]>,
    sigma: f32,
    noise: f32,
    noise_v: f32,
    sky: f32,
    tol: f32,
    lock_t: f32,
    strain: f32,
    jam_t: f32,
    trail: Vec<[f32; 3]>,
    trail_t: f32,
    specks: Vec<Speck>,
    bands: [Band; 3],
    sel: usize,
    grab: Option<usize>,
    clean_t: f32,
    played: bool,
    clock: f32,
    hand: Hand,
}

impl Sensors {
    fn signal(&self, x: f32, y: f32) -> f32 {
        let s = gauss(x - self.src[0], y - self.src[1], self.sigma);
        let d = self.decoy.map_or(0.0, |d| 0.58 * gauss(x - d[0], y - d[1], self.sigma * 1.2));
        s.max(d)
    }
    fn residual(&self, i: usize) -> f32 {
        ((self.bands[i].v - self.bands[i].c).abs() * 3.0).min(1.0)
    }
    fn clean(&self) -> bool {
        (0..3).all(|i| self.residual(i) < self.tol)
    }

    /// The dish's facing from the reticle: elevation from height on the map, azimuth from across it.
    fn pose(&self) -> Pose {
        let [x0, x1, y0, y1] = R;
        let el = (10.0 + 70.0 * (y1 - self.dish[1]) / (y1 - y0)).to_radians();
        let az = (((self.dish[0] - x0) / (x1 - x0) - 0.5) * 240.0).to_radians();
        let side = if az < 0.0 { -1.0 } else { 1.0 };
        let squash = 0.45 + 0.55 * (az * 0.5).cos().abs();
        let dir = [el.cos() * side, -el.sin()];
        let focus = [PIVOT[0] + dir[0] * 110.0 * squash, PIVOT[1] + dir[1] * 110.0];
        Pose { az, dir, squash, focus }
    }
}

/// A quadratic curve's points (canvas's `quadraticCurveTo`).
fn quad_curve(p0: [f32; 2], cp: [f32; 2], p1: [f32; 2]) -> Vec<[f32; 2]> {
    (0..=24)
        .map(|i| {
            let u = i as f32 / 24.0;
            let v = 1.0 - u;
            [v * v * p0[0] + 2.0 * v * u * cp[0] + u * u * p1[0], v * v * p0[1] + 2.0 * v * u * cp[1] + u * u * p1[1]]
        })
        .collect()
}

/// A rounded rectangle filled with a vertical gradient through `stops` (0 at `g_y0`, 1 at `g_y1`), as horizontal strips.
#[allow(clippy::too_many_arguments)]
fn gradient_round(g: &Pen, x: f32, y: f32, w: f32, h: f32, r: f32, g_y0: f32, g_y1: f32, stops: &[(f32, Color32)]) {
    let r = r.min(w / 2.0).min(h / 2.0);
    let col = |yy: f32| {
        let u = ((yy - g_y0) / (g_y1 - g_y0)).clamp(0.0, 1.0);
        let i = stops.windows(2).position(|s| u <= s[1].0).unwrap_or(stops.len().saturating_sub(2));
        let (a, b) = (stops[i], stops[(i + 1).min(stops.len() - 1)]);
        mix(a.1, b.1, if b.0 > a.0 { (u - a.0) / (b.0 - a.0) } else { 0.0 })
    };
    let inset = |yy: f32| {
        let d = if yy < y + r {
            y + r - yy
        } else if yy > y + h - r {
            yy - (y + h - r)
        } else {
            0.0
        };
        r - (r * r - d * d).max(0.0).sqrt()
    };
    // Strip edges: through the corners in small steps, and at every stop in between.
    let mut ys: Vec<f32> =
        (0..=6).map(|k| y + r * k as f32 / 6.0).chain((0..=6).map(|k| y + h - r + r * k as f32 / 6.0)).collect();
    for s in stops {
        let sy = g_y0 + (g_y1 - g_y0) * s.0;
        if sy > y && sy < y + h {
            ys.push(sy);
        }
    }
    ys.sort_by(f32::total_cmp);
    for p in ys.windows(2) {
        let (a, b) = (p[0], p[1]);
        if b - a < 1e-3 {
            continue;
        }
        let (ia, ib) = (inset(a), inset(b));
        let (ca, cb) = (col(a), col(b));
        g.quad([[x + ia, a], [x + w - ia, a], [x + w - ib, b], [x + ib, b]], [ca, ca, cb, cb]);
    }
}

impl Game for Sensors {
    fn id(&self) -> &'static str {
        "sensors"
    }

    fn knobs(&self) -> &'static [&'static str] {
        &["return_px", "decoy_count", "filter_tol_frac", "sky_noise_frac"]
    }

    fn step(&mut self, cx: &mut Ctx) {
        let mut r = cx.dice();
        self.part = cx.part;
        self.phase = if self.part { Phase::Part } else { Phase::Sweep };
        self.horn = [1160.0, 400.0];
        self.horn_held = false;
        self.horn_set = false;
        let [x0, x1, y0, y1] = R;
        // The return somewhere inside the stops, the dish parked away from it.
        self.src = [x0 + 50.0 + r.f() * (x1 - x0 - 100.0), y0 + 40.0 + r.f() * (y1 - y0 - 80.0)];
        self.decoy = (cx.knob("decoy_count") >= 0.5)
            .then(|| [x0 + 60.0 + r.f() * (x1 - x0 - 120.0), y0 + 40.0 + r.f() * (y1 - y0 - 80.0)]);
        if let Some(d) = &mut self.decoy {
            if (d[0] - self.src[0]).hypot(d[1] - self.src[1]) < 180.0 {
                d[0] = if self.src[0] > 330.0 { self.src[0] - 220.0 } else { self.src[0] + 220.0 };
            }
        }
        self.sigma = cx.knob("return_px");
        self.tol = cx.knob("filter_tol_frac");
        self.sky = cx.knob("sky_noise_frac");
        self.dish = [
            if self.src[0] > 330.0 { x0 + 30.0 } else { x1 - 30.0 },
            if self.src[1] > 285.0 { y0 + 30.0 } else { y1 - 30.0 },
        ];
        self.steering = false;
        self.noise = 0.0;
        self.noise_v = 0.0;
        self.lock_t = 0.0;
        self.strain = 0.0;
        self.jam_t = 0.0;
        self.trail.clear();
        self.trail_t = 0.0;
        let [mx, my, mw, mh] = MAP;
        self.specks = (0..150)
            .map(|_| Speck {
                x: mx + 8.0 + r.f() * (mw - 16.0),
                y: my + 8.0 + r.f() * (mh - 16.0),
                p: r.f() * 6.3,
                s: 1.0 + r.f() * 2.0,
            })
            .collect();
        for b in &mut self.bands {
            let c = 0.15 + 0.7 * r.f();
            let v = if c > 0.5 { c - 0.3 - 0.2 * r.f() } else { c + 0.3 + 0.2 * r.f() };
            *b = Band { c, v, p: r.f() * 6.3 };
        }
        self.sel = 0;
        self.grab = None;
        self.clean_t = 0.0;
        self.played = false;
        self.hand = Hand::default();
    }

    fn update(&mut self, cx: &mut Ctx, dt: f32, input: &Input) {
        self.clock += dt;
        match self.phase {
            Phase::Part => {
                if input.pressed && input.dist(self.horn[0], self.horn[1]) < 50.0 {
                    self.horn_held = true;
                }
                if self.horn_held && input.down {
                    self.horn = [input.x, input.y];
                }
                if self.horn_held && input.released {
                    self.horn_held = false;
                    let f = self.pose().focus;
                    if (self.horn[0] - f[0]).hypot(self.horn[1] - f[1]) < 40.0 {
                        self.horn_set = true;
                        self.phase = Phase::Fitted;
                        cx.step_done();
                    }
                }
            }
            Phase::Fitted => {}
            Phase::Sweep => {
                self.jam_t = (self.jam_t - dt).max(0.0);
                let [mx, my, mw, mh] = MAP;
                let [x0, x1, y0, y1] = R;
                // Where the hands want the dish: the pointer while steering on the map, or the arrows.
                if input.pressed && input.x >= mx && input.x <= mx + mw && input.y >= my && input.y <= my + mh {
                    self.steering = true;
                }
                if !input.down {
                    self.steering = false;
                }
                let mut want = if self.steering {
                    [input.x, input.y]
                } else {
                    [self.dish[0] + input.stick[0] * 60.0, self.dish[1] + input.stick[1] * 60.0]
                };
                if self.jam_t > 0.0 {
                    want = self.dish;
                }
                let (dx, dy) = (want[0] - self.dish[0], want[1] - self.dish[1]);
                let d = dx.hypot(dy);
                let step = d.min(SLEW * dt);
                if d > 0.01 {
                    self.dish[0] += dx / d * step;
                    self.dish[1] += dy / d * step;
                }
                // The stops: past them the dish does not go, and pushing on them strains it.
                let pushed = (want[0] < x0 - 4.0 && self.dish[0] <= x0)
                    || (want[0] > x1 + 4.0 && self.dish[0] >= x1)
                    || (want[1] < y0 - 4.0 && self.dish[1] <= y0)
                    || (want[1] > y1 + 4.0 && self.dish[1] >= y1);
                self.dish = [self.dish[0].clamp(x0, x1), self.dish[1].clamp(y0, y1)];
                self.strain =
                    if pushed && self.jam_t == 0.0 { self.strain + dt } else { (self.strain - 2.0 * dt).max(0.0) };
                if self.strain >= JAM_AT {
                    self.strain = 0.0;
                    self.jam_t = JAM_S;
                    self.steering = false;
                    self.dish = [self.dish[0].clamp(x0 + 16.0, x1 - 16.0), self.dish[1].clamp(y0 + 16.0, y1 - 16.0)];
                    cx.fumble("The dish jams");
                    return;
                }
                // The return: the true signal with the sky's noise over it on the meter.
                let k = self.clock;
                self.noise_v +=
                    ((k * 7.3).sin() * 0.5 + (k * 13.1 + 1.0).sin() * 0.5) * dt * 3.0 - self.noise_v * dt * 4.0;
                self.noise = (self.noise + self.noise_v * dt * 8.0).clamp(-1.0, 1.0) * 0.98;
                let s = self.signal(self.dish[0], self.dish[1]);
                self.lock_t =
                    if s >= LOCK && self.jam_t == 0.0 { self.lock_t + dt } else { (self.lock_t - 2.0 * dt).max(0.0) };
                self.trail_t += dt;
                if self.trail_t > 0.06 {
                    self.trail_t = 0.0;
                    self.trail.push([self.dish[0], self.dish[1], s]);
                    if self.trail.len() > 180 {
                        self.trail.remove(0);
                    }
                }
                if self.lock_t >= LOCK_S {
                    self.lock_t = LOCK_S;
                    self.dish = self.src;
                    self.phase = Phase::Filter;
                }
            }
            Phase::Filter => {
                if self.played {
                    return;
                }
                // The three band filters.
                if input.pressed {
                    for (i, &x) in SL_X.iter().enumerate() {
                        if (input.x - x).abs() < 45.0 && input.y > SL_Y0 - 30.0 && input.y < SL_Y1 + 30.0 {
                            self.grab = Some(i);
                            self.sel = i;
                        }
                    }
                }
                if let (Some(gi), true) = (self.grab, input.down) {
                    self.bands[gi].v = ((SL_Y1 - input.y) / (SL_Y1 - SL_Y0)).clamp(0.0, 1.0);
                }
                if input.released {
                    self.grab = None;
                }
                if input.hit(Key::ArrowLeft) || input.hit(Key::A) {
                    self.sel = (self.sel + 2) % 3;
                }
                if input.hit(Key::ArrowRight) || input.hit(Key::D) {
                    self.sel = (self.sel + 1) % 3;
                }
                if input.stick[1] != 0.0 {
                    let b = &mut self.bands[self.sel];
                    b.v = (b.v - input.stick[1] * 0.3 * dt).clamp(0.0, 1.0);
                }
                self.clean_t = if self.clean() { self.clean_t + dt } else { (self.clean_t - 2.0 * dt).max(0.0) };
                if self.clean_t >= CLEAN_S {
                    self.clean_t = CLEAN_S;
                    self.played = true;
                    cx.step_done();
                }
            }
        }
    }

    fn draw(&self, g: &Pen, _cx: &Ctx, t: f32, _input: &Input) {
        use std::f32::consts::{FRAC_PI_2, TAU};
        g.rect(0.0, BAR_H, W, 720.0, hex(0x070b12));
        let sweep = self.phase == Phase::Sweep;
        let filt = self.phase == Phase::Filter;
        let [mx, my, mw, mh] = MAP;
        let [x0, x1, y0, y1] = R;
        // ---- The sky map: noise speckle, the stops hatched, the trail coloured by return, the reticle.
        g.panel(mx, my, mw, mh, 16.0, hex(0x081019), c::LINE);
        let m = g.alpha(if sweep { 1.0 } else { 0.5 });
        for p in &self.specks {
            let a = 0.25 + 0.25 * (t * 3.0 + p.p * 7.0).sin() + 0.2 * (t * 11.0 + p.p * 3.0).sin();
            m.rect(p.x, p.y, p.s, p.s, rgba(160, 190, 220, a.clamp(0.0, 1.0)));
        }
        let stop = if self.strain > 0.0 || self.jam_t > 0.0 {
            rgba(255, 71, 87, if self.jam_t > 0.0 { 0.9 } else { 0.3 + 0.6 * self.strain / JAM_AT })
        } else {
            rgba(255, 71, 87, 0.2)
        };
        m.hatch(mx + 2.0, my + 2.0, x0 - mx - 14.0, mh - 4.0, stop);
        m.hatch(x1 + 12.0, my + 2.0, mx + mw - x1 - 14.0, mh - 4.0, stop);
        m.hatch(mx + 2.0, my + 2.0, mw - 4.0, y0 - my - 14.0, stop);
        m.hatch(mx + 2.0, y1 + 12.0, mw - 4.0, my + mh - y1 - 14.0, stop);
        for i in 1..6 {
            let x = x0 + (x1 - x0) * i as f32 / 6.0;
            m.line(x, y0, x, y1, 1.0, hex(0x1b2636));
        }
        for i in 1..4 {
            let y = y0 + (y1 - y0) * i as f32 / 4.0;
            m.line(x0, y, x1, y, 1.0, hex(0x1b2636));
        }
        for &[px, py, hot] in &self.trail {
            let col = if hot > 0.6 { rgba(242, 160, 70, 0.3 + 0.6 * hot) } else { rgba(79, 195, 247, 0.2 + 0.5 * hot) };
            m.disc(px, py, 3.0 + 4.0 * hot, col);
        }
        if !self.part {
            let jam = self.jam_t > 0.0;
            let [dx, dy] = self.dish;
            let rc = if jam {
                c::DANGER
            } else if sweep {
                c::FG
            } else {
                c::OK
            };
            g.ring(dx, dy, 18.0, rc, 3.0);
            if jam {
                g.cross(dx, dy, 12.0, 3.0, rc);
            } else {
                for [ax, ay] in [[1.0, 0.0], [-1.0, 0.0], [0.0, 1.0], [0.0, -1.0]] {
                    g.line(dx + ax * 24.0, dy + ay * 24.0, dx + ax * 34.0, dy + ay * 34.0, 3.0, rc);
                }
            }
            if self.lock_t > 0.0 && sweep {
                g.arc(dx, dy, 28.0, -FRAC_PI_2, -FRAC_PI_2 + TAU * self.lock_t / LOCK_S, 5.0, c::OK);
            }
        }
        // ---- The return meter: cold to hot, the lock line bracketed.
        let s = if self.part {
            0.0
        } else if sweep {
            (self.signal(self.dish[0], self.dish[1]) + self.noise * self.sky).clamp(0.0, 1.0)
        } else {
            1.0
        };
        let [ex, ey, ew, eh] = METER;
        g.round(ex, ey, ew, eh, 10.0, Some(hex(0x121a26)), None);
        let fh = eh * s;
        let stops = [(0.0, c::ACCENT), (0.6, hex(0x9fb7c9)), (0.85, c::AMBER), (1.0, hex(0xff8a3d))];
        gradient_round(g, ex, ey + eh - fh, ew, fh.max(10.0), 10.0, ey + eh, ey, &stops);
        let ly = ey + eh * (1.0 - LOCK);
        g.path(
            &[[ex - 8.0, ly - 6.0], [ex - 8.0, ly], [ex + ew + 8.0, ly], [ex + ew + 8.0, ly - 6.0]],
            false,
            3.0,
            c::OK,
        );
        // ---- The dish on its turntable.
        g.panel(710.0, my, 530.0, mh, 16.0, hex(0x0b111b), c::LINE);
        let pz = self.pose();
        let [pvx, pvy] = PIVOT;
        let turntable = ellipse(pvx, 440.0, 140.0, 22.0);
        g.poly(&turntable, hex(0x1a2232));
        g.path(&turntable, true, 2.0, c::STEEL);
        g.disc(pvx + pz.az.sin() * 120.0, 440.0 + pz.az.cos() * 18.0, 7.0, c::AMBER);
        g.rect(pvx - 22.0, pvy, 44.0, 140.0, hex(0x2a3446));
        g.rect(pvx - 46.0, pvy - 8.0, 92.0, 22.0, hex(0x364257));
        let px = [-pz.dir[1], pz.dir[0]];
        let rim = |k: f32| {
            [
                pvx + px[0] * 125.0 * k * pz.squash + pz.dir[0] * 26.0 * pz.squash,
                pvy + px[1] * 125.0 * k + pz.dir[1] * 26.0,
            ]
        };
        let (r1, r2) = (rim(1.0), rim(-1.0));
        let cp = [pvx - pz.dir[0] * 80.0 * pz.squash, pvy - pz.dir[1] * 80.0];
        let bowl = quad_curve(r1, cp, r2);
        g.poly(&bowl, hex(0xc9d1dc));
        g.path(&bowl, true, 6.0, hex(0x8792a3));
        g.line(r1[0], r1[1], pz.focus[0], pz.focus[1], 3.0, hex(0x8792a3));
        g.line(r2[0], r2[1], pz.focus[0], pz.focus[1], 3.0, hex(0x8792a3));
        g.disc(pvx, pvy, 12.0, hex(0x3a4558));
        if !self.part || self.horn_set {
            g.disc(pz.focus[0], pz.focus[1], 11.0, if self.jam_t > 0.0 { c::DANGER } else { c::COPPER });
        } else {
            dashed_ring(g, pz.focus[0], pz.focus[1], 16.0, c::AMBER, 3.0, 6.0, 5.0);
        }
        if self.part && !self.horn_set {
            g.panel(1100.0, 330.0, 120.0, 120.0, 14.0, hex(0x141b27), c::LINE);
            let [hx, hy] = self.horn;
            g.disc(hx, hy, 14.0, c::COPPER);
            g.ring(hx, hy, 14.0, hex(0xf0c08a), 3.0);
            g.rect(hx - 6.0, hy + 12.0, 12.0, 18.0, hex(0x7a4a22));
        }
        // ---- The scope: the trace, the noise by band, the contact's blip.
        let [sx, sy, sw, sh] = SCOPE;
        g.panel(sx, sy, sw, sh, 16.0, hex(0x081019), c::LINE);
        let base = sy + sh - 72.0;
        let q = g.clip(sx + 4.0, sy + 4.0, sw - 8.0, sh - 8.0);
        for i in 1..8 {
            let x = sx + sw * i as f32 / 8.0;
            q.line(x, sy + 10.0, x, sy + sh - 10.0, 1.0, hex(0x16212f));
        }
        q.line(sx + 10.0, base, sx + sw - 10.0, base, 1.0, hex(0x16212f));
        let n = if filt { [self.residual(0), self.residual(1), self.residual(2)] } else { [1.0; 3] };
        let blip_h = if self.part {
            0.0
        } else if filt {
            96.0
        } else {
            26.0
        };
        let frame = (f64::from(t) * 30.0).floor();
        let clean = self.clean() && filt;
        let nn = 260;
        let trace: Vec<[f32; 2]> = (0..=nn)
            .map(|j| {
                let u = j as f32 / nn as f32;
                let x = sx + 14.0 + (sw - 28.0) * u;
                let h = (f64::from(j) * 12.9898 + frame * 78.233).sin() * 43758.5453;
                let hiss = ((h - h.floor()) * 2.0 - 1.0) as f32;
                let y = base
                    - blip_h * (-((u - 0.56).powi(2)) / (2.0 * 0.011f32.powi(2))).exp()
                    - n[0] * 34.0 * (u * 9.4 + t * 0.9 + self.bands[0].p).sin()
                    - n[1] * 20.0 * (u * 62.0 + t * 5.0 + self.bands[1].p).sin()
                    - n[2] * 16.0 * hiss;
                [x, y]
            })
            .collect();
        let tc = if clean {
            c::OK
        } else if sweep || self.part {
            hex(0x3d6f8f)
        } else {
            c::ACCENT
        };
        q.path(&trace, false, 2.5, tc);
        if filt {
            let (bx, by) = (sx + 14.0 + (sw - 28.0) * 0.56, base - blip_h - 22.0);
            let dia = [[bx, by - 10.0], [bx + 10.0, by], [bx, by + 10.0], [bx - 10.0, by]];
            if clean {
                q.poly(&dia, c::OK);
            }
            q.path(&dia, true, 3.0, if clean { c::OK } else { c::AMBER });
            if self.clean_t > 0.0 {
                q.arc(bx, by, 20.0, -FRAC_PI_2, -FRAC_PI_2 + TAU * self.clean_t / CLEAN_S, 4.0, c::OK);
            }
        }
        // ---- The band filters.
        let f = g.alpha(if filt { 1.0 } else { 0.4 });
        f.panel(890.0, sy, 350.0, sh, 16.0, hex(0x0f1620), c::LINE);
        for (i, label) in ["LO", "MID", "HI"].iter().enumerate() {
            let x = SL_X[i];
            f.round(x - 7.0, SL_Y0, 14.0, SL_Y1 - SL_Y0, 7.0, Some(hex(0x1a2232)), None);
            let ky = SL_Y1 - (SL_Y1 - SL_Y0) * self.bands[i].v;
            let edge = if filt && self.sel == i { c::AMBER } else { hex(0x3a4558) };
            f.round(x - 30.0, ky - 13.0, 60.0, 26.0, 8.0, Some(hex(0xc9d1dc)), Some((3.0, edge)));
            f.text(label, x, SL_Y1 + 26.0, 18.0, c::DIM, Align::Center);
        }
    }

    fn hand(&mut self, _cx: &Ctx, _t: f32) -> Input {
        if self.hand.resting() {
            return Input::default();
        }
        match self.phase {
            Phase::Part => self.hand.carry(1, self.horn, self.pose().focus, PART_STEP),
            // Hold the pointer on the strongest return; the dish slews there and locks.
            Phase::Sweep => self.hand.hold(2, self.src),
            // Slide each filter to where its band cancels, then keep still while the blip holds clean.
            Phase::Filter if !self.played => {
                for (i, b) in self.bands.iter().enumerate() {
                    if (b.v - b.c).abs() > 1e-3 {
                        let ky = |v: f32| SL_Y1 - (SL_Y1 - SL_Y0) * v;
                        return self.hand.carry(10 + i as u32, [SL_X[i], ky(b.v)], [SL_X[i], ky(b.c)], 6.0);
                    }
                }
                self.hand.idle()
            }
            _ => self.hand.idle(),
        }
    }

    fn guide_now(&self) -> Option<usize> {
        match self.phase {
            _ if self.part => None,
            Phase::Sweep => Some(if self.lock_t > 0.0 { 1 } else { 0 }),
            Phase::Filter => Some(2),
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
        plays_to_end("sensors");
    }

    #[test]
    fn holding_the_dish_against_a_stop_jams_it() {
        // Press on the sky map's corner, outside the stops, and keep pushing.
        let mut first = true;
        fumble_check("sensors", move |_r| {
            let i = Input::hold(45.0, 105.0, first);
            first = false;
            i
        });
    }
}
