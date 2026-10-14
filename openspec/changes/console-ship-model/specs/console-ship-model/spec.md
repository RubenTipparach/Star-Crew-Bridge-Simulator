# console-ship-model

## ADDED Requirements

### Requirement: The consoles draw the Tern's low poly model

The console's 3D pictures of the Tern (the camera widget and the shield view) SHALL draw the triangles of the Tern's
lowest level of detail exterior model, from a data file built from that model and checked against it.

#### Scenario: The camera widget
- **WHEN** a seated player looks at the camera widget beside the viewscreen
- **THEN** the ship in it is the Tern's exterior, nacelles and all, not the pressure hull's capsule

#### Scenario: A new model
- **WHEN** `tern_lod2.glb` changes and the data file is not rebuilt
- **THEN** `tools/ships/console_model.py --check` fails
