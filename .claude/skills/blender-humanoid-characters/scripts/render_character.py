"""A turnaround still of character glbs, for the capture that ends every character step (CLAUDE.md 9).

It owns nothing the game loads. It imports the exported glbs, the files Godot will import, not
a .blend (CLAUDE.md 5.6, validate the real artifact), so the picture shows the rig, the
skinning, the atlases and the gear as exported. Each body stands in a row of five: front,
three-quarter, side, back, and front again at the table's stress_pose (the pose the build tests
gear against), so bad skinning and gear that pokes through show at a glance. One row per glb,
labelled with its file name. Warm key, cool blue fill and a rim (CLAUDE.md 7.3). Cycles on the
CPU, so it runs headless without a GPU.

Run:
  blender -b --factory-startup -P .claude/skills/blender-humanoid-characters/scripts/render_character.py -- \
      <out.png> <a.glb> [<b.glb> ...] [--table tools/blender/npcs.json] [--samples 32]
"""
import json
import math
import os
import sys

import bpy
from mathutils import Matrix, Vector

SPACING_M = 1.1    # between the figures in a row
ROW_M = 2.3        # between rows, stacked downward
VIEWS_DEG = (0, 45, 90, 180, 0)   # turn about up for each figure; the fifth is posed


def args():
    argv = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []
    opts = {"--table": None, "--samples": "32"}
    rest = []
    it = iter(argv)
    for a in it:
        if a in opts:
            opts[a] = next(it)
        else:
            rest.append(a)
    if len(rest) < 2:
        raise SystemExit(__doc__)
    return rest[0], rest[1:], opts


def stress_pose(table_path):
    if not table_path:
        return {}
    with open(table_path, encoding="utf-8") as f:
        pose = json.load(f).get("stress_pose", {})
    return {k: v for k, v in pose.items() if not k.startswith("_comment")}


def pose_rig(rig, pose):
    """Rotations about armature axes in each bone's rest frame, as build_npcs.apply_pose does."""
    for name, rot in pose.items():
        pb = rig.pose.bones.get(name)
        if pb is None:
            continue
        rest = pb.bone.matrix_local.to_3x3()
        r = Matrix.Rotation(math.radians(rot["deg"]), 3, rot["axis"])
        pb.rotation_mode = "QUATERNION"
        pb.rotation_quaternion = (rest.inverted() @ r @ rest).to_quaternion()


def place(glb, x, z, turn_deg, pose):
    before = set(bpy.data.objects)
    bpy.ops.import_scene.gltf(filepath=glb)
    new = [o for o in bpy.data.objects if o not in before]
    roots = [o for o in new if o.parent is None]
    for r in roots:
        r.matrix_world = Matrix.Translation((x, 0, z)) @ Matrix.Rotation(math.radians(turn_deg), 4, "Z") @ r.matrix_world
    if pose:
        for o in new:
            if o.type == "ARMATURE":
                pose_rig(o, pose)
    return new


def label(text, x, z):
    cu = bpy.data.curves.new("label", "FONT")
    cu.body = text
    cu.size = 0.12
    cu.align_x = "CENTER"
    ob = bpy.data.objects.new("label", cu)
    ob.location = (x, -0.5, z)
    ob.rotation_euler = (math.radians(90), 0, 0)
    mat = bpy.data.materials.new("label")
    mat.diffuse_color = (0.9, 0.9, 0.9, 1)
    cu.materials.append(mat)
    bpy.context.scene.collection.objects.link(ob)


def light(kind, loc, energy, color, target):
    ld = bpy.data.lights.new(kind, "AREA")
    ld.energy, ld.color, ld.size = energy, color, 3.0
    ob = bpy.data.objects.new(kind, ld)
    ob.location = loc
    ob.rotation_euler = (Vector(target) - Vector(loc)).to_track_quat("-Z", "Y").to_euler()
    bpy.context.scene.collection.objects.link(ob)


def main():
    out, glbs, opts = args()
    pose = stress_pose(opts["--table"])
    bpy.ops.wm.read_factory_settings(use_empty=True)
    sc = bpy.context.scene
    sc.render.engine = "CYCLES"
    sc.cycles.device = "CPU"
    sc.cycles.samples = int(opts["--samples"])
    sc.cycles.use_denoising = True
    sc.view_settings.view_transform = "AgX"
    world = bpy.data.worlds.new("World")
    world.use_nodes = True
    world.node_tree.nodes["Background"].inputs["Color"].default_value = (0.06, 0.07, 0.1, 1)
    sc.world = world
    for row, glb in enumerate(glbs):
        z = -row * ROW_M   # rows stack downward, each on its own floor strip
        for i, turn in enumerate(VIEWS_DEG):
            place(os.path.abspath(glb), (i - 2) * SPACING_M, z, turn, pose if i == 4 else {})
        label(os.path.splitext(os.path.basename(glb))[0] + ("   (last: stress pose)" if pose else ""),
              0, z - 0.25)
        bpy.ops.mesh.primitive_plane_add(size=1, location=(0, 0.3, z))
        bpy.context.active_object.scale = (5 * SPACING_M + 0.6, 1.4, 1)
    width = 5 * SPACING_M + 0.6
    height = len(glbs) * ROW_M + 0.2   # room under the last label
    centre = Vector((0, 0, 2.05 - height / 2))
    light("Key", (-3, -5, centre.z + 3), 1200, (1.0, 0.85, 0.7), centre)
    light("Fill", (4, -4, centre.z + 1), 450, (0.55, 0.7, 1.0), centre)
    light("Rim", (0, 4, centre.z + 3), 700, (1.0, 1.0, 1.0), centre)
    cd = bpy.data.cameras.new("Cam")
    cd.type = "ORTHO"
    cd.ortho_scale = max(width, height)
    cam = bpy.data.objects.new("Cam", cd)
    cam.location = (0, -12, centre.z)
    cam.rotation_euler = (math.radians(90), 0, 0)
    sc.collection.objects.link(cam)
    sc.camera = cam
    sc.render.resolution_x = 1600 if width >= height else int(1600 * width / height)
    sc.render.resolution_y = int(1600 * height / width) if width >= height else 1600
    sc.render.filepath = os.path.abspath(out)
    bpy.ops.render.render(write_still=True)
    print("[render] wrote", out)


if __name__ == "__main__":
    main()
