# Design: two laser banks and their stations

## Context

The owner's words are in the proposal. The other half of the model, armour and missiles, is `armour-and-missiles`;
fighters and the cannons' new role are `enemy-fighters`; fires and broken systems are `hull-fires`. The numbers here
are the first set, tuned by running whole fights headless (`armour-and-missiles` 6) toward a fight of 5-10 minutes.

## 1. A bank

| Field | Tern | Hound | Unit |
| --- | --- | --- | --- |
| Banks | 2 (port, starboard) | 2 (port, starboard) | |
| Capacitor | 150 | 150 | MJ |
| Charge | 5 (empty to full in 30 s) | 5 | MW |
| Least charge to fire | 40 | 40 | % |
| Range | 8,000 | 8,000 | m |
| Aim sigma | 0.05 + 0.01 x the target's angular rate | same | degrees |
| Arc | its broadside half of the sky, 10 degrees past the keel line | same | |

A bank charges whenever it is not firing, from the ship's power (the grid is not simulated yet, so at its fixed rate).
Its data live in `data/weapons.json` (`lasers.<id>`) and each ship's combat file (`lasers`: id, position, arc).

## 2. A shot

1. **Fire** spends the whole capacitor, E MJ, in one beam that arrives at once (light). It needs the ship's lock on
   the target, the target in the bank's arc and range, and at least the least charge.
2. **Hit or miss** is one roll of the same `hit_chance` the cannons use, with the laser's sigma and the target's
   radius: the number the console shows is the chance.
3. **On a hit**, the target's face toward the beam (`face_of`) takes up to E from its shield. What passes, R, meets
   that face's armour (`armour-and-missiles` 1): with any armour left it stops there, and the armour loses nothing (a
   laser does not strip armour); with none, R reaches the hull, with a chance of a fire and a broken system there
   (`hull-fires` 1).

## 3. The stations

- **LASER PORT** and **LASER STBD** join the drill's stations (8 with `enemy-fighters`' GUNNER). Their seats are the
  bridge's comms seat (port, x +6.9 m) and flight-ops seat (starboard, x -6.9 m) in the layout, named in
  `data/stations.json`.
- **The console**, four panels in the consoles' style (CLAUDE.md 10): CHARGE (the capacitor filling, seconds to full),
  ARC (the ship from above with the bank's arc and the target in it or not), SPOT (the target's face the beam would
  meet: its shield as a ring, its armour as a plate, struck through when gone), and FIRE (the biggest control, its
  hit chance in its tooltip). The objectives tab is on it (`objectives-tab`).
- **Automation** fires a bank when it is full and the hit chance is at least 0.5, or at the least charge when the spot
  it bears on has no armour and the chance is at least 0.35.

## 4. The enemy

The Hound has the same two banks, charged and fired by the same automation; its lock is its own (`armour-and-
missiles` 4). Its AI turns a broadside toward the Tern while a bank is ready.

## 5. Cost

One beam event per shot (two a ship every 30 s), two numbers a bank in the snapshot. On the client a beam is a line
drawn for 0.4 s. The consoles add two screens.
