---
name: threejs-mockups
description: Build, inline, screenshot and publish a Star Crew three.js mockup the repository's way - one page per subject in docs/mockups, reading the one ship layout through shipkit.js, lit by its own fixtures in normal and red-alert states, showing its Pi 5 triangle and draw-call cost, with named screenshot shots checked by eye before it is shown. Use whenever making or changing a mockup, a 3D view of the ship, a console UI mockup, or anything under docs/mockups ("mock up the bridge", "show the hangar in 3D", "update the deck plan", "screenshot the mockups", "publish the mockup").
metadata:
  author: Star Crew (Claude Code)
  version: "2.0"
---

# three.js mockups

The owner, 2026-10-04: "First we need to do some extreme documentation and mockups in 3js."
A mockup is how a design is seen before it is built. CLAUDE.md section 11 has the rules; this
is how to follow them.

## The parts

| File | What it is |
| --- | --- |
| `data/ships/<id>/layout.json` | The one layout source. A mockup never hand-places a room, door, seat or system. |
| `docs/mockups/lib/shipkit.js` | The shared interpretation of the layout (schema `starcrew.ship-layout/2`: rooms are convex prism brushes that follow the hull; portals carry a normal). `compartmentMesh()` builds a room as one textured mesh: the shell with every opening cut, plus the generated detail (frames, coves, beams, baseboards, door and window frames, conduits, railings, ladders, rims, lamp fixtures) from `data/ships/<id>/detailing.json`. `loadMaterials()` decodes the Material Maker layers into one texture array, `surfaceMaterial()` draws with it (lit, or `lit: false` for baked vertex light), `bakeDirect()` is the stand-in bake, `lampsFor()` the lamp rule. Also `PALETTE` (colour roles), `LIGHTING` (normal, red alert, emergency), `PI_BUDGET`, `hullGeometry()` (textured with `{ mats }`), `label()`, `budgetHud()`, `titleBlock()`, `registerShots()`, `markReady()` (which also gives every floating panel its close button: see below), `applyPatch()` (a layout patch such as `command_suite.json` applied to a copy of the layout), and lookups and geometry (`compartment`, `portalsOf`, `portalFrame`, `portalPoint`, `wallsOf`, `measure`, `bounds`, `center`, `brushAt`, `floorAt`, `insidePoly`, `chord`). Edit it here, never inside a page. |
| `docs/mockups/lib/freecam.js` | The one free camera (owner: "I want to be able to look around, or orbit camera or pan around instead of fixed camera angles too"): orbit, pan and zoom, or look around and move at eye level, or walk on the deck. Every interior page gives the viewer one, from wherever its fixed views put the camera. Inlined by `<!-- INLINE lib:freecam -->`; the page imports three's `OrbitControls` and passes it in. |
| `docs/mockups/lib/shipwalk.js` | Walking a whole ship in first person, deck to deck (owner: "do you have fps mode in deck plan for me to run arond?"): crew-on-deck's body (0.30 m capsule, eye at 1.65 m, 0.35 m steps) at its speeds, colliding with the rooms as drawn through an Octree of their meshes (`ShipWalk.octreeOf`, with covers over floor openings, windows and doors to space; a placed prop by its bounds), climbing ladders and hatches with E, riding a lift. Keyboard and mouse (pointer lock or drag), or a stick and buttons on a touch screen. The deck plan's Walk button uses it, and `window.MOCKUP_WALK` drives it headless for checks. Inlined by `<!-- INLINE lib:shipwalk -->`; the page imports three's `Octree` and `Capsule` and passes `Capsule` in. freecam.js's one-floor walk is the older copy: a page with one floor may keep it until it moves onto this. |
| `docs/mockups/lib/propkit.js` | The Blender props in a page: loads the prop sets the page carries (`<!-- INLINE models:<set> -->`), picks a station's own variant of a prop, places a prop by its back and the way it faces (`placeProp`, with an accent colour), seats a crew figure (`placeCrew`), and draws the console faces of `bridge-stations` 11.6 in one atlas (`addFaces`, `faceMat`; a `strip` screen tiles the station's images along a table's inlay). The bridge variants, the command deck and the deck plan use it; a box of `opts.sizes[kind]` stands in for a prop the page does not carry. A prop's `light_panel` strips glow. `spaceViews(layout, opts)` puts the starfield on every viewscreen and in every window (one copy for every page). Inlined by `<!-- INLINE lib:propkit -->`; the page imports `GLTFLoader` and passes it in. |
| `data/ships/<id>/detailing.json` | The detail rules' sizes and the finish table (which material each generated surface takes). Inlined by `<!-- INLINE data:<ship>/detailing -->`. |
| `data/materials/materials.json`, `assets/textures/` | The surface materials (the `material-maker` skill). Inlined by `<!-- INLINE materials -->`. |
| `docs/mockups/lib/template.html` | The page skeleton: import map (three.js 0.169.0 from jsDelivr), the INLINE markers, a scene, lighting buttons, a budget meter, a shot. Copy it to start. |
| `tools/mockups/inline.py` | Writes the layout, data files, materials, shipkit and Blender-built props (`<!-- INLINE models:<set> -->`: `assets/models/<set>/props.json` and its `.glb` files, read by the page with three.js's `GLTFLoader`) between each page's INLINE markers. `--check` fails on a stale page, and checks `PI_BUDGET` against the `engine-stack` table's marker. |
| `tools/mockups/shoot.mjs` | Headless Chromium (SwiftShader) screenshots: the default view, then every registered shot, into `docs/screenshots/mockups/<page>-<shot>.png`. Fails on console errors or a page that never sets `MOCKUP_READY`. |
| `tools/mockups/shrink_png.py` | Re-saves screenshots as dithered 256-colour PNGs, about a quarter of the size; run it on `docs/screenshots` before committing shots. Leaves shots already shrunk alone. |

## Panels a viewer can close

The owner, 2026-10-06: "There's lots of popups, cluttered on my phone. Add little x marks so I can close some of
these and get a better view." `markReady()` puts a small close button on the corner of every floating panel: the
title block, the Pi 5 meter, and any element of class `controls`, `info`, `legend` or `route`, id `side`, or with a
`data-closable` attribute. Give a new floating panel one of those, never its own close code. A closed panel is
hidden, a "Show closed panels" pill at the top brings them back, and the viewer's choice is remembered in their
browser. On a phone (narrower than 760 px) a page opens with only its controls showing. The buttons sit in a layer
of their own, so a page may rewrite a panel's contents freely.

## Make one

1. **Write the change first** (CLAUDE.md section 4). The mockup presents an OpenSpec change and
   names it in `ShipKit.titleBlock({ change })`.
2. `cp docs/mockups/lib/template.html docs/mockups/<subject>.html`, set the `<title>` (two to
   four words) and the title block.
3. Build from the layout: `const mats = await ShipKit.loadMaterials(THREE)`, then
   `ShipKit.compartmentMesh(THREE, L, compartment, mats, { material })` for rooms (one draw call
   each, textured and detailed), the layout's `stations`, `fixtures`, `systems`, `mounts` and
   `craft` for everything placed in them. Rooms are polygons: use `bounds`, `center`,
   `insidePoly`, `floorAt` and `portalFrame`, never a box. Props may be flat colours from
   `ShipKit.PALETTE` roles, or textured with `geometryOf` and a finish.
4. **Light it with its own fixtures**: lamps where the ceiling fixtures are, strips, console
   glow. Provide at least normal and red alert (`ShipKit.LIGHTING`); emergency power where power
   matters. Space is dark and the sun is one light.
5. **Show the cost.** `const hud = ShipKit.budgetHud({ textureBytes: mats.bytes }); window.MOCKUP_BUDGET = () => hud.read();`
   and wrap every frame's renders (render-to-texture passes included) in
   `hud.beginFrame(renderer)` and `hud.endFrame(renderer)`. Merge static geometry (one mesh per
   compartment, as the engine draws it) so the meter says something true about draw calls.
6. **Register shots**: `ShipKit.registerShots([{ name, setup }])`, where `setup` puts the camera,
   lighting and any simulation into a known state. Step simulations a fixed number of times in
   `setup`, never by wall-clock time, so a shot is the same every run. Call `ShipKit.markReady()`
   after the first rendered frame.
7. Inline: `python3 tools/mockups/inline.py docs/mockups/<subject>.html`.
8. Shoot: `node tools/mockups/shoot.mjs docs/mockups/<subject>.html`.
9. **Look at every shot** (Read the PNGs). Black frames, a camera inside a wall, labels in a
   heap, a panel off screen, a missing viewscreen picture: fix and reshoot. A shot you have not
   looked at is not done.
10. Shrink the shots before committing them: `python3 tools/mockups/shrink_png.py`.

## Publish

- Mockups are self-contained after inlining (three.js comes from jsDelivr), so a page publishes
  as one artifact file. The publisher adds its own document wrapper, so publish the copy that
  `python3 tools/mockups/artifact_copy.py docs/mockups/<page>.html <scratchpad dir>` writes
  (wrapper removed, title first, dark colour scheme kept), from the same scratchpad path every
  time so the URL stays the same. Load the `artifact-design` skill before the first publish, give it an
  icon, and keep republishing to the same URL.
- Record each mockup's artifact URL in `docs/design/README.md` beside the change it presents,
  and end the reply with the links (CLAUDE.md section 13).

## Keep in mind

- **A mockup is not a Pi measurement.** The meter shows the triangles and draw calls the design
  would spend; the frame rate is a desktop's, and the page says so.
- **No shadow maps, no post-processing** that the Pi 5 renderer will not have; they also fail on
  SwiftShader.
- **Screens and consoles** are HTML and CSS overlays in fixed-size panels (CLAUDE.md 10), not
  textures rendered every frame.
- **Never hand-edit between INLINE markers.** Change the layout or shipkit and re-run the
  inliner; `--check` catches a page that drifted.
- **No em or en dashes** in page text either (CLAUDE.md 5).
