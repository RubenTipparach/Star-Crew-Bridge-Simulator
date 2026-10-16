# leave-station

## ADDED Requirements

### Requirement: One key sits and stands at every station

The use key (E, from `data/input/bindings.json`) SHALL sit the body down at a seat in reach, and held for
`seats.stand_hold_s` while seated SHALL stand it up, at every station and in every phase. A hold SHALL count only
from a press made while seated.

#### Scenario: Standing at the helm in a fight
- **WHEN** a player seated at Helm in Engage holds E for 0.5 s
- **THEN** the body stands behind the seat, the helm passes to automation, and the ship does not roll

#### Scenario: The key that sat you down
- **WHEN** a player presses E to sit and keeps it held after the body is seated
- **THEN** the body stays seated until E is let go and held again

### Requirement: LEAVE STATION in every console's title band

Every console SHALL show LEAVE STATION in its title band, right of the order and left of the condition, with the stand
key drawn as a keycap whose ring fills while the key is held. A click or a tap on it SHALL stand the body up at once.

#### Scenario: Clicking LEAVE STATION mid-fight
- **WHEN** a seated player clicks LEAVE STATION in Engage
- **THEN** the body stands up and the station passes to automation or a bot

### Requirement: Helm rolls on Z and C

Helm SHALL roll on Z and C and strafe to port and starboard on the Left and Right arrows, from
`data/input/bindings.json`, leaving E to the stand.

#### Scenario: Rolling
- **WHEN** the helm holds C
- **THEN** the stick rolls to starboard
