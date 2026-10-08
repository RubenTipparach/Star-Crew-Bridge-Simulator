# Design: the armory, sidearms, rifles and armour

## Context

- `crew-on-deck` section 6: "A body holds one thing"; section 3's speeds: walk 1.8 m/s, run 4.0 m/s,
  wounded 1.4 m/s, suited 1.5 m/s, carrying 1.2 m/s. Section 7: `crew::injure(body, hp, cause)` is the
  one way a body is hurt.
- `command-suite`: the head and the bridge locker leave spare space for "an officer's cabin or a
  small armoury".

## Goals / Non-Goals

**Goals**
- Every crew member can defend themselves at once; better kit costs a walk to the armory and speed.
- Firefights are short and readable; armour is a choice, not a default.

**Non-Goals**
- A shooter's depth: no recoil patterns, no attachments, no grenades. A boarding fight is one beat of
  the evening, not the game.
- Weapons that harm the ship. A stray round does not breach the hull (a design choice, said plainly).

## Decisions

### 1. The kit

| Item | Where | Mass | Damage | Rate | Range | Magazine |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| Pistol | Every body's belt, from the start | 1.2 kg | 20 HP | 3 shots/s | 25 m | 12, 3 spare magazines |
| Rifle | The armory's rack, 6 | 4.0 kg | 30 HP | 8 shots/s | 60 m | 30, 4 spare magazines |
| Light armour (vest) | Armour lockers, 6 | 6 kg | Takes 40 % of each hit | | | |
| Heavy armour (suit) | Armour lockers, 2 | 18 kg | Takes 70 % of each hit | | | |

- Hit chance falls with range and with the shooter moving; a target behind a doorframe or a console is
  harder to hit (the compartment graph's cover points).
- Damage reaches the body through `crew::injure(body, hp, "gunfire")` after armour's share; armour is
  worn out after it has taken 200 HP (vest) or 500 HP (suit).

### 2. Speeds

| Gear | Effect | Why |
| --- | --- | --- |
| A rifle carried | x 0.95 | Two-handed and heavy |
| Light armour | x 0.90 | |
| Heavy armour | x 0.75, no running | Like a suit |

The speed function multiplies these into `crew-on-deck`'s speeds, then the slowest cap applies:
a wounded body in heavy armour with a rifle walks min(1.4, 1.8 x 0.75 x 0.95) = 1.28 m/s.

### 3. Hands and the holster

- The pistol rides in a belt holster outside the hands. Draw takes 0.4 s and puts any held item down
  at the body's feet (where it stays, as any dropped item does); holster takes 0.4 s.
- A rifle is a two-handed held item: it takes the hands, as a patch kit does; it can be slung onto
  the back (1.0 s) to free them, one sling per body.

### 4. The armory

- On deck A in the command suite's spare space by the bridge locker, about 2.4 x 3.0 m: a rifle rack,
  two armour lockers, an ammunition cabinet, a bench.
- Its door is locked by default (`crew-on-deck`'s lock); the security officer and the captain unlock
  it from their consoles, anyone else overrides it at the panel in 3.0 s, logged.
- Taking a rifle: Use at the rack, 1.0 s. Putting on a vest: 4 s; a heavy suit: 15 s.

## Risks / Trade-offs

- **Guns in a cooperative ship game invite friendly fire.** Friendly fire is off by default (a host
  setting), and a round never harms the ship.
- **Holster against "one thing in the hands".** The rule still holds for the hands; the holster is a
  slot of its own, said in `crew-on-deck`'s delta.

## Pi 5 budget

Per armed body one pistol prop (about 150 triangles) and, when carried, a rifle (about 300); the armory
once (racks and lockers about 1,500 triangles). Muzzle flashes are runtime lights, at most 4 at once,
inside `light-baking`'s runtime light budget. Network: the held item and the holster state ride the
snapshot's existing fields; a shot is one event (shooter, target or point, hit).
