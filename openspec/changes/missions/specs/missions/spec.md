# missions

## ADDED Requirements

### Requirement: The captain picks the mission

Before anyone is ready, the captain SHALL be able to pick a mission from every mission the server has, and every crew
member SHALL then be briefed on it.

#### Scenario: The Grabon demo
- **WHEN** the captain picks "Demo: The Grabon Raider" in the room
- **THEN** every briefing reads the raider's situation and the fight is against the Grabon raider

### Requirement: Warp out ends the mission

The Tern SHALL be able to jump out after a spin-up held at Helm, away from hostile ships, and the jump SHALL end the
mission: complete after a victory, escaped before one.

#### Scenario: Jumping home after the fight
- **WHEN** the raider is destroyed and Helm holds JUMP for the spin-up
- **THEN** the Tern jumps and every screen shows MISSION COMPLETE
