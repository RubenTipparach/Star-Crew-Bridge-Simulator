# Design: the engine stack

Status: **proposed** (2026-10-04); the language and stack are **decided** by the owner
(E1, below). Nothing here is built. The budget table in section 5 is
provisional until `sc-probe` measures it on a Pi 3 (section 11).

## 1. The target: Raspberry Pi 3 Model B

| Part | What it is | What it means for us |
| --- | --- | --- |
| CPU | 4 x ARM Cortex-A53, 1.2 GHz (the 3B+ runs at 1.4 GHz) | In-order cores with small caches: data-oriented arrays, no pointer chasing, one core per job (render, server, network, audio). |
| RAM | 1 GB LPDDR2, shared with the GPU | OS, GPU buffers, client and (when hosting) server all fit in 1 GB. Every process has a fixed allocation (section 5). |
| GPU | Broadcom VideoCore IV, a tile-based renderer | **OpenGL ES 2.0 is the ceiling.** Vertex and fragment shaders in GLSL ES 1.00. Tile binning makes triangle count and overdraw the costs to watch. |
| GL driver | Mesa `vc4` under the KMS display driver | GLES 2.0 and desktop OpenGL 2.1 through EGL. No ES 3.0, no Vulkan (Vulkan arrived with the Pi 4's `v3dv`). |
| Display | HDMI, up to 1920 x 1080 | We render 1280 x 720. |
| Network | 100 Mbit/s Ethernet (over USB 2.0), 2.4 GHz 802.11n Wi-Fi | Bandwidth is plentiful for us; Wi-Fi jitter is the risk (`netcode-and-sessions`). |
| Storage | microSD | Loads are slow and random reads are slower: one compiled file per ship, read sequentially. |

The model (3B or 3B+) changes the CPU numbers by 17%. The probe reports which it ran on.

Published reference point, to verify with the probe: Quake III Arena (ioquake3) has run on the
original Raspberry Pi's VideoCore IV at 1080p, and the Pi 3 has the same GPU family at higher
clocks with a much faster CPU. A Quake III level is lightmapped brush geometry of tens of
thousands of triangles per view; our decks are vertex-lit brush geometry of a similar scale,
which is why the budget below starts where it does.

## 2. Options considered

| Option | Draws on a Pi 3? | Runs without a desktop? | Decoupled interior and exterior | Safety for network code | Headless, tested core | Owner has used it | Verdict |
| --- | --- | --- | --- | --- | --- | --- | --- |
| **Rust + SDL2 + glow (GLES 2.0)** | Yes (ES 2.0) | Yes (SDL2 KMS/DRM) | Ours to design | Memory safe | `cargo test`, as Pale-Blue-Dot | Rust (Pale-Blue-Dot) | **Recommended** |
| C11 + raylib | Yes (raylib's GLES2 path) | Yes (raylib's DRM platform) | Ours to design | Manual | Possible, by discipline | C (star-crew-64) | Runner-up |
| C++ + SDL2 + bgfx | Yes, with caveats (to verify) | Yes (SDL2) | Ours to design | Manual | Possible | No | Heavier build, little gain |
| Rust + miniquad | Yes (GLES2 / WebGL1) | No KMS/DRM backend (to verify) | Ours to design | Memory safe | Yes | No | Would need a desktop session on the Pi |
| Bevy | **No**: wgpu's GL backend needs ES 3.0 | | Engine scene graph | Memory safe | Yes | Yes (Pale-Blue-Dot) | Cannot run |
| WebGPU (native wgpu, or a browser) | **No**: needs Vulkan (or ES 3.0 for wgpu's GL fallback); the Pi 3 has neither. Vulkan arrives with the Pi 4 (Mesa `v3dv`) | | | | | | Cannot run on the floor; possible from a Pi 4 up |
| Godot 4 | **No**: Compatibility renderer needs GL 3.3 / ES 3.0 | | Engine scene graph | | | Yes (Undercity) | Cannot run |
| Godot 3 (GLES2) | Yes, slowly (to verify) | Community ports only | Fights one scene tree | | | Related | Not custom; 1 GB is tight |
| three.js in Chromium | WebGL 1, slowly | No | Ours | | | Yes (mockups) | Browser takes too much of 1 GB |

**Why Rust over C.** Both meet the hard requirement; the difference is in what the owner's
other projects taught.

- Pale-Blue-Dot's rules (an engine-independent core crate, `clippy -D warnings`, tests that
  read as sentences, validated data with units) are Rust-shaped and already work. Rust makes
  "an unknown data key is an error" one attribute (`#[serde(deny_unknown_fields)]`).
- The server parses packets from the network. In Rust a malformed packet is an error value; in
  C it is a memory bug. star-crew-64 ended with an unresolved crash on real hardware.
- The ship simulation must be tested headless and replayed deterministically; `cargo test` on
  a plain library crate gives that from the first day.

What Rust costs: longer compile times (on the desktop only; the Pi never compiles), and
cross-compiling SDL2 for the Pi needs a Raspberry Pi OS sysroot (section 10). C with raylib
would reach first pixels sooner and could even compile on the Pi itself. If the owner prefers
C, everything else in this design holds: the crate boundaries become library boundaries, and
the budget, the renderer floor and the probe are unchanged.

**Decided (E1):** Rust + SDL2 + glow. The owner, 2026-10-04: "That's fine..this game doesn't need high end graphics. Your stack sounds like a solid plan".

## 3. Architecture

```text
            data/ (JSON, validated)        assets/ (PNG, generated meshes)
                   |                                   |
                   v                                   v
   sc-tools (deckc, meshc: offline, desktop) ---> compiled/ (.deck, .mesh)
                   |
                   | (uses sc-core's compartment graph: one implementation)
                   v
 +------------------------------------------------------------------+
 | sc-core   the ship simulation. No I/O, no GL, no sockets, no SDL. |
 |   frames, compartment graph, power, life support, heat, damage,   |
 |   weapons, craft, flight, crew state, stations, automation        |
 +------------------------------------------------------------------+
        ^                        ^                          ^
        |                        |                          |
   sc-net (messages,        sc-server (headless:       sc-render (GLES 2.0:
   transport, deltas)       core + net + saves)        passes, decks, culling, UI draw)
        ^                                                   ^
        +---------------------- sc-client -----------------+
                    (SDL2: window, input, audio; prediction,
                     interpolation, console UIs, the frame loop)

   sc-probe: a standalone binary that measures the Pi (section 11)
```

Dependencies only point up this picture. `sc-core` depends on nothing of ours. `sc-render`
reads core types but never mutates simulation state. `sc-client` can host `sc-server` on a
thread (a listen server); a solo game is exactly that (CLAUDE.md 6.3).

| Crate | Owns | Must never |
| --- | --- | --- |
| `sc-core` | Every gameplay rule, fixed-step at 30 Hz; data schemas and validation; stable ids; seeded randomness; replay hashing | Read files, open sockets, call GL or SDL, read the clock |
| `sc-net` | Message types (one schema for both ends), the UDP transport, reliability, delta compression | Contain gameplay rules |
| `sc-server` | The authoritative loop, sessions and seats, saves | Render, play audio |
| `sc-render` | GL state, shaders, vertex formats, passes, portal culling, text and panel drawing | Decide gameplay outcomes |
| `sc-client` | Platform (SDL2), input mapping, prediction, interpolation, console UI logic, audio mixing | Resolve a gameplay outcome itself (it previews by calling `sc-core`) |
| `sc-tools` | `deckc` (decks), `meshc` (meshes), validators | Ship in the game build |
| `sc-probe` | Measuring the Pi | Share code paths with the game it does not need (it links `sc-render`) |

## 4. What we borrow and what we build

| Need | Choice | Why |
| --- | --- | --- |
| Window, GL context, input, gamepads, audio out | SDL2 (`sdl2` crate, system library) | KMS/DRM backend on the Pi, X11/Wayland/Windows/macOS on desktops, the GameController database for pads. |
| GL bindings | `glow` | Thin and unopinionated; we own every GL call. |
| Maths | `glam` (f32 and f64 types) | Small, SIMD on aarch64 NEON, both precisions for the frames rule. |
| Data | `serde`, `serde_json` with `deny_unknown_fields` | Validated JSON data with units in keys (CLAUDE.md 6.5). |
| Wire and save format | `postcard` (compact binary over serde) | Small packets; one schema crate for both ends. |
| Images | `png` (tools and load time) | PNG sources are committed (CLAUDE.md 9). |
| Audio decode | `lewton` (Ogg Vorbis) | Small, pure Rust. |
| Logging | `log` with a tiny logger | Nothing heavier on the Pi. |
| ECS | **None.** Typed arenas with generational ids | Tens of crew and craft, hundreds of projectiles: plain arrays are faster on an A53 and easier to reason about. |
| Physics | **Ours.** Capsule against convex brushes in the interior; spheres, capsules and rays outside | Two small, specific problems. A general physics engine would cost memory and give us a single world we would then have to split. |
| Renderer | **Ours**, on GLES 2.0 | Nothing borrowable targets this floor and our pass structure. |
| UI | **Ours**: an immediate-mode panel and text layer on one bitmap font atlas | Fixed-size console panels (CLAUDE.md 10) at a known cost; `egui` (with `egui_glow`) is measured by the probe as an alternative (question E3). |
| Networking | **Ours**: UDP with reliable and unreliable channels | See `netcode-and-sessions`. |

## 5. The Pi 3 budget

<!-- pi3-budget triangles=50000 draw_calls=120 texture_mb=32 -->

**Provisional.** Estimates from published figures and from the scale of comparable games, to be
replaced by `sc-probe` measurements on a Pi 3. This table is the one source for these numbers
(CLAUDE.md section 2); the marker comment above is read by `tools/mockups/inline.py --check`
so the mockups' meter cannot drift from it.

| Budget | Value | Notes |
| --- | ---: | --- |
| Output resolution | 1280 x 720 | 3D may render at 960 x 540 and scale up if fill rate binds. |
| Frame time | 33.3 ms | 30 frames a second is the floor; 60 where it fits. |
| Visible triangles per frame, all passes | 50,000 | The number most likely to move after the probe. |
| Draw calls per frame, all passes | 120 | Driver overhead on an A53 is the cost. |
| Texture memory | 32 MB | Palette atlas, font atlas, decals, screens, the viewscreen target. |
| Vertex and index buffers | 24 MB | All decks of one ship resident; 16-bit indices. |
| Viewscreen render target | 512 x 256, one, at most 30 Hz | About 0.75 MB with depth. |
| Average overdraw | 2.5x | Glass, particles and UI over the scene. |
| Client main thread, CPU | 20 ms | Culling and submission 8, UI 3, prediction and interpolation 3, the rest slack. |
| Server tick | 30 Hz, at most 4 ms per ship on one core | Ship systems sub-step at 10 Hz inside it. |
| Client resident memory | 256 MB | Allocated at startup by budget; refuses to load what does not fit. |
| Server resident memory | 64 MB | Including the listen server inside a client. |
| GPU allocation (CMA) | 128 MB | Textures, buffers, framebuffers. To set and verify on the Pi. |
| OS (Raspberry Pi OS Lite, no desktop) | about 150 MB | To measure. |
| Network, per client | 64 kbit/s down, 16 kbit/s up | `netcode-and-sessions`. |
| Install size | 200 MB | |
| Bridge compartment geometry | 8,000 triangles | The most detailed room. |
| Other compartment geometry | about 2,500 triangles each | `deck-pipeline` gives the per-compartment table. |
| Ship exterior | 3,000 triangles; 600 at the far LOD | |
| Fighter / missile | 400 / 60 triangles | |
| Crew avatar | 600 triangles, at most 24 bones | Skinned in the vertex shader. |
| Active exterior bodies near the ship | 64 | Ships, craft, missiles. |
| Projectiles | 256, as billboards in one dynamic buffer | One draw call. |
| Audio voices | 16 | Mixed on SDL's audio thread. |

A worked frame on the bridge, to show the table is coherent: exterior pass (stars 1 call, planet
1, two enemy ships 2, projectiles 1: about 3,500 triangles), viewscreen target (the same scene
from another camera at 512 x 256, 15 Hz: about 3,500 triangles every other frame), interior
pass (the bridge and the two compartments seen through its aft door: about 13,000 triangles
in 12 calls, crew avatars 4 x 600 in 4 calls), glass 2 calls, UI 6 calls. About 22,000
triangles and 30 calls: well inside the budget, which leaves room for the probe to cut it.

## 6. Frame and tick structure

- **Threads on the client:** main (input, prediction, render, UI), network (socket I/O,
  packet assembly), SDL's audio thread, and, when hosting, the server thread. Four threads on
  four cores.
- **Server loop:** fixed 30 Hz tick. Each tick applies queued commands in a stable order, steps
  flight and weapons, and every third tick steps the ship systems (power, atmosphere, heat,
  fire) at 10 Hz. Snapshots go out at 20 Hz.
- **Client loop:** renders at the display rate (vsync), interpolating other entities 100 ms in
  the past and predicting its own avatar or fighter from inputs not yet acknowledged.
- **Time is seconds,** carried as `f64` simulation time and integer tick numbers. Nothing in
  `sc-core` counts frames (star-crew-64's lesson).

## 7. The renderer's floor

**Allowed:** OpenGL ES 2.0 core, GLSL ES 1.00 (`#version 100`, or `#version 120` with a small
define prelude on desktop GL 2.1), 16-bit indices, RGBA8 and RGB565 textures, one depth
buffer (16- or 24-bit), stencil if the probe finds it cheap, render-to-texture through a
framebuffer object with a colour texture and a depth renderbuffer.

**Not assumed:** instancing, multiple render targets, float textures or targets, 3D textures,
texture arrays, compute, geometry shaders, 32-bit indices, sRGB framebuffers, MSAA (an
extension; measured, off by default).

**Shaders (about six programs):**
1. Interior: position, three baked vertex colour sets blended by per-compartment uniforms
   (normal, red alert, emergency), palette atlas lookup, fog.
2. Exterior lit: per-vertex Lambert from the sun plus ambient, flat-shaded by unshared normals.
3. Unlit and emissive: screens, engine glow, lamps.
4. Billboards: projectiles, sparks, markers, in one dynamic buffer.
5. Stars: points.
6. UI: textured quads for text and panels.

**Interior vertex format, 28 bytes:** position 3 x int16 in centimetres (plus padding), normal
3 x int8, three colour sets 3 x RGBA8, atlas UV 2 x uint16. Flat shading means unshared
vertices: 50,000 triangles cost about 4.2 MB.

**Keeping draw calls down:** static geometry is merged per compartment per material at compile
time (one palette atlas means most compartments are one or two calls); doors and other moving
parts are separate; projectiles and particles are rebuilt into one buffer per frame; UI is
batched by texture. The frame is drawn exterior first, then interior, then glass, then UI,
sorted by program within each pass.

## 8. Memory

- **Sized at startup.** The client computes its pools from the ship it loads and the budget
  table, allocates them once and refuses to start if they exceed it, naming what did not fit.
- **Arenas and pools.** A per-frame bump arena for transient data; fixed pools with
  generational ids for crew, craft, projectiles, particles and network entities.
- **No per-frame heap allocation** in steady state, checked by a counting allocator in debug
  builds.

## 9. Data and assets

- **Authored:** JSON under `data/` (ships, tuning tables), PNG under `assets/`, generator
  scripts under `tools/`. Validated on load and by tests (CLAUDE.md 6.5).
- **Compiled:** `deckc` turns a ship's layout and brushes into one `.deck` file
  (`deck-pipeline`); `meshc` turns generated meshes into `.mesh`. Both are little-endian,
  chunked and versioned with a header and checksum, read with one sequential pass.
- **One schema.** Data types live in `sc-core`; the tools and the game use the same structs.

## 10. Platforms, build and deployment

| Target | Rust target | Display | Status |
| --- | --- | --- | --- |
| Raspberry Pi 3, Raspberry Pi OS Lite 64-bit | `aarch64-unknown-linux-gnu` | SDL2 KMS/DRM, full screen, no desktop | The floor |
| Raspberry Pi 3, 32-bit OS | `armv7-unknown-linux-gnueabihf` | Same | Measured by the probe as a fallback if memory binds (question E2) |
| Linux desktop | `x86_64-unknown-linux-gnu` | SDL2, GLES 2.0 or GL 2.1 | Development and play |
| Windows | `x86_64-pc-windows-msvc` | SDL2, GL 2.1 | Play |
| macOS | `aarch64-apple-darwin` | SDL2, GL 2.1 (deprecated by Apple, still working) | Best effort |

- **Cross-compiling for the Pi** uses a Raspberry Pi OS sysroot with SDL2, libdrm, GBM and EGL
  development files, through `cross` with a custom image or `cargo zigbuild` with the
  sysroot. `-C target-cpu=cortex-a53`. The Pi never compiles.
- **On the Pi** the game is a systemd service or an autologin launch on the console with
  `SDL_VIDEODRIVER=kmsdrm`, the KMS overlay (`vc4-kms-v3d`) enabled and the user in the
  `video`, `render`, `input` and `audio` groups. A deploy script copies the binary, `data/` and
  `compiled/` over SSH.
- **A web build** (WebGL 1 is ES 2.0) is possible later but is out of scope: it needs a
  different platform layer.

## 11. The probe: measuring before building

`sc-probe` is a measurement instrument (CLAUDE.md section 4) and the first thing built. It
links `sc-render` and draws synthetic scenes full-screen at 1280 x 720 on the Pi:

1. **Triangle curve:** flat-shaded, vertex-coloured triangles in the interior format, 10,000 to
   150,000 in steps, in 30 calls. Frame time p50, p95 and p99 at each step.
2. **Draw-call curve:** 30,000 triangles split into 10 to 400 calls.
3. **Fill rate:** full-screen quads, overdraw 1x to 6x, with the interior shader and with the
   unlit shader.
4. **Render to texture:** the viewscreen target at 256 x 128, 512 x 256 and 1024 x 512.
5. **UI:** 200 panels and 4,000 glyphs with our UI layer, and the same with `egui`.
6. **Memory:** resident size of the probe with the Tern's compiled deck loaded, on the 64-bit
   and 32-bit OS.
7. **Simulation (once `sc-core` exists):** ticks per second of the Tern's systems on one core.

It writes a JSON report and a short Markdown summary to `docs/benchmarks/<date>-pi3-probe/`,
naming the board model, OS, clocks, resolution, build and repeats, with the spread between
repeats (CLAUDE.md 12). The budget table is then corrected from it in the same commit. The
owner runs it on a Pi 3; a cloud session cannot.

## 12. Verification

`scripts/check.sh` runs, in order, stopping at the first failure:

1. `cargo fmt --all -- --check`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace` (the core's tests run headless)
4. `openspec validate --all`
5. the dash check (CLAUDE.md 5)
6. `python3 tools/layout_check.py`
7. `python3 tools/mockups/inline.py --check`
8. `deckc --check` for every ship (once it exists)

Render tests capture frames headless through Mesa's software GLES 2.0 (`llvmpipe` over EGL)
and compare them to committed references with a tolerance. They prove the passes compose; they
do not prove anything about the Pi's speed.

## 13. Risks and trade-offs

- **The triangle budget may be too high.** If the probe finds the Pi 3 binding well below
  50,000 triangles, decks get simpler and the viewscreen refreshes less often. The mockups show
  every design's spend so the cut can be made where it hurts least.
- **SDL2's KMS/DRM path on a Pi 3 is the platform risk.** The probe is its first test; raylib's
  DRM platform is the fallback for the window and context only.
- **Rust cross-compilation needs a sysroot.** A Docker image with the Pi OS sysroot is a task,
  not a research problem, but it has to exist before the first Pi build.
- **Our own UI and physics** are more code than a library, but small, specific and measurable.
- **macOS OpenGL is deprecated.** It still works; if it stops, macOS drops to best effort.

## Open questions

Per CLAUDE.md section 13, only E1 went to the owner (a fork in the road that changes every
later change); the owner answered it in chat. The others take the recommendation and are
recorded as "recommendation taken (ask only with screenshots)".

| # | Question | Options | Recommendation | Status |
| --- | --- | --- | --- | --- |
| E1 | The engine's language. Rust matches Pale-Blue-Dot's rules and is memory safe for network code; C with raylib reaches first pixels sooner and matches star-crew-64. Both meet the Pi 3 floor. | Rust + SDL2 + glow / C11 + raylib | Rust + SDL2 + glow | **Decided 2026-10-04: Rust + SDL2 + glow** (owner: "Your stack sounds like a solid plan") |
| E2 | 64-bit or 32-bit Raspberry Pi OS. 64-bit has better code generation; 32-bit uses less memory for pointers. | 64-bit / 32-bit | 64-bit, with 32-bit measured by the probe | Recommendation taken (ask only with screenshots) |
| E3 | Console UI library. Ours costs code; `egui` costs CPU on an A53. | Ours / egui | Ours, with egui measured by the probe | Recommendation taken (ask only with screenshots) |
| E4 | Which Pi 3 the owner will test on (3B at 1.2 GHz or 3B+ at 1.4 GHz). | 3B / 3B+ | Budget against the 3B | Recommendation taken; the probe reports the model |
