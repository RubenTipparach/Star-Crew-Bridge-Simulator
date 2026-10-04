# Weapons and Shields

## Purpose

The Tern's turrets, manned and automated, its missiles from magazine to impact, point defence, the
six shield faces, the one damage resolution that hands hull hits to damage-control, and the enemies
of the early missions.

## ADDED Requirements

### Requirement: Turrets fire under one fire rule with capacitor, heat and arc limits
A twin pulse cannon SHALL fire at most 4 bolts per second, alternating barrels, only when its
capacitor holds 1.5 MJ, its heat is below lockout, it has supply, and its arc mask allows the
barrel's direction. Each bolt SHALL draw 1.5 MJ, add 0.3 MJ of heat, leave the muzzle at 1,500 m/s
plus the muzzle point's velocity, live 1.6 s, and deliver 1.2 MJ on a hit. The same fire rule SHALL
serve every gun, parameterized by data.

#### Scenario: Sustained fire drains the capacitor
- **WHEN** a turret with a full 24 MJ capacitor charging at 4 MW fires continuously
- **THEN** it fires at 4 bolts per second for about 12 s, then at about 2.67 bolts per second

#### Scenario: The hull blocks a shot
- **WHEN** the dorsal turret's barrel points at a fighter below the Tern's keel
- **THEN** the turret does not fire, and tactical's console shows the target outside its arc

### Requirement: Arcs are baked from the hull and leave no blind spot
Each turret's arc mask SHALL be computed offline from the layout's hull loft and mounts, at 5 degree
bins, from rays that clear the hull for 200 m within elevation -10 to +90 degrees. Every direction
around the Tern SHALL be covered by at least two turrets.

#### Scenario: Checking coverage after a layout change
- **WHEN** the arc tool runs on the Tern's layout
- **THEN** it reports each turret's share of the sphere and fails if any direction is covered by
  fewer than two turrets

### Requirement: One lead solver and one hit-chance estimate
The lead solver SHALL solve the bolt's intercept with the target's current relative position and
velocity, and the hit-chance estimate SHALL be `1 - exp(-R^2 / (2 sigma^2))` with sigma combining
aim error times range, the target's manoeuvre over the flight time, and track error. Gunners'
sights, automated turrets, point defence, enemy AI and tactical's console SHALL all call these two
functions. The estimate SHALL be calibrated: over seeded engagements, the observed hit rate in each
10% band of predicted chance SHALL fall within that band plus or minus 5 points.

#### Scenario: Preview and outcome agree
- **WHEN** 10,000 seeded bolts are fired by automation at Jackals at mixed ranges and manoeuvres
- **THEN** in every band of predicted hit chance the observed hit rate lies within the band plus or
  minus 5 points

### Requirement: A gunner aims through the turret sight
A seated gunner SHALL aim with mouse, stick or pad, the turret slewing toward the aim point at no
more than its traverse rates, and SHALL see the reticle, the selected target's lead pip, the hit
chance computed with the gunner's own measured aim error, range, the capacitor and heat, and the arc
edge. A manned turret SHALL ignore its automation mode and add no computer core load.

#### Scenario: Aiming faster than the turret
- **WHEN** a gunner flicks the mouse 90 degrees in yaw
- **THEN** the barrels reach the new direction after about 1.5 s, at 60 deg/s

### Requirement: Automated turrets follow tactical's priorities at a stated competence
An automated turret SHALL choose targets by tactical's priority list (missiles within 1,200 m,
fighters attacking its side, the designated target, the nearest hostile), acquire after 0.5 s, aim
with an error of `0.15 deg + 0.02 x angular rate in deg/s`, fire only when the estimated hit chance
is at least 4%, and load the computer core by 6% (9% in point defence). When the core is overloaded
its reaction and aim error SHALL scale by demand over capacity.

#### Scenario: A distant jinking fighter
- **WHEN** a jinking Jackal crosses at 200 m/s 1,500 m from an automated turret
- **THEN** the turret tracks it but holds fire, because its estimated hit chance is 2%

#### Scenario: A missile overrides a fighter
- **WHEN** an automated turret is firing at a Jackal and a missile enters 1,200 m inside its arc
- **THEN** it switches to the missile after its 0.5 s acquisition

### Requirement: Point defence stops missiles at a known rate
A turret firing at a missile SHALL use proximity bolts that destroy it within 1.0 m and an aim error
of 0.12 degrees. One turret SHALL stop a single missile closing at 1,000 m/s from 1,200 m about 56%
of the time, and two about 81%, in seeded tests.

#### Scenario: Two turrets against one Lance
- **WHEN** 1,000 seeded Lance missiles approach the Tern where two turrets bear
- **THEN** between 76% and 86% are destroyed before impact

### Requirement: Gannets go from magazine to target through tube states
The magazine SHALL hold 12 Gannets; the hoist SHALL move one to a ready rack in 20 s; a tube SHALL
load from a ready rack in 18 s automatically, 10 s with crew hands-on, or 40 s by hand without
power; arming SHALL take 3 s and safing 2 s. A tube SHALL be in exactly one of the states empty,
loading, loaded, arming, armed, fired, jammed or damaged. A load SHALL jam with the seeded chance
`0.01 + 0.25 x (1 - torpedo room health) + 0.3 x brownout`, halved with crew, and only crew SHALL
clear a jam, in 20 s.

#### Scenario: Automation cannot clear a jam
- **WHEN** tube 2 jams and nobody is in the torpedo room
- **THEN** tube 2 stays jammed, tactical sees "jammed: crew needed", and tube 1 keeps working

#### Scenario: Reloading under fire
- **WHEN** both tubes fire with both ready racks stocked
- **THEN** the autoloader has both tubes loaded 18 s later, and the hoist has restocked one ready
  rack 20 s after it started

### Requirement: Missiles are guided bodies that can be intercepted
A Gannet SHALL be handed to the system frame at launch with the tube's point velocity plus 30 m/s,
ignite 0.5 s later, boost at 120 m/s^2 for 6 s and sustain at 40 m/s^2 for 20 s, steer by
proportional navigation with N = 4 and at most 150 m/s^2 laterally, detonate on contact or within
5 m for 60 MJ falling linearly to 0 at 30 m, self-destruct at 60 s, and be destroyed by any bolt
that reaches it.

#### Scenario: Time to a target at 5 km
- **WHEN** a Gannet is launched at a stationary target 5 km ahead
- **THEN** it reaches it in about 9.7 s

### Requirement: Shields have six faces chosen by the normalized dominant axis
The Tern's shield SHALL be an ellipsoid of semi-axes 17.0 x 13.0 x 54.0 m centred at [0.0, 1.5,
-2.0] in the ship frame, with six faces (bow, stern, port, starboard, dorsal, ventral). A hit's face
SHALL be the largest component of its ship-frame offset from that centre divided by the semi-axes.
Total capacity SHALL be 240 MJ times the generator's health, regeneration `0.25 x delivered power`,
distributed by science's face weights; below 15% of nominal supply every face SHALL decay at
2 MJ/s.

#### Scenario: A hit on the side near the bow
- **WHEN** a bolt strikes the shield at ship-frame offset (12, 0, 20) m from its centre
- **THEN** the port face absorbs it, not the bow

#### Scenario: Shields without power
- **WHEN** the shield generator's supply falls to 10% of nominal
- **THEN** every face loses 2 MJ per second until it is empty or power returns

### Requirement: Only overflow reaches the hull, as a hull hit for damage-control
The damage resolution SHALL be one function for every ship and craft. A face SHALL absorb the hit's
energy times its band factor; if the face holds less, it SHALL empty and the remainder SHALL reach
the hull as `HullHit { ship, tick, point, normal, direction, energy, kind, source }` at the point
where the projectile's path meets the hull's collision mesh. Collisions SHALL bypass the shields.

#### Scenario: A Gannet on a weakened face
- **WHEN** a 60 MJ Gannet strikes a Hound face holding 20 MJ, with no band match
- **THEN** the face empties and a 40 MJ blast hull hit is reported at the hull point under the
  impact

#### Scenario: A matched frequency band
- **WHEN** science has matched the shield band to an attacker's band and a 1.0 MJ bolt hits a full
  face
- **THEN** the face loses 0.75 MJ

### Requirement: Early enemies follow their behaviours with seeded timers
Jackal fighters SHALL cycle orbit (800 m, 3-5 s), attack run (to 400 m or 6 s), fire (2.0 s, led
with the lead solver and their aim error) and retreat (8 s), with seeded phases and a clamped orbit
correction. Hound corvettes SHALL hold an 1,800 m orbit with their strongest face toward the target,
fire bursts of four every 3 s per turret and a Lance every 30 s. Both SHALL retreat at 25% hull.

#### Scenario: A group does not move in lockstep
- **WHEN** four Jackals spawn together in mission 2
- **THEN** their first attack runs start at different times, the same on every replay of that seed
