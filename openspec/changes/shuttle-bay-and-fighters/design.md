# Design: shuttle bay and fighters

Status: **proposed** (2026-10-04). Nothing here is built. Pressures, pump-down, venting and
repressurization times are `life-support`'s (its section 13, computed from
`data/ships/tern/atmosphere.json`). Power draws are `power-grid`'s. Budget numbers are against
`openspec/changes/engine-stack/design.md` section 5 (the Pi 5 table, the one source). Corrected
2026-10-04 to life-support's figures, and again 2026-10-05 when it reran them on layout v2: launch is
permitted at 5 kPa after a 23.6 s pump-down (28.6 s on v1; not 1 kPa after an assumed 30 s), a bay
refills in 13.0 s (15.7 s on v1; not 25 s), the hangar pumps down in 206.5 s (208.4 s on v1; not
120 s); the timelines below are recomputed from their steps.

## Context

From `data/ships/tern/layout.json`:

| Thing | Where | Numbers |
| --- | --- | --- |
| Port launch bay (`launch_bay_p`, POI 16) | Deck C, x 5.0 to 10.4, z -16 to -4, 3.0 m tall (y -3.5 to -0.5), its outer corners chamfered | 190.1 m^3, 63.4 m^2 floor |
| Starboard launch bay (`launch_bay_s`, POI 17) | Mirror of port | 190.1 m^3 |
| Hangar (`hangar`, POI 15) | Decks C and B: the double-height floor plus galleries over both launch bays, a 0.5 m slab between | 1,682.4 m^3 |
| Drop doors (`p_drop_p`, `p_drop_s`) | In each bay's floor, centre [+/-7.75, -3.5, -10.0] | 4.8 x 9.0 m |
| Pad door (`p_hangar_pad`) | In the hangar floor, centre [0.0, -3.5, -10.0] | 6.0 x 12.0 m |
| Bay pressure doors (`p_bay_p`, `p_bay_s`) | Hangar floor to each bay, [+/-5.0, -2.4, -5.0] | 1.0 x 2.2 m |
| Cradles (`cradle_p`, `cradle_s`), lift pad (`shuttle_pad`) | Under each craft | |
| Bay pumps and reserve tank (`hangar_pumps`) | Hangar floor, [-4.0, -3.5, -2.0] | |
| Bay control (`bay_control`) | The hangar's forward landing, [3.5, 0.0, -1.2], facing aft | |
| Swift 1, Swift 2 | On the cradles, centre y -2.6 | 7.0 x 4.6 x 1.8 m, 1 seat |
| Petrel | On the pad, centre y -1.9 | 10.0 x 4.5 x 3.2 m, 4 seats |

Volumes are the v2 plan's, whose rooms follow the hull (`reference-ship-tern` section 1a,
2026-10-05; on the v1 boxes each bay was x 5.0 to 10.5 and 3.5 m tall, 231 m^3 and 66 m^2, and the
hangar 1,698 m^3). The pump-down, vent and refill times below are `life-support`'s, rerun on the v2
volumes (2026-10-05); they move when it recomputes them.

The keel (the hull's bottom) is at -5.8 m under all three doors, so each door opens into a 2.3 m
trunk through the lower hull with an outer fairing door flush with the keel.

## Goals / Non-Goals

**Goals:**
- A launch a player can wait through and a crew can speed up: every step has a time, a reason and
  an owner.
- Recovery that a player can fly by hand and that the ship's helm must cooperate with.
- Failures that are fixable by crew on foot.
- Fighters useful with and without players.

**Non-Goals:**
- Carrier-scale operations (catapults, arresting wires, more than three craft).
- The air simulation itself (`life-support`), and EVA movement (`crew-on-deck`).
- Fighter-specific console art (`bridge-stations`).

## Decisions

### 1. Release and capture geometry

| Craft | Cradle travel | Release and capture point (ship frame) | Speed of travel | Time |
| --- | ---: | --- | ---: | ---: |
| Swift 1 | 4.6 m down | [7.75, -7.2, -10.0] | 1.3 m/s | 3.5 s |
| Swift 2 | 4.6 m down | [-7.75, -7.2, -10.0] | 1.3 m/s | 3.5 s |
| Petrel | 6.0 m down | [0.0, -7.9, -10.0] | 0.5 m/s | 12 s |

The travel puts the top of the craft 0.5 m below the keel (Swift: centre -2.6, top -1.7, to centre
-7.2, top -6.3). Doors: a drop door and its fairing open together in 4 s; the pad door in 8 s.

### 2. Bay postures

A bay is always in one posture, set by flight ops (or bay control, or automation on a pilot's
request):

| Posture | State | From "launch" to release |
| --- | --- | ---: |
| Stowed | Pressurized; no pilot; preflight not done | Pilot's walk, then 37.6 s |
| Ready 5 | Pilot aboard, canopy sealed, preflight done; bay pressurized | 31.6 s (23.6 + 4 + 3.5 + 0.5) |
| Ready 1 | As ready 5, bay pumped down, drop door shut | 8 s (4 + 3.5 + 0.5) |
| Hot | Ready 1 with the door open and the cradle lowered | 0.5 s |

What the postures cost: a pilot waiting in a cockpit holds no other station; a bay in vacuum
cannot be entered without a suit; an open door is an opening in the hull (a hit through it can
strike the craft, `damage-control`), and while any door is open the ship's shields still cover it
(the ellipsoid encloses the keel).

### 3. The launch sequence (a Swift, cold)

| # | Step | Time | Who | Interlock |
| --- | --- | ---: | --- | --- |
| 1 | The pilot enters the bay through its pressure door and climbs in; canopy seals | 6 s | Pilot | Bay pressurized (or the pilot is suited) |
| 2 | Preflight: propellant, capacitor, Darts, systems | 8 s, during step 3 | Automatic | Skipped if done in the last 10 minutes |
| 3 | Pump-down: the bay pressure door closes and seals; the bay pumps move the bay's air into the reserve tank (life-support's bay receiver) and stop at 5 kPa, where launch is permitted and the bay's 0.1 m^2 vent valve takes the rest | 23.6 s (life-support section 13) | Flight ops or bay control | Nobody in the bay outside a sealed cockpit unless suited |
| 4 | Drop door and fairing open; the last 12 kg of air leaves through it | 4 s | Flight ops | Bay at or below 5 kPa (launch permitted), or emergency vented |
| 5 | Cradle lowers to the release point | 3.5 s | Automatic | Door fully open |
| 6 | Release: clamps open, a 4 m/s ejection along -Y; `ship-frames` hands the craft to the system frame at this tick | 0.5 s | The pilot or flight ops | Ship turning under 10 deg/s and accelerating under 5 m/s^2 (proper), or overridden |
| 7 | Clear: main engine allowed 15 m from the keel; "clear" at 30 m | 2-7.5 s | Pilot | |
| 8 | Cradle rises; door closes | 3.5 s + 4 s | Automatic | Craft clear |
| 9 | Repressurize from the receiver, if wanted (a bay left in vacuum is ready for a recovery) | 13.0 s (life-support section 13) | Flight ops | |

From the pilot entering the bay to release: 6 + 23.6 + 4 + 3.5 + 0.5 = **37.6 s** (preflight
overlaps the pump-down; corrected 2026-10-04 from 44 s at an assumed 30 s pump-down, and 2026-10-05
from 42.6 s on layout v1).
**Emergency vent** instead of the pump-down: the drop door opens with the bay full, and its opening
is the vent. The bay passes Armstrong's limit 0.9 s after the door starts to open, losing all
228 kg of its air (life-support section 13), and is below 1 kPa well inside the door's 4 s travel
(at 1.4 s on the v1 bay; not rerun) (`docs/mockups/lib/shipsystems.js` run headless on the proposed data): release 4 + 3.5 + 0.5 =
**8 s** after the vent starts (corrected 2026-10-04 from 14 s, which assumed a separate 6 s vent
before the door).

**The Petrel**: the whole hangar must be emptied: its pressure doors to the main corridor, the lower
corridor, engineering and the two galleries close, and everyone in the hangar and galleries must be
suited or out (bay control is on the forward landing, inside the hangar: its operator suits up or
hands control to flight ops). Pump-down to 5 kPa 206.5 s at the pumps' full 6 MW (life-support
section 13; the emergency vent instead passes Armstrong's limit 2.8 s after the pad door starts to
open and loses 2,019 kg of air, 74% of the ship's reserve gas); pad door 8 s; pad 12 s; release at
1 m/s. About **232 s** from the seats being filled (corrected 2026-10-04 from 145 s at an assumed
120 s pump-down, and 2026-10-05 from 233 s on layout v1; life-support question L3): the Petrel is
not a combat launch.

### 4. Recovery

| Step | Detail | Time |
| --- | --- | ---: |
| Request | The pilot requests recovery; flight ops assigns a bay (its own, or the other if free; the pad when the hangar is in vacuum) | |
| Prepare | The bay pumps down, the door opens, the cradle lowers to the capture point | 23.6 + 4 + 3.5 = 31.1 s, unless already hot |
| Approach | From astern, inside the recovery corridor: a box under the bay, 3 m either side of the capture point, from 2 m below the keel to 40 m below it, from the capture point to 200 m aft | |
| Speed limits, relative to the ship | Under 20 m/s inside 200 m; 5 m/s inside 50 m; 1 m/s inside 10 m | |
| Recovery hold | While a craft is inside 50 m, helm's flight assist holds the ship under 3 deg/s and 2 m/s^2 (helm sees "recovery: hold steady" and may override) | |
| Capture | The craft's centre within 0.6 m of the capture point, relative speed under 1.0 m/s, attitude within 5 degrees of the cradle's, for 1 s; the clamps close | 1 s |
| Hand-off | `ship-frames` capture: the relative velocity is absorbed as an impulse | One tick |
| Stow | Cradle rises 3.5 s; door closes 4 s | 7.5 s |
| Repressurize | From the receiver (life-support section 13) | 13.0 s |
| Pilot exits | Canopy opens, climbs down | 4 s |

**Auto recovery**: inside 200 m the pilot (or a drone) can hand the last leg to the craft's
autopilot, which flies the corridor with `flight-and-navigation`'s docking autopilot (one
implementation) at the same limits. A player flying it by hand is faster: the autopilot flies the
limits exactly, about 35 s from 200 m.

The capture limits mean the ship's motion matters. With the Tern turning at 3 deg/s, the capture
point (about 15 m from the centre of mass) moves at 0.8 m/s relative to a straight-flying craft:
inside the 1.0 m/s limit, but only just. That is why the recovery hold exists.

### 5. Failures

| Failure | Cause | What happens | Fix |
| --- | --- | --- | --- |
| Drop door jammed | A hull hit near the door, its actuator damaged, or power lost mid-cycle | The door stops where it is; a Swift (4.6 m span) needs it at least 96% open | A suited crew member cranks it in the bay (60 s closed to open), or damage-control repairs the actuator |
| Bay breached | A hull hit opens a breach in the bay | Life-support empties the bay; launches need no pump-down; the pressure door to the hangar locks (interlock), so the pilot must be suited to board | Damage-control patches the breach (life-support repressurizes) |
| Ship manoeuvring at release | Helm turning over 10 deg/s or accelerating over 5 m/s^2 | Release is held; helm and flight ops see "launch hold: steady the ship" | Helm steadies; or flight ops overrides ("combat drop") after seeing the predicted clearance |
| Combat drop goes wrong | Override with a predicted clearance under 0.3 m | The trunk strikes the craft: a 1-3 MJ kinetic hit on the craft, reported to the pilot and damage-control | |
| Cradle without power | The cradle's bus fails mid-travel | The cradle stops; the craft cannot release or be stowed | A crew member cranks it (45 s full travel) |
| Pressure door will not seal | Door damaged | The bay cannot be pumped down: launches blocked unless the hangar side is emptied too | Repair the door |
| Craft too damaged to capture | Its attitude control is damaged and it cannot hold 5 degrees | The pilot ejects near the ship; the Petrel or a bay recovers the pod | |
| Recovery under fire | A hit near the corridor | Nothing special: the craft is a body like any other; the hold still applies | |

The predicted clearance for a combat drop is computed by the same function the release uses
(the trunk's swept motion over the next 1 s from the ship's state), so flight ops' preview cannot
disagree with what happens (CLAUDE.md 6.1).

### 6. Flight ops and bay control

`bridge-stations` owns the console; this is what it shows and does.

**Sees**, for each of the three bays: pressure (kPa, life-support's value), posture, door state and
percent open, cradle position, the craft (propellant, capacitor, Darts, shield, hull), the pilot
aboard and anyone else in the bay with their suit state, each sequence step with its countdown, and
every interlock that is holding the sequence with its reason in words.

**Sees**, for each craft in flight: position on a local plot, propellant (with bingo), damage, orders,
player or drone; and for an inbound recovery, the approach panel: the craft's offset from the capture
point (x, y, z in metres), closure speed, alignment error, and the predicted capture result.

**Does**: set a bay's posture; start, hold and abort a sequence; emergency vent; override an
interlock (with a confirm); assign a recovery bay; order a drone launch or recall; task craft
(escort the Tern, engage the designated target, patrol a point, hold, return); show a craft's nose
camera on a secondary feed (`ship-frames` section 8).

**Bay control** in the hangar is the same console (one implementation) plus what only hands on site
can do: the door and cradle cranks, the bay lights, the Petrel pad, and opening a bay pressure door
locally. **When flight ops has no player** it is merged into tactical (vision.md); a pilot in a
cockpit can request a launch or a recovery and automation runs the sequence with every interlock
and no overrides.

### 7. The Swift fighter

| Quantity | Value | Notes |
| --- | ---: | --- |
| Size | 7.0 x 4.6 x 1.8 m | From the layout |
| Mass | 8,000 kg with propellant (7,200 kg dry) | |
| Inertia (box estimate) | pitch 34,800, yaw 46,800, roll 16,300 kg m^2 | `m/12 (a^2 + b^2)` on the box |
| Main engine | 480 kN | 60 m/s^2 full, 67 m/s^2 dry |
| Reverse | 160 kN | 20 m/s^2 |
| Lateral and vertical (RCS) | 120 kN | 15 m/s^2 |
| Rotation limits under assist | pitch 90, yaw 60, roll 180 deg/s | |
| Angular acceleration | pitch 270, yaw 180, roll 540 deg/s^2 | Torques 164, 147, 154 kN m |
| Propellant | 800 kg at 150 km/s exhaust velocity | 3.2 kg/s at full thrust: about 250 s of full main thrust; delta-v 15.8 km/s |
| Assist speed limit | 320 m/s | Off-assist: none |
| Guns | Two light pulse guns, fixed forward with a 5 degree gimbal | 3 bolts/s each, 1.0 MJ drawn, 0.8 MJ delivered, 1,500 m/s, life 1.2 s |
| Gun capacitor | 18 MJ, charged at 3 MW by the craft's own reactor | 6 s of full-rate fire, then 3 bolts/s |
| Gun heat | 0.15 MJ per bolt into a 4 MJ sink, 0.6 MW dissipation | Locks after about 13 s of full rate |
| Darts | 2 short-range missiles: 80 kg, 15 MJ warhead, boost 200 m/s^2 for 3 s, range 3 km | The Gannet's code with Dart data |
| Shield | One bubble, radius 4.0 m, 8 MJ, regenerating 0.8 MJ/s after 3 s without a hit | `weapons-and-shields`' damage resolution |
| Hull | 6 MJ | |
| Cockpit dampers | Rated 40 m/s^2 | `ship-frames`' residual: a full burn leaves 20 m/s^2, felt as a camera lurch (the pilot is strapped in) |
| Datalink | Shares the Tern's sensor tracks within 20 km | |
| Ejection | At hull 0 the pilot ejects in a 300 kg pod with a beacon and 30 minutes of air | |
| Triangles | 1,500 (400 at the far LOD) | engine-stack's fighter ceiling; cockpit 1,200 |

**Flight** is `flight-and-navigation`'s one model with the Swift's data: six degrees of freedom,
Newtonian, with the same three assist modes (full, rotation only, off) and the same controller. The
pilot's own client predicts the fighter (`netcode-and-sessions` section 5). Propellant drains with
thrust (main and RCS together, by the magnitude of thrust used) and the mass falls with it, so a dry
Swift is livelier.

**Against a Jackal** (`weapons-and-shields` section 13): the Swift has twice its shield, double its
hull and better guns; a good pilot wins one on one, and two Jackals are a fair fight.

**Pilot's view**: the cockpit (`ship-frames` section 7) with a HUD: speed relative to the Tern or the
target, assist mode, propellant with bingo, capacitor and heat, shield and hull, Darts, the target's
lead pip and hit chance, and the recovery corridor when approaching.

### 8. The Petrel shuttle

| Quantity | Value |
| --- | ---: |
| Size | 10.0 x 4.5 x 3.2 m (layout) |
| Seats | 4, the pilot's included |
| Mass | 28,000 kg loaded |
| Main engine | 280 kN (10 m/s^2) |
| RCS | 84 kN (3 m/s^2); rotation 30 deg/s |
| Propellant | 2,000 kg |
| Cargo | 2,500 kg: two Gannets, spare parts, oxygen bottles |
| Shield | One bubble, 6 MJ; hull 12 MJ |
| Weapons | None |
| Docking collar | Docks with a station, a derelict or another ship (`ship-frames` section 12) |
| Triangles | 2,000 (500 far); a proposed addition to engine-stack's table |

**Uses**: carrying an away team to a derelict or station and docking there; rescue (picking up
ejected pilots' pods and survivors through its side hatch, inside a 5 m capture range); cargo
(resupply from a depot mid-mission); EVA support (a platform for suited crew repairing the hull).
With no pilot it flies only as an autopilot ferry (go to, dock, return), never into combat.

### 9. Automation with no pilot

**Recommendation: fighters fly as drones, launched only on an order.** A drone is a Swift whose
pilot is the computer core:

| | Drone | Player |
| --- | --- | --- |
| Launch | Only on flight ops' (or tactical's) order; automation runs the sequence | The player |
| Gun aim error | 0.5 deg (a veteran Jackal is 0.3) | The player's |
| Reaction | 1.0 s | The player's |
| Behaviour | Escort the Tern within 2 km by default; engage on order with the Jackal state machine (orbit, run, fire, retreat) | Free |
| Range | Within 10 km of the Tern (datalink) | Anywhere |
| Fuel | Returns for auto recovery at 25% propellant | The player |
| Computer core | 10% each (assumed; the core's model is bridge-stations') | None |
| Loss | A destroyed drone is a destroyed Swift: it persists as destroyed | Same, and the pilot ejects |

The alternative, fighters that never leave without a player, wastes two craft in a three-player game
and removes flight ops' tasking job. Drones never launch on their own: a crew that forgets them is
not surprised by them.

**When a player wants a fighter a drone is flying**, flight ops recalls it; the player boards after
it is stowed. A player cannot take over a drone in flight (there is no one in the seat).

### 10. Rearm and refuel

On the cradle, through the cradle's umbilical, pressurized or not:

| Item | Time | How |
| --- | ---: | --- |
| Propellant, 800 kg at 20 kg/s | 40 s | From the Tern's craft propellant store (a campaign consumable) |
| Gun capacitor, 18 MJ at 3 MW | 6 s | By the craft's own reactor (section 7): `power-grid` carries no ship load for it ("a craft's own capacitors are the craft's", its section 15; the cradle's 0.6 MW is its release charge). Corrected 2026-10-04 from 6 MW from the bus |
| Darts, two | 15 s each, automatic from the bay rack (4 spares a bay); 8 s each by crew | |
| Shield | 10 s | It regenerates on the cradle |
| Hull | Damage-control's repair | Crew with a repair kit |

A full turnaround is about **40 s** (propellant is the long pole; the rest runs in parallel). The
Petrel refuels in 100 s.

### 11. The Pi 5 budget this change spends

**Rendering**: Swift 1,500 triangles (400 far), Petrel 2,000 (500 far), the cockpit 1,200 in its own
pass; the doors, cradles and pad are the Tern's instanced attachments (`ship-frames` section 9). Bay
interiors are `deck-pipeline`'s. **Server CPU**: three craft and up to two drones through the one
flight model and the Jackal behaviour, under 0.05 ms per tick. **Network**: a craft in flight is a
body near the ship in netcode's existing group (20 bytes at 20 Hz: 3.2 kbit/s for each one in
view); bay states change rarely (doors and cradles are a few bits on change; pressures ride the
compartment air group). **Memory**: fixed slots for three craft and two pods.

### 12. Data

| File | Holds |
| --- | --- |
| `data/ships/tern/bays.json` | Per bay: cradle travel and speed, door times, release and capture points, the corridor box, speed limits, interlock thresholds, override clearance |
| `data/craft/swift.json` | Mass, inertia, thrusts, rotation limits, propellant, guns, Darts, shield, hull, cockpit dampers, LODs |
| `data/craft/petrel.json` | The same for the Petrel, plus seats and cargo |
| `data/craft/drone.json` | Drone competence: aim error, reaction, escort radius, bingo, core load |

## Layout notes (proposed for reference-ship-tern)

1. The drop and pad doors sit at the bay floors (y -3.5 m) and the keel is at -5.8 m, so each has a
   2.3 m trunk that the layout does not record. Proposed addition to `mounts`, so the exterior
   model, the release and the capture all read one source:

```json
{ "id": "trunk_p", "kind": "launch_trunk", "center_m": [7.75, -5.8, -10.0], "facing": [0, -1, 0], "size_m": [4.8, 9.0], "depth_m": 2.3, "release_m": [7.75, -7.2, -10.0], "bay": "launch_bay_p" },
{ "id": "trunk_s", "kind": "launch_trunk", "center_m": [-7.75, -5.8, -10.0], "facing": [0, -1, 0], "size_m": [4.8, 9.0], "depth_m": 2.3, "release_m": [-7.75, -7.2, -10.0], "bay": "launch_bay_s" },
{ "id": "trunk_pad", "kind": "launch_trunk", "center_m": [0.0, -5.8, -10.0], "facing": [0, -1, 0], "size_m": [6.0, 12.0], "depth_m": 2.3, "release_m": [0.0, -7.9, -10.0], "bay": "hangar" }
```

2. The outer 0.3-0.6 m of each 4.8 m drop door's fairing lies on the hull's lower chamfer (the keel
   flat ends at |x| 9.9 m at z -5.5 and 9.5 m at z -14.5; the door reaches 10.15 m). The fairing
   must follow the chamfer, or the bays move 0.6 m inboard; the mockup draws a flat fairing that
   stands proud of the chamfer by up to 0.6 m.

## Risks / Trade-offs

- **A 37.6 s cold launch may feel long.** Mitigation: the ready postures (8 s from ready 1), the
  emergency vent (8 s, at the cost of 228 kg of air), and the sequence overlapping preflight with
  the pump-down. Question H2.
- **Recovery by hand may be too hard.** Mitigation: auto recovery inside 200 m; the limits are data.
- **Drones may make players feel redundant.** Mitigation: drones are worse shots and slower to react,
  escort by default and only launch on an order.
- **The pump-down and vent numbers are life-support's.** If they move, the sequence times here move
  with them; nothing in this design hard-codes them.

## Mockup shots

| Shot | Shows |
| --- | --- |
| `fighter-drop` | Swift 1 just released below the open port drop door, the cradle lowered, the Tern above |
| `fighter-recovery` | Swift 2 rising into the starboard trunk on its cradle, the door open |
| `fighter-cockpit` | The Swift's cockpit view, flying alongside the Tern |

## Open questions

Ids H (hangar). Questions with a shot go to the owner's survey; the rest are recommendations taken
(ask only with screenshots, CLAUDE.md 13).

| Id | Question and the fact behind it | Options | Recommendation | Shot |
| --- | --- | --- | --- | --- |
| H1 | What do fighters do with no pilot? | (a) Fly as drones at reduced competence, launched only on an order. (b) Stay docked. (c) Launch themselves in combat. | (a) | `fighter-drop` |
| H2 | How fast is a cold launch? 37.6 s from the pilot in the bay to release (life-support's 23.6 s pump-down to 5 kPa; corrected 2026-10-04 from 44 s, and 2026-10-05 from 42.6 s on layout v1), 8 s from ready 1. | (a) As designed, with the postures. (b) Faster pumps (a 15 s pump-down, life-support's call). | (a): the wait is a crew decision (keep a pilot on ready 1), not a delay to remove. | `fighter-drop` |
| H3 | What happens to a pilot who ejects? | (a) The pod persists and can be recovered; the player respawns in the medbay after 45 s. (b) The player waits in the pod until picked up. | (a): nobody sits out a mission; rescue still matters for the campaign. | none: recommendation taken (ask only with screenshots) |
| H4 | Can a Swift be recovered through the pad door? | (a) Yes, when the hangar is in vacuum. (b) Launch bays only. | (a) | none: recommendation taken (ask only with screenshots) |
| H5 | May flight ops override the launch hold? | (a) Yes, a combat drop with a predicted clearance shown first. (b) Never. | (a) | none: recommendation taken (ask only with screenshots) |
