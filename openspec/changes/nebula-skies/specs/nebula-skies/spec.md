# nebula-skies

## ADDED Requirements

### Requirement: A system's sky is a sphere of coloured vertices

Each star system SHALL name a sky file in `data/space/skies/`, generated from a recipe, and the client SHALL draw it as
one indexed mesh of vertex colours with no texture, at most 8,000 vertices, in the system's frame, so that it turns
with the ship and stays put as the ship moves.

#### Scenario: Turning in the Shoals
- **WHEN** the Tern yaws 90 degrees at Tamsin Station
- **THEN** the nebula moves 90 degrees across the windows and the viewscreen, as the stars do

#### Scenario: A bad sky file
- **WHEN** a sky file has an index past its vertex count
- **THEN** startup stops with the file's path and the field

### Requirement: The sky never draws over a ship

The sky, its faint stars, its sun and planet, and its vector stars SHALL be drawn before any ship, station, craft, prop
or room in a pass, with no depth writes, so that everything in the world draws over them.

#### Scenario: A star behind the Hound
- **WHEN** a bright star's direction lies behind the Hound's hull on screen
- **THEN** the Hound's hull is drawn there and the star is not

### Requirement: Stars streak when the view turns

Each vector star SHALL be drawn as a line from where it is on screen toward where it was `tail_s` earlier, its tail
capped at `max_deg` and its brightness dimmed by `dim_per_deg` to no less than `dim_floor` (`data/space/starfield.json`),
a star at rest drawn as a 1 px point, and no tail drawn across a camera cut.

#### Scenario: A fast turn
- **WHEN** the view turns at 70 degrees a second
- **THEN** every star's tail is 1.7 degrees long and the star is drawn at 29% of its brightness

#### Scenario: Sitting down
- **WHEN** the player sits at a station and the camera moves to the seated view
- **THEN** that frame draws the stars as points, not lines across the screen

### Requirement: The sky's colour reaches the hulls, subtly

The outside ambient light on ships and stations SHALL take the sky file's ambient cube, whose brightest face's luminance
is the sky's `ambient.strength` (linear), no more than 0.13 (Homeworld's 40% on the display).

#### Scenario: Under a rose nebula
- **WHEN** the Tern lies at Tamsin Station under sky A
- **THEN** its shaded side takes a faint rose tint from the cube's faces, and its sunlit side is lit by the star as before
