# viewscreen

## ADDED Requirements

### Requirement: CHASE is a chase camera on the target

The CHASE camera SHALL stand outside the Tern, behind it on the line from the target through the ship, look at the
target, show the Tern with its centre below the middle of the picture, and let the player swing it round the ship
while the target stays in the middle.

#### Scenario: CHASE with the Hound locked
- **WHEN** a station's viewscreen is on CHASE with the Hound locked
- **THEN** the Hound is in the middle of the picture and the Tern is in the picture's lower half

#### Scenario: Swung round the ship
- **WHEN** the player drags the camera widget with CHASE on
- **THEN** the camera moves round the ship, and the Hound stays in the middle and the Tern in the lower half

### Requirement: The main viewscreen shows the captain's camera

The bridge's main viewscreen SHALL show the camera on the captain's console, or the camera he holds on every
viewscreen while he holds it.

#### Scenario: The captain picks CHASE
- **WHEN** the captain presses CHASE on his console
- **THEN** the bridge's main viewscreen shows the chase camera
