//! The lobby (openspec/changes/lobby, design 1): the screen before the ship, where the player names their officer,
//! picks the station they would like and beams aboard. Its names come from `sc-core::names`; the officer is kept in
//! `settings/officer.json` between sessions (design 4).
//!
//! Menus name things, they don't explain them (CLAUDE.md 10): every row is a label and a control, and the panel is a
//! fixed size whatever the name typed into it.

use egui::{Align2, Color32, RichText, Vec2};
use sc_core::names::{NameData, NAME_MAX_CHARS};
use sc_core::rng::Rng;
use serde::{Deserialize, Serialize};

/// The stations a player can ask for, in the order the lobby shows them (`bridge-stations`).
pub const STATIONS: [&str; 8] = ["Captain", "Helm", "Tactical", "Engineering", "Science", "Comms", "Flight ops", "Any"];

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
}

/// What the lobby asks of the client after a frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LobbyAction {
    /// Nothing yet.
    None,
    /// Start a solo game with the officer.
    BeamAboard,
}

/// The lobby's state.
pub struct Lobby {
    /// The officer being made.
    pub officer: Officer,
    names: NameData,
    /// The rank was chosen by hand: a station change no longer sets it.
    rank_chosen: bool,
    rolls: u64,
    seed: u64,
    code: String,
}

impl Lobby {
    /// A lobby with the kept officer, or a freshly rolled one from `seed` (the client's clock: a new crew each start).
    pub fn new(names: NameData, seed: u64) -> Self {
        let kept = std::fs::read_to_string(OFFICER_FILE)
            .ok()
            .and_then(|t| serde_json::from_str::<Officer>(&t).ok())
            .filter(|o| o.rank < names.ranks.len() && o.station < STATIONS.len() && !o.name.trim().is_empty());
        let mut l = Self {
            officer: Officer { rank: 0, name: String::new(), station: 1 },
            rank_chosen: kept.is_some(),
            names,
            rolls: 0,
            seed,
            code: String::new(),
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

    /// The rank a station suggests (design 2): Captain for the captain's chair, Lieutenant for the others.
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

    /// Draw the lobby; returns what the player asked for.
    pub fn ui(&mut self, ctx: &egui::Context) -> LobbyAction {
        let mut action = LobbyAction::None;
        let amber = Color32::from_rgb(242, 160, 70);
        let lilac = Color32::from_rgb(190, 160, 230);
        let pale = Color32::from_rgb(200, 212, 230);
        egui::Area::new(egui::Id::new("title")).anchor(Align2::CENTER_TOP, Vec2::new(0.0, 48.0)).show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.label(RichText::new("STAR CREW").size(64.0).strong().color(amber));
                ui.label(RichText::new("SCS TERN").size(22.0).color(lilac));
            });
        });
        // The panel: a fixed size (CLAUDE.md 10), centred a little below the middle.
        let size = Vec2::new(620.0, 430.0);
        egui::Window::new("lobby")
            .title_bar(false)
            .resizable(false)
            .fixed_size(size)
            .anchor(Align2::CENTER_CENTER, Vec2::new(0.0, 60.0))
            .frame(egui::Frame::window(&ctx.style()).inner_margin(22.0).corner_radius(18.0))
            .show(ctx, |ui| {
                ui.set_width(size.x);
                ui.label(RichText::new("OFFICER").size(14.0).color(amber).strong());
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
                    let r = ui.add(name);
                    if r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) && self.ready() {
                        action = LobbyAction::BeamAboard;
                    }
                    if ui.add(egui::Button::new(RichText::new("Roll").color(pale)).corner_radius(14.0)).clicked() {
                        self.roll();
                    }
                });
                ui.add_space(8.0);
                ui.label(RichText::new("STATION").size(14.0).color(amber).strong());
                egui::Grid::new("stations").num_columns(4).spacing(Vec2::new(10.0, 10.0)).show(ui, |ui| {
                    for (i, s) in STATIONS.iter().enumerate() {
                        let on = self.officer.station == i;
                        let b = egui::Button::new(RichText::new(*s).color(if on { Color32::BLACK } else { pale }))
                            .min_size(Vec2::new(140.0, 36.0))
                            .corner_radius(18.0)
                            .fill(if on { lilac } else { Color32::from_rgb(38, 46, 66) });
                        if ui.add(b).clicked() {
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
                // The main action is the biggest control (CLAUDE.md 10).
                let beam = egui::Button::new(RichText::new("BEAM ABOARD").size(28.0).strong().color(Color32::BLACK))
                    .min_size(Vec2::new(size.x, 64.0))
                    .corner_radius(32.0)
                    .fill(amber);
                if ui.add_enabled(self.ready(), beam).clicked() {
                    action = LobbyAction::BeamAboard;
                }
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("JOIN A CREW").size(14.0).color(amber).strong());
                    ui.add_enabled(
                        false,
                        egui::TextEdit::singleline(&mut self.code).char_limit(6).desired_width(110.0),
                    );
                    ui.add_enabled(false, egui::Button::new("Join").corner_radius(14.0));
                });
            });
        if ctx.input(|i| i.key_pressed(egui::Key::Enter)) && !ctx.wants_keyboard_input() && self.ready() {
            action = LobbyAction::BeamAboard;
        }
        action
    }

    fn ready(&self) -> bool {
        !self.officer.name.trim().is_empty()
    }
}
