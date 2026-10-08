# Proposal: the armory, sidearms, rifles and armour

## Why

The owner, 2026-10-08: "ship armory: crews can get guns, eeryone has a pistol, but can grab armor and
rifles, but would suffer some movement speed if they get more gear".

Boarders (`security-station`) need something to be fought with. Nothing in the design arms a body:
`crew-on-deck` says "a body holds one thing", has no holster, and lists hand-to-hand combat and
boarders as a later change. This is that change's kit half.

## What Changes

- **Everyone carries a pistol**, holstered on the belt. A holstered pistol does not fill the hands, so
  a crew member still carries a repair kit, an extinguisher or a casualty; drawing it (0.4 s) puts the
  held item down at their feet, holstering (0.4 s) frees the hands again.
- **An armory** on deck A: the command suite's spare space beside the bridge locker (`command-suite`
  design, "an officer's cabin or a small armoury"), with a rifle rack (6 rifles), armour lockers (6
  vests, 2 heavy suits) and an ammunition cabinet. A locked door the security officer or the captain
  opens.
- **Gear slows the body.** Light armour 10 % slower, heavy armour 25 % slower and no running, a rifle
  carried 5 % slower. The slowest rule wins with the others multiplied in, through the one speed
  function `crew-on-deck` already has.
- **Weapons hurt through `crew::injure`**, the one way a body is hurt, with the cause recorded for the
  debrief. Armour takes a share of each hit.
- **A rifle is a held item** (two-handed): it fills the hands like a patch kit does.

## Capabilities

### New Capabilities
- `armory`: sidearms, rifles, armour, the armory room, their speeds and their damage.

### Modified Capabilities
None now. `crew-on-deck`'s held items and speeds take this change's delta when it is built (the
holster is a slot outside the hands; the speed table gains armour and the rifle).

## Impact

- `crew-on-deck`: a belt slot for the pistol; three rows in the speed table; draw and holster in
  the action nibble (it has 4 spare values).
- `command-suite`: the armory in the spare space by the bridge locker; `layout.json` a compartment
  and a door.
- `security-station`: its boarders and its orders use these weapons.
- Pi 5 budget: a pistol and a rifle prop on every body that carries one (about 150 and 300
  triangles), the armory's racks once; muzzle flashes are `light-baking`'s runtime lights (at most
  4 at once).
- Recommendation taken (ask only with screenshots) for every number here.
