#!/usr/bin/env bash
# Push a browser build folder to itch.io with butler (https://itch.io/docs/butler/).
#
# Usage: tools/web/push_itch.sh [dir, default build/web]
# ITCH_TARGET (user/game) defaults to the game's page, https://ruben-tipparach.itch.io/star-crew-bridge-simulator
# (owner, 2026-10-08). Needs BUTLER_API_KEY or a prior `butler login`. butler is
# fetched into build/butler/ when it is not on PATH. The version shown on itch.io is the commit.
set -euo pipefail
cd "$(dirname "$0")/../.."
DIR="${1:-build/web}"
ITCH_TARGET="${ITCH_TARGET:-ruben-tipparach/star-crew-bridge-simulator}"
[ -f "$DIR/index.html" ] || { echo "tools/web/push_itch.sh: $DIR/index.html missing; run tools/web/build.sh" >&2; exit 1; }
BUTLER="$(command -v butler || true)"
if [ -z "$BUTLER" ]; then
  mkdir -p build/butler
  if [ ! -x build/butler/butler ]; then
    curl -fsSL -o build/butler/butler.zip https://broth.itch.zone/butler/linux-amd64/LATEST/archive/default
    (cd build/butler && unzip -o -q butler.zip && chmod +x butler)
  fi
  BUTLER=build/butler/butler
fi
"$BUTLER" -V
VERSION="$(git rev-parse --short HEAD 2>/dev/null || echo local)"
"$BUTLER" push "$DIR" "$ITCH_TARGET:html5" --userversion "$VERSION"
