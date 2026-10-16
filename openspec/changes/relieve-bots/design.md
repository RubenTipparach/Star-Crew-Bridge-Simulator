# Design: a person takes any station a bot holds

## Context

| Decided elsewhere | Where |
| --- | --- |
| Claim, release, relieve: "A seat held by an NPC body is always claimable" | `bridge-stations` 6 |
| A claim on foot is a walk; a player sits only within reach of the seat | `coop-drill` 9, `bridge-stations` 6 |
| Bots' posts and reassignment | `data/stations.json` `bot_posts`, `sc-net::bot` |
| Single player: an NPC at every other station, AUTO and MANUAL | `single-player` 1-4 |
| One implementation of every rule | CLAUDE.md 6.1 |

## 1. What stopped a player (2026-10-15)

| Candidate | Finding |
| --- | --- |
| A bot walking to the seat | Refused: `claim_on_foot` returned `StationTaken` for any body walking there, and the prompt said "X is on the way" |
| The named captain | Not an NPC: "Capt. ..." is the player's own title from the lobby's rank; NPCs join as bots ("Lt. ...", shown "(NPC)" over their heads and "(bot)" in the briefing) |
| No prompt on a bot-held seat | There was one ("E  Take Captain from Lt. X") while the bot sat; none while it walked |
| The briefing's station buttons | Off for every station anyone held, bots included |
| A phase rule | None in the core; but in single player the console of a taken station stayed the NPC's (AUTO), so the player's presses were dropped, and READY waited for the NPC to sit at the station it lost |

## 2. The rule

`sc-core::combat::seats::claim_rule(me_bot, holder, walker) -> ClaimRule`, where `holder` and `walker` are the
occupant of the seat and the body walking to it (each a slot and whether it is a bot), other than the claimer:

| Claimer | Seat held or walked to by | Outcome |
| --- | --- | --- |
| anyone | nobody | `Sit` |
| a person | bots only | `Relieve`: each bot is sent on (section 3) |
| a person | a person | `Taken` |
| a bot | anyone | `Taken` |

The server's claim, the bots' choice of a free seat and the client's prompt and briefing buttons call this one
function.

## 3. Where a relieved bot goes

`seats::relief_post(left, candidates, free, dist)`:

1. the station the person just left (`Player::left`: set when they stand up from a seat or walk off it to another),
   when it is unmanned: no operator and nobody walking to it;
2. else the nearest unmanned station of the mission's roster that the bridge has a seat for, by the straight distance
   from the bot's body to the seat;
3. else its muster place.

The body is sent straight there; when it sits, it holds that station (the core's sit rule). The relief event is logged
as before.

## 4. Walking on from where it is

A body sent somewhere new while walking (a bot redirected) walks from where it stands, not from where it set off: back
along the path it has walked and on from its start, or on to the seat it was walking to and from there, whichever is
shorter. Both follow the walk grid's paths, so nobody walks through a wall.

## 5. Single player follows the seats

`SoloHost` keeps each NPC's station as the one its session holds, or the seat its body walks to, so its tab, AUTO and
MANUAL and the console shown for it follow it. The player's own seat (`SoloHost::seat`, the own chip and Tab's first
console) is the station the player holds. READY waits for every NPC to sit wherever it is going.

## 6. On screen

- The seat prompt: "E  Take <station> from <bot>" while a bot sits there or walks to it; "<person> holds <station>"
  and "<person> is on the way to <station>" for people.
- The briefing's station buttons: on for a station that is free or a bot's, off for a person's.

## 7. Checks

- `seats.rs`: every row of the table in section 2; the relief post's three cases.
- `combat`: a person at Helm takes the captain's seat from a bot mid-Engage, and the bot walks to Helm and holds it; a
  person takes a seat a bot is walking to; a person never takes another person's; a bot never takes a person's.
- `bodies.rs`: a body redirected mid-walk never jumps.
- `sc-client` `solo.rs`: after a swap, the NPC's tab is its new station and the player's own chip is the captain's;
  every NPC seated again.
- `sc-client` `drill.rs`: the prompt offers a seat a bot walks to and not one a person walks to.
- The walking playtest shows the player take a bot's station.
