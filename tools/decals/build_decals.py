#!/usr/bin/env python3
"""Star Crew's decal textures: burn marks a damaged station wears (openspec/changes/repairs-on-deck, design 3f).

It owns assets/textures/decals/scorch_flat.png, scorch_streak.png and decals.json (each file's size and sha256).
CLAUDE.md 9: decals are a small set of textures committed as PNG sources, so they are written here, from a seed, by a
generator that is committed beside them, and never painted by hand.

A scorch is soot round a hot spot: a near-black core where the paint burnt away, a brown-black ring where it blistered,
a ragged edge from fractal noise, and flecks thrown out round it. scorch_streak is the same burn on a vertical face,
its soot carried upward by the heat. Colour is RGB in sRGB, coverage is alpha; the page draws a decal with ordinary
alpha blending over the lit surface.

Run (Python 3 with numpy and Pillow): python3 tools/decals/build_decals.py
"""

import hashlib
import json
import os

import numpy as np
from PIL import Image

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
OUT = os.path.join(ROOT, "assets", "textures", "decals")
SIZE_PX = 256
SEED = 20261010

CORE_SRGB = (0.03, 0.028, 0.027)   # burnt to the metal's char
RING_SRGB = (0.075, 0.065, 0.058)  # soot over blistered paint, near black


def value_noise(rng, size, cells):
    """Smooth noise in 0-1: a random lattice of cells x cells, interpolated with a smoothstep."""
    lat = rng.random((cells + 1, cells + 1))
    t = np.linspace(0, cells, size, endpoint=False)
    i = t.astype(int)
    f = t - i
    f = f * f * (3 - 2 * f)
    a = lat[i][:, i] * (1 - f)[None, :] + lat[i][:, i + 1] * f[None, :]
    b = lat[i + 1][:, i] * (1 - f)[None, :] + lat[i + 1][:, i + 1] * f[None, :]
    return a * (1 - f)[:, None] + b * f[:, None]


def fbm(rng, size, octaves=5, base=4):
    out = np.zeros((size, size))
    amp, total = 1.0, 0.0
    for k in range(octaves):
        out += amp * value_noise(rng, size, base * 2**k)
        total += amp
        amp *= 0.5
    return out / total


def smoothstep(e0, e1, v):
    t = np.clip((v - e0) / (e1 - e0), 0, 1)
    return t * t * (3 - 2 * t)


def scorch(rng, streak):
    n = SIZE_PX
    y, x = np.mgrid[0:n, 0:n] / (n - 1) * 2 - 1     # -1..1, y down the image
    if streak:
        # Heat carries the soot up: the burn's seat low in the image, its plume stretched upward and narrowing.
        cy = 0.42
        dy = y - cy
        plume = np.clip(-dy / 1.3, 0, 1)
        r = np.sqrt((x / (0.62 - 0.3 * plume)) ** 2 + np.where(dy < 0, dy / 2.1, dy / 0.55) ** 2) * 0.62
    else:
        r = np.sqrt(x**2 + y**2)
    # A ragged outline: the distance pushed about by fractal noise, so the edge has lobes and bays, not a circle.
    lobes = fbm(rng, n, octaves=4, base=3) - 0.5
    fine = fbm(rng, n, octaves=4, base=12) - 0.5
    r = r + 0.55 * lobes + 0.18 * fine
    outer = 1 - smoothstep(0.62, 0.70, r)           # the soot's edge, crisp
    char = 1 - smoothstep(0.18, 0.40, r)            # burnt to the metal
    tint = smoothstep(0.55, 0.66, r) * (1 - smoothstep(0.70, 0.82, r))   # heat-tinted paint just outside
    grain = fbm(rng, n, octaves=3, base=24)
    flecks = (rng.random((n, n)) > 0.994) & (r > 0.6) & (r < 0.9)
    alpha = np.clip(outer * (0.62 + 0.3 * grain + 0.25 * char), 0, 0.97)
    alpha = np.maximum(alpha, tint * 0.22)
    alpha = np.maximum(alpha, flecks * 0.9)
    if streak:   # the plume thins out before the top of its box, so it never ends on a straight line
        alpha = alpha * smoothstep(-0.98, -0.45, y)
    rgb = np.array(RING_SRGB)[None, None] * (0.85 + 0.3 * grain[..., None])
    rgb = rgb * (1 - char[..., None]) + np.array(CORE_SRGB)[None, None] * char[..., None]
    heat = np.array((0.24, 0.19, 0.15))[None, None]
    rgb = rgb * (1 - (tint * (1 - outer))[..., None]) + heat * (tint * (1 - outer))[..., None]
    rgba = np.concatenate([np.clip(rgb, 0, 1), alpha[..., None]], axis=-1)
    # Clear the outermost texels, so the decal's box edge never shows.
    rgba[[0, -1], :, 3] = 0
    rgba[:, [0, -1], 3] = 0
    return (rgba * 255 + 0.5).astype(np.uint8)


def main():
    os.makedirs(OUT, exist_ok=True)
    rng = np.random.default_rng(SEED)
    manifest = {"schema": "starcrew.decals/1", "built_by": "tools/decals/build_decals.py", "seed": SEED,
                "versions": {"numpy": np.__version__, "pillow": Image.__version__}, "decals": {}}
    for name, streak in (("scorch_flat", False), ("scorch_streak", True)):
        img = Image.fromarray(scorch(rng, streak), "RGBA")
        rel = f"assets/textures/decals/{name}.png"
        path = os.path.join(ROOT, rel)
        img.save(path, optimize=True)
        data = open(path, "rb").read()
        manifest["decals"][name] = {"file": rel, "px": [SIZE_PX, SIZE_PX], "bytes": len(data),
                                    "sha256": hashlib.sha256(data).hexdigest(), "vertical": streak}
        print(f"decals: {rel} ({len(data)} bytes)")
    with open(os.path.join(OUT, "decals.json"), "w", encoding="utf-8") as f:
        json.dump(manifest, f, indent=2)
        f.write("\n")


if __name__ == "__main__":
    main()
