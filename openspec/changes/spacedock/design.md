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

## 5a. Version E: O'Neill Pair (two hollow, counter-rotating cylinders, a port on the axis)

The owner, later on 2026-10-14: "what about an oneil cylinder space station? can you make some textures for that? a giant
cylinder? wiht a big space station attached at one end. theres usually two of them to counter balance rotational
effects"; "I'd also like you to light it properly as well, texturing is really important to sell scale, we should have 2
layers of texture, detail layer, and the big albiedo color map, and maybe normal maps on the details"; and, on the first
draft (solid tubes with the land painted on the outside, mirrors hinged along the windows): "What oneil cylinders are
hollowed cylinders. Didn't you research what they looked like and how they work?", "Also notice how the solar panels
open up on one side they are blooming slightly like aflower", "The ends are round and should have an opening for ships
to enter". Two reference pictures (a long cylinder with a ring of pods round one end; two cylinders side by side with
orange and white end caps) were taken for spirit only.

**How an O'Neill cylinder works.** Source: Wikipedia, "O'Neill cylinder"
(https://en.wikipedia.org/wiki/O%27Neill_cylinder), which cites Gerard K. O'Neill's *The High Frontier* (1976) and the
NASA Ames summer studies of 1975 and 1977 (CLAUDE.md 15: cited, not recalled).

1. Each cylinder is a hollow pressurised shell; people live on its inside surface, held there by the spin. O'Neill's
   Island Three is 8 km across and 32 km long, turning about 28 times an hour.
2. Six stripes of equal area run its length: three of land (the shell's inside) and three windows, each window many
   small glazed sections in an aluminium or steel frame that carries the pressure.
3. Each cylinder's axis points at the sun. Three large mirrors, one a window, are hinged at the window's far end (away
   from the sun), their free edges toward the sun, so the three splay open like the petals of a flower opening toward
   it; they throw sunlight in through the windows, and moving them moves the sun in the inside sky. Night is the mirrors
   opened wide, so the windows face empty space (which also sheds heat).
4. The two cylinders of a pair turn opposite ways, so their gyroscopic effects cancel and they stay aimed at the sun;
   they are joined at each end by a rod through a bearing system.
5. An agricultural ring round one end turns at its own rate.
6. Inside is enough air for small local weather (clouds); the air and the shell shield against radiation.
7. The ends are hemispherical caps. The sources do not describe docking.

**At our scale.** Island Three is much larger; ours keeps a game scale where the Tern is a speck but a crossing takes
minutes: 6.4 km long and 1.8 km across at 1 rpm (60 s a turn), 1 g at the floor (radius 900 m, w = 0.104 rad/s,
w^2 r = 9.8 m/s^2).

| Part | Size |
| --- | --- |
| Shell | A pressurised shell 40 m thick on the three land stripes, its cut edges seen along every window; the land stripes' outside is pearl plating (the macro's hull strips), never land |
| Windows | Three real openings, each 60 degrees of the circumference and the full length, glazed with many panes in thin graphite frames: from outside, dark faintly blue glass showing the lit land inside and letting a little of its light out; from inside, nearly clear glass onto black space, the mirror beyond it and the star's glare where the mirror throws it in |
| Inside | Three concave valleys of hedged fields, woods, rivers and lakes (smooth deep blue-green water with a darker shore, a soft sheen of the windows' light, no detail layer on it), roads and small towns (blocks of grey and terracotta roofs on a street grid, gardens and trees between), stopping at a lit retaining wall at the foot of each dome; 70 cumulus a cylinder, 90-180 m across and 150-260 m off the floor, over the land stripes only, standing where the Material Maker cloud map is dense (three blobs each with a flat base, white tops toward the axis where the windows' light comes from, grey bellies, hazed with distance); a thin light spine down the zero-g axis, stopping short of the hall |
| Ends | Domed caps 500 m deep (a spherical cap; hemispherical in O'Neill's design), pearl with an orange rim outside. Inside, engineered structure, never land (see "Why the domes carry no land" below) (owner, 2026-10-14: "the end cap should be like greebles here, cant plant stuff on the cap"): a greeble macro map (`oneill_dome`: plated sectors, radial ribs and rings, vent banks, hatches, rows of lit windows) over the hull detail layer, and on it 24 radial ribs, pipes down every other meridian, five rings, 70 platforms and boxes, about 900 work lights that twinkle (so the dome reads from the far end, 6 km off), one plated mesh and one point set a dome |
| Axes | On the star: the station frame's sun is along +z for E (`spacedock.json` `versions.E.sun_dir`); A-D keep `sky.sun_dir` |
| Mirrors | Three a cylinder, 900 m wide and 6.1 km long, faceted, hinged at the far (anti-sunward) end of each window and opened 35 degrees toward the star: a three-petal flower round each cylinder. Reflective: dark at most angles, Tamsin in them, a hot spot and bloom where they turn the star to the eye, each facet tilting its reflection a little. A flat mirror at 35 degrees throws the star back into its own window, so from outside its blaze is only seen from inside the beam; late in the day, opened to 68 degrees, a petal throws the star clear past its own dome, and the mockup's outside view stands in that beam, 23 km off (`late_deg`). Night: opened to 80 degrees |
| Pair | 9.5 km apart centre to centre (the petals reach 4.4 km from each axis, so the two flowers clear each other as they turn), turning opposite ways |
| Farm ring | 18 pods on a ring 2.6 km across round each cylinder's sunward end, on three spokes from the land stripes |
| Joins | Non-rotating rods across the pair at both ends into bearings on the domes' axes; tethers between them |
| Port and bay | Our own design (the sources do not describe docking), guided by fact 7's zero-g axis. A non-rotating hub on cylinder 0's axis at its sunward dome, 600 m across, **a pressure boundary**: the dome's hole takes exactly the hub's outer radius, the hub passes through both shells of the dome, and a rotating seal collar on a bearing closes each shell to the hub, flush all the way round (owner, 2026-10-14: "cant have space leaking out, that opening on the cylinder MUST CONFORM TO STATION OPENING"). Inside the hub, one round cross-section 540 m across: a lit throat 320 m deep (rings every 40 m, eight light strips) from the mouth to an iris of eight leaves hinged at the rim, folding back into the bay as it opens, then the round bay, 620 m long, sealed from the habitat (its back wall and the hub's back close it). Six ships and the Tern berth round the bay's wall, keel to the wall and up toward the axis, in two rows of four, one berth free. The only ways in from space are the iris and airlocked crew tubes. The other three domes' holes take their bearings' radius the same way |

**Why the domes carry no land.** The owner, 2026-10-14: "caps are still wrong, you need to fix that, gravity doesnt
WORK on the caps!". Spin gravity points straight out from the axis, perpendicular to it. On a land stripe that is
straight down into the floor; on an end dome it runs along the dome's surface, out toward its rim: "down" slides down
the dome to the valley floor. Nothing can be planted on a dome or stand on it, and water would run off it, so a dome is a
wall: structure only (ribs, rings, plated sectors, pipes, hatches, docking and service galleries, lights), its texture
the `oneill_dome` greeble map, never the land map. The valleys end at its foot against a lit retaining wall. (The first
rebuild terraced the domes into steps of land; that was wrong for the same reason, and is gone.)

**Why the sunward end.** The petals are hinged at the anti-sunward end and open toward the star; by the time they reach
the sunward end they stand 4.4 km out from the axis, and they end 1.9 km before the sunward dome's apex, so the axis
on the sunward side is clear: an approach down the axis passes through the farm ring's middle and meets no mirror. The
sunward dome is also lit, so the port reads from far off. (The anti-sunward axis is clear too, but there the mirrors'
hinges crowd the dome and the dome is in shadow.)

**Entering** (`traffic` as A-D): cleared by traffic control, the Tern flies down the axis from the star's side, through
the farm ring and the rings of light on the dome, into the round mouth, down the lit throat and through the iris, into
the round bay, then out from the axis to its berth on the wall, about 2 minutes from the mouth. The hub and its bay do
not turn; the dome turns round them, once a minute, on the seals.

**The seal, checked in geometry.** The mockup's `SPACEDOCK_SEAL_CHECK()` casts 1,600 rays from points inside both
habitats toward their domes (300 of every 400 aimed at the hub or bearing, the rest anywhere on the dome), every surface
tested from both sides. Result at the commit that added it, and again after the bay polish: 0 of 1,600 reach space. As a control, with the hub and the
bay taken away, 96 of 1,600 do, so the check sees a leak when there is one.

### 5b. E's two layers of texture and its light

Made in Material Maker through the repository's pipeline (rendered on the Pi 5 itself: Godot 4.7 arm64, Material Maker
`174f15e` with the `material-maker-headless` patches, lavapipe), graphs written in `tools/materials/build_ptex.py`,
packaged by `tools/materials/oneill_atlases.py` into `assets/textures/oneill/`, and shown on
`docs/screenshots/materials/oneill-contact-sheet.png`:

| Texture | Size | Covers | Bytes with mips |
| --- | ---: | --- | ---: |
| `oneill_macro` (albedo, alpha the panes) | 1024 x 1024 | Once round a cylinder's outside: hull strips, glazing frames and panes, bands, km-scale grime | 5,592,404 |
| `oneill_land` (alpha its water) | 512 x 2048 | Once over a land stripe inside: hedged fields, woods, rivers, lakes with their shores, roads, towns of roofs and trees | 5,592,412 |
| `oneill_land_detail` | 256 x 256 | 64 m of land, tiled: a soft mottle and faint crop rows, kept off the water | 349,524 |
| `oneill_clouds` (alpha the cover) | 512 x 512 | The cloud cover round a cylinder: where the cumulus stand | 1,398,100 |
| `oneill_dome` (alpha its lit windows) | 512 x 512 | Once over a dome's inside: greeble structure | 1,398,100 |
| `oneill_cap` | 512 x 512 | Once over a dome | 1,398,100 |
| `oneill_detail` | 512 x 512 | 16 m of hull, tiled: uneven plates, bolt rows, louvred service plates, scratches | 1,398,100 |
| `oneill_detail_normal` | 512 x 512 | Its normal map | 1,398,100 |
| `oneill_mirror` | 256 x 256 | 120 m of mirror facets, tiled | 349,524 |
| `oneill_mirror_normal` | 256 x 256 | Its normal map | 349,524 |
| **Total** | | | **19,223,888 (18.3 MiB)** |

- **Two layers, outside and in.** Every macro map (the outside wrapped once round the cylinder, the land once over a
  stripe, the clouds once round their layer, a dome once over it) carries what is kilometres wide; a tiled detail layer
  multiplies it (near white, divided by its own mean so it adds structure without changing the colour) and fades to that
  mean with distance (350 m to 2.6 km outside, 250 m to 1.8 km on the land), so a close view shows plates or crop rows
  and a far one the macro map with no tiling. The hall's exterior takes its paint as the macro and the hull detail at 24
  m.
- **Normal maps on the detail**, from the same Material Maker graphs. In the mockup E's hull, domes, mirrors, farm ring
  and hall exterior take a per-pixel shader: the detail's normal in the surface's tangent frame, the star's diffuse light
  and a sheen on the paint so bevels and bolts catch the star.
- **The light, with the axes on the star.** The sunward domes are lit full on; the cylinders' sides only at a graze (the
  normal maps catch it), the rest in shadow but for Tamsin's earthshine (the station sees Tamsin's day side, the star
  behind it), and the windows let a little of the lit land's glow out, so the sides read as lived in. The mirrors are
  mirrors: dark at most angles, Tamsin in them, the star's hot spot and bloom where they turn it to the eye. Inside, the land is
  lit by the star the mirrors throw in through the windows, from across the axis, in daylight (and dark at night, the
  mirrors wide); air haze inside lifts the far valley and the far dome; from inside, the windows look out onto black space
  and the mirrors, with the star's glare along the beam the mirror throws in. From outside, through the dark panes (a
  glint of the star on them), the far land shows bright and concave under its clouds. Running lights down every window edge and on the farm pods shrink and crowd together with distance; beyond 6
  km a faint blue haze lifts the far end outside. Inside the hall, its floods and work lights as in A-D.
- **No mutual shadow.** The first draft tested each fragment's ray to the star against the other cylinder. With both
  axes on the star and the cylinders side by side, neither can shade the other, so the test is dropped.

**A proposed exception to "no per-pixel lighting on the Pi 5"** (`surface-materials`, ES 5, CLAUDE.md 9), for the
exterior of stations only, and only once measured. The cost, estimated:

| Cost | E |
| --- | --- |
| Texture memory | 18.3 MiB (outside 9.0, land, clouds and domes 8.6, mirror 0.7): about 19% of ES 5's 96 MB; the macro and the land at half size would save 8 MiB, ETC2 a further three quarters (both measured first) |
| Fragment work | Outside: three texture reads (macro, detail, normal), a tangent-frame normal, a diffuse and a specular term, about 50 ALU instructions a pixel. Inside: two reads and a few terms. Glazing and clouds: one read each, blended. The deck program is one texture read and a vertex-colour multiply |
| Overdraw | Through a window: the glazing blended over the land and the cumulus behind, two layers where a window covers the screen |
| Pixels | At most a cylinder's share of a 1280 x 720 frame, about 0.9 million pixels; none while berthed (the hall is the baked path) |

The recommendation: measure the shaders on the Pi (the `engine-stack` probe, the outside and the inside views at 1280 x
720) before the engine takes them; if they do not fit, E falls back to the rule as it stands: each detail's relief baked
into its albedo by `postprocess.py`'s key light, no normal map, the star's light per vertex. Nothing in the engine
changes in this change.

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
| E | The round bay's wall curving overhead, its ribs and light rings, a ship berthed on the wall a quarter turn round, the back wall's tiers of windows |

The Tern's own warp pylon shows at the bottom of the bridge's windows, as it does in space. Through the starboard
windows: the pier, its rail lights and the crew tube a few metres off.

## 8a. The bay's baked light

The owner, 2026-10-15: "we should have some baked lighting in the interior hanger bay". The patrol begins here, in
Tamsin Station's round bay (E), so the bay is the first place the crew look out at. Section 9 already puts the
inside on the baked path ("the inside and every glowing part unlit with the bay's light baked into vertex colours");
this section says what is baked and how, by `light-baking`'s rules (LB below), so the bay is lit the way the Tern's
decks are and nothing new is invented for it.

- **Fixtures, as data** (LB 11). The bay's lights are fixtures in `data/lighting/fixtures.json`'s format, placed by
  rule from the version's sizes in `data/space/spacedock.json` (the layout holds no lights). E's round bay:

  | Fixture | Kind (LB 11) | Where | Light |
  | --- | --- | --- | --- |
  | `bay_ring` | area, a strip round the bay's wall | Every 80 m down the bay's 620 m, on the wall's ribs | Cool white, the bay's general light |
  | `bay_flood` | point, beam down to the wall | On the eight gantries under the axis, aimed at the berth rows | Warm white floods: the pools the ships lie in |
  | `berth_work` | point, wide beam | Four at each berth's pad corners (the flood masts) and two on each portal frame | Bright, short range: the pier, the clamps and the hull under them |
  | `window_tier` | area, emissive | The back wall's tiers of control-room windows and the hub's | Soft, warm: lit offices; it lights the back wall a little |
  | `throat_ring`, `throat_strip` | area | The throat's rings every 40 m and its eight strips (5a) | The run of light the Tern leaves down |

  The approach and door beacons blink and turn: they are runtime lights (LB 7) and emissive faces, never baked.

- **The bake** (LB 1-3): the same irradiance function as the decks (direct light with soft shadows from the gantries,
  the piers, the ships' berths and the iris leaves, occlusion, one bounce), into the inside's vertex colours with
  adaptive subdivision: the bay's walls and back wall cut to about 40 m cells (the risk in section 12), finer, about
  4 m, on each berth's pad and pier so a work light's pool lands where the clamps are. Offline, in the station's build;
  nothing is computed at run time but the blend.
- **Three states from one set of rays** (LB 5): **normal**; **alert** (the station's own red alert, raised by traffic
  control when hostiles are reported in the system: the rings dim, red strips come up, the floods stay for work);
  **emergency** (the station on its emergency supply: floods and rings out, the berths' work lights and the windows'
  emergency share only). One weight per station, sent with the doors' state (a byte on change), as a deck's
  compartments are blended.
- **The ships in the bay are lit by the bay, not by the star** (LB 6). The bay is sealed: no sunlight reaches in.
  Ambient-cube probes are baked through the bay's air every 20 m (finer, 5 m, round each berth), and every ship in it
  is lit from them: the docked ships, and the **Tern's own hull**. This corrects what `patrol-mission` built: its hull
  relight (PM 11.3) takes the star's direction while the Tern is berthed, so the hull outside the windows is lit as if
  in open space. Inside the bay the hull takes the probes at the Tern's place; from the iris to the mouth the probes'
  light gives way to the star's over the throat's 320 m, so leaving the station, the light changes as the Tern comes
  out into the sun.
- **What the crew see** (section 8, E): the bay's wall overhead in the rings' light, the ships across the bay in their
  floods' pools, the pier and the clamps bright under the work lights through the starboard windows, the back wall's
  lit window tiers; in the alert state the red strips; through the throat on the way out, the rings sliding past.
- **The mockup first.** `docs/mockups/spacedock.html` bakes E's bay with the mockup baker (`docs/mockups/lib/
  lightbake.js`, the one the deck plan uses, by the `light-baking` skill), in all three states, with the probes lighting
  the Tern and the docked ships, and shows its cost; its shots go into `docs/screenshots/mockups/` for the owner to see
  before the station generator bakes it.
- **Cost** (section 9): no draw and no texture added; three colour sets on the inside's vertices (4 bytes a set a
  vertex, the bay's cells adding about 20,000 vertices, about 0.25 MB with the sets) and the probes (54 bytes each, LB
  6: a few thousand, under 0.3 MB), client only; at run time one blend a vertex and a probe lookup a ship.

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
| E O'Neill Pair: hub and round bay | about 600 (the hub's drum, mouth, back and seal collar) | about 15,500 (with eight gantries) | about 3,300 (flood rings, the hub's lights, the ships' running lights) | 8 (eight iris leaves) | 3 (the mockup draws the leaves apart) | 6 ships, about 8,700 triangles, 6 draws |
| E O'Neill Pair: the throat | 128 (its wall) and 9,952 (its light rings and strips, the dome's rings) | | | | 21 (the mockup draws each ring and strip apart; built, 2) | |
| E O'Neill Pair: the two cylinders | about 96,000 (48,000 a cylinder: shell, land, glazing, two domes outside and in, the inner domes' greebles, about 3,500 a dome, the seal collars, 70 cumulus of 320 triangles, 22,400, spine, three mirrors, farm ring) and the rods' 60 | | | | 30 (15 a cylinder) and the rods' 1 | |

Station totals: A 30,700, B 17,500, C 20,600, D 23,200, E about 123,000 triangles (E's are 64 draws: a cylinder is 15,
its outside, land, glazing, domes, greebles, cumulus, spine and mirrors each their own material; a built E would merge
the opaque ones by material, about 7 a cylinder, and the port's rings into one). The cumulus are the biggest share
(44,800 for the pair): fewer or simpler puffs, or puffs drawn only in the cylinder the camera is in or looking into, are
the first cut. The mockup's busiest E view, outside with both cylinders, the planet and the docked ships, is about
131,000 triangles and 92 draws, against 200,000 and 300; inside a cylinder about 86,000 and 76. With the docked ships, the Tern at full detail
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

## 11a. The stations of the Shoals

The owner, 2026-10-14 (S1): "I want to keep all of them as various stations you can visit across the system, starting at
oniel pairs". All five versions are stations of the Shoals system (`data/space/systems/shoals.json`: the star Shoal;
Cinder, rock; Tamsin, terran; Orrin, a gas giant, with its moon Pell; the Shoals belt with the relay). Each orbits at
3.33 of its body's radii, outside the jump's 3-radius mass lock, as Tamsin Station does (patrol-mission design 2). The
list is `spacedock.json` `stations`. Recommendation taken (ask only with screenshots):

| Station | Version | Where | Role |
| --- | --- | --- | --- |
| Tamsin Station | E O'Neill Pair | Tamsin, 2.0e7 m | The home port: the system's people, farms and government; the patrol starts docked here |
| Cinder Yards | C Drydock | Cinder, 8.0e6 m | The shipyard: builds and refits the fleet in its long drydock, fed by Cinder's metal mines |
| Orrin Highport | A Bastion | Orrin, 1.93e8 m | The refinery and fleet command: skims Orrin's gas for fuel, the patrols' forward base |
| Pell Wheel | B Wheel | Pell, 4.0e6 m | The water and ice depot: mines Pell's ice for water and air, rests crews in its spinning wheel |
| Shoals Hollow | D Hollow | In the belt, 47 km from the Shoals relay | The frontier post: prospectors' market, the patrol's last port before the deep belt |

`data/space/systems/shoals.json` is read strictly (`sc-core::system`, `deny_unknown_fields`) and today holds one
station, `tamsin_station`, of kind `starbase` with its berth and departure legs. Naming each station's version there is
an engine schema change (a new `kind` per version, or a `spacedock` field), so the list lives in `spacedock.json` for now
and moving it into the system file is the engine step (tasks 2.1). The patrol-mission change follows: its starbase
becomes E.

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
| S1 | Which version: A Bastion, B Wheel, C Drydock, D Hollow or E O'Neill Pair | **Decided by the owner**, 2026-10-14: "I want to keep all of them as various stations you can visit across the system, starting at oniel pairs". All five are stations of the Shoals (section 11a); the patrol starts at E, Tamsin Station |
| S10 | Where each station sits, its name and its role | Section 11a. Recommendation taken (ask only with screenshots) |
| S2 | Is the bay pressurised | No: the bay is sealed and lit, in vacuum; the crew tubes are pressurised. No pump-down wait to leave, and the windows look the same. Recommendation taken (ask only with screenshots) |
| S3 | Can the crew walk onto the station | Not in this change: the crew tube ends at the Tern's airlock door, which opens onto the tube; walking the station is a later change. Recommendation taken (ask only with screenshots) |
| S4 | Where the jump's 2 km station mass lock is measured from | The station's hull, not its centre. Recommendation taken (ask only with screenshots) |
| S5 | Does B's wheel turn | No: the game's gravity is the ships' own. Recommendation taken (ask only with screenshots) |
| S6 | Who flies the Tern in | Helm by hand on the corridor's limits, or the dock autopilot (FN 10) on the captain's order. Recommendation taken (ask only with screenshots) |
| S7 | Per-pixel lighting (normal maps) on station exteriors on the Pi 5 | Proposed as an exception (section 5b) and measured on the Pi before the engine takes it; until then the rule stands and E falls back to relief baked into the detail's albedo. Recommendation taken (ask only with screenshots) |
| S8 | E's cylinders: their size and spin | 6.4 km by 1.8 km at 1 rpm (1 g at the floor), smaller than Island Three, the pair 9.5 km apart so the petals clear; textures are atlases, not layers of the array, so no layer is renumbered. Recommendation taken (ask only with screenshots) |
| S9 | E's port: which end, and does every cylinder have one | Cylinder 0's sunward dome only (section 5a, why the sunward end); the other cylinder's axis keeps its bearing. Recommendation taken (ask only with screenshots) |
| S11 | How the bay is lit (owner, 2026-10-15: "we should have some baked lighting in the interior hanger bay") | Baked, by `light-baking`'s rules: the bay's rings, floods, berth work lights and window tiers as fixtures, three states, probes lighting every ship in the bay including the Tern's hull (section 8a). Recommendation taken (ask only with screenshots) |
