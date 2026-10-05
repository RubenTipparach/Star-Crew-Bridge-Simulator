"""Star Crew's wall panel modules and strips, modelled in Blender the hard-surface CSG way and baked
to texture layers (openspec/changes/wall-panels, design section 6, option B: the prototype).

It owns the panel layers (assets/textures/panels/<px>/<finish>_<module>.png and
<finish>_strips.png at 256 and 128 px), the reusable screen and key images
(assets/textures/panels/ui_screen_<finish>.png, keys_<finish>.png), their manifest
(assets/textures/panels/manifest.json) and the contact sheet
(docs/screenshots/materials/panels-contact-sheet.png). It lives in tools/blender because the
panels are modelled, not painted: every module is low relief carved with boolean cutters,
chamfered and baked under one fixed light, the way the bridge props are built
(tools/blender/build_bridge_props.py, whose primitives and Prop class it imports rather than
copies; the blender-hard-surface skill). data/materials/panels.json is the one source for what
is built: sizes, colours, wear, layers and render settings.

Why Blender and not Material Maker (CLAUDE.md section 9 asks for graphs): Material Maker cannot run
in a cloud session (it needs Godot 4.7, and GitHub release downloads return 403 here). Question V2
in the wall-panels design asks the owner which way panels are made from now on.

Run (from anywhere), with Pillow installed beside bpy (pip install bpy pillow):
  <python with the bpy module> tools/blender/build_wall_panels.py [--only crew_plate,working_strips,...]
      [--samples N] [--post-only] [--no-sheet]
  --only       render only these targets (<finish>_<module>, <finish>_strips, <finish>_ui, <finish>_keys);
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
that is periodic in x, so the layer tiles along a wall.

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
import build_bridge_props as kit  # noqa: E402  the hard-surface kit: prism, obox, ngon, frame, Prop, apply_modifiers
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

# Material roles. The first seven are the props kit's slot order (kit.ROLES); a panel uses them
# with these meanings, plus its own. kit.ROLES is extended at build time (set_roles), so the kit's
# primitives give every mesh all of them in one fixed order.
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
}
EXTRA_ROLES = ("paint2", "rubber", "stencil", "amber", "safety")
EMISSIVE = {"light_panel", "screen", "accent", "amber"}
WORN = {"bulkhead", "paint2", "machinery", "trim", "hazard", "safety"}   # take edge wear and rust
UI_ROLES = ("ui_bg", "ui_dim", "ui_fg", "ui_hi", "ui_alert")

I4 = Matrix.Identity(4)


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


def load_panels():
    """Read and validate panels.json. Every key the build uses is checked; unknown keys stop it."""
    with open(PANELS_JSON, encoding="utf-8") as f:
        d = json.load(f)
    if d.get("schema") != SCHEMA:
        fail(f"schema must be {SCHEMA!r}")
    keys_exact(d, {"schema", "status", "layers", "bands", "bays", "rule", "glow", "render", "ui", "finishes", "source"}, "top level")
    keys_exact(d["layers"], {"first_layer", "span_m", "margin_m", "sizes_px", "colours", "dir"}, "layers")
    with open(MATERIALS_JSON, encoding="utf-8") as f:
        n_mats = len(json.load(f)["materials"])
    if d["layers"]["first_layer"] != n_mats:
        fail(f"layers.first_layer is {d['layers']['first_layer']}, but materials.json has {n_mats} materials")
    if sorted(d["layers"]["sizes_px"]) != [128, 256]:
        fail("layers.sizes_px must be [128, 256] (64 and 128 px per metre over 2 m)")
    keys_exact(d["bands"], {"base_m", "module_m", "strip_m", "base", "between", "top"}, "bands")
    keys_exact(d["bays"], {"full_min_m", "full_max_m", "narrow_min_m", "narrow_feature_m"}, "bays")
    keys_exact(d["rule"], {"seed", "hash", "key", "door_kinds"}, "rule")
    keys_exact(d["glow"], {"normal", "red_alert", "emergency"}, "glow")
    for k, v in d["glow"].items():
        num(v, f"glow.{k}", 0, 1)
    r = d["render"]
    keys_exact(r, {"px_per_m", "samples", "mask_samples", "key_light_tangent", "key_angle_deg", "ambient", "bounces"}, "render")
    num(r["ambient"], "render.ambient", 0, 0.95)
    if len(r["key_light_tangent"]) != 3 or r["key_light_tangent"][2] <= 0:
        fail("render.key_light_tangent must be [x, y, z] with z above 0 (out of the wall)")
    keys_exact(d["ui"], {"screen_m", "screen_px", "keys_m", "keys_px"}, "ui")
    keys_exact(d["finishes"], set(FINISHES), "finishes")
    layers_seen = []
    colour_keys = {"plate", "paint2", "dark", "metal", "hazard_a", "hazard_b", "safety", "rubber", "stencil", "light",
                   "amber", "status", "rust"} | set(UI_ROLES)
    for fn in FINISHES:
        F = d["finishes"][fn]
        w = f"finishes.{fn}"
        keys_exact(F, {"colours_srgb", "wear", "modules", "strips"}, w)
        keys_exact(F["colours_srgb"], colour_keys, w + ".colours_srgb")
        for k, c in F["colours_srgb"].items():
            if not (isinstance(c, list) and len(c) == 3):
                fail(f"{w}.colours_srgb.{k} must be [r, g, b]")
            for v in c:
                num(v, f"{w}.colours_srgb.{k}", 0, 1)
        keys_exact(F["wear"], {"grime", "grime_scale_per_m", "crevice", "crevice_m", "edge", "streaks", "rust"}, w + ".wear")
        for k, v in F["wear"].items():
            num(v, f"{w}.wear.{k}", 0, 100)
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
    first = d["layers"]["first_layer"]
    if sorted(layers_seen) != list(range(first, first + len(layers_seen))):
        fail(f"panel layers must be numbered {first}-{first + len(layers_seen) - 1} once each, got {sorted(layers_seen)}")
    return d


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
    strips: x is wrapped onto a circle of circumference 2 m, so the noise repeats exactly."""
    geo = N.new("ShaderNodeNewGeometry")
    sep = N.new("ShaderNodeSeparateXYZ")
    N.nt.links.new(geo.outputs["Position"], sep.inputs[0])
    X, Yb, Z = sep.outputs[0], sep.outputs[1], sep.outputs[2]   # Blender: y is -depth, z is up
    if not periodic:
        return N.xyz(X, Z, Yb), X, Z
    th = N.math("MULTIPLY", X, math.pi)
    r = 1.0 / math.pi
    return N.xyz(N.math("MULTIPLY", N.math("COSINE", th), r), N.math("MULTIPLY", N.math("SINE", th), r),
                 N.math("ADD", Z, N.math("MULTIPLY", Yb, 0.5))), X, Z


def worn(N, base, F, role, periodic):
    """Base colour (a socket or an RGBA) under grime, crevice occlusion, edge wear and, for a
    working finish, rust streaks. Returns a colour socket."""
    W, C = F["wear"], F["colours_srgb"]
    v, X, Z = coords(N, periodic)
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


def diffuse(N, col):
    d = N.new("ShaderNodeBsdfDiffuse")
    N.feed(d.inputs["Color"], col)
    d.inputs["Roughness"].default_value = 1.0
    N.out(d.outputs[0])


def emission(N, col, strength=1.0):
    e = N.new("ShaderNodeEmission")
    N.feed(e.inputs["Color"], col)
    e.inputs["Strength"].default_value = strength
    N.out(e.outputs[0])


def make_materials(F, periodic, ui_image=None, ui_rect=None):
    """One material per role for a finish. ui_image (a loaded bpy image) is shown on `screen`
    faces, mapped onto ui_rect (x0, y0, x1, y1 in panel space)."""
    C = F["colours_srgb"]
    for role in kit.ROLES:
        m = bpy.data.materials.new(role)
        N = Nodes(m)
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
                key = ROLE_COLOUR[role] or "ui_fg"
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
            base = lin(C[ROLE_COLOUR[role]]) + (1.0,)
        diffuse(N, worn(N, base, F, role, periodic))
        m["emissive"] = False


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

def plate_slab(P, strip=False):
    if strip:
        return P.box("plate", (-3.7, -0.4, -0.25), (3.7, 0.9, 0.0), "bulkhead")
    return P.box("plate", (-1.7, -0.6, -0.25), (1.7, 2.6, 0.0), "bulkhead")


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
    """One module, the strips of a finish (item 'strips': four renders), or the keys image."""
    R = D["render"]
    F = D["finishes"][fn]
    set_roles()
    ppm = R["px_per_m"]
    if kind == "strip":
        jobs = [(s, (int(2 * ppm), int(0.5 * ppm)), 2.0, 0.25, True) for s in STRIPS]
    elif kind == "keys":
        kw, kh = D["ui"]["keys_m"]
        res = (D["ui"]["keys_px"][0] * 4, D["ui"]["keys_px"][1] * 4)
        jobs = [("keys", res, kw, kh / 2, False)]
    else:
        jobs = [(item, (int(2 * ppm), int(2 * ppm)), 2.0, 1.0, False)]
    for sub, res, width, cy, periodic in jobs:
        name = f"{fn}_{sub}" if kind != "module" else f"{fn}_{item}"
        for mask in (False, True):
            reset(R, R["mask_samples"] if mask else samples, res, mask=mask)
            ui_img = None
            if kind == "module" and item == "screen":
                ui_img = bpy.data.images.load(raw_path(f"{fn}_ui"), check_existing=False)
            make_materials(F, periodic, ui_img, (-0.50, 0.94, 0.50, 1.565))
            P = Panel(name)
            if kind == "strip":
                cuts = []
                for ox in (-2.0, 0.0, 2.0):
                    cuts += STRIP_BUILDERS[sub](P, ox)
                plate = plate_slab(P, strip=True)
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
    return out


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


def report(D, layers, files, out_dir):
    n_layers = 2 * (len(MODULES) + 1)
    gpu = {px: n_layers * mm.gpu_bytes(px) for px in D["layers"]["sizes_px"]}
    mats = D["layers"]["first_layer"]
    total = {px: (n_layers + mats) * mm.gpu_bytes(px) for px in D["layers"]["sizes_px"]}
    rows = {}
    for fn in FINISHES:
        F = D["finishes"][fn]
        for m in MODULES + ("strips",):
            stem = f"{fn}_{m}"
            img = layers[(stem, 256)]
            layer = F["strips"]["layer"] if m == "strips" else F["modules"][m]["layer"]
            emissive = bool(np.any(img[..., 3] > 0))
            declared = True if m == "strips" else F["modules"][m]["emissive"]
            if m != "strips" and emissive != declared:
                raise SystemExit(f"[panels] {stem}: panels.json says emissive {declared}, the render glows on {glow_fraction(img):.1%}")
            seam = mm.seam_ratio(img[..., :3], 1) if m == "strips" else None
            rows[stem] = {"layer": layer, "glow_fraction": round(glow_fraction(img), 4),
                          "seam_ratio_x": None if seam is None else round(seam, 3)}
            print(f"  {layer:>3}  {stem:<18} glows on {glow_fraction(img):6.1%}" + (f"  seam x {seam:.2f}" if seam is not None else ""))
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
    """Both finishes: every module and the strip layer at 256 px with its name and layer, the UI
    images, and an illustrative 12 m wall of each finish at 64 px per metre (base strip, a run of
    modules between ribs, the top strip) so the composition can be judged, not only the tiles."""
    tile, gap, lab = 256, 14, 40
    cols = 6
    font = ImageFont.load_default(size=17)
    small = ImageFont.load_default(size=13)
    head = ImageFont.load_default(size=22)
    wall_w = 12 * 64
    W = gap + cols * (tile + gap)
    per_finish = 36 + 2 * (lab + tile + gap) + (lab + 210 + gap) + (lab + 300 + gap)
    sheet = Image.new("RGB", (W, gap + 2 * per_finish), (22, 24, 28))
    dr = ImageDraw.Draw(sheet)
    y = gap
    for fn in FINISHES:
        F = D["finishes"][fn]
        dr.text((gap, y), f"{fn} finish", fill=(236, 238, 242), font=head)
        y += 36
        items = list(MODULES) + ["strips"]
        for i, m in enumerate(items):
            r, c = divmod(i, cols)
            x0, y0 = gap + c * (tile + gap), y + r * (lab + tile + gap)
            img = layers[(f"{fn}_{m}", 256)]
            layer = F["strips"]["layer"] if m == "strips" else F["modules"][m]["layer"]
            spec = F["modules"].get(m, {})
            sub = "4 strips, 2 m period" if m == "strips" else (f"weight {spec['weight']:g}" if spec["weight"] else spec["placed"])
            dr.text((x0, y0), f"{layer}  {m}", fill=(232, 234, 238), font=font)
            lit = float(np.count_nonzero(img[..., 3])) / img[..., 3].size
            dr.text((x0, y0 + 20), f"{sub}" + (f", glows {lit:.1%}" if lit else ""), fill=(150, 156, 166), font=small)
            sheet.paste(Image.fromarray(np.ascontiguousarray(img[..., :3]), "RGB"), (x0, y0 + lab))
        y += 2 * (lab + tile + gap)
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
    mm.save_png(sheet, SHEET)
    print("contact sheet:", os.path.relpath(SHEET, ROOT))


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


def targets():
    out = []
    for fn in FINISHES:
        out.append((fn, "ui", "ui"))
        for m in MODULES:
            out.append((fn, "module", m))
        out.append((fn, "strip", "strips"))
        out.append((fn, "keys", "keys"))
    return out


def main():
    args = parse_args()
    D = load_panels()
    if not args.post_only:
        only = set(args.only.split(",")) if args.only else None
        samples = args.samples or D["render"]["samples"]
        for fn, kind, item in targets():
            name = f"{fn}_{item}"
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
