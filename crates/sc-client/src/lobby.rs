//! The front menu (openspec/changes/front-menu) and the lobby inside it (openspec/changes/lobby, design 1): the game
//! opens on four items, Single Player, Multiplayer, Options and Quit. Single Player and Multiplayer hold the officer
//! panel, where the player names their officer and picks the station they would like. Its names come from
//! `sc-core::names`; the officer is kept in `settings/officer.json` between sessions (lobby design 4).
//!
//! Drawn as the approved UI art direction mockup draws them (openspec/changes/ui-art-direction design 11, shots
//! menu-after and lobby-after): a left-hung column on the Tern's key art, one primary action, stations as cards with
//! the consoles' role icons, laid out in reference pixels and hung from the screen's middle left (`ui::anchored`).
//! Menus name things, they don't explain them (CLAUDE.md 10): every row is a label and a control, and each panel is a
//! fixed size whatever is typed into it.

use crate::look::{self, hit, kit, look, Align, Art, KeyTok, Kit, Role};
use crate::options::Options;
use crate::ui::Anchor;
use egui::Vec2;
use sc_core::combat::Station;
use sc_core::names::{NameData, NAME_MAX_CHARS};
use sc_core::rng::Rng;
use serde::{Deserialize, Serialize};

/// The stations a player can ask for, by their index kept in `settings/officer.json` (`bridge-stations`).
pub const STATIONS: [&str; 8] = ["Captain", "Helm", "Tactical", "Engineering", "Science", "Comms", "Flight ops", "Any"];

/// The station cards shown, as indices into [`STATIONS`], with their role icons: the stations built and Any. Comms
/// and Flight ops are not built, so they are not shown (Q4: a shipped game does not show what it does not have).
const CARDS: [(usize, &str); 6] =
    [(0, "command"), (1, "helm"), (2, "tactical"), (3, "engineering"), (4, "science"), (7, "any")];

/// The station a lobby choice stands for in a game: none for Any (the game picks), and none for Comms and Flight ops,
/// whose consoles are not built.
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
    /// Walk the ship (Explore the ship).
    Tour,
    /// Join the drill server on the Multiplayer page.
    Join,
    /// Close the game.
    Quit,
}

/// Which page is up.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    /// The four items.
    Main,
    /// The officer, Beam aboard and Explore the ship.
    Single,
    /// The officer, a server and Join.
    Multi,
    /// Options' pages.
    Options,
}

/// The four items, in order.
const MAIN: [(&str, Page); 3] =
    [("SINGLE PLAYER", Page::Single), ("MULTIPLAYER", Page::Multi), ("OPTIONS", Page::Options)];

/// The menu's state.
pub struct Lobby {
    /// The officer being made.
    pub officer: Officer,
    /// The page up.
    pub page: Page,
    /// The kept options.
    pub options: Options,
    /// The Options screen's page and tab.
    pub screen: crate::options::Screen,
    names: NameData,
    /// The rank was chosen by hand: a station change no longer sets it.
    rank_chosen: bool,
    /// The rank's list is open.
    rank_open: bool,
    rolls: u64,
    seed: u64,
    /// The focused item of the four (Up, Down, Enter; the mouse over one).
    focus: usize,
    /// The key art behind the menus, loaded on the first frame and freed with the lobby (design 15).
    art: Option<Option<egui::TextureHandle>>,
    /// What Options > Controls lists (`data/input/bindings.json`).
    controls: crate::bindings::Controls,
    /// The page on screen and when it came up, egui's clock: a new page fades in from the ground (design 8).
    shown: Option<(Page, f64)>,
}

impl Lobby {
    /// A menu with the kept officer, or a freshly rolled one from `seed` (the client's clock: a new crew each start).
    pub fn new(names: NameData, seed: u64, controls: crate::bindings::Controls) -> Self {
        let kept = std::fs::read_to_string(OFFICER_FILE)
            .ok()
            .and_then(|t| serde_json::from_str::<Officer>(&t).ok())
            .filter(|o| o.rank < names.ranks.len() && o.station < STATIONS.len() && !o.name.trim().is_empty());
        let mut l = Self {
            officer: Officer { rank: 0, name: String::new(), station: 1, server: String::new() },
            page: Page::Main,
            options: Options::load(),
            screen: crate::options::Screen::default(),
            rank_chosen: kept.is_some(),
            rank_open: false,
            names,
            rolls: 0,
            seed,
            focus: 0,
            art: None,
            controls,
            shown: None,
        };
        match kept {
            // A station that is not built any more is Any.
            Some(o) => {
                l.officer = o;
                if !CARDS.iter().any(|(i, _)| *i == l.officer.station) {
                    l.officer.station = 7;
                }
            }
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

    /// The focus one item down the four (a scripted run's Down key).
    pub fn focus_down(&mut self) {
        self.focus = (self.focus + 1).min(MAIN.len());
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
        if self.art.is_none() {
            self.art = Some(look::load_art(ctx, Art::Menu));
        }
        // Esc on a page goes back to the four items (an open list closes first).
        if self.page != Page::Main && ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            if self.rank_open {
                self.rank_open = false;
            } else {
                self.page = Page::Main;
            }
            return LobbyAction::None;
        }
        let page = self.page;
        let now = ctx.input(|i| i.time);
        if self.shown.is_none_or(|(p, _)| p != page) {
            self.shown = Some((page, now));
        }
        let since = self.shown.map_or(now, |s| s.1);
        let fade = ((now - since) * 1000.0 / f64::from(look().f.motion_ms.screen_fade)).clamp(0.0, 1.0) as f32;
        let veil = egui::LayerId::new(egui::Order::Foreground, egui::Id::new("lobby-fade"));
        if fade < 1.0 {
            look::cover(&ctx.layer_painter(veil), look::ground(1.0 - fade));
        }
        look::screen_area(ctx, "lobby", egui::Order::Background, |ui, screen| {
            let art = self.art.as_ref().and_then(Option::as_ref);
            look::cover_art(ui.painter(), screen, art, 0.5, 1.0);
            let flat = Kit::new(ui.painter().clone(), screen.min, 1.0);
            let (w, h) = (screen.width(), screen.height());
            match page {
                Page::Main => {
                    flat.hgrad(
                        0.0,
                        0.0,
                        w,
                        h,
                        &[
                            (0.0, look::ground(0.96)),
                            (0.3, look::ground(0.82)),
                            (0.62, look::ground(0.0)),
                            (1.0, look::ground(0.0)),
                        ],
                    );
                    self.main_page(ui, screen)
                }
                Page::Single | Page::Multi => {
                    flat.hgrad(
                        0.0,
                        0.0,
                        w,
                        h,
                        &[
                            (0.0, look::ground(0.97)),
                            (0.36, look::ground(0.9)),
                            (0.7, look::ground(0.15)),
                            (1.0, look::ground(0.15)),
                        ],
                    );
                    self.game_page(ui, screen)
                }
                Page::Options => {
                    look::cover(ui.painter(), look::ground(0.9));
                    let web = cfg!(target_os = "emscripten");
                    if self.screen.show(ui, screen, &mut self.options, &self.controls, "BACK", web) {
                        self.page = Page::Main;
                    }
                    LobbyAction::None
                }
            }
        })
    }

    /// The four items (front-menu design 1; ui-art-direction shot menu-after): the wordmark, the subtitle and a column
    /// of Title items hung from the middle left, the focused one raised with the accent bar; the version bottom right.
    fn main_page(&mut self, ui: &mut egui::Ui, screen: egui::Rect) -> LobbyAction {
        let c = look().c();
        let web = cfg!(target_os = "emscripten");
        // The browser cannot close its own page: there is no QUIT there (browser-page-fit design 6), rather than an
        // item that does nothing.
        let rows = if web { MAIN.len() } else { MAIN.len() + 1 };
        let (up, down, enter) = ui.input(|i| {
            (i.key_pressed(egui::Key::ArrowUp), i.key_pressed(egui::Key::ArrowDown), i.key_pressed(egui::Key::Enter))
        });
        if up {
            self.focus = self.focus.saturating_sub(1);
        }
        if down {
            self.focus = (self.focus + 1).min(rows - 1);
        }
        let k = kit(ui.painter(), screen, Anchor::MidLeft, Vec2::new(760.0, 1080.0));
        // The wordmark is art, not type (design 4): 128 px, two lines 0.9 apart, one picture.
        k.wordmark(96.0, 150.0, 128.0, "STAR\nCREW");
        k.track_em(0.42).text(Role::Label, 96.0, 402.4, c.dim.0, "Bridge simulator", Align::Start);
        let item_h = look().f.sizes_px.item_h;
        let mut go = None;
        let mut action = LobbyAction::None;
        for i in 0..rows {
            let (word, page) = MAIN.get(i).map_or(("QUIT", None), |(w, p)| (*w, Some(*p)));
            let y = 546.4 + i as f32 * item_h;
            let r = hit(ui, k.r(96.0, y, 520.0, item_h), word);
            if r.hovered() {
                self.focus = i;
            }
            k.item(96.0, y, 520.0, word, i == self.focus, Role::Title);
            if r.clicked() || (enter && i == self.focus) {
                self.focus = i;
                match page {
                    Some(p) => go = Some(p),
                    None => action = LobbyAction::Quit,
                }
            }
        }
        let br = kit(ui.painter(), screen, Anchor::BottomRight, Vec2::new(640.0, 80.0));
        let version = env!("CARGO_PKG_VERSION").rsplit_once('.').map_or(env!("CARGO_PKG_VERSION"), |v| v.0);
        br.text(
            Role::Label,
            640.0 - 96.0,
            80.0 - 54.0 - br.line(Role::Label),
            c.faint.0,
            &format!("Early access {version}"),
            Align::End,
        );
        if let Some(p) = go {
            self.page = p;
        }
        action
    }

    /// Single Player and Multiplayer (front-menu design 2 and 3; ui-art-direction shot lobby-after): Esc Back, your
    /// officer (rank, name, the dice), the station cards, then the page's actions.
    fn game_page(&mut self, ui: &mut egui::Ui, screen: egui::Rect) -> LobbyAction {
        let (c, sz) = (look().c(), &look().f.sizes_px);
        let single = self.page == Page::Single;
        let k = kit(ui.painter(), screen, Anchor::MidLeft, Vec2::new(840.0, 1080.0));
        let mut action = LobbyAction::None;
        // Esc Back.
        let w = k.keys(96.0, 84.0, &[KeyTok::Key("Esc".into())]);
        let bw = k.text_mid(Role::Label, 96.0 + w + 12.0, 84.0, c.dim.0, "Back", Align::Start);
        if hit(ui, k.r(96.0, 62.0, w + 12.0 + bw, 44.0), "BACK").clicked() {
            self.page = Page::Main;
            return action;
        }
        k.text(
            Role::Label,
            96.0,
            178.0,
            c.accent.0,
            if single { "Single player" } else { "Multiplayer" },
            Align::Start,
        );
        k.text(Role::Title, 96.0, 210.0, c.text.0, "Your officer", Align::Start);
        // Rank, name and the dice.
        let y = 278.0;
        k.panel(96.0, y, 220.0, 64.0);
        let rank = self.names.ranks[self.officer.rank].name.clone();
        k.text_mid(Role::Body, 116.0, y + 32.0, c.text.0, &rank, Align::Start);
        k.glyph("chevron", 96.0 + 220.0 - 31.0, y + 32.0, 22.0, c.dim.0);
        let rank_hit = hit(ui, k.r(96.0, y, 220.0, 64.0), "rank");
        k.cv.rect(324.0, y, 360.0, 64.0, 0.0, c.panel.0, Some(crate::vg::Pen::new(1.0, c.line2.0)));
        let field = k.r(344.0, y + 8.0, 320.0, 48.0);
        let font = egui::FontId::new(k.size(Role::Body) * k.cv.s, egui::FontFamily::Name(crate::vg::SEMIBOLD.into()));
        let name = egui::TextEdit::singleline(&mut self.officer.name)
            .char_limit(NAME_MAX_CHARS)
            .frame(false)
            .font(font)
            .text_color(c.text.0)
            .vertical_align(egui::Align::Center);
        ui.put(field, name);
        k.panel(692.0, y, 64.0, 64.0);
        k.glyph("dice", 724.0, y + 32.0, 30.0, c.text.0);
        if hit(ui, k.r(692.0, y, 64.0, 64.0), "New name").clicked() {
            self.roll();
        }
        k.text(Role::Label, 96.0, 390.0, c.dim.0, "Station", Align::Start);
        let cards_top = 430.0;
        for (n, (i, role)) in CARDS.iter().enumerate() {
            let (cx, cy) = (96.0 + (n % 4) as f32 * 160.0, cards_top + (n / 4) as f32 * 120.0);
            let on = self.officer.station == *i;
            if on {
                k.cv.rect(cx, cy, 152.0, 112.0, 0.0, c.raised.0, Some(crate::vg::Pen::new(1.0, c.accent.0)));
                k.fill(cx, cy + 108.0, 152.0, 4.0, c.accent.0);
            } else {
                k.panel(cx, cy, 152.0, 112.0);
            }
            let col = if *role == "any" { c.dim.0 } else { look().role(role) };
            k.role_icon(role, cx + 76.0, cy + 38.0, 36.0, col);
            let word = STATIONS[*i];
            k.track_em(0.1).text(
                Role::Label,
                cx + 76.0 + 1.0,
                cy + 68.0,
                if on { c.text.0 } else { c.dim.0 },
                word,
                Align::Middle,
            );
            if !self.rank_open && hit(ui, k.r(cx, cy, 152.0, 112.0), &format!("station {word}")).clicked() {
                self.officer.station = *i;
                if !self.rank_chosen {
                    self.officer.rank = self.default_rank();
                }
            }
        }
        let ready = self.ready();
        let primary = |k: &Kit, ui: &mut egui::Ui, y: f32, word: &str| -> bool {
            let k = if ready { k.clone() } else { k.faded(0.45) };
            k.fill(96.0, y, 632.0, sz.primary_h, c.accent.0);
            k.text_mid(Role::Title, 96.0 + 316.0, y + sz.primary_h / 2.0, c.ink.0, word, Align::Middle);
            hit(ui, k.r(96.0, y, 632.0, sz.primary_h), &word.to_uppercase()).clicked() && ready
        };
        if single {
            // The main action is the biggest control (CLAUDE.md 10).
            let beam_y = cards_top + 232.0 + 56.0;
            if primary(&k, ui, beam_y, "Beam aboard") {
                action = LobbyAction::Solo;
            }
            let ey = beam_y + sz.primary_h + 12.0;
            k.edge(96.0, ey, 632.0, sz.secondary_h, c.line2.0);
            k.text_mid(
                Role::Label,
                96.0 + 316.0,
                ey + sz.secondary_h / 2.0,
                c.text.0,
                "Explore the ship",
                Align::Middle,
            );
            if hit(ui, k.r(96.0, ey, 632.0, sz.secondary_h), "EXPLORE THE SHIP").clicked() && ready {
                action = LobbyAction::Tour;
            }
        } else {
            let sy = cards_top + 232.0 + 40.0;
            k.text(Role::Label, 96.0, sy, c.dim.0, "Server", Align::Start);
            let fy = sy + 40.0;
            k.cv.rect(96.0, fy, 632.0, 64.0, 0.0, c.panel.0, Some(crate::vg::Pen::new(1.0, c.line2.0)));
            let font =
                egui::FontId::new(k.size(Role::Body) * k.cv.s, egui::FontFamily::Name(crate::vg::SEMIBOLD.into()));
            let server = egui::TextEdit::singleline(&mut self.officer.server)
                .char_limit(64)
                .frame(false)
                .font(font)
                .text_color(c.text.0)
                .vertical_align(egui::Align::Center);
            ui.put(k.r(116.0, fy + 8.0, 592.0, 48.0), server);
            let jy = fy + 64.0 + 24.0;
            let can = ready && !self.officer.server.trim().is_empty();
            let jk = if can { k.clone() } else { k.faded(0.45) };
            jk.fill(96.0, jy, 632.0, sz.primary_h, c.accent.0);
            jk.text_mid(Role::Title, 96.0 + 316.0, jy + sz.primary_h / 2.0, c.ink.0, "Join", Align::Middle);
            if hit(ui, k.r(96.0, jy, 632.0, sz.primary_h), "JOIN").clicked() && can {
                action = LobbyAction::Join;
            }
        }
        // The rank's list, over the cards, last so it takes the clicks.
        if rank_hit.clicked() {
            self.rank_open = !self.rank_open;
        }
        if self.rank_open {
            let ranks: Vec<String> = self.names.ranks.iter().map(|r| r.name.clone()).collect();
            let ly = y + 64.0;
            k.panel(96.0, ly, 220.0, 56.0 * ranks.len() as f32);
            for (i, r) in ranks.iter().enumerate() {
                let ry = ly + 56.0 * i as f32;
                let h = hit(ui, k.r(96.0, ry, 220.0, 56.0), &format!("rank {r}"));
                if h.hovered() || i == self.officer.rank {
                    k.fill(97.0, ry, 218.0, 56.0, c.raised.0);
                }
                if i == self.officer.rank {
                    k.bar(96.0, ry, 56.0);
                }
                k.text_mid(Role::Body, 116.0, ry + 28.0, c.text.0, r, Align::Start);
                if h.clicked() {
                    self.officer.rank = i;
                    self.rank_chosen = true;
                    self.rank_open = false;
                }
            }
        }
        let enter = ui.input(|i| i.key_pressed(egui::Key::Enter)) && !ui.ctx().wants_keyboard_input();
        if enter && self.ready() && single {
            action = LobbyAction::Solo;
        }
        action
    }

    fn ready(&self) -> bool {
        !self.officer.name.trim().is_empty()
    }
}
