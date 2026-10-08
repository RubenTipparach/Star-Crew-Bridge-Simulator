# ship-interactables

## ADDED Requirements

### Requirement: A fixture answers Use by one rule

A placed prop with use points SHALL be a fixture whose state the server owns; Use within reach SHALL
apply the fixture kind's rule, the same for a player's body and an NPC's.

#### Scenario: Flushing a toilet
- **WHEN** a body taps Use within 1.2 m of a toilet's use point and the potable tank holds at least 6 L
- **THEN** the toilet flushes for 3.0 s and 6 L move from the potable tank to the grey tank

#### Scenario: Flushing again too soon
- **WHEN** a body taps Use on a toilet 2 s after it flushed
- **THEN** nothing happens and no water moves

### Requirement: A shower runs on water and power

A shower that is on SHALL draw 8 L/min from the potable tank and 4 kW from its bus, and SHALL turn
itself off when the tank is empty or the bus is dead.

#### Scenario: The tank runs dry
- **WHEN** a shower is on and the potable tank reaches 0 L
- **THEN** the shower turns off

### Requirement: Room lighting has a dimmer and a colour

A room with a lighting panel SHALL keep an on flag, a dimmer of 5-100 % and an RGB8 colour, and its
baked lamp light SHALL be drawn multiplied by the colour and the dimmer; emissive faces SHALL not be.

#### Scenario: Night preset
- **WHEN** a body picks Night on the quarters' panel
- **THEN** the quarters' lamp light is drawn deep red at 10 %, and the corridor outside is unchanged

#### Scenario: Red alert overrides, then returns
- **WHEN** red alert is set while the quarters are on Relax, and later cleared
- **THEN** the quarters show the red alert lighting while it holds, and Relax after
