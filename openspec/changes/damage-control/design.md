# Design: damage control

Status: **proposed** (2026-10-04). Nothing here is built. The data file
`data/ships/tern/damage.json` and the `fire` block of `data/ships/tern/atmosphere.json` are proposed
by this change; the systems mockup reads them. Every table of results below was computed by running
the proposed library `docs/mockups/lib/shipsystems.js` headless in node 22 on the proposed data
(scratch scripts, not committed): results of the design's formulas, not engine measurements. Where a
rule is designed but not in the mockup's library, it says so. Budgets are against `engine-stack`
section 5.

## Context

`weapons-and-shields` section 12 ends every weapon impact the shields do not stop with
`HullHit { ship, tick, point_m, normal, direction, energy_mj, kind, source }`: a point on the hull in
ship metres, the direction the energy travels, and the energy left after the shield face. Collisions
come straight here (`flight-and-navigation`). A Gannet carries 60 MJ, a Lance 40 MJ, a pulse bolt
1.2 MJ.

The Tern's layout puts systems in rooms (`systems[].compartment`) and `power-grid` routes its
conduits through named compartments with polylines, so a hit can find what lies near its path. Its
damage control room (POI 9) is on deck B forward, its board 20.8 s walking from the forward
switchboard and 39.1 s from the main switchboard by the aft passage, 30.9 s through the hangar
once T1 is applied (`reference-ship-tern` section 6 at `crew-on-deck`'s speeds; v2 plan,
2026-10-05, the board on the room's new outer wall, was 19.0 s, 37.3 s and 29.1 s; corrected
2026-10-04 from 20.7 s and 29.9 s at an assumed 1.6 m/s). The
engineering bay console (`eng_main`) and the damage control board (`damage_board`) are the two seats
this change works through (`bridge-stations` sections 10.8 and 10.9).

**From star-crew-64** (`docs/analysis/star-crew-64.md`):

| It did | Kept | Changed |
| --- | --- | --- |
| Shield face by the dominant axis; overflow to the hull | Yes (`weapons-and-shields`, with round caps fore and aft) | The overflow is a hull hit with a point and an energy |
| Damage routed to a station by a four-way angle test | | A ray from the hull point through the compartments it crosses |
| Station HP 50; 0 HP stops a station; damage passed to its occupant | Systems that stop when broken; people near them hurt | Integrity in percent, four states, capability by state; crew hurt by distance |
| A room ignites after three hits in 180 frames; fire never goes out alone; 2 HP/s to officers, 0.5 HP/s to the station | Fire persists until crew act or it starves | Fire from energy, fuel and oxygen; t-squared growth; out when starved; it heats, smokes and damages |
| Single-use extinguishers that clear the room you stand in (none placed in the shipped level) | Crew carry extinguishers | Six in a locker; each cuts a fire's heat release for 15 s; a big fire needs two or three, or a fixed system |
| Vent: clears the fire, 30 HP to everyone in the room, 600-frame cooldown | Venting trades crew safety for a clear room | The room really goes to vacuum (24 s to 20 kPa); crew inside die in 90 s at Armstrong's limit unless out or suited |
| Repair: hold Z, 5 HP/s times engineering's own HP share; power edits frozen; a cosmetic repair pulse | Repair is crew work | Done where the damage is, with kits and parts, at stated rates, previewed by the function that resolves it |
| Timings in frames | | Seconds (CLAUDE.md 6.4) |

## Goals / Non-Goals

**Goals:**
- One hit resolution for every ship and craft, deterministic, with seeded chance (CLAUDE.md 6.1,
  6.4).
- Damage that has a place and a cause the crew can follow: this breach, this cut conduit, this fire.
- Repairs that need people in the right place with the right things, at times the board can show
  before they start.
- Numbers from the same model as power and air, so a fire's oxygen, a breach's air and a switchboard's
  loss are one simulation.

**Non-Goals:**
- Structural failure of the hull as a whole (breaking the ship in two). A hull section can be
  stripped; the ship is lost when the mission's loss conditions say so.
- Visual destruction of geometry (`deck-pipeline`: damage changes state, not meshes).
- Boarding actions.

## Decisions

### 1. The hit resolution

`damage::resolve(ship, hull_hit) -> HitReport`, the one implementation, used by the server for every
ship and by the mockup:

1. **Find the hull section**: the span between two of the layout's hull sections that holds the
   point, and the face (dorsal, ventral, port, starboard, bow, stern) by the direction's dominant
   axis. The Tern has 9 spans by 6 faces: 54 sections.
2. **Armour**: the section absorbs `min(E, 4 MJ x integrity)` and loses `min(E, 4 MJ) / 40 MJ x 100`
   points of integrity. Ten full-armour hits strip a section.
3. **March**: the energy left goes on along the direction from the hull point in 0.25 m steps, for at
   most 30 m. Each step loses `dE = E (1 - exp(-0.25 / 4))` (a 4 m decay length) and deposits it at
   the step's point if the point is inside a compartment; crossing into a second compartment costs a
   bulkhead's 1.5 MJ.
4. **Breach**: the first compartment entered from outside gets a breach of area `0.02 m^2 per MJ x E`
   at its skin, between 0.002 m^2 (a bolt hole) and 2.0 m^2. It is a link to space for `life-support`.
5. **Damage near each step**: every load, power node, conduit segment and door within
   `r = 1.5 m + 0.3 m x sqrt(E)` of the step's point takes `10 points per MJ x dE x (1 - d / r)`.
   Conduits: 1 MJ or more deposited near one severs it; 0.3 MJ halves its capacity. Power nodes lose
   health as `points / 100` and are destroyed below 0.25. Doors that take 25 points jam where they
   are (not in the mockup's library).
6. **Fire**: in each compartment that received energy `E_c`, a fire starts with chance
   `min(0.6, 0.1 x E_c) x oxygen factor`, decided by a hash of (session seed, hit id, compartment
   id): the same hit always does the same thing. It starts at `50 kW + 100 kW x E_c`.
7. **Crew**: a crew member near the path loses `15 HP per MJ x 0.3 x` the energy deposited near them,
   with the same falloff. (The mockup has no crew positions and gives everyone in the room the room's
   total times the share of its floor within `r` of the path.)
8. **Report** to the consoles and the log, and the shake to `ship-frames`.

**What passes the armour**, with the hull at full integrity: a pulse bolt (1.2 MJ) never does until
its section's armour is worn below 30%; a Lance (40 MJ) passes 36 MJ; a Gannet (60 MJ) 56 MJ. Of that,
the void between the hull and the first room takes its share by distance: in walkthrough W1, 34.0 MJ
reached engineering's skin 2.0 m in (a replica of the march, rerun on layout v2 2026-10-05; on v1,
30.7 MJ, 2.4 m in).

### 2. System states

| State | Integrity | Capability (what `power-grid` multiplies demand by) |
| --- | --- | --- |
| Nominal | 75-100% | 1 |
| Damaged | 25-75% | integrity / 75 (0.33 to 1) |
| Disabled | 0-25% | 0 |
| Destroyed | 0% | 0, and needs a rebuild |

Integrity falls from hits (section 1), from overheating (`power-grid` section 10: 0.01% a second per
kelvin over 393 K), from overdrive wear, and from fire (section 4). Switchboard sections, panels and
the emergency bus have health 0-1 by the same rules and are destroyed below 0.25. The reactor's own
integrity scrams it below 25% (`power-grid` section 3). Turret, tube, shield and craft effects of a
damaged state are their changes'; they read this integrity.

### 3. Fire

**Ignition**: a hit (section 1), a room's air above 300 C (autoignition: spread), or a fault the
missions add. **Growth**: the t-squared "fast" class (`alpha = 0.047 kW/s^2`), so a fire seeded at
50 kW passes 1 MW about 113 s later, toward a ceiling of `250 kW/m^2 x floor x f_O2`, where `f_O2` is
the product of `clamp((x_O2 - 0.13) / 0.05, 0, 1)` and `clamp((P - 20 kPa) / 40 kPa, 0, 1)`: no fire
below 13% oxygen or 20 kPa. Above its ceiling it decays with a 10 s time constant; below 5 kW it is out.
**Fuel**: 150 MJ/m^2 of floor by default, 400 in the magazine, 300 in cargo, 250 in the quarters and
mess, 200 in the switchboard and computer core, 120 in engineering and the drive, 100 in the hangar.

**Chemistry** (`life-support` carries the gases): 0.419 MJ per mol of oxygen consumed (Huggett's
13.1 MJ/kg), 0.67 mol of CO2 per mol of oxygen, smoke at 0.02 mol per mol of oxygen in a ventilated
fire and 0.17 when the room is below 15% oxygen. The heat goes into the room's air.

**Damage to what is in the room** (proposed and in the library): above 150 C, every system and power
node in the room loses 0.01% of integrity a second for each kelvin over; the reactor a fifth of that.
At 300 C that is 1.5% a second.

**Spread**: through an open door by the hot two-way flow (`life-support` section 5), carrying heat and
smoke; to any room whose air passes 300 C. Through shut doors and bulkheads (5 W/(m^2 K)) a fire does
not spread within a mission: in every case run, no fire spread, including through a door held open
for 900 s (the corridor stayed below 300 C). Vent dampers shut on smoke above 2,000 ppm and on a
fire's expansion, so the duct does not carry it.

**Fires, from the simulation** (seeded at 50 kW unless stated, the room's door shut, one unsuited
crew member inside; times in seconds from ignition; rerun on layout v2 2026-10-05, whose rooms are
larger, with the harness unchanged):

| Case | 1 MW | Air 60 C | Air 300 C | Smoke 2,000 ppm | Crew impaired / unconscious / dead | Out | Peak MW | Peak air C | Peak kPa | Oxygen at end % |
| --- | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: | ---: |
| Quarters, door shut (default) | 113 | 118 | 262 | 238 | 96 / 181 / 193 | 321 | 3.71 | 307 | 195 | 12.6 |
| Quarters, door held open | 113 | 120 | 267 | 246 | 97 / 184 / 196 | burning at 900 | 3.95 | 327 | 167 | 13.0 |
| Quarters, one extinguisher at 30 s | | | | | | 33 | 0.18 | 24 | 102 | 20.8 |
| Quarters, one extinguisher at 60 s | | | | | | 67 | 0.40 | 31 | 104 | 20.6 |
| Quarters, one extinguisher at 90 s | | | | | | 102 | 0.71 | 45 | 109 | 20.3 |
| Quarters, one extinguisher at 120 s | 113 | 118 | 364 | 324 | 96 / 225 / 244 | 407 | 3.29 | 300 | 193 | 12.6 |
| Quarters, two extinguishers at 120 s | 113 | 118 | | | 96 / - / - | 129 | 1.10 | 65 | 116 | 19.7 |
| Quarters, two extinguishers at 180 s | 113 | 118 | | 327 | 96 / 181 / 193 | 415 | 2.55 | 291 | 190 | 12.7 |
| Quarters, three extinguishers at 180 s | 113 | 118 | | | 96 / 181 / - | 192 | 2.12 | 139 | 140 | 17.6 |
| Quarters, vented at 60 s | | | | | 66 / 92 / - | 132 | 0.63 | 30 | 104 | 18.7 |
| Medbay, door shut | 113 | 99 | 228 | 207 | 79 / 158 / 170 | 284 | 2.87 | 308 | 195 | 12.5 |
| Forward switchboard, inert gas at 30 s | | | | | 66 / - / - | 127 | 0.58 | 53 | 112 | 12.4 |
| Magazine, no action | 113 | 151 | 325 | 300 | 124 / 222 / 235 | 390 | 5.64 | 310 | 196 | 12.7 |
| Magazine, inert gas at 30 s | | | | | 63 / - / - | 131 | 0.63 | 33 | 105 | 12.5 |
| Engineering, 1 MW seed, water mist automatic | at once | | | | | 22 | 1.20 | 22 | 102 | 20.9 |
| Engineering, 1 MW seed, mist disabled | at once | 173 | 467 | 425 | 130 / 265 / 283 | 576 | 15.97 | 310 | 196 | 13.0 |
| Hangar, 300 kW seed, mist automatic | 66 | | | | | 87 | 1.14 | 27 | 103 | 20.7 |

What the table says:
- **A fire caught in the first 90 s is one extinguisher's work; by 120 s it needs two, by 180 s
  three.** That is the damage control race: the board's dispatch, the walk, the locker.
- **A sealed room starves its own fire** in about 5.4 minutes (321 s, oxygen down to 12.6%), but the
  crew inside are dead in 3.2 (193 s), and the room's air reaches 195 kPa: the doors' interlock then
  keeps it shut, which protects the corridor and traps anyone inside.
- **Unattended, a fire destroys what is in its room**: with the fire damage rule, the forward
  switchboard's two sections and the emergency bus were destroyed 238 s after a 50 kW fire started
  (the first draft's 172 s did not reproduce: the same harness gives 204 s on the v1 plan, and the
  v2 room is larger); an engineering fire with its mist disabled took the reactor's auxiliaries,
  pumps and both main switchboard sections below 75% at 395 s and scrammed the reactor at 415 s.
- **The magazine's air passes 200 C at 274 s** (the cook-off rule, section 5, would set off its first
  missile at about 334 s); inert gas at 30 s ends the fire at 131 s with the air never above 33 C.

### 4. Suppression

| Means | Where | Effect | Cost |
| --- | --- | --- | --- |
| Portable extinguisher | Six in the damage control locker; refilled in 30 s there | Cuts the fire's heat release by 60 kW a second for 15 s each (two crew together: 120 kW/s) | A crew member at the fire, unsuited up to 2 MW |
| Water mist | Engineering, the hangar, the drive | Automatic when a fire passes 1 MW for 10 s (the board can inhibit or trigger it): 100 kW/s cut and 1 MW of cooling for 60 s; two discharges | Wet equipment (none modelled) |
| Inert gas | Magazine, forward switchboard, computer core, shield room, life support | The room's doors and damper close, nitrogen from the reserve replaces the air to 12.5% oxygen in 60 s and holds it 600 s (relief overboard) | Reserve nitrogen; anyone inside unsuited is hypoxic within a minute |
| Venting | Any room, through the duct and the overboard dump | The room to 20 kPa in 24 s (engineering 82 s); fire out below 20 kPa | The room's air; anyone inside dies unless suited or out; the refill costs reserve gas |
| Starvation | Any sealed room | Out in about 5 min | Everything in the room |

### 5. The magazine's cook-off (proposed; not in the mockup's library)

A Gannet in the racks cooks off when the magazine's air has been above 200 C for 60 s, or when a hit
deposits 10 MJ within 2 m of the racks. It is resolved as a 60 MJ hit from inside, at the racks, by the
same hit resolution (no armour; the march goes downward through the deck to the hull); the next
missile checks again 30 s later. An unattended magazine fire reaches 200 C at 274 s, so its first
missile would cook off near 334 s. The magazine has inert gas for this reason.

### 6. Repair

| Job | Needs | Time | Result |
| --- | --- | --- | --- |
| A damaged system (25-75%) | A repair kit | 1.8% a second for an officer, 0.6% for a rating (section 6a); two working together 1.6 times the faster | Up to 100% |
| A disabled system (0-25%) | A kit and 1 spare part | As above, from its integrity | Up to 100% |
| A destroyed system (0%) | 3 parts | 180 s rebuild to 25%, then the kit | Up to 100% |
| A switchboard section or panel (destroyed) | 4 parts | 120 s | Health 0.5, then the kit to 1 |
| A severed conduit | 1 part | 30 s splice | Half its capacity until the dock |
| A jammed door | A kit | 30 s | Free (a drop door can also be cranked, `shuttle-bay-and-fighters`) |
| A breach up to 1 m^2, from inside | Plates (12 in the locker), 0.25 m^2 each | 15 s a plate | Sealed; cabin pressure holds the plate; holes through one hull section into one room are patched together |
| A larger breach, or a hull section's armour | EVA through the airlock (`life-support` section 14) | 60 s per m^2 of breach; 0.5% a second of armour | Sealed; armour restored |

Spare parts: 24 aboard, in cargo. Kits: 6 in the damage control locker.

### 6a. Repairs are done in person, and officers repair three times as fast

The owner, 2026-10-08: "repairs: you need to physically go to the station to repair stuff, officers have
repair buff of 3x of normal crew members".

- **In person.** A repair is a body with a kit at the system's repair point (`crew-on-deck` section 6:
  kneeling or standing, unable to move). No console repairs anything: engineering reroutes power round
  a fault and the damage board sends people, but a system's integrity rises only under someone's hands.
  A damaged bridge console is repaired at the console.
- **Rank.** Every body has a rank, set by `crew-on-deck`'s roster: **officer** or **rating**. Every
  player's body is an officer; the watch bodies and the damage control teams are ratings.
- **Rates.** A rating restores 0.6% of integrity a second with a kit (the team rate this design already
  used); an officer 3 times that, **1.8% a second**. A system at 25% takes an officer 42 s to reach
  100% and a rating 125 s. Two working together go 1.6 times the faster of them. The rebuild and splice
  times above (180 s, 120 s, 30 s) and the breach plates (15 s a plate) are the officer's; a rating
  takes 3 times as long.
- **One rate function.** `damage::repair_time(job, who)` reads the body's rank, so the board's "42 s"
  is the time the officer will take (CLAUDE.md 6.1). Wounds, cold and gloves still slow tool work as
  `crew-on-deck` says, multiplied in.
- Recommendation taken (ask only with screenshots): players are the officers, and the rating's rate is
  unchanged, so a repair by a player is faster than before (1.8% a second against 1%).

**Previews from the resolver.** The board's time to repair (`damage::repair_time(job, who)`) is the
repair functions' own rates over the job's remaining work, plus the walk on the crew-portal graph
(`crew-on-deck`'s speeds), plus suiting if the place is below 50 kPa: the same functions the repair
calls each sub-step. Examples:

| Job | Who | Time |
| --- | --- | --- |
| The port cradle at 34% (W2) to 100% | A player with a kit / a team of two | 66 s / 69 s, plus the walk |
| A destroyed turret | A player | 180 s rebuild, then 75 s: 255 s |
| The port main switchboard section (W1) | A team, suited | 20 s suits, 46 s walk suited, 120 s rebuild: power back about 3.2 minutes (191 s) after the hit (v2 plan, 2026-10-05: the suit lockers on damage control's new outer wall, was a 44 s walk and 189 s; corrected 2026-10-04 from a 30 s walk and 3 minutes; reconciled 2026-10-05: crew-on-deck owns this, suits 20 s, was 30 s and 199 s) |

### 7. Damage control teams

Two teams of two NPC crew bodies (`crew-on-deck`), at home in damage control. Proposed values
(`damage.json` `teams`):

| Quantity | Value |
| --- | --- |
| Dispatch delay | 5 s after the board (or its automation) assigns a job |
| Speed | `crew-on-deck`'s: walk 1.8 m/s; run 4.0 m/s to a fire or to a breach with crew in it; suited 1.5 m/s with no running; ladders 0.8 m/s up and 1.0 m/s down (corrected 2026-10-04 from walk 1.6 m/s and ladders 0.8 m/s). Reconciled 2026-10-05: crew-on-deck owns this; `damage.json` `teams` now holds the same values |
| Suit | `crew-on-deck`'s donning time, 20 s, at the damage control lockers, before entering a room below 50 kPa or a fire over 2 MW. Reconciled 2026-10-05: crew-on-deck owns this (was 30 s) |
| Repair rate | 0.6% a second each with a kit: ratings (an officer, every player, 1.8%: section 6a) |
| Risk | The same air, heat, smoke and blast as players; they can be hurt, fall unconscious and die, and must be carried out |

**Automation** (the board unmanned, `bridge-stations`): jobs in this order, nearest team first: a
fire in an occupied room or a protected room's fire the systems did not stop; a breach with crew
inside; the power path (switchboard sections, trunks, the emergency bus); the reactor's auxiliaries
and coolant pumps; life support; weapons and shields; everything else. Automation never vents a room
with crew in it and never opens a door across its interlock.

### 8. The engineering bay as a place

The bay console (`eng_main`) is on the mezzanine beside the reactor; around it:

| Thing | Where (layout patch, section 15) | What crew do there |
| --- | --- | --- |
| Reactor panel | [0.0, 0.0, -21.4], mezzanine ring | Scram reset (hold 3 s, refused with its reason while hot), manual throttle (`power-grid` section 3) |
| Coolant valve manifold | [-6.5, -3.5, -27.4], lower floor by the pumps | Close a system's coolant branch to isolate a leak (that system then heats on its own mass); reopen |
| Main switchboard sections | [7.5, 0.0, -21.0] port, [-7.5, 0.0, -21.0] starboard | Local breakers: they work when the bridge cannot command them |
| Coolant pumps | [-6.5, -3.5, -29.0] | Repair |
| Water mist | Throughout | Triggered or inhibited from the panel or the board |

**When the bridge loses control.** Breakers, dampers, doors and suppression are commanded from the
bridge and the damage control board through the computer core; below half its supply (`damage.json`
`remote_control`, proposed; not in the mockup's library) they can only be worked locally: at the
switchboard sections, at each door, at each system. A lost computer core sends people to engineering.

**A reactor-room fire** is the worst fire aboard: engineering's 2,538 m^3 (v2 plan, 2026-10-05; was
2,520 m^3) and 120 MJ/m^2 make a 16 MW
fire possible. The water mist ends a 1 MW fire in 22 s; with the mist disabled, the fire put the
reactor's auxiliaries and both switchboard sections below 75% by 395 s and scrammed the reactor at
415 s. Venting engineering takes 82 s and its air cannot be replaced from the reserves (`life-support`
section 11).

### 9. Walkthrough W1: a Gannet into the main switchboard

Combat (`power-grid` section 12), shields down on the port face. A 60 MJ hit on engineering's port
side at deck B height, toward the main switchboard's port section.

| Time | What happens | From |
| --- | --- | --- |
| 0 s | Hull section 2:port absorbs 4 MJ (left at 90%). A 0.68 m^2 breach in engineering. 33.7 MJ deposited in engineering, blast radius 3.2 m. The port switchboard section is destroyed (the starboard section, 15 m away, keeps 98%). The port drive feeder and the port main trunk aft are severed. The engineer on the mezzanine takes 74 HP. No fire (the seeded roll). | The simulation |
| 0 s | The reactor carries on through the starboard auxiliary train and generator: 30 MW, 63% throttle. The battery gives 24.8 MW. Engineering at 101 kPa | The simulation |
| 2 s | Engineering's doors and damper are shut (the default and the trip); 96 kPa | The simulation |
| 0-6.9 s | The engineer can still leave: below 20 kPa of difference the door opens on approach. After that it needs the override | `life-support` section 3 |
| 5 s | The board (or its automation) dispatches both teams: they must suit (engineering will be below 50 kPa at 21.5 s) | Section 7 |
| 10 s | Engineering 73 kPa; the engineer, still inside, is impaired | The simulation |
| 30 s | Battery at its reserve: priority 3 drops (the drive stops), priority 2 at 68%; engineering 38 kPa | The simulation |
| 60 s | Engineering 15 kPa; the engineer unconscious (26 HP left) | The simulation |
| 71 s | Both teams arrive, suited (5 s + 20 s suits + 46 s walk at the suited 1.5 m/s by the aft passage) | Section 7 |
| 71-116 s | Team 1 patches the breach from inside: three plates, 45 s | Section 6 |
| 71-191 s | Team 2 rebuilds the port switchboard section: 4 parts, 120 s | Section 6 |
| 191 s | Section rebuilt at 50%: 31 MW (the severed trunk still isolates it) | The simulation |
| 221 s | The port trunk aft spliced: the reactor back to 100%, 48 MW; priority 3 back to its combat share | The simulation |
| 251 s | The port drive feeder spliced | The simulation |
| 177 s | If nobody pulled the engineer out: dead (90 s below Armstrong's limit) | The simulation |

Corrected 2026-10-04 to `crew-on-deck`'s speeds: the teams' walk was 30 s (the first route table's
29.9 s at 1.6 m/s, through the hangar with T1). Suited, at 1.5 m/s with no running, by the aft
passage (the route as laid out then; `tools/walk_times.py` printed 59.7 m and 44.3 s from the suit
lockers on the v1 boxes), it is 44 s, so every team event moved 14 s later. The events were
shifted, not re-run in the library (which does not walk teams); the power at each event is the
same. Reconciled 2026-10-05: crew-on-deck owns this. Its suit donning time is 20 s, not the 30 s
this table first used, so every team event moves 10 s earlier again (arrival 79 s to 69 s, the
section back at 189 s instead of 199 s, the last splice at 249 s instead of 259 s), shifted the
same way. The engineer's time (188 s then) does not move with the teams: it is the air's.

Re-measured on the v2 plan, 2026-10-05: the suit lockers stand on damage control's new raked outer
wall (x -9.09, was -6.6), so `tools/walk_times.py` prints 62.0 m and 45.9 s for the suited walk
(was 59.7 m and 44.3 s) and every team event moves 2 s later, shifted the same way (arrival 69 s to
71 s, the section back at 191 s instead of 189 s, the last splice at 251 s instead of 249 s). The
air, the breach, the energy and the engineer's rows were rerun in the library on layout v2 the same
day, the harness unchanged: on v1 the breach was 0.60 m^2, 29.6 MJ deposited (blast radius 3.1 m),
60 HP to the engineer, engineering at 76, 42 and 18 kPa at 10, 30 and 60 s, the door passable until
7.8 s, below 50 kPa at 24.1 s, the engineer impaired at 60 s and dead at 188 s, and 3,043 kg lost.

Engineering lost 3,069 kg of air; the reserves cannot refill it, so it stays in vacuum and the
engineering bay is worked suited for the rest of the mission (`life-support` question L5). The
reactor never scrammed: separated switchboard sections and two auxiliary trains (`power-grid`
sections 3 and 5) are why. In the first draft, before both, this hit destroyed both sections and
scrammed the reactor 1.9 s later.

### 10. Walkthrough W2: a Lance under the port launch bay

Combat; Swift 1's pilot sealed in the cockpit, suited, in the pressurized port launch bay; a deck
hand in the hangar. A 40 MJ Lance strikes the keel under the bay, travelling up. (Rerun on layout
v2, 2026-10-05, whose bay is 190.1 m^3, was 231 m^3: on v1, 13.6 MJ in the bay, a 1.4 MW fire, the
pilot 19 HP, the bay at 41 kPa at 4 s, the fire out at 28 s and 294 kg lost.)

| Time | What happens |
| --- | --- |
| 0 s | Hull section 3:ventral absorbs 4 MJ. A 0.46 m^2 breach in the port launch bay. 12.3 MJ in the bay, 4.3 MJ in the hangar's port gallery above (through the deck, less a bulkhead's 1.5 MJ). The port cradle falls to 34% (damaged: capability 45%, its release charge takes 2.2 times as long). The port main trunk aft is severed in the gallery: the hangar's port panel is now fed from forward through the midships trunk, and nothing is lost. A 1.3 MW fire starts in the bay. The pilot takes 15 HP, the deck hand 1 |
| 2 s | The bay at 60.5 kPa (its pressure door was shut, as pressure doors are); the fire at 1.31 MW |
| 4 s | 33.9 kPa (the `breach-launch-bay` shot) |
| 10 s | 6.2 kPa; the fire at 0.88 MW and failing for oxygen |
| 23 s | The fire out; the bay at vacuum, 244 kg of air lost |

What the crew do: flight ops can still launch Swift 1 (the bay is in vacuum: no pump-down), with the
damaged cradle's slower release, if the drop door was not jammed (doors within reach that take 25
points jam: section 1). A player with a kit brings the cradle to 100% in 66 s, suited. The splice can
wait: the forward feed carries the panel. The breach (2 plates, 30 s, suited) is patched before the
bay is repressurized, which the receiver then does in 13 s.

### 11. Walkthrough W3: pulse fire raking the forward switchboard

Combat with the starboard shield face down; a Hound's turrets put 40 bolts of 1.2 MJ into one hull
section (4:starboard, deck C, abreast the forward switchboard) at 4 a second. (Rerun on layout v2,
2026-10-05: the room now runs out to 1.28 m from the skin, was 1.63 m, so the bolts reach it with
more energy. On v1, bolt 25 did not reach the room, bolts 26-40 made 15 holes of 0.074 m^2 together,
the fire was 69 kW and the room at 6.8 kPa at 44 s.)

| Bolt | What happens |
| --- | --- |
| 1-24 | Each absorbed by the armour; the section loses 3 points each, to 28% |
| 25 | The armour now holds less than a bolt: 0.1 MJ passes, enough to hole the forward switchboard (0.002 m^2) |
| 26-40 | Every bolt holes the forward switchboard: with bolt 25's, 16 holes of 0.002-0.016 m^2, 0.185 m^2 together. At bolt 32 a 107 kW fire starts |
| 44 s after the first bolt (30 s after the last) | The room at 1.5 kPa; the crew member inside unconscious; the emergency bus at 99.6%, the battery untouched; the section at 0% armour |

The danger is the next burst: the section is stripped, and bolts now carry most of their energy into
a room that holds both forward switchboard sections, the emergency bus and the battery bank. What the
crew do: helm turns a fresh face toward the Hound and science rebalances the shields
(`weapons-and-shields`); a suited team patches the holes together (one plate, 15 s, by the grouping
rule: 0.185 m^2 is under a plate's 0.25 m^2); the hull section's armour waits for an EVA (200 s
from 0% at 0.5% a second) or the dock.

### 12. Console readouts and previews

The damage control board (`bridge-stations` 10.9) and the engineering consoles show:

| Panel | Readout | Source |
| --- | --- | --- |
| D1 Compartments | Per compartment: fire (MW), smoke, breaches (m^2), pressure, power (lighting state), crew and their state | `life-support` and this change's state |
| D1 | Hull sections: armour integrity by span and face | The hull state |
| D2 Teams | Each team: place, job, progress, suits, health | Team state |
| D3 Queue | Jobs by priority, each with time to repair, parts and kits needed | `damage::repair_time` (the resolver's rates) |
| D4 Doors | Open, shut, held, jammed, interlocked (with the difference in kPa); SEAL COMPARTMENT; vent (guarded, with crew inside named) | `life-support` |
| Status strip | Fires and breaches counted | State |

### 13. The data: `data/ships/tern/damage.json` and the `fire` block

| Block | Fields |
| --- | --- |
| `hull` | Armour MJ, section MJ, faces |
| `breach` | m^2 per MJ, minimum, maximum |
| `propagation` | Step, maximum march, decay length, bulkhead MJ, base radius, radius per square-root MJ |
| `systems` | Points per MJ, the nominal and disabled thresholds |
| `nodes` | The destroyed threshold |
| `conduits` | Sever MJ, damage MJ, damaged capacity |
| `fire` | Ignition chance per MJ and cap, seed kW per MJ |
| `crew` | HP per MJ, share |
| `repair` | Kit rates, lockers and counts, parts on board and per job, rebuild and splice times and results, EVA hull rate, two-hands factor |
| `patch` | Plate area, plates, seconds per plate, inside limit, EVA rate and limit, hole grouping |
| `doors` | Jam points, repair time |
| `cook_off` | Compartment, air limit K and hold s, hit MJ within m, warhead MJ, next check s |
| `remote_control` | Computer supply needed |
| `teams` | Count, members, home, speeds, suit time, dispatch delay, entry limits |
| `atmosphere.json` `fire` | Chemistry, growth, ceiling, fuel by room, oxygen and pressure limits, seed, autoignition, decay, out threshold, room-content damage, extinguisher, inert gas, water mist |

Validated at startup as `power-grid` section 13 says.

### 14. The Pi 5 budget this change spends

**A hit that passes the armour**: 0.55 ms in the JavaScript instrument for a 60 MJ hit (on the cloud
container's Xeon), after pre-filtering targets to those within the largest radius of the whole ray.
Estimated under 0.1 ms in the Rust core with each compartment's targets precomputed at load. Rule: at
most four such hits are resolved per 30 Hz tick; more wait for the next tick (33 ms, unseen). Hits
the armour stops, and every shield hit, cost microseconds. **Fire, repair, teams' jobs**: part of the
systems sub-step (`power-grid` section 14). **Memory**: 54 hull sections, a pool of 64 breaches,
30 fire states, job queues: a few kB. **Network**: hit reports are events (about 40 bytes each:
section, breach, the systems hit); fires, breaches and team states go with `life-support`'s
compartment values.

### 15. Layout patches proposed

For `reference-ship-tern` to apply to `layout.json`, beside `power-grid`'s reactor panel and coolant
valves. The layout holds all three; on the v2 plan (2026-10-05) both lockers stand on damage
control's raked outer wall, facing in, and the positions below are the layout's (was
[-6.6, 0.0, 24.0] and [-6.6, 0.0, 19.6], facing -90 degrees, on the v1 box's wall):

```json
{ "id": "dc_lockers", "kind": "locker", "compartment": "damage_control", "center_m": [-8.38, 0.0, 24.0], "size_m": [0.6, 2.4], "facing_yaw_deg": 99,
  "note": "Six extinguishers, six repair kits, twelve breach plates (damage-control)." },
{ "id": "eva_suits", "kind": "locker", "compartment": "damage_control", "center_m": [-9.09, 0.0, 19.6], "size_m": [0.6, 2.4], "facing_yaw_deg": 99,
  "note": "Four EVA suits; suiting takes 20 s (crew-on-deck)." },
{ "id": "spare_parts", "kind": "rack", "compartment": "cargo", "center_m": [-4.0, -3.5, 7.5], "size_m": [3.0, 1.0], "facing_yaw_deg": 0,
  "note": "24 spare parts (damage-control)." }
```

## Risks / Trade-offs

- **Fires do not spread through shut doors.** Realistic for a steel ship over a mission, but it makes
  spread rare. Spread still happens through held or jammed doors and to rooms whose air passes 300 C;
  if play wants more, the bulkhead U value and autoignition are data.
- **A switchboard fire is lethal to the switchboard in four minutes** (238 s). That is why it has inert gas;
  the board must see protected rooms' systems' state.
- **Crew hurt by expected share in the mockup** is not the engine's rule; positions decide there.
- **Teams as bodies** cost path finding and animation (`crew-on-deck`), and can die; that is the point
  (question D1).
- **Hit cost** is bounded by the per-tick cap; a missile salvo's blasts spread over a few ticks.

## Mockup shots

`docs/mockups/systems.html`, screenshots in `docs/screenshots/mockups/`:

| Shot | Shows |
| --- | --- |
| `fire-in-engineering` | A 300 kW fire in engineering 70 s later at 1.06 MW, the water mist about to discharge (it needs 10 s over 1 MW), the damage overlay |
| `breach-launch-bay` | Walkthrough W2, 4 s after the hit: the breach ring, the fire, the bay at 33.9 kPa |
| `power-severed` | The port trunk cut: what a severed conduit looks like on the board |

## Open questions

Ids D (damage control). Questions with a shot go to the owner's survey with the shot beside them;
the rest are recommendations taken (ask only with screenshots, CLAUDE.md 13).

| Id | Question and the fact it turns on | Options | Recommendation | Shot |
| --- | --- | --- | --- | --- |
| D1 | Are damage control teams crew bodies? `bridge-stations` left it here. | (a) Two teams of two NPC bodies that walk, suit, work and can die. (b) Abstract teams: a timer per job. | (a): the ship is a place and the danger is real for everyone. | none: recommendation taken (ask only with screenshots) |
| D2 | Does the water mist fire itself? It ends a 1 MW engineering fire in 22 s; without it the reactor scrams at 415 s. | (a) Automatic above 1 MW for 10 s, inhibitable from the board. (b) Manual only. | (a). | `fire-in-engineering` |
| D3 | Who may vent a room with crew inside? Venting kills unsuited crew in about 90 s. | (a) The board and engineering, a guarded hold, the crew inside named. (b) Only with the captain's authorization. | (a): fast, and the warning makes it a choice. | none: recommendation taken (ask only with screenshots) |
| D4 | Should pulse bolts ever penetrate? Twenty-four on one section wear its armour through; from the twenty-fifth they hole the room. | (a) As designed. (b) Bolts never pass armour; only missiles breach. | (a): focused fire on a weakened face is a tactic for both sides. | none: recommendation taken (ask only with screenshots) |
| D5 | The magazine cook-off. | (a) As designed: 200 C for 60 s, or 10 MJ near the racks. (b) No cook-off. | (a). | none: recommendation taken (ask only with screenshots) |
| D6 | How a breach looks on the board and the deck plan. | (a) A red ring sized by area, the room tinted by pressure. (b) Text only. | (a). | `breach-launch-bay` |
