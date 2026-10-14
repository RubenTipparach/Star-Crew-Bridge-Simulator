# viewscreen

## ADDED Requirements

### Requirement: The target bracket frames the target

A console's viewscreen SHALL draw the locked target inside four corners that close round the target's hull as the
viewscreen's camera sees it, coloured by its IFF, with its IFF word and hull bar above and its id and range below.

#### Scenario: The Hound near and far
- **WHEN** the Hound is the target at 300 m and later at 3 km
- **THEN** the corners enclose the Hound's hull with at most 4 px to spare on each side at both ranges

#### Scenario: Seen from the chase camera
- **WHEN** CHASE is on and TARGET is the camera
- **THEN** the corners close round the Hound where the chase camera's picture shows it

#### Scenario: Corners, not ticks
- **WHEN** the bracket is drawn
- **THEN** it is four L shapes at the corners of one rectangle and nothing along its sides
