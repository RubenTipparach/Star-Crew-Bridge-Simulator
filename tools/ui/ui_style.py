#!/usr/bin/env python3
"""The screens around the consoles as data: the UI art direction's style, read from its approved mockup.

The mockup docs/mockups/ui-art-direction.html (openspec/changes/ui-art-direction, approved by the owner on
2026-10-16) is the specification of every screen outside the consoles (CLAUDE.md 10). Its tokens (`const UI`: the
safe area, the grid, the four type sizes, the colour roles, the role colours, the motion), its type roles and its
keycap, item and button sizes (the `.t-*`, `.key`, `.item` and `.btn-*` rules of its style sheet), the glyphs drawn
in its screens (the mouse keycaps, the dice, the shuffle of Any, the objective diamond, the marker) and its two key
art pictures (the front menu's and the loading screen's) are the look the engine has to draw. Rather than a second
hand-typed copy in Rust, this tool reads them out of the mockup's own text and writes `data/ui/ui_style.json` and
`assets/ui/keyart-*.png`, which the engine compiles in (design 9). `--check` fails when the files are not what the
mockup says, so the two cannot drift: change a token in the mockup, rerun this.

The reference frame is not copied: it is data/ui/reference.json's, and the mockup's `reference_px` is checked
against it. The text floor (12 px, the owner's answer to Q8 on 2026-10-16) is the one value that is not in the
mockup's tokens; it is written here with its source.
"""
import argparse
import base64
import io
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
MOCKUP = ROOT / "docs" / "mockups" / "ui-art-direction.html"
REFERENCE = ROOT / "data" / "ui" / "reference.json"
OUT = ROOT / "data" / "ui" / "ui_style.json"
ART = ROOT / "assets" / "ui"
# The key art's size as loaded (design 15: the 3D pass's size, about 3.7 MB each as uploaded).
ART_PX = (1280, 720)
# design 16, Q8 (owner, 2026-10-16, "approve"): never draw text under 12 px; panels grow.
TEXT_FLOOR_PX = 12

# The glyphs the screens draw beyond the consoles' icons, each found after a marker in the mockup's text.
GLYPHS = {
    # The objective's diamond (objRows) and the first-time hint's station marker.
    "diamond": "'<path d=\"M12 4l8 8-8 8-8-8z\"/>'",
    "marker": '<path d="M12 3l9 9-9 9-9-9z"/>',
    # The rank list's chevron and the dice for a new name (the Single Player page).
    "chevron": '<path d="M6 9l6 6 6-6"/>',
}


def block(text, start, open_ch="{", close_ch="}"):
    """The text between the bracket that opens at or after `start` and its partner."""
    i = text.index(open_ch, start)
    depth = 0
    for j in range(i, len(text)):
        if text[j] == open_ch:
            depth += 1
        elif text[j] == close_ch:
            depth -= 1
            if depth == 0:
                return text[i + 1 : j]
    raise SystemExit("ui_style: unbalanced block")


def js_object(body):
    """A JavaScript object literal of numbers, strings, arrays and objects, as Python."""
    s = re.sub(r"([{,]\s*)(\w+)\s*:", r'\1"\2":', "{" + body + "}")
    s = re.sub(r",(\s*[}\]])", r"\1", s)
    return json.loads(s)


def type_roles(page):
    """The four type roles from the style sheet: size token, weight, line height, tracking and case."""
    out = {}
    for name in ("display", "title", "body", "label"):
        rule = re.search(r"\.t-%s \{([^}]*)\}" % name, page).group(1)
        weight, line = re.search(r"font:\s*(\d+) var\(--t-%s\)/([\d.]+)" % name, rule).groups()
        track = re.search(r"letter-spacing:\s*([\d.]+)em", rule)
        out[name] = {
            "weight": int(weight),
            "line": float(line),
            "tracking_em": float(track.group(1)) if track else 0.0,
            "caps": "uppercase" in rule,
        }
    return out


def px(rule, prop):
    m = re.search(r"%s:\s*([\d.]+)px" % re.escape(prop), rule)
    if not m:
        raise SystemExit(f"ui_style: no {prop} in {rule!r}")
    return float(m.group(1))


def sizes(page):
    """Keycaps, menu items and buttons, reference pixels (design 6 and 7)."""
    rule = lambda sel: re.search(r"%s \{([^}]*)\}" % re.escape(sel), page).group(1)
    key, item = rule(".key"), rule(".item")
    hold = rule(".hold")
    return {
        "key_h": px(key, "height"),
        "key_min_w": px(key, "min-width"),
        "key_pad": float(re.search(r"padding: 0 ([\d.]+)px", key).group(1)),
        "key_edge": px(key, "border-bottom-width"),
        "key_radius": px(key, "border-radius"),
        "key_gap": px(rule(".keys"), "gap"),
        "hold_px": float(re.search(r"font: 700 ([\d.]+)px", hold).group(1)),
        "item_h": px(item, "height"),
        "item_pad": float(re.search(r"padding: 0 ([\d.]+)px", item).group(1)),
        "bar": float(re.search(r"inset ([\d.]+)px 0 0 var\(--accent\)", rule(".barl")).group(1)),
        "primary_h": px(rule(".btn-primary"), "height"),
        "secondary_h": px(rule(".btn-secondary"), "height"),
    }


def svg_after(page, marker):
    """The inner SVG of the first `<svg ...>` at or after `marker`."""
    i = page.index(marker)
    j = page.index(">", page.index("<svg", i)) + 1
    return page[j : page.index("</svg>", j)]


def glyphs(page):
    mouse = block(page, page.index("const MOUSE = {"))
    out = {k: v for k, v in re.findall(r"^\s*(\w+):\s*'([^']*)',?\s*$", mouse, re.M)}
    for name, marker in GLYPHS.items():
        if marker not in page:
            raise SystemExit(f"ui_style: the mockup has no {name} glyph ({marker})")
        out[name] = marker.strip("'")
    out["dice"] = svg_after(page, 'title="New name"')
    out["any"] = svg_after(page, "'<svg class=\"i\" viewBox=\"0 0 24 24\" style=\"width:36px;color:var(--dim)\">")
    return dict(sorted(out.items()))


def style(page):
    ui = js_object(block(page, page.index("const UI = {")))
    ref = json.loads(REFERENCE.read_text())["reference_px"]
    if [float(v) for v in ui["reference_px"]] != [float(v) for v in ref]:
        raise SystemExit(f"ui_style: the mockup's reference_px {ui['reference_px']} is not {REFERENCE.name}'s {ref}")
    roles = type_roles(page)
    for k, v in ui["type_px"].items():
        roles[k]["px"] = v
    return {
        "schema": "starcrew.ui-style/1",
        "_doc": (
            "Generated by tools/ui/ui_style.py from docs/mockups/ui-art-direction.html (const UI, its type, keycap, "
            "item and button rules, and the glyphs its screens draw). Do not edit: change the mockup and rerun the "
            "tool (openspec/changes/ui-art-direction design 9). Sizes are reference pixels on data/ui/reference.json's "
            "1920 x 1080 frame; colours are sRGB hex; glyphs are SVG fragments on a 24-unit grid, stroked 2 units wide "
            "with round caps and joins, a fill of currentColor or var(--accent) filled. min_text_px is the owner's "
            "answer to Q8 (2026-10-16): no text is drawn under 12 output pixels. motion_ease names each motion's curve (linear, "
            "out_cubic or in_out_cubic, design 18a)."
        ),
        "safe_px": ui["safe_px"],
        "grid_px": ui["grid_px"],
        "min_text_px": TEXT_FLOOR_PX,
        "type": roles,
        "sizes_px": sizes(page),
        "colours": ui["colours"],
        "accent_alt": ui["accent_alt"],
        "roles": ui["roles"],
        "motion_ms": ui["motion_ms"],
        "motion_ease": ui["motion_ease"],
        "glyphs": glyphs(page),
    }


def key_art(page):
    """The front menu's and the loading screen's pictures (the `world` image of each scene), at ART_PX."""
    from PIL import Image

    out = {}
    for scene in ("menu", "loading"):
        i = page.index(f'<section class="scene after" data-id="{scene}">')
        m = re.compile(r'<div class="world"><img src="data:image/\w+;base64,([A-Za-z0-9+/=]+)"').search(page, i)
        im = Image.open(io.BytesIO(base64.b64decode(m.group(1)))).convert("RGB")
        out[scene] = im.resize(ART_PX, Image.LANCZOS)
    return out


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--check", action="store_true", help="fail if the committed files are not what the mockup says")
    a = ap.parse_args()
    page = MOCKUP.read_text()
    text = json.dumps(style(page), indent=1) + "\n"
    art = key_art(page)
    if a.check:
        bad = []
        if not OUT.exists() or OUT.read_text() != text:
            bad.append(str(OUT.relative_to(ROOT)))
        from PIL import Image, ImageChops

        for name, im in art.items():
            p = ART / f"keyart-{name}.png"
            if not p.exists():
                bad.append(str(p.relative_to(ROOT)))
                continue
            have = Image.open(p).convert("RGB")
            if have.size != im.size or ImageChops.difference(have, im).getbbox() is not None:
                bad.append(str(p.relative_to(ROOT)))
        if bad:
            print("ui_style: stale, rerun tools/ui/ui_style.py: " + ", ".join(bad), file=sys.stderr)
            return 1
        print(f"ui_style: {OUT.relative_to(ROOT)} and the key art are the mockup's")
        return 0
    OUT.write_text(text)
    ART.mkdir(parents=True, exist_ok=True)
    for name, im in art.items():
        im.save(ART / f"keyart-{name}.png", optimize=True)
    print(f"ui_style: wrote {OUT.relative_to(ROOT)} and {len(art)} key art pictures in {ART.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
