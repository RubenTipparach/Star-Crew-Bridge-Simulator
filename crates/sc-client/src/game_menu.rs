//! The game menu on Esc (openspec/changes/game-menu): Resume, Options, Return to menu and Exit, each with its Yes or
//! No where it leaves, over a dark veil. The walk and the drill both open it; nothing pauses underneath it (design 2:
//! the ship is shared), it only takes this player's input. It lives in the client because it is the client's screen.

use crate::options::Options;
use egui::{Color32, RichText, Vec2};

const AMBER: Color32 = Color32::from_rgb(242, 160, 70);
const LILAC: Color32 = Color32::from_rgb(190, 160, 230);
const PALE: Color32 = Color32::from_rgb(200, 212, 230);
const DIM: Color32 = Color32::from_rgb(150, 160, 178);
const SLATE: Color32 = Color32::from_rgb(38, 46, 66);
const PANEL: Color32 = Color32::from_rgb(16, 22, 34);
/// The panel's margin round its rows, points.
const MARGIN: f32 = 22.0;

/// The rows, in their order.
const ROWS: [&str; 4] = ["Resume", "Options", "Return to menu", "Exit"];

/// What the menu shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Page {
    /// The four rows; the focused one.
    Main(usize),
    /// Options' rows.
    Options,
    /// A leaving row asks Yes or No: which row, and whether Yes has the focus.
    Confirm(usize, bool),
}

/// What the player chose.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuAsk {
    /// Back to the game.
    Resume,
    /// Leave the crew for the front menu.
    Leave,
    /// Close the game.
    Exit,
}

/// The menu: closed, or open on a page.
#[derive(Debug)]
pub struct GameMenu {
    page: Option<Page>,
    /// In a browser: a page cannot close itself, so there is no Exit (browser-page-fit design 6). Exit there stopped
    /// the engine and left a frozen frame with no way back.
    web: bool,
}

// Not derived: `web` is true in the browser build, where clippy (run for the desktop) does not look.
#[allow(clippy::derivable_impls)]
impl Default for GameMenu {
    fn default() -> Self {
        Self { page: None, web: cfg!(target_os = "emscripten") }
    }
}

impl GameMenu {
    /// Whether it is up (the game underneath takes no input then).
    pub fn open(&self) -> bool {
        self.page.is_some()
    }

    /// Esc (game-menu 2.4): opens it on Resume, or steps back one layer (a question to its row, Options to the
    /// rows, the rows to the game).
    pub fn escape(&mut self) {
        self.page = match self.page {
            None => Some(Page::Main(0)),
            Some(Page::Main(_)) => None,
            Some(Page::Options) => Some(Page::Main(1)),
            Some(Page::Confirm(row, _)) => Some(Page::Main(row)),
        };
    }

    /// Draw it over the screen (when it is up) and take its keys: Up and Down move the focus, Enter presses, the mouse
    /// works on every row. `place` says where the player is ("SCS Tern, Helm"). Returns what was chosen.
    pub fn show(&mut self, ctx: &egui::Context, place: &str, options: &mut Options) -> Option<MenuAsk> {
        let page = self.page?;
        let rows = if self.web { &ROWS[..3] } else { &ROWS[..] };
        let (up, down, left, right, enter) = ctx.input(|i| {
            (
                i.key_pressed(egui::Key::ArrowUp),
                i.key_pressed(egui::Key::ArrowDown),
                i.key_pressed(egui::Key::ArrowLeft),
                i.key_pressed(egui::Key::ArrowRight),
                i.key_pressed(egui::Key::Enter),
            )
        });
        let screen = ctx.content_rect();
        ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::new("menu-veil"))).rect_filled(
            screen,
            0.0,
            Color32::from_black_alpha(128),
        );
        let mut ask = None;
        let mut next = page;
        let size = if page == Page::Options { Vec2::new(560.0, 420.0) } else { Vec2::new(434.0, 360.0) };
        // Placed by its fixed size, not anchored: anchored, the rows were drawn a frame at Options' place coming back
        // from it, and a click there missed (browser-page-fit design 6).
        egui::Window::new("game-menu")
            .title_bar(false)
            .resizable(false)
            .fixed_size(size)
            // Above the veil, which is on the foreground layer over everything else.
            .order(egui::Order::Tooltip)
            .fixed_pos(crate::ui::centred(screen, size + Vec2::splat(2.0 * MARGIN), Vec2::ZERO))
            .frame(egui::Frame::window(&ctx.style()).fill(PANEL).inner_margin(MARGIN).corner_radius(18.0))
            .show(ctx, |ui| {
                ui.set_width(size.x);
                let title = if page == Page::Options { "OPTIONS" } else { "MENU" };
                ui.label(RichText::new(title).size(20.0).color(AMBER));
                ui.label(RichText::new(place).size(15.0).color(DIM));
                ui.add_space(14.0);
                match page {
                    Page::Options => {
                        options.rows(ui);
                        ui.add_space(14.0);
                        if row_button(ui, "Back", true, 200.0).clicked() || enter {
                            next = Page::Main(1);
                        }
                    }
                    Page::Main(focus) => {
                        let mut focus = focus.min(rows.len() - 1);
                        if up {
                            focus = (focus + rows.len() - 1) % rows.len();
                        }
                        if down {
                            focus = (focus + 1) % rows.len();
                        }
                        let mut pressed = enter.then_some(focus);
                        for (i, w) in rows.iter().enumerate() {
                            if row_button(ui, w, i == focus, size.x).clicked() {
                                pressed = Some(i);
                            }
                            ui.add_space(8.0);
                        }
                        next = Page::Main(focus);
                        match pressed {
                            Some(0) => ask = Some(MenuAsk::Resume),
                            Some(1) => next = Page::Options,
                            Some(r @ (2 | 3)) => next = Page::Confirm(r, false),
                            _ => {}
                        }
                    }
                    Page::Confirm(row, yes) => {
                        let mut yes = yes;
                        if left || right || up || down {
                            yes = !yes;
                        }
                        for (i, w) in rows.iter().enumerate() {
                            if i != row {
                                if row_button(ui, w, false, size.x).clicked() {
                                    next = Page::Main(i);
                                }
                                ui.add_space(8.0);
                                continue;
                            }
                            // The row asks its question with Yes and No in it; No has the focus first.
                            let q = if row == 2 { "Leave the ship?" } else { "Exit Star Crew?" };
                            let mut choice = enter.then_some(yes);
                            egui::Frame::new()
                                .fill(SLATE)
                                .corner_radius(10.0)
                                .inner_margin(egui::Margin::symmetric(18, 9))
                                .show(ui, |ui| {
                                    ui.set_width(size.x - 36.0);
                                    ui.horizontal(|ui| {
                                        ui.label(RichText::new(q).size(22.0).color(PALE));
                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                            if small_button(ui, "No", !yes).clicked() {
                                                choice = Some(false);
                                            }
                                            if small_button(ui, "Yes", yes).clicked() {
                                                choice = Some(true);
                                            }
                                        });
                                    });
                                });
                            ui.add_space(8.0);
                            next = Page::Confirm(row, yes);
                            match choice {
                                Some(true) => ask = Some(if row == 2 { MenuAsk::Leave } else { MenuAsk::Exit }),
                                Some(false) => next = Page::Main(row),
                                None => {}
                            }
                        }
                    }
                }
            });
        self.page = if ask.is_some() { None } else { Some(next) };
        ask
    }
}

/// A row: lilac with dark words when focused, slate with pale words otherwise.
fn row_button(ui: &mut egui::Ui, word: &str, on: bool, w: f32) -> egui::Response {
    let b = egui::Button::new(RichText::new(word).size(22.0).color(if on { Color32::BLACK } else { PALE }))
        .min_size(Vec2::new(w, 50.0))
        .corner_radius(10.0)
        .fill(if on { LILAC } else { SLATE });
    let r = ui.add(b);
    crate::ui::mark(word, r)
}

/// Yes or No inside a question's row.
fn small_button(ui: &mut egui::Ui, word: &str, on: bool) -> egui::Response {
    let b = egui::Button::new(RichText::new(word).size(20.0).color(if on { Color32::BLACK } else { PALE }))
        .min_size(Vec2::new(60.0, 40.0))
        .corner_radius(8.0)
        .fill(if on { LILAC } else { Color32::from_rgb(28, 34, 50) });
    ui.add(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn esc_steps_back_one_layer_at_a_time() {
        let mut m = GameMenu::default();
        m.escape();
        assert_eq!(m.page, Some(Page::Main(0)), "Esc opens it on Resume");
        m.page = Some(Page::Confirm(3, false));
        m.escape();
        assert_eq!(m.page, Some(Page::Main(3)), "a question goes back to its row");
        m.page = Some(Page::Options);
        m.escape();
        assert_eq!(m.page, Some(Page::Main(1)), "Options goes back to the rows, on Options");
        m.escape();
        assert!(!m.open(), "the rows go back to the game");
    }

    #[test]
    fn exit_asks_first_and_no_is_where_the_focus_starts() {
        let ctx = egui::Context::default();
        let mut m = GameMenu { page: Some(Page::Main(3)), web: false };
        let mut o = Options::default();
        let press = |ctx: &egui::Context, m: &mut GameMenu, o: &mut Options, key: Option<egui::Key>| {
            let events = key
                .map(|k| {
                    vec![egui::Event::Key {
                        key: k,
                        physical_key: None,
                        pressed: true,
                        repeat: false,
                        modifiers: Default::default(),
                    }]
                })
                .unwrap_or_default();
            let raw = egui::RawInput { events, ..Default::default() };
            let mut out = None;
            let _ = ctx.run(raw, |ctx| out = m.show(ctx, "SCS Tern, Helm", o));
            out
        };
        assert_eq!(press(&ctx, &mut m, &mut o, Some(egui::Key::Enter)), None, "Exit asks before it closes the game");
        assert_eq!(m.page, Some(Page::Confirm(3, false)));
        assert_eq!(press(&ctx, &mut m, &mut o, Some(egui::Key::Enter)), None, "Enter on No goes back");
        assert_eq!(m.page, Some(Page::Main(3)));
        press(&ctx, &mut m, &mut o, Some(egui::Key::Enter));
        press(&ctx, &mut m, &mut o, Some(egui::Key::ArrowLeft));
        assert_eq!(press(&ctx, &mut m, &mut o, Some(egui::Key::Enter)), Some(MenuAsk::Exit), "Yes exits");
        assert!(!m.open());
    }

    #[test]
    fn the_browser_has_no_exit_to_leave_a_frozen_page() {
        // Regression (browser-page-fit design 6): Exit in the browser stopped the engine, and the page sat frozen.
        let ctx = egui::Context::default();
        let mut m = GameMenu { page: Some(Page::Main(2)), web: true };
        let mut o = Options::default();
        let down = egui::Event::Key {
            key: egui::Key::ArrowDown,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Default::default(),
        };
        let raw = egui::RawInput { events: vec![down], ..Default::default() };
        let _ = ctx.run(raw, |ctx| {
            m.show(ctx, "SCS Tern, Helm", &mut o);
        });
        // A frame more with no keys, past egui's first sizing frame, to read what is drawn.
        let out = ctx.run(egui::RawInput::default(), |ctx| {
            m.show(ctx, "SCS Tern, Helm", &mut o);
        });
        let texts = crate::ui::texts(&out.shapes);
        assert_eq!(m.page, Some(Page::Main(0)), "down from Return to menu wraps to Resume, past no Exit");
        assert!(!texts.iter().any(|t| t == "Exit"), "no Exit row is drawn in the browser: {texts:?}");
        assert!(texts.iter().any(|t| t == "Return to menu"), "the way back to the front menu stays");
    }

    #[test]
    fn the_rows_are_where_they_settle_on_the_first_frame_back_from_options() {
        // Regression (browser-page-fit design 6): back from Options the rows were drawn a frame at Options' place, so a
        // click there missed.
        let ctx = egui::Context::default();
        let mut o = Options::default();
        let mut m = GameMenu { page: Some(Page::Options), web: false };
        let frame = |m: &mut GameMenu, o: &mut Options| {
            let raw = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(1280.0, 720.0))),
                ..Default::default()
            };
            let _ = ctx.run(raw, |ctx| {
                m.show(ctx, "SCS Tern, Helm", o);
            });
            crate::ui::take_marks(&ctx).into_iter().find(|(l, _)| l == "Resume").map(|x| x.1)
        };
        for _ in 0..3 {
            frame(&mut m, &mut o);
        }
        m.page = Some(Page::Main(1));
        let first = frame(&mut m, &mut o).expect("Resume is drawn");
        let mut settled = first;
        for _ in 0..3 {
            settled = frame(&mut m, &mut o).expect("Resume is drawn");
        }
        assert!((first.center() - settled.center()).length() < 0.5, "Resume moved from {first:?} to {settled:?}");
    }
}
