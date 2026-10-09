# repairs-on-deck

## ADDED Requirements

### Requirement: A repair is played docked at the machine

A player SHALL start a repair by Use at the system's repair point while holding a kit, which docks the body at the
point; the service face's cover, a textured plate, SHALL be unscrewed and lifted off in the 3D view, which SHALL
show a still picture of the system's mini-game in the machine. Only a click on that picture (or Use while it is
pointed at) SHALL draw the mini-game over the live, darkened 3D view, and Esc SHALL put it back into the picture
with the round paused. The job SHALL count as done only once the cover is screwed back on in 3D.

#### Scenario: Docking at the coolant pump
- **WHEN** a player holding a kit uses the damaged coolant pump's repair point
- **THEN** the body kneels at the point, the camera frames the coupling guard, the player turns its four screws out
  on the pump in 3D and the guard lifts off, showing a picture of the pump game in the pump

#### Scenario: Clicking the picture opens the game
- **WHEN** the docked player clicks the pump game's picture in the opened pump
- **THEN** the pump game is drawn over the engineering room, which keeps rendering around it
- **AND** Esc puts the game back into the picture with its round paused

### Requirement: A landed round shows in the room

A landed round SHALL change the machine's state cues (its lamp, sound and effects) for everyone near, and a fumble
SHALL apply the system's hazard in the room to every body within its radius.

#### Scenario: An arc flash at the switchboard
- **WHEN** a player fumbles a breaker round with a crewmate 1 m away
- **THEN** both take 10 HP and see the flash

### Requirement: The server lands rounds, not the client

The server SHALL land a round only for a body docked at that job, for the job's next round, and when the round's
play time is at least that level's `min_round_s`.

#### Scenario: A round too fast to be played
- **WHEN** a client reports a level 3 round played in less than its `min_round_s`
- **THEN** the server refuses it, the job's health does not change, and the client plays the round again

### Requirement: Leaving keeps landed rounds

A player who leaves or is pushed out of a repair SHALL keep every round already landed, and SHALL resume the
paused round within 30 s.

#### Scenario: A hit mid-round
- **WHEN** a docked player takes a 12 HP hit during the second round of three
- **THEN** the body is undocked, the first round's share stays landed, and docking again within 30 s resumes the
  second round where it paused
