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


def flat_band_probes(mesh, spec):
    """Trace the actual upper skin at five X positions through each central band."""
    mesh.calc_loop_triangles()
    roofs = [tri for tri in mesh.loop_triangles if tri.normal.z > .9]
    traces = []
    for interval in spec["stripe_intervals_m"]:
        for x in np.linspace(-spec["stripe_flat_half_width_m"] + .5,
                             spec["stripe_flat_half_width_m"] - .5, 5):
            samples = []
            for z in np.linspace(interval[0] - 3, interval[1] + 5, 181):
                hits = []
                for tri in roofs:
                    verts = [mesh.vertices[i].co for i in tri.vertices]
                    a, b, c = [Vector((v.x, -v.y)) for v in verts]
                    ab, ac, ap = b-a, c-a, Vector((x, z))-a
                    det = ab.x*ac.y-ab.y*ac.x
                    if abs(det) < 1e-8:
                        continue
                    u = (ap.x*ac.y-ap.y*ac.x)/det
                    v = (ab.x*ap.y-ab.y*ap.x)/det
                    if min(u, v, 1-u-v) < -1e-6:
                        continue
                    weights = (1-u-v, u, v)
                    height = sum(w*p.z for w,p in zip(weights, verts))
                    uv = sum((w*mesh.uv_layers.active.data[i].uv for w,i in zip(weights, tri.loops)), Vector((0, 0)))
                    hits.append((height, list(uv)))
                if hits:
                    height, uv = max(hits, key=lambda hit: hit[0])
                    samples.append({"z_m": float(z), "y_m": height, "uv": uv})
            assert samples, "Flat-band probe missed the actual roof"
            traces.append({"interval_m": interval, "x_m": float(x), "samples": samples})
    return traces


def shared_edge_probes(mesh, spec):
    """Sample both receiving UV faces at authored seams crossing an actual shared edge."""
    edges = {}
    for face in mesh.polygons:
        loops = list(face.loop_indices)
        for ia, ib in zip(loops, loops[1:] + loops[:1]):
            va, vb = mesh.loops[ia].vertex_index, mesh.loops[ib].vertex_index
            key = tuple(sorted((va, vb)))
            pair = [mesh.uv_layers.active.data[i].uv.copy() for i in (ia, ib)]
            edges.setdefault(key, []).append(pair if va < vb else pair[::-1])
    probes = []
    for (ia, ib), faces in edges.items():
        if len(faces) != 2 or max((a-b).length for a,b in zip(*faces)) < .001:
            continue
        a, b = [P(mesh.vertices[i].co) for i in (ia, ib)]
        if abs(a.z-b.z) < 1:
            continue
        for z in spec["station_seams_z_m"]:
            t = (z-a.z)/(b.z-a.z)
            delta = .6/abs(b.z-a.z)
            if not delta < t < 1-delta:
                continue
            center = a.lerp(b, t)
            probes.append({"position_m": list(center), "station_m": z,
                           "profiles_uv": [[list(u.lerp(v, t + step*delta))
                                            for step in np.linspace(-1, 1, 81)] for u,v in faces]})
    assert probes, "No UV-island joins crossed by panel seams"
    return probes


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
        field = position.z - max(abs(position.x), spec["stripe_flat_half_width_m"]) * spec["stripe_axis"][0] + position.y * spec["stripe_axis"][1]
        distances = [abs(field - edge) for interval in spec["stripe_intervals_m"] for edge in interval]
        # The few texels touching an edge are allowed to filter across that boundary.
        if min(abs(position.y-edge) for edge in spec["equator_y_m"]) < .08:
            continue
        if min(distances) < .3:
            continue
        uv = sum((mesh.uv_layers.active.data[i].uv for i in face.loop_indices), Vector((0, 0))) / len(face.loop_indices)
        pixel = pixels[min(height - 1, max(0, int(uv.y * height))),
                       min(width - 1, max(0, int(uv.x * width)))]
        expected = (not spec["equator_y_m"][0] < position.y < spec["equator_y_m"][1]
                    and any(a < field < b for a, b in spec["stripe_intervals_m"]))
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

    # Station cuts are one 3D plane through roof, bevel, wall and keel.
    seam = 0
    for axis, stations, scope in [(y, spec["wall_seams_y_m"], wall),
                                  (z, spec["station_seams_z_m"], 1)]:
        for station in stations:
            distance = math("ABSOLUTE", math("SUBTRACT", axis, station))
            line = math("MULTIPLY", math("LESS_THAN", distance, spec["seam_width_m"] / 2), scope)
            seam = math("MAXIMUM", seam, line)

    # Flat center bridge with oblique outer shoulders, continuous over every hull face.
    shoulder = math("MAXIMUM", x, spec["stripe_flat_half_width_m"])
    field = math("ADD", math("SUBTRACT", z, math("MULTIPLY", shoulder, spec["stripe_axis"][0])),
                 math("MULTIPLY", y, spec["stripe_axis"][1]))
    stripe = 0
    for lo, hi in spec["stripe_intervals_m"]:
        band = math("MULTIPLY", math("GREATER_THAN", field, lo), math("LESS_THAN", field, hi))
        stripe = math("MAXIMUM", stripe, band)
    equator = math("MULTIPLY", math("GREATER_THAN", y, spec["equator_y_m"][0]),
                   math("LESS_THAN", y, spec["equator_y_m"][1]))
    stripe = math("MULTIPLY", stripe, math("SUBTRACT", 1, equator))

    original = nodes.new("ShaderNodeTexImage")
    original.image = bpy.data.images.load(str(exterior.ROOT / spec["source_paint"]))
    original.label = "Seam-free generated finish, retained machinery and registration"
    # The small front-cap island retains fittings outside the receiving hull UVs.
    # Its receiving patch needs neutral paint before adding the new projected seams.
    bow = math("GREATER_THAN", math("ABSOLUTE", normal.outputs["Y"]), .85)
    base = mix(bow, original.outputs["Color"], color(spec["plate_srgb"]))
    # Reuse the generated machinery finish in a narrow belt, at ship-local scale.
    belt_uv = nodes.new("ShaderNodeCombineXYZ")
    along = mix(bow, z, position.outputs["X"])
    tile = math("FRACT", math("DIVIDE", along, spec["equator_repeat_m"]))
    feed(math("ADD", .79, math("MULTIPLY", tile, .18)), belt_uv.inputs["X"])
    height = math("DIVIDE", math("SUBTRACT", y, spec["equator_y_m"][0]),
                  spec["equator_y_m"][1]-spec["equator_y_m"][0])
    feed(math("ADD", .06, math("MULTIPLY", height, .13)), belt_uv.inputs["Y"])
    belt_image = nodes.new("ShaderNodeTexImage")
    belt_image.image = original.image
    links.new(belt_uv.outputs[0], belt_image.inputs["Vector"])
    base = mix(equator, base, belt_image.outputs["Color"])
    painted = mix(seam, mix(stripe, base, color(spec["stripe_srgb"])), color(spec["seam_srgb"]))
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
    (folder / "tern-seam-probes.json").write_text(json.dumps({
        "profile_half_length_z_m": .6, "probes": shared_edge_probes(target.data, spec),
    }, indent=1) + "\n")
    (folder / "tern-flat-band-probes.json").write_text(json.dumps({
        "traces": flat_band_probes(target.data, spec),
    }, separators=(",", ":")) + "\n")
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
