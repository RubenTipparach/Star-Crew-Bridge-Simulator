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

#### Scenario: Owner reviews Gorath's lower tier
- **WHEN** Gorath is viewed from the side or above
- **THEN** the bottom deck is narrower and shorter than the engineering deck above
- **AND** its fitted rooms, access shafts and two fighter berths remain inside its hull

### Requirement: Three vertical access methods
Every ship SHALL contain stairs with landings, a ladder and an elevator serving
every occupied main deck, with openings aligned to their actual floor heights.

#### Scenario: Review deck connections
- **WHEN** the access view is selected
- **THEN** the stairs, floor openings, ladder and elevator shaft are visible together
- **AND** calling the elevator moves its car to the selected deck height

### Requirement: Grabon interior identity
Grabon interiors SHALL use their own modeled and Blender-baked panel designs,
angular furniture and warm fixture lighting, distinct from Federation interiors.

#### Scenario: Owner enters a Grabon room
- **WHEN** an interior view is selected
- **THEN** Grabon wall, ceiling, floor and trim designs are visible at crew scale
- **AND** normal and red-alert lighting are inspectable

### Requirement: Windows connect to occupied decks
Each detailed ship SHALL have symmetric, deck-aligned windows that open onto
the same furnished rooms shown in the fitted interior review.

#### Scenario: Owner looks through a window
- **WHEN** an exterior window is inspected up close
- **THEN** its modeled opening and shallow reveal expose the corresponding room
- **AND** the hull, service pipes and interior walls do not obstruct its aperture
