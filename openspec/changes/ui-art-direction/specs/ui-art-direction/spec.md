# ui-art-direction

## ADDED Requirements

### Requirement: UI scales from a 1920 x 1080 reference frame

Every screen outside the consoles SHALL be laid out in reference pixels on a 1920 x 1080 frame and drawn at a scale of
min(width / 1920, height / 1080), with each group anchored to an edge, a corner or the centre of the screen, so that
no player-facing element is drawn outside the screen at any window size.

#### Scenario: An ultrawide window
- **WHEN** the game runs in a 2560 x 1080 window
- **THEN** the UI is drawn at scale 1, corner groups sit in their corners and centred frames stay centred

#### Scenario: A short embed
- **WHEN** the game runs in a 985 x 455 window
- **THEN** every menu, prompt and panel is drawn wholly on screen, at scale 0.42

### Requirement: One typeface and four type sizes

The screens around the consoles SHALL use the consoles' typeface (Barlow Semi Condensed, weights 600 and 700) at four
sizes in reference pixels (72, 40, 28 and 20), read from `data/ui/ui_style.json`.

#### Scenario: The front menu
- **WHEN** the front menu is drawn
- **THEN** every word on it is set in Barlow Semi Condensed at one of the four sizes, apart from the wordmark

### Requirement: Opaque panels behind text

Every panel that carries text SHALL be opaque, so no part of the game shows through its words.

#### Scenario: The briefing at a console
- **WHEN** the briefing is up at a console before a round
- **THEN** it is drawn on a solid backdrop and none of the console's controls show through it

### Requirement: Keys drawn as keycaps, only where they act

A key SHALL be shown as a keycap glyph beside a short action name, and SHALL NOT be spelled into a sentence or shown
on the game's screen when it does nothing there.

#### Scenario: A seat in reach
- **WHEN** a walking player comes within reach of an empty Helm seat
- **THEN** a solid prompt below the centre shows HELM over an E keycap and the verb Sit down

#### Scenario: Nothing in reach
- **WHEN** nothing the player can use is in reach
- **THEN** no key is shown on the walking screen

### Requirement: Controls listed in Options, not on the game's screen

Options SHALL hold a Controls page, a square panel listing the client's bindings as keycaps grouped by activity, with
pages for on foot, at a console and touch; no screen of the game SHALL show a strip or line of key hints.

#### Scenario: Looking up the lift keys
- **WHEN** a player opens Options and then Controls
- **THEN** the on-foot page lists Lift with Up a deck E, Down a deck Q and Call the lift E

#### Scenario: Lights under the Captain
- **WHEN** a player looks for the lighting keys
- **THEN** they are listed only under Captain on the console page

### Requirement: Player-facing text carries no internal names or dev notes

No player screen SHALL show an internal identifier, an enum name, a debug value, a placeholder name, a dev tag or a
note about how the game is built.

#### Scenario: The briefing's crew list
- **WHEN** the briefing lists a station with no one at it and stations held by NPC crew
- **THEN** the empty station reads Ship's computer, and no name carries "(bot)" or reads "automation"

### Requirement: Leave station from the console and the menu

A seated player SHALL be able to stand up from a Leave station button in the console's top bar, showing the Stand key
as a keycap with a ring that fills while it is held, and from a Leave station item in the Esc menu.

#### Scenario: Leaving by the band
- **WHEN** a seated player clicks Leave station in the console's band
- **THEN** the body stands up at the station, as a held Stand key does

#### Scenario: The Esc menu while walking
- **WHEN** a walking player opens the Esc menu
- **THEN** it lists Resume, Options, Controls and Leave ship, without Leave station
