# Design: patrol-mission

The owner, 2026-10-14: "have the ship undock from star base, go patrol a star system and encounter hostiles", and "the
view through the window should reflect whats going on around the ship".

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

`data/missions/patrol-shoals.json`, "Patrol: the Shoals".

| # | Phase | What the crew do, on foot | Ends | Typical time |
| --- | --- | --- | --- | --- |
| 1 | Muster | Join, claim stations, read the briefing, READY | Every held station ready | |
| 2 | Countdown | 5 s on every screen (CD 1) | Timer | 5 s |
| 3 | Docked | Walk from the muster rooms to the stations (CD 9). Engineering brings the reactor up, closes the drive's breaker and drops shore power (section 4.1). The captain orders UNDOCK | Helm releases the clamps | 60-90 s |
| 4 | Undocking | Helm backs the Tern out of the cradle on the bow thrusters, along the berth's axis, under the berth's limits (4.2) | The bow clears the cradle's end | 3 s + 36 s |
| 5 | Departing | Helm turns off the berth's axis and burns on impulse; tactical holds weapons cold inside 5 km of the station | Past the station's 2 km mass lock | 25 s |
| 6 | Spool | The captain orders the jump to the next waypoint; engineering finds the 20 MW; helm aligns and holds | The spool completes (FN 7) | 40 s |
| 7 | Waypoint: Pell beacon | Arrival 5 km from the beacon; science sweeps (20 s); the captain orders the next jump when the 60 s cooldown ends | The next spool completes | 60 s + 40 s |
| 8 | Encounter: the Shoals | Arrival 18.6 km from the relay; the Hound is there with its fighters; red alert; the drill's Engage runs as today (CD 1, 2) | The Hound destroyed, the Tern lost, or 600 s | 4-6 min |
| 9 | Home | Clear of the Hound's 5 km lock, the captain orders the jump home; helm aligns; the spool | Arrival at Tamsin Station's arrival point, 5 km out | 40 s |
| 10 | Debrief | The end screen (M 4): MISSION COMPLETE, ESCAPED or THE TERN IS LOST | 20 s, up to 240 s for damage control (CD) | |

About 11 minutes for a crew that knows the ship. Docking again at the end is not in this change: it is FN 10's dock
autopilot, a later change; the mission ends on arrival home.

**Why the fight is unchanged.** The encounter starts the drill's Engage with the Hound placed from the Tern's arrival
exactly as the drill places it from the Tern's start (`enemy_start_m` in the Tern's frame), so every number the drill
was tuned with (the 6 km start, the 1,800 m orbit, the fighters at 20 s, the 600 s limit) holds. Only the frame moves:
section 5.3.

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

Each point of interest has an `arrival_m`: the Pell beacon's 5 km sunward of it; the relay's at the drill's offset,
the relay standing at `waypoint_m` from it in the Tern's arrival frame, so the drill's geometry holds; Tamsin
Station's 5 km out along its berth arms' axis.

What a window sees follows from these: the star's disc is 0.53 degrees across at Tamsin, 0.21 at Orrin and 0.14 at
the Shoals; Tamsin from the station is a disc of 17.5 degrees radius (`asin(6.0 / 20.0)`), Orrin from the beacon one of
16.9 degrees (`asin(5.8e7 / 2.0e8)`), Pell from the beacon one of 1.6 degrees.

**The dock's view follows.** `exterior.json`'s `sun.dir` and `planet.dir`, `planet.radius_deg` describe the view from
the berth today, by hand (the planet 34 degrees in radius). In phase 2 they are derived from the system file and the
berth's attitude instead (section 11.2), and Tamsin shrinks to 17.5 degrees in radius in the port windows: the
station has to sit outside Tamsin's 3-radius mass lock to jump from it, and a 34 degree disc would need it at 1.8
radii. The berth's attitude is the one that keeps the star and Tamsin where the dock's view has them (the star high to
starboard and ahead, Tamsin low to port), computed by a tool from the two directions and checked, never typed in
(CLAUDE.md 6.6).

## 3. The starbase and the relay, placeholders

The starbase model is being made on another machine (`starbase` change, `assets/models/exteriors/starbase.glb`). Until
it arrives the client generates a placeholder, as it does the Hound (`sc-client/src/ships3d.rs`, CLAUDE.md 9: a
committed generator), named for what it is: `ships3d::station(StationKind::Starbase)`.

| Part | Shape | Size |
| --- | --- | --- |
| Spindle | An octagonal prism along the station's axis, with a cap at each end | 60 m across, 260 m long |
| Habitat ring | 16 box segments round the spindle at its middle, lit window bands on the outer face | 600 m radius to the ring's middle, a 40 x 24 m section |
| Spokes | 4 square tubes from the spindle to the ring | 8 m |
| Berth arms | 2 trusses along the axis out of the spindle's ends, each ending in a cradle: the dock frame of `exterior.json`, its length across the axis | 440 m long: a cradle's centre 570 m from the station's centre |
| Lights | Window bands (warm white, glowing), red and green beacons at the ring's poles and the arms' tips, blinking at 1 Hz on for 0.15 s, white floodlights at the cradles | |

About 1,200 triangles and two draws (the albedo lit by the sun, the glowing parts flat). The relay is the same
generator's `StationKind::Relay`: a 120 m truss with three dishes, a solar wing and one beacon, about 300 triangles.
When `starbase.glb` lands, its full model replaces the placeholder at the berth (inside 20 km) and its far LOD beyond,
with the berth's cradle taken from the model if it has one; the placeholder's generator stays for the relay.

The Tern's berth is the cradle at one arm's end, the station's axis along the Tern's x, so the spindle and the ring stand off the Tern's starboard side. The Tern lies in it as it lies in the walk's
dock today: the cradle is `exterior.json`'s `dock` frame (120 m long, ring frames every 20 m, the work lights), now
belonging to the station: while docked it is drawn in the Tern's frame as today; once the clamps release it stays
with the station, drawn in the system frame at the berth's pose, and the Tern moves out of it. A gangway from the
cradle's starboard girders meets the docking collar (x -10.8 m, deck C); it retracts when the clamps release.

## 4. Undocking

### 4.1 Docked

- **Shore power.** Docked, the Tern draws from the station through the umbilical at up to 12 MW (`shore_mw`), enough
  for life support, lighting and the consoles; the reactor is at hot standby, 10% (`reactor_standby_frac`), and the
  impulse drive's breaker is open (the berth's rule: no drive while clamped).
- **Engineering's job** (on foot: the engine room's console, or the bridge's engineering station): raise the reactor
  to RUN (it ramps at PG's 0.02 of rated a second, 17 s from 10% to the 44% cruise level), close the drive's breaker
  (the drive comes to its 0.2 MW standby), and open SHORE when the reactor carries the load. The console's DRIVE light
  goes green when all three are done; the drive and the RCS cannot be given more than standby until then.
- **The captain's job**: the briefing's route on the system map (FN 9), and the order UNDOCK once DRIVE is green.
- **Tactical**: weapons are locked inside 5 km of the station (`weapons_safe_m`, the station's rule); the console shows
  SAFE in the turret modes' place until then.
- **Science**: the station's traffic on the local plot; nothing to do yet.

### 4.2 The clamps and backing out

- **UNDOCK** is Helm's, a guarded control held 0.6 s (BS 8.5, as M 3's JUMP), live only when DRIVE is green and the
  captain has ordered it (or, with no captain aboard, at once). The clamps open in 3 s (`clamps_s`); the gangway
  retracts with them.
- **Backing out**: the cradle is open at its aft end; the Tern backs out stern first along the berth's axis (the
  ship's -Z) on its bow thrusters (FN 2: 15 MN, 5 m/s^2), under the berth's departure limits: at most 1 m/s for the
  first 10 m (`depart_slow_mps`, `depart_slow_m`) and 4 m/s to the cradle's end (`depart_mps`). The bow must clear the
  cradle's aft ring: the cradle reaches 60 m aft of its centre and the Tern's bow 37.3 m ahead of it (the bow camera
  sits 5.7 m beyond the bow at z 43 m), so 97.3 m and a 10 m margin, 107 m. Time: 10 s for the first 10 m, then 97 m
  at 4 m/s with the speed changes, about 26 s: **36 s**.
- **Lateral and attitude**: the assist holds the berth's axis (lateral set points zero, attitude held), so the hull
  clears the cradle's girders by their 2.9 m (`tern-exterior` 4). A contact (helm off the axis by more than 2.9 m, or
  rolling) is FN 11's collision with the cradle, a kinetic hit to both.
- **Automation** (no helm aboard): backs out at half the limits (72 s).

### 4.3 Departing

Clear of the cradle, the Tern is 580 m from the station's centre (the cradle's 570 m and the 107 m backed out across
it). The jump needs 2 km (FN 7's station mass lock). Helm turns away from the station (at most 180 degrees: 11.5 s at
the drill's 18 deg/s and 12 deg/s^2), and burns: 1,420 m at 15 m/s^2 takes 13.8 s and leaves the Tern at 206 m/s.
About **25 s**. The turn toward the jump vector can be made during the spool: the alignment is asked
only for the spool's last 3 s.

### 4.4 Clamps to the first jump

3 s + 36 s + 25 s + 40 s spool: **about 104 s** from the clamps to the first jump, after the minute or so of walking
and starting the drive.

## 5. The route and the jumps

### 5.1 Jump, not cruise

FN 7 already chose the jump drive over a cruise drive (its table: every station has a job in the 40 s spool, nothing
moves fast near anything, one rebase at a tick, one network event; a cruise drive at 0.01 c takes minutes per AU and
moves a pose thousands of kilometres a tick). This change takes that choice and its numbers: 800 MJ at 20 MW, 40 s; the
alignment; the mass locks; the arrival; the 60 s cooldown. On impulse the Tern flies at up to 400 m/s (the assist's
cap, `data/ships/tern/flight.json`) inside a waypoint's few kilometres.

| Leg | From | To | Distance | Spool | Transit |
| --- | --- | --- | --- | --- | --- |
| 1 | Tamsin Station, past 2 km | Pell beacon | 2.0e11 m | 40 s | Instant at one tick |
| 2 | Pell beacon, after the 60 s cooldown | Shoals relay | 2.3e11 m | 40 s | Instant |
| 3 | The Shoals, clear of the Hound's 5 km lock | Tamsin Station | 3.3e11 m | 40 s | Instant |

### 5.2 At a waypoint

- **Arrival** at the point's `arrival_m`, scattered within 1.5 km (seeded, CLAUDE.md 6.4), the velocity kept relative
  to the reference (FN 7: the Tern arrives moving as it left).
- **Science's sweep**: SWEEP on the science console (seated, on foot) takes 20 s (`sweep_s`) and reports the contacts
  within 50 km; at the Pell beacon it finds nothing, and the objective "Sweep the Pell beacon" is done.
- **The next order**: the captain's JUMP to the next waypoint, live once the 60 s cooldown has run.

### 5.3 The encounter at the Shoals

- **Arrival**: 18.6 km from the relay, the relay at `waypoint_m` in the Tern's arrival frame, as the drill has it.
- **The Hound** is placed at `enemy_start_m` from the Tern's arrival in the Tern's frame (6 km ahead, as the drill), its
  orientation toward the Tern; the drill's Engage starts at once (no second countdown: the crew are already at their
  stations); the fighters launch 20 s in (EF 1). Science's sweep is automatic on arrival here: the Hound is in the
  open.
- **The frame**: combat already runs in `f64` in a system frame (CD 2); its positions become the Shoals system's,
  about 4.5e11 m from the origin, where an `f64` keeps 0.06 mm. Every draw subtracts the eye before narrowing to `f32`
  (CLAUDE.md 7), so the client is unchanged; `combat`'s "within 20 km" checks become "within 20 km of the arrival".
- **Ending**: the Hound destroyed, the Tern lost, or the Hound withdrawing at 600 s (CD). The Hound locks the jump
  inside 5 km (FN 7) while it lives; destroyed or withdrawn, the lock is gone.

### 5.4 Home

The captain orders the jump home; helm aligns; the spool (with what engineering can free after the fight: a damaged
reactor stretches it, `warp-pylons` 3 a damaged pylon). Arrival at Tamsin Station's arrival point, 5 km out, ends the
mission: MISSION COMPLETE when the Hound was destroyed, ESCAPED when it withdrew (M 3, 4).

## 6. Objectives

`objectives-tab` 2's kinds, with three new ones:

| Words | Kind |
| --- | --- |
| Undock from Tamsin Station | `undock` (new): done when the bow clears the cradle |
| Sweep the Pell beacon | `sweep` (new): done when science's sweep completes within 20 km of the point |
| Destroy the Hound | `destroy` |
| Shoot down both fighters | `destroy_fighters`, 2 |
| Keep the Tern's hull above zero | `survive` |
| Return to Tamsin Station | `return` (new): done on arrival home |

## 7. Commands and state (phase 2)

All through the existing validation and apply path (CD 3: a person, a bot and automation differ only in what they
decide). Each new control goes into `docs/mockups/consoles.html` for the owner's approval before it is built
(CLAUDE.md 10).

| Command | From | Does |
| --- | --- | --- |
| `Reactor(Run)` | Engineering | Raises the reactor from standby (PG's ramp) |
| `Breaker(drive, closed)` | Engineering | The existing breaker command on the drive's breaker |
| `Shore(bool)` | Engineering | Takes or drops shore power; dropping it is refused while the reactor cannot carry the load |
| `Undock` | Helm, held 0.6 s | Opens the clamps when DRIVE is green and the order is given |
| `Jump(poi)` / `JumpAbort` | Helm, held 0.6 s (M 3) | Spools toward the point of interest the captain ordered |
| `Sweep` | Science | The 20 s sweep |
| `Order(Undock)`, `Order(Jump(poi))`, `Order(Home)` | Captain | The captain's order verbs, shown on every console's title band |

The snapshot gains a `patrol` block of 6 bytes: the phase, the waypoint's index, the clamps' state, the spool's
fraction (0-255), the sweep's fraction and the jump's tick parity (the client times the streaks from it). The Tern's
pose is already `f64` in the system frame. A jump is one reliable event (FN 12).

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

| Station | Docked and undocking | Departing and the jumps | At Pell | At the Shoals | Home |
| --- | --- | --- | --- | --- | --- |
| Engineering | Reactor to RUN, the drive's breaker, SHORE off. "Reactor coming up", "Drive on line", "Off shore power" | Frees 20 MW for the spool. "Twenty megawatts to the jump drive" | | As the drill. "Rerouting round the damage" | "Spool power ready" |
| Helm | Waits for the order; UNDOCK; backs out on the limits. "Clamps released", "Clear of the berth" | Burns clear of the lock, aligns, holds. "Clear of the station", "Aligned", "Jumping" | "Holding at the beacon" | As the drill (`bot` profile: bow on at 1,500 m, weaving) | "Aligned for home" |
| Tactical | Weapons SAFE. "Weapons safe" | Past 5 km: "Weapons free" | | As the drill. "Locked on the Hound", "Missile away" | |
| Science | Traffic on the plot | | SWEEP. "Sweep clear" | "Contact: Hound class at the relay", "Two fighters launching" | |
| Captain | UNDOCK when DRIVE is green. "Undock" | "Jump to the Pell beacon" | After the cooldown: "Jump to the Shoals relay" | "Red alert", the drill's orders | "Take us home" |
| Laser banks, gunner | At their seats | | | As the drill | |

With no captain aboard the helm bot takes the order itself when DRIVE is green, and the next jump when the cooldown
ends, as FN 7 lets tactical order the jump when there is no captain.

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
9. **Not in this change, on purpose**: sunlight falling through a window onto the bridge's floor (the interior is
   baked, CLAUDE.md 9; a moving sunbeam is a dynamic light the Pi does not draw), the planet's shadow on the Tern (the
   route never passes behind Tamsin), and the Tern's own lights on the station.

## 11. The view through the windows: what each shows

Every window and the viewscreen draw the one exterior state of the frame (`ship-frames` 7: one ship pose a frame):
the snapshot's ships, craft, missiles, bolts and bursts, the system file's bodies and stations, and the Tern's pose in
the system frame. Nothing is a backdrop.

### 11.1 By phase

| Phase | Bridge windows (forward quarters) | Port side (ready room, quarters, medbay, life support, shield room) | Starboard side (briefing room, mess, damage control, cargo, switchboard) | Viewscreen (the feed) |
| --- | --- | --- | --- | --- |
| Docked | The cradle's forward rings and work lights round the bow; the star high to starboard and ahead; the habitat ring's arc beyond to starboard | Tamsin's lit disc low and large (17.5 degrees radius), the cradle's girders across it, the port pylon and nacelle lit by the star | The habitat ring face on, 1.2 km across and 570 m off, the berth arm, lit window bands, blinking beacons, the gangway at the collar (deck C) | The bow camera: the cradle's forward ring and space, the ring's arc |
| Undocking | The cradle's rings drawing away ahead as the Tern backs out, 20 m apart, at up to 4 m/s; the bow thrusters' puffs | The girders sliding forward past each window, then gone; Tamsin unchanged | The station's lights drawing back; the gangway retracted | The cradle receding ahead |
| Departing | The station turning out of view as helm turns; motes streaming aft | The station receding: 1.2 km across, 33 degrees at 2 km, 3.4 at 20 km; Tamsin; the star | The same from the other side | As the bridge |
| Spool | The bow swinging onto the jump vector; in the last 3 s the stars ahead stretch | As departing; the stars stretch toward the bow | The same | The stretch |
| Jump | Streaks from the bow's point through every window, a white flash of 0.3 s at the jump's tick, then the new sky settling in 1 s | Streaks running aft along the side windows | The same | The same streaks |
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
- the Tern's hull: the deck program divides the bake's light out and multiplies the frame's in, per vertex:
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
| The relay at the Shoals drawn at `waypoint_m` (finding 3), from `ships3d::station`; the starbase placeholder generated and checked, drawn from phase 2 | `sc-client/src/ships3d.rs`, `drill.rs` | Triangle budgets of both placeholders in tests; a capture with the relay |
| The beacons' blink from data | `data/space/exterior.json` `beacons` | Validated with the file |
| Windows on other decks (finding 7) | | Captures from the medbay and damage control |

## 13. The Pi 5 budget

Against `engine-stack` 5.

| Cost | Phase 1 (built) | Phase 2 | Against |
| --- | --- | --- | --- |
| Triangles, worst frame | Relay 300 (in the bridge pass and the feed) | Starbase 1,200 and the cradle's girders (already compiled, about 2,600) while near; motes 400; streaks 800 for 4.3 s; rocks 3,200 at the Shoals: under 8,000 in a pass | 200,000 a frame |
| Draw calls | Relay 2 a pass | Starbase 2, cradle about 12 (its bays, as the walk draws them), motes 1, streaks 1, rocks 1: under 20 a pass, two passes | 300 a frame |
| Vertex work | The hull's relight: two dot products and a divide a vertex, 8,730 vertices | | |
| Fragment work | None | The sky's three far-body slots and bands: a few ALU a pixel | 3x average overdraw |
| Memory | Relay mesh 8 KB, starbase 34 KB (28-byte vertices) | The system file, a few kB; 40 rock instances | 64 MB buffers |
| Server | None | The patrol's phase machine, under 0.01 ms a tick; no new bodies integrated (stations are fixed) | 2 ms a ship a tick |
| Network | None | 6 bytes a snapshot; one reliable event a jump | 80 kbit/s down |

A desktop or the Pi's own desktop session frame time says nothing about the Pi's game (CLAUDE.md 2); the Pi measures
the built change in a release build.

## 14. Data

| File | Holds |
| --- | --- |
| `data/space/systems/shoals.json` (phase 2) | The bodies, their looks, the stations and their berths, the points of interest and their arrival points; the star's `light_exponent` and `d_ref` |
| `data/missions/patrol-shoals.json` (phase 2) | The drill's fields (stations, orders by phase, objectives, `enemy`, `enemy_start_m`, `waypoint_m`, countdown, limits) and the patrol's: `system`, `berth`, `route`, `encounter`, `home`, `shore_mw` 12, `reactor_standby_frac` 0.10, `clamps_s` 3, `depart_slow_m` 10, `depart_slow_mps` 1, `depart_mps` 4, `weapons_safe_m` 5,000, `sweep_s` 20, `sweep_range_m` 50,000 |
| `data/space/exterior.json` | Phase 1: `beacons` (`period_s` 1.0, `on_s` 0.15, the colours). Phase 2: `sun.dir`, `planet.dir` and `planet.radius_deg` move into the system file and the berth's attitude; the looks stay |
| `data/ships/exteriors.json` | Read, not changed: `sun_dir_ship` and `ambient_fraction`, the hull bake's light |

## 15. Risks and trade-offs

- **The jump makes the system small** (FN's own risk). Mitigation: the departure and the waypoints are flown on
  impulse with something to see, and the spool is everyone's job.
- **The placeholder starbase sets an expectation.** It is named a placeholder in code, in the mockup and in the
  captures, and replaced by the model.
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
