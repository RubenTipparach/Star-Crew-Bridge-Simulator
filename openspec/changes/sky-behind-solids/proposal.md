# Proposal: stars never draw over ships

## Why

The owner, 2026-10-15: "make sure stars dont draw in front of ships also". Stars must never show over a ship, a
station, a missile or any solid outside, in any view, now and once the stars become streaked lines on a nebula sphere
(the star-lines mockup being designed beside this change).

An audit of every place stars are drawn (design 0) found the engine's views drawing the sky first, so they were right
by the order of their draws alone, and three real ways for the sky to land on a solid:

- **The far plane cut stations open.** The outside's far plane was 80 km, the same as the distance stations are drawn
  within. Tamsin Station reaches 12.9 km from its origin, so from 67 km out the far plane sliced it, and the stars
  showed through the cut where its hull should be; past 80 km it vanished whole.
- **The sky's depth test was off.** The sky pipeline drew with depth "always": anything drawn before it in a pass
  (a later star-streak pass, a reordered draw) would be painted over by stars.
- **Mockups put solids behind the star shell.** In `patrol.html` and `spacedock.html` the stars are points on a shell
  90 km and 120 km round the camera, and the jump's streaks a shell 900 m round it; in `ship-exteriors.html` a shell
  1 km round the origin. A camera pulled back past the shell (the orbit has no limit) saw stars, glows and streaks
  over the ships and stations.

## What Changes

- **The sky layer is behind every solid, whatever the order** (design 1): the sky's vertices on the far plane, its
  depth tested less-or-equal and never written (`sc_render::sky_layer_depth`, one copy for every sky pass, star
  streaks included).
- **One set of outside depth numbers** (design 2): `sc_render::depth` holds the outside's lens (near 1 m, far 160 km),
  the farthest a solid may stand (120 km), and the bridge's front-half and back-half maps, for the feeds, the chase
  camera and the bridge's windows alike. Stations check at load that their draw distance and reach fit.
- **A render test** (design 3) draws a ship in front of a sky packed with bright stars, through a feed's lens and
  through a bridge window, with the sky drawn first and last, and fails on any solid pixel the sky changes.
- **Mockups draw their sky layer first** (design 4), and `tools/mockups/skycheck.mjs` checks it in every shot.

## Impact

- `sc-render`: `depth.rs` (new), the sky pipeline's depth state. `sc-client`: the feed and the bridge take the
  shared lens; stations measure their reach; a patrol shot of Tamsin Station from 76 km; the render test.
- Mockups: `patrol.html`, `spacedock.html`, `ship-exteriors.html`; `tools/mockups/skycheck.mjs`.
- Pi 5 budget: nothing new to draw. The sky's depth test reads the depth buffer it already has (its fragments are
  shaded as before, since it is still drawn first); the farther far plane changes no geometry and no memory.
