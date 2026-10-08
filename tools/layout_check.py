#!/usr/bin/env python3
"""Validate a ship layout (data/ships/<id>/layout.json) and report its numbers.

The layout is the one source for a ship's floor plan (CLAUDE.md section 8). The
mockups, the deck build and the simulation all read it, so this check is what
keeps it honest. It is documentation tooling and a measurement instrument
(CLAUDE.md section 4): it changes nothing, it only refuses a bad layout and
prints the volumes and floor areas the life-support and deck designs quote.

Schema starcrew.ship-layout/2: a compartment's air is the union of convex
prisms ("brushes"), each a footprint polygon in plan (x, z) and a floor and
ceiling height, so a room can follow the hull instead of being a box
(openspec/changes/reference-ship-tern, section 10).

Checks:
  - ids are unique and every reference resolves;
  - every brush is convex, wound with positive area, and has a floor below its ceiling;
  - no two brushes overlap, in one compartment or across two (touching is fine);
  - every wall portal lies on a wall both its compartments share, facing along its
    normal, and fits inside that wall; every floor portal (hatch, ladder, hoist,
    bay door) fits inside both footprints, one deck slab apart at most;
  - every compartment except a pod keeps hull.clearance_m of hull outside its air;
  - every compartment is reachable from the bridge through crew portals;
  - every station seat, fixture, system and craft sits inside its compartment;
  - every compartment's finish is defined in detailing.json beside the layout, and every
    material a finish names exists in data/materials/materials.json.

Usage:
  python3 tools/layout_check.py [layout.json ...] [--quiet]
With no path it checks every data/ships/*/layout.json. Exit status 1 on failure.
Standard library only.
"""

import glob
import json
import math
import os
import sys

SCHEMA = "starcrew.ship-layout/2"
EPS = 1e-6
TOL_M = 1e-3          # a point on a wall is within a millimetre of it
TOL_DEG = 0.5         # a portal's normal matches its wall within half a degree
CREW_PORTALS = {"door", "hatch", "ladder", "pressure_door"}


# ------------------------------------------------------------------ plan geometry (x, z)

def signed_area(poly):
    """Signed area in square metres; positive is the layout's winding."""
    n = len(poly)
    return sum(poly[i][0] * poly[(i + 1) % n][1] - poly[(i + 1) % n][0] * poly[i][1] for i in range(n)) / 2.0


def edges(poly):
    n = len(poly)
    return [(poly[i], poly[(i + 1) % n]) for i in range(n)]


def is_convex(poly):
    """Every corner turns left (positive winding); collinear corners are refused."""
    n = len(poly)
    for i in range(n):
        a, b, c = poly[i - 1], poly[i], poly[(i + 1) % n]
        cross = (b[0] - a[0]) * (c[1] - b[1]) - (b[1] - a[1]) * (c[0] - b[0])
        if cross <= EPS:
            return False
    return True


def outward_normal(a, b):
    """Unit outward normal of edge a->b of a positively wound polygon."""
    dx, dz = b[0] - a[0], b[1] - a[1]
    ln = math.hypot(dx, dz)
    return (dz / ln, -dx / ln)


def inside_poly(poly, x, z, tol=TOL_M):
    """Point inside a convex, positively wound polygon (on the boundary counts)."""
    for a, b in edges(poly):
        nx, nz = outward_normal(a, b)
        if (x - a[0]) * nx + (z - a[1]) * nz > tol:
            return False
    return True


def polys_overlap(p, q):
    """Separating axis test: True when two convex polygons share area (touching does not count)."""
    for poly in (p, q):
        for a, b in edges(poly):
            nx, nz = outward_normal(a, b)
            pa = [x * nx + z * nz for x, z in p]
            qa = [x * nx + z * nz for x, z in q]
            if max(pa) <= min(qa) + TOL_M or max(qa) <= min(pa) + TOL_M:
                return False
    return True


def clip_z(poly, z0, z1):
    """The part of a convex polygon with z0 <= z <= z1 (Sutherland-Hodgman, two planes)."""
    def clip(pts, keep, at):
        out = []
        for i in range(len(pts)):
            a, b = pts[i], pts[(i + 1) % len(pts)]
            ka, kb = keep(a), keep(b)
            if ka:
                out.append(a)
            if ka != kb:
                t = (at - a[1]) / (b[1] - a[1])
                out.append((a[0] + t * (b[0] - a[0]), at))
        return out
    pts = clip(list(poly), lambda p: p[1] >= z0 - EPS, z0)
    if pts:
        pts = clip(pts, lambda p: p[1] <= z1 + EPS, z1)
    return pts


# ------------------------------------------------------------------ hull

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
    """Point (x, y) inside an octagonal hull section, and the margin to its skin in metres."""
    hw, top, bot, c = sec["half_beam_m"], sec["top_m"], sec["bottom_m"], sec["chamfer_m"]
    ax = abs(x)
    margins = [hw - ax, top - y, y - bot]
    # 45 degree corner cuts: |x| - (hw - c) <= top - y  and  |x| - (hw - c) <= y - bot
    margins.append(((top - y) - (ax - (hw - c))) / 2 ** 0.5)
    margins.append(((y - bot) - (ax - (hw - c))) / 2 ** 0.5)
    m = min(margins)
    return m >= -EPS, m


def hull_margin(sections, poly, y0, y1):
    """Least distance from a brush's air to the hull skin, and where. Each hull segment
    between two sections is convex (the sections share their edge directions), so the
    corners of the brush clipped to each segment are the points to test."""
    zs = sorted(s["z_m"] for s in sections)
    worst = (math.inf, None)
    zmin, zmax = min(p[1] for p in poly), max(p[1] for p in poly)
    if zmin < zs[0] - EPS or zmax > zs[-1] + EPS:
        return (-math.inf, (None, None, zmin if zmin < zs[0] else zmax))
    for lo, hi in zip(zs, zs[1:]):
        if hi < zmin - EPS or lo > zmax + EPS:
            continue
        for x, z in clip_z(poly, lo, hi):
            sec = hull_at(sections, z)
            for y in (y0, y1):
                m = inside_octagon(sec, x, y)[1]
                if m < worst[0]:
                    worst = (m, (x, y, z))
    return worst


# ------------------------------------------------------------------ check

def check(path, quiet=False):
    errors = []
    with open(path, encoding="utf-8") as f:
        L = json.load(f)
    if L.get("schema") != SCHEMA:
        errors.append(f"schema is {L.get('schema')!r}, expected {SCHEMA!r}")
        for e in errors:
            print(f"{path}\n  FAIL {e}")
        return False
    slab = float(L.get("conventions", {}).get("deck_slab_m", 0.5))
    clearance = float(L["hull"].get("clearance_m", 0.0))
    sections = L["hull"]["sections"]

    decks = {d["id"]: d for d in L["decks"]}
    comps = {}
    for c in L["compartments"]:
        if c["id"] in comps:
            errors.append(f"compartment id {c['id']} is used twice")
        comps[c["id"]] = c
        for d in c["decks"]:
            if d not in decks:
                errors.append(f"{c['id']}: unknown deck {d}")
        if not c.get("brushes"):
            errors.append(f"{c['id']}: no brushes")
        for i, b in enumerate(c.get("brushes", [])):
            poly = [tuple(p) for p in b["poly"]]
            b["_poly"] = poly
            if len(poly) < 3:
                errors.append(f"{c['id']} brush {i}: fewer than three corners")
                continue
            if not all(math.isfinite(v) for p in poly for v in p) or not all(math.isfinite(v) for v in b["y"]):
                errors.append(f"{c['id']} brush {i}: a coordinate is not finite")
            if signed_area(poly) <= 0:
                errors.append(f"{c['id']} brush {i}: wound the wrong way (signed area {signed_area(poly):.2f})")
            elif not is_convex(poly):
                errors.append(f"{c['id']} brush {i}: not convex (or has a straight corner)")
            if not b["y"][0] < b["y"][1]:
                errors.append(f"{c['id']} brush {i}: y range {b['y']} is empty")
    pois = [c["poi"] for c in L["compartments"]]
    if len(set(pois)) != len(pois):
        errors.append("two compartments share a point-of-interest number")
    if errors:
        for e in errors:
            print(f"  FAIL {e}")
        return False

    # No overlaps between any two brushes whose heights overlap.
    items = [(c["id"], i, b) for c in L["compartments"] for i, b in enumerate(c["brushes"])]
    for i in range(len(items)):
        for j in range(i + 1, len(items)):
            (ca, ia, a), (cb, ib, b) = items[i], items[j]
            if a["y"][0] < b["y"][1] - EPS and b["y"][0] < a["y"][1] - EPS and polys_overlap(a["_poly"], b["_poly"]):
                errors.append(f"{ca} brush {ia} overlaps {cb} brush {ib}")

    # Hull clearance.
    for cid, i, b in items:
        if comps[cid]["kind"] == "pod":
            continue
        m, where = hull_margin(sections, b["_poly"], b["y"][0], b["y"][1])
        comps[cid].setdefault("_hull_margin", []).append(m)
        if where[0] is None:
            errors.append(f"{cid} brush {i}: z {where[2]:.2f} lies outside the hull's length")
        elif m < clearance - TOL_M:
            errors.append(f"{cid} brush {i}: ({where[0]:.2f}, {where[1]:.2f}, {where[2]:.2f}) is {m:.2f} m from the hull, "
                          f"less than the {clearance:.2f} m clearance")

    # Portals.
    portal_ids = set()
    adj = {cid: set() for cid in comps}
    for p in L["portals"]:
        if p["id"] in portal_ids:
            errors.append(f"portal id {p['id']} is used twice")
        portal_ids.add(p["id"])
        n = p["normal"]
        ln = math.sqrt(sum(v * v for v in n))
        if abs(ln - 1.0) > 1e-3:
            errors.append(f"portal {p['id']}: normal {n} is not a unit vector")
            continue
        n = [v / ln for v in n]
        sides = list(p["between"])
        if sides[0] == "space":
            errors.append(f"portal {p['id']}: 'space' must be second, the normal points out of the ship")
            continue
        for s in sides:
            if s != "space" and s not in comps:
                errors.append(f"portal {p['id']}: unknown compartment {s}")
        if any(s != "space" and s not in comps for s in sides):
            continue
        cx, cy, cz = p["center_m"]
        floor = abs(n[1]) > 0.999
        if not floor and abs(n[1]) > 1e-6:
            errors.append(f"portal {p['id']}: normal {n} is neither horizontal nor vertical")
            continue
        for k, s in enumerate(sides):
            if s == "space":
                continue
            out = n if k == 0 else [-v for v in n]  # the portal leaves s along its outward normal
            fits = False
            for b in comps[s]["brushes"]:
                y0, y1 = b["y"]
                if floor:
                    sx, sz = p["size_m"][0] / 2, p["size_m"][1] / 2
                    face_y = y1 if out[1] > 0 else y0
                    on_face = (face_y - cy) * out[1] <= EPS and abs(face_y - cy) <= slab + EPS
                    within = all(inside_poly(b["_poly"], cx + dx, cz + dz) for dx in (-sx, sx) for dz in (-sz, sz))
                    if on_face and within:
                        fits = True
                        break
                else:
                    w, h = p["size_m"][0] / 2, p["size_m"][1] / 2
                    if not (y0 - EPS <= cy - h and cy + h <= y1 + EPS):
                        continue
                    tx, tz = -out[2], out[0]
                    for a, e in edges(b["_poly"]):
                        nx, nz = outward_normal(a, e)
                        if math.degrees(math.acos(max(-1.0, min(1.0, nx * out[0] + nz * out[2])))) > TOL_DEG:
                            continue
                        if abs((cx - a[0]) * nx + (cz - a[1]) * nz) > TOL_M:
                            continue
                        ea = (a[0] * tx + a[1] * tz, e[0] * tx + e[1] * tz)
                        c0 = cx * tx + cz * tz
                        if min(ea) - TOL_M <= c0 - w and c0 + w <= max(ea) + TOL_M:
                            fits = True
                            break
                    if fits:
                        break
            if not fits:
                kind = "floor or ceiling" if floor else "wall"
                errors.append(f"portal {p['id']}: does not fit a {kind} of {s} facing {[round(v, 3) for v in out]}")
        if len(sides) == 2 and "space" not in sides and p["kind"] in CREW_PORTALS:
            adj[sides[0]].add(sides[1])
            adj[sides[1]].add(sides[0])

    # Reachability through crew portals.
    start = "bridge" if "bridge" in comps else next(iter(comps))
    seen, todo = {start}, [start]
    while todo:
        for nb in adj[todo.pop()]:
            if nb not in seen:
                seen.add(nb)
                todo.append(nb)
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
        for b in comps[cid]["brushes"]:
            if inside_poly(b["_poly"], pt[0], pt[2]) and b["y"][0] - y_tol <= pt[1] <= b["y"][1] + EPS:
                return
        errors.append(f"{label}: {pt} is not inside {cid}")

    for s in L.get("stations", []):
        inside(s["compartment"], s["seat_m"], f"station {s['id']}")
    for f_ in L.get("fixtures", []):
        inside(f_["compartment"], f_["center_m"], f"fixture {f_['id']}")
        for k, (x, z) in enumerate(f_.get("poly", [])):
            inside(f_["compartment"], [x, f_["center_m"][1], z], f"fixture {f_['id']} corner {k}")
    sys_ids = set()
    for s in L.get("systems", []):
        sys_ids.add(s["id"])
        inside(s["compartment"], s["center_m"], f"system {s['id']}")
        for k, u in enumerate(s.get("units_m", [])):   # a system built as several machines (the impulse drive's two)
            inside(s["compartment"], u, f"system {s['id']} unit {k}")
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

    # Finishes: every compartment names one the ship's detailing defines, and every role of a
    # finish names a material in data/materials/materials.json (deck-pipeline section 5a).
    det_path = os.path.join(os.path.dirname(os.path.abspath(path)), "detailing.json")
    mat_path = os.path.join(os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(path)))), "data", "materials", "materials.json")
    if os.path.exists(det_path):
        with open(det_path, encoding="utf-8") as f:
            finishes = {k: v for k, v in json.load(f).get("finishes", {}).items() if not k.startswith("_")}
        materials = set()
        if os.path.exists(mat_path):
            with open(mat_path, encoding="utf-8") as f:
                materials = set(json.load(f).get("materials", {}))
        for c in L["compartments"]:
            if c.get("finish") not in finishes:
                errors.append(f"{c['id']}: finish {c.get('finish')!r} is not one of {sorted(finishes)} in detailing.json")
        for name, roles in finishes.items():
            for role, mat in roles.items():
                if materials and mat not in materials:
                    errors.append(f"detailing.json finish {name}: role {role} names material {mat!r}, not in data/materials/materials.json")
    else:
        for c in L["compartments"]:
            if "finish" in c:
                errors.append(f"{c['id']}: has a finish but there is no detailing.json beside the layout")
                break

    # Report.
    name = L["ship"]["name"]
    print(f"{path}: {name} ({L['ship']['class']})")
    if not quiet:
        print(f"  {'poi':>3}  {'compartment':<16} {'decks':<6} {'brushes':>7} {'volume m3':>10} {'floor m2':>9} {'hull margin m':>14}")
        tot_v = tot_a = 0.0
        for c in sorted(L["compartments"], key=lambda c: c["poi"]):
            v = sum(signed_area(b["_poly"]) * (b["y"][1] - b["y"][0]) for b in c["brushes"])
            a = sum(signed_area(b["_poly"]) for b in c["brushes"])
            tot_v += v
            tot_a += a
            hm = c.get("_hull_margin")
            hm_s = f"{min(hm):.2f}" if hm else "pod"
            print(f"  {c['poi']:>3}  {c['id']:<16} {'+'.join(c['decks']):<6} {len(c['brushes']):>7} {v:>10.1f} {a:>9.1f} {hm_s:>14}")
        print(f"  {'':>3}  {'total':<16} {'':<6} {'':>7} {tot_v:>10.1f} {tot_a:>9.1f}")
        print(f"  {len(L['compartments'])} compartments ({len(items)} brushes), {len(L['portals'])} portals, "
              f"{len(L.get('stations', []))} stations, {len(L.get('systems', []))} systems, "
              f"{len(L.get('mounts', []))} mounts, {len(L.get('craft', []))} craft; hull clearance {clearance:.2f} m")
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
