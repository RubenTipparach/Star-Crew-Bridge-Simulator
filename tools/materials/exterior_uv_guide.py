"""Rasterize actual exported UV polygons as the owner's wireframe painting guide.

This makes no surface artwork. The PNG is a geometry template for image painting; the JSON
comes directly from the Blender mesh's UV loop data and remains its single source.
"""
import json
from pathlib import Path
from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parents[2]


def main():
    """Write an opaque UV guide and a separate island coverage mask at the exported size."""
    folder = ROOT / "tools/materials/sources"
    data = json.loads((folder / "tern-uv-layout.json").read_text())
    size = data["size_px"]
    image = Image.new("RGB", (size, size), (12, 16, 23))
    mask = Image.new("L", (size, size), 0)
    draw, md = ImageDraw.Draw(image), ImageDraw.Draw(mask)
    palette = {"hull": (172, 187, 203), "fittings": (145, 183, 193), "shoulder": (145, 183, 193), "crown": (203, 178, 147)}
    for face in data["faces"]:
        points = [(round(u * (size - 1)), round((1 - v) * (size - 1))) for u, v in face["uv"]]
        draw.polygon(points, fill=palette[face["region"]])
        md.polygon(points, fill=255)
    # Preserve all actual mesh edges, including internal triangulation, as thin painting guides.
    for face in data["faces"]:
        points = [(round(u * (size - 1)), round((1 - v) * (size - 1))) for u, v in face["uv"]]
        draw.line(points + points[:1], fill=(45, 60, 73), width=1)
    image.save(folder / "tern-uv-wireframe.png")
    mask.save(folder / "tern-uv-mask.png")
    print("Exported actual UV wireframe and island mask", size, "px")


if __name__ == "__main__":
    main()
