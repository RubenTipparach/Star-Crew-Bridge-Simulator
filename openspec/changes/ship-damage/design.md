# Design: hits that damage parts of the ship

## Context

The owner's words are in the proposal. The model is `damage-control`'s (design 1-7, data `damage.json`), proven in
the systems mockup's `resolveHit`; this design says how the drill runs it and what it adds. Where the two disagree,
the owner's 2026-10-10 combat direction wins (lasers do not strip armour; missiles do; `armour-and-missiles`).

## 1. Where a shot lands

1. **The shield.** A shot's path (a bolt's step, a beam's ray, a missile's blast point toward the ship's centre) meets
   the shield's ellipsoid; the face there (`face_of`) holds what it can. A cannon bolt counts at its weapon's
   `capital_factor` against a ship (0.25: cannons wear shields slowly).
2. **The hull.** What passes goes on along the path to the hull: the lofted octagons of `hull.sections` (each section
   a z, half-beam, top, bottom and chamfer, linear between). The entry point is where the path first goes inside,
   found by stepping 0.25 m and bisecting.
3. **The armour section.** The entry point's span (between two hull sections along z) and face (the path's main axis,
   as `hullSection`) name one of 54 sections, each with integrity 0-100.

## 2. What it does inside

As `damage-control` 2 and the mockup: the armour section takes its share (`armour-and-missiles` 1 by weapon); the
energy E left marches along the path in 0.25 m steps up to 30 m, each step depositing E (1 - exp(-0.25 / 4)); a
system (a power load, at its system's centre), a node or a conduit segment within r = 1.5 + 0.3 sqrt(E) m takes
10 points per MJ deposited, falling off with distance; a bulkhead crossed costs 1.5 MJ; the first room entered is
breached (the area recorded; air is not simulated in the drill yet). Integrity sets capability: 1 at 75 or more,
integrity / 75 between, 0 below 25. Each room the march deposits in may ignite: chance min(0.6, 0.1 x its MJ).
What reaches the hull past the armour also comes off the ship's structural hull (the number that ends the fight).

## 3. The cascade (the owner's)

When a hit takes points from a system already below nominal (75), a surge of 0.6 of those points goes into its node,
and from the node to every other load on it and, through each conduit, to the node at its other end, halving at each
step. A component the surge reaches takes its share; if it too was below nominal before the surge, the surge goes on
from it. It stops at 4 steps or below 1 point. Each component it damages is a burst (section 6).

## 4. Damage that matters

| System (layout id) | In the drill |
| --- | --- |
| `impulse_drive`, `rcs_*` | Thrust and turn rate times capability |
| `shield_generator` | Shield regeneration times capability |
| `sensor_array` | Lock time divided by capability (none: no lock) |
| `tube_1`, `tube_2` | Its tube loads and fires only at capability above 0 |
| `laser_port`, `laser_stbd` (new loads) | Its bank charges at capability times its rate; at 0 it holds |
| `turret_*` (mount loads) | Its turret fires only at capability above 0 |
| `reactor` | Every charge rate (banks, turrets) times capability |
| `computer` | Automation's reaction time divided by capability |
| `magazine_racks` | Hit while above 0: a cook-off (`damage.json` `cook_off`), resolved as a hit from inside |

## 5. Repairs and fires

- **Fires** burn in a room, taking 0.4 points a second from every system there and 0.05 MJ a second of hull, until put
  out. A fire not reached in 90 s spreads to a neighbouring room through an open portal.
- **Two teams** of two NPC crew (`damage.json` `teams`), at damage control, take the most urgent job: a fire, then the
  system with the highest priority (`data/stations.json`, the captain's REPAIR order raises one), then the lowest
  integrity. Travel takes the straight distance times 1.4 at the walk speed (on foot through the deck when
  `crew-on-deck` lands); a fire takes 20 s to put out; a repair restores 2 x 0.6 x 1.6 points a second (two ratings,
  two hands) up to nominal.
- **The crew of eight**: the five stations' people or bots plus at least five NPCs aboard; the teams are four of them.

## 6. Explosions

Events, server to clients: `HullHit` (ship, point in the ship's frame, the energy past the shield, the section),
`Burst` (ship, point, size: a system destroyed or a cascade). The client throws 6-20 debris pieces and sparks off the
point along the outward normal and its own spin, and a burst sphere.

## 7. The Hound's layout

A corvette 30 m long in `data/enemies.json` (`layout`): five hull sections, seven rooms (bridge, magazine, reactor,
hangar, port and starboard laser rooms, drive), loads (drive, reactor, shield generator, sensors, two banks, two tubes,
two turrets, the hangar's launch), two nodes and four conduits. Its armour sections are its own spans x 6.

## 8. Cost

Per hit: one hull test (up to 120 steps), one march (up to 120 steps over the systems in reach). A fight has a few
hundred hits that pass a shield. The snapshot gains each ship's system integrities (one byte each), fires and teams.
