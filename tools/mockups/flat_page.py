#!/usr/bin/env python3
"""Build a flat 2D mockup page over engine captures as one self-contained file (the game menu's, the mission room's).

A template's __NAME__ slots take the captures in a folder of docs/screenshots/mockups/ (as JPEG data URIs) and __FONT__
takes egui's default typeface (Ubuntu Light, from the `epaint_default_fonts` crate cargo fetched), so the page
published as an artifact needs no server. With --check it fails when the page on disk is stale.
"""

import base64
import glob
import os
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))


def uri(path, mime):
    with open(path, "rb") as f:
        return f"data:{mime};base64," + base64.b64encode(f.read()).decode("ascii")


def font():
    home = os.environ.get("CARGO_HOME", os.path.expanduser("~/.cargo"))
    found = sorted(glob.glob(os.path.join(home, "registry", "src", "*", "epaint_default_fonts-*", "fonts", "Ubuntu-Light.ttf")))
    if not found:
        sys.exit("flat_page.py: Ubuntu-Light.ttf not found; run `cargo fetch` first")
    return found[-1]


def build(template, shots, slots):
    """The page: `template` with each (slot, file) of `slots` from the folder `shots`, and the font."""
    with open(template, encoding="utf-8") as f:
        page = f.read()
    for slot, name in slots:
        page = page.replace(slot, uri(os.path.join(shots, name), "image/jpeg"))
    return page.replace("__FONT__", uri(font(), "font/ttf"))


def main(argv, template, shots, slots, page_path, tool):
    page = build(template, shots, slots)
    if "--check" in argv:
        with open(page_path, encoding="utf-8") as f:
            if f.read() != page:
                sys.exit(f"{tool}: {os.path.relpath(page_path, ROOT)} is stale; run tools/mockups/{tool}")
        return
    with open(page_path, "w", encoding="utf-8") as f:
        f.write(page)
    print(f"wrote {os.path.relpath(page_path, ROOT)} ({len(page) // 1024} KiB)")
