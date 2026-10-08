#!/usr/bin/env bash
# The browser build (openspec/changes/engine-stack design 10a): sc-client for wasm32-unknown-emscripten,
# with the page and the compressed deck, into one folder a static host serves as it is.
#
# Usage: tools/web/build.sh [out dir, default build/web]
# Needs: the Emscripten SDK on PATH (source <emsdk>/emsdk_env.sh), `rustup target add
# wasm32-unknown-emscripten`, and compiled/tern.deck (`sc-tools deckc`). Writes:
#   index.html (web/index.html), sc-client.js, sc_client.wasm, tern.deck.gz
set -euo pipefail
cd "$(dirname "$0")/../.."
OUT="${1:-build/web}"
command -v emcc >/dev/null || { echo "tools/web/build.sh: emcc not found; source <emsdk>/emsdk_env.sh" >&2; exit 1; }
[ -f compiled/tern.deck ] || { echo "tools/web/build.sh: compiled/tern.deck missing; run sc-tools deckc" >&2; exit 1; }
cargo build -p sc-client --target wasm32-unknown-emscripten --release
TARGET="${CARGO_TARGET_DIR:-target}/wasm32-unknown-emscripten/release"
mkdir -p "$OUT"
cp web/index.html "$OUT/index.html"
cp "$TARGET/sc-client.js" "$OUT/sc-client.js"
cp "$TARGET/sc_client.wasm" "$OUT/sc_client.wasm"
gzip -9 -n -c compiled/tern.deck > "$OUT/tern.deck.gz"
ls -l "$OUT"
