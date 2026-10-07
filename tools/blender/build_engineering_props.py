"""Star Crew's engineering props, modelled in Blender the hard-surface CSG way: the engine core room's
plant, after the owner's brief of 2026-10-07 ("engine core room is kinda empty, need consiles, lots of
pipes, and heavy machinery in here"; "think about how a spaceship engine works, its heavy hot, needs
fuel, needs coolant"; "needs constant maintentance with tools and pipes to move resources around";
"tanks to store or buffer stuff, pumps to force fluids to move"). The coolant loop (heat exchangers,
the pressurizer, the drain tank, the reactor coolant pumps), the fuel train (deuterium dewars, the
helium-3 rack, the fuel processor and pellet injector), the magnets' cryoplant, the exhaust (vacuum
pumps, the ash tank), the power converters, the magnet dressing round the reactor column, an overhead
crane, the engineer's desk, local panels and the mimic board, and the maintenance kit (a bench, tool
chests, a parts rack) and two valves for the pipe runs. The shapes are taken from industrial and
accelerator references (a lagged pipe gallery, CERN's ALICE, ATLAS and CMS, a reactor pool), never
their art (CLAUDE.md 15).

It owns the engineering props' geometry (assets/models/engineering/<name>.glb), their atlases
(assets/models/engineering/atlas/<name>.png) and their manifest (assets/models/engineering/props.json).
It lives in tools/blender because meshes are files built by a committed generator (CLAUDE.md section
9): this script is the source, the .glb files are its output, and a second run writes the same bytes.
It holds only these props, their details and two primitives only they use so far (hoop, a closed
polygonal ring, and turned, a solid of revolution about any axis). Everything else is imported, never
copied (CLAUDE.md 6.1): the hard-surface kit (tools/blender/hs_kit.py: materials, primitives, the Prop
class and its CSG steps, clean, check, the atlas and its bake, the glb export and read-back, the
manifest and the command line) and the machinery set's primitives (tools/blender/
build_machinery_props.py: revolve, pipe, rod, wheel, fan, vgroove, facet, near), which move into the
kit when its owner moves them. How to work this way is the blender-hard-surface skill
(.claude/skills/blender-hard-surface).

Run (from anywhere):
  <python with the bpy module> tools/blender/build_engineering_props.py [--check] [--only a,b] [--blend out.blend]
  blender -b --factory-startup -P tools/blender/build_engineering_props.py -- [--check] [--only a,b] [--blend out.blend]
  --check   build in memory and compare with the committed .glb files, atlases and props.json; write nothing
  --only    build only these props (the manifest keeps the others' rows)
  --blend   also save the scene with every cutter collection, for looking at the CSG in Blender
            (a debugging view, not a source: it is not reproducible and is not committed)

Conventions (written into props.json too):
  * Metres. Prop space is the exported glTF frame: +Y up, +Z the prop's front (the side it is worked
    from), +X the right of someone facing it.
  * Anchors. A FREE prop has its origin on the floor at the centre of its footprint; a WALL prop on the
    floor at the centre of its back, the back flat on the wall plane z = 0 (the vacuum pump's wall is
    the reactor port it stands against). The control desk keeps free_console's anchor (the floor at
    the centre of the pedestal's back). Two props hang or sit in a pipe run and say so in their anchor:
    the crane (its hook's lowest point on y = 0, the rails' top at rail_top_m) and the valves (the
    floor under the pipe's centreline, the centreline at centreline_m). The kit's read-back requires
    every prop's lowest point on y = 0, so the brief's own origins for those three are recorded, not
    used.
  * Ports. Every prop a pipe meets records ports {name: {at_m, dir, dia_m}}: the centre of the flange
    face, the unit direction the pipe leaves in and the pipe's outside diameter. A flange is modelled at
    every port, its outer face exactly on at_m, so the pipe the page draws butts onto it; the build
    checks each port lies on a face of the mesh that faces dir.
  * One material per role, as every set: machinery, trim, bulkhead, hazard and light_panel are
    data/materials/materials.json layers, screen is emissive and coloured by the page, accent is tinted
    by the page (engineering's amber: valve wheels, actuators, guards). The atlas carries the colour:
    data/materials/prop_atlas.json bakes each role in its finish, and a prop's finish_of re-colours a
    role (lagging, magnet red, safety yellow, white cryogenic jackets, red tool chests).
  * UV0 in metres as shipkit.js worldUv; UV1 the prop's atlas; flat shaded; triangulated; one closed
    manifold solid per prop; a prop over its triangle budget (BUDGETS) is refused and nothing is written.
"""
import math
import os
import sys

import bpy  # first: with the pip bpy module, bmesh and mathutils exist only once bpy is imported
import bmesh  # noqa: E402
from mathutils import Matrix, Vector  # noqa: E402

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import hs_kit  # noqa: E402
from hs_kit import (P, PROP_TO_BLENDER, ROLES, ROOT, Prop, PropSet, _object, clip_polygon, frame,  # noqa: E402
                    in_frame, lathe, r3, run)
from build_machinery_props import facet, fan, near, pipe, revolve, rod, vgroove, wheel  # noqa: E402

OUT = os.path.join(ROOT, "assets", "models", "engineering")
GENERATOR = "tools/blender/build_engineering_props.py"

# Triangles per prop: the budgets these props were briefed with (2026-10-07).
BUDGETS = {
    "heat_exchanger": 900,
    "pressurizer": 600,
    "coolant_pump": 600,
    "coolant_tank": 500,
    "fuel_dewar": 600,
    "helium3_rack": 700,
    "fuel_processor": 800,
    "cryoplant": 900,
    "vacuum_pump": 300,
    "ash_tank": 400,
    "power_converter": 600,
    "reactor_dressing": 1800,
    "gantry_crane": 500,
    "local_panel": 350,
    "control_desk": 900,
    "mimic_board": 300,
    "tool_board": 600,
    "tool_chest": 160,
    "parts_rack": 500,
    "valve_large": 160,
    "valve_small": 90,
}

WALL = "floor, centre of the back, on the wall plane"
FREE = "floor, centre of the footprint"


def prop(name, presents, anchor):
    """An engineering prop: on a wall (its back at z = 0) when the anchor says so. It carries ports
    (the manifest's ports) and extra (further manifest fields: rail_top_m, centreline_m)."""
    p = Prop(name, presents, anchor, wall=(anchor == WALL))
    p.ports = {}
    p.flanges = []
    p.extra = {}
    return p


# ----------------------------------------------------------------------------- frames and primitives

def azim(a_deg):
    """The horizontal unit direction at azimuth a (degrees): 0 is +Z, 90 is +X."""
    a = math.radians(a_deg)
    return Vector((math.sin(a), 0.0, math.cos(a)))


def aim(d):
    """A rotation taking +Z to the direction d; about Y for a horizontal d (so a revolve's flat that
    faced the floor still does), about X for a vertical one."""
    d = Vector(d).normalized()
    if abs(d.y) > 0.999:
        return Matrix.Rotation(-math.pi / 2 if d.y > 0 else math.pi / 2, 4, "X")
    return Matrix.Rotation(math.atan2(d.x, d.z), 4, "Y")


def at(point, d=(0.0, 0.0, 1.0)):
    """A frame at a prop-space point whose local +Z is d."""
    return Matrix.Translation(Vector(point)) @ aim(d)


def place(ob, M):
    """Move an object made in prop space by the prop-space transform M."""
    ob.data.transform(PROP_TO_BLENDER @ M @ PROP_TO_BLENDER.inverted())
    ob.data.update()
    return ob


def turned(p, what, M, prof, roles="trim", sides=8, caps=None):
    """A solid of revolution about local +Z of frame M (revolve's profile of (r, t) points, r the
    apothem, t along local +Z): nozzles, flanges, knobs and drums on any axis."""
    return place(revolve(p, what, prof, roles, "z", (0.0, 0.0), sides, caps), M)


def hoop(p, what, M, rc, w, h, sides, role="trim", phase_deg=0.0):
    """A closed polygonal ring about local +Y of frame M: a section w wide (square to its runs) and h
    tall round a centreline polygon whose corners sit at radius rc, the first at azimuth phase_deg
    (0 is local +Z). Every face is planar: 8 x sides triangles (a revolve cannot leave a hole)."""
    n = sides
    k = (w / 2) / math.cos(math.pi / n)
    bm = bmesh.new()
    rings = []
    for i in range(n):
        u = azim(phase_deg + 360.0 * i / n)
        rings.append([bm.verts.new(M @ Vector((u.x * r, y, u.z * r)))
                      for r, y in ((rc - k, -h / 2), (rc + k, -h / 2), (rc + k, h / 2), (rc - k, h / 2))])
    for i in range(n):
        j = (i + 1) % n
        for a in range(4):
            b = (a + 1) % 4
            f = bm.faces.new((rings[i][a], rings[i][b], rings[j][b], rings[j][a]))
            f.material_index = ROLES.index(role)
    return _object(bm, f"{p.name}.{what}", p.coll)


def flange(p, name, point, d, dia, rim=None, t=None, role="trim", sides=8):
    """A port's flange: a disc of apothem dia / 2 + rim, t thick, its outer face exactly on point,
    facing d; the port is recorded for the manifest."""
    rim = rim if rim is not None else max(0.025, round(0.14 * dia, 4))
    t = t if t is not None else max(0.025, round(0.12 * dia, 4))
    M = at(point, d)
    p.ports[name] = {"at_m": r3(point), "dir": r3(Vector(d).normalized()), "dia_m": round(dia, 4)}
    p.flanges.append((name, M, dia / 2, dia / 2 + rim))
    return turned(p, f"{name}_flange", M, [(dia / 2 + rim, -t), (dia / 2 + rim, 0.0)], role, sides)


def port(p, name, point, d, dia, body=0.25, role="trim", body_role=None, sides=8, rim=None, t=None):
    """A nozzle of the pipe's diameter running `body` back from the port into what it leaves, and
    its flange (flange). Returns both pieces, for a union."""
    M = at(point, d)
    nozzle = turned(p, f"{name}_nozzle", M, [(dia / 2, -body), (dia / 2, -0.012)], body_role or role, sides)
    return [nozzle, flange(p, name, point, d, dia, rim, t, role, sides)]


def nozzle_port(p, name, point, d, dia, body, role="trim", body_role="bulkhead"):
    """port with a six-sided nozzle (apothem dia / 2) inside the eight-sided flange: where a nozzle meets a
    flange's back and a vessel's flat it costs fewer triangles than an eight-sided one, and reads the same."""
    M = at(point, d)
    nozzle = turned(p, f"{name}_nozzle", M, [(dia / 2, -body), (dia / 2, -0.012)], body_role, 6)
    return [nozzle, flange(p, name, point, d, dia, role=role)]


def lift(objs, dy):
    """Raise pieces by dy (the crane and the valves are designed about their own datum, then stood on
    the floor)."""
    for o in objs:
        place(o, Matrix.Translation((0.0, dy, 0.0)))
    return objs


def facets(n, r, c=(0.0, 0.0)):
    """The flats of an n-sided vertical revolve (revolve's default phase, n even): for each, its
    azimuth, a frame on it (local y is world y, local x across it) and its width."""
    out = []
    for k in range(n):
        a = 360.0 * k / n
        u = azim(a)
        m = facet((c[0] + r * u.x, 0.0, c[1] + r * u.z), tuple(u), (0.0, 1.0, 0.0))
        out.append((a, m, 2 * r * math.tan(math.pi / n)))
    return out


# ----------------------------------------------------------------------------- details for the atlas
# Helpers over hs_kit.Detail (the prop's detail version, baked onto its atlas): each takes D first.

def tidy(poly):
    out = []
    for q in poly:
        if not out or math.hypot(q[0] - out[-1][0], q[1] - out[-1][1]) > 1e-5:
            out.append(q)
    if len(out) > 1 and math.hypot(out[0][0] - out[-1][0], out[0][1] - out[-1][1]) <= 1e-5:
        out.pop()
    if len(out) < 3:
        return None
    a = sum(out[i][0] * out[(i + 1) % len(out)][1] - out[(i + 1) % len(out)][0] * out[i][1] for i in range(len(out))) / 2
    return out if abs(a) > 2e-6 else None


def regions(D, where):
    """The planar regions of the prop (hs_kit.Region) that where(R) accepts."""
    return [R for R in D.regions if where(R)]


def strip(D, R, axis, t0, t1, finish, reserve=True):
    """Paint the part of region R whose position along the prop-space axis lies in [t0, t1]: a band
    round a tank, a lagging lap, a fuel ring."""
    a = Vector(axis)
    base, ka, kb = a.dot(R.origin), a.dot(R.right), a.dot(R.up)
    xs, ys = [], []
    for tri in R.tris:
        poly = clip_polygon(list(tri), ka, kb, t1 - base)
        if len(poly) >= 3:
            poly = clip_polygon(poly, -ka, -kb, base - t0)
        poly = tidy(poly) if len(poly) >= 3 else None
        if poly:
            D.paint(R.m, poly, finish, reserve=False)
            xs += [q[0] for q in poly]
            ys += [q[1] for q in poly]
    if reserve and xs:
        D.reserve(R.m, min(xs), min(ys), max(xs), max(ys))


def whole(D, R, finish):
    """Paint all of region R (a part in another colour than its role's finish)."""
    for tri in R.tris:
        poly = tidy(list(tri))
        if poly:
            D.paint(R.m, poly, finish, reserve=False)
    D.reserve(R.m, *R.box)


def rect_in(D, R, x0, y0, x1, y1, finish):
    """Paint the rectangle x0..x1, y0..y1 of region R's own frame, clipped to R."""
    for tri in R.tris:
        poly = list(tri)
        for a, b, c in ((1, 0, x1), (-1, 0, -x0), (0, 1, y1), (0, -1, -y0)):
            if len(poly) >= 3:
                poly = clip_polygon(poly, a, b, c)
        poly = tidy(poly) if len(poly) >= 3 else None
        if poly:
            D.paint(R.m, poly, finish, reserve=False)


def stripes(D, R, finishes):
    """Paint region R in stripes along its long side, one per finish (a loom's coloured cables)."""
    x0, y0, x1, y1 = R.box
    n = len(finishes)
    for i, fin in enumerate(finishes):
        if x1 - x0 >= y1 - y0:
            rect_in(D, R, x0 - 1, y0 + (y1 - y0) * i / n, x1 + 1, y0 + (y1 - y0) * (i + 1) / n, fin)
        else:
            rect_in(D, R, x0 + (x1 - x0) * i / n, y0 - 1, x0 + (x1 - x0) * (i + 1) / n, y1 + 1, fin)
    D.reserve(R.m, *R.box)


def in_box(lo, hi):
    """A region test: its centroid inside the prop-space box lo..hi."""
    return lambda R: all(lo[i] <= R.origin[i] <= hi[i] for i in range(3))


def gauge(D, m, x, y, r, needle_deg=35.0, face="stencil"):
    """A round analogue gauge on frame m: a steel bezel, a white face, a red arc and a dark needle."""
    D.ring(m, x, y, r * 0.80, r, -0.002, 0.010, "bolt", sides=16)
    D.disc(m, x, y, r * 0.82, -0.002, 0.004, face, sides=16, reserve=False)
    arc = [(x + r * 0.70 * math.cos(math.radians(a)), y + r * 0.70 * math.sin(math.radians(a))) for a in (10, 40)]
    arc += [(x + r * 0.55 * math.cos(math.radians(a)), y + r * 0.55 * math.sin(math.radians(a))) for a in (40, 10)]
    D.prism(m, arc, -0.002, 0.0048, "red", reserve=False)
    mn = m @ Matrix.Translation((x, y, 0.0)) @ Matrix.Rotation(math.radians(-needle_deg), 4, "Z")
    D.box(mn, -0.0035, -r * 0.12, 0.0035, r * 0.68, 0.003, 0.0062, "stencil_dark", reserve=False)
    D.disc(m, x, y, r * 0.13, 0.003, 0.0075, "stencil_dark", sides=8, reserve=False)


def bolt_ring(D, m, r, n, size=0.012, x=0.0, y=0.0, phase=0.5):
    for k in range(n):
        a = 2 * math.pi * (k + phase) / n
        D.disc(m, x + r * math.cos(a), y + r * math.sin(a), size, -0.002, 0.008, "bolt", sides=8, inset=0.003, reserve=False)


def flange_bolts(D, p):
    """Bolt heads round every port's flange face, between the pipe and the flange's rim."""
    for _name, M, r_in, r_out in p.flanges:
        n = 8 if r_out > 0.12 else 4
        bolt_ring(D, M, (r_in + r_out) / 2 + 0.003, n, size=min(0.016, (r_out - r_in) * 0.3))


def diamond(D, m, x, y, s, fill="red", label="2"):
    """A hazard diamond placard: a white border, the class colour and its number."""
    D.prism(m, [(x, y - s), (x + s, y), (x, y + s), (x - s, y)], -0.002, 0.003, "stencil")
    k = s - 0.014
    D.prism(m, [(x, y - k), (x + k, y), (x, y + k), (x - k, y)], -0.002, 0.0045, fill, reserve=False)
    D.text(m, label, x, y - s * 0.42, s * 0.36, "stencil", z=0.0058)


def placard(D, m, x, y, w, h, text, plate="yellow", ink="stencil_dark"):
    """A painted warning placard with its text."""
    D.box(m, x - w / 2, y - h / 2, x + w / 2, y + h / 2, -0.002, 0.004, plate, inset=0.002)
    D.text(m, text, x, y, min(h * 0.55, 0.85 * w / (0.62 * max(1, len(text)))), ink, z=0.0056)


def label(D, m, x, y, text, size, ink="stencil_dark"):
    D.text(m, text, x, y, size, ink)


def hazard_band(D, m, x0, y0, x1, y1):
    D.box(m, x0, y0, x1, y1, -0.002, 0.003, "hazard")


def lagging(D, n, r, y0, y1, bands, laps, skip=(), c=(0.0, 0.0), every=2):
    """Lagging on a vertical vessel's flats (n of them, apothem r, from y0 to y1): raised steel clamp
    bands with a buckle on every third flat, dark sheet laps between them, a vertical lap with screws
    on every other flat. skip lists (azimuth, y_lo, y_hi) where something stands on a flat; nothing is
    drawn there. Each flat is reserved, so the kit's rules add nothing of their own to it."""
    for k, (a, m, w) in enumerate(facets(n, r, c)):
        def clear(ya, yb, a=a):
            return not any(abs((a - s[0] + 180.0) % 360.0 - 180.0) < 1.0 and ya < s[2] and yb > s[1] for s in skip)
        x0, x1 = -w / 2 + 0.004, w / 2 - 0.004
        for y in bands:
            if clear(y - 0.04, y + 0.04):
                D.box(m, x0, y - 0.022, x1, y + 0.022, -0.002, 0.008, "steel", inset=0.003, reserve=False)
                if k % 3 == 0:
                    D.box(m, -0.035, y - 0.034, 0.035, y + 0.034, 0.0, 0.016, "steel", inset=0.005, reserve=False)
        for y in laps:
            if clear(y - 0.01, y + 0.01):
                D.paint(m, [(x0, y - 0.004), (x1, y - 0.004), (x1, y + 0.004), (x0, y + 0.004)], "seam", reserve=False)
        if k % every == 0:
            xv = w / 2 - 0.035
            ys = sorted([y0 + 0.03, y1 - 0.03] + [b for b in bands])
            for ya, yb in zip(ys[:-1], ys[1:]):
                ya, yb = ya + 0.04, yb - 0.04
                if yb - ya > 0.1 and clear(ya, yb):
                    D.paint(m, [(xv - 0.004, ya), (xv + 0.004, ya), (xv + 0.004, yb), (xv - 0.004, yb)], "seam", reserve=False)
                    for j in range(1, int((yb - ya) / 0.3) + 1):
                        D.disc(m, xv - 0.016, ya + (yb - ya) * j / (int((yb - ya) / 0.3) + 1), 0.005, -0.002, 0.005,
                               "bolt", sides=6, reserve=False)
        D.reserve(m, -w / 2, y0, w / 2, y1)


def plain(D, where):
    """Reserve whole regions, so the kit's generic seams and bolts leave them as their finish."""
    for R in regions(D, where):
        D.reserve(R.m, *R.box)


# ----------------------------------------------------------------------------- the coolant loop

def heat_exchanger():
    """The coolant loop's heat exchanger, after a reactor pool's steam generators: a vertical vessel
    1.5 m across in silver lagging on a skirt with four gussets and a hazard-striped base ring, a
    domed top with a lifting lug, out of which a gooseneck 0.45 m across rises, crests near 4.65 m and
    comes down to its flange (hot_in, toward +Z). The cold leg leaves the lower shell at the back
    (cold_out), the secondary loop to the hull radiators the sides (sec_in low on -X, sec_out high on
    +X). Two bolted manways on the front, instrument taps with gauges front right, and a caged ladder
    on the front left (the -X side) from the floor to the dome, kept inside the 1.9 m square."""
    p = prop("heat_exchanger", "A coolant loop heat exchanger: hot coolant in at the gooseneck, cold out at the back, "
                               "the secondary loop to the hull radiators at the sides (thermal)", FREE)
    p.finish_of = {"bulkhead": "lagging"}
    base = p.box("base_ring", (-0.92, 0.0, -0.92), (0.92, 0.12, 0.92), {"+y": "machinery", "*": "hazard"})
    parts = [revolve(p, "skirt", [(0.42, 0.0), (0.42, 0.50)], "machinery", "y", (0.0, 0.0), sides=12),
             revolve(p, "vessel", [(0.40, 0.45), (0.75, 0.68), (0.75, 3.40), (0.20, 3.86)], "bulkhead", "y", (0.0, 0.0), sides=12,
                     caps=("machinery", "machinery"))]
    for k, a in enumerate((0, 90, 180, 270)):
        m = frame((0.0, 0.0, 0.0), 0.0, a)
        parts.append(p.hull(f"gusset_{k}", in_frame(m, [(s, y, r) for s in (-0.04, 0.04) for r, y in
                                                        ((0.36, 0.10), (0.86, 0.10), (0.36, 0.48))]),
                            "machinery"))
    parts.append(pipe(p, "gooseneck", [(0.0, 3.80, 0.0), (0.0, 4.20, 0.0), (0.0, 4.40, 0.20), (0.0, 4.25, 0.55), (0.0, 4.25, 1.02)],
                      0.225, "bulkhead", sides=8))
    parts.append(flange(p, "hot_in", (0.0, 4.25, 1.05), (0, 0, 1), 0.45))
    parts += nozzle_port(p, "cold_out", (0.0, 0.75, -0.95), (0, 0, -1), 0.45, 0.60)
    parts += nozzle_port(p, "sec_out", (0.95, 3.2, 0.0), (1, 0, 0), 0.30, 0.40)
    parts += nozzle_port(p, "sec_in", (-0.95, 1.6, 0.0), (-1, 0, 0), 0.30, 0.40)
    for i, y in enumerate((1.25, 2.75)):
        parts.append(rod(p, f"manway_{i}", "z", (0.0, y), 0.60, 0.86, 0.26, "machinery", sides=6))
    parts.append(p.box("lifting_lug", (-0.03, 3.50, -0.66), (0.03, 3.72, -0.40), "machinery"))
    # the caged ladder, in a frame on the front-left diagonal (local z out from the axis)
    ml = frame((0.0, 0.0, 0.0), 0.0, 315.0)
    for s in (-1, 1):
        parts.append(p.box(f"rail_{s:+d}", (s * 0.19 - 0.02, 0.05, 0.84), (s * 0.19 + 0.02, 3.62, 0.88), "trim", m=ml))
        parts.append(p.box(f"standoff_{s:+d}", (s * 0.19 - 0.018, 3.50, 0.62), (s * 0.19 + 0.018, 3.56, 0.86), "trim", m=ml))
    for i in range(6):
        y = 0.45 + 0.52 * i
        parts.append(p.hull(f"rung_{i}", in_frame(ml, [(x, yy, z) for x in (-0.18, 0.18)
                                                      for yy, z in ((y - 0.016, 0.850), (y + 0.016, 0.850), (y, 0.874))]), "trim"))
    for i, y in enumerate((2.40, 3.40)):
        parts.append(pipe(p, f"cage_hoop_{i}", in_frame(ml, [(-0.19, y, 0.86), (-0.24, y, 1.03), (0.24, y, 1.03), (0.19, y, 0.86)]),
                          0.016, "trim", sides=3))
    p.union(base, "vessel_nozzles_ladder", parts)
    p.body = base
    p.operators.append([0.0, 0.0, 1.35])

    def decor(D):
        skip = [(0, 0.95, 1.55), (0, 2.45, 3.05), (90, 2.95, 3.45), (270, 1.35, 1.85), (180, 0.45, 1.05), (60, 1.88, 2.12)]
        lagging(D, 12, 0.75, 0.68, 3.40, bands=(0.95, 2.20, 3.22), laps=(1.50, 2.62), skip=skip)
        f30 = facets(12, 0.75)[1][1]
        label(D, f30, 0.0, 2.42, "HX-1", 0.085)
        label(D, f30, 0.0, 2.30, "PRIMARY", 0.032)
        f0 = facets(12, 0.75)[0][1]
        for i, y in enumerate((1.25, 2.75)):
            mc = at((0.0, y, 0.86))
            bolt_ring(D, mc, 0.20, 10, size=0.013)
            D.box(mc, -0.10, -0.03, 0.10, 0.03, -0.002, 0.012, "steel", inset=0.006)      # the cover's lifting handle
            D.text(mc, "MANWAY" if i == 0 else "MANWAY 2", 0.0, -0.13, 0.03, "stencil_dark")
        label(D, f0, 0.0, 2.10, "HOT LEG ABOVE", 0.028)
        mg = facets(12, 0.75)[2][1]
        D.box(mg, -0.07, 1.93, 0.07, 2.07, -0.002, 0.012, "steel", inset=0.004)
        gauge(D, mg @ Matrix.Translation((0.0, 0.0, 0.012)), 0.0, 2.0, 0.055, needle_deg=40)
        flange_bolts(D, p)
        for sx in (-1, 1):
            for sz in (-1, 1):
                D.disc(frame((sx * 0.82, 0.12, sz * 0.82), 90.0), 0.0, 0.0, 0.022, -0.002, 0.012, "bolt", sides=6, inset=0.004)
        mtop = frame((0.0, 0.12, 0.0), 90.0)
        D.box(mtop, -0.88, 0.80, 0.88, 0.88, -0.002, 0.003, "dark", reserve=False)
    p.decor.append(decor)
    return p


def pressurizer():
    """The loop's pressure buffer: a tall narrow vessel 1.0 m across in lagging, hemispherical heads,
    on a skirt with an opening each side showing the heater stubs that hang from the bottom head. Two
    relief valves on top with tailpipes turned out, a spray line nozzle at the front of the top head, a
    level gauge column standing off the front right, and the surge line out of the skirt's front."""
    p = prop("pressurizer", "The coolant loop's pressurizer: a steam bubble over the coolant holds the loop's pressure "
                            "(thermal)", FREE)
    p.finish_of = {"bulkhead": "lagging"}
    skirt = revolve(p, "skirt", [(0.40, 0.0), (0.40, 0.82)], "machinery", "y", (0.0, 0.0), sides=12)
    p.cut(skirt, "heater_openings", [p.box(f"opening_{s:+d}", (min(s * 0.22, s * 0.50), 0.10, -0.15),
                                           (max(s * 0.22, s * 0.50), 0.50, 0.15), "machinery") for s in (-1, 1)])
    parts = [rod(p, f"heater_{s:+d}_{i}", "y", (s * 0.31, z), 0.24, 0.52, 0.016, "trim", sides=4)
             for s in (-1, 1) for i, z in enumerate((-0.06, 0.06))]
    parts.append(revolve(p, "vessel", [(0.25, 0.64), (0.50, 0.90), (0.50, 3.62), (0.25, 3.88)], "bulkhead", "y", (0.0, 0.0), sides=12))
    for s in (-1, 1):
        x = s * 0.17
        parts.append(rod(p, f"relief_riser_{s:+d}", "y", (x, -0.06), 3.80, 3.98, 0.04, "trim", sides=4))
        parts.append(p.box(f"relief_body_{s:+d}", (x - 0.065, 3.96, -0.12), (x + 0.065, 4.12, 0.0), "accent"))
        parts.append(p.hull(f"relief_bonnet_{s:+d}", [(x + a * 0.04, 4.11, -0.06 + b * 0.04) for a in (-1, 1) for b in (-1, 1)]
                            + [(x + a * 0.022, 4.22, -0.06 + b * 0.022) for a in (-1, 1) for b in (-1, 1)], "accent"))
        parts.append(pipe(p, f"relief_tail_{s:+d}", [(x + s * 0.03, 4.04, -0.06), (x + s * 0.20, 4.04, -0.06),
                                                     (x + s * 0.20, 4.20, -0.06)], 0.03, "trim", sides=4))
    parts.append(rod(p, "spray_nozzle", "y", (0.0, 0.24), 3.80, 3.98, 0.05, "trim", sides=6))
    parts.append(revolve(p, "spray_flange", [(0.085, 3.955), (0.085, 4.00)], "trim", "y", (0.0, 0.24), sides=6))
    mg = frame((0.0, 0.0, 0.0), 0.0, 45.0)
    parts.append(pipe(p, "level_column", in_frame(mg, [(0.0, 1.10, 0.40), (0.0, 1.10, 0.64), (0.0, 3.30, 0.64), (0.0, 3.30, 0.40)]),
                      0.03, "trim", sides=4))
    parts += port(p, "surge", (0.0, 0.40, 0.60), (0, 0, 1), 0.20, body=0.45)
    p.union(skirt, "vessel_valves_lines", parts)
    p.body = skirt
    p.operators.append([0.0, 0.0, 1.0])

    def decor(D):
        skip = [(30, 1.04, 1.16), (60, 1.04, 1.16), (30, 3.24, 3.36), (60, 3.24, 3.36)]
        lagging(D, 12, 0.50, 0.90, 3.62, bands=(1.45, 2.40, 3.10), laps=(1.95, 2.80), skip=skip)
        f0 = facets(12, 0.50)[0][1]
        label(D, f0, 0.0, 2.10, "PZR", 0.09)
        label(D, f0, 0.0, 1.98, "LOOP 1", 0.03)
        for R in regions(D, lambda R: R.role == "machinery" and 0.2 < abs(R.origin.x) < 0.39 and 0.05 < R.origin.y < 0.55
                         and abs(R.n.y) < 0.5 and abs(R.origin.z) < 0.2):
            whole(D, R, "dark")
        for R in regions(D, lambda R: R.role == "trim" and 0.2 < abs(R.origin.x) < 0.45 and R.origin.y < 0.55):
            whole(D, R, "copper")
        for a, m, w in facets(12, 0.40):
            hazard_band(D, m, -w / 2 + 0.004, 0.56, w / 2 - 0.004, 0.68)
        flange_bolts(D, p)
        mg_ = frame((0.0, 0.0, 0.0), 0.0, 45.0)
        for y in (1.6, 2.2, 2.8):
            D.box(mg_ @ Matrix.Translation((0.0, 0.0, 0.67)), -0.018, y - 0.004, 0.018, y + 0.004, -0.002, 0.003, "stencil", reserve=False)
    p.decor.append(decor)
    return p


def coolant_pump():
    """A reactor coolant pump for 0.45 m pipe, after a PWR's main coolant pump: a pump casing 1.0 m
    across at the bottom with two horizontal flanged nozzles (suction from the back, discharge to the
    front) on a base plate with hazard edges, an open motor stand of four posts round a coupling guard,
    a tall vertical motor 0.9 m across with a top fan cowl and lifting lugs, a junction box and an oil
    level sight glass on its side."""
    p = prop("coolant_pump", "A reactor coolant pump: drives the coolant loop through the reactor and its heat exchanger "
                             "(thermal)", FREE)
    plate = p.box("base_plate", (-0.80, 0.0, -0.62), (0.80, 0.10, 0.62), {"+y": "machinery", "*": "hazard"})
    p.chamfer(plate, "plate_edges", 0.015, lambda m, d, n1, n2: near(m.y, 0.10))
    parts = [revolve(p, "casing", [(0.40, 0.08), (0.50, 0.22), (0.50, 0.92), (0.30, 1.08)], "machinery", "y", (0.0, 0.0),
                     sides=10, caps=("machinery", "trim"))]
    parts += port(p, "suction", (0.0, 0.70, -0.80), (0, 0, -1), 0.45, body=0.55)
    parts += port(p, "discharge", (0.0, 0.70, 0.80), (0, 0, 1), 0.45, body=0.55)
    for k, s_ in enumerate((-1, 1)):
        parts.append(p.box(f"stand_post_{k}", (s_ * 0.27 - 0.05, 1.02, -0.07), (s_ * 0.27 + 0.05, 1.52, 0.07), "trim"))
    parts.append(revolve(p, "coupling_guard", [(0.13, 1.05), (0.13, 1.52)], "accent", "y", (0.0, 0.0), sides=6))
    parts.append(revolve(p, "motor", [(0.45, 1.50), (0.45, 2.40), (0.38, 2.46), (0.38, 2.60), (0.22, 2.72)],
                         ["bulkhead", "trim", "machinery", "machinery"], "y", (0.0, 0.0), sides=12, caps=("trim", "trim")))
    for s in (-1, 1):
        parts.append(p.box(f"lifting_lug_{s:+d}", (s * 0.24 - 0.02, 2.58, -0.06), (s * 0.24 + 0.02, 2.75, 0.06), "trim"))
    parts.append(p.box("junction_box", (0.42, 1.80, -0.16), (0.58, 2.12, 0.16), {"+x": "bulkhead", "*": "machinery"}))
    parts.append(pipe(p, "conduit", [(0.50, 1.82, 0.0), (0.66, 1.62, 0.0), (0.66, 0.08, 0.0)], 0.03, "trim", sides=4))
    p.union(plate, "casing_stand_motor", parts)
    p.body = plate
    p.operators.append([1.20, 0.0, 0.0])

    def decor(D):
        for a, m, w in facets(12, 0.45):
            for y in (1.70, 1.84, 1.98, 2.12, 2.26):
                if 80 < a < 100 and 1.75 < y < 2.15 or 260 < a < 280 and y < 1.9:
                    continue
                D.box(m, -w / 2 + 0.006, y - 0.018, w / 2 - 0.006, y + 0.018, -0.002, 0.014, "bolt", inset=0.006, reserve=False)
            D.reserve(m, -w / 2, 1.56, w / 2, 2.40)
        mf = facets(12, 0.45)[0][1]
        D.plate(mf, 0.0, 2.33, 0.20, 0.06, "RCP-1", size=0.032)
        for a, m, w in facets(10, 0.50):
            if a in (0.0, 180.0):
                continue
            hazard_band(D, m, -w / 2 + 0.004, 0.30, w / 2 - 0.004, 0.36)
        mc = facets(10, 0.50)[2][1]
        label(D, mc, 0.0, 0.62, "COOLANT", 0.045)
        label(D, mc, 0.0, 0.54, "PUMP 1", 0.045)
        mj = frame((0.58, 1.96, 0.0), 0.0, 90.0)
        placard(D, mj, 0.0, 0.06, 0.24, 0.07, "6.6 kV")
        D.leds(mj, -0.06, -0.06, 3, 0.06, ("led_green", "led_amber", "led_white"), r=0.01)
        mo = frame((-0.45, 1.74, 0.0), 0.0, -90.0)
        D.box(mo, -0.03, -0.09, 0.03, 0.09, -0.002, 0.004, "glass")
        D.box(mo, -0.026, -0.085, 0.026, 0.0, -0.002, 0.005, "led_amber", reserve=False)
        D.box(mo, -0.04, -0.002, 0.04, 0.002, 0.0, 0.006, "red", reserve=False)
        D.grille(frame((0.0, 2.72, 0.0), 90.0), -0.15, -0.15, 0.15, 0.15, pitch=0.03)
        flange_bolts(D, p)
        for sx in (-1, 1):
            for sz in (-1, 1):
                D.disc(frame((sx * 0.72, 0.10, sz * 0.54), 90.0), 0.0, 0.0, 0.022, -0.002, 0.012, "bolt", sides=6, inset=0.004)
    p.decor.append(decor)
    return p


def coolant_tank():
    """The coolant drain and makeup tank: a buffer lying along X, 3.0 m long and 1.3 m across with
    dished heads, on two saddles with base plates; a bolted manway on top, a vent with a gooseneck, a
    sight-glass level gauge on the front, three grab rungs up its front-left, and the outlet leaving
    the bottom to the front through a hand valve."""
    p = prop("coolant_tank", "The coolant drain and makeup tank: buffers the loop's coolant (thermal)", FREE)
    cy = 0.95
    tank = revolve(p, "tank", [(0.38, -1.50), (0.65, -1.30), (0.65, 1.30), (0.38, 1.50)], "bulkhead", "x", (cy, 0.0), sides=12,
                   caps=("machinery", "machinery"))
    parts = []
    for i, x in enumerate((-0.95, 0.95)):
        parts.append(p.box(f"saddle_{i}", (x - 0.10, 0.03, -0.48), (x + 0.10, 0.55, 0.48), "machinery"))
        parts.append(p.box(f"saddle_plate_{i}", (x - 0.16, 0.0, -0.56), (x + 0.16, 0.05, 0.56), {"+y": "trim", "*": "hazard"}))
    parts.append(rod(p, "manway", "y", (-0.55, 0.0), 1.45, 1.68, 0.27, "machinery", sides=8))
    parts.append(pipe(p, "vent", [(0.75, 1.50, 0.0), (0.75, 1.80, 0.0), (0.92, 1.80, 0.0), (0.92, 1.70, 0.0)], 0.04, "trim", sides=4))
    parts.append(pipe(p, "sight_glass", [(0.45, 0.62, 0.40), (0.45, 0.62, 0.745), (0.45, 1.30, 0.745), (0.45, 1.30, 0.40)],
                      0.025, "trim", sides=4))
    for i, y in enumerate((0.85, 1.30)):
        zin = math.sqrt(max(0.0, 0.65 ** 2 - (y - cy) ** 2)) - 0.06
        parts.append(pipe(p, f"rung_{i}", [(-1.10, y, zin), (-1.10, y, 0.80), (-0.78, y, 0.80), (-0.78, y, zin)], 0.014, "trim", sides=4))
    parts.append(p.box("sump", (0.90, 0.20, -0.12), (1.10, 0.42, 0.12), "machinery"))
    parts += port(p, "outlet", (1.0, 0.35, 0.60), (0, 0, 1), 0.15, body=0.55, sides=6)
    parts.append(p.box("outlet_valve", (0.92, 0.27, 0.30), (1.08, 0.45, 0.44), "machinery"))
    parts += wheel(p, "outlet_wheel", (1.0, 0.37), 0.52, 0.09)
    p.union(tank, "saddles_and_fittings", parts)
    p.body = tank
    p.operators.append([0.45, 0.0, 1.25])

    def decor(D):
        mfront = facet((0.0, cy, 0.65), (0.0, 0.0, 1.0), (0.0, 1.0, 0.0))
        label(D, mfront, -0.10, 0.06, "COOLANT DRAIN", 0.07)
        label(D, mfront, -0.10, -0.05, "TK-2   6 m3", 0.04)
        bolt_ring(D, at((-0.55, 1.68, 0.0), (0, 1, 0)), 0.235, 10, size=0.013)
        for R in regions(D, lambda R: R.role == "bulkhead" and R.n.dot(Vector((1, 0, 0))) ** 2 < 0.01):
            for x in (-1.10, 1.10):
                strip(D, R, (1, 0, 0), x - 0.025, x + 0.025, "steel", reserve=False)
            for x in (-0.40, 0.55):
                strip(D, R, (1, 0, 0), x - 0.004, x + 0.004, "seam", reserve=False)
        for R in regions(D, in_box((0.40, 0.70, 0.70), (0.50, 1.25, 0.80))):
            if R.n.z > 0.9:
                rect_in(D, R, -1, -1, 1, 1, "glass")
        msg = facet((0.45, 0.95, 0.77), (0.0, 0.0, 1.0), (0.0, 1.0, 0.0))
        D.box(msg, -0.012, -0.30, 0.012, 0.06, 0.0005, 0.004, "display", reserve=False)
        for y in (-0.25, -0.1, 0.05, 0.20):
            D.box(msg, 0.026, y - 0.003, 0.05, y + 0.003, -0.002, 0.003, "stencil", reserve=False)
        flange_bolts(D, p)
    p.decor.append(decor)
    return p


# ----------------------------------------------------------------------------- the fuel train

def fuel_dewar():
    """A deuterium dewar: a vertical vacuum-jacketed tank 1.2 m across in white with dished heads and an
    orange fuel band, on four short legs; its plumbing tree on top (a riser, a header with two valves
    and their hand wheels, a pressure gauge and a frosted vent stack) inside a guard rail on four
    posts; the fuel line leaving the bottom of the shell to the front."""
    p = prop("fuel_dewar", "A deuterium dewar: stores the reactor's deuterium fuel as a cryogenic liquid (fuel)", FREE)
    p.finish_of = {"bulkhead": "white"}
    tank = revolve(p, "tank", [(0.35, 0.24), (0.60, 0.42), (0.60, 2.08), (0.35, 2.26)], "bulkhead", "y", (0.0, 0.0), sides=10,
                   caps=("machinery", "machinery"))
    parts = []
    for k in range(3):
        u = azim(60.0 + 120.0 * k)
        c = (0.40 * u.x, 0.40 * u.z)
        parts.append(p.box(f"leg_{k}", (c[0] - 0.05, 0.0, c[1] - 0.05), (c[0] + 0.05, 0.40, c[1] + 0.05), "machinery"))
    parts.append(pipe(p, "header", [(-0.28, 2.18, 0.0), (-0.28, 2.42, 0.0), (0.28, 2.42, 0.0), (0.28, 2.18, 0.0)], 0.045, "trim", sides=6))
    for s in (-1, 1):
        parts += wheel(p, f"wheel_{s:+d}", (s * 0.15, 0.0), 2.53, 0.075)
    parts.append(p.box("gauge", (-0.05, 2.38, 0.04), (0.05, 2.48, 0.09), "trim"))
    parts.append(rod(p, "vent_stack", "y", (0.0, -0.18), 2.20, 2.72, 0.04, "bulkhead", sides=4))
    parts.append(hoop(p, "guard_rail", Matrix.Translation((0.0, 2.55, 0.0)), 0.50, 0.035, 0.035, 6, "accent", phase_deg=30.0))
    for k in range(3):
        u = azim(30.0 + 120.0 * k)
        parts.append(p.box(f"rail_post_{k}", (0.50 * u.x - 0.016, 2.04, 0.50 * u.z - 0.016),
                           (0.50 * u.x + 0.016, 2.56, 0.50 * u.z + 0.016), "trim"))
    parts += port(p, "fuel_out", (0.0, 0.45, 0.65), (0, 0, 1), 0.10, body=0.35, sides=6)
    p.union(tank, "legs_tree_rail", parts)
    p.body = tank
    p.operators.append([0.0, 0.0, 1.1])

    def decor(D):
        for a, m, w in facets(10, 0.60):
            D.box(m, -w / 2 + 0.004, 1.52, w / 2 - 0.004, 1.76, -0.002, 0.004, "fuel_orange", reserve=False)
            D.box(m, -w / 2 + 0.004, 1.49, w / 2 - 0.004, 1.51, -0.002, 0.003, "stencil_dark", reserve=False)
            D.box(m, -w / 2 + 0.004, 1.77, w / 2 - 0.004, 1.79, -0.002, 0.003, "stencil_dark", reserve=False)
            if a in (0.0, 144.0, 216.0):
                D.text(m, "D2", 0.0, 1.64, 0.13, "stencil_dark", z=0.006)
            if a in (72.0, 180.0, 288.0):
                diamond(D, m, 0.0, 1.20, 0.085, "red", "2")
            D.reserve(m, -w / 2, 0.42, w / 2, 2.08)
        m0 = facets(10, 0.60)[0][1]
        label(D, m0, 0.0, 1.08, "DEUTERIUM", 0.036)
        label(D, m0, 0.0, 1.02, "CRYOGENIC   -250 C", 0.022)
        D.plate(m0, 0.0, 0.80, 0.24, 0.07, "DW-1", size=0.035)
        gauge(D, at((0.0, 2.43, 0.09)), 0.0, 0.0, 0.042, needle_deg=-25)
        for R in regions(D, in_box((-0.08, 2.40, -0.26), (0.08, 2.80, -0.10))):
            whole(D, R, "frost")
        for R in regions(D, lambda R: R.role == "trim" and R.origin.y > 2.20 and abs(R.origin.x) > 0.22):
            strip(D, R, (0, 1, 0), 2.20, 2.32, "frost", reserve=False)
        flange_bolts(D, p)
    p.decor.append(decor)
    return p


def helium3_rack():
    """The helium-3 store: eight gas cylinders in two rows of four in a steel frame (a back plate, a
    base, two uprights and two restraint bars across the front row), each cylinder's neck rising into
    its row's header; the headers run into the regulator cabinet at the right end, which carries two
    gauges on its front and sends the fuel line out of its right side."""
    p = prop("helium3_rack", "The helium-3 store: racked gas cylinders feeding the fuel processor through a regulator (fuel)", WALL)
    back = p.box("back_plate", (-1.20, 0.0, 0.0), (1.20, 2.0, 0.04), "machinery")
    parts = [p.box("upright_l", (-1.20, 0.0, 0.02), (-1.14, 1.99, 0.70), "trim")]
    xs = (-0.98, -0.66, -0.34, -0.02)
    for row, z in enumerate((0.20, 0.52)):
        for i, x in enumerate(xs):
            parts.append(revolve(p, f"cylinder_{row}_{i}", [(0.14, 0.0), (0.14, 1.46), (0.02, 1.62), (0.02, 1.80)],
                                 ["bulkhead", "bulkhead", "trim"], "y", (x, z), sides=6))
        parts.append(rod(p, f"header_{row}", "x", (1.80, z), -1.15, 0.45, 0.045, "trim", sides=6))
    parts.append(p.box("restraint", (-1.15, 1.17, 0.665), (0.40, 1.23, 0.70), "accent"))
    parts.append(p.box("regulator", (0.39, 0.0, 0.03), (1.10, 1.95, 0.68), {"+z": "bulkhead", "*": "machinery"}))
    parts += port(p, "fuel_out", (1.15, 1.85, 0.50), (1, 0, 0), 0.08, body=0.10, sides=6)
    p.union(back, "frame_cylinders_regulator", parts)
    p.body = back
    p.operators.append([0.75, 0.0, 1.3])

    def decor(D):
        for row, z in enumerate((0.20, 0.52)):
            for i, x in enumerate(xs):
                for a, m, w in facets(6, 0.14, (x, z)):
                    D.box(m, -w / 2 + 0.004, 1.22, w / 2 - 0.004, 1.40, -0.002, 0.004, "fuel_orange", reserve=False)
                    if a == 0.0:
                        D.text(m, "He3", 0.0, 0.95, 0.045, "stencil_dark")
                        D.text(m, f"{row * 4 + i + 1:02d}", 0.0, 0.85, 0.035, "stencil_dark")
                    D.reserve(m, -w / 2, 0.0, w / 2, 1.46)
        mreg = at((0.745, 1.525, 0.68))
        gauge(D, at((0.58, 1.68, 0.68)), 0.0, 0.0, 0.065, needle_deg=10)
        gauge(D, at((0.92, 1.68, 0.68)), 0.0, 0.0, 0.065, needle_deg=-40)
        label(D, mreg, -0.17, 0.0, "SUPPLY", 0.026)
        label(D, mreg, 0.17, 0.0, "LINE", 0.026)
        D.plate(mreg, 0.0, -0.20, 0.42, 0.08, "HELIUM-3 REGULATOR", size=0.026)
        D.leds(mreg, -0.10, -0.32, 3, 0.10, ("led_green", "led_amber", "led_green"), r=0.011)
        D.grille(mreg, -0.26, -1.30, 0.26, -0.80, pitch=0.035)
        hazard_band(D, mreg, -0.35, -1.52, 0.35, -1.42)
        mb = at((0.0, 0.0, 0.70))
        for y in (1.20,):
            for x in (-0.95, -0.40, 0.15):
                D.disc(mb, x, y, 0.012, -0.002, 0.008, "bolt", sides=8, reserve=False)
        mback = at((0.0, 0.0, 0.04))
        placard(D, mback, -0.40, 1.92, 0.70, 0.08, "NO NAKED FLAME   FUEL STORE")
        flange_bolts(D, p)
    p.decor.append(decor)
    return p


def fuel_processor():
    """The fuel processing and pellet injector skid: a hazard-edged base frame carrying the pellet
    freezer cryostat (0.6 m across, white jacket) on the left, the gas manifold behind it (the
    deuterium and helium-3 lines in from the back, each through a hand valve), a feed pump, a control
    box with its screen and keys on the right, and the injector line from the cryostat through the
    pellet gun to the front."""
    p = prop("fuel_processor", "The fuel processing and pellet injector skid: freezes fuel into pellets and fires them into "
                               "the reactor (fuel)", FREE)
    skid = p.box("skid", (-1.20, 0.0, -0.65), (1.20, 0.14, 0.65), {"+y": "machinery", "*": "hazard"})
    p.chamfer(skid, "skid_edges", 0.015, lambda m, d, n1, n2: near(m.y, 0.14))
    parts = [revolve(p, "cryostat", [(0.30, 0.10), (0.30, 1.62), (0.24, 1.78), (0.12, 1.84)], ["bulkhead", "bulkhead", "trim"], "y",
                     (-0.60, 0.05), sides=12, caps=("bulkhead", "trim")),
             rod(p, "turret", "y", (-0.60, 0.05), 1.80, 1.90, 0.06, "trim", sides=6)]
    parts += port(p, "d2_in", (-0.60, 0.80, -0.65), (0, 0, -1), 0.10, body=0.55, sides=6)
    parts += port(p, "he3_in", (0.0, 0.80, -0.65), (0, 0, -1), 0.08, body=0.10, sides=6)
    parts.append(pipe(p, "he3_line", [(0.0, 0.80, -0.56), (0.0, 0.80, -0.10), (-0.40, 0.80, -0.10)], 0.04, "trim", sides=6))
    for name, x, z in (("d2", -0.60, -0.48), ("he3", 0.0, -0.40)):
        parts.append(p.box(f"{name}_valve", (x - 0.06, 0.72, z - 0.05), (x + 0.06, 0.90, z + 0.05), "machinery"))
        parts += wheel(p, f"{name}_wheel", (x, z), 0.98, 0.07)
    parts.append(revolve(p, "pump_motor", [(0.15, 0.10), (0.15, 0.62)], "bulkhead", "x", (0.40, -0.30), sides=8))
    parts.append(p.box("pump_head", (0.04, 0.13, -0.46), (0.16, 0.58, -0.14), "machinery"))
    parts.append(p.box("pump_foot", (0.40, 0.13, -0.38), (0.56, 0.28, -0.22), "machinery"))
    parts.append(pipe(p, "pump_line", [(0.10, 0.50, -0.30), (0.10, 0.62, -0.30), (-0.38, 0.62, -0.10)], 0.035, "trim", sides=6))
    box = p.box("control_box", (0.75, 0.13, -0.05), (1.15, 1.45, 0.45), {"+z": "bulkhead", "*": "machinery"})
    p.chamfer(box, "control_lamp", 0.025, lambda m, d, n1, n2: near(m.y, 1.45) and near(m.z, 0.45), role="light_panel")
    parts.append(box)
    parts.append(pipe(p, "injector_line", [(-0.40, 1.20, 0.05), (0.60, 1.20, 0.05), (0.60, 1.20, 0.64)], 0.05, "trim", sides=6))
    parts += [flange(p, "inject_out", (0.60, 1.20, 0.65), (0, 0, 1), 0.10)]
    parts.append(p.box("pellet_gun", (0.52, 1.10, 0.18), (0.68, 1.31, 0.46), {"+y": "bulkhead", "*": "machinery"}))
    parts.append(rod(p, "gun_post", "y", (0.60, 0.32), 0.12, 1.12, 0.035, "trim", sides=6))
    p.union(skid, "cryostat_manifold_pump_box", parts)
    screens = []
    p.recess(screens, "screen", frame((0.95, 1.16, 0.45)), 0.30, 0.20, 0.015, shows="console")
    p.cut(skid, "screens", screens)
    p.body = skid
    p.operators.append([0.95, 0.0, 1.15])

    def decor(D):
        mbox = at((0.95, 0.0, 0.45))
        D.buttons(mbox, -0.14, 0.92, 5, 0.06, 0.04, ("key", "led_green", "key", "led_amber", "key"))
        D.buttons(mbox, -0.14, 0.85, 5, 0.06, 0.04, ("key", "key", "led_white", "key", "key"))
        D.plate(mbox, 0.0, 0.70, 0.30, 0.07, "FUEL PROC", size=0.032)
        placard(D, mbox, 0.0, 0.45, 0.30, 0.10, "CRYOGENIC")
        D.grille(mbox, -0.15, 0.20, 0.15, 0.34, pitch=0.03)
        for a, m, w in facets(12, 0.30, (-0.60, 0.05)):
            D.box(m, -w / 2 + 0.003, 1.30, w / 2 - 0.003, 1.42, -0.002, 0.004, "fuel_orange", reserve=False)
            D.reserve(m, -w / 2, 0.14, w / 2, 1.62)
        m0 = facets(12, 0.30, (-0.60, 0.05))[0][1]
        label(D, m0, 0.0, 1.05, "PELLET", 0.04)
        label(D, m0, 0.0, 0.98, "FREEZER", 0.04)
        diamond(D, m0, 0.0, 0.72, 0.07, "red", "2")
        mg = frame((0.60, 1.205, 0.32), 90.0)
        placard(D, mg, 0.0, 0.0, 0.14, 0.18, "INJ")
        for R in regions(D, lambda R: R.role == "trim" and -0.50 < R.origin.x < 0.70 and 1.10 < R.origin.y < 1.30 and R.origin.z < 0.15):
            strip(D, R, (1, 0, 0), -0.35, -0.25, "frost", reserve=False)
        flange_bolts(D, p)
    p.decor.append(decor)
    return p


def cryoplant():
    """The magnets' cryoplant on one hazard-edged base: on the left a compressor skid (an electric motor
    lying along X behind a fan guard, a coupling, a screw compressor block and a horizontal oil
    separator on saddles behind them, piped to the cold box); on the right the tall insulated cold box
    (1.4 x 2.7 x 1.4, panelled) with three white vacuum-jacketed pipes on its face, gauges, and the
    liquid helium supply and return leaving its front high up."""
    p = prop("cryoplant", "The magnets' cryoplant: compresses and expands helium to cool the reactor's magnets (cryogenics)", FREE)
    p.finish_of = {"bulkhead": "white", "machinery": "panel"}
    base = p.box("base", (-1.70, 0.0, -0.80), (1.70, 0.15, 0.80), {"+y": "trim", "*": "hazard"})
    p.chamfer(base, "base_edges", 0.015, lambda m, d, n1, n2: near(m.y, 0.15))
    cold = p.box("cold_box", (0.30, 0.10, -0.70), (1.70, 2.75, 0.70), "machinery")
    p.chamfer(cold, "cold_box_top", 0.03, lambda m, d, n1, n2: near(m.y, 2.75))
    parts = [cold]
    for i, x in enumerate((0.42, 0.85, 1.38)):
        top = 2.12 - 0.12 * i
        parts.append(pipe(p, f"vj_pipe_{i}", [(x, top, 0.62), (x, top, 0.75), (x, 0.50 + 0.1 * i, 0.75), (x, 0.50 + 0.1 * i, 0.62)],
                          0.045, "bulkhead", sides=6))
    parts += port(p, "lhe_out", (1.10, 2.40, 0.75), (0, 0, 1), 0.15, body=0.12)
    parts += port(p, "lhe_return", (0.60, 2.40, 0.75), (0, 0, 1), 0.15, body=0.12)
    for i, x in enumerate((0.62, 1.12)):
        parts.append(p.box(f"gauge_{i}", (x - 0.07, 1.45, 0.69), (x + 0.07, 1.59, 0.74), "trim"))
    # the compressor skid
    parts.append(revolve(p, "motor", [(0.24, -1.55), (0.28, -1.50), (0.28, -0.98), (0.20, -0.92)], ["trim", "trim", "trim"],
                         "x", (0.55, 0.15), sides=10))
    parts.append(revolve(p, "fan_guard", [(0.31, -1.66), (0.31, -1.55)], "trim", "x", (0.55, 0.15), sides=10))
    parts.append(p.box("motor_feet", (-1.50, 0.14, -0.04), (-1.00, 0.34, 0.34), "machinery"))
    parts.append(revolve(p, "coupling", [(0.14, -0.94), (0.14, -0.80)], "accent", "x", (0.55, 0.15), sides=8))
    comp = p.box("compressor", (-0.82, 0.14, -0.12), (-0.30, 0.92, 0.42), "machinery")
    p.chamfer(comp, "compressor_top", 0.02, lambda m, d, n1, n2: near(m.y, 0.92))
    parts.append(comp)
    parts.append(revolve(p, "separator", [(0.18, -1.55), (0.28, -1.45), (0.28, -0.30), (0.18, -0.20)], "bulkhead", "x", (0.62, -0.48),
                         sides=10, caps=("bulkhead", "bulkhead")))
    for i, x in enumerate((-1.30, -0.50)):
        parts.append(p.box(f"separator_saddle_{i}", (x - 0.06, 0.14, -0.70), (x + 0.06, 0.45, -0.26), "machinery"))
    parts.append(pipe(p, "discharge", [(-0.56, 0.90, 0.10), (-0.56, 1.05, 0.10), (-0.56, 1.05, -0.48), (-0.56, 0.86, -0.48)],
                      0.04, "trim", sides=6))
    parts.append(pipe(p, "feed", [(-0.25, 0.62, -0.48), (0.36, 0.62, -0.48)], 0.05, "trim", sides=6))
    p.union(base, "cold_box_and_compressor", parts)
    p.body = base
    p.operators.append([1.0, 0.0, 1.45])

    def decor(D):
        mface = at((1.0, 0.0, 0.70))
        for x0, x1 in ((-0.70, 0.70),):
            for y in (0.80, 1.80):
                D.paint(mface, [(x0 + 0.01, y - 0.004), (x1 - 0.01, y - 0.004), (x1 - 0.01, y + 0.004), (x0 + 0.01, y + 0.004)],
                        "seam", reserve=False)
        D.plate(mface, 0.0, 2.62, 0.62, 0.08, "COLD BOX  CB-1", size=0.036)
        placard(D, mface, -0.42, 1.25, 0.24, 0.08, "4.5 K")
        D.leds(mface, 0.18, 1.25, 4, 0.06, ("led_green", "led_green", "led_amber", "led_blue"), r=0.012)
        for x in (0.62, 1.12):
            gauge(D, at((x, 1.52, 0.74)), 0.0, 0.0, 0.055, needle_deg=-30 + 40 * x)
        for R in regions(D, lambda R: R.role == "bulkhead" and R.origin.x > 0.3 and R.origin.z > 0.6):
            for y0, y1 in ((0.40, 0.70), (1.95, 2.20)):
                strip(D, R, (0, 1, 0), y0, y1, "frost", reserve=False)
        for R in regions(D, lambda R: R.role == "trim" and R.origin.x > 0.4 and R.origin.y > 2.2 and R.origin.z > 0.6):
            strip(D, R, (0, 0, 1), 0.60, 0.68, "frost", reserve=False)
        D.grille(at((-1.66, 0.55, 0.15), (-1, 0, 0)), -0.22, -0.22, 0.22, 0.22, pitch=0.04)
        for R in regions(D, lambda R: R.role == "trim" and -1.50 < R.origin.x < -0.98 and abs(R.n.x) < 0.2):
            for x in (-1.42, -1.34, -1.26, -1.18, -1.10):
                strip(D, R, (1, 0, 0), x - 0.012, x + 0.012, "dark", reserve=False)
        mc = at((-0.56, 0.0, 0.42))
        D.plate(mc, 0.0, 0.70, 0.36, 0.07, "He COMPRESSOR", size=0.026)
        D.grille(mc, -0.20, 0.25, 0.20, 0.52, pitch=0.035)
        flange_bolts(D, p)
    p.decor.append(decor)
    return p


# ----------------------------------------------------------------------------- the exhaust

def vacuum_pump():
    """The reaction chamber's exhaust pump, standing against a port on the reactor (its wall plane): a
    turbomolecular pump lying along Z on two open side frames and a cradle bar, its inlet flange and gate
    valve on the back plane (the inlet port), a backing pump box on the floor under it fed by the
    foreline, the exhaust leaving the box's front, and a cooling hose looped off the pump's side."""
    p = prop("vacuum_pump", "The reaction chamber's exhaust pump: a turbomolecular pump backed by a roughing pump (exhaust)", WALL)
    body = p.box("gate_valve", (-0.26, 0.74, 0.0), (0.26, 1.26, 0.15), {"-z": "trim", "*": "machinery"})
    p.ports["inlet"] = {"at_m": [0.0, 1.0, 0.0], "dir": [0.0, 0.0, -1.0], "dia_m": 0.35}
    p.flanges.append(("inlet", at((0.0, 1.0, 0.0), (0, 0, -1)), 0.175, 0.26))
    parts = [
             p.box("gate_bonnet", (-0.08, 1.23, 0.05), (0.08, 1.40, 0.13), "accent"),
             revolve(p, "turbo", [(0.25, 0.14), (0.25, 0.64), (0.17, 0.72)], ["bulkhead", "trim"], "z", (0.0, 1.0), sides=8,
                     caps=("trim", "trim"))]
    parts.append(p.prism("stand_frame", [(-0.34, 0.0), (-0.28, 0.0), (-0.28, 0.70), (0.28, 0.70), (0.28, 0.0), (0.34, 0.0), (0.34, 0.76),
                                         (-0.34, 0.76)], "z", 0.24, 0.30, "trim"))
    parts.append(p.box("backing_pump", (-0.19, 0.0, 0.36), (0.37, 0.40, 0.88), {"+z": "bulkhead", "*": "machinery"}))
    parts.append(rod(p, "foreline", "y", (0.0, 0.62), 0.38, 0.77, 0.035, "trim", sides=4))
    parts += port(p, "exhaust", (0.30, 0.30, 1.10), (0, 0, 1), 0.10, body=0.25, sides=6)
    parts.append(pipe(p, "cooling_hose", [(0.20, 1.06, 0.48), (0.33, 1.06, 0.48), (0.33, 0.55, 0.64), (0.33, 0.36, 0.64)], 0.013,
                      "machinery", sides=4))
    p.union(body, "valve_pump_stand", parts)
    p.body = body
    p.operators.append([0.0, 0.0, 1.6])

    def decor(D):
        mv = at((0.0, 1.0, 0.15))
        D.text(mv, "GV-3", 0.0, 0.17, 0.035, "stencil")
        for x in (-0.20, 0.20):
            for y in (-0.20, 0.20):
                D.disc(mv, x, y, 0.012, -0.002, 0.008, "bolt", sides=8, reserve=False)
        mt = at((0.0, 1.0, 0.72))
        D.disc(mt, 0.0, 0.0, 0.08, -0.002, 0.004, "dark", sides=12)
        D.leds(mt, -0.03, -0.11, 2, 0.06, ("led_green", "led_amber"), r=0.009)
        for R in regions(D, lambda R: R.role == "bulkhead" and R.origin.y > 0.7):
            for z in (0.34, 0.40, 0.46, 0.52, 0.58):
                strip(D, R, (0, 0, 1), z - 0.012, z + 0.012, "bolt", reserve=False)
        mb = at((0.09, 0.0, 0.88))
        D.plate(mb, -0.08, 0.32, 0.26, 0.06, "ROUGHING", size=0.03)
        D.grille(mb, -0.24, 0.06, 0.08, 0.22, pitch=0.03)
        for R in regions(D, lambda R: R.role == "machinery" and R.origin.x > 0.28):
            whole(D, R, "blue")
        flange_bolts(D, p)
    p.decor.append(decor)
    return p

def ash_tank():
    """The exhaust and ash tank: a vertical tank 0.9 m across on four legs, a hazard band, a pressure
    gauge and a relief stub on top, the exhaust inlet on its -X side, and a small roughing pump on a
    plinth beside it, piped into the tank's +X side."""
    p = prop("ash_tank", "The exhaust and ash tank: collects the reactor's helium ash and unburnt fuel (exhaust)", FREE)
    tx = -0.10
    tank = revolve(p, "tank", [(0.25, 0.42), (0.45, 0.58), (0.45, 2.24), (0.25, 2.40)], "bulkhead", "y", (tx, 0.0), sides=12,
                   caps=("machinery", "machinery"))
    parts = []
    for k in range(4):
        u = azim(45.0 + 90.0 * k)
        c = (tx + 0.30 * u.x, 0.30 * u.z)
        parts.append(p.box(f"leg_{k}", (c[0] - 0.045, 0.0, c[1] - 0.045), (c[0] + 0.045, 0.56, c[1] + 0.045), "machinery"))
    parts += port(p, "inlet", (-0.60, 1.80, 0.0), (-1, 0, 0), 0.15, body=0.25, sides=6)
    parts.append(rod(p, "relief", "y", (tx, 0.10), 2.30, 2.52, 0.035, "accent", sides=6))
    parts.append(p.box("gauge", (tx - 0.05, 1.47, 0.40), (tx + 0.05, 1.57, 0.50), "trim"))
    parts.append(p.box("plinth", (0.45, 0.0, -0.32), (0.80, 0.24, 0.32), {"+y": "trim", "*": "hazard"}))
    parts.append(revolve(p, "pump_motor", [(0.13, -0.28), (0.13, 0.12)], "bulkhead", "z", (0.625, 0.36), sides=8))
    parts.append(p.box("pump_head", (0.52, 0.22, 0.10), (0.73, 0.58, 0.28), "machinery"))
    parts.append(pipe(p, "pump_line", [(0.625, 0.56, 0.19), (0.625, 0.95, 0.19), (0.25, 0.95, 0.19)], 0.035, "trim", sides=6))
    p.union(tank, "legs_inlet_pump", parts)
    p.body = tank
    p.operators.append([0.30, 0.0, 0.95])

    def decor(D):
        for a, m, w in facets(12, 0.45, (tx, 0.0)):
            if a not in (270.0,):
                hazard_band(D, m, -w / 2 + 0.004, 1.02, w / 2 - 0.004, 1.18)
            for y in (0.70, 2.10):
                D.box(m, -w / 2 + 0.004, y - 0.02, w / 2 - 0.004, y + 0.02, -0.002, 0.006, "steel", inset=0.003, reserve=False)
            D.reserve(m, -w / 2, 0.58, w / 2, 2.24)
        m0 = facets(12, 0.45, (tx, 0.0))[0][1]
        label(D, m0, 0.0, 1.85, "ASH", 0.08)
        label(D, m0, 0.0, 1.75, "TK-7", 0.035)
        gauge(D, at((tx, 1.52, 0.50)), 0.0, 0.0, 0.042, needle_deg=50)
        diamond(D, m0, 0.0, 1.30, 0.06, "yellow", "!")
        mp = at((0.625, 0.0, 0.32))
        D.text(mp, "ROUGHING", 0.0, 0.12, 0.03, "stencil")
        flange_bolts(D, p)
    p.decor.append(decor)
    return p


# ----------------------------------------------------------------------------- power

def power_converter():
    """A reactor generator's direct energy converter cabinet: three bays on a hazard toe kick split by
    V-grooves, two large cooling fans let into the upper outer bays, a mimic display in the centre
    bay, a lit status strip along the top's front chamfer, door handles, louvres and warning stencils
    on the doors, and the busbar duct riser on top (0.5 x 0.2 m) ending in its flange (bus_out)."""
    p = prop("power_converter", "A direct energy converter cabinet: turns the reactor's charged exhaust into power for the grid "
                                "(power grid)", WALL)
    W, D_, top = 2.60, 1.0, 2.50
    body = p.prism("cabinet", [(0.0, 0.0), (0.92, 0.0), (0.92, 0.12), (D_, 0.12), (D_, top - 0.04), (D_ - 0.04, top), (0.0, top)], "x",
                   -W / 2, W / 2, ["machinery", "hazard", "machinery", "bulkhead", "light_panel", "machinery", "machinery"], cap="machinery")
    p.cut(body, "bay_splits", [vgroove(p, f"split_{i}", "y", x, 0.14, top - 0.06, D_) for i, x in enumerate((-0.43, 0.43))])
    duct = p.box("busbar_duct", (-0.25, top - 0.01, 0.40), (0.25, 2.765, 0.60), "trim")
    duct_flange = p.box("bus_out_flange", (-0.30, 2.76, 0.35), (0.30, 2.80, 0.65), "trim")
    p.ports["bus_out"] = {"at_m": [0.0, 2.8, 0.5], "dir": [0.0, 1.0, 0.0], "dia_m": 0.5}
    p.extra["port_sections_m"] = {"bus_out": [0.5, 0.2]}
    handles = [p.box(f"handle_{i}", (x - 0.02, 0.95, D_ - 0.01), (x + 0.02, 1.25, D_ + 0.035), "trim")
               for i, x in enumerate((-0.52, 0.34, 0.52))]
    p.union(body, "duct_and_handles", [duct, duct_flange] + handles)
    recesses = []
    for s in (-1, 1):
        p.recess(recesses, f"fan_bay_{s:+d}", frame((s * 0.86, 1.86, D_)), 0.62, 0.62, 0.09, floor_role="machinery", record=False)
    p.recess(recesses, "mimic", frame((0.0, 1.90, D_)), 0.56, 0.36, 0.015, floor_role="light_panel", record=False)
    p.cut(body, "fan_bays_and_mimic", recesses)
    blades = []
    for s in (-1, 1):
        blades += fan(p, f"fan_{s:+d}", (s * 0.86, 1.86), D_ - 0.09, 0.27, 4)
        blades.append(p.box(f"fan_bar_{s:+d}", (s * 0.86 - 0.32, 1.85, D_ - 0.06), (s * 0.86 + 0.32, 1.87, D_ - 0.035), "trim"))
    p.union(body, "fans", blades)
    p.body = body
    p.operators.append([0.0, 0.0, 1.5])

    def decor(D):
        mf = at((0.0, 0.0, D_))
        for x in (-0.86, 0.0, 0.86):
            D.grille(mf, x - 0.30, 0.30, x + 0.30, 0.80, pitch=0.035)
            placard(D, mf, x, 1.42 if x else 1.50, 0.34, 0.09, "DANGER 25 kV")
        D.plate(mf, 0.0, 1.38, 0.40, 0.07, "DEC-1", size=0.035)
        mm = at((0.0, 1.90, D_ - 0.015))
        for pts, fin in ((((-0.22, 0.0), (0.22, 0.0)), "display"), (((-0.22, 0.10), (0.0, 0.10)), "display_amber"),
                         (((0.0, -0.10), (0.22, -0.10)), "led_green")):
            (x0, y0), (x1, y1) = pts
            D.box(mm, x0, y0 - 0.006, x1, y0 + 0.006, -0.002, 0.003, fin, reserve=False)
        for x, fin in ((-0.20, "led_green"), (0.0, "led_amber"), (0.20, "led_green")):
            D.disc(mm, x, 0.0, 0.018, -0.002, 0.004, fin, sides=10, reserve=False)
        D.reserve(mm, -0.28, -0.18, 0.28, 0.18)
        mbus = at((0.0, 2.63, 0.60))
        D.text(mbus, "BUS A", 0.0, 0.0, 0.05, "stencil_dark")
        plain(D, lambda R: R.role == "trim" and R.origin.y > 2.4)
        mtop = frame((0.0, 2.80, 0.50), 90.0)
        for x in (-0.25, 0.25):
            for z in (-0.11, 0.11):
                D.disc(mtop, x, z, 0.012, -0.002, 0.008, "bolt", sides=8, reserve=False)
    p.decor.append(decor)
    return p


# ----------------------------------------------------------------------------- the reactor's magnets

COIL_AZ = [22.5 + 45.0 * k for k in range(8)]
PORT_AZ = [45.0 * k for k in range(8)]
# A coil case's outline in (r, y): the straight inner leg on r 2.25, the outer side on r 3.25 with its corners
# drawn in, so the poloidal rings (r 2.99-3.21 where they pass) run through the case's solid, clamped in it.
COIL = [(2.25, 0.40), (3.02, 0.40), (3.24, 0.46), (3.25, 0.80), (3.25, 5.20), (3.24, 5.54), (3.02, 5.60), (2.25, 5.60)]


def reactor_dressing():
    """Dressing round the reactor column (the machinery set's reactor_core, untouched), after CERN's
    magnets: eight toroidal field coil cases, deep red D-shaped plates 0.3 m thick in radial planes at
    22.5 + 45 k degrees (r 2.25-3.25, y 0.4-5.6); two octagonal poloidal coil rings (a 0.2 m square
    tube, its corners on r 3.1 inside the coils) at y 0.6 and 5.4; between the coils eight radial port
    stubs at y 1.0 ending in flanges on r 3.30 (port_0 to port_7), each standing on a pedestal; a
    cable loom down every coil's outer edge to the floor; a magnet feed box on top of two coils with
    white cryogenic lines down their sides. Everything stays between r 2.24 and 3.30 and below y 6.7,
    and clear of the azimuths 45 k between y 2.2 and 2.8 and between y 5.8 and 6.4 (the coolant,
    fuel and cryogenic lines enter the column there)."""
    p = prop("reactor_dressing", "The reactor's magnet dressing: field coils, poloidal rings, diagnostic ports, cable looms and "
                                 "feeds round the reactor column (power grid)", FREE)
    p.finish_of = {"bulkhead": "magnet_red", "hazard": "dark", "machinery": "white"}
    coils = []
    for k, a in enumerate(COIL_AZ):
        m = frame((0.0, 0.0, 0.0), 0.0, a)
        # the plate's profile in (r, y), extruded 0.3 m across its radial plane (local x)
        # a prism along X takes its profile in (z, y): the plate in the radial plane of azimuth 0, turned onto a
        coils.append(place(p.prism(f"coil_{k}", COIL, "x", -0.15, 0.15, "bulkhead"), m))
    body = coils[0]
    parts = coils[1:]
    for i, y in enumerate((0.60, 5.40)):
        parts.append(hoop(p, f"ring_{i}", Matrix.Translation((0.0, y, 0.0)), 3.10, 0.20, 0.20, 8, "trim", phase_deg=22.5))
    for k, a in enumerate(PORT_AZ):
        u = azim(a)
        parts += nozzle_port(p, f"port_{k}", tuple(3.30 * u + Vector((0.0, 1.0, 0.0))), tuple(u), 0.35, 1.04, body_role="trim")
        m = frame((0.0, 0.0, 0.0), 0.0, a)
        parts.append(p.box(f"pedestal_{k}", (-0.06, 0.0, 2.40), (0.06, 0.86, 2.52), "machinery", m=m))
    for k, a in enumerate(COIL_AZ):
        m = frame((0.0, 0.0, 0.0), 0.0, a)
        loom = p.prism(f"loom_{k}", [(3.10, 5.05), (3.295, 4.60), (3.295, 0.0), (3.225, 0.0), (3.225, 4.58), (3.08, 4.92)], "x",
                       -0.12, 0.12, "hazard")
        parts.append(place(loom, m))
    for i, k in enumerate((0, 4)):
        a = COIL_AZ[k]
        m = frame((0.0, 0.0, 0.0), 0.0, a)
        parts.append(p.box(f"feed_box_{i}", (-0.24, 5.55, 2.40), (0.24, 6.20, 3.05), "machinery", m=m))
        for j, s in enumerate((-1, 1)):
            path = in_frame(m, [(s * 0.21, 5.95, 3.00), (s * 0.21, 5.95, 3.20), (s * 0.21, 0.0, 3.20)])
            parts.append(pipe(p, f"cryo_line_{i}_{j}", path, 0.035, "machinery", sides=6))
    p.union(body, "coils_rings_ports_looms_feeds", parts)
    p.body = body

    def decor(D):
        for R in regions(D, lambda R: R.role == "hazard" and abs(R.n.y) < 0.5):
            stripes(D, R, ("blue", "red", "dark"))
        for k, a in enumerate(COIL_AZ):
            u = azim(a)
            t = azim(a + 90.0)
            for s in (-1, 1):
                n = t * s
                mside = facet(tuple(u * 2.75 + n * 0.15), tuple(n), (0.0, 1.0, 0.0))
                lx = -1.0 if (mside.to_3x3() @ Vector((1, 0, 0))).dot(u) < 0 else 1.0
                D.text(mside, f"TF-{k + 1:02d}", lx * 0.0, 3.05, 0.14, "stencil")
                D.text(mside, "B 5.3 T", 0.0, 2.85, 0.06, "stencil")
                for y in (1.2, 2.0, 3.6, 4.4):
                    for rr in (2.40, 3.08):
                        D.disc(mside, lx * (rr - 2.75), y, 0.018, -0.002, 0.012, "bolt", sides=8, inset=0.005)
                placard(D, mside, 0.0, 1.55, 0.36, 0.09, "MAGNET")
        flange_bolts(D, p)
        for i, k in enumerate((0, 4)):
            a = COIL_AZ[k]
            u = azim(a)
            mf = facet(tuple(u * 3.05 + Vector((0.0, 0.0, 0.0))), tuple(u), (0.0, 1.0, 0.0))
            placard(D, mf, 0.0, 5.80, 0.40, 0.09, "CRYO FEED")
            D.leds(mf, -0.12, 5.66, 3, 0.12, ("led_green", "led_blue", "led_green"), r=0.016)
            mt = facet(tuple(Vector((0.0, 6.20, 0.0)) + u * 2.725), (0.0, 1.0, 0.0), tuple(u))
            D.grille(mt, -0.18, -0.25, 0.18, 0.25, pitch=0.04)
        for R in regions(D, lambda R: R.role == "machinery" and R.origin.y < 5.6):
            for y0, y1 in ((5.70, 5.92), (0.05, 0.25)):
                strip(D, R, (0, 1, 0), y0, y1, "frost", reserve=False)
    p.decor.append(decor)
    return p


# ----------------------------------------------------------------------------- handling

RAIL_TOP = 1.60     # the crane's rails' top above its hook's lowest point


def gantry_crane():
    """An overhead travelling crane in safety yellow: a box girder 17.0 m long along X stiffened by
    three ribs, an end truck at each end (centred on x = +-8.5, 0.5 x 0.4 x 1.6 along Z) with a drive
    motor and buffers on short rail stubs, an underhung hoist trolley at x = +2.0 with its motor, two
    chains and a hook block hanging 1.2 m below, and the capacity stencil on the girder's side. Its
    hook's lowest point is on y = 0 (the kit's read-back wants every prop's lowest point there): the
    rails' top is rail_top_m (1.60 m) above it."""
    p = prop("gantry_crane", "An overhead travelling crane over the engine core room's floor (maintenance)",
             "under the girder's centre, at the hook's lowest point; the rails' top is rail_top_m above it")
    p.finish_of = {"machinery": "yellow"}
    Y = RAIL_TOP
    girder = p.box("girder", (-8.5, Y + 0.30, -0.25), (8.5, Y + 0.90, 0.25), "machinery")
    parts = []
    for i, x in enumerate((-4.25, 0.0, 4.25)):
        parts.append(p.box(f"rib_{i}", (x - 0.03, Y + 0.28, -0.27), (x + 0.03, Y + 0.92, 0.27), "machinery"))
    for s in (-1, 1):
        x = s * 8.5
        parts.append(p.box(f"end_truck_{s:+d}", (x - 0.25, Y - 0.01, -0.80), (x + 0.25, Y + 0.40, 0.80),
                           {"+y": "machinery", "-y": "machinery", "*": "hazard"}))
        parts.append(p.box(f"rail_{s:+d}", (x - 0.04, Y - 0.14, -1.0), (x + 0.04, Y + 0.0, 1.0), "trim"))
        for e in (-1, 1):
            parts.append(p.box(f"buffer_{s:+d}_{e:+d}", (x - 0.09, Y + 0.12, min(e * 0.79, e * 0.92)), (x + 0.09, Y + 0.28, max(e * 0.79, e * 0.92)),
                               "trim"))
    parts.append(p.box("trolley", (1.65, Y - 0.20, -0.32), (2.35, Y + 0.31, 0.32), {"*": "machinery"}))
    parts.append(revolve(p, "hoist_motor", [(0.12, 1.70), (0.12, 2.30)], "trim", "x", (Y + 0.0, 0.40), sides=8))
    for i, x in enumerate((1.89, 2.11)):
        parts.append(p.box(f"chain_{i}", (x - 0.015, Y - 1.11, -0.015), (x + 0.015, Y - 0.19, 0.015), "trim"))
    parts.append(p.box("hook_block", (1.82, Y - 1.40, -0.10), (2.18, Y - 1.10, 0.10), {"*": "hazard"}))
    parts.append(p.hull("hook_shank", [(2.0 + a, Y - 1.38, b) for a in (-0.03, 0.03) for b in (-0.025, 0.025)]
                        + [(2.0 + a, Y - 1.52, b) for a in (-0.03, 0.03) for b in (-0.025, 0.025)], "trim"))
    parts.append(p.hull("hook_bow", [(1.97, Y - 1.51, -0.025), (1.97, Y - 1.51, 0.025), (2.10, Y - 1.51, -0.025), (2.10, Y - 1.51, 0.025),
                                     (2.02, 0.0, -0.02), (2.02, 0.0, 0.02), (2.08, 0.0, -0.02), (2.08, 0.0, 0.02)], "trim"))
    parts.append(p.hull("hook_tip", [(2.05, 0.005, -0.02), (2.05, 0.005, 0.02), (2.13, 0.01, -0.02), (2.13, 0.01, 0.02),
                                     (2.16, 0.14, -0.015), (2.16, 0.14, 0.015), (2.12, 0.14, -0.015), (2.12, 0.14, 0.015)], "trim"))
    p.union(girder, "ribs_trucks_trolley_hook", parts)
    p.body = girder
    p.extra["rail_top_m"] = RAIL_TOP

    def decor(D):
        mside = at((0.0, 0.0, 0.25))
        D.text(mside, "SWL 20 t", -1.6, Y + 0.60, 0.30, "stencil_dark")
        D.text(at((0.0, 0.0, -0.25), (0, 0, -1)), "SWL 20 t", 1.6, Y + 0.60, 0.30, "stencil_dark")
        D.text(mside, "OHC-1", -6.0, Y + 0.60, 0.20, "stencil_dark")
        mb = at((2.0, Y - 1.25, 0.10))
        D.text(mb, "20 t", 0.0, 0.0, 0.08, "stencil_dark")
        for s in (-1, 1):
            me = at((s * 8.5, 0.0, 0.80))
            D.plate(me, 0.0, Y + 0.30, 0.30, 0.07, "TRUCK " + ("A" if s < 0 else "B"), size=0.03)
    p.decor.append(decor)
    return p


# ----------------------------------------------------------------------------- consoles

def local_panel():
    """A local control station at a machine: a pedestal cabinet with a hazard toe kick and a sloped
    instrument face between 1.0 and 1.5 m, carrying four round gauges in bezels, a small screen, a row
    of lit push buttons, a red emergency-stop mushroom in a yellow guard, a key switch and a label
    plate; a lit chamfer on top and a conduit down its side to the floor."""
    p = prop("local_panel", "A local control station beside a machine (engineering)", WALL)
    hw = 0.40
    prof = [(0.0, 0.0), (0.40, 0.0), (0.40, 0.10), (0.46, 0.10), (0.46, 0.98), (0.50, 1.00), (0.32, 1.50), (0.32, 1.72), (0.29, 1.75),
            (0.0, 1.75)]
    roles = ["machinery", "hazard", "machinery", "machinery", "trim", "bulkhead", "machinery", "light_panel", "machinery", "machinery"]
    body = p.prism("cabinet", prof, "x", -hw, hw, roles, cap="machinery")
    tilt = math.degrees(math.atan2(0.18, 0.50))

    def on_face(x, t):
        """A frame on the sloped face at x across, t up the face from its middle."""
        L = math.hypot(0.18, 0.50)
        return frame((x, 1.25 + t * 0.50 / L, 0.41 - t * 0.18 / L), tilt)
    parts = []
    for i, x in enumerate((-0.27, -0.09, 0.09, 0.27)):
        parts.append(turned(p, f"bezel_{i}", on_face(x, 0.13), [(0.068, -0.01), (0.068, 0.016)], "trim", 8))
    parts.append(p.box("estop_guard", (-0.055, -0.055, -0.01), (0.055, 0.055, 0.035), "accent", m=on_face(0.26, -0.11)))
    parts.append(turned(p, "estop", on_face(0.26, -0.11), [(0.022, 0.02), (0.022, 0.05), (0.04, 0.05), (0.04, 0.072)], "trim", 8))
    parts.append(pipe(p, "conduit", [(0.36, 0.55, 0.22), (0.425, 0.55, 0.22), (0.425, 0.0, 0.22)], 0.025, "trim", sides=6))
    p.union(body, "bezels_estop_conduit", parts)
    screens = []
    p.recess(screens, "screen", on_face(-0.13, -0.08), 0.24, 0.13, 0.012, shows="console")
    p.cut(body, "screen", screens)
    p.body = body
    p.operators.append([0.0, 0.0, 0.90])

    def decor(D):
        for i, x in enumerate((-0.27, -0.09, 0.09, 0.27)):
            gauge(D, on_face(x, 0.13) @ Matrix.Translation((0.0, 0.0, 0.016)), 0.0, 0.0, 0.058, needle_deg=-40 + 25 * i)
        mf = on_face(0.0, 0.0)
        D.buttons(mf, -0.27, -0.215, 6, 0.06, 0.035, ("led_green", "key", "led_amber", "key", "led_red", "key"))
        D.disc(mf, 0.12, -0.11, 0.02, -0.002, 0.012, "steel", sides=10, inset=0.004)
        D.box(mf, 0.115, -0.125, 0.125, -0.095, 0.010, 0.016, "stencil_dark", reserve=False)
        D.text(mf, "KEY", 0.12, -0.155, 0.018, "stencil")
        for R in regions(D, lambda R: R.role == "trim" and R.origin.x > 0.2 and R.origin.y > 1.1 and R.origin.y < 1.25):
            if R.origin.z > 0.42:
                whole(D, R, "red")
        mfront = at((0.0, 0.0, 0.46))
        D.plate(mfront, 0.0, 0.84, 0.36, 0.08, "LCP-1", size=0.04)
        D.grille(mfront, -0.25, 0.25, 0.25, 0.55, pitch=0.03)
        mup = at((0.0, 0.0, 0.32))
        D.leds(mup, -0.12, 1.62, 4, 0.08, ("led_green", "led_green", "led_amber", "led_red"), r=0.012)
    p.decor.append(decor)
    return p


def control_desk():
    """The engineering control desk (the engineer's station, eng_main), after free_console's conventions:
    a desk 3.0 m wide, slightly curved toward the operator (a lathe in three flat facets on a 4 m
    radius), a knee recess under its top, a sloped control shelf with a lit mimic strip along its top
    edge, and three monitors on stands angled at the seat (the centre one the console image, the outer
    ones the two halves of the upper image). Two big rotary knobs and breaker toggles stand on the
    shelf, a handset in its cradle on the left end."""
    p = prop("control_desk", "The engineering control desk: the engineer's station (eng_main)", "floor, centre of the pedestal's back")
    steps, rb = 3, 4.0
    half = math.asin(1.5 / rb)
    fa = 2 * half / steps
    cz = rb * math.cos(fa / 2)
    prof = [(0.0, 0.0), (0.60, 0.0), (0.60, 0.66), (1.18, 0.66), (1.18, 0.76), (0.78, 0.80), (0.42, 1.02), (0.36, 1.08), (0.0, 1.08)]
    roles = ["machinery", "machinery", "machinery", "trim", "bulkhead", "machinery", "light_panel", "machinery", "machinery"]
    body = lathe(p.coll, "control_desk.body", prof, roles, cz, rb, -half, half, steps, "machinery")
    p.chamfer(body, "front_edge", 0.02, lambda m, d, n1, n2: near(math.hypot(m.x, m.z - cz), (rb - 1.18) * math.cos(fa / 2), 0.03)
              and near(m.y, 0.76, 0.01))

    def on(k, d, y, tilt):
        """A frame on facet k (0 left, 1 centre, 2 right) at depth d in from the back and height y."""
        phi = -half + (k + 0.5) * fa
        rr = (rb - d) * math.cos(fa / 2)
        return frame((rr * math.sin(phi), y, cz - rr * math.cos(phi)), tilt, -math.degrees(phi))
    shelf_tilt = 90.0 - math.degrees(math.atan2(0.22, 0.36))
    parts = []
    housings = []
    for k in range(3):
        mh = on(k, 0.18, 1.30, 15.0)
        h = p.box(f"monitor_{k}", (-0.46, -0.20, -0.07), (0.46, 0.20, 0.0), {"*": "machinery"}, m=mh)
        p.chamfer(h, f"monitor_{k}_edges", 0.012, lambda m, d, n1, n2, k=k: m.y > 1.40)
        housings.append((k, mh))
        parts.append(h)
        ms = on(k, 0.17, 0.0, 0.0)
        parts.append(p.box(f"stand_{k}", (-0.07, 1.06, -0.05), (0.07, 1.20, 0.05), "trim", m=ms))
    md = lambda k, d: on(k, d, 0.80 + (0.78 - d) / 0.36 * 0.22, shelf_tilt)   # noqa: E731  the control shelf
    for i, x in enumerate((-0.18, 0.18)):
        parts.append(turned(p, f"knob_{i}", md(1, 0.60) @ Matrix.Translation((x, 0.0, 0.0)),
                            [(0.055, -0.01), (0.055, 0.035), (0.035, 0.06)], ["trim", "trim"], 8))
    for k in (0, 2):
        for i in range(4):
            x = -0.27 + 0.18 * i
            up = not (k == 2 and i == 1)
            lean = 0.02 if up else -0.02
            parts.append(p.hull(f"toggle_{k}_{i}", in_frame(md(k, 0.60) @ Matrix.Translation((x, 0.0, 0.0)),
                                                         [(-0.011, -0.008, -0.012), (0.011, -0.008, -0.012), (0.0, 0.012, -0.012),
                                                          (-0.014, -0.004 + lean, 0.06), (0.014, -0.004 + lean, 0.06),
                                                          (0.0, 0.006 + lean, 0.06)]), "machinery" if up else "accent"))
    mtop = on(0, 0.98, 0.78, 84.0)
    parts.append(p.box("handset_cradle", (-0.36, -0.07, -0.02), (-0.16, 0.07, 0.03), "machinery", m=mtop))
    parts.append(p.hull("handset", in_frame(mtop, [(x, y, z) for x in (-0.35, -0.17) for y in (-0.035, 0.035) for z in (0.025, 0.06)]),
                        "trim"))
    p.union(body, "monitors_and_controls", parts)
    screens = []
    for k, mh in housings:
        p.recess(screens, f"monitor_{k}", mh, 0.82, 0.32, 0.015, shows="console" if k == 1 else "upper",
                 half=None if k == 1 else (0 if k == 0 else 1))
    p.cut(body, "screens", screens)
    p.body = body
    p.operators.append([0.0, 0.0, round(1.18 * math.cos(fa / 2) + 0.35, 4)])

    def decor(D):
        for k in range(3):
            m = md(k, 0.60)
            if k == 1:
                for x in (-0.18, 0.18):
                    D.ring(m, x, 0.0, 0.075, 0.09, -0.002, 0.004, "stencil", sides=20)
                D.buttons(m, -0.40, -0.12, 4, 0.06, 0.035, ("led_green", "key", "led_amber", "key"))
                D.buttons(m, 0.24, -0.12, 3, 0.06, 0.035, ("key", "led_red", "key"))
                D.text(m, "POWER", -0.18, 0.11, 0.025, "stencil")
                D.text(m, "FLOW", 0.18, 0.11, 0.025, "stencil")
            else:
                D.leds(m, -0.27, 0.09, 4, 0.18, ("led_green", "led_amber", "led_green", "led_red") if k == 2 else ("led_green",) * 4, r=0.01)
                D.buttons(m, -0.33, -0.12, 6, 0.11, 0.04, ("key", "led_white", "key"))
            D.grille(on(k, 0.0, 0.0, 0.0) @ Matrix.Translation((0.0, 0.0, 0.60)), -0.40, 0.15, 0.40, 0.50, pitch=0.035)
            if k == 1:
                D.plate(on(1, 0.0, 0.0, 0.0) @ Matrix.Translation((0.0, 0.0, 0.60)), 0.0, 0.58, 0.46, 0.08, "ENGINEERING", size=0.04)
        for R in regions(D, lambda R: R.role == "light_panel"):
            stripes(D, R, ("display", "lamp", "display_amber", "lamp", "led_green"))
    p.decor.append(decor)
    return p


def mimic_board():
    """The plant mimic wall: a board 3.6 m wide on a recessed kick, the plant drawn as lit lines in a
    dark field let into its upper part (the reactor, its two heat exchangers, pumps, tanks and the fuel
    train, with status lamps at the nodes and labels), a digital readout strip under it and a row of
    eight analogue meters along the bottom; its lit parts are baked light in the atlas."""
    p = prop("mimic_board", "The plant mimic wall: the coolant, power and fuel loops drawn in light (engineering)", WALL)
    p.finish_of = {"bulkhead": "panel"}
    body = p.prism("board", [(0.0, 0.0), (0.10, 0.0), (0.10, 0.08), (0.15, 0.12), (0.15, 2.36), (0.12, 2.40), (0.0, 2.40)], "x", -1.80,
                   1.80, ["machinery", "hazard", "machinery", "bulkhead", "trim", "machinery", "machinery"], cap="machinery")
    meters = [p.box(f"meter_{i}", (-1.54 + 0.44 * i - 0.15, 0.56, 0.14), (-1.54 + 0.44 * i + 0.15, 0.84, 0.20), {"*": "trim"})
              for i in range(8)]
    p.union(body, "meters", meters)
    recesses = []
    p.recess(recesses, "field", frame((0.0, 1.66, 0.15)), 3.44, 1.20, 0.02, floor_role="machinery", record=False)
    p.recess(recesses, "readout", frame((0.0, 0.96, 0.15)), 3.20, 0.10, 0.012, floor_role="light_panel", record=False)
    p.cut(body, "field_and_readout", recesses)
    p.body = body
    p.operators.append([0.0, 0.0, 1.5])

    def decor(D):
        m = at((0.0, 1.66, 0.13))

        def line(pts, fin, w=0.012):
            for (x0, y0), (x1, y1) in zip(pts[:-1], pts[1:]):
                D.box(m, min(x0, x1) - w, min(y0, y1) - w, max(x0, x1) + w, max(y0, y1) + w, -0.002, 0.003, fin, reserve=False)

        def outline(x0, y0, x1, y1, fin):
            line([(x0, y0), (x1, y0), (x1, y1), (x0, y1), (x0, y0)], fin, 0.008)
        # the reactor and its loops
        outline(-0.20, -0.30, 0.20, 0.30, "led_amber")
        D.text(m, "REACTOR", 0.0, 0.0, 0.045, "led_amber")
        for s in (-1, 1):
            hx = s * 0.90
            outline(hx - 0.14, -0.20, hx + 0.14, 0.32, "display")
            D.text(m, "HX-" + ("1" if s < 0 else "2"), hx, 0.06, 0.04, "display")
            line([(s * 0.20, 0.20), (hx - s * 0.14, 0.20)], "led_red")
            line([(hx, -0.20), (hx, -0.42), (s * 0.45, -0.42), (s * 0.45, -0.20), (s * 0.20, -0.20)], "display")
            D.disc(m, s * 0.45, -0.42, 0.045, -0.002, 0.003, "display", sides=12, reserve=False)
            line([(hx + s * 0.14, 0.10), (s * 1.55, 0.10), (s * 1.55, 0.45)], "led_green")
            D.text(m, "RADIATORS", s * 1.40, 0.52, 0.03, "led_green")
            for xx, yy in ((s * 0.45, -0.42), (hx, 0.32), (s * 1.55, 0.10)):
                D.disc(m, xx, yy + 0.07, 0.016, -0.002, 0.004, "led_green", sides=10, reserve=False)
        outline(-1.62, -0.52, -1.30, -0.30, "led_white")
        D.text(m, "D2", -1.46, -0.41, 0.04, "led_white")
        line([(-1.30, -0.41), (-0.70, -0.41), (-0.70, -0.52), (-0.12, -0.52), (-0.12, -0.30)], "led_white")
        outline(1.30, -0.52, 1.62, -0.30, "led_white")
        D.text(m, "DRAIN", 1.46, -0.41, 0.032, "led_white")
        D.text(m, "PLANT MIMIC   LOOP 1 + 2", 0.0, 0.52, 0.05, "stencil")
        D.disc(m, 0.0, -0.45, 0.02, -0.002, 0.004, "led_red", sides=10, reserve=False)
        mr = at((0.0, 0.96, 0.138))
        for i in range(10):
            x = -1.45 + 0.32 * i
            D.box(mr, x - 0.11, -0.03, x + 0.11, 0.03, -0.002, 0.003, "display_amber" if i % 3 else "display", reserve=False)
        for i in range(8):
            mm = at((-1.54 + 0.44 * i, 0.70, 0.20))
            D.box(mm, -0.13, -0.11, 0.13, 0.09, -0.002, 0.004, "stencil")
            arc = [(-0.10 + 0.20 * j / 6, 0.03 + 0.03 * math.sin(math.pi * j / 6)) for j in range(7)]
            for (x0, y0), (x1, y1) in zip(arc[:-1], arc[1:]):
                D.box(mm, min(x0, x1), min(y0, y1) - 0.002, max(x0, x1), max(y0, y1) + 0.002, 0.003, 0.0055, "stencil_dark", reserve=False)
            mn = mm @ Matrix.Translation((0.0, -0.08, 0.0)) @ Matrix.Rotation(math.radians(-50 + 14 * i), 4, "Z")
            D.box(mn, -0.003, 0.0, 0.003, 0.15, 0.004, 0.0068, "red", reserve=False)
            D.text(mm, ("MW", "kA", "K", "bar", "%", "rpm", "kV", "Hz")[i], 0.0, -0.085, 0.022, "stencil_dark", z=0.0068)
    p.decor.append(decor)
    return p


# ----------------------------------------------------------------------------- maintenance

def tool_board():
    """The maintenance bench: a heavy steel workbench (top at 0.9 m) with a drawer bank and a cupboard
    split by V-grooves on a recessed kick, a vice at its right front, a pegboard above between two
    uprights, its tools hung on it (wrenches, a pipe wrench, a hammer, a torque wrench and a coiled
    hose), a shelf of boxes on top and a task lamp on an arm."""
    p = prop("tool_board", "The maintenance bench: the engineers' tools for keeping the plant running (maintenance)", WALL)
    p.finish_of = {"bulkhead": "green"}
    bench = p.box("carcass", (-1.10, 0.0, 0.0), (1.10, 0.86, 0.68), {"+z": "bulkhead", "*": "machinery"})
    p.cut(bench, "kick", [p.box("kick_cut", (-1.15, -0.05, 0.62), (1.15, 0.10, 0.73), "hazard")])
    grooves = [vgroove(p, f"drawer_{i}", "x", y, -1.06, -0.30, 0.68) for i, y in enumerate((0.32, 0.50, 0.68))]
    grooves += [vgroove(p, "bank_side", "y", -0.30, 0.10, 0.82, 0.68), vgroove(p, "door_split", "y", 0.40, 0.10, 0.82, 0.68)]
    p.cut(bench, "drawer_and_door_lines", grooves)
    parts = [p.box("top", (-1.10, 0.85, 0.0), (1.10, 0.90, 0.75), "trim")]
    parts += [p.box("vice_base", (0.58, 0.89, 0.42), (0.86, 0.98, 0.66), "machinery"),
              p.box("vice_fixed_jaw", (0.60, 0.97, 0.44), (0.84, 1.08, 0.52), "accent"),
              p.box("vice_moving_jaw", (0.60, 0.95, 0.60), (0.84, 1.08, 0.68), "accent"),
              rod(p, "vice_screw", "z", (0.72, 1.01), 0.66, 0.745, 0.016, "trim", sides=4)]
    for s in (-1, 1):
        parts.append(p.box(f"upright_{s:+d}", (min(s * 1.04, s * 1.10), 0.89, 0.0), (max(s * 1.04, s * 1.10), 2.20, 0.06), "trim"))
    parts.append(p.box("pegboard", (-1.05, 1.02, 0.0), (1.05, 2.02, 0.025), "machinery"))
    parts.append(p.box("shelf", (-1.10, 2.10, 0.0), (1.10, 2.15, 0.34), "trim"))
    for i, (x0, x1, h) in enumerate(((-0.95, -0.45, 0.18), (0.05, 0.50, 0.20))):
        parts.append(p.box(f"box_{i}", (x0, 2.14, 0.04), (x1, 2.14 + h, 0.30), "accent" if i == 1 else "bulkhead"))
    tools = [
        ("wrench_a", [(-0.80, 1.20), (-0.76, 1.20), (-0.76, 1.62), (-0.72, 1.66), (-0.74, 1.72), (-0.82, 1.72), (-0.84, 1.66), (-0.80, 1.62)]),
        ("wrench_b", [(-0.64, 1.28), (-0.605, 1.28), (-0.605, 1.62), (-0.57, 1.65), (-0.59, 1.70), (-0.655, 1.70), (-0.675, 1.65), (-0.64, 1.62)]),
        ("pipe_wrench", [(-0.42, 1.12), (-0.37, 1.12), (-0.37, 1.70), (-0.30, 1.70), (-0.30, 1.80), (-0.45, 1.80), (-0.45, 1.70), (-0.42, 1.70)]),
        ("hammer", [(-0.16, 1.20), (-0.13, 1.20), (-0.13, 1.66), (-0.06, 1.66), (-0.06, 1.74), (-0.24, 1.74), (-0.24, 1.66), (-0.16, 1.66)]),
        ("torque_wrench", [(0.06, 1.10), (0.10, 1.10), (0.10, 1.72), (0.12, 1.76), (0.08, 1.82), (0.04, 1.76), (0.06, 1.72)]),
    ]
    for name, poly in tools:
        parts.append(p.prism(name, [(x, y) for x, y in poly], "z", 0.015, 0.045, "trim"))
    parts.append(hoop(p, "hose_coil", Matrix.Translation((0.62, 1.50, 0.04)) @ Matrix.Rotation(math.pi / 2, 4, "X"), 0.20, 0.035, 0.04,
                      6, "machinery", phase_deg=0.0))
    parts.append(pipe(p, "lamp_arm", [(1.07, 1.95, 0.03), (0.85, 2.05, 0.30), (0.70, 1.80, 0.42)], 0.018, "trim", sides=4))
    parts.append(p.hull("lamp_shade", [(0.70 + 0.07 * math.cos(a), 1.82, 0.42 + 0.07 * math.sin(a)) for a in (0, 2.1, 4.2)]
                        + [(0.70 + 0.13 * math.cos(a), 1.70, 0.42 + 0.13 * math.sin(a)) for a in (0.0, 1.05, 2.1, 3.15, 4.2, 5.25)],
                        "machinery"))
    p.union(bench, "top_vice_board_tools", parts)
    p.body = bench
    p.operators.append([0.0, 0.0, 1.2])

    def decor(D):
        mf = at((0.0, 0.0, 0.68))
        for y in (0.22, 0.41, 0.59, 0.77):
            D.box(mf, -0.76, y - 0.012, -0.60, y + 0.012, -0.002, 0.016, "steel", inset=0.004)
        for x in (0.30, 0.50):
            D.box(mf, x - 0.012, 0.55, x + 0.012, 0.72, -0.002, 0.016, "steel", inset=0.004)
        D.plate(mf, 0.74, 0.30, 0.30, 0.07, "TOOLS", size=0.035)
        mp = at((0.0, 0.0, 0.025))
        for i in range(-20, 21):
            for j in range(10):
                x, y = i * 0.05, 1.07 + j * 0.10
                if all(not (x0 - 0.04 < x < x1 + 0.04 and y0 - 0.04 < y < y1 + 0.04) for x0, y0, x1, y1 in
                       ((-0.86, 1.08, 0.14, 1.84), (0.38, 1.25, 0.86, 1.75))):
                    D.disc(mp, x, y, 0.006, -0.002, 0.002, "dark", sides=6, reserve=False)
        for name, poly in tools:
            xs = [q[0] for q in poly]
            ys = [q[1] for q in poly]
            cx, cy_ = (min(xs) + max(xs)) / 2, (min(ys) + max(ys)) / 2
            D.prism(mp, [(cx + (x - cx) * 1.25, cy_ + (y - cy_) * 1.08) for x, y in poly], -0.002, 0.0015, "stencil", reserve=False)
        for R in regions(D, lambda R: R.role == "trim" and R.n.z > 0.9 and abs(R.origin.z - 0.045) < 0.002):
            whole(D, R, "steel")
        label(D, mp, 0.0, 1.96, "RETURN EVERY TOOL", 0.04, ink="stencil")
        for R in regions(D, lambda R: R.role == "machinery" and 1.68 < R.origin.y < 1.85 and R.origin.z > 0.3):
            if R.n.y < -0.5:
                whole(D, R, "lamp")
        mb = at((0.0, 0.0, 0.30))
        D.text(mb, "SEALS", -0.70, 2.23, 0.035, "stencil_dark")
        D.text(mb, "FILTERS", 0.275, 2.24, 0.035, "stencil_dark")
    p.decor.append(decor)
    return p


def tool_chest():
    """A rolling tool chest in red: a six-drawer cabinet on four casters with a side push handle and a
    top box with a lid; drawer fronts, pulls and the lid line are in the atlas."""
    p = prop("tool_chest", "A rolling tool chest (maintenance)", FREE)
    p.finish_of = {"machinery": "red"}
    body = p.box("cabinet", (-0.46, 0.10, -0.275), (0.46, 0.86, 0.275), "machinery")
    parts = [p.box("top_box", (-0.45, 0.85, -0.25), (0.45, 1.10, 0.25), "machinery")]
    for sx in (-1, 1):
        for sz in (-1, 1):
            parts.append(p.box(f"caster_{sx:+d}{sz:+d}", (sx * 0.38 - 0.04, 0.0, sz * 0.19 - 0.04), (sx * 0.38 + 0.04, 0.11, sz * 0.19 + 0.04),
                               "trim"))
    parts.append(pipe(p, "push_handle", [(0.45, 0.80, -0.17), (0.50, 0.80, -0.17), (0.50, 0.80, 0.17), (0.45, 0.80, 0.17)], 0.015, "trim",
                      sides=4))
    p.union(body, "top_casters_handle", parts)
    p.body = body
    p.operators.append([0.0, 0.0, 0.85])

    def decor(D):
        mf = at((0.0, 0.0, 0.275))
        ys = [0.12, 0.24, 0.36, 0.48, 0.60, 0.72, 0.84]
        for y in ys[1:-1]:
            D.paint(mf, [(-0.445, y - 0.004), (0.445, y - 0.004), (0.445, y + 0.004), (-0.445, y + 0.004)], "seam", reserve=False)
        for y0, y1 in zip(ys[:-1], ys[1:]):
            D.box(mf, -0.30, y1 - 0.035, 0.30, y1 - 0.018, -0.002, 0.014, "steel", inset=0.004, reserve=False)
        D.reserve(mf, -0.46, 0.10, 0.46, 0.86)
        mt = at((0.0, 0.0, 0.25))
        D.paint(mt, [(-0.445, 1.035), (0.445, 1.035), (0.445, 1.043), (-0.445, 1.043)], "seam", reserve=False)
        D.box(mt, -0.06, 0.995, 0.06, 1.02, -0.002, 0.012, "steel", inset=0.004)
        D.plate(mt, 0.25, 0.94, 0.20, 0.06, "TC-1", size=0.03)
    p.decor.append(decor)
    return p


def parts_rack():
    """Heavy shelving of spares: four shelves between two solid end panels, two gas bottles standing on
    the left panel's foot plate (their chain is in the atlas); on the shelves crates, a pump impeller, a
    valve body, a pipe spool and a cable reel."""
    p = prop("parts_rack", "A rack of spares: impellers, valves, pipe spools, cable and gas (maintenance)", WALL)
    p.finish_of = {"bulkhead": "green"}
    xl, xr = -0.78, 1.06
    body = p.box("panel_l", (xl - 0.06, 0.01, 0.0), (xl, 2.30, 0.70), "machinery")
    parts = [p.box("panel_r", (xr, 0.01, 0.0), (xr + 0.06, 2.30, 0.70), "machinery"),
             p.box("foot_plate", (-1.12, 0.0, 0.0), (xl - 0.03, 0.04, 0.70), {"+y": "trim", "*": "hazard"})]
    for i, y in enumerate((0.10, 0.66, 1.22, 1.78)):
        parts.append(p.box(f"shelf_{i}", (xl - 0.02, y, 0.01), (xr + 0.02, y + 0.04, 0.68), "trim"))
    for i, (x0, x1, y, h) in enumerate(((-0.70, -0.25, 0.13, 0.36), (0.55, 1.00, 1.25, 0.30), (-0.15, 0.25, 1.81, 0.24))):
        parts.append(p.box(f"crate_{i}", (x0, y, 0.06), (x1, y + h, 0.58), "bulkhead"))
    parts.append(revolve(p, "impeller", [(0.20, 0.69), (0.20, 0.74), (0.06, 0.80), (0.06, 0.86)], "accent", "y", (-0.40, 0.35), sides=6))
    parts.append(p.box("valve_body", (0.30, 0.69, 0.22), (0.54, 0.90, 0.46), "machinery"))
    parts.append(rod(p, "valve_flanges", "x", (0.795, 0.34), 0.24, 0.60, 0.10, "trim", sides=6))
    parts.append(revolve(p, "pipe_spool", [(0.12, -0.55), (0.12, 0.25)], "trim", "x", (1.37, 0.35), sides=6))
    parts.append(revolve(p, "cable_reel", [(0.20, 1.81), (0.20, 2.00)], "machinery", "y", (0.72, 0.35), sides=8))
    for i, z in enumerate((0.20, 0.50)):
        parts.append(revolve(p, f"gas_bottle_{i}", [(0.11, 0.02), (0.11, 1.30), (0.04, 1.42), (0.04, 1.50)], ["bulkhead", "bulkhead", "trim"],
                             "y", (-0.98, z), sides=6))
    p.union(body, "shelves_and_spares", parts)
    p.body = body
    p.operators.append([0.0, 0.0, 1.2])

    def decor(D):
        for i, (x0, x1, y, h) in enumerate(((-0.70, -0.25, 0.13, 0.36), (0.55, 1.00, 1.25, 0.30), (-0.15, 0.25, 1.81, 0.24))):
            mc = at(((x0 + x1) / 2, y + h / 2, 0.58))
            D.text(mc, ("SEALS", "BEARINGS", "GASKETS")[i], 0.0, 0.03, 0.035, "stencil")
            D.text(mc, f"P/N 40{i}7-{i + 2}", 0.0, -0.04, 0.022, "stencil")
        for z in (0.20, 0.50):
            for a, m, w in facets(6, 0.11, (-0.98, z)):
                D.box(m, -w / 2 + 0.003, 1.05, w / 2 - 0.003, 1.30, -0.002, 0.004, "white", reserve=False)
                for y in (0.55, 1.00):
                    for j in range(3):
                        D.box(m, -w / 2 + 0.004 + j * (w - 0.008) / 3, y - 0.012, -w / 2 + 0.004 + (j + 1) * (w - 0.008) / 3 - 0.006, y + 0.012,
                              -0.002, 0.010, "steel", inset=0.004, reserve=False)
                D.reserve(m, -w / 2, 0.02, w / 2, 1.30)
            D.text(facets(6, 0.11, (-0.98, z))[0][1], "N2" if z < 0.3 else "Ar", 0.0, 0.80, 0.05, "stencil_dark")
        mside = at((xl - 0.06, 0.0, 0.35), (-1, 0, 0))
        for y in (0.55, 1.00):
            D.box(mside, -0.30, y - 0.015, 0.30, y + 0.015, -0.002, 0.012, "steel", inset=0.004, reserve=False)
        for R in regions(D, lambda R: R.role == "trim" and R.n.z > 0.9 and abs(R.origin.z - 0.68) < 0.01):
            for x in (-0.6, 0.2, 0.9):
                strip(D, R, (1, 0, 0), x - 0.08, x + 0.08, "stencil", reserve=False)
        mp = at((xr + 0.06, 0.0, 0.35), (1, 0, 0))
        D.plate(mp, 0.0, 1.60, 0.30, 0.08, "SPARES", size=0.04)
    p.decor.append(decor)
    return p



# ----------------------------------------------------------------------------- valves

def valve_large():
    """A flanged gate valve for a 0.45 m pipe along X: two flanges (ports a and b, their faces at
    x = +-0.35), the gate body between them, a tapered bonnet and a hand wheel 0.6 m across on its
    spindle above, pointing +Y. The pipe's centreline is centreline_m above the floor (the flanges'
    lowest flat on y = 0): the kit's read-back wants every prop's lowest point on y = 0."""
    p = prop("valve_large", "A gate valve for the 0.45 m coolant pipe (pipework)",
             "floor under the pipe's centreline, at the middle of the valve; the centreline is centreline_m above it")
    cy = 0.45 / 2 + max(0.025, round(0.14 * 0.45, 4))
    body = p.box("gate_body", (-0.31, cy - 0.20, -0.15), (0.31, cy + 0.24, 0.15), "machinery")
    parts = [flange(p, "a", (-0.35, cy, 0.0), (-1, 0, 0), 0.45), flange(p, "b", (0.35, cy, 0.0), (1, 0, 0), 0.45),
             p.hull("bonnet", [(x, cy + 0.23, z) for x in (-0.20, 0.20) for z in (-0.12, 0.12)]
                    + [(x, cy + 0.58, z) for x in (-0.05, 0.05) for z in (-0.05, 0.05)], "machinery")]
    parts += wheel(p, "wheel", (0.0, 0.0), cy + 0.64, 0.30)
    p.union(body, "flanges_bonnet_wheel", parts)
    p.body = body
    p.extra["centreline_m"] = round(cy, 4)

    def decor(D):
        mf = at((0.0, cy, 0.15))
        D.text(mf, "V-101", 0.0, 0.07, 0.05, "stencil")
        D.text(mf, "DN 450", 0.0, -0.03, 0.035, "stencil")
        D.box(mf, -0.20, -0.14, 0.20, -0.10, -0.002, 0.004, "hazard")
        flange_bolts(D, p)
    p.decor.append(decor)
    return p

def valve_small():
    """A globe valve for a 0.15 m pipe along X: a hexagonal body 0.2 m across whose end faces are its
    flanges (ports a and b at x = +-0.15), a tapered bonnet, a spindle and a hand wheel 0.25 m across
    above, pointing +Y; centreline_m as valve_large."""
    p = prop("valve_small", "A globe valve for the 0.15 m pipes (pipework)",
             "floor under the pipe's centreline, at the middle of the valve; the centreline is centreline_m above it")
    dia, rim = 0.15, 0.025
    cy = dia / 2 + rim
    body = rod(p, "body", "x", (cy, 0.0), -0.15, 0.15, cy, "machinery", sides=6)
    for name, x, d in (("a", -0.15, (-1, 0, 0)), ("b", 0.15, (1, 0, 0))):
        p.ports[name] = {"at_m": r3((x, cy, 0.0)), "dir": r3(d), "dia_m": dia}
        p.flanges.append((name, at((x, cy, 0.0), d), dia / 2, cy))
    parts = [p.hull("bonnet", [(x, cy + 0.08, z) for x in (-0.05, 0.05) for z in (-0.04, 0.04)]
                    + [(x, cy + 0.18, z) for x in (-0.025, 0.025) for z in (-0.025, 0.025)], "machinery")]
    parts += wheel(p, "wheel", (0.0, 0.0), cy + 0.24, 0.125)
    p.union(body, "bonnet_wheel", parts)
    p.body = body
    p.extra["centreline_m"] = round(cy, 4)

    def decor(D):
        for R in regions(D, lambda R: R.role == "machinery" and abs(R.n.x) < 0.1 and R.origin.y < cy + 0.11):
            for x0, x1 in ((-0.15, -0.115), (0.115, 0.15)):
                strip(D, R, (1, 0, 0), x0, x1, "steel", reserve=False)
            strip(D, R, (1, 0, 0), -0.03, 0.03, "hazard", reserve=False)
        flange_bolts(D, p)
    p.decor.append(decor)
    return p



# ----------------------------------------------------------------------------- the manifest's ports

def check_ports(p):
    """Every recorded port lies on a face of the finished mesh that faces its dir: a flange face the
    page's pipe can butt onto. Refuses the build otherwise (nothing is written)."""
    bm = bmesh.new()
    bm.from_mesh(p.body.data)
    bm.normal_update()
    tris = [([P(v.co) for v in f.verts], P(f.normal).normalized()) for f in bm.faces]
    bm.free()
    bad = []
    for name, row in p.ports.items():
        c, d = Vector(row["at_m"]), Vector(row["dir"])
        ok = False
        for pts, n in tris:
            if n.dot(d) < 0.999 or abs(n.dot(c - pts[0])) > 1e-4:
                continue
            u = n.orthogonal().normalized()
            w = n.cross(u)
            q = [((v - c).dot(u), (v - c).dot(w)) for v in pts]
            s = [q[i][0] * q[(i + 1) % 3][1] - q[i][1] * q[(i + 1) % 3][0] for i in range(3)]
            if all(x >= -1e-9 for x in s) or all(x <= 1e-9 for x in s):
                ok = True
                break
        if not ok:
            bad.append(name)
    if bad:
        raise SystemExit(f"[props] {p.name}: ports not on a face facing their dir: {', '.join(bad)}")


_KIT_ROW = hs_kit.manifest_row


def manifest_row(n, p, facts, budget, digest):
    """The kit's row with this set's fields: ports after screens (checked against the mesh first), then
    extra (rail_top_m, centreline_m, port_sections_m)."""
    row = _KIT_ROW(n, p, facts, budget, digest)
    ports = getattr(p, "ports", None)
    if ports:
        check_ports(p)
    out = {}
    for k, v in row.items():
        out[k] = v
        if k == "screens" and ports:
            out["ports"] = {name: ports[name] for name in sorted(ports)}
    for k in sorted(getattr(p, "extra", {})):
        out[k] = p.extra[k]
    return out


hs_kit.manifest_row = manifest_row     # the kit's run() writes rows through this name


PROPS = {
    "heat_exchanger": heat_exchanger,
    "pressurizer": pressurizer,
    "coolant_pump": coolant_pump,
    "coolant_tank": coolant_tank,
    "fuel_dewar": fuel_dewar,
    "helium3_rack": helium3_rack,
    "fuel_processor": fuel_processor,
    "cryoplant": cryoplant,
    "vacuum_pump": vacuum_pump,
    "ash_tank": ash_tank,
    "power_converter": power_converter,
    "reactor_dressing": reactor_dressing,
    "gantry_crane": gantry_crane,
    "local_panel": local_panel,
    "control_desk": control_desk,
    "mimic_board": mimic_board,
    "tool_board": tool_board,
    "tool_chest": tool_chest,
    "parts_rack": parts_rack,
    "valve_large": valve_large,
    "valve_small": valve_small,
}


# ----------------------------------------------------------------------------- main

STATUS = ("Built (2026-10-07) by " + GENERATOR + ". Proposed plant for the Tern's engine core room (the owner, 2026-10-07: "
          "consoles, lots of pipes and heavy machinery: fuel, coolant, tanks, pumps and maintenance); no page or engine code loads "
          "them yet.")
RULES = [
    "This file is written by " + GENERATOR + "; never edit it by hand. Rebuild, and the .glb files, the atlases and this file change together.",
    "Metres. Prop space is the glTF frame: +Y up, +Z the prop's front (the side it is worked from), +X the right of someone facing it.",
    "anchor says where the origin is. 'floor, centre of the back, on the wall plane' (a WALL prop): on the floor at the centre of its back, the back flat on a wall at z = 0 (the vacuum pump's wall is the reactor port it stands against). 'floor, centre of the footprint' (a FREE prop). control_desk keeps free_console's anchor (the floor at the centre of the pedestal's back). gantry_crane's origin is under the girder's centre at the hook's lowest point, its rails' top rail_top_m above; the valves' origin is on the floor under the pipe's centreline, the centreline centreline_m above.",
    "Placing props: a FREE prop's anchor goes on the layout's floor point; a WALL prop's on the wall plane with its +Z into the room. operators_m are floor points where a crew member stands or sits to work it; control_desk's operators_m[0] is its seat, as free_console's.",
    "ports are where pipes meet the prop: at_m the centre of the flange face (in prop space), dir the unit direction the pipe leaves in, dia_m the pipe's outside diameter. A flange is modelled at every port with its outer face on at_m, so a pipe drawn from at_m along dir butts onto it; the build checks each lies on a face facing dir. port_sections_m gives a port's section where it is not round (power_converter's bus_out: a 0.5 x 0.2 m busbar duct, dia_m its width).",
    "Materials are roles: machinery, trim, bulkhead, hazard and light_panel are data/materials/materials.json layers (light_panel is emissive); screen is emissive and coloured by the page; accent (valve wheels, actuators, guards, rails) is tinted by the page with the station's colour (engineering's amber).",
    "screens are the recess floors a page draws on: centre_m on the floor, normal out of it, up the in-plane direction of height_m. shows says what: console (a main screen image), upper (half 0 or 1 of an upper image) or keys.",
    "atlas is the prop's own texture (glTF TEXCOORD_1), baked by tools/blender/hs_kit.py from data/materials/prop_atlas.json: every role in its finish (a prop's finish_of re-colours a role: lagging, magnet red, safety yellow, white cryogenic jackets) and the prop's details (lagging bands, gauges, stencils, placards, bolts, lamps). Its alpha is the glow mask.",
    "UV0 is in metres, projected per face as shipkit.js worldUv does (x, z where |n.y| > 0.75, else the face's horizontal tangent and y); divide by the material's span_m.",
    "Flat shaded (one normal per face), triangulated, one closed manifold solid per prop; faces against the floor or the wall are kept for the deck compiler to drop.",
    "triangles is counted in the .glb; the build refuses a prop over budget_triangles. sha256 is the .glb's, from the Blender and exporter versions in generator: a second build with them writes the same bytes.",
]

ENGINEERING = PropSet("engineering", "EngineeringProps", OUT, GENERATOR, PROPS, BUDGETS, STATUS, RULES, __doc__)


if __name__ == "__main__":
    run(ENGINEERING)
