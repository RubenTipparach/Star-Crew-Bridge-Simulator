# sky-layer

## ADDED Requirements

### Requirement: The sky layer never covers a solid

The sky layer (stars, the sun, planets, star streaks and any later sky pass) SHALL be drawn at the far plane, depth
tested less-or-equal and never writing depth, so that no pixel of a ship, station, missile or any other solid drawn
in the same pass shows the sky, whether the sky is drawn before the solids or after them, through a viewscreen feed,
the chase camera or the bridge's windows.

#### Scenario: The sky drawn after a ship
- **WHEN** a ship is drawn in front of a sky full of bright stars and the sky is drawn after it
- **THEN** every pixel of the ship's silhouette is the same as with no sky at all

#### Scenario: Through a bridge window
- **WHEN** the outside is drawn behind the bridge's walls and a ship stands beyond the window
- **THEN** no star shows on the ship or on the bridge's walls

### Requirement: Every drawn solid stands inside the far plane's reach

Every solid outside SHALL lie within the reach at which it resolves in front of the sky layer (120 km from the eye,
with the far plane at 160 km), and a station whose draw distance and size would carry it past that reach SHALL be
refused at load.

#### Scenario: Tamsin Station at the edge of its draw distance
- **WHEN** Tamsin Station, 12.9 km in reach, is drawn with its origin 80 km away
- **THEN** the whole station stands in front of the sky and none of it is cut away

### Requirement: Mockup skies are drawn first

A three.js mockup with a sky SHALL list its sky groups in `window.MOCKUP_SKY` and draw them before every solid without
writing depth, checked in every named shot by `tools/mockups/skycheck.mjs`.

#### Scenario: Pulling the camera back past the star shell
- **WHEN** the camera stands farther from a ship than the mockup's star shell
- **THEN** the ship is still drawn over every star
