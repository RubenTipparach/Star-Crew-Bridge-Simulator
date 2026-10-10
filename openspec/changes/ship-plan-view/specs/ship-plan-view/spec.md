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

### Requirement: Any crew member can open the ship map at any time

A player on foot, flying or riding the lift SHALL be able to open a 3D map of the whole ship with one key and close it
with the same key, and the map SHALL show every body aboard at its position: every player with their name and every
bot crew member, the player's own marker set apart.

#### Scenario: Finding a friend
- **WHEN** a player presses M while walking on deck B
- **THEN** the map shows all three decks exploded, with every player's and bot's marker where their body stands

### Requirement: The map has a damage control mode

The ship map SHALL have a Damage mode that shows, from the simulations that own them, every damaged system by its state,
every damaged room (breaches, damaged wall sections, charred floor, the hull's armour), every fire, every switchboard
section and distribution panel that needs repair or has a breaker tripped, and every conduit as live, dead or severed.

#### Scenario: After a hit in engineering
- **WHEN** a hit severs the port drive feeder and destroys the port switchboard section, and a fire starts in engineering
- **THEN** the Damage mode shows the feeder red with a break, the switchboard box red with a cross, the burning cells in engineering, and the drive's loads fed from the starboard feeder drawn live
