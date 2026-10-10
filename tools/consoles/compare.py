#!/usr/bin/env python3
"""Lay the engine's console beside the mockup's, and measure how far apart they are.

The console parity check (openspec/changes/console-parity, design 4): tools/consoles/parity.mjs captures the mockup's
named shots and their states (`<shot>-mockup.png`, `<shot>.json`); `sc-client --console-fixture` draws each state in
the engine (`<shot>-engine.png`). For every shot with both pictures this writes `<shot>-compare.png`: the mockup, the
engine and their difference, one above the other. The viewscreen's inside is left out: the engine shows its 3D feed
there, the mockup a flat sketch of one.

Two numbers, per region of the console (title band, look band, each panel, status strip):

- **pixels**: the share of pixels that differ by more than --threshold in any channel. Font smoothing alone leaves
  1-8 %, so it says where to look, not whether something is wrong.
- **missing**: the share of the region's ink (pixels that are not the region's background, in either picture) where
  the two pictures, each blurred by 2.5 px, differ by more than 32. Blurring forgives smoothing and sub-pixel
  placement; a control that is missing, extra, a different colour or moved by 3 px or more is not forgiven. Measured
  2026-10-10: 0-2.5 % on the seven shots, about 7 % for a panel's contents moved 2 px, 20 % for 4 px, 86-94 % for a
  panel drawn empty.

With --max-missing (and --max-pct for the whole shot's pixels) it is the check scripts/check.sh runs, and it fails
when a region is over the limit.

Usage: python3 tools/consoles/compare.py DIR [--threshold 48] [--max-pct 2.5 --max-missing 10]
"""
import argparse
import json
import sys
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw, ImageFilter

# The console's regions in layout points (consoles.html: the bands and the 104 x 102 grid).
def grid(c, r, w, h):
    return (20 + 104 * c, 280 + 102 * r, 104 * w - 8, 102 * h - 8)

PANELS = {
    "helm": {"THRUST": grid(0, 0, 3, 4), "SCANNER": grid(3, 0, 5, 4), "ATTITUDE": grid(8, 0, 4, 2), "ORIENT": grid(8, 2, 4, 2)},
    "tactical": {"TARGETS": grid(0, 0, 3, 4), "PLOT": grid(3, 0, 5, 4), "TURRETS": grid(8, 0, 4, 2), "TUBES": grid(8, 2, 4, 2)},
}
BANDS = {"title band": (0, 0, 1280, 32), "look band": (0, 32, 1280, 240), "status strip": (0, 688, 1280, 32)}
VIEWSCREEN = (333, 53, 614, 194)
BLUR_PX = 2.5
INK = 24
MISSING = 32


def missing_pct(mb, eb, region, mask):
    """The share of the region's ink where the blurred pictures differ (see the module's doc)."""
    x, y, w, h = region
    ms, es, keep = mb[y : y + h, x : x + w], eb[y : y + h, x : x + w], mask[y : y + h, x : x + w]
    bg = np.median(ms.reshape(-1, 3), axis=0)
    ink = ((np.abs(ms - bg).max(axis=2) > INK) | (np.abs(es - bg).max(axis=2) > INK)) & keep
    bad = (np.abs(ms - es).max(axis=2) > MISSING) & ink
    return round(100.0 * bad.sum() / max(1, ink.sum()), 2)


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("dir")
    ap.add_argument("--threshold", type=int, default=48, help="a pixel differs when a channel moves by more than this")
    ap.add_argument("--max-pct", type=float, help="fail when a shot's share of differing pixels is over this")
    ap.add_argument("--max-missing", type=float, help="fail when any region's missing share is over this")
    a = ap.parse_args()
    d = Path(a.dir)
    report = {}
    fails = []
    for mock in sorted(d.glob("*-mockup.png")):
        shot = mock.name[: -len("-mockup.png")]
        eng = d / f"{shot}-engine.png"
        state = d / f"{shot}.json"
        if not eng.exists() or not state.exists():
            continue
        station = json.loads(state.read_text())["station"]
        m = np.asarray(Image.open(mock).convert("RGB")).astype(np.int16)
        e = np.asarray(Image.open(eng).convert("RGB")).astype(np.int16)
        if m.shape != e.shape:
            sys.exit(f"{shot}: mockup {m.shape} and engine {e.shape} differ in size")
        diff = np.abs(m - e).max(axis=2)
        mask = np.ones(diff.shape, bool)
        x, y, w, h = VIEWSCREEN
        mask[y : y + h, x : x + w] = False
        bad = (diff > a.threshold) & mask
        regions = dict(BANDS)
        regions.update(PANELS.get(station, {}))
        mb = np.asarray(Image.open(mock).convert("RGB").filter(ImageFilter.GaussianBlur(BLUR_PX))).astype(np.int16)
        eb = np.asarray(Image.open(eng).convert("RGB").filter(ImageFilter.GaussianBlur(BLUR_PX))).astype(np.int16)
        rows, miss = {}, {}
        for name, (x, y, w, h) in regions.items():
            sub = bad[y : y + h, x : x + w]
            keep = mask[y : y + h, x : x + w]
            rows[name] = round(100.0 * sub.sum() / max(1, keep.sum()), 2)
            miss[name] = missing_pct(mb, eb, (x, y, w, h), mask)
        total = round(100.0 * bad.sum() / mask.sum(), 2)
        report[shot] = {"differing_pct": total, "regions": rows, "missing_pct": miss}
        heat = np.zeros_like(m, dtype=np.uint8)
        heat[..., 0] = np.clip(diff * 2, 0, 255).astype(np.uint8)
        heat[..., 1] = np.clip(diff, 0, 255).astype(np.uint8) // 3
        heat[~mask] = (20, 20, 40)
        out = Image.new("RGB", (1280, 720 * 3 + 2 * 24), (7, 10, 15))
        dr = ImageDraw.Draw(out)
        for i, (img, label) in enumerate(
            [(Image.open(mock).convert("RGB"), "MOCKUP (consoles.html)"), (Image.open(eng).convert("RGB"), "ENGINE (sc-client)"), (Image.fromarray(heat), f"DIFFERENCE ({total}% of pixels)")]
        ):
            y0 = i * (720 + 24)
            out.paste(img, (0, y0))
            if i < 2:
                dr.rectangle([0, y0 + 720, 1280, y0 + 744], fill=(12, 18, 26))
            dr.text((8, y0 + 4 if i == 2 else y0 + 724), label, fill=(232, 238, 246))
        out.save(d / f"{shot}-compare.png")
        print(f"{shot:20s} pixels {total:5.2f}%  missing  " + "  ".join(f"{k} {v}" for k, v in miss.items()))
        if a.max_pct is not None and total > a.max_pct:
            fails.append(f"{shot}: {total}% of pixels differ (limit {a.max_pct}%)")
        for k, v in miss.items():
            if a.max_missing is not None and v > a.max_missing:
                fails.append(f"{shot}: {k} is {v}% missing (limit {a.max_missing}%)")
    (d / "parity-report.json").write_text(json.dumps(report, indent=1) + "\n")
    if not report:
        sys.exit(f"{d}: no shot has both a mockup and an engine picture")
    if fails:
        sys.exit("console parity FAILED:\n  " + "\n  ".join(fails))


if __name__ == "__main__":
    main()
