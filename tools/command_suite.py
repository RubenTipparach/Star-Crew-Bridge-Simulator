#!/usr/bin/env python3
"""Write the Tern's command suite (data/ships/tern/command_suite.json) and check it.

The owner, 2026-10-06, after the bridge variants (bridge-stations design 11a): "can we do the more
circular bridge? but have like side rooms for meetings, captains quarters and stuff?". That picks
variant B, the round bridge, and asks for rooms around it, as in R2 of
docs/analysis/star-trek-bridges.md (a round main room with a briefing room attached on one side).

This is documentation tooling and a measurement instrument (CLAUDE.md section 4): it computes the
proposed change to the layout for openspec/changes/command-suite, writes it as a patch the
command deck mockup reads, and checks it by applying the patch to a copy of the layout in memory
and running tools/layout_check.py's checks on the copy. It never edits the layout; applying the
patch is that change's task 2.

What the patch does to deck A, the command deck (port is +x):

  * the bridge becomes variant B (tools/bridge_variants.py, the one copy of its geometry), with
    the ring starting further forward so a door in each aft diagonal wall opens at walkway level,
    and helm and tactical at two desks 1.5 m either side of the centreline (11a's fix);
  * the captain's ready room keeps its room and grows forward into the corner between the round
    bridge and the hull, where its new door opens onto the bridge;
  * the briefing room takes the computer core's place, mirrored, with its own door onto the bridge;
  * the computer core moves aft to z 9-14 starboard; the captain's quarters take z 9-14 port, with
    a door into the ready room; a head (washroom) and a bridge locker take z 4-9;
  * the rooms are furnished from the suite props (assets/models/suite, tools/blender/build_suite_props.py).

Usage: python3 tools/command_suite.py [--check] [--sightlines]
  --check       writes nothing, fails if the data file is stale or a check fails
  --sightlines  prints the suite bridge's sightlines to the viewscreen (as bridge_variants.py does)
Standard library only.
"""

import copy
import io
import json
import math
import os
import sys
from contextlib import redirect_stdout

sys.dont_write_bytecode = True
ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.join(ROOT, "tools"))
import bridge_variants as BV  # noqa: E402
import layout_check  # noqa: E402

LAYOUT = BV.LAYOUT
OUT = os.path.join(ROOT, "data", "ships", "tern", "command_suite.json")
SUITE_PROPS = os.path.join(ROOT, "assets", "models", "suite", "props.json")

FLOOR = BV.FLOOR          # deck A floor, 3.5 m
ROOM_TOP = 6.5            # the side rooms' ceiling (deck A's 3.0 m rooms; the bridge is 3.5 m)
DOOR_Y = FLOOR + 1.1      # a 2.2 m door's centre
WINDOW_Y = 5.0            # a window's centre, as the bridge's
WALK_M_S = 1.8            # crew-on-deck's walking speed
STAIR_M_S = 0.7           # crew-on-deck's speed along a flight (walk_times.py STAIR)
LADDER_DOWN_M_S, LADDER_MOUNT_S = 1.0, 0.5   # walk_times.py DOWN and MOUNT (to get on, and again to get off)
DOOR_CLEAR_M = 1.0        # furniture keeps this deep a zone clear in front of every door, both sides
GAP_M = 0.01              # furniture stands at least 1 cm off a wall it does not stand against

r3 = BV.r3


def pt(p):
    return [r3(p[0]), r3(p[1])]


def mirror_poly(poly):
    return BV.mirror_x(poly)


# ------------------------------------------------------------------ the bridge

# The suite's port ring: the door into the ready room takes the aft diagonal wall (-67.5 deg),
# so the ring starts at the next corner and the status board moves forward a segment.
SUITE_BANKS = ((-45.0, "comms", "wall_bank", True), (-22.5, "status_p", "wall_bank", False),
               (0.0, "engineering", "wall_bank_core", True), (22.5, "repeater_p", "wall_bank", False))
RING_FROM_DEG = -56.25


def suite_bridge():
    b = BV.variant_b(ring_from=RING_FROM_DEG, banks=SUITE_BANKS, helm="pair", helm_x=1.5)
    b.update({
        "id": "suite", "name": "Round, with side rooms",
        "after": "R2 (a round bridge with a briefing room on one side) and the owner's pick of B, 2026-10-06",
        "summary": ("Variant B's sixteen-sided room. The ring starts a segment further forward, so a door "
                    "in each aft diagonal wall opens from the walkway: the ready room to port, the briefing "
                    "room to starboard. Helm and tactical sit at two desks 1.5 m either side of the "
                    "centreline, so their heads clear the screen from the captain's eye (11a's fix)."),
    })
    return b


# ------------------------------------------------------------------ the side rooms

L0 = BV.LAYOUT_DOC


def comp(L, cid):
    return next(c for c in L["compartments"] if c["id"] == cid)


def raked_x(z):
    """The deck's raked outer wall: the ready room's (9.6, 14.8)-(8.9, 20.0), extended aft to z 8,
    where the hull is widest (hull section z 8), and run parallel to the skin from there to z 4."""
    a, b = (9.6, 14.8), (8.9, 20.0)
    k = (b[0] - a[0]) / (b[1] - a[1])
    if z >= 8.0:
        return a[0] + k * (z - a[1])
    x8 = a[0] + k * (8.0 - a[1])
    return x8 - 0.04 * (8.0 - z)     # the hull's half beam barely changes aft of z 8


def port_pocket(bridge_poly):
    """The ready room's new brushes: the corner between the round bridge's aft port walls and the
    deck's outer wall, cut into one convex prism per bridge wall segment (the bridge's corners are
    reflex seen from the corner, so one prism cannot take two of its walls)."""
    aft = sorted([tuple(p) for p in bridge_poly if p[0] > 0 and p[1] < 25.5], key=lambda p: p[1])
    # aft: (1.233, 20.0), (3.512, 20.944), (5.256, 22.688), (6.2, 24.967)
    out_x = lambda z: raked_x(z) if z <= 20.0 else 8.9 + (8.05 - 8.9) * (z - 20.0) / (aft[-1][1] - 20.0)
    brushes = []
    for a, b in zip(aft, aft[1:]):
        za, zb = a[1], b[1]
        poly = [a, (out_x(za), za), (out_x(zb), zb), b]
        if za == 20.0:
            poly[1] = (8.9, 20.0)
        brushes.append({"y": [FLOOR, ROOM_TOP], "poly": [pt(p) for p in poly]})
    return brushes, aft


def side_rooms(bridge):
    """The changed and new compartments: (records, the bridge's aft port corners)."""
    poly = bridge["brushes"][0]["poly"]
    pocket, aft = port_pocket(poly)
    ready0 = comp(L0, "ready_room")
    main = copy.deepcopy(ready0["brushes"][0])
    assert main["poly"] == [[1.25, 14.0], [8.8, 14.0], [9.6, 14.8], [8.9, 20.0], [1.25, 20.0]], "the ready room has moved"
    ready = copy.deepcopy(ready0)
    ready["brushes"] = [main] + pocket
    ready["purpose"] = ("The captain's office: a desk with the captain's private console, a sofa, and a window. "
                        "Its door opens onto the bridge's walkway; a second door to the command passage, a third "
                        "to the captain's quarters.")
    briefing = {
        "id": "briefing_room", "poi": 31, "name": "Briefing room", "decks": ["A"], "kind": "room", "finish": "crew",
        "purpose": ("Meetings and mission briefings: a table for eight facing a wall screen, a window. Its door "
                    "opens onto the bridge's walkway; a second door to the command passage."),
        "brushes": [{"y": b["y"], "poly": mirror_poly(b["poly"])} for b in ready["brushes"]],
    }
    x9, x14, x4, x8 = raked_x(9.0), raked_x(14.0), raked_x(4.8), raked_x(8.0)
    quarters_poly = [(1.25, 9.0), (x9, 9.0), (x14, 14.0), (1.25, 14.0)]
    aft_poly = [(1.25, 4.0), (x4 - 0.8, 4.0), (x4, 4.8), (x8, 8.0), (x9, 9.0), (1.25, 9.0)]
    quarters = {
        "id": "captains_quarters", "poi": 32, "name": "Captain's quarters", "decks": ["A"], "kind": "room", "finish": "crew",
        "purpose": "The captain's cabin: a bed, a wardrobe, a desk and an en-suite. Doors to the command passage and the ready room.",
        "brushes": [{"y": [FLOOR, ROOM_TOP], "poly": [pt(p) for p in quarters_poly]}],
    }
    head = {
        "id": "head", "poi": 33, "name": "Head", "decks": ["A"], "kind": "room", "finish": "crew",
        "purpose": "The command deck's washroom: two toilets, two showers, a wash counter and lockers.",
        "brushes": [{"y": [FLOOR, ROOM_TOP], "poly": [pt(p) for p in aft_poly]}],
    }
    core0 = comp(L0, "computer_core")
    core = copy.deepcopy(core0)
    core["brushes"] = [{"y": [FLOOR, ROOM_TOP], "poly": mirror_poly([pt(p) for p in quarters_poly])}]
    core["purpose"] = core0["purpose"] + " Two rows of racks; moved aft of the briefing room (command-suite)."
    locker = {
        "id": "bridge_locker", "poi": 34, "name": "Bridge locker", "decks": ["A"], "kind": "room", "finish": "working",
        "purpose": "The bridge crew's EVA suits and emergency kit, spare console boards and a workbench.",
        "brushes": [{"y": [FLOOR, ROOM_TOP], "poly": mirror_poly([pt(p) for p in aft_poly])}],
    }
    return [ready, briefing, core, quarters, head, locker], aft


def wall_door(pid, between, a, b, size, y=DOOR_Y, kind="door", at_m=None):
    """A portal on wall a->b of between[0]'s brush (positively wound), its normal out of it: centred on
    the wall, or at_m metres along it from a."""
    ln = math.hypot(b[0] - a[0], b[1] - a[1])
    t = 0.5 if at_m is None else at_m / ln
    m = (a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t)
    nx, nz = layout_check.outward_normal(a, b)
    return {"id": pid, "kind": kind, "between": between, "center_m": [r3(m[0]), r3(y), r3(m[1])],
            "normal": [round(nx, 4), 0.0, round(nz, 4)], "size_m": size}


def portals(bridge, aft):
    """Changed and new portals."""
    p = []
    # The bridge's aft port diagonal wall, (1.233, 20.0) to (3.512, 20.944). The door sits toward its
    # aft end, 0.85 m along, so the foot of the ring's stair stays out of the door's clear zone.
    a, b = aft[0], aft[1]
    door = wall_door("p_ready_bridge", ["bridge", "ready_room"], a, b, [1.2, 2.2], at_m=0.85)
    p.append(door)
    twin = copy.deepcopy(door)
    twin.update({"id": "p_briefing_bridge", "between": ["bridge", "briefing_room"]})
    twin["center_m"][0] = r3(-door["center_m"][0])
    twin["normal"][0] = -door["normal"][0]
    p.append(twin)
    side = lambda pid, between, x, z, w=1.0: {"id": pid, "kind": "door", "between": between,
                                              "center_m": [x, r3(DOOR_Y), z], "normal": [1.0 if x < 0 else -1.0, 0.0, 0.0], "size_m": [w, 2.2]}
    p.append(side("p_briefing", ["briefing_room", "a_corridor"], -1.25, 17.0, 1.2))
    p.append(side("p_core", ["computer_core", "a_corridor"], -1.25, 12.5))
    p.append(side("p_captains_quarters", ["captains_quarters", "a_corridor"], 1.25, 12.5))
    p.append(side("p_head", ["head", "a_corridor"], 1.25, 6.5))
    p.append(side("p_bridge_locker", ["bridge_locker", "a_corridor"], -1.25, 6.5))
    p.append({"id": "p_quarters_ready", "kind": "door", "between": ["captains_quarters", "ready_room"],
              "center_m": [5.6, r3(DOOR_Y), 14.0], "normal": [0.0, 0.0, 1.0], "size_m": [1.0, 2.2]})
    wa, wb = (9.6, 14.8), (8.9, 20.0)
    p.append(wall_door("p_ready_window", ["ready_room", "space"], wa, wb, [2.0, 1.0], y=WINDOW_Y, kind="window"))
    p.append(wall_door("p_briefing_window", ["briefing_room", "space"], (-wb[0], wb[1]), (-wa[0], wa[1]), [2.0, 1.0], y=WINDOW_Y, kind="window"))
    for q in bridge.get("portals", []):
        rec = copy.deepcopy(next(x for x in L0["portals"] if x["id"] == q["id"]))
        rec.update({k: v for k, v in q.items() if k != "id"})
        p.append(rec)
    return p


# ------------------------------------------------------------------ furniture

# The size the brief gave the Blender build for each piece, metres: width along the prop's x,
# height, and depth along its z, out from its back (tools/blender/build_suite_props.py). When the
# build's manifest is present the check uses its measured bounds instead, and refuses a prop more
# than 2 cm off its brief. The mockup draws a box of this size when the page has no models.
BRIEF = {
    "briefing_table": (4.0, 0.75, 1.4), "wall_screen": (3.0, 2.6, 0.1), "desk": (1.6, 1.25, 0.75),
    "sofa": (2.0, 0.85, 0.85), "low_table": (1.1, 0.42, 0.6), "shelf": (1.2, 2.0, 0.4),
    "bed": (1.4, 1.0, 2.1), "wardrobe": (1.2, 2.1, 0.6), "wet_cell": (1.8, 2.3, 1.8),
    "toilet_stall": (1.0, 2.0, 1.5), "wash_counter": (2.4, 2.0, 0.6), "shower_stall": (1.0, 2.2, 1.0),
    "locker_bank": (2.4, 2.1, 0.55), "server_rack": (0.8, 2.1, 1.1), "workbench": (1.8, 1.75, 0.75),
}
CHAIR = "crew_chair"     # the bridge set's swivel chair, at desks and the table


def furnish(room, prop, x, z, yaw, note=None, kit="suite"):
    """A piece placed by its back at floor level, facing yaw (0 the bow, +90 port), as the bridge props are."""
    f = {"room": room, "prop": prop, "set": kit, "back_m": [r3(x), FLOOR, r3(z)], "yaw_deg": float(yaw)}
    if note:
        f["note"] = note
    return f


def chair_at(room, seat_x, seat_z, yaw, who=None):
    """A crew chair whose sitter's seat point is (seat_x, seat_z), facing yaw (bridge props' crew_chair)."""
    c = BV.operator("crew_chair")[1]
    th = math.radians(yaw)
    f = furnish(room, CHAIR, seat_x - math.sin(th) * c, seat_z - math.cos(th) * c, yaw, who, kit="bridge")
    f["seat_m"] = [r3(seat_x), FLOOR, r3(seat_z)]
    return f


def furnishings():
    F = []
    # Ready room (port, z 14-20 and the corner forward of it). The captain's desk faces the room
    # from in front of the window; the sofa and a low table against the aft wall; a shelf beside the
    # door to the quarters; a shelf at the far end of the corner.
    F.append(furnish("ready_room", "desk", 6.95, 17.4, 90, "the captain's desk and private console, its back to the room"))
    F.append(chair_at("ready_room", 7.9, 17.4, -90, "the captain, facing the room with the window behind"))
    F.append(chair_at("ready_room", 5.55, 16.85, 90, "a visitor"))
    F.append(chair_at("ready_room", 5.55, 17.95, 90, "a visitor"))
    F.append(furnish("ready_room", "sofa", 3.0, 14.0, 0))
    F.append(furnish("ready_room", "low_table", 3.0, 15.25, 0))
    F.append(furnish("ready_room", "shelf", 7.7, 14.0, 0))
    F.append(furnish("ready_room", "shelf", 7.13, 24.95, 180, "at the end of the corner"))
    # Briefing room (starboard). The table runs fore and aft, eight seats, the screen on the aft wall.
    F.append(furnish("briefing_room", "briefing_table", -4.3, 17.0, -90, "its length fore and aft"))
    for z in (15.5, 16.5, 17.5, 18.5):
        F.append(chair_at("briefing_room", -3.75, z, -90))
        F.append(chair_at("briefing_room", -6.25, z, 90))
    F.append(furnish("briefing_room", "wall_screen", -5.0, 14.0, 0, "the briefing screen, down the table"))
    F.append(furnish("briefing_room", "shelf", -7.13, 24.95, 180, "at the end of the corner"))
    # Captain's quarters (port, z 9-14).
    F.append(furnish("captains_quarters", "bed", 7.0, 9.0, 0, "its head on the aft wall"))
    F.append(furnish("captains_quarters", "wardrobe", 4.4, 9.0, 0))
    F.append(furnish("captains_quarters", "desk", 3.2, 14.0, 180))
    F.append(chair_at("captains_quarters", 3.2, 12.9, 0))
    F.append(furnish("captains_quarters", "wet_cell", 8.3, 14.0, 180, "the en-suite"))
    # Head (port, z 4-9).
    F.append(furnish("head", "wash_counter", 4.0, 4.0, 0))
    F.append(furnish("head", "toilet_stall", 6.9, 4.0, 0))
    F.append(furnish("head", "toilet_stall", 8.0, 4.0, 0))
    F.append(furnish("head", "shower_stall", 7.7, 9.0, 180))
    F.append(furnish("head", "shower_stall", 8.8, 9.0, 180))
    F.append(furnish("head", "locker_bank", 3.9, 9.0, 180))
    # Bridge locker (starboard, z 4-9).
    F.append(furnish("bridge_locker", "locker_bank", -3.9, 9.0, 180, "EVA suits for the bridge crew (fixture eva_suits_a)"))
    F.append(furnish("bridge_locker", "locker_bank", -6.5, 9.0, 180, "fire and first-aid kits, breathing sets"))
    F.append(furnish("bridge_locker", "workbench", -5.0, 4.0, 0, "spare console boards"))
    F.append(furnish("bridge_locker", "shelf", -2.4, 4.0, 0))
    F.append(furnish("bridge_locker", "shelf", -7.6, 4.0, 0))
    # Computer core (starboard, z 9-14): two rows of five racks back to back, aisles in front of each.
    for x in (-3.4, -4.2, -5.0, -5.8, -6.6):
        F.append(furnish("computer_core", "server_rack", x, 11.6, 180))
        F.append(furnish("computer_core", "server_rack", x, 11.8, 0))
    return F


def footprints():
    """{prop: (min x, max x, min z, max z)} in prop space: the build's bounds when present, else the brief."""
    fp = {}
    man = None
    if os.path.exists(SUITE_PROPS):
        with open(SUITE_PROPS, encoding="utf-8") as f:
            man = json.load(f)["props"]
    for name, (w, _, d) in BRIEF.items():
        if man and name in man:
            b = man[name]["bounds_m"]
            fp[name] = (b["min"][0], b["max"][0], b["min"][2], b["max"][2])
            if abs((b["max"][0] - b["min"][0]) - w) > 0.02 or abs((b["max"][2] - b["min"][2]) - d) > 0.02:
                raise SystemExit(f"{name}: built {b['max'][0] - b['min'][0]:.2f} x {b['max'][2] - b['min'][2]:.2f} m, the brief is {w} x {d} m")
        else:
            fp[name] = (-w / 2, w / 2, 0.0, d)
    b = BV.PROPS[CHAIR]["bounds_m"]
    fp[CHAIR] = (b["min"][0], b["max"][0], b["min"][2], b["max"][2])
    return fp


def corners(f, fp):
    x0, x1, z0, z1 = fp[f["prop"]]
    th = math.radians(f["yaw_deg"])
    c, s = math.cos(th), math.sin(th)
    bx, bz = f["back_m"][0], f["back_m"][2]
    out = [(bx + x * c + z * s, bz - x * s + z * c) for x, z in ((x0, z0), (x1, z0), (x1, z1), (x0, z1))]
    if layout_check.signed_area(out) < 0:
        out.reverse()
    return out


# ------------------------------------------------------------------ patch and checks

def patch_layout(L, patch):
    """Apply the patch to a layout document in place (the one rule; the mockup reads the same fields)."""
    by = lambda lst, i: next((x for x in lst if x["id"] == i), None)
    for c in patch["compartments"]:
        old = by(L["compartments"], c["id"])
        if old:
            L["compartments"][L["compartments"].index(old)] = copy.deepcopy(c)
        else:
            L["compartments"].append(copy.deepcopy(c))
    for p in patch["portals"]:
        old = by(L["portals"], p["id"])
        if old:
            L["portals"][L["portals"].index(old)] = copy.deepcopy(p)
        else:
            L["portals"].append(copy.deepcopy(p))
    for s in patch["stations"]:
        st = by(L["stations"], s["station"])
        st["seat_m"], st["yaw_deg"] = s["seat_m"], s["yaw_deg"]
    L["fixtures"] = [f for f in L["fixtures"] if f["id"] not in patch["fixtures_removed"]]
    for f in patch["fixtures"]:
        old = by(L["fixtures"], f["id"])
        if old:
            old.update(copy.deepcopy(f))
        else:
            L["fixtures"].append(copy.deepcopy(f))
    for s in patch["systems"]:
        by(L["systems"], s["id"]).update(copy.deepcopy(s))
    return L


def run_layout_check(L):
    tmp = os.path.join(os.path.dirname(LAYOUT), ".command_suite_check.json")
    with open(tmp, "w", encoding="utf-8") as f:
        json.dump(L, f)
    buf = io.StringIO()
    try:
        with redirect_stdout(buf):
            ok = layout_check.check(tmp, quiet=False)
    finally:
        os.remove(tmp)
    return ok, buf.getvalue()


def in_room(L, room, pts, tol=0.012):
    brushes = [b["poly"] for b in comp(L, room)["brushes"]]
    return all(any(layout_check.inside_poly(b, x, z, tol=tol) for b in brushes) for x, z in pts)


def door_zones(L):
    """Each door's clear zone, DOOR_CLEAR_M deep on both sides, as (compartment, polygon)."""
    out = []
    for p in L["portals"]:
        if p["kind"] != "door" or abs(p["normal"][1]) > 0.5:
            continue
        cx, _, cz = p["center_m"]
        nx, nz = p["normal"][0], p["normal"][2]
        tx, tz = -nz, nx
        hw = p["size_m"][0] / 2 + 0.1
        for cid, sgn in ((p["between"][0], -1), (p["between"][1], 1)):
            if cid == "space":
                continue
            d = DOOR_CLEAR_M * sgn
            poly = [(cx + tx * hw, cz + tz * hw), (cx - tx * hw, cz - tz * hw),
                    (cx - tx * hw + nx * d, cz - tz * hw + nz * d), (cx + tx * hw + nx * d, cz + tz * hw + nz * d)]
            if layout_check.signed_area(poly) < 0:
                poly.reverse()
            out.append((p["id"], cid, poly))
    return out


def stair_poly(st):
    """A flight's footprint: width_m across the line from its top to its foot."""
    (tx, _, tz), (fx, _, fz) = st["top_m"], st["foot_m"]
    dx, dz = fx - tx, fz - tz
    ln = math.hypot(dx, dz)
    px, pz = -dz / ln * st["width_m"] / 2, dx / ln * st["width_m"] / 2
    poly = [(tx + px, tz + pz), (fx + px, fz + pz), (fx - px, fz - pz), (tx - px, tz - pz)]
    if layout_check.signed_area(poly) < 0:
        poly.reverse()
    return poly


def check_furniture(L, F, fp, stairs=()):
    problems = []
    for pid, cid, zone in door_zones(L):
        for st in stairs:
            if cid == "bridge" and layout_check.polys_overlap(stair_poly(st), zone):
                problems.append(f"stair '{st['note']}' stands in front of door {pid}")
    for f in F:
        cs = corners(f, fp)
        if not in_room(L, f["room"], cs):
            problems.append(f"{f['prop']} in {f['room']} at {f['back_m']}: not inside the room")
    solid = [f for f in F if f["prop"] != CHAIR]
    for i, a in enumerate(solid):
        for b in solid[i + 1:]:
            if a["room"] == b["room"] and layout_check.polys_overlap(corners(a, fp), corners(b, fp)):
                problems.append(f"{a['prop']} at {a['back_m']} overlaps {b['prop']} at {b['back_m']}")
    for pid, cid, zone in door_zones(L):
        for f in F:
            if f["room"] == cid and layout_check.polys_overlap(corners(f, fp), zone):
                problems.append(f"{f['prop']} at {f['back_m']} stands in front of door {pid}")
    return problems


def volume(c):
    return sum(abs(layout_check.signed_area(b["poly"])) * (b["y"][1] - b["y"][0]) for b in c["brushes"])


def floor_area(c):
    return sum(abs(layout_check.signed_area(b["poly"])) for b in c["brushes"])


def door_floor(L, pid):
    p = next(x for x in L["portals"] if x["id"] == pid)
    return (p["center_m"][0], p["center_m"][2])


def walk(legs):
    """Metres and seconds along straight legs [(x, z), ...]; a ('stair', top, foot) leg runs at STAIR_M_S."""
    m, s, prev = 0.0, 0.0, None
    for leg in legs:
        if leg[0] == "ladder":
            _, at, down_m = leg
            if prev is not None:
                d0 = math.dist(prev, at)
                m += d0; s += d0 / WALK_M_S
            m += down_m; s += down_m / LADDER_DOWN_M_S + 2 * LADDER_MOUNT_S
            prev = at
            continue
        if leg[0] == "stair":
            _, top, foot = leg
            d = math.dist(top, foot)
            if prev is not None:
                d0 = math.dist(prev, top)
                m += d0; s += d0 / WALK_M_S
            m += d; s += d / STAIR_M_S
            prev = foot
            continue
        if prev is not None:
            d = math.dist(prev, leg)
            m += d; s += d / WALK_M_S
        prev = leg
    return m, s


def build():
    bridge = suite_bridge()
    rooms, aft = side_rooms(bridge)
    br = copy.deepcopy(comp(L0, "bridge"))
    br["brushes"] = copy.deepcopy(bridge["brushes"])
    br["purpose"] = ("The ship's command centre, round (bridge-stations 11a, variant B): seven stations and the "
                     "ring's wall banks facing the main viewscreen and two forward windows; doors to the command "
                     "passage, the ready room and the briefing room.")
    patch = {
        "compartments": [br] + rooms,
        "portals": portals(bridge, aft),
        "stations": [s for s in bridge["seats"] if any(st["id"] == s["station"] for st in L0["stations"])],
        "fixtures": [dict(f, **{"compartment": "bridge"}) for f in bridge.get("fixtures", [])] + [
            {"id": "eva_suits_a", "kind": "locker", "compartment": "bridge_locker", "center_m": [-3.9, FLOOR, 8.725],
             "size_m": [2.4, 0.55], "facing_yaw_deg": 180,
             "note": "EVA suits for the bridge crew, beside the bridge (crew-on-deck, damage-control: a second suit locker, proposed)."},
        ],
        "fixtures_removed": ["captain_dais"],
        "systems": [{"id": "computer", "center_m": [-5.0, FLOOR, 11.7]}],
    }
    # The viewscreen keeps every field but its centre.
    patch["fixtures"][0] = {"id": "viewscreen", "center_m": bridge["fixtures"][0]["center_m"]}
    return bridge, patch


def measure(L_after, F):
    rows = []
    for cid in ("bridge", "ready_room", "briefing_room", "captains_quarters", "computer_core", "head", "bridge_locker", "a_corridor"):
        c = comp(L_after, cid)
        before = next((x for x in L0["compartments"] if x["id"] == cid), None)
        margin = min(layout_check.hull_margin(L_after["hull"]["sections"], b["poly"], *b["y"])[0] for b in c["brushes"])
        doors = sorted(p["id"] for p in L_after["portals"] if cid in p["between"] and p["kind"] == "door")
        windows = sorted(p["id"] for p in L_after["portals"] if cid in p["between"] and p["kind"] == "window")
        rows.append({
            "id": cid, "name": c["name"], "brushes": len(c["brushes"]),
            "air_m3": round(volume(c), 1), "floor_m2": round(floor_area(c), 1),
            "was_air_m3": round(volume(before), 1) if before else None,
            "hull_margin_m": round(margin, 2), "doors": doors, "windows": windows,
            "furniture": sum(1 for f in F if f["room"] == cid),
        })
    ship_before = sum(volume(c) for c in L0["compartments"])
    ship_after = sum(volume(c) for c in L_after["compartments"])
    # Walks, along the walkway and through the doors' centres (floor points), seated eye to seat.
    seat = {s["id"]: (s["seat_m"][0], s["seat_m"][2]) for s in L_after["stations"]}
    dais = next(s for s in suite_bridge()["stairs"] if s["note"].startswith("Captain's dais"))
    dais_top, dais_foot = (dais["top_m"][0], dais["top_m"][2]), (dais["foot_m"][0], dais["foot_m"][2])
    capt_desk = (7.9, 17.4)
    walks = {}
    walks["captain's chair to the ready room desk"] = walk([seat["captain"], ("stair", dais_top, dais_foot), door_floor(L_after, "p_ready_bridge"), (4.4, 19.2), capt_desk])
    walks["captain's chair to the briefing table's head"] = walk([seat["captain"], ("stair", dais_top, dais_foot), door_floor(L_after, "p_briefing_bridge"), (-4.3, 19.9)])
    walks["captain's bed to the captain's chair, through the ready room"] = walk([(6.3, 10.4), door_floor(L_after, "p_quarters_ready"), (4.4, 19.2), door_floor(L_after, "p_ready_bridge"), ("stair", dais_foot, dais_top), seat["captain"]])
    walks["captain's bed to the captain's chair, by the passage"] = walk([(6.3, 10.4), (3.0, 12.5), door_floor(L_after, "p_captains_quarters"), (0.0, 12.5), (0.0, 19.6), door_floor(L_after, "p_bridge_aft"), ("stair", dais_foot, dais_top), seat["captain"]])
    walks["helm's seat to the bridge locker's suits"] = walk([seat["helm"], (1.5, 26.4), (2.0, 25.0), (0.8, 22.0), door_floor(L_after, "p_bridge_aft"), (0.0, 7.5), door_floor(L_after, "p_bridge_locker"), (-3.9, 7.9)])
    # Today, for comparison: helm to damage control's suit locker on deck B, down the ladder trunk.
    seat0 = {s["id"]: (s["seat_m"][0], s["seat_m"][2]) for s in L0["stations"]}
    walks["helm's seat to damage control's suits, today"] = walk([seat0["helm"], (2.4, 24.2), door_floor(L0, "p_bridge_aft"), (0.0, 11.5),
                                                                  ("ladder", (0.0, 11.0), 3.5), door_floor(L0, "p_damage_control"), (-8.5, 19.7)])
    return {"rooms": rows, "ship_air_m3": {"before": round(ship_before, 1), "after": round(ship_after, 1)},
            "walks": {k: {"m": round(v[0], 1), "s": round(v[1], 1)} for k, v in walks.items()}}


def main(argv):
    bridge, patch = build()
    if "--sightlines" in argv:
        vs = next(f for f in L0["fixtures"] if f["id"] == "viewscreen")
        BV.sightlines("Command suite bridge", bridge["seats"], (tuple(bridge["fixtures"][0]["center_m"]), tuple(vs["size_m"])))
        return 0
    F = furnishings()
    fp = footprints()
    L = patch_layout(copy.deepcopy(L0), patch)
    ok, report = run_layout_check(L)
    problems = [ln.strip() for ln in report.splitlines() if "FAIL" in ln]
    poly = bridge["brushes"][0]["poly"]
    for pf in bridge["platforms"]:
        for x, z in pf["poly"]:
            if not layout_check.inside_poly(poly, x, z, tol=0.01):
                problems.append(f"platform {pf['id']}: corner ({x}, {z}) is outside the bridge")
    for p in bridge["props"]:
        if not layout_check.inside_poly(poly, p["back_m"][0], p["back_m"][2], tol=0.02):
            problems.append(f"prop {p['prop']} ({p['station']}): its back is outside the bridge")
    problems += check_furniture(L, F, fp, bridge["stairs"])
    numbers = measure(L, F)
    doc = {
        "schema": "starcrew.layout-patch/1",
        "status": ("Proposed (2026-10-06): openspec/changes/command-suite, docs/mockups/command-deck.html. Written by "
                   "tools/command_suite.py; edit the script, not this file. The layout does not hold it yet: "
                   "applying it is the change's task 2."),
        "_rules": [
            "compartments and portals replace the layout's record of the same id, or are added; stations move the named station's seat; "
            "fixtures update the layout's fixture of the same id field by field, or are added; fixtures_removed are dropped; systems update field by field.",
            "bridge holds the round bridge's platforms, stairs, consoles and seats, as a variant of data/ships/tern/bridge_variants.json does.",
            "furnishings are the side rooms' furniture, placed as the bridge props are: back_m is the back of the piece at floor level and "
            "yaw_deg the way its front faces (0 the bow, +90 port); set names the prop set (assets/models/<set>); a chair's seat_m is its sitter's seat point. "
            "They become the deck's detail file when the patch is applied.",
        ],
        "bridge": {k: bridge[k] for k in ("id", "name", "after", "summary", "platforms", "stairs", "props", "seats")},
        **patch,
        "furnishings": F,
        "prop_brief_m": {k: list(v) for k, v in BRIEF.items()},
        "numbers": numbers,
    }
    text = json.dumps(doc, indent=1, ensure_ascii=False) + "\n"
    print(f"  layout check on the patched copy: {'ok' if ok else 'FAIL'}")
    for p in problems:
        print(f"    {p}")
    for r in numbers["rooms"]:
        was = f" (was {r['was_air_m3']})" if r["was_air_m3"] is not None and r["was_air_m3"] != r["air_m3"] else ""
        print(f"  {r['name']:<20} {r['air_m3']:>7.1f} m3{was:<16} {r['floor_m2']:>6.1f} m2  hull {r['hull_margin_m']:.2f} m  "
              f"{r['brushes']} brush(es), doors {', '.join(r['doors'])}{'; windows ' + ', '.join(r['windows']) if r['windows'] else ''}; {r['furniture']} pieces")
    print(f"  ship air {numbers['ship_air_m3']['before']} -> {numbers['ship_air_m3']['after']} m3")
    for k, v in numbers["walks"].items():
        print(f"  {k}: {v['m']} m, {v['s']} s")
    good = ok and not problems
    if "--check" in argv:
        with open(OUT, encoding="utf-8") as f:
            stale = f.read() != text
        if stale:
            print(f"  FAIL {os.path.relpath(OUT, ROOT)} is stale; run python3 tools/command_suite.py")
        return 1 if stale or not good else 0
    with open(OUT, "w", encoding="utf-8") as f:
        f.write(text)
    print(f"  wrote {os.path.relpath(OUT, ROOT)}")
    return 0 if good else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
