# Tasks: the ship plan view

## 1. Write-up

- [x] 1.1 Proposal, design and spec delta from the owner's "I kind of like the floor plan of the ship view ... downscale some of the geometry on furniture ... monitor players moving about on the ship" (2026-10-08).

## 2. Light models

- [ ] 2.1 `hs_kit.py` writes each prop's light model in its glb, within section 2's budget, in the same atlas; every set rebuilt.
- [ ] 2.2 `deckc` stores both levels; the walk switches at 15 m.

## 3. The view

- [ ] 3.1 The engine's plan camera: deck, exploded and whole; the cut and the wall-top bands.
- [ ] 3.2 Markers for every body from the snapshot; hazards from the simulation.
- [ ] 3.3 The whole ship in plan measured on the Pi 5 (the probe's scene 6).

## 4. The first version in the engine (design 5)

- [x] 4.1 The deck shader's clip height; the exploded decks drawn with it.
- [x] 4.2 M toggles the map from any mode; orbit, zoom, Tab steps the decks.
- [x] 4.3 Markers for the player and every bot crew member, names by the UI layer.
- [x] 4.4 Shots in `docs/screenshots/engine/` (`map-1-all-decks.png`, `map-2-deck-B.png`, from `sc-client --headless --lobby-test`).

## 5. Damage control mode (design 6, owner 2026-10-09)

- [x] 5.1 Design 6: the five layers, how the mode is opened, what is seen at rest, the engine's wait for the simulations.
- [x] 5.2 `docs/mockups/damage-map.html`: the mode on the mockups' simulation, with a hit, a fire and a severed conduit to show; shots.
- [ ] 5.3 In the engine, once `sc-core` has damage, power and fire: the mode on the M map.
