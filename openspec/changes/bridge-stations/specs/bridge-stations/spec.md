# Bridge Stations

## Purpose

The stations a crew sits at, who operates each one (a player, automation, or a tab on another
console), how seats are claimed and released, how command works, what a seated player sees, and
the bridge as a room.

## ADDED Requirements

### Requirement: Stations come from the layout
The game SHALL build its stations from the ship layout's `stations` list and `data/stations.json`,
matched by id. A station id present in one and missing from the other SHALL stop startup with the
file and the id. No station position, facing or role SHALL be written in code.

#### Scenario: A station without tuning
- **WHEN** the Tern's layout lists `bay_control` and `data/stations.json` has no `bay_control` entry
- **THEN** startup stops and names `data/stations.json` and `bay_control`

### Requirement: The operator is never a body
Each station SHALL have an operator that is a player, automation or merged, held by the server
apart from the seat's occupant body. A body in a seat SHALL NOT operate a station unless a player
drives that body and has claimed the seat.

#### Scenario: Swapping away from the helm
- **WHEN** the player seated at the helm swaps into an NPC body in the mess while the helm stick is held hard to port
- **THEN** at the next tick the helm's operator is automation, its setpoints are the ship's current heading and speed, the ship stops turning, and the old body stays in the chair as an NPC body

#### Scenario: Relieving an NPC body
- **WHEN** a player uses the tactical seat while an NPC body sits in it
- **THEN** the NPC body stands and walks to the next station on its post list, the player sits, and the player is tactical's operator, all within 0.8 s

### Requirement: Releasing a station hands it to automation from its current state
When a station's player stands, disconnects, is downed, swaps body or is relieved, the station
SHALL pass to automation (or to its merge host as a tab) at the next 30 Hz tick, keeping every
setpoint the player set. A downed body SHALL leave the seat, so that a seat is never blocked by a
downed body.

#### Scenario: Downed at the engineering console
- **WHEN** the player at engineering is downed by smoke with the shields' power request raised to 9 MW
- **THEN** the body falls beside the seat, the seat is empty, engineering's automation takes over at the next tick, and the shields' request stays 9 MW

#### Scenario: Reconnecting
- **WHEN** a seated player disconnects and reconnects 60 s later
- **THEN** the player gets their body back and, since the body still holds the seat, the station

### Requirement: Seat claims are checked by the server
The server SHALL grant a seat claim only when the claiming body is standing within 1.5 m of the
seat point, is not downed, climbing or carrying a two-handed load, and the seat is empty, held by
an NPC body, or the station is automated. Two claims of one seat in one tick SHALL be resolved by
the command order of `netcode-and-sessions` (seat id, then arrival), and the loser SHALL receive a
refusal with a reason.

#### Scenario: Two players reach the helm at once
- **WHEN** two players claim the helm in the same tick
- **THEN** one sits and the other's console shows "refused: taken"

#### Scenario: Too far away
- **WHEN** a player 2.4 m from the science seat uses it
- **THEN** the claim is refused and nothing changes

### Requirement: Automation works at a stated, lower competence
Automation SHALL run every station that has no seated player, stepping at 10 Hz on the server,
issuing the same commands a console issues through the same validation. Its reaction times,
accuracy and limits SHALL come from `data/stations.json`. It SHALL maintain the setpoints players
set, and a manual setpoint SHALL win until a player or a change of condition replaces it.

#### Scenario: Automated turret against a fighter
- **WHEN** an automated turret engages a fighter 1 km away
- **THEN** it acquires after 0.8 s, tracks at half the mount's slew rate, and aims with a 4 mrad (1 sigma) error

#### Scenario: A manual setpoint survives automation
- **WHEN** the helm player opens the merged Engineering tab and raises the turrets' power request, and Engineering's automation then steps
- **THEN** the turrets' request stays where the player set it

### Requirement: Automation refuses the crew's decisions
Automation SHALL NOT fire missiles without a "missiles free" order or an order naming the target,
fly evasive patterns, ram, dock, choose a destination, enter a hazard, scram or vent, negotiate or
surrender, send a distress call unordered, ping actively, change shield frequency, launch a craft
unordered, or depressurize a bay with an unsuited person in it. An order that names the action
SHALL authorize it.

#### Scenario: Missiles held
- **WHEN** Tactical is automated, both tubes are loaded and a hostile frigate is in the tubes' arc
- **THEN** no missile is fired

#### Scenario: Missiles ordered
- **WHEN** the captain orders "Tactical: fire tube 1 at T3" while Tactical is automated and tube 1 is loaded with T3 in arc
- **THEN** automation acknowledges within 2.0 s and fires tube 1 at T3

### Requirement: Automation needs the computer core
Automation SHALL run at full competence while the computer core is powered and above 50 %
integrity, with doubled reaction times and turret aim error between 1 % and 50 %, and SHALL stop
deciding when the core is unpowered or destroyed, leaving every automated station holding its
setpoints passively.

#### Scenario: The core loses power
- **WHEN** the computer core's breaker trips with Helm and the turrets automated
- **THEN** the ship holds its heading and speed and the automated turrets stop firing until the core is powered again

### Requirement: Unmanned stations merge onto manned consoles
A station with no seated player SHALL appear as a tab on the first manned station in its merge
list from `data/stations.json`, while its automation keeps running. While no captain is seated,
the condition, orders and brace SHALL be available in the title band of every manned bridge
console.

#### Scenario: One player
- **WHEN** one player sits at the helm and no one else is aboard
- **THEN** the helm console shows tabs for Tactical, Engineering, Science, Comms, Flight ops and the Damage board, and the title band offers the condition

#### Scenario: Four players
- **WHEN** players sit at helm, tactical, engineering and science
- **THEN** Flight ops is a tab on Tactical, Comms a tab on Science, and the Damage board a tab on Engineering

### Requirement: Body swap never takes a player's body
When body swap is enabled, a player SHALL be able to swap into any NPC body aboard that is not
downed, with a cooldown from `data/stations.json` (10 s by default). Swapping into a seated NPC body
SHALL make the player that station's operator at the next tick. A player SHALL NOT swap into
another player's body.

#### Scenario: Swapping to a seat
- **WHEN** a solo player at the helm swaps into the NPC body seated at engineering
- **THEN** the player operates engineering at the next tick and the helm passes to automation

### Requirement: Condition and red alert
The ship SHALL have a condition of normal or red alert, set by the captain, by any seated bridge
player while no captain is seated, or by the auto-condition. Entering red alert SHALL, at the next
tick, blend every powered compartment to its red alert lighting over 0.5 s, sound the klaxon, apply
the Combat preset to automated Engineering, and make automated Tactical raise shields. A
compartment whose lighting load is unpowered while the emergency bus is up SHALL show emergency
lighting whatever the condition.

#### Scenario: Red alert called
- **WHEN** the captain holds red alert for 0.6 s
- **THEN** within 0.5 s every powered compartment's lighting is the red alert set and every console's condition chip reads RED ALERT

#### Scenario: Auto-condition
- **WHEN** no captain is seated and a contact identified hostile comes within 15 km
- **THEN** the condition becomes red alert, and it returns to normal only 60 s after the last hostile is beyond 25 km

### Requirement: Orders have a lifecycle
An order SHALL name a station, a verb from that station's list in `data/stations.json` and an
object. It SHALL be shown on the recipient's title band and be acknowledged, refused as unable with
a reason, done, superseded by a newer order to the same station, or expired after 60 s. An
automated recipient SHALL answer within its reaction time using `automation::can_execute`, the
same function that greys the order in the composer before it is sent.

#### Scenario: An order automation cannot follow
- **WHEN** the captain composes "Helm: dock with Station K" while Helm is automated
- **THEN** the composer shows it greyed with the reason "cannot comply", the same answer automation would give

### Requirement: Consoles use a fixed grid with a look band
Every console SHALL be laid out on a 1280 x 720 logical canvas with a 32 lp title band, a 240 lp
look band showing the 3D view from the seat toward the viewscreen, a 12 by 4 panel grid
(columns 96 lp, rows 94 lp, gutters 8 lp, from x 20 and y 280), and a 32 lp status strip. Panels
SHALL be fixed in size, SHALL NOT overlap, and SHALL fill the grid; text SHALL be clipped with an
ellipsis and lists SHALL scroll inside their panel. A validator SHALL check every
`data/consoles/*.json` against these rules.

#### Scenario: An overlapping panel
- **WHEN** a console file places a 5-column panel at column 8
- **THEN** the validator fails and names the console and the panel

#### Scenario: A long name
- **WHEN** a contact's name is wider than the Targets panel's row
- **THEN** it is clipped with an ellipsis and the panel keeps its size

### Requirement: A console's preview is computed by the resolver
Every preview a console shows (a power request's delivered MW, a turn's time, a turret's hit
chance, a scan's time, a pump-down time, a repair time, whether an order can be executed) SHALL be
computed by the same `sc-core` function the server uses to resolve that action, applied to the
client's replicated state.

#### Scenario: A breaker preview
- **WHEN** the engineer hovers over the port main bus breaker
- **THEN** the allocation bars show the delivered MW `power::solve` returns with that breaker open, and opening it produces those values on the server

### Requirement: Guarded controls
Firing a missile, scram, opening a main bus breaker, venting, opening a drop door, depressurizing a
bay, an active ping, red alert, a distress call and a launch SHALL require holding the control for
0.6 s, or arming it and confirming within 3 s.

#### Scenario: A brushed key
- **WHEN** the tactical player taps the fire key for 0.2 s with tube 1 ready
- **THEN** nothing is fired

### Requirement: Every input device drives every console
A seated player SHALL be able to operate every control of their console and its tabs with a
keyboard and mouse or with a gamepad, whichever they are using, with bindings from
`data/input/bindings.json`.

#### Scenario: Engineering on a gamepad
- **WHEN** the engineer uses a gamepad
- **THEN** the D-pad selects load groups and breakers, left and right change a request by 0.5 MW, and holding A for 0.6 s opens a guarded breaker

### Requirement: The viewscreen shows the exterior from the ship's pose
The bridge viewscreen SHALL show the exterior scene from the selected feed's camera, rendered into
one 1024 x 512 render target at up to 30 Hz, following the ship's exterior pose, while the bridge
interior does not move. Science SHALL choose the feed and zoom; the captain SHALL be able to
override it. A client SHALL skip the pass when none of its views shows the viewscreen. A console
SHALL show at most one secondary feed, rendered at 512 x 256 at up to 15 Hz, and only while that
console shows it.

#### Scenario: A craft camera on Flight ops
- **WHEN** the Flight ops player switches F4 to Swift 1's camera
- **THEN** that client renders one 512 x 256 view at up to 15 Hz into F4, and stops rendering it when F4 returns to its plot

#### Scenario: The ship turns
- **WHEN** the ship yaws 30 deg to port
- **THEN** the forward feed's picture pans by 30 deg and nothing in the bridge moves

#### Scenario: Off the bridge
- **WHEN** the local player is in engineering
- **THEN** that client renders no viewscreen pass

### Requirement: The bridge stays within its Pi 5 budget
The compiled bridge geometry SHALL be at most 30,000 triangles in at most 3 draw calls, and the
seated console UI at most 6,000 triangles in at most 16 draw calls. The deck build SHALL refuse a
bridge over its budget.

#### Scenario: Over budget
- **WHEN** a change to the bridge's furniture brings its compiled geometry to 31,200 triangles
- **THEN** the deck build fails and names the bridge and its triangle count
