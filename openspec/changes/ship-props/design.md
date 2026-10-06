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

Filled in from the build and the deck plan's own counts (section 5 table, after the build).

## Risks / Trade-offs

- **More triangles in machine rooms.** Engineering and the hangar carry the largest props; both stay inside
  `deck-pipeline` section 11's ceilings (section 5).
- **Nominal sizes.** Until T4 the machines are the deck plan's guesses at size. When T4 sets sizes, the props are
  rebuilt to them.
- **A rule, not a layout.** The station and system rules place things where a designer might not. Each placement
  is visible in the deck plan and the tour, and a rule is easier to correct than forty hand-placed pieces.

## Open questions

None with something to look at yet; the after tour is the review material.
