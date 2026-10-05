"""Star Crew's bridge furniture, modelled in Blender the hard-surface CSG way: wall-built console
banks between pierced structural fins, a free-standing helm desk, a curved two-seat helm in the
first series' style, the captain's chair, a crew chair and a stand-up console.

It owns the bridge props' geometry (assets/models/bridge/<name>.glb) and their manifest
(assets/models/bridge/props.json). It lives in tools/blender because meshes are files built by a
committed generator (CLAUDE.md section 9): this script is the source, the .glb files are its
output, and a second run writes the same bytes. How to work this way is the blender-hard-surface
skill (.claude/skills/blender-hard-surface).

Run (from anywhere):
  <python with the bpy module> tools/blender/build_bridge_props.py [--check] [--only a,b] [--blend out.blend]
  blender -b --factory-startup -P tools/blender/build_bridge_props.py -- [--check] [--only a,b] [--blend out.blend]
  --check   build in memory and compare with the committed .glb files and props.json; write nothing
  --only    build only these props (the manifest keeps the others' rows)
  --blend   also save the scene with every cutter collection, for looking at the CSG in Blender
            (a debugging view, not a source: it is not reproducible and is not committed)

The method (the skill's "cutter workflow"): every prop is blocked out from primitives (boxes,
extruded profiles, convex hulls, a lathe for the arc), carved by named cutter objects in a
collection through a Boolean modifier (Exact solver, materials transferred from the cutter's
faces), joined to its other pieces by Boolean unions, chamfered by a 1-segment Bevel on the
edges that should catch light, then cleaned (merge by distance, degenerate and limited
dissolve) and triangulated.

Conventions (written into props.json too):
  * Metres. Prop space is the exported glTF frame: +Y up, +Z toward the operator (out of the
    console's working face; for a chair, the way the sitter faces), +X the operator's right as
    they face the prop. Blender is Z-up: PROP_TO_BLENDER turns every primitive into Blender's
    frame as it is made (x, y, z to x, -z, y), and the glTF export (export_yup) turns it back.
  * The origin is on the floor at the centre of the prop's back: the wall plane for a wall bank,
    the back of the pedestal for everything else.
  * One material per role, named for a Star Crew material (data/materials/materials.json:
    machinery, trim, bulkhead, hazard, light_panel) or `screen` (emissive, coloured by the page)
    or `accent` (the station's role colour, set by the page).
  * UV0 is in metres, projected per face exactly as shipkit.js worldUv does: x, z on faces whose
    normal is within 41 degrees of vertical, else the face's horizontal tangent and y. The page
    divides by the material's span_m.
  * Flat shaded (normals split per face), triangulated, every prop one closed manifold solid.
  * Triangle budgets (BUDGETS): a prop over its budget is refused, named with its count, and
    nothing is written.
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
OUT = os.path.join(ROOT, "assets", "models", "bridge")
MANIFEST = os.path.join(OUT, "props.json")
MATERIALS_JSON = os.path.join(ROOT, "data", "materials", "materials.json")
TEXTURES = os.path.join(ROOT, "assets", "textures")
SHIPKIT = os.path.join(ROOT, "docs", "mockups", "lib", "shipkit.js")
GENERATOR = "tools/blender/build_bridge_props.py"

# Triangles per prop: the budgets these props were briefed with (2026-10-05), inside
# bridge-stations section 12's allowance of 700 for a desk with its screen and 300 for a seat.
# A station's variant (the same body with its own hand controls, bridge-stations 11.6) keeps its
# base's budget.
BUDGETS = {
    "wall_bank_core": 420,
    "wall_bank_core_engineering": 420,
    "wall_bank": 420,
    "wall_bank_comms": 420,
    "wall_bank_double": 700,
    "free_console": 420,
    "free_console_helm": 420,
    "free_console_tactical": 420,
    "helm_arc": 600,
    "captain_chair": 300,
    "crew_chair": 160,
    "standup_console": 160,
}

# Material roles, in slot order (every mesh carries all of them, so a Boolean's material
# transfer never adds a slot and the export's material order is fixed).
ROLES = ("machinery", "trim", "bulkhead", "hazard", "light_panel", "screen", "accent")
PAGE_COLOURED = {"screen": "screen", "accent": "engineering"}   # role -> shipkit PALETTE key for previews
EMISSIVE = {"light_panel", "screen"}

SHOWS = ("console", "upper", "keys")   # what a recorded screen shows (bridge-stations 11.6)
WELD_M = 1e-4          # merge by distance: 0.1 mm, as deckc welds (deck-pipeline section 5)
FLAT_DEG = 1.0         # limited dissolve: faces within 1 degree are one face
CLEAR_M = 0.01         # parallel faces facing the same way stand at least 1 cm apart (CLAUDE.md 8)
TAN20 = math.tan(math.radians(20.0))

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
    """{role: (sRGB colour, emissive)}. Star Crew roles must be materials.json keys."""
    known = json.load(open(MATERIALS_JSON, encoding="utf-8"))["materials"]
    out = {}
    for r in ROLES:
        if r in PAGE_COLOURED:
            out[r] = (_palette(PAGE_COLOURED[r]), r in EMISSIVE)
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
    """One prop: its object, its cutter collections and the facts the manifest records."""

    def __init__(self, name, presents, anchor):
        self.name = name
        self.presents = presents
        self.anchor = anchor
        self.coll = bpy.data.collections.new(name)
        bpy.context.scene.collection.children.link(self.coll)
        self.cutters = bpy.data.collections.new(name + ".cutters")
        self.coll.children.link(self.cutters)
        self.cutters.hide_render = True
        self.body = None
        self.operators = []     # floor points where operators sit or stand (a chair's: under its seat)
        self.seat = None        # a chair's sitting point, top of the cushion
        self.screens = []
        self.steps = []         # the CSG history, for the manifest and the skill
        self.variant_of = None  # a variant's base prop: the same body with a station's own controls
        self.stations = []      # the stations a variant is for (bridge_variants.json station ids)
        self.controls = []      # the hand controls modelled on it, in words, for the manifest

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
        image) or "keys" (a key panel); up is the in-plane direction of its height."""
        ob = obox(self.coll, f"{self.name}.{label}", (-w / 2, -h / 2, -depth), (w / 2, h / 2, 0.05), m,
                  {"-z": floor_role, "*": wall_role})
        coll_objs.append(ob)
        if record:
            if shows not in SHOWS:
                raise SystemExit(f"[props] {self.name}: screen {label} shows {shows!r}, not one of {SHOWS}")
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
    facing the same way, nothing flush with the floor or (for a wall bank) the wall facing the
    same way they do. Returns (triangles, [problems])."""
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
                           "uv": acc(p["attributes"]["TEXCOORD_0"]), "idx": acc(p["indices"])}
    return js, prims


def verify_glb(path, prop):
    """Check the file: +Y up with the floor at y = 0, the back at z = 0, flat normals, UV0 equal
    to worldUv in metres. Returns the manifest's measured facts."""
    js, prims = read_glb(path)
    pts = [p for m in prims.values() for p in m["pos"]]
    lo = [min(p[i] for p in pts) for i in range(3)]
    hi = [max(p[i] for p in pts) for i in range(3)]
    problems = []
    if abs(lo[1]) > 1e-4:
        problems.append(f"lowest point y = {lo[1]:.4f} m, not on the floor (axes?)")
    if prop.wall and abs(lo[2]) > 1e-4:
        problems.append(f"backmost point z = {lo[2]:.4f} m, not on the wall plane")
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


# ----------------------------------------------------------------------------- hand controls
# Keys are texture; levers are geometry (bridge-stations 11.6): the controls a player's hand reaches
# for are small solids unioned onto a desk, each built in a desk frame m (frame(): local +Z the
# desk's normal, +Y up the desk away from the operator, +X their right). Every piece reaches 1-2 cm
# into what it stands on, and pieces that meet keep parallel faces at least 1 cm apart, so the
# union leaves no coplanar faces (CLAUDE.md 8). Sizes are a hand's: a grip 7 cm across, a guard
# 7 cm square, a toggle 4 cm tall.

def _local(m, pts):
    return [tuple(m @ Vector(q)) for q in pts]


def throttle(p, m, tag):
    """A throttle lever: a quadrant housing on the desk, a lever pushed forward of centre and a
    grip across its top in the station's colour."""
    base = obox(p.coll, f"{p.name}.{tag}_quadrant", (-0.04, -0.09, -0.02), (0.04, 0.09, 0.035), m,
                {"*": "machinery", "+z": "bulkhead"})
    lever = hull(p.coll, f"{p.name}.{tag}_lever", _local(m, [(sx * 0.008, y, 0.02) for sx in (-1, 1) for y in (-0.012, 0.012)]
                                                         + [(sx * 0.007, 0.035 + y, 0.125) for sx in (-1, 1) for y in (-0.008, 0.008)]), "trim")
    grip = obox(p.coll, f"{p.name}.{tag}_grip", (-0.035, 0.021, 0.115), (0.035, 0.049, 0.145), m, {"*": "accent"})
    return [base, lever, grip]


def stick(p, m, tag):
    """A flight stick: a square boot, a shaft leaning a little toward the operator and a grip in
    the station's colour."""
    boot = hull(p.coll, f"{p.name}.{tag}_boot", _local(m, [(sx * 0.04, sy * 0.04, -0.02) for sx in (-1, 1) for sy in (-1, 1)]
                                                       + [(sx * 0.026, sy * 0.026, 0.03) for sx in (-1, 1) for sy in (-1, 1)]), "machinery")
    shaft = hull(p.coll, f"{p.name}.{tag}_shaft", _local(m, [(sx * 0.005, sy * 0.005, 0.02) for sx in (-1, 1) for sy in (-1, 1)]
                                                         + [(sx * 0.005, sy * 0.005 - 0.01, 0.13) for sx in (-1, 1) for sy in (-1, 1)]), "trim")
    grip = obox(p.coll, f"{p.name}.{tag}_grip", (-0.016, -0.034, 0.115), (0.016, 0.012, 0.195), m, {"*": "accent"})
    return [boot, shaft, grip]


def guarded_button(p, m, tag):
    """A guarded fire button (design 8.5's guarded controls, as hardware): a housing with hazard
    sides, the button in the station's colour, and its flip cover open, hinged at the back and
    leaning away from the operator."""
    housing = obox(p.coll, f"{p.name}.{tag}_housing", (-0.035, -0.035, -0.02), (0.035, 0.035, 0.022), m,
                   {"*": "hazard", "+z": "machinery"})
    button = obox(p.coll, f"{p.name}.{tag}_button", (-0.016, -0.016, 0.012), (0.016, 0.016, 0.036), m, {"*": "accent"})
    hinge = m @ Matrix.Translation((0.0, 0.03, 0.022)) @ Matrix.Rotation(math.radians(-15.0), 4, "X")
    cover = obox(p.coll, f"{p.name}.{tag}_cover", (-0.025, -0.006, -0.012), (0.025, 0.0, 0.07), hinge, {"*": "trim"})
    return [housing, button, cover]


def toggle(p, m, tag, up):
    """A breaker toggle: a three-sided handle 6 cm tall that widens into a paddle, thrown up
    (away from the operator) and dark when the breaker is closed, down and in the station's
    colour when it has tripped. Larger than life, so it reads from across the room."""
    lean = 0.02 if up else -0.02
    return hull(p.coll, f"{p.name}.{tag}", _local(m, [(-0.011, -0.008, -0.012), (0.011, -0.008, -0.012), (0.0, 0.012, -0.012),
                                                      (-0.014, -0.004 + lean, 0.06), (0.014, -0.004 + lean, 0.06),
                                                      (0.0, 0.006 + lean, 0.06)]), "machinery" if up else "accent")


def slider_bank(p, m, tag, levels):
    """A bank of faders: a block 3 cm high on the desk and a cap per fader in the station's
    colour, each at its own level along the block. Larger than life, so it reads from across
    the room."""
    block = obox(p.coll, f"{p.name}.{tag}_block", (-0.15, -0.035, -0.02), (0.15, 0.035, 0.03), m,
                 {"*": "machinery", "+z": "bulkhead"})
    caps = []
    for i, lv in enumerate(levels):
        x = -0.11 + i * 0.22 / (len(levels) - 1)
        y = -0.015 + 0.03 * lv
        caps.append(obox(p.coll, f"{p.name}.{tag}_cap_{i}", (x - 0.014, y - 0.01, 0.02), (x + 0.014, y + 0.01, 0.06), m,
                         {"*": "accent"}))
    return [block] + caps


# ----------------------------------------------------------------------------- the props

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


def wall_bank(name, bay_w, seats, presents, controls=None):
    """A console bank built into a wall, in a bay between two structural fins.

    Profile (z toward the room, y up): a recessed toe kick, the cabinet front, a desk top
    sloping up from 0.75 m at its front edge, a steeper button band, the main screen panel
    leaning back 20 degrees, a vertical display wall up to 2.03 m and a lit chamfer under the
    2.10 m top; an access panel let into the cabinet front. The fins stand 6 cm proud of the
    desk (so no face of theirs shares its front's plane) and 12 cm above the top, their
    front edges raked at the screen panel's 20 degrees, pierced by a truss of triangular holes.
    A keyboard well 15 cm deep is let into the desk in front of each seat (its key panel is
    texture), and controls adds a station's own hand controls on the desk behind it:
    "breakers" (engineering: a row of five breaker toggles, one tripped) or "sliders" (comms: a
    bank of four faders)."""
    p = Prop(name, presents, "floor, centre of the back, on the wall plane")
    p.wall = True
    hw = bay_w / 2
    D, H = 0.60, 2.10                       # desk depth, top of the bank
    toe_d, toe_h = 0.08, 0.10
    desk_front = (D, 0.75)
    desk_back = (0.32, 0.82)
    band_top = (0.29, 0.88)
    panel_top_y = 1.42
    z_wall = band_top[0] - (panel_top_y - band_top[1]) * TAN20
    strip = (z_wall, 2.02)                  # the light strip runs from here to the top
    strip_dz = 0.045

    # 1. Block out: the cabinet as one box.
    body = prism(p.coll, f"{name}.body", [(0, 0), (D, 0), (D, H), (0, H)], "x", -hw, hw,
                 ["machinery", "bulkhead", "machinery", "machinery"], cap="machinery")

    # 2. Carve the profile: toe kick, the desk, band, screen panel and display wall in one
    #    convex cutter (its faces carry their materials), and the light strip's chamfer.
    x0, x1 = -hw - 0.05, hw + 0.05
    ze = D + 0.05
    y_e = desk_front[1] - (desk_back[1] - desk_front[1]) * (ze - D) / (D - desk_back[0])
    carve = [
        prism(p.coll, f"{name}.cut_toe", [(D - toe_d, -0.05), (ze, -0.05), (ze, toe_h), (D - toe_d, toe_h)], "x",
              x0, x1, ["machinery", "machinery", "machinery", "light_panel"]),
        prism(p.coll, f"{name}.cut_front", [(ze, y_e), desk_back, band_top, (z_wall, panel_top_y), (z_wall, H + 0.1),
                                            (ze, H + 0.1)], "x", x0, x1,
              ["trim", "accent", "machinery", "machinery", "machinery", "machinery"]),
        prism(p.coll, f"{name}.cut_strip", [(strip[0] + 0.05 * strip_dz / 0.08, strip[1] - 0.05),
                                            (strip[0] - strip_dz * 1.6, strip[1] + 0.08 * 1.6),
                                            (strip[0] + 0.1, strip[1] + 0.08 * 1.6)], "x", x0, x1,
              ["light_panel", "machinery", "machinery"]),
    ]
    p.cut(body, "carve", carve)

    # 3. Chamfer the desk's front edge, before anything else meets it.
    p.chamfer(body, "desk_edge", 0.02,
              lambda m, d, n1, n2: abs(m.z - D) < 0.005 and abs(m.y - desk_front[1]) < 0.005 and abs(d.x) > 0.9)

    # 4. The fins: blocked out, chamfered on the edges that face the room, then pierced.
    DF, knee, HF = D + 0.06, 0.90, H + 0.12
    z_top = DF - (HF - knee) * TAN20
    fin_profile = [(0, 0), (DF, 0), (DF, knee), (z_top, HF), (0, HF)]
    # The free part of a fin: in front of the screen panel and display wall, above the band,
    # inside the fin's raked front edge. Webs: 5 cm to the fin's edges and to the cabinet.
    web, gap = 0.05, 0.05
    region = [(0.0, 0.97), (1.0, 0.97), (1.0, 2.12), (0.0, 2.12)]
    region = clip_polygon(region, 1.0, TAN20, DF + knee * TAN20 - web / math.cos(math.radians(20)))   # z <= front - web
    region = clip_polygon(region, -1.0, -TAN20, -(band_top[0] + band_top[1] * TAN20 + gap))           # z >= panel + gap
    region = clip_polygon(region, -1.0, 0.0, -(z_wall + gap))                                           # z >= wall + gap
    # a Warren truss across the strip: inner chord A-E (up the panel), outer chord B-C
    ys = sorted(set(round(q[1], 6) for q in region))
    A = min((q for q in region if abs(q[1] - ys[0]) < 1e-6), key=lambda q: q[0])
    Bq = max((q for q in region if abs(q[1] - ys[0]) < 1e-6), key=lambda q: q[0])
    E = (z_wall + gap, band_top[1] + (band_top[0] + gap - (z_wall + gap)) / TAN20)
    C = max(region, key=lambda q: q[1] - 0.001 * q[0])
    front = lambda y: DF - (y - knee) * TAN20 - web / math.cos(math.radians(20))
    P1 = (front(E[1]), E[1])
    holes = [triangle_inset(t, web / 2) for t in ((A, Bq, P1), (A, P1, E), (E, P1, C))]
    holes = [h for h in holes if h]
    fins = []
    for side in (-1, 1):
        xa, xb = sorted((side * (hw - 0.01), side * (hw + 0.12)))
        fin = prism(p.coll, f"{name}.fin_{'l' if side < 0 else 'r'}", fin_profile, "x", xa, xb, "trim")
        p.chamfer(fin, f"fin_{'l' if side < 0 else 'r'}_edges", 0.015,
                  lambda m, d, n1, n2: m.y > 0.01 and m.z > 0.01)
        cutters = [prism(p.coll, f"{name}.fin_hole_{i}", list(h), "x", xa - 0.05, xb + 0.05, "machinery")
                   for i, h in enumerate(holes)]
        p.cut(fin, f"fin_{'l' if side < 0 else 'r'}_holes", cutters)
        fins.append(fin)
    p.union(body, "fins", fins)

    # 5. A station's hand controls on the desk, behind the keyboard well (desk_at: a point s
    #    metres up the desk from its front edge).
    slope = math.atan2(desk_back[1] - desk_front[1], desk_front[0] - desk_back[0])
    desk_tilt = 90.0 - math.degrees(slope)
    desk_at = lambda x, s: (x, desk_front[1] + s * math.sin(slope), desk_front[0] - s * math.cos(slope))
    if controls == "breakers":
        pieces = [toggle(p, frame(desk_at(-0.30 + 0.15 * i, 0.235), desk_tilt), f"breaker_{i}", up)
                  for i, up in enumerate((True, True, False, True, True))]
        p.union(body, "breakers", pieces)
        p.controls.append("five breaker toggles behind the keyboard, the third tripped")
    elif controls == "sliders":
        p.union(body, "sliders", slider_bank(p, frame(desk_at(0.25, 0.2375), desk_tilt), "faders", (0.8, 0.45, 0.65, 0.15)))
        p.controls.append("a bank of four faders behind the keyboard, to the operator's right")
    elif controls is not None:
        raise SystemExit(f"[props] {name}: unknown controls {controls!r}")

    # 6. Screens, cut last so no bevel or union touches their edges.
    screens = []
    t = math.hypot(band_top[0] - z_wall, panel_top_y - band_top[1])
    pc = (0.0, (band_top[1] + panel_top_y) / 2, (band_top[0] + z_wall) / 2)
    main_h = 0.50 if bay_w >= 1.4 else 0.45
    if seats == 1:
        main = [(0.0, 1.20 if bay_w >= 1.4 else 0.90)]
    else:
        main = [(-bay_w / 4, bay_w / 2 - 0.20), (bay_w / 4, bay_w / 2 - 0.20)]
    assert main_h <= t - 0.06, "main screen taller than its panel"
    for i, (cx, w) in enumerate(main):
        p.recess(screens, f"screen_main_{i}", frame((cx, pc[1], pc[2]), 20.0), w, main_h, 0.02, shows="console")
    n_up = 2 * seats
    w_up = (bay_w - 0.16 - (n_up - 1) * 0.08) / n_up
    yc = (panel_top_y + strip[1]) / 2
    for i in range(n_up):
        cx = -bay_w / 2 + 0.08 + w_up / 2 + i * (w_up + 0.08)
        p.recess(screens, f"screen_upper_{i}", frame((cx, yc, z_wall)), w_up, 0.40, 0.02, shows="upper", half=i % 2)
    # a keyboard well per seat, 12 mm deep, from 3.5 cm behind the desk's front edge
    for i, cx in enumerate([0.0] if seats == 1 else [-bay_w / 4, bay_w / 4]):
        kw = 1.20 if bay_w >= 1.4 and seats == 1 else 0.86
        p.recess(screens, f"keys_{i}", frame(desk_at(cx, 0.11), desk_tilt), kw, 0.15, 0.012, shows="keys")
    # an access panel per seat in the cabinet front, 12 mm deep (parallel faces stay 1 cm apart)
    for i, cx in enumerate([0.0] if seats == 1 else [-bay_w / 4, bay_w / 4]):
        w = (bay_w if seats == 1 else bay_w / 2) - 0.20
        p.recess(screens, f"access_{i}", frame((cx, 0.41, D)), w, 0.42, 0.012, floor_role="bulkhead", record=False)
    p.cut(body, "screens", screens)

    p.body = body
    desk_mid = (desk_front[0] + 0.0) / 2
    for cx in ([0.0] if seats == 1 else [-bay_w / 4, bay_w / 4]):
        p.operators.append([round(cx, 4), 0.0, round(desk_mid + 0.65, 4)])
    p.fin_holes = len(holes)
    return p


def free_console(name, presents, controls=None):
    """A free-standing helm or tactical desk: a sloped top on an angled pedestal over a plinth,
    a touch panel and a button band let into the top and a small upright screen at its back
    edge. controls adds a station's own hand controls in the strips either side of the touch
    panel: "helm" (a throttle lever to the operator's left, a stick to their right) or
    "tactical" (two guarded fire buttons to their right)."""
    p = Prop(name, presents, "floor, centre of the pedestal's back")
    p.wall = False
    hw, zb, zf = 0.70, -0.04, 0.56          # desk half width, back and front edges (z)
    slope = math.tan(math.radians(12.0))
    top = lambda z: 0.75 + (zf - z) * slope
    desk = prism(p.coll, f"{name}.desk", [(zb, 0.58), (zf, 0.58), (zf, 0.95), (zb, 0.95)], "x", -hw, hw, "machinery")
    c = 0.14                                # plan chamfer on the front corners
    carve = [
        prism(p.coll, f"{name}.cut_top", [(zf + 0.05, top(zf + 0.05)), (zb - 0.05, top(zb - 0.05)), (zb - 0.05, 1.2),
                                          (zf + 0.05, 1.2)], "x", -hw - 0.05, hw + 0.05, "trim"),
        prism(p.coll, f"{name}.cut_under", [(zf + 0.05, 0.73), (zf - 0.26, 0.545), (zf + 0.05, 0.545)], "x",
              -hw - 0.05, hw + 0.05, "machinery"),
    ]
    for s in (-1, 1):
        pts = [(s * (hw - c - 0.05), zf + 0.05), (s * (hw + 0.05), zf - c - 0.05), (s * (hw + 0.05), zf + 0.05)]
        if s < 0:
            pts.reverse()
        carve.append(prism(p.coll, f"{name}.cut_corner_{s:+d}", pts, "y", 0.5, 1.3, "machinery"))
    p.cut(desk, "carve", carve)
    p.chamfer(desk, "top_edges", 0.02, lambda m, d, n1, n2: m.y > 0.70 and max(n1.y, n2.y) > 0.9)

    ped = hull(p.coll, f"{name}.pedestal", [(sx * 0.36, 0.0, z) for sx in (-1, 1) for z in (0.0, 0.26)]
               + [(sx * 0.52, 0.60, z) for sx in (-1, 1) for z in (0.0, 0.40)], "bulkhead")
    plinth = prism(p.coll, f"{name}.plinth", [(-0.08, 0.0), (0.36, 0.0), (0.36, 0.06), (-0.08, 0.06)], "x",
                   -0.46, 0.46, "trim")
    p.chamfer(plinth, "plinth_edges", 0.02, lambda m, d, n1, n2: m.y > 0.03)
    # The upright screen's housing leans back 20 degrees from the back of the top.
    zh = 0.15
    m_h = frame((0.0, top(zh), zh), 20.0)
    housing = obox(p.coll, f"{name}.housing", (-0.45, -0.06, -0.07), (0.45, 0.42, 0.0), m_h, {"*": "machinery"})
    p.chamfer(housing, "housing_edges", 0.015, lambda m, d, n1, n2: m.y > top(0.0) + 0.05)
    p.union(desk, "pedestal_plinth_housing", [ped, plinth, housing])

    on_top = lambda x, z: frame((x, top(z), z), 78.0)      # the desk top slopes 12 degrees
    if controls == "helm":
        p.union(desk, "helm_controls", throttle(p, on_top(-0.625, 0.33), "throttle") + stick(p, on_top(0.625, 0.33), "stick"))
        p.controls += ["a throttle lever to the operator's left", "a stick to their right"]
    elif controls == "tactical":
        p.union(desk, "fire_buttons", guarded_button(p, on_top(0.62, 0.26), "fire_0") + guarded_button(p, on_top(0.62, 0.40), "fire_1"))
        p.controls.append("two guarded fire buttons to the operator's right")
    elif controls is not None:
        raise SystemExit(f"[props] {name}: unknown controls {controls!r}")

    screens = []
    p.recess(screens, "screen_upright", m_h @ Matrix.Translation((0, 0.20, 0)), 0.80, 0.30, 0.015, shows="console")
    zt = 0.36
    p.recess(screens, "touch_panel", on_top(0.0, zt), 1.10, 0.24, 0.012, shows="keys")
    zb_ = 0.205                             # a button band in the station's colour behind the touch panel
    p.recess(screens, "button_band", frame((0.0, top(zb_), zb_), 78.0), 1.10, 0.05, 0.012, floor_role="accent",
             record=False)
    p.cut(desk, "screens", screens)
    p.body = desk
    p.operators.append([0.0, 0.0, round((zb + zf) / 2 + 0.65, 4)])
    return p


def helm_arc(name, presents):
    """A curved two-seat console in the first series' style: a lathe of one profile (toe kicks
    front and back, a sloped top, a raised rim with a button band) over an arc 2.4 m long at
    mid depth, a hooded viewer between the two seats and a key panel let into the top at each.
    Seat 0 (to port of the centre, the operator's left) is helm's, with a throttle lever left of
    its panel and a stick between the panel and the viewer; seat 1 is tactical's, with two
    guarded fire buttons right of its panel. Each control stands on one facet of the lathe."""
    p = Prop(name, presents, "floor, centre of the back of the arc")
    p.wall = False
    depth, r_mid, arc = 0.60, 2.90, 2.40
    r_back = r_mid + depth / 2
    half = arc / r_mid / 2
    profile = [(0.08, 0.0), (0.46, 0.0), (0.46, 0.10), (0.60, 0.10), (0.60, 0.75), (0.14, 0.85), (0.10, 0.90),
               (0.0, 0.90), (0.0, 0.10), (0.08, 0.10)]
    roles = ["machinery", "light_panel", "machinery", "bulkhead", "machinery", "accent", "trim", "bulkhead",
             "machinery", "machinery"]
    steps = 8
    body = lathe(p.coll, f"{name}.body", profile, roles, r_back, r_back, -half, half, steps, "trim")
    p.chamfer(body, "front_edge", 0.02,
              lambda m, d, n1, n2: abs(math.hypot(m.x, m.z - r_back) - (r_back - 0.60)) < 0.01 and abs(m.y - 0.75) < 0.01)
    # the hooded viewer: a trapezoid prism, its front face toward the seats
    hood = hull(p.coll, f"{name}.hood", [(sx * 0.17, y, r_back - (r_back - d)) for sx in (-1, 1)
                                         for d, y in ((0.44, 0.76), (0.32, 1.02), (0.06, 1.02), (0.06, 0.82))], "bulkhead")
    p.chamfer(hood, "hood_edges", 0.012, lambda m, d, n1, n2: m.y > 1.0)
    p.union(body, "hood", [hood])
    slope = math.degrees(math.atan2(0.10, 0.46))
    facet = 2 * half / steps

    def on_top(phi, d):
        """A desk frame on the facet of the lathe's top that holds angle phi, at depth d in from
        the back: the point on the facet's chord, the facet's own yaw and the top's slope."""
        k = min(steps - 1, max(0, int((phi + half) / facet)))
        a = -half + k * facet
        y = 0.75 + (0.60 - d) / 0.46 * 0.10
        pa = Vector((math.sin(a) * (r_back - d), y, r_back - math.cos(a) * (r_back - d)))
        pb = Vector((math.sin(a + facet) * (r_back - d), y, r_back - math.cos(a + facet) * (r_back - d)))
        return frame(tuple(pa.lerp(pb, (phi - a) / facet)), 90.0 - slope, -math.degrees(a + facet / 2))
    p.union(body, "controls", throttle(p, on_top(-0.362, 0.37), "throttle") + stick(p, on_top(-0.0855, 0.42), "stick")
            + guarded_button(p, on_top(0.3355, 0.36), "fire_0") + guarded_button(p, on_top(0.3885, 0.36), "fire_1"))
    p.controls += ["helm (seat 0): a throttle lever left of its panel, a stick right of it",
                   "tactical (seat 1): two guarded fire buttons right of its panel"]
    screens = []
    for i, s in enumerate((-1, 1)):
        phi = s * 0.6 / r_mid
        d = 0.36
        y = 0.75 + (0.60 - d) / 0.46 * 0.10
        c = (math.sin(phi) * (r_back - d), y, r_back - math.cos(phi) * (r_back - d))
        p.recess(screens, f"panel_{i}", frame(c, 90.0 - slope, -math.degrees(phi)), 0.56, 0.24, 0.02, shows="keys")
        r_op = r_mid - 0.65
        p.operators.append([round(math.sin(phi) * r_op, 4), 0.0, round(r_back - math.cos(phi) * r_op, 4)])
    # the viewer's screen on the hood's sloped face, which runs from (z 0.44, y 0.76) to (0.32, 1.02)
    hood_tilt = math.degrees(math.atan2(0.12, 0.26))
    yv = 0.92
    p.recess(screens, "viewer", frame((0.0, yv, 0.44 - 0.12 * (yv - 0.76) / 0.26), hood_tilt), 0.22, 0.12, 0.015,
             shows="upper", half=0)
    p.cut(body, "screens", screens)
    p.body = body
    return p


def chair(name, presents, captain):
    """A swivel chair: a hexagonal foot, a pedestal, a seat shell with its cushion (top at
    0.45 m), a back shell leaning 10 degrees with its cushion and a headrest band in the
    station's role colour; the captain's adds armrests 0.15 m wide with a small console let
    into each, and a taller back."""
    p = Prop(name, presents, "floor, centre of the pedestal's back")
    p.wall = False
    col_r = 0.07 if captain else 0.055
    az = col_r * math.cos(math.pi / 6)        # the column's flat back is at z = 0
    seat_w, seat_d = (0.60, 0.52) if captain else (0.55, 0.50)
    foot_r = 0.32 if captain else 0.27
    foot = prism(p.coll, f"{name}.foot", ngon(0.0, az, foot_r, 6), "y", 0.0, 0.05, "trim")
    p.chamfer(foot, "foot_edges", 0.02, lambda m, d, n1, n2: m.y > 0.03)
    column = prism(p.coll, f"{name}.column", ngon(0.0, az, col_r, 6), "y", 0.03, 0.37, "trim")
    # the seat: a trim shell with a cushion inset 25 mm, its top at 0.45 m
    z0, z1 = az - seat_d / 2, az + seat_d / 2
    shell = obox(p.coll, f"{name}.seat_shell", (-seat_w / 2, 0.35, z0), (seat_w / 2, 0.40, z1), Matrix(), {"*": "trim"})
    cushion = obox(p.coll, f"{name}.seat_cushion", (-seat_w / 2 + 0.025, 0.39, z0 + 0.02), (seat_w / 2 - 0.025, 0.45, z1 - 0.015),
                   Matrix(), {"*": "machinery"})
    p.chamfer(cushion, "cushion_edge", 0.02, lambda m, d, n1, n2: m.y > 0.44 and m.z > z1 - 0.03 and abs(d.x) > 0.9)
    # the back: a trim shell leaning back, a cushion 25 mm proud of it, the headrest band on top
    back_h = 0.78 if captain else 0.64
    m_b = frame((0.0, 0.40, z0 + 0.03), 12.0 if captain else 10.0)
    hb = seat_w / 2 - 0.02
    back = obox(p.coll, f"{name}.back", (-hb, 0.0, -0.08), (hb, back_h, 0.0), m_b, {"*": "trim"})
    pad = obox(p.coll, f"{name}.back_cushion", (-hb + 0.03, 0.07, -0.01), (hb - 0.03, back_h - 0.10, 0.025), m_b,
               {"*": "machinery"})
    band = obox(p.coll, f"{name}.headrest", (-hb - 0.01, back_h - 0.09, -0.09), (hb + 0.01, back_h + 0.01, 0.012),
                m_b, {"*": "accent"})
    seat = shell
    parts = [foot, column, seat, cushion, back, pad, band]
    screens = []
    if captain:
        zr, zf = az - 0.20, az + 0.30
        for s in (-1, 1):
            xa, xb = sorted((s * (seat_w / 2 - 0.02), s * (seat_w / 2 + 0.13)))
            arm = prism(p.coll, f"{name}.arm_{s:+d}", [(zr, 0.36), (zf - 0.08, 0.36), (zf, 0.44), (zf, 0.62), (zr, 0.66)],
                        "x", xa, xb, ["machinery", "machinery", "trim", "machinery", "trim"], cap="machinery")
            p.chamfer(arm, f"arm_{s:+d}_edges", 0.015, lambda m, d, n1, n2: m.y > 0.6 and abs(d.z) > 0.9)
            parts.append(arm)
            tilt = 90.0 - math.degrees(math.atan2(0.04, zf - zr))
            ymid = 0.62 + 0.04 * 0.5
            p.recess(screens, f"arm_panel_{s:+d}", frame((s * (seat_w / 2 + 0.055), ymid, (zr + zf) / 2), tilt),
                     0.10, 0.30, 0.012, shows="keys")
    p.union(foot, "chair", parts[1:])
    if screens:
        p.cut(foot, "screens", screens)
    p.body = foot
    p.seat = [0.0, 0.45, round(az, 4)]
    p.operators.append([0.0, 0.0, round(az, 4)])      # the floor under the seat
    return p


def standup_console(name, presents):
    """A pedestal stand-up console: a base plate, a column leaning toward the operator and a
    head whose sloped top (about 1.1 m) holds one screen."""
    p = Prop(name, presents, "floor, centre of the pedestal's back")
    p.wall = False
    zb, zf = -0.04, 0.42
    yb, yf = 1.16, 1.04
    top = lambda z: yf + (zf - z) / (zf - zb) * (yb - yf)
    head = prism(p.coll, f"{name}.head", [(zb, 0.86), (zf, 0.86), (zf, 1.25), (zb, 1.25)], "x", -0.32, 0.32, "machinery")
    p.cut(head, "carve", [
        prism(p.coll, f"{name}.cut_top", [(zf + 0.05, top(zf + 0.05)), (zb - 0.05, top(zb - 0.05)), (zb - 0.05, 1.4),
                                          (zf + 0.05, 1.4)], "x", -0.37, 0.37, "trim"),
        prism(p.coll, f"{name}.cut_under", [(zf + 0.05, 1.01), (zf - 0.14, 0.835), (zf + 0.05, 0.835)], "x",
              -0.37, 0.37, "machinery"),
    ])
    p.chamfer(head, "top_edges", 0.015, lambda m, d, n1, n2: max(n1.y, n2.y) > 0.9)
    column = hull(p.coll, f"{name}.column", [(sx * 0.11, 0.03, z) for sx in (-1, 1) for z in (0.0, 0.18)]
                  + [(sx * 0.15, 0.90, z) for sx in (-1, 1) for z in (0.06, 0.24)], "bulkhead")
    base = prism(p.coll, f"{name}.base", [(-0.10, 0.0), (0.32, 0.0), (0.32, 0.05), (-0.10, 0.05)], "x", -0.25, 0.25, "trim")
    p.chamfer(base, "base_edges", 0.02, lambda m, d, n1, n2: m.y > 0.03)
    p.union(head, "column_base", [column, base])
    zt = (zb + zf) / 2 + 0.02
    slope = math.degrees(math.atan2(yb - yf, zf - zb))
    screens = []
    p.recess(screens, "screen", frame((0.0, top(zt), zt), 90.0 - slope), 0.50, 0.30, 0.015, shows="console")
    p.cut(head, "screens", screens)
    p.body = head
    p.operators.append([0.0, 0.0, round(zf + 0.35, 4)])
    return p


def variant(p, base, stations):
    """Mark a prop as a station's variant of base: the variants file (tools/bridge_variants.py)
    and the mockup place it wherever base would stand at one of these stations."""
    p.variant_of, p.stations = base, list(stations)
    return p


PROPS = {
    "wall_bank_core": lambda: wall_bank("wall_bank_core", 1.40, 1,
                                        "A core station built into a wall (bridge-stations 11.1: desk 1.40 m)"),
    "wall_bank_core_engineering": lambda: variant(wall_bank("wall_bank_core_engineering", 1.40, 1,
                                                            "Engineering's core bank, with its breaker toggles", "breakers"),
                                                  "wall_bank_core", ["engineering"]),
    "wall_bank": lambda: wall_bank("wall_bank", 1.10, 1,
                                   "A station built into a wall (bridge-stations 11.1: desk 1.10 m)"),
    "wall_bank_comms": lambda: variant(wall_bank("wall_bank_comms", 1.10, 1, "Comms' bank, with its slider bank", "sliders"),
                                       "wall_bank", ["comms"]),
    "wall_bank_double": lambda: wall_bank("wall_bank_double", 2.40, 2, "Two seats in one wall bay"),
    "free_console": lambda: free_console("free_console", "The free-standing helm or tactical desk (desk 1.40 m)"),
    "free_console_helm": lambda: variant(free_console("free_console_helm", "Helm's free-standing desk, with its throttle and stick",
                                                      "helm"), "free_console", ["helm"]),
    "free_console_tactical": lambda: variant(free_console("free_console_tactical",
                                                          "Tactical's free-standing desk, with its guarded fire buttons", "tactical"),
                                             "free_console", ["tactical"]),
    "helm_arc": lambda: helm_arc("helm_arc", "A curved two-seat helm, the first series' style"),
    "captain_chair": lambda: chair("captain_chair", "The captain's chair on the dais", True),
    "crew_chair": lambda: chair("crew_chair", "A station's swivel chair", False),
    "standup_console": lambda: standup_console("standup_console", "A pedestal stand-up console"),
}


# ----------------------------------------------------------------------------- main

def parse_args():
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
            raise SystemExit(f"[props] unknown argument {a!r}\n{__doc__}")
        i += 1
    for n in opts["only"] or []:
        if n not in PROPS:
            raise SystemExit(f"[props] no prop {n!r}; the props are {', '.join(PROPS)}")
    return opts


def sha256(path):
    return hashlib.sha256(open(path, "rb").read()).hexdigest()


def gltf_exporter_version():
    import addon_utils
    for m in addon_utils.modules():
        if m.__name__ == "io_scene_gltf2":
            return ".".join(str(v) for v in m.bl_info["version"])
    return "unknown"


def manifest_text(rows):
    doc = {
        "schema": "starcrew.props/1",
        "status": "Built (2026-10-05) by " + GENERATOR + ". Proposed furniture for the bridge "
                  "(bridge-stations section 11.1, deck-pipeline section 5). docs/mockups/bridge-variants.html shows them "
                  "(tools/mockups/inline.py models:bridge copies them in); no engine code loads them yet.",
        "_rules": [
            "This file is written by " + GENERATOR + "; never edit it by hand. Rebuild, and the .glb files and this file change together.",
            "Metres. Prop space is the glTF frame: +Y up, +Z toward the operator (out of a console's working face; for a chair, the way the sitter faces), +X the operator's right as they face the prop.",
            "The origin is on the floor at the centre of the prop's back: the wall plane for a wall bank, the back of the pedestal otherwise.",
            "Placing props at a layout station (seat_m on the floor, yaw_deg, the seat facing +Z at yaw 0): operators_m are floor points, so the layout's seat_m lands on one. A chair turns by yaw_deg (its operators_m is the floor under its seat); a console faces the seat, so it turns by yaw_deg + 180. seat_m in a chair's row is the sitting point on the cushion, 0.45 m up.",
            "Materials are roles: machinery, trim, bulkhead, hazard and light_panel are data/materials/materials.json layers; screen is emissive and coloured by the page (its console UI); accent is the station's role colour.",
            "screens are the recess floors a page draws on: centre_m on the floor, normal out of it, up the in-plane direction of height_m (width_m runs along up x normal). shows says what: console (the station's main screen image), upper (half 0 or 1 of its upper image) or keys (a key panel); the images and how to fit them are assets/textures/screens/screens.json (bridge-stations 11.6).",
            "A row with variant_of is that prop with a station's own hand controls (controls says which): it stands wherever variant_of would at one of its stations, with the same operators_m.",
            "UV0 is in metres, projected per face as shipkit.js worldUv does (x, z where |n.y| > 0.75, else the face's horizontal tangent and y); divide by the material's span_m.",
            "Flat shaded (one normal per face), triangulated, one closed manifold solid per prop; faces against the floor or the wall are kept for the deck compiler to drop.",
            "triangles is counted in the .glb; the build refuses a prop over budget_triangles. sha256 is the .glb's, from the Blender and exporter versions in generator: a second build with them writes the same bytes.",
        ],
        "generator": {"script": GENERATOR, "blender": bpy.app.version_string, "gltf_exporter": gltf_exporter_version()},
        "props": rows,
    }
    return json.dumps(doc, indent=2, ensure_ascii=True) + "\n"


def main():
    opts = parse_args()
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.context.scene.name = "BridgeProps"
    bpy.context.scene.unit_settings.system = "METRIC"
    make_materials()
    names = opts["only"] or list(PROPS)
    built, failures = {}, []
    for n in names:
        p = PROPS[n]()
        p.body.name = n
        p.body.data.name = n
        clean(p.body)
        tris, problems = check(p)
        if tris > BUDGETS[n]:
            problems.append(f"{tris} triangles, over its budget of {BUDGETS[n]}")
        finish(p.body)
        print(f"[props] {n}: {tris} triangles (budget {BUDGETS[n]}), {len(p.screens)} screens; " + "; ".join(p.steps))
        if problems:
            failures.append(f"{n}: " + "; ".join(sorted(set(problems))))
        built[n] = p
    if opts["blend"]:
        bpy.ops.wm.save_as_mainfile(filepath=opts["blend"], compress=True)
        print("[props] saved", opts["blend"])
    if failures:
        raise SystemExit("[props] refused, nothing written:\n  " + "\n  ".join(failures))

    tmp = tempfile.mkdtemp(prefix="bridge_props_")
    try:
        rows = {}
        old = {}
        if os.path.exists(MANIFEST):
            old = json.load(open(MANIFEST, encoding="utf-8")).get("props", {})
        for n in PROPS:
            if n not in built:
                if n in old:
                    rows[n] = old[n]
                continue
            p = built[n]
            path = os.path.join(tmp, n + ".glb")
            export_glb(p.body, path)
            facts = verify_glb(path, p)
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
            row["screens"] = p.screens
            row["triangles"] = facts["triangles"]
            row["budget_triangles"] = BUDGETS[n]
            row["materials"] = facts["materials"]
            row["csg"] = p.steps
            row["sha256"] = sha256(path)
            rows[n] = row
        text = manifest_text(rows)
        if opts["check"]:
            stale = []
            for n in built:
                dst = os.path.join(OUT, n + ".glb")
                if not os.path.exists(dst) or sha256(dst) != rows[n]["sha256"]:
                    stale.append(dst)
            if not os.path.exists(MANIFEST) or open(MANIFEST, encoding="utf-8").read() != text:
                stale.append(MANIFEST)
            if stale:
                raise SystemExit("[props] --check: stale or different:\n  " + "\n  ".join(stale))
            print(f"[props] --check: {len(built)} props and props.json match the build")
            return
        os.makedirs(OUT, exist_ok=True)
        for n in built:
            shutil.copyfile(os.path.join(tmp, n + ".glb"), os.path.join(OUT, n + ".glb"))
        with open(MANIFEST, "w", encoding="utf-8", newline="\n") as f:
            f.write(text)
        print(f"[props] wrote {len(built)} props to {OUT} and {MANIFEST}")
    finally:
        shutil.rmtree(tmp, ignore_errors=True)


if __name__ == "__main__":
    main()
