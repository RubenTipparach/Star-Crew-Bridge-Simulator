# repairs-on-deck

## ADDED Requirements

### Requirement: A repair is played docked at the machine

A player SHALL start a repair by Use at the system's repair point while holding a kit, which docks the body at the
point, opens the service face and draws the system's mini-game over the live, darkened 3D view.

#### Scenario: Docking at the coolant pump
- **WHEN** a player holding a kit uses the damaged coolant pump's repair point
- **THEN** the body kneels at the point, the pump's coupling guard opens, and the pump game is drawn over the
  engineering room, which keeps rendering around it

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
