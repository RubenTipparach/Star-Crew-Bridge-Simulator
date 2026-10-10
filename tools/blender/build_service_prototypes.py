"""Star Crew's service panel prototypes (openspec/changes/repairs-on-deck, designs 3c-3e): two engineering
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
                    holes in it, and the bay behind
  local_panel_svc   the local control cabinet with a door in its back (-Z), its operating face (the sloped
                    instruments, +Z) untouched
  <machine>_hw      the bay's hardware on its mounting plate (design 3d, 3e): the pump's rail, terminal blocks,
                    contactor, fuses, relay socket and wires; the cabinet's board (its face showing the bare board
                    bake, board_circuit) with its parts. A prop of its own, so its colours have their own atlas
  part_relay, part_relay_burnt, part_cap, part_cap_bulged  the replaceable parts (design 3e), new and broken,
                    seated in the faulty slot from service.json
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
# fault: the slot whose part has failed; fits: the part that replaces it (PARTS); pouch: what the kit carries to it (design
# 3e: three parts, one right); game: the 2D game that calibrates the machine once the part is in.
BAYS = {
    "coolant_pump_svc": {"label": "terminal box", "centre": (0.62, 1.92, 0.0), "yaw": 90.0, "size_m": (0.38, 0.32),
                         "opening_m": (0.31, 0.25), "depth_m": 0.12, "shows": None, "cover": "cover_pump", "fault": "K1",
                         "fits": "relay_24vdc", "pouch": ["relay_230vac", "relay_24vdc", "relay_12vdc"], "game": "pump"},
    "local_panel_svc": {"label": "valve board", "centre": (0.0, 0.85, 0.0), "yaw": 180.0, "size_m": (0.56, 0.455),
                        "opening_m": (0.49, 0.385), "depth_m": 0.12, "shows": "board_circuit", "cover": "cover_panel",
                        "fault": "C2", "fits": "cap_470u_35v", "pouch": ["cap_470u_16v", "cap_47u_35v", "cap_470u_35v"],
                        "game": "coolant"},
}
# The parts catalogue (design 3e; in the game, data/ships/tern/parts.json): a part's type and rating, and its model.
PARTS = {
    "relay_24vdc": {"type": "Relay", "rating": "24 V DC", "model": "part_relay"},
    "relay_230vac": {"type": "Relay", "rating": "230 V AC", "model": "part_relay"},
    "relay_12vdc": {"type": "Relay", "rating": "12 V DC", "model": "part_relay"},
    "cap_470u_35v": {"type": "Capacitor", "rating": "470 \u00b5F 35 V", "model": "part_cap"},
    "cap_470u_16v": {"type": "Capacitor", "rating": "470 \u00b5F 16 V", "model": "part_cap"},
    "cap_47u_35v": {"type": "Capacitor", "rating": "47 \u00b5F 35 V", "model": "part_cap"},
}
BROKEN = {"coolant_pump_svc": "part_relay_burnt", "local_panel_svc": "part_cap_bulged"}
# A bay is worked kneeling at arm's length, so its machine's atlas reaches this density (a texel 1 cm): at the set's
# 52 px/m the cover's seat, 3.5 cm wide, was two blotchy texels across (the owner, 2026-10-10: "The frame around the
# open panel").
BAY_PX_PER_M = 100
COVER_T = 0.012       # the cover's thickness, and the rebate's depth: it sits flush
HOLE_R = 0.0042       # a tapped hole in the rebate, under each screw

SCREW_SHANK_M = 0.03   # a screw's shank: the head's underside is this far up its prop space
BUDGETS = {"coolant_pump_svc": 800, "local_panel_svc": 400, "coolant_pump_svc_hw": 1200, "local_panel_svc_hw": 1000,
           "part_relay": 40, "part_relay_burnt": 40, "part_cap": 80, "part_cap_bulged": 80, "cover_pump": 40, "cover_panel": 40, "service_screw": 90}


def hole_points(bay):
    """The screws' points on the cover, local (x, y), from the cover bake's hole fractions (design 3b)."""
    with open(COVERS, encoding="utf-8") as f:
        uvs = json.load(f)["covers"]["working"]["holes_uv"]
    w, h = bay["size_m"]
    return [((u - 0.5) * w, (0.5 - v) * h) for u, v in uvs]


def cut_bay(p, bay, label):
    """Cut a bay into p's body: the rebate (the cover's size, COVER_T deep, bare metal), the opening behind it
    (machinery walls, a plain floor the hardware's plate stands in front of) and a tapped hole under each screw."""
    m = frame(bay["centre"], 0.0, bay["yaw"])
    p.shows = tuple(SHOWS) + INTERIORS
    w, h = bay["size_m"]
    ow, oh = bay["opening_m"]
    cuts = []
    p.recess(cuts, f"{label}_rebate", m, w, h, COVER_T, wall_role="trim", floor_role="trim", record=False)
    # The bay's floor is plain: the hardware stands on its own mounting plate in front of it (design 3e).
    p.recess(cuts, f"{label}_bay", m @ Matrix.Translation((0.0, 0.0, -COVER_T)), ow, oh, bay["depth_m"],
             wall_role="machinery", floor_role="bulkhead", record=False)
    for k, (x, y) in enumerate(hole_points(bay)):
        cuts.append(p.box(f"{label}_tap_{k}", (-HOLE_R, -HOLE_R, -COVER_T - 0.03), (HOLE_R, HOLE_R, 0.02), "machinery",
                          m=m @ Matrix.Translation((x, y, 0.0))))
    p.cut(p.body, f"{label}_bay", cuts)

    def decor(D):
        """The cover's seat, the rebate's walls and the bay's walls, machined steel (a flat finish: no rust or edge wear),
        so the frame round the open bay reads as one clean part (the owner, 2026-10-10: "The frame around the open
        panel"). The bay's floor, behind the hardware's plate, keeps its paint."""
        inv = m.inverted()
        for R in regions(D, lambda R: R.role in ("trim", "machinery")):
            q = inv @ R.origin
            if abs(q.x) <= w / 2 + 0.001 and abs(q.y) <= h / 2 + 0.001 and -COVER_T - bay["depth_m"] - 0.001 <= q.z <= 0.001:
                whole(D, R, "machined")
    p.decor.append(decor)


# ----------------------------------------------------------------------------- the hardware in the bays (design 3d)
# A piece: kind, id, label, its box on the bay floor (c its centre and s its size across and up the bay, metres), z its
# height out of the floor, its finish (prop_atlas.json), and whether a click names it. Built into the machine's mesh; its
# box goes to service.json as a hit target. The bay floor's frame: +Z out of the machine.

def pump_hardware():
    """The motor terminal box: a DIN rail of 12 terminal blocks (two blue neutrals, a yellow earth), the relay K1, the
    contactor KM1 and three fuses F1-F3, with wires up from the blocks into the box's top."""
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
    return out, wires


def board_hardware(bay):
    """The cabinet's board: the parts the bare board bake recorded (assets/textures/repairs/covers.json boards), placed
    on the board's face at the bake's scale, so they stand on their own pads and outlines."""
    with open(COVERS, encoding="utf-8") as f:
        board = json.load(f)["boards"]["circuit"]
    (fw, fh), (ow, oh) = board["frame_m"], board_m(bay)
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


# The hardware stands on a mounting plate, a prop of its own (design 3e: one atlas has one palette, and the parts' colours
# took the casing's). Its prop space: the plate's back on z = 0, its bottom edge's middle at the origin, +Y up the bay,
# +X across it, the parts standing out along +Z from the plate's face. It sits PLATE_GAP off the bay's floor.
PLATE_T = 0.004       # the mounting plate's thickness
PLATE_GAP = 0.008     # the plate stands off the bay floor (as a board on standoffs): its face 11-12 mm out, never within 1 cm
PLATE_INSET = 0.004   # between the plate's edges and the bay's walls
SINK = 0.002          # every part is sunk this far into the plate (or the board's recess floor), so the union is clean


def plate_m(bay):
    """The mounting plate's width and height, the opening less the inset round it."""
    ow, oh = bay["opening_m"]
    return ow - 2 * PLATE_INSET, oh - 2 * PLATE_INSET


BOARD_RIM = 0.012   # the plate's rim round the board's recess: over 1 cm, so its floor-side wall clears the kit's check


def board_m(bay):
    """The board's face (the recorded screen showing the bare board bake): the plate less its rim."""
    pw, ph = plate_m(bay)
    return pw - 2 * BOARD_RIM, ph - 2 * BOARD_RIM


def bay_floor(bay):
    """The bay floor's frame in the machine's prop space: its centre, +Z out of the machine, +Y up the cover."""
    return frame(bay["centre"], 0.0, bay["yaw"]) @ Matrix.Translation((0.0, 0.0, -COVER_T - bay["depth_m"]))


def mount_in_machine(bay):
    """Where the parts stand, in the machine's prop space: the plate's face, centred."""
    return bay_floor(bay) @ Matrix.Translation((0.0, 0.0, PLATE_GAP + PLATE_T))


def mount_in_hw(bay):
    """Where the parts stand, in the hardware prop's space: the plate's face, centred."""
    return Matrix.Translation((0.0, plate_m(bay)[1] / 2, PLATE_T))


def hw_origin(bay):
    """The hardware prop's origin in the machine's prop space (its axes are the bay floor's)."""
    return bay_floor(bay) @ Matrix.Translation((0.0, -plate_m(bay)[1] / 2, PLATE_GAP))


# Inside a closed box the hardware is clean: it and the parts bake under the crew spaces' wear (no rust, no streaks), and
# the rust the working wear puts on the blue and the housing paint does not take the palette's colours (design 3e).
CLEAN = "crew"
SOCKET_M = 0.008   # the relay's socket, which stays on the plate when the relay comes out


def build_hardware(p, bay, label, parts, wires):
    """Model the pieces on the hardware prop's plate (p.body), paint each in its finish, and return their boxes in the
    machine's prop space (hit targets) and the faulty part's slot. The faulty part is not modelled here: it is a prop of
    its own (design 3e), and only a relay's socket stays."""
    mg, mm = mount_in_hw(bay), mount_in_machine(bay)
    fault = bay["fault"]
    objs, boxes, slot = [], [], None
    for q in parts:
        (cx, cy), (w, h), z = q["c"], q["s"], q["z"]
        at = mg @ Matrix.Translation((cx, cy, 0.0))
        k, nm = q["kind"], f"{label}_{q['id']}"
        if q["id"] == fault:
            seat = SOCKET_M if k == "relay" else 0.0
            if seat:
                objs.append(p.box(nm + "_socket", (-w / 2 - 0.003, -h / 2 - 0.003, -SINK), (w / 2 + 0.003, h / 2 + 0.003, seat), "trim", m=at))
            # The part's own origin (the middle of its footprint's bottom edge, on its seat) in the hardware prop's space.
            o = mg @ Vector((cx, cy - h / 2, seat))
            slot = {"id": q["id"], "kind": k, "at_m": r3(o), "size_m": r3([w, h, z - seat]), "finish": q["finish"]}
        elif k == "can":
            r = min(w, h) / 2
            objs.append(eng.turned(p, nm, at, [(r, -SINK), (r, z)], "trim", sides=10, caps=("trim", "trim")))
        elif k == "screw":
            objs.append(eng.turned(p, nm, at, [(w / 2, -SINK), (w / 2, z)], "trim", sides=8, caps=("trim", "trim")))
        elif k == "heatsink":
            objs.append(p.box(nm + "_base", (-w / 2, -h / 2, -SINK), (w / 2, h / 2, 0.004), "trim", m=at))
            # Fins 3.6 mm thick at least 1 cm apart (CLAUDE.md 8: deliberately parallel faces 1 cm apart), as many as fit.
            n = max(2, min(q.get("fins", 8), int((w - 0.006) / 0.0136) + 1))
            for j in range(n):
                fx = -w / 2 + 0.003 + j * (w - 0.006) / (n - 1)
                objs.append(p.box(f"{nm}_fin{j}", (fx - 0.0018, -h / 2, 0.003), (fx + 0.0018, h / 2, z), "trim", m=at))
        elif k == "fuse":
            objs.append(p.box(nm + "_holder", (-w / 2, -h / 2, -SINK), (w / 2, h / 2, z), "trim", m=at))
            cart = at @ Matrix.Translation((0.0, -h * 0.36, z + 0.004)) @ Matrix.Rotation(-math.pi / 2, 4, "X")
            objs.append(eng.turned(p, nm + "_cartridge", cart, [(0.0085, 0.0), (0.0085, h * 0.72)], "trim", sides=8, caps=("trim", "trim")))
        elif k != "group":   # chip, box, relay, rail, block, header; a group is a hit target over pieces built on their own
            objs.append(p.box(nm, (-w / 2, -h / 2, -SINK), (w / 2, h / 2, z), "trim", m=at))
        if k == "header":
            nw = q.get("wires", 4)
            for j in range(nw):
                x0 = -w / 2 + w * (j + 0.5) / nw
                wires.append(([(cx + x0, cy, z - 0.002), (cx + x0, cy + 0.02, z + 0.004), (cx + x0 * 1.4, plate_m(bay)[1] / 2 - 0.004, z)],
                              ("rubber", "red")[j % 2]))
        box = {"id": q["id"], "label": q["label"], "finish": q["finish"], "click": q["click"], "fault": q["id"] == fault, "kind": k}
        for key, m in (("", mm), ("g", mg)):
            corners = [m @ Vector((cx + sx_ * w / 2, cy + sy_ * h / 2, zz)) for sx_ in (-1, 1) for sy_ in (-1, 1) for zz in (0.0, z + 0.008)]
            box[key + "lo_m"] = r3([min(c[i] for c in corners) for i in range(3)])
            box[key + "hi_m"] = r3([max(c[i] for c in corners) for i in range(3)])
        boxes.append(box)
    for j, (pts, fin) in enumerate(wires):
        path = [tuple(mg @ Vector(pt)) for pt in pts]
        objs.append(pipe(p, f"{label}_wire{j}", path, 0.0034, "trim", sides=4))
        boxes.append({"id": f"wire{j}", "label": "Wire", "path_m": [r3(v) for v in path], "finish": fin, "click": False,
                      "fault": False, "kind": "wire"})
    p.union(p.body, f"{label}_hardware", objs)

    def decor(D):
        """Each piece in its own finish, found by its box (in this prop's space) or its wire's path."""
        for b in boxes:
            if b["kind"] == "group" or not b["finish"] or b["fault"]:
                continue
            if b["kind"] == "wire":
                pts = [Vector(v) for v in b["path_m"]]
                for R in regions(D, lambda R, pts=pts: R.role == "trim" and min(_seg_dist(R.origin, a, c) for a, c in zip(pts, pts[1:])) < 0.006):
                    whole(D, R, b["finish"])
                continue
            lo, hi = Vector(b["glo_m"]) - Vector((0.0015,) * 3), Vector(b["ghi_m"]) + Vector((0.0015,) * 3)
            for R in regions(D, lambda R, lo=lo, hi=hi: R.role == "trim" and all(lo[i] <= R.origin[i] <= hi[i] for i in range(3))):
                top = R.n.z > 0.99
                fin = b["finish"]
                if b["kind"] == "can" and top:
                    fin = "plate"                                # an aluminium top
                elif b["kind"] == "box" and top:
                    fin = "steel"                                # the contactor's face plate
                elif b["kind"] == "fuse" and R.origin.z - PLATE_T > 0.023:
                    fin = "copper"                               # the cartridge, above its holder
                whole(D, R, fin)
        if slot and slot["kind"] == "relay":                     # the relay's socket, dark
            sx, sy, sz = slot["at_m"]
            w, h = slot["size_m"][0] + 0.006, slot["size_m"][1] + 0.006
            for R in regions(D, lambda R: R.role == "trim" and abs(R.origin.x - sx) < w / 2 + 0.001
                             and sy - 0.004 < R.origin.y < sy + h + 0.001 and R.origin.z <= sz + 0.001):
                whole(D, R, "rubber")
    p.decor.append(decor)
    return boxes, slot


HARDWARE = {}   # each machine's hardware: its boxes (hit targets) and its faulty part's slot, for service.json


def coolant_pump_svc():
    """The coolant pump, with a terminal box on the motor's +X flank in place of the small junction box (it
    covers it), and the box's lid bay cut into its +X face. Its hardware is coolant_pump_svc_hw."""
    p = eng.coolant_pump()
    p.name = "coolant_pump_svc"
    box = p.box("terminal_box", (0.40, 1.72, -0.24), (0.62, 2.12, 0.24), {"+x": "bulkhead", "*": "machinery"})
    p.union(p.body, "terminal_box", [box])
    cut_bay(p, BAYS["coolant_pump_svc"], "lid")
    p.presents = "Prototype (repairs-on-deck 3c, 3e): the coolant pump with its motor terminal box's lid bay built in"
    p.min_px_per_m = BAY_PX_PER_M
    return p


def local_panel_svc():
    """The local control cabinet with a service door bay in its back (-Z); the sloped instrument face is the
    operating face and is not touched. Its hardware is local_panel_svc_hw."""
    p = eng.local_panel()
    p.name = "local_panel_svc"
    cut_bay(p, BAYS["local_panel_svc"], "door")
    p.presents = "Prototype (repairs-on-deck 3c, 3e): the local control cabinet with a service door bay in its back"
    p.min_px_per_m = BAY_PX_PER_M
    return p


def hardware_parts(machine):
    """A machine's bay hardware, as pieces and wires."""
    return pump_hardware() if machine == "coolant_pump_svc" else board_hardware(BAYS[machine])


def hardware_prop(machine):
    """A bay's hardware on its mounting plate (design 3e). The cabinet's plate is the board: its face a recorded screen
    in a 1 mm recess showing the bare board bake, the parts standing on their pads; the pump's is a painted plate."""
    bay = BAYS[machine]

    def build():
        pw, ph = plate_m(bay)
        p = Prop(f"{machine}_hw", f"Prototype (repairs-on-deck 3e): the {bay['label']} bay's hardware on its mounting plate",
                 "the middle of the plate's bottom edge, the plate's rear face on z = 0", back_at_z0=True, shows=tuple(SHOWS) + INTERIORS)
        p.body = p.box("plate", (-pw / 2, 0.0, 0.0), (pw / 2, ph, PLATE_T), {"+z": "bulkhead" if not bay["shows"] else "machinery",
                                                                              "*": "machinery"})
        if bay["shows"]:
            cuts = []
            p.recess(cuts, "board", Matrix.Translation((0.0, ph / 2, PLATE_T)), *board_m(bay), 0.001,
                     wall_role="machinery", floor_role="screen", record=True, shows=bay["shows"])
            p.cut(p.body, "board", cuts)
        parts, wires = hardware_parts(machine)
        HARDWARE[machine] = build_hardware(p, bay, bay["label"], parts, wires)
        p.wear = CLEAN
        return p
    return build


def part_prop(name, machine, broken):
    """A replaceable part (design 3e), standing as it sits in its slot: its footprint's bottom edge on y = 0, its back
    (the face on its seat) on z = 0, out along +Z. The broken one is the slot's fault (the scorched relay with its lamp
    lit, the bulged capacitor with its top scorched); the new one is clean."""
    def build():
        parts, wires = hardware_parts(machine)
        q = next(q for q in parts if q["id"] == BAYS[machine]["fault"])
        (w, h), z = q["s"], q["z"]
        what = "broken" if broken else "new"
        p = Prop(name, f"Prototype (repairs-on-deck 3e): the {what} {q['kind']} of {q['label']}", "the middle of its bottom edge, the face on its seat on z = 0",
                 back_at_z0=True)
        if q["kind"] == "relay":
            zt = z - SOCKET_M
            p.body = p.box("relay", (-w / 2, 0.0, 0.0), (w / 2, h, zt), "trim")

            def decor(D):
                for R in regions(D, lambda R: R.role == "trim"):
                    whole(D, R, "rubber" if (broken and R.n.z > 0.99) else q["finish"])
                D.disc(Matrix.Translation((0.012, h / 2 + 0.016, zt + 0.0005)), 0.0, 0.0, 0.005, 0.0, 0.002,
                       "led_red" if broken else "led_green", sides=10, reserve=False)
        else:
            r = min(w, h) / 2
            prof = [(r, 0.0), (r, z), (r * 0.8, z + 0.006)] if broken else [(r, 0.0), (r, z)]
            p.body = eng.turned(p, "can", Matrix.Translation((0.0, r, 0.0)), prof, "trim", sides=10, caps=("trim", "trim"))
            # Stand it on y = 0 whatever the polygon's phase leaves lowest.
            lo = min((p.body.matrix_world @ v.co).y for v in p.body.data.vertices)
            p.body.matrix_world = Matrix.Translation((0.0, -lo, 0.0)) @ p.body.matrix_world

            def decor(D):
                for R in regions(D, lambda R: R.role == "trim"):
                    top = R.n.z > 0.5
                    whole(D, R, ("dark" if broken else "plate") if top else q["finish"])
        p.decor.append(decor)
        p.wear = CLEAN
        return p
    return build


def cover(name, bay):
    """A cover: a plate the rebate's size, standing on its bottom edge, from z = 0 (its back, on the rebate floor)
    to COVER_T (its front)."""
    def build():
        w, h = bay["size_m"]
        p = Prop(name, f"Prototype (repairs-on-deck 3c): the service cover of the {bay['label']} bay, {w} x {h} m",
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
         "coolant_pump_svc_hw": hardware_prop("coolant_pump_svc"), "local_panel_svc_hw": hardware_prop("local_panel_svc"),
         "part_relay": part_prop("part_relay", "coolant_pump_svc", False),
         "part_relay_burnt": part_prop("part_relay_burnt", "coolant_pump_svc", True),
         "part_cap": part_prop("part_cap", "local_panel_svc", False),
         "part_cap_bulged": part_prop("part_cap_bulged", "local_panel_svc", True),
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
        boxes, slot = HARDWARE.get(name, ([], None))
        hw = [{k: b[k] for k in ("id", "label", "lo_m", "hi_m", "fault") if k in b} for b in boxes if b["click"]]
        # The calibrate target (design 3e): the whole opening, from the bay's floor to its face.
        ow, oh = bay["opening_m"]
        corners = [m @ Vector((sx * ow / 2, sy * oh / 2, z)) for sx in (-1, 1) for sy in (-1, 1) for z in (-COVER_T - bay["depth_m"], 0.0)]
        ho = hw_origin(bay)
        out[name] = {"label": bay["label"], "cover": bay["cover"], "size_m": list(bay["size_m"]), "thickness_m": COVER_T,
                     "game": bay["game"], "hardware": hw,
                     "hardware_prop": f"{name}_hw", "hardware_at_m": r3(ho.translation),
                     "slot": dict(slot or {}, broken=BROKEN[name], fits=bay["fits"]), "pouch": list(bay["pouch"]),
                     "bay_box_m": {"lo_m": r3([min(c[i] for c in corners) for i in range(3)]),
                                   "hi_m": r3([max(c[i] for c in corners) for i in range(3)])},
                     "cover_at_m": r3(home), "normal": r3(n), "up": r3(up),
                     "screws_m": [r3(m @ Vector((x, y, 0.0))) for x, y in hole_points(bay)],
                     "bay_depth_m": bay["depth_m"], "opening_m": list(bay["opening_m"])}
    return {"schema": "starcrew.service-bays/1", "built_by": GENERATOR,
            "_rules": ["Prop space of each machine (+Y up, +Z its front). cover_at_m is the centre of the cover's back when it is home "
                       "in the rebate, normal out of the machine, up the cover's up; the cover's front is thickness_m along normal. "
                       "screws_m are the screws' head undersides when driven home, on the face. shows is the bake the "
                       "hardware is every piece a click names (design 3d): its box in the machine's prop space. hardware_prop "
                       "is the prop the pieces stand on (design 3e), its origin at hardware_at_m with the bay's axes (x across, "
                       "up, normal). slot is the faulty part's: at_m its origin in the hardware prop's space, broken the model "
                       "seated there, fits the part (parts) that replaces it; pouch is what the kit carries. bay_box_m is the "
                       "calibrate target; game the 2D game that calibrates the machine once the right part is in."],
            "parts": PARTS, "machines": out}


STATUS = ("Prototype (2026-10-09, 3e 2026-10-10) by " + GENERATOR + ", for the owner's review (repairs-on-deck design 3c-3e). Not the shipped "
          "engineering props: those are untouched until the owner approves these.")
RULES = list(eng.RULES) + ["a screen with shows board_circuit is a hardware plate's board: the page lays that bake "
                           "(assets/textures/repairs) on it; the parts on it are modelled (design 3d, 3e).",
                           "the <machine>_hw and part_* props bake under the crew wear (Prop.wear), clean inside a closed box; "
                           "the machines and covers under the set's."]
SET = PropSet("service_proto", "ServicePrototypes", OUT, GENERATOR, PROPS, BUDGETS, STATUS, RULES, __doc__)


if __name__ == "__main__":
    run(SET)
    if "--check" not in sys.argv:
        with open(os.path.join(OUT, "service.json"), "w", encoding="utf-8") as f:
            json.dump(service_json(), f, indent=2)
            f.write("\n")
        print("[props] wrote", os.path.relpath(os.path.join(OUT, "service.json"), ROOT))
