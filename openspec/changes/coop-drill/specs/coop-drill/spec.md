# coop-drill

## ADDED Requirements

### Requirement: The drill runs on the server and repeats

The server SHALL run the drill through Muster, Countdown, Engage and Debrief, and SHALL start a new Muster with the
next seed after each Debrief.

#### Scenario: Nobody aboard
- **WHEN** the server runs the drill with no client connected
- **THEN** it stays in Muster

#### Scenario: A drill ends
- **WHEN** the Hound's hull reaches zero
- **THEN** the drill moves to Debrief with a victory, and after the debrief to a new Muster

### Requirement: A station without a player is run by automation through the same commands

Every station the drill uses SHALL be operated, when no player holds it, by `sc-core::automation` at the
`automation` profile, issuing the same commands a console issues, through the same validation.

#### Scenario: One player
- **WHEN** one player holds Helm and nobody holds Tactical
- **THEN** Tactical's automation locks the Hound and sets the turrets to AUTO, and never fires a missile

### Requirement: The hit chance a console shows is the hit rate

The probability of a turret's bolt hitting SHALL be computed by one function that the turret's fire decision, the
automation and the Tactical console all use, and over many shots the observed hit rate SHALL agree with it.

#### Scenario: Calibration
- **WHEN** 4,000 bolts are fired at a fixed target whose predicted hit chance is 50 %
- **THEN** between 47 % and 53 % of them hit

### Requirement: Every network message is validated on arrival

A message SHALL be decoded by the one codec both ends share, and a message with a non-finite number, a value out of
range, a count over its cap or a truncated body SHALL be dropped and counted, never applied.

#### Scenario: A non-finite stick
- **WHEN** a client sends a stick axis that is NaN
- **THEN** the server drops the message and counts it
