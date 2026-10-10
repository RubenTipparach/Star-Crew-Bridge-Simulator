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
from build_machinery_props import revolve  # noqa: E402

OUT = os.path.join(ROOT, "assets", "models", "service_proto")
GENERATOR = "tools/blender/build_service_prototypes.py"
COVERS = os.path.join(ROOT, "assets", "textures", "repairs", "covers.json")
INTERIORS = ("interior_wiring", "interior_circuit")

# The bays (prop space of their machine). frame: the bay's face, local +Z out of the machine, local x across
# the cover, y up it. size_m: the cover (and the rebate it sits in); opening_m: the hole behind; depth_m: the bay.
BAYS = {
    "coolant_pump_svc": {"centre": (0.62, 1.92, 0.0), "yaw": 90.0, "size_m": (0.38, 0.32), "opening_m": (0.31, 0.25),
                         "depth_m": 0.12, "shows": "interior_wiring", "cover": "cover_pump"},
    "local_panel_svc": {"centre": (0.0, 0.85, 0.0), "yaw": 180.0, "size_m": (0.56, 0.455), "opening_m": (0.49, 0.385),
                        "depth_m": 0.12, "shows": "interior_circuit", "cover": "cover_panel"},
}
COVER_T = 0.012       # the cover's thickness, and the rebate's depth: it sits flush
HOLE_R = 0.0042       # a tapped hole in the rebate, under each screw

SCREW_SHANK_M = 0.03   # a screw's shank: the head's underside is this far up its prop space
BUDGETS = {"coolant_pump_svc": 800, "local_panel_svc": 500, "cover_pump": 40, "cover_panel": 40, "service_screw": 90}


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
    p.recess(cuts, f"{label}_bay", m @ Matrix.Translation((0.0, 0.0, -COVER_T)), ow, oh, bay["depth_m"],
             wall_role="machinery", shows=bay["shows"])
    for k, (x, y) in enumerate(hole_points(bay)):
        cuts.append(p.box(f"{label}_tap_{k}", (-HOLE_R, -HOLE_R, -COVER_T - 0.03), (HOLE_R, HOLE_R, 0.02), "machinery",
                          m=m @ Matrix.Translation((x, y, 0.0))))
    p.cut(p.body, f"{label}_bay", cuts)


def coolant_pump_svc():
    """The coolant pump, with a terminal box on the motor's +X flank in place of the small junction box (it
    covers it), and the box's lid bay cut into its +X face."""
    p = eng.coolant_pump()
    p.name = "coolant_pump_svc"
    box = p.box("terminal_box", (0.40, 1.72, -0.24), (0.62, 2.12, 0.24), {"+x": "bulkhead", "*": "machinery"})
    p.union(p.body, "terminal_box", [box])
    cut_bay(p, BAYS["coolant_pump_svc"], "lid")
    p.presents = "Prototype (repairs-on-deck 3c): the coolant pump with its motor terminal box's lid bay built in"
    return p


def local_panel_svc():
    """The local control cabinet with a service door bay in its back (-Z); the sloped instrument face is the
    operating face and is not touched."""
    p = eng.local_panel()
    p.name = "local_panel_svc"
    cut_bay(p, BAYS["local_panel_svc"], "door")
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
        out[name] = {"cover": bay["cover"], "interior": bay["shows"][9:], "size_m": list(bay["size_m"]), "thickness_m": COVER_T,
                     "cover_at_m": r3(home), "normal": r3(n), "up": r3(up),
                     "screws_m": [r3(m @ Vector((x, y, 0.0))) for x, y in hole_points(bay)],
                     "bay_depth_m": bay["depth_m"], "opening_m": list(bay["opening_m"])}
    return {"schema": "starcrew.service-bays/1", "built_by": GENERATOR,
            "_rules": ["Prop space of each machine (+Y up, +Z its front). cover_at_m is the centre of the cover's back when it is home "
                       "in the rebate, normal out of the machine, up the cover's up; the cover's front is thickness_m along normal. "
                       "screws_m are the screws' head undersides when driven home, on the face. interior is the bake of "
                       "repairs-on-deck 3b the bay's recorded floor shows."],
            "machines": out}


STATUS = ("Prototype (2026-10-09) by " + GENERATOR + ", for the owner's review (repairs-on-deck design 3c). Not the shipped "
          "engineering props: those are untouched until the owner approves these.")
RULES = list(eng.RULES) + ["screens with shows interior_wiring or interior_circuit are service bay floors: the page lays that "
                           "interior bake (assets/textures/repairs) on them."]
SET = PropSet("service_proto", "ServicePrototypes", OUT, GENERATOR, PROPS, BUDGETS, STATUS, RULES, __doc__)


if __name__ == "__main__":
    run(SET)
    if "--check" not in sys.argv:
        with open(os.path.join(OUT, "service.json"), "w", encoding="utf-8") as f:
            json.dump(service_json(), f, indent=2)
            f.write("\n")
        print("[props] wrote", os.path.relpath(os.path.join(OUT, "service.json"), ROOT))
