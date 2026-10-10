# Power Grid

## Purpose

The ship's energy system: a fusion reactor and a battery feeding switchboards, buses, conduits and
panels to forty loads, allocated by engineering's setpoints and priorities through one solve, with
heat carried away by a coolant loop and radiators, and every console readout and preview coming
from that solve.

## ADDED Requirements

### Requirement: Ship systems step at 10 Hz inside the 30 Hz tick
The server SHALL step the ship systems (power, heat, atmosphere, fire, crew) in sub-steps of 0.1 s,
one every third 30 Hz tick, in the fixed order: automation, power, heat, life support and fire,
scram checks. Simulation time SHALL be counted in seconds, never in frames.

#### Scenario: A minute of ship time
- **WHEN** the server runs 1,800 ticks at 30 Hz
- **THEN** the ship systems have run exactly 600 sub-steps and their clock reads 60.0 s

### Requirement: Power flows only where sources and paths allow
Each sub-step SHALL solve power as a flow from the reactor (at most 60% of its thermal output) and
the battery (at most 30 MW and its usable charge) through generators, ties, conduits and the
battery link, each limited by its capacity, to loads at their nodes. A severed conduit, an open
breaker, or a destroyed node at either end SHALL carry nothing.

#### Scenario: A cut trunk with a parallel path
- **WHEN** in combat the port main trunk midships (`k_mid_p`) is severed
- **THEN** every load keeps its supply, carried by the starboard trunk and the cross-ties

#### Scenario: Both trunks cut
- **WHEN** both midships trunks are severed in combat
- **THEN** the forward ship receives at most the command deck's aft feeder (8 MW) plus the battery,
  and once the battery reaches its reserve the turrets, sensors and gravity generator drop out

### Requirement: Shortfalls are shed by priority, then shared in proportion
When demand exceeds supply, the solve SHALL serve priority classes in order (0 vital, 1 crew, 2
fighting, 3 moving and support), within a class first from the reactor and then from the battery,
and within a class SHALL give each load the same fraction of its demand unless a path limits it. A
load that receives less than its minimum ratio SHALL switch off and the solve SHALL run once more
without it.

#### Scenario: Combat over budget
- **WHEN** the combat preset wants 56.2 MW, the reactor gives 48 MW and the battery has reached its
  reserve
- **THEN** priorities 0-2 are fully served and priority 3 shares the remainder, the impulse drive
  getting about a third of what it wants

#### Scenario: All out
- **WHEN** every activity is at full with setpoints at 100% (73.6 MW wanted)
- **THEN** priority 2 takes 43 MW and every priority 3 load drops out, the impulse drive included

### Requirement: Engineering allocates by setpoint, priority and preset
Engineering SHALL set each load group's setpoint from 0% to its maximum (150% for most) in 5%
steps and its priority from 1 to 3; the vital class 0 SHALL be fixed. The presets cruise, combat,
silent and emergency SHALL set every group's setpoint from the data. A setpoint above 100% SHALL add
heat in proportion to the excess and wear the load at 2% of integrity a minute at 150%.

#### Scenario: Overdriving the turrets
- **WHEN** the turrets are set to 150% and fire continuously
- **THEN** each draws up to 6 MW, puts out half again as much heat as at nominal, and reaches its
  damage temperature in under 4 minutes

### Requirement: The reactor follows the load it can deliver
In automatic mode the reactor's throttle target SHALL be the power the grid can take from it (a
solve with the reactor at its rated output and the battery held back) plus the battery's charge
want, divided by 48 MW, between 10% and 100%, ramping at 2% a second up and 10% down. In manual mode
a player SHALL set 10-120%, and power the grid cannot take SHALL heat the blanket.

#### Scenario: A grid that cannot take the reactor's output
- **WHEN** both midships trunks are cut and the grid can take 27 MW from the reactor
- **THEN** the automatic throttle settles near 56% instead of 100%, and the blanket stays below its
  warning temperature

### Requirement: Scram has four causes and a hands-on reset
The reactor SHALL scram when the coolant loop exceeds 380 K for 2 s, the blanket exceeds 820 K,
neither auxiliary train has 80% of
its supply for 2 s, or its integrity is under 25%. Only the reactor panel in engineering SHALL reset
a scram, by a 3 s hold, refused with its reason while the loop is at or above 350 K, the blanket at
or above 700 K, or the reactor is damaged. After a reset the reactor SHALL ignite for 20 s on 8 MW,
at the pace its auxiliary trains' supply allows, then run from 10% throttle.

#### Scenario: One switchboard section lost
- **WHEN** the port main switchboard section is destroyed while the reactor runs
- **THEN** the reactor keeps running on the starboard auxiliary train

#### Scenario: Reset refused while hot
- **WHEN** the reactor scrammed on loop over-temperature and the engineer holds RESET with the loop
  at 365 K
- **THEN** the panel refuses with "loop too hot (365 K)" and the reactor stays scrammed

### Requirement: The battery keeps a restart reserve
The battery SHALL hold 1,800 MJ, give at most 30 MW and take at most 12 MW at 95% efficiency each
way, charge only from reactor surplus while not discharging, and SHALL keep 350 MJ that only
emergency lighting, the reactor auxiliaries while igniting and the coolant pumps while the reactor
is not running may spend, unless engineering releases it.

#### Scenario: Restart after the worst cool-down
- **WHEN** the reactor scrams from loop over-temperature with the radiators at 60% health and the
  battery at its reserve
- **THEN** the coolant pumps run from the reserve until the loop is below 350 K, the reset is
  accepted, and ignition completes without the battery running empty

#### Scenario: Scram in combat
- **WHEN** the reactor scrams during combat with the battery full
- **THEN** the battery gives 30 MW, priority 2 runs at about two thirds, priority 3 drops, and the
  battery reaches its reserve within about 20 s

### Requirement: Every delivered megawatt becomes heat
Each load's delivered power SHALL go, by its heat route, to the coolant loop, its own thermal mass
(cooled by the loop and leaking to its room), a room, the supply air or the lights it powers, or
SHALL leave as work; the reactor's thermal output not delivered SHALL heat its blanket. The loop
SHALL reject heat through radiators whose capacity follows `40 MW x health x flow x (T^4 - 4^4) /
(330^4 - 4^4)`, throttled by a bypass that holds the loop between 325 K and 330 K when heat is low.
A system above its damage temperature (393 K) SHALL lose integrity at 0.01% a second per kelvin
over.

#### Scenario: Cruise
- **WHEN** the ship cruises for ten minutes
- **THEN** the loop holds at about 328 K with the radiators using about 24 of their 39 MW

#### Scenario: Combat heat
- **WHEN** the combat preset runs for ten minutes with the battery at its reserve
- **THEN** the radiators saturate and the loop climbs past 350 K toward its 360 K warning

### Requirement: Readouts and previews come from the solve
Every power readout on a console SHALL be the state the last sub-step produced, and every preview
(an allocation ghost bar, a breaker's effect, the heat projection) SHALL be computed by the same
solve and heat functions on a copy of the state. No console SHALL compute power by a formula of its
own.

#### Scenario: A preview is what happens
- **WHEN** engineering drags the shields' setpoint to 150% and the ghost bar shows 18.00 MW, then
  releases it
- **THEN** the next sub-step delivers 18.00 MW to the shield generator

### Requirement: Lighting follows the panels
A compartment SHALL show its normal (or red-alert) lighting while its lighting panel receives at
least half its demand, its emergency lighting while the emergency lighting load on the emergency
bus receives at least half, and be dark otherwise.

#### Scenario: A panel loses both feeds
- **WHEN** the port trunk is cut aft and midships
- **THEN** the hangar and the port launch bay, lit by the hangar's port panel, show emergency
  lighting while the rest of the ship keeps its normal or red-alert lighting

### Requirement: The solve is deterministic and cheap
The solve SHALL use no randomness and only the data's orders (loads, nodes, edges, adjacency), so
the same state and inputs give the same allocation on the same build. One whole systems sub-step
SHALL fit within 0.25 ms on one Cortex-A76 core of a Pi 5, and its state within 64 kB per ship,
both measured by the engine probe.

#### Scenario: Two identical runs
- **WHEN** the same ship state and the same commands are stepped twice for 600 sub-steps
- **THEN** every load's allocation is identical in both runs

### Requirement: Power data is validated at startup
`data/ships/<ship>/power.json` SHALL be loaded and validated at startup: an unknown key, a
non-finite number, a node, load, system or compartment id that does not exist, or a negative
capacity SHALL stop startup with the file and the field; a missing optional field SHALL take the
code default, and a present zero SHALL be zero.

#### Scenario: A misspelt key
- **WHEN** a conduit in `power.json` says `capacity_mws` instead of `capacity_mw`
- **THEN** the game refuses to start and names the file, the conduit and the unknown key
