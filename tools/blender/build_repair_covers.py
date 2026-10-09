"""Star Crew's repair covers and what they hide: the service cover a player unscrews in 3D before a
repair, and the machine's insides that the opening shows once it is off (openspec/changes/repairs-on-deck,
design 3b), modelled the hard-surface way and baked to textures.

It owns assets/textures/repairs/cover_<finish>.png, interior_<kind>.png and covers.json (each file's
size and sha256, and the cover's screw holes as fractions of the plate, which the 3D screws are placed
from). It lives beside build_wall_panels.py and imports it rather than copying it (CLAUDE.md 6.1): it
takes the panels' scene, light, materials, wear, render and post-process. A cover is a wall part, so it
takes the finish's colours and wear from data/materials/panels.json; the interiors take their own
colours, wear and role maps (a circuit board has no wall paint and no rust). What is particular to both
is data/materials/repair_covers.json.

Run, with Pillow and numpy beside bpy (pip install bpy==4.5.4 pillow numpy):
  <python with the bpy module> tools/blender/build_repair_covers.py [--samples N] [--only interior_circuit,...]
  --only renders only these targets (cover_<finish>, interior_<kind>); the rest keep their files and entries

Panel space: x right, y up, z out of the face, metres, the plate centred on (0, 1).

The cover:
  a dark backing the cover's size, so the chamfered corners read as cut;
  the plate, 20 mm thick with 45 degree corners, in the finish's second paint;
  a raised rim round it and a hazard band inside the rim;
  a countersunk hole at each corner, inset_m from both edges: a dark bore and a bright chamfer ring;
  the stencils, SERVICE across the middle and a small note under it.

The interiors, at the cover's size (the opening's back):
  circuit: a green board on four standoffs; a processor and smaller chips with their pins, copper traces
    and vias between them; capacitors, a finned heatsink, a header whose wires leave over the top edge,
    two status LEDs and silkscreen marks;
  wiring: a terminal box; a zinc mounting plate, a slotted wire duct, a DIN rail of terminal blocks
    with their screws, wires from each block gathered into a tied loom that leaves at the bottom, a relay
    with its lamp, and a stencilled label.
"""

import argparse
import copy
import json
import math
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import build_wall_panels as wp  # noqa: E402  imports bpy first, then the kit and the post-process
import PIL  # noqa: E402

ROOT = wp.ROOT
DATA = os.path.join(ROOT, "data", "materials", "repair_covers.json")
OUT = os.path.join(ROOT, "assets", "textures", "repairs")
RAW = os.path.join(ROOT, "tools", "materials", "raw", "repairs")
SCHEMA = "starcrew.repair-covers/1"
WIRES = ("rubber", "walk", "upholstery", "safety")   # the roles the kinds map to wire colours, in turn


def load():
    """Read and validate repair_covers.json: every key checked, unknown keys stop the build."""
    with open(DATA, encoding="utf-8") as f:
        d = json.load(f)
    if d.get("schema") != SCHEMA:
        raise SystemExit(f"[covers] {DATA}: schema must be {SCHEMA!r}")
    wp.keys_exact(d, {"schema", "status", "_rules", "size_m", "out_px", "supersample", "colours", "finishes",
                      "corner_m", "rim_w_m", "hazard_w_m", "screw", "text", "interiors"}, "repair_covers")
    wp.keys_exact(d["screw"], {"inset_m", "hole_r_m", "countersink_r_m"}, "repair_covers.screw")
    wp.keys_exact(d["text"], {"label", "label_m", "note", "note_m"}, "repair_covers.text")
    (w, h), (pw, ph) = d["size_m"], d["out_px"]
    if abs(pw / w - ph / h) > 0.5:
        raise SystemExit(f"[covers] out_px {pw}x{ph} is not at one density over size_m {w}x{h}")
    if not 0 < d["screw"]["countersink_r_m"] < d["screw"]["inset_m"]:
        raise SystemExit("[covers] screw.countersink_r_m must be above 0 and inside inset_m")
    inner = d["interiors"]
    wp.keys_exact(inner, {"colours_srgb", "wear", "kinds"}, "repair_covers.interiors")
    wp.wear_ok(inner["wear"], "repair_covers.interiors.wear")
    for name, rgb in inner["colours_srgb"].items():
        if len(rgb) != 3 or not all(0.0 <= v <= 1.0 for v in rgb):
            raise SystemExit(f"[covers] interiors.colours_srgb.{name} must be three numbers 0-1")
    if set(inner["kinds"]) != set(INTERIOR_BUILDERS):
        raise SystemExit(f"[covers] interiors.kinds must be exactly {sorted(INTERIOR_BUILDERS)}")
    for kind, K in inner["kinds"].items():
        wp.keys_exact(K, {"roles"}, f"repair_covers.interiors.kinds.{kind}")
        for role, col in K["roles"].items():
            if role not in wp.kit.ROLES + wp.EXTRA_ROLES:
                raise SystemExit(f"[covers] interiors.kinds.{kind}.roles: no material role {role!r}")
            if col not in inner["colours_srgb"]:
                raise SystemExit(f"[covers] interiors.kinds.{kind}.roles.{role}: no colour {col!r}")
    return d


def frame_of(d):
    w, h = d["size_m"]
    return -w / 2, w / 2, 1.0 - h / 2, 1.0 + h / 2


# ----------------------------------------------------------------------------- the cover

def build_cover(P, d):
    """Model one cover (module docstring). Returns the holes as (u from the left, v from the top)."""
    w, h = d["size_m"]
    x0, x1, y0, y1 = frame_of(d)
    c, rim, band, s = d["corner_m"], d["rim_w_m"], d["hazard_w_m"], d["screw"]
    P.box("backing", (x0 - 0.05, y0 - 0.05, -0.06), (x1 + 0.05, y1 + 0.05, -0.02), "machinery")
    holes = [(x, y) for y in (y1 - s["inset_m"], y0 + s["inset_m"]) for x in (x0 + s["inset_m"], x1 - s["inset_m"])]
    plate = wp.cplate(P, "plate", x0, y0, x1, y1, c, -0.02, 0.0, "paint2", bevel=0.004)
    P.cut(plate, "holes", [P.cyl("bore", "z", hp, s["hole_r_m"], -0.015, 0.05, "machinery", sides=24, smooth=False)
                           for hp in holes])
    rim_ob = wp.frame_ring(P, "rim", x0 + c * 0.6, y0 + c * 0.6, x1 - c * 0.6, y1 - c * 0.6, rim, -0.004, 0.004,
                           "paint2", bevel=0.002)
    i = c * 0.6 + rim + 0.006
    band_ob = wp.frame_ring(P, "hazard", x0 + i, y0 + i, x1 - i, y1 - i, band, -0.004, 0.002, "hazard")
    # The band and the rim stop short of the holes: each hole sits in a clear pad.
    pads = [P.cyl("pad", "z", hp, s["countersink_r_m"] + 0.008, -0.02, 0.05, "paint2", sides=24, smooth=False)
            for hp in holes]
    P.cut(band_ob, "band_pads", pads)
    P.cut(rim_ob, "rim_pads", [P.cyl("pad2", "z", hp, s["countersink_r_m"] + 0.008, -0.02, 0.05, "paint2", sides=24,
                                     smooth=False) for hp in holes])
    for hp in holes:
        wp.ring(P, "countersink", hp[0], hp[1], s["countersink_r_m"], s["hole_r_m"], -0.006, 0.0008, "trim",
                sides=28, bevel=0.002)
    t = d["text"]
    P.text(t["label"], 0.0, 1.0 - t["label_m"] * 0.35, t["label_m"], z=0.0, align="CENTER")
    P.text(t["note"], 0.0, 1.0 - t["label_m"] * 0.35 - t["note_m"] * 1.9, t["note_m"], z=0.0, align="CENTER")
    return [((hx - x0) / w, (y1 - hy) / h) for hx, hy in holes]


# ----------------------------------------------------------------------------- the interiors
# Features are sized to read at the bake's 457 px per metre (a texel is 2.2 mm): traces 3 mm, pins 2.5 mm.
# The board is a real one's look: traces run under the green solder mask (a lighter green, 0.4 mm proud),
# bend at 45 degrees and run in parallel buses; only pads, vias, pins and the edge fingers are bare gold;
# every part has a white silkscreen outline and its designator.

BOARD_Z = 0.0035            # the board's top face
MOVES = {"n": (0, 1), "s": (0, -1), "e": (1, 0), "w": (-1, 0), "ne": (1, 1), "nw": (-1, 1), "se": (1, -1), "sw": (-1, -1)}


def seg(P, what, a, b, w, z0, z1, role):
    """A flat bar from a to b (panel x, y), w wide, its ends squared past the points by w / 2."""
    dx, dy = b[0] - a[0], b[1] - a[1]
    n = math.hypot(dx, dy)
    if n < 1e-6:
        return
    m = wp.Matrix.Translation(((a[0] + b[0]) / 2, (a[1] + b[1]) / 2, 0.0)) @ wp.Matrix.Rotation(math.atan2(dy, dx), 4, "Z")
    P.box(what, (-n / 2 - w / 2, -w / 2, z0), (n / 2 + w / 2, w / 2, z1), role, m=m)


def route(start, moves):
    """A trace's points from a start and moves [(direction, axis offset), ...]: a diagonal of 0.01 moves 0.01 in x and y."""
    pts = [start]
    for d, k in moves:
        ux, uy = MOVES[d]
        pts.append((pts[-1][0] + ux * k, pts[-1][1] + uy * k))
    return pts


def trace(P, pts, w=0.0026, via=True):
    """A trace under the mask along pts, with a gold via at its far end."""
    for a, b in zip(pts, pts[1:]):
        seg(P, "trace", a, b, w, BOARD_Z, BOARD_Z + 0.0004, "safety")
    if via:
        ex, ey = pts[-1]
        wp.ring(P, "via", ex, ey, 0.0034, 0.0014, BOARD_Z, BOARD_Z + 0.0006, "trim", sides=12)


def bus(P, starts, moves, w=0.0026, via=True):
    """Parallel traces: every start takes the same moves, so the bus keeps its pitch through its bends."""
    for st in starts:
        trace(P, route(st, moves), w, via)


def silk_box(P, cx, cy, w, h, label=None, lx=None, ly=None):
    """A part's silkscreen outline, 1.2 mm wide, and its designator."""
    wp.frame_ring(P, "silk", cx - w / 2, cy - h / 2, cx + w / 2, cy + h / 2, 0.0012, BOARD_Z, BOARD_Z + 0.0005, "stencil")
    if label:
        P.text(label, cx - w / 2 if lx is None else lx, cy + h / 2 + 0.003 if ly is None else ly, 0.009, z=BOARD_Z)


def qfp(P, cx, cy, size, n, label):
    """A square chip with n pins a side: the body, gull-wing pins on all four sides, a pin-1 dot. Returns each
    side's pin tips (left, right, top, bottom), where its traces start."""
    h = size / 2
    P.box("chip", (cx - h, cy - h, BOARD_Z), (cx + h, cy + h, BOARD_Z + 0.004), "upholstery_panel", bevel=0.0012)
    P.cyl("pin1", "z", (cx - h + 0.007, cy + h - 0.007), 0.0025, BOARD_Z + 0.004, BOARD_Z + 0.0046, "paint2", sides=12)
    pitch = (size - 0.012) / (n - 1)
    out = 0.006
    tips = {"l": [], "r": [], "t": [], "b": []}
    for k in range(n):
        o = -h + 0.006 + k * pitch
        for side, (x0, y0, x1, y1), tip in (
                ("l", (cx - h - out, cy + o - 0.0012, cx - h, cy + o + 0.0012), (cx - h - out, cy + o)),
                ("r", (cx + h, cy + o - 0.0012, cx + h + out, cy + o + 0.0012), (cx + h + out, cy + o)),
                ("t", (cx + o - 0.0012, cy + h, cx + o + 0.0012, cy + h + out), (cx + o, cy + h + out)),
                ("b", (cx + o - 0.0012, cy - h - out, cx + o + 0.0012, cy - h), (cx + o, cy - h - out))):
            P.box("pin", (x0, y0, BOARD_Z), (x1, y1, BOARD_Z + 0.0014), "trim")
            tips[side].append(tip)
    silk_box(P, cx, cy, size + 2 * out + 0.006, size + 2 * out + 0.006, label)
    return tips


def soic(P, cx, cy, w, h, n, label):
    """A long chip with n pins along its top and bottom sides. Returns the top and bottom pin tips."""
    P.box("chip", (cx - w / 2, cy - h / 2, BOARD_Z), (cx + w / 2, cy + h / 2, BOARD_Z + 0.004), "upholstery_panel", bevel=0.001)
    P.cyl("pin1", "z", (cx - w / 2 + 0.005, cy - h / 2 + 0.005), 0.0018, BOARD_Z + 0.004, BOARD_Z + 0.0046, "paint2", sides=10)
    pitch = (w - 0.008) / (n - 1)
    tips = {"t": [], "b": []}
    for k in range(n):
        x = cx - w / 2 + 0.004 + k * pitch
        P.box("pin", (x - 0.0013, cy + h / 2, BOARD_Z), (x + 0.0013, cy + h / 2 + 0.006, BOARD_Z + 0.0014), "trim")
        P.box("pin", (x - 0.0013, cy - h / 2 - 0.006, BOARD_Z), (x + 0.0013, cy - h / 2, BOARD_Z + 0.0014), "trim")
        tips["t"].append((x, cy + h / 2 + 0.006))
        tips["b"].append((x, cy - h / 2 - 0.006))
    silk_box(P, cx, cy, w + 0.006, h + 0.018, label)
    return tips


def chip_part(P, cx, cy, w, h, role):
    """A two-pad part (a resistor or a capacitor): gold end pads and a body between."""
    P.box("pad", (cx - w / 2, cy - h / 2, BOARD_Z), (cx + w / 2, cy + h / 2, BOARD_Z + 0.0008), "trim")
    P.box("body", (cx - w / 2 + w * 0.22, cy - h / 2, BOARD_Z), (cx + w / 2 - w * 0.22, cy + h / 2, BOARD_Z + 0.0022), role, bevel=0.0005)


def can(P, x, y, r, label):
    """An electrolytic capacitor from above: the blue sleeve's rim, the aluminium top with its pressed vent cross,
    the polarity stripe, and its silkscreen ring."""
    P.cyl("can", "z", (x, y), r, BOARD_Z, BOARD_Z + 0.02, "upholstery", sides=24, bevel=0.0015)
    top = P.cyl("can_top", "z", (x, y), r * 0.86, BOARD_Z + 0.02, BOARD_Z + 0.0208, "paint2", sides=24)
    P.cut(top, "vent", [P.box("vent", (x - r * 0.55, y - 0.0007, BOARD_Z + 0.0203), (x + r * 0.55, y + 0.0007, BOARD_Z + 0.03), "machinery"),
                        P.box("vent", (x - 0.0007, y - r * 0.55, BOARD_Z + 0.0203), (x + 0.0007, y + r * 0.55, BOARD_Z + 0.03), "machinery")])
    P.box("stripe", (x - r, y - r * 0.4, BOARD_Z), (x - r * 0.82, y + r * 0.4, BOARD_Z + 0.0202), "stencil")
    wp.ring(P, "silk_can", x, y, r + 0.003, r + 0.0018, BOARD_Z, BOARD_Z + 0.0005, "stencil", sides=28)
    P.text(label, x + r + 0.002, y + r - 0.004, 0.009, z=BOARD_Z)


def interior_circuit(P, d):
    x0, x1, y0, y1 = frame_of(d)
    P.box("back", (x0 - 0.05, y0 - 0.05, -0.04), (x1 + 0.05, y1 + 0.05, -0.012), "machinery")
    m = 0.02
    bx0, bx1, by0, by1 = x0 + m, x1 - m, y0 + m, y1 - m
    holes = [(bx0 + 0.013, by0 + 0.013), (bx1 - 0.013, by0 + 0.013), (bx0 + 0.013, by1 - 0.013), (bx1 - 0.013, by1 - 0.013)]
    for hx, hy in holes:
        P.cyl("standoff", "z", (hx, hy), 0.0075, -0.012, 0.0, "paint2", sides=6, smooth=False)
    board = P.box("board", (bx0, by0, 0.0), (bx1, by1, BOARD_Z), "bulkhead", bevel=0.001)
    P.cut(board, "mount_holes", [P.cyl("hole", "z", hp, 0.0034, -0.01, 0.01, "machinery", sides=16, smooth=False) for hp in holes])
    for hx, hy in holes:
        wp.ring(P, "mount_pad", hx, hy, 0.0072, 0.0034, BOARD_Z, BOARD_Z + 0.0008, "trim", sides=20)
        P.cyl("screw", "z", (hx, hy), 0.0045, BOARD_Z, BOARD_Z + 0.003, "paint2", sides=12, bevel=0.0008)
    # The processor, its memory, a driver, the crystal.
    U1 = qfp(P, -0.06, 1.0, 0.07, 10, "U1")   # a pin every 6.4 mm, so the traces read apart
    U2 = soic(P, 0.075, 1.085, 0.06, 0.026, 8, "U2")
    U3 = soic(P, 0.075, 0.93, 0.06, 0.026, 8, "U3")
    U4 = qfp(P, -0.2, 1.135, 0.036, 5, "U4")
    P.box("xtal", (-0.075, 0.885, BOARD_Z), (-0.045, 0.897, BOARD_Z + 0.004), "paint2", bevel=0.0035)
    silk_box(P, -0.06, 0.891, 0.036, 0.018, "Y1", ly=0.902)
    # Buses with 45 degree bends: the processor's right side to the memory, its top to the header, its left to the
    # driver, its bottom to the edge fingers.
    r = U1["r"]
    # The upper right pins each to a memory pin: along, up at 45 degrees, and straight in; no two share a path.
    for k, tip in enumerate(r[5:]):
        px, py = U2["b"][k]
        dd = min(py - tip[1] - 0.006, px - tip[0] - 0.006)
        trace(P, [tip, (px - dd, tip[1]), (px, tip[1] + dd), (px, py)], via=False)
    for k, tip in enumerate(r[:5]):   # down the right side, stopping short of U3
        trace(P, route(tip, [("e", 0.004 + 0.004 * (4 - k)), ("se", 0.015), ("e", 0.012)]))
    for k, tip in enumerate(U1["t"]):
        trace(P, route(tip, [("n", 0.03), ("ne", 0.04), ("n", 0.07)]) if k >= 5 else route(tip, [("n", 0.02), ("nw", 0.03), ("n", 0.05)]))
    bus(P, U1["l"][1:9], [("w", 0.018), ("nw", 0.04), ("w", 0.03)])
    bus(P, U4["b"], [("s", 0.015), ("sw", 0.02), ("s", 0.06)])
    bus(P, U4["l"], [("w", 0.008)])
    for k, tip in enumerate(U1["b"]):
        trace(P, route(tip, [("s", 0.012), ("se", 0.03), ("s", tip[1] - 0.012 - 0.03 - (by0 + 0.022))]), via=False)
    # The memory's outer pins fan out to the right, the leftmost deepest, so no trace crosses another.
    for chip_pins, way in ((U2["t"], "n"), (U3["b"], "s")):
        for k, tip in enumerate(chip_pins):
            trace(P, route(tip, [(way, 0.006 + 0.0048 * (len(chip_pins) - 1 - k)), ("e", 0.11 - (tip[0] - 0.045))]))
    # The edge fingers the bottom bus reaches.
    for tip in U1["b"]:
        fx = tip[0] + 0.03
        P.box("finger", (fx - 0.0018, by0, BOARD_Z), (fx + 0.0018, by0 + 0.022, BOARD_Z + 0.0008), "trim")
    # Power: a regulator on its heatsink, two inductors and the capacitors round them, with wide traces.
    hx0, hx1, hy0, hy1 = 0.175, 0.25, 1.06, 1.17
    P.box("sink_base", (hx0, hy0, BOARD_Z), (hx1, hy1, BOARD_Z + 0.004), "upholstery_panel", bevel=0.001)
    for k in range(10):
        x = hx0 + 0.004 + k * (hx1 - hx0 - 0.008) / 9
        P.box("fin", (x - 0.002, hy0, BOARD_Z + 0.004), (x + 0.002, hy1, BOARD_Z + 0.032), "upholstery_panel", bevel=0.0008)
    silk_box(P, (hx0 + hx1) / 2, (hy0 + hy1) / 2, hx1 - hx0 + 0.006, hy1 - hy0 + 0.006, "Q1")
    for lx, ly, lab in ((0.165, 0.95, "L1"), (0.165, 0.88, "L2")):
        P.box("inductor", (lx - 0.016, ly - 0.016, BOARD_Z), (lx + 0.016, ly + 0.016, BOARD_Z + 0.012), "upholstery_panel", bevel=0.003)
        P.text("100", lx - 0.009, ly - 0.004, 0.009, z=BOARD_Z + 0.012)
        silk_box(P, lx, ly, 0.038, 0.038, lab)
    for x, y, lab in ((0.225, 0.985, "C1"), (0.225, 0.935, "C2"), (0.225, 0.885, "C3"), (-0.215, 0.98, "C4"), (-0.215, 0.925, "C5")):
        can(P, x, y, 0.014, lab)
    trace(P, [(0.165, 1.06), (0.165, 0.968)], w=0.008, via=False)
    trace(P, [(0.181, 0.95), (0.211, 0.95), (0.211, 0.985)], w=0.008, via=False)
    trace(P, [(0.181, 0.88), (0.211, 0.88)], w=0.008, via=False)
    # Small parts in rows beside the chips, with their outlines.
    for (sx, sy, n, vertical) in ((-0.15, 1.05, 6, False), (-0.15, 0.94, 6, False), (0.02, 1.15, 5, False), (0.0, 0.86, 4, False)):
        for k in range(n):
            px = sx + k * 0.0115
            chip_part(P, px, sy, 0.0075, 0.0042, "upholstery_panel" if k % 3 else "paint2")
        P.text("R" + str(int(sx * 100 + 30)), sx - 0.004, sy + 0.006, 0.0075, z=BOARD_Z)
    # The header at the top edge and its wires, the LEDs, test points, the board's name.
    jx0, jx1 = 0.0, 0.11
    P.box("header", (jx0, 1.172, BOARD_Z), (jx1, 1.198, BOARD_Z + 0.014), "upholstery_panel", bevel=0.0015)
    for k in range(8):
        for row in (1.179, 1.191):
            P.box("hpin", (jx0 + 0.007 + k * 0.0135 - 0.0018, row - 0.0018, BOARD_Z + 0.014), (jx0 + 0.007 + k * 0.0135 + 0.0018, row + 0.0018, BOARD_Z + 0.016), "trim")
    for k in range(4):
        x = jx0 + 0.02 + k * 0.024
        P.cable("wire", [(x, 1.19, 0.018), (x + 0.003, 1.205, 0.03), (x + 0.008 * (k - 1.5), 1.25, 0.03)], 0.0036, "walk" if k % 2 else "rubber")
    silk_box(P, (jx0 + jx1) / 2, 1.185, jx1 - jx0 + 0.006, 0.032, "J1", lx=jx1 + 0.004, ly=1.18)
    P.box("led", (-0.245, 1.172, BOARD_Z), (-0.236, 1.18, BOARD_Z + 0.004), "accent", bevel=0.001)
    P.box("led", (-0.23, 1.172, BOARD_Z), (-0.221, 1.18, BOARD_Z + 0.004), "amber", bevel=0.001)
    P.text("PWR ACT", -0.247, 1.185, 0.0085, z=BOARD_Z)
    for tx, ty in ((-0.12, 0.83), (0.11, 1.0), (-0.24, 1.07), (0.05, 0.825)):
        wp.ring(P, "testpoint", tx, ty, 0.0045, 0.002, BOARD_Z, BOARD_Z + 0.0009, "trim", sides=16)
    P.text("VALVE CTL 3B", -0.245, 0.833, 0.013, z=BOARD_Z)
    P.text("REV C  2291", -0.245, 0.815, 0.009, z=BOARD_Z)


def interior_wiring(P, d):
    x0, x1, y0, y1 = frame_of(d)
    P.box("back", (x0 - 0.05, y0 - 0.05, -0.04), (x1 + 0.05, y1 + 0.05, -0.012), "machinery")
    m = 0.018
    plate = P.box("mount", (x0 + m, y0 + m, -0.012), (x1 - m, y1 - m, -0.008), "bulkhead", bevel=0.001)
    P.cut(plate, "mount_holes", [P.cyl("hole", "z", (sx, sy), 0.004, -0.02, 0.0, "machinery", sides=14, smooth=False)
                                 for sx in (x0 + 0.03, x1 - 0.03) for sy in (y0 + 0.03, y1 - 0.03)])
    # A slotted wire duct along the top, the wires dropping out of its fingers.
    dy0, dy1 = 1.15, 1.2
    duct = P.box("duct", (x0 + 0.04, dy0, -0.008), (x1 - 0.04, dy1, 0.022), "paint2", bevel=0.0015)
    P.cut(duct, "duct_slots", [P.box("slot", (x0 + 0.05 + k * 0.024, dy0 - 0.01, 0.0), (x0 + 0.058 + k * 0.024, dy0 + 0.03, 0.05), "machinery")
                               for k in range(19)])
    P.box("duct_lid", (x0 + 0.038, dy1 - 0.004, 0.022), (x1 - 0.038, dy1 + 0.003, 0.026), "paint2", bevel=0.001)
    # The DIN rail and its terminal blocks.
    ry = 1.03
    rail = P.box("rail", (x0 + 0.03, ry - 0.0175, -0.008), (x1 - 0.03, ry + 0.0175, -0.002), "trim", bevel=0.0008)
    P.cut(rail, "rail_slots", [P.box("rslot", (x0 + 0.045 + k * 0.05, ry - 0.0035, -0.02), (x0 + 0.065 + k * 0.05, ry + 0.0035, 0.01), "machinery")
                               for k in range(10)])
    n, bw = 14, 0.0175
    bx = -0.05 - n * bw / 2 + 0.06
    tops = []
    for k in range(n):
        x = bx + k * bw
        role = "upholstery" if k in (4, 9) else "safety" if k == 13 else "paint2"
        P.box("block", (x + 0.0006, ry - 0.045, -0.002), (x + bw - 0.0006, ry + 0.045, 0.026), role, bevel=0.0012)
        for sy in (1, -1):
            yc = ry + sy * 0.028
            P.cyl("screw_well", "z", (x + bw / 2, yc), 0.0055, 0.02, 0.0265, "machinery", sides=14, smooth=False)
            P.cyl("screw", "z", (x + bw / 2, yc), 0.0042, 0.02, 0.024, "trim", sides=14, bevel=0.001)
            P.box("screw_slot", (x + bw / 2 - 0.0035, yc - 0.0007, 0.0236), (x + bw / 2 + 0.0035, yc + 0.0007, 0.0246), "machinery")
        P.box("marker", (x + 0.002, ry - 0.007, 0.026), (x + bw - 0.002, ry + 0.007, 0.0272), "stencil")
        tops.append(x + bw / 2)
    # Wires: up from each block into the duct, down from each into the loom.
    for k, x in enumerate(tops):
        role = WIRES[k % 4]
        P.cable("wire_up", [(x, ry + 0.045, 0.012), (x, ry + 0.07, 0.016), (x0 + 0.054 + round((x - x0 - 0.054) / 0.024) * 0.024, dy0 + 0.005, 0.012)], 0.0032, role)
        lx = -0.02 + (k - n / 2) * 0.0042
        P.cable("wire_down", [(x, ry - 0.045, 0.012), (x, ry - 0.075, 0.018), (lx * 0.6 + x * 0.4, 0.88, 0.02), (lx, 0.8, 0.02), (lx, y0 - 0.02, 0.018)], 0.0032, role)
    for yt in (0.86, 0.815):
        P.box("tie", (-0.055, yt - 0.003, 0.012), (0.015, yt + 0.003, 0.0275), "stencil", bevel=0.001)
    # A relay with its lamp, and the box's label.
    rx0, rx1, ry0, ry1 = 0.12, 0.235, 0.8, 0.93
    P.box("relay", (rx0, ry0, -0.008), (rx1, ry1, 0.03), "upholstery_panel", bevel=0.0025)
    P.box("relay_face", (rx0 + 0.008, ry0 + 0.04, 0.03), (rx1 - 0.008, ry1 - 0.008, 0.032), "paint2", bevel=0.001)
    P.cyl("relay_lamp", "z", (rx0 + 0.02, ry0 + 0.02), 0.006, 0.03, 0.036, "amber", sides=16, bevel=0.001)
    P.text("K1", rx0 + 0.016, ry1 - 0.04, 0.02, z=0.032)
    P.cyl("relay_lamp2", "z", (rx0 + 0.04, ry0 + 0.02), 0.006, 0.03, 0.036, "accent", sides=16, bevel=0.001)
    P.text("M1 MOTOR", x0 + 0.035, 0.935, 0.017, z=-0.008)
    P.text("440 V  3 PH", x0 + 0.035, 0.912, 0.011, z=-0.008)


INTERIOR_BUILDERS = {"circuit": interior_circuit, "wiring": interior_wiring}


# ----------------------------------------------------------------------------- render

def render(d, name, F, role_colour, build):
    """Beauty and mask renders of one target at supersample x out_px; returns their paths and build()'s result."""
    D = wp.load_panels()
    R = D["render"]
    wp.set_roles()
    k = d["supersample"]
    res = (d["out_px"][0] * k, d["out_px"][1] * k)
    out, result = {}, None
    os.makedirs(RAW, exist_ok=True)
    for mask in (False, True):
        wp.reset(R, R["mask_samples"] if mask else (SAMPLES or R["samples"]), res, mask=mask)
        wp.make_materials(F, False, streaky=False, role_colour=role_colour)
        P = wp.Panel(name)
        result = build(P, d)
        wp.camera(0.0, 1.0, d["size_m"][0], res)
        wp.check_roles(name)
        if mask:
            wp.mask_materials()
        path = os.path.join(RAW, f"{name}.{'mask' if mask else 'beauty'}.exr")
        wp.render_to(path)
        out["mask" if mask else "beauty"] = path
    return out, result


def write(d, name, raw):
    img = wp.finish_image(wp.read_exr(raw["beauty"]), wp.read_exr(raw["mask"]), d["out_px"][0], d["colours"])
    path = os.path.join(OUT, f"{name}.png")
    wp.mm.save_png(wp.Image.fromarray(img, "RGBA"), path)
    return {"file": os.path.relpath(path, ROOT), "px": d["out_px"], "bytes": os.path.getsize(path), "sha256": wp.sha256(path)}


SAMPLES = 0


def main():
    global SAMPLES
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--samples", type=int, default=0, help="override render.samples for a quick look")
    ap.add_argument("--only", default="", help="comma-separated targets to render (cover_<finish>, interior_<kind>)")
    a = ap.parse_args()
    SAMPLES = a.samples
    only = set(filter(None, a.only.split(",")))
    old_path = os.path.join(OUT, "covers.json")
    old = json.load(open(old_path, encoding="utf-8")) if os.path.exists(old_path) else {"covers": {}, "interiors": {}}
    d = load()
    D = wp.load_panels()
    covers, interiors = {}, {}
    for fn in d["finishes"]:
        if only and f"cover_{fn}" not in only:
            covers[fn] = old["covers"][fn]
            continue
        raw, holes = render(d, f"cover_{fn}", D["finishes"][fn], None, build_cover)
        covers[fn] = dict(write(d, f"cover_{fn}", raw), holes_uv=[[round(u, 5), round(v, 5)] for u, v in holes])
        print(f"[covers] {covers[fn]['file']}: holes {covers[fn]['holes_uv']}")
    inner = d["interiors"]
    for kind in sorted(inner["kinds"]):
        if only and f"interior_{kind}" not in only:
            interiors[kind] = old.get("interiors", {})[kind]
            continue
        F = {"colours_srgb": copy.deepcopy(inner["colours_srgb"]), "wear": inner["wear"]}
        raw, _ = render(d, f"interior_{kind}", F, inner["kinds"][kind]["roles"], lambda P, d, b=INTERIOR_BUILDERS[kind]: b(P, d))
        interiors[kind] = write(d, f"interior_{kind}", raw)
        print(f"[covers] {interiors[kind]['file']}")
    man = {"schema": "starcrew.repair-covers-built/1", "source": os.path.relpath(DATA, ROOT),
           "built_by": os.path.relpath(__file__, ROOT), "size_m": d["size_m"],
           "versions": {"blender": wp.bpy.app.version_string, "pillow": PIL.__version__, "numpy": wp.np.__version__},
           "covers": covers, "interiors": interiors}
    with open(os.path.join(OUT, "covers.json"), "w", encoding="utf-8") as f:
        json.dump(man, f, indent=2)
        f.write("\n")


if __name__ == "__main__":
    main()
