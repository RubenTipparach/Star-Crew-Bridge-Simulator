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

### Requirement: Floors stay within their budget

Floor modules SHALL be layers of the deck's one texture array, so a compartment stays one draw
call, and the build SHALL report their memory against the `engine-stack` texture budget.

#### Scenario: One draw per compartment
- **WHEN** a compartment with dressed walls, ceiling and floor is drawn
- **THEN** it is one draw call
