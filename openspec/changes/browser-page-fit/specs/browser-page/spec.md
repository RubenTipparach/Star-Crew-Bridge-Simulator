# browser-page

## ADDED Requirements

### Requirement: The game follows the page's size

In the browser build the engine's window SHALL take the canvas's size on the page and its drawing buffer that size
times the page's pixel ratio, capped at 2, after every resize and on entering or leaving full screen, so the frame is
drawn whole and the mouse is read at the size the frame is drawn.

#### Scenario: itch.io's full screen button
- **WHEN** the player presses itch.io's full screen button, or resizes the browser window, at any pixel ratio
- **THEN** the next frame fills the page, nothing is cut off, and a click at a button's centre presses that button

#### Scenario: A phone at a pixel ratio of 3
- **WHEN** the page is opened on a screen whose pixel ratio is 3
- **THEN** the drawing buffer is the canvas's size times 2

### Requirement: The page is only the game

The browser page SHALL show only the game's canvas, filling the page edge to edge, with no text, keys, notes or
border around it.

#### Scenario: The itch.io page
- **WHEN** the player opens the game on itch.io
- **THEN** the frame holds only the game, and its keys are learned in the game, not read off the page

### Requirement: Every screen is laid out on one reference frame

Every screen SHALL be laid out on one reference frame of 1920 x 1080 reference pixels (written as 1280 x 720 layout
points, 1.5 reference pixels each), scaled to the output by the smaller of its width over 1920 and its height over
1080 and centred, with the extra space at the edges. One function SHALL map the frame to the output and the mouse's and
fingers' window positions back to the frame, and no screen SHALL read the output's pixels, its pixel ratio or the
window's size itself.

#### Scenario: A narrow or short screen
- **WHEN** the game runs at 1024 x 768, at 2560 x 1080, in itch.io's 985 x 455 embed or on a phone at 844 x 390
- **THEN** every button and text of the front menu, the briefing, the bridge's band and the consoles is whole on the
  screen, and a click at a button's centre presses it

#### Scenario: A screen reads its size
- **WHEN** a UI module reads the window's size or makes a scale from the output's pixels outside the one function
- **THEN** the check `ui::no_screen_reads_the_output_size` fails

### Requirement: Every screen in the browser has a way on

No control in the browser build SHALL do nothing or leave the player on a frozen frame: there is no Exit or Quit where a
page cannot close itself, no Display choice where the page decides full screen, and on foot a round SHALL wait for
every person aboard to sit, their sitting being their READY.

#### Scenario: The game menu in the browser
- **WHEN** the player opens the game menu in the browser
- **THEN** it offers Resume, Options and Return to menu, and Return to menu brings back the front menu

#### Scenario: Sitting before the crew
- **WHEN** the player sits at their station in Muster before the last bot has sat
- **THEN** the countdown begins once the crew are seated, without a READY to press
