#!/usr/bin/env python3
"""The Tern's crew rooms furnished: the crew quarters, the mess, damage control and the two turret rooms.

It owns data/ships/tern/crew_rooms.json: where each piece of furniture stands in those rooms (its back on
the floor, the way it faces), for the mockups to place as props (the machinery set's bunks, mess tables and
galley, and the suite set's lockers and workbench). The rooms' purposes in layout.json name the furniture
("Bunks and lockers", "Galley and tables", "Repair kits, extinguishers, EVA suits"); until now the deck plan
drew them empty (owner, 2026-10-06: "anything left over from initial design should be redone").

It lives in tools/ beside tools/command_suite.py because it is the same kind of thing, a furniture list
written and checked by a script, and it uses that script's checks rather than a copy (CLAUDE.md 6.1):
every piece inside its room and clear of every door's zone, no two pieces overlapping. It checks on the
layout as it stands, with the command suite, and with deck access (the stair towers, the lift and the second
doors take floor and add doors), so the furniture fits whichever the owner keeps.
Documentation tooling (CLAUDE.md section 4), standard library only.

Usage: python3 tools/crew_rooms.py [--check]
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

ROOT = os.path.dirname(HERE)
OUT = os.path.join(ROOT, "data", "ships", "tern", "crew_rooms.json")
MACHINERY = os.path.join(ROOT, "assets", "models", "machinery", "props.json")
ENGINEERING = os.path.join(ROOT, "assets", "models", "engineering", "props.json")
FLOOR_B = 0.0

# The machinery set's crew furniture, as briefed (W x H x D, metres; tools/blender/build_machinery_props.py),
# and whether its origin is the centre of its back on a wall or the centre of its footprint.
BRIEF = {"bunk": ((2.1, 2.0, 0.95), "wall"), "mess_table": ((2.0, 0.8, 1.9), "free"), "galley_counter": ((2.4, 2.2, 0.7), "wall")}
SUITE_SET = {"locker_bank", "workbench", "shelf"}


def on_slant(a, b, t):
    """A point t of the way along wall a->b (x, z) and the yaw (degrees) facing off the wall to its left (the
    room's side, when a->b runs aft to forward on the port hull side)."""
    x, z = a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t
    nx, nz = -(b[1] - a[1]), b[0] - a[0]   # left of a->b
    ln = math.hypot(nx, nz)
    return [round(x, 3), FLOOR_B, round(z, 3)], round(math.degrees(math.atan2(nx / ln, nz / ln)), 2)


def prop_set(prop):
    """The prop set that models a piece: the suite's, the engineering set's (its manifest names it) or the machinery set's."""
    if prop in SUITE_SET:
        return "suite"
    if os.path.exists(ENGINEERING):
        with open(ENGINEERING, encoding="utf-8") as f:
            if prop in json.load(f)["props"]:
                return "engineering"
    return "machinery"


def piece(room, prop, back, yaw, note):
    return {"room": room, "prop": prop, "set": prop_set(prop),
            "back_m": [round(v, 3) for v in back], "yaw_deg": yaw, "note": note}


def furnishings():
    F = []
    # Crew quarters (deck B, port): twelve berths in six two-tier bunks, for eight crew and the four of the damage
    # control teams (owner, 2026-10-08: "should have more bunks in crew quarters"), and lockers on the hull side. The
    # walls are full (the doors from the stair tower, the turret room and the spine), so two stand back to back in
    # the middle of the room, 2 cm apart, between aisles of 1.6 m and 2.1 m.
    F.append(piece("quarters", "bunk", [5.0, FLOOR_B, 18.0], 180.0, "two berths on the forward wall"))
    F.append(piece("quarters", "bunk", [7.3, FLOOR_B, 18.0], 180.0, "two berths on the forward wall"))
    F.append(piece("quarters", "bunk", [8.0, FLOOR_B, 8.0], 0.0, "two berths on the aft wall, clear of the door from the turret room"))
    F.append(piece("quarters", "bunk", [1.25, FLOOR_B, 16.4], 90.0, "two berths on the corridor wall, forward of the door"))
    F.append(piece("quarters", "bunk", [6.39, FLOOR_B, 12.5], -90.0, "two berths in the middle of the room, facing the stair tower"))
    F.append(piece("quarters", "bunk", [6.41, FLOOR_B, 12.5], 90.0, "two berths in the middle of the room, facing the hull"))
    for t, note in ((0.42, "lockers on the hull side"), (0.68, "lockers on the hull side")):
        back, yaw = on_slant((10.9, 8.0), (9.7, 18.0), t)
        F.append(piece("quarters", "locker_bank", back, yaw, note))
    # Mess (deck B, starboard): the galley on the forward wall, two tables of four.
    F.append(piece("mess", "galley_counter", [-6.5, FLOOR_B, 18.0], 180.0, "the galley, on the forward wall"))
    F.append(piece("mess", "mess_table", [-6.0, FLOOR_B, 12.0], 0.0, "a table for four"))
    F.append(piece("mess", "mess_table", [-6.0, FLOOR_B, 14.6], 0.0, "a table for four"))
    # Damage control (deck B, starboard forward): a workbench for repair kits, lockers for EVA suits and extinguishers,
    # the board's wall bank on the hull side (a station), and the briefing room's scuttle ladder kept clear.
    F.append(piece("damage_control", "workbench", [-5.0, FLOOR_B, 18.0], 0.0, "repair kits, on the aft wall"))
    F.append(piece("damage_control", "locker_bank", [-7.0, FLOOR_B, 26.0], 180.0, "EVA suits, on the forward wall"))
    F.append(piece("damage_control", "locker_bank", [-2.55, FLOOR_B, 26.0], 180.0, "extinguishers, on the forward wall"))
    # The turret rooms (deck B, port and starboard, 9.75 x 8 m): what keeps a pod's gun and gunner going, which the
    # rooms' purposes name and the deck plan drew as nothing (owner, 2026-10-08: "turret access rooms are empty, they
    # need to have some kind of battery or life support stuff there"). The turret's capacitor bank on the aft wall,
    # its power converter and power panel by the pod's hatch, the pod's air handler and scrubbers on the forward wall,
    # the gunner's lockers and a tool board on the corridor wall; the middle stays open between the three doors.
    for sx, room in ((1, "port_turret"), (-1, "stbd_turret")):
        side = "port" if sx > 0 else "starboard"
        for x in (3.2, 5.4, 7.6):
            F.append(piece(room, "battery_bank", [sx * x, FLOOR_B, 0.0], 0.0, f"the {side} turret's capacitor bank, on the aft wall"))
        F.append(piece(room, "power_converter", [sx * 11.0, FLOOR_B, 6.6], -sx * 90.0, "the turret's power converter, on the hull side forward of the pod's hatch"))
        F.append(piece(room, "local_panel", [sx * 11.0, FLOOR_B, 2.5], -sx * 90.0, "the turret's power panel, aft of the pod's hatch"))
        F.append(piece(room, "ls_air_handler", [sx * 3.0, FLOOR_B, 8.0], 180.0, "the pod's air handler, on the forward wall"))
        F.append(piece(room, "ls_scrubbers", [sx * 8.0, FLOOR_B, 8.0], 180.0, "the pod's scrubbers, on the forward wall"))
        F.append(piece(room, "locker_bank", [sx * 1.25, FLOOR_B, 6.6], sx * 90.0, "the gunner's suit and spares, on the corridor wall"))
        F.append(piece(room, "tool_board", [sx * 1.25, FLOOR_B, 1.6], sx * 90.0, "tools for the gun and the bank, on the corridor wall"))
    return F


def footprints():
    """{prop: (min x, max x, min z, max z)} in prop space: the build's bounds when the set is built, else the brief."""
    fp = CS.footprints()
    man = None
    if os.path.exists(MACHINERY):
        with open(MACHINERY, encoding="utf-8") as f:
            man = json.load(f)["props"]
    # Any other prop of the machinery or engineering sets: its built bounds.
    for path in (MACHINERY, ENGINEERING):
        if os.path.exists(path):
            with open(path, encoding="utf-8") as f:
                for name, rec in json.load(f)["props"].items():
                    b = rec["bounds_m"]
                    fp.setdefault(name, (b["min"][0], b["max"][0], b["min"][2], b["max"][2]))
    for name, ((w, _, d), anchor) in BRIEF.items():
        if man and name in man:
            b = man[name]["bounds_m"]
            fp[name] = (b["min"][0], b["max"][0], b["min"][2], b["max"][2])
        else:
            fp[name] = (-w / 2, w / 2, 0.0, d) if anchor == "wall" else (-w / 2, w / 2, -d / 2, d / 2)
    return fp


# Keep clear: the scuttle ladders' floor under the hatches (deck access), a 1.2 m square.
KEEP_CLEAR = [("damage_control", -7.6, 19.2), ("medbay", 7.6, 19.2)]


def checks(F):
    fp = footprints()
    problems = []
    _, LS, LA = DA.build()
    for name, L in (("today", copy.deepcopy(CS.L0)), ("with the suite", LS), ("with deck access", LA)):
        for p in CS.check_furniture(L, F, fp):
            problems.append(f"{name}: {p}")
    for room, x, z in KEEP_CLEAR:
        sq = [(x - 0.6, z - 0.6), (x + 0.6, z - 0.6), (x + 0.6, z + 0.6), (x - 0.6, z + 0.6)]
        for f in F:
            if f["room"] == room and CS.layout_check.polys_overlap(CS.corners(f, fp), sq):
                problems.append(f"{f['prop']} at {f['back_m']} stands under the scuttle in {room}")
    return problems


def main(argv):
    F = furnishings()
    problems = checks(F)
    doc = {
        "schema": "starcrew.furnishings/1",
        "status": ("Proposed (2026-10-06): the crew rooms furnished for the mockups (openspec/changes/ship-props). "
                   "Written by tools/crew_rooms.py; edit the script, not this file."),
        "_rules": ["back_m: the piece's origin on the floor (a wall piece: the centre of its back on the wall plane; a free one: "
                   "the centre of its footprint). yaw_deg: the way its front faces, 0 the bow, +90 port. set: the prop set that models it."],
        "furnishings": F,
    }
    text = json.dumps(doc, indent=1, ensure_ascii=False) + "\n"
    print(f"  {len(F)} pieces in {len({f['room'] for f in F})} rooms; checks {'ok' if not problems else 'FAIL'}")
    for p in problems:
        print(f"    {p}")
    if "--check" in argv:
        with open(OUT, encoding="utf-8") as f:
            stale = f.read() != text
        if stale:
            print(f"  FAIL {os.path.relpath(OUT, ROOT)} is stale; run python3 tools/crew_rooms.py")
        return 1 if stale or problems else 0
    with open(OUT, "w", encoding="utf-8") as f:
        f.write(text)
    print(f"  wrote {os.path.relpath(OUT, ROOT)}")
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
