#!/usr/bin/env python3
"""Draw a ship's deck plans as SVG design maps, one per deck, from its layout.

Documentation tooling (CLAUDE.md section 4): it changes nothing in the game and exists to make
the reference-ship-tern write-up readable. It reads the one layout source,
data/ships/<id>/layout.json (CLAUDE.md section 8), and the colour roles in
docs/mockups/lib/shipkit.js, so the maps, the three.js mockups and the deck build cannot
disagree about where a room is or what colour a compartment kind is. Its plan geometry
(polygon area, winding, point in polygon, edge normals, the hull's sections) is
tools/layout_check.py's, imported, so the map and the checker measure a room the same way.

Schema starcrew.ship-layout/2: a compartment is the union of convex brushes, each a footprint
polygon in plan and a floor and ceiling height, so each brush is drawn as its polygon. Where two
brushes of one compartment meet face to face the face is open: it is drawn as no wall when their
floors are level, and as a railing (a thin dashed line) where one floor drops to the other.

Each map is drawn in the style of a Deus Ex hub map (Undercity's standard,
/home/user/fps-game-demo/CLAUDE.md section 1): compartments filled by kind with numbered points
of interest, doors and other portals coloured by kind (a wall portal along its wall, which may be
angled, a floor portal as its opening), ladders, stations as role-coloured seats, systems as
lettered markers, craft, the hull outline at the deck's mid height, a metre grid, a scale bar and
a legend. Plan orientation: bow to the right, port at the top (looking down on the deck from
above). Areas open to the deck below (the hangar, engineering) are hatched.

Usage:
  python3 tools/deck_plans.py [layout.json] [--out docs/design/maps]
With no path it draws data/ships/tern/layout.json. Writes <out>/<ship>-deck-<id>.svg.
Standard library only.
"""

import json
import math
import os
import re
import sys

sys.dont_write_bytecode = True  # importing the checker must not leave a __pycache__ in tools/
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from layout_check import SCHEMA, edges, hull_at, inside_poly, outward_normal, signed_area  # noqa: E402

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SHIPKIT = os.path.join(ROOT, "docs", "mockups", "lib", "shipkit.js")

S = 14.0          # pixels per metre
MARGIN = 44       # around the plan, for the grid labels
HEAD = 64         # title band
KEY_H = 118       # symbol key band under the plan
LEGEND_W = 430    # legend column at the right
GRID_M = 5        # grid spacing, metres
HULL_FILL = "#0d141c"
ROOM_OPACITY = 0.88   # a room's fill over the hull, flattened to one opaque colour (no seams)
LEVEL_M = 0.05    # two brush floors within this are one level (their shared face is open)

# Portal kind colours come from shipkit's PALETTE.portal, the one table the mockups also read.
ROLE_CODE = {
    "command": "CPT", "helm": "HLM", "tactical": "TAC", "engineering": "ENG", "science": "SCI",
    "comms": "COM", "flight_ops": "FLT", "gunner": "GUN",
}


def palette():
    """Parse PALETTE out of shipkit.js: named roles plus the role, portal and kind tables, as #rrggbb."""
    with open(SHIPKIT, encoding="utf-8") as f:
        src = f.read()
    m = re.search(r"const PALETTE = \{(.*?)\n  \};", src, re.S)
    if not m:
        raise SystemExit("deck_plans: no PALETTE in " + SHIPKIT)
    body = m.group(1)
    out = {"role": {}, "portal": {}, "kind": {}}
    for table in ("role", "portal", "kind"):
        t = re.search(table + r":\s*\{(.*?)\}", body, re.S)
        for k, v in re.findall(r"(\w+):\s*0x([0-9a-fA-F]{6})", t.group(1)):
            out[table][k] = "#" + v.lower()
        body = body.replace(t.group(0), "")
    for k, v in re.findall(r"(\w+):\s*0x([0-9a-fA-F]{6})", body):
        out[k] = "#" + v.lower()
    return out


def esc(s):
    return s.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;").replace('"', "&quot;")


def mix(col, under, a):
    """col drawn at opacity a over under, as one opaque #rrggbb."""
    c = [int(col[i:i + 2], 16) for i in (1, 3, 5)]
    u = [int(under[i:i + 2], 16) for i in (1, 3, 5)]
    return "#" + "".join(f"{round(a * x + (1 - a) * y):02x}" for x, y in zip(c, u))


def hull_half_width(sec, y):
    """Half beam of an octagonal section at height y, or None above or below it."""
    hw, top, bot, c = sec["half_beam_m"], sec["top_m"], sec["bottom_m"], sec["chamfer_m"]
    if y > top or y < bot:
        return None
    w = hw
    if y > top - c:
        w = hw - (y - (top - c))
    if y < bot + c:
        w = hw - ((bot + c) - y)
    return w


def poly_of(b):
    return [tuple(p) for p in b["poly"]]


def centroid(poly):
    """Vertex centroid (x, z): well inside a convex polygon, which is all a label needs."""
    return sum(p[0] for p in poly) / len(poly), sum(p[1] for p in poly) / len(poly)


def volume_area(comp):
    """A compartment's air volume (m^3) and floor area (m^2), summed over its brushes."""
    v = sum(signed_area(poly_of(b)) * (b["y"][1] - b["y"][0]) for b in comp["brushes"])
    a = sum(signed_area(poly_of(b)) for b in comp["brushes"])
    return v, a


def is_floor_portal(p):
    return abs(p["normal"][1]) > 0.5


def face_runs(b, others):
    """Split each edge of brush b into runs, each (kind, a, e): 'wall', or where the edge meets a
    face of another brush of the same compartment, 'open' (floors level) or 'drop' (one floor
    lower: a railing). A run is a straight piece of the edge in plan (x, z)."""
    out = []
    for a, e in edges(poly_of(b)):
        dx, dz = e[0] - a[0], e[1] - a[1]
        l2 = dx * dx + dz * dz
        n = outward_normal(a, e)
        cover = []
        for o in others:
            for r, s in edges(poly_of(o)):
                m = outward_normal(r, s)
                if n[0] * m[0] + n[1] * m[1] > -0.9999:
                    continue
                if abs((r[0] - a[0]) * n[0] + (r[1] - a[1]) * n[1]) > 0.02:
                    continue
                t0 = ((r[0] - a[0]) * dx + (r[1] - a[1]) * dz) / l2
                t1 = ((s[0] - a[0]) * dx + (s[1] - a[1]) * dz) / l2
                lo, hi = max(0.0, min(t0, t1)), min(1.0, max(t0, t1))
                if hi - lo > 1e-6:
                    level = abs(o["y"][0] - b["y"][0]) < LEVEL_M
                    cover.append((lo, hi, "open" if level else "drop"))
        at = lambda t: (a[0] + dx * t, a[1] + dz * t)  # noqa: E731
        t = 0.0
        for lo, hi, kind in sorted(cover):
            if lo > t + 1e-6:
                out.append(("wall", at(t), at(lo)))
            if hi > t + 1e-6:
                out.append((kind, at(max(lo, t)), at(hi)))
                t = hi
        if t < 1.0 - 1e-6:
            out.append(("wall", at(t), e))
    return out


class Plan:
    def __init__(self, L, deck, pal):
        self.L, self.deck, self.pal = L, deck, pal
        zs = [s["z_m"] for s in L["hull"]["sections"]]
        xs = [s["half_beam_m"] for s in L["hull"]["sections"]]
        reach = max([max(xs)] + [abs(p[0]) for c in L["compartments"] for b in c["brushes"] for p in b["poly"]])
        self.z0, self.z1 = math.floor(min(zs)) - 2, math.ceil(max(zs)) + 2
        self.x0, self.x1 = -math.ceil(reach) - 2, math.ceil(reach) + 2
        self.pw = (self.z1 - self.z0) * S
        self.ph = (self.x1 - self.x0) * S
        self.W = MARGIN * 2 + self.pw + LEGEND_W
        self.H = HEAD + MARGIN * 2 + self.ph + KEY_H
        self.out = []
        self.floor = deck["floor_y_m"]
        self.band = (self.floor, self.floor + deck["clear_height_m"])

    # ship (x, z) -> svg (X, Y): bow (+z) right, port (+x) up
    def X(self, z):
        return MARGIN + (z - self.z0) * S

    def Y(self, x):
        return HEAD + MARGIN + (self.x1 - x) * S

    def pts(self, poly):
        """A plan polygon [(x, z), ...] as SVG points."""
        return " ".join(f"{self.X(z):.1f},{self.Y(x):.1f}" for x, z in poly)

    def rect(self, cx, cz, sx, sz):
        """SVG x, y, width, height of an axis-aligned plan rectangle: centre (cx, cz), extents sx by sz."""
        return self.X(cz - sz / 2), self.Y(cx + sx / 2), sz * S, sx * S

    def add(self, s):
        self.out.append(s)

    def on_deck(self, comp):
        return self.deck["id"] in comp["decks"]

    def brushes_here(self, comp):
        lo, hi = self.band
        bs = [b for b in comp["brushes"] if min(b["y"][1], hi) - max(b["y"][0], lo) > 0.1]
        return bs or list(comp["brushes"])

    def off_deck(self, b):
        """True for a brush wholly above or below this deck's clear height (a turret pod)."""
        lo, hi = self.band
        return b["y"][0] >= hi - 1e-6 or b["y"][1] <= lo + 1e-6

    def below(self, b):
        """True for a brush whose floor is on a lower deck: open to the deck below, hatched."""
        return b["y"][0] < self.floor - 0.6

    def portal_here(self, p):
        lo, hi = self.band
        y = p["center_m"][1]
        if is_floor_portal(p):
            return lo - 1.0 <= y <= hi + 0.1
        return lo - 1e-6 <= y <= hi + 1e-6


def draw(L, deck, pal, path):
    P = Plan(L, deck, pal)
    comps = [c for c in L["compartments"] if P.on_deck(c)]
    sid = L["ship"]["id"]
    title = f'{L["ship"]["name"]}, deck {deck["id"]}: {deck["name"]}'
    P.add(f'<svg xmlns="http://www.w3.org/2000/svg" width="{P.W:.0f}" height="{P.H:.0f}" '
          f'viewBox="0 0 {P.W:.0f} {P.H:.0f}" font-family="DejaVu Sans, Helvetica, Arial, sans-serif">')
    P.add(f'<title>{esc(title)}</title>')
    P.add('<defs>'
          '<pattern id="below" width="8" height="8" patternUnits="userSpaceOnUse" patternTransform="rotate(45)">'
          '<rect width="8" height="8" fill="#0b1118"/><line x1="0" y1="0" x2="0" y2="8" stroke="#2a3a4c" stroke-width="3"/></pattern>'
          '</defs>')
    P.add(f'<rect width="100%" height="100%" fill="{pal["space"]}"/>')
    # Title band
    P.add(f'<text x="{MARGIN}" y="30" fill="#e8eef5" font-size="20" font-weight="700">{esc(title)}</text>')
    P.add(f'<text x="{MARGIN}" y="50" fill="#8a98a8" font-size="12">Floor {deck["floor_y_m"]:+.1f} m, clear height '
          f'{deck["clear_height_m"]:.1f} m. Bow to the right, port at the top. Presents openspec/changes/reference-ship-tern; '
          f'generated by tools/deck_plans.py from data/ships/{sid}/layout.json.</text>')

    # Grid with metre labels
    for z in range(int(math.ceil(P.z0 / GRID_M) * GRID_M), P.z1 + 1, GRID_M):
        X = P.X(z)
        major = z % 10 == 0
        P.add(f'<line x1="{X:.1f}" y1="{HEAD + MARGIN}" x2="{X:.1f}" y2="{HEAD + MARGIN + P.ph:.1f}" stroke="#16202a" stroke-width="{1.2 if major else 0.6}"/>')
        if major:
            P.add(f'<text x="{X:.1f}" y="{HEAD + MARGIN - 8}" fill="#5f6e7d" font-size="10" text-anchor="middle">z {z}</text>')
    for x in range(int(math.ceil(P.x0 / GRID_M) * GRID_M), P.x1 + 1, GRID_M):
        Y = P.Y(x)
        major = x % 10 == 0
        P.add(f'<line x1="{MARGIN}" y1="{Y:.1f}" x2="{MARGIN + P.pw:.1f}" y2="{Y:.1f}" stroke="#16202a" stroke-width="{1.2 if major else 0.6}"/>')
        if major:
            P.add(f'<text x="{MARGIN - 6}" y="{Y + 3:.1f}" fill="#5f6e7d" font-size="10" text-anchor="end">x {x}</text>')

    # Hull outline at the deck's mid height
    ymid = deck["floor_y_m"] + deck["clear_height_m"] / 2
    top, bot = [], []
    z = min(s["z_m"] for s in L["hull"]["sections"])
    zend = max(s["z_m"] for s in L["hull"]["sections"])
    while z <= zend + 1e-6:
        sec = hull_at(L["hull"]["sections"], z)
        w = hull_half_width(sec, ymid) if sec else None
        if w is not None and w > 0:
            top.append((P.X(z), P.Y(w)))
            bot.append((P.X(z), P.Y(-w)))
        z += 0.5
    if top:
        pts = top + bot[::-1]
        P.add('<polygon points="' + " ".join(f"{a:.1f},{b:.1f}" for a, b in pts) +
              f'" fill="{HULL_FILL}" stroke="{pal["hullLight"]}" stroke-width="1.6" stroke-dasharray="6 3"/>')

    # Compartments: each brush's footprint filled by kind (hatched where it is open to the deck
    # below), then the floors that fixtures provide at this deck's level.
    fixtures = list(L.get("fixtures", []))
    for c in comps:
        col = pal["kind"].get(c["kind"], "#555555")
        solid = mix(col, HULL_FILL, ROOM_OPACITY)
        for b in P.brushes_here(c):
            pts = P.pts(poly_of(b))
            if P.off_deck(b):
                # A turret pod above or below this deck, reached by a hatch: drawn dashed.
                P.add(f'<polygon points="{pts}" fill="{col}" fill-opacity="0.45" '
                      f'stroke="{col}" stroke-width="1.6" stroke-dasharray="4 3"/>')
                continue
            # A thin stroke of the fill's own colour closes the seam where two brushes meet.
            fill = "url(#below)" if P.below(b) else solid
            seam = "#0b1118" if P.below(b) else solid
            P.add(f'<polygon points="{pts}" fill="{fill}" stroke="{seam}" stroke-width="0.8"/>')
        # Floors provided by fixtures at this deck's level (landing, mezzanine, catwalk)
        for f in fixtures:
            if f["compartment"] != c["id"] or f["kind"] not in ("landing", "mezzanine", "catwalk"):
                continue
            if abs(f["center_m"][1] - P.floor) > 0.6:
                continue
            fx, fy, fz = f["center_m"]
            style = f'fill="{col}" fill-opacity="0.95" stroke="{pal["trim"]}" stroke-width="1" stroke-dasharray="4 2"'
            if f.get("poly"):
                P.add(f'<polygon points="{P.pts([tuple(p) for p in f["poly"]])}" {style}/>')
            else:
                x, y, w, h = P.rect(fx, fz, f["size_m"][0], f["size_m"][1])
                P.add(f'<rect x="{x:.1f}" y="{y:.1f}" width="{w:.1f}" height="{h:.1f}" {style}/>')
            if f.get("ring_inner_radius_m"):
                r = f["ring_inner_radius_m"] * S
                P.add(f'<circle cx="{P.X(fz):.1f}" cy="{P.Y(fx):.1f}" r="{r:.1f}" fill="url(#below)" stroke="{pal["trim"]}" stroke-width="1"/>')

    # Walls: every brush edge that does not meet another brush of its own compartment. Where two
    # brushes meet on one level the face is open (nothing drawn); where one floor drops to the
    # other it is a railing, drawn once, from the higher floor.
    for c in comps:
        here = [b for b in P.brushes_here(c) if not P.off_deck(b)]
        for b in here:
            for kind, a, e in face_runs(b, [o for o in here if o is not b]):
                line = f'x1="{P.X(a[1]):.1f}" y1="{P.Y(a[0]):.1f}" x2="{P.X(e[1]):.1f}" y2="{P.Y(e[0]):.1f}"'
                if kind == "wall":
                    P.add(f'<line {line} stroke="{pal["wall"]}" stroke-width="1.4" stroke-linecap="square"/>')
                elif kind == "drop" and not P.below(b):
                    P.add(f'<line {line} stroke="{pal["trim"]}" stroke-width="1" stroke-dasharray="3 2"/>')

    # Dais (a raised floor) and viewscreen
    for c in comps:
        for f in fixtures:
            if f["compartment"] == c["id"] and f["kind"] == "dais":
                fx, fy, fz = f["center_m"]
                x, y, w, h = P.rect(fx, fz, f["size_m"][0], f["size_m"][1])
                P.add(f'<rect x="{x:.1f}" y="{y:.1f}" width="{w:.1f}" height="{h:.1f}" '
                      f'rx="6" fill="none" stroke="{pal["trim"]}" stroke-width="1"/>')
            if f["compartment"] == c["id"] and f["kind"] == "viewscreen":
                fx, fy, fz = f["center_m"]
                w = f["size_m"][0]
                yaw = math.radians(f.get("facing_yaw_deg", 180))
                ux, uz = math.cos(yaw) * w / 2, -math.sin(yaw) * w / 2  # across the screen's facing
                P.add(f'<line x1="{P.X(fz - uz):.1f}" y1="{P.Y(fx - ux):.1f}" x2="{P.X(fz + uz):.1f}" y2="{P.Y(fx + ux):.1f}" '
                      f'stroke="{pal["screen"]}" stroke-width="4"/>')

    # Reactor (a system with a radius) drawn as a circle where it passes through this deck
    for s in L.get("systems", []):
        if s.get("radius_m") and any(s["compartment"] == c["id"] for c in comps):
            x, y, z = s["center_m"]
            P.add(f'<circle cx="{P.X(z):.1f}" cy="{P.Y(x):.1f}" r="{s["radius_m"] * S:.1f}" fill="#1d2a36" stroke="{pal["screenWarm"]}" stroke-width="2"/>')

    # Portals: a wall portal along its wall (perpendicular to its normal, so an angled wall's
    # window lies on the angle), a floor portal as its opening.
    ids_here = [c["id"] for c in comps]
    for p in L["portals"]:
        if not P.portal_here(p):
            continue
        if not any(s in ids_here for s in p["between"]):
            continue
        col = pal["portal"][p["kind"]]
        x, y, z = p["center_m"]
        a, b = p["size_m"]
        if not is_floor_portal(p):
            n = p["normal"]
            tx, tz = -n[2], n[0]  # along the wall, in plan
            P.add(f'<line x1="{P.X(z - tz * a / 2):.1f}" y1="{P.Y(x - tx * a / 2):.1f}" '
                  f'x2="{P.X(z + tz * a / 2):.1f}" y2="{P.Y(x + tx * a / 2):.1f}" stroke="{col}" stroke-width="5"/>')
        else:
            X0, Y0, w, h = P.rect(x, z, a, b)
            dash = ' stroke-dasharray="6 3"' if p["kind"] in ("bay_door", "hoist") else ""
            P.add(f'<rect x="{X0:.1f}" y="{Y0:.1f}" width="{w:.1f}" height="{h:.1f}" fill="none" stroke="{col}" stroke-width="2"{dash}/>')
            if p["kind"] in ("ladder", "hatch"):
                n = max(2, int(b / 0.3))
                for i in range(1, n):
                    Xi = X0 + i * w / n
                    P.add(f'<line x1="{Xi:.1f}" y1="{Y0:.1f}" x2="{Xi:.1f}" y2="{Y0 + h:.1f}" stroke="{col}" stroke-width="1"/>')

    # Craft
    for cr in L.get("craft", []):
        if cr["bay"] not in ids_here:
            continue
        x, y, z = cr["center_m"]
        if not (P.band[0] - 0.6 <= y - cr["height_m"] / 2 <= P.band[1]):
            continue
        hl, hs = cr["length_m"] / 2, cr["span_m"] / 2
        nose = z + hl
        pts = [(z - hl, x + hs), (nose - hl * 0.6, x + hs), (nose, x), (nose - hl * 0.6, x - hs), (z - hl, x - hs)]
        P.add('<polygon points="' + " ".join(f"{P.X(a):.1f},{P.Y(b):.1f}" for a, b in pts) +
              f'" fill="none" stroke="{pal["hullAccent"]}" stroke-width="1.6"/>')
        P.add(f'<text x="{P.X(z - hl + 0.3):.1f}" y="{P.Y(x + hs - 0.3) + 11:.1f}" fill="{pal["hullAccent"]}" font-size="11" font-weight="700">{esc(cr["name"])}</text>')

    # Mounts (turrets, tubes, engines) on the deck nearest their height
    for m in L.get("mounts", []):
        x, y, z = m["center_m"]
        nearest = min(L["decks"], key=lambda d: abs(y - (d["floor_y_m"] + d["clear_height_m"] / 2)))
        if nearest["id"] != deck["id"]:
            continue
        X, Y = P.X(z), P.Y(x)
        if m["kind"] == "turret":
            P.add(f'<circle cx="{X:.1f}" cy="{Y:.1f}" r="9" fill="none" stroke="{pal["role"]["gunner"]}" stroke-width="2"/>')
        elif m["kind"] == "missile_tube":
            P.add(f'<line x1="{P.X(z - 6):.1f}" y1="{Y:.1f}" x2="{X:.1f}" y2="{Y:.1f}" stroke="{pal["role"]["tactical"]}" stroke-width="3"/>')
        elif m["kind"] == "engine":
            P.add(f'<rect x="{X - 6:.1f}" y="{Y - 9:.1f}" width="6" height="18" fill="{pal["emergency"]}"/>')

    # Systems: lettered markers
    sys_here = [s for s in L.get("systems", []) if s["compartment"] in ids_here
                and (s.get("radius_m") or P.band[0] - 0.6 <= s["center_m"][1] < P.band[1])]
    seen = set()
    sys_here = [s for s in sys_here if not (s["id"] in seen or seen.add(s["id"]))]
    letters = {}
    for i, s in enumerate(sys_here):
        letters[s["id"]] = chr(ord("a") + i)
        x, y, z = s["center_m"]
        X, Y = P.X(z), P.Y(x)
        P.add(f'<rect x="{X - 7:.1f}" y="{Y - 7:.1f}" width="14" height="14" rx="2" fill="{HULL_FILL}" stroke="{pal["screenWarm"]}" stroke-width="1.4"/>')
        P.add(f'<text x="{X:.1f}" y="{Y + 4:.1f}" fill="{pal["screenWarm"]}" font-size="11" font-weight="700" text-anchor="middle">{letters[s["id"]]}</text>')

    # Stations: seats with a facing tick and a role code behind the seat
    kinds = {c["id"]: c["kind"] for c in comps}
    st_here = [s for s in L.get("stations", []) if s["compartment"] in ids_here
               and (kinds[s["compartment"]] == "pod" or P.band[0] - 0.6 <= s["seat_m"][1] <= P.band[0] + 1.0)]
    # Wall portals drawn on this deck, in pixels: a role code never sits on one (a pod's hatch).
    portal_px = [(P.X(p["center_m"][2]), P.Y(p["center_m"][0])) for p in L["portals"]
                 if not is_floor_portal(p) and P.portal_here(p) and any(sd in ids_here for sd in p["between"])]
    for s in st_here:
        x, y, z = s["seat_m"]
        X, Y = P.X(z), P.Y(x)
        col = pal["role"].get(s["role"], "#ffffff")
        yaw = math.radians(s["yaw_deg"])
        # facing: yaw 0 faces +z (right), +90 faces +x (up)
        dx, dy = math.cos(yaw) * 14, -math.sin(yaw) * 14
        P.add(f'<line x1="{X:.1f}" y1="{Y:.1f}" x2="{X + dx:.1f}" y2="{Y + dy:.1f}" stroke="{col}" stroke-width="2"/>')
        P.add(f'<circle cx="{X:.1f}" cy="{Y:.1f}" r="6" fill="{col}" stroke="#05070d" stroke-width="1.5"/>')
        # The code sits behind the seat; beside it (aft, then forward) when that would cover a portal.
        lx, ly = X - dx * 0.6, Y - dy * 0.6 + (14 if abs(dy) < 1 else 4)
        if abs(dy) >= 1:
            ly = Y - dy * 1.1 + 4
        spots = [(lx, ly, "middle"), (X - 10, Y + 4, "end"), (X + 10, Y + 4, "start")]
        lx, ly, anchor = next((sp for sp in spots if all(math.hypot(sp[0] - px, sp[1] - 4 - py) > 12 for px, py in portal_px)), spots[0])
        # A dark halo keeps the code legible where it crosses a wall, a pod or the hull line.
        P.add(f'<text x="{lx:.1f}" y="{ly:.1f}" fill="{col}" font-size="9.5" font-weight="700" text-anchor="{anchor}" '
              f'stroke="#05070d" stroke-width="2.5" stroke-linejoin="round" paint-order="stroke">{ROLE_CODE.get(s["role"], "?")}</text>')

    # POI badges, each placed on a free spot: away from seats, systems, craft, ladders and
    # other badges, inside its compartment when one fits there, beside it when not (pods).
    occupied = [(st["seat_m"][0], st["seat_m"][2], 1.3) for st in st_here]
    for st in st_here:
        yw = math.radians(st["yaw_deg"])
        occupied.append((st["seat_m"][0] - math.sin(yw) * 1.1, st["seat_m"][2] - math.cos(yw) * 1.1, 1.3))
        occupied.append((st["seat_m"][0] - 1.0, st["seat_m"][2], 1.1))
    occupied += [(sy["center_m"][0], sy["center_m"][2], 1.0) for sy in sys_here]
    for cr in L.get("craft", []):
        if cr["bay"] in ids_here and P.band[0] - 0.6 <= cr["center_m"][1] - cr["height_m"] / 2 <= P.band[1]:
            occupied.append((cr["center_m"][0], cr["center_m"][2] - cr["length_m"] * 0.2, 2.0))
    for p in L["portals"]:
        if is_floor_portal(p) and P.portal_here(p) and any(sd in ids_here for sd in p["between"]):
            # Ladders and hatches are solid symbols; a bay door is only a dashed outline under
            # its craft, so a badge may sit over it.
            if p["kind"] != "bay_door":
                occupied.append((p["center_m"][0], p["center_m"][2], 0.9 + min(max(p["size_m"]) / 2, 1.5)))

    for p in L["portals"]:
        if not is_floor_portal(p) and P.portal_here(p) and any(sd in ids_here for sd in p["between"]):
            occupied.append((p["center_m"][0], p["center_m"][2], 1.0))

    def free(x, z, need=1.7):
        return all(math.hypot(x - ox, z - oz) >= need + orad - 1.3 for ox, oz, orad in occupied)

    def inside(poly, x, z, pad=0.8):
        """At least pad metres inside the polygon, so the badge stays inside its walls."""
        return inside_poly(poly, x, z, tol=-pad)

    for c in sorted(comps, key=lambda c: (c["kind"] == "pod", c["poi"])):
        bs = P.brushes_here(c)
        b = max(bs, key=lambda b: signed_area(poly_of(b)))
        poly = poly_of(b)
        cx, cz = centroid(poly)
        xs, zs = [p[0] for p in poly], [p[1] for p in poly]
        if c["id"] == "engineering":
            cx, cz = 0.0, min(zs) + 2.0
        if c["kind"] == "corridor" and max(zs) - min(zs) > max(xs) - min(xs):
            cz = min(zs) + 0.3 * (max(zs) - min(zs))
        cands = []
        for r in (0.0, 1.5, 2.5, 3.5, 4.5):
            for k in range(8 if r else 1):
                a = k * math.pi / 4
                cands.append((cx + r * math.sin(a), cz + r * math.cos(a)))
        pick = next(((x, z) for x, z in cands if inside(poly, x, z) and free(x, z)), None)
        leader = None
        if pick is None and c["kind"] == "pod" and P.off_deck(b):
            ring = []
            for r in (2.6, 3.4, 4.2):
                for k in (2, 6, 1, 3, 5, 7, 0, 4):
                    a = k * math.pi / 4
                    ring.append((cx + r * math.sin(a), cz + r * math.cos(a)))
            pick = next(((x, z) for x, z in ring if free(x, z)), None)
            leader = (cx, cz)
        if pick is None:
            # beside the compartment: outboard for side pods, aft for the others
            out = []
            for d in (2.2, 3.2, 4.2):
                out += [(cx + math.copysign(d + (max(xs) - min(xs)) / 2, cx or 1.0), cz),
                        (cx, min(zs) - d + 0.6), (cx, max(zs) + d - 0.6)]
            pick = next(((x, z) for x, z in out if free(x, z)), (cx, cz))
            leader = (cx, cz)
        occupied.append((pick[0], pick[1], 1.3))
        X, Y = P.X(pick[1]), P.Y(pick[0])
        if leader and math.hypot(pick[0] - leader[0], pick[1] - leader[1]) > 0.5:
            # The badge sits beside its compartment: a thin line points at it.
            lx0, ly0 = P.X(leader[1]), P.Y(leader[0])
            d = math.hypot(X - lx0, Y - ly0)
            ex, ey = X + (lx0 - X) * 11 / d, Y + (ly0 - Y) * 11 / d
            P.add(f'<line x1="{ex:.1f}" y1="{ey:.1f}" x2="{lx0:.1f}" y2="{ly0:.1f}" stroke="#e8eef5" stroke-width="1" stroke-opacity="0.8"/>'
                  f'<circle cx="{lx0:.1f}" cy="{ly0:.1f}" r="2" fill="#e8eef5"/>')
        P.add(f'<circle cx="{X:.1f}" cy="{Y:.1f}" r="11" fill="#05070d" stroke="#e8eef5" stroke-width="1.6"/>')
        P.add(f'<text x="{X:.1f}" y="{Y + 4:.1f}" fill="#e8eef5" font-size="11.5" font-weight="700" text-anchor="middle">{c["poi"]}</text>')

    # Scale bar and orientation, bottom left of the plan
    sx, sy = MARGIN + 6, HEAD + MARGIN + P.ph + 22
    P.add(f'<rect x="{sx}" y="{sy}" width="{10 * S:.0f}" height="6" fill="#e8eef5"/><rect x="{sx + 5 * S:.0f}" y="{sy}" width="{5 * S:.0f}" height="6" fill="#5f6e7d"/>')
    P.add(f'<text x="{sx}" y="{sy + 20}" fill="#8a98a8" font-size="11">0</text><text x="{sx + 10 * S:.0f}" y="{sy + 20}" fill="#8a98a8" font-size="11" text-anchor="middle">10 m</text>')
    ox = sx + 10 * S + 50
    P.add(f'<polygon points="{ox + 34},{sy + 3} {ox + 22},{sy - 3} {ox + 22},{sy + 9}" fill="#e8eef5"/><line x1="{ox}" y1="{sy + 3}" x2="{ox + 24}" y2="{sy + 3}" stroke="#e8eef5" stroke-width="2"/>')
    P.add(f'<text x="{ox + 40}" y="{sy + 7}" fill="#e8eef5" font-size="11" font-weight="700">BOW</text>')
    P.add(f'<polygon points="{ox + 90},{sy - 9} {ox + 84},{sy + 3} {ox + 96},{sy + 3}" fill="#e8eef5"/><line x1="{ox + 90}" y1="{sy + 1}" x2="{ox + 90}" y2="{sy + 15}" stroke="#e8eef5" stroke-width="2"/>')
    P.add(f'<text x="{ox + 100}" y="{sy + 7}" fill="#e8eef5" font-size="11" font-weight="700">PORT</text>')

    # Symbol key band
    ky = sy + 40
    kx = MARGIN + 6
    P.add(f'<text x="{kx}" y="{ky}" fill="#8a98a8" font-size="11" font-weight="700">COMPARTMENTS</text>')
    for i, (k, col) in enumerate(pal["kind"].items()):
        X = kx + i * 105
        P.add(f'<rect x="{X}" y="{ky + 8}" width="16" height="12" fill="{col}" stroke="{pal["wall"]}"/><text x="{X + 22}" y="{ky + 18}" fill="#c8d2dc" font-size="11">{k}</text>')
    X = kx + len(pal["kind"]) * 105
    P.add(f'<rect x="{X}" y="{ky + 8}" width="16" height="12" fill="url(#below)" stroke="{pal["wall"]}"/><text x="{X + 22}" y="{ky + 18}" fill="#c8d2dc" font-size="11">open to the deck below</text>')
    X += 170
    pc = pal["kind"].get("pod", "#aa5555")
    P.add(f'<rect x="{X}" y="{ky + 8}" width="16" height="12" fill="{pc}" fill-opacity="0.45" stroke="{pc}" stroke-dasharray="4 3"/><text x="{X + 22}" y="{ky + 18}" fill="#c8d2dc" font-size="11">pod above or below, by hatch</text>')
    ky2 = ky + 40
    P.add(f'<text x="{kx}" y="{ky2}" fill="#8a98a8" font-size="11" font-weight="700">PORTALS</text>')
    for i, (k, col) in enumerate(pal["portal"].items()):
        X = kx + i * 120
        P.add(f'<line x1="{X}" y1="{ky2 + 14}" x2="{X + 18}" y2="{ky2 + 14}" stroke="{col}" stroke-width="5"/><text x="{X + 24}" y="{ky2 + 18}" fill="#c8d2dc" font-size="11">{k.replace("_", " ")}</text>')
    X = kx + len(pal["portal"]) * 120
    P.add(f'<polyline points="{X},{ky2 + 18} {X + 9},{ky2 + 8} {X + 18},{ky2 + 18}" fill="none" stroke="{pal["hullAccent"]}" stroke-width="1.6"/><text x="{X + 24}" y="{ky2 + 18}" fill="#c8d2dc" font-size="11">craft</text>')
    X += 80
    P.add(f'<line x1="{X}" y1="{ky2 + 14}" x2="{X + 18}" y2="{ky2 + 14}" stroke="{pal["trim"]}" stroke-width="1" stroke-dasharray="3 2"/><text x="{X + 24}" y="{ky2 + 18}" fill="#c8d2dc" font-size="11">railing</text>')

    # Legend column
    lx = MARGIN * 2 + P.pw
    ly = HEAD + 6
    P.add(f'<rect x="{lx - 14}" y="{HEAD - 8}" width="{LEGEND_W}" height="{P.H - HEAD - 4}" fill="#0a0f16" stroke="#2b3540"/>')
    P.add(f'<text x="{lx}" y="{ly + 10}" fill="#e8eef5" font-size="13" font-weight="700">POINTS OF INTEREST</text>')
    ly += 30
    for c in sorted(comps, key=lambda c: c["poi"]):
        v, a = volume_area(c)
        decks = "+".join(c["decks"])
        P.add(f'<circle cx="{lx + 9}" cy="{ly - 4}" r="9" fill="#05070d" stroke="#e8eef5" stroke-width="1.2"/>'
              f'<text x="{lx + 9}" y="{ly}" fill="#e8eef5" font-size="10" font-weight="700" text-anchor="middle">{c["poi"]}</text>')
        P.add(f'<rect x="{lx + 22}" y="{ly - 10}" width="8" height="12" fill="{pal["kind"].get(c["kind"], "#555")}"/>')
        P.add(f'<text x="{lx + 36}" y="{ly}" fill="#dbe4ec" font-size="11.5">{esc(c["name"])}</text>')
        P.add(f'<text x="{lx + LEGEND_W - 30}" y="{ly}" fill="#7d8e9e" font-size="10.5" text-anchor="end">{decks}  {v:,.0f} m3  {a:,.0f} m2</text>')
        ly += 21
    ly += 10
    P.add(f'<text x="{lx}" y="{ly}" fill="{pal["screenWarm"]}" font-size="13" font-weight="700">SYSTEMS</text>')
    ly += 18
    for s in sys_here:
        comp = next(c for c in L["compartments"] if c["id"] == s["compartment"])
        P.add(f'<text x="{lx + 4}" y="{ly}" fill="{pal["screenWarm"]}" font-size="11" font-weight="700">{letters[s["id"]]}</text>'
              f'<text x="{lx + 22}" y="{ly}" fill="#dbe4ec" font-size="11">{esc(s["name"])}</text>'
              f'<text x="{lx + LEGEND_W - 30}" y="{ly}" fill="#7d8e9e" font-size="10.5" text-anchor="end">in {comp["poi"]}</text>')
        ly += 16
    ly += 10
    P.add(f'<text x="{lx}" y="{ly}" fill="#e8eef5" font-size="13" font-weight="700">STATIONS</text>')
    ly += 18
    for s in st_here:
        col = pal["role"].get(s["role"], "#ffffff")
        comp = next(c for c in L["compartments"] if c["id"] == s["compartment"])
        P.add(f'<circle cx="{lx + 6}" cy="{ly - 4}" r="5" fill="{col}"/>'
              f'<text x="{lx + 16}" y="{ly}" fill="{col}" font-size="10.5" font-weight="700">{ROLE_CODE.get(s["role"], "?")}</text>'
              f'<text x="{lx + 50}" y="{ly}" fill="#dbe4ec" font-size="11">{esc(s["name"])}{" (core)" if s.get("core") else ""}</text>'
              f'<text x="{lx + LEGEND_W - 30}" y="{ly}" fill="#7d8e9e" font-size="10.5" text-anchor="end">in {comp["poi"]}</text>')
        ly += 16
    need_h = ly + 20
    P.add("</svg>")
    svg = "\n".join(P.out)
    if need_h > P.H:
        svg = svg.replace(f'height="{P.H:.0f}" viewBox="0 0 {P.W:.0f} {P.H:.0f}"', f'height="{need_h:.0f}" viewBox="0 0 {P.W:.0f} {need_h:.0f}"', 1)
        svg = svg.replace(f'height="{P.H - HEAD - 4}" fill="#0a0f16"', f'height="{need_h - HEAD - 4}" fill="#0a0f16"', 1)
    with open(path, "w", encoding="utf-8") as f:
        f.write(svg + "\n")
    return len(comps), len(sys_here), len(st_here)


def main(argv):
    out = os.path.join(ROOT, "docs", "design", "maps")
    if "--out" in argv:
        i = argv.index("--out")
        out = argv[i + 1]
        del argv[i:i + 2]
    paths = [a for a in argv if not a.startswith("--")] or [os.path.join(ROOT, "data", "ships", "tern", "layout.json")]
    pal = palette()
    os.makedirs(out, exist_ok=True)
    for p in paths:
        with open(p, encoding="utf-8") as f:
            L = json.load(f)
        if L.get("schema") != SCHEMA:
            raise SystemExit(f"deck_plans: {p} is {L.get('schema')!r}, expected {SCHEMA!r}")
        for d in L["decks"]:
            path = os.path.join(out, f'{L["ship"]["id"]}-deck-{d["id"]}.svg')
            n, s, t = draw(L, d, pal, path)
            print(f"  {os.path.relpath(path, ROOT)}: {n} compartments, {s} systems, {t} stations")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
