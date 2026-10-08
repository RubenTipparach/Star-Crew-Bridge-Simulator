"""Star Crew's command suite furniture, modelled in Blender the hard-surface CSG way: the rooms off
the round bridge (the captain's ready room, the briefing room, the captain's quarters, the head,
the bridge locker room and the computer core), openspec/changes/command-suite design section 5.

It owns the suite props' geometry (assets/models/suite/<name>.glb) and their manifest
(assets/models/suite/props.json). It lives in tools/blender because meshes are files built by a
committed generator (CLAUDE.md section 9): this script is the source, the .glb files are its
output, and a second run writes the same bytes. It holds only the suite's props; the machinery
every prop set shares (materials, primitives, the Prop class and its CSG steps, clean, check, the
glb export and read-back, the manifest and the command line) is the hard-surface kit,
tools/blender/hs_kit.py, which tools/blender/build_bridge_props.py uses too. How to work this way
is the blender-hard-surface skill (.claude/skills/blender-hard-surface).

Run (from anywhere):
  <python with the bpy module> tools/blender/build_suite_props.py [--check] [--only a,b] [--blend out.blend]
  blender -b --factory-startup -P tools/blender/build_suite_props.py -- [--check] [--only a,b] [--blend out.blend]
  --check   build in memory and compare with the committed .glb files and props.json; write nothing
  --only    build only these props (the manifest keeps the others' rows)
  --blend   also save the scene with every cutter collection, for looking at the CSG in Blender
            (a debugging view, not a source: it is not reproducible and is not committed)

The method is the bridge props' (the skill's "cutter workflow"): block out from boxes, extruded
profiles and convex hulls; carve with named cutters (Exact solver, materials transferred from the
cutters' faces); union the pieces; chamfer the edges that catch light with a 1-segment Bevel;
cut the recesses (screens, drawers, doors, vents) last; clean, triangulate and check.

Conventions (written into props.json too):
  * Metres. Prop space is the exported glTF frame: +Y up, +Z toward the user (the prop's front:
    the side a sitter or a user faces it from, a bed's foot, a door's outside), +X the user's
    right as they face the prop.
  * The origin is on the floor at the centre of the prop's back (z = 0). A wall-standing prop
    (the anchor says "on the wall plane") has its back flat on the wall there; the briefing table's
    and the low table's back is a long edge.
  * One material per role, named for a Star Crew material (data/materials/materials.json:
    machinery, trim, bulkhead, hazard, light_panel) or `screen` (emissive, coloured by the page)
    or `accent` (upholstery, blankets and binders, tinted by the page).
  * UV0 in metres as shipkit.js worldUv; flat shaded; triangulated; one closed manifold solid
    per prop; a prop over its triangle budget (BUDGETS) is refused and nothing is written.
"""
import math
import os
import sys

import bpy  # noqa: F401  first: with the pip bpy module, mathutils exists only once bpy is imported
from mathutils import Matrix  # noqa: E402

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from hs_kit import ROOT, SHOWS, Prop, PropSet, frame, ngon, run  # noqa: E402

OUT = os.path.join(ROOT, "assets", "models", "suite")
GENERATOR = "tools/blender/build_suite_props.py"

# Triangles per prop: the budgets these props were briefed with (2026-10-06), the command-suite
# design's section 5 table.
# The armory's three (2026-10-08): the armory design's "racks and lockers about 1,500 triangles" together.
BUDGETS = {
    "briefing_table": 400,
    "wall_screen": 120,
    "desk": 300,
    "sofa": 200,
    "low_table": 60,
    "shelf": 160,
    "bed": 200,
    "wardrobe": 80,
    "wet_cell": 140,
    "toilet_stall": 200,
    "wash_counter": 220,
    "shower_stall": 160,
    "locker_bank": 200,
    "server_rack": 140,
    "workbench": 220,
    "rifle_rack": 700,
    "ammo_cabinet": 170,
    "armour_rack": 400,
}

# What a recorded screen shows: the bridge's kinds and "strip", a long thin display let into a
# table top (the briefing table's), which the page fills with a generic display.
SUITE_SHOWS = SHOWS + ("strip",)

WALL = "floor, centre of the back, on the wall plane"


def prop(name, presents, anchor=WALL):
    """A suite prop: its back at z = 0, on a wall when the anchor says so."""
    return Prop(name, presents, anchor, wall=(anchor == WALL), back_at_z0=True, shows=SUITE_SHOWS)


def edge(axis, at, tol=0.005):
    """A chamfer predicate's helper: the edge runs along axis ('x', 'y' or 'z') and its midpoint
    matches every coordinate in `at` ({'y': 0.75, 'z': 0.0}) within tol."""
    i = "xyz".index(axis)
    return lambda m, d, n1, n2: abs(d[i]) > 0.9 and all(abs(m["xyz".index(k)] - v) < tol for k, v in at.items())


# ----------------------------------------------------------------------------- the briefing room

def briefing_table():
    """A conference table for eight: a top with its corners cut in plan and a chamfered upper
    edge, a display strip let into the middle of the top, on two tapered pedestals over plinths."""
    p = prop("briefing_table", "The briefing room's table, for eight, with a display strip down its middle",
             "floor, centre of the long back edge (z = 0 is one long side, z = 1.4 the other)")
    L, D, H, t, c = 4.0, 1.4, 0.75, 0.06, 0.22
    top = p.prism("top", [(-L / 2 + c, 0.0), (L / 2 - c, 0.0), (L / 2, c), (L / 2, D - c), (L / 2 - c, D),
                          (-L / 2 + c, D), (-L / 2, D - c), (-L / 2, c)], "y", H - t, H, "trim", cap="bulkhead")
    p.chamfer(top, "top_edge", 0.02, lambda m, d, n1, n2: m.y > H - 0.005, role="trim")
    parts = []
    for i, x in enumerate((-1.15, 1.15)):
        ped = p.hull(f"pedestal_{i}", [(x + sx * 0.17, 0.04, D / 2 + sz * 0.34) for sx in (-1, 1) for sz in (-1, 1)]
                     + [(x + sx * 0.10, H - t + 0.01, D / 2 + sz * 0.24) for sx in (-1, 1) for sz in (-1, 1)], "machinery")
        plinth = p.box(f"plinth_{i}", (x - 0.30, 0.0, D / 2 - 0.48), (x + 0.30, 0.05, D / 2 + 0.48), "trim")
        p.chamfer(plinth, f"plinth_{i}_edges", 0.015, lambda m, d, n1, n2: m.y > 0.03)
        parts += [ped, plinth]
    p.union(top, "pedestals", parts)
    screens = []
    p.recess(screens, "strip", frame((0.0, H, D / 2), 90.0), 3.0, 0.30, 0.015, shows="strip")
    p.cut(top, "screens", screens)
    p.body = top
    for z, yaw in ((-0.55, 0.0), (D + 0.55, 180.0)):
        for x in (-1.5, -0.5, 0.5, 1.5):
            p.operators.append([x, 0.0, z])
    p.operators_yaw = [0.0] * 4 + [180.0] * 4
    return p


def wall_screen():
    """A wall-mounted display, floor to 2.6 m: a slim lower panel with two speaker grilles, a lit
    ledge under the screen section and the screen itself let into a dark bezel from 1.0 m up."""
    p = prop("wall_screen", "The briefing room's wall screen, 3.0 x 1.6 m, on the aft wall")
    W, H = 3.0, 2.6
    body = p.prism("body", [(0.0, 0.0), (0.06, 0.0), (0.06, 0.94), (0.10, 0.98), (0.10, H), (0.0, H)], "x", -W / 2, W / 2,
                   ["machinery", "bulkhead", "light_panel", "machinery", "machinery", "machinery"], cap="machinery")
    p.chamfer(body, "bezel_edges", 0.015, lambda m, d, n1, n2: m.z > 0.09 and m.y > 1.0)
    screens = []
    p.recess(screens, "screen", frame((0.0, 1.79, 0.10)), 2.88, 1.50, 0.02, shows="console")
    for i, x in enumerate((-1.0, 1.0)):
        p.recess(screens, f"grille_{i}", frame((x, 0.50, 0.06)), 0.50, 0.56, 0.012, floor_role="machinery", record=False)
    p.cut(body, "screens", screens)
    p.body = body
    return p


# ----------------------------------------------------------------------------- the ready room and quarters

def desk():
    """A desk: a top at 0.75 m on a side panel and a drawer pedestal joined by a modesty panel, a
    monitor on a stand at its back edge and a keyboard well let into the top in front of it."""
    p = prop("desk", "A desk with its monitor (the ready room's and the quarters')",
             "floor, centre of the back edge")
    W, D, H = 1.6, 0.75, 0.75
    top = p.box("top", (-W / 2, H - 0.04, 0.0), (W / 2, H, D), {"+y": "bulkhead", "*": "trim"})
    p.chamfer(top, "front_edge", 0.02, edge("x", {"y": H, "z": D}), role="trim")
    ped = p.box("pedestal", (0.32, 0.0, 0.04), (0.76, H - 0.03, D - 0.05), {"+z": "bulkhead", "*": "machinery"})
    leg = p.box("side_panel", (-0.78, 0.0, 0.04), (-0.74, H - 0.03, D - 0.05), "machinery")
    modesty = p.box("modesty_panel", (-0.75, 0.30, 0.06), (0.33, H - 0.03, 0.09), "machinery")
    foot = p.box("stand_foot", (-0.14, H - 0.01, 0.05), (0.14, H + 0.015, 0.23), "trim")
    p.chamfer(foot, "stand_foot_edges", 0.008, lambda m, d, n1, n2: m.y > H + 0.01)
    neck = p.box("stand_neck", (-0.03, H + 0.005, 0.09), (0.03, 0.92, 0.13), "trim")
    monitor = p.box("monitor", (-0.40, 0.83, 0.12), (0.40, 1.25, 0.17), "machinery")
    p.union(top, "frame_and_monitor", [ped, leg, modesty, foot, neck, monitor])
    screens = []
    p.recess(screens, "monitor", frame((0.0, 1.04, 0.17)), 0.74, 0.36, 0.012, shows="console")
    p.recess(screens, "keys", frame((0.0, H, 0.47), 90.0), 0.60, 0.20, 0.012, shows="keys")
    for i, (y0, y1) in enumerate(((0.06, 0.25), (0.27, 0.46), (0.48, 0.68))):
        p.recess(screens, f"drawer_{i}", frame((0.54, (y0 + y1) / 2, D - 0.05)), 0.38, y1 - y0, 0.012,
                 floor_role="trim", record=False)
    p.cut(top, "recesses", screens)
    p.body = top
    p.operators.append([0.0, 0.0, 0.95])
    return p


def sofa():
    """A three-seat sofa: one profile for the base and the leaning back, arms at each end, three
    seat cushions (top at 0.45 m) and three back cushions in the page's accent."""
    p = prop("sofa", "The ready room's sofa, three seats")
    top = 0.85
    lean = math.atan2(0.06, top - 0.36)
    body = p.prism("body", [(0.0, 0.0), (0.70, 0.0), (0.70, 0.07), (0.78, 0.10), (0.78, 0.36), (0.24, 0.36),
                            (0.18, top), (0.0, top)], "x", -0.86, 0.86,
                   ["machinery", "machinery", "machinery", "bulkhead", "bulkhead", "bulkhead", "trim", "bulkhead"],
                   cap="bulkhead")
    arms = []
    for s in (-1, 1):
        xa, xb = sorted((s * 0.85, s * 1.0))
        arm = p.prism(f"arm_{s:+d}", [(0.0, 0.0), (0.85, 0.0), (0.85, 0.58), (0.82, 0.62), (0.0, 0.62)], "x", xa, xb,
                      ["machinery", "bulkhead", "trim", "trim", "bulkhead"], cap="bulkhead")
        p.chamfer(arm, f"arm_{s:+d}_edge", 0.015, lambda m, d, n1, n2, s=s: m.y > 0.61 and m.x * s > 0.99 and abs(d.z) > 0.9)
        arms.append(arm)
    cushions = []
    spans = ((-0.855, -0.29), (-0.275, 0.275), (0.29, 0.855))
    for i, (x0, x1) in enumerate(spans):
        seat = p.box(f"seat_{i}", (x0, 0.35, 0.22), (x1, 0.45, 0.80), "accent")
        p.chamfer(seat, f"seat_{i}_edge", 0.02, edge("x", {"y": 0.45, "z": 0.80}))
        cushions.append(seat)
        m = frame((0.0, 0.44, 0.24 - (0.44 - 0.36) * math.tan(lean)), math.degrees(lean))
        bx0, bx1 = x0 + (0.01 if i > 0 else 0.0), x1 - (0.01 if i < 2 else 0.0)   # the outer ends reach into the arms
        cushions.append(p.box(f"back_{i}", (bx0, -0.03, -0.01), (bx1, 0.37, 0.10), "accent", m=m))
    p.union(body, "arms_and_cushions", arms + cushions)
    p.body = body
    return p


def low_table():
    """A coffee table: one profile, a top on two slab legs set in from its ends, the top's long
    upper edges chamfered."""
    p = prop("low_table", "The ready room's low table", "floor, centre of the back long edge")
    H, t = 0.42, 0.04
    pts = [(-0.55, H), (0.55, H), (0.55, H - t), (0.52, H - t), (0.50, 0.0), (0.45, 0.0), (0.46, H - t),
           (-0.46, H - t), (-0.45, 0.0), (-0.50, 0.0), (-0.52, H - t), (-0.55, H - t)]
    body = p.prism("body", pts, "z", 0.0, 0.6, ["bulkhead", "trim", "trim", "trim", "trim", "trim", "machinery", "trim",
                                                  "trim", "trim", "trim", "trim"], cap="trim")
    p.chamfer(body, "top_edges", 0.015, lambda m, d, n1, n2: m.y > H - 0.005 and abs(d.x) > 0.9)
    p.body = body
    return p


def shelf():
    """Open wall shelving: a carcass with four bays carved out of it (four shelves over a plinth),
    and boxes and binders standing on the shelves."""
    p = prop("shelf", "Open shelving (the ready room, the briefing room and the bridge locker)")
    W, H, D = 1.2, 2.0, 0.4
    body = p.box("carcass", (-W / 2, 0.0, 0.0), (W / 2, H, D), "trim")
    shelves = (0.12, 0.58, 1.04, 1.50)
    bays = [p.box(f"bay_{i}", (-W / 2 + 0.03, y, 0.02), (W / 2 - 0.03, (shelves + (H,))[i + 1] - 0.03, D + 0.05),
                  {"-z": "bulkhead", "*": "machinery"}) for i, y in enumerate(shelves)]
    p.cut(body, "bays", bays)
    # Items stand on a shelf (1 cm into it) and, all but the top one, against a side of their bay
    # (1 cm into it): a notch in two faces costs fewer triangles than a hole in one.
    side = W / 2 - 0.02
    items = [
        p.box("crate_0", (-side, shelves[0] - 0.01, 0.05), (-0.12, shelves[0] + 0.28, 0.35), "bulkhead"),
        p.box("binders_1", (-side, shelves[1] - 0.01, 0.06), (-0.28, shelves[1] + 0.31, 0.33), "accent"),
        p.box("box_1", (0.16, shelves[1] - 0.01, 0.08), (side, shelves[1] + 0.17, 0.34), "trim"),
        p.box("binders_2", (0.30, shelves[2] - 0.01, 0.06), (side, shelves[2] + 0.31, 0.33), "accent"),
        p.box("crate_3", (-0.30, shelves[3] - 0.01, 0.06), (0.16, shelves[3] + 0.22, 0.34), "bulkhead"),
    ]
    p.union(body, "items", items)
    p.body = body
    return p


def bed():
    """The captain's bed: a headboard flat on the wall with a reading light, a base with two
    drawers a side, a mattress (top at 0.55 m), a pillow, and a blanket in the page's accent
    with the sheet turned down over it."""
    p = prop("bed", "The captain's bed, its head on the wall")
    head = p.box("headboard", (-0.70, 0.0, 0.0), (0.70, 1.0, 0.08), {"+z": "bulkhead", "*": "trim"})
    p.chamfer(head, "headboard_edge", 0.02, edge("x", {"y": 1.0, "z": 0.08}), role="trim")
    base = p.box("base", (-0.68, 0.0, 0.07), (0.68, 0.38, 2.10), "machinery")
    p.chamfer(base, "base_edge", 0.015, edge("x", {"y": 0.38, "z": 2.10}))
    mattress = p.box("mattress", (-0.65, 0.37, 0.07), (0.65, 0.55, 2.07), "trim")
    blanket = p.box("blanket", (-0.665, 0.37, 0.86), (0.665, 0.57, 2.085), "accent")
    p.chamfer(blanket, "blanket_edge", 0.02, edge("x", {"y": 0.57, "z": 2.085}))
    sheet = p.box("sheet", (-0.675, 0.37, 0.78), (0.675, 0.585, 0.88), "trim")
    pillow = p.box("pillow", (-0.42, 0.54, 0.12), (0.42, 0.66, 0.46), "trim")
    p.chamfer(pillow, "pillow_edges", 0.02, lambda m, d, n1, n2: m.y > 0.65)
    p.union(head, "bed", [base, mattress, blanket, sheet, pillow])
    recesses = []
    for s in (-1, 1):
        for i, zc in enumerate((0.55, 1.55)):
            p.recess(recesses, f"drawer_{s:+d}_{i}", frame((s * 0.68, 0.215, zc), 0.0, s * 90.0), 0.86, 0.22, 0.012,
                     floor_role="bulkhead", record=False)
    p.recess(recesses, "reading_light", frame((0.0, 0.89, 0.08)), 1.0, 0.05, 0.012, floor_role="light_panel", record=False)
    p.cut(head, "recesses", recesses)
    p.body = head
    return p


def wardrobe():
    """A two-door locker: one profile with a toe kick and a vent band across its top, the doors
    split by a groove, a handle bar beside it on each door."""
    p = prop("wardrobe", "The captain's wardrobe, two doors")
    body = p.prism("body", [(0.0, 0.0), (0.55, 0.0), (0.55, 0.10), (0.60, 0.13), (0.60, 1.94), (0.56, 2.00), (0.56, 2.10),
                            (0.0, 2.10)], "x", -0.6, 0.6,
                   ["machinery", "machinery", "machinery", "bulkhead", "machinery", "bulkhead", "trim", "bulkhead"],
                   cap="bulkhead")
    handles = [p.prism(f"handle_{s:+d}", [(s * 0.035 - 0.012, 0.59), (s * 0.035 + 0.012, 0.59), (s * 0.035, 0.615)],
                       "y", 0.92, 1.28, "trim") for s in (-1, 1)]
    p.union(body, "handles", handles)
    p.cut(body, "door_split", [p.box("door_split", (-0.006, 0.16, 0.588), (0.006, 1.91, 0.65), "machinery")])
    p.body = body
    return p


def wet_cell():
    """A prefabricated en-suite shower and toilet pod: a box with its vertical edges cut in plan,
    a chamfered roof edge, a closed sliding door let into the front with its handle and the rail
    it slides on, a vent let into the roof."""
    p = prop("wet_cell", "The captain's en-suite pod, shower and toilet", "floor, centre of the back")
    W, H, D, c = 1.8, 2.3, 1.79, 0.12          # the rail and handle stand 2 cm proud: 1.81 m deep overall
    body = p.prism("body", [(-W / 2 + c, 0.0), (W / 2 - c, 0.0), (W / 2, c), (W / 2, D - c), (W / 2 - c, D),
                            (-W / 2 + c, D), (-W / 2, D - c), (-W / 2, c)], "y", 0.0, H, "bulkhead", cap="trim")
    p.chamfer(body, "roof_edge", 0.03, lambda m, d, n1, n2: m.y > H - 0.005, role="trim")
    rail = p.box("rail", (-0.66, 2.10, D - 0.01), (0.66, 2.15, D + 0.02), "trim")
    p.union(body, "rail", [rail])
    recesses = []
    p.recess(recesses, "door", frame((-0.25, 1.07, D)), 0.80, 1.98, 0.015, floor_role="trim", record=False)
    p.recess(recesses, "vent", frame((0.45, H, 0.45), 90.0), 0.40, 0.40, 0.03, floor_role="machinery", record=False)
    p.cut(body, "recesses", recesses)
    p.union(body, "handle", [p.box("handle", (0.05, 0.90, D - 0.025), (0.08, 1.30, D + 0.015), "machinery")])
    p.body = body
    return p


# ----------------------------------------------------------------------------- the head

def toilet_stall():
    """A toilet cubicle: partitions 3 cm thick standing 0.15 m off the floor, open at the top and
    to the floor (one closed solid all the same, a tube), two pilasters to the floor at the front
    with the closed door set 4 cm back between them, a handle and an occupied light on the door,
    and the toilet inside on the back wall, seen from above in a cutaway."""
    p = prop("toilet_stall", "A toilet cubicle, door closed")
    shell = p.box("partitions", (-0.48, 0.15, 0.0), (0.48, 1.98, 1.46), {"+z": "trim", "+y": "trim", "*": "bulkhead"})
    posts = [p.box(f"pilaster_{s:+d}", (min(s * 0.40, s * 0.50), 0.0, 1.40), (max(s * 0.40, s * 0.50), 2.0, 1.50), "machinery")
             for s in (-1, 1)]
    for s, post in zip((-1, 1), posts):
        p.chamfer(post, f"pilaster_{s:+d}_edges", 0.012, lambda m, d, n1, n2: m.z > 1.49 and abs(d.y) > 0.9)
    p.union(shell, "pilasters", posts)
    p.cut(shell, "inside", [p.box("inside", (-0.45, 0.05, 0.03), (0.45, 2.1, 1.39), "bulkhead")])
    cistern = p.box("cistern", (-0.20, 0.40, 0.02), (0.20, 0.80, 0.19), "trim")
    bowl = p.hull("bowl", [(sx * 0.13, 0.0, z) for sx in (-1, 1) for z in (0.17, 0.52)]
                  + [(sx * 0.19, 0.42, z) for sx in (-1, 1) for z in (0.12, 0.66)], "trim")
    p.union(shell, "toilet", [cistern, bowl])
    recesses = []
    p.recess(recesses, "door_panel", frame((0.0, 1.07, 1.46)), 0.62, 1.60, 0.012, floor_role="trim",
             wall_role="bulkhead", record=False)
    p.recess(recesses, "occupied", frame((0.355, 1.24, 1.46)), 0.05, 0.035, 0.008, floor_role="accent", record=False)
    p.cut(shell, "recesses", recesses)
    p.union(shell, "handle", [p.box("handle", (0.335, 0.98, 1.45), (0.375, 1.06, 1.49), "machinery")])
    p.body = shell
    return p


def wash_counter():
    """A wash counter: a cabinet with a lit toe kick and two doors, a dark top with a chamfered
    front edge, two basins carved into it with a tap behind each, and a backboard rising to a
    mirror ringed by a lit chamfer."""
    p = prop("wash_counter", "The head's wash counter, two basins and a mirror")
    W = 2.4
    cab = p.prism("cabinet", [(0.0, 0.0), (0.50, 0.0), (0.50, 0.10), (0.54, 0.12), (0.54, 0.80), (0.0, 0.80)], "x",
                  -W / 2 + 0.02, W / 2 - 0.02, ["machinery", "machinery", "light_panel", "bulkhead", "machinery", "bulkhead"],
                  cap="bulkhead")
    top = p.box("top", (-W / 2, 0.79, 0.0), (W / 2, 0.85, 0.60), "machinery")
    p.chamfer(top, "top_edge", 0.02, edge("x", {"y": 0.85, "z": 0.60}))
    board = p.box("backboard", (-W / 2 + 0.02, 0.84, 0.0), (W / 2 - 0.02, 2.0, 0.03), "bulkhead")
    mirror = p.box("mirror", (-1.08, 1.10, 0.02), (1.08, 1.90, 0.055), "trim")
    p.chamfer(mirror, "mirror_glow", 0.02, lambda m, d, n1, n2: m.z > 0.05, role="light_panel")
    p.union(cab, "top_and_mirror", [top, board, mirror])
    basins = [p.hull(f"basin_{i}", [(x + sx * 0.24, 0.90, 0.31 + sz * 0.17) for sx in (-1, 1) for sz in (-1, 1)]
                     + [(x + sx * 0.17, 0.71, 0.31 + sz * 0.11) for sx in (-1, 1) for sz in (-1, 1)], "trim")
              for i, x in enumerate((-0.6, 0.6))]
    p.cut(cab, "basins", basins)
    taps = [p.hull(f"tap_{i}", [(x + sx * 0.025, 0.84, z) for sx in (-1, 1) for z in (0.04, 0.10)]
                   + [(x + sx * 0.02, 1.02, z) for sx in (-1, 1) for z in (0.04, 0.09)]
                   + [(x + sx * 0.015, y, 0.21) for sx in (-1, 1) for y in (0.985, 1.01)], "trim")
            for i, x in enumerate((-0.6, 0.6))]
    p.union(cab, "taps", taps)
    doors = []
    for i, x in enumerate((-0.6, 0.6)):
        p.recess(doors, f"door_{i}", frame((x, 0.46, 0.54)), 0.92, 0.56, 0.012, floor_role="bulkhead", record=False)
    p.cut(cab, "doors", doors)
    p.body = cab
    return p


def shower_stall():
    """A shower: a box carved open at the front and top into a tray, two side walls and a back,
    the tray's floor sunk 5 cm, a rail across the top of the open front, a shower head on an arm
    from the back wall and a light strip along its top."""
    p = prop("shower_stall", "A shower stall, open front")
    W, H, D = 1.0, 2.2, 1.0
    body = p.box("shell", (-W / 2, 0.0, 0.0), (W / 2, H, D), {"+y": "trim", "*": "bulkhead"})
    p.chamfer(body, "front_edges", 0.015, lambda m, d, n1, n2: m.z > D - 0.005 and abs(d.y) > 0.9)
    p.cut(body, "stall", [p.box("stall", (-0.46, 0.12, 0.04), (0.46, H + 0.1, D + 0.05), {"-y": "machinery", "*": "trim"})])
    p.cut(body, "tray", [p.box("tray", (-0.42, 0.07, 0.08), (0.42, 0.20, D - 0.04), "machinery")])
    rail = p.box("rail", (-0.47, 2.08, D - 0.08), (0.47, 2.12, D - 0.05), "machinery")
    arm = p.box("arm", (-0.015, 1.90, 0.03), (0.015, 1.93, 0.29), "machinery")
    head = p.prism("head", ngon(0.0, 0.30, 0.08, 6), "y", 1.86, 1.91, "machinery")
    p.union(body, "fittings", [rail, arm, head])
    lights = []
    p.recess(lights, "light", frame((0.0, 2.05, 0.04)), 0.80, 0.05, 0.012, floor_role="light_panel", record=False)
    p.cut(body, "light", lights)
    p.body = body
    return p


def locker_bank():
    """Four tall lockers for suits and gear: one profile with a hazard-striped plinth and a
    chamfered top, the doors split by V-grooves, a window (the suit behind it in the page's
    accent) and a vent slot on each door."""
    p = prop("locker_bank", "A bank of four tall lockers (EVA suits in the bridge locker, gear in the head)")
    W, D = 2.4, 0.55
    body = p.prism("body", [(0.0, 0.0), (0.50, 0.0), (0.50, 0.12), (D, 0.12), (D, 2.04), (0.51, 2.10), (0.0, 2.10)], "x",
                   -W / 2, W / 2, ["machinery", "hazard", "machinery", "bulkhead", "trim", "machinery", "bulkhead"],
                   cap="bulkhead")
    # The doors are split by V-grooves 12 mm wide and deep at the face (a V is four faces, a
    # box groove five), and each door has a window and a vent slot whose top shades it (a wedge).
    cuts = [p.prism(f"door_split_{i}", [(x - 0.031, D + 0.05), (x + 0.031, D + 0.05), (x, D - 0.012)], "y", 0.14, 2.02,
                    "machinery") for i, x in enumerate((-0.6, 0.0, 0.6))]
    for i, x in enumerate((-0.9, -0.3, 0.3, 0.9)):
        cuts.append(p.box(f"window_{i}", (x - 0.11, 1.30, D - 0.015), (x + 0.11, 1.64, D + 0.05), {"-z": "accent", "*": "machinery"}))
        cuts.append(p.prism(f"vent_{i}", [(D + 0.05, 0.24), (D + 0.05, 0.38), (D - 0.04, 0.38)], "x", x - 0.18, x + 0.18,
                            "machinery"))
    p.cut(body, "doors", cuts)
    p.body = body
    return p


# ----------------------------------------------------------------------------- the bridge locker and the core

def workbench():
    """A workbench: a top at 0.9 m on two side frames with a lower shelf between them, a pegboard
    standing on its back edge to 1.75 m with a lamp along its top and tools hung on it, and a vice
    at the front corner."""
    p = prop("workbench", "The bridge locker's workbench, for spare console boards")
    W, H, D = 1.8, 0.9, 0.75
    top = p.box("top", (-W / 2, H - 0.05, 0.0), (W / 2, H, D), {"+y": "trim", "*": "machinery"})
    p.chamfer(top, "front_edge", 0.015, edge("x", {"y": H, "z": D}), role="machinery")
    frames = [p.box(f"frame_{s:+d}", (min(s * 0.82, s * 0.87), 0.0, 0.04), (max(s * 0.82, s * 0.87), H - 0.04, D - 0.04),
                    "machinery") for s in (-1, 1)]
    shelf = p.box("shelf", (-0.83, 0.15, 0.06), (0.83, 0.18, D - 0.06), "bulkhead")
    board = p.box("pegboard", (-W / 2 + 0.02, H - 0.01, 0.0), (W / 2 - 0.02, 1.75, 0.03), "bulkhead")
    lamp = p.box("lamp", (-0.80, 1.66, 0.02), (0.80, 1.72, 0.10), {"-y": "light_panel", "*": "machinery"})
    tools = [
        p.box("wrench", (-0.02, -0.17, 0.02), (0.02, 0.17, 0.045), "trim",
              m=Matrix.Translation((-0.68, 1.25, 0.0)) @ Matrix.Rotation(math.radians(-20.0), 4, "Z")),
        p.box("hammer_handle", (-0.52, 1.06, 0.02), (-0.49, 1.38, 0.05), "accent"),
        p.box("hammer_head", (-0.58, 1.36, 0.02), (-0.43, 1.41, 0.06), "trim"),
        p.box("driver_rack", (-0.25, 1.30, 0.02), (0.15, 1.36, 0.08), "accent"),
        p.prism("cable_coil", ngon(0.50, 1.25, 0.13, 5, flat_back=False), "z", 0.02, 0.06, "machinery"),
    ]
    vice = p.box("vice", (-0.78, H - 0.01, D - 0.22), (-0.58, H + 0.10, D - 0.02), "machinery")
    p.union(top, "bench", frames + [shelf, board, lamp, vice] + tools)
    p.body = top
    return p


def server_rack():
    """A computer core rack: a cabinet with its front corners cut, one lit as a vertical strip, a
    toe kick, a front grille of six louvre slots, and a cable tray along its top."""
    p = prop("server_rack", "A computer core rack, its front at z = 1.1", "floor, centre of the back (the side away from the front)")
    W, H, D = 0.8, 2.0, 1.1
    body = p.prism("cabinet", [(-W / 2, 0.0), (W / 2, 0.0), (W / 2, D - 0.04), (W / 2 - 0.04, D), (-W / 2 + 0.04, D),
                               (-W / 2, D - 0.04)], "y", 0.0, H,
                   ["machinery", "machinery", "light_panel", "bulkhead", "machinery", "machinery"], cap="trim")
    cuts = [p.prism("toe_kick", [(D - 0.06, -0.05), (D + 0.05, -0.05), (D + 0.05, 0.08), (D - 0.06, 0.08)], "x",
                    -W / 2 - 0.05, W / 2 + 0.05, "machinery")]
    for i in range(6):
        y0 = 0.30 + i * 0.25
        cuts.append(p.prism(f"slot_{i}", [(D + 0.05, y0), (D + 0.05, y0 + 0.09), (D - 0.04, y0 + 0.045)], "x", -0.27, 0.27,
                            "machinery"))
    p.cut(body, "grille", cuts)
    tray = p.box("tray", (-0.16, H - 0.01, 0.06), (0.16, 2.10, D - 0.06), "trim")
    p.cut(tray, "tray_channel", [p.box("tray_channel", (-0.13, H + 0.03, 0.0), (0.13, 2.2, D), "machinery")])
    p.union(body, "tray", [tray])
    p.body = body
    return p


# ----------------------------------------------------------------------------- the armory (openspec/changes/armory)

def rifle(p, i, x):
    """One rifle standing muzzle up in a rack, its side to the room: a side profile in x (across the rifle) and y
    (along it), 50 mm thick, its butt in the rack's base and its handguard through the locking bar."""
    prof = [(-0.03, 0.04), (0.07, 0.04), (0.03, 0.42), (0.10, 0.47), (0.10, 0.51), (0.03, 0.53), (0.12, 0.61),
            (0.12, 0.67), (0.03, 0.69), (0.02, 0.96), (0.008, 1.14), (-0.012, 1.14), (-0.045, 0.92), (-0.045, 0.40)]
    roles = ["machinery", "machinery", "machinery", "trim", "machinery", "machinery", "trim", "machinery", "machinery",
             "trim", "trim", "trim", "machinery", "machinery"]
    return p.prism(f"rifle_{i}", [(x + u, v) for u, v in prof], "z", 0.095, 0.145, roles, cap="machinery")


def rifle_rack():
    """A rifle rack for six: a backboard, a base trough the butts stand in, a locking bar across the handguards
    (hazard-striped, its lock lit), a shelf of magazines above and six rifles standing muzzle up, sides to the room."""
    p = prop("rifle_rack", "The armory's rifle rack: six rifles behind a locking bar")
    W, D = 1.9, 0.26
    board = p.box("backboard", (-W / 2, 0.0, 0.0), (W / 2, 1.85, 0.04), "bulkhead")
    base = p.box("base", (-W / 2, 0.0, 0.04), (W / 2, 0.10, D), {"+y": "machinery", "*": "trim"})
    sides = [p.box(f"side_{s:+d}", (min(s * W / 2, s * (W / 2 - 0.04)), 0.0, 0.04), (max(s * W / 2, s * (W / 2 - 0.04)), 1.50, D),
                   "trim") for s in (-1, 1)]
    bar = p.box("lock_bar", (-W / 2 + 0.04, 0.80, 0.04), (W / 2 - 0.04, 0.87, 0.22), {"+z": "hazard", "*": "trim"})
    lock = p.box("lock", (0.80, 0.78, 0.20), (0.90, 0.89, 0.25), {"+z": "light_panel", "*": "machinery"})
    shelf = p.box("shelf", (-W / 2, 1.46, 0.04), (W / 2, 1.50, D), "trim")
    mags = [p.box(f"mags_{k}", (-0.82 + 0.33 * k, 1.50, 0.07), (-0.62 + 0.33 * k, 1.62, 0.22), "machinery") for k in range(6)]
    guns = [rifle(p, i, -0.80 + 0.32 * i) for i in range(6)]
    p.union(board, "rack", [base, bar, lock, shelf] + sides + mags + guns)
    p.chamfer(board, "front_edges", 0.012, edge("x", {"y": 0.10, "z": D}))
    p.body = board
    return p


def ammo_cabinet():
    """A steel ammunition cabinet: two doors split by a V-groove, a hazard plinth, a keypad lit by its lamp, louvre
    slots low on each door and a chamfered top."""
    p = prop("ammo_cabinet", "The armory's ammunition cabinet, locked by a keypad")
    W, D = 1.0, 0.50
    body = p.prism("body", [(0.0, 0.0), (0.45, 0.0), (0.45, 0.10), (D, 0.10), (D, 1.84), (0.46, 1.90), (0.0, 1.90)], "x",
                   -W / 2, W / 2, ["machinery", "hazard", "machinery", "bulkhead", "trim", "machinery", "bulkhead"], cap="bulkhead")
    cuts = [p.prism("door_split", [(-0.031, D + 0.05), (0.031, D + 0.05), (0.0, D - 0.012)], "y", 0.12, 1.82, "machinery")]
    for i, x in enumerate((-0.25, 0.25)):
        cuts.append(p.prism(f"vent_{i}", [(D + 0.05, 0.22), (D + 0.05, 0.36), (D - 0.04, 0.36)], "x", x - 0.16, x + 0.16, "machinery"))
    p.cut(body, "doors", cuts)
    pad = p.box("keypad", (0.06, 1.10, D - 0.01), (0.20, 1.30, D + 0.03), {"+z": "light_panel", "*": "machinery"})
    lamp = p.box("lamp", (0.08, 1.33, D - 0.01), (0.18, 1.36, D + 0.02), {"+z": "light_panel", "*": "machinery"})
    p.union(body, "keypad", [pad, lamp])
    p.body = body
    return p


def armour_rack():
    """An armour rack: a backboard with a rail of three vests on hangers (each a chest and back plate in one block,
    shoulders cut in) and a shelf of three helmets above, the rail's ends on brackets."""
    p = prop("armour_rack", "The armory's armour rack: three vests and three helmets")
    W, D = 1.8, 0.42
    board = p.box("backboard", (-W / 2, 0.0, 0.0), (W / 2, 2.0, 0.04), "bulkhead")
    rail = p.box("rail", (-W / 2 + 0.05, 1.50, 0.18), (W / 2 - 0.05, 1.54, 0.22), "trim")
    brackets = [p.box(f"bracket_{s:+d}", (s * (W / 2 - 0.05) - 0.03, 1.46, 0.04), (s * (W / 2 - 0.05) + 0.03, 1.56, 0.24), "trim")
                for s in (-1, 1)]
    shelf = p.box("shelf", (-W / 2, 1.70, 0.04), (W / 2, 1.74, D - 0.06), "trim")
    parts = [rail, shelf] + brackets
    for i, x in enumerate((-0.58, 0.0, 0.58)):
        vest = p.hull(f"vest_{i}", [(x + sx * 0.24, 0.80, z) for sx in (-1, 1) for z in (0.08, 0.30)]
                      + [(x + sx * 0.27, 1.20, z) for sx in (-1, 1) for z in (0.07, 0.31)]
                      + [(x + sx * 0.20, 1.47, z) for sx in (-1, 1) for z in (0.10, 0.28)], "accent")
        neck = p.box(f"neck_{i}", (x - 0.11, 1.30, 0.0), (x + 0.11, 1.60, 0.40), "accent")
        p.cut(vest, f"vest_{i}_neck", [neck])
        hook = p.box(f"hook_{i}", (x - 0.02, 1.40, 0.17), (x + 0.02, 1.56, 0.23), "trim")
        helmet = p.hull(f"helmet_{i}", [(x + 0.15 * math.cos(a), 1.74, 0.20 + 0.16 * math.sin(a)) for a in [k * math.pi / 4 for k in range(8)]]
                        + [(x + 0.10 * math.cos(a), 1.88, 0.20 + 0.11 * math.sin(a)) for a in [k * math.pi / 4 for k in range(8)]]
                        + [(x, 1.93, 0.20)], "machinery")
        parts += [vest, hook, helmet]
    p.union(board, "rack", parts)
    p.body = board
    return p


# ----------------------------------------------------------------------------- the atlas's details
# What each prop carries on its baked atlas besides the kit's rules (the owner, 2026-10-07: "basically
# everything using the metal tile grid needs to get replaced with custom textures"): furniture takes a
# furniture finish on its painted faces (FINISH_OF: wood, laminate, fabric, porcelain), and labels,
# placards and lamps go where they fit on the face looking the given way (hs_kit.Detail.spot).
F, BK, L, R, U = (0.0, 0.0, 1.0), (0.0, 0.0, -1.0), (-1.0, 0.0, 0.0), (1.0, 0.0, 0.0), (0.0, 1.0, 0.0)
FINISH_OF = {
    "briefing_table": {"bulkhead": "wood"},
    "desk": {"bulkhead": "wood"},
    "sofa": {"bulkhead": "fabric"},
    "low_table": {"bulkhead": "wood"},
    "shelf": {"bulkhead": "wood"},
    "bed": {"bulkhead": "wood"},
    "wardrobe": {"bulkhead": "laminate"},
    "wet_cell": {"bulkhead": "laminate"},
    "toilet_stall": {"bulkhead": "laminate"},
    "wash_counter": {"bulkhead": "laminate"},
    "shower_stall": {"bulkhead": "porcelain"},
}
DECOR = {
    "briefing_table": [],
    "wall_screen": [("placard", "BRIEFING 1", 0.42, 0.08, F, "plate", "stencil_dark", "low")],
    "desk": [],
    "sofa": [],
    "low_table": [],
    "shelf": [],
    "bed": [],
    "wardrobe": [("placard", "CAPTAIN", 0.24, 0.07, F, "plate", "stencil_dark", "high")],
    "wet_cell": [("placard", "HEAD", 0.18, 0.07, F, "plate", "stencil_dark", "high")],
    "toilet_stall": [("placard", "HEAD", 0.18, 0.07, F, "plate", "stencil_dark", "high")],
    "wash_counter": [],
    "shower_stall": [],
    "locker_bank": [("stencil", "EVA 1", 0.06, F, "stencil", "high"), ("stencil", "EVA 2", 0.06, F, "stencil", "high"),
                    ("stencil", "EVA 3", 0.06, F, "stencil", "high"), ("stencil", "EVA 4", 0.06, F, "stencil", "high"),
                    ("placard", "SUITS|CHECK SEALS", 0.30, 0.12, [L, R, F], "yellow", "stencil_dark")],
    "workbench": [("placard", "EYE PROTECTION", 0.42, 0.08, F, "yellow", "stencil_dark", "high")],
    "rifle_rack": [("placard", "SIGN OUT|SECURITY", 0.36, 0.12, F, "yellow", "stencil_dark", "high")],
    "ammo_cabinet": [("placard", "AMMUNITION", 0.40, 0.08, F, "yellow", "stencil_dark", "high"), ("stencil", "ARMS 2", 0.06, F, "stencil", "low")],
    "armour_rack": [("placard", "ARMOUR", 0.30, 0.08, F, "plate", "stencil_dark", "high")],
    "server_rack": [("stencil", "CORE 01", 0.07, F, "stencil", "high"), ("lamps", 8, F, ("led_green", "led_green", "led_blue", "led_amber"), 0.01, 0.035, "high"),
                    ("lamps", 8, F, ("led_blue", "led_green", "led_green"), 0.01, 0.035, "low"),
                    ("placard", "HIGH VOLTAGE", 0.34, 0.09, [L, R, BK])],
}


def decorated(name):
    """Build a prop with its atlas details (DECOR) and finishes (FINISH_OF) attached."""
    p = BUILDERS[name]()
    p.decor.append(lambda D: D.apply(DECOR[name]))
    p.finish_of.update(FINISH_OF.get(name, {}))
    return p


BUILDERS = {
    "briefing_table": briefing_table,
    "wall_screen": wall_screen,
    "desk": desk,
    "sofa": sofa,
    "low_table": low_table,
    "shelf": shelf,
    "bed": bed,
    "wardrobe": wardrobe,
    "wet_cell": wet_cell,
    "toilet_stall": toilet_stall,
    "wash_counter": wash_counter,
    "shower_stall": shower_stall,
    "locker_bank": locker_bank,
    "server_rack": server_rack,
    "workbench": workbench,
    "rifle_rack": rifle_rack,
    "ammo_cabinet": ammo_cabinet,
    "armour_rack": armour_rack,
}
PROPS = {n: (lambda n=n: decorated(n)) for n in BUILDERS}


# ----------------------------------------------------------------------------- main

STATUS = ("Built (2026-10-06) by " + GENERATOR + ". Proposed furniture for the command suite's rooms "
          "(openspec/changes/command-suite design section 5, deck-pipeline section 5); no page or engine code loads them yet.")
RULES = [
    "This file is written by " + GENERATOR + "; never edit it by hand. Rebuild, and the .glb files and this file change together.",
    "Metres. Prop space is the glTF frame: +Y up, +Z toward the user (the prop's front: the side it is used or sat at, a bed's foot, a door's outside), +X the user's right as they face the prop.",
    "The origin is on the floor at the centre of the prop's back (z = 0). An anchor 'on the wall plane' means the back stands flat on a wall there; the briefing table's and the low table's back is one long edge.",
    "Placing props (command_suite.json furnishings): back_m is the anchor and yaw_deg the way the prop's +Z faces. operators_m are floor points where a user sits or stands, and operators_yaw_deg (where given) the way each one faces in prop space, 0 toward +Z and 180 toward -Z.",
    "Materials are roles: machinery, trim, bulkhead, hazard and light_panel are data/materials/materials.json layers; screen is emissive and coloured by the page; accent (upholstery, blankets, binders, a suit behind a locker window) is tinted by the page.",
    "screens are the recess floors a page draws on: centre_m on the floor, normal out of it, up the in-plane direction of height_m (width_m runs along up x normal). shows says what: console (a main screen image, assets/textures/screens/screens.json), keys (a key panel) or strip (a long table display the page fills with a generic display).",
    "UV0 is in metres, projected per face as shipkit.js worldUv does (x, z where |n.y| > 0.75, else the face's horizontal tangent and y); divide by the material's span_m.",
    "Flat shaded (one normal per face), triangulated, one closed manifold solid per prop; faces against the floor or the wall are kept for the deck compiler to drop.",
    "triangles is counted in the .glb; the build refuses a prop over budget_triangles. sha256 is the .glb's, from the Blender and exporter versions in generator: a second build with them writes the same bytes.",
    "atlas is the prop's own baked texture (assets/models/<set>/atlas/<prop>.png, written by the build with its sha256): 256 x 256 RGBA, alpha the glow mask, read through the glb's second UV map (TEXCOORD_1; TEXCOORD_0 stays the metre UVs). px_per_m is its texel density, charts how many pieces its surface was cut into. data/materials/prop_atlas.json says how it is baked; screen faces carry no content in it (the page draws the console faces), accent faces bake light and neutral for the page to tint.",
]

SUITE = PropSet("suite", "SuiteProps", OUT, GENERATOR, PROPS, BUDGETS, STATUS, RULES, __doc__)


if __name__ == "__main__":
    run(SUITE)
