# Design: the Tern's hull in the engine, seen from its windows

The owner, 2026-10-11: "can you start integrateing the TERN ship model in to the game?", "so players can look out the
window na dsee their ship", "or go outside to fix pylons (which needs fixing if they are damaged in combat)". This
change is the first half: the hull in the engine and every window of the model in the decks. Going outside is
`warp-pylons`.

## 1. What a player sees

- **From deck B's side windows** (quarters and mess, three each; medbay and damage control, two each): the hull's
  flank curving away, the pylon swept out level at 1.2 m above the floor, a few metres aft, and the nacelle running
  56 m alongside. These rooms are where the crew sleep, eat and muster, so the ship is in view most of a session.
- **From deck C** (life support, cargo, the shield room, the forward switchboard): the same from 3.5 m lower, under
  the pylon.
- **From deck A** (the bridge's two forward windows, the ready room's, the briefing room's): the bow's rim and the
  forward hull; from the side rooms, down onto the pylons.
- **Outside, in the dock**: the whole ship in its girders, from the free camera (F, the walk's fly mode) and the captures.

Nothing about the interior changes when the hull appears. The hull is a mesh in the ship's frame like the rooms
(CLAUDE.md 7: the interior is decoupled from the exterior; the hull is part of the ship-local space, the sky and the
dock are what lie outside it).

## 2. The asset

`assets/models/exteriors/` (`ship-exteriors`, PR #20). The Tern is one glTF binary per detail level and livery:

| Fact | Value | From |
| --- | --- | --- |
| File | `liveries/cobalt/tern.glb`, the full model in the Cyan livery | `props.json` `default_livery` "cobalt" (the owner's preferred, README "Cyan (preferred)") |
| Triangles | 4,440 in seven primitives (materials) | `props.json`; counted again by `deckc` |
| Materials | `tern_hull` (2,647 triangles) and `tern_rim` (648) on the 1024 px hull atlas; `hazard` (713) on the 128 px graphite; `accent` (132) on the 128 px pearl; `light_panel` (156) and `screen` (144) untextured, their colour their glow | the glb's materials, `KHR_materials_unlit` |
| Light | The sun baked into `COLOR_0` (16-bit, linear, 0.24-0.99 on the hull) | the README: fixed sun, unlit materials, no normal maps |
| Frame | Metres, +x port, +y up, +z bow; origin the layout's; bounds x -22.5..22.5, y -8.02..10.02, z -44.2..37.27 | `props.json` `axes`, `bounds_m` |
| Sampling | Trilinear mips, repeat. Up close: bilinear on the painted atlases (`tern_hull`, `tern_rim`, `tern_nacelle`), nearest on the 128 px tiles (`hazard`, `accent`) | the deck's sampler; `data/ships/exteriors.json` `smooth_magnify_materials` names the atlases (design 4a) |
| Windows | 20 apertures, each with the room, centre, facing and size it was cut for | `props.json` `windows` |

The pylons' roots are on the hull's flanks at about z 2-8 m, swept out to |x| 16-19 m; the nacelles run from z 13 m
to z -44 m at |x| 16-22.4 m, centred 1.2 m above deck B's floor (measured from the glb's vertices).

## 3. The compile: the hull joins the deck file

Nacelle artwork update, 2026-10-11 (`ship-exteriors`): the full model now has seven
material parts and four images. Its dedicated nacelle image is 1024 x 2048 px.
`deckc` accepts power-of-two rectangles, expands the shorter axis by exact pixel
replication to the longer dimension, then uses the existing square-array mip chain.
Normalized UVs and the client renderer stay unchanged. The new 2048 px square layer
costs 21.33 MiB, bringing the complete runtime hull texture allocation to 26.83 MiB.
With the previously measured 58.4 MB room array, the texture estimate is about
82.53 MiB against 96 MiB. The HTML preview uses the rectangular image directly and
reports its own separate allocation. No Pi timing claim is made.

`deckc` already compiles the ship's render data into `compiled/<ship>.deck`; the hull is the ship's too, so it goes
into the same file rather than a second one. That keeps one loader and one download: the browser build, CI and
`tools/web/release.sh` need no new step.

- **Reading the glb** (`sc-tools/src/glb.rs`): the 12-byte header, the JSON chunk, the binary chunk, and accessors of
  the types the model uses (float vectors, 16-bit normalized colours, 16-bit indices); embedded PNGs decoded with the
  `png` crate the client already uses. Anything else the reader does not know (a sparse accessor, a second mesh, an
  external buffer) is an error naming the file, not a silent skip (CLAUDE.md 6.5).
- **Vertices**: the deck's 28-byte vertex, packed by `sc-core`'s one packer. Positions are stored relative to the
  model's box centre and divided by a scale (2 for the Tern), so the 81.5 m hull fits the vertex's +/-32 range at
  1/512 m (2 mm) steps; the client multiplies the scale back. The colour byte is the display multiplier
  `c^(1/2.2) / 2`, the rule the rooms' bake uses, from one shared function (it moves out of `deckc` for both). The
  three state sets are the same colour: the outside does not follow the interior's red alert. Texture coordinates are
  the glb's (within +/-11 spans); the layer is the texture's place in its array.
- **Textures**: three arrays beside the rooms' one, each with a full box-filter mip chain from the same shared code as
  the rooms': `atlas` (1024 px, one layer: the hull) and `tiles` (128 px, two layers: graphite, pearl).
- **Parts**: one index range per primitive, with its program: the deck's textured program on `atlas` or `tiles`, or
  the deck's flat program for the two glowing materials.
- **The file**: version 6 adds an optional `hull` to the index (origin, scale, its vertex and index ranges in the
  shared blobs, its parts, its two arrays' mip offsets in the texture blob). `sc-core::deck::read` checks every range
  as it does a compartment's. A ship without an exterior model compiles as before, without `hull`.
- **Checks in `deckc`**: the triangle count is the manifest's, the bounds are the manifest's within 6 mm (the manifest
  rounds them to the centimetre), and the triangles fit the Pi 5 exterior budget (12,000, the engine-stack table);
  otherwise the compile stops.

## 4. The draw

- The client builds one mesh for the hull and the three texture arrays when it loads the deck, and draws the hull's seven
  parts after the rooms, every frame, in the walk and on the drill's bridge: model matrix = translate(origin minus the
  camera, the frames rule) times scale. Drawn after the rooms, the depth test throws away almost every hull pixel
  behind a wall before it is shaded.
- The deck pipeline culls back faces; the glb's front faces are counter-clockwise like the rooms'. The hull's inner
  skin faces inward, behind the rooms' walls.
- Not drawn into the bow camera's picture: the camera sits at z 43 m, 5.7 m ahead of the bow, looking forward.
- The dock's girders stand 26 m out from the centre line; the nacelles reach 22.4 m, so the frame clears them by 2.9 m
  (`data/space/exterior.json`, `dock.width_m` 52).

### 4a. The painted atlas up close (fixed 2026-10-15)

Seen from the bridge's windows the bow showed its paint as big blocks (`docs/screenshots/engine/stations/`, the
Tamsin undock captures). Measured on the glb's UVs, the hero atlas gives the surfaces under the bridge's windows 6-12
px a metre (the dorsal deck 11.6, the bow's flanks 6-10), against the 32 px a metre the 128 px hull tiles carry and
70-160 elsewhere on the atlas. One to three metres from the eye a texel there is 20-30 px wide at 1080p, and the
deck's sampler, nearest on magnification for the tiles' texel look, drew each one as a square.

Not the cause: the level of detail (the full model is the one compiled), the mips (a full chain, and up close the
surface is magnified, so mip 0 is sampled), the bridge's 1280 x 720 target (it softens 1080p by 1.5x, it does not make
blocks).

The fix: an array carries its magnification filter (`sc_render::Magnify`), and the hull's arrays sampled by the
materials `smooth_magnify_materials` names take bilinear magnification; trilinear minification is unchanged, and the
128 px tiles keep nearest. `sc-core`'s `DeckHull::smooth_arrays` refuses a named material the hull lacks and an array a
painted and a tiling material both sample. Cost on the Pi 5: no memory, one more sampler object, and still one
texture fetch a hull fragment, now bilinear; only the hull's pixels through a window change. Its frame time is not
measured on a Pi.

What it does not do: add detail. The bow stays at 6-12 px a metre, now soft instead of blocky. More detail there means
a hero atlas that gives the bow more of its 1024 px (an unwrap and paint in Blender), or a larger atlas: at 2048 px
the hull's textures grow by 16.8 MiB, past the 96 MiB texture budget (design 7).

## 5. The windows in the decks

The four command windows are already in the runtime layout through `command_suite.json`, at exactly the model's
centres, facings and sizes (bridge port and starboard 2.0 x 1.2 m at 45 degrees, ready room and briefing room
2.0 x 1.0 m). The other 16 are in `exterior_windows.json`, which the model's preview applies and the game does not.

- **The patch becomes runtime.** The deck plan applies it last, after the command suite, deck access, engineering and
  the armory (its mode `access`, the one the export uses). `tools/exterior_windows.py` generates it through the same
  chain; today it starts from the layout and the command suite alone, so applied after deck access it would undo the
  stair towers' cut-outs in the quarters, the mess, damage control, life support and cargo.
- **The rooms grow to the hull at their windows.** The generator moves each room's outboard walls to the hull's
  service margin (0.56 m from the skin) so a window is a short opening, not a tunnel: quarters and mess grow to
  11.64 m from the centre line (from 10.9), medbay and damage control to 10.47 m (from 9.7), life support, cargo, the
  shield room and the switchboard to 11.64 m (from 10.6). Measured on the runtime rooms (after deck access): 191 m^3
  of air in all, quarters and mess 22.7 m^3 each, medbay and damage control 19.4, life support 40.6, cargo 28.2, the
  shield room and the switchboard 19.3 each.
- **The preview's furniture moves clear of the stair towers.** Four groups the preview placed where deck access puts
  a stair tower move outboard (the forward pair of quarters beds, both mess tables with their chairs) or back onto the
  tower's wall (life support's three racks); the generator checks every piece stays in its room.
- **The rooms keep their own fit-out, clear of the windows.** The patch's `furnishings` are the preview's furniture;
  the deck plan keeps its own (bunks, galley, racks), which `applyPatch` already ignores. `tools/crew_rooms.py` checks
  its furniture with the windows applied: nothing taller than a window's sill (1.1 m above deck B) stands in the
  metre of floor in front of it. The quarters' two locker banks stood on the old hull side, in front of windows 2
  and 3; they move to the two ends of the room's bunk island (recommendation taken, ask only with screenshots).
- **Baked again.** The eight rooms' shapes change, so their bake cache keys change and the export bakes them and their
  light probes again; the cache files are rewritten in the same commit.

## 6. The windows match the hull

`python3 tools/exterior_windows.py --check` (in `scripts/check.sh`, beside the layout check) regenerates the patch
and compares it with the committed one, then holds the runtime layout's windows to the model's apertures
(`props.json` `windows`): every window has an aperture with the same id, centre within 1 cm, the same size and a
facing within 1 degree, and every aperture has its window. It reads the real artifacts on both sides (CLAUDE.md 6.6).

## 7. The Pi 5 budget

| Cost | Amount | Against |
| --- | --- | --- |
| Triangles | 4,440 when any window or the outside is in view; the walk draws every room every frame today, so 4,440 every frame | Ship exterior 12,000 (LODs 3,000 and 600); 200,000 a frame |
| Draw calls | 7 | 300 a frame |
| Texture memory | 26.83 MiB including the expanded nacelle atlas and all mip levels | 96 MiB; the rooms' array is 58.4 MB (the Mac's deck rebuild, 2026-10-10) |
| Buffers | 8,730 vertices x 28 B = 0.24 MB, 13,320 indices x 4 B = 0.05 MB | 64 MB |
| Rooms | 8 rooms 191 m^3 larger; no new lights; the compiled rooms gain 881 triangles (175,611 to 176,492) and 2 texture layers (64.1 to 64.8 MB) | `tools/exterior_windows.py`, `deckc` |

Overdraw: the hull is drawn after the rooms and mostly fails the depth test behind their walls; through a window, a
pixel shows one hull surface or the sky. A desktop or cloud frame time says nothing about the Pi (CLAUDE.md 2): the
Pi measures it when the change is built.

## 8. Checks and captures

- `glb.rs`: the committed glb reads to seven primitives, 4,440 triangles and the manifest's bounds; a truncated file
  and an unknown accessor type are errors.
- `sc-core::deck`: a version 6 file with a hull round-trips; a hull range past the blob, a part past the hull's
  indices, a mip offset past the texture blob are refused.
- `deckc`: every packed hull vertex unpacks to within 2 mm of the glb's; the Tern's atlas arrays are the smooth ones
  and its 128 px tile array is not (design 4a).
- The render test: an array magnified bilinear blends its two texels a quarter of the way between them.
- `tools/exterior_windows.py --check` (design 6).
- Headless captures in `docs/screenshots/exterior/`: the quarters and the mess at a window, looking aft to the pylon
  and the nacelle; the medbay; life support on deck C; the bridge's port window; the ready room; outside in the dock
  from four quarters. Each beside the model's preview where the preview has the same view.

## 9. Not in this change

- The far detail levels (1,290 and 496 triangles): nothing in the engine sees the Tern from far away yet. They come
  with a view that does (a fighter returning, the captain's chase view).
- Going outside, pylon damage and its repair: `warp-pylons`.
- Warp glow, turret and bay-door animation, weapon occlusion and shields against the attachments: `ship-exteriors`
  tasks.
- A livery choice in the game: Cyan, the default.

## Open questions

| Id | Question | Recommendation | Status |
| --- | --- | --- | --- |
| O1 | The hull seen from the windows and from outside: approve the captures | Approve | Approved, the owner, 2026-10-16 (survey 2): "approve". The walk outside (`warp-pylons` O7) is asked with its own captures |
| O2 | The Cyan livery in the game | Cyan, the model's default and the owner's preferred | Recommendation taken (ask only with screenshots) |
| O3 | The eight lower-deck rooms grow outboard to their windows (191 m^3) | Grow them, as the window design does | Recommendation taken (ask only with screenshots) |
