#!/usr/bin/env python3
"""Bake each bridge station's console into the images its screens show in the room.

The owner, 2026-10-05, on the bridge variants' consoles: "you should come up with better UI place
holders, keyboards etc ... for the panel textures I mean". openspec/changes/bridge-stations design
section 11.6 (task 1.7): a console screen in the room shows its own station's console, so the room
previews what a player gets when they sit. Until the console definitions exist as data
(data/consoles/<role>.json, task 2.2), the source is the mockup's console overlays: the headless
shots docs/screenshots/mockups/bridge-<station>-console.png that tools/mockups/shoot.mjs takes of
docs/mockups/bridge.html (1440 x 900).

Where the console canvas is in a shot: DERIVED from bridge.html's layoutConsole(), whose numbers
this tool reads out of the page (scale = min((width - 16) / 1280, (height - 140) / 720), the canvas
centred across and 8 px from the top), then CONFIRMED on every shot: the gap between the look band
and the panel grid (y 272-280 lp) and the margins either side of it (x 0-20 and 1260-1280 lp) must
be the plain console background. A page that moved its console fails here instead of giving a
shifted crop.
The canvas is first resampled to one pixel per logical pixel (lp, design section 8.1) by area
averaging, so every crop below is in the design's own units.

The budget HUD that every mockup shows (CLAUDE.md 11) sits over the canvas's bottom-right corner.
It is found in each shot (its left border column and its top), and the pixels under it are
rebuilt from the console's own grid: a panel's plain fill with its border lines redrawn, the rest
console background (whatever the panel showed there, the lower half of a footer button, is lost).
Panel rectangles come from bridge.html's panel(id, title, [c, r, w, h]) calls and the grid formula
of section 8.1.

What it writes, all 256 x 128 px RGBA (design 11.6: 2:1, two to a 256 px layer):
  assets/textures/screens/<station>.png        the main screen: the title band (y 0-32 lp) over
                                               the panel grid band (y 272-720 lp), 1280 x 480 lp
                                               area-averaged to 256 x 96 and letterboxed with the
                                               console background
  assets/textures/screens/<station>_upper.png  the upper pair: two 128 x 128 halves, each one of
                                               the station's own panels (STATIONS below), with half
                                               its gutter, fitted whole and letterboxed; a wall
                                               bank's first upper screen shows the left half, its
                                               second the right
  assets/textures/screens/screens.json         the manifest: every image's content rectangle (the
                                               part that is not letterbox), crop, bytes and
                                               sha256; the shared images (the wall panels'
                                               ui_screen_crew.png and keys_crew.png, reused, not
                                               remade); which station's images an unseated board
                                               shows; what the set costs as texture layers

Alpha is the emission mask (surface-materials): a screen is display all over, so the mask is 255
on every texel and the room's light never shades it. (keys_crew.png's mask marks only its lit
keys: the rest of a key panel is lit by the room.) Colours are reduced to PALETTE colours by
median cut without dithering, as the material layers are (tools/materials/postprocess.py), which
keeps the files small and the result deterministic: the same shots give the same bytes.

Documentation tooling (CLAUDE.md section 4): nothing the engine loads. Needs Pillow and numpy.

Usage: python3 tools/mockups/console_screens.py [--check]
  --check   writes nothing; fails when an image or the manifest differs from what the shots give
"""

import hashlib
import io
import json
import os
import re
import sys

import numpy as np
from PIL import Image

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
PAGE = os.path.join(ROOT, "docs", "mockups", "bridge.html")
SHOTS = os.path.join(ROOT, "docs", "screenshots", "mockups")
OUT = os.path.join(ROOT, "assets", "textures", "screens")
MANIFEST = os.path.join(OUT, "screens.json")
PANELS_JSON = os.path.join(ROOT, "data", "materials", "panels.json")
PANEL_UI = "assets/textures/panels"

CANVAS_LP = (1280, 720)          # the console canvas (design 8.1)
TITLE_LP = (0, 32)               # title band rows
BAND_LP = (272, 720)             # panel grid band and status strip rows
IMAGE_PX = (256, 128)            # a screen image (design 11.6)
HALF_PX = 128                    # one upper screen's half of an upper image
GUTTER_LP = 4                    # half the grid's 8 lp gutter, kept round a panel so its border shows
PALETTE = 64                     # colours per image after median cut

# Station id (the layout's) -> the shot's name and the two panels its upper screens show, chosen
# from section 11.6's list of what each station's screen is about and 11.6's "secondary displays":
# a plot or a schematic first, a status page second.
STATIONS = {
    "helm": {"shot": "helm", "upper": ("H3", "H1")},
    "tactical": {"shot": "tactical", "upper": ("T2", "T3")},
    "engineering": {"shot": "engineering", "upper": ("E2", "E3")},
    "science": {"shot": "science", "upper": ("S1", "S3")},
    "captain": {"shot": "captain", "upper": ("C1", "C2")},
    "comms": {"shot": "comms", "upper": ("M1", "M3")},
    "flight_ops": {"shot": "flight-ops", "upper": ("F4", "F1")},
}
# Boards with no console of their own (bridge_variants.json station ids, without the _p or _s of a
# side): the status boards repeat the captain's ship and stations pages; the curved helm's hooded
# viewer between its two seats shows tactical's plot; repeaters and spares show the wall panels'
# generic screen.
BOARDS = {"status": "captain", "helm_tactical": "tactical", "repeater": "generic", "spare": "generic"}


class Fail(Exception):
    pass


def read_page():
    with open(PAGE, encoding="utf-8") as f:
        src = f.read()
    m = re.search(r"cScale = Math\.min\(\(innerWidth - (\d+)\) / (\d+), \(innerHeight - (\d+)\) / (\d+)\);\s*"
                  r"cX = Math\.round\(\(innerWidth - (\d+) \* cScale\) / 2\); cY = (\d+);", src)
    if not m:
        raise Fail(f"{os.path.relpath(PAGE, ROOT)}: layoutConsole() is not the shape this tool reads")
    side, cw, below, ch, cw2, top = (int(g) for g in m.groups())
    if (cw, ch) != CANVAS_LP or cw2 != cw:
        raise Fail(f"layoutConsole() lays out a {cw} x {ch} canvas, not {CANVAS_LP}")
    g = re.search(r"const GRID = \(c, r, w, h\) => \(\{ x: (\d+) \+ (\d+) \* c, y: (\d+) \+ (\d+) \* r, "
                  r"w: (\d+) \* w - (\d+), h: (\d+) \* h - (\d+) \}\);", src)
    if not g:
        raise Fail(f"{os.path.relpath(PAGE, ROOT)}: the GRID formula is not the shape this tool reads")
    gx, gc, gy, gr, gw, gwg, gh, ghg = (int(v) for v in g.groups())
    panels = {}
    for pid, title, c, r, w, h in re.findall(r'panel\("([A-Z]\d+)", "([^"]*)", \[(\d+), (\d+), (\d+), (\d+)\]', src):
        c, r, w, h = int(c), int(r), int(w), int(h)
        panels[pid] = {"title": title, "grid": [c, r, w, h],
                       "lp": [gx + gc * c, gy + gr * r, gw * w - gwg, gh * h - ghg]}
    return {"side": side, "below": below, "top": top}, panels


def canvas_rect(layout, w, h):
    """The canvas in shot pixels: (x, y, scale), as layoutConsole() places it."""
    s = min((w - layout["side"]) / CANVAS_LP[0], (h - layout["below"]) / CANVAS_LP[1])
    return round((w - CANVAS_LP[0] * s) / 2), layout["top"], s


def near(a, b, tol):
    return float(np.abs(np.asarray(a, float) - np.asarray(b, float)).max()) <= tol


def find_hud(px):
    """The budget HUD's box in the shot (shipkit budgetHud: fixed 12 px from the bottom right, a
    #2b3540 border, 8 px padding, 6 px corners). Along the row 20 px above the shot's bottom, in the
    HUD's padding, walk left from its right edge over the HUD's dark fill to its left border; the
    top is where that border column ends, less the corner. Returns (x, y) or None."""
    h, w, _ = px.shape
    border = np.array([43, 53, 64], float)
    y_probe = h - 20
    x = w - 20
    while x > w // 2 and px[y_probe, x].astype(int).sum() < 30:
        x -= 1
    if x <= w // 2 or not near(px[y_probe, x], border, 18):
        return None
    y = y_probe
    while y > 0 and near(px[y - 1, x], border, 18):
        y -= 1
    return x, max(0, y - 7)


def lp_canvas(path, layout, panels, station_panels):
    """The shot's console canvas at one pixel per lp (float RGB), with the HUD's corner rebuilt."""
    im = Image.open(path).convert("RGB")
    w, h = im.size
    x0, y0, s = canvas_rect(layout, w, h)
    hud = find_hud(np.asarray(im, np.uint8))
    canvas = np.asarray(im.resize(CANVAS_LP, Image.Resampling.BOX,
                                  box=(x0, y0, x0 + CANVAS_LP[0] * s, y0 + CANVAS_LP[1] * s)), float)
    rel = os.path.relpath(path, ROOT)
    bg = canvas[300:660, 2:17].reshape(-1, 3).mean(axis=0)          # the left-hand margin, x 0-20 lp

    def plain(region):
        r = region.reshape(-1, 3)
        return near(r.mean(axis=0), bg, 3) and r.std(axis=0).max() < 3
    if not plain(canvas[300:660, 2:17]):
        raise Fail(f"{rel}: the left-hand margin (x 0-20 lp) is not plain background: "
                   "bridge.html's console is no longer where layoutConsole() says")
    if not plain(canvas[273:279, 24:1256]):
        raise Fail(f"{rel}: the gap above the panel grid (y 272-280 lp) is not plain background: "
                   "bridge.html's console is no longer where layoutConsole() says")
    if not plain(canvas[300:660, 1263:1278]):
        raise Fail(f"{rel}: the right-hand margin (x 1260-1280 lp) is not plain background")
    if hud:
        hx = max(0, int((hud[0] - x0) / s) - 1)
        hy = max(0, int((hud[1] - y0) / s) - 1)
        rects = [panels[p]["lp"] for p in station_panels]
        for y in range(hy, CANVAS_LP[1]):
            for x in range(hx, CANVAS_LP[0]):
                r = next((r for r in rects if r[0] <= x < r[0] + r[2] and r[1] <= y < r[1] + r[3]), None)
                if r is None:
                    canvas[y, x] = bg
                elif y >= r[1] + r[3] - 1 or x in (r[0], r[0] + r[2] - 1):
                    canvas[y, x] = canvas[r[1] + 40, r[0]]           # the border: the panel's left edge
                else:
                    canvas[y, x] = canvas[r[1] + 40, r[0] + 3]       # the fill: inside its 6 lp padding
        hud = [hx, hy]
    return canvas, bg, hud


def fit(canvas, rect, box_w, box_h):
    """A canvas rectangle (x, y, w, h in lp), area-averaged to fit box_w x box_h whole."""
    x, y, w, h = rect
    k = min(box_w / w, box_h / h)
    ow, oh = max(1, round(w * k)), max(1, round(h * k))
    img = Image.fromarray(np.clip(canvas[y:y + h, x:x + w] + 0.5, 0, 255).astype(np.uint8), "RGB")
    return np.asarray(img.resize((ow, oh), Image.Resampling.BOX), np.uint8)


def finish(rgb):
    """Median-cut to PALETTE colours (no dither) and add the emission mask (255: all display)."""
    q = Image.fromarray(rgb, "RGB").quantize(colors=PALETTE, method=Image.Quantize.MEDIANCUT,
                                             dither=Image.Dither.NONE).convert("RGB")
    out = Image.fromarray(np.dstack([np.asarray(q), np.full(rgb.shape[:2], 255, np.uint8)]), "RGBA")
    buf = io.BytesIO()
    out.save(buf, format="PNG", optimize=True)
    return buf.getvalue()


def bake_station(sid, cfg, layout, panels):
    shot = os.path.join(SHOTS, f"bridge-{cfg['shot']}-console.png")
    if not os.path.exists(shot):
        raise Fail(f"{os.path.relpath(shot, ROOT)} is missing: run node tools/mockups/shoot.mjs docs/mockups/bridge.html")
    prefix = cfg["upper"][0][0]
    own = sorted(p for p in panels if p[0] == prefix)
    for p in cfg["upper"]:
        if p not in panels:
            raise Fail(f"{sid}: panel {p} is not in {os.path.relpath(PAGE, ROOT)}")
    canvas, bg, hud = lp_canvas(shot, layout, panels, own)
    W, H = IMAGE_PX
    bg8 = np.clip(bg + 0.5, 0, 255).astype(np.uint8)
    # Main: the title band over the panel grid band, 1280 x 480 lp -> 256 x 96, letterboxed.
    strip = np.concatenate([canvas[TITLE_LP[0]:TITLE_LP[1]], canvas[BAND_LP[0]:BAND_LP[1]]], axis=0)
    body = fit(strip, (0, 0, strip.shape[1], strip.shape[0]), W, H)
    main = np.tile(bg8, (H, W, 1))
    oy, ox = (H - body.shape[0]) // 2, (W - body.shape[1]) // 2
    main[oy:oy + body.shape[0], ox:ox + body.shape[1]] = body
    main_rec = {"content_px": [ox, oy, body.shape[1], body.shape[0]],
                "crop_lp": [[0, TITLE_LP[0], CANVAS_LP[0], TITLE_LP[1] - TITLE_LP[0]],
                            [0, BAND_LP[0], CANVAS_LP[0], BAND_LP[1] - BAND_LP[0]]]}
    # Upper: two panels, each fitted whole into its 128 x 128 half.
    upper = np.tile(bg8, (H, W, 1))
    halves = []
    for i, pid in enumerate(cfg["upper"]):
        x, y, w, h = panels[pid]["lp"]
        rect = [max(0, x - GUTTER_LP), max(0, y - GUTTER_LP), w + 2 * GUTTER_LP, h + 2 * GUTTER_LP]
        rect[2] = min(rect[2], CANVAS_LP[0] - rect[0])
        rect[3] = min(rect[3], CANVAS_LP[1] - rect[1])
        img = fit(canvas, rect, HALF_PX, HALF_PX)
        hx, hy = i * HALF_PX + (HALF_PX - img.shape[1]) // 2, (HALF_PX - img.shape[0]) // 2
        upper[hy:hy + img.shape[0], hx:hx + img.shape[1]] = img
        halves.append({"panel": pid, "title": panels[pid]["title"], "crop_lp": rect,
                       "content_px": [hx, hy, img.shape[1], img.shape[0]]})
    return {
        "shot": os.path.relpath(shot, ROOT).replace(os.sep, "/"),
        "hud_masked_from_lp": hud,
        "main": dict(file=f"{sid}.png", **main_rec),
        "upper": {"file": f"{sid}_upper.png", "halves": halves},
    }, {f"{sid}.png": finish(main), f"{sid}_upper.png": finish(upper)}


def build():
    layout, panels = read_page()
    with open(PANELS_JSON, encoding="utf-8") as f:
        ui = json.load(f)["ui"]
    stations, files = {}, {}
    for sid, cfg in STATIONS.items():
        rec, imgs = bake_station(sid, cfg, layout, panels)
        stations[sid] = rec
        files.update(imgs)
    for sid, rec in stations.items():
        for part in ("main", "upper"):
            data = files[rec[part]["file"]]
            rec[part]["bytes"] = len(data)
            rec[part]["sha256"] = hashlib.sha256(data).hexdigest()
    shared = {}
    for key, stem, size_m in (("generic", "ui_screen_crew", ui["screen_m"]), ("keys", "keys_crew", ui["keys_m"])):
        path = os.path.join(ROOT, PANEL_UI, stem + ".png")
        with Image.open(path) as im:
            w, h = im.size
        with open(path, "rb") as f:
            data = f.read()
        shared[key] = {"file": f"{PANEL_UI}/{stem}.png", "size_m": size_m, "content_px": [0, 0, w, h],
                       "bytes": len(data), "sha256": hashlib.sha256(data).hexdigest()}
    # The trackpad of a key panel: one unlit key of keys_crew (row 1, column 3 of its 3 x 12),
    # stretched; keys_crew's keys fill 80 % of a pitch of 1/12 of its width and 1/3 of its height.
    kw, kh = shared["keys"]["content_px"][2:]
    pw, ph = kw / 12, kh / 3
    shared["keys"]["pad_px"] = [round(3 * pw + 0.1 * pw, 2), round(1 * ph + 0.1 * ph, 2), round(0.8 * pw, 2), round(0.8 * ph, 2)]
    images = 2 * len(stations)
    slots = images + 1 + 2          # each station's two, the keys (64 rows) and the generic screen (160 rows: two slots)
    layers = (slots + 1) // 2
    doc = {
        "schema": "starcrew.screens/1",
        "status": ("Built (2026-10-05) by tools/mockups/console_screens.py from the bridge mockup's console shots, "
                   "for openspec/changes/bridge-stations design 11.6 (task 1.7). docs/mockups/bridge-variants.html "
                   "draws them on the Blender consoles (tools/mockups/inline.py screens copies them in); no engine "
                   "code loads them yet."),
        "_rules": [
            "Written by tools/mockups/console_screens.py; never edit it or the images by hand. Re-shoot docs/mockups/bridge.html and rerun the tool.",
            "Every image is 256 x 128 px RGBA. content_px is [x, y, w, h] of the part that is not letterbox, from the image's top left; a screen shows its content fitted whole, and the rest of the screen is dark glass.",
            "Alpha is the emission mask: 255 is display, unshaded by the room. Station images are display all over. keys_crew.png marks only its lit keys; its other texels are lit by the room like any surface.",
            "A prop's screens row (assets/models/bridge/props.json) says what it shows: console (the station's main image), upper (half 0 or 1 of its upper image) or keys (a key panel of keys_crew blocks at their size_m, and a trackpad from pad_px).",
            "A station's images are its own; a board without a console of its own shows the station named in boards (its id without _p or _s), and generic means the wall panels' ui_screen_crew.png.",
        ],
        "source": {
            "page": "docs/mockups/bridge.html",
            "shots": "docs/screenshots/mockups/bridge-<shot>-console.png",
            "canvas": ("derived from bridge.html layoutConsole() (its numbers read from the page), confirmed on each shot "
                       "(the gap at y 272-280 lp and the margins at x 0-20 and 1260-1280 lp are plain background), resampled to "
                       "1 px per lp by area averaging"),
            "hud": "the budget HUD's corner is found in each shot and rebuilt from the panel grid (hud_masked_from_lp: x, y)",
        },
        "image_px": list(IMAGE_PX),
        "palette_colours": PALETTE,
        "stations": stations,
        "boards": BOARDS,
        "shared": shared,
        "cost": {
            "images": images + 2,
            "texture_layers_256": layers,
            "texture_bytes": layers * 256 * 256 * 4,
            "texture_bytes_with_mips": round(layers * 256 * 256 * 4 * 4 / 3),
            "note": ("Two 256 x 128 images to a 256 x 256 RGBA8 layer of the texture array (the keys take half a "
                     "layer, the 160-row generic screen a whole one)."),
        },
    }
    text = json.dumps(doc, indent=1, ensure_ascii=True) + "\n"
    return text, files


def main(argv):
    check = "--check" in argv
    try:
        text, files = build()
    except Fail as e:
        print(f"  FAIL {e}")
        return 1
    if check:
        stale = []
        for name, data in sorted(files.items()):
            p = os.path.join(OUT, name)
            if not os.path.exists(p) or open(p, "rb").read() != data:
                stale.append(os.path.relpath(p, ROOT))
        if not os.path.exists(MANIFEST) or open(MANIFEST, encoding="utf-8").read() != text:
            stale.append(os.path.relpath(MANIFEST, ROOT))
        if stale:
            print("  FAIL stale or different (run python3 tools/mockups/console_screens.py):\n    " + "\n    ".join(stale))
            return 1
        print(f"  {len(files)} screen images and screens.json match the shots")
        return 0
    os.makedirs(OUT, exist_ok=True)
    for name, data in sorted(files.items()):
        with open(os.path.join(OUT, name), "wb") as f:
            f.write(data)
    with open(MANIFEST, "w", encoding="utf-8", newline="\n") as f:
        f.write(text)
    total = sum(len(d) for d in files.values())
    print(f"  wrote {len(files)} images ({total:,} bytes) and {os.path.relpath(MANIFEST, ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
