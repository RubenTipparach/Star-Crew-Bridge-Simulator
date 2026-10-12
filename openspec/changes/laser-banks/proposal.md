# Proposal: two laser banks and their stations

## Why

The owner, 2026-10-10, after watching the drill: "combat: cannon shots are mainly for strafing off fighters. I need two main laser banks. trhese take about 30 seconds each to build up, have their own capacitor systems for building up high amounts of energy and can fire at the enemy. Cannons will knock down shields slowly and do minimal damage to enemy ships. Shields will typically regen faster than cannon can damage, but lasers will deal large amounts of damage to shioelds and damage ships that dont have lost armor on a specific spot. so how do we strip away armor? Missiles. A ship will have lmiterd missiles: about 20 for this scenario. missiles take a minute to load. The fire, the enemy and your ship have counter measures: 5 of them. So they can cause a missle to miss or blow up prematurely if used right." Then: "implement the new laser firing stations. have the enemy launch two fighters as well so a player can assume the gunner seat and shoot them down. The enmey should be given the same tools and strategies to attack the player", "missiles can also damage the ships hull as well. and both missils and lasers can cause fire", and "document all this stuff in openspec, and implement this".

In the drill today four pulse cannons out-damage the shields (about 19 MJ/s at full rate against 1.5-3 MJ/s of
regeneration) and every hit goes into one hull pool, so the fight is a cannon race of about 70 s. The owner wants the
main weapon to be two laser banks, each charged over about 30 s from its own capacitors, that strip shields and hurt
the ship only where its armour is gone (`armour-and-missiles`).

## What Changes

- **Two laser banks a ship** (design 1): port and starboard on the Tern, each with its own capacitor (150 MJ, 30 s
  to charge from empty), its arc (the broadside half of the sky), and a beam that arrives at once.
- **A laser shot** (design 2): the facing shield takes the energy first; what passes reaches the armour of that spot
  and stops there while any armour is left; where the armour is gone it reaches the hull, and can start a fire or
  break a system (`hull-fires`).
- **Two laser stations** (design 3): LASER PORT and LASER STBD, in the bridge's comms and flight-ops seats, each a
  console with the charge, the arc, the target's facing spot (shield and armour) and FIRE. Automation fires a bank
  when it is charged and the shot is good, or early when the spot it bears on has no armour.
- **The enemy has the same** (design 4): the Hound gets two banks and fires them by the same rule.

## Impact

- `sc-core::combat`: the bank's state, the shot, the `LaserFire` command, two stations, automation; data in
  `data/weapons.json` (the laser) and each ship's combat file (its banks).
- `sc-net`: the banks in the snapshot, the beam as an event; `sc-client`: the two consoles and the beam drawn;
  `sc-server`: the seats.
- Supersedes `weapons-and-shields`' "beam weapons... later changes if ever" (Non-Goals) for the drill.
- Budget: the owner, 2026-10-10: "dont worry about overbudget". Costs are stated, not capped.
