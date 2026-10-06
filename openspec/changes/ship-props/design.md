# Design: ship props

## Context

The deck plan (`docs/mockups/deck-plan.html`) draws the whole Tern, and since 2026-10-06 it can be walked in
first person (`docs/mockups/lib/shipwalk.js`). Walking it shows every placeholder the first designs left: a
system was a grey box sized by the deck plan's nominal table, a station a block and a disc, a craft a few boxes,
and the suite's furniture a block of its brief's size. The command deck page already draws the Blender consoles,
chairs and furniture (`bridge` and `suite` prop sets, placed by `lib/propkit.js`); the deck plan did not use them.

## Goals / Non-Goals

**Goals**
- Every system, craft, station and crew room in the deck plan drawn as a modelled prop, as good as the
  bridge consoles, each within a triangle budget the build enforces.
- One rule for placing a station's console and chair, one for placing a system's machine, and the crew
  rooms' furniture as checked data, so nothing is hand-placed in a page (CLAUDE.md 8, 11).
- Fix what the tour found in the kit: the lamp lenses, the hatch ladders, the lift car's materials.

**Non-Goals**
- System sizes in the layout (`reference-ship-tern` T4): the props are built to the deck plan's nominal sizes
  until T4 sets them.
- Door leaves. The kit's walls are drawn back to back with no thickness to slide a leaf into; a door is an
  open frame until the deck pipeline draws walls with thickness.
- The exterior: hull mounts, sensors and the radiators stay as they are.

## Decisions

### 1. The tour

`MOCKUP_WALK.enterRoom(id)` stands a body 1.2 m inside a compartment's first door, facing in, where a body can
stand; a script shoots one frame in every compartment (37). The before tour is
`docs/screenshots/mockups/walk-tour-before/`, the after tour `walk-tour-after/`. Found, besides the placeholders:
- the lift stood behind the briefing room's door from the bridge (`deck-access` design section 3, fixed there);
- a walk starting on top of a chair or inside a machine (the body now starts only where it can stand:
  `ShipWalk` `canStand`, a ray from 2.9 m and a capsule test);
- the turret pods' gunners standing above the ceiling, because the walk started on the seat block.

The after tour, with every set placed, found and fixed:
- **a walk into a launch bay began inside the fighter.** The bay's door from the hangar is a pressure door, which
  the start rule did not count as a door, so it fell back to the bay's middle, where the Swift stands. A walk now
  starts by a door or a pressure door, never one to space;
- **a walk could start face to a wall.** The briefing room's door from the bridge opens on a 1.0 m strip beside the
  lift shaft, and the start faced the shaft 0.5 m off. A start now faces the room's middle when the view that way
  runs 1.2 m or more on three sight lines (ahead and 25 degrees either side), else the longest of the ways in;
- **a prop's glowing faces showed the ceiling lamps' lens grid**: the Petrel's canopy and the reactor's window band
  read as lamp panels. They now glow evenly (`propkit`, one lit cell of the lamp layer).

And left for the owner, with its screenshot in the survey: the captain's dais is railed on six of its eight edges
in `bridge_variants.py`'s B, where `bridge-stations` says "railed behind"; seated, the captain looks out through
the bars.

### 2. The machinery set

`tools/blender/build_machinery_props.py`, with the hard-surface kit (`tools/blender/hs_kit.py`) and the
`blender-hard-surface` skill's method: block out, carve with named cutters, union, chamfer the edges that catch
light, clean, triangulate, check, and export byte-reproducibly with a manifest. Sizes are the deck plan's nominal
system sizes (its `SYS_NOMINAL`) or the craft's layout sizes; the build refuses a prop over its budget. The table
of props and their triangles is section 5's.

Anchors, as the suite's: a wall prop's origin is on the floor at the centre of its back, on the wall plane; a free
prop's on the floor at the centre of its footprint; a craft's on the floor at the centre of its footprint, nose to
+Z.

### 3. Placing them

**Stations.** One rule, in the deck plan's `stationItems`:
- A bridge station of the command suite takes the suite's own props and seats (`command_suite.json` bridge).
- The captain's seat elsewhere takes the captain's chair.
- A gunner takes a crew chair at the seat and a stand-up console in front, in the pod. Bay control, on the
  hangar's landing, takes a stand-up console.
- Any other station: facing a wall within 1.7 m, a wall bank on that wall (an engineer's the core bank with its
  breakers), its chair at the bank's operator distance; in the open, a free-standing desk with the chair at the
  seat. The station's own variant (`props.json` `variant_of`) where the set has one.

**Systems.** Each system kind maps to a prop, or a system's own id where one kind holds several machines (life
support's five). A wall prop stands with its back on the nearest wall, of the four along the axes, facing the
room; a free prop stands at the system's point; a missile tube's breech faces aft, where its crew loads it; the
medbay's beds are a pair; the reactor stands from its base on deck C. The computer is the suite's server racks.

**Craft.** At their layout centre, on their bay's floor, nose to the bow.

**Crew rooms.** `tools/crew_rooms.py` writes `data/ships/tern/crew_rooms.json` and checks it with
`tools/command_suite.py`'s furniture checks, on the layout as it stands, with the suite, and with deck access:
every piece inside its room, clear of every door's zone (1.0 m), none overlapping, and the scuttle ladders' floor
in damage control and the medbay kept clear.

| Room | Pieces |
| --- | --- |
| Crew quarters | Four two-tier bunks (eight berths), two locker banks on the hull side |
| Mess | A galley on the forward wall, two tables of four |
| Damage control | A workbench (repair kits), two locker banks (EVA suits, extinguishers) |

Recommendation taken (ask only with screenshots): eight berths, the most crew the netcode allows (M3).

### 4. The kit

- **Lamp lenses.** A lens is 2:1 (`detailing.json` `panel_m` and `corridor_panel_m`); the `light_panel` layer's
  2 m span holds two panels across and two down. The lens takes the layer's top half exactly, so it shows two
  whole panels, centred, at any lamp's size (`Builder.triUvm`, texture coordinates given, not world-projected).
- **Hatch ladders.** The kit drew ladders only up through ladder wells; a scuttle or a turret pod's hatch had
  none. It now draws one up through every floor hatch, from the lower floor to the floor above it (read from the
  room above, so a pod with no slab between gets no extra half metre), with its handholds above.
- **The lift car.** Its parts name their materials (deck plate floor, bulkhead walls, a trim rail) rather than
  taking the shaft's finish, whose risers are machinery.
- **Glowing strips.** A prop's `light_panel` parts glow, as the rooms' lamps and status strips do.
- **Space outside.** The command deck's starfield on the viewscreen and in the windows moves into `propkit`
  (`spaceViews`), and the deck plan uses it.

### 5. The Pi 5 budget this change spends

Measured on the deck plan, 2026-10-06: `MOCKUP_STATS` per compartment, which counts the compartment's mesh and its
screen faces, before the props (the page as it stood after `deck-access`) and after them. A cloud session renders on
lavapipe; these are triangle counts, not frame times, and say nothing about the Pi's speed (CLAUDE.md section 2).

**The machinery set**, as built (`assets/models/machinery/props.json`; the build refuses a prop over its budget):

| Prop | Triangles | Budget | Size W x H x D, m |
| --- | ---: | ---: | --- |
| `switchboard` | 598 | 600 | 2.40 x 2.00 x 0.76 |
| `battery_bank` | 450 | 500 | 2.00 x 1.60 x 1.19 |
| `coolant_pumps` | 576 | 600 | 1.96 x 1.41 x 1.57 |
| `impulse_drive` | 882 | 900 | 3.00 x 2.70 x 4.00 |
| `inertial_dampers` | 412 | 500 | 1.40 x 1.60 x 1.40 |
| `shield_generator` | 636 | 700 | 2.00 x 2.20 x 2.00 |
| `ls_tanks` | 470 | 500 | 1.80 x 2.02 x 1.56 |
| `ls_scrubbers` | 404 | 500 | 1.80 x 2.00 x 1.58 |
| `ls_air_handler` | 326 | 500 | 1.80 x 2.00 x 1.55 |
| `gravity_generator` | 472 | 500 | 1.60 x 1.40 x 1.60 |
| `med_bed` | 236 | 400 | 1.00 x 1.20 x 2.24 |
| `magazine_rack` | 768 | 900 | 5.10 x 1.60 x 2.80 |
| `missile_tube` | 420 | 500 | 0.90 x 2.00 x 3.08 |
| `reactor_core` | 1,068 | 1,200 | 4.40 x 10.00 x 4.40 |
| `launch_cradle` | 260 | 400 | 3.20 x 0.50 x 6.40 |
| `swift_fighter` | 886 | 1,000 | 4.59 x 1.80 x 7.00 |
| `petrel_shuttle` | 1,918 | 2,000 | 4.52 x 3.20 x 10.00 |
| `bunk` | 262 | 300 | 2.10 x 2.00 x 0.95 |
| `mess_table` | 240 | 300 | 2.00 x 0.80 x 1.90 |
| `galley_counter` | 330 | 450 | 2.40 x 2.20 x 0.70 |
| **20 props** | **11,614** | **13,250** | |

**Per compartment.** The ceilings are `engine-stack`'s (the bridge 30,000, every other compartment 8,000). Every
compartment stays inside its ceiling; engineering, with the reactor, the drive and its consoles, is the fullest at
69 %, and the hangar with the Petrel, its cradle and the pumps next at 67 %.

| Compartment | Before | After | Added | Ceiling | Use |
| --- | ---: | ---: | ---: | ---: | ---: |
| Bridge | 2,576 | 6,856 | 4,280 | 30,000 | 23 % |
| Engineering | 2,692 | 5,528 | 2,836 | 8,000 | 69 % |
| Hangar | 2,936 | 5,394 | 2,458 | 8,000 | 67 % |
| Briefing room | 1,006 | 2,496 | 1,490 | 8,000 | 31 % |
| Crew quarters | 934 | 2,242 | 1,308 | 8,000 | 28 % |
| Drive section | 928 | 2,184 | 1,256 | 8,000 | 27 % |
| Captain's ready room | 952 | 2,078 | 1,126 | 8,000 | 26 % |
| Life support | 1,283 | 1,943 | 660 | 8,000 | 24 % |
| Computer core | 678 | 1,940 | 1,262 | 8,000 | 24 % |
| Damage control | 676 | 1,920 | 1,244 | 8,000 | 24 % |
| Magazine | 1,164 | 1,914 | 750 | 8,000 | 24 % |
| Cargo and stores | 1,321 | 1,835 | 514 | 8,000 | 23 % |
| Torpedo room | 976 | 1,776 | 800 | 8,000 | 22 % |
| Mess | 974 | 1,756 | 782 | 8,000 | 22 % |
| Port launch bay | 666 | 1,742 | 1,076 | 8,000 | 22 % |
| Starboard launch bay | 666 | 1,742 | 1,076 | 8,000 | 22 % |
| Forward switchboard | 598 | 1,576 | 978 | 8,000 | 20 % |
| Main corridor | 1,546 | 1,546 | 0 | 8,000 | 19 % |
| Head | 585 | 1,531 | 946 | 8,000 | 19 % |
| Bridge locker | 575 | 1,359 | 784 | 8,000 | 17 % |
| Lower corridor | 1,324 | 1,324 | 0 | 8,000 | 17 % |
| Captain's quarters | 634 | 1,252 | 618 | 8,000 | 16 % |
| Medbay | 618 | 1,208 | 590 | 8,000 | 15 % |
| Shield generator | 580 | 1,198 | 618 | 8,000 | 15 % |
| Command passage | 1,036 | 1,036 | 0 | 8,000 | 13 % |
| Port stair tower | 856 | 856 | 0 | 8,000 | 11 % |
| Starboard stair tower | 856 | 856 | 0 | 8,000 | 11 % |
| Aft passage | 822 | 822 | 0 | 8,000 | 10 % |
| Port turret access | 652 | 652 | 0 | 8,000 | 8 % |
| Starboard turret access | 652 | 652 | 0 | 8,000 | 8 % |
| Dorsal turret access | 472 | 596 | 124 | 8,000 | 7 % |
| Ventral turret pod | 234 | 580 | 346 | 8,000 | 7 % |
| Dorsal turret pod | 208 | 422 | 214 | 8,000 | 5 % |
| Lift | 388 | 404 | 16 | 8,000 | 5 % |
| Port turret pod | 184 | 398 | 214 | 8,000 | 5 % |
| Starboard turret pod | 184 | 398 | 214 | 8,000 | 5 % |
| Airlock | 254 | 254 | 0 | 8,000 | 3 % |
| **All 37** | **33,686** | **62,266** | **28,580** | | |

**What else it spends.**

- **Draw calls: none added.** Props are merged into their compartment's static mesh, as the kit's detail is, so a
  compartment stays one draw for its surfaces. Screen faces join the screen pass the compartment already has
  (`deck-pipeline` section 5).
- **Texture memory: none added.** Props take the material array's existing layers (machinery, trim, bulkhead,
  hazard, light panel) and the screens' existing images; `accent` is a tint, not a texture.
- **Vertex memory: about 3.4 MB more, estimated.** 28,580 triangles at three vertices of about 40 bytes (position,
  packed normal, UV, layer and the three lighting states' colours), unindexed, against the 64 MB of vertex and
  index buffers. The whole ship's 62,266 triangles come to about 7.5 MB the same way. Indexing and the baker's
  subdivision both move this; the compiler's count (task 2.2) replaces the estimate.
- **Visible triangles per frame.** A crew member sees one compartment and what its portals show. Counting a
  compartment and every room its portals open on, whole, the most is the hangar's 17,276 (with both spines, both
  launch bays and engineering, on `layout.json`'s portals), under a tenth of the 200,000 a frame.
- **Install size.** The machinery glbs are 0.79 MB; the build bakes them into the deck files, so they are source,
  not shipped as they are.

## Risks / Trade-offs

- **More triangles in machine rooms.** Engineering and the hangar carry the largest props; both stay inside
  `deck-pipeline` section 11's ceilings (section 5).
- **Nominal sizes.** Until T4 the machines are the deck plan's guesses at size. When T4 sets sizes, the props are
  rebuilt to them.
- **A rule, not a layout.** The station and system rules place things where a designer might not. Each placement
  is visible in the deck plan and the tour, and a rule is easier to correct than forty hand-placed pieces.

## Open questions

- Y2 in the survey: the captain's dais rails (section 1), with the after tour's bridge shots.
