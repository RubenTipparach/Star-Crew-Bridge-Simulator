#!/usr/bin/env python3
"""The Tern's armory on deck A (openspec/changes/armory, design section 4).

It owns data/ships/tern/armory.json: a layout patch (schema starcrew.layout-patch/1, applied after
data/ships/tern/engineering.json) that adds the armory, its door from the aft passage, and under "furnishings"
what the mockups place in it (the suite set's rifle rack, armour racks and ammunition cabinet, a workbench, a
locker bank and a shelf).

The owner, 2026-10-08: "did you add armory room somewhere tioo? theres still room for more rooms on top deck".
With the command suite and deck access, deck A is empty aft of the head and the bridge locker on both sides of
the aft passage (z -18 to 0) and beside the dorsal turret's access room. The armory takes the port side's forward
end, z -6 to 0, from the passage out to the hull's line: inside the hull's 0.5 m clearance at the ceiling's chamfer
(10.7 m from the centreline at z -6, 10.9 m at z 0) and within its 1.6 m service band.

It lives in tools/ beside tools/crew_rooms.py and uses command_suite's patch rule, layout check and furniture checks
(every piece inside the room, clear of every door's zone, none overlapping) rather than copies (CLAUDE.md 6.1).
Documentation tooling (CLAUDE.md section 4), standard library only.

Usage: python3 tools/armory.py [--check]
"""
import copy
import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import command_suite as CS  # noqa: E402
import crew_rooms as CR  # noqa: E402
import deck_access as DA  # noqa: E402

ROOT = os.path.dirname(HERE)
OUT = os.path.join(ROOT, "data", "ships", "tern", "armory.json")
ENGINEERING = os.path.join(ROOT, "data", "ships", "tern", "engineering.json")
SUITE_PROPS = os.path.join(ROOT, "assets", "models", "suite", "props.json")

FLOOR_A, CEILING_A = 3.5, 6.5
X0, X1, Z0, Z1 = 1.25, 10.6, -6.0, 0.0      # the room: the aft passage's port wall out to the hull's line
DOOR_Z, DOOR_W, DOOR_H = -3.0, 1.0, 2.2


def patch():
    room = {"id": "armory", "poi": 38, "name": "Armory", "decks": ["A"], "kind": "room", "finish": "working",
            "purpose": ("Sidearm ammunition, six rifles in a locked rack, armour vests and helmets, heavy suits. Its door "
                        "is locked; the security officer and the captain open it from their consoles (armory)."),
            "brushes": [{"y": [FLOOR_A, CEILING_A], "poly": [[X0, Z0], [X1, Z0], [X1, Z1], [X0, Z1]]}]}
    door = {"id": "p_armory", "kind": "door", "between": ["armory", "a_aft_passage"],
            "center_m": [X0, round(FLOOR_A + DOOR_H / 2, 3), DOOR_Z], "normal": [-1.0, 0.0, 0.0], "size_m": [DOOR_W, DOOR_H]}
    return {"compartments": [room], "portals": [door], "stations": [], "fixtures": [], "fixtures_removed": [], "systems": []}


def piece(prop, x, z, yaw, note):
    return {"room": "armory", "prop": prop, "set": "suite", "back_m": [x, FLOOR_A, z], "yaw_deg": yaw, "note": note}


def furnishings():
    return [
        piece("rifle_rack", 3.6, Z0, 0.0, "six rifles behind the locking bar, on the aft wall facing the door"),
        piece("ammo_cabinet", 5.6, Z0, 0.0, "pistol and rifle magazines, on the aft wall"),
        piece("workbench", 7.6, Z0, 0.0, "cleaning and checking weapons, on the aft wall"),
        piece("armour_rack", 4.1, Z1, 180.0, "three vests and helmets, on the forward wall"),
        piece("armour_rack", 6.2, Z1, 180.0, "three vests and helmets, on the forward wall"),
        piece("locker_bank", 8.7, Z1, 180.0, "heavy suits, on the forward wall"),
        piece("shelf", X1, -3.0, -90.0, "spare holsters and slings, on the hull side"),
    ]


def footprints():
    fp = CR.footprints()
    with open(SUITE_PROPS, encoding="utf-8") as f:
        for name, rec in json.load(f)["props"].items():
            b = rec["bounds_m"]
            fp.setdefault(name, (b["min"][0], b["max"][0], b["min"][2], b["max"][2]))
    return fp


def layouts(P):
    """(the layout with deck access and engineering, the same with the armory)."""
    _, _, LA = DA.build()
    with open(ENGINEERING, encoding="utf-8") as f:
        LE = CS.patch_layout(copy.deepcopy(LA), json.load(f))
    return LE, CS.patch_layout(copy.deepcopy(LE), P)


def main(argv):
    P, F = patch(), furnishings()
    _, L = layouts(P)
    ok, report = CS.run_layout_check(L)
    problems = [] if ok else ["the layout check fails with the armory:\n" + report]
    problems += CS.check_furniture(L, F, footprints())
    doc = {"schema": "starcrew.layout-patch/1",
           "status": ("Proposed (2026-10-08): openspec/changes/armory. Applies after data/ships/tern/engineering.json. Written by "
                      "tools/armory.py; edit the script, not this file. The layout does not hold it yet."),
           "_rules": ["The same fields and rules as command_suite.json (ShipKit.applyPatch, tools/command_suite.py patch_layout).",
                      "furnishings: as crew_rooms.json's (back_m on the floor, yaw_deg the way the front faces, 0 the bow, +90 port)."]}
    doc.update(P)
    doc["furnishings"] = F
    text = json.dumps(doc, indent=1, ensure_ascii=False) + "\n"
    c = P["compartments"][0]["brushes"][0]
    print(f"  armory: {(X1 - X0) * (Z1 - Z0):.1f} m2, {(X1 - X0) * (Z1 - Z0) * (c['y'][1] - c['y'][0]):.1f} m3; "
          f"{len(F)} pieces; checks {'ok' if not problems else 'FAIL'}")
    for p in problems:
        print(f"    {p}")
    if "--check" in argv:
        with open(OUT, encoding="utf-8") as f:
            stale = f.read() != text
        if stale:
            print(f"  FAIL {os.path.relpath(OUT, ROOT)} is stale; run python3 tools/armory.py")
        return 1 if stale or problems else 0
    with open(OUT, "w", encoding="utf-8") as f:
        f.write(text)
    print(f"  wrote {os.path.relpath(OUT, ROOT)}")
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
