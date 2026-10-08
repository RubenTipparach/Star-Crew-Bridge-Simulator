# normal-maps

## ADDED Requirements

### Requirement: Relief is lit from the baked light's direction

Every textured surface SHALL carry a normal layer, and a pixel's light SHALL be the baked light of its
vertices shaded by its normal against the baked light's direction for the current lighting state, as
design section 3 states; a flat pixel SHALL take exactly the baked light it takes today.

#### Scenario: A bolt under a lamp
- **WHEN** a wall panel's bolt stands below a ceiling lamp in the normal lighting state
- **THEN** its upper side is lighter than its lower side

#### Scenario: A flat face is unchanged
- **WHEN** a pixel's normal from its normal layer equals its face's normal
- **THEN** its colour equals the colour without normal maps

### Requirement: Light is counted once

A colour layer used with a normal layer SHALL carry no baked key light in its relief: only its albedo,
occlusion and wear.

#### Scenario: A layer built for normal maps
- **WHEN** a panel, material or prop bake writes a normal layer
- **THEN** its colour layer is baked without the key light, and the page or deck pairs them

### Requirement: Normal maps fit the Pi 5

Normal maps SHALL ship only after the deck pass with them is measured on a Raspberry Pi 5 at 1280 x
720 against the pass without them, and the difference fits the frame budget.

#### Scenario: The probe decides
- **WHEN** the probe measures option C's deck pass over the frame budget
- **THEN** normal maps fall back to a distance fade, then props only, then none, as design section 5 orders
