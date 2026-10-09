## ADDED Requirements

### Requirement: A medical officer tends the beds
A body holding the medical role SHALL make a medbay bed heal three times as fast while it is within
3 m of the bed or seated at the medical console: 6.0 HP/s powered and 1.5 HP/s unpowered, and a bed
SHALL bring round an incapacitated, stabilized or critical body in 8 s instead of 20 s while tended. The rates SHALL come from
one function that the console's previews also read.

#### Scenario: A tended bed
- **WHEN** a body at 25 HP lies on a powered bed with the medical officer seated at the console
- **THEN** it reaches 100 HP after 12.5 s, and the console's bar said 12.5 s when it lay down

#### Scenario: No medic aboard
- **WHEN** no body holds the medical role
- **THEN** the beds heal at 2.0 HP/s powered, as `crew-on-deck` section 8 says

### Requirement: The medkit stabilizes and heals in the field
The medical officer's medkit SHALL stabilize an incapacitated body in 2.0 s (anyone else with a first aid kit in
5.0 s, `crew-on-deck`), heal a wounded body of 20 HP or more at 4.0 HP/s up to 75 HP, and SHALL NOT raise an
incapacitated body's HP. It SHALL hold 20 doses, spend one per stabilize and per 25 HP given, and refill at the
medbay's cabinet in 4 s.

#### Scenario: A stabilize under fire
- **WHEN** the medic with a stocked medkit holds Use over an incapacitated crew member at 9 HP
- **THEN** after 2.0 s the crew member's vitals stop falling, it is still at 9 HP and must be carried to the medbay, and the kit has one dose fewer

### Requirement: An incapacitated player can call the medic
An incapacitated or critical player SHALL be able to call for a medic from the crew panel; the call SHALL
mark the body on the medical console's plan and on the medic's HUD with its distance, and SHALL be
logged on the damage board when no player holds the role.

#### Scenario: A call from engineering
- **WHEN** an incapacitated engineer presses MEDIC
- **THEN** the medic's HUD shows the engineer's marker with the walking distance to them
