## ADDED Requirements

### Requirement: Every crew member carries a holstered pistol
Every crew body SHALL start a mission with a pistol in a belt holster that does not occupy its hands.
Drawing the pistol SHALL take 0.4 s and put any held item down at the body's feet; holstering it SHALL
take 0.4 s.

#### Scenario: Carrying a repair kit armed
- **WHEN** a crew member walks with a repair kit in hand
- **THEN** their pistol is still in its holster and can be drawn

### Requirement: Gear slows the body
Carrying a rifle SHALL multiply a body's speeds by 0.95, light armour by 0.90 and heavy armour by 0.75
with no running, through the one speed function that applies `crew-on-deck`'s caps.

#### Scenario: Heavy armour and a rifle, wounded
- **WHEN** a wounded body in heavy armour carries a rifle
- **THEN** it walks at 1.28 m/s and cannot run

### Requirement: Weapons hurt only through the one injury function
A weapon's hit SHALL reach a body through `crew::injure` with the cause "gunfire", after the body's
armour takes its share (40 % for light armour, 70 % for heavy), and SHALL never damage the ship.

#### Scenario: A rifle hit on a vest
- **WHEN** a 30 HP rifle round hits a body in light armour
- **THEN** the body loses 18 HP and the vest has taken 12 HP of its 200
