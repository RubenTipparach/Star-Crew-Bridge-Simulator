//! Crew chat (openspec/changes/crew-chat): the kinds of line, the vocabulary the NPC crew speak from
//! (`data/crew/chat.json`), the one mapping from a decision to its line, an NPC's pacing ([`Voice`]) and the lines the
//! server keeps ([`ChatLog`]).
//!
//! It lives in the core because three speakers must say the same thing for the same decision: a bot client
//! (`sc-net::bot`), the drill's automation for an empty seat, and the server's events (damage control, the computer).
//! One mapping, three callers (CLAUDE.md 6.1). A line is only ever said for a command that was sent or an event that
//! happened: nothing here invents one. No clock, no socket: callers pass the time.

use std::collections::{BTreeMap, VecDeque};

use serde::Deserialize;

use super::damage::{DamageEvent, Job, ShipLayout};
use super::data::DrillData;
use super::{Command, HelmMode, PowerCmd, Station, TurretMode};
use crate::data::{Checks, DataError, Validate};

/// The vocabulary's file, relative to the repository's root.
pub const CHAT_FILE: &str = "data/crew/chat.json";
/// The longest line on the wire, in characters: a player's typing is cut here, and no template may pass it.
pub const MAX_CHAT_CHARS: usize = 80;

/// What a line is (crew-chat design 2). The wire carries it as its discriminant.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Kind {
    /// A player's typing.
    Say = 0,
    /// The captain, to a station.
    Order = 1,
    /// A station's "aye" to the captain's order.
    Ack = 2,
    /// A station saying what it does, as it does it.
    Intent = 3,
    /// An outcome.
    Report = 4,
}

impl Kind {
    /// Every kind, in wire order.
    pub const ALL: [Kind; 5] = [Kind::Say, Kind::Order, Kind::Ack, Kind::Intent, Kind::Report];

    /// From its wire byte.
    pub fn from_u8(v: u8) -> Option<Self> {
        Self::ALL.get(usize::from(v)).copied()
    }

    /// Its id in the vocabulary file (`say`, `order`, `ack`, `intent`, `report`).
    pub fn id(self) -> &'static str {
        match self {
            Kind::Say => "say",
            Kind::Order => "order",
            Kind::Ack => "ack",
            Kind::Intent => "intent",
            Kind::Report => "report",
        }
    }

    /// From its id.
    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|k| k.id() == id)
    }
}

/// How a kind looks: one colour and one shape, the same on every screen (CLAUDE.md 10).
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct KindStyle {
    /// Its label (a legend, a tooltip).
    pub label: String,
    /// Its colour, sRGB 0-255.
    pub colour: [u8; 3],
    /// Its shape: `chevron`, `tick`, `arrow`, `square` or `none`.
    pub shape: String,
}

/// The shapes a kind may take.
pub const SHAPES: [&str; 5] = ["chevron", "tick", "arrow", "square", "none"];

/// What keeps the channel readable (crew-chat design 4).
#[derive(Debug, Clone, Copy, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Limits {
    /// The longest NPC template, characters.
    pub line_chars: usize,
    /// The lines the server keeps and sends a late joiner.
    pub keep_lines: usize,
    /// A player's least time between lines, seconds: the server drops a faster one.
    pub speaker_gap_s: f64,
    /// An NPC's least time between lines, seconds.
    pub npc_gap_s: f64,
    /// An NPC never says the same text again within this, seconds.
    pub repeat_s: f64,
    /// The in-game strip's lines.
    pub strip_lines: usize,
    /// How long a line stays in the strip, seconds.
    pub strip_fade_s: f64,
    /// The log window's lines seen at once.
    pub log_lines: usize,
}

/// The server's own voices.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Speakers {
    /// The ship's computer: hull, shields, breaches, a kill.
    pub computer: String,
    /// The damage-control teams.
    pub damage_control: String,
}

/// One line of the vocabulary.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Template {
    /// Its kind's id.
    pub kind: String,
    /// The text, with `{slot}`s.
    pub text: String,
    /// How important it is, 1 first: an NPC with several lines to say says the lowest.
    pub rank: u8,
}

/// `data/crew/chat.json`: the kinds, the limits, the server's speakers and every line the NPC crew can say.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ChatData {
    /// `starcrew.chat/1`.
    pub schema: String,
    /// Each kind's look, by kind id.
    pub kinds: BTreeMap<String, KindStyle>,
    /// The limits.
    pub limits: Limits,
    /// The server's voices.
    pub speakers: Speakers,
    /// The computer reports the hull and the shields as they fall past each of these, percent of full.
    pub report_steps_pct: Vec<f64>,
    /// The captain's order for each verb, by station id then verb.
    pub orders: BTreeMap<String, BTreeMap<String, String>>,
    /// A station's ack of each verb, by station id then verb.
    pub acks: BTreeMap<String, BTreeMap<String, String>>,
    /// Every other line, by id.
    pub lines: BTreeMap<String, Template>,
    /// The layout's room names by id (from `layout.json`, bound at load).
    #[serde(skip)]
    pub room_names: BTreeMap<String, String>,
    /// The layout's system names by id (from `layout.json`, bound at load).
    #[serde(skip)]
    pub system_names: BTreeMap<String, String>,
    /// The enemy's class name, `{target}` (bound at load).
    #[serde(skip)]
    pub target: String,
}

/// The line ids the engine speaks: the file holds each of them and no other.
pub const LINE_IDS: [&str; 51] = [
    "seated",
    "alert_on",
    "alert_off",
    "brace",
    "helm_course",
    "helm_chase",
    "helm_match",
    "helm_evade",
    "helm_hold",
    "helm_stop",
    "helm_orient",
    "helm_hunt",
    "lock",
    "lock_clear",
    "turrets_free",
    "turrets_hold",
    "shields",
    "tube_load",
    "tube_fire",
    "decoy",
    "laser_fire",
    "scan",
    "ping",
    "band",
    "power_preset",
    "power_reroute",
    "fire_team",
    "repair_team",
    "fire_out",
    "fire_spread",
    "repaired",
    "target_destroyed",
    "hull",
    "shields_low",
    "breach",
    // The patrol (patrol-mission design 9).
    "shore_off",
    "shore_on",
    "undock",
    "jump_spool",
    "jump_abort",
    "sweep",
    "order_undock",
    "order_jump",
    "drive_ready",
    "clamps_released",
    "clear_of_berth",
    "mass_lock",
    "arrived",
    "sweep_clear",
    "sweep_contact",
    "home",
];

/// The slots a template may name.
pub const SLOTS: [&str; 11] =
    ["target", "tube", "station", "preset", "band", "room", "system", "pct", "heading", "pitch", "place"];

/// The slots in `text`, or the first problem with its braces.
fn slots_of(text: &str) -> Result<Vec<&str>, String> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(open) = rest.find(['{', '}']) {
        if rest.as_bytes()[open] == b'}' {
            return Err("a '}' with no '{'".into());
        }
        let after = &rest[open + 1..];
        let close = after.find('}').ok_or("a '{' with no '}'")?;
        out.push(&after[..close]);
        rest = &after[close + 1..];
    }
    Ok(out)
}

impl Validate for ChatData {
    fn validate(&self, c: &mut Checks) {
        c.equals("schema", &self.schema, "starcrew.chat/1");
        for k in Kind::ALL {
            match self.kinds.get(k.id()) {
                None => c.equals(&format!("kinds.{}", k.id()), "(missing)", "a kind's look"),
                Some(s) => {
                    c.count(&format!("kinds.{}.label", k.id()), s.label.chars().count() as i64, 1, 12);
                    if !SHAPES.contains(&s.shape.as_str()) {
                        c.equals(&format!("kinds.{}.shape", k.id()), &s.shape, "chevron, tick, arrow, square or none");
                    }
                }
            }
        }
        for id in self.kinds.keys() {
            if Kind::from_id(id).is_none() {
                c.equals(&format!("kinds.{id}"), id, "say, order, ack, intent or report");
            }
        }
        let l = &self.limits;
        c.count("limits.line_chars", l.line_chars as i64, 8, MAX_CHAT_CHARS as i64);
        c.count("limits.keep_lines", l.keep_lines as i64, 1, 200);
        c.number("limits.speaker_gap_s", l.speaker_gap_s, 0.0, 60.0);
        c.number("limits.npc_gap_s", l.npc_gap_s, 0.0, 60.0);
        c.number("limits.repeat_s", l.repeat_s, 0.0, 600.0);
        c.count("limits.strip_lines", l.strip_lines as i64, 1, 8);
        c.number("limits.strip_fade_s", l.strip_fade_s, 1.0, 60.0);
        c.count("limits.log_lines", l.log_lines as i64, 1, l.keep_lines as i64);
        c.count("speakers.computer", self.speakers.computer.chars().count() as i64, 1, super::MAX_NAME_CHARS as i64);
        c.count(
            "speakers.damage_control",
            self.speakers.damage_control.chars().count() as i64,
            1,
            super::MAX_NAME_CHARS as i64,
        );
        for (i, p) in self.report_steps_pct.iter().enumerate() {
            let below = if i == 0 { 100.0 } else { self.report_steps_pct[i - 1] };
            c.number(&format!("report_steps_pct[{i}] (falling)"), *p, 0.0, below - 1e-9);
        }
        let mut text = |field: &str, t: &str| {
            c.count(&format!("{field} (characters)"), t.chars().count() as i64, 1, l.line_chars as i64);
            match slots_of(t) {
                Err(e) => c.equals(field, &e, "a template whose braces close"),
                Ok(s) => {
                    for slot in s {
                        if !SLOTS.contains(&slot) {
                            c.equals(field, &format!("{{{slot}}}"), "a slot the engine fills");
                        }
                    }
                }
            }
        };
        for (what, table) in [("orders", &self.orders), ("acks", &self.acks)] {
            for (st, verbs) in table {
                for (verb, t) in verbs {
                    text(&format!("{what}.{st}.{verb}"), t);
                }
            }
        }
        for (id, t) in &self.lines {
            text(&format!("lines.{id}"), &t.text);
        }
        for (id, t) in &self.lines {
            if !LINE_IDS.contains(&id.as_str()) {
                c.equals(&format!("lines.{id}"), id, "a line the engine speaks");
            }
            if Kind::from_id(&t.kind).is_none() {
                c.equals(&format!("lines.{id}.kind"), &t.kind, "say, order, ack, intent or report");
            }
            c.count(&format!("lines.{id}.rank"), i64::from(t.rank), 1, 9);
        }
        for id in LINE_IDS {
            if !self.lines.contains_key(id) {
                c.equals(&format!("lines.{id}"), "(missing)", "a line the engine speaks");
            }
        }
    }
}

/// A name as it reads inside a sentence: its first letter lower case unless it starts an acronym ("CO2").
fn in_sentence(name: &str) -> String {
    let mut cs = name.chars();
    match (cs.next(), cs.next()) {
        (Some(a), Some(b)) if a.is_uppercase() && !b.is_uppercase() => {
            a.to_lowercase().chain(name.chars().skip(1)).collect()
        }
        _ => name.to_owned(),
    }
}

impl ChatData {
    /// Check the vocabulary against the drill's other files and take what it needs from them: an order and an ack
    /// for every verb the captain can send (`stations.json`), the layout's room and system names, the enemy's name.
    pub fn bind(
        &mut self,
        file: &str,
        verbs: &BTreeMap<String, Vec<[String; 2]>>,
        layout_text: &str,
        target: &str,
    ) -> Result<(), DataError> {
        let err = |field: String, message: String| DataError { file: file.to_owned(), field, message };
        for (what, table) in [("orders", &self.orders), ("acks", &self.acks)] {
            for (st, vs) in verbs {
                for [verb, _] in vs {
                    if !table.get(st).is_some_and(|t| t.contains_key(verb)) {
                        return Err(err(format!("{what}.{st}"), format!("has no line for the verb {verb:?}")));
                    }
                }
            }
            for (st, t) in table {
                for verb in t.keys() {
                    if !verbs.get(st).is_some_and(|vs| vs.iter().any(|v| &v[0] == verb)) {
                        return Err(err(format!("{what}.{st}.{verb}"), "is no verb of data/stations.json".into()));
                    }
                }
            }
        }
        let v: serde_json::Value = serde_json::from_str(layout_text).map_err(|e| err(".".into(), e.to_string()))?;
        let names = |key: &str| -> BTreeMap<String, String> {
            v.get(key)
                .and_then(serde_json::Value::as_array)
                .map(|a| {
                    a.iter()
                        .filter_map(|x| Some((x.get("id")?.as_str()?.to_owned(), x.get("name")?.as_str()?.to_owned())))
                        .collect()
                })
                .unwrap_or_default()
        };
        self.room_names = names("compartments");
        self.system_names = names("systems");
        self.target = target.to_owned();
        Ok(())
    }

    /// A room's name inside a sentence, by its layout id (the id itself if the layout names none).
    pub fn room(&self, id: &str) -> String {
        in_sentence(self.room_names.get(id).map_or(id, String::as_str))
    }

    /// A system's name inside a sentence, by its layout id (the id, spaced, if the layout names none).
    pub fn system(&self, id: &str) -> String {
        self.system_names.get(id).map_or_else(|| id.replace('_', " "), |n| in_sentence(n))
    }

    /// A kind's look.
    pub fn style(&self, k: Kind) -> Option<&KindStyle> {
        self.kinds.get(k.id())
    }

    /// The text with its slots filled, its first letter upper case, cut to [`MAX_CHAT_CHARS`].
    fn fill(&self, text: &str, args: &[(&str, &str)]) -> String {
        let mut out = text.to_owned();
        out = out.replace("{target}", &self.target);
        for (slot, value) in args {
            out = out.replace(&format!("{{{slot}}}"), value);
        }
        let mut cs = out.chars();
        let first: String = cs.next().map(|c| c.to_uppercase().collect()).unwrap_or_default();
        first.chars().chain(cs).take(MAX_CHAT_CHARS).collect()
    }

    /// The line `id` with its slots filled; none for an id the file does not hold.
    pub fn line(&self, id: &str, args: &[(&str, &str)]) -> Option<Said> {
        let t = self.lines.get(id)?;
        Some(Said { kind: Kind::from_id(&t.kind)?, text: self.fill(&t.text, args), rank: t.rank })
    }

    /// The captain's order to `to`, by the verb's name. Orders come first (rank 1).
    pub fn order(&self, to: Station, verb: &str) -> Option<Said> {
        let t = self.orders.get(to.id())?.get(verb)?;
        Some(Said { kind: Kind::Order, text: self.fill(t, &[("station", to.name())]), rank: 1 })
    }

    /// Station `st`'s ack of an order, by the verb's name (rank 2).
    pub fn ack(&self, st: Station, verb: &str) -> Option<Said> {
        let t = self.acks.get(st.id())?.get(verb)?;
        Some(Said { kind: Kind::Ack, text: self.fill(t, &[("station", st.name())]), rank: 2 })
    }
}

/// A line to say: its kind, its text and how important it is (1 first).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Said {
    /// Its kind.
    pub kind: Kind,
    /// Its text.
    pub text: String,
    /// Its rank, 1 first.
    pub rank: u8,
}

/// The line for a command station `from` sends: the one mapping a bot and the drill's automation share. None for a
/// command that says nothing (the stick, an ack: see [`said_ack`]).
pub fn said_for(data: &DrillData, from: Station, cmd: &Command) -> Option<Said> {
    let chat = &data.chat;
    let st = from.name();
    match *cmd {
        Command::Alert(true) => chat.line("alert_on", &[]),
        Command::Alert(false) => chat.line("alert_off", &[]),
        Command::Brace(true) => chat.line("brace", &[]),
        Command::Order(to, verb) => chat.order(to, &data.order_verbs(to).get(usize::from(verb))?[0]),
        Command::Helm(m) => chat.line(
            match m {
                HelmMode::Hold => "helm_hold",
                HelmMode::Course => "helm_course",
                HelmMode::Chase => "helm_chase",
                HelmMode::Match => "helm_match",
                HelmMode::Evade => "helm_evade",
            },
            &[],
        ),
        Command::AllStop => chat.line("helm_stop", &[]),
        Command::Orient(Some([h, p, _])) => {
            let heading = format!("{:03}", (h.round() as i64).rem_euclid(360));
            let pitch = format!("{}", p.round() as i64);
            chat.line("helm_orient", &[("heading", &heading), ("pitch", &pitch)])
        }
        Command::Lock(Some(_)) => chat.line("lock", &[]),
        Command::Lock(None) => chat.line("lock_clear", &[]),
        Command::TurretMode(_, TurretMode::Auto | TurretMode::Target) => chat.line("turrets_free", &[]),
        Command::TurretMode(_, TurretMode::Hold) => chat.line("turrets_hold", &[]),
        Command::Preset(i) => {
            let p = data.tern_shields.presets.get(usize::from(i))?;
            chat.line("shields", &[("preset", &p.id)])
        }
        Command::Load(i) => chat.line("tube_load", &[("tube", &(u32::from(i) + 1).to_string())]),
        Command::Fire(i) => chat.line("tube_fire", &[("tube", &(u32::from(i) + 1).to_string())]),
        Command::Decoy => chat.line("decoy", &[]),
        Command::LaserFire(_) => chat.line("laser_fire", &[("station", st)]),
        Command::Scan(true) => chat.line("scan", &[]),
        Command::Ping => chat.line("ping", &[]),
        Command::Freq(b) => {
            let band = char::from(b'A' + b.min(25)).to_string();
            chat.line("band", &[("band", &band)])
        }
        Command::Power(PowerCmd::Preset(i)) => {
            let presets = data.power.presets();
            let p = presets.get(usize::from(i))?;
            chat.line("power_preset", &[("preset", &p.to_lowercase())])
        }
        Command::Power(PowerCmd::Run(_, true) | PowerCmd::Feed(..)) => chat.line("power_reroute", &[]),
        // The patrol (patrol-mission design 9).
        Command::Shore(false) => chat.line("shore_off", &[]),
        Command::Shore(true) => chat.line("shore_on", &[]),
        Command::Undock => chat.line("undock", &[]),
        Command::Jump(true) => chat.line("jump_spool", &[]),
        Command::Jump(false) => chat.line("jump_abort", &[]),
        Command::Sweep => chat.line("sweep", &[]),
        Command::Patrol(super::patrol::PatrolOrder::Undock) => chat.line("order_undock", &[]),
        Command::Patrol(super::patrol::PatrolOrder::Jump) => chat.line("order_jump", &[]),
        _ => None,
    }
}

/// Station `st`'s ack of the captain's order, the verb by its index in that station's verbs.
pub fn said_ack(data: &DrillData, st: Station, verb: u8) -> Option<Said> {
    data.chat.ack(st, &data.order_verbs(st).get(usize::from(verb))?[0])
}

/// "At {station}": a crew member sat down at `st`.
pub fn said_seated(data: &DrillData, st: Station) -> Option<Said> {
    data.chat.line("seated", &[("station", st.name())])
}

/// What damage control says of an event aboard the Tern: a team sent, a fire out or spreading, a system back.
pub fn said_damage(data: &DrillData, layout: &ShipLayout, e: &DamageEvent) -> Option<Said> {
    let chat = &data.chat;
    let room = |r: usize| layout.rooms.get(r).map(|x| chat.room(&x.id));
    let system =
        |l: usize| layout.loads.get(l).map(|x| chat.system(if x.system.is_empty() { &x.id } else { &x.system }));
    match *e {
        DamageEvent::Dispatched { job: Job::Fire(r), .. } => chat.line("fire_team", &[("room", &room(r)?)]),
        DamageEvent::Dispatched { job: Job::Repair(l), .. } => chat.line("repair_team", &[("system", &system(l)?)]),
        DamageEvent::FireOut { room: r } => chat.line("fire_out", &[("room", &room(r)?)]),
        DamageEvent::FireSpread { to, .. } => chat.line("fire_spread", &[("room", &room(to)?)]),
        DamageEvent::Repaired { load } => chat.line("repaired", &[("system", &system(load)?)]),
        DamageEvent::Arrived { .. } | DamageEvent::HullBurned { .. } => None,
    }
}

/// The index of the lowest of `steps` (falling percentages) that `pct` is under, past `from`: the next report step
/// reached, if any.
pub fn step_reached(steps: &[f64], from: usize, pct: f64) -> Option<usize> {
    steps.iter().enumerate().skip(from).filter(|(_, s)| pct < **s).map(|(i, _)| i).next_back()
}

/// A player's typed line as the server keeps it: no control characters, trimmed, at most [`MAX_CHAT_CHARS`]; none if
/// nothing is left.
pub fn clean_text(text: &str) -> Option<String> {
    let t: String =
        text.chars().filter(|c| !c.is_control()).collect::<String>().trim().chars().take(MAX_CHAT_CHARS).collect();
    let t = t.trim_end().to_owned();
    (!t.is_empty()).then_some(t)
}

/// An NPC's pacing (crew-chat design 4): it says at most one line each `npc_gap_s`, the most important of those it
/// had to say (the newest of a tie), and never the same text twice within `repeat_s`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Voice {
    pending: Option<Said>,
    last_s: Option<f64>,
    recent: Vec<(String, f64)>,
}

impl Voice {
    /// Something to say at `now_s`; kept if it is at least as important as what is waiting.
    pub fn offer(&mut self, s: Said, now_s: f64, l: &Limits) {
        if self.recent.iter().any(|(t, at)| *t == s.text && now_s - at < l.repeat_s) {
            return;
        }
        if self.pending.as_ref().is_none_or(|p| s.rank <= p.rank) {
            self.pending = Some(s);
        }
    }

    /// The line to say now, if its gap has passed and it has one.
    pub fn take(&mut self, now_s: f64, l: &Limits) -> Option<Said> {
        if self.last_s.is_some_and(|t| now_s - t < l.npc_gap_s) {
            return None;
        }
        let s = self.pending.take()?;
        self.last_s = Some(now_s);
        self.recent.retain(|(_, at)| now_s - at < l.repeat_s);
        self.recent.push((s.text.clone(), now_s));
        Some(s)
    }

    /// Forget what was waiting (the speaker gave the station up).
    pub fn clear(&mut self) {
        self.pending = None;
    }
}

/// A line as everyone sees it: who said it (slot, name, station, an NPC), its kind, its text and the game time.
#[derive(Clone, Debug, PartialEq)]
pub struct Line {
    /// The speaker's slot; none for the server's own voices and automation.
    pub slot: Option<u8>,
    /// The speaker's name.
    pub name: String,
    /// The station the speaker held, if any.
    pub station: Option<Station>,
    /// An NPC: a bot, automation, damage control or the computer.
    pub npc: bool,
    /// Its kind.
    pub kind: Kind,
    /// Its text.
    pub text: String,
    /// The drill's clock when the server took it, seconds.
    pub at_s: f64,
}

impl Line {
    /// The line as plain text, for a log or a terminal: `[mm:ss] KIND Name (NPC) (Station): text`.
    pub fn plain(&self) -> String {
        let t = self.at_s.max(0.0) as u64;
        let npc = if self.npc { " (NPC)" } else { "" };
        let station = self.station.map(|s| format!(" ({})", s.name())).unwrap_or_default();
        format!(
            "[{:02}:{:02}] {} {}{npc}{station}: {}",
            t / 60,
            t % 60,
            self.kind.id().to_uppercase(),
            self.name,
            self.text
        )
    }
}

/// The lines the server keeps (crew-chat design 1 and 4): the last `keep_lines`, and each player's pace.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ChatLog {
    lines: VecDeque<Line>,
    last_by_slot: BTreeMap<u8, f64>,
}

impl ChatLog {
    /// Keep `line`, unless its speaker (a slot) spoke less than `speaker_gap_s` ago. Returns whether it was kept.
    pub fn post(&mut self, line: Line, l: &Limits) -> bool {
        if let Some(slot) = line.slot {
            if self.last_by_slot.get(&slot).is_some_and(|t| line.at_s - t < l.speaker_gap_s) {
                return false;
            }
            self.last_by_slot.insert(slot, line.at_s);
        }
        self.lines.push_back(line);
        while self.lines.len() > l.keep_lines {
            self.lines.pop_front();
        }
        true
    }

    /// The lines kept, oldest first.
    pub fn lines(&self) -> impl Iterator<Item = &Line> {
        self.lines.iter()
    }
}

/// The drill's own voices: one per automated station, the computer and damage control, and what the computer has
/// already reported this round.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DrillVoices {
    /// Each station's automation, by `Station as usize`.
    pub stations: [Voice; Station::ALL.len()],
    /// The ship's computer.
    pub computer: Voice,
    /// Damage control.
    pub damage_control: Voice,
    /// The next hull report step (an index in `report_steps_pct`).
    pub hull_step: usize,
    /// The next shields report step.
    pub shield_step: usize,
    /// Breaches already reported.
    pub breaches: usize,
    /// The enemy's kill reported.
    pub killed: bool,
}

impl DrillVoices {
    /// A new round: nothing reported yet (the voices keep their pace).
    pub fn new_round(&mut self) {
        self.hull_step = 0;
        self.shield_step = 0;
        self.breaches = 0;
        self.killed = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat::damage::Job;

    fn data() -> DrillData {
        DrillData::shipped()
    }

    fn limits() -> Limits {
        data().chat.limits
    }

    fn text(s: Option<Said>) -> String {
        s.expect("a line").text
    }

    #[test]
    fn a_fired_tube_says_torpedo_away_with_its_number() {
        let d = data();
        let s = said_for(&d, Station::Tactical, &Command::Fire(0)).unwrap();
        assert_eq!((s.kind, s.text.as_str()), (Kind::Intent, "Torpedo away, tube 1"));
    }

    #[test]
    fn a_lock_names_the_enemy_class() {
        let d = data();
        assert_eq!(text(said_for(&d, Station::Tactical, &Command::Lock(Some(2)))), "Locking the Hound corvette");
    }

    #[test]
    fn every_captains_verb_has_an_order_and_an_ack() {
        let d = data();
        for st in Station::ALL {
            for (i, v) in d.order_verbs(st).iter().enumerate() {
                let o = said_for(&d, Station::Captain, &Command::Order(st, i as u8));
                assert_eq!(o.map(|s| s.kind), Some(Kind::Order), "{} {}", st.id(), v[0]);
                assert_eq!(said_ack(&d, st, i as u8).map(|s| s.kind), Some(Kind::Ack), "{} {}", st.id(), v[0]);
            }
        }
    }

    #[test]
    fn an_attitude_order_reads_as_heading_and_mark() {
        let d = data();
        let s = said_for(&d, Station::Helm, &Command::Orient(Some([90.0, 10.0, 0.0])));
        assert_eq!(text(s), "Coming to 090 mark 10");
    }

    #[test]
    fn the_stick_says_nothing() {
        let d = data();
        let stick = Command::Stick { speed_set_mps: 100.0, yaw: 0.5, pitch: 0.0, roll: 0.0 };
        assert_eq!(said_for(&d, Station::Helm, &stick), None, "flying is not news, many times a second");
    }

    #[test]
    fn damage_control_names_the_room_of_a_fire_in_a_sentence() {
        let d = data();
        let l = &d.tern_layout;
        let mess = l.room("mess").unwrap();
        let s = said_damage(&d, l, &DamageEvent::Dispatched { team: 0, job: Job::Fire(mess) }).unwrap();
        assert_eq!((s.kind, s.text.as_str()), (Kind::Intent, "Fire in the mess, on my way"));
        assert_eq!(text(said_damage(&d, l, &DamageEvent::FireOut { room: mess })), "Fire out in the mess");
    }

    #[test]
    fn a_repaired_system_starts_its_line_with_a_capital() {
        let d = data();
        let l = &d.tern_layout;
        let k = l.loads.iter().position(|x| x.system == "shield_generator").expect("the shield generator's load");
        assert_eq!(text(said_damage(&d, l, &DamageEvent::Repaired { load: k })), "Shield generator back to nominal");
    }

    #[test]
    fn a_voice_says_the_most_important_line_and_then_waits_its_gap() {
        let (d, l) = (data(), limits());
        let mut v = Voice::default();
        for c in [Command::Lock(Some(2)), Command::TurretMode(0, TurretMode::Auto), Command::Fire(0)] {
            v.offer(said_for(&d, Station::Tactical, &c).unwrap(), 10.0, &l);
        }
        assert_eq!(text(v.take(10.0, &l)), "Torpedo away, tube 1");
        v.offer(said_for(&d, Station::Tactical, &Command::Decoy).unwrap(), 10.5, &l);
        assert_eq!(v.take(10.0 + l.npc_gap_s * 0.5, &l), None, "inside its gap it waits");
        assert_eq!(text(v.take(10.0 + l.npc_gap_s, &l)), "Decoy out");
    }

    #[test]
    fn a_voice_does_not_repeat_itself_within_repeat_s() {
        let (d, l) = (data(), limits());
        let mut v = Voice::default();
        let fire = said_for(&d, Station::LaserPort, &Command::LaserFire(0)).unwrap();
        v.offer(fire.clone(), 0.0, &l);
        assert!(v.take(0.0, &l).is_some());
        v.offer(fire.clone(), l.npc_gap_s * 2.0, &l);
        assert_eq!(v.take(l.npc_gap_s * 2.0, &l), None, "a bank firing each charge says so once");
        v.offer(fire, l.repeat_s + 1.0, &l);
        assert!(v.take(l.repeat_s + 1.0, &l).is_some(), "and again once repeat_s has passed");
    }

    fn typed(slot: u8, at_s: f64, text: &str) -> Line {
        Line {
            slot: Some(slot),
            name: "Ens. Dara Holt".into(),
            station: Some(Station::Tactical),
            npc: false,
            kind: Kind::Say,
            text: text.into(),
            at_s,
        }
    }

    #[test]
    fn a_line_prints_with_its_time_kind_and_speaker() {
        let l = Line { at_s: 134.9, kind: Kind::Intent, text: "Torpedo away, tube 1".into(), ..typed(1, 0.0, "") };
        assert_eq!(l.plain(), "[02:14] INTENT Ens. Dara Holt (Tactical): Torpedo away, tube 1");
    }

    #[test]
    fn a_player_typing_faster_than_the_gap_is_dropped() {
        let l = limits();
        let mut log = ChatLog::default();
        assert!(log.post(typed(1, 5.0, "Ready"), &l));
        assert!(!log.post(typed(1, 5.2, "Ready again"), &l), "0.2 s after the last");
        assert!(log.post(typed(2, 5.2, "Me too"), &l), "another player is not held back");
        assert!(log.post(typed(1, 5.0 + l.speaker_gap_s, "Now"), &l));
    }

    #[test]
    fn the_log_keeps_the_last_lines_only() {
        let l = limits();
        let mut log = ChatLog::default();
        for i in 0..l.keep_lines + 5 {
            log.post(typed(1, i as f64 * 10.0, &format!("line {i}")), &l);
        }
        let kept: Vec<&str> = log.lines().map(|x| x.text.as_str()).collect();
        assert_eq!(kept.len(), l.keep_lines);
        assert_eq!(kept[0], "line 5", "the oldest go first");
    }

    #[test]
    fn typing_is_cleaned_and_an_empty_line_dropped() {
        assert_eq!(clean_text("  Fire\u{7}s out  "), Some("Fires out".into()));
        assert_eq!(clean_text(" \n\t "), None);
        assert_eq!(clean_text(&"x".repeat(200)).map(|t| t.chars().count()), Some(MAX_CHAT_CHARS));
    }

    #[test]
    fn report_steps_are_reached_once_each_and_a_big_fall_skips_to_the_lowest() {
        let steps = [75.0, 50.0, 25.0, 10.0];
        assert_eq!(step_reached(&steps, 0, 80.0), None);
        assert_eq!(step_reached(&steps, 0, 70.0), Some(0));
        assert_eq!(step_reached(&steps, 1, 70.0), None, "75 was reported already");
        assert_eq!(step_reached(&steps, 1, 20.0), Some(2), "past 50 and 25 at once: one report, at 25");
    }

    fn vocabulary(edit: impl Fn(&mut serde_json::Value)) -> Result<ChatData, DataError> {
        let mut v: serde_json::Value = serde_json::from_str(include_str!("../../../../data/crew/chat.json")).unwrap();
        edit(&mut v);
        let d = data();
        let mut c: ChatData = crate::data::parse(CHAT_FILE, &v.to_string())?;
        c.bind(CHAT_FILE, &d.stations.orders, include_str!("../../../../data/ships/tern/layout.json"), &d.enemy.name)?;
        Ok(c)
    }

    use crate::combat::automation::{self, CaptainMemory, Picture};
    use crate::combat::{Bridge, Drill, DrillEvent, Phase};

    /// A drill with one player at Helm, `s` seconds into Engage, its chat lines so far.
    fn engaged(s: f64) -> (Drill, Vec<Line>) {
        let mut d = Drill::new(DrillData::shipped(), 3);
        let a = d.join("A", false).unwrap();
        d.claim(a, Station::Helm).unwrap();
        d.set_ready(a, true).unwrap();
        // The round waits for START MISSION (start-mission 1).
        d.start_mission(a).unwrap();
        let mut said = Vec::new();
        while d.phase != Phase::Engage || d.phase_s < s {
            d.step();
            said.extend(d.drain_events().into_iter().filter_map(|e| {
                if let DrillEvent::Said(l) = e {
                    Some(l)
                } else {
                    None
                }
            }));
        }
        (d, said)
    }

    #[test]
    fn automation_at_the_captains_seat_calls_red_alert_in_chat() {
        let (_, said) = engaged(120.0);
        let l =
            said.iter().find(|l| l.text == "Red alert, all hands to stations").expect("the captain's automation spoke");
        assert_eq!(
            (l.name.as_str(), l.station, l.kind, l.slot, l.npc),
            ("Captain", Some(Station::Captain), Kind::Order, None, true)
        );
    }

    #[test]
    fn damage_control_says_when_it_sends_a_team_to_a_fire_and_when_it_is_out() {
        let (mut d, _) = engaged(0.0);
        let mess = d.data.tern_layout.room("mess").unwrap();
        d.damage[0].fires.push(crate::combat::damage::Fire::new(mess));
        let mut said = Vec::new();
        for _ in 0..(240.0 * crate::combat::TICK_HZ) as usize {
            d.step();
            said.extend(d.drain_events().into_iter().filter_map(|e| {
                if let DrillEvent::Said(l) = e {
                    Some(l)
                } else {
                    None
                }
            }));
            if said.iter().any(|l| l.text == "Fire out in the mess") {
                break;
            }
        }
        let by_dc = |t: &str| said.iter().any(|l| l.name == "Damage control" && l.text == t);
        assert!(by_dc("Fire in the mess, on my way"), "{said:?}");
        assert!(by_dc("Fire out in the mess"), "{said:?}");
    }

    #[test]
    fn a_bot_captain_orders_tactical_to_engage_at_red_alert_then_waits_its_gap() {
        let (d, _) = engaged(1.0);
        let pic = Picture::from_drill(&d);
        assert!(pic.hostile.is_some(), "the Hound is in the fight");
        let o = d.data.profile("bot").captain.orders.expect("a bot captain gives orders");
        let bridge = Bridge { red_alert: true, ..Bridge::default() };
        let mut mem = CaptainMemory::default();
        let verb = |st: Station, v: &str| d.data.order_verbs(st).iter().position(|x| x[0] == v).unwrap() as u8;
        let first = automation::captain_orders(&pic, &bridge, &d.data, &o, &mut mem, 10.0);
        assert_eq!(first, vec![Command::Order(Station::Tactical, verb(Station::Tactical, "ENGAGE"))]);
        assert_eq!(text(said_for(&d.data, Station::Captain, &first[0])), "Tactical, engage the Hound corvette");
        assert!(automation::captain_orders(&pic, &bridge, &d.data, &o, &mut mem, 10.0 + o.gap_s * 0.5).is_empty());
        let next = automation::captain_orders(&pic, &bridge, &d.data, &o, &mut mem, 10.0 + o.gap_s);
        assert_eq!(next, vec![Command::Order(Station::Helm, verb(Station::Helm, "CHASE"))], "each order once");
    }

    #[test]
    fn the_drills_automation_captain_gives_no_orders() {
        assert_eq!(data().profile("automation").captain.orders, None, "bridge-stations 3: auto-condition only");
    }

    #[test]
    fn the_shipped_vocabulary_loads() {
        vocabulary(|_| {}).unwrap();
    }

    #[test]
    fn a_misspelt_slot_is_refused_at_load() {
        let e = vocabulary(|v| v["lines"]["lock"]["text"] = "Locking the {taget}".into()).unwrap_err();
        assert_eq!(e.field, "lines.lock");
    }

    #[test]
    fn a_verb_without_its_ack_is_refused_at_load() {
        let e = vocabulary(|v| {
            v["acks"]["helm"].as_object_mut().unwrap().remove("EVADE");
        })
        .unwrap_err();
        assert_eq!(e.field, "acks.helm");
    }

    #[test]
    fn a_line_the_engine_does_not_speak_is_refused_at_load() {
        let e =
            vocabulary(|v| v["lines"]["warp"] = serde_json::json!({"kind": "intent", "text": "Warping", "rank": 1}))
                .unwrap_err();
        assert_eq!(e.field, "lines.warp");
    }

    #[test]
    fn an_unknown_kind_is_refused_at_load() {
        let e = vocabulary(|v| v["lines"]["decoy"]["kind"] = "shout".into()).unwrap_err();
        assert_eq!(e.field, "lines.decoy.kind");
    }
}
