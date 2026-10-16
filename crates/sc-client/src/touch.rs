//! Playing by thumb (openspec/changes/touch-controls, its approved mockup docs/mockups/touch-controls.html): the touch
//! layer on foot, and a finger on a seated console.
//!
//! On foot ([`Layer`]): a move stick that appears where the left thumb lands (held at its rim, it runs), a drag
//! anywhere else to look, and buttons: USE (E; it carries the verb of what is in reach), DOWN (Q), JUMP (Space), MAP and
//! MENU. Seated: STAND (held, the stand key) and MENU. Nothing here walks, sits or stands: the layer turns fingers into
//! the keys, the stick and the look the keyboard and mouse already give the drill, so there is one walk and one seat
//! (CLAUDE.md 6.1). It shows only while touch is the last input used.
//!
//! On a console ([`ConsoleFinger`], design 4): a tap presses the nearest control within the data's reach (a hit area of
//! at least 44 px, what is drawn unchanged); held still, the control's tip shows and nothing is pressed. A drag or a
//! hold control takes the finger at once, as it takes the mouse.
//!
//! The numbers are `data/input/touch.json`, sizes in CSS px of a 390 px screen. Fingers arrive in window coordinates
//! (the platform's); the layer works in them and draws in the UI's points.

use egui::{Align2, Color32, FontFamily, FontId, Pos2, Rect, Stroke, Vec2};
use sc_client::platform::{keys, TouchPhase};
use sc_core::data::{Checks, Validate};
use serde::Deserialize;

use crate::console::kit::{Hit, Hits};
use crate::seat::PointerEv;

/// `data/input/touch.json` (`starcrew.touch/1`).
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct TouchData {
    /// `starcrew.touch/1`.
    pub schema: String,
    /// Proposal and provenance.
    pub status: String,
    /// The screen the sizes are for: its shorter side, CSS px. A screen scales them by its own shorter side over this.
    pub screen_ref_px: f64,
    /// The move stick.
    pub stick: StickData,
    /// Radians the view turns a CSS px of drag.
    pub look_rad_px: f64,
    /// The buttons.
    pub buttons: ButtonData,
    /// A finger on a seated console.
    pub console: ConsoleData,
}

/// The move stick (design 3).
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct StickData {
    /// The ring's radius, px: the knob at its edge is full speed.
    pub ring_px: f64,
    /// The knob's radius, px.
    pub knob_px: f64,
    /// Pushed less than this, px, the body stands.
    pub dead_px: f64,
    /// Pushed this far, px, held `run_hold_s`, the body runs.
    pub run_rim_px: f64,
    /// Seconds at the rim before the run.
    pub run_hold_s: f64,
    /// The stick's zone: the left share of the screen's width.
    pub zone_w: f64,
    /// The stick's zone starts this share of the screen's height down.
    pub zone_top: f64,
}

/// The buttons' sizes and places (design 3), px.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ButtonData {
    /// USE's diameter.
    pub use_px: f64,
    /// JUMP's and DOWN's diameter.
    pub small_px: f64,
    /// MAP's, MENU's and STAND's size.
    pub corner_px: f64,
    /// USE from the bottom and right edges.
    pub edge_px: f64,
    /// MAP and MENU from the top and side edges.
    pub corner_edge_px: f64,
    /// The gap between USE and JUMP or DOWN.
    pub gap_px: f64,
    /// STAND sits in the bar beside the console when the bar is at least this wide, else over the console's corner.
    pub stand_bar_min_px: f64,
}

/// A finger on a seated console (design 4).
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ConsoleData {
    /// A tap presses the nearest control within this of the finger, px.
    pub reach_px: f64,
    /// Held this long, seconds, still, a press shows the control's tip instead.
    pub long_press_s: f64,
    /// Moved further than this, px, a press is not a tap.
    pub still_px: f64,
}

impl Validate for TouchData {
    fn validate(&self, c: &mut Checks) {
        c.equals("schema", &self.schema, "starcrew.touch/1");
        c.number("screen_ref_px", self.screen_ref_px, 100.0, 4000.0);
        let s = &self.stick;
        for (k, v) in
            [("ring_px", s.ring_px), ("knob_px", s.knob_px), ("dead_px", s.dead_px), ("run_rim_px", s.run_rim_px)]
        {
            c.number(&format!("stick.{k}"), v, 0.0, 400.0);
        }
        if s.run_rim_px > s.ring_px || s.dead_px >= s.ring_px {
            c.number("stick.run_rim_px", -1.0, 0.0, 0.0);
        }
        c.number("stick.run_hold_s", s.run_hold_s, 0.0, 5.0);
        c.number("stick.zone_w", s.zone_w, 0.1, 0.9);
        c.number("stick.zone_top", s.zone_top, 0.0, 0.9);
        c.number("look_rad_px", self.look_rad_px, 1e-5, 0.1);
        let b = &self.buttons;
        for (k, v) in [
            ("use_px", b.use_px),
            ("small_px", b.small_px),
            ("corner_px", b.corner_px),
            ("edge_px", b.edge_px),
            ("corner_edge_px", b.corner_edge_px),
            ("gap_px", b.gap_px),
            ("stand_bar_min_px", b.stand_bar_min_px),
        ] {
            c.number(&format!("buttons.{k}"), v, 0.0, 400.0);
        }
        // Nothing a finger touches is under the 44 px Apple and Google ask for (design 3).
        for (k, v) in [("use_px", b.use_px), ("small_px", b.small_px), ("corner_px", b.corner_px)] {
            c.number(&format!("buttons.{k}"), v, 44.0, 400.0);
        }
        let k = &self.console;
        c.number("console.reach_px", k.reach_px, 0.0, 100.0);
        c.number("console.long_press_s", k.long_press_s, 0.1, 5.0);
        c.number("console.still_px", k.still_px, 0.0, 100.0);
    }
}

/// The file's path, from the repository's root (the browser build embeds `data/`).
pub const TOUCH_FILE: &str = "data/input/touch.json";

/// Read and check `data/input/touch.json`; a file that fails stops the client (CLAUDE.md 6.5).
pub fn load() -> Result<TouchData, String> {
    let text = std::fs::read_to_string(TOUCH_FILE).map_err(|e| format!("{TOUCH_FILE}: {e}"))?;
    sc_core::data::parse(TOUCH_FILE, &text).map_err(|e| e.to_string())
}

/// A button of the layer.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Btn {
    /// E: sit, take, repair, climb, the lift; held, spray (F).
    Use,
    /// Q: down a ladder or a lift.
    Down,
    /// Space.
    Jump,
    /// The ship's overview (the drill's V).
    Map,
    /// Esc: the game menu.
    Menu,
    /// Seated: held, the stand key.
    Stand,
}

/// What the screen is doing, for which parts of the layer show and take fingers.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Mode {
    /// Nothing of the layer: the menus and the briefing take the fingers.
    Off,
    /// On foot: everything; `down` shows DOWN (something below to go down to).
    Foot {
        /// DOWN shows.
        down: bool,
    },
    /// Seated: STAND and MENU; the console takes the other fingers.
    Seated,
    /// At a machine: MENU (it steps away, as Esc); the repair game takes the other fingers.
    Repair,
}

impl Mode {
    fn shows(self, b: Btn) -> bool {
        match self {
            Mode::Off => false,
            Mode::Foot { down } => matches!(b, Btn::Use | Btn::Jump | Btn::Map | Btn::Menu) || (down && b == Btn::Down),
            Mode::Seated => matches!(b, Btn::Stand | Btn::Menu),
            Mode::Repair => b == Btn::Menu,
        }
    }
}

const BUTTONS: [Btn; 6] = [Btn::Use, Btn::Down, Btn::Jump, Btn::Map, Btn::Menu, Btn::Stand];

#[derive(Copy, Clone, Debug, PartialEq)]
enum Role {
    Stick,
    Look,
    Button(Btn),
}

#[derive(Copy, Clone, Debug)]
struct Finger {
    id: u64,
    role: Role,
    at: [f32; 2],
}

/// The stick under a thumb: where it landed and how far it is pushed, window coordinates.
#[derive(Copy, Clone, Debug, PartialEq)]
struct Stick {
    o: [f32; 2],
    d: [f32; 2],
    rim_s: f64,
    run: bool,
}

/// This frame's hands from the layer.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Hands {
    /// Forward and right, -1..1, from the stick (0 with no thumb on it).
    pub forward: f32,
    /// As `forward`.
    pub right: f32,
    /// The stick is held at its rim.
    pub run: bool,
    /// The look's turn this frame: yaw left and pitch up, radians.
    pub look: (f32, f32),
    /// Buttons tapped this frame.
    pub taps: Vec<Btn>,
    /// Buttons held now.
    pub held: Vec<Btn>,
}

impl Hands {
    /// The keys these hands press this frame and hold now, as the keyboard's keycodes (design 3: the same intents).
    pub fn keys(&self) -> (Vec<u32>, Vec<u32>) {
        let key = |b: &Btn| match b {
            Btn::Use => Some(keys::E),
            Btn::Down => Some(keys::Q),
            Btn::Jump => Some(keys::SPACE),
            Btn::Stand => Some(keys::E),
            Btn::Map | Btn::Menu => None,
        };
        let pressed = self.taps.iter().filter_map(key).collect();
        // USE held is F held as well: the extinguisher sprays while it is down (all-hands 4).
        let mut held: Vec<u32> = self.held.iter().filter_map(key).collect();
        if self.held.contains(&Btn::Use) {
            held.push(keys::F);
        }
        (pressed, held)
    }
}

/// The most fingers followed at once; another is ignored.
const FINGERS: usize = 4;

/// The touch layer.
#[derive(Debug)]
pub struct Layer {
    d: TouchData,
    fingers: Vec<Finger>,
    stick: Option<Stick>,
    taps: Vec<Btn>,
    look: (f32, f32),
    /// Touch is the last input used: the layer shows.
    active: bool,
    /// The window, window coordinates.
    win: [f32; 2],
    mode: Mode,
}

impl Layer {
    /// A layer from the data, for a window `w` x `h`.
    pub fn new(d: TouchData) -> Self {
        Self {
            d,
            fingers: Vec::with_capacity(FINGERS),
            stick: None,
            taps: Vec::new(),
            look: (0.0, 0.0),
            active: false,
            win: [844.0, 390.0],
            mode: Mode::Off,
        }
    }

    /// Whether the layer shows: touch was the last input used.
    pub fn active(&self) -> bool {
        self.active
    }

    /// A key or the mouse was used: the layer hides until the next finger.
    pub fn other_input(&mut self) {
        self.active = false;
    }

    /// Set the window's size (window coordinates) and what the screen is doing. A change of mode lets go of every
    /// finger the layer held, so a USE that sat the body down is not also a STAND held.
    pub fn set(&mut self, win: [f32; 2], mode: Mode) {
        self.win = win;
        if mode != self.mode {
            self.fingers.clear();
            self.stick = None;
            self.mode = mode;
        }
    }

    /// The screen's scale for the data's px.
    fn k(&self) -> f32 {
        (self.win[0].min(self.win[1]) / self.d.screen_ref_px as f32).max(0.1)
    }

    /// Where button `b` is (its centre, window coordinates) and its radius (half its side for a square).
    pub fn place(&self, b: Btn) -> (Pos2, f32) {
        let (w, h, k, d) = (self.win[0], self.win[1], self.k(), &self.d.buttons);
        let px = |v: f64| v as f32 * k;
        let (ur, sr, cr) = (px(d.use_px) / 2.0, px(d.small_px) / 2.0, px(d.corner_px) / 2.0);
        let use_c = Pos2::new(w - px(d.edge_px) - ur, h - px(d.edge_px) - ur);
        match b {
            Btn::Use => (use_c, ur),
            Btn::Jump => (Pos2::new(use_c.x, use_c.y - ur - px(d.gap_px) - sr), sr),
            Btn::Down => (Pos2::new(use_c.x - ur - px(d.gap_px) - sr, use_c.y), sr),
            Btn::Map => (Pos2::new(px(d.corner_edge_px) + cr, px(d.corner_edge_px) + cr), cr),
            Btn::Menu => (Pos2::new(w - px(d.corner_edge_px) - cr, px(d.corner_edge_px) + cr), cr),
            Btn::Stand => {
                // In the black bar beside the 16:9 console where the screen leaves one wide enough, else over the
                // console's left end below its title band (design 4).
                let bar = (w - h * 16.0 / 9.0) / 2.0;
                if bar >= px(d.stand_bar_min_px) {
                    (Pos2::new(bar / 2.0, px(d.corner_edge_px) + cr), cr)
                } else {
                    (Pos2::new(bar.max(0.0) + px(d.corner_edge_px) + cr, h * 0.12 + cr), cr)
                }
            }
        }
    }

    fn button_at(&self, p: [f32; 2]) -> Option<Btn> {
        BUTTONS.iter().copied().filter(|b| self.mode.shows(*b)).find(|b| {
            let (c, r) = self.place(*b);
            let (dx, dy) = (p[0] - c.x, p[1] - c.y);
            match b {
                Btn::Stand => dx.abs() <= r && dy.abs() <= r,
                _ => dx.hypot(dy) <= r,
            }
        })
    }

    /// A finger: true when the layer takes it (a button, the stick, the look), so nothing else gets it.
    pub fn touch(&mut self, id: u64, phase: TouchPhase, x: f32, y: f32) -> bool {
        if !self.active {
            println!("sc-client: touch layer on");
        }
        self.active = true;
        let at = [x, y];
        match phase {
            TouchPhase::Down => {
                if self.fingers.len() >= FINGERS {
                    return true;
                }
                let role = if let Some(b) = self.button_at(at) {
                    self.taps.push(b);
                    Role::Button(b)
                } else if let Mode::Foot { .. } = self.mode {
                    let s = &self.d.stick;
                    let zone = x < self.win[0] * s.zone_w as f32 && y > self.win[1] * s.zone_top as f32;
                    if zone && self.stick.is_none() {
                        self.stick = Some(Stick { o: at, d: [0.0, 0.0], rim_s: 0.0, run: false });
                        Role::Stick
                    } else {
                        Role::Look
                    }
                } else {
                    return false;
                };
                self.fingers.push(Finger { id, role, at });
                true
            }
            TouchPhase::Move => {
                let Some(i) = self.fingers.iter().position(|f| f.id == id) else { return false };
                let f = self.fingers[i];
                match f.role {
                    Role::Look => {
                        let r = self.d.look_rad_px as f32 / self.k();
                        self.look.0 += (x - f.at[0]) * r;
                        self.look.1 += (y - f.at[1]) * r;
                    }
                    Role::Stick => {
                        let ring = self.d.stick.ring_px as f32 * self.k();
                        if let Some(s) = self.stick.as_mut() {
                            let (dx, dy) = (x - s.o[0], y - s.o[1]);
                            let m = dx.hypot(dy);
                            let c = if m > ring { ring / m } else { 1.0 };
                            s.d = [dx * c, dy * c];
                        }
                    }
                    Role::Button(_) => {}
                }
                self.fingers[i].at = at;
                true
            }
            TouchPhase::Up | TouchPhase::Cancel => {
                let Some(i) = self.fingers.iter().position(|f| f.id == id) else { return false };
                if self.fingers[i].role == Role::Stick {
                    self.stick = None;
                }
                self.fingers.remove(i);
                true
            }
        }
    }

    /// This frame's hands, `dt` seconds after the last: the stick's walk (and its run, once held at the rim long
    /// enough), the look's turn since, the buttons tapped since and held now.
    pub fn frame(&mut self, dt: f64) -> Hands {
        let k = self.k();
        let s = &self.d.stick;
        let (ring, dead, rim) = (s.ring_px as f32 * k, s.dead_px as f32 * k, s.run_rim_px as f32 * k);
        let mut hands = Hands {
            look: std::mem::take(&mut self.look),
            taps: std::mem::take(&mut self.taps),
            held: self
                .fingers
                .iter()
                .filter_map(|f| if let Role::Button(b) = f.role { Some(b) } else { None })
                .collect(),
            ..Hands::default()
        };
        if let Some(st) = self.stick.as_mut() {
            let len = st.d[0].hypot(st.d[1]);
            st.rim_s = if len >= rim { st.rim_s + dt } else { 0.0 };
            st.run = st.rim_s >= s.run_hold_s;
            if len >= dead {
                hands.forward = -st.d[1] / ring;
                hands.right = st.d[0] / ring;
                hands.run = st.run;
            }
        }
        hands
    }

    /// Paint the layer on `painter` (UI points; `pt` UI points a window unit): the stick under the thumb, the
    /// buttons this mode shows, USE lit with `use_word` when something is in reach.
    pub fn paint(&self, painter: &egui::Painter, pt: f32, use_word: Option<&str>) {
        if !self.active || self.mode == Mode::Off {
            return;
        }
        let p = |v: Pos2| Pos2::new(v.x * pt, v.y * pt);
        let glass = Color32::from_rgba_unmultiplied(10, 16, 24, 115);
        let rim = Color32::from_rgba_unmultiplied(230, 240, 250, 128);
        let amber = Color32::from_rgb(240, 163, 71);
        let ink = Color32::from_rgb(11, 15, 20);
        let text = Color32::from_rgb(230, 238, 246);
        let font = |size: f32| FontId::new(size, FontFamily::Name(crate::vg::BOLD.into()));
        let k = self.k() * pt;
        if let (Some(s), Mode::Foot { .. }) = (self.stick, self.mode) {
            let (o, r) = (p(Pos2::new(s.o[0], s.o[1])), self.d.stick.ring_px as f32 * k);
            let ring = if s.run { amber } else { rim };
            painter.circle(
                o,
                r,
                Color32::from_rgba_unmultiplied(10, 16, 24, 77),
                Stroke::new(if s.run { 4.0_f32 } else { 2.0 }, ring),
            );
            let knob = Pos2::new(o.x + s.d[0] * pt, o.y + s.d[1] * pt);
            let fill = if s.run {
                Color32::from_rgba_unmultiplied(240, 163, 71, 224)
            } else {
                Color32::from_rgba_unmultiplied(230, 240, 250, 190)
            };
            painter.circle_filled(knob, self.d.stick.knob_px as f32 * k, fill);
        }
        let held: Vec<Btn> =
            self.fingers.iter().filter_map(|f| if let Role::Button(b) = f.role { Some(b) } else { None }).collect();
        for b in BUTTONS.iter().copied().filter(|b| self.mode.shows(*b)) {
            let (c, r) = self.place(b);
            let (c, r) = (p(c), r * pt);
            let down = held.contains(&b);
            let fill = if down { Color32::from_rgba_unmultiplied(230, 240, 250, 77) } else { glass };
            match b {
                Btn::Use => {
                    let live = use_word.is_some();
                    let (f, st, col) = if live {
                        (
                            if down {
                                Color32::from_rgb(255, 210, 154)
                            } else {
                                Color32::from_rgba_unmultiplied(240, 163, 71, 230)
                            },
                            Color32::from_rgb(255, 210, 154),
                            ink,
                        )
                    } else {
                        (
                            fill,
                            Color32::from_rgba_unmultiplied(230, 240, 250, 90),
                            Color32::from_rgba_unmultiplied(230, 240, 250, 140),
                        )
                    };
                    painter.circle(c, r, f, Stroke::new(3.0_f32, st));
                    let word = use_word.unwrap_or("USE").to_uppercase();
                    let size = if word.len() > 6 { r * 0.34 } else { r * 0.43 };
                    painter.text(c, Align2::CENTER_CENTER, word, font(size), col);
                }
                Btn::Stand => {
                    let rect = Rect::from_center_size(c, Vec2::splat(r * 2.0));
                    painter.rect(
                        rect,
                        8.0 * pt,
                        Color32::from_rgba_unmultiplied(10, 16, 24, 224),
                        Stroke::new(2.0_f32, rim),
                        egui::StrokeKind::Inside,
                    );
                    arrow(painter, Pos2::new(c.x, c.y - r * 0.18), r * 0.42, true, text, true);
                    painter.text(Pos2::new(c.x, c.y + r * 0.55), Align2::CENTER_CENTER, "STAND", font(r * 0.42), text);
                }
                _ => {
                    painter.circle(c, r, fill, Stroke::new(2.0_f32, rim));
                    match b {
                        Btn::Jump => arrow(painter, c, r * 0.42, true, text, false),
                        Btn::Down => arrow(painter, c, r * 0.42, false, text, false),
                        Btn::Map => map_icon(painter, c, r * 0.4, text),
                        _ => menu_icon(painter, c, r * 0.36, text),
                    }
                }
            }
        }
    }

    /// The stand ring: how much of STAND's hold has passed, 0..1, drawn round STAND.
    pub fn paint_stand(&self, painter: &egui::Painter, pt: f32, share: f64) {
        if !self.active || self.mode != Mode::Seated || share <= 0.0 {
            return;
        }
        let (c, r) = self.place(Btn::Stand);
        let (c, r) = (Pos2::new(c.x * pt, c.y * pt), r * pt + 6.0);
        let n = 32;
        let end = (share.min(1.0) * f64::from(n)).round() as i32;
        let pts: Vec<Pos2> = (0..=end)
            .map(|i| {
                let a = -std::f32::consts::FRAC_PI_2 + std::f32::consts::TAU * i as f32 / n as f32;
                Pos2::new(c.x + r * a.cos(), c.y + r * a.sin())
            })
            .collect();
        painter.add(egui::Shape::line(pts, Stroke::new(4.0_f32, Color32::from_rgb(240, 163, 71))));
    }
}

fn arrow(painter: &egui::Painter, c: Pos2, s: f32, up: bool, col: Color32, bar: bool) {
    let d = if up { -1.0 } else { 1.0 };
    let st = Stroke::new(2.5_f32, col);
    painter.line_segment([Pos2::new(c.x, c.y - d * s), Pos2::new(c.x, c.y + d * s)], st);
    painter.line_segment([Pos2::new(c.x - s * 0.7, c.y + d * s * 0.25), Pos2::new(c.x, c.y + d * s)], st);
    painter.line_segment([Pos2::new(c.x + s * 0.7, c.y + d * s * 0.25), Pos2::new(c.x, c.y + d * s)], st);
    if bar {
        painter.line_segment([Pos2::new(c.x - s, c.y - s * 1.35), Pos2::new(c.x + s, c.y - s * 1.35)], st);
    }
}

fn map_icon(painter: &egui::Painter, c: Pos2, s: f32, col: Color32) {
    let st = Stroke::new(2.5_f32, col);
    let pts = [
        (-1.0, -0.78),
        (-0.33, -1.0),
        (0.33, -0.78),
        (1.0, -1.0),
        (1.0, 0.78),
        (0.33, 1.0),
        (-0.33, 0.78),
        (-1.0, 1.0),
    ];
    let v: Vec<Pos2> = pts.iter().map(|(x, y)| Pos2::new(c.x + x * s, c.y + y * s)).collect();
    painter.add(egui::Shape::closed_line(v, st));
    painter.line_segment([Pos2::new(c.x - 0.33 * s, c.y - s), Pos2::new(c.x - 0.33 * s, c.y + 0.78 * s)], st);
    painter.line_segment([Pos2::new(c.x + 0.33 * s, c.y - 0.78 * s), Pos2::new(c.x + 0.33 * s, c.y + s)], st);
}

fn menu_icon(painter: &egui::Painter, c: Pos2, s: f32, col: Color32) {
    let st = Stroke::new(2.5_f32, col);
    for dy in [-0.75, 0.0, 0.75] {
        painter.line_segment([Pos2::new(c.x - s, c.y + dy * s), Pos2::new(c.x + s, c.y + dy * s)], st);
    }
}

/// The word USE carries for an on-foot prompt (design 3: the verb of what is in reach), and the prompt without its key
/// letters: "E  Sit at Helm" is "Sit" and "Sit at Helm". A prompt with no E to it lights nothing.
pub fn touch_prompt(prompt: &str) -> (Option<String>, String) {
    let clean = prompt.replace("E  ", "").replace("F  ", "").replace("Q  ", "");
    let clean = clean.split_whitespace().collect::<Vec<_>>().join(" ");
    let word = if let Some(rest) = prompt.strip_prefix("E  ") {
        rest.split([' ', ':']).next().map(str::to_owned)
    } else if prompt.contains("F  Spray") {
        Some("Spray".to_owned())
    } else {
        None
    };
    (word, clean)
}

/// A finger on a seated console (design 4).
#[derive(Debug)]
pub struct ConsoleFinger {
    d: ConsoleData,
    /// Fingers since the last frame, UI points, and the time.
    queue: Vec<(u64, TouchPhase, Pos2, f64)>,
    press: Option<Press>,
}

#[derive(Debug, Clone, Copy)]
struct Press {
    id: u64,
    at: Pos2,
    t0: f64,
    /// A tap to come: where it presses (the control's centre). None: the finger is the mouse (a drag, a hold).
    tap: Option<Pos2>,
    long: bool,
}

impl ConsoleFinger {
    /// A console's finger from the data.
    pub fn new(d: ConsoleData) -> Self {
        Self { d, queue: Vec::new(), press: None }
    }

    /// A finger on the console, UI points, at `now` seconds; it acts at the next [`ConsoleFinger::resolve`].
    pub fn take(&mut self, id: u64, phase: TouchPhase, at: Pos2, now: f64) {
        self.queue.push((id, phase, at, now));
    }

    /// Let go of everything (the console went away).
    pub fn clear(&mut self) {
        self.queue.clear();
        self.press = None;
    }

    /// Where the console's tip shows: the finger of a long press.
    pub fn hover(&self) -> Option<Pos2> {
        self.press.filter(|p| p.long).map(|p| p.at)
    }

    /// The fingers since the last frame as the pointer's events, against this frame's controls; `pt` is UI points a
    /// window unit (the data's px are window units), `now` seconds.
    pub fn resolve(&mut self, hits: &Hits, pt: f32, now: f64) -> Vec<PointerEv> {
        let (reach, still) = (self.d.reach_px as f32 * pt, self.d.still_px as f32 * pt);
        let mut out = Vec::new();
        for (id, phase, at, t) in std::mem::take(&mut self.queue) {
            match phase {
                TouchPhase::Down if self.press.is_none() => {
                    let direct = hits.at(at).and_then(|r| r.hit.as_ref());
                    let mouse = matches!(direct, Some(Hit::Drag(_) | Hit::Hold(_) | Hit::Tap(_)));
                    let tap = if mouse { None } else { nearest_act(hits, at, reach) };
                    if tap.is_none() {
                        out.push(PointerEv::Press(at));
                    }
                    self.press = Some(Press { id, at, t0: t, tap, long: false });
                }
                TouchPhase::Move => {
                    let Some(p) = self.press.as_mut().filter(|p| p.id == id) else { continue };
                    if p.tap.is_some() {
                        // A press that moved is not a tap: it is let go of, acting on nothing.
                        if p.at.distance(at) > still {
                            self.press = None;
                        }
                    } else {
                        p.at = at;
                        out.push(PointerEv::Move(at));
                    }
                }
                TouchPhase::Up | TouchPhase::Cancel => {
                    let Some(p) = self.press.filter(|p| p.id == id) else { continue };
                    self.press = None;
                    match (p.tap, p.long, phase) {
                        (Some(c), false, TouchPhase::Up) if t - p.t0 < self.d.long_press_s => {
                            out.push(PointerEv::Press(c));
                            out.push(PointerEv::Release(c));
                        }
                        (Some(_), _, _) => {}
                        (None, _, _) => out.push(PointerEv::Release(at)),
                    }
                }
                TouchPhase::Down => {}
            }
        }
        // Held still long enough: the tip shows (and the tap will not come).
        if let Some(p) = self.press.as_mut() {
            if p.tap.is_some() && now - p.t0 >= self.d.long_press_s {
                p.long = true;
            }
        }
        out
    }
}

/// The nearest control that acts on a press within `reach` of `at` (0 on it), its centre; the topmost of equals.
fn nearest_act(hits: &Hits, at: Pos2, reach: f32) -> Option<Pos2> {
    let mut best: Option<(f32, Pos2)> = None;
    for r in &hits.regions {
        if !matches!(r.hit, Some(Hit::Act(_))) {
            continue;
        }
        let d = if r.contains(at) { 0.0 } else { r.rect.distance_to_pos(at) };
        if d <= reach && best.is_none_or(|(b, _)| d <= b) {
            best = Some((d, if r.contains(at) { at } else { r.rect.center() }));
        }
    }
    best.map(|(_, c)| c)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::console::kit::{Act, Region};

    fn data() -> TouchData {
        let text =
            std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../data/input/touch.json")).expect("read");
        sc_core::data::parse(TOUCH_FILE, &text).expect("data/input/touch.json is valid")
    }

    /// A layer on a phone held sideways (844 x 390, the scale 1), on foot.
    fn layer() -> Layer {
        let mut l = Layer::new(data());
        l.set([844.0, 390.0], Mode::Foot { down: false });
        l
    }

    #[test]
    fn the_touch_data_file_is_valid() {
        let d = data();
        assert_eq!(d.schema, "starcrew.touch/1");
    }

    #[test]
    fn a_thumb_at_the_ring_rim_walks_at_full_speed() {
        let mut l = layer();
        assert!(l.touch(1, TouchPhase::Down, 150.0, 300.0));
        l.touch(1, TouchPhase::Move, 150.0, 240.0);
        let h = l.frame(1.0 / 60.0);
        assert!((h.forward - 1.0).abs() < 1e-4, "pushed to the rim is full speed, got {}", h.forward);
        assert!(h.right.abs() < 1e-4);
        assert!(!h.run, "a run waits for the thumb to stay at the rim");
    }

    #[test]
    fn a_half_pushed_stick_walks_at_half_speed_and_a_small_one_stands() {
        let mut l = layer();
        l.touch(1, TouchPhase::Down, 150.0, 300.0);
        l.touch(1, TouchPhase::Move, 180.0, 300.0);
        assert!((l.frame(0.016).right - 0.5).abs() < 1e-4);
        l.touch(1, TouchPhase::Move, 155.0, 300.0);
        assert_eq!(l.frame(0.016).right, 0.0, "inside the dead zone the body stands");
    }

    #[test]
    fn a_thumb_held_at_the_rim_runs() {
        let mut l = layer();
        l.touch(1, TouchPhase::Down, 150.0, 300.0);
        l.touch(1, TouchPhase::Move, 150.0, 200.0);
        let mut run = false;
        for _ in 0..30 {
            run = l.frame(1.0 / 60.0).run;
        }
        assert!(run, "half a second at the rim is past run_hold_s");
        l.touch(1, TouchPhase::Up, 150.0, 200.0);
        assert_eq!(l.frame(0.016), Hands::default(), "a lifted thumb stops the body");
    }

    #[test]
    fn a_tap_on_use_is_one_press_of_e() {
        let mut l = layer();
        let (c, _) = l.place(Btn::Use);
        assert!(l.touch(9, TouchPhase::Down, c.x, c.y), "USE takes the finger, so nothing else gets it as a click");
        let h = l.frame(0.016);
        assert_eq!(h.keys().0, vec![keys::E]);
        l.touch(9, TouchPhase::Up, c.x, c.y);
        assert!(l.frame(0.016).keys().0.is_empty(), "one tap, one press");
    }

    #[test]
    fn holding_stand_holds_the_stand_key() {
        let mut l = layer();
        l.set([844.0, 390.0], Mode::Seated);
        let (c, _) = l.place(Btn::Stand);
        l.touch(3, TouchPhase::Down, c.x, c.y);
        assert!(l.frame(0.016).keys().1.contains(&keys::E));
        assert!(l.frame(0.3).keys().1.contains(&keys::E), "held until the finger lifts");
        l.touch(3, TouchPhase::Up, c.x, c.y);
        assert!(l.frame(0.016).keys().1.is_empty());
    }

    #[test]
    fn a_change_of_mode_lets_go_of_use() {
        let mut l = layer();
        let (c, _) = l.place(Btn::Use);
        l.touch(9, TouchPhase::Down, c.x, c.y);
        l.frame(0.016);
        l.set([844.0, 390.0], Mode::Seated);
        assert!(l.frame(0.016).held.is_empty(), "the finger that sat the body down must not hold STAND");
    }

    #[test]
    fn a_finger_on_the_look_side_turns_the_view_not_the_body() {
        let mut l = layer();
        assert!(l.touch(4, TouchPhase::Down, 600.0, 200.0));
        l.touch(4, TouchPhase::Move, 650.0, 190.0);
        let h = l.frame(0.016);
        assert!((h.look.0 - 50.0 * 0.006).abs() < 1e-5, "50 px of drag at the data's rate, got {}", h.look.0);
        assert!(h.look.1 < 0.0);
        assert_eq!((h.forward, h.right), (0.0, 0.0));
    }

    #[test]
    fn two_thumbs_walk_and_look_at_once() {
        let mut l = layer();
        l.touch(1, TouchPhase::Down, 150.0, 300.0);
        l.touch(2, TouchPhase::Down, 600.0, 200.0);
        l.touch(1, TouchPhase::Move, 150.0, 260.0);
        l.touch(2, TouchPhase::Move, 610.0, 200.0);
        let h = l.frame(0.016);
        assert!(h.forward > 0.5 && h.look.0 > 0.0);
    }

    #[test]
    fn with_the_layer_off_a_finger_is_not_taken() {
        let mut l = layer();
        l.set([844.0, 390.0], Mode::Off);
        assert!(!l.touch(1, TouchPhase::Down, 150.0, 300.0), "the menus take it");
    }

    #[test]
    fn a_prompt_gives_use_its_verb_and_loses_its_keys() {
        assert_eq!(touch_prompt("E  Sit at Helm"), (Some("Sit".into()), "Sit at Helm".into()));
        assert_eq!(touch_prompt("E  Repair: Coolant"), (Some("Repair".into()), "Repair: Coolant".into()));
        assert_eq!(touch_prompt("E  Up        Q  Down").0, Some("Up".into()));
        assert_eq!(touch_prompt("FIRE   F  Spray it   (agent 80%)").0, Some("Spray".into()));
        assert_eq!(touch_prompt("Lt. Min holds Helm"), (None, "Lt. Min holds Helm".into()));
    }

    fn hits() -> Hits {
        // A 13 point button at (100, 100), the size a 24 lp control is on a phone.
        let rect = Rect::from_min_size(Pos2::new(100.0, 100.0), Vec2::splat(13.0));
        Hits {
            regions: vec![Region {
                rect,
                hit: Some(Hit::Act(Act::Ack)),
                tip: Some("Ack".into()),
                track: rect,
                seg: None,
            }],
        }
    }

    #[test]
    fn a_tap_near_a_small_control_hits_it() {
        let mut f = ConsoleFinger::new(data().console);
        f.take(1, TouchPhase::Down, Pos2::new(123.0, 106.0), 0.0);
        f.take(1, TouchPhase::Up, Pos2::new(123.0, 106.0), 0.1);
        let c = Pos2::new(106.5, 106.5);
        assert_eq!(
            f.resolve(&hits(), 1.0, 0.1),
            vec![PointerEv::Press(c), PointerEv::Release(c)],
            "10 px off its edge is a hit"
        );
    }

    #[test]
    fn a_tap_out_of_reach_presses_nothing_there() {
        let mut f = ConsoleFinger::new(data().console);
        f.take(1, TouchPhase::Down, Pos2::new(160.0, 106.0), 0.0);
        f.take(1, TouchPhase::Up, Pos2::new(160.0, 106.0), 0.1);
        let out = f.resolve(&hits(), 1.0, 0.1);
        assert_eq!(
            out,
            vec![PointerEv::Press(Pos2::new(160.0, 106.0)), PointerEv::Release(Pos2::new(160.0, 106.0))],
            "the bare press, as the mouse's"
        );
    }

    #[test]
    fn a_long_press_shows_the_tooltip_and_presses_nothing() {
        let mut f = ConsoleFinger::new(data().console);
        let at = Pos2::new(105.0, 105.0);
        f.take(1, TouchPhase::Down, at, 0.0);
        assert!(f.resolve(&hits(), 1.0, 0.0).is_empty());
        assert_eq!(f.hover(), None, "no tip at rest");
        assert!(f.resolve(&hits(), 1.0, 0.6).is_empty());
        assert_eq!(f.hover(), Some(at), "held still past long_press_s, the tip shows at the finger");
        f.take(1, TouchPhase::Up, at, 0.7);
        assert!(f.resolve(&hits(), 1.0, 0.7).is_empty(), "a long press presses nothing");
        assert_eq!(f.hover(), None);
    }

    #[test]
    fn a_press_that_moves_is_not_a_tap() {
        let mut f = ConsoleFinger::new(data().console);
        f.take(1, TouchPhase::Down, Pos2::new(105.0, 105.0), 0.0);
        f.take(1, TouchPhase::Move, Pos2::new(140.0, 105.0), 0.05);
        f.take(1, TouchPhase::Up, Pos2::new(140.0, 105.0), 0.1);
        assert!(f.resolve(&hits(), 1.0, 0.1).is_empty());
    }

    #[test]
    fn a_drag_control_takes_the_finger_at_once() {
        let rect = Rect::from_min_size(Pos2::new(0.0, 0.0), Vec2::splat(50.0));
        let hits = Hits {
            regions: vec![Region {
                rect,
                hit: Some(Hit::Drag(crate::console::kit::Drag::Throttle)),
                tip: None,
                track: rect,
                seg: None,
            }],
        };
        let mut f = ConsoleFinger::new(data().console);
        f.take(1, TouchPhase::Down, Pos2::new(10.0, 40.0), 0.0);
        f.take(1, TouchPhase::Move, Pos2::new(10.0, 20.0), 0.05);
        f.take(1, TouchPhase::Up, Pos2::new(10.0, 20.0), 0.1);
        assert_eq!(
            f.resolve(&hits, 1.0, 0.1),
            vec![
                PointerEv::Press(Pos2::new(10.0, 40.0)),
                PointerEv::Move(Pos2::new(10.0, 20.0)),
                PointerEv::Release(Pos2::new(10.0, 20.0))
            ]
        );
    }
}
