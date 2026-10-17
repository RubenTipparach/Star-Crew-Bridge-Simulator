//! The game menu on Esc (openspec/changes/game-menu; drawn as ui-art-direction's shot pause-after): a solid side panel
//! over a scrim with the mission, Resume, Leave station (seated only), Options, Controls and Leave ship, your name with
//! your station's icon; on the right the objectives and the crew. Leave ship offers Return to main menu and Quit to
//! desktop, each asking once. The walk and the game both open it; nothing pauses underneath it (design 2: the ship is
//! shared), it only takes this player's input. It lives in the client because it is the client's screen.

use crate::hud::{self, CrewRow};
use crate::look::{self, hit, kit, look, Align, Kit, Role};
use crate::options::Options;
use crate::ui::Anchor;
use egui::Vec2;

/// A row of the menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Row {
    Resume,
    LeaveStation,
    Options,
    Controls,
    LeaveShip,
}

impl Row {
    fn word(self) -> &'static str {
        match self {
            Row::Resume => "Resume",
            Row::LeaveStation => "Leave station",
            Row::Options => "Options",
            Row::Controls => "Controls",
            Row::LeaveShip => "Leave ship",
        }
    }
}

/// Where Leave ship goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Leave {
    /// Back to the front menu.
    Menu,
    /// Close the game.
    Quit,
}

/// What the menu shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Page {
    /// The rows; the focused one.
    Main(usize),
    /// Options (or Controls, its page) over the scrim.
    Options,
    /// Leave ship's two ways (and Back); the focused one.
    Leave(usize),
    /// A way out asks once: which, and whether Yes has the focus.
    Confirm(Leave, bool),
}

/// What the player chose.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuAsk {
    /// Back to the game.
    Resume,
    /// Stand up from the seat (leave-station 4), as the held stand key does.
    LeaveStation,
    /// Leave the crew for the front menu.
    Leave,
    /// Close the game.
    Exit,
}

/// What the menu shows of the game: the mission, whether this player sits, who they are, the objectives and crew.
#[derive(Debug, Clone, Default)]
pub struct MenuView {
    /// The small line over the title ("Training drill 1").
    pub label: String,
    /// The mission's name.
    pub title: String,
    /// Seated at a station: Leave station is offered.
    pub seated: bool,
    /// This player's name and station's role id, at the panel's foot.
    pub me: Option<(String, String)>,
    /// The objectives and whether each is done.
    pub objectives: Vec<(String, bool)>,
    /// The crew.
    pub crew: Vec<CrewRow>,
}

/// The menu: closed, or open on a page.
#[derive(Debug)]
pub struct GameMenu {
    page: Option<Page>,
    /// In a browser: a page cannot close itself, so there is no Quit to desktop (browser-page-fit design 6). Exit
    /// there stopped the engine and left a frozen frame with no way back.
    web: bool,
    /// Options' page and tab.
    screen: crate::options::Screen,
    /// The rows offered on the last frame (Leave station comes and goes with the seat).
    rows: Vec<Row>,
    /// When it opened, egui's clock: the scrim and the panel come in over `panel_in` (design 8).
    opened: Option<f64>,
    /// The focus slab's slide (design 18a), with the list it slides on.
    slide: (String, crate::motion::Slide),
}

// Not derived: `web` is true in the browser build, where clippy (run for the desktop) does not look.
#[allow(clippy::derivable_impls)]
impl Default for GameMenu {
    fn default() -> Self {
        Self {
            page: None,
            web: cfg!(target_os = "emscripten"),
            screen: crate::options::Screen::default(),
            rows: rows(false),
            opened: None,
            slide: Default::default(),
        }
    }
}

/// The rows for a player seated or not.
fn rows(seated: bool) -> Vec<Row> {
    let mut r = vec![Row::Resume];
    if seated {
        r.push(Row::LeaveStation);
    }
    r.extend([Row::Options, Row::Controls, Row::LeaveShip]);
    r
}

impl GameMenu {
    /// Whether it is up (the game underneath takes no input then).
    pub fn open(&self) -> bool {
        self.page.is_some()
    }

    fn index(&self, r: Row) -> usize {
        self.rows.iter().position(|x| *x == r).unwrap_or(0)
    }

    /// Put the menu on a page by name, for the UI's capture runs (`--ui-shots`, `--demo`; CLAUDE.md 4: a capture
    /// instrument): "closed", "main", "leave", "display", "controls-foot", "controls-console" or "controls-touch".
    pub fn tour_page(&mut self, page: &str) {
        use crate::options::{Page as P, Tab};
        let mut options = |p: P, t: Tab| {
            self.screen.page = p;
            self.screen.tab = t;
            Some(Page::Options)
        };
        self.page = match page {
            "main" => Some(Page::Main(0)),
            "leave" => Some(Page::Leave(0)),
            "display" => options(P::Display, Tab::OnFoot),
            "controls-foot" => options(P::Controls, Tab::OnFoot),
            "controls-console" => options(P::Controls, Tab::Console),
            "controls-touch" => options(P::Controls, Tab::Touch),
            _ => None,
        };
    }

    /// Esc (game-menu 2.4): opens it on Resume, or steps back one layer (a question to Leave ship's ways, those and
    /// Options to the rows, the rows to the game).
    pub fn escape(&mut self) {
        self.page = match self.page {
            None => Some(Page::Main(0)),
            Some(Page::Main(_)) => None,
            Some(Page::Options) => {
                Some(Page::Main(self.index(if self.screen.page == crate::options::Page::Controls {
                    Row::Controls
                } else {
                    Row::Options
                })))
            }
            Some(Page::Leave(_)) => Some(Page::Main(self.index(Row::LeaveShip))),
            Some(Page::Confirm(l, _)) => Some(Page::Leave(if l == Leave::Menu { 0 } else { 1 })),
        };
    }

    /// Draw it over the screen (when it is up) and take its keys: Up and Down move the focus, Enter presses, the mouse
    /// works on every row. Returns what was chosen.
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        v: &MenuView,
        options: &mut Options,
        controls: &crate::bindings::Controls,
    ) -> Option<MenuAsk> {
        let Some(page) = self.page else {
            self.opened = None;
            return None;
        };
        self.rows = rows(v.seated);
        let now = ctx.input(|i| i.time);
        let since = *self.opened.get_or_insert(now);
        let fade = crate::motion::eased(now - since, look().f.motion_ms.panel_in, look().f.motion_ease.panel_in);
        let (up, down, enter) = ctx.input(|i| {
            (
                i.key_pressed(egui::Key::ArrowUp) || i.key_pressed(egui::Key::ArrowLeft),
                i.key_pressed(egui::Key::ArrowDown) || i.key_pressed(egui::Key::ArrowRight),
                i.key_pressed(egui::Key::Enter),
            )
        });
        let mut ask = None;
        let mut next = page;
        let web = self.web;
        look::screen_area(ctx, "game-menu", egui::Order::Foreground, |ui, screen| {
            ui.multiply_opacity(fade.max(0.05));
            let c = look().c();
            look::cover(ui.painter(), look::alpha(egui::Color32::from_rgb(5, 8, 12), 0.9));
            if page == Page::Options {
                if self.screen.show(ui, screen, options, controls, "Back", web) || enter {
                    next = Page::Main(self.index(Row::Options));
                }
                return;
            }
            // The side panel, the screen's height, 680 reference pixels wide.
            let (_, s) = crate::ui::anchored(screen, Anchor::MidLeft, Vec2::ZERO);
            let side = egui::Rect::from_min_size(screen.min, Vec2::new(680.0 * s, screen.height()));
            ui.painter().rect_filled(side, 0.0, c.panel.0);
            ui.painter().line_segment([side.right_top(), side.right_bottom()], egui::Stroke::new(1.0_f32, c.line.0));
            let k = kit(ui.painter(), screen, Anchor::MidLeft, Vec2::new(680.0, 1080.0));
            k.text(Role::Label, 96.0, 150.0, c.accent.0, &v.label, Align::Start);
            k.text(Role::Title, 96.0, 182.0, c.text.0, &v.title, Align::Start);
            let item_h = look().f.sizes_px.item_h;
            let (words, focus): (Vec<&str>, usize) = match page {
                Page::Main(f) => (self.rows.iter().map(|r| r.word()).collect(), f),
                Page::Leave(f) => {
                    let mut w = vec!["Return to main menu"];
                    if !web {
                        w.push("Quit to desktop");
                    }
                    w.push("Back");
                    (w, f)
                }
                Page::Confirm(l, yes) => {
                    let q = if l == Leave::Menu { "Return to main menu?" } else { "Quit to desktop?" };
                    k.text(Role::Label, 96.0, 306.0 - 8.0 - k.line(Role::Label), c.warn.0, q, Align::Start);
                    (vec!["No", "Yes"], usize::from(yes))
                }
                Page::Options => (Vec::new(), 0),
            };
            let n = words.len();
            let mut focus = focus.min(n.saturating_sub(1));
            if up {
                focus = (focus + n - 1) % n;
            }
            if down {
                focus = (focus + 1) % n;
            }
            let mut pressed = enter.then_some(focus);
            // The pointer moves the focus first, so the slab is drawn under the item that has it (design 18a); a new
            // list (another page) starts its slab where its focus is.
            let hits: Vec<egui::Response> = words
                .iter()
                .enumerate()
                .map(|(i, w)| {
                    let r = hit(ui, k.r(64.0, 306.0 + i as f32 * item_h, 520.0, item_h), w);
                    if r.hovered() && ui.input(|inp| inp.pointer.delta() != Vec2::ZERO) {
                        focus = i;
                    }
                    r
                })
                .collect();
            let list = words.join("|");
            if self.slide.0 != list {
                self.slide = (list, crate::motion::Slide::default());
            }
            k.focus_slab(&mut self.slide.1, 64.0, 306.0 + focus as f32 * item_h, 520.0, ui.input(|i| i.time));
            for (i, (w, r)) in words.iter().zip(&hits).enumerate() {
                let y = 306.0 + i as f32 * item_h;
                k.item(64.0, y, w, i == focus, Role::Title);
                if r.clicked() {
                    pressed = Some(i);
                }
            }
            if let Some((name, role)) = &v.me {
                let cy = 1080.0 - 54.0 - 19.0;
                k.role_icon(role, 96.0 + 16.0, cy, 32.0, look().role(role));
                k.text_mid(Role::Body, 96.0 + 46.0, cy, c.text.0, name, Align::Start);
            }
            next = match page {
                Page::Main(_) => match pressed.and_then(|i| self.rows.get(i)) {
                    Some(Row::Resume) => {
                        ask = Some(MenuAsk::Resume);
                        page
                    }
                    Some(Row::LeaveStation) => {
                        ask = Some(MenuAsk::LeaveStation);
                        page
                    }
                    Some(Row::Options) => {
                        self.screen.page = crate::options::Page::Display;
                        Page::Options
                    }
                    Some(Row::Controls) => {
                        self.screen.page = crate::options::Page::Controls;
                        Page::Options
                    }
                    Some(Row::LeaveShip) => Page::Leave(0),
                    None => Page::Main(focus),
                },
                Page::Leave(_) => match pressed.map(|i| words[i]) {
                    Some("Return to main menu") => Page::Confirm(Leave::Menu, false),
                    Some("Quit to desktop") => Page::Confirm(Leave::Quit, false),
                    Some(_) => Page::Main(self.index(Row::LeaveShip)),
                    None => Page::Leave(focus),
                },
                Page::Confirm(l, _) => match pressed {
                    Some(1) => {
                        ask = Some(if l == Leave::Menu { MenuAsk::Leave } else { MenuAsk::Exit });
                        page
                    }
                    Some(_) => Page::Leave(if l == Leave::Menu { 0 } else { 1 }),
                    None => Page::Confirm(l, focus == 1),
                },
                Page::Options => page,
            };
            // The objectives and the crew, on the right.
            panel_right(ui, screen, v);
        });
        self.page = if ask.is_some() { None } else { Some(next) };
        ask
    }
}

/// The right side's panel (shot pause-after): Objectives over Crew, hung from the middle right.
fn panel_right(ui: &mut egui::Ui, screen: egui::Rect, v: &MenuView) {
    let c = look().c();
    let probe: Kit = kit(ui.painter(), screen, Anchor::MidRight, Vec2::ZERO);
    let lab = probe.line(Role::Label);
    let h = 24.0
        + lab
        + 8.0
        + v.objectives.len() as f32 * 48.0
        + 16.0
        + 16.0
        + lab
        + 8.0
        + v.crew.len() as f32 * 60.0
        + 8.0;
    let k = kit(ui.painter(), screen, Anchor::MidRight, Vec2::new(840.0, h));
    k.panel(0.0, 0.0, 744.0, h);
    k.text(Role::Label, 32.0, 24.0, c.dim.0, "Objectives", Align::Start);
    let oy = 24.0 + lab + 8.0;
    hud::objectives(&k, 32.0, oy, &v.objectives);
    let sep = oy + v.objectives.len() as f32 * 48.0 + 16.0;
    k.fill(1.0, sep, 742.0, 1.0, c.line.0);
    k.text(Role::Label, 32.0, sep + 16.0, c.dim.0, "Crew", Align::Start);
    hud::crew_rows(ui, &k, 1.0, sep + 16.0 + lab + 8.0, 742.0, 60.0, 32.0, &v.crew, None);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn press(
        ctx: &egui::Context,
        m: &mut GameMenu,
        o: &mut Options,
        v: &MenuView,
        key: Option<egui::Key>,
    ) -> Option<MenuAsk> {
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
        let ctl = crate::bindings::Bindings::shipped().controls;
        let mut out = None;
        let _ = ctx.run(raw, |ctx| out = m.show(ctx, v, o, &ctl));
        out
    }

    #[test]
    fn esc_steps_back_one_layer_at_a_time() {
        let mut m = GameMenu::default();
        m.escape();
        assert_eq!(m.page, Some(Page::Main(0)), "Esc opens it on Resume");
        m.page = Some(Page::Confirm(Leave::Quit, false));
        m.escape();
        assert_eq!(m.page, Some(Page::Leave(1)), "a question goes back to its way out");
        m.escape();
        assert_eq!(m.page, Some(Page::Main(m.index(Row::LeaveShip))), "the ways out go back to Leave ship");
        m.page = Some(Page::Options);
        m.escape();
        assert_eq!(m.page, Some(Page::Main(m.index(Row::Options))), "Options goes back to the rows, on Options");
        m.escape();
        assert!(!m.open(), "the rows go back to the game");
    }

    #[test]
    fn quit_asks_first_and_no_is_where_the_focus_starts() {
        let ctx = egui::Context::default();
        crate::vg::install_fonts(&ctx);
        let mut m = GameMenu { page: Some(Page::Leave(1)), web: false, ..GameMenu::default() };
        let mut o = Options::default();
        let v = MenuView::default();
        assert_eq!(
            press(&ctx, &mut m, &mut o, &v, Some(egui::Key::Enter)),
            None,
            "Quit asks before it closes the game"
        );
        assert_eq!(m.page, Some(Page::Confirm(Leave::Quit, false)));
        assert_eq!(press(&ctx, &mut m, &mut o, &v, Some(egui::Key::Enter)), None, "Enter on No goes back");
        assert_eq!(m.page, Some(Page::Leave(1)));
        press(&ctx, &mut m, &mut o, &v, Some(egui::Key::Enter));
        press(&ctx, &mut m, &mut o, &v, Some(egui::Key::ArrowDown));
        assert_eq!(press(&ctx, &mut m, &mut o, &v, Some(egui::Key::Enter)), Some(MenuAsk::Exit), "Yes quits");
        assert!(!m.open());
    }

    #[test]
    fn leave_station_is_offered_only_to_a_seated_player() {
        // The spec's scenario (ui-art-direction): a walking player's Esc menu lists Resume, Options, Controls and
        // Leave ship, without Leave station.
        let words = |seated: bool| -> Vec<&'static str> { rows(seated).iter().map(|r| r.word()).collect() };
        assert_eq!(words(false), ["Resume", "Options", "Controls", "Leave ship"]);
        assert_eq!(words(true), ["Resume", "Leave station", "Options", "Controls", "Leave ship"]);
        let ctx = egui::Context::default();
        crate::vg::install_fonts(&ctx);
        let mut m = GameMenu { page: Some(Page::Main(1)), web: false, ..GameMenu::default() };
        let mut o = Options::default();
        let v = MenuView { seated: true, ..MenuView::default() };
        assert_eq!(press(&ctx, &mut m, &mut o, &v, Some(egui::Key::Enter)), Some(MenuAsk::LeaveStation));
    }

    #[test]
    fn the_browser_has_no_quit_to_leave_a_frozen_page() {
        // Regression (browser-page-fit design 6): Exit in the browser stopped the engine, and the page sat frozen.
        let ctx = egui::Context::default();
        crate::vg::install_fonts(&ctx);
        let mut m = GameMenu { page: Some(Page::Leave(0)), web: true, ..GameMenu::default() };
        let mut o = Options::default();
        let v = MenuView::default();
        press(&ctx, &mut m, &mut o, &v, None);
        let ctl = crate::bindings::Bindings::shipped().controls;
        let out = ctx.run(egui::RawInput::default(), |ctx| {
            m.show(ctx, &v, &mut o, &ctl);
        });
        let texts = crate::ui::texts(&out.shapes);
        assert!(!texts.iter().any(|t| t == "QUIT TO DESKTOP"), "no Quit to desktop in the browser: {texts:?}");
        assert!(texts.iter().any(|t| t == "RETURN TO MAIN MENU"), "the way back to the front menu stays");
    }

    #[test]
    fn the_rows_are_where_they_settle_on_the_first_frame_back_from_options() {
        // Regression (browser-page-fit design 6): back from Options the rows were drawn a frame at Options' place, so a
        // click there missed.
        let ctx = egui::Context::default();
        crate::vg::install_fonts(&ctx);
        let mut o = Options::default();
        let v = MenuView::default();
        let ctl = crate::bindings::Bindings::shipped().controls;
        let mut m = GameMenu { page: Some(Page::Options), ..GameMenu::default() };
        let frame = |m: &mut GameMenu, o: &mut Options| {
            let raw = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(1280.0, 720.0))),
                ..Default::default()
            };
            let _ = ctx.run(raw, |ctx| {
                m.show(ctx, &v, o, &ctl);
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
