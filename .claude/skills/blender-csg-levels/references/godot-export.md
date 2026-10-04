# From Blender to a baked Godot level

What the exported glb holds, how Godot imports it, and the scene and bake around it. The code
is `blendkit.export_level`, `tools/godot/import_presets.py`,
`game/addons/brushfire_tools/blender_level_import.gd`, `tools/godot/level_common.py` and
`gen_level_blender.py`.

## What `export_level(glb)` does

It needs an object named `Shell` (with the Carve boolean) and collections `Detail` and
`Entities`. The `.blend` itself is not modified.

1. Evaluates the shell (booleans applied) and every detail object into new meshes, in world space.
2. Deletes the shell's faces that lie on its bounding box's six planes: the outside nobody sees,
   which would otherwise take lightmap texels.
3. Joins everything into one mesh named `Level-col`.
4. Box-projects UVs per face at each material's `tile_m` from `materials.json`.
5. Renames `stone_blocks.001` style materials back to the clean name.
6. Exports the mesh and the entity empties:

```python
bpy.ops.export_scene.gltf(filepath=glb, export_format="GLB", use_selection=True, export_extras=True,
                          export_image_format="NONE", export_materials="EXPORT", export_yup=True,
                          export_apply=False, export_lights=False, export_cameras=False,
                          export_texcoords=True, export_normals=True, export_tangents=False,
                          export_animations=False)
```

- `export_image_format="NONE"`: the glb carries material names only; Godot maps each name onto
  its own `.tres`. Textures live once, in `game/textures`.
- `export_extras=True`: custom properties on empties and meshes arrive in Godot as the node's
  `extras` metadata.
- No lights: Blender lights in the `Preview` collection are for the viewport only.

## Names decide collision

Godot's import suffixes (`nodes/use_name_suffixes=true`):

| Object name | Godot builds |
|---|---|
| `Level-col` | the mesh plus a trimesh static body |
| `<name>-colonly` | only the collider, no mesh (simplified building hulls in the city) |
| `<name>` | a mesh with no collision (facades, coronas, far detail) |

## The import preset

`import_presets.py` writes each glb's `.import` file (regenerate, don't hand-edit):

| Setting | Value | Why |
|---|---|---|
| `meshes/light_baking` | 2 (static lightmaps) | generates UV2 and marks the mesh for LightmapGI |
| `meshes/lightmap_texel_size` | 0.1 m (E1M3), 0.4 m (city exteriors) | lightmap resolution |
| `_subresources.materials` | every name to `res://materials/<name>.tres` | one material set for all three level tools |
| `import_script/path` | `blender_level_import.gd` | turns empties into entities |
| `meshes/ensure_tangents` | true | normal maps |

Add a new material name to `LEVEL_MATS` (or the folder it's read from) or it imports as a
blank default.

## Entities (`blender_level_import.gd`)

An empty named `ENT_<kind>_<n>` becomes, in this order of lookup:

1. An Undercity entity, when its `kind` extra names one (`npc`, `civ`, `loot`, `door`, `exit`,
   `terminal`, ...): its scene, every extra as metadata, the group `ent_<kind>`.
2. A Brushfire scene from `SCENES` (`player_start`, enemies, pickups, `doorway`, lamps).
3. `light`: a baked `OmniLight3D` from `energy`, `range`, `color`.
4. `secret` or `message`: an `Area3D` trigger sized by `size` (Blender x, y, z), with `text`.
5. Anything else: a `Marker3D` named `<kind>_<id>`, extras as metadata, in `ent_<kind>`.

Meshes may carry render extras: `visibility_range_end_m`, `lightmap_texel_scale`, `gi_mode`,
`cast_shadow`, applied to the `GeometryInstance3D`.

## The level scene

A generator writes the `.tscn` that hosts the glb (`gen_level_blender.py` for E1M3):

```python
s = Scene("LevelBlender", "Node3D")
lc.setup_root(s, "E1M3: The Cistern")
nav = lc.add_navigation(s)
s.instance("Cistern", "res://levels/blender/cistern.glb", nav)
lc.add_environment(s, fog_color="#1a2630", fog_density=0.007)
lc.add_lightmap(s, texel_scale=1.0)
lc.add_fill_lights(s, [...])          # a few big soft fills
lc.add_zone_ambient(s, rooms)         # blue per-room ambient, from the build's room list
lc.add_probes(s, [...])               # LightmapProbe where dynamic things stand
lc.add_reflection_probes(s, [...])    # one box per room
s.save(out)
```

- The glb goes under the navigation region so the navmesh bakes from it.
- Hand-place light probes at head and waist height along every path enemies and pickups use.
- A moonlight `SpotLight3D` (bake mode static) shines through each sky face.

## The bake

```
BRUSHFIRE_BATCH="res://levels/blender/level_blender.tscn:nav,lightmap" godot --editor --path game
```

- Needs a display and Vulkan: Xvfb (`DISPLAY=:99`) and lavapipe in a cloud session. About 15
  minutes a level. Don't regenerate level files or run `dotnet build` during it (CLAUDE.md 11).
- Commit the `.lmbake`, `.exr` and scene files it writes.
- A sectorized level bakes one `.lmbake` per sector; `lightmap@<sector>` bakes one.

## After the bake

- AutoTest captures (`BRUSHFIRE_AUTOTEST=script.json godot --path game`), one per requirement,
  into `docs/screenshots/<topic>/`. `{"debug_draw": 1}` is unshaded: good for checking geometry
  before spending a bake.
- City levels: `placement_test.tscn` checks every person against the built colliders.
- In a cloud session say the capture is lavapipe: it proves the picture, not the frame time.
