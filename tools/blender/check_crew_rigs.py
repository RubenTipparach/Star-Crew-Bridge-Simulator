"""Validate actual crew GLB deformation and saved Blender IK controls headlessly.

The imported file supplies bones, vertex weights and animation tracks. Garment
containment reuses the builder's geometric checker at sampled exported poses.
The source-file checks move actual controls and measure the solved end points.
"""
import argparse
import hashlib
import json
import math
from pathlib import Path
import sys

import bpy
from mathutils import Vector

sys.path.insert(0,str(Path(__file__).resolve().parent))
from crew_rig import FINGERS, load_rig, reset_pose


def activate_clip(armature, name):
    """Select the imported glTF animation by its NLA track identity."""
    animation = armature.animation_data
    for track in animation.nla_tracks:
        track.mute = True
    matches = [strip for track in animation.nla_tracks if track.name == name for strip in track.strips]
    if len(matches) != 1:
        raise AssertionError(f"{name}: expected one imported animation track; got {[t.name for t in animation.nla_tracks]}")
    strip = matches[0]
    animation.action = strip.action
    if hasattr(strip,"action_slot") and strip.action_slot:
        animation.action_slot = strip.action_slot
    return strip.action.frame_range[:]


def geometry_parts(ob, parts):
    """Read tagged triangles and posed world positions from the evaluated mesh."""
    evaluated = ob.evaluated_get(bpy.context.evaluated_depsgraph_get())
    mesh = evaluated.data
    mesh.calc_loop_triangles()
    positions = [evaluated.matrix_world @ vertex.co for vertex in mesh.vertices]
    names = {value:name for name,value in parts.items()}
    groups = {name:set() for name in parts}
    faces = {name:[] for name in parts}
    uv = mesh.uv_layers.active
    for triangle in mesh.loop_triangles:
        name = names[round(uv.data[triangle.loops[0]].uv.x)]
        indices = list(triangle.vertices)
        groups[name].update(indices)
        faces[name].append(indices)
    return positions,groups,faces


def validate_weights(ob, armature, parts):
    """All exported vertices have finite normalized influences on the proper rig."""
    influences = 0
    used = set()
    groups = {group.index:group.name for group in ob.vertex_groups}
    for vertex in ob.data.vertices:
        weights = [(groups[item.group],item.weight) for item in vertex.groups if item.weight > 0]
        if not weights or len(weights)>4 or any(not math.isfinite(w) or name not in armature.data.bones for name,w in weights):
            raise AssertionError(f"{ob.name}: invalid weights at vertex {vertex.index}")
        if abs(sum(w for _,w in weights)-1)>1e-5:
            raise AssertionError(f"{ob.name}: weights do not sum to one at {vertex.index}")
        influences = max(influences,len(weights))
        used.update(name for name,_ in weights)
    _,tagged,_ = geometry_parts(ob,parts)
    for side in ("L","R"):
        for finger in FINGERS:
            name = f"{finger}_{side}"
            if name not in used:
                raise AssertionError(f"{ob.name}: finger bone {name} deforms no vertices")
            if any(dict((groups[item.group],item.weight) for item in ob.data.vertices[vi].groups).get(name,0)<.999 for vi in tagged[name]):
                raise AssertionError(f"{ob.name}: {name} geometry does not follow its curl bone")
    if {"pelvis","spine","chest","clavicle_L","clavicle_R"}-used:
        raise AssertionError(f"{ob.name}: incomplete torso or shoulder weights")
    return {"vertices_checked":len(ob.data.vertices),"maximum_influences":influences,"weighted_bones":len(used)}


def validate_exported(path, row, coverage_check):
    """Import an actual GLB and sample its actual clips for garment containment."""
    from build_crew_characters import surface_check
    if hashlib.sha256(path.read_bytes()).hexdigest() != row["sha256"]:
        raise AssertionError(f"{row['id']}: GLB hash differs from the measured manifest")
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.context.scene.render.fps = load_rig()["fps"]
    bpy.ops.import_scene.gltf(filepath=str(path),disable_bone_shape=True)
    meshes = [ob for ob in bpy.context.scene.objects if ob.type=="MESH"]
    arms = [ob for ob in bpy.context.scene.objects if ob.type=="ARMATURE"]
    if len(meshes)!=1 or len(arms)!=1:
        raise AssertionError(f"{row['id']}: expected one imported mesh and armature")
    ob,armature = meshes[0],arms[0]
    if len(armature.data.bones)!=30 or any(b.name.startswith("CTRL_") for b in armature.data.bones):
        raise AssertionError(f"{row['id']}: wrong imported deform rig")
    for side in ("L","R"):
        for chain in (("chest",f"clavicle_{side}",f"upper_arm_{side}",f"forearm_{side}",f"hand_{side}"),("pelvis",f"thigh_{side}",f"shin_{side}",f"foot_{side}")):
            for parent,child in zip(chain,chain[1:]):
                if armature.data.bones[child].parent.name!=parent:
                    raise AssertionError(f"{row['id']}: broken hierarchy {parent}/{child}")
    facts = validate_weights(ob,armature,row["parts"])
    for track in armature.animation_data.nla_tracks:
        track.mute = True
    reset_pose(armature)
    rest_positions,_,_ = geometry_parts(ob,row["parts"])
    tests = []
    for clip in load_rig()["clips"]:
        start,end = activate_clip(armature,clip["name"])
        endpoints=[]
        for fraction in (0,.125,.25,.5,.75,1):
            frame = start+(end-start)*fraction
            bpy.context.scene.frame_set(int(frame),subframe=frame-int(frame))
            bpy.context.view_layer.update()
            positions,groups,faces = geometry_parts(ob,row["parts"])
            samples = coverage_check(f"{row['id']}:{clip['name']}:{fraction:g}",positions,groups,faces)
            surfaces = surface_check(f"{row['id']}:{clip['name']}:{fraction:g}",positions,groups,faces,rest_positions,up_axis=2)
            floor = min(point.z for point in positions)
            if clip["floor"] and any(abs(key["t"]-fraction)<1e-6 for key in clip["keys"]) and abs(floor)>1e-5:
                raise AssertionError(f"{row['id']}:{clip['name']}:{fraction}: keyed boot is off floor by {floor:.6f} m")
            if fraction in (0,1):
                endpoints.append(positions)
            tests.append({"clip":clip["name"],"fraction":fraction,"covered_skin_samples":samples,"surface_checks":surfaces,"lowest_vertex_m":round(floor,6)})
        if clip["loop"] and max((a-b).length for a,b in zip(*endpoints))>1e-5:
            raise AssertionError(f"{row['id']}:{clip['name']}: exported loop does not close")
    return {**facts,"pose_samples":tests,"imported_bones":len(armature.data.bones)}


def validate_controls(path):
    """Open the saved source and prove both IK target and pole control the limb."""
    bpy.ops.wm.open_mainfile(filepath=str(path))
    armature = next(ob for ob in bpy.context.scene.objects if ob.type=="ARMATURE")
    if len(armature.data.bones)!=38 or sum(b.use_deform for b in armature.data.bones)!=30:
        raise AssertionError(f"{path.name}: expected 30 deform bones and 8 controls")
    results=[]
    for side,sign in (("L",1),("R",-1)):
        for limb,end,joint,pole in (("arm","hand","forearm","elbow"),("leg","foot","shin","knee")):
            reset_pose(armature)
            armature.pose.bones["root"][f"ik_{limb}_{side}"]=1.0
            control=armature.pose.bones[f"CTRL_{end}_{side}"]
            matrix=control.matrix.copy()
            matrix.translation+=Vector((-sign*.04,-.055,.055))
            control.matrix=matrix
            armature.update_tag(); bpy.context.view_layer.update()
            tip=armature.pose.bones[f"{joint}_{side}"].tail.copy()
            error=(tip-control.head).length
            if error>.004:
                raise AssertionError(f"{path.name}:{limb}_{side}: IK misses target by {error:.6f} m")
            before=armature.pose.bones[f"{joint}_{side}"].head.copy()
            pole_bone=armature.pose.bones[f"CTRL_{pole}_{side}"]
            matrix=pole_bone.matrix.copy()
            matrix.translation.y*=-1
            pole_bone.matrix=matrix
            armature.update_tag(); bpy.context.view_layer.update()
            moved=(armature.pose.bones[f"{joint}_{side}"].head-before).length
            if moved<.01:
                raise AssertionError(f"{path.name}:{limb}_{side}: pole does not steer the joint")
            results.append({"limb":f"{limb}_{side}","target_error_m":round(error,7),"pole_joint_motion_m":round(moved,7)})
    return {"file":f"blender/{path.name}","sha256":hashlib.sha256(path.read_bytes()).hexdigest(),"authoring_bones":38,"deform_bones":30,"controls":results}


def main():
    """Validate each manifest file and its corresponding editable control rig."""
    from build_crew_characters import coverage_check, OUTPUT, ROOT
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--models",type=Path,default=OUTPUT)
    parser.add_argument("--report",type=Path,default=ROOT/"docs/screenshots/crew-characters/rig-validation.json")
    args=parser.parse_args(sys.argv[sys.argv.index("--")+1:] if "--" in sys.argv else [])
    manifest=json.loads((args.models/"characters.json").read_text())
    records=[]
    for row in manifest["characters"]:
        print(f"[rig check] {row['id']}: imported GLB",flush=True)
        exported=validate_exported(args.models/row["file"],row,coverage_check)
        controls=validate_controls(args.models/"blender"/f"{row['id']}.blend")
        records.append({"id":row["id"],"sha256":row["sha256"],"exported":exported,"source":controls})
        print(f"[rig check] PASS {row['id']}: weights, exported poses, loops and four IK limbs",flush=True)
    args.report.parent.mkdir(parents=True,exist_ok=True)
    args.report.write_text(json.dumps({"schema":"starcrew.crew-rig-checks/1","characters":records,"scope":"Named exported pose samples and saved Blender controls; arbitrary animation poses and engine adoption remain outside this check."},indent=2)+"\n")


if __name__=="__main__":
    main()
