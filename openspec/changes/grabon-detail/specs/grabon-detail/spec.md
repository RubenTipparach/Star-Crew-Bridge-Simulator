## ADDED Requirements

### Requirement: Detailed Grabon review assets
The review SHALL provide detailed Shrike, Vark and Gorath hulls with purple armor,
orange livery, authored UV textures and connected deck interiors.

#### Scenario: Owner reviews a ship
- **WHEN** a ship is selected in the detailed HTML review
- **THEN** its textured hull, fitted decks and complete vertical access are inspectable
- **AND** the rough concept remains available for comparison

#### Scenario: Owner inspects aft propulsion and armor breaks
- **WHEN** a detailed ship is viewed from behind or above
- **THEN** a central, mirrored impulse exhaust is visible above the hangar
- **AND** Shrike has no projecting engine cubes
- **AND** continuous recessed machinery belts meet the aft roof channels

### Requirement: Three vertical access methods
Every ship SHALL contain stairs with landings, a ladder and an elevator serving
every occupied main deck, with openings aligned to their actual floor heights.

#### Scenario: Review deck connections
- **WHEN** the access view is selected
- **THEN** the stairs, floor openings, ladder and elevator shaft are visible together
- **AND** calling the elevator moves its car to the selected deck height
