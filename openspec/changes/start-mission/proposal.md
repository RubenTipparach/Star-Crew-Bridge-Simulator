# Proposal: START MISSION, so the round need not wait for every seat

## Why

The owner answered survey 2's Q3 (single player's crew tabs) on 2026-10-16: "approve, but need a start mission button
when everyone is ready, or captain can start even when everyone is not ready. it doesnt matter." Z1 (names and the Esc
menu) points back to it: "looks good, see previous comments on lobby Q3".

Today a round waits in Muster until every person aboard has sat down, sitting being their READY (`browser-page-fit`
design 6, finding 8), and every seated player is ready (`coop-drill` 1). A crew that is slow to its seats, or a bot
still walking the decks, holds everyone else at the briefing. Nothing on a console lets the captain say "go".

The owner, 2026-10-17, after playing the build: "I need a button to actually start the mission. Not just have the
mission autostart". The automatic start this change first kept (design 1, as written on 2026-10-16) is what they
asked to remove: the round began the moment the player sat down, so START MISSION was rarely seen and never needed.

## What Changes

- **The round never starts by itself** (design 1, revised 2026-10-17). Muster ends only when START MISSION is
  pressed. READY stays as a signal the captain reads on the chip (`ready/aboard`), never a trigger.
- **START MISSION** (design 2): the person in the captain's seat may start the countdown at any moment of Muster, ready
  or not. When no person holds the captain's seat (single player, where an NPC or automation has it; or a crew with no
  captain), any seated person may. The server decides: a request from anyone else is refused with "Captain's call".
- **One rule, both ends** (design 3): `sc-core` holds who may start (`combat::may_start`); the server applies it and the
  client asks it only to decide whether to draw the control.
- **The control** (design 4): a chip in the console's title band, in the order's place, while the round waits: a
  keycap with the bound key, `START MISSION`, and how many of those aboard are ready. A click, a tap or the key starts
  the countdown. Drawn in `consoles.html` first, with two parity states.
- **A key in data** (design 5): `data/input/bindings.json` gains `start` (Return), free at every console.
- **On the wire** (design 6): `ClientMsg::StartMission` (tag 14), protocol 18; refusals as every other.

## Impact

- `sc-core::combat`: `may_start`, `Drill::start_mission`, `Refusal::CaptainsCall`.
- `sc-net`: the message, the session's `start_mission`, the protocol number. `sc-server`: the message's handler.
- `sc-client`: the title band's chip (`console/mod.rs`), `StartView`, the key and the click (`drill.rs`), the binding
  (`bindings.rs`).
- `docs/mockups/consoles.html`: the chip and the parity states `start-captain` and `solo-start`.
- Pi 5 budget: one title-band chip, a few dozen UI vertices in the console's existing batch and no new draw call; one
  one-byte message per press; one comparison a tick on the server. Nothing in 3D.
