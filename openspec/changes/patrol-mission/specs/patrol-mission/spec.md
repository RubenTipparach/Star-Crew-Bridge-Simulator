# patrol-mission

## ADDED Requirements

### Requirement: The patrol starts docked at the starbase

The patrol mission SHALL start with the Tern docked in its berth at Tamsin Station on shore power with its impulse
drive's breaker open, and the crew SHALL walk to their stations to bring the drive up before the Tern can leave.

#### Scenario: The drive must be up before the clamps open
- **WHEN** helm holds UNDOCK before engineering has closed the drive's breaker and dropped shore power
- **THEN** the clamps stay closed and the command is refused as not ready

### Requirement: Undocking flies the Tern out of the bay under its limits

Once the clamps open, the Tern SHALL leave Tamsin Station's bay by its berth's departure legs, up off the wall to the
axis and down the axis out through the throat, no faster than the bay's 5 m/s inside the station and the corridor's
40 m/s beyond it, and the jump SHALL be refused while the Tern is inside the station's mass lock.

#### Scenario: Leaving the berth
- **WHEN** helm releases the clamps
- **THEN** the Tern is clear of the pair about 4.4 minutes later and the undock objective is done

#### Scenario: Too close to jump
- **WHEN** the captain orders a jump from inside the station's throat
- **THEN** the spool does not start and helm's console shows the mass lock

### Requirement: The patrol travels between waypoints by the jump drive

The Tern SHALL travel between the system's points of interest by the jump drive of `flight-and-navigation` 7 (a 40 s
spool at 20 MW, the bow aligned for its last 3 s, mass locks, an instant transit, a 60 s cooldown), arriving at the
point's arrival point within 1.5 km.

#### Scenario: The first jump
- **WHEN** the spool completes with the bow within 5 degrees of the Pell beacon
- **THEN** the Tern arrives within 1.5 km of the beacon's arrival point moving as it left

### Requirement: The encounter is the drill's fight at the relay

On arrival at the Shoals relay the Hound SHALL be placed from the Tern's arrival as the drill places it from the
Tern's start, and the drill's Engage SHALL run with its numbers unchanged.

#### Scenario: Hostiles at the relay
- **WHEN** the Tern arrives at the Shoals
- **THEN** the Hound is 6 km ahead, its fighters launch 20 s later, and the Tern cannot jump while the Hound is alive
  within 5 km

### Requirement: The patrol ends on arrival home

After the encounter the Tern SHALL be able to jump home, and arriving at Tamsin Station's arrival point SHALL end the
mission: complete when the Hound was destroyed, escaped when it withdrew.

#### Scenario: Home after a victory
- **WHEN** the Hound is destroyed and the Tern arrives home
- **THEN** every screen shows MISSION COMPLETE with the patrol's objectives marked

### Requirement: Every step of the patrol is played on foot

Every action of the patrol SHALL be taken at a station's console reached by walking the body to it, by players and
bots alike, and each bot SHALL say what it does in the crew chat.

#### Scenario: The engineering bot starts the drive
- **WHEN** a bot holds engineering in the docked phase
- **THEN** it walks to its station, closes the drive's breaker and drops shore power, and says "Off shore power", and
  the ship's computer reports "Drive on line"
