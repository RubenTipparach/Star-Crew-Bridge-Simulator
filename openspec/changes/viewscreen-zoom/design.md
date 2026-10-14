# Design: zoom on every viewscreen

## Context

| Decided elsewhere | Where |
| --- | --- |
| The viewscreen, its camera pad and the camera widget's cone | `bridge-stations` 8, the approved `consoles.html` |
| The chase camera and its lens | `chase-camera` 1 |
| The captain's camera on the main viewscreen | `chase-camera` 2 |
| Glance first: the main action biggest, nothing touched under 24 lp, one word a label | CLAUDE.md 10 |

## 1. Five zooms

`ZOOMS = [1, 2, 4, 10, 20]` (`sc-core::combat`, beside `FEEDS`), the owner's five. Each console's look band carries a
column of five buttons, 34 x 24 lp, at x 158 from y 10, 3 lp apart, just left of the camera widget, each labelled
`1×` to `20×`, the one in use lit like the camera pad's. One press sets the zoom. Science's + and - step one place
along
the same five (from 8x, which is no longer a step, to the nearest). Tooltip: "Viewscreen zoom".

## 2. Zoom narrows the lens

As today, a zoom of z divides the tangent of the half field of view by z: the bow camera's 70 deg across at 1x is
4.0 deg at 20x; the chase camera's 100 deg across at 1x is 6.8 deg at 20x.

- **The bow camera** stands where the feed has always been drawn from, 32 m ahead of the ship's centre and 3 m up
  (`console::BOW_EYE`). TARGET aims from there, and the target bracket is projected from there: at 20x the 32 m
  between the bow and the centre would otherwise put the bracket a ship's width beside the ship (seen in the first
  capture of this change).
- **The chase camera** stands back `zoom` times as far as at 1x as its lens narrows, so the Tern keeps its size, low
  and in the middle, and what lies beyond it is magnified. Left where it stood at 1x, a zoom only blew the Tern up to
  fill the screen (the same capture).

The target bracket, the camera widget's cone and the feed all read the same zoom, so they agree.

## 3. The main viewscreen

`Command::MainView { feed, chase, zoom }`, `zoom` the index into `ZOOMS`, refused beyond it; `Bridge::main_zoom`
starts at 0 (1x). The captain's zoom presses, his camera presses and CHASE send it, and the bridge's feed is drawn at
`ZOOMS[main_zoom]`.

## 4. Pi 5 budget

Five buttons in the console's UI batch. The lens changes only the projection: no cost.

## 5. Checks

- `sc-core`: `MainView` with a zoom past `ZOOMS` is refused; the bridge keeps the captain's zoom.
- `sc-net`: the command and snapshot round trip with a zoom.
- `sc-client`: a zoom press sets each of the five; Science's + and - step along them and stop at the ends; the
  captain's press sends `MainView` with it.
- Captures: the viewscreen at 1x, 4x and 20x on TARGET.
