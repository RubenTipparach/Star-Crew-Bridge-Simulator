#!/usr/bin/env python3
"""Copy the one layout source and the shared mockup kit into every mockup page.

Mockups must work when opened straight from disk and when published as a single
claude.ai artifact, so they cannot fetch data/ships/<id>/layout.json at run
time. Instead each page carries marker comments and this tool writes the
current files between them (CLAUDE.md section 11):

    <!-- INLINE layout:tern BEGIN -->   ... <!-- INLINE layout:tern END -->
    <!-- INLINE shipkit BEGIN -->       ... <!-- INLINE shipkit END -->
    <!-- INLINE lib:lightbake BEGIN --> ... <!-- INLINE lib:lightbake END -->

"lib:<name>" copies docs/mockups/lib/<name>.js; "shipkit" is lib:shipkit.
"data:<ship>/<name>" copies data/ships/<ship>/<name>.json into a
<script id="ship-data-<name>" type="application/json"> block, for mockups that
read a ship's other data files (power.json, atmosphere.json).

--check rewrites nothing and fails when a page holds a stale copy, and also
checks that shipkit's PI_BUDGET matches the budget marker in the engine-stack
design (the table there is the source). Documentation tooling, standard library
only.

Usage: python3 tools/mockups/inline.py [--check] [page.html ...]
"""

import glob
import json
import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
LIB = os.path.join(ROOT, "docs", "mockups", "lib")
SHIPKIT = os.path.join(LIB, "shipkit.js")
BUDGET_SOURCES = [
    os.path.join(ROOT, "openspec", "specs", "engine-platform", "spec.md"),
    os.path.join(ROOT, "openspec", "changes", "engine-stack", "design.md"),
]
MARK = re.compile(r"(<!-- INLINE (layout:[a-z0-9_-]+|lib:[a-z0-9_-]+|data:[a-z0-9_-]+/[a-z0-9_-]+|shipkit) BEGIN -->)(.*?)(<!-- INLINE \2 END -->)", re.S)


def block(kind):
    if kind == "shipkit" or kind.startswith("lib:"):
        name = "shipkit" if kind == "shipkit" else kind.split(":", 1)[1]
        with open(os.path.join(LIB, name + ".js"), encoding="utf-8") as f:
            return "\n<script>\n" + f.read().rstrip() + "\n</script>\n"
    if kind.startswith("data:"):
        ship, name = kind.split(":", 1)[1].split("/", 1)
        with open(os.path.join(ROOT, "data", "ships", ship, name + ".json"), encoding="utf-8") as f:
            data = json.load(f)
        text = json.dumps(data, separators=(",", ":"), ensure_ascii=False).replace("</", "<\\/")
        return f'\n<script id="ship-data-{name}" type="application/json">\n{text}\n</script>\n'
    ship = kind.split(":", 1)[1]
    path = os.path.join(ROOT, "data", "ships", ship, "layout.json")
    with open(path, encoding="utf-8") as f:
        data = json.load(f)
    text = json.dumps(data, separators=(",", ":"), ensure_ascii=False).replace("</", "<\\/")
    return f'\n<script id="ship-layout" type="application/json">\n{text}\n</script>\n'


def process(page, check):
    with open(page, encoding="utf-8") as f:
        src = f.read()
    found = []

    def sub(m):
        found.append(m.group(2))
        return m.group(1) + block(m.group(2)) + m.group(4)

    out = MARK.sub(sub, src)
    rel = os.path.relpath(page, ROOT)
    if not found:
        print(f"  {rel}: no INLINE markers (skipped)")
        return True
    if out == src:
        print(f"  {rel}: current ({', '.join(found)})")
        return True
    if check:
        print(f"  FAIL {rel}: stale copy of {', '.join(found)}; run python3 tools/mockups/inline.py")
        return False
    with open(page, "w", encoding="utf-8") as f:
        f.write(out)
    print(f"  {rel}: updated ({', '.join(found)})")
    return True


def budget_ok():
    """shipkit's PI_BUDGET must equal the marker in the budget's source document."""
    with open(SHIPKIT, encoding="utf-8") as f:
        kit = f.read()
    kit_vals = {
        "triangles": int(re.search(r"triangles:\s*(\d+)", kit).group(1)),
        "draw_calls": int(re.search(r"drawCalls:\s*(\d+)", kit).group(1)),
        "texture_mb": int(re.search(r"textureMB:\s*(\d+)", kit).group(1)),
    }
    for src in BUDGET_SOURCES:
        if not os.path.exists(src):
            continue
        with open(src, encoding="utf-8") as f:
            m = re.search(r"<!-- pi-budget ([^>]*)-->", f.read())
        if not m:
            continue
        doc = {k: int(v) for k, v in re.findall(r"(\w+)=(\d+)", m.group(1))}
        bad = {k: (kit_vals[k], doc.get(k)) for k in kit_vals if doc.get(k) != kit_vals[k]}
        rel = os.path.relpath(src, ROOT)
        if bad:
            for k, (a, b) in bad.items():
                print(f"  FAIL shipkit PI_BUDGET {k}={a} but {rel} says {b}")
            return False
        print(f"  shipkit PI_BUDGET matches {rel}")
        return True
    print("  FAIL no pi-budget marker found in " + " or ".join(os.path.relpath(s, ROOT) for s in BUDGET_SOURCES))
    return False


def main(argv):
    check = "--check" in argv
    pages = [a for a in argv if not a.startswith("--")]
    if not pages:
        pages = sorted(glob.glob(os.path.join(ROOT, "docs", "mockups", "*.html")))
    ok = all([process(p, check) for p in pages])
    ok = budget_ok() and ok
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
