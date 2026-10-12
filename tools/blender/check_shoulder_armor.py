"""Read the delivered shoulder UVs and embedded paint to reject the old pointed sliver.

Run in headless Blender for the shared GLB reader. Rays hit actual exported
triangles, including each LOD's separate baked atlas, on both sides of every livery.
"""
import argparse
import io
import json
from pathlib import Path
import struct
import sys

import numpy as np
from PIL import Image

sys.path.insert(0, str(Path(__file__).resolve().parent))
from hs_kit import ROOT, read_glb


def inspect(path):
    js, primitives = read_glb(str(path))
    data = path.read_bytes()
    binary = data[28 + struct.unpack_from("<I", data, 12)[0]:]
    surfaces = []
    for material in js["materials"]:
        primitive = primitives.get(material["name"])
        texture = material.get("pbrMetallicRoughness", {}).get("baseColorTexture")
        if primitive is None or texture is None:
            continue
        view = js["bufferViews"][js["images"][js["textures"][texture["index"]]["source"]]["bufferView"]]
        png = binary[view["byteOffset"]:view["byteOffset"] + view["byteLength"]]
        pixels = np.array(Image.open(io.BytesIO(png)).convert("RGB"))
        indices = np.array(primitive["idx"]).reshape(-1, 3)
        surfaces.append((np.array(primitive["pos"])[indices], np.array(primitive["uv"])[indices], pixels))

    def sample(x, z):
        hits = []
        for triangles, uvs, pixels in surfaces:
            a = triangles[:, 0, [0, 2]]
            ab = triangles[:, 1, [0, 2]] - a
            ac = triangles[:, 2, [0, 2]] - a
            ap = np.array([x, z]) - a
            det = ab[:, 0]*ac[:, 1] - ab[:, 1]*ac[:, 0]
            valid = np.abs(det) > 1e-7
            safe = np.where(valid, det, 1)
            u = (ap[:, 0]*ac[:, 1] - ap[:, 1]*ac[:, 0])/safe
            v = (ab[:, 0]*ap[:, 1] - ab[:, 1]*ap[:, 0])/safe
            for i in np.flatnonzero(valid & (u >= -1e-6) & (v >= -1e-6) & (u+v <= 1+1e-6)):
                weights = np.array([1-u[i]-v[i], u[i], v[i]])
                height = float(weights @ triangles[i, :, 1])
                uv = weights @ uvs[i]
                px = np.clip((uv * [pixels.shape[1], pixels.shape[0]]).astype(int), 0,
                             [pixels.shape[1]-1, pixels.shape[0]-1])
                hits.append((height, float(pixels[px[1], px[0]].mean())))
        assert hits, f"{path}: shoulder ray missed {(x, z)}"
        return max(hits, key=lambda hit: hit[0])[1]

    rows = []
    for side in (-1, 1):
        # Stay below the authored Y = 6.65 m panel seam, which remains dark.
        armor = [sample(side*x, z) for x in (11.75, 11.95) for z in (.2, .7, 1.2)]
        forward = [sample(side*x, z) for x in (11.4, 11.5) for z in (12.7, 13.0, 13.3)]
        channel = [sample(side*x, z) for x in (11.75, 11.95) for z in (3.5, 4.0, 4.5)]
        reference = [sample(side*x, -2.5) for x in (11.75, 11.95)]
        # LODs include the baked sun; compare with neighboring armor on that side.
        assert min(armor) > .8 * np.median(reference), f"{path}: pointed dark sliver survives on side {side}: {armor}"
        assert min(forward) > .8 * np.median(reference), f"{path}: forward triangular exposure survives on side {side}: {forward}"
        assert np.median(armor) - np.median(channel) > 35, f"{path}: remaining channel was painted over"
        rows.append({"side": side, "armor_min_rgb": round(min(armor), 2),
                     "forward_armor_min_rgb": round(min(forward), 2),
                     "channel_median_rgb": round(float(np.median(channel)), 2),
                     "reference_armor_rgb": round(float(np.median(reference)), 2), "samples": 20})
    return {"file": str(path), "sides": rows}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--file", type=Path, help="Inspect a candidate or regression fixture")
    args = parser.parse_args(sys.argv[sys.argv.index("--")+1:] if "--" in sys.argv else [])
    folder = Path(ROOT) / "assets/models/exteriors"
    paths = [args.file] if args.file else [base / ("tern" + suffix + ".glb")
            for base in (folder, folder / "liveries/cobalt", folder / "liveries/rescue")
            for suffix in ("", "_lod1", "_lod2")]
    print(json.dumps([inspect(path) for path in paths], indent=2), flush=True)


if __name__ == "__main__":
    main()
