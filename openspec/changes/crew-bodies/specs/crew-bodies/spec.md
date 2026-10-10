# crew-bodies

## ADDED Requirements

### Requirement: Crew bodies are built from MakeHuman to the Pi 5 avatar budget

Each crew body SHALL be generated from a row of `data/crew/bodies.json` by `tools/blender/build_crew_bodies.py`
from MakeHuman's pinned CC0 assets, as one mesh of at most 3,000 triangles with one material, per-vertex
palette colours and at most 30 bones, and the build SHALL refuse a body over that budget, naming it.

#### Scenario: A body over budget is refused
- **WHEN** a row's body decimates to more than 3,000 triangles
- **THEN** the build stops and names the body and its triangle count, and writes nothing

### Requirement: Crew bodies have cartoon proportions

A crew body SHALL have the head, hands and feet scaled up and the neck shortened by the table's cartoon
scales, applied on the rig so the skin weights carry them, and SHALL stand no taller than the walk body's
1.80 m.

#### Scenario: The head reads at a glance
- **WHEN** a body is built with the table's default scales
- **THEN** it is between four and five heads tall, not seven and a half, and no taller than 1.80 m

### Requirement: Crew bodies wear fitted clothes

A crew body SHALL wear garments fitted to it by MPFB's clothes service (a tunic, trousers and boots), with
the skin the garments cover deleted, so its hem, cuffs and neckline are the garments' own edges and no skin
shows through them.

#### Scenario: No skin under the tunic
- **WHEN** a body is built
- **THEN** no body face whose strongest bone is in the tunic, trousers or boots region remains in its mesh

### Requirement: Crew eyes match the references' ratios

A crew body's eyes SHALL be painted with an iris whose width is the table's `iris_ratio` of the eyeball's
and a pupil whose width is `pupil_ratio` of the iris's, within 0.02, and the build SHALL print both as
measured on the built mesh.

#### Scenario: The ratios hold on every body
- **WHEN** the six bodies are built with `iris_ratio` 0.55 and `pupil_ratio` 0.40
- **THEN** each body's printed ratios are 0.55 and 0.40
