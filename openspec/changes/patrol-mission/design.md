# Design: patrol-mission

The owner, 2026-10-14: "have the ship undock from star base, go patrol a star system and encounter hostiles", and "the
view through the window should reflect whats going on around the ship". And 2026-10-15: "there should b something
describing a patrol mission where we travel to various planets in the star system and encounter enemies we have to
clear out, begining of the game starts at space dock".

The 2026-10-15 words widen the route: from one empty beacon and one fight (built, phase 2) to a tour of the system's
planets (phase 3, sections 1, 5, 6 and 12b), with the owner's mix later the same day: "Some stops should just be scan
the area or planet and move on, some stops will be give some fuel to a stranded civilian maybe 2-3 stops will be actual
battle". The start is unchanged: docked at the spacedock.

Numbers marked FN come from `flight-and-navigation`, WS from `weapons-and-shields`, CD from `coop-drill`, PG from
`power-grid`, EF from `enemy-fighters`, M from `missions`. A number this change sets is in data and named in section 14.

## 0. What exists

- **The drill** (CD 1, built): Muster, a 5 s countdown, Engage (the Hound 6 km ahead, its two Jackals launched 20 s
  in, EF 1), Debrief. `sc-core::combat` steps at 30 Hz in `f64` in one frame round one origin, positions within 20 km.
  The Tern starts at the origin at 60 m/s; `waypoint_m` [9000, 1500, 16000] is "the relay at the Shoals", where the
  helm's COURSE flies; nothing is drawn there.
- **The jump** (FN 7, M 3, designed): 800 MJ spooled at 20 MW (40 s), the bow within 5 degrees of the jump vector and
  turning under 2 deg/s for the last 3 s, mass lock within 3 radii of a planet's centre, 5 km of a ship over 500 t (the
  Hound locks the Tern) or 2 km of a station, arrival at a point of interest's arrival point (at least 5 km from
  anything) scattered within 1.5 km, a 60 s cooldown, the velocity kept relative to the reference. `warp-pylons` 3: no
  jump while either pylon's capability is 0.
- **Docking** (FN 10, `escape-pods-and-docking` 2, designed): the docking corridor's limits (2 m/s inside 100 m,
  0.5 m/s inside 10 m, 0.2 m/s at contact); the Tern's docking collar is the airlock's outer door on the starboard hull,
  deck C (x -10.8 m, z 12 m).
- **The outside** (`deck-pipeline` 13b, CD 9.1, `tern-exterior`, built): `data/space/exterior.json` gives the sun (a
  direction), one planet (a direction and an angular radius), the stars, the dock's girder frame round the hull (deck
  `outside` in the deck file, drawn in the walk and not in the drill) and the bow camera. The drill's bridge
  (`sc-client/src/drill_bridge.rs`) draws the sky turned by the Tern's attitude, every room of every deck, the hull,
  and the fight's objects (`drill.rs` `World::draw_objects`) at the back of the depth buffer, so every window of every
  deck shows them; the viewscreens show the feed (`World::draw`: the same sky and objects from the captain's camera).

## 1. The mission in one page

`data/missions/patrol-shoals.json`, "Patrol: the Shoals". The game begins at the spacedock, at the O'Neill cylinders:
Tamsin Station is the O'Neill pair (spacedock version E, section 3; the owner, 2026-10-15: "we do start at the oneil
cylinders"), and the Tern lies berthed on the wall of its round bay, in the bay's baked light (spacedock 8a). The patrol tours the system's planets, and at each stop
there are hostiles the crew must clear out before the next jump.

| # | Phase | What the crew do, on foot | Ends | Typical time |
| --- | --- | --- | --- | --- |
| 1 | Muster | Join, claim stations, read the briefing (the route on the system map, its six stops), READY | Every held station ready | |
| 2 | Countdown | 5 s on every screen (CD 1) | Timer | 5 s |
| 3 | Docked | Walk from the muster rooms to the stations (CD 9). Engineering brings the reactor up, closes the drive's breaker and drops shore power (section 4.1). The captain orders UNDOCK | Helm releases the clamps | 60-90 s |
| 4 | Undocking | Helm flies the berth's departure legs out of the bay, down the axis and out of the mouth (4.2) | Clear of the pair, past its mass lock | 4-5 min |
| 5 | Stop 1, **scan**: Cinder from orbit | Science scans the planet (5.3a); nothing hostile | The scan completes | 1 min |
| 6 | Stop 2, **battle**: Cinder's ore lanes | A pack of three Jackals raiding the haulers out of Cinder Yards, found by the sweep; the gunner's and the cannons' fight (5.3) | Clear | 2-3 min |
| 7 | Stop 3, **civilian**: Orrin's rings | The hauler *Marrow Lark*, out of fuel and drifting: helm comes alongside and holds; a crew member walks to the fuel coupling at the docking collar on deck C and connects the line; engineering pumps (5.3b) | The fuel is across | 3-4 min |
| 8 | Stop 4, **battle**: the Pell beacon, at Orrin | The beacon has gone dark; the Hound and its two Jackals: the drill's fight (CD 1, 2) | Clear | 4-6 min |
| 9 | Stop 5, **scan**: the Shoals' edge | Science sweeps the belt's edge for the raider's track (5.3a): it points to the relay | The scan completes | 1 min |
| 10 | Stop 6, **battle**: the Shoals relay | A raider stripping the relay, with two Jackals: the patrol's hardest fight (5.3) | Clear | 5-7 min |
| 11 | Home | The captain orders the jump home; helm aligns; the spool, with what the ship has left | Arrival at Tamsin Station's arrival point, 5 km out | 40 s |
| 12 | Debrief | The end screen (M 4): PATROL COMPLETE (every stop's job done), PATROL INCOMPLETE (the stops done, by name) or THE TERN IS LOST | 20 s, up to 240 s for damage control (CD) | |

Between every two stops: the 60 s cooldown and the next 40 s spool, the crew's time to put the ship right (5.2).

About 30 to 35 minutes for a crew that knows the ship: six stops, three of them fights of rising strength with quieter
stops between (two scans and a rescue), a breather after each to put the ship right, and the ship's damage, fires and missiles carried from stop to stop (CLAUDE.md 7: a ship persists).
Docking again at the end is not in this change: it is FN 10's dock autopilot, a later change; the mission ends on
arrival home.

**Why each fight keeps the drill's numbers.** A stop's fight starts the drill's Engage with its hostiles placed from
the Tern's arrival exactly as the drill places the Hound from the Tern's start (`enemy_start_m` in the Tern's frame), so
the numbers the drill was tuned with (the 6 km start, the 1,800 m orbit, the fighters at 20 s) hold at the Pell beacon,
and the other two stops are tuned from them. Only the frame moves: section 5.3.

**As built (phase 2, 2026-10-14)** the route is two places: the Pell beacon, swept and empty, then the Shoals relay
with the Hound and its fighters, then home. Phase 3 (section 12b) turns it into the three-stop clearing route above.

## 2. The Shoals system

`data/space/systems/shoals.json` (schema `starcrew.system/1`): FN 13's system file, placed beside
`data/space/exterior.json` so the outside's data sits together. The system frame: origin at the star's centre, x and z
in the plane of the planets' orbits, +y north; positions `f64` metres. For this change the bodies are fixed where they
stand at the mission's epoch: FN 8's rails (Kepler's equation each tick) come later and change nothing here but the
positions' time (an 11 minute mission moves Tamsin about 22,000 km along its orbit, which no window shows).

| Body | Kind | Position or orbit | Radius | Look |
| --- | --- | --- | --- | --- |
| Shoal | Star | The origin | 5.6e8 m | Orange-white, sRGB [1.0, 0.93, 0.82] |
| Cinder | Planet, rock | 5.5e10 m from Shoal | 2.4e6 m | Dark, no air: a point of light from the patrol |
| Tamsin | Planet, terran | 1.2e11 m from Shoal (0.80 AU) | 6.0e6 m | `exterior.json`'s planet: oceans, land, cloud, a blue rim |
| Tamsin Station | Starbase | 2.0e7 m from Tamsin's centre (3.33 radii: outside its 3-radius mass lock) | 600 m (the habitat ring) | Section 3 |
| Orrin | Planet, gas giant | 3.0e11 m from Shoal (2.0 AU) | 5.8e7 m | Banded tan and cream, no land |
| Pell | Moon of Orrin, ice | 1.6e8 m from Orrin | 1.2e6 m | White and grey |
| Pell beacon | Point of interest | 2.0e8 m from Orrin, 4.4e7 m from Pell | | A picket buoy, one blinking light |
| The Shoals | Belt | 4.5e11 m from Shoal | | Far rocks (section 11.6) |
| Shoals relay | Point of interest | In the belt | 120 m | Section 3, the relay placeholder |
| Orrin's rings | Point of interest (phase 3) | 1.3e8 m from Orrin's centre, outside the rings' plane by 2,000 km, 4.0e7 m from Pell | | Where the *Marrow Lark* drifts: Orrin's banded disc (26 degrees radius) and the rings edge on |
| The Shoals' edge | Point of interest (phase 3) | On the belt's inner edge, 6.0e7 m sunward of the relay | | Far rocks thinning toward the star |
| Cinder orbit | Point of interest (phase 3) | 1.2e7 m from Cinder's centre on its day side, 200 km from the lanes | | The scan's point: Cinder's lit face filling the bow |
| Cinder's ore lanes | Point of interest (phase 3) | 1.2e7 m from Cinder's centre (5 radii, outside its lock), 60 km from Cinder Yards | | The haulers' lane: Cinder Yards' lights 60 km off, Cinder a dark disc |

Each point of interest has an `arrival_m`: Cinder's ore lanes' 8 km from the lane's centre on the yards' side; the Pell
beacon's 5 km sunward of it; the relay's at the drill's offset,
the relay standing at `waypoint_m` from it in the Tern's arrival frame, so the drill's geometry holds; Tamsin
Station's 5 km out along its berth arms' axis.

What a window sees follows from these: the star's disc is 1.17 degrees across at Cinder, 0.53 at Tamsin, 0.21 at
Orrin and 0.14 at the Shoals; Cinder from its lanes is a dark disc of 11.5 degrees radius (`asin(2.4 / 12.0)`); Tamsin from the station is a disc of 17.5 degrees radius (`asin(6.0 / 20.0)`), Orrin from the beacon one of
16.9 degrees (`asin(5.8e7 / 2.0e8)`), Pell from the beacon one of 1.6 degrees.

**The dock's view follows.** `exterior.json`'s `sun.dir` and `planet.dir`, `planet.radius_deg` describe the view from
the berth today, by hand (the planet 34 degrees in radius). In phase 2 they are derived from the system file and the
berth's attitude instead (section 11.2), and Tamsin shrinks to 17.5 degrees in radius in the port windows: the
station has to sit outside Tamsin's 3-radius mass lock to jump from it, and a 34 degree disc would need it at 1.8
radii. The berth's attitude is the one that keeps the star and Tamsin where the dock's view has them (the star high to
starboard and ahead, Tamsin low to port), computed by a tool from the two directions and checked, never typed in
(CLAUDE.md 6.6).

## 3. Tamsin Station, the other stations, and the relay

**Tamsin Station is spacedock's version E, the O'Neill pair** (the owner, 2026-10-14, spacedock S1: "I want to keep all
of them as various stations you can visit across the system, starting at oniel pairs"). The system file lists the five
stations of spacedock design 11a, each with its version, its body and orbit (or a point and an offset), its attitude,
its mass lock, its arrival point and its berths; `sc-core::system` checks every body or point exists, every orbit is
outside the jump's planet lock (`jump.json`), every berth is inside its version's bay (`sc-core::spacedock`, which
reads the bays and the traffic limits from `data/space/spacedock.json`) and every departure leg keeps to the bay's and
the corridor's limits. Tamsin Station's axes point at the star, Tamsin below it (a test). The patrol starts in its round
bay; the placeholder starbase and its girder cradle are gone from the drill (the walk's dock in `exterior.json` stays
for the tour). The client draws E from the same sizes (`stations::oneill`, design 12b) and the other four as flagged
placeholders.

The relay at the Shoals stays the placeholder generator's `StationKind::Relay`: a 120 m truss with three dishes, a
solar wing and two beacons, 136 triangles. A beacon is drawn at least `beacons.min_radius_deg` (0.15 degrees) in
angular radius however far off it is.

## 4. Undocking

### 4.1 Docked

- **Shore power.** Docked, the umbilical is connected and the impulse drive's feed breaker is open (the berth's rule:
  no drive while clamped). The reactor runs on AUTO and carries the docked ship's load: a reactor at standby on a
  shore feed would need the grid to model a shore source, which it does not yet (power-grid), and a standby reactor
  without one browns the ship out; so the umbilical is a connection the clamps refuse to open with, not a power
  source (as built, phase 2).
- **Engineering's job** (on foot: the engine room's console, or the bridge's engineering station): close the drive's
  breaker (the console's own breaker control, power-grid 5: the real grid's breaker, the drive's demand jumping and the
  reactor ramping at its 0.02 of rated a second), and drop SHORE. DRIVE is ready when shore power is off and the
  impulse drive's capability (its power and its integrity) is at least `drive_ready_min` 0.5; the computer says
  "Drive on line".
- **The captain's job**: the briefing's route on the system map (FN 9), and the order UNDOCK once DRIVE is green.
- **Tactical**: weapons are locked inside 5 km of the station (`weapons_safe_m`, the station's rule); the console shows
  SAFE in the turret modes' place until then.
- **Science**: the station's traffic on the local plot; nothing to do yet.

### 4.2 The clamps and the bay

- **UNDOCK** is Helm's, a guarded control held 0.6 s (BS 8.5, as M 3's JUMP), live only when DRIVE is green and the
  captain has ordered it (or, with no captain aboard, at once). The clamps open in 3 s (`clamps_s`).
- **The berth** is on the round bay's wall, keel to the wall, bow toward the mouth (spacedock 5a: `berths` slot
  [1, 0], 200 m from the axis, 60 m up the bay). The departure is the berth's legs in the system file, flown by the
  assist with the attitude held, never faster than each leg allows (spacedock design 6's `bay_mps` 5 and
  `corridor_mps` 40, a test holds the legs to them):

| Leg | To (the berth's frame) | Most | Time |
| --- | --- | ---: | ---: |
| Up off the clamps | 10 m up | 1 m/s | 10 s |
| To the axis | 200 m up | 5 m/s | 38 s |
| Down the axis, through the iris and the throat, out of the mouth | 780 m ahead | 5 m/s | 156 s |
| Clear of the dome and the pair, past the lock | 3,000 m ahead | 40 m/s | 60 s |

  About **4.4 minutes** from the clamps (a test: 240-300 s). The iris stands open (its doors are spacedock 2.6).
- **Automation** (no helm aboard) flies the same legs.

### 4.3 Clear of the station

The departure ends on Tamsin Station's axis 2.2 km beyond its mouth, on the star's side, clear of the dome, the
petals and the farm ring, and outside the station's mass lock (2,780 m from its frame's origin: the mouth's 780 m and
FN 7's 2 km, spacedock S4's "from the hull" on the axis). The helm takes her from there: the next jump's spool, the
turn toward its vector during it.

### 4.4 Clamps to the first jump

3 s + 264 s + 40 s spool: **about 5 minutes** from the clamps to the first jump (a crew of automation in a test: clear
after 268 s, the jump 40 s later), after the minute or so of walking and starting the drive.

## 5. The route and the jumps

### 5.1 Jump, not cruise

FN 7 already chose the jump drive over a cruise drive (its table: every station has a job in the 40 s spool, nothing
moves fast near anything, one rebase at a tick, one network event; a cruise drive at 0.01 c takes minutes per AU and
moves a pose thousands of kilometres a tick). This change takes that choice and its numbers: 800 MJ at 20 MW, 40 s; the
alignment; the mass locks; the arrival; the 60 s cooldown. On impulse the Tern flies at up to 400 m/s (the assist's
cap, `data/ships/tern/flight.json`) inside a waypoint's few kilometres.

| Leg | From | To | Distance | Spool | Transit |
| --- | --- | --- | --- | --- | --- |
| 1 | Tamsin Station, past its lock | Cinder orbit | 1.0e11 m | 40 s | Instant at one tick |
| 2 | Cinder orbit, after the 60 s cooldown | Cinder's ore lanes | 2.0e5 m | 40 s | Instant (a jump: 200 km at 400 m/s would be 8 minutes) |
| 3 | Cinder's ore lanes | Orrin's rings | 2.4e11 m | 40 s | Instant |
| 4 | Orrin's rings | The Pell beacon | 4.0e7 m | 40 s | Instant |
| 5 | The Pell beacon | The Shoals' edge | 7.1e11 m | 40 s | Instant |
| 6 | The Shoals' edge | The Shoals relay | 6.0e7 m | 40 s | Instant |
| 7 | The Shoals relay, clear of any hostile ship's 5 km lock | Tamsin Station | 3.9e11 m | 40 s | Instant |

The order is the stops' rising strength with a quiet stop before each fight, not the distance: a jump costs the same 40 s whatever the distance (FN 7). As
built (phase 2) legs 1 and 2 were one leg, Tamsin to the Pell beacon, with nothing at Cinder.

### 5.2 At a stop

- **Arrival** at the point's `arrival_m`, scattered within 1.5 km (seeded, CLAUDE.md 6.4), the velocity kept relative
  to the reference (FN 7: the Tern arrives moving as it left).
- **Science's sweep**: SWEEP on the science console (seated, on foot) takes 20 s (`sweep_s`) and reports the contacts
  within 50 km (`sweep_range_m`). On arrival a stop's hostiles that are in the open (a ship) are on the plot at once; a
  pack lying among rocks or behind the yards (the Jackals at Cinder) shows only once the sweep finds it, so the first
  thing the crew do at a stop is sweep.
- **The fight**: once the stop's hostiles are on the plot, red alert, and the drill's Engage runs with that stop's group
  (5.3). No second countdown: the crew are at their stations.
- **Clear**: every hostile of the stop's group destroyed, and a sweep after the last one shows none within 50 km. The
  objective "Clear <stop>" is done and the computer says "<Stop> clear".
- **Moving on uncleared**: the captain may order the next jump with hostiles left, once no hostile ship over 500 t is
  within 5 km (FN 7's lock; fighters do not lock). The stop's objective stays open; a pack or a ship left behind is not
  met again.
- **Between stops**: the 60 s cooldown and the next 40 s spool are the crew's time to put the ship right: fires out,
  machines repaired on deck, power rerouted. The Tern's damage, fires, missiles and the Swifts' state carry over; nothing
  is reset at a jump. A damaged reactor or pylon stretches the next spool (`warp-pylons` 3).

A stop is one of three kinds (`kind` in the route, phase 3): **scan** (5.3a), **battle** (5.3) or **civilian**
(5.3b). Its objective is done when its job is: the scan complete, the battle's group cleared, the fuel across. Any stop
may be left undone, as "moving on uncleared" above.

### 5.3 The battle stops' hostiles

Each battle stop names a hostile group (`groups` in the mission, phase 3). The fight's rules are the drill's; only who
is there changes.

| Stop | Group | What it is | Placed | Clear when | Strength |
| --- | --- | --- | --- | --- | --- |
| Cinder's ore lanes | `jackal_pack` | Three Jackals (WS 13, EF 1) with no carrier, raiding the haulers out of Cinder Yards | 4-6 km from the arrival on the yards' side, found by the sweep, then attacking at once | All three destroyed | The warm-up: the cannons' and the gunner's fight (EF 2: a bolt does its full damage to a fighter); the shields take the Jackals' cannons with little harm |
| The Pell beacon | `hound_patrol` | The Hound and its two Jackals: the drill's fight, unchanged | `enemy_start_m` 6 km ahead, the Jackals launched 20 s in (EF 1) | The Hound and both Jackals destroyed | The drill's |
| The Shoals relay | `relay_raider` | The Grabon raider (`missions` design 2: faster than the Hound, two heavy turrets, four tubes, a heavier hull, weaker shields), escorted by two Jackals | 6 km ahead of the arrival, at the relay | The raider and both Jackals destroyed | The hardest: the crew arrive with the first two fights' damage. Until `missions` builds the raider's class, the Hound with four Jackals stands in |

- **The frame**: combat already runs in `f64`; at each stop its origin moves to the arrival point (a floating origin,
  section 7), so positions stay within a few tens of kilometres and every draw subtracts the eye before narrowing to
  `f32` (CLAUDE.md 7).
- **Withdrawal**: a ship of a group that reaches the drill's 600 s limit withdraws past 5 km and jumps away, and the
  stop is not cleared. The Jackals of a group whose carrier has gone stay and fight.
- **Losing**: the Tern's hull at zero ends the patrol at any stop: THE TERN IS LOST.

### 5.3a The scan stops

Arrive, scan, move on: the patrol's quiet stops, a breath between fights, and the reason science has a job outside one.

- **Cinder from orbit**: SCAN with a planet selected on the science console (the sweep's control, aimed at a body)
  takes 30 s (`scan_s`) while the body is within 2.0e7 m and the Tern holds its attitude within 10 degrees of it; it
  reports the surface (Cinder's mines, the yards' traffic) and finds nothing hostile.
- **The Shoals' edge**: the sweep of 5.2 at the belt's edge, 20 s; it finds the raider's drive trail pointing to the
  relay and no contact. The computer says where it leads, which is the briefing for the last fight.
- Done when the scan completes; nothing to clear. A crew may jump on without it (the objective stays open).

### 5.3b The civilian stop: fuel for a stranded hauler

At Orrin's rings the hauler *Marrow Lark*, a civilian ore carrier (no weapons, 800 t; a friendly ship locks no jump), has run its
drive dry and is drifting at 30 m/s across the rings' shadow. Its master hails on arrival ("Patrol ship, we're dry,
can you spare us fuel?").

- **Helm** comes alongside: within `fuel_range_m` 100 m of the hauler and under `fuel_rel_mps` 1.0 m/s relative to it,
  starboard side to it (the docking collar is on the Tern's starboard hull, deck C, x -10.8 m, z 12 m). The hauler holds
  still; the Tern does the flying. Automation can hold station; a bot helm does.
- **A crew member walks to the fuel coupling**: a panel at the docking collar's inner door in the airlock (deck C, a
  new interactable of `ship-interactables`), held E 2 s to connect the line the hauler passes across, so the transfer
  is a walk to the hull, not a console button (the law).
- **Engineering pumps**: the engineering console's FUEL control (a mockup first, CLAUDE.md 10) moves `fuel_give_kg`
  0.25 kg of the reactor's deuterium and helium-3 at `fuel_rate_kg_s` 0.005 kg/s (50 s) from the Tern's store
  (`power.json` reactor `fuel_load_kg`, 4.0 kg) to the hauler. The Tern's own fuel goes down by that much and stays down
  for the patrol (a ship persists), a sixteenth of its load: hours of full power, not minutes, so it never strands the
  Tern on this patrol.
- **The line parts** if helm drifts past 100 m or 1 m/s relative while it is connected: the transfer stops, the coupling
  must be connected again. A fight is not met here.
- Done when the fuel is across: the hauler's master thanks the crew and the *Marrow Lark* lights its drive and leaves.
  The objective "Fuel the *Marrow Lark*" is done.

### 5.4 Home

The captain orders the jump home, after the last stop or at any time between stops (cutting the patrol short); helm
aligns; the spool (with what engineering can free after the fights: a damaged reactor stretches it, `warp-pylons` 3 a
damaged pylon). Arrival at Tamsin Station's arrival point, 5 km out, ends the mission: PATROL COMPLETE when every stop
was cleared, PATROL INCOMPLETE with the stops cleared named when not (M 3, 4). As built (phase 2): MISSION COMPLETE when
the Hound was destroyed, ESCAPED when it withdrew.

## 6. Objectives

`objectives-tab` 2's kinds, with new ones:

| Words | Kind |
| --- | --- |
| Undock from Tamsin Station | `undock` (built): done when the Tern is clear of the station |
| Scan Cinder | `scan` (phase 3), a body: done when science's scan of it completes |
| Clear Cinder's ore lanes | `clear` (phase 3), the stop: done when its group is destroyed and a sweep there shows no hostile |
| Fuel the *Marrow Lark* | `refuel` (phase 3): done when `fuel_give_kg` is across |
| Clear the Pell beacon | `clear`, the stop |
| Sweep the Shoals' edge | `sweep` (built): done when science's sweep there completes |
| Clear the Shoals relay | `clear`, the stop |
| Keep the Tern's hull above zero | `survive` |
| Return to Tamsin Station | `return` (built): done on arrival home |

`clear` replaces the built `sweep` at the Pell beacon (which finds nothing as built) and the built `destroy` and
`destroy_fighters` at the Shoals (the Hound and its fighters are now one stop's group). `sweep` stays a kind, for a
mission whose stop has nothing in it.

## 7. Commands and state (phase 2; built as below)

All through the existing validation and apply path (CD 3: a person, a bot and automation differ only in what they
decide). Each new control goes into `docs/mockups/consoles.html` for the owner's approval before it is built
(CLAUDE.md 10).

| Command | From | Does |
| --- | --- | --- |
| `Power(Breaker(drive group))` | Engineering | The existing breaker command, on the drive's group |
| `Shore(bool)` | Engineering | Connects or drops shore power, docked only |
| `Undock` | Helm, held 0.6 s | Opens the clamps when DRIVE is ready (refused `NotReady` otherwise) |
| `Jump(true)` / `Jump(false)` | Helm, held 0.6 s (M 3) | Spools toward the patrol's next place (refused `MassLocked` or `NotReady` in the cooldown), or aborts |
| `Sweep` | Science | The 20 s sweep at the point the Tern is at; at a stop with a hidden group, the sweep that finds it (phase 3) |
| `Patrol(Undock)`, `Patrol(Jump)` | Captain | The captain's standing order to the helm; a separate command, not a `stations.json` verb, so no console draws it before its mockup is approved |

`Phase::Patrol` (wire byte 4) is the phase of a patrol outside a fight; Engage is the drill's, entered at the
encounter. The snapshot gains a `patrol` block of about 50 bytes: the stage, the drill's local origin in the system
frame (three `f64`, so a client can place the sky), the place it is at and the next, seven flags (shore, the drive's
breaker open, drive ready, spooling, sweeping, undocked, mass locked), the spool's and the sweep's fractions, the
cooldown, each place swept, the standing order and the jumps made. `PROTOCOL` is 15. Two refusals join: `NotReady`
and `MassLocked`. The drill's positions stay small: its frame's origin moves to the arrival point at each jump (a
floating origin, CLAUDE.md 7), so the wire's 1,000 km position limit and the client's `f32` never see 1e11 m.

## 8. The phases on the server

`sc-core::patrol` (new): the system file's loader and checks, the berth, the patrol's phase machine and the jump
between waypoints, stepped from `combat`'s tick before the flight (commands, automation, then the patrol, then flight,
CD 2's order). It owns no flight: undocking drives the same assist with set points and the berth's limits as a speed
cap, so the helm's stick, a bot and automation fly the same Tern.

## 9. The bots and the chat

The single-player crew (`single-player` 2) and the bot clients (`sc-net::bot`) play every phase at their profile
(`data/stations.json`); automation (an empty seat) is slower and never jumps on its own (FN 10). Each line is what the
bot sends through `crew-chat`'s `Chat`, at most one each 3 s; `crew-chat` owns the message, this table only names the
lines.

| Station | Docked and undocking | Departing and the jumps | At Cinder | At the Pell beacon | At the Shoals | Between stops, home |
| --- | --- | --- | --- | --- | --- | --- |
| Engineering | The drive's breaker, SHORE off. "Off shore power" (the computer: "Drive on line") | The spool draws on the drive's own power (as built) | As the drill | As the drill. "Rerouting round the damage" | As the drill | Power to the next spool; sends damage control. "Spool power ready" |
| Helm | Waits for the order; UNDOCK; flies the legs. "Clamps released", "Clear of the berth" | Clear of the lock, aligns, holds. "Clear of the station", "Aligned", "Jumping" | Turns the bow to the pack | As the drill (`bot` profile: bow on at 1,500 m, weaving) | As the drill, against the raider | "Aligned for Pell", "Aligned for home" |
| Tactical | Weapons SAFE. "Weapons safe" | Past 5 km: "Weapons free" | Turrets on the Jackals; holds the missiles for later | "Locked on the Hound", "Missile away" | "Locked on the raider" | "Missiles: N left" |
| Science | Traffic on the plot | | SWEEP. "Three fighters, bearing ..." | "The beacon is dark. Contact: Hound class" | "Contact: raider at the relay, two fighters" | After the last kill, SWEEP. "Cinder clear" |
| Captain | UNDOCK when DRIVE is green. "Undock" | "Jump to Cinder" | "Red alert", "Gunner, take the fighters" | The drill's orders | The drill's orders | "Put out the fires, then jump to Pell", "Take us home" |
| Laser banks, gunner | At their seats | | The gunner on the dorsal turret | As the drill | As the drill | Damage control, if no team |

With no captain aboard the helm bot takes the order itself when DRIVE is green, and the next jump when the stop is
clear and the cooldown has ended, as FN 7 lets tactical order the jump when there is no captain.

## 10. The view through the windows: what is there and what is missing

The Tern's 20 windows (`command_suite.json`, `exterior_windows.json`): the bridge's two forward quarters (45 degrees,
2.0 x 1.2 m), the ready room to port and the briefing room to starboard (2.0 x 1.0 m) on deck A; on deck B the
quarters and the medbay to port, the mess and damage control to starboard (1.7 x 0.8 m), looking along the pylons and
nacelles; on deck C life support and the shield room to port, cargo and the switchboard to starboard (1.3 x 0.9 m).
The viewscreen on the bridge's bow wall and the screens on the command deck show the feed.

Found by reading the client and the models (2026-10-14):

1. **The hull's light is fixed to the ship and off the sky's sun.** `deckc` packs the model's vertex colours, which
   are the exterior build's bake: `ambient_fraction` 0.38 plus the rest times the light from `sun_dir_ship`
   [-0.5, 0.8, 0.35] (`data/ships/exteriors.json`, `tools/blender/build_ship_exteriors.py`). The sky's sun is
   `exterior.json`'s [-0.45, 0.55, 0.7], 25 degrees away; and in the drill the sky turns with the Tern's attitude
   while the hull's lit side stays where the bake put it, so a roll in the fight leaves the nacelle outside the mess
   lit from a sun that has moved to the other side of the window.
2. **Two copies of one lighting rule.** The Hound and the Jackals are lit by an ambient cube from the sun, written out
   twice in `World::draw_objects`, the Jackals' copy without the Hound's fill from the shade side (CLAUDE.md 6.1).
3. **The relay is a point with nothing at it.** The helm's COURSE flies to `waypoint_m`, "the relay at the Shoals",
   and the windows show empty space there.
4. **Nothing outside depends on where the Tern is.** The sun and the planet are fixed directions with a fixed size.
   Within the drill's 20 km that is right (Tamsin's direction would move by under a thousandth of a degree); after a
   jump it is wrong: the patrol needs them from the Tern's position in the system file.
5. **No motion past the windows.** The stars are at infinity; at 60-400 m/s nothing passes the windows but the Hound
   and the bolts, so the Tern looks parked.
6. **No starbase.** The walk's dock is girders round the hull with nothing they belong to.
7. **The windows elsewhere work.** The drill's bridge draws every room of every deck and the outside over the whole
   target behind them, so the quarters', the medbay's and the cargo hold's windows already show the sky, the hull and
   the fight (checked in the captures, section 12).
8. **The viewscreen is one implementation with the windows.** The feed draws `World::draw` (the sky and the same
   objects), so whatever the windows gain the viewscreen gains.
9. **A safety sign in a window.** The extinguisher brackets' green safety signs (`safety-points`) stand in front of
   the bridge's forward-quarter windows and the ready room's, seen against the stars in the captures: a bracket placed
   on a window's wall. It is `safety-points`' to move, and is reported there; this change does not touch it.
10. **Not in this change, on purpose**: sunlight falling through a window onto the bridge's floor (the interior is
   baked, CLAUDE.md 9; a moving sunbeam is a dynamic light the Pi does not draw), the planet's shadow on the Tern (the
   route never passes behind Tamsin), and the Tern's own lights on the station.

## 11. The view through the windows: what each shows

Every window and the viewscreen draw the one exterior state of the frame (`ship-frames` 7: one ship pose a frame):
the snapshot's ships, craft, missiles, bolts and bursts, the system file's bodies and stations, and the Tern's pose in
the system frame. Nothing is a backdrop.

### 11.1 By phase

| Phase | Bridge windows (forward quarters) | Port side (ready room, quarters, medbay, life support, shield room) | Starboard side (briefing room, mess, damage control, cargo, switchboard) | Viewscreen (the feed) |
| --- | --- | --- | --- | --- |
| Docked | The round bay's wall curving overhead in its rings' baked light, a ship berthed a quarter turn round in its floods' pool, the back wall's lit window tiers (spacedock 8); the bay's alert state's red strips when the station raises it | The bay's far wall and the berths across it, the docked ships lit by the bay's probes | The pier a few metres off under the berth's work lights, the clamps on the hull, the crew tube at the collar (deck C) | The bow camera: down the bay to the iris |
| Undocking | The bay's wall dropping away as the Tern lifts to the axis, then the iris opening ahead and the throat's rings sliding past; the hull's light going from the bay's to the star's over the throat | The berths and ships receding round the bay; then the throat's wall | The pier falling away; then the throat | The iris, the throat, the mouth and the star |
| Departing | The station turning out of view as helm turns; motes streaming aft | The station receding: 1.2 km across, 33 degrees at 2 km, 3.4 at 20 km; Tamsin; the star | The same from the other side | As the bridge |
| Spool | The bow swinging onto the jump vector; in the last 3 s the stars ahead stretch | As departing; the stars stretch toward the bow | The same | The stretch |
| Jump | Streaks from the bow's point through every window, a white flash of 0.3 s at the jump's tick, then the new sky settling in 1 s | Streaks running aft along the side windows | The same | The same streaks |
| Cinder's ore lanes (phase 3) | Cinder's dark, cratered disc (11.5 degrees radius) under a large, bright star (1.17 degrees across); Cinder Yards' long hall 60 km off as a line of lights; the Jackals' flares and bolts | The same from the side | | |
| Pell beacon | Orrin's banded disc (16.9 degrees radius) where the Tern points; Pell a small icy disc; the beacon's light 5 km off; the star smaller | Whatever the Tern's attitude puts there, from the same bodies | | |
| The Shoals | The Hound at 1-6 km, the Jackals, bolts, missiles and bursts (today's), the relay 18.6 km off with its beacon, far rocks of the belt | The same fight seen from the side, the star at 0.14 degrees | | The captain's camera on the fight (CD 9.1) |
| Home | Tamsin and the station at 5 km, its lights; the cradle empty | | | |

### 11.2 The sky from the system file (phase 2)

Each frame the client computes, from the eye's system position (the Tern's pose and the eye, `f64`):

- **The star**: its direction, its disc's angular radius (`asin(R / d)`), and its light: the colour times
  `clamp((d_ref / d)^0.5, 0.55, 1.3)` (`light_exponent` 0.5, `d_ref` Tamsin's 1.2e11 m: a softened inverse square, so
  Orrin's light is 0.63 of Tamsin's rather than 0.16 and the Shoals' 0.55, still readable on a Pi's screen).
- **The near planet**: the body with the largest angular radius goes into the sky program's planet slot (as today:
  the sphere lit by the star, its look from the system file: terran, or banded for a gas giant with bands in place of
  land and cloud).
- **Far bodies**: up to three more (the next largest above 0.05 degrees) as lit discs: direction, angular radius,
  colour; a body under 0.05 degrees is a star-like point.
- **Stations** inside 200 km as meshes (section 3) at their positions; beyond, a point with its beacon.

The sky program gains the banded look and three far-body slots (12 floats); the per-pixel cost is three dot products
and a disc test.

### 11.3 One sun for everything outside

The star's direction of the frame (11.2; in phase 1 `exterior.json`'s `sun.dir`, a system direction) lights:

- the sky's disc and glow and the planet's day side (as today);
- the Tern's hull in open space (inside a station's sealed bay it is lit by the bay's baked probes instead, giving way
  to the star over the throat: spacedock 8a, phase 3; as built it takes the star even when berthed, which is wrong):
  the deck program divides the bake's light out and multiplies the frame's in, per vertex:
  `light(s) = a + (1 - a) max(0, n . s)`, the colour times `light(s_now) / light(s_bake)` with `a` = 0.38 and `s_bake`
  from `data/ships/exteriors.json` (the values the bake used) and `s_now` the star in the ship's frame (the attitude's
  inverse applied to the system direction). The glowing parts (lights, screens) are not relit.
- the Hound, the Jackals, the stations and the relay: one ambient cube from the star, `0.2 + 1.3 max(0, a . s)` on the
  axis `a` with the shade side's fill, in one function (`sc-client/src/outside.rs`).

### 11.4 Motion: motes (phase 2)

200 motes in a 300 m cube round the eye, wrapping, each a short line along the Tern's velocity relative to the local
reference (the station, the waypoint's body), its length the distance covered in 1/30 s, fading beyond 150 m. At
4 m/s backing out they hang; at 220 m/s they stream. One draw of 400 triangles, built once, moved by a uniform.

### 11.5 The jump's streaks (phase 2)

400 thin quads on a sphere round the eye, seeded, each a star's direction, stretched along the jump vector by a
uniform `k`: 0 until the spool's last 3 s, rising to 1 at the jump's tick, a white flash over the whole window for
0.3 s (the sky's colour lifted), then falling to 0 over 1 s as the new sky comes in. Drawn after the sky in the same
pass, so every window and the feed carry it. One draw of 800 triangles, during the 4.3 s only.

### 11.6 The belt's far rocks (phase 2)

At the Shoals, 40 rocks of three low-poly shapes (about 80 triangles each, seeded sizes 50-400 m, slow tumbles) 30-60
km from the relay, never within 5 km of the fight, lit by the star; instanced, one draw. They are scenery: FN 11's
collisions do not need them.

### 11.7 The viewscreens

The feed is `World::draw` from the captain's camera (CD 9.1), so it shows the same sky, bodies, stations, motes,
streaks and fight; it gains nothing of its own. Docked, the bow camera's view is the cradle's forward ring and the
habitat ring (the cradle drawn in the feed as in the walk's bow camera today).

## 12. Built in phase 1 (the client's exterior drawing)

| Item | Where | Check |
| --- | --- | --- |
| The hull relit by the sky's sun (11.3, finding 1): a `relight` uniform in the deck program (`s_bake`, `a`), the frame's sun in `flash`; the hull's textured parts drawn with it, in the walk and the drill | `crates/sc-render/shaders/deck.glsl`, `sc-render` `DeckParams`, `sc-client/src/hull.rs`, `outside.rs` | A test that the shipped hull's colours are the bake's formula with `exteriors.json`'s numbers (validate the real artifact); a test that the relight is 1 when the two suns agree; captures from the bridge and the deck B windows |
| One lighting rule outside (11.3, finding 2) | `sc-client/src/outside.rs`, `drill.rs` | A test that the cube's sunward face is the brightest and the opposite face gets the fill |
| The relay at the Shoals drawn at `waypoint_m` (finding 3), from `ships3d::station`; the starbase placeholder generated and checked (replaced by Tamsin Station's model, 12b) | `sc-client/src/ships3d.rs`, `outside.rs`, `drill.rs` | Triangle budgets, the vertex format and the winding of both placeholders in tests; the beacons' least size in a test. Not caught on screen yet: 18.6 km off it is a few pixels and its beacon is lit 0.15 s a second |
| The beacons' blink from data | `data/space/exterior.json` `beacons` | Validated with the file |
| Windows on other decks (finding 7) | | The ready room's window in the captures; deck B's seen through the medbay's door in a run |

## 12a. Built in phase 2 (2026-10-14)

| Item | Where | Check |
| --- | --- | --- |
| The system file and its loader: bodies, stations with a berth's pose and departure path, points with arrival points, mass locks, the sky from any point | `data/space/systems/shoals.json`, `crates/sc-core/src/system.rs` | Tests: the berth shows the dock view's sun and planet within 1 degree, Tamsin 17.5 degrees, Orrin 16.9 at the beacon, every arrival clear of every lock, the star dimmer at the Shoals |
| The jump drive's data | `data/ships/tern/jump.json` | Validated |
| The patrol: phases, shore, the drive's breaker, the clamps, the departure flown under the berth's limits, the spool, alignment, mass locks, the cooldown, the sweep, the encounter placed from the arrival, the jump home, the outcome | `crates/sc-core/src/combat/patrol.rs`, `combat/mod.rs` | Tests: clamped in the berth, clamps refused until the drive is ready, the departure 107 m out at no more than 4 m/s in 30-45 s, the station's lock, a crew of automation from the berth to the Hound, the patrol going on after the fight and ending home |
| The bots' and automation's patrol (one rule, `patrol::decide`) and their lines through crew chat | `patrol.rs`, `sc-net/src/bot.rs`, `data/crew/chat.json` | Tests on each decision; a server test with four bot clients that undock, jump to the Pell beacon, sweep it and say "Off shore power", "Helm, take us out", "Releasing the clamps", "Spooling the jump drive", "Sweeping" |
| The objectives `undock`, `sweep`, `return` | `combat/data.rs`, the OBJECTIVES tab | A test on each |
| The protocol: `Phase::Patrol`, the patrol block, the commands, `PROTOCOL` 15 | `sc-net/src/msg.rs` | Round-trip tests; a patrol's snapshot fits one datagram with the grid or the routing |
| The client: the mission by `--mission` (a server's welcome loads its own), the sky from the system file (the star's direction, size and light, the near planet's look), the starbase and relay placeholders where the system has them, the berth's cradle (the deck's `outside` rooms) carried with the berth as the Tern leaves it (both replaced by 12b) | `crates/sc-client/src/drill.rs`, `drill_bridge.rs`, `outside.rs` | A test of the sky at the Pell beacon; captures |
| The playtest's patrol script: out of the bridge's and the mess's windows as the patrol goes on, then to work | `crates/sc-client/src/playtest.rs` | A test holds its windows to the layout's patches |
| The new console controls as a mockup for approval, not built | `docs/mockups/patrol-consoles.html` | Shots |

Measured on the Pi 5 (release, single player, NPC crew on foot): the drive ready and UNDOCK at 0:33, clear of the
berth at 1:12 (39 s with the clamps, design 4.2's 39), the spool from 1:41 and the jump at 2:21 (40 s), the sweep
20 s, the second jump at 4:02, the Hound engaged at once. A crew of automation in a test: clear of the berth after
40.5 s and the first jump 59.6 s after that.

Not built yet: the jump's streaks and flash, motes, far bodies' discs and a banded gas giant in the sky program (Orrin
is drawn with the terran look's fields in its own colours), the belt's far rocks, the station drawn as a point beyond
200 km, the console controls (awaiting approval), docking again at the end, the weapons' 5 km safe rule.

## 12b. The stations' models in the engine (2026-10-15)

Written before the code (CLAUDE.md 4); spacedock tasks 2.3 (E first), 2.5 (the docked ships, in part) and, as a
stand-in, 2.4.

**What is drawn.** `crates/sc-client/src/stations.rs` generates every station of the system file at load, from
`data/space/spacedock.json`'s sizes (read through `sc_core::spacedock`, the same reading the berths and departure legs
are checked against), with what only the drawing needs in `data/space/station_models.json` (the texture layers, the
light, the bay's lamps, the paints, the tessellation, the draw distance):

| Station | Drawn as |
| --- | --- |
| Tamsin Station (E) | The pair: both cylinders' shells with the macro map once round them (its panes glowing, standing in for the see-through windows), both domes with the cap map, the seal collars and bearings, three mirrors a cylinder opened 35 degrees toward the star (the mirror layer on their faces), the farm rings with their pods and spokes, the rods across the pair. The port: the hub standing out of the sunward dome, the mouth's face, the lit throat (its rings and eight strips), the iris's rim (the iris stands open: its leaves are tasks 2.6), the round bay with its ribs, flood rings, back wall and its window tiers, the axial post, and a pier, pad and gantry at every berth; the guide rings on the dome. The Tern's own gantry stands swung back to its upright so the way up off the wall is clear (the first two legs) |
| The docked ships | At E's slots whose fleet entry is a Tern livery (the sister Heron, the rescue Heron), drawn with the Tern's own hull model (one livery: the deck file carries one); the Osprey and the Federation concepts have no engine model yet, so their berths stand empty (tasks 2.5) |
| Cinder Yards (C), Orrin Highport (A), Pell Wheel (B), Shoals Hollow (D) | **Placeholders**, flagged in code and printed at load: each outside's silhouette from its sizes (A's cap and ringed spine, B's wheel, hub and spokes, C's ribbed octagonal hall, D's rough rock and gate plate), doors shut and drawn as dark panels, no bay, no berths |

**The light: spacedock 5b's fallback.** The engine keeps CLAUDE.md 9 (no per-pixel lighting) until tasks 2.7 measures
the per-pixel shaders on the Pi. A station does not turn against its star, so its light is baked per vertex when it is
generated: outside, `ambient + earthshine max(0, n . planet) + star max(0, n . sun)` with the star's and Tamsin's ways in
the station's frame from the system file; inside the bay and the throat, the bay's ambient and its lamps (flood rings,
axis lamps, a lamp under each gantry, the throat's rings), each `strength (1 - d / reach)^2 max(0, n . l)` with no
shadow, a stand-in for tasks 2.4's offline bake with the fixtures in data. The Material Maker textures carry their
relief in their albedo; the detail layer tiles on the hub, the bay and the gear (its paint in the vertex colour). One
texture array of four 512 px layers: the hull detail, the cap, the mirror, and the macro map last, whose alpha (the
panes) glows (the deck program's emission mask). A ship under the roof (E's bay or throat) is lit from its own up (the
floods and gantries over it), the players' Tern's hull included; outside, by the star as before.

**Depth.** The outside is drawn behind the bridge in a slice of the depth buffer. With 1 m to 80 km in the last 0.0001
of it, a surface 2 km off resolved to about 2 km, so a station's own parts would fight. The bridge's own pass moves to
the front half of the buffer and the outside takes the back half (`drill_bridge`'s `in_front_half` and
`behind_everything`, held by a test): the bridge's 40 m resolves to under a centimetre, the outside's 200 m to 5 mm and
2 km to under half a metre. Glowing rings stand at least 1.5 m off the wall they light.

**The girder cradle is gone from the drill**, with its `outside` rooms; the walk's dock in `exterior.json` stays for
the tour. Stations are drawn within `draw_m` (80 km, the outside pass's far plane); beyond, nothing yet (a point, later).

**The Pi 5 budget** (against `engine-stack` 5; counted by the tests):

| Cost | Tamsin Station | A placeholder | Against |
| --- | --- | --- | --- |
| Triangles | 18,264 (outside 6,756, inside 8,868, glow 2,640; a test holds it under 25,000), and the two docked Herons at the Tern's model each | 268 (C) to 2,288 (A); the five together 22,303 | 200,000 a frame |
| Draw calls | 3 (outside, inside, glow), the Herons' 7 each, its 8 beacons while lit (0.15 s a second) | 1, and 2 beacons while lit | 300 a frame |
| Vertex memory | About 1.6 MB (28-byte vertices, not indexed) | under 0.2 MB | 64 MB buffers |
| Texture memory | 5.6 MB: four 512 px layers with mips, shared by all five | | 96 MB |
| Load | The generation, the bay's bake (about 200 lamps over 27,000 vertices) and four PNGs decoded and resized, once at startup | | |
| Server, network | None: the stations are fixed scenery | | |

Inside the bay the outside's mesh is drawn too (there is no portal cull of the station yet): its 6,756 triangles are
the cost of not having one. The first two sister ships are the Tern's full model (its middle level of detail is not in the deck file yet).

## 12c. Phase 3: the clearing route (designed 2026-10-15, not built)

What turns the built two-place patrol into the three-stop route of section 1, each in the existing paths:

| Item | Where | Check |
| --- | --- | --- |
| Stops in the route: each `route` entry names its place, its kind (scan, battle or civilian) and, for a battle, its hostile group; the mission's `groups` hold who is there (ships by class, fighters by count, placed from the arrival, hidden until swept or not); the single `encounter` field goes | `combat/patrol.rs` (`PatrolBlock`, `PatrolData::resolve`), `data/missions/patrol-shoals.json`, `combat/data.rs` | Validation: every stop's place and group exist; a group has a ship or fighters; tests on the resolve |
| Cinder's ore lanes in the system file, with its arrival point | `data/space/systems/shoals.json` | `sc-core::system`'s checks (arrival 5 km clear of every body, outside Cinder's lock and the yards') |
| A fighter pack with no carrier: Jackals placed from the arrival and stepped by `fighters.rs`'s same AI, their launch skipped | `combat/fighters.rs` | Tests: a pack of three attacks the Tern; the cannons kill a Jackal at EF 2's numbers |
| The fight re-armed at each stop: Engage started with the stop's group at its arrival, the Tern's damage, fires, missiles, power state and crew kept; bolts, bursts and debris cleared | `combat/mod.rs`, `patrol.rs` | Tests: a Tern damaged at stop 1 arrives at stop 2 with the same integrity and missiles; a fire burning at the jump still burns |
| A hidden group found by the sweep | `patrol.rs` | Test: the pack is not in the snapshot's contacts until the sweep at Cinder completes |
| `clear`: the stop's objective, done when its group is destroyed and a sweep shows none; moving on uncleared; the outcome PATROL COMPLETE or INCOMPLETE naming the stops | `combat/data.rs`, `patrol.rs`, the end screen | Tests on each; a crew of automation clears all three stops and returns |
| The raider: `missions`' Grabon raider class when it is built; until then `relay_raider` is the Hound with four Jackals | `data/enemies.json` (from `missions`) | The group validates against the classes present |
| The bots' patrol: sweep on arrival, fight, sweep after the last kill, report clear, wait for the fires to be out before the next jump; their lines (section 9) | `patrol::decide`, `sc-net/src/bot.rs`, `data/crew/chat.json` | A server test with bot clients clearing Cinder and jumping to Pell |
| The briefing's route on the system map and the OBJECTIVES tab's three `clear` rows | the briefing, `objectives-tab` | Captures |
| The scan stops: SCAN of a body (`scan_s` 30, the range and attitude rule) and the sweep at the Shoals' edge with its trail (5.3a) | `patrol.rs`, the science console (SCAN aimed at a body: a mockup first) | Tests: a scan out of range or off attitude does not complete; the trail's report at the edge |
| The civilian stop (5.3b): the *Marrow Lark* (a civilian hull, no weapons, its own placeholder mesh), its hail, the alongside rule, the fuel coupling interactable at the docking collar (hold E 2 s), engineering's FUEL, the transfer from the reactor's store, the line parting | `patrol.rs`, `ship-interactables` (the coupling), `power-grid`'s fuel store, `ships3d` (the hauler's placeholder), the mission's `civilian` block | Tests: the transfer runs only alongside and connected; the line parts past 100 m or 1 m/s; the Tern's fuel is 0.25 kg lower after; the objective |
| The windows at Cinder (11.1): the dark disc, the yards' lights | `outside.rs`, the system file | A test of the sky at the lanes; captures |
| The hull in the bay lit by the bay's probes, the star's light coming in over the throat (11.3, spacedock 8a) | `outside.rs`, `hull.rs` | A test: berthed, the hull's light is the probes', not the star's; at the mouth it is the star's; captures through the starboard windows while berthed |

The console controls of 4.1 (SHORE, DRIVE, UNDOCK, JUMP, SWEEP) are unchanged by phase 3; they still await the owner's
approval in `docs/mockups/patrol-consoles.html`. Phase 3 adds two controls, each to its mockup first: science's SCAN aimed at a body
(the sweep's control with a body chosen on the plot) and engineering's FUEL (the transfer's rate and stop). Clearing a
stop is the fight, and the next jump is the existing JUMP.

## 13. The Pi 5 budget

Against `engine-stack` 5.

| Cost | Phase 1 (built) | Phase 2 | Against |
| --- | --- | --- | --- |
| Triangles, worst frame | Relay 136 (in the bridge pass and the feed); phase 3: at most three Jackals or a raider and two Jackals at once, no more than the drill's Hound and two Jackals plus one Jackal's few hundred triangles | Tamsin Station 18,264 and two sister ships while near (design 12b); motes 400; streaks 800 for 4.3 s; rocks 3,200 at the Shoals: under 33,000 in a pass | 200,000 a frame |
| Draw calls | Relay 2 a pass, and 2 more while its beacons are lit | Tamsin Station 3, its ships about 14, its beacons 8 while lit, motes 1, streaks 1, rocks 1: under 30 a pass, two passes | 300 a frame |
| Vertex work | The hull's relight: two dot products and a divide a vertex, 8,730 vertices | | |
| Fragment work | None | The sky's three far-body slots and bands: a few ALU a pixel | 3x average overdraw |
| Memory | Relay meshes 11 KB (28-byte vertices) | The system file, a few kB; the stations' meshes about 1.9 MB and their texture array 5.6 MB (design 12b); 40 rock instances | 64 MB buffers |
| Server | None | The patrol's phase machine, under 0.01 ms a tick; no new bodies integrated (stations are fixed); phase 3: at most one more fighter than the drill steps today | 2 ms a ship a tick |
| Network | None | 6 bytes a snapshot; one reliable event a jump | 80 kbit/s down |

A desktop or the Pi's own desktop session frame time says nothing about the Pi's game (CLAUDE.md 2); the Pi measures
the built change in a release build.

## 14. Data

| File | Holds |
| --- | --- |
| `data/space/systems/shoals.json` (built) | The bodies, their looks, the stations and their berths, the points of interest and their arrival points; the star's `light_exponent` and `d_ref` |
| `data/missions/patrol-shoals.json` (built; phase 3 turns `route` into stops and adds `groups`) | The drill's fields (stations, orders, objectives, `enemy`, `enemy_start_m`, `waypoint_m`, countdown, limits) and the `patrol` block: `system`, `station`, `berth`, `route`, `encounter`, `clamps_s` 3, `drive_ready_min` 0.5, `sweep_s` 20, `sweep_range_m` 50,000, and a crew's manner (`clear_mps` 150, `loiter_mps` 40, `resend_s` 2). The berth's departure limits (1 m/s for 10 m, then 4 m/s to 107 m) are its path's legs in the system file, so any station's shape keeps them |
| `data/space/exterior.json` | Phase 1: `beacons` (`period_s` 1.0, `on_s` 0.15, `min_radius_deg` 0.15; the colours are the placeholder generator's, as the Hound's are). Phase 2: `sun.dir`, `planet.dir` and `planet.radius_deg` move into the system file and the berth's attitude; the looks stay |
| `data/ships/exteriors.json` | Read, not changed: `sun_dir_ship` and `ambient_fraction`, the hull bake's light |

## 15. Risks and trade-offs

- **The jump makes the system small** (FN's own risk). Mitigation: the departure and the waypoints are flown on
  impulse with something to see, and the spool is everyone's job.
- **The placeholder stations set an expectation.** Versions A-D are named placeholders in code, in the log at load and
  in this design (12b), and replaced by their generators (spacedock tasks 2.3).
- **A softened inverse square is not physics.** It keeps a far waypoint lit on a Pi's screen; the data names it.
- **Phase 2 touches combat's frame.** The encounter keeps the drill's numbers by placing the Hound from the arrival as
  the drill does from the start; the drill's tests stay and gain one at a far origin.

## Open questions

None for the owner yet; recommendations taken (ask only with screenshots, CLAUDE.md 13):

| # | Question | Taken |
| --- | --- | --- |
| P1 | Jump or cruise between waypoints | The jump: FN 7 and its N1 |
| P2 | How the Tern leaves the berth | Stern first out of the cradle's open end on the bow thrusters, on the berth's limits |
| P3 | Dock again at the end | Not yet: the mission ends at the arrival home; docking is FN 10's autopilot, a later change |
| P4 | How far the starbase sits from Tamsin | 3.33 radii, outside the jump's mass lock, which halves the planet in the dock's port windows (section 2) |
| P5 | Which planets the patrol visits, and in what order (2026-10-15) | Cinder (a scan, then a fight), Orrin (a rescue at its rings, then the Pell beacon's fight), the Shoals (a scan at its edge, then the relay's fight), then home: the owner's mix of scans, a rescue and three battles, the fights in rising strength (section 1); Tamsin is home |
| P10 | How fuel goes to the stranded hauler | Helm holds alongside, a crew member walks to a fuel coupling at the docking collar and connects the line, engineering pumps 0.25 kg of the reactor's fuel; the line parts if helm drifts (5.3b) |
| P6 | What "clear" means | Every hostile of the stop's group destroyed, confirmed by a sweep that shows none within 50 km |
| P7 | Can the crew leave a stop uncleared | Yes, once no hostile ship locks the jump; the objective stays open and the debrief names the stops cleared |
| P8 | Does the ship carry damage between stops, and can it resupply | Damage, fires and missiles carry over (a ship persists); no resupply mid-patrol: docking at Orrin Highport to rearm needs FN 10's docking, a later change |
| P9 | Can the captain choose the order of the stops | Not yet: the route is fixed in the mission; a choice on the system map comes with FN 9's map |
