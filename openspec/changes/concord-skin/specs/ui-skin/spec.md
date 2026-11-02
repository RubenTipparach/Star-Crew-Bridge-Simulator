# ui-skin

## ADDED Requirements

### Requirement: Every screen wears the Concord skin from one kit

Every screen SHALL draw its frames, keys, tabs, bars, strips, keycaps, cards, lamps, pips, crest and icons only from
the named kit pieces of the Concord skin, in its palette (Deep, Signal, Sky, Accord, Caution, Alert, Lilac and Text
on black) and its typeface (Antonio in three sizes), with no screen drawing a frame or button of its own.

#### Scenario: A menu key
- **WHEN** the front menu draws its items
- **THEN** each is the kit's pill, Signal at rest, Sky when focused and Lilac when pressed, its label right-aligned

#### Scenario: A console's title band
- **WHEN** a console opens
- **THEN** its title band is the kit's strip with the station's role colour only in its end cap

### Requirement: Colour keeps one meaning

A Concord screen SHALL use at most five colours at rest, and Caution and Alert SHALL appear only for a warning or
danger, never as decoration or a station's identity.

#### Scenario: Tactical's console at rest
- **WHEN** Tactical's console is shown with nothing wrong
- **THEN** no red is on screen
