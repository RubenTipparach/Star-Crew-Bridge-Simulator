//! Every repair game, one file a game (repair-minigames design 8), each ported from `docs/mockups/repairs/<id>.js`.
//!
//! A game not yet ported is the kit's [`Unported`] placeholder, so the menu lists every system and says which ones
//! are still to come; its module says `PORTED = false`.

use crate::kit::Game;
pub use crate::kit::Unported;

pub mod biobed;
pub mod breakers;
pub mod chiller;
pub mod conduits;
pub mod coolant;
pub mod doors;
pub mod fighter;
pub mod galley;
pub mod gravity;
pub mod hull;
pub mod impulse;
pub mod medic;
pub mod pipes;
pub mod pump;
pub mod pylons;
pub mod reactor;
pub mod scrubbers;
pub mod sensors;
pub mod shields;
pub mod shower;
pub mod shuttle;
pub mod toilet;
pub mod tubes;
pub mod turret;

/// Every game's id, in the menu's order within its group (its file names the group).
pub const IDS: [&str; 24] = [
    "biobed",
    "breakers",
    "chiller",
    "conduits",
    "coolant",
    "doors",
    "fighter",
    "galley",
    "gravity",
    "hull",
    "impulse",
    "medic",
    "pipes",
    "pump",
    "pylons",
    "reactor",
    "scrubbers",
    "sensors",
    "shields",
    "shower",
    "shuttle",
    "toilet",
    "tubes",
    "turret",
];

/// A fresh game `id`.
pub fn make(id: &str) -> Option<Box<dyn Game>> {
    Some(match id {
        "biobed" => biobed::new(),
        "breakers" => breakers::new(),
        "chiller" => chiller::new(),
        "conduits" => conduits::new(),
        "coolant" => coolant::new(),
        "doors" => doors::new(),
        "fighter" => fighter::new(),
        "galley" => galley::new(),
        "gravity" => gravity::new(),
        "hull" => hull::new(),
        "impulse" => impulse::new(),
        "medic" => medic::new(),
        "pipes" => pipes::new(),
        "pump" => pump::new(),
        "pylons" => pylons::new(),
        "reactor" => reactor::new(),
        "scrubbers" => scrubbers::new(),
        "sensors" => sensors::new(),
        "shields" => shields::new(),
        "shower" => shower::new(),
        "shuttle" => shuttle::new(),
        "toilet" => toilet::new(),
        "tubes" => tubes::new(),
        "turret" => turret::new(),
        _ => return None,
    })
}

/// Whether game `id` is in the engine yet.
pub fn ported(id: &str) -> bool {
    match id {
        "biobed" => biobed::PORTED,
        "breakers" => breakers::PORTED,
        "chiller" => chiller::PORTED,
        "conduits" => conduits::PORTED,
        "coolant" => coolant::PORTED,
        "doors" => doors::PORTED,
        "fighter" => fighter::PORTED,
        "galley" => galley::PORTED,
        "gravity" => gravity::PORTED,
        "hull" => hull::PORTED,
        "impulse" => impulse::PORTED,
        "medic" => medic::PORTED,
        "pipes" => pipes::PORTED,
        "pump" => pump::PORTED,
        "pylons" => pylons::PORTED,
        "reactor" => reactor::PORTED,
        "scrubbers" => scrubbers::PORTED,
        "sensors" => sensors::PORTED,
        "shields" => shields::PORTED,
        "shower" => shower::PORTED,
        "shuttle" => shuttle::PORTED,
        "toilet" => toilet::PORTED,
        "tubes" => tubes::PORTED,
        "turret" => turret::PORTED,
        _ => false,
    }
}

/// Load every shipped repair file from the repository (the tests' and the tools' loader; the client has its own).
pub fn load_shipped(root: &std::path::Path) -> Result<sc_core::repair::data::RepairData, String> {
    let ids: Vec<String> = IDS.iter().map(|s| (*s).to_owned()).collect();
    sc_core::repair::data::RepairData::load(&ids, |rel| {
        std::fs::read_to_string(root.join(rel)).map_err(|e| format!("{rel}: cannot be read: {e}"))
    })
}

#[cfg(test)]
pub mod tests {
    //! The checks every game shares: its file holds its knobs, a steady hand repairs it from every state with no
    //! fumble, a deliberate mistake fumbles, and its `min_round_s` never refuses the hand's rounds.

    use super::*;
    use crate::kit::{play, CoverPhase, Input, Options, Runner};
    use sc_core::repair::data::RepairData;
    use sc_core::repair::{State, Who};

    /// The shipped data.
    pub fn data() -> RepairData {
        load_shipped(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
            .expect("the shipped repair data loads")
    }

    /// Open `id` from `state` for an officer.
    pub fn open(id: &str, state: State) -> Runner {
        let opts = Options { who: Who::Officer, state, combat: false, seed: 1 };
        Runner::new(make(id).expect("a game"), &data(), opts, true).expect("opens")
    }

    /// The hand repairs `id` from every state, with no fumble, inside ten minutes.
    pub fn plays_to_end(id: &str) {
        for state in State::ALL {
            let mut r = open(id, state);
            let p = play(&mut r, 600.0, |_, _| {});
            assert!(p.done, "{id} from {state:?}: the hand finishes the job ({p:?})");
            assert_eq!(p.fumbles, 0, "{id} from {state:?}: a steady hand never fumbles ({p:?})");
            assert_eq!(r.job.value, r.job.target, "{id}: the job ends on its target");
        }
    }

    /// Play `id` damaged with its hand until its game is live (the cover off), then feed `mistake` until a fumble.
    pub fn fumble_check(id: &str, mut mistake: impl FnMut(&Runner) -> Input) {
        let mut r = open(id, State::Damaged);
        let dt = 1.0 / 60.0;
        let mut n = 0;
        while r.cover.as_ref().is_some_and(|c| c.phase != CoverPhase::Work) && n < 6000 {
            let i = r.hand();
            r.update(dt, &i);
            n += 1;
        }
        let before = r.job.value;
        for _ in 0..6000 {
            let i = mistake(&r);
            r.update(dt, &i);
            if r.job.fumbles_total > 0 {
                assert!(r.job.value < before, "{id}: the fumble cost the job ({} from {before})", r.job.value);
                return;
            }
        }
        panic!("{id}: the mistake never fumbled");
    }

    #[test]
    fn every_game_file_holds_exactly_its_games_knobs() {
        let d = data();
        for id in IDS {
            let g = make(id).expect("every id makes a game");
            assert_eq!(g.id(), id);
            let f = d.games.get(id).expect("every game has a file");
            f.needs(&sc_core::repair::data::game_file(id), g.knobs()).expect("its knobs");
        }
    }

    /// Each ported game's hand's round times by level, over every state and the seeds in `SEEDS`: the fastest
    /// round seen at each level.
    fn fastest_rounds(id: &str) -> Vec<f32> {
        let mut best = vec![f32::INFINITY; 6];
        for state in State::ALL {
            for seed in SEEDS {
                let opts = Options { who: Who::Officer, state, combat: false, seed };
                let mut r = Runner::new(make(id).expect("a game"), &data(), opts, true).expect("opens");
                let p = play(&mut r, 900.0, |_, _| {});
                assert!(p.done, "{id} {state:?} seed {seed}: {p:?}");
                for (i, s) in p.round_s.iter().enumerate() {
                    let l = (r.job.level_of_round(i as u32) as usize - 1).min(5);
                    best[l] = best[l].min(*s);
                }
            }
        }
        best
    }

    /// The seeds the floor is checked over: the rounds' puzzles differ by seed, and an easy one is faster.
    const SEEDS: [u64; 4] = [1, 2, 3, 4];

    #[test]
    fn a_ported_games_floor_never_refuses_a_steady_hand() {
        for id in IDS.iter().filter(|id| ported(id)) {
            let f = data().games.get(*id).cloned().expect("a file");
            for (i, s) in fastest_rounds(id).iter().enumerate() {
                let floor = f.min_round(i as u32 + 1) as f32;
                assert!(floor > 0.0, "{id}: min_round_s is filled in (design 8)");
                assert!(
                    s.is_infinite() || *s >= floor,
                    "{id} level {}: the hand's {s:.2} s is under the floor {floor}",
                    i + 1
                );
            }
        }
    }

    /// `cargo test -p sc-repairs round_times -- --ignored --nocapture`: each ported game's fastest hand round by
    /// level over every state and seed, and the provisional floor (40% of it, design 8).
    #[test]
    #[ignore]
    fn round_times() {
        for id in IDS.iter().filter(|id| ported(id)) {
            let best = fastest_rounds(id);
            let floor: Vec<String> = best
                .iter()
                .map(|s| if s.is_finite() { format!("{:.1}", (s * 0.4 * 10.0).floor() / 10.0) } else { "-".into() })
                .collect();
            println!("{id}: fastest rounds {best:.2?} s; min_round_s [{}]", floor.join(", "));
        }
    }

    #[test]
    fn a_rating_repairs_with_no_game_at_its_rate() {
        let opts = Options { who: Who::Rating, state: State::Damaged, combat: false, seed: 1 };
        let mut r = Runner::new(make("impulse").expect("a game"), &data(), opts, true).expect("opens");
        let preview = r.job.time_left(data().damage.kit_pct_per_s.rating) as f32;
        let p = play(&mut r, 600.0, |_, _| {});
        assert!(p.done);
        assert!((p.t_s - preview).abs() < 0.2, "the board's time is the time it takes: {} against {preview}", p.t_s);
    }
}
