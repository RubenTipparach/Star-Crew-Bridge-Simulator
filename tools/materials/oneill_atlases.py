#!/usr/bin/env python3
"""Package the O'Neill pair's textures from their Material Maker renders (spacedock, version E).

It owns the step from Material Maker's 2048 px renders (tools/materials/raw/oneill/, gitignored,
written by the material-maker-headless skill's render.sh from tools/materials/ptex/oneill_*.ptex)
to the committed textures in assets/textures/oneill/, and the contact sheet
docs/screenshots/materials/oneill-contact-sheet.png. It lives beside postprocess.py because it is
the same kind of step for a different kind of texture: these are fitted exterior atlases and
tiled detail maps, not 128 px layers of the texture array (tools/materials/README.md, "Exterior
atlases"). It resizes and packs channels; it never draws a pixel of its own (CLAUDE.md 9).

Two layers of texture, as the owner asked (2026-10-14: "we should have 2 layers of texture, detail
layer, and the big albiedo color map, and maybe normal maps on the details"):

- oneill_macro: the outside, wrapped once round a cylinder (x round it, y along it), 1024 x 1024 px.
  Alpha is the glazing's panes (Material Maker's emission output), drawn see-through.
- oneill_cap: once over an end dome, 512 x 512 px.
- oneill_land: the land inside, once over one land stripe (x across, y along), 512 x 2048 px, alpha its water.
- oneill_land_detail: the land's tiled detail (64 m a repeat), 256 x 256 px.
- oneill_clouds: the cloud cover inside, once round the cylinder, 512 x 512 px, alpha the cover (where the cumulus stand).
- oneill_dome: a dome's inside, engineered structure, once over the disc, 512 x 512 px, alpha its lit windows.
- oneill_detail, oneill_detail_normal: the tiled detail layer (16 m a repeat), 512 x 512 px, its
  albedo near white (it multiplies the macro colour) and its tangent-space normal map
  (OpenGL convention, green up), for the per-pixel lighting proposed in the spacedock design.
- oneill_mirror, oneill_mirror_normal: a mirror wing's facets, 256 x 256 px.

Run: python3 tools/materials/oneill_atlases.py   (prints a table and a digest; deterministic)
"""
import hashlib
import os
import sys

import numpy as np
from PIL import Image, ImageDraw

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
RAW = os.path.join(ROOT, "tools", "materials", "raw", "oneill")
OUT = os.path.join(ROOT, "assets", "textures", "oneill")
SHEET = os.path.join(ROOT, "docs", "screenshots", "materials", "oneill-contact-sheet.png")

# name: (graph, map, size px, what one repeat covers, alpha source)
TEXTURES = [
    ("oneill_macro", "oneill_macro", "albedo", 1024, "once round the cylinder", "emission"),
    ("oneill_land", "oneill_land", "albedo", (512, 2048), "once over a land stripe", "emission"),
    ("oneill_land_detail", "oneill_land_detail", "albedo", 256, "64 m of land, tiled", None),
    ("oneill_clouds", "oneill_clouds", "albedo", 512, "the cloud cover map", "emission"),
    ("oneill_dome", "oneill_dome", "albedo", 512, "once over a dome's inside", "emission"),
    ("oneill_cap", "oneill_cap", "albedo", 512, "once over an end cap", None),
    ("oneill_detail", "oneill_detail", "albedo", 512, "16 m of hull, tiled", None),
    ("oneill_detail_normal", "oneill_detail", "normal", 512, "16 m of hull, tiled", "opaque"),
    ("oneill_mirror", "oneill_mirror", "albedo", 256, "one mirror facet group, tiled", None),
    ("oneill_mirror_normal", "oneill_mirror", "normal", 256, "one mirror facet group, tiled", "opaque"),
]


def mip_bytes(px):
    """RGBA8 bytes of a texture (px, or (w, h)) with its full mip chain."""
    w, h = px if isinstance(px, tuple) else (px, px)
    total = 0
    while True:
        total += w * h * 4
        if w == 1 and h == 1:
            return total
        w, h = max(1, w // 2), max(1, h // 2)


def build(name, graph, kind, px, alpha):
    src = os.path.join(RAW, f"{graph}_{kind}.png")
    if not os.path.exists(src):
        sys.exit(f"oneill_atlases: no render {os.path.relpath(src, ROOT)} (render the graphs first, tools/materials/README.md)")
    size = px if isinstance(px, tuple) else (px, px)
    rgb = Image.open(src).convert("RGB").resize(size, Image.Resampling.LANCZOS)
    a = np.zeros((size[1], size[0]), np.uint8)
    if alpha == "opaque":
        a[:] = 255
    elif alpha == "emission":
        em = np.asarray(Image.open(os.path.join(RAW, f"{graph}_emission.png")).convert("RGB"), np.float32)
        mask = (em.max(axis=2) > 6.0).astype(np.float32) * 255.0
        if graph in ("oneill_clouds", "oneill_land"):
            mask = em.max(axis=2)   # soft-edged: the cloud cover, the land's water
        a = np.asarray(Image.fromarray(mask.astype(np.uint8)).resize(size, Image.Resampling.BOX))
    img = np.dstack([np.asarray(rgb), a]).astype(np.uint8)
    os.makedirs(OUT, exist_ok=True)
    path = os.path.join(OUT, name + ".png")
    Image.fromarray(img, "RGBA").save(path, compress_level=9)
    return img, path


def main():
    rows, digest, tiles = [], hashlib.sha256(), []
    for name, graph, kind, px, covers, alpha in TEXTURES:
        img, path = build(name, graph, kind, px, alpha)
        digest.update(img.tobytes())
        rows.append((name, px, covers, mip_bytes(px)))
        tiles.append((name, px, covers, img))
    # The contact sheet: each texture at 384 px with its name and size, the tiled ones 2 x 2 beside.
    cell = 384
    sheet = Image.new("RGB", (cell * 4 + 50, (cell + 40) * ((len(tiles) + 1) // 2) + 10), (18, 22, 28))
    d = ImageDraw.Draw(sheet)
    for i, (name, px, covers, img) in enumerate(tiles):
        col, row = (i % 2) * 2, i // 2
        x, y = 10 + col * (cell + 10), 10 + row * (cell + 40)
        rgb = Image.fromarray(img[:, :, :3])
        sheet.paste(rgb.resize((cell, cell), Image.Resampling.LANCZOS), (x, y))
        if "tiled" in covers:
            half = rgb.resize((cell // 2, cell // 2), Image.Resampling.LANCZOS)
            for k in range(4):
                sheet.paste(half, (x + cell + 10 + (k % 2) * (cell // 2), y + (k // 2) * (cell // 2)))
        elif img[:, :, 3].any():
            a = Image.fromarray(img[:, :, 3]).convert("RGB").resize((cell, cell), Image.Resampling.BOX)
            sheet.paste(a, (x + cell + 10, y))
        dims = f"{px[0]} x {px[1]}" if isinstance(px, tuple) else f"{px}"
        d.text((x, y + cell + 6), f"{name}  {dims} px  {covers}", fill=(220, 228, 236))   # a tall one is shown squeezed
    os.makedirs(os.path.dirname(SHEET), exist_ok=True)
    sheet.save(SHEET)
    total = 0
    for name, px, covers, b in rows:
        total += b
        dims = f"{px[0]} x {px[1]}" if isinstance(px, tuple) else f"{px}"
        print(f"{name:24s} {dims:>11s} px  {covers:32s} {b:10,d} bytes with mips")
    print(f"{'total':24s} {'':8s}  {'':32s} {total:10,d} bytes ({total / 1048576:.2f} MiB)")
    print("sheet:", os.path.relpath(SHEET, ROOT))
    print("digest:", digest.hexdigest())


if __name__ == "__main__":
    main()
