"""Check enhanced livery colors against the retained 3D bake, without editing artwork."""
import hashlib
import json
from pathlib import Path

import numpy as np
from PIL import Image, ImageFilter

ROOT = Path(__file__).resolve().parents[2]
SOURCES = ROOT / "tools/materials/sources"


def check_seam_joins(packed, projection):
    """Read both UV profiles where projected seams cross real mesh edges."""
    probes = json.loads((SOURCES / "tern-seam-probes.json").read_text())
    target = np.asarray(projection["seam_srgb"])
    rgb = packed[:, :, :3].astype(float) / 255
    offsets = np.linspace(-probes["profile_half_length_z_m"],
                          probes["profile_half_length_z_m"], 81)
    errors, separations = [], []
    for probe in probes["probes"]:
        centers, tolerances = [], []
        for profile in probe["profiles_uv"]:
            uv = np.asarray(profile)
            x = np.clip((uv[:, 0] * rgb.shape[1]).astype(int), 0, rgb.shape[1]-1)
            y = np.clip(((1-uv[:, 1]) * rgb.shape[0]).astype(int), 0, rgb.shape[0]-1)
            difference = np.max(abs(rgb[y, x] - target), axis=1)
            error = float(difference.min())
            errors.append(error)
            assert error < .09, f"Missing seam at {probe['position_m']}: RGB error {error:.3f}"
            candidates = np.flatnonzero(difference <= error + .012)
            # A nearby pipe can share the seam color. Locate a contiguous stroke;
            # averaging disjoint strokes invents a center in the white space.
            strokes = np.split(candidates, np.flatnonzero(np.diff(candidates) > 1) + 1)
            stroke = min(strokes, key=lambda group: abs(offsets[group].mean()))
            center = float(offsets[stroke].mean())
            pixels = max(abs((uv[-1]-uv[0]) * [rgb.shape[1], rgb.shape[0]]))
            tolerance = max(.16, 1.4 * (offsets[-1]-offsets[0]) / max(pixels, 1))
            assert abs(center) <= tolerance, f"Seam shifted from {probe['position_m']} by {center:.3f} m"
            centers.append(center)
            tolerances.append(tolerance)
        separation = abs(centers[0]-centers[1])
        assert separation <= sum(tolerances), f"Seam jumps across edge at {probe['position_m']}"
        separations.append(separation)
    return {"shared_edge_crossings": len(separations), "receiving_face_profiles": len(errors),
            "max_seam_rgb_error": max(errors), "max_join_offset_m": max(separations),
            "position_tolerance": "1.4 texels per receiving face, minimum 0.16 m"}


def main():
    mask_path = SOURCES / "tern-projected-masks.png"
    mask = Image.open(mask_path).convert("RGB")
    report = {"mask_sha256": hashlib.sha256(mask_path.read_bytes()).hexdigest(),
              "edge_clearance_texture_px": 4, "liveries": {}}
    for livery, texture in [("cyan", "tern_hull_cobalt"), ("copper", "tern_hull"),
                             ("rescue", "tern_hull_rescue")]:
        source_path = SOURCES / "tern-clean-finish.png"
        source = Image.open(source_path).convert("RGB")
        packed = np.asarray(Image.open(ROOT / "assets/textures" / f"{texture}.png"))
        rgb = packed[:, :, :3].astype(float)
        regions = np.asarray(mask.resize((packed.shape[1], packed.shape[0]), Image.Resampling.NEAREST))
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
        rendered = np.asarray(Image.open(ROOT / "tools/materials/raw" / f"{texture}_albedo.png").convert("RGB"))
        assert np.array_equal(packed[:, :, :3], rendered), f"{livery}: Material Maker RGB changed"
        assert not packed[:, :, 3].any(), f"{livery}: hull paint emits light"
        report["liveries"][livery] = {
            "source_sha256": hashlib.sha256(source_path.read_bytes()).hexdigest(),
            "source_px": source.width, "stripe_interior_pixels": int(inside.sum()),
            "stripe_color_coverage": coverage, "material_maker_rgb_preserved": True,
            "hull_emission_pixels": 0,
            "seam_joins": check_seam_joins(packed, json.loads((ROOT / "data/ships/tern/hull_paint_projection.json").read_text())),
        }
    (SOURCES / "tern-enhancement-validation.json").write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps(report, indent=2))


if __name__ == "__main__":
    main()
