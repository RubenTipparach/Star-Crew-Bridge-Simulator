# Proposal: a custom Rust engine on an OpenGL ES 2.0 floor, measured on a Raspberry Pi 3

## Why

The owner, 2026-10-04: "we also need to decide on what game engine or stack would work for
this. Ideally a custom engine because our ship needs to decouple the bridge and internal ship
aspects from the exterior of the ship. And I'm all for low poly aesthetics. Another hard
requirement is I want this to run on pi3 1gb ram!"

Three facts decide most of it:

1. **The Pi 3's GPU stops at OpenGL ES 2.0.** Its VideoCore IV offers OpenGL ES 2.0 (and
   desktop OpenGL 2.1 through Mesa's `vc4` driver), and nothing newer: no ES 3.0, no Vulkan.
   Every mainstream engine the owner already uses needs more. Bevy (Pale-Blue-Dot) renders
   through wgpu, whose OpenGL backend needs ES 3.0. Godot 4 (Undercity) has a Compatibility
   renderer that needs OpenGL 3.3 or ES 3.0. Neither can draw a frame on a Pi 3.
2. **1 GB is shared by the OS, the GPU, the client and possibly the server.** A browser
   (three.js in Chromium) or a desktop session spends a large part of that before the game
   loads. The game must run full-screen from the console, without X11 or Wayland, and size
   its memory up front.
3. **The interior must be decoupled from the exterior.** Crew walk a ship whose decks are a
   fixed local space while the ship turns and accelerates in a star system. General-purpose
   engines put everything in one scene graph and one physics world; this game needs two
   frames, explicit hand-offs between them, and a renderer that composes them (the
   `ship-frames` change). That is a small amount of engine, and it is the part that has to
   be ours.

A custom engine is therefore not a preference but what is left. The question is what it is
built from.

**Decided** (owner, 2026-10-04, on this recommendation): "That's fine..this game doesn't need high end graphics. Your stack sounds like a solid plan". Survey question E1 is
closed: the engine is Rust with SDL2 and glow on OpenGL ES 2.0.

## What Changes

- **Language: Rust** (stable, edition 2024), in a Cargo workspace, cross-compiled to the Pi.
  C11 with raylib is the documented runner-up (design section 2).
- **Platform layer: SDL2**, for the window and GL context (its KMS/DRM backend runs
  full-screen on a Pi with no desktop), keyboard, mouse, gamepads and audio output.
- **Renderer: our own, on OpenGL ES 2.0** through the thin `glow` bindings, with GLSL ES 1.00
  shaders. The same renderer runs on desktop OpenGL 2.1. No feature above the ES 2.0 floor is
  assumed; any extension sits behind a measured fallback.
- **Crates:** `sc-core` (the ship simulation, no I/O), `sc-net` (protocol and transport),
  `sc-render` (the GLES2 renderer), `sc-client` (the game), `sc-server` (headless), `sc-tools`
  (offline compilers such as `deckc`) and `sc-probe` (the Pi 3 measurement instrument).
- **The Pi 3 budget table** (design section 5) becomes the one source for frame, triangle,
  draw-call, memory, CPU and network budgets. It is provisional until the probe measures it.
- **The first build step is a measurement:** `sc-probe` draws synthetic low-poly scenes on a
  Pi 3 and reports the triangle, draw-call and fill-rate curves that correct the table.
- **One check script**, `scripts/check.sh`, runs every check in CLAUDE.md section 12 in order.

## Capabilities

### New Capabilities

- `engine-platform`: the target hardware and its budget, the renderer's feature floor, the
  crate boundaries, the build and deployment targets, and the checks every change runs.

### Modified Capabilities

None.

## Impact

- New Cargo workspace at the repository root with the seven crates above.
- New `scripts/check.sh`.
- Every other change states its cost against the budget table this change owns.
- The mockups' Pi 3 meter (`docs/mockups/lib/shipkit.js`, `PI3_BUDGET`) is checked against the
  table's marker by `tools/mockups/inline.py --check`.
- No engine code exists yet. Nothing here is built until the owner asks for it (CLAUDE.md
  section 4).
