#!/usr/bin/env python3
"""Write the Tern bridge variants (data/ships/tern/bridge_variants.json) and check each one.

The owner, 2026-10-05, sharing Star Trek bridge cutaways: "notice how elevation is dynamic? the
upper deck is for work, the lower one is walk way to move around, and the captains chair is
raised on a platform, with two consoles for helms and tactical a sub platform. notice how
majority of bridge consoles are buit into walls? notice the shape of the bridge ... circular
... or the ship shape", then "make a few variations, and I'll give you my feed back on whats
the best". The references are described in docs/analysis/star-trek-bridges.md.

This is documentation tooling (CLAUDE.md section 4): it computes three proposed bridges for
openspec/changes/bridge-stations design section 11a, writes them as data the variants mockup
reads, and checks each against the layout rules by patching a copy of the layout in memory and
running tools/layout_check.py's checks on it. It never edits the layout. The variant the owner
picks becomes a layout patch in its own commit.

Every variant keeps the bridge's place, its aft door to the command passage, its seven
stations and its 3.5 m ceiling (deck A floor 3.5 m, ceiling 7.0 m). Levels are platforms
(solid, like the dais), not air: the air stays one brush. Steps rise 0.225 m.

  A  Wedge, tiered: the ship-shaped wedge (R5, R6) with the owner's three levels.
  B  Round: a sixteen-sided room with a ring (R1, R2, R3, R8).
  C  Split level: the wedge split front and back by a riser (R9, R2).

Usage: python3 tools/bridge_variants.py [--check] [--sightlines]
  --check       writes nothing, fails if the data file is stale
  --sightlines  prints each bridge's sightlines to the viewscreen (design section 11a's tables)
Standard library only.
"""

import copy
import io
import json
import math
import os
import sys
from contextlib import redirect_stdout

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.join(ROOT, "tools"))
import layout_check  # noqa: E402

LAYOUT = os.path.join(ROOT, "data", "ships", "tern", "layout.json")
OUT = os.path.join(ROOT, "data", "ships", "tern", "bridge_variants.json")

FLOOR = 3.5          # deck A floor, the walkway
CEIL = 7.0           # the bridge's ceiling
STEP = 0.225         # one step's rise
RING = FLOOR + 2 * STEP      # the work ring, two steps up
SUB = FLOOR + STEP           # the helm and tactical sub-platform, one step up
CAPT = FLOOR + 3 * STEP      # the captain's platform, three steps up
PROPS_JSON = os.path.join(ROOT, "assets", "models", "bridge", "props.json")
with open(PROPS_JSON, encoding="utf-8") as _f:
    PROPS = json.load(_f)["props"]


def operator(prop, i=0):
    """Where a prop's operator i sits, in prop space (x right, z out of its face), from the Blender
    build's manifest: the one source, so a seat lands where the model expects its operator."""
    x, _, z = PROPS[prop]["operators_m"][i]
    return x, z


SEAT_FROM_WALL = operator("wall_bank")[1]       # a wall bank's seat, out from the wall (0.95 m)
assert SEAT_FROM_WALL == operator("wall_bank_core")[1]
DESK_FROM_SEAT = operator("free_console")[1]    # a free console's back, ahead of its seat (0.91 m)
CHAIR_SEAT = operator("captain_chair")[1]       # the captain's seat point, ahead of the chair's back


def r3(v):
    return round(v + 0.0, 3)


def pt(p):
    return [r3(p[0]), r3(p[1])]


def octagon(cx, cz, apothem):
    """A regular octagon with flat edges facing the bow and the stern, wound positive."""
    r = apothem / math.cos(math.pi / 8)
    return [pt((cx + r * math.cos(math.radians(-67.5 + 45 * k)), cz + r * math.sin(math.radians(-67.5 + 45 * k)))) for k in range(8)]


INSIDE = (0.0, 26.0)  # a point inside every variant's room


def wall_frame(a, b, u):
    """Point u metres along wall a->b, the wall's unit direction and its normal into the room."""
    dx, dz = b[0] - a[0], b[1] - a[1]
    ln = math.hypot(dx, dz)
    t = (dx / ln, dz / ln)
    inward = (-t[1], t[0])
    p = (a[0] + t[0] * u, a[1] + t[1] * u)
    if (INSIDE[0] - p[0]) * inward[0] + (INSIDE[1] - p[1]) * inward[1] < 0:
        inward = (-inward[0], -inward[1])
    return p, t, inward


def wall_bank(station, prop, a, b, u, floor, seat=True):
    """A console built into the wall a->b, centred u metres along it, and its seat facing it."""
    p, t, n = wall_frame(a, b, u)
    out_yaw = math.degrees(math.atan2(-n[0], -n[1]))          # facing the wall
    face_yaw = math.degrees(math.atan2(n[0], n[1]))           # the bank faces into the room
    prop_rec = {"prop": prop, "station": station, "back_m": [r3(p[0]), r3(floor), r3(p[1])], "yaw_deg": round(face_yaw, 1)}
    seat_rec = None
    if seat:
        s = (p[0] + n[0] * SEAT_FROM_WALL, p[1] + n[1] * SEAT_FROM_WALL)
        seat_rec = {"station": station, "seat_m": [r3(s[0]), r3(floor), r3(s[1])], "yaw_deg": round(out_yaw, 1)}
    return prop_rec, seat_rec


def mirror_x(poly):
    return [pt((-x, z)) for x, z in reversed(poly)]


def mirror_edges(edges):
    """Edge kinds of a polygon mirrored by mirror_x (the order of edges reverses)."""
    n = len(edges)
    return [edges[(n - 2 - i) % n] for i in range(n)]


def mirror_rec(rec, station=None):
    r = copy.deepcopy(rec)
    for k in ("back_m", "seat_m", "center_m", "top_m", "foot_m"):
        if k in r:
            r[k][0] = r3(-r[k][0])
    if "yaw_deg" in r:
        r["yaw_deg"] = round(-r["yaw_deg"], 1)
    if station is not None:
        r["station"] = station
    return r


def stair(top, foot, width, floor_top, floor_foot, note):
    return {"top_m": [r3(top[0]), r3(floor_top), r3(top[1])], "foot_m": [r3(foot[0]), r3(floor_foot), r3(foot[1])],
            "width_m": width, "note": note}


# --------------------------------------------------------------------------- the wedge (layout)

with open(LAYOUT, encoding="utf-8") as f:
    LAYOUT_DOC = json.load(f)
BRIDGE = next(c for c in LAYOUT_DOC["compartments"] if c["id"] == "bridge")
WEDGE = [tuple(p) for p in BRIDGE["brushes"][0]["poly"]]
# Named corners of the wedge (port is +x): aft corner, side corner, window corner, bow corner.
P_AFT, P_SIDE, P_WIN, P_BOW = (7.6, 20.0), (8.6, 21.0), (6.6, 29.4), (3.4, 32.4)
for p in (P_AFT, P_SIDE, P_WIN, P_BOW):
    assert p in WEDGE, f"the wedge has moved: {p} is not a corner of the bridge"


def variant_a():
    """Wedge, tiered: work rings along the side walls, a walkway, the captain raised, helm and tactical a step below."""
    d = 2.0  # the ring's depth from the side wall
    _, t, n = wall_frame(P_SIDE, P_WIN, 0)
    side_in = (P_SIDE[0] + n[0] * d, P_SIDE[1] + n[1] * d)
    win_in = (P_WIN[0] + n[0] * d, P_WIN[1] + n[1] * d)
    z_band = 22.0
    tt = (z_band - side_in[1]) / t[1]
    corner = (side_in[0] + t[0] * tt, z_band)
    ring_p = [pt(p) for p in [(2.4, 20.0), P_AFT, P_SIDE, P_WIN, win_in, corner, (2.4, z_band)]]
    ring_edges = ["wall", "wall", "wall", "step", "rail", "rail", "step"]
    capt = octagon(0.0, 24.0, 1.5)
    capt_edges = ["rail", "rail", "rail", "step", "rail", "rail", "rail", "step"]
    sub = [pt(p) for p in [(-3.4, 25.5), (3.4, 25.5), (3.4, 29.6), (-3.4, 29.6)]]
    sub_edges = ["riser", "riser", "riser", "riser"]
    platforms = [
        {"id": "ring_p", "name": "Port work ring", "poly": ring_p, "top_m": r3(RING), "edges": ring_edges},
        {"id": "ring_s", "name": "Starboard work ring", "poly": mirror_x(ring_p), "top_m": r3(RING), "edges": mirror_edges(ring_edges)},
        {"id": "sub", "name": "Helm and tactical sub-platform", "poly": sub, "top_m": r3(SUB), "edges": sub_edges},
        {"id": "command", "name": "Captain's platform", "poly": capt, "top_m": r3(CAPT), "edges": capt_edges},
    ]
    a_end = ((P_WIN[0] + win_in[0]) / 2, (P_WIN[1] + win_in[1]) / 2)
    fwd = (a_end[0] - n[1] * 0.0 - t[0] * 0.0, a_end[1])
    stairs = [
        stair((2.4, 21.0), (1.4, 21.0), 1.6, RING, FLOOR, "Port ring down to the door aisle."),
        stair((fwd[0] - t[0] * -0.0, fwd[1]), (fwd[0] - 0.42 * 0.6, fwd[1] - 0.6 * 0.0 + 0.0), 1.6, RING, FLOOR, "placeholder"),
        stair((0.0, 22.5), (0.0, 21.75), 1.2, CAPT, FLOOR, "Captain's platform down to the door aisle."),
        stair((0.0, 25.5), (0.0, 26.0), 1.2, CAPT, SUB, "Captain's platform down to the sub-platform."),
    ]
    # The forward end of the port ring steps down toward the window: along the edge's outward normal.
    e0, e1 = P_WIN, win_in
    ex, ez = e1[0] - e0[0], e1[1] - e0[1]
    el = math.hypot(ex, ez)
    on = (ez / el, -ex / el)          # outward normal of the ring polygon's forward end edge
    mid = ((e0[0] + e1[0]) / 2, (e0[1] + e1[1]) / 2)
    stairs[1] = stair(mid, (mid[0] + on[0] * 0.5, mid[1] + on[1] * 0.5), 1.6, RING, FLOOR, "Port ring down to the walkway by the window.")
    stairs += [mirror_rec(stairs[0]), mirror_rec(stairs[1])]
    stairs[-2]["note"] = "Starboard ring down to the door aisle."
    stairs[-1]["note"] = "Starboard ring down to the walkway by the window."
    props, seats = [], []
    for st, prop, u in (("comms", "wall_bank", 2.2), ("engineering", "wall_bank_core", 5.6)):
        pr, se = wall_bank(st, prop, P_SIDE, P_WIN, u, RING)
        props.append(pr); seats.append(se)
    pr, _ = wall_bank("status_p", "wall_bank_double", P_AFT, (2.4, 20.0), 2.6, RING, seat=False)
    props.append(pr)
    for st_p, st_s in (("comms", "flight_ops"), ("engineering", "science"), ("status_p", "status_s")):
        props += [mirror_rec(p, st_s) for p in props if p["station"] == st_p]
        seats += [mirror_rec(s, st_s) for s in seats if s["station"] == st_p]
    props += [
        {"prop": "free_console", "station": "helm", "back_m": [1.8, r3(SUB), r3(28.0 + DESK_FROM_SEAT)], "yaw_deg": 180.0},
        {"prop": "free_console", "station": "tactical", "back_m": [-1.8, r3(SUB), r3(28.0 + DESK_FROM_SEAT)], "yaw_deg": 180.0},
        {"prop": "captain_chair", "station": "captain", "back_m": [0.0, r3(CAPT), r3(24.0 - CHAIR_SEAT)], "yaw_deg": 0.0},
    ]
    seats += [
        {"station": "helm", "seat_m": [1.8, r3(SUB), 28.0], "yaw_deg": 0.0},
        {"station": "tactical", "seat_m": [-1.8, r3(SUB), 28.0], "yaw_deg": 0.0},
        {"station": "captain", "seat_m": [0.0, r3(CAPT), 24.0], "yaw_deg": 0.0},
    ]
    return {
        "id": "A", "name": "Wedge, tiered",
        "after": "R5 and R6 (the ship-shaped bridge), with the owner's levels from R1 and R2",
        "summary": "The ship-shaped wedge kept. Work rings two steps up along the raked side walls carry the wall-built consoles; the walkway between them runs from the aft door to the viewscreen; the captain sits three steps up on an octagonal platform; helm and tactical one step up on a sub-platform in front.",
        "brushes": BRIDGE["brushes"], "platforms": platforms, "stairs": stairs, "props": props, "seats": seats,
    }


def ngon_room():
    """B's room: a sixteen-sided ring with flat edges at the stern (the door) and a flat chord at the bow (the viewscreen)."""
    n, cz, aft_z, chord_z = 16, 26.2, 20.0, 31.6
    apothem = cz - aft_z
    R = apothem / math.cos(math.pi / n)
    pts = [(R * math.cos(math.radians(-90 + 180 / n + 360 * k / n)), cz + R * math.sin(math.radians(-90 + 180 / n + 360 * k / n))) for k in range(n)]
    out = []
    for i in range(n):
        a, b = pts[i], pts[(i + 1) % n]
        ia, ib = a[1] <= chord_z, b[1] <= chord_z
        if ia:
            out.append(a)
        if ia != ib:
            s = (chord_z - a[1]) / (b[1] - a[1])
            out.append((a[0] + (b[0] - a[0]) * s, chord_z))
    return [pt(p) for p in out], cz, R, apothem


def arc_ring(R_out_pts, cz, r_in, ang0, ang1, steps):
    """A ring segment between angles ang0 and ang1 (degrees, 0 is +x, 90 the bow): the room's
    own wall corners outside, a polygonal arc of radius r_in inside."""
    def ang(p):
        return math.degrees(math.atan2(p[1] - cz, p[0]))
    outer = [p for p in R_out_pts if ang0 <= ang(p) <= ang1]
    outer.sort(key=ang)
    def on_wall(a):
        # where the ray from the centre at angle a meets the room's wall
        best = None
        for i in range(len(R_out_pts)):
            p, q = R_out_pts[i], R_out_pts[(i + 1) % len(R_out_pts)]
            dx, dz = math.cos(math.radians(a)), math.sin(math.radians(a))
            ex, ez = q[0] - p[0], q[1] - p[1]
            den = dx * ez - dz * ex
            if abs(den) < 1e-9:
                continue
            s = ((p[0]) * ez - (p[1] - cz) * ex) / den
            u = ((p[0]) * dz - (p[1] - cz) * dx) / den
            if s > 0 and -1e-9 <= u <= 1 + 1e-9:
                best = (dx * s, cz + dz * s)
        return best
    outer = [on_wall(ang0)] + outer + [on_wall(ang1)]
    inner = [(r_in * math.cos(math.radians(a)), cz + r_in * math.sin(math.radians(a))) for a in [ang0 + (ang1 - ang0) * k / steps for k in range(steps + 1)]]
    poly = outer + list(reversed(inner))
    if layout_check.signed_area(poly) < 0:
        poly = list(reversed(poly))
    return [pt(p) for p in poly], len(outer), len(inner)


def variant_b():
    """Round: a ring two steps up all round the sides, a well, the captain on a round dais, helm and tactical at one curved console."""
    room, cz, R, apothem = ngon_room()
    r_in = apothem - 2.0
    ring_p, n_out, n_in = arc_ring(room, cz, r_in, -78.0, 40.0, 6)
    # Edges: the outer run is wall; the closing edges and the inner arc are risers, rails, steps.
    edges = []
    for i in range(len(ring_p)):
        edges.append("wall")
    # Mark edges by geometry: an edge whose midpoint lies on the room wall is "wall", else rail.
    def on_room_wall(a, b):
        mx, mz = (a[0] + b[0]) / 2, (a[1] + b[1]) / 2
        for i in range(len(room)):
            p, q = room[i], room[(i + 1) % len(room)]
            nx, nz = layout_check.outward_normal(p, q)
            if abs((mx - p[0]) * nx + (mz - p[1]) * nz) < 0.02:
                return True
        return False
    for i in range(len(ring_p)):
        a, b = ring_p[i], ring_p[(i + 1) % len(ring_p)]
        edges[i] = "wall" if on_room_wall(a, b) else "rail"
    # The two short closing edges (ring ends) are steps.
    for i in range(len(ring_p)):
        a, b = ring_p[i], ring_p[(i + 1) % len(ring_p)]
        if edges[i] == "rail" and math.hypot(b[0] - a[0], b[1] - a[1]) > 1.9:
            ang_a = math.degrees(math.atan2(a[1] - cz, a[0]))
            ang_b = math.degrees(math.atan2(b[1] - cz, b[0]))
            if abs(ang_a - ang_b) < 1.0:
                edges[i] = "step"
    dais = octagon(0.0, 25.0, 1.1)
    dais_edges = ["rail", "rail", "rail", "riser", "rail", "rail", "rail", "step"]
    sub = [pt(p) for p in [(-2.6, 26.6), (2.6, 26.6), (3.0, 27.2), (3.0, 29.2), (-3.0, 29.2), (-3.0, 27.2)]]
    sub_edges = ["riser"] * len(sub)
    platforms = [
        {"id": "ring_p", "name": "Port ring", "poly": ring_p, "top_m": r3(RING), "edges": edges},
        {"id": "ring_s", "name": "Starboard ring", "poly": mirror_x(ring_p), "top_m": r3(RING), "edges": mirror_edges(edges)},
        {"id": "sub", "name": "Helm and tactical sub-platform", "poly": sub, "top_m": r3(SUB), "edges": sub_edges},
        {"id": "command", "name": "Captain's dais", "poly": dais, "top_m": r3(RING), "edges": dais_edges},
    ]
    def ring_end(a_deg, inward):
        a = math.radians(a_deg)
        mid_r = (apothem + r_in) / 2
        c = (mid_r * math.cos(a), cz + mid_r * math.sin(a))
        tdir = (-math.sin(a), math.cos(a))  # along increasing angle
        foot = (c[0] + tdir[0] * 0.5 * inward, c[1] + tdir[1] * 0.5 * inward)
        return c, foot
    st = []
    c, foot = ring_end(-78.0, -1)
    st.append(stair(c, foot, 1.6, RING, FLOOR, "Port ring down to the door aisle."))
    c, foot = ring_end(40.0, 1)
    st.append(stair(c, foot, 1.6, RING, FLOOR, "Port ring down to the forward walkway."))
    st += [mirror_rec(st[0]), mirror_rec(st[1])]
    st[2]["note"] = "Starboard ring down to the door aisle."
    st[3]["note"] = "Starboard ring down to the forward walkway."
    # The dais's aft step: 1.1 m behind the captain's seat (z 25.0), its foot 0.5 m further aft.
    st.append(stair((0.0, 25.0 - 1.1), (0.0, 25.0 - 1.6), 1.0, RING, FLOOR, "Captain's dais down to the aisle."))
    props, seats = [], []
    # Wall banks on the ring's wall edges, by the wall's angle.
    walls = []
    for i in range(len(room)):
        a, b = room[i], room[(i + 1) % len(room)]
        m = ((a[0] + b[0]) / 2, (a[1] + b[1]) / 2)
        walls.append((math.degrees(math.atan2(m[1] - cz, m[0])), a, b))
    def bank_at(deg, station, prop, seat=True):
        w = min(walls, key=lambda w: abs(w[0] - deg))
        a, b = w[1], w[2]
        ln = math.hypot(b[0] - a[0], b[1] - a[1])
        return wall_bank(station, prop, a, b, ln / 2, RING, seat)
    for deg, station, prop, seat in ((-67.5, "status_p", "wall_bank", False), (-45.0, "comms", "wall_bank", True),
                                     (-22.5, "spare_p", "wall_bank", False), (0.0, "engineering", "wall_bank_core", True),
                                     (22.5, "repeater_p", "wall_bank", False)):
        pr, se = bank_at(deg, station, prop, seat)
        props.append(pr)
        if se:
            seats.append(se)
    pairs = (("status_p", "status_s"), ("comms", "flight_ops"), ("spare_p", "spare_s"), ("engineering", "science"), ("repeater_p", "repeater_s"))
    for st_p, st_s in pairs:
        props += [mirror_rec(p, st_s) for p in props if p["station"] == st_p]
        seats += [mirror_rec(s, st_s) for s in seats if s["station"] == st_p]
    props += [
        {"prop": "helm_arc", "station": "helm_tactical", "back_m": [0.0, r3(SUB), 28.9], "yaw_deg": 180.0},
        {"prop": "captain_chair", "station": "captain", "back_m": [0.0, r3(RING), r3(25.0 - CHAIR_SEAT)], "yaw_deg": 0.0},
    ]
    # The curved helm's two operators, turned to face the bow (yaw 180 sends its +x to -X): helm to port.
    (hx, hz), (tx, tz) = operator("helm_arc", 0), operator("helm_arc", 1)
    seats += [
        {"station": "helm", "seat_m": [r3(-hx), r3(SUB), r3(28.9 - hz)], "yaw_deg": 0.0},
        {"station": "tactical", "seat_m": [r3(-tx), r3(SUB), r3(28.9 - tz)], "yaw_deg": 0.0},
        {"station": "captain", "seat_m": [0.0, r3(RING), 25.0], "yaw_deg": 0.0},
    ]
    # B moves the walls, so the windows and the viewscreen move with them.
    win = [w for w in walls if 41.0 < w[0] < 50.0][0]
    a, b = win[1], win[2]
    m = ((a[0] + b[0]) / 2, (a[1] + b[1]) / 2)
    nx, nz = layout_check.outward_normal(a, b)
    portals = [
        {"id": "p_bridge_window_p", "center_m": [r3(m[0]), 5.0, r3(m[1])], "normal": [round(nx, 4), 0.0, round(nz, 4)], "size_m": [2.0, 1.2]},
        {"id": "p_bridge_window_s", "center_m": [r3(-m[0]), 5.0, r3(m[1])], "normal": [round(-nx, 4), 0.0, round(nz, 4)], "size_m": [2.0, 1.2]},
    ]
    fixtures = [{"id": "viewscreen", "center_m": [0.0, 5.2, 31.5]}]
    return {
        "id": "B", "name": "Round",
        "after": "R1, R3 and R8 (the circular bridge with a ring), R2 (the captain's dais and a helm well)",
        "summary": "A sixteen-sided room, 12.4 m across, with a flat bow for the viewscreen. A ring two steps up runs round both sides with a console built into every wall segment and a rail along its edge; the aisle from the aft door leads into the well; the captain's round dais is at ring height; helm and tactical share one curved console a step up in front.",
        "brushes": [{"y": [FLOOR, CEIL], "poly": room}], "portals": portals, "fixtures": fixtures,
        "platforms": platforms, "stairs": st, "props": props, "seats": seats,
    }


def variant_c():
    """Split level: an upper level at the back for command and comms, the lower front for helm, tactical and the side stations."""
    z_r = 25.6
    _, t, _ = wall_frame(P_SIDE, P_WIN, 0)
    side_x = P_SIDE[0] + t[0] * ((z_r - P_SIDE[1]) / t[1])
    upper = [pt(p) for p in [(1.1, 20.0), P_AFT, P_SIDE, (side_x, z_r), (-side_x, z_r), (-P_SIDE[0], P_SIDE[1]),
                             (-P_AFT[0], P_AFT[1]), (-1.1, 20.0), (-1.1, 21.1), (1.1, 21.1)]]
    # Edge kinds in polygon order; the riser across the room is a rail with gaps (the captain, two stairs).
    edges = ["wall", "wall", "wall", "rail", "wall", "wall", "wall", "riser", "step", "riser"]
    platforms = [{"id": "upper", "name": "Upper level", "poly": upper, "top_m": r3(RING), "edges": edges,
                  "rail_gaps_m": [[-4.6, -3.4], [-0.9, 0.9], [3.4, 4.6]]}]
    stairs = [
        stair((0.0, 21.1), (0.0, 20.6), 2.2, RING, FLOOR, "From the aft door's landing up to the upper level."),
        stair((4.0, z_r), (4.0, z_r + 0.5), 1.2, RING, FLOOR, "Port stair down to the lower level."),
        stair((-4.0, z_r), (-4.0, z_r + 0.5), 1.2, RING, FLOOR, "Starboard stair down to the lower level."),
    ]
    props, seats = [], []
    for st, prop, u, fl in (("comms", "wall_bank", 2.1, RING), ("engineering", "wall_bank_core", 6.8, FLOOR)):
        pr, se = wall_bank(st, prop, P_SIDE, P_WIN, u, fl)
        props.append(pr); seats.append(se)
    pr, _ = wall_bank("status_p", "wall_bank_double", P_AFT, (1.1, 20.0), 3.2, RING, seat=False)
    props.append(pr)
    for st_p, st_s in (("comms", "flight_ops"), ("engineering", "science"), ("status_p", "status_s")):
        props += [mirror_rec(p, st_s) for p in props if p["station"] == st_p]
        seats += [mirror_rec(s, st_s) for s in seats if s["station"] == st_p]
    props += [
        {"prop": "free_console", "station": "helm", "back_m": [1.8, FLOOR, r3(28.0 + DESK_FROM_SEAT)], "yaw_deg": 180.0},
        {"prop": "free_console", "station": "tactical", "back_m": [-1.8, FLOOR, r3(28.0 + DESK_FROM_SEAT)], "yaw_deg": 180.0},
        {"prop": "captain_chair", "station": "captain", "back_m": [0.0, r3(RING), r3(24.9 - CHAIR_SEAT)], "yaw_deg": 0.0},
        {"prop": "standup_console", "station": "repeater_p", "back_m": [6.3, r3(RING), 25.3], "yaw_deg": 180.0},
        {"prop": "standup_console", "station": "repeater_s", "back_m": [-6.3, r3(RING), 25.3], "yaw_deg": 180.0},
    ]
    seats += [
        {"station": "helm", "seat_m": [1.8, FLOOR, 28.0], "yaw_deg": 0.0},
        {"station": "tactical", "seat_m": [-1.8, FLOOR, 28.0], "yaw_deg": 0.0},
        {"station": "captain", "seat_m": [0.0, r3(RING), 24.9], "yaw_deg": 0.0},
    ]
    return {
        "id": "C", "name": "Split level",
        "after": "R9 (the two-level bridge split by a riser), R2 (the sunken helm well)",
        "summary": "The wedge split across by a two-step riser. The upper level at the back holds the aft door's landing, comms and flight operations built into the walls, two stand-up repeaters and the captain at the riser's edge; the lower front holds helm and tactical facing the viewscreen and engineering and science built into the forward walls.",
        "brushes": BRIDGE["brushes"], "platforms": platforms, "stairs": stairs, "props": props, "seats": seats,
    }


def check(variant):
    """Patch a copy of the layout with the variant's room and seats, and run the layout checks on it."""
    L = copy.deepcopy(LAYOUT_DOC)
    br = next(c for c in L["compartments"] if c["id"] == "bridge")
    br["brushes"] = copy.deepcopy(variant["brushes"])
    for p in variant.get("portals", []):
        q = next(x for x in L["portals"] if x["id"] == p["id"])
        q.update({k: v for k, v in p.items() if k != "id"})
    for f_ in variant.get("fixtures", []):
        q = next(x for x in L["fixtures"] if x["id"] == f_["id"])
        q.update({k: v for k, v in f_.items() if k != "id"})
    for s in variant["seats"]:
        q = next((x for x in L["stations"] if x["id"] == s["station"]), None)
        if q:
            q["seat_m"], q["yaw_deg"] = s["seat_m"], s["yaw_deg"]
    tmp = os.path.join(os.path.dirname(LAYOUT), ".bridge_variant_check.json")
    with open(tmp, "w", encoding="utf-8") as f:
        json.dump(L, f)
    buf = io.StringIO()
    try:
        with redirect_stdout(buf):
            ok = layout_check.check(tmp, quiet=True)
    finally:
        os.remove(tmp)
    problems = [ln.strip() for ln in buf.getvalue().splitlines() if "FAIL" in ln]
    # Platforms, stairs and props must stand inside the room.
    poly = [tuple(p) for p in variant["brushes"][0]["poly"]]
    for pf in variant["platforms"]:
        for x, z in pf["poly"]:
            if not layout_check.inside_poly(poly, x, z, tol=0.01):
                problems.append(f"platform {pf['id']}: corner ({x}, {z}) is outside the room")
    for s in variant["stairs"]:
        for k in ("top_m", "foot_m"):
            if not layout_check.inside_poly(poly, s[k][0], s[k][2], tol=0.01):
                problems.append(f"stair {s['note']}: {k} is outside the room")
    for p in variant["props"]:
        if not layout_check.inside_poly(poly, p["back_m"][0], p["back_m"][2], tol=0.02):
            problems.append(f"prop {p['prop']} ({p['station']}): its back is outside the room")
    m = layout_check.signed_area(poly) * (CEIL - FLOOR)
    return ok and not problems, problems, m


# --------------------------------------------------------------------------- sightlines

EYE_M = 1.20        # seated eye over the seat point (bridge-stations design 11.1)
HEAD_TOP_M = 1.32   # top of a seated head over the seat point
HEAD_HALF_W_M = 0.11


def _angle(a, b):
    dot = sum(x * y for x, y in zip(a, b))
    return math.degrees(math.acos(max(-1.0, min(1.0, dot / (math.dist(a, (0, 0, 0)) * math.dist(b, (0, 0, 0)))))))


def sightlines(name, seats, screen):
    """Print, for each seat, the distance to the screen's centre, what the screen subtends and the head
    turn to it (as design 11.1 measures them), then how much of the screen the heads of the seats in front
    hide from the captain's eye. screen: centre (x, y, z) on a wall facing aft, and its size (w, h)."""
    (cx, cy, cz), (w, h) = screen
    print(f"\n{name}  (screen centre ({cx}, {cy}, {cz}), {w} x {h} m)")
    print(f"  {'station':<12} {'seat x, z':>14} {'eye y':>6} {'dist m':>7} {'h x v deg':>12} {'turn deg':>9}")
    by = {s["station"]: s for s in seats}
    for st in ("captain", "helm", "tactical", "engineering", "science", "comms", "flight_ops"):
        s = by[st]
        x, y, z = s["seat_m"]
        e = (x, y + EYE_M, z)
        d = math.dist(e, (cx, cy, cz))
        hor = _angle((cx - w / 2 - e[0], cy - e[1], cz - e[2]), (cx + w / 2 - e[0], cy - e[1], cz - e[2]))
        ver = _angle((cx - e[0], cy - h / 2 - e[1], cz - e[2]), (cx - e[0], cy + h / 2 - e[1], cz - e[2]))
        yaw = math.radians(s["yaw_deg"])
        face = (math.sin(yaw), math.cos(yaw))
        to = (cx - x, cz - z)
        turn = math.degrees(math.atan2(face[0] * to[1] - face[1] * to[0], face[0] * to[0] + face[1] * to[1]))
        side = "" if abs(turn) < 0.5 else (" right" if turn > 0 else " left")
        print(f"  {st:<12} {x:>6.2f}, {z:>6.2f} {e[1]:>6.2f} {d:>7.2f} {hor:>5.1f} x {ver:>4.1f} {abs(turn):>5.0f}{side}")
    # The heads of the seats between the captain and the screen, projected onto the screen's plane.
    c = by["captain"]["seat_m"]
    eye = (c[0], c[1] + EYE_M, c[2])
    hidden = 0.0
    for st, s in by.items():
        x, y, z = s["seat_m"]
        if st == "captain" or not (eye[2] < z < cz):
            continue
        k = (cz - eye[2]) / (z - eye[2])
        top = eye[1] + (y + HEAD_TOP_M - eye[1]) * k
        x0, x1 = eye[0] + (x - HEAD_HALF_W_M - eye[0]) * k, eye[0] + (x + HEAD_HALF_W_M - eye[0]) * k
        ox = max(0.0, min(x1, cx + w / 2) - max(x0, cx - w / 2))
        oy = max(0.0, min(top, cy + h / 2) - (cy - h / 2))
        hidden += ox * oy
        print(f"  {st}'s head, seen from the captain's eye: x {x0:.2f} to {x1:.2f} m, top {top:.2f} m on the screen's plane")
    print(f"  screen hidden from the captain by heads: {hidden:.2f} m2 of {w * h:.1f} m2 ({100 * hidden / (w * h):.1f} %)")


def today_seats():
    return [{"station": s["id"], "seat_m": s["seat_m"], "yaw_deg": s["yaw_deg"]}
            for s in LAYOUT_DOC["stations"] if s.get("compartment") == "bridge"]


def print_sightlines(variants):
    vs = next(f for f in LAYOUT_DOC["fixtures"] if f["id"] == "viewscreen")
    size = tuple(vs["size_m"])
    sightlines("Today's bridge", today_seats(), (tuple(vs["center_m"]), size))
    for v in variants:
        c = next((f["center_m"] for f in v.get("fixtures", []) if f["id"] == "viewscreen"), vs["center_m"])
        sightlines(f"{v['id']}. {v['name']}", v["seats"], (tuple(c), size))


def main(argv):
    variants = [variant_a(), variant_b(), variant_c()]
    if "--sightlines" in argv:
        print_sightlines(variants)
        return 0
    report = []
    for v in variants:
        ok, problems, vol = check(v)
        v["air_m3"] = round(vol, 1)
        report.append((v["id"], ok, problems, vol))
    doc = {
        "schema": "starcrew.bridge-variants/1",
        "status": ("Proposed (2026-10-05) for the owner to choose from: openspec/changes/bridge-stations design section 11a, "
                   "docs/mockups/bridge-variants.html. Written by tools/bridge_variants.py; edit the script, not this file. "
                   "None is in the layout until the owner picks one."),
        "_rules": [
            "Each variant replaces the bridge's brushes (and, for B, its windows and viewscreen) and its seats; every other part of the layout is unchanged.",
            "Platforms are solid (like the dais), not air: the bridge stays one brush, so its volume is the brush's. top_m is the platform's top. edges[i] is the kind of the edge from poly[i] to poly[i+1]: wall (against the room's wall), riser (a step face), rail (a step face with a railing), step (a riser with a stair at it).",
            "Steps rise 0.225 m: the walkway is the floor (3.5 m), the helm and tactical sub-platform one step up, the work ring two, the captain's platform three (A) or two (B, C).",
            "A prop is placed by its back at floor level (back_m) and the way its front faces (yaw_deg, 0 the bow, +90 port): a wall bank's back is on the wall and its front faces into the room; a free console's or stand-up console's front faces its operator, so its back is the side away from them; a chair faces the way its occupant looks. Props are the Blender-built models of assets/models/bridge/props.json. A seat is a station's seat point and the way its occupant faces.",
            "Stairs use the layout's stair convention: top_m and foot_m are the centres of the flight's top and bottom edges; width_m its width.",
        ],
        "variants": variants,
    }
    text = json.dumps(doc, indent=1, ensure_ascii=False) + "\n"
    for vid, ok, problems, vol in report:
        print(f"  {vid}: {'ok' if ok else 'FAIL'}  air {vol:.1f} m3" + "".join(f"\n    {p}" for p in problems))
    if "--check" in argv:
        with open(OUT, encoding="utf-8") as f:
            stale = f.read() != text
        if stale:
            print(f"  FAIL {os.path.relpath(OUT, ROOT)} is stale; run python3 tools/bridge_variants.py")
        return 1 if stale or not all(r[1] for r in report) else 0
    with open(OUT, "w", encoding="utf-8") as f:
        f.write(text)
    print(f"  wrote {os.path.relpath(OUT, ROOT)}")
    return 0 if all(r[1] for r in report) else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
