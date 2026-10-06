# Tasks: the command suite

## 1. Write-up, patch and mockup

- [x] 1.1 Proposal, design and spec deltas, from the owner's "can we do the more circular bridge? but have like side rooms for meetings, captains quarters and stuff?" (2026-10-06).
- [x] 1.2 `tools/bridge_variants.py`: `variant_b` takes the ring's start, the banks and the helm layout; `bridge_variants.json` unchanged (`--check`).
- [x] 1.3 `tools/command_suite.py` writes `data/ships/tern/command_suite.json` and checks it: the layout rules on a patched copy, platforms and consoles inside the bridge, furniture inside its room, apart, and clear of every door's zone, and the bridge's stairs clear of its doors; `--sightlines`.
- [ ] 1.4 The suite props in Blender (`tools/blender/build_suite_props.py`, `assets/models/suite`), sharing one module with the bridge props' build (whose output stays byte for byte the same); contact sheet looked at.
- [ ] 1.5 `docs/mockups/command-deck.html`: deck A's bridge, side rooms and passage from the patched layout, with panels, the Blender consoles and furniture, console faces, the three lighting states and the free camera; the props code shared with `bridge-variants.html` in `lib/propkit.js`.
- [ ] 1.6 Shots of every view, looked at; the Pi 5 numbers of design section 9 measured on the page and filled in.
- [ ] 1.7 Survey: B11 answered (the owner's words), A1 asked with the shots; A2-A4 recorded as recommendations taken.

## 2. Apply the patch (its own commit)

- [ ] 2.1 `layout.json` takes the patch; `layout_check.py` learns `platforms` (the dais fixture retires); the furniture becomes the deck's detail file.
- [ ] 2.2 `power.json`, `atmosphere.json` and `damage.json` take the new rooms.
- [ ] 2.3 Rerun every plan-dependent number (design section 8) with the tool that made it, and move each quote with a dated note: `bridge-stations` 11.1 and 12, `reference-ship-tern` section 2 and the deck A map, `life-support`, `power-grid`, `damage-control`, `crew-on-deck` (`tools/walk_times.py`'s bridge routes), `deck-pipeline` 11, `light-baking`.
- [ ] 2.4 Every mockup that reads the layout re-shot and looked at; the variants page keeps the record of the choice.

## 3. The deck build (`deckc`)

- [ ] 3.1 Compile the suite's rooms, platforms and furniture; tests `the_side_doors_open_at_walkway_level`, `furniture_keeps_every_door_clear`, `the_captain_sees_the_whole_screen`.

## 4. Verification

- [ ] 4.1 Captures in the three lighting states; triangles and draws per room on the Pi 5 probe (owner).
- [ ] 4.2 Move each requirement into `openspec/specs/command-deck/` with the check that proves it.
