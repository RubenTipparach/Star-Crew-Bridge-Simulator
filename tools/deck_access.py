#!/usr/bin/env python3
"""Write the Tern's deck access patch (data/ships/tern/deck_access.json) and check it: the ways up and down.

The owner, 2026-10-06: "how do the levels connect? can we get starways on the sides? elevators?", then
"we need to install multiple ways to go up and down since various parts of the ship can be damaged".

This is documentation tooling and a measurement instrument (CLAUDE.md section 4), for
openspec/changes/deck-access. It does three things:

  * measures how the decks connect: for every compartment, which rooms the bridge loses when that one
    compartment is lost (breached, on fire or sealed), on today's layout, on the layout with the
    command suite, and with this patch as well, and how long the main routes between decks take;
  * computes the patch: a spiral stair tower each side of the spine, a lift beside the bridge door,
    and two emergency scuttles from the side rooms by the bridge down to deck B;
  * checks it, applied after the command suite to a copy of the layout in memory: tools/layout_check.py's
    rules, and the suite's furniture still inside its rooms and clear of every door.

The patch builds on the command suite (openspec/changes/command-suite), because the suite is where the
bridge's side rooms are; it never edits the layout. Applying both is their changes' task 2.

Usage: python3 tools/deck_access.py [--check]
  --check   writes nothing, fails if the data file is stale or a check fails
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
import command_suite as CS  # noqa: E402
import layout_check as lc  # noqa: E402

OUT = os.path.join(ROOT, "data", "ships", "tern", "deck_access.json")
LAYOUT0 = CS.L0
with open(CS.OUT, encoding="utf-8") as _f:
    SUITE = json.load(_f)

DECK_Y = {"A": 3.5, "B": 0.0, "C": -3.5}   # deck floors (layout decks)
ROOM_H = 3.0                               # a deck's rooms: floor to ceiling
TRUNK_Y = [-3.5, 6.5]                      # a trunk runs from deck C's floor to deck A's ceiling
DOOR_W, DOOR_H = 1.0, 2.2
CREW = {"door", "hatch", "ladder", "pressure_door"}

# The stair towers: one each side of the spine, beside the corridor, where no system, fixture or door is
# in the way on any deck (measured by a scan over z, design section 2).
TOWER = {"x": (1.25, 3.85), "z": (9.0, 11.6)}
LANDING_D = 1.1           # the landing at each deck, across the tower's aft end
SPIRAL_R, COLUMN_R = 1.2, 0.15
SWEEP_DEG = 240.0         # the treads' sweep per deck; the landing takes the rest, over the aft
RISERS = 18               # per deck: 3.5 m in 18 risers of 0.194 m
WALK_LINE_R = 0.85        # where a person walks a spiral stair
# The lift: starboard, forward, its door on the passage. It stands between the briefing room's two doors, clear
# of both their zones: its passage door aft (z 16.4) and its door from the bridge forward (2026-10-06: the first
# lift, at z 17.6-20.0, stood 0.6 m behind that one; the trunk check below now catches it). Its outboard wall
# meets the backs of the briefing table's inboard chairs, at x -3.3.
LIFT = {"x": (-3.3, -1.25), "z": (17.1, 19.0)}
LIFT_CAR = [1.9, 2.2, 1.6]   # depth from its door (x), height, width (z) in metres: a 1.9 m stretcher fits lengthwise
LIFT_SPEED_M_S = 1.5         # assumed (design section 3)
LIFT_DOOR_S = 2.0            # to open, and again to close (assumed)
# The scuttles: a hatch and ladder from each side room by the bridge down to the room below.
SCUTTLES = [("p_scuttle_ready", "ready_room", "medbay", 7.6, 19.2),
            ("p_bridge_scuttle", "briefing_room", "damage_control", -7.6, 19.2)]   # the second is T3's
SCUTTLE_SIZE = [0.9, 0.9]
# Second ways out: a door through a wall two rooms already share, for the rooms whose only door is on a
# spine corridor (a ship's "two means of escape"). (id, from, into, centre x, deck, z, normal along +z or -z).
SECOND_EXITS = [
    ("p_port_turret_quarters", "port_turret", "quarters", 5.5, "B", 8.0, 1.0),
    ("p_stbd_turret_mess", "stbd_turret", "mess", -5.5, "B", 8.0, 1.0),
    ("p_dc_torpedo", "damage_control", "torpedo_room", -4.5, "B", 26.0, 1.0),
    ("p_shield_life_support", "shield_room", "life_support", 5.5, "C", 6.0, 1.0),
    ("p_switchboard_cargo", "switchboard", "cargo", -6.3, "C", 6.0, 1.0),
    ("p_life_support_magazine", "life_support", "magazine", 5.0, "C", 20.0, 1.0),
]

# Speeds: crew-on-deck's, as tools/walk_times.py uses them.
WALK, STAIR = 1.8, 0.7
LADDER_UP, LADDER_DOWN, MOUNT = 0.8, 1.0, 0.5
CASUALTY, CAS_LADDER = 1.2, 0.5

r3 = CS.r3


# ------------------------------------------------------------------ geometry

def _clip(poly, keep, cut):
    """Sutherland-Hodgman against one half plane: keep(p) says which side, cut(a, b) the crossing point."""
    out = []
    for i in range(len(poly)):
        a, b = poly[i], poly[(i + 1) % len(poly)]
        ka, kb = keep(a), keep(b)
        if ka:
            out.append(a)
        if ka != kb:
            out.append(cut(a, b))
    return out


def _cut_x(x):
    return lambda a, b: (x, a[1] + (b[1] - a[1]) * (x - a[0]) / (b[0] - a[0]))


def _cut_z(z):
    return lambda a, b: (a[0] + (b[0] - a[0]) * (z - a[1]) / (b[1] - a[1]), z)


def _clean(poly):
    """Drop repeated and collinear corners, which the layout checker refuses."""
    pts = []
    for p in poly:
        q = (round(p[0], 3), round(p[1], 3))
        if not pts or q != pts[-1]:
            pts.append(q)
    if len(pts) > 1 and pts[0] == pts[-1]:
        pts.pop()
    changed = True
    while changed and len(pts) >= 3:
        changed = False
        for i in range(len(pts)):
            a, b, c = pts[i - 1], pts[i], pts[(i + 1) % len(pts)]
            if abs((b[0] - a[0]) * (c[1] - b[1]) - (b[1] - a[1]) * (c[0] - b[0])) < 1e-6:
                pts.pop(i)
                changed = True
                break
    return [list(p) for p in pts]


def subtract_rect(poly, x0, x1, z0, z1):
    """A convex polygon less an axis-aligned rectangle, as convex pieces: the parts either side of the
    rectangle's x span, whole along z, then the parts of that span aft and forward of it. Splitting across x
    first keeps a room's long outer walls whole, so the doors and windows on them still fit one wall.
    Pieces share faces, so they are one room's brushes, open between them."""
    pieces = [
        _clip(poly, lambda p: p[0] <= x0 + 1e-9, _cut_x(x0)),
        _clip(poly, lambda p: p[0] >= x1 - 1e-9, _cut_x(x1)),
    ]
    band = _clip(_clip(poly, lambda p: p[0] >= x0 - 1e-9, _cut_x(x0)), lambda p: p[0] <= x1 + 1e-9, _cut_x(x1))
    if band:
        pieces.append(_clip(band, lambda p: p[1] <= z0 + 1e-9, _cut_z(z0)))
        pieces.append(_clip(band, lambda p: p[1] >= z1 - 1e-9, _cut_z(z1)))
    out = []
    for p in pieces:
        p = _clean(p)
        if len(p) >= 3 and abs(lc.signed_area(p)) > 0.01:
            if lc.signed_area(p) < 0:
                p.reverse()
            out.append(p)
    return out


def rect(x0, x1, z0, z1):
    return [[r3(x0), r3(z0)], [r3(x1), r3(z0)], [r3(x1), r3(z1)], [r3(x0), r3(z1)]]


def overlaps(poly, x0, x1, z0, z1):
    return lc.polys_overlap(poly, rect(x0, x1, z0, z1))


def carve(L, x0, x1, z0, z1, skip=()):
    """Take a trunk's footprint out of every room it crosses, on every deck. Returns the changed compartments."""
    changed = []
    for c in L["compartments"]:
        if c["id"] in skip:
            continue
        new, hit = [], False
        for b in c["brushes"]:
            if b["y"][1] > TRUNK_Y[0] and b["y"][0] < TRUNK_Y[1] and overlaps(b["poly"], x0, x1, z0, z1):
                hit = True
                new += [{"y": b["y"], "poly": p} for p in subtract_rect(b["poly"], x0, x1, z0, z1)]
            else:
                new.append(b)
        if hit:
            c["brushes"] = new
            changed.append(c)
    return changed


def room_at(L, x, z, deck):
    y = DECK_Y[deck] + 0.5
    for c in L["compartments"]:
        for b in c["brushes"]:
            if b["y"][0] <= y <= b["y"][1] and lc.inside_poly(b["poly"], x, z, tol=1e-3):
                return c["id"]
    return None


# ------------------------------------------------------------------ the patch

def door(pid, a, b, x, deck, z, normal_x, w=DOOR_W):
    return {"id": pid, "kind": "door", "between": [a, b], "center_m": [r3(x), r3(DECK_Y[deck] + DOOR_H / 2), r3(z)],
            "normal": [normal_x, 0.0, 0.0], "size_m": [w, DOOR_H]}


def build():
    """(the patch, the suite layout, the patched layout)."""
    LS = CS.patch_layout(copy.deepcopy(LAYOUT0), SUITE)
    L = copy.deepcopy(LS)
    comps, portals, fixtures = [], [], []
    door_z = r3(TOWER["z"][0] + LANDING_D / 2)
    corridors = {"A": "a_corridor", "B": "b_spine", "C": "c_spine"}
    for side, tag, name in ((1, "p", "Port"), (-1, "s", "Starboard")):
        x0, x1 = sorted((side * TOWER["x"][0], side * TOWER["x"][1]))
        z0, z1 = TOWER["z"]
        inboard, outboard = side * TOWER["x"][0], side * TOWER["x"][1]
        # The rooms the tower opens into, read before it is carved out of them.
        rooms = {d: room_at(L, outboard + side * 0.5, door_z, d) for d in "ABC"}
        cid = f"stair_{tag}"
        comps.append({"id": cid, "poi": 35 if side > 0 else 36, "name": f"{name} stair tower", "decks": ["C", "B", "A"],
                      "kind": "trunk", "finish": "crew",
                      "purpose": (f"A spiral stair from deck C to deck A, with a landing and two doors on every deck: the spine "
                                  f"corridor, and the {', '.join(rooms[d].replace('_', ' ') for d in 'ABC')} beside it. "
                                  "Needs no power."),
                      "brushes": [{"y": TRUNK_Y, "poly": rect(x0, x1, z0, z1)}]})
        L["compartments"].append(copy.deepcopy(comps[-1]))
        for d in "ABC":
            # Normals point out of the tower: toward the corridor inboard, toward the room outboard.
            portals.append(door(f"p_{cid}_{d.lower()}", cid, corridors[d], inboard, d, door_z, -float(side)))
            portals.append(door(f"p_{cid}_{d.lower()}_room", cid, rooms[d], outboard, d, door_z, float(side)))
        # On deck A the landing's aft wall is the head's or the bridge locker's: a third door, so they keep a way out.
        aft_room = room_at(L, (x0 + x1) / 2, z0 - 0.5, "A")
        portals.append({"id": f"p_{cid}_a_aft", "kind": "door", "between": [cid, aft_room],
                        "center_m": [r3((x0 + x1) / 2), r3(DECK_Y["A"] + DOOR_H / 2), z0], "normal": [0.0, 0.0, -1.0], "size_m": [DOOR_W, DOOR_H]})
        cx = r3((x0 + x1) / 2)
        cz = r3(z1 - SPIRAL_R - 0.1)
        fixtures.append({"id": f"{cid}_spiral", "kind": "spiral_stair", "compartment": cid,
                         "center_m": [cx, DECK_Y["C"], cz], "radius_m": SPIRAL_R, "column_radius_m": COLUMN_R,
                         "floors_y_m": [DECK_Y["C"], DECK_Y["B"], DECK_Y["A"]], "risers_per_deck": RISERS,
                         "start_yaw_deg": 240.0 if side > 0 else 120.0, "sweep_deg": SWEEP_DEG * side,
                         "landing_poly": rect(x0, x1, z0, z0 + LANDING_D), "walk_line_m": WALK_LINE_R,
                         "well_poly": rect(x0, x1, z0, z1),
                         "note": ("Treads sweep forward from the landing and round to the next deck's landing over it, one deck a sweep. "
                                  "They run out to the trunk's walls (well_poly), so there is no gap to fall through (owner, 2026-10-07).")})
    x0, x1 = LIFT["x"]
    z0, z1 = LIFT["z"]
    comps.append({"id": "lift", "poi": 37, "name": "Lift", "decks": ["C", "B", "A"], "kind": "trunk", "finish": "working",
                  "purpose": ("A lift (elevator) from deck C to deck A beside the bridge door, its car big enough for a stretcher. "
                              "Fast with power; stops without it, and then the stair towers and ladders remain."),
                  "brushes": [{"y": TRUNK_Y, "poly": rect(x0, x1, z0, z1)}]})
    L["compartments"].append(copy.deepcopy(comps[-1]))
    lz = r3((z0 + z1) / 2)
    for d in "ABC":
        portals.append(door(f"p_lift_{d.lower()}", "lift", corridors[d], x1, d, lz, 1.0, w=1.2))
    fixtures.append({"id": "lift_car", "kind": "lift", "compartment": "lift", "center_m": [r3((x0 + x1) / 2), DECK_Y["A"], lz],
                     "car_m": LIFT_CAR, "stops_y_m": [DECK_Y["C"], DECK_Y["B"], DECK_Y["A"]],
                     "speed_m_s": LIFT_SPEED_M_S, "door_s": LIFT_DOOR_S, "facing_yaw_deg": 90.0,
                     "note": "Drawn at deck A; its door faces the passage (+x). Speed and door times assumed (deck-access design 3)."})
    for pid, upper, lower, x, z in SCUTTLES:
        portals.append({"id": pid, "kind": "hatch", "between": [upper, lower], "center_m": [x, DECK_Y["A"] - 0.5, z],
                        "normal": [0.0, -1.0, 0.0], "size_m": SCUTTLE_SIZE})
    for pid, a, b, x, d, z, nz in SECOND_EXITS:
        portals.append({"id": pid, "kind": "door", "between": [a, b], "center_m": [x, r3(DECK_Y[d] + DOOR_H / 2), z],
                        "normal": [0.0, 0.0, nz], "size_m": [DOOR_W, DOOR_H]})
    # Carve the trunks out of the rooms they stand in (after the side rooms were read).
    changed = {}
    for t in (TOWER, ):
        for side in (1, -1):
            xs = sorted((side * t["x"][0], side * t["x"][1]))
            for c in carve(L, xs[0], xs[1], t["z"][0], t["z"][1], skip={"stair_p", "stair_s", "lift"}):
                changed[c["id"]] = c
    for c in carve(L, LIFT["x"][0], LIFT["x"][1], LIFT["z"][0], LIFT["z"][1], skip={"stair_p", "stair_s", "lift"}):
        changed[c["id"]] = c
    L["portals"] += copy.deepcopy(portals)
    L["fixtures"] += copy.deepcopy(fixtures)
    patch = {
        "compartments": comps + [copy.deepcopy(changed[k]) for k in sorted(changed)],
        "portals": portals, "stations": [], "fixtures": fixtures, "fixtures_removed": [], "systems": [],
    }
    return patch, LS, L


# ------------------------------------------------------------------ measurements

def graph(L, dead=(), no_power=False):
    g = {c["id"]: set() for c in L["compartments"]}
    for p in L["portals"]:
        a, b = p["between"]
        if p["kind"] in CREW and b != "space" and a not in dead and b not in dead:
            if no_power and "lift" in (a, b):
                continue
            g[a].add(b)
            g[b].add(a)
    return g


def reach(g, start):
    seen, st = {start}, [start]
    while st:
        for m in g[st.pop()]:
            if m not in seen:
                seen.add(m)
                st.append(m)
    return seen


def single_losses(L):
    """{lost compartment: [rooms the bridge can no longer reach]}, for every compartment but the bridge."""
    out = {}
    for c in L["compartments"]:
        if c["id"] == "bridge":
            continue
        g = graph(L, dead={c["id"]})
        lost = sorted(set(g) - reach(g, "bridge") - {c["id"]})
        if lost:
            out[c["id"]] = lost
    return out


def decks_reached(L, dead):
    g = graph(L, dead=dead)
    r = reach(g, "bridge")
    by = {c["id"]: c["decks"] for c in L["compartments"]}
    return sorted({d for cid in r for d in by[cid]})


def spiral_time_s():
    """One deck on a tower: the walk line's sweep at stair speed, and the landing crossed at a walk."""
    arc = math.radians(SWEEP_DEG) * WALK_LINE_R
    return arc / (WALK * STAIR) + LANDING_D / WALK


def legs_time(legs, carry=False):
    """Seconds along legs: (x, z) points walked; ('ladder', dy) up or down; ('spiral', decks); ('lift', dy)."""
    speed = CASUALTY if carry else WALK
    t, prev, m = 0.0, None, 0.0
    for leg in legs:
        if isinstance(leg[0], str):
            kind, v = leg
            if kind == "ladder":
                rate = (LADDER_UP if v > 0 else LADDER_DOWN) * (CAS_LADDER if carry else 1.0)
                t += abs(v) / rate + 2 * MOUNT
            elif kind == "spiral":
                t += spiral_time_s() * v * (WALK / speed)
            elif kind == "lift":
                t += abs(v) / LIFT_SPEED_M_S + 2 * LIFT_DOOR_S
            continue
        if prev is not None:
            d = math.dist(prev, leg)
            m += d
            t += d / speed
        prev = leg
    return t


def routes(LS, L):
    """The main routes between decks, before (the suite alone) and after (with deck access)."""
    seat = {s["id"]: (s["seat_m"][0], s["seat_m"][2]) for s in LS["stations"]}
    pz = lambda Lx, pid: (lambda p: (p["center_m"][0], p["center_m"][2]))(next(p for p in Lx["portals"] if p["id"] == pid))
    helm, bridge_door = seat["helm"], pz(LS, "p_bridge_aft")
    bed = (4.5, 22.0)
    medbay_door = pz(LS, "p_medbay")
    ladder = (0.0, 11.0)
    quarters_door = pz(LS, "p_quarters")
    tower_b = (2.55, 9.55)
    out = {}
    out["casualty, helm to a medbay bed: down the ladder (today)"] = legs_time(
        [helm, (2.0, 25.0), (0.8, 22.0), bridge_door, (0.0, 11.6), ("ladder", -3.5), ladder, (0.0, 21.0), medbay_door, bed], carry=True)
    out["casualty, helm to a medbay bed: by the lift"] = legs_time(
        [helm, (2.0, 25.0), (0.8, 22.0), bridge_door, (-0.6, 18.8), ("lift", -3.5), (-0.6, 18.8), (0.0, 21.0), medbay_door, bed], carry=True)
    out["crew quarters door to the bridge door: up the ladder (today)"] = legs_time(
        [quarters_door, (0.0, 13.0), (0.0, 11.6), ("ladder", 3.5), (0.0, 11.6), bridge_door])
    out["crew quarters door to the bridge door: up the port tower"] = legs_time(
        [quarters_door, (0.6, 11.0), (0.6, 9.55), (1.25, 9.55), tower_b, ("spiral", 1), (1.25, 9.55), (0.6, 9.55), (0.0, 12.0), bridge_door])
    out["deck C spine to the bridge door: the ladder (today)"] = legs_time(
        [(0.0, 9.0), (0.0, 11.6), ("ladder", 7.0), (0.0, 11.6), bridge_door])
    out["deck C spine to the bridge door: a tower"] = legs_time(
        [(0.0, 9.0), (0.6, 9.55), (1.25, 9.55), tower_b, ("spiral", 2), (1.25, 9.55), (0.6, 9.55), (0.0, 12.0), bridge_door])
    out["deck C spine to the bridge door: the lift"] = legs_time(
        [(0.0, 9.0), (-0.6, 18.8), ("lift", 7.0), (-0.6, 18.8), bridge_door])
    return {k: round(v, 1) for k, v in out.items()}


# ------------------------------------------------------------------ checks

def run_layout_check(L):
    tmp = os.path.join(os.path.dirname(CS.LAYOUT), ".deck_access_check.json")
    with open(tmp, "w", encoding="utf-8") as f:
        json.dump(L, f)
    buf = io.StringIO()
    try:
        with redirect_stdout(buf):
            ok = lc.check(tmp, quiet=False)
    finally:
        os.remove(tmp)
    return ok, buf.getvalue()


def checks(L):
    ok, report = run_layout_check(L)
    problems = [ln.strip() for ln in report.splitlines() if "FAIL" in ln]
    # The suite's furniture still inside its (now carved) rooms and clear of every door, the new ones too.
    problems += CS.check_furniture(L, SUITE["furnishings"], CS.footprints())
    # Nothing a system or fixture stands on lies inside a trunk.
    trunks = [c for c in L["compartments"] if c["kind"] == "trunk"]
    for s in L["systems"] + [f for f in L["fixtures"] if f["kind"] not in ("spiral_stair", "lift")]:
        x, _, z = s["center_m"]
        for t in trunks:
            if lc.inside_poly(t["brushes"][0]["poly"], x, z, tol=-0.01):
                problems.append(f"{s['id']} stands inside {t['id']}")
    # No trunk stands in a door's clear zone (a trunk spans every deck, so on any deck), unless the door is its own:
    # the first lift stood 0.6 m behind the briefing room's door from the bridge (found walking it, 2026-10-06).
    by_id = {p["id"]: p for p in L["portals"]}
    for pid, cid, zone in CS.door_zones(L):
        for t in trunks:
            if t["id"] not in by_id[pid]["between"] and lc.polys_overlap(t["brushes"][0]["poly"], zone):
                problems.append(f"{t['id']} stands in the clear zone of {pid} on the {cid} side")
    return ok and not problems, problems


def main(argv):
    patch, LS, L = build()
    good, problems = checks(L)
    before0, before, after = single_losses(LAYOUT0), single_losses(LS), single_losses(L)
    after_np = {}
    for c in L["compartments"]:
        if c["id"] in ("bridge", "lift"):
            continue
        g = graph(L, dead={c["id"]}, no_power=True)
        lost = sorted(set(g) - reach(g, "bridge") - {c["id"], "lift"})
        if lost:
            after_np[c["id"]] = lost
    vols = {c["id"]: round(CS.volume(c), 1) for c in patch["compartments"]}
    rt = routes(LS, L)
    numbers = {
        "single_losses": {"today": before0, "with_the_suite": before, "with_deck_access": after, "with_deck_access_and_no_power": after_np},
        "air_m3": vols,
        "ship_air_m3": {"with_the_suite": round(sum(CS.volume(c) for c in LS["compartments"]), 1),
                         "with_deck_access": round(sum(CS.volume(c) for c in L["compartments"]), 1)},
        "spiral_s_per_deck": round(spiral_time_s(), 1),
        "routes_s": rt,
    }
    doc = {
        "schema": "starcrew.layout-patch/1",
        "status": ("Proposed (2026-10-06): openspec/changes/deck-access. Applies after data/ships/tern/command_suite.json. "
                   "Written by tools/deck_access.py; edit the script, not this file. The layout does not hold it yet."),
        "_rules": ["The same fields and rules as command_suite.json (ShipKit.applyPatch, tools/command_suite.py patch_layout), applied after it."],
        **patch,
        "numbers": numbers,
    }
    text = json.dumps(doc, indent=1, ensure_ascii=False) + "\n"
    print(f"  checks on the layout with the suite and deck access: {'ok' if good else 'FAIL'}")
    for p in problems:
        print(f"    {p}")
    for name, losses in numbers["single_losses"].items():
        worst = sorted(losses.items(), key=lambda kv: -len(kv[1]))
        print(f"  {name}: {len(losses)} single losses cut rooms off; worst: " +
              "; ".join(f"{k} {len(v)}" for k, v in worst[:4]))
    for k, v in after.items():
        print(f"    with deck access, losing {k} cuts off {', '.join(v)}")
    for k, v in rt.items():
        print(f"  {k}: {v} s")
    print(f"  a tower takes {numbers['spiral_s_per_deck']} s a deck; trunks {vols.get('stair_p')}, {vols.get('stair_s')}, {vols.get('lift')} m3")
    print(f"  ship air {numbers['ship_air_m3']}")
    if "--check" in argv:
        with open(OUT, encoding="utf-8") as f:
            stale = f.read() != text
        if stale:
            print(f"  FAIL {os.path.relpath(OUT, ROOT)} is stale; run python3 tools/deck_access.py")
        return 1 if stale or not good else 0
    with open(OUT, "w", encoding="utf-8") as f:
        f.write(text)
    print(f"  wrote {os.path.relpath(OUT, ROOT)}")
    return 0 if good else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
