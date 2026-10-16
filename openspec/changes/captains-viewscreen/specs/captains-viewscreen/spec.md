# captains-viewscreen

## ADDED Requirements

### Requirement: The captain's picture on every screen

While the captain holds the viewscreen, every console's VIEWSCREEN panel and the bridge's main viewscreen SHALL show
his camera with his CHASE and zoom, decided by one function, and taking the viewscreen SHALL send all three.

#### Scenario: On screen
- **WHEN** the captain, on CHASE at 10x, presses SCREEN
- **THEN** the helm's VIEWSCREEN panel and the bridge's main viewscreen show the chase camera at 10x

#### Scenario: Given back
- **WHEN** the captain presses SCREEN again
- **THEN** each console's panel shows its own camera again, and the main viewscreen the captain's
