# ceilings-and-trims

## ADDED Requirements

### Requirement: Ceilings are dressed cell by cell

The deck build SHALL cut every ceiling into bays at its beams and each bay into 2 m cells from
the room's centreline outward, and SHALL give each cell one ceiling module from the set of its
compartment's finish (`data/materials/panels.json`), by the rule of design section 1.

#### Scenario: Neighbours differ
- **WHEN** two cells sit side by side in a bay, or in the same place in adjacent bays, and
  neither is fixed by a lamp or a collar
- **THEN** they take different modules

#### Scenario: A lamp sits in its surround
- **WHEN** the kit's lamp rule places a lamp in a cell
- **THEN** that cell takes `lamp_surround`

#### Scenario: Same input, same ceilings
- **WHEN** the same layout is built twice with the same ship seed
- **THEN** every cell takes the same module

### Requirement: Trims take strips along their length

Ribs, beams, coves, baseboards and door frames SHALL take the trim strip their member names,
with u along the member's length (tiling with a 2 m period) and v across its face within the
strip's row, and SHALL stretch a strip by no more than 1.6 times across a face.

#### Scenario: A beam across the bridge
- **WHEN** a beam runs 13 m across the bridge
- **THEN** its faces repeat the `beam` strip along its length without a seam at the period

### Requirement: Ceilings and trims stay within their budget

Ceiling modules and trim strips SHALL be layers of the deck's one texture array, so a
compartment stays one draw call, and the build SHALL report their memory against the
`engine-stack` texture budget.

#### Scenario: One draw per compartment
- **WHEN** a compartment with panelled walls, a dressed ceiling and stripped trims is drawn
- **THEN** it is one draw call
