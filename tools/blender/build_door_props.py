"""Star Crew's door leaves, modelled in Blender the hard-surface CSG way: one prop per leaf size, after
the owner's "doors are still flat geometry" and their Next Generation corridor door reference
(openspec/changes/ship-props design section 4g). Taken in shape only (CLAUDE.md 15): two leaves meet
on a stepped seam, raised panels above and below a dark band with a grille in it, a small round lamp
and a plate by the seam; a pressure door is one heavy ribbed leaf.

It owns the leaves' geometry (assets/models/doors/<name>.glb), their atlases and their manifest
(assets/models/doors/props.json). It lives in tools/blender because meshes are files built by a
committed generator (CLAUDE.md section 9). The machinery every set shares is the hard-surface kit,
tools/blender/hs_kit.py; how to work this way is the blender-hard-surface skill.

The sizes are the ship's: every door and pressure door the layout and its patches hold
(data/ships/<ship>/layout.json, deck_access.json, command_suite.json, armory.json) and each lift's car
door (its fixture's door_m). A size the data holds that this set does not build stops the build.

Run (from anywhere):
  <python with the bpy module> tools/blender/build_door_props.py [--check] [--only a,b] [--blend out.blend]
  --check   build in memory and compare with the committed .glb files and props.json; write nothing
  --only    build only these props (the manifest keeps the others' rows)

Conventions (written into props.json too):
  * Metres. Prop space is the glTF frame: +Y up, +Z out of the door's front, +X from the leaf's jamb
    toward the opening's middle for a left leaf, the other way for a right leaf (a left leaf stands at
    the opening's -x jamb as seen from its front).
  * The origin is on the floor at the jamb's face: x = 0 is where the opening begins. A leaf reaches
    IN_JAMB_M into its jamb, so its end is hidden in the frame shut or open.
  * Roles: bulkhead is the leaf's face, trim its raised panels and bars, machinery its dark band and
    edges; their finishes are this set's own (FINISH_OF).
"""
import json
import math
import os
import sys

import bpy  # noqa: F401  first: with the pip bpy module, mathutils exists only once bpy is imported
from mathutils import Matrix  # noqa: E402

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from hs_kit import ROOT, Prop, PropSet, run  # noqa: E402

OUT = os.path.join(ROOT, "assets", "models", "doors")
GENERATOR = "tools/blender/build_door_props.py"
SHIP = "tern"
PATCHES = ("layout", "deck_access", "command_suite", "armory")

# The leaf (design 4g), metres.
LINTEL_M = 0.005       # clear of the lintel; a leaf stands on the floor (the build reads a prop back with its foot at y = 0)
IN_JAMB_M = 0.02       # how far a leaf reaches into its jamb
THICK_M = 0.06         # a door leaf's slab
PANEL_M = 0.012        # a raised panel stands this proud of the slab
PANEL_IN_M = 0.06      # and this far in from the leaf's edges
STEP_M = 0.08          # the seam stands this far either side of the opening's middle
SEAM_Y = (1.00, 1.16)  # where it steps across, at 45 degrees
GAP_M = 0.002          # each leaf stops this short of the seam line
BAND_Y = (1.30, 1.52)  # the dark band
BAND_M = 0.008         # its depth
LOWER_Y = (0.30, 1.20)  # the raised panels
UPPER_Y0 = 1.62
TOP_GAP_M = 0.12       # the upper panel stops this far under the leaf's top
PRESSURE_THICK_M = 0.20
RIB_Y = ((0.50, 0.58), (1.92, 2.00))  # a pressure leaf's two ribs across each face, from and to (modelled)
RIM_M = (0.06, 0.02)                   # its rim: width, proud
WHEEL_Y, WHEEL_R = 1.10, 0.24          # its locking wheel's centre height and outer radius
SEAM_PLATES_Y = (0.80, 1.52)  # a pressure leaf's plate seams

# The set's colours: a warm light grey leaf, its raised panels a shade lighter, the band and edges dark
# (data/materials/prop_atlas.json finishes).
FINISH_OF = {"bulkhead": "door_leaf", "trim": "door_panel", "machinery": "dark"}
PRESSURE_FINISH_OF = {"bulkhead": "panel", "trim": "steel", "machinery": "dark"}


def name_of(kind, w, h, side):
    base = f"{kind}_{round(w * 100)}x{round(h * 100)}"
    return base if side is None else f"{base}_{side}"


def ship_sizes():
    """Every leaf the ship needs: (kind, opening width, height) from the layout, its patches and the lifts."""
    sizes = set()
    for stem in PATCHES:
        path = os.path.join(ROOT, "data", "ships", SHIP, stem + ".json")

        def walk(o):
            if isinstance(o, dict):
                if o.get("kind") in ("door", "pressure_door") and "size_m" in o:
                    sizes.add((o["kind"], float(o["size_m"][0]), float(o["size_m"][1])))
                if o.get("kind") == "lift" and "door_m" in o:
                    sizes.add(("door", float(o["door_m"][0]), float(o["door_m"][1])))
                for v in o.values():
                    walk(v)
            elif isinstance(o, list):
                for v in o:
                    walk(v)
        walk(json.load(open(path, encoding="utf-8")))
    return sorted(sizes)


def seam(lw, side):
    """The leaf's seam edge, x in its prop space below and above the step."""
    if side == "l":
        return lw + STEP_M - GAP_M, lw - STEP_M - GAP_M
    return -(lw - STEP_M - GAP_M), -(lw + STEP_M - GAP_M)


def leaf_outline(lw, h, side):
    """The leaf's face, counter-clockwise in (x, y) seen from its front."""
    lo, hi = seam(lw, side)
    y0, y1 = 0.0, h - LINTEL_M
    if side == "l":
        return [(-IN_JAMB_M, y0), (lo, y0), (lo, SEAM_Y[0]), (hi, SEAM_Y[1]), (hi, y1), (-IN_JAMB_M, y1)]
    return [(lo, y0), (IN_JAMB_M, y0), (IN_JAMB_M, y1), (hi, y1), (hi, SEAM_Y[1]), (lo, SEAM_Y[0])]


def panel_outlines(lw, h, side):
    """The raised panels' outlines, counter-clockwise, PANEL_IN_M inside the visible leaf (the seam
    edge moved toward the jamb by that much, so the step's diagonal follows it)."""
    lo, hi = seam(lw, side)
    s = 1 if side == "l" else -1
    j, a, b = s * PANEL_IN_M, lo - s * PANEL_IN_M, hi - s * PANEL_IN_M
    ya, yb = LOWER_Y
    lower = [(j, ya), (a, ya), (a, SEAM_Y[0]), (b, SEAM_Y[1]), (b, yb), (j, yb)]
    upper = [(j, UPPER_Y0), (b, UPPER_Y0), (b, h - TOP_GAP_M), (j, h - TOP_GAP_M)]
    if side == "r":
        lower, upper = [p for p in reversed(lower)], [p for p in reversed(upper)]
    return lower, upper


def faces():
    """The frames of a leaf's two faces for the atlas details: local x along the leaf as seen, z out."""
    turn = Matrix.Rotation(3.141592653589793, 4, "Y")
    return [(1, lambda z: Matrix.Translation((0.0, 0.0, z))), (-1, lambda z: Matrix.Translation((0.0, 0.0, -z)) @ turn)]


def door_leaf(kind, w, h, side):
    """A sliding door's leaf: the slab with its stepped seam, raised panels each side, the dark band cut
    across both faces; in the atlas, the band's grille, a louvre, and by the seam the lamp and plate."""
    lw = w / 2
    n = name_of(kind, w, h, side)
    p = Prop(n, f"The {'left' if side == 'l' else 'right'} leaf of a {w:.1f} x {h:.1f} m sliding door (ship-props design 4g)",
             f"floor, at the {'left' if side == 'l' else 'right'} jamb's face; the leaf runs toward {'+' if side == 'l' else '-'}x")
    t = THICK_M / 2
    body = p.prism("slab", leaf_outline(lw, h, side), "z", -t, t, "machinery", cap="bulkhead")
    raised = []
    for zs in (1, -1):
        for k, poly in enumerate(panel_outlines(lw, h, side)):
            lo, hi = (t - 0.005, t + PANEL_M) if zs > 0 else (-t - PANEL_M, -t + 0.005)
            raised.append(p.prism(f"panel_{'f' if zs > 0 else 'b'}{k}", poly, "z", lo, hi, "trim"))
    p.union(body, "raised_panels", raised)
    p.chamfer(body, "panel_edges", 0.006, lambda m, d, n1, n2: abs(m[2]) > t + PANEL_M - 0.002)
    xa, xb = (-0.10, lw + STEP_M + 0.05) if side == "l" else (-(lw + STEP_M + 0.05), 0.10)
    bands = [p.box(f"band_{'f' if zs > 0 else 'b'}", (xa, BAND_Y[0], zs * (t - BAND_M) if zs > 0 else -t - 0.05),
                   (xb, BAND_Y[1], t + 0.05 if zs > 0 else -(t - BAND_M)), "machinery") for zs in (1, -1)]
    p.cut(body, "band", bands)
    p.body = body
    lo, hi = seam(lw, side)
    s = 1 if side == "l" else -1

    def decor(D):
        for zs, at in faces():
            # In a face's frame x runs along the leaf as seen from that face: the front sees the prop's x,
            # the back its mirror. X(x) is a prop-space x in that frame.
            X = (lambda x: x) if zs > 0 else (lambda x: -x)
            band, face, panel = at(t - BAND_M), at(t), at(t + PANEL_M)
            gx = sorted((X(s * 0.06), X(hi - s * (0.12 if side == "l" else 0.06))))
            D.grille(band, gx[0], BAND_Y[0] + 0.04, gx[1], BAND_Y[1] - 0.04, finish="dark", slat="steel", pitch=0.028)
            lx = sorted((X(s * 0.08), X(min(lo, hi, key=abs) - s * 0.10)))
            D.grille(face, lx[0], 0.10, lx[1], 0.22, finish="dark", slat="steel", pitch=0.024)
            if side == "l":
                D.disc(band, X(hi - 0.06), (BAND_Y[0] + BAND_Y[1]) / 2, 0.024, -0.002, 0.006, "led_amber", sides=14, inset=0.003)
                D.ring(band, X(hi - 0.06), (BAND_Y[0] + BAND_Y[1]) / 2, 0.024, 0.032, -0.002, 0.008, "bolt", sides=14)
                D.plate(panel, X(hi - 0.06 - 0.09), UPPER_Y0 + 0.12, 0.15, 0.07, "", finish="plate")
                D.paint(panel, [(X(hi - 0.06 - 0.155), UPPER_Y0 + 0.035), (X(hi - 0.06 - 0.025), UPPER_Y0 + 0.035),
                                (X(hi - 0.06 - 0.025), UPPER_Y0 + 0.055), (X(hi - 0.06 - 0.155), UPPER_Y0 + 0.055)][::zs], "yellow")
            else:
                D.plate(panel, X(hi + 0.06 + 0.07), UPPER_Y0 + 0.12, 0.10, 0.05, "", finish="plate")
            # Nothing of the kit's generic seams and bolts on a leaf: its faces are its own design.
            for z in (t, t + PANEL_M, t - BAND_M):
                D.reserve(at(z), -3.0, -0.5, 3.0, h + 0.5)
    p.decor.append(decor)
    p.finish_of.update(FINISH_OF)
    return p


def pressure_leaf(kind, w, h):
    """A pressure door's one leaf, jamb to jamb, the heaviest door on the ship (design 4i): a 20 cm slab with
    a raised rim round each face, two ribs across it, a locking wheel (hub, spokes, an eight-sided rim), the
    locking bar above it and two hydraulic rams; in the atlas a hazard foot, a window, an amber lamp and the
    PRESSURE plate."""
    n = name_of(kind, w, h, None)
    p = Prop(n, f"The leaf of a {w:.1f} x {h:.1f} m pressure door, sliding whole into its left jamb (ship-props design 4i)",
             "floor, at the left jamb's face; the leaf runs toward +x to the right jamb")
    t = PRESSURE_THICK_M / 2
    top = h - LINTEL_M
    x0, x1 = -IN_JAMB_M, w + IN_JAMB_M
    body = p.box("slab", (x0, 0.0, -t), (x1, top, t), {"+z": "bulkhead", "-z": "bulkhead", "*": "machinery"})
    pieces = []
    for zs in (1, -1):
        f = "f" if zs > 0 else "b"

        def z(lo, hi):
            """A span proud of this face: lo..hi metres out of it (into the slab when negative)."""
            return (t + lo, t + hi) if zs > 0 else (-t - hi, -t - lo)
        r0, r1 = z(-0.005, RIM_M[1])
        # The rim: one frame round the face, a plate with its middle cut out (bars would meet in one plane).
        rim = p.box(f"rim_{f}", (0.0, 0.0, r0), (w, top, r1), "trim")
        c0_, c1_ = z(-0.05, RIM_M[1] + 0.05)
        p.cut(rim, f"rim_{f}_open", [p.box(f"rim_{f}_hole", (RIM_M[0], RIM_M[0], c0_), (w - RIM_M[0], top - RIM_M[0], c1_), "trim")])
        pieces.append(rim)
        for k, (ya, yb) in enumerate(RIB_Y):
            q0, q1 = z(-0.005, RIM_M[1] - 0.006)
            pieces.append(p.box(f"rib_{f}{k}", (RIM_M[0] - 0.01, ya, q0), (w - RIM_M[0] + 0.01, yb, q1), "trim"))
        # The locking bar, and the wheel below it: hub, four spokes and an eight-sided rim.
        b0, b1 = z(-0.005, 0.035)
        pieces.append(p.box(f"bar_{f}", (0.20, 1.36, b0), (w - 0.20, 1.46, b1), "trim"))
        cx, cy = w / 2, WHEEL_Y
        h0, h1 = z(-0.005, 0.07)
        pieces.append(p.prism(f"hub_{f}", [(cx + 0.075 * math.cos(math.pi / 8 + k * math.pi / 4), cy + 0.075 * math.sin(math.pi / 8 + k * math.pi / 4))
                                          for k in range(8)], "z", h0, h1, "machinery"))
        s0, s1 = z(0.02, 0.055)
        for k in range(4):
            m = Matrix.Translation((cx, cy, 0.0)) @ Matrix.Rotation(math.pi / 4 + k * math.pi / 2, 4, "Z")
            pieces.append(p.box(f"spoke_{f}{k}", (0.05, -0.018, s0), (WHEEL_R - 0.02, 0.018, s1), "trim", m=m))
        w0, w1 = z(0.015, 0.06)
        octo = lambda r: [(cx + r * math.cos(math.pi / 8 + k * math.pi / 4), cy + r * math.sin(math.pi / 8 + k * math.pi / 4)) for k in range(8)]
        ring = p.prism(f"wheel_{f}", octo(WHEEL_R / math.cos(math.pi / 8)), "z", w0, w1, "machinery")
        i0, i1 = z(-0.05, 0.11)
        p.cut(ring, f"wheel_{f}_open", [p.prism(f"wheel_{f}_hole", octo((WHEEL_R - 0.035) / math.cos(math.pi / 8)), "z", i0, i1, "machinery")])
        pieces.append(ring)
        # Two hydraulic rams at the wheel's lower corners: a cylinder body and its rod, on a slant.
        for sgn in (-1, 1):
            ang = math.radians(35.0) * sgn
            m = Matrix.Translation((cx + sgn * 0.33, 0.70, 0.0)) @ Matrix.Rotation(ang, 4, "Z")
            c0, c1 = z(-0.005, 0.05)
            pieces.append(p.box(f"ram_{f}{sgn:+d}", (-0.025, -0.10, c0), (0.025, 0.04, c1), "machinery", m=m))
            d0, d1 = z(-0.005, 0.035)
            pieces.append(p.box(f"rod_{f}{sgn:+d}", (-0.012, 0.03, d0), (0.012, 0.12, d1), "trim", m=m))
    p.union(body, "rim_ribs_wheel_rams", pieces)
    p.chamfer(body, "proud_edges", 0.006, lambda m, d, n1, n2: abs(m[2]) > t + 0.012 and abs(d[0]) > 0.99)
    # Two plate seams across each face (V-grooves 12 mm wide and deep), between the ribs, so no chart of a face
    # is long (design 4g).
    p.cut(body, "plate_seams", [p.prism(f"seam_{'f' if zs > 0 else 'b'}{k}", [(zs * (t + 0.05), y - 0.006), (zs * (t - 0.012), y), (zs * (t + 0.05), y + 0.006)][::zs],
                                        "x", RIM_M[0] + 0.02, w - RIM_M[0] - 0.02, "machinery") for zs in (1, -1) for k, y in enumerate(SEAM_PLATES_Y)])
    p.body = body

    def decor(D):
        for zs, at in faces():
            X = (lambda x: x) if zs > 0 else (lambda x: -x)
            face = at(t)
            xs = sorted((X(RIM_M[0] + 0.01), X(w - RIM_M[0] - 0.01)))
            D.paint(face, [(xs[0], RIM_M[0] + 0.02), (xs[1], RIM_M[0] + 0.02), (xs[1], RIB_Y[0][0] - 0.03), (xs[0], RIB_Y[0][0] - 0.03)], "hazard")
            wx = sorted((X(w / 2 - 0.15), X(w / 2 + 0.15)))
            D.box(face, wx[0] - 0.02, 1.56, wx[1] + 0.02, 1.84, -0.002, 0.008, "steel", inset=0.004)
            D.box(face, wx[0], 1.58, wx[1], 1.82, -0.002, 0.009, "glass")
            D.disc(face, X(w / 2 + 0.28), 1.70, 0.026, -0.002, 0.008, "led_amber", sides=14, inset=0.003)
            D.ring(face, X(w / 2 + 0.28), 1.70, 0.026, 0.034, -0.002, 0.01, "bolt", sides=14)
            D.plate(face, X(w / 2), (RIB_Y[1][1] + top - RIM_M[0]) / 2, 0.30, 0.06, "PRESSURE", finish="yellow", ink="stencil_dark")
    p.decor.append(decor)
    p.finish_of.update(PRESSURE_FINISH_OF)
    return p


# Every leaf the ship needs, in a stable order: two a sliding door, one a pressure door.
LEAVES = {}
for _kind, _w, _h in ship_sizes():
    if _kind == "pressure_door":
        LEAVES[name_of("pressure", _w, _h, None)] = (lambda k=_kind, w=_w, h=_h: pressure_leaf("pressure", w, h))
    else:
        for _side in ("l", "r"):
            LEAVES[name_of("door", _w, _h, _side)] = (lambda w=_w, h=_h, s=_side: door_leaf("door", w, h, s))

# Triangles per leaf (design 4g's table, 4i): a door leaf 220-240 by its size, a pressure leaf 1,700.
BUDGETS = {n: (1700 if n.startswith("pressure") else 220 if float(n.split("_")[1].split("x")[0]) <= 120 else 240) for n in LEAVES}

STATUS = ("Built (2026-10-08) by " + GENERATOR + " for the owner's 'doors are still flat geometry' "
          "(openspec/changes/ship-props design section 4g). docs/mockups/deck-plan.html draws them; the engine does not yet.")
RULES = [
    "This file is written by " + GENERATOR + "; never edit it by hand. Rebuild, and the .glb files and this file change together.",
    "Metres. Prop space is the glTF frame: +Y up, +Z out of the door's front, +X from a left leaf's jamb toward the opening's middle (a right leaf runs toward -X from its jamb).",
    "The origin is on the floor at the jamb's face, where the opening begins. A leaf reaches 0.02 m into its jamb, so its end is hidden in the frame; a page cuts it there with a clipping plane as it slides.",
    "A name is <kind>_<width cm>x<height cm>_<l|r>: the opening's clear size, and which leaf (l stands at the opening's -x jamb in prop space, r at its +x jamb). A pressure door has one leaf, no side.",
    "Materials are roles: bulkhead (the leaf's faces), trim (raised panels, ribs, bars) and machinery (the band and the edges); the atlas carries their colours.",
    "UV0 is in metres as shipkit.js worldUv; TEXCOORD_1 is the leaf's own atlas (assets/models/doors/atlas/<prop>.png), alpha its glow mask (the lamp).",
    "Flat shaded, triangulated, one closed manifold solid per leaf. triangles is counted in the .glb; the build refuses a leaf over budget_triangles.",
]

DOORS = PropSet("doors", "DoorProps", OUT, GENERATOR, LEAVES, BUDGETS, STATUS, RULES, __doc__)


if __name__ == "__main__":
    run(DOORS)
