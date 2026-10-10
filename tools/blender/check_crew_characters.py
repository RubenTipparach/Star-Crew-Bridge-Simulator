"""Exercise character requirements against exported and deliberately broken GLBs.

These artifact regression checks catch excess geometry, incorrect eye geometry,
skin outside clothing and extra materials. They run in headless Blender because
the generator and garment BVHs require bpy. No invalid asset reaches the review
folder. Run with the same Blender flags as build_crew_characters.py.
"""
import copy
import json
from pathlib import Path
import sys
import tempfile

import bmesh
from mathutils import Vector

sys.path.insert(0,str(Path(__file__).resolve().parent))
from build_crew_characters import build, export, inspect, load_table, OUTPUT, ROOT


def rejection(title, expected, change_table=None, change_mesh=None):
    """Construct an actual invalid file and require a clear artifact rejection."""
    table,departments = load_table()
    table = copy.deepcopy(table)
    row = table["characters"][0]
    if change_table:
        change_table(table)
    ob,armature,parts = build(row,table,departments[row["department"]])
    if change_mesh:
        change_mesh(ob,parts)
    with tempfile.TemporaryDirectory(prefix="starcrew-invalid-character-") as directory:
        path = Path(directory)/f"{row['id']}.glb"
        export(ob,armature,path)
        try:
            inspect(path,row,table,parts)
        except RuntimeError as error:
            message = str(error)
            if expected not in message or row["id"] not in message:
                raise AssertionError(f"{title}: wrong rejection: {message}") from error
            print(f"[crew check] PASS {title}: {message}")
            return {"check":title,"result":"pass","rejection":message}
        raise AssertionError(f"{title}: accepted invalid exported geometry")


def move_covered_skin(ob, parts):
    """Move covered torso skin out through its shirt without touching garment data."""
    uv = ob.data.uv_layers.active
    vertices = {ob.data.loops[li].vertex_index for face in ob.data.polygons for li in face.loop_indices if round(uv.data[li].uv.x) == parts["covered_torso"]}
    for vi in vertices:
        ob.data.vertices[vi].co.x += 0.45
    ob.data.update()


def extra_material(ob, _parts):
    """Force an extra GLB primitive rather than merely adding an unused slot."""
    second = ob.data.materials[0].copy()
    second.name = "forbidden_second_material"
    ob.data.materials.append(second)
    ob.data.polygons[0].material_index = 1


def tagged_vertices(ob, parts, name):
    """Find artifact-tagged source vertices without assuming their order."""
    uv = ob.data.uv_layers.active
    return {ob.data.loops[li].vertex_index for face in ob.data.polygons for li in face.loop_indices if round(uv.data[li].uv.x)==parts[name]}


def hair_into_scalp(ob, parts):
    """Reproduce the rejected scalp cutting through the hair cap."""
    for vi in tagged_vertices(ob,parts,"hair_cap"):
        ob.data.vertices[vi].co.z -= .045
    ob.data.update()


def trousers_through_hem(ob, parts):
    """Displace the trouser waistband out through the visible tunic wall."""
    vertices = tagged_vertices(ob,parts,"garment_trousers")
    top = max(ob.data.vertices[vi].co.z for vi in vertices)
    for vi in vertices:
        if abs(ob.data.vertices[vi].co.z-top)<1e-5:
            ob.data.vertices[vi].co.y -= .10
    ob.data.update()


def disconnected_tunic(ob, parts):
    """Add a closed detached garment island, preserving the single material."""
    bm = bmesh.new()
    bm.from_mesh(ob.data)
    verts = bmesh.ops.create_cube(bm,size=.05)["verts"]
    uv = bm.loops.layers.uv.active
    deform = bm.verts.layers.deform.active
    for vertex in verts:
        vertex.co += Vector((.6,0,1))
        vertex[deform][ob.vertex_groups["chest"].index] = 1
    for face in {face for vertex in verts for face in vertex.link_faces}:
        for loop in face.loops:
            loop[uv].uv = (parts["garment_tunic"],0)
    bm.to_mesh(ob.data)
    bm.free()


def main():
    """Validate all shipped review files, then verify the requested failure paths."""
    table,_ = load_table()
    manifest = json.loads((OUTPUT/"characters.json").read_text())
    lookup = {row["id"]:row for row in table["characters"]}
    verified = []
    for artifact in manifest["characters"]:
        facts = inspect(OUTPUT/artifact["file"],lookup[artifact["id"]],table,artifact["parts"])
        if any(facts[key] != artifact[key] for key in facts):
            raise AssertionError(f"{artifact['id']}: manifest disagrees with the actual GLB")
        verified.append(artifact["id"])
    failures = [
        rejection("more than 3000 triangles names the character and count","over budget",lambda table:table["topology"].update(head_sides=80)),
        rejection("mesh iris outside the requested range is rejected","measured iris",lambda table:table["face"].update(iris_width_eye_ratio=.62)),
        rejection("mesh pupil with the wrong ratio is rejected","pupil",lambda table:table["face"].update(pupil_width_iris_ratio=.5)),
        rejection("covered skin protruding through a tunic is rejected","skin outside its garment",change_mesh=move_covered_skin),
        rejection("second material and primitive are rejected","materials",change_mesh=extra_material),
        rejection("scalp clipping through hair is rejected","hair intersects scalp",change_mesh=hair_into_scalp),
        rejection("trousers clipping through the hem are rejected","trousers intersect tunic",change_mesh=trousers_through_hem),
        rejection("closed but detached garment geometry is rejected","disconnected garment",change_mesh=disconnected_tunic),
    ]
    report = {"schema":"starcrew.character-checks/1","verified_artifacts":verified,
        "checks":failures,"scope":"Exported review assets and rejection cases. Posed clothing and source IK controls are checked separately in rig-validation.json. Engine adoption, live deck probes and Pi measurements remain pending."}
    target = ROOT/"docs/screenshots/crew-characters/validation.json"
    target.parent.mkdir(parents=True,exist_ok=True)
    target.write_text(json.dumps(report,indent=2)+"\n",encoding="utf-8")
    print(f"[crew check] PASS: {len(verified)} real artifacts and {len(failures)} invalid-artifact cases")


if __name__ == "__main__":
    main()
