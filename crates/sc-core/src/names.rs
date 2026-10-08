//! Officer and crew names from `data/crew/names.json` (openspec/changes/lobby, design 2; crew-npcs design 7).
//!
//! It lives in the core because a crew's names are part of a session: the server, a replay and every client must draw
//! the same name for the same seed, and the lobby's roll and the bot crew use one generator (CLAUDE.md 6.1, 6.4).
//!
//! A style is drawn by weight, then each `{part}` of its pattern: one syllable from each of the part's slots in turn.
//! A name equal (case-blind) to an entry of the deny list, or with a word equal to one, is drawn again.

use crate::data::{Checks, Validate};
use crate::rng::Rng;
use serde::Deserialize;
use std::collections::BTreeMap;

/// A rank and its short form.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Rank {
    /// In full: "Lieutenant commander".
    pub name: String,
    /// Short: "Lt. Cmdr.".
    pub short: String,
}

/// A way of building a name.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Style {
    /// Its id, the key of its weight.
    pub id: String,
    /// The name with `{part}` placeholders.
    pub pattern: String,
    /// Each part's syllable slots (BTreeMap: a stable order, CLAUDE.md 6.4).
    pub parts: BTreeMap<String, Vec<Vec<String>>>,
}

/// The whole file.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct NameData {
    /// `starcrew.names/1`.
    pub schema: String,
    /// The ranks, lowest first.
    pub ranks: Vec<Rank>,
    /// The styles.
    pub styles: Vec<Style>,
    /// How often each style is drawn, by id.
    pub weights: BTreeMap<String, f64>,
    /// Names never generated: canon characters.
    pub deny: Vec<String>,
}

/// The longest name the lobby's field takes, characters.
pub const NAME_MAX_CHARS: usize = 24;

impl Validate for NameData {
    fn validate(&self, c: &mut Checks) {
        c.equals("schema", &self.schema, "starcrew.names/1");
        c.count("ranks", self.ranks.len() as i64, 1, 32);
        c.count("styles", self.styles.len() as i64, 1, 64);
        let mut total = 0.0;
        for (i, s) in self.styles.iter().enumerate() {
            let w = self.weights.get(&s.id).copied().unwrap_or(f64::NAN);
            c.number(&format!("weights.{}", s.id), w, 0.0, 1000.0);
            total += w;
            for (name, slots) in &s.parts {
                if !s.pattern.contains(&format!("{{{name}}}")) {
                    c.equals(&format!("styles[{i}].parts.{name}"), "unused", "in the pattern");
                }
                for (k, slot) in slots.iter().enumerate() {
                    c.count(&format!("styles[{i}].parts.{name}[{k}]"), slot.len() as i64, 1, 1000);
                }
            }
        }
        for id in self.weights.keys() {
            if !self.styles.iter().any(|s| &s.id == id) {
                c.equals(&format!("weights.{id}"), "no such style", "a style's id");
            }
        }
        c.number("weights (their sum)", total, 1e-6, 1e9);
    }
}

impl NameData {
    fn denied(&self, name: &str) -> bool {
        let deny = |w: &str| self.deny.iter().any(|d| d.eq_ignore_ascii_case(w));
        deny(name) || name.split([' ', '\'']).any(|w| !w.is_empty() && deny(w))
    }

    fn draw(&self, rng: &mut Rng) -> String {
        let total: f64 = self.styles.iter().map(|s| self.weights[&s.id]).sum();
        let mut r = rng.next_f64() * total;
        let mut style = &self.styles[self.styles.len() - 1];
        for s in &self.styles {
            let w = self.weights[&s.id];
            if r < w {
                style = s;
                break;
            }
            r -= w;
        }
        let mut out = style.pattern.clone();
        for (name, slots) in &style.parts {
            let mut part = String::new();
            for slot in slots {
                part.push_str(&slot[(rng.next_u64() % slot.len() as u64) as usize]);
            }
            out = out.replace(&format!("{{{name}}}"), &part);
        }
        out
    }

    /// A name for `seed`'s stream: the same seed, the same name; never a denied one and never longer than
    /// `NAME_MAX_CHARS`.
    pub fn generate(&self, rng: &mut Rng) -> String {
        loop {
            let n = self.draw(rng);
            if !self.denied(&n) && n.chars().count() <= NAME_MAX_CHARS {
                return n;
            }
        }
    }

    /// The name of crew slot `slot` in the session seeded `session_seed`.
    pub fn for_slot(&self, session_seed: u64, slot: u64) -> String {
        self.generate(&mut Rng::for_purpose(session_seed, slot, "crew_name"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::parse;

    fn data() -> NameData {
        parse("data/crew/names.json", include_str!("../../../data/crew/names.json")).expect("the shipped names load")
    }

    #[test]
    fn the_same_seed_gives_the_same_name() {
        let d = data();
        for slot in 0..20 {
            assert_eq!(d.for_slot(7, slot), d.for_slot(7, slot), "a replay draws the same crew");
        }
        let names: std::collections::BTreeSet<String> = (0..20).map(|s| d.for_slot(7, s)).collect();
        assert!(names.len() >= 18, "twenty slots give nearly twenty names: {names:?}");
    }

    #[test]
    fn a_denied_name_is_drawn_again() {
        let mut d = data();
        // Deny the first name a stream gives: the generator must give another.
        let first = d.generate(&mut Rng::for_purpose(1, 1, "t"));
        d.deny.push(first.clone());
        let second = d.generate(&mut Rng::for_purpose(1, 1, "t"));
        assert_ne!(first, second, "a name on the deny list is never returned");
    }

    #[test]
    fn no_generated_name_is_a_canon_character() {
        let d = data();
        for s in 0..2000 {
            let n = d.for_slot(42, s);
            assert!(!d.denied(&n) && n.chars().count() <= NAME_MAX_CHARS, "{n}");
        }
    }
}
