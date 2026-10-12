"""Read delivered GLBs to guard nacelle direction, UV density and shared texture allocation.

Run with headless Blender so the shared artifact reader can be imported. This checks
real exported positions, UV loops and embedded images, not duplicated expected meshes.
"""
import argparse
import hashlib
import io
import json
from pathlib import Path
import struct
import sys

import bpy
import numpy as np
from PIL import Image

sys.path.insert(0, str(Path(__file__).resolve().parent))
from hs_kit import ROOT, read_glb


def inspect(path, config, ship):
    """Return measured facts from one exported full-detail model or fail on a regression."""
    js, primitives = read_glb(str(path))
    assert "tern_nacelle" in primitives, f"{path}: nacelles must have a dedicated material"
    primitive = primitives["tern_nacelle"]
    points, uvs = np.array(primitive["pos"]), np.array(primitive["uv"])
    rect = config["nacelle_atlas"]["regions"]["shell"]
    shell = (uvs[:, 0] >= rect[0] - 1e-5) & (uvs[:, 0] <= rect[2] + 1e-5)
    nc = ship["nacelle"]
    rows = []
    for sign in (-1, 1):
        chosen = shell & (points[:, 0] * sign > 0)
        pos, uv = points[chosen], uvs[chosen]
        zmin, zmax = pos[:, 2].min(), pos[:, 2].max()
        front = pos[np.abs(pos[:, 2] - zmax) < .001]
        rear = pos[np.abs(pos[:, 2] - zmin) < .001]
        fw, rw = float(np.ptp(front[:, 0])), float(np.ptp(rear[:, 0]))
        assert fw > 1.5 * rw, f"{path}: collector end points backward"
        assert abs(zmax - zmin - nc["length_m"]) < .001, "Nacelle length changed"
        assert abs((pos[:, 1].min() + pos[:, 1].max()) / 2 - nc["y_m"]) < .001, "Nacelle centreline moved"
        wide = pos[np.abs(np.abs(pos[:, 0]) - nc["x_m"]) > nc["half_width_m"] - .001]
        aft_taper, fore_taper = float(wide[:, 2].min() - zmin), float(zmax - wide[:, 2].max())
        assert aft_taper > fore_taper * 2, "Long taper must face aft"
        density = float(np.ptp(uv[:, 1]) * config["nacelle_atlas"]["size_px"][1] / (zmax - zmin))
        assert density >= 32, f"{path}: stretched shell UVs at {density:.2f} px/m"
        # Shared positions agree around the whole shell, allowing the single underside wrap.
        seams = {}
        for point, texcoord in zip(pos, uv):
            key = tuple(np.round(point, 4))
            if key in seams:
                delta = np.abs(texcoord - seams[key])
                assert delta[1] < 1e-5 and min(delta[0], abs(delta[0] - (rect[2] - rect[0]))) < 1e-5, "Discontinuous shell UVs"
            seams[key] = texcoord
        rows.append({"side": sign, "fore_width_m": round(fw, 4), "aft_width_m": round(rw, 4),
                     "aft_taper_m": round(aft_taper, 4), "fore_taper_m": round(fore_taper, 4),
                     "longitudinal_px_per_m": round(density, 4), "shared_vertices_checked": len(seams)})
    material = next(m for m in js["materials"] if m["name"] == "tern_nacelle")
    texture = js["textures"][material["pbrMetallicRoughness"]["baseColorTexture"]["index"]]
    view = js["bufferViews"][js["images"][texture["source"]]["bufferView"]]
    data = path.read_bytes()
    binary = data[28 + struct.unpack_from("<I", data, 12)[0]:]
    png = binary[view["byteOffset"]:view["byteOffset"] + view["byteLength"]]
    image = Image.open(io.BytesIO(png))
    assert list(image.size) == config["nacelle_atlas"]["size_px"], "Wrong dedicated texture allocation"
    assert not np.array(image.convert("RGBA"))[:, :, 3].any(), "Paint must not emit light"
    assert png == (Path(ROOT) / "assets/textures/tern_nacelle.png").read_bytes(), "Embedded paint differs from packaged material"
    return {"file": str(path.relative_to(ROOT)), "sides": rows,
            "texture_size_px": image.size, "texture_sha256": hashlib.sha256(png).hexdigest()}


def main():
    """Check every delivered livery, including byte-identical shared nacelle artwork."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--livery")
    parser.add_argument("--file", type=Path, help="Inspect a candidate or regression fixture")
    argv = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else ([] if bpy.app.background else sys.argv[1:])
    args = parser.parse_args(argv)
    config = json.loads((Path(ROOT) / "data/ships/exteriors.json").read_text())
    regions = list(config["nacelle_atlas"]["regions"].items())
    for i, (name, a) in enumerate(regions):
        assert 0 <= a[0] < a[2] <= 1 and 0 <= a[1] < a[3] <= 1, f"Invalid {name} atlas region"
        for other, b in regions[i + 1:]:
            assert min(a[2], b[2]) <= max(a[0], b[0]) or min(a[3], b[3]) <= max(a[1], b[1]), f"Overlapping {name}/{other} atlas regions"
    ship = next(s for s in config["ships"] if s["id"] == "tern")
    paths = [args.file] if args.file else [Path(ROOT) / "assets/models/exteriors" /
             ("tern.glb" if i == 0 else "liveries/" + livery["id"] + "/tern.glb")
             for i, livery in enumerate(config["liveries"]) if not args.livery or args.livery == livery["id"]]
    results = [inspect(path.resolve(), config, ship) for path in paths]
    assert len({r["texture_sha256"] for r in results}) == 1, "Liveries must share one neutral nacelle image"
    print(json.dumps(results, indent=2), flush=True)


if __name__ == "__main__":
    main()
