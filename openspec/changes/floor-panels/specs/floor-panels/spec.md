# floor-panels

## ADDED Requirements

### Requirement: Floors are dressed cell by cell

The deck build SHALL cut every floor into the same bays and cells as its ceiling and SHALL give
each cell one floor module from the set of its compartment's finish
(`data/materials/panels.json`), by the rule of design section 1.

#### Scenario: The walkway joins the doors
- **WHEN** a cell lies on the 0.9 m path from one of the room's doors to its centroid
- **THEN** it takes `walkway`

#### Scenario: A hatch keeps its rim
- **WHEN** a cell holds a floor portal
- **THEN** it takes `plate`, and the portal's hazard rim is still drawn

#### Scenario: Neighbours differ off the walkway
- **WHEN** two cells side by side are neither walkway nor fixed by a portal
- **THEN** they take different modules

### Requirement: No floor module is cut by a platform

A floor cell that a platform, or a band at its foot, reaches into SHALL take `plate`, and a cell
wholly under one SHALL NOT be drawn, so no drawn floor module is ever partly hidden by something
standing on the floor (design section 6).

#### Scenario: A grate at a ring's foot
- **WHEN** the bridge's raised ring covers part of a floor cell's core
- **THEN** that cell is plain plate, and no grate, vent, drain or access plate runs under the ring

#### Scenario: Under the dais
- **WHEN** a floor cell lies wholly under the captain's dais
- **THEN** no triangle is drawn for it

### Requirement: Platform edges carry whole fittings

Every `rail` and `riser` edge of a platform at least 0.25 m long SHALL carry a band on the floor at
its foot (vents under rails, trench covers under risers) whose row holds a whole number of 0.5 m
periods, so the band begins and ends on a frame and no grille is cut (design section 6).

#### Scenario: A 1.3 m rail edge
- **WHEN** a rail edge is 1.3 m long
- **THEN** its band holds three grilles, each stretched by 1.3 / 1.5

### Requirement: Floors stay within their budget

Floor modules SHALL be layers of the deck's one texture array, so a compartment stays one draw
call, and the build SHALL report their memory against the `engine-stack` texture budget.

#### Scenario: One draw per compartment
- **WHEN** a compartment with dressed walls, ceiling and floor is drawn
- **THEN** it is one draw call
