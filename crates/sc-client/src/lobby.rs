//! The front menu (openspec/changes/front-menu) and the lobby inside it (openspec/changes/lobby, design 1): the game
//! opens on four buttons, Single Player, Multiplayer, Options and Quit. Single Player and Multiplayer hold the officer
//! panel, where the player names their officer and picks the station they would like. Its names come from
//! `sc-core::names`; the officer is kept in `settings/officer.json` between sessions (lobby design 4).
//!
//! Menus name things, they don't explain them (CLAUDE.md 10): every row is a label and a control, and each panel is a
//! fixed size whatever is typed into it.

use egui::{Align2, Color32, RichText, Vec2};
use sc_core::combat::Station;
use sc_core::names::{NameData, NAME_MAX_CHARS};
use sc_core::rng::Rng;
use serde::{Deserialize, Serialize};

use crate::options::{Display, Options};

/// The stations a player can ask for, in the order the lobby shows them (`bridge-stations`).
pub const STATIONS: [&str; 8] = ["Captain", "Helm", "Tactical", "Engineering", "Science", "Comms", "Flight ops", "Any"];

/// The station a lobby choice stands for in a game: none for Any (the game picks), and none for Comms and Flight ops,
/// whose consoles are not built (their buttons are off in a game's page).
pub fn station_of(i: usize) -> Option<Station> {
    match STATIONS.get(i).copied() {
        Some("Captain") => Some(Station::Captain),
        Some("Helm") => Some(Station::Helm),
        Some("Tactical") => Some(Station::Tactical),
        Some("Engineering") => Some(Station::Engineering),
        Some("Science") => Some(Station::Science),
        _ => None,
    }
}

/// Where the officer is kept between sessions.
const OFFICER_FILE: &str = "settings/officer.json";

/// The player's officer.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Officer {
    /// Index into `names.json` ranks.
    pub rank: usize,
    /// Their name.
    pub name: String,
    /// Index into `STATIONS`.
    pub station: usize,
    /// The drill server last joined, a host or host:port (empty before the first).
    #[serde(default)]
    pub server: String,
}

/// What the menu asks of the client after a frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LobbyAction {
    /// Nothing yet.
    None,
    /// Start a single-player game with the officer (Beam aboard).
    Solo,
    /// Walk the ship (Tour).
    Tour,
    /// Join the drill server on the Multiplayer page.
    Join,
    /// Close the game.
    Quit,
}

/// Which page is up.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    /// The four buttons.
    Main,
    /// The officer, Beam aboard and Tour.
    Single,
    /// The officer, a server and Join.
    Multi,
    /// Display.
    Options,
}

/// The four buttons, in order.
const MAIN: [(&str, Page); 3] =
    [("SINGLE PLAYER", Page::Single), ("MULTIPLAYER", Page::Multi), ("OPTIONS", Page::Options)];

const AMBER: Color32 = Color32::from_rgb(242, 160, 70);
const LILAC: Color32 = Color32::from_rgb(190, 160, 230);
const PALE: Color32 = Color32::from_rgb(200, 212, 230);
const SLATE: Color32 = Color32::from_rgb(38, 46, 66);

/// The menu's state.
pub struct Lobby {
    /// The officer being made.
    pub officer: Officer,
    /// The page up.
    pub page: Page,
    /// The kept options.
    pub options: Options,
    names: NameData,
    /// The rank was chosen by hand: a station change no longer sets it.
    rank_chosen: bool,
    rolls: u64,
    seed: u64,
    code: String,
    /// The focused row of the four buttons (Up, Down, Enter).
    focus: usize,
}

impl Lobby {
    /// A menu with the kept officer, or a freshly rolled one from `seed` (the client's clock: a new crew each start).
    pub fn new(names: NameData, seed: u64) -> Self {
        let kept = std::fs::read_to_string(OFFICER_FILE)
            .ok()
            .and_then(|t| serde_json::from_str::<Officer>(&t).ok())
            .filter(|o| o.rank < names.ranks.len() && o.station < STATIONS.len() && !o.name.trim().is_empty());
        let mut l = Self {
            officer: Officer { rank: 0, name: String::new(), station: 1, server: String::new() },
            page: Page::Main,
            options: Options::load(),
            rank_chosen: kept.is_some(),
            names,
            rolls: 0,
            seed,
            code: String::new(),
            focus: 0,
        };
        match kept {
            Some(o) => l.officer = o,
            None => {
                l.roll();
                l.officer.rank = l.default_rank();
            }
        }
        l
    }

    /// The session's seed (the bot crew's names and choices are drawn from it).
    pub fn seed(&self) -> u64 {
        self.seed
    }

    /// Roll a new name: a fresh stream each press.
    pub fn roll(&mut self) {
        self.rolls += 1;
        self.officer.name = self.names.generate(&mut Rng::for_purpose(self.seed, self.rolls, "officer_name"));
    }

    /// The rank a station suggests (lobby design 2): Captain for the captain's chair, Lieutenant for the others.
    fn default_rank(&self) -> usize {
        let want = if self.officer.station == 0 { "Captain" } else { "Lieutenant" };
        self.names.ranks.iter().position(|r| r.name == want).unwrap_or(0)
    }

    /// "Lt. Maren Holloway": the rank's short form and the name.
    pub fn title(&self) -> String {
        format!("{} {}", self.names.ranks[self.officer.rank].short, self.officer.name.trim())
    }

    /// Keep the officer for the next session (a failure is said, not fatal: the game goes on).
    pub fn keep(&self) {
        let r = std::fs::create_dir_all("settings").and_then(|()| {
            std::fs::write(OFFICER_FILE, serde_json::to_string_pretty(&self.officer).unwrap_or_default() + "\n")
        });
        if let Err(e) = r {
            eprintln!("sc-client: {OFFICER_FILE}: {e} (the officer is not kept)");
        }
    }

    /// Draw the menu; returns what the player asked for.
    pub fn ui(&mut self, ctx: &egui::Context) -> LobbyAction {
        egui::Area::new(egui::Id::new("title")).anchor(Align2::CENTER_TOP, Vec2::new(0.0, 48.0)).show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.label(RichText::new("STAR CREW").size(64.0).strong().color(AMBER));
                ui.label(RichText::new("SCS TERN").size(22.0).color(LILAC));
            });
        });
        // Esc on a page goes back to the four buttons.
        if self.page != Page::Main && ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.page = Page::Main;
            return LobbyAction::None;
        }
        match self.page {
            Page::Main => self.main_page(ctx),
            Page::Single | Page::Multi => self.game_page(ctx),
            Page::Options => {
                self.options_page(ctx);
                LobbyAction::None
            }
        }
    }

    /// A panel of fixed `size` in the middle, a little below the title (CLAUDE.md 10: content never changes its size).
    fn panel(ctx: &egui::Context, id: &str, size: Vec2, add: impl FnOnce(&mut egui::Ui)) {
        egui::Window::new(id)
            .title_bar(false)
            .resizable(false)
            .fixed_size(size)
            .anchor(Align2::CENTER_CENTER, Vec2::new(0.0, 60.0))
            .frame(egui::Frame::window(&ctx.style()).inner_margin(22.0).corner_radius(18.0))
            .show(ctx, |ui| {
                ui.set_width(size.x);
                add(ui);
            });
    }

    /// The four buttons (front-menu design 1): Single Player first, focused and filled, the main action.
    fn main_page(&mut self, ctx: &egui::Context) -> LobbyAction {
        let web = cfg!(target_os = "emscripten");
        let rows = MAIN.len() + 1;
        let (up, down, enter) = ctx.input(|i| {
            (i.key_pressed(egui::Key::ArrowUp), i.key_pressed(egui::Key::ArrowDown), i.key_pressed(egui::Key::Enter))
        });
        // The browser cannot close its own page: Quit is off there, and the focus passes it by.
        let last = if web { rows - 2 } else { rows - 1 };
        if up {
            self.focus = self.focus.saturating_sub(1);
        }
        if down {
            self.focus = (self.focus + 1).min(last);
        }
        let mut action = LobbyAction::None;
        let mut go = None;
        Self::panel(ctx, "menu", Vec2::new(380.0, 316.0), |ui| {
            ui.spacing_mut().item_spacing.y = 16.0;
            // Each word centred in its button.
            ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                for i in 0..rows {
                    let (word, page) = MAIN.get(i).map_or(("QUIT", None), |(w, p)| (*w, Some(*p)));
                    let main = i == 0;
                    let focused = i == self.focus;
                    let colour = if main { Color32::BLACK } else { PALE };
                    let b = egui::Button::new(RichText::new(word).size(24.0).strong().color(colour))
                        .min_size(Vec2::new(380.0, 64.0))
                        .corner_radius(32.0)
                        .fill(if main { AMBER } else { SLATE })
                        .stroke(egui::Stroke::new(if focused { 3.0_f32 } else { 0.0 }, LILAC));
                    let enabled = page.is_some() || !web;
                    let clicked = ui.add_enabled(enabled, b).clicked() || (enter && focused && enabled);
                    if clicked {
                        self.focus = i;
                        match page {
                            Some(p) => go = Some(p),
                            None => action = LobbyAction::Quit,
                        }
                    }
                }
            });
        });
        if let Some(p) = go {
            self.page = p;
        }
        action
    }

    /// Single Player and Multiplayer (front-menu design 2 and 3): the officer panel with Back, then the page's actions.
    fn game_page(&mut self, ctx: &egui::Context) -> LobbyAction {
        let mut action = LobbyAction::None;
        let single = self.page == Page::Single;
        let size = Vec2::new(620.0, 470.0);
        let mut back = false;
        Self::panel(ctx, "lobby", size, |ui| {
            ui.horizontal(|ui| {
                if ui.add(egui::Button::new(RichText::new("BACK").color(PALE)).corner_radius(14.0)).clicked() {
                    back = true;
                }
                ui.add_space(8.0);
                let head = if single { "SINGLE PLAYER" } else { "MULTIPLAYER" };
                ui.label(RichText::new(head).size(20.0).color(LILAC).strong());
            });
            ui.add_space(6.0);
            ui.label(RichText::new("OFFICER").size(14.0).color(AMBER).strong());
            ui.horizontal(|ui| {
                let ranks: Vec<String> = self.names.ranks.iter().map(|r| r.name.clone()).collect();
                egui::ComboBox::from_id_salt("rank").width(230.0).selected_text(&ranks[self.officer.rank]).show_ui(
                    ui,
                    |ui| {
                        for (i, r) in ranks.iter().enumerate() {
                            if ui.selectable_value(&mut self.officer.rank, i, r).clicked() {
                                self.rank_chosen = true;
                            }
                        }
                    },
                );
                let name = egui::TextEdit::singleline(&mut self.officer.name)
                    .char_limit(NAME_MAX_CHARS)
                    .desired_width(250.0)
                    .font(egui::TextStyle::Body);
                ui.add(name);
                if ui.add(egui::Button::new(RichText::new("Roll").color(PALE)).corner_radius(14.0)).clicked() {
                    self.roll();
                }
            });
            ui.add_space(8.0);
            ui.label(RichText::new("STATION").size(14.0).color(AMBER).strong());
            egui::Grid::new("stations").num_columns(4).spacing(Vec2::new(10.0, 10.0)).show(ui, |ui| {
                for (i, s) in STATIONS.iter().enumerate() {
                    let on = self.officer.station == i;
                    // A game seats the player only where a console is built (Any: the game picks).
                    let built = station_of(i).is_some() || *s == "Any";
                    let b = egui::Button::new(RichText::new(*s).color(if on { Color32::BLACK } else { PALE }))
                        .min_size(Vec2::new(140.0, 36.0))
                        .corner_radius(18.0)
                        .fill(if on { LILAC } else { SLATE });
                    if ui.add_enabled(built, b).clicked() {
                        self.officer.station = i;
                        if !self.rank_chosen {
                            self.officer.rank = self.default_rank();
                        }
                    }
                    if i % 4 == 3 {
                        ui.end_row();
                    }
                }
            });
            ui.add_space(14.0);
            let ready = self.ready();
            if single {
                // The main action is the biggest control (CLAUDE.md 10).
                let beam = egui::Button::new(RichText::new("BEAM ABOARD").size(28.0).strong().color(Color32::BLACK))
                    .min_size(Vec2::new(size.x, 64.0))
                    .corner_radius(32.0)
                    .fill(AMBER);
                if ui.add_enabled(ready, beam).clicked() {
                    action = LobbyAction::Solo;
                }
                ui.add_space(8.0);
                let tour = egui::Button::new(RichText::new("TOUR").size(18.0).strong().color(PALE))
                    .min_size(Vec2::new(size.x, 40.0))
                    .corner_radius(20.0)
                    .fill(SLATE);
                if ui.add_enabled(ready, tour).clicked() {
                    action = LobbyAction::Tour;
                }
            } else {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("SERVER").size(14.0).color(AMBER).strong());
                    let field = egui::TextEdit::singleline(&mut self.officer.server)
                        .char_limit(64)
                        .desired_width(360.0)
                        .hint_text("192.168.1.20");
                    ui.add(field);
                });
                ui.add_space(8.0);
                let join = egui::Button::new(RichText::new("JOIN").size(28.0).strong().color(Color32::BLACK))
                    .min_size(Vec2::new(size.x, 64.0))
                    .corner_radius(32.0)
                    .fill(AMBER);
                if ui.add_enabled(ready && !self.officer.server.trim().is_empty(), join).clicked() {
                    action = LobbyAction::Join;
                }
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("CODE").size(14.0).color(AMBER).strong());
                    ui.add_enabled(
                        false,
                        egui::TextEdit::singleline(&mut self.code).char_limit(6).desired_width(110.0),
                    );
                    ui.add_enabled(false, egui::Button::new("Join").corner_radius(14.0));
                });
            }
        });
        if back {
            self.page = Page::Main;
        }
        let enter = ctx.input(|i| i.key_pressed(egui::Key::Enter)) && !ctx.wants_keyboard_input();
        if enter && self.ready() && single {
            action = LobbyAction::Solo;
        }
        action
    }

    /// Options (front-menu design 4): Display, applied at once and kept.
    fn options_page(&mut self, ctx: &egui::Context) {
        let mut back = false;
        let mut set = None;
        Self::panel(ctx, "options", Vec2::new(520.0, 180.0), |ui| {
            ui.horizontal(|ui| {
                if ui.add(egui::Button::new(RichText::new("BACK").color(PALE)).corner_radius(14.0)).clicked() {
                    back = true;
                }
                ui.add_space(8.0);
                ui.label(RichText::new("OPTIONS").size(20.0).color(LILAC).strong());
            });
            ui.add_space(18.0);
            ui.horizontal(|ui| {
                ui.add_sized(Vec2::new(120.0, 40.0), egui::Label::new(RichText::new("Display").size(18.0).color(PALE)));
                for (word, d) in [("Full screen", Display::Fullscreen), ("Window", Display::Window)] {
                    let on = self.options.display == d;
                    let b =
                        egui::Button::new(RichText::new(word).size(16.0).color(if on { Color32::BLACK } else { PALE }))
                            .min_size(Vec2::new(150.0, 40.0))
                            .corner_radius(20.0)
                            .fill(if on { LILAC } else { SLATE });
                    if ui.add(b).clicked() && !on {
                        set = Some(d);
                    }
                }
            });
        });
        if let Some(d) = set {
            self.options.display = d;
            self.options.keep();
            crate::platform_display(d);
        }
        if back {
            self.page = Page::Main;
        }
    }

    fn ready(&self) -> bool {
        !self.officer.name.trim().is_empty()
    }
}
