# Proposal: armour, twenty missiles and countermeasures

## Why

The owner, 2026-10-10: "so how do we strip away armor? Missiles. A ship will have lmiterd missiles: about 20 for this
scenario. missiles take a minute to load. The fire, the enemy and your ship have counter measures: 5 of them. So they
can cause a missle to miss or blow up prematurely if used right", "missiles can also damage the ships hull as well",
and "The enmey should be given the same tools and strategies to attack the player". The full words are in
`laser-banks`' proposal.

Today the Tern alone carries missiles (12, 18 s to load, the target hard-coded to the Hound), the enemy has none, no
ship has armour, and point defence never shoots at a missile.

## What Changes

- **Armour by section and by weapon** (design 1): `ship-damage`'s 54 sections; a missile strips the section it hits and
  passes the rest into the ship; a laser stops at any armour left; a cannon bolt is held by it.
- **Twenty missiles, a minute to load** (design 2): each ship's magazine holds 20 for the scenario; a tube takes 60 s
  to load and 3 s to arm; both ships fire them, at any target.
- **Five countermeasures a ship** (design 3): a decoy burst that can pull an inbound missile off or set it off early;
  how well depends on when it is used.
- **Point defence shoots missiles** (design 4): turrets on AUTO or PD take inbound missiles first.
- **The enemy uses all of it** (design 5): the Hound loads, fires and decoys by the same automation.

## Impact

- `sc-core::combat`: tubes and missiles for both ships, the `Countermeasure` command, decoys, point defence; data in
  each ship's combat file and `data/weapons.json`. Armour itself is in `ship-damage`'s module.
- `sc-net`: missile owner and target, decoys, the magazine and countermeasures left; `sc-client`: TUBES and a DECOY
  control on Tactical (mockup first), flares drawn.
- `weapons-and-shields` 9-10 (point defence, the Gannet) are the full design; these numbers are the drill's.
