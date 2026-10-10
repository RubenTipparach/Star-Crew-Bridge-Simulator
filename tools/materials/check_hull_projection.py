"""Check enhanced livery colors against the retained 3D bake, without editing artwork."""
import hashlib
import json
from pathlib import Path

import numpy as np
from PIL import Image, ImageFilter

ROOT = Path(__file__).resolve().parents[2]
SOURCES = ROOT / "tools/materials/sources"


def main():
    mask_path = SOURCES / "tern-projected-masks.png"
    mask = Image.open(mask_path).convert("RGB")
    report = {"mask_sha256": hashlib.sha256(mask_path.read_bytes()).hexdigest(),
              "edge_clearance_source_px": 4, "liveries": {}}
    for livery, texture in [("cyan", "tern_hull_cobalt"), ("copper", "tern_hull"),
                             ("rescue", "tern_hull_rescue")]:
        source_path = SOURCES / f"tern-projected-{livery}.png"
        source = Image.open(source_path).convert("RGB")
        rgb = np.asarray(source, dtype=float)
        regions = np.asarray(mask.resize(source.size, Image.Resampling.NEAREST))
        band = (regions[:, :, 0] > 230) & (regions[:, :, 2] > 230)
        # Ignore filtered boundary texels; visual inspection checks the exact joins.
        inside = np.asarray(Image.fromarray(band.astype("uint8") * 255)
                            .filter(ImageFilter.MinFilter(9))) > 0
        r, g, b = rgb.transpose(2, 0, 1)
        if livery == "cyan":
            colored = (g - r > 50) & (b - r > 60)
        elif livery == "copper":
            colored = (r - g > 45) & (g - b > 12)
        else:
            colored = (r - b > 80) & (g - b > 70)
        assert inside.any(), "No projected stripe pixels sampled"
        coverage = float(colored[inside].mean())
        assert coverage > .98, f"{livery}: only {coverage:.1%} of the projected band retains paint"
        packed = np.asarray(Image.open(ROOT / "assets/textures" / f"{texture}.png"))
        rendered = np.asarray(Image.open(ROOT / "tools/materials/raw" / f"{texture}_albedo.png").convert("RGB"))
        assert np.array_equal(packed[:, :, :3], rendered), f"{livery}: Material Maker RGB changed"
        assert not packed[:, :, 3].any(), f"{livery}: hull paint emits light"
        report["liveries"][livery] = {
            "source_sha256": hashlib.sha256(source_path.read_bytes()).hexdigest(),
            "source_px": source.width, "stripe_interior_pixels": int(inside.sum()),
            "stripe_color_coverage": coverage, "material_maker_rgb_preserved": True,
            "hull_emission_pixels": 0,
        }
    (SOURCES / "tern-enhancement-validation.json").write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps(report, indent=2))


if __name__ == "__main__":
    main()
