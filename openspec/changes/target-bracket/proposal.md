# Proposal: the target bracket frames the target

## Why

The owner, 2026-10-14, on the chase camera's video, with a frame of Starfleet Command beside it: "the targetting
bracket is messed up still here;s what it looks like in starfleet command".

What the viewscreen draws today, in the engine and in the approved `consoles.html` alike, is a square dashed `6 gap`
from 3 px in, meant to leave only its corners. The gap is the side less 12 px, but a dash pattern runs on round the
whole outline, so after the first side the dashes stop meeting the corners: the picture is eight loose ticks scattered
round a point, not four corners. And its size is a number from the range (60 px at 1 km, between 5 and 70 px), not the
ship: the Hound at 300 m sits in a box too small for it, and at 3 km in one far too large.

Starfleet Command's bracket, in the owner's frame, is four corners closed round the ship's own outline, coloured by
what it is (blue for an ally there), with one word and a bar above it and the contact's numbers below.

## What Changes

- **Four corners round the ship** (design 1): the target's hull box, as its model measures, turned to the way the ship
  lies, is projected through the viewscreen's camera (bow or chase); the bracket is the rectangle round it with 4 px to
  spare, at least 16 px a side, drawn as four L corners whose arms are a quarter of the shorter side (5 to 14 px).
- **Coloured by IFF** (design 2): hostile red, friendly green, neutral amber, the colours every console already uses.
- **A word and a bar above, the name and range below** (design 3): the IFF word ("HOSTILE") over the left corner, the
  hull as a thin bar across the top once Science's scan shows it, and the contact's name and range under the bottom
  edge ("T1 3.2 km").
- **The mockup and the engine together** (design 4): `consoles.html` draws the same bracket from the same rule, so the
  parity check holds.

## Impact

- `sc-client`: `console::ContactView` gains `up` and `box_m` (serde defaults, so saved fixtures still load);
  `seat.rs` fills them from the Hound's and the Jackal's orientation and from their meshes' measured bounds
  (`ships3d::bounds`); the look band draws the new bracket (`console/mod.rs`).
- `docs/mockups/consoles.html`: the same bracket, its ship boxes copied from the generators and checked against them by
  a test.
- Pi 5 budget: a handful of UI lines and two short texts per frame on a seated console. Nothing measurable.
