"""Star Crew's panel textures: the wall modules and strips, the ceiling and floor modules, the trim
strips, the platform faces and the chairs' upholstery, modelled in Blender the hard-surface CSG way and
baked to texture layers (openspec/changes/wall-panels, design section 6, option B; ceilings-and-trims;
floor-panels).

It owns the panel layers (assets/textures/panels/<px>/<finish>_<module>.png, <finish>_strips.png,
<finish>_ceiling_<module>.png, <finish>_floor_<module>.png, <finish>_trims.png, <finish>_platforms.png,
upholstery_channel.png and upholstery_panel.png at 256 and 128 px), the reusable screen and key images
(assets/textures/panels/ui_screen_<finish>.png, keys_<finish>.png), their manifest
(assets/textures/panels/manifest.json) and the contact sheet
(docs/screenshots/materials/panels-contact-sheet.png). It lives in tools/blender because the
panels are modelled, not painted: every module is low relief carved with boolean cutters,
chamfered and baked under one fixed light, the way the bridge props are built
(the hard-surface kit, tools/blender/hs_kit.py, whose primitives and Prop class it imports rather
than copies; the blender-hard-surface skill). data/materials/panels.json is the one source for what
is built: sizes, colours, wear, layers and render settings.

Why Blender and not Material Maker (CLAUDE.md section 9 asks for graphs): Material Maker cannot run
in a cloud session (it needs Godot 4.7, and GitHub release downloads return 403 here). Question V2
in the wall-panels design asks the owner which way panels are made from now on.

Run (from anywhere), with Pillow installed beside bpy (pip install bpy pillow):
  <python with the bpy module> tools/blender/build_wall_panels.py [--only crew_plate,working_strips,...]
      [--samples N] [--post-only] [--no-sheet]
  --only       render only these targets (<finish>_<module>, <finish>_strips, <finish>_ui, <finish>_keys,
               <finish>_ceiling_<module>, <finish>_floor_<module>, <finish>_trims, <finish>_platforms,
               upholstery_channel, upholstery_panel);
               the post-process still writes every layer from the raw renders it finds
  --samples    override render.samples (a quick look; the committed layers use panels.json's)
  --post-only  skip rendering; rebuild the layers and the sheet from tools/materials/raw/panels
  --no-sheet   skip the contact sheet

The method, per module (panel space = the bridge props' prop space: x right, y up, z out of the wall,
metres; the module spans x -1..1, y 0..2):
  1. block out a plate that overfills the frame, and the module's pieces (boxes, extruded
     profiles, cylinders, a torus, text and cable curves);
  2. carve recesses, grooves, slots and holes with named cutter collections (Boolean Difference,
     Exact solver, materials transferred from the cutters, so a recess floor comes out dark);
  3. chamfer the raised pieces (an angle-limited Bevel; triangles do not matter in a texture);
  4. render face-on, orthographic, under the key light and ambient of panels.json render, with
     occlusion from the path tracing, then render the emission mask;
  5. post-process (in this file, with tools/materials/postprocess.py's palette reduction and
     sizes): area-average in linear light to 256 and 128 px, encode sRGB, palette-reduce, write the
     mask into alpha.
Strips are rendered 2 m wide with copies of their geometry 2 m to each side and with wear noise
that is periodic in x, so the layer tiles along a wall. Ceiling and floor modules are 2 m cells
seen from the room (panel x is world +x, panel y world +z, the bow), lit nearly square on as a room's
lamps light them (render.ceiling, render.floor). The walkway tiles both ways: its pattern repeats
every 2 m and its wear noise lies on a 4D torus, so it is periodic in x and in y. Trim rows are
rendered like strips, one render a row, and stacked into one 2 m layer per finish at the rows'
places (trims.rows), the gaps between them filled by repeating each row's edge. Platform faces are
built the same way (platforms.rows), each row designed to fit one face height, lit from above
(render.platforms). The upholstery layers have no finish: padded leather modelled as smooth pillows
(a heightfield per pillow, welts as cords), baked neutral in a soft light (render.upholstery) with wear
and leather grain on a 4D torus, so they tile both ways, for a page to tint per chair.

Determinism: Cycles on the CPU with a fixed seed and no denoiser renders the same pixels on a second
run (each pixel's random sequence depends only on its position and sample index), and the
post-process is deterministic for a given Pillow and numpy, so a second build writes the same
bytes. The manifest records each file's sha256 with the Blender, Pillow and numpy versions.
"""
import argparse
import hashlib
import json
import math
import os
import sys

import bpy  # first: with the pip bpy module, bmesh and mathutils exist only once bpy is imported
import bmesh  # noqa: E402
from mathutils import Matrix, Vector  # noqa: E402

import numpy as np  # noqa: E402
from PIL import Image, ImageDraw, ImageFont  # noqa: E402

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(os.path.dirname(HERE))
sys.path.insert(0, HERE)
sys.path.insert(0, os.path.join(ROOT, "tools", "materials"))
import hs_kit as kit  # noqa: E402  the hard-surface kit: prism, obox, ngon, frame, Prop, apply_modifiers
import postprocess as mm  # noqa: E402  the materials' post-process: reduce_palette, to_u8, gpu_bytes, save_png

PANELS_JSON = os.path.join(ROOT, "data", "materials", "panels.json")
MATERIALS_JSON = os.path.join(ROOT, "data", "materials", "materials.json")
RAW = os.path.join(ROOT, "tools", "materials", "raw", "panels")
SHEET = os.path.join(ROOT, "docs", "screenshots", "materials", "panels-contact-sheet.png")
GENERATOR = "tools/blender/build_wall_panels.py"
SCHEMA = "starcrew.panels/1"

MODULES = ("plate", "vent", "pipes", "hatch", "junction", "light", "ribbed", "screen", "flank", "narrow")
STRIPS = ("base", "louvre", "spare", "top")
FINISHES = ("crew", "working")
# Ceilings and floors (ceilings-and-trims design section 2, floor-panels design section 2): the
# catalogue a finish picks from. A ceiling set has all eight; a floor set has walkway and plate and
# any of the rest (crew floors have no hazard module).
CEILING_MODULES = ("plate", "grille", "fan", "cable_tray", "pipes", "hatch", "ribbed", "lamp_surround")
FLOOR_MODULES = ("walkway", "plate", "grate", "access", "trench", "drain", "vent", "hazard")
KIND_MODULES = {"ceiling": CEILING_MODULES, "floor": FLOOR_MODULES}
KIND_REQUIRED = {"ceiling": set(CEILING_MODULES), "floor": {"walkway", "plate"}}
# Trim rows (ceilings-and-trims design section 3), bottom to top in the layer, and what uses them.
TRIM_ROWS = ("side", "baseboard", "rib", "rib_ends", "beam", "frame", "cove")
TRIM_PIECES = ("rib_base", "rib_capital")
TRIM_MEMBERS = ("rib", "beam", "baseboard", "frame", "window_frame", "cove")
# Platform face rows (panels.json platforms.rows), bottom to top in the layer: each fits one face height
# (a 0.45 m riser, a 0.225 m riser, a stair step's front), not a crop of another.
PLATFORM_ROWS = ("riser", "riser_low", "step")
# The upholstery layers (panels.json upholstery.layers), in layer order, and the colour each role they
# use takes from upholstery.colours_srgb. A role not named here is not used by them (check_roles).
UPHOLSTERY = ("channel", "panel")
UPHOLSTERY_ROLE_COLOUR = {"upholstery": "leather", "paint2": "welt", "machinery": "seam", "stencil": "stitch"}
GRAIN_ROLES = {"upholstery", "paint2"}    # leather: the padding and the welts take the grain
TEXEL_64 = 1.0 / 64.0     # a row lands on whole texels at 64 px per metre, so at 128 too

# Material roles. The first ones are the props kit's slot order (kit.ROLES); a panel uses them
# with these meanings, plus its own. kit.ROLES is extended at build time (set_roles), so the kit's
# primitives give every mesh all of them in one fixed order. The kit's upholstery roles are not in this
# table: a wall, ceiling, floor, trim or platform layer never uses them (make_materials marks them
# unused and check_roles refuses a face that has one); the upholstery layers map their own roles
# (UPHOLSTERY_ROLE_COLOUR).
ROLE_COLOUR = {
    "machinery": "dark",      # dark metal: recess floors and walls, grilles, kick plates
    "trim": "metal",          # bare metal: frames, collars, rivets, handles
    "bulkhead": "plate",      # the plate's paint
    "hazard": None,           # hazard stripes, hazard_a and hazard_b
    "light_panel": "light",   # white light (emissive)
    "screen": None,           # the UI display (emissive, the ui image)
    "accent": "status",       # a status lamp (emissive)
    "paint2": "paint2",       # a second, darker paint
    "rubber": "rubber",       # cables, gaskets, a gauge needle
    "stencil": "stencil",     # stencilled marks
    "amber": "amber",         # amber light (emissive)
    "safety": "safety",       # solid safety yellow (a valve wheel)
    "walk": "walk",           # walkway plate (floor-panels), lighter than the deck around it
}
EXTRA_ROLES = ("paint2", "rubber", "stencil", "amber", "safety", "walk")
EMISSIVE = {"light_panel", "screen", "accent", "amber"}
WORN = {"bulkhead", "paint2", "machinery", "trim", "hazard", "safety", "walk"}   # take edge wear and rust
UI_ROLES = ("ui_bg", "ui_dim", "ui_fg", "ui_hi", "ui_alert")

I4 = Matrix.Identity(4)
DATA = None   # panels.json once load_panels has read it (the trim rows' builders read their pieces)


# ----------------------------------------------------------------------------- data

class DataError(SystemExit):
    """A panels.json problem: stops the build with the path and field."""


def fail(msg):
    raise DataError(f"{os.path.relpath(PANELS_JSON, ROOT)}: {msg}")


def keys_exact(obj, known, where):
    if not isinstance(obj, dict):
        fail(f"{where} must be an object")
    extra = sorted(k for k in obj if k not in known and not k.startswith("_"))
    if extra:
        fail(f"{where}: unknown key {', '.join(extra)}")
    missing = sorted(k for k in known if k not in obj)
    if missing:
        fail(f"{where}: missing {', '.join(missing)}")


def num(v, where, lo=None, hi=None):
    if isinstance(v, bool) or not isinstance(v, (int, float)) or not math.isfinite(v):
        fail(f"{where} must be a finite number, got {v!r}")
    if (lo is not None and v < lo) or (hi is not None and v > hi):
        fail(f"{where} must be in {lo}-{hi}, got {v}")
    return float(v)


def colours_ok(C, where, keys, exact=True):
    if exact:
        keys_exact(C, keys, where)
    else:
        extra = sorted(k for k in C if k not in keys and not k.startswith("_"))
        if extra:
            fail(f"{where}: unknown colour {', '.join(extra)}")
    for k, c in C.items():
        if k.startswith("_"):
            continue
        if not (isinstance(c, list) and len(c) == 3):
            fail(f"{where}.{k} must be [r, g, b]")
        for v in c:
            num(v, f"{where}.{k}", 0, 1)


def wear_ok(W, where):
    keys_exact(W, {"grime", "grime_scale_per_m", "crevice", "crevice_m", "edge", "streaks", "rust"}, where)
    for k, v in W.items():
        if not k.startswith("_"):
            num(v, f"{where}.{k}", 0, 100)


def light_ok(r, where):
    keys_exact(r, {"key_light_tangent", "key_angle_deg", "ambient"}, where)
    num(r["ambient"], where + ".ambient", 0, 0.95)
    num(r["key_angle_deg"], where + ".key_angle_deg", 0, 90)
    if len(r["key_light_tangent"]) != 3 or r["key_light_tangent"][2] <= 0:
        fail(f"{where}.key_light_tangent must be [x, y, z] with z above 0 (out of the surface)")


def on_texel(v, where):
    if abs(v / TEXEL_64 - round(v / TEXEL_64)) > 1e-9:
        fail(f"{where} must be a whole number of 1/64 m (a texel at 64 px per metre), got {v}")


def rows_ok(rows, names, span, where):
    """Rows of a stacked layer (trims.rows, platforms.rows): exactly these names, each v0_m and h_m whole
    texels at 64 px per metre, a texel of guard at the layer's top and bottom and two between rows."""
    keys_exact(rows, set(names), where)
    spans = []
    for name in names:
        R = rows[name]
        w = f"{where}.{name}"
        keys_exact(R, {"v0_m", "h_m"}, w)
        num(R["v0_m"], w + ".v0_m", 0, span)
        num(R["h_m"], w + ".h_m", TEXEL_64, span)
        on_texel(R["v0_m"], w + ".v0_m")
        on_texel(R["h_m"], w + ".h_m")
        spans.append((R["v0_m"], R["v0_m"] + R["h_m"], name))
    spans.sort()
    if spans[0][0] < TEXEL_64 or spans[-1][1] > span - TEXEL_64:
        fail(f"{where} must leave at least one texel (1/64 m) of guard at the layer's top and bottom")
    for (a0, a1, an), (b0, b1, bn) in zip(spans, spans[1:]):
        if b0 < a1 + 2 * TEXEL_64 - 1e-9:
            fail(f"{where} {an} and {bn} need at least two texels (2/64 m) of guard between them")


def module_ok(M, where, drawn_ok=True):
    keys_exact(M, {"layer", "weight", "placed", "emissive"}, where)
    num(M["weight"], where + ".weight", 0)
    if (M["placed"] == "draw") != (M["weight"] > 0):
        fail(f"{where}: a drawn module has a positive weight, a module placed by a rule has weight 0")


def load_panels():
    """Read and validate panels.json. Every key the build uses is checked; unknown keys stop it."""
    with open(PANELS_JSON, encoding="utf-8") as f:
        d = json.load(f)
    if d.get("schema") != SCHEMA:
        fail(f"schema must be {SCHEMA!r}")
    keys_exact(d, {"schema", "status", "layers", "bands", "bays", "rule", "glow", "render", "ui", "finishes", "source",
                   "cells", "walkway", "trims", "platforms", "upholstery"}, "top level")
    keys_exact(d["layers"], {"first_layer", "span_m", "margin_m", "sizes_px", "colours", "dir"}, "layers")
    with open(MATERIALS_JSON, encoding="utf-8") as f:
        n_mats = len(json.load(f)["materials"])
    if d["layers"]["first_layer"] != n_mats:
        fail(f"layers.first_layer is {d['layers']['first_layer']}, but materials.json has {n_mats} materials")
    if sorted(d["layers"]["sizes_px"]) != [128, 256]:
        fail("layers.sizes_px must be [128, 256] (64 and 128 px per metre over 2 m)")
    span = d["layers"]["span_m"]
    keys_exact(d["bands"], {"base_m", "module_m", "strip_m", "base", "between", "top"}, "bands")
    keys_exact(d["bays"], {"full_min_m", "full_max_m", "narrow_min_m", "narrow_feature_m"}, "bays")
    keys_exact(d["rule"], {"seed", "hash", "key", "door_kinds"}, "rule")
    keys_exact(d["glow"], {"normal", "red_alert", "emergency"}, "glow")
    for k, v in d["glow"].items():
        num(v, f"glow.{k}", 0, 1)
    r = d["render"]
    keys_exact(r, {"px_per_m", "samples", "mask_samples", "key_light_tangent", "key_angle_deg", "ambient", "bounces",
                   "ceiling", "floor", "trims", "platforms", "upholstery"}, "render")
    num(r["ambient"], "render.ambient", 0, 0.95)
    if len(r["key_light_tangent"]) != 3 or r["key_light_tangent"][2] <= 0:
        fail("render.key_light_tangent must be [x, y, z] with z above 0 (out of the wall)")
    for k in ("ceiling", "floor", "trims", "platforms", "upholstery"):
        light_ok(r[k], f"render.{k}")
    keys_exact(d["ui"], {"screen_m", "screen_px", "keys_m", "keys_px"}, "ui")
    # cells, walkway, trims (ceilings-and-trims, floor-panels)
    keys_exact(d["cells"], {"size_m", "origin_x_m", "margin_m", "core_m"}, "cells")
    num(d["cells"]["core_m"], "cells.core_m", 0, span / 2)
    if num(d["cells"]["size_m"], "cells.size_m", 0) != span:
        fail(f"cells.size_m must equal layers.span_m ({span}): a cell shows one module once")
    num(d["cells"]["margin_m"], "cells.margin_m", 0, span / 2)
    num(d["cells"]["origin_x_m"], "cells.origin_x_m")
    keys_exact(d["walkway"], {"width_m", "min_overlap_m"}, "walkway")
    num(d["walkway"]["width_m"], "walkway.width_m", 0.1, span)
    num(d["walkway"]["min_overlap_m"], "walkway.min_overlap_m", 0, d["walkway"]["width_m"])
    T = d["trims"]
    keys_exact(T, {"stretch_max", "rows", "pieces", "pillar", "members"}, "trims")
    num(T["stretch_max"], "trims.stretch_max", 1.0, 4.0)
    rows_ok(T["rows"], TRIM_ROWS, span, "trims.rows")
    keys_exact(T["pieces"], set(TRIM_PIECES), "trims.pieces")
    for name, Pc in T["pieces"].items():
        w = f"trims.pieces.{name}"
        keys_exact(Pc, {"row", "centre_m", "length_m"}, w)
        if Pc["row"] not in T["rows"]:
            fail(f"{w}.row names no row: {Pc['row']}")
        if abs(num(Pc["centre_m"], w + ".centre_m", -span / 2, span / 2)) + num(Pc["length_m"], w + ".length_m", 0.05, span) / 2 > span / 2:
            fail(f"{w} must lie inside its row's 2 m")
    keys_exact(T["pillar"], {"base_m", "capital_m", "tall_base_m", "tall_capital_m", "min_shaft_m"}, "trims.pillar")
    for k, v in T["pillar"].items():
        num(v, f"trims.pillar.{k}", 0)
    keys_exact(T["members"], set(TRIM_MEMBERS), "trims.members")
    for name, M in T["members"].items():
        keys_exact(M, {"face", "sides"}, f"trims.members.{name}")
        for k in ("face", "sides"):
            if M[k] not in T["rows"]:
                fail(f"trims.members.{name}.{k} names no row: {M[k]}")
    # platform faces (the owner, 2026-10-07) and the chairs' upholstery
    keys_exact(d["platforms"], {"rows"}, "platforms")
    rows_ok(d["platforms"]["rows"], PLATFORM_ROWS, span, "platforms.rows")
    U = d["upholstery"]
    keys_exact(U, {"layers", "colours_srgb", "wear", "grain", "tints_srgb"}, "upholstery")
    keys_exact(U["layers"], set(UPHOLSTERY), "upholstery.layers")
    colours_ok(U["colours_srgb"], "upholstery.colours_srgb", set(UPHOLSTERY_ROLE_COLOUR.values()))
    wear_ok(U["wear"], "upholstery.wear")
    keys_exact(U["grain"], {"cell_m", "relief", "mottle"}, "upholstery.grain")
    num(U["grain"]["cell_m"], "upholstery.grain.cell_m", 0.001, 0.1)
    num(U["grain"]["relief"], "upholstery.grain.relief", 0, 1)
    num(U["grain"]["mottle"], "upholstery.grain.mottle", 0, 1)
    if not U["tints_srgb"] or any(k.startswith("_") for k in U["tints_srgb"]):
        fail("upholstery.tints_srgb must name at least one prop")
    colours_ok(U["tints_srgb"], "upholstery.tints_srgb", set(U["tints_srgb"]))
    keys_exact(d["finishes"], set(FINISHES), "finishes")
    layers_seen = [U["layers"][k] for k in UPHOLSTERY]
    colour_keys = {"plate", "paint2", "dark", "metal", "hazard_a", "hazard_b", "safety", "rubber", "stencil", "light",
                   "amber", "status", "rust", "walk"} | set(UI_ROLES)
    for fn in FINISHES:
        F = d["finishes"][fn]
        w = f"finishes.{fn}"
        keys_exact(F, {"colours_srgb", "wear", "modules", "strips", "ceiling", "floor", "trims", "platforms"}, w)
        colours_ok(F["colours_srgb"], w + ".colours_srgb", colour_keys)
        wear_ok(F["wear"], w + ".wear")
        keys_exact(F["modules"], set(MODULES), w + ".modules")
        for m in MODULES:
            M = F["modules"][m]
            known = {"layer", "weight", "placed", "emissive", "mirrored"} | ({"variant"} if m == "vent" else set())
            keys_exact(M, known, f"{w}.modules.{m}")
            num(M["weight"], f"{w}.modules.{m}.weight", 0)
            if (M["placed"] == "draw") != (M["weight"] > 0):
                fail(f"{w}.modules.{m}: a drawn module has a positive weight, a module placed by a rule has weight 0")
            layers_seen.append(M["layer"])
        keys_exact(F["strips"], {"layer", "guard_m", "rows"}, w + ".strips")
        keys_exact(F["strips"]["rows"], set(STRIPS), w + ".strips.rows")
        if sorted(F["strips"]["rows"].values()) != [0, 1, 2, 3]:
            fail(f"{w}.strips.rows must number the four strips 0-3")
        layers_seen.append(F["strips"]["layer"])
        for kind in ("ceiling", "floor"):
            K = F[kind]
            wk = f"{w}.{kind}"
            keys_exact(K, {"colours_srgb", "wear", "modules"}, wk)
            colours_ok(K["colours_srgb"], wk + ".colours_srgb", colour_keys, exact=False)
            wear_ok(K["wear"], wk + ".wear")
            mods = [m for m in K["modules"] if not m.startswith("_")]
            unknown = sorted(set(mods) - set(KIND_MODULES[kind]))
            if unknown:
                fail(f"{wk}.modules: unknown module {', '.join(unknown)} (the catalogue is {', '.join(KIND_MODULES[kind])})")
            missing = sorted(KIND_REQUIRED[kind] - set(mods))
            if missing:
                fail(f"{wk}.modules: missing {', '.join(missing)}")
            for m in mods:
                module_ok(K["modules"][m], f"{wk}.modules.{m}")
                layers_seen.append(K["modules"][m]["layer"])
            if not any(K["modules"][m]["placed"] == "draw" for m in mods):
                fail(f"{wk}.modules: at least one module must be drawn")
        Tf = F["trims"]
        keys_exact(Tf, {"layer", "colours_srgb", "wear"}, w + ".trims")
        colours_ok(Tf["colours_srgb"], w + ".trims.colours_srgb", colour_keys, exact=False)
        wear_ok(Tf["wear"], w + ".trims.wear")
        layers_seen.append(Tf["layer"])
        keys_exact(F["platforms"], {"layer"}, w + ".platforms")
        layers_seen.append(F["platforms"]["layer"])
    first = d["layers"]["first_layer"]
    if sorted(layers_seen) != list(range(first, first + len(layers_seen))):
        fail(f"panel layers must be numbered {first}-{first + len(layers_seen) - 1} once each, got {sorted(layers_seen)}")
    global DATA
    DATA = d
    return d


def kind_finish(F, kind):
    """A finish as one kind of surface sees it: its colours with the kind's overrides and the kind's
    wear (walls are the finish itself; platform faces take the finish's trim colours and wear)."""
    if kind in ("module", "strip", "keys", "ui"):
        return F
    K = F["trims"] if kind == "platforms" else F[kind]
    C = dict(F["colours_srgb"])
    C.update({k: v for k, v in K["colours_srgb"].items() if not k.startswith("_")})
    return {"colours_srgb": C, "wear": K["wear"], "modules": K.get("modules", {})}


def module_ids(F, kind):
    """The modules a finish has for a ceiling or a floor, in catalogue order."""
    return [m for m in KIND_MODULES[kind] if m in F[kind]["modules"]]


# ----------------------------------------------------------------------------- scene

def lin(c):
    return tuple(kit._srgb_to_linear(v) for v in c)


def B(p):
    """Panel space to Blender (x, y, z to x, -z, y), as kit.PROP_TO_BLENDER does."""
    return Vector((p[0], -p[2], p[1]))


def set_roles():
    if kit.ROLES[-len(EXTRA_ROLES):] != EXTRA_ROLES:
        kit.ROLES = tuple(kit.ROLES) + EXTRA_ROLES


def reset(cfg, samples, res, mask=False):
    """An empty scene: Cycles on the CPU, no denoiser, fixed seed; output linear EXR."""
    bpy.ops.wm.read_factory_settings(use_empty=True)
    sc = bpy.context.scene
    sc.render.engine = "CYCLES"
    cy = sc.cycles
    cy.device = "CPU"
    cy.samples = samples
    cy.use_adaptive_sampling = False
    cy.use_denoising = False
    cy.seed = 0
    cy.use_animated_seed = False
    b = 0 if mask else cfg["bounces"]
    cy.max_bounces = b
    cy.diffuse_bounces = b
    cy.glossy_bounces = 0
    cy.transmission_bounces = 0
    cy.volume_bounces = 0
    cy.transparent_max_bounces = 0
    cy.sample_clamp_indirect = 2.0
    sc.render.resolution_x, sc.render.resolution_y = res
    sc.render.resolution_percentage = 100
    sc.render.film_transparent = False
    sc.render.use_compositing = False
    sc.render.use_sequencer = False
    im = sc.render.image_settings
    im.file_format = "OPEN_EXR"
    im.color_depth = "32"
    im.exr_codec = "ZIP"
    sc.view_settings.view_transform = "Standard"
    world = bpy.data.worlds.new("World")
    world.use_nodes = True
    a = 0.0 if mask else cfg["ambient"]
    world.node_tree.nodes["Background"].inputs["Color"].default_value = (a, a, a, 1.0)
    world.node_tree.nodes["Background"].inputs["Strength"].default_value = 1.0
    sc.world = world
    if not mask:
        # The key light: a flat face (normal z) gets (1 - ambient) of its colour from it, so with
        # the ambient a flat, unoccluded face renders at exactly its colour.
        t = Vector(cfg["key_light_tangent"]).normalized()
        ld = bpy.data.lights.new("Key", "SUN")
        ld.energy = math.pi * (1.0 - cfg["ambient"]) / t.z
        ld.angle = math.radians(cfg["key_angle_deg"])
        ob = bpy.data.objects.new("Key", ld)
        ob.rotation_euler = B(t).to_track_quat("Z", "Y").to_euler()
        sc.collection.objects.link(ob)
    return sc


def camera(cx, cy, width_m, res):
    """Orthographic, face-on: looking into the wall (panel -z), centred on panel (cx, cy)."""
    cd = bpy.data.cameras.new("Cam")
    cd.type = "ORTHO"
    cd.ortho_scale = width_m if res[0] >= res[1] else width_m * res[1] / res[0]
    cd.clip_start = 0.1
    cd.clip_end = 40.0
    cam = bpy.data.objects.new("Cam", cd)
    cam.location = B((cx, cy, 10.0))
    cam.rotation_euler = (math.radians(90), 0.0, 0.0)
    bpy.context.scene.collection.objects.link(cam)
    bpy.context.scene.camera = cam


# ----------------------------------------------------------------------------- materials

class Nodes:
    """A small helper for shader node trees: sockets or constants in, sockets out."""

    def __init__(self, mat):
        mat.use_nodes = True
        self.nt = mat.node_tree
        for n in list(self.nt.nodes):
            self.nt.nodes.remove(n)

    def new(self, kind, **props):
        n = self.nt.nodes.new(kind)
        for k, v in props.items():
            setattr(n, k, v)
        return n

    def feed(self, sock, v):
        if isinstance(v, bpy.types.NodeSocket):
            self.nt.links.new(v, sock)
        else:
            sock.default_value = v

    def math(self, op, a, b=0.0, clamp=False):
        n = self.new("ShaderNodeMath", operation=op, use_clamp=clamp)
        self.feed(n.inputs[0], a)
        self.feed(n.inputs[1], b)
        return n.outputs[0]

    def mix(self, fac, a, b, blend="MIX"):
        n = self.new("ShaderNodeMix", data_type="RGBA", blend_type=blend, clamp_factor=True)
        ins = {s.identifier: s for s in n.inputs}
        self.feed(ins["Factor_Float"], fac)
        self.feed(ins["A_Color"], a)
        self.feed(ins["B_Color"], b)
        return {s.identifier: s for s in n.outputs}["Result_Color"]

    def ramp(self, v, lo, hi):
        n = self.new("ShaderNodeMapRange", clamp=True)
        self.feed(n.inputs[0], v)
        n.inputs[1].default_value = lo
        n.inputs[2].default_value = hi
        n.inputs[3].default_value = 0.0
        n.inputs[4].default_value = 1.0
        return n.outputs[0]

    def noise(self, vec, scale, detail=4.0, rough=0.55):
        """Fractal noise at vec: a socket (3D noise) or (socket, w) for 4D noise on a torus."""
        if isinstance(vec, tuple):
            n = self.new("ShaderNodeTexNoise", noise_dimensions="4D")
            self.feed(n.inputs["Vector"], vec[0])
            self.feed(n.inputs["W"], vec[1])
        else:
            n = self.new("ShaderNodeTexNoise", noise_dimensions="3D")
            self.feed(n.inputs["Vector"], vec)
        n.inputs["Scale"].default_value = scale
        n.inputs["Detail"].default_value = detail
        n.inputs["Roughness"].default_value = rough
        return n.outputs["Fac"]

    def xyz(self, x, y, z):
        n = self.new("ShaderNodeCombineXYZ")
        for i, v in enumerate((x, y, z)):
            self.feed(n.inputs[i], v)
        return n.outputs[0]

    def out(self, shader):
        o = self.new("ShaderNodeOutputMaterial")
        self.nt.links.new(shader, o.inputs["Surface"])


def coords(N, periodic):
    """Panel-space position for wear noise: (x, y, depth). Periodic in x with a 2 m period for
    strips: x is wrapped onto a circle of circumference 2 m, so the noise repeats exactly. With
    periodic "xy" (a walkway, which tiles both ways) x and y are each wrapped onto a circle of
    circumference 2 m, which puts the point on a flat torus in 4D: the noise is 4D and periodic in
    both, with no mirror and no stretch."""
    geo = N.new("ShaderNodeNewGeometry")
    sep = N.new("ShaderNodeSeparateXYZ")
    N.nt.links.new(geo.outputs["Position"], sep.inputs[0])
    X, Yb, Z = sep.outputs[0], sep.outputs[1], sep.outputs[2]   # Blender: y is -depth, z is up
    if not periodic:
        return N.xyz(X, Z, Yb), X, Z
    if periodic == "xy":
        r = 1.0 / math.pi
        tx, ty = N.math("MULTIPLY", X, math.pi), N.math("MULTIPLY", Z, math.pi)
        vec = N.xyz(N.math("MULTIPLY", N.math("COSINE", tx), r), N.math("MULTIPLY", N.math("SINE", tx), r),
                    N.math("MULTIPLY", N.math("COSINE", ty), r))
        return (vec, N.math("ADD", N.math("MULTIPLY", N.math("SINE", ty), r), N.math("MULTIPLY", Yb, 0.5))), X, Z
    th = N.math("MULTIPLY", X, math.pi)
    r = 1.0 / math.pi
    return N.xyz(N.math("MULTIPLY", N.math("COSINE", th), r), N.math("MULTIPLY", N.math("SINE", th), r),
                 N.math("ADD", Z, N.math("MULTIPLY", Yb, 0.5))), X, Z


def worn(N, base, F, role, periodic, streaky=True):
    """Base colour (a socket or an RGBA) under grime, crevice occlusion, edge wear and, for a
    working finish, rust streaks. Returns a colour socket. streaky False (ceilings, floors, trims,
    which do not hang like a wall) turns the streaks and the rust into blotches with no direction."""
    W, C = F["wear"], F["colours_srgb"]
    v, X, Z = coords(N, periodic)
    if not streaky:
        return worn_blotchy(N, base, F, role, v)
    s = W["grime_scale_per_m"]
    col = base
    # grime: soft, large blotches
    g = N.ramp(N.noise(v, s, 6.0, 0.6), 0.38, 0.78)
    col = N.mix(N.math("MULTIPLY", g, W["grime"]), col, (0.0, 0.0, 0.0, 1.0))
    # crevices: occlusion within crevice_m darkens what the ambient already darkens
    ao = N.new("ShaderNodeAmbientOcclusion", samples=8, only_local=False)
    ao.inputs["Distance"].default_value = W["crevice_m"]
    col = N.mix(N.math("MULTIPLY", N.math("SUBTRACT", 1.0, ao.outputs["AO"]), W["crevice"]), col, (0.0, 0.0, 0.0, 1.0))
    if role in WORN and W["edge"] > 0:
        bev = N.new("ShaderNodeBevel", samples=8)
        bev.inputs["Radius"].default_value = 0.006
        geo = N.new("ShaderNodeNewGeometry")
        dot = N.new("ShaderNodeVectorMath", operation="DOT_PRODUCT")
        N.nt.links.new(bev.outputs["Normal"], dot.inputs[0])
        N.nt.links.new(geo.outputs["Normal"], dot.inputs[1])
        edge = N.ramp(dot.outputs["Value"], 0.97, 0.80)          # 1 on an edge, 0 on a flat
        chips = N.ramp(N.noise(v, 9.0, 3.0, 0.7), 0.42, 0.62)
        worn_c = lin([min(1.0, c * 1.25 + 0.06) for c in C["metal"]]) + (1.0,)
        col = N.mix(N.math("MULTIPLY", N.math("MULTIPLY", edge, chips), W["edge"]), col, worn_c)
    if role in WORN and W["streaks"] > 0:
        # Dark grime streaks run down from seams and fittings: vertical noise, gated by patches.
        if periodic:
            sv = N.xyz(N.math("MULTIPLY", N.math("COSINE", N.math("MULTIPLY", X, math.pi)), 7.0),
                       N.math("MULTIPLY", N.math("SINE", N.math("MULTIPLY", X, math.pi)), 7.0),
                       N.math("ADD", N.math("MULTIPLY", Z, 0.7), 11.0))
        else:
            sv = N.xyz(N.math("MULTIPLY", X, 22.0), N.math("ADD", N.math("MULTIPLY", Z, 0.7), 11.0), 3.0)
        streak = N.ramp(N.noise(sv, 1.0, 3.0, 0.5), 0.55, 0.78)
        patch = N.ramp(N.noise(v, 1.1, 2.0, 0.5), 0.40, 0.70)
        col = N.mix(N.math("MULTIPLY", N.math("MULTIPLY", streak, patch), W["streaks"]), col, (0.0, 0.0, 0.0, 1.0))
    if role in WORN and W["rust"] > 0:
        # Streaks run down the wall: noise stretched vertically, gated by larger patches.
        if periodic:
            streak_v = N.xyz(N.math("MULTIPLY", N.math("COSINE", N.math("MULTIPLY", X, math.pi)), 9.0),
                             N.math("MULTIPLY", N.math("SINE", N.math("MULTIPLY", X, math.pi)), 9.0),
                             N.math("MULTIPLY", Z, 0.9))
        else:
            streak_v = N.xyz(N.math("MULTIPLY", X, 28.0), N.math("MULTIPLY", Z, 0.9), 0.0)
        streak = N.ramp(N.noise(streak_v, 1.0, 3.0, 0.5), 0.52, 0.72)
        patch = N.ramp(N.noise(v, 1.4, 2.0, 0.5), 0.45, 0.70)
        amt = N.math("MULTIPLY", N.math("MULTIPLY", streak, patch), W["rust"])
        col = N.mix(amt, col, lin(C["rust"]) + (1.0,))
        speck = N.ramp(N.noise(v, 22.0, 2.0, 0.6), 0.66, 0.74)
        col = N.mix(N.math("MULTIPLY", speck, W["rust"] * 0.6), col, lin(C["rust"]) + (1.0,))
    return col


def worn_blotchy(N, base, F, role, v):
    """worn() for surfaces that do not hang: the same grime, crevices and edge wear, with grime
    blotches in place of streaks and rust in patches and specks."""
    W, C = F["wear"], F["colours_srgb"]
    s = W["grime_scale_per_m"]
    col = N.mix(N.math("MULTIPLY", N.ramp(N.noise(v, s, 6.0, 0.6), 0.38, 0.78), W["grime"]), base, (0.0, 0.0, 0.0, 1.0))
    ao = N.new("ShaderNodeAmbientOcclusion", samples=8, only_local=False)
    ao.inputs["Distance"].default_value = W["crevice_m"]
    col = N.mix(N.math("MULTIPLY", N.math("SUBTRACT", 1.0, ao.outputs["AO"]), W["crevice"]), col, (0.0, 0.0, 0.0, 1.0))
    if role in WORN and W["edge"] > 0:
        bev = N.new("ShaderNodeBevel", samples=8)
        bev.inputs["Radius"].default_value = 0.006
        geo = N.new("ShaderNodeNewGeometry")
        dot = N.new("ShaderNodeVectorMath", operation="DOT_PRODUCT")
        N.nt.links.new(bev.outputs["Normal"], dot.inputs[0])
        N.nt.links.new(geo.outputs["Normal"], dot.inputs[1])
        edge = N.ramp(dot.outputs["Value"], 0.97, 0.80)
        chips = N.ramp(N.noise(v, 9.0, 3.0, 0.7), 0.42, 0.62)
        worn_c = lin([min(1.0, c * 1.25 + 0.06) for c in C["metal"]]) + (1.0,)
        col = N.mix(N.math("MULTIPLY", N.math("MULTIPLY", edge, chips), W["edge"]), col, worn_c)
    if role in WORN and W["streaks"] > 0:
        # scuffs and dirt: small, sharp blotches
        blot = N.ramp(N.noise(v, 6.0, 3.0, 0.6), 0.60, 0.74)
        col = N.mix(N.math("MULTIPLY", blot, W["streaks"]), col, (0.0, 0.0, 0.0, 1.0))
    if role in WORN and W["rust"] > 0:
        patch = N.ramp(N.noise(v, 2.2, 4.0, 0.62), 0.56, 0.74)
        col = N.mix(N.math("MULTIPLY", patch, W["rust"]), col, lin(C["rust"]) + (1.0,))
        speck = N.ramp(N.noise(v, 22.0, 2.0, 0.6), 0.66, 0.74)
        col = N.mix(N.math("MULTIPLY", speck, W["rust"] * 0.6), col, lin(C["rust"]) + (1.0,))
    return col


def diffuse(N, col, normal=None):
    d = N.new("ShaderNodeBsdfDiffuse")
    N.feed(d.inputs["Color"], col)
    d.inputs["Roughness"].default_value = 1.0
    if normal is not None:
        N.feed(d.inputs["Normal"], normal)
    N.out(d.outputs[0])


def leather(N, col, grain):
    """Leather grain on a colour (upholstery.grain): a pebble of Voronoi cells cell_m across, as a bump
    of strength relief and a slight darkening in the cracks between cells, and a soft mottle of mottle
    across a few centimetres. On the 4D torus of coords(N, "xy"), so it tiles both ways. Returns
    (colour, normal)."""
    vec, w = coords(N, "xy")[0]
    vor = N.new("ShaderNodeTexVoronoi", voronoi_dimensions="4D", feature="DISTANCE_TO_EDGE")
    N.feed(vor.inputs["Vector"], vec)
    N.feed(vor.inputs["W"], w)
    vor.inputs["Scale"].default_value = 1.0 / grain["cell_m"]
    crack = N.ramp(vor.outputs["Distance"], 0.0, 0.12)          # 0 in a crack, 1 on a pebble
    bump = N.new("ShaderNodeBump")
    bump.inputs["Strength"].default_value = grain["relief"]
    bump.inputs["Distance"].default_value = 0.0015
    N.feed(bump.inputs["Height"], crack)
    col = N.mix(N.math("MULTIPLY", N.math("SUBTRACT", 1.0, crack), grain["relief"] * 0.35), col, (0.0, 0.0, 0.0, 1.0))
    mot = N.noise((vec, w), 30.0, 3.0, 0.55)
    col = N.mix(N.math("MULTIPLY", N.ramp(mot, 0.35, 0.65), grain["mottle"]), col, (0.0, 0.0, 0.0, 1.0))
    return col, bump.outputs["Normal"]


def emission(N, col, strength=1.0):
    e = N.new("ShaderNodeEmission")
    N.feed(e.inputs["Color"], col)
    e.inputs["Strength"].default_value = strength
    N.out(e.outputs[0])


def make_materials(F, periodic, ui_image=None, ui_rect=None, streaky=True, role_colour=None, grain=None):
    """One material per role for a finish. ui_image (a loaded bpy image) is shown on `screen`
    faces, mapped onto ui_rect (x0, y0, x1, y1 in panel space). streaky: see worn(). role_colour maps
    a role to its colour in F (ROLE_COLOUR unless given); a role it does not name gets a material
    marked unused, which check_roles refuses on any face. grain (upholstery.grain) gives GRAIN_ROLES
    leather grain."""
    C = F["colours_srgb"]
    rc = ROLE_COLOUR if role_colour is None else role_colour
    for role in kit.ROLES:
        m = bpy.data.materials.new(role)
        N = Nodes(m)
        if role not in rc:
            diffuse(N, (0.0, 0.0, 0.0, 1.0))
            m["emissive"] = False
            m["unused"] = True
            continue
        if role in EMISSIVE:
            if role == "screen" and ui_image is not None:
                geo = N.new("ShaderNodeNewGeometry")
                sep = N.new("ShaderNodeSeparateXYZ")
                N.nt.links.new(geo.outputs["Position"], sep.inputs[0])
                x0, y0, x1, y1 = ui_rect
                u = N.math("DIVIDE", N.math("SUBTRACT", sep.outputs[0], x0), x1 - x0)
                vv = N.math("DIVIDE", N.math("SUBTRACT", sep.outputs[2], y0), y1 - y0)
                tex = N.new("ShaderNodeTexImage", interpolation="Linear", extension="EXTEND")
                tex.image = ui_image
                N.nt.links.new(N.xyz(u, vv, 0.0), tex.inputs["Vector"])
                emission(N, tex.outputs["Color"])
            else:
                key = rc[role] or "ui_fg"
                emission(N, lin(C[key]) + (1.0,))
            m["emissive"] = True
            continue
        if role == "hazard":
            geo = N.new("ShaderNodeNewGeometry")
            sep = N.new("ShaderNodeSeparateXYZ")
            N.nt.links.new(geo.outputs["Position"], sep.inputs[0])
            # Diagonal stripes 0.125 m apart along x (16 to a 2 m period, so strips still tile).
            ph = N.math("FRACT", N.math("DIVIDE", N.math("ADD", sep.outputs[0], sep.outputs[2]), 0.125))
            band = N.math("GREATER_THAN", ph, 0.5)
            base = N.mix(band, lin(C["hazard_a"]) + (1.0,), lin(C["hazard_b"]) + (1.0,))
        else:
            base = lin(C[rc[role]]) + (1.0,)
        col = worn(N, base, F, role, periodic, streaky)
        if grain is not None and role in GRAIN_ROLES:
            diffuse(N, *leather(N, col, grain))
        else:
            diffuse(N, col)
        m["emissive"] = False


def check_roles(name):
    """Refuse a panel whose faces use a role its kind does not colour (make_materials marked it unused):
    a wall drawn in an upholstery role, or upholstery in a hazard stripe."""
    for ob in bpy.context.scene.objects:
        if ob.type == "MESH":
            slots = ob.material_slots
            used = {p.material_index for p in ob.data.polygons}
            bad = sorted(slots[i].material.name for i in used if slots[i].material is not None and slots[i].material.get("unused"))
        elif ob.type in ("CURVE", "FONT"):
            bad = sorted(m.name for m in ob.data.materials if m is not None and m.get("unused"))
        else:
            continue
        if bad:
            raise SystemExit(f"[panels] {name}: {ob.name} uses role {', '.join(bad)}, which this kind of layer does not colour")


def mask_materials():
    """The emission mask render: emissive roles glow white, every other surface is black."""
    white = bpy.data.materials.new("#mask_white")
    emission(Nodes(white), (1.0, 1.0, 1.0, 1.0))
    black = bpy.data.materials.new("#mask_black")
    emission(Nodes(black), (0.0, 0.0, 0.0, 1.0))
    for ob in bpy.context.scene.objects:
        if not hasattr(ob, "material_slots"):
            continue
        for slot in ob.material_slots:
            if slot.material is not None and not slot.material.name.startswith("#mask"):
                slot.material = white if slot.material.get("emissive") else black


# ----------------------------------------------------------------------------- modelling kit

class Panel:
    """One module or strip being modelled: the props kit's Prop (cutter collections, booleans) and
    panel-space primitives on top of kit.prism, kit.obox and kit.ngon."""

    def __init__(self, name):
        self.name = name
        self.p = kit.Prop(name, "wall panel", "panel space")
        self.coll = self.p.coll
        self.n = 0

    def _name(self, what):
        self.n += 1
        return f"{self.name}.{what}.{self.n:03d}"

    def box(self, what, lo, hi, roles="bulkhead", bevel=0.0, seg=1, m=None):
        roles = roles if isinstance(roles, dict) else {"*": roles}
        ob = kit.obox(self.coll, self._name(what), lo, hi, m if m is not None else I4, roles)
        if bevel:
            bevel_ob(ob, bevel, seg)
        return ob

    def tilted(self, what, centre, half, tilt_deg, roles, bevel=0.0):
        """A box of half sizes `half` about `centre`, its face normal tipped up by tilt_deg."""
        m = kit.frame(centre, tilt_deg)
        return self.box(what, tuple(-h for h in half), tuple(half), roles, bevel, m=m)

    def cyl(self, what, axis, c, r, lo, hi, role, sides=16, smooth=True, bevel=0.0, seg=1):
        """A cylinder along a panel axis from lo to hi; c is the centre in the other two axes
        (axis x: (y, z); y: (x, z); z: (x, y))."""
        pts = [(math.cos(2 * math.pi * k / sides) * r, math.sin(2 * math.pi * k / sides) * r) for k in range(sides)]
        if axis == "x":
            prof = [(c[1] + a, c[0] + b) for a, b in pts]   # (z, y)
        elif axis == "y":
            prof = [(c[0] + a, c[1] + b) for a, b in pts]   # (x, z)
        else:
            prof = [(c[0] + a, c[1] + b) for a, b in pts]   # (x, y)
        ob = kit.prism(self.coll, self._name(what), prof, axis, lo, hi, role)
        if bevel:
            bevel_ob(ob, bevel, seg)
        if smooth:
            for poly in ob.data.polygons:
                n = kit.P(poly.normal)
                along = {"x": abs(n.x), "y": abs(n.y), "z": abs(n.z)}[axis]
                poly.use_smooth = along < 0.5
        return ob

    def ngon_prism(self, what, cx, cy, r, sides, z0, z1, role, bevel=0.0):
        ob = kit.prism(self.coll, self._name(what), kit.ngon(cx, cy, r, sides), "z", z0, z1, role)
        if bevel:
            bevel_ob(ob, bevel)
        return ob

    def torus(self, what, c, R, r, role, segs=28, ring=10):
        """A ring facing the viewer (its axis along panel z), centred on c = (x, y, z)."""
        bm = bmesh.new()
        rings = []
        for i in range(segs):
            a = 2 * math.pi * i / segs
            vs = []
            for j in range(ring):
                b = 2 * math.pi * j / ring
                rad = R + r * math.cos(b)
                vs.append(bm.verts.new(Vector((c[0] + rad * math.cos(a), c[1] + rad * math.sin(a), c[2] + r * math.sin(b)))))
            rings.append(vs)
        for i in range(segs):
            for j in range(ring):
                f = bm.faces.new((rings[i][j], rings[(i + 1) % segs][j], rings[(i + 1) % segs][(j + 1) % ring], rings[i][(j + 1) % ring]))
                f.material_index = kit.ROLES.index(role)
                f.smooth = True
        return kit._object(bm, self._name(what), self.coll)

    def poly(self, what, pts, z0, z1, role, bevel=0.0):
        """A flat profile (x, y) extruded along z."""
        ob = kit.prism(self.coll, self._name(what), pts, "z", z0, z1, role)
        if bevel:
            bevel_ob(ob, bevel)
        return ob

    def text(self, body, x, y, size, role="stencil", z=0.0, align="LEFT"):
        """Stencilled marks: a font curve laid on a face at height z, raised 0.4-2 mm off it."""
        cu = bpy.data.curves.new(self._name("text"), "FONT")
        cu.body = body
        cu.size = size
        cu.align_x = align
        cu.extrude = 0.0008
        cu.materials.append(bpy.data.materials[role])
        ob = bpy.data.objects.new(cu.name, cu)
        ob.rotation_euler = (math.radians(90), 0.0, 0.0)
        ob.location = B((x, y, z + 0.0012))
        self.coll.objects.link(ob)
        return ob

    def cable(self, what, pts, r, role="rubber"):
        cu = bpy.data.curves.new(self._name(what), "CURVE")
        cu.dimensions = "3D"
        cu.bevel_depth = r
        cu.bevel_resolution = 3
        cu.use_fill_caps = True
        sp = cu.splines.new("BEZIER")
        sp.bezier_points.add(len(pts) - 1)
        for bp, p in zip(sp.bezier_points, pts):
            bp.co = B(p)
            bp.handle_left_type = bp.handle_right_type = "AUTO"
        cu.materials.append(bpy.data.materials[role])
        ob = bpy.data.objects.new(cu.name, cu)
        self.coll.objects.link(ob)
        return ob

    def cut(self, target, label, objs):
        return self.p.cut(target, label, [o for o in objs if o is not None])

    def union(self, target, label, objs):
        return self.p.union(target, label, objs)

    def recess(self, x0, y0, x1, y1, depth, floor="machinery", walls="machinery"):
        """A cutter for a recess `depth` into a face at z = 0, reaching 5 cm out of it."""
        return self.box("recess", (x0, y0, -depth), (x1, y1, 0.05), {"-z": floor, "*": walls})


def cplate(P, what, x0, y0, x1, y1, c, z0, z1, role, corners="tl,tr,bl,br", bevel=0.005):
    """A plate with 45 degree chamfered corners (the panel shape of the references: not a square),
    extruded from z0 to z1. corners lists which corners are cut."""
    cs = set(corners.split(","))
    pts = []
    for (x, y, k, d1, d2) in ((x0, y0, "bl", (0, c), (c, 0)), (x1, y0, "br", (-c, 0), (0, c)),
                              (x1, y1, "tr", (0, -c), (-c, 0)), (x0, y1, "tl", (c, 0), (0, -c))):
        if k in cs:
            pts += [(x + d1[0], y + d1[1]), (x + d2[0], y + d2[1])]
        else:
            pts.append((x, y))
    return P.poly(what, pts, z0, z1, role, bevel=bevel)


def bevel_ob(ob, width, seg=1, angle=30.0):
    m = ob.modifiers.new("chamfer", "BEVEL")
    m.width = width
    m.segments = seg
    m.limit_method = "ANGLE"
    m.angle_limit = math.radians(angle)
    m.use_clamp_overlap = True
    m.harden_normals = False
    kit.apply_modifiers(ob)
    return ob


def bolts(P, pts, r=0.014, z0=-0.01, z1=0.012, role="trim"):
    return [P.cyl("bolt", "z", (x, y), r, z0, z1, role, sides=10, bevel=min(0.004, r * 0.35))
            for x, y in pts]


def keypad(P, x0, y0, cols, rows, key, pitch, z_face, lit=(), bezel_role="paint2", key_role="trim"):
    """A block of square keys on a dark well in a raised bezel: the one keypad every module and the
    keys image use. (x0, y0) is the bezel's lower left; lit maps (col, row) to an emissive role.
    Returns the bezel (to be cut by its well) and the keys."""
    m = pitch - key
    w, h = cols * pitch + m + 0.03, rows * pitch + m + 0.03
    bez = P.box("keypad_bezel", (x0, y0, z_face - 0.03), (x0 + w, y0 + h, z_face + 0.015), bezel_role, bevel=0.006)
    well = P.box("keypad_well", (x0 + 0.015, y0 + 0.015, z_face + 0.005), (x0 + w - 0.015, y0 + h - 0.015, z_face + 0.1), "machinery")
    P.cut(bez, "keypad_well", [well])
    lit = dict(lit)
    keys = []
    for r in range(rows):
        for c in range(cols):
            kx = x0 + 0.015 + m / 2 + c * pitch
            ky = y0 + 0.015 + m / 2 + r * pitch
            role = lit.get((c, r), key_role)
            keys.append(P.box("key", (kx, ky, z_face - 0.01), (kx + key, ky + key, z_face + 0.017), role,
                              bevel=min(0.004, key * 0.18), seg=2))
    return bez, keys


def seams(P, x0=-1.7, x1=1.7, ys=(0.004, 1.982), w=0.014, d=0.012):
    """Cutters for the plate joints at a module's top and bottom edges."""
    return [P.box("seam", (x0, y, -d), (x1, y + w, 0.05), "machinery") for y in ys]


def grooves(P, x0, y0, x1, y1, w=0.012, d=0.012, mids=(), z=0.0):
    """Cutters for a grooved rectangle (a plate outline) in a face at height z, with optional
    horizontal seams at mids."""
    h = w / 2
    lo, hi = z - d, z + 0.05
    out = [P.box("groove", (x0 - h, y0 - h, lo), (x1 + h, y0 + h, hi), "machinery"),
           P.box("groove", (x0 - h, y1 - h, lo), (x1 + h, y1 + h, hi), "machinery"),
           P.box("groove", (x0 - h, y0 - h, lo), (x0 + h, y1 + h, hi), "machinery"),
           P.box("groove", (x1 - h, y0 - h, lo), (x1 + h, y1 + h, hi), "machinery")]
    for y in mids:
        out.append(P.box("groove", (x0 - h, y - h, lo), (x1 + h, y + h, hi), "machinery"))
    return out


def octagon_grille(P, plate_cuts, cx, cy, r, depth=0.08, bar=0.012, pitch=0.045, ring_w=0.05):
    """A stacked-octagon vent (X2): an octagonal recess with horizontal bars and a raised ring."""
    plate_cuts.append(P.ngon_prism("oct_cut", cx, cy, r, 8, -depth, 0.05, "machinery"))
    n = int((2 * r) / pitch)
    for k in range(n):
        y = cy - r + pitch * (k + 0.5)
        P.box("oct_bar", (cx - r - 0.02, y - bar / 2, -depth * 0.6), (cx + r + 0.02, y + bar / 2, -depth * 0.45), "machinery", bevel=0.003)
    P.box("oct_bar", (cx - bar / 2, cy - r - 0.02, -depth * 0.5), (cx + bar / 2, cy + r + 0.02, -depth * 0.3), "trim", bevel=0.003)
    ring = P.ngon_prism("oct_ring", cx, cy, r + ring_w, 8, -0.02, 0.022, "trim")
    P.cut(ring, "oct_ring_hole", [P.ngon_prism("oct_ring_cut", cx, cy, r, 8, -0.1, 0.1, "machinery")])
    bevel_ob(ring, 0.006)
    return ring


def slats(P, x0, x1, y0, y1, n, z, tilt, role="paint2", thick=0.006):
    pitch = (y1 - y0) / n
    for k in range(n):
        yc = y0 + pitch * (k + 0.5)
        P.tilted("slat", (0.5 * (x0 + x1), yc, z), ((x1 - x0) / 2, pitch * 0.55, thick), tilt, role, bevel=0.002)


# ----------------------------------------------------------------------------- modules
# Panel space: x right, y up, z out of the wall, metres. A module spans x -1..1, y 0..2; its
# features stay inside x -0.8..0.8 (0.2 m plain margins, design section 1).

def plate_slab(P, strip=False, role="bulkhead"):
    if strip:
        return P.box("plate", (-3.7, -0.4, -0.25), (3.7, 0.9, 0.0), role)
    return P.box("plate", (-1.7, -0.6, -0.25), (1.7, 2.6, 0.0), role)


def m_plate(P, F, spec):
    """The rest panel: a raised lower plate with chamfered shoulders, two upper plates, rivets, a
    riveted repair patch, a data plate and stencilled marks."""
    plate = plate_slab(P)
    X0, X1, Y0, Ym, Y1, Xs = -0.76, 0.76, 0.08, 1.12, 1.92, 0.14
    P.cut(plate, "grooves", seams(P) + grooves(P, X0, Ym + 0.04, X1, Y1)
          + [P.box("groove", (Xs - 0.006, Ym + 0.04, -0.012), (Xs + 0.006, Y1, 0.05), "machinery")])
    cplate(P, "lower_plate", X0, Y0, X1, Ym, 0.12, -0.05, 0.008, "paint2", corners="tl,tr")
    pts = []
    for k in range(9):
        pts.append((X0 + 0.07 + k * (X1 - X0 - 0.14) / 8, Y0 + 0.05, 0.008))
    for x in (X0 + 0.05, X1 - 0.05):
        for y in (0.30, 0.55, 0.80):
            pts.append((x, y, 0.008))
    for x0, x1 in ((X0, Xs), (Xs, X1)):
        for x in (x0 + 0.06, x1 - 0.06):
            for y in (Ym + 0.10, Y1 - 0.06):
                pts.append((x, y, 0.0))
    for x, y, z in pts:
        P.cyl("rivet", "z", (x, y), 0.013, z - 0.01, z + 0.009, "trim", sides=10, bevel=0.005)
    # a riveted repair patch on the upper plate
    P.box("patch", (-0.58, 1.38, -0.01), (-0.30, 1.58, 0.006), "bulkhead", bevel=0.004)
    for x in (-0.555, -0.325):
        for y in (1.405, 1.555):
            P.cyl("rivet", "z", (x, y), 0.01, 0.0, 0.013, "trim", sides=8, bevel=0.004)
    P.text("PNL 2B-07", Xs + 0.08, Y1 - 0.2, 0.075)
    P.text("FR 12", X0 + 0.10, Ym - 0.26, 0.06, z=0.008)
    P.box("data_plate", (0.30, 0.30, -0.01), (0.62, 0.44, 0.017), "trim", bevel=0.004)
    bolts(P, [(0.33, 0.37), (0.59, 0.37)], r=0.009, z0=0.0, z1=0.021, role="paint2")
    P.box("data_lines", (0.36, 0.34, 0.0), (0.56, 0.355, 0.0185), "machinery")
    P.box("data_lines", (0.36, 0.38, 0.0), (0.52, 0.395, 0.0185), "machinery")
    if F["wear"]["rust"] > 0:
        P.box("tag", (0.40, 1.24, -0.01), (0.64, 1.34, 0.004), "hazard", bevel=0.002)


def m_vent(P, F, spec):
    plate = plate_slab(P)
    cuts = seams(P)
    if spec.get("variant") == "octagons":
        for cy in (0.42, 1.0, 1.58):
            octagon_grille(P, cuts, 0.0, cy, 0.24)
        P.cut(plate, "grilles", cuts)
        for side in (-1, 1):
            P.box("hazard_rail", (side * 0.40 - 0.04, 0.12, -0.01), (side * 0.40 + 0.04, 1.88, 0.004), "hazard", bevel=0.002)
        bolts(P, [(sx * 0.40, y) for sx in (-1, 1) for y in (0.08, 1.92)])
        P.text("VENT 07", -0.62, 1.94, 0.055)
        return
    cuts.append(P.recess(-0.5, 0.42, 0.5, 1.42, 0.10))
    P.cut(plate, "grille", cuts)
    sur = cplate(P, "surround", -0.70, 0.24, 0.70, 1.66, 0.14, -0.04, 0.012, "paint2")
    P.cut(sur, "surround_open", [P.box("sur_cut", (-0.5, 0.42, -0.1), (0.5, 1.42, 0.1), "machinery")])
    cplate(P, "sign", -0.30, 0.04, 0.30, 0.16, 0.03, -0.01, 0.005, "bulkhead", bevel=0.003)
    frame = P.box("frame", (-0.58, 0.34, -0.02), (0.58, 1.50, 0.03), "trim")
    P.cut(frame, "frame_open", [P.box("frame_cut", (-0.5, 0.42, -0.1), (0.5, 1.42, 0.1), "machinery")])
    bevel_ob(frame, 0.008)
    slats(P, -0.52, 0.52, 0.44, 1.40, 11, -0.05, 50.0)
    P.box("mesh", (-0.52, 0.42, -0.095), (0.52, 1.42, -0.085), "machinery")
    bolts(P, [(sx * 0.54, y) for sx in (-1, 1) for y in (0.38, 1.46)], z0=0.0, z1=0.042)
    P.text("VENT 04", -0.56, 1.545, 0.06, z=0.012)
    P.text("KEEP CLEAR", -0.25, 0.075, 0.05, z=0.005)


def m_pipes(P, F, spec):
    plate = plate_slab(P)
    cuts = seams(P) + [P.recess(-0.56, -0.2, 0.30, 2.2, 0.06, floor="paint2", walls="machinery")]
    P.cut(plate, "channel", cuts)
    xa, xb = -0.30, 0.06
    P.cyl("pipe", "y", (xa, 0.005), 0.085, -0.2, 2.2, "trim")
    P.cyl("pipe", "y", (xb, -0.005), 0.05, -0.2, 2.2, "machinery")
    for y in (0.12, 1.88):
        P.cyl("flange", "y", (xa, 0.005), 0.115, y - 0.02, y + 0.02, "trim", sides=20, bevel=0.006)
        P.cyl("flange", "y", (xb, -0.005), 0.07, y - 0.015, y + 0.015, "trim", sides=16, bevel=0.004)
    for y in (0.42, 1.0, 1.55):
        P.cyl("collar", "y", (xa, 0.005), 0.097, y - 0.03, y + 0.03, "paint2", sides=20, bevel=0.006)
        P.cyl("collar", "y", (xb, -0.005), 0.061, y - 0.025, y + 0.025, "paint2", sides=16, bevel=0.004)
        P.box("bracket", (xa - 0.02, y - 0.03, -0.06), (xb + 0.02, y + 0.03, -0.02), "machinery")
        bolts(P, [(xa - 0.13, y), (xb + 0.09, y)], r=0.012, z0=-0.065, z1=-0.045)
    P.cyl("marker", "y", (xa, 0.005), 0.088, 0.66, 0.74, "hazard", sides=20)
    P.cyl("marker", "y", (xb, -0.005), 0.053, 0.76, 0.81, "stencil", sides=16)
    # the valve: a stem out of the big pipe and a wheel facing the room
    yv = 1.27
    P.cyl("valve_body", "y", (xa, 0.005), 0.11, yv - 0.07, yv + 0.07, "paint2", sides=12, bevel=0.01)
    P.cyl("stem", "z", (xa, yv), 0.022, 0.0, 0.21, "trim", sides=12)
    P.torus("wheel", (xa, yv, 0.21), 0.12, 0.015, "safety")
    for a in (0.0, 60.0, 120.0):
        P.box("spoke", (-0.115, -0.008, -0.006), (0.115, 0.008, 0.006), "safety",
              m=Matrix.Translation(Vector((xa, yv, 0.21))) @ Matrix.Rotation(math.radians(a), 4, "Z"))
    P.cyl("hub", "z", (xa, yv), 0.03, 0.19, 0.225, "trim", sides=12, bevel=0.006)
    # a gauge on a stub from the small pipe
    yg = 1.45
    P.cyl("stub", "x", (yg, -0.005), 0.016, xb, 0.20, "trim", sides=10)
    P.cyl("gauge", "z", (0.24, yg), 0.068, -0.02, 0.06, "trim", sides=24, bevel=0.008)
    P.cyl("gauge_face", "z", (0.24, yg), 0.054, 0.0, 0.064, "stencil", sides=24)
    P.poly("needle", [(0.236, yg - 0.004), (0.272, yg + 0.035), (0.244, yg + 0.003)], 0.06, 0.068, "rubber")
    P.box("red_arc", (0.255, yg - 0.045, 0.06), (0.285, yg - 0.035, 0.0655), "safety")
    # the plates beside the channel, and a sensor box wired into the small pipe
    P.cut(plate, "right_seams", [P.box("groove", (0.30, 1.05, -0.012), (0.80, 1.062, 0.05), "machinery"),
                                 P.box("groove", (0.30, 0.55, -0.012), (0.80, 0.562, 0.05), "machinery")])
    cplate(P, "sensor", 0.36, 0.70, 0.56, 0.92, 0.03, -0.01, 0.06, "paint2")
    P.box("sensor_lamp", (0.40, 0.86, 0.05), (0.46, 0.88, 0.064), "amber", bevel=0.003)
    P.cable("cable", [(0.36, 0.78, 0.03), (0.22, 0.76, 0.04), (0.11, 0.80, 0.03)], 0.01)
    # a thin conduit up the right with clips, and its tag
    P.cyl("conduit", "y", (0.62, 0.02), 0.02, -0.2, 2.2, "machinery", sides=10)
    for y in (0.3, 0.95, 1.6):
        P.box("clip", (0.59, y - 0.015, -0.01), (0.65, y + 0.015, 0.046), "trim", bevel=0.003)
    P.text("CLT-3", 0.36, 1.84, 0.05)
    P.text("CLT-3", 0.36, 0.22, 0.05)


def m_hatch(P, F, spec):
    plate = plate_slab(P)
    bw = 0.10 if F["wear"]["rust"] > 0 else 0.055
    hx0, hy0, hx1, hy1 = -0.40, 0.42, 0.40, 1.30
    g = 0.02
    cuts = seams(P) + [P.recess(hx0 - g, hy0 - g, hx1 + g, hy1 + g, 0.04)]
    P.cut(plate, "hatch_gap", cuts)
    sur = cplate(P, "surround", -0.64, 0.24, 0.78, 1.56, 0.16, -0.04, 0.010, "bulkhead", corners="tl,tr,br")
    P.cut(sur, "surround_open", [P.box("sur_cut", (hx0 - g - bw, hy0 - g - bw, -0.1), (hx1 + g + bw, hy1 + g + bw, 0.1), "machinery")])
    ring = P.box("hazard_ring", (hx0 - g - bw, hy0 - g - bw, -0.03), (hx1 + g + bw, hy1 + g + bw, 0.004), "hazard")
    P.cut(ring, "ring_open", [P.box("ring_cut", (hx0 - g, hy0 - g, -0.1), (hx1 + g, hy1 + g, 0.1), "machinery")])
    bevel_ob(ring, 0.003)
    hatch = P.box("hatch", (hx0, hy0, -0.06), (hx1, hy1, 0.014), "paint2")
    P.cut(hatch, "handle_pocket", [P.box("pocket", (0.20, 0.80, -0.016), (0.34, 0.92, 0.1), "machinery")])
    bevel_ob(hatch, 0.007)
    P.box("handle", (0.215, 0.84, -0.01), (0.325, 0.88, 0.012), "trim", bevel=0.004)
    P.box("inner_rib", (hx0 + 0.08, hy0 + 0.08, 0.0), (hx1 - 0.08, hy0 + 0.11, 0.019), "paint2", bevel=0.003)
    P.box("inner_rib", (hx0 + 0.08, hy1 - 0.11, 0.0), (hx1 - 0.08, hy1 - 0.08, 0.019), "paint2", bevel=0.003)
    for y in (0.56, 1.16):
        P.box("hinge", (hx0 - 0.05, y - 0.06, -0.01), (hx0 + 0.04, y + 0.06, 0.03), "trim", bevel=0.005)
        P.cyl("pin", "y", (hx0 - 0.005, 0.033), 0.012, y - 0.075, y + 0.075, "trim", sides=10)
    bolts(P, [(x, y) for x in (hx0 + 0.05, hx1 - 0.05) for y in (hy0 + 0.05, hy1 - 0.05)], z0=0.0, z1=0.024)
    P.text("ACCESS 2B", hx0, hy1 + g + bw + 0.05, 0.065, z=0.010)
    P.text("PULL", 0.19, 0.95, 0.04, z=0.014)
    # an access keypad beside the hatch (owner, 2026-10-05: keypads where they fit)
    kx = hx1 + g + bw + 0.05
    keypad(P, kx, 0.70, 3, 4, 0.034, 0.044, 0.010, lit={(2, 3): "accent", (0, 3): "amber"})
    P.box("keypad_lamp", (kx + 0.02, 0.94, 0.0), (kx + 0.13, 0.96, 0.024), "amber", bevel=0.003)


def m_junction(P, F, spec):
    plate = plate_slab(P)
    P.cut(plate, "seams", seams(P) + grooves(P, -0.62, 0.08, 0.62, 1.92))
    cplate(P, "backing", -0.46, 0.52, 0.46, 1.58, 0.10, -0.04, 0.012, "paint2")
    bolts(P, [(x, y) for x in (-0.40, 0.40) for y in (0.60, 1.50)], r=0.011, z0=0.0, z1=0.024)
    bx0, by0, bx1, by1, bz = -0.30, 0.70, 0.30, 1.40, 0.12
    box = P.box("box", (bx0, by0, -0.03), (bx1, by1, bz), "paint2")
    P.cut(box, "lid", grooves(P, bx0 + 0.035, by0 + 0.035, bx1 - 0.035, by1 - 0.035, w=0.008, d=0.008, z=bz))
    bevel_ob(box, 0.01)
    bolts(P, [(x, y) for x in (bx0 + 0.07, bx1 - 0.07) for y in (by0 + 0.07, by1 - 0.07)], z0=bz - 0.01, z1=bz + 0.014)
    P.text("JB-14", bx0 + 0.11, by1 - 0.14, 0.06, z=bz)
    P.box("sticker", (bx1 - 0.15, by1 - 0.155, bz - 0.01), (bx1 - 0.08, by1 - 0.085, bz + 0.0025), "hazard")
    # a button block on the lid (owner, 2026-10-05)
    keypad(P, -0.12, by0 + 0.10, 3, 2, 0.044, 0.056, bz, lit={(0, 1): "accent", (2, 1): "amber"})
    P.box("status_pill", (0.10, by0 + 0.22, bz - 0.01), (0.20, by0 + 0.245, bz + 0.012), "accent", bevel=0.006, seg=2)
    # conduits up into the top band and down into the base band, with fittings and clips
    for x, r in ((-0.18, 0.022), (-0.06, 0.022), (0.08, 0.03)):
        P.cyl("conduit", "y", (x, 0.045), r, by1 - 0.02, 2.2, "machinery", sides=12)
        P.cyl("fitting", "y", (x, 0.045), r + 0.012, by1 - 0.01, by1 + 0.06, "trim", sides=12, bevel=0.004)
    for x, r in ((-0.14, 0.03), (0.14, 0.03)):
        P.cyl("conduit", "y", (x, 0.045), r, -0.2, by0 + 0.02, "machinery", sides=12)
        P.cyl("fitting", "y", (x, 0.045), r + 0.012, by0 - 0.06, by0 + 0.01, "trim", sides=12, bevel=0.004)
    for y in (1.72, 0.32):
        P.box("clip", (-0.24, y - 0.018, -0.01), (0.16, y + 0.018, 0.085), "trim", bevel=0.004)
    P.cable("cable", [(0.28, 1.06, 0.06), (0.42, 0.98, 0.07), (0.52, 0.66, 0.05), (0.55, 0.22, 0.02), (0.555, 0.06, 0.0)], 0.014)
    P.cable("cable", [(0.28, 0.92, 0.04), (0.38, 0.80, 0.05), (0.43, 0.45, 0.04), (0.45, 0.10, 0.0)], 0.011)
    P.cyl("grommet", "z", (0.555, 0.07), 0.03, -0.01, 0.012, "machinery", sides=12, bevel=0.005)
    P.cyl("grommet", "z", (0.45, 0.11), 0.026, -0.01, 0.012, "machinery", sides=12, bevel=0.005)
    P.text("440", bx0 + 0.11, by0 + 0.04, 0.045, z=bz)


def m_light(P, F, spec):
    plate = plate_slab(P)
    P.cut(plate, "column", seams(P) + [P.recess(-0.15, 0.17, 0.15, 1.83, 0.07)])
    for side in (-1, 1):
        xa, xb = sorted((side * 0.27, side * 0.62))
        side_plate = cplate(P, "side_plate", xa, 0.16, xb, 1.84, 0.08, -0.04, 0.014, "paint2",
                            corners="tr,br" if side > 0 else "tl,bl")
        perf = [P.cyl("perf", "z", (side * 0.445, 0.30 + k * 0.14), 0.024, -0.02, 0.05, "machinery", sides=12, smooth=False)
                for k in range(10)]
        P.cut(side_plate, "perforations", perf)
    frame = P.box("frame", (-0.22, 0.10, -0.02), (0.22, 1.90, 0.025), "trim")
    P.cut(frame, "frame_open", [P.box("frame_cut", (-0.15, 0.17, -0.1), (0.15, 1.83, 0.1), "machinery")])
    bevel_ob(frame, 0.008)
    n, y0, y1 = 9, 0.21, 1.79
    pitch = (y1 - y0) / n
    for k in range(n):
        y = y0 + k * pitch
        P.box("pill_seat", (-0.13, y, -0.08), (0.13, y + pitch - 0.012, -0.045), "machinery", bevel=0.004)
        P.box("pill", (-0.10, y + 0.022, -0.07), (0.10, y + pitch - 0.034, -0.028), "light_panel", bevel=0.018, seg=3)
    bolts(P, [(sx * 0.19, y) for sx in (-1, 1) for y in (0.135, 1.865)], r=0.011, z0=0.0, z1=0.034)
    P.text("LUM 3", 0.25, 0.12, 0.05)


def m_ribbed(P, F, spec):
    plate = plate_slab(P)
    P.cut(plate, "seams", seams(P))
    n, y0, y1, gap = 8, 0.10, 1.90, 0.022
    pitch = (y1 - y0) / n
    for k in range(n):
        ya, yb = y0 + k * pitch + gap / 2, y0 + (k + 1) * pitch - gap / 2
        blk = P.box("block", (-0.62, ya, -0.05), (0.74, yb, 0.035), "bulkhead" if k % 3 else "paint2")
        xs = (-0.40, 0.05, 0.50) if k % 2 == 0 else (-0.18, 0.28)
        P.cut(blk, "sockets", [P.cyl("socket", "z", (x, (ya + yb) / 2), 0.038, 0.01, 0.1, "machinery", sides=14, smooth=False) for x in xs])
        bevel_ob(blk, 0.012)
        for x in xs:
            P.cyl("socket_pin", "z", (x, (ya + yb) / 2), 0.014, -0.01, 0.018, "trim", sides=10)
    P.cyl("conduit", "y", (-0.71, 0.035), 0.034, -0.2, 2.2, "machinery", sides=12)
    P.cyl("conduit", "y", (-0.665, 0.02), 0.016, -0.2, 2.2, "trim", sides=10)
    for k in (1, 4, 7):
        y = y0 + k * pitch
        P.box("clip", (-0.76, y - 0.015, -0.01), (-0.64, y + 0.015, 0.078), "trim", bevel=0.003)
    P.text("R-5", 0.56, 0.035, 0.045)


def m_screen(P, F, spec):
    plate = plate_slab(P)
    slots = [P.box("slot", (x - 0.12, y, -0.03), (x + 0.12, y + 0.03, 0.05), "machinery")
             for x in (-0.45, -0.15, 0.15, 0.45) for y in (0.24, 0.31)]
    P.cut(plate, "seams", seams(P) + slots)
    cplate(P, "surround", -0.72, 0.46, 0.72, 1.82, 0.16, -0.04, 0.012, "bulkhead", corners="tl,tr")
    bez = P.box("bezel", (-0.56, 0.60, -0.02), (0.56, 1.64, 0.05), "paint2")
    scr = P.box("display", (-0.50, 0.94, 0.02), (0.50, 1.565, 0.15), {"-z": "screen", "*": "machinery"})
    kb = P.box("key_well", (-0.50, 0.66, 0.035), (0.50, 0.88, 0.15), "machinery")
    P.cut(bez, "display", [scr, kb])
    bevel_ob(bez, 0.012)
    # keys under the display (owner, 2026-10-05): two rows of 15, a few lit
    lit = {(0, 1): "accent", (1, 1): "accent", (13, 0): "amber", (14, 0): "amber", (7, 1): "light_panel"}
    for r in range(2):
        for c in range(15):
            x = -0.48 + c * 0.064
            y = 0.68 + r * 0.098
            P.box("key", (x, y, 0.025), (x + 0.052, y + 0.082 if r else y + 0.07, 0.058), lit.get((c, r), "trim"), bevel=0.006, seg=2)
    P.text("SYS 3", -0.54, 1.68, 0.05)
    P.box("label_plate", (0.30, 1.67, -0.01), (0.54, 1.73, 0.004), "trim", bevel=0.002)
    for x in (0.30, 0.40):
        P.cyl("conduit", "y", (x, 0.03), 0.022, 1.62, 2.2, "machinery", sides=12)
    bolts(P, [(sx * 0.53, y) for sx in (-1, 1) for y in (0.63, 1.61)], r=0.011, z0=0.03, z1=0.06)


def m_flank(P, F, spec):
    """The panel beside a door: its amber strip is on the right (the door's side); a bay with the
    door on its left shows it mirrored."""
    plate = plate_slab(P)
    cuts = seams(P) + [P.recess(0.47, 0.10, 0.59, 1.90, 0.045)]
    for cy in (0.55, 1.35):
        octagon_grille(P, cuts, -0.40, cy, 0.17, depth=0.06, pitch=0.04, ring_w=0.04)
    cuts += [P.box("groove", (0.035, 0.05, -0.012), (0.047, 1.95, 0.05), "machinery"),
             P.box("groove", (-0.80, 0.95, -0.012), (0.04, 0.962, 0.05), "machinery")]
    P.cut(plate, "strip_channel", cuts)
    P.box("strip", (0.485, 0.14, -0.06), (0.575, 1.86, -0.012), "amber", bevel=0.012, seg=2)
    for x0, x1 in ((0.445, 0.47), (0.59, 0.615)):
        P.box("rail", (x0, 0.08, -0.01), (x1, 1.92, 0.018), "trim", bevel=0.004)
    if F["wear"]["rust"] > 0:
        P.box("hazard_band", (0.64, 0.10, -0.01), (0.75, 1.90, 0.004), "hazard", bevel=0.002)
    # the door control: a lit push button and a small keypad
    bz = P.box("control", (0.08, 0.52, -0.02), (0.36, 0.98, 0.035), "paint2", bevel=0.008)
    P.cut(bz, "button_seat", [P.cyl("seat", "z", (0.22, 0.64), 0.06, 0.02, 0.1, "machinery", sides=20, smooth=False)])
    P.cyl("button", "z", (0.22, 0.64), 0.045, 0.0, 0.05, "accent", sides=20, bevel=0.01, seg=2)
    keypad(P, 0.115, 0.735, 3, 2, 0.04, 0.05, 0.035, lit={(1, 1): "amber"})
    P.box("indicator", (0.13, 0.905, 0.03), (0.31, 0.935, 0.045), "amber", bevel=0.004)
    P.text("B-14", -0.62, 1.70, 0.06)
    P.text("DOOR", 0.11, 1.00, 0.045)


def m_narrow(P, F, spec):
    """For narrow bays: a 0.6 m channel with a pipe, a short lamp at eye level, a small grille and a
    tag, in the middle of plain plate."""
    plate = plate_slab(P)
    cuts = seams(P) + [P.recess(-0.26, 0.05, 0.26, 1.95, 0.06, floor="paint2")]
    P.cut(plate, "channel", cuts)
    P.cyl("pipe", "y", (-0.10, -0.01), 0.065, -0.2, 2.2, "trim")
    for y in (0.40, 1.00, 1.60):
        P.cyl("collar", "y", (-0.10, -0.01), 0.077, y - 0.025, y + 0.025, "paint2", sides=18, bevel=0.005)
    P.box("lamp_seat", (0.07, 1.02, -0.07), (0.20, 1.46, -0.03), "machinery", bevel=0.004)
    P.box("lamp", (0.09, 1.04, -0.06), (0.18, 1.44, -0.02), "light_panel", bevel=0.014, seg=2)
    P.box("grille_back", (0.05, 0.24, -0.07), (0.22, 0.64, -0.05), "machinery")
    slats(P, 0.05, 0.22, 0.25, 0.63, 6, -0.04, 50.0, role="trim", thick=0.004)
    P.box("tag", (0.06, 1.64, -0.065), (0.22, 1.76, -0.04), "trim", bevel=0.003)
    for x0, x1 in ((-0.29, -0.26), (0.26, 0.29)):
        P.box("rail", (x0, 0.03, -0.01), (x1, 1.97, 0.016), "trim", bevel=0.004)
        bolts(P, [((x0 + x1) / 2, y) for y in (0.2, 0.7, 1.3, 1.8)], r=0.009, z0=0.0, z1=0.022)


# ----------------------------------------------------------------------------- strips
# One 2 m period at x offset ox, built three times (ox -2, 0, 2) so the render tiles. A strip
# spans y 0..0.5 with guard_m of plain plate at its top and bottom.

def s_base(P, ox):
    kick = P.box("kick", (ox - 1.0, 0.03, -0.05), (ox + 1.0, 0.40, 0.012), "machinery")
    P.cut(kick, "grille", [P.box("grille_cut", (ox - 0.30, 0.16, -0.04), (ox + 0.30, 0.355, 0.1), "machinery")])
    bevel_ob(kick, 0.005)
    slats(P, ox - 0.31, ox + 0.31, 0.165, 0.35, 4, -0.02, 50.0, role="trim", thick=0.005)
    bolts(P, [(ox + dx, 0.37) for dx in (-0.85, -0.45, 0.45, 0.85)], r=0.011, z0=0.0, z1=0.022)
    return [P.box("groove", (ox - 1.1, 0.436, -0.01), (ox + 1.1, 0.448, 0.05), "machinery")]


def s_louvre(P, ox):
    for y0, y1 in ((0.035, 0.07), (0.43, 0.465)):
        P.box("rail", (ox - 1.0, y0, -0.02), (ox + 1.0, y1, 0.02), "trim", bevel=0.004)
    slats(P, ox - 1.0, ox + 1.0, 0.075, 0.425, 5, -0.04, 50.0)
    for k in (-2, -1, 0, 1):
        x, hw = ox + 0.5 * k, (0.04 if k == 0 else 0.02)
        P.box("mullion", (x - hw, 0.07, -0.08), (x + hw, 0.43, 0.012), "trim", bevel=0.004)
    bolts(P, [(ox, 0.12), (ox, 0.38)], r=0.01, z0=0.0, z1=0.022)
    return [P.recess(ox - 1.05, 0.07, ox + 1.05, 0.43, 0.08)]


def s_spare(P, ox):
    for y0, y1 in ((0.03, 0.055), (0.445, 0.47)):
        P.box("rail", (ox - 1.0, y0, -0.02), (ox + 1.0, y1, 0.016), "trim", bevel=0.004)
    for k in range(20):
        x = ox - 0.95 + 0.1 * k
        P.box("rib", (x - 0.024, 0.055, -0.03), (x + 0.024, 0.445, 0.014), "machinery", bevel=0.006)
    P.box("bolt_plate", (ox - 0.09, 0.08, -0.01), (ox + 0.09, 0.42, 0.026), "paint2", bevel=0.006)
    bolts(P, [(ox + dx, y) for dx in (-0.05, 0.05) for y in (0.12, 0.38)], r=0.01, z0=0.02, z1=0.034)
    return [P.recess(ox - 1.05, 0.055, ox + 1.05, 0.445, 0.03)]


def s_top(P, ox):
    tray = P.box("tray", (ox - 1.0, 0.03, -0.02), (ox + 1.0, 0.165, 0.05), "machinery")
    P.cut(tray, "tray_channel", [P.box("tray_cut", (ox - 1.05, 0.045, 0.012), (ox + 1.05, 0.15, 0.1), "machinery")])
    for y, r, role in ((0.065, 0.011, "rubber"), (0.095, 0.013, "trim"), (0.126, 0.011, "safety")):
        P.cyl("tray_conduit", "x", (y, 0.026), r, ox - 1.0, ox + 1.0, role, sides=10)
    for k in range(4):
        x = ox - 0.75 + 0.5 * k
        P.box("tray_strap", (x - 0.015, 0.03, 0.045), (x + 0.015, 0.165, 0.056), "trim", bevel=0.003)
    P.cyl("pipe", "x", (0.27, 0.05), 0.07, ox - 1.0, ox + 1.0, "trim")
    for dx in (-0.025, 0.025):
        P.cyl("flange", "x", (0.27, 0.05), 0.092, ox + 0.5 + dx - 0.018, ox + 0.5 + dx + 0.018, "trim", sides=20, bevel=0.005)
    P.cyl("collar", "x", (0.27, 0.05), 0.08, ox - 0.43, ox - 0.37, "paint2", sides=20, bevel=0.005)
    P.box("bracket", (ox - 0.42, 0.20, -0.03), (ox - 0.38, 0.34, 0.0), "machinery")
    P.cyl("marker", "x", (0.27, 0.05), 0.072, ox - 0.80, ox - 0.72, "hazard", sides=20)
    P.cyl("pipe", "x", (0.41, 0.03), 0.032, ox - 1.0, ox + 1.0, "machinery", sides=12)
    for dx in (-0.9, 0.1):
        P.box("clip", (ox + dx - 0.015, 0.37, -0.01), (ox + dx + 0.015, 0.45, 0.066), "trim", bevel=0.003)
    return []


STRIP_BUILDERS = {"base": s_base, "louvre": s_louvre, "spare": s_spare, "top": s_top}
MODULE_BUILDERS = {"plate": m_plate, "vent": m_vent, "pipes": m_pipes, "hatch": m_hatch, "junction": m_junction,
                   "light": m_light, "ribbed": m_ribbed, "screen": m_screen, "flank": m_flank, "narrow": m_narrow}


# ----------------------------------------------------------------------------- ceilings and floors
# A cell (ceilings-and-trims design section 1, floor-panels design section 1): panel space as for a
# wall module, x -1..1 and y 0..2, seen from the room: panel x is world +x, panel y world +z (the
# bow), z points into the room. Features stay inside x and y 0.2 m from the cell's edges (cells.margin_m),
# except runs that cross the cell from beam to beam (cable trays, pipes), whose ends the beams hide.

def cell_seams(P, w=0.016, d=0.012):
    """Cutters for the joints on a cell's right and top edges: each joint between two cells is then
    one groove, cut by the cell on its left or below."""
    return [P.box("seam", (1.0 - w, -0.7, -d), (1.2, 2.7, 0.05), "machinery"),
            P.box("seam", (-1.7, 2.0 - w, -d), (1.7, 2.2, 0.05), "machinery")]


def studs(P, what, items, role, top=0.55):
    """Many small raised bumps as one mesh, without booleans: items are (profile, z0, z1), a convex
    profile [(x, y), ...] extruded from z0 to z1 with its top shrunk to `top` of its size about its
    centre, so each bump has sloped sides that catch the light (tread plate, anti-slip studs)."""
    bm = bmesh.new()
    ri = kit.ROLES.index(role)
    for pts, z0, z1 in items:
        cx = sum(p[0] for p in pts) / len(pts)
        cy = sum(p[1] for p in pts) / len(pts)
        bot = [bm.verts.new(Vector((x, y, z0))) for x, y in pts]
        tp = [bm.verts.new(Vector((cx + (x - cx) * top, cy + (y - cy) * top, z1))) for x, y in pts]
        n = len(pts)
        for i in range(n):
            j = (i + 1) % n
            bm.faces.new((bot[i], bot[j], tp[j], tp[i])).material_index = ri
        bm.faces.new(tp).material_index = ri
        bm.faces.new(list(reversed(bot))).material_index = ri
    return kit._object(bm, P._name(what), P.coll)


def lozenge(cx, cy, length, width, angle_deg):
    """An elongated hexagon (a diamond-plate lug) centred on (cx, cy), its long axis at angle_deg."""
    a = math.radians(angle_deg)
    ca, sa = math.cos(a), math.sin(a)
    hl, hw = length / 2, width / 2
    local = [(-hl, 0.0), (-hl + hw, -hw), (hl - hw, -hw), (hl, 0.0), (hl - hw, hw), (-hl + hw, hw)]
    return [(cx + x * ca - y * sa, cy + x * sa + y * ca) for x, y in local]


def tread(x0, x1, y0, y1, pitch=0.05, length=0.075, width=0.02, z1=0.0045, keep_out=()):
    """Diamond plate: lugs on a square lattice at pitch, alternating +-45 degrees, kept whole inside
    the rectangle and off the openings keep_out lists (("rect", x0, y0, x1, y1) or ("circle", cx, cy,
    r): a lug must not float over a grate's pit or a drain). Periodic: a lattice pitch that divides
    2 m repeats exactly from cell to cell."""
    def blocked(cx, cy):
        m = length / 2
        for k in keep_out:
            if k[0] == "rect" and k[1] - m < cx < k[3] + m and k[2] - m < cy < k[4] + m:
                return True
            if k[0] == "circle" and math.hypot(cx - k[1], cy - k[2]) < k[3] + m:
                return True
        return False
    items = []
    i0, i1 = math.floor(x0 / pitch), math.ceil(x1 / pitch)
    j0, j1 = math.floor(y0 / pitch), math.ceil(y1 / pitch)
    for i in range(i0, i1 + 1):
        for j in range(j0, j1 + 1):
            cx, cy = (i + 0.5) * pitch, (j + 0.5) * pitch
            if not (x0 + length / 2 <= cx <= x1 - length / 2 and y0 + length / 2 <= cy <= y1 - length / 2):
                continue
            if blocked(cx, cy):
                continue
            items.append((lozenge(cx, cy, length, width, 45.0 if (i + j) % 2 == 0 else -45.0), -0.004, z1))
    return items


def ring(P, what, cx, cy, r_out, r_in, z0, z1, role, sides=40, bevel=0.0):
    """A flat round ring facing the viewer (an annulus extruded along z)."""
    ob = P.cyl(what, "z", (cx, cy), r_out, z0, z1, role, sides=sides, smooth=False)
    P.cut(ob, what + "_hole", [P.cyl(what + "_cut", "z", (cx, cy), r_in, z0 - 0.1, z1 + 0.1, "machinery", sides=sides, smooth=False)])
    if bevel:
        bevel_ob(ob, bevel)
    return ob


def frame_ring(P, what, x0, y0, x1, y1, w, z0, z1, role, bevel=0.0):
    """A rectangular frame: the box x0..x1, y0..y1 less its inside, w wide."""
    fr = P.box(what, (x0, y0, z0), (x1, y1, z1), role)
    P.cut(fr, what + "_open", [P.box(what + "_cut", (x0 + w, y0 + w, z0 - 0.1), (x1 - w, y1 - w, z1 + 0.1), "machinery")])
    if bevel:
        bevel_ob(fr, bevel)
    return fr


def rivet_row(P, x0, y0, x1, y1, n, r=0.011, z=0.0, role="trim"):
    pts = [(x0 + (x1 - x0) * k / (n - 1), y0 + (y1 - y0) * k / (n - 1)) for k in range(n)] if n > 1 else [(x0, y0)]
    return [P.cyl("rivet", "z", (x, y), r, z - 0.01, z + 0.008, role, sides=10, bevel=min(0.004, r * 0.4)) for x, y in pts]


def rusty(F):
    return F["wear"]["rust"] > 0


def c_plate(P, F, spec):
    """The rest panel: two plates side by side with a seam between and rivet rows down their edges,
    and a small inspection cover."""
    plate = plate_slab(P)
    P.cut(plate, "seams", cell_seams(P))
    for x0, x1, corners in ((-0.86, -0.025, "tl,bl"), (0.025, 0.86, "tr,br")):
        cplate(P, "plate", x0, 0.12, x1, 1.88, 0.09, -0.04, 0.007, "bulkhead", corners=corners, bevel=0.004)
        for x in (x0 + 0.045, x1 - 0.045):
            rivet_row(P, x, 0.24, x, 1.76, 9, z=0.007)
    cplate(P, "cover", 0.30, 1.30, 0.66, 1.58, 0.03, -0.01, 0.017, "paint2", bevel=0.004)
    bolts(P, [(x, y) for x in (0.33, 0.63) for y in (1.33, 1.55)], r=0.009, z0=0.0, z1=0.024)
    cplate(P, "doubler", -0.70, 0.38, -0.30, 0.62, 0.03, -0.01, 0.014, "bulkhead", bevel=0.004)
    rivet_row(P, -0.67, 0.41, -0.33, 0.41, 5, r=0.008, z=0.014)
    rivet_row(P, -0.67, 0.59, -0.33, 0.59, 5, r=0.008, z=0.014)
    if rusty(F):
        P.box("tag", (0.36, 0.26, -0.01), (0.62, 0.34, 0.009), "hazard", bevel=0.002)


def c_grille(P, F, spec):
    """A framed ventilation grille: an egg-crate of bars in a square opening, a frame and a surround."""
    plate = plate_slab(P)
    g = 0.46
    P.cut(plate, "opening", cell_seams(P) + [P.recess(-g, 1 - g, g, 1 + g, 0.16)])
    sur = cplate(P, "surround", -0.70, 0.30, 0.70, 1.70, 0.12, -0.04, 0.010, "paint2", bevel=0.005)
    P.cut(sur, "surround_open", [P.box("sur_cut", (-g, 1 - g, -0.1), (g, 1 + g, 0.1), "machinery")])
    frame_ring(P, "frame", -g - 0.06, 1 - g - 0.06, g + 0.06, 1 + g + 0.06, 0.06, -0.02, 0.03, "trim", bevel=0.008)
    n = 9
    for k in range(1, n):
        t = -g + 2 * g * k / n
        P.box("bar", (t - 0.007, 1 - g - 0.01, -0.10), (t + 0.007, 1 + g + 0.01, -0.012), "trim")
        P.box("bar", (-g - 0.01, 1 + t - 0.007, -0.09), (g + 0.01, 1 + t + 0.007, -0.02), "paint2")
    P.box("filter", (-g, 1 - g, -0.155), (g, 1 + g, -0.14), "machinery")
    bolts(P, [(sx * (g + 0.03), 1 + sy * (g + 0.03)) for sx in (-1, 1) for sy in (-1, 1)], z0=0.0, z1=0.042)
    if rusty(F):
        for sx in (-1, 1):
            P.box("tab", (sx * 0.62 - 0.05, 0.34, -0.01), (sx * 0.62 + 0.05, 0.44, 0.012), "hazard", bevel=0.002)


def c_fan(P, F, spec):
    """A round fan in a square frame: hub and pitched blades in a well, a guard of rings and spokes."""
    plate = plate_slab(P)
    R = 0.50
    P.cut(plate, "fan_well", cell_seams(P) + [P.cyl("well", "z", (0.0, 1.0), R, -0.18, 0.05, "machinery", sides=48, smooth=False)])
    fr = cplate(P, "frame", -0.66, 0.34, 0.66, 1.66, 0.12, -0.03, 0.016, "paint2", bevel=0.005)
    P.cut(fr, "frame_open", [P.cyl("frame_cut", "z", (0.0, 1.0), R, -0.1, 0.1, "machinery", sides=48, smooth=False)])
    ring(P, "collar", 0.0, 1.0, R + 0.05, R, -0.02, 0.03, "trim", sides=48, bevel=0.008)
    P.cyl("hub", "z", (0.0, 1.0), 0.10, -0.15, -0.07, "trim", sides=24, bevel=0.012, seg=2)
    for k in range(7):
        a = 360.0 * k / 7
        m = (Matrix.Translation(Vector((0.0, 1.0, -0.115))) @ Matrix.Rotation(math.radians(a), 4, "Z")
             @ Matrix.Rotation(math.radians(28.0), 4, "Y"))
        P.box("blade", (-0.05, 0.09, -0.006), (0.05, 0.45, 0.006), "paint2", bevel=0.003, m=m)
    for r in (0.17, 0.29, 0.41):
        P.torus("guard", (0.0, 1.0, -0.03), r, 0.007, "trim", segs=48, ring=6)
    for a in (0.0, 45.0, 90.0, 135.0):
        P.box("spoke", (-R - 0.01, -0.008, -0.04), (R + 0.01, 0.008, -0.022), "trim",
              m=Matrix.Translation(Vector((0.0, 1.0, 0.0))) @ Matrix.Rotation(math.radians(a), 4, "Z"))
    bolts(P, [(sx * 0.58, 1 + sy * 0.58) for sx in (-1, 1) for sy in (-1, 1)], z0=0.0, z1=0.03)
    if rusty(F):
        P.box("tag", (0.30, 0.38, -0.01), (0.56, 0.44, 0.022), "hazard", bevel=0.002)


def c_cable_tray(P, F, spec):
    """Two cable runs in an open ladder tray, hung from rods, beam to beam, and a conduit beside it."""
    plate = plate_slab(P)
    P.cut(plate, "seams", cell_seams(P))
    x0, x1 = -0.58, 0.22
    for x in (x0, x1):
        P.box("rail", (x - 0.022, -0.3, 0.02), (x + 0.022, 2.3, 0.15), "trim", bevel=0.004)
    for k in range(9):
        y = 0.12 + 0.22 * k
        P.box("rung", (x0, y - 0.022, 0.13), (x1, y + 0.022, 0.152), "paint2", bevel=0.003)
    for x, r, role in ((-0.49, 0.034, "rubber"), (-0.415, 0.03, "rubber"), (-0.345, 0.026, "safety"), (-0.28, 0.022, "rubber"),
                       (-0.13, 0.05, "machinery"), (-0.02, 0.034, "rubber"), (0.09, 0.03, "stencil"), (0.165, 0.024, "rubber")):
        P.cyl("cable", "y", (x, 0.128 - r), r, -0.3, 2.3, role, sides=14)
    for y in (0.34, 1.66):
        for x in (x0 - 0.05, x1 + 0.05):
            P.cyl("rod", "z", (x, y), 0.009, -0.01, 0.17, "trim", sides=8)
        P.box("strut", (x0 - 0.08, y - 0.025, 0.15), (x1 + 0.08, y + 0.025, 0.175), "machinery", bevel=0.004)
        for x in (-0.38, -0.07, 0.12):
            P.box("tie", (x - 0.09, y - 0.008, 0.06), (x + 0.09, y + 0.008, 0.135), "rubber")
    P.cyl("conduit", "y", (0.52, 0.045), 0.032, -0.3, 2.3, "paint2", sides=14)
    for y in (0.5, 1.5):
        P.box("clip", (0.47, y - 0.016, -0.01), (0.57, y + 0.016, 0.086), "trim", bevel=0.003)
    P.box("pull_box", (0.43, 0.86, -0.01), (0.63, 1.14, 0.11), "paint2", bevel=0.008)
    bolts(P, [(x, y) for x in (0.46, 0.60) for y in (0.89, 1.11)], r=0.008, z0=0.10, z1=0.118)
    if rusty(F):
        P.cyl("band", "y", (0.52, 0.045), 0.034, 1.25, 1.31, "hazard", sides=14)


def c_pipes(P, F, spec):
    """Three pipes crossing the cell on hangers, with flanges, collars, a marker band and a valve."""
    plate = plate_slab(P)
    P.cut(plate, "seams", cell_seams(P))
    pipes = ((-0.44, 0.085, "trim"), (-0.12, 0.06, "paint2"), (0.18, 0.045, "machinery"))
    zc = {}
    for x, r, role in pipes:
        z = 0.07 + r
        zc[x] = z
        P.cyl("pipe", "y", (x, z), r, -0.3, 2.3, role, sides=20)
    bot = max(zc[x] + r for x, r, _ in pipes)
    for y in (0.42, 1.58):
        P.box("strut", (-0.62, y - 0.03, bot), (0.34, y + 0.03, bot + 0.025), "machinery", bevel=0.004)
        for x in (-0.60, 0.32):
            P.cyl("rod", "z", (x, y), 0.01, -0.01, bot + 0.01, "trim", sides=8)
        for x, r, _ in pipes:
            P.cyl("collar", "y", (x, zc[x]), r + 0.012, y - 0.022, y + 0.022, "paint2", sides=20, bevel=0.004)
    x0, r0 = pipes[0][0], pipes[0][1]
    for y in (0.98, 1.06):
        P.cyl("flange", "y", (x0, zc[x0]), r0 + 0.028, y - 0.018, y + 0.018, "trim", sides=20, bevel=0.006)
    x1, r1 = pipes[1][0], pipes[1][1]
    P.cyl("marker", "y", (x1, zc[x1]), r1 + 0.003, 0.70, 0.78, "hazard", sides=20)
    P.cyl("marker", "y", (x0, zc[x0]), r0 + 0.003, 1.30, 1.36, "stencil", sides=20)
    x2, r2 = pipes[2][0], pipes[2][1]
    P.cyl("valve", "y", (x2, zc[x2]), r2 + 0.03, 1.12, 1.24, "paint2", sides=12, bevel=0.008)
    P.cyl("stem", "z", (x2, 1.18), 0.016, zc[x2], zc[x2] + r2 + 0.12, "trim", sides=10)
    P.torus("wheel", (x2, 1.18, zc[x2] + r2 + 0.12), 0.075, 0.011, "safety")
    for a in (0.0, 60.0, 120.0):
        P.box("spoke", (-0.072, -0.006, -0.005), (0.072, 0.006, 0.005), "safety",
              m=Matrix.Translation(Vector((x2, 1.18, zc[x2] + r2 + 0.12))) @ Matrix.Rotation(math.radians(a), 4, "Z"))
    P.cyl("hub", "z", (x2, 1.18), 0.022, zc[x2] + r2 + 0.10, zc[x2] + r2 + 0.135, "trim", sides=10)
    P.box("tag", (0.42, 0.62, -0.01), (0.66, 0.74, 0.008), "trim", bevel=0.003)
    bolts(P, [(0.45, 0.68), (0.63, 0.68)], r=0.007, z0=0.0, z1=0.013)


def c_hatch(P, F, spec):
    """A square access hatch in a hazard border, with hinges, a handle and a status pill."""
    plate = plate_slab(P)
    bw = 0.09 if rusty(F) else 0.05
    hx0, hy0, hx1, hy1, g = -0.40, 0.60, 0.40, 1.40, 0.02
    P.cut(plate, "hatch_gap", cell_seams(P) + [P.recess(hx0 - g, hy0 - g, hx1 + g, hy1 + g, 0.04)])
    sur = cplate(P, "surround", -0.66, 0.34, 0.66, 1.66, 0.12, -0.04, 0.009, "bulkhead", bevel=0.004)
    P.cut(sur, "surround_open", [P.box("sur_cut", (hx0 - g - bw, hy0 - g - bw, -0.1), (hx1 + g + bw, hy1 + g + bw, 0.1), "machinery")])
    frame_ring(P, "hazard_ring", hx0 - g - bw, hy0 - g - bw, hx1 + g + bw, hy1 + g + bw, bw, -0.03, 0.004, "hazard", bevel=0.003)
    hatch = P.box("hatch", (hx0, hy0, -0.06), (hx1, hy1, 0.014), "paint2")
    P.cut(hatch, "handle_pocket", [P.box("pocket", (0.18, 0.94, -0.016), (0.32, 1.06, 0.1), "machinery")])
    bevel_ob(hatch, 0.007)
    P.box("handle", (0.195, 0.98, -0.01), (0.305, 1.02, 0.012), "trim", bevel=0.004)
    for y in (0.74, 1.26):
        P.box("hinge", (hx0 - 0.05, y - 0.06, -0.01), (hx0 + 0.04, y + 0.06, 0.03), "trim", bevel=0.005)
    for x0, x1 in ((hx0 + 0.08, hx1 - 0.08),):
        P.box("rib", (x0, hy0 + 0.08, 0.0), (x1, hy0 + 0.11, 0.019), "paint2", bevel=0.003)
        P.box("rib", (x0, hy1 - 0.11, 0.0), (x1, hy1 - 0.08, 0.019), "paint2", bevel=0.003)
    bolts(P, [(x, y) for x in (hx0 + 0.05, hx1 - 0.05) for y in (hy0 + 0.05, hy1 - 0.05)], z0=0.0, z1=0.024)
    P.box("pill_seat", (0.44, 0.40, -0.01), (0.62, 0.47, 0.012), "machinery", bevel=0.004)
    P.box("status_pill", (0.46, 0.415, 0.0), (0.60, 0.455, 0.02), "accent", bevel=0.008, seg=2)


def c_ribbed(P, F, spec):
    """Corrugated sheet spanning beam to beam between riveted edge plates, with a cross strap."""
    plate = plate_slab(P)
    P.cut(plate, "seams", cell_seams(P))
    P.box("sheet", (-0.78, -0.3, -0.03), (0.78, 2.3, 0.002), "paint2")
    pitch = 0.13
    n = int(1.52 / pitch)
    x = -0.76 + (1.52 - n * pitch) / 2
    for k in range(n):
        xc = x + pitch * (k + 0.5)
        ob = kit.prism(P.coll, P._name("ridge"), [(xc - 0.048, -0.01), (xc + 0.048, -0.01), (xc + 0.026, 0.036), (xc - 0.026, 0.036)],
                       "y", -0.3, 2.3, "bulkhead")
        bevel_ob(ob, 0.004)
    for sx in (-1, 1):
        xa, xb = sorted((sx * 0.78, sx * 0.90))
        P.box("edge_plate", (xa, -0.3, -0.02), (xb, 2.3, 0.012), "bulkhead", bevel=0.004)
        rivet_row(P, (xa + xb) / 2, 0.15, (xa + xb) / 2, 1.85, 8, z=0.012)
    P.box("strap", (-0.80, 0.96, 0.02), (0.80, 1.04, 0.046), "trim", bevel=0.004)
    for k in range(7):
        P.cyl("rivet", "z", (-0.70 + k * 0.233, 1.0), 0.009, 0.036, 0.054, "trim", sides=8, bevel=0.003)


def c_lamp_surround(P, F, spec):
    """The cells a lamp housing spans: a light channel across the whole cell between bolted rails,
    a mounting plate and the feed cable inside it, splice plates at the cell's edges. It runs edge to
    edge so a housing sits in it wherever the lamp rule put it, and two lamp cells side by side join
    into one channel."""
    plate = plate_slab(P)
    y0, y1 = 0.60, 1.40
    P.cut(plate, "channel", cell_seams(P) + [P.recess(-1.2, y0, 1.2, y1, 0.07)])
    for ya, yb in ((y0 - 0.07, y0 + 0.01), (y1 - 0.01, y1 + 0.07)):
        P.box("rail", (-1.2, ya, -0.02), (1.2, yb, 0.024), "trim", bevel=0.006)
    for k in range(8):
        x = -0.875 + 0.25 * k
        for y in (y0 - 0.03, y1 + 0.03):
            P.cyl("bolt", "z", (x, y), 0.011, 0.0, 0.034, "trim", sides=8, bevel=0.004)
    mp = P.box("mount", (-1.2, y0 + 0.06, -0.08), (1.2, y1 - 0.06, -0.035), "paint2")
    P.cut(mp, "slots", [P.box("slot", (x - 0.06, y - 0.012, -0.06), (x + 0.06, y + 0.012, 0.1), "machinery")
                        for x in (-0.75, -0.25, 0.25, 0.75) for y in (y0 + 0.11, y1 - 0.11)])
    P.cyl("feed", "x", (y0 + 0.15, -0.03), 0.018, -1.2, 1.2, "rubber", sides=12)
    P.cyl("feed", "x", (y1 - 0.16, -0.028), 0.014, -1.2, 1.2, "safety" if rusty(F) else "rubber", sides=12)
    for x in (-1.0, 1.0):
        P.box("splice", (x - 0.06, y0 - 0.09, -0.01), (x + 0.06, y1 + 0.09, 0.032), "paint2", bevel=0.006)
        bolts(P, [(x + sx * 0.03, y) for sx in (-1, 1) for y in (y0 - 0.05, y1 + 0.05)], r=0.009, z0=0.02, z1=0.042)
    cplate(P, "doubler", -0.70, 0.18, -0.30, 0.42, 0.03, -0.01, 0.012, "paint2", bevel=0.004)
    cplate(P, "doubler", 0.30, 1.58, 0.70, 1.82, 0.03, -0.01, 0.012, "paint2", bevel=0.004)
    if rusty(F):
        for x in (-0.5, 0.5):
            P.box("tag", (x - 0.08, y0 - 0.16, -0.01), (x + 0.08, y0 - 0.10, 0.01), "hazard", bevel=0.002)


def f_walkway(P, F, spec):
    """Walkway plate, lighter than the deck: a non-slip pattern and bolt rows, repeating every 2 m in
    both directions (rendered periodic in x and y), so a run of walkway cells reads as one path."""
    P.box("plate", (-3.2, -1.2, -0.25), (3.2, 3.2, 0.0), "walk")
    lo_x, hi_x, lo_y, hi_y = -1.35, 1.35, -0.35, 2.35
    if rusty(F):
        # working: heavy five-bar tread, bars alternating across and along in 0.25 m squares
        items = []
        for i in range(-6, 6):
            for j in range(-2, 10):
                x0, y0 = 0.25 * i, 0.25 * j
                if not (lo_x <= x0 and x0 + 0.25 <= hi_x and lo_y <= y0 and y0 + 0.25 <= hi_y):
                    continue
                for k in range(4):
                    t = 0.04 + 0.057 * k
                    if (i + j) % 2 == 0:
                        pts = [(x0 + 0.03, y0 + t), (x0 + 0.22, y0 + t), (x0 + 0.22, y0 + t + 0.022), (x0 + 0.03, y0 + t + 0.022)]
                    else:
                        pts = [(x0 + t, y0 + 0.03), (x0 + t + 0.022, y0 + 0.03), (x0 + t + 0.022, y0 + 0.22), (x0 + t, y0 + 0.22)]
                    items.append((pts, -0.004, 0.006))
        studs(P, "tread", items, "walk", top=0.7)
    else:
        # crew: raised dashes in staggered rows (0.1 m along, 2/30 m between rows), big enough to
        # read at 64 px per metre
        items = []
        for j in range(-6, 36):
            cy = j * 2.0 / 30.0
            for i in range(-14, 15):
                cx = 0.1 * i + (0.05 if j % 2 else 0.0)
                if lo_x <= cx <= hi_x and lo_y <= cy <= hi_y:
                    items.append((lozenge(cx, cy, 0.072, 0.026, 0.0), -0.003, 0.0065))
        studs(P, "studs", items, "walk", top=0.5)
    # bolt rows 0.08 m in from each edge of the 2 m repeat, all the way round
    pts = []
    for ox in (-2.0, 0.0, 2.0):
        for oy in (-2.0, 0.0, 2.0):
            for k in range(8):
                t = -0.875 + 0.25 * k
                for x, y in ((ox - 0.92, 1 + oy + t), (ox + 0.92, 1 + oy + t), (ox + t, oy + 0.08), (ox + t, oy + 1.92)):
                    if lo_x - 0.05 <= x <= hi_x + 0.05 and lo_y - 0.05 <= y <= hi_y + 0.05:
                        pts.append((round(x, 4), round(y, 4)))
    for x, y in sorted(set(pts)):
        P.cyl("bolt", "z", (x, y), 0.013, -0.01, 0.007, "trim", sides=8, bevel=0.004)


def deck_plate(P, F, keep_out=()):
    """The deck a floor module is cut into: plain plate in crew spaces, diamond plate in working ones,
    its lugs kept off the module's openings (keep_out, as tread takes it)."""
    plate = plate_slab(P)
    if rusty(F):
        studs(P, "tread", tread(-1.02, 1.02, -0.02, 2.02, keep_out=keep_out), "bulkhead", top=0.55)
    return plate


def f_plate(P, F, spec):
    """Deck plate: two plates with a seam across the middle, countersunk bolts at their corners and
    edges, a lifting point; diamond plate in working spaces."""
    plate = deck_plate(P, F)
    P.cut(plate, "seams", cell_seams(P) + [P.box("seam", (-1.7, 0.994, -0.012), (1.7, 1.006, 0.05), "machinery")])
    pts = [(x, y) for x in (-0.94, -0.47, 0.0, 0.47, 0.94) for y in (0.06, 0.94, 1.06, 1.94)]
    for x, y in pts:
        P.cyl("bolt", "z", (x, y), 0.014, -0.01, 0.008, "trim", sides=8, bevel=0.004)
    for y in (0.42, 1.58):
        P.cyl("lift_ring", "z", (0.62, y), 0.03, -0.01, 0.006, "paint2", sides=16, bevel=0.004)
        P.cyl("lift_pin", "z", (0.62, y), 0.012, -0.01, 0.009, "machinery", sides=10)


def f_grate(P, F, spec):
    """Four square grates, each in its own raised frame (X2), over a dark pit."""
    cs = [(sx * 0.46, 1 + sy * 0.46) for sx in (-1, 1) for sy in (-1, 1)]
    a = 0.33
    plate = deck_plate(P, F, [("rect", cx - a - 0.06, cy - a - 0.06, cx + a + 0.06, cy + a + 0.06) for cx, cy in cs])
    P.cut(plate, "pits", cell_seams(P) + [P.recess(cx - a, cy - a, cx + a, cy + a, 0.22) for cx, cy in cs])
    for cx, cy in cs:
        frame_ring(P, "frame", cx - a - 0.06, cy - a - 0.06, cx + a + 0.06, cy + a + 0.06, 0.06, -0.02, 0.018, "trim", bevel=0.006)
        for k in range(12):
            x = cx - a + 2 * a * (k + 0.5) / 12
            P.box("bar", (x - 0.009, cy - a - 0.01, -0.04), (x + 0.009, cy + a + 0.01, -0.006), "paint2")
        for k in range(1, 4):
            y = cy - a + 2 * a * k / 4
            P.box("cross", (cx - a - 0.01, y - 0.008, -0.07), (cx + a + 0.01, y + 0.008, -0.03), "machinery")
        P.box("pit_floor", (cx - a, cy - a, -0.215), (cx + a, cy + a, -0.2), "machinery")
        bolts(P, [(cx + sx * (a + 0.03), cy + sy * (a + 0.03)) for sx in (-1, 1) for sy in (-1, 1)], r=0.01, z0=0.0, z1=0.026)


def f_access(P, F, spec):
    """A bolted access plate with a recessed handle, flush in the deck."""
    plate = deck_plate(P, F)
    x0, y0, x1, y1, g = -0.52, 0.48, 0.52, 1.52, 0.014
    P.cut(plate, "gap", cell_seams(P) + [P.recess(x0 - g, y0 - g, x1 + g, y1 + g, 0.05)])
    cov = P.box("cover", (x0, y0, -0.06), (x1, y1, 0.004), "paint2")
    P.cut(cov, "handle_pocket", [P.box("pocket", (-0.12, 0.62, -0.03), (0.12, 0.74, 0.1), "machinery"),
                                 P.box("pocket", (-0.12, 1.26, -0.03), (0.12, 1.38, 0.1), "machinery")])
    bevel_ob(cov, 0.004)
    for y in (0.68, 1.32):
        P.box("handle", (-0.10, y - 0.012, -0.03), (0.10, y + 0.012, -0.012), "trim", bevel=0.003)
    pts = [(x0 + 0.05 + (x1 - x0 - 0.1) * k / 4, y) for k in range(5) for y in (y0 + 0.05, y1 - 0.05)]
    pts += [(x, y0 + 0.05 + (y1 - y0 - 0.1) * k / 4) for k in (1, 2, 3) for x in (x0 + 0.05, x1 - 0.05)]
    for x, y in pts:
        P.cyl("bolt", "z", (x, y), 0.014, -0.01, 0.012, "trim", sides=6, bevel=0.004)
    for sx in (-1, 1):
        P.box("tick", (sx * 0.36 - 0.05, 0.56, -0.01), (sx * 0.36 + 0.05, 0.585, 0.0065), "hazard", bevel=0.002)


def f_trench(P, F, spec):
    """A cable trench cover: a long plate down the cell between rails, with finger slots and bolts."""
    plate = deck_plate(P, F)
    x0, x1, y0, y1, g = -0.30, 0.30, 0.10, 1.90, 0.014
    P.cut(plate, "gap", cell_seams(P) + [P.recess(x0 - g, y0 - g, x1 + g, y1 + g, 0.06)])
    cov = P.box("cover", (x0, y0, -0.06), (x1, y1, 0.004), "paint2")
    slots = []
    for y in (0.24, 1.76):
        for x in (-0.13, 0.13):
            slots.append(P.box("slot", (x - 0.05, y - 0.016, -0.03), (x + 0.05, y + 0.016, 0.1), "machinery"))
            for sx in (-1, 1):
                slots.append(P.cyl("slot_end", "z", (x + sx * 0.05, y), 0.016, -0.03, 0.1, "machinery", sides=12, smooth=False))
    P.cut(cov, "finger_slots", slots)
    bevel_ob(cov, 0.004)
    for x in (x0 - g - 0.03, x1 + g + 0.03):
        P.box("rail", (x - 0.016, y0 - 0.04, -0.01), (x + 0.016, y1 + 0.04, 0.012), "trim", bevel=0.004)
    for k in range(7):
        y = 0.40 + 0.2 * k
        for x in (x0 + 0.04, x1 - 0.04):
            P.cyl("bolt", "z", (x, y), 0.012, -0.01, 0.012, "trim", sides=6, bevel=0.004)


def f_drain(P, F, spec):
    """A round drain grille in the deck: a collar, ring and spoke bars over a dark sump, and two
    shallow channels leading to it."""
    R = 0.28
    plate = deck_plate(P, F, [("circle", 0.0, 1.0, R + 0.07), ("rect", -0.78, 0.97, 0.78, 1.03)])
    cuts = cell_seams(P) + [P.cyl("sump", "z", (0.0, 1.0), R, -0.2, 0.05, "machinery", sides=40, smooth=False)]
    for sx in (-1, 1):
        xa, xb = sorted((sx * (R + 0.02), sx * 0.78))
        cuts.append(P.box("channel", (xa, 0.97, -0.01), (xb, 1.03, 0.05), "machinery"))
    P.cut(plate, "drain", cuts)
    ring(P, "collar", 0.0, 1.0, R + 0.07, R, -0.02, 0.012, "trim", sides=40, bevel=0.006)
    for r in (0.09, 0.17, 0.25):
        ring(P, "grid_ring", 0.0, 1.0, r + 0.012, r - 0.012, -0.05, -0.012, "paint2", sides=32)
    for a in (0.0, 45.0, 90.0, 135.0):
        P.box("bar", (-R - 0.01, -0.012, -0.06), (R + 0.01, 0.012, -0.02), "paint2",
              m=Matrix.Translation(Vector((0.0, 1.0, 0.0))) @ Matrix.Rotation(math.radians(a), 4, "Z"))
    P.box("sump_floor", (-R, 1 - R, -0.195), (R, 1 + R, -0.18), "machinery")
    bolts(P, [(math.cos(math.radians(30 + 60 * k)) * (R + 0.035), 1 + math.sin(math.radians(30 + 60 * k)) * (R + 0.035)) for k in range(6)],
          r=0.011, z0=0.0, z1=0.02)


def f_vent(P, F, spec):
    """Two raised floor vents (X2): slatted boxes on the deck with bolted feet."""
    plate = deck_plate(P, F, [("rect", cx - 0.31, 0.56, cx + 0.31, 1.44) for cx in (-0.36, 0.36)])
    P.cut(plate, "seams", cell_seams(P))
    for cx in (-0.36, 0.36):
        x0, x1, y0, y1 = cx - 0.27, cx + 0.27, 0.62, 1.38
        box = P.box("housing", (x0, y0, -0.02), (x1, y1, 0.05), "paint2")
        P.cut(box, "opening", [P.box("open", (x0 + 0.04, y0 + 0.04, 0.0), (x1 - 0.04, y1 - 0.04, 0.2), "machinery")])
        bevel_ob(box, 0.006)
        P.box("pit", (x0 + 0.04, y0 + 0.04, -0.02), (x1 - 0.04, y1 - 0.04, 0.002), "machinery")
        n = 9
        pitch = (y1 - y0 - 0.08) / n
        for k in range(n):
            yc = y0 + 0.04 + pitch * (k + 0.5)
            P.tilted("slat", (cx, yc, 0.028), (0.23, pitch * 0.55, 0.004), 40.0, "trim", bevel=0.002)
        for sx in (-1, 1):
            P.box("foot", (cx + sx * 0.27 - 0.04, y0 - 0.06, -0.01), (cx + sx * 0.27 + 0.04, y0 + 0.02, 0.014), "trim", bevel=0.004)
            P.box("foot", (cx + sx * 0.27 - 0.04, y1 - 0.02, -0.01), (cx + sx * 0.27 + 0.04, y1 + 0.06, 0.014), "trim", bevel=0.004)
        bolts(P, [(cx + sx * 0.27, y) for sx in (-1, 1) for y in (y0 - 0.03, y1 + 0.03)], r=0.01, z0=0.0, z1=0.022)


def f_hazard(P, F, spec):
    """Plate with a hazard-striped border (working spaces): keep clear of the plate it frames."""
    plate = deck_plate(P, F)
    P.cut(plate, "seams", cell_seams(P))
    frame_ring(P, "border", -0.78, 0.22, 0.78, 1.78, 0.14, -0.02, 0.008, "hazard", bevel=0.003)
    cov = cplate(P, "cover", -0.58, 0.42, 0.58, 1.58, 0.08, -0.02, 0.012, "paint2", bevel=0.005)
    P.cut(cov, "pocket", [P.box("pocket", (-0.10, 0.50, -0.01), (0.10, 0.60, 0.1), "machinery")])
    P.box("handle", (-0.08, 0.53, -0.01), (0.08, 0.57, 0.004), "trim", bevel=0.003)
    bolts(P, [(sx * 0.71, 1 + sy * 0.71) for sx in (-1, 1) for sy in (-1, 1)], r=0.014, z0=0.0, z1=0.024)
    bolts(P, [(sx * 0.52, 1 + sy * 0.52) for sx in (-1, 1) for sy in (-1, 1)], r=0.011, z0=0.0, z1=0.026)


CEILING_BUILDERS = {"plate": c_plate, "grille": c_grille, "fan": c_fan, "cable_tray": c_cable_tray, "pipes": c_pipes,
                    "hatch": c_hatch, "ribbed": c_ribbed, "lamp_surround": c_lamp_surround}
FLOOR_BUILDERS = {"walkway": f_walkway, "plate": f_plate, "grate": f_grate, "access": f_access, "trench": f_trench,
                  "drain": f_drain, "vent": f_vent, "hazard": f_hazard}


# ----------------------------------------------------------------------------- trims
# A trim row: panel x along the member (one 2 m period at x offset ox, built at ox -2, 0 and 2 so the
# render tiles), panel y across the face from 0 to the row's height h, z out of the face. A row
# builder returns the cutters for the slab behind it.

def flange_shaft(P, ox, h, F, holes="stadium", hazard=False, splice=True):
    """An I-beam's face: riveted edge bands, a raised web plate pierced by lightening holes every
    0.5 m, and once a period a bolted splice plate. hazard stripes the lower band."""
    e = 0.042
    cuts = []
    for k, (y0, y1) in enumerate(((-0.02, e), (h - e, h + 0.02))):
        # hazard: the lower flange only (on a beam's side, the edge a head meets)
        P.box("flange", (ox - 1.05, y0, -0.01), (ox + 1.05, y1, 0.008), "hazard" if hazard and k == 0 else "bulkhead", bevel=0.003)
    web = P.box("web", (ox - 1.05, e + 0.008, -0.01), (ox + 1.05, h - e - 0.008, 0.016), "bulkhead")
    hc = []
    hy, hr = h / 2, (h - 2 * e) * 0.26
    for dx in (-0.75, -0.25, 0.25, 0.75):
        x = ox + dx
        if holes == "stadium":
            hc.append(P.box("hole", (x - 0.07, hy - hr, -0.05), (x + 0.07, hy + hr, 0.1), "machinery"))
            for sx in (-1, 1):
                hc.append(P.cyl("hole_end", "z", (x + sx * 0.07, hy), hr, -0.05, 0.1, "machinery", sides=16, smooth=False))
        else:
            hc.append(P.cyl("hole", "z", (x, hy), hr * 1.25, -0.05, 0.1, "machinery", sides=20, smooth=False))
        cuts.append(P.box("pocket", (x - 0.13, hy - hr * 1.3, -0.04), (x + 0.13, hy + hr * 1.3, 0.05), "machinery"))
    P.cut(web, "lightening_holes", hc)
    bevel_ob(web, 0.004)
    for k in range(20):
        x = ox - 0.95 + 0.1 * k
        if splice and abs(x - ox) < 0.12:
            continue
        for y in (e / 2, h - e / 2):
            P.cyl("rivet", "z", (x, y), 0.0085, 0.0, 0.016, "trim", sides=8, bevel=0.003)
    if splice:
        P.box("splice", (ox - 0.09, -0.02, -0.01), (ox + 0.09, h + 0.02, 0.03), "paint2", bevel=0.006)
        bolts(P, [(ox + sx * 0.05, y) for sx in (-1, 1) for y in (0.05, h - 0.05)], r=0.011, z0=0.02, z1=0.042)
    return cuts


def t_side(P, ox, h, F):
    cuts = [P.box("groove", (ox - 1.1, y - 0.004, -0.006), (ox + 1.1, y + 0.004, 0.05), "machinery") for y in (0.016, h - 0.016)]
    for dx in (-0.75, -0.25, 0.25, 0.75):
        P.cyl("rivet", "z", (ox + dx, h / 2), 0.009, -0.005, 0.007, "trim", sides=8, bevel=0.003)
    return cuts


def t_baseboard(P, ox, h, F):
    """A kick plate: a top lip, slot vents every 1 m and bolts between; hazard-striped in working spaces."""
    P.box("lip", (ox - 1.05, h - 0.034, -0.01), (ox + 1.05, h + 0.02, 0.012), "trim", bevel=0.004)
    P.box("toe", (ox - 1.05, -0.02, -0.01), (ox + 1.05, 0.014, 0.006), "machinery")
    cuts = []
    for cx in (ox - 0.5, ox + 0.5):
        for k in range(4):
            x = cx - 0.105 + 0.07 * k
            cuts.append(P.box("slot", (x - 0.022, 0.034, -0.03), (x + 0.022, h - 0.052, 0.05), "machinery"))
        P.box("slot_floor", (cx - 0.14, 0.03, -0.03), (cx + 0.14, h - 0.048, -0.018), "machinery")
    for dx in (-1.0, 0.0):   # every 1 m, between the vents (the copies at ox +-2 give the rest)
        P.cyl("bolt", "z", (ox + dx, (0.024 + h - 0.04) / 2), 0.013, -0.005, 0.011, "trim", sides=6, bevel=0.004)
    return cuts


def t_rib(P, ox, h, F):
    return flange_shaft(P, ox, h, F, holes="stadium")


def t_rib_ends(P, ox, h, F):
    """The pillar's base (centred at ox - 0.5) and capital (at ox + 0.5), each 0.45 m of the row,
    rising to +x: prouder than the shaft (7 cm against its 1.6), stepped and chamfered, with a shadow
    line where they meet it and heavy hex bolts; a working base is hazard-striped. The trims' key
    light runs along +x, so a pillar is lit from above: a base's top chamfer catches it and a
    capital's underside is in shadow, which is what makes them read as structure."""
    T = DATA["trims"]["pieces"]
    for name in ("rib_base", "rib_capital"):
        c = ox + T[name]["centre_m"]
        L = T[name]["length_m"]
        x0, x1 = c - L / 2, c + L / 2
        if name == "rib_base":   # foot at x0, rising to the shaft at x1
            foot = P.box("foot", (x0 - 0.03, -0.02, -0.01), (x0 + 0.24, h + 0.02, 0.07), "hazard" if rusty(F) else "paint2")
            bevel_ob(foot, 0.02)
            P.box("step", (x0 + 0.20, -0.02, -0.01), (x0 + 0.33, h + 0.02, 0.042), "bulkhead", bevel=0.012)
            P.box("band", (x0 + 0.31, -0.02, -0.01), (x1 + 0.03, h + 0.02, 0.022), "bulkhead", bevel=0.005)
            P.box("shadow", (x0 + 0.325, -0.02, -0.01), (x0 + 0.34, h + 0.02, 0.0235), "machinery")
            bolts(P, [(x0 + 0.11, y) for y in (0.065, h - 0.065)], r=0.024, z0=0.06, z1=0.088, role="trim")
            P.box("weld", (x0 + 0.02, h / 2 - 0.008, 0.06), (x0 + 0.20, h / 2 + 0.008, 0.074), "trim", bevel=0.003)
        else:                    # from the shaft at x0, flaring to the head at x1
            P.box("band", (x0 - 0.03, -0.02, -0.01), (x0 + 0.11, h + 0.02, 0.022), "bulkhead", bevel=0.005)
            P.box("shadow", (x0 + 0.095, -0.02, -0.01), (x0 + 0.11, h + 0.02, 0.0235), "machinery")
            P.box("step", (x0 + 0.11, -0.02, -0.01), (x0 + 0.22, h + 0.02, 0.042), "bulkhead", bevel=0.012)
            head = P.box("head", (x0 + 0.20, -0.02, -0.01), (x1 + 0.03, h + 0.02, 0.07), "paint2")
            P.cut(head, "gussets", [P.box("gusset_cut", (x0 + 0.26, y0, 0.055), (x1 - 0.02, y1, 0.1), "machinery")
                                    for y0, y1 in ((0.05, 0.085), (h - 0.085, h - 0.05))])
            bevel_ob(head, 0.02)
            bolts(P, [(x0 + 0.34, y) for y in (0.065, h - 0.065)], r=0.024, z0=0.06, z1=0.088, role="trim")
    return []


def t_beam(P, ox, h, F):
    return flange_shaft(P, ox, h, F, holes="round", hazard=rusty(F))


def t_frame(P, ox, h, F):
    """A door frame's face: symmetric steps up to a raised centre band, bolts along it; the outer
    steps hazard-striped in working spaces."""
    P.box("outer", (ox - 1.05, -0.02, -0.01), (ox + 1.05, h + 0.02, 0.006), "hazard" if rusty(F) else "paint2")
    P.box("step", (ox - 1.05, 0.045, -0.01), (ox + 1.05, h - 0.045, 0.024), "bulkhead", bevel=0.01)
    P.box("centre", (ox - 1.05, 0.085, -0.01), (ox + 1.05, h - 0.085, 0.044), "paint2", bevel=0.014)
    bolts(P, [(ox + dx, h / 2) for dx in (-0.8, -0.4, 0.0, 0.4, 0.8)], r=0.014, z0=0.03, z1=0.056)
    return [P.box("groove", (ox - 1.1, y - 0.004, -0.008), (ox + 1.1, y + 0.004, 0.05), "machinery") for y in (0.04, h - 0.04)]


def t_cove(P, ox, h, F):
    """The cove: an open cable tray with two runs on hangers, slots in its floor."""
    for y0, y1 in ((-0.02, 0.034), (h - 0.034, h + 0.02)):
        P.box("rail", (ox - 1.05, y0, -0.01), (ox + 1.05, y1, 0.06), "trim", bevel=0.004)
    for y, r, role in ((0.075, 0.021, "rubber"), (0.118, 0.019, "safety"), (0.162, 0.024, "rubber"), (0.205, 0.017, "rubber")):
        P.cyl("cable", "x", (y, r + 0.002), r, ox - 1.05, ox + 1.05, role, sides=12)
    P.cyl("conduit", "x", (0.285, 0.036), 0.034, ox - 1.05, ox + 1.05, "paint2", sides=14)
    P.cyl("conduit", "x", (0.338, 0.016), 0.013, ox - 1.05, ox + 1.05, "trim", sides=10)
    for dx in (-0.75, -0.25, 0.25, 0.75):
        P.box("hanger", (ox + dx - 0.016, -0.02, 0.054), (ox + dx + 0.016, h + 0.02, 0.066), "trim", bevel=0.003)
        P.box("tie", (ox + dx + 0.03, 0.05, 0.0), (ox + dx + 0.045, 0.23, 0.05), "rubber")
    cuts = []
    for k in range(20):
        x = ox - 0.95 + 0.1 * k
        if min(abs(x - (ox + dx)) for dx in (-0.75, -0.25, 0.25, 0.75)) < 0.04:
            continue
        cuts.append(P.box("slot", (x - 0.025, 0.245, -0.012), (x + 0.025, 0.262, 0.05), "machinery"))
    return cuts


TRIM_BUILDERS = {"side": t_side, "baseboard": t_baseboard, "rib": t_rib, "rib_ends": t_rib_ends, "beam": t_beam,
                 "frame": t_frame, "cove": t_cove}


# ----------------------------------------------------------------------------- platform faces
# A platform row (platforms.rows; the owner, 2026-10-07: "small metal panels that should fit vertically
# on the geometry uv"): panel x along the face (one 2 m period at x offset ox, built at ox -2, 0 and 2 so
# the render tiles), panel y up the face from the floor (0) to the row's height h (the top, under the
# hazard nosing), z out of the face. A page maps the face's bottom to y 0 and its top to y h exactly, so
# each row is designed for its height: a dark toe kick at the floor, a steel lip under the nosing, and
# between them bays of vent grilles and bolted access panels as tall as the clear height, divided by
# stiles. Nothing is under about 3 cm, so it reads at 128 px per metre. A builder returns the cutters
# for the slab behind it.

def face_frame(P, ox, h, toe, lip):
    """The toe kick (a dark recess toe metres high at the floor) and the lip (a steel bar lip metres
    deep under the nosing, its lower edge chamfered to catch the light). Returns the toe's cutter."""
    P.box("lip", (ox - 1.05, h - lip, -0.012), (ox + 1.05, h + 0.03, 0.014), "trim", bevel=0.004)
    return [P.box("toe_cut", (ox - 1.1, -0.2, -0.035), (ox + 1.1, toe, 0.05), "machinery")]


def bays(ox, widths, stile_w):
    """The bays of one period: (x0, x1) for each width in turn, a stile stile_w wide before each (the
    first on the period's edge, shared with the period before). Returns (bays, stile centres)."""
    if abs(sum(widths) + stile_w * len(widths) - 2.0) > 1e-9:
        raise SystemExit("[panels] a platform row's bays and stiles must fill its 2 m period")
    x, out, stiles = ox - 1.0, [], []
    for w in widths:
        stiles.append(x)
        out.append((x + stile_w / 2, x + stile_w / 2 + w))
        x += stile_w + w
    return out, stiles


def access_panel(P, x0, y0, x1, y1, role="bulkhead", pull=True, lens=None, port=False):
    """A small bolted access panel filling (x0, y0)-(x1, y1): a plate 2 cm proud with chamfered
    corners, a bolt in each corner and a pull slot under its top edge. lens (an emissive role) puts a
    step light in a bezel at its middle; port a round cable port in its lower half."""
    c = min(0.035, 0.15 * (y1 - y0))
    pl = cplate(P, "access", x0, y0, x1, y1, c, -0.01, 0.010, role, bevel=0.004)
    cx = (x0 + x1) / 2
    cuts = []
    if pull:
        yp = y1 - 0.055
        cuts.append(P.box("pull_cut", (cx - 0.055, yp - 0.016, 0.0), (cx + 0.055, yp + 0.016, 0.06), "machinery"))
    if port:
        yq = y0 + 0.36 * (y1 - y0)
        cuts.append(P.cyl("port_cut", "z", (cx, yq), 0.03, -0.02, 0.06, "machinery", sides=20, smooth=False))
    if cuts:
        P.cut(pl, "access_cuts", cuts)
    if pull:
        P.box("pull_bar", (cx - 0.045, yp - 0.005, -0.004), (cx + 0.045, yp + 0.005, 0.008), "trim", bevel=0.002)
    if port:
        ring(P, "port_ring", cx, yq, 0.042, 0.03, -0.005, 0.02, "trim", sides=24, bevel=0.003)
        P.cyl("port_cable", "z", (cx, yq), 0.018, -0.02, 0.03, "rubber", sides=12)
    if lens:
        yl = (y0 + y1) / 2 + (0.02 if pull else 0.0)
        bez = P.box("lens_bezel", (cx - 0.075, yl - 0.03, 0.0), (cx + 0.075, yl + 0.03, 0.022), "trim", bevel=0.004)
        P.cut(bez, "lens_well", [P.box("lens_well", (cx - 0.06, yl - 0.017, 0.012), (cx + 0.06, yl + 0.017, 0.06), "machinery")])
        P.box("lens", (cx - 0.056, yl - 0.013, 0.0), (cx + 0.056, yl + 0.013, 0.019), lens, bevel=0.004, seg=2)
    bolts(P, [(x, y) for x in (x0 + 0.03, x1 - 0.03) for y in (y0 + 0.03, y1 - 0.03)], r=0.011, z0=0.0, z1=0.021)
    return pl


def louvre_grille(P, x0, y0, x1, y1, n, cuts, mullion=False):
    """A louvred vent filling (x0, y0)-(x1, y1): a steel frame 3 cm wide with a bolt at each corner,
    n slats tipped up 45 degrees in a dark well cut 6 cm into the slab (its cutter appended to cuts),
    and with mullion a bar down the middle."""
    w = 0.028
    frame_ring(P, "grille_frame", x0, y0, x1, y1, w, -0.02, 0.016, "trim", bevel=0.004)
    cuts.append(P.box("grille_well", (x0 + w - 0.004, y0 + w - 0.004, -0.06), (x1 - w + 0.004, y1 - w + 0.004, 0.05), "machinery"))
    slats(P, x0 + w, x1 - w, y0 + w, y1 - w, n, -0.03, 45.0, role="paint2", thick=0.005)
    if mullion:
        cx = (x0 + x1) / 2
        P.box("mullion", (cx - 0.016, y0 + w - 0.01, -0.05), (cx + 0.016, y1 - w + 0.01, 0.012), "trim", bevel=0.003)
    bolts(P, [(x, y) for x in (x0 + w / 2, x1 - w / 2) for y in (y0 + w / 2, y1 - w / 2)], r=0.009, z0=0.006, z1=0.024)


def slot_grille(P, x0, y0, x1, y1, pitch, cuts):
    """A slotted vent plate filling (x0, y0)-(x1, y1): a plate 1.5 cm proud, pierced by upright slots
    3 cm wide at pitch into a dark well, a bolt at each corner."""
    pl = cplate(P, "slot_plate", x0, y0, x1, y1, 0.025, -0.01, 0.015, "paint2", bevel=0.004)
    n = int(round((x1 - x0 - 0.08) / pitch))
    xs = [(x0 + x1) / 2 + (k - (n - 1) / 2) * pitch for k in range(n)]
    P.cut(pl, "slots", [P.box("slot", (x - 0.015, y0 + 0.05, -0.02), (x + 0.015, y1 - 0.05, 0.06), "machinery") for x in xs])
    cuts.append(P.box("slot_well", (xs[0] - 0.02, y0 + 0.045, -0.05), (xs[-1] + 0.02, y1 - 0.045, 0.05), "machinery"))
    bolts(P, [(x, y) for x in (x0 + 0.025, x1 - 0.025) for y in (y0 + 0.025, y1 - 0.025)], r=0.01, z0=0.0, z1=0.026)


def p_riser(P, ox, h, F):
    """A 0.45 m riser: a 4.5 cm toe kick, a 2.5 cm lip, and between them, every 2 m, a bolted access
    panel, a louvred vent, an access panel with a step light and a slotted vent, each the clear height
    tall, between stiles; a working face's first panel is hazard-striped and its step light amber."""
    toe, lip = 0.045, 0.025
    cuts = face_frame(P, ox, h, toe, lip)
    (a, b, c, d), stiles = bays(ox, (0.38, 0.56, 0.38, 0.56), 0.03)
    for x in stiles:
        P.box("stile", (x - 0.015, toe - 0.01, -0.01), (x + 0.015, h - lip + 0.005, 0.018), "paint2", bevel=0.004)
    y0, y1 = toe + 0.016, h - lip - 0.016
    access_panel(P, a[0] + 0.012, y0, a[1] - 0.012, y1, role="hazard" if rusty(F) else "bulkhead")
    louvre_grille(P, b[0] + 0.012, y0, b[1] - 0.012, y1, 6, cuts)
    access_panel(P, c[0] + 0.012, y0, c[1] - 0.012, y1, pull=False, lens="amber" if rusty(F) else "light_panel")
    slot_grille(P, d[0] + 0.012, y0, d[1] - 0.012, y1, 0.06, cuts)
    return cuts


def p_riser_low(P, ox, h, F):
    """A 0.225 m riser, designed at its own height (not a crop of the riser): a 3.5 cm toe kick, a 2 cm
    lip, and every 2 m a long louvred vent, a short access panel with a cable port, a long vent with a
    centre bar and a short panel with a step light."""
    toe, lip = 0.035, 0.02
    cuts = face_frame(P, ox, h, toe, lip)
    (a, b, c, d), stiles = bays(ox, (0.66, 0.28, 0.66, 0.28), 0.03)
    for x in stiles:
        P.box("stile", (x - 0.015, toe - 0.01, -0.01), (x + 0.015, h - lip + 0.005, 0.018), "paint2", bevel=0.004)
    y0, y1 = toe + 0.013, h - lip - 0.013
    louvre_grille(P, a[0] + 0.012, y0, a[1] - 0.012, y1, 3, cuts)
    access_panel(P, b[0] + 0.012, y0, b[1] - 0.012, y1, role="hazard" if rusty(F) else "bulkhead", pull=False, port=True)
    louvre_grille(P, c[0] + 0.012, y0, c[1] - 0.012, y1, 3, cuts, mullion=True)
    access_panel(P, d[0] + 0.012, y0, d[1] - 0.012, y1, pull=False, lens="amber" if rusty(F) else "light_panel")
    return cuts


def p_step(P, ox, h, F):
    """A stair step's 0.225 m front, quieter than a riser: a 3 cm toe kick, a 2 cm lip, and a bolted
    kick plate with a band of slot vents centred on x = 0 (a stair's middle: a page measures u from the
    stair's centre) and plate joints at the period's edges; a working step's plate has a hazard band
    along its foot."""
    toe, lip = 0.03, 0.02
    cuts = face_frame(P, ox, h, toe, lip)
    y0, y1 = toe + 0.008, h - lip - 0.008
    pl = P.box("kick_plate", (ox - 0.994, y0, -0.01), (ox + 0.994, y1, 0.008), "bulkhead", bevel=0.004)
    ym = (y0 + y1) / 2
    slots = [P.box("slot", (ox + x - 0.035, ym - 0.016, -0.02), (ox + x + 0.035, ym + 0.016, 0.06), "machinery")
             for x in (-0.35, -0.25, -0.15, -0.05, 0.05, 0.15, 0.25, 0.35)]
    P.cut(pl, "slot_band", slots)
    cuts.append(P.box("slot_well", (ox - 0.40, ym - 0.02, -0.04), (ox + 0.40, ym + 0.02, 0.05), "machinery"))
    if rusty(F):
        P.box("hazard_band", (ox - 0.994, y0 + 0.006, 0.0), (ox + 0.994, y0 + 0.042, 0.012), "hazard", bevel=0.002)
    bolts(P, [(ox + x, y) for x in (-0.9, -0.5, 0.5, 0.9) for y in (y0 + 0.028, y1 - 0.028)], r=0.011, z0=0.0, z1=0.02)
    return cuts


PLATFORM_BUILDERS = {"riser": p_riser, "riser_low": p_riser_low, "step": p_step}


# ----------------------------------------------------------------------------- upholstery
# Padded leather (panels.json upholstery; the owner, 2026-10-07: "chairs suck still mainly its a texture
# problem"): panel space as for a floor cell, x -1..1 and y 0..2, z out of the padding. Both layers tile
# both ways: every pitch divides 2 m and the pieces are laid out past the frame on every side, so a
# pillow the frame cuts continues on the opposite edge. Pillows are smooth heightfields, not booleans,
# and smooth shaded, so the padding reads soft under render.upholstery's soft light.

def pad_rise(t, r):
    """The padding's rise at distance t in from a pillow's edge: 0 at the edge, 1 from r in, curving
    over like a filled cushion (a parabola, steepest at the edge)."""
    u = min(1.0, max(0.0, t / r))
    return 1.0 - (1.0 - u) ** 2


def pad_height(x, y, cell, height, rx, ry):
    x0, y0, x1, y1 = cell
    return height * pad_rise(min(x - x0, x1 - x), rx) * pad_rise(min(y - y0, y1 - y), ry)


def pillows(P, what, cells, height, rx, ry, role, nx, ny, z0=-0.03):
    """Padded pillows as one mesh: each cell (x0, y0, x1, y1) a smooth heightfield rising `height` above
    z = 0 in its middle and falling to z = 0 at its edges over rx (across x) and ry (along y), closed by
    flat sides down to z0 and a flat bottom. nx, ny: the grid's segments across and along."""
    bm = bmesh.new()
    ri = kit.ROLES.index(role)
    for cell in cells:
        x0, y0, x1, y1 = cell
        xs = [x0 + (x1 - x0) * i / nx for i in range(nx + 1)]
        ys = [y0 + (y1 - y0) * j / ny for j in range(ny + 1)]
        top = [[bm.verts.new(Vector((x, y, pad_height(x, y, cell, height, rx, ry)))) for x in xs] for y in ys]
        for j in range(ny):
            for i in range(nx):
                f = bm.faces.new((top[j][i], top[j][i + 1], top[j + 1][i + 1], top[j + 1][i]))
                f.material_index = ri
                f.smooth = True
        rim = ([top[0][i] for i in range(nx + 1)] + [top[j][nx] for j in range(1, ny + 1)]
               + [top[ny][i] for i in range(nx - 1, -1, -1)] + [top[j][0] for j in range(ny - 1, 0, -1)])
        bot = [bm.verts.new(Vector((v.co.x, v.co.y, z0))) for v in rim]
        n = len(rim)
        for k in range(n):
            bm.faces.new((rim[k], bot[k], bot[(k + 1) % n], rim[(k + 1) % n])).material_index = ri
        bm.faces.new(list(reversed(bot))).material_index = ri
    return kit._object(bm, P._name(what), P.coll)


def stitches(P, pts, along, cell, height, rx, ry, length=0.011, width=0.0035):
    """A stitch line on a pillow's surface: a raised dash of thread at each point, laid along `along`
    ('x' or 'y') and lifted to the padding's height there."""
    items = []
    for x, y in pts:
        z = pad_height(x, y, cell, height, rx, ry)
        hl, hw = (length / 2, width / 2) if along == "x" else (width / 2, length / 2)
        items.append(([(x - hl, y - hw), (x + hl, y - hw), (x + hl, y + hw), (x - hl, y + hw)], z - 0.003, z + 0.0012))
    return items


def u_channel(P):
    """Channel-stitched padding: rolled vertical channels 2/21 m wide (95 mm) with a stitched groove
    between them, crossed every 0.5 m (at y 0.25 + 0.5 k) by a welted seam, a piping cord in a pinched
    gap with a twin topstitch either side, so a seat or a back reads as quilted panels."""
    pitch, gap, welt_gap, height = 2.0 / 21.0, 0.009, 0.024, 0.016
    welts = [0.25 + 0.5 * k for k in range(-1, 5)]
    cells = []
    for j in range(len(welts) - 1):
        ya, yb = welts[j] + welt_gap / 2, welts[j + 1] - welt_gap / 2
        for i in range(-12, 12):
            xa = i * pitch + gap / 2
            cells.append((xa, ya, xa + pitch - gap, yb))
    rx, ry = (pitch - gap) / 2, 0.07
    P.box("base", (-1.6, -0.6, -0.25), (1.6, 2.6, 0.0), "machinery")
    pillows(P, "channels", cells, height, rx, ry, "upholstery", 8, 24)
    items = []
    for c in cells:
        for y in (c[1] + 0.012, c[3] - 0.012):
            xs = [c[0] + 0.008 + k * 0.0195 for k in range(5)]
            items += stitches(P, [(x, y) for x in xs if x < c[2] - 0.006], "x", c, height, rx, ry)
    studs(P, "topstitch", items, "stencil", top=0.7)
    for y in welts:
        P.cyl("welt", "x", (y, 0.0015), 0.0075, -1.4, 1.4, "paint2", sides=12)


def u_panel(P):
    """Padded panels 2/7 m by 0.25 m: each a cushion rising 2 cm, its edges rolled over 4.5 cm, a
    piping cord in the welted seam on every edge and a stitch line 2.2 cm inside it; for bolsters,
    headrests and arm pads."""
    pw, ph, gap, height = 2.0 / 7.0, 0.25, 0.02, 0.02
    cells = []
    for j in range(-1, 9):
        for i in range(-4, 4):
            cells.append((i * pw + gap / 2, j * ph + gap / 2, (i + 1) * pw - gap / 2, (j + 1) * ph - gap / 2))
    r = 0.045
    P.box("base", (-1.6, -0.6, -0.25), (1.6, 2.6, 0.0), "machinery")
    pillows(P, "panels", cells, height, r, r, "upholstery", 16, 14)
    items = []
    for c in cells:
        x0, y0, x1, y1 = c[0] + 0.022, c[1] + 0.022, c[2] - 0.022, c[3] - 0.022
        n = int((x1 - x0) / 0.02)
        for k in range(n + 1):
            x = x0 + (x1 - x0) * k / n
            items += stitches(P, [(x, y0), (x, y1)], "x", c, height, r, r)
        n = int((y1 - y0) / 0.02)
        for k in range(1, n):
            y = y0 + (y1 - y0) * k / n
            items += stitches(P, [(x0, y), (x1, y)], "y", c, height, r, r)
    studs(P, "topstitch", items, "stencil", top=0.7)
    for j in range(-1, 10):
        P.cyl("welt", "x", (j * ph, 0.0015), 0.0075, -1.4, 1.4, "paint2", sides=12)
    for i in range(-4, 5):
        P.cyl("welt", "y", (i * pw, 0.0015), 0.0075, -0.4, 2.4, "paint2", sides=12)


UPHOLSTERY_BUILDERS = {"channel": u_channel, "panel": u_panel}


# ----------------------------------------------------------------------------- the UI images

def ui_rect(coll, name, x0, y0, x1, y1, z, mat):
    me = bpy.data.meshes.new(name)
    me.from_pydata([B((x0, y0, z)), B((x1, y0, z)), B((x1, y1, z)), B((x0, y1, z))], [], [(0, 1, 2, 3)])
    me.materials.append(mat)
    ob = bpy.data.objects.new(name, me)
    coll.objects.link(ob)
    return ob


def ui_line(coll, name, pts, w, z, mat):
    """A polyline of width w: one quad per segment, joined by squares at the corners."""
    obs = []
    for i in range(len(pts) - 1):
        a, b = Vector(pts[i]), Vector(pts[i + 1])
        d = (b - a).normalized()
        n = Vector((-d.y, d.x)) * (w / 2)
        quad = [a + n, b + n, b - n, a - n]
        me = bpy.data.meshes.new(f"{name}.{i}")
        me.from_pydata([B((p.x, p.y, z)) for p in quad], [], [(0, 1, 2, 3)])
        me.materials.append(mat)
        ob = bpy.data.objects.new(me.name, me)
        coll.objects.link(ob)
        obs.append(ob)
    for p in pts[1:-1]:
        obs.append(ui_rect(coll, name + ".j", p[0] - w / 2, p[1] - w / 2, p[0] + w / 2, p[1] + w / 2, z, mat))
    return obs


class Lcg:
    """A fixed pseudo-random sequence for the UI's tick lengths and bar heights (no state shared)."""

    def __init__(self, seed):
        self.s = seed

    def __call__(self):
        self.s = (self.s * 1103515245 + 12345) & 0x7FFFFFFF
        return self.s / 0x7FFFFFFF


def build_ui(F, W, H):
    """The UI placeholder in display space (0..W, 0..H metres): emissive shapes, no real text and
    no copied interface (CLAUDE.md 15): a status header band, bar graphs, a line plot over a grid,
    a schematic of boxes and links, rows of text-like ticks, all inside a thin frame."""
    C = F["colours_srgb"]
    coll = bpy.data.collections.new("ui")
    bpy.context.scene.collection.children.link(coll)
    mats = {}
    for k in UI_ROLES:
        m = bpy.data.materials.new(k)
        emission(Nodes(m), lin(C[k]) + (1.0,))
        mats[k] = m
    rnd = Lcg(7)
    R = lambda *a: ui_rect(coll, "ui", *a)   # noqa: E731
    R(-0.1, -0.1, W + 0.1, H + 0.1, 0.0, mats["ui_bg"])
    # the frame and the header band
    m, t = 0.016, 0.006
    for x0, y0, x1, y1 in ((m, m, W - m, m + t), (m, H - m - t, W - m, H - m), (m, m, m + t, H - m), (W - m - t, m, W - m, H - m)):
        R(x0, y0, x1, y1, 0.001, mats["ui_dim"])
    hy0, hy1 = H - 0.085, H - 0.03
    R(0.03, hy0, W - 0.03, hy1, 0.001, mats["ui_fg"])
    x = 0.045
    for wtick in (0.11, 0.05, 0.08):
        R(x, hy0 + 0.015, x + wtick, hy1 - 0.015, 0.002, mats["ui_bg"])
        x += wtick + 0.025
    R(W - 0.15, hy0 + 0.01, W - 0.045, hy1 - 0.01, 0.002, mats["ui_alert"])
    # left: bar graphs in a box
    lx0, lx1, ly0, ly1 = 0.035, 0.37, 0.215, hy0 - 0.03
    for x0, y0, x1, y1 in ((lx0, ly0, lx1, ly0 + 0.006), (lx0, ly0, lx0 + 0.006, ly1)):
        R(x0, y0, x1, y1, 0.001, mats["ui_dim"])
    nb = 6
    bw = (lx1 - lx0 - 0.04) / nb
    for k in range(nb):
        h = (0.25 + 0.75 * rnd()) * (ly1 - ly0 - 0.03)
        role = "ui_alert" if k == 4 else "ui_fg"
        bx = lx0 + 0.025 + k * bw
        R(bx, ly0 + 0.012, bx + bw * 0.62, ly0 + 0.012 + h, 0.002, mats[role])
        R(bx, ly0 + 0.012 + h + 0.008, bx + bw * 0.62, ly0 + 0.012 + h + 0.014, 0.002, mats["ui_hi"])
    # right top: a line plot over a grid
    px0, px1, py0, py1 = 0.40, W - 0.035, 0.30, hy0 - 0.03
    for k in range(5):
        y = py0 + k * (py1 - py0) / 4
        R(px0, y - 0.002, px1, y + 0.002, 0.001, mats["ui_dim"])
    for k in range(7):
        xg = px0 + k * (px1 - px0) / 6
        R(xg - 0.002, py0, xg + 0.002, py1, 0.001, mats["ui_dim"])
    pts = []
    for k in range(25):
        u = k / 24
        v = 0.5 + 0.28 * math.sin(u * 7.0 + 0.6) + 0.12 * math.sin(u * 19.0)
        pts.append((px0 + u * (px1 - px0), py0 + v * (py1 - py0)))
    ui_line(coll, "plot", pts, 0.012, 0.003, mats["ui_hi"])
    pts2 = [(px0 + k / 12 * (px1 - px0), py0 + (0.25 + 0.1 * math.cos(k * 0.9)) * (py1 - py0)) for k in range(13)]
    ui_line(coll, "plot2", pts2, 0.008, 0.0025, mats["ui_fg"])
    # right bottom: a schematic of boxes and links
    sx0, sx1, sy0, sy1 = 0.40, W - 0.035, 0.035, 0.265
    boxes = [(sx0 + 0.02, sy0 + 0.13, 0.12, 0.07), (sx0 + 0.22, sy0 + 0.13, 0.12, 0.07), (sx0 + 0.42, sy0 + 0.13, 0.12, 0.07),
             (sx0 + 0.12, sy0 + 0.02, 0.12, 0.07), (sx0 + 0.36, sy0 + 0.02, 0.12, 0.07)]
    for i, (bx, by, bw2, bh) in enumerate(boxes):
        for x0, y0, x1, y1 in ((bx, by, bx + bw2, by + 0.006), (bx, by + bh - 0.006, bx + bw2, by + bh),
                               (bx, by, bx + 0.006, by + bh), (bx + bw2 - 0.006, by, bx + bw2, by + bh)):
            R(x0, y0, x1, y1, 0.002, mats["ui_fg"])
        R(bx + 0.02, by + 0.022, bx + 0.02 + 0.045 + 0.03 * rnd(), by + 0.035, 0.002, mats["ui_dim"])
        R(bx + bw2 - 0.035, by + bh - 0.03, bx + bw2 - 0.015, by + bh - 0.015, 0.003, mats["ui_alert" if i == 2 else "ui_hi"])
    for a, b in ((0, 1), (1, 2), (0, 3), (3, 4), (4, 2)):
        ax, ay, aw, ah = boxes[a]
        bx, by, bw2, bh = boxes[b]
        p, q = (ax + aw / 2, ay + ah / 2), (bx + bw2 / 2, by + bh / 2)
        ui_line(coll, "link", [p, (q[0], p[1]), q] if a != 1 else [p, q], 0.005, 0.0015, mats["ui_dim"])
    # left bottom: rows of text-like ticks
    tx0, tx1 = 0.035, 0.37
    for row in range(5):
        y = 0.04 + row * 0.033
        x = tx0
        while True:
            wt = 0.015 + 0.06 * rnd()
            if x + wt > tx1:
                break
            role = "ui_hi" if row == 4 else ("ui_fg" if rnd() > 0.25 else "ui_dim")
            R(x, y, x + wt, y + 0.014, 0.002, mats[role])
            x += wt + 0.012
    return coll


# ----------------------------------------------------------------------------- render

def render_to(path):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    bpy.context.scene.render.filepath = path
    bpy.ops.render.render(write_still=True)


def raw_path(name, kind="beauty"):
    return os.path.join(RAW, f"{name}.{kind}.exr")


def render_ui(D, fn, samples):
    """The screen image of a finish: emission only, so no light and no noise beyond antialiasing."""
    W, H = D["ui"]["screen_m"]
    scale = 4
    res = (D["ui"]["screen_px"][0] * scale, D["ui"]["screen_px"][1] * scale)
    reset(D["render"], samples, res, mask=True)
    build_ui(D["finishes"][fn], W, H)
    camera(W / 2, H / 2, W, res)
    render_to(raw_path(f"{fn}_ui"))


def render_target(D, fn, kind, item, samples):
    """One wall module, the strips of a finish (item 'strips': four renders), the keys image, a
    ceiling or floor module (kind 'ceiling' or 'floor'), or the trim rows of a finish (kind
    'trims': one render a row)."""
    R = D["render"]
    F = D["finishes"][fn]
    set_roles()
    ppm = R["px_per_m"]
    cfg = dict(R, **R[kind]) if kind in ("ceiling", "floor", "trims") else R
    FK = kind_finish(F, kind)
    streaky = kind in ("module", "strip", "keys")
    if kind == "strip":
        jobs = [(s, (int(2 * ppm), int(0.5 * ppm)), 2.0, 0.25, True) for s in STRIPS]
    elif kind == "trims":
        jobs = [(r, (int(2 * ppm), int(round(D["trims"]["rows"][r]["h_m"] * ppm))), 2.0, D["trims"]["rows"][r]["h_m"] / 2, True)
                for r in TRIM_ROWS]
    elif kind == "keys":
        kw, kh = D["ui"]["keys_m"]
        res = (D["ui"]["keys_px"][0] * 4, D["ui"]["keys_px"][1] * 4)
        jobs = [("keys", res, kw, kh / 2, False)]
    else:
        periodic = "xy" if (kind == "floor" and item == "walkway") else False
        jobs = [(item, (int(2 * ppm), int(2 * ppm)), 2.0, 1.0, periodic)]
    for sub, res, width, cy, periodic in jobs:
        if kind == "module":
            name = f"{fn}_{item}"
        elif kind in ("ceiling", "floor"):
            name = f"{fn}_{kind}_{item}"
        elif kind == "trims":
            name = f"{fn}_trim_{sub}"
        else:
            name = f"{fn}_{sub}"
        for mask in (False, True):
            reset(cfg, R["mask_samples"] if mask else samples, res, mask=mask)
            ui_img = None
            if kind == "module" and item == "screen":
                ui_img = bpy.data.images.load(raw_path(f"{fn}_ui"), check_existing=False)
            make_materials(FK, periodic, ui_img, (-0.50, 0.94, 0.50, 1.565), streaky=streaky)
            P = Panel(name)
            if kind == "strip":
                cuts = []
                for ox in (-2.0, 0.0, 2.0):
                    cuts += STRIP_BUILDERS[sub](P, ox)
                plate = plate_slab(P, strip=True)
                if cuts:
                    P.cut(plate, "cuts", cuts)
            elif kind == "trims":
                h = D["trims"]["rows"][sub]["h_m"]
                cuts = []
                for ox in (-2.0, 0.0, 2.0):
                    cuts += TRIM_BUILDERS[sub](P, ox, h, FK)
                plate = plate_slab(P, strip=True, role="paint2" if sub in ("rib", "beam") else "bulkhead")
                if cuts:
                    P.cut(plate, "cuts", cuts)
            elif kind == "keys":
                kw, kh = D["ui"]["keys_m"]
                P.box("plate", (-1.0, -1.0, -0.2), (1.0, 1.0, -0.012), "machinery")
                lit = {(0, 0): "accent", (1, 0): "accent", (10, 2): "amber", (11, 2): "amber", (5, 1): "light_panel", (11, 0): "amber"}
                pitch = kw / 12
                for r in range(3):
                    for c in range(12):
                        x0 = -kw / 2 + c * pitch + pitch * 0.1
                        y0 = r * kh / 3 + kh / 3 * 0.1
                        P.box("key", (x0, y0, -0.02), (x0 + pitch * 0.8, y0 + kh / 3 * 0.8, 0.008),
                              lit.get((c, r), "trim"), bevel=0.0035, seg=2)
            elif kind == "ceiling":
                CEILING_BUILDERS[item](P, FK, F["ceiling"]["modules"][item])
            elif kind == "floor":
                FLOOR_BUILDERS[item](P, FK, F["floor"]["modules"][item])
            else:
                MODULE_BUILDERS[item](P, F, F["modules"][item])
            camera(0.0, cy, width, res)
            if mask:
                mask_materials()
            render_to(raw_path(name, "mask" if mask else "beauty"))


# ----------------------------------------------------------------------------- post-process

def read_exr(path):
    """A rendered EXR as float32 RGB, top row first (Blender stores images bottom up)."""
    img = bpy.data.images.load(path, check_existing=False)
    w, h = img.size
    px = np.empty(w * h * 4, np.float32)
    img.pixels.foreach_get(px)
    bpy.data.images.remove(img)
    return px.reshape(h, w, 4)[::-1, :, :3].copy()


def area_average(a, f):
    h, w = a.shape[:2]
    return a.reshape(h // f, f, w // f, f, a.shape[2]).mean(axis=(1, 3))


def linear_to_srgb(x):
    x = np.clip(x, 0.0, 1.0)
    return np.where(x <= 0.0031308, x * 12.92, 1.055 * np.power(x, 1.0 / 2.4) - 0.055)


def finish_image(beauty, mask, px, colours):
    """Area-average both renders to px wide, encode, palette-reduce; alpha is the mask."""
    f = beauty.shape[1] // px
    rgb = mm.to_u8(linear_to_srgb(area_average(beauty, f)))
    if colours:
        rgb = mm.reduce_palette(rgb, colours)
    a = mm.to_u8(area_average(mask, f).max(axis=2)) if mask is not None else np.full(rgb.shape[:2], 255, np.uint8)
    return np.dstack([rgb, a])


def layer_sources(D, fn):
    """(file stem, beauty, mask) at render size for every layer of a finish; strips stacked into
    one square layer, row 0 at the bottom."""
    out = []
    for m in MODULES:
        b, k = raw_path(f"{fn}_{m}"), raw_path(f"{fn}_{m}", "mask")
        if not (os.path.exists(b) and os.path.exists(k)):
            raise SystemExit(f"[panels] missing raw render {os.path.relpath(b, ROOT)}: render it first")
        out.append((f"{fn}_{m}", read_exr(b), read_exr(k)))
    rows = D["finishes"][fn]["strips"]["rows"]
    order = sorted(STRIPS, key=lambda s: -rows[s])        # top of the image is the highest row
    bs, ks = [], []
    for s in order:
        b, k = raw_path(f"{fn}_{s}"), raw_path(f"{fn}_{s}", "mask")
        if not (os.path.exists(b) and os.path.exists(k)):
            raise SystemExit(f"[panels] missing raw render {os.path.relpath(b, ROOT)}: render it first")
        bs.append(read_exr(b))
        ks.append(read_exr(k))
    out.append((f"{fn}_strips", np.concatenate(bs, axis=0), np.concatenate(ks, axis=0)))
    F = D["finishes"][fn]
    for kind in ("ceiling", "floor"):
        for m in module_ids(F, kind):
            b, k = raw_path(f"{fn}_{kind}_{m}"), raw_path(f"{fn}_{kind}_{m}", "mask")
            if not (os.path.exists(b) and os.path.exists(k)):
                raise SystemExit(f"[panels] missing raw render {os.path.relpath(b, ROOT)}: render it first")
            out.append((f"{fn}_{kind}_{m}", read_exr(b), read_exr(k)))
    out.append((f"{fn}_trims",) + trim_layer(D, fn))
    return out


def trim_layer(D, fn):
    """The trim rows of a finish stacked into one square layer at render size: each row at its
    place (trims.rows, v0_m from the layer's bottom), and every gap filled by repeating the nearest
    row's edge, so a mip blends a row only with its own edge colour (ceilings-and-trims section 3)."""
    ppm = D["render"]["px_per_m"]
    size = int(round(D["layers"]["span_m"] * ppm))
    out = []
    for kind in ("beauty", "mask"):
        canvas = np.zeros((size, size, 3), np.float32)
        owner = np.full(size, -1, np.int64)
        for r in TRIM_ROWS:
            path = raw_path(f"{fn}_trim_{r}", kind)
            if not os.path.exists(path):
                raise SystemExit(f"[panels] missing raw render {os.path.relpath(path, ROOT)}: render it first")
            img = read_exr(path)
            R = D["trims"]["rows"][r]
            top = int(round((D["layers"]["span_m"] - R["v0_m"] - R["h_m"]) * ppm))   # image rows run top down
            if img.shape[0] != int(round(R["h_m"] * ppm)) or img.shape[1] != size:
                raise SystemExit(f"[panels] {os.path.relpath(path, ROOT)} is {img.shape[1]}x{img.shape[0]}, expected {size}x{int(round(R['h_m'] * ppm))}")
            canvas[top:top + img.shape[0]] = img
            owner[top:top + img.shape[0]] = np.arange(top, top + img.shape[0])
        covered = np.nonzero(owner >= 0)[0]
        for y in range(size):
            if owner[y] < 0:
                canvas[y] = canvas[covered[np.argmin(np.abs(covered - y))]]
        out.append(canvas)
    return out[0], out[1]


def sha256(path):
    with open(path, "rb") as f:
        return hashlib.sha256(f.read()).hexdigest()


def post(D, sheet=True):
    out_dir = os.path.join(ROOT, D["layers"]["dir"])
    colours = D["layers"]["colours"]
    files, layers = {}, {}
    for fn in FINISHES:
        for stem, beauty, mask in layer_sources(D, fn):
            for px in D["layers"]["sizes_px"]:
                img = finish_image(beauty, mask, px, colours)
                path = os.path.join(out_dir, str(px), stem + ".png")
                mm.save_png(Image.fromarray(img, "RGBA"), path)
                layers[(stem, px)] = img
        ui = read_exr(raw_path(f"{fn}_ui"))
        sw, sh = D["ui"]["screen_px"]
        img = finish_image(ui, None, sw, colours)
        mm.save_png(Image.fromarray(img, "RGBA"), os.path.join(out_dir, f"ui_screen_{fn}.png"))
        layers[(f"ui_screen_{fn}", sw)] = img
        kb, kk = read_exr(raw_path(f"{fn}_keys")), read_exr(raw_path(f"{fn}_keys", "mask"))
        img = finish_image(kb, kk, D["ui"]["keys_px"][0], colours)
        mm.save_png(Image.fromarray(img, "RGBA"), os.path.join(out_dir, f"keys_{fn}.png"))
        layers[(f"keys_{fn}", D["ui"]["keys_px"][0])] = img
    for root, _, names in os.walk(out_dir):
        for n in sorted(names):
            if n.endswith(".png"):
                p = os.path.join(root, n)
                files[os.path.relpath(p, out_dir)] = {"sha256": sha256(p), "bytes": os.path.getsize(p)}
    report(D, layers, files, out_dir)
    if sheet:
        contact_sheet(D, layers)
    return layers


def glow_fraction(img):
    return float(np.count_nonzero(img[..., 3] >= 128)) / img[..., 3].size


def all_layers(D):
    """Every panel layer: (finish, stem, layer number, declared emissive or None for strips and trims)."""
    out = []
    for fn in FINISHES:
        F = D["finishes"][fn]
        for m in MODULES:
            out.append((fn, f"{fn}_{m}", F["modules"][m]["layer"], F["modules"][m]["emissive"]))
        out.append((fn, f"{fn}_strips", F["strips"]["layer"], None))
        for kind in ("ceiling", "floor"):
            for m in module_ids(F, kind):
                M = F[kind]["modules"][m]
                out.append((fn, f"{fn}_{kind}_{m}", M["layer"], M["emissive"]))
        out.append((fn, f"{fn}_trims", F["trims"]["layer"], None))
    return out


def report(D, layers, files, out_dir):
    every = all_layers(D)
    n_layers = len(every)
    gpu = {px: n_layers * mm.gpu_bytes(px) for px in D["layers"]["sizes_px"]}
    mats = D["layers"]["first_layer"]
    total = {px: (n_layers + mats) * mm.gpu_bytes(px) for px in D["layers"]["sizes_px"]}
    rows = {}
    by_kind = {}
    for fn, stem, layer, declared in every:
        img = layers[(stem, 256)]
        emissive = bool(np.any(img[..., 3] > 0))
        if declared is not None and emissive != declared:
            raise SystemExit(f"[panels] {stem}: panels.json says emissive {declared}, the render glows on {glow_fraction(img):.1%}")
        tiles = stem.endswith("_strips") or stem.endswith("_trims") or stem.endswith("_floor_walkway")
        seam = mm.seam_ratio(img[..., :3], 1) if tiles else None
        seam_y = mm.seam_ratio(img[..., :3], 0) if stem.endswith("_floor_walkway") else None
        rows[stem] = {"layer": layer, "glow_fraction": round(glow_fraction(img), 4),
                      "seam_ratio_x": None if seam is None else round(seam, 3)}
        if seam_y is not None:
            rows[stem]["seam_ratio_y"] = round(seam_y, 3)
        kind = "trims" if stem.endswith("_trims") else "ceiling" if "_ceiling_" in stem else "floor" if "_floor_" in stem else "walls"
        by_kind[kind] = by_kind.get(kind, 0) + 1
        print(f"  {layer:>3}  {stem:<26} glows on {glow_fraction(img):6.1%}" + (f"  seam x {seam:.2f}" if seam is not None else "")
              + (f"  seam y {seam_y:.2f}" if seam_y is not None else ""))
    print("layers by kind: " + ", ".join(f"{k} {n}" for k, n in sorted(by_kind.items()))
          + "; GPU bytes with mips per kind at 256 px: " + ", ".join(f"{k} {n * mm.gpu_bytes(256):,}" for k, n in sorted(by_kind.items())))
    print(f"panel layers: {n_layers}; GPU bytes with mips: {gpu[128]:,} at 128 px (64 px/m), {gpu[256]:,} at 256 px (128 px/m)")
    print(f"whole array with the {mats} materials: {total[128]:,} at 128 px, {total[256]:,} at 256 px")
    man = {
        "schema": "starcrew.panel-layers/1",
        "_doc": "Written by tools/blender/build_wall_panels.py, never by hand: every panel layer and UI image it built, with its sha256, and what the panel layers cost. data/materials/panels.json is the source.",
        "generator": GENERATOR,
        "generator_version": D["source"]["generator_version"],
        "blender": bpy.app.version_string,
        "pillow": Image.__version__,
        "numpy": np.__version__,
        "gpu_bytes_panel_layers": {str(k): v for k, v in gpu.items()},
        "gpu_bytes_with_materials": {str(k): v for k, v in total.items()},
        "layers_by_kind": by_kind,
        "layers": rows,
        "files": files,
    }
    with open(os.path.join(out_dir, "manifest.json"), "w", encoding="utf-8") as f:
        json.dump(man, f, indent=1, sort_keys=True)
        f.write("\n")
    h = hashlib.sha256()
    for k in sorted(files):
        h.update(k.encode() + b"\0" + files[k]["sha256"].encode())
    print(f"digest: {h.hexdigest()}")


def contact_sheet(D, layers):
    """Both finishes: every wall module and the strip layer at 256 px with its name and layer, the UI
    images and an illustrative 12 m wall at 64 px per metre (base strip, a run of modules between
    ribs, the top strip); then the ceiling and floor modules and the trim layer, an illustrative
    ceiling (cells between beams, lamps in their surrounds) and floor (a walkway down the middle), and
    the trims as members: a crew pillar, engineering's tall pillar, a beam, a cove, a baseboard and a
    door jamb. The illustrations use fixed sequences, not the rule: they judge composition."""
    tile, gap, lab = 256, 14, 40
    cols = 6
    font = ImageFont.load_default(size=17)
    small = ImageFont.load_default(size=13)
    head = ImageFont.load_default(size=22)
    W = gap + cols * (tile + gap)
    sheet = Image.new("RGB", (W, 12000), (22, 24, 28))
    dr = ImageDraw.Draw(sheet)
    y = gap

    def tiles(fn, items, y):
        for i, (stem, title, sub) in enumerate(items):
            r, c = divmod(i, cols)
            x0, y0 = gap + c * (tile + gap), y + r * (lab + tile + gap)
            img = layers[(stem, 256)]
            dr.text((x0, y0), title, fill=(232, 234, 238), font=font)
            lit = float(np.count_nonzero(img[..., 3])) / img[..., 3].size
            dr.text((x0, y0 + 20), sub + (f", glows {lit:.1%}" if lit else ""), fill=(150, 156, 166), font=small)
            sheet.paste(Image.fromarray(np.ascontiguousarray(img[..., :3]), "RGB"), (x0, y0 + lab))
        return y + ((len(items) + cols - 1) // cols) * (lab + tile + gap)

    def sub_of(spec):
        return f"weight {spec['weight']:g}" if spec["weight"] else spec["placed"]

    for fn in FINISHES:
        F = D["finishes"][fn]
        dr.text((gap, y), f"{fn} finish", fill=(236, 238, 242), font=head)
        y += 36
        dr.text((gap, y), "walls", fill=(200, 204, 210), font=font)
        y += 26
        items = [(f"{fn}_{m}", f"{F['modules'][m]['layer']}  {m}", sub_of(F["modules"][m])) for m in MODULES]
        items.append((f"{fn}_strips", f"{F['strips']['layer']}  strips", "4 strips, 2 m period"))
        y = tiles(fn, items, y)
        # the UI images
        ui = layers[(f"ui_screen_{fn}", D["ui"]["screen_px"][0])]
        keys = layers[(f"keys_{fn}", D["ui"]["keys_px"][0])]
        dr.text((gap, y), f"ui_screen_{fn}.png, keys_{fn}.png (for console props)", fill=(232, 234, 238), font=font)
        sheet.paste(Image.fromarray(np.ascontiguousarray(ui[..., :3]), "RGB"), (gap, y + lab))
        sheet.paste(Image.fromarray(np.ascontiguousarray(keys[..., :3]), "RGB"), (gap + 256 + gap, y + lab))
        kk = Image.fromarray(np.ascontiguousarray(keys[..., :3]), "RGB").resize((512, 128), Image.NEAREST)
        sheet.paste(kk, (gap + 2 * (256 + gap), y + lab))
        y += lab + 210 + gap
        # an illustrative wall: a fixed sequence, not the rule (the kit's rule draws it per bay)
        seq = ["plate", "pipes", "light", "hatch", "screen", "plate"] if fn == "crew" else ["vent", "plate", "junction", "ribbed", "pipes", "light"]
        dr.text((gap, y), "illustrative 12 m wall at 64 px/m: base strip, modules between ribs, top strip (a fixed sequence, not the rule)",
                fill=(232, 234, 238), font=font)
        wall = wall_image(D, fn, seq, layers)
        sheet.paste(wall, (gap, y + lab))
        y += lab + 300 + gap
        # ceilings, floors, trims
        for kind in ("ceiling", "floor"):
            dr.text((gap, y), f"{kind} modules (2 m cells, seen from the room; panel up is the bow)", fill=(200, 204, 210), font=font)
            y += 26
            items = [(f"{fn}_{kind}_{m}", f"{F[kind]['modules'][m]['layer']}  {m}", sub_of(F[kind]["modules"][m])) for m in module_ids(F, kind)]
            if kind == "floor":
                items.append((f"{fn}_trims", f"{F['trims']['layer']}  trims", "7 rows, 2 m period"))
            y = tiles(fn, items, y)
        dr.text((gap, y), "illustrative ceiling and floor at 64 px/m, 6 m x 8 m (fixed sequences, not the rule); the trims as members at 128 px/m",
                fill=(232, 234, 238), font=font)
        y += lab
        ceil = ceiling_image(D, fn, layers)
        floor = floor_image(D, fn, layers)
        sheet.paste(ceil, (gap, y))
        sheet.paste(floor, (gap + ceil.width + gap, y))
        trims = trims_image(D, fn, layers, small)
        sheet.paste(trims, (gap + ceil.width + floor.width + 2 * gap, y))
        y += max(ceil.height, floor.height, trims.height) + 2 * gap
    sheet = sheet.crop((0, 0, W, y))
    mm.save_png(sheet, SHEET)
    print("contact sheet:", os.path.relpath(SHEET, ROOT))


def row_image(D, fn, layers, row, px=256):
    """One trim row of a finish's layer at `px` (rows of the image: top = high v)."""
    img = layers[(f"{fn}_trims", px)][..., :3]
    R = D["trims"]["rows"][row]
    k = px / D["layers"]["span_m"]
    top = int(round((D["layers"]["span_m"] - R["v0_m"] - R["h_m"]) * k))
    return img[top:top + int(round(R["h_m"] * k))]


def ceiling_image(D, fn, layers):
    """A 6 m x 8 m ceiling seen from below at 64 px/m: three cells across, four bays, the beams
    (their undersides, the beam row) at the bay joints, a lamp housing in the middle cells."""
    seq = ["grille", "plate", "pipes", "ribbed", "cable_tray", "plate", "fan", "hatch"]
    ppm, cell = 64, 128
    img = np.zeros((4 * cell, 3 * cell, 3), np.uint8)
    k = 0
    for b in range(4):
        for c in range(3):
            m = "lamp_surround" if c == 1 else seq[k % len(seq)]
            if c != 1:
                k += 1
            img[(3 - b) * cell:(4 - b) * cell, c * cell:(c + 1) * cell] = layers[(f"{fn}_ceiling_{m}", 128)][..., :3]
            if c == 1:
                lw, lh = int(0.9 * ppm), int(0.45 * ppm)
                cx, cy = cell + cell // 2, (3 - b) * cell + cell // 2
                img[cy - lh // 2:cy + lh // 2, cx - lw // 2:cx + lw // 2] = (70, 72, 76)
                img[cy - lh // 2 + 3:cy + lh // 2 - 3, cx - lw // 2 + 3:cx + lw // 2 - 3] = (250, 238, 210)
    beam = row_image(D, fn, layers, "beam", 128)
    bw = int(round(0.24 * ppm))
    beam = np.asarray(Image.fromarray(np.ascontiguousarray(beam)).resize((beam.shape[1], bw), Image.NEAREST))
    for b in range(1, 4):
        y = b * cell
        strip = np.tile(beam, (1, 4, 1))[:, :img.shape[1]]
        img[y - bw // 2:y - bw // 2 + bw] = strip
    return Image.fromarray(img, "RGB")


def floor_image(D, fn, layers):
    """A 6 m x 8 m floor seen from above at 64 px/m: the walkway down the middle column, other cells
    from a fixed sequence."""
    mods = module_ids(D["finishes"][fn], "floor")
    seq = [m for m in ("grate", "plate", "access", "trench", "plate", "drain", "vent", "hazard", "grate", "plate") if m in mods]
    cell = 128
    img = np.zeros((4 * cell, 3 * cell, 3), np.uint8)
    k = 0
    for b in range(4):
        for c in range(3):
            m = "walkway" if c == 1 else seq[k % len(seq)]
            if c != 1:
                k += 1
            img[(3 - b) * cell:(4 - b) * cell, c * cell:(c + 1) * cell] = layers[(f"{fn}_floor_{m}", 128)][..., :3]
    return Image.fromarray(img, "RGB")


def member_face(D, fn, layers, length_m, ppm, tall=False, pillar=True, row="rib"):
    """A rib's front face (or any member's, pillar False) as the kit maps it: u up the member, v
    across it; with pillar, the base and capital pieces at its ends. Returns an image, top = up."""
    T = D["trims"]
    lay = layers[(f"{fn}_trims", 256)][..., :3]
    k256 = 256 / D["layers"]["span_m"]

    def vertical(rowimg):
        return np.transpose(rowimg[::-1], (1, 0, 2))[::-1]   # u up the image, v left to right

    def take(row_name, u0_m, u1_m, out_px):
        r = row_image(D, fn, layers, row_name, 256)
        a, b = int(round(u0_m * k256)), int(round(u1_m * k256))
        seg = r[:, a:b] if b <= r.shape[1] else np.concatenate([r[:, a:], r[:, :b - r.shape[1]]], axis=1)
        v = vertical(seg)
        w = int(round(T["rows"][row_name]["h_m"] * ppm * 0.96))
        return np.asarray(Image.fromarray(np.ascontiguousarray(v)).resize((w, max(1, out_px)), Image.NEAREST))

    del lay
    parts = []
    if pillar:
        pl = T["pillar"]
        bm, cm = (pl["tall_base_m"], pl["tall_capital_m"]) if tall else (pl["base_m"], pl["capital_m"])
        pc, pb = T["pieces"]["rib_capital"], T["pieces"]["rib_base"]
        span = D["layers"]["span_m"]
        parts.append(take("rib_ends", span / 2 + pc["centre_m"] - pc["length_m"] / 2, span / 2 + pc["centre_m"] + pc["length_m"] / 2, int(cm * ppm)))
        shaft = length_m - bm - cm
    else:
        shaft = length_m
    n = int(round(shaft * ppm))
    reps = []
    left = n
    while left > 0:
        seg = take(row, 0.0, D["layers"]["span_m"], int(D["layers"]["span_m"] * ppm))
        reps.append(seg[:left] if left < seg.shape[0] else seg)
        left -= seg.shape[0]
    parts.append(np.concatenate(reps, axis=0))
    if pillar:
        parts.append(take("rib_ends", span / 2 + pb["centre_m"] - pb["length_m"] / 2, span / 2 + pb["centre_m"] + pb["length_m"] / 2, int(bm * ppm)))
    w = min(p.shape[1] for p in parts)
    return np.concatenate([p[:, :w] for p in parts], axis=0)


def trims_image(D, fn, layers, font):
    """The trims as members, at 128 px/m: a 2.65 m crew pillar and engineering's 9.4 m pillar (at
    half scale), a beam's side, a cove, a baseboard and a door jamb's face."""
    ppm = 128
    bg = (40, 42, 46)
    crew = member_face(D, fn, layers, 2.65, ppm)
    tall = member_face(D, fn, layers, 9.4, ppm // 2, tall=True)
    jamb = member_face(D, fn, layers, 2.2, ppm, pillar=False, row="frame")
    H = max(crew.shape[0], tall.shape[0], jamb.shape[0]) + 30
    horiz = []
    for row, length, label in (("beam", 4.0, "beam side, 4 m"), ("cove", 4.0, "cove, 4 m"), ("baseboard", 4.0, "baseboard, 4 m"),
                               ("side", 4.0, "side faces, 4 m")):
        r = row_image(D, fn, layers, row, 256)
        n = int(length * ppm)
        horiz.append((np.tile(r, (1, n // r.shape[1] + 1, 1))[:, :n], label))
    Wd = 3 * 70 + int(4.0 * ppm) + 60
    img = Image.new("RGB", (Wd, max(H, sum(h.shape[0] + 34 for h, _ in horiz) + 10)), bg)
    dr = ImageDraw.Draw(img)
    x = 10
    for face, label in ((crew, "crew"), (tall, "eng. 1/2"), (jamb, "jamb")):
        img.paste(Image.fromarray(np.ascontiguousarray(face)), (x, 24))
        dr.text((x, 4), label, fill=(200, 204, 210), font=font)
        x += 70
    yy = 24
    x0 = x + 20
    for h, label in horiz:
        dr.text((x0, yy - 18), label, fill=(200, 204, 210), font=font)
        img.paste(Image.fromarray(np.ascontiguousarray(h)), (x0, yy))
        yy += h.shape[0] + 34
    return img


def wall_image(D, fn, seq, layers):
    """A 12 m x 3.0 m crew-height wall (2.65 m flat under a 0.35 m cove) at 64 px per metre, with
    0.24 m ribs drawn as plain dark bars at the bay joints."""
    ppm = 64
    strips = layers[(f"{fn}_strips", 128)][..., :3]
    rows = D["finishes"][fn]["strips"]["rows"]
    q = 128 // 4

    def strip(name):
        r = rows[name]
        return strips[128 - (r + 1) * q:128 - r * q]

    H = int(2.65 * ppm)
    img = np.zeros((H + int(0.35 * ppm), len(seq) * 128, 3), np.uint8)
    img[:] = (40, 42, 46)
    base = np.tile(strip("base"), (1, len(seq), 1))
    top = np.tile(strip("top"), (1, len(seq), 1))
    yb = img.shape[0] - 32
    img[yb:yb + 32] = base
    for i, m in enumerate(seq):
        img[yb - 128:yb, i * 128:(i + 1) * 128] = layers[(f"{fn}_{m}", 128)][..., :3]
    th = H - 32 - 128
    img[yb - 128 - th:yb - 128] = top[32 - th:32]
    for k in range(len(seq) + 1):
        x = k * 128
        img[int(0.35 * ppm):, max(0, x - 8):x + 8] = (58, 60, 64)
    return Image.fromarray(img, "RGB")


# ----------------------------------------------------------------------------- main

def parse_args():
    argv = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else sys.argv[1:]
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--only", default=None)
    ap.add_argument("--samples", type=int, default=None)
    ap.add_argument("--post-only", action="store_true")
    ap.add_argument("--no-sheet", action="store_true")
    return ap.parse_args(argv)


def targets(D):
    """(finish, kind, item, target name) for everything the build renders, in a fixed order."""
    out = []
    for fn in FINISHES:
        out.append((fn, "ui", "ui", f"{fn}_ui"))
        for m in MODULES:
            out.append((fn, "module", m, f"{fn}_{m}"))
        out.append((fn, "strip", "strips", f"{fn}_strips"))
        out.append((fn, "keys", "keys", f"{fn}_keys"))
        for kind in ("ceiling", "floor"):
            for m in module_ids(D["finishes"][fn], kind):
                out.append((fn, kind, m, f"{fn}_{kind}_{m}"))
        out.append((fn, "trims", "trims", f"{fn}_trims"))
    return out


def main():
    args = parse_args()
    D = load_panels()
    if not args.post_only:
        only = set(args.only.split(",")) if args.only else None
        samples = args.samples or D["render"]["samples"]
        names = {t[3] for t in targets(D)}
        if only and only - names:
            raise SystemExit(f"[panels] --only names no target: {', '.join(sorted(only - names))}")
        for fn, kind, item, name in targets(D):
            if only and name not in only:
                continue
            print(f"[panels] rendering {name}", flush=True)
            if kind == "ui":
                render_ui(D, fn, 16)
            else:
                render_target(D, fn, kind, item, samples)
    post(D, sheet=not args.no_sheet)


if __name__ == "__main__":
    main()
