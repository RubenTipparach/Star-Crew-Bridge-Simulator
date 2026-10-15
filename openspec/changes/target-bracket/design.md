# Design: the target bracket

## Context

| Decided elsewhere | Where |
| --- | --- |
| The look band and its viewscreen; the target's bracket on it | `bridge-stations` 8, the approved `consoles.html` |
| Consoles are glance first: one word a label, a number only where a player acts on it, colour with a shape | CLAUDE.md 10, `bridge-stations` 8.0 |
| An engine screen looks like its approved mockup | CLAUDE.md 10, `console-parity` |
| The chase camera's eye and lens, which the bracket follows with chase on | `chase-camera` 1 |
| The hull shows once Science's scan reaches 0.3 | `bridge-stations` (Science) |

The owner's reference is a frame of Starfleet Command (2000) in which the targeted ship sits inside four blue corners,
"Allied" and a bar above, its numbers below. The shape is taken, not the text (CLAUDE.md 15).

## 1. Four corners round the ship

- **The box.** Each contact class has a hull box: the minimum and maximum of its model's vertices in its own axes
  (+z its bow, +y its top), measured off the mesh the engine draws (`ships3d::bounds(&ships3d::hound())`, and the
  Jackal's), so the box is the ship, not a guess. A class with no model (the mockup's freighter) has a stand-in box in
  the mockup only.
- **Turned as the ship lies.** The contact carries its forward (`dir`) and its up (`up`), both in the Tern's axes. The
  box's 8 corners go through `rel + right * x + up * y + dir * z` (right = up x dir).
- **Projected through the viewscreen's camera**: the same eye, axes and lens the feed is drawn with (`feed_view`: the
  bow camera, or the chase camera with chase on), so the bracket sits on the ship the picture shows.
- **The rectangle** round the projected corners, padded 4 px each way and at least 16 px a side about its middle.
  When a corner is behind the camera the bracket is not drawn (the ship is beside or behind the eye).
- **Four L corners**, 1.5 px lines, each arm a quarter of the shorter side, between 5 and 14 px.

## 2. Coloured by IFF

Hostile `hostile` (red), friendly `friendly` (green), neutral `neutral` (amber): the palette's colours, the same as the
scanner's. Unknown contacts are never bracketed (as today). The shape (the corners) carries the meaning with the
colour.

## 3. Above and below

- **Above:** the IFF word in capitals, 10 px, over the left corner, in the IFF colour; and from Science's scan of 0.3,
  the hull as a bar 3 px high across the bracket's width, 3 px over its top edge, filled by the hull's share. Before
  the scan the bar is not drawn.
- **Below:** the contact's id and its range, 10 px, under the bottom-left corner: `T1 3.2 km` (one decimal in km from
  1 km, whole metres under it: `T1 640 m`). The range is a number a player acts on (weapon ranges), so it is shown.
- **Inside the screen:** when the bracket is near an edge of the viewscreen the texts move inside it, never off it.
- At most three words at rest (CLAUDE.md 10).

## 4. The mockup and the engine

`consoles.html` draws the same bracket by the same rule: its contacts gain `up`, and a `SHIP_BOX` table holds each
class's box. The engine's boxes are measured from its generators; a test reads `SHIP_BOX` out of the mockup and holds
it to them within 1 cm, so the two cannot drift (CLAUDE.md 6.6, validate the real artifact). The mockup's ship glyph is
sized to the bracket so the picture inside it is the right size.

## 5. Pi 5 budget

Eight short lines, one bar and two texts on the look band of a seated console: a few dozen UI vertices, no draw call
of its own (the console's UI batch). Nothing measurable against the budget table.

## 6. Checks

- `sc-client`: the bracket's rectangle encloses every projected corner of the Hound's box and is no more than 4 px
  wider than they are, on the bow camera and with chase on; it is at least 16 px; a box behind the eye draws nothing;
  the corners' arm length is clamped; the range text reads `640 m` and `3.2 km`.
- The mockup's `SHIP_BOX` matches the generators' bounds.
- Captures: the captain's viewscreen with the Hound bracketed near and far, TARGET with chase on and off.

## Open questions

| Id | Question | Recommendation | Status |
| --- | --- | --- | --- |
| TB1 | The text under the bracket: id and range, or also speed as Starfleet Command shows | Id and range: the range is acted on, the speed is not (glance first) | Recommendation taken (ask only with screenshots) |
