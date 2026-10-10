"""Star Crew's service panel prototypes (openspec/changes/repairs-on-deck, design 3c): two engineering
machines with their service bay built into their meshes, on a side or the back where a real machine has
one, and the cover and screw that close it as props of their own.

The owner, 2026-10-09: "The service panel should not be in front of the station like that. It should be
on the side of a machine or found on the back if it's accessible", "Did you integrate service panel 3d
into the geometry", and "do some quick prototypes rather than apply it to every object". So this is a
prototype set, assets/models/service_proto/, and the shipped engineering props are untouched.

It owns assets/models/service_proto/<name>.glb, their atlases, props.json and service.json. Everything is
imported, never copied (CLAUDE.md 6.1): the machines are the engineering set's own builders
(tools/blender/build_engineering_props.py), changed only by the bay; the build, check, atlas, export and
manifest are the hard-surface kit's (tools/blender/hs_kit.py run()).

  coolant_pump_svc  the coolant pump with a terminal box on its motor's +X flank (the walkway side, worked
                    standing), its lid's bay cut into the box: a rebate the cover sits flush in, four tapped
                    holes in it, and the bay behind, whose floor shows the wiring (shows interior_wiring)
  local_panel_svc   the local control cabinet with a door in its back (-Z), its operating face (the sloped
                    instruments, +Z) untouched; the bay's floor shows the circuit board (interior_circuit)
  cover_pump, cover_panel  each bay's cover: a plate the rebate's size and depth, standing on its bottom edge
                    (x across, y up from 0, its back on z = 0, its front at z = thickness); the page lays the
                    cover bake of design 3b on its front
  service_screw     one screw standing on its shank's tip: the shank up to y = SCREW_SHANK_M, a pan head
                    above, its slot painted (the page turns +Y to the bay's normal)

service.json says, per machine in its prop space, where the cover sits (its centre, normal and up, at
home in the rebate), its size and thickness, the screws' points and the bay's interior. The page places
the cover and screws from it, never by hand.

Run:
  <python with the bpy module> tools/blender/build_service_prototypes.py [--check] [--only a,b]
"""
import json
import os
import sys

import bpy  # noqa: F401  first: with the pip bpy module, mathutils exists only once bpy is imported
from mathutils import Matrix, Vector  # noqa: E402

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import build_engineering_props as eng  # noqa: E402
from hs_kit import ROOT, SHOWS, Prop, PropSet, frame, r3, run  # noqa: E402
from build_engineering_props import regions, whole  # noqa: E402
from build_machinery_props import pipe, revolve  # noqa: E402
import math  # noqa: E402

OUT = os.path.join(ROOT, "assets", "models", "service_proto")
GENERATOR = "tools/blender/build_service_prototypes.py"
COVERS = os.path.join(ROOT, "assets", "textures", "repairs", "covers.json")
INTERIORS = ("interior_wiring", "interior_circuit", "board_circuit")

# The bays (prop space of their machine). frame: the bay's face, local +Z out of the machine, local x across
# the cover, y up it. size_m: the cover (and the rebate it sits in); opening_m: the hole behind; depth_m: the bay.
BAYS = {
    "coolant_pump_svc": {"centre": (0.62, 1.92, 0.0), "yaw": 90.0, "size_m": (0.38, 0.32), "opening_m": (0.31, 0.25),
                         "depth_m": 0.12, "shows": None, "cover": "cover_pump", "fault": "K1", "game": "pump"},
    "local_panel_svc": {"centre": (0.0, 0.85, 0.0), "yaw": 180.0, "size_m": (0.56, 0.455), "opening_m": (0.49, 0.385),
                        "depth_m": 0.12, "shows": "board_circuit", "cover": "cover_panel", "fault": "C2", "game": "coolant"},
}
COVER_T = 0.012       # the cover's thickness, and the rebate's depth: it sits flush
HOLE_R = 0.0042       # a tapped hole in the rebate, under each screw

SCREW_SHANK_M = 0.03   # a screw's shank: the head's underside is this far up its prop space
BUDGETS = {"coolant_pump_svc": 1900, "local_panel_svc": 1300, "cover_pump": 40, "cover_panel": 40, "service_screw": 90}


def hole_points(bay):
    """The screws' points on the cover, local (x, y), from the cover bake's hole fractions (design 3b)."""
    with open(COVERS, encoding="utf-8") as f:
        uvs = json.load(f)["covers"]["working"]["holes_uv"]
    w, h = bay["size_m"]
    return [((u - 0.5) * w, (0.5 - v) * h) for u, v in uvs]


def cut_bay(p, bay, label):
    """Cut a bay into p's body: the rebate (the cover's size, COVER_T deep, bare metal), the opening behind it
    (machinery walls, its floor a recorded face showing the interior) and a tapped hole under each screw."""
    m = frame(bay["centre"], 0.0, bay["yaw"])
    p.shows = tuple(SHOWS) + INTERIORS
    w, h = bay["size_m"]
    ow, oh = bay["opening_m"]
    cuts = []
    p.recess(cuts, f"{label}_rebate", m, w, h, COVER_T, wall_role="trim", floor_role="trim", record=False)
    # The bay's floor: a recorded face carrying a bake where the hardware needs finer detail than the atlas holds (the
    # bare board, design 3d), else the plain mounting plate the hardware stands on.
    p.recess(cuts, f"{label}_bay", m @ Matrix.Translation((0.0, 0.0, -COVER_T)), ow, oh, bay["depth_m"],
             wall_role="machinery", floor_role="screen" if bay["shows"] else "bulkhead", record=bool(bay["shows"]),
             shows=bay["shows"])
    for k, (x, y) in enumerate(hole_points(bay)):
        cuts.append(p.box(f"{label}_tap_{k}", (-HOLE_R, -HOLE_R, -COVER_T - 0.03), (HOLE_R, HOLE_R, 0.02), "machinery",
                          m=m @ Matrix.Translation((x, y, 0.0))))
    p.cut(p.body, f"{label}_bay", cuts)


# ----------------------------------------------------------------------------- the hardware in the bays (design 3d)
# A piece: kind, id, label, its box on the bay floor (c its centre and s its size across and up the bay, metres), z its
# height out of the floor, its finish (prop_atlas.json), and whether a click names it. Built into the machine's mesh; its
# box goes to service.json as a hit target. The bay floor's frame: +Z out of the machine.

def pump_hardware():
    """The motor terminal box: a DIN rail of 12 terminal blocks (two blue neutrals, a yellow earth), the relay K1, the
    contactor KM1 and three fuses F1-F3, with wires up from the blocks into the box's top and from the switchgear in."""
    out = [{"kind": "rail", "id": "rail", "label": "DIN rail", "c": (0.0, 0.0325), "s": (0.29, 0.035), "z": 0.007, "finish": "steel",
            "click": False}]
    blocks = []
    for k in range(12):
        x = -0.118 + k * 0.0215
        fin = "blue" if k in (4, 9) else "yellow" if k == 11 else "housing"
        out.append({"kind": "block", "id": f"X1_{k + 1}", "label": "Terminal strip X1", "c": (x, 0.0325), "s": (0.019, 0.075), "z": 0.045,
                    "finish": fin, "click": False})
        blocks.append(x)
    out.append({"kind": "group", "id": "X1", "label": "Terminal strip X1", "c": (0.0, 0.0325), "s": (0.26, 0.075), "z": 0.045,
                "finish": None, "click": True})
    out.append({"kind": "relay", "id": "K1", "label": "Relay K1", "c": (-0.105, -0.085), "s": (0.05, 0.06), "z": 0.055, "finish": "dark",
                "click": True})
    out.append({"kind": "box", "id": "KM1", "label": "Contactor KM1", "c": (-0.035, -0.085), "s": (0.065, 0.065), "z": 0.06,
                "finish": "housing", "click": True})
    for k, x in enumerate((0.045, 0.08, 0.115)):
        out.append({"kind": "fuse", "id": f"F{k + 1}", "label": f"Fuse F{k + 1}", "c": (x, -0.085), "s": (0.026, 0.07), "z": 0.022,
                    "finish": "white", "click": True})
    finishes = ("rubber", "red", "blue", "yellow")
    wires = [([(x, 0.07, 0.03), (x, 0.1, 0.03), (x * 0.9, 0.135, 0.02)], finishes[k % 4]) for k, x in enumerate(blocks)]
    # The switchgear's wires drop straight out of the box's bottom edge, clear of every top (no wire runs over a part).
    for k, sx in enumerate((-0.12, -0.09, -0.05, -0.02)):
        wires.append(([(sx, -0.11, 0.025), (sx, -0.135, 0.025)], finishes[(k + 1) % 4]))
    return out, wires


def board_hardware(bay):
    """The cabinet's board: the parts the bare board bake recorded (assets/textures/repairs/covers.json boards), placed
    on the bay floor at the bake's scale, so they stand on their own pads and outlines."""
    with open(COVERS, encoding="utf-8") as f:
        board = json.load(f)["boards"]["circuit"]
    (fw, fh), (ow, oh) = board["frame_m"], bay["opening_m"]
    sx, sy = ow / fw, oh / fh
    out = []
    for q in board["parts"]:
        out.append(dict({"kind": q["kind"], "id": q["id"], "label": q["label"], "c": (q["x_m"] * sx, q["y_m"] * sy),
                         "s": (q["w_m"] * sx, q["h_m"] * sy), "z": q["height_m"], "finish": q["finish"], "click": q["click"]},
                        **{k: q[k] for k in ("fins", "wires") if k in q}))
    return out, []


def _seg_dist(p_, a, b):
    """A point's distance from the segment a-b."""
    ab = b - a
    t = max(0.0, min(1.0, (p_ - a).dot(ab) / max(ab.dot(ab), 1e-12)))
    return (a + ab * t - p_).length


def build_hardware(p, bay, label, parts, wires):
    """Model the pieces on the bay floor, union them into the body, paint each in its finish, and return their boxes."""
    mf = frame(bay["centre"], 0.0, bay["yaw"]) @ Matrix.Translation((0.0, 0.0, -COVER_T - bay["depth_m"]))
    out_n = (mf.to_3x3() @ Vector((0, 0, 1))).normalized()
    floor_d = out_n.dot(mf.translation)
    fault = bay["fault"]
    objs, boxes = [], []
    for q in parts:
        (cx, cy), (w, h), z = q["c"], q["s"], q["z"]
        at = mf @ Matrix.Translation((cx, cy, 0.0))
        k, nm = q["kind"], f"{label}_{q['id']}"
        if k == "can":
            r = min(w, h) / 2
            prof = [(r, -0.001), (r, z), (r * 0.8, z + 0.006)] if q["id"] == fault else [(r, -0.001), (r, z)]   # a failed can bulges
            objs.append(eng.turned(p, nm, at, prof, "trim", sides=10, caps=("trim", "trim")))
        elif k == "screw":
            objs.append(eng.turned(p, nm, at, [(w / 2, -0.001), (w / 2, z)], "trim", sides=8, caps=("trim", "trim")))
        elif k == "heatsink":
            objs.append(p.box(nm + "_base", (-w / 2, -h / 2, -0.001), (w / 2, h / 2, 0.004), "trim", m=at))
            # Fins 3.6 mm thick at least 1 cm apart (CLAUDE.md 8: deliberately parallel faces 1 cm apart), as many as fit.
            n = max(2, min(q.get("fins", 8), int((w - 0.006) / 0.0136) + 1))
            for j in range(n):
                fx = -w / 2 + 0.003 + j * (w - 0.006) / (n - 1)
                objs.append(p.box(f"{nm}_fin{j}", (fx - 0.0018, -h / 2, 0.003), (fx + 0.0018, h / 2, z), "trim", m=at))
        elif k == "fuse":
            objs.append(p.box(nm + "_holder", (-w / 2, -h / 2, -0.001), (w / 2, h / 2, z), "trim", m=at))
            cart = at @ Matrix.Translation((0.0, -h * 0.36, z + 0.004)) @ Matrix.Rotation(-math.pi / 2, 4, "X")
            objs.append(eng.turned(p, nm + "_cartridge", cart, [(0.0085, 0.0), (0.0085, h * 0.72)], "trim", sides=8, caps=("trim", "trim")))
        elif k != "group":   # chip, box, relay, rail, block, header; a group is a hit target over pieces built on their own
            objs.append(p.box(nm, (-w / 2, -h / 2, -0.001), (w / 2, h / 2, z), "trim", m=at))
        if k == "header":
            nw = q.get("wires", 4)
            for j in range(nw):
                x0 = -w / 2 + w * (j + 0.5) / nw
                wires.append(([(cx + x0, cy, z - 0.002), (cx + x0, cy + 0.02, z + 0.004), (cx + x0 * 1.4, bay["opening_m"][1] / 2 + 0.01, z)],
                              ("rubber", "red")[j % 2]))
        corners = [at @ Vector((sx_ * w / 2, sy_ * h / 2, zz)) for sx_ in (-1, 1) for sy_ in (-1, 1) for zz in (0.0, z + 0.008)]
        boxes.append({"id": q["id"], "label": q["label"], "lo_m": r3([min(c[i] for c in corners) for i in range(3)]),
                      "hi_m": r3([max(c[i] for c in corners) for i in range(3)]), "finish": q["finish"], "click": q["click"],
                      "fault": q["id"] == fault, "kind": k, "local": {"c": list(q["c"]), "s": list(q["s"]), "z": z}})
    for j, (pts, fin) in enumerate(wires):
        path = [tuple(mf @ Vector(pt)) for pt in pts]
        objs.append(pipe(p, f"{label}_wire{j}", path, 0.0034, "trim", sides=4))
        boxes.append({"id": f"wire{j}", "label": "Wire", "path_m": [r3(v) for v in path], "finish": fin, "click": False,
                      "fault": False, "kind": "wire"})
    p.union(p.body, f"{label}_hardware", objs)

    def decor(D):
        """Each piece in its own finish, found by its box or its wire's path; the fault's scorch and its lit lamp."""
        for b in boxes:
            if b["kind"] == "group" or not b["finish"]:
                continue
            if b["kind"] == "wire":
                pts = [Vector(v) for v in b["path_m"]]
                for R in regions(D, lambda R, pts=pts: R.role == "trim" and min(_seg_dist(R.origin, a, c) for a, c in zip(pts, pts[1:])) < 0.006):
                    whole(D, R, b["finish"])
                continue
            lo, hi = Vector(b["lo_m"]) - Vector((0.0015,) * 3), Vector(b["hi_m"]) + Vector((0.0015,) * 3)
            for R in regions(D, lambda R, lo=lo, hi=hi: R.role == "trim" and all(lo[i] <= R.origin[i] <= hi[i] for i in range(3))):
                top = R.n.dot(out_n) > 0.99
                fin = b["finish"]
                if b["kind"] == "can" and top:
                    fin = "dark" if b["fault"] else "plate"      # an aluminium top, scorched on the failed one
                elif b["kind"] == "relay" and top and b["fault"]:
                    fin = "rubber"                               # the relay's housing burnt black on top
                elif b["kind"] == "box" and top:
                    fin = "steel"                                # the contactor's face plate
                elif b["kind"] == "fuse" and R.origin.dot(out_n) - floor_d > 0.023:
                    fin = "copper"                               # the cartridge, above its holder
                whole(D, R, fin)
            if b["fault"] and b["kind"] == "relay":             # the relay's fault lamp, lit
                c = b["local"]["c"]
                D.disc(mf @ Matrix.Translation((c[0] + 0.012, c[1] + 0.016, b["local"]["z"] + 0.0005)), 0.0, 0.0, 0.005, 0.0, 0.002,
                       "led_red", sides=10, reserve=False)
    p.decor.append(decor)
    return boxes


HARDWARE = {}   # each machine's hardware boxes, for service.json


def coolant_pump_svc():
    """The coolant pump, with a terminal box on the motor's +X flank in place of the small junction box (it
    covers it), and the box's lid bay cut into its +X face."""
    p = eng.coolant_pump()
    p.name = "coolant_pump_svc"
    box = p.box("terminal_box", (0.40, 1.72, -0.24), (0.62, 2.12, 0.24), {"+x": "bulkhead", "*": "machinery"})
    p.union(p.body, "terminal_box", [box])
    cut_bay(p, BAYS["coolant_pump_svc"], "lid")
    parts, wires = pump_hardware()
    HARDWARE["coolant_pump_svc"] = build_hardware(p, BAYS["coolant_pump_svc"], "lid", parts, wires)
    p.presents = "Prototype (repairs-on-deck 3c): the coolant pump with its motor terminal box's lid bay built in"
    return p


def local_panel_svc():
    """The local control cabinet with a service door bay in its back (-Z); the sloped instrument face is the
    operating face and is not touched."""
    p = eng.local_panel()
    p.name = "local_panel_svc"
    cut_bay(p, BAYS["local_panel_svc"], "door")
    parts, wires = board_hardware(BAYS["local_panel_svc"])
    HARDWARE["local_panel_svc"] = build_hardware(p, BAYS["local_panel_svc"], "door", parts, wires)
    p.presents = "Prototype (repairs-on-deck 3c): the local control cabinet with a service door bay in its back"
    return p


def cover(name, bay):
    """A cover: a plate the rebate's size, standing on its bottom edge, from z = 0 (its back, on the rebate floor)
    to COVER_T (its front)."""
    def build():
        w, h = bay["size_m"]
        p = Prop(name, f"Prototype (repairs-on-deck 3c): the service cover of the {bay['shows'][9:]} bay, {w} x {h} m",
                 "the middle of its bottom edge, at z = 0", back_at_z0=True)
        p.body = p.box("plate", (-w / 2, 0.0, 0.0), (w / 2, h, COVER_T), {"+z": "bulkhead", "*": "trim"})
        return p
    return build


def service_screw():
    """A pan-head screw standing on its shank's tip: the shank 5.6 mm across up to SCREW_SHANK_M, the head 14 mm
    across and 5 mm high above it, a slot painted across the head."""
    p = Prop("service_screw", "Prototype (repairs-on-deck 3c): a service cover's screw", "the shank's tip, on the floor")
    y0 = SCREW_SHANK_M
    head = revolve(p, "head", [(0.007, y0), (0.007, y0 + 0.0035), (0.0045, y0 + 0.005)], "trim", "y", (0.0, 0.0), sides=8,
                   caps=("trim", "trim"))
    shank = revolve(p, "shank", [(0.0028, 0.0), (0.0028, y0 + 0.001)], "trim", "y", (0.0, 0.0), sides=6, caps=("trim", "trim"))
    p.union(head, "shank", [shank])

    def decor(D):   # the slot, a dark line across the head's top: painted, as a 1.6 mm cut is too thin to model
        D.box(frame((0.0, y0 + 0.005, 0.0), 90.0), -0.0045, -0.0008, 0.0045, 0.0008, -0.0004, 0.0004, "stencil_dark", reserve=False)
    p.decor.append(decor)
    p.body = head
    return p


PROPS = {"coolant_pump_svc": coolant_pump_svc, "local_panel_svc": local_panel_svc,
         "cover_pump": cover("cover_pump", BAYS["coolant_pump_svc"]),
         "cover_panel": cover("cover_panel", BAYS["local_panel_svc"]), "service_screw": service_screw}


def service_json():
    """Where each machine's cover and screws sit, in the machine's prop space (the page reads this)."""
    out = {}
    for name, bay in BAYS.items():
        m = frame(bay["centre"], 0.0, bay["yaw"])
        n = (m.to_3x3() @ Vector((0, 0, 1))).normalized()
        up = (m.to_3x3() @ Vector((0, 1, 0))).normalized()
        home = m @ Vector((0.0, 0.0, -COVER_T))   # the cover's back on the rebate floor
        hw = [{k: b[k] for k in ("id", "label", "lo_m", "hi_m", "fault") if k in b} for b in HARDWARE.get(name, []) if b["click"]]
        out[name] = {"cover": bay["cover"], "shows": bay["shows"], "size_m": list(bay["size_m"]), "thickness_m": COVER_T,
                     "fault": bay["fault"], "game": bay["game"], "hardware": hw,
                     "cover_at_m": r3(home), "normal": r3(n), "up": r3(up),
                     "screws_m": [r3(m @ Vector((x, y, 0.0))) for x, y in hole_points(bay)],
                     "bay_depth_m": bay["depth_m"], "opening_m": list(bay["opening_m"])}
    return {"schema": "starcrew.service-bays/1", "built_by": GENERATOR,
            "_rules": ["Prop space of each machine (+Y up, +Z its front). cover_at_m is the centre of the cover's back when it is home "
                       "in the rebate, normal out of the machine, up the cover's up; the cover's front is thickness_m along normal. "
                       "screws_m are the screws' head undersides when driven home, on the face. shows is the bake the "
                       "bay's recorded floor carries (none: a plain plate). hardware is every piece a click names (design 3d): its "
                       "box in prop space; fault is the piece the job's repair starts from, game the 2D game it opens."],
            "machines": out}


STATUS = ("Prototype (2026-10-09) by " + GENERATOR + ", for the owner's review (repairs-on-deck design 3c). Not the shipped "
          "engineering props: those are untouched until the owner approves these.")
RULES = list(eng.RULES) + ["screens with shows interior_wiring, interior_circuit or board_circuit are service bay floors: the "
                           "page lays that bake (assets/textures/repairs) on them; the hardware on them is modelled (design 3d)."]
SET = PropSet("service_proto", "ServicePrototypes", OUT, GENERATOR, PROPS, BUDGETS, STATUS, RULES, __doc__)


if __name__ == "__main__":
    run(SET)
    if "--check" not in sys.argv:
        with open(os.path.join(OUT, "service.json"), "w", encoding="utf-8") as f:
            json.dump(service_json(), f, indent=2)
            f.write("\n")
        print("[props] wrote", os.path.relpath(os.path.join(OUT, "service.json"), ROOT))
