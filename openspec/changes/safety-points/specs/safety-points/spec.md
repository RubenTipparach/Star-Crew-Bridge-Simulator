# safety-points

## ADDED Requirements

### Requirement: A safety point inside every room's main door

Every room except airlocks, turret pods and escape pods SHALL have a safety point (an extinguisher bracket and a first
aid cabinet) on the wall beside its main door, inside, clear of the door's opening by at least 0.4 m.

#### Scenario: Running into the quarters
- **WHEN** a crew member enters the quarters through its main door
- **THEN** an extinguisher and a first aid kit are on the wall within 1.5 m of the door

### Requirement: No floor more than 10 m's walk from a safety point

On every deck, every walkable floor point SHALL be within 10 m's walk, by the deck's walk graph, of a safety point.

#### Scenario: The far end of engineering
- **WHEN** the placement tool runs on the Tern
- **THEN** it reports the longest walk to a safety point on each deck, and none is over 10 m

### Requirement: A first aid kit stabilizes and holds three doses

A first aid kit SHALL hold 3 doses, SHALL stabilize an incapacitated body for one dose as `crew-on-deck` says, SHALL
give no HP, and SHALL be refilled only at the medbay cabinet.

#### Scenario: The fourth casualty
- **WHEN** a crew member has stabilized three bodies with one first aid kit
- **THEN** the kit is empty and a fourth stabilize needs another kit or a refill at the medbay

### Requirement: An emptied bracket restocks in 15 s

When an extinguisher is taken from a safety point's bracket, the bracket SHALL hold a full extinguisher again
`fire.extinguisher.bracket_restock_s` (15 s) later, and SHALL show the time left as a ghost extinguisher filling from
the bottom up while it is empty. The extinguisher taken SHALL keep the agent it has.

#### Scenario: Taking the quarters' extinguisher
- **WHEN** a crew member takes the extinguisher from the quarters' bracket
- **THEN** the bracket is empty, and 15 s later it holds a full extinguisher, whatever the one taken has left

#### Scenario: Putting one back
- **WHEN** a crew member puts an extinguisher into an empty bracket before its 15 s are up
- **THEN** the bracket holds that extinguisher at once and its timer stops
