# Damage Control

## Purpose

What a hit does to the ship and what the crew do about it: one hit resolution from the hull inward to
compartments, systems, conduits, doors and crew; system damage states; fire and its suppression; the
magazine's cook-off; repairs with kits, parts and plates; damage control teams; and the engineering
bay's hands-on controls.

## ADDED Requirements

### Requirement: One hit resolution for every ship
Every hull hit that passes the shields SHALL be resolved by one function: the hull section (span and
face) absorbs up to 4 MJ times its integrity and loses `min(E, 4 MJ) / 40 MJ x 100` points; the rest
marches inward along the hit's direction in 0.25 m steps, losing energy with a 4 m decay length and
1.5 MJ at each bulkhead, depositing it at each step; the first compartment entered is breached with
an area of 0.02 m^2 per MJ arriving at its skin, between 0.002 and 2.0 m^2; every load, power node,
conduit and door within `1.5 m + 0.3 m x sqrt(E)` of a step takes `10 points per MJ x dE x (1 - d /
r)`. The same function SHALL serve the Tern, enemy ships and craft with their own data.

#### Scenario: A Gannet on the main switchboard
- **WHEN** a 60 MJ hit strikes engineering's port side at deck B height toward the main switchboard
- **THEN** the hull section keeps 90%, engineering is breached by about 0.6 m^2, the port switchboard
  section is destroyed, the starboard section 15 m away keeps about 98%, and the port drive feeder
  and port main trunk are severed

#### Scenario: Pulse bolts against fresh armour
- **WHEN** a 1.2 MJ bolt strikes a hull section above 30% integrity
- **THEN** the armour absorbs it all and nothing inside is damaged

### Requirement: Hits are deterministic
Every chance in a hit (fire ignition) SHALL be decided by a hash of the session seed, the hit's id and
the compartment's id, so the same hit on the same ship state always has the same result.

#### Scenario: Replaying a hit
- **WHEN** the same hit is resolved twice on identical ship states with the same seed
- **THEN** both reports name the same breach, damage, severed conduits and fires

### Requirement: Systems have four damage states
A system's integrity SHALL give its state and capability: nominal at 75% or more (capability 1),
damaged from 25% to 75% (integrity / 75), disabled below 25% (0), destroyed at 0% (0, rebuild
needed). Integrity SHALL fall from hits, from temperatures above the system's damage temperature,
from overdrive, and from fire in its room. Power nodes SHALL be destroyed below 0.25 health.

#### Scenario: A damaged cradle
- **WHEN** the port launch cradle is at 34% integrity
- **THEN** it is damaged, works at a capability of about 0.45, and its release charge takes about
  2.2 times as long

### Requirement: Fire grows from fuel and oxygen and damages its room
A fire SHALL grow as a t-squared fire (0.047 kW/s^2) toward a ceiling of 250 kW per m^2 of floor
times an oxygen factor that is zero below 13% oxygen or 20 kPa; consume 1 mol of oxygen per 0.419 MJ,
making 0.67 mol of CO2 and smoke (more when starved); heat its room's air; go out below 5 kW; spread
to a room whose air passes 300 C or through open doors by hot air. Above 150 C, systems and power
nodes in the room SHALL lose 0.01% of integrity a second per kelvin over, and the reactor a fifth of
that.

#### Scenario: A fire caught late
- **WHEN** a 50 kW fire starts in the quarters and one extinguisher is used 120 s later
- **THEN** the fire is not stopped, and only two extinguishers at once put it out

#### Scenario: A sealed room's fire
- **WHEN** a fire starts in the medbay with its door shut and nobody acts
- **THEN** it peaks above 2 MW, the air passes 300 C, a crew member inside dies within about
  2.5 minutes, and the fire goes out for want of oxygen within about 4.5 minutes

#### Scenario: An unattended switchboard fire
- **WHEN** a fire starts in the forward switchboard and nobody acts
- **THEN** both forward switchboard sections and the emergency bus are destroyed within about
  3 minutes

### Requirement: Suppression has stated effects
An extinguisher SHALL cut a fire's heat release by 60 kW a second for 15 s (two together, 120 kW a
second); water mist in engineering, the hangar and the drive SHALL discharge automatically after a fire
has exceeded 1 MW for 10 s (inhibitable) and cut 100 kW a second with 1 MW of cooling for 60 s;
inert gas SHALL shut the room's doors and damper and bring its oxygen to 12.5% within 60 s; venting
SHALL take the room's air overboard through the duct.

#### Scenario: Water mist in engineering
- **WHEN** a 1 MW fire starts in engineering with the mist armed
- **THEN** the mist discharges after 10 s and the fire is out about 22 s after it started

#### Scenario: Inert gas in the magazine
- **WHEN** a fire starts in the magazine and inert gas is discharged 30 s later
- **THEN** the fire is out by about 130 s and the magazine's air never reaches 50 C

### Requirement: The magazine can cook off
A missile in the magazine's racks SHALL cook off when the magazine's air has been above 200 C for
60 s, or a hit deposits 10 MJ within 2 m of the racks, and SHALL be resolved as a 60 MJ hit from
inside at the racks by the one hit resolution; the next missile SHALL check again 30 s later.

#### Scenario: A magazine fire left alone
- **WHEN** a fire burns in the magazine with nobody acting
- **THEN** the air passes 200 C at about 207 s and the first missile cooks off about 60 s later

### Requirement: Repairs need people, tools and time
Repairs SHALL be done at the damage: a kit restores 1% of integrity a second for a player and 0.6%
for a team member (two together 1.6 times one); a disabled system needs a spare part first; a
destroyed system needs 3 parts and a 180 s rebuild to 25%; a destroyed power node 4 parts and 120 s
to half health; a severed conduit 1 part and a 30 s splice to half capacity; a breach up to 1 m^2
plates from inside at 15 s per 0.25 m^2, larger ones EVA at 60 s per m^2. The board's time to repair
SHALL be computed by the repair's own rates plus the walk and any suiting.

#### Scenario: Restoring power after the switchboard hit
- **WHEN** a suited team rebuilds the destroyed port switchboard section and splices the port trunk
- **THEN** the section returns at half health after 120 s and the reactor's full 48 MW is available
  again after the 30 s splice

### Requirement: Damage control teams are crew bodies
Two teams of two NPC crew bodies SHALL take jobs from the damage control board or its automation,
dispatching after 5 s, walking at 1.6 m/s (running at 4.0 m/s to a fire or a breach with crew in it),
suiting for 30 s before entering a room below 50 kPa or a fire over 2 MW, repairing at 0.6% a second
each, and SHALL suffer the same harm as players. Automation SHALL never vent a room with crew inside
and never open a door across its interlock.

#### Scenario: A team sent into vacuum
- **WHEN** the board sends a team to engineering at 18 kPa
- **THEN** the team suits up before entering, arriving about 65 s after the job is assigned

### Requirement: The engineering bay is worked by hand
Scram reset, the manual throttle and the coolant branch valves SHALL be worked at the reactor panel
and the valve manifold in engineering. Breakers, dampers, doors and suppression SHALL be commanded
remotely only while the computer core has at least half its supply, and otherwise only at the
switchboards, doors and systems themselves.

#### Scenario: The computer core lost
- **WHEN** the computer core falls below half its supply and engineering on the bridge tries to close
  a breaker
- **THEN** the command is refused as "no remote control", and the breaker can be closed at its
  switchboard section
