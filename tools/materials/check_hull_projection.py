"""Check enhanced livery colors against the retained 3D bake, without editing artwork."""
import hashlib
import json
from pathlib import Path

import numpy as np
from PIL import Image, ImageFilter

ROOT = Path(__file__).resolve().parents[2]
SOURCES = ROOT / "tools/materials/sources"


def check_flat_bands(mask, projection):
    """Read the baked band edges along real roof traces, including the centerline."""
    traces = json.loads((SOURCES / "tern-flat-band-probes.json").read_text())["traces"]
    pixels = np.asarray(mask)
    bounds = {}
    for trace in traces:
        samples = trace["samples"]
        uv = np.asarray([s["uv"] for s in samples])
        x = np.clip((uv[:, 0]*pixels.shape[1]).astype(int), 0, pixels.shape[1]-1)
        y = np.clip(((1-uv[:, 1])*pixels.shape[0]).astype(int), 0, pixels.shape[0]-1)
        colored = pixels[y, x, 0] > 127
        assert colored.any(), f"Band missing from actual roof at X={trace['x_m']} m"
        # Remove the height term so a roof step cannot be mistaken for an arrow tip.
        stations = np.asarray([s["z_m"] + s["y_m"]*projection["stripe_axis"][1]
                               for s in samples])[colored]
        edges = [float(stations.min()), float(stations.max())]
        expected = np.asarray(trace["interval_m"]) + projection["stripe_flat_half_width_m"]*projection["stripe_axis"][0]
        assert max(abs(np.asarray(edges)-expected)) < .18, f"Non-flat band at X={trace['x_m']} m: {edges}"
        bounds.setdefault(tuple(trace["interval_m"]), []).append(edges)
    spread = max(float(np.ptp(values, axis=0).max()) for values in bounds.values())
    assert spread < .18, f"Central paint join has an arrow tip: edge spread {spread:.3f} m"
    return {"roof_traces": len(traces), "central_width_m": 2*projection["stripe_flat_half_width_m"],
            "max_edge_spread_m": spread, "edge_tolerance_m": .18}


def check_seam_joins(packed):
    """Check all geometric crossings, and textured cores where they cross armor.

    Exposed pipes no longer carry painted seam strokes. Classify armor using the
    retained seam-free painting, independently of the enhanced joint's color.
    """
    probes = json.loads((SOURCES / "tern-seam-probes.json").read_text())
    rgb = packed[:, :, :3].astype(float) / 255
    mask = np.asarray(Image.open(SOURCES / "tern-projected-masks.png").convert("RGB")) / 255
    original = np.asarray(Image.open(SOURCES / "tern-seamless-finish.png").convert("RGB")) / 255
    offsets = np.linspace(-probes["profile_half_length_z_m"],
                          probes["profile_half_length_z_m"], 81)

    def sample(image, uv):
        x = np.clip((uv[:, 0]*image.shape[1]).astype(int), 0, image.shape[1]-1)
        y = np.clip(((1-uv[:, 1])*image.shape[0]).astype(int), 0, image.shape[0]-1)
        return image[y, x]

    def stroke_center(candidates):
        assert len(candidates), "Projected armor joint has no recessed core"
        strokes = np.split(candidates, np.flatnonzero(np.diff(candidates) > 1) + 1)
        stroke = min(strokes, key=lambda group: abs(offsets[group].mean()))
        return float(offsets[stroke].mean())

    mask_separations, armor_offsets, core_colors = [], [], []
    for probe in probes["probes"]:
        centers, tolerances = [], []
        for profile in probe["profiles_uv"]:
            uv = np.asarray(profile)
            pixels = max(abs((uv[-1]-uv[0]) * [rgb.shape[1], rgb.shape[0]]))
            tolerance = max(.16, 1.4*(offsets[-1]-offsets[0])/max(pixels, 1))
            center = stroke_center(np.flatnonzero(sample(mask, uv)[:, 1] > .5))
            assert abs(center) <= tolerance, f"Projected seam shifted at {probe['position_m']}"
            centers.append(center)
            tolerances.append(tolerance)
            armor = sample(original, uv).min(axis=1)
            # At a mechanical boundary there need not be a seam on both sides.
            # Broad armor profiles must still contain the actual shaded recess.
            if armor[40] < .75 or (armor > .75).mean() < .8:
                continue
            colors = sample(rgb, uv)
            luminance = colors @ np.asarray([.2126, .7152, .0722])
            neutral = colors.max(axis=1)-colors.min(axis=1) < .13
            core = np.flatnonzero(neutral & (luminance >= .23) & (luminance <= .52))
            painted_center = stroke_center(core)
            assert abs(painted_center) <= tolerance, (
                f"Enhanced joint shifted from {probe['position_m']} by {painted_center:.3f} m")
            armor_offsets.append(abs(painted_center))
            core_colors.append(colors[np.argmin(abs(offsets-painted_center))])
        separation = abs(centers[0]-centers[1])
        assert separation <= sum(tolerances), f"Seam jumps across edge at {probe['position_m']}"
        mask_separations.append(separation)
    assert len(armor_offsets) >= 20, "Too few finished armor joints checked"
    variation = float(np.ptp(np.asarray(core_colors), axis=0).max()*255)
    assert variation > 10, "Joint cores have been replaced with a uniform color"
    return {"shared_edge_crossings": len(mask_separations),
            "projected_face_profiles": 2*len(mask_separations),
            "finished_armor_profiles": len(armor_offsets),
            "max_projected_join_offset_m": max(mask_separations),
            "max_finished_core_offset_m": max(armor_offsets),
            "core_channel_range": variation,
            "position_tolerance": "1.4 texels per receiving face, minimum 0.16 m"}


def check_joint_finish(packed):
    """Reject constant seam fills on independently classified ivory armor."""
    size = (packed.shape[1], packed.shape[0])
    mask = np.asarray(Image.open(SOURCES / "tern-projected-masks.png").convert("RGB")
                      .resize(size, Image.Resampling.BILINEAR))
    original = np.asarray(Image.open(SOURCES / "tern-seamless-finish.png").convert("RGB")
                          .resize(size, Image.Resampling.BILINEAR))
    cores = (mask[:, :, 1] > 210) & (mask[:, :, 2] > 230) & (original.min(axis=2) > 220)
    assert cores.sum() >= 100, "Too few armor seam texels to judge their finish"
    spread = float(np.ptp(np.percentile(packed[:, :, :3][cores], [5, 95], axis=0), axis=0).max())
    assert spread > 8, f"Joint material is a uniform line ({spread}/255 range)"
    return {"armor_core_texels": int(cores.sum()), "core_5_to_95_percentile_range": spread}


def main():
    mask_path = SOURCES / "tern-projected-masks.png"
    mask = Image.open(mask_path).convert("RGB")
    projection = json.loads((ROOT / "data/ships/tern/hull_paint_projection.json").read_text())
    report = {"mask_sha256": hashlib.sha256(mask_path.read_bytes()).hexdigest(),
              "edge_clearance_texture_px": 4, "flat_bands": check_flat_bands(mask, projection),
              "liveries": {}}
    for livery, texture in [("cyan", "tern_hull_cobalt"), ("copper", "tern_hull"),
                             ("rescue", "tern_hull_rescue")]:
        source_path = SOURCES / "tern-flat-painted-cyan.png"
        source = Image.open(source_path).convert("RGB")
        packed = np.asarray(Image.open(ROOT / "assets/textures" / f"{texture}.png"))
        rgb = packed[:, :, :3].astype(float)
        regions = np.asarray(mask.resize((packed.shape[1], packed.shape[0]), Image.Resampling.NEAREST))
        band = (regions[:, :, 0] > 230) & (regions[:, :, 2] > 230)
        # Ignore filtered boundary texels; visual inspection checks the exact joins.
        inside = np.asarray(Image.fromarray(band.astype("uint8") * 255)
                            .filter(ImageFilter.MinFilter(9))) > 0
        neutral = np.asarray(Image.open(SOURCES / "tern-clean-finish.png").convert("RGB")
                             .resize((packed.shape[1], packed.shape[0]), Image.Resampling.BILINEAR))
        # Paint belongs on armor; panel joints and grey machinery keep their relief.
        inside &= (neutral.min(axis=2) > 200) & (regions[:, :, 1] < 10)
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
        finish_range = float(np.ptp(np.percentile(rgb[inside], [5, 95], axis=0), axis=0).max())
        assert finish_range > 8, f"{livery}: paint finish is a uniform fill ({finish_range}/255 range)"
        rendered = np.asarray(Image.open(ROOT / "tools/materials/raw" / f"{texture}_albedo.png").convert("RGB"))
        assert np.array_equal(packed[:, :, :3], rendered), f"{livery}: Material Maker RGB changed"
        assert not packed[:, :, 3].any(), f"{livery}: hull paint emits light"
        report["liveries"][livery] = {
            "source_sha256": hashlib.sha256(source_path.read_bytes()).hexdigest(),
            "source_px": source.width,
            "joint_source_sha256": hashlib.sha256((SOURCES / "tern-integrated-joints.png").read_bytes()).hexdigest(),
            "stripe_interior_pixels": int(inside.sum()),
            "stripe_color_coverage": coverage, "material_maker_rgb_preserved": True,
            "paint_finish_5_to_95_percentile_range": finish_range,
            "hull_emission_pixels": 0,
            "seam_joins": check_seam_joins(packed),
            "joint_finish": check_joint_finish(packed),
        }
    (SOURCES / "tern-enhancement-validation.json").write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps(report, indent=2))


if __name__ == "__main__":
    main()
