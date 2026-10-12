# Design: enemy fighters, the gunner and the cannons' new role

## Context

The owner's words are in the proposal; `weapons-and-shields` 13 designed the Jackal and 6 the manned turret.

## 1. The Jackal

| Field | Value |
| --- | --- |
| Count | 2, launched 20 s into Engage from the Hound's hangar, 5 s apart |
| Hull | 3 MJ; a shield bubble of 4 MJ, regenerating 0.5 MJ/s |
| Flight | 300 m/s top, 60 m/s^2, turns 90 degrees/s |
| Guns | one twin cannon: 1.0 MJ bolts at 6 Hz, 1,200 m/s, 1.5 s of life |
| Hit radius | 4 m |

**Its AI** (`weapons-and-shields` 13): orbit the Tern at 800 m; every 12-20 s pick a part (a turret, a laser bank, the
bridge) and make a run at it from the side that part faces, firing from 1,200 m to 300 m; break off and climb out to
the orbit; repeat. It dodges (jinks) when a turret bears on it. Destroyed fighters stay destroyed.

## 2. The cannons

A bolt carries its gun's damage. Against a ship it counts `capital_factor` 0.25 of that at the shield and the armour
(`ship-damage` 1): the Tern's four turrets on the Hound give under its 1.5 MJ/s of regeneration on one face. Against a
fighter or a missile it counts in full. Turret targeting (`armour-and-missiles` 4): inbound missiles within 1,200 m,
then fighters within 2,000 m, then the locked ship.

## 3. The gunner

- **The seat**: GUNNER joins the drill's stations, seated at `gunner_dorsal` (the dorsal pod, deck A); the walk there
  is `crew-on-deck`'s; until the pod's ladder is in the walk grid, the body walks to the pod's foot and the seat is
  taken from there.
- **Commands**: `GunnerAim { yaw, pitch }` (the barrel's direction in the ship's frame, degrees, sent at 20 Hz while it
  moves) and `GunnerTrigger(bool)`. The turret fires along the aim while the trigger is held, at its rate and
  capacitor, with the manned sigma (`weapons-and-shields` 6: half the automated base).
- **The console** is a sight, not panels (`bridge-stations` 10.11): the 3D view along the barrel at 30 degrees, the
  chosen target ringed, a lead pip where to aim, its hull and shield as two arcs, the capacitor and heat as two bars,
  and TAB to choose the next target. The objectives tab is on it (`objectives-tab`).

## 4. Fighters hit parts

A fighter's bolts are aimed at its chosen part's point on the hull; where they land is `ship-damage`'s, so a run can
break a turret or a bank.

## 5. Cost

Two more craft in the snapshot (about 30 bytes each); their bolts are the drill's bolt events.
