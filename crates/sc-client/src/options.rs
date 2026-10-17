//! The player's options (openspec/changes/front-menu design 4), kept in `settings/options.json` between sessions.
//!
//! It lives in the client because it is the client's: how the window is shown. It is player data, so a file that does
//! not parse is repaired to the defaults with a warning, never a stop (CLAUDE.md 6.5).

use serde::{Deserialize, Serialize};

/// Where the options are kept.
pub const OPTIONS_FILE: &str = "settings/options.json";

/// How the game's window is shown.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum Display {
    /// The whole screen (the default, and the Pi's).
    #[default]
    Fullscreen,
    /// A window on the desktop.
    Window,
}

/// Look speed's range and default, percent of [`LOOK_RAD_PX`] (game-menu 3).
pub const LOOK_SPEED_PCT: [f64; 3] = [25.0, 200.0, 100.0];
/// Field of view's range and default, degrees of the eye's vertical angle (game-menu 3; 70 is the eye's angle the
/// walk and the bridge were drawn at before it was an option).
pub const FOV_DEG: [f64; 3] = [40.0, 80.0, 70.0];
/// The mouse's look at 100 %, radians a pixel.
pub const LOOK_RAD_PX: f32 = 0.0025;

/// Everything Options sets.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Options {
    /// Full screen or a window.
    #[serde(default)]
    pub display: Display,
    /// Names over the crew (crew-nameplates 3).
    #[serde(default = "on")]
    pub names: bool,
    /// The mouse's look, percent of [`LOOK_RAD_PX`].
    #[serde(default = "look_default")]
    pub look_speed_pct: f64,
    /// The eye's vertical field of view, degrees.
    #[serde(default = "fov_default")]
    pub fov_deg: f64,
    /// The first-time hint has been seen through: the player sat down once (ui-art-direction design 11).
    #[serde(default)]
    pub first_steps_done: bool,
}

fn on() -> bool {
    true
}

fn look_default() -> f64 {
    LOOK_SPEED_PCT[2]
}

fn fov_default() -> f64 {
    FOV_DEG[2]
}

impl Default for Options {
    fn default() -> Self {
        Self {
            display: Display::default(),
            names: true,
            look_speed_pct: LOOK_SPEED_PCT[2],
            fov_deg: FOV_DEG[2],
            first_steps_done: false,
        }
    }
}

impl Options {
    /// The options in `text`, or the defaults and why when it does not parse; a number out of its range is brought
    /// into it, and said (player data is repaired, CLAUDE.md 6.5).
    pub fn parse(text: &str) -> (Self, Option<String>) {
        match serde_json::from_str::<Self>(text) {
            Ok(mut o) => {
                let fixed = |v: &mut f64, [lo, hi, d]: [f64; 3]| {
                    let w = if v.is_finite() { v.clamp(lo, hi) } else { d };
                    let changed = w != *v;
                    *v = w;
                    changed
                };
                let a = fixed(&mut o.look_speed_pct, LOOK_SPEED_PCT);
                let b = fixed(&mut o.fov_deg, FOV_DEG);
                let warning =
                    (a || b).then(|| format!("{OPTIONS_FILE}: a number out of its range was brought into it"));
                (o, warning)
            }
            Err(e) => (Self::default(), Some(format!("{OPTIONS_FILE}: {e}; using the defaults"))),
        }
    }

    /// The mouse's look, radians a pixel.
    pub fn look_rad_px(&self) -> f32 {
        LOOK_RAD_PX * (self.look_speed_pct / 100.0) as f32
    }

    /// Apply a change made on the Options screen: keep it, and show the window as it now says.
    fn changed_from(&self, before: &Options) {
        self.keep();
        if self.display != before.display {
            crate::platform_display(self.display);
        }
    }

    /// The kept options: the defaults when there is no file, repaired when it is damaged.
    pub fn load() -> Self {
        let Ok(text) = std::fs::read_to_string(OPTIONS_FILE) else { return Self::default() };
        let (o, warning) = Self::parse(&text);
        if let Some(w) = warning {
            eprintln!("sc-client: {w}");
        }
        o
    }

    /// Keep them for the next session (a failure is said, not fatal).
    pub fn keep(&self) {
        let r = std::fs::create_dir_all("settings")
            .and_then(|()| std::fs::write(OPTIONS_FILE, serde_json::to_string_pretty(self).unwrap_or_default() + "\n"));
        if let Err(e) = r {
            eprintln!("sc-client: {OPTIONS_FILE}: {e} (the options are not kept)");
        }
    }
}

/// A page of the Options screen (ui-art-direction design 10): its rail.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Page {
    /// Full screen or a window, and the field of view.
    #[default]
    Display,
    /// Names over the crew, and the look speed.
    Game,
    /// The bindings, read-only (Q10).
    Controls,
}

/// A tab of the Controls page.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Tab {
    #[default]
    OnFoot,
    Console,
    Touch,
}

/// The Options screen (ui-art-direction design 10, the mockup's controls shots): Esc Back, the title, a rail of three
/// pages and a square panel. The front menu and the game menu both draw it, so it is one screen.
#[derive(Clone, Debug, Default)]
pub struct Screen {
    /// The page up.
    pub page: Page,
    /// The Controls page's tab.
    pub tab: Tab,
    /// The station chosen on the console tab, by its index in the bindings' stations.
    pub chip: usize,
    /// The rail's focus slab sliding between the pages (design 18a).
    pub slide: crate::motion::Slide,
}

/// The rail's pages in their order, with their words.
const PAGES: [(Page, &str); 3] = [(Page::Display, "Display"), (Page::Game, "Game"), (Page::Controls, "Controls")];

/// The square panel, reference pixels on the 1920 x 1080 frame (the mockup's), and its padding.
const PANEL: [f32; 4] = [706.0, 116.0, 880.0, 880.0];
const PANEL_PAD: [f32; 2] = [48.0, 40.0];

impl Screen {
    /// Draw it on `ui` over `screen` and take its clicks; `back` names the Back control for the browser's tests. Changes
    /// to `o` are applied and kept at once. True when Back was pressed (Esc is the caller's).
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        screen: egui::Rect,
        o: &mut Options,
        controls: &crate::bindings::Controls,
        back: &str,
        web: bool,
    ) -> bool {
        use crate::look::{hit, kit, look, Align, KeyTok, Role};
        let c = look().c();
        let k = kit(ui.painter(), screen, crate::ui::Anchor::Centre, egui::Vec2::new(1920.0, 1080.0));
        // Esc Back.
        let esc = [KeyTok::Key("Esc".into())];
        let w = k.keys(96.0, 84.0, &esc);
        let bw = k.text_mid(Role::Label, 96.0 + w + 12.0, 84.0, c.dim.0, "Back", Align::Start);
        let pressed_back = hit(ui, k.r(96.0, 62.0, w + 12.0 + bw, 44.0), back).clicked();
        k.text(Role::Title, 334.0, 150.0, c.text.0, "Options", Align::Start);
        let item_h = look().f.sizes_px.item_h;
        let rail = k.px(look().f.type_.body.px);
        let hits: Vec<bool> = PAGES
            .iter()
            .enumerate()
            .map(|(i, (_, w))| hit(ui, k.r(334.0, 224.0 + i as f32 * item_h, 300.0, item_h), w).clicked())
            .collect();
        if let Some(i) = hits.iter().position(|c| *c) {
            self.page = PAGES[i].0;
        }
        let at = PAGES.iter().position(|(p, _)| *p == self.page).unwrap_or(0);
        rail.focus_slab(&mut self.slide, 334.0, 224.0 + at as f32 * item_h, 300.0, ui.input(|i| i.time));
        for (i, (p, word)) in PAGES.iter().enumerate() {
            rail.item(334.0, 224.0 + i as f32 * item_h, word, self.page == *p, Role::Title);
        }
        let [px, py, pw, ph] = PANEL;
        k.panel(px, py, pw, ph);
        let (x0, y0, w0) = (px + PANEL_PAD[0], py + PANEL_PAD[1], pw - 2.0 * PANEL_PAD[0]);
        let before = *o;
        match self.page {
            Page::Display => {
                let mut y = y0;
                if !web {
                    // A page decides its own full screen in a browser (itch.io's button): no choice there that does
                    // nothing (browser-page-fit design 6).
                    let pick = seg_row(ui, &k, x0, y, w0, "Screen", &["Full screen", "Window"], o.display as usize);
                    if let Some(i) = pick {
                        o.display = if i == 0 { Display::Fullscreen } else { Display::Window };
                    }
                    y += ROW_H;
                }
                slider_row(ui, &k, x0, y, w0, "Field of view", &mut o.fov_deg, FOV_DEG, "\u{b0}");
            }
            Page::Game => {
                let pick = seg_row(ui, &k, x0, y0, w0, "Names over the crew", &["On", "Off"], usize::from(!o.names));
                if let Some(i) = pick {
                    o.names = i == 0;
                }
                slider_row(ui, &k, x0, y0 + ROW_H, w0, "Look speed", &mut o.look_speed_pct, LOOK_SPEED_PCT, " %");
            }
            Page::Controls => self.controls(ui, &k, x0, y0, w0, controls),
        }
        if *o != before {
            o.changed_from(&before);
        }
        pressed_back
    }

    /// The Controls page: three tabs over two columns of groups (the mockup's controls shots).
    fn controls(
        &mut self,
        ui: &mut egui::Ui,
        k: &crate::look::Kit,
        x0: f32,
        y0: f32,
        w0: f32,
        ctl: &crate::bindings::Controls,
    ) {
        use crate::look::{hit, look, Align, Role};
        let c = look().c();
        // The tabs: a box of three, the one up raised with the accent under it.
        let tabs = [(Tab::OnFoot, "On foot"), (Tab::Console, "At a console"), (Tab::Touch, "Touch")];
        let mut x = x0 + 1.0;
        let (th, label) = (47.0, k.line(Role::Label));
        let widths: Vec<f32> = tabs.iter().map(|(_, w)| k.width(Role::Label, w) + 44.0).collect();
        k.edge(x0, y0, widths.iter().sum::<f32>() + 2.0, th + 2.0, c.line2.0);
        for ((t, word), w) in tabs.iter().zip(&widths) {
            let on = self.tab == *t;
            if on {
                k.fill(x, y0 + 1.0, *w, th, c.raised.0);
                k.fill(x, y0 + 1.0 + th - 4.0, *w, 4.0, c.accent.0);
            }
            let col = if on { c.text.0 } else { c.dim.0 };
            k.text(Role::Label, x + 22.0, y0 + 1.0 + (th - 4.0 - label) / 2.0 + 1.0, col, word, Align::Start);
            if hit(ui, k.r(x, y0, *w, th + 2.0), &format!("Controls {word}")).clicked() {
                self.tab = *t;
            }
            x += w;
        }
        let top = y0 + th + 2.0 + 32.0;
        let col_w = (w0 - 56.0) / 2.0;
        let cols = [x0, x0 + col_w + 56.0];
        match self.tab {
            Tab::OnFoot => {
                let mut ys = [top, top];
                for g in &ctl.on_foot {
                    let i = usize::from(g.column.min(1));
                    ys[i] = group(k, cols[i], ys[i], col_w, g);
                }
            }
            Tab::Console => {
                let mut y = group(k, cols[0], top, col_w, &ctl.any_console);
                if let Some(cap) = ctl.stations.iter().find(|g| g.station.as_deref() == Some("command")) {
                    y = group(k, cols[0], y, col_w, cap);
                }
                let _ = y;
                // The stations' chips, the captain's last; the one chosen shows its group.
                let chips: Vec<&crate::bindings::Group> = ctl.stations.iter().collect();
                self.chip = self.chip.min(chips.len().saturating_sub(1));
                let mut cx = cols[1];
                let cy = top - 4.0;
                for (i, g) in chips.iter().enumerate() {
                    let role = g.station.as_deref().unwrap_or("helm");
                    let on = i == self.chip;
                    let rc = look().role(role);
                    if on {
                        k.fill(cx, cy, 56.0, 48.0, c.raised.0);
                        k.fill(cx, cy + 44.0, 56.0, 4.0, rc);
                    }
                    k.edge(cx, cy, 56.0, 48.0, if on { c.line2.0 } else { c.line.0 });
                    k.role_icon(role, cx + 28.0, cy + 24.0, 24.0, rc);
                    if hit(ui, k.r(cx, cy, 56.0, 48.0), &format!("Controls {}", g.name)).clicked() {
                        self.chip = i;
                    }
                    cx += 62.0;
                }
                if let Some(g) = chips.get(self.chip) {
                    group(k, cols[1], cy + 48.0 + 18.0, col_w, g);
                }
            }
            Tab::Touch => {
                touch_drawing(k, x0, top);
                let mut ys = [top + 360.0 + 24.0, top + 360.0 + 24.0];
                for g in &ctl.touch {
                    let i = usize::from(g.column.min(1));
                    ys[i] = group(k, cols[i], ys[i], col_w, g);
                }
            }
        }
    }
}

/// A row of the Display and Game pages, reference pixels.
const ROW_H: f32 = 88.0;

/// A labelled row with a segmented choice at its right; the index pressed, when one other than `on` is.
#[allow(clippy::too_many_arguments)]
fn seg_row(
    ui: &mut egui::Ui,
    k: &crate::look::Kit,
    x: f32,
    y: f32,
    w: f32,
    label: &str,
    words: &[&str],
    on: usize,
) -> Option<usize> {
    use crate::look::{hit, look, Align, Role};
    let c = look().c();
    let cy = y + ROW_H / 2.0;
    k.text_mid(Role::Body, x, cy, c.text.0, label, Align::Start);
    let widths: Vec<f32> = words.iter().map(|wd| k.width(Role::Label, wd) + 44.0).collect();
    let total: f32 = widths.iter().sum();
    let (mut sx, h) = (x + w - total - 2.0, 48.0);
    k.edge(sx, cy - h / 2.0 - 1.0, total + 2.0, h + 2.0, c.line2.0);
    sx += 1.0;
    let mut pick = None;
    for (i, (word, bw)) in words.iter().zip(&widths).enumerate() {
        let sel = i == on;
        if sel {
            k.fill(sx, cy - h / 2.0, *bw, h, c.raised.0);
            k.fill(sx, cy + h / 2.0 - 4.0, *bw, 4.0, c.accent.0);
        }
        k.text_mid(Role::Label, sx + bw / 2.0, cy - 2.0, if sel { c.text.0 } else { c.dim.0 }, word, Align::Middle);
        if hit(ui, k.r(sx, cy - h / 2.0, *bw, h), &format!("{label} {word}")).clicked() && !sel {
            pick = Some(i);
        }
        sx += bw;
    }
    pick
}

/// A labelled row with a slider at its right and its value after it; dragging or clicking the track sets it.
#[allow(clippy::too_many_arguments)]
fn slider_row(
    ui: &mut egui::Ui,
    k: &crate::look::Kit,
    x: f32,
    y: f32,
    w: f32,
    label: &str,
    v: &mut f64,
    [lo, hi, _]: [f64; 3],
    unit: &str,
) {
    use crate::look::{look, Align, Role};
    let c = look().c();
    let cy = y + ROW_H / 2.0;
    k.text_mid(Role::Body, x, cy, c.text.0, label, Align::Start);
    let (tw, vw) = (300.0, 96.0);
    let tx = x + w - vw - tw;
    let r = ui.interact(
        k.r(tx - 10.0, cy - 20.0, tw + 20.0, 40.0),
        egui::Id::new(("slider", label)),
        egui::Sense::click_and_drag(),
    );
    let r = crate::ui::mark(label, r);
    if let Some(p) = r.interact_pointer_pos().filter(|_| r.dragged() || r.clicked()) {
        let track = k.r(tx, cy, tw, 0.0);
        let t = ((p.x - track.min.x) / track.width().max(1.0)).clamp(0.0, 1.0);
        let raw = lo + (hi - lo) * f64::from(t);
        *v = ((raw / 5.0).round() * 5.0).clamp(lo, hi);
    }
    let t = ((*v - lo) / (hi - lo)).clamp(0.0, 1.0) as f32;
    k.fill(tx, cy - 3.0, tw, 6.0, c.line.0);
    k.fill(tx, cy - 3.0, tw * t, 6.0, c.accent.0);
    let hx = tx + tw * t;
    k.fill(hx - 8.0, cy - 16.0, 16.0, 32.0, if r.hovered() || r.dragged() { c.accent.0 } else { c.text.0 });
    k.text_mid(Role::Body, x + w, cy, c.dim.0, &format!("{v:.0}{unit}"), Align::End);
}

/// One group of the Controls page at `x`, `top`, `w` wide: its heading over a hairline, then a row of 56 for each
/// action, its keys at the right; returns where the next group starts (the mockup's `group`).
fn group(k: &crate::look::Kit, x: f32, top: f32, w: f32, g: &crate::bindings::Group) -> f32 {
    use crate::look::{look, Align, Role};
    let c = look().c();
    k.text(Role::Label, x, top, c.dim.0, &g.name, Align::Start);
    let rule = top + k.line(Role::Label) + 10.0;
    k.fill(x, rule, w, 1.0, c.line.0);
    let mut y = rule + 1.0 + 4.0;
    let row = k.px(26.0);
    for (action, keys) in &g.rows {
        let cy = y + 28.0;
        row.text_mid(Role::Body, x, cy, c.text.0, action, Align::Start);
        k.keys_right(x + w, cy, keys);
        y += 56.0;
    }
    y + 28.0
}

/// The touch layout of openspec/changes/touch-controls on a phone held sideways (the mockup's touch tab), 784 x 360.
fn touch_drawing(k: &crate::look::Kit, x0: f32, y0: f32) {
    use crate::look::look;
    use crate::vg::{Pen, T};
    let c = look().c();
    let cv = k.at(x0, y0);
    let p = &cv.cv;
    let lab = |x: f32, y: f32, t: &str, a: crate::vg::Anchor| {
        let px = k.size_px(18.0);
        p.text(x, y, px, c.dim.0, &t.to_uppercase(), T { w: 700, ls: 0.14 * px, a, ..T::default() });
    };
    let btn = |x: f32, y: f32, t: &str, px: f32| {
        let px = k.size_px(px);
        p.text(x, y, px, c.text.0, t, T { w: 700, ls: 0.1 * px, a: crate::vg::Anchor::Middle, ..T::default() });
    };
    use crate::vg::Anchor::{End, Middle, Start};
    p.rect(112.0, 40.0, 560.0, 270.0, 28.0, egui::Color32::from_rgb(5, 8, 12), Some(Pen::new(2.0, c.line2.0)));
    p.circle(232.0, 236.0, 46.0, egui::Color32::TRANSPARENT, Some(Pen::new(3.0, c.line2.0)));
    p.circle(246.0, 226.0, 18.0, c.raised.0, Some(Pen::new(2.0, c.key_edge.0)));
    for (cx, cy, r, t, px) in
        [(604.0, 246.0, 36.0, "USE", 15.0), (534.0, 262.0, 24.0, "DOWN", 12.0), (604.0, 176.0, 24.0, "JUMP", 12.0)]
    {
        p.circle(cx, cy, r, c.raised.0, Some(Pen::new(2.0, c.key_edge.0)));
        btn(cx, cy + 5.0, t, px);
    }
    for (x, w, t) in [(138.0, 44.0, "MAP"), (602.0, 48.0, "MENU")] {
        p.rect(x, 62.0, w, 30.0, 4.0, c.raised.0, Some(Pen::new(2.0, c.key_edge.0)));
        btn(x + w / 2.0, 82.0, t, 12.0);
    }
    let acc = Pen::new(3.0, c.accent.0).round();
    p.path("M430 130c30-14 60-14 90 0", crate::vg::Xf::default(), None, Some(acc));
    p.path("M512 120l9 10-12 4", crate::vg::Xf::default(), None, Some(acc));
    let lead = Pen::new(1.5, c.faint.0);
    for (x1, y1, x2, y2) in [
        (186.0, 236.0, 96.0, 236.0),
        (160.0, 62.0, 160.0, 22.0),
        (626.0, 62.0, 626.0, 22.0),
        (640.0, 246.0, 690.0, 246.0),
        (628.0, 176.0, 690.0, 176.0),
        (534.0, 286.0, 534.0, 336.0),
        (475.0, 118.0, 475.0, 22.0),
    ] {
        p.seg(x1, y1, x2, y2, lead);
    }
    lab(90.0, 242.0, "Move", End);
    lab(160.0, 16.0, "Ship map", Middle);
    lab(626.0, 16.0, "Menu", Middle);
    lab(696.0, 252.0, "Use", Start);
    lab(696.0, 182.0, "Jump", Start);
    lab(534.0, 354.0, "Climb down", Middle);
    lab(475.0, 16.0, "Look", Middle);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_damaged_options_file_loads_the_defaults_and_says_why() {
        let (o, warning) = Options::parse("{\"display\": \"sideways\"");
        assert_eq!(o, Options::default(), "player data is repaired, never a stop (CLAUDE.md 6.5)");
        assert!(warning.is_some_and(|w| w.contains(OPTIONS_FILE)), "the warning names the file");
    }

    #[test]
    fn a_field_of_view_out_of_range_is_brought_in_and_said() {
        let (o, warning) = Options::parse("{\"fov_deg\": 170, \"names\": false}");
        assert_eq!(o.fov_deg, FOV_DEG[1]);
        assert!(!o.names, "the rest is kept");
        assert!(warning.is_some());
    }

    #[test]
    fn a_window_is_kept_and_read_back() {
        let o = Options { display: Display::Window, ..Options::default() };
        let (back, warning) = Options::parse(&serde_json::to_string(&o).unwrap());
        assert_eq!((back, warning), (o, None));
        assert_eq!(Options::parse("{}").0, Options::default(), "a missing field takes its default");
    }

    #[test]
    fn the_browser_shows_no_display_choice_that_does_nothing() {
        // Regression (browser-page-fit design 6): in the browser Display read "Full screen" in a window and did
        // nothing when pressed; the page's own full screen is the way there.
        let texts = |web: bool| {
            let ctx = egui::Context::default();
            crate::vg::install_fonts(&ctx);
            let mut o = Options::default();
            let mut s = Screen::default();
            let ctl = crate::bindings::Bindings::shipped().controls;
            // Two frames: an area's first is a sizing pass that draws nothing.
            let mut out = Vec::new();
            for _ in 0..2 {
                out = ctx
                    .run(egui::RawInput::default(), |ctx| {
                        crate::look::screen_area(ctx, "options", egui::Order::Middle, |ui, screen| {
                            s.show(ui, screen, &mut o, &ctl, "BACK", web);
                        });
                    })
                    .shapes;
            }
            crate::ui::texts(&out)
        };
        assert!(!texts(true).iter().any(|t| t == "Screen"), "no full screen or window row in the browser");
        assert!(texts(true).iter().any(|t| t == "Field of view"), "the other rows stay");
        assert!(texts(false).iter().any(|t| t == "Screen"), "the desktop and the Pi keep it");
    }
}
