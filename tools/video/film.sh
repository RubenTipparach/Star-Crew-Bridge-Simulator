#!/usr/bin/env bash
# The viewscreen cameras' video (openspec/changes/chase-camera design 3): the frames that
# `sc-client --headless --solo --station captain --film --shots DIR` writes, made into an MP4 at 30 frames a second
# (the drill's headless step), each shot captioned with what it shows from the cues the film writes beside its frames
# (`film-cues.txt`: a shot's first frame, a tab, its caption).
#
# Usage: tools/video/film.sh DIR OUT.mp4
set -euo pipefail
DIR="${1:?frames folder}"
OUT="${2:?output mp4}"
FONT="$(cd "$(dirname "$0")/../.." && pwd)/assets/fonts/barlow-semi-condensed/semibold-600.ttf"
[ -f "$FONT" ] || FONT=/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf
LAST=$(ls "$DIR"/film-*.png | wc -l)
FILTER=""
mapfile -t CUES < "$DIR/film-cues.txt"
for i in "${!CUES[@]}"; do
  first="${CUES[$i]%%$'\t'*}"
  text="${CUES[$i]#*$'\t'}"
  next="${CUES[$((i + 1))]:-}"
  if [ -n "$next" ]; then last=$(( ${next%%$'\t'*} - 1 )); else last=$LAST; fi
  # ffmpeg counts frames from 0, the film from 1.
  text="${text//:/\\:}"
  text="${text//\'/\\\\\'}"
  FILTER+="${FILTER:+,}drawtext=fontfile=$FONT:text='$text':x=40:y=h-96:fontsize=36:fontcolor=white:box=1:boxcolor=0x000000AA:boxborderw=14:enable='between(n,$((first - 1)),$((last - 1)))'"
done
ffmpeg -y -loglevel error -framerate 30 -i "$DIR/film-%05d.png" -vf "$FILTER,format=yuv420p" -c:v libx264 -crf 20 \
  -preset medium -movflags +faststart "$OUT"
echo "tools/video/film.sh: wrote $OUT ($LAST frames, ${#CUES[@]} shots)"
