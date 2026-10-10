#!/usr/bin/env python3
"""Build docs/mockups/game-menu.html, the mockup of `crew-nameplates` and `game-menu`, as one self-contained page.

The template (tools/mockups/game_menu.template.html) is a flat 2D page drawn over engine captures; this fills its
__WALK__, __DRILL__ and __HELM__ slots with the captures in docs/screenshots/mockups/game-menu/ and __FONT__ with egui's
default typeface (Ubuntu Light, from the `epaint_default_fonts` crate cargo fetched), so the page published as an
artifact needs no server. `--check` fails when the page is stale.
"""

import base64
import glob
import os
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
TEMPLATE = os.path.join(ROOT, "tools", "mockups", "game_menu.template.html")
SHOTS = os.path.join(ROOT, "docs", "screenshots", "mockups", "game-menu")
PAGE = os.path.join(ROOT, "docs", "mockups", "game-menu.html")


def uri(path, mime):
    with open(path, "rb") as f:
        return f"data:{mime};base64," + base64.b64encode(f.read()).decode("ascii")


def font():
    home = os.environ.get("CARGO_HOME", os.path.expanduser("~/.cargo"))
    found = sorted(glob.glob(os.path.join(home, "registry", "src", "*", "epaint_default_fonts-*", "fonts", "Ubuntu-Light.ttf")))
    if not found:
        sys.exit("game_menu.py: Ubuntu-Light.ttf not found; run `cargo fetch` first")
    return found[-1]


def build():
    with open(TEMPLATE, encoding="utf-8") as f:
        page = f.read()
    for slot, name in (("__WALK__", "walk.jpg"), ("__DRILL__", "drill.jpg"), ("__HELM__", "helm.jpg")):
        page = page.replace(slot, uri(os.path.join(SHOTS, name), "image/jpeg"))
    return page.replace("__FONT__", uri(font(), "font/ttf"))


def main(argv):
    page = build()
    if "--check" in argv:
        with open(PAGE, encoding="utf-8") as f:
            if f.read() != page:
                sys.exit("game_menu.py: docs/mockups/game-menu.html is stale; run tools/mockups/game_menu.py")
        return
    with open(PAGE, "w", encoding="utf-8") as f:
        f.write(page)
    print(f"wrote {os.path.relpath(PAGE, ROOT)} ({len(page) // 1024} KiB)")


if __name__ == "__main__":
    main(sys.argv[1:])
