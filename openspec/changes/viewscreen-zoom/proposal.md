# Proposal: zoom on every viewscreen

## Why

The owner, 2026-10-14, on the chase camera's video and the target bracket: "add zoom controls for that window too,
1x,2x,4x, 10x, 20x".

Today only Science can zoom the viewscreen, with + and - on its SCREEN panel, doubling from 1x to 8x. Every other
console's viewscreen is fixed at 1x, and the bridge's main viewscreen always is.

## What Changes

- **Five zooms on every console's viewscreen** (design 1): 1x, 2x, 4x, 10x and 20x, as a column of five buttons beside
  the camera pad, the one in use lit. Science's + and - step through the same five.
- **Zoom narrows the lens** of whichever camera is on (design 2): the bow camera and the chase camera alike, so the
  target bracket and the camera widget's cone follow it.
- **The captain's zoom is the main viewscreen's** (design 3): it rides with his camera and CHASE to the bridge's big
  screen.

## Impact

- `sc-core::combat`: `ZOOMS`; `Command::MainView` gains the zoom's index, `Bridge::main_zoom`. `sc-net`: the command
  and the snapshot field (protocol 13, not yet released, so it is not bumped again).
- `sc-client`: the zoom column in the look band (`console/kit.rs`), the seat's zoom acts, the bridge's feed at the
  captain's zoom.
- `docs/mockups/consoles.html`: the same column and steps.
- Pi 5 budget: five UI buttons; no rendering cost (the lens only changes).
