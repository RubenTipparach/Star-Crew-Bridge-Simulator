# Design: the command suite

Status: **proposed** (2026-10-06). Nothing here is built, and the layout does not hold it yet.
The suite is a patch, `data/ships/tern/command_suite.json`, written and checked by
`tools/command_suite.py`, and drawn by `docs/mockups/command-deck.html`. Every number below comes
from that tool, run on the patched copy of the layout it checks, unless the line says otherwise.

## Context

The owner, 2026-10-06, on the bridge variants: "can we do the more circular bridge? but have like
side rooms for meetings, captains quarters and stuff?"

- **The bridge.** Variant B of `bridge-stations` 11a: sixteen sides, 12.4 m across, a ring two
  steps up with consoles built into its walls, the captain's dais at ring height, helm and
  tactical a step up in front. 11a listed what B costs:
  - a third of the wedge's air;
  - side operators 135-157 deg from the screen;
  - helm's and tactical's heads hiding 6.3 % of the screen from the captain;
  - smaller windows;
  - corners between the round wall and the hull that nothing used.
- **Side rooms.** R2 of `docs/analysis/star-trek-bridges.md` is the owner's own picture of the
  idea: "a round main room about 11 m across with a rectangular briefing room attached on one side
  (a conference table, wall displays, the door to the ready room)".
- **Deck A today** (`reference-ship-tern` section 2):
  - the bridge, at z 20.0-32.4;
  - the ready room and the computer core, at z 14-20 either side of the command passage;
  - the passage itself, z 4-20, with the ladder trunk at z 11;
  - the dorsal turret access, at z 0-4.

  Either side of the passage from z 4 to z 14 there is nothing: about 90 m^2 of the hull's
  widest part, at the height of a 3 m room.

## Goals / Non-Goals

**Goals:**
- The round bridge, with its known faults fixed where a fix is cheap.
- Rooms the owner asked for, each with a job in the game:
  - a briefing room for the crew to gather before and during a mission;
  - the captain's ready room and quarters;
  - "and stuff" read as a head and a locker the bridge crew uses.
- Every room checked the way the plan is checked: brushes, hull clearance, portals, reachability,
  and furniture that keeps doors and walkways clear.
- One copy of every rule: B's geometry stays in `tools/bridge_variants.py` and is called with
  parameters, and the patch is applied by one function that the tool and the mockup both follow.

**Non-Goals:**
- Gameplay inside the side rooms beyond what other changes already have. The ready room's private
  console is `bridge-stations`' captain console. The briefing screen shows what the captain's
  console shows. A briefing flow, if wanted, belongs to `netcode-and-sessions`' lobby.
- Applying the patch. That is task 2, a separate commit that also reruns every dependent number
  (section 8).
- New texture layers. The furniture takes the existing materials.

## Decisions

### 1. The bridge: B, with two changes

The room, the ring's depth (2.0 m), its height (+0.45 m), the captain's dais and the
sub-platform are B's, built by `variant_b` in `tools/bridge_variants.py`. That function now takes
parameters, and its defaults reproduce `bridge_variants.json` byte for byte (`--check` passes).
The suite calls it with:

- **`ring_from = -56.25 deg`** (B: -78 deg). The ring starts at the corner between the aft
  diagonal wall and the next one, so the aft diagonal walls are at walkway level and take the side
  doors. Each side keeps four wall banks:
  - comms or flight operations (-45 deg);
  - a status board (-22.5 deg), moved forward one segment from B's door-side place;
  - engineering or science (0 deg);
  - a repeater (+22.5 deg).

  B's two spares give way to the doors.
- **`helm = "pair"`, `helm_x = 1.5 m`** (B: one curved console seating them 0.46 m either side).
  Helm and tactical sit at two free desks (the Blender `free_console_helm` and
  `free_console_tactical`), 1.5 m either side of the centreline, fore and aft where the curved
  console's operators sat. This is the fix 11a gave for B.

Sightlines (`python3 tools/command_suite.py --sightlines`, measured as 11.1's are):

| | B as drawn in 11a | The suite |
| --- | --- | --- |
| Helm and tactical seats | x +/-0.46 m, z 27.90 | x +/-1.50 m, z 27.90 |
| Their heads on the screen's plane, from the captain's eye | x 0.79-1.28 m, top 4.91 m | x 3.11-3.61 m (past the screen's edge at 3.0 m) |
| Screen hidden from the captain | 0.90 m^2, 6.3 % | none |
| Helm and tactical: distance to the screen; width seen; turn to it | 3.64 m; 78.9 deg; 7 deg | 3.91 m; 73.8 deg; 23 deg |
| Captain to the screen; width seen | 6.50 m; 49.5 deg | 6.50 m; 49.5 deg |
| Engineering and science; comms and flight ops: turn to the screen | 135 deg; 157 deg | 135 deg; 157 deg |

Helm's 23 deg turn to the screen's centre is the same as variant A's. The side stations still face their walls, built into them, as the owner answered for `bridge-stations` B1 (2026-10-06: "yup, they have to integrate as part of the wall."); the look band of their consoles (B2, answered the same day) keeps the screen in view.

Unchanged from B: 415.9 m^3 of air and 118.8 m^2 of floor, 0.80 m from the hull at its closest.
The viewscreen is 6.0 x 2.4 m at z 31.5, and the windows are 2.0 x 1.2 m in the two segments
either side of the bow.

### 2. The plan of deck A

```text
                                            bow (+Z)
                          z 31.6    +----[ viewscreen 6.0 m ]----+
                         [window]  /    helm         tactical     \  [window]
                                  /  repeater  sub-platform  repeater \
                        port     |  eng (bank)  +--dais--+  (bank) sci |     stbd
                        (+X)     |   ring       | captain|      ring   |     (-X)
                                  \  status     +--------+     status /
         ready room's corner  ->   \  comms       aisle     flight ops/  <- briefing room's corner
         (shelf, its new door)      [door]                    [door]
  z 20 +-------------------------+------------[ aft door ]------------+-------------------------+
       |  CAPTAIN'S READY ROOM   |                                    |  BRIEFING ROOM           |
       |  desk and console,      |                                    |  table for eight,        |
       |  sofa, window [door]--->|            COMMAND PASSAGE         |<--[door] screen, window  |
  z 14 +------[door]-------------+               (z 4-20)             +--------------------------+
       |  CAPTAIN'S QUARTERS     |                                    |  COMPUTER CORE (moved)   |
       |  bed, desk, en-suite    |<--[door]    ladder trunk z 11  [door]-->| ten racks           |
  z 9  +-------------------------+                                    +--------------------------+
       |  HEAD                   |                                    |  BRIDGE LOCKER           |
       |  2 WC, 2 showers, basins|<--[door]                     [door]-->| EVA suits, kit, bench|
  z 4  +-------------------------+-------[ dorsal turret access ]-----+--------------------------+
                                            stern (-Z)            (not to scale)
```

| Room | POI | Brushes | Air m^3 | Floor m^2 | Hull margin m | Doors | Windows | Furniture |
| --- | ---: | ---: | ---: | ---: | ---: | --- | --- | ---: |
| Bridge | 1 | 1 | 415.9 (was 608.6) | 118.8 (was 173.9) | 0.80 | aft door, ready room, briefing room | 2 | (11 consoles and chairs) |
| Captain's ready room | 2 | 4 | 201.3 (was 143.9) | 67.1 (was 48.0) | 1.06 | bridge, passage, quarters | 1 | 8 pieces |
| Briefing room | 31 | 4 | 201.3 | 67.1 | 1.06 | bridge, passage | 1 | 11 pieces |
| Computer core | 3 | 1 | 131.9 (was 143.9) | 44.0 (was 48.0) | 0.99 | passage | none | 10 racks |
| Captain's quarters | 32 | 1 | 131.9 | 44.0 | 0.99 | passage, ready room | none | 5 pieces |
| Head | 33 | 1 | 136.9 | 45.6 | 0.98 | passage | none | 6 pieces |
| Bridge locker | 34 | 1 | 136.9 | 45.6 | 0.98 | passage | none | 5 pieces |
| Command passage | 4 | 1 | 120.0 | 40.0 | 1.30 | eight doors and the ladder | none | none |

The ship's air goes from 9,824.8 to 10,284.7 m^3 (+4.7 %). Every side room keeps the 0.5 m hull
clearance and stands within 1.6 m of the skin (the spec's "rooms follow the hull"). The patched
copy passes every `layout_check.py` rule.

**The deck's outer wall** is one line. It is the ready room's raked wall, from (9.6, 14.8) to
(8.9, 20.0), extended aft to z 8, where the hull is widest. From there it runs parallel to the skin
to z 4 (`raked_x` in the tool), so the four new rooms line up with the ready room and the old core.

**The corners by the bridge.** The round wall's aft port corners are (1.233, 20.0),
(3.512, 20.944), (5.256, 22.688) and (6.2, 24.967). Seen from the corner room, each of the
bridge's corners is reflex, so no single convex brush can take two of its walls. The ready room's
corner is therefore three prisms, one per bridge wall. Their outer walls carry the deck's line on
to (8.05, 24.967), and they end in a short wall there. Forward of that the gap between the round
wall and the hull is under 1.9 m wide, which is service space (ducts and cable runs, as 11a said).
The briefing room's corner is the mirror image.

### 3. The rooms

**Captain's ready room** (port, POI 2):
- **Use:** the captain's office (R2's "to ready room"), one door off the walkway, 11.8 m and
  7.0 s from the captain's chair to the desk.
- **The desk** stands in front of the window and faces into the room, with the captain's private
  console on it (`bridge-stations` 10.5: the same console as the chair's). Two visitor chairs face it.
- **Other furniture:** a sofa and a low table against the aft wall, a shelf beside the door to
  the quarters, and a shelf at the far end of the corner.
- **Window:** 2.0 x 1.0 m, in the raked outer wall.

**Briefing room** (starboard, POI 31, new):
- **Use:** meetings and mission briefings, for up to the full crew of eight (`bridge-stations`
  section 5: four players are the core, up to eight can play). R2's briefing room, with its own
  door off the walkway: 7.6 m and 4.7 s from the captain's chair to the table's head.
- **The table** runs fore and aft, 4.0 x 1.4 m, with four seats a side.
- **The screen** is a 3.0 x 1.6 m wall screen on the aft wall, at the table's end, so every seat
  looks along the table to it.
- **Other furniture:** a shelf at the corner's end.
- **Window:** 2.0 x 1.0 m, in the outer wall.
- **What the screen shows is not this change's.** The mockup shows the captain's console. A
  briefing feed (the mission map, a hail) is for `bridge-stations` to define if it is wanted.

**Captain's quarters** (port, POI 32, new):
- **Use:** the captain's cabin, the captain's own spawn point if `crew-on-deck` wants one.
- **Furniture:** a bed with its head on the aft wall, a wardrobe, a desk, and an en-suite pod (a
  prefabricated shower and toilet, 1.8 x 1.8 m).
- **Doors:** one to the passage and one through the forward wall into the ready room. The captain
  goes from bed to chair in 16.9 m and 9.8 s through the ready room, or 19.4 m and 11.2 s by the
  passage.

**Head** (port, POI 33, new):
- **Use:** the command deck's washroom, so the bridge crew need not go below.
- **Furniture:** two toilet stalls and a wash counter on the aft wall, two showers and a locker
  bank on the forward wall.

**Bridge locker** (starboard, POI 34, new, working finish):
- **Suits:** the bridge crew's own EVA suits, in a new locker fixture `eva_suits_a`. Helm reaches
  them in 26.5 m and 14.7 s (the locker moved outboard for `deck-access`'s stair tower; it was 25.5 m and 14.2 s). Today the nearest suits are damage control's on deck B, down the
  ladder trunk: 40.1 m and 24.8 s.
- **Other furniture:** a second locker bank (fire and first-aid kits, breathing sets), a
  workbench for spare console boards, and two shelves.
- **What it changes elsewhere:** a second suit locker changes `damage-control`'s suit-up and
  `crew-on-deck`'s routes. That is a proposal to them (section 8), not a rule this change sets.

**Computer core** (starboard, POI 3, moved):
- **Place:** from z 14-20 to z 9-14, beside the briefing room, still one door off the passage and
  two doors from the bridge. The system `computer` moves with it, to (-5.0, 3.5, 11.7).
- **Size:** 131.9 m^3 (was 143.9), which changes its row in `life-support`'s table and
  `deck-pipeline`'s budget.
- **Furniture:** two rows of five racks, back to back, an aisle in front of each.

### 4. Doors and circulation

New and changed portals (all 2.2 m doors unless noted):

| Portal | Between | Where | Size m |
| --- | --- | --- | --- |
| `p_ready_bridge` | bridge, ready room | the bridge's aft port diagonal wall, 0.85 m from its aft end | 1.2 x 2.2 |
| `p_briefing_bridge` | bridge, briefing room | its mirror | 1.2 x 2.2 |
| `p_briefing` | briefing room, passage | x -1.25, z 16.4 (beside the core's old door at z 17.0, so `deck-access`'s lift fits forward of it) | 1.2 x 2.2 |
| `p_core` (moved) | computer core, passage | x -1.25, z 12.5 (was z 17.0) | 1.0 x 2.2 |
| `p_captains_quarters` | quarters, passage | x 1.25, z 12.5 | 1.0 x 2.2 |
| `p_quarters_ready` | quarters, ready room | x 5.6, z 14.0 | 1.0 x 2.2 |
| `p_head` | head, passage | x 1.25, z 6.5 | 1.0 x 2.2 |
| `p_bridge_locker` | bridge locker, passage | x -1.25, z 6.5 | 1.0 x 2.2 |
| `p_ready_window`, `p_briefing_window` | the room, space | the raked outer wall's middle, centre 5.0 m up | 2.0 x 1.0 window |
| `p_bridge_window_p`, `_s` (moved) | bridge, space | B's: the segments either side of the bow | 2.0 x 1.2 window |

**Why the side doors are not centred on their walls.** Centred, the foot of the ring's aft stair
stands in the door's clear zone. The tool's check found that and named the two doors. At 0.85 m
from the wall's aft end, the stair is clear and the door keeps 0.25 m of wall beside its aft jamb.

**Doors off the passage** stay clear of the ladder trunk's opening (z 10.5-11.5): the core and the
quarters at z 12.5, the head and the locker at z 6.5.

**A second way off the bridge.** With the side rooms the bridge has three doors, and the ready
room and briefing room each lead on to the passage. A jammed aft door no longer traps the bridge
crew. A lost passage still cuts the bridge off from the rest of the ship, because all three ways
lead into it. That is `reference-ship-tern` question T3 (a scuttle down to damage control), which
this change does not answer. If T3's scuttle is taken, it moves into the briefing room's corner,
over damage control.

**Furniture keeps doors clear.** Every door has a clear zone on each side, 1.0 m deep and 0.1 m
wider than the door at each jamb. No piece of furniture, and on the bridge no stair, may overlap
one. The check also keeps every piece inside its room and off every other piece (chairs, which
tuck under tables, excepted).

### 5. Furniture

Fifteen props, modelled in Blender with boolean cutters as the bridge props are (the
`blender-hard-surface` skill). They are built by `tools/blender/build_suite_props.py` into
`assets/models/suite`, sharing one module of machinery with the bridge props' build. The chairs
are the bridge set's `crew_chair`.

| Prop | Size m (w x h x d) | Where | Budget, triangles |
| --- | --- | --- | ---: |
| `briefing_table` | 4.0 x 0.75 x 1.4 | briefing room | 400 |
| `wall_screen` | 3.0 x 2.6 x 0.1 | briefing room | 120 |
| `desk` | 1.6 x 1.25 x 0.75 | ready room, quarters | 300 |
| `sofa` | 2.0 x 0.85 x 0.85 | ready room | 200 |
| `low_table` | 1.1 x 0.42 x 0.6 | ready room | 60 |
| `shelf` | 1.2 x 2.0 x 0.4 | ready room, briefing room, locker | 160 |
| `bed` | 1.4 x 1.0 x 2.1 | quarters | 200 |
| `wardrobe` | 1.2 x 2.1 x 0.6 | quarters | 80 |
| `wet_cell` | 1.8 x 2.3 x 1.8 | quarters (en-suite) | 140 |
| `toilet_stall` | 1.0 x 2.0 x 1.5 | head | 200 |
| `wash_counter` | 2.4 x 2.0 x 0.6 | head | 220 |
| `shower_stall` | 1.0 x 2.2 x 1.0 | head | 160 |
| `locker_bank` | 2.4 x 2.1 x 0.55 | head, bridge locker | 200 |
| `server_rack` | 0.8 x 2.1 x 1.1 | computer core | 140 |
| `workbench` | 1.8 x 1.75 x 0.75 | bridge locker | 220 |

**Placement.** The patch's `furnishings` list places each piece as the bridge's props are placed:
`back_m` is the back of the piece at floor level and `yaw_deg` the way its front faces. A chair
carries its sitter's `seat_m`. When the patch is applied, the list becomes the deck's detail file
(`deck-pipeline`: "hand-placed hero detail is a detail file"), not part of the layout.

### 6. The patch and its check

`data/ships/tern/command_suite.json` (schema `starcrew.layout-patch/1`) holds:
- the bridge's platforms, stairs, consoles and seats, as a variant of `bridge_variants.json` does;
- the compartments and portals it adds or replaces, whole;
- the seats it moves, the fixtures it adds or changes (`eva_suits_a`, the viewscreen's centre),
  the fixture it removes (`captain_dais`, which the platforms replace) and the system it moves;
- the furniture;
- the numbers above.

`tools/command_suite.py` writes it. It also:
- applies it to a copy of the layout in memory, with one function (`patch_layout`) that the
  mockup follows field for field;
- runs `tools/layout_check.py` on the copy;
- checks the platforms and consoles inside the bridge, and every piece of furniture and every
  bridge stair against the rules of section 4.

`--check` fails if the file is stale or any check fails. `--sightlines` prints section 1's table.

### 7. The mockup

`docs/mockups/command-deck.html` draws deck A's bridge, side rooms and passage, from the layout
patched with the suite (the same fields, applied the same way as `patch_layout`). The page:
- uses the panels (`wall-panels`, `ceilings-and-trims`, `floor-panels`) and the kit's lamps baked
  in the three lighting states;
- draws the Blender consoles and furniture and the console faces of `bridge-stations` 11.6;
- has the free camera (`lib/freecam.js`).

The whole-ship deck plan (`deck-plan.html`) shows the suite too, by default, applied to its copy of the layout by
the same `ShipKit.applyPatch`. Its Bridge buttons switch between the suite and today's layout, which the page
reads as `#today` in its address, and the furniture there is blocks of each piece's brief size.

Views: a cutaway of the whole suite, the captain's chair, the aft walkway with its three doors,
and inside each room. The prop-placing code it shares with `bridge-variants.html` moves into one
library (`lib/propkit.js`), so the two pages place, face and light props the same way.

### 8. What applying the patch changes elsewhere (task 2)

In the commit that applies it, every number that depends on the plan is rerun with the tool that
made it, and every quote moves with a dated note, as layout v2's did:

- **`layout.json` and `layout_check.py`:**
  - the patch;
  - a `platforms` field the checker learns (11a: "a new layout field the checker learns");
  - the dais fixture retires.
- **`bridge-stations`:**
  - 11.1 rewritten from B with this design's seats and sightlines;
  - 11a closed with the owner's answer;
  - B11 moved to answered;
  - section 12's bridge row.
- **`reference-ship-tern`:**
  - section 2's deck A table: four new rooms, three moved or grown;
  - the deck A map (`docs/design/maps/tern-deck-A.svg`, `tools/deck_plans.py`);
  - section 6's routes;
  - T3's note.
- **`life-support`:**
  - the bridge's 608.6 m^3 falls to 415.9, so its endurance rows shorten by about a third;
  - four new rooms and two new windows (more breach cases);
  - the ship's total.
- **`power-grid`:** the new rooms on node `dp_a` (lamps and the briefing screen).
- **`damage-control`:**
  - fire loads for the new rooms;
  - inert gas stays with the core;
  - the second suit locker (section 3), with its suit-up times;
  - section 8's doors.
- **`crew-on-deck`:**
  - `tools/walk_times.py`'s bridge routes (its detour points are the wedge's);
  - the stairs' climbing time;
  - the suit routes.
- **`deck-pipeline`:**
  - section 11's room budgets (the bridge's row, and new rows);
  - the detail file that takes the furniture.
- **`light-baking`:** the bridge's bake numbers.
- **Mockups:** `bridge.html`, `deck-plan.html`, `lighting.html`, `systems.html`, `exterior.html`
  and `wall-panels.html` read the layout, so they change when it does. Each is re-shot and looked at.

### 9. The Pi 5 budget this change spends

Measured on the mockup (`docs/mockups/command-deck.html`, 2026-10-06), with the panels, the
generated detail, the Blender consoles and furniture, and the crew figures. Each room's mesh
counts its console faces. The ceilings are `deck-pipeline` section 11's: 30,000 triangles for the
bridge and 8,000 for a side room.

| Room | Triangles | Ceiling | Use | Draws |
| --- | ---: | ---: | ---: | ---: |
| Bridge | 9,764 | 30,000 | 33 % | 2 (room, console faces) |
| Captain's ready room | 3,398 | 8,000 | 42 % | 2 |
| Briefing room | 3,696 | 8,000 | 46 % | 2 |
| Captain's quarters | 2,478 | 8,000 | 31 % | 1 |
| Computer core | 3,268 | 8,000 | 41 % | 1 |
| Head | 2,740 | 8,000 | 34 % | 1 |
| Bridge locker | 2,604 | 8,000 | 33 % | 1 |
| Command passage | 3,536 | 8,000 | 44 % | 1 |
| All eight | 31,484 | | | 12 |

- **Draw calls:** one per room, plus one for a room's console faces where it has screens: twelve
  for all eight rooms in the cutaway. From a seat on the bridge, portal culling (`deck-pipeline`)
  draws the bridge and what its open doors show.
- **Textures:** none new. The furniture takes the materials already in the array (`trim`,
  `machinery`, `bulkhead`, `hazard`, `light_panel`), and its upholstery is a tint (`PALETTE.furniture`).
- **Geometry memory:** about 31,500 triangles of non-indexed vertices at about 32 bytes each is
  3.0 MB for the whole suite on the desktop mockup. Indexed and baked by the deck compiler, it
  will be less.
- **Not measured on a Pi.** A cloud session renders on SwiftShader; these are counts, not times.

## Risks / Trade-offs

- **B's costs stay.**
  - The bridge has a third less air, so its endurance on a breach or a dead plant is about a
    third shorter (`life-support` reruns it).
  - The side stations turn 135-157 deg to see the screen (the look band of B2 answers that).
  - The windows are smaller.
- **More doors to manage.** Ten new portals are ten more things for `damage-control` to seal,
  jam and repair. The two windows are two more breach points.
- **The core moves.** Its volume falls 12.0 m^3 and its door moves 4.5 m aft. No rule depends on
  where it is, only on its id and supply, so the systems that use it are unchanged.
- **The passage stays a single point.** All three bridge doors lead into it (T3 stands).
- **Large side rooms.** The head and the locker are about 45 m^2 each, generous for what they
  hold. Smaller rooms would leave hull space empty, which the owner ruled out (T6, "the floor plan
  shouldnt consist of square rooms"). The space is there for later use: an officer's cabin or a
  small armoury.

## Open questions

Per CLAUDE.md 13 a question goes to the owner only with something to look at. The shots are in
`docs/screenshots/mockups/command-deck-*.png`. The rest take the recommendation, recorded here as
"recommendation taken (ask only with screenshots)".

| Id | Question, and the fact it turns on | Options | Recommendation | Shot |
| --- | --- | --- | --- | --- |
| A1 | The side rooms as drawn: the ready room to port with the quarters behind it, the briefing room to starboard with the core behind it, the head and the bridge locker aft. Each side room is 44-67 m^2; the ship gains 459.9 m^3 of air | As drawn / swap port and starboard / give the head's place to an officer's cabin / other | As drawn: the captain's rooms are together on one side, the room the whole crew uses on the other | `command-deck-cutaway.png`, `command-deck-ready-room.png`, `command-deck-briefing-room.png`, `command-deck-quarters.png` |
| A2 | Helm and tactical at two desks 1.5 m apart (heads clear of the screen) or B's one curved console (6.3 % of the screen hidden) | Two desks / one curved console | Two desks. Recommendation taken (ask only with screenshots): 11a's fix, and the captain's view is the bridge's point | `command-deck-captain.png` |
| A3 | The bridge crew's EVA suits on deck A (14.7 s from helm) as well as damage control's (24.8 s) | Both / damage control's only | Both. Recommendation taken (ask only with screenshots); `damage-control` decides its use | none |
| A4 | Windows in the ready room and the briefing room (two more breach points) | Windows / none | Windows: the rooms look out, as R2's do. Recommendation taken (ask only with screenshots) | `command-deck-ready-room.png` |
