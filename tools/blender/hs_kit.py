"""Star Crew's hard-surface kit: what every prop builder shares, so each builder holds only its own
props (CLAUDE.md 6.1, one implementation of every rule).

It owns the material roles and their preview colours, the primitives (prism, obox, hull, ngon,
lathe, frame), the CSG steps (the Prop class: cut, union, chamfer, recess), clean, finish (UV0 in
metres as shipkit.js worldUv, flat shading), check (manifold, no coplanar overlaps, nothing flush
with the floor or the wall), the glb export and its pure-Python read-back (verify_glb), the
manifest writer (starcrew.props/1) and the build's command line (run). It lives in tools/blender
beside the builders that import it:
  tools/blender/build_bridge_props.py   the bridge's furniture (assets/models/bridge)
  tools/blender/build_suite_props.py    the bridge suite's rooms (assets/models/suite)
  tools/blender/build_wall_panels.py    the panel textures (its modules use the primitives and Prop)
How to work this way is the blender-hard-surface skill (.claude/skills/blender-hard-surface).

A builder declares a PropSet (its props, their budgets, its output folder, its manifest's status
and rules) and calls run(prop_set): the command line is the same for every set:
  --check   build in memory and compare with the committed .glb files and props.json; write nothing
  --only    build only these props (the manifest keeps the others' rows)
  --blend   also save the scene with every cutter collection, for looking at the CSG in Blender
            (a debugging view, not a source: it is not reproducible and is not committed)

Conventions (every builder's manifest repeats them in its _rules):
  * Metres. Prop space is the exported glTF frame: +Y up, +Z toward the operator or user (the
    prop's front), +X their right as they face the prop. Blender is Z-up: PROP_TO_BLENDER turns
    every primitive into Blender's frame as it is made (x, y, z to x, -z, y), and the glTF export
    (export_yup) turns it back.
  * The origin is on the floor at the centre of the prop's back.
  * One material per role (ROLES), named for a Star Crew material (data/materials/materials.json)
    or `screen` (emissive, coloured by the page) or `accent` (tinted by the page), or for one of the
    panel build's upholstery layers (`upholstery`, `upholstery_panel`: neutral leather a page tints
    per chair, data/materials/panels.json upholstery).
  * UV0 is in metres, projected per face exactly as shipkit.js worldUv does.
  * Flat shaded (normals split per face), triangulated, every prop one closed manifold solid.
  * A prop over its triangle budget is refused, named with its count, and nothing is written.
"""
import hashlib
import json
import math
import os
import re
import shutil
import struct
import sys
import tempfile

import bpy  # first: with the pip bpy module, bmesh and mathutils exist only once bpy is imported
import bmesh  # noqa: E402
from mathutils import Matrix, Vector  # noqa: E402

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(os.path.dirname(HERE))
MATERIALS_JSON = os.path.join(ROOT, "data", "materials", "materials.json")
TEXTURES = os.path.join(ROOT, "assets", "textures")
SHIPKIT = os.path.join(ROOT, "docs", "mockups", "lib", "shipkit.js")

# Material roles, in slot order (every mesh carries all of them, so a Boolean's material
# transfer never adds a slot and the export's material order is fixed). A glb lists only the roles
# its faces use, so a role appended here changes no prop that does not use it. The two upholstery
# roles (the owner, 2026-10-07: "chairs suck still mainly its a texture problem") are panel layers,
# not materials: channel-stitched padding for seats and backs, padded panels for bolsters, headrests
# and arm pads.
ROLES = ("machinery", "trim", "bulkhead", "hazard", "light_panel", "screen", "accent", "upholstery", "upholstery_panel")
PAGE_COLOURED = {"screen": "screen", "accent": "engineering"}   # role -> shipkit PALETTE key for previews
# Roles drawn on a panel layer (tools/blender/build_wall_panels.py, assets/textures/panels/256/<stem>.png):
# their preview colour is that layer's mean, as a material role's is its layer's.
PANEL_LAYERED = {"upholstery": "upholstery_channel", "upholstery_panel": "upholstery_panel"}
EMISSIVE = {"light_panel", "screen"}

SHOWS = ("console", "upper", "keys")   # what a recorded screen shows (bridge-stations 11.6)
WELD_M = 1e-4          # merge by distance: 0.1 mm, as deckc welds (deck-pipeline section 5)
FLAT_DEG = 1.0         # limited dissolve: faces within 1 degree are one face
CLEAR_M = 0.01         # parallel faces facing the same way stand at least 1 cm apart (CLAUDE.md 8)

# Prop space (x right, y up, z toward the operator) to Blender (x right, y away, z up).
PROP_TO_BLENDER = Matrix(((1, 0, 0, 0), (0, 0, -1, 0), (0, 1, 0, 0), (0, 0, 0, 1)))


def P(v):
    """Blender to prop space (a point or a direction)."""
    return Vector((v.x, v.z, -v.y))


# ----------------------------------------------------------------------------- materials

def _srgb_to_linear(c):
    return c / 12.92 if c <= 0.04045 else ((c + 0.055) / 1.055) ** 2.4


def _layer_mean(name):
    """Mean sRGB colour of a material's texture layer (assets/textures/<name>.png): the preview
    colour comes from the layer itself, so it cannot disagree with it."""
    img = bpy.data.images.load(os.path.join(TEXTURES, name + ".png"), check_existing=True)
    px = [0.0] * (img.size[0] * img.size[1] * 4)
    img.pixels.foreach_get(px)
    n = len(px) // 4
    mean = tuple(round(sum(px[i::4]) / n, 4) for i in range(3))
    bpy.data.images.remove(img)
    return mean


def _palette(key):
    """A colour from shipkit.js PALETTE (the mockups' one table of named colours)."""
    src = open(SHIPKIT, encoding="utf-8").read()
    m = re.search(r"\b%s:\s*0x([0-9a-fA-F]{6})" % re.escape(key), src)
    if not m:
        raise SystemExit(f"[props] {SHIPKIT}: no PALETTE colour {key!r}")
    h = m.group(1)
    return tuple(round(int(h[i:i + 2], 16) / 255.0, 4) for i in (0, 2, 4))


def role_colours():
    """{role: (sRGB colour, emissive)}. Star Crew roles must be materials.json keys or panel layers."""
    known = json.load(open(MATERIALS_JSON, encoding="utf-8"))["materials"]
    out = {}
    for r in ROLES:
        if r in PAGE_COLOURED:
            out[r] = (_palette(PAGE_COLOURED[r]), r in EMISSIVE)
        elif r in PANEL_LAYERED:
            out[r] = (_layer_mean(os.path.join("panels", "256", PANEL_LAYERED[r])), r in EMISSIVE)
        elif r not in known:
            raise SystemExit(f"[props] material role {r!r} is not in {MATERIALS_JSON}")
        else:
            out[r] = (_layer_mean(r), r in EMISSIVE)
    return out


def make_materials():
    for r, (srgb, emissive) in role_colours().items():
        m = bpy.data.materials.new(r)
        m.use_nodes = True
        bsdf = m.node_tree.nodes["Principled BSDF"]
        lin = tuple(_srgb_to_linear(c) for c in srgb) + (1.0,)
        bsdf.inputs["Base Color"].default_value = lin
        bsdf.inputs["Roughness"].default_value = 0.8
        if emissive:
            bsdf.inputs["Emission Color"].default_value = lin
            bsdf.inputs["Emission Strength"].default_value = 1.0
        m.diffuse_color = lin
        # Every prop is a closed solid: back faces are never seen, so the glTF material says
        # doubleSided false and the renderer may cull them (Blender's default exports true).
        m.use_backface_culling = True


def material_slots(me):
    for r in ROLES:
        me.materials.append(bpy.data.materials[r])


# ----------------------------------------------------------------------------- primitives

def _object(bm, name, coll):
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces[:])
    bmesh.ops.transform(bm, matrix=PROP_TO_BLENDER, verts=bm.verts[:])
    me = bpy.data.meshes.new(name)
    bm.to_mesh(me)
    bm.free()
    material_slots(me)
    ob = bpy.data.objects.new(name, me)
    coll.objects.link(ob)
    return ob


def _lift(axis, a, b, t):
    """A profile point (a, b) at position t along the extrusion axis, in prop space."""
    if axis == "x":
        return Vector((t, b, a))      # profile in (z, y)
    if axis == "y":
        return Vector((a, t, b))      # profile in (x, z)
    return Vector((a, b, t))          # axis "z": profile in (x, y)


def prism(coll, name, pts, axis, lo, hi, roles, cap=None):
    """A profile extruded along a prop axis from lo to hi. roles is one role or one per profile
    edge (edge i runs from pts[i] to pts[i + 1]); cap is the role of the two end faces."""
    if isinstance(roles, str):
        roles = [roles] * len(pts)
    cap = cap or roles[0]
    bm = bmesh.new()
    a = [bm.verts.new(_lift(axis, p[0], p[1], lo)) for p in pts]
    b = [bm.verts.new(_lift(axis, p[0], p[1], hi)) for p in pts]
    n = len(pts)
    for i in range(n):
        j = (i + 1) % n
        f = bm.faces.new((a[i], a[j], b[j], b[i]))
        f.material_index = ROLES.index(roles[i])
    for ring in (a, b):
        f = bm.faces.new(ring)
        f.material_index = ROLES.index(cap)
    return _object(bm, name, coll)


def hull(coll, name, pts, role):
    """The convex hull of prop-space points, coplanar triangles merged into one face."""
    bm = bmesh.new()
    for p in pts:
        bm.verts.new(Vector(p))
    res = bmesh.ops.convex_hull(bm, input=bm.verts[:])
    bmesh.ops.delete(bm, geom=[g for g in res["geom_interior"] if isinstance(g, bmesh.types.BMVert)], context="VERTS")
    bmesh.ops.dissolve_limit(bm, angle_limit=math.radians(FLAT_DEG), verts=bm.verts[:], edges=bm.edges[:])
    for f in bm.faces:
        f.material_index = ROLES.index(role)
    return _object(bm, name, coll)


def frame(centre, tilt_back_deg=0.0, yaw_deg=0.0):
    """A local frame at a prop-space point: local +Z is the face normal, tipped up by tilt_back
    (a panel leaning back 20 degrees is tilt 20; a desk top sloping 12 degrees is tilt 78) and
    turned about +Y by yaw."""
    return (Matrix.Translation(Vector(centre)) @ Matrix.Rotation(math.radians(yaw_deg), 4, "Y")
            @ Matrix.Rotation(math.radians(-tilt_back_deg), 4, "X"))


def obox(coll, name, lo, hi, m, roles):
    """A box from local lo to hi in frame m. roles maps a local face ('+x', '-x', '+y', '-y',
    '+z', '-z') to a role, '*' for the rest."""
    bm = bmesh.new()
    bmesh.ops.create_cube(bm, size=1.0)
    for v in bm.verts:
        v.co = Vector(tuple(lo[i] if v.co[i] < 0 else hi[i] for i in range(3)))
    bm.normal_update()
    for f in bm.faces:
        n = f.normal
        k = max(range(3), key=lambda i: abs(n[i]))
        key = ("+" if n[k] > 0 else "-") + "xyz"[k]
        f.material_index = ROLES.index(roles.get(key, roles.get("*")))
    bmesh.ops.transform(bm, matrix=m, verts=bm.verts[:])
    return _object(bm, name, coll)


def ngon(cx, cz, r, sides, flat_back=True):
    """A regular polygon in (x, z) around (cx, cz); with flat_back a flat side faces -z."""
    ph = math.pi / sides if flat_back else 0.0
    return [(cx + r * math.sin(ph + 2 * math.pi * k / sides), cz - r * math.cos(ph + 2 * math.pi * k / sides))
            for k in range(sides)]


def lathe(coll, name, profile, roles, centre_z, r_back, phi0, phi1, steps, cap):
    """A profile in (d, y), d measured in from the back toward the operator, swept about a
    vertical axis through (0, centre_z) from angle phi0 to phi1 (0 is the -Z direction)."""
    bm = bmesh.new()
    rings = []
    for k in range(steps + 1):
        phi = phi0 + (phi1 - phi0) * k / steps
        s, c = math.sin(phi), math.cos(phi)
        rings.append([bm.verts.new(Vector(((r_back - d) * s, y, centre_z - (r_back - d) * c))) for d, y in profile])
    n = len(profile)
    for k in range(steps):
        for i in range(n):
            j = (i + 1) % n
            f = bm.faces.new((rings[k][i], rings[k][j], rings[k + 1][j], rings[k + 1][i]))
            f.material_index = ROLES.index(roles[i])
    for ring in (rings[0], rings[-1]):
        f = bm.faces.new(ring)
        f.material_index = ROLES.index(cap)
    return _object(bm, name, coll)


def in_frame(m, pts):
    """Prop-space points of points given in frame m (for a hull built in a desk's frame)."""
    return [tuple(m @ Vector(q)) for q in pts]


def triangle_inset(tri, d):
    """A triangle shrunk by moving every edge d inward, or None when too small for it."""
    a, b, c = (Vector((p[0], p[1])) for p in tri)
    s = (b - a).length + (c - b).length + (a - c).length
    area = abs((b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)) / 2
    r = 2 * area / s                        # the inradius
    if r < 1.6 * d:
        return None
    incentre = (a * (c - b).length + b * (a - c).length + c * (b - a).length) / s
    k = (r - d) / r                         # shrinking about the incentre moves every edge by d
    return [tuple(incentre + (p - incentre) * k) for p in (a, b, c)]


def clip_polygon(poly, a, b, c):
    """Keep the part of a convex 2D polygon where a*x + b*y <= c."""
    out = []
    for i in range(len(poly)):
        p, q = poly[i], poly[(i + 1) % len(poly)]
        fp, fq = a * p[0] + b * p[1] - c, a * q[0] + b * q[1] - c
        if fp <= 0:
            out.append(p)
        if (fp <= 0) != (fq <= 0):
            t = fp / (fp - fq)
            out.append((p[0] + t * (q[0] - p[0]), p[1] + t * (q[1] - p[1])))
    return out


# ----------------------------------------------------------------------------- CSG steps

def apply_modifiers(ob):
    """Bake an object's modifier stack into its mesh (context-free, so it runs headless)."""
    deps = bpy.context.evaluated_depsgraph_get()
    me = bpy.data.meshes.new_from_object(ob.evaluated_get(deps), preserve_all_data_layers=True, depsgraph=deps)
    old = ob.data
    ob.modifiers.clear()
    ob.data = me
    me.name = ob.name
    if old.users == 0:
        bpy.data.meshes.remove(old)


class Prop:
    """One prop: its object, its cutter collections and the facts the manifest records.

    wall: the prop stands with its back flat on a wall at z = 0 (the read-back checks the back,
    and check() refuses a face toward the room within 1 cm of the wall). back_at_z0: the back is
    at z = 0 without standing on a wall (a table's back edge). shows: what a recorded screen may
    show (SHOWS unless a set adds its own kinds)."""

    def __init__(self, name, presents, anchor, wall=False, back_at_z0=False, shows=SHOWS):
        self.name = name
        self.presents = presents
        self.anchor = anchor
        self.wall = wall
        self.back_at_z0 = back_at_z0
        self.shows = tuple(shows)
        self.coll = bpy.data.collections.new(name)
        bpy.context.scene.collection.children.link(self.coll)
        self.cutters = bpy.data.collections.new(name + ".cutters")
        self.coll.children.link(self.cutters)
        self.cutters.hide_render = True
        self.body = None
        self.operators = []     # floor points where operators sit or stand (a chair's: under its seat)
        self.operators_yaw = None   # each operator's facing, degrees about +Y (0 faces +Z), when recorded
        self.seat = None        # a chair's sitting point, top of the cushion
        self.screens = []
        self.steps = []         # the CSG history, for the manifest and the skill
        self.variant_of = None  # a variant's base prop: the same body with a station's own controls
        self.stations = []      # the stations a variant is for (bridge_variants.json station ids)
        self.controls = []      # the hand controls modelled on it, in words, for the manifest
        self.recesses = []      # every recess cut: (label, frame, width, height, depth, floor role)
        self.decor = []         # the prop's own details for its atlas bake: functions of a Detail, run first
        self.finish_of = {}     # a role baked in another finish than prop_atlas.json roles names

    def _operands(self, label, objs):
        c = bpy.data.collections.new(f"{self.name}.{label}")
        self.cutters.children.link(c)
        for o in objs:
            for u in list(o.users_collection):
                u.objects.unlink(o)
            c.objects.link(o)
            o.display_type = "WIRE"
            o.hide_render = True
        return c

    # Named primitives in this prop's collection (<prop>.<what>), in prop space.
    def box(self, what, lo, hi, roles="machinery", m=None):
        """An axis-aligned box from lo to hi (or a box in frame m); roles is one role or a face map."""
        return obox(self.coll, f"{self.name}.{what}", lo, hi, m if m is not None else Matrix(),
                    {"*": roles} if isinstance(roles, str) else roles)

    def prism(self, what, pts, axis, lo, hi, roles, cap=None):
        return prism(self.coll, f"{self.name}.{what}", pts, axis, lo, hi, roles, cap)

    def hull(self, what, pts, role):
        return hull(self.coll, f"{self.name}.{what}", pts, role)

    def boolean(self, target, op, label, objs):
        """One Boolean modifier: operand collection `label`, Exact solver, materials from the
        operands' faces; applied at once."""
        mod = target.modifiers.new(label, "BOOLEAN")
        mod.operation = op
        mod.operand_type = "COLLECTION"
        mod.collection = self._operands(label, objs)
        mod.solver = "EXACT"
        mod.material_mode = "TRANSFER"
        apply_modifiers(target)
        self.steps.append(f"{op.lower()} {label} ({len(objs)})")
        return target

    def cut(self, target, label, objs):
        return self.boolean(target, "DIFFERENCE", label, objs)

    def union(self, target, label, objs):
        return self.boolean(target, "UNION", label, objs)

    def chamfer(self, ob, label, width, where, min_deg=25.0, role=None):
        """A 1-segment Bevel on the convex edges whose prop-space midpoint, direction and face
        normals `where(mid, d, n1, n2)` accepts. Marked by the bevel_weight_edge attribute,
        limit method Weight, so no other edge is touched."""
        me = ob.data
        if "bevel_weight_edge" not in me.attributes:
            me.attributes.new("bevel_weight_edge", "FLOAT", "EDGE")
        bm = bmesh.new()
        bm.from_mesh(me)
        bm.normal_update()
        lay = bm.edges.layers.float.get("bevel_weight_edge")
        count = 0
        for e in bm.edges:
            e[lay] = 0.0
            if len(e.link_faces) != 2 or e.calc_face_angle_signed(0.0) < math.radians(min_deg):
                continue
            a, b = P(e.verts[0].co), P(e.verts[1].co)
            n1, n2 = P(e.link_faces[0].normal), P(e.link_faces[1].normal)
            if where((a + b) / 2, (b - a).normalized(), n1, n2):
                e[lay] = 1.0
                count += 1
        bm.to_mesh(me)
        bm.free()
        if count == 0:
            raise SystemExit(f"[props] {self.name}: chamfer {label} matched no edge")
        mod = ob.modifiers.new(label, "BEVEL")
        mod.segments = 1
        mod.width = width
        mod.limit_method = "WEIGHT"
        mod.edge_weight = "bevel_weight_edge"
        mod.use_clamp_overlap = True
        mod.miter_outer = "MITER_SHARP"
        mod.harden_normals = False
        if role:
            mod.material = ROLES.index(role)
        apply_modifiers(ob)
        if "bevel_weight_edge" in ob.data.attributes:
            ob.data.attributes.remove(ob.data.attributes["bevel_weight_edge"])
        self.steps.append(f"chamfer {label} ({count} edges, {width * 1000:.0f} mm)")
        return ob

    def recess(self, coll_objs, label, m, w, h, depth, wall_role="machinery", floor_role="screen", record=True,
               shows=None, half=None):
        """A recess cutter: a box in frame m whose back face, `depth` behind the panel, becomes
        the recess floor (a screen, unless floor_role says otherwise) and whose sides become its
        walls. It reaches 5 cm out of the panel so no cutter face lies on the panel's plane.
        A recorded screen says what it shows (bridge-stations 11.6, assets/textures/screens/
        screens.json): "console" (its station's main image), "upper" (half `half` of the upper
        image) or "keys" (a key panel), or a kind its set adds; up is the in-plane direction of
        its height."""
        ob = obox(self.coll, f"{self.name}.{label}", (-w / 2, -h / 2, -depth), (w / 2, h / 2, 0.05), m,
                  {"-z": floor_role, "*": wall_role})
        coll_objs.append(ob)
        self.recesses.append((label, m.copy(), w, h, depth, floor_role))
        if record:
            if shows not in self.shows:
                raise SystemExit(f"[props] {self.name}: screen {label} shows {shows!r}, not one of {self.shows}")
            c = m @ Vector((0, 0, -depth))
            n = (m.to_3x3() @ Vector((0, 0, 1))).normalized()
            up = (m.to_3x3() @ Vector((0, 1, 0))).normalized()
            row = {"label": label, "shows": shows, "centre_m": r3(c), "normal": r3(n), "up": r3(up),
                   "width_m": round(w, 4), "height_m": round(h, 4)}
            if half is not None:
                row["half"] = half
            self.screens.append(row)
        return ob


def r3(v):
    return [round(float(c), 4) + 0.0 for c in v]


def clean(ob):
    """After the booleans: weld, drop degenerate geometry, merge coplanar faces of one material,
    triangulate. Returns nothing; check() judges the result."""
    me = ob.data
    bm = bmesh.new()
    bm.from_mesh(me)
    bmesh.ops.remove_doubles(bm, verts=bm.verts[:], dist=WELD_M)
    bmesh.ops.dissolve_degenerate(bm, dist=WELD_M, edges=bm.edges[:])
    bmesh.ops.dissolve_limit(bm, angle_limit=math.radians(FLAT_DEG), use_dissolve_boundaries=False,
                             verts=bm.verts[:], edges=bm.edges[:], delimit={"MATERIAL"})
    loose = [v for v in bm.verts if not v.link_faces]
    if loose:
        bmesh.ops.delete(bm, geom=loose, context="VERTS")
    bmesh.ops.triangulate(bm, faces=bm.faces[:], quad_method="BEAUTY", ngon_method="BEAUTY")
    bmesh.ops.dissolve_degenerate(bm, dist=WELD_M, edges=bm.edges[:])
    bmesh.ops.triangulate(bm, faces=[f for f in bm.faces if len(f.verts) > 3], quad_method="BEAUTY",
                          ngon_method="BEAUTY")
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces[:])
    bm.to_mesh(me)
    bm.free()


def world_uv(p, n):
    """shipkit.js worldUv, in metres: x, z on floors and ceilings; along the face's horizontal
    tangent and y otherwise."""
    if abs(n.y) > 0.75:
        return p.x, p.z
    lh = math.hypot(n.x, n.z) or 1.0
    tx, tz = -n.z / lh, n.x / lh
    return p.x * tx + p.z * tz, p.y


def finish(ob):
    """UV0 in metres (V stored flipped, because the glTF exporter writes 1 - v) and flat shading."""
    me = ob.data
    uv = me.uv_layers.new(name="UVMap")
    for poly in me.polygons:
        n = P(poly.normal)
        for li in poly.loop_indices:
            u, v = world_uv(P(me.vertices[me.loops[li].vertex_index].co), n)
            uv.data[li].uv = (u, 1.0 - v)
        poly.use_smooth = False


# ----------------------------------------------------------------------------- checks

def _tri_area(a, b, c):
    return (b - a).cross(c - a).length / 2


def _overlap_2d(t1, t2):
    """Area of the intersection of two 2D triangles (convex clipping)."""
    def clip(poly, a, b):
        out = []
        side = lambda p: (b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0])
        for i in range(len(poly)):
            p, q = poly[i], poly[(i + 1) % len(poly)]
            sp, sq = side(p), side(q)
            if sp >= 0:
                out.append(p)
            if (sp >= 0) != (sq >= 0):
                t = sp / (sp - sq)
                out.append((p[0] + t * (q[0] - p[0]), p[1] + t * (q[1] - p[1])))
        return out

    def ccw(t):
        a = (t[1][0] - t[0][0]) * (t[2][1] - t[0][1]) - (t[1][1] - t[0][1]) * (t[2][0] - t[0][0])
        return t if a > 0 else [t[0], t[2], t[1]]
    poly = ccw(t1)
    t2 = ccw(t2)
    for i in range(3):
        poly = clip(poly, t2[i], t2[(i + 1) % 3])
        if not poly:
            return 0.0
    return abs(sum(poly[i][0] * poly[(i + 1) % len(poly)][1] - poly[(i + 1) % len(poly)][0] * poly[i][1]
                   for i in range(len(poly)))) / 2


def check(prop):
    """Judge the finished mesh: manifold, no degenerate triangles, no coplanar overlapping faces
    facing the same way, nothing flush with the floor or (for a wall-standing prop) the wall
    facing the same way they do. Returns (triangles, [problems])."""
    ob = prop.body
    bm = bmesh.new()
    bm.from_mesh(ob.data)
    bm.normal_update()
    problems = []
    bad_edges = [e for e in bm.edges if len(e.link_faces) != 2]
    if bad_edges:
        problems.append(f"{len(bad_edges)} non-manifold edges")
    bad_verts = [v for v in bm.verts if not v.is_manifold]
    if bad_verts:
        problems.append(f"{len(bad_verts)} non-manifold vertices")
    tris = []
    for f in bm.faces:
        if len(f.verts) != 3:
            problems.append("a face that is not a triangle")
            continue
        pts = [P(v.co) for v in f.verts]
        area = _tri_area(*pts)
        if area < 1e-7:
            problems.append(f"degenerate triangle near {r3(pts[0])}")
            continue
        n = P(f.normal).normalized()
        tris.append((pts, n, n.dot(pts[0]), f.material_index))
        if n.y > 0.999 and max(p.y for p in pts) < CLEAR_M:
            problems.append(f"upward face within 1 cm of the floor near {r3(pts[0])}")
        if prop.wall and n.z > 0.999 and max(p.z for p in pts) < CLEAR_M:
            problems.append(f"face within 1 cm of the wall facing into the room near {r3(pts[0])}")
    # coplanar or nearly parallel (under CLEAR_M apart) overlapping faces facing the same way
    for i in range(len(tris)):
        pi, ni, di, _ = tris[i]
        for j in range(i + 1, len(tris)):
            pj, nj, dj, _ = tris[j]
            if ni.dot(nj) < 0.9999 or abs(di - dj) >= CLEAR_M:
                continue
            u = ni.orthogonal().normalized()
            w = ni.cross(u)
            a = _overlap_2d([(p.dot(u), p.dot(w)) for p in pi], [(p.dot(u), p.dot(w)) for p in pj])
            if a > 1e-6:
                problems.append(f"faces {abs(di - dj) * 1000:.1f} mm apart overlap by {a * 1e4:.1f} cm2 near {r3(pi[0])}")
    bm.free()
    return len(tris), problems


# ----------------------------------------------------------------------------- shading
# Shader node helpers and the procedural wear, shared by the panel textures' bake
# (tools/blender/build_wall_panels.py) and every prop's atlas bake below: one implementation.

def lin(c):
    return tuple(_srgb_to_linear(v) for v in c)


class Nodes:
    """A small helper for shader node trees: sockets or constants in, sockets out."""

    def __init__(self, mat):
        mat.use_nodes = True
        self.nt = mat.node_tree
        for n in list(self.nt.nodes):
            self.nt.nodes.remove(n)

    def new(self, kind, **props):
        n = self.nt.nodes.new(kind)
        for k, v in props.items():
            setattr(n, k, v)
        return n

    def feed(self, sock, v):
        if isinstance(v, bpy.types.NodeSocket):
            self.nt.links.new(v, sock)
        else:
            sock.default_value = v

    def math(self, op, a, b=0.0, clamp=False):
        n = self.new("ShaderNodeMath", operation=op, use_clamp=clamp)
        self.feed(n.inputs[0], a)
        self.feed(n.inputs[1], b)
        return n.outputs[0]

    def mix(self, fac, a, b, blend="MIX"):
        n = self.new("ShaderNodeMix", data_type="RGBA", blend_type=blend, clamp_factor=True)
        ins = {s.identifier: s for s in n.inputs}
        self.feed(ins["Factor_Float"], fac)
        self.feed(ins["A_Color"], a)
        self.feed(ins["B_Color"], b)
        return {s.identifier: s for s in n.outputs}["Result_Color"]

    def ramp(self, v, lo, hi):
        n = self.new("ShaderNodeMapRange", clamp=True)
        self.feed(n.inputs[0], v)
        n.inputs[1].default_value = lo
        n.inputs[2].default_value = hi
        n.inputs[3].default_value = 0.0
        n.inputs[4].default_value = 1.0
        return n.outputs[0]

    def noise(self, vec, scale, detail=4.0, rough=0.55):
        """Fractal noise at vec: a socket (3D noise) or (socket, w) for 4D noise on a torus."""
        if isinstance(vec, tuple):
            n = self.new("ShaderNodeTexNoise", noise_dimensions="4D")
            self.feed(n.inputs["Vector"], vec[0])
            self.feed(n.inputs["W"], vec[1])
        else:
            n = self.new("ShaderNodeTexNoise", noise_dimensions="3D")
            self.feed(n.inputs["Vector"], vec)
        n.inputs["Scale"].default_value = scale
        n.inputs["Detail"].default_value = detail
        n.inputs["Roughness"].default_value = rough
        return n.outputs["Fac"]

    def xyz(self, x, y, z):
        n = self.new("ShaderNodeCombineXYZ")
        for i, v in enumerate((x, y, z)):
            self.feed(n.inputs[i], v)
        return n.outputs[0]

    def out(self, shader):
        o = self.new("ShaderNodeOutputMaterial")
        self.nt.links.new(shader, o.inputs["Surface"])


def worn_blotchy(N, base, F, wears, v):
    """A colour under wear for surfaces that do not hang: soft grime blotches, crevice occlusion and,
    where wears, edge wear to bare metal (F's colours_srgb metal), scuffs and rust in patches and
    specks (colours_srgb rust). F holds wear (panels.json's wear keys) and colours_srgb; v is the
    position the noise reads."""
    W, C = F["wear"], F["colours_srgb"]
    s = W["grime_scale_per_m"]
    col = N.mix(N.math("MULTIPLY", N.ramp(N.noise(v, s, 6.0, 0.6), 0.38, 0.78), W["grime"]), base, (0.0, 0.0, 0.0, 1.0))
    ao = N.new("ShaderNodeAmbientOcclusion", samples=8, only_local=False)
    ao.inputs["Distance"].default_value = W["crevice_m"]
    col = N.mix(N.math("MULTIPLY", N.math("SUBTRACT", 1.0, ao.outputs["AO"]), W["crevice"]), col, (0.0, 0.0, 0.0, 1.0))
    if wears and W["edge"] > 0:
        bev = N.new("ShaderNodeBevel", samples=8)
        bev.inputs["Radius"].default_value = 0.006
        geo = N.new("ShaderNodeNewGeometry")
        dot = N.new("ShaderNodeVectorMath", operation="DOT_PRODUCT")
        N.nt.links.new(bev.outputs["Normal"], dot.inputs[0])
        N.nt.links.new(geo.outputs["Normal"], dot.inputs[1])
        edge = N.ramp(dot.outputs["Value"], 0.97, 0.80)
        chips = N.ramp(N.noise(v, 9.0, 3.0, 0.7), 0.42, 0.62)
        worn_c = lin([min(1.0, c * 1.25 + 0.06) for c in C["metal"]]) + (1.0,)
        col = N.mix(N.math("MULTIPLY", N.math("MULTIPLY", edge, chips), W["edge"]), col, worn_c)
    if wears and W["streaks"] > 0:
        # scuffs and dirt: small, sharp blotches
        blot = N.ramp(N.noise(v, 6.0, 3.0, 0.6), 0.60, 0.74)
        col = N.mix(N.math("MULTIPLY", blot, W["streaks"]), col, (0.0, 0.0, 0.0, 1.0))
    if wears and W["rust"] > 0:
        patch = N.ramp(N.noise(v, 2.2, 4.0, 0.62), 0.56, 0.74)
        col = N.mix(N.math("MULTIPLY", patch, W["rust"]), col, lin(C["rust"]) + (1.0,))
        speck = N.ramp(N.noise(v, 22.0, 2.0, 0.6), 0.66, 0.74)
        col = N.mix(N.math("MULTIPLY", speck, W["rust"] * 0.6), col, lin(C["rust"]) + (1.0,))
    return col


def diffuse(N, col, normal=None):
    d = N.new("ShaderNodeBsdfDiffuse")
    N.feed(d.inputs["Color"], col)
    d.inputs["Roughness"].default_value = 1.0
    if normal is not None:
        N.feed(d.inputs["Normal"], normal)
    N.out(d.outputs[0])


def emission(N, col, strength=1.0):
    e = N.new("ShaderNodeEmission")
    N.feed(e.inputs["Color"], col)
    e.inputs["Strength"].default_value = strength
    N.out(e.outputs[0])


# ----------------------------------------------------------------------------- the atlas: UV1
# The owner, 2026-10-07: "the textures on the console man, they need to be full on custom details,
# youre using generic textures on them". Every prop gets a second UV map (glTF TEXCOORD_1) and an
# atlas baked onto it from a detail version of the prop (data/materials/prop_atlas.json).

ATLAS_JSON = os.path.join(ROOT, "data", "materials", "prop_atlas.json")
PANELS_JSON = os.path.join(ROOT, "data", "materials", "panels.json")
ATLAS_UV = "Atlas"
FINISH_KINDS = ("paint", "metal", "brushed", "hazard", "flat", "glow", "layer", "wood", "laminate", "fabric")
WEARING = {"paint", "metal", "brushed", "hazard"}       # kinds that take edge wear, scuffs and rust


def _atlas_fail(msg):
    raise SystemExit(f"{os.path.relpath(ATLAS_JSON, ROOT)}: {msg}")


def _exact(obj, keys, where):
    if not isinstance(obj, dict):
        _atlas_fail(f"{where} must be an object")
    extra = sorted(k for k in obj if k not in keys and not k.startswith("_"))
    missing = sorted(k for k in keys if k not in obj)
    if extra or missing:
        _atlas_fail(f"{where}: " + "; ".join(([f"unknown key {', '.join(extra)}"] if extra else [])
                                             + ([f"missing {', '.join(missing)}"] if missing else [])))


def _num(v, where, lo=None, hi=None):
    if isinstance(v, bool) or not isinstance(v, (int, float)) or not math.isfinite(v):
        _atlas_fail(f"{where} must be a finite number, got {v!r}")
    if (lo is not None and v < lo) or (hi is not None and v > hi):
        _atlas_fail(f"{where} must be in {lo}-{hi}, got {v}")
    return v


def _rgb(c, where):
    if not (isinstance(c, list) and len(c) == 3):
        _atlas_fail(f"{where} must be [r, g, b]")
    for v in c:
        _num(v, where, 0, 1)


ATLAS = None


def load_atlas():
    """data/materials/prop_atlas.json, validated (unknown or missing keys stop the build), with the
    upholstery tints from panels.json (the one source of a chair's colour)."""
    global ATLAS
    if ATLAS is not None:
        return ATLAS
    d = json.load(open(ATLAS_JSON, encoding="utf-8"))
    if d.get("schema") != "starcrew.prop-atlas/1":
        _atlas_fail("schema must be 'starcrew.prop-atlas/1'")
    _exact(d, {"schema", "status", "atlas", "bake", "wear", "sets", "edge_metal_srgb", "rust_srgb", "roles", "finishes"}, "top level")
    _exact(d["atlas"], {"px", "px_max", "min_px_per_m", "bake_scale", "margin_px", "max_px_per_m", "colours", "dir"}, "atlas")
    if d["atlas"]["bake_scale"] not in (1, 2, 4):
        _atlas_fail("atlas.bake_scale must be 1, 2 or 4")
    if d["atlas"]["px_max"] < d["atlas"]["px"] or d["atlas"]["px_max"] not in [d["atlas"]["px"] * 2 ** k for k in range(5)]:
        _atlas_fail("atlas.px_max must be atlas.px times a power of two")
    _num(d["atlas"]["min_px_per_m"], "atlas.min_px_per_m", 0, 1024)
    _num(d["atlas"]["margin_px"], "atlas.margin_px", 1, 16)
    _num(d["atlas"]["max_px_per_m"], "atlas.max_px_per_m", 8, 1024)
    _exact(d["bake"], {"samples", "mask_samples", "cage_m", "ambient", "key_tangent", "relief_max"}, "bake")
    _num(d["bake"]["ambient"], "bake.ambient", 0, 1)
    _num(d["bake"]["cage_m"], "bake.cage_m", 0.002, 0.2)
    if len(d["bake"]["key_tangent"]) != 3 or d["bake"]["key_tangent"][2] <= 0:
        _atlas_fail("bake.key_tangent must be [x, y, z] with z above 0 (out of the face)")
    for k, W in d["wear"].items():
        if k.startswith("_"):
            continue
        _exact(W, {"grime", "grime_scale_per_m", "crevice", "crevice_m", "edge", "streaks", "rust"}, f"wear.{k}")
        for kk, v in W.items():
            _num(v, f"wear.{k}.{kk}", 0, 100)
    for s, w in d["sets"].items():
        if w not in d["wear"]:
            _atlas_fail(f"sets.{s} names no wear: {w}")
    _rgb(d["edge_metal_srgb"], "edge_metal_srgb")
    _rgb(d["rust_srgb"], "rust_srgb")
    _exact(d["roles"], set(ROLES), "roles")
    for r, f in d["roles"].items():
        if f not in d["finishes"]:
            _atlas_fail(f"roles.{r} names no finish: {f}")
    for k, F in d["finishes"].items():
        if k.startswith("_"):
            continue
        kind = F.get("kind")
        if kind not in FINISH_KINDS:
            _atlas_fail(f"finishes.{k}.kind must be one of {', '.join(FINISH_KINDS)}")
        keys = {"kind", "layer"} if kind == "layer" else {"kind", "srgb", "srgb_b"} if kind == "hazard" else {"kind", "srgb"}
        _exact(F, keys, f"finishes.{k}")
        for c in ("srgb", "srgb_b"):
            if c in F:
                _rgb(F[c], f"finishes.{k}.{c}")
    d["tints"] = json.load(open(PANELS_JSON, encoding="utf-8"))["upholstery"]["tints_srgb"]
    ATLAS = d
    return d


def _plane_groups(bm, by_material):
    """bm's faces grouped into planar pieces: faces of one plane (normals within 1e-5, offsets within
    0.01 mm) joined through shared vertices, and with by_material of one material too. Returns lists of
    faces in a stable order (each group by its first face index, groups by their first face)."""
    bm.faces.ensure_lookup_table()
    parent = list(range(len(bm.faces)))

    def find(i):
        while parent[i] != i:
            parent[i] = parent[parent[i]]
            i = parent[i]
        return i
    planes = [(f.normal.copy(), f.normal.dot(f.verts[0].co)) for f in bm.faces]
    for v in bm.verts:
        fs = sorted(f.index for f in v.link_faces)
        for a in range(len(fs)):
            for b in range(a + 1, len(fs)):
                fa, fb = fs[a], fs[b]
                (na, da), (nb, db) = planes[fa], planes[fb]
                if na.dot(nb) < 1.0 - 1e-5 or abs(da - db) > 1e-5:
                    continue
                if by_material and bm.faces[fa].material_index != bm.faces[fb].material_index:
                    continue
                ra, rb = find(fa), find(fb)
                if ra != rb:
                    parent[max(ra, rb)] = min(ra, rb)
    groups = {}
    for f in bm.faces:
        groups.setdefault(find(f.index), []).append(f)
    return [groups[k] for k in sorted(groups)]


def face_axes(n):
    """A face's frame in prop space from its normal: (right, up, n). up is world up on the face, or
    on a top face away from the prop's front (-Z), on an underside toward it; right = up x n."""
    if abs(n.y) < 0.75:
        up = Vector((0.0, 1.0, 0.0)) - n * n.y
    else:
        ref = Vector((0.0, 0.0, -1.0)) if n.y > 0 else Vector((0.0, 0.0, 1.0))
        up = ref - n * ref.dot(n)
    up.normalize()
    return up.cross(n).normalized(), up, n


def _hull2(pts):
    pts = sorted(set((round(x, 9), round(y, 9)) for x, y in pts))
    if len(pts) < 3:
        return pts

    def cross(o, a, b):
        return (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0])
    lo, hi = [], []
    for p in pts:
        while len(lo) >= 2 and cross(lo[-2], lo[-1], p) <= 0:
            lo.pop()
        lo.append(p)
    for p in reversed(pts):
        while len(hi) >= 2 and cross(hi[-2], hi[-1], p) <= 0:
            hi.pop()
        hi.append(p)
    return lo[:-1] + hi[:-1]


def _min_rect(pts):
    """The smallest-area rectangle round 2D points: (cos, sin) of its axis, its min corner and size in
    that axis' frame. Rotating calipers over the hull's edges; ties keep the first edge."""
    hull_pts = _hull2(pts)
    best = None
    cands = []
    for i in range(len(hull_pts)):
        a, b = hull_pts[i], hull_pts[(i + 1) % len(hull_pts)]
        dx, dy = b[0] - a[0], b[1] - a[1]
        ln = math.hypot(dx, dy)
        if ln > 1e-9:
            cands.append((dx / ln, dy / ln))
    cands.append((1.0, 0.0))
    for c, s in cands:
        xs = [x * c + y * s for x, y in hull_pts or pts]
        ys = [-x * s + y * c for x, y in hull_pts or pts]
        area = (max(xs) - min(xs)) * (max(ys) - min(ys))
        if best is None or area < best[0] - 1e-12:
            best = (area, c, s, min(xs), min(ys), max(xs) - min(xs), max(ys) - min(ys))
    return best[1:]


def _chart_mask(tris, k, margin):
    """A chart's pixels at k px per metre: every pixel its triangles touch (each triangle grown by half
    a pixel's diagonal), dilated by margin pixels. tris are 2D triangles from the chart's corner (0, 0).
    Returns the boolean mask (rows down from the top) and the pixel offset of the chart's (0, 0)."""
    import numpy as np
    pts = [(x * k, y * k) for t in tris for x, y in t]
    w = int(math.ceil(max(p_[0] for p_ in pts))) + 1 + 2 * margin
    h = int(math.ceil(max(p_[1] for p_ in pts))) + 1 + 2 * margin
    m = np.zeros((h, w), bool)
    o = margin + 0.5
    jj, ii = np.mgrid[0:h, 0:w]
    cx, cy = ii + 0.5, jj + 0.5
    for t in tris:
        a, b, c = [(x * k + o, y * k + o) for x, y in t]
        area = (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
        if area < 0:
            b, c = c, b
        inside = np.ones((h, w), bool)
        for (px, py), (qx, qy) in ((a, b), (b, c), (c, a)):
            ln = math.hypot(qx - px, qy - py) or 1.0
            inside &= ((qx - px) * (cy - py) - (qy - py) * (cx - px)) >= -0.71 * ln
        m |= inside
    for _ in range(margin):
        g = m.copy()
        g[1:, :] |= m[:-1, :]
        g[:-1, :] |= m[1:, :]
        g[:, 1:] |= m[:, :-1]
        g[:, :-1] |= m[:, 1:]
        g[1:, 1:] |= m[:-1, :-1]
        g[:-1, :-1] |= m[1:, 1:]
        g[1:, :-1] |= m[:-1, 1:]
        g[:-1, 1:] |= m[1:, :-1]
        m = g
    return m


def _raster_pack(masks, side):
    """Place chart masks in a side x side atlas, largest first, each at the free spot nearest the top
    and then the left, turned 90 degrees when that sits higher: overlap with what is placed is counted
    for every offset at once by FFT correlation (the counts are whole numbers, read with a 0.5
    threshold, so rounding cannot move a chart). Returns {index: (x, y, turned)} or None."""
    import numpy as np
    occ = np.zeros((side, side), np.float64)
    order = sorted(range(len(masks)), key=lambda i: (-int(masks[i].sum()), i))
    out = {}
    for i in order:
        best = None
        F = np.fft.rfft2(occ)
        for turned in (False, True):
            mk = masks[i].T[::-1] if turned else masks[i]
            h, w = mk.shape
            if h > side or w > side:
                continue
            pad = np.zeros((side, side), np.float64)
            pad[:h, :w] = mk
            corr = np.fft.irfft2(F * np.conj(np.fft.rfft2(pad)), s=(side, side))
            free = corr[:side - h + 1, :side - w + 1] < 0.5
            if not free.any():
                continue
            ys, xs = np.nonzero(free)
            j = np.lexsort((xs, ys + h))[0]
            cand = (int(ys[j]) + h, int(xs[j]), turned, int(ys[j]), int(xs[j]), mk)
            if best is None or cand[:3] < best[:3]:
                best = cand
        if best is None:
            return None
        _, _, turned, y, x, mk = best
        occ[y:y + mk.shape[0], x:x + mk.shape[1]] += mk
        out[i] = (x, y, turned)
    return out


def _unfold(groups, bm, max_len):
    """Join planar charts into bigger ones by unfolding a neighbour flat across the edge it shares
    (a rigid turn in the plane, so nothing stretches), longest shared edges first, whenever the
    result does not overlap itself and fills its smallest rectangle (face area over rectangle area) at
    least 0.9 times as well as the emptier of the two did, and at least half (so a strip of chamfers
    joins its face, but a lathe's facets do not fan out into an arc that packs badly), and no side of
    that rectangle is longer than max_len metres (so a chart still fits the atlas). groups are [{"faces", "pos": {vertex: (x, y)}, "tris", "fixed"}];
    charts with a fixed scale (unseen faces) stay alone. Returns the joined charts in a stable order."""
    owner = {}
    for gi, g in enumerate(groups):
        for fi in g["faces"]:
            owner[fi] = gi
    cand = []
    for e in bm.edges:
        if len(e.link_faces) != 2:
            continue
        a, b = owner[e.link_faces[0].index], owner[e.link_faces[1].index]
        if a == b or groups[a]["fixed"] is not None or groups[b]["fixed"] is not None or groups[a]["lamp"] or groups[b]["lamp"]:
            continue
        cand.append((-round(e.calc_length(), 6), min(a, b), max(a, b), e.verts[0].index, e.verts[1].index))
    cand.sort()
    parent = list(range(len(groups)))

    def find(i):
        while parent[i] != i:
            i = parent[i]
        return i
    def rect(pos):
        r = _min_rect(list(pos.values()))
        return r[4] * r[5] if max(r[4], r[5]) <= max_len else float("inf")

    def tri_area(t):
        return abs((t[1][0] - t[0][0]) * (t[2][1] - t[0][1]) - (t[1][1] - t[0][1]) * (t[2][0] - t[0][0])) / 2
    charts = {i: {"faces": list(g["faces"]), "pos": dict(g["pos"]), "tris": [list(t) for t in g["tris"]], "fixed": g["fixed"],
                  "lamp": g["lamp"]} for i, g in enumerate(groups)}
    for ch in charts.values():
        ch["rect"] = rect(ch["pos"])
        ch["area"] = sum(tri_area(t) for t in ch["tris"])
    for _, a, b, v1, v2 in cand:
        ra, rb = find(a), find(b)
        if ra == rb:
            continue
        A_, B_ = charts[ra], charts[rb]
        if v1 not in A_["pos"] or v2 not in A_["pos"] or v1 not in B_["pos"] or v2 not in B_["pos"]:
            continue
        (ax1, ay1), (ax2, ay2) = A_["pos"][v1], A_["pos"][v2]
        (bx1, by1), (bx2, by2) = B_["pos"][v1], B_["pos"][v2]
        ang = math.atan2(ay2 - ay1, ax2 - ax1) - math.atan2(by2 - by1, bx2 - bx1)
        c, s_ = math.cos(ang), math.sin(ang)

        def tf(q):
            x, y = q[0] - bx1, q[1] - by1
            return (ax1 + x * c - y * s_, ay1 + x * s_ + y * c)
        moved = [[tf(q) for q in t] for t in B_["tris"]]
        pos = dict(A_["pos"])
        for v, q in B_["pos"].items():
            pos.setdefault(v, tf(q))
        joined = rect(pos)
        fill = (A_["area"] + B_["area"]) / max(joined, 1e-12)
        if fill < 0.5 or fill < 0.9 * min(A_["area"] / max(A_["rect"], 1e-12), B_["area"] / max(B_["rect"], 1e-12)):
            continue
        if any(_overlap_2d(t1, t2) > 1e-9 for t1 in moved for t2 in A_["tris"]):
            continue
        A_["faces"] += B_["faces"]
        A_["tris"] += moved
        A_["pos"] = pos
        A_["rect"] = joined
        A_["area"] += B_["area"]
        parent[rb] = ra
        del charts[rb]
    return [charts[k] for k in sorted(charts)]


def atlas_uv(p, A):
    """The atlas UV (atlas_uv_at) in the smallest square, from atlas.px doubling up to px_max, that
    gives the prop at least min_px_per_m. Returns its facts with the square's size, px."""
    side = A["atlas"]["px"]
    while True:
        facts = atlas_uv_at(p, A, side)
        if facts["px_per_m"] >= A["atlas"]["min_px_per_m"] or side >= A["atlas"]["px_max"]:
            facts["px"] = side
            return facts
        p.body.data.uv_layers.remove(p.body.data.uv_layers[ATLAS_UV])
        side *= 2


def atlas_uv_at(p, A, side):
    """Add the atlas UV map (ATLAS_UV, exported as TEXCOORD_1) to a finished prop: each planar chart
    laid flat in its smallest rectangle's frame, rasterized at a trial texel density with margin_px
    round it and packed by its shape (_raster_pack), at the highest density up to max_px_per_m that
    fits the square. A chart nobody sees (faces down on the floor, or a wall prop's back on the wall: the
    deck compiler drops them) gets 2 px; a lamp's chart (light_panel faces) at least 5 px across. Returns the facts the manifest records: px_per_m, charts and
    the share of the atlas the charts' faces cover."""
    margin, top = int(A["atlas"]["margin_px"]), float(A["atlas"]["max_px_per_m"])
    ob = p.body
    me = ob.data
    bm = bmesh.new()
    bm.from_mesh(me)
    bm.normal_update()
    groups = []
    bm.faces.ensure_lookup_table()
    for faces in _plane_groups(bm, by_material=False):
        n = P(faces[0].normal).normalized()
        right, up, _ = face_axes(n)
        pts = {}
        for f in faces:
            for v in f.verts:
                q = P(v.co)
                pts[v.index] = (q.dot(right), q.dot(up))
        q = [P(v.co) for f in faces for v in f.verts]
        unseen = ((n.y < -0.999 and max(v.y for v in q) < 0.001)
                  or (p.wall and n.z < -0.999 and max(v.z for v in q) < 0.001))
        groups.append({"faces": [f.index for f in faces], "pos": pts, "tris": [[pts[v.index] for v in f.verts] for f in faces],
                       "fixed": unseen, "lamp": all(ROLES[f.material_index] == "light_panel" for f in faces)})
    for g in groups:
        g["fixed"] = True if g["fixed"] else None
    charts = []
    seen_area = sum(sum(bm.faces[fi].calc_area() for fi in g["faces"]) for g in groups if not g["fixed"])
    max_len = 0.9 * side / min(top, 0.75 * math.sqrt(side * side / max(seen_area, 1e-9)))
    for g in _unfold(groups, bm, max_len):
        c, s, x0, y0, w, h = _min_rect(list(g["pos"].values()))
        loc = {i: (a * c + b * s - x0, -a * s + b * c - y0) for i, (a, b) in g["pos"].items()}
        charts.append({"faces": g["faces"], "loc": loc,
                       "area": sum(bm.faces[fi].calc_area() for fi in g["faces"]),
                       "tris": [[loc[v.index] for v in bm.faces[fi].verts] for fi in g["faces"]],
                       "fixed": 2.0 / max(w, h, 1e-6) if g["fixed"] else None,
                       "short": min(w, h) if g["lamp"] else None, "long": max(w, h)})
    bm.free()
    area = sum(ch["area"] for ch in charts if ch["fixed"] is None)

    def scale(ch, ppm):
        """A chart's texel density: unseen faces 2 px; a lamp (its faces all light_panel) at least
        5 px across its short side, so a thin lamp strip still glows, but no longer than 0.45 of the
        square; the rest ppm."""
        if ch["fixed"] is not None:
            return ch["fixed"]
        if ch["short"] is not None:
            return max(ppm, min(8.0 * ppm, 5.0 / max(ch["short"], 1e-4), 0.45 * side / max(ch["long"], 1e-4)))
        return ppm

    def attempt(ppm):
        return _raster_pack([_chart_mask(ch["tris"], scale(ch, ppm), margin) for ch in charts], side)
    ppm = top
    placed = attempt(ppm)
    if placed is None:
        lo, hi = 0.0, min(top, math.sqrt(side * side / max(area, 1e-9)))
        got = None
        while got is None:                      # a density that fits, halving from the ideal
            hi *= 0.5
            got = attempt(hi)
            if hi < 0.5:
                raise SystemExit(f"[props] {ob.name}: {len(charts)} charts do not fit a {side} px atlas")
        lo, placed = hi, got
        hi = min(top, hi * 2.0)
        for _ in range(9):
            mid = (lo + hi) / 2
            got = attempt(mid)
            if got is None:
                hi = mid
            else:
                lo, placed = mid, got
        ppm = lo
    uv = me.uv_layers.new(name=ATLAS_UV)
    o = margin + 0.5
    for i, ch in enumerate(charts):
        bx, by, turned = placed[i]
        k = scale(ch, ppm)
        mh, mw = _chart_mask(ch["tris"], k, margin).shape
        for fi in ch["faces"]:
            poly = me.polygons[fi]
            for li in poly.loop_indices:
                a, b = ch["loc"][me.loops[li].vertex_index]
                u, v = a * k + o, b * k + o
                if turned:                      # the mask turned: column u became row (mw - u)
                    u, v = v, mw - u
                uv.data[li].uv = ((bx + u) / side, 1.0 - (by + v) / side)
    me.uv_layers.active_index = 0
    return {"px_per_m": round(ppm, 2), "charts": len(charts), "cover": round(area * ppm * ppm / (side * side), 3)}


# ----------------------------------------------------------------------------- the detail version
# A prop's atlas is baked from a detail version of it that exists only for the bake: a copy of its
# low-poly surfaces in their finishes plus details modelled on them, never exported. Details are
# small solids in a face's frame (x right, y up the face, z out of it, metres): plates, bumps with
# chamfered tops, discs, rings, letters, painted strips. A builder adds a prop's own (Prop.decor, run
# first: they reserve their footprint), then the rules dress what is left by role (Detail.generic).

class Region:
    """A planar piece of a prop's surface in one role: its frame (face_axes), its triangles and its
    boundary in 2D, so details can be placed inside it and kept clear of its edges and holes."""

    def __init__(self, faces, role):
        self.role = role
        n = P(faces[0].normal).normalized()
        self.right, self.up, self.n = face_axes(n)
        vs = {v.index: P(v.co) for f in faces for v in f.verts}
        self.origin = sum(vs.values(), Vector()) / len(vs)
        self.d = self.n.dot(self.origin)
        self.m = Matrix((tuple(self.right) + (0,), tuple(self.up) + (0,), tuple(self.n) + (0,), (0, 0, 0, 1))).transposed()
        self.m.translation = self.origin
        loc = {i: ((q - self.origin).dot(self.right), (q - self.origin).dot(self.up)) for i, q in vs.items()}
        self.tris = [[loc[v.index] for v in f.verts] for f in faces]
        fset = {f.index for f in faces}
        self.edges = []
        for f in faces:
            for e in f.edges:
                if sum(1 for g in e.link_faces if g.index in fset) == 1:
                    self.edges.append((loc[e.verts[0].index], loc[e.verts[1].index]))
        self.area = sum(abs((t[1][0] - t[0][0]) * (t[2][1] - t[0][1]) - (t[1][1] - t[0][1]) * (t[2][0] - t[0][0])) / 2 for t in self.tris)
        xs = [p[0] for p in loc.values()]
        ys = [p[1] for p in loc.values()]
        self.box = (min(xs), min(ys), max(xs), max(ys))

    def _in_tri(self, x, y):
        for a, b, c in self.tris:
            d1 = (b[0] - a[0]) * (y - a[1]) - (b[1] - a[1]) * (x - a[0])
            d2 = (c[0] - b[0]) * (y - b[1]) - (c[1] - b[1]) * (x - b[0])
            d3 = (a[0] - c[0]) * (y - c[1]) - (a[1] - c[1]) * (x - c[0])
            if (d1 >= -1e-9 and d2 >= -1e-9 and d3 >= -1e-9) or (d1 <= 1e-9 and d2 <= 1e-9 and d3 <= 1e-9):
                return True
        return False

    def clearance(self, x, y):
        best = 1e9
        for (ax, ay), (bx, by) in self.edges:
            dx, dy = bx - ax, by - ay
            t = max(0.0, min(1.0, ((x - ax) * dx + (y - ay) * dy) / (dx * dx + dy * dy or 1e-12)))
            best = min(best, math.hypot(x - ax - t * dx, y - ay - t * dy))
        return best

    def inside(self, x, y, margin=0.0):
        return self._in_tri(x, y) and self.clearance(x, y) >= margin

    def rect_ok(self, x0, y0, x1, y1, margin=0.0):
        pts = [(x, y) for x in (x0, (x0 + x1) / 2, x1) for y in (y0, (y0 + y1) / 2, y1)]
        return all(self.inside(x, y, margin) for x, y in pts)


def _clip(poly, a, b):
    """Keep the part of a 2D polygon left of the directed line a to b (Sutherland-Hodgman)."""
    out = []
    side = lambda p: (b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0])   # noqa: E731
    for i in range(len(poly)):
        p, q = poly[i], poly[(i + 1) % len(poly)]
        sp, sq = side(p), side(q)
        if sp >= 0:
            out.append(p)
        if (sp >= 0) != (sq >= 0):
            t = sp / (sp - sq)
            out.append((p[0] + t * (q[0] - p[0]), p[1] + t * (q[1] - p[1])))
    return out


def _ccw(t):
    a = (t[1][0] - t[0][0]) * (t[2][1] - t[0][1]) - (t[1][1] - t[0][1]) * (t[2][0] - t[0][0])
    return t if a > 0 else [t[0], t[2], t[1]]


class Detail:
    """The detail version of one prop, built for its bake: geometry gathered per finish (one object
    each), every face carrying the key light of the face it sits on (key_l, key_n)."""

    def __init__(self, prop, A):
        self.p = prop
        self.A = A
        self.geo = {}
        self.reserved = []
        self.bezelled = set()
        bm = bmesh.new()
        bm.from_mesh(prop.body.data)
        bm.normal_update()
        self.regions = [Region(fs, ROLES[fs[0].material_index]) for fs in _plane_groups(bm, by_material=True)]
        bm.free()

    def key(self, right, up, n):
        k = self.A["bake"]["key_tangent"]
        return (right * k[0] + up * k[1] + n * k[2]).normalized()

    def add(self, finish, m, verts, faces, reserve=None):
        """A solid in frame m: verts in m's local space, faces as vertex index tuples. reserve
        (x0, y0, x1, y1) keeps the rules' details off that part of the face."""
        if finish.startswith("role_") and finish not in self.A["finishes"]:
            # a station's colour, from shipkit's PALETTE (the one table of named colours)
            self.A["finishes"][finish] = {"kind": "paint", "srgb": list(_palette(finish[5:]))}
        if finish not in self.A["finishes"]:
            raise SystemExit(f"[props] {self.p.name}: detail finish {finish!r} is not in {ATLAS_JSON}")
        g = self.geo.setdefault(finish, {"verts": [], "faces": [], "keys": []})
        r3_ = m.to_3x3()
        right, up, n = (r3_ @ Vector((1, 0, 0))).normalized(), (r3_ @ Vector((0, 1, 0))).normalized(), (r3_ @ Vector((0, 0, 1))).normalized()
        L = self.key(right, up, n)
        base = len(g["verts"])
        g["verts"] += [m @ Vector(v) for v in verts]
        for f in faces:
            g["faces"].append(tuple(base + i for i in f))
            g["keys"].append((L, n))
        if reserve:
            self.reserve(m, *reserve)

    def reserve(self, m, x0, y0, x1, y1):
        n = (m.to_3x3() @ Vector((0, 0, 1))).normalized()
        self.reserved.append((n, n.dot(m.translation), [m @ Vector((x, y, 0.0)) for x in (x0, x1) for y in (y0, y1)]))

    def taken(self, R):
        """The reserved rectangles on region R's plane, in R's own 2D frame."""
        out = []
        for n, d, corners in self.reserved:
            if n.dot(R.n) < 0.999 or abs(d - R.d) > 0.01:
                continue
            xs = [(c - R.origin).dot(R.right) for c in corners]
            ys = [(c - R.origin).dot(R.up) for c in corners]
            out.append((min(xs), min(ys), max(xs), max(ys)))
        return out

    def free(self, R, x0, y0, x1, y1):
        """Whether a rectangle of region R (its own 2D frame) overlaps nothing reserved on its plane."""
        return not any(a < x1 and c > x0 and b < y1 and d > y0 for a, b, c, d in self.taken(R))

    # ---- solids in a frame (local x, y on the face, z out of it)
    def prism(self, m, pts, z0, z1, finish, inset=0.0, reserve=True):
        """A polygon (counter-clockwise, local x, y) from z0 to z1, its top shrunk by inset metres toward
        its centre (a chamfer that catches the key light)."""
        n = len(pts)
        cx, cy = sum(p[0] for p in pts) / n, sum(p[1] for p in pts) / n
        rad = max(math.hypot(p[0] - cx, p[1] - cy) for p in pts) or 1.0
        k = max(0.05, 1.0 - inset / rad)
        verts = [(x, y, z0) for x, y in pts] + [(cx + (x - cx) * k, cy + (y - cy) * k, z1) for x, y in pts]
        faces = [tuple(range(n - 1, -1, -1)), tuple(range(n, 2 * n))]
        faces += [(i, (i + 1) % n, n + (i + 1) % n, n + i) for i in range(n)]
        xs, ys = [p[0] for p in pts], [p[1] for p in pts]
        self.add(finish, m, verts, faces, (min(xs) - 0.01, min(ys) - 0.01, max(xs) + 0.01, max(ys) + 0.01) if reserve else None)

    def box(self, m, x0, y0, x1, y1, z0, z1, finish, inset=0.0, reserve=True):
        """A box from (x0, y0, z0) to (x1, y1, z1) of frame m, its top shrunk by inset on every side."""
        i = min(inset, (x1 - x0) * 0.45, (y1 - y0) * 0.45)
        verts = [(x0, y0, z0), (x1, y0, z0), (x1, y1, z0), (x0, y1, z0),
                 (x0 + i, y0 + i, z1), (x1 - i, y0 + i, z1), (x1 - i, y1 - i, z1), (x0 + i, y1 - i, z1)]
        faces = [(3, 2, 1, 0), (4, 5, 6, 7), (0, 1, 5, 4), (1, 2, 6, 5), (2, 3, 7, 6), (3, 0, 4, 7)]
        self.add(finish, m, verts, faces, (x0 - 0.01, y0 - 0.01, x1 + 0.01, y1 + 0.01) if reserve else None)

    def disc(self, m, x, y, r, z0, z1, finish, sides=12, inset=0.0, reserve=True):
        pts = [(x + r * math.cos(2 * math.pi * k / sides), y + r * math.sin(2 * math.pi * k / sides)) for k in range(sides)]
        self.prism(m, pts, z0, z1, finish, inset=inset, reserve=reserve)

    def ring(self, m, x, y, r0, r1, z0, z1, finish, sides=20):
        verts, faces = [], []
        for k in range(sides):
            a = 2 * math.pi * k / sides
            c, s = math.cos(a), math.sin(a)
            verts += [(x + r0 * c, y + r0 * s, z0), (x + r1 * c, y + r1 * s, z0), (x + r1 * c, y + r1 * s, z1), (x + r0 * c, y + r0 * s, z1)]
        for k in range(sides):
            a, b = 4 * k, 4 * ((k + 1) % sides)
            faces += [(a + 1, b + 1, b + 2, a + 2), (a + 3, a + 2, b + 2, b + 3), (a, a + 3, b + 3, b), (a, b, b + 1, a + 1)]
        self.add(finish, m, verts, faces, (x - r1 - 0.01, y - r1 - 0.01, x + r1 + 0.01, y + r1 + 0.01))

    def paint(self, m, pts, finish, z=0.0012, reserve=True):
        """A painted polygon: a solid 1.2 mm thick on the face (a seam, a stripe, a placard's field)."""
        self.prism(m, pts, -0.002, z, finish, reserve=reserve)

    def text(self, m, body, x, y, size, finish, align="CENTER", z=0.0016):
        """Letters size metres tall (Blender's built-in font), centred on (x, y) of frame m, raised z."""
        cu = bpy.data.curves.new("detail_text", "FONT")
        cu.body = body
        cu.size = size / 0.72          # the built-in font's capitals are about 0.72 of its size
        cu.align_x = align
        cu.align_y = "CENTER"
        cu.extrude = z / 2
        ob = bpy.data.objects.new("detail_text", cu)
        bpy.context.scene.collection.objects.link(ob)
        deps = bpy.context.evaluated_depsgraph_get()
        me = bpy.data.meshes.new_from_object(ob.evaluated_get(deps), depsgraph=deps)
        verts = [(v.co.x + x, v.co.y + y, v.co.z + z / 2) for v in me.vertices]
        faces = [tuple(p.vertices) for p in me.polygons]
        bpy.data.objects.remove(ob)
        bpy.data.curves.remove(cu)
        bpy.data.meshes.remove(me)
        if verts:
            xs, ys = [v[0] for v in verts], [v[1] for v in verts]
            self.add(finish, m, verts, faces, (min(xs) - 0.01, min(ys) - 0.01, max(xs) + 0.01, max(ys) + 0.01))

    # ---- composite details
    def plate(self, m, x, y, w, h, label, finish="plate", ink="stencil_dark", size=None):
        """A name plate w x h centred on (x, y): a bevelled plate, four rivets and a stencilled label."""
        self.box(m, x - w / 2, y - h / 2, x + w / 2, y + h / 2, -0.002, 0.004, finish, inset=0.003)
        for sx in (-1, 1):
            for sy in (-1, 1):
                self.disc(m, x + sx * (w / 2 - 0.012), y + sy * (h / 2 - 0.012), 0.005, 0.0, 0.007, "bolt", sides=8, inset=0.002, reserve=False)
        if label:
            self.text(m, label, x, y, size or min(h * 0.55, 0.07), ink, z=0.0052)

    def grille(self, m, x0, y0, x1, y1, finish="dark", slat="steel", pitch=0.03, along="x"):
        """A vent: a dark well and slats across it (along x: horizontal slats), a frame of finish slat."""
        self.box(m, x0, y0, x1, y1, -0.002, 0.002, finish)
        w = 0.008
        self.box(m, x0 - w, y0 - w, x1 + w, y0, -0.002, 0.006, slat, inset=0.002, reserve=False)
        self.box(m, x0 - w, y1, x1 + w, y1 + w, -0.002, 0.006, slat, inset=0.002, reserve=False)
        self.box(m, x0 - w, y0, x0, y1, -0.002, 0.006, slat, inset=0.002, reserve=False)
        self.box(m, x1, y0, x1 + w, y1, -0.002, 0.006, slat, inset=0.002, reserve=False)
        if along == "x":
            n = max(1, int((y1 - y0) / pitch))
            for k in range(n):
                yc = y0 + (y1 - y0) * (k + 0.5) / n
                self.box(m, x0, yc - pitch * 0.22, x1, yc + pitch * 0.22, 0.0, 0.006, slat, inset=0.0025, reserve=False)
        else:
            n = max(1, int((x1 - x0) / pitch))
            for k in range(n):
                xc = x0 + (x1 - x0) * (k + 0.5) / n
                self.box(m, xc - pitch * 0.22, y0, xc + pitch * 0.22, y1, 0.0, 0.006, slat, inset=0.0025, reserve=False)

    def buttons(self, m, x0, y, n, pitch, size, finishes, z=0.007):
        """A row of n square keys size wide from x0 at height y, finishes cycling (a glow finish lights one)."""
        for k in range(n):
            x = x0 + k * pitch
            self.box(m, x, y - size / 2, x + size, y + size / 2, -0.002, z, finishes[k % len(finishes)], inset=0.0025)

    def leds(self, m, x0, y, n, pitch, finishes, r=0.006):
        for k in range(n):
            self.disc(m, x0 + k * pitch, y, r, -0.002, 0.004, finishes[k % len(finishes)], sides=10)

    def bezel(self, rec, finish="dark", w=0.014, leds=None):
        """A dark frame round a recorded screen recess (on the panel's plane, never over the screen),
        with optional LEDs along its foot."""
        label, m, rw, rh = rec[0], rec[1], rec[2], rec[3]
        self.bezelled.add(label)
        x0, y0, x1, y1 = -rw / 2, -rh / 2, rw / 2, rh / 2
        for a in ((x0 - w, y0 - w, x1 + w, y0), (x0 - w, y1, x1 + w, y1 + w), (x0 - w, y0, x0, y1), (x1, y0, x1 + w, y1)):
            self.box(m, *a, -0.002, 0.003, finish, reserve=False)
        self.reserve(m, x0 - w - 0.02, y0 - w - 0.02, x1 + w + 0.02, y1 + w + 0.02)
        if leds:
            self.leds(m, x0 + 0.02, y0 - w - 0.014, len(leds), 0.026, leds, r=0.0055)

    # ---- details placed by where they fit: for a prop whose faces its builder has no frames for
    def spot(self, w, h, toward, margin=0.03, rank="centre", facing=0.9):
        """A free w x h place on the surface looking toward a prop-space direction (or the first of a
        list that has one): the biggest painted face within acos(facing) of it, at the free spot
        nearest its middle (rank 'high': its top, 'low': its foot). Returns (frame, x, y) or None."""
        for d in ([toward] if isinstance(toward[0], (int, float)) else toward):
            d = Vector(d).normalized()
            cands = [(-R.area, i, R) for i, R in enumerate(self.regions) if R.n.dot(d) > facing
                     and self.A["finishes"][self.finish_of(R.role)]["kind"] in ("paint", "metal", "brushed", "wood", "laminate", "fabric")]
            for _, _, R in sorted(cands, key=lambda c: (c[0], c[1])):
                x0, y0, x1, y1 = R.box
                if x1 - x0 < w + 2 * margin or y1 - y0 < h + 2 * margin:
                    continue
                step = max(0.01, (max(x1 - x0, y1 - y0)) / 40)
                cx = (x0 + x1) / 2
                cy = {"centre": (y0 + y1) / 2, "high": y1, "low": y0}[rank]
                pts = []
                nx, ny = int((x1 - x0 - w - 2 * margin) / step) + 1, int((y1 - y0 - h - 2 * margin) / step) + 1
                for i in range(nx):
                    for j in range(ny):
                        x, y = x0 + margin + w / 2 + i * step, y0 + margin + h / 2 + j * step
                        pts.append((abs(x - cx) + abs(y - cy), x, y))
                for _, x, y in sorted(pts):
                    if R.rect_ok(x - w / 2 - margin, y - h / 2 - margin, x + w / 2 + margin, y + h / 2 + margin) and \
                            self.free(R, x - w / 2, y - h / 2, x + w / 2, y + h / 2):
                        return R.m, x, y
        return None

    def apply(self, items):
        """Place a table of details: (kind, args...) with kind stencil, placard, gauge or lamps, in
        order, each where it fits; one that fits nowhere is reported, not fatal."""
        for it in items:
            if not getattr(self, it[0])(*it[1:]):
                print(f"[props] {self.p.name}: no room for {it[0]} {it[1]!r}")

    def turned_spot(self, w, h, toward, rank, facing):
        """spot(w, h), or failing that a spot h x w with a frame turned a quarter so w runs up the face
        (a name down a column's narrow facet). Returns (frame, x, y) in that frame or None."""
        at = self.spot(w, h, toward, rank=rank, facing=facing)
        if at or w <= h:
            return at
        at = self.spot(h, w, toward, rank=rank, facing=facing)
        if not at:
            return None
        m, x, y = at
        return m @ Matrix.Translation((x, y, 0.0)) @ Matrix.Rotation(math.radians(90.0), 4, "Z"), 0.0, 0.0

    def stencil(self, text, size, toward, ink="stencil", rank="centre", facing=0.9):
        """Stencilled letters size metres tall where they fit on a face looking toward (running up a
        narrow face when they do not fit across it)."""
        at = self.turned_spot(0.64 * size * len(text) + 0.02, size * 1.3, toward, rank, facing)
        if at:
            m, x, y = at
            self.text(m, text, x, y, size, ink)
        return at

    def placard(self, text, w, h, toward, finish="yellow", ink="stencil_dark", rank="centre", facing=0.9):
        """A warning placard w x h (a bevelled plate in finish, its text in ink) where it fits."""
        at = self.turned_spot(w, h, toward, rank, facing)
        if at:
            m, x, y = at
            self.box(m, x - w / 2, y - h / 2, x + w / 2, y + h / 2, -0.002, 0.004, finish, inset=0.003)
            if text:
                lines = text.split("|")
                sz = min(h * 0.6 / len(lines), 0.85 * w / (0.62 * max(len(t) for t in lines)))
                for k, t in enumerate(lines):
                    self.text(m, t, x, y + (len(lines) - 1 - 2 * k) * sz * 0.65, sz, ink, z=0.0056)
        return at

    def gauge(self, r, toward, rank="centre", facing=0.9, lit=False):
        """A round gauge r metres across: a steel bezel, a white face, a needle and a red arc."""
        at = self.spot(2 * r + 0.02, 2 * r + 0.02, toward, rank=rank, facing=facing)
        if at:
            m, x, y = at
            self.ring(m, x, y, r * 0.86, r, -0.002, 0.012, "bolt")
            self.disc(m, x, y, r * 0.88, -0.002, 0.005, "led_white" if lit else "white", sides=20)
            a = math.radians(35.0)
            self.prism(m, [(x - 0.12 * r * math.sin(a) - 0.05 * r * math.cos(a), y - 0.05 * r * math.sin(a) + 0.12 * r * math.cos(a)),
                           (x + 0.75 * r * math.sin(a), y - 0.75 * r * math.cos(a) + 0.0),
                           (x + 0.05 * r * math.cos(a), y + 0.05 * r * math.sin(a))][::-1], 0.004, 0.008, "stencil_dark", reserve=False)
            self.paint(m, [(x + 0.45 * r, y - 0.55 * r), (x + 0.7 * r, y - 0.2 * r), (x + 0.6 * r, y - 0.12 * r), (x + 0.38 * r, y - 0.45 * r)],
                       "red", z=0.0062, reserve=False)
        return at

    def lamps(self, n, toward, finishes, r=0.012, pitch=0.04, rank="high", facing=0.9):
        """A row of n lamps where it fits on a face looking toward."""
        at = self.spot(pitch * n + 0.02, 2 * r + 0.02, toward, rank=rank, facing=facing)
        if at:
            m, x, y = at
            self.leds(m, x - pitch * (n - 1) / 2, y, n, pitch, finishes, r=r)
        return at

    # ---- the rules, by role, for what the prop's own details left
    def generic(self):
        """Seams and bolts on painted faces large enough to be panelled, a vent on a big blank housing
        face, rivets along a steel frame's long faces; screens, glass, lamps, upholstery and small
        faces stay plain. Nothing lands where a prop's own details reserved."""
        finish_of = self.finish_of
        for i, R in enumerate(self.regions):
            fin = finish_of(R.role)
            kind = self.A["finishes"][fin]["kind"]
            x0, y0, x1, y1 = R.box
            w, h = x1 - x0, y1 - y0
            if kind == "paint" and R.area >= 0.12 and max(w, h) >= 0.5 and min(w, h) >= 0.2:
                self._panel(R, i)
            elif kind in ("brushed", "metal") and max(w, h) >= 0.3 and min(w, h) >= 0.045:
                self._rivets(R)

    def _panel(self, R, i):
        x0, y0, x1, y1 = R.box
        w, h = x1 - x0, y1 - y0
        nx, ny = max(1, round(w / 0.55)), max(1, round(h / 0.55))
        xs = [x0 + w * k / nx for k in range(nx + 1)]
        ys = [y0 + h * k / ny for k in range(ny + 1)]
        sw = 0.006
        taken = self.taken(R)

        def gaps(lo, hi, cuts):
            """[lo, hi] less the intervals cut (a seam stops 1 cm short of a reserved detail)."""
            out, at = [], lo
            for a, b in sorted(cuts):
                if b < at or a > hi:
                    continue
                if a - 0.01 > at:
                    out.append((at, a - 0.01))
                at = max(at, b + 0.01)
            if at < hi:
                out.append((at, hi))
            return out
        for x in xs[1:-1]:
            for a, b in gaps(y0 - 1, y1 + 1, [(t[1], t[3]) for t in taken if t[0] < x + sw and t[2] > x - sw]):
                self._seam(R, [(x - sw, a), (x + sw, a), (x + sw, b), (x - sw, b)])
        for y in ys[1:-1]:
            for a, b in gaps(x0 - 1, x1 + 1, [(t[0], t[2]) for t in taken if t[1] < y + sw and t[3] > y - sw]):
                self._seam(R, [(a, y - sw), (b, y - sw), (b, y + sw), (a, y + sw)])
        for a in range(nx):
            for b in range(ny):
                for x in (xs[a] + 0.035, xs[a + 1] - 0.035):
                    for y in (ys[b] + 0.035, ys[b + 1] - 0.035):
                        if R.inside(x, y, 0.025) and self.free(R, x - 0.012, y - 0.012, x + 0.012, y + 0.012):
                            self.disc(R.m, x, y, 0.009, -0.002, 0.006, "bolt", sides=10, inset=0.003)
        # a vent on a big blank housing face, low and centred in its widest panel, when nothing is there
        if self.finish_of(R.role) == "housing" and R.area >= 0.3 and (i + len(self.p.name)) % 2 == 0:
            gw, gh = min(0.42, w * 0.5), 0.12
            cx, cy = x0 + w / 2, y0 + min(h * 0.3, 0.3)
            if R.rect_ok(cx - gw / 2, cy - gh / 2, cx + gw / 2, cy + gh / 2, 0.04) and self.free(R, cx - gw / 2, cy - gh / 2, cx + gw / 2, cy + gh / 2):
                self.grille(R.m, cx - gw / 2, cy - gh / 2, cx + gw / 2, cy + gh / 2)

    def _seam(self, R, poly):
        """A painted seam: the strip clipped to the region's triangles."""
        for t in R.tris:
            piece = poly
            t = _ccw(t)
            for k in range(3):
                piece = _clip(piece, t[k], t[(k + 1) % 3])
                if len(piece) < 3:
                    break
            if len(piece) >= 3:
                self.paint(R.m, piece, "seam", reserve=False)

    def _rivets(self, R):
        x0, y0, x1, y1 = R.box
        along_x = (x1 - x0) >= (y1 - y0)
        L = (x1 - x0) if along_x else (y1 - y0)
        n = int(L / 0.12)
        for k in range(1, n):
            t = k / n
            for e in ((0.018,) if min(x1 - x0, y1 - y0) < 0.08 else (0.018, -0.018)):
                x = x0 + (x1 - x0) * t if along_x else (x0 + e if e > 0 else x1 + e)
                y = (y0 + e if e > 0 else y1 + e) if along_x else y0 + (y1 - y0) * t
                if R.inside(x, y, 0.012) and self.free(R, x - 0.008, y - 0.008, x + 0.008, y + 0.008):
                    self.disc(R.m, x, y, 0.0055, -0.002, 0.0045, "bolt", sides=8, inset=0.002, reserve=False)

    def finish_of(self, role):
        return self.p.finish_of.get(role, self.A["roles"][role])


# ----------------------------------------------------------------------------- the bake

def _bake_material(name, fin, A, W, tint=None):
    """A bake material: the finish's colour (by kind) under wear, lit by the face-relative key (the
    key_l and key_n attributes), as emission; a glow finish is its colour, unlit."""
    m = bpy.data.materials.new("bk." + name)
    N = Nodes(m)
    F = A["finishes"][fin]
    kind = F["kind"]
    m["glow"] = kind == "glow"
    if kind == "glow":
        emission(N, lin(F["srgb"]) + (1.0,))
        return m
    geo = N.new("ShaderNodeNewGeometry")
    pos = geo.outputs["Position"]
    if kind == "layer":
        uvn = N.new("ShaderNodeUVMap", uv_map="UVMap")
        sep = N.new("ShaderNodeSeparateXYZ")
        N.nt.links.new(uvn.outputs["UV"], sep.inputs[0])
        vec = N.xyz(N.math("MULTIPLY", sep.outputs[0], 0.5), N.math("MULTIPLY", N.math("SUBTRACT", 1.0, sep.outputs[1]), 0.5), 0.0)
        tex = N.new("ShaderNodeTexImage", interpolation="Linear", extension="REPEAT")
        tex.image = bpy.data.images.load(os.path.join(TEXTURES, "panels", "256", F["layer"] + ".png"), check_existing=True)
        tex.image.alpha_mode = "CHANNEL_PACKED"     # a panel layer's alpha is its glow mask: never premultiply by it
        N.nt.links.new(vec, tex.inputs["Vector"])
        base = N.mix(1.0, tex.outputs["Color"], lin(tint) + (1.0,), blend="MULTIPLY") if tint else tex.outputs["Color"]
    elif kind == "hazard":
        sep = N.new("ShaderNodeSeparateXYZ")
        N.nt.links.new(pos, sep.inputs[0])
        ph = N.math("FRACT", N.math("DIVIDE", N.math("ADD", N.math("ADD", sep.outputs[0], sep.outputs[1]), sep.outputs[2]), 0.125))
        base = N.mix(N.math("GREATER_THAN", ph, 0.5), lin(F["srgb"]) + (1.0,), lin(F["srgb_b"]) + (1.0,))
    else:
        c = lin(F["srgb"]) + (1.0,)
        lo = tuple(v * 0.86 for v in c[:3]) + (1.0,)
        hi = tuple(min(1.0, v * 1.08) for v in c[:3]) + (1.0,)
        if kind == "brushed":
            sep = N.new("ShaderNodeSeparateXYZ")
            N.nt.links.new(pos, sep.inputs[0])
            v = N.xyz(N.math("MULTIPLY", sep.outputs[0], 4.0), N.math("MULTIPLY", sep.outputs[1], 4.0), N.math("MULTIPLY", sep.outputs[2], 90.0))
            base = N.mix(N.ramp(N.noise(v, 1.0, 3.0, 0.5), 0.3, 0.7), lo, hi)
        elif kind == "wood":
            uvn = N.new("ShaderNodeUVMap", uv_map="UVMap")
            sep = N.new("ShaderNodeSeparateXYZ")
            N.nt.links.new(uvn.outputs["UV"], sep.inputs[0])
            v = N.xyz(N.math("MULTIPLY", sep.outputs[0], 1.5), N.math("MULTIPLY", sep.outputs[1], 28.0), 0.0)
            grain = N.ramp(N.noise(v, 1.0, 4.0, 0.6), 0.35, 0.65)
            base = N.mix(grain, tuple(x * 0.78 for x in c[:3]) + (1.0,), hi)
        elif kind == "fabric":
            base = N.mix(N.ramp(N.noise(pos, 260.0, 2.0, 0.5), 0.3, 0.7), lo, hi)
        elif kind == "laminate":
            base = N.mix(N.ramp(N.noise(pos, 180.0, 2.0, 0.5), 0.45, 0.55), tuple(x * 0.97 for x in c[:3]) + (1.0,), c)
        else:
            base = c
    Fw = {"wear": W, "colours_srgb": {"metal": A["edge_metal_srgb"], "rust": A["rust_srgb"]}}
    col = worn_blotchy(N, base, Fw, kind in WEARING, pos)
    al = N.new("ShaderNodeAttribute", attribute_name="key_l")
    an = N.new("ShaderNodeAttribute", attribute_name="key_n")
    dl = N.new("ShaderNodeVectorMath", operation="DOT_PRODUCT")
    N.nt.links.new(geo.outputs["Normal"], dl.inputs[0])
    N.nt.links.new(al.outputs["Vector"], dl.inputs[1])
    dn = N.new("ShaderNodeVectorMath", operation="DOT_PRODUCT")
    N.nt.links.new(an.outputs["Vector"], dn.inputs[0])
    N.nt.links.new(al.outputs["Vector"], dn.inputs[1])
    a = A["bake"]["ambient"]
    rel = N.math("MINIMUM", N.math("DIVIDE", N.math("MAXIMUM", dl.outputs["Value"], 0.0), N.math("MAXIMUM", dn.outputs["Value"], 0.25)),
                 A["bake"]["relief_max"])
    shade = N.math("ADD", a, N.math("MULTIPLY", rel, 1.0 - a))
    emission(N, N.mix(1.0, col, N.xyz(shade, shade, shade), blend="MULTIPLY"))
    return m


def _mask_material(glow):
    name = "bk.mask_white" if glow else "bk.mask_black"
    m = bpy.data.materials.get(name)
    if m is None:
        m = bpy.data.materials.new(name)
        emission(Nodes(m), (1.0, 1.0, 1.0, 1.0) if glow else (0.0, 0.0, 0.0, 1.0))
    return m


def _set_keys(me, keys):
    """Face attributes key_l and key_n (Blender space) from [(L, n)] in prop space, one per face."""
    for name, k in (("key_l", 0), ("key_n", 1)):
        at = me.attributes.new(name, "FLOAT_VECTOR", "FACE")
        flat = []
        for pair in keys:
            v = pair[k]
            flat += [v.x, -v.z, v.y]
        at.data.foreach_set("vector", flat)


def _bake_image(name, px):
    img = bpy.data.images.new(name, px, px, alpha=False, float_buffer=True)
    img.colorspace_settings.name = "Linear Rec.709"
    return img


def _pixels(img):
    import numpy as np
    w, h = img.size
    a = np.empty(w * h * 4, np.float32)
    img.pixels.foreach_get(a)
    return a.reshape(h, w, 4)[::-1, :, :3].copy()


def bake_atlas(p, A, wear, path, side):
    """Bake one prop's atlas into path (a side x side RGBA PNG): build its detail version (the
    builder's Prop.decor, then Detail.generic), bake colour and the emission mask onto the body's
    atlas UV, post-process as the panels are. Everything made here is removed afterwards. Returns
    the PNG's bytes facts."""
    import numpy as np
    sys.path.insert(0, os.path.join(ROOT, "tools", "materials"))
    import postprocess as mm
    from PIL import Image
    sc = bpy.context.scene
    hidden = {o.name: o.hide_render for o in sc.objects}
    for o in sc.objects:
        o.hide_render = True
    sc.render.engine = "CYCLES"
    cy = sc.cycles
    cy.device = "CPU"
    cy.use_adaptive_sampling = False
    cy.use_denoising = False
    cy.seed = 0
    cy.use_animated_seed = False
    cy.max_bounces = 0
    cy.diffuse_bounces = 0
    cy.glossy_bounces = 0
    cy.transmission_bounces = 0
    cy.volume_bounces = 0
    cy.transparent_max_bounces = 0
    bk = sc.render.bake
    bk.use_selected_to_active = True
    bk.use_cage = False
    bk.cage_extrusion = A["bake"]["cage_m"]
    bk.max_ray_distance = 0.0
    px = side * A["atlas"]["bake_scale"]
    bk.margin = 2 * int(A["atlas"]["margin_px"]) * A["atlas"]["bake_scale"]
    bk.margin_type = "EXTEND"
    bk.target = "IMAGE_TEXTURES"
    bk.use_clear = True
    coll = bpy.data.collections.new(f"{p.name}.bake")
    sc.collection.children.link(coll)
    made = []

    def mk(name, me):
        ob = bpy.data.objects.new(name, me)
        coll.objects.link(ob)
        made.append(ob)
        return ob
    tgt = mk(f"{p.name}.bake_target", p.body.data.copy())
    tgt.data.uv_layers.active = tgt.data.uv_layers[ATLAS_UV]
    for vis in ("visible_camera", "visible_diffuse", "visible_glossy", "visible_transmission", "visible_volume_scatter", "visible_shadow"):
        setattr(tgt, vis, False)
    img = _bake_image(f"{p.name}.atlas", px)
    tmat = bpy.data.materials.new("bk.target")
    tn = Nodes(tmat)
    node = tn.new("ShaderNodeTexImage")
    node.image = img
    tn.nt.nodes.active = node
    emission(tn, (0.0, 0.0, 0.0, 1.0))
    for i in range(len(tgt.data.materials)):     # (materials.clear() would reset every face's slot)
        tgt.data.materials[i] = tmat
    D = Detail(p, A)
    tint = A["tints"].get(p.name)
    mats = {}

    def mat(fin):
        if fin not in mats:
            mats[fin] = _bake_material(f"{p.name}.{fin}", fin, A, A["wear"][wear], tint)
        return mats[fin]
    base_me = p.body.data.copy()
    base = mk(f"{p.name}.bake_base", base_me)
    roles = list(ROLES)
    keys = [None] * len(base_me.polygons)
    bm = bmesh.new()
    bm.from_mesh(p.body.data)
    bm.normal_update()
    for fs in _plane_groups(bm, by_material=True):
        R = Region(fs, ROLES[fs[0].material_index])
        L = D.key(R.right, R.up, R.n)
        for f in fs:
            keys[f.index] = (L, R.n)
    bm.free()
    for i, r in enumerate(roles):
        base_me.materials[i] = mat(D.finish_of(r))
    _set_keys(base_me, keys)
    for fn in p.decor:
        fn(D)
    for rec in p.recesses:
        if rec[5] == "screen" and rec[0] not in D.bezelled:
            D.bezel(rec)
    D.generic()
    for fin in sorted(D.geo):
        g = D.geo[fin]
        bmd = bmesh.new()
        vs = [bmd.verts.new(PROP_TO_BLENDER @ v) for v in g["verts"]]
        for f in g["faces"]:
            bmd.faces.new([vs[i] for i in f])
        bmesh.ops.recalc_face_normals(bmd, faces=bmd.faces[:])
        me = bpy.data.meshes.new(f"{p.name}.detail.{fin}")
        bmd.to_mesh(me)
        bmd.free()
        me.materials.append(mat(fin))
        _set_keys(me, g["keys"])
        mk(f"{p.name}.detail.{fin}", me)
    srcs = [o for o in made if o is not tgt]
    for o in sc.objects:
        o.select_set(False)
    for o in srcs:
        o.select_set(True)
    tgt.select_set(True)
    bpy.context.view_layer.objects.active = tgt
    cy.samples = A["bake"]["samples"]
    bpy.ops.object.bake(type="EMIT")
    colour = _pixels(img)
    for o in srcs:
        for i, m_ in enumerate(list(o.data.materials)):
            o.data.materials[i] = _mask_material(bool(m_.get("glow")))
    cy.samples = A["bake"]["mask_samples"]
    bpy.ops.object.bake(type="EMIT")
    mask = _pixels(img)
    f = A["atlas"]["bake_scale"]

    def avg(a):
        h, w = a.shape[:2]
        return a.reshape(h // f, f, w // f, f, a.shape[2]).mean(axis=(1, 3))
    x = np.clip(avg(colour), 0.0, 1.0)
    srgb = np.where(x <= 0.0031308, x * 12.92, 1.055 * np.power(x, 1.0 / 2.4) - 0.055)
    rgb = mm.reduce_palette(mm.to_u8(srgb), A["atlas"]["colours"], method="max_coverage")
    alpha = mm.to_u8(avg(mask).max(axis=2))
    os.makedirs(os.path.dirname(path), exist_ok=True)
    mm.save_png(Image.fromarray(np.dstack([rgb, alpha]), "RGBA"), path)
    glow = float(np.count_nonzero(alpha >= 128)) / alpha.size
    # remove everything the bake made, and give the scene back its render visibility
    for o in made:
        me = o.data
        bpy.data.objects.remove(o)
        if me.users == 0:
            bpy.data.meshes.remove(me)
    bpy.data.collections.remove(coll)
    bpy.data.images.remove(img)
    for m_ in list(mats.values()) + [tmat]:
        bpy.data.materials.remove(m_)
    for o in sc.objects:
        if o.name in hidden:
            o.hide_render = hidden[o.name]
    return {"glow": round(glow, 4)}


# ----------------------------------------------------------------------------- glb

GLB_OPTIONS = dict(
    export_format="GLB", use_selection=True, export_apply=False, export_yup=True,
    export_texcoords=True, export_normals=True, export_tangents=False,
    export_materials="EXPORT", export_image_format="NONE", export_vertex_color="NONE",
    export_attributes=False, export_extras=False, export_cameras=False, export_lights=False,
    export_animations=False, export_skins=False, export_morph=False, use_mesh_edges=False,
    use_mesh_vertices=False, export_shared_accessors=False, export_gpu_instances=False,
    export_copyright="",
)


def export_glb(ob, path):
    for o in bpy.context.view_layer.objects:
        o.select_set(False)
    ob.select_set(True)
    bpy.context.view_layer.objects.active = ob
    bpy.ops.export_scene.gltf(filepath=path, **GLB_OPTIONS)


def read_glb(path):
    """The exported artifact, read back: {material: {'pos': [...], 'nrm': [...], 'uv': [...],
    'idx': [...]}} plus the JSON. Pure Python, so the checks judge the file, not the scene."""
    data = open(path, "rb").read()
    magic, _version, _length = struct.unpack_from("<III", data, 0)
    if magic != 0x46546C67:
        raise SystemExit(f"[props] {path}: not a glb")
    jlen, _ = struct.unpack_from("<II", data, 12)
    js = json.loads(data[20:20 + jlen])
    blen, _ = struct.unpack_from("<II", data, 20 + jlen)
    binary = data[28 + jlen:28 + jlen + blen]
    comps = {"SCALAR": 1, "VEC2": 2, "VEC3": 3}
    fmt = {5126: "f", 5123: "H", 5125: "I", 5121: "B"}

    def acc(i):
        a = js["accessors"][i]
        bv = js["bufferViews"][a["bufferView"]]
        n = a["count"] * comps[a["type"]]
        off = bv.get("byteOffset", 0) + a.get("byteOffset", 0)
        vals = struct.unpack_from("<%d%s" % (n, fmt[a["componentType"]]), binary, off)
        k = comps[a["type"]]
        return [vals[i:i + k] for i in range(0, n, k)] if k > 1 else list(vals)
    prims = {}
    for mesh in js["meshes"]:
        for p in mesh["primitives"]:
            name = js["materials"][p["material"]]["name"]
            prims[name] = {"pos": acc(p["attributes"]["POSITION"]), "nrm": acc(p["attributes"]["NORMAL"]),
                           "uv": acc(p["attributes"]["TEXCOORD_0"]), "idx": acc(p["indices"]),
                           "uv1": acc(p["attributes"]["TEXCOORD_1"]) if "TEXCOORD_1" in p["attributes"] else None}
    return js, prims


def verify_glb(path, prop):
    """Check the file: +Y up with the floor at y = 0, the back at z = 0 (a wall-standing prop,
    or one whose back_at_z0 says so), flat normals, UV0 equal to worldUv in metres. Returns the
    manifest's measured facts."""
    js, prims = read_glb(path)
    pts = [p for m in prims.values() for p in m["pos"]]
    lo = [min(p[i] for p in pts) for i in range(3)]
    hi = [max(p[i] for p in pts) for i in range(3)]
    problems = []
    if abs(lo[1]) > 1e-4:
        problems.append(f"lowest point y = {lo[1]:.4f} m, not on the floor (axes?)")
    if prop.wall and abs(lo[2]) > 1e-4:
        problems.append(f"backmost point z = {lo[2]:.4f} m, not on the wall plane")
    if prop.back_at_z0 and abs(lo[2]) > 1e-4:
        problems.append(f"backmost point z = {lo[2]:.4f} m, not at the anchor's z = 0")
    tris = 0
    worst_uv = 0.0
    for name, m in prims.items():
        idx = m["idx"]
        tris += len(idx) // 3
        for t in range(0, len(idx), 3):
            a, b, c = (Vector(m["pos"][k]) for k in idx[t:t + 3])
            fn = (b - a).cross(c - a).normalized()
            for k in idx[t:t + 3]:
                if Vector(m["nrm"][k]).dot(fn) < 0.999:
                    problems.append(f"{name}: a vertex normal is not its face's (not flat shaded)")
                    break
                u, v = world_uv(Vector(m["pos"][k]), fn)
                worst_uv = max(worst_uv, abs(m["uv"][k][0] - u), abs(m["uv"][k][1] - v))
    if worst_uv > 1e-4:
        problems.append(f"UV0 differs from worldUv by up to {worst_uv:.5f} m")
    # the atlas: TEXCOORD_1 on every primitive, inside the square
    for name, m in prims.items():
        if m["uv1"] is None:
            problems.append(f"{name}: no TEXCOORD_1 (the atlas)")
        elif any(not (0.0 <= c <= 1.0) for uv in m["uv1"] for c in uv):
            problems.append(f"{name}: an atlas coordinate outside 0-1")
    # Screens face the operator (+Z) or up: a flipped axis on export would turn them away. And
    # every screen the manifest records lies on a screen face of the file, facing its way.
    planes = []
    m = prims.get("screen", {"idx": [], "pos": []})
    for t in range(0, len(m["idx"]), 3):
        a, b, c = (Vector(m["pos"][k]) for k in m["idx"][t:t + 3])
        fn = (b - a).cross(c - a).normalized()
        planes.append((fn, fn.dot(a)))
        if fn.z < -0.01:
            problems.append(f"a screen faces away from the operator (normal {r3(fn)})")
    for sc in prop.screens:
        c, n = Vector(sc["centre_m"]), Vector(sc["normal"])
        if not any(fn.dot(n) > 0.999 and abs(fn.dot(c) - d) < 1e-3 for fn, d in planes):
            problems.append(f"screen {sc['label']} is not on a screen face of the file")
    if problems:
        raise SystemExit(f"[props] {path}: " + "; ".join(sorted(set(problems))))
    return {"triangles": tris, "bounds_m": {"min": r3(lo), "max": r3(hi)},
            "dimensions_m": r3([hi[i] - lo[i] for i in range(3)]),
            "materials": [m["name"] for m in js["materials"]], "uv_error_m": worst_uv}


# ----------------------------------------------------------------------------- the build

class PropSet:
    """What a builder hands run(): one set of props and where they go.

    name     short id: the scene is named after it and so is the build's temporary folder
    scene    the Blender scene's name (it is written into every glb, so it is part of the bytes)
    out      the folder for <name>.glb and props.json (absolute)
    generator  the builder's path from the repository root, as the manifest records it
    props    {prop name: a function returning a finished Prop}, in manifest order
    budgets  {prop name: triangles}
    status   the manifest's status line; rules its _rules list
    doc      the builder's docstring, printed with a command-line error"""

    def __init__(self, name, scene, out, generator, props, budgets, status, rules, doc):
        self.name, self.scene, self.out, self.generator = name, scene, out, generator
        self.props, self.budgets, self.status, self.rules, self.doc = props, budgets, status, rules, doc
        self.manifest = os.path.join(out, "props.json")
        missing = [n for n in props if n not in budgets]
        if missing:
            raise SystemExit(f"[props] no triangle budget for {', '.join(missing)}")


def parse_args(props, doc):
    if "--" in sys.argv:
        args = sys.argv[sys.argv.index("--") + 1:]
    elif os.path.basename(sys.argv[0]).startswith("blender"):
        args = []
    else:
        args = sys.argv[1:]
    opts = {"check": False, "only": None, "blend": None}
    i = 0
    while i < len(args):
        a = args[i]
        if a == "--check":
            opts["check"] = True
        elif a == "--only":
            i += 1
            opts["only"] = args[i].split(",")
        elif a == "--blend":
            i += 1
            opts["blend"] = os.path.abspath(args[i])
        else:
            raise SystemExit(f"[props] unknown argument {a!r}\n{doc}")
        i += 1
    for n in opts["only"] or []:
        if n not in props:
            raise SystemExit(f"[props] no prop {n!r}; the props are {', '.join(props)}")
    return opts


def sha256(path):
    return hashlib.sha256(open(path, "rb").read()).hexdigest()


def gltf_exporter_version():
    import addon_utils
    for m in addon_utils.modules():
        if m.__name__ == "io_scene_gltf2":
            return ".".join(str(v) for v in m.bl_info["version"])
    return "unknown"


def manifest_text(ps, rows):
    doc = {
        "schema": "starcrew.props/1",
        "status": ps.status,
        "_rules": ps.rules,
        "generator": {"script": ps.generator, "blender": bpy.app.version_string, "gltf_exporter": gltf_exporter_version()},
        "props": rows,
    }
    return json.dumps(doc, indent=2, ensure_ascii=True) + "\n"


def manifest_row(n, p, facts, budget, digest):
    """One prop's manifest row, its keys in the order every set's manifest uses."""
    row = {
        "file": n + ".glb",
        "presents": p.presents,
    }
    if p.variant_of:
        row["variant_of"] = p.variant_of
        row["stations"] = p.stations
    if p.controls:
        row["controls"] = p.controls
    row.update({
        "anchor": p.anchor,
        "dimensions_m": facts["dimensions_m"],
        "bounds_m": facts["bounds_m"],
    })
    if p.seat:
        row["seat_m"] = p.seat
    row["operators_m"] = p.operators
    if p.operators_yaw is not None:
        if len(p.operators_yaw) != len(p.operators):
            raise SystemExit(f"[props] {n}: {len(p.operators_yaw)} operator yaws for {len(p.operators)} operators")
        row["operators_yaw_deg"] = p.operators_yaw
    row["screens"] = p.screens
    row["triangles"] = facts["triangles"]
    row["budget_triangles"] = budget
    row["materials"] = facts["materials"]
    row["csg"] = p.steps
    row["sha256"] = digest
    if getattr(p, "atlas", None):
        row["atlas"] = p.atlas
    return row


def run(ps):
    """The build: make the materials, build every prop (or --only these), clean, check against
    the budget, finish, lay out its atlas UV; refuse the whole set on any problem; export, read back
    and bake each prop's atlas; write the glbs, the atlases and the manifest (or, with --check,
    compare them with the committed files)."""
    opts = parse_args(ps.props, ps.doc)
    A = load_atlas()
    if ps.name not in A["sets"]:
        raise SystemExit(f"[props] {ATLAS_JSON}: sets names no wear for the {ps.name} set")
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.context.scene.name = ps.scene
    bpy.context.scene.unit_settings.system = "METRIC"
    make_materials()
    names = opts["only"] or list(ps.props)
    built, failures = {}, []
    for n in names:
        p = ps.props[n]()
        p.body.name = n
        p.body.data.name = n
        clean(p.body)
        tris, problems = check(p)
        if tris > ps.budgets[n]:
            problems.append(f"{tris} triangles, over its budget of {ps.budgets[n]}")
        finish(p.body)
        p.atlas_uv = atlas_uv(p, A)
        print(f"[props] {n}: {tris} triangles (budget {ps.budgets[n]}), {len(p.screens)} screens, atlas {p.atlas_uv['px']} px, "
              f"{p.atlas_uv['px_per_m']} px/m in {p.atlas_uv['charts']} charts; " + "; ".join(p.steps))
        if problems:
            failures.append(f"{n}: " + "; ".join(sorted(set(problems))))
        built[n] = p
    if opts["blend"]:
        bpy.ops.wm.save_as_mainfile(filepath=opts["blend"], compress=True)
        print("[props] saved", opts["blend"])
    if failures:
        raise SystemExit("[props] refused, nothing written:\n  " + "\n  ".join(failures))

    tmp = tempfile.mkdtemp(prefix=f"{ps.name}_props_")
    try:
        rows = {}
        old = {}
        if os.path.exists(ps.manifest):
            old = json.load(open(ps.manifest, encoding="utf-8")).get("props", {})
        for n in ps.props:
            if n not in built:
                if n in old:
                    rows[n] = old[n]
                continue
            p = built[n]
            path = os.path.join(tmp, n + ".glb")
            export_glb(p.body, path)
            facts = verify_glb(path, p)
            rel = A["atlas"]["dir"] + "/" + n + ".png"
            png = os.path.join(tmp, rel)
            baked = bake_atlas(p, A, A["sets"][ps.name], png, p.atlas_uv["px"])
            p.atlas = {"file": rel, "px": p.atlas_uv["px"], "px_per_m": p.atlas_uv["px_per_m"], "charts": p.atlas_uv["charts"],
                       "cover": p.atlas_uv["cover"], "glow": baked["glow"], "sha256": sha256(png), "bytes": os.path.getsize(png)}
            print(f"[props] {n}: atlas baked, {p.atlas['px_per_m']} px/m, glows on {baked['glow']:.1%}", flush=True)
            rows[n] = manifest_row(n, p, facts, ps.budgets[n], sha256(path))
        text = manifest_text(ps, rows)
        if opts["check"]:
            stale = []
            for n in built:
                dst = os.path.join(ps.out, n + ".glb")
                if not os.path.exists(dst) or sha256(dst) != rows[n]["sha256"]:
                    stale.append(dst)
                at = os.path.join(ps.out, rows[n]["atlas"]["file"])
                if not os.path.exists(at) or sha256(at) != rows[n]["atlas"]["sha256"]:
                    stale.append(at)
            if not os.path.exists(ps.manifest) or open(ps.manifest, encoding="utf-8").read() != text:
                stale.append(ps.manifest)
            if stale:
                raise SystemExit("[props] --check: stale or different:\n  " + "\n  ".join(stale))
            print(f"[props] --check: {len(built)} props and props.json match the build")
            return
        os.makedirs(os.path.join(ps.out, A["atlas"]["dir"]), exist_ok=True)
        for n in built:
            shutil.copyfile(os.path.join(tmp, n + ".glb"), os.path.join(ps.out, n + ".glb"))
            shutil.copyfile(os.path.join(tmp, rows[n]["atlas"]["file"]), os.path.join(ps.out, rows[n]["atlas"]["file"]))
        with open(ps.manifest, "w", encoding="utf-8", newline="\n") as f:
            f.write(text)
        print(f"[props] wrote {len(built)} props and their atlases to {ps.out} and {ps.manifest}")
    finally:
        shutil.rmtree(tmp, ignore_errors=True)
