# Design: helm-nav-map

The owner, 2026-10-16: "Does the helmsman have a navigation map?" "Like where to warp to next". And the same day:

> "Here are some rules to help with nav: 1. Each star system has a unique background 2. Each star system will have at
> least 1 station 3. Each star system will consist of several rocky worlds, at least 1 gas giant, and multiple
> asteroids"

Presented in `docs/mockups/helm-nav-map.html`, shots `docs/screenshots/mockups/helm-nav-map-*.png`. Nothing here is
built. Numbers marked FN come from `flight-and-navigation`, PM from `patrol-mission`, BS from `bridge-stations`, UA
from `ui-art-direction`.

## 1. What exists

- **Helm's console** (BS 9, approved `consoles.html` helm, built in `console/helm.rs`): the title band (HELM,
  OBJECTIVES, the swap chips, the captain's order chip, LEAVE STATION, the condition), the look band (the viewscreen,
  the camera pad, the attitude ball), four panels on the 12 by 4 grid (THRUST 3 columns, SCANNER 5, ATTITUDE and ORIENT
  4 by 2 each), the status strip. SCANNER is the local plot of FN 9: 1 to 50 km, the mission's waypoint, the COURSE
  autopilot.
- **The jump** (FN 7, built by PM 5 in `sc-core::combat::patrol`, numbers in `data/ships/tern/jump.json`): 800 MJ at 20
  MW (40 s); the bow within 5 degrees of the jump vector and turning under 2 deg/s for the spool's last 3 s; mass lock
  within 3 radii of a body's centre, 5 km of a hostile ship over 500 t, or a station's own `mass_lock_m`; arrival at the
  place's arrival point scattered within 1.5 km; a 60 s cooldown. `Command::Jump(true)` (helm, held 0.6 s) spools toward
  "the patrol's next place", the route's next entry; `Patrol(Jump)` is the captain's standing order. Refusals
  `MassLocked` and `NotReady` exist.
- **The patrol's console controls** (PM 7, `patrol-consoles.html`, awaiting approval): helm's UNDOCK and JUMP with the
  ALIGN glyph (the bow line in the 5 degree cone), the captain's UNDOCK and JUMP orders.
- **The system file** (`data/space/systems/shoals.json`, `sc-core::system`): the star, four bodies (no kind), five
  stations, two points of interest, the sky `bloom`. `SystemFile::place` and `mass_lock` are the one rule for places and
  locks.
- **The system map** (FN 9) is designed and never mocked up: "a top-down view of the system on its reference plane with
  a logarithmic zoom from 1 km to 100 AU: bodies with their orbits, points of interest (stations, beacons, asteroid
  fields, wrecks, mission markers), sensor contacts (from science), the ship's velocity vector and its
  straight-line-plus-gravity path for the next 60 s, mass lock circles, the jump target and spool state, waypoints and
  the course", for helm and captain.
- **The route** (PM 1, 5, 12c; phase 3 designed, not built): six stops from Tamsin Station (Cinder orbit, scan;
  Cinder's ore lanes, battle; Orrin's rings, civilian; the Pell beacon, battle; the Shoals' edge, scan; the Shoals
  relay, battle), then home. PM's P9: "the route is fixed in the mission; a choice on the system map comes with FN 9's
  map". This change is that map.

## 2. What shipped games do (research)

Read at the sources on 2026-10-16. Where a page could not be opened (the FTL and Elite fan wikis returned errors) it is
cited only through what could be read.

| Game | How the destination is chosen, and by whom | What we take | Source |
| --- | --- | --- | --- |
| Star Trek: Bridge Crew | The captain holds "a warp map displaying far-flung locations"; in the campaign "only mission-relevant locations can be accessed". Warping is two stages: a reviewer's early "Engage!" orders did nothing until the drive was charged and the ship aligned. "Warping to a new location needs the Captain to issue the command, the Engineer to route power to the warp coils, and then for the Helm to plot the course, point the ship in the right direction and engage." Helm aligns "impulse and warp vectors". | The split: helm plots and aligns, engineering powers, the captain's order is the go. Mission places first. Two stages that are seen, so nobody shouts "engage" at a cold drive. | https://www.roadtovr.com/star-trek-bridge-crew-review/, https://msn.gamereactor.eu/star-trek-bridge-crew-review/, https://steamcommunity.com/app/527100/discussions/0/2650805212061373366 |
| FTL: Faster Than Light | A sector of beacons; JUMP is pressed once "the FTL drive is ready" and opens the map to "choose another reachable beacon"; a jump is "a route decision". The exit beacon is always marked. | One tap on a marked place chooses; the next goal is always visible; the map shows only what can be reached. | https://pixeltwelve.com/guides/ftl-first-run-pause-power-targeting-repairs, https://ftl.fandom.com/wiki/Sectors (search summary only: the page refused the fetch) |
| Elite Dangerous | Galaxy map: search a system and "plot route"; the route is checked against the ship's jump range and goes invalid when the ship changes. System map: bodies and stations to target. Mass lock near stations and planets stops the frame shift drive charging, at distances by object type. | A plotted target is checked against the ship's state, and the map says when it is not reachable. Mass lock is drawn, not explained. | https://forum.quartertothree.com/t/elite-dangerous-kickstarter-launched/70900?page=130, https://steamcommunity.com/app/359320/discussions/0/2264691117423933337, https://elite-dangerous.fandom.com/wiki/Mass_Lock (search summary only) |
| Artemis Spaceship Bridge Simulator | Helm's jump drive: "you must first set a course by adjusting the distance slider (green) and direction slider (blue). Once a course has been plotted, click Initiate", then "Confirm" or "Cancel"; "The duration of the countdown depends on power the Engineer has allocated". Helm "must be prepared to ask for the Engineer's cooperation". | A plot, then a guarded go; spool time is engineering's power, shown to helm. Sliders for a destination are what we avoid: a map tap is faster. | Artemis manual 1.70 rev 5, pages 25-29, https://shared.fastly.steamstatic.com/store_item_assets/steam/apps/247350/manuals/Artemis_Manual_1.70-rev5.pdf |
| FreeSpace 2 | Inside a system, subspace "hops" can be made "anywhere in that system", "nearly instantaneous"; mission nav points are marked visited when the player comes within 1,000 m. | Anywhere in the system may be a destination (a free point); stops change state when reached. | https://wiki.hard-light.net/index.php/Subspace, https://wiki.hard-light.net/index.php/Tutorial_-_Autopilot_missions |

Taken together: **mission places first and always visible** (Bridge Crew, FTL), **any point as an option** (FreeSpace,
Elite), **plot then go, two visible stages** (Bridge Crew, Artemis), **checks drawn on the map** (Elite's mass lock and
invalid routes), **who plots is helm, who says go is the captain** (Bridge Crew).

## 3. A mode of the helm console, not a new screen

Three places were weighed:

| Option | For | Against |
| --- | --- | --- |
| **A NAV tab on the helm's own chip** (chosen) | The approved pattern: the captain's console already has COMMAND and SHIP on its chip, and OBJECTIVES already opens over the four panels with the look band kept. The map gets 8 by 4 grid cells (824 x 400 px at 1280 x 720), enough for a system 4 AU across with its clusters apart. The jump is a moment between fights, when THRUST and ORIENT are not in use. | Helm leaves the stick panels while plotting. Mitigation: flying is never panel-bound (the keys and the pad fly on every tab, BS 6), ALIGN is on the NAV tab, one tap or Tab returns to HELM. |
| A mode switch on SCANNER (LOCAL / SYSTEM) | No tab; THRUST, ATTITUDE and ORIENT stay. | SCANNER's body is 512 x 340 px: at the whole system, Orrin's six places sit inside 2 px; the JUMP controls would have to squeeze into a panel that is already the busiest. The local plot is what helm needs in a fight, and a mode that hides it is a trap. |
| A free-floating screen | The most room. | Not in the approved grid, a second layout to approve and keep at parity, and against BS 10 (four panels, fixed). |

The tab is drawn exactly as the captain's: `[HELM][NAV]` as one own chip pair, the station's role colour on the
selected one, then OBJECTIVES. The order chip (CAPTAIN: JUMP) and LEAVE STATION keep their places. Tab cycles HELM and
NAV with the keyboard; the pad's View button does the same.

## 4. The layout

On the 12 by 4 grid (`grid(c, r, w, h)`, BS 8):

| Panel | Grid | Holds |
| --- | --- | --- |
| MAP | 0, 0, 8 by 4 | The map (section 5), its zoom chips (SYSTEM and the cluster zoomed to) at its top left, the scale bar at its foot. |
| TARGET | 8, 0, 4 by 2 | The target's kind icon and name, its state (CLEAR, MASS LOCK, PLOTTED), the main button PLOT (while a ghost is shown) and NEXT STOP (back to the route's preselection). |
| JUMP | 8, 2, 4 by 2 | The ALIGN cue (the bow and the 5 degree cone, with the off angle in degrees: helm acts on it), the ALIGN autopilot button, and the existing JUMP (held 0.6 s) with the spool ring, its time, the cooldown or MASS LOCK. |

Words at rest (CLAUDE.md 10, 30 at most, names aside): MAP, TARGET, JUMP, SYSTEM, PLOT, NEXT STOP, ALIGN, JUMP and one
state word: 10. The map's names are place names (Shoal, Tamsin, Cinder, Orrin, the Shoals) and appear only at the zoom
where they do not collide (section 5.4). Distances are not shown: a jump costs the same 40 s whatever the distance (FN
7), so a distance is a number no one acts on; it is in the tooltip.

## 5. The map

### 5.1 Frame and zoom

- Top down on the system's plane (x right, z down the screen, +y toward the viewer, the file's frame), north fixed: the
  map never turns with the ship, so a place is always where it was (Elite's system map, FTL's sector).
- **Zoom** is logarithmic from 1 km to 100 AU across the map's width (FN 9), in steps of 1.5 times (the wheel, a pinch,
  the pad's triggers). Two chips jump straight to a frame: SYSTEM (every body and the belts fitted) and the cluster
  helm last tapped (a body with its stations and points fitted, for example CINDER). The scale bar shows one round
  length (1 AU, 10,000 km, 50 km).
- **Clusters.** A body owns the stations that orbit it and the points within its influence (its 3-radius lock times
  ten, or the parent's for a moon). When a cluster is under 24 px across at the current zoom it draws as its body's
  glyph alone, with its stops beside it (5.3); a tap on it zooms to the cluster. This is what keeps Orrin, Pell, the
  beacon, the rings, Orrin Highport and Pell Wheel (all within 2e8 m) from heaping at the system zoom.

### 5.2 What is drawn, by kind

| Thing | Glyph (colour always with a shape) | Drawn when |
| --- | --- | --- |
| Star | A filled disc with a soft halo, the star's colour | Always |
| Rocky world (`kind` rocky) | A filled disc, its `look.land_srgb`, at least 6 px | Always |
| Gas giant (`gas_giant`) | A larger disc (at least 10 px) with two bands across it and, when it has `rings`, an ellipse through it | Always |
| Moon (a body with a `parent`) | Its kind's glyph at 0.7 times, its orbit round the parent | Its cluster is open |
| Asteroid (`asteroid`) | A small irregular polygon (five points, seeded by its id), 4 px | Always (rule 3 makes them landmarks) |
| Belt (`belts`) | Two dotted arcs at its inner and outer radius, dots every 6 px, the band between them faintly filled | Always, clipped to the map |
| Orbit | A thin `line2` circle round the parent, 48 segments | Always for planets; moons in an open cluster |
| Station | A small square with its version's mark, in `text` | Its cluster is open |
| Point of interest | A diamond outline | Its cluster is open |
| Route stop (5.3) | A circle with the kind's icon | Always |
| Mass lock | A hatched ring at the lock's radius: `faint` while the ship is outside it, `danger` hatching while the ship or a picked point is inside | When its radius is at least 6 px |
| The ship | The ship glyph (consoles' `shipGlyph`), pointing along its heading in the map's plane, with its own lock state as a ring (hatched `danger`: locked) | Always, at least 18 px |
| Contacts | The scanner's glyphs in their IFF colours, from science's sweep | When the zoom shows 200 km or less |
| Velocity | A short tick from the ship along its velocity, 60 s long (FN 9) | When the zoom shows 5,000 km or less |
| The route | A dim dashed polyline through the stops in order and home | Always |
| The plotted course | A solid accent line from the ship to the target's arrival point; `warn` while spooling | A target is plotted |
| The ghost | A dashed `text` line at 50 % to the place under the cursor or last tapped, with a ring on it | While choosing |

### 5.3 The route's stops

Each stop of the mission's route (PM 12c: `route` entries with `kind` scan, battle or civilian) is a 24 px circle
beside its place (or its cluster at a far zoom), stacked in route order where several share a cluster:

| Kind | Icon (consoles' set) | State |
| --- | --- | --- |
| scan | `sensors` | Open: `dim` outline. Next (preselected): accent outline, double ring. Done: filled `ok` with a tick. Left undone (PM 5.2, "moving on uncleared"): `warn` outline with a gap |
| battle | `target` | The same; a battle stop whose group the sweep has found adds the hostile colour to its outline |
| civilian | `person` | The same |
| home | `bays` | Accent once every stop is done or skipped |

### 5.4 Labels

Names in Label size (UA 4) beside a glyph, placed by a fixed order of eight offsets (right, left, above, below and the
diagonals) and dropped when none is free; a cluster's name shows at the system zoom, its places' names when it is
open. The star is named. The selected or plotted target is always named, in the TARGET panel.

## 6. Choosing the next jump

### 6.1 What may be a target

| Pick | Arrival (system frame) | Notes |
| --- | --- | --- |
| A route stop, a station, a point of interest | `SystemFile::place(id).arrival` (authored at least 5 km from anything) | The usual pick |
| A body | Its parking point: 4 radii from its centre toward the star (the lit side; outside the 3 radius lock with a radius to spare) | `system::parking_point`, one rule |
| A free point | The point tapped, on the system's plane at the ship's height | Allowed when clear (6.3); FreeSpace's "anywhere in that system" |

### 6.2 Preselection

After every arrival, after undocking and at the start, the server sets the target to **the route's first stop not yet
done** in route order (home once every stop is done or left). Helm sees it plotted already, with nothing to press: a
new player, a bot or automation that never opens NAV jumps the route exactly as today. NEXT STOP on the TARGET panel
puts the preselection back after helm plotted something else.

### 6.3 The check (one rule, both ends)

`system::target_clear(arrival, jump)` is `None` when the arrival is at least `arrival_scatter_m` (1.5 km) outside every
lock (`planet_radii` 3 times each body's radius, each station's `mass_lock_m`) and inside the system (100 AU), and
`Some(what)` naming the lock otherwise. The client calls it to draw the ghost's state before PLOT; the server calls it on
the command. An authored arrival point always passes (it is checked at load); a free point near a body or a station may
not.

### 6.4 The flow on the console

1. Helm taps a stop, a place, a body or empty space on the map (or moves the pad's cursor to it and presses A). The ghost
   line runs from the ship to it; TARGET shows its icon and name and CLEAR or MASS LOCK.
2. **PLOT** (the panel's biggest control, a press, not a hold) sends `NavTarget`. The course line turns solid; TARGET
   says PLOTTED. A refused plot leaves the old target and flashes the lock that refused it.
3. Helm aligns: by hand, or ALIGN (section 8).
4. JUMP (held 0.6 s, unchanged) spools, on the captain's order.

Re-plotting during a spool is refused (`NotReady`): a stray tap must not cost a 40 s spool; abort the spool first (JUMP
held again aborts, as built).

## 7. Who decides

**Helm plots; the captain's JUMP order is the go** (Bridge Crew's split, section 2):

| Seat | Plots | Says go | Spools |
| --- | --- | --- | --- |
| Helm (a person) | Yes, on NAV | | Holds JUMP once the order is given (the order chip lights green until helm acts, as PM 7) |
| Captain (a person) | No: sees the plot on their own NAV tab (section 9) | `Patrol(Jump)`, now meaning "jump to the plotted target" | |
| Helm bot or automation | Keeps the preselection (6.2); never plots on its own | | On the order, as PM 9 and FN 10 |
| No captain aboard | | Tactical's order, or the helm bot takes it when the stop is done and the cooldown has ended (PM 9, unchanged) | |

The server does not gate a person's JUMP on the captain's order (as built): a crew talks, and a crew of two has no
captain. The order is what bots wait for and what lights the chip (question NM3).

## 8. Alignment, feeding ALIGN and JUMP

- **The jump vector** is the target's arrival minus the ship's position. On the map it is the plotted course; on the
  viewscreen (the look band, kept on NAV) the target is a small accent diamond where the vector meets the view, or an
  edge arrow when it is outside the view.
- **The ALIGN cue** in the JUMP panel is `patrol-consoles.html`'s glyph made live: a 5 degree cone, drawn 6 times wider
  than true so it reads, and the bow line at its true off angle (clamped to the panel), `faint` outside, `ok` inside
  with the rate under 2 deg/s; the off angle in whole degrees beside it. It is `patrol::aligned` drawn: the same
  function the server's spool asks.
- **ALIGN** (new autopilot mode, FN 10's "Point bow" with the jump vector as the target): turns the bow onto the vector
  through the flight assist at the ship's rate limits and holds it, until helm touches the stick or presses another
  mode. It is the existing `Command::Helm(Autopilot)` with a new mode byte; HOLD, COURSE, CHASE, MATCH and EVADE stay on
  the HELM tab's SCANNER.
- **The spool and the jump** are PM 5's, unchanged, except that the place they jump to is the plotted target.

## 9. The captain's view

The captain's console gains a third tab, `[COMMAND][SHIP][NAV]`. The map is the same function and the same picture,
read only: the plotted course, the ghost helm is choosing (so a captain sees what helm is considering, the Bridge Crew
conversation), the route. The right column:

| Panel | Holds |
| --- | --- |
| TARGET | Helm's plotted target: icon, name, PLOTTED or none |
| ORDER | The JUMP order as the biggest control (held 0.6 s, `Patrol(Jump)`), and three lights in a row with a shape each: PLOT (a target), ALIGN (the cue's state), POWER (the spool's power: green at full allocation, amber when engineering gives less, the spool's time then on hover) |

## 10. On the server

- `Command::NavTarget(NavPick)` from Helm, where `NavPick` is `Place(u8)` (index into the system's places: stations,
  points, then bodies, in file order), `Free([f64; 3])` (system frame) or `Next` (the preselection). Validation in
  `combat::patrol`: the station is helm; the phase is Patrol (docked included: a crew may plot before undocking); not
  spooling (`NotReady`); the place exists (`NoSuch`); the arrival is finite and inside 100 AU (`BadValue`);
  `target_clear` (`MassLocked`, the computer naming the lock as today's `mass_lock` line).
- `PatrolState` gains `target: Option<Target>` (the pick and its arrival), set by 6.2 after each arrival. `Jump(true)`
  needs a target (`NoSuch` otherwise, as today's `next`), and the jump moves the ship to the target's arrival. Arriving
  at a route stop makes it the stop the patrol is at; arriving anywhere else leaves the stop states as they are (PM 5.2,
  "moving on uncleared"). A stop passed over stays open and the debrief names it (PM 5.4).
- The snapshot's patrol block gains one byte: the target (a place index, 0xFE a free point, 0xFF none). A free point's
  position goes once, as a reliable `NavPlotted` event of 25 bytes. `PROTOCOL` is bumped.
- Bots (`sc-net::bot`) and automation do not send `NavTarget` (7).

## 11. The owner's system rules

### 11.1 Kinds and belts in the system file

- `bodies[].kind`: `rocky`, `gas_giant` or `asteroid`, required. `bodies[].parent`: the id of the body it orbits, for a
  moon (optional). `bodies[].rings`: `{ inner_m, outer_m }` (optional; a gas giant's ring for the map's glyph and, later,
  the sky).
- `belts`: a new list, `{ id, name, around, inner_m, outer_m }`, `around` the star (`"star"`) or a body's id.

**Why a belt is not a body.** A body has a centre and a radius, and two rules depend on it: the mass lock (3 radii of
its centre) and the sky's nearest planet (`sky_from` picks the largest disc). A belt has neither: a ring 4e10 m wide
would lock half the system or draw as a planet. A belt is a region the map draws and a later change may give rocks
(PM 11.6's far rocks are already one); the rocks a crew can name, lock against and jump to are asteroids, which are
bodies (`kind` asteroid, optionally a `belt` id), with the 3 radius lock like any body.

### 11.2 The checks (in `sc-core::system`'s `Validate` and a cross-file check in `check-data`)

| Rule | Check | Count |
| --- | --- | --- |
| 1. A unique background | No two files in `data/space/systems/` share `sky`, and the sky's file exists in `data/space/skies/` | A cross-file check, `sc-tools check-data` |
| 2. At least one station | `stations` has at least 1 | 1 |
| 3a. Several rocky worlds | Bodies of kind `rocky` (moons included) at least 3 | 3: "several" is more than two (question NM6) |
| 3b. At least one gas giant | Bodies of kind `gas_giant` at least 1 | 1 |
| 3c. Multiple asteroids | Bodies of kind `asteroid` at least 2 | 2: "multiple" is more than one (question NM6) |
| Kinds agree with sizes | A gas giant's radius at least 2.0e7 m; a rocky world's between 1.0e5 and 2.0e7 m; an asteroid's under 1.0e6 m | Catches a wrong kind |
| Parents | A `parent` names a body of the file, not itself, not an asteroid | |
| Belts | `inner_m` < `outer_m`, `around` names the star or a body; no station or point of interest inside a belt's band unless it is meant to be (the relay is: allowed) | |

A file breaking any rule is refused at load with the path and the field (CLAUDE.md 6.5), as every check in `Validate`.

### 11.3 The Shoals today, and the proposed delta

| Rule | Today | After the delta |
| --- | --- | --- |
| 1 | Bloom, the only system file: passes | Unchanged |
| 2 | Five stations: passes | Unchanged |
| 3a | Cinder, Tamsin, Pell (a moon): three rocky worlds, but no `kind` to count | `rocky` on all three: passes at 3 |
| 3b | Orrin is a gas giant by size (5.8e7 m) and by its look, no `kind` | `gas_giant`, with rings |
| 3c | None | Skerry, Holm and Wick in the Shoals belt: passes |

The proposed delta (applied to the data file only after the owner approves; the rest of each existing entry unchanged; Holm's and Wick's `look` is Skerry's):

```text
{
  "bodies": [
    { "id": "cinder", "kind": "rocky" },
    { "id": "tamsin", "kind": "rocky" },
    { "id": "orrin", "kind": "gas_giant", "rings": { "inner_m": 75000000.0, "outer_m": 120000000.0 } },
    { "id": "pell", "kind": "rocky", "parent": "orrin" },
    { "id": "skerry", "name": "Skerry", "kind": "asteroid", "belt": "the_shoals",
      "pos_m": [-378041000000.0, 1500000000.0, -254992000000.0], "radius_m": 60000.0,
      "look": { "ocean_srgb": [0.30, 0.28, 0.26], "land_srgb": [0.42, 0.39, 0.35], "cloud_srgb": [0.5, 0.5, 0.5],
                "atmosphere_srgb": [0.3, 0.3, 0.3], "atmosphere_frac": 0.0, "land_frac": 0.9, "cloud_frac": 0.0, "night": 0.02 } },
    { "id": "holm", "name": "Holm", "kind": "asteroid", "belt": "the_shoals",
      "pos_m": [-401960000000.0, 1500000000.0, -178964000000.0], "radius_m": 35000.0, "look": "as Skerry's" },
    { "id": "wick", "name": "Wick", "kind": "asteroid", "belt": "the_shoals",
      "pos_m": [-347384000000.0, 1500000000.0, -281306000000.0], "radius_m": 12000.0, "look": "as Skerry's" }
  ],
  "belts": [
    { "id": "the_shoals", "name": "the Shoals", "around": "star", "inner_m": 430000000000.0, "outer_m": 470000000000.0 }
  ]
}
```

Where they sit: the Shoals belt is PM 2's "Belt, 4.5e11 m from Shoal", 4.3e11 to 4.7e11 m, with the relay (4.50e11 m)
and the Shoals' edge stop (6e7 m sunward of it) inside it. Skerry (60 km radius, 3.2e10 m from the relay), Holm (35 km,
4.8e10 m) and Wick (12 km, 7.1e10 m) are 146, 156 and 141 degrees round from +x (the relay is at 150): in the same
reach of the belt as the patrol's last two stops, so the map shows them where the crew go, and far enough off (over 1e10
m, and their locks 180 km, 105 km and 36 km) never to touch the relay's fight or its 30-60 km far rocks (PM 11.6). The
names follow the system's: a skerry, a holm and a wick are a rock, an islet and a creek.

**A finding in PM 2, for that change**: "Cinder's ore lanes, 1.2e7 m from Cinder's centre, 60 km from Cinder Yards"
cannot both hold, because the yards orbit at 8.0e6 m (4,000 km below the lanes at the least). Likewise "Orrin's
rings, 1.3e8 m from Orrin's centre, 4.0e7 m from Pell" cannot hold with Pell 1.9e8 m from Orrin (the two distances sum
to 1.7e8 m). The mockup puts the lanes 60 km beyond the yards (NM9's recommendation) and the rings point 1.3e8 m from
Orrin toward Pell (6.0e7 m from it); PM's phase 3 data should settle both (question NM9).

## 12. The Pi 5 budget this change spends

Against `engine-stack` 5 and BS 8.7's console ceiling (16 draw calls and 6,000 triangles a console).

| Cost | Spend | Against |
| --- | --- | --- |
| Draw calls | The NAV tab draws the title band, the look band, three panels and the status strip: the HELM tab's count less one panel. The map is one clip rectangle; the hatch of a mass lock ring is line segments, not a texture | 16 a console |
| Triangles (egui, feathered) | Orbits at 48 segments, 4 to 6 drawn: about 1,150; belt dots, 2 arcs at 6 px spacing clipped to the map, at most 280 dots of 2 triangles: 560; lock rings with hatching at most 3 on screen: 600; glyphs (bodies, stations, points, stops, contacts) about 60 at 12 triangles: 720; labels about 80 glyphs: 160; route and course dashes about 100: 400. **About 3,600 for the map**, the TARGET and JUMP panels about 700: under 6,000 | 6,000 a console |
| Client CPU | Projecting about 40 things and placing labels by a fixed order of 8 offsets each: under 0.1 ms; the map's shapes rebuilt only when the zoom, the ship's cell on the map or a state changes (otherwise the cached mesh) | UI 2 ms |
| Memory | The system file is already loaded; label boxes and the cached mesh under 64 kB | |
| Server | `target_clear` on a plot: one loop over bodies and stations (under 30), under 2 microseconds; nothing per tick | 2 ms a ship a tick |
| Network | `NavTarget` 2 to 26 bytes a plot; 1 byte a snapshot; `NavPlotted` 25 bytes once | 80 kbit/s down |
| 3D | None | |

The mockup's cost line counts its SVG shapes and text runs as UI draw items (as `patrol-consoles.html` does), which is a
picture of the work, not a Pi measurement.

## 13. Data

| File | Change |
| --- | --- |
| `data/space/systems/shoals.json` | 11.3's delta, after approval |
| `data/ships/tern/jump.json` | Read: `arrival_scatter_m` as the clear margin; a new `parking_radii` 4.0 (6.1) |
| `data/missions/patrol-shoals.json` | Read: the route (PM 12c's stops and kinds); nothing new |
| `data/ui/console_style.json` | The NAV tab's icons (`map`, `plot`), generated from the mockup as today |

## 14. Mockup shots

`docs/mockups/helm-nav-map.html`, the 1280 x 720 console, real Shoals data inlined (`data:space/systems/shoals`,
`data:tern/jump`) with 11.3's delta and PM 12c's phase 3 stops overlaid in the page, marked proposed:

| Shot | Shows |
| --- | --- |
| `docked` | At Tamsin Station's berth: the system zoom, the route's six stops and home, Cinder orbit preselected, the ship's own lock ring hatched, JUMP at MASS LOCK |
| `choosing` | At Cinder's ore lanes, stops 1 and 2 done, Orrin's rings preselected; helm's ghost on the Pell beacon, PLOT lit |
| `aligning` | Orrin's rings plotted, the cooldown over, ALIGN on, the bow 14 degrees off; the target diamond on the viewscreen |
| `spooling` | Aligned, the order chip CAPTAIN: JUMP lit, the spool at 60 %, the course in `warn` |
| `refused` | At Cinder orbit, zoomed to CINDER: a free point tapped inside Cinder's lock, the lock ring in `danger` hatching, TARGET at MASS LOCK, PLOT off |
| `captain` | The captain's NAV tab: helm's plotted course, TARGET, the JUMP order with PLOT, ALIGN and POWER lights |
| `kinds` | The Shoals' far reach zoomed: the belt's dotted arcs, Skerry, Holm and Wick, the relay and the Shoals' edge |

## 15. Risks and trade-offs

- **Helm leaves the stick panels to plot.** Plotting happens in the cooldown, between fights; flying never needs a panel
  (BS 6); one tap or Tab returns.
- **A free point lets a crew skip the patrol.** Moving on uncleared is already allowed (PM P7); the debrief names the
  stops done. A free point is also how a crew runs from a fight it cannot win, which is a decision worth having.
- **One more tab for the captain.** COMMAND, SHIP and NAV fit the band (the start chip and the swap chips already give
  way to the own tabs, `start-mission` 4).
- **Rule counts are the owner's words read as numbers.** "Several" as 3 and "multiple" as 2 are recommendations
  (NM6); the checks read them from one constant each.

## Open questions

None asked yet. Those marked "screenshots" go into the owner's survey with the mockup's shots beside them (CLAUDE.md 13);
the others are recommendations taken (ask only with screenshots).

| # | Question | Options | Recommendation | Ask with |
| --- | --- | --- | --- | --- |
| NM1 | Where the map lives | A NAV tab on helm's chip; a SYSTEM mode on SCANNER; a separate screen | The NAV tab (section 3) | Screenshots: `docked`, `choosing` |
| NM2 | Who plots the jump | Helm; the captain; either | Helm plots, the captain's order is the go (Bridge Crew) | Screenshots: `captain`, `spooling` |
| NM3 | Does a person's JUMP need the captain's order on the server | A hard gate; the order lights the chip only | Only the chip: a crew of two has no captain, and a crew talks. Bots wait for the order | Recommendation taken |
| NM4 | What may be a target | Route stops only (Bridge Crew's campaign); any place; any place and free points | Any place, bodies at a parking point, and free points outside every lock (FreeSpace, Elite) | Screenshots: `choosing`, `refused` |
| NM5 | May the crew take the stops out of order | No; yes, the route only preselected | Yes: the preselection keeps the order for anyone who does not choose (closes PM P9) | Screenshots: `choosing` |
| NM6 | The rule counts: "several rocky worlds", "multiple asteroids"; do moons count | 2 or 3 rocky; 2 or 3 asteroids; moons in or out | At least 3 rocky worlds, rocky moons counted; at least 2 asteroids; at least 1 gas giant | Screenshots: `kinds` |
| NM7 | Asteroid belts: a body or a region | A body kind `belt`; a separate `belts` list | A separate list (11.1: a belt has no centre to lock on or to draw as a disc); the named asteroids are bodies | Recommendation taken |
| NM8 | Re-plotting during a spool | Refused; aborts the spool and plots | Refused: a stray tap must not throw away up to 40 s | Recommendation taken |
| NM9 | PM 2's Cinder's ore lanes: 1.2e7 m from Cinder or 60 km from the yards (and Orrin's rings: 1.3e8 m from Orrin or 4.0e7 m from Pell) | Keep 1.2e7 m (the lanes 4,000 km above the yards); move the lanes to 60 km from the yards at about 8.0e6 m (outside Cinder's 7.2e6 m lock) | Move the lanes to 60 km from the yards: the yards' lights are the stop's picture (PM 11.1); keep the rings 1.3e8 m from Orrin (6.0e7 m from Pell). For patrol-mission's phase 3 data | Recommendation taken |
| NM10 | The Shoals' new asteroids: names and where | Skerry, Holm, Wick in the Shoals belt near the relay; elsewhere | As 11.3 | Screenshots: `kinds` |
| NM11 | Does science see and mark the map | Science reads it only; science adds contacts (FN 9) | Contacts from science's sweep appear on it (FN 9); a science NAV tab is a later change | Recommendation taken |
