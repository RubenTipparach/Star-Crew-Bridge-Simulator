# Design: armour, twenty missiles and countermeasures

## Context

The owner's words are in the proposal. Numbers are the first set, tuned in section 6.

## 1. Armour by weapon

Armour is `ship-damage`'s: 54 sections a ship (9 spans x 6 faces; the Hound its own), integrity 0-100. What reaches a
section past the shield, R MJ, meets it by weapon:

| Weapon | Armour with integrity I | Passes into the ship | The section loses |
| --- | --- | --- | --- |
| Missile | absorbs min(R, 4 MJ x I / 100) | the rest | R / 40 MJ x 100 points (a 30 MJ blast strips 75) |
| Laser | holds all of it while I > 0 | all of it when I = 0 | nothing |
| Cannon | absorbs min(R, 4 MJ x I / 100) | the rest | min(R, 4) / 40 x 100 points (damage-control's rule) |

A missile's blast also meets the shield at half strength: the face takes up to half the warhead, the other half goes
on (blast and fragments).

## 2. Twenty missiles

| Field | Value |
| --- | --- |
| Magazine | 20 a ship for the scenario (the Tern's racks and the Hound's) |
| Tubes | Tern 2, Hound 2 |
| Load | 60 s, then arm 3 s |
| Warhead | 60 MJ (`gannet`), blast 30 m, seeker 30 degrees, 60 s of flight |

A missile keeps its owner and its target, so either ship can fire at the other, and a missile can be fired at a
fighter (`enemy-fighters`).

## 3. Countermeasures

Five a ship. `Countermeasure` (Tactical) throws a decoy burst that lasts 4 s around the ship. Each inbound missile
within 3 km that sees it rolls once, by its time to impact t:

| t | Pulled off (misses, flies on, self-destructs) | Set off early (bursts where it is) |
| --- | --- | --- |
| 1-3 s | 0.6 | 0.25 |
| 3-6 s | 0.3 | 0.15 |
| otherwise | 0.1 | 0.05 |

So the best time is in the last three seconds, and a decoy thrown early is wasted. Automation throws one when the
nearest inbound missile is 1.2-2.5 s out and none was thrown in the last 4 s.

## 4. Point defence

A turret on AUTO or PD fires first at an inbound missile within 1,200 m (any hit kills it, a 3 m radius), then
(`enemy-fighters`) at a fighter within 2,000 m, then at the locked ship.

## 5. The enemy

The Hound has the Tern's automation for Tactical: it locks, loads both tubes, fires when armed and the seeker sees,
throws decoys, and its turrets defend. Both ships' missiles are aimed at a ship's centre; the face and section hit
follow from the geometry.

## 6. Tuning

A headless run of whole fights with automation on both sides (`sc-core` test, 20 seeds) reports the fight's length,
the hits by weapon and the systems broken. The target is a fight of 5-10 minutes that the Tern wins about two in three
with its crew on automation.

## 7. Cost

Up to 16 missiles in flight and two decoys; each missile in the snapshot gains its owner and target (4 bytes).
