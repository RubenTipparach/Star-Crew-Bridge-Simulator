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
