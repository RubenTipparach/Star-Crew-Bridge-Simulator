# spacedock

## ADDED Requirements

### Requirement: The starbase has an enclosed bay the Tern flies into

Tamsin Station SHALL be a solid structure at least 1.5 km across its longest axis with an enclosed bay, walled and
roofed, which the Tern enters and leaves through doors at least three times the Tern's 45 m width over its nacelles,
and the doors SHALL be shut while the Tern lies berthed inside.

#### Scenario: Cleared to enter
- **WHEN** traffic control has cleared the Tern to a berth and the Tern's bow is inside 1 km of the doors on the
  approach corridor
- **THEN** the doors open over 20 s and the Tern can fly through them

#### Scenario: Berthed with the doors shut
- **WHEN** the Tern's stern is 200 m inside the bay
- **THEN** the doors close behind it and the windows show the bay's lit inside, not the sky

### Requirement: The bay's speed limits hold to the berth

Inside the doors the Tern SHALL be held to the bay's limit of 5 m/s until 100 m from its berth, and to the docking
corridor's limits of `flight-and-navigation` 10 from there to contact, whether helm flies it by hand or the dock
autopilot does.

#### Scenario: Too fast in the bay
- **WHEN** helm flies the Tern at 8 m/s inside the bay
- **THEN** the flight assist holds it to 5 m/s and helm's console shows the bay's limit

### Requirement: The berth meets the docking collar

At its berth the Tern SHALL be held by clamps from a pier along its starboard side, and a crew tube SHALL meet its
docking collar on deck C, so the crew reach the airlock by walking the ship.

#### Scenario: Berthed
- **WHEN** the Tern touches its berth inside the docking corridor's limits
- **THEN** the clamps close, the crew tube meets the collar and shore power comes on

### Requirement: Other ships lie docked at the station

The bay SHALL hold at least six other ships at their berths, drawn from the Federation's models at their middle level
of detail, and at least one free berth, lit.

#### Scenario: The view while berthed
- **WHEN** a crew member looks out of a bridge window while the Tern is berthed
- **THEN** the window shows at least one other docked ship, the berths' lights and the bay's lit walls

### Requirement: The station keeps within its Pi 5 budget

The station's near model SHALL be drawn in at most three draws (its outside, its inside with its glowing parts, and
its doors) and within the triangle count its chosen version states in the design, and it SHALL take no texture beyond
the existing texture array.

#### Scenario: Berthed on the Pi
- **WHEN** the Tern lies berthed and the frame is drawn on a Pi 5
- **THEN** the station adds at most three draws and its stated triangles to the frame

### Requirement: A rotating station's dock does not rotate

Where the station's habitat rotates (a wheel or a pair of O'Neill cylinders), the dock the Tern berths in SHALL NOT
rotate, and a pair of habitats SHALL turn in opposite directions so their spin cancels.

#### Scenario: Berthed beside a turning habitat
- **WHEN** the Tern lies berthed in the dock of a station whose cylinders turn at 1 rpm
- **THEN** the berth, its clamps and its crew tube stay still relative to the Tern while the cylinders turn behind the
  dock, one each way

### Requirement: The bay is lit by baked light

The inside of a station's bay SHALL be lit by light baked offline from its fixtures (the rings, the floods, the berths'
work lights and the window tiers) into vertex colours in three states, and every ship in the bay, the Tern's hull
included, SHALL be lit by the bay's baked probes rather than by the star.

#### Scenario: Berthed in Tamsin Station's bay
- **WHEN** the Tern lies in its berth in E's round bay
- **THEN** its hull and the docked ships are lit by the bay's floods and work lights from the probes, and no sunlight
  falls on them

#### Scenario: Leaving the bay
- **WHEN** the Tern flies out down the throat
- **THEN** the light on its hull goes from the bay's probes to the star's over the throat, and in open space it is the
  star's
