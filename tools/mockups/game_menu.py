#!/usr/bin/env python3
"""Build docs/mockups/game-menu.html, the mockup of `crew-nameplates` and `game-menu`, as one self-contained page.

The template (tools/mockups/game_menu.template.html) is a flat 2D page drawn over engine captures; this fills its
__WALK__, __DRILL__ and __HELM__ slots with the captures in docs/screenshots/mockups/game-menu/ and __FONT__ with egui's
default typeface (Ubuntu Light, from the `epaint_default_fonts` crate cargo fetched), so the page published as an
artifact needs no server (tools/mockups/flat_page.py builds it). `--check` fails when the page is stale.
"""

import os
import sys

from flat_page import ROOT, main

TEMPLATE = os.path.join(ROOT, "tools", "mockups", "game_menu.template.html")
SHOTS = os.path.join(ROOT, "docs", "screenshots", "mockups", "game-menu")
PAGE = os.path.join(ROOT, "docs", "mockups", "game-menu.html")
SLOTS = (("__WALK__", "walk.jpg"), ("__DRILL__", "drill.jpg"), ("__HELM__", "helm.jpg"))

if __name__ == "__main__":
    main(sys.argv[1:], TEMPLATE, SHOTS, SLOTS, PAGE, "game_menu.py")
