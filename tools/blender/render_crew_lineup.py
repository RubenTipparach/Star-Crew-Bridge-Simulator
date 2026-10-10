"""The crew bodies' lineup render (openspec/changes/crew-bodies, design section 5): every body of
assets/models/crew/bodies.json side by side on a deck plate, at night under a warm key and a cool fill,
beside a 1.80 m rail (the walk body's height, data/crew/walk.json) and a door frame 2.10 m high, from the
front and from three-quarters. Cycles on the CPU, so it runs in a cloud session.

Run (any Python with the bpy module, 4.5):
  <python with bpy> tools/blender/render_crew_lineup.py OUT_DIR [--only id,id] [--samples N]
"""
import json

import os
import sys

import bpy  # noqa: F401
from mathutils import Vector  # noqa: E402

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
CREW = os.path.join(ROOT, "assets", "models", "crew")
SPACING_M = 0.95
RAIL_M = 1.80
DOOR_M = (1.0, 2.10)


def emissive(name, rgb, strength):
    m = bpy.data.materials.new(name)
    m.use_nodes = True
    b = m.node_tree.nodes["Principled BSDF"]
    b.inputs["Base Color"].default_value = (*rgb, 1)
    b.inputs["Emission Color"].default_value = (*rgb, 1)
    b.inputs["Emission Strength"].default_value = strength
    return m


def plain(name, rgb, rough=0.8):
    m = bpy.data.materials.new(name)
    m.use_nodes = True
    b = m.node_tree.nodes["Principled BSDF"]
    b.inputs["Base Color"].default_value = (*rgb, 1)
    b.inputs["Roughness"].default_value = rough
    return m


def box(name, lo, hi, mat):
    bpy.ops.mesh.primitive_cube_add(size=1.0)
    o = bpy.context.active_object
    o.name = name
    o.scale = [(h - a) for a, h in zip(lo, hi)]
    o.location = [(h + a) / 2 for a, h in zip(lo, hi)]
    o.data.materials.append(mat)
    return o


def main():
    args = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else sys.argv[1:]
    out = args[0]
    only = set(args[args.index("--only") + 1].split(",")) if "--only" in args else None
    samples = int(args[args.index("--samples") + 1]) if "--samples" in args else 48
    os.makedirs(out, exist_ok=True)
    with open(os.path.join(CREW, "bodies.json"), encoding="utf-8") as f:
        ids = [k for k in json.load(f)["bodies"] if not only or k in only]
    bpy.ops.wm.read_factory_settings(use_empty=True)
    scene = bpy.context.scene
    n = len(ids)
    x0 = -(n - 1) * SPACING_M / 2
    for i, bid in enumerate(ids):
        bpy.ops.import_scene.gltf(filepath=os.path.join(CREW, bid + ".glb"))
        for o in bpy.context.selected_objects:
            if o.parent is None:
                o.location.x += x0 + i * SPACING_M
    # The deck, the 1.80 m rail and a door frame for scale.
    deck = plain("deck", (0.16, 0.17, 0.19), 0.6)
    box("deck", (-6, -3, -0.02), (6, 3, 0.0), deck)
    rail = emissive("rail", (0.95, 0.65, 0.2), 2.0)
    left = x0 - SPACING_M * 0.75
    right = x0 + (n - 1) * SPACING_M + SPACING_M * 0.75
    box("rail_180", (left, 0.35, RAIL_M - 0.004), (right, 0.36, RAIL_M + 0.004), rail)
    frame = plain("door", (0.32, 0.33, 0.36), 0.5)
    dx = right + 0.35
    w, h = DOOR_M
    box("jamb_l", (dx, -0.1, 0), (dx + 0.08, 0.1, h), frame)
    box("jamb_r", (dx + 0.08 + w, -0.1, 0), (dx + 0.16 + w, 0.1, h), frame)
    box("lintel", (dx, -0.1, h), (dx + 0.16 + w, 0.1, h + 0.1), frame)
    box("wall", (left - 1, 0.6, 0), (dx + 3, 0.7, 3.0), plain("wall", (0.10, 0.11, 0.13), 0.7))
    # Night: a warm key from a ceiling lamp, a cool fill, a rim from behind (CLAUDE.md 11).
    for name, loc, energy, rgb, size in (("key", (-1.5, -3.0, 3.2), 160, (1.0, 0.86, 0.68), 1.2),
                                          ("fill", (3.0, -2.5, 1.6), 45, (0.62, 0.74, 1.0), 2.0),
                                          ("rim", (0.0, 0.5, 2.8), 90, (0.8, 0.85, 1.0), 1.5)):
        bpy.ops.object.light_add(type='AREA', location=loc)
        L = bpy.context.active_object
        L.name = name
        L.data.energy = energy
        L.data.color = rgb
        L.data.size = size
        L.rotation_euler = (Vector((0, 0, 1.0)) - Vector(loc)).to_track_quat('-Z', 'Y').to_euler()
    scene.world = bpy.data.worlds.new("night")
    scene.world.color = (0.01, 0.012, 0.02)
    scene.render.engine = 'CYCLES'
    scene.cycles.device = 'CPU'
    scene.cycles.samples = samples
    scene.cycles.seed = 7
    scene.render.resolution_x, scene.render.resolution_y = 1600, 800
    scene.view_settings.view_transform = 'Standard'
    cx = (left + dx + 1.2) / 2
    mid = x0 + (n - 1) * SPACING_M / 2
    for name, eye, look, lens in (("front", (cx, -7.0, 1.15), (cx, 0.0, 0.95), 34),
                                  ("three-quarter", (cx - 4.2, -4.8, 1.45), (cx, 0.0, 0.95), 36),
                                  ("heads", (mid - 0.6, -2.4, 1.55), (mid, 0.0, 1.45), 50)):
        bpy.ops.object.camera_add(location=eye)
        cam = bpy.context.active_object
        cam.data.lens = lens
        cam.rotation_euler = (Vector(look) - Vector(eye)).to_track_quat('-Z', 'Y').to_euler()
        scene.camera = cam
        scene.render.filepath = os.path.join(out, f"crew-lineup-{name}.png")
        bpy.ops.render.render(write_still=True)
        print(f"[crew] rendered {scene.render.filepath}", flush=True)



if __name__ == "__main__":
    main()
    sys.stdout.flush()
    os._exit(0)
