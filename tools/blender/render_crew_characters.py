"""Render exported crew review GLBs headlessly, with turnarounds and probe evidence.

This is offline review tooling. It shares prop camera and CPU render setup, loads
the actual written files, and never selects a game character. The six-face probe
preview is illustrative; it does not claim live engine probe interpolation.
"""
import argparse
import json
import math
from pathlib import Path
import sys

import bpy
from mathutils import Quaternion, Vector

sys.path.insert(0,str(Path(__file__).resolve().parent))
from build_crew_characters import load_table, ROOT, OUTPUT
from crew_rig import reset_pose
from render_props import reset, camera, plain, plane, label

MODEL_ROOT = OUTPUT
SHOTS = ROOT / "docs/screenshots/crew-characters"


def scene(samples):
    """Use the shared CPU Cycles studio, with diffuse character materials."""
    collection = reset(samples)
    sc = bpy.context.scene
    sc.render.use_freestyle = False
    sc.view_layers[0].use_freestyle = False
    sc.render.image_settings.file_format = "PNG"
    sc.render.resolution_percentage = 100
    sc.world.color = (.025,.035,.045)
    floor = plain("review_floor",(.028,.039,.053))
    plane("Floor",(200,200),(0,0,-.005),(0,0,0),floor)
    return collection


def material(ob, probe=None):
    """Use exported albedo directly, optionally multiplied by an ambient cube."""
    mesh = ob.data
    albedo = mesh.color_attributes.active_color or mesh.color_attributes[0]
    layer = albedo.name
    if probe is not None:
        colours = [tuple(value.color) for value in albedo.data]
        baked = mesh.color_attributes.new(name="review_probe",type="FLOAT_COLOR",domain="CORNER")
        # Game normals are Blender (x,z,-y), and cube faces are +X,-X,+Y,-Y,+Z,-Z.
        for polygon in mesh.polygons:
            for li in polygon.loop_indices:
                normal = mesh.corner_normals[li].vector
                nx,ny,nz = normal.x,normal.z,-normal.y
                light = [0.0,0.0,0.0]
                for component,positive,negative in ((nx,0,1),(ny,2,3),(nz,4,5)):
                    face = probe[positive if component >= 0 else negative]
                    for channel in range(3):
                        light[channel] += component*component*face[channel]
                colour = colours[li]
                baked.data[li].color = (*(colour[i]*light[i] for i in range(3)),1.0)
        layer = baked.name
    mat = bpy.data.materials.new("review_palette")
    mat.use_nodes = True
    nodes = mat.node_tree.nodes
    nodes.clear()
    out = nodes.new("ShaderNodeOutputMaterial")
    attr = nodes.new("ShaderNodeVertexColor")
    attr.layer_name = layer
    if probe is None:
        diffuse = nodes.new("ShaderNodeBsdfDiffuse")
        mat.node_tree.links.new(attr.outputs["Color"],diffuse.inputs["Color"])
    else:
        diffuse = nodes.new("ShaderNodeEmission")
        mat.node_tree.links.new(attr.outputs["Color"],diffuse.inputs["Color"])
    mat.node_tree.links.new(diffuse.outputs[0],out.inputs["Surface"])
    mesh.materials.clear()
    mesh.materials.append(mat)


def import_character(row, offset=(0,0,0), yaw=0.0, probe=None):
    """Import the GLB, moving only its root so the rig and mesh stay aligned."""
    before = set(bpy.data.objects)
    bpy.ops.import_scene.gltf(filepath=str(MODEL_ROOT/row["file"]),disable_bone_shape=True)
    new = [ob for ob in bpy.data.objects if ob not in before]
    for ob in new:
        if ob.type == "ARMATURE":
            for track in ob.animation_data.nla_tracks:
                track.mute = True
            reset_pose(ob)
    for ob in new:
        if ob.parent is None or ob.parent not in new:
            ob.location += Vector(offset)
            orientation = ob.rotation_quaternion.copy() if ob.rotation_mode == "QUATERNION" else ob.rotation_euler.to_quaternion()
            ob.rotation_mode = "QUATERNION"
            ob.rotation_quaternion = Quaternion((0,0,1),math.radians(yaw)) @ orientation
        if ob.type == "MESH":
            material(ob,probe)
    bpy.context.view_layer.update()
    return [ob for ob in new if ob.type == "MESH"]


def text(content, x, z, size=.075):
    """Place a short label facing the front camera below the standing figures."""
    ob = label(content,(x,-.40,z),size,bpy.context.scene.collection,plain("review_text",(.66,.76,.81)))
    ob.rotation_euler = (math.pi/2,0,0)
    return ob


def capture(path):
    """Write one PNG, with a camera fitted to the actual imported geometry."""
    bpy.context.scene.render.filepath = str(path)
    bpy.ops.render.render(write_still=True)
    print(f"[crew render] wrote {path}")


def lineup(rows, shots, samples, filename="lineup.png"):
    """Six departments together under the same light and at their true heights."""
    scene(samples)
    objects = []
    for i,row in enumerate(rows):
        x = (i-(len(rows)-1)/2)*1.40
        objects += import_character(row,(x,0,0))
        text(row["department"].capitalize(),x,2.10)
        text(f"{row['triangles']} triangles",x,1.99,.060)
    camera(objects,(2000,820),margin=1.27,view=Vector((0,-1,.035)).normalized())
    capture(shots/filename)


def turnaround(row, shots, samples):
    """Front, both sides, back and three-quarter of the exported rest pose."""
    scene(samples)
    objects = []
    for i,(name,yaw) in enumerate((("Front",0),("Left",90),("Back",180),("Right",270),("Three-quarter",-30))):
        x = (i-2)*1.11
        objects += import_character(row,(x,0,0),yaw)
        text(name,x,2.04)
    camera(objects,(1800,900),margin=1.21,view=Vector((0,-1,.025)).normalized())
    capture(shots/f"{row['id']}-turnaround.png")


def face_closeup(row, shots, samples):
    """Close-up showing actual geometric iris and pupil rings with simple features."""
    scene(samples)
    objects = import_character(row)
    ob = objects[0]
    tag = ob.data.uv_layers.active
    allowed = set(row["parts"][part] for part in row["parts"] if part.startswith("eye_") or part in ("head_skin","nose","mouth") or part.startswith("brow_"))
    points = [ob.matrix_world @ ob.data.vertices[loop.vertex_index].co for polygon in ob.data.polygons for li in polygon.loop_indices for loop in [ob.data.loops[li]] if round(tag.data[li].uv.x) in allowed]
    center = Vector((0,0,(min(p.z for p in points)+max(p.z for p in points))/2))
    cam = camera(objects,(900,900),view=Vector((0,-1,0)))
    cam.location = center+Vector((0,-10,0))
    cam.data.ortho_scale = (max(p.x for p in points)-min(p.x for p in points))*1.35
    cam.data.shift_x = cam.data.shift_y = 0
    capture(shots/"eyes-closeup.png")


def probe_sheet(row, table, shots, samples):
    """Same albedo and geometry under three illustrative six-face probe states."""
    scene(samples)
    objects = []
    for i,state in enumerate(("normal","red_alert","emergency")):
        x = (i-1)*1.20
        objects += import_character(row,(x,0,0),probe=table["preview_cubes_linear"][state])
        text(state.replace("_"," ").capitalize(),x,2.04)
    camera(objects,(1500,1000),margin=1.19,view=Vector((0,-1,.025)).normalized())
    capture(shots/"probe-states.png")


def detail_sheet(row, shots, samples, name, prefixes, height_range=(0,1)):
    """Frame actual tagged geometry for front, three-quarter and side repairs."""
    scene(samples)
    objects,points = [],[]
    for i,yaw in enumerate((0,-35,90)):
        imported = import_character(row,((i-1)*1.05,0,0),yaw)
        objects += imported
        for ob in imported:
            tags = ob.data.uv_layers.active
            allowed = {tag for part,tag in row["parts"].items() if part.startswith(prefixes)}
            for face in ob.data.polygons:
                if round(tags.data[face.loop_indices[0]].uv.x) not in allowed:
                    continue
                for vi in face.vertices:
                    point = ob.matrix_world @ ob.data.vertices[vi].co
                    if height_range[0]*row["height_m"] <= point.z <= height_range[1]*row["height_m"]:
                        points.append(point)
    center = Vector(((min(p.x for p in points)+max(p.x for p in points))/2,0,(min(p.z for p in points)+max(p.z for p in points))/2))
    cam = camera(objects,(1600,640),view=Vector((0,-1,0)))
    cam.location = center+Vector((0,-10,0))
    cam.data.ortho_scale = max(max(p.x for p in points)-min(p.x for p in points),(max(p.z for p in points)-min(p.z for p in points))*2.5)*1.12
    cam.data.shift_x = cam.data.shift_y = 0
    capture(shots/f"{name}.png")


def main():
    """Produce a small review package, always from the exported files."""
    args = sys.argv[sys.argv.index("--")+1:] if "--" in sys.argv else ([] if "-P" in sys.argv or "--python" in sys.argv else sys.argv[1:])
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--samples",type=int,default=32)
    parser.add_argument("--shots",type=Path,default=SHOTS)
    parser.add_argument("--only",help="comma-separated character ids for turnarounds")
    parser.add_argument("--quick",action="store_true",help="lineup and eyes only")
    parser.add_argument("--lineups-only",action="store_true",help="only the male and female department lineups")
    parser.add_argument("--probe-only",action="store_true",help="only the offline lighting comparison")
    parser.add_argument("--models",type=Path,default=OUTPUT)
    options = parser.parse_args(args)
    global MODEL_ROOT
    MODEL_ROOT = options.models.resolve()
    options.shots = options.shots.resolve()
    table,_ = load_table()
    rows = json.loads((MODEL_ROOT/"characters.json").read_text())["characters"]
    options.shots.mkdir(parents=True,exist_ok=True)
    if options.probe_only:
        probe_sheet(rows[0],table,options.shots,options.samples)
        return
    lineup([row for row in rows if row["sex"]=="male"],options.shots,options.samples)
    lineup([row for row in rows if row["sex"]=="female"],options.shots,options.samples,"lineup-female.png")
    if options.lineups_only:
        return
    face_closeup(rows[0],options.shots,options.samples)
    if not options.quick:
        for row in rows:
            if not options.only or row["id"] in options.only.split(","):
                turnaround(row,options.shots,options.samples)
        probe_sheet(rows[0],table,options.shots,options.samples)
        detail_sheet(next(row for row in rows if row["id"]=="medical_02"),options.shots,options.samples,"hair-clearance",("head_skin","hair_","ear_"))
        detail_sheet(next(row for row in rows if row["id"]=="engineering_02"),options.shots,options.samples,"waist-clearance",("garment_",),(.32,.62))
        detail_sheet(rows[0],options.shots,options.samples,"boots",("boot_",))


if __name__ == "__main__":
    main()
