#!/usr/bin/env python3
"""Turn Material Maker renders into Star Crew's texture layers.

It owns the step between a material's source maps (albedo, normal, occlusion, emission, as
Material Maker renders them) and the 128 px RGBA layers the game samples
(assets/textures/<name>.png), plus the contact sheet that is looked at before a material is
called done (docs/screenshots/materials/contact-sheet.png). It lives in tools/ because it is an
asset build step: the game reads its output, never its input.

data/materials/materials.json is the one source for what is built: which graph each material
comes from, its tile and span in metres, and its post-process knobs. Its "_rules" say what a
layer is; this file does what they say, in this order:

  1. load the graph's maps from the raw directory (both namings, see load_maps);
  2. recolour (optional "ramp"): luminance, stretched between two percentiles of the material,
     mapped onto a dark-to-light colour ramp and blended with the original by "ramp_mix";
  3. relief bake: the Pi 5 renderer has no per-pixel lighting, so the normal map and occlusion
     are baked into the albedo under one fixed key light (bake_relief);
  4. fit the layer: tile the graph k x k (or take 1/k of it) so the layer spans its "span_m",
     then Lanczos-downsample with wrap-around so the layer itself tiles without a seam;
  5. palette-reduce (optional "colours") with Pillow's median cut, no dithering;
  6. write RGB albedo with alpha as the emission mask (255 fully emissive, 0 not).

Output is byte-for-byte deterministic for a given Pillow and numpy: no randomness, no
timestamps, stable order. Run it twice and compare hashes; the last line prints a digest.

Provenance: adapted from fps-game-demo (Undercity/Brushfire) revision f6cd25c,
tools/material_maker/postprocess.py: the raw export naming, the Lanczos downsample, the normal
renormalisation and the luminance ramp come from there. Undercity writes Godot PBR materials at
1024 px; Star Crew bakes relief into a small albedo layer instead (no PBR on a Pi 5,
openspec/changes/engine-stack design section 5).

Usage:
  python3 tools/materials/postprocess.py [--raw DIR]   build every layer and the contact sheet
  python3 tools/materials/postprocess.py --list-graphs print the graphs materials.json uses
Needs numpy and Pillow (pip install numpy pillow).
"""

import argparse
import hashlib
import json
import math
import os
import sys

import numpy as np
from PIL import Image, ImageDraw, ImageFont

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(os.path.dirname(HERE))
MATERIALS_JSON = os.path.join(ROOT, "data", "materials", "materials.json")
PTEX_DIR = os.path.join(HERE, "ptex")
RAW_DIR = os.path.join(HERE, "raw")
TEXTURE_DIR = os.path.join(ROOT, "assets", "textures")
SHEET_PATH = os.path.join(ROOT, "docs", "screenshots", "materials", "contact-sheet.png")

SCHEMA = "starcrew.materials/1"

# The one source for each knob's default (CLAUDE.md 6.5). A missing knob takes its default here;
# a present zero is zero. "ramp": None means no recolour, "colours": None no palette reduction.
DEFAULTS = {
    "ramp": None,
    "ramp_mix": 1.0,
    "relief": 1.0,
    "relief_ambient": 0.45,
    "colours": 32,
    "emissive": False,
}
REQUIRED = ("layer", "role", "source", "tile_m", "span_m")
TOP_KEYS = {"schema", "status", "layers", "bake", "materials"}
LAYER_KEYS = {"px", "interior_span_m", "exterior_span_m", "format", "mag_filter", "min_filter"}
BAKE_KEYS = {"key_light_tangent", "luminance_weights", "ramp_stretch_percentiles"}
SOURCE_KEYS = {"graph", "provenance"}

# The raw maps a graph renders to. Material Maker's command-line export (--export-material)
# writes <graph>_albedo.png; fps-game-demo's committed exports (its postprocess.py output in
# game/textures, 1024 px, normals renormalised) name the albedo <graph>.png. The other maps are
# named alike in both. One loader reads either (load_maps).
NAMINGS = (
    ("Material Maker export", {"albedo": "{g}_albedo.png", "normal": "{g}_normal.png",
                               "orm": "{g}_orm.png", "emission": "{g}_emission.png"}),
    ("fps-game-demo export", {"albedo": "{g}.png", "normal": "{g}_normal.png",
                              "orm": "{g}_orm.png", "emission": "{g}_emission.png"}),
)

# Wrap padding around a layer before the Lanczos downsample, in output pixels. Lanczos-3 reaches
# 3 output pixels either side, so 4 is enough for the layer's edges to see its far side.
WRAP_PAD_PX = 4
# A seam ratio (wrap-around step against the mean step inside the layer) above this is reported.
SEAM_WARN = 2.0
SHEET_SCALE = 3
SHEET_COLUMNS = 6


class MaterialError(SystemExit):
    """A materials.json or raw-map problem: stops the build with the path and field."""


def fail(msg):
    raise MaterialError(f"{os.path.relpath(MATERIALS_JSON, ROOT)}: {msg}")


def finite(v, where, lo=None, hi=None, positive=False):
    if isinstance(v, bool) or not isinstance(v, (int, float)) or not math.isfinite(v):
        fail(f"{where} must be a finite number, got {v!r}")
    if positive and v <= 0:
        fail(f"{where} must be above 0, got {v}")
    if lo is not None and v < lo or hi is not None and v > hi:
        fail(f"{where} must be in {lo}-{hi}, got {v}")
    return float(v)


def unknown_keys(obj, known, where):
    extra = sorted(k for k in obj if k not in known and not k.startswith("_"))
    if extra:
        fail(f"{where}: unknown key {', '.join(extra)} (a misspelt knob would silently do nothing)")


def load_manifest():
    """Read and validate materials.json. Returns (layers, bake, materials) with defaults filled,
    materials in layer order. Any problem stops the build with the field it is in."""
    with open(MATERIALS_JSON) as f:
        doc = json.load(f)
    if doc.get("schema") != SCHEMA:
        fail(f"schema must be {SCHEMA!r}, got {doc.get('schema')!r}")
    unknown_keys(doc, TOP_KEYS, "top level")

    layers = doc.get("layers") or fail("missing 'layers'")
    unknown_keys(layers, LAYER_KEYS, "layers")
    px = layers.get("px")
    if not isinstance(px, int) or isinstance(px, bool) or px < 8 or px & (px - 1):
        fail(f"layers.px must be a power of two of at least 8, got {px!r}")
    spans = {"interior": finite(layers.get("interior_span_m"), "layers.interior_span_m", positive=True),
             "exterior": finite(layers.get("exterior_span_m"), "layers.exterior_span_m", positive=True)}

    bake = doc.get("bake") or fail("missing 'bake'")
    unknown_keys(bake, BAKE_KEYS, "bake")
    light = bake.get("key_light_tangent")
    if not isinstance(light, list) or len(light) != 3:
        fail("bake.key_light_tangent must be [x, y, z]")
    light = np.array([finite(c, "bake.key_light_tangent") for c in light], dtype=np.float64)
    if light[2] <= 0:
        fail("bake.key_light_tangent must point out of the surface (z above 0)")
    weights = bake.get("luminance_weights")
    if not isinstance(weights, list) or len(weights) != 3:
        fail("bake.luminance_weights must be [r, g, b]")
    weights = np.array([finite(c, "bake.luminance_weights", 0, 1) for c in weights], dtype=np.float32)
    pct = bake.get("ramp_stretch_percentiles")
    if not isinstance(pct, list) or len(pct) != 2:
        fail("bake.ramp_stretch_percentiles must be [low, high]")
    pct = [finite(p, "bake.ramp_stretch_percentiles", 0, 100) for p in pct]
    if pct[0] >= pct[1]:
        fail("bake.ramp_stretch_percentiles must rise")
    bake = {"light": light / np.linalg.norm(light), "weights": weights, "pct": pct}

    mats = doc.get("materials") or fail("missing 'materials'")
    out = []
    for name, spec in mats.items():
        where = f"materials.{name}"
        if not isinstance(spec, dict):
            fail(f"{where} must be an object")
        unknown_keys(spec, set(REQUIRED) | set(DEFAULTS), where)
        for k in REQUIRED:
            if k not in spec:
                fail(f"{where}: missing '{k}'")
        m = dict(DEFAULTS)
        m.update(spec)
        m["name"] = name
        if not isinstance(m["layer"], int) or isinstance(m["layer"], bool):
            fail(f"{where}.layer must be an integer")
        src = m["source"]
        if not isinstance(src, dict):
            fail(f"{where}.source must be an object with graph and provenance")
        unknown_keys(src, SOURCE_KEYS, f"{where}.source")
        if not isinstance(src.get("graph"), str) or not isinstance(src.get("provenance"), str):
            fail(f"{where}.source needs a graph name and its provenance")
        m["tile_m"] = finite(m["tile_m"], f"{where}.tile_m", positive=True)
        m["span_m"] = finite(m["span_m"], f"{where}.span_m", positive=True)
        space = [s for s, v in spans.items() if v == m["span_m"]]
        if not space:
            fail(f"{where}.span_m must be an interior or exterior span ({sorted(spans.values())}), got {m['span_m']}")
        m["space"] = space[0]
        m["repeat"] = repeat_of(m["tile_m"], m["span_m"], px, where)
        if m["ramp"] is not None:
            r = m["ramp"]
            if not (isinstance(r, list) and len(r) == 2 and all(isinstance(c, list) and len(c) == 3 for c in r)):
                fail(f"{where}.ramp must be [[r, g, b] dark, [r, g, b] light]")
            m["ramp"] = np.array([[finite(v, f"{where}.ramp", 0, 1) for v in c] for c in r], dtype=np.float32)
        elif "ramp_mix" in spec:
            fail(f"{where}: ramp_mix without a ramp")
        m["ramp_mix"] = finite(m["ramp_mix"], f"{where}.ramp_mix", 0, 1)
        m["relief"] = finite(m["relief"], f"{where}.relief", 0, 1)
        m["relief_ambient"] = finite(m["relief_ambient"], f"{where}.relief_ambient", 0, 1)
        c = m["colours"]
        if c is not None and (not isinstance(c, int) or isinstance(c, bool) or not 2 <= c <= 256):
            fail(f"{where}.colours must be 2-256, or null for no palette reduction, got {c!r}")
        if not isinstance(m["emissive"], bool):
            fail(f"{where}.emissive must be true or false")
        if not isinstance(m["role"], str) or not m["role"]:
            fail(f"{where}.role must say what surfaces use it")
        out.append(m)
    out.sort(key=lambda m: m["layer"])
    if [m["layer"] for m in out] != list(range(len(out))):
        fail(f"layers must be numbered 0-{len(out) - 1} once each, got {[m['layer'] for m in out]}")
    return {"px": px, "spans": spans, **{k: layers[k] for k in ("format", "mag_filter", "min_filter") if k in layers}}, bake, out


def repeat_of(tile_m, span_m, px, where):
    """How the graph fills a layer: (k, 1) tiles it k x k, (1, c) takes the top-left 1/c of it.
    Both must be whole, so the layer tiles exactly where the graph does."""
    ratio = span_m / tile_m
    if ratio >= 1:
        k = round(ratio)
        if abs(k - ratio) > 1e-9 or px % k:
            fail(f"{where}: span_m / tile_m must be a whole number dividing {px} px, got {ratio:g}")
        return (k, 1)
    c = round(1 / ratio)
    if abs(c - 1 / ratio) > 1e-9:
        fail(f"{where}: tile_m / span_m must be a whole number, got {1 / ratio:g}")
    return (1, c)


def read_rgb(path):
    return np.asarray(Image.open(path).convert("RGB"), dtype=np.float32) / 255.0


def load_maps(raw, graph):
    """A graph's maps from `raw`, in either naming (NAMINGS). Returns float arrays in 0-1:
    albedo (RGB), normal (unit tangent-space vectors, OpenGL convention: green is up the image),
    ao (from the ORM map's red channel, 1 where there is none), emission (RGB, or None)."""
    found = [(label, n) for label, n in NAMINGS if os.path.exists(os.path.join(raw, n["albedo"].format(g=graph)))]
    if not found:
        raise MaterialError(f"{raw}: no albedo for graph {graph!r} (looked for "
                            + ", ".join(n["albedo"].format(g=graph) for _, n in NAMINGS) + ")")
    if len(found) > 1:
        raise MaterialError(f"{raw}: graph {graph!r} is there in two namings; clear the directory and export again")
    label, naming = found[0]
    path = {k: os.path.join(raw, v.format(g=graph)) for k, v in naming.items()}
    albedo = read_rgb(path["albedo"])
    size = albedo.shape[:2]
    if size[0] != size[1]:
        raise MaterialError(f"{path['albedo']}: not square ({size[1]} x {size[0]})")

    def same_size(a, key):
        if a.shape[:2] != size:
            raise MaterialError(f"{path[key]}: {a.shape[1]} px, the albedo is {size[1]} px")
        return a

    if os.path.exists(path["normal"]):
        n = same_size(read_rgb(path["normal"]), "normal") * 2.0 - 1.0
        n[..., 2] = np.clip(n[..., 2], 0.0, 1.0)
        n /= np.maximum(np.linalg.norm(n, axis=2, keepdims=True), 1e-6)
    else:
        n = np.zeros(size + (3,), dtype=np.float32)
        n[..., 2] = 1.0
    ao = same_size(read_rgb(path["orm"]), "orm")[..., 0] if os.path.exists(path["orm"]) else np.ones(size, np.float32)
    emission = same_size(read_rgb(path["emission"]), "emission") if os.path.exists(path["emission"]) else None
    return {"label": label, "albedo": albedo, "normal": n, "ao": ao, "emission": emission}


def recolour(albedo, ramp, mix, bake):
    """Map luminance onto the ramp and blend with the original by `mix`. Luminance is stretched
    so the material's low percentile lands on the dark end and its high percentile on the light
    end, so a ramp's two colours are the colours of the material's darkest and lightest texels."""
    lum = albedo @ bake["weights"]
    lo, hi = np.percentile(lum, bake["pct"])
    t = np.clip((lum - lo) / max(hi - lo, 1e-6), 0.0, 1.0)[..., None]
    ramped = ramp[0] + (ramp[1] - ramp[0]) * t
    return albedo + (ramped - albedo) * mix


def bake_relief(albedo, normal, ao, relief, ambient, bake):
    """Bake the normal map and occlusion into the albedo under the fixed key light:
    shade = ao * (ambient + (1 - ambient) * max(0, n.L) / max(1e-3, n0.L)), n0 = (0, 0, 1).
    A flat texel keeps its colour times its occlusion; a face toward the light brightens, one
    away from it darkens. `relief` blends albedo toward albedo * shade. The result is clamped."""
    light = bake["light"].astype(np.float32)
    n_dot_l = np.maximum(normal @ light, 0.0)
    shade = ao * (ambient + (1.0 - ambient) * n_dot_l / max(1e-3, float(light[2])))
    return np.clip(albedo * (1.0 + relief * (shade[..., None] - 1.0)), 0.0, 1.0)


def fit_layer(img, repeat, px):
    """Fill one layer from a graph render: tile it k x k or take its top-left 1/c, then Lanczos
    downsample to px with wrap-around padding, so the layer's edges are filtered with the far
    side of the layer and it tiles without a seam. Works per channel in float."""
    k, c = repeat
    if k > 1:
        img = np.tile(img, (k, k, 1))
    if c > 1:
        n = img.shape[0] // c
        if n * c != img.shape[0]:
            raise MaterialError(f"a {img.shape[0]} px render cannot be cut into {c} x {c}")
        img = img[:n, :n]
    n = img.shape[0]
    if n % px:
        raise MaterialError(f"a {n} px region does not downsample evenly to {px} px")
    pad = WRAP_PAD_PX * (n // px)
    img = np.pad(img, ((pad, pad), (pad, pad), (0, 0)), mode="wrap")
    out_px = px + 2 * WRAP_PAD_PX
    chans = [np.asarray(Image.fromarray(np.ascontiguousarray(img[..., i]), "F").resize((out_px, out_px), Image.LANCZOS))
             for i in range(img.shape[2])]
    out = np.stack(chans, axis=2)[WRAP_PAD_PX:WRAP_PAD_PX + px, WRAP_PAD_PX:WRAP_PAD_PX + px]
    return np.clip(out, 0.0, 1.0)


def to_u8(a):
    return (np.clip(a, 0.0, 1.0) * 255.0 + 0.5).astype(np.uint8)


def reduce_palette(rgb8, colours, method="median_cut"):
    """Palette reduction without dithering: deterministic, and a per-texel mapping, so a layer that
    tiled before still tiles. Median cut (the default, the materials' and panels') splits the colour
    boxes by how many texels they hold; max coverage (the props' atlases) by their colour range, so a
    small saturated area, a lamp or a station's stripe, keeps its own colour."""
    how = {"median_cut": Image.Quantize.MEDIANCUT, "max_coverage": Image.Quantize.MAXCOVERAGE}[method]
    img = Image.fromarray(rgb8, "RGB").quantize(colors=colours, method=how, dither=Image.Dither.NONE)
    return np.asarray(img.convert("RGB"))


def build_layer(m, raw, cfg, bake):
    maps = load_maps(raw, m["source"]["graph"])
    if m["emissive"] and maps["emission"] is None:
        raise MaterialError(f"materials.{m['name']} is emissive but graph {m['source']['graph']!r} has no emission map in {raw}")
    albedo = maps["albedo"]
    if m["ramp"] is not None:
        albedo = recolour(albedo, m["ramp"], m["ramp_mix"], bake)
    if m["relief"] > 0:
        albedo = bake_relief(albedo, maps["normal"], maps["ao"], m["relief"], m["relief_ambient"], bake)
    if m["emissive"]:
        mask = maps["emission"].max(axis=2)
    else:
        mask = np.zeros(albedo.shape[:2], np.float32)
    layer = fit_layer(np.dstack([albedo, mask]).astype(np.float32), m["repeat"], cfg["px"])
    rgb8 = to_u8(layer[..., :3])
    if m["colours"] is not None:
        rgb8 = reduce_palette(rgb8, m["colours"])
    a8 = to_u8(layer[..., 3]) if m["emissive"] else np.zeros(rgb8.shape[:2], np.uint8)
    return np.dstack([rgb8, a8]), maps["label"]


def seam_ratio(rgb8, axis):
    """The colour step across the layer's wrap-around edge against the mean step between
    neighbouring texels inside it, along one axis. About 1 means the seam is invisible."""
    a = rgb8.astype(np.float32)
    inner = np.abs(np.diff(a, axis=axis)).mean()
    first = np.take(a, 0, axis=axis)
    last = np.take(a, -1, axis=axis)
    return float(np.abs(first - last).mean() / max(inner, 1e-6))


def gpu_bytes(px):
    """RGBA8 bytes of one layer with its full mip chain (128 px: 87,380)."""
    total, s = 0, px
    while s >= 1:
        total += s * s * 4
        s //= 2
    return total


def save_png(img, path):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    img.save(path, optimize=True)


def contact_sheet(layers, cfg):
    """Every layer at SHEET_SCALE x nearest neighbour with its name, and under it the same layer
    tiled 3 x 3 at 1 x, so a seam shows as a line through the middle tile. RGB only; the label
    says how much of an emissive layer glows."""
    px = cfg["px"]
    cell = px * SHEET_SCALE
    gap, label_h = 16, 48
    cols = min(SHEET_COLUMNS, len(layers))
    rows = (len(layers) + cols - 1) // cols
    w = gap + cols * (cell + gap)
    h = gap + rows * (label_h + 2 * cell + 2 * gap)
    sheet = Image.new("RGB", (w, h), (24, 26, 30))
    draw = ImageDraw.Draw(sheet)
    font = ImageFont.load_default(size=18)
    small = ImageFont.load_default(size=14)
    for i, (m, rgba) in enumerate(layers):
        x = gap + (i % cols) * (cell + gap)
        y = gap + (i // cols) * (label_h + 2 * cell + 2 * gap)
        draw.text((x, y), f"{m['layer']}  {m['name']}", fill=(232, 234, 238), font=font)
        glow = f", glows on {np.count_nonzero(rgba[..., 3] >= 128) / rgba[..., 3].size:.0%}" if m["emissive"] else ""
        draw.text((x, y + 24), f"{m['source']['graph']}, {m['span_m']:g} m, {px / m['span_m']:g} px/m{glow}",
                  fill=(150, 156, 166), font=small)
        rgb = Image.fromarray(np.ascontiguousarray(rgba[..., :3]), "RGB")
        sheet.paste(rgb.resize((cell, cell), Image.NEAREST), (x, y + label_h))
        tiled = Image.fromarray(np.tile(rgba[..., :3], (3, 3, 1)), "RGB")
        sheet.paste(tiled, (x, y + label_h + cell + gap))
    return sheet


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--raw", default=RAW_DIR, help="directory of raw maps (default tools/materials/raw)")
    ap.add_argument("--list-graphs", action="store_true", help="print the graphs materials.json uses and stop")
    ap.add_argument("--only", nargs="+", help="rebuild these layers; use committed PNGs for the rest of the contact sheet")
    args = ap.parse_args()

    cfg, bake, mats = load_manifest()
    if args.only and set(args.only) - {m["name"] for m in mats}:
        fail("--only names an unknown material")
    if args.list_graphs:
        for g in sorted({m["source"]["graph"] for m in mats}):
            print(g)
        return
    for g in sorted({m["source"]["graph"] for m in mats}):
        if not os.path.exists(os.path.join(PTEX_DIR, g + ".ptex")):
            print(f"note: graph {g} has no tools/materials/ptex/{g}.ptex; it exists only as an export")

    source_note = os.path.join(args.raw, "SOURCE.txt")
    if os.path.exists(source_note):
        with open(source_note) as f:
            print("raw maps:", f.read().strip())

    built = []
    for m in mats:
        path = os.path.join(TEXTURE_DIR, m["name"] + ".png")
        if not args.only or m["name"] in args.only:
            rgba, label = build_layer(m, args.raw, cfg, bake)
            save_png(Image.fromarray(rgba, "RGBA"), path)
        else:
            rgba = np.asarray(Image.open(path).convert("RGBA"))
            label = "committed layer"
        built.append((m, rgba, label, path))

    save_png(contact_sheet([(m, rgba) for m, rgba, _, _ in built], cfg), SHEET_PATH)

    px = cfg["px"]
    head = f"{'layer':>5}  {'name':<12} {'graph':<15} {'tile m':>6} {'span m':>6} {'fill':>5} {'px':>4} {'px/m':>5} {'gpu bytes':>10} {'png bytes':>9} {'seam x':>6} {'seam y':>6}"
    print(head)
    print("-" * len(head))
    total = 0
    digest = hashlib.sha256()
    warnings = []
    for m, rgba, label, path in built:
        k, c = m["repeat"]
        fill = f"{k}x{k}" if k > 1 else (f"1/{c}" if c > 1 else "1x1")
        sx, sy = seam_ratio(rgba[..., :3], 1), seam_ratio(rgba[..., :3], 0)
        size = gpu_bytes(px)
        total += size
        with open(path, "rb") as f:
            data = f.read()
        digest.update(m["name"].encode() + b"\0" + data)
        print(f"{m['layer']:>5}  {m['name']:<12} {m['source']['graph']:<15} {m['tile_m']:>6g} {m['span_m']:>6g} {fill:>5} "
              f"{px:>4} {px / m['span_m']:>5g} {size:>10,} {len(data):>9,} {sx:>6.2f} {sy:>6.2f}")
        if max(sx, sy) > SEAM_WARN:
            warnings.append(f"{m['name']}: seam ratio {max(sx, sy):.2f} above {SEAM_WARN}; look at its 3 x 3 tiling")
    print("-" * len(head))
    print(f"{len(built)} layers of {px} x {px} RGBA8 with mips in one texture array: {total:,} bytes "
          f"({total / 2**20:.2f} MiB); the Pi 5 texture budget is openspec/changes/engine-stack design section 5")
    print(f"raw maps from: {', '.join(sorted({label for _, _, label, _ in built}))} in {os.path.relpath(args.raw, ROOT)}")
    print(f"contact sheet: {os.path.relpath(SHEET_PATH, ROOT)}")
    for w in warnings:
        print("warning:", w)
    print(f"digest: {digest.hexdigest()}")


if __name__ == "__main__":
    main()
