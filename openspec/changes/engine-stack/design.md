# Design: the engine stack

Status: **proposed** (2026-10-04); the language and stack are **decided** by the owner (E1,
below), and the graphics layer since 2026-10-07 (E2: sokol_gfx with SDL3). Nothing here is built. The budget table in section 5 is provisional until `sc-probe`
measures it on a Pi 5 (section 11).

**History.** This design was first written against a Raspberry Pi 3 (OpenGL ES 2.0, 1 GB). The
same day the owner corrected the target: "I'm sorry we're running on a pi5 1gb-4gb, 4 GB can be
used as main server too". The floor is now a Pi 5 with 1 GB as the client and a 4 GB Pi 5 as
the main server; the renderer floor rose from OpenGL ES 2.0 to 3.0, and every budget below was
re-estimated. The stack the owner approved (Rust, SDL, glow) is unchanged; the platform layer
moved from SDL2 to SDL3 because the Pi 5 needs SDL's atomic KMS/DRM path (section 4).

**2026-10-07: sokol_gfx in place of glow.** The owner asked whether a browser build was possible
and how sokol compared with glow, then decided: "sokol it is then", "with sdl3". The renderer now
draws through `sokol_gfx` on its OpenGL ES 3.0 backend, with shaders compiled from one source by
`sokol-shdc`; SDL3 stays the platform layer (section 4). The browser became a playtest target
(section 10a), and the owner chose WebRTC for every client the same day (`netcode-and-sessions`
section 2, M4).

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
| **Rust + SDL3 + sokol_gfx (OpenGL ES 3.0)** | Yes | Yes (SDL 3.4 atomic KMS/DRM) | Ours to design | Memory safe | `cargo test`, as Pale-Blue-Dot | Rust (Pale-Blue-Dot); sokol (owner, 2026-10-07: "I've used sokol for this as well") | **Decided** (2026-10-07) |
| Rust + SDL3 + glow (OpenGL ES 3.0) | Yes | Yes | Ours to design | Memory safe | Yes | Rust | Decided 2026-10-04, superseded by sokol_gfx 2026-10-07 (E2) |
| Rust + SDL3 + wgpu (Vulkan) | Yes, through Mesa `v3dv` (to verify) | Needs a Vulkan display surface without a desktop, or a small kiosk compositor (to verify) | Ours to design | Memory safe | Yes | WGSL and wgpu through Bevy | Possible; more driver surface and memory than a low-poly game needs |
| C11 + raylib | Yes (raylib's GLES path) | Yes (raylib's DRM platform) | Ours to design | Manual | By discipline | C (star-crew-64) | Runner-up; the owner chose Rust |
| Bevy | Yes, through wgpu and Vulkan (memory on 1 GB to verify) | As wgpu | One ECS world and one physics world to split | Memory safe | Yes | Yes (Pale-Blue-Dot) | Not chosen: the owner asked for a custom engine for the decoupling, and 1 GB is tight |
| Godot 4 | Yes, Compatibility renderer on ES 3 (performance to verify) | Community setups | One scene tree | | | Yes (Undercity) | Not chosen, for the same reasons |
| three.js in Chromium | WebGL 2 | No | Ours | | | Yes (mockups) | A browser takes too much of 1 GB for the Pi client; the browser playtest build is our own engine compiled to WebAssembly instead (section 10a) |

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

What Rust costs: longer compile times, and cross-compiling needs a Raspberry Pi OS sysroot
(section 10). A Pi 5 can compile the game itself, slowly; the desktop cross-compile is faster.

**Why sokol_gfx on OpenGL ES 3.0** (E2, decided by the owner 2026-10-07). OpenGL ES 3.0 is what a
low-poly, vertex-lit game needs; it runs full screen from the console through SDL3's KMS/DRM
backend with nothing else installed; it uses the least memory and driver surface on the 1 GB
board; and it is WebGL 2, so the same renderer runs in a browser (section 10a). The question was
how to call it:

| | glow (decided 2026-10-04) | **sokol_gfx (decided 2026-10-07)** |
| --- | --- | --- |
| Level | Raw GL calls with one Rust signature for GL, GLES and WebGL 2 | A small rendering API over GL: buffers, images, samplers, shaders, pipelines, passes, bindings |
| What we write | Every GL call, then our own layer for pipeline state, resource lifetimes and state caching | Pipelines created once (shader, vertex layout, blend, depth, cull), then bind and draw; state caching is sokol's |
| Shaders | GLSL ES 3.00 by hand; the Rust uniform structs kept in step by us | One source compiled by `sokol-shdc` to GLSL ES 3.00 (Pi, Linux, browser) and GLSL 4.10 (Windows, macOS), with the uniform structs generated as Rust (`-f sokol_rust`): the Rust and GLSL layouts cannot disagree (CLAUDE.md 6.6, "validate the real artifact") |
| Mistakes | GL errors, often silent | A validation layer in debug builds that names the mismatch |
| Memory | Ours to build | Fixed resource pools sized at `sg_setup`, which is CLAUDE.md 2's "allocate up front" |
| Reach | All of GLES 3.0 and any extension | What its API exposes; the native GL handles are available when we need to go around it |
| Other backends | GL only | Metal, D3D11, WebGPU and Vulkan behind the same API and the same shader source: macOS's deprecated OpenGL stops being a risk (section 13) |
| Build | Pure Rust | A C library: a C compiler in every build, cross-builds included (section 4) |
| Browser | `wasm32-unknown-unknown` with `web-sys` | `wasm32-unknown-emscripten`, through Emscripten (section 10a) |

On the Pi neither is faster: both end in the same Mesa driver (`v3d`), and the deck pass is about
one draw per compartment, so call overhead does not decide it. sokol_gfx saves writing and
debugging a rendering layer that would end up shaped like it, and `sokol-shdc` removes the
hand-kept Rust and GLSL layouts. wgpu, the other candidate in E2, is not pursued: sokol_gfx's
WebGPU and Vulkan backends sit behind the same interface if a later need appears, and the
1 GB board still argues for the smallest driver surface.

**Decided (E1):** Rust + SDL + glow (SDL3 since the same day, section 4). The owner, 2026-10-04: "That's fine..this game doesn't need
high end graphics. Your stack sounds like a solid plan". **Decided (E2), 2026-10-07:** sokol_gfx in
place of glow, with SDL3 (owner: "sokol it is then", "with sdl3").

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
   sc-net (messages,        sc-server (headless:       sc-render (sokol_gfx on
   WebRTC, deltas)          core + net + saves)        OpenGL ES 3.0: passes, decks,
                                                       culling, UI draw)
        ^                                                   ^
        +---------------------- sc-client -----------------+
                    (SDL3: window, input, audio; prediction,
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
| `sc-net` | Message types (one schema for both ends), the WebRTC data channel transport (`netcode-and-sessions` section 2), delta compression | Contain gameplay rules |
| `sc-server` | The authoritative loop, sessions and seats, saves | Render, play audio |
| `sc-render` | sokol_gfx resources and pipelines, the shaders (one `sokol-shdc` source each), vertex formats, passes, portal culling, text and panel drawing | Decide gameplay outcomes; call SDL (it is handed a GL context and a framebuffer size) |
| `sc-client` | Platform (SDL3), input mapping, prediction, interpolation, console UI logic, audio mixing | Resolve a gameplay outcome itself (it previews by calling `sc-core`) |
| `sc-tools` | `deckc` (decks), `meshc` (meshes), `bake` (light, `light-baking`), validators | Ship in the game build |
| `sc-probe` | Measuring the Pi | Share code paths with the game it does not need (it links `sc-render`) |

## 4. What we borrow and what we build

| Need | Choice | Why |
| --- | --- | --- |
| Window, GL context, input, gamepads, audio out | SDL3, version 3.4 or later (`sdl3` crate, with SDL built from source through `sdl3-src` so the version does not depend on the OS's package) | KMS/DRM backend on the Pi with no desktop (the atomic path, which the Pi 5 needs); X11, Wayland, Windows and macOS on desktops; the gamepad database. |
| Graphics API | `sokol_gfx` on its GLES3 backend (GLCORE, GL 4.1, on Windows and macOS), from the sokol headers at a pinned revision | Pipelines, pools and a validation layer over GL; WebGL 2 in the browser; decided 2026-10-07 (section 2, E2). |
| Shader compiler | `sokol-shdc`, a prebuilt binary at a pinned revision (Linux x86-64 and arm64, macOS, Windows builds exist), run by `sc-render`'s build script | One GLSL source per program, compiled to `glsl300es` and `glsl410`, with Rust uniform structs and reflection generated. |
| Maths | `glam` (f32 and f64 types) | Small, SIMD on aarch64 NEON, both precisions for the frames rule. |
| Data | `serde`, `serde_json` with `deny_unknown_fields` | Validated JSON data with units in keys (CLAUDE.md 6.5). |
| Wire and save format | `postcard` (compact binary over serde) | Small packets; one schema crate for both ends. |
| Images | `png` (tools and load time) | PNG sources are committed (CLAUDE.md 9). |
| Audio decode | `lewton` (Ogg Vorbis) | Small, pure Rust. |
| Logging | `log` with a small logger | Nothing heavier on the Pi. |
| ECS | **None.** Typed arenas with generational ids | Tens of crew and craft, hundreds of projectiles: plain arrays are fast and easy to reason about. |
| Physics | **Ours.** Capsule against convex brushes in the interior; spheres, capsules and rays outside | Two small, specific problems. A general physics engine would give us a single world we would then have to split. |
| Renderer | **Ours**, on sokol_gfx and OpenGL ES 3.0 | Our pass structure (space, viewscreen, decks through portals, glass, UI) is small and specific. |
| UI | `egui`, drawn by a painter of ours on sokol_gfx, inside our fixed-panel layout rules | Immediate mode, fast to build consoles with, affordable on an A76; the probe measures it, and our own panel layer is the fallback (question E3). egui hands out textured triangle meshes with a clip rectangle each, so the painter is one pipeline, one font texture and a scissor per mesh (it replaces `egui_glow`). Feeding SDL3's events into egui is a small adapter of ours unless a maintained crate exists (to verify). |
| Networking | WebRTC data channels: `str0m` (pure Rust, sans I/O, to verify on the Pi) natively, the browser's own `RTCPeerConnection` on the web; our channels and messages above them | Decided by the owner 2026-10-07 for web and desktop alike; see `netcode-and-sessions` section 2 and the `matchmaker` change. |

**Why SDL3, not SDL2** (the owner asked, 2026-10-04: "What framework is best for this gles on
the pi with rust?"). Of the Rust options, only SDL gives a mature KMS/DRM backend (full screen
from the console, no desktop) together with input, gamepads, audio and the desktop platforms. On
a Pi 5 its KMS/DRM backend needs the atomic mode-setting path: SDL's pull request 11511, merged
2025-10-19 for SDL 3.4.0, restored it, with the note "Main's kmsdrm backend is totally broken on
a Raspberry Pi 5, but the atomic version in this PR works", and issue 8579 recorded garbage on
screen from the non-atomic path on a Pi 5. The Rust `sdl3` crate reached 0.20 in September
2026, and `sdl3-sys` bundles SDL 3.4.10. So the platform layer is SDL 3.4 or later, built from
source with the game. Sources are in `docs/references.md`, "Rust platform layer".

| Rust option for windowing and the GL context (the graphics API above it is section 2's choice) | KMS/DRM, no desktop | Input, pads, audio | Verdict |
| --- | --- | --- | --- |
| **SDL3 (`sdl3` crate)**, with sokol_gfx drawing into its GL context | Yes, atomic in SDL 3.4+ | All three | **Chosen** |
| SDL2 (`sdl2` crate) | Non-atomic path broken on a Pi 5 in 2023-2025 (issue 8579; whether SDL 2.30 fixed it is to verify) | All three | Superseded |
| winit + glutin | No (X11 and Wayland only) | Input only; pads and audio separate | Needs a desktop or kiosk compositor on the Pi |
| miniquad (macroquad) | No backend for it found (to verify) | Basic | Same; and its renderer abstraction would sit between us and GL |
| `drm` + `gbm` + `khronos-egl` crates | Yes, our own code | None: evdev, a pad library and an audio crate on top | The fallback if SDL's KMS/DRM path fails the probe |
| sokol_app (with sokol_gfx) | No: X11 or Wayland only | Input; no gamepads; minimal audio (`sokol_audio`) | Not chosen for the Pi or desktops (owner, 2026-10-07: "with sdl3"); a fallback for the browser only (section 10a) |
| Bevy, Fyrox, three-d | Through winit, so no | Yes | Engines; see section 2 |

**How sokol_gfx is built and bound.** sokol is a set of single-file C headers. The official Rust
bindings, `sokol-rust` (`https://github.com/floooh/sokol-rust`, not on crates.io: the `sokol`
crate there is an unrelated 2019 binding), are generated from the headers. Their build script
compiles every sokol module together, `sokol_app` and `sokol_audio` included, and on Linux links
X11 (or Wayland), ALSA and libGL whether or not they are used. So `sc-render` does not take the
crate as a dependency. It vendors, with provenance (CLAUDE.md 6.6), `sokol_gfx.h`, `sokol_log.h`
and the generated `gfx.rs` and `log.rs` from one `sokol-rust` revision, and compiles one C file
with the `cc` crate:

- `SOKOL_GLES3` on the Pi, Linux desktops and the browser; `SOKOL_GLCORE` on Windows and macOS.
- No `sokol_app` and no `sokol_glue`: SDL3 creates the GL context and makes it current, and
  `sc-render` fills sokol's `Environment` and `Swapchain` (default framebuffer 0, the drawable
  size from SDL, the sample count and depth format SDL was asked for) itself.
- The revisions at the time of writing are `sokol` `401f21f`, `sokol-rust` `1a5cb22` and
  `sokol-tools-bin` `11d0cf6` (2026-10-07); updating them is one commit that rebuilds every
  shader and runs the render tests. `sokol-rust`'s own warning applies: a changed header with a
  stale build directory gives Rust structs that no longer match the C ones, so the build script
  reruns on any vendored file's change and the render tests catch the rest.


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
| Texture memory | 96 MB | The material texture array (11 layers of 128 x 128 RGBA8 with mipmaps: 0.92 MB, `surface-materials`), font atlas, decals, screens, lightmaps if `light-baking` adopts them, render targets. |
| Vertex and index buffers | 64 MB | All decks of one ship resident; 32-bit indices allowed. |
| Viewscreen render target | 1024 x 512, one, at most 30 Hz | 4 MB: 2 MB of colour and 2 MB of 24-bit depth with stencil (corrected from "about 3 MB", which assumed a 16-bit depth; `ship-frames` section 8). |
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
| Network, per client | 80 kbit/s down, 32 kbit/s up | `netcode-and-sessions`; sized for clients on Wi-Fi. Raised 2026-10-07 from 64 and 16 for WebRTC's headers (about 101 bytes a packet), netcode section 4. |
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
- **The frame is a callback, never a blocking loop.** The client is written to SDL3's main
  callbacks (`SDL_AppInit`, `SDL_AppEvent`, `SDL_AppIterate`, `SDL_AppQuit`): each call of
  `SDL_AppIterate` runs one frame (poll input, advance the fixed-step clock, render) and returns.
  On the Pi and desktops SDL calls it in its own loop; in a browser it is called from
  `requestAnimationFrame`, because a page only repaints and delivers input after the code returns
  to it (section 10a). Nothing in the client may block the frame waiting on the network or a
  file.
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
desktop OpenGL 4.1 core through sokol_gfx's GLCORE backend (raised from 3.3 on 2026-10-07: 4.1 is
the oldest version `sokol-shdc` targets, and every GPU that runs 3.3 on Windows today runs 4.1);
the browser build uses WebGL 2, which is ES 3.0 (section 10a).

**Shader sources.** Each program is one file in `sokol-shdc`'s dialect (Vulkan-style GLSL 4.50
with `@vs`, `@fs` and `@program` tags) under `sc-render/shaders/`, compiled by the build script to
`glsl300es` and `glsl410`. The compiler stays inside this floor because the floor is checked on
its output, not on the source: a render test compiles each program on llvmpipe's OpenGL ES 3.0.
One difference from the GLES 3.0 feature list below: sokol_gfx's GL backends upload a uniform
block as an array of `vec4` with `glUniform4fv`, not as a uniform buffer object. Our uniforms are
small and per draw (a compartment's three state weights, a transform), so nothing here needs a
buffer shared between programs.

**The deck vertex format in sokol_gfx's terms.** `deck-pipeline` section 5 owns the 28-byte
vertex; it does not change. sokol_gfx has no three-component 16-bit format, so its first 8 bytes
are read as one four-lane attribute, checked against the real header's list of formats
(`SG_VERTEXFORMAT_*`, `sokol_gfx.h` at the pinned revision):

| Bytes | Content (`deck-pipeline`) | sokol_gfx format | In the shader |
| ---: | --- | --- | --- |
| 0-7 | Position 3 x `i16`, then mover and layer 2 x `u8` | `SHORT4` | `xyz` is the position in 1/1024 m; `w` holds mover + 256 x layer as a signed 16-bit number, decoded exactly in floats (add 65,536 when negative, then divide by 256) |
| 8-11 | Normal, 2_10_10_10 | `INT10_N2` | Normalized; sokol reports whether the context has it (`vertexformat_int10_n2`), and GLES 3.0 does |
| 12-23 | Three colour sets | `UBYTE4N` x 3 | As before |
| 24-27 | Texture coordinate 2 x `i16` | `SHORT2` | Times 1/1024 of a layer's span |

`deckc`'s writer and `sc-render`'s pipeline description name these offsets once, in `sc-core`'s
format table, and a test builds the pipeline from that table and draws a known vertex
(CLAUDE.md 6.6).

**Shaders (about six programs):**
1. Deck: position, three baked vertex colour sets blended by per-compartment uniforms (normal,
   red alert, emergency), a texture array lookup (one layer per Material Maker material,
   nearest on magnification; `surface-materials`, 2026-10-05, in place of a palette atlas), an
   optional lightmap (`light-baking`), fog.
2. Exterior lit: per-vertex or per-pixel Lambert from the sun plus ambient, flat-shaded.
3. Unlit and emissive: screens, engine glow, lamps.
4. Instanced billboards: projectiles, sparks, markers.
5. Stars: points.
6. UI: textured quads for text and panels.

**Deck vertex format, 28 bytes:** `deck-pipeline` section 5 owns it: position 3 x int16
(1/1024 m, compartment-local), mover index and texture layer 2 x uint8, normal 2_10_10_10, three
colour sets 3 x RGBA8, texture coordinate 2 x int16 (revised 2026-10-05: the layer and texture
coordinate replace an atlas UV, for `surface-materials`). Compact on purpose: the 1 GB board
still makes memory the budget to respect. Flat shading means unshared vertices: 200,000
triangles cost about 17 MB.

**Keeping draw calls down:** static geometry is merged per compartment at compile time, one
draw whatever its materials, because every material is a layer of one texture array; repeated props (seats, lockers, racks) and projectiles are instanced; doors and other
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
| Raspberry Pi 5, 1-4 GB, Raspberry Pi OS Lite 64-bit | `aarch64-unknown-linux-gnu` | SDL3 KMS/DRM (atomic), full screen, no desktop | The floor |
| Raspberry Pi 5, 4 GB, as the main server | `aarch64-unknown-linux-gnu` | None (headless `sc-server`) | The main server |
| Linux desktop | `x86_64-unknown-linux-gnu` | SDL3, OpenGL ES 3.0 through EGL | Development and play |
| Windows | `x86_64-pc-windows-msvc` | SDL3, OpenGL 4.1 core (sokol_gfx GLCORE; D3D11 is available behind the same API) | Play |
| macOS | `aarch64-apple-darwin` | SDL3, OpenGL 4.1 core (deprecated by Apple, still working; Metal is available behind the same API) | Best effort |
| Browser | `wasm32-unknown-emscripten` | SDL3's Emscripten port and sokol_gfx's GLES3 backend on WebGL 2 | A playtest build (section 10a), after the Pi client |

- **Cross-compiling for the Pi** uses a Raspberry Pi OS sysroot with libdrm, GBM, EGL and the
  input and audio development files SDL3 builds against, through `cross` with a custom image or `cargo zigbuild` with the sysroot.
  `-C target-cpu=cortex-a76`. A Pi 5 can also build natively.
- **On the Pi** the client is a systemd service or an autologin launch on the console with
  `SDL_VIDEODRIVER=kmsdrm` and the user in the `video`, `render`, `input` and `audio` groups.
  The main server is a systemd service that restarts on failure and writes its saves to local
  storage. A deploy script copies binaries, `data/` and `compiled/` over SSH.

### 10a. The browser build (2026-10-07)

The owner asked, 2026-10-07: "does gles support web builds? how easy would it be for me to hand
out builds for people to playtest in the browser". OpenGL ES 3.0 is WebGL 2, so the renderer and
its shaders carry over. **Recommendation taken (ask only with screenshots), E6:** the browser is
a playtest target, designed for now and built after the Pi client. It is never the measure of
the game's speed (CLAUDE.md 2): the Pi stays the floor.

What the browser changes, and the rule each one sets for the code written before it:

| Difference | In the browser | The rule now |
| --- | --- | --- |
| The frame | Driven by `requestAnimationFrame`; a loop that never returns freezes the page | The client is written to SDL3's main callbacks (section 6), so the same code runs on both |
| Window, input, gamepads, audio | SDL3's Emscripten port maps them onto the canvas, DOM events, the Gamepad API and Web Audio (whether the `sdl3-sys` build supports Emscripten is to verify; `sokol_app` is the fallback, whose web backend is mature) | Nothing above the platform layer calls a browser or an OS API |
| Networking | No sockets: WebRTC data channels only, through the browser's `RTCPeerConnection` | Every client uses WebRTC (owner, 2026-10-07, `netcode-and-sessions` section 2); `sc-net` has one transport interface with two implementations, the only fork |
| Threads | Need the page served with two headers (`Cross-Origin-Opener-Policy: same-origin`, `Cross-Origin-Embedder-Policy: require-corp`) | The client runs correctly on one thread: the network and audio threads are optional, and a browser client never hosts a listen server |
| Files and saves | Assets are fetched; saves go to the browser's storage | Assets and saves go through one small file interface |
| Memory | WebAssembly's heap, grown as needed; a tab has far more than the Pi's 384 MB | The same startup sizing from the budget table: a browser build that fits proves nothing about the Pi |
| Hidden tabs | `requestAnimationFrame` stops | Rendering pauses; the connection's keep-alive runs from a timer so a tabbed-out player stays in the session, and the server holds the seat as for any client |

**The build.** Rust's `wasm32-unknown-emscripten` target, because sokol_gfx and SDL3 are C and
Emscripten supplies the C library and the GL-to-WebGL layer (`-sUSE_WEBGL2`, `-sMAX_WEBGL_VERSION=2`).
The output is a folder: an HTML page, the `.wasm`, Emscripten's JavaScript glue and a packed
`data/` and `compiled/` archive.

**Handing it out.** The folder is uploaded as a zip. itch.io is the planned host: an HTML5 game
behind a restricted page (a secret link or a password), with its SharedArrayBuffer option for
the two headers. Cloudflare Pages is the alternative (headers in a `_headers` file). A CI job
that builds and uploads on every push is a later task, with the CI logic in a script developers
run too.

**What a browser playtest is for.** Showing the game to people without a Pi: the decks, the
consoles, the crew working together. It is not a performance test and not a Pi substitute.

## 11. The probe: measuring before building

`sc-probe` is a measurement instrument (CLAUDE.md section 4) and the first thing built. It links
`sc-render` and draws synthetic scenes full screen at 1920 x 1080 output with 3D at 1280 x 720,
on a 1 GB Pi 5:

1. **Triangle curve:** flat-shaded, vertex-coloured triangles in the deck format, 50,000 to
   1,000,000 in steps, in 50 calls. Frame time p50, p95 and p99 at each step.
2. **Draw-call curve:** 100,000 triangles split into 50 to 2,000 calls; the same with
   instancing.
3. **Fill rate:** full-screen quads, overdraw 1x to 8x, with the deck shader (with and without
   its texture array fetch, `surface-materials`), with an added lightmap fetch, and with MSAA 4x.
4. **Render to texture:** viewscreen targets at 512 x 256, 1024 x 512 and 2048 x 1024.
5. **UI:** 200 panels and 4,000 glyphs with egui through our sokol_gfx painter, and with a
   minimal panel layer of our own.
6. **Memory:** resident size and GPU allocations with the Tern's compiled decks loaded; free
   memory left on a 1 GB board.
7. **Simulation (once `sc-core` exists):** ticks per second of the Tern's systems on one core,
   on the 4 GB server.
8. **sokol_gfx's cost (question E2, decided 2026-10-07):** the CPU time per draw call and per
   pass from scenes 1 and 2, and the memory sokol_gfx's pools take at the sizes the budget sets.
   (Until 2026-10-07 this scene compared glow with wgpu; E2 is decided, so it now checks the
   choice rather than making it.)
9. **The browser build runs** (not a Pi measurement): scenes 1, 2 and 5 built for
   `wasm32-unknown-emscripten`, opened in Chromium and Firefox on a desktop. It reports the
   download size (`.wasm`, glue and data, compressed) and that every scene draws, so the web
   path is proven before the client grows around it.

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
- **SDL3's KMS/DRM path on the Pi 5** is the platform risk. The probe is its first test (with
  `SDL_KMSDRM_ATOMIC` on); the fallbacks are our own `drm`, `gbm` and EGL setup, then a small kiosk
  Wayland compositor.
- **Rust cross-compilation needs a sysroot**, a task rather than a research problem.
- **Our own renderer and physics** are more code than a library, but small, specific and
  measurable.
- **macOS OpenGL is deprecated.** It still works; if it stops, sokol_gfx's Metal backend takes
  over behind the same API, with `sokol-shdc` compiling the same shader sources to Metal.
- **sokol is a C library with essentially one maintainer** (Andre Weissflog). It is a decade old
  and stable, and it is vendored at a pinned revision, so nothing changes under us; the Rust
  bindings are generated and see less use than the C side. A C compiler joins every build,
  cross-builds to the Pi and Emscripten included.
- **SDL3 on Emscripten through the Rust crates** is unproven here (probe scene 9 tests it); the
  fallback for the browser alone is `sokol_app`.

## Open questions

Per CLAUDE.md section 13, only E1 went to the owner (a fork in the road that changes every later
change); the owner answered it in chat, and decided E2 in chat on 2026-10-07. The others take the recommendation and are recorded as
"recommendation taken (ask only with screenshots)".

| # | Question | Options | Recommendation | Status |
| --- | --- | --- | --- | --- |
| E1 | The engine's language. | Rust + SDL + glow / C11 + raylib | Rust + SDL + glow | **Decided 2026-10-04: Rust + SDL + glow** (owner: "Your stack sounds like a solid plan"); SDL3 rather than SDL2 for the Pi 5's KMS/DRM, section 4 |
| E2 | The GPU API now that the Pi 5 has Vulkan 1.3. glow on ES 3.0 is the smallest and runs with no desktop; wgpu brings WGSL (as in Pale-Blue-Dot), Metal on macOS and compute, at some cost in memory and per-call CPU. The owner asked "So webgpu is good?" (2026-10-04), then, 2026-10-07, "I've used sokol for this as well, how viable is this?" and "alright so how does sokol compare vs glow". | glow / wgpu / sokol_gfx, each on OpenGL ES 3.0 where it can | sokol_gfx on OpenGL ES 3.0 with SDL3 as the platform layer (section 2) | **Decided 2026-10-07 by the owner:** "sokol it is then", "with sdl3" |
| E3 | Console UI library. On an A76, egui costs about a millisecond or two; ours costs code. | egui / ours | egui with our fixed-panel rules, measured by the probe | Recommendation taken (ask only with screenshots) |
| E4 | The client's memory floor. | 1 GB / 2 GB | 1 GB | **Decided 2026-10-04 by the owner** ("pi5 1gb-4gb") |
| E6 | A browser playtest build (section 10a; the owner asked "how easy would it be for me to hand out builds for people to playtest in the browser", 2026-10-07). | No browser build / a playtest target designed now, built after the Pi client / a first-class target from the first build | A playtest target designed now: the rules it sets (callbacks, one thread, a file interface, WebRTC) cost nothing in code not yet written | Recommendation taken (ask only with screenshots) |
| E5 | The main server. | 4 GB Pi 5 / any machine | A 4 GB Pi 5 running `sc-server` on Ethernet with the Active Cooler; any desktop also works | **Decided 2026-10-04 by the owner** ("4 GB can be used as main server too") |
