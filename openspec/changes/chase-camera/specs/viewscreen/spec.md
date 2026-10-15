# viewscreen

## ADDED Requirements

### Requirement: CHASE toggles the chase camera

CHASE SHALL turn the chase camera on or off for whichever viewscreen camera is chosen. With it on, the camera SHALL
stand behind the Tern along the line the camera looks, show the Tern in the middle across with its centre below the
middle of the picture, and look the camera's way; TARGET SHALL look at the target, keeping it in the middle.

#### Scenario: TARGET with chase on
- **WHEN** a station's viewscreen is on TARGET with the Hound locked and CHASE is pressed
- **THEN** the Hound is in the middle of the picture and the Tern is in the middle across and in the lower half

#### Scenario: CHASE pressed again
- **WHEN** CHASE is pressed with the chase camera on
- **THEN** the viewscreen shows the chosen camera from the bow again

### Requirement: The main viewscreen shows the captain's camera

The bridge's main viewscreen SHALL show the camera on the captain's console, or the camera he holds on every
viewscreen while he holds it.

#### Scenario: The captain picks CHASE
- **WHEN** the captain has TARGET on his console and presses CHASE
- **THEN** the bridge's main viewscreen shows TARGET from behind the ship
