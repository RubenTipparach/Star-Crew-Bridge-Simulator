# Tasks: wall panels

## 1. Write-up and prototype

- [x] 1.1 The references described (`docs/analysis/texture-references.md`); proposal, design and spec deltas for `wall-panels`.
- [ ] 1.2 `tools/blender/build_wall_panels.py`: the ten modules and the strip layer for each finish, modelled with boolean cutters and baked (design section 6, option B), at 128 px per metre and reduced to 64; a contact sheet in `docs/screenshots/materials/`.
- [ ] 1.3 `data/materials/panels.json` (proposed) and the kit's opt-in wall dressing in `docs/mockups/lib/shipkit.js` (bays, bands, the rule), leaving every other page unchanged.
- [ ] 1.4 `docs/mockups/wall-panels.html`: today's walls against the panels in five rooms, both texel densities, three lighting states; screenshots looked at.
- [ ] 1.5 Survey V1 and V2 (and S1 with the new shots); fold the answers back. If B or C is picked, amend CLAUDE.md section 9 in the same commit.

## 2. Data and tools

- [ ] 2.1 `panels.json` validated by `tools/layout_check.py`'s data checks: every module's layer exists, weights are positive, band heights fit the shortest room.
- [ ] 2.2 The panel post-process joins the materials' (recolour, reduce, emission mask, digest).

## 3. The deck build (`deckc`, with `deck-pipeline`)

- [ ] 3.1 Bays and bands from the brushes, ribs and openings; tests `neighbouring_bays_never_match`, `a_door_is_flanked_on_its_side`, `editing_one_wall_leaves_the_others_alone`.
- [ ] 3.2 Bay-local texture coordinates; test `a_short_bay_crops_only_margin`.
- [ ] 3.3 The digest of the Tern's walls pinned.

## 4. Verification

- [ ] 4.1 Captures of every compartment kind in the three lighting states in `docs/screenshots/`.
- [ ] 4.2 Texture memory and triangle counts on the Pi 5 probe (owner, on hardware).
- [ ] 4.3 Move each requirement into `openspec/specs/wall-panels/` with the test that proves it.
