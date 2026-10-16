//! The keys the game's bindings name (openspec/changes/leave-station design 6): `data/input/bindings.json`, read at the
//! client's start and validated, so a key can be rebound in data and, later, from Options > Controls.
//!
//! It lives in the client because keys are the platform's; the rules the keys drive are elsewhere. What has been moved
//! into data is here: the use key (sit, and held, stand), the climb-down and spray keys the walking prompts show,
//! Helm's roll and lateral strafe, the captain's lighting keys, and the Controls page's table
//! (openspec/changes/ui-art-direction design 10): every action the page lists with its keys, a key that is a binding
//! named by its field (`@use`), so the page shows what the game reads.

use crate::look::{cap_name, KeyTok};
use sc_client::platform::keys;
use sc_core::data::{Checks, Validate};
use serde::Deserialize;

/// `data/input/bindings.json` (`starcrew.bindings/1`). Keys by SDL's names ("E", "Left").
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct BindingsFile {
    /// `starcrew.bindings/1`.
    pub schema: String,
    /// What the file is and who decided it.
    pub status: String,
    /// On foot: sit at a seat in reach (and take, repair, climb); seated, held `seats.stand_hold_s`: stand up.
    #[serde(rename = "use")]
    pub use_key: String,
    /// START MISSION while the round waits (start-mission 5).
    pub start: String,
    /// On foot: climb down a ladder, a lift down a deck.
    pub down: String,
    /// On foot, held: spray the extinguisher in hand.
    pub spray: String,
    /// Helm's keys.
    pub helm: HelmKeys,
    /// The viewscreen's keys, at every console with one.
    pub viewscreen: ViewscreenKeys,
    /// The captain's lighting keys, at the captain's seat only (owner, 2026-10-15: "lighting can only be changed by
    /// the captain"); the ALERT panel's command.
    pub captain: CaptainKeys,
    /// What Options > Controls lists (ui-art-direction design 10).
    pub controls: ControlsFile,
}

/// The captain's lighting keys.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CaptainKeys {
    /// Normal lighting (red alert off).
    pub normal: String,
    /// Red alert.
    pub red_alert: String,
}

/// One group of the Controls page: a heading and its rows, each an action and its keys. A key is a binding's field
/// (`@use`, `@helm.roll_port`), a key by name (`W`, `Left Shift`), a mouse glyph (`mouse`, `click`, `drag`,
/// `wheel`), `hold` before a held key, or `/` between two ways.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GroupFile {
    /// The heading.
    pub group: String,
    /// The column it stands in on its page, 0 or 1.
    #[serde(default)]
    pub column: u8,
    /// The station whose chip shows it, on the console page.
    #[serde(default)]
    pub station: Option<String>,
    /// [action, [keys]].
    pub rows: Vec<(String, Vec<String>)>,
}

/// The Controls page's three tabs.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ControlsFile {
    /// On foot.
    pub on_foot: Vec<GroupFile>,
    /// At any console.
    pub any_console: GroupFile,
    /// Each station's own, chosen by its chip.
    pub stations: Vec<GroupFile>,
    /// Touch's words beside its drawing (the touch-controls layout); its keys are the touch layer's buttons' names.
    pub touch: Vec<GroupFile>,
}

/// The viewscreen's bindings (chase-orbit design 5).
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ViewscreenKeys {
    /// ORBIT on or off, while CHASE is on.
    pub orbit: String,
}

/// Helm's bindings that are data (the rest are `seat.rs`'s, as the mockup has them).
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct HelmKeys {
    /// Roll to port, while held.
    pub roll_port: String,
    /// Roll to starboard, while held.
    pub roll_starboard: String,
    /// Strafe to port, while held.
    pub strafe_port: String,
    /// Strafe to starboard, while held.
    pub strafe_starboard: String,
}

/// The names a binding may use and their keycodes.
const NAMES: [(&str, u32); 39] = [
    ("1", keys::N1),
    ("2", keys::N2),
    ("A", keys::A),
    ("C", keys::C),
    ("D", keys::D),
    ("E", keys::E),
    ("F", keys::F),
    ("G", keys::G),
    ("K", keys::K),
    ("L", keys::L),
    ("M", keys::M),
    ("O", keys::O),
    ("Q", keys::Q),
    ("R", keys::R),
    ("S", keys::S),
    ("T", keys::T),
    ("V", keys::V),
    ("W", keys::W),
    ("X", keys::X),
    ("Y", keys::Y),
    ("Z", keys::Z),
    ("Space", keys::SPACE),
    ("Tab", keys::TAB),
    ("Return", keys::RETURN),
    ("Backspace", keys::BACKSPACE),
    ("Delete", keys::DELETE),
    ("Left Shift", keys::LSHIFT),
    ("Right Shift", keys::RSHIFT),
    ("Left Ctrl", keys::LCTRL),
    ("Left", keys::LEFT),
    ("Right", keys::RIGHT),
    ("Up", keys::UP),
    ("Down", keys::DOWN),
    ("Home", keys::HOME),
    ("End", keys::END),
    ("Escape", keys::ESCAPE),
    ("F1", keys::F1),
    ("F3", keys::F3),
    ("F12", keys::F12),
];

/// The keycode of a binding's name, if the client knows it.
pub fn key_of(name: &str) -> Option<u32> {
    NAMES.iter().find(|(n, _)| *n == name).map(|(_, k)| *k)
}

impl BindingsFile {
    /// Every key binding by its field, as the Controls table names it (`use`, `helm.roll_port`).
    fn fields(&self) -> [(&'static str, &String); 10] {
        let h = &self.helm;
        [
            ("use", &self.use_key),
            ("start", &self.start),
            ("down", &self.down),
            ("spray", &self.spray),
            ("helm.roll_port", &h.roll_port),
            ("helm.roll_starboard", &h.roll_starboard),
            ("helm.strafe_port", &h.strafe_port),
            ("helm.strafe_starboard", &h.strafe_starboard),
            ("viewscreen.orbit", &self.viewscreen.orbit),
            ("captain.normal", &self.captain.normal),
        ]
    }

    /// A row's key token as drawn: a binding's field resolved to its key, the rest as they are.
    fn token(&self, t: &str) -> Option<KeyTok> {
        Some(match t {
            "hold" => KeyTok::Hold,
            "/" => KeyTok::Or,
            "mouse" | "click" | "drag" | "wheel" => KeyTok::Glyph(t.to_owned()),
            _ => match t.strip_prefix('@') {
                Some("captain.red_alert") => KeyTok::Key(cap_name(&self.captain.red_alert)),
                Some(f) => KeyTok::Key(cap_name(self.fields().iter().find(|(n, _)| *n == f)?.1)),
                None => KeyTok::Key(cap_name(t)),
            },
        })
    }
}

impl Validate for BindingsFile {
    fn validate(&self, c: &mut Checks) {
        c.equals("schema", &self.schema, "starcrew.bindings/1");
        let mut all: Vec<(&str, &String)> = self.fields().to_vec();
        all.push(("captain.red_alert", &self.captain.red_alert));
        for (field, name) in &all {
            if key_of(name).is_none() {
                c.equals(field, name, "a key the client knows (bindings.rs NAMES)");
            }
        }
        // Seated at the helm every one of the helm's and the viewscreen's keys is live at once, with the use key that
        // stands: no key twice.
        let seated =
            ["use", "start", "helm.roll_port", "helm.roll_starboard", "helm.strafe_port", "helm.strafe_starboard"];
        let seated: Vec<&(&str, &String)> =
            all.iter().filter(|(f, _)| seated.contains(f) || *f == "viewscreen.orbit").collect();
        for (i, (fa, a)) in seated.iter().enumerate() {
            if let Some((fb, _)) = seated[..i].iter().find(|(_, b)| b == a) {
                c.equals(fa, a, &format!("a key not bound already to {fb}"));
            }
        }
        // On foot the use, down and spray keys are live at once.
        let foot = [("use", &self.use_key), ("down", &self.down), ("spray", &self.spray)];
        for (i, (fa, a)) in foot.iter().enumerate() {
            if let Some((fb, _)) = foot[..i].iter().find(|(_, b)| b == a) {
                c.equals(fa, a, &format!("a key not bound already to {fb}"));
            }
        }
        // The Controls table: every key a binding's field, a key the client knows, a glyph, hold or "/".
        let ctl = &self.controls;
        let keyed = ctl
            .on_foot
            .iter()
            .map(|g| ("controls.on_foot", g))
            .chain(std::iter::once(("controls.any_console", &ctl.any_console)))
            .chain(ctl.stations.iter().map(|g| ("controls.stations", g)));
        for (path, g) in keyed {
            if g.column > 1 {
                c.equals(&format!("{path}.{}.column", g.group), &g.column.to_string(), "0 or 1");
            }
            for (action, toks) in &g.rows {
                for t in toks {
                    let known = match t.strip_prefix('@') {
                        Some(f) => {
                            self.token(t).is_some()
                                && (f == "captain.red_alert" || self.fields().iter().any(|(n, _)| *n == f))
                        }
                        None => {
                            matches!(t.as_str(), "hold" | "/" | "mouse" | "click" | "drag" | "wheel")
                                || key_of(t).is_some()
                        }
                    };
                    if !known {
                        c.equals(
                            &format!("{path}.{}.{action}", g.group),
                            t,
                            "a binding (@field), a key the client knows, a mouse glyph, hold or /",
                        );
                    }
                }
            }
        }
        for g in &ctl.stations {
            if g.station.as_deref().is_none_or(|s| {
                sc_core::combat::Station::ALL.iter().all(|st| st.id() != s) && s != "laser" && s != "command"
            }) {
                c.equals(
                    &format!("controls.stations.{}.station", g.group),
                    g.station.as_deref().unwrap_or(""),
                    "a station's role id",
                );
            }
        }
    }
}

/// One group of the Controls page, its keys resolved for drawing.
#[derive(Clone, Debug, PartialEq)]
pub struct Group {
    /// The heading.
    pub name: String,
    /// Its column, 0 or 1.
    pub column: u8,
    /// The station's role id, for the console page's chips.
    pub station: Option<String>,
    /// Each action and its keys.
    pub rows: Vec<(String, Vec<KeyTok>)>,
}

/// The Controls page, resolved.
#[derive(Clone, Debug, PartialEq)]
pub struct Controls {
    pub on_foot: Vec<Group>,
    pub any_console: Group,
    pub stations: Vec<Group>,
    pub touch: Vec<Group>,
}

/// The bindings as keycodes, and the use key's label for the screen.
#[derive(Clone, Debug, PartialEq)]
pub struct Bindings {
    /// Sit; held while seated, stand.
    pub use_key: u32,
    /// Its name, drawn on LEAVE STATION's keycap and in the walking prompts.
    pub use_label: String,
    /// START MISSION (start-mission 5).
    pub start_key: u32,
    /// Its name, drawn on START MISSION's keycap.
    pub start_label: String,
    /// Climb down, a lift down a deck.
    pub down_key: u32,
    /// Its keycap's name.
    pub down_label: String,
    /// Spray the extinguisher, held.
    pub spray_key: u32,
    /// Its keycap's name.
    pub spray_label: String,
    /// The captain's normal lighting and red alert.
    pub lights: (u32, u32),
    /// What Options > Controls lists.
    pub controls: Controls,
    /// Every binding's field and its keycap's name (`use` is E), for a key named by its field elsewhere (the tips).
    pub fields: Vec<(String, String)>,
    /// Helm: roll to port and to starboard, strafe to port and to starboard.
    pub roll: (u32, u32),
    /// See `roll`.
    pub strafe: (u32, u32),
    /// ORBIT on or off (chase-orbit design 5).
    pub orbit: u32,
}

impl Bindings {
    fn of(f: &BindingsFile) -> Self {
        let k = |n: &str| key_of(n).unwrap_or(0);
        let h = &f.helm;
        let group = |g: &GroupFile| Group {
            name: g.group.clone(),
            column: g.column,
            station: g.station.clone(),
            rows: g
                .rows
                .iter()
                .map(|(a, toks)| (a.clone(), toks.iter().filter_map(|t| f.token(t)).collect()))
                .collect(),
        };
        let ctl = &f.controls;
        Self {
            use_key: k(&f.use_key),
            use_label: cap_name(&f.use_key),
            down_key: k(&f.down),
            down_label: cap_name(&f.down),
            spray_key: k(&f.spray),
            spray_label: cap_name(&f.spray),
            lights: (k(&f.captain.normal), k(&f.captain.red_alert)),
            fields: f
                .fields()
                .iter()
                .map(|(n, v)| ((*n).to_owned(), cap_name(v)))
                .chain(std::iter::once(("captain.red_alert".to_owned(), cap_name(&f.captain.red_alert))))
                .collect(),
            controls: Controls {
                on_foot: ctl.on_foot.iter().map(group).collect(),
                any_console: group(&ctl.any_console),
                stations: ctl.stations.iter().map(group).collect(),
                touch: ctl.touch.iter().map(group).collect(),
            },
            start_key: k(&f.start),
            start_label: f.start.clone(),
            roll: (k(&h.roll_port), k(&h.roll_starboard)),
            strafe: (k(&h.strafe_port), k(&h.strafe_starboard)),
            orbit: k(&f.viewscreen.orbit),
        }
    }

    /// Keys named as the Controls table names them (`@use`, `hold`, `M`, `click`), drawn; None when a field is unknown.
    pub fn tokens(&self, toks: &[String]) -> Option<Vec<KeyTok>> {
        toks.iter()
            .map(|t| match t.as_str() {
                "hold" => Some(KeyTok::Hold),
                "/" => Some(KeyTok::Or),
                "mouse" | "click" | "drag" | "wheel" => Some(KeyTok::Glyph(t.clone())),
                _ => match t.strip_prefix('@') {
                    Some(f) => self.fields.iter().find(|(n, _)| n == f).map(|(_, k)| KeyTok::Key(k.clone())),
                    None => key_of(t).map(|_| KeyTok::Key(cap_name(t))),
                },
            })
            .collect()
    }

    /// The shipped bindings, for tests and the console fixture.
    pub fn shipped() -> Self {
        Self::of(&parse(include_str!("../../../data/input/bindings.json")).expect("the shipped bindings are valid"))
    }
}

impl Default for Bindings {
    fn default() -> Self {
        Self::shipped()
    }
}

/// The file's path, from the repository's root (the browser build embeds `data/`).
pub const BINDINGS_FILE: &str = "data/input/bindings.json";

fn parse(text: &str) -> Result<BindingsFile, String> {
    sc_core::data::parse(BINDINGS_FILE, text).map_err(|e| e.to_string())
}

/// Read and check `data/input/bindings.json`; a file that fails stops the client (CLAUDE.md 6.5).
pub fn load() -> Result<Bindings, String> {
    let text = std::fs::read_to_string(BINDINGS_FILE).map_err(|e| format!("{BINDINGS_FILE}: {e}"))?;
    parse(&text).map(|f| Bindings::of(&f))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SHIPPED: &str = include_str!("../../../data/input/bindings.json");

    #[test]
    fn the_shipped_bindings_sit_and_stand_on_e_and_roll_the_helm_on_z_and_c() {
        let b = Bindings::shipped();
        assert_eq!((b.use_key, b.use_label.as_str()), (keys::E, "E"));
        assert_eq!(b.roll, (keys::Z, keys::C), "the helm rolls off E (leave-station 3)");
        assert_eq!(b.strafe, (keys::LEFT, keys::RIGHT));
        assert_eq!(b.orbit, keys::O, "ORBIT on O at every console (chase-orbit design 5)");
        assert_eq!((b.start_key, b.start_label.as_str()), (keys::RETURN, "Return"), "START MISSION on Return");
    }

    #[test]
    fn an_unknown_key_a_key_bound_twice_and_an_unknown_field_are_refused() {
        let bad = SHIPPED.replace("\"use\": \"E\"", "\"use\": \"Hyperspace\"");
        assert!(parse(&bad).unwrap_err().contains("use"), "an unknown key");
        let twice = SHIPPED.replace("\"roll_port\": \"Z\"", "\"roll_port\": \"E\"");
        assert!(parse(&twice).unwrap_err().contains("roll_port"), "E is the use key already");
        let extra = SHIPPED.replacen('{', "{\"jump\": \"Space\",", 1);
        assert!(parse(&extra).is_err(), "an unknown field");
        let bad_row = SHIPPED.replacen("[\"Jump\", [\"Space\"]]", "[\"Jump\", [\"Hyperspace\"]]", 1);
        assert!(parse(&bad_row).unwrap_err().contains("Jump"), "a Controls row with a key the client does not know");
        let bad_field = SHIPPED.replacen("\"@down\"", "\"@crouch\"", 1);
        assert!(parse(&bad_field).is_err(), "a Controls row naming a binding that does not exist");
    }

    #[test]
    fn the_controls_page_lists_the_lift_keys_and_puts_the_lights_under_the_captain() {
        // The spec's scenarios (ui-art-direction): Lift with Up a deck E, Down a deck Q and Call the lift E; the
        // lighting keys only under Captain on the console page.
        let c = Bindings::shipped().controls;
        let lift = c.on_foot.iter().find(|g| g.name == "Lift").expect("a Lift group");
        let key = |k: &str| vec![KeyTok::Key(k.into())];
        assert_eq!(
            lift.rows,
            vec![("Up a deck".into(), key("E")), ("Down a deck".into(), key("Q")), ("Call the lift".into(), key("E"))]
        );
        let lights: Vec<&Group> = c
            .on_foot
            .iter()
            .chain(std::iter::once(&c.any_console))
            .chain(&c.stations)
            .filter(|g| g.rows.iter().any(|(a, _)| a.contains("lighting") || a == "Red alert"))
            .collect();
        assert_eq!(lights.len(), 1, "the lighting keys are listed once");
        assert_eq!(lights[0].station.as_deref(), Some("command"), "under the captain");
        let roll = c.stations.iter().find(|g| g.name == "Helm").and_then(|g| g.rows.iter().find(|r| r.0 == "Roll"));
        assert_eq!(
            roll.map(|r| r.1.clone()),
            Some(vec![KeyTok::Key("Z".into()), KeyTok::Key("C".into())]),
            "the bound roll keys"
        );
    }
}
