#!/usr/bin/env python3
"""Copies a Blender pipeline kit from this repository into another project, with its provenance.

It owns taking a kit (the files a skill's kit.json lists) to another repository. The Blender
skills (blender-csg-levels, blender-humanoid-characters) each keep a kit.json; this one script
serves both, so there is one way to borrow a kit. Every file lands at the same relative path
under the target, and KIT_PROVENANCE_<kit>.md at the target's root records the source
repository, the revision, the date, the files and what to adapt, because borrowed code keeps
its provenance (CLAUDE.md 5.6). It never overwrites a file in the target unless --force.

  python3 copy_kit.py <kit.json> --check                  # every listed file exists here
  python3 copy_kit.py <kit.json> <target_root> [--force]  # copy, and write the provenance

Plain Python, no dependencies. Run it from anywhere inside this repository.
"""
import datetime
import json
import os
import shutil
import subprocess
import sys


def repo_root():
    here = os.path.dirname(os.path.abspath(__file__))
    return subprocess.run(["git", "-C", here, "rev-parse", "--show-toplevel"], capture_output=True, text=True,
                          check=True).stdout.strip()


def load(path):
    with open(path, encoding="utf-8") as f:
        kit = json.load(f)
    for key in kit:
        if key not in ("kit", "source", "files", "adapt", "_comment"):
            raise SystemExit(f"{path}: unknown key {key!r}")
    for i, entry in enumerate(kit["files"]):
        for key in entry:
            if key not in ("path", "why"):
                raise SystemExit(f"{path}: files[{i}]: unknown key {key!r}")
    return kit


def check(kit, root):
    missing = [e["path"] for e in kit["files"] if not os.path.exists(os.path.join(root, e["path"]))]
    if missing:
        raise SystemExit(f"kit {kit['kit']}: listed but missing here: {', '.join(missing)}")
    print(f"kit {kit['kit']}: {len(kit['files'])} files present")


def copy(kit, root, target, force):
    rev = subprocess.run(["git", "-C", root, "rev-parse", "HEAD"], capture_output=True, text=True,
                         check=True).stdout.strip()
    clashes = [e["path"] for e in kit["files"] if os.path.exists(os.path.join(target, e["path"]))]
    if clashes and not force:
        raise SystemExit(f"would overwrite {len(clashes)} files in {target} (first: {clashes[0]}); pass --force")
    for e in kit["files"]:
        src, dst = os.path.join(root, e["path"]), os.path.join(target, e["path"])
        if os.path.isdir(src):
            shutil.copytree(src, dst, dirs_exist_ok=True)
        else:
            os.makedirs(os.path.dirname(dst), exist_ok=True)
            shutil.copy2(src, dst)
    lines = [f"# Borrowed kit: {kit['kit']}", "",
             f"Copied from {kit['source']} at revision {rev}, on {datetime.date.today().isoformat()}, by the",
             f"skill's copy_kit.py. The files keep their source paths.", "", "## Files", ""]
    lines += [f"- `{e['path']}`: {e['why']}" for e in kit["files"]]
    lines += ["", "## Adapt before the first build", ""] + [f"- {a}" for a in kit["adapt"]] + [""]
    out = os.path.join(target, f"KIT_PROVENANCE_{kit['kit']}.md")
    with open(out, "w", encoding="utf-8") as f:
        f.write("\n".join(lines))
    print(f"copied {len(kit['files'])} files to {target}; provenance in {out}")


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    if not args:
        raise SystemExit(__doc__)
    kit, root = load(os.path.abspath(args[0])), repo_root()
    check(kit, root)
    if "--check" in sys.argv:
        return
    if len(args) < 2:
        raise SystemExit("name the target project's root, or pass --check")
    copy(kit, root, os.path.abspath(args[1]), "--force" in sys.argv)


if __name__ == "__main__":
    main()
