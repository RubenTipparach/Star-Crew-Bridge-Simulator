#!/usr/bin/env python3
"""Write a mockup page in the form the claude.ai artifact publisher takes.

A mockup in docs/mockups is a complete HTML document, so it opens straight from disk. The
artifact publisher wraps a page in its own <!doctype html>, <html>, <head> and <body>, and asks
for the content without them. This tool writes that content: the same page with its document
wrapper, charset and viewport tags removed, and nothing else changed (the import map, the
inlined layout and kit, the styles and the scripts keep their order). Documentation tooling
(CLAUDE.md section 4), standard library only.

A page over the artifact limit (16 MB a page) takes --split KB: every inlined data block (a
<script id=... type="application/json|application/octet-stream|text/plain"> of at least KB
kilobytes: models, textures, panels) is gzipped and written as base64 text into
<out_dir>/blk/<id>.txt (the artifact host serves text, not raw binary), and a small loader
fetches, decodes, unzips and puts each one back as the same <script> element before the page's module
runs, so the page reads them exactly as it does from disk. The publish then passes the blk files
as the artifact's supporting files (the tool prints the list). A split copy needs a web server:
fetch() does not read file:// URLs.

Usage: python3 tools/mockups/artifact_copy.py docs/mockups/<page>.html <out_dir> [--split KB]
"""

import base64
import gzip
import json
import os
import re
import sys

# Fetches every split block, decodes and unzips it and re-creates its
# <script> element; the page's module scripts await it before their first statement.
LOADER = """<script>
window.__blocksReady = Promise.all(%s.map(async ([id, type, src]) => {
  const r = await fetch(src);
  if (!r.ok) throw new Error("block " + src + ": HTTP " + r.status);
  const bin = atob((await r.text()).trim());
  const buf = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) buf[i] = bin.charCodeAt(i);
  const text = await new Response(new Blob([buf]).stream().pipeThrough(new DecompressionStream("gzip"))).text();
  const s = document.createElement("script");
  s.id = id; s.type = type; s.text = text;
  document.head.appendChild(s);
}));
</script>
"""

DATA_BLOCK = re.compile(
    r'<script\s+id="([^"]+)"\s+type="(application/json|application/octet-stream|text/plain)"[^>]*>(.*?)</script>',
    re.S)

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


def split(out, out_dir, min_bytes):
    """Move each data block of at least min_bytes into out_dir/blk/<id>.txt, gzipped and base64, behind LOADER."""
    os.makedirs(os.path.join(out_dir, "blk"), exist_ok=True)
    blocks = []

    def move(m):
        sid, typ, body = m.group(1), m.group(2), m.group(3)
        if len(body) < min_bytes:
            return m.group(0)
        rel = "blk/%s.txt" % sid
        with open(os.path.join(out_dir, rel), "wb") as f:
            f.write(base64.b64encode(gzip.compress(body.encode("utf-8"), 9, mtime=0)))
        blocks.append([sid, typ, rel])
        return ""

    out = DATA_BLOCK.sub(move, out)
    if not blocks:
        return out, []
    out = out.replace('<script type="module">', '<script type="module">\nawait window.__blocksReady;')
    m = re.search(r"</title>.*?\n", out, flags=re.S)
    out = out[: m.end()] + LOADER % json.dumps(blocks) + out[m.end():]
    return out, [b[2] for b in blocks]


def main(argv):
    args = list(argv)
    min_kb = None
    if "--split" in args:
        i = args.index("--split")
        min_kb = int(args[i + 1])
        del args[i:i + 2]
    if len(args) != 2:
        print(__doc__)
        return 2
    page, out_dir = args
    with open(page, encoding="utf-8") as f:
        src = f.read()
    os.makedirs(out_dir, exist_ok=True)
    out_html = convert(src)
    files = []
    if min_kb is not None:
        out_html, files = split(out_html, out_dir, min_kb * 1024)
    out = os.path.join(out_dir, os.path.basename(page))
    with open(out, "w", encoding="utf-8") as f:
        f.write(out_html)
    print(out)
    for rel in files:
        print("%10d %s" % (os.path.getsize(os.path.join(out_dir, rel)), rel))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
