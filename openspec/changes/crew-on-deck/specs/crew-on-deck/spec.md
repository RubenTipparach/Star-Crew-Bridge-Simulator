# Crew on Deck

## Purpose

The crew body in the ship's interior frame: how it moves, climbs and passes doors, what it can
use and carry, how it is hurt, downed and revived, what the air, zero gravity and the dampers'
residual do to it, how long the Tern takes to cross on foot, and the avatar's geometry, animation
and network form.

## ADDED Requirements

### Requirement: Bodies move in the interior frame by one function
Every crew body SHALL move in the ship's interior frame by one `sc-core` movement function,
stepped at the server's 30 Hz tick and replayed unchanged by the client's prediction of its own
body. The ship's motion SHALL reach a body only through the dampers' residual and the hit shake
of `ship-frames`.

#### Scenario: The ship turns hard with the dampers at full power
- **WHEN** the ship yaws at 18 deg/s with the dampers at full supply and health
- **THEN** a body standing on the bridge does not move relative to the deck

#### Scenario: The same input on both ends
- **WHEN** the server and the client step a body from the same state with the same input frames, deck and door states
- **THEN** both arrive at the same position to within 1 mm

### Requirement: The crew capsule fits every crew portal
A body SHALL collide as a vertical capsule of radius 0.30 m and height 1.80 m standing, 1.20 m
crouched and 1.30 m seated, and 0.35 m and 1.90 m suited, from `data/crew.json`. The layout check
SHALL refuse a layout in which a door, pressure door or ladder trunk is narrower than the
capsule's diameter plus 0.1 m, or a door or pressure door lower than the standing height, for
the standing, suited and casualty-carrying body.

#### Scenario: A door too narrow
- **WHEN** a layout edit makes `p_quarters` 0.6 m wide
- **THEN** the layout check fails and names `p_quarters`, its clear width and the 0.7 m it needs

#### Scenario: The side pod hatch
- **WHEN** a body uses `p_pod_port`, which is 1.4 m high with a 0.40 m sill
- **THEN** it passes by the 1.5 s climb-through clip and comes out in the pod

### Requirement: Walking and running speeds come from data and traction
A body SHALL walk at 1.8 m/s, run at 4.0 m/s while the run input is held, move at 0.9 m/s
crouched and at 70 % of its speed on stairs, step up at most 0.35 m, and accelerate and brake at
no more than the lesser of 6.0 m/s^2 and its grip less the residual's horizontal part, where grip
is 0.6 times the felt gravity. There SHALL be no jump.

#### Scenario: From standing to a run
- **WHEN** a healthy body at rest on a level deck at full gravity starts running
- **THEN** it reaches 4.0 m/s after 0.68 s

#### Scenario: Low gravity
- **WHEN** the gravity generator delivers 4.9 m/s^2 and a body starts walking
- **THEN** it reaches 1.8 m/s after about 0.61 s, twice the time at full gravity

### Requirement: Ladders are climbed at stated speeds with free hands
A body SHALL get on and off a ladder in 0.5 s each, climb up at 0.8 m/s and down at 1.0 m/s, and
pass through an intermediate deck without getting off. A body holding a two-handed load or a
trolley SHALL NOT get on a ladder; a body carrying a casualty SHALL climb at half speed.

#### Scenario: Deck A to deck C
- **WHEN** a body climbs down the trunk at z = 11 m from deck A to deck C
- **THEN** the climb takes 8.0 s

#### Scenario: A patch kit at the ladder
- **WHEN** a body holding a patch kit uses the ladder
- **THEN** it is refused and the body stays on the deck

### Requirement: Doors open on approach and never close on a body
An ordinary door SHALL open in 0.6 s when a body comes within 3.0 m of it, unless it is locked or
held by the pressure interlock, and SHALL close 2.0 s after its zone is empty. A door or hatch
SHALL NOT close while any body is inside its opening, including when a breach orders it shut. A
pressure door SHALL open only on use, in 2.0 s.

#### Scenario: Running through a door
- **WHEN** a body runs at 4.0 m/s toward a closed, unlocked door between two pressurized compartments
- **THEN** the door is open before the body reaches it and the body does not slow

#### Scenario: Standing in the doorway during a breach
- **WHEN** the mess is breached and its door is ordered shut while a body stands in the doorway
- **THEN** the door stays open until the doorway is clear, and then closes

### Requirement: Nothing opens across a pressure difference
A door, hatch or pressure door SHALL refuse to open while more than 5 kPa lies across it. A body
at an ordinary door or hatch SHALL be able to override the refusal by holding use for 3.0 s when
the difference is at most 30 kPa, and the override SHALL be logged on the damage control board. A
pressure door SHALL have no override and SHALL open only in its own cycle.

#### Scenario: Vacuum beyond the door
- **WHEN** a body uses the forward switchboard's door with 101 kPa on its side and 0.4 kPa beyond
- **THEN** the door stays shut, the panel shows both pressures, and no override is offered

#### Scenario: Equalizing a smoky compartment
- **WHEN** a body holds use for 3.0 s at a door with 12 kPa across it
- **THEN** the door opens, the air flows as `life-support` computes, and the damage board logs the override

### Requirement: Use is checked by the server within reach
A body SHALL use a thing only when its eye is within 1.5 m of the thing's use point with a clear
line through the deck's brushes, the body is not downed, climbing or floating without a hold, and
its hands suit the use. The server SHALL decide every use; the client SHALL show the pending
result at once.

#### Scenario: Too far away
- **WHEN** a player uses an extinguisher bracket 2.1 m from the body's eye
- **THEN** the use is refused and nothing changes

### Requirement: A body holds one thing
A body SHALL hold at most one item, casualty or trolley. An extinguisher or repair kit SHALL leave
movement unchanged; a patch kit or a casualty SHALL limit the body to 1.2 m/s with no running. A
body that is knocked down or downed SHALL drop what it holds.

#### Scenario: Knocked down with an extinguisher
- **WHEN** a body spraying an extinguisher is knocked down by the residual
- **THEN** the extinguisher falls to the deck beside it with the agent it had left

### Requirement: The missile trolley stays in its rooms
A loaded trolley SHALL be pushed with 300 N against rolling resistance of 0.01 times its weight
and SHALL move no faster than 0.8 m/s. It SHALL brake at 1.0 m/s^2 and hold whenever nobody pushes
it, hold against a residual up to twice grip, and never leave the compartment it belongs to.

#### Scenario: Letting go
- **WHEN** a body releases a trolley rolling at 0.8 m/s
- **THEN** the trolley stops within 0.8 s

### Requirement: Downed bodies are revived by hand within a window
A body SHALL be downed at 0 HP and SHALL have a 120 s stabilize window, shortened by 1 s for each
HP of damage taken while down. A body with empty hands within 1.2 m that holds use for 5.0 s
SHALL revive it at 25 HP. A body whose window has run out SHALL be critical and SHALL be revived
only on a medbay bed. A downed body SHALL NOT block a doorway or a corridor.

#### Scenario: A quick revive
- **WHEN** a teammate holds use for 5.0 s beside a body downed 30 s earlier in clean air
- **THEN** the body gets up at 25 HP and can walk but not run

#### Scenario: Down in the smoke
- **WHEN** a downed body lies in smoke that costs it 1.5 HP/s
- **THEN** its stabilize window runs out after 48 s, and it becomes critical

### Requirement: Every body down ends the mission
The mission SHALL end as lost when every body aboard the ship and its craft, players' and NPC
bodies alike, is downed or critical. No body SHALL die permanently; a body critical at the end of
a mission SHALL carry a campaign injury that lowers its maximum HP to 80 until the next resupply.

#### Scenario: The last body falls
- **WHEN** the last standing body aboard is downed while every other body is downed or critical
- **THEN** the mission ends as lost

### Requirement: Medbay beds heal
A body on a medbay bed SHALL heal at 2.0 HP/s while the beds' load is powered and 0.5 HP/s while
it is not, and SHALL stop taking damage from hypoxia, carbon dioxide and cold. A downed or critical
body placed on a bed SHALL be revived at 25 HP after 20 s.

#### Scenario: From a revive to full health
- **WHEN** a body at 25 HP lies on a powered bed
- **THEN** it reaches 100 HP after 37.5 s

### Requirement: The air acts on a body by name
A body SHALL suffer hypoxia, hypercapnia, smoke, cold, heat, low pressure and vacuum exposure at
the thresholds and rates in `data/crew.json`, reading the values `life-support` computes for the
compartment it is in, and a body within 4 m of a breach in a venting compartment SHALL be pushed
along the flow at up to 3 m/s unless it holds a handhold or is braced. Rates SHALL add when
effects stack, and the client SHALL name each active effect.

#### Scenario: Thin air
- **WHEN** a body stands in a compartment whose O2 partial pressure is 10 kPa
- **THEN** it cannot run, walks at 75 % speed, loses 0.5 HP/s, and its status line reads "Hypoxia"

#### Scenario: Vacuum without a suit
- **WHEN** a healthy unsuited body is in a compartment at 0.5 kPa
- **THEN** it is downed in under 10 s

### Requirement: EVA suits protect for a stated time
A body SHALL don a suit at a locker in 20 s and doff it in 10 s. A suited body SHALL carry 1,800 s
of oxygen, move at 1.5 m/s without running, and be protected from vacuum, hypoxia, carbon
dioxide, smoke and cold, taking fire damage at half rate. A hit costing a suited body more than
20 HP SHALL puncture the suit, leaving 120 s of oxygen until it is patched.

#### Scenario: Into the vacuum
- **WHEN** a suited body with 1,800 s of oxygen enters a compartment at 0.5 kPa
- **THEN** it takes no damage from the air and its oxygen falls by 1 s each second

### Requirement: Zero gravity is crossed by handholds
When felt gravity falls under 2 m/s^2 a standing body SHALL float. It SHALL kick off a surface it
touches at up to 2.0 m/s, move hand over hand along a handhold at 1.5 m/s, and accelerate at
0.1 m/s^2 when nothing is in reach. A floating body striking a surface faster than 3 m/s SHALL
take 5 HP per m/s above 3. A seated body SHALL stay seated.

#### Scenario: The generator fails
- **WHEN** the gravity generator loses power while a body walks the main corridor
- **THEN** the body floats, and holding grab takes the handhold rail beside it

### Requirement: The dampers' residual moves a body by the ship-frames table
A body SHALL respond to its compartment's residual as `ship-frames` section 4 proposes: a camera
lurch from 0.3 m/s^2, a stumble above grip, a knockdown above twice grip, with standing crew
stumbling on hit shake trauma over 0.7. A braced or climbing body SHALL stumble only above twice
grip and be knocked down only above three times grip, and a seated body SHALL only feel it in the
camera.

#### Scenario: Braced for the burn
- **WHEN** the residual on the bridge is 14 m/s^2 at full gravity with one body braced at a console and one standing
- **THEN** the standing body is knocked down and drops what it held, and the braced body stumbles but stays up

### Requirement: Key routes stay within their walk times
The walk times computed from the layout and `data/crew.json` SHALL keep every station within
30 s of a launch bay's pressure door at walking speed, every core station within 20 s of the
quarters, and the engineering bay console within 40 s of the helm by each of two routes.
`deck-pipeline`'s walkable check SHALL fail while any station cannot be reached.

#### Scenario: As laid out today
- **WHEN** the walk times are computed from the Tern's layout without `reference-ship-tern`'s patches T1 and T2
- **THEN** the engineering bay console has no route from the helm and the check fails, naming the engineering mezzanine

#### Scenario: With the patches
- **WHEN** T1 and T2 are applied
- **THEN** the helm reaches the engineering bay console in 33.0 s by the aft passage and 35.4 s through the hangar

### Requirement: The avatar fits the Pi 5 budget
A crew body SHALL be at most 3,000 triangles at its nearest level of detail with at most 48
bones, drawn in one draw call and skinned in the vertex shader, and its motion SHALL come from
baked clips, with procedural motion only layered on top. The build SHALL refuse an avatar over
its budget.

#### Scenario: An avatar over budget
- **WHEN** a body mesh of 3,400 triangles is built
- **THEN** the build fails and names the mesh and its triangle count

### Requirement: The client predicts its own body and nothing it does not decide
The client SHALL predict its own body's walking, running, crouching, stairs, ladders, zero
gravity movement and doors opening on approach through the movement function, and SHALL NOT
predict the outcome of a use, a revive, a stumble, a knockdown or damage. A correction under 5 cm
SHALL blend over 100 ms and a larger one SHALL snap, as `netcode-and-sessions` sets.

#### Scenario: A door the client thought was open
- **WHEN** the client predicts its body through a door that the server had locked 50 ms earlier
- **THEN** the next snapshot corrects the body to the server's position on the near side of the door
