"""Bake the shoulder channel's blunt end from the actual hull into its existing UVs.

Only a geometric region mask is generated here: R coverage, G forward, B port.
The native Material Maker graph
reuses the approved painted armor under that mask. No runtime geometry is added.
Run through headless Blender with the exterior builder's Python dependencies.
"""
import json
from pathlib import Path
import sys

import bpy
import bmesh

sys.path.insert(0, str(Path(__file__).resolve().parent))
import build_ship_exteriors as exterior


def main():
    root = exterior.ROOT
    spec = json.loads((root / "data/ships/tern/hull_paint_projection.json").read_text())
    cap = spec["shoulder_termination"]
    config = json.loads(exterior.CONFIG.read_text())
    ship = next(s for s in config["ships"] if s["id"] == "tern")
    bpy.ops.wm.read_factory_settings(use_empty=True)
    exterior.setup_materials(config)
    prop, _, _ = exterior.build(ship, 0)
    objects = exterior.prepare(prop, ship, config)
    target = next(o for o in objects if o.name == "tern.hull")
    bm = bmesh.new()
    bm.from_mesh(target.data)
    bm.faces.ensure_lookup_table()
    projected = target.data.attributes["HullProjection"].data
    bmesh.ops.delete(bm, geom=[f for f in bm.faces if not projected[f.index].value], context="FACES")
    bm.to_mesh(target.data)
    bm.free()
    mat = bpy.data.materials.new("Shoulder termination: ship-local region")
    mat.use_nodes = True
    nodes, links = mat.node_tree.nodes, mat.node_tree.links
    nodes.clear()

    def math(op, a, b=0):
        node = nodes.new("ShaderNodeMath")
        node.operation = op
        for value, socket in zip((a, b), node.inputs):
            if hasattr(value, "node"):
                links.new(value, socket)
            else:
                socket.default_value = value
        return node.outputs[0]

    geometry = nodes.new("ShaderNodeNewGeometry")
    position = nodes.new("ShaderNodeSeparateXYZ")
    normal = nodes.new("ShaderNodeSeparateXYZ")
    links.new(geometry.outputs["Position"], position.inputs[0])
    links.new(geometry.outputs["Normal"], normal.inputs[0])
    x = math("ABSOLUTE", position.outputs["X"])
    y = position.outputs["Z"]
    z = math("MULTIPLY", position.outputs["Y"], -1)
    station_mask = 0
    for lo, hi in cap["z_ranges_m"]:
        interval = math("MULTIPLY", math("GREATER_THAN", z, lo), math("LESS_THAN", z, hi))
        station_mask = math("MAXIMUM", station_mask, interval)
    mask = math("GREATER_THAN", x, cap["abs_x_min_m"])
    for condition in [math("GREATER_THAN", y, cap["y_min_m"]),
                      station_mask,
                      math("GREATER_THAN", normal.outputs["Z"], .65)]:
        mask = math("MULTIPLY", mask, condition)
    channels = nodes.new("ShaderNodeCombineColor")
    links.new(mask, channels.inputs["Red"])
    forward = math("MULTIPLY", mask, math("GREATER_THAN", z, cap["z_ranges_m"][1][0]))
    port = math("MULTIPLY", mask, math("GREATER_THAN", position.outputs["X"], 0))
    links.new(forward, channels.inputs["Green"])
    links.new(port, channels.inputs["Blue"])
    emit = nodes.new("ShaderNodeEmission")
    links.new(channels.outputs[0], emit.inputs["Color"])
    output = nodes.new("ShaderNodeOutputMaterial")
    links.new(emit.outputs[0], output.inputs["Surface"])
    target.data.materials.clear()
    target.data.materials.append(mat)
    for face in target.data.polygons:
        face.material_index = 0
    image = bpy.data.images.new("Projected shoulder armor termination", width=spec["guide_px"], height=spec["guide_px"])
    image.colorspace_settings.name = "Non-Color"
    bake = nodes.new("ShaderNodeTexImage")
    bake.image = image
    nodes.active = bake
    bpy.ops.object.select_all(action="DESELECT")
    for ob in objects:
        ob.hide_render = ob != target
    target.select_set(True)
    bpy.context.view_layer.objects.active = target
    scene = bpy.context.scene
    scene.render.engine = "CYCLES"
    scene.cycles.device = "CPU"
    scene.cycles.samples = 1
    scene.render.bake.margin = spec["bake_margin_px"]
    bpy.ops.object.bake(type="EMIT")
    image.filepath_raw = str(root / "tools/materials/sources/tern-shoulder-armor-mask.png")
    image.file_format = "PNG"
    image.save()
    for source in bpy.data.images:
        if source.source == "FILE":
            source.pack()
    bpy.context.preferences.filepaths.save_version = 0
    bpy.ops.wm.save_as_mainfile(filepath=str(root / "assets/models/exteriors/shoulder-armor-projection.blend"))
    print("Shoulder termination projected on both actual hull surfaces.", flush=True)


if __name__ == "__main__":
    main()
