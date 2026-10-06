# Proposal: ceilings and trims as baked panels

## Why

The owner, 2026-10-05, on the wall panel prototype (`wall-panels`): "Ooh now that is good. Walls
and ceilings still look awkward update those as well using your new textures skill".

The walls now take designed panels, but the structure around them does not:
- every ceiling is still one tiled texture (`ceiling` in crew spaces, `machinery` in working
  spaces);
- the ribs, beams, coves, baseboards and door frames wear `trim`, which is the same pale
  1 m square (`tech_panel`) the walls used to be.

So the frame the generator builds (`deck-pipeline` 5a) reads paler and flatter than the panels
it frames (`wall-panels-*-after.png`). The references (`docs/analysis/texture-references.md`)
have structural ceilings packed with grilles, cable trays and pipes (X1's lintels, X2's framed
ceiling recess), and trims that read as steel members, not tiles.

## What Changes

- **Ceilings are dressed cell by cell.** The beams at every frame already divide a ceiling into
  2 m bays across the room. Each bay is cut into 2 m cells from the room's centreline outward,
  and each cell takes a **ceiling module** by the same stable rule as the walls: plate, grille,
  fan, cable tray, pipe run, access hatch, ribbed sheet, or a lamp surround where the kit
  places a lamp. Neighbours never match.
- **Trims are steel, not tiles.** Ribs, beams, coves, baseboards and door frames take **trim
  strips**: textures that run along a member's length, with u along the member and v across
  its face. They show flanges, rivet lines, lightening holes and cable trays, and a hazard edge
  in working spaces.
- **Two sets, as for walls:** crew and working, eight ceiling modules and one trim layer each.
- **Made the same way.** Modelled with the hard-surface kit and baked in Blender by the panel
  build (CLAUDE.md section 9, as amended on the owner's answer to V2), into the same texture
  array.
- **Turned on everywhere.** The owner approved the panels (V1), so the mockups' rooms take the
  wall panels, the ceilings and the trims together.

## Capabilities

### New Capabilities

- `ceilings-and-trims`: how a ceiling is cut into cells, which module each cell takes, and how
  a trim member takes its strip.

### Modified Capabilities

None in `openspec/specs/`. It extends the unbuilt `wall-panels` (same rule, same build, same
data file) and `deck-pipeline` 5a's finishes, which name the layers the trims take.

## Impact

- **Data:** `data/materials/panels.json` gains `ceiling` and `trims` sections for each finish.
- **Tooling:**
  - the panel build (`tools/blender/build_wall_panels.py`) gains the ceiling modules and the trim
    strips;
  - the kit (`shipkit.js`) dresses ceilings and UVs trims behind the same option as wall panels;
  - the `wall-panels` comparison page shows ceilings and trims before and after.
- **Skill:** the panel texture method gets its own skill (`panel-textures`), as the owner
  asked for a "textures skill": modules, bands and cells, the rule, baking, the contact sheet.
- **Pi 5 budget:** about 18 more texture layers (6.3 MB at 128 px per metre, 1.6 MB at 64). Ceiling
  cells add about 2 triangles a cell (about 610 cells on the Tern). Trims add none, because only
  their texture coordinates change. No new draw calls.
