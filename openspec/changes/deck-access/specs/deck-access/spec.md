# deck-access

## ADDED Requirements

### Requirement: One lost compartment cuts off only dead ends

With any one compartment lost (breached, burning or sealed), the bridge SHALL still reach, through
crew portals, every room except the dead ends. A dead end is a room with one way in by its nature:
a turret pod, a launch bay, the drive section, the airlock. This SHALL hold with the lift unpowered.

#### Scenario: The command passage is lost
- **WHEN** the command passage behind the bridge is sealed
- **THEN** the bridge reaches decks B and C through a side room and a stair tower, or a scuttle

#### Scenario: A spine corridor is lost
- **WHEN** deck B's main corridor or deck C's lower corridor is sealed
- **THEN** every room on that deck but a dead end is still reached, through a tower or a second door

#### Scenario: No power
- **WHEN** the lift has no power
- **THEN** the single-loss rule above still holds

### Requirement: Stair towers each side

The Tern SHALL have a stair tower each side of the spine from deck C to deck A that needs no power,
with a landing on every deck, a door from each landing onto the spine corridor, and a door from
each landing into the room beside it.

#### Scenario: A tower opens both ways
- **WHEN** a crew member stands on a tower's landing on any deck
- **THEN** one door leads to the spine corridor and another into the room beside it

#### Scenario: A tower is lit at every deck
- **WHEN** a stair tower's lamps are placed
- **THEN** each deck's flight has a lamp over it, and at least one lamp in the tower is on the emergency bus

### Requirement: A lift by the bridge

The Tern SHALL have a lift from deck C to deck A with its door on the command passage beside the
bridge door, and a car long enough for a stretcher. It SHALL stop when it has no power.

#### Scenario: A casualty to the medbay
- **WHEN** a crew member carries a casualty from helm's seat to a medbay bed
- **THEN** the route by the lift is shorter in time than the route down the ladder

### Requirement: Scuttles from the bridge's side rooms

The ready room and the briefing room SHALL each have a hatch and ladder down to the room below
them, the medbay and damage control.

#### Scenario: The bridge's way down when the passage is lost
- **WHEN** the command passage is sealed
- **THEN** a crew member on the bridge reaches deck B through a side room's scuttle
