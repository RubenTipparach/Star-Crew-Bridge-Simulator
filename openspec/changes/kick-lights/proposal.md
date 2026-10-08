# Proposal: kick lights, indirect blue light at the bridge's feet

## Why

The owner, 2026-10-07, on the baked bridge: "star trek likes to put in indirect floor lights, we
should have some of that on the bridge, and behind the consoles", with two reference images and
"image for reference, this cool blue tint":

- **Reference K1**, a render of a Star Trek bridge of the late 1990s series: a raised platform with
  two command chairs, its edge an overhang with a saturated blue strip hidden under it. The strip is
  never seen; the floor in front of the edge glows blue for about half a metre and fades. A console
  bank behind it has a band of blue light at its foot.
- **Reference K2**, a Star Trek film bridge: blue light pooling on the floor at the foot of every
  platform edge and every console pedestal, and lit step faces.

The bake (`light-baking` design section 15) lights the bridge from the ceiling, its coves and its
screens. Nothing lights it from the floor, so every platform edge and console base reads as one dark
line where it meets the deck, and the room has no colour of its own but the screens'.

## What Changes

- **Platform toe kicks.** Every riser and rail edge of a bridge platform is undercut at its foot by
  a recess 0.10 m high and 0.08 m deep, the wall banks' own toe kick (`build_bridge_props.py`). A
  light strip on a 45 degree chamfer at the back of the recess's ceiling faces out and down, so a
  standing crew member sees the glow on the floor, not the strip.
- **Console kicks.** Every wall bank gets a strip in the toe kick it already has, facing the room.
  Every free console (helm and tactical) gets a strip at the foot of its pedestal, front and back:
  the back one is the light "behind the consoles".
- **Light columns behind the terminals** (the owner, on the first shots: "vertical lights behind
  terminals too"): a vertical strip on the wall at each side of every wall bank, a new fixture type
  `column_strip` in the same colour.
- **A new fixture type, `kick_strip`,** in `data/lighting/fixtures.json`: an area emitter, its
  colour a new state colour `kick` in the state palette. Normal is a saturated cool blue taken from
  K1 (`#1f4bff`); red alert turns it red and emergency power amber, as every other light does
  (the alert is carried by light, `docs/analysis/star-trek-bridges.md`). The strips stay lit on
  emergency power: low enough to run from the emergency bus, they mark the floor edges to the doors.
- **Placement is data and rules.** `data/ships/tern/detailing.json` gains `kicks`: the recess and
  strip sizes, which platform edge kinds take one, and each console family's strips in prop space.
- **The deck plan** draws the recesses and the strips and bakes them as emitters in all three
  states, beside the lamps and coves.

Not changed: the ceiling lamps, the coves, the stairs' treads (lit treads are a later step, K2),
the console props' glb files.

## Capabilities

### New Capabilities
- `kick-lights`: indirect light from the foot of platform edges and consoles, as fixtures.

### Modified Capabilities
- None. `light-baking` already bakes any fixture type; `deck-pipeline` section 5a's rules gain one
  more generated detail when this lands.

## Impact

- `data/lighting/fixtures.json` (`kick_strip`), `tools/lighting_check.py` (the `kick` light).
- `data/ships/tern/detailing.json` (`kicks`).
- `docs/mockups/lib/shipkit.js` (the recesses and strips, the `kick` state colour, the covers inset
  under a recess), `docs/mockups/lib/propkit.js` (console strips), `docs/mockups/deck-plan.html`
  (the strips as emitters in the bake).
- Pi 5: 230 triangles on the bridge (about 6 a kicked platform edge, 2 a console strip), no draw call, no texture,
  no runtime light (design section 6).
