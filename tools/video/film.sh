#!/usr/bin/env bash
# The viewscreen cameras' video (openspec/changes/chase-camera design 3): the frames that
# `sc-client --headless --solo --station captain --film --shots DIR` writes, made into an MP4 at 30 frames a second
# (the drill's headless step), each shot captioned with what it shows from the cues the film writes beside its frames
# (`film-cues.txt`: a shot's first frame, a tab, its caption).
#
# Usage: tools/video/film.sh DIR OUT.mp4
# FPS (default 30) is the frames per second the run wrote (`--record-fps`); EXT (png or jpg, default png) is the
# frames' file type, jpg when they were compressed as the run went to save disk.
set -euo pipefail
DIR="${1:?frames folder}"
OUT="${2:?output mp4}"
FONT="$(cd "$(dirname "$0")/../.." && pwd)/assets/fonts/barlow-semi-condensed/semibold-600.ttf"
[ -f "$FONT" ] || FONT=/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf
FPS="${FPS:-30}"
EXT="${EXT:-png}"
LAST=$(ls "$DIR"/film-*."$EXT" | wc -l)
# Each caption is read from its own file (drawtext's textfile), so its commas, colons and quotes need no escaping.
TXT="$(mktemp -d)"
trap 'rm -rf "$TXT"' EXIT
FILTER=""
mapfile -t CUES < "$DIR/film-cues.txt"
for i in "${!CUES[@]}"; do
  first="${CUES[$i]%%$'\t'*}"
  printf '%s' "${CUES[$i]#*$'\t'}" > "$TXT/$i.txt"
  next="${CUES[$((i + 1))]:-}"
  if [ -n "$next" ]; then last=$(( ${next%%$'\t'*} - 1 )); else last=$LAST; fi
  # ffmpeg counts frames from 0, the film from 1.
  FILTER+="${FILTER:+,}drawtext=fontfile=$FONT:textfile=$TXT/$i.txt:x=40:y=h-96:fontsize=36:fontcolor=white"
  FILTER+=":box=1:boxcolor=0x000000AA:boxborderw=14:enable='between(n\,$((first - 1))\,$((last - 1)))'"
done
ffmpeg -y -loglevel error -framerate "$FPS" -i "$DIR/film-%05d.$EXT" -vf "$FILTER,format=yuv420p" -c:v libx264 -crf 20 \
  -preset medium -movflags +faststart "$OUT"
echo "tools/video/film.sh: wrote $OUT ($LAST frames, ${#CUES[@]} shots)"
