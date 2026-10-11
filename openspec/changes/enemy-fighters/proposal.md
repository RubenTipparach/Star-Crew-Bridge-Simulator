# Proposal: enemy fighters, the gunner and the cannons' new role

## Why

The owner, 2026-10-10: "cannon shots are mainly for strafing off fighters", "Cannons will knock down shields slowly
and do minimal damage to enemy ships. Shields will typically regen faster than cannon can damage", and "have the enemy
launch two fighters as well so a player can assume the gunner seat and shoot them down". The full words are in
`laser-banks`' proposal.

## What Changes

- **Two enemy fighters** (design 1): the Hound launches two Jackals from its hangar 20 s into the fight (`weapons-and-
  shields` 13's Jackal: orbit, attack run, fire, retreat); a broken hangar (`ship-damage`) cannot launch them.
- **The cannons' role** (design 2): a bolt does its full damage to a fighter or a missile and a quarter of it to a
  ship (`capital_factor` 0.25), so shields outlast cannons and the cannons' work is the fighters.
- **The gunner seat** (design 3): GUNNER, in the dorsal pod's seat (`gunner_dorsal`), takes the dorsal turret by hand:
  a turret sight (the 3D view along the barrel, a lead pip on the chosen target), aim with the mouse or a stick, fire
  with the trigger. Unmanned, the turret is on AUTO.
- **Fighters attack parts of the ship** (design 4): their bolts are cannon bolts; a fighter's run aims at a part (a
  turret, the bridge, a bank), so the precise hits of `ship-damage` land there.

## Impact

- `sc-core::combat`: a `fighters` module (state, flight, AI, guns), the gunner's commands; bolts that hit fighters and
  missiles. `sc-net`: fighters in the snapshot. `sc-client`: the fighter drawn, the GUNNER sight (mockup first).
- `weapons-and-shields` 6-7 and 13 (manned turrets, the Jackal), `shuttle-bay-and-fighters` stay the full design.
