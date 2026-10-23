# warp-to-mission

## ADDED Requirements

### Requirement: A mission starts at its departure point, away from the fight

When START MISSION is pressed for a mission with a departure, the drill SHALL move from Muster to Departure with the
Tern still at the departure place's arrival point, its bow on the jump vector to the destination, and no hostile
placed.

#### Scenario: The drill starts at Tamsin Station
- **WHEN** START MISSION is pressed in the drill's Muster
- **THEN** the Tern waits at Tamsin Station's arrival point, 5,780 m out on its axis, with the bow on the Shoals relay
  and no Hound in the scene

### Requirement: Helm jumps to the mission with the jump drive

In Departure, helm's JUMP held 0.6 s SHALL spool the jump drive from the berth's charge (`departure_charge_mj`) under
the jump drive's own rules (power, alignment over the last `align_last_s`, mass lock), and the jump SHALL move the Tern
to the destination's arrival point, where the mission's fight begins.

#### Scenario: A whole, powered drive
- **WHEN** helm holds JUMP in Departure with the drive whole and 20 MW delivered to the spool
- **THEN** the Tern jumps 6.0 s later and the drill's Engage begins with the Hound 6 km ahead

#### Scenario: Half the power
- **WHEN** the spool gets 10 MW
- **THEN** the jump comes 12.0 s after the hold

#### Scenario: A later jump
- **WHEN** the Tern spools again after its first jump
- **THEN** the spool takes the full 800 MJ (40 s at 20 MW)

### Requirement: A helm without a person jumps after the countdown's seconds

When no person holds the helm in Departure, the helm bot or the helm's automation SHALL hold JUMP once Departure has
lasted `mission.countdown_s`; a person at helm SHALL be left to jump themselves.

#### Scenario: Single player at tactical
- **WHEN** the player, seated at Tactical, presses START MISSION with the helm NPC on AUTO
- **THEN** the helm NPC holds JUMP 5 s into Departure

### Requirement: Every console shows the jump in START MISSION's place

In Departure and Warp every console's title band SHALL show, in START MISSION's place, the destination and either who
the jump waits on or the seconds left on the spool; helm's HELM tab SHALL show the JUMP panel in ORIENT's cell.

#### Scenario: The spool on another console
- **WHEN** the spool has 2 s left
- **THEN** the tactical console's band shows the destination and 2 s
