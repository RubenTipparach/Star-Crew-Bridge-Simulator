# Design: nebula-skies

The owner, 2026-10-15: "we also need some dope nebula backgrounds", "something with vertex colors like homeworld did
it", "and drawing stars and simulating motion blur with gl lines", "like old school games", "freespace 1 did fake motion
blur with vector graphics stars", "dope, make me some mockups to approve for the background".

Mockup: `docs/mockups/nebula-skies.html` (shots in `docs/screenshots/mockups/nebula-skies-*.png`). Data:
`data/space/skies/recipes.json`, `data/space/skies/<id>.json`, `data/space/starfield.json`. Generator:
`tools/space/nebula_skies.py`.

## 1. How the old games did it (researched 2026-10-15)

Every fact here comes from a page or a source file opened on that date; what could not be confirmed says so.

### 1.1 Homeworld (Relic, 1999): a painted sphere of vertex colours

- **The tool and the idea, in Relic's words.** The BTG ("Background Tool of the Geeks") user guide, Relic's own
  documentation shipped with the source release: "The BTG is a tool that allows you to create a flat polygonal mesh,
  assign colours to each of the vertices, and place small bitmap stars. In the game, this flat mesh is mapped onto a
  giant sphere that encircles the level. Goraud shading between vertices is used to render the background."
  [btg-user-guide.rtf](https://github.com/GardensOfKadesh/Homeworld/blob/master-tenhauser/zarchive/documentation/btg-user-guide.rtf)
- **How few polygons.** The same guide: "The average poly count for a background in Homeworld was under 900 polys. The
  more polys, the slower the framerate." A background over 60 KB was not sent to players in an internet game.
- **A base colour behind the mesh.** "Even the darkest blacks in Homeworld's space were coloured slightly blue-grey."
  A uniform background colour shows where no polygon is and through vertices given alpha, "so you can still make
  colourful backgrounds but by using considerably less polygons".
- **Painted by hand.** The artist loads a reference image, places vertices along its edges, picks each vertex's colour
  from the image and links vertices into triangles; detail near the poles is avoided because the page is stretched
  there (same guide). Which artist painted which background: unconfirmed (the game's art credits name Rob Cunningham and
  Aaron Kambeitz, [Wikipedia](https://en.wikipedia.org/wiki/Homeworld), not who made the skies).
- **The format.** `BTG.h` holds `btgVertex { flags; x, y; red, green, blue, alpha, brightness }`, `btgStar` (a
  position, a colour and a bitmap name) and `btgPolygon { v0, v1, v2 }`, with the header's counts and the base colour
  ([BTG.h](https://github.com/GardensOfKadesh/Homeworld/blob/master-tenhauser/src/Game/BTG.h)).
- **How it was drawn.** `BTG.c` maps the page onto a sphere (`theta = 2.0f * M_PI_F * xFrac`, `radius =
  CAMERA_CLIP_FAR - 500.0f`), sets `glShadeModel(GL_SMOOTH)`, turns off the depth test, culling, texturing and blending,
  draws the mesh with one `glDrawElements(GL_TRIANGLES, 3 * numPolys, GL_UNSIGNED_SHORT, ...)`, then draws the BTG's
  stars as additive textured quads ([BTG.c](https://github.com/GardensOfKadesh/Homeworld/blob/master-tenhauser/src/Game/BTG.c),
  the Gardens of Kadesh port of Relic's 2003 source release; the port added vertex-buffer paths, so the 1999 code may
  have issued the same draw differently).
- **The stars over it.** Besides the BTG's bitmap stars, the game scatters `NUM_STARS = 800` and `NUM_BIG_STARS = 175`
  points on the sphere ([UnivUpdate.c](https://github.com/GardensOfKadesh/Homeworld/blob/master-tenhauser/src/Game/UnivUpdate.c)),
  drawn as `GL_POINTS` 1 to 2.8 pixels across. They never streak: nothing keeps a star's last position.
- **The light came with the sky.** A level names a background and a lighting file side by side (`LevelLoad.c`); the
  BTG guide's light mode places at most two lights "on major features of the background that seem like they should be
  emitting light", and "no lighting file that shipped with Homeworld had more than 40% ambient". So the ambient that
  lit the ships was authored to match the sky; no code samples the vertex colours (confirmed in the source).

### 1.2 Homeworld 2 (2003) and Remastered (2015)

- Homeworld 2's backgrounds "are not background or cubemap textures but tesselated spheres with vertex colors. The
  geometry is more finely tesselated in places where more precision is needed in the image"
  ([hw2view README](https://github.com/laanwj/hw2view)), stored in `.hod` files beside `_light.hod` files holding the
  lights.
- Simon Schreibt's study of them ([Homeworld 2 Backgrounds](https://simonschreibt.de/gat/homeworld-2-backgrounds/)):
  the background builder "decides whether or not to add a new vertex and color" by the image's contrast; "they painted
  only smooth gradients where the poles of the sphere are", where "the vertices are pretty rare"; the stars "are single
  textures/billboards". Homeworld 2's vertex counts: unconfirmed.
- Homeworld Remastered replaced them with a texture: "We use a single 4kx2k texture for our basic backgrounds", on "a
  raw Cube instead of much, much more optimal Sphere" (Gearbox's Bitvenom, quoted on the
  [Karos Graveyard wiki](https://github.com/hwrm/KarosGraveyard/wiki/Shaders)). A 4096 x 2048 RGBA texture is 32 MB
  before mipmaps: a third of our whole texture budget, which is why this change goes back to vertices.
- Tools since: [img2sky](https://github.com/r-lyeh-archived/img2sky), "a vertex-color mesh builder tool for skyboxes, as
  seen in HomeWorld 2 .HOD files", which "creates meshes that are more vertex dense on bright areas"; the
  [Homeworld Remastered Toolkit](https://github.com/DKesserich/Homeworld-Remastered-Toolkit) for Blender;
  [Spacescape](https://wiki.ogre3d.org/Spacescape) (layered noise and stars baked to six textures, the texture route).

### 1.3 FreeSpace (Volition): stars as vector lines

FreeSpace 2's engine source was released in 2002 and is the root of the FreeSpace 2 Source Code Project
([Wikipedia](https://en.wikipedia.org/wiki/FreeSpace_2_Source_Code_Project)). Its starfield, from Volition's own release
([StarField.cpp](https://github.com/videogamepreservation/freespace2/blob/master/code/Starfield/StarField.cpp)):

- **Stars are directions.** 2,000 unit vectors (`MAX_STARS`), 500 drawn by default (`Num_stars`, set per mission,
  scaled by the detail level). The comment: "when the ship translates, the stars do not change". So stars streak only
  when the view turns.
- **The streak.** Each star keeps `last_star_pos`, its rotated position last frame. Each frame: `dist` is the distance
  between this frame's and last frame's rotated positions (for a unit vector, about the angle turned, in radians); if
  `dist > Star_max_length` the tail is shortened to that (`ratio = Star_max_length / dist`); `ratio *= Star_amount`;
  the tail point is `p2 + (last_star_pos - p2) * ratio`; a line is drawn from tail to head (`gr_aaline` or `gr_line`).
  Defaults: `STAR_AMOUNT_DEFAULT 0.75f`, `STAR_MAX_LENGTH_DEFAULT 0.04f` (a tail of at most 0.75 x 0.04 rad, 1.7
  degrees).
- **The dimming.** `colorf = 255.0f - dist * Star_dim`, never under `Star_cap` (`STAR_DIM_DEFAULT 7800.0f`,
  `STAR_CAP_DEFAULT 75.0f`): a star loses brightness as its tail grows and keeps at least 75/255 of it. Brightness
  classes: eight, by the star's index (`i & 7`), alpha `(i + 1) * 24`.
- **A star at rest is a point.** When both ends land on one pixel the code moves one by a pixel ("if the two points are
  the same, fudge it"), so the line still draws.
- **Cuts.** `stars_camera_cut()` forgets the last positions "so blur effect doesn't draw lines all over the screen".
- **Moving forward: motion debris.** 200 small bitmaps (`MAX_DEBRIS`) round the eye, each drawn as a streak from its
  last position relative to the eye to its new one: the streak is the camera's movement that frame.
- **Today's SCP** ([starfield.cpp](https://github.com/scp-fs2open/fs2open.github.com/blob/master/code/starfield/starfield.cpp))
  keeps the rule and the cap, dims by `1 / len^(1/2.2)` of the pixel length instead of the linear fade, and draws
  streaks of 2 pixels or less as 1 pixel.
- **FreeSpace 1 against FreeSpace 2.** FreeSpace 1's own engine source was never released (only FreeSpace 2's), and the
  port that builds FS1 from FS2's code ([ptitSeb/freespace2](https://github.com/ptitSeb/freespace2/blob/master/src/starfield/starfield.cpp))
  uses the same star loop with no FS1 branch. That FS1 retail drew its stars the same way is very likely and the
  owner's memory of it agrees, but it is unconfirmed from a source. This design takes FreeSpace 2's released numbers.
- **Wing Commander** (Origin, 1990): how its stars streaked is undocumented in anything found; not used.

### 1.4 What we take

| From | We take | We change |
| --- | --- | --- |
| Homeworld | A sphere of vertex colours, Gouraud shaded, no texture, one draw, depth off, drawn first; a base colour; stars on top; ambient light authored to match the sky (at most 40%) | Painted by a generator from a recipe, not by hand; refined where the colour changes (Homeworld 2's builder); about 8,000 vertices, not 900 |
| FreeSpace | Stars as directions, drawn as alpha-blended lines from now to last frame; tail capped; dimmed as it grows, with a floor; a point at rest; cuts reset; debris streaks for forward motion | The tail is scaled to a fixed time (`tail_s`) so it does not depend on the frame rate; the floor is 0.5, not 0.29, and a streak whitens, so it reads over a bright cloud; the tail end fades; the faint hashed stars fade during a fast turn |

## 2. The sky, as the player sees it

A system has one sky, fixed to the system's frame like the stars: wherever the Tern is in the Shoals the same nebula
hangs in the same place, and turning the ship turns it past the windows. Over it: the faint hashed stars as today
(fewer and dimmer inside dense cloud), about two thousand sharper vector stars, the system's star, the near planet and
the far bodies (`patrol-mission` 11.2), all where the system file puts them from the Tern's position. When the ship
turns fast the vector stars smear into short lines and dim, and the faint stars fade; when it stops they snap back to
points. Moving fast, motes stream past. Ships, stations, fighters and the bridge always draw over the sky.

## 3. The mesh

- **Shape.** A unit sphere. It starts as an icosphere of 4 subdivisions (2,562 vertices, 5,120 triangles, edges about
  4 degrees), the icosahedron's 12 vertices and 20 faces in the generator's order, each face split in four four times.
- **Refined where the painting needs it** (Homeworld 2's builder, 1.2): each round the generator samples the sky at
  every edge's midpoint and compares it with the mean of the edge's ends (what the GPU will interpolate there); the
  worst edges over `error_bytes` (2 sRGB steps) are split, until `vertex_budget` (8,000) or no edge is over the error,
  never below `min_edge_deg` (0.35 degrees). An edge is split only when it is the longest edge of both its triangles,
  and a neighbour's longer edge is split first (Rivara's longest-edge bisection): no cracks, no T-junctions, no
  triangle thinner than the bisection allows. The result spends its vertices along dust lanes and cloud edges and
  leaves smooth gradients with few (the `A-mesh` shot).
- **Colour per vertex**: sRGB bytes, so the GPU interpolates in sRGB as Homeworld's did (no linear pipeline on the Pi's
  sky), and a fourth byte: how much of the stars behind that point the nebula hides (0-255).
- **Prefiltered.** A vertex's colour is the mean of the painting at it and at six points 0.4 degrees round it, so no
  feature is finer than the mesh can carry: no speckle between vertices.
- **Shading**: Gouraud, then a 4 x 4 ordered (Bayer) dither of one 8-bit step in the fragment shader, computed from
  `gl_FragCoord` with no texture, so a slow gradient over near-black never bands on an 8-bit target.
- **Banding, found 2026-10-16.** The first shots showed terraced contours, like a topographic map, round C's core and
  across A's cloud. The cause was not the sky: rendered in full colour the gradients are smooth even at three times the
  contrast. The bands were made by `tools/mockups/shrink_png.py`, which re-saves shots as 256-colour palettes; it now
  leaves the nebula shots in full colour. The causes it could have been, checked: the vertex colours are 8-bit, but the
  GPU interpolates them in floating point, so steps appear only at the output's own 8 bits (the dither covers that);
  the prefilter and the refinement keep neighbouring vertices within 2 sRGB steps of the painting; the noise is
  continuous (quintic-faded gradient noise). The engine draws the same way: 8-bit vertex colour, float interpolation,
  the same ordered dither.
- **Counts** (the generated files): 7,992 to 7,993 vertices, 15,980 to 15,982 triangles a sky.

## 4. The generator and the recipes

`tools/space/nebula_skies.py` reads `data/space/skies/recipes.json` and `data/space/starfield.json` and writes one file
a sky; `--check` fails on a stale file. Deterministic: integer hashes (splitmix64), no random state, so a recipe always
paints the same sky.

A sky is painted back to front in linear light, then tone mapped (`1 - exp(-colour x exposure)`, which turns a hot core
white the way film does) and written as sRGB:

| Layer | What it is | Its numbers |
| --- | --- | --- |
| space | the base colour behind everything (Homeworld's blue-grey black) | `space_srgb` |
| band | the galaxy's disc seen edge on: a great circle, brighter toward a bulge; tilted `tilt_deg` (55-62) from the planets' plane, as the Milky Way's plane is about 60 degrees from the ecliptic | `through_deg`, `tilt_deg`, `width_deg`, `colour_srgb`, `intensity`, `bulge` |
| glow | a brush stroke: a path of points joined by smoothed great-circle arcs, a soft width tapered at both ends, a heart colour and an edge colour, its edge broken into wisps by warping the direction with noise, its body varied by clumps (`grain`) and thin bright strands (`filaments`, ridged noise) | `path_deg`, `width_deg`, `taper`, `colour_srgb`, `edge_srgb`, `intensity`, `warp`, `grain`, `filaments` |
| dust | the same stroke absorbing what is painted behind it, reddening it (`redden`), so a dark lane cuts a bright cloud | `density`, `redden` |
| core | a round glow, a hot centre in a wider halo: the heart of the nebula | `at_deg`, `radius_deg`, `halo` |

**Placed with the system.** Directions are the system frame's (+y north, the planets in the xz plane), the frame of
`data/space/systems/<id>.json` and of the sky program's sun and planets. The sky never paints the star or a planet: they
move with the Tern's position and are drawn from the system file (`patrol-mission` 11.2). A recipe keeps its core and
brightest strokes off the planets' plane (latitudes 8-20 degrees in A, C and D) so the star, which always lies near that
plane, never sits in the nebula's heart, and a recipe for a system is composed against its main places: A and B against
Tamsin Station's view (the star at longitude 143, Tamsin 17.5 degrees in radius below at longitude 15), D against a gas
giant's.

**The stars** are made with the sky (5.1): directions sampled more densely along the band (`band_bias`), a few clustered
in the core (`cluster`), each dimmed by the dust in front of it, so a dark lane is dark in stars too.

**The ambient** is computed from the stored colours (6).

## 5. The vector stars

### 5.1 Count, classes and colours (`data/space/starfield.json`)

| Class | Share | Brightness | Head at rest |
| --- | --- | --- | --- |
| faint | 56% | 0.10-0.28 | a 1 px line |
| plain | 29% | 0.28-0.52 | a 1 px line |
| bright | 12% | 0.52-0.82 | a 2 px round head over the line |
| brilliant | 3% | 0.82-1.00 | a 3 px round head over the line |

Colours: blue-white 16%, white 34%, cream 28%, gold 15%, red 7%. Counts a sky: 1,400 (C) to 2,400 (B), against
FreeSpace's 500 by default and Homeworld's 975: our faint layer is still the hashed grid (5.5), so these are the stars
worth a line. Brightness is stored a star, already dimmed by the dust in front of it.

### 5.2 The streak (each frame, in the vertex shader)

For a star in direction `d`, with this frame's view rotation `R` and last frame's `R'` (the rotation only: stars do not
move when the ship translates, FreeSpace's rule):

```
moved  = angle between R d and R' d                          (radians this frame)
tail   = min(moved x tail_s / dt, max_deg)                    (tail_s 0.025 s, max_deg 1.7 degrees)
head   = project(R d)                                          (pixels)
end    = head + (project(R' d) - head) x (tail / moved)        (toward where it was)
if |end - head| < min_px: end = head + min_px along the motion (a point at rest: FreeSpace's one-pixel fudge)
dim    = max(dim_floor, 1 - dim_per_deg x tail_in_degrees)    (0.534 a degree: FreeSpace 2's 7,800; floor 0.5)
s      = smoothstep(0, 0.25 degrees, tail)                     (0 at rest, 1 streaking)
colour = mix(star colour, mix(star colour, white, whiten), s)  (whiten 0.4)
alpha  = mix(brightness, alpha_min + (1 - alpha_min) x brightness, s) x dim   (alpha_min 0.3)
blend  = source alpha over the sky (not added); the tail end's alpha x tail_fade (0.45, once the line is over 3 px)
length = never more than 1.5 x the cap's length in pixels at the view's centre (the projection stretches near the edge)
```

- **Blended, not added** (changed 2026-10-16 after the first shots). FreeSpace drew its streaks as alpha-blended lines,
  in its code's words "so that dark stars don't look bad on top of nebulas". The first mockup added them, and with
  FreeSpace 2's floor of 75/255 (0.29) a streak over the bright cloud was a faint grey dash. Blended toward a whitened
  colour, with an opacity of at least `alpha_min`, and the floor raised to 0.5, a streak reads as a pale white or
  blue-white line on the cloud and a bright one on black (`A-turn`, `A-turn-dark`). At rest nothing changes: `s` is 0,
  so a star over black is its colour at its brightness, as before.

- `tail_s` makes a turn of a given speed draw the same tail at 30 or 60 frames a second (FreeSpace's tail was one
  frame's motion, so it changed with the frame rate). 0.025 s and the 1.7-degree cap are FreeSpace 2's 0.75 x 0.04 rad
  at about 30 frames a second.
- **What a turn looks like.** At the 70 degrees a second of the `*-turn` shots a star's tail is 1.7 degrees (the cap),
  about 30 px at 1080 lines and a 50-degree view, at half its opacity. The Tern turning at 10 degrees a second draws
  0.25-degree tails, 4 px, at 87%.
- **A cut** (the camera jumps: sitting down, standing up, switching to the feed, a jump's end) resets `R'` to `R`; so
  does any turn faster than `cut_deg_s` (720 degrees a second).
- **Lines, not quads.** One `GL_LINES` draw, two vertices a star, as the owner asked. OpenGL ES 3.0 only promises a
  line width of 1 px (`GL_ALIASED_LINE_WIDTH_RANGE` may be [1, 1]); the design needs no more. The brighter classes'
  heads are a second draw of `GL_POINTS` (2-3 px, `gl_PointSize`, a round falloff), dimmed by the same `dim`. If the
  probe finds `GL_LINES` slow or ugly on the VideoCore VII, the fallback is one draw of thin quads built in the vertex
  shader (four vertices a star), measured first.
- **Not over the planet.** A star whose direction is inside the near planet's disc (`dot(d, planet) > cos(radius)`) is
  moved off screen in the vertex shader; the same test keeps stars off the star's disc.
- **Which turns streak.** Any rotation of the view: the ship turning, and the player's own head turning on the bridge
  (Q2 in 10).

### 5.3 Motes: moving forward

`patrol-mission` 11.4's motes (200 points in a 300 m cube round the eye, wrapping) are drawn by the same rule from points
in the eye's space instead of directions: a line from the mote back along the ship's velocity for `tail_s` (1/30 s),
fading beyond 150 m. That is FreeSpace's motion debris. They are world geometry, so they draw after the ships, depth
tested. The `A-forward` shot shows them at 400 m/s.

### 5.4 The jump's streaks

`patrol-mission` 11.5's 400 streaks become the star program with a jump term: the tail is stretched along the jump's
vector by its `k` (0 to 1 over the spool's last 3 s) instead of along the view's rotation. One implementation of a
streaking star (CLAUDE.md 6.1); 11.5's flash and settle stay.

### 5.5 The faint hashed stars

`sky.glsl`'s three hashed grids stay as the faintest layer (per-sky `dust_stars.density` and `brightness`), multiplied
by `1 - hide` (the vertex's fourth byte) so they thin out in dense cloud, and by a frame factor that falls with the
frame's tail (to 30% at the cap) so they fade during a fast turn instead of staying crisp beside streaking stars.

## 6. The sky's light on the hulls

Homeworld authored the ships' ambient to match each sky and never above 40% (1.1). Here the generator computes it:

- the stored colours (back to linear) projected onto nine spherical harmonic coefficients, each vertex weighted by its
  share of the sphere's area;
- the irradiance a white surface takes facing +x, -x, +y, -y, +z and -z (Ramamoorthi and Hanrahan's formula);
- scaled so the brightest face's linear luminance is the recipe's `ambient.strength` (0.045-0.07, about 0.24-0.29 on
  the display; Homeworld's 40% cap is 0.13 linear): the sky decides the colour and the way the ambient leans, the
  recipe how strong it is.

The client's outside lighting (`patrol-mission` 11.3, one ambient cube for the Hound, the Jackals, the stations and
the relay) adds this cube, in the system frame turned into each object's, in place of its flat shade-side fill. The Tern's
hull relight (11.3) gains the same cube on its ambient term. The `A-ambient-off` shot shows the hull with a flat grey fill
of the same strength instead. Interiors are untouched (baked, CLAUDE.md 9).

## 7. Drawn first, never over a ship

The owner's rule (stars must never draw in front of ships) holds by order: each pass clears, draws the sky mesh, the
faint stars, the sun and the planet in one program with the depth test and depth writes off, then the vector stars'
lines and heads, also with no depth, then clears depth (or draws everything else with the depth test on over a depth
cleared to far) and draws every ship, station, prop and room. Nothing in the sky writes depth, and nothing in the world
is drawn before it. The motes and jump streaks that live in the world are drawn after the ships with the depth test on.
The windows and the viewscreen feed are the same draw (`patrol-mission` 8 of 11.1). The other agent's fix in hand to
`draw_sky` and the drill's order is kept as it is; this change only adds to the sky pass.

## 8. The Pi 5 budget

| Item | Cost | Against |
| --- | --- | --- |
| Sky mesh | 8,000 vertices, 16,000 triangles, one draw; about 2,000-4,000 triangles inside a 70-degree view (the frustum clips the rest early) | 200,000 visible triangles a frame |
| Vector stars | 1,400-2,400 lines (2,800-4,800 vertices) and 210-350 heads: two draws | 300 draw calls |
| Faint stars, sun, planet | the per-pixel work `sky.glsl` does today, moved onto the mesh: the same fill | the sky's pass today |
| Fill | every pixel of the sky once (no overdraw: a sphere seen from inside), plus 1-3 px lines | as today |
| Buffers | positions 12 bytes, colour 4 bytes, indices 6 bytes a triangle: about 230 KB; stars about 100 KB; about 330 KB on the GPU for the loaded sky | 192 MB GPU (CMA) |
| Textures | none | 96 MB |
| Files | 275-320 KB of JSON a sky on disk, read once at a jump | |
| CPU | two rotation matrices a frame | |

Two passes a frame (the bridge view and the viewscreen feed) double the draws to six and the vertices processed to
about 26,000. Desktop numbers from the mockup are not Pi measurements; task 6 measures it.

## 9. The data

- `data/space/skies/recipes.json` (`starcrew.sky-recipes/1`): the recipes (4), the authored source.
- `data/space/skies/<id>.json` (`starcrew.sky/1`), generated, never edited: `id`, `label`, `name`, `for`, `look`;
  `mesh` (`vertices`, `triangles`, the settings it was refined with, `vertices_b64`: 8 bytes a vertex, its direction as
  two little-endian 16-bit octahedral coordinates and its RGBA bytes; `triangles_b64`: three little-endian 16-bit
  indices, counter-clockwise seen from the centre); `dust_stars`; `stars.list` (`[x, y, z, brightness, class, colour]`,
  brightest first); `ambient` (`strength`, `cube_linear` six RGB, `sh9_linear`, `dominant_srgb`).
- `data/space/starfield.json` (`starcrew.starfield/1`): the classes, colours, `streak` and `motes`.
- A system names its sky: `data/space/systems/<id>.json` gains `"sky": "<id>"` (task 3); a missing sky is an error at
  load (CLAUDE.md 6.5).

## 10. The four skies, and the owner's questions

| | Sky | For | Look |
| --- | --- | --- | --- |
| A | Bloom | the Shoals (Tamsin's home space) | Bold: a rose and violet cloud sweeping over the station, cut by dark lanes round a gold core, a teal veil above it |
| B | Quiet | the Shoals, the darker choice | Dark and subtle: a pale galaxy band with brown dust lanes, a faint rust cloud, mostly black and full of stars |
| C | Ember Gate | a deep-space jump point | Bold and hot: a red and amber cloud torn by black dust, a blue-white cluster at its heart |
| D | Verdigris | a second system (shown with a gas giant) | Dark and cold: teal and sea-green veils over a faint blue band, a pale core |

Each is shot alone (`<L>-sky`), behind the Tern (`<L>-tern`), behind Tamsin Station (`<L>-station`, an untextured
stand-in for spacedock version E built from `data/space/spacedock.json`), through the bridge's port forward window with
the viewscreen's feed beside it (`<L>-bridge`) and in a fast turn (`<L>-turn`).

Questions (to the survey, with these shots):

- **Q1. Which sky for the Shoals, A or B (a bold sky or a dark one)?** Shots `A-sky`, `A-tern`, `A-station`, `B-sky`,
  `B-tern`, `B-station`. Recommendation: A, with B kept for a later system.
- **Q2. Do the stars streak when the player turns their head on the bridge, or only when the ship turns?** Shots
  `A-turn`, `A-turn-streaks-off`. Recommendation: any turn of the view, as FreeSpace did; cuts never streak.
- **Q3. How strong the streaks: FreeSpace 2's length (a 1.7-degree cap) dimmed to half and whitened, as shown, or
  longer and brighter?** Shots `A-turn`, `A-turn-dark`, `B-turn` to `D-turn`. Recommendation: as shown.
- **Q4. The sky's colour on the hulls: on, at about a quarter of full strength on the display?** Shots `A-tern`,
  `A-ambient-off`. Recommendation: on.
- **Q5. The mesh budget: 8,000 vertices a sky, or 4,000 for the 1 GB Pi?** Shot `A-mesh`. Recommendation: 8,000 until
  the probe says otherwise (the buffers are 330 KB either way within a factor of two).

## 11. Proven, and not

- Proven by the mockup: the look, the data format round trip (the page decodes the generator's files), the streak rule
  in a vertex shader, the draw order. Not proven: anything about the Pi's speed, `GL_LINES` quality on the VideoCore
  VII, the engine's integration.
- Unconfirmed from sources: FreeSpace 1's own code (1.3), Homeworld 2's vertex counts and who painted Homeworld's skies
  (1.1, 1.2), how Wing Commander streaked its stars.
