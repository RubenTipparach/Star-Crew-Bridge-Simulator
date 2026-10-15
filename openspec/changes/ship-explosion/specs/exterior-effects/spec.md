# exterior-effects

## ADDED Requirements

### Requirement: A ship's death lingers

When a ship is destroyed, every client SHALL draw a flash, a fireball, a shock ring, a glowing cloud that drifts and
fades over 15 s, and the charred wreck breaking apart, from a seed every client shares.

#### Scenario: The Hound dies on the viewscreen
- **WHEN** the Hound is destroyed while the captain's viewscreen is on TARGET
- **THEN** the viewscreen shows the flash and the fireball, and 10 s later still shows the glowing cloud and the
  wreck's pieces where the Hound was

#### Scenario: Gone by the next round
- **WHEN** 15 s have passed since the death, or the next round's muster starts
- **THEN** nothing of the death is drawn
