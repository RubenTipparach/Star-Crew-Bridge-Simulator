# Tasks: ceilings and trims

## 1. Write-up, build and mockups

- [x] 1.1 Proposal, design and spec deltas, from the owner's "Walls and ceilings still look awkward update those as well using your new textures skill".
- [x] 1.2 The panel build gains the eight ceiling modules and the trim layer per finish (design 2-3, 5); contact sheet; byte-reproducible. Built 2026-10-06 (`tools/blender/build_wall_panels.py`): seven trim rows with the pillars' base and capital (design 8), the contact sheet with an illustrative ceiling and the trims as members; a second build wrote the same bytes.
- [x] 1.3 `panels.json` gains `ceiling` and `trims`; the kit dresses ceilings by cell and UVs trims by member, behind the panels option. Checked: without the option every compartment's parts hash the same as before; `kit_report.mjs --panels` measures the dressing (design 7).
- [x] 1.4 The `panel-textures` skill: how to design, model, bake, judge and add a panel module, a ceiling module or a trim strip (`.claude/skills/panel-textures`).
- [x] 1.5 Panels, ceilings and trims on in the bridge, the deck plan, light baking and the comparison page (design 6), at 128 px per metre; shots before and after, looked at (`docs/screenshots/mockups/`).
- [ ] 1.6 The bridge variants with panels (the main session), and survey U1 with the shots.

## 2. The deck build (`deckc`, with `deck-pipeline` and `wall-panels`)

- [ ] 2.1 Ceiling cells and trim coordinates from the brushes, beams and lamps; tests `neighbouring_ceiling_cells_never_match`, `a_lamp_sits_in_its_surround`, `a_long_beam_tiles_without_a_seam`.
- [ ] 2.2 The digest of the Tern's ceilings pinned.

## 3. Verification

- [ ] 3.1 Captures of every compartment kind in the three lighting states in `docs/screenshots/`.
- [ ] 3.2 Texture memory and triangle counts on the Pi 5 probe (owner, on hardware).
- [ ] 3.3 Move each requirement into `openspec/specs/ceilings-and-trims/` with the test that proves it.
