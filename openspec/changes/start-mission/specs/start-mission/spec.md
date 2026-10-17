# start-mission

## ADDED Requirements

### Requirement: The round starts by itself when everyone is ready

In Muster the drill SHALL move to Countdown once at least one player is seated, every seated player is ready, and, on
foot, every person aboard is seated.

#### Scenario: Everyone sits
- **WHEN** the last person aboard sits at their station in Muster
- **THEN** the countdown begins without anyone pressing START MISSION

### Requirement: The captain may start the mission without waiting

In Muster the server SHALL move the drill to Countdown when a seated person sends START MISSION and either holds the
captain's seat or no person holds it, whoever else is ready or seated. It SHALL refuse the request, changing nothing,
from a bot, from a person not seated, from a seated person while another person holds the captain's seat, and outside
Muster.

#### Scenario: The captain starts with the crew still walking
- **WHEN** the person in the captain's seat sends START MISSION while another person is still walking to a seat
- **THEN** the countdown begins

#### Scenario: Another station asks
- **WHEN** the person at Helm sends START MISSION while a person holds the captain's seat
- **THEN** the server refuses it with "Captain's call" and the drill stays in Muster

#### Scenario: Single player
- **WHEN** the player, seated at Helm with an NPC in the captain's seat, presses START MISSION before the crew have sat
- **THEN** the countdown begins

### Requirement: START MISSION in the title band while the round waits

While the drill is in Muster, a console whose player may start SHALL show START MISSION in its title band, in the
order's place, with the bound key (Return, from `data/input/bindings.json`) on a keycap and how many of those aboard
are ready. A click, a tap or the key SHALL send the request.

#### Scenario: The captain's console in Muster
- **WHEN** the captain sits at their console in Muster with three of eight aboard ready
- **THEN** the title band shows START MISSION with Return on its keycap and 3/8
