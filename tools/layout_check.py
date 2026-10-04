#!/usr/bin/env python3
"""Validate a ship layout (data/ships/<id>/layout.json) and report its numbers.

The layout is the one source for a ship's floor plan (CLAUDE.md section 8). The
mockups, the deck build and the simulation all read it, so this check is what
keeps it honest. It is documentation tooling and a measurement instrument
(CLAUDE.md section 4): it changes nothing, it only refuses a bad layout and
prints the volumes and floor areas the life-support and deck designs quote.

Checks:
  - ids are unique and every reference resolves;
  - boxes are well formed, and no two compartments' boxes overlap;
  - every portal sits on a face both of its compartments share (a ladder, hoist
    or hatch between decks may cross one deck slab) and fits inside that face;
  - every compartment except a pod sits inside the hull, walls and slabs included;
  - every compartment is reachable from the bridge through crew portals;
  - every station seat, fixture and system sits inside its compartment.

Usage:
  python3 tools/layout_check.py [layout.json ...] [--quiet]
With no path it checks every data/ships/*/layout.json. Exit status 1 on failure.
Standard library only.
"""

import glob
import json
import os
import sys

EPS = 1e-6
AXES = "xyz"
CREW_PORTALS = {"door", "hatch", "ladder", "pressure_door"}


def box_overlap(a, b):
    """True when two boxes share volume (touching faces do not count)."""
    return all(a[k][0] < b[k][1] - EPS and b[k][0] < a[k][1] - EPS for k in AXES)


def box_volume(b):
    return (b["x"][1] - b["x"][0]) * (b["y"][1] - b["y"][0]) * (b["z"][1] - b["z"][0])


def hull_at(sections, z):
    """Interpolated hull section at z, or None outside the hull's length."""
    secs = sorted(sections, key=lambda s: s["z_m"])
    if z < secs[0]["z_m"] - EPS or z > secs[-1]["z_m"] + EPS:
        return None
    for lo, hi in zip(secs, secs[1:]):
        if lo["z_m"] - EPS <= z <= hi["z_m"] + EPS:
            t = 0.0 if hi["z_m"] == lo["z_m"] else (z - lo["z_m"]) / (hi["z_m"] - lo["z_m"])
            t = min(1.0, max(0.0, t))
            return {k: lo[k] + (hi[k] - lo[k]) * t for k in ("half_beam_m", "top_m", "bottom_m", "chamfer_m")}
    return dict(secs[-1])


def inside_octagon(sec, x, y):
    """Point (x, y) inside an octagonal hull section, and the margin to its edge in metres."""
    hw, top, bot, c = sec["half_beam_m"], sec["top_m"], sec["bottom_m"], sec["chamfer_m"]
    ax = abs(x)
    margins = [hw - ax, top - y, y - bot]
    # 45 degree corner cuts: |x| - (hw - c) <= top - y  and  |x| - (hw - c) <= y - bot
    margins.append(((top - y) - (ax - (hw - c))) / 2 ** 0.5)
    margins.append(((y - bot) - (ax - (hw - c))) / 2 ** 0.5)
    m = min(margins)
    return m >= -EPS, m


def check(path, quiet=False):
    errors, notes = [], []
    with open(path, encoding="utf-8") as f:
        L = json.load(f)
    conv = L.get("conventions", {})
    wall = float(conv.get("wall_thickness_m", 0.25))
    slab = float(conv.get("deck_slab_m", 0.5))

    decks = {d["id"]: d for d in L["decks"]}
    comps = {}
    for c in L["compartments"]:
        if c["id"] in comps:
            errors.append(f"compartment id {c['id']} is used twice")
        comps[c["id"]] = c
        for d in c["decks"]:
            if d not in decks:
                errors.append(f"{c['id']}: unknown deck {d}")
        for i, b in enumerate(c["boxes"]):
            for k in AXES:
                if not b[k][0] < b[k][1]:
                    errors.append(f"{c['id']} box {i}: {k} range {b[k]} is empty")
    pois = [c["poi"] for c in L["compartments"]]
    if len(set(pois)) != len(pois):
        errors.append("two compartments share a point-of-interest number")

    # No overlaps between compartments, or between boxes of one compartment.
    items = [(c["id"], i, b) for c in L["compartments"] for i, b in enumerate(c["boxes"])]
    for i in range(len(items)):
        for j in range(i + 1, len(items)):
            (ca, ia, a), (cb, ib, b) = items[i], items[j]
            if box_overlap(a, b):
                errors.append(f"{ca} box {ia} overlaps {cb} box {ib}")

    # Hull containment, walls and slabs included.
    sections = L["hull"]["sections"]
    zs = sorted(s["z_m"] for s in sections)
    for cid, i, b in items:
        if comps[cid]["kind"] == "pod":
            continue
        x0, x1 = b["x"][0] - wall, b["x"][1] + wall
        y0, y1 = b["y"][0] - slab, b["y"][1] + slab
        z0, z1 = b["z"][0] - wall, b["z"][1] + wall
        worst = None
        for z in [z0, z1] + [z for z in zs if z0 < z < z1]:
            sec = hull_at(sections, z)
            if sec is None:
                errors.append(f"{cid} box {i}: z {z:.2f} lies outside the hull's length")
                continue
            for x in (x0, x1):
                for y in (y0, y1):
                    ok, m = inside_octagon(sec, x, y)
                    worst = m if worst is None else min(worst, m)
                    if not ok:
                        errors.append(f"{cid} box {i}: corner ({x:.2f}, {y:.2f}, {z:.2f}) is {-m:.2f} m outside the hull")
        comps[cid].setdefault("_hull_margin", []).append(worst)

    # Portals.
    portal_ids = set()
    adj = {cid: set() for cid in comps}
    for p in L["portals"]:
        if p["id"] in portal_ids:
            errors.append(f"portal id {p['id']} is used twice")
        portal_ids.add(p["id"])
        ax = p["axis"]
        others = [k for k in AXES if k != ax]
        if ax == "x":
            half = {"z": p["size_m"][0] / 2, "y": p["size_m"][1] / 2}
        elif ax == "z":
            half = {"x": p["size_m"][0] / 2, "y": p["size_m"][1] / 2}
        else:
            half = {"x": p["size_m"][0] / 2, "z": p["size_m"][1] / 2}
        ctr = dict(zip(AXES, p["center_m"]))
        plane = ctr[ax]
        sides = [s for s in p["between"] if s != "space"]
        for s in sides:
            if s not in comps:
                errors.append(f"portal {p['id']}: unknown compartment {s}")
        if any(s not in comps for s in sides):
            continue
        tol = slab + EPS if (ax == "y" and len(sides) == 2) else EPS
        for s in sides:
            fits = False
            for b in comps[s]["boxes"]:
                on_face = min(abs(b[ax][0] - plane), abs(b[ax][1] - plane)) <= tol
                within = all(b[k][0] - EPS <= ctr[k] - half[k] and ctr[k] + half[k] <= b[k][1] + EPS for k in others)
                if on_face and within:
                    fits = True
                    break
            if not fits:
                errors.append(f"portal {p['id']}: does not sit on a face of {s} (axis {ax} at {plane})")
        if len(sides) == 2 and p["kind"] in CREW_PORTALS:
            adj[sides[0]].add(sides[1])
            adj[sides[1]].add(sides[0])

    # Reachability through crew portals.
    start = "bridge" if "bridge" in comps else next(iter(comps))
    seen, todo = {start}, [start]
    while todo:
        for n in adj[todo.pop()]:
            if n not in seen:
                seen.add(n)
                todo.append(n)
    for cid in comps:
        if cid not in seen:
            errors.append(f"{cid} cannot be reached from {start} through doors, hatches or ladders")

    # Points that must lie inside their compartment.
    def inside(cid, pt, label, y_tol=0.6):
        if cid == "space":
            return
        if cid not in comps:
            errors.append(f"{label}: unknown compartment {cid}")
            return
        for b in comps[cid]["boxes"]:
            if (b["x"][0] - EPS <= pt[0] <= b["x"][1] + EPS and b["z"][0] - EPS <= pt[2] <= b["z"][1] + EPS
                    and b["y"][0] - y_tol <= pt[1] <= b["y"][1] + EPS):
                return
        errors.append(f"{label}: {pt} is not inside {cid}")

    for s in L.get("stations", []):
        inside(s["compartment"], s["seat_m"], f"station {s['id']}")
    for f_ in L.get("fixtures", []):
        inside(f_["compartment"], f_["center_m"], f"fixture {f_['id']}")
    sys_ids = set()
    for s in L.get("systems", []):
        sys_ids.add(s["id"])
        inside(s["compartment"], s["center_m"], f"system {s['id']}")
    mounts = {m["id"] for m in L.get("mounts", [])}
    station_ids = {s["id"] for s in L.get("stations", [])}
    for s in L.get("stations", []):
        if "mount" in s and s["mount"] not in mounts:
            errors.append(f"station {s['id']}: unknown mount {s['mount']}")
    for m in L.get("mounts", []):
        if "crew_seat" in m and m["crew_seat"] not in station_ids:
            errors.append(f"mount {m['id']}: unknown seat {m['crew_seat']}")
    for c in L.get("craft", []):
        if c["bay"] not in comps:
            errors.append(f"craft {c['id']}: unknown bay {c['bay']}")
        if c["cradle"] not in sys_ids:
            errors.append(f"craft {c['id']}: unknown cradle {c['cradle']}")
        if c["drop_door"] not in portal_ids:
            errors.append(f"craft {c['id']}: unknown drop door {c['drop_door']}")
        inside(c["bay"], c["center_m"], f"craft {c['id']}", y_tol=0.0)

    # Report.
    name = L["ship"]["name"]
    print(f"{path}: {name} ({L['ship']['class']})")
    if not quiet:
        print(f"  {'poi':>3}  {'compartment':<16} {'decks':<6} {'volume m3':>10} {'floor m2':>9} {'hull margin m':>14}")
        tot_v = tot_a = 0.0
        for c in sorted(L["compartments"], key=lambda c: c["poi"]):
            v = sum(box_volume(b) for b in c["boxes"])
            floor_y = min(b["y"][0] for b in c["boxes"])
            a = sum((b["x"][1] - b["x"][0]) * (b["z"][1] - b["z"][0]) for b in c["boxes"])
            tot_v += v
            tot_a += a
            hm = c.get("_hull_margin")
            hm_s = f"{min(m for m in hm if m is not None):.2f}" if hm and any(m is not None for m in hm) else "pod"
            print(f"  {c['poi']:>3}  {c['id']:<16} {'+'.join(c['decks']):<6} {v:>10.1f} {a:>9.1f} {hm_s:>14}")
        print(f"  {'':>3}  {'total':<16} {'':<6} {tot_v:>10.1f} {tot_a:>9.1f}")
        print(f"  {len(L['compartments'])} compartments, {len(L['portals'])} portals, "
              f"{len(L.get('stations', []))} stations, {len(L.get('systems', []))} systems, "
              f"{len(L.get('mounts', []))} mounts, {len(L.get('craft', []))} craft")
    for e in errors:
        print(f"  FAIL {e}")
    if not errors:
        print("  ok")
    return not errors


def main(argv):
    quiet = "--quiet" in argv
    paths = [a for a in argv if not a.startswith("--")]
    if not paths:
        root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
        paths = sorted(glob.glob(os.path.join(root, "data", "ships", "*", "layout.json")))
    ok = all([check(p, quiet) for p in paths])
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
