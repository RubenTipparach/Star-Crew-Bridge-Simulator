"""Star Crew's repair covers: the service cover a player unscrews in 3D before a repair
(openspec/changes/repairs-on-deck, design 3b), modelled the hard-surface way and baked to a texture.

It owns assets/textures/repairs/cover_<finish>.png and assets/textures/repairs/covers.json (each
file's size and sha256, and the screw holes as fractions of the plate, which the 3D screws are placed
from). It lives beside build_wall_panels.py and imports it rather than copying it (CLAUDE.md 6.1): a
cover is a wall part, so it takes the panels' scene, light, materials, wear, render and
post-process, and the finish's colours from data/materials/panels.json. What is particular to the
cover (its size, rim, hazard band, holes and stencils) is data/materials/repair_covers.json.

Run, with Pillow and numpy beside bpy (pip install bpy==4.5.4 pillow numpy):
  <python with the bpy module> tools/blender/build_repair_covers.py [--samples N]

The cover, in panel space (x right, y up, z out of the face, metres), centred on (0, 1):
  a dark backing the cover's size, so the chamfered corners read as cut;
  the plate, 10 mm proud with 45 degree corners, in the finish's second paint;
  a raised rim round it and a hazard band inside the rim;
  a countersunk hole at each corner, inset_m from both edges: a dark bore and a bright chamfer ring;
  the stencils, SERVICE across the middle and a small note under it.
"""

import argparse
import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import build_wall_panels as wp  # noqa: E402  imports bpy first, then the kit and the post-process
import PIL  # noqa: E402

ROOT = wp.ROOT
DATA = os.path.join(ROOT, "data", "materials", "repair_covers.json")
OUT = os.path.join(ROOT, "assets", "textures", "repairs")
RAW = os.path.join(ROOT, "tools", "materials", "raw", "repairs")
SCHEMA = "starcrew.repair-covers/1"


def load():
    """Read and validate repair_covers.json: every key checked, unknown keys stop the build."""
    with open(DATA, encoding="utf-8") as f:
        d = json.load(f)
    if d.get("schema") != SCHEMA:
        raise SystemExit(f"[covers] {DATA}: schema must be {SCHEMA!r}")
    wp.keys_exact(d, {"schema", "status", "_rules", "size_m", "out_px", "supersample", "colours", "finishes",
                      "corner_m", "rim_w_m", "hazard_w_m", "screw", "text"}, "repair_covers")
    wp.keys_exact(d["screw"], {"inset_m", "hole_r_m", "countersink_r_m"}, "repair_covers.screw")
    wp.keys_exact(d["text"], {"label", "label_m", "note", "note_m"}, "repair_covers.text")
    (w, h), (pw, ph) = d["size_m"], d["out_px"]
    if abs(pw / w - ph / h) > 0.5:
        raise SystemExit(f"[covers] out_px {pw}x{ph} is not at one density over size_m {w}x{h}")
    if not 0 < d["screw"]["countersink_r_m"] < d["screw"]["inset_m"]:
        raise SystemExit("[covers] screw.countersink_r_m must be above 0 and inside inset_m")
    return d


def build(P, d):
    """Model one cover (module docstring)."""
    w, h = d["size_m"]
    x0, x1, y0, y1 = -w / 2, w / 2, 1.0 - h / 2, 1.0 + h / 2
    c, rim, band, s = d["corner_m"], d["rim_w_m"], d["hazard_w_m"], d["screw"]
    P.box("backing", (x0 - 0.05, y0 - 0.05, -0.06), (x1 + 0.05, y1 + 0.05, -0.02), "machinery")
    holes = [(x, y) for y in (y1 - s["inset_m"], y0 + s["inset_m"]) for x in (x0 + s["inset_m"], x1 - s["inset_m"])]
    plate = wp.cplate(P, "plate", x0, y0, x1, y1, c, -0.02, 0.0, "paint2", bevel=0.004)
    P.cut(plate, "holes", [P.cyl("bore", "z", hp, s["hole_r_m"], -0.015, 0.05, "machinery", sides=24, smooth=False)
                           for hp in holes])
    rim_ob = wp.frame_ring(P, "rim", x0 + c * 0.6, y0 + c * 0.6, x1 - c * 0.6, y1 - c * 0.6, rim, -0.004, 0.004,
                           "paint2", bevel=0.002)
    i = c * 0.6 + rim + 0.006
    band_ob = wp.frame_ring(P, "hazard", x0 + i, y0 + i, x1 - i, y1 - i, band, -0.004, 0.002, "hazard")
    # The band and the rim stop short of the holes: each hole sits in a clear pad.
    pads = [P.cyl("pad", "z", hp, s["countersink_r_m"] + 0.008, -0.02, 0.05, "paint2", sides=24, smooth=False)
            for hp in holes]
    P.cut(band_ob, "band_pads", pads)
    P.cut(rim_ob, "rim_pads", [P.cyl("pad2", "z", hp, s["countersink_r_m"] + 0.008, -0.02, 0.05, "paint2", sides=24,
                                     smooth=False) for hp in holes])
    for hp in holes:
        wp.ring(P, "countersink", hp[0], hp[1], s["countersink_r_m"], s["hole_r_m"], -0.006, 0.0008, "trim",
                sides=28, bevel=0.002)
    t = d["text"]
    P.text(t["label"], 0.0, 1.0 - t["label_m"] * 0.35, t["label_m"], z=0.0, align="CENTER")
    P.text(t["note"], 0.0, 1.0 - t["label_m"] * 0.35 - t["note_m"] * 1.9, t["note_m"], z=0.0, align="CENTER")
    return [((hx - x0) / w, (y1 - hy) / h) for hx, hy in holes]   # u from the left, v from the top


def render(d, fn, samples):
    """Beauty and mask renders of one finish's cover, at supersample x out_px."""
    D = wp.load_panels()
    R = D["render"]
    F = D["finishes"][fn]
    wp.set_roles()
    k = d["supersample"]
    res = (d["out_px"][0] * k, d["out_px"][1] * k)
    holes = None
    out = {}
    for mask in (False, True):
        wp.reset(R, R["mask_samples"] if mask else (samples or R["samples"]), res, mask=mask)
        wp.make_materials(F, False, streaky=True)
        P = wp.Panel(f"cover_{fn}")
        holes = build(P, d)
        wp.camera(0.0, 1.0, d["size_m"][0], res)
        wp.check_roles(f"cover_{fn}")
        if mask:
            wp.mask_materials()
        path = os.path.join(RAW, f"cover_{fn}.{'mask' if mask else 'beauty'}.exr")
        os.makedirs(RAW, exist_ok=True)
        wp.render_to(path)
        out["mask" if mask else "beauty"] = path
    return out, holes


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--samples", type=int, default=0, help="override render.samples for a quick look")
    a = ap.parse_args()
    d = load()
    covers = {}
    for fn in d["finishes"]:
        raw, holes = render(d, fn, a.samples)
        img = wp.finish_image(wp.read_exr(raw["beauty"]), wp.read_exr(raw["mask"]), d["out_px"][0], d["colours"])
        path = os.path.join(OUT, f"cover_{fn}.png")
        wp.mm.save_png(wp.Image.fromarray(img, "RGBA"), path)
        covers[fn] = {"file": os.path.relpath(path, ROOT), "px": d["out_px"], "bytes": os.path.getsize(path),
                      "sha256": wp.sha256(path), "holes_uv": [[round(u, 5), round(v, 5)] for u, v in holes]}
        print(f"[covers] {covers[fn]['file']}: {d['out_px'][0]}x{d['out_px'][1]}, holes {covers[fn]['holes_uv']}")
    man = {"schema": "starcrew.repair-covers-built/1", "source": os.path.relpath(DATA, ROOT),
           "built_by": os.path.relpath(__file__, ROOT), "size_m": d["size_m"],
           "versions": {"blender": wp.bpy.app.version_string, "pillow": PIL.__version__, "numpy": wp.np.__version__},
           "covers": covers}
    with open(os.path.join(OUT, "covers.json"), "w", encoding="utf-8") as f:
        json.dump(man, f, indent=2)
        f.write("\n")


if __name__ == "__main__":
    main()
