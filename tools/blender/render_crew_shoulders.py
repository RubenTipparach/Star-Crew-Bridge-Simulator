"""Render shoulder edge flow from saved rigs and stress poses from exported GLBs.

Only review scene copies are cropped or given wire overlays. Authoring quads
come from the saved Blender source; deformation comes from the game artifact.
"""
import argparse
import json
from pathlib import Path
import sys

import bpy
import bmesh
from mathutils import Quaternion,Vector
import math

sys.path.insert(0,str(Path(__file__).resolve().parent))
import render_crew_characters as render
from render_crew_rigs import posed_mesh
from check_crew_rigs import load_shoulder_review
from crew_rig import reset_pose
from render_props import camera,plain
from build_crew_characters import ROOT,OUTPUT


def source_tunic(path,row,offset,yaw,wire):
    """Display actual authoring quad edges on a copy of the saved tunic."""
    with bpy.data.libraries.load(str(path),link=False) as (source,target):
        target.objects = [row["id"]]
    ob = target.objects[0]
    arm = ob.parent
    for item in (arm,ob):
        bpy.context.scene.collection.objects.link(item)
    for track in arm.animation_data.nla_tracks:
        track.mute = True
    reset_pose(arm)
    ob.data = ob.data.copy()
    bm = bmesh.new()
    bm.from_mesh(ob.data)
    uv = bm.loops.layers.uv.active
    kept = {v for face in bm.faces if round(face.loops[0][uv].uv.x)==row["parts"]["garment_tunic"] for v in face.verts}
    bmesh.ops.delete(bm,geom=[v for v in bm.verts if v not in kept],context="VERTS")
    bm.to_mesh(ob.data)
    bm.free()
    render.material(ob)
    if wire:
        ob.data.materials.append(plain("authoring_edges",(.006,.012,.018)))
        modifier = ob.modifiers.new("Authoring quad edges","WIREFRAME")
        modifier.thickness = .0012
        modifier.use_replace = False
        modifier.material_offset = 1
    arm.rotation_mode = "QUATERNION"
    arm.rotation_quaternion = Quaternion((0,0,1),math.radians(yaw)) @ arm.rotation_quaternion
    arm.location += Vector(offset)
    bpy.context.view_layer.update()
    return ob


def main():
    """Capture two body variants with edge flow and the same six stress poses."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--models",type=Path,default=OUTPUT)
    parser.add_argument("--shots",type=Path,default=ROOT/"docs/screenshots/crew-characters")
    parser.add_argument("--before",type=Path,help="optional earlier measured package for a comparison")
    parser.add_argument("--samples",type=int,default=16)
    args = parser.parse_args(sys.argv[sys.argv.index("--")+1:] if "--" in sys.argv else [])
    args.models,args.shots = args.models.resolve(),args.shots.resolve()
    args.shots.mkdir(parents=True,exist_ok=True)
    rows = json.loads((args.models/"characters.json").read_text())["characters"]
    poses = load_shoulder_review()["poses"]
    for sex in ("male","female"):
        row = next(r for r in rows if r["sex"]==sex)
        render.scene(args.samples)
        objects = []
        for i,(title,yaw,wire) in enumerate((("Front",0,False),("Front edges",0,True),("Back edges",180,True),("Side edges",-45,True))):
            x = (i-1.5)*1.1
            objects.append(source_tunic(args.models/"blender"/f"{row['id']}.blend",row,(x,0,0),yaw,wire))
            render.text(title,x,1.42,.065)
        camera(objects,(2000,660),margin=1.3,view=Vector((0,-1,0)))
        render.capture(args.shots/f"shoulder-topology-{sex}.png")
        render.scene(args.samples)
        objects = []
        for i,pose in enumerate(poses):
            x = (i-2.5)*2.05
            objects.append(posed_mesh(args.models/row["file"],None,0,(x,0,0),0,pose=pose))
            render.text(pose["name"],x,2.38,.09)
        bpy.context.view_layer.update()
        camera(objects,(2400,1000),margin=1.24,view=Vector((0,-1,.015)).normalized())
        render.capture(args.shots/f"shoulder-poses-{sex}.png")
    if args.before:
        render.scene(args.samples)
        objects = []
        for i,(sex,folder,title) in enumerate((("male",args.before,"Male before"),("male",args.models,"Male revised"),("female",args.before,"Female before"),("female",args.models,"Female revised"))):
            row = next(r for r in rows if r["sex"]==sex)
            render.MODEL_ROOT = folder.resolve()
            x = (i-1.5)*1.3
            objects += render.import_character(row,(x,0,0))
            render.text(title,x,2.07)
        camera(objects,(2000,1000),margin=1.23,view=Vector((0,-1,.025)).normalized())
        render.capture(args.shots/"shoulder-comparison.png")


if __name__=="__main__":
    main()
