# Proposal: a person takes any station a bot holds

## Why

The owner, 2026-10-15: "as a human player, I should be able to ovveride bot stations and take control of whatever
station a bot is in cotnrol of : for example I'm helmsman, I should be able to sit in captains seat whenever".

The core already lets a player relieve a seated bot (`bridge-stations` 6, "An NPC body never holds a station against a
player"), but from the player's side in single player it did not work end to end:

- **A bot walking to the seat refused the player.** The server said "Station taken" and the prompt said "Lt. X is on
  the way to Captain", so a player who stood up at the start or mid-fight could lose a seat to a bot for good.
- **The briefing's station buttons were off for every held station,** bots' included.
- **Single player kept each NPC on the station it joined at, and the player on theirs.** After the player took the
  captain's chair, the client still showed the captain's console as the NPC's on AUTO: the player's presses went
  nowhere, the NPC's tab said it was walking to a seat it no longer had, and the next round's READY waited forever for
  it to sit there.
- **The relieved bot walked back to its muster place** and only then picked a post, so the helm stood empty while it
  walked.

The "Capt." officer in the briefing without "(bot)" is the player: the menu's rank (the officer chosen in the lobby),
not an NPC. Every NPC joins as a bot and is shown so.

## What Changes

- **One rule for who may take a seat** (design 2), in the core and used by the server, the bots and the screens: a
  person takes any seat a bot holds or is walking to, in any phase; a person never takes one another person holds or is
  walking to; a bot never takes one a person holds or is walking to.
- **The relieved bot swaps** (design 3): it stands and walks straight to the station the person just left, when that is
  still unmanned; else to the nearest unmanned station; else to its muster place.
- **A bot redirected mid-walk walks on from where it is** (design 4), along the walk's paths.
- **Single player follows the seats** (design 5): each NPC's tab and mode follow the station it holds or walks to, and
  the player's own chip follows the seat they sit in.
- **The screens say the rule** (design 6): "E  Take Captain from Lt. X" while X sits there or walks to it; the
  briefing's buttons are on for every station a bot holds.

## Impact

- `sc-core::combat`: `seats.rs` (the rule and the relief post), `claim_on_foot`, `Player::left`, `CrewBody::go`
  walking on from where it is.
- `sc-net::bot`: its free seat is the rule's.
- `sc-client`: the seat prompt, the briefing's buttons, `solo.rs` following the seats.
- Pi 5 budget: nothing measurable (a few comparisons a claim).
