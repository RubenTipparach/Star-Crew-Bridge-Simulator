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


def fitted_tern_hull(livery="copper"):
    """Projected livery over generated recessed joints and worn armor lips."""
    g = Graph("Tern Integrated Armor Joints - " + livery.title())
    source = g.node("image", image="../sources/tern-integrated-joints.png")
    enhanced = g.node("decompose", g.node("image", image="../sources/tern-flat-painted-cyan.png"))
    mask = g.node("decompose", g.node("image", image="../sources/tern-projected-masks.png"))
    # The enhancement retains guide strokes over some pipes. Restore only those
    # mechanical texels from the seam-free painting, using the original 3D mask.
    mechanical = g.node("image", image="../sources/tern-seamless-finish.png")
    original = g.node("decompose", mechanical)
    original_value = g.math(g.math((original, 0), (original, 1), op=MIN), (original, 2), op=MIN)
    original_armor = g.math(g.math(original_value, g.grey(.55), op=SUB), g.grey(.2), op=DIV, clamp=True)
    repair = g.math((mask, 1), g.math(g.grey(1), original_armor, op=SUB), op=MUL)
    source = g.blend(mechanical, source, repair, mode=NORMAL_B)
    with open(os.path.join(HERE, "../../data/ships/tern/hull_paint_projection.json")) as f:
        projection = json.load(f)
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
    shade = g.math((enhanced, 2), g.grey(projection["stripe_srgb"][2]), op=DIV)
    shade = g.math(g.grey(1), g.math(g.math(shade, g.grey(1), op=SUB), g.grey(2), op=MUL), op=ADD)
    shade = g.math(g.math(shade, g.grey(.60), op=MAX), g.grey(1.12), op=MIN)
    coat = g.blend(g.color(color), shade, mode=MULTIPLY)
    albedo = g.blend(coat, source, paint, mode=NORMAL_B)
    # Keep the narrow recess centered on the 3D projection, sampling its tone from
    # the generated finish. Its varied core sits inside the painted bevel and wear;
    # exposed machinery is excluded. No constant-color ruler stroke is applied.
    joint = g.colorize((rgba, 1), grad((0, "#434740"), (1, "#77776f")))
    albedo = g.blend(joint, albedo, g.math((mask, 1), original_armor, op=MUL), mode=NORMAL_B)
    g.material(albedo=albedo, metallic_v=0.0, roughness_v=0.85)
    return g


GRAPHS = {
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
