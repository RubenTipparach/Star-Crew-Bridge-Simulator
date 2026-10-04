"""A small example level built the "CSG in Blender" way: the starting point for a new level.

It owns nothing in the game. It is the worked example of the blender-csg-levels skill: a
groin-vaulted hall with a column and an oculus with a sky face, a door into a barrel-vaulted
corridor with an arch ring, a flat-ceiled chamber with a sunken pit, stairs and railings, trims from
detailing.py, lights, a player start and the z-fighting check. It imports the level kit
(tools/blender/blendkit.py, the Level class of tools/blender/build_cistern.py and
tools/godot/detailing.py) instead of copying it, so it can't drift from the code the real levels
use. Copy it to tools/blender/build_<level>.py and replace build() to start a new level.

Run (from anywhere; the kit is found beside this file or through BLENDER_LEVEL_KIT):
  BLENDER_LEVEL_KIT=tools/blender blender -b --factory-startup \
      -P .claude/skills/blender-csg-levels/templates/example_level.py -- <out.blend> <out.glb>

Coordinates are Godot metres (x right, y up, -z north); the kit maps them to Blender.
"""
import os
import sys

import bpy

KIT = os.path.abspath(os.environ.get("BLENDER_LEVEL_KIT") or os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, KIT)
from blendkit import add_world_uv, box_bm, export_level, mesh_object  # noqa: E402
from build_cistern import Level  # noqa: E402  (its module sets up detailing's import path)

import detailing  # noqa: E402


def reset(name):
    bpy.ops.wm.read_factory_settings(use_empty=True)
    scene = bpy.context.scene
    scene.name = name
    scene.unit_settings.system = "METRIC"


def build():
    L = Level()
    # 1. The solid the level is carved from. It must enclose every cutter with margin: a vault
    #    crown or an oculus that reaches the shell's bounding box opens a hole to the void.
    shell = mesh_object("Shell", box_bm((-20, -4, -36), (20, 12, 12), {"side": 0, "bottom": 0, "top": 0}),
                        bpy.context.scene.collection, ["stone_blocks"])

    # 2. Cutters. Each room is an air box: its bottom face becomes the floor, its sides the walls,
    #    its top the ceiling. Neighbouring cutters overlap by 0.1 m so no two boolean faces coincide.
    L.room("Hall", (-8, 0, -8), (8, 5, 8), "stone_blocks", "floor_tiles", "brick_wall")
    # four groin bays: two barrel vaults each way, sprung from the wall tops. A vault cutter is a
    # whole cylinder, so its lower half carves too: keep centre - radius above the floor and the
    # cylinder inside the room's own air box. A length 0.02 m under the room's keeps the end caps
    # off the wall planes.
    for i, c in enumerate((-4, 4)):
        L.vault(f"HallVaultNS{i}", (c, 5, 0), 4.0, 15.98, "z", "brick_wall", "brick_wall")
        L.vault(f"HallVaultEW{i}", (0, 5, c), 4.0, 15.98, "x", "brick_wall", "brick_wall")
    # an oculus through one bay's crown (y 9), capped by a sky face (the cutter's caps carry sky_moon)
    L.vault("Oculus", (4, 9.5, 4), 1.0, 2.0, "y", "stone_blocks", "sky_moon")
    # the door slab: exactly the door frame's `fits` opening (detailing.FRAMES), 0.1 m into each side
    w, h = detailing.FRAMES["doorway"]["fits"]
    L.room("DoorN", (-w / 2, 0, -8.5), (w / 2, h, -7.9), "tech_panel", "diamond_plate", "tech_panel", trims=False)
    L.room("Corridor", (-2, 0, -20), (2, 3, -8.4), "brick_wall", "diamond_plate", "brick_wall")
    L.vault("CorridorVault", (0, 3, -14.2), 2.0, 11.56, "z", "brick_wall", "brick_wall")
    L.room("Chamber", (-6, 0, -32), (6, 5, -19.9), "stone_blocks", "floor_tiles", "ceiling_tiles")
    # a sunken pit: its top sits 0.01 m above the chamber floor so the two floors never coincide
    L.room("Pit", (-3, -2, -30), (3, 0.01, -24), "stone_blocks", "concrete", "concrete", trims=False)

    mod = shell.modifiers.new("Carve", "BOOLEAN")
    mod.operation = "DIFFERENCE"
    mod.operand_type = "COLLECTION"
    mod.collection = L.carve
    mod.solver = "EXACT"
    mod.material_mode = "TRANSFER"
    mod.use_hole_tolerant = True

    # 3. Detail: additive brushes. Every box goes through L.block so the z-fighting check sees it.
    # the column where the four bays meet, its capital rising past the springing into the ridges
    L.column(0, 0, 0, 5.2, 0.5, "stone_blocks", "tech_panel")   # base and capital, always
    # pit stairs on the south side: 5 risers of 0.4 m; the 5th tread is the pit floor itself
    for k in range(4):
        z1 = -24 - 0.5 * k
        L.block((-1.5, -2, z1 - 0.5), (1.5, -0.4 * (k + 1), z1), "diamond_plate")
    # railings round the pit, open at the stairs; rails lap at the corners without sharing a face
    rail = "rust_metal"
    L.block((-3, 0, -30.25), (3, 1.0, -30.0), rail)
    L.block((-3.25, 0, -30.25), (-3, 1.0, -23.75), rail)
    L.block((3, 0, -30.25), (3.25, 1.0, -23.75), rail)
    L.block((-3, 0, -24), (-1.5, 1.0, -23.75), rail)
    L.block((1.5, 0, -24), (3, 1.0, -23.75), rail)
    L.block((-5.6, 0, -31.6), (-4.1, 1.5, -30.1), "crate_large")
    # 4. Trims from the room list: baseboards, cornices, pilasters with plinths and capitals
    for lo, hi, mat in detailing.all_trims(L.rooms):
        L.block(lo, hi, mat, skip=()).modifiers.new("Bevel", "BEVEL").width = 0.02
    # stone arch rings where the round corridor meets the chamber (not boxes: check them by eye)
    L.arch_ring(0, 3.0, 2.0, "x", -19.9, -1)

    # 5. Entities: empties named ENT_<kind>_<n>, custom properties become glTF extras
    E = L.ent
    E("player_start", (-4, 0.05, 7), -15)
    frames = [("doorway", (0, 0, -8.2), 0)]
    for kind, p, yaw in frames:
        E(kind, p, yaw)
    for z in (-5.33, 5.33):   # in the bays between the pilasters, never on one
        E("wall_lamp", (-7.95, 3.2, z), -90)
        E("wall_lamp", (7.95, 3.2, z), 90)
    for z in (-12, -17):
        E("wall_lamp", (-1.95, 2.4, z), -90)
        E("wall_lamp", (1.95, 2.4, z), 90)
    E("ceiling_light", (0, 4.9, -27))
    E("light", (0, -1.0, -27), energy=1.2, range=7.0, color=(0.45, 0.8, 1.0))
    E("health", (4.5, 0, -22))

    # 6. No z-fighting: refuse to write a level with coplanar overlapping faces (CLAUDE.md 7.2)
    detailing.assert_no_zfighting("Example", L.rooms, L.boxes,
                                  [(lo, hi, f"{k}@{p}#{i}") for k, p, yaw in frames
                                   for i, (lo, hi) in enumerate(detailing.frame_solids(k, p, yaw))])
    return shell


def main():
    argv = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []
    if len(argv) != 2:
        raise SystemExit("usage: blender -b --factory-startup -P example_level.py -- <out.blend> <out.glb>")
    blend, glb = (os.path.abspath(a) for a in argv)
    reset("Example")
    shell = build()
    for ob in [shell] + list(bpy.data.collections["Detail"].objects):
        add_world_uv(ob)   # live world-aligned UVs, so the .blend previews at the game's texel density
    os.makedirs(os.path.dirname(blend), exist_ok=True)
    bpy.ops.wm.save_as_mainfile(filepath=blend, relative_remap=True, compress=True)
    print("[example] saved", blend)
    export_level(glb)


if __name__ == "__main__":
    main()
