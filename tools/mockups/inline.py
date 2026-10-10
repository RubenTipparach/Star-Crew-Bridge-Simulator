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
read a ship's other data files (power.json, atmosphere.json, detailing.json).
"data:lighting/<name>" copies data/lighting/<name>.json (fixtures, bake) the same way, and
"data:crew/<name>" data/crew/<name>.json (the walk's numbers, crew-on-deck).
"bakecache:<name>" copies docs/mockups/cache/<name>.bin (a page's baked light, written by
tools/mockups/bake_ship.mjs --write-cache) as base64 into <script id="bake-cache-<name>">, empty when
there is none; --check also fails when a cache was baked by another lightbake.js.
"materials" copies data/materials/materials.json and every layer it names
(assets/textures/<name>.png, as a base64 data URI) into a
<script id="ship-materials" type="application/json"> block, which shipkit's
loadMaterials() decodes into one texture array (surface-materials).
"panels" copies data/materials/panels.json, every panel layer it names at every size
(assets/textures/panels/<px>/<finish>_<module>.png, <finish>_strips.png, and the ceilings,
floors and trims: <finish>_ceiling_<module>.png, <finish>_floor_<module>.png, <finish>_trims.png)
and the UI images
(ui_screen_<finish>.png, keys_<finish>.png) into a <script id="ship-panels"
type="application/json"> block, which shipkit's loadPanels() adds to the texture array
(wall-panels, ceilings-and-trims, floor-panels).
"font:<name>" copies assets/fonts/<name>/<style>-<weight>.woff2 into a <style> block of
@font-face rules with data URIs (the family is the folder name in title case), so a page
published as one artifact needs no font server.
"models:<set>" copies assets/models/<set>/props.json and every .glb it lists (as base64
data URIs), and each prop's baked atlas PNG where it has one, into a
<script id="ship-models-<set>" type="application/json"> block, for pages that place the
Blender-built props (tools/blender, the blender-hard-surface skill).
Character review sets use characters.json (schema starcrew.crew-review/1) instead
of props.json, with their character rows keyed by id in the same models block.
"manifest:<set>" copies only assets/models/<set>/props.json, into a
<script id="ship-manifest-<set>" type="application/json"> block, for pages that need what
stands where (a prop's bounds, for fire-spread's fuel) but draw no prop: the damage map.
"screens" copies assets/textures/screens/screens.json and every image it names (each
station's <station>.png and <station>_upper.png, and the shared ui_screen_crew.png and
keys_crew.png of the wall panels) into a <script id="ship-screens" type="application/json">
block, { manifest, images: { <file as the manifest names it>: data URI } }, for pages that
draw console faces on the props (tools/mockups/console_screens.py; bridge-stations 11.6).

"repairs" copies assets/textures/repairs/covers.json with every image it names (the service
covers and the machines' insides, a circuit board and a terminal box's wiring, baked by
tools/blender/build_repair_covers.py) into a <script id="ship-repairs" type="application/json">
block, { covers, images: { <file>: data URI } }, for pages that open a machine for a repair
(repairs-on-deck design 3b).

--check rewrites nothing and fails when a page holds a stale copy, and also
checks that shipkit's PI_BUDGET matches the budget marker in the engine-stack
design (the table there is the source). Documentation tooling, standard library
only.

Usage: python3 tools/mockups/inline.py [--check] [page.html ...]
"""

import base64
import glob
import gzip
import hashlib
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
MATERIALS = os.path.join(ROOT, "data", "materials", "materials.json")
TEXTURES = os.path.join(ROOT, "assets", "textures")
PANELS = os.path.join(ROOT, "data", "materials", "panels.json")
SCREENS = os.path.join(ROOT, "assets", "textures", "screens", "screens.json")
CACHE = os.path.join(ROOT, "docs", "mockups", "cache")
LIGHTBAKE = os.path.join(LIB, "lightbake.js")
MARK = re.compile(r"(<!-- INLINE (layout:[a-z0-9_-]+|lib:[a-z0-9_-]+|data:[a-z0-9_-]+/[a-z0-9_-]+|bakecache:[a-z0-9_-]+|shipkit|materials|panels|screens|repairs|models:[a-z0-9_-]+|manifest:[a-z0-9_-]+|font:[a-z0-9_-]+) BEGIN -->)(.*?)(<!-- INLINE \2 END -->)", re.S)


def png_uri(path):
    with open(path, "rb") as f:
        return "data:image/png;base64," + base64.b64encode(f.read()).decode("ascii")


def model_manifest(name):
    """Read a prop or character-review manifest without inventing a second copy."""
    base = os.path.join(ROOT, "assets", "models", name)
    path = os.path.join(base, "props.json")
    characters = not os.path.exists(path)
    if characters:
        path = os.path.join(base, "characters.json")
    with open(path, encoding="utf-8") as f:
        manifest = json.load(f)
    if characters:
        if manifest.get("schema") != "starcrew.crew-review/1":
            raise ValueError(f"{path}: unsupported character manifest schema")
        rows = {row["id"]: row for row in manifest["characters"]}
        if len(rows) != len(manifest["characters"]):
            raise ValueError(f"{path}: duplicate character ids")
    else:
        rows = manifest.get("props", {})
    return manifest, rows


def bake_cache_index(path):
    """A bake cache's index (docs/mockups/cache/<name>.bin: gzip of "SCBK", version 1, index length, index JSON, data)."""
    with gzip.open(path, "rb") as f:
        raw = f.read()
    if raw[:4] != b"SCBK" or int.from_bytes(raw[4:8], "little") != 1:
        raise ValueError(f"{os.path.relpath(path, ROOT)} is not a version 1 bake cache")
    n = int.from_bytes(raw[8:12], "little")
    return json.loads(raw[12:12 + n].decode("utf-8"))


def bake_caches_ok():
    """Every bake cache was written by the baker the pages carry (lightbake.js), or it would be stale light."""
    with open(LIGHTBAKE, "rb") as f:
        sha = hashlib.sha256(f.read()).hexdigest()
    ok = True
    for path in sorted(glob.glob(os.path.join(CACHE, "*.bin"))):
        rel = os.path.relpath(path, ROOT)
        if bake_cache_index(path).get("baker_sha256") != sha:
            print(f"  FAIL {rel}: baked by another lightbake.js; run node tools/mockups/bake_ship.mjs --write-cache")
            ok = False
        else:
            print(f"  {rel}: baked by the current lightbake.js")
    return ok


def block(kind):
    if kind.startswith("font:"):
        # A typeface's woff2 files (assets/fonts/<name>/<style>-<weight>.woff2) as @font-face rules with data URIs,
        # so a page published as one artifact needs no font server. The family is the folder's name in title case.
        name = kind.split(":", 1)[1]
        base = os.path.join(ROOT, "assets", "fonts", name)
        family = " ".join(w.capitalize() for w in name.split("-"))
        rules = []
        for path in sorted(glob.glob(os.path.join(base, "*.woff2"))):
            weight = int(os.path.basename(path)[:-6].rsplit("-", 1)[1])
            with open(path, "rb") as f:
                uri = "data:font/woff2;base64," + base64.b64encode(f.read()).decode("ascii")
            rules.append(f'@font-face {{ font-family: "{family}"; font-weight: {weight}; font-style: normal; src: url({uri}) format("woff2"); }}')
        if not rules:
            raise ValueError(f"no woff2 files in {os.path.relpath(base, ROOT)}")
        return "\n<style>\n" + "\n".join(rules) + "\n</style>\n"
    if kind.startswith("bakecache:"):
        # The deck plan's baked light (tools/mockups/bake_ship.mjs --write-cache), as base64; empty when there is none,
        # and the page then bakes live.
        path = os.path.join(CACHE, kind.split(":", 1)[1] + ".bin")
        name = kind.split(":", 1)[1]
        if not os.path.exists(path):
            return f'\n<script id="bake-cache-{name}" type="application/octet-stream"></script>\n'
        with open(path, "rb") as f:
            text = base64.b64encode(f.read()).decode("ascii")
        return f'\n<script id="bake-cache-{name}" type="application/octet-stream">\n{text}\n</script>\n'
    if kind == "screens":
        with open(SCREENS, encoding="utf-8") as f:
            manifest = json.load(f)
        base = os.path.dirname(SCREENS)
        images = {}
        for rec in manifest["stations"].values():
            for part in ("main", "upper"):
                images[rec[part]["file"]] = png_uri(os.path.join(base, rec[part]["file"]))
        for rec in manifest["shared"].values():
            images[rec["file"]] = png_uri(os.path.join(ROOT, rec["file"]))
        text = json.dumps({"manifest": manifest, "images": dict(sorted(images.items()))}, separators=(",", ":"),
                          ensure_ascii=False).replace("</", "<\\/")
        return f'\n<script id="ship-screens" type="application/json">\n{text}\n</script>\n'
    if kind.startswith("manifest:"):
        name = kind.split(":", 1)[1]
        manifest, _ = model_manifest(name)
        text = json.dumps(manifest, separators=(",", ":"), ensure_ascii=False).replace("</", "<\\/")
        return f'\n<script id="ship-manifest-{name}" type="application/json">\n{text}\n</script>\n'
    if kind.startswith("models:"):
        name = kind.split(":", 1)[1]
        base = os.path.join(ROOT, "assets", "models", name)
        manifest, rows = model_manifest(name)
        models, atlases = {}, {}
        # Alternate paint sets use the same model loader and retain their own measured manifests.
        for livery in manifest.get("liveries", []):
            directory = livery["directory"]
            livery["model_suffix"] = "" if directory == "." else "__" + livery["id"]
            if directory == ".":
                continue
            with open(os.path.join(base, directory, "props.json"), encoding="utf-8") as f:
                variant = json.load(f)
            for key, rec in variant["props"].items():
                rec["file"] = directory + "/" + rec["file"]
                if rec.get("atlas"):
                    rec["atlas"]["file"] = directory + "/" + rec["atlas"]["file"]
                rows[key + livery["model_suffix"]] = rec
        for key, rec in sorted(rows.items()):
            with open(os.path.join(base, rec["file"]), "rb") as f:
                models[key] = "data:model/gltf-binary;base64," + base64.b64encode(f.read()).decode("ascii")
            # A prop's own baked texture (ship-props design 4c), when its set's build has made one.
            if rec.get("atlas"):
                atlases[key] = png_uri(os.path.join(base, rec["atlas"]["file"]))
        block = {"manifest": manifest, "models": models, "atlases": atlases}
        # A set's service bays (repairs-on-deck 3c: where each machine's cover and screws sit), when its build writes them.
        if os.path.exists(os.path.join(base, "service.json")):
            with open(os.path.join(base, "service.json"), encoding="utf-8") as f:
                block["service"] = json.load(f)
        text = json.dumps(block, separators=(",", ":"), ensure_ascii=False).replace("</", "<\\/")
        return f'\n<script id="ship-models-{name}" type="application/json">\n{text}\n</script>\n'
    if kind == "panels":
        with open(PANELS, encoding="utf-8") as f:
            manifest = json.load(f)
        base = os.path.join(ROOT, manifest["layers"]["dir"])
        stems = []
        for fn, fin in manifest["finishes"].items():
            stems += sorted(((m["layer"], f"{fn}_{name}") for name, m in fin["modules"].items()))
            stems.append((fin["strips"]["layer"], f"{fn}_strips"))
            for kind in ("ceiling", "floor"):
                if kind in fin:
                    stems += sorted(((m["layer"], f"{fn}_{kind}_{name}") for name, m in fin[kind]["modules"].items()
                                     if not name.startswith("_")))
            if "trims" in fin:
                stems.append((fin["trims"]["layer"], f"{fn}_trims"))
            if "platforms" in fin:   # risers, step fronts and the members still on tiling trim (ceilings-and-trims 9)
                stems.append((fin["platforms"]["layer"], f"{fn}_platforms"))
            if "edges" in fin:   # the bands at platforms' feet (floor-panels 6)
                stems.append((fin["edges"]["layer"], f"{fn}_floor_edges"))
        # Upholstery (ship-props 4b): one bake for every finish.
        for name, layer in manifest.get("upholstery", {}).get("layers", {}).items():
            stems.append((layer, f"upholstery_{name}"))
        layers = {str(px): {stem: png_uri(os.path.join(base, str(px), stem + ".png")) for _, stem in sorted(stems)}
                  for px in manifest["layers"]["sizes_px"]}
        ui = {}
        for fn in manifest["finishes"]:
            for stem in (f"ui_screen_{fn}", f"keys_{fn}"):
                ui[stem] = png_uri(os.path.join(base, stem + ".png"))
        text = json.dumps({"manifest": manifest, "layers": layers, "ui": ui}, separators=(",", ":"), ensure_ascii=False).replace("</", "<\\/")
        return f'\n<script id="ship-panels" type="application/json">\n{text}\n</script>\n'
    if kind == "repairs":
        base = os.path.join(ROOT, "assets", "textures", "repairs")
        with open(os.path.join(base, "covers.json"), encoding="utf-8") as f:
            covers = json.load(f)
        files = ([c["file"] for c in covers["covers"].values()] + [r["file"] for r in covers["interiors"].values()]
                 + [b["file"] for b in covers.get("boards", {}).values()])
        images = {rel: png_uri(os.path.join(ROOT, rel)) for rel in sorted(files)}
        text = json.dumps({"covers": covers, "images": images}, separators=(",", ":"),
                          ensure_ascii=False).replace("</", "<\\/")
        return f'\n<script id="ship-repairs" type="application/json">\n{text}\n</script>\n'
    if kind == "materials":
        with open(MATERIALS, encoding="utf-8") as f:
            manifest = json.load(f)
        layers = {}
        for name in sorted(manifest["materials"], key=lambda n: manifest["materials"][n]["layer"]):
            with open(os.path.join(TEXTURES, name + ".png"), "rb") as f:
                layers[name] = "data:image/png;base64," + base64.b64encode(f.read()).decode("ascii")
        text = json.dumps({"manifest": manifest, "layers": layers}, separators=(",", ":"), ensure_ascii=False).replace("</", "<\\/")
        return f'\n<script id="ship-materials" type="application/json">\n{text}\n</script>\n'
    if kind == "shipkit" or kind.startswith("lib:"):
        name = "shipkit" if kind == "shipkit" else kind.split(":", 1)[1]
        with open(os.path.join(LIB, name + ".js"), encoding="utf-8") as f:
            return "\n<script>\n" + f.read().rstrip() + "\n</script>\n"
    if kind.startswith("data:"):
        ship, name = kind.split(":", 1)[1].split("/", 1)
        # data:lighting/<name> is data/lighting/<name>.json (the fixture types and bake settings, light-baking
        # design 15), data:crew/<name> data/crew/<name>.json (the walk); any other data:<ship>/<name> a ship's file.
        sub = (ship,) if ship in ("lighting", "crew") else ("ships", ship)
        with open(os.path.join(ROOT, "data", *sub, name + ".json"), encoding="utf-8") as f:
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
    ok = bake_caches_ok() and ok
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
