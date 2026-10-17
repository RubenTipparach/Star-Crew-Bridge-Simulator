# start-mission

## ADDED Requirements

### Requirement: The round waits in Muster until START MISSION is pressed

In Muster the drill SHALL NOT move to Countdown by itself, however many are seated and ready. It SHALL move to
Countdown only on a START MISSION request the server takes. READY SHALL stay a signal shown to the captain, not a
trigger. A bot SHALL be refused while a person is aboard; in a crew of bots alone a seated bot MAY start it.

#### Scenario: Everyone sits and is ready
- **WHEN** every person aboard is seated and ready in Muster and nobody presses START MISSION
- **THEN** the drill stays in Muster and no countdown is shown

#### Scenario: Sitting down on foot
- **WHEN** the last person aboard sits at their station in Muster on foot
- **THEN** they are marked ready and the drill stays in Muster

#### Scenario: START MISSION is pressed
- **WHEN** the player who may start presses START MISSION in Muster
- **THEN** the countdown begins, and only then does the briefing show "Launch in"

### Requirement: The captain may start the mission without waiting

In Muster the server SHALL move the drill to Countdown when a seated person sends START MISSION and either holds the
captain's seat or no person holds it, whoever else is ready or seated. It SHALL refuse the request, changing nothing,
from a bot while a person is aboard, from anyone not seated, from a seated person while another person holds the captain's seat, and outside
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
