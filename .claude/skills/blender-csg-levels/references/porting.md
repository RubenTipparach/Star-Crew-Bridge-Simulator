# Starting a CSG level pipeline in another project

The pipeline is a handful of files with few dependencies: Blender 4.2 or later (tested on
5.2.2 LTS), Python 3 for the Godot generators, and Godot 4 for the import side. This page is
the order of work in a fresh repository.

## 1. Copy the kit

```
python3 .claude/skills/blender-csg-levels/scripts/copy_kit.py \
    .claude/skills/blender-csg-levels/kit.json /path/to/new-project
```

Files keep their paths (`tools/blender`, `tools/godot`, `game/addons/brushfire_tools`), and the
skill comes along. `KIT_PROVENANCE_blender-csg-levels.md` at the target's root records the
source revision; keep it, and update it when you pull a newer kit.

If the new project's Godot folder isn't `game/`, change `GAME` in `blendkit.py` and the
generators' `ROOT` joins, and the `res://` paths stay as they are.

## 2. Materials

`blendkit.py` reads `game/materials/materials.json` when imported:

```json
{
  "stone_blocks": {"tile_m": 2.0},
  "floor_tiles": {"tile_m": 2.0},
  "brick_wall": {"tile_m": 2.5},
  "sky_moon": {"tile_m": 8.0}
}
```

- `tile_m` is metres per texture repeat; it sets the texel density of every projected face.
- For viewport previews, put `game/textures/<name>.png`, `<name>_orm.png` (occlusion,
  roughness, metallic) and `<name>_normal.png` beside it. Without them materials are plain.
- In Godot, one `res://materials/<name>.tres` per name, listed in `import_presets.py`.

## 3. Trims and frames

`detailing.py` is pure Python and has no project paths. Edit:

- `STYLES`: your material names per style. Keep the rules: baseboard heights distinct from
  each other and from any plinth, girders never as deep as a cornice or capital.
- `FRAMES`: your door and archway props, with `fits` (the level opening) larger than `clear`
  (the prop's own opening) by `REVEAL` per side, and `frame_solids()` to match the props'
  boxes, so the z-fighting check can test them.

## 4. Entities

In `blender_level_import.gd`, replace the `SCENES` map with your scenes and delete the
`UNDERCITY` map (or fill it). An unknown kind still arrives, as a `Marker3D` in the group
`ent_<kind>` carrying its extras, so game code can find it without an importer change.

## 5. The first level

1. Copy `templates/example_level.py` to `tools/blender/build_<level>.py`.
2. Run it: `blender -b --factory-startup -P tools/blender/build_<level>.py -- <out.blend> <out.glb>`.
3. Render it with `scripts/render_level.py` and read both stills.
4. Write the import preset and the level scene generator, then open Godot or run the batch bake.

## 6. Keep the rules with it

The rules this pipeline depends on (no z-fighting, one layout source, generated files are
never hand-edited, captures end every step) are in this repository's `CLAUDE.md` sections 6, 7
and 11. Carry the ones you want into the new project's own rules file: the skill cites them,
it doesn't replace them.
