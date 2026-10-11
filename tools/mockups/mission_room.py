#!/usr/bin/env python3
"""Build docs/mockups/mission-room.html, the mockup of `crew-chat` and `missions`, as one self-contained page.

The template (tools/mockups/mission_room.template.html) is drawn over the engine captures in
docs/screenshots/mockups/mission-room/; tools/mockups/flat_page.py does the building. `--check` fails when the page is
stale.
"""

import os
import sys

from flat_page import ROOT, main

TEMPLATE = os.path.join(ROOT, "tools", "mockups", "mission_room.template.html")
SHOTS = os.path.join(ROOT, "docs", "screenshots", "mockups", "mission-room")
PAGE = os.path.join(ROOT, "docs", "mockups", "mission-room.html")
SLOTS = (("__CAPTAIN__", "captain.jpg"), ("__HELM__", "helm.jpg"), ("__WALKING__", "walking.jpg"), ("__BRIDGE__", "bridge.jpg"))

if __name__ == "__main__":
    main(sys.argv[1:], TEMPLATE, SHOTS, SLOTS, PAGE, "mission_room.py")
