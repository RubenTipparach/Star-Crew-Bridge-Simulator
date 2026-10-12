# Proposal: missions

## Why

The First Mission Gate (owner, 2026-10-10), step 3: "The captain picks a mission ... the demo mission against a Grabon
ship", and step 14: "Warp out ends the scenario". Today there is one mission, fixed when the server starts, and a
fight ends only by a kill, a loss or the time limit.

## What changes

- **A mission list** in the room: every file in `data/missions/`, its title and one line. The captain (or, in single
  player, the player) picks one before anyone is READY; the server loads it for everyone.
- **The Grabon demo mission**: a Grabon raider at a derelict relay, `data/missions/demo-grabon.json`, with the Grabon
  raider as a new enemy class.
- **Warp out**: the captain orders JUMP; Helm holds JUMP on its THRUST panel; the drive spins up for 10 s, then the
  Tern jumps and the mission ends. After a victory that is MISSION COMPLETE; before it, the Tern has escaped.
- **The end screen**: the outcome, the fight's numbers and the ship's state, and Return to menu.

## Impact

- `sc-core::combat` (the mission chosen in Muster, the jump, the outcome), `data/missions/`, `data/enemies.json`,
  `sc-net` (the pick, the jump's state), `sc-client` (the list in the room, Helm's JUMP, the end screen), the captain's
  order verbs. Mockup first: `docs/mockups/mission-room.html` states 2, 4 and 5; Helm's JUMP goes into
  `docs/mockups/consoles.html` once approved.
