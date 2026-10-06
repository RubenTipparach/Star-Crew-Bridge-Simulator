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
from hs_kit import ROLES, ROOT, Prop, PropSet, _object, frame, run  # noqa: E402

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


PROPS = {
    "switchboard": switchboard,
    "battery_bank": battery_bank,
    "coolant_pumps": coolant_pumps,
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
