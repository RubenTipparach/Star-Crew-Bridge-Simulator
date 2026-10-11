"""Export editable Material Maker graphs using the installed application's own renderer.

This cross-platform wrapper resolves image paths in scratch copies of graphs and invokes
render_graph.gd inside the application. It never generates procedural pixels. The existing
postprocess.py builds the runtime layers and contact sheet from the resulting real exports.
Run: python tools/materials/export_graph.py exterior_paint --material-maker PATH_TO_EXE
"""
import argparse
import json
from pathlib import Path
import subprocess
import shutil
import sys
import tempfile
from PIL import Image

HERE = Path(__file__).resolve().parent


def main():
    """Resolve graph inputs, render through Material Maker, and reject missing output."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("graphs", nargs="+")
    parser.add_argument("--material-maker", required=True, type=Path)
    parser.add_argument("--size", default=2048, type=int)
    args = parser.parse_args()
    executable = args.material_maker.resolve(strict=True)
    raw = HERE / "raw"
    raw.mkdir(exist_ok=True)
    startup = None
    if sys.platform == "win32":
        startup = subprocess.STARTUPINFO()
        startup.dwFlags |= subprocess.STARTF_USESHOWWINDOW
        startup.wShowWindow = subprocess.SW_HIDE
    with tempfile.TemporaryDirectory(prefix="starcrew-material-graphs-") as scratch:
        rendered = Path(scratch) / "rendered"
        rendered.mkdir()
        for name in args.graphs:
            source = HERE / "ptex" / (name + ".ptex")
            graph = json.loads(source.read_text(encoding="utf-8"))
            for node in graph["nodes"]:
                if node["type"] == "image":
                    image = (source.parent / node["parameters"]["image"]).resolve(strict=True)
                    # Material Maker 1.4's device import expects RGBA rather than an RGB PNG.
                    imported = Path(scratch) / image.name
                    Image.open(image).convert("RGBA").save(imported)
                    node["parameters"]["image"] = imported.as_posix()
            target = Path(scratch) / source.name
            target.write_text(json.dumps(graph), encoding="utf-8")
            with (raw / (name + "_render.log")).open("w", encoding="utf-8") as log:
                subprocess.run([str(executable), "--script", (HERE / "render_graph.gd").as_posix(), "--",
                                target.as_posix(), rendered.as_posix(), str(args.size)],
                               cwd=executable.parent, startupinfo=startup, stdin=subprocess.DEVNULL,
                               stdout=log, stderr=subprocess.STDOUT, check=True, timeout=120)
            assert (rendered / (name + "_albedo.png")).is_file(), f"No albedo export for {name}"
            for output in rendered.glob(name + "*"):
                shutil.copyfile(output, raw / output.name)


if __name__ == "__main__":
    main()
