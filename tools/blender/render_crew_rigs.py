"""Capture baked crew poses from the actual GLBs using the shared CPU studio."""
import argparse
import json
from pathlib import Path
import sys

import bpy
from mathutils import Quaternion, Vector
import math

sys.path.insert(0,str(Path(__file__).resolve().parent))
from build_crew_characters import OUTPUT, ROOT
from check_crew_rigs import activate_clip
from render_crew_characters import scene, material, text, capture
from render_props import camera


def posed_mesh(path, clip, fraction, offset, yaw):
    """Freeze an imported animation sample for a stable multi-pose review sheet."""
    before = set(bpy.data.objects)
    bpy.ops.import_scene.gltf(filepath=str(path),disable_bone_shape=True)
    imported = [ob for ob in bpy.data.objects if ob not in before]
    armature = next(ob for ob in imported if ob.type == "ARMATURE")
    ob = next(ob for ob in imported if ob.type == "MESH")
    start,end = activate_clip(armature,clip)
    frame = start+(end-start)*fraction
    bpy.context.scene.frame_set(int(frame),subframe=frame % 1)
    evaluated = ob.evaluated_get(bpy.context.evaluated_depsgraph_get())
    mesh = bpy.data.meshes.new_from_object(evaluated,preserve_all_data_layers=True,depsgraph=bpy.context.evaluated_depsgraph_get())
    transform = Quaternion((0,0,1),math.radians(yaw)).to_matrix().to_4x4() @ evaluated.matrix_world
    mesh.transform(transform)
    frozen = bpy.data.objects.new(clip,mesh)
    bpy.context.scene.collection.objects.link(frozen)
    frozen.location = offset
    for item in imported:
        bpy.data.objects.remove(item,do_unlink=True)
    material(frozen)
    return frozen


def main():
    args = sys.argv[sys.argv.index("--")+1:] if "--" in sys.argv else []
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--models",type=Path,default=OUTPUT)
    parser.add_argument("--shots",type=Path,default=ROOT/"docs/screenshots/crew-characters")
    parser.add_argument("--samples",type=int,default=24)
    options = parser.parse_args(args)
    options.models = options.models.resolve()
    options.shots = options.shots.resolve()
    options.shots.mkdir(parents=True,exist_ok=True)
    rows = json.loads((options.models/"characters.json").read_text())["characters"]
    poses = [("Idle_Loop",.25,0),("Walk_Loop",.125,35),("Seated",0,55),("Reach",0,35),("Wave_Loop",.25,0)]
    for sex in ("male","female"):
        row = next(row for row in rows if row["sex"]==sex)
        scene(options.samples)
        objects = []
        for i,(clip,fraction,yaw) in enumerate(poses):
            x = (i-2)*1.4
            objects.append(posed_mesh(options.models/row["file"],clip,fraction,(x,0,0),yaw))
            text(clip.replace("_Loop", ""),x,2.16)
        bpy.context.view_layer.update()
        camera(objects,(2000,960),margin=1.25,view=Vector((0,-1,.07)).normalized())
        capture(options.shots/("rig-poses.png" if sex=="male" else "rig-poses-female.png"))



if __name__ == "__main__":
    main()
