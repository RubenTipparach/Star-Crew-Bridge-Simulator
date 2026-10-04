# Design: the engine stack

Status: **proposed** (2026-10-04); the language and stack are **decided** by the owner (E1,
below). Nothing here is built. The budget table in section 5 is provisional until `sc-probe`
measures it on a Pi 5 (section 11).

**History.** This design was first written against a Raspberry Pi 3 (OpenGL ES 2.0, 1 GB). The
same day the owner corrected the target: "I'm sorry we're running on a pi5 1gb-4gb, 4 GB can be
used as main server too". The floor is now a Pi 5 with 1 GB as the client and a 4 GB Pi 5 as
the main server; the renderer floor rose from OpenGL ES 2.0 to 3.0, and every budget below was
re-estimated. The stack the owner approved (Rust, SDL2, glow) is unchanged.

## 1. The target: Raspberry Pi 5

| Part | What it is | What it means for us |
| --- | --- | --- |
| CPU | Broadcom BCM2712: 4 x ARM Cortex-A76 at 2.4 GHz, 512 KB L2 per core, 2 MB shared L3 | Out-of-order cores, several times a Pi 3's speed. One core per job (render, server, network, audio) still keeps the frame steady. |
| RAM | LPDDR4X, shared with the GPU. Client floor: **1 GB**. Main server: **4 GB** | On the 1 GB board the OS, the GPU's allocations and the client share 1 GB: every process has a fixed allocation (section 5). |
| GPU | Broadcom VideoCore VII, a tile-based renderer | OpenGL ES 3.1 and Vulkan through Mesa (`v3d`, `v3dv`). **Our floor is OpenGL ES 3.0** (section 7). Tile-based: MSAA is cheap, overdraw and bandwidth are the costs to watch. |
| Display | Two HDMI outputs, up to 4K at 60 Hz | We drive 1920 x 1080 and render 3D at 1280 x 720 by default. |
| Network | Gigabit Ethernet, dual-band 802.11ac Wi-Fi | The main server should be on Ethernet; clients on Wi-Fi are the jitter risk (`netcode-and-sessions`). |
| Storage | microSD, or NVMe through the PCIe 2.0 x1 connector | One compiled file per ship, read sequentially. |
| Cooling | Throttles when hot under sustained load (to verify the threshold) | The main server runs with the Active Cooler. |

Which Pi 5 the probe ran on (RAM size, clocks, cooling) goes in every report.

Published reference points, to verify with the probe: the Pi 5's GPU runs OpenGL ES 3.1 and
Vulkan 1.2 or later through Mesa, and ports of brush-built games of the Quake III and Doom 3
generation run on it at 720p and above. Our decks are vertex-lit, low-poly brush geometry, well
inside that class, which is why the budget below starts where it does.

## 2. Options considered

| Option | Draws on a Pi 5? | Runs without a desktop? | Decoupled interior and exterior | Safety for network code | Headless, tested core | Owner has used it | Verdict |
| --- | --- | --- | --- | --- | --- | --- | --- |
| **Rust + SDL2 + glow (OpenGL ES 3.0)** | Yes | Yes (SDL2 KMS/DRM) | Ours to design | Memory safe | `cargo test`, as Pale-Blue-Dot | Rust (Pale-Blue-Dot) | **Decided** |
| Rust + SDL2 + wgpu (Vulkan) | Yes, through Mesa `v3dv` (to verify) | Needs a Vulkan display surface without a desktop, or a small kiosk compositor (to verify) | Ours to design | Memory safe | Yes | WGSL and wgpu through Bevy | Possible; more driver surface and memory than a low-poly game needs |
| C11 + raylib | Yes (raylib's GLES path) | Yes (raylib's DRM platform) | Ours to design | Manual | By discipline | C (star-crew-64) | Runner-up; the owner chose Rust |
| Bevy | Yes, through wgpu and Vulkan (memory on 1 GB to verify) | As wgpu | One ECS world and one physics world to split | Memory safe | Yes | Yes (Pale-Blue-Dot) | Not chosen: the owner asked for a custom engine for the decoupling, and 1 GB is tight |
| Godot 4 | Yes, Compatibility renderer on ES 3 (performance to verify) | Community setups | One scene tree | | | Yes (Undercity) | Not chosen, for the same reasons |
| three.js in Chromium | WebGL 2 | No | Ours | | | Yes (mockups) | A browser takes too much of 1 GB |

**Why custom, now that the Pi 5 could run an off-the-shelf engine.** On a Pi 3 a custom engine
was forced; on a Pi 5 it is chosen. The owner wants it "because our ship needs to decouple the
bridge and internal ship aspects from the exterior of the ship" (`ship-frames`), the 1 GB client
leaves little room for an engine's general machinery, and the game needs a small, specific
renderer: low-poly, vertex-lit decks, portal culling, a space pass and a viewscreen.

**Why Rust over C.**

- Pale-Blue-Dot's rules (an engine-independent core crate, `clippy -D warnings`, tests that
  read as sentences, validated data with units) are Rust-shaped and already work. Rust makes
  "an unknown data key is an error" one attribute (`#[serde(deny_unknown_fields)]`).
- The server parses packets from the network. In Rust a malformed packet is an error value; in
  C it is a memory bug. star-crew-64 ended with an unresolved crash on real hardware.
- The ship simulation must be tested headless and replayed deterministically; `cargo test` on
  a plain library crate gives that from the first day.

What Rust costs: longer compile times, and cross-compiling SDL2 needs a Raspberry Pi OS sysroot
(section 10). A Pi 5 can compile the game itself, slowly; the desktop cross-compile is faster.

**Why glow on OpenGL ES 3.0 rather than wgpu, for now.** Both work on a Pi 5, and the probe
compares them on the real board before the renderer is built (section 11, scene 8). ES 3.0 is what a low-poly,
vertex-lit game needs; it runs full screen from the console through SDL2's KMS/DRM backend with
nothing else installed; it uses the least memory and driver surface on the 1 GB board; and it is
WebGL 2, so a browser client stays possible later through `glow`'s web backend. wgpu would bring
WGSL, Metal on macOS and Vulkan, at the cost of a heavier stack. `sc-render` keeps every GL call
behind its own interface, so the API can change later without touching the simulation
(question E2).

**Decided (E1):** Rust + SDL2 + glow. The owner, 2026-10-04: "That's fine..this game doesn't need
high end graphics. Your stack sounds like a solid plan".

## 3. Architecture

```text
            data/ (JSON, validated)        assets/ (PNG, generated meshes)
                   |                                   |
                   v                                   v
   sc-tools (deckc, meshc, bake: offline) -------> compiled/ (.deck, .mesh)
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
   sc-net (messages,        sc-server (headless:       sc-render (OpenGL ES 3.0:
   transport, deltas)       core + net + saves)        passes, decks, culling, UI draw)
        ^                                                   ^
        +---------------------- sc-client -----------------+
                    (SDL2: window, input, audio; prediction,
                     interpolation, console UIs, the frame loop)

   sc-probe: a standalone binary that measures the Pi (section 11)
```

Dependencies only point up this picture. `sc-core` depends on nothing of ours. `sc-render`
reads core types but never mutates simulation state. `sc-client` can host `sc-server` on a
thread (a listen server); a solo game is exactly that (CLAUDE.md 6.3). The main server is
`sc-server` on its own, on a 4 GB Pi 5.

| Crate | Owns | Must never |
| --- | --- | --- |
| `sc-core` | Every gameplay rule, fixed-step at 30 Hz; data schemas and validation; stable ids; seeded randomness; replay hashing | Read files, open sockets, call GL or SDL, read the clock |
| `sc-net` | Message types (one schema for both ends), the UDP transport, reliability, delta compression | Contain gameplay rules |
| `sc-server` | The authoritative loop, sessions and seats, saves | Render, play audio |
| `sc-render` | GL state, shaders, vertex formats, passes, portal culling, text and panel drawing | Decide gameplay outcomes |
| `sc-client` | Platform (SDL2), input mapping, prediction, interpolation, console UI logic, audio mixing | Resolve a gameplay outcome itself (it previews by calling `sc-core`) |
| `sc-tools` | `deckc` (decks), `meshc` (meshes), `bake` (light, `light-baking`), validators | Ship in the game build |
| `sc-probe` | Measuring the Pi | Share code paths with the game it does not need (it links `sc-render`) |

## 4. What we borrow and what we build

| Need | Choice | Why |
| --- | --- | --- |
| Window, GL context, input, gamepads, audio out | SDL2 (`sdl2` crate, system library) | KMS/DRM backend on the Pi with no desktop; X11, Wayland, Windows and macOS on desktops; the GameController database for pads. |
| GL bindings | `glow` | Thin and unopinionated; we own every GL call; also targets WebGL 2. |
| Maths | `glam` (f32 and f64 types) | Small, SIMD on aarch64 NEON, both precisions for the frames rule. |
| Data | `serde`, `serde_json` with `deny_unknown_fields` | Validated JSON data with units in keys (CLAUDE.md 6.5). |
| Wire and save format | `postcard` (compact binary over serde) | Small packets; one schema crate for both ends. |
| Images | `png` (tools and load time) | PNG sources are committed (CLAUDE.md 9). |
| Audio decode | `lewton` (Ogg Vorbis) | Small, pure Rust. |
| Logging | `log` with a small logger | Nothing heavier on the Pi. |
| ECS | **None.** Typed arenas with generational ids | Tens of crew and craft, hundreds of projectiles: plain arrays are fast and easy to reason about. |
| Physics | **Ours.** Capsule against convex brushes in the interior; spheres, capsules and rays outside | Two small, specific problems. A general physics engine would give us a single world we would then have to split. |
| Renderer | **Ours**, on OpenGL ES 3.0 | Our pass structure (space, viewscreen, decks through portals, glass, UI) is small and specific. |
| UI | `egui` with `egui_glow`, inside our fixed-panel layout rules | Immediate mode, fast to build consoles with, affordable on an A76; the probe measures it, and our own panel layer is the fallback (question E3). |
| Networking | **Ours**: UDP with reliable and unreliable channels | See `netcode-and-sessions`. |

## 5. The Pi 5 budget

<!-- pi-budget triangles=200000 draw_calls=300 texture_mb=96 -->

**Provisional.** Estimates from published figures and from the scale of comparable games, to be
replaced by `sc-probe` measurements on a 1 GB Pi 5. This table is the one source for these
numbers (CLAUDE.md section 2); the marker comment above is read by
`tools/mockups/inline.py --check` so the mockups' meter cannot drift from it. The budgets are
ceilings, not targets: low poly is the style (owner: "this game doesn't need high end graphics").

**The client, on a 1 GB Pi 5**

| Budget | Value | Notes |
| --- | ---: | --- |
| Output resolution | 1920 x 1080 | 3D renders at 1280 x 720 by default and scales up; UI draws at full resolution. |
| Frame time | 16.7 ms target, 33.3 ms floor | 60 frames a second where it fits, never below 30. |
| Visible triangles per frame, all passes | 200,000 | The number most likely to move after the probe. |
| Draw calls per frame, all passes | 300 | Instancing is available for repeated props and projectiles. |
| Texture memory | 96 MB | Palette atlas, font atlas, decals, screens, lightmaps if `light-baking` adopts them, render targets. |
| Vertex and index buffers | 64 MB | All decks of one ship resident; 32-bit indices allowed. |
| Viewscreen render target | 1024 x 512, one, at most 30 Hz | About 3 MB with depth. |
| Secondary views | Up to two at 512 x 256, at most 15 Hz | A turret feed, a fighter's camera on a console. |
| MSAA | 4x where the probe finds it cheap | Tile-based GPUs resolve MSAA on chip. |
| Average overdraw | 3x | Glass, particles and UI over the scene. |
| Client main thread, CPU | 8 ms | Culling and submission 3, UI 2, prediction and interpolation 1, the rest slack. |
| Client resident memory | 384 MB | Allocated at startup by budget; refuses to load what does not fit. |
| GPU allocations (CMA) | 192 MB | Textures, buffers, framebuffers. To set and verify on the Pi. |
| A solo listen server inside the client | 64 MB | One player with automation; a crew uses the main server. |
| OS (Raspberry Pi OS Lite, 64-bit, no desktop) | about 200 MB | To measure. 1 GB less these leaves about 180 MB of headroom. |
| Install size | 500 MB | |

**The main server, on a 4 GB Pi 5**

| Budget | Value | Notes |
| --- | ---: | --- |
| Server tick | 30 Hz, at most 2 ms per ship on one A76 core | Ship systems (power, atmosphere, heat, fire) sub-step at 10 Hz inside it. |
| Server resident memory | 512 MB per session | A session is one player ship and its crew, the enemies and the star system. |
| Sessions | 1 now; more measured later | 4 GB leaves room for several; how many is a measurement, not a promise. |
| Network, per client | 64 kbit/s down, 16 kbit/s up | `netcode-and-sessions`; sized for clients on Wi-Fi. |
| Cooling | Active Cooler | A server runs flat out for hours. |

**Content**

| Budget | Value | Notes |
| --- | ---: | --- |
| Bridge compartment geometry | 30,000 triangles | The most detailed room. |
| Other compartment geometry | about 8,000 triangles each | `deck-pipeline` gives the per-compartment table. |
| Ship exterior | 12,000 triangles; LODs 3,000 and 600 | |
| Fighter / missile | 1,500 / 200 triangles | |
| Crew avatar | 3,000 triangles, at most 48 bones | Skinned in the vertex shader. |
| Active exterior bodies near the ship | 128 | Ships, craft, missiles. |
| Projectiles | 1,024, instanced | One draw call. |
| Audio voices | 32 | Mixed on SDL's audio thread. |

**What 60 frames a second buys on a Pi 5.** The owner asked (2026-10-04): "What kind of graphics
stack can we get with pi5 at 60fps?" The published evidence, gathered that day (sources in
`docs/references.md`, "Raspberry Pi 5 graphics"):

| Evidence | Number | What it tells us |
| --- | --- | --- |
| Raspberry Pi's own benchmarking post (Core Electronics' tests) | glmark2 202 against the Pi 4's 97; OpenArena timedemo 27.05 fps against 8.77 (settings not stated) | About 2-3 times a Pi 4. An old Quake III-engine game with its legacy OpenGL path does not reach 60 at those settings. |
| Phoronix, Mesa drivers | glmark2 about 4.3 times a Pi 4 at 1080p; YQuake2 above 230 fps against under 90 on a Pi 4 | A Quake II-class renderer has several times the 60 fps headroom. |
| User reports | Xonotic at about 65 fps on an overclocked Pi 5 (settings not stated); SuperTuxKart struggles at 1080p on maximum settings and runs well on low; Godot users call the GPU weak for 3D | Modern-looking 3D with heavy settings does not hold 60; lean settings do. |
| Khronos and Mesa | OpenGL ES 3.1 and Vulkan 1.3 conformant (`v3d`, `v3dv`, Mesa 24.3 and later) | The API is not the limit; shader cost, fill rate and bandwidth are. |

So, at 60 frames a second the Pi 5 carries a clean late-1990s to early-2000s look done with
modern batching: Quake II to Quake III fidelity, which is the low-poly, brush-built, baked-light
style this game already chose. In practice, provisional until the probe:

| Fits at 60 fps | Maybe, after the probe | Out |
| --- | --- | --- |
| 3D at 1280 x 720 scaled to 1080p, UI at full 1080p | 3D at native 1080p on simple decks | PBR materials, normal and specular maps |
| Flat or vertex-coloured low poly with baked light (`light-baking`), three lighting states | A quarter-resolution bloom pass for screens and engine glow | Many dynamic lights with shadows, deferred lighting |
| A few dynamic point lights per room, lit per vertex (muzzle flash, sparks, alarm beacons) | One low-resolution shadow map for the sun on the exterior | Screen-space ambient occlusion, reflections, volumetrics |
| MSAA 4x (cheap on a tile-based GPU), fog, emissive strips | Simple per-pixel lighting on the hero ship exterior | A post-processing stack |
| Stencil-masked windows into space; one 1024 x 512 viewscreen at 30 Hz | A second live view (turret, fighter) at 15 Hz | High-resolution textures |
| Instanced projectiles and particles, skinned crew of a few thousand triangles, a starfield and a planet with a rim-light shader | | |

**Plan scenes at about half the triangle ceiling for 60 fps** (about 100,000 triangles and 150
draw calls, as the worked frame below does); the ceiling is what a busy moment may reach without
dropping below the 30 fps floor. If the probe says the GPU is weaker than this, the first cuts
are the native 1080p option, then the second live view, then the triangle ceiling.

A worked frame on the bridge, to show the table is coherent: space pass (stars 1 call, planet 1,
two enemy ships 2, projectiles 1 instanced: about 30,000 triangles), viewscreen target (the same
scene from another camera at 1024 x 512, 30 Hz: about 30,000 triangles every other frame), deck
pass (the bridge and the two compartments seen through its aft door: about 46,000 triangles in
20 calls; four crew avatars, 12,000 triangles in 4 calls), glass 2 calls, UI 10 calls. About
100,000 triangles and 45 calls: half the budget, which leaves room for the probe to cut it and
for the ship to be busier than this.

## 6. Frame and tick structure

- **Threads on the client:** main (input, prediction, render, UI), network (socket I/O, packet
  assembly), SDL's audio thread, and, in a solo game, the server thread. Four threads on four
  cores.
- **Server loop:** fixed 30 Hz tick. Each tick applies queued commands in a stable order, steps
  flight and weapons, and every third tick steps the ship systems (power, atmosphere, heat,
  fire) at 10 Hz. Snapshots go out at 20 Hz.
- **Client loop:** renders at the display rate (vsync), interpolating other entities 100 ms in
  the past and predicting its own avatar or fighter from inputs not yet acknowledged.
- **Time is seconds,** carried as `f64` simulation time and integer tick numbers. Nothing in
  `sc-core` counts frames (star-crew-64's lesson).

## 7. The renderer's floor

**Allowed (OpenGL ES 3.0 core, GLSL ES 3.00):** vertex array objects, instanced drawing, up to
four colour attachments, 32-bit indices, uniform buffer objects, texture arrays and 3D textures,
sRGB textures and framebuffers, integer and half-float vertex attributes, multisampled
renderbuffers, ETC2 compressed textures, render-to-texture with depth.

**Not assumed:** compute shaders and storage buffers (the Pi 5 has ES 3.1, but desktop GL 3.3
and WebGL 2 do not), geometry and tessellation shaders, float render targets (an extension;
measured, with an 8-bit fallback), bindless anything. An extension is used only behind a
fallback that has been measured on a Pi 5.

**Desktop and web:** Linux desktops request OpenGL ES 3.0 through EGL; Windows and macOS use
desktop OpenGL 3.3 core (4.1 on macOS) with a small prelude that compiles the same GLSL ES 3.00
sources; a future browser client would use WebGL 2, which is ES 3.0.

**Shaders (about six programs):**
1. Deck: position, three baked vertex colour sets blended by per-compartment uniforms (normal,
   red alert, emergency), palette atlas lookup, an optional lightmap (`light-baking`), fog.
2. Exterior lit: per-vertex or per-pixel Lambert from the sun plus ambient, flat-shaded.
3. Unlit and emissive: screens, engine glow, lamps.
4. Instanced billboards: projectiles, sparks, markers.
5. Stars: points.
6. UI: textured quads for text and panels.

**Deck vertex format, 28 bytes:** position 3 x int16 in centimetres (plus padding), normal
3 x int8, three colour sets 3 x RGBA8, atlas UV 2 x uint16. Compact on purpose: the 1 GB board
still makes memory the budget to respect. Flat shading means unshared vertices: 200,000
triangles cost about 17 MB.

**Keeping draw calls down:** static geometry is merged per compartment per material at compile
time; repeated props (seats, lockers, racks) and projectiles are instanced; doors and other
moving parts are separate; UI is batched by texture. The frame is drawn space first, then decks,
then glass, then UI, sorted by program within each pass.

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
  (`deck-pipeline`, with light from `light-baking`); `meshc` turns generated meshes into
  `.mesh`. Both are little-endian, chunked and versioned with a header and checksum, read with
  one sequential pass.
- **One schema.** Data types live in `sc-core`; the tools and the game use the same structs.

## 10. Platforms, build and deployment

| Target | Rust target | Display | Status |
| --- | --- | --- | --- |
| Raspberry Pi 5, 1-4 GB, Raspberry Pi OS Lite 64-bit | `aarch64-unknown-linux-gnu` | SDL2 KMS/DRM, full screen, no desktop | The floor |
| Raspberry Pi 5, 4 GB, as the main server | `aarch64-unknown-linux-gnu` | None (headless `sc-server`) | The main server |
| Linux desktop | `x86_64-unknown-linux-gnu` | SDL2, OpenGL ES 3.0 through EGL | Development and play |
| Windows | `x86_64-pc-windows-msvc` | SDL2, OpenGL 3.3 core | Play |
| macOS | `aarch64-apple-darwin` | SDL2, OpenGL 4.1 core (deprecated by Apple, still working) | Best effort |
| Browser | `wasm32-unknown-unknown` | WebGL 2 through `glow` | Later, out of scope here |

- **Cross-compiling for the Pi** uses a Raspberry Pi OS sysroot with SDL2, libdrm, GBM and EGL
  development files, through `cross` with a custom image or `cargo zigbuild` with the sysroot.
  `-C target-cpu=cortex-a76`. A Pi 5 can also build natively.
- **On the Pi** the client is a systemd service or an autologin launch on the console with
  `SDL_VIDEODRIVER=kmsdrm` and the user in the `video`, `render`, `input` and `audio` groups.
  The main server is a systemd service that restarts on failure and writes its saves to local
  storage. A deploy script copies binaries, `data/` and `compiled/` over SSH.

## 11. The probe: measuring before building

`sc-probe` is a measurement instrument (CLAUDE.md section 4) and the first thing built. It links
`sc-render` and draws synthetic scenes full screen at 1920 x 1080 output with 3D at 1280 x 720,
on a 1 GB Pi 5:

1. **Triangle curve:** flat-shaded, vertex-coloured triangles in the deck format, 50,000 to
   1,000,000 in steps, in 50 calls. Frame time p50, p95 and p99 at each step.
2. **Draw-call curve:** 100,000 triangles split into 50 to 2,000 calls; the same with
   instancing.
3. **Fill rate:** full-screen quads, overdraw 1x to 8x, with the deck shader, with an added
   lightmap fetch, and with MSAA 4x.
4. **Render to texture:** viewscreen targets at 512 x 256, 1024 x 512 and 2048 x 1024.
5. **UI:** 200 panels and 4,000 glyphs with egui, and with a minimal panel layer of our own.
6. **Memory:** resident size and GPU allocations with the Tern's compiled decks loaded; free
   memory left on a 1 GB board.
7. **Simulation (once `sc-core` exists):** ticks per second of the Tern's systems on one core,
   on the 4 GB server.
8. **glow against wgpu (question E2):** scenes 1, 2 and 6 drawn twice, once through glow on
   OpenGL ES 3.0 and once through wgpu on Vulkan, on the same 1 GB Pi 5. Reports frame time,
   CPU time per draw call, resident memory and GPU allocations, startup time, and whether each
   runs full screen with no desktop (wgpu needs a Vulkan display surface or a small kiosk
   compositor; to verify).

It writes a JSON report and a short Markdown summary to `docs/benchmarks/<date>-pi5-probe/`,
naming the board, its RAM, OS, clocks, cooling, resolution, build and repeats, with the spread
between repeats (CLAUDE.md 12). The budget table is then corrected from it in the same commit.
The owner runs it on a Pi 5; a cloud session cannot.

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

Render tests capture frames headless through Mesa's software renderer (`llvmpipe` over EGL,
OpenGL ES 3.0) and compare them to committed references with a tolerance. They prove the passes
compose; they do not prove anything about the Pi's speed.

## 13. Risks and trade-offs

- **The budget is an estimate.** The probe may move any number in it; the mockups show every
  design's spend so a cut can be made where it hurts least.
- **The 1 GB board is the tight one.** Memory, not speed, is the Pi 5's limit for us. Every
  change states its memory, and the client refuses to start rather than swap.
- **SDL2's KMS/DRM path on the Pi 5** is the platform risk. The probe is its first test; a small
  kiosk Wayland compositor is the fallback.
- **Rust cross-compilation needs a sysroot**, a task rather than a research problem.
- **Our own renderer and physics** are more code than a library, but small, specific and
  measurable.
- **macOS OpenGL is deprecated.** It still works; if it stops, `sc-render`'s interface is where
  a wgpu or Metal path would go (question E2).

## Open questions

Per CLAUDE.md section 13, only E1 went to the owner (a fork in the road that changes every later
change); the owner answered it in chat. The others take the recommendation and are recorded as
"recommendation taken (ask only with screenshots)".

| # | Question | Options | Recommendation | Status |
| --- | --- | --- | --- | --- |
| E1 | The engine's language. | Rust + SDL2 + glow / C11 + raylib | Rust + SDL2 + glow | **Decided 2026-10-04: Rust + SDL2 + glow** (owner: "Your stack sounds like a solid plan") |
| E2 | The GPU API now that the Pi 5 has Vulkan 1.3. glow on ES 3.0 is the smallest and runs with no desktop; wgpu brings WGSL (as in Pale-Blue-Dot), Metal on macOS and compute, at some cost in memory and per-call CPU. The owner asked "So webgpu is good?" (2026-10-04). | glow on OpenGL ES 3.0 / wgpu | Decide by measurement: probe scene 8 draws the same scenes both ways on a 1 GB Pi 5. glow stays the default until then, behind `sc-render`'s own interface so the switch touches nothing else | Open until the probe |
| E3 | Console UI library. On an A76, egui costs about a millisecond or two; ours costs code. | egui / ours | egui with our fixed-panel rules, measured by the probe | Recommendation taken (ask only with screenshots) |
| E4 | The client's memory floor. | 1 GB / 2 GB | 1 GB | **Decided 2026-10-04 by the owner** ("pi5 1gb-4gb") |
| E5 | The main server. | 4 GB Pi 5 / any machine | A 4 GB Pi 5 running `sc-server` on Ethernet with the Active Cooler; any desktop also works | **Decided 2026-10-04 by the owner** ("4 GB can be used as main server too") |
