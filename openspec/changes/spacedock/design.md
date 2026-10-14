# Design: spacedock

The owner, 2026-10-14: "are there multiple planets in the system? also can you mockup some versions of spacedock? I want
a big structure where the ship can actually enter rather than something flimsy. I want there to be multiple ships
docket there too".

Numbers marked FN come from `flight-and-navigation`, PM from `patrol-mission`, ES from `engine-stack`. Every size here
is in `data/space/spacedock.json` (section 10); the mockup reads it, never its own copy.

## 1. What every version shares

- **Big and solid.** Each station is 1.7-2.3 km across its longest axis, 20 to 27 times the Tern's 84 m. Its hull is
  closed plate (the exterior pearl and graphite paints), ribbed and banded, with rows of lit windows (the light panel
  layer, a 7 m repeat of 2 x 2 panes), blue accent lights along its edges and door frames, and red and green beacons
  blinking at 1 Hz (`exterior.json` `beacons`). No open girder frame anywhere.
- **An enclosed bay the Tern flies into through doors.** Each has doors of 150-320 m by 90-170 m, sliding leaves of
  armoured plate with hazard-striped leading edges, opening in 20 s (`doors_open_s`) and shut behind the ship. The bay
  inside is walled, roofed and lit by its own floods; the star never reaches it.
- **Berths, not cradles.** A berth (section 7) is a deep steel pier along the docked ship's side, three clamp arms to
  the hull, a glazed crew tube from the airlock across the pier, two portal frames over the ship carrying light strips
  and a crane trolley, flood masts at the corners of a hazard-edged pad. The berths' work lights pool on the hulls.
- **Walls you can see.** Every bay's walls carry tiers of lit control-room windows, steel pilasters and gantries; the
  ceiling carries light rings or strips, so the enclosure reads from anywhere inside it.
- **The sky at the station** (`spacedock.json` `sky`): the star high behind the approach, Tamsin below and beyond at
  PM 2's 17.5 degrees in radius (the station 3.33 radii out), Orrin and Cinder as points of light.
- **The approach**: a line of blue and green approach lights every 250 m for 2 km out from the doors, on the corridor
  the Tern flies in along.

## 2. Version A: Bastion (the mushroom tower)

The reference picture's spirit: a domed cap on a tower, with doors in the cap. Original in its parts: the doors are in
the cap's rim (the ship flies in level, not down through the roof), and the berths stand round a central core.

| Part | Size |
| --- | --- |
| Cap | 1,800 m across; its rim wall 160 m tall; a crown dome rising 250 m above the rim to a control tower and mast; an underside sloping to the tower; 12 ribs over the crown and under the cap |
| Doors | Four in the rim, at 90 degree bearings, each 150 m wide and 90 m tall |
| Tower | A core 240 m across hanging 1,650 m below the cap, eight stacked rings 300-760 m across (a habitat disc 760 m across near its foot), a tapered tip with a green beacon |
| Hall | One round hall 1,720 m across and 140 m high inside the cap; a core column 220 m across in its middle with three tiers of windows; 72 pilasters and four tiers of control-room windows round the wall; 12 radial gantries under the ceiling carrying floods; four light rings across the ceiling |
| Berths | Nine: five on an outer ring (the ship's middle 560 m out) and four on an inner ring (370 m), bows to the core; each pier has a lit walkway back to the hall's wall |

**Entering.** The Tern flies in level through the door at bearing 0 on its approach corridor and on to its outer-ring
berth along the same line, its bow toward the core: 340 m from the rim at the bay's 5 m/s, the last 100 m under the
docking corridor's limits, about 2 minutes from the threshold.

## 3. Version B: Wheel (the habitat wheel and the hub hangar)

| Part | Size |
| --- | --- |
| Wheel | 1,600 m across its middle (1,750 m overall), a section 150 m deep and 110 m wide, window rows on its outer face and both flat faces |
| Spokes | Six, 44 m across, each with a lit band |
| Hub | A drum 540 m across and 580 m long on the wheel's axis; a mast and dish behind |
| Doors | One pair in the hub's front face, 190 m wide and 130 m tall, on the axis |
| Hangar | A cylinder 480 m across and 520 m long round the axis; steel ribs every 40 m, light rings every 80 m; tiers of control-room windows across its back wall and round the doors |
| Berths | Nine in two rows round the curved wall (the ship's middle 168 m from the axis), each ship's keel to the wall, so ships hang overhead and on the walls round the Tern |

**Entering.** Along the axis through the doors, a 180 degree turn in the hangar's middle (ships berth bow out, toward
the doors), then down to the berth: 410 m along the axis, the turn and 168 m down, about 2.5 minutes from the
threshold. The wheel does not turn: the game's gravity is the ships' and the station's own (`ship-frames`).

## 4. Version C: Drydock (the long hall)

| Part | Size |
| --- | --- |
| Hull | An octagonal hall 1,700 m long, 440 m wide and 240 m tall, a deeper rib every 120 m, window rows down both flanks, blue lights down the chamfers |
| Doors | One pair in the bow face, 320 m wide and 150 m tall |
| Stern | A command tower 175 m above the hull with a wider bridge on top, a habitat block 760 m across the stern, radiators on the back |
| Hall | A fairway 400 m wide and 200 m high, 1,650 m long; pilasters every 60 m, four tiers of control-room windows down each wall and five across the stern wall, a gantry across the hall at every berth row, three light strips down the ceiling |
| Berths | Eleven down both walls (every 240 m), ships lying bow out (toward the doors), each pier reaching the wall |

**Entering.** In through the doors along the fairway's centre line, past the berth, a 180 degree turn in the fairway,
then sideways 128 m to the berth: 530 m, the turn and the slide, about 3 minutes. **Leaving** is bow first straight out
of the doors, the quickest of the four.

## 5. Version D: Hollow (the asteroid)

Our own idea: a captured nickel-iron asteroid, hollowed.

| Part | Size |
| --- | --- |
| Rock | An irregular body 1,800 x 1,360 x 2,200 m, faceted (4,500 triangles), its face cut back flat behind the gate |
| Gate | An armoured plate 780 m wide and 560 m tall set into the rock, pearl armour courses over graphite, window rows across it; the doors 300 m wide and 170 m tall |
| Outside works | Two habitat drums on the flanks with window bands, a radiator wing and a mast on top, radiator fins below, lights pricked over the rock |
| Cavern | A rough rock tube 600 m across and 1,330 m deep; a built deck floor 500 m wide; two rows of lit galleries let into each wall; floods hanging from the roof every 130 m |
| Berths | Nine down both sides of the deck |

**Entering.** As C: through the gate, along the deck, a turn and the slide to the berth, about 3 minutes.

## 6. Entering and berthing, on foot

The law (CLAUDE.md): the crew are bodies in the Tern throughout. Nothing here is a cut-scene.

1. **Arrival** 5 km out from the station's hull on its approach corridor (`arrival_from_hull_m`; PM 2 measured it
   from the centre, which for a 2 km station is inside the doors' approach lights).
2. **Clearance.** The captain, on foot at the command chair, requests docking; traffic control (the station, a bot
   voice in chat) clears the Tern to a berth number and the doors start to open when the bow is inside 1 km of them.
3. **The approach.** Helm flies the corridor by hand at up to 40 m/s (`corridor_mps`), slowing to 5 m/s at the doors
   (`door_mps`), or hands it to FN 10's autopilot in Dock mode with the berth as its port. Helm automation (no helm
   player) docks on the captain's order.
4. **Inside.** The bay's 5 m/s (`bay_mps`) holds to 100 m from the berth; FN 10's docking corridor (2 m/s inside 100 m,
   0.5 m/s inside 10 m, 0.2 m/s at contact) takes over. The doors close behind once the stern is 200 m inside.
5. **Berthed.** The clamps close, the crew tube meets the docking collar (deck C, starboard, x -10.8 m, z 12 m) and
   shore power comes on (PM 4.1). Engineering walks to the engine room to bring the reactor down to standby.

**What the crew see on the way in.** From the bridge's windows: the station growing from a lit model to a wall of
plate and windows, the doors parting, the threshold sliding past, then the bay's lit walls and the other ships. The
viewscreens carry the bow camera (`exterior.json` `viewscreen`): the doors ahead, then the core (A), the hangar's far
wall (B), the fairway (C, D). A crew member walking to a deck B or C window sees the berths slide by.

**Leaving** is the reverse: the clamps open (PM 4.2's DRIVE-green rule holds), the Tern backs off its berth on the bow
thrusters, turns where it must and leaves through the doors at the bay's limit. FN 7's jump mass lock of 2 km from a
station is measured from the station's hull (section 12, S4).

## 7. The ships docked there

From the existing models only (no new ship is made for this):

| Ship | Model | Docked as |
| --- | --- | --- |
| Heron-class frigate, sister ship | `exteriors/liveries/cobalt/tern` | The Tern's own class in the cyan livery |
| Heron, rescue livery | `exteriors/liveries/rescue/tern_lod1` | |
| Heron, copper livery | `exteriors/tern_lod2` | |
| Osprey-class courier | `exteriors/osprey` | Once or twice per version |
| Pursuer, Splitjaw, Bulwark | `federation-concepts` (generated from `layouts.json`) | The Federation's concept classes |

The Shrike is a hostile raider and is not docked at a Federation station. Each version has six to eight ships docked
and one or two berths free, lit and waiting. In the engine a docked ship is drawn at its middle level of detail (the
Tern's 1,290 triangles in one draw); only the players' Tern is at full detail.

## 8. The view from the Tern while berthed

The mockup's window view stands 1.6 m in from the bridge's port forward-quarter window (closer than `patrol.html`'s
3.2 m, as a crew member stands to look out), so the window shows a cone from about 15 to 70 degrees off the bow and
19 degrees up and down. What is in it, by version:

| Version | Through the port forward window |
| --- | --- |
| A | The inner-ring ship to port on its pier and light frames, the core's window tiers, the radial gantries and the hall's window tiers |
| B | The hangar's curved wall rising to port, its ribs and light rings, and the next row's ship hanging up and ahead |
| C | The fairway, the ship across it in its frame, the far wall's tiers of windows and the doors |
| D | The deck, the ship across it, the rock wall and its galleries |

The Tern's own warp pylon shows at the bottom of the bridge's windows, as it does in space. Through the starboard
windows: the pier, its rail lights and the crew tube a few metres off.

## 9. The Pi 5 budget

Against ES 5's table (200,000 triangles and 300 draws a frame, all passes; 96 MB of textures). The station's near
model is three draws: the outside lit by the star, the inside and every glowing part unlit with the bay's light baked
into vertex colours, and the doors' leaves (one mesh, each leaf moved by a uniform). Triangles counted from the
mockup's meshes (`docs/mockups/spacedock.html`):

| Version | Outside | Inside (baked) | Glow | Doors | Station draws | Docked ships at LOD 1 |
| --- | ---: | ---: | ---: | ---: | ---: | --- |
| A Bastion | 4,308 | 19,504 | 6,444 | 480 | 3 (the mockup draws 8 leaves apart) | 7 ships, about 10,000 triangles, 7 draws |
| B Wheel | 662 | 12,204 | 4,476 | 120 | 3 | 7 ships, about 10,000 triangles, 7 draws |
| C Drydock | 910 | 16,012 | 3,576 | 120 | 3 | 8 ships, about 11,500 triangles, 8 draws |
| D Hollow | 5,162 | 14,340 | 3,588 | 120 | 3 | 7 ships, about 10,000 triangles, 7 draws |

Station totals: A 30,700, B 17,500, C 20,600, D 23,200 triangles. With the docked ships, the Tern at full detail
(4,440 triangles, 7 draws) and the deck the crew stand in, the worst berthed frame is under 60,000 triangles and 40
draws for every version: under a third of the frame's triangles and an eighth of its draws. A is the dearest (its
cap's ribs and its round hall's fine cells for the bake); B the cheapest.

- **Inside the bay** the outside's mesh is behind the walls: a portal cull (deck-pipeline's) leaves the inside, the
  glow and the docked ships, so a berthed frame draws about half the station.
- **Outside**, beyond 20 km, a far model of under 1,000 triangles in one draw (the outside's silhouette with its window
  rows painted), and beyond 200 km a beacon only.
- **Memory**: no new texture (every surface is a layer of the one array); the station's vertex buffers at 28 bytes a
  vertex are 2-4 MB per version (non-indexed, as baked); the docked ships share their models.
- **Simulation**: the station is fixed in its orbit's frame (PM 2); the doors are a 2-state timer; the docked ships
  are scenery, not simulated bodies. **Network**: the doors' state and the berth assignment, a few bytes on change.
- The mockup's meter adds the planet's sphere (16,000 triangles, standing in for the sky program's one full-screen
  triangle), the Tern at full detail and, in the window view, the bridge: its worst frame is the number to read, not
  the station's.

## 10. Data

`data/space/spacedock.json` (schema `starcrew.spacedock-options/1`): the sky at the station, the traffic limits
(`arrival_from_hull_m`, `corridor_mps`, `door_mps`, `bay_mps`, `doors_open_s`, `doors_close_s`), the colours, the fleet
(which model draws which docked ship) and each version's sizes, doors and berths. When the owner chooses, the chosen
version's block becomes the station's data (`starcrew.spacedock/1`), read by the station generator and checked (doors
wider than the Tern's 45 m over its nacelles by at least 3 times, every berth inside the bay with 30 m clear round the
ship, the approach corridor clear of the station's own parts).

## 11. What changes in patrol-mission once a version is chosen

Recorded here, edited there when the choice is made: PM 3's placeholder starbase and its cradle become the chosen
spacedock; PM 4.1 starts the Tern berthed inside the bay with its doors shut; PM 4.2's "backing out of the cradle"
becomes leaving the berth and the bay (section 6); PM 4.3's departure starts at the doors and the 2 km lock is from
the hull; `exterior.json` `dock` (the girder frame) is retired.

## 12. Risks and trade-offs

- **A big hall's light.** A 1.7 km hall lit only by vertex light needs its floors and walls cut finer than the rest of
  the station (40-45 m cells in the mockup); the bake's cost is offline.
- **Scale in a frame.** At 2 km a station is 20 to 27 Tern lengths: the windows show the bay's near parts at walking
  scale and its far walls as tiers of lights, which is what makes it read as enclosed.
- **B's ships overhead** are striking but every ship in B lies at its own angle to the Tern's floor; the crew see them
  turned. A, C and D keep every ship upright.
- **D's rock** is the cheapest outside per kilometre but the hardest to make read as built: its gate plate and lights
  carry that.

## Open questions

| # | Question | Taken |
| --- | --- | --- |
| S1 | Which version: A Bastion, B Wheel, C Drydock or D Hollow (or a mix: say A's cap with C's hall) | **The owner's choice**, with the mockup and its shots beside it (`docs/screenshots/mockups/spacedock-*.png`) |
| S2 | Is the bay pressurised | No: the bay is sealed and lit, in vacuum; the crew tubes are pressurised. No pump-down wait to leave, and the windows look the same. Recommendation taken (ask only with screenshots) |
| S3 | Can the crew walk onto the station | Not in this change: the crew tube ends at the Tern's airlock door, which opens onto the tube; walking the station is a later change. Recommendation taken (ask only with screenshots) |
| S4 | Where the jump's 2 km station mass lock is measured from | The station's hull, not its centre. Recommendation taken (ask only with screenshots) |
| S5 | Does B's wheel turn | No: the game's gravity is the ships' own. Recommendation taken (ask only with screenshots) |
| S6 | Who flies the Tern in | Helm by hand on the corridor's limits, or the dock autopilot (FN 10) on the captain's order. Recommendation taken (ask only with screenshots) |
