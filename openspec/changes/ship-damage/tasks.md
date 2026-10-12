# Tasks: ship-damage

- [x] 1.1 Proposal, design and spec delta from the owner's direction (2026-10-10).
- [x] 2.1 The hull test, armour sections (armour by weapon, `armour-and-missiles` 1) and the march in
  `sc-core::combat::damage`, ported from `resolveHit`, with tests (workstream B, 2026-10-11).
- [x] 2.2 The cascade and `capability`, with tests; loads for both laser banks and both tubes in `power.json` (workstream B).
- [x] 2.2a Capability wired into the drill (drive, shields, sensors, tubes, banks, turrets, reactor): workstream A.
- [x] 2.3 Fires and the two teams, with `tick`'s `priority` for the captain's raised system; their numbers in
  `damage.json` (`fire`, `cascade`, `teams.path_factor`) (workstream B).
- [x] 2.3a The captain's REPAIR order passed to `tick` as its priority: workstream A.
- [x] 2.4 The Hound's layout, in `data/ships/hound/layout.json` (`starcrew.combat-layout/1`; the work plan moved it
  out of `data/enemies.json`), read by `ShipLayout::hound` (workstream B).
- [x] 2.5 Snapshot and events (`sc-net`); debris and the SHIP plan (`sc-client`): the Tern's integrities, fires,
  breaches and teams in the snapshot; `room_states` and `team_room` in the core; the plan's rooms, the teams as crew
  dots, and the status strip's fires and breaches.
- [ ] 2.6 A recording: a hit breaking a system, a fire, a team fixing it.
- [x] 2.7 After the fight: the teams repair through the debrief, the next drill waits for them (design 9); the debrief panel says what is being repaired.
