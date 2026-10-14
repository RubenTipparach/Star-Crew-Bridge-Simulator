#!/usr/bin/env python3
"""The Tern's low poly model for the consoles' 3D pictures (openspec/changes/console-ship-model, design 2).

Reads assets/models/exteriors/tern_lod2.glb and writes data/ships/tern/console_model.json: its positions (ship axes,
+x port, +y dorsal, +z bow, metres, to the centimetre) and its triangles, each with one sRGB colour, the model's atlas
sampled at the triangle's middle times its vertex colour. The engine includes the file (sc-client console::kit) and
tools/mockups/inline.py copies it into docs/mockups/consoles.html, so both draw the same triangles.

It also holds the shield to the ship (console-ship-model 3; owner, 2026-10-14: "shield should envolop the ship
entirely"): every vertex of the full exterior model, assets/models/exteriors/tern.glb, lies inside the shield ellipsoid
of data/ships/tern/shields.json shrunk by 1 m.

Usage: python3 tools/ships/console_model.py [--check]
  --check  rebuild in memory and fail when the file differs from the model (a new model without a new file), or when
           the shield no longer holds the ship with 1 m to spare.
"""
import hashlib
import io
import json
import struct
import sys
from pathlib import Path

from PIL import Image

ROOT = Path(__file__).resolve().parents[2]
MODEL = "assets/models/exteriors/tern_lod2.glb"
FULL_MODEL = "assets/models/exteriors/tern.glb"
SHIELDS = ROOT / "data/ships/tern/shields.json"
SHIELD_SPARE_M = 1.0
OUT = ROOT / "data/ships/tern/console_model.json"

COMPONENTS = {"SCALAR": 1, "VEC2": 2, "VEC3": 3, "VEC4": 4}
FORMATS = {5121: ("B", 255.0), 5123: ("H", 65535.0), 5125: ("I", None), 5126: ("f", None)}


def read_glb(data):
    """The glb's JSON chunk and its binary chunk."""
    magic, _version, _length = struct.unpack_from("<III", data, 0)
    if magic != 0x46546C67:
        raise SystemExit(f"{MODEL}: not a glb")
    jlen, _ = struct.unpack_from("<II", data, 12)
    gltf = json.loads(data[20 : 20 + jlen])
    blen, _ = struct.unpack_from("<II", data, 20 + jlen)
    return gltf, data[28 + jlen : 28 + jlen + blen]


def accessor(gltf, binary, index):
    """An accessor's elements as lists of floats (normalised integers scaled to 0-1)."""
    a = gltf["accessors"][index]
    view = gltf["bufferViews"][a["bufferView"]]
    fmt, scale = FORMATS[a["componentType"]]
    n = COMPONENTS[a["type"]]
    size = struct.calcsize("<" + fmt)
    stride = view.get("byteStride", size * n)
    base = view.get("byteOffset", 0) + a.get("byteOffset", 0)
    out = []
    for i in range(a["count"]):
        vals = struct.unpack_from("<" + fmt * n, binary, base + i * stride)
        if scale and a.get("normalized"):
            vals = tuple(v / scale for v in vals)
        out.append(vals)
    return out


def build():
    data = (ROOT / MODEL).read_bytes()
    gltf, binary = read_glb(data)
    if len(gltf["meshes"]) != 1 or len(gltf["meshes"][0]["primitives"]) != 1:
        raise SystemExit(f"{MODEL}: expected one mesh with one primitive")
    node = gltf["nodes"][0]
    if any(k in node for k in ("translation", "rotation", "scale", "matrix")):
        raise SystemExit(f"{MODEL}: the node carries a transform; the tool expects the model in ship axes")
    prim = gltf["meshes"][0]["primitives"][0]
    at = prim["attributes"]
    pos = accessor(gltf, binary, at["POSITION"])
    uv = accessor(gltf, binary, at["TEXCOORD_0"])
    col = accessor(gltf, binary, at["COLOR_0"]) if "COLOR_0" in at else [(1.0, 1.0, 1.0, 1.0)] * len(pos)
    idx = [i[0] for i in accessor(gltf, binary, prim["indices"])]
    mat = gltf["materials"][prim["material"]]["pbrMetallicRoughness"]
    img = gltf["images"][gltf["textures"][mat["baseColorTexture"]["index"]]["source"]]
    view = gltf["bufferViews"][img["bufferView"]]
    start = view.get("byteOffset", 0)
    atlas = Image.open(io.BytesIO(binary[start : start + view["byteLength"]])).convert("RGB")
    w, h = atlas.size

    # Welded positions: the glb splits vertices at UV seams; the console needs only where they are.
    key = lambda p: tuple(round(c, 2) for c in p)
    welded, where = [], {}
    remap = []
    for p in pos:
        k = key(p)
        if k not in where:
            where[k] = len(welded)
            welded.append([round(c, 2) for c in p])
        remap.append(where[k])
    tris = []
    for t in range(0, len(idx), 3):
        a, b, c = idx[t : t + 3]
        u = sum(uv[i][0] for i in (a, b, c)) / 3.0
        v = sum(uv[i][1] for i in (a, b, c)) / 3.0
        px = atlas.getpixel((min(w - 1, max(0, int(u % 1.0 * w))), min(h - 1, max(0, int(v % 1.0 * h)))))
        vc = [sum(col[i][k] for i in (a, b, c)) / 3.0 for k in range(3)]
        rgb = [max(0, min(255, round(px[k] * vc[k]))) for k in range(3)]
        tris.append([remap[a], remap[b], remap[c], "#%02x%02x%02x" % tuple(rgb)])
    doc = {
        "schema": "starcrew.console-model/1",
        "_doc": "The Tern's low poly model for the consoles' 3D pictures (openspec/changes/console-ship-model): "
        "written by tools/ships/console_model.py from the source model, never by hand. Ship axes (+x port, +y "
        "dorsal, +z bow), metres; each triangle is three indices into positions_m and its colour.",
        "source": MODEL,
        "sha256": hashlib.sha256(data).hexdigest(),
        "positions_m": welded,
        "triangles": tris,
    }
    return json.dumps(doc, separators=(",", ":")) + "\n"


def shield_holds_ship():
    """The worst vertex of the full model, as a share of the shield shrunk by SHIELD_SPARE_M (at most 1 to pass)."""
    gltf, binary = read_glb((ROOT / FULL_MODEL).read_bytes())
    e = json.loads(SHIELDS.read_text())["shields"]["ellipsoid"]
    axes = [a - SHIELD_SPARE_M for a in e["axes_m"]]
    worst = 0.0
    for mesh in gltf["meshes"]:
        for prim in mesh["primitives"]:
            for p in accessor(gltf, binary, prim["attributes"]["POSITION"]):
                k = sum(((p[i] - e["centre_m"][i]) / axes[i]) ** 2 for i in range(3))
                worst = max(worst, k)
    return worst


def main():
    worst = shield_holds_ship()
    if worst > 1.0:
        print(f"the shield in {SHIELDS.relative_to(ROOT)} does not hold {FULL_MODEL} with {SHIELD_SPARE_M} m to spare "
              f"(worst vertex at {worst:.3f} of the shrunk ellipsoid)", file=sys.stderr)
        return 1
    print(f"the shield holds the ship with {SHIELD_SPARE_M} m to spare (worst vertex at {worst:.3f})")
    text = build()
    if "--check" in sys.argv[1:]:
        if not OUT.exists() or OUT.read_text() != text:
            print(f"{OUT.relative_to(ROOT)} is stale: run python3 tools/ships/console_model.py", file=sys.stderr)
            return 1
        print(f"{OUT.relative_to(ROOT)} is current")
        return 0
    OUT.write_text(text)
    d = json.loads(text)
    print(f"wrote {OUT.relative_to(ROOT)}: {len(d['positions_m'])} positions, {len(d['triangles'])} triangles")
    return 0


if __name__ == "__main__":
    sys.exit(main())
