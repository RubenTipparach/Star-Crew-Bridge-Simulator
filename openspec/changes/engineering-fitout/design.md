# Design: engineering fitted out as a working fusion plant

## Context

Engineering (layout POI 18) is the Tern's tallest room. It spans x -9.6 to 9.6 m and z -18 to
-32 m, and y -3.5 to 6.5 m, through decks C, B and A.
- Its **lower floor** is at y -3.5 with 3.0 m clear under the mezzanine.
- Its **mezzanine** is at y 0 with 6.5 m clear to the ceiling, with a well 3.4 m in radius round
  the reactor.
- A **catwalk** at deck A (y 3.5) meets the aft passage's door on the forward wall.
- The **reactor** is a column 2.2 m in radius and 10 m tall at (0, -25), through all three levels.

Stairs run along the forward wall:
- the upper one from the catwalk down to the mezzanine on the port side;
- the lower one from the mezzanine down to the lower floor on the starboard side, through a hole
  in the mezzanine.

The doors:
- the hangar door on the lower floor;
- the two gallery doors from the hangar on the mezzanine (x +/-7.75);
- the aft passage door onto the catwalk;
- the drive hatch in the aft wall at mezzanine level.

`power-grid` already simulates this room's plant, in `data/ships/tern/power.json`:
- the reactor: deuterium and helium-3, 80 MW thermal, a 4 kg fuel load, a blanket temperature;
- the coolant loop: water and glycol, 740 kg/s at design flow;
- coolant pumps A and B, on the port and starboard switchboards;
- the radiators;
- generators `gen_p` and `gen_s`, and switchboards `msb_p` and `msb_s`;
- five feeders that leave through this room, each with a `path_m` drawn for `damage-control`'s
  hit reach.

What the page drew of it before this change (`docs/screenshots/mockups/engineering-before/`):
- the column;
- two switchboards;
- one desk;
- a 2 m skid holding both coolant pumps, whose pipes are 0.14 m;
- nothing that joins any of them.

**Goals:**
- Draw the plant as the simulation runs it.
- Make engineering read as heavy, hot and hard-worked: machines, plumbing, consoles, tools.
- Route every pipe by rules a script checks.
- Stay inside a Pi 5 budget that is argued.

**Non-goals:**
- New simulation. Machines `power.json` does not simulate are drawn and listed (section 2) for
  `power-grid` to take or leave.
- The drive room, and the other machine rooms (life support, the shield room), which get the same
  treatment later if the owner likes this one.

## Decisions

### 1. The references, and what each gives

The owner's five photos, 2026-10-07. Take the shape, not the art (CLAUDE.md 15).

| Reference | Taken |
| --- | --- |
| An industrial pipe gallery | Big pipes in silver lagging with band clamps, long-radius elbows, flanged joints, hangers; many runs in parallel |
| CERN ALICE | A deep red magnet frame round a cylindrical core packed with cable looms; stairs and catwalks with railings round it |
| CERN ATLAS | Magnets radiating round a central barrel; a steel frame you can see; orange and yellow railings |
| CERN CMS | Dense cable runs and modules on every surface; stencilled labels; yellow access lifts |
| A reactor pool from above | Steam generators: tall vessels with gooseneck pipes off the top; a yellow railed bridge over the glowing core |

The colours that follow: dark working grey for machine bodies, silver for lagging, deep red for
the magnet coils, safety yellow for the crane, guards and rail tops, white for cryogenic jackets,
orange for fuel, and hazard stripes where something moves or is hot. The reactor's glow stays the
blue-white of its window bands.

### 2. How the engine works, and what stands where for it

A deuterium and helium-3 fusion reactor needs, in order: fuel fed in, its plasma held by magnets
that must be kept colder than anything else in the ship, its exhaust pumped out, its heat carried
away, and its power taken off. Each is a flow, and each flow is a set of machines joined by pipe.

| Flow | Machines | Pipe | What `power.json` simulates |
| --- | --- | --- | --- |
| Fuel | 2 deuterium dewars and a helium-3 cylinder rack (store); a fuel processor (meters, freezes pellets); the pellet injector line into the reactor | 0.08-0.1 m, cryogenic white | `reactor.fuel_load_kg`; the dewars, the rack and the processor are not simulated |
| Confinement | A cryoplant (compressor and cold box); the magnet coils on the reactor | 0.15 m, cryogenic white | Not simulated (the reactor auxiliaries' load stands for it) |
| Exhaust | 4 vacuum pumps on the reactor's ports; a ring header; an ash tank | 0.15 m, bare steel | Not simulated |
| Heat, primary | Per loop: the hot leg out of the reactor, a heat exchanger, the cold leg down, a coolant pump, the return to the reactor. A pressurizer on loop B (the loop's pressure buffer) and a drain and makeup tank on loop A | 0.45 m lagged; 0.2 m and 0.15 m painted | `coolant` (one loop, 740 kg/s), `coolant_pump_a`, `coolant_pump_b`; the two loops are one simulated loop, half the flow each, as `flow_per_pump` 0.5 already says |
| Heat, secondary | Each heat exchanger's out and in, up through the ceiling to the hull radiators | 0.3 m lagged | `radiators` |
| Power | 2 converters (the generators) on the aft walls, busbars up to the trays; the switchboards; the feeders' trays | Trays 0.4 x 0.1 m; busbar duct 0.5 x 0.2 m | `generators` `gen_p` and `gen_s`, `nodes` `msb_p` and `msb_s`, `conduits` |
| Upkeep | A bench and tool board, two tool chests, two parts racks, an overhead crane over the aft bay | | `overdrive.wear_pct_per_min_at_150` and the reactor's wear are what upkeep answers; repair is `damage-control`'s |

**Machines not yet simulated** (the dewars, the rack, the processor, the cryoplant, the vacuum
pumps, the ash tank, the pressurizer, the makeup tank) are drawn as the plant's parts with no
readout that claims a number. CLAUDE.md section 7 forbids a screen showing a number the
simulation does not use, so their local panels show only what `power.json` has (fuel kg, loop K,
flow). Whether `power-grid` simulates any of them (a fuel line that can be cut, a cryoplant whose
loss quenches the magnets) is its call. Recommendation taken (ask only with screenshots): not in
this change.

**Two loops on the athwartship axis.** The loops run port (A) and starboard (B) along z = -25, the
reactor's centre line:
- the heat exchangers stand on the mezzanine at x +/-7.0;
- each loop's pump stands under its heat exchanger on the lower floor at x +/-7.2;
- the hot legs leave the reactor at y 2.6, the returns enter it at y -1.15.

The layout's `coolant_pumps` system (one skid at (-6.5, -3.5, -29)) becomes these two pumps. The
patch moves the system's centre to pump B's position, the starboard pump. Pump A is the same
system's second machine, until `reference-ship-tern` T4 gives systems their machines.

**The reactor's azimuths.** Pipes enter the column only at azimuths k x 45 degrees, where the
dressing's eight coils (at 22.5 + k x 45 degrees) leave room:

| Azimuth | What enters | Height (y) |
| --- | --- | --- |
| 0 (bow) | Pellet injector | -1.0 |
| 45, 135, 225, 315 | Vacuum pumps on the coil ports | -2.5 |
| 90 (port), 270 (starboard) | Loop A, loop B: hot leg out at 2.6, return in at -1.15 | |
| 180 (aft) | Cryogenic supply and return | -1.1 |

### 3. The room, level by level

Positions are the machine's origin in ship metres. A wall machine's origin is the centre of its
back. Yaw is the way its front faces, 0 the bow and +90 port. `tools/engineering_fitout.py` is the
one source and these tables present it.

**Lower floor (y -3.5), the plant floor.**

| Machine | Prop | At | Yaw |
| --- | --- | --- | ---: |
| Reactor dressing (coils, rings, ports, looms) | `reactor_dressing` | (0, -3.5, -25) | 0 |
| Vacuum pumps, 4 | `vacuum_pump` | on the ports at r 3.3, azimuths 45, 135, 225, 315 | the azimuth |
| Coolant pump A | `coolant_pump` | (7.2, -3.5, -25) | -90 |
| Coolant pump B | `coolant_pump` | (-7.2, -3.5, -25) | 90 |
| Deuterium dewars, 2 | `fuel_dewar` | (8.3, -3.5, -20.4), (8.25, -3.5, -21.9) | -90 |
| Helium-3 rack | `helium3_rack` | back (6.0, -3.5, -18.0), forward wall | 180 |
| Fuel processor | `fuel_processor` | (5.0, -3.5, -21.4) | -90 |
| Fuel panel | `local_panel` | back (3.4, -3.5, -18.0), forward wall | 180 |
| Ash tank | `ash_tank` | (-5.0, -3.5, -21.0) | 0 |
| Cryoplant | `cryoplant` | (0, -3.5, -30.85), against the aft wall | 0 |
| Cryoplant panel | `local_panel` | back (-2.4, -3.5, -32.0) | 0 |
| Drain and makeup tank | `coolant_tank` | (6.2, -3.5, -28.7) | 0 |
| Coolant valves panel (the layout's `coolant_valves` fixture) | `local_panel` | back (-6.5, -3.5, -27.4) | 0 |
| Bench and tool board | `tool_board` | back (4.4, -3.5, -32.0) | 0 |
| Tool chest | `tool_chest` | (2.3, -3.5, -31.5) | 0 |
| Parts rack | `parts_rack` | back (-4.4, -3.5, -32.0) | 0 |

The forward walkway from the hangar door (x -0.9 to 0.9) and a ring round the reactor from r 4.6
to 5.8 are kept clear to head height.

**Mezzanine (y 0), the operating floor.**

| Machine | Prop | At | Yaw |
| --- | --- | --- | ---: |
| Heat exchanger A | `heat_exchanger` | (7.0, 0, -25) | -90 |
| Heat exchanger B | `heat_exchanger` | (-7.0, 0, -25) | 90 |
| Pressurizer | `pressurizer` | (-7.4, 0, -28.4) | 90 |
| Converters, 2 (the generators) | `power_converter` | backs on the aft chamfers, (+/-7.75, 0, -31.0) | -/+38.66 |
| The engineer's control desk (station `eng_main`) | `control_desk` | at the station's seat, (5.5, 0, -20.5), facing aft | 180 |
| Reactor panel (the layout's `reactor_panel` fixture) | `local_panel` | back (0, 0, -21.4) | 0 |
| Plant mimic wall | `mimic_board` | back (0, 0.6, -18.0), under the catwalk | 180 |
| Parts rack | `parts_rack` | back (3.6, 0, -32.0) | 0 |
| Tool chest | `tool_chest` | (-3.5, 0, -31.4) | 0 |
| Overhead crane | `gantry_crane` | rails' top at (0, 5.6, -29.0) | 0 |

A ring round the well from r 3.45 to 4.6, the forward floor in front of the mimic wall and the
desks, and the way aft to the drive hatch (x -1.3 to 1.3) are kept clear to head height.

**Deck A level (y 3.5), the gantry.**
- The catwalk stays where it is: 4 x 2 m on the forward wall.
- A **bridge** 1.2 m wide runs from it to a **ring catwalk** round the reactor: an octagon of
  apothem 3.35 m, its inner edge 2.35 m from the axis.
- The ring hangs from the ceiling on eight rods.
- It lets a crew reach the column's upper half, and it is the reference pool's yellow bridge over
  the core.

### 4. Pipe runs

A run is data: `{ id, loop, kind, dia_m, from, to, points_m, valves, supports }`.
- `from` and `to` name a machine's port, the reactor's surface at an azimuth, a tee onto another
  run, or a penetration (ceiling or floor).
- `points_m` is the centreline as a polyline.
- The kit sweeps a run:
  - a tube along each leg: eight sides at 0.3 m or more, six below;
  - an elbow of bend radius 1.0 x the diameter at each corner, in four segments for a right
    angle;
  - a flange ring at each end and at every joint 6 m apart;
  - a collar where it passes through a slab;
  - a valve where the data puts one.

The rules, all checked by `tools/engineering_fitout.py`:

| Rule | Check |
| --- | --- |
| A run starts and ends on what it names | The end point lies within 1 cm of the port's flange centre and leaves in the port's direction within 2 degrees; a tee ends on the other run's surface |
| Bends fit | Each leg is long enough for the bends at its ends (2 x bend radius when both ends bend) |
| Inside the room | Every point inside engineering's outline and between its floor and ceiling, clear of the walls by the pipe's radius plus the ribs' depth (0.22 m) |
| Heads clear | Over a walk zone (section 3), a pipe's underside is at least 2.1 m above the floor below it |
| Doors, stairs and catwalks clear | No pipe inside a door's zone (its width plus 0.6 m, 1.2 m deep each side, to its top plus 0.3 m), a stair's volume up to 2.1 m over its treads, or a catwalk's up to 2.1 m |
| Pipes do not touch | Any two runs' centrelines at least the sum of their radii plus 5 cm apart, except where one tees onto the other |
| Machines are not pierced | No leg passes through a machine's bounds except the machine it connects to, within 0.4 m of the port |
| Reactor entries | A run reaches the reactor only at an azimuth of section 2's table and its height |
| Supports | A horizontal leg longer than 3 m gets a hanger to the surface above (the ceiling, or the mezzanine's underside) every 3 m |

**Kinds and finishes.**

| Kind | Used for | Look |
| --- | --- | --- |
| `lagged` | Primary and secondary coolant | Silver aluminium cladding, a band clamp every 1 m; the loop's colour band at each end |
| `painted` | Surge, makeup | Grey paint with a flow arrow |
| `cryo` | Fuel, cryogenic lines | White vacuum jacket, frost at the joints |
| `steel` | Exhaust | Bare steel, heat-tinted near the pumps |
| `tray` | The feeders | A ladder tray of cables, from `power.json` paths |
| `busbar` | Converter to tray | A closed duct |

Until the pipework layer below is baked, every kind takes the platforms layer's `pipe` row (a
conduit side on, with clamps and flanges), which is the generic stand-in. The pipework layer is a
later task:
- one 2 m layer for the working finish, built by `tools/blender/build_wall_panels.py` like the
  trims;
- six rows: lagged, painted, cryo, steel, tray side, tray top;
- u along the pipe, v round it.

**The feeders.** Five `power.json` conduits cross engineering today on diagonals at y 2.4-2.7
(`k_drive_p`, `k_drive_s`, `k_aft_p`, `k_aft_s`, `k_a_aft`). Those paths pass through where the hot
legs now run, and through a person's head on the mezzanine. They move onto trays:
- along the side walls at y 5.0, inboard of the ribs at x +/-8.5, from the switchboards' tops
  aft;
- along the aft wall to the drive hatch, then down to y 2.5 through the wall;
- the hangar feeders forward along the forward wall over the gallery doors at y 2.7;
- `k_a_aft` along the forward wall at y 6.2, as it already continues forward.
The paths outside engineering do not change. The drive feeders grow about 6 m each. Lengths are
not used by the solve: a conduit has a capacity, not a resistance, in `power-grid` section 5.

### 5. Structure: catwalks and railings

- **Railings** follow the platforms' rule (`detailing.json` `railing.height_m` 1.05: posts, a top
  rail, a mid rail and a kickplate), along polylines in the data. They run:
  - round the well;
  - round the ring catwalk's outer edge, but not across the bridge;
  - along both sides of the bridge;
  - along the existing catwalk's open edges;
  - along the lower stair's hole;
  - up the upper stair's open side.
- **The ring catwalk's inner edge** is 0.15 m from the column. A gap that narrow is not a fall, so
  it gets a kickplate only.
- **The mezzanine and catwalk floors** took the generic floor tile, which is what the owner asked
  to have replaced everywhere (2026-10-07: "basically everything using the metal tile grid needs to
  get replaced with custom textures"). They take the floor panels' cells, as a compartment's own
  floor does (`floor-panels`).

### 6. Consoles

The owner asked for consoles. Engineering gets:
- **The control desk** for `eng_main`: three screens, a control shelf, a mimic strip. It replaces
  the bridge set's free desk there.
- **The plant mimic wall** on the forward wall under the catwalk: the loops drawn in light. The
  engineer at the reactor panel turns round to it.
- **Four local panels**: at the reactor (the layout's `reactor_panel`), at the coolant valves (the
  layout's `coolant_valves`), at the fuel processor and at the cryoplant. The two layout fixtures
  were never drawn in the deck plan before.

Every screen shows what `bridge-stations` and `power-grid` give it; nothing more.

### 7. The Pi 5 budget

**The ceiling.** `engine-stack` gives the bridge 30,000 triangles and every other compartment
about 8,000. Engineering is at 69 % of that today with almost nothing in it. This change gives
engineering the bridge's 30,000, as the ship's second showpiece room:
- **The frame.** Engineering has no windows, so standing in it there is no space pass and no
  viewscreen pass. The deck pass is engineering (30,000), what shows through its doors (the
  hangar about 5,200, the aft passage about 800, the drive about 2,100) and four crew (12,000):
  about 50,000 triangles. The bridge's worked frame in `engine-stack` is about 100,000. Planning
  at half the 200,000 ceiling still holds.
- **From elsewhere.** Seen from the hangar through its doors, portal culling draws engineering
  only when a door is open and in view; the hangar's frame is then about 40,000 triangles.
- **Memory.** About 25,000 more triangles merged into engineering's mesh at about 108 bytes each
  (three vertices of position, normal, two UVs, colour and layer) is about 2.7 MB of the 64 MB
  vertex budget.
- **Draw calls.** None added: everything merges into the compartment's one draw.
- **Texture.** Each new prop's atlas is 256 px, about 0.35 MB with mipmaps. 21 props is about
  7.3 MB of the 96 MB texture budget (the panels and props use about 23 MB today). The pipework
  layer adds 0.35 MB.

**Where it goes.** Measured in the deck plan (`window.MOCKUP_STATS`, 2026-10-07): engineering is **27,762
triangles**, 93 % of 30,000. The estimate it replaces:

| Part | Triangles (estimate) |
| --- | ---: |
| Today's shell, kit detail, reactor, switchboards, desk, pumps | 5,486 |
| Removed: the small pump skid and the free desk | -760 |
| Machines (21 props, 34 placed; built at 85-97 % of their budgets) | about 16,000 |
| Pipe runs (24), trays, busbars, hangers, crane runways | about 6,000 |
| Railings, the ring catwalk and the lamps under the mezzanine | about 1,500 |
| Total | about 28,000 |

**The big props' atlases.** The reactor (1024 px), the dressing (1024), the heat exchanger and the crane
(512) and five machinery props carry atlases bigger than the page's 256 px array, so they read at walking
distance (the reactor was 10.8 px per metre and read as blotches). The mockups hold them in two more arrays of
their own sizes (`ShipKit.loadPanels`, `big`): 10 layers, about 26 MB with mipmaps in the browser. On the Pi
that is 26 MB more of the 96 MB texture budget; whether the engine keeps one array at 256 px and tiles the big
atlases, or keeps a second array, is a measurement for `engine-stack`'s probe.

**Found on the way:** every prop atlas had been sampled upside down in the pages. glTF's v runs down from an
image's top and the layer array's runs up from its bottom, and nothing turned it over. The reactor column
showed the black, unused half of its atlas. `propkit.js` now turns v over; the bridge and suite sets look as
their Blender stills do.

### 8. What is measured and checked

- `tools/engineering_fitout.py --check`:
  - the patch is current;
  - the section 4 rules;
  - every machine inside the room, clear of doors, stairs, walk zones and every other machine;
  - the layout check on the patched layout.
- `build_engineering_props.py --check`: byte-reproducible, each prop within budget.
- `kit_report.mjs` and the deck plan's own count: engineering's triangles against 30,000.
- `zfight.mjs`: no fighting surfaces (flanges are proud of their pipes, collars proud of slabs).
- Walk shots of the four engineering views, before and after, and the props' contact sheet.

## Risks / Trade-offs

- **Busy at a distance.** On a Pi at 1280 x 720, much of this detail is a few pixels. The atlas
  carries most of it, and the silhouettes (vessels, coils, pipes) carry the rest. A Pi capture
  decides; until then this is a mockup.
- **Walk collision.** Pipes are part of the room's mesh, so walk mode collides with them. A low
  pipe in the wrong place would be a wall; the head-clearance rule is what prevents that.
- **The two pumps change a layout system's position.** Until `reference-ship-tern` T4 gives
  systems their machines, `damage-control`'s reach for the coolant pumps uses pump B's position
  only.

## Open questions

None for the owner without screenshots. The calls taken here (recommendation taken, ask only with
screenshots):
- loops on the athwartship axis;
- the pumps under their heat exchangers;
- the feeders onto trays;
- the 30,000 ceiling;
- the ring catwalk.
The owner judges the result on the walk shots.
