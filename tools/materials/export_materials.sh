#!/usr/bin/env bash
# Render Star Crew's material graphs and build the texture layers from them.
#
# It owns getting raw maps into tools/materials/raw/ (gitignored) and then running
# tools/materials/postprocess.py, which writes assets/textures/<name>.png and the contact sheet.
# It lives beside the graphs it renders. Adapted from fps-game-demo f6cd25c
# tools/material_maker/export_materials.sh (the Material Maker command line is the same).
#
# Two ways to get the raw maps:
#
#   1. Material Maker (the normal way). Renders tools/materials/ptex/<graph>.ptex for every graph
#      data/materials/materials.json uses, or the graphs named on the command line:
#        MATERIAL_MAKER_DIR=/path/to/material-maker tools/materials/export_materials.sh [graph ...]
#      MATERIAL_MAKER_DIR is a Material Maker release (with material_maker.x86_64) or a source
#      checkout run by a Godot 4.7 binary (GODOT, default "godot"). Material Maker writes
#      <graph>_albedo.png, _normal.png, _orm.png and _emission.png at 2048 px.
#
#   2. --from-fps <fps-game-demo dir> (when Material Maker cannot run, as in a Claude Code cloud
#      session: it needs Godot and a GPU, and release downloads are blocked there). Copies
#      fps-game-demo's committed exports of the same graphs (game/textures/<graph>.png,
#      _normal, _orm, _emission: Material Maker renders of the same .ptex files, already
#      downsampled to 1024 px by fps's postprocess.py) into raw/, and records the fps revision in
#      raw/SOURCE.txt. The 2026-10-05 layers were built this way, from fps-game-demo f6cd25c.
#        tools/materials/export_materials.sh --from-fps /path/to/fps-game-demo
#
# Either way raw/ is cleared first, so it never mixes the two namings.
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
RAW="$HERE/raw"

FPS=""
if [ "${1:-}" = "--from-fps" ]; then
  FPS="${2:?--from-fps needs the fps-game-demo directory}"
  shift 2
fi

if [ $# -gt 0 ]; then
  graphs=("$@")
else
  mapfile -t graphs < <(python3 "$HERE/postprocess.py" --list-graphs)
fi

rm -rf "$RAW"
mkdir -p "$RAW"

if [ -n "$FPS" ]; then
  TEX="$FPS/game/textures"
  [ -d "$TEX" ] || { echo "no $TEX: is $FPS an fps-game-demo checkout?" >&2; exit 1; }
  for g in "${graphs[@]}"; do
    [ -f "$TEX/$g.png" ] || { echo "fps-game-demo has no export of graph $g ($TEX/$g.png)" >&2; exit 1; }
    for suffix in "" _normal _orm _emission; do
      if [ -f "$TEX/$g$suffix.png" ]; then cp "$TEX/$g$suffix.png" "$RAW/"; fi
    done
  done
  rev="$(git -C "$FPS" rev-parse --short HEAD 2>/dev/null || echo unknown)"
  echo "fps-game-demo $rev game/textures (its committed Material Maker exports, 1024 px)" > "$RAW/SOURCE.txt"
else
  MM="${MATERIAL_MAKER_DIR:?set MATERIAL_MAKER_DIR to your Material Maker folder, or use --from-fps <fps-game-demo dir>}"
  files=()
  for g in "${graphs[@]}"; do
    [ -f "$HERE/ptex/$g.ptex" ] || { echo "no graph tools/materials/ptex/$g.ptex" >&2; exit 1; }
    files+=("$HERE/ptex/$g.ptex")
  done
  if [ -x "$MM/material_maker.x86_64" ]; then
    "$MM/material_maker.x86_64" --export-material -o "$RAW" "${files[@]}"
  else
    # Material Maker from a source checkout, run by a Godot 4.7 binary.
    "${GODOT:-godot}" --path "$MM" --rendering-driver vulkan --export-material -o "$RAW" "${files[@]}"
  fi
  echo "Material Maker render of tools/materials/ptex ($MM)" > "$RAW/SOURCE.txt"
fi

python3 "$HERE/postprocess.py" --raw "$RAW"
