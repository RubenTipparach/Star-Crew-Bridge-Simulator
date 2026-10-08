# Tasks: deck pipeline

## 1. Data and the shared rules

- [x] 1.0 The detail rules as data: `data/ships/tern/detailing.json` (schema `starcrew.detailing/1`, design section 5a), drawn by the mockups' `shipkit.js` and counted by `tools/mockups/kit_report.mjs` (documentation tooling, 2026-10-05).
- [ ] 1.1 Load and validate `detailing.json` in `sc-core` (unknown key is an error; every finish role names a material), with the prop triangle costs of design section 5 beside it.
- [ ] 1.2 The detail file schema (`starcrew.deck-detail/1`): brushes as plane lists, props with mesh ids and poses, light fixtures with three state colours; a validator.
- [ ] 1.3 Move the layout rules into `sc-core`'s layout module with tests that read as sentences (one per `ship-layout` requirement); in the same commit, make `tools/layout_check.py` call `deckc --check-layout` or retire it, so the rules exist once.
- [ ] 1.4 The `.deck` format module (header, chunk table, CRC-32, every chunk's record types) shared by `deckc`, `sc-server` and `sc-render`, with round-trip and corruption tests.

## 2. Generator

- [ ] 2.0 `deckgen` generates design section 5a's detail from the brushes with the same rules as `shipkit.js` `buildCompartment`; a test compares its per-compartment counts by role with `kit_report.mjs --json` for the Tern, so the two implementations cannot drift apart unnoticed.

- [ ] 2.1 `deckgen`: shell slabs per compartment outside its own air, split at portal openings (zero-thickness partitions, design section 3, pending K1).
- [ ] 2.2 Door, pressure door, hatch, ladder, hoist, window and bay door frames from the portal's kind and clear size (reveal 0.1 m, casing 0.1 m, proud 0.1 m).
- [ ] 2.3 Trims, ribs, beams and lamp fixtures by the kit rules; stairs, landings, catwalks and the mezzanine ring from the layout's fixtures.
- [ ] 2.4 Props for every station, system and fixture in the layout, with a collision proxy each.
- [ ] 2.5 The Blender detail export: layout as locked reference, convexity refusal, `PROP_` and `LIGHT_` empties, the axis rotation, JSON out; a round-trip test on a sample compartment.

## 3. Compiler: geometry

- [ ] 3.1 Brushes to faces; hidden-face removal against the compartment's other brushes.
- [ ] 3.2 Call the `light-baking` baker per compartment for the three states; take back tessellated faces and colours.
- [ ] 3.3 T-junction repair and welding (0.1 mm); merge by pass; the 28-byte vertex; 16-bit or 32-bit indices per draw.
- [ ] 3.4 Collision brushes with contents; the k-d tree for compartments over 32 colliding brushes.
- [ ] 3.5 Entities: seats, spawns, ladders, stairs, movers, light fixtures, system anchors, each keyed to its layout id.

## 4. Compiler: checks (each refuses with names and coordinates)

- [ ] 4.1 Brush convexity and closure; finite numbers.
- [ ] 4.2 Inside the air and clear of portal openings.
- [ ] 4.3 Z-fighting: plane buckets, 5 mm coplanar, 0.5 degree normals, 1 mm^2 overlap; 1 cm rule for deliberate parallels; back to back allowed. Tests from Undercity's known cases (a trim flush with a wall, a lintel flush with a ceiling).
- [ ] 4.4 Frames fit their faces with 1 cm to spare.
- [ ] 4.5 People stand clear with the `crew-on-deck` capsule; clear width of crew portals; the walk flood fill from every spawn to every seat.
- [ ] 4.6 Budgets per compartment and the worst visible set (offline portal flow) against the interior pass ceiling.
- [ ] 4.7 A regression deck for every refusal: a small layout that fails exactly one check, committed with its expected message.

## 5. Run time

- [ ] 5.1 Loader: header, CRCs, required chunks, layout hash against the server's layout; server reads core chunks only.
- [ ] 5.2 `sc-core` capsule trace against brush planes, compartment tracking across portals, tests that read as sentences (walking into a wall, through a door, up a stair).
- [ ] 5.3 `sc-render` portal traversal with narrowing rectangles, door state from the snapshot, scissor per compartment, exterior rectangles for `ship-frames`.
- [ ] 5.4 The deck shader: three colour sets, state weights, dimmer, flicker, movers and dynamic lights from a uniform block per compartment.
- [ ] 5.5 Glass: distance sort, or the glass BSP where a compartment needs one.

## 6. Prove it

- [ ] 6.1 `deckc --report` for the Tern; replace the estimate table in design section 11 with the compiled numbers in the same commit.
- [ ] 6.2 `sc-probe` on a Pi 5 (1 GB): portal traversal time, the Tern's worst view, the whole Tern drawn without culling; record in `docs/validation/`.
- [ ] 6.3 Captures in `docs/screenshots/deck-pipeline/`: each refusal's message, the Tern from the helm, the main corridor with doors open and closed, red alert, emergency power.
- [ ] 6.4 Owner review of the open questions K1-K4 with the `deck-plan` mockup shots.
- [x] 6.5 `docs/mockups/deck-plan.html` presents this change: lamp fixtures by the kit rule, three colour sets blended per compartment by one shader, partitions as single lines; shots in `docs/screenshots/mockups/deck-plan-*.png` (documentation tooling, not engine code).

## 7. The first deck in the engine (design section 13; the owner: "just build it")

- [x] 7.1 The deck plan exports every compartment as it draws it (`window.MOCKUP_EXPORT_DECK`), and `tools/deck/export_deck.mjs` writes `build/deck/<ship>/`.
- [x] 7.2 `sc-core::deck`: the first compiled deck format (index, vertices, indices, texture mips), read with every range checked; tests for a round trip, a truncated file and a compartment pointing outside it.
- [x] 7.3 `sc-tools deckc`: packs with `sc-core`'s vertex packer, merges identical vertices, remaps and resamples the texture arrays into one, builds the mip chain.
- [x] 7.4 `sc-client` draws the whole Tern in the three states and flies through it; headless shots from the deck plan's walk viewpoints in `docs/screenshots/engine/`.
- [ ] 7.5 The console faces and the viewscreens and windows (space) in the engine.
- [x] 7.6 The walk world exported with the deck (design section 13a): the deck plan's collision soup and walk entities in `MOCKUP_EXPORT_DECK`, carried by `export_deck.mjs`, packed by `deckc` into deck version 2, read with every range checked.
- [x] 7.7 `sc-core::walk` on Rapier's character controller with `data/crew/walk.json` (read by `shipwalk.js` too); tests that read as sentences.
- [x] 7.8 `sc-client` walks: on its feet on the bridge, F to fly, a scripted headless walk with shots in `docs/screenshots/engine/`.

