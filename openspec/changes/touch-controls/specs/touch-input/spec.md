# touch-input

## ADDED Requirements

### Requirement: A touch screen plays the game on foot

The client SHALL let a player with only a touch screen walk the ship, look around, use what is in reach, sit at a
station, work its console and stand up, through the same intents the keyboard and mouse make.

#### Scenario: Walking to a station by thumb
- **WHEN** a player on a phone held sideways puts a thumb on the left of the screen and pushes it toward Helm
- **THEN** a stick appears under the thumb and the body walks toward Helm at a speed set by how far the stick is
  pushed

#### Scenario: Sitting and standing by USE
- **WHEN** the player is within 1.5 m of a station's seat and taps USE
- **THEN** the body sits and the console comes up from the desk, and holding USE for the seats' `stand_hold_s` stands the body up

#### Scenario: Looking without pointer lock
- **WHEN** the player drags a finger on the right of the screen, away from the buttons
- **THEN** the view turns by the drag and the body does not move

### Requirement: A tap is never also a click

The client SHALL deliver each touch once: as a touch, with no mouse event synthesized from it.

#### Scenario: One tap on USE
- **WHEN** a finger taps USE once
- **THEN** exactly one press of E reaches the game and no mouse button press does

### Requirement: Console controls are reachable by finger

On a touch screen each console control SHALL be hit within at least 44 CSS px on its shorter side, with what is
drawn unchanged, and a long press SHALL show the tooltip a hover shows without pressing the control.

#### Scenario: A small control on a phone
- **WHEN** a seated player on a phone taps 10 CSS px outside the drawn edge of a 13 CSS px control with no other
  control nearer
- **THEN** that control is pressed

#### Scenario: Long press for the exact value
- **WHEN** a seated player holds a finger still on a control for 0.5 s and lifts it
- **THEN** the control's tooltip was shown and the control was not pressed
