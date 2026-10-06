# Hangar and Craft

## Purpose

How the Tern launches and recovers its craft through its launch bays and hangar, what flight ops
and bay control see and do, how bays fail and are fixed, and the Swift fighter and Petrel shuttle
themselves, flown by players or as drones.

## ADDED Requirements

### Requirement: A launch runs as a timed sequence behind interlocks
A Swift launch SHALL run the steps board (6 s), preflight (8 s, overlapping the pump-down),
pump-down (life-support's time), drop door open (4 s), cradle down 4.6 m (3.5 s) and release with a
4 m/s ejection along -Y. Each step SHALL wait for its interlock: the bay pressurized for an unsuited
pilot to board; nobody unsuited in the bay outside a sealed cockpit for the pump-down; the bay at or
below 5 kPa, where life-support's pumps stop and launch is permitted (or vented), for the door; the door fully open for the cradle; the ship turning under 10 deg/s
and accelerating under 5 m/s^2 for the release. A held step SHALL show its reason on flight ops.

#### Scenario: A cold launch
- **WHEN** a pilot enters the port launch bay with the bay stowed and life-support's pump-down to 5 kPa at 23.6 s
- **THEN** Swift 1 is released 37.6 s later, and it becomes an exterior body at the tick of release

#### Scenario: Someone unsuited in the bay
- **WHEN** flight ops starts the pump-down while an unsuited crew member stands in the port bay
- **THEN** the pump-down holds with "crew unsuited in bay", and resumes once they leave

#### Scenario: Launch while the ship turns hard
- **WHEN** the release step is reached while the Tern yaws at 14 deg/s
- **THEN** the release holds with "launch hold: steady the ship" on flight ops and helm

### Requirement: Postures trade readiness for cost
A bay SHALL be in one of the postures stowed, ready 5, ready 1 or hot, and the time from "launch" to
release SHALL follow from the remaining steps: 31.6 s from ready 5, 8 s from ready 1 and 0.5 s from
hot at life-support's 23.6 s pump-down. A bay in ready 1 or hot SHALL be in vacuum and its pressure door SHALL stay
locked to unsuited crew.

#### Scenario: A pilot on ready 1
- **WHEN** flight ops orders a launch with the starboard bay at ready 1
- **THEN** Swift 2 is released 8 s later

### Requirement: Recovery requires a stable approach and a capture tolerance
A craft SHALL be captured only when its centre is within 0.6 m of its bay's capture point, its speed
relative to that point is under 1.0 m/s and its attitude is within 5 degrees of the cradle's, held for
1 s. Inside the recovery corridor the speed relative to the ship SHALL be limited to 20 m/s inside
200 m, 5 m/s inside 50 m and 1 m/s inside 10 m by the craft's assist. While a craft is within 50 m of
a recovering bay, helm's flight assist SHALL hold the Tern under 3 deg/s and 2 m/s^2 unless helm
overrides.

#### Scenario: Capture
- **WHEN** Swift 2 holds 0.3 m from the starboard capture point at 0.4 m/s relative for 1 s
- **THEN** the clamps close, `ship-frames` hands it to the interior frame at that tick, and the cradle
  raises it in 3.5 s

#### Scenario: Too fast
- **WHEN** Swift 2 reaches the capture point at 1.6 m/s relative
- **THEN** it is not captured, and the approach panel shows "closing too fast"

### Requirement: Bay failures are fixed by crew on foot
A drop door that stops short of 96% open SHALL block a Swift's release until a suited crew member
cranks it (60 s from closed to open) or damage-control repairs it. A cradle without power SHALL stop
and be crankable (45 s full travel). A breached bay SHALL need no pump-down and SHALL lock its
pressure door to unsuited crew. A combat drop override SHALL show the predicted clearance first,
computed by the release's own function, and a clearance under 0.3 m SHALL deliver a 1-3 MJ kinetic
hit to the craft.

#### Scenario: A jammed door
- **WHEN** a hull hit jams the port drop door at 40% open
- **THEN** Swift 1 cannot be released, flight ops shows "door jammed 40%", and a suited crew member
  at the crank opens it fully in about 36 s

### Requirement: The Swift flies the one flight model with its own data
The Swift SHALL fly `flight-and-navigation`'s six-degree-of-freedom Newtonian model and flight assist
with mass 8,000 kg, 480 kN main, 160 kN reverse, 120 kN lateral and vertical, rotation limits of 90,
60 and 180 deg/s, 800 kg of propellant at 150 km/s exhaust velocity, a 320 m/s assist limit, twin guns,
two Darts, an 8 MJ shield bubble and a 6 MJ hull. Its mass SHALL fall as propellant burns.

#### Scenario: Full burn from the cradle
- **WHEN** a fully fuelled Swift burns its main engine at full thrust
- **THEN** it accelerates at 60 m/s^2, rising toward 67 m/s^2 as its propellant runs out after about
  250 s of full thrust

### Requirement: The Petrel needs the hangar emptied
The Petrel SHALL launch only through the pad door with the whole hangar at or below 5 kPa, every hangar
pressure door shut, and nobody unsuited in the hangar or its galleries; the pad SHALL lower it 6.0 m in
12 s. It SHALL carry four people and 2,500 kg of cargo, dock through its collar, and SHALL have no
weapons.

#### Scenario: Bay control is in the hangar
- **WHEN** flight ops starts the Petrel's launch while an unsuited crew member sits at bay control
- **THEN** the hangar pump-down holds with "crew unsuited in hangar"

### Requirement: Fighters fly as drones only on an order
With no player in the seat, a Swift SHALL launch only on an order from flight ops, or tactical when
flight ops is merged, and SHALL fly as a drone with a 0.5 degree gun aim error, a 1.0 s reaction,
escort within 2 km by default, stay within 10 km of the Tern, return for auto recovery at 25%
propellant, and load the computer core by 10%.

#### Scenario: Nobody orders a drone
- **WHEN** a mission is fought with no pilot and no drone order
- **THEN** both Swifts stay on their cradles for the whole mission

#### Scenario: Bingo fuel
- **WHEN** a drone's propellant falls to 25%
- **THEN** it breaks off, requests recovery and flies the corridor on auto recovery

### Requirement: Craft rearm and refuel on the cradle
A craft on its cradle SHALL refuel at 20 kg/s from the Tern's craft propellant store, recharge its gun
capacitor from its own reactor at 3 MW, and reload Darts in 15 s each automatically or 8 s each by crew, pressurized or not.
The propellant drawn SHALL leave the Tern's store, which persists between missions.

#### Scenario: Turnaround
- **WHEN** an empty Swift is stowed on its cradle
- **THEN** it is full of propellant 40 s later, with its capacitor and Darts ready before then
