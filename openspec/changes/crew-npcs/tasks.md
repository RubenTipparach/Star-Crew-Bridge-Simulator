# Tasks: NPC crew with jobs

## 1. Write-up

- [x] 1.1 Proposal, design and spec delta from the owner's "maybe we can add NPCs that have jobs on the ship too" (2026-10-08).

## 2. The rules (sc-core)

- [ ] 2.1 `sc-core::crew`: the company from the seed, departments, watches, the schedule; tests.
- [ ] 2.2 The nav graph from the compartment graph (`deckc`), A* over portals; tests on the Tern.
- [ ] 2.3 Tasks through the players' intents: walk, use, repair, carry, treat, sit.
- [ ] 2.4 Orders and red-alert stations.

## 3. Seen

- [ ] 3.1 NPCs in the engine client walking their schedules; on the plan view (`ship-plan-view`).
- [ ] 3.2 Server CPU with eight NPCs measured on the 4 GB Pi.

## 4. The first version (design 7)

- [x] 4.1 `data/crew/company.json`; the company from the seed with names from `sc-core::names`.
- [x] 4.2 `sc-core::nav`: the walk grid and A*; tests.
- [x] 4.3 Places from the deck file's compartment floors; the bots' loop; figures from `deckc`.
- [x] 4.4 Shots of the bots aboard in `docs/screenshots/engine/` (`ship-corridor-B-normal.png`, the map shots). The grid builds in about 30-40 ms on a cloud CPU (7,884 cells); not measured on a Pi.
