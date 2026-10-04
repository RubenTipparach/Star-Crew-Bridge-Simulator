---
name: threejs-mockups
description: Build, inline, screenshot and publish a Star Crew three.js mockup the repository's way - one page per subject in docs/mockups, reading the one ship layout through shipkit.js, lit by its own fixtures in normal and red-alert states, showing its Pi 5 triangle and draw-call cost, with named screenshot shots checked by eye before it is shown. Use whenever making or changing a mockup, a 3D view of the ship, a console UI mockup, or anything under docs/mockups ("mock up the bridge", "show the hangar in 3D", "update the deck plan", "screenshot the mockups", "publish the mockup").
metadata:
  author: Star Crew (Claude Code)
  version: "1.0"
---

# three.js mockups

The owner, 2026-10-04: "First we need to do some extreme documentation and mockups in 3js."
A mockup is how a design is seen before it is built. CLAUDE.md section 11 has the rules; this
is how to follow them.

## The parts

| File | What it is |
| --- | --- |
| `data/ships/<id>/layout.json` | The one layout source. A mockup never hand-places a room, door, seat or system. |
| `docs/mockups/lib/shipkit.js` | The shared interpretation of the layout: `layout()`, `PALETTE` (colour roles), `LIGHTING` (normal, red alert, emergency), `PI_BUDGET`, `roomShell()` (floor, walls and ceiling with door openings cut), `hullGeometry()`, `label()`, `budgetHud()`, `titleBlock()`, `registerShots()`, `markReady()`, lookups (`compartment`, `portalsOf`, `stationsIn`, `systemsIn`, `measure`, `bounds`, `center`). Edit it here, never inside a page. |
| `docs/mockups/lib/template.html` | The page skeleton: import map (three.js 0.169.0 from jsDelivr), the INLINE markers, a scene, lighting buttons, a budget meter, a shot. Copy it to start. |
| `tools/mockups/inline.py` | Writes the layout and shipkit between each page's INLINE markers. `--check` fails on a stale page, and checks `PI_BUDGET` against the `engine-stack` table's marker. |
| `tools/mockups/shoot.mjs` | Headless Chromium (SwiftShader) screenshots: the default view, then every registered shot, into `docs/screenshots/mockups/<page>-<shot>.png`. Fails on console errors or a page that never sets `MOCKUP_READY`. |

## Make one

1. **Write the change first** (CLAUDE.md section 4). The mockup presents an OpenSpec change and
   names it in `ShipKit.titleBlock({ change })`.
2. `cp docs/mockups/lib/template.html docs/mockups/<subject>.html`, set the `<title>` (two to
   four words) and the title block.
3. Build from the layout: `ShipKit.roomShell(THREE, L, compartment)` for rooms, the layout's
   `stations`, `fixtures`, `systems`, `mounts` and `craft` for everything placed in them.
   Colours come from `ShipKit.PALETTE` roles.
4. **Light it with its own fixtures**: lamps where the ceiling fixtures are, strips, console
   glow. Provide at least normal and red alert (`ShipKit.LIGHTING`); emergency power where power
   matters. Space is dark and the sun is one light.
5. **Show the cost.** `const hud = ShipKit.budgetHud(); window.MOCKUP_BUDGET = () => hud.read();`
   and wrap every frame's renders (render-to-texture passes included) in
   `hud.beginFrame(renderer)` and `hud.endFrame(renderer)`. Merge static geometry and use
   `MeshLambertMaterial({ vertexColors: true, flatShading: true })` so the meter says something
   true about draw calls.
6. **Register shots**: `ShipKit.registerShots([{ name, setup }])`, where `setup` puts the camera,
   lighting and any simulation into a known state. Step simulations a fixed number of times in
   `setup`, never by wall-clock time, so a shot is the same every run. Call `ShipKit.markReady()`
   after the first rendered frame.
7. Inline: `python3 tools/mockups/inline.py docs/mockups/<subject>.html`.
8. Shoot: `node tools/mockups/shoot.mjs docs/mockups/<subject>.html`.
9. **Look at every shot** (Read the PNGs). Black frames, a camera inside a wall, labels in a
   heap, a panel off screen, a missing viewscreen picture: fix and reshoot. A shot you have not
   looked at is not done.

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
