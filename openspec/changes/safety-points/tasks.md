# Tasks: safety-points

## 1. Placement and data

- [x] 1.1 Proposal, design and spec delta from the owner's "fire extinguishers and med kits across the ship" (2026-10-09).
- [ ] 1.2 `tools/safety_points.py`: the rule (design 2), writes `data/ships/tern/kit.json`, reports counts and the longest walk per deck.
- [ ] 1.3 `tools/layout_check.py` validates `kit.json` (design 4).

## 2. Props and mockups

- [ ] 2.1 `tools/blender/build_safety_props.py`: extinguisher, bracket, cabinet, sign within budget; glb and manifest.
- [ ] 2.2 shipkit or propkit draws the points; `deck-plan.html` and `fire.html` show them; shots in `docs/screenshots/`.

## 3. Engine

- [ ] 3.1 `deckc` writes the points as static props, the kits as item entities.
- [ ] 3.2 Take, carry, return and refill (`crew-on-deck` 6), with the spec's scenarios as tests.
