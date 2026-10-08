//! The ship's company: who the bot crew are (openspec/changes/crew-npcs, design 1 and 7), from
//! `data/crew/company.json` and the session seed.
//!
//! It lives in the core because a crew is part of a session: the server, a replay and every client must agree who is
//! aboard, their names (`crate::names`) and their departments (CLAUDE.md 6.4).

use crate::data::{Checks, Validate};
use crate::names::NameData;
use serde::Deserialize;

/// A department.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Department {
    /// Its id.
    pub id: String,
    /// Its name.
    pub name: String,
    /// Bots in it with one player aboard.
    pub count: u32,
    /// The tunic and the map marker, sRGB 0-1.
    pub colour_srgb: [f64; 3],
    /// The layout compartments it works in.
    pub rooms: Vec<String>,
}

/// The whole file.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CompanyData {
    /// `starcrew.company/1`.
    pub schema: String,
    /// Berths aboard: players and bots together never exceed it.
    pub berths: u32,
    /// How long a bot stays at a place, seconds: least and most.
    pub stay_s: [f64; 2],
    /// The departments, in the order bots leave as players join (the last first).
    pub departments: Vec<Department>,
}

impl Validate for CompanyData {
    fn validate(&self, c: &mut Checks) {
        c.equals("schema", &self.schema, "starcrew.company/1");
        c.count("berths", i64::from(self.berths), 1, 64);
        c.number("stay_s[0]", self.stay_s[0], 0.0, 600.0);
        c.number("stay_s[1]", self.stay_s[1], self.stay_s[0], 600.0);
        let total: u32 = self.departments.iter().map(|d| d.count).sum();
        c.count("departments (bots with one player)", i64::from(total), 0, i64::from(self.berths) - 1);
        for (i, d) in self.departments.iter().enumerate() {
            c.count(&format!("departments[{i}].rooms"), d.rooms.len() as i64, 1, 64);
            for (k, v) in d.colour_srgb.iter().enumerate() {
                c.number(&format!("departments[{i}].colour_srgb[{k}]"), *v, 0.0, 1.0);
            }
        }
    }
}

/// A bot crew member.
#[derive(Debug, Clone, PartialEq)]
pub struct CrewMember {
    /// The crew slot (stable for the session: names and choices are drawn from it).
    pub slot: u64,
    /// Their name.
    pub name: String,
    /// Index into `CompanyData::departments`.
    pub department: usize,
}

impl CompanyData {
    /// The bot crew for `players` players in the session seeded `session_seed`: the table's counts, less one bot for
    /// each player past the first, taken from the last department first (crew-npcs 1). Slots start after the
    /// players' berths, so a bot's name never depends on how many bots there are.
    pub fn bots(&self, names: &NameData, session_seed: u64, players: u32) -> Vec<CrewMember> {
        let mut counts: Vec<u32> = self.departments.iter().map(|d| d.count).collect();
        let mut leave = players.saturating_sub(1);
        for c in counts.iter_mut().rev() {
            let k = leave.min(*c);
            *c -= k;
            leave -= k;
        }
        let mut out = Vec::new();
        let mut slot = u64::from(self.berths);
        for (d, &n) in counts.iter().enumerate() {
            for _ in 0..n {
                out.push(CrewMember { slot, name: names.for_slot(session_seed, slot), department: d });
                slot += 1;
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::parse;

    fn company() -> CompanyData {
        parse("data/crew/company.json", include_str!("../../../data/crew/company.json")).expect("the company loads")
    }
    fn names() -> NameData {
        parse("data/crew/names.json", include_str!("../../../data/crew/names.json")).expect("the names load")
    }

    #[test]
    fn one_player_sails_with_nine_bots_and_the_same_seed_gives_the_same_crew() {
        let (c, n) = (company(), names());
        let a = c.bots(&n, 99, 1);
        assert_eq!(a.len(), 9, "2 command, 2 engineering, 2 deckhands, medical, security, galley");
        assert_eq!(a, c.bots(&n, 99, 1), "a replay has the same crew");
    }

    #[test]
    fn players_take_berths_from_the_galley_first() {
        let (c, n) = (company(), names());
        let four = c.bots(&n, 99, 4);
        assert_eq!(four.len(), 6);
        let depts: Vec<&str> = four.iter().map(|b| c.departments[b.department].id.as_str()).collect();
        assert!(
            !depts.contains(&"galley") && !depts.contains(&"security"),
            "galley and security leave first: {depts:?}"
        );
    }
}
