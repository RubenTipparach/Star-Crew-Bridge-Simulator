# ship-exterior

## ADDED Requirements

### Requirement: The player ship's hull is drawn from its exterior model

The client SHALL draw the player ship's hull from its exterior model's full detail level, in the default livery, in
the ship's frame with the model's origin at the layout's origin, wherever the outside of the ship can be seen: through
a window, and from the free camera outside.

#### Scenario: Looking out of the quarters
- **WHEN** a crew member stands at a quarters window on deck B and looks aft
- **THEN** the hull's flank, the starboard pylon and its nacelle are drawn beyond the window, lit by the model's baked
  sunlight

#### Scenario: Outside in the dock
- **WHEN** the free camera is outside the ship in the space dock
- **THEN** the whole hull is drawn inside the dock's frame, and no room's wall shows through it

### Requirement: The hull is compiled with the decks

`deckc` SHALL compile the ship's exterior model into the deck file beside the rooms, with the rooms' vertex packer,
colour rule and mip chain, and SHALL refuse a model whose triangle count or bounds disagree with its manifest or whose
triangles exceed the Pi 5 exterior budget.

#### Scenario: A model over budget
- **WHEN** the exterior model holds more than 12,000 triangles
- **THEN** `deckc` stops with the model's path and its triangle count, and writes no deck file

### Requirement: Every window meets the hull

Every window of the runtime layout SHALL coincide with an aperture of the exterior model (the same id, centre within
1 cm, the same size, facing within 1 degree), and every aperture SHALL have its window.

#### Scenario: A window moved in the layout alone
- **WHEN** a window's centre in the layout moves 5 cm and the model is not rebuilt
- **THEN** `tools/exterior_windows.py --check` fails, naming the window and the distance
