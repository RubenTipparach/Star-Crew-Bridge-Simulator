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
