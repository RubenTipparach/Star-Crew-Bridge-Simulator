"""Project authored ship-local seams and livery on the real hull, then bake its UV guide.

Uses the exterior builder's actual mesh and UVs. This authors the layout only; imagegen
enhances the resulting surface finish. The editable blend retains the projection shader.
Run with Blender's headless Python, with the same dependencies as build_ship_exteriors.py.
"""
import json
from pathlib import Path
import sys

import bpy
import bmesh
import numpy as np
from mathutils import Vector

sys.path.insert(0, str(Path(__file__).resolve().parent))
import build_ship_exteriors as exterior
from hs_kit import lin, P


def validate_projection(mesh, image, spec):
    """Read the baked mask at real surface samples and compare its stripe membership."""
    width, height = image.size
    pixels = np.empty(width * height * 4, dtype=np.float32)
    image.pixels.foreach_get(pixels)
    pixels = pixels.reshape(height, width, 4)
    counts = {"stripe": 0, "plate": 0}
    failures = []
    for face in mesh.polygons:
        position = sum((P(mesh.vertices[i].co) for i in face.vertices), Vector()) / len(face.vertices)
        field = position.z - abs(position.x) * spec["stripe_axis"][0] + position.y * spec["stripe_axis"][1]
        distances = [abs(field - edge) for interval in spec["stripe_intervals_m"] for edge in interval]
        # The few texels touching an edge are allowed to filter across that boundary.
        if min(distances) < .3 or abs(abs(position.x) - spec["stripe_min_abs_x_m"]) < .3:
            continue
        uv = sum((mesh.uv_layers.active.data[i].uv for i in face.loop_indices), Vector((0, 0))) / len(face.loop_indices)
        pixel = pixels[min(height - 1, max(0, int(uv.y * height))),
                       min(width - 1, max(0, int(uv.x * width)))]
        expected = abs(position.x) > spec["stripe_min_abs_x_m"] and any(a < field < b for a, b in spec["stripe_intervals_m"])
        if (pixel[0] > .5) != expected:
            failures.append([list(position), list(uv), float(pixel[0]), expected])
        counts["stripe" if expected else "plate"] += 1
    assert not failures, f"Projected stripe mask disagrees with {len(failures)} surface samples: {failures[:5]}"
    assert min(counts.values()) > 0, "Projection validation sampled no stripe or no plate"
    return counts


def projection_material(spec):
    """Construct one continuous 3D field, independent of the receiving UV islands."""
    mat = bpy.data.materials.new("Hull paint: ship-local projected layout")
    mat.use_nodes = True
    nodes, links = mat.node_tree.nodes, mat.node_tree.links
    nodes.clear()

    def feed(value, socket):
        if hasattr(value, "node"):
            links.new(value, socket)
        else:
            socket.default_value = value

    def math(op, a, b=0):
        node = nodes.new("ShaderNodeMath")
        node.operation = op
        feed(a, node.inputs[0])
        feed(b, node.inputs[1])
        return node.outputs[0]

    def mix(factor, a, b):
        node = nodes.new("ShaderNodeMixRGB")
        feed(factor, node.inputs[0])
        feed(a, node.inputs[1])
        feed(b, node.inputs[2])
        return node.outputs[0]

    def color(rgb):
        return (*lin(rgb), 1)

    geometry = nodes.new("ShaderNodeNewGeometry")
    position = nodes.new("ShaderNodeSeparateXYZ")
    links.new(geometry.outputs["Position"], position.inputs[0])
    x = math("ABSOLUTE", position.outputs["X"])
    y = position.outputs["Z"]
    z = math("MULTIPLY", position.outputs["Y"], -1)
    normal = nodes.new("ShaderNodeSeparateXYZ")
    links.new(geometry.outputs["Normal"], normal.inputs[0])
    wall = math("LESS_THAN", math("ABSOLUTE", normal.outputs["Z"]), .70)

    seam = 0
    for axis, stations in [(y, spec["wall_seams_y_m"]), (z, spec["station_seams_z_m"])]:
        for station in stations:
            distance = math("ABSOLUTE", math("SUBTRACT", axis, station))
            seam = math("MAXIMUM", seam, math("LESS_THAN", distance, spec["seam_width_m"] / 2))
    seam = math("MULTIPLY", seam, wall)

    # The same oblique plane crosses roof, bevel and side; there is no per-face offset.
    field = math("ADD", math("SUBTRACT", z, math("MULTIPLY", x, spec["stripe_axis"][0])),
                 math("MULTIPLY", y, spec["stripe_axis"][1]))
    stripe = 0
    for lo, hi in spec["stripe_intervals_m"]:
        band = math("MULTIPLY", math("GREATER_THAN", field, lo), math("LESS_THAN", field, hi))
        stripe = math("MAXIMUM", stripe, band)
    stripe = math("MULTIPLY", stripe, math("GREATER_THAN", x, spec["stripe_min_abs_x_m"]))

    original = nodes.new("ShaderNodeTexImage")
    original.image = bpy.data.images.load(str(exterior.ROOT / spec["source_paint"]))
    original.label = "Retained roof, underside and registration"
    rgb = nodes.new("ShaderNodeSeparateColor")
    links.new(original.outputs["Color"], rgb.inputs[0])
    old_cyan = math("MULTIPLY", math("LESS_THAN", rgb.outputs["Red"], .15),
                    math("MULTIPLY", math("GREATER_THAN", rgb.outputs["Green"], .22),
                         math("GREATER_THAN", rgb.outputs["Blue"], .30)))
    base = mix(math("MAXIMUM", wall, old_cyan), original.outputs["Color"], color(spec["plate_srgb"]))
    painted = mix(stripe, mix(seam, base, color(spec["seam_srgb"])), color(spec["stripe_srgb"]))
    emit = nodes.new("ShaderNodeEmission")
    links.new(painted, emit.inputs["Color"])
    output = nodes.new("ShaderNodeOutputMaterial")
    links.new(emit.outputs[0], output.inputs["Surface"])
    masks = nodes.new("ShaderNodeCombineColor")
    links.new(stripe, masks.inputs["Red"])
    links.new(seam, masks.inputs["Green"])
    masks.inputs["Blue"].default_value = 1
    return mat, emit, painted, masks.outputs[0], original.image


def main():
    """Build the source hull once and retain its editable projection and two UV bakes."""
    root = exterior.ROOT
    spec = json.loads((root / "data/ships/tern/hull_paint_projection.json").read_text())
    config = json.loads(exterior.CONFIG.read_text())
    config["hero_atlas"]["texture"] = "tern_hull_cobalt"
    ship = next(s for s in config["ships"] if s["id"] == "tern")
    ship["livery_srgb"] = spec["stripe_srgb"]
    bpy.ops.wm.read_factory_settings(use_empty=True)
    exterior.setup_materials(config)
    prop, _, _ = exterior.build(ship, 0)
    objects = exterior.prepare(prop, ship, config)
    exterior.export_hero_guide(objects, config)
    hull = next(o for o in objects if o.name == "tern.hull")
    target = hull.copy()
    target.data = hull.data.copy()
    prop.coll.objects.link(target)
    target.name = "Hull projection bake surface"
    bm = bmesh.new()
    bm.from_mesh(target.data)
    bm.faces.ensure_lookup_table()
    projected_faces = hull.data.attributes["HullProjection"].data
    remove = [f for f in bm.faces if not projected_faces[f.index].value]
    bmesh.ops.delete(bm, geom=remove, context="FACES")
    bm.to_mesh(target.data)
    bm.free()
    mat, emit, painted, masks, original = projection_material(spec)
    target.data.materials.clear()
    target.data.materials.append(mat)
    for face in target.data.polygons:
        face.material_index = 0
    for ob in objects:
        ob.hide_render = True
    target.hide_render = False
    bpy.ops.object.select_all(action="DESELECT")
    target.select_set(True)
    bpy.context.view_layer.objects.active = target
    scene = bpy.context.scene
    scene.render.engine = "CYCLES"
    scene.cycles.device = "CPU"
    scene.cycles.samples = 1
    scene.cycles.use_denoising = False
    scene.render.bake.use_selected_to_active = False
    scene.render.bake.margin = spec["bake_margin_px"]
    scene.render.bake.use_clear = False
    scene.view_settings.view_transform = "Standard"
    scene.view_settings.look = "None"
    folder = root / "tools/materials/sources"
    unenhanced = bpy.data.images.new("Tern projected linework before enhancement",
                                     width=spec["guide_px"], height=spec["guide_px"])
    unenhanced.colorspace_settings.name = "sRGB"
    mask = bpy.data.images.new("Tern projected region masks", width=spec["guide_px"], height=spec["guide_px"])
    mask.colorspace_settings.name = "Non-Color"
    bake = mat.node_tree.nodes.new("ShaderNodeTexImage")
    mat.node_tree.nodes.active = bake
    # Seed untouched fittings through Blender's own image sampling. A loaded image copy
    # can retain its disk dimensions during baking, so the target is a generated image.
    bpy.ops.mesh.primitive_plane_add(size=2, location=(0, 0, 200))
    seed = bpy.context.object
    seed_mat = bpy.data.materials.new("Retained atlas background")
    seed_mat.use_nodes = True
    seed.data.materials.append(seed_mat)
    sn, sl = seed_mat.node_tree.nodes, seed_mat.node_tree.links
    sn.clear()
    source = sn.new("ShaderNodeTexImage")
    source.image = original
    background = sn.new("ShaderNodeEmission")
    sl.new(source.outputs["Color"], background.inputs["Color"])
    result = sn.new("ShaderNodeOutputMaterial")
    sl.new(background.outputs[0], result.inputs["Surface"])
    seed_target = sn.new("ShaderNodeTexImage")
    seed_target.image = unenhanced
    sn.active = seed_target
    bpy.ops.object.select_all(action="DESELECT")
    seed.select_set(True)
    bpy.context.view_layer.objects.active = seed
    bpy.ops.object.bake(type="EMIT")
    bpy.data.objects.remove(seed, do_unlink=True)
    target.select_set(True)
    bpy.context.view_layer.objects.active = target
    for image, socket, filename in [(unenhanced, painted, "tern-projected-layout.png"),
                                     (mask, masks, "tern-projected-masks.png")]:
        mat.node_tree.links.new(socket, emit.inputs["Color"])
        bake.image = image
        bpy.ops.object.bake(type="EMIT")
        image.filepath_raw = str(folder / filename)
        image.file_format = "PNG"
        image.save()
    counts = validate_projection(target.data, mask, spec)
    (folder / "tern-projection-validation.json").write_text(json.dumps({
        "source": "data/ships/tern/hull_paint_projection.json", "samples": counts,
        "mismatches": 0, "boundary_clearance_m": .3,
    }, indent=2) + "\n")
    mat.node_tree.links.new(painted, emit.inputs["Color"])
    bake.image = unenhanced
    for image in bpy.data.images:
        if image.source == "FILE":
            image.pack()
    bpy.context.preferences.filepaths.save_version = 0
    bpy.ops.wm.save_as_mainfile(filepath=str(root / "assets/models/exteriors/hull-paint-projection.blend"))
    print("Projected hull linework and region masks baked from the actual 3D surface.", flush=True)


if __name__ == "__main__":
    main()
