# Proposal: a custom Rust engine on an OpenGL ES 3.0 floor, measured on a Raspberry Pi 5

## Why

The owner, 2026-10-04: "we also need to decide on what game engine or stack would work for
this. Ideally a custom engine because our ship needs to decouple the bridge and internal ship
aspects from the exterior of the ship. And I'm all for low poly aesthetics." And, correcting the
hardware the same day: "I'm sorry we're running on a pi5 1gb-4gb, 4 GB can be used as main
server too".

Three facts decide most of it:

1. **The client floor is a Raspberry Pi 5 with 1 GB.** Its CPU is fast (four Cortex-A76 cores
   at 2.4 GHz) and its GPU runs OpenGL ES 3.1 and Vulkan. Speed is not the hard limit; memory
   is. The OS, the GPU's allocations and the client share 1 GB, so the game runs full screen
   from the console without a desktop and sizes its memory up front.
2. **A 4 GB Pi 5 is the main server.** The authoritative simulation runs headless there, so it
   must be a separate program that needs no GPU, built from the same core as the client.
3. **The interior must be decoupled from the exterior.** Crew walk a ship whose decks are a
   fixed local space while the ship turns and accelerates in a star system. General-purpose
   engines put everything in one scene graph and one physics world; this game needs two frames,
   explicit hand-offs between them, and a renderer that composes them (`ship-frames`). That is
   a small amount of engine, and it is the part that has to be ours.

On a Pi 5 an off-the-shelf engine could run, but it would bring a general scene graph, a single
physics world and a memory footprint the 1 GB client cannot spare. A custom engine is what the
owner asked for and what the game needs.

**Decided** (owner, 2026-10-04, on this recommendation): "That's fine..this game doesn't need high
end graphics. Your stack sounds like a solid plan". Survey question E1 is closed: the engine is
Rust with SDL and glow (SDL3, for the Pi 5's KMS/DRM path). The design was first floored on a Pi 3 with OpenGL ES 2.0; the move to
the Pi 5 raised the renderer floor to ES 3.0 and re-estimated every budget, and left the approved
stack as it was.

**Decided** (owner, 2026-10-07, after asking whether a browser build was possible and how sokol
compared with glow): "sokol it is then", "with sdl3". Question E2 is closed: the renderer draws
through `sokol_gfx` in place of glow, SDL3 stays the platform layer, and the browser becomes a
playtest target (design section 10a). The same day the owner chose WebRTC for every client:
"I want to use webrtc if posible to do multiplayer on web and desktop" (`netcode-and-sessions`).

## What Changes

- **Language: Rust** (stable, edition 2024), in a Cargo workspace, cross-compiled to the Pi.
- **Platform layer: SDL3 (3.4 or later)**, for the window and GL context (its atomic KMS/DRM backend runs full screen
  on a Pi with no desktop), keyboard, mouse, gamepads and audio output.
- **Renderer: our own, on OpenGL ES 3.0** through `sokol_gfx` (vendored at a pinned revision,
  its GLES3 backend; GL 4.1 core on Windows and macOS). Each shader is one source compiled by
  `sokol-shdc` to GLSL ES 3.00 and GLSL 4.10, with its Rust uniform structs generated. The same
  renderer runs in a browser on WebGL 2. Nothing above ES 3.0 is assumed; any extension sits
  behind a measured fallback.
- **The frame is a callback** (SDL3's main callbacks), so the client runs unchanged where the
  browser drives the frame.
- **A browser playtest build** (`wasm32-unknown-emscripten`), designed now and built after the Pi
  client, handed out through a restricted itch.io page. Never a measure of the game's speed.
- **Crates:** `sc-core` (the ship simulation, no I/O), `sc-net` (protocol and the WebRTC transport),
  `sc-render` (the renderer), `sc-client` (the game), `sc-server` (headless, the main server),
  `sc-tools` (offline compilers: `deckc`, `meshc`, the light baker) and `sc-probe` (the Pi 5
  measurement instrument).
- **The Pi 5 budget table** (design section 5) becomes the one source for frame, triangle,
  draw-call, memory, CPU and network budgets, for the 1 GB client and the 4 GB server. It is
  provisional until the probe measures it.
- **The first build step is a measurement:** `sc-probe` draws synthetic low-poly scenes on a
  1 GB Pi 5 and reports the triangle, draw-call, fill-rate and memory numbers that correct the
  table.
- **One check script**, `scripts/check.sh`, runs every check in CLAUDE.md section 12 in order.

## Capabilities

### New Capabilities

- `engine-platform`: the target hardware and its budget, the renderer's feature floor, the crate
  boundaries, the build and deployment targets, and the checks every change runs.

### Modified Capabilities

None.

## Impact

- New Cargo workspace at the repository root with the seven crates above.
- New `scripts/check.sh`.
- `third_party/sokol/` holds the vendored headers and generated bindings with their provenance.
- `bridge-stations` draws egui through our sokol_gfx painter in place of `egui_glow`.
- Every other change states its cost against the budget table this change owns.
- The mockups' budget meter (`docs/mockups/lib/shipkit.js`, `PI_BUDGET`) is checked against the
  table's marker by `tools/mockups/inline.py --check`.
- No engine code exists yet. Nothing here is built until the owner asks for it (CLAUDE.md
  section 4).
