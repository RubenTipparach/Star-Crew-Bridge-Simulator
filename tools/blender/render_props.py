"""Stills of a prop set (the bridge, suite or machinery props), for looking at them before anyone
else does (CLAUDE.md 12).

It owns nothing the game loads. It imports the exported .glb files listed in the set's
props.json (assets/models/<set>/props.json), not the build's scene (CLAUDE.md 6.6, validate the
real artifact), so the pictures show the geometry, normals and materials as the page will get
them. Each prop wears its baked atlas (props.json atlas, read through the glb's TEXCOORD_1, nearest
sampling as the page samples it up close), glowing where the atlas's alpha says; screen faces stay
the page's flat screen colour and accent faces are multiplied by the glb's accent preview colour, as
a page multiplies them by a station's. It writes, into docs/screenshots/props/ (or --shots):
  <name>.png            one prop, isometric, on a floor (and a wall when its anchor is on the wall plane)
  <name>-<view>.png     with --views, the other views: front (a low front three-quarter) and back
  the set's sheet       every prop laid out together, labelled with its triangles: contact-sheet.png
                        for the bridge set, suite-props.png for the suite set, machinery-props.png
                        for the machinery set
Flat colours come from the glb's own materials (the build takes them from each Material Maker
layer's mean colour, and from shipkit.js for screen and accent); screens and light strips glow.
Edges are drawn with Freestyle so the chamfers and cuts read, the cutaway look. Cycles on the CPU
with few samples: it runs headless, without a GPU (Workbench and EEVEE need an OpenGL context
that a cloud session does not have).

Run (from anywhere):
  <python with the bpy module> tools/blender/render_props.py [--set bridge|suite|engineering|machinery|doors] [--only a,b] [--samples 24]
      [--no-sheet] [--no-stills] [--shots DIR] [--views iso,front,back]
  blender -b --factory-startup -P tools/blender/render_props.py -- [same options]
  --set        which prop set: bridge (the default), suite, engineering, machinery or doors
  --only       render the stills of only these props (the sheet still shows the whole set)
  --no-sheet   skip the contact sheet; --no-stills skip the stills
  --shots      write into DIR instead of docs/screenshots/props
  --views      which views of each still: iso (the default, <name>.png), front, back
"""
import json
import math
import os
import sys

import bpy  # first: with the pip bpy module, mathutils exists only once bpy is imported
from mathutils import Vector  # noqa: E402

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(os.path.dirname(HERE))
SHOTS = os.path.join(ROOT, "docs", "screenshots", "props")
VIEW = Vector((1.0, -1.35, 0.95)).normalized()      # from the prop toward the camera: front right, above
# the stills' views (Blender frame: the prop's front, prop +Z, is Blender -Y): the isometric, a low
# front three-quarter from the left and a three-quarter from behind
VIEWS = {"iso": VIEW, "front": Vector((-0.75, -1.3, 0.55)).normalized(), "back": Vector((0.8, 1.25, 0.75)).normalized()}
FLOOR_RGB = (0.025, 0.028, 0.034)   # linear
WALL_RGB = (0.06, 0.066, 0.078)
# Per set: its folder, the sheet's rows (the first row stands against a wall), the sheet's file, the
# gaps between its rows (wide enough that a tall prop does not hide the labels of the row behind) and
# its props, and the labels' size; optionally the sheet's pixel size (res, default 1600 x 1100) and
# the floor's size in metres (floor_m, default 40 x 40) for a set too large for those.
SETS = {
    "bridge": {
        "dir": os.path.join(ROOT, "assets", "models", "bridge"),
        "rows": [["wall_bank_core", "wall_bank_core_engineering", "wall_bank_core_science", "wall_bank", "wall_bank_comms",
                  "wall_bank_flight_ops", "wall_bank_double"],
                 ["free_console", "free_console_helm", "free_console_tactical", "helm_arc", "standup_console"],
                 ["captain_chair", "crew_chair"]],
        "sheet": "contact-sheet.png",
        "gap_y": 2.6,     # metres between rows, front of one to back of the next
        "gap_x": 0.8,     # metres between props in a row
        "label_m": 0.13,  # the labels' letter height
    },
    "suite": {
        "dir": os.path.join(ROOT, "assets", "models", "suite"),
        "rows": [["wall_screen", "shelf", "wardrobe", "locker_bank", "wash_counter"],
                 ["workbench", "sofa", "bed", "toilet_stall", "shower_stall"],
                 ["briefing_table", "desk", "low_table", "wet_cell", "server_rack"],
                 ["rifle_rack", "ammo_cabinet", "armour_rack"]],
        "sheet": "suite-props.png",
        "gap_y": 3.8,     # the suite's pod and stalls are tall: a 2.3 m prop hides 3.3 m of floor behind it
        "gap_x": 1.0,
        "label_m": 0.17,  # a wider scene than the bridge's, so larger labels to stay legible
    },
    "engineering": {
        "dir": os.path.join(ROOT, "assets", "models", "engineering"),
        "rows": [["control_desk", "mimic_board", "local_panel", "tool_board", "tool_chest", "parts_rack"],
                 ["coolant_pump", "coolant_tank", "pressurizer", "heat_exchanger", "power_converter", "valve_large", "valve_small"],
                 ["fuel_dewar", "helium3_rack", "fuel_processor", "cryoplant", "vacuum_pump", "ash_tank"]],
        "sheet": "engineering-props.png",
        "gap_y": 3.6,
        "gap_x": 1.0,
        "label_m": 0.2,
    },
    "machinery": {
        "dir": os.path.join(ROOT, "assets", "models", "machinery"),
        # the reactor (10 m tall) stands at the left end of the back row, where it hides nothing
        "rows": [["reactor_core", "switchboard", "battery_bank", "ls_tanks", "ls_scrubbers", "ls_air_handler", "bunk",
                  "galley_counter"],
                 ["coolant_pumps", "inertial_dampers", "shield_generator", "gravity_generator", "med_bed", "mess_table",
                  "missile_tube"],
                 ["impulse_drive", "magazine_rack", "launch_cradle", "swift_fighter", "petrel_shuttle"]],
        "sheet": "machinery-props.png",
        "gap_y": 3.4,
        "gap_x": 1.0,
        "label_m": 0.36,  # the widest scene of the three
        "res": (2400, 1650),   # the reactor is 10 m tall: the rest of the set needs the pixels
        "floor_m": (90, 90),   # the rows nearer the camera shift right past a 40 m floor
    },
    "doors": {
        "dir": os.path.join(ROOT, "assets", "models", "doors"),
        # each sliding door's two leaves side by side as they meet, the pressure leaf and the lift car's at the end
        "rows": [["door_100x220_l", "door_100x220_r", "door_120x220_l", "door_120x220_r", "door_120x210_l", "door_120x210_r"],
                 ["door_160x230_l", "door_160x230_r", "door_200x230_l", "door_200x230_r", "door_200x250_l", "door_200x250_r",
                  "pressure_100x220"]],
        "sheet": "door-props.png",
        "gap_y": 2.4,
        "gap_x": 0.6,
        "label_m": 0.12,
    },
}


def on_wall(row):
    """A prop whose back stands on a wall (its anchor says so): its stills draw the wall."""
    return "wall plane" in row["anchor"]


def parse_args():
    args = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else (
        [] if os.path.basename(sys.argv[0]).startswith("blender") else sys.argv[1:])
    opts = {"set": "bridge", "only": None, "samples": 24, "sheet": True, "stills": True, "shots": SHOTS, "views": ["iso"]}
    i = 0
    while i < len(args):
        if args[i] == "--set":
            opts["set"] = args[i + 1]
            i += 1
            if opts["set"] not in SETS:
                raise SystemExit(f"[render] no prop set {opts['set']!r}; the sets are {', '.join(SETS)}")
        elif args[i] == "--shots":
            opts["shots"] = os.path.abspath(args[i + 1])
            i += 1
        elif args[i] == "--no-stills":
            opts["stills"] = False
        elif args[i] == "--only":
            opts["only"] = args[i + 1].split(",")
            i += 1
        elif args[i] == "--samples":
            opts["samples"] = int(args[i + 1])
            i += 1
        elif args[i] == "--no-sheet":
            opts["sheet"] = False
        elif args[i] == "--views":
            opts["views"] = args[i + 1].split(",")
            i += 1
            for v in opts["views"]:
                if v not in VIEWS:
                    raise SystemExit(f"[render] no view {v!r}; the views are {', '.join(VIEWS)}")
        else:
            raise SystemExit(f"[render] unknown argument {args[i]!r}\n{__doc__}")
        i += 1
    return opts


def reset(samples):
    bpy.ops.wm.read_factory_settings(use_empty=True)
    sc = bpy.context.scene
    sc.render.engine = "CYCLES"
    sc.cycles.device = "CPU"
    sc.cycles.samples = samples
    sc.cycles.use_denoising = True
    sc.cycles.max_bounces = 4
    sc.view_settings.view_transform = "Standard"
    sc.view_settings.look = "None"
    sc.render.use_freestyle = True
    sc.render.line_thickness_mode = "ABSOLUTE"
    sc.render.line_thickness = 1.1
    vl = sc.view_layers[0]
    vl.use_freestyle = True
    fs = vl.freestyle_settings
    fs.crease_angle = math.radians(140.0)
    ls = fs.linesets[0] if fs.linesets else fs.linesets.new("Lines")
    ls.select_by_visibility = True
    ls.select_by_edge_types = True
    ls.select_silhouette = ls.select_border = ls.select_crease = True
    ls.select_by_collection = True
    if ls.linestyle is None:     # an empty factory scene has a line set without a style
        ls.linestyle = bpy.data.linestyles.new("Ink")
    ls.linestyle.color = (0.02, 0.025, 0.03)
    world = bpy.data.worlds.new("World")
    world.use_nodes = True
    bg = world.node_tree.nodes["Background"]
    bg.inputs["Color"].default_value = (0.055, 0.065, 0.085, 1.0)
    bg.inputs["Strength"].default_value = 1.0
    sc.world = world
    for name, energy, rot in (("Key", 3.2, (math.radians(50), 0, math.radians(-35))),
                              ("Fill", 0.9, (math.radians(65), 0, math.radians(120)))):
        ld = bpy.data.lights.new(name, "SUN")
        ld.energy = energy
        ld.angle = math.radians(8)
        ob = bpy.data.objects.new(name, ld)
        ob.rotation_euler = rot
        sc.collection.objects.link(ob)
    # an ambient term for the parts the suns miss, so a face never goes black
    world.node_tree.nodes["Background"].inputs["Color"].default_value = (0.11, 0.12, 0.15, 1.0)
    props = bpy.data.collections.new("Props")
    sc.collection.children.link(props)
    ls.collection = props
    return props


def flat_material(mat):
    """Rebuild an imported glTF material as a flat colour: diffuse, or emission for a screen or a lamp."""
    nt = mat.node_tree
    bsdf = next((n for n in nt.nodes if n.type == "BSDF_PRINCIPLED"), None)
    if bsdf is None:
        return
    rgba = tuple(bsdf.inputs["Base Color"].default_value)
    glow = tuple(bsdf.inputs["Emission Color"].default_value)[:3]
    strength = bsdf.inputs["Emission Strength"].default_value
    emissive = strength > 0 and max(glow) > 0
    for n in list(nt.nodes):
        nt.nodes.remove(n)
    out = nt.nodes.new("ShaderNodeOutputMaterial")
    if emissive:
        sh = nt.nodes.new("ShaderNodeEmission")
        sh.inputs["Color"].default_value = rgba
        sh.inputs["Strength"].default_value = 1.0
    else:
        sh = nt.nodes.new("ShaderNodeBsdfDiffuse")
        sh.inputs["Color"].default_value = rgba
    nt.links.new(sh.outputs[0], out.inputs["Surface"])


def plain(name, rgb):
    m = bpy.data.materials.new(name)
    m.use_nodes = True
    nt = m.node_tree
    nt.nodes["Principled BSDF"].inputs["Base Color"].default_value = (*rgb, 1.0)
    nt.nodes["Principled BSDF"].inputs["Roughness"].default_value = 0.9
    return m


def plane(name, size, loc, rot, mat):
    bpy.ops.mesh.primitive_plane_add(size=1.0)
    ob = bpy.context.active_object
    ob.name = name
    ob.scale = (size[0], size[1], 1.0)
    ob.location = loc
    ob.rotation_euler = rot
    ob.data.materials.append(mat)
    return ob


def atlas_material(mat, atlas_path):
    """Rebuild an imported glTF material to wear the prop's atlas through the second UV map: diffuse
    where the atlas's alpha is 0, glowing where it is 1; accent multiplied by the material's own
    (preview) colour, as a page multiplies it by the station's; screen stays flat (the page draws
    the console faces there)."""
    role = mat.name.split(".")[0]
    if role == "screen":
        flat_material(mat)
        return
    nt = mat.node_tree
    bsdf = next((n for n in nt.nodes if n.type == "BSDF_PRINCIPLED"), None)
    tint = tuple(bsdf.inputs["Base Color"].default_value) if bsdf else (1.0, 1.0, 1.0, 1.0)
    for n in list(nt.nodes):
        nt.nodes.remove(n)
    out = nt.nodes.new("ShaderNodeOutputMaterial")
    uvn = nt.nodes.new("ShaderNodeUVMap")
    uvn.uv_map = "UVMap.001"
    tex = nt.nodes.new("ShaderNodeTexImage")
    tex.image = bpy.data.images.load(atlas_path, check_existing=True)
    tex.image.alpha_mode = "CHANNEL_PACKED"     # alpha is the glow mask, not coverage: never premultiply by it
    tex.interpolation = "Closest"
    nt.links.new(uvn.outputs["UV"], tex.inputs["Vector"])
    col = tex.outputs["Color"]
    if role == "accent":
        mix = nt.nodes.new("ShaderNodeMix")
        mix.data_type = "RGBA"
        mix.blend_type = "MULTIPLY"
        mix.inputs["Factor"].default_value = 1.0
        nt.links.new(col, mix.inputs[6])
        mix.inputs[7].default_value = tint
        col = mix.outputs[2]
    dif = nt.nodes.new("ShaderNodeBsdfDiffuse")
    nt.links.new(col, dif.inputs["Color"])
    emi = nt.nodes.new("ShaderNodeEmission")
    nt.links.new(col, emi.inputs["Color"])
    sh = nt.nodes.new("ShaderNodeMixShader")
    nt.links.new(tex.outputs["Alpha"], sh.inputs[0])
    nt.links.new(dif.outputs[0], sh.inputs[1])
    nt.links.new(emi.outputs[0], sh.inputs[2])
    nt.links.new(sh.outputs[0], out.inputs["Surface"])


def import_prop(path, coll, offset, atlas=None):
    """Import a prop's glb into coll, moved by offset; with atlas (its PNG's path) it wears it."""
    before = set(bpy.data.objects)
    bpy.ops.import_scene.gltf(filepath=path)
    new = [o for o in bpy.data.objects if o not in before]
    for o in new:
        for c in list(o.users_collection):
            c.objects.unlink(o)
        coll.objects.link(o)
        o.location += offset
        if o.type == "MESH":
            if atlas and len(o.data.uv_layers) > 1:
                o.data.uv_layers[1].name = "UVMap.001"
            for slot in o.material_slots:
                if slot.material and not slot.material.get("_flat"):
                    if atlas:
                        slot.material = slot.material.copy()
                        atlas_material(slot.material, atlas)
                    else:
                        flat_material(slot.material)
                    slot.material["_flat"] = True
    return [o for o in new if o.type == "MESH"]


def atlas_of(props_dir, row):
    return os.path.join(props_dir, row["atlas"]["file"]) if row.get("atlas") else None


def points(objs):
    """World-space vertices of meshes (bounding box corners for anything else): the camera fits
    these, not a box round them, which in an isometric view is much wider than the props."""
    out = []
    for o in objs:
        if o.type == "MESH":
            out += [o.matrix_world @ v.co for v in o.data.vertices]
        else:
            out += [o.matrix_world @ Vector(c) for c in o.bound_box]
    return out


def bounds(objs):
    pts = points(objs)
    return (Vector((min(p.x for p in pts), min(p.y for p in pts), min(p.z for p in pts))),
            Vector((max(p.x for p in pts), max(p.y for p in pts), max(p.z for p in pts))))


def camera(objs, res, margin=1.12, view=VIEW):
    lo, hi = bounds(objs)
    sc = bpy.context.scene
    sc.render.resolution_x, sc.render.resolution_y = res
    c = (lo + hi) / 2
    rot = (-view).to_track_quat("-Z", "Y")
    cd = bpy.data.cameras.new("Cam")
    cd.type = "ORTHO"
    cd.clip_end = 200.0
    cam = bpy.data.objects.new("Cam", cd)
    cam.rotation_euler = rot.to_euler()
    cam.location = c + view * 30.0
    sc.collection.objects.link(cam)
    sc.camera = cam
    right, up = rot @ Vector((1, 0, 0)), rot @ Vector((0, 1, 0))
    pts = points(objs)
    us = [(p - c).dot(right) for p in pts]
    vs = [(p - c).dot(up) for p in pts]
    aspect = res[0] / res[1]
    w = max(max(us) - min(us), (max(vs) - min(vs)) * aspect) * margin
    cd.ortho_scale = w
    cd.shift_x = (max(us) + min(us)) / 2 / w
    cd.shift_y = (max(vs) + min(vs)) / 2 / w
    return cam


def label(text, loc, size, coll, mat):
    cu = bpy.data.curves.new("Label", "FONT")
    cu.body = text
    cu.size = size
    cu.align_x = "CENTER"
    cu.align_y = "TOP"
    ob = bpy.data.objects.new("Label", cu)
    ob.location = loc
    ob.data.materials.append(mat)
    coll.objects.link(ob)
    return ob


def render(path):
    bpy.context.scene.render.filepath = path
    bpy.ops.render.render(write_still=True)
    print("[render] wrote", path)


def one(props_dir, shots, name, row, samples, view="iso"):
    props = reset(samples)
    objs = import_prop(os.path.join(props_dir, row["file"]), props, Vector(), atlas_of(props_dir, row))
    plane("Floor", (12, 12), (0, 0, 0), (0, 0, 0), plain("floor", FLOOR_RGB))
    if on_wall(row) and view != "back":
        # the wall the prop stands on, 2 mm behind its back face (prop z = 0 is Blender y = 0)
        plane("Wall", (12, 6), (0, 0.002, 3), (math.radians(90), 0, 0), plain("wall", WALL_RGB))
    camera(objs, (1024, 768), view=VIEWS[view])
    render(os.path.join(shots, name + ("" if view == "iso" else "-" + view) + ".png"))


def sheet(ps, out, rows, samples):
    """Every prop on one floor: the wall banks against a wall, the free-standing consoles in front
    of them, the chairs in front of those, each labelled with its triangles and budget."""
    props = reset(samples)
    text_mat = plain("label", (0.75, 0.78, 0.82))
    labels = bpy.data.collections.new("Labels")
    bpy.context.scene.collection.children.link(labels)
    gap_x, gap_y = ps["gap_x"], ps["gap_y"]
    y = 0.0                                                 # Blender y of the row's back line
    placed = []
    for r, names in enumerate(ps["rows"]):
        names = [n for n in names if n in rows]
        if not names:
            continue
        widths = [rows[n]["dimensions_m"][0] for n in names]
        # a row nearer the camera moves right, so on screen the rows stack instead of drifting left
        x = -(sum(widths) + gap_x * (len(names) - 1)) / 2 - y * VIEW.x / -VIEW.y
        for n, w in zip(names, widths):
            b = rows[n]["bounds_m"]
            # prop z (toward the operator) is Blender -y: put the prop's back on the row's line
            off = Vector((x - b["min"][0], y + b["min"][2], 0.0))
            objs = import_prop(os.path.join(ps["dir"], rows[n]["file"]), props, off, atlas_of(ps["dir"], rows[n]))
            placed += objs
            front_y = y - (b["max"][2] - b["min"][2])
            placed.append(label(f"{n}\n{rows[n]['triangles']} / {rows[n]['budget_triangles']} triangles",
                                Vector((x + w / 2, front_y - 0.12, 0.003)), ps["label_m"], labels, text_mat))
            x += w + gap_x
        if r == 0:
            plane("Wall", (40, 6), (0, 0.002, 3), (math.radians(90), 0, 0), plain("wall", WALL_RGB))
        y -= max(rows[n]["dimensions_m"][2] for n in names) + gap_y
    plane("Floor", ps.get("floor_m", (40, 40)), (0, -5, 0), (0, 0, 0), plain("floor", FLOOR_RGB))
    camera(placed, ps.get("res", (1600, 1100)), margin=1.05)
    render(out)


def main():
    opts = parse_args()
    ps = SETS[opts["set"]]
    rows = json.load(open(os.path.join(ps["dir"], "props.json"), encoding="utf-8"))["props"]
    os.makedirs(opts["shots"], exist_ok=True)
    if opts["stills"]:
        for n in opts["only"] or list(rows):
            if n not in rows:
                raise SystemExit(f"[render] {n!r} is not in props.json")
            for v in opts["views"]:
                one(ps["dir"], opts["shots"], n, rows[n], opts["samples"], v)
    if opts["sheet"]:
        sheet(ps, os.path.join(opts["shots"], ps["sheet"]), rows, opts["samples"])


if __name__ == "__main__":
    main()
