#!/usr/bin/env python3
"""Conduit boxes, doorway runs and feeders for a ship's power grid (openspec/changes/power-routing).

The owner, 2026-10-11: "A power grid needs to connect everything together", "Conduits are placed in breaker boxes
throughout the ship", "Conduit boxes can be turned on or off to route power through the ship".

From the ship's layout (rooms, doorways) and the hand-authored part of its power.json (switchboards, panels, trunks,
ties, loads), by the rules in power.json's `distribution` block, this writes:

- a conduit box in every room that has no switchboard or panel of its own (`bx_<room>`), on the wall beside the
  doorway its feed comes in by;
- a run through every doorway between two rooms (`r_<portal>`), from box to box under the ceilings, unless a trunk or
  a tie already joins the two rooms' nodes; the runs that feed a room for the first time from the powered rooms
  (breadth first from the switchboards' and panels' rooms) are closed at the factory, the rest are spare routes,
  open;
- runs through walls and decks: every box with fewer than `min_runs_per_box` runs (trunks, ties, doorway runs) gets a
  spare to the nearest box in a room that shares a wall or a deck with its own (the owner, 2026-10-11: "panels should
  connect to at least 2 other panels"), else to the nearest box in another room;
- every system's feeds (the owner, 2026-10-11: "systems should be connected to 2 or more panels, so we can swap with
  a different route, it should generally connect to the nearest panel"): its `node`, the nearest panel, and its
  `alt_node`, the next nearest, each with its feeder from that panel to the system (`feeder_m`, `alt_feeder_m`); a
  room's lights stay on its own box;
- the lighting panels: each room lit by its own box;
- where the reactor's feeds, the bus ties and the battery's link run (`path_m` on each generator, tie and the battery),
  from end to end under the ceilings.

Every path it writes runs along the ship's axes (the owner, 2026-10-12: "we've got lots of diagonal lines here that
just take shortest distance", "they look messy and unorganized"): see `orthogonal`. `--check` also fails on a
hand-authored trunk with a leg off the axes.

It never edits a hand-authored node, trunk or tie (but for a generator's or tie's `path_m`), and it is idempotent: generated entries are recognised by their
id prefixes and written again. `--check` fails when power.json is not what the rules give.

Usage: python3 tools/power/boxes.py [--ship tern] [--check]
"""

import argparse
import json
import math
import re
import sys
from collections import deque
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
BOX = "bx_"
RUN = "r_"
# Doorways no conduit runs through: they open to space or are glazed.
NO_RUN = {"window", "bay_door"}


def r2(v):
    """Centimetres, and no negative zero."""
    v = round(v, 2)
    return 0.0 if v == 0 else v


def pt(p):
    return [r2(p[0]), r2(p[1]), r2(p[2])]


def inside_convex(poly, x, z, eps=1e-9):
    n = len(poly)
    return all(
        (poly[(i + 1) % n][0] - poly[i][0]) * (z - poly[i][1]) - (poly[(i + 1) % n][1] - poly[i][1]) * (x - poly[i][0])
        >= -eps
        for i in range(n)
    )


def brush_at(room, p):
    """The room's brush that holds point p, else None."""
    for b in room["brushes"]:
        if b["y"][0] - 0.01 <= p[1] <= b["y"][1] + 0.01 and inside_convex(b["poly"], p[0], p[2]):
            return b
    return None


def floor_brush(room, p):
    """The brush p stands in or over: the one whose footprint holds it, the highest floor at or under p."""
    best = None
    for b in room["brushes"]:
        if inside_convex(b["poly"], p[0], p[2]) and b["y"][0] <= p[1] + 0.05:
            if best is None or b["y"][0] > best["y"][0]:
                best = b
    if best is None:
        best = min(room["brushes"], key=lambda b: abs(b["y"][0] - p[1]))
    return best


def room_of_point(layout, p):
    for c in layout["compartments"]:
        if brush_at(c, p):
            return c["id"]
    return None


def nearest_on_boundary(poly, x, z):
    """The nearest point on a convex footprint's boundary to (x, z): the point, its edge's start and end."""
    best = None
    n = len(poly)
    for i in range(n):
        a, b = poly[i], poly[(i + 1) % n]
        dx, dz = b[0] - a[0], b[1] - a[1]
        l2 = dx * dx + dz * dz or 1e-12
        t = max(0.0, min(1.0, ((x - a[0]) * dx + (z - a[1]) * dz) / l2))
        q = (a[0] + dx * t, a[1] + dz * t)
        d = math.hypot(q[0] - x, q[1] - z)
        if best is None or d < best[0] - 1e-9:
            best = (d, q, a, b, t)
    return best[1], best[2], best[3], best[4]


def place_box(room, portal, rules, deck_m):
    """A box on the wall beside the doorway: the nearest point of the room's floor footprint to the doorway, moved
    along that wall clear of the opening and set into the room, at its height over the floor."""
    c = portal["center_m"]
    n = portal["normal"]
    q = None
    for s in (1.0, -1.0):
        cand = [c[0] + s * n[0] * 0.6, c[1] + s * n[1] * 0.6, c[2] + s * n[2] * 0.6]
        if brush_at(room, cand):
            q = cand
            break
    if q is None:
        q = c
    b = floor_brush(room, q)
    (px, pz), a, e, t = nearest_on_boundary(b["poly"], c[0], c[2])
    ex, ez = e[0] - a[0], e[1] - a[1]
    elen = math.hypot(ex, ez) or 1e-9
    ux, uz = ex / elen, ez / elen
    along = t * elen
    beside = rules["box_beside_door_m"]
    # Clear of the doorway on whichever side of it the wall is long enough; a short wall takes its middle.
    if along + beside <= elen - 0.3:
        s = along + beside
    elif along - beside >= 0.3:
        s = along - beside
    else:
        s = elen / 2
    bx, bz = a[0] + ux * s, a[1] + uz * s
    # Into the room: the edge's inward normal (the footprints run anticlockwise, so inward is to the left).
    ix, iz = -uz, ux
    if not inside_convex(b["poly"], bx + ix * 0.05, bz + iz * 0.05):
        ix, iz = -ix, -iz
    inset = rules["box_into_room_m"]
    return [bx + ix * inset, b["y"][0] + rules["box_height_m"], bz + iz * inset], b


def shared_wall(a, b, gap, min_len):
    """Where two footprints on a deck share a wall: the middle of the longest stretch along which an edge of one runs
    within `gap` of an edge of the other, parallel, for at least `min_len`; None when they share none."""
    best = None
    for pa in (a, b):
        pb = b if pa is a else a
        for i in range(len(pa)):
            p0, p1 = pa[i], pa[(i + 1) % len(pa)]
            ex, ez = p1[0] - p0[0], p1[1] - p0[1]
            el = math.hypot(ex, ez)
            if el < 1e-6:
                continue
            ux, uz = ex / el, ez / el
            for j in range(len(pb)):
                q0, q1 = pb[j], pb[(j + 1) % len(pb)]
                fx, fz = q1[0] - q0[0], q1[1] - q0[1]
                fl = math.hypot(fx, fz)
                if fl < 1e-6 or abs(ux * fz / fl - uz * fx / fl) > 0.1:
                    continue
                d0 = abs((q0[0] - p0[0]) * -uz + (q0[1] - p0[1]) * ux)
                d1 = abs((q1[0] - p0[0]) * -uz + (q1[1] - p0[1]) * ux)
                if max(d0, d1) > gap:
                    continue
                t0 = (q0[0] - p0[0]) * ux + (q0[1] - p0[1]) * uz
                t1 = (q1[0] - p0[0]) * ux + (q1[1] - p0[1]) * uz
                lo, hi = max(0.0, min(t0, t1)), min(el, max(t0, t1))
                if hi - lo >= min_len and (best is None or hi - lo > best[0]):
                    t = (lo + hi) / 2
                    best = (hi - lo, (p0[0] + ux * t, p0[1] + uz * t))
    return best[1] if best else None


def adjacency(room_a, room_b, rules):
    """How two rooms touch: ("wall", the shared wall's middle [x, z], the deck's floor y) for rooms side by side on a
    deck, ("deck", a point [x, z] in both footprints, None) for one over the other; None when they do not touch."""
    gap, min_len = rules["adjacent_gap_m"], rules["min_shared_wall_m"]
    for ba in room_a["brushes"]:
        for bb in room_b["brushes"]:
            overlap = min(ba["y"][1], bb["y"][1]) - max(ba["y"][0], bb["y"][0])
            if overlap > 1.0:
                at = shared_wall(ba["poly"], bb["poly"], gap, min_len)
                if at:
                    return ("wall", at, max(ba["y"][0], bb["y"][0]))
            elif abs(ba["y"][1] - bb["y"][0]) < gap or abs(bb["y"][1] - ba["y"][0]) < gap:
                inside = [p for p in ba["poly"] if inside_convex(bb["poly"], p[0], p[1])]
                inside += [p for p in bb["poly"] if inside_convex(ba["poly"], p[0], p[1])]
                if len(inside) >= 2:
                    return ("deck", (sum(p[0] for p in inside) / len(inside), sum(p[1] for p in inside) / len(inside)), None)
    return None


def run_height(brush, rules, deck_m):
    """Where runs go in a brush: under its ceiling, or under the next deck where the room is taller than a deck."""
    return min(brush["y"][1], brush["y"][0] + deck_m) - rules["run_below_ceiling_m"]


def orthogonal(path, tol=0.02):
    """The path with every leg along the ship's axes, as cable trays run (power-routing 3a). A leg that changes height
    keeps level at its higher end (runs stay under the ceilings) and rises or drops at its lower end's spot. A level leg
    that changes both x and z runs fore and aft on whichever end is nearer the centreline (the corridors and spines)
    and athwartships at the other. A leg that goes out and back along one line (to a wall's crossing and back) is cut
    short, and points along a straight leg are dropped."""
    out = [pt(path[0])]
    for b in path[1:]:
        a = out[-1]
        legs = [(a, b)]
        if abs(b[1] - a[1]) > tol and (abs(b[0] - a[0]) > tol or abs(b[2] - a[2]) > tol):
            mid = [b[0], a[1], b[2]] if b[1] < a[1] else [a[0], b[1], a[2]]
            legs = [(a, mid), (mid, b)]
        for p, q in legs:
            if abs(q[0] - p[0]) > tol and abs(q[2] - p[2]) > tol:
                out.append(pt([p[0], p[1], q[2]] if abs(p[0]) <= abs(q[0]) else [q[0], p[1], p[2]]))
            out.append(pt(q))
    out = dedupe(out)
    axis = lambda u: [abs(x) > tol for x in u]
    i = 1
    while i + 1 < len(out):
        a, b, c = out[i - 1], out[i], out[i + 1]
        u, v = [b[k] - a[k] for k in range(3)], [c[k] - b[k] for k in range(3)]
        if axis(u) == axis(v) and sum(axis(u)) == 1:
            del out[i]
            i = max(1, i - 1)
        else:
            i += 1
    return dedupe(out)


def off_axis(path, tol=0.02):
    """The legs of a path that change more than one of x, y and z."""
    return [(a, b) for a, b in zip(path, path[1:]) if sum(abs(b[k] - a[k]) > tol for k in range(3)) > 1]


def dedupe(path):
    out = []
    for p in path:
        if not out or math.dist(out[-1], p) > 0.01:
            out.append(p)
    return out


def rating(mw, rules):
    want = mw * rules["rating_margin"]
    for r in rules["ratings_mw"]:
        if r >= want - 1e-9:
            return float(r)
    return float(rules["ratings_mw"][-1])


def load_center(layout, l):
    if l.get("center_m"):
        return l["center_m"]
    if l.get("mount"):
        return next(m["center_m"] for m in layout["mounts"] if m["id"] == l["mount"])
    return next(s["center_m"] for s in layout["systems"] if s["id"] == l["system"])


def generate(layout, power):
    rules = power["distribution"]
    decks = sorted(d["floor_y_m"] for d in layout["decks"])
    deck_m = min(b - a for a, b in zip(decks, decks[1:]))
    rooms = {c["id"]: c for c in layout["compartments"]}
    room_order = [c["id"] for c in layout["compartments"]]
    hand_nodes = [n for n in power["nodes"] if not n["id"].startswith(BOX)]
    hand_conduits = [k for k in power["conduits"] if not k["id"].startswith(RUN)]
    nodes_in = {}
    for n in hand_nodes:
        nodes_in.setdefault(n["compartment"], []).append(n)
    portals = [
        p
        for p in layout["portals"]
        if p["kind"] not in NO_RUN and all(r in rooms for r in p["between"])
    ]
    # Breadth first from the rooms with a switchboard or panel: each other room's feed, the doorway it is first
    # reached by (in the layout's order of rooms and doorways).
    powered = [r for r in room_order if r in nodes_in]
    parent = {r: None for r in powered}
    queue = deque(powered)
    while queue:
        r = queue.popleft()
        for p in portals:
            if r not in p["between"]:
                continue
            o = p["between"][1] if p["between"][0] == r else p["between"][0]
            if o not in parent:
                parent[o] = (r, p)
                queue.append(o)
    # The boxes.
    boxes = {}  # room -> node dict
    box_brush = {}
    for r in room_order:
        if r in nodes_in or r not in parent:
            continue
        _, p = parent[r]
        at, b = place_box(rooms[r], p, rules, deck_m)
        boxes[r] = {"id": BOX + r, "name": "Conduit box, " + rooms[r]["name"].lower(), "kind": "box", "compartment": r,
                    "center_m": pt(at)}
        box_brush[r] = b
    all_nodes = hand_nodes + [boxes[r] for r in room_order if r in boxes]
    node_by_id = {n["id"]: n for n in all_nodes}

    def node_brush(n):
        room = rooms[n["compartment"]]
        return box_brush.get(n["compartment"]) or floor_brush(room, n["center_m"])

    def box_for(room, near):
        """The node a room's runs and loads use: its box, or its switchboard or panel nearest `near`."""
        if room in boxes:
            return boxes[room]
        cands = nodes_in.get(room, [])
        return min(cands, key=lambda n: math.dist(n["center_m"], near)) if cands else None

    # Each system's feeds: the nearest panels as wiring runs, not as the crow flies: its own room's first, then its own
    # deck's, then the nearest deck's, nearest first (ties by id).
    load_node, alt_node = {}, {}
    for l in power["loads"]:
        c = load_center(layout, l)
        room = room_of_point(layout, c)
        floor = floor_brush(rooms[room], c)["y"][0] if room else c[1]

        def key(n, room=room, floor=floor, c=c):
            decks_off = round(abs(node_brush(n)["y"][0] - floor) / deck_m)
            return (n["compartment"] != room, decks_off, math.dist(n["center_m"], c), n["id"])

        near = sorted(all_nodes, key=key)
        load_node[l["id"]] = near[0]["id"]
        if rules["feeds_per_system"] >= 2:
            alt_node[l["id"]] = near[1]["id"]
    # Lighting: each room lit by its own node.
    area = {}
    for c in layout["compartments"]:
        a = 0.0
        for b in c["brushes"]:
            poly = b["poly"]
            a += 0.5 * sum(poly[i][0] * poly[(i + 1) % len(poly)][1] - poly[(i + 1) % len(poly)][0] * poly[i][1]
                           for i in range(len(poly)))
        area[c["id"]] = a
    old_lit = {room: node for node, rs in power["lighting"]["panels"].items() for room in rs}
    lit_by = {}
    for r in room_order:
        if r in boxes:
            lit_by[r] = boxes[r]["id"]
        elif r in nodes_in:
            keep = old_lit.get(r)
            lit_by[r] = keep if keep in {n["id"] for n in nodes_in[r]} else nodes_in[r][0]["id"]
    panels = {}
    for r in room_order:
        if r in lit_by:
            panels.setdefault(lit_by[r], []).append(r)
    # Demand under each node (its loads' most, and its rooms' lights), for the feed runs' ratings.
    w_m2 = power["lighting"]["normal_w_per_m2"]
    demand = {n["id"]: 0.0 for n in all_nodes}
    for l in power["loads"]:
        demand[load_node[l["id"]]] += l["nominal_mw"] * l["setpoint_max"]
    for node, rs in panels.items():
        demand[node] += sum(area[r] for r in rs) * w_m2 / 1e6
    children = {}
    for r, pr in parent.items():
        if pr:
            children.setdefault(pr[0], []).append(r)

    def subtree_mw(room):
        n = box_for(room, [0, 0, 0])
        return demand[n["id"]] + sum(subtree_mw(c) for c in children.get(room, []))

    joined = set()
    for k in hand_conduits + power["ties"]:
        a, b = (node_by_id[k["between"][0]]["compartment"], node_by_id[k["between"][1]]["compartment"])
        joined.add(frozenset((a, b)))
    # The doorway runs.
    runs = []
    feed_rating = {}
    for p in portals:
        ra, rb = p["between"]
        if frozenset((ra, rb)) in joined or ra == rb:
            continue
        na, nb = box_for(ra, p["center_m"]), box_for(rb, p["center_m"])
        if na is None or nb is None:
            continue
        tree = parent.get(rb) == (ra, p) or parent.get(ra) == (rb, p)
        if tree:
            child = rb if parent.get(rb) == (ra, p) else ra
            cap = rating(subtree_mw(child), rules)
            feed_rating[child] = cap
        else:
            cap = None
        ha, hb = run_height(node_brush(na), rules, deck_m), run_height(node_brush(nb), rules, deck_m)
        a, b, c = na["center_m"], nb["center_m"], p["center_m"]
        path = orthogonal(dedupe([pt(a), pt([a[0], ha, a[2]]), pt([c[0], ha, c[2]]), pt([c[0], hb, c[2]]),
                                  pt([b[0], hb, b[2]]), pt(b)]))
        runs.append({"id": RUN + p["id"][2:] if p["id"].startswith("p_") else RUN + p["id"],
                     "name": "Run through " + p["id"][2:].replace("_", " "), "between": [na["id"], nb["id"]],
                     "capacity_mw": cap, "route": [ra, rb], "path_m": path, "factory_closed": tree})
    # Runs through walls and decks: every node joined to fewer other nodes than the rule gets spares to its
    # neighbours' boxes.
    pairs = set()
    for k in hand_conduits + power["ties"] + runs:
        pairs.add(frozenset(k["between"]))
    degree = {n["id"]: sum(1 for p in pairs if n["id"] in p) for n in all_nodes}
    touch = {}
    for i, ra in enumerate(room_order):
        for rb in room_order[i + 1 :]:
            t = adjacency(rooms[ra], rooms[rb], rules)
            if t:
                touch[(ra, rb)] = t
                touch[(rb, ra)] = t
    for n in all_nodes:
        ra = n["compartment"]
        while degree[n["id"]] < rules["min_runs_per_box"]:
            def cands(adjacent_only):
                out = []
                for rb in room_order:
                    if rb == ra or (adjacent_only and (ra, rb) not in touch):
                        continue
                    for m in [boxes[rb]] if rb in boxes else nodes_in.get(rb, []):
                        if frozenset((n["id"], m["id"])) not in pairs:
                            # The same deck first, then the nearest deck, then the nearest box.
                            decks_off = round(abs(node_brush(n)["y"][0] - node_brush(m)["y"][0]) / deck_m)
                            out.append((decks_off, math.dist(n["center_m"], m["center_m"]), m["id"], m, rb))
                return sorted(out, key=lambda c: (c[0], c[1], c[2]))
            found = cands(True) or cands(False)
            if not found:
                break
            _, _, _, m, rb = found[0]
            t = touch.get((ra, rb))
            ha, hb = run_height(node_brush(n), rules, deck_m), run_height(node_brush(m), rules, deck_m)
            a, b = n["center_m"], m["center_m"]
            if t and t[0] == "wall":
                (wx, wz), h = t[1], min(ha, hb)
                path = [pt(a), pt([a[0], ha, a[2]]), pt([wx, h, wz]), pt([b[0], hb, b[2]]), pt(b)]
                how, word = "wall", "through the wall"
            elif t:
                wx, wz = t[1]
                path = [pt(a), pt([a[0], ha, a[2]]), pt([wx, ha, wz]), pt([wx, hb, wz]), pt([b[0], hb, b[2]]), pt(b)]
                how, word = "deck", "through the deck"
            else:
                path = [pt(a), pt([a[0], ha, a[2]]), pt([b[0], hb, b[2]]), pt(b)]
                how, word = "hull", "along the hull"
            runs.append({"id": f"{RUN}{how}_{ra}_{rb}", "name": f"Run {word}, {ra.replace('_', ' ')} to {rb.replace('_', ' ')}",
                         "between": [n["id"], m["id"]], "capacity_mw": None, "route": [ra, rb], "path_m": orthogonal(path),
                         "factory_closed": False})
            degree[n["id"]] += 1
            degree[m["id"]] += 1
            pairs.add(frozenset((n["id"], m["id"])))
    for k in runs:
        if k["capacity_mw"] is None:
            ra, rb = k["route"]
            k["capacity_mw"] = max(float(rules["spare_mw"]), feed_rating.get(ra, 0.0), feed_rating.get(rb, 0.0))
    # Feeders: from the panel, under its ceiling, over the system, down to it.
    def feeder(node_id, c):
        n = node_by_id[node_id]
        h = run_height(node_brush(n), rules, deck_m)
        a = n["center_m"]
        return orthogonal(dedupe([pt(a), pt([a[0], h, a[2]]), pt([c[0], h, c[2]]), pt(c)]))

    feeders, alt_feeders = {}, {}
    for l in power["loads"]:
        c = load_center(layout, l)
        feeders[l["id"]] = feeder(load_node[l["id"]], c)
        if l["id"] in alt_node:
            alt_feeders[l["id"]] = feeder(alt_node[l["id"]], c)
    # The reactor's feeds, the ties and the battery's link: from end to end at each end's run height, or at the end's
    # own height where it stands above that (the reactor in a room two decks tall: no drop to the tray and back up).
    def between(a, b):
        ha = max(a[1], run_height(floor_brush(rooms[room_of_point(layout, a)], a), rules, deck_m))
        hb = max(b[1], run_height(floor_brush(rooms[room_of_point(layout, b)], b), rules, deck_m))
        return orthogonal(dedupe([pt(a), pt([a[0], ha, a[2]]), pt([b[0], hb, b[2]]), pt(b)]))

    sys_at = {s["id"]: s["center_m"] for s in layout["systems"]}
    gen_paths = {g["id"]: between(sys_at[power["reactor"]["system"]], node_by_id[g["node"]]["center_m"])
                 for g in power["generators"]}
    tie_paths = {t["id"]: between(node_by_id[t["between"][0]]["center_m"], node_by_id[t["between"][1]]["center_m"])
                 for t in power["ties"]}
    battery_path = between(sys_at[power["battery"]["system"]], node_by_id[power["battery"]["node"]]["center_m"])
    return {"boxes": [boxes[r] for r in room_order if r in boxes], "runs": runs, "load_node": load_node,
            "alt_node": alt_node, "feeders": feeders, "alt_feeders": alt_feeders, "panels": panels,
            "gen_paths": gen_paths, "tie_paths": tie_paths, "battery_path": battery_path}


def fmt(v):
    return json.dumps(v, ensure_ascii=False)


def entry_line(d, keys):
    return "    { " + ", ".join(f"{fmt(k)}: {fmt(d[k])}" for k in keys if k in d) + " }"


def rewrite(text, gen):
    """Write the generated parts into power.json's text, keeping its hand formatting."""
    lines = text.split("\n")

    def array_span(key):
        start = next(i for i, l in enumerate(lines) if l.startswith(f'  "{key}": ['))
        end = next(i for i in range(start + 1, len(lines)) if lines[i].startswith("  ]"))
        return start, end

    def entries(start, end):
        """The array's entries as (first line, last line) spans: an entry starts at a line opening with '    {'."""
        heads = [i for i in range(start + 1, end) if lines[i].startswith("    {")]
        spans = []
        for j, h in enumerate(heads):
            last = (heads[j + 1] - 1) if j + 1 < len(heads) else end - 1
            while last > h and not lines[last].strip():
                last -= 1
            spans.append((h, last))
        return spans

    def set_array(key, keep_prefix_out, new_lines):
        nonlocal lines
        s, e = array_span(key)
        kept = []
        for a, b in entries(s, e):
            block = lines[a : b + 1]
            m = re.match(r'\s*\{ "id": "([^"]+)"', block[0])
            if m and m.group(1).startswith(keep_prefix_out):
                continue
            kept.append(block)
        body = []
        for block in kept:
            body.extend(block)
        body.extend(new_lines)
        # Commas: every entry but the last ends with one.
        heads = [i for i, l in enumerate(body) if l.startswith("    {")]
        for j, h in enumerate(heads):
            last = (heads[j + 1] - 1) if j + 1 < len(heads) else len(body) - 1
            body[last] = body[last].rstrip().rstrip(",") + ("," if j + 1 < len(heads) else "")
        lines = lines[: s + 1] + body + lines[e:]

    set_array("nodes", BOX, [entry_line(n, ["id", "name", "kind", "compartment", "center_m"]) for n in gen["boxes"]])
    run_lines = []
    for k in gen["runs"]:
        run_lines.append("    { " + ", ".join(f"{fmt(x)}: {fmt(k[x])}" for x in ["id", "name", "between", "capacity_mw"]) + ",")
        run_lines.append("      " + ", ".join(f"{fmt(x)}: {fmt(k[x])}" for x in ["route", "factory_closed", "path_m"]) + " }")
    set_array("conduits", RUN, run_lines)
    # Loads: the node in place, the feeder as its own line at the end of the entry.
    s, e = array_span("loads")
    for a, b in reversed(entries(s, e)):
        m = re.match(r'\s*\{ "id": "([^"]+)"', lines[a])
        lid = m.group(1)
        had = [l for l in lines[a : b + 1] if l.strip().startswith('"feeder_m"')]
        block = [l for l in lines[a : b + 1] if not l.strip().startswith('"feeder_m"')]
        joined = "\n".join(block)
        joined = re.sub(r'"node": "[^"]+"', f'"node": "{gen["load_node"][lid]}"', joined, count=1)
        block = joined.split("\n")
        comma = (had[-1] if had else block[-1]).rstrip().endswith(",")
        last = block[-1].rstrip().rstrip(",")
        if not had:
            assert last.endswith("}"), (lid, last[-80:])
            last = last[:-1].rstrip().rstrip(",")
        block[-1] = last + ","
        tail = f'      "feeder_m": {fmt(gen["feeders"][lid])}'
        if lid in gen["alt_node"]:
            tail += f', "alt_node": {fmt(gen["alt_node"][lid])}, "alt_feeder_m": {fmt(gen["alt_feeders"][lid])}'
        block.append(tail + " }" + ("," if comma else ""))
        lines = lines[:a] + block + lines[b + 1 :]
    # The generators' and ties' paths, at the end of each one's line.
    for key, paths, keys in (("generators", gen["gen_paths"], ["id", "name", "node", "capacity_mw"]),
                             ("ties", gen["tie_paths"], ["id", "name", "between", "capacity_mw", "closed"])):
        s, e = array_span(key)
        src = {d["id"]: d for d in json.loads("\n".join(["["] + [l.rstrip().rstrip(",") + "," for l in lines[s + 1 : e]])[:-1] + "]")}
        body = [entry_line({**src[i], "path_m": paths[i]}, keys + ["path_m"]) for i in paths]
        lines = lines[: s + 1] + [l + ("," if j + 1 < len(body) else "") for j, l in enumerate(body)] + lines[e:]
    # The battery's link: its own line in the battery block, after its node.
    s = next(i for i, l in enumerate(lines) if l.startswith('  "battery": {'))
    e = next(i for i in range(s + 1, len(lines)) if lines[i].startswith("  }"))
    lines = lines[:s] + [l for l in lines[s:e] if not l.strip().startswith('"path_m"')] + lines[e:]
    e = next(i for i in range(s + 1, len(lines)) if lines[i].startswith("  }"))
    at = next(i for i in range(s + 1, e) if lines[i].strip().startswith('"node"'))
    lines.insert(at + 1, f'    "path_m": {fmt(gen["battery_path"])},')
    # Lighting panels.
    s = next(i for i, l in enumerate(lines) if l.startswith('    "panels": {'))
    e = next(i for i in range(s + 1, len(lines)) if lines[i].startswith("    }"))
    items = list(gen["panels"].items())
    body = [f"      {fmt(k)}: {fmt(v)}" + ("," if j + 1 < len(items) else "") for j, (k, v) in enumerate(items)]
    lines = lines[: s + 1] + body + lines[e:]
    return "\n".join(lines)


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--ship", default="tern")
    ap.add_argument("--check", action="store_true")
    a = ap.parse_args()
    lp = ROOT / "data" / "ships" / a.ship / "layout.json"
    pp = ROOT / "data" / "ships" / a.ship / "power.json"
    layout = json.loads(lp.read_text())
    text = pp.read_text()
    power = json.loads(text)
    gen = generate(layout, power)
    out = rewrite(text, gen)
    json.loads(out)  # still JSON
    live = sum(1 for k in power["conduits"] if not k["id"].startswith(RUN) and k.get("factory_closed", True))
    live += sum(1 for t in power["ties"] if t["closed"]) + sum(1 for k in gen["runs"] if k["factory_closed"])
    spare = sum(1 for k in gen["runs"] if not k["factory_closed"])
    summary = (f"{len(gen['boxes'])} boxes, {len(gen['runs'])} runs through doorways, walls and decks ({spare} spare), "
               f"{live} runs live at the factory, limit {power['routing']['max_live_runs']}")
    crooked = [k["id"] for k in power["conduits"] if not k["id"].startswith(RUN) and off_axis(k["path_m"])]
    if crooked:
        print(f"{pp.relative_to(ROOT)}: hand-authored paths off the ship's axes (power-routing 3a): {', '.join(crooked)}",
              file=sys.stderr)
        if a.check:
            return 1
    if a.check:
        if out != text:
            print(f"{pp.relative_to(ROOT)}: stale; run tools/power/boxes.py", file=sys.stderr)
            return 1
        print(f"ok: {summary}")
        return 0
    if out != text:
        pp.write_text(out)
    print(f"wrote {pp.relative_to(ROOT)}: {summary}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
