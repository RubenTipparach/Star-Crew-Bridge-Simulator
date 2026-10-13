#!/usr/bin/env python3
"""Write a mockup page in the form the claude.ai artifact publisher takes.

A mockup in docs/mockups is a complete HTML document, so it opens straight from disk. The
artifact publisher wraps a page in its own <!doctype html>, <html>, <head> and <body>, and asks
for the content without them. This tool writes that content: the same page with its document
wrapper, charset and viewport tags removed, and nothing else changed (the import map, the
inlined layout and kit, the styles and the scripts keep their order). Documentation tooling
(CLAUDE.md section 4), standard library only.

One more change, for the browsers: the published copy has no import map. Firefox ignores an import
map that comes after a module the host has already loaded (owner, 2026-10-13, on the touch
controls mockup: 'The specifier "three" was a bare specifier, but was not remapped to anything'),
so every specifier the page's map names is written out in full instead, as jsDelivr's "+esm" form
of the same file: jsDelivr rewrites the library's own imports in that form (an addon's "three"
becomes /npm/three@<version>/+esm), so the page and its addons share one three.js with no map.

Usage: python3 tools/mockups/artifact_copy.py docs/mockups/<page>.html <out_dir>
"""

import json
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


IMPORTMAP = re.compile(r'<script type="importmap">\s*(\{.*?\})\s*</script>\s*', re.S)
MODULE = re.compile(r'(<script type="module">)(.*?)(</script>)', re.S)
JSDELIVR = "https://cdn.jsdelivr.net/npm/"


def esm_url(url):
    """jsDelivr's +esm form of a file: a package's own entry (build/<name>.module.js) as the package's
    +esm, which is what jsDelivr writes into the files that import it; any other .js file with /+esm."""
    if not url.startswith(JSDELIVR):
        return url
    rest = url[len(JSDELIVR):]
    pkg = rest.split("/", 1)[0] if not rest.startswith("@") else "/".join(rest.split("/", 2)[:2])
    name = pkg.rsplit("@", 1)[0].split("/")[-1]
    if rest == pkg + "/build/" + name + ".module.js":
        return JSDELIVR + pkg + "/+esm"
    return url + "/+esm" if url.endswith(".js") else url


def resolve(spec, imports):
    """The URL an import map gives spec: an exact key, else the longest prefix key ending in '/'."""
    if spec in imports:
        return imports[spec]
    keys = [k for k in imports if k.endswith("/") and spec.startswith(k)]
    if not keys:
        return None
    k = max(keys, key=len)
    return imports[k] + spec[len(k):]


def unmap(out):
    """Drop the import map and write each mapped specifier in the module scripts out in full."""
    m = IMPORTMAP.search(out)
    if not m:
        return out
    imports = json.loads(m.group(1))["imports"]
    out = out[: m.start()] + out[m.end():]

    def spec(mm):
        url = resolve(mm.group(2), imports)
        if url is None:
            raise SystemExit(f"artifact_copy: {mm.group(2)!r} is not in the page's import map")
        return mm.group(1) + esm_url(url) + mm.group(3)

    def module(mm):
        body = re.sub(r'((?:\bfrom|\bimport)\s*\(?\s*")([^".][^"]*)(")', spec, mm.group(2))
        return mm.group(1) + body + mm.group(3)

    return MODULE.sub(module, out)


def convert(src):
    out = unmap(src)
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
