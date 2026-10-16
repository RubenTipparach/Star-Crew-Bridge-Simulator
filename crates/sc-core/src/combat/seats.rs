//! Who may take a seat, and where a bot relieved of one goes (openspec/changes/relieve-bots design 2 and 3).
//!
//! One rule for the server's claim, the bots' choice of a free seat and the client's prompt and briefing buttons
//! (CLAUDE.md 6.1). The owner, 2026-10-15: "as a human player, I should be able to ovveride bot stations and take
//! control of whatever station a bot is in cotnrol of". It lives in the core because the server decides by it; it
//! takes plain facts so a client can ask it of a snapshot.

use crate::combat::Station;

/// A body in the way of a seat: who holds it, or who is walking to it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Occupant {
    /// The player's slot.
    pub slot: u8,
    /// A bot.
    pub bot: bool,
}

/// What a claim on a seat comes to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClaimRule {
    /// Nobody holds it or walks to it: sit.
    Sit,
    /// Bots hold it or walk to it, and a person claims it: they are sent on and the person sits.
    Relieve,
    /// A person holds it or walks to it (or a bot claims a seat anyone has): refused, by this slot.
    Taken(u8),
}

/// The rule (design 2): a person takes any seat bots hold or walk to; nobody takes one a person holds or walks to; a
/// bot takes only a seat nobody has. `holder` and `walker` are other than the claimer.
pub fn claim_rule(me_bot: bool, holder: Option<Occupant>, walker: Option<Occupant>) -> ClaimRule {
    let others = [holder, walker];
    if let Some(p) = others.iter().flatten().find(|o| !o.bot) {
        return ClaimRule::Taken(p.slot);
    }
    match others.iter().flatten().next() {
        None => ClaimRule::Sit,
        Some(b) if me_bot => ClaimRule::Taken(b.slot),
        Some(_) => ClaimRule::Relieve,
    }
}

/// Where a relieved bot goes (design 3): the station the person just `left` when it is `free` (unmanned: no operator,
/// nobody walking to it), else the nearest free one of `candidates` by `dist` from the bot, else none (its muster
/// place). Ties go to the earlier candidate.
pub fn relief_post(
    left: Option<Station>,
    candidates: &[Station],
    free: impl Fn(Station) -> bool,
    dist: impl Fn(Station) -> f32,
) -> Option<Station> {
    if let Some(l) = left.filter(|l| candidates.contains(l) && free(*l)) {
        return Some(l);
    }
    candidates.iter().copied().filter(|s| free(*s)).min_by(|a, b| dist(*a).total_cmp(&dist(*b)))
}

#[cfg(test)]
mod tests {
    use super::*;

    const BOT: Option<Occupant> = Some(Occupant { slot: 3, bot: true });
    const PERSON: Option<Occupant> = Some(Occupant { slot: 4, bot: false });

    #[test]
    fn a_free_seat_is_anyones() {
        assert_eq!(claim_rule(false, None, None), ClaimRule::Sit);
        assert_eq!(claim_rule(true, None, None), ClaimRule::Sit);
    }

    #[test]
    fn a_person_takes_a_seat_a_bot_holds_or_walks_to() {
        assert_eq!(claim_rule(false, BOT, None), ClaimRule::Relieve, "a bot in the seat");
        assert_eq!(claim_rule(false, None, BOT), ClaimRule::Relieve, "a bot on its way");
        assert_eq!(claim_rule(false, BOT, Some(Occupant { slot: 5, bot: true })), ClaimRule::Relieve, "two bots");
    }

    #[test]
    fn nobody_takes_a_seat_a_person_holds_or_walks_to() {
        assert_eq!(claim_rule(false, PERSON, None), ClaimRule::Taken(4), "a person never relieves a person");
        assert_eq!(claim_rule(false, None, PERSON), ClaimRule::Taken(4), "nor takes the seat they walk to");
        assert_eq!(claim_rule(false, BOT, PERSON), ClaimRule::Taken(4), "a person on the way to a bot's seat has it");
        assert_eq!(claim_rule(true, PERSON, None), ClaimRule::Taken(4));
    }

    #[test]
    fn a_bot_takes_only_a_seat_nobody_has() {
        assert_eq!(claim_rule(true, BOT, None), ClaimRule::Taken(3));
        assert_eq!(claim_rule(true, None, BOT), ClaimRule::Taken(3));
    }

    #[test]
    fn a_relieved_bot_swaps_into_the_seat_the_person_left_else_the_nearest_free_one() {
        let roster = [Station::Helm, Station::Tactical, Station::Science, Station::Captain];
        let dist = |s: Station| match s {
            Station::Science => 2.0,
            Station::Tactical => 5.0,
            _ => 9.0,
        };
        let all_free = |_: Station| true;
        assert_eq!(relief_post(Some(Station::Helm), &roster, all_free, dist), Some(Station::Helm), "the swap");
        let helm_taken = |s: Station| s != Station::Helm;
        assert_eq!(relief_post(Some(Station::Helm), &roster, helm_taken, dist), Some(Station::Science), "the nearest");
        assert_eq!(relief_post(None, &roster, all_free, dist), Some(Station::Science), "nobody left a seat");
        assert_eq!(relief_post(Some(Station::Helm), &roster, |_| false, dist), None, "all manned: to muster");
    }
}
