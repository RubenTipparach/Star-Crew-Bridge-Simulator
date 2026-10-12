# Proposal: the Tern's hull in the engine, seen from its windows

## Why

The owner, 2026-10-11: "new ship model is in yay! can you start integrateing the TERN ship model in to the game?", then
"so players can look out the window na dsee their ship", "or go outside to fix pylons (which needs fixing if they are
damaged in combat)".

The exterior model (`ship-exteriors`, merged in PR #20) is a finished asset: `assets/models/exteriors/` holds the Tern
at three detail levels in three liveries, with 20 real window apertures cut where the furnished rooms are. The engine
does not load it (`ship-exteriors` task "Load these exterior assets in the Rust renderer in a separate implementation
request"). Inside the game today a window shows the sky and the space dock and nothing of the ship itself, and only
four rooms have windows: the bridge's two, the ready room's and the briefing room's. The other 16 windows of the model
are in a layout patch that the model's preview reads and the game does not.

This change puts the hull into the engine and the model's windows into the decks, so a player at a window sees their
own ship: the pylons and the nacelles alongside deck B, the bow from the bridge. Going outside to repair a damaged
pylon is the next change, `warp-pylons`, written alongside this one and built after the owner has seen this one.

## What Changes

- **The hull, compiled** (design 3): `sc-tools deckc` reads the Cyan livery's full model (`liveries/cobalt/tern.glb`,
  4,440 triangles, the owner's preferred livery) into the deck file beside the rooms: its triangles in the deck
  vertex, its baked sunlight as the vertex colour, its 1024 px hull atlas and its two 128 px trim textures with their
  mipmaps.
- **The hull, drawn** (design 4): the client draws it in ship coordinates (the model's origin is the layout's) after
  the rooms, in the walk and on the drill's bridge, with the deck program the rooms use. Nothing about the interior
  moves: the hull is one more mesh in the ship's frame.
- **All 20 windows, in the decks** (design 5): the model's lower-deck window patch (`exterior_windows.json`) is applied
  to the runtime layout after `deck_access`, so quarters, the mess, the medbay, damage control, life support, cargo,
  the shield room and the forward switchboard each get their windows and reach the hull at them. The deck plan applies
  it, bakes those rooms again and exports them; the engine gets them through `deckc` as today.
- **The windows match the hull** (design 6): a check holds every window of the runtime layout to a hull aperture of
  the model, and every aperture to a window, by centre, size and facing.
- **Captures** (design 8): from the quarters and the mess onto the pylons, from the bridge onto the bow, from the
  medbay and the lower deck, and from outside in the dock.

## Capabilities

### New Capabilities

- `ship-exterior`: the player ship's hull drawn in the engine from its exterior model, and the rooms' windows held to
  the model's apertures.

### Modified Capabilities

(none in `openspec/specs/`; the window patch moves from `ship-exteriors`' preview into the runtime layout, design 5)

## Impact

- `crates/sc-core/src/deck.rs`: deck file version 6, an optional hull in the index.
- `crates/sc-tools/src/glb.rs` (new), `deckc.rs`: a small glTF binary reader, and the hull compiled beside the rooms with
  the rooms' own vertex packer, colour byte and mip chain.
- `crates/sc-client/src/main.rs`, `drill_bridge.rs`: load and draw the hull.
- `tools/exterior_windows.py`: generated against the runtime chain (layout, command suite, deck access), with a
  `--check` that holds the windows to the model's apertures.
- `docs/mockups/deck-plan.html`: applies the window patch; `tools/mockups/inline.py` inlines it there.
- Pi 5 budget (design 7): 4,440 triangles and 6 draw calls, 5.76 MB of textures, 0.3 MB of buffers.
