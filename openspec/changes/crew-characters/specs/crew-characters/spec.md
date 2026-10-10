# crew-characters

## ADDED Requirements

### Requirement: The crew are blocky figures until a character is approved

The crew in the engine and the mockups SHALL be the blocky figures of `crew-npcs` 7 until a character design
is approved by the owner on screenshots.

#### Scenario: No character model without approval
- **WHEN** a character model is built but the owner has not approved its screenshots
- **THEN** the bots aboard are still drawn as the blocky figures

### Requirement: A crew character is a cartoon with big eyes at the references' ratios

A crew character SHALL have exaggerated cartoon proportions (a head, hands and feet bigger than life and a
short neck, by amounts held in data) and big eyes whose iris is 0.45 to 0.58 of the eye's width and whose
pupil is 0.4 of the iris's width, both edges being geometry.

#### Scenario: The eye ratios are measured on the built model
- **WHEN** a character is built
- **THEN** the build prints its iris and pupil ratios measured on the mesh, and they are within the ranges

### Requirement: A crew character's clothes are garments

A crew character's tunic, trousers and boots SHALL be geometry over the body, with the hem, cuffs, collar and
trouser legs as their own edges, and no body surface SHALL show through them.

#### Scenario: Nothing shows through the tunic
- **WHEN** a character is rendered from any side in its rest pose
- **THEN** no skin is visible inside the tunic's or the trousers' outline

### Requirement: A crew character fits the avatar budget and takes probe light

A crew character SHALL be at most 3,000 triangles, 30 bones, one material and one draw call, read back from
its built file, and SHALL be lit by the light probes around it.

#### Scenario: Over budget is refused
- **WHEN** a character's built file holds more than 3,000 triangles
- **THEN** the build stops and names the character and its count
