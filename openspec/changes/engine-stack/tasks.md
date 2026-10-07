# Tasks

Nothing here starts until the owner asks for implementation (CLAUDE.md section 4). Task 2 is a
measurement instrument and comes first.

## 1. Decide

- [x] 1.1 Owner answers question E1 (Rust or C); fold the answer into this design. Answered 2026-10-04: Rust + SDL + glow; SDL3 chosen over SDL2 the same day for the Pi 5's atomic KMS/DRM path.
- [x] 1.2 Re-floor the design on the owner's hardware correction (2026-10-04): a 1 GB Pi 5 client, a 4 GB Pi 5 main server, OpenGL ES 3.0.
- [x] 1.3 Record E3 as "recommendation taken (ask only with screenshots)", E4 and E5 as decided by the owner, and E2 (glow or wgpu) as open until probe scene 8.
- [x] 1.4 Owner decides E2, 2026-10-07: sokol_gfx with SDL3 ("sokol it is then", "with sdl3"); fold it into the design (sections 2, 4, 7, 10, 11, 13). Record E6 (the browser playtest build) as "recommendation taken (ask only with screenshots)".

## 2. Measure the Pi (the probe)

- [ ] 2.1 Workspace skeleton: `Cargo.toml` with `sc-core`, `sc-net`, `sc-render`, `sc-client`, `sc-server`, `sc-tools`, `sc-probe`; `rustfmt.toml`; workspace lints with warnings as errors.
- [ ] 2.2 Cross-compilation: a Raspberry Pi OS (64-bit) sysroot with libdrm, GBM, EGL, GLESv2 and SDL3's build dependencies (SDL 3.4+ built from source through `sdl3-src`), and a C cross-compiler for sokol_gfx; `cross` or `cargo zigbuild` config for `aarch64-unknown-linux-gnu` with `-C target-cpu=cortex-a76`.
- [ ] 2.2a Vendor `sokol_gfx.h`, `sokol_log.h` and the generated `gfx.rs` and `log.rs` into `third_party/sokol/` at pinned revisions with a provenance file; `sc-render`'s build script compiles them (`SOKOL_GLES3` or `SOKOL_GLCORE`) and runs the pinned `sokol-shdc` over `sc-render/shaders/`.
- [ ] 2.3 `sc-render` minimum: SDL3 window and OpenGL ES 3.0 context (atomic KMS/DRM on the Pi, `SDL_KMSDRM_ATOMIC=1`) driven by SDL3's main callbacks, sokol_gfx set up on it with the budget's pool sizes, the deck shader and vertex format in sokol_gfx's terms (design section 7), a frame timer.
- [ ] 2.4 `sc-probe` scenes 1-6 and 8 (design section 11; scene 8 measures sokol_gfx's own cost) with a JSON and Markdown report.
- [ ] 2.4a Probe scene 9: the probe built for `wasm32-unknown-emscripten` (SDL3's Emscripten port, or `sokol_app` if it fails), opened in Chromium and Firefox; report the download size and that each scene draws.
- [ ] 2.5 Owner runs the probe on a 1 GB Pi 5; report in `docs/benchmarks/<date>-pi5-probe/`.
- [ ] 2.6 Correct the budget table (design section 5) and its marker from the report, with `docs/mockups/lib/shipkit.js` `PI_BUDGET` in the same commit.

## 3. Foundations

- [ ] 3.1 `scripts/check.sh` running design section 12's list in order.
- [ ] 3.2 `sc-core` skeleton: fixed-step clock in seconds, generational arenas, seeded random source (seed, stable id, purpose), replay hash.
- [ ] 3.3 Data loading in `sc-core`: serde types with `deny_unknown_fields`, units in keys, validation that stops startup naming the file and field; tests for a misspelt key and a non-finite value.
- [ ] 3.4 Startup memory sizing from the budget table; a debug counting allocator that fails a test on steady-state allocation.
- [ ] 3.5 Headless render tests through Mesa `llvmpipe` (EGL, OpenGL ES 3.0) with reference images.
- [ ] 3.6 Deploy: copy binaries, `data/` and `compiled/` to a Pi over SSH; a systemd unit for the full-screen client with `SDL_VIDEODRIVER=kmsdrm`, and one for `sc-server` that restarts on failure.

## 4. Move to specs

- [ ] 4.1 When 2.6 and 3.1 are done, move the `engine-platform` requirements that are then true into `openspec/specs/engine-platform/spec.md`, each with the check that proves it, and move the `pi-budget` marker with them.
