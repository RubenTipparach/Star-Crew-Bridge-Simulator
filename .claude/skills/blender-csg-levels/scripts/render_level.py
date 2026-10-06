"""Stills of a level glb, for the capture that ends every level step (CLAUDE.md 9).

It owns nothing the game loads. It imports the exported glb, the file Godot will import, not
the .blend (CLAUDE.md 5.6, validate the real artifact), so the pictures show the booleans as
applied, the outside culled, the UVs as projected and the entity empties as exported. It
writes two stills:
  eye.png  from the ENT_player_start empty at eye height, looking the way it faces
  top.png  an orthographic plan from above; faces seen from behind are see-through, so the
           ceilings vanish and the rooms read as a cutaway
Materials come from blendkit.material() (the game's Material Maker textures) when the level
kit is found, else flat grey. Lights stand in for the lamp entities (warm), ENT_light (its own
colour and energy) and a dim blue world, the palette CLAUDE.md 7.3 asks for. Cycles on the CPU,
so it runs headless without a GPU.

Run:
  BLENDER_LEVEL_KIT=tools/blender blender -b --factory-startup \
      -P .claude/skills/blender-csg-levels/scripts/render_level.py -- <level.glb> <out_dir> [samples]
"""
import math
import os
import sys

import bpy
from mathutils import Vector

KIT = os.environ.get("BLENDER_LEVEL_KIT")
EYE_M = 1.6            # eye height above the player start, metres
LAMP_W = 150.0         # Blender watts per unit of the game's light energy (as blendkit's previews)
LAMP_KINDS = {"wall_lamp": (1.0, 0.8, 0.6), "ceiling_light": (1.0, 0.85, 0.65)}


def kit_material(name):
    if not KIT:
        return None
    sys.path.insert(0, os.path.abspath(KIT))
    try:
        from blendkit import material
    except Exception as e:  # a kit without materials.json still renders, in grey
        print(f"[render] no kit materials ({e}); rendering flat")
        return None
    return material(name)


def see_through_backs(mat):
    """Camera rays that hit a face from behind pass through it (Cycles has no backface culling)."""
    nt = mat.node_tree
    out = next(n for n in nt.nodes if n.type == "OUTPUT_MATERIAL")
    link = next((lk for lk in nt.links if lk.to_node == out and lk.to_socket.name == "Surface"), None)
    if link is None:
        return
    surf = link.from_socket
    geo = nt.nodes.new("ShaderNodeNewGeometry")
    ray = nt.nodes.new("ShaderNodeLightPath")
    both = nt.nodes.new("ShaderNodeMath")
    both.operation = "MULTIPLY"
    nt.links.new(geo.outputs["Backfacing"], both.inputs[0])
    nt.links.new(ray.outputs["Is Camera Ray"], both.inputs[1])
    mix = nt.nodes.new("ShaderNodeMixShader")
    clear = nt.nodes.new("ShaderNodeBsdfTransparent")
    nt.links.new(both.outputs[0], mix.inputs["Fac"])
    nt.links.new(surf, mix.inputs[1])
    nt.links.new(clear.outputs[0], mix.inputs[2])
    nt.links.new(mix.outputs[0], out.inputs["Surface"])


def setup(glb, samples):
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.import_scene.gltf(filepath=glb)
    sc = bpy.context.scene
    sc.render.engine = "CYCLES"
    sc.cycles.device = "CPU"
    sc.cycles.samples = samples
    sc.cycles.use_denoising = True
    sc.render.resolution_x, sc.render.resolution_y = 1280, 720
    sc.view_settings.view_transform = "AgX"
    world = bpy.data.worlds.new("World")
    world.use_nodes = True
    world.node_tree.nodes["Background"].inputs["Color"].default_value = (0.05, 0.08, 0.16, 1.0)
    world.node_tree.nodes["Background"].inputs["Strength"].default_value = 0.6
    sc.world = world
    for ob in list(bpy.data.objects):
        if ob.type != "MESH":
            continue
        for slot in ob.material_slots:
            if slot.material is None:
                continue
            name = slot.material.name.split(".")[0]
            slot.material.name = name + "_gltf"
            mat = kit_material(name)
            if mat is not None:
                slot.material = mat
    for ob in bpy.data.objects:
        if ob.type != "EMPTY" or not ob.name.startswith("ENT_"):
            continue
        kind = ob.name[4:].rsplit("_", 1)[0]
        if kind in LAMP_KINDS or kind == "light":
            ld = bpy.data.lights.new(ob.name + "_lamp", "POINT")
            ld.color = tuple(ob.get("color", LAMP_KINDS.get(kind, (1.0, 0.85, 0.65))))[:3]
            ld.energy = float(ob.get("energy", 1.5)) * LAMP_W
            ld.shadow_soft_size = 0.2
            lo = bpy.data.objects.new(ld.name, ld)
            lo.location = ob.matrix_world.translation + (Vector((0, 0, -0.4)) if kind == "ceiling_light" else Vector())
            sc.collection.objects.link(lo)


def level_bounds():
    pts = [ob.matrix_world @ Vector(c) for ob in bpy.data.objects if ob.type == "MESH" for c in ob.bound_box]
    lo = Vector((min(p.x for p in pts), min(p.y for p in pts), min(p.z for p in pts)))
    hi = Vector((max(p.x for p in pts), max(p.y for p in pts), max(p.z for p in pts)))
    return lo, hi


def camera(name, loc, rot, ortho=None):
    cd = bpy.data.cameras.new(name)
    if ortho:
        cd.type = "ORTHO"
        cd.ortho_scale = ortho
    else:
        cd.lens = 18.0
    cd.clip_end = 1000.0
    cam = bpy.data.objects.new(name, cd)
    cam.location, cam.rotation_euler = loc, rot
    bpy.context.scene.collection.objects.link(cam)
    bpy.context.scene.camera = cam
    return cam


def still(path):
    bpy.context.scene.render.filepath = path
    bpy.ops.render.render(write_still=True)
    print("[render] wrote", path)


def main():
    argv = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []
    if len(argv) < 2:
        raise SystemExit("usage: blender -b --factory-startup -P render_level.py -- <level.glb> <out_dir> [samples]")
    glb, out = os.path.abspath(argv[0]), os.path.abspath(argv[1])
    setup(glb, int(argv[2]) if len(argv) > 2 else 48)
    os.makedirs(out, exist_ok=True)
    start = next((ob for ob in bpy.data.objects if ob.name.startswith("ENT_player_start")), None)
    if start is not None:
        yaw = start.matrix_world.to_euler().z   # 0 looks north: Blender +Y, Godot -Z
        camera("Eye", start.matrix_world.translation + Vector((0, 0, EYE_M)), (math.radians(90), 0, yaw))
        still(os.path.join(out, "eye.png"))
    else:
        print("[render] no ENT_player_start: skipping eye.png")
    for mat in bpy.data.materials:
        if mat.node_tree:
            see_through_backs(mat)
    lo, hi = level_bounds()
    c = (lo + hi) / 2
    span = max(hi.x - lo.x, (hi.y - lo.y) * 16 / 9)
    camera("Top", Vector((c.x, c.y, hi.z + 50)), (0, 0, 0), ortho=span * 1.05)
    still(os.path.join(out, "top.png"))


if __name__ == "__main__":
    main()
