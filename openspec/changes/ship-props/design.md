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
- Door leaves as the engine will build them. The kit's walls are drawn back to back with no thickness to slide
  a leaf into, so the deck plan's leaves (section 4e) slide toward their jambs and are clipped there, until the
  deck pipeline draws walls with thickness.
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
  lift shaft, and the start faced the shaft 0.5 m off. A start now faces what the room holds (its props' middle,
  weighted by footprint: the fighter, the shield generator, the briefing table) when that view runs 0.5 m or more on
  three sight lines (ahead and 25 degrees either side); else straight in when that runs 3 m; else the longest of the
  ways in, to the room's middle and along the door's wall;
- **a prop's glowing faces showed the ceiling lamps' lens grid**: the Petrel's canopy and the reactor's window band
  read as lamp panels. They now glow evenly (`propkit`, one lit cell of the lamp layer);
- **life support still showed two plain blocks**: the oxygen generator's and the CO2 scrubbers' points (from the v1
  boxes) stand 3 m and more from the hull room's walls, and the wall rule looked only 2 m out, so the page drew their
  nominal blocks. The rule now looks 4.5 m out, keeps a machine's whole width in the room and 1 cm off a side wall,
  keeps it off every door's opening, and slides it up to 1.5 m along a wall when none takes it where it stands. That
  also moved the forward switchboard off the strip beside its door (it stood within 0.1 m of the opening) to the
  forward wall.

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

**The Petrel is walked into** (owner, 2026-10-08: "need interior for shuttle, and I should be able to walk into it").
Its hull is hollowed by a loft of its own sections inset 8 cm, from the stern wall to the cockpit under the canopy,
with a flat deck 0.6 m above the floor at the ramp's top. The stern opening is 1.32 m wide and 1.95 m clear. Inside:
- a bench of two seats along each side, its back on the hull, with belts;
- a light strip along the ceiling;
- in the cockpit, two seats on pedestals, 0.6 m apart so a body passes between them to the console under the
  canopy; four lamps along the ceiling and over the cockpit light the cabin in the bake.
- the canopy is open: ten panes, five across the roof and upper sides over each of two bands (the cabin's front,
  1.10-2.30 m along the hull, and the cockpit, 2.30-4.05 m), cut right through into the cabin, the hull left
  between them as the frames (section 4f).

It stays one closed solid (`build_machinery_props.py`), 2,732 triangles against 1,918 before (budget 2,900). In the walk it collides
as its own triangles instead of its bounds box (`deck-plan.html` `MESH_PROPS`), so a body walks up the 22 degree
ramp, along the aisle and into the cockpit. Measured headless: the ramp climbed and the deck reached at 0.6 m in
2.3 s from the hangar floor, and out again.

**Crew rooms.** `tools/crew_rooms.py` writes `data/ships/tern/crew_rooms.json` and checks it with
`tools/command_suite.py`'s furniture checks, on the layout as it stands, with the suite, and with deck access:
every piece inside its room, clear of every door's zone (1.0 m), none overlapping, and the scuttle ladders' floor
in damage control and the medbay kept clear.

| Room | Pieces |
| --- | --- |
| Crew quarters | Six two-tier bunks (twelve berths): two on the forward wall, one on the aft wall, one on the corridor wall, and two back to back in the middle of the room; two locker banks on the hull side |
| Mess | A galley on the forward wall, two tables of four |
| Damage control | A workbench (repair kits), two locker banks (EVA suits, extinguishers) |
| Port and starboard turret rooms | Each: the turret's capacitor bank (three battery banks) on the aft wall; its power converter and power panel by the pod's hatch; the pod's air handler and scrubbers on the forward wall; the gunner's locker bank and a tool board on the corridor wall |

Recommendation taken (ask only with screenshots): eight berths, the most crew the netcode allows (M3). The owner,
2026-10-08: "should have more bunks in crew quarters": twelve berths, for eight crew and the four bodies of the damage
control teams, who sleep aboard too. The walls are full (the doors from the stair tower, the turret room and the
spine), so the two new bunks stand back to back in the middle, 2 cm apart, between aisles of 1.6 m and 2.1 m
(`tools/crew_rooms.py` checks them against the doors and the walls).

The turret rooms (owner, 2026-10-08: "turret access rooms are empty, they need to have some kind of battery or life
support stuff there"). Their purposes in the layout name the hatch into the pod and the turret's capacitor bank; the
deck plan drew 78 m^2 each of bare floor. They take what keeps a pod's gun and gunner going, from the machinery and
engineering sets already built, and the middle stays open between the room's three doors. Each room adds 3,576
triangles of props (three battery banks at 450, the converter 448, the panel 304, the air handler 326, the scrubbers
404, the lockers 190, the tool board 554), merged into its one draw.

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

### 4a. No z-fighting

The owner, 2026-10-06, on a deck plan screenshot of a wall with a dark panel flickering through it: "fix z fighting".
CLAUDE.md section 8 says two surfaces never share a plane while overlapping and facing the same way, but the mockups
had no check for it.

**The instrument.** `tools/mockups/zfight.mjs` (new) loads a page headless, takes every triangle its scene draws
(`window.MOCKUP_SCENE`, a hook each mockup now sets), and reports the pairs that lie on one plane (each centre within
5 mm of the other's plane), face the same way and overlap by more than 1 cm^2, summed by the two surfaces. It leaves
out what cannot fight: a faint x-ray overlay (opacity under 0.2), an overlay that does not write depth, and a plan's
own marks overlapping in one colour. It fails above 0.1 m^2 a page.

**What it found on the deck plan: 96 m^2 of surfaces fighting**, nearly all of it one cause:

| Cause | Area | Fix |
| --- | ---: | --- |
| A wall prop's back on the wall plane, which the room behind shares and faces the same way (the owner's screenshot: a locker's back seen through the next room's wall) | 83.8 m^2 | Every prop placed by its back stands 1 cm forward of it (`propkit` `offWall`), and a wall prop's back faces, pressed to the wall, are left out. The wall rule finds the wall plane exactly, not in 2.5 cm steps that put some machines past it |
| The turrets' bases and bodies on the turret pods' ceiling, floor and wall planes | 6.2 m^2 | The turret sits on the pod's skin, a wall's thickness out and 1 cm more |
| The lift car's walls, floor and roof overlapping | 1.3 m^2 | The walls stand on the floor, under the roof, and the back wall between the side walls |
| Skirting and beam end caps pressed into a wall the next room shares; two skirtings overlapping in an inside corner | 0.9 m^2 | No cap where a run ends on a wall; inside corners belong to the walls running athwartships (Undercity's rule 7.2) |
| Beams as wide as the ribs they meet; railing posts as thick as their rails; platform nosings overlapping at corners | 0.5 m^2 | Beams 2 cm narrower; posts 20 % thinner; a nosing starts a nosing's width in where the edge before is nosed |
| The centre ladder drawn by deck C and deck B in the slab between | 0.4 m^2 | The lower ladder's handholds stop where the upper ladder carries on |
| Prop undersides on the floor (the launch cradle's plate and its hazard edge) | 0.4 m^2 | Left out: never seen |
| The plan's wall-top bands 4 mm over the floor where a pod lies under a corridor | 1.0 m^2 | 1 cm |

Left: 0.07 m^2, slivers of a few cm^2 where the bridge ring's short railing segments meet and where skirtings meet at
a slanted corner, one material each. The command deck had the seated crew's shins on the console cabinets' face (now
2 cm short) and is at 0.05 m^2. The other mockups share the kit: `systems.html` drew the turret pods' floors on their
access rooms' floors in its plan (now 2 cm up) and its markers on one plane (now an overlay), and `bridge-variants.html`
built stairs of three steps or more from blocks whose sides overlapped (now one layer a step) and two rooms' wall caps
as two meshes; all are under 0.1 m^2. `bridge.html`, `lighting.html` and `wall-panels.html` show none, `exterior.html` 0.001 m^2.

Before and after, in first person: `docs/screenshots/mockups/zfight-briefing-wall-before-after.png` (a bridge wall
bank's back tearing through the briefing room's wall, the owner's screenshot) and `zfight-quarters-wall-before-after.png`.

**What it changes in the kit's counts.** No end caps in walls, so `kit_report` gives 14,996 detail triangles on today's
layout where it gave 15,944 (`deck-pipeline` section 11's table was made at 15,688 and stays an upper bound). Props
lose their undersides and wall backs (section 5's table is measured after this).

### 4b. Seats (2026-10-07)

The owner, on the captain's chair in the deck plan: "chairs suck still mainly its a texture problem", with four
references: two Next Generation captain's chairs (tan leather; burgundy channelled leather), Elite Force 2's Klingon
bridge and a Bridge Commander bridge. The chairs were boxes in the machinery texture, a grey-brown stone that reads as
nothing anyone would sit on.

**Upholstery is a texture of its own.** Two panel layers baked in Blender (`panel-textures`: designed relief under a
fixed light, not a Material Maker graph, because the padding is modelled), shared by every finish:

| Layer | What it shows | Used on |
| --- | --- | --- |
| `upholstery_channel` (68) | padded vertical channels about 9-10 cm wide with a stitched groove between them, a welted seam every 0.5 m, a fine leather grain | seat and back cushions |
| `upholstery_panel` (69) | smooth padded panels with welted, piped seams | bolsters, headrests, arm pads |

Both are baked in a light neutral grey and tile both ways over 2 m; the page multiplies them by the seat's colour, so one
bake serves every colour: the captain's chair burgundy, after the second reference, the crew's chairs a dark slate with
the station's colour as piping (the `accent` role, as today).

**The chairs are remodelled after the references** (`tools/blender/build_bridge_props.py`, two new roles in the
hard-surface kit, `upholstery` and `upholstery_panel`): the captain's a tall back with the Next Generation chair's centre
slot, a headrest, wing bolsters, a rolled seat front, arms on angled supports carrying their consoles, a louvred pedestal
on a base plate; the crew's the same family, simpler. Their anchors, seat heights and operator points are unchanged, so
every page places them where they stood. Budgets rise for the shape: the captain's from 300 to 900 triangles, the crew's
from 160 to 480 (section 5 is measured after them).

**What it costs.** Two layers, 0.35 MB each at 256 px with mips. At most 600 triangles more for the captain's chair and
320 for each crew chair: the bridge (the captain and six crew) about 2,500 more, inside its 30,000; the briefing room
(eight chairs) about 2,600 more, inside its 8,000. Section 5's table is re-measured with them. No draw call.

### 4c. A custom texture for every prop (2026-10-07)

The owner, on the consoles: "the textures on the console man, they need to be full on custom details, youre using generic
textures on them which looks pretty bad", then: "basically everything using the metal tile grid needs to get replaced with
custom textures". Every prop took the deck's tiling materials by role (the grey tiled `trim`, the `machinery`, the
`bulkhead`), so a helm console, a battery bank and a locker were the same squares.

**An atlas per prop.** Each prop gets its own 256 x 256 layer (`assets/models/<set>/atlas/<prop>.png`), the way game
props have always been textured: the low-poly mesh unwrapped once into a second UV set (glTF `TEXCOORD_1`, the first
staying the metre UVs), and a detailed version of the prop that exists only for the bake (panel seams, bolts and
rivets, vents and louvres, stencilled labels and name plates, button rows, toggle banks, LED strips, gauges, warning
placards, wear) baked onto it in Blender under the panels' fixed light, relief and occlusion in the colour, the glow
in alpha (CLAUDE.md section 9: no normal maps). The low-poly geometry does not change; its triangles are the budget's.
Consoles carry their station's name; no two kinds look alike.

**In the page.** A prop with an atlas takes it on every face but its screens (the console faces are drawn there, as
today) and its `accent` faces, which are baked light so the station's colour still tints them. Chairs carry their
upholstery in the atlas, in their colour. A prop without one keeps the tiling materials.

**What it costs.** One layer a prop kind: 0.35 MB at 256 px with mips, 87 KB at 128 px. The Tern's 47 kinds
(bridge 12, suite 15, machinery 20) come to 16.4 MB at 256 px or 4.1 MB at 128 px of the 96 MB texture budget; a
deck's array need hold only the kinds it places. No draw call and no triangle added.

### 4d. The owner's walk of 2026-10-08

The owner walked the deck plan and found nine things; each is fixed in the page and the libraries, shown in
`docs/screenshots/mockups/walk-fixes-2026-10-08/`, and checked with the walk's own hooks (`MOCKUP_WALK`):

| The owner | Cause | Fix |
| --- | --- | --- |
| "I cant walk up these steps anymore" | The bridge platforms' steps were blocks, not ramps; with the floor bands and nosings added at their feet, the first riser and its 5 cm nosing stood as a 0.24 m lip in front of the ramp, too shallow for the controller to step onto | The platform steps collide as ramps like every stair (`crew-on-deck` 3a); the walk leaves out every step inside a flight's footprint lengthened 8 cm at both ends. Head-on, every approach across the width now climbs |
| "walking is to much like ice skating" | Starting and stopping at the feet's grip, 6 m/s2 | 20 m/s2 to speed up, 30 m/s2 to brake (`crew-on-deck` section 3) |
| "bring back jumping" | C5 had none | A 0.46 m jump on Space (`crew-on-deck` section 3, C5) |
| "elevator interior using crappy texture" | The car took the stand-in `bulkhead` and `deck_plate` | The car's inside wears the crew finish's panels: a light column facing the door, ribbed sides, a plate floor (`ShipKit.buildLiftCar`, `f.panels`) |
| "the whole ship is missing doors!" | No leaves were drawn (above) | Every door and pressure door has its leaves, driven by `shipwalk.js` with `crew-on-deck` section 5's rules: a door opens as a body comes within 3 m and closes 2 s after it leaves; a pressure door opens and closes on E; a closed door is a wall; the lift's landing and car doors open with the car. A leaf retracts into its jamb |
| "the back of this console in the engine room is missing geometry" | The local panel is built as a wall prop, so the page dropped its back wherever it stood, though two stand free | A placement's own `on_wall` decides (`PropKit.placeProp`, `o.wall`), and the panel's back gets its share of the atlas |
| "some more weird missing geometry" (the magazine's ceiling) | A hole between decks showed the half metre between one room's ceiling and the next one's floor | Every floor hole is lined from the ceiling to the floor above (`ShipKit`, the collars) |
| "I cant walk around the reactor all the way" | The reactor collided as its square bounds, whose corners reached into the ring catwalk at the diagonals | Round machines (the reactor, the pressurizer, the dewars) collide as 16-sided prisms; the ring walks round both ways |
| "there should be two impulse thingies connected to these thrusters" | One drive unit on the centreline, between two engines | The layout's impulse drive stands as two units, one on each engine's centreline, nozzles against the aft bulkhead (`units_m`; one system for power and damage, `reference-ship-tern`) |

### 4e. The owner's walk, second round (2026-10-08)

The owner walked it again the same day. Each is fixed in the page, the libraries or the data, shown in
`docs/screenshots/mockups/walk-fixes-2026-10-08b/` and checked headless with `MOCKUP_WALK`:

| The owner | Cause | Fix |
| --- | --- | --- |
| "theres no pipe connecting these modules to the thrusters" | The impulse units stood free of the engines | A duct 1.2 m across from each unit's nozzle into the aft bulkhead, on the engine's axis (`deck-plan.html`, the drive's fit-out) |
| "my collider is too wide, this chair is blocking me from walking" | A chair collided as its bounds, armrests and all, and the body was 0.6 m across | A chair collides as its pedestal (0.25 m round); the body is 0.5 m across (`crew-on-deck` section 2) |
| "the doors are scaled to open ... doors should preserve volume" | A leaf shrank toward its jamb | A leaf keeps its size and slides toward its jamb, clipped there, so it never shows past the frame or over its neighbour; its texture moves with it |
| "elevator is severly broken still", "im stuck in the elevator well" | The car's floor was a moving box flush with the deck, which jammed Rapier's controller at the seam; a shut shaft was guarded only by a soft check that froze every move near it | Each landing door is a wall while shut; inside the shaft the feet are held on the car's floor; a step away from a shut shaft is never stopped (`crew-on-deck` 3a). Every ride between the three decks, and walking off, measured |
| "i cant walk up spiral stairs" | Their collision ramp was one quad a tread from column to wall; one of its triangles stood at 75 degrees across the walk line | The ramp is cut in 0.12 m rings, each at the helix's own slope (`crew-on-deck` 3a); both towers climbed and descended at three radii |
| "whats the deal with this hole right in front of the door?" | The ventral pod's hatch lay between the shield room's and the switchboard's doors, which faced each other across deck C's corridor at z 3.0 | The doors move: the shield room's forward to z 5.0, the switchboard's aft to z 1.2 (`reference-ship-tern` section 4) |
| "engine room textures get cut off here" | Engineering's walls were banded once from its lowest floor, so the mezzanine and the catwalk cut through module rows | A wall's bands start again above every floor inside the room (`wall-panels` section 2) |
| "we need to be able to see some cool plasma ... inside a confined ring" | The reactor's windows were a flat glow | A ray-marched plasma ring behind each window, turning, with helical filaments (`engineering-fitout` 5a) |
| "turret access rooms are empty" | Nothing was placed there | Capacitor banks, a power converter and panel, the pod's air handler and scrubbers, lockers and a tool board (section 4 crew rooms) |
| "did you add armory room somewhere too?" | It was designed (`armory`) but not drawn | An armory on deck A, 56 m^2, with three new props: the rifle rack, the ammunition cabinet and the armour rack (`armory` section 4) |
| "need interior for shuttle, and I should be able to walk into it" | The Petrel was a solid hull | A cabin, cockpit and fittings inside, walked into up the ramp (Craft, above) |
| "this console has some messed up geometry" | The control desk's end cheeks were the convex hulls of a concave profile, so each stood as a slab 0.12 m above the control shelf | Each cheek follows the desk's profile as two convex pieces, a 5 cm rim above the top, the shelf and the deck (`build_engineering_props.py`) |

Recommendation taken (ask only with screenshots): the doors themselves, modelled after the owner's reference
(a stepped seam, raised panels, a black band with a grille, vents and a sign plate), are the next step; this round
keeps the sliding leaves.

### 4f. The owner's walk, third round (2026-10-08)

Shown in `docs/screenshots/mockups/walk-fixes-2026-10-08c/`, before and after:

| The owner | Cause | Fix |
| --- | --- | --- |
| "shuttle window should be like way bigger lol", "what it looks like form outside" | The canopy was five shallow recesses over the cockpit's band only, their floors glowing white from outside and lined with dark glass inside, so the cockpit looked at a wall | Ten panes over two bands, from the cabin's front to the nose, cut through the hull: from inside the pilots see the bay through the frames, from outside the lit cockpit and its seats show (Craft, section 3). 2,732 triangles, 212 more |
| "these pipes arent textured and arent integrated into the wall correctly" | Each duct began inside the impulse unit's nozzle bell and ran on through the aft bulkhead to the engine, one tube with a ring of vertices at each end; both rings lay in shadow (in the bell, and outside the room), so the vertex bake lit the whole tube black, and it showed going into the wall with nothing where it met it | The duct runs from the nozzle's exit to the bulkhead's face, with a ring every 0.4 m for the bake to light, a flange at the nozzle and a collar on the wall (`deck-plan.html` the drive's fit-out; `ShipKit.buildFitout` takes `wall` for an end that enters a wall, as `ceiling` does) |

Not done: the canopy has no glass. A tinted pane would be one more transparent draw per craft, and a later step
decides glass with the exterior view (`ship-frames`).

### 5. The Pi 5 budget this change spends

Measured on the deck plan, 2026-10-06: `MOCKUP_STATS` per compartment, which counts the compartment's mesh and its
screen faces, before the props (the page as it stood after `deck-access`) and after them. "After" is also after
section 4a, whose kit leaves out the caps pressed into walls and the props' hidden undersides and backs. A cloud session renders on
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
| `bunk` | 264 | 300 | 2.10 x 2.00 x 0.95 |
| `mess_table` | 240 | 300 | 2.00 x 0.80 x 1.90 |
| `galley_counter` | 330 | 450 | 2.40 x 2.20 x 0.70 |
| **20 props** | **11,616** | **13,250** | |

**Per compartment.** The ceilings are `engine-stack`'s (the bridge 30,000, every other compartment 8,000). Every
compartment stays inside its ceiling; engineering, with the reactor, the drive and its consoles, is the fullest at
69 %, and the hangar with the Petrel, its cradle and the pumps next at 65 %.

| Compartment | Before | After | Added | Ceiling | Use |
| --- | ---: | ---: | ---: | ---: | ---: |
| Bridge | 2,576 | 6,492 | 3,916 | 30,000 | 22 % |
| Engineering | 2,692 | 5,486 | 2,794 | 8,000 | 69 % |
| Hangar | 2,936 | 5,204 | 2,268 | 8,000 | 65 % |
| Life support | 1,283 | 2,701 | 1,418 | 8,000 | 34 % |
| Briefing room | 1,006 | 2,408 | 1,402 | 8,000 | 30 % |
| Crew quarters | 934 | 2,162 | 1,228 | 8,000 | 27 % |
| Drive section | 928 | 2,130 | 1,202 | 8,000 | 27 % |
| Captain's ready room | 952 | 2,004 | 1,052 | 8,000 | 25 % |
| Computer core | 678 | 1,892 | 1,214 | 8,000 | 24 % |
| Magazine | 1,164 | 1,882 | 718 | 8,000 | 24 % |
| Damage control | 676 | 1,834 | 1,158 | 8,000 | 23 % |
| Torpedo room | 976 | 1,748 | 772 | 8,000 | 22 % |
| Cargo and stores | 1,321 | 1,701 | 380 | 8,000 | 21 % |
| Port launch bay | 666 | 1,692 | 1,026 | 8,000 | 21 % |
| Starboard launch bay | 666 | 1,692 | 1,026 | 8,000 | 21 % |
| Mess | 974 | 1,652 | 678 | 8,000 | 21 % |
| Forward switchboard | 598 | 1,562 | 964 | 8,000 | 20 % |
| Main corridor | 1,546 | 1,490 | -56 | 8,000 | 19 % |
| Head | 585 | 1,455 | 870 | 8,000 | 18 % |
| Bridge locker | 575 | 1,313 | 738 | 8,000 | 16 % |
| Lower corridor | 1,324 | 1,256 | -68 | 8,000 | 16 % |
| Captain's quarters | 634 | 1,202 | 568 | 8,000 | 15 % |
| Medbay | 618 | 1,184 | 566 | 8,000 | 15 % |
| Shield generator | 580 | 1,172 | 592 | 8,000 | 15 % |
| Command passage | 1,036 | 1,000 | -36 | 8,000 | 12 % |
| Port stair tower | 856 | 840 | -16 | 8,000 | 10 % |
| Starboard stair tower | 856 | 840 | -16 | 8,000 | 10 % |
| Aft passage | 822 | 782 | -40 | 8,000 | 10 % |
| Port turret access | 652 | 628 | -24 | 8,000 | 8 % |
| Starboard turret access | 652 | 628 | -24 | 8,000 | 8 % |
| Dorsal turret access | 472 | 568 | 96 | 8,000 | 7 % |
| Ventral turret pod | 234 | 566 | 332 | 8,000 | 7 % |
| Dorsal turret pod | 208 | 400 | 192 | 8,000 | 5 % |
| Lift | 388 | 388 | 0 | 8,000 | 5 % |
| Port turret pod | 184 | 384 | 200 | 8,000 | 5 % |
| Starboard turret pod | 184 | 384 | 200 | 8,000 | 5 % |
| Airlock | 254 | 242 | -12 | 8,000 | 3 % |
| **All 37** | **33,686** | **60,964** | **27,278** | | |

**What else it spends.**

- **Draw calls: none added.** Props are merged into their compartment's static mesh, as the kit's detail is, so a
  compartment stays one draw for its surfaces. Screen faces join the screen pass the compartment already has
  (`deck-pipeline` section 5).
- **Texture memory: none added.** Props take the material array's existing layers (machinery, trim, bulkhead,
  hazard, light panel) and the screens' existing images; `accent` is a tint, not a texture.
- **Vertex memory: about 3.3 MB more, estimated.** 27,278 triangles at three vertices of about 40 bytes (position,
  packed normal, UV, layer and the three lighting states' colours), unindexed, against the 64 MB of vertex and
  index buffers. The whole ship's 60,964 triangles come to about 7.3 MB the same way. Indexing and the baker's
  subdivision both move this; the compiler's count (task 2.2) replaces the estimate.
- **Visible triangles per frame.** A crew member sees one compartment and what its portals show. Counting a
  compartment and every room its portals open on, whole, the most is deck B's main corridor's 18,786 (with the nine
  rooms off it and the corridors above and below, on `layout.json`'s portals; the hangar's is 16,820), under a tenth
  of the 200,000 a frame.
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

- A5 in the survey (`command-suite`'s open questions): the captain's dais rails (section 1), shot three ways
  (`docs/screenshots/mockups/dais-rails-*.png`).
