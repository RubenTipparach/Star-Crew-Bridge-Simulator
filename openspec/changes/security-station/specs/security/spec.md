## ADDED Requirements

### Requirement: Internal sensors count who is in each compartment
Every compartment SHALL count the bodies inside it each tick by side (crew, hostile, unknown), and the
security console SHALL show exactly that count; a compartment whose sensor is damaged or unpowered
SHALL show "?" instead of a count.

#### Scenario: A boarder in the cargo bay
- **WHEN** one hostile body stands in the cargo bay and its sensor works
- **THEN** the security console's cargo cell shows one hostile

#### Scenario: A blind compartment
- **WHEN** the magazine's sensor is unpowered
- **THEN** the magazine's cell shows "?" whoever is inside

### Requirement: Entry points raise the alarm
The airlock, the docking port, every hull breach and every boarding pod's cut SHALL be an entry point;
when one opens with a hostile body within 10 m, every console SHALL sound the alarm and the security
console SHALL flash the entry point.

#### Scenario: A boarding pod
- **WHEN** a boarding pod clamps to the hull and finishes its 30 s cut
- **THEN** the alarm sounds and the breach flashes on the security plan

### Requirement: Security locks doors with the damage board's lock
The security console SHALL lock any door and seal any pressure door through the same lock the damage
board uses, and SHALL lock every door of a deck at once. A hostile body SHALL force a locked door in
20 s and a sealed pressure door in 60 s, and the console SHALL show the forcing's progress.

#### Scenario: Deck B locked down
- **WHEN** security presses LOCK DECK B
- **THEN** every door on deck B is locked, and a crew member opening one at its panel holds Use 3.0 s

### Requirement: Security orders crew to a compartment
The security console SHALL order a watch body or a damage control team to a compartment with GO, HOLD
or CLEAR, showing the walk time before the order is given; NPC bodies SHALL obey, and a player SHALL
see the order on the HUD without being moved.

#### Scenario: Sending the watch
- **WHEN** security orders the watch to HOLD the reactor room
- **THEN** the watch bodies walk there in the time the console showed and stay there
