# patrol-mission

## ADDED Requirements

### Requirement: The patrol starts docked at the starbase

The patrol mission SHALL start with the Tern berthed in the round bay of Tamsin Station, the O'Neill pair, on shore
power with its impulse
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
- **WHEN** the spool completes with the bow within 5 degrees of the first stop, Cinder's ore lanes
- **THEN** the Tern arrives within 1.5 km of the lanes' arrival point moving as it left

### Requirement: The patrol tours the system's planets and clears each stop

The patrol SHALL visit Cinder's ore lanes, the Pell beacon at Orrin and the Shoals relay in that order, each with its
hostile group: three Jackals with no carrier at Cinder, found by science's sweep; the Hound and its two Jackals at the
Pell beacon, placed from the Tern's arrival as the drill places them from its start; a raider and two Jackals at the
relay. A stop SHALL be clear when every hostile of its group is destroyed and a sweep there shows none within 50 km.

#### Scenario: The pack at Cinder
- **WHEN** the Tern arrives at Cinder's ore lanes and science sweeps
- **THEN** three Jackals appear on the plot and attack, and the stop is clear once all three are destroyed and a
  second sweep shows no hostile

#### Scenario: The drill's fight at the Pell beacon
- **WHEN** the Tern arrives at the Pell beacon
- **THEN** the Hound is 6 km ahead, its fighters launch 20 s later, and the Tern cannot jump while the Hound is alive
  within 5 km

#### Scenario: Moving on uncleared
- **WHEN** the captain orders the next jump with fighters of the stop's group still alive and no hostile ship within
  5 km
- **THEN** the jump spools, and the stop's objective stays open

### Requirement: The ship carries its state from stop to stop

Nothing aboard the Tern SHALL be reset by a jump or a new fight: its hull and systems' damage, its fires, its missiles,
its power and its crew SHALL carry from one stop to the next.

#### Scenario: Damage at Cinder, at Pell
- **WHEN** the Tern leaves Cinder with its port shield emitter at half integrity, a fire in the mess and six missiles
  fired
- **THEN** it arrives at the Pell beacon with the same emitter at half, the fire still burning and six missiles fewer

### Requirement: The patrol ends on arrival home

The Tern SHALL be able to jump home after the last stop or between stops, and arriving at Tamsin Station's arrival
point SHALL end the mission: PATROL COMPLETE when every stop was cleared, PATROL INCOMPLETE naming the stops cleared
when not. The Tern's hull at zero SHALL end it at any stop: THE TERN IS LOST.

#### Scenario: Home after clearing every stop
- **WHEN** all three stops are clear and the Tern arrives home
- **THEN** every screen shows PATROL COMPLETE with the patrol's objectives marked

#### Scenario: Home early
- **WHEN** the captain takes the Tern home after clearing Cinder only
- **THEN** the end screen shows PATROL INCOMPLETE, one of three stops cleared, Cinder

### Requirement: Every step of the patrol is played on foot

Every action of the patrol SHALL be taken at a station's console reached by walking the body to it, by players and
bots alike, and each bot SHALL say what it does in the crew chat.

#### Scenario: The engineering bot starts the drive
- **WHEN** a bot holds engineering in the docked phase
- **THEN** it walks to its station, closes the drive's breaker and drops shore power, and says "Off shore power", and
  the ship's computer reports "Drive on line"
