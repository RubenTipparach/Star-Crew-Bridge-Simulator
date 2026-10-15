#!/usr/bin/env python3
"""Author the Material Maker (.ptex) graphs Star Crew's materials are rendered from.

It owns the graph definitions for the custom materials in tools/materials/ptex/, and lives in
tools/ because a graph is an asset source: Material Maker renders it, and
tools/materials/postprocess.py turns the render into the game's texture layers
(data/materials/materials.json says which graph each material uses). Nothing in the game reads a
.ptex.

Provenance: ported from fps-game-demo (Undercity/Brushfire) at revision f6cd25c,
tools/material_maker/build_ptex.py. The graph functions, the Graph helper and the diamond plate
patch are that file's code unchanged; the port drops the graphs Star Crew does not use (concrete)
and the patches for Material Maker examples it does not ship (lava, brick wall), and writes into
this directory's ptex/. Running it reproduces the fps graphs byte for byte.

Material Maker stores graphs as JSON. This script builds the custom graphs from code so they are
reproducible and diff-friendly; every generated .ptex opens normally in Material Maker 1.7+ for
hand tweaking. Once you edit a graph in Material Maker, remove its entry from GRAPHS below (or it
will be overwritten).

Run:  python3 tools/materials/build_ptex.py
Then: tools/materials/export_materials.sh   (renders and post-processes the textures)
"""
import json
import os

HERE = os.path.dirname(os.path.abspath(__file__))
OUT_DIR = os.path.join(HERE, "ptex")

# Material inputs on the "material" node (Material Maker's static PBR output).
ALBEDO, METALLIC, ROUGHNESS, EMISSION, NORMAL, AO, DEPTH = range(7)


def rgb(r, g, b, a=1.0):
    return {"r": r, "g": g, "b": b, "a": a, "type": "Color"}


def hexc(h):
    h = h.lstrip("#")
    return tuple(int(h[i:i + 2], 16) / 255.0 for i in (0, 2, 4))


def grad(*stops, interpolation=1):
    """stops: (pos, '#rrggbb') or (pos, (r, g, b)) or (pos, grey_float)."""
    pts = []
    for pos, c in stops:
        if isinstance(c, str):
            c = hexc(c)
        elif isinstance(c, (int, float)):
            c = (c, c, c)
        pts.append({"pos": pos, "r": c[0], "g": c[1], "b": c[2], "a": 1})
    return {"interpolation": interpolation, "points": pts, "type": "Gradient"}


def curve(*pts):
    """Tonality curve points: (x, y) pairs, linear tangents."""
    out = []
    for i, (x, y) in enumerate(pts):
        out.append({"x": x, "y": y, "ls": 0.0, "rs": 0.0})
    # compute simple slopes
    for i in range(len(out)):
        if i > 0:
            dx = out[i]["x"] - out[i - 1]["x"]
            out[i]["ls"] = (out[i]["y"] - out[i - 1]["y"]) / dx if dx else 0
        if i < len(out) - 1:
            dx = out[i + 1]["x"] - out[i]["x"]
            out[i]["rs"] = (out[i + 1]["y"] - out[i]["y"]) / dx if dx else 0
    return {"points": out, "type": "Curve"}


class Graph:
    def __init__(self, label):
        self.label = label
        self.nodes = []
        self.conns = []
        self.counter = {}
        self.col = 0

    def node(self, type_, *inputs, name=None, **params):
        """Create a node. inputs: sequence of (node_name, port) or node_name (port 0) or None."""
        idx = self.counter.get(type_, 0)
        self.counter[type_] = idx + 1
        name = name or f"{type_}_{idx}"
        self.col += 1
        n = {
            "name": name,
            "type": type_,
            "node_position": {"x": 220 * (self.col % 8), "y": 160 * (self.col // 8)},
            "parameters": params,
        }
        self.nodes.append(n)
        for port, src in enumerate(inputs):
            if src is None:
                continue
            self.link(src, name, port)
        return name

    def link(self, src, dst, dst_port):
        if isinstance(src, tuple):
            s, sp = src
        else:
            s, sp = src, 0
        self.conns.append({"from": s, "from_port": sp, "to": dst, "to_port": dst_port})

    def material(self, albedo=None, metallic=None, roughness=None, emission=None,
                 normal=None, ao=None, depth=None, metallic_v=1.0, roughness_v=1.0,
                 emission_energy=1.0, normal_v=1.0, ao_v=1.0):
        name = self.node(
            "material", name="Material",
            albedo_color=rgb(1, 1, 1), metallic=metallic_v, roughness=roughness_v,
            emission_energy=emission_energy, normal=normal_v, ao=ao_v,
            depth_scale=0.0, flags_transparent=False, sss=0.0, size=11)
        self.nodes[-1]["export_paths"] = {}
        for port, src in ((ALBEDO, albedo), (METALLIC, metallic), (ROUGHNESS, roughness),
                          (EMISSION, emission), (NORMAL, normal), (AO, ao), (DEPTH, depth)):
            if src is not None:
                self.link(src, name, port)
        return name

    # --- convenience wrappers -------------------------------------------------
    def colorize(self, src, gradient):
        return self.node("colorize", src, gradient=gradient)

    def blend(self, top, bottom, mask=None, mode=0, amount=1.0):
        # Material Maker blend: s1 (top) over s2 (bottom) using mask * amount.
        return self.node("blend", top, bottom, mask, blend_type=mode, amount=amount)

    def math(self, a, b=None, op=0, in1=0.0, in2=0.0, clamp=False):
        return self.node("math", a, b, op=op, default_in1=in1, default_in2=in2, clamp=clamp)

    def normal_map(self, height, strength=1.0):
        return self.node("normal_map", height, param0=11, param1=strength, param2=0, param4=1)

    def occlusion(self, height, strength=1.5, radius=20):
        return self.node("occlusion2", height, param0=11, param1=radius, param2=strength, param3=1)

    def tone(self, src, *pts):
        return self.node("tonality", src, curve=curve(*pts))

    def grey(self, v):
        return self.node("uniform_greyscale", color=v)

    def color(self, c):
        if isinstance(c, str):
            c = hexc(c)
        return self.node("uniform", color=rgb(*c))

    def to_json(self):
        return {
            "name": "Material", "label": self.label, "type": "graph",
            "node_position": {"x": 0, "y": 0},
            "parameters": {}, "connections": self.conns, "nodes": self.nodes,
        }


# Math op indices (see Material Maker math node)
ADD, SUB, MUL, DIV = 0, 1, 2, 3
POW, ABS = 6, 7
MIN, MAX = 13, 14
LT = 15
SMOOTH = 20

# Blend modes
NORMAL_B, DISSOLVE, MULTIPLY, SCREEN, OVERLAY, HARD_LIGHT, SOFT_LIGHT = range(7)
BURN, DODGE, LIGHTEN, DARKEN, DIFFERENCE, ADDITIVE = range(7, 13)


# ----------------------------------------------------------------------------
# Custom materials
# ----------------------------------------------------------------------------

def tech_panel():
    """Painted steel wall panels with bevels, bolts, grime and chipped paint."""
    g = Graph("Tech Panel")
    panels = g.node("bricks", pattern=0, repeat=1, rows=2, columns=2, row_offset=0.0,
                    mortar=0.035, bevel=0.06, round=0.02, corner=0.0)
    # bolts: 4 small circles per panel near the corners. sdrepeat works in texture
    # units centred on 0.5, so offsets/radii are relative to the whole texture.
    corners = []
    for cx, cy in ((-0.085, -0.085), (0.085, 0.085), (0.085, -0.085), (-0.085, 0.085)):
        c = g.node("sdcircle", r=0.009, cx=cx, cy=cy)
        corners.append(g.node("sdrepeat", c, rx=2, ry=2, r=0.0, variations=False))
    u1 = g.node("sdboolean", corners[0], corners[1], op=0)
    u2 = g.node("sdboolean", corners[2], corners[3], op=0)
    bolts_sdf = g.node("sdboolean", u1, u2, op=0)
    bolts = g.node("sdshow", bolts_sdf, bevel=0.007, base=0.0)
    # horizontal groove strip in the middle of each panel
    groove = g.node("pattern", mix=0, x_wave=4, x_scale=1, y_wave=2, y_scale=4)
    groove_t = g.tone(groove, (0, 0), (1, 0.15))
    # noise
    fbm = g.node("fbm2", noise=2, scale_x=8, scale_y=8, folds=0, iterations=6, persistence=0.55, offset=0)
    fine = g.node("fbm2", noise=0, scale_x=32, scale_y=32, folds=0, iterations=4, persistence=0.5, offset=0.3)
    scratch = g.node("scratches", length=0.15, width=0.25, layers=3, waviness=0.4, angle=-15, randomness=0.6)

    # height = panels - groove + bolts + fine noise
    h1 = g.math(g.math(panels, None, op=MUL, in2=0.8), groove_t, op=SUB, clamp=True)
    h2 = g.math(h1, g.math(bolts, None, op=MUL, in2=0.18), op=ADD)
    fine_s = g.math(fine, None, op=MUL, in2=0.03)
    height = g.math(h2, fine_s, op=ADD)

    # paint wear mask: edges of panels + noise
    edge = g.tone(panels, (0, 1), (0.35, 1), (0.6, 0), (1, 0))
    wear_n = g.tone(fbm, (0, 0), (0.55, 0), (0.75, 1), (1, 1))
    wear = g.math(edge, wear_n, op=MUL, clamp=True)
    scr = g.math(scratch, None, op=MUL, in2=0.3)
    wear2 = g.math(wear, scr, op=MAX)

    paint_var = g.node("bricks", pattern=0, repeat=1, rows=2, columns=2, row_offset=0.0,
                       mortar=0.035, bevel=0.06, round=0.02, corner=0.0)
    tint = g.node("greyscale", (paint_var, 1), mode=2)
    paint = g.colorize(tint, grad((0, "#5d6f7b"), (1, "#74868f")))
    grime = g.colorize(fbm, grad((0, "#4a4f55"), (1, "#ffffff")))
    paint_d = g.blend(grime, paint, None, mode=MULTIPLY, amount=0.6)
    metal = g.colorize(fine, grad((0, "#7d7f82"), (1, "#a3a5a8")))
    albedo_a = g.blend(metal, paint_d, wear2, mode=NORMAL_B, amount=1.0)
    mortar_dark = g.tone(panels, (0, 0.3), (0.1, 1), (1, 1))
    albedo = g.blend(mortar_dark, albedo_a, None, mode=MULTIPLY, amount=1.0)

    metallic = g.tone(wear2, (0, 0.15), (1, 1))
    rough = g.math(g.colorize(fbm, grad((0, 0.45), (1, 0.7))), wear2, op=SUB, clamp=True)
    rough2 = g.math(rough, None, op=MAX, in2=0.25)
    normal = g.normal_map(height, strength=1.2)
    ao = g.occlusion(height, strength=1.2, radius=12)
    g.material(albedo=albedo, metallic=metallic, roughness=rough2, normal=normal, ao=ao)
    return g


def ceiling_tiles():
    """Perforated acoustic ceiling tiles in a steel grid."""
    g = Graph("Ceiling Tiles")
    tiles = g.node("bricks", pattern=0, repeat=1, rows=2, columns=2, row_offset=0.0,
                   mortar=0.05, bevel=0.02, round=0.0, corner=0.0)
    holes_c = g.node("sdcircle", r=0.0065, cx=0, cy=0)
    holes_r = g.node("sdrepeat", holes_c, rx=24, ry=24, r=0.0, variations=False)
    holes = g.node("sdshow", holes_r, bevel=0.003, base=0.0)
    inner = g.tone(tiles, (0, 0), (0.5, 0), (0.7, 1), (1, 1))
    holes_m = g.math(holes, inner, op=MUL)
    n = g.node("fbm2", noise=2, scale_x=6, scale_y=6, folds=0, iterations=6, persistence=0.6, offset=0.3)
    water = g.tone(n, (0, 0), (0.6, 0), (0.85, 1), (1, 1))
    height = g.math(tiles, g.math(holes_m, None, op=MUL, in2=0.3), op=SUB)
    grid = g.tone(tiles, (0, 1), (0.05, 0), (1, 0))
    tile_c = g.colorize(n, grad((0, "#b8b4aa"), (1, "#cfcbc0")))
    stain = g.color("#7a6a4c")
    tile_s = g.blend(stain, tile_c, water, mode=MULTIPLY, amount=0.55)
    hole_dark = g.color("#2a2926")
    tile_h = g.blend(hole_dark, tile_s, holes_m, mode=NORMAL_B, amount=0.8)
    grid_c = g.color("#55595c")
    albedo = g.blend(grid_c, tile_h, grid, mode=NORMAL_B, amount=1.0)
    metallic = g.math(grid, None, op=MUL, in2=0.9)
    rough = g.math(g.math(grid, None, op=MUL, in2=-0.5), None, op=ADD, in2=0.9)
    normal = g.normal_map(height, strength=0.8)
    ao = g.occlusion(height, strength=1.0, radius=8)
    g.material(albedo=albedo, metallic=metallic, roughness=rough, normal=normal, ao=ao)
    return g


def floor_tiles():
    """Worn grey industrial floor tiles with dirty grout."""
    g = Graph("Floor Tiles")
    tiles = g.node("bricks", pattern=0, repeat=1, rows=4, columns=4, row_offset=0.0,
                   mortar=0.03, bevel=0.03, round=0.01, corner=0.0)
    tiles_var = g.node("bricks", pattern=0, repeat=1, rows=4, columns=4, row_offset=0.0,
                       mortar=0.03, bevel=0.03, round=0.01, corner=0.0)
    var = g.node("greyscale", (tiles_var, 1), mode=2)  # per-tile random colour
    n = g.node("fbm2", noise=2, scale_x=8, scale_y=8, folds=0, iterations=6, persistence=0.6, offset=0.2)
    fine = g.node("fbm2", noise=0, scale_x=32, scale_y=32, folds=0, iterations=4, persistence=0.5, offset=0.8)
    scratch = g.node("scratches", length=0.3, width=0.3, layers=6, waviness=0.3, angle=30, randomness=0.8)
    height = g.math(tiles, g.math(fine, None, op=MUL, in2=0.04), op=ADD)
    base = g.colorize(var, grad((0, "#5d5f60"), (1, "#7c7e7e")))
    dirt = g.colorize(n, grad((0, "#2f2b25"), (1, "#9c968c")))
    col = g.blend(dirt, base, None, mode=MULTIPLY, amount=0.5)
    scr = g.color("#a8aaac")
    col2 = g.blend(scr, col, scratch, mode=NORMAL_B, amount=0.12)
    grout = g.tone(tiles, (0, 1), (0.08, 0), (1, 0))
    grout_c = g.color("#1d1b18")
    albedo = g.blend(grout_c, col2, grout, mode=NORMAL_B, amount=1.0)
    rough = g.math(g.colorize(n, grad((0, 0.35), (1, 0.7))), grout, op=MAX)
    normal = g.normal_map(height, strength=1.0)
    ao = g.occlusion(height, strength=1.2, radius=8)
    g.material(albedo=albedo, roughness=rough, normal=normal, ao=ao, metallic_v=0.0)
    return g


def crate():
    """Wooden supply crate: planks in a thick frame with a metal-capped border."""
    g = Graph("Crate")
    frame_sdf = g.node("sdbox", w=0.38, h=0.38, cx=0, cy=0)
    inner = g.node("sdshow", frame_sdf, bevel=0.02, base=0.0)
    frame = g.node("invert", inner)
    frame_g = g.node("greyscale", frame, mode=2)
    planks = g.node("bricks", pattern=0, repeat=1, rows=5, columns=1, row_offset=0.0,
                    mortar=0.015, bevel=0.03, round=0.0, corner=0.0)
    # diagonal brace
    brace_sdf = g.node("sdline", ax=-0.4, ay=-0.4, bx=0.4, by=0.4, r=0.06,
                       profile={"points": [{"ls": 0, "rs": 0, "x": 0, "y": 1}, {"ls": 0, "rs": 0, "x": 1, "y": 1}], "type": "Curve"})
    brace = g.node("sdshow", brace_sdf, bevel=0.015, base=0.0)
    grain = g.node("fbm2", noise=1, scale_x=2, scale_y=24, folds=0, iterations=6, persistence=0.7, offset=0.1)
    grain2 = g.node("fbm2", noise=1, scale_x=24, scale_y=2, folds=0, iterations=6, persistence=0.7, offset=0.6)
    wood_h = g.colorize(grain, grad((0, "#5a3a1e"), (0.5, "#8a5a2e"), (1, "#a8763e")))
    wood_v = g.colorize(grain2, grad((0, "#4e3219"), (0.5, "#7d5129"), (1, "#9c6c38")))
    # planks run horizontally inside, frame vertical grain
    planks_col = g.blend(g.tone(planks, (0, 0.3), (0.1, 1), (1, 1)), wood_h, None, mode=MULTIPLY, amount=1.0)
    brace_frame = g.math(frame_g, brace, op=MAX)
    albedo_w = g.blend(wood_v, planks_col, brace_frame, mode=NORMAL_B, amount=1.0)
    # metal corner caps
    cap_sdf = g.node("sdbox", w=0.47, h=0.47, cx=0, cy=0)
    cap_in = g.node("sdshow", cap_sdf, bevel=0.005, base=0.0)
    edge = g.node("greyscale", g.node("invert", cap_in), mode=2)
    grime = g.node("fbm2", noise=2, scale_x=6, scale_y=6, folds=0, iterations=6, persistence=0.6, offset=0.5)
    metal = g.colorize(grime, grad((0, "#3a3b3d"), (1, "#6a6c6f")))
    albedo_m = g.blend(metal, albedo_w, edge, mode=NORMAL_B, amount=1.0)
    dirt = g.colorize(grime, grad((0, "#3a3024"), (1, "#c8c0b0")))
    albedo = g.blend(dirt, albedo_m, None, mode=MULTIPLY, amount=0.35)
    h_pl = g.math(planks, None, op=MUL, in2=0.6)
    h_fr = g.math(brace_frame, None, op=MUL, in2=0.9)
    h = g.math(h_pl, h_fr, op=MAX)
    h2 = g.math(h, g.math(edge, None, op=MUL, in2=0.95), op=MAX)
    h3 = g.math(h2, g.math(grain, None, op=MUL, in2=0.05), op=ADD)
    metallic = edge
    rough = g.math(g.math(edge, None, op=MUL, in2=-0.35), None, op=ADD, in2=0.78)
    normal = g.normal_map(h3, strength=1.4)
    ao = g.occlusion(h3, strength=1.3, radius=10)
    g.material(albedo=albedo, metallic=metallic, roughness=rough, normal=normal, ao=ao)
    return g


def hazard_stripes():
    """Yellow/black warning stripes on steel, with chipped paint."""
    g = Graph("Hazard Stripes")
    # A 45 degree gradient repeats seamlessly (Material Maker normalises the diagonal).
    band = g.node("gradient", repeat=8, rotate=45, mirror=False,
                  gradient=grad((0, 0.0), (0.5, 1.0), interpolation=0))
    n = g.node("fbm2", noise=2, scale_x=10, scale_y=10, folds=0, iterations=6, persistence=0.6, offset=0.3)
    chips = g.tone(n, (0, 0), (0.62, 0), (0.7, 1), (1, 1))
    yellow = g.color("#d9a514")
    black = g.color("#18181a")
    band_g = g.node("greyscale", band, mode=2)
    paint = g.blend(yellow, black, band_g, mode=NORMAL_B, amount=1.0)
    metal = g.colorize(n, grad((0, "#5a5c5e"), (1, "#9a9c9e")))
    grime = g.colorize(n, grad((0, "#3a342a"), (1, "#ffffff")))
    col = g.blend(metal, paint, chips, mode=NORMAL_B, amount=1.0)
    albedo = g.blend(grime, col, None, mode=MULTIPLY, amount=0.4)
    height = g.math(g.math(chips, None, op=MUL, in2=-0.2), None, op=ADD, in2=0.5)
    metallic = chips
    rough = g.math(g.math(chips, None, op=MUL, in2=-0.25), None, op=ADD, in2=0.6)
    normal = g.normal_map(height, strength=0.6)
    g.material(albedo=albedo, metallic=metallic, roughness=rough, normal=normal)
    return g


def light_panel():
    """Recessed emissive light fixture panel (used for baked area lighting)."""
    g = Graph("Light Panel")
    frame_sdf = g.node("sdroundedbox", width=0.42, height=0.42, top_left=0.04, top_right=0.04,
                       bottom_left=0.04, bottom_right=0.04, center_x=0, center_y=0)
    diffuser = g.node("sdshow", frame_sdf, bevel=0.01, base=0.0)
    louvre = g.node("bricks", pattern=0, repeat=1, rows=4, columns=4, row_offset=0.0,
                    mortar=0.035, bevel=0.01, round=0.0, corner=0.0)
    grid_t = g.tone(louvre, (0, 1), (0.2, 0), (1, 0))
    grid_m = g.math(grid_t, diffuser, op=MUL)
    lit = g.math(diffuser, grid_m, op=SUB, clamp=True)
    n = g.node("fbm2", noise=2, scale_x=6, scale_y=6, folds=0, iterations=5, persistence=0.6, offset=0.8)
    housing = g.colorize(n, grad((0, "#3d3f42"), (1, "#5c5f63")))
    glow = g.color("#fff4dc")
    albedo = g.blend(glow, housing, lit, mode=NORMAL_B, amount=1.0)
    emission = g.blend(glow, g.color("#000000"), lit, mode=NORMAL_B, amount=1.0)
    height = g.math(g.math(diffuser, None, op=MUL, in2=-0.4), None, op=ADD, in2=0.6)
    height2 = g.math(height, g.math(grid_m, None, op=MUL, in2=0.3), op=ADD)
    metallic = g.math(g.math(diffuser, None, op=MUL, in2=-1.0), None, op=ADD, in2=1.0)
    rough = g.math(g.math(diffuser, None, op=MUL, in2=-0.2), None, op=ADD, in2=0.5)
    normal = g.normal_map(height2, strength=1.0)
    g.material(albedo=albedo, metallic=metallic, roughness=rough, emission=emission, normal=normal)
    return g


def rust_metal():
    """Heavily rusted steel plate with pitting (normal-mapped)."""
    g = Graph("Rust Metal")
    plates = g.node("bricks", pattern=0, repeat=1, rows=2, columns=1, row_offset=0.0,
                    mortar=0.01, bevel=0.02, round=0.0, corner=0.0)
    n1 = g.node("fbm2", noise=2, scale_x=5, scale_y=5, folds=0, iterations=8, persistence=0.75, offset=0.1)
    n2 = g.node("fbm2", noise=1, scale_x=10, scale_y=10, folds=0, iterations=7, persistence=0.7, offset=0.4)
    pit = g.node("fbm2", noise=3, scale_x=20, scale_y=20, folds=0, iterations=3, persistence=0.5, offset=0.2)
    rust_mask = g.tone(n1, (0, 0), (0.42, 0), (0.58, 1), (1, 1))
    rust_col = g.colorize(n2, grad((0, "#3b1c0c"), (0.4, "#7a3714"), (0.75, "#a4521e"), (1, "#c27a3a")))
    steel = g.colorize(n2, grad((0, "#4b4d50"), (1, "#8d9094")))
    col = g.blend(rust_col, steel, rust_mask, mode=NORMAL_B, amount=1.0)
    seam = g.tone(plates, (0, 0.2), (0.1, 1), (1, 1))
    albedo = g.blend(seam, col, None, mode=MULTIPLY, amount=1.0)
    pits = g.tone(pit, (0, 0), (0.7, 0), (0.85, 1), (1, 1))
    h1 = g.math(g.math(rust_mask, None, op=MUL, in2=0.12), g.math(n2, None, op=MUL, in2=0.1), op=ADD)
    h2 = g.math(h1, g.math(g.math(pits, rust_mask, op=MUL), None, op=MUL, in2=0.2), op=SUB)
    height = g.math(h2, plates, op=MUL)
    metallic = g.tone(rust_mask, (0, 0.95), (1, 0.05))
    rough = g.math(g.colorize(rust_mask, grad((0, 0.35), (1, 0.88))), g.math(n2, None, op=MUL, in2=0.1), op=ADD)
    normal = g.normal_map(height, strength=1.2)
    ao = g.occlusion(height, strength=1.0, radius=8)
    g.material(albedo=albedo, metallic=metallic, roughness=rough, normal=normal, ao=ao)
    return g


def plain_steel():
    """Plain painted steel with a light horizontal brushed grain and fine scratches: no panels,
    no seams, no grid. Star Crew's own graph (2026-10-14), for the interior walls and trims after
    the owner had the tech_panel grid taken off them."""
    g = Graph("Plain Steel")
    # soft mottling of the paint, and a long horizontal grain (scale_y far above scale_x)
    mottle = g.node("fbm2", noise=2, scale_x=6, scale_y=6, folds=0, iterations=6, persistence=0.5, offset=0.3)
    grain = g.node("fbm2", noise=1, scale_x=2, scale_y=48, folds=0, iterations=4, persistence=0.5, offset=0.6)
    fine = g.node("fbm2", noise=0, scale_x=32, scale_y=32, folds=0, iterations=3, persistence=0.5, offset=0.1)
    # light scratches: long near-horizontal ones and a few short crossing ones, gathered where a
    # coarse wear noise is high so the wall is not evenly covered
    long_scr = g.node("scratches", length=0.2, width=0.15, layers=2, waviness=0.1, angle=5, randomness=0.3)
    short_scr = g.node("scratches", length=0.08, width=0.12, layers=1, waviness=0.2, angle=-50, randomness=0.6)
    wear_n = g.node("fbm2", noise=2, scale_x=3, scale_y=3, folds=0, iterations=5, persistence=0.55, offset=0.7)
    wear = g.tone(wear_n, (0, 0.15), (0.5, 0.15), (0.7, 1), (1, 1))
    scr_a = g.math(g.math(long_scr, None, op=MUL, in2=0.5), g.math(short_scr, None, op=MUL, in2=0.3), op=MAX)
    scr = g.math(scr_a, wear, op=MUL, clamp=True)

    paint = g.colorize(mottle, grad((0, "#636f7a"), (1, "#69757f")))
    brushed = g.colorize(grain, grad((0, "#4f5860"), (1, "#8c959c")))
    paint_g = g.blend(brushed, paint, None, mode=OVERLAY, amount=0.35)
    bare = g.colorize(fine, grad((0, "#9aa0a5"), (1, "#b7bcc0")))
    albedo_a = g.blend(bare, paint_g, scr, mode=NORMAL_B, amount=0.7)
    grime = g.colorize(mottle, grad((0, "#565b60"), (1, "#ffffff")))
    albedo = g.blend(grime, albedo_a, None, mode=MULTIPLY, amount=0.12)

    # nearly flat: the grain and the fine noise lift it a little, a scratch cuts into it
    h1 = g.math(g.math(grain, None, op=MUL, in2=0.04), g.math(fine, None, op=MUL, in2=0.02), op=ADD)
    height = g.math(g.math(h1, None, op=ADD, in2=0.5), g.math(scr, None, op=MUL, in2=0.08), op=SUB, clamp=True)
    metallic = g.tone(scr, (0, 0.2), (1, 1))
    rough = g.math(g.colorize(mottle, grad((0, 0.45), (1, 0.6))), g.math(scr, None, op=MUL, in2=0.2), op=SUB, clamp=True)
    normal = g.normal_map(height, strength=0.6)
    ao = g.occlusion(height, strength=0.6, radius=8)
    g.material(albedo=albedo, metallic=metallic, roughness=rough, normal=normal, ao=ao)
    return g


def exterior_paint():
    """Original generated brushed paint, with no panel grid. Image remains an editable graph input."""
    g = Graph("Exterior Pearl Paint")
    source = g.node("image", image="../sources/exterior-paint.png")
    g.material(albedo=source, metallic_v=0.0, roughness_v=0.85)
    return g


def exterior_surfaces():
    """Generated fitted ship artwork; four UV regions, not a repeating array layer."""
    g = Graph("Exterior Fitted Surface Atlas")
    source = g.node("image", image="../sources/exterior-surfaces.png")
    g.material(albedo=source, metallic_v=0.0, roughness_v=0.85)
    return g


def fitted_tern_nacelle():
    """Dedicated neutral artwork painted over projected nacelle UV linework."""
    g = Graph("Tern Nacelle and Level Support Paint")
    source = g.node("image", image="../sources/tern-nacelle-finish.png")
    g.material(albedo=source, metallic_v=0.0, roughness_v=0.85)
    return g


def fitted_tern_hull(livery="copper"):
    """Projected livery over generated recessed joints and worn armor lips."""
    g = Graph("Tern Integrated Armor Joints - " + livery.title())
    with open(os.path.join(HERE, "../../data/ships/tern/hull_paint_projection.json")) as f:
        projection = json.load(f)
    cap = projection["shoulder_termination"]
    shoulder = g.node("decompose", g.node("image", image="../sources/tern-shoulder-armor-mask.png"))
    mechanical = g.node("image", image="../sources/tern-seamless-finish.png")
    source = g.node("image", image="../sources/tern-integrated-joints.png")
    # Sample approved clear armor, avoiding registration and preexisting panel lines.
    # Its new boundaries and joints come from the actual 3D projection.
    # R is coverage, G the forward cap, B the port cap. Native transform maps use
    # 2*input-1, allowing opposite X offsets and a forward-only Y offset at 1:1 scale.
    tx = g.math(g.grey(1), (shoulder, 2), op=SUB)
    ty = g.math(g.grey(.5), g.math((shoulder, 1), g.grey(.5), op=MUL), op=ADD)
    plate = g.node("transform", mechanical, tx, ty, scale_x=1, scale_y=1,
                   translate_x=cap["armor_sample_translate_x"],
                   translate_y=cap["armor_sample_translate_y"], rotate=0, repeat=False)
    source = g.blend(plate, source, (shoulder, 0), mode=NORMAL_B)
    enhanced = g.node("decompose", g.node("image", image="../sources/tern-flat-painted-cyan.png"))
    mask = g.node("decompose", g.node("image", image="../sources/tern-projected-masks.png"))
    ribbon = g.node("decompose", g.node("image", image="../sources/tern-projected-ribbon.png"))
    # The enhancement retains guide strokes over some pipes. Restore only those
    # mechanical texels from the seam-free painting, using the original 3D mask.
    mechanical = g.blend(plate, mechanical, (shoulder, 0), mode=NORMAL_B)
    original = g.node("decompose", mechanical)
    original_value = g.math(g.math((original, 0), (original, 1), op=MIN), (original, 2), op=MIN)
    original_armor = g.math(g.math(original_value, g.grey(.55), op=SUB), g.grey(.2), op=DIV, clamp=True)
    repair = g.math((mask, 1), g.math(g.grey(1), original_armor, op=SUB), op=MUL)
    source = g.blend(mechanical, source, repair, mode=NORMAL_B)
    with open(os.path.join(HERE, "../../data/ships/exteriors.json")) as f:
        liveries = json.load(f)["liveries"]
    color = next(row["paint_srgb"] for row in liveries
                 if row["id"] == ("cobalt" if livery == "cyan" else livery))
    rgba = g.node("decompose", source)
    # Dark mechanical recesses retain their neutral metal; bright armor receives paint.
    value = g.math(g.math((rgba, 0), (rgba, 1), op=MIN), (rgba, 2), op=MIN)
    armor = g.math(g.math(value, g.grey(.55), op=SUB), g.grey(.2), op=DIV, clamp=True)
    fitting_blue = g.math(g.math((rgba, 0), g.grey(.35), op=LT),
                         g.math(g.math(g.grey(.5), (rgba, 1), op=LT),
                                g.math(g.grey(.6), (rgba, 2), op=LT), op=MUL), op=MUL)
    paint = g.math(g.math((mask, 0), armor, op=MUL), fitting_blue, op=MAX)
    # Sample the generated pigment finish, preserving variation instead of flat RGB.
    shade = g.math((enhanced, 2), g.grey(projection["pigment_reference_srgb"][2]), op=DIV)
    shade = g.math(g.grey(1), g.math(g.math(shade, g.grey(1), op=SUB), g.grey(2), op=MUL), op=ADD)
    shade = g.math(g.math(shade, g.grey(.60), op=MAX), g.grey(1.12), op=MIN)
    coat = g.blend(g.color(color), shade, mode=MULTIPLY)
    albedo = g.blend(coat, source, paint, mode=NORMAL_B)
    # The same ship-local projection places the fine white line on roofs and walls.
    # Retain a little enamel variation, while joints and mechanics remain recessed.
    border_shade = g.math(g.math(shade, g.grey(.96), op=MAX), g.grey(1.03), op=MIN)
    border_coat = g.blend(g.color(projection["ribbon_border_srgb"]), border_shade, mode=MULTIPLY)
    albedo = g.blend(border_coat, albedo, g.math((ribbon, 0), armor, op=MUL), mode=NORMAL_B)
    # Keep the narrow recess centered on the 3D projection, sampling its tone from
    # the generated finish. Its varied core sits inside the painted bevel and wear;
    # exposed machinery is excluded. No constant-color ruler stroke is applied.
    joint = g.colorize((rgba, 1), grad((0, "#434740"), (1, "#77776f")))
    albedo = g.blend(joint, albedo, g.math((mask, 1), original_armor, op=MUL), mode=NORMAL_B)
    g.material(albedo=albedo, metallic_v=0.0, roughness_v=0.85)
    return g


# ----------------------------------------------------------------------------
# The O'Neill pair at Tamsin Station (openspec/changes/spacedock, version E): two texture layers
# for a surface kilometres long, as the owner asked ("we should have 2 layers of texture, detail
# layer, and the big albiedo color map, and maybe normal maps on the details"). The macro maps are
# fitted atlases wrapped once round the cylinder or once over its end cap; the detail layer and the
# mirror tile many times and keep their normal maps. Star Crew's own graphs, 2026-10-14.
# ----------------------------------------------------------------------------

def _steps(*edges, lo=0.0, hi=1.0):
    """A constant-interpolation gradient that is `hi` between each pair of edges, `lo` elsewhere.
    Material Maker stores a constant step at t as two points either side of it, t - e with the
    old value and t + e with the new (types/gradient.gd serialize), and reads them back so."""
    e = 0.00001
    def at(x):
        return any(a <= x < b for a, b in edges)
    cuts = sorted({c for ab in edges for c in ab if 0.0 < c < 1.0})
    stops, cur = [(0.0, hi if at(0.0) else lo)], at(0.0)
    for c in cuts:
        nxt = at(c)
        if nxt != cur:
            stops += [(c - e, hi if cur else lo), (c + e, hi if nxt else lo)]
            cur = nxt
    return grad(*stops, interpolation=0)


def oneill_macro():
    """The cylinder's outer macro albedo, wrapped once round it: x runs round the circumference, y
    along the length. Three hull strips (the outside of the shell under the land) alternate with
    three window strips of glazing: many small panes in a frame that carries the pressure (Wikipedia,
    "O'Neill cylinder"). The land is inside, not painted here: the panes are dark glass, and the
    emission output marks them (the packer's alpha), so the mockup draws them see-through. The hull is
    pearl plating with large panel variation, km-scale grime, orange identity bands round the cylinder
    and graphite frames along the windows."""
    g = Graph("ONeill Macro")
    # 1 in the window strips (the second half of each third of the circumference), 0 on the hull
    win = g.node("greyscale", g.node("gradient", repeat=3, rotate=0, mirror=False, gradient=_steps((0.5, 1.0))), mode=2)
    # a frame along both edges of every strip
    edge = g.node("greyscale", g.node("gradient", repeat=6, rotate=0, mirror=False,
                                      gradient=_steps((0.0, 0.035), (0.965, 1.0))), mode=2)
    # hull: big plates with their own shade, km-scale grime, streaks along the length
    plates = g.node("bricks", pattern=0, repeat=1, rows=40, columns=24, row_offset=0.5,
                    mortar=0.04, bevel=0.0, round=0.0, corner=0.0)
    shade = g.node("greyscale", (plates, 1), mode=2)
    pearl = g.colorize(shade, grad((0, "#c3c7cb"), (0.5, "#d5d8da"), (1, "#e6e7e6")))
    grime_n = g.node("fbm2", noise=2, scale_x=4, scale_y=4, folds=0, iterations=6, persistence=0.6, offset=0.2)
    streak_n = g.node("fbm2", noise=1, scale_x=24, scale_y=2, folds=0, iterations=5, persistence=0.55, offset=0.6)
    grime = g.colorize(grime_n, grad((0, "#8e8a82"), (0.55, "#e9e6df"), (1, "#ffffff")))
    streak = g.colorize(streak_n, grad((0, "#a8a49c"), (1, "#ffffff")))
    hull_a = g.blend(grime, pearl, None, mode=MULTIPLY, amount=0.55)
    hull_b = g.blend(streak, hull_a, None, mode=MULTIPLY, amount=0.35)
    # faint seams: at this scale a plate is 150 m across, and its joint is a line, not a groove
    seam = g.tone(plates, (0, 0.88), (0.08, 1), (1, 1))
    hull_c = g.blend(seam, hull_b, None, mode=MULTIPLY, amount=1.0)
    # identity bands round the cylinder: orange, near both ends and at the middle
    bands = g.node("greyscale", g.node("gradient", repeat=1, rotate=90, mirror=False,
                                       gradient=_steps((0.035, 0.065), (0.48, 0.52), (0.935, 0.965))), mode=2)
    orange = g.colorize(grime_n, grad((0, "#b8541e"), (1, "#dc6f2c")))
    hull_d = g.blend(orange, hull_c, bands, mode=NORMAL_B, amount=1.0)
    frame = g.colorize(grime_n, grad((0, "#3d464f"), (1, "#56606a")))
    hull = g.blend(frame, hull_d, edge, mode=NORMAL_B, amount=1.0)
    # the glazing: many small panes, mullions down the strip and frames across it every 33 m
    mull = g.node("greyscale", g.node("gradient", repeat=72, rotate=0, mirror=False, gradient=_steps((0.0, 0.04))), mode=2)
    xfr = g.node("greyscale", g.node("gradient", repeat=192, rotate=90, mirror=False, gradient=_steps((0.0, 0.05))), mode=2)
    bars = g.math(mull, xfr, op=MAX)
    pane_n = g.node("fbm2", noise=0, scale_x=72, scale_y=192, folds=0, iterations=1, persistence=0.5, offset=0.5)
    pane = g.colorize(pane_n, grad((0, "#1c2a38"), (1, "#2d4256")))
    glass = g.blend(frame, pane, bars, mode=NORMAL_B, amount=1.0)
    win_in = g.math(win, edge, op=SUB, clamp=True)
    albedo = g.blend(glass, hull, win_in, mode=NORMAL_B, amount=1.0)
    panes = g.math(win_in, bars, op=SUB, clamp=True)
    emission = g.blend(g.color("#ffffff"), g.color("#000000"), panes, mode=NORMAL_B, amount=1.0)
    height = g.math(g.math(plates, None, op=MUL, in2=0.3), g.math(win_in, None, op=MUL, in2=-0.2), op=ADD)
    normal = g.normal_map(height, strength=0.5)
    g.material(albedo=albedo, emission=emission, normal=normal, metallic_v=0.0, roughness_v=0.8)
    return g


def oneill_land():
    """The land inside, one stripe (x across its 940 m, y along its 6.4 km; the packer makes it
    512 x 2048 px, so the noises are 6.8 times finer along y than across x to come out round).
    Fields of many crops, woods, a river meandering the length with lakes, a road down the stripe
    and lanes across it, small towns of white roofs. Lit by the mirrors' light through the windows."""
    g = Graph("ONeill Land")
    base_n = g.node("fbm2", noise=2, scale_x=2, scale_y=14, folds=0, iterations=6, persistence=0.6, offset=0.35)
    land = g.colorize(base_n, grad((0, "#2c4a22"), (0.45, "#46672b"), (0.62, "#6e7b3b"), (0.8, "#8d8458"), (1, "#a39a77")))
    field_n = g.node("bricks", pattern=0, repeat=1, rows=80, columns=8, row_offset=0.37,
                     mortar=0.03, bevel=0.0, round=0.0, corner=0.0)
    crop = g.node("greyscale", (field_n, 1), mode=2)
    fields = g.colorize(crop, grad((0, "#3f6324"), (0.3, "#5d7a2e"), (0.55, "#9a9548"), (0.75, "#b9a35e"), (1, "#7a6a45")))
    hedge = g.tone(field_n, (0, 1), (0.1, 0), (1, 0))
    fields_h = g.blend(g.color("#2a3f1c"), fields, hedge, mode=NORMAL_B, amount=0.8)
    farm = g.tone(base_n, (0, 0), (0.4, 0), (0.5, 1), (1, 1))
    land_b = g.blend(fields_h, land, farm, mode=NORMAL_B, amount=1.0)
    wood_n = g.node("fbm2", noise=2, scale_x=4, scale_y=27, folds=0, iterations=6, persistence=0.65, offset=0.7)
    wood = g.tone(wood_n, (0, 0), (0.6, 0), (0.68, 1), (1, 1))
    land_w = g.blend(g.colorize(wood_n, grad((0, "#1b3518"), (1, "#2c4d22"))), land_b, wood, mode=NORMAL_B, amount=1.0)
    river_n = g.node("fbm2", noise=1, scale_x=1, scale_y=7, folds=0, iterations=5, persistence=0.5, offset=0.15)
    river = g.tone(river_n, (0, 0), (0.485, 0), (0.5, 1), (0.515, 0), (1, 0))
    lake = g.tone(base_n, (0, 1), (0.2, 1), (0.24, 0), (1, 0))
    water = g.math(river, lake, op=MAX)
    land_r = g.blend(g.color("#2a5874"), land_w, water, mode=NORMAL_B, amount=1.0)
    # towns: clusters of small blocks where a coarse noise is high, white and grey roofs and streets
    town_n = g.node("fbm2", noise=0, scale_x=3, scale_y=20, folds=0, iterations=3, persistence=0.5, offset=0.9)
    town = g.tone(town_n, (0, 0), (0.8, 0), (0.83, 1), (1, 1))
    blocks = g.node("bricks", pattern=0, repeat=1, rows=640, columns=48, row_offset=0.5,
                    mortar=0.25, bevel=0.0, round=0.0, corner=0.0)
    roof = g.colorize(g.node("greyscale", (blocks, 1), mode=2), grad((0, "#9a9a96"), (0.6, "#d6d4cc"), (1, "#b05a3c")))
    street = g.tone(blocks, (0, 1), (0.05, 0), (1, 0))
    town_c = g.blend(g.color("#6d6c68"), roof, street, mode=NORMAL_B, amount=1.0)
    land_t = g.blend(town_c, land_r, town, mode=NORMAL_B, amount=1.0)
    # a road down the stripe and lanes across it every 530 m
    road = g.node("greyscale", g.node("gradient", repeat=1, rotate=0, mirror=False, gradient=_steps((0.31, 0.318))), mode=2)
    lanes = g.node("greyscale", g.node("gradient", repeat=12, rotate=90, mirror=False, gradient=_steps((0.0, 0.004))), mode=2)
    roads = g.math(road, lanes, op=MAX)
    albedo = g.blend(g.color("#8a8780"), land_t, roads, mode=NORMAL_B, amount=0.9)
    height = g.math(g.math(wood, None, op=MUL, in2=0.3), g.math(water, None, op=MUL, in2=-0.2), op=ADD)
    normal = g.normal_map(height, strength=0.3)
    g.material(albedo=albedo, normal=normal, metallic_v=0.0, roughness_v=0.9)
    return g


def oneill_land_detail():
    """The land's tiled detail layer, 64 m a repeat: crop rows, tree crowns and tracks, near white
    (it multiplies the land macro) so close views of the valley floor are not smooth paint."""
    g = Graph("ONeill Land Detail")
    rows = g.node("greyscale", g.node("gradient", repeat=24, rotate=0, mirror=False, gradient=grad((0, 0.0), (0.5, 1.0), (1, 0.0))), mode=2)
    rows_t = g.tone(rows, (0, 0.8), (1, 1))
    crowns_c = g.node("sdcircle", r=0.018, cx=0, cy=0)
    crowns = g.node("sdshow", g.node("sdrepeat", crowns_c, rx=16, ry=16, r=0.35, variations=True), bevel=0.012, base=0.0)
    patch_n = g.node("fbm2", noise=2, scale_x=3, scale_y=3, folds=0, iterations=5, persistence=0.6, offset=0.3)
    patch = g.tone(patch_n, (0, 0), (0.55, 0), (0.65, 1), (1, 1))
    trees = g.math(crowns, patch, op=MUL)
    fine = g.node("fbm2", noise=0, scale_x=32, scale_y=32, folds=0, iterations=4, persistence=0.5, offset=0.1)
    base = g.colorize(fine, grad((0, "#d6d6d0"), (1, "#f6f6f0")))
    a1 = g.blend(rows_t, base, None, mode=MULTIPLY, amount=0.6)
    albedo = g.blend(g.color("#6a7a5e"), a1, trees, mode=MULTIPLY, amount=0.9)
    height = g.math(trees, g.math(rows, None, op=MUL, in2=0.15), op=ADD)
    normal = g.normal_map(height, strength=0.8)
    g.material(albedo=albedo, normal=normal, metallic_v=0.0, roughness_v=0.9)
    return g


def oneill_clouds():
    """The cloud layer inside, wrapped once round its cylinder a few hundred metres off the floor
    (x round it, y along it; noise 1.8 times finer along y to come out round). White and grey
    tops; the packer takes the alpha from the cloud cover, patchy, with clear sky between."""
    g = Graph("ONeill Clouds")
    n1 = g.node("fbm2", noise=2, scale_x=6, scale_y=11, folds=0, iterations=7, persistence=0.62, offset=0.4)
    n2 = g.node("fbm2", noise=1, scale_x=18, scale_y=33, folds=0, iterations=5, persistence=0.55, offset=0.8)
    mix_n = g.blend(n2, n1, None, mode=OVERLAY, amount=0.5)
    cover = g.tone(mix_n, (0, 0), (0.54, 0), (0.7, 1), (1, 1))
    tops = g.colorize(n2, grad((0, "#b8c0c8"), (1, "#ffffff")))
    albedo = g.blend(tops, g.color("#000000"), cover, mode=NORMAL_B, amount=1.0)
    emission = g.blend(g.color("#ffffff"), g.color("#000000"), cover, mode=NORMAL_B, amount=1.0)
    g.material(albedo=albedo, emission=emission, metallic_v=0.0, roughness_v=1.0)
    return g


def oneill_cap():
    """An end cap's macro albedo, once over the disc: pearl plating in rings and sectors round a
    graphite hub, an orange rim band, dark seams, km-scale grime."""
    g = Graph("ONeill Cap")
    rings = g.node("greyscale", g.node("radial_gradient", repeat=7, mirror=False, gradient=grad((0, 0.0), (1, 1.0))), mode=2)
    sectors = g.node("greyscale", g.node("circular_gradient", repeat=24, mirror=False, gradient=grad((0, 0.0), (1, 1.0))), mode=2)
    ring_seam = g.tone(rings, (0, 1), (0.03, 0), (0.97, 0), (1, 1))
    sect_seam = g.tone(sectors, (0, 1), (0.04, 0), (0.96, 0), (1, 1))
    seams = g.math(ring_seam, sect_seam, op=MAX)
    panel_n = g.node("fbm2", noise=0, scale_x=24, scale_y=24, folds=0, iterations=2, persistence=0.5, offset=0.4)
    pearl = g.colorize(panel_n, grad((0, "#c6c9cc"), (1, "#e4e5e4")))
    grime_n = g.node("fbm2", noise=2, scale_x=5, scale_y=5, folds=0, iterations=6, persistence=0.6, offset=0.9)
    grime = g.colorize(grime_n, grad((0, "#8f8b84"), (0.55, "#ebe8e2"), (1, "#ffffff")))
    plate = g.blend(grime, pearl, None, mode=MULTIPLY, amount=0.55)
    rim = g.math(g.node("sdshow", g.node("sdcircle", r=0.5, cx=0, cy=0), bevel=0.0, base=0.0),
                 g.node("sdshow", g.node("sdcircle", r=0.43, cx=0, cy=0), bevel=0.0, base=0.0), op=SUB, clamp=True)
    hub = g.node("sdshow", g.node("sdcircle", r=0.075, cx=0, cy=0), bevel=0.004, base=0.0)
    orange = g.colorize(grime_n, grad((0, "#b8541e"), (1, "#dc6f2c")))
    graphite = g.colorize(grime_n, grad((0, "#353d45"), (1, "#525c66")))
    a1 = g.blend(orange, plate, rim, mode=NORMAL_B, amount=1.0)
    a2 = g.blend(graphite, a1, hub, mode=NORMAL_B, amount=1.0)
    albedo = g.blend(g.color("#59616a"), a2, seams, mode=NORMAL_B, amount=0.85)
    height = g.math(g.math(seams, None, op=MUL, in2=-0.3), None, op=ADD, in2=0.6)
    normal = g.normal_map(height, strength=0.5)
    g.material(albedo=albedo, normal=normal, metallic_v=0.0, roughness_v=0.8)
    return g


def oneill_detail():
    """The tiled detail layer under the macro maps: one repeat is 16 m of hull. Uneven plates
    with chamfered seams and bolt rows, about one plate in five a recessed service plate with
    louvres, fine scratches; a near-white albedo (it multiplies the macro colour) and a real normal
    map for per-pixel light. Nothing sits on a regular grid, so the repeat does not show as one."""
    g = Graph("ONeill Detail")
    # plates of uneven sizes, split again and again, so no square grid shows (CLAUDE.md 9)
    plates = g.node("bricks_uneven", iterations=6, min_size=0.12, randomness=0.65,
                    mortar=0.0035, bevel=0.012, round=0.0, corner=0.0)
    shade = g.node("greyscale", (plates, 1), mode=2)
    bolt = g.node("sdcircle", r=0.004, cx=0, cy=0)
    bolts = g.node("sdshow", g.node("sdrepeat", bolt, rx=48, ry=48, r=0.0, variations=False), bevel=0.003, base=0.0)
    # bolts only in a band along each plate's edge
    bolt_rows = g.tone(plates, (0, 0), (0.05, 0), (0.12, 1), (0.3, 1), (0.45, 0), (1, 0))
    bolts_m = g.math(bolts, bolt_rows, op=MUL)
    # service plates: the plates whose random shade is highest, recessed, louvred inside their rim
    service = g.tone(shade, (0, 0), (0.76, 0), (0.78, 1), (1, 1))
    inner = g.tone(plates, (0, 0), (0.2, 0), (0.35, 1), (1, 1))
    serv_in = g.math(service, inner, op=MUL)
    slats = g.node("greyscale", g.node("gradient", repeat=96, rotate=90, mirror=False, gradient=grad((0, 0.0), (1, 1.0))), mode=2)
    vents = g.math(serv_in, g.tone(slats, (0, 0.2), (0.5, 1), (1, 0.2)), op=MUL)
    fine = g.node("fbm2", noise=0, scale_x=48, scale_y=48, folds=0, iterations=4, persistence=0.5, offset=0.2)
    scr = g.node("scratches", length=0.12, width=0.12, layers=2, waviness=0.2, angle=10, randomness=0.6)
    height_a = g.math(plates, None, op=MUL, in2=0.75)
    height_b = g.math(height_a, g.math(bolts_m, None, op=MUL, in2=0.2), op=ADD)
    height_c = g.math(height_b, g.math(serv_in, None, op=MUL, in2=-0.12), op=ADD)
    height_d = g.math(height_c, g.math(vents, None, op=MUL, in2=-0.12), op=ADD)
    height = g.math(height_d, g.math(fine, None, op=MUL, in2=0.03), op=ADD)
    base = g.colorize(shade, grad((0, "#cdcdca"), (0.5, "#e6e6e3"), (1, "#f7f7f4")))
    grime_n = g.node("fbm2", noise=2, scale_x=6, scale_y=6, folds=0, iterations=6, persistence=0.6, offset=0.5)
    grime = g.colorize(grime_n, grad((0, "#b9b5ad"), (1, "#ffffff")))
    a1 = g.blend(grime, base, None, mode=MULTIPLY, amount=0.6)
    a2 = g.blend(g.color("#8d9296"), a1, scr, mode=NORMAL_B, amount=0.25)
    seam = g.tone(plates, (0, 0.6), (0.06, 1), (1, 1))
    a3 = g.blend(seam, a2, None, mode=MULTIPLY, amount=1.0)
    louvre = g.colorize(slats, grad((0, "#3e454c"), (0.5, "#7a828a"), (1, "#3e454c")))
    albedo = g.blend(louvre, a3, serv_in, mode=NORMAL_B, amount=0.8)
    normal = g.normal_map(height, strength=1.8)
    ao = g.occlusion(height, strength=1.0, radius=10)
    g.material(albedo=albedo, normal=normal, ao=ao, metallic_v=0.3, roughness_v=0.6)
    return g


def oneill_mirror():
    """A mirror wing's facets: silvered panels a little apart in tone, in thin graphite frames,
    with a soft bloom of wear; a normal map so the facets catch the sun one by one."""
    g = Graph("ONeill Mirror")
    facets = g.node("bricks", pattern=0, repeat=1, rows=6, columns=3, row_offset=0.0,
                    mortar=0.025, bevel=0.012, round=0.0, corner=0.0)
    tone_v = g.node("greyscale", (facets, 1), mode=2)
    silver = g.colorize(tone_v, grad((0, "#8e9aa6"), (0.5, "#a9b6c2"), (1, "#c5d0d8")))
    haze_n = g.node("fbm2", noise=2, scale_x=4, scale_y=4, folds=0, iterations=5, persistence=0.55, offset=0.1)
    haze = g.colorize(haze_n, grad((0, "#9ea4aa"), (1, "#ffffff")))
    a1 = g.blend(haze, silver, None, mode=MULTIPLY, amount=0.3)
    frame = g.tone(facets, (0, 1), (0.06, 0), (1, 0))
    albedo = g.blend(g.color("#30373e"), a1, frame, mode=NORMAL_B, amount=1.0)
    height = g.math(facets, g.math(tone_v, None, op=MUL, in2=0.05), op=ADD)
    normal = g.normal_map(height, strength=0.8)
    g.material(albedo=albedo, normal=normal, metallic_v=1.0, roughness_v=0.15)
    return g


GRAPHS = {
    "tern_nacelle": fitted_tern_nacelle,
    "tern_hull": fitted_tern_hull,
    "tern_hull_cobalt": lambda: fitted_tern_hull("cyan"),
    "tern_hull_rescue": lambda: fitted_tern_hull("rescue"),
    "exterior_surfaces": exterior_surfaces,
    "exterior_paint": exterior_paint,
    "tech_panel": tech_panel,
    "ceiling_tiles": ceiling_tiles,
    "floor_tiles": floor_tiles,
    "crate": crate,
    "hazard_stripes": hazard_stripes,
    "light_panel": light_panel,
    "rust_metal": rust_metal,
    "plain_steel": plain_steel,
    "oneill_macro": oneill_macro,
    "oneill_cap": oneill_cap,
    "oneill_detail": oneill_detail,
    "oneill_mirror": oneill_mirror,
    "oneill_land": oneill_land,
    "oneill_land_detail": oneill_land_detail,
    "oneill_clouds": oneill_clouds,
}




# ----------------------------------------------------------------------------
# Material Maker example graphs (MIT, © Rodolphe Suescun and contributors) that we
# ship with small PBR corrections: a few examples leave metallic/roughness unwired,
# which Material Maker exports as a constant (e.g. fully metallic bricks).
# ----------------------------------------------------------------------------

def _patch(name, fn):
    path = os.path.join(OUT_DIR, name + ".ptex")
    with open(path) as f:
        data = json.load(f)
    if any(n["name"].startswith("bf_") for n in data["nodes"]):
        return  # already patched
    fn(data)
    with open(path, "w") as f:
        json.dump(data, f, indent=1, sort_keys=True)
    print("patched", os.path.relpath(path))


def _add(data, name, type_, params, x=0, y=600):
    data["nodes"].append({"name": name, "type": type_, "parameters": params,
                          "node_position": {"x": x, "y": y}})


def _connect(data, src, src_port, dst, dst_port):
    data["connections"] = [c for c in data["connections"]
                           if not (c["to"] == dst and c["to_port"] == dst_port)]
    data["connections"].append({"from": src, "from_port": src_port, "to": dst, "to_port": dst_port})


def patch_diamond_plate(d):
    # 'blend' is the rust mask: rusted areas are dielectric, bare steel is metal.
    _add(d, "bf_metallic", "colorize", {"gradient": grad((0, 1.0), (0.6, 0.0))})
    _connect(d, "blend", 0, "bf_metallic", 0)
    _connect(d, "bf_metallic", 0, "Material", METALLIC)


PATCHES = {
    "diamond_plate": patch_diamond_plate,
}


def main():
    os.makedirs(OUT_DIR, exist_ok=True)
    for name, fn in GRAPHS.items():
        g = fn()
        path = os.path.join(OUT_DIR, name + ".ptex")
        with open(path, "w") as f:
            json.dump(g.to_json(), f, indent=1, sort_keys=True)
        print("wrote", os.path.relpath(path))
    for name, fn in PATCHES.items():
        if os.path.exists(os.path.join(OUT_DIR, name + ".ptex")):
            _patch(name, fn)


if __name__ == "__main__":
    main()
