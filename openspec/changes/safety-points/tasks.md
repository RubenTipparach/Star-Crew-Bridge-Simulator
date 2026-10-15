# Tasks: safety-points

## 1. Placement and data

- [x] 1.1 Proposal, design and spec delta from the owner's "fire extinguishers and med kits across the ship" (2026-10-09).
- [x] 1.2 The rule (design 2, corridors evenly from the owner's word of 2026-10-12) as `sc-tools safety-points` rather than a Python tool, so each place is checked against the compiled deck as built (its wall, nothing proud of it, floor before it): writes `data/ships/tern/kit.json`, reports counts and the farthest floor per deck (2026-10-12: 45 points, 21 main door, 7 big room, 12 corridor, 5 reach; farthest floor 8.6, 9.5 and 9.4 m on decks A, B and C). `the_committed_kit_file_is_the_one_the_layout_places`, `every_room_has_one_and_the_corridors_are_spaced_evenly`.
- [ ] 1.3 `tools/layout_check.py` validates `kit.json` (design 4).
- [x] 1.4 Design 3a from the owner's "a 15 second cool down replenishes it" (2026-10-10): `fire.extinguisher.bracket_restock_s`.

## 2. Props and mockups

- [ ] 2.1 `tools/blender/build_safety_props.py`: extinguisher, bracket, cabinet, sign within budget; glb and manifest.
- [ ] 2.2 shipkit or propkit draws the points; `deck-plan.html` and `fire.html` show them; shots in `docs/screenshots/`.
- [x] 2.3 The mockups' points and their restock (3a) in one place, `docs/mockups/lib/firefight.js`, used by `fire.html` and `deck-plan.html`; shots.

## 3. Engine

- [ ] 3.1 `deckc` writes the points as static props, the kits as item entities.
- [ ] 3.2 Take, carry, return and refill (`crew-on-deck` 6), with the spec's scenarios as tests.
- [x] 3.3 The first build (design 5a): extinguisher brackets read from `kit.json` (`ShipLayout::brackets`), taken at any full bracket, restocked in 15 s by the server (`a_player_takes_an_extinguisher_from_a_bracket_and_it_is_back_15_s_later`), the empty ones in the snapshot (protocol 13), drawn as the mockups paint them; the playtest takes from the nearest (2026-10-12).
