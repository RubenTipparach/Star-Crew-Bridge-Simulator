# relieve-bots

## ADDED Requirements

### Requirement: A person takes any station a bot holds

A person SHALL be able to take any station a bot holds or is walking to, in any phase, by walking to its seat and
using it, or from the briefing. A person SHALL never take a station another person holds or is walking to, and a bot
SHALL never take one a person holds or is walking to. The server, the bots and the screens SHALL decide by one rule.

#### Scenario: The helmsman takes the captain's chair mid-fight
- **WHEN** a person seated at Helm in Engage stands, walks to the captain's chair a bot holds and presses E
- **THEN** the bot stands, the person sits and holds Captain

#### Scenario: A bot on its way
- **WHEN** a bot is walking to the captain's chair and a person within reach of it presses E
- **THEN** the person takes the seat and the bot is sent elsewhere

#### Scenario: Another person's seat
- **WHEN** a person presses E at a seat another person holds
- **THEN** the claim is refused and the prompt names who holds it

### Requirement: A relieved bot swaps with the person

A bot relieved by a person SHALL walk to the station that person just left when it is unmanned, else to the nearest
unmanned station, else to its muster place.

#### Scenario: The swap
- **WHEN** a person leaves Helm and relieves the captain's bot
- **THEN** that bot walks to Helm and holds it once seated
