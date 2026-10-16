# Design: stars never draw over ships

## Context

| Decided elsewhere | Where |
| --- | --- |
| The sky shader: stars, the sun and a planet in one full-screen triangle | `deck-pipeline` 13b |
| The bridge in the front half of the depth buffer, the outside behind it | `patrol-mission` 12b |
| Stations drawn within `draw_m` of the Tern | `patrol-mission` 11.2, `data/space/station_models.json` |
| Stars as streaked lines on a nebula sphere, later | the star-lines mockup (in design beside this change) |

## 0. Where stars are drawn, and what was found

| Place | How | Found |
| --- | --- | --- |
| `sc-render` `draw_sky` (`shaders/sky.glsl`) | One triangle at the far plane, depth "always", no write | Right only while drawn first |
| Drill feeds, target and chase cameras (`drill.rs` `World::draw`) | Sky first, then the solids depth tested | Right by order; far plane 80 km |
| Bridge windows (`drill_bridge.rs`) | Sky first, bridge in the front half, outside behind | Right by order; far plane 80 km |
| Walk (`main.rs`: lobby, bow camera, ship) | Sky first in each pass | Right by order |
| Consoles (`console/`) | The gunner's sight paints its stars, then the ship glyphs; no other console paints stars; the feed is drawn under the console | Right |
| Tamsin Station from far off | Reach 12.9 km, drawn within 80 km, far plane 80 km | **Cut open from 67 km** |
| `patrol.html`, `spacedock.html` | Star points on a 90 km and 120 km shell round the camera, glows and planets in it; jump streaks on a 900 m shell | **Over the solids past the shell** |
| `ship-exteriors.html` | Star points on a 1 km shell round the origin, no depth write | **Over the ship past the shell** |
| `bridge.html`, `exterior.html`, propkit's window panes, `consoles.html` | Shell far past every solid; a far pass and a depth clear; panes facing into the room; SVG stars painted first | Right |

Captures before and after (`docs/screenshots/engine/sky-behind-solids/`): the feed and the bridge window with the
sky drawn last, under the old depth state every solid pixel starred, under the new one none. The patrol's engine
shots (helm, Tamsin Station from 14 km) showed no star on a solid: the views drew the sky first. A new patrol shot,
`p6-pair-far`, looks at Tamsin Station from 76 km, where the old far plane cut it.

## 1. The sky layer

**Rule: everything in the sky layer is drawn first, or depth tested behind every solid object; it never writes
depth.** The sky layer is the stars, the sun's disc and glow, a planet, and anything later drawn as the sky: star
streaks, motion-blurred star lines, a nebula sphere.

- The sky's vertices sit on the far plane (clip z = w, window depth 1, the value the depth buffer is cleared to).
- Its pipelines take `sc_render::sky_layer_depth`: compare less-or-equal, write off. Where a solid has written a
  nearer depth the sky fails the test, so it can be drawn before or after the solids and the picture is the same.
- It is still drawn first in every pass, as today. Drawing it last would shade fewer pixels behind the bridge's walls
  (early depth rejection); that is a speed claim to measure on the Pi first (CLAUDE.md 12), not part of this change.
- A star-streak pass follows the same rule: its lines at the far plane through `sky_layer_depth`, or drawn first
  without writing depth.

## 2. One far plane, with headroom

`sc_render::depth`:

| Name | Value | Why |
| --- | --- | --- |
| `OUTSIDE_NEAR_M` | 1 m | as before |
| `OUTSIDE_FAR_M` | 160 km | the far plane |
| `SOLID_REACH_M` | 120 km | the farthest a solid may stand from the eye |
| `OUTSIDE_DEPTH` | 0.5 | the back half of the buffer, behind the bridge (as before) |

- **A solid near the far plane ties with the sky.** In a 24-bit buffer one step is 6e-8; through a 1 m near plane a
  point d metres off is 1/d - 1/far short of the far plane, so the last 0.6 % of the range rounds to the sky's depth
  (the last 1.2 % behind the bridge, where the range is halved). Every solid therefore stays within `SOLID_REACH_M`,
  where it is at least 17 steps in front of the sky through either lens (`in_front_of_sky` asks for four).
- **Stations fit.** A station is drawn while its origin is within `draw_m` (80 km) of the Tern. At load,
  `StationsView` measures each station's reach from its drawn vertices (Tamsin Station 12.9 km, the placeholders under
  2 km) and refuses one whose `draw_m` + reach + 2 km (the eye's stand off the Tern) passes `SOLID_REACH_M`: 94.9 km
  for Tamsin Station. The relay is drawn within 80 km.
- **Resolution is kept.** The near plane sets it: behind the bridge a metre at 2 km and 5 cm at 200 m are still a step
  or more apart.
- The feed's lens, the chase camera's and the bridge window's outside all come from `outside_lens`; the bridge's
  front-half and back-half maps moved from `drill_bridge.rs` to `sc_render::depth` unchanged.

## 3. The render test

`crates/sc-client/tests/render.rs`, through a real GL context: a dark box (the ship) in front of a sky whose every
cell holds a bright star, 256 x 256 px, six cases: a feed's lens and a bridge window (a wall 5 m off in the front half
covering part of the ship), each with the sky drawn first, drawn last, and with the ship at `SOLID_REACH_M` and the sky
last. Each case is drawn without the sky, then with it; every pixel a solid covers must be unchanged (each channel
within 3), and the sky must really be there (over 200 star-bright pixels round the solids). A later sky pass joins
`sky_layer` in the test and is held to the same rule. Under the old depth state the four sky-last cases fail with
over 99 % of their solid pixels starred.

## 4. Mockups

A page with a sky lists its sky groups in `window.MOCKUP_SKY`, and draws them first: every material in them writes no
depth and sits in three.js's opaque list (its transparent list is drawn after every opaque object), with the group's
`renderOrder` -1 so they sort before every solid; additive glows keep their blending. `patrol.html` (the stars, the
sun, the planets and the jump's streaks; the jump flash becomes additive at the same strength), `spacedock.html` (the
stars, glows and Tamsin), `ship-exteriors.html` (the stars). `tools/mockups/skycheck.mjs` loads each page, applies
every named shot and fails when a sky drawable writes depth, is transparent, or sorts after a solid.

The two pages' `skyLayer` helper is the same short function twice; it belongs in `lib/shipkit.js` once the inliner is
free to rewrite every page's copy (it was being changed by other work when this landed).

## 5. Pi 5 budget

No geometry, draw call, texture or memory added. The sky's fragments are shaded as before (it is still drawn first);
the depth test it now makes reads the buffer the pass already has.
