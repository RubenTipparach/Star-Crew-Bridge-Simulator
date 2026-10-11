"""Author fleet registration and package fitted surface artwork, separate from tiling layers.

The PNG is a decal source with ordinary opacity, not an emission-mask texture. The fleet builder
packages it in its GLBs. Drawn typography is original; the font is the repository's own font asset.
The generated surface artwork is rendered by Material Maker before its emission-mask packaging.
"""
from pathlib import Path
from io import BytesIO
import json

from PIL import Image, ImageDraw, ImageFont
import numpy as np
from fontTools.ttLib import TTFont

ROOT = Path(__file__).resolve().parents[2]


def pack_surfaces(name, size, window_emission=True):
    """Package actual Material Maker RGB unchanged, with cyan window emission in alpha."""
    source = ROOT / "tools/materials/raw" / (name + "_albedo.png")
    image = Image.open(source).convert("RGBA")
    assert image.size == (size, size), f"{name}: expected {size} px"
    pixels = np.array(image)
    rgb = pixels[:, :, :3].astype(np.float32)
    r, g, b = rgb[:, :, 0], rgb[:, :, 1], rgb[:, :, 2]
    pixels[:, :, 3] = (((b > r * 1.4) & (g > r * 1.2) & (b > 150) & (g > 100)) * 255
                       if window_emission else 0)
    target = ROOT / "assets/textures" / (name + ".png")
    Image.fromarray(pixels).save(target, compress_level=9)
    print(target.relative_to(ROOT))


def main():
    """Write the deterministic 512 x 128 px registration sheet."""
    fonts = sorted((ROOT / "assets/fonts").glob("**/*.woff2"))
    if not fonts:
        raise SystemExit("A committed font is required for fleet decals")
    source = TTFont(str(fonts[0]))
    source.flavor = None
    font_bytes = BytesIO()
    source.save(font_bytes)
    image = Image.new("RGBA", (512, 128), (14, 24, 34, 255))
    draw = ImageDraw.Draw(image)
    for x, width, name, number in [(0, 256, "SCS TERN", "SC-084"),
                                   (256, 128, "OSPREY", "SC-054"),
                                   (384, 128, "SHRIKE", "R-050")]:
        font = ImageFont.truetype(BytesIO(font_bytes.getvalue()), 32 if width > 128 else 21)
        small = ImageFont.truetype(BytesIO(font_bytes.getvalue()), 17)
        draw.rectangle((x + 8, 10, x + width - 9, 13), fill=(200, 215, 226, 255))
        draw.text((x + 12, 27), name, font=font, fill=(220, 234, 243, 255))
        draw.text((x + 12, 74), number, font=small, fill=(153, 177, 194, 255))
        draw.polygon([(x + width - 24, 92), (x + width - 12, 108), (x + width - 36, 108)], fill=(220, 234, 243, 255))
    target = ROOT / "assets/textures/exterior_decals.png"
    image.save(target, optimize=False, compress_level=9)
    print(target.relative_to(ROOT))
    pack_surfaces("exterior_surfaces", 2048)
    config = json.loads((ROOT / "data/ships/exteriors.json").read_text())
    for livery in config["liveries"]:
        pack_surfaces(livery["texture"], config["hero_atlas"]["px"], window_emission=False)


if __name__ == "__main__":
    main()
