# Proposal: nebula-skies

## Why

The owner, 2026-10-15: "we also need some dope nebula backgrounds", "something with vertex colors like homeworld did
it", then "and drawing stars and simulating motion blur with gl lines", "like old school games", and "freespace 1 did
fake motion blur with vector graphics stars". And: "dope, make me some mockups to approve for the background".

Today every sky is the same: `sky.glsl` draws three hashed grids of stars, the sun and one planet over black
(`data/space/exterior.json`, `patrol-mission` 11.2). Space looks empty, every system looks alike, and a turning ship
shows nothing: the stars are crisp dots whatever the ship does. A released space game's sky has colour, a composition
and a sense of motion. Homeworld (Relic, 1999) got that from a sphere of coloured vertices with no texture at all, and
FreeSpace (Volition, 1998 and 1999) got its sense of speed from stars drawn as short lines; both fit a Raspberry Pi 5
far better than the texture skyboxes that replaced them.

## What changes

- **A nebula sky a system**, painted as a sphere of coloured vertices (no texture), refined where the colour changes
  fast, about 8,000 vertices and 16,000 triangles in one draw (design 2 and 3).
- **A generator** (`tools/space/nebula_skies.py`) that paints a sky from a recipe of layers (a galaxy band, glowing
  brush strokes, dust lanes that absorb, a bright core) into `data/space/skies/<id>.json`, deterministic, the recipes in
  `data/space/skies/recipes.json` (design 4).
- **Vector stars over it**, 1,400 to 2,400 a sky, drawn as GL lines from where each star is on screen back toward where
  it was, so a turning view streaks them: FreeSpace's rule, with its length cap and its dimming, tuned in
  `data/space/starfield.json` (design 5). The same rule draws `patrol-mission`'s motes and jump streaks.
- **The faint hashed stars stay**, hidden where the nebula is dense, and fade while the view turns fast (design 5.5).
- **The sky lights the hulls, subtly**: an ambient cube taken from the sky's colours tints the outside ambient term
  (`patrol-mission` 11.3), about a quarter of full strength on the display (design 6).
- **The sky is drawn first, with no depth, and never over a ship** (design 7).
- **Four skies for the owner to choose between** (`docs/mockups/nebula-skies.html`): A Bloom and C Ember Gate bold
  and saturated, B Quiet and D Verdigris dark and subtle.

Not in this change: a nebula you fly into (fog, Homeworld's cloud banks), animated skies, a sky per point of a system.

## Impact

- New data: `data/space/skies/` (recipes and four skies), `data/space/starfield.json`.
- New tool: `tools/space/nebula_skies.py`. The mockup inliner learns `data:space/<dir>/<name>`.
- Engine, after approval (tasks 2 to 6): `sc-render`'s sky program moves from one full-screen triangle onto the sky
  mesh, a star program is added, `sc-core::system` names a system's sky, `sc-client`'s outside ambient takes the sky's
  cube. `stars-never-over-ships` (another agent's fix in hand) is kept: design 7.
- Pi 5 cost: one more draw call than today for the stars' lines and one for their heads; about 330 KB of buffers; no
  texture; the same per-pixel work as today's sky (design 8).
