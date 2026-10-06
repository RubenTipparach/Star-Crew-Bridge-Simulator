"""Star Crew's machinery props, modelled in Blender the hard-surface CSG way: the ship systems (the
switchboard, the battery bank, the coolant pumps, the impulse drive, the inertial dampers, the shield
generator, life support's oxygen tanks, scrubbers and air handler, the gravity generator, the
reactor), the weapons and craft (the magazine's missile racks, a missile tube, a launch cradle, the
Swift fighter and the Petrel shuttle) and the crew rooms' furniture (a medical bed, a bunk, a mess
table, a galley counter). They replace the plain grey boxes the deck plan mockup draws for them.

It owns the machinery props' geometry (assets/models/machinery/<name>.glb) and their manifest
(assets/models/machinery/props.json). It lives in tools/blender because meshes are files built by a
committed generator (CLAUDE.md section 9): this script is the source, the .glb files are its output,
and a second run writes the same bytes. It holds only these props and one primitive of their own
(revolve, a solid of revolution with a stepped profile, for tanks, pipes, coils, nozzles and the
reactor); the machinery every prop set shares (materials, primitives, the Prop class and its CSG
steps, clean, check, the glb export and read-back, the manifest and the command line) is the
hard-surface kit, tools/blender/hs_kit.py, which tools/blender/build_bridge_props.py and
build_suite_props.py use too. How to work this way is the blender-hard-surface skill
(.claude/skills/blender-hard-surface).

Run (from anywhere):
  <python with the bpy module> tools/blender/build_machinery_props.py [--check] [--only a,b] [--blend out.blend]
  blender -b --factory-startup -P tools/blender/build_machinery_props.py -- [--check] [--only a,b] [--blend out.blend]
  --check   build in memory and compare with the committed .glb files and props.json; write nothing
  --only    build only these props (the manifest keeps the others' rows)
  --blend   also save the scene with every cutter collection, for looking at the CSG in Blender
            (a debugging view, not a source: it is not reproducible and is not committed)

The method is the bridge and suite props' (the skill's "cutter workflow"): block out from boxes,
extruded profiles, convex hulls and solids of revolution; carve with named cutters (Exact solver,
materials transferred from the cutters' faces); union the pieces; chamfer the edges that catch light
with a 1-segment Bevel; cut the recesses (screens, doors, vents, dials) last; clean, triangulate and
check.

Conventions (written into props.json too):
  * Metres. Prop space is the exported glTF frame: +Y up, +Z toward the user (the prop's front: the
    side it is worked from; a craft's nose), +X the user's right as they face the prop.
  * Three anchors. A WALL prop has its origin on the floor at the centre of its back, the back flat
    on the wall plane z = 0. A FREE prop has its origin on the floor at the centre of its footprint.
    A CRAFT has its origin on the floor at the centre of its footprint, its nose toward +Z.
  * One material per role, named for a Star Crew material (data/materials/materials.json:
    machinery, trim, bulkhead, hazard, light_panel) or `screen` (emissive, coloured by the page) or
    `accent` (blankets, valve wheels, bus bars, coil windings: tinted by the page).
  * UV0 in metres as shipkit.js worldUv; flat shaded; triangulated; one closed manifold solid per
    prop; a prop over its triangle budget (BUDGETS) is refused and nothing is written.
"""
import math
import os
import sys

import bpy  # first: with the pip bpy module, bmesh and mathutils exist only once bpy is imported
import bmesh  # noqa: E402
from mathutils import Matrix, Vector  # noqa: E402

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from hs_kit import ROLES, ROOT, Prop, PropSet, _object, frame, ngon, run  # noqa: E402

OUT = os.path.join(ROOT, "assets", "models", "machinery")
GENERATOR = "tools/blender/build_machinery_props.py"

# Triangles per prop: the budgets these props were briefed with (2026-10-06).
BUDGETS = {
    "switchboard": 600,
    "battery_bank": 500,
    "coolant_pumps": 600,
    "impulse_drive": 900,
    "inertial_dampers": 500,
    "shield_generator": 700,
    "ls_tanks": 500,
    "ls_scrubbers": 500,
    "ls_air_handler": 500,
    "gravity_generator": 500,
    "med_bed": 400,
    "magazine_rack": 900,
    "missile_tube": 500,
    "reactor_core": 1200,
    "launch_cradle": 400,
    "swift_fighter": 1000,
    "petrel_shuttle": 1400,
    "bunk": 300,
    "mess_table": 300,
    "galley_counter": 450,
}

WALL = "floor, centre of the back, on the wall plane"
FREE = "floor, centre of the footprint"
CRAFT = "floor, centre of the footprint, nose toward +Z"


def prop(name, presents, anchor):
    """A machinery prop: on a wall (its back at z = 0) when the anchor says so, else centred."""
    return Prop(name, presents, anchor, wall=(anchor == WALL))


# ----------------------------------------------------------------------------- primitives of this set

def revolve(p, what, prof, roles, axis, c, sides=8, caps=None, phase=None):
    """A solid of revolution: a stepped profile of (r, t) points swept round an axis in `sides`
    flat facets, closed by a flat cap at each end.

    r is the apothem (centre to the middle of a facet, so a profile's r is the half width a ruler
    measures across the flats) and t the position along the axis. axis 'y' turns about the vertical
    line through c = (x, z); 'z' about the line along Z through c = (x, y); 'x' about the line along
    X through c = (y, z). With the default phase a flat faces -z (axis y) or the floor (axes z and
    x), so with 8 or 12 sides the flats also face the other three ways. Segment i (profile point i
    to i + 1) takes roles[i]; caps are the roles of the first and last rings' faces. Every quad is
    planar (both its rings are the same regular polygon), so a profile is as cheap as it looks:
    2 x sides triangles a segment and sides - 2 a cap."""
    n = sides
    ph = math.pi / n if phase is None else phase
    if isinstance(roles, str):
        roles = [roles] * (len(prof) - 1)
    if len(roles) != len(prof) - 1:
        raise SystemExit(f"[props] {p.name}.{what}: {len(roles)} roles for {len(prof) - 1} segments")
    caps = caps or (roles[0], roles[-1])
    bm = bmesh.new()
    rings = []
    for r, t in prof:
        rr = r / math.cos(math.pi / n)
        ring = []
        for k in range(n):
            a = ph + 2 * math.pi * k / n
            s, co = rr * math.sin(a), rr * math.cos(a)
            if axis == "y":
                v = (c[0] + s, t, c[1] - co)
            elif axis == "z":
                v = (c[0] + s, c[1] - co, t)
            else:
                v = (t, c[0] - co, c[1] + s)
            ring.append(bm.verts.new(Vector(v)))
        rings.append(ring)
    for i in range(len(prof) - 1):
        for k in range(n):
            j = (k + 1) % n
            f = bm.faces.new((rings[i][k], rings[i][j], rings[i + 1][j], rings[i + 1][k]))
            f.material_index = ROLES.index(roles[i])
    for ring, role in ((rings[0], caps[0]), (rings[-1], caps[1])):
        f = bm.faces.new(ring)
        f.material_index = ROLES.index(role)
    return _object(bm, f"{p.name}.{what}", p.coll)


def pipe(p, what, path, r, role="trim", sides=6, caps=None):
    """A pipe, conduit or cable swept along a path of prop-space points: straight runs joined by
    mitred elbows, one closed solid with no boolean between its runs. r is the apothem; a flat
    faces down on a run that starts level (toward -z on one that starts vertical). Each ring after
    the first is the one before it carried along the run onto the next mitre plane (the bisector
    of the two runs; the last ring's plane is square to the run), so every side face is planar
    (its two long edges are parallel to the run) and an elbow costs nothing but its ring: a run
    is 2 x sides triangles, a cap sides - 2. role is one role or one per run."""
    n = sides
    pts = [Vector(q) for q in path]
    runs = len(pts) - 1
    roles = [role] * runs if isinstance(role, str) else list(role)
    caps = caps or (roles[0], roles[-1])
    d0 = (pts[1] - pts[0]).normalized()
    ref = Vector((0.0, 1.0, 0.0)) if abs(d0.y) < 0.9 else Vector((0.0, 0.0, 1.0))
    u = (ref - d0 * ref.dot(d0)).normalized()
    w = d0.cross(u)
    rr = r / math.cos(math.pi / n)
    ring = [pts[0] + rr * (math.sin(math.pi / n + 2 * math.pi * k / n) * w - math.cos(math.pi / n + 2 * math.pi * k / n) * u)
            for k in range(n)]
    rings = [ring]
    for i in range(1, runs + 1):
        d = (pts[i] - pts[i - 1]).normalized()
        m = d if i == runs else (d + (pts[i + 1] - pts[i]).normalized()).normalized()
        ring = [v + d * ((pts[i] - v).dot(m) / d.dot(m)) for v in ring]
        rings.append(ring)
    bm = bmesh.new()
    vs = [[bm.verts.new(v) for v in rg] for rg in rings]
    for i in range(runs):
        for k in range(n):
            j = (k + 1) % n
            f = bm.faces.new((vs[i][k], vs[i][j], vs[i + 1][j], vs[i + 1][k]))
            f.material_index = ROLES.index(roles[i])
    for rg, rl in ((vs[0], caps[0]), (vs[-1], caps[1])):
        f = bm.faces.new(rg)
        f.material_index = ROLES.index(rl)
    return _object(bm, f"{p.name}.{what}", p.coll)


def rod(p, what, axis, c, t0, t1, r, role="machinery", sides=6):
    """A straight pipe, strut or conduit: a revolve of two rings (see revolve for axis and c)."""
    return revolve(p, what, [(r, t0), (r, t1)], role, axis, c, sides)


def wheel(p, what, c, y, r, role="accent"):
    """A valve hand wheel seen from above, one hull: a flat hexagonal rim 3 cm thick at y over a
    spindle that tapers to a 4 cm square 10 cm below it, into the pipe or valve it stands on, on
    the vertical line c = (x, z)."""
    rr = r / math.cos(math.pi / 6)
    rim = [(c[0] + rr * math.sin(math.pi / 6 + k * math.pi / 3), yy, c[1] - rr * math.cos(math.pi / 6 + k * math.pi / 3))
           for yy in (y, y + 0.03) for k in range(6)]
    foot = [(c[0] + sx * 0.02, y - 0.10, c[1] + sz * 0.02) for sx in (-1, 1) for sz in (-1, 1)]
    return [p.hull(what, rim + foot, role)]


def vgroove(p, what, axis, at, lo, hi, face, w=0.03):
    """A V-groove cutter in a front face (normal +z, at z = face), running along axis 'x' or 'y'
    from lo to hi at `at` across it: 1.6 cm wide and 1.8 cm deep at the face. A V is four faces
    where a box groove is five (the suite's locker doors)."""
    if axis == "y":
        return p.prism(what, [(at - w, face + 0.05), (at + w, face + 0.05), (at, face - 0.6 * w)], "y", lo, hi, "machinery")
    return p.prism(what, [(face + 0.05, at - w), (face + 0.05, at + w), (face - 0.6 * w, at)], "x", lo, hi, "machinery")


def near(v, target, tol=0.005):
    return abs(v - target) < tol


# ----------------------------------------------------------------------------- power

def switchboard():
    """The main switchboard: three breaker cabinets in one carcass on a hazard-striped kick plate.
    The side cabinets each hold a breaker panel with two rows of four toggles (one tripped, in the
    page's colour) over a door; the centre one a status screen over two lit gauges with their
    needles and a door. The doors are the cabinets' lower fronts, split off by a V-groove. A lit
    chamfer runs along the top, and a cable trunking 2.4 m long sits on the wall above, fed by
    three conduits that rise from the cabinet tops and turn back into it."""
    p = prop("switchboard", "The main switchboard: breaker cabinets that route the reactor's and the battery's power to every bus (power grid)", WALL)
    W, D, top = 2.36, 0.72, 1.74
    body = p.prism("carcass", [(0.0, 0.0), (0.70, 0.0), (0.70, 0.15), (D, 0.15), (D, top - 0.04), (D - 0.04, top),
                               (0.0, top)], "x", -W / 2, W / 2,
                   ["machinery", "hazard", "machinery", "bulkhead", "light_panel", "machinery", "machinery"], cap="machinery")
    splits = [vgroove(p, f"split_{i}", "y", x, 0.17, top - 0.06, D) for i, x in enumerate((-0.40, 0.40))]
    splits.append(vgroove(p, "door_line", "x", 0.80, -W / 2 - 0.05, W / 2 + 0.05, D))
    p.cut(body, "cabinet_and_door_splits", splits)
    trunk = p.box("trunking", (-1.2, 1.84, 0.0), (1.2, 2.0, 0.42), {"+z": "trim", "+y": "trim", "*": "machinery"})
    p.chamfer(trunk, "trunking_edge", 0.02, lambda m, d, n1, n2: near(m.y, 2.0) and near(m.z, 0.42))
    conduits = [pipe(p, f"conduit_{i}", [(x, top - 0.02, 0.56), (x, 1.92, 0.56), (x, 1.92, 0.40)], 0.05, "trim", sides=4)
                for i, x in enumerate((-0.79, 0.0, 0.79))]
    p.union(body, "trunking", [trunk] + conduits)
    # Panels, doors, the screen and the dials (lit, their walls a trim bezel), let into the fronts.
    recesses = []
    for s in (-1, 1):
        p.recess(recesses, f"breaker_panel_{s:+d}", frame((s * 0.79, 1.22, D)), 0.62, 0.74, 0.03, floor_role="machinery",
                 record=False)
    p.recess(recesses, "status", frame((0.0, 1.43, D)), 0.62, 0.34, 0.015, shows="console")
    for i, x in enumerate((-0.17, 0.17)):
        recesses.append(revolve(p, f"gauge_{i}", [(0.10, D - 0.015), (0.10, D + 0.05)], "trim", "z", (x, 1.0), sides=6,
                                caps=("light_panel", "trim")))
    p.cut(body, "panels", recesses)
    # Toggles: two rows of four in each breaker panel (its floor at z = D - 0.03), three-sided so
    # each holes the floor with a triangle; door pulls; the gauges' needles.
    parts = []
    zf = D - 0.03
    for s in (-1, 1):
        for row, y in enumerate((1.04, 1.38)):
            for i in range(4):
                x = s * 0.79 + (i - 1.5) * 0.14
                tripped = (s, row, i) == (1, 0, 2) or (s, row, i) == (-1, 1, 1)
                lean = -0.035 if tripped else 0.035
                base = [(-0.028, -0.03), (0.028, -0.03), (0.0, 0.04)]    # the paddle's tip is this, shrunk and leant,
                parts.append(p.hull(f"toggle_{s:+d}_{row}_{i}",           # so every side is a planar trapezoid
                                    [(x + bx, y + by, zf - 0.01) for bx, by in base]
                                    + [(x + 0.8 * bx, y + lean + 0.8 * by, zf + 0.065) for bx, by in base],
                                    "accent" if tripped else "trim"))
    for i, x in enumerate((-0.79, 0.0, 0.79)):
        parts.append(p.box(f"door_pull_{i}", (x + 0.24, 0.40, D - 0.01), (x + 0.28, 0.66, D + 0.03), "trim"))
    for i, (x, a) in enumerate(((-0.17, 35.0), (0.17, -20.0))):
        m = Matrix.Translation((x, 1.0, D - 0.015)) @ Matrix.Rotation(math.radians(a), 4, "Z")
        parts.append(p.box(f"needle_{i}", (-0.008, -0.01, -0.01), (0.008, 0.08, 0.012), "accent", m=m))
    p.union(body, "toggles_and_pulls", parts)
    p.body = body
    p.operators.append([0.0, 0.0, 1.35])
    return p


def battery_bank():
    """The battery bank: a rack of nine cell modules in three columns and three tiers, each module
    set back in its bay with a lit status chamfer along its top, two copper bus bars across the
    front on the rails between the tiers, each ending in a terminal block, a hazard-striped plinth and a
    chamfered top."""
    p = prop("battery_bank", "The battery bank: racked cell modules that store the grid's reserve (power grid)", WALL)
    W, H, D = 2.0, 1.6, 1.12
    body = p.prism("rack", [(0.0, 0.0), (D - 0.03, 0.0), (D - 0.03, 0.13), (D, 0.13), (D, H - 0.03), (D - 0.03, H), (0.0, H)],
                   "x", -W / 2, W / 2, ["machinery", "hazard", "machinery", "trim", "trim", "machinery", "machinery"],
                   cap="machinery")
    cols = [(-0.95 + i * 0.64, -0.95 + i * 0.64 + 0.6) for i in range(3)]
    tiers = [(0.17, 0.58), (0.64, 1.05), (1.11, 1.54)]
    bays = [p.box(f"bay_{c}_{t}", (x0, y0, 0.12), (x1, y1, D + 0.05), {"-z": "machinery", "*": "machinery"})
            for t, (y0, y1) in enumerate(tiers) for c, (x0, x1) in enumerate(cols)]
    p.cut(body, "bays", bays)
    modules = []
    for t, (y0, y1) in enumerate(tiers):
        for c, (x0, x1) in enumerate(cols):
            m = p.box(f"module_{c}_{t}", (x0 + 0.03, y0 - 0.01, 0.30), (x1 - 0.03, y1 - 0.05, D - 0.05),
                      {"+z": "bulkhead", "*": "machinery"})
            p.chamfer(m, f"module_{c}_{t}_lamp", 0.03, lambda mm, d, n1, n2, y1=y1: near(mm.y, y1 - 0.05) and near(mm.z, D - 0.05),
                      role="light_panel")
            modules.append(m)
    p.union(body, "modules", modules)
    bars = []
    for i, (ya, yb) in enumerate(((0.585, 0.635), (1.055, 1.105))):
        bars.append(p.box(f"bus_bar_{i}", (-0.97, ya + 0.008, D - 0.01), (0.90, yb - 0.008, D + 0.035), "accent"))
        bars.append(p.box(f"terminal_{i}", (0.86, ya - 0.012, D - 0.01), (0.97, yb + 0.012, D + 0.065), "trim"))
    p.union(body, "bus_bars", bars)
    p.body = body
    p.operators.append([0.0, 0.0, 1.6])
    return p


# ----------------------------------------------------------------------------- thermal

def coolant_pumps():
    """Two coolant pumps on a hazard-edged skid: each a motor lying along X with a cooling sleeve
    and a pump casing at its inner end, on a saddle. The casings' discharge pipes rise and loop
    over between them, each suction pipe comes forward out of its casing and turns down into the
    floor, with a valve wheel (in the page's colour) on each. A starter cabinet with a lit edge
    stands at the back of the skid."""
    p = prop("coolant_pumps", "The reactor's coolant pumps: two pumps on a skid that drive the coolant loop (power grid, thermal)", FREE)
    skid = p.box("skid", (-0.98, 0.0, -0.78), (0.98, 0.14, 0.55), {"+y": "bulkhead", "*": "hazard"})
    p.chamfer(skid, "skid_edges", 0.02, lambda m, d, n1, n2: near(m.y, 0.14))
    cy = 0.47
    parts = []
    for s in (-1, 1):
        tag = "l" if s < 0 else "r"
        motor = revolve(p, f"motor_{tag}", [(0.235, s * 0.96), (0.235, s * 0.70), (0.265, s * 0.66), (0.265, s * 0.40)],
                        ["bulkhead", "trim", "machinery"], "x", (cy, 0.0), caps=("trim", "machinery"))
        casing = revolve(p, f"casing_{tag}", [(0.25, s * 0.11), (0.34, s * 0.17), (0.34, s * 0.42)],
                         ["machinery", "machinery"], "x", (cy, 0.0))
        saddle = p.box(f"saddle_{tag}", (min(s * 0.48, s * 0.84), 0.13, -0.15), (max(s * 0.48, s * 0.84), 0.25, 0.15), "machinery")
        suction = pipe(p, f"suction_{tag}", [(s * 0.27, cy, 0.20), (s * 0.27, cy, 0.72), (s * 0.27, 0.0, 0.72)], 0.07)
        parts += [motor, casing, saddle, suction] + wheel(p, f"wheel_{tag}", (s * 0.27, 0.40), cy + 0.13, 0.12)
    loop = pipe(p, "loop", [(-0.27, cy + 0.25, 0.0), (-0.27, 1.32, 0.0), (0.27, 1.32, 0.0), (0.27, cy + 0.25, 0.0)], 0.075)
    starter = p.box("starter", (-0.30, 0.13, -0.76), (0.30, 0.86, -0.50), {"+z": "bulkhead", "*": "machinery"})
    p.chamfer(starter, "starter_lamp", 0.03, lambda m, d, n1, n2: near(m.y, 0.86) and near(m.z, -0.50), role="light_panel")
    parts += [loop, starter]
    p.union(skid, "pumps_and_pipes", parts)
    p.body = skid
    p.operators.append([0.0, 0.0, 1.2])
    return p


def facet(centre, normal, along):
    """A recess frame on any face: local +Z the face's outward normal, local +Y the in-plane
    direction nearest `along` (a recess's height runs that way), local +X their cross (its width)."""
    n = Vector(normal).normalized()
    y = Vector(along)
    y = (y - n * y.dot(n)).normalized()
    x = y.cross(n)
    return Matrix(((x.x, y.x, n.x, centre[0]), (x.y, y.y, n.y, centre[1]), (x.z, y.z, n.z, centre[2]), (0.0, 0.0, 0.0, 1.0)))


def around(axis, c, r, a, t):
    """A point on a revolve's facet and its outward normal: apothem r, angle a (radians; a facet
    of an n-sided revolve faces a = 2 pi k / n, 0 being -z for axis y and the floor for axes z
    and x), at t along the axis (see revolve for axis and c)."""
    s, co = math.sin(a), math.cos(a)
    if axis == "y":
        return (c[0] + r * s, t, c[1] - r * co), (s, 0.0, -co)
    if axis == "z":
        return (c[0] + r * s, c[1] - r * co, t), (s, -co, 0.0)
    return (t, c[0] - r * co, c[1] + r * s), (0.0, -co, s)


def fan(p, what, c, z, r, blades, pitch_deg=20.0):
    """A fan's hub and blades standing on a recess floor at z (facing +z), centred at c = (x, y):
    a hexagonal hub and `blades` flat blades reaching to r, each 1 cm into the floor."""
    hub = rod(p, f"{what}_hub", "z", c, z - 0.01, z + 0.06, 0.06 + r * 0.08, "trim")
    parts = [hub]
    for k in range(blades):
        m = (Matrix.Translation((c[0], c[1], z)) @ Matrix.Rotation(math.radians(pitch_deg + 360.0 * k / blades), 4, "Z"))
        parts.append(p.box(f"{what}_blade_{k}", (-r * 0.17, r * 0.15, -0.01), (r * 0.17, r * 0.92, 0.03), "machinery", m=m))
    return parts


# ----------------------------------------------------------------------------- propulsion

def impulse_drive():
    """The impulse drive: an engine core 2.5 m across lying along Z on two pierced saddles over
    hazard-striped base plates. Its profile carries a nozzle bell at the back (-Z) glowing inside,
    three raised rings and a domed front; glowing vent slots run along the upper facets between
    the rings, and four conduits leave its flanks and drop into the floor."""
    p = prop("impulse_drive", "The impulse drive: the engine core on its cradle in the drive room (propulsion)", FREE)
    cy = 1.45
    prof = [(0.60, -1.86), (0.60, -2.0), (0.92, -2.0), (1.08, -1.62), (1.25, -1.62), (1.25, -1.40), (1.15, -1.40),
            (1.15, -0.30), (1.25, -0.30), (1.25, -0.05), (1.15, -0.05), (1.15, 1.10), (1.25, 1.10), (1.25, 1.35),
            (1.10, 1.50), (0.95, 1.50), (0.70, 1.85), (0.40, 2.0)]
    roles = ["machinery", "trim", "machinery", "trim", "trim", "trim", "bulkhead", "trim", "trim", "trim", "bulkhead",
             "trim", "trim", "trim", "machinery", "machinery", "machinery"]
    core = revolve(p, "core", prof, roles, "z", (0.0, cy), sides=12, caps=("light_panel", "trim"))
    parts = []
    for i, z in enumerate((-0.85, 0.65)):
        saddle = p.hull(f"saddle_{i}", [(sx * 1.42, 0.05, z + sz * 0.2) for sx in (-1, 1) for sz in (-1, 1)]
                        + [(sx * 0.95, 1.05, z + sz * 0.16) for sx in (-1, 1) for sz in (-1, 1)], "machinery")
        holes = [p.prism(f"saddle_{i}_hole_{s:+d}", [(s * 0.75, 0.16), (s * 1.18, 0.16), (s * 0.80, 0.48)], "z", z - 0.3, z + 0.3,
                         "machinery") for s in (-1, 1)]
        p.cut(saddle, f"saddle_{i}_holes", holes)
        plate = p.box(f"plate_{i}", (-1.5, 0.0, z - 0.27), (1.5, 0.08, z + 0.27), {"+y": "trim", "*": "hazard"})
        parts += [saddle, plate]
    for s in (-1, 1):
        for j, z in enumerate((-0.30, 0.20)):
            parts.append(pipe(p, f"conduit_{s:+d}_{j}", [(s * 0.95, 1.05, z), (s * 1.36, 1.05, z), (s * 1.36, 0.0, z)], 0.07, "trim"))
    p.union(core, "cradle_and_conduits", parts)
    vents = []
    for i, t in enumerate((-0.85, 0.525)):
        for k in (4, 6, 8):
            c, n = around("z", (0.0, cy), 1.15, 2 * math.pi * k / 12, t)
            vents.append(p.recess([], f"vent_{i}_{k}", facet(c, n, (0.0, 0.0, 1.0)), 0.30, 0.80, 0.04,
                                  floor_role="light_panel", record=False))
    p.cut(core, "vents", vents)
    p.body = core
    p.operators.append([1.9, 0.0, 0.0])
    return p


def inertial_dampers():
    """The inertial dampers: a twelve-sided gyro housing on a plinth with a hazard band, a raised
    ring round its lower half, a glowing band let in above it, a domed top with a cap, and two
    access panels on the sides a crew member works from."""
    p = prop("inertial_dampers", "The inertial dampers: the gyro housing that keeps the crew from feeling the ship's acceleration (dampers)", FREE)
    prof = [(0.70, 0.0), (0.70, 0.14), (0.62, 0.20), (0.56, 0.20), (0.56, 0.62), (0.61, 0.66), (0.61, 0.76), (0.56, 0.80),
            (0.56, 0.86), (0.50, 0.86), (0.50, 1.08), (0.56, 1.08), (0.56, 1.32), (0.42, 1.46), (0.28, 1.46), (0.28, 1.60)]
    roles = ["hazard", "trim", "machinery", "bulkhead", "trim", "trim", "trim", "bulkhead", "machinery", "light_panel",
             "machinery", "bulkhead", "machinery", "trim", "trim"]
    body = revolve(p, "housing", prof, roles, "y", (0.0, 0.0), sides=12, caps=("machinery", "trim"))
    panels = []
    for k, label in ((6, "front"), (3, "side")):
        c, n = around("y", (0.0, 0.0), 0.56, 2 * math.pi * k / 12, 0.41)
        panels.append(p.recess([], f"panel_{label}", facet(c, n, (0.0, 1.0, 0.0)), 0.20, 0.30, 0.012,
                               floor_role="bulkhead", record=False))
    p.cut(body, "access_panels", panels)
    p.body = body
    p.operators.append([0.0, 0.0, 1.2])
    return p


def shield_generator():
    """The shield generator: an emitter column rising from a hazard-ringed base with four glowing
    vents, through four stacked coils (their windings in the page's colour) that narrow upward
    with a glowing band between each, to a tapered emitter with a glowing tip."""
    p = prop("shield_generator", "The shield generator: the emitter column and its coils (shields)", FREE)
    prof = [(1.00, 0.0), (1.00, 0.16), (0.92, 0.26), (0.44, 0.26), (0.32, 0.36), (0.32, 0.50)]
    roles = ["hazard", "trim", "machinery", "trim", "machinery"]
    y = 0.50
    for i, r in enumerate((0.80, 0.74, 0.68, 0.62)):
        core = 0.36 - 0.02 * i
        prof += [(r, y), (r, y + 0.16), (core, y + 0.16), (core, y + 0.36)]
        roles += ["machinery", "accent", "machinery", "light_panel"]
        y += 0.36
    prof[-1] = (prof[-1][0], y - 0.20 + 0.12)      # the last glow is the column's top, 12 cm above the top coil
    roles[-1] = "trim"
    prof += [(0.16, 2.12), (0.07, 2.20)]
    roles += ["machinery", "light_panel"]
    body = revolve(p, "column", prof, roles, "y", (0.0, 0.0), sides=12, caps=("machinery", "light_panel"))
    vents = []
    for k in (0, 3, 6, 9):
        c, n = around("y", (0.0, 0.0), 1.00, 2 * math.pi * k / 12, 0.08)
        vents.append(p.recess([], f"vent_{k}", facet(c, n, (0.0, 1.0, 0.0)), 0.36, 0.07, 0.03, floor_role="light_panel",
                              record=False))
    p.cut(body, "vents", vents)
    p.body = body
    p.operators.append([0.0, 0.0, 1.6])
    return p


# ----------------------------------------------------------------------------- life support

def ls_tanks():
    """Life support's oxygen generator: three gas tanks on a hazard-edged plinth, each banded with
    a raised ring and capped with a valve, two tall ones against the wall joined by a header pipe
    that runs forward and down into a control box with its screen and a lit edge."""
    p = prop("ls_tanks", "Life support's oxygen generator: gas tanks, their valves and the control box (life support)", WALL)
    plinth = p.box("plinth", (-0.9, 0.0, 0.0), (0.9, 0.12, 1.56), {"+y": "bulkhead", "*": "hazard"})
    p.chamfer(plinth, "plinth_edge", 0.02, lambda m, d, n1, n2: near(m.y, 0.12) and near(m.z, 1.56))
    parts = []
    for name, x, z, r, h, band in (("tank_l", -0.45, 0.46, 0.36, 1.62, True), ("tank_r", 0.45, 0.46, 0.36, 1.62, True),
                                   ("tank_f", -0.45, 1.18, 0.28, 1.20, False)):
        prof = [(r, 0.11), (r, h * 0.55)]
        roles = ["bulkhead"]
        if band:
            prof += [(r + 0.025, h * 0.55), (r + 0.025, h * 0.55 + 0.08), (r, h * 0.55 + 0.08)]
            roles += ["trim", "trim", "trim"]
        prof += [(r, h), (r * 0.45, h + 0.16)]
        roles += ["bulkhead", "machinery"]
        parts.append(revolve(p, name, prof, roles, "y", (x, z), caps=("machinery", "trim")))
        parts.append(rod(p, f"{name}_valve", "y", (x, z), h + 0.10, h + 0.30, 0.06, "accent"))
    header = pipe(p, "header", [(-0.45, h_top := 1.84, 0.46), (-0.45, 1.97, 0.46), (0.45, 1.97, 0.46), (0.45, 1.97, 1.15),
                                (0.45, 1.20, 1.15)], 0.04, "trim")
    box = p.box("control_box", (0.10, 0.11, 0.95), (0.86, 1.26, 1.55), {"+z": "bulkhead", "*": "machinery"})
    p.chamfer(box, "control_lamp", 0.03, lambda m, d, n1, n2: near(m.y, 1.26) and near(m.z, 1.55), role="light_panel")
    p.union(plinth, "tanks_and_box", parts + [header, box])
    screens = []
    p.recess(screens, "screen", frame((0.48, 0.92, 1.55)), 0.52, 0.34, 0.015, shows="console")
    p.cut(plinth, "screens", screens)
    p.body = plinth
    p.operators.append([0.48, 0.0, 2.0])
    return p


def ls_scrubbers():
    """Life support's CO2 scrubbers: two cabinets in one carcass split by a V-groove, on a
    hazard-striped toe kick, under a lit chamfer and an exhaust duct to the ceiling line. Each has
    a big fan let into its upper front (a hub and three blades behind the opening) over two filter
    drawers with pulls."""
    p = prop("ls_scrubbers", "Life support's CO2 scrubbers: fan cabinets with filter drawers (life support)", WALL)
    D, H = 1.55, 1.78
    body = p.prism("cabinets", [(0.0, 0.0), (D - 0.05, 0.0), (D - 0.05, 0.10), (D, 0.12), (D, H - 0.04), (D - 0.04, H), (0.0, H)],
                   "x", -0.9, 0.9, ["machinery", "hazard", "machinery", "bulkhead", "light_panel", "machinery", "machinery"],
                   cap="machinery")
    p.cut(body, "split", [vgroove(p, "split", "y", 0.0, 0.14, H - 0.06, D)])
    duct = p.box("duct", (-0.55, H - 0.01, 0.10), (0.55, 2.0, 0.70), {"*": "trim"})
    p.chamfer(duct, "duct_edge", 0.02, lambda m, d, n1, n2: near(m.y, 2.0) and near(m.z, 0.70))
    p.union(body, "duct", [duct])
    cuts = []
    for s in (-1, 1):
        cuts.append(revolve(p, f"fan_{s:+d}", [(0.30, D - 0.08), (0.30, D + 0.05)], "trim", "z", (s * 0.45, 1.22),
                            caps=("machinery", "trim")))
        for i, y in enumerate((0.33, 0.62)):
            p.recess(cuts, f"drawer_{s:+d}_{i}", frame((s * 0.45, y, D)), 0.72, 0.24, 0.012, floor_role="bulkhead", record=False)
    p.cut(body, "fans_and_drawers", cuts)
    parts = []
    for s in (-1, 1):
        parts += fan(p, f"fan_{s:+d}", (s * 0.45, 1.22), D - 0.08, 0.27, 3)
        for i, y in enumerate((0.33, 0.62)):
            parts.append(p.box(f"pull_{s:+d}_{i}", (s * 0.45 - 0.13, y - 0.02, D - 0.022), (s * 0.45 + 0.13, y + 0.02, D + 0.03), "trim"))
    p.union(body, "fans_and_pulls", parts)
    p.body = body
    p.operators.append([0.0, 0.0, 2.1])
    return p


def ls_air_handler():
    """Life support's air handler: a duct box on a hazard-striped toe kick with a big twelve-sided
    fan let into its front behind a glowing ring, a key panel and louvres beside it, an access
    panel on its side, and two square ducts with flanges rising to the ceiling line."""
    p = prop("ls_air_handler", "Life support's air handler: a duct box with a big fan, its ducts rising to the ceiling (life support)", WALL)
    D, H = 1.55, 1.46
    body = p.prism("duct_box", [(0.0, 0.0), (D - 0.05, 0.0), (D - 0.05, 0.10), (D, 0.12), (D, H - 0.05), (D - 0.05, H), (0.0, H)],
                   "x", -0.9, 0.9, ["machinery", "hazard", "machinery", "bulkhead", "trim", "machinery", "machinery"], cap="machinery")
    ducts = []
    for i, x in enumerate((-0.48, 0.48)):
        ducts.append(p.box(f"duct_{i}", (x - 0.24, H - 0.01, 0.12), (x + 0.24, 2.0, 0.62), "trim"))
        ducts.append(p.box(f"flange_{i}", (x - 0.28, 1.70, 0.08), (x + 0.28, 1.76, 0.66), "machinery"))
    p.union(body, "ducts", ducts)
    cuts = [revolve(p, "fan", [(0.52, D - 0.10), (0.52, D + 0.05)], "light_panel", "z", (-0.30, 0.76), sides=12,
                    caps=("machinery", "light_panel"))]
    for i, y in enumerate((0.34, 0.48, 0.62)):
        cuts.append(p.prism(f"louvre_{i}", [(D + 0.05, y - 0.04), (D + 0.05, y + 0.04), (D - 0.035, y + 0.04)], "x", 0.36, 0.78,
                            "machinery"))
    p.recess(cuts, "keys", frame((0.57, 1.08, D)), 0.40, 0.26, 0.012, shows="keys")
    p.recess(cuts, "side_panel", frame((0.9, 0.74, 0.80), 0.0, 90.0), 0.90, 0.90, 0.012, floor_role="bulkhead", record=False)
    p.cut(body, "fan_louvres_panels", cuts)
    p.union(body, "fan", fan(p, "fan", (-0.30, 0.76), D - 0.10, 0.49, 5))
    p.body = body
    p.operators.append([0.0, 0.0, 2.1])
    return p


# ----------------------------------------------------------------------------- gravity, medical

def gravity_generator():
    """The gravity generator: a ring coil standing on edge in a saddle on an octagonal base with
    a hazard band and glowing vents. The ring's outer facets alternate windings (in the page's
    colour) with dark plates, its inner faces glow, four clamps grip it, and a conduit rises from
    the base into each side."""
    p = prop("gravity_generator", "The gravity generator: the ring coil that holds the ship's artificial gravity (gravity)", FREE)
    base = revolve(p, "base", [(0.80, 0.0), (0.80, 0.14), (0.72, 0.22), (0.46, 0.22), (0.40, 0.30)],
                   ["hazard", "trim", "machinery", "machinery"], "y", (0.0, 0.0), caps=("machinery", "machinery"))
    cy, ro, ri = 0.82, 0.58, 0.40
    rc = ro / math.cos(math.pi / 12)
    ring = p.prism("ring", [(cx, cz) for cx, cz in ngon(0.0, cy, rc, 12)], "z", -0.20, 0.20, ["accent", "machinery"] * 6, cap="trim")
    p.cut(ring, "bore", [revolve(p, "bore", [(ri, -0.3), (ri, 0.3)], "light_panel", "z", (0.0, cy), sides=12)])
    saddle = p.hull("saddle", [(sx * 0.42, 0.28, sz * 0.30) for sx in (-1, 1) for sz in (-1, 1)]
                    + [(sx * 0.30, 0.42, sz * 0.26) for sx in (-1, 1) for sz in (-1, 1)], "machinery")
    clamps = [p.box(f"clamp_{k}", (-0.07, 0.54, -0.24), (0.07, 0.63, 0.24), "trim",
                    m=Matrix.Translation((0.0, cy, 0.0)) @ Matrix.Rotation(math.radians(45.0 + 90.0 * k), 4, "Z")) for k in range(4)]
    conduits = [pipe(p, f"conduit_{s:+d}", [(s * 0.66, 0.20, 0.0), (s * 0.66, cy, 0.0), (s * 0.54, cy, 0.0)], 0.045, "trim")
                for s in (-1, 1)]
    p.union(base, "ring", [ring, saddle] + clamps + conduits)
    vents = []
    for k in (0, 2, 4, 6):
        c, n = around("y", (0.0, 0.0), 0.80, 2 * math.pi * k / 8, 0.07)
        vents.append(p.recess([], f"vent_{k}", facet(c, n, (0.0, 1.0, 0.0)), 0.34, 0.06, 0.025, floor_role="light_panel",
                              record=False))
    p.cut(base, "vents", vents)
    p.body = base
    p.operators.append([0.0, 0.0, 1.3])
    return p


def med_bed():
    """A medical bed lying along Z, its head at -Z: a chamfered frame on a tapered column over a
    base plate, a padded mattress and a pillow, a scanner arch across it lit on its inner faces,
    and a monitor on a post at its head facing the foot."""
    p = prop("med_bed", "A sick bay bed with its scanner arch and monitor (medical)", FREE)
    base = p.box("base", (-0.36, 0.0, -0.80), (0.36, 0.06, 0.72), "trim")
    p.chamfer(base, "base_edges", 0.02, lambda m, d, n1, n2: near(m.y, 0.06))
    column = p.hull("column", [(sx * 0.24, 0.05, sz) for sx in (-1, 1) for sz in (-0.56, 0.46)]
                    + [(sx * 0.32, 0.56, sz) for sx in (-1, 1) for sz in (-0.78, 0.68)], "machinery")
    bed = p.box("frame", (-0.45, 0.55, -1.04), (0.45, 0.68, 1.12), "trim")
    p.chamfer(bed, "frame_edges", 0.02, lambda m, d, n1, n2: near(m.y, 0.68) and abs(d.z) > 0.9)
    pad = p.box("pad", (-0.41, 0.67, -0.97), (0.41, 0.78, 1.08), "bulkhead")
    p.chamfer(pad, "pad_edges", 0.025, lambda m, d, n1, n2: near(m.y, 0.78))
    pillow = p.box("pillow", (-0.30, 0.77, -0.94), (0.30, 0.85, -0.64), "trim")
    arch = p.prism("arch", [(-0.50, 0.58), (-0.44, 0.58), (-0.44, 1.08), (0.44, 1.08), (0.44, 0.58), (0.50, 0.58), (0.50, 1.14),
                            (0.44, 1.20), (-0.44, 1.20), (-0.50, 1.14)], "z", 0.10, 0.34,
                   ["machinery", "light_panel", "light_panel", "light_panel", "machinery", "machinery", "trim", "machinery",
                    "trim", "machinery"], cap="machinery")
    post = p.box("post", (-0.03, 0.66, -1.03), (0.03, 0.95, -0.98), "machinery")
    monitor = p.box("monitor", (-0.28, 0.86, -1.05), (0.28, 1.20, -0.975), "machinery")
    p.union(bed, "bed", [base, column, pad, pillow, arch, post, monitor])
    screens = []
    p.recess(screens, "monitor", frame((0.0, 1.03, -0.975)), 0.48, 0.28, 0.012, shows="console")
    p.cut(bed, "screens", screens)
    p.body = bed
    p.operators.append([0.75, 0.0, 0.0])
    return p


# ----------------------------------------------------------------------------- weapons

def magazine_rack():
    """The magazine's missile racks, 5 m along X: two tiers of four missiles lying along Z, each
    tier in a wide cradle beam with a hazard face, the beams held between two end frames on
    hazard-striped feet, and a rail hoist over them, its rail on the end frames, with a trolley
    and a hook block in the page's colour."""
    p = prop("magazine_rack", "The magazine's missile racks: two tiers of missiles in cradles under a rail hoist (weapons, magazine)", FREE)
    feet = [p.box(f"foot_{s:+d}", (min(s * 2.33, s * 2.55), 0.0, -1.40), (max(s * 2.33, s * 2.55), 0.08, 1.40),
                  {"+y": "trim", "*": "hazard"}) for s in (-1, 1)]
    body = feet[0]
    parts = feet[1:]
    for s in (-1, 1):
        end = p.box(f"end_frame_{s:+d}", (min(s * 2.40, s * 2.50), 0.05, -0.34), (max(s * 2.40, s * 2.50), 1.47, 0.34), "machinery")
        p.cut(end, f"end_frame_{s:+d}_hole", [p.box(f"end_hole_{s:+d}", (min(s * 2.35, s * 2.55), 1.0, -0.22),
                                                    (max(s * 2.35, s * 2.55), 1.36, 0.22), "trim")])
        parts.append(end)
    for tier, top in enumerate((0.24, 0.80)):
        parts.append(p.box(f"cradle_{tier}", (-2.41, top - 0.10, -0.30), (2.41, top, 0.30), {"+z": "hazard", "-z": "hazard", "*": "machinery"}))
        for i, x in enumerate((-1.8, -0.6, 0.6, 1.8)):
            parts.append(revolve(p, f"missile_{tier}_{i}", [(0.19, -1.25), (0.19, 0.72), (0.19, 0.84), (0.06, 1.28)],
                                 ["bulkhead", "hazard", "trim"], "z", (x, top + 0.17), sides=6, caps=("machinery", "trim")))
    rail = p.box("rail", (-2.5, 1.46, -0.08), (2.5, 1.60, 0.08), {"+z": "hazard", "-z": "hazard", "*": "machinery"})
    trolley = p.box("trolley", (1.05, 1.36, -0.12), (1.35, 1.47, 0.12), "trim")
    block = p.box("hook_block", (1.14, 1.12, -0.06), (1.26, 1.37, 0.06), "accent")
    hook = p.box("hook", (1.16, 1.04, -0.03), (1.24, 1.13, 0.03), "trim")
    p.union(body, "rack_and_hoist", parts + [rail, trolley, block, hook])
    p.body = body
    p.operators.append([0.0, 0.0, 1.9])
    return p


def missile_tube():
    """A launch tube lying along Z, its axis 1.5 m above the floor, on two saddles over hazard base
    plates: banded along its length, with a wider hazard-striped breech block at the +Z end whose
    face holds the loading door (a recess with a handle bar and two hinge blocks) and a vent on
    top."""
    p = prop("missile_tube", "A missile launch tube, its breech toward the torpedo room (weapons)", FREE)
    cy = 1.5
    prof = [(0.27, -1.40), (0.27, -1.50), (0.36, -1.50), (0.36, -0.95), (0.40, -0.95), (0.40, -0.80), (0.36, -0.80),
            (0.36, 0.20), (0.40, 0.20), (0.40, 0.35), (0.36, 0.35), (0.36, 1.05), (0.45, 1.05), (0.45, 1.50)]
    roles = ["machinery", "trim", "bulkhead", "trim", "trim", "trim", "bulkhead", "trim", "trim", "trim", "bulkhead", "hazard",
             "hazard"]
    tube = revolve(p, "tube", prof, roles, "z", (0.0, cy), caps=("machinery", "trim"))
    parts = []
    for i, z in enumerate((-0.60, 0.62)):
        parts.append(p.hull(f"saddle_{i}", [(sx * 0.38, 0.05, z + sz * 0.14) for sx in (-1, 1) for sz in (-1, 1)]
                            + [(sx * 0.26, 1.25, z + sz * 0.10) for sx in (-1, 1) for sz in (-1, 1)], "machinery"))
        parts.append(p.box(f"plate_{i}", (-0.44, 0.0, z - 0.22), (0.44, 0.06, z + 0.22), {"+y": "trim", "*": "hazard"}))
    parts.append(rod(p, "vent", "y", (0.0, 1.28), 1.93, 2.0, 0.05, "trim"))
    p.union(tube, "saddles", parts)
    door = []
    p.recess(door, "door", frame((0.0, cy, 1.50)), 0.50, 0.50, 0.02, floor_role="bulkhead", record=False)
    p.cut(tube, "door", door)
    fittings = [p.box("handle", (-0.15, cy - 0.03, 1.47), (0.15, cy + 0.03, 1.54), "accent")]
    for i, y in enumerate((cy - 0.17, cy + 0.17)):
        fittings.append(p.box(f"hinge_{i}", (-0.30, y - 0.04, 1.47), (-0.22, y + 0.04, 1.53), "trim"))
    p.union(tube, "door_fittings", fittings)
    p.body = tube
    p.operators.append([0.0, 0.0, 2.0])
    return p


# ----------------------------------------------------------------------------- the reactor

def reactor_core():
    """The reactor, 10 m tall through three decks: a twelve-sided column 3.8 m across on a
    hazard-banded plinth 4.4 m across, with glowing window bands at 1.6, 5.1 and 8.6 m (the three
    decks' eye heights), raised rings between them and at the top, a domed cap, and four coolant
    pipes running up its sides in the rings' grip, clear of the column and the windows."""
    p = prop("reactor_core", "The reactor: the column that runs through three decks, seen at each deck's eye height (power grid)", FREE)
    body_r, ring_r, win_r = 1.90, 2.16, 1.82
    prof = [(2.20, 0.0), (2.20, 0.30), (2.10, 0.40), (body_r, 0.40)]
    roles = ["hazard", "trim", "trim"]
    for i, (eye, ring) in enumerate(((1.60, 3.30), (5.10, 6.80), (8.60, 9.50))):
        prof += [(body_r, eye - 0.32), (win_r, eye - 0.32), (win_r, eye + 0.32), (body_r, eye + 0.32),
                 (body_r, ring), (ring_r, ring), (ring_r, ring + 0.20)]
        roles += ["bulkhead", "machinery", "light_panel", "machinery", "bulkhead", "trim", "trim"]
        if i < 2:
            prof.append((body_r, ring + 0.20))
            roles.append("trim")
    prof += [(1.70, 9.90), (0.90, 9.90), (0.90, 10.0)]
    roles += ["machinery", "machinery", "trim"]
    body = revolve(p, "column", prof, roles, "y", (0.0, 0.0), sides=12, caps=("machinery", "trim"))
    pipes = []
    for k in (1, 4, 7, 10):
        c, _ = around("y", (0.0, 0.0), 2.04, 2 * math.pi * k / 12, 0.0)
        pipes.append(rod(p, f"coolant_{k}", "y", (c[0], c[2]), 0.25, 9.60, 0.10, "trim"))
    p.union(body, "coolant_pipes", pipes)
    p.body = body
    p.operators.append([0.0, 0.0, 2.8])
    return p


# ----------------------------------------------------------------------------- craft

def launch_cradle():
    """A fighter launch cradle on the floor: a hazard-striped plate with a dark deck inset in it,
    two rails along Z, four clamps beside the rails (each with a jaw stepped toward its rail, a
    hazard top and a lit outer edge) and two glowing guide strips along the deck."""
    p = prop("launch_cradle", "A fighter launch cradle: rails and clamps on a hazard-striped plate (shuttle bay, fighters)", FREE)
    plate = p.box("plate", (-1.6, 0.0, -3.2), (1.6, 0.10, 3.2), "hazard")
    p.chamfer(plate, "plate_edges", 0.02, lambda m, d, n1, n2: near(m.y, 0.10))
    deck = p.box("deck", (-1.34, 0.09, -2.94), (1.34, 0.13, 2.94), {"+y": "machinery", "*": "trim"})
    parts = [deck]
    for s in (-1, 1):
        x = s * 0.55
        parts.append(p.prism(f"rail_{s:+d}", [(x - 0.05, 0.12), (x + 0.05, 0.12), (x + 0.05, 0.20), (x + 0.09, 0.22), (x + 0.09, 0.27),
                                               (x - 0.09, 0.27), (x - 0.09, 0.22), (x - 0.05, 0.20)], "z", -2.9, 2.9, "trim"))
        for t in (-1, 1):
            z, xc = t * 1.7, s * 0.86
            clamp = p.box(f"clamp_{s:+d}_{t:+d}", (xc - 0.16, 0.12, z - 0.20), (xc + 0.16, 0.50, z + 0.20), {"+y": "hazard", "*": "machinery"})
            p.chamfer(clamp, f"clamp_{s:+d}_{t:+d}_lamp", 0.03,
                      lambda m, d, n1, n2, xc=xc, s=s: near(m.y, 0.50) and near(m.x, xc + s * 0.16), role="light_panel")
            p.cut(clamp, f"clamp_{s:+d}_{t:+d}_jaw", [p.box(f"jaw_{s:+d}_{t:+d}", (xc - s * 0.25, 0.34, z - 0.3), (xc - s * 0.04, 0.6, z + 0.3),
                                                            "trim")])
            parts.append(clamp)
    p.union(plate, "deck_rails_clamps", parts)
    strips = []
    for s in (-1, 1):
        p.recess(strips, f"strip_{s:+d}", frame((s * 1.15, 0.13, 0.0), 90.0), 0.08, 5.4, 0.012, floor_role="light_panel", record=False)
    p.cut(plate, "strips", strips)
    p.body = plate
    p.operators.append([2.2, 0.0, 0.0])
    return p


def swift_fighter():
    """The Swift fighter, nose toward +Z, gear down: a dart fuselage (one hull of four octagonal
    stations), a glowing canopy, swept wings with gun pods at their tips (in the page's colour), a
    raked fin, two engine nacelles whose nozzles glow, twin cannon under the nose and three gear
    legs on pads."""
    p = prop("swift_fighter", "The Swift: a single-seat fighter, gear down (shuttle bay, fighters)", CRAFT)

    def station(z, hw, y0, y1, c):
        return [(-hw + c, y0, z), (hw - c, y0, z), (hw, y0 + c, z), (hw, y1 - c, z), (hw - c, y1, z), (-hw + c, y1, z),
                (-hw, y1 - c, z), (-hw, y0 + c, z)]
    fus = p.hull("fuselage", station(3.5, 0.07, 0.80, 0.90, 0.02) + station(1.8, 0.42, 0.58, 1.16, 0.12)
                 + station(-1.6, 0.55, 0.56, 1.12, 0.14) + station(-3.1, 0.44, 0.62, 1.04, 0.10), "bulkhead")
    canopy = p.hull("canopy", [(sx * 0.28, 1.08, z) for sx in (-1, 1) for z in (0.40, 2.00)]
                    + [(sx * 0.15, 1.40, z) for sx in (-1, 1) for z in (0.80, 1.40)] + [(0.0, 1.12, 2.45)], "light_panel")
    parts = [canopy]
    for s in (-1, 1):
        parts.append(p.prism(f"wing_{s:+d}", [(s * 0.40, 1.0), (s * 2.22, -1.50), (s * 2.22, -2.20), (s * 0.40, -2.10)], "y", 0.72, 0.80,
                             ["trim", "machinery", "trim", "machinery"], cap="bulkhead"))
        parts.append(revolve(p, f"pod_{s:+d}", [(0.04, 0.0), (0.075, -0.20), (0.075, -2.40)], ["trim", "accent"], "z", (s * 2.24, 0.76),
                             sides=6, caps=("trim", "machinery")))
        parts.append(revolve(p, f"nacelle_{s:+d}", [(0.24, -1.0), (0.24, -3.10), (0.28, -3.22), (0.28, -3.50), (0.19, -3.50),
                                                    (0.19, -3.36)], ["machinery", "trim", "machinery", "trim", "machinery"], "z",
                             (s * 0.40, 0.80), caps=("machinery", "light_panel")))
        parts.append(rod(p, f"cannon_{s:+d}", "z", (s * 0.22, 0.66), 1.4, 2.9, 0.035, "trim"))
        parts.append(p.box(f"gear_{s:+d}", (s * 1.0 - 0.035, 0.02, -1.44), (s * 1.0 + 0.035, 0.74, -1.36), "machinery"))
        parts.append(p.box(f"pad_{s:+d}", (s * 1.0 - 0.08, 0.0, -1.52), (s * 1.0 + 0.08, 0.04, -1.28), "trim"))
    parts.append(p.prism("fin", [(-1.5, 1.05), (-2.9, 1.80), (-3.35, 1.80), (-3.05, 1.05)], "x", -0.04, 0.04, "trim"))
    parts.append(p.box("nose_gear", (-0.035, 0.02, 1.96), (0.035, 0.62, 2.04), "machinery"))
    parts.append(p.box("nose_pad", (-0.08, 0.0, 1.88), (0.08, 0.04, 2.12), "trim"))
    p.union(fus, "craft", parts)
    p.body = fus
    p.operators.append([1.0, 0.0, 1.5])
    return p


def petrel_shuttle():
    """The Petrel shuttle, nose toward +Z: an octagonal hull with a tapered nose and a glowing
    windscreen, three windows a side and a crew door, a lit hold behind its lowered rear ramp, two
    engine pods on pylons with glowing nozzles, a sensor dome and a light strip on the roof, and
    two landing skids on struts."""
    p = prop("petrel_shuttle", "The Petrel: a four-seat shuttle, its rear ramp down (shuttle bay)", CRAFT)
    sect = [(-1.15, 0.75), (1.15, 0.75), (1.45, 1.05), (1.45, 2.60), (1.05, 3.05), (-1.05, 3.05), (-1.45, 2.60), (-1.45, 1.05)]
    hull_ = p.prism("hull", sect, "z", -4.2, 2.6, ["machinery", "machinery", "bulkhead", "trim", "bulkhead", "trim", "bulkhead",
                                                    "machinery"], cap="bulkhead")
    nose = p.hull("nose", [(x, y, 2.55) for x, y in sect]
                  + [(sx * 0.85, 0.95, 4.75) for sx in (-1, 1)] + [(sx * 1.05, 1.15, 4.75) for sx in (-1, 1)]
                  + [(sx * 1.05, 1.75, 4.75) for sx in (-1, 1)] + [(sx * 0.75, 2.05, 4.75) for sx in (-1, 1)], "bulkhead")
    parts = [nose]
    for s in (-1, 1):
        parts.append(revolve(p, f"pod_{s:+d}", [(0.24, 0.95), (0.36, 0.75), (0.36, -3.60), (0.40, -3.70), (0.40, -4.40), (0.28, -4.40),
                                                (0.28, -4.25)], ["trim", "machinery", "trim", "machinery", "trim", "machinery"], "z",
                             (s * 1.86, 1.85), sides=12, caps=("machinery", "light_panel")))
        parts.append(p.box(f"pylon_{s:+d}", (min(s * 1.40, s * 1.62), 1.72, -3.4), (max(s * 1.40, s * 1.62), 1.98, 0.2), "trim"))
        x = s * 1.05
        parts.append(p.box(f"skid_{s:+d}", (x - 0.09, 0.0, -3.2), (x + 0.09, 0.12, 2.6), {"+y": "hazard", "*": "machinery"}))
        for z in (-2.4, 1.6):
            parts.append(p.box(f"strut_{s:+d}_{z:+.1f}", (x - 0.05, 0.10, z - 0.06), (x + 0.05, 0.80, z + 0.06), "machinery"))
    parts.append(revolve(p, "dome", [(0.30, 3.00), (0.30, 3.10), (0.14, 3.20)], ["trim", "machinery"], "y", (0.0, -0.6),
                         caps=("trim", "light_panel")))
    ramp = p.hull("ramp", [(sx * 1.0, y, z) for sx in (-1, 1) for y, z in ((0.94, -4.10), (0.86, -4.10), (0.06, -5.0), (0.0, -4.93))],
                  "trim")
    parts.append(ramp)
    p.union(hull_, "nose_pods_skids", parts)
    cuts = [p.box("hold", (-1.10, 0.95, -4.4), (1.10, 2.75, -2.6), {"+z": "light_panel", "-y": "machinery", "*": "bulkhead"})]
    slope = (0.0, 2.2, 1.0)
    p.recess(cuts, "windscreen", facet((0.0, 2.55, 3.65), slope, (0.0, 1.0, 0.0)), 1.40, 0.70, 0.03, floor_role="light_panel",
             record=False)
    for s in (-1, 1):
        for i, z in enumerate((1.7, 0.6, -0.5)):
            p.recess(cuts, f"window_{s:+d}_{i}", frame((s * 1.45, 2.10, z), 0.0, s * 90.0), 0.55, 0.42, 0.02,
                     floor_role="light_panel", record=False)
    p.recess(cuts, "door", frame((1.45, 1.80, -1.75), 0.0, 90.0), 0.90, 1.40, 0.015, floor_role="trim", record=False)
    p.recess(cuts, "roof_light", frame((0.0, 3.05, -1.5), 90.0), 0.10, 3.6, 0.012, floor_role="light_panel", record=False)
    p.cut(hull_, "hold_and_windows", cuts)
    p.body = hull_
    p.operators.append([2.6, 0.0, -1.75])
    return p


# ----------------------------------------------------------------------------- crew rooms

def bunk():
    """A two-tier crew bunk along the wall: a carcass on a toe kick with two berths carved out of
    it, each with a lit reading strip where its ceiling meets its back, a mattress, a blanket in
    the page's colour and a pillow; a lip along the upper berth and a two-rung ladder at the right
    end."""
    p = prop("bunk", "A two-tier crew bunk (crew quarters)", WALL)
    D = 0.90
    body = p.prism("carcass", [(0.0, 0.0), (D - 0.06, 0.0), (D - 0.06, 0.10), (D, 0.12), (D, 1.98), (D - 0.02, 2.0), (0.0, 2.0)],
                   "x", -1.05, 1.05, ["machinery", "machinery", "machinery", "trim", "trim", "trim", "machinery"], cap="trim")
    berths = []
    for i, (y0, y1) in enumerate(((0.36, 1.02), (1.18, 1.88))):
        b = p.box(f"berth_{i}", (-0.98, y0, 0.05), (0.98, y1, D + 0.05), {"-z": "bulkhead", "*": "trim"})
        p.chamfer(b, f"berth_{i}_light", 0.05, lambda m, d, n1, n2, y1=y1: near(m.y, y1) and near(m.z, 0.05), role="light_panel")
        berths.append(b)
    p.cut(body, "berths", berths)
    parts = []
    for i, y0 in enumerate((0.36, 1.18)):
        parts.append(p.box(f"mattress_{i}", (-0.99, y0 - 0.01, 0.04), (0.99, y0 + 0.14, 0.84), "trim"))
        parts.append(p.box(f"blanket_{i}", (-0.50, y0 - 0.01, 0.035), (0.99, y0 + 0.17, 0.855), "accent"))
        parts.append(p.box(f"pillow_{i}", (-0.95, y0 + 0.12, 0.12), (-0.62, y0 + 0.22, 0.62), "bulkhead"))
    parts.append(p.box("lip", (-0.99, 1.17, 0.83), (0.70, 1.32, 0.88), "trim"))
    for i, x in enumerate((0.76, 0.98)):
        parts.append(p.box(f"ladder_rail_{i}", (x - 0.02, 0.0, D - 0.01), (x + 0.02, 1.95, D + 0.05), "machinery"))
    for i, y in enumerate((0.62, 1.10)):
        parts.append(p.box(f"rung_{i}", (0.77, y - 0.015, D + 0.01), (0.97, y + 0.015, D + 0.04), "trim"))
    p.union(body, "bedding_and_ladder", parts)
    p.body = body
    p.operators.append([0.0, 0.0, 1.4])
    return p


def mess_table():
    """A mess table for six: a chamfered top 2.0 x 0.9 m on two tapered legs, a bench along each
    long side with its cushion in the page's colour, legs and benches standing on two floor
    rails that fix the set to the deck."""
    p = prop("mess_table", "A mess table with its two benches, fixed to the deck (mess)", FREE)
    top = p.box("top", (-1.0, 0.74, -0.45), (1.0, 0.80, 0.45), {"+y": "bulkhead", "*": "trim"})
    p.chamfer(top, "top_edges", 0.02, lambda m, d, n1, n2: near(m.y, 0.80))
    parts = []
    for s in (-1, 1):
        x = s * 0.66
        parts.append(p.box(f"rail_{s:+d}", (x - 0.06, 0.0, -0.92), (x + 0.06, 0.06, 0.92), "machinery"))
        parts.append(p.hull(f"leg_{s:+d}", [(x + sx * 0.05, 0.05, sz * 0.30) for sx in (-1, 1) for sz in (-1, 1)]
                            + [(x + sx * 0.05, 0.75, sz * 0.12) for sx in (-1, 1) for sz in (-1, 1)], "machinery"))
        for t in (-1, 1):
            parts.append(p.box(f"bench_leg_{s:+d}_{t:+d}", (x - 0.04, 0.05, t * 0.75 - 0.05), (x + 0.04, 0.41, t * 0.75 + 0.05), "machinery"))
    for t in (-1, 1):
        z0, z1 = sorted((t * 0.57, t * 0.95))
        seat = p.box(f"seat_{t:+d}", (-0.96, 0.40, z0), (0.96, 0.44, z1), "trim")
        cushion = p.box(f"cushion_{t:+d}", (-0.93, 0.43, z0 + 0.02), (0.93, 0.47, z1 - 0.02), "accent")
        p.chamfer(cushion, f"cushion_{t:+d}_edge", 0.015, lambda m, d, n1, n2, t=t: near(m.y, 0.47) and abs(d.x) > 0.9)
        parts += [seat, cushion]
    p.union(top, "legs_and_benches", parts)
    p.body = top
    p.operators += [[x, 0.0, t * 0.76] for t in (-1, 1) for x in (-0.6, 0.0, 0.6)]
    return p


def galley_counter():
    """A galley: a base cabinet with a lit toe kick and three doors (V-grooves), a counter with a
    chamfered front edge holding a cooktop (two glowing burners in a dark plate) and a sink with
    its tap, a backsplash, a drinks dispenser with a lit nozzle bay and a key panel, and upper
    cabinets with a light strip along their underside."""
    p = prop("galley_counter", "The galley's counter: cooktop, sink, dispenser and upper cabinets (mess)", WALL)
    W = 2.4
    cab = p.prism("cabinet", [(0.0, 0.0), (0.56, 0.0), (0.56, 0.10), (0.60, 0.12), (0.60, 0.86), (0.0, 0.86)], "x", -W / 2 + 0.02, W / 2 - 0.02,
                  ["machinery", "light_panel", "machinery", "bulkhead", "bulkhead", "machinery"], cap="bulkhead")
    p.cut(cab, "doors", [vgroove(p, f"door_split_{i}", "y", x, 0.14, 0.82, 0.60) for i, x in enumerate((-0.6, 0.0, 0.6))])
    top = p.box("top", (-W / 2, 0.85, 0.0), (W / 2, 0.91, 0.70), "trim")
    p.chamfer(top, "top_edge", 0.02, lambda m, d, n1, n2: near(m.y, 0.91) and near(m.z, 0.70))
    splash = p.box("backsplash", (-W / 2 + 0.02, 0.90, 0.0), (W / 2 - 0.02, 1.56, 0.03), "bulkhead")
    upper = p.box("upper", (-W / 2, 1.55, 0.0), (W / 2, 2.2, 0.38), {"+z": "bulkhead", "*": "machinery"})
    p.chamfer(upper, "under_light", 0.04, lambda m, d, n1, n2: near(m.y, 1.55) and near(m.z, 0.38), role="light_panel")
    plate = p.box("cooktop", (-0.95, 0.90, 0.10), (-0.25, 0.935, 0.58), "machinery")
    dispenser = p.box("dispenser", (0.62, 0.90, 0.02), (1.10, 1.50, 0.30), {"+z": "trim", "*": "machinery"})
    tap = p.hull("tap", [(0.15 + sx * 0.025, 0.90, z) for sx in (-1, 1) for z in (0.03, 0.09)]
                 + [(0.15 + sx * 0.02, 1.10, z) for sx in (-1, 1) for z in (0.03, 0.08)]
                 + [(0.15 + sx * 0.015, y, 0.24) for sx in (-1, 1) for y in (1.07, 1.10)], "trim")
    p.union(cab, "top_and_fittings", [top, splash, upper, plate, dispenser, tap])
    p.cut(cab, "sink", [p.hull("basin", [(0.15 + sx * 0.28, 0.96, 0.36 + sz * 0.20) for sx in (-1, 1) for sz in (-1, 1)]
                                + [(0.15 + sx * 0.20, 0.72, 0.36 + sz * 0.13) for sx in (-1, 1) for sz in (-1, 1)], "machinery")])
    cuts = [vgroove(p, f"upper_split_{i}", "y", x, 1.60, 2.15, 0.38) for i, x in enumerate((-0.6, 0.0, 0.6))]
    for i, x in enumerate((-0.78, -0.42)):
        cuts.append(revolve(p, f"burner_{i}", [(0.11, 0.925), (0.11, 1.0)], "machinery", "y", (x, 0.34), sides=6,
                            caps=("light_panel", "machinery")))
    p.recess(cuts, "nozzle_bay", frame((0.86, 1.08, 0.30)), 0.30, 0.22, 0.08, floor_role="light_panel", record=False)
    p.recess(cuts, "dispenser_keys", frame((0.86, 1.36, 0.30)), 0.34, 0.16, 0.012, shows="keys")
    p.cut(cab, "burners_and_dispenser", cuts)
    p.body = cab
    p.operators.append([-0.3, 0.0, 1.2])
    return p


PROPS = {
    "switchboard": switchboard,
    "battery_bank": battery_bank,
    "coolant_pumps": coolant_pumps,
    "impulse_drive": impulse_drive,
    "inertial_dampers": inertial_dampers,
    "shield_generator": shield_generator,
    "ls_tanks": ls_tanks,
    "ls_scrubbers": ls_scrubbers,
    "ls_air_handler": ls_air_handler,
    "gravity_generator": gravity_generator,
    "med_bed": med_bed,
    "magazine_rack": magazine_rack,
    "missile_tube": missile_tube,
    "reactor_core": reactor_core,
    "launch_cradle": launch_cradle,
    "swift_fighter": swift_fighter,
    "petrel_shuttle": petrel_shuttle,
    "bunk": bunk,
    "mess_table": mess_table,
    "galley_counter": galley_counter,
}


# ----------------------------------------------------------------------------- main

STATUS = ("Built (2026-10-06) by " + GENERATOR + ". Proposed machinery, craft and crew-room furniture for the Tern "
          "(data/ships/tern/layout.json systems and craft; deck-pipeline section 5), to replace the deck plan mockup's grey "
          "boxes; no page or engine code loads them yet.")
RULES = [
    "This file is written by " + GENERATOR + "; never edit it by hand. Rebuild, and the .glb files and this file change together.",
    "Metres. Prop space is the glTF frame: +Y up, +Z toward the user (the prop's front: the side it is worked from; a craft's nose), +X the user's right as they face the prop.",
    "anchor says where the origin is. 'floor, centre of the back, on the wall plane' (a WALL prop): on the floor at the centre of its back, the back flat on a wall at z = 0. 'floor, centre of the footprint' (a FREE prop): on the floor at the centre of its footprint. 'floor, centre of the footprint, nose toward +Z' (a CRAFT): the same, its nose toward +Z.",
    "Placing props: the layout's system or craft centre_m (on the floor) is the anchor for a FREE prop or a CRAFT; a WALL prop's anchor goes on the wall plane with its +Z into the room. operators_m are floor points where a crew member stands or sits to work it, in prop space.",
    "Materials are roles: machinery, trim, bulkhead, hazard and light_panel are data/materials/materials.json layers (light_panel is emissive: lamps, glowing bands, vents, nozzles and canopies); screen is emissive and coloured by the page; accent (blankets, valve wheels, bus bars, coil windings, tripped breakers) is tinted by the page.",
    "screens are the recess floors a page draws on: centre_m on the floor, normal out of it, up the in-plane direction of height_m (width_m runs along up x normal). shows says what: console (a main screen image, assets/textures/screens/screens.json), upper (half of an upper image) or keys (a key panel).",
    "UV0 is in metres, projected per face as shipkit.js worldUv does (x, z where |n.y| > 0.75, else the face's horizontal tangent and y); divide by the material's span_m.",
    "Flat shaded (one normal per face), triangulated, one closed manifold solid per prop; faces against the floor or the wall are kept for the deck compiler to drop.",
    "triangles is counted in the .glb; the build refuses a prop over budget_triangles. sha256 is the .glb's, from the Blender and exporter versions in generator: a second build with them writes the same bytes.",
]

MACHINERY = PropSet("machinery", "MachineryProps", OUT, GENERATOR, PROPS, BUDGETS, STATUS, RULES, __doc__)


if __name__ == "__main__":
    run(MACHINERY)
