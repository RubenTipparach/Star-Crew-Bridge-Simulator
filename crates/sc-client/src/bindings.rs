//! The keys the game's bindings name (openspec/changes/leave-station design 6): `data/input/bindings.json`, read at the
//! client's start and validated, so a key can be rebound in data and, later, from Options > Controls.
//!
//! It lives in the client because keys are the platform's; the rules the keys drive are elsewhere. Only what has been
//! moved into data is here: the use key (sit, and held, stand) and Helm's roll and lateral strafe.

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
    /// Helm's keys.
    pub helm: HelmKeys,
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
const NAMES: [(&str, u32); 36] = [
    ("A", keys::A),
    ("C", keys::C),
    ("D", keys::D),
    ("E", keys::E),
    ("F", keys::F),
    ("G", keys::G),
    ("K", keys::K),
    ("L", keys::L),
    ("M", keys::M),
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

impl Validate for BindingsFile {
    fn validate(&self, c: &mut Checks) {
        c.equals("schema", &self.schema, "starcrew.bindings/1");
        let h = &self.helm;
        let all = [
            ("use", &self.use_key),
            ("helm.roll_port", &h.roll_port),
            ("helm.roll_starboard", &h.roll_starboard),
            ("helm.strafe_port", &h.strafe_port),
            ("helm.strafe_starboard", &h.strafe_starboard),
        ];
        for (field, name) in all {
            if key_of(name).is_none() {
                c.equals(field, name, "a key the client knows (bindings.rs NAMES)");
            }
        }
        // Seated at the helm every one of these is live at once: no key twice.
        for (i, (fa, a)) in all.iter().enumerate() {
            if let Some((fb, _)) = all[..i].iter().find(|(_, b)| b == a) {
                c.equals(fa, a, &format!("a key not bound already to {fb}"));
            }
        }
    }
}

/// The bindings as keycodes, and the use key's label for the screen.
#[derive(Clone, Debug, PartialEq)]
pub struct Bindings {
    /// Sit; held while seated, stand.
    pub use_key: u32,
    /// Its name, drawn on LEAVE STATION's keycap.
    pub use_label: String,
    /// Helm: roll to port and to starboard, strafe to port and to starboard.
    pub roll: (u32, u32),
    /// See `roll`.
    pub strafe: (u32, u32),
}

impl Bindings {
    fn of(f: &BindingsFile) -> Self {
        let k = |n: &str| key_of(n).unwrap_or(0);
        let h = &f.helm;
        Self {
            use_key: k(&f.use_key),
            use_label: f.use_key.clone(),
            roll: (k(&h.roll_port), k(&h.roll_starboard)),
            strafe: (k(&h.strafe_port), k(&h.strafe_starboard)),
        }
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
    }

    #[test]
    fn an_unknown_key_a_key_bound_twice_and_an_unknown_field_are_refused() {
        let bad = SHIPPED.replace("\"use\": \"E\"", "\"use\": \"Hyperspace\"");
        assert!(parse(&bad).unwrap_err().contains("use"), "an unknown key");
        let twice = SHIPPED.replace("\"roll_port\": \"Z\"", "\"roll_port\": \"E\"");
        assert!(parse(&twice).unwrap_err().contains("roll_port"), "E is the use key already");
        let extra = SHIPPED.replacen('{', "{\"jump\": \"Space\",", 1);
        assert!(parse(&extra).is_err(), "an unknown field");
    }
}
