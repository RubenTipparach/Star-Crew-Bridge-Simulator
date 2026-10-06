# City-scale levels: layout, plan, sector glbs

A hub or a mission map is too big and too irregular for one script of hand-placed boxes. The
Undercity hub splits the job in three, so the design map and the level can never disagree
(CLAUDE.md 7.1), and the same CSG ideas (solids minus cutters) still build every building.

```
tools/levels/layouts/<id>.py          the one layout source: districts, blocks, buildings, streets,
  (+ <id>_entities.py)                rooms, POIs, patrols, mission markers, entities
        |                                        \
tools/levels/render_map.py             tools/levels/city_plan.py   (system python3 + shapely)
  the Deus Ex style design map           the build plan as JSON: sectors, 40 m chunks, primitives,
  (docs/design/maps/)                    lights, entities; every check runs here
                                                 |
                                       tools/blender/build_undercity.py   (Blender's python)
                                         meshes the plan, one glb per sector
                                         game/levels/undercity/<id>/<id>_<sector>.glb
```

Run:

```
python3 tools/levels/city_plan.py hub --stats                       # the plan and its checks
blender -b --factory-startup -P tools/blender/build_undercity.py -- hub [--sector lantern_row]
python3 tools/godot/import_presets.py undercity
```

Set `UNDERCITY_PYTHON` when the `python3` with shapely isn't the one on `PATH`.

## Why the split

- **Blender's Python has no shapely.** Polygon unions, buffers, offsets and triangulation with
  holes happen in `city_plan.py`, in the system Python, where they are also unit-testable
  (`tools/levels/test_city_plan.py`).
- **The map and the level share functions.** `city_plan.py` calls `render_map.py`'s own shape
  functions for lots, pillars and footprints, so the drawn map is the built level.
- **Checks run before any mesh exists.** The plan refuses a layout with z-fighting, uncarved
  rooms, water without a way out, or a person inside a solid (below), in seconds instead of a
  Blender run.

## The plan's primitives

The mesher knows seven primitives (full schema in `city_plan.py`'s docstring). Coordinates are
layout metres: x east, y south, z up; Blender takes `(x, -y, z)`.

| Prim | Is | Used for |
|---|---|---|
| `prism` | polygon rings (with holes) extruded z0..z1, per-edge side heights, optional caps, `flip` for pits | ground, sidewalks, canals, roofs |
| `hexa` | 8 corners, 6 face materials, skippable faces | walls, beams, stairs, any sheared box |
| `cyl` | cylinder on any axis | pillars, pipes, lamp posts |
| `quad` | one face | signs, decals |
| `loft` | rings at several heights | tapered towers |
| `text` | extruded font | neon signs |
| `bool` | a prism solid minus `hexa` cutters, Exact, material Transfer; faces inside an interior box route to that interior's object | buildings with windows, doors and enterable rooms |

`bool` is the CSG step at city scale: a building is a solid block with its windows, doors and
rooms carved out, exactly like the Cistern's shell. Carved faces take the cutter's materials.

## Sectors, chunks and objects

- The plan groups geometry by **sector** (a district or scenery band), one glb each, so a
  sector can be rebuilt and rebaked alone.
- Inside a sector, geometry is split into **40 m chunks** so Godot can frustum-cull it.
- Object names carry collision and role:

| Name | Holds |
|---|---|
| `<sector>_walk_<i>_<j>-col` | walkable and blocking geometry (trimesh collision) |
| `<sector>_vis_<i>_<j>` | facades and details, no collision |
| `<sector>_hull_<i>_<j>-colonly` | simplified building collision |
| `interior_<building>-col` | an enterable interior; extras set visibility range and 0.15 m lightmap texels |
| `prop_<kind>_<n>-col` | a small prop, with a visibility range |
| `corona_<n>` | a light's corona: no GI, no shadow |
| `ENT_<kind>_<id>` | entity empties with extras |

- Exteriors bake at 0.4 m lightmap texels, interiors at 0.15 m (through the
  `lightmap_texel_scale` extra); skyline scenery bakes nothing.

## People stand clear of the level (CLAUDE.md 7.4)

Every NPC, civilian, patrol stop and spawn is tested with the body's own collider, read from the
scene file (`npc.tscn`'s capsule, `player.tscn`'s cylinder), never a typed size.

- `city_plan.py` refuses a person inside a detail, stall, pillar, fixture, prop footprint, wall
  or building, on a curb edge, or a patrol leg through any of those.
- A solid that isn't an axis-aligned box registers its footprint with `Plan.solid(...)` where
  it is built; otherwise the check can't see it.
- After a rebuild, `placement_test.tscn` checks every placement against the real colliders.

## Adding to a city level

1. Change the layout module (and its entities module), never the plan or the glb.
2. `python3 tools/levels/render_map.py && python3 tools/design/build_page.py`: the map first.
3. `python3 tools/levels/city_plan.py <id> --stats`: the checks.
4. Build the touched sector, render it (`render_level.py` works on a sector glb), then the
   import presets, the bake and the placement test.
5. New construction rules (a kind of building, a railing rule) go into `city_plan.py`'s city
   kit as generic rules with named constants in metres; level-specific numbers stay in the
   layout.
