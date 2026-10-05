# Tasks: surface materials

## 1. Pipeline (documentation tooling and data)

- [x] 1.1 `tools/materials/`: the graphs copied from Undercity with provenance, `build_ptex.py` (reproduces them byte for byte), `export_materials.sh` (Material Maker or `--from-fps`), `postprocess.py` (validation, ramp, relief bake, layer fit, palette, emission alpha, contact sheet, digest).
- [x] 1.2 `data/materials/materials.json` and the first eleven layers in `assets/textures/`; contact sheet looked at; two runs give one digest.
- [x] 1.3 The `material-maker` skill, its provenance row, CLAUDE.md sections 9 and 14.
- [x] 1.4 The mockups draw with the layers: `shipkit.loadMaterials`, `surfaceMaterial`, world-projected UVs, the `materials` INLINE marker, texture memory on the meter.

## 2. Owner and look

- [ ] 2.1 Owner answers S1 (density and look) in the survey, with the contact sheet and mockup shots beside it.
- [ ] 2.2 Render the graphs with Material Maker on the owner's machine (`MATERIAL_MAKER_DIR=... tools/materials/export_materials.sh`), compare the contact sheet with the `--from-fps` set, and commit the fresh layers.

## 3. Engine (on request, after the probe)

- [ ] 3.1 `sc-probe` scene 3 with and without the texture array; record the cost on a Pi 5.
- [ ] 3.2 `sc-render`: load `materials.json` and the PNGs (straight alpha), build the array with mipmaps, the deck shader's array fetch and emissive term.
- [ ] 3.3 `deckc`: write each vertex's layer and world-projected texture coordinate from the compartment's finish; a test compares its layer choice with `detailing.json`'s finishes.
- [ ] 3.4 Move the requirements into `openspec/specs/surface-materials/spec.md` as each is made true with its test.
