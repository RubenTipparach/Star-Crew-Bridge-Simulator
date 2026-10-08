# ship-plan-view

## ADDED Requirements

### Requirement: Every prop has a light model

Every prop build SHALL write, beside each prop over 0.6 m, a light model within the budget for its size,
mapped into the same atlas, closed and inside the full model's bounds.

#### Scenario: A console's light model
- **WHEN** the bridge set builds a 1.2 m wall bank
- **THEN** its glb holds a light model of at most 24 triangles using the bank's atlas

### Requirement: The whole ship in plan fits the Pi 5

The plan view of the whole ship SHALL draw only light models and SHALL stay within half the Pi 5's frame
budget for triangles and draw calls.

#### Scenario: All three decks exploded
- **WHEN** the plan view shows all three decks of the Tern
- **THEN** it draws at most 100,000 triangles in at most 150 draws

### Requirement: Bodies are shown where they are

The plan view SHALL show every body aboard at its position on its deck, from the client's snapshot, a
player's marker in its role colour with its name and a downed body flashing.

#### Scenario: A player goes down in engineering
- **WHEN** a player's body is downed in engineering
- **THEN** its marker on the captain's crew page flashes there within one snapshot
