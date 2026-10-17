//! The release check (openspec/changes/ui-art-direction design 13 and 14, tasks 5.1 and 5.3): every screen a player
//! can reach outside the consoles, laid out headless at 1280 x 720 and 1920 x 1080 in the game's fonts and style, its
//! texts collected from egui's output and held to the release bar: no text holds a forbidden string
//! (`data/ui/forbidden_strings.json`), no two texts are drawn over one another, every text is whole on the screen and
//! inside its clip, and nothing is cut with an ellipsis. A failure names the screen, the size and the text.
//!
//! The scripted captures of the same screens (task 5.2) are `sc-client --release-shots`, run by `scripts/check.sh`
//! beside this test. It lives in the client because the screens are the client's; the consoles keep their own check
//! (console-parity).

use egui::{Pos2, Rect};
use sc_core::data::{Checks, Validate};
use serde::Deserialize;

/// `data/ui/forbidden_strings.json` (`starcrew.ui-forbidden/1`).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Forbidden {
    pub schema: String,
    /// Refused anywhere in a text, as written.
    pub contains: Vec<String>,
    /// Refused as a whole word, as written.
    pub words: Vec<String>,
    /// Refuse four numbers joined by dots (a raw address).
    pub ip_address: bool,
}

impl Validate for Forbidden {
    fn validate(&self, c: &mut Checks) {
        c.equals("schema", &self.schema, "starcrew.ui-forbidden/1");
        for (i, s) in self.contains.iter().chain(&self.words).enumerate() {
            if s.trim().is_empty() {
                c.equals(&format!("entry {i}"), s, "a piece of text");
            }
        }
    }
}

/// The file's path.
pub const FORBIDDEN_FILE: &str = "data/ui/forbidden_strings.json";

impl Forbidden {
    /// The shipped list.
    pub fn shipped() -> Self {
        sc_core::data::parse(FORBIDDEN_FILE, include_str!("../../../data/ui/forbidden_strings.json"))
            .unwrap_or_else(|e| panic!("{e}"))
    }

    /// What in `text` is forbidden, if anything.
    pub fn find(&self, text: &str) -> Option<String> {
        if let Some(s) = self.contains.iter().find(|s| text.contains(s.as_str())) {
            return Some(s.clone());
        }
        let words: Vec<&str> = text.split(|c: char| !(c.is_alphanumeric() || c == '\'')).collect();
        if let Some(w) = self.words.iter().find(|w| words.contains(&w.as_str())) {
            return Some(w.clone());
        }
        if self.ip_address {
            for token in text.split(|c: char| !(c.is_ascii_digit() || c == '.')) {
                let parts: Vec<&str> = token.split('.').collect();
                if parts.len() == 4 && parts.iter().all(|p| !p.is_empty() && p.len() <= 3) {
                    return Some(token.to_owned());
                }
            }
        }
        None
    }
}

/// One drawn text: its words, where it is drawn and the clip it is drawn in, points.
pub struct Drawn {
    pub text: String,
    pub rect: Rect,
    pub clip: Rect,
}

/// The release sizes (design 14): the Pi's 720p and 1080p.
pub const RELEASE_PX: [(u32, u32); 2] = [(1280, 720), (1920, 1080)];

/// Every text `draw` puts on an output of `px` pixels, with the game's fonts and style, after the frames egui takes
/// to size its areas; and the output's size in points.
pub fn texts_drawn(px: (u32, u32), mut draw: impl FnMut(&egui::Context)) -> (Vec<Drawn>, egui::Vec2) {
    fn collect(shape: &egui::Shape, clip: Rect, out: &mut Vec<Drawn>) {
        match shape {
            egui::Shape::Text(t) if !t.galley.is_empty() => out.push(Drawn {
                text: t.galley.text().to_owned(),
                rect: t.galley.rect.translate(t.pos.to_vec2()),
                clip,
            }),
            egui::Shape::Vec(v) => v.iter().for_each(|s| collect(s, clip, out)),
            _ => {}
        }
    }
    let ui = crate::ui::Ui::new();
    let fit = crate::ui::Fit::new(px.0, px.1, px.0 as f32);
    let (ppp, points) = (fit.ppp, fit.points);
    let mut out = Vec::new();
    for _ in 0..4 {
        let mut raw =
            egui::RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, points)), ..Default::default() };
        if let Some(vp) = raw.viewports.get_mut(&raw.viewport_id) {
            vp.native_pixels_per_point = Some(ppp);
        }
        out = ui.ctx.run(raw, &mut draw).shapes;
    }
    let mut drawn = Vec::new();
    for c in &out {
        collect(&c.shape, c.clip_rect, &mut drawn);
    }
    (drawn, points)
}

/// Each pair of texts drawn over one another (by more than a point each way), named.
pub fn overlapping(drawn: &[Drawn]) -> Vec<String> {
    let mut bad = Vec::new();
    for (i, a) in drawn.iter().enumerate() {
        for b in &drawn[i + 1..] {
            let o = a.rect.intersect(b.rect);
            if o.width() > 1.0 && o.height() > 1.0 {
                bad.push(format!("{:?} over {:?}", a.text, b.text));
            }
        }
    }
    bad
}

/// Hold one screen to the release bar at both release sizes; every failure, named.
pub fn check(screen: &str, mut draw: impl FnMut(&egui::Context)) -> Vec<String> {
    let forbidden = Forbidden::shipped();
    let mut bad = Vec::new();
    for px in RELEASE_PX {
        let (drawn, points) = texts_drawn(px, &mut draw);
        if drawn.is_empty() {
            bad.push(format!("{screen} at {px:?}: nothing drawn"));
        }
        let whole = Rect::from_min_size(Pos2::ZERO, points).expand(0.5);
        for d in &drawn {
            if let Some(f) = forbidden.find(&d.text) {
                bad.push(format!("{screen} at {px:?}: {:?} holds {f:?}", d.text));
            }
            if !whole.contains_rect(d.rect) {
                bad.push(format!("{screen} at {px:?}: {:?} at {:?} leaves the screen", d.text, d.rect));
            }
            if !d.clip.expand(0.5).contains_rect(d.rect) {
                bad.push(format!("{screen} at {px:?}: {:?} is cut by its clip {:?}", d.text, d.clip));
            }
            if d.text.contains('\u{2026}') {
                bad.push(format!("{screen} at {px:?}: {:?} is cut with an ellipsis", d.text));
            }
        }
        for o in overlapping(&drawn) {
            bad.push(format!("{screen} at {px:?}: {o}"));
        }
    }
    bad
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hud::{self, CrewRow, Prompt};
    use crate::look::KeyTok;
    use sc_core::combat::Station;

    fn names() -> sc_core::names::NameData {
        let text = include_str!("../../../data/crew/names.json");
        sc_core::data::parse("data/crew/names.json", text).expect("names")
    }

    fn controls() -> crate::bindings::Controls {
        crate::bindings::Bindings::shipped().controls
    }

    #[test]
    fn the_forbidden_list_finds_what_it_lists_and_spares_the_games_words() {
        let f = Forbidden::shipped();
        assert_eq!(f.find("Lt. Vor (bot)").as_deref(), Some("(bot)"));
        assert_eq!(f.find("Hound at the Shoals   MUSTER   00:03").as_deref(), Some("MUSTER"));
        assert_eq!(f.find("192.168.1.20").as_deref(), Some("192.168.1.20"));
        assert_eq!(f.find("PD").as_deref(), Some("PD"));
        for fine in ["Patrol", "SPEED", "Ship's computer", "Starboard laser", "Hound at the Shoals", "0.1"] {
            assert_eq!(f.find(fine), None, "{fine} is the game's own word");
        }
    }

    #[test]
    fn the_front_menu_and_its_pages_meet_the_release_bar() {
        use crate::lobby::{Lobby, Page};
        let mut bad = Vec::new();
        for (name, page) in [
            ("front menu", Page::Main),
            ("single player", Page::Single),
            ("multiplayer", Page::Multi),
            ("options", Page::Options),
        ] {
            let mut l = Lobby::new(names(), 7, controls());
            l.officer.name = "Hurath of House Zarlen".into();
            l.page = page;
            bad.extend(check(name, |ctx| {
                l.ui(ctx);
            }));
        }
        assert!(bad.is_empty(), "{bad:#?}");
    }

    #[test]
    fn every_options_page_and_controls_tab_meets_the_release_bar() {
        use crate::options::{Options, Page, Screen, Tab};
        let ctl = controls();
        let mut bad = Vec::new();
        let mut pages = vec![
            ("options display", Page::Display, Tab::OnFoot, 0),
            ("options game", Page::Game, Tab::OnFoot, 0),
            ("controls on foot", Page::Controls, Tab::OnFoot, 0),
            ("controls touch", Page::Controls, Tab::Touch, 0),
        ];
        for i in 0..ctl.stations.len() {
            pages.push(("controls at a console", Page::Controls, Tab::Console, i));
        }
        for (name, page, tab, chip) in pages {
            let mut s = Screen { page, tab, chip };
            let mut o = Options::default();
            bad.extend(check(&format!("{name} {chip}"), |ctx| {
                crate::look::screen_area(ctx, "options", egui::Order::Middle, |ui, screen| {
                    s.show(ui, screen, &mut o, &ctl, "BACK", false);
                });
            }));
        }
        assert!(bad.is_empty(), "{bad:#?}");
    }

    fn crew() -> Vec<CrewRow> {
        let names = [
            Some("Lt. Hurath of House Zarlen"),
            Some("Lt. Sil Saren"),
            Some("Lt. Jaya Strandraari"),
            None,
            Some("Lt. Xa'lensha"),
            Some("Lt. Kiya Ashberg"),
            Some("Lt. Ni'lenru"),
            Some("Lt. Haro Lundivan"),
        ];
        Station::ALL
            .iter()
            .zip(names)
            .enumerate()
            .map(|(i, (s, n))| CrewRow {
                station: *s,
                name: n.map(str::to_owned),
                you: i == 0,
                ready: i != 3,
                npc: i > 0,
            })
            .collect()
    }

    #[test]
    fn the_game_menu_on_every_page_meets_the_release_bar() {
        use crate::game_menu::{GameMenu, MenuView};
        let ctl = controls();
        let mut bad = Vec::new();
        for seated in [false, true] {
            let v = MenuView {
                label: "Training drill 1".into(),
                title: "Hound at the Shoals".into(),
                seated,
                me: Some(("Lt. Hurath of House Zarlen".into(), "helm".into())),
                objectives: vec![
                    ("Destroy the Hound".into(), true),
                    ("Keep the Tern's hull above zero".into(), false),
                    ("Shoot down both fighters".into(), false),
                ],
                crew: crew(),
            };
            // The rows, then Leave ship's ways, then a question (Esc and Enter walk the pages).
            for keys in [
                vec![],
                vec![egui::Key::ArrowUp, egui::Key::Enter],
                vec![egui::Key::ArrowUp, egui::Key::Enter, egui::Key::Enter],
            ] {
                let mut m = GameMenu::default();
                m.escape();
                let mut o = crate::options::Options::default();
                let mut pending = keys.clone();
                bad.extend(check(&format!("game menu seated {seated} after {keys:?}"), |ctx| {
                    if let Some(k) = pending.first().copied() {
                        ctx.input_mut(|i| {
                            i.events.push(egui::Event::Key {
                                key: k,
                                physical_key: None,
                                pressed: true,
                                repeat: false,
                                modifiers: Default::default(),
                            });
                        });
                        pending.remove(0);
                    }
                    m.show(ctx, &v, &mut o, &ctl);
                }));
            }
        }
        assert!(bad.is_empty(), "{bad:#?}");
    }

    #[test]
    fn the_loading_screen_and_the_walking_hud_meet_the_release_bar() {
        let b = crate::bindings::Bindings::shipped();
        let tip =
            hud::Tip { keys: vec![KeyTok::Key("M".into())], text: "Ship map. Drag to turn it, scroll to zoom.".into() };
        let mut bad = check("loading", |ctx| {
            hud::loading(
                &ctx.layer_painter(egui::LayerId::background()),
                ctx.content_rect(),
                None,
                "Training drill 1",
                "Hound at the Shoals",
                Some(&tip),
                0.6,
            );
        });
        let e = || vec![KeyTok::Key(b.use_label.clone())];
        let mut lift = Prompt::act("Lift", e(), "Up");
        lift.acts.push((vec![KeyTok::Key(b.down_label.clone())], "Down".into()));
        let mut spray = Prompt::act("Fire", vec![KeyTok::Hold, KeyTok::Key(b.spray_label.clone())], "Spray");
        spray.bar = Some(0.8);
        let prompts = [
            Prompt::act("Helm", e(), "Sit down"),
            Prompt::act("Captain", e(), "Relieve Lt. Sa'ral"),
            Prompt::note("Captain", "Lt. Sa'ral"),
            lift,
            Prompt::act("Extinguisher", e(), "Take"),
            spray,
            Prompt::note("Fire", "Find an extinguisher"),
        ];
        for (i, p) in prompts.iter().enumerate() {
            bad.extend(check(&format!("walking prompt {i}"), |ctx| {
                let painter = ctx.layer_painter(egui::LayerId::background());
                let screen = ctx.content_rect();
                hud::objective_card(&painter, screen, "Report to the helm");
                hud::alert_chip(&painter, screen, true);
                hud::reticle(&painter, screen);
                hud::carried_bar(&painter, screen, 0.5);
                hud::prompt_plate(&painter, screen, p, 1.0, false);
            }));
        }
        bad.extend(check("first steps", |ctx| {
            let painter = ctx.layer_painter(egui::LayerId::background());
            let screen = ctx.content_rect();
            hud::objective_card(&painter, screen, "Report to the helm");
            hud::first_steps(
                &painter,
                screen,
                "Helm",
                &[KeyTok::Key("E".into())],
                Some((screen.center() - egui::vec2(0.0, 140.0), 6.0)),
            );
        }));
        assert!(bad.is_empty(), "{bad:#?}");
    }
}
