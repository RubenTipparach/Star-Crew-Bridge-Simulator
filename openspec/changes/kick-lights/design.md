# Design: kick lights

## Context

The owner, 2026-10-07: "star trek likes to put in indirect floor lights, we should have some of
that on the bridge, and behind the consoles", with references K1 and K2 (proposal) and "this cool
blue tint". The bridge is the round bridge of `command-suite`: two ring platforms along the walls
(0.45 m up, the wall banks on them), the command platform behind (0.45 m, the captain's chair), and
the sub-platform in front (0.225 m, helm and tactical at free consoles). Its platforms are
`data/ships/tern/command_suite.json` `bridge.platforms`, each edge a kind: `wall`, `riser`, `rail`
or `step` (`ShipKit.buildPlatforms`).

### What the references show (take the shape, not the art: CLAUDE.md 15)

| Reference | Where the light is | What is seen |
| --- | --- | --- |
| K1 | Under the overhang of a platform's edge, at floor level; at the foot of a console bank | A blue band on the floor, brightest at the edge, gone in about 0.5 m; the source never |
| K2 | At the foot of every platform edge and console pedestal; on step faces | Blue pools round every raised thing; white light in the steps |

Both: the source is hidden, the colour is saturated, and it is the floor that glows.

## Goals / Non-Goals

**Goals:** indirect blue light at the foot of every bridge platform edge a crew member can walk to,
and at every bridge console's foot, front and back where it stands free; vertical light columns
behind the terminals (section 2a); in the bake in all three states; data, not code, for sizes,
colours and placement.

**Non-goals:** lit stair treads (K2's step faces: a later step); kick lights outside the bridge
(the same rules apply anywhere once a room has platforms or consoles, but only the bridge is drawn
with them now); runtime lights.

## Decisions

### 1. Platform toe kicks

On every platform edge whose kind is in `kicks.platform.on` (`riser` and `rail`), the riser face
stops `height_m` (0.10 m) above the floor and the platform is undercut below it by `depth_m`
(0.08 m): the same toe kick the wall banks have (`build_bridge_props.py`, `toe_d` 0.08 m and `toe_h`
0.10 m), so the bridge has one kick.

Each kicked edge, in section (out is away from the platform):

| Face | From | To | Faces |
| --- | --- | --- | --- |
| Riser (upper) | the floor plus `height_m` | the platform's top | out |
| Soffit | the edge | `depth_m - chamfer_m` in, at `height_m` | down |
| Strip (chamfer) | the soffit's inner edge | `chamfer_m` (0.03 m) lower, at `depth_m` in | out and down, 45 degrees |
| Back | the floor | `height_m - chamfer_m` | out, at `depth_m` in |

- **Corners.** Where two kicked edges meet, the recess's inner line is the mitre of the two inset
  lines, so the back faces meet and no face overlaps another (CLAUDE.md 8). Where a kicked edge
  meets an edge that is not kicked (a `step` where a stair lands, or a `wall`), the recess is closed
  by an end face in the plane of the end.
- **The floor in the recess.** The floor cells stop where a platform hides them (`floor-panels`
  design 6). Under a kicked edge the platform's cover is its outline moved in by `depth_m`, so a cell
  that the recess opens is drawn.
- **The nosing and the rail** stay on the platform's top edge, so the hazard strip still marks it;
  the riser above the kick keeps its fitted panel row.
- **Not on `step` edges:** a stair's top tread lands there, and a recess under it would undercut
  the stair.
- **Walking** is unchanged: the walk mode's collider is the platform's outline, and an 8 cm undercut
  at the toe is where a foot goes anyway.

### 2. Console kicks

`kicks.props` lists, for each console family, its strips in prop space (props.json: +Y up, +Z
towards the operator, the origin on the floor at the centre of the back):

| Family | Strip | Where (prop space) |
| --- | --- | --- |
| `wall_bank` (all widths) | front | In the toe kick, 1.2 cm proud of its back face (z 0.532 m), y 0.005-0.095 m (it covers the white light panel the toe kick's back already shows), the bank's width less 3 cm each end |
| `free_console` | front | On the pedestal's front face at its foot, just above the plinth (y 0.06-0.10 m), 1.2 cm proud, x within 0.34 m of the centre, facing the way the face leans (down 13 degrees) |
| `free_console` | back | On the pedestal's back face at its foot (y 0.06-0.10 m, z -0.012 m), x within 0.34 m, facing back: the light "behind the consoles" |

A strip is drawn by the kit (`PropKit.placeKicks`) after the prop is placed, with the same turn and
move, so the prop's glb is unchanged. A wall bank's strip lies in front of the face its toe kick's
back already shows; the 1.2 cm keeps the two clear of each other (CLAUDE.md 8). When the props are
next rebuilt, the strips move into the generator: `build_bridge_props.py` writes each prop's kick
strips into `props.json` (task 3.1), and the table here becomes the generator's.

### 2a. Light columns behind the terminals

The owner, on the first shots: "vertical lights behind terminals too", after K1, where a console
bank stands against panels of vertical blue light, and the first series' upright frames that carry
light strips (`docs/analysis/star-trek-bridges.md`, R7). Every wall bank gets a column on the wall
at each side:

| Item | Value |
| --- | --- |
| Where (prop space) | 3-8 cm out from the bank's bounds each side (its fins), y 0.15-2.05 m, 2.2 cm off the wall (z 0.012 m; the bank's back stands 1 cm off it) |
| Width | 5 cm |
| Fixture type | `column_strip`: area, 1500 cd/m^2, range 3.0 m, the `kick` colour, lit on emergency power at half |

A column is seen directly, unlike a kick strip: it is the bridge's blue seen at eye height, and it
lights the bank's fins and the wall either side. It stands on the wall plane, so the room's wall
panels behind it keep their texture; a rib that stands where a column falls hides part of it (the
shots show none on the Tern's bridge).

### 3. The fixture type

`data/lighting/fixtures.json` gains `kick_strip`:

| Key | Value | Why |
| --- | --- | --- |
| `kind` | `area` | A strip lights the floor as a face, as a cove strip does |
| `luminance_cd_m2` | 3000 (judged on the shots, section 7) | A narrow strip that has to throw a band about 0.5 m onto the floor |
| `range_m` | 2.5 | Its light is gone well inside that; the bake stops tracing it there |
| `light` | `kick` | The new state colour (below) |
| `emergency` | `always` | Path light to the doors on emergency power |
| `emergency_scale` | 0.7 | Dimmer than its share on emergency power |

**The state colour** (`ShipKit.LIGHTING`, the one palette every mockup shares):

| State | `kick` | Weight |
| --- | --- | --- |
| Normal | `#1f4bff`, the blue of K1 | 1.0 |
| Red alert | `#ff2020`, the strips' red | 1.0 |
| Emergency power | `#ff8a1c`, the emergency amber | 0.8 |

Red alert and emergency power follow the state, as every other light in the ship: "Alert is light,
not geometry" (`docs/analysis/star-trek-bridges.md`). Keeping the strips blue on red alert is a
change of one colour in the palette, no rebake of geometry. Recommendation taken (ask only with
screenshots); the shots in section 7 show it.

The validator (`tools/lighting_check.py`) learns `kick` as a light.

### 4. The bake

The deck plan's bake (`light-baking` design 15) takes every strip as an emitter of `kick_strip`
beside the cove strips: the platform strips from the kit's recesses, the console strips from
`placeKicks`. The visible strip is a drawn light (role `kick`), like a lamp's lens: it takes the
state's `kick` colour and is not lit by the bake. The quick light (`?bake=0`) draws the strips but
does not light the floor with them: it is the stand-in until the bake arrives.

### 5. In the engine

The kit's rule becomes a generated detail in `deckgen` (`deck-pipeline` section 5a): the recess is
three more faces on a platform edge brush and the strip a fixture record (`type: kick_strip`,
`from_m`, `to_m`, `facing`), as the lamps are. The console strips come from the prop manifests. The
bake treats them as any area fixture (`light-baking` section 11).

### 6. The Pi 5 budget

| Item | Cost |
| --- | --- |
| Triangles | 230 on the bridge, measured (section 7): 6 a kicked platform edge (soffit, strip, back) and 4 an end face, 2 a console strip or column; 0.8 % of the bridge's 30,000 |
| Draw calls | None: the strips are vertices in the compartment's one mesh, their colour per state in the baked sets |
| Texture memory | None: the strips take the light panel layer the status strips use |
| Runtime light | None: baked |
| Bake time | One more emitter a strip: the bridge's bake from 15.7 s to 17.4 s on the cloud CPU (section 7), not a Pi or engine number |

### 7. What the mockup shows (2026-10-07)

The deck plan with the kick lights and columns, baked whole by `tools/mockups/bake_ship.mjs`
(`docs/benchmarks/2026-10-07-kick-lights/report.md`), against the same bake without them
(`docs/benchmarks/2026-10-07-tern-bake/`), in this cloud container (headless Chromium, SwiftShader,
one thread: never a Pi 5 or engine number).

| Quantity | Without | With |
| --- | ---: | ---: |
| Bridge emitters (coves; then coves, kicks, columns) | 14 | 67 (14 + 37 kicks + 16 columns) |
| Platform edges kicked | 0 | 25, with 6 end faces |
| Bridge triangles | 11,525 | 11,755 (+230: 174 platform, 56 console strips) |
| Bridge bake time | 15.7 s | 17.4 s |
| Every other room | | Unchanged: the same triangles and the same bake digest |

**The first shots, and what changed after them:**
- At `#2a64ff` and 2400 cd/m^2 the floor's wash read pale cyan and white where it was brightest:
  the bake's encoding (`light-baking` section 5, the shoulder) carries a strong blue with green in
  it towards white. A deeper blue, `#1f4bff`, keeps it blue where it is bright, and 3000 cd/m^2
  makes the band read at the reference's strength.
- A wall bank's strip at y 0.05-0.09 m left the white light panel of its toe kick's back showing
  above and below it; the strip now covers the back.
- Then the owner: "vertical lights behind terminals too" (section 2a).

**The shots** (`docs/screenshots/mockups/kick-lights/`: before in normal light, then with the kick
lights in the three states):
- `walk-bridge-kick-ring`: the ring's rail edge throws a blue band along its vent grilles; the wall
  banks' toe kicks glow; a light column stands each side of every terminal.
- `walk-bridge-kick-helm` and `walk-bridge-kick-front`: the helm's and tactical's consoles glow at
  their feet, front and back; the sub-platform and the command platform glow along their edges.
- `walk-bridge-helm-chairs` and `walk-bridge-forward`: the room as a whole, the blue under every
  edge as in K1 and K2.
- `walk-bridge-science-columns`: the light columns close to, in normal light and on red alert.
- On red alert everything turns red with the room; on emergency power the strips outline every
  platform edge in amber, the path to the doors.

## Risks / Trade-offs

- **A blue the owner did not mean.** K1's blue is saturated; the shots show it in place, and the
  colour is one value in the palette.
- **The 1 m bake split** is coarse for a band 0.5 m wide: the floor in front of an edge gets one
  row of vertices inside the band. The edge bands at the platforms' feet (`floor-panels` design 6)
  are split finer than the floor (0.3125 m wide), which helps; the engine's adaptive subdivision
  (`light-baking` section 3) refines where the light changes fastest, which is exactly here.
- **Strips on props drawn by the kit** are a second place a console's shape is known (prop space
  numbers in `detailing.json`). Task 3.1 moves them into the prop generator.
