# Proposal: floors as baked panels

## Why

The owner, 2026-10-05, after the wall panel prototype: "Walls and ceilings still look awkward
update those as well using your new textures skill", then "Floors and ceilings in mean. Wall panels
look great..some wall pillars might need retouching up too".

Every floor is still one tiled texture: `deck_tiles` in crew spaces and `deck_plate` (diamond plate)
in working spaces, repeating every 2 m, with a raised runner down each corridor. The owner's
references (`docs/analysis/texture-references.md`) floor their spaces differently:
- X1 has dark grate tiles with a lighter solid walkway down the middle;
- X2 has square grates, each in its own raised frame, with floor vents by the walls.

## What Changes

- **Floors are dressed cell by cell.** As with ceilings (`ceilings-and-trims`), each 2 m bay
  between frames is cut into 2 m cells from the centreline outward. Each cell takes a **floor
  module**: a framed grate, deck plate, an access plate, a trench cover, a drain grille, a vent,
  or **walkway** plate where people walk.
- **A walkway, not a runner.** Cells on the line from each door to the room's centre (and down the
  middle of a corridor) take walkway plate. Grates and service plates fill the rest, as in X1. The
  corridors' raised runner retires, since the walkway does its job.
- **Hatches keep their rims.** A cell holding a floor portal takes plain plate, under its
  existing hazard rim.
- **Same build, same array, two finishes:** eight modules each for crew and working spaces,
  baked by the panel build.

## Capabilities

### New Capabilities

- `floor-panels`: how a floor is cut into cells, where the walkway runs, and which module each
  cell takes.

### Modified Capabilities

None in `openspec/specs/`. It extends the unbuilt `wall-panels` and `ceilings-and-trims` (same
rule, build and data file) and retires `deck-pipeline` 5a's corridor runner once it is built.

## Impact

- **Data:** `data/materials/panels.json` gains `floor` per finish. `detailing.json`'s `runner`
  is dropped when floors are dressed.
- **Tooling:** the panel build gains the floor modules; the kit dresses floors behind the panels
  option; the comparison page shows floors before and after.
- **Pi 5 budget:** 16 more texture layers (5.6 MB at 128 px per metre). About 2 triangles a cell
  (about 610 cells on the Tern), less the runners' triangles. No new draw calls.
