# playtest

## ADDED Requirements

### Requirement: The seat tour sits and stands at every seat

`--playtest-seats` SHALL walk the scripted player to every station seat aboard in turn, sit, stand and walk on, and
log for each seat whether it sat and stood.

#### Scenario: The gunner's pod
- **WHEN** the tour reaches the gunner's seat at the top of the dorsal pod's ladder
- **THEN** it climbs up, sits, stands, and climbs back down to walk to the next seat

### Requirement: The retreat playtest takes damage, runs and repairs

`--playtest-retreat` against the Mastiff SHALL fight at the helm until hit, turn about and run at full speed, then put
out the fires and play the repair games on foot.

#### Scenario: After the retreat
- **WHEN** the Tern has opened the range with fires aboard
- **THEN** the scripted player leaves the helm, takes an extinguisher, puts out the fires and plays the repair games

### Requirement: A bot that takes the helm in a fight holds the course it found

A bot SHALL hold the heading and speed of a helm it sits at during an engagement for the rest of that round, sending
no steering and acknowledging no order it is not flying; a bot seated at the helm from the round's start SHALL fly
as its profile says.

#### Scenario: The player leaves the helm mid-retreat
- **WHEN** the player has turned the Tern away at full speed and stands up, and a bot takes the helm
- **THEN** the Tern keeps its heading and speed, and the bot neither turns onto the hostile nor says it will

### Requirement: The retreat does not leave the helm before its run is done

Seated in the retreat, the scripted player SHALL NOT stand for a fire before the run beat ends, and a scripted
recorded run SHALL end 8 s into the Debrief.

#### Scenario: A fire during the run
- **WHEN** a fire starts while the Tern is running at full speed
- **THEN** the scripted player stays at the helm until the run beat ends, then goes to the fire
