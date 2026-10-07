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
    return row


def run(ps):
    """The build: make the materials, build every prop (or --only these), clean, check against
    the budget, finish; refuse the whole set on any problem; export, read back and write the glbs
    and the manifest (or, with --check, compare them with the committed files)."""
    opts = parse_args(ps.props, ps.doc)
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
        print(f"[props] {n}: {tris} triangles (budget {ps.budgets[n]}), {len(p.screens)} screens; " + "; ".join(p.steps))
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
            rows[n] = manifest_row(n, p, facts, ps.budgets[n], sha256(path))
        text = manifest_text(ps, rows)
        if opts["check"]:
            stale = []
            for n in built:
                dst = os.path.join(ps.out, n + ".glb")
                if not os.path.exists(dst) or sha256(dst) != rows[n]["sha256"]:
                    stale.append(dst)
            if not os.path.exists(ps.manifest) or open(ps.manifest, encoding="utf-8").read() != text:
                stale.append(ps.manifest)
            if stale:
                raise SystemExit("[props] --check: stale or different:\n  " + "\n  ".join(stale))
            print(f"[props] --check: {len(built)} props and props.json match the build")
            return
        os.makedirs(ps.out, exist_ok=True)
        for n in built:
            shutil.copyfile(os.path.join(tmp, n + ".glb"), os.path.join(ps.out, n + ".glb"))
        with open(ps.manifest, "w", encoding="utf-8", newline="\n") as f:
            f.write(text)
        print(f"[props] wrote {len(built)} props to {ps.out} and {ps.manifest}")
    finally:
        shutil.rmtree(tmp, ignore_errors=True)
