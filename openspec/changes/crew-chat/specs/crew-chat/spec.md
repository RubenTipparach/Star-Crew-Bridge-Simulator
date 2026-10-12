# crew-chat

## ADDED Requirements

### Requirement: The crew can talk

A crew member SHALL be able to send a line of chat that every crew member sees with the sender's name, in the room
before the fight and during it, and a crew member who joins late SHALL see the last lines.

#### Scenario: Two machines in the room
- **WHEN** a player on Pi 2 types "Ready at Tactical" in the room
- **THEN** the captain on Pi 1 sees "Ens. Dara Holt: Ready at Tactical" in the CHAT column

### Requirement: The NPC crew say what they do

An NPC crew member SHALL post a line in chat for each decision of its station that changes the ship, through the same
chat a player uses.

#### Scenario: The helm's NPC sits down
- **WHEN** an NPC sits down at Helm
- **THEN** every crew member sees "Lt. Sil Saren (NPC): At Helm"
