"""Export actual nacelle UVs and 3D plane intersections before generated paint enhancement.

The PNGs are geometric drawing guides, not surface artwork. Both nacelles share a
perimeter unwrap; station lines originate on their delivered 3D triangles.
Run with the same headless Blender environment as build_ship_exteriors.py.
"""
import json
from pathlib import Path
import sys

import bpy
from mathutils import Vector
from PIL import Image, ImageDraw

sys.path.insert(0, str(Path(__file__).resolve().parent))
import build_ship_exteriors as exterior


def main():
    """Build the source, project station planes and save an inspectable GLB and UV guides."""
    root = exterior.ROOT
    config = json.loads(exterior.CONFIG.read_text())
    ship = next(s for s in config["ships"] if s["id"] == "tern")
    livery = next(v for v in config["liveries"] if v["id"] == config["default_livery"])
    config["hero_atlas"]["texture"] = livery["texture"]
    ship["livery_srgb"] = livery["accent_srgb"]
    atlas = config["nacelle_atlas"]
    # Bootstrap the guide without requiring its future enhanced artwork.
    atlas["texture"] = "exterior_surfaces"
    bpy.ops.wm.read_factory_settings(use_empty=True)
    exterior.setup_materials(config)
    p, _, _ = exterior.build(ship, 0)
    objects = exterior.prepare(p, ship, config)
    faces, lines = [], []
    for ob in objects:
        for face in ob.data.polygons:
            if ob.data.materials[face.material_index].name != "tern_nacelle":
                continue
            points = [exterior.P(ob.data.vertices[i].co) for i in face.vertices]
            uvs = [ob.data.uv_layers.active.data[i].uv.copy() for i in face.loop_indices]
            region = exterior.nacelle_region(ob, exterior.P(face.normal), ship)
            faces.append({"assembly": ob.name, "region": region,
                          "points_m": [list(v) for v in points], "uv": [list(v) for v in uvs]})
            if region not in ("shell", "support"):
                continue
            for station in atlas["station_seams_z_m"]:
                hits = []
                for i in range(len(points)):
                    j = (i + 1) % len(points)
                    a, b = points[i], points[j]
                    if abs(a.z - b.z) < 1e-6:
                        continue
                    t = (station - a.z) / (b.z - a.z)
                    if 0 <= t <= 1:
                        hits.append(list(uvs[i].lerp(uvs[j], t)))
                if len(hits) == 2:
                    lines.append({"assembly": ob.name, "station_z_m": station, "uv": hits})
    folder = root / "tools/materials/sources"
    data = {"size_px": atlas["size_px"], "faces": faces, "projected_lines": lines}
    (folder / "tern-nacelle-uv-layout.json").write_text(json.dumps(data, indent=1) + "\n")
    width, height = atlas["size_px"]
    image = Image.new("RGB", (width, height), (22, 26, 29))
    draw = ImageDraw.Draw(image)

    def pixel(uv):
        return (round(uv[0] * (width - 1)), round((1 - uv[1]) * (height - 1)))

    palette = {"shell": (211, 211, 204), "support": (205, 207, 201),
               "rail": (66, 73, 76), "fore": (71, 77, 80),
               "aft": (63, 67, 69), "fittings": (145, 151, 150)}
    for face in faces:
        draw.polygon([pixel(v) for v in face["uv"]], fill=palette[face["region"]])
    # Draw the real plane/triangle intersections onto the receiving UV surface.
    line_width = max(1, round(atlas["seam_width_m"] * height * .96 / ship["nacelle"]["length_m"]))
    for line in lines:
        draw.line([pixel(v) for v in line["uv"]], fill=(69, 77, 79), width=line_width)
    layout_path = folder / "tern-nacelle-projected-layout.png"
    image.save(layout_path)
    wire = image.copy()
    wd = ImageDraw.Draw(wire)
    for face in faces:
        points = [pixel(v) for v in face["uv"]]
        wd.line(points + points[:1], fill=(85, 112, 128), width=1)
    wire.save(folder / "tern-nacelle-uv-wireframe.png")
    material = bpy.data.materials["tern_nacelle"]
    node = next(n for n in material.node_tree.nodes if n.type == "TEX_IMAGE")
    node.image = bpy.data.images.load(str(layout_path))
    path = root / "build/exterior-tools/nacelle-projection.glb"
    exterior.export(objects, "tern", path)
    facts = exterior.verify(path, 12000)
    facts["projected_segments"] = len(lines)
    facts["shell_longitudinal_px_per_m"] = height * .96 / ship["nacelle"]["length_m"]
    path.with_suffix(".json").write_text(json.dumps(facts, indent=2) + "\n")
    print(json.dumps(facts, indent=2), flush=True)


if __name__ == "__main__":
    main()
