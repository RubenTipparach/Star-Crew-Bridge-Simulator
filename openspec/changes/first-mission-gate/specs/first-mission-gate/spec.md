# first-mission-gate

## ADDED Requirements

### Requirement: The gate passes on one in-game recording

The first mission gate SHALL pass only on one recording of the game's own window on real hardware, with at least two
machines in the session, that shows the fifteen steps of the gate's design in order, from the lobby room to the end
screen.

#### Scenario: A headless capture
- **WHEN** a step is shown only by headless captures, scripted camera poses or a test harness
- **THEN** the gate does not pass on it

#### Scenario: A missing step
- **WHEN** the recording does not show one of the fifteen steps
- **THEN** the gate does not pass

### Requirement: The recording carries a shot list

A gate recording SHALL be listed in `docs/videos/README.md` with the time in the video at which each step is shown.

#### Scenario: Jumping to a step
- **WHEN** the owner wants to see the repair step
- **THEN** the shot list gives its time in the video
