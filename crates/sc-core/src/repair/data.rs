//! The repair data's schemas (openspec/changes/repair-minigames design 8): the rules every job follows, each game's
//! own file (its words, its cover, its knobs by level), the kit's rates from `damage.json`, and the Tern's jobs.
//!
//! They live in the core because the server checks a round against the same numbers the client plays it at, and
//! `sc-tools check-data` validates the same structs (CLAUDE.md 6.5).

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::data::{Checks, DataError, Validate};

/// The rule files every game reads, in load order.
pub const RULE_FILES: [&str; 2] = ["data/repairs/rules.json", "data/ships/tern/damage.json"];

/// The Tern's jobs.
pub const SHIP_FILE: &str = "data/ships/tern/repairs.json";

/// A game's data file.
pub fn game_file(id: &str) -> String {
    format!("data/repairs/{id}.json")
}

/// How a job starts from one state (design 2's table).
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct StateRule {
    /// Integrity where the damage left it, percent.
    pub start_pct: f64,
    /// Rounds to 100%.
    pub rounds: u32,
    /// Whether the first round opens by fitting a spare part.
    pub part: bool,
}

/// The three states a job is repaired from.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct States {
    /// 25-75%.
    pub damaged: StateRule,
    /// Under 25%.
    pub disabled: StateRule,
    /// 0%.
    pub destroyed: StateRule,
}

/// `data/repairs/rules.json` (`starcrew.repair-rules/1`).
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RulesFile {
    /// `starcrew.repair-rules/1`.
    pub schema: String,
    /// Rounds and start by state.
    pub states: States,
    /// Fumbles in one round that restart it.
    pub fumbles_per_round: u32,
    /// Round n is played at level min(n, this).
    pub max_level: u32,
}

impl Validate for RulesFile {
    fn validate(&self, c: &mut Checks) {
        c.equals("schema", &self.schema, "starcrew.repair-rules/1");
        for (k, s) in [
            ("damaged", &self.states.damaged),
            ("disabled", &self.states.disabled),
            ("destroyed", &self.states.destroyed),
        ] {
            c.number(&format!("states.{k}.start_pct"), s.start_pct, 0.0, 99.0);
            c.count(&format!("states.{k}.rounds"), i64::from(s.rounds), 1, 12);
        }
        c.count("fumbles_per_round", i64::from(self.fumbles_per_round), 1, 10);
        c.count("max_level", i64::from(self.max_level), 1, 12);
    }
}

/// The kit's rates, percent of integrity a second (`damage-control` 6a).
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct KitRates {
    /// Every player.
    pub officer: f64,
    /// A rating, a bot or a damage control team.
    pub rating: f64,
}

/// Spare parts a job takes (`damage-control` 6).
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Parts {
    /// A disabled system's first round.
    pub disabled_system: u32,
    /// A destroyed system's rebuild.
    pub destroyed_system: u32,
    /// A conduit splice.
    pub conduit_splice: u32,
    /// A power node rebuilt.
    pub node_rebuild: u32,
}

/// `damage.json`'s `repair` block, the one source of the kit's rates and the fumble's cost.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct DamageRepair {
    /// The kit's rates.
    pub kit_pct_per_s: KitRates,
    /// Where the kits are.
    pub kit_locker: String,
    /// How many.
    pub kits_in_locker: u32,
    /// Where the parts are.
    pub parts_store: String,
    /// How many.
    pub parts_on_board: u32,
    /// Parts by job.
    pub parts: Parts,
    /// Where a disabled system's part takes it, percent.
    pub disabled_to_pct: f64,
    /// A destroyed system's rebuild, seconds at the officer's rate.
    pub destroyed_rebuild_s: f64,
    /// Where the rebuild takes it, percent.
    pub destroyed_rebuild_to_pct: f64,
    /// A conduit splice, seconds.
    pub conduit_splice_s: f64,
    /// What a splice carries, of the conduit's rating.
    pub conduit_splice_capacity: f64,
    /// A power node's rebuild, seconds.
    pub node_rebuild_s: f64,
    /// What a rebuilt node carries.
    pub node_rebuild_to: f64,
    /// EVA hull repair, percent a second.
    pub hull_section_pct_per_s_eva: f64,
    /// Two at a job: times the faster one's rate.
    pub two_hands_factor: f64,
    /// A fumble's cost, percent of the job (repair-minigames design 1).
    pub fumble_share_pct: f64,
}

/// `damage.json`, of which the repair rules read only `repair`; the rest belongs to `damage-control`'s engine work.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct DamageFile {
    /// The repair block.
    pub repair: DamageRepair,
}

impl Validate for DamageFile {
    fn validate(&self, c: &mut Checks) {
        let r = &self.repair;
        c.number("repair.kit_pct_per_s.officer", r.kit_pct_per_s.officer, 0.01, 100.0);
        c.number("repair.kit_pct_per_s.rating", r.kit_pct_per_s.rating, 0.01, 100.0);
        c.number("repair.fumble_share_pct", r.fumble_share_pct, 0.0, 50.0);
        c.number("repair.two_hands_factor", r.two_hands_factor, 1.0, 2.0);
        c.count("repair.parts.disabled_system", i64::from(r.parts.disabled_system), 0, 10);
        c.count("repair.parts.destroyed_system", i64::from(r.parts.destroyed_system), 0, 10);
    }
}

/// What a fumble does to the room (repairs-on-deck 5).
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Hazard {
    /// What the bar says, a few words.
    pub text: String,
    /// Hit points taken from each body within `radius_m`.
    pub hp: f64,
    /// Metres from the repair point.
    pub radius_m: f64,
}

/// One step of the how-to card (design 6g).
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GuideStep {
    /// The kit's icon.
    pub icon: String,
    /// Three to six words.
    pub text: String,
    /// Lit while the cover's screws are worked.
    #[serde(default)]
    pub cover: bool,
}

/// The how-to card.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Guide {
    /// Up to four steps, in order.
    pub steps: Vec<GuideStep>,
    /// The mistake: cause and cost, one line.
    pub mistake: String,
}

/// A cover plate over the machine, canvas pixels (kit: access panels).
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Cover {
    /// Left.
    pub x: f64,
    /// Top.
    pub y: f64,
    /// Width.
    pub w: f64,
    /// Height.
    pub h: f64,
    /// 4 or 6.
    pub screws: u32,
}

/// A job a game keeps for itself (the medic's hit points).
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct OwnJob {
    /// What the bar counts: "HP".
    pub unit: String,
    /// Where it starts.
    pub start: f64,
    /// Where it ends.
    pub target: f64,
    /// Rounds.
    pub rounds: u32,
    /// A slip's cost, in the unit.
    pub fumble: f64,
    /// The rate with no game, unit a second.
    pub rate_per_s: f64,
}

/// A tuning knob by level (design 1a): a value at level 1 and a change a level, held inside `min` and `max`; or one
/// value a level, the last repeated past the end.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Knob {
    /// At level 1.
    #[serde(default)]
    pub base: Option<f64>,
    /// Added each level after the first.
    #[serde(default)]
    pub per_level: Option<f64>,
    /// Never below.
    #[serde(default)]
    pub min: Option<f64>,
    /// Never above.
    #[serde(default)]
    pub max: Option<f64>,
    /// One value a level instead.
    #[serde(default)]
    pub levels: Option<Vec<f64>>,
}

impl Knob {
    /// The value at `level` (1 up).
    pub fn at(&self, level: u32) -> f64 {
        let n = level.max(1) - 1;
        let v = match (&self.levels, self.base) {
            (Some(l), _) if !l.is_empty() => l[(n as usize).min(l.len() - 1)],
            (_, Some(b)) => b + self.per_level.unwrap_or(0.0) * f64::from(n),
            _ => 0.0,
        };
        let v = self.min.map_or(v, |m| v.max(m));
        self.max.map_or(v, |m| v.min(m))
    }

    fn check(&self, c: &mut Checks, field: &str) {
        match (&self.levels, self.base) {
            (Some(l), None) if !l.is_empty() && self.per_level.is_none() => {
                for (i, v) in l.iter().enumerate() {
                    c.number(&format!("{field}.levels.{i}"), *v, -1e9, 1e9);
                }
            }
            (None, Some(b)) => {
                c.number(&format!("{field}.base"), b, -1e9, 1e9);
                if let Some(p) = self.per_level {
                    c.number(&format!("{field}.per_level"), p, -1e9, 1e9);
                }
            }
            _ => c.equals(field, "neither or both", "base (with per_level) or levels"),
        }
        if let (Some(lo), Some(hi)) = (self.min, self.max) {
            c.number(&format!("{field}.max"), hi, lo, 1e9);
        }
    }
}

/// `data/repairs/<game>.json` (`starcrew.repair-game/1`).
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct GameFile {
    /// `starcrew.repair-game/1`.
    pub schema: String,
    /// The game's id, the file's name.
    pub id: String,
    /// The system's name.
    pub title: String,
    /// Where its repair point is.
    pub place: String,
    /// The menu's column.
    pub group: String,
    /// What a fumble does.
    pub hazard: Hazard,
    /// What the ship loses while it is down.
    pub down: String,
    /// The how-to card.
    pub guide: Guide,
    /// The cover, if the machine has one in the menu.
    #[serde(default)]
    pub cover: Option<Cover>,
    /// The word when it is done, "Repaired" unless said.
    #[serde(default)]
    pub done_word: Option<String>,
    /// A job the game keeps for itself.
    #[serde(default)]
    pub job: Option<OwnJob>,
    /// The game's knobs by name, the unit in the name.
    pub knobs: BTreeMap<String, Knob>,
    /// The shortest round the server takes, seconds, by level (repairs-on-deck 6).
    pub min_round_s: Vec<f64>,
    /// Whether `min_round_s` has been measured from people's play (design 8: until then, provisional).
    pub min_round_measured: bool,
}

impl GameFile {
    /// The knob `name` at `level`. A knob the file lacks is a bug the load catches (`needs`); here it is zero.
    pub fn knob(&self, name: &str, level: u32) -> f64 {
        self.knobs.get(name).map_or(0.0, |k| k.at(level))
    }

    /// The shortest round at `level`.
    pub fn min_round(&self, level: u32) -> f64 {
        let i = (level.max(1) as usize - 1).min(self.min_round_s.len().saturating_sub(1));
        self.min_round_s.get(i).copied().unwrap_or(0.0)
    }

    /// Check the file holds exactly the knobs a game uses: every one named, none it does not know.
    pub fn needs(&self, file: &str, knobs: &[&str]) -> Result<(), DataError> {
        let err = |field: String, message: String| DataError { file: file.to_owned(), field, message };
        for k in knobs {
            if !self.knobs.contains_key(*k) {
                return Err(err(format!("knobs.{k}"), "the game uses this knob; the file lacks it".into()));
            }
        }
        for k in self.knobs.keys() {
            if !knobs.contains(&k.as_str()) {
                return Err(err(format!("knobs.{k}"), "the game has no such knob".into()));
            }
        }
        Ok(())
    }
}

/// Icons the guide card can draw (the kit's set).
pub const ICONS: [&str; 20] = [
    "tap",
    "drag",
    "hold",
    "turn",
    "screw",
    "swap",
    "sweep",
    "rhythm",
    "band",
    "order",
    "rubber",
    "alternate",
    "slider",
    "match",
    "valve",
    "pipes",
    "flow",
    "aim",
    "tool",
    "plug",
];

impl Validate for GameFile {
    fn validate(&self, c: &mut Checks) {
        c.equals("schema", &self.schema, "starcrew.repair-game/1");
        c.number("hazard.hp", self.hazard.hp, 0.0, 100.0);
        c.number("hazard.radius_m", self.hazard.radius_m, 0.0, 20.0);
        c.count("guide.steps", self.guide.steps.len() as i64, 1, 4);
        for (i, s) in self.guide.steps.iter().enumerate() {
            if !ICONS.contains(&s.icon.as_str()) {
                c.equals(&format!("guide.steps.{i}.icon"), &s.icon, "one of the kit's icons");
            }
        }
        if let Some(p) = &self.cover {
            c.number("cover.x", p.x, 0.0, 1280.0);
            c.number("cover.y", p.y, 72.0, 720.0);
            c.number("cover.w", p.w, 40.0, 1280.0 - p.x);
            c.number("cover.h", p.h, 40.0, 720.0 - p.y);
            c.count("cover.screws", i64::from(p.screws), 4, 6);
        }
        if let Some(j) = &self.job {
            c.number("job.target", j.target, j.start + 1.0, 1e4);
            c.count("job.rounds", i64::from(j.rounds), 1, 12);
            c.number("job.fumble", j.fumble, 0.0, j.target);
            c.number("job.rate_per_s", j.rate_per_s, 0.01, 1e3);
        }
        for (k, v) in &self.knobs {
            v.check(c, &format!("knobs.{k}"));
        }
        c.count("min_round_s", self.min_round_s.len() as i64, 1, 12);
        for (i, v) in self.min_round_s.iter().enumerate() {
            c.number(&format!("min_round_s.{i}"), *v, 0.0, 600.0);
        }
    }
}

/// One of the ship's repair jobs (repairs-on-deck 5).
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ShipJob {
    /// Stable id.
    pub id: String,
    /// The game.
    pub game: String,
    /// The layout's compartment.
    pub place: String,
    /// Players who can work it at once.
    pub workers: u32,
}

/// `data/ships/tern/repairs.json` (`starcrew.ship-repairs/1`).
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ShipRepairs {
    /// `starcrew.ship-repairs/1`.
    pub schema: String,
    /// The jobs.
    pub jobs: Vec<ShipJob>,
}

impl Validate for ShipRepairs {
    fn validate(&self, c: &mut Checks) {
        c.equals("schema", &self.schema, "starcrew.ship-repairs/1");
        for (i, j) in self.jobs.iter().enumerate() {
            c.count(&format!("jobs.{i}.workers"), i64::from(j.workers), 1, 2);
            if self.jobs[..i].iter().any(|o| o.id == j.id) {
                c.equals(&format!("jobs.{i}.id"), &j.id, "an id no other job has");
            }
        }
    }
}

/// Every repair file, loaded and cross-checked: the rules, the kit's rates, the ship's jobs and each game a job names.
#[derive(Debug, Clone, PartialEq)]
pub struct RepairData {
    /// `data/repairs/rules.json`.
    pub rules: RulesFile,
    /// `damage.json`'s repair block.
    pub damage: DamageRepair,
    /// The Tern's jobs.
    pub ship: ShipRepairs,
    /// Each game's file, by id, in id order.
    pub games: BTreeMap<String, GameFile>,
}

impl RepairData {
    /// Parse the files: `rules` and `damage` as `RULE_FILES`, `ship` as `SHIP_FILE`, and every game's (id, text).
    pub fn parse(rules: &str, damage: &str, ship: &str, games: &[(String, String)]) -> Result<Self, DataError> {
        let rules: RulesFile = crate::data::parse(RULE_FILES[0], rules)?;
        let damage = crate::data::parse::<DamageFile>(RULE_FILES[1], damage)?.repair;
        let ship: ShipRepairs = crate::data::parse(SHIP_FILE, ship)?;
        let mut out = BTreeMap::new();
        for (id, text) in games {
            let file = game_file(id);
            let g: GameFile = crate::data::parse(&file, text)?;
            if g.id != *id {
                return Err(DataError {
                    file,
                    field: "id".into(),
                    message: format!("must be the file's name, {id:?}"),
                });
            }
            out.insert(id.clone(), g);
        }
        for (i, j) in ship.jobs.iter().enumerate() {
            if !out.contains_key(&j.game) {
                return Err(DataError {
                    file: SHIP_FILE.into(),
                    field: format!("jobs.{i}.game"),
                    message: format!("no game {:?} in data/repairs/", j.game),
                });
            }
        }
        Ok(Self { rules, damage, ship, games: out })
    }

    /// Read every file under `root` with `read` (the caller owns the I/O: the core never reads a file), the games
    /// being every `data/repairs/<id>.json` but the rules, as `ids` lists them.
    pub fn load(ids: &[String], read: impl Fn(&str) -> Result<String, String>) -> Result<Self, String> {
        let games = ids.iter().map(|id| Ok((id.clone(), read(&game_file(id))?))).collect::<Result<Vec<_>, String>>()?;
        Self::parse(&read(RULE_FILES[0])?, &read(RULE_FILES[1])?, &read(SHIP_FILE)?, &games).map_err(|e| e.to_string())
    }
}
