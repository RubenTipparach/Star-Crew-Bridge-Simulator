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

Point defence is the twin pulse cannon turrets (the Tern's four, the Hound's two): a turret on AUTO or PD fires first
at an inbound missile within `pd_range_m` (900 m; any hit kills it), then (`enemy-fighters`) at a fighter within
2,000 m, then at the locked ship.

**Rounds** (owner, 2026-10-11: "they should be limited to 100shots with a 30 second cooldown"): every turret fires
100 bolts, then reloads for 30 s before it fires again (`rounds`, `reload_s` on the gun in `data/weapons.json`), on
both ships. A reloading turret draws as cooling on Tactical's TURRETS. Before this, point defence never ran dry and
shot down 75-100 % of the Hound's missiles, so almost nothing reached the Tern's hull.

## 5. The enemy

The Hound has the Tern's automation for Tactical: it locks, loads both tubes, fires when armed and the seeker sees,
throws decoys, and its turrets defend. A missile aims at a point on its target's long axis, drawn at launch along the
middle four fifths of the hull (`Missile::aim_m`), so hits land from bow to stern; aimed at the centre, they all
burst amidships around the Tern's hangar and broke only its pumps and pad. The face and section hit follow from the
geometry.

## 6. Tuning

A headless run of whole fights with automation on both sides (`sc-core` test, 20 seeds) reports the fight's length,
the hits by weapon and the systems broken. The target is a fight of 5-10 minutes that the Tern wins about two in three
with its crew on automation.

Measured 2026-10-11 with bots at Helm and Tactical, seeds 1-8 (`tuning_report`), after the rounds and the aim points:
every fight a victory in 376-513 s; the Hound lands 4-8 of 10-16 missiles; the Tern ends with 61-260 of 400 MJ of
hull; at the worst moment 1-7 systems are below nominal at once (the reactor's auxiliaries, the coolant pumps, the
gravity generator, the bridge's consoles, the tubes) with one or two fires, and the teams bring them back. The test
`two_bots_at_helm_and_tactical_win_the_drill` now holds the Tern hurt in every fight and a system broken in five of
six.

With the power grid in the drill (`power-grid` design 12a, the same day) the combat preset wants more than the reactor
gives, and on Engineering's automation the drive is shed whenever the guns and shields draw together. Seeds 1-8 with
bots at Helm and Tactical: 3 victories in 448-551 s, 1 defeat, and 4 fights run to the 600 s limit with the Hound at
18-70 of its 250 MJ. The test is now `two_bots_at_helm_and_tactical_take_the_hound_to_the_wire_with_engineering_on_automation`:
at least half won, every fight 4-10 minutes, the Hound below 60 % in every one. A player at Engineering is what turns
it (recommendation taken, ask only with screenshots: the drill stays winnable without one). With the conduit boxes
(`power-routing` design 4, the same day) a hit in the forward switchboard room can cut the bow off: 2 victories, 3
defeats, 3 fights to the limit; the test holds a win in six.

## 7. Cost

Up to 16 missiles in flight and two decoys; each missile in the snapshot gains its owner and target (4 bytes).
