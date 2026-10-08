# Tasks: kick lights

## 1. Write-up and mockup

- [x] 1.1 Proposal, design and spec delta, from the owner's "star trek likes to put in indirect floor lights, we should have some of that on the bridge, and behind the consoles" and references K1 and K2 (2026-10-07).
- [x] 1.2 `kick_strip` in `data/lighting/fixtures.json`, the `kick` light in `tools/lighting_check.py`, the `kick` state colour in `ShipKit.LIGHTING`; `kicks` in `data/ships/tern/detailing.json`.
- [x] 1.3 The kit: toe kicks on platform edges (`ShipKit.buildPlatforms`, `opts.kicks`), the covers moved in under a recess; console strips (`PropKit.placeKicks`).
- [x] 1.4 The deck plan draws them and bakes them as emitters in the three states.
- [x] 1.5 Shots of the bridge in the three states beside the bake before, looked at; the numbers in design section 7; checks (z-fighting, inlining, lighting data, specs, dashes).
- [x] 1.6 Light columns behind the terminals (design section 2a, the owner: "vertical lights behind terminals too"): `column_strip`, a column each side of every wall bank.

## 2. Into the deck (with `deck-pipeline` and `light-baking`)

- [ ] 2.1 `deckgen` generates the recess faces on a platform edge brush and a `kick_strip` fixture record per kicked edge (`deck-pipeline` section 5a).
- [ ] 2.2 `sc-tools bake` bakes `kick_strip` as an area fixture (`light-baking` task 3.2).

## 3. Props

- [ ] 3.1 `build_bridge_props.py` writes each console's kick strips into `props.json` (prop space, with the fixture type), and the kit reads them there in place of `detailing.json` `kicks.props`.
- [ ] 3.2 Lit stair treads (K2's step faces), if the owner wants them after the shots.
