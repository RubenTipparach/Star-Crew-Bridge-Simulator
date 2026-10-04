---
name: blender-csg-levels
description: Build game levels in Blender the CSG way, scripted. A solid shell is carved by boolean cutter brushes (rooms, barrel and groin vaults, arches, door slabs, pits, an oculus), with additive detail brushes, generated UT99 trims, ENT_ entity empties, a z-fighting check that refuses bad geometry, and a glTF export that Godot imports with collision, lightmaps and entities. Use when making or editing a Blender level or a level generator (tools/blender/build_*.py), laying out rooms, vaults, doors, stairs or trims, exporting a level to Godot, rendering a level capture, or taking this pipeline to another project.
metadata:
  author: Undercity (Claude Code), distilled from Brushfire's E1M3 and Undercity's hub builds
  version: "1.0"
---

## In Star Crew

Copied from the Undercity repository (`fps-game-demo`, `.claude/skills/blender-csg-levels` at
`f6cd25c`, 2026-10-04); provenance in `.claude/skills/PROVENANCE.md`. Everything below this note
is the source's text, kept so a later copy can be diffed. It is the reference for how the owner
likes levels built (scripted Blender CSG, one layout source, a z-fighting check that refuses
bad geometry) while `openspec/changes/deck-pipeline` designs Star Crew's own deck pipeline.

What does not apply here:
- **The Godot tail** (import presets, `.tscn` scenes, LightmapGI, Jolt): Star Crew has a custom
  engine. Decks compile to `.deck` files with baked vertex lighting (`deck-pipeline`).
- **Its file paths** (`tools/blender/`, `tools/godot/`, `game/`): those are Undercity's. Use
  `copy_kit.py` and `references/porting.md` if the deck pipeline adopts the kit.
- **Its budgets**: Star Crew's are the Pi 3 table in `openspec/changes/engine-stack/design.md`.

What does apply: one layout source (`data/ships/<id>/layout.json` here), brushes and cutters,
no z-fighting (CLAUDE.md section 8), people stand clear of the level, write it up first.

# CSG level design in Blender

The owner's call (CLAUDE.md 1): Blender is "definitely" the level pipeline, borrowing
TrenchBroom's brush-style detailing and entity placement. This skill is how a level gets
made that way. It works in this repository and travels to others (see "Another project").

```
build_<level>.py (Godot metres)
  Shell (solid box) - Boolean DIFFERENCE, Exact, material Transfer - Carve collection (cutters)
  + Detail collection (additive brushes, trims)   + Entities collection (ENT_<kind>_<n> empties)
  -> detailing.assert_no_zfighting()   refuses to write a level that fails
  -> <level>.blend (editable) + <level>.glb (booleans applied, outside culled, world UVs, Level-col)
Godot: import preset (material remap, UV2) -> post-import (ENT_ -> scenes) -> level .tscn -> bake
```

## Before you start

1. **Write it up first** (CLAUDE.md 3). A new level is an OpenSpec change with its layout,
   numbers and the three ways into each key room (7.1). Build on a request to build.
2. **One layout source** (7.1). A city-scale level keeps its layout in
   `tools/levels/layouts/<id>.py`, shared by the design map and the Blender build: read
   [references/city-scale.md](references/city-scale.md). A small interior level can keep its
   rooms in its build script, as `build_cistern.py` does.
3. **Import the kit, never copy it** inside this repository (CLAUDE.md 5.1). The kit:

| File | Gives you |
|---|---|
| `tools/blender/blendkit.py` | `B()` axes, `box_bm`, `cylinder_bm`, `material()`, `project_uvs`, `add_world_uv`, `export_level`, `save_reproducible` |
| `tools/blender/build_cistern.py` | `Level`: `room`, `vault`, `block`, `column`, `pipe`, `arch_ring`, `ent`; the E1M3 reference level |
| `tools/godot/detailing.py` | `Room`, `all_trims`, `keep_out`, `FRAMES`, `frame_solids`, `assert_no_zfighting` (pure Python) |
| `tools/godot/gen_level_blender.py`, `level_common.py`, `tscn.py` | the Godot scene around the glb: environment, LightmapGI, probes, zone ambient, navigation |
| `game/addons/brushfire_tools/blender_level_import.gd` | ENT_ empties to scenes, lights, triggers, markers |

## The workflow

1. **Start from the example.** Copy
   [templates/example_level.py](templates/example_level.py) to
   `tools/blender/build_<level>.py` and replace `build()`. It is a complete, working level:
   groin-vaulted hall, door, barrel-vaulted corridor, chamber with a pit, trims, lights.
2. **Plan the air volumes in Godot metres** (x right, y up, -z north). Every room is an
   axis-aligned box; openings are thin boxes (door slabs) that overlap both rooms by 0.1 m.
3. **Shell and cutters.** `mesh_object("Shell", box_bm(lo, hi, ...))`, then `L.room(...)` and
   `L.vault(...)`. One Boolean modifier on the shell: DIFFERENCE, operand COLLECTION = Carve,
   solver EXACT, material_mode TRANSFER, use_hole_tolerant. The cutter's faces carry the
   materials: bottom = floor, sides = walls, top and curved faces = ceiling.
4. **Detail.** `L.block` (registers the box for the z-fighting check), `L.column` (base and
   capital, always), `L.pipe`, `L.arch_ring` (bevelled voussoirs, jambs, plinths).
5. **Trims.** `detailing.all_trims(L.rooms, avoid=[keep_out(fixture_centre, half_size)])`, each
   box through `L.block(..., skip=())` with a 0.02 m bevel. Pass light fixtures as `avoid`.
6. **Entities.** `L.ent(kind, pos, yaw, **extras)`. Kinds the importer knows become scenes;
   `light` takes `energy`, `range`, `color`; `secret` and `message` take `size` (and `text`).
   Any other kind becomes a `Marker3D` in the group `ent_<kind>` with its extras as metadata.
7. **Assert.** `detailing.assert_no_zfighting(name, L.rooms, L.boxes, frame_boxes)`. It exits
   with every coplanar overlapping pair named. Fix the geometry, never the check.
8. **Save and export.** `add_world_uv` on the shell and details, save the `.blend`, then
   `export_level(glb)`. Run:
   `blender -b --factory-startup -P tools/blender/build_<level>.py`
9. **Look at it.** Render the exported glb (below), read both stills, and fix what they show
   before any Godot work. The render catches what the checks can't.
10. **Godot.** Import presets (`python3 tools/godot/import_presets.py`), a scene generator
    like `gen_level_blender.py`, then the bake (README, "Rebuilding everything from scripts";
    about 15 minutes a level, needs Xvfb and lavapipe). Details:
    [references/godot-export.md](references/godot-export.md).
11. **Capture and record** (CLAUDE.md 9): stills or a video from `BRUSHFIRE_AUTOTEST`, into
    `docs/screenshots/<topic>/`, and a validation record that says what was proven.

## Render a capture from the glb

```
BLENDER_LEVEL_KIT=tools/blender blender -b --factory-startup \
    -P .claude/skills/blender-csg-levels/scripts/render_level.py -- <level.glb> <out_dir> [samples]
```

It writes `eye.png` (from `ENT_player_start` at 1.6 m, facing its yaw) and `top.png` (an
orthographic plan; faces seen from behind are see-through, so ceilings vanish). Cycles on the
CPU: it runs in a cloud session without a GPU, in about a minute at 32 samples. Door frames
and other entity props are not in the glb, so openings look bare here; Godot instances them.

## Brush rules that are easy to get wrong

These come from bugs that happened. The numbers are in
[references/brush-cookbook.md](references/brush-cookbook.md).

- **A vault cutter is a whole cylinder, and its lower half carves too.** Keep
  `centre_y - radius` above the floor and the cylinder inside the room's own air box, or it
  digs a trough through the floor. A 16 m hall is not one r 8 vault on 6 m walls: use 4 m
  groin bays (two cylinders each way), as the example and the Cistern do.
- **Nothing coincides.** Cutters overlap their neighbours by 0.1 m. A vault is 0.02 m shorter
  than its room. A pit's top sits at floor + 0.01. A radius 0.01 under the half span keeps the
  cylinder off the wall planes where it needn't be tangent.
- **The shell encloses everything with margin.** A cutter that reaches the shell's bounding box
  opens the level to the void, and the export's outside cull then deletes real faces.
- **The last stair tread is the floor itself.** No zero-height blocks.
- **Door slabs are the frame's `fits` size** (`detailing.FRAMES`, doors 3.2 x 3.3 m); the frame
  prop's clear opening is 0.1 m smaller, so its reveals stand proud (CLAUDE.md 7.2).
- **Taken heights:** baseboards are 0.30, 0.35 and 0.45 m tall. A plinth or dais that meets a
  baseboard picks another height (arch plinths use 0.55 m).
- **Pillars have a base and a capital; every light has a fixture; ceiling lights go in the
  bays between girders; wall lamps go between pilasters, not on them** (CLAUDE.md 7.3).
- **Material names are keys of `materials.json`.** The export strips Blender's `.001`
  suffixes; any other name gets the 2 m default tile and no Godot material.
- **Non-box geometry is not checked** (columns, arch rings, vaults). Look at it up close in the
  render, and in Godot with AutoTest `{"debug_draw": 1}`.

## Checks before calling a level done

| Check | How |
|---|---|
| Z-fighting | the build's own `assert_no_zfighting` passes (it refuses to write otherwise) |
| People placement | city levels: `python3 tools/levels/city_plan.py <id> --stats`, then `placement_test.tscn` (CLAUDE.md 7.4) |
| The glb looks right | `render_level.py` stills, read by you before anyone else |
| Dashes | CLAUDE.md 4 grep |
| In game | bake, then an AutoTest capture per requirement; say it's lavapipe (no frame times) |

## When it goes wrong

| You see | Cause | Fix |
|---|---|---|
| A curved trough in a floor | a vault cylinder dips below the floor | lower the radius or raise the springing (rule 1) |
| Sky or void through a wall | a cutter reaches the shell's bounds | grow the shell |
| Missing faces after export | the outside cull removed faces on the shell's bounding planes | keep every carved face off the shell's outer planes |
| Boolean leaves slivers or fails | coincident cutter faces | overlap by 0.1 m or shorten by 0.02 m |
| Flicker in game | coplanar faces the checker can't see (cylinders, props) | move one surface 1 cm or more |
| Texture scale wrong | material name not in `materials.json`, or a `.001` name | use the manifest's names |
| An entity missing in Godot | its kind isn't in the importer's maps | add the scene, or read the `ent_<kind>` marker in game code |
| Black shadows | no zone ambient | `level_common.add_zone_ambient` over the rooms |
| Old door meshes after a prop change | Godot keeps saved `.res` meshes | delete `models/doorway/leaf_*.res`, reimport (CLAUDE.md 11) |

## Another project

The kit travels. From this repository:

```
python3 .claude/skills/blender-csg-levels/scripts/copy_kit.py .claude/skills/blender-csg-levels/kit.json <target_root>
```

It copies the files in [kit.json](kit.json) to the same paths in the target, with this skill,
and writes `KIT_PROVENANCE_blender-csg-levels.md` (source, revision, date, files, and the list
of things to adapt). `--check` proves the list still matches this repository. The adapt list
is in [references/porting.md](references/porting.md).

## References

- [references/brush-cookbook.md](references/brush-cookbook.md): every construct with its numbers
- [references/godot-export.md](references/godot-export.md): the glb contract, import, scene, bake
- [references/city-scale.md](references/city-scale.md): layout to plan to sector glbs, for hubs
- [references/porting.md](references/porting.md): starting a level pipeline in a new project
- `docs/ut99_reference.md`: the art direction the trims implement
