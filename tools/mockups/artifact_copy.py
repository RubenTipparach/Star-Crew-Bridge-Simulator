#!/usr/bin/env python3
"""Write a mockup page in the form the claude.ai artifact publisher takes.

A mockup in docs/mockups is a complete HTML document, so it opens straight from disk. The
artifact publisher wraps a page in its own <!doctype html>, <html>, <head> and <body>, and asks
for the content without them. This tool writes that content: the same page with its document
wrapper, charset and viewport tags removed, and nothing else changed (the import map, the
inlined layout and kit, the styles and the scripts keep their order). Documentation tooling
(CLAUDE.md section 4), standard library only.

Usage: python3 tools/mockups/artifact_copy.py docs/mockups/<page>.html <out_dir>
"""

import os
import re
import sys

STRIP = [
    r"^\s*<!doctype html>\s*",
    r"<html[^>]*>",
    r"</html>",
    r"<head>",
    r"</head>",
    r"<body[^>]*>",
    r"</body>",
    r'<meta charset="[^"]*">',
    r'<meta name="viewport"[^>]*>',
]


def convert(src):
    out = src
    # The leading comment that tells a reader how to rebuild the page stays out of the artifact.
    out = re.sub(r"^\s*<!doctype html>\s*<!--.*?-->", "", out, count=1, flags=re.S | re.I)
    for pat in STRIP:
        out = re.sub(pat, "", out, count=1, flags=re.I | re.M)
    # The publisher scans the first 8 KB for <title>; keep it first.
    m = re.search(r"<title>.*?</title>", out, flags=re.S)
    if m:
        out = m.group(0) + "\n" + out[: m.start()] + out[m.end():]
    # Mockups are deliberately dark (space); the publisher's skeleton pins a light colour scheme,
    # which would draw native controls (selects, sliders) light.
    out = out.replace("</title>", "</title>\n<style>:root { color-scheme: dark; }</style>", 1)
    return out.strip() + "\n"


def main(argv):
    if len(argv) != 2:
        print(__doc__)
        return 2
    page, out_dir = argv
    with open(page, encoding="utf-8") as f:
        src = f.read()
    os.makedirs(out_dir, exist_ok=True)
    out = os.path.join(out_dir, os.path.basename(page))
    with open(out, "w", encoding="utf-8") as f:
        f.write(convert(src))
    print(out)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
