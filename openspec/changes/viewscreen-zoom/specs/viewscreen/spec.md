# viewscreen

## ADDED Requirements

### Requirement: Every viewscreen zooms in five steps

Every console's viewscreen SHALL offer the zooms 1x, 2x, 4x, 10x and 20x, each narrowing the lens of the camera in use,
and the bridge's main viewscreen SHALL show the captain's zoom with his camera.

#### Scenario: Zoom to 10x
- **WHEN** a seated player presses 10x
- **THEN** the viewscreen's picture is ten times as magnified as at 1x and the 10x button is lit

#### Scenario: The captain zooms
- **WHEN** the captain presses 4x
- **THEN** the bridge's main viewscreen shows his camera at 4x
