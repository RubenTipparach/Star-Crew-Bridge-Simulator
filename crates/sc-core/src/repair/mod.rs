//! Repair jobs (openspec/changes/repair-minigames design 1, 1a and 8; repairs-on-deck 6 and 7): a job's integrity
//! cut into rounds, each the system's mini-game played once at the next level, landing its share at once; a fumble's
//! cost; three fumbles restarting a round; a rating repairing at its rate with no game; and the server's checks of a
//! player's intents.
//!
//! It lives in the core because the menu, the ship and the server run one rule (CLAUDE.md 6.1 and 6.3): the client
//! plays a round, and this decides what it is worth. Nothing here draws or reads input; `sc-repairs` holds the games.

pub mod data;

use crate::rng::Rng;
use data::{DamageRepair, GameFile, RulesFile, StateRule};

/// Who is repairing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Who {
    /// A player: plays the game.
    Officer,
    /// A rating, a bot or a team: no game, a rate.
    Rating,
}

/// The state a job starts from (design 2's table).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    /// 25-75%.
    Damaged,
    /// Under 25%: the first round opens with a spare part.
    Disabled,
    /// 0%: rebuilt from level 1, the first round opening with the parts.
    Destroyed,
}

impl State {
    /// Every state, in order.
    pub const ALL: [State; 3] = [State::Damaged, State::Disabled, State::Destroyed];

    /// Its rule.
    pub fn rule(self, r: &RulesFile) -> &StateRule {
        match self {
            State::Damaged => &r.states.damaged,
            State::Disabled => &r.states.disabled,
            State::Destroyed => &r.states.destroyed,
        }
    }

    /// Its name, as the menu says it.
    pub fn label(self) -> &'static str {
        match self {
            State::Damaged => "Damaged",
            State::Disabled => "Disabled",
            State::Destroyed => "Destroyed",
        }
    }

    /// Spare parts the job needs, from `damage.json`.
    pub fn parts(self, d: &DamageRepair) -> u32 {
        match self {
            State::Damaged => 0,
            State::Disabled => d.parts.disabled_system,
            State::Destroyed => d.parts.destroyed_system,
        }
    }
}

/// What a fumble did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fumbled {
    /// The cost is taken; play on.
    Cost,
    /// The third this round: the round starts again.
    Restart,
}

/// One repair job's progress: the value the bar shows and the rounds landed.
#[derive(Clone, Debug, PartialEq)]
pub struct Job {
    /// Where it started.
    pub start: f64,
    /// Where it ends (100%, or the medic's 75 HP).
    pub target: f64,
    /// Rounds in all.
    pub rounds: u32,
    /// Rounds landed.
    pub landed: u32,
    /// The value now.
    pub value: f64,
    /// The first round still opens with the part.
    pub part_pending: bool,
    /// Fumbles in the round being played.
    pub fumbles: u32,
    /// Fumbles over the job.
    pub fumbles_total: u32,
    /// A fumble's cost, in the job's unit.
    pub fumble_cost: f64,
    fumbles_per_round: u32,
    max_level: u32,
}

impl Job {
    /// A system's job from `state` (percent integrity).
    pub fn new(rules: &RulesFile, damage: &DamageRepair, state: State) -> Self {
        let s = state.rule(rules);
        Self::custom(s.start_pct, 100.0, s.rounds, damage.fumble_share_pct, s.part, rules)
    }

    /// A job a game keeps for itself (the medic's hit points), or any explicit one.
    pub fn custom(start: f64, target: f64, rounds: u32, fumble_cost: f64, part: bool, rules: &RulesFile) -> Self {
        Self {
            start,
            target,
            rounds: rounds.max(1),
            landed: 0,
            value: start,
            part_pending: part,
            fumbles: 0,
            fumbles_total: 0,
            fumble_cost,
            fumbles_per_round: rules.fumbles_per_round,
            max_level: rules.max_level,
        }
    }

    /// The job for `game` from `state`: its own job if it keeps one, else the system's.
    pub fn for_game(game: &GameFile, rules: &RulesFile, damage: &DamageRepair, state: State) -> Self {
        match &game.job {
            Some(j) => Self::custom(j.start, j.target, j.rounds, j.fumble, false, rules),
            None => Self::new(rules, damage, state),
        }
    }

    /// One round's share of the span.
    pub fn share(&self) -> f64 {
        (self.target - self.start) / f64::from(self.rounds)
    }

    /// Where the round being played lands: never past the target.
    pub fn cap(&self) -> f64 {
        (self.start + self.share() * f64::from(self.landed + 1)).min(self.target)
    }

    /// The level of the round being played (design 1a): round n at level min(n, top).
    pub fn level(&self) -> u32 {
        level_of(self.landed, self.max_level)
    }

    /// The level round `round` (0 up) is played at.
    pub fn level_of_round(&self, round: u32) -> u32 {
        level_of(round, self.max_level)
    }

    /// Every round landed.
    pub fn done(&self) -> bool {
        self.landed >= self.rounds
    }

    /// Rounds still to play, what the board shows a player (design 1).
    pub fn rounds_left(&self) -> u32 {
        self.rounds.saturating_sub(self.landed)
    }

    /// Seconds left at `rate_per_s` with no game, what the board shows for a rating (design 1, CLAUDE.md 6.1).
    pub fn time_left(&self, rate_per_s: f64) -> f64 {
        ((self.target - self.value) / rate_per_s.max(1e-9)).max(0.0)
    }

    /// The part is fitted: the first round's game begins.
    pub fn fit_part(&mut self) {
        self.part_pending = false;
    }

    /// The round being played is played: its share lands at once (design 1). The last lands on the target.
    pub fn land(&mut self) {
        if self.done() {
            return;
        }
        self.value = self.cap();
        self.landed += 1;
        self.fumbles = 0;
        if self.done() {
            self.value = self.target;
        }
    }

    /// A mistake: the cost at once, never below one share under the start nor below zero; the third in a round
    /// restarts it (design 1).
    pub fn fumble(&mut self) -> Fumbled {
        self.value = (self.value - self.fumble_cost).max(self.start - self.share()).max(0.0);
        self.fumbles += 1;
        self.fumbles_total += 1;
        if self.fumbles >= self.fumbles_per_round {
            self.fumbles = 0;
            Fumbled::Restart
        } else {
            Fumbled::Cost
        }
    }

    /// A rating at work for `dt` seconds at `rate_per_s`: the bar fills to the round's cap and lands it there.
    /// Returns whether a round landed.
    pub fn work(&mut self, dt: f64, rate_per_s: f64) -> bool {
        if self.done() {
            return false;
        }
        self.part_pending = false;
        self.value = (self.value + rate_per_s * dt).min(self.cap());
        if self.value >= self.cap() - 1e-9 {
            self.land();
            return true;
        }
        false
    }
}

/// Round `landed` (0 up) is played at this level.
pub fn level_of(landed: u32, max_level: u32) -> u32 {
    (landed + 1).min(max_level.max(1))
}

/// The seeded stream of a round's puzzle (repairs-on-deck 6, CLAUDE.md 6.4): the same job, round and phase draw
/// the same puzzle every time, so a restarted round is the same and a replay agrees.
pub fn round_rng(session_seed: u64, job: &str, round: u32, phase: u32, part: bool) -> Rng {
    let id = job.bytes().fold(0u64, |h, b| h.rotate_left(5) ^ u64::from(b));
    let purpose = format!("repair:{round}:{phase}:{}", if part { "part" } else { "round" });
    Rng::for_purpose(session_seed, id, &purpose)
}

/// A player, by the server's stable id.
pub type PlayerId = u32;

/// Why the server refused an intent (repairs-on-deck 6).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// No such job, or it is repaired.
    NoJob,
    /// Not holding a kit.
    NoKit,
    /// The job needs a part the kit's pouch lacks.
    NeedsPart,
    /// Every worker's place at the job is taken.
    Full,
    /// Not docked at that job.
    NotDocked,
    /// That is not the round this player was given.
    WrongRound,
    /// Faster than any hand: the round is not landed and is played again.
    TooFast,
}

/// A worker at a job: who, and the round they were given.
#[derive(Clone, Debug, PartialEq)]
struct Worker {
    player: PlayerId,
    round: u32,
}

/// A job the server holds, with its workers.
#[derive(Clone, Debug, PartialEq)]
pub struct JobSlot {
    /// Stable id.
    pub id: String,
    /// The game's data.
    pub game: GameFile,
    /// Places at the job.
    pub workers_max: u32,
    /// The job.
    pub job: Job,
    /// The state it was repaired from.
    pub state: State,
    workers: Vec<Worker>,
    /// Rounds landed, by index (two at a job land them out of order).
    played: Vec<u32>,
}

/// What a docked player is told to play.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Order {
    /// The round (0 up).
    pub round: u32,
    /// Its level.
    pub level: u32,
    /// This round opens with the part.
    pub part: bool,
}

/// The ship's repair jobs and who is at them: the server's side of repairs-on-deck 6 and 7.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Repairs {
    /// The jobs open now.
    pub jobs: Vec<JobSlot>,
}

impl Repairs {
    /// Open a job on `id` (a system was hit).
    pub fn open(&mut self, id: &str, game: GameFile, workers_max: u32, job: Job, state: State) {
        self.jobs.retain(|j| j.id != id);
        self.jobs.push(JobSlot {
            id: id.to_owned(),
            game,
            workers_max,
            job,
            state,
            workers: Vec::new(),
            played: Vec::new(),
        });
    }

    /// The job `id`.
    pub fn job(&self, id: &str) -> Option<&JobSlot> {
        self.jobs.iter().find(|j| j.id == id)
    }

    fn slot(&mut self, id: &str) -> Result<&mut JobSlot, Refusal> {
        self.jobs.iter_mut().find(|j| j.id == id).ok_or(Refusal::NoJob)
    }

    /// `Dock { point }`: a player with a kit (and `parts` in its pouch) docks at a job and is given the next round
    /// no one else is playing. The reach and posture checks belong to the deck (`ship-interactables`) and come first.
    pub fn dock(
        &mut self,
        player: PlayerId,
        id: &str,
        has_kit: bool,
        parts: u32,
        damage: &DamageRepair,
    ) -> Result<Order, Refusal> {
        let s = self.slot(id)?;
        if s.job.done() {
            return Err(Refusal::NoJob);
        }
        if !has_kit {
            return Err(Refusal::NoKit);
        }
        if let Some(w) = s.workers.iter().find(|w| w.player == player) {
            return Ok(order_for(&s.job, w.round));
        }
        if s.workers.len() as u32 >= s.workers_max {
            return Err(Refusal::Full);
        }
        let round = next_round(&s.job, &s.workers, &s.played).ok_or(Refusal::Full)?;
        let part = round == 0 && s.job.part_pending;
        if part && parts < s.state.parts(damage) {
            return Err(Refusal::NeedsPart);
        }
        s.workers.push(Worker { player, round });
        Ok(order_for(&s.job, round))
    }

    /// `RoundDone { job, round, fumbles, play_s }`: lands the round if this player was given it and played it no
    /// faster than the level's floor; then gives the player the next round, if any is left.
    pub fn round_done(
        &mut self,
        player: PlayerId,
        id: &str,
        round: u32,
        play_s: f64,
    ) -> Result<Option<Order>, Refusal> {
        let s = self.slot(id)?;
        let w = s.workers.iter().position(|w| w.player == player).ok_or(Refusal::NotDocked)?;
        if s.workers[w].round != round {
            return Err(Refusal::WrongRound);
        }
        let level = order_for(&s.job, round).level;
        if !play_s.is_finite() || play_s < s.game.min_round(level) {
            return Err(Refusal::TooFast);
        }
        if round == 0 {
            s.job.fit_part();
        }
        s.job.land();
        s.played.push(round);
        s.workers.remove(w);
        match next_round(&s.job, &s.workers, &s.played) {
            Some(next) if !s.job.done() => {
                s.workers.push(Worker { player, round: next });
                Ok(Some(order_for(&s.job, next)))
            }
            _ => Ok(None),
        }
    }

    /// `Fumble { job, round }`: the cost and whether the round restarts; the hazard is the deck's to apply.
    pub fn fumble(&mut self, player: PlayerId, id: &str, round: u32) -> Result<Fumbled, Refusal> {
        let s = self.slot(id)?;
        let w = s.workers.iter().find(|w| w.player == player).ok_or(Refusal::NotDocked)?;
        if w.round != round {
            return Err(Refusal::WrongRound);
        }
        Ok(s.job.fumble())
    }

    /// `Undock`: the player stands up; the round they held goes back to the list.
    pub fn undock(&mut self, player: PlayerId) {
        for s in &mut self.jobs {
            s.workers.retain(|w| w.player != player);
        }
    }

    /// Players docked at `id`.
    pub fn docked(&self, id: &str) -> Vec<PlayerId> {
        self.job(id).map(|s| s.workers.iter().map(|w| w.player).collect()).unwrap_or_default()
    }
}

fn order_for(job: &Job, round: u32) -> Order {
    Order { round, level: level_of(round, job.max_level), part: round == 0 && job.part_pending }
}

/// The lowest round not landed and not being played (two at a job each take the next one left, design 7).
fn next_round(job: &Job, workers: &[Worker], played: &[u32]) -> Option<u32> {
    (0..job.rounds).find(|r| !played.contains(r) && !workers.iter().any(|w| w.round == *r))
}

#[cfg(test)]
mod tests {
    use super::data::*;
    use super::*;
    use crate::data::parse;

    fn rules() -> RulesFile {
        parse("data/repairs/rules.json", include_str!("../../../../data/repairs/rules.json")).expect("shipped rules")
    }

    fn damage() -> DamageRepair {
        parse::<DamageFile>("data/ships/tern/damage.json", include_str!("../../../../data/ships/tern/damage.json"))
            .expect("shipped damage")
            .repair
    }

    /// A shipped game, with a floor of 4 s a round whatever its file says (the floor is provisional, design 8).
    fn game() -> GameFile {
        let mut g: GameFile = parse("data/repairs/impulse.json", include_str!("../../../../data/repairs/impulse.json"))
            .expect("shipped game");
        g.min_round_s = vec![4.0; 6];
        g
    }

    #[test]
    fn a_damaged_job_is_three_rounds_each_landing_its_share_at_once() {
        let mut j = Job::new(&rules(), &damage(), State::Damaged);
        assert_eq!((j.start, j.rounds, j.level()), (25.0, 3, 1));
        j.land();
        assert!((j.value - 50.0).abs() < 1e-9, "the first round lands 25% at once, not at a rate: {}", j.value);
        assert_eq!(j.level(), 2, "the next round is the same game, harder");
        j.land();
        j.land();
        assert!(j.done() && j.value == 100.0, "the last round lands exactly on the target");
    }

    #[test]
    fn rounds_by_state_and_levels_follow_the_table() {
        let (r, d) = (rules(), damage());
        let dis = Job::new(&r, &d, State::Disabled);
        assert_eq!((dis.rounds, dis.part_pending), (4, true), "a disabled job opens with its spare part");
        let mut des = Job::new(&r, &d, State::Destroyed);
        assert_eq!(des.rounds, 6);
        let mut levels = Vec::new();
        while !des.done() {
            levels.push(des.level());
            des.land();
        }
        assert_eq!(levels, [1, 2, 3, 4, 5, 6]);
        assert_eq!(level_of(9, 6), 6, "the level stops at the top");
    }

    #[test]
    fn a_fumble_costs_five_percent_and_the_third_restarts_the_round() {
        let mut j = Job::new(&rules(), &damage(), State::Damaged);
        j.land();
        assert_eq!(j.fumble(), Fumbled::Cost);
        assert!((j.value - 45.0).abs() < 1e-9, "5% of the job at once: {}", j.value);
        assert_eq!(j.fumble(), Fumbled::Cost);
        assert_eq!(j.fumble(), Fumbled::Restart, "three in one round restart it");
        assert_eq!(j.fumbles, 0);
        for _ in 0..20 {
            j.fumble();
        }
        assert!(j.value >= j.start - j.share() - 1e-9, "never more than a share below the start: {}", j.value);
    }

    #[test]
    fn a_rating_fills_at_its_rate_and_the_board_previews_its_time_exactly() {
        let d = damage();
        let mut j = Job::new(&rules(), &d, State::Damaged);
        let rate = d.kit_pct_per_s.rating;
        let preview = j.time_left(rate);
        let mut t = 0.0;
        while !j.done() {
            j.work(0.01, rate);
            t += 0.01;
        }
        assert!((t - preview).abs() < 0.05, "the preview is the time it takes (CLAUDE.md 6.1): {t} against {preview}");
    }

    #[test]
    fn a_round_faster_than_its_floor_is_refused_and_not_landed() {
        let (r, d) = (rules(), damage());
        let mut s = Repairs::default();
        s.open("impulse_1", game(), 1, Job::new(&r, &d, State::Damaged), State::Damaged);
        let o = s.dock(7, "impulse_1", true, 0, &d).expect("docks");
        assert_eq!(o, Order { round: 0, level: 1, part: false });
        assert_eq!(s.round_done(7, "impulse_1", 0, 0.2), Err(Refusal::TooFast));
        assert_eq!(s.job("impulse_1").map(|j| j.job.landed), Some(0), "a refused round is not landed");
        let next = s.round_done(7, "impulse_1", 0, 30.0).expect("a steady hand's round lands");
        assert_eq!(next, Some(Order { round: 1, level: 2, part: false }));
        assert!((s.job("impulse_1").map(|j| j.job.value).unwrap_or(0.0) - 50.0).abs() < 1e-9);
    }

    #[test]
    fn docking_needs_a_kit_a_part_and_a_free_place() {
        let (r, d) = (rules(), damage());
        let mut s = Repairs::default();
        s.open("pump_a", game(), 1, Job::new(&r, &d, State::Disabled), State::Disabled);
        assert_eq!(s.dock(1, "pump_a", false, 0, &d), Err(Refusal::NoKit));
        assert_eq!(s.dock(1, "pump_a", true, 0, &d), Err(Refusal::NeedsPart));
        assert_eq!(s.dock(1, "pump_a", true, 1, &d).map(|o| o.part), Ok(true));
        assert_eq!(s.dock(2, "pump_a", true, 1, &d), Err(Refusal::Full));
        assert_eq!(s.dock(1, "nothing", true, 1, &d), Err(Refusal::NoJob));
    }

    #[test]
    fn two_at_one_job_each_play_the_next_round_left() {
        let (r, d) = (rules(), damage());
        let mut s = Repairs::default();
        s.open("gravity", game(), 2, Job::new(&r, &d, State::Damaged), State::Damaged);
        let a = s.dock(1, "gravity", true, 0, &d).expect("first");
        let b = s.dock(2, "gravity", true, 0, &d).expect("second");
        assert_eq!((a.round, b.round), (0, 1), "rounds 1 and 2 at once");
        assert_eq!(s.round_done(1, "gravity", 1, 30.0), Err(Refusal::WrongRound));
        let after_b = s.round_done(2, "gravity", 1, 30.0).expect("lands");
        assert_eq!(after_b.map(|o| o.round), Some(2), "whoever finishes first plays round 3");
        assert_eq!(s.round_done(1, "gravity", 0, 30.0).expect("lands"), None, "nothing left for the other");
        assert_eq!(s.round_done(2, "gravity", 2, 30.0).expect("lands"), None);
        assert!(s.job("gravity").is_some_and(|j| j.job.done()));
    }

    #[test]
    fn undocking_hands_the_round_back() {
        let (r, d) = (rules(), damage());
        let mut s = Repairs::default();
        s.open("doors", game(), 1, Job::new(&r, &d, State::Damaged), State::Damaged);
        s.dock(1, "doors", true, 0, &d).expect("docks");
        s.undock(1);
        assert!(s.docked("doors").is_empty());
        assert_eq!(s.dock(2, "doors", true, 0, &d).map(|o| o.round), Ok(0), "the round is the next player's");
    }

    #[test]
    fn a_round_draws_the_same_puzzle_every_time() {
        let a = round_rng(1, "pump_a", 2, 0, false).next_u64();
        assert_eq!(a, round_rng(1, "pump_a", 2, 0, false).next_u64());
        assert_ne!(a, round_rng(1, "pump_a", 3, 0, false).next_u64());
        assert_ne!(a, round_rng(1, "pump_b", 2, 0, false).next_u64());
    }

    #[test]
    fn a_knob_steps_by_level_inside_its_floor() {
        let k = Knob { base: Some(30.0), per_level: Some(-3.0), min: Some(18.0), max: None, levels: None };
        assert_eq!((k.at(1), k.at(3), k.at(6)), (30.0, 24.0, 18.0));
        let l = Knob { base: None, per_level: None, min: None, max: None, levels: Some(vec![6.0, 9.0, 12.0]) };
        assert_eq!((l.at(1), l.at(3), l.at(5)), (6.0, 12.0, 12.0), "the last value repeats");
    }

    #[test]
    fn a_game_file_must_hold_exactly_the_knobs_its_game_uses() {
        let g = game();
        let mut names: Vec<&str> = g.knobs.keys().map(String::as_str).collect();
        assert!(g.needs("impulse.json", &names).is_ok());
        names.push("not_a_knob");
        assert!(g.needs("impulse.json", &names).is_err(), "a knob the file lacks");
        names.truncate(names.len() - 2);
        assert!(g.needs("impulse.json", &names).is_err(), "a knob the game does not know");
    }
}
