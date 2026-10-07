# kick-lights

## ADDED Requirements

### Requirement: Platform edges carry a lit toe kick

Every bridge platform edge of a kind listed in `kicks.platform.on` SHALL be undercut at its foot by
a recess `kicks.platform.height_m` high and `kicks.platform.depth_m` deep, with a light strip at the
back of the recess's ceiling facing out and down, so that from a standing eye height the strip is
hidden and the floor in front of the edge is lit. A `step` edge SHALL not be undercut, and a recess
that ends at an edge without one SHALL be closed by an end face.

#### Scenario: The sub-platform's front edge
- **WHEN** a crew member stands on the bridge floor in front of the helm's sub-platform
- **THEN** the floor at the platform's edge is lit by its kick strip and the strip itself is out of sight under the edge

#### Scenario: A stair lands on a platform
- **WHEN** a platform edge is a `step` edge
- **THEN** it has no recess, and the recess of the edge beside it is closed by an end face

### Requirement: Consoles carry kick strips

Every bridge console SHALL carry the kick strips its family lists in `kicks.props`: a wall bank one
in its toe kick, facing the room; a free console one at the foot of its pedestal's front and one at
the foot of its back.

#### Scenario: Behind the helm
- **WHEN** a crew member stands behind the helm's console
- **THEN** the floor at the foot of the console's pedestal is lit by its back strip

### Requirement: Light columns stand behind the terminals

Every bridge wall bank SHALL have a vertical light column on the wall at each side, a fixture of
type `column_strip`, from near the floor to the top of the bank.

#### Scenario: The ring of stations
- **WHEN** a crew member looks along the bridge's ring of wall banks
- **THEN** each bank stands between two vertical columns of light on the wall behind it

### Requirement: Kick strips are fixtures lit in every state

A kick strip and a light column SHALL be fixtures (`kick_strip`, `column_strip` in
`data/lighting/fixtures.json`) whose colour in
each lighting state is the state palette's `kick` colour: a saturated blue in normal light, red on
red alert and amber on emergency power. They SHALL stay lit on emergency power, and SHALL be
baked as emitters, adding no runtime light and no draw call.

#### Scenario: Red alert
- **WHEN** the bridge goes to red alert
- **THEN** the kick strips and the floor they light turn from blue to red with the rest of the room's lights

#### Scenario: Emergency power
- **WHEN** the bridge runs on emergency power
- **THEN** every kick strip is lit, amber and dimmer, marking the platform edges
