# Design references

What Star Crew borrows from, and what each source is cited for. CLAUDE.md section 15: cite,
don't recall; take the shape, not the text. A claim marked "to verify" is from memory and must
be checked against the source before a design depends on it.

## Bridge simulators

| Source | Cited for |
| --- | --- |
| *Artemis Spaceship Bridge Simulator* (Incandescent Workshop, 2011) | The genre's template: one server, each station on its own screen (helm, weapons, engineering, science, comms, the captain's view). Engineering's per-system power allocation with heat and coolant, and damage control teams sent through the ship (to verify the details). |
| *EmptyEpsilon* (open source, C++) | An open-source Artemis-style simulator; stations combined into fewer consoles for smaller crews, which is our merge rule's shape (to verify its presets). Useful to read for networking a bridge simulator. |
| *Starship Horizons* | A bridge simulator with flight operations and small craft (to verify). |
| *Pulsar: Lost Colony* | A walkable ship interior with crew classes and bridge stations while the ship flies: the closest existing game to "a full 3D bridge" with crew on foot. |

## Ship systems games

| Source | Cited for |
| --- | --- |
| *FTL: Faster Than Light* (Subset Games, 2012) | Per-room oxygen, power as bars per system, doors opened to space to put out fires, crew repairing systems in place. Our compartments are its rooms with real gas and real watts. |
| *Barotrauma* | A wired power grid with junction boxes, and per-hull oxygen and water flowing through gaps: the closest model for `power-grid` and `life-support` (to verify its flow model). |
| *Space Station 13* | How far atmospheric simulation can go: gas mixtures per tile and decompression. We choose per-compartment for the Pi's sake. |

## Engines and level technology

| Source | Cited for |
| --- | --- |
| *Quake* (id Software, 1996) | Brush-built levels compiled offline into a BSP tree with a potentially visible set, and collision against brush planes. |
| *Quake III Arena* and ioquake3 | Lightmapped brush levels, compiled with a light tool: the generation of game the Raspberry Pi 5 runs comfortably, and a reference point for our triangle budget (to verify the figures). |
| *Descent* (Parallax, 1995) | Levels of connected cube segments rendered through portals, in a six-degrees-of-freedom game. |
| The Build engine (*Duke Nukem 3D*) | Sectors joined by portals. |
| *Thief: The Dark Project* | Portal-based rendering, and sound propagating through rooms and doors: the shape for sound in our compartment graph. |
| *Doom 3* | Areas and portals, with closed doors cutting visibility at runtime: the shape of our portal culling. |
| Raspberry Pi documentation | The Pi 5's CPU, memory, GPU, display stack and cooling (to verify clocks, CMA defaults and throttling when writing the probe). |
| Mesa `v3d` and `v3dv` driver documentation | What OpenGL ES 3.1 and Vulkan features the Pi 5 exposes. |

## Raspberry Pi 5 graphics

Gathered 2026-10-04 for `engine-stack` ("What 60 frames a second buys on a Pi 5"). Numbers as
the sources state them; settings are often not stated, so they bound expectations and do not
replace the probe.

| Source | Cited for |
| --- | --- |
| [Raspberry Pi, "Benchmarking Raspberry Pi 5"](https://www.raspberrypi.com/news/benchmarking-raspberry-pi-5/) | glmark2 202 against 97 on a Pi 4; OpenArena timedemo 27.05 fps against 8.77 (Core Electronics' tests). |
| [Phoronix, Raspberry Pi 5 graphics review](https://www.phoronix.com/review/raspberry-pi-5-graphics/2) | glmark2 about 4.3 times a Pi 4 at 1080p; YQuake2 above 230 fps (from search results; the page refused a direct fetch, to verify). |
| [Mesa 24.3 adds Vulkan 1.3 conformance for V3DV](https://www.linuxtoday.com/blog/mesa-24-3-open-source-graphics-stack-adds-vulkan-1-3-conformance-for-v3dv/) | Vulkan 1.3 on the Pi 5 through Mesa. |
| [Xonotic forums, Xonotic on Raspberry Pi](https://forums.xonotic.org/showthread.php?tid=7724) | About 65 fps on an overclocked Pi 5 (user report, settings not stated). |
| [Jeff Geerling, an external GPU on a Raspberry Pi 5](https://www.jeffgeerling.com/blog/2024/use-external-gpu-on-raspberry-pi-5-4k-gaming/) | SuperTuxKart struggles at 1080p on maximum settings on the Pi 5's own GPU. |
| [Godot forum, Godot performance on Raspberry Pi](https://forum.godotengine.org/t/how-is-the-performance-of-godot-games-on-raspberry-pi-nowaday/112953) | Users find the Pi 5's GPU weak for 3D with a general engine's defaults. |

## Rust platform layer

Gathered 2026-10-04 for `engine-stack` ("Why SDL3, not SDL2").

| Source | Cited for |
| --- | --- |
| [SDL pull request 11511, "kmsdrm: Restore atomic support"](https://github.com/libsdl-org/SDL/pull/11511) | Merged 2025-10-19 for SDL 3.4.0; "Main's kmsdrm backend is totally broken on a Raspberry Pi 5, but the atomic version in this PR works". |
| [SDL issue 8579](https://github.com/libsdl-org/SDL/issues/8579) | Garbage on screen from the non-atomic KMS/DRM path on a Pi 5 with no X11 or Wayland (2023). |
| [`sdl3` crate](https://crates.io/crates/sdl3), [`sdl3-sys` docs](https://docs.rs/crate/sdl3-sys/latest) | Rust bindings at 0.20 (September 2026); `sdl3-sys` bundles SDL 3.4.10. |
| [SDL3 environment variables](https://wiki.libsdl.org/SDL3/EnvironmentVariables) | `SDL_KMSDRM_ATOMIC`, `SDL_KMSDRM_DEVICE_INDEX` and the other KMS/DRM hints. |

## Graphics layer, transport and matchmaking

Gathered 2026-10-07 for `engine-stack` (E2, sokol_gfx; section 10a, the browser build),
`netcode-and-sessions` (section 2, WebRTC) and `matchmaker`. Read that day; revisions pinned.

| Source | Cited for |
| --- | --- |
| [sokol](https://github.com/floooh/sokol), `sokol_gfx.h` at `401f21f` | The backends (`SOKOL_GLCORE`, `SOKOL_GLES3`, D3D11, Metal, WebGPU, Vulkan); the vertex formats (`SG_VERTEXFORMAT_*`: no three-component 16-bit format, `SHORT4`, `INT10_N2`, `UBYTE4N`, `SHORT2`); `SOKOL_EXTERNAL_GL_LOADER`; uniform blocks uploaded with `glUniform4fv` in the GL backends; GLSL `#version 410` as the desktop GL floor. |
| [sokol-rust](https://github.com/floooh/sokol-rust) at `1a5cb22` | The generated Rust bindings; a git dependency, not on crates.io; its `build.rs` compiles every module and links X11 or Wayland, ALSA and GL on Linux; web builds through `wasm32-unknown-emscripten`; its warning about stale builds after a header update. |
| [crates.io, `sokol`](https://crates.io/crates/sokol) | An unrelated binding, version 0.3.0, last updated 2019-04-29. |
| [sokol-shdc documentation](https://github.com/floooh/sokol-tools/blob/master/docs/sokol-shdc.md) | Output languages `glsl300es`, `glsl410`, `glsl430`; the `sokol_rust` output format; reflection. |
| [sokol-tools-bin](https://github.com/floooh/sokol-tools-bin) at `11d0cf6` | Prebuilt `sokol-shdc` for Linux x86-64 and arm64, macOS and Windows. |
| [str0m](https://github.com/algesten/str0m), version 0.24.1 (2026-10-03) | WebRTC in Rust, sans I/O; data channels through `sctp-proto`; crypto backends `rust-crypto` (DTLS by `dimpl`), `aws-lc-rs`, OpenSSL; the `str0m-netem` network emulator. |
| [Fly.io, "UDP and TCP"](https://fly.io/docs/networking/udp-and-tcp/) | UDP needs a dedicated IPv4 address (not shared IPv4 or IPv6), an app bound to `fly-global-services`, and the same port externally; Fly takes "a couple dozen bytes" of the MTU. |
| RFC 8831 (WebRTC data channels), RFC 8445 (ICE), RFC 8656 (TURN) | The transport and connection setup (`netcode-and-sessions` section 2a). |

## Flight and networking

| Source | Cited for |
| --- | --- |
| *Elite Dangerous* | Newtonian flight with a flight-assist switch. |
| Glenn Fiedler, *Gaffer on Games* | UDP reliability over acknowledgement bitfields, snapshot interpolation, snapshot compression. |
| Valve, "Source Multiplayer Networking" | Interpolation delay, client-side prediction and lag compensation. |

## Bridge layout

| Source | Cited for |
| --- | --- |
| *Star Trek: The Next Generation Technical Manual* (Sternbach and Okuda) | Conventions for a bridge's arrangement: flight control and operations forward of the captain, tactical behind, side stations along the walls (to verify). Shape only. |
