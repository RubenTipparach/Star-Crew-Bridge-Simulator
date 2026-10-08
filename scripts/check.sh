#!/usr/bin/env bash
# Every check, in order, stopping at the first failure (openspec/changes/engine-stack, design
# section 12; Pale-Blue-Dot's check_all). Run it before every push.
#
# 1-3 are the engine's: formatting, lints with warnings as errors, and the workspace's tests (the
# core's run headless; the render tests need an OpenGL ES 3.0 context, which a cloud session gets
# from Mesa's llvmpipe through EGL). 4-7 are the repository's. 8 onwards: the generated shader
# modules match their sources, the engine data and the lighting data are valid.
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
step "7. mockups hold the current layout"
python3 tools/mockups/inline.py --check
step "8. shader modules match their sources"
python3 tools/sokol_shaders.py --check
step "9. engine data"
cargo run -q -p sc-tools -- check-data .
step "10. lighting data"
python3 tools/lighting_check.py

printf '\nall checks passed\n'
