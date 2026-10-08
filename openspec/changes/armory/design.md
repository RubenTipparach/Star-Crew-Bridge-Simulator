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

- On deck A, a room of its own (owner, 2026-10-08: "did you add armory room somewhere too? theres still room
  for more rooms on top deck"). With the command suite and deck access, deck A is empty aft of the head and the
  bridge locker on both sides of the aft passage (z -18 to 0). The armory takes the port side's forward end: 9.35 x
  6.0 m (x 1.25 to 10.6, z -6 to 0, 56 m^2, 168 m^3), from the passage out to the hull's line, inside the hull's
  0.5 m clearance at the ceiling's chamfer. Its door `p_armory` (1.0 x 2.2 m) opens from the aft passage at z -3.0,
  8 m aft of the command passage, so the bridge crew reach it in about 15 s at a run. It is a layout patch,
  `data/ships/tern/armory.json`, written and checked by `tools/armory.py` (the layout check with the hull, every
  piece inside the room, clear of the door and of each other).
- What stands in it, from the suite prop set (`tools/blender/build_suite_props.py`, built 2026-10-08):
  - on the aft wall, facing the door: the rifle rack (six rifles standing behind a hazard-striped locking bar,
    magazines on a shelf above, 688 triangles), the ammunition cabinet (two doors, a keypad, 100) and a workbench
    for cleaning and checking weapons;
  - on the forward wall: two armour racks (three vests on hangers and three helmets each, 388) and a locker bank
    for heavy suits;
  - on the hull side: a shelf of holsters and slings.
- The rest of deck A's spare space (the port side aft of the armory, z -18 to -6, and the starboard side, z -18 to
  0) is left for the rooms still to come: the security office (`security-station`), an officer's cabin.
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
once (its racks, lockers and benches 2,126 triangles, measured off the built props). Muzzle flashes are runtime lights, at most 4 at once,
inside `light-baking`'s runtime light budget. Network: the held item and the holster state ride the
snapshot's existing fields; a shot is one event (shooter, target or point, hit).
