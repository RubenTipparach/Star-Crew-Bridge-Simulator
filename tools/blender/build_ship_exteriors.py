"""Build the exterior design fleet from ship data with the repository's hard-surface kit.

This owns exterior assemblies and their GLB packaging, not decks or simulation. Hull lofts,
convex solids, CSG and cleanup come from existing builders. Named source assemblies remain in
the editable blend; exported assemblies merge into one mesh with at most nine material draws.

Run: blender -b --factory-startup -P tools/blender/build_ship_exteriors.py -- [--check]
Requires tools/materials/exterior_decals.py to have authored the registration sheet first.
"""
import argparse
import hashlib
import json
import math
from pathlib import Path
import struct
import sys
import tempfile
from types import SimpleNamespace

import bpy
import bmesh
import numpy as np
from mathutils import Vector
from mathutils.geometry import barycentric_transform

sys.path.insert(0, str(Path(__file__).resolve().parent))
from hs_kit import P, PROP_TO_BLENDER, ROLES, ROOT, Prop, _hull2, atlas_uv_at, clean, clip_polygon, finish, lin
from build_machinery_props import facet, loft, pipe, revolve
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from command_suite import patch_layout

ROOT = Path(ROOT)
OUT = ROOT / "assets/models/exteriors"
CONFIG = ROOT / "data/ships/exteriors.json"


def profile(width, top, bottom, chamfer):
    """Octagonal cross-section in the common ship-local frame, metres."""
    return [(-width + chamfer, bottom), (width - chamfer, bottom),
            (width, bottom + chamfer), (width, top - chamfer),
            (width - chamfer, top), (-width + chamfer, top),
            (-width, top - chamfer), (-width, bottom + chamfer)]


def sections(ship):
    """Read the player's existing hull or a companion's exterior-only sections."""
    if "layout" in ship:
        layout = json.loads((ROOT / ship["layout"]).read_text(encoding="utf-8"))
        if ship.get("layout_patch"):
            patch_layout(layout, json.loads((ROOT / ship["layout_patch"]).read_text(encoding="utf-8")))
        if ship.get("window_patch"):
            patch_layout(layout, json.loads((ROOT / ship["window_patch"]).read_text(encoding="utf-8")))
        rows = [[s[k] for k in ("z_m", "half_beam_m", "top_m", "bottom_m", "chamfer_m")]
                for s in layout["hull"]["sections"]]
        if ship.get("bow"):
            bow = ship["bow"]
            start = at(sorted(rows), bow["from_z_m"])
            reference = next(c for c in layout["compartments"] if c["id"] == bow["reference_room"])
            stations = {}
            for brush in reference["brushes"]:
                for x, z in brush["poly"]:
                    if z > bow["from_z_m"]:
                        station = bow["from_z_m"] + (z-bow["from_z_m"]) * bow["length_scale"]
                        stations[station] = max(stations.get(station, 0), abs(x)*bow["width_scale"])
            end = max(stations)
            rounded = [start]
            for z, width in sorted(stations.items()):
                t = (z-start[0])/(end-start[0])
                rounded.append([z, width, bow["top_m"], start[3]+t*(bow["bottom_m"]-start[3]), bow["chamfer_m"]])
            rows = [r for r in rows if r[0] < bow["from_z_m"]] + rounded
        return sorted(rows), layout
    return ship["sections_m"], None


def at(rows, z):
    """Interpolate the same section dimensions used to build the hull."""
    for a, b in zip(rows, rows[1:]):
        if a[0] <= z <= b[0]:
            t = (z - a[0]) / (b[0] - a[0])
            return [z] + [a[i] + t * (b[i] - a[i]) for i in range(1, 5)]
    raise ValueError(f"Hull station {z} m outside its sections")


def setup_materials(config):
    """Use original Material Maker RGB with baked vertex colour and opaque glTF materials."""
    for role in ROLES + ("registration", "tern_hull", "tern_rim"):
        row = config["materials"].get(role, config["materials"]["machinery"])
        if role in ("tern_hull", "tern_rim"):
            row = {"texture": config["hero_atlas"]["texture"], "tint_srgb": [1, 1, 1]}
        mat = bpy.data.materials.new(role)
        mat.use_nodes = True
        mat.use_backface_culling = True
        nodes = mat.node_tree.nodes
        nodes.clear()
        output = nodes.new("ShaderNodeOutputMaterial")
        colour = nodes.new("ShaderNodeVertexColor")
        colour.layer_name = "SunBake"
        mix = nodes.new("ShaderNodeMixRGB")
        mix.blend_type = "MULTIPLY"
        mix.inputs[0].default_value = 1
        mix.inputs[2].default_value = (1, 1, 1, 1)
        mat.node_tree.links.new(colour.outputs["Color"], mix.inputs[1])
        socket = mix.outputs[0]
        if row["texture"]:
            image = nodes.new("ShaderNodeTexImage")
            source = ROOT / "assets/textures" / (row["texture"] + ".png")
            image.image = bpy.data.images.load(str(source), check_existing=True)
            image.image.alpha_mode = "CHANNEL_PACKED"
            image.interpolation = "Closest"
            image.extension = "REPEAT"
            mat.node_tree.links.new(image.outputs["Color"], mix.inputs[2])
        # Colour directly into surface is Blender/glTF's shadeless material convention.
        mat.node_tree.links.new(socket, output.inputs["Surface"])
        mat.diffuse_color = (*lin(row["tint_srgb"]), 1)


def box(p, name, center, size, role):
    """A named box from its centre and SI dimensions."""
    return p.box(name, [center[i] - size[i] / 2 for i in range(3)],
                 [center[i] + size[i] / 2 for i in range(3)], role)


def fitted_hull(p, rows, layout, ship, name, role, inset=0, caps=None):
    """A hull solid whose forward upper section follows the actual command rooms."""
    rings = []
    for i, row in enumerate(rows):
        z = row[0] + (inset if i == 0 else -inset if i == len(rows) - 1 else 0)
        _, width, top, bottom, chamfer = at(rows, z)
        rings.append((z, profile(width - inset, top - inset, bottom + inset, chamfer)))
    body = loft(p, name, rings, [role] * 8, caps or (role, role))
    if not layout or not ship.get("command_hull"):
        return body
    config = ship["command_hull"]
    rooms = [c for c in layout["compartments"] if c["id"] in config["rooms"]]
    brushes = [b for c in rooms for b in c["brushes"]]
    outline = _hull2([tuple(q) for b in brushes for q in b["poly"]])
    clearance = config["clearance_m"] - inset
    # Offset the existing convex boundary by intersecting its shifted half-planes.
    def offset_outline(distance):
        padded = [(-100, -100), (100, -100), (100, 100), (-100, 100)]
        for a, b in zip(outline, outline[1:] + outline[:1]):
            nx, nz = b[1] - a[1], a[0] - b[0]
            length = math.hypot(nx, nz)
            padded = clip_polygon(padded, nx, nz, nx * a[0] + nz * a[1] + distance * length)
        return padded
    padded = offset_outline(clearance)
    floor = min(b["y"][0] for b in brushes)
    roof = max(b["y"][1] for b in brushes) + clearance
    lower = p.box(name + ".lower_envelope", (-100, -100, -100), (100, floor - inset, 100), role)
    outer_roof = max(b["y"][1] for b in brushes) + config["clearance_m"]
    slope = config["transition_m"] / (outer_roof - floor)
    plane = config["from_z_m"] + slope * outer_roof - inset * math.hypot(1, slope)
    aft = p.hull(name + ".aft_envelope", [(x, y, z) for x in (-100, 100) for y in (-100, 100)
                                          for z in (-200, plane - slope * y)], role)
    bevel = config["roof_chamfer_m"]
    crown = offset_outline(clearance - bevel)
    command = p.hull(name + ".command_envelope",
                     [(x, y, z) for x, z in padded for y in (floor - 1, roof - bevel)] +
                     [(x, roof, z) for x, z in crown], role)
    p.union(lower, name + ".join_envelopes", [aft, command])
    p.boolean(body, "INTERSECT", name + ".fit_command_rooms", [lower])
    if ship.get("bow"):
        p.chamfer(body, name + ".rounded_bow_rim", ship["bow"]["front_bevel_m"],
                  lambda mid, *args: mid.z > rows[-1][0] - inset - .002)
    # Actual CSG read-back: command and shortened-bow rooms must fit both hull solids.
    checked_brushes = brushes + [b for c in layout["compartments"]
                                if c["id"] in ship.get("bow", {}).get("contained_rooms", []) for b in c["brushes"]]
    direction = Vector((1, 0, 0))
    for brush in checked_brushes:
        for x, z in brush["poly"]:
            for y in brush["y"]:
                hit, _, normal, _ = body.ray_cast(PROP_TO_BLENDER @ Vector((x, y, z)), direction)
                assert hit and normal.dot(direction) > 0, f"{name}: fitted hull clips room at {(x, y, z)}"
    return body


def window_rim(p, rows, layout, portal, frame, depth, ship):
    """Cut a thin ring from the actual hull profile, including every bevel crossing."""
    width, height = portal["size_m"]
    rim = ship["window_rim_m"]
    outward = Vector(portal["normal"])
    name = portal["id"]
    role = ship.get("window_rim_role", "accent")
    ring = p.box(name + ".rim", (-width / 2 - rim, -height / 2 - rim, -0.1),
                 (width / 2 + rim, height / 2 + rim, depth + 3), role, frame)
    for operation, offset, label in [("INTERSECT", 0.015 + ship["window_rim_depth_m"], "rim_surface"),
                                     ("DIFFERENCE", 0.015, "rim_back")]:
        surface = fitted_hull(p, rows, layout, ship, name + "." + label, role)
        shift = PROP_TO_BLENDER.to_3x3() @ (outward * offset)
        for vertex in surface.data.vertices:
            vertex.co += shift
        surface.data.update()
        p.boolean(ring, operation, name + "." + label, [surface])
    opening = p.box(name + ".rim_opening", (-width / 2, -height / 2, -0.2),
                    (width / 2, height / 2, depth + 3.1), role, frame)
    p.cut(ring, name + ".open_rim", [opening])


def detail(p, rows, layout, level, command_from_z=None, nav_light_z=17):
    """Model player attachments from the existing system, mount and portal positions."""
    if not layout or level == 2:
        return
    for mount in layout["mounts"]:
        c, direction = Vector(mount["center_m"]), Vector(mount["facing"])
        if mount["kind"] == "turret":
            frame = facet(c, direction, (0, 0, 1))
            p.box(mount["id"] + ".base", (-1.6, -1.9, -1.1), (1.6, 1.9, 0.18), "machinery", frame)
            head = p.box(mount["id"] + ".head", (-1.15, -1.1, 0.0), (1.15, 1.1, 0.9), "trim", frame)
            if level == 0:
                p.chamfer(head, "turret_chamfer_" + mount["id"], 0.16, lambda *args: True)
            for sign in (-1, 1):
                p.box(mount["id"] + f".barrel_{sign}", (sign * 0.6 - 0.16, 0.2, 0.7),
                      (sign * 0.6 + 0.16, 3.8, 1.02), "machinery", frame)
        if mount["kind"] == "engine":
            x, y, z = c
            bell = revolve(p, mount["id"] + ".bell", [(1.3, z + 0.8), (1.8, z - 1.5), (1.7, z - 2.2)],
                           ["trim", "machinery"], "z", (x, y), sides=10 if level == 0 else 8)
            cut = revolve(p, mount["id"] + ".intake_cutter", [(1.44, z - 2.3), (0.94, z - 0.5)],
                          "machinery", "z", (x, y), sides=10 if level == 0 else 8)
            p.cut(bell, mount["id"] + ".hollow_bell", [cut])
            revolve(p, mount["id"] + ".glow", [(0.92, z - 0.53), (0.92, z - 0.49)],
                    "screen", "z", (x, y), sides=10 if level == 0 else 8)
        if mount["kind"] == "missile_tube":
            # Follow the tube axis until it leaves the exact loft's bow cross-section.
            x, y, z = c
            end = rows[-1][0]
            start = min(z, end)
            z = end
            for step in range(501):
                test_z = start + (end - start) * step / 500
                s = at(rows, test_z)
                if abs(x) > s[1] or y > s[2] or y < s[3]:
                    z = test_z
                    break
            revolve(p, mount["id"] + ".mouth", [(0.54, z - 0.4), (0.54, z + 0.14)],
                    "machinery", "z", (x, y), sides=8)
            revolve(p, mount["id"] + ".bore", [(0.37, z + 0.16), (0.37, z + 0.18)],
                    "hazard", "z", (x, y), sides=8)
    if level == 0:
        # Shoulder armour and a service spine break up the broad deck silhouette.
        for side in (-1, 1):
            for index, (za, zb) in enumerate([(-18, -2), (-2, 8), (8, 20)]):
                if command_from_z is not None:
                    zb = min(zb, command_from_z - 0.03)
                points = []
                for zz in (za, zb):
                    _, hw, top, _, chamfer = at(rows, zz)
                    for x in (hw - chamfer - 3.0, hw - chamfer - 0.55):
                        for yy in (top + 0.06, top + 0.20):
                            points.append((side * x, yy, zz))
                p.hull(f"shoulder_armour_{side}_{index}", points, "machinery")
            for index, zz in enumerate((-33, 30)):
                _, hw, _, _, _ = at(rows, zz)
                box(p, f"rcs_cluster_{side}_{index}", (side * (hw + 0.17), 2.4, zz), (0.24, 0.9, 1.6), "machinery")
                for k in (-1, 1):
                    box(p, f"rcs_nozzle_{side}_{index}_{k}", (side * (hw + 0.33), 2.4, zz + k * 0.4),
                        (0.06, 0.42, 0.35), "hazard")
            hw = at(rows, nav_light_z)[1]
            box(p, f"nav_light_{side}", (side * (hw + 0.14), 2.3, nav_light_z), (0.12, 0.20, 0.8),
                "light_panel" if side > 0 else "screen")
        spine = box(p, "dorsal_service_spine", (0, at(rows, -10)[2] + 0.32, -10), (3.4, 0.7, 13), "machinery")
        p.chamfer(spine, "service_spine_chamfer", 0.18, lambda *args: True)
        sensor = next(s for s in layout["systems"] if s["id"] == "sensor_array")
        x, y, z = sensor["center_m"]
        box(p, "sensor_housing", (x, y, rows[-1][0] + 0.10), (1.5, 1.1, 0.2), "machinery")
        box(p, "sensor_lens", (x, y, rows[-1][0] + 0.24), (0.94, 0.58, 0.06), "screen")
        radiator = next(s for s in layout["systems"] if s["id"] == "radiators")
        z = radiator["center_m"][2]
        y = at(rows, z)[2]
        for side in (-1, 1):
            box(p, f"radiator_{side}", (side * 5.7, y + 0.32, z), (6.4, 0.5, 10.0), "machinery")
            for i in range(12):
                box(p, f"radiator_{side}.fin_{i}", (side * 5.7, y + 0.73, z - 4.7 + i * 0.84),
                    (6.0, 0.28, 0.16), "trim")
        for portal in layout["portals"]:
            if portal["kind"] == "bay_door":
                x, _, z = portal["center_m"]
                w, length = portal["size_m"]
                # A proud closed door clears the sloping keel at every corner.
                y = min(at(rows, zz)[3] for zz in (z - length / 2, z + length / 2)) - 0.25
                box(p, portal["id"] + ".frame", (x, y, z), (w + 0.4, 0.22, length + 0.4), "machinery")
                for side in (-1, 1):
                    box(p, portal["id"] + f".leaf_{side}", (x + side * w / 4, y - 0.17, z),
                        (w / 2 - 0.09, 0.12, length - 0.15), "trim")
                for end in (-1, 1):
                    box(p, portal["id"] + f".marker_{end}", (x, y - 0.26, z + end * (length / 2 - 0.5)),
                        (w - 0.3, 0.05, 0.16), "light_panel")


def service_breaks(ship, rows):
    """One continuous side recess, split only at the source hull's planar stations."""
    band = ship.get("service_band")
    if not band:
        return []
    za, zb = band["z_m"]
    ya, yb = band["y_m"]
    stations = [za] + [r[0] for r in rows if za < r[0] < zb] + [zb]
    return [{"id": f"service_band_{side}_{i}", "normal": [side, 0, 0],
             "depth_m": band["depth_m"], "role": "bulkhead",
             "polygon_m": [[ya, a], [yb, a], [yb, b], [ya, b]]}
            for side in (-1, 1) for i, (a, b) in enumerate(zip(stations, stations[1:]))]


def service_details(p, ship, rows):
    """Hull-following pipes and small modules inside the authored service band."""
    band = ship.get("service_band")
    if not band:
        return
    za, zb = band["z_m"]
    stations = [za + .3] + [r[0] for r in rows if za + .3 < r[0] < zb - .3] + [zb - .3]
    for side in (-1, 1):
        for index, spec in enumerate(band["pipes"]):
            path = [(side * (at(rows, z)[1] + .055), spec["y_m"], z) for z in stations]
            pipe(p, f"service_pipe_{side}_{index}", path, spec["radius_m"], "trim", sides=6)
        for index, z in enumerate(band["clamp_stations_m"]):
            width = at(rows, z)[1]
            # Brackets sit behind the pipes; no parallel exterior faces share a plane.
            box(p, f"service_clamp_{side}_{index}", (side * (width+.01), -.23, z),
                (.08, .92, .19), "machinery")
        for index, z in enumerate(band["module_stations_m"]):
            width = at(rows, z)[1]
            box(p, f"service_module_{side}_{index}", (side * (width+.035), .92, z),
                (.15, .66, 1.15), "machinery")
            box(p, f"service_latch_{side}_{index}", (side * (width+.13), .92, z-.30),
                (.06, .29, .18), "trim")


def build(ship, level):
    """Make one named exterior assembly at the layout's own origin."""
    p = Prop(ship["id"] + ("" if level == 0 else f"_lod{level}"), "ship-exteriors", "ship-local origin")
    rows, layout = sections(ship)
    body_role = "machinery" if ship["id"] == "shrike" else "bulkhead"
    body = fitted_hull(p, rows, layout, ship, "hull", body_role, caps=("machinery", body_role))
    if layout:
        body["hero_uv"] = "hull"
    windows = []
    if layout and level == 0:
        skin = ship["skin_m"]
        cavity = fitted_hull(p, rows, layout, ship, "hull_cavity", "hazard", skin)
        p.cut(body, "hollow_skin", [cavity])
        # Shallow armor breaks expose the subhull while keeping the pressure skin closed.
        for recess in ship.get("armor_breaks", []) + service_breaks(ship, rows):
            direction = Vector(recess["normal"])
            axis = max(range(3), key=lambda i: abs(direction[i]))
            axes = [i for i in range(3) if i != axis]
            points = []
            for uv in recess["polygon_m"]:
                point = Vector((0, 0, 0))
                for i, value in zip(axes, uv):
                    point[i] = value
                origin = PROP_TO_BLENDER @ (point + direction * 100)
                ray = PROP_TO_BLENDER.to_3x3() @ direction
                hit, surface, _, _ = body.ray_cast(origin, -ray)
                assert hit, f"{recess['id']}: armor break misses hull"
                for distance in (-recess["depth_m"], 1.0):
                    points.append(list(P(surface) + direction * distance))
            assert 0 < recess["depth_m"] < skin, "Armor break perforates pressure skin"
            centre = sum((Vector(v) for v in points), Vector()) / len(points)
            ray = PROP_TO_BLENDER.to_3x3() @ direction
            found, before, _, _ = body.ray_cast(PROP_TO_BLENDER @ (centre + direction * 10), -ray)
            assert found
            cutter = p.hull(recess["id"], points, recess.get("role", "machinery"))
            p.cut(body, recess["id"], [cutter])
            found, after, _, _ = body.ray_cast(before + ray, -ray)
            assert found and abs((before-after).dot(ray)-recess["depth_m"]) < .002, "Incorrect armor depth"
            found, inner, _, _ = body.ray_cast(after - ray, ray)
            assert found and (after-inner).dot(ray) >= skin-recess["depth_m"]-.002, "Armor break thins pressure skin"
    if layout and level < 2:
        # Trace existing bridge windows and the docking airlock to the real hull.
        cutters = []
        for portal in layout["portals"]:
            window = portal["kind"] == "window"
            if level and portal.get("bake_only_lods"):
                continue
            if not window and portal["id"] != "p_airlock_outer":
                continue
            origin = PROP_TO_BLENDER @ Vector(portal["center_m"])
            direction = PROP_TO_BLENDER.to_3x3() @ Vector(portal["normal"])
            # A ray from outside finds the outer skin even when the hull is hollow.
            hit, point, normal, _ = body.ray_cast(origin + direction * 30, -direction)
            if not hit:
                raise ValueError(f"{portal['id']}: sight beam misses hull")
            frame = facet(P(point), P(normal), (0, 1, 0))
            width, height = portal["size_m"]
            if window:
                depth = (point - origin).length
                max_depth = portal.get("max_depth_m", ship["command_hull"]["max_window_depth_m"])
                assert depth <= max_depth, f"{portal['id']}: excessive window depth {depth:.3f} m"
                # Match the interior portal's plane, not the hull facet's different angle.
                frame = facet(Vector(portal["center_m"]), Vector(portal["normal"]), (0, 1, 0))
                cutters.append(p.box(portal["id"] + ".through_cut", (-width / 2, -height / 2, -0.10 if level == 0 else depth - 2),
                                     (width / 2, height / 2, depth + 3.0), "hazard", frame))
                corners, inner_corners = [], []
                tangent = Vector((portal["normal"][2], 0, -portal["normal"][0]))
                rim = ship["window_rim_m"]
                for u, v in [(-1, -1), (1, -1), (1, 1), (-1, 1)]:
                    inner = Vector(portal["center_m"]) + tangent * (u * width / 2) + Vector((0, v * height / 2, 0))
                    ray = PROP_TO_BLENDER @ inner
                    found, outer, _, _ = body.ray_cast(ray + direction * 30, -direction)
                    assert found, f"{portal['id']}: window corner misses hull"
                    assert (outer-ray).length <= max_depth, f"{portal['id']}: deep window corner"
                    corners.append(list(P(outer)))
                    if level == 0:
                        found, inner_skin, _, _ = body.ray_cast(ray, direction)
                        assert found, f"{portal['id']}: window corner misses inner skin"
                        assert (inner_skin-ray).length <= max_depth - skin + 0.002, f"{portal['id']}: deep liner"
                        inner_corners.append(list(P(inner_skin)))
                window_rim(p, rows, layout, portal, frame, depth, ship)
                if level == 0:
                    windows.append({"id": portal["id"], "room": portal["between"][0],
                                "center_m": portal["center_m"], "normal": portal["normal"],
                                "size_m": portal["size_m"], "outer_center_m": list(P(point)),
                                "outer_corners_m": corners, "inner_corners_m": inner_corners,
                                "rim_width_m": rim, "max_depth_m": max_depth, "depth_m": depth,
                                "rim_material": "tern_rim" if ship.get("fittings_atlas") else ship.get("window_rim_role", "accent"),
                                "deck": next(c["decks"][0] for c in layout["compartments"] if c["id"] == portal["between"][0])})
            else:
                cutters.append(p.box(portal["id"] + ".glazing_cut", (-width / 2, -height / 2, -0.14),
                                     (width / 2, height / 2, 0.3), {"-z": "screen" if window else "trim", "*": "machinery"}, frame))
        p.cut(body, "window_and_airlock_recesses", cutters)
        for window in windows:
            direction = PROP_TO_BLENDER.to_3x3() @ Vector(window["normal"])
            origin = PROP_TO_BLENDER @ Vector(window["center_m"])
            hit, _, _, _ = body.ray_cast(origin + direction * 30, -direction, distance=30)
            assert not hit, f"{window['id']}: hull blocks the actual interior portal"
    p.windows = windows
    if level == 0:
        service_details(p, ship, rows)
    # Colour bands follow each section, never a floating rectangular approximation.
    if level < 2 and ship.get("decorative_bands", True):
        for i, (a, b) in enumerate(zip(rows, rows[1:])):
            if i in (0, len(rows) - 2):
                continue
            for side in (-1, 1):
                for name, y0, y1, role, off in [("belt", 0.35, 1.25, "machinery", 0.06),
                                                ("stripe", 1.38, 1.78, "accent", 0.10)]:
                    points = [(side * (s[1] + dx), y, s[0]) for s in (a, b)
                              for dx in (off, off + 0.06) for y in (y0, y1)]
                    p.hull(f"{name}_{side}_{i}", points, role)
    pl, nc = ship["pylon"], ship["nacelle"]
    for side in (-1, 1):
        points = []
        for prefix in ("root", "tip"):
            for dy in (-pl["thickness_m"] / 2, pl["thickness_m"] / 2):
                for dz in (-pl[prefix + "_chord_m"] / 2, pl[prefix + "_chord_m"] / 2):
                    points.append((side * pl[prefix + "_x_m"], pl[prefix + "_y_m"] + dy, pl[prefix + "_z_m"] + dz))
        p.hull(f"warp_pylon_{side}", points, "trim")
        if level == 0:
            # A single swept copper inset follows the level pylon rather than tiling its surface.
            inset = []
            for prefix in ("root", "tip"):
                for dz in (-1.0, 1.0):
                    for dy in (0.04, 0.10):
                        inset.append((side * pl[prefix + "_x_m"],
                                      pl[prefix + "_y_m"] + pl["thickness_m"] / 2 + dy,
                                      pl[prefix + "_z_m"] + dz))
            p.hull(f"pylon_identity_{side}", inset, "accent")
        x, y, z = side * nc["x_m"], nc["y_m"], nc["z_m"]
        length, width, height = nc["length_m"], nc["half_width_m"], nc["half_height_m"]
        spec = [(-0.5, 0.56), (-0.40, 1.0), (0.29, 1.0), (0.44, 0.85), (0.5, 0.52)]
        rings = [(z + length * t, [(x + xx, y + yy) for xx, yy in profile(width * scale, height * scale, -height * scale, 0.5 * scale)])
                 for t, scale in spec]
        nacelle = loft(p, f"warp_nacelle_{side}", rings, ["bulkhead"] * 8, ("machinery", "accent"))
        for outer in (-1, 1):
            box(p, f"field_{side}_{outer}", (x + outer * (width + 0.14), y, z - 0.05 * length),
                (0.12, 0.62, length * 0.59), "screen")
            if level < 2:
                box(p, f"field_frame_{side}_{outer}", (x + outer * (width + 0.02), y, z - 0.05 * length),
                    (0.12, 1.00, length * 0.65), "machinery")
        if level < 2:
            box(p, f"nacelle_spine_{side}", (x, y + height + 0.1, z - length * 0.07),
                (1.1, 0.18, length * 0.62), "machinery")
        if level == 0:
            for k in range(5):
                box(p, f"nacelle_{side}.service_band_{k}", (x, y + height + 0.23, z - 0.27 * length + k * length * 0.09),
                    (width * 1.3, 0.12, 0.3), "trim")
            intake = revolve(p, f"nacelle_{side}.intake", [(0.9, z + length * 0.50), (1.0, z + length * 0.52)],
                             "accent", "z", (x, y), sides=10)
            p.cut(intake, f"nacelle_{side}.recess", [revolve(p, f"intake_cutter_{side}",
                  [(0.66, z + length * 0.51), (0.66, z + length * 0.54)], "machinery", "z", (x, y), sides=10)])
            revolve(p, f"nacelle_{side}.collector", [(0.64, z + length * 0.508), (0.64, z + length * 0.512)],
                    "light_panel", "z", (x, y), sides=10)
    if level < 2:
        # Command brow and low dorsal armour spine follow the existing loft height.
        z0, z1 = (20.0, 31.0) if layout else (rows[-3][0], rows[-2][0])
        top = min(at(rows, z0)[2], at(rows, z1)[2])
        if not layout:
            brow = p.hull("command_brow", [(x, yy, zz) for zz, hw, yy in [(z0, 3.8, top - 0.1),
                (z0, 3.5, top + 0.7), (z1, 2.4, top - 0.1), (z1, 2.0, top + 0.45)] for x in (-hw, hw)], "trim")
            box(p, "bridge_glazing", (0, top + 0.38, z1 + 0.06), (3.6, 0.3, 0.14), "screen")
        if level == 0 and not layout:
            glazing = []
            for side in (-1, 1):
                for k in range(3):
                    zz = z0 + 2.0 + k * (z1 - z0 - 3.0) / 3
                    origin = PROP_TO_BLENDER @ Vector((0, top + 0.3, zz))
                    direction = PROP_TO_BLENDER.to_3x3() @ Vector((side, 0, 0))
                    hit, point, normal, _ = brow.ray_cast(origin, direction)
                    if hit:
                        frame = facet(P(point), P(normal), (0, 1, 0))
                        glazing.append(p.box(f"brow_glazing_{side}_{k}", (-0.6, -0.11, -0.08), (0.6, 0.11, 0.2),
                                             {"-z": "screen", "*": "machinery"}, frame))
            if glazing:
                p.cut(brow, "command_brow_glazing", glazing)
            detail(p, rows, layout, level)
        else:
            detail(p, rows, layout, level, ship.get("command_hull", {}).get("from_z_m"), ship.get("nav_light_z_m", 17))
        if not layout:
            for side in (-1, 1):
                revolve(p, f"impulse_{side}", [(1.0, rows[0][0] - 1.0), (0.7, rows[0][0] + 0.3)],
                        "trim", "z", (side * rows[0][1] * 0.55, 0), sides=8)
                revolve(p, f"impulse_glow_{side}", [(0.6, rows[0][0] - 1.04), (0.6, rows[0][0] - 1.02)],
                        "screen", "z", (side * rows[0][1] * 0.55, 0), sides=8)
            if ship["id"] == "shrike":
                for side in (-1, 1):
                    p.hull(f"weapon_shoulder_{side}", [(side * x, yy, zz) for x, zz in [(7, -12), (14, -10), (12, 8), (6, 15)]
                           for yy in (-0.8, 0.8)], "machinery")
                    box(p, f"railgun_{side}", (side * 10.8, 1.15, 2), (0.6, 0.5, 20), "accent")
        if level == 0:
            for side in (-1, 1):
                # Long restrained identity marks, shaped to the hull's changing width.
                bands = [(2.0, 18.0), (32.5, 36.5)] if layout else [(2.0, 18.0)]
                if layout and ship.get("command_hull"):
                    bands = [(2.0, ship["command_hull"]["from_z_m"] - 0.03)]
                if not ship.get("decorative_bands", True):
                    bands = []
                for index, (za, zb) in enumerate(bands):
                    points = []
                    for zz in (za, zb):
                        _, hw, top, _, chamfer = at(rows, zz)
                        for xx in (hw - chamfer - 4.1, hw - chamfer - 3.4):
                            for yy in (top + 0.05, top + 0.11):
                                points.append((side * xx, yy, zz))
                    p.hull(f"dorsal_livery_{side}_{index}", points, "accent")
            # The player's lettering is painted in its UV atlas. Companions keep their plate.
            width = 9.0 if layout else 5.0
            zz = 10.0 if layout else 5.0
            y = at(rows, zz)[2] + 0.08
            if not ship.get("painted_registration"):
                decal = box(p, "registration", (0, y, zz), (width, 0.06, 2.3), "trim")
                decal["registration"] = True
    return p, rows, layout


def assembly_uv(point, normal, bounds):
    """Project an assembly face with its longest in-plane dimension along V."""
    lo, hi = bounds
    span = hi - lo
    maximum = max(abs(v) for v in normal)
    axis = next(i for i in range(3) if abs(normal[i]) >= maximum - .0001)
    short, long = sorted((i for i in range(3) if i != axis), key=lambda i: span[i])
    return ((point[short]-lo[short])/max(span[short], .001),
            (point[long]-lo[long])/max(span[long], .001), short, long)


def surface_uv(ob, point, normal, role, rows, config, bounds):
    """Fit designed atlas strips to assemblies, keeping seams off the broad hull faces."""
    settings = config["surface_atlas"]
    lo, hi = bounds
    span = hi - lo
    # UV front is image top. Both ship sides share a consistent fore-aft orientation.
    if ob.name.endswith(".hull") and role != "machinery":
        z = min(rows[-1][0], max(rows[0][0], point.z))
        _, width, top, bottom, chamfer = at(rows, z)
        v = (z - rows[0][0]) / (rows[-1][0] - rows[0][0])
        if abs(normal.z) > 0.85:
            region = "ventral"
            u = 0.5 + point.x / (2 * width)
            v = 0.4 + 0.15 * (point.y - bottom) / (top - bottom)
        elif abs(normal.y) > 0.65:
            region = "dorsal" if normal.y > 0 else "ventral"
            # Include the chamfer in the wrap: clamping at the flat deck width collapses it.
            u = 0.5 + point.x / (2 * width)
        else:
            region = "side"
            u = (point.y - bottom) / (top - bottom)
    else:
        u, v, short, long = assembly_uv(point, normal, bounds)
        region = "mechanical" if role == "machinery" else ("ventral" if normal.y < -0.65 else "dorsal")
        if "warp_nacelle" in ob.name:
            # Use a narrow silver portion at hull-like texel density, with no habitation windows.
            u = 0.35 + u * 0.30
            v = 0.10 + v * 0.70
        elif "warp_pylon" in ob.name:
            u = 0.12 + u * 0.76
            v = 0.35 + v * 0.20
        elif role == "machinery":
            # Preserve mechanical detail size on small fittings; broad service beds span more art.
            u = 0.10 + u * min(0.8, max(0.12, span[short] / 12))
            v = 0.20 + v * min(0.75, max(0.08, span[long] / 55))
        else:
            u = 0.32 + u * 0.36
            v = 0.30 + v * min(0.65, max(0.06, span[long] / 60))
    rect = settings["regions"][region]
    gutter = settings["gutter_px"] / settings["px"]
    u, v = max(0, min(1, u)), max(0, min(1, v))
    return (rect[0] + gutter + u * (rect[2] - rect[0] - 2 * gutter),
            rect[1] + gutter + v * (rect[3] - rect[1] - 2 * gutter))


def hero_uv(ob, point, normal, rows, bounds, config):
    """Artist-readable UV islands: hull deck, keel and wall."""
    za, zb = config["hero_atlas"]["longitudinal_range_m"]
    # Keep paint anchored when exterior sections are shortened or extended.
    longitudinal = (point.z - za) / (zb - za)
    # A tolerance keeps coplanar 45-degree triangles on the same island after CSG rounding.
    if abs(normal.z) > 0.85 or (abs(normal.y) <= 0.65 and abs(normal.z) > abs(normal.x) + 0.001):
        # Use the solid ivory centre of the bow island, clear of its open arch-shaped margins.
        rect = (0.84, 0.62, 0.918, 0.827)
        u, v = (point.x + 12.2) / 24.4, (point.y + 5.8) / 14
    elif abs(normal.y) > 0.65:
        rect = (0.01, 0.08, 0.30, 0.99) if normal.y > 0 else (0.32, 0.08, 0.61, 0.99)
        # Looking down from +Y with the bow up, +X is image-left. Keep painted type readable.
        u, v = ((12.2 - point.x) if normal.y > 0 else (point.x + 12.2)) / 24.4, longitudinal
    else:
        service_uv = config["hero_atlas"].get("side_service_uv")
        if service_uv:
            # Anchor the generated strip to both actual recess edges, leaving the windows white.
            return (float(np.interp(point.y, service_uv["height_m"], service_uv["u"])),
                    float(np.interp(point.z, service_uv["station_m"], service_uv["v"])))
        rect = (0.63, 0.08, 0.745, 0.99)
        u, v = (point.y + 5.8) / 14, longitudinal
    return (rect[0] + u * (rect[2] - rect[0]), rect[1] + v * (rect[3] - rect[1]))


def export_hero_guide(objects, config):
    """Export the actual painted-material UV polygons for wireframe and mask rasterization."""
    faces = []
    for ob in objects:
        for poly in ob.data.polygons:
            if ob.data.materials[poly.material_index].name not in ("tern_hull", "tern_rim"):
                continue
            faces.append({"assembly": ob.name, "region": ob.get("hero_uv", "fittings"),
                          "uv": [list(ob.data.uv_layers.active.data[li].uv) for li in poly.loop_indices]})
    target = ROOT / "tools/materials/sources/tern-uv-layout.json"
    target.write_text(json.dumps({"size_px": config["hero_atlas"]["guide_px"], "faces": faces}, indent=1) + "\n", encoding="utf-8")
    print(f"UV GUIDE: {len(faces)} real mesh faces", flush=True)


def prepare(p, ship, config):
    """Clean and validate each closed assembly, project UVs, and bake the fixed sun per face."""
    objects = [o for o in p.coll.objects if o.type == "MESH"]
    sun = Vector(config["sun_dir_ship"]).normalized()
    rows, _ = sections(ship)
    for ob in objects:
        clean(ob)
        bm = bmesh.new()
        bm.from_mesh(ob.data)
        if any(len(e.link_faces) != 2 for e in bm.edges) or any(not v.is_manifold for v in bm.verts):
            raise ValueError(f"{ob.name}: non-manifold exterior assembly")
        if any(f.calc_area() < 1e-7 for f in bm.faces):
            raise ValueError(f"{ob.name}: degenerate exterior triangle")
        bm.free()
        finish(ob)
        points = [P(vertex.co) for vertex in ob.data.vertices]
        bounds = (Vector([min(v[i] for v in points) for i in range(3)]),
                  Vector([max(v[i] for v in points) for i in range(3)]))
        colour = ob.data.color_attributes.new(name="SunBake", type="FLOAT_COLOR", domain="CORNER")
        ob.data.color_attributes.active_color = colour
        for poly in ob.data.polygons:
            role = ROLES[poly.material_index]
            row = config["materials"].get(role, config["materials"]["machinery"])
            fittings = ship.get("fittings_atlas", {}).get(role) if not (ob.get("hero_uv") and role == "bulkhead") else None
            tint = ship["livery_srgb"] if role == "accent" else row["tint_srgb"]
            brightness = 1.0 if row.get("emissive") else config["ambient_fraction"] + (1 - config["ambient_fraction"]) * max(0, P(poly.normal).dot(sun))
            for li in poly.loop_indices:
                colour.data[li].color = (*[v * brightness for v in lin(tint)], 1)
                uv = ob.data.uv_layers.active.data[li].uv
                uv[0] /= row["span_m"]
                uv[1] = 1 - (1 - uv[1]) / row["span_m"]
                if row["texture"] == config["surface_atlas"]["texture"]:
                    uv[:] = surface_uv(ob, points[ob.data.loops[li].vertex_index], P(poly.normal),
                                       role, rows, config, bounds)
                if ob.get("hero_uv") and role == "bulkhead":
                    uv[:] = hero_uv(ob, points[ob.data.loops[li].vertex_index], P(poly.normal), rows, bounds, config)
                elif fittings:
                    u, v, _, _ = assembly_uv(points[ob.data.loops[li].vertex_index], P(poly.normal), bounds)
                    gutter = config["hero_atlas"]["gutter_px"]/config["hero_atlas"]["px"]
                    uv[:] = (fittings[0]+gutter+u*(fittings[2]-fittings[0]-2*gutter),
                             fittings[1]+gutter+v*(fittings[3]-fittings[1]-2*gutter))
            if (ob.get("hero_uv") and role == "bulkhead") or fittings:
                painted = bpy.data.materials["tern_rim" if ob.name.endswith(".rim") else "tern_hull"]
                if painted not in list(ob.data.materials):
                    ob.data.materials.append(painted)
                poly.material_index = list(ob.data.materials).index(painted)
            if ob.get("registration") and P(poly.normal).y > 0.99:
                if bpy.data.materials["registration"] not in list(ob.data.materials):
                    ob.data.materials.append(bpy.data.materials["registration"])
                poly.material_index = len(ob.data.materials) - 1
                xmin, xmax = bounds[0].x, bounds[1].x
                zmin, zmax = bounds[0].z, bounds[1].z
                for li in poly.loop_indices:
                    v = P(ob.data.vertices[ob.data.loops[li].vertex_index].co)
                    u0, u1 = ship["decal_u"]
                    ob.data.uv_layers.active.data[li].uv = (u1 - (u1 - u0) * (v.x - xmin) / (xmax - xmin), (v.z - zmin) / (zmax - zmin))
                    colour.data[li].color = (brightness, brightness, brightness, 1)
        # Coplanar triangles must not choose different painted islands at a shared vertex.
        painted_uvs = {}
        for poly in ob.data.polygons:
            if ob.data.materials[poly.material_index].name not in ("tern_hull", "tern_rim"):
                continue
            normal_key = tuple(round(v, 3) for v in poly.normal)
            for li in poly.loop_indices:
                key = (ob.data.loops[li].vertex_index, normal_key)
                uv = ob.data.uv_layers.active.data[li].uv.copy()
                if key in painted_uvs:
                    assert (uv - painted_uvs[key]).length < 0.00002, f"{ob.name}: painted seam on a coplanar face"
                painted_uvs[key] = uv
    return objects


def merge_copies(objects, name):
    """A disposable merged bake/export mesh; editable source assemblies remain untouched."""
    for ob in bpy.context.selected_objects:
        ob.select_set(False)
    copies = []
    for ob in objects:
        copy = ob.copy()
        copy.data = ob.data.copy()
        bpy.context.scene.collection.objects.link(copy)
        copy.select_set(True)
        copies.append(copy)
    bpy.context.view_layer.objects.active = copies[0]
    bpy.ops.object.join()
    merged = bpy.context.view_layer.objects.active
    merged.name = name
    return merged


def lod_uv_cache_path(mesh, settings, side):
    """Reuse chart placement only when geometry, packer source and packing settings agree."""
    vertices = np.empty(len(mesh.vertices) * 3, dtype="<f4")
    loops = np.empty(len(mesh.loops), dtype="<i4")
    counts = np.empty(len(mesh.polygons), dtype="<i4")
    mesh.vertices.foreach_get("co", vertices)
    mesh.loops.foreach_get("vertex_index", loops)
    mesh.polygons.foreach_get("loop_total", counts)
    digest = hashlib.sha256(b"exterior-uv-cache-v1")
    for data in (vertices.tobytes(), loops.tobytes(), counts.tobytes(),
                 (ROOT / "tools/blender/hs_kit.py").read_bytes(),
                 json.dumps([bpy.app.version_string, side, settings["margin_px"], 32]).encode()):
        digest.update(data)
    return ROOT / "build/exterior-tools/uv-cache" / (digest.hexdigest() + ".json")


def save_lod_uv_cache(mesh, layer, facts, path):
    """Store the packer's float32 UVs without altering the actual exported atlas."""
    values = np.empty(len(mesh.loops) * 2, dtype=np.float32)
    layer.data.foreach_get("uv", values)
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps({"uv": values.tolist(), "facts": facts}), encoding="utf-8")


def bake_lod(p, objects, detailed, windows, ship, config, path):
    """Transfer actual high-detail paint, fittings and emission onto this LOD's own UV atlas."""
    settings = config["lod_bake"]
    side = settings["atlas_px"][int(p.name[-1])]
    scale = settings["supersample"]
    bake_side = side * scale
    print(f"ATLAS {p.name}: packing {side} px charts", flush=True)
    low = merge_copies(objects, p.name + ".bake_target")
    # Chart geometry has one role. Emission is transferred from the source in its own pass.
    for poly in low.data.polygons:
        poly.material_index = 0
    target_prop = SimpleNamespace(body=low, wall=False, atlas_all_faces=True)
    cache = lod_uv_cache_path(low.data, settings, side)
    if cache.exists():
        print(f"ATLAS {p.name}: reusing unchanged geometry's chart placement", flush=True)
    else:
        facts = atlas_uv_at(target_prop, {"atlas": {"margin_px": settings["margin_px"], "max_px_per_m": 32}}, side)
        save_lod_uv_cache(low.data, low.data.uv_layers[-1], facts, cache)
        low.data.uv_layers.remove(low.data.uv_layers[-1])
    # Fresh and cached charts must take the same layer activation and data update path.
    packed = json.loads(cache.read_text(encoding="utf-8"))
    values = np.array(packed["uv"], dtype=np.float32)
    assert values.size == len(low.data.loops) * 2 and np.all(np.isfinite(values)), "Invalid cached UVs"
    assert np.all((values >= 0) & (values <= 1)), "Cached UVs outside atlas"
    low.data.uv_layers.new(name="atlas").data.foreach_set("uv", values)
    facts = packed["facts"]
    print(f"ATLAS {p.name}: baking colour and emission", flush=True)
    low.data.uv_layers.remove(low.data.uv_layers[0])
    low.data.uv_layers.active_index = 0
    low.data.uv_layers[0].active_render = True
    low.data.update()
    # Opaque LODs need a dark source at each opening. Without it, missed bake rays
    # are margin-filled from the copper rim, producing solid orange rectangles.
    caps = Prop(p.name + ".window_bake", "ship-exteriors", "temporary bake glazing")
    for w in windows:
        width, height = w["size_m"]
        frame = facet(Vector(w["center_m"]), Vector(w["normal"]), (0, 1, 0))
        caps.box(w["id"], (-width/2, -height/2, w["depth_m"]-.06),
                 (width/2, height/2, w["depth_m"]-.04), "hazard", frame)
    cap_objects = prepare(caps, ship, config)
    high = merge_copies(detailed + cap_objects, p.name + ".bake_source")
    for ob in cap_objects:
        bpy.data.objects.remove(ob, do_unlink=True)
    bpy.data.collections.remove(caps.cutters)
    bpy.data.collections.remove(caps.coll)
    # Sources belonging to another LOD must not compete for rays in this bake.
    scene = bpy.context.scene
    scene.render.engine = "CYCLES"
    scene.cycles.device = "CPU"
    scene.cycles.samples = settings["samples"]
    scene.cycles.seed = 0
    scene.cycles.use_denoising = False
    scene.render.bake.use_selected_to_active = True
    scene.render.bake.cage_extrusion = settings["cage_m"]
    scene.render.bake.max_ray_distance = settings["ray_distance_m"]
    scene.render.bake.margin = settings["margin_px"] * scale
    scene.view_settings.view_transform = "Standard"
    scene.view_settings.look = "None"
    scene.view_settings.exposure = 0
    scene.view_settings.gamma = 1
    colour = bpy.data.images.new(p.name + ".supersampled_colour", width=bake_side, height=bake_side, alpha=True)
    colour.alpha_mode = "CHANNEL_PACKED"
    mat = bpy.data.materials.new(p.name + "_atlas")
    mat.use_nodes = True
    mat.use_backface_culling = True
    mat.node_tree.nodes.clear()
    image = mat.node_tree.nodes.new("ShaderNodeTexImage")
    image.image = colour
    image.interpolation = "Closest"
    mat.node_tree.nodes.active = image
    output = mat.node_tree.nodes.new("ShaderNodeOutputMaterial")
    mat.node_tree.links.new(image.outputs["Color"], output.inputs["Surface"])
    low.data.materials.clear()
    low.data.materials.append(mat)
    for ob in bpy.context.selected_objects:
        ob.select_set(False)
    high.select_set(True)
    low.select_set(True)
    bpy.context.view_layer.objects.active = low
    bpy.ops.object.bake(type="EMIT")
    pixels = np.empty(bake_side * bake_side * 4, dtype=np.float32)
    colour.pixels.foreach_get(pixels)
    # The second real bake carries only the source's emission classification.
    mask = bpy.data.images.new(p.name + ".emission_mask", width=bake_side, height=bake_side, alpha=False)
    mask.colorspace_settings.name = "Non-Color"
    image.image = mask
    mask_materials = []
    for source in list(high.data.materials):
        role = source.name
        mask_mat = bpy.data.materials.new(role + ".bake_mask")
        mask_mat.use_nodes = True
        nodes = mask_mat.node_tree.nodes
        nodes.clear()
        emission = nodes.new("ShaderNodeEmission")
        value = float(config["materials"].get(role, {}).get("emissive", False))
        emission.inputs["Color"].default_value = (value, value, value, 1)
        if role in ("tern_hull", "tern_rim") or config["materials"].get(role, {}).get("texture") == config["surface_atlas"]["texture"]:
            texture = nodes.new("ShaderNodeTexImage")
            texture.image = source.node_tree.nodes.get("Image Texture").image
            texture.interpolation = "Closest"
            mask_mat.node_tree.links.new(texture.outputs["Alpha"], emission.inputs["Color"])
        out = nodes.new("ShaderNodeOutputMaterial")
        mask_mat.node_tree.links.new(emission.outputs[0], out.inputs["Surface"])
        mask_materials.append(mask_mat)
    for index, mask_mat in enumerate(mask_materials):
        high.data.materials[index] = mask_mat
    bpy.ops.object.bake(type="EMIT")
    mask_pixels = np.empty_like(pixels)
    mask.pixels.foreach_get(mask_pixels)
    pixels.reshape(-1, 4)[:, 3] = np.clip(mask_pixels.reshape(-1, 4)[:, 0], 0, 1)
    # Average the actual render in linear light: fine grilles and chart edges survive minification.
    pixels = pixels.reshape(side, scale, side, scale, 4).mean(axis=(1, 3)).reshape(-1)
    supersampled = colour
    colour = bpy.data.images.new(p.name + "_atlas", width=side, height=side, alpha=True)
    colour.alpha_mode = "CHANNEL_PACKED"
    assert np.std(pixels.reshape(-1, 4)[:, :3]) > 0.025, "Empty LOD colour bake"
    coverage = np.count_nonzero(pixels.reshape(-1, 4)[:, 3] > 0.1) / (side * side)
    assert coverage > 0.0001, "LOD bake lost the field strip emission mask"
    colour.pixels.foreach_set(pixels)
    image.image = colour
    bpy.data.images.remove(supersampled)
    colour.filepath_raw = str(path)
    colour.file_format = "PNG"
    colour.save()
    # Sample real packed UVs at each window centre, so copper flood-fill cannot regress.
    baked_pixels = pixels.reshape(side, side, 4)
    for w in windows:
        direction = PROP_TO_BLENDER.to_3x3() @ Vector(w["normal"])
        origin = PROP_TO_BLENDER @ Vector(w["outer_center_m"])
        hit, point, _, face = low.ray_cast(origin + direction * 4, -direction)
        assert hit, f"{p.name}: missing baked window {w['id']}"
        poly = low.data.polygons[face]
        assert len(poly.vertices) == 3
        vertices = [low.data.vertices[i].co for i in poly.vertices]
        uvs = [Vector((*low.data.uv_layers[0].data[i].uv, 0)) for i in poly.loop_indices]
        uv = barycentric_transform(point, *vertices, *uvs)
        rgb = baked_pixels[min(side-1,int(uv.y*side)), min(side-1,int(uv.x*side)), :3]
        luminance = float(rgb @ np.array((.2126, .7152, .0722)))
        assert luminance < .15 and rgb[0]-rgb[1] < .02, f"{p.name}: filled window {w['id']}: {rgb}"
    facts["window_centres_checked"] = len(windows)
    for attr in low.data.color_attributes:
        for item in attr.data:
            item.color = (1, 1, 1, 1)
    for original in objects:
        original.hide_render = True
        original.hide_set(True)
    for collection in list(low.users_collection):
        collection.objects.unlink(low)
    p.coll.objects.link(low)
    bpy.data.objects.remove(high, do_unlink=True)
    bpy.data.images.remove(mask)
    for mask_mat in mask_materials:
        bpy.data.materials.remove(mask_mat)
    facts.update({"file": "atlases/" + path.name, "px": side, "emission_coverage": round(coverage, 5),
                  "sha256": hashlib.sha256(path.read_bytes()).hexdigest(), "bytes": path.stat().st_size})
    return [low], facts


def export(objects, name, path):
    """Merge copies for material batching; keep original source assemblies editable."""
    merged = merge_copies(objects, name)
    bpy.ops.export_scene.gltf(filepath=str(path), export_format="GLB", use_selection=True,
        export_yup=True, export_texcoords=True, export_normals=True, export_tangents=False,
        export_vertex_color="ACTIVE", export_active_vertex_color_when_no_material=True,
        export_materials="EXPORT", export_image_format="AUTO", export_extras=False,
        export_animations=False, export_cameras=False, export_lights=False,
        export_skins=False, export_morph=False)
    bpy.data.objects.remove(merged, do_unlink=True)
    # The exporter couples minification to Blender's Closest setting. Keep the repo's
    # nearest magnification and trilinear mipmaps explicitly in the delivered artifact.
    data = path.read_bytes()
    json_length = struct.unpack_from("<I", data, 12)[0]
    document = json.loads(data[20:20 + json_length])
    for sampler in document.get("samplers", []):
        sampler.update({"magFilter": 9728, "minFilter": 9987, "wrapS": 10497, "wrapT": 10497})
    payload = json.dumps(document, separators=(",", ":")).encode("utf-8")
    payload += b" " * ((-len(payload)) % 4)
    binary_chunk = data[20 + json_length:]
    path.write_bytes(struct.pack("<III", 0x46546C67, 2, 20 + len(payload) + len(binary_chunk))
                     + struct.pack("<II", len(payload), 0x4E4F534A) + payload + binary_chunk)


def verify(path, budget):
    """Read the real GLB's accessors, material batches, baked colours and embedded PNGs."""
    data = path.read_bytes()
    magic, version, length = struct.unpack_from("<III", data)
    assert magic == 0x46546C67 and version == 2 and length == len(data)
    jlen = struct.unpack_from("<I", data, 12)[0]
    js = json.loads(data[20:20 + jlen])
    binary = data[28 + jlen:]
    triangles, vertices, draws, mins, maxs, gpu_bytes = 0, 0, 0, [], [], 0
    for mesh in js["meshes"]:
        for primitive in mesh["primitives"]:
            attrs = primitive["attributes"]
            assert all(k in attrs for k in ("POSITION", "NORMAL", "TEXCOORD_0", "COLOR_0")), "Missing UV, normal or sun bake"
            position = js["accessors"][attrs["POSITION"]]
            indices = js["accessors"][primitive["indices"]]
            assert indices["count"] % 3 == 0
            bv = js["bufferViews"][indices["bufferView"]]
            fmt = {5121: "B", 5123: "H", 5125: "I"}[indices["componentType"]]
            idx = struct.unpack_from("<" + str(indices["count"]) + fmt, binary, bv.get("byteOffset", 0) + indices.get("byteOffset", 0))
            assert max(idx) < position["count"], "Invalid model index"
            uv_accessor = js["accessors"][attrs["TEXCOORD_0"]]
            uv_view = js["bufferViews"][uv_accessor["bufferView"]]
            uv_values = np.frombuffer(binary, dtype="<f4", count=uv_accessor["count"] * 2,
                                      offset=uv_view.get("byteOffset", 0) + uv_accessor.get("byteOffset", 0)).reshape(-1, 2)
            uv_triangles = uv_values[np.array(idx).reshape(-1, 3)]
            edge_a, edge_b = uv_triangles[:, 1] - uv_triangles[:, 0], uv_triangles[:, 2] - uv_triangles[:, 0]
            uv_area = np.abs(edge_a[:, 0] * edge_b[:, 1] - edge_a[:, 1] * edge_b[:, 0])
            if np.any(uv_area <= 1e-12):
                positions_view = js["bufferViews"][position["bufferView"]]
                positions = np.frombuffer(binary, dtype="<f4", count=position["count"] * 3,
                                          offset=positions_view.get("byteOffset", 0) + position.get("byteOffset", 0)).reshape(-1, 3)
                bad = np.array(idx).reshape(-1, 3)[uv_area <= 1e-12]
                raise ValueError(f"{path.name}: {len(bad)} collapsed UV triangles in "
                                 f"{js['materials'][primitive['material']]['name']}: {positions[bad[:4]].tolist()}")
            for accessor_id in attrs.values():
                accessor = js["accessors"][accessor_id]
                if accessor["componentType"] == 5126:
                    view = js["bufferViews"][accessor["bufferView"]]
                    count = accessor["count"] * {"VEC2": 2, "VEC3": 3, "VEC4": 4}[accessor["type"]]
                    values = struct.unpack_from("<" + str(count) + "f", binary, view.get("byteOffset", 0) + accessor.get("byteOffset", 0))
                    assert all(math.isfinite(v) for v in values), "Non-finite model attribute"
            triangles += indices["count"] // 3
            vertices += position["count"]
            draws += 1
            mins.append(position["min"])
            maxs.append(position["max"])
    assert triangles <= budget, f"{triangles} triangles over {budget} ceiling"
    assert draws <= 9, f"{draws} material draws over 9 ceiling"
    for material in js["materials"]:
        assert "KHR_materials_unlit" in material.get("extensions", {}), "Runtime lighting would duplicate the bake"
        assert "normalTexture" not in material and not material.get("doubleSided", False)
    assert all(s["magFilter"] == 9728 and s["minFilter"] == 9987 for s in js.get("samplers", [])), "Incorrect texture sampling"
    for image in js.get("images", []):
        view = js["bufferViews"][image["bufferView"]]
        image_data = binary[view["byteOffset"]:view["byteOffset"] + view["byteLength"]]
        assert image_data[:8] == b"\x89PNG\r\n\x1a\n", "Texture must be an embedded PNG"
        w, h = struct.unpack_from(">II", image_data, 16)
        while True:
            gpu_bytes += w * h * 4
            if w == h == 1:
                break
            w, h = max(1, w // 2), max(1, h // 2)
    lo = [round(min(v[i] for v in mins), 4) for i in range(3)]
    hi = [round(max(v[i] for v in maxs), 4) for i in range(3)]
    return {"file": path.name, "triangles": triangles, "triangle_budget": budget, "vertices": vertices,
            "draw_calls": draws, "dimensions_m": [round(hi[i] - lo[i], 4) for i in range(3)],
            "bounds_m": [lo, hi], "embedded_images": len(js.get("images", [])),
            "texture_bytes_with_mips": gpu_bytes, "sha256": hashlib.sha256(data).hexdigest(), "bytes": len(data)}


def main():
    """Build all fleet assets, or compare generated bytes without changing delivered assets."""
    global OUT
    args = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else sys.argv[1:]
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--livery", default="copper", help="Player paint scheme from the exterior design data")
    parser.add_argument("--uv-guide", action="store_true", help="Build only the player source and export its actual UV guide")
    opts = parser.parse_args(args)
    config = json.loads(CONFIG.read_text(encoding="utf-8"))
    livery = next((v for v in config["liveries"] if v["id"] == opts.livery), None)
    if livery is None:
        parser.error(f"Unknown livery: {opts.livery}")
    config["hero_atlas"]["texture"] = livery["texture"]
    player = next(s for s in config["ships"] if s["id"] == "tern")
    player["livery_srgb"] = livery["accent_srgb"]
    if opts.livery != config["liveries"][0]["id"]:
        OUT = OUT / "liveries" / opts.livery
        config["ships"] = [player]
    bpy.ops.wm.read_factory_settings(use_empty=True)
    setup_materials(config)
    if opts.uv_guide:
        ship = next(s for s in config["ships"] if s["id"] == "tern")
        p, _, _ = build(ship, 0)
        objects = prepare(p, ship, config)
        export_hero_guide(objects, config)
        path = ROOT / "build/exterior-tools/tern-shape.glb"
        export(objects, "tern", path)
        facts = verify(path, 12000)
        facts["windows"] = p.windows
        path.with_suffix(".json").write_text(json.dumps(facts, indent=2) + "\n", encoding="utf-8")
        print(facts, flush=True)
        return
    OUT.mkdir(parents=True, exist_ok=True)
    (OUT / "atlases").mkdir(exist_ok=True)
    manifest = {"schema": "starcrew.props/1", "status": "Exterior design assets, no runtime integration",
                "generator": "tools/blender/build_ship_exteriors.py", "blender": bpy.app.version_string,
                "source": "data/ships/exteriors.json", "material_source": "data/materials/materials.json",
                "axes": config["axes"], "props": {}}
    manifest["livery"] = livery
    if opts.livery == config["liveries"][0]["id"]:
        manifest["liveries"] = [{**v, "directory": "." if i == 0 else "liveries/" + v["id"]}
                                for i, v in enumerate(config["liveries"])]
        manifest["default_livery"] = config["default_livery"]
        assert any(v["id"] == manifest["default_livery"] for v in manifest["liveries"])
    with tempfile.TemporaryDirectory(prefix="starcrew-exteriors-") as scratch:
        for ship in config["ships"]:
            detailed = None
            for level in range(3) if ship["id"] == "tern" else [0]:
                p, rows, _ = build(ship, level)
                objects = prepare(p, ship, config)
                assemblies = [ob.name for ob in objects]
                atlas = None
                if ship["id"] == "tern":
                    pl, nc = ship["pylon"], ship["nacelle"]
                    assert pl["root_y_m"] == pl["tip_y_m"] == nc["y_m"], "Warp attachments must share one level centreline"
                    for ob in objects:
                        if "warp_pylon" in ob.name or "warp_nacelle" in ob.name:
                            assert max(P(v.co).y for v in ob.data.vertices) < max(r[2] for r in rows), "Warp attachment rises above the hull"
                if level == 0:
                    detailed = objects
                    detailed_windows = p.windows
                    if ship["id"] == "tern" and not opts.check and opts.livery == config["liveries"][0]["id"]:
                        export_hero_guide(objects, config)
                else:
                    png = Path(scratch) / (p.name + ".png")
                    objects, atlas = bake_lod(p, objects, detailed, detailed_windows, ship, config, png)
                    target_png = OUT / "atlases" / png.name
                    if opts.check:
                        assert target_png.read_bytes() == png.read_bytes(), f"{target_png}: baked bytes differ"
                    else:
                        target_png.write_bytes(png.read_bytes())
                    objects[0].data.materials[0].node_tree.nodes.get("Image Texture").image.filepath_raw = str(target_png)
                path = Path(scratch) / (p.name + ".glb")
                export(objects, p.name, path)
                row = verify(path, ship["budget_triangles"][level])
                row.update({"name": ship["name"], "class": ship["class"], "role": ship["role"],
                            "lod": level, "warp_pylons": 2, "warp_nacelles": 2,
                            "assemblies": assemblies, "csg": p.steps,
                            "warp_center_y_m": ship["nacelle"]["y_m"], "nacelle_length_m": ship["nacelle"]["length_m"]})
                if p.windows:
                    row["windows"] = p.windows
                if atlas:
                    row["atlas"] = atlas
                    assert row["draw_calls"] == row["embedded_images"] == 1, "LOD must contain its own single baked atlas"
                manifest["props"][p.name] = row
                target = OUT / path.name
                if opts.check:
                    assert target.read_bytes() == path.read_bytes(), f"{target}: generated bytes differ"
                else:
                    target.write_bytes(path.read_bytes())
                print(f"EXTERIOR {p.name}: {row['triangles']}/{row['triangle_budget']} triangles, {row['draw_calls']} draws, {row['bytes']} bytes", flush=True)
                if level:
                    p.coll.hide_viewport = True
                    p.coll.hide_render = True
                elif ship["id"] != "tern":
                    for ob in objects:
                        ob.location.x += 76 if ship["id"] == "osprey" else -76
                p.cutters.hide_viewport = True
        content = json.dumps(manifest, indent=2, ensure_ascii=False) + "\n"
        if opts.check:
            assert (OUT / "props.json").read_text(encoding="utf-8") == content, "Exterior manifest differs"
        else:
            (OUT / "props.json").write_text(content, encoding="utf-8")
            for image in bpy.data.images:
                if image.source == "FILE" or image.name.endswith("_atlas"):
                    image.pack()
            scene = bpy.context.scene
            scene.unit_settings.system = "METRIC"
            scene.unit_settings.scale_length = 1
            bpy.context.preferences.filepaths.save_version = 0
            for area in bpy.context.screen.areas:
                if area.type == "VIEW_3D":
                    area.spaces.active.region_3d.view_distance = 170
                    area.spaces.active.region_3d.view_location = Vector((0, 0, 3))
                    area.spaces.active.region_3d.view_rotation = Vector((80, -130, 90)).to_track_quat("Z", "Y")
                    area.spaces.active.shading.type = "MATERIAL"
            bpy.ops.wm.save_as_mainfile(filepath=str(OUT / "fleet.blend"))
    print("Exterior geometry, materials, budgets and delivered bytes validated.", flush=True)


if __name__ == "__main__":
    main()
