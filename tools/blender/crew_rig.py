"""Shared cartoon crew deformation, Blender controls and baked review actions.

The rig data owns dimensions and poses. This module owns their Blender meaning;
the builder, pose validator and capture tools use the same implementation.
Control bones are authoring helpers, never part of the exported game skin.
"""
import json
import math
from pathlib import Path

import bpy
from mathutils import Euler, Quaternion, Vector

RIG_TABLE = Path(__file__).resolve().parents[2] / "data/crew/rig.json"
FINGERS = ("index", "middle", "ring", "pinky", "thumb")


def load_rig():
    """Reject invalid rig/clip data before creating Blender objects."""
    data = json.loads(RIG_TABLE.read_text(encoding="utf-8"))
    keys = set("schema _doc fps joint_blend_fraction shoulder_blend_fraction shoulder_attachment_weight limb_rings elbow_offset_m knee_offset_m pole_distance_m control_size_m palm_length_fraction finger_radius_width_fraction finger_spacing_width_fraction finger_length_fractions thumb_length_fraction finger_curl_limit_deg clips".split())
    if set(data) != keys or data["schema"] != "starcrew.crew-rig/1":
        raise ValueError(f"{RIG_TABLE}: invalid rig schema or fields")
    for key, value in data.items():
        if isinstance(value, (int,float)) and (not math.isfinite(value) or value <= 0):
            raise ValueError(f"{RIG_TABLE}:{key}: expected a finite positive number")
    if type(data["fps"]) is not int or not 1 <= data["fps"] <= 60:
        raise ValueError("rig fps must be an integer in 1-60")
    rings = data["limb_rings"]
    if sorted(set(rings)) != rings or rings[0] != 0 or rings[-1] != 1 or .5 not in rings:
        raise ValueError("limb_rings must increase from 0 to 1 and contain the joint at 0.5")
    if len(data["finger_length_fractions"]) != 4 or any(not 0 < n <= 1.5 for n in data["finger_length_fractions"]):
        raise ValueError("expected four positive finger length fractions")
    for key in ("joint_blend_fraction","shoulder_blend_fraction","shoulder_attachment_weight","palm_length_fraction"):
        if not 0 < data[key] < 1:
            raise ValueError(f"{key}: expected a fraction between zero and one")
    names = set()
    for clip in data["clips"]:
        if set(clip) != {"name","duration_s","loop","floor","keys"} or clip["name"] in names:
            raise ValueError("invalid or duplicate clip")
        names.add(clip["name"])
        if clip["duration_s"] <= 0 or abs(clip["duration_s"]*data["fps"]-round(clip["duration_s"]*data["fps"])) > 1e-6:
            raise ValueError("clip duration must be a positive whole number of frames")
        ts = [key["t"] for key in clip["keys"]]
        if sorted(set(ts)) != ts or ts[0] != 0 or ts[-1] != 1:
            raise ValueError("clip keys must increase from 0 to 1")
        for key in clip["keys"]:
            if set(key) != {"t","bones","curl_deg"}:
                raise ValueError("unknown or missing pose field")
            if not 0 <= key["curl_deg"] <= data["finger_curl_limit_deg"]:
                raise ValueError("finger curl exceeds its limit")
            for values in key["bones"].values():
                if len(values) != 3 or any(not math.isfinite(v) or abs(v)>180 for v in values):
                    raise ValueError("bone rotations need three finite angles in -180 to 180 degrees")
        if clip["loop"] and {k:v for k,v in clip["keys"][0].items() if k!="t"} != {k:v for k,v in clip["keys"][-1].items() if k!="t"}:
            raise ValueError(f"{clip['name']}: looping poses must close exactly")
    return data


def blend(a, b, t):
    """Two normalized influences with zero weights omitted."""
    t = max(0.0,min(1.0,t))
    return {name:weight for name,weight in ((a,1-t),(b,t)) if weight > 1e-8}


def limb_weights(t, upper, lower, settings, attachment=None):
    """The same ring blend for a limb's body and clothing shells."""
    if attachment and t < settings["shoulder_blend_fraction"]:
        weight = settings["shoulder_attachment_weight"]
        return blend(attachment,upper,1-weight+weight*t/settings["shoulder_blend_fraction"])
    span = settings["joint_blend_fraction"]
    return blend(upper,lower,(t-(.5-span/2))/span)


def torso_weights(z, hip, shoulder):
    """Continuous pelvis/spine/chest weights, shared by tunic, skin and badge."""
    middle = (hip+shoulder)/2
    if z <= middle:
        return blend("pelvis","spine",(z-hip)/(middle-hip))
    return blend("spine","chest",(z-middle)/(shoulder-middle))


def reset_pose(armature):
    """Restore all local transforms and disable IK for reproducible FK poses."""
    if armature.animation_data:
        armature.animation_data.action = None
    for bone in armature.pose.bones:
        bone.location = (0,0,0)
        bone.rotation_mode = "QUATERNION"
        bone.rotation_quaternion = (1,0,0,0)
        bone.scale = (1,1,1)
    root = armature.pose.bones["root"]
    for side in ("L","R"):
        for limb in ("arm","leg"):
            root[f"ik_{limb}_{side}"] = 0.0
    armature.update_tag()
    bpy.context.view_layer.update()


def set_pose(armature, key):
    """Apply data-held armature-axis rotations and local finger curls."""
    reset_pose(armature)
    for name, degrees in key["bones"].items():
        if name not in armature.pose.bones:
            raise ValueError(f"pose refers to missing bone {name}")
        bone = armature.pose.bones[name]
        rest = bone.bone.matrix_local.to_quaternion()
        rotation = Euler(tuple(math.radians(v) for v in degrees),"XYZ").to_quaternion()
        bone.rotation_quaternion = rest.inverted() @ rotation @ rest
    for side in ("L","R"):
        for finger in FINGERS:
            armature.pose.bones[f"{finger}_{side}"].rotation_quaternion = Quaternion((1,0,0),math.radians(key["curl_deg"]))
    bpy.context.view_layer.update()


def ground_pose(ob, armature):
    """Place the lowest evaluated boot on the floor when baking a review key."""
    evaluated = ob.evaluated_get(bpy.context.evaluated_depsgraph_get())
    floor = min((evaluated.matrix_world @ vertex.co).z for vertex in evaluated.data.vertices)
    root = armature.pose.bones["root"]
    root.location += root.bone.matrix_local.to_3x3().inverted() @ Vector((0,0,-floor))
    bpy.context.view_layer.update()


def add_actions(ob, armature, settings):
    """Create shared FK review actions which export as baked glTF animation tracks."""
    scene = bpy.context.scene
    scene.render.fps = settings["fps"]
    armature.animation_data_create()
    for clip in settings["clips"]:
        action = bpy.data.actions.new(clip["name"])
        action.use_fake_user = True
        for key in clip["keys"]:
            set_pose(armature,key)
            if clip["floor"]:
                ground_pose(ob,armature)
            armature.animation_data.action = action
            frame = 1+round(key["t"]*clip["duration_s"]*settings["fps"])
            for bone in armature.pose.bones:
                if not bone.bone.use_deform:
                    continue
                bone.keyframe_insert("rotation_quaternion",frame=frame,group=bone.name)
                if bone.name == "root":
                    bone.keyframe_insert("location",frame=frame,group=bone.name)
        for curve in action.fcurves:
            for key in curve.keyframe_points:
                key.interpolation = "LINEAR"
    reset_pose(armature)
    scene.frame_start = 1
    scene.frame_end = 1+round(max(clip["duration_s"] for clip in settings["clips"])*settings["fps"])
    scene.frame_set(1)


def widget(name, vertices, edges, collection):
    """A hidden authoring mesh used only as a bone's custom shape."""
    mesh = bpy.data.meshes.new(name)
    mesh.from_pydata(vertices,edges,[])
    ob = bpy.data.objects.new(name,mesh)
    collection.objects.link(ob)
    ob.hide_render = True
    ob.hide_set(True)
    return ob


def add_controls(armature, settings, scale):
    """Add four IK targets, four poles, switches, collections and FK shapes."""
    bpy.context.view_layer.objects.active = armature
    bpy.ops.object.mode_set(mode="EDIT")
    specs = []
    for side in ("L","R"):
        for limb, end, joint in (("arm","hand","forearm"),("leg","foot","shin")):
            source = armature.data.edit_bones[f"{end}_{side}"]
            target = armature.data.edit_bones.new(f"CTRL_{end}_{side}")
            target.head, target.tail, target.roll = source.head.copy(),source.tail.copy(),source.roll
            target.parent = armature.data.edit_bones["root"]
            target.use_deform = False
            pole = armature.data.edit_bones.new(f"CTRL_{'elbow' if limb=='arm' else 'knee'}_{side}")
            pole.head = armature.data.edit_bones[f"{joint}_{side}"].head + Vector((0,settings["pole_distance_m"]*scale*(1 if limb=="arm" else -1),0))
            pole.tail = pole.head+Vector((0,0,settings["control_size_m"]*scale))
            pole.parent = armature.data.edit_bones["root"]
            pole.use_deform = False
            specs.append((limb,side,end,joint,target.name,pole.name))
    bpy.ops.object.mode_set(mode="OBJECT")
    armature.show_in_front = True
    armature.data.display_type = "OCTAHEDRAL"
    collections = {name:armature.data.collections.new(name) for name in ("Body FK","Fingers","IK targets","IK poles")}
    shapes = bpy.data.collections.new("Rig widgets (not exported)")
    bpy.context.scene.collection.children.link(shapes)
    ring = widget("WGT_ring",[(math.cos(i*math.tau/24),0,math.sin(i*math.tau/24)) for i in range(24)],[(i,(i+1)%24) for i in range(24)],shapes)
    square = widget("WGT_box",[(x,y,z) for x,y,z in ((-1,-1,-1),(1,-1,-1),(1,1,-1),(-1,1,-1),(-1,-1,1),(1,-1,1),(1,1,1),(-1,1,1))],[(a,b) for a,b in ((0,1),(1,2),(2,3),(3,0),(4,5),(5,6),(6,7),(7,4),(0,4),(1,5),(2,6),(3,7))],shapes)
    for bone in armature.pose.bones:
        finger = bone.name.split("_")[0] in FINGERS
        control = bone.name.startswith("CTRL_")
        pole = control and ("elbow" in bone.name or "knee" in bone.name)
        group = "IK poles" if pole else "IK targets" if control else "Fingers" if finger else "Body FK"
        for old in list(bone.bone.collections):
            old.unassign(bone.bone)
        collections[group].assign(bone.bone)
        bone.custom_shape = square if control else ring
        factor = .35 if finger else .45 if control else .55
        bone.custom_shape_scale_xyz = (factor,)*3
        bone.color.palette = "THEME04" if control else "THEME03" if bone.name.endswith("_L") else "THEME01" if bone.name.endswith("_R") else "THEME02"
        if finger:
            limit = bone.constraints.new("LIMIT_ROTATION")
            limit.name = "Finger curl limit"
            limit.owner_space = "LOCAL"
            limit.use_limit_x = True
            limit.min_x,limit.max_x = 0,math.radians(settings["finger_curl_limit_deg"])
            bone.lock_rotation = (False,True,True)
    root = armature.pose.bones["root"]
    for limb,side,end,joint,target,pole in specs:
        prop = f"ik_{limb}_{side}"
        root[prop] = 0.0
        root.id_properties_ui(prop).update(min=0,max=1,description="0: FK / baked actions. 1: IK target and pole.")
        constraint = armature.pose.bones[f"{joint}_{side}"].constraints.new("IK")
        constraint.name = f"{limb.capitalize()} IK"
        constraint.target, constraint.subtarget = armature,target
        constraint.pole_target, constraint.pole_subtarget = armature,pole
        constraint.chain_count = 2
        constraint.use_stretch = False
        constraint.iterations = 100
        constraints = [constraint]
        rotation = armature.pose.bones[f"{end}_{side}"].constraints.new("COPY_ROTATION")
        rotation.name = "IK end orientation"
        rotation.target,rotation.subtarget = armature,target
        rotation.target_space = rotation.owner_space = "POSE"
        constraints.append(rotation)
        for item in constraints:
            driver = item.driver_add("influence").driver
            variable = driver.variables.new()
            variable.name = "ik"
            variable.type = "SINGLE_PROP"
            variable.targets[0].id = armature
            variable.targets[0].data_path = f'pose.bones["root"]["{prop}"]'
            driver.expression = "ik"
        # Fit the pole angle to the actual bent rest chain, independent of bone roll.
        root[prop] = 1.0
        expected = armature.data.bones[f"{joint}_{side}"].head_local.copy()
        best = (float("inf"),0.0)
        for step in range(32):
            angle = -math.pi+step*math.tau/32
            constraint.pole_angle = angle
            armature.update_tag(); bpy.context.view_layer.update()
            error = (armature.pose.bones[f"{joint}_{side}"].head-expected).length
            if error < best[0]:
                best = (error,angle)
        angle = best[1]
        width = math.tau/32
        for _ in range(10):
            candidates = []
            for candidate in (angle-width/2,angle,angle+width/2):
                constraint.pole_angle = candidate
                armature.update_tag(); bpy.context.view_layer.update()
                candidates.append(((armature.pose.bones[f"{joint}_{side}"].head-expected).length,candidate))
            _,angle = min(candidates)
            width /= 2
        constraint.pole_angle = angle
        root[prop] = 0.0
    reset_pose(armature)
    armature["README"] = "Pose mode: FK rings or root custom properties ik_arm_L/R and ik_leg_L/R = 1 for IK boxes/poles. Set all IK properties to 0 before playing baked actions. Fingers curl on local X. Game export excludes CTRL bones."


def prepare_blend(ob, armature):
    """Open the generated source in pose mode with a useful material-colour view."""
    reset_pose(armature)
    bpy.ops.object.select_all(action="DESELECT")
    armature.select_set(True)
    bpy.context.view_layer.objects.active = armature
    armature.data.bones.active = armature.data.bones["root"]
    armature.data.bones["root"].select = True
    bpy.ops.object.mode_set(mode="POSE")
    for screen in bpy.data.screens:
        for area in screen.areas:
            if area.type == "VIEW_3D":
                area.spaces.active.shading.color_type = "VERTEX"
                area.spaces.active.overlay.show_floor = True
                area.spaces.active.region_3d.view_distance = 3.4
                area.spaces.active.region_3d.view_location = (0,0,.95)
                area.spaces.active.region_3d.view_rotation = Euler((math.radians(83),0,math.radians(-18)),"XYZ").to_quaternion()
