#!/usr/bin/env python3
"""Engineering fitted out as a working fusion plant (openspec/changes/engineering-fitout).

It owns data/ships/tern/engineering.json: a layout patch (schema starcrew.layout-patch/1, applied after
data/ships/tern/deck_access.json) that adds the ring catwalk round the reactor and its bridge, and moves the
reactor panel and the coolant pumps' system to where they are drawn; plus, under "engineering", what the
mockups place and sweep:
- machines: the engineering prop set's machines (and valves), each at its back and yaw;
- runs: pipe runs between the machines' ports, the reactor and each other, with their hangers;
- trays: the feeders' cable trays, the conduits' power.json paths clipped to the room (derived: --check
  fails when power.json moves and this file does not), and the converters' busbars;
- railings, the ring's hanger rods and the crane's runways.

The owner, 2026-10-07: "engine core room is kinda empty, need consiles, lots of pipes, and heavy machinery
in here", "think about how a spaceship engine works, its heavy hot, needs fuel, needs coolant", "needs
constant maintentance with tools and pipes to move resources around", "tanks to store or buffer stuff, pumps
to force fluids to move".

Every run is checked against the design's section 4 rules (ends on what it names, bends fit, inside the room,
heads clear over walk zones, doors, stairs and catwalks clear, runs apart, machines not pierced, reactor
entries), and every machine against the room, the doors, the stairs, the walk zones and the other machines.
It lives in tools/ beside tools/crew_rooms.py and uses command_suite's door zones, stair footprints, layout
check and patch rule rather than copies (CLAUDE.md 6.1). Documentation tooling (CLAUDE.md section 4),
standard library only.

Usage: python3 tools/engineering_fitout.py [--check]
"""
import copy
import json
import math
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import command_suite as CS  # noqa: E402
import deck_access as DA  # noqa: E402
import layout_check  # noqa: E402

ROOT = os.path.dirname(HERE)
OUT = os.path.join(ROOT, "data", "ships", "tern", "engineering.json")
POWER = os.path.join(ROOT, "data", "ships", "tern", "power.json")
DETAILING = os.path.join(ROOT, "data", "ships", "tern", "detailing.json")
PROPS = os.path.join(ROOT, "assets", "models", "engineering", "props.json")
ROOM = "engineering"

# ------------------------------------------------------------------ the room (layout.json and the deck plan's reading)

FLOOR_LOWER, FLOOR_MEZZ, FLOOR_GANTRY, CEILING = -3.5, 0.0, 3.5, 6.5
SLAB_M = 0.2                    # a fixture floor's thickness in the mockups (deck-plan.html SLAB_M)
WELL_R = 3.4                    # the mezzanine's hole round the reactor (layout fixture eng_mezzanine)
REACTOR = (0.0, -25.0)          # the reactor's axis (layout system reactor)
REACTOR_R = 2.2                 # its plinth and raised rings
REACTOR_ENTRY_R = 1.80          # where a pipe ends inside the column (its twelve-sided body is 1.90 at the corners)
RING_IN, RING_APOTHEM = 2.35, 3.35   # the ring catwalk at deck A level
HEAD_M = 2.1                    # headroom kept over every walk zone, stair and catwalk
RIB_M = 0.22                    # the tall room's ribs and stiffeners (detailing.json frames.tall_rib_depth_m)
GAP_M = 0.05                    # least gap between two runs, and between a run and a machine it does not join
BEAM_BOTTOM = CEILING - 0.38    # the deep beams under the ceiling at every frame z = 2k
PORT_CLEAR_M = 0.4              # a run may pass this close to the port it leaves from inside its machine's bounds


def rot(yaw_deg, x, z):
    """Prop space to world, about +Y (three.js and the deck plan's placeProp): +z faces yaw."""
    th = math.radians(yaw_deg)
    c, s = math.cos(th), math.sin(th)
    return x * c + z * s, -x * s + z * c


def az_point(az_deg, r, y):
    """A point at azimuth az (0 the bow, 90 port) and radius r from the reactor's axis."""
    a = math.radians(az_deg)
    return [REACTOR[0] + r * math.sin(a), y, REACTOR[1] + r * math.cos(a)]


def r3(v):
    return round(v + 0.0, 4)


def p3(p):
    return [r3(p[0]), r3(p[1]), r3(p[2])]


# ------------------------------------------------------------------ the props, as briefed (the build's manifest wins when present)

# W x H x D (metres), anchor ("free": the footprint's centre; "wall" or "back": the centre of its back, on a wall or
# not; "axis": on a pipe's axis), ports {name: (at, dir, dia)} in prop space: the brief given to the build
# (tools/blender/build_engineering_props.py). y0 is the bottom of the bounds where the origin is not on the floor.
BRIEF = {
    "heat_exchanger": ((1.9, 4.6, 1.9), "free", {"hot_in": ((0, 4.25, 1.05), (0, 0, 1), 0.45), "cold_out": ((0, 0.75, -0.95), (0, 0, -1), 0.45),
                                                 "sec_out": ((0.95, 3.2, 0), (1, 0, 0), 0.3), "sec_in": ((-0.95, 1.6, 0), (-1, 0, 0), 0.3)}),
    "pressurizer": ((1.0, 4.2, 1.0), "free", {"surge": ((0, 0.4, 0.6), (0, 0, 1), 0.2)}),
    "coolant_tank": ((3.0, 1.75, 1.3), "free", {"outlet": ((1.0, 0.35, 0.6), (0, 0, 1), 0.15)}),
    "fuel_dewar": ((1.2, 2.75, 1.2), "free", {"fuel_out": ((0, 0.45, 0.65), (0, 0, 1), 0.1)}),
    "helium3_rack": ((2.4, 2.0, 0.7), "wall", {"fuel_out": ((1.15, 1.85, 0.5), (1, 0, 0), 0.08)}),
    "fuel_processor": ((2.4, 1.9, 1.3), "free", {"d2_in": ((-0.6, 0.8, -0.65), (0, 0, -1), 0.1), "he3_in": ((0.0, 0.8, -0.65), (0, 0, -1), 0.08),
                                                 "inject_out": ((0.6, 1.2, 0.65), (0, 0, 1), 0.1)}),
    "cryoplant": ((3.4, 2.75, 1.6), "free", {"lhe_out": ((1.1, 2.4, 0.75), (0, 0, 1), 0.15), "lhe_return": ((0.6, 2.4, 0.75), (0, 0, 1), 0.15)}),
    "vacuum_pump": ((0.8, 1.4, 1.2), "back", {"inlet": ((0, 1.0, 0), (0, 0, -1), 0.35), "exhaust": ((0.3, 0.3, 1.1), (0, 0, 1), 0.1)}),
    "ash_tank": ((1.6, 2.4, 1.0), "free", {"inlet": ((-0.6, 1.8, 0), (-1, 0, 0), 0.15)}),
    "power_converter": ((2.6, 2.8, 1.0), "wall", {"bus_out": ((0, 2.8, 0.5), (0, 1, 0), 0.4)}),
    "reactor_dressing": ((6.6, 6.2, 6.6), "free", {}),
    "gantry_crane": ((17.5, 2.2, 1.6), "free", {}),
    "coolant_pump": ((1.7, 2.75, 1.7), "free", {"suction": ((0, 0.7, -0.8), (0, 0, -1), 0.45), "discharge": ((0, 0.7, 0.8), (0, 0, 1), 0.45)}),
    "local_panel": ((0.9, 1.75, 0.5), "wall", {}),
    "control_desk": ((3.0, 1.5, 1.2), "back", {}),
    "mimic_board": ((3.6, 2.4, 0.15), "wall", {}),
    "tool_board": ((2.2, 2.2, 0.75), "wall", {}),
    "tool_chest": ((1.0, 1.1, 0.55), "free", {}),
    "parts_rack": ((2.2, 2.3, 0.7), "wall", {}),
    "valve_large": ((0.7, 1.0, 0.6), "axis", {"a": ((-0.35, 0, 0), (-1, 0, 0), 0.45), "b": ((0.35, 0, 0), (1, 0, 0), 0.45)}),
    "valve_small": ((0.3, 0.45, 0.25), "axis", {"a": ((-0.15, 0, 0), (-1, 0, 0), 0.15), "b": ((0.15, 0, 0), (1, 0, 0), 0.15)}),
}
# Bounds that are not "floor up" (prop space min y, max y): the crane hangs from its rails' top, a valve sits on its pipe.
Y_RANGE = {"gantry_crane": (-1.6, 0.6), "valve_large": (-0.3, 0.75), "valve_small": (-0.1, 0.35)}


def manifest():
    if os.path.exists(PROPS):
        with open(PROPS, encoding="utf-8") as f:
            return json.load(f)["props"]
    return {}


def prop_shape(name, man):
    """(min x, max x, min y, max y, min z, max z) and ports {name: (at, dir, dia)} in prop space, built or briefed."""
    (w, h, d), anchor, ports = BRIEF[name]
    if name in man:
        b = man[name]["bounds_m"]
        box = (b["min"][0], b["max"][0], b["min"][1], b["max"][1], b["min"][2], b["max"][2])
        built = man[name].get("ports")
        if built:
            ports = {k: (tuple(v["at_m"]), tuple(v["dir"]), v["dia_m"]) for k, v in built.items()}
        return box, ports
    y0, y1 = Y_RANGE.get(name, (0.0, h))
    if anchor in ("wall", "back"):
        return (-w / 2, w / 2, y0, y1, 0.0, d), ports
    return (-w / 2, w / 2, y0, y1, -d / 2, d / 2), ports


# ------------------------------------------------------------------ the machines (design section 3)

def machines():
    M = []

    def put(mid, prop, back, yaw, level, serves, note, **extra):
        M.append(dict({"id": mid, "prop": prop, "set": "engineering", "back_m": p3(back), "yaw_deg": round(yaw, 2),
                       "level": level, "serves": serves, "note": note}, **extra))

    L, Z = FLOOR_LOWER, FLOOR_MEZZ
    # Lower floor: the plant floor.
    put("dressing", "reactor_dressing", [0, L, -25.0], 0, "lower", "reactor", "magnet coils, field rings, ports and cable looms round the column")
    for az in (45, 135, 225, 315):
        put(f"vac_{az}", "vacuum_pump", az_point(az, 3.30, L), az, "lower", "exhaust", f"exhaust pump on the reactor's port at azimuth {az}")
    put("pump_a", "coolant_pump", [7.2, L, -25.0], -90, "lower", "primary_a", "coolant pump A, under heat exchanger A")
    put("pump_b", "coolant_pump", [-7.2, L, -25.0], 90, "lower", "primary_b", "coolant pump B, under heat exchanger B")
    put("dewar_1", "fuel_dewar", [8.3, L, -20.4], -90, "lower", "fuel", "deuterium")
    put("dewar_2", "fuel_dewar", [8.25, L, -21.9], -90, "lower", "fuel", "deuterium")
    put("he3_rack", "helium3_rack", [6.0, L, -18.0], 180, "lower", "fuel", "helium-3, on the forward wall")
    put("fuel_processor", "fuel_processor", [5.0, L, -20.4], -90, "lower", "fuel", "meters the fuel and fires pellets into the reactor")
    put("fuel_panel", "local_panel", [3.4, L, -18.0], 180, "lower", "fuel", "the fuel processor's local panel: fuel kg")
    put("ash_tank", "ash_tank", [-5.2, L, -20.8], 180, "lower", "exhaust", "the exhaust's ash, and its roughing pump")
    put("cryoplant", "cryoplant", [-4.3, L, -30.94], 0, "lower", "cryogenic", "cools the magnet coils; against the aft wall")
    put("cryo_panel", "local_panel", [-2.0, L, -32.0], 0, "lower", "cryogenic", "the cryoplant's local panel")
    put("makeup_tank", "coolant_tank", [6.2, L, -28.7], 0, "lower", "primary_a", "the loop's drain and makeup tank: a buffer")
    put("coolant_valves", "local_panel", [-6.5, L, -27.65], 0, "lower", "primary", "the layout's coolant_valves fixture: loop valves and the radiator bypass",
        fixture="coolant_valves")
    put("bench", "tool_board", [4.4, L, -32.0], 0, "lower", "upkeep", "bench, vice and tool board")
    put("chest_lower", "tool_chest", [2.2, L, -31.45], 0, "lower", "upkeep", "rolling tool chest")
    put("rack_lower", "parts_rack", [0.0, L, -32.0], 0, "lower", "upkeep", "spares: impellers, valve bodies, pipe spools")
    # Mezzanine: the operating floor.
    put("hx_a", "heat_exchanger", [7.0, Z, -25.0], -90, "mezz", "primary_a", "heat exchanger A: the hot leg in at the top, the cold leg down to pump A")
    put("hx_b", "heat_exchanger", [-7.0, Z, -25.0], 90, "mezz", "primary_b", "heat exchanger B")
    put("pressurizer", "pressurizer", [-7.4, Z, -28.4], 90, "mezz", "primary_b", "the loop's pressure buffer, on hot leg B")
    for side, tag in ((1, "p"), (-1, "s")):
        put(f"converter_{tag}", "power_converter", [side * 7.75, Z, -31.0], -side * 38.66, "mezz", f"gen_{tag}",
            f"reactor generator {tag} (power.json gen_{tag}), on the aft chamfer")
    put("control_desk", "control_desk", [5.5, Z, -22.0], 0, "mezz", "eng_main", "the engineer's desk (station eng_main): the seat stands forward of it",
        station="eng_main")
    put("reactor_panel", "local_panel", [0.0, Z, -20.55], 0, "mezz", "reactor", "the layout's reactor_panel fixture: scram reset, throttle, branch valves",
        fixture="reactor_panel")
    put("mimic", "mimic_board", [0.0, 0.6, -18.0], 180, "mezz", "plant", "the plant mimic, under the catwalk")
    put("rack_mezz", "parts_rack", [3.6, Z, -32.0], 0, "mezz", "upkeep", "spares")
    put("chest_mezz", "tool_chest", [-3.5, Z, -31.4], 0, "mezz", "upkeep", "rolling tool chest")
    put("crane", "gantry_crane", [0.0, 5.6, -29.0], 0, "mezz", "upkeep", "overhead crane over the aft bay, on runways along the side walls")
    # Valves, on their runs' horizontal legs (origin on the pipe's axis).
    for side, tag in ((1, "a"), (-1, "b")):
        put(f"valve_hot_{tag}", "valve_large", [side * 4.4, 2.6, -25.0], 0, "mezz", f"primary_{tag}", f"hot leg {tag.upper()} isolation valve", run=f"hot_{tag}")
    put("valve_makeup", "valve_small", [7.9, -2.0, -27.7], 0, "lower", "primary_a", "makeup valve", run="makeup")
    return M


def mark_on_wall(L, M, shapes):
    """A machine whose back lies on a wall of the room (within 2 cm): the page keeps that wall's ribs clear of it."""
    polys = [b["poly"] for b in CS.comp(L, ROOM)["brushes"]]
    for m in M:
        bx, _, bz = m["back_m"]
        m["on_wall"] = BRIEF[m["prop"]][1] == "wall" and min(dist_poly2_edge(p, bx, bz) for p in polys) < 0.02


def placed(m, shapes):
    """A machine's world box corners (x, z), y range, and its ports in world: (at, dir, dia)."""
    box, ports = shapes[m["prop"]]
    bx, by, bz = m["back_m"]
    x0, x1, y0, y1, z0, z1 = box
    corners = []
    for x, z in ((x0, z0), (x1, z0), (x1, z1), (x0, z1)):
        dx, dz = rot(m["yaw_deg"], x, z)
        corners.append((bx + dx, bz + dz))
    if layout_check.signed_area(corners) < 0:
        corners.reverse()
    wp = {}
    for k, (at, d, dia) in ports.items():
        ax, az = rot(m["yaw_deg"], at[0], at[2])
        dx, dz = rot(m["yaw_deg"], d[0], d[2])
        wp[k] = ([bx + ax, by + at[1], bz + az], [dx, d[1], dz], dia)
    return corners, (by + y0, by + y1), wp


# ------------------------------------------------------------------ the runs (design section 4)

def runs(M, shapes):
    by = {m["id"]: m for m in M}
    port = lambda mid, name: placed(by[mid], shapes)[2][name]
    R = []

    def run(rid, loop, kind, dia, frm, via, to, note):
        pts, ends = [], []
        for spec, where in ((frm, "from"), (to, "to")):
            if spec[0] == "port":
                at, d, pdia = port(spec[1], spec[2])
                ends.append({"port": f"{spec[1]}.{spec[2]}", "at": at, "dir": d, "dia": pdia})
            elif spec[0] == "reactor":
                # ("reactor", azimuth, y[, offset]): on the column's entry radius, offset sideways (along the tangent
                # (cos a, 0, -sin a)) for two lines side by side; it leaves radially.
                az, y, off = spec[1], spec[2], spec[3] if len(spec) > 3 else 0.0
                a = math.radians(az)
                base = az_point(az, math.sqrt(REACTOR_ENTRY_R ** 2 - off ** 2), y)
                at = [base[0] + off * math.cos(a), y, base[2] - off * math.sin(a)]
                ends.append({"reactor": az, "at": at, "dir": [math.sin(a), 0.0, math.cos(a)]})
            elif spec[0] == "tee":
                # ("tee", run, point): this end meets that run (on its surface, or on its axis where a branch starts).
                ends.append({"tee": spec[1], "at": list(spec[2])})
            elif spec[0] == "ceiling":
                ends.append({"ceiling": True, "at": list(spec[1])})
            elif spec[0] == "loop":
                ends.append({"loop": True, "at": list(spec[1])})
        pts = [ends[0]["at"]] + [list(v) for v in via] + [ends[1]["at"]]
        R.append({"id": rid, "loop": loop, "kind": kind, "dia_m": dia, "from": ends[0], "to": ends[1], "points_m": [p3(p) for p in pts], "note": note})

    for side, t in ((1, "a"), (-1, "b")):
        s = side
        hx_hot = port(f"hx_{t}", "hot_in")[0]
        run(f"hot_{t}", f"primary_{t}", "lagged", 0.45, ("reactor", 90 if s > 0 else 270, 2.6),
            [(s * 5.2, 2.6, -25.0), (s * 5.2, hx_hot[1], -25.0)], ("port", f"hx_{t}", "hot_in"),
            f"hot leg {t.upper()}: the reactor's blanket to heat exchanger {t.upper()}'s gooseneck")
        cold = port(f"hx_{t}", "cold_out")[0]
        suction = port(f"pump_{t}", "suction")[0]
        run(f"cold_{t}", f"primary_{t}", "lagged", 0.45, ("port", f"hx_{t}", "cold_out"),
            [(s * 8.65, cold[1], -25.0), (s * 8.65, suction[1], -25.0)], ("port", f"pump_{t}", "suction"),
            f"cold leg {t.upper()}: down through the mezzanine to pump {t.upper()}")
        dis = port(f"pump_{t}", "discharge")[0]
        run(f"return_{t}", f"primary_{t}", "lagged", 0.45, ("port", f"pump_{t}", "discharge"),
            [(s * 5.9, dis[1], -25.0), (s * 5.9, -1.1, -25.0)], ("reactor", 90 if s > 0 else 270, -1.1),
            f"return {t.upper()}: pump {t.upper()} back into the reactor")
        so = port(f"hx_{t}", "sec_out")[0]
        si = port(f"hx_{t}", "sec_in")[0]
        zo, zi = (-23.5, -26.5) if s > 0 else (-26.5, -23.5)
        run(f"sec_out_{t}", f"secondary_{t}", "lagged", 0.3, ("port", f"hx_{t}", "sec_out"),
            [(so[0], so[1], zo)], ("ceiling", (so[0], CEILING + 0.05, zo)), "secondary out, up to the hull radiators")
        run(f"sec_in_{t}", f"secondary_{t}", "lagged", 0.3, ("port", f"hx_{t}", "sec_in"),
            [(si[0], si[1], zi)], ("ceiling", (si[0], CEILING + 0.05, zi)), "secondary in, down from the hull radiators")
    sg = port("pressurizer", "surge")[0]
    run("surge", "primary_b", "painted", 0.2, ("port", "pressurizer", "surge"),
        [(-6.4, sg[1], sg[2]), (-6.4, 2.6, sg[2]), (-3.6, 2.6, sg[2])], ("tee", "hot_b", (-3.6, 2.6, -25.0 - 0.225)),
        "the pressurizer's surge line into hot leg B")
    mk = port("makeup_tank", "outlet")[0]
    run("makeup", "primary_a", "painted", 0.15, ("port", "makeup_tank", "outlet"),
        [(mk[0], mk[1], -27.7), (mk[0], -2.0, -27.7), (8.65, -2.0, -27.7)], ("tee", "cold_a", (8.65, -2.0, -25.0 - 0.225)),
        "makeup from the drain and makeup tank into cold leg A")
    # Fuel.
    d1, d2 = port("dewar_1", "fuel_out")[0], port("dewar_2", "fuel_out")[0]
    d2in, he3in, inj = port("fuel_processor", "d2_in")[0], port("fuel_processor", "he3_in")[0], port("fuel_processor", "inject_out")[0]
    run("d2_a", "fuel", "cryo", 0.1, ("port", "dewar_1", "fuel_out"),
        [(6.3, d1[1], d1[2]), (6.3, d1[1], d2in[2]), (6.3, d2in[1], d2in[2])], ("port", "fuel_processor", "d2_in"),
        "deuterium from dewar 1 to the fuel processor")
    run("d2_b", "fuel", "cryo", 0.1, ("port", "dewar_2", "fuel_out"),
        [(6.9, d2[1], d2[2])], ("tee", "d2_a", (6.9, d1[1], d1[2] - 0.05)), "deuterium from dewar 2, teed into dewar 1's line")
    h3 = port("he3_rack", "fuel_out")[0]
    run("he3", "fuel", "cryo", 0.08, ("port", "he3_rack", "fuel_out"),
        [(4.2, h3[1], h3[2]), (4.2, h3[1], -18.95), (6.0, h3[1], -18.95), (6.0, h3[1], he3in[2]), (6.0, he3in[1], he3in[2])],
        ("port", "fuel_processor", "he3_in"), "helium-3 from the rack to the fuel processor, over the gap between them")
    run("inject", "fuel", "cryo", 0.1, ("port", "fuel_processor", "inject_out"),
        [(3.0, inj[1], inj[2]), (3.0, -1.0, inj[2]), (0.0, -1.0, inj[2])], ("reactor", 0, -1.0), "the pellet injector line into the reactor")
    # Cryogenics.
    lo, lr = port("cryoplant", "lhe_out")[0], port("cryoplant", "lhe_return")[0]
    run("lhe_supply", "cryogenic", "cryo", 0.15, ("port", "cryoplant", "lhe_out"),
        [(lo[0], lo[1], -29.75), (0.15, lo[1], -29.75)], ("reactor", 180, lo[1], -0.15), "liquid helium to the magnet coils")
    run("lhe_return", "cryogenic", "cryo", 0.15, ("port", "cryoplant", "lhe_return"),
        [(lr[0], lr[1], -29.25), (-0.15, lr[1], -29.25)], ("reactor", 180, lr[1], 0.15), "helium gas back from the coils")
    # Exhaust: a riser from each pump into a ring header, and a branch to the ash tank.
    ring_y, ring_a = -0.7, 4.55
    for az in (45, 135, 225, 315):
        ex, d, _ = port(f"vac_{az}", "exhaust")
        a = [ex[0] + d[0] * 0.15, ex[1], ex[2] + d[2] * 0.15]
        run(f"exhaust_{az}", "exhaust", "steel", 0.1, ("port", f"vac_{az}", "exhaust"), [a], ("tee", "exhaust_ring", (a[0], ring_y - 0.075, a[2])),
            f"exhaust riser from the pump at azimuth {az}")
    ring = [az_point(0, ring_a, ring_y)] + [az_point(22.5 + 45 * k, ring_a / math.cos(math.radians(22.5)), ring_y) for k in range(8)] + [az_point(0, ring_a, ring_y)]
    R.append({"id": "exhaust_ring", "loop": "exhaust", "kind": "steel", "dia_m": 0.15, "from": {"loop": True, "at": p3(ring[0])},
              "to": {"loop": True, "at": p3(ring[-1])}, "points_m": [p3(p) for p in ring], "note": "the exhaust ring header under the mezzanine"})
    ash = port("ash_tank", "inlet")[0]
    n315 = (-math.sqrt(0.5), math.sqrt(0.5))      # the flat facing azimuth 315: n . (x, z - z_reactor) = ring_a
    xt = (ring_a - n315[1] * (ash[2] - REACTOR[1])) / n315[0]
    xt -= 0.075 / math.sqrt(0.5)       # from the ring's surface, not its axis: the branch leaves along -x across a 45 degree flat
    run("ash", "exhaust", "steel", 0.15, ("tee", "exhaust_ring", (xt, ring_y, ash[2])),
        [(-4.0, ring_y, ash[2]), (-4.0, ash[1], ash[2])], ("port", "ash_tank", "inlet"), "the ring header to the ash tank")
    return R


# ------------------------------------------------------------------ structure: the ring catwalk, railings, rods, runways

def octagon(apothem, y=None):
    R = apothem / math.cos(math.radians(22.5))
    return [az_point(22.5 + 45 * k, R, 0.0 if y is None else y) for k in range(8)]


def structure(R):
    ring_poly = [[r3(p[0]), r3(p[2])] for p in octagon(RING_APOTHEM)]
    if layout_check.signed_area(ring_poly) < 0:
        ring_poly.reverse()
    bridge_z0, bridge_z1 = REACTOR[1] + RING_APOTHEM, -20.0
    fixtures = [
        {"id": "eng_gantry_ring", "kind": "mezzanine", "compartment": ROOM, "center_m": [0.0, FLOOR_GANTRY, REACTOR[1]],
         "ring_inner_radius_m": RING_IN, "poly": ring_poly,
         "note": "A ring catwalk round the reactor at deck A level, hung from the ceiling: the column's upper half within reach (engineering-fitout)."},
        {"id": "eng_gantry_bridge", "kind": "catwalk", "compartment": ROOM,
         "center_m": [0.0, FLOOR_GANTRY, r3((bridge_z0 + bridge_z1) / 2)], "size_m": [1.2, r3(bridge_z1 - bridge_z0)],
         "note": "The bridge from the catwalk to the ring catwalk (engineering-fitout)."},
        {"id": "reactor_panel", "center_m": [0.0, 0.0, -20.3]},
    ]
    rails = []
    # The well: a twelve-sided rail outside the 24-sided hole.
    ap = WELL_R + 0.07
    pts = [az_point(15 + 30 * k, ap / math.cos(math.radians(15)), FLOOR_MEZZ) for k in range(12)]
    rails.append({"id": "rail_well", "points_m": [p3(p) for p in pts], "closed": True})
    # The ring catwalk's outer edge, open across the bridge.
    oc = octagon(RING_APOTHEM - 0.05, FLOOR_GANTRY)       # vertices at azimuths 22.5 + 45k; the bridge meets the flat facing 0
    z_flat = REACTOR[1] + RING_APOTHEM - 0.05
    pts = [[0.65, FLOOR_GANTRY, z_flat]] + [oc[k] for k in range(8)] + [[-0.65, FLOOR_GANTRY, z_flat]]
    # oc[0] is at azimuth 22.5 (port of the bridge); walk round through the aft and back to azimuth 337.5.
    rails.append({"id": "rail_ring", "points_m": [p3(p) for p in pts], "closed": False})
    for s in (1, -1):
        rails.append({"id": f"rail_bridge_{'p' if s > 0 else 's'}", "points_m": [p3([s * 0.55, FLOOR_GANTRY, -20.0]), p3([s * 0.55, FLOOR_GANTRY, z_flat])], "closed": False})
    rails.append({"id": "rail_catwalk_p", "points_m": [p3([1.95, FLOOR_GANTRY, -19.25]), p3([1.95, FLOOR_GANTRY, -19.95]), p3([0.6, FLOOR_GANTRY, -19.95])], "closed": False})
    rails.append({"id": "rail_catwalk_s", "points_m": [p3([-0.6, FLOOR_GANTRY, -19.95]), p3([-1.95, FLOOR_GANTRY, -19.95]), p3([-1.95, FLOOR_GANTRY, -18.1])], "closed": False})
    rails.append({"id": "rail_stair_hole", "points_m": [p3([-2.0, FLOOR_MEZZ, -19.25]), p3([-6.05, FLOOR_MEZZ, -19.25]), p3([-6.05, FLOOR_MEZZ, -18.1])], "closed": False})
    rails.append({"id": "rail_upper_stair", "points_m": [p3([2.05, FLOOR_GANTRY, -19.25]), p3([6.0, FLOOR_MEZZ, -19.25])], "closed": False, "sloped": True})
    rails.append({"id": "rail_lower_stair", "points_m": [p3([-4.0, -1.75, -19.25]), p3([-6.0, FLOOR_LOWER, -19.25])], "closed": False, "sloped": True})
    rods = []
    for k in range(8):
        a = 22.5 + 45 * k
        p = az_point(a, 3.5, FLOOR_GANTRY)
        rods.append({"id": f"hanger_ring_{k}", "from_m": p3(p), "to_m": p3([p[0], CEILING - 0.05, p[2]]), "dia_m": 0.06})
    runways = [{"id": f"runway_{t}", "from_m": [s * 8.5, 5.45, -30.0], "to_m": [s * 8.5, 5.45, -27.6], "size_m": [0.3, 0.3]}
               for s, t in ((1, "p"), (-1, "s"))]
    # The lower floor's own lamps, under the mezzanine (CLAUDE.md 11: lit by its own fixtures): over the ring round the
    # reactor between the pipes' azimuths, and over each bay. Every third is on the emergency bus.
    under = FLOOR_MEZZ - SLAB_M
    def clear(p):   # a lamp's capsule (along x, as the page draws it) clear of every run and hanger by 0.1 m
        a, b = [p[0] - 0.25, under, p[2]], [p[0] + 0.25, under, p[2]]
        segs = [(r["dia_m"] / 2, u, v) for r in R for u, v in legs(r["points_m"])]
        segs += [(0.02, [h[0], h[1], h[3]], [h[0], h[2], h[3]]) for r in R for h in r["hangers"]]
        return all(seg_seg(a, b, u, v) >= rad + 0.225 + GAP_M + 0.05 for rad, u, v in segs)
    ring = [next((q for q in (az_point(22.5 + 45 * k + da, rr, under) for da in (0, -8, 8, -15, 15) for rr in (5.05, 5.2, 5.35)) if clear(q)),
                 az_point(22.5 + 45 * k, 5.05, under)) for k in range(8)]
    spots = ring + [[6.8, under, -21.2], [4.6, under, -30.4], [-4.4, under, -29.4], [-6.3, under, -22.6]]
    lamps = [{"id": f"lamp_lower_{i}", "at_m": p3([p[0], under - 0.03, p[2]]), "size_m": [0.9, 0.45], "emergency": i % 3 == 0} for i, p in enumerate(spots)]
    return fixtures, rails, rods, runways, lamps


# ------------------------------------------------------------------ the feeders' trays (power.json, design section 4)

# The new engineering legs of the five conduits (power.json is edited to these; this table is what --check compares
# power.json against, so the two cannot drift).
TRAYS_ROUTE = {
    "k_drive_p": [[8.6, 2.0, -20.6], [8.6, 5.0, -20.6], [8.6, 5.0, -29.6], [6.2, 5.0, -29.6], [6.2, 5.0, -31.45], [0.5, 5.0, -31.45], [0.5, 2.5, -31.45], [0.5, 2.5, -34.0]],
    "k_drive_s": [[-8.6, 2.0, -21.45], [-8.6, 5.0, -21.45], [-8.6, 5.0, -29.6], [-6.2, 5.0, -29.6], [-6.2, 5.0, -31.45], [-0.5, 5.0, -31.45], [-0.5, 2.5, -31.45], [-0.5, 2.5, -34.0]],
    "k_aft_p": [[8.6, 2.0, -20.15], [8.6, 2.7, -20.15], [8.6, 2.7, -18.75], [7.75, 2.7, -18.75], [7.75, 2.7, -18.0], [8.0, 2.6, -8.0]],
    "k_aft_s": [[-8.6, 2.0, -20.55], [-8.6, 2.7, -20.55], [-8.6, 2.7, -18.75], [-7.75, 2.7, -18.75], [-7.75, 2.7, -18.0], [-8.0, 2.6, -8.0]],
    "k_a_aft": [[-8.6, 2.0, -21.0], [-8.6, 2.95, -21.0], [-8.6, 2.95, -18.75], [-6.8, 2.95, -18.75], [-6.8, 6.2, -18.75], [0.6, 6.2, -18.75], [0.6, 6.2, -18.0],
                [0.6, 6.2, 0.0], [0.6, 6.2, 4.0], [0.6, 6.2, 10.5]],
}
TRAY_M = (0.4, 0.1)        # a ladder tray: width, depth
BUSBAR_M = (0.5, 0.2)      # the converters' busbar duct


def inside_room(L, x, z, tol=0.0):
    return any(layout_check.inside_poly(b["poly"], x, z, tol=tol) for b in CS.comp(L, ROOM)["brushes"])


def clip_to_room(L, path):
    """The part of a polyline inside the room, as one polyline (a conduit crosses the room once), ending on its walls."""
    out = []
    for i in range(len(path) - 1):
        a, b = path[i], path[i + 1]
        n = max(2, int(math.dist(a, b) / 0.01))
        seg = [[a[k] + (b[k] - a[k]) * t / n for k in range(3)] for t in range(n + 1)]
        inside = [p for p in seg if inside_room(L, p[0], p[2])]
        if not inside:
            continue
        for p in (inside[0], inside[-1]):
            if not out or math.dist(out[-1], p) > 1e-6:
                out.append(p)
    # Keep the corners: drop the interior points that lie on a straight line.
    keep = [out[0]]
    for i in range(1, len(out) - 1):
        u = [out[i][k] - keep[-1][k] for k in range(3)]
        v = [out[i + 1][k] - out[i][k] for k in range(3)]
        cr = [u[1] * v[2] - u[2] * v[1], u[2] * v[0] - u[0] * v[2], u[0] * v[1] - u[1] * v[0]]
        if math.hypot(*cr) > 1e-9:
            keep.append(out[i])
    keep.append(out[-1])
    return [p3(p) for p in keep]


def trays(L, M, shapes, power):
    out = []
    for k in power["conduits"]:
        if ROOM not in k.get("route", []):
            continue
        out.append({"conduit": k["id"], "kind": "tray", "size_m": list(TRAY_M), "points_m": clip_to_room(L, k["path_m"])})
    by = {m["id"]: m for m in M}
    for t in ("p", "s"):
        at = placed(by[f"converter_{t}"], shapes)[2]["bus_out"][0]
        # Up off the cabinet, forward under the tray's leg along the aft bay, and up to the tray's underside.
        pts = [at, [at[0], at[1] + 0.3, at[2]], [at[0], at[1] + 0.3, -29.6], [at[0], 5.0 - TRAY_M[1] / 2, -29.6]]
        out.append({"conduit": f"gen_{t}", "kind": "busbar", "size_m": list(BUSBAR_M), "points_m": [p3(p) for p in pts],
                    "note": f"converter {t} (the generator) up to the drive feeder's tray"})
    return out


# ------------------------------------------------------------------ geometry for the checks

def sub(a, b):
    return [a[0] - b[0], a[1] - b[1], a[2] - b[2]]


def dot(a, b):
    return a[0] * b[0] + a[1] * b[1] + a[2] * b[2]


def seg_seg(p1, q1, p2, q2):
    """Least distance between segments p1-q1 and p2-q2."""
    d1, d2, r = sub(q1, p1), sub(q2, p2), sub(p1, p2)
    a, e, f = dot(d1, d1), dot(d2, d2), dot(d2, r)
    if a < 1e-12 and e < 1e-12:
        return math.dist(p1, p2)
    if a < 1e-12:
        s, t = 0.0, min(1.0, max(0.0, f / e))
    else:
        c = dot(d1, r)
        if e < 1e-12:
            t, s = 0.0, min(1.0, max(0.0, -c / a))
        else:
            b = dot(d1, d2)
            den = a * e - b * b
            s = min(1.0, max(0.0, (b * f - c * e) / den)) if den > 1e-12 else 0.0
            t = (b * s + f) / e
            if t < 0:
                t, s = 0.0, min(1.0, max(0.0, -c / a))
            elif t > 1:
                t, s = 1.0, min(1.0, max(0.0, (b - c) / a))
    c1 = [p1[k] + d1[k] * s for k in range(3)]
    c2 = [p2[k] + d2[k] * t for k in range(3)]
    return math.dist(c1, c2)


def dist_poly2(poly, x, z):
    """Distance from (x, z) to a convex polygon in plan: 0 inside."""
    if layout_check.inside_poly(poly, x, z, tol=0.0):
        return 0.0
    best = 1e9
    for i in range(len(poly)):
        a, b = poly[i], poly[(i + 1) % len(poly)]
        ux, uz = b[0] - a[0], b[1] - a[1]
        ln2 = ux * ux + uz * uz
        t = 0.0 if ln2 < 1e-12 else max(0.0, min(1.0, ((x - a[0]) * ux + (z - a[1]) * uz) / ln2))
        best = min(best, math.hypot(x - a[0] - ux * t, z - a[1] - uz * t))
    return best


def rect(x0, x1, z0, z1):
    return [(x0, z0), (x1, z0), (x1, z1), (x0, z1)]


def r_axis(x, z):
    return math.hypot(x - REACTOR[0], z - REACTOR[1])


def legs(pts):
    return [(pts[i], pts[i + 1]) for i in range(len(pts) - 1)]


def samples(a, b, step=0.05):
    n = max(1, int(math.ceil(math.dist(a, b) / step)))
    return [[a[k] + (b[k] - a[k]) * i / n for k in range(3)] for i in range(n + 1)]


# Walk zones (design section 3): (name, floor y, plan shape) where a shape is ("annulus", r0, r1) or ("poly", polygon).
WALK = [
    ("the lower floor's ring", FLOOR_LOWER, ("annulus", 4.7, 5.4)),
    ("the walkway from the hangar door", FLOOR_LOWER, ("poly", rect(-0.9, 0.9, -20.3, -18.0))),
    ("the lower stair's landing", FLOOR_LOWER, ("poly", rect(-7.2, -6.0, -22.3, -18.0))),
    ("the way from the lower stair to the ring", FLOOR_LOWER, ("poly", rect(-7.2, -4.4, -22.3, -21.5))),
    ("the mezzanine's ring round the well", FLOOR_MEZZ, ("annulus", 3.45, 4.4)),
    ("the floor before the mimic wall", FLOOR_MEZZ, ("poly", rect(-2.0, 2.0, -20.05, -18.15))),
    ("the way from the port gallery door", FLOOR_MEZZ, ("poly", rect(2.0, 7.2, -20.2, -19.25))),
    ("the way from the starboard gallery door", FLOOR_MEZZ, ("poly", rect(-7.2, -2.0, -20.3, -19.25))),
    ("the way aft to the drive hatch", FLOOR_MEZZ, ("poly", rect(-1.3, 1.3, -31.75, -28.4))),
    ("the ring catwalk", FLOOR_GANTRY, ("annulus", RING_IN, RING_APOTHEM)),
    ("the bridge", FLOOR_GANTRY, ("poly", rect(-0.6, 0.6, REACTOR[1] + RING_APOTHEM, -20.0))),
    ("the catwalk", FLOOR_GANTRY, ("poly", rect(-2.0, 2.0, -20.0, -18.0))),
]


def in_shape(shape, x, z, pad):
    if shape[0] == "annulus":
        r = r_axis(x, z)
        return shape[1] - pad < r < shape[2] + pad
    return dist_poly2(shape[1], x, z) < pad if pad > 0 else layout_check.inside_poly(shape[1], x, z, tol=0.0)


def poly_meets_shape(poly, shape):
    if shape[0] == "annulus":
        rmin = dist_poly2([(p[0] - REACTOR[0], p[1] - REACTOR[1]) for p in poly], 0.0, 0.0)
        rmax = max(r_axis(*p) for p in poly)
        return rmin < shape[2] - 0.005 and rmax > shape[1] + 0.005
    return layout_check.polys_overlap(poly, shape[1])


def coil_boxes():
    """The dressing's eight coils as plan rectangles and their y range (design section 2)."""
    out = []
    for k in range(8):
        a = math.radians(22.5 + 45 * k)
        ux, uz, tx, tz = math.sin(a), math.cos(a), math.cos(a), -math.sin(a)
        poly = [(REACTOR[0] + ux * r + tx * h, REACTOR[1] + uz * r + tz * h) for r, h in ((2.25, -0.15), (3.25, -0.15), (3.25, 0.15), (2.25, 0.15))]
        if layout_check.signed_area(poly) < 0:
            poly.reverse()
        out.append((poly, FLOOR_LOWER + 0.4, FLOOR_LOWER + 5.6))
    return out


ENTRIES = {0: (-1.0,), 45: (-2.5,), 90: (2.6, -1.1), 135: (-2.5,), 180: (-1.1,), 225: (-2.5,), 270: (2.6, -1.1), 315: (-2.5,)}


def stair_volumes(L):
    out = []
    for f in L["fixtures"]:
        if f.get("compartment") == ROOM and f["kind"] == "stair":
            out.append((f["id"], CS.stair_poly(f), f["top_m"], f["foot_m"]))
    return out


def tread_y(top, foot, x, z):
    dx, dz = foot[0] - top[0], foot[2] - top[2]
    t = max(0.0, min(1.0, ((x - top[0]) * dx + (z - top[2]) * dz) / (dx * dx + dz * dz)))
    return top[1] + (foot[1] - top[1]) * t


def door_volumes(L):
    """Each wall door's and hatch's clear zone in the room, with its floor and its top plus 0.3 m."""
    out = []
    for p in L["portals"]:
        if ROOM not in p["between"] or p["kind"] not in ("door", "hatch", "pressure_door") or abs(p["normal"][1]) > 0.5:
            continue
        cx, cy, cz = p["center_m"]
        nx, nz = p["normal"][0], p["normal"][2]
        sgn = 1 if p["between"][1] == ROOM else -1
        tx, tz = -nz, nx
        hw, d = p["size_m"][0] / 2 + 0.1, CS.DOOR_CLEAR_M * sgn
        poly = [(cx + tx * hw, cz + tz * hw), (cx - tx * hw, cz - tz * hw), (cx - tx * hw + nx * d, cz - tz * hw + nz * d), (cx + tx * hw + nx * d, cz + tz * hw + nz * d)]
        if layout_check.signed_area(poly) < 0:
            poly.reverse()
        out.append((p["id"], poly, cy - p["size_m"][1] / 2, cy + p["size_m"][1] / 2 + 0.3))
    return out


def top_above(x, y, z):
    """The underside of what is over a point: the mezzanine slab, the ring catwalk or the catwalks, or the ceiling."""
    in_hole = -6.0 <= x <= -2.0 and -19.2 <= z <= -18.0
    if y < FLOOR_MEZZ - SLAB_M and r_axis(x, z) > WELL_R and not in_hole:
        return FLOOR_MEZZ - SLAB_M
    if y < FLOOR_GANTRY - SLAB_M:
        r = r_axis(x, z)
        if RING_IN < r < RING_APOTHEM or (-0.6 <= x <= 0.6 and REACTOR[1] + RING_APOTHEM <= z <= -20.0) or (-2 <= x <= 2 and -20 <= z <= -18):
            return FLOOR_GANTRY - SLAB_M
    k = round(z / 2.0)
    return BEAM_BOTTOM if abs(z - 2.0 * k) < 0.12 + 0.03 else CEILING


def hangers(run):
    """Rods from a long horizontal leg's top up to what is over it (design section 4): every 3 m, at least one."""
    out, r = [], run["dia_m"] / 2
    for a, b in legs(run["points_m"]):
        if abs(a[1] - b[1]) > 1e-6:
            continue
        ln = math.dist(a, b)
        if ln <= 3.0:
            continue
        n = int(ln // 3.0)
        for i in range(1, n + 1):
            t = i / (n + 1)
            p = [a[k] + (b[k] - a[k]) * t for k in range(3)]
            out.append([r3(p[0]), r3(p[1] + r), r3(top_above(p[0], p[1], p[2])), r3(p[2])])
    return out


def bend_r(dia):
    return dia


def check_runs(L, M, shapes, R, T, rods, lamps=()):
    problems = []
    by = {m["id"]: m for m in M}
    boxes = {m["id"]: placed(m, shapes) for m in M}
    room_polys = [b["poly"] for b in CS.comp(L, ROOM)["brushes"]]
    doors, stairs = door_volumes(L), stair_volumes(L)
    coils = coil_boxes()
    run_by = {r["id"]: r for r in R}

    def end_machine(e):
        return e["port"].split(".")[0] if "port" in e else None

    for r in R:
        pts, rad = r["points_m"], r["dia_m"] / 2
        # Ends on what they name, leaving in the port's direction.
        for key, i, j in (("from", 0, 1), ("to", -1, -2)):
            e = r[key]
            if "port" in e or "reactor" in e:
                # The leg at this end, pointing away from the end into the run: a port's direction, or outward radial.
                d = sub(pts[j], pts[i])
                ang = math.degrees(math.acos(max(-1.0, min(1.0, dot(d, e["dir"]) / math.hypot(*d) / math.hypot(*e["dir"])))))
                if ang > 2.0:
                    problems.append(f"{r['id']}: its {key} end leaves {ang:.1f} degrees off {'the port' if 'port' in e else 'radial'}")
            if "port" in e and abs(e["dia"] - r["dia_m"]) > 0.051:
                problems.append(f"{r['id']}: {r['dia_m']} m pipe on a {e['dia']} m port ({e['port']})")
            if "reactor" in e:
                az = e["reactor"]
                p = pts[i]
                ok_y = any(abs(p[1] - y) < 0.01 for y in ENTRIES.get(az, ()))
                a = math.degrees(math.atan2(p[0] - REACTOR[0], p[2] - REACTOR[1])) % 360
                if not ok_y or min(abs(a - az), 360 - abs(a - az)) > 6.0:
                    problems.append(f"{r['id']}: enters the reactor at azimuth {a:.0f}, y {p[1]}, not an entry of design section 2")
            if "tee" in e:
                t = run_by.get(e["tee"])
                if not t:
                    problems.append(f"{r['id']}: tees onto {e['tee']}, which is not a run")
                    continue
                d = min(seg_seg(pts[i], pts[i], a, b) for a, b in legs(t["points_m"]))
                if not (d < 0.012 or abs(d - t["dia_m"] / 2) < 0.012):
                    problems.append(f"{r['id']}: its {key} end is {d:.3f} m from {t['id']}'s axis, neither on it nor on its surface")
        # Bends fit.
        closed = "loop" in r["from"]
        tl = []
        for i in range(1, len(pts) - 1):
            u, v = sub(pts[i], pts[i - 1]), sub(pts[i + 1], pts[i])
            c = max(-1.0, min(1.0, dot(u, v) / math.hypot(*u) / math.hypot(*v)))
            tl.append(bend_r(r["dia_m"]) * math.tan(math.acos(c) / 2))
        for i, (a, b) in enumerate(legs(pts)):
            need = (tl[i - 1] if i > 0 else 0.0) + (tl[i] if i < len(tl) else 0.0)
            if math.dist(a, b) + 1e-6 < need:
                problems.append(f"{r['id']}: leg {i} is {math.dist(a, b):.2f} m, its bends need {need:.2f} m")
        del closed
        ends = {end_machine(r["from"]), end_machine(r["to"])} - {None}
        valve_ids = {m["id"] for m in M if m.get("run") == r["id"]}
        reported = set()

        def report(what):
            if what not in reported:
                reported.add(what)
                problems.append(f"{r['id']}: {what}")

        for li, (a, b) in enumerate(legs(pts)):
            vertical = abs(a[0] - b[0]) < 1e-6 and abs(a[2] - b[2]) < 1e-6
            first, last = li == 0, li == len(pts) - 2
            for p in samples(a, b):
                x, y, z = p
                # Inside the room, clear of the walls and ribs, between floor and ceiling.
                near_ceiling_end = "ceiling" in r["to"] and math.dist(p, pts[-1]) < 0.6
                if not any(layout_check.inside_poly(poly, x, z, tol=0.0) for poly in room_polys):
                    report(f"leaves the room at ({x:.2f}, {y:.2f}, {z:.2f})")
                elif min(dist_poly2_edge(poly, x, z) for poly in room_polys) < rad + RIB_M - 1e-6:
                    report(f"within {rad + RIB_M:.2f} m of a wall at ({x:.2f}, {y:.2f}, {z:.2f})")
                if y - rad < FLOOR_LOWER - 1e-6 or (y + rad > CEILING + 1e-6 and not near_ceiling_end):
                    report(f"through the floor or the ceiling at ({x:.2f}, {y:.2f}, {z:.2f})")
                if y + rad > BEAM_BOTTOM and abs(z - 2.0 * round(z / 2.0)) < 0.12 + rad:
                    report(f"through a ceiling beam at ({x:.2f}, {y:.2f}, {z:.2f})")
                # Heads clear over walk zones, and the catwalks' slabs.
                for name, fy, shape in WALK:
                    if in_shape(shape, x, z, rad) and y - rad < fy + HEAD_M and y + rad > fy - SLAB_M:
                        report(f"in {name}'s headroom at ({x:.2f}, {y:.2f}, {z:.2f})")
                # The mezzanine slab: only a vertical leg may pass through it.
                in_hole = -6.0 <= x <= -2.0 and -19.2 <= z <= -18.0
                if not vertical and y - rad < FLOOR_MEZZ and y + rad > FLOOR_MEZZ - SLAB_M and r_axis(x, z) > WELL_R - rad and not in_hole:
                    report(f"runs inside the mezzanine's slab at ({x:.2f}, {y:.2f}, {z:.2f})")
                for pid, poly, y0, y1 in doors:
                    if dist_poly2(poly, x, z) < rad and y - rad < y1 and y + rad > y0:
                        report(f"in door {pid}'s zone at ({x:.2f}, {y:.2f}, {z:.2f})")
                for sid, poly, top, foot in stairs:
                    if dist_poly2(poly, x, z) < rad and y - rad < tread_y(top, foot, x, z) + HEAD_M and y + rad > min(top[1], foot[1]):
                        report(f"in stair {sid}'s volume at ({x:.2f}, {y:.2f}, {z:.2f})")
                # The reactor's column: only the leg that ends on it.
                at_reactor = ("reactor" in r["from"] and first) or ("reactor" in r["to"] and last)
                if r_axis(x, z) < REACTOR_R + rad and not at_reactor:
                    report(f"through the reactor at ({x:.2f}, {y:.2f}, {z:.2f})")
                for poly, y0, y1 in coils:
                    if dist_poly2(poly, x, z) < rad and y - rad < y1 and y + rad > y0:
                        report(f"through a magnet coil at ({x:.2f}, {y:.2f}, {z:.2f})")
                for ry in (FLOOR_LOWER + 0.6, FLOOR_LOWER + 5.4):
                    if abs(r_axis(x, z) - 3.1) < 0.1 + rad and abs(y - ry) < 0.1 + rad:
                        report(f"through a field ring at ({x:.2f}, {y:.2f}, {z:.2f})")
                # Machines it does not join, and its own machines away from the port.
                for mid, (corners, (y0, y1), ports) in boxes.items():
                    if by[mid]["prop"] in ("reactor_dressing",) or mid in valve_ids:
                        continue
                    if dist_poly2(corners, x, z) < rad - 0.005 and y - rad < y1 - 0.005 and y + rad > y0 + 0.005:
                        if mid in ends and min(math.dist(p, pp[0]) for pp in ports.values()) < PORT_CLEAR_M + rad:
                            continue
                        report(f"through {mid} at ({x:.2f}, {y:.2f}, {z:.2f})")
    # Runs, trays, busbars, hangers and rods apart from each other.
    tubes = []
    for r in R:
        for li, (a, b) in enumerate(legs(r["points_m"])):
            tubes.append((r["id"], r["dia_m"] / 2, a, b, li, len(r["points_m"]) - 2))
        for hx, y0, y1, hz in r["hangers"]:
            tubes.append((r["id"] + "/hanger", 0.02, [hx, y0, hz], [hx, y1, hz], 0, 0))
    for t in T:
        for li, (a, b) in enumerate(legs(t["points_m"])):
            tubes.append((t["conduit"], max(t["size_m"]) / 2, a, b, li, len(t["points_m"]) - 2))
    for rd in rods:
        tubes.append((rd["id"], rd["dia_m"] / 2, rd["from_m"], rd["to_m"], 0, 0))
    for lp in lamps:
        x, y, z = lp["at_m"]
        tubes.append((lp["id"], lp["size_m"][1] / 2, [x - lp["size_m"][0] / 2 + 0.2, y + 0.03, z], [x + lp["size_m"][0] / 2 - 0.2, y + 0.03, z], 0, 0))
    tee = {}
    for r in R:
        for key in ("from", "to"):
            if "tee" in r[key]:
                tee.setdefault(r["id"], set()).add(r[key]["tee"])
    trays_ids = {t["conduit"] for t in T}
    seen = set()
    for i in range(len(tubes)):
        for j in range(i + 1, len(tubes)):
            a, b = tubes[i], tubes[j]
            ida, idb = a[0].split("/")[0], b[0].split("/")[0]
            if ida == idb or idb in tee.get(ida, ()) or ida in tee.get(idb, ()):
                continue
            if ida in trays_ids and idb in trays_ids:
                continue   # the trays are routed together, stacked and side by side (design section 4)
            d = seg_seg(a[2], a[3], b[2], b[3])
            if d < a[1] + b[1] + GAP_M - 1e-6 and (a[0], b[0]) not in seen:
                seen.add((a[0], b[0]))
                problems.append(f"{a[0]} and {b[0]} are {d:.3f} m apart, need {a[1] + b[1] + GAP_M:.3f}")
    return problems


def dist_poly2_edge(poly, x, z):
    """Distance from a point inside a convex polygon to its nearest edge."""
    best = 1e9
    for i in range(len(poly)):
        a, b = poly[i], poly[(i + 1) % len(poly)]
        ux, uz = b[0] - a[0], b[1] - a[1]
        ln2 = ux * ux + uz * uz
        t = max(0.0, min(1.0, ((x - a[0]) * ux + (z - a[1]) * uz) / ln2))
        best = min(best, math.hypot(x - a[0] - ux * t, z - a[1] - uz * t))
    return best


def check_machines(L, M, shapes):
    problems = []
    doors, stairs = door_volumes(L), stair_volumes(L)
    level_floor = {"lower": FLOOR_LOWER, "mezz": FLOOR_MEZZ}
    solid = [m for m in M if not m["prop"].startswith("valve") and m["prop"] != "gantry_crane"]
    geo = {m["id"]: placed(m, shapes) for m in M}
    for m in solid:
        corners, (y0, y1), _ = geo[m["id"]]
        if m["prop"] == "reactor_dressing":
            continue
        if not CS.in_room(L, ROOM, corners):
            problems.append(f"{m['id']} ({m['prop']}) at {m['back_m']}: not inside the room")
        f = level_floor[m["level"]]
        for name, fy, shape in WALK:
            if abs(fy - f) < 0.01 and poly_meets_shape(corners, shape):
                problems.append(f"{m['id']} stands in {name}")
        for pid, poly, dy0, dy1 in doors:
            if layout_check.polys_overlap(corners, poly) and y0 < dy1 and y1 > dy0:
                problems.append(f"{m['id']} stands in door {pid}'s zone")
        for sid, poly, top, foot in stairs:
            if layout_check.polys_overlap(corners, poly) and y0 < max(top[1], foot[1]) + HEAD_M and y1 > min(top[1], foot[1]):
                problems.append(f"{m['id']} stands in stair {sid}")
        if m["level"] == "mezz" and m["prop"] != "mimic_board":
            if min(r_axis(*c) for c in corners) < WELL_R + 0.05 or dist_poly2([(c[0] - REACTOR[0], c[1] - REACTOR[1]) for c in corners], 0, 0) < WELL_R + 0.05:
                problems.append(f"{m['id']} stands over the well")
        if m["level"] == "lower" and m["prop"] == "vacuum_pump":
            continue
        if m["level"] == "lower" and dist_poly2([(c[0] - REACTOR[0], c[1] - REACTOR[1]) for c in corners], 0, 0) < 3.30 - 0.02:
            problems.append(f"{m['id']} stands in the reactor's dressing")
    for i, a in enumerate(solid):
        for b in solid[i + 1:]:
            if a["level"] != b["level"] or "reactor_dressing" in (a["prop"], b["prop"]):
                continue
            if layout_check.polys_overlap(geo[a["id"]][0], geo[b["id"]][0]):
                problems.append(f"{a['id']} overlaps {b['id']}")
    return problems


def estimate(R, T, rails, rods, runways):
    """Triangles the kit will draw for the runs and the structure, by its tessellation (design section 4)."""
    tri = 0
    for r in R:
        n = 8 if r["dia_m"] >= 0.3 else 6
        pts = r["points_m"]
        tri += 2 * n * (len(pts) - 1)
        for i in range(1, len(pts) - 1):
            u, v = sub(pts[i], pts[i - 1]), sub(pts[i + 1], pts[i])
            ang = math.degrees(math.acos(max(-1.0, min(1.0, dot(u, v) / math.hypot(*u) / math.hypot(*v)))))
            tri += 2 * n * max(1, round(4 * ang / 90.0))
        flanges = 2 + int(sum(math.dist(a, b) for a, b in legs(pts)) // 6.0)
        tri += 6 * n * flanges
        tri += 12 * len(r["hangers"])
    for t in T:
        tri += 8 * (len(t["points_m"]) - 1) + 8 * max(0, len(t["points_m"]) - 2)
    for rl in rails:
        segs = len(rl["points_m"]) - (0 if rl["closed"] else 1)
        posts = sum(1 + int(math.dist(a, b) // 1.6) for a, b in legs(rl["points_m"] + (rl["points_m"][:1] if rl["closed"] else [])))
        tri += segs * 3 * 8 + posts * 10
    tri += 12 * len(rods) + 10 * len(runways)
    return tri


# ------------------------------------------------------------------ patch and main

def build():
    """(the document, the deck-access layout with the patch applied)."""
    man = manifest()
    shapes = {name: prop_shape(name, man) for name in BRIEF}
    M = machines()
    R = runs(M, shapes)
    for r in R:
        r["hangers"] = hangers(r)
    fixtures, rails, rods, runways, lamps = structure(R)
    with open(POWER, encoding="utf-8") as f:
        power = json.load(f)
    _, _, LA = DA.build()
    by = {m["id"]: m for m in M}
    pump_b = by["pump_b"]["back_m"]
    patch = {"compartments": [], "portals": [], "stations": [], "fixtures": fixtures, "fixtures_removed": [],
             "systems": [{"id": "coolant_pumps", "center_m": [pump_b[0], pump_b[1], pump_b[2]]}]}
    L = CS.patch_layout(copy.deepcopy(LA), patch)
    mark_on_wall(L, M, shapes)
    T = trays(L, M, shapes, power)
    doc = {
        "schema": "starcrew.layout-patch/1",
        "status": ("Proposed (2026-10-07): openspec/changes/engineering-fitout. Applies after data/ships/tern/deck_access.json. "
                   "Written by tools/engineering_fitout.py; edit the script, not this file."),
        "_rules": [
            "The patch fields are command_suite.json's (ShipKit.applyPatch, tools/command_suite.py patch_layout).",
            "engineering.machines: back_m is the prop's origin (a wall prop: the centre of its back on the wall plane; a free one: the centre "
            "of its footprint; the crane: its rails' top; a valve: on its pipe's axis), yaw_deg the way its front faces (0 the bow, +90 port), "
            "set the prop set that models it. station: the station whose console it is. fixture: the layout fixture it draws.",
            "engineering.runs: points_m the centreline, dia_m the outside diameter; from and to the port (machine.port), the reactor "
            "(azimuth), a tee onto another run, the ceiling, or the run's own start (a loop); hangers [x, y_bottom, y_top, z] rods up from it.",
            "engineering.trays: the feeders' trays, power.json conduit paths clipped to the room (derived), and the converters' busbars; "
            "size_m [width, depth]. railings: polylines along the top of what they guard (sloped on a stair). rods, runways: straight members. "
            "lamps: lamps under the mezzanine, at_m the lens's centre, size_m its plan size.",
            "engineering.replaces: what the fit-out draws instead of the deck plan's own placement.",
        ],
        **patch,
        "engineering": {
            "machines": M,
            "runs": R,
            "trays": T,
            "railings": rails,
            "rods": rods,
            "runways": runways,
            "lamps": lamps,
            "replaces": {"systems": ["coolant_pumps"], "stations": {"eng_main": "control_desk"}},
            # Until the engineering set is built, a page stands a block of the brief's size in for a prop.
            "prop_brief_m": {name: {"size_m": list(dims), "anchor": anchor, "y_m": list(Y_RANGE.get(name, (0.0, dims[1])))}
                             for name, (dims, anchor, _) in BRIEF.items()},
        },
    }
    return doc, L, shapes, power


def main(argv):
    doc, L, shapes, power = build()
    E = doc["engineering"]
    problems = []
    # The patch on every layout the deck plan can show it with.
    _, LS, LA = DA.build()
    for name, base in (("today", copy.deepcopy(CS.L0)), ("with the suite", LS), ("with deck access", LA)):
        Lp = CS.patch_layout(copy.deepcopy(base), {k: doc[k] for k in ("compartments", "portals", "stations", "fixtures", "fixtures_removed", "systems")})
        ok, out = CS.run_layout_check(Lp)
        if not ok:
            problems.append(f"layout check {name}: " + " | ".join(line for line in out.splitlines() if "FAIL" in line or "ERROR" in line)[:400])
    problems += check_machines(L, E["machines"], shapes)
    problems += check_runs(L, E["machines"], shapes, E["runs"], E["trays"], E["rods"], E["lamps"])
    for k in power["conduits"]:
        if k["id"] in TRAYS_ROUTE and k["path_m"] != TRAYS_ROUTE[k["id"]]:
            problems.append(f"power.json {k['id']}: path_m is not the fit-out's route (design section 4)")
    man = manifest()
    missing = sorted({m["prop"] for m in E["machines"]} - set(man))
    length = sum(math.dist(a, b) for r in E["runs"] for a, b in legs(r["points_m"]))
    est = estimate(E["runs"], E["trays"], E["railings"], E["rods"], E["runways"])
    print(f"  {len(E['machines'])} machines, {len(E['runs'])} runs ({length:.1f} m), {len(E['trays'])} trays and busbars, "
          f"{len(E['railings'])} railings; runs and structure about {est} triangles")
    if missing:
        print(f"  not built yet (the brief's sizes stand in): {', '.join(missing)}")
    print(f"  checks {'ok' if not problems else 'FAIL'}")
    for p in problems:
        print(f"    {p}")
    text = json.dumps(doc, indent=1, ensure_ascii=False) + "\n"
    if "--check" in argv:
        with open(OUT, encoding="utf-8") as f:
            stale = f.read() != text
        if stale:
            print(f"  FAIL {os.path.relpath(OUT, ROOT)} is stale; run python3 tools/engineering_fitout.py")
        return 1 if stale or problems else 0
    with open(OUT, "w", encoding="utf-8") as f:
        f.write(text)
    print(f"  wrote {os.path.relpath(OUT, ROOT)}")
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
