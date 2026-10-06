# Brush cookbook

Every construct the Blender levels use, with the numbers that are known to pass the booleans
and the z-fighting check. Coordinates are Godot metres (x right, y up, -z north); `L` is the
`Level` of `tools/blender/build_cistern.py`. The sources are `build_cistern.py` (E1M3) and
the skill's `templates/example_level.py`.

## Axes

| Space | x | y | z |
|---|---|---|---|
| Godot, the build scripts | right (east) | up | south (so -z is north) |
| Blender | right | north | up |
| City layouts (`tools/levels/layouts`) | east | south | up |

`blendkit.B(x, y, z)` maps Godot to Blender as `(x, -z, y)`; the glTF export (`export_yup`)
maps it back. An entity's `yaw` is degrees about up; 0 faces north (Godot -z), and positive
turns toward the west (counter-clockwise from above).

## The shell

```python
shell = mesh_object("Shell", box_bm(lo, hi, {"side": 0, "bottom": 0, "top": 0}),
                    bpy.context.scene.collection, ["stone_blocks"])
```

- One box, at least 2 m past every cutter on every side, including vault crowns and oculi.
- Its material shows only where a cutter doesn't transfer one; give it the level's main wall.
- Its outside is culled at export (faces lying on the bounding box's six planes). Carved faces
  must never lie on those planes.

## Rooms (box cutters)

```python
L.room("Hall", (-8, 0, -8), (8, 5, 8), wall, floor, ceiling, trims=True)
```

- Materials by face: sides = `wall`, bottom = `floor`, top = `ceiling`.
- Registers a `detailing.Room` for trims and the check. Style comes from the wall material
  (`STYLE` in `build_cistern.py`: tech, brick or stone). `beams=False` always, because vaulted
  rooms don't take girders; a flat-ceiled level that wants girders builds `detailing.Room`
  itself with `beams=True` and `ceiling=<material>`.
- `trims=False` for door slabs, arch openings, pits, crawlspaces and other non-rooms.
- Room heights: corridors 3 m (no cornice under 3 m; pilasters only at 3 m and over), halls
  5-7 m, chambers 5-6 m. Pilaster spacing is per style: tech 4 m, stone 5 m, brick 3.5 m.

## Openings

| Opening | Cutter | Numbers |
|---|---|---|
| Door | thin room, `trims=False` | `detailing.FRAMES["doorway"]["fits"]` = 3.2 x 3.3 m, 0.6 m deep, 0.1 m into each room; `ENT_doorway` at the slab's centre, yaw along the passage |
| Archway prop | thin room | `archway_400x400` fits 4.0 x 4.0, `archway_400x350` fits 4.0 x 3.5 |
| Round arch | thin room + vault on top | room 4 m wide to the springing (3.2 m), vault r 2.0, length = the slab's depth (0.8 m) along the wall's normal |
| Tunnel mouth | the tunnel room itself runs 0.1 m into the room | add `L.arch_ring` on each side |

`L.arch_ring(centre, y_spring, radius, span, face, out, y_floor=0.0)`: `span` is the axis the
opening spans (`"x"` or `"z"`), `face` the wall plane's coordinate on the other horizontal axis,
`out` +1 or -1, the side the ring stands proud on. It builds 9 bevelled voussoirs (the
keystone proud and in `tech_panel`), two jambs and two 0.55 m plinths.

## Vaults (cylinder cutters)

```python
L.vault(name, centre, radius, length, axis, surface, ends)   # axis in "x", "y", "z"
```

- **The whole cylinder carves.** Its lower half must lie inside the room it roofs: keep
  `centre_y - radius` at or above the floor, and `|centre_x| + radius` (or z) within the room.
- Barrel vault over a room: centre at the wall top (the springing), radius = half the span,
  length = the room's length less 0.02 m. The Cistern's entry: room 10 m wide, walls 5.2 m,
  `vault((0, 5.2, 45), 4.99, 9.98, "z")`.
- Tunnel: 4 m wide, walls 3 m, `vault(..., 2.0, length - 0.04, "z")`. Radius equal to the half
  span is tangent to the walls at the springing; the Exact solver handles it.
- Groin vault: two barrels crossing at right angles, same radius and springing. A big hall is
  several bays: Cistern rooms 36 x 38 m under r 4.0 vaults at x = -8, 0, 8 and z = 1, 9, 17;
  columns stand where the vaults meet, capitals 0.2 m past the springing.
- Oculus: a vertical cylinder (`axis "y"`) whose bottom dips 0.5 m into the vault crown and
  whose top stays under the shell. Its end caps carry a `sky_*` material: that face becomes
  the sky window in Godot. Aim the level's moonlight spot from the sky's `moon_direction`.

## Pits and changes of level

- A pit is a room whose top is floor + 0.01 (`(-11, -3, -2), (11, 0.01, 20)`), `trims=False`,
  so its top never coincides with the floor it opens from.
- Stairs are blocks inside the pit, each from the pit floor to its tread: `n` risers means
  `n - 1` blocks, because the last tread is the pit floor. Cistern: 8 risers of 0.375 m.
  Keep risers 0.2-0.4 m: the player steps up at most `MaxStepHeight`, 0.45 m in
  `PlayerController.cs`.
- Raised galleries: a room with `y0 > 0`, stairs of `L.block` up to a landing block, and a rail
  that stops short of the arch jamb.

## Detail brushes

| Brush | Call | Notes |
|---|---|---|
| Box | `L.block(lo, hi, mat, skip=("bottom",))` | registered for the check; skip faces nobody sees |
| Column | `L.column(x, z, y0, y1, r, mat, cap_mat)` | 16 sides, square plinth 0.5 m and capital 0.4 m at 1.3 r |
| Pipe | `L.pipe(centre, r, length, axis, mat)` | 12 sides, open ends: run them into walls |
| Railing | boxes 0.25 m thick, 1.0 m tall | lap at corners: one rail owns the corner, the other stops at its face |
| Crate | `L.block` with `crate` (1 m) or `crate_large` (2 m) | keep 0.1 m off trims and rails, or overlap them fully |

## Trims (`detailing.py`)

`detailing.all_trims(rooms, airs=None, avoid=())` returns `(lo, hi, material)` boxes:

| Piece | tech | stone | brick |
|---|---|---|---|
| Baseboard (h, depth) | 0.35, 0.10 | 0.45, 0.14 | 0.30, 0.08 |
| Cornice (rooms over 3 m) | 0.30, 0.18 | 0.35, 0.22 | 0.25, 0.14 |
| Hazard band | 0.12 at 1.15 m | none | none |
| Pilaster (w, depth, spacing) | 0.6, 0.16, 4.0 | 0.8, 0.22, 5.0 | 0.5, 0.14, 3.5 |
| Plinth and capital | +0.08 depth | +0.10 | +0.06 |

- Trims stop at every opening another air volume cuts through the wall (0.15 m margin).
- Inside corners belong to the x-running walls; trims on z-running walls stop at their faces.
- Girders span the short way, lined up with the long walls' pilasters, 0.75 of a pilaster wide,
  never as deep as a cornice or capital. They contrast with the ceiling material.
- `avoid` takes `keep_out(centre, half)` boxes: pilasters and girders touching one are dropped.

## Lights

`L.ent("wall_lamp", pos, yaw)`, `L.ent("ceiling_light", pos)`, and
`L.ent("light", pos, energy=, range=, color=)` for a bare baked omni (a glow in a pit, a fire).

- Wall lamps: 0.05 m off the wall, 2.4-3.2 m up, yaw facing into the room (-90 on a west
  wall, 90 on an east wall, 0 on a south wall, 180 on a north wall). Place them in the bays between pilasters.
- Ceiling lights: 0.1 m under the ceiling or the vault crown, in bays between girders.
- Warm fixtures, a cool blue fill, one accent colour (CLAUDE.md 7.3). The level scene adds the
  blue zone ambient per room from the room list; write the rooms out for it (as
  `write_rooms` does) when the level has trims.

## Materials

Names are keys of `game/materials/materials.json`, each with `tile_m` (metres per repeat).
`project_uvs` box-projects every face at its material's `tile_m`, so all three level tools share
one texel density. Emissive materials use `emission_operator = 1` in Godot (CLAUDE.md 11).
