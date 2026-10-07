"""Star Crew's bridge furniture, modelled in Blender the hard-surface CSG way: wall-built console
banks between pierced structural fins, a free-standing helm desk, a curved two-seat helm in the
first series' style, the captain's chair (after a TNG captain's chair), a crew chair of the same family
and a stand-up console.

It owns the bridge props' geometry (assets/models/bridge/<name>.glb) and their manifest
(assets/models/bridge/props.json). It lives in tools/blender because meshes are files built by a
committed generator (CLAUDE.md section 9): this script is the source, the .glb files are its
output, and a second run writes the same bytes. It holds only the bridge's props and their hand
controls; the machinery every prop set shares (materials, primitives, the Prop class and its CSG
steps, clean, check, the glb export and read-back, the manifest and the command line) is the
hard-surface kit, tools/blender/hs_kit.py. How to work this way is the blender-hard-surface skill
(.claude/skills/blender-hard-surface).

Run (from anywhere):
  <python with the bpy module> tools/blender/build_bridge_props.py [--check] [--only a,b] [--blend out.blend]
  blender -b --factory-startup -P tools/blender/build_bridge_props.py -- [--check] [--only a,b] [--blend out.blend]
  --check   build in memory and compare with the committed .glb files and props.json; write nothing
  --only    build only these props (the manifest keeps the others' rows)
  --blend   also save the scene with every cutter collection, for looking at the CSG in Blender
            (a debugging view, not a source: it is not reproducible and is not committed)

The method (the skill's "cutter workflow"): every prop is blocked out from primitives (boxes,
extruded profiles, convex hulls, a lathe for the arc), carved by named cutter objects in a
collection through a Boolean modifier (Exact solver, materials transferred from the cutter's
faces), joined to its other pieces by Boolean unions, chamfered by a 1-segment Bevel on the
edges that should catch light, then cleaned (merge by distance, degenerate and limited
dissolve) and triangulated.

Conventions (written into props.json too):
  * Metres. Prop space is the exported glTF frame: +Y up, +Z toward the operator (out of the
    console's working face; for a chair, the way the sitter faces), +X the operator's right as
    they face the prop. Blender is Z-up: hs_kit.PROP_TO_BLENDER turns every primitive into
    Blender's frame as it is made (x, y, z to x, -z, y), and the glTF export (export_yup) turns
    it back.
  * The origin is on the floor at the centre of the prop's back: the wall plane for a wall bank,
    the back of the pedestal for everything else.
  * One material per role, named for a Star Crew material (data/materials/materials.json:
    machinery, trim, bulkhead, hazard, light_panel) or `screen` (emissive, coloured by the page)
    or `accent` (the station's role colour, set by the page).
  * UV0 is in metres, projected per face exactly as shipkit.js worldUv does: x, z on faces whose
    normal is within 41 degrees of vertical, else the face's horizontal tangent and y. The page
    divides by the material's span_m.
  * Flat shaded (normals split per face), triangulated, every prop one closed manifold solid.
  * Triangle budgets (BUDGETS): a prop over its budget is refused, named with its count, and
    nothing is written.
"""
import math
import os
import sys

import bpy  # noqa: F401  first: with the pip bpy module, mathutils exists only once bpy is imported
from mathutils import Matrix, Vector  # noqa: E402

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from hs_kit import (ROOT, Prop, PropSet, clip_polygon, frame, hull, in_frame, lathe, ngon, obox,  # noqa: E402
                    prism, run, triangle_inset)

OUT = os.path.join(ROOT, "assets", "models", "bridge")
GENERATOR = "tools/blender/build_bridge_props.py"

# Triangles per prop: the budgets these props were briefed with (2026-10-05), inside
# bridge-stations section 12's allowance of 700 for a desk with its screen and 300 for a seat.
# A station's variant (the same body with its own hand controls, bridge-stations 11.6) keeps its
# base's budget. The chairs' were raised on 2026-10-07 (captain 300 to 900, crew 160 to 480) for the
# owner's references ("chairs suck still mainly its a texture problem", with a TNG captain's chair,
# a burgundy one and Bridge Commander's): a slotted back with wings and a headrest, bolsters, a rolled
# seat front, angled arm supports and a louvred pedestal cannot be drawn in a seat's 300.
BUDGETS = {
    "wall_bank_core": 420,
    "wall_bank_core_engineering": 420,
    "wall_bank_core_science": 420,
    "wall_bank": 420,
    "wall_bank_comms": 420,
    "wall_bank_flight_ops": 420,
    "wall_bank_double": 700,
    "free_console": 420,
    "free_console_helm": 420,
    "free_console_tactical": 420,
    "helm_arc": 600,
    "captain_chair": 900,
    "crew_chair": 480,
    "standup_console": 160,
}

TAN20 = math.tan(math.radians(20.0))


# ----------------------------------------------------------------------------- hand controls
# Keys are texture; levers are geometry (bridge-stations 11.6): the controls a player's hand reaches
# for are small solids unioned onto a desk, each built in a desk frame m (frame(): local +Z the
# desk's normal, +Y up the desk away from the operator, +X their right). Every piece reaches 1-2 cm
# into what it stands on, and pieces that meet keep parallel faces at least 1 cm apart, so the
# union leaves no coplanar faces (CLAUDE.md 8). Sizes are a hand's: a grip 7 cm across, a guard
# 7 cm square, a toggle 4 cm tall.

def throttle(p, m, tag):
    """A throttle lever: a quadrant housing on the desk, a lever pushed forward of centre and a
    grip across its top in the station's colour."""
    base = obox(p.coll, f"{p.name}.{tag}_quadrant", (-0.04, -0.09, -0.02), (0.04, 0.09, 0.035), m,
                {"*": "machinery", "+z": "bulkhead"})
    lever = hull(p.coll, f"{p.name}.{tag}_lever", in_frame(m, [(sx * 0.008, y, 0.02) for sx in (-1, 1) for y in (-0.012, 0.012)]
                                                         + [(sx * 0.007, 0.035 + y, 0.125) for sx in (-1, 1) for y in (-0.008, 0.008)]), "trim")
    grip = obox(p.coll, f"{p.name}.{tag}_grip", (-0.035, 0.021, 0.115), (0.035, 0.049, 0.145), m, {"*": "accent"})
    return [base, lever, grip]


def stick(p, m, tag):
    """A flight stick: a square boot, a shaft leaning a little toward the operator and a grip in
    the station's colour."""
    boot = hull(p.coll, f"{p.name}.{tag}_boot", in_frame(m, [(sx * 0.04, sy * 0.04, -0.02) for sx in (-1, 1) for sy in (-1, 1)]
                                                       + [(sx * 0.026, sy * 0.026, 0.03) for sx in (-1, 1) for sy in (-1, 1)]), "machinery")
    shaft = hull(p.coll, f"{p.name}.{tag}_shaft", in_frame(m, [(sx * 0.005, sy * 0.005, 0.02) for sx in (-1, 1) for sy in (-1, 1)]
                                                         + [(sx * 0.005, sy * 0.005 - 0.01, 0.13) for sx in (-1, 1) for sy in (-1, 1)]), "trim")
    grip = obox(p.coll, f"{p.name}.{tag}_grip", (-0.016, -0.034, 0.115), (0.016, 0.012, 0.195), m, {"*": "accent"})
    return [boot, shaft, grip]


def guarded_button(p, m, tag):
    """A guarded fire button (design 8.5's guarded controls, as hardware): a housing with hazard
    sides, the button in the station's colour, and its flip cover open, hinged at the back and
    leaning away from the operator."""
    housing = obox(p.coll, f"{p.name}.{tag}_housing", (-0.035, -0.035, -0.02), (0.035, 0.035, 0.022), m,
                   {"*": "hazard", "+z": "machinery"})
    button = obox(p.coll, f"{p.name}.{tag}_button", (-0.016, -0.016, 0.012), (0.016, 0.016, 0.036), m, {"*": "accent"})
    hinge = m @ Matrix.Translation((0.0, 0.03, 0.022)) @ Matrix.Rotation(math.radians(-15.0), 4, "X")
    cover = obox(p.coll, f"{p.name}.{tag}_cover", (-0.025, -0.006, -0.012), (0.025, 0.0, 0.07), hinge, {"*": "trim"})
    return [housing, button, cover]


def toggle(p, m, tag, up):
    """A breaker toggle: a three-sided handle 6 cm tall that widens into a paddle, thrown up
    (away from the operator) and dark when the breaker is closed, down and in the station's
    colour when it has tripped. Larger than life, so it reads from across the room."""
    lean = 0.02 if up else -0.02
    return hull(p.coll, f"{p.name}.{tag}", in_frame(m, [(-0.011, -0.008, -0.012), (0.011, -0.008, -0.012), (0.0, 0.012, -0.012),
                                                      (-0.014, -0.004 + lean, 0.06), (0.014, -0.004 + lean, 0.06),
                                                      (0.0, 0.006 + lean, 0.06)]), "machinery" if up else "accent")


def slider_bank(p, m, tag, levels):
    """A bank of faders: a block 3 cm high on the desk and a cap per fader in the station's
    colour, each at its own level along the block. Larger than life, so it reads from across
    the room."""
    block = obox(p.coll, f"{p.name}.{tag}_block", (-0.15, -0.035, -0.02), (0.15, 0.035, 0.03), m,
                 {"*": "machinery", "+z": "bulkhead"})
    caps = []
    for i, lv in enumerate(levels):
        x = -0.11 + i * 0.22 / (len(levels) - 1)
        y = -0.015 + 0.03 * lv
        caps.append(obox(p.coll, f"{p.name}.{tag}_cap_{i}", (x - 0.014, y - 0.01, 0.02), (x + 0.014, y + 0.01, 0.06), m,
                         {"*": "accent"}))
    return [block] + caps


def dial_bank(p, m, tag, n=3):
    """A bank of tuning dials: a block 3 cm high on the desk and n knobs on it, square and turned, their
    pointers and scales painted on by the atlas. Larger than life, so it reads from across the room."""
    block = obox(p.coll, f"{p.name}.{tag}_block", (-0.14, -0.035, -0.02), (0.14, 0.035, 0.03), m,
                 {"*": "machinery", "+z": "bulkhead"})
    knobs = []
    for i in range(n):
        x = -0.09 + i * 0.18 / (n - 1)
        knobs.append(obox(p.coll, f"{p.name}.{tag}_knob_{i}", (-0.02, -0.02, 0.02), (0.02, 0.02, 0.06),
                          m @ Matrix.Translation((x, 0.0, 0.0)) @ Matrix.Rotation(math.radians(45.0 + 20.0 * i), 4, "Z"),
                          {"*": "trim"}))
    return [block] + knobs


# ----------------------------------------------------------------------------- the props

# ----------------------------------------------------------------------------- the atlas's details
# What each console and chair carries on its atlas (hs_kit.Detail, baked onto the prop's own UV; the
# owner, 2026-10-07: "the textures on the console man, they need to be full on custom details, youre
# using generic textures on them"). A station's console says whose it is with a stencilled name plate
# (a station-less one an AUX number), and no two kinds look alike: each has its own scheme of keys,
# lamps and status displays. Sizes suit the atlas's 50-110 px per metre: keys 3-4 cm, lamps 1-2 cm
# (a lamp reads by its glow), letters 5-8 cm tall.

LABELS = {
    "wall_bank_core": ("AUX 01",), "wall_bank_core_engineering": ("ENGINEERING",), "wall_bank_core_science": ("SCIENCE",),
    "wall_bank": ("AUX 02",), "wall_bank_comms": ("COMMS",), "wall_bank_flight_ops": ("FLIGHT OPS",),
    "wall_bank_double": ("AUX 03", "AUX 04"), "free_console": ("AUX 05",), "free_console_helm": ("HELM",),
    "free_console_tactical": ("TACTICAL",), "helm_arc": ("HELM", "TACTICAL"), "standup_console": ("AUX 06",),
}
# a station's label to its role in shipkit's PALETTE (its stripe's colour, finish role_<role>)
ROLE_OF_LABEL = {"HELM": "helm", "TACTICAL": "tactical", "ENGINEERING": "engineering", "SCIENCE": "science",
                 "COMMS": "comms", "FLIGHT OPS": "flight_ops"}
# a console kind's keys (cycling: a glow finish is a lit key), lamps and status displays
SCHEMES = {
    None: {"keys": ("key", "key", "led_white", "key", "key", "led_green"), "leds": ("led_green", "led_green", "led_amber"),
           "display": "display"},
    "breakers": {"keys": ("key", "led_amber", "key", "key"), "leds": ("led_amber", "led_green", "led_red"), "display": "display_amber"},
    "sliders": {"keys": ("key", "led_green", "key"), "leds": ("led_green", "led_amber", "led_green"), "display": "display"},
    "dials": {"keys": ("key", "led_blue", "key", "led_white"), "leds": ("led_blue", "led_white"), "display": "display"},
    "levers": {"keys": ("key", "led_red", "key", "led_green"), "leds": ("led_red", "led_green", "led_amber"), "display": "display_amber"},
    "helm": {"keys": ("key", "led_blue", "key", "key"), "leds": ("led_blue", "led_white"), "display": "display"},
    "tactical": {"keys": ("key", "led_red", "key", "led_amber"), "leds": ("led_red", "led_amber"), "display": "display_amber"},
}


def label_size(text, w, h):
    """Letters as tall as fit a plate w x h: at most 0.6 of its height, and the label 0.85 of its width
    (the built-in font's capitals advance about 0.62 of their height)."""
    return min(h * 0.6, 0.85 * w / (0.62 * max(1, len(text))))


def name_plate(D, m, x, y, w, h, text):
    D.plate(m, x, y, w, h, text, size=label_size(text, w, h))


def led_column(D, m, x, ys, finishes):
    for k, y in enumerate(ys):
        D.disc(m, x, y, 0.009, -0.002, 0.005, finishes[k % len(finishes)], sides=10)


def wall_bank_decor(p, name, bay_w, seats, controls, D_, desk_at, desk_tilt, band, pc, main, z_wall, yc, w_up):
    """A wall bank's details: per seat a name plate on its access panel over a vent, cable glands
    under it, a row of keys and lamps behind the keyboard well (left of a comms bank's faders), lit
    keys along the button band, lamp columns beside the main screen, status displays under the upper
    screens and a lit seam between them; engineering a HIGH VOLTAGE placard in place of the vent."""
    labels = LABELS[name]
    sc = SCHEMES[controls]
    xs = [0.0] if seats == 1 else [-bay_w / 4, bay_w / 4]
    seat_w = bay_w if seats == 1 else bay_w / 2
    band_tilt = math.degrees(math.atan2(band[0][0] - band[1][0], band[1][1] - band[0][1]))
    band_c = ((band[0][1] + band[1][1]) / 2, (band[0][0] + band[1][0]) / 2)

    def decor(D):
        for i, cx in enumerate(xs):
            aw = seat_w - 0.20
            ma = frame((cx, 0.0, D_ - 0.012))          # the access panel's floor, 12 mm in
            name_plate(D, ma, 0.0, 0.535, min(aw - 0.10, 0.70), 0.11, labels[i])
            if controls == "breakers":
                D.box(ma, -0.20, 0.25, 0.20, 0.41, -0.002, 0.004, "yellow", inset=0.003)
                D.text(ma, "HIGH VOLTAGE", 0.0, 0.33, label_size("HIGH VOLTAGE", 0.38, 0.14), "stencil_dark", z=0.0056)
                D.grille(ma, -aw / 2 + 0.06, 0.25, -0.26, 0.41, along="y", pitch=0.03)
                D.grille(ma, 0.26, 0.25, aw / 2 - 0.06, 0.41, along="y", pitch=0.03)
            else:
                D.grille(ma, -aw / 2 + 0.06, 0.25, aw / 2 - 0.06, 0.41, pitch=0.032 if controls != "sliders" else 0.022)
            for sx in (-1, 1):
                for yy in (0.23, 0.59):
                    D.disc(ma, sx * (aw / 2 - 0.03), yy, 0.01, -0.002, 0.007, "bolt", sides=10, inset=0.003)
            mf = frame((cx, 0.0, D_))
            if labels[i] in ROLE_OF_LABEL:      # the station's stripe under the desk's edge
                D.paint(mf, [(-seat_w / 2 + 0.03, 0.655), (seat_w / 2 - 0.03, 0.655), (seat_w / 2 - 0.03, 0.695),
                             (-seat_w / 2 + 0.03, 0.695)], "role_" + ROLE_OF_LABEL[labels[i]])
            for sx in (-1, 1):
                D.ring(mf, sx * (seat_w / 2 - 0.16), 0.15, 0.013, 0.026, -0.002, 0.012, "bolt")
                D.disc(mf, sx * (seat_w / 2 - 0.16), 0.15, 0.013, -0.002, 0.004, "rubber", sides=12, reserve=False)
            # behind the keyboard well
            md = frame(desk_at(cx, 0.235), desk_tilt)
            kw = 1.20 if bay_w >= 1.4 and seats == 1 else 0.86
            if controls == "breakers":
                D.leds(md, -kw / 2 + 0.03, 0.0, 3, 0.04, sc["leds"])
                D.leds(md, kw / 2 - 0.11, 0.0, 3, 0.04, sc["leds"][::-1])
            elif controls == "sliders":
                D.buttons(md, -kw / 2 + 0.02, 0.0, 6, 0.06, 0.04, sc["keys"])
                D.leds(md, -kw / 2 + 0.40, 0.0, 3, 0.035, sc["leds"])
            elif controls in ("dials", "levers"):
                D.buttons(md, -kw / 2 + 0.02, 0.0, 4, 0.06, 0.04, sc["keys"])
                D.leds(md, kw / 2 - 0.12, 0.0, 3, 0.04, sc["leds"])
            else:
                n = int((kw - 0.20) / 0.06)
                D.buttons(md, -kw / 2 + 0.02, 0.0, n, 0.06, 0.04, sc["keys"])
                D.leds(md, kw / 2 - 0.08, 0.0, 2, 0.035, sc["leds"])
            # the button band, lit keys along it (the page multiplies this band by the station's colour)
            mb = frame((cx, band_c[0], band_c[1]), band_tilt)
            n = int((seat_w - 0.16) / 0.07)
            D.buttons(mb, -(n - 1) * 0.07 / 2 - 0.016, 0.0, n, 0.07, 0.032, sc["keys"][::-1])
            # lamps beside the main screen
            mx, mw = main[i]
            mm_ = frame((mx, pc[1], pc[2]), 20.0)
            for sx in (-1, 1):
                led_column(D, mm_, sx * (mw / 2 + 0.045), (-0.12, -0.06, 0.0, 0.06, 0.12), sc["leds"])
        # status displays under the upper screens and a lit seam between each pair
        mw_ = frame((0.0, 0.0, z_wall))
        n_up = 2 * seats
        for k in range(n_up):
            cx = -bay_w / 2 + 0.08 + w_up / 2 + k * (w_up + 0.08)
            for j in range(3):
                x = cx - w_up / 2 + 0.04 + j * (w_up - 0.08) / 2
                D.box(mw_, x - 0.045, 1.445, x + 0.045, 1.47, -0.002, 0.003, sc["display"] if j != 1 else "led_white")
            if k < n_up - 1:
                gx = cx + w_up / 2 + 0.04
                D.box(mw_, gx - 0.007, yc - 0.17, gx + 0.007, yc + 0.17, -0.002, 0.004, sc["leds"][0])
    p.decor.append(decor)


def free_console_decor(p, name, controls, top, zb, zf, m_h, zt, zbb):
    """A free console's details: a name plate on the pedestal's sloped front over a vent, lamps under the
    upright screen, lit keys in the button band, keys on the desk's free side strips (a throttle,
    a stick or fire buttons take theirs), a row of lamps along the front edge."""
    sc = SCHEMES[controls]
    label = LABELS[name][0]
    tilt_ped = -math.degrees(math.atan2(0.14, 0.60))      # the pedestal's front leans 13 degrees forward

    def decor(D):
        mp = frame((0.0, 0.30, 0.26 + 0.14 * 0.30 / 0.60), tilt_ped)
        name_plate(D, mp, 0.0, 0.11, 0.58, 0.12, label)
        if label in ROLE_OF_LABEL:
            D.paint(mp, [(-0.40, 0.21), (0.40, 0.21), (0.40, 0.245), (-0.40, 0.245)], "role_" + ROLE_OF_LABEL[label])
        D.grille(mp, -0.28, -0.20, 0.28, -0.06, pitch=0.03)
        D.leds(m_h, -0.36, 0.005, 8, 0.025, sc["leds"], r=0.007)
        mbb = frame((0.0, top(zbb), zbb), 78.0) @ Matrix.Translation((0.0, 0.0, -0.012))
        D.buttons(mbb, -0.53, 0.0, 15, 0.072, 0.034, sc["keys"])
        for side in (-1, 1):
            ms = frame((side * 0.625, top(zt), zt), 78.0)
            free = (controls is None) or (controls == "tactical" and side < 0)
            if free:
                for r in range(3):
                    D.buttons(ms, -0.05, -0.07 + 0.07 * r, 3, 0.045, 0.034, sc["keys"][r:] + sc["keys"][:r])
        mfr = frame((0.0, top(zf - 0.035), zf - 0.035), 78.0)
        D.leds(mfr, -0.45, 0.0, 10, 0.1, sc["leds"], r=0.007)
    p.decor.append(decor)


def helm_arc_decor(p, r_back, half, steps, r_mid):
    """The helm arc's details: a name plate on the front of each seat's facet (HELM, TACTICAL), lamps
    along the sloped top's front edge, lit keys along the rim's band and vents on the hood's sides."""
    facet = 2 * half / steps

    def decor(D):
        for k in range(steps):
            phi = -half + (k + 0.5) * facet
            cf = math.cos(facet / 2)          # a facet is the chord: its middle is cos(facet / 2) of the radius out
            r = (r_back - 0.60) * cf
            mfront = frame((math.sin(phi) * r, 0.0, r_back - math.cos(phi) * r), 0.0, -math.degrees(phi))
            chord = 2 * (r_back - 0.60) * math.sin(facet / 2)
            if k in (1, steps - 2):
                text = LABELS["helm_arc"][0 if k == 1 else 1]
                name_plate(D, mfront, 0.0, 0.55, chord - 0.05, 0.10, text)
                D.paint(mfront, [(-chord / 2 + 0.01, 0.65), (chord / 2 - 0.01, 0.65), (chord / 2 - 0.01, 0.685),
                                 (-chord / 2 + 0.01, 0.685)], "role_" + ROLE_OF_LABEL[text])
                D.grille(mfront, -chord / 2 + 0.05, 0.22, chord / 2 - 0.05, 0.38, pitch=0.03)
            elif k in (3, 4):
                D.grille(mfront, -chord / 2 + 0.05, 0.30, chord / 2 - 0.05, 0.55, along="y", pitch=0.035)
            # the top's front strip: lamps; the rim band: lit keys
            sc = SCHEMES["helm" if phi < 0 else "tactical"]
            d = 0.56
            y = 0.75 + (0.60 - d) / 0.46 * 0.10
            slope = math.degrees(math.atan2(0.10, 0.46))
            rt = (r_back - d) * cf
            mtop = frame((math.sin(phi) * rt, y, r_back - math.cos(phi) * rt), 90.0 - slope, -math.degrees(phi))
            D.leds(mtop, -chord / 2 + 0.05, 0.0, 4, (chord - 0.10) / 3, sc["leds"], r=0.007)
            rr = (r_back - 0.12) * cf
            mrim = frame((math.sin(phi) * rr, 0.875, r_back - math.cos(phi) * rr), math.degrees(math.atan2(0.04, 0.05)), -math.degrees(phi))
            D.buttons(mrim, -chord / 2 + 0.04, 0.0, 4, (chord - 0.08 - 0.03) / 3, 0.03, sc["keys"])
        for s in (-1, 1):
            mh = frame((s * 0.17, 0.89, 0.25), 0.0, s * 90.0)
            D.grille(mh, -0.12, -0.08, 0.10, 0.06, along="y", pitch=0.025)
    p.decor.append(decor)


def standup_decor(p, top, zb, zf):
    """The stand-up console's details: a name plate over a vent on its column, lamp columns either side
    of its screen."""
    sc = SCHEMES[None]

    def decor(D):
        mc = frame((0.0, 0.55, 0.18 + 0.06 * 0.52 / 0.87), -math.degrees(math.atan2(0.06, 0.87)))
        name_plate(D, mc, 0.0, 0.10, 0.24, 0.09, LABELS["standup_console"][0])
        D.grille(mc, -0.09, -0.25, 0.09, -0.05, along="y", pitch=0.03)
        slope = math.degrees(math.atan2(1.16 - 1.04, zf - zb))
        for side in (-1, 1):
            ms = frame((side * 0.29, top((zb + zf) / 2), (zb + zf) / 2), 90.0 - slope)
            led_column(D, ms, 0.0, (-0.10, -0.05, 0.0, 0.05, 0.10), sc["leds"])
    p.decor.append(decor)


def chair_decor(p, arms, az):
    """A chair's details: on the captain's, lit keys on each arm console's inner face and a plate on the
    pedestal's back; on both, bolts round the base."""
    def decor(D):
        if arms:
            zfa = az + 0.31
            for s in (-1, 1):
                mi = frame((s * 0.315, 0.63, zfa - 0.10), 0.0, -s * 90.0)
                D.buttons(mi, -0.075, 0.005, 4, 0.04, 0.026, ("led_amber", "key", "led_green", "key"))
            mb = frame((0.0, 0.20, 0.0), 0.0, 180.0)
            D.plate(mb, 0.0, 0.0, 0.2, 0.07, "CO", size=0.04)
    p.decor.append(decor)


def wall_bank(name, bay_w, seats, presents, controls=None):
    """A console bank built into a wall, in a bay between two structural fins.

    Profile (z toward the room, y up): a recessed toe kick, the cabinet front, a desk top
    sloping up from 0.75 m at its front edge, a steeper button band, the main screen panel
    leaning back 20 degrees, a vertical display wall up to 2.03 m and a lit chamfer under the
    2.10 m top; an access panel let into the cabinet front. The fins stand 6 cm proud of the
    desk (so no face of theirs shares its front's plane) and 12 cm above the top, their
    front edges raked at the screen panel's 20 degrees, pierced by a truss of triangular holes.
    A keyboard well 15 cm deep is let into the desk in front of each seat (its key panel is
    texture), and controls adds a station's own hand controls on the desk behind it:
    "breakers" (engineering: a row of five breaker toggles, one tripped), "sliders" (comms: a
    bank of four faders), "dials" (science: three sensor tuning dials) or "levers" (flight ops: a
    launch lever and an abort button). Its atlas's details are wall_bank_decor's."""
    p = Prop(name, presents, "floor, centre of the back, on the wall plane")
    p.wall = True
    hw = bay_w / 2
    D, H = 0.60, 2.10                       # desk depth, top of the bank
    toe_d, toe_h = 0.08, 0.10
    desk_front = (D, 0.75)
    desk_back = (0.32, 0.82)
    band_top = (0.29, 0.88)
    panel_top_y = 1.42
    z_wall = band_top[0] - (panel_top_y - band_top[1]) * TAN20
    strip = (z_wall, 2.02)                  # the light strip runs from here to the top
    strip_dz = 0.045

    # 1. Block out: the cabinet as one box.
    body = prism(p.coll, f"{name}.body", [(0, 0), (D, 0), (D, H), (0, H)], "x", -hw, hw,
                 ["machinery", "bulkhead", "machinery", "machinery"], cap="machinery")

    # 2. Carve the profile: toe kick, the desk, band, screen panel and display wall in one
    #    convex cutter (its faces carry their materials), and the light strip's chamfer.
    x0, x1 = -hw - 0.05, hw + 0.05
    ze = D + 0.05
    y_e = desk_front[1] - (desk_back[1] - desk_front[1]) * (ze - D) / (D - desk_back[0])
    carve = [
        prism(p.coll, f"{name}.cut_toe", [(D - toe_d, -0.05), (ze, -0.05), (ze, toe_h), (D - toe_d, toe_h)], "x",
              x0, x1, ["machinery", "machinery", "machinery", "light_panel"]),
        prism(p.coll, f"{name}.cut_front", [(ze, y_e), desk_back, band_top, (z_wall, panel_top_y), (z_wall, H + 0.1),
                                            (ze, H + 0.1)], "x", x0, x1,
              ["trim", "accent", "machinery", "machinery", "machinery", "machinery"]),
        prism(p.coll, f"{name}.cut_strip", [(strip[0] + 0.05 * strip_dz / 0.08, strip[1] - 0.05),
                                            (strip[0] - strip_dz * 1.6, strip[1] + 0.08 * 1.6),
                                            (strip[0] + 0.1, strip[1] + 0.08 * 1.6)], "x", x0, x1,
              ["light_panel", "machinery", "machinery"]),
    ]
    p.cut(body, "carve", carve)

    # 3. Chamfer the desk's front edge, before anything else meets it.
    p.chamfer(body, "desk_edge", 0.02,
              lambda m, d, n1, n2: abs(m.z - D) < 0.005 and abs(m.y - desk_front[1]) < 0.005 and abs(d.x) > 0.9)

    # 4. The fins: blocked out, chamfered on the edges that face the room, then pierced.
    DF, knee, HF = D + 0.06, 0.90, H + 0.12
    z_top = DF - (HF - knee) * TAN20
    fin_profile = [(0, 0), (DF, 0), (DF, knee), (z_top, HF), (0, HF)]
    # The free part of a fin: in front of the screen panel and display wall, above the band,
    # inside the fin's raked front edge. Webs: 5 cm to the fin's edges and to the cabinet.
    web, gap = 0.05, 0.05
    region = [(0.0, 0.97), (1.0, 0.97), (1.0, 2.12), (0.0, 2.12)]
    region = clip_polygon(region, 1.0, TAN20, DF + knee * TAN20 - web / math.cos(math.radians(20)))   # z <= front - web
    region = clip_polygon(region, -1.0, -TAN20, -(band_top[0] + band_top[1] * TAN20 + gap))           # z >= panel + gap
    region = clip_polygon(region, -1.0, 0.0, -(z_wall + gap))                                           # z >= wall + gap
    # a Warren truss across the strip: inner chord A-E (up the panel), outer chord B-C
    ys = sorted(set(round(q[1], 6) for q in region))
    A = min((q for q in region if abs(q[1] - ys[0]) < 1e-6), key=lambda q: q[0])
    Bq = max((q for q in region if abs(q[1] - ys[0]) < 1e-6), key=lambda q: q[0])
    E = (z_wall + gap, band_top[1] + (band_top[0] + gap - (z_wall + gap)) / TAN20)
    C = max(region, key=lambda q: q[1] - 0.001 * q[0])
    front = lambda y: DF - (y - knee) * TAN20 - web / math.cos(math.radians(20))
    P1 = (front(E[1]), E[1])
    holes = [triangle_inset(t, web / 2) for t in ((A, Bq, P1), (A, P1, E), (E, P1, C))]
    holes = [h for h in holes if h]
    fins = []
    for side in (-1, 1):
        xa, xb = sorted((side * (hw - 0.01), side * (hw + 0.12)))
        fin = prism(p.coll, f"{name}.fin_{'l' if side < 0 else 'r'}", fin_profile, "x", xa, xb, "trim")
        p.chamfer(fin, f"fin_{'l' if side < 0 else 'r'}_edges", 0.015,
                  lambda m, d, n1, n2: m.y > 0.01 and m.z > 0.01)
        cutters = [prism(p.coll, f"{name}.fin_hole_{i}", list(h), "x", xa - 0.05, xb + 0.05, "machinery")
                   for i, h in enumerate(holes)]
        p.cut(fin, f"fin_{'l' if side < 0 else 'r'}_holes", cutters)
        fins.append(fin)
    p.union(body, "fins", fins)

    # 5. A station's hand controls on the desk, behind the keyboard well (desk_at: a point s
    #    metres up the desk from its front edge).
    slope = math.atan2(desk_back[1] - desk_front[1], desk_front[0] - desk_back[0])
    desk_tilt = 90.0 - math.degrees(slope)
    desk_at = lambda x, s: (x, desk_front[1] + s * math.sin(slope), desk_front[0] - s * math.cos(slope))
    if controls == "breakers":
        pieces = [toggle(p, frame(desk_at(-0.30 + 0.15 * i, 0.235), desk_tilt), f"breaker_{i}", up)
                  for i, up in enumerate((True, True, False, True, True))]
        p.union(body, "breakers", pieces)
        p.controls.append("five breaker toggles behind the keyboard, the third tripped")
    elif controls == "sliders":
        p.union(body, "sliders", slider_bank(p, frame(desk_at(0.25, 0.2375), desk_tilt), "faders", (0.8, 0.45, 0.65, 0.15)))
        p.controls.append("a bank of four faders behind the keyboard, to the operator's right")
    elif controls == "dials":
        p.union(body, "dials", dial_bank(p, frame(desk_at(0.0, 0.2375), desk_tilt), "dials"))
        p.controls.append("a bank of three sensor tuning dials behind the keyboard")
    elif controls == "levers":
        ma = frame(desk_at(-0.25, 0.235), desk_tilt)
        p.union(body, "levers", throttle(p, frame(desk_at(0.25, 0.235), desk_tilt), "launch")
                + [obox(p.coll, f"{name}.abort_housing", (-0.035, -0.035, -0.02), (0.035, 0.035, 0.022), ma, {"*": "hazard", "+z": "machinery"}),
                   obox(p.coll, f"{name}.abort_button", (-0.018, -0.018, 0.012), (0.018, 0.018, 0.04), ma, {"*": "accent"})])
        p.controls.append("a launch lever behind the keyboard to the operator's right, an abort button in a hazard housing to their left")
    elif controls is not None:
        raise SystemExit(f"[props] {name}: unknown controls {controls!r}")

    # 6. Screens, cut last so no bevel or union touches their edges.
    screens = []
    t = math.hypot(band_top[0] - z_wall, panel_top_y - band_top[1])
    pc = (0.0, (band_top[1] + panel_top_y) / 2, (band_top[0] + z_wall) / 2)
    main_h = 0.50 if bay_w >= 1.4 else 0.45
    if seats == 1:
        main = [(0.0, 1.20 if bay_w >= 1.4 else 0.90)]
    else:
        main = [(-bay_w / 4, bay_w / 2 - 0.20), (bay_w / 4, bay_w / 2 - 0.20)]
    assert main_h <= t - 0.06, "main screen taller than its panel"
    for i, (cx, w) in enumerate(main):
        p.recess(screens, f"screen_main_{i}", frame((cx, pc[1], pc[2]), 20.0), w, main_h, 0.02, shows="console")
    n_up = 2 * seats
    w_up = (bay_w - 0.16 - (n_up - 1) * 0.08) / n_up
    yc = (panel_top_y + strip[1]) / 2
    for i in range(n_up):
        cx = -bay_w / 2 + 0.08 + w_up / 2 + i * (w_up + 0.08)
        p.recess(screens, f"screen_upper_{i}", frame((cx, yc, z_wall)), w_up, 0.40, 0.02, shows="upper", half=i % 2)
    # a keyboard well per seat, 12 mm deep, from 3.5 cm behind the desk's front edge
    for i, cx in enumerate([0.0] if seats == 1 else [-bay_w / 4, bay_w / 4]):
        kw = 1.20 if bay_w >= 1.4 and seats == 1 else 0.86
        p.recess(screens, f"keys_{i}", frame(desk_at(cx, 0.11), desk_tilt), kw, 0.15, 0.012, shows="keys")
    # an access panel per seat in the cabinet front, 12 mm deep (parallel faces stay 1 cm apart)
    for i, cx in enumerate([0.0] if seats == 1 else [-bay_w / 4, bay_w / 4]):
        w = (bay_w if seats == 1 else bay_w / 2) - 0.20
        p.recess(screens, f"access_{i}", frame((cx, 0.41, D)), w, 0.42, 0.012, floor_role="bulkhead", record=False)
    p.cut(body, "screens", screens)

    p.body = body
    wall_bank_decor(p, name, bay_w, seats, controls, D, desk_at, desk_tilt, (desk_back, band_top), pc, main, z_wall, yc, w_up)
    desk_mid = (desk_front[0] + 0.0) / 2
    for cx in ([0.0] if seats == 1 else [-bay_w / 4, bay_w / 4]):
        p.operators.append([round(cx, 4), 0.0, round(desk_mid + 0.65, 4)])
    p.fin_holes = len(holes)
    return p


def free_console(name, presents, controls=None):
    """A free-standing helm or tactical desk: a sloped top on an angled pedestal over a plinth,
    a touch panel and a button band let into the top and a small upright screen at its back
    edge. controls adds a station's own hand controls in the strips either side of the touch
    panel: "helm" (a throttle lever to the operator's left, a stick to their right) or
    "tactical" (two guarded fire buttons to their right)."""
    p = Prop(name, presents, "floor, centre of the pedestal's back")
    p.wall = False
    hw, zb, zf = 0.70, -0.04, 0.56          # desk half width, back and front edges (z)
    slope = math.tan(math.radians(12.0))
    top = lambda z: 0.75 + (zf - z) * slope
    desk = prism(p.coll, f"{name}.desk", [(zb, 0.58), (zf, 0.58), (zf, 0.95), (zb, 0.95)], "x", -hw, hw, "machinery")
    c = 0.14                                # plan chamfer on the front corners
    carve = [
        prism(p.coll, f"{name}.cut_top", [(zf + 0.05, top(zf + 0.05)), (zb - 0.05, top(zb - 0.05)), (zb - 0.05, 1.2),
                                          (zf + 0.05, 1.2)], "x", -hw - 0.05, hw + 0.05, "trim"),
        prism(p.coll, f"{name}.cut_under", [(zf + 0.05, 0.73), (zf - 0.26, 0.545), (zf + 0.05, 0.545)], "x",
              -hw - 0.05, hw + 0.05, "machinery"),
    ]
    for s in (-1, 1):
        pts = [(s * (hw - c - 0.05), zf + 0.05), (s * (hw + 0.05), zf - c - 0.05), (s * (hw + 0.05), zf + 0.05)]
        if s < 0:
            pts.reverse()
        carve.append(prism(p.coll, f"{name}.cut_corner_{s:+d}", pts, "y", 0.5, 1.3, "machinery"))
    p.cut(desk, "carve", carve)
    p.chamfer(desk, "top_edges", 0.02, lambda m, d, n1, n2: m.y > 0.70 and max(n1.y, n2.y) > 0.9)

    ped = hull(p.coll, f"{name}.pedestal", [(sx * 0.36, 0.0, z) for sx in (-1, 1) for z in (0.0, 0.26)]
               + [(sx * 0.52, 0.60, z) for sx in (-1, 1) for z in (0.0, 0.40)], "bulkhead")
    plinth = prism(p.coll, f"{name}.plinth", [(-0.08, 0.0), (0.36, 0.0), (0.36, 0.06), (-0.08, 0.06)], "x",
                   -0.46, 0.46, "trim")
    p.chamfer(plinth, "plinth_edges", 0.02, lambda m, d, n1, n2: m.y > 0.03)
    # The upright screen's housing leans back 20 degrees from the back of the top.
    zh = 0.15
    m_h = frame((0.0, top(zh), zh), 20.0)
    housing = obox(p.coll, f"{name}.housing", (-0.45, -0.06, -0.07), (0.45, 0.42, 0.0), m_h, {"*": "machinery"})
    p.chamfer(housing, "housing_edges", 0.015, lambda m, d, n1, n2: m.y > top(0.0) + 0.05)
    p.union(desk, "pedestal_plinth_housing", [ped, plinth, housing])

    on_top = lambda x, z: frame((x, top(z), z), 78.0)      # the desk top slopes 12 degrees
    if controls == "helm":
        p.union(desk, "helm_controls", throttle(p, on_top(-0.625, 0.33), "throttle") + stick(p, on_top(0.625, 0.33), "stick"))
        p.controls += ["a throttle lever to the operator's left", "a stick to their right"]
    elif controls == "tactical":
        p.union(desk, "fire_buttons", guarded_button(p, on_top(0.62, 0.26), "fire_0") + guarded_button(p, on_top(0.62, 0.40), "fire_1"))
        p.controls.append("two guarded fire buttons to the operator's right")
    elif controls is not None:
        raise SystemExit(f"[props] {name}: unknown controls {controls!r}")

    screens = []
    p.recess(screens, "screen_upright", m_h @ Matrix.Translation((0, 0.20, 0)), 0.80, 0.30, 0.015, shows="console")
    zt = 0.36
    p.recess(screens, "touch_panel", on_top(0.0, zt), 1.10, 0.24, 0.012, shows="keys")
    zb_ = 0.205                             # a button band in the station's colour behind the touch panel
    p.recess(screens, "button_band", frame((0.0, top(zb_), zb_), 78.0), 1.10, 0.05, 0.012, floor_role="accent",
             record=False)
    p.cut(desk, "screens", screens)
    p.body = desk
    free_console_decor(p, name, controls, top, zb, zf, m_h, zt, zb_)
    p.operators.append([0.0, 0.0, round((zb + zf) / 2 + 0.65, 4)])
    return p


def helm_arc(name, presents):
    """A curved two-seat console in the first series' style: a lathe of one profile (toe kicks
    front and back, a sloped top, a raised rim with a button band) over an arc 2.4 m long at
    mid depth, a hooded viewer between the two seats and a key panel let into the top at each.
    Seat 0 (to port of the centre, the operator's left) is helm's, with a throttle lever left of
    its panel and a stick between the panel and the viewer; seat 1 is tactical's, with two
    guarded fire buttons right of its panel. Each control stands on one facet of the lathe."""
    p = Prop(name, presents, "floor, centre of the back of the arc")
    p.wall = False
    depth, r_mid, arc = 0.60, 2.90, 2.40
    r_back = r_mid + depth / 2
    half = arc / r_mid / 2
    profile = [(0.08, 0.0), (0.46, 0.0), (0.46, 0.10), (0.60, 0.10), (0.60, 0.75), (0.14, 0.85), (0.10, 0.90),
               (0.0, 0.90), (0.0, 0.10), (0.08, 0.10)]
    roles = ["machinery", "light_panel", "machinery", "bulkhead", "machinery", "accent", "trim", "bulkhead",
             "machinery", "machinery"]
    steps = 8
    body = lathe(p.coll, f"{name}.body", profile, roles, r_back, r_back, -half, half, steps, "trim")
    p.chamfer(body, "front_edge", 0.02,
              lambda m, d, n1, n2: abs(math.hypot(m.x, m.z - r_back) - (r_back - 0.60)) < 0.01 and abs(m.y - 0.75) < 0.01)
    # the hooded viewer: a trapezoid prism, its front face toward the seats
    hood = hull(p.coll, f"{name}.hood", [(sx * 0.17, y, r_back - (r_back - d)) for sx in (-1, 1)
                                         for d, y in ((0.44, 0.76), (0.32, 1.02), (0.06, 1.02), (0.06, 0.82))], "bulkhead")
    p.chamfer(hood, "hood_edges", 0.012, lambda m, d, n1, n2: m.y > 1.0)
    p.union(body, "hood", [hood])
    slope = math.degrees(math.atan2(0.10, 0.46))
    facet = 2 * half / steps

    def on_top(phi, d):
        """A desk frame on the facet of the lathe's top that holds angle phi, at depth d in from
        the back: the point on the facet's chord, the facet's own yaw and the top's slope."""
        k = min(steps - 1, max(0, int((phi + half) / facet)))
        a = -half + k * facet
        y = 0.75 + (0.60 - d) / 0.46 * 0.10
        pa = Vector((math.sin(a) * (r_back - d), y, r_back - math.cos(a) * (r_back - d)))
        pb = Vector((math.sin(a + facet) * (r_back - d), y, r_back - math.cos(a + facet) * (r_back - d)))
        return frame(tuple(pa.lerp(pb, (phi - a) / facet)), 90.0 - slope, -math.degrees(a + facet / 2))
    p.union(body, "controls", throttle(p, on_top(-0.362, 0.37), "throttle") + stick(p, on_top(-0.0855, 0.42), "stick")
            + guarded_button(p, on_top(0.3355, 0.36), "fire_0") + guarded_button(p, on_top(0.3885, 0.36), "fire_1"))
    p.controls += ["helm (seat 0): a throttle lever left of its panel, a stick right of it",
                   "tactical (seat 1): two guarded fire buttons right of its panel"]
    screens = []
    for i, s in enumerate((-1, 1)):
        phi = s * 0.6 / r_mid
        d = 0.36
        y = 0.75 + (0.60 - d) / 0.46 * 0.10
        c = (math.sin(phi) * (r_back - d), y, r_back - math.cos(phi) * (r_back - d))
        p.recess(screens, f"panel_{i}", frame(c, 90.0 - slope, -math.degrees(phi)), 0.56, 0.24, 0.02, shows="keys")
        r_op = r_mid - 0.65
        p.operators.append([round(math.sin(phi) * r_op, 4), 0.0, round(r_back - math.cos(phi) * r_op, 4)])
    # the viewer's screen on the hood's sloped face, which runs from (z 0.44, y 0.76) to (0.32, 1.02)
    hood_tilt = math.degrees(math.atan2(0.12, 0.26))
    yv = 0.92
    p.recess(screens, "viewer", frame((0.0, yv, 0.44 - 0.12 * (yv - 0.76) / 0.26), hood_tilt), 0.22, 0.12, 0.015,
             shows="upper", half=0)
    p.cut(body, "screens", screens)
    p.body = body
    helm_arc_decor(p, r_back, half, steps, r_mid)
    return p


# ----------------------------------------------------------------------------- chairs
# The owner, 2026-10-07, on the captain's chair: "chairs suck still mainly its a texture problem",
# with the references (the shape taken, never the art, CLAUDE.md 15): a TNG captain's chair (a tall
# padded back split by a vertical slot, a padded headrest, wing bolsters, a seat with a rolled front,
# armrests on angled metal supports with console tops, a pedestal on a base plate), a burgundy one
# (channelled leather, a louvred pedestal on a base, angled arm consoles on bent supports) and Bridge
# Commander's grey chairs on pedestals. The cushions take the upholstery layers: `upholstery` (rolled
# channels) on seat and back faces, `upholstery_panel` (padded panels) on bolsters, headrests, wings,
# arm pads and the back's leather; frames and supports `trim`; pedestals, bases and consoles
# `machinery`; a thin `accent` stripe the page tints with the station's colour.
#
# Both keep today's placement exactly: the origin on the floor at the centre of the pedestal's back
# (the column's flat back, or the louvred pedestal's back face, at z = 0), the seat's top 0.45 m up,
# its centre over operators_m [0, 0, az] with az = col_r cos 30 degrees (0.0606 m for the captain,
# 0.0476 m for a crew chair, the values the variants and the command suite place them by).

SEAT_Y = 0.45      # the cushion's top: seat_m


def facing(m):
    """The prop-space direction of a frame's local +Z (a face's normal)."""
    return (m.to_3x3() @ Vector((0.0, 0.0, 1.0))).normalized()


def front_edges(m, lo=0.99):
    """A chamfer predicate: the edges round a face whose normal is frame m's +Z (a pad's front)."""
    f = facing(m)
    return lambda mid, d, n1, n2: max(n1.dot(f), n2.dot(f)) > lo and min(n1.dot(f), n2.dot(f)) < 0.5


def swept(p, what, m, x0, x1, profile, role):
    """A convex profile in frame m's (z, y) swept along its x from x0 to x1, as a hull (one role):
    a rolled cushion edge, a headrest. A list of (x, profile) pairs instead tapers it."""
    pts = [(x, y, z) for x in (x0, x1) for z, y in profile]
    return hull(p.coll, f"{p.name}.{what}", in_frame(m, pts), role)


def seat_parts(p, az, half_w, z_back, z_front, bolster):
    """A seat on its pan: the pan (trim) 5 cm deep, the cushion (channels on top, padded panels on
    its sides) whose top is SEAT_Y, a rolled front 1.5 cm prouder, and with bolster > 0 side
    bolsters that wide, 5 cm higher than the cushion. Returns the pieces."""
    n = p.name
    inner = half_w - bolster
    pan = obox(p.coll, f"{n}.seat_pan", (-half_w - 0.02, 0.33, z_back - 0.02), (half_w + 0.02, 0.38, z_front + 0.01), Matrix(),
               {"*": "trim"})
    p.chamfer(pan, "pan_edges", 0.015, lambda mid, d, n1, n2: mid.y < 0.335 and abs(d.x) > 0.9)
    roll_z = z_front - 0.115
    cushion = obox(p.coll, f"{n}.seat_cushion", (-inner - 0.01, 0.37, z_back), (inner + 0.01, SEAT_Y, roll_z + 0.02), Matrix(),
                   {"+y": "upholstery", "*": "upholstery_panel"})
    roll = swept(p, "seat_roll", Matrix(), -inner - 0.005, inner + 0.005,
                 [(roll_z, 0.37), (z_front - 0.03, 0.355), (z_front, 0.385), (z_front + 0.004, 0.425), (z_front - 0.02, 0.458),
                  (z_front - 0.06, SEAT_Y + 0.015), (roll_z, SEAT_Y + 0.015)], "upholstery_panel")
    parts = [pan, cushion, roll]
    if bolster:
        for s in (-1, 1):
            xa, xb = inner - 0.012, half_w
            prof = [(xa, 0.37), (xb, 0.37), (xb, SEAT_Y + 0.03), (xb - 0.02, SEAT_Y + 0.05), (xa + 0.025, SEAT_Y + 0.05),
                    (xa, SEAT_Y + 0.025)]
            pts = [(s * x, y, z) for z in (z_back, z_front - 0.03) for x, y in prof]
            b = hull(p.coll, f"{n}.bolster_{s:+d}", pts, "upholstery_panel")
            p.chamfer(b, f"bolster_{s:+d}_front", 0.02, lambda mid, d, n1, n2: mid.z > z_front - 0.04 and mid.y > SEAT_Y)
            parts.append(b)
    return parts


def captain_chair(name, presents):
    """The captain's chair (the TNG chair's family, in low poly): a louvred pedestal on a chamfered
    base plate; a seat pan, a cushion in channels with a rolled front and side bolsters; a tall back
    leaning 11 degrees, two channelled pads either side of a vertical slot that runs through the back,
    wing bolsters and a padded headrest with a stripe in the station's colour; armrests on angled
    supports, each with a padded rest at the back and a console head with its key panel at the front."""
    p = Prop(name, presents, "floor, centre of the pedestal's back")
    p.wall = False
    col_r = 0.07                               # today's column: the seat point stays where it was
    az = col_r * math.cos(math.pi / 6)
    # the base plate and the louvred pedestal, whose back face is the origin's plane (z = 0)
    zc = 0.12
    base = prism(p.coll, f"{name}.base", [(-0.30, zc - 0.20), (-0.22, zc - 0.28), (0.22, zc - 0.28), (0.30, zc - 0.20),
                                          (0.30, zc + 0.20), (0.22, zc + 0.28), (-0.22, zc + 0.28), (-0.30, zc + 0.20)],
                 "y", 0.0, 0.045, "machinery")
    p.chamfer(base, "base_edges", 0.02, lambda m, d, n1, n2: m.y > 0.03)
    ped = hull(p.coll, f"{name}.pedestal", [(sx * 0.17, 0.03, z) for sx in (-1, 1) for z in (0.0, 0.25)]
               + [(sx * 0.14, 0.345, z) for sx in (-1, 1) for z in (0.0, 0.20)], "machinery")
    louvres = []
    for k in range(4):
        # a sawtooth louvre: a shadowed lip over a slope that catches the light, across the front and
        # down both sides (the pedestal tapers, so each cutter follows its face)
        y = 0.12 + 0.06 * k
        zf = 0.25 - (y - 0.03) * 0.05 / 0.315
        xs = 0.17 - (y - 0.03) * 0.03 / 0.315
        louvres.append(prism(p.coll, f"{name}.louvre_front_{k}", [(zf + 0.05, y), (zf - 0.014, y), (zf + 0.05, y - 0.034)], "x",
                             -0.13, 0.13, "machinery"))
        for s in (-1, 1):
            tri = [(s * (xs + 0.05), y), (s * (xs - 0.014), y), (s * (xs + 0.05), y - 0.034)]
            louvres.append(hull(p.coll, f"{name}.louvre_side_{k}_{s:+d}", [(x, yy, z) for x, yy in tri for z in (0.035, zf - 0.035)],
                                "machinery"))
    p.cut(ped, "louvres", louvres)
    parts = [ped]
    # the seat
    z_back, z_front = az - 0.235, az + 0.275
    parts += seat_parts(p, az, 0.30, z_back, z_front, 0.085)
    # the back: a frame on the cushion's back edge, leaning 11 degrees
    m_b = frame((0.0, 0.385, z_back - 0.005), 11.0)
    shell = obox(p.coll, f"{name}.back_shell", (-0.27, -0.02, -0.10), (0.27, 0.84, -0.025), m_b, {"*": "upholstery_panel"})
    up_b = m_b.to_3x3() @ Vector((0.0, 1.0, 0.0))
    p.chamfer(shell, "shell_top", 0.02, lambda m, d, n1, n2: min(n1.dot(facing(m_b)), n2.dot(facing(m_b))) < -0.9
              and (m - m_b.to_translation()).dot(up_b) > 0.8)
    p.cut(shell, "slot", [obox(p.coll, f"{name}.slot_cut", (-0.035, 0.13, -0.3), (0.035, 0.60, 0.3), m_b, {"*": "trim"})])
    for s in (-1, 1):
        xa, xb = sorted((s * 0.042, s * 0.205))
        pad = obox(p.coll, f"{name}.back_pad_{s:+d}", (xa, 0.05, -0.04), (xb, 0.74, 0.035), m_b,
                   {"+z": "upholstery", "*": "upholstery_panel"})
        p.chamfer(pad, f"back_pad_{s:+d}_edges", 0.018, front_edges(m_b))
        parts.append(pad)
        wing = [(s * x, z) for x, z in ((0.195, -0.09), (0.28, -0.09), (0.29, -0.01), (0.275, 0.055), (0.24, 0.085), (0.205, 0.07))]
        top = [(s * x, z) for x, z in ((0.195, -0.09), (0.28, -0.09), (0.285, -0.02), (0.27, 0.02), (0.235, 0.035), (0.205, 0.03))]
        parts.append(hull(p.coll, f"{name}.wing_{s:+d}", in_frame(m_b, [(x, 0.0, z) for x, z in wing] + [(x, 0.79, z) for x, z in top]),
                          "upholstery_panel"))
    parts.append(shell)
    head = swept(p, "headrest", m_b, -0.165, 0.165,
                 [(-0.08, 0.79), (0.02, 0.79), (0.06, 0.82), (0.07, 0.905), (0.05, 0.965), (-0.06, 0.965), (-0.085, 0.93)],
                 "upholstery_panel")
    band = obox(p.coll, f"{name}.headrest_stripe", (-0.12, 0.862, 0.06), (0.12, 0.884, 0.078), m_b, {"*": "accent"})
    parts += [head, band]
    # the arms: an angled support from the pan, the rest and the console head
    screens = []
    zr, zfa = az - 0.20, az + 0.31
    for s in (-1, 1):
        xa, xb = sorted((s * 0.315, s * 0.445))
        arm = prism(p.coll, f"{name}.arm_{s:+d}", [(zr, 0.585), (zfa - 0.05, 0.585), (zfa, 0.615), (zfa, 0.64), (zfa - 0.025, 0.662),
                                                   (az - 0.03, 0.675), (az - 0.03, 0.62), (zr, 0.62)],
                    "x", xa, xb, ["machinery", "machinery", "machinery", "trim", "trim", "machinery", "machinery", "machinery"],
                    cap="machinery")
        p.chamfer(arm, f"arm_{s:+d}_edges", 0.012, lambda m, d, n1, n2: m.y > 0.6 and abs(d.x) > 0.9 and m.z > az)
        sa, sb = sorted((s * 0.31, s * 0.355))
        sup = prism(p.coll, f"{name}.arm_support_{s:+d}", [(az - 0.17, 0.335), (az + 0.04, 0.335), (az + 0.20, 0.595),
                                                           (az + 0.08, 0.595), (az - 0.06, 0.395), (az - 0.17, 0.395)],
                    "x", sa, sb, "trim")
        rest = obox(p.coll, f"{name}.arm_rest_{s:+d}", (xa + 0.012, 0.612, zr + 0.012), (xb - 0.012, 0.652, az - 0.02), Matrix(),
                    {"*": "upholstery_panel"})
        p.chamfer(rest, f"arm_rest_{s:+d}_edges", 0.012, lambda m, d, n1, n2: m.y > 0.64)
        parts += [arm, sup, rest]
        a0, a1 = (az - 0.03, 0.675), (zfa - 0.025, 0.662)
        tilt = 90.0 - math.degrees(math.atan2(a0[1] - a1[1], a1[0] - a0[0]))
        c = (s * 0.38, (a0[1] + a1[1]) / 2, (a0[0] + a1[0]) / 2)
        p.recess(screens, f"arm_panel_{s:+d}", frame(c, tilt), 0.10, 0.24, 0.012, shows="keys")
    p.union(base, "chair", parts)
    p.cut(base, "screens", screens)
    p.body = base
    chair_decor(p, True, az)
    p.seat = [0.0, SEAT_Y, round(az, 4)]
    p.operators.append([0.0, 0.0, round(az, 4)])      # the floor under the seat
    return p


def crew_chair(name, presents):
    """A station's swivel chair, the captain's family made simpler: a hexagonal column (its flat back
    at z = 0) with a collar on an octagonal base; a seat pan, a channelled cushion with a rolled
    front; a padded back leaning 10 degrees in channels, its leather shell, a stripe in the station's
    colour under a padded headrest; short arms, a padded rest on a post either side."""
    p = Prop(name, presents, "floor, centre of the pedestal's back")
    p.wall = False
    col_r = 0.055
    az = col_r * math.cos(math.pi / 6)        # the column's flat back is at z = 0
    base = prism(p.coll, f"{name}.base", ngon(0.0, az, 0.26, 8), "y", 0.0, 0.04, "machinery")
    p.chamfer(base, "base_edges", 0.018, lambda m, d, n1, n2: m.y > 0.03)
    column = prism(p.coll, f"{name}.column", ngon(0.0, az, col_r, 6), "y", 0.03, 0.345, "machinery")
    collar = prism(p.coll, f"{name}.collar", ngon(0.0, az, 0.085, 6, flat_back=False), "y", 0.27, 0.34, "trim")
    parts = [column, collar]
    z_back, z_front = az - 0.225, az + 0.255
    parts += seat_parts(p, az, 0.235, z_back, z_front, 0.0)
    m_b = frame((0.0, 0.39, z_back - 0.005), 10.0)
    shell = obox(p.coll, f"{name}.back_shell", (-0.225, -0.02, -0.075), (0.225, 0.60, -0.02), m_b, {"*": "upholstery_panel"})
    pad = obox(p.coll, f"{name}.back_pad", (-0.195, 0.05, -0.03), (0.195, 0.555, 0.03), m_b,
               {"+z": "upholstery", "*": "upholstery_panel"})
    p.chamfer(pad, "back_pad_edges", 0.016, front_edges(m_b))
    stripe = obox(p.coll, f"{name}.back_stripe", (-0.226, 0.565, -0.076), (0.226, 0.585, -0.019), m_b, {"*": "accent"})
    head = swept(p, "headrest", m_b, -0.14, 0.14,
                 [(-0.065, 0.59), (0.01, 0.59), (0.045, 0.615), (0.05, 0.685), (0.03, 0.725), (-0.05, 0.725), (-0.07, 0.70)],
                 "upholstery_panel")
    parts += [shell, pad, stripe, head]
    for s in (-1, 1):
        xa, xb = sorted((s * 0.235, s * 0.29))
        post = obox(p.coll, f"{name}.arm_post_{s:+d}", (min(xa, xb) + 0.012, 0.36, az - 0.09), (max(xa, xb) - 0.012, 0.585, az - 0.02),
                    Matrix(), {"*": "trim"})
        rest = obox(p.coll, f"{name}.arm_rest_{s:+d}", (xa, 0.575, az - 0.16), (xb, 0.615, az + 0.11), Matrix(),
                    {"*": "upholstery_panel"})
        p.chamfer(rest, f"arm_rest_{s:+d}_edges", 0.012, lambda m, d, n1, n2: m.y > 0.6 and abs(d.x) > 0.9)
        parts += [post, rest]
    p.union(base, "chair", parts)
    p.body = base
    chair_decor(p, False, az)
    p.seat = [0.0, SEAT_Y, round(az, 4)]
    p.operators.append([0.0, 0.0, round(az, 4)])      # the floor under the seat
    return p


def standup_console(name, presents):
    """A pedestal stand-up console: a base plate, a column leaning toward the operator and a
    head whose sloped top (about 1.1 m) holds one screen."""
    p = Prop(name, presents, "floor, centre of the pedestal's back")
    p.wall = False
    zb, zf = -0.04, 0.42
    yb, yf = 1.16, 1.04
    top = lambda z: yf + (zf - z) / (zf - zb) * (yb - yf)
    head = prism(p.coll, f"{name}.head", [(zb, 0.86), (zf, 0.86), (zf, 1.25), (zb, 1.25)], "x", -0.32, 0.32, "machinery")
    p.cut(head, "carve", [
        prism(p.coll, f"{name}.cut_top", [(zf + 0.05, top(zf + 0.05)), (zb - 0.05, top(zb - 0.05)), (zb - 0.05, 1.4),
                                          (zf + 0.05, 1.4)], "x", -0.37, 0.37, "trim"),
        prism(p.coll, f"{name}.cut_under", [(zf + 0.05, 1.01), (zf - 0.14, 0.835), (zf + 0.05, 0.835)], "x",
              -0.37, 0.37, "machinery"),
    ])
    p.chamfer(head, "top_edges", 0.015, lambda m, d, n1, n2: max(n1.y, n2.y) > 0.9)
    column = hull(p.coll, f"{name}.column", [(sx * 0.11, 0.03, z) for sx in (-1, 1) for z in (0.0, 0.18)]
                  + [(sx * 0.15, 0.90, z) for sx in (-1, 1) for z in (0.06, 0.24)], "bulkhead")
    base = prism(p.coll, f"{name}.base", [(-0.10, 0.0), (0.32, 0.0), (0.32, 0.05), (-0.10, 0.05)], "x", -0.25, 0.25, "trim")
    p.chamfer(base, "base_edges", 0.02, lambda m, d, n1, n2: m.y > 0.03)
    p.union(head, "column_base", [column, base])
    zt = (zb + zf) / 2 + 0.02
    slope = math.degrees(math.atan2(yb - yf, zf - zb))
    screens = []
    p.recess(screens, "screen", frame((0.0, top(zt), zt), 90.0 - slope), 0.50, 0.30, 0.015, shows="console")
    p.cut(head, "screens", screens)
    p.body = head
    standup_decor(p, top, zb, zf)
    p.operators.append([0.0, 0.0, round(zf + 0.35, 4)])
    return p


def variant(p, base, stations):
    """Mark a prop as a station's variant of base: the variants file (tools/bridge_variants.py)
    and the mockup place it wherever base would stand at one of these stations."""
    p.variant_of, p.stations = base, list(stations)
    return p


PROPS = {
    "wall_bank_core": lambda: wall_bank("wall_bank_core", 1.40, 1,
                                        "A core station built into a wall (bridge-stations 11.1: desk 1.40 m)"),
    "wall_bank_core_engineering": lambda: variant(wall_bank("wall_bank_core_engineering", 1.40, 1,
                                                            "Engineering's core bank, with its breaker toggles", "breakers"),
                                                  "wall_bank_core", ["engineering"]),
    "wall_bank_core_science": lambda: variant(wall_bank("wall_bank_core_science", 1.40, 1,
                                                        "Science's core bank, with its sensor tuning dials", "dials"),
                                              "wall_bank_core", ["science"]),
    "wall_bank": lambda: wall_bank("wall_bank", 1.10, 1,
                                   "A station built into a wall (bridge-stations 11.1: desk 1.10 m)"),
    "wall_bank_comms": lambda: variant(wall_bank("wall_bank_comms", 1.10, 1, "Comms' bank, with its slider bank", "sliders"),
                                       "wall_bank", ["comms"]),
    "wall_bank_flight_ops": lambda: variant(wall_bank("wall_bank_flight_ops", 1.10, 1,
                                                      "Flight ops' bank, with its launch lever and abort button", "levers"),
                                            "wall_bank", ["flight_ops"]),
    "wall_bank_double": lambda: wall_bank("wall_bank_double", 2.40, 2, "Two seats in one wall bay"),
    "free_console": lambda: free_console("free_console", "The free-standing helm or tactical desk (desk 1.40 m)"),
    "free_console_helm": lambda: variant(free_console("free_console_helm", "Helm's free-standing desk, with its throttle and stick",
                                                      "helm"), "free_console", ["helm"]),
    "free_console_tactical": lambda: variant(free_console("free_console_tactical",
                                                          "Tactical's free-standing desk, with its guarded fire buttons", "tactical"),
                                             "free_console", ["tactical"]),
    "helm_arc": lambda: helm_arc("helm_arc", "A curved two-seat helm, the first series' style"),
    "captain_chair": lambda: captain_chair("captain_chair", "The captain's chair on the dais"),
    "crew_chair": lambda: crew_chair("crew_chair", "A station's swivel chair"),
    "standup_console": lambda: standup_console("standup_console", "A pedestal stand-up console"),
}


# ----------------------------------------------------------------------------- main

STATUS = ("Built (2026-10-05) by " + GENERATOR + ". Proposed furniture for the bridge "
          "(bridge-stations section 11.1, deck-pipeline section 5). docs/mockups/bridge-variants.html shows them "
          "(tools/mockups/inline.py models:bridge copies them in); no engine code loads them yet.")
RULES = [
    "This file is written by " + GENERATOR + "; never edit it by hand. Rebuild, and the .glb files and this file change together.",
    "Metres. Prop space is the glTF frame: +Y up, +Z toward the operator (out of a console's working face; for a chair, the way the sitter faces), +X the operator's right as they face the prop.",
    "The origin is on the floor at the centre of the prop's back: the wall plane for a wall bank, the back of the pedestal otherwise.",
    "Placing props at a layout station (seat_m on the floor, yaw_deg, the seat facing +Z at yaw 0): operators_m are floor points, so the layout's seat_m lands on one. A chair turns by yaw_deg (its operators_m is the floor under its seat); a console faces the seat, so it turns by yaw_deg + 180. seat_m in a chair's row is the sitting point on the cushion, 0.45 m up.",
    "Materials are roles: machinery, trim, bulkhead, hazard and light_panel are data/materials/materials.json layers; screen is emissive and coloured by the page (its console UI); accent is the station's role colour; upholstery and upholstery_panel are the panel layers upholstery_channel and upholstery_panel (data/materials/panels.json upholstery), neutral leather the page tints per chair (upholstery.tints_srgb).",
    "screens are the recess floors a page draws on: centre_m on the floor, normal out of it, up the in-plane direction of height_m (width_m runs along up x normal). shows says what: console (the station's main screen image), upper (half 0 or 1 of its upper image) or keys (a key panel); the images and how to fit them are assets/textures/screens/screens.json (bridge-stations 11.6).",
    "A row with variant_of is that prop with a station's own hand controls (controls says which): it stands wherever variant_of would at one of its stations, with the same operators_m.",
    "UV0 is in metres, projected per face as shipkit.js worldUv does (x, z where |n.y| > 0.75, else the face's horizontal tangent and y); divide by the material's span_m.",
    "Flat shaded (one normal per face), triangulated, one closed manifold solid per prop; faces against the floor or the wall are kept for the deck compiler to drop.",
    "triangles is counted in the .glb; the build refuses a prop over budget_triangles. sha256 is the .glb's, from the Blender and exporter versions in generator: a second build with them writes the same bytes.",
    "atlas is the prop's own baked texture (assets/models/<set>/atlas/<prop>.png, written by the build with its sha256): 256 x 256 RGBA, alpha the glow mask, read through the glb's second UV map (TEXCOORD_1; TEXCOORD_0 stays the metre UVs). px_per_m is its texel density, charts how many pieces its surface was cut into. data/materials/prop_atlas.json says how it is baked; screen faces carry no content in it (the page draws the console faces), accent faces bake light and neutral for the page to tint.",
]

BRIDGE = PropSet("bridge", "BridgeProps", OUT, GENERATOR, PROPS, BUDGETS, STATUS, RULES, __doc__)


if __name__ == "__main__":
    run(BRIDGE)
