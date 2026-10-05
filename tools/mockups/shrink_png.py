#!/usr/bin/env python3
"""Shrink committed screenshots: re-save each PNG as a 256-colour palette image, dithered.

Mockup screenshots are 1440 x 900 renders of flat-shaded, textured rooms; as full-colour PNGs
they are about 1 MB each, and the repository holds about a hundred. A 256-colour palette with
Floyd-Steinberg dithering keeps them readable for judging a layout or a lighting state (the
only job they have; CLAUDE.md section 11) at about a quarter of the size. A shot already in
palette mode is left alone, so running this twice changes nothing.

Documentation tooling (CLAUDE.md section 4). Needs Pillow.

Usage: python3 tools/mockups/shrink_png.py [file.png | directory ...]
       (default: docs/screenshots)
"""

import os
import sys

from PIL import Image

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))


def shrink(path):
    """Re-save one PNG as a dithered 256-colour palette image; returns (bytes before, bytes after)."""
    before = os.path.getsize(path)
    with Image.open(path) as im:
        if im.mode == "P":
            return before, before
        rgb = im.convert("RGB")
    q = rgb.quantize(colors=256, method=Image.Quantize.FASTOCTREE, dither=Image.Dither.FLOYDSTEINBERG)
    tmp = path + ".tmp"
    q.save(tmp, format="PNG", optimize=True)
    after = os.path.getsize(tmp)
    if after < before:
        os.replace(tmp, path)
        return before, after
    os.remove(tmp)
    return before, before


def main(argv):
    targets = argv or [os.path.join(ROOT, "docs", "screenshots")]
    files = []
    for t in targets:
        if os.path.isdir(t):
            for d, _, names in os.walk(t):
                files += [os.path.join(d, n) for n in sorted(names) if n.lower().endswith(".png")]
        elif t.lower().endswith(".png"):
            files.append(t)
    total_b = total_a = 0
    for f in sorted(files):
        b, a = shrink(f)
        total_b += b
        total_a += a
    print(f"  {len(files)} PNGs: {total_b / 1e6:.1f} MB -> {total_a / 1e6:.1f} MB")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
