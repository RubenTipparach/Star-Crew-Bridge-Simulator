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
        Self { display: Display::default(), names: true, look_speed_pct: LOOK_SPEED_PCT[2], fov_deg: FOV_DEG[2] }
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

    /// The rows of Options (game-menu 3), a label and a control each; whether any changed. The front menu and the
    /// game menu both draw these, so they are one list.
    pub fn rows(&mut self, ui: &mut egui::Ui) -> bool {
        self.rows_on(ui, cfg!(target_os = "emscripten"))
    }

    /// [`Options::rows`] in a browser (`web`) or not: a page decides its own full screen (the browser's, itch.io's
    /// button), so there Display is not shown rather than a choice that does nothing (browser-page-fit design 6).
    fn rows_on(&mut self, ui: &mut egui::Ui, web: bool) -> bool {
        use egui::{Color32, RichText, Vec2};
        let (pale, lilac, slate) =
            (Color32::from_rgb(200, 212, 230), Color32::from_rgb(190, 160, 230), Color32::from_rgb(38, 46, 66));
        let before = *self;
        let label = |ui: &mut egui::Ui, w: &str| {
            ui.add_sized(Vec2::new(150.0, 40.0), egui::Label::new(RichText::new(w).size(18.0).color(pale)));
        };
        let pick = |ui: &mut egui::Ui, word: &str, on: bool, w: f32| {
            let b = egui::Button::new(RichText::new(word).size(16.0).color(if on { Color32::BLACK } else { pale }))
                .min_size(Vec2::new(w, 40.0))
                .corner_radius(20.0)
                .fill(if on { lilac } else { slate });
            ui.add(b).clicked() && !on
        };
        if !web {
            ui.horizontal(|ui| {
                label(ui, "Display");
                for (word, d) in [("Full screen", Display::Fullscreen), ("Window", Display::Window)] {
                    if pick(ui, word, self.display == d, 150.0) {
                        self.display = d;
                    }
                }
            });
            ui.add_space(6.0);
        }
        ui.horizontal(|ui| {
            label(ui, "Names");
            for (word, v) in [("On", true), ("Off", false)] {
                if pick(ui, word, self.names == v, 76.0) {
                    self.names = v;
                }
            }
        });
        ui.add_space(6.0);
        let slider = |ui: &mut egui::Ui, w: &str, v: &mut f64, [lo, hi, _]: [f64; 3], unit: &str| {
            ui.horizontal(|ui| {
                label(ui, w);
                ui.spacing_mut().slider_width = 250.0;
                ui.add(egui::Slider::new(v, lo..=hi).step_by(5.0).show_value(false));
                ui.label(RichText::new(format!("{v:.0}{unit}")).size(18.0).color(pale));
            });
        };
        slider(ui, "Look speed", &mut self.look_speed_pct, LOOK_SPEED_PCT, " %");
        ui.add_space(6.0);
        slider(ui, "Field of view", &mut self.fov_deg, FOV_DEG, "°");
        let changed = *self != before;
        if changed {
            self.keep();
            if self.display != before.display {
                crate::platform_display(self.display);
            }
        }
        changed
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
            let mut o = Options::default();
            let out = ctx.run(egui::RawInput::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    o.rows_on(ui, web);
                });
            });
            crate::ui::texts(&out.shapes)
        };
        assert!(!texts(true).iter().any(|t| t == "Display"), "no Display row in the browser");
        assert!(texts(true).iter().any(|t| t == "Look speed"), "the other rows stay");
        assert!(texts(false).iter().any(|t| t == "Display"), "the desktop and the Pi keep it");
    }
}
