"""Build original cartoon crew for review, never for automatic adoption aboard.

The data table owns the look. This headless Blender tool owns loft construction,
thirty-bone skinning, palette albedo, artifact measurements and rest-pose garment
coverage. It shares the existing GLB reader with the prop builders. Run with:
blender -b --factory-startup --python-exit-code 1 -P tools/blender/build_crew_characters.py -- [--check] [--only id] [--out DIR]
"""
import argparse
import hashlib
import json
import math
from pathlib import Path
import re
import sys
import tempfile

import bpy
import bmesh
from mathutils import Vector
from mathutils.bvhtree import BVHTree

sys.path.insert(0, str(Path(__file__).resolve().parent))
from hs_kit import read_glb  # One artifact reader, shared with the existing builders.
from crew_rig import RIG_TABLE, FINGERS, load_rig, blend, limb_weights, torso_weights, add_controls, add_actions, prepare_blend

from crew_anatomy import ANATOMY_TABLE, load_anatomy, build_clothes, fitted_hair

ROOT = Path(__file__).resolve().parents[2]
TABLE = ROOT / "data/crew/characters.json"
OUTPUT = ROOT / "assets/models/crew_review"


def fail(message):
    """Stop Blender with a named, readable authoring error."""
    raise RuntimeError(f"[crew] {message}")


def linear(colour):
    """Convert palette sRGB values to the linear albedo stored in COLOR_0."""
    return tuple(c / 12.92 if c <= 0.04045 else ((c + 0.055) / 1.055) ** 2.4 for c in colour)


def load_table(path=TABLE):
    """Validate authored data and the owner's ceilings before constructing anything."""
    table = json.loads(Path(path).read_text(encoding="utf-8"))
    keys = {"schema", "_doc", "budget", "proportions", "anatomy_m", "garments_m", "face", "topology", "palette_srgb", "preview_cubes_linear", "characters"}
    if set(table) != keys or table["schema"] != "starcrew.characters/1":
        fail(f"{path}: invalid schema or unknown/missing fields")
    group_keys = {
        "proportions": "head_scale hand_scale foot_scale neck_length_scale shoulder_width_scale waist_width_scale leg_length_scale",
        "anatomy_m": "head_width head_depth head_height neck_radius neck_length chest_width chest_depth torso_length waist_width waist_depth hip_width leg_length leg_spacing leg_radius boot_height foot_width foot_length hand_width hand_depth hand_height arm_radius arm_length",
        "garments_m": "clearance hem_height cuff_height collar_height collar_thickness boot_sole_height badge_width badge_height badge_depth",
        "face": "eye_width_face_ratio eye_height_width_ratio iris_width_eye_ratio pupil_width_iris_ratio eye_spacing_face_ratio eye_height_head_ratio eye_bulge_m eyelid_margin_m brow_width_eye_ratio brow_thickness_m nose_width_m nose_depth_m mouth_width_m mouth_thickness_m ear_width_m ear_height_m hair_clearance_m",
        "topology": "head_sides body_sides limb_sides eye_sides detail_sides",
        "palette_srgb": "trousers boots trim seam eye_white pupil badge mouth",
        "budget": "triangles bones materials draw_calls",
    }
    for group, fields in group_keys.items():
        if set(table[group]) != set(fields.split()):
            fail(f"{path}:{group}: unknown or missing field")
    ceilings = {"triangles": 3000, "bones": 30, "materials": 1, "draw_calls": 1}
    for key, cap in ceilings.items():
        value = table["budget"][key]
        if type(value) is not int or not 1 <= value <= cap:
            fail(f"{path}:budget.{key}: ceiling must be an integer in 1-{cap}")
    for group in ("proportions", "anatomy_m", "garments_m", "face", "topology"):
        for key, value in table[group].items():
            if type(value) not in (int, float) or not math.isfinite(value) or value <= 0:
                fail(f"{path}:{group}.{key}: expected a finite positive number")
            if group == "topology" and (type(value) is not int or value < 8 or value % 2):
                fail(f"{path}:{group}.{key}: expected an even integer of at least 8")
    face = table["face"]
    if not 0.45 <= face["iris_width_eye_ratio"] <= 0.58 or abs(face["pupil_width_iris_ratio"] - 0.4) > 1e-8:
        fail(f"{path}:face: iris must be 0.45-0.58 and pupil must be 0.40")
    if not 0.18 <= face["eye_width_face_ratio"] <= 0.22:
        fail(f"{path}:face.eye_width_face_ratio: expected about a fifth (0.18-0.22)")
    if set(table["preview_cubes_linear"]) != {"_doc", "normal", "red_alert", "emergency"}:
        fail(f"{path}:preview_cubes_linear: unknown or missing field")
    for state in ("normal", "red_alert", "emergency"):
        cube = table["preview_cubes_linear"][state]
        if len(cube) != 6:
            fail(f"{path}:preview_cubes_linear.{state}: expected six faces")
        for colour in cube:
            rgb(colour, f"{path}:preview_cubes_linear.{state}")
    for key, value in table["palette_srgb"].items():
        rgb(value, f"{path}:palette_srgb.{key}")
    departments = {row["id"]: row for row in json.loads((ROOT / "data/crew/company.json").read_text())["departments"]}
    seen = set()
    row_keys = set("id department sex approval height_m build_scale skin_srgb hair_srgb iris_srgb hair_style".split())
    for row in table["characters"]:
        if set(row) != row_keys or not re.fullmatch(r"[a-z][a-z0-9_]*", row["id"]) or row["id"] in seen:
            fail(f"{path}:characters: invalid row fields or duplicate/invalid id")
        seen.add(row["id"])
        if row["approval"] != "pending":
            fail(f"{row['id']}: review builder only accepts pending rows; approval must name screenshots separately")
        if row["sex"] not in ("male", "female") or row["department"] not in departments or row["hair_style"] not in ("crop", "swept", "bob"):
            fail(f"{row['id']}: unknown department or hair style")
        for key, lo, hi in (("height_m", 1.4, 2.1), ("build_scale", 0.8, 1.2)):
            v = row[key]
            if type(v) not in (int, float) or not math.isfinite(v) or not lo <= v <= hi:
                fail(f"{row['id']}:{key}: expected {lo}-{hi}")
        for key in ("skin_srgb", "hair_srgb", "iris_srgb"):
            rgb(row[key], f"{row['id']}:{key}")
    if not seen:
        fail(f"{path}:characters: no rows")
    pairs = [(row["department"],row["sex"]) for row in table["characters"]]
    if len(pairs) != len(set(pairs)) or set(pairs) != {(department,sex) for department in departments for sex in ("male","female")}:
        fail(f"{path}: each department requires exactly one male and one female")
    return table, departments


def rgb(value, where):
    """Reject non-finite or out-of-range palette and probe colours."""
    if not isinstance(value, list) or len(value) != 3 or any(type(c) not in (int, float) or not math.isfinite(c) or not 0 <= c <= 1 for c in value):
        fail(f"{where}: expected three finite components in 0-1")


class Surface:
    """Accumulate one mesh with per-face palette colour, part tags and skin weights."""

    def __init__(self):
        self.vertices, self.faces, self.colours, self.tags, self.weights = [], [], [], [], []
        self.parts = {}

    def vertex(self, point, bone):
        """Append a Blender-space point and its rigid or blended bone assignment."""
        self.vertices.append(tuple(point))
        self.weights.append(bone(point) if callable(bone) else {bone: 1.0} if isinstance(bone, str) else bone)
        return len(self.vertices) - 1

    def face(self, indices, colour, part):
        """Append a polygon with its artifact-readable part identity."""
        self.parts.setdefault(part, len(self.parts) + 1)
        self.faces.append(indices)
        self.colours.append(linear(colour))
        self.tags.append(self.parts[part])

    def loft(self, part, rings, colour, bone, sides, caps=True, height_offset=None):
        """Closed elliptical loft: rings are centre plus X/Y radius in metres."""
        loops = []
        for ring_index,(center, rx, ry) in enumerate(rings):
            loop = []
            for i in range(sides):
                angle = math.tau * i / sides
                z = center[2]+(height_offset(ring_index,angle) if height_offset else 0)
                loop.append(self.vertex((center[0] + rx * math.cos(angle), center[1] + ry * math.sin(angle), z), bone))
            loops.append(loop)
        for lower, upper in zip(loops, loops[1:]):
            for i in range(sides):
                j = (i + 1) % sides
                self.face((lower[i], lower[j], upper[j], upper[i]), colour, part)
        if caps:
            self.face(tuple(reversed(loops[0])), colour, part)
            self.face(tuple(loops[-1]), colour, part)

    def ellipsoid(self, part, center, radii, colour, bone, sides=10, bands=4):
        """Low-poly smooth ellipsoid, with true poles and no degenerate triangles."""
        south = self.vertex((center[0], center[1], center[2] - radii[2]), bone)
        loops = []
        for j in range(1, bands):
            latitude = -math.pi / 2 + math.pi * j / bands
            loop = []
            for i in range(sides):
                a = math.tau * i / sides
                loop.append(self.vertex((center[0] + radii[0] * math.cos(latitude) * math.cos(a), center[1] + radii[1] * math.cos(latitude) * math.sin(a), center[2] + radii[2] * math.sin(latitude)), bone))
            loops.append(loop)
        north = self.vertex((center[0], center[1], center[2] + radii[2]), bone)
        for i in range(sides):
            j = (i + 1) % sides
            self.face((south, loops[0][j], loops[0][i]), colour, part)
            self.face((loops[-1][i], loops[-1][j], north), colour, part)
        for lower, upper in zip(loops, loops[1:]):
            for i in range(sides):
                j = (i + 1) % sides
                self.face((lower[i], lower[j], upper[j], upper[i]), colour, part)

    def tube(self, part, start, end, radius, colour, bone, sides=10, taper=1.0):
        """Closed limb or facial stroke aligned to two arbitrary points."""
        axis = (Vector(end) - Vector(start)).normalized()
        right = axis.cross(Vector((0, 1, 0))).normalized()
        if right.length < 0.5:
            right = axis.cross(Vector((1, 0, 0))).normalized()
        back = axis.cross(right).normalized()
        loops = []
        for center, scale in ((start, 1.0), (end, taper)):
            loops.append([self.vertex(Vector(center) + radius * scale * (right * math.cos(math.tau * i / sides) + back * math.sin(math.tau * i / sides)), bone) for i in range(sides)])
        for i in range(sides):
            j = (i + 1) % sides
            self.face((loops[0][i], loops[0][j], loops[1][j], loops[1][i]), colour, part)
        self.face(tuple(reversed(loops[0])), colour, part)
        self.face(tuple(loops[1]), colour, part)

    def limb(self, part, start, joint, end, radius, colour, upper, lower, settings, sides, taper=1.0, extension=0.0, attachment=None):
        """Closed bent tube with joint loops and matching skin/garment ring weights."""
        start,joint,end = Vector(start),Vector(joint),Vector(end)
        axis = (end-start).normalized()
        right = axis.cross(Vector((0,1,0))).normalized()
        back = axis.cross(right).normalized()
        loops = []
        for t in settings["limb_rings"]:
            center = start.lerp(joint,t*2) if t <= .5 else joint.lerp(end,(t-.5)*2)
            if t == 0:
                center -= axis*extension
            elif t == 1:
                center += axis*extension
            r = radius*(1+(taper-1)*t)
            weights = limb_weights(t,upper,lower,settings,attachment)
            loops.append([self.vertex(center+r*(right*math.cos(math.tau*i/sides)+back*math.sin(math.tau*i/sides)),weights) for i in range(sides)])
        for a,b in zip(loops,loops[1:]):
            for i in range(sides):
                j = (i+1)%sides
                self.face((a[i],a[j],b[j],b[i]),colour,part)
        self.face(tuple(reversed(loops[0])),colour,part)
        self.face(tuple(loops[-1]),colour,part)

    def box(self, part, center, size, colour, bone):
        """A small solid chest badge or badge bar, never an insignia texture."""
        ids = [self.vertex((center[0] + x * size[0] / 2, center[1] + y * size[1] / 2, center[2] + z * size[2] / 2), bone) for x, y, z in ((-1,-1,-1),(1,-1,-1),(1,1,-1),(-1,1,-1),(-1,-1,1),(1,-1,1),(1,1,1),(-1,1,1))]
        for face in ((0,3,2,1),(4,5,6,7),(0,1,5,4),(1,2,6,5),(2,3,7,6),(3,0,4,7)):
            self.face(tuple(ids[i] for i in face), colour, part)

    def stroke(self, part, points, radius, colour, bone, sides=8):
        """A continuous small mouth or brow curve following the face surface."""
        loops = [[self.vertex((p[0],p[1]+radius*math.cos(math.tau*i/sides),p[2]+radius*math.sin(math.tau*i/sides)),bone) for i in range(sides)] for p in points]
        for a,b in zip(loops,loops[1:]):
            for i in range(sides):
                j = (i+1)%sides
                self.face((a[i],a[j],b[j],b[i]),colour,part)
        self.face(tuple(reversed(loops[0])),colour,part)
        self.face(tuple(loops[-1]),colour,part)

    def eye(self, side, center, width, face, palette, iris, surface_y, skin):
        """Continuous domed eye with actual white/iris/pupil boundary edge loops."""
        n = face["sides"]
        iris_ratio = face["iris_width_eye_ratio"]
        pupil_ratio = iris_ratio * face["pupil_width_iris_ratio"]
        loops = []
        # Outer white is oval; iris and pupil are circular, measured along X.
        outline = 1+2*face["eyelid_margin_m"]/width
        for radius, oval in ((outline,face["eye_height_width_ratio"]), (1.0, face["eye_height_width_ratio"]), (iris_ratio, 1.0), (pupil_ratio, 1.0)):
            depth = face["eye_bulge_m"] * max(0,1-radius*radius)+face["eyelid_margin_m"]/2
            ring = []
            for i in range(n):
                x = center[0]+width/2*radius*math.cos(math.tau*i/n)
                z = center[2]+width/2*radius*oval*math.sin(math.tau*i/n)
                ring.append(self.vertex((x,surface_y(x,z)-depth,z),"head"))
            loops.append(ring)
        for outer, inner, colour, region in ((loops[0],loops[1],skin,"lid"),(loops[1], loops[2], palette["eye_white"], "white"), (loops[2], loops[3], iris, "iris")):
            for i in range(n):
                j = (i + 1) % n
                self.face((outer[i], outer[j], inner[j], inner[i]), colour, f"eye_{side}_{region}")
        pole = self.vertex((center[0], surface_y(center[0],center[2])-face["eye_bulge_m"]-face["eyelid_margin_m"]/2, center[2]), "head")
        for i in range(n):
            self.face((loops[-1][i], loops[-1][(i + 1) % n], pole), palette["pupil"], f"eye_{side}_pupil")


def rig(spec):
    """Create the deform hierarchy in the same frame as the authored mesh."""
    arm = bpy.data.armatures.new("crew_rig")
    ob = bpy.data.objects.new("crew_rig", arm)
    bpy.context.scene.collection.objects.link(ob)
    bpy.context.view_layer.objects.active = ob
    ob.select_set(True)
    bpy.ops.object.mode_set(mode="EDIT")
    for name, parent, head, tail in spec:
        bone = arm.edit_bones.new(name)
        bone.head, bone.tail = head, tail
        axis = (Vector(tail)-Vector(head)).normalized()
        bone.align_roll(Vector((0,-1,0)) if abs(axis.y) < .95 else Vector((0,0,1)))
        if parent:
            bone.parent = arm.edit_bones[parent]
    bpy.ops.object.mode_set(mode="OBJECT")
    return ob


def build(row, table, department):
    """Construct one character, with all tunable dimensions read from the table."""
    bpy.ops.wm.read_factory_settings(use_empty=True)
    a, p, g, f, topology = (table[k] for k in ("anatomy_m", "proportions", "garments_m", "face", "topology"))
    settings = load_rig()
    tuning = load_anatomy()
    variant = tuning["variants"][row["sex"]]
    c = table["palette_srgb"]
    skin, hair, tunic = row["skin_srgb"], row["hair_srgb"], department["colour_srgb"]
    width = row["build_scale"]
    head_w, head_d, head_h = (a[k] * p["head_scale"] for k in ("head_width", "head_depth", "head_height"))
    head_w *= variant["head_width"]
    boot = a["boot_height"]
    legs = a["leg_length"] * p["leg_length_scale"]
    hip = boot + legs
    shoulder = hip + a["torso_length"]
    neck_bottom = shoulder + g["collar_height"]
    head_bottom = neck_bottom + a["neck_length"] * p["neck_length_scale"]
    total = head_bottom + head_h + tuning["hair_clearance_m"]
    scale = row["height_m"] / total
    clearance = g["clearance"]
    limb, detail = topology["limb_sides"], topology["detail_sides"]
    s = Surface()
    limbs,body_weights,chest_x,chest_y,neck_r = build_clothes(s,a,p,g,c,tuning,variant,settings,width,hip,shoulder,neck_bottom,skin,tunic)
    s.loft("neck", [((0,0,shoulder),neck_r,neck_r), ((0,0,head_bottom+clearance),neck_r,neck_r)], skin, "neck", 12)
    specs = [("root",None,(0,0,0),(0,0,boot)), ("pelvis","root",(0,0,hip-clearance),(0,0,hip+clearance)), ("spine","pelvis",(0,0,hip),(0,0,(hip+shoulder)/2)), ("chest","spine",(0,0,(hip+shoulder)/2),(0,0,shoulder)), ("neck","chest",(0,0,shoulder),(0,0,head_bottom)), ("head","neck",(0,0,head_bottom),(0,0,head_bottom+head_h))]
    for side, sign in (("L",1),("R",-1)):
        landmarks = limbs[side]
        x,leg_joint,ankle = (landmarks[k] for k in ("x","knee","ankle"))
        arm_start,elbow,wrist,arm_axis = (landmarks[k] for k in ("arm_start","elbow","wrist","arm_axis"))
        foot_l = a["foot_length"]*p["foot_scale"]*tuning["boot_toe_length_fraction"]
        hand_h = a["hand_height"] * p["hand_scale"] / 2
        palm_length = hand_h*2*settings["palm_length_fraction"]
        hand_center = wrist+arm_axis*palm_length/2
        hand_width = a["hand_width"]*p["hand_scale"]
        s.ellipsoid(f"hand_{side}",hand_center,(hand_width/2,a["hand_depth"]*p["hand_scale"]/2,palm_length*.62),skin,f"hand_{side}",limb)
        specs.extend([(f"clavicle_{side}","chest",(0,0,shoulder),arm_start), (f"upper_arm_{side}",f"clavicle_{side}",arm_start,elbow), (f"forearm_{side}",f"upper_arm_{side}",elbow,wrist), (f"hand_{side}",f"forearm_{side}",wrist,wrist+arm_axis*palm_length), (f"thigh_{side}","pelvis",landmarks["thigh_start"],leg_joint), (f"shin_{side}",f"thigh_{side}",leg_joint,ankle), (f"foot_{side}",f"shin_{side}",ankle,(x,-foot_l,tuning["boot_toe_height_m"]))])
        for index,finger in enumerate(FINGERS[:4]):
            start = wrist+arm_axis*palm_length*.82+Vector((sign*(index-1.5)*hand_width*settings["finger_spacing_width_fraction"],0,0))
            end = start+arm_axis*(hand_h*settings["finger_length_fractions"][index])
            s.tube(f"{finger}_{side}",start,end,hand_width*settings["finger_radius_width_fraction"],skin,f"{finger}_{side}",detail,taper=.78)
            specs.append((f"{finger}_{side}",f"hand_{side}",start,end))
        start = hand_center+Vector((-sign*hand_width*.36,-a["hand_depth"]*.15,0))
        end = start+Vector((-sign*.65,-.20,-.72)).normalized()*hand_h*settings["thumb_length_fraction"]
        s.tube(f"thumb_{side}",start,end,hand_width*settings["finger_radius_width_fraction"]*1.2,skin,f"thumb_{side}",detail,taper=.78)
        specs.append((f"thumb_{side}",f"hand_{side}",start,end))
    # Large head, full jaw and softly faceted cheek planes.
    head_profile = [(0.00,0.47),(0.08,0.73),(0.23,0.94),(0.46,1.00),(0.67,0.98),(0.85,0.81),(0.96,0.47),(1.00,0.15)]
    s.loft("head_skin", [((0,0,head_bottom+head_h*z),head_w*r/2,head_d*r/2) for z,r in head_profile], skin, "head", topology["head_sides"])

    def face_y(x,z):
        # Interpolate the actual loft's faceted front, so eyes do not float off it.
        height = (z-head_bottom)/head_h
        for (z0,r0),(z1,r1) in zip(head_profile,head_profile[1:]):
            if z0 <= height <= z1:
                radius = r0+(r1-r0)*(height-z0)/(z1-z0)
                break
        else:
            fail(f"{row['id']}: facial feature outside the head profile")
        rx,ry = head_w*radius/2,head_d*radius/2
        sides = topology["head_sides"]
        ring = [(rx*math.cos(math.tau*i/sides),ry*math.sin(math.tau*i/sides)) for i in range(sides//2,sides+1)]
        for (x0,y0),(x1,y1) in zip(ring,ring[1:]):
            if x0-1e-8 <= x <= x1+1e-8:
                return y0+(y1-y0)*(x-x0)/(x1-x0)
        fail(f"{row['id']}: facial feature outside head width at {x}, {z} m")

    eye_z = head_bottom + head_h * f["eye_height_head_ratio"]
    eye_w = head_w * f["eye_width_face_ratio"]
    eye_y = -head_d/2 - f["eyelid_margin_m"]
    for side, sign in (("L",1),("R",-1)):
        eye_x = sign * head_w * f["eye_spacing_face_ratio"]
        s.eye(side, (eye_x,eye_y,eye_z), eye_w, {**f,"sides":topology["eye_sides"]}, c, row["iris_srgb"],face_y,tuple(channel*.84 for channel in skin))
        brow_w = eye_w * f["brow_width_eye_ratio"]
        brow_z = eye_z + eye_w*f["eye_height_width_ratio"]/2 + f["brow_thickness_m"]
        brow = []
        for step in (-1,0,1):
            x = eye_x+step*brow_w/2
            z = brow_z+(1-abs(step))*f["brow_thickness_m"]/2
            brow.append((x,face_y(x,z)-f["brow_thickness_m"]/2,z))
        s.stroke(f"brow_{side}",brow,f["brow_thickness_m"]/2,hair,"head",detail)
        s.ellipsoid(f"ear_{side}",(sign*head_w/2,0,eye_z-eye_w/4),(f["ear_width_m"]/2,f["ear_width_m"]/3,f["ear_height_m"]/2),skin,"head",detail)
    s.ellipsoid("nose",(0,eye_y,eye_z-eye_w/2),(f["nose_width_m"]/2,f["nose_depth_m"],f["nose_width_m"]/2),skin,"head",detail)
    mouth_z = head_bottom + head_h*.28
    mouth = []
    for step in (-1,-.5,0,.5,1):
        x,z = step*f["mouth_width_m"]/2,mouth_z+step*step*f["mouth_thickness_m"]*1.5
        mouth.append((x,face_y(x,z)-f["mouth_thickness_m"],z))
    s.stroke("mouth",mouth,f["mouth_thickness_m"]/2,c["mouth"],"head",detail)
    fitted_hair(s,row,head_profile,head_w,head_d,head_h,head_bottom,topology["head_sides"],tuning)
    badge_center = (chest_x*.48,-chest_y-g["badge_depth"]*.6,hip+a["torso_length"]*.69)
    s.box("badge",badge_center,(g["badge_width"],g["badge_depth"],g["badge_height"]),c["badge"],body_weights)
    for sign in (-1,1):
        s.box(f"badge_bar_{sign}",(badge_center[0],badge_center[1]-g["badge_depth"]*.7,badge_center[2]+sign*g["badge_height"]/5),(g["badge_width"]*.62,g["badge_depth"]*.6,g["badge_height"]*.12),c["trim"],body_weights)
    s.vertices = [tuple(v*scale for v in point) for point in s.vertices]
    specs = [(name,parent,tuple(v*scale for v in head),tuple(v*scale for v in tail)) for name,parent,head,tail in specs]
    armature = rig(specs)
    mesh = bpy.data.meshes.new(row["id"])
    mesh.from_pydata(s.vertices, [], s.faces)
    mesh.update()
    ob = bpy.data.objects.new(row["id"],mesh)
    bpy.context.scene.collection.objects.link(ob)
    colour = mesh.color_attributes.new(name="albedo", type="FLOAT_COLOR", domain="CORNER")
    uv = mesh.uv_layers.new(name="part_tag")
    for polygon, tint, tag in zip(mesh.polygons,s.colours,s.tags):
        polygon.use_smooth = True
        for li in polygon.loop_indices:
            colour.data[li].color = (*tint,1)
            uv.data[li].uv = (tag,0)
    bm = bmesh.new()
    bm.from_mesh(mesh)
    bmesh.ops.recalc_face_normals(bm,faces=list(bm.faces))
    bm.to_mesh(mesh)
    bm.free()
    mesh.color_attributes.active_color = mesh.color_attributes["albedo"]
    mat = bpy.data.materials.new("crew_palette")
    mat.use_nodes = True
    mat.use_backface_culling = True
    nodes = mat.node_tree.nodes
    nodes.clear()
    output = nodes.new("ShaderNodeOutputMaterial")
    diffuse = nodes.new("ShaderNodeBsdfPrincipled")
    diffuse.inputs["Roughness"].default_value = 1.0
    diffuse.inputs["Specular IOR Level"].default_value = 0.0
    attr = nodes.new("ShaderNodeVertexColor")
    attr.layer_name = "albedo"
    mat.node_tree.links.new(attr.outputs["Color"],diffuse.inputs["Base Color"])
    mat.node_tree.links.new(diffuse.outputs["BSDF"],output.inputs["Surface"])
    mesh.materials.append(mat)
    for name,_,_,_ in specs:
        group = ob.vertex_groups.new(name=name)
        for vi,weights in enumerate(s.weights):
            if name in weights:
                group.add([vi],weights[name],"REPLACE")
    ob.parent = armature
    mod = ob.modifiers.new("skin","ARMATURE")
    mod.object = armature
    add_controls(armature,settings,scale)
    add_actions(ob,armature,settings)
    return ob, armature, s.parts


def export(ob, armature, path):
    """Write a single skinned GLB using the installed Blender exporter."""
    bpy.ops.object.select_all(action="DESELECT")
    ob.select_set(True)
    armature.select_set(True)
    bpy.context.view_layer.objects.active = ob
    bpy.ops.export_scene.gltf(filepath=str(path), export_format="GLB", use_selection=True,
        export_yup=True, export_apply=False, export_materials="EXPORT", export_normals=True,
        export_texcoords=True, export_tangents=False, export_vertex_color="ACTIVE",
        export_all_vertex_colors=False, export_skins=True, export_animations=True,
        export_def_bones=True, export_animation_mode="ACTIONS", export_anim_slide_to_zero=True, export_force_sampling=True,
        export_frame_range=False, export_optimize_animation_size=False,
        export_morph=False, export_cameras=False, export_lights=False, export_extras=False)


def inspect(path, row, table, parts):
    """Measure the real exported file and refuse malformed or over-budget characters."""
    js, primitives = read_glb(str(path))
    all_primitives = [p for mesh in js.get("meshes",[]) for p in mesh["primitives"]]
    tris = sum(js["accessors"][p["indices"]]["count"]//3 for p in all_primitives)
    joints = len({joint for skin in js.get("skins",[]) for joint in skin["joints"]})
    mesh_nodes = [node for node in js.get("nodes",[]) if "mesh" in node]
    draws = sum(len(js["meshes"][node["mesh"]]["primitives"]) for node in mesh_nodes)
    counts = {"triangles":tris,"bones":joints,"materials":len(js.get("materials",[])),"draw_calls":draws}
    for key, value in counts.items():
        if value > table["budget"][key]:
            fail(f"{row['id']}: over budget: {value} {key} (limit {table['budget'][key]}), read from {path.name}")
    if len(js.get("meshes",[])) != 1 or len(mesh_nodes) != 1 or len(js.get("skins",[])) != 1 or counts["materials"] != 1 or counts["draw_calls"] != 1 or not 1 <= joints <= 30:
        fail(f"{row['id']}: expected one mesh, one material, one primitive and 1-30 bones")
    joint_names = [js["nodes"][index]["name"] for index in js["skins"][0]["joints"]]
    if joints != 30 or any(name.startswith("CTRL_") for name in joint_names):
        fail(f"{row['id']}: expected thirty deform joints, with no Blender control bones")
    for side in ("L","R"):
        for finger in FINGERS:
            if f"{finger}_{side}" not in joint_names:
                fail(f"{row['id']}: missing {finger}_{side} finger joint")
    clips = []
    for animation in js.get("animations",[]):
        times = [js["accessors"][sampler["input"]] for sampler in animation["samplers"]]
        if any(abs(t["min"][0])>1e-6 for t in times):
            fail(f"{row['id']}:{animation['name']}: exported tracks must begin at zero seconds")
        clips.append({"name":animation["name"],"duration_s":round(max(t["max"][0] for t in times)-min(t["min"][0] for t in times),6),"channels":len(animation["channels"])})
        if any(js["nodes"][channel["target"]["node"]]["name"].startswith("CTRL_") for channel in animation["channels"]):
            fail(f"{row['id']}: animation targets an authoring control")
    expected = {clip["name"]:clip["duration_s"] for clip in load_rig()["clips"]}
    if {clip["name"]:clip["duration_s"] for clip in clips} != expected:
        fail(f"{row['id']}: exported animation names or durations differ: {clips}")
    material = js["materials"][0]
    if "KHR_materials_unlit" in material.get("extensions",{}) or any(material.get("emissiveFactor",[0,0,0])):
        fail(f"{row['id']}: material must take diffuse probe light")
    prim = all_primitives[0]
    if not {"COLOR_0","NORMAL","JOINTS_0","WEIGHTS_0"} <= set(prim["attributes"]):
        fail(f"{row['id']}: missing albedo, normals or skinning")
    data = next(iter(primitives.values()))
    pos,uv,idx = data["pos"],data["uv"],data["idx"]
    groups = {name: [] for name in parts}
    faces = {name: [] for name in parts}
    names = {value:name for name,value in parts.items()}
    for vi,tag in enumerate(uv):
        name = names.get(round(tag[0]))
        if name is None or abs(tag[0]-round(tag[0])) > 1e-5:
            fail(f"{row['id']}: corrupted geometric part tag")
        groups[name].append(vi)
    for i in range(0,len(idx),3):
        face = idx[i:i+3]
        tags = {round(uv[vi][0]) for vi in face}
        if len(tags) != 1:
            fail(f"{row['id']}: triangle crosses a part tag")
        faces[names[tags.pop()]].append(face)
    ratios = {}
    head_points = [pos[vi] for vi in groups["head_skin"]]
    head_width = max(point[0] for point in head_points)-min(point[0] for point in head_points)
    for side in ("L","R"):
        widths = {}
        for region in ("white","iris","pupil"):
            points = [pos[vi] for vi in groups[f"eye_{side}_{region}"]]
            widths[region] = max(v[0] for v in points)-min(v[0] for v in points)
        iris_ratio,pupil_ratio = widths["iris"]/widths["white"],widths["pupil"]/widths["iris"]
        if not .45-1e-6 <= iris_ratio <= .58+1e-6 or abs(pupil_ratio-.4) > 1e-5:
            fail(f"{row['id']}: eye {side}: measured iris {iris_ratio:.6f}, pupil {pupil_ratio:.6f}")
        eye_face = widths["white"]/head_width
        if not .18-1e-6 <= eye_face <= .22+1e-6:
            fail(f"{row['id']}: eye {side}: measured eye/face {eye_face:.6f}, expected about a fifth")
        ratios[side] = {"iris_eye":round(iris_ratio,6),"pupil_iris":round(pupil_ratio,6),"eye_face":round(eye_face,6),"eye_width_m":round(widths["white"],6)}
        print(f"[crew] {row['id']} eye {side}: iris/eye={iris_ratio:.6f}, pupil/iris={pupil_ratio:.6f} (exported geometry)")
    coverage = coverage_check(row["id"], pos, groups, faces)
    surface_checks = surface_check(row["id"], pos, groups, faces)
    mins = [min(p[i] for p in pos) for i in range(3)]
    maxs = [max(p[i] for p in pos) for i in range(3)]
    if abs(mins[1]) > 1e-5 or abs(maxs[1]-row["height_m"]) > .005:
        fail(f"{row['id']}: floor or height is wrong: bounds {mins} to {maxs}")
    return {**counts,"vertices":len(pos),"file_bytes":path.stat().st_size,
        "buffer_bytes":sum(b["byteLength"] for b in js.get("buffers",[])),
        "height_m":round(maxs[1]-mins[1],6),"eye_ratios":ratios,
        "covered_skin_samples":coverage,"surface_checks":surface_checks,"parts":parts,"joint_names":joint_names,"animations":clips,
        "sha256":hashlib.sha256(path.read_bytes()).hexdigest()}


def coverage_check(name, positions, groups, faces):
    """Require exported covered skin samples inside closed exported garment shells."""
    clothes = [part for part in groups if part.startswith("garment_") or part.startswith("boot_")]
    for part in clothes:
        edges = {}
        for triangle in faces[part]:
            points = [tuple(round(component,7) for component in positions[vi]) for vi in triangle]
            for a,b in zip(points,points[1:]+points[:1]):
                edge = tuple(sorted((a,b)))
                edges[edge] = edges.get(edge,0)+1
        if any(count != 2 for count in edges.values()):
            fail(f"{name}: {part}: garment shell is not closed")
    volumes = {part:BVHTree.FromPolygons(positions,faces[part],all_triangles=True) for part in clothes}
    ray = Vector((.782131,.352619,.513247)).normalized()

    def inside(point, tree):
        hits = 0
        origin = Vector(point)
        # Count unique surface crossings; advance beyond edge duplicates.
        for _ in range(100):
            location,normal,index,distance = tree.ray_cast(origin,ray,10.0)
            if location is None:
                return hits % 2 == 1
            hits += 1
            origin = location+ray*1e-5
        fail(f"{name}: excessive garment ray crossings")

    checked = 0
    for part in sorted(groups):
        if not part.startswith("covered_"):
            continue
        if part == "covered_torso":
            candidates = ["garment_tunic"]
        elif part.startswith("covered_arm_"):
            candidates = ["garment_tunic"]
        else:
            side = part.rsplit("_",1)[1]
            candidates = ["garment_trousers","boot_"+side]
        samples = [Vector(positions[vi]) for vi in groups[part]]
        for triangle in faces[part]:
            a,b,c = (Vector(positions[vi]) for vi in triangle)
            samples.extend([(a+b+c)/3,(a+b)/2,(b+c)/2,(c+a)/2])
        for point in samples:
            if not any(inside(point,volumes[garment]) for garment in candidates):
                fail(f"{name}: {part} skin outside its garment at {tuple(round(v,5) for v in point)} m")
            checked += 1
    print(f"[crew] {name}: {checked} exported covered-skin samples inside garment shells")
    return checked


def surface_check(name, positions, groups, faces, rest_positions=None, up_axis=1):
    """Check welded garments, self intersections, hair and the visible waist.

    Horizontal closure caps are internal construction surfaces, so the waist
    check excludes them using bind-pose positions even when testing animations.
    """
    rest_positions = positions if rest_positions is None else rest_positions
    components = {}
    shells = {}
    for part in ("garment_tunic","garment_trousers"):
        graph = {}
        for face in faces[part]:
            keys = [tuple(round(c,7) for c in positions[vi]) for vi in face]
            for a,b in zip(keys,keys[1:]+keys[:1]):
                graph.setdefault(a,set()).add(b)
                graph.setdefault(b,set()).add(a)
        unseen = set(graph)
        count = 0
        while unseen:
            count += 1
            pending = [unseen.pop()]
            while pending:
                for other in graph[pending.pop()] & unseen:
                    unseen.remove(other)
                    pending.append(other)
        if count != 1:
            fail(f"{name}: {part}: disconnected garment: {count} components")
        components[part] = count
        triangles = faces[part]
        tree = BVHTree.FromPolygons(positions,triangles,all_triangles=True)
        overlaps = [(i,j) for i,j in tree.overlap(tree) if i<j and not any(
            (Vector(positions[a])-Vector(positions[b])).length<1e-6 for a in triangles[i] for b in triangles[j])]
        if overlaps:
            fail(f"{name}: {part}: self intersection: {len(overlaps)} triangle pairs")
        shells[part] = [face for face in triangles if max(rest_positions[vi][up_axis] for vi in face)-min(rest_positions[vi][up_axis] for vi in face)>1e-6]
    tunic = BVHTree.FromPolygons(positions,shells["garment_tunic"],all_triangles=True)
    trousers = BVHTree.FromPolygons(positions,shells["garment_trousers"],all_triangles=True)
    waist = tunic.overlap(trousers)
    if waist:
        fail(f"{name}: trousers intersect tunic: {len(waist)} triangle pairs")
    hair = BVHTree.FromPolygons(positions,faces["hair_cap"],all_triangles=True)
    scalp = BVHTree.FromPolygons(positions,faces["head_skin"],all_triangles=True)
    overlaps = hair.overlap(scalp)
    if overlaps:
        fail(f"{name}: hair intersects scalp: {len(overlaps)} triangle pairs")
    return {"garment_components":components,"hair_scalp_intersections":len(overlaps),"waist_intersections":len(waist),"garment_self_intersections":0}


def main():
    """Build or verify all requested review artifacts without changing game selection."""
    from check_crew_rigs import validate_exported, validate_controls
    args = sys.argv[sys.argv.index("--")+1:] if "--" in sys.argv else ([] if "-P" in sys.argv or "--python" in sys.argv else sys.argv[1:])
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check",action="store_true")
    parser.add_argument("--only",help="comma-separated character ids")
    parser.add_argument("--out",type=Path,default=OUTPUT)
    parser.add_argument("--table",type=Path,default=TABLE)
    parser.add_argument("--blend",type=Path,help="editable Blender control rigs; generated from the committed data")
    options = parser.parse_args(args)
    table,departments = load_table(options.table)
    if options.only and (options.out/"characters.json").exists():
        previous = json.loads((options.out/"characters.json").read_text())
        if previous["source_sha256"] != hashlib.sha256(options.table.read_bytes()).hexdigest() or previous["company_sha256"] != hashlib.sha256((ROOT/"data/crew/company.json").read_bytes()).hexdigest() or previous.get("anatomy_source_sha256") != hashlib.sha256(ANATOMY_TABLE.read_bytes()).hexdigest() or previous.get("rig_source_sha256") != hashlib.sha256(RIG_TABLE.read_bytes()).hexdigest():
            fail("partial build would mix different data revisions; rebuild the whole table")
    rows = table["characters"]
    if options.only:
        wanted = set(options.only.split(","))
        if wanted - {row["id"] for row in rows}:
            fail(f"unknown character ids {sorted(wanted)}")
        rows = [row for row in rows if row["id"] in wanted]
    # Stage the whole requested batch so failed measurements cannot publish partial assets.
    with tempfile.TemporaryDirectory(prefix="starcrew-characters-") as directory:
        stage = Path(directory)
        facts = []
        for row in rows:
            ob,armature,parts = build(row,table,departments[row["department"]])
            path = stage / f"{row['id']}.glb"
            export(ob,armature,path)
            measured = inspect(path,row,table,parts)
            facts.append({"id":row["id"],"department":row["department"],"sex":row["sex"],"approval":"pending","file":path.name,**measured})
            if options.blend:
                (stage/"blender").mkdir(exist_ok=True)
                prepare_blend(ob,armature)
                bpy.context.preferences.filepaths.save_version = 0
                bpy.ops.wm.save_as_mainfile(filepath=str(stage/"blender"/f"{row['id']}.blend"),compress=True,check_existing=False)
            validate_exported(path,facts[-1],coverage_check)
            if options.blend:
                validate_controls(stage/"blender"/f"{row['id']}.blend")
            print(f"[crew] {row['id']}: {measured['triangles']} triangles, {measured['bones']} bones, {measured['materials']} material, {measured['draw_calls']} primitive")
        manifest = {"schema":"starcrew.crew-review/1","status":"pending screenshot approval; engine and mockups retain blocky figures",
            "generator":{"script":"tools/blender/build_crew_characters.py","blender":bpy.app.version_string},
            "source_sha256":hashlib.sha256(options.table.read_bytes()).hexdigest(),
            "company_sha256":hashlib.sha256((ROOT/"data/crew/company.json").read_bytes()).hexdigest(),
            "rig_source_sha256":hashlib.sha256(RIG_TABLE.read_bytes()).hexdigest(),
            "anatomy_source_sha256":hashlib.sha256(ANATOMY_TABLE.read_bytes()).hexdigest(),
            "budget":table["budget"],"characters":facts}
        if options.only and (options.out/"characters.json").exists():
            previous = json.loads((options.out/"characters.json").read_text())
            new = {row["id"]:row for row in facts}
            manifest["characters"] = [new.get(row["id"],row) for row in previous["characters"]]
        (stage/"characters.json").write_text(json.dumps(manifest,indent=2)+"\n",encoding="utf-8")
        for path in stage.iterdir():
            if path.is_dir():
                continue
            target = options.out/path.name
            if options.check:
                if not target.exists() or path.read_bytes() != target.read_bytes():
                    fail(f"{target}: rebuild differs")
            else:
                options.out.mkdir(parents=True,exist_ok=True)
                target.write_bytes(path.read_bytes())
        if options.blend and not options.check:
            options.blend.mkdir(parents=True,exist_ok=True)
            for path in (stage/"blender").iterdir():
                (options.blend/path.name).write_bytes(path.read_bytes())
        print(f"[crew] {'verified identical bytes' if options.check else 'wrote review artifacts'}: {len(rows)} characters")


if __name__ == "__main__":
    main()
