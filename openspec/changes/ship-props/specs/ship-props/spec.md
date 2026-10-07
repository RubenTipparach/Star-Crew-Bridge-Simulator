# ship-props

## ADDED Requirements

### Requirement: Every system and craft is a modelled prop within its budget

Each ship system inside the hull and each craft SHALL be drawn as a prop of the machinery set, built by a
committed generator, within the triangle budget that generator states; the build SHALL refuse a prop over its
budget.

#### Scenario: A prop over budget is refused
- **WHEN** the machinery build makes a prop with more triangles than its budget
- **THEN** it writes nothing and names the prop

#### Scenario: The build is reproducible
- **WHEN** the machinery build runs with `--check`
- **THEN** every prop's bytes and the manifest match the committed files

### Requirement: Every station has its console and chair

Each station SHALL be drawn with a console and, where its crew sits, a chair: a wall bank on a wall within 1.7 m
ahead of the seat, else a free-standing desk; a stand-up console and a chair in a turret pod; the station's own
variant where the set has one.

#### Scenario: A station facing a wall
- **WHEN** a station's seat faces a wall 1.7 m away or nearer
- **THEN** its console is a wall bank with its back on that wall, and its chair stands at the bank's operator distance

### Requirement: The crew rooms are furnished for their purpose

The crew quarters, the mess and damage control SHALL hold the furniture their purposes name, every piece inside
its room, clear of every door's zone, and not overlapping another, on the layout as it stands and with each
proposed patch.

#### Scenario: Eight berths
- **WHEN** the crew quarters are furnished
- **THEN** they hold eight berths

#### Scenario: A door kept clear
- **WHEN** any piece is checked against the doors of its room
- **THEN** none stands in a door's 1.0 m zone

### Requirement: A lamp's lens shows whole panels, centred

A lamp's lens SHALL show whole light panels centred on the lamp, whatever the lamp's position and size.

#### Scenario: Two lamps in one room
- **WHEN** two lamps of one room are compared
- **THEN** their lenses show the same picture, two whole panels side by side

### Requirement: Every floor hatch has a ladder

A ladder SHALL run up through every floor hatch a crew member climbs, from the floor below to the floor above,
with handholds past it.

#### Scenario: A scuttle
- **WHEN** a crew member stands under a scuttle's hatch
- **THEN** a ladder runs from their floor up through the hatch to the floor of the room above
