## ADDED Requirements

### Requirement: A medical officer tends the beds
A body holding the medical role SHALL make a medbay bed heal three times as fast while it is within
3 m of the bed or seated at the medical console: 6.0 HP/s powered and 1.5 HP/s unpowered, and a bed
SHALL revive a downed or critical body in 8 s instead of 20 s while tended. The rates SHALL come from
one function that the console's previews also read.

#### Scenario: A tended bed
- **WHEN** a body at 25 HP lies on a powered bed with the medical officer seated at the console
- **THEN** it reaches 100 HP after 12.5 s, and the console's bar said 12.5 s when it lay down

#### Scenario: No medic aboard
- **WHEN** no body holds the medical role
- **THEN** the beds heal at 2.0 HP/s powered, as `crew-on-deck` section 8 says

### Requirement: The medkit revives and heals in the field
The medical officer's medkit SHALL revive a downed body in 2.0 s to 40 HP, heal a wounded body at
4.0 HP/s up to 75 HP, and stabilize a critical body in 3.0 s so its window stops while it is carried.
It SHALL hold 20 doses, spend one per revive, per stabilize and per 25 HP given, and refill at the
medbay's cabinet in 4 s. Any other body revives by hand as `crew-on-deck` says.

#### Scenario: A revive under fire
- **WHEN** the medic with a stocked medkit holds Use over a downed crew member
- **THEN** the crew member gets up at 40 HP after 2.0 s and the kit has one dose fewer

### Requirement: A downed player can call the medic
A downed or critical player SHALL be able to call for a medic from the crew panel; the call SHALL
mark the body on the medical console's plan and on the medic's HUD with its distance, and SHALL be
logged on the damage board when no player holds the role.

#### Scenario: A call from engineering
- **WHEN** a downed engineer presses MEDIC
- **THEN** the medic's HUD shows the engineer's marker with the walking distance to them
