# wall-panels

## ADDED Requirements

### Requirement: Walls are dressed bay by bay

The deck build SHALL divide every wall face into bays at its ribs, stiffeners, corners, opening
frames and wall-fixture keep-outs, and SHALL give each bay one panel module from the set of its
compartment's finish (`data/materials/panels.json`), by the rule of design section 4.

#### Scenario: Neighbours differ
- **WHEN** two bays sit side by side on one wall face, or one above the other in a tall wall,
  and neither is fixed by a door, window, fixture or vent
- **THEN** they take different modules

#### Scenario: A door is flanked
- **WHEN** a bay touches a door's frame
- **THEN** it takes the `flank` module, mirrored so that its light strip is on the door's side

#### Scenario: A window or a console keeps its bay plain
- **WHEN** a bay holds a window, a wall fixture or a portal's frame
- **THEN** it takes `plate`

#### Scenario: Same input, same walls
- **WHEN** the same layout is built twice with the same ship seed
- **THEN** every bay takes the same module

#### Scenario: An edit stays local
- **WHEN** one wall of one compartment changes and the layout is rebuilt
- **THEN** no bay on any other wall changes its module

### Requirement: Walls are banded

The deck build SHALL dress each wall face, from the floor up, with a base strip of `base_m`, a
module band of `module_m`, one further module band for each further whole `base_m + module_m`
of height, and a top strip filling the rest up to the cove, from `panels.json`.

#### Scenario: A crew room 3.0 m high
- **WHEN** a crew compartment's wall is 2.65 m of flat wall under a 0.35 m cove
- **THEN** it is dressed with a 0.5 m base strip, a 2.0 m module band and a 0.15 m top strip

#### Scenario: A tall wall stacks modules
- **WHEN** a wall has 5.0 m or more of flat wall
- **THEN** it carries a second module band above the first, chosen by the same rule one level up

### Requirement: Bays crop modules without cutting features

A module SHALL be centred on its bay at the space's texel density. A bay 1.6-2.4 m wide SHALL
take a full module, a bay 0.6-1.6 m wide the `narrow` module and a bay under 0.6 m `plate`.

#### Scenario: A short bay
- **WHEN** a bay is 1.7 m wide
- **THEN** its module shows u 0.075-0.925, and the cropped parts are its plain margins

### Requirement: Panels stay within their budget

Panel layers SHALL be layers of the deck's one texture array, so a compartment stays one draw
call, and the build SHALL report the panel layers' memory against the `engine-stack` texture
budget.

#### Scenario: One draw per compartment
- **WHEN** a compartment with panelled walls is drawn
- **THEN** it is one draw call, as it was without panels
