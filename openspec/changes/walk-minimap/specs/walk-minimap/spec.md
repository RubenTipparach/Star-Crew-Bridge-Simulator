# walk-minimap

## ADDED Requirements

### Requirement: A minimap of the deck while walking

While the player is on foot aboard, the client SHALL show a minimap of the deck the body's feet are on: 384 x 256
reference px at the bottom right of the safe area on the desktop, and in the MAP button's corner on a touch screen. It
SHALL NOT show while the player is seated at a console, while the ship map, the menu, the briefing or the debrief is
open, or while flying.

#### Scenario: Walking the main deck
- **WHEN** the player walks the main corridor on deck B
- **THEN** the minimap shows deck B around them, with the band reading the corridor's name and B

#### Scenario: Sitting down
- **WHEN** the player sits at a station
- **THEN** the minimap fades out as the console rises, and returns when they stand

### Requirement: The minimap is drawn from the deck the body walks

The minimap's rooms, doors and ways off the deck SHALL come from the compiled deck and its walk (the walk grid, the
deck's compartments and walk entities), and its people from the same function as the ship map's, so that it never
disagrees with the ship as walked.

#### Scenario: The command deck
- **WHEN** the player walks deck A
- **THEN** the minimap shows the command suite's rooms (the briefing room, the captain's quarters), not the layout's
  older rooms

### Requirement: The minimap shows the player, the crew and the hazards by shape

The minimap SHALL show the player as an arrow of their facing, every crew member aboard as the ship map's marker (a
square round a seated one, a flashing cross on one who is down), every way off the deck as a glyph with an up or down
chevron, and fire, a damaged system and a breach in their state colours with the shapes the other screens use. Its only
words SHALL be the current room's name and the deck's letter.

#### Scenario: A fire next door
- **WHEN** the briefing room burns while the player stands on the bridge
- **THEN** the minimap shows the flame glyph in the briefing room and the panel's edge in the red alert colour

### Requirement: The minimap follows the player between decks and opens the full map

The minimap SHALL switch to another deck when the body steps off a ladder, a stair or the lift onto that deck's floor,
and a click or a tap on it SHALL open the ship map on the deck the player is on.

#### Scenario: Down the ladder
- **WHEN** the player climbs down the ladder from deck B and steps onto deck C
- **THEN** the minimap shows deck C

#### Scenario: A tap on a phone
- **WHEN** the player taps the minimap on a phone
- **THEN** the ship map opens on the deck they are on
