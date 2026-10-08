#!/usr/bin/env bash
# The browser playtest build, from the repository to itch.io (openspec/changes/engine-stack design 10a).
# Every step a developer can run by hand; the CI workflow (.github/workflows/web.yml) only installs the
# toolchains and calls this.
#
#   1. export the deck from the deck plan (tools/deck/export_deck.mjs, headless Chromium)
#   2. compile it (sc-tools deckc) into compiled/tern.deck
#   3. build the client for the browser (tools/web/build.sh) into build/web/
#   4. push build/web/ to itch.io with butler, when BUTLER_API_KEY and ITCH_TARGET are set
#
# Usage: tools/web/release.sh [--no-export] [--no-push]
#   ITCH_TARGET   the itch.io page, user/game (the channel html5 is added); defaults to
#                 ruben-tipparach/star-crew-bridge-simulator
#   BUTLER_API_KEY  the butler key (a repository secret in CI; `butler login` by hand)
set -euo pipefail
cd "$(dirname "$0")/../.."
export_deck=1; push=1
for a in "$@"; do
  case "$a" in
    --no-export) export_deck=0 ;;
    --no-push) push=0 ;;
    *) echo "tools/web/release.sh: unknown option $a" >&2; exit 2 ;;
  esac
done
if [ "$export_deck" = 1 ]; then
  node tools/deck/export_deck.mjs
  cargo run -q --release -p sc-tools -- deckc tern
fi
tools/web/build.sh build/web
if [ "$push" = 1 ]; then
  tools/web/push_itch.sh build/web
fi
