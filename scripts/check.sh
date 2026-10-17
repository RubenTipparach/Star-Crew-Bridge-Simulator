#!/usr/bin/env bash
# Every check, in order, stopping at the first failure (openspec/changes/engine-stack, design
# section 12; Pale-Blue-Dot's check_all). Run it before every push.
#
# 1-3 are the engine's: formatting, lints with warnings as errors, and the workspace's tests (the
# core's run headless; the render tests need an OpenGL ES 3.0 context, which a cloud session gets
# from Mesa's llvmpipe through EGL). 4-7 are the repository's. 8 onwards: the generated shader
# modules match their sources, the engine data and the lighting data are valid. 11-13 hold the
# consoles to their approved mockup (openspec/changes/console-parity): the fonts and the style data
# are what the mockup says, and the engine draws each of the mockup's saved states within the limits. 14 holds the
# screens around the consoles to the UI art direction (openspec/changes/ui-art-direction design 9 and 14): the style
# data is its mockup's, no screen shows a forbidden string or text over text, and every screen is shot at two sizes.
set -euo pipefail
cd "$(dirname "$0")/.."

step() { printf '\n== %s\n' "$1"; }

step "1. cargo fmt"
cargo fmt --all -- --check
step "2. cargo clippy (warnings are errors)"
cargo clippy --workspace --all-targets -- -D warnings
step "3. cargo test"
cargo test --workspace
step "4. openspec validate"
openspec validate --all
step "5. no em or en dashes (CLAUDE.md 5)"
if LC_ALL=C.UTF-8 grep -rnIP '\x{2014}|\x{2013}' --exclude-dir=.git --exclude-dir=.claude --exclude-dir=target --exclude-dir=third_party . ; then
  echo "FAIL: dashes found above"; exit 1
fi
echo ok
step "6. ship layouts"
python3 tools/layout_check.py
# The window patch is current and every window meets the exterior model's aperture (tern-exterior design 6).
python3 tools/exterior_windows.py --check
step "7. mockups hold the current layout"
python3 tools/mockups/inline.py --check
python3 tools/ships/console_model.py --check
step "8. shader modules match their sources"
python3 tools/sokol_shaders.py --check
step "9. engine data"
cargo run -q -p sc-tools -- check-data .
python3 tools/power/boxes.py --check
step "10. lighting data"
python3 tools/lighting_check.py
step "11. console fonts are the mockup's"
python3 tools/ui/fonts.py --check
step "12. console style data is the mockup's, and the screens' style is the art direction's"
python3 tools/ui/console_style.py --check
python3 tools/ui/ui_style.py --check
step "13. console parity: the engine draws the mockup's states"
cargo build -q -p sc-client
parity=$(mktemp -d)
trap 'rm -rf "$parity"' EXIT
cp docs/screenshots/parity/*-mockup.png "$parity"/
for state in docs/screenshots/parity/*.json; do
  [ -f "${state%.json}-mockup.png" ] || continue
  cp "$state" "$parity"/
  target/debug/sc-client --headless --console-fixture "$parity/$(basename "$state")" --shots "$parity" >/dev/null
done
python3 tools/consoles/compare.py "$parity" --max-pct 2.5 --max-missing 10
step "14. release check: the screens around the consoles (ui-art-direction design 14)"
# 14a: every reachable screen laid out headless at 1280 x 720 and 1920 x 1080, held to data/ui/forbidden_strings.json,
# no text over text, none off the screen or cut (crates/sc-client/src/release.rs; also run by step 3).
cargo test -q -p sc-client --bin sc-client release
# 14b: the scripted walk through every screen at both sizes, the front menu's pages and then the game's, captured
# (docs/screenshots/engine/release/ holds the committed set; here they go to a scratch folder and must all be written).
cargo build -q --release -p sc-client
release=$(mktemp -d)
trap 'rm -rf "$parity" "$release"' EXIT
for size in 1280x720 1920x1080; do
  mkdir -p "$release/$size"
  target/release/sc-client --headless --size "$size" --lobby-test "$release/$size" >/dev/null
  target/release/sc-client --headless --size "$size" --solo --ui-shots "$release/$size" >/dev/null
  for shot in menu-1-main menu-2-multi menu-3-options menu-4-options-game menu-5-controls-foot \
      menu-6-controls-console menu-7-controls-touch lobby-1 lobby-3-loading 1-loading 2-hud-first-steps \
      3-pause-walking 4-prompt 5-briefing 6-pause-seated 7-pause-leave 8-options 9-controls-foot \
      10-controls-console 11-controls-touch 12-console; do
    [ -s "$release/$size/$shot.png" ] || { echo "FAIL: $size: no $shot.png"; exit 1; }
  done
done
echo "ok: 21 screens shot at 1280 x 720 and 1920 x 1080"

printf '\nall checks passed\n'
