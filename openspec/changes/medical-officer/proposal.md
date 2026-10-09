# Proposal: a playable medical officer

## Why

The owner, 2026-10-08, listing in-ship activities: "medical bay: ship can be damaged, a player can
play as the medical officer to heal players".

`crew-on-deck` (sections 7 and 8) already hurts bodies and heals them: below 20 HP a body is
incapacitated, anyone with a first aid kit stabilizes it and it is carried to the medbay, a body out of danger recovers to 50 HP on its own, and above 50 HP only a medbay bed
heals, at 2.0 HP/s with nobody tending it. Nothing there is a job. A fifth to eighth player has no
reason to stand in the medbay, and the crew have no one to call when three of them are incapacitated in a
burning engineering bay.

## What Changes

- **A medical officer role**, a station like the others (`bridge-stations` roles), with its seat at a
  new **medical console** in the medbay (POI 8, deck B). Like every station it runs with or without a
  player: without one, automation keeps the beds running at today's untended rates and nothing more.
- **Tending is what the medic adds.** A bed the medic tends heals at 6.0 HP/s instead of 2.0, and
  revives a downed or critical body in 8 s instead of 20.
- **The medkit**: the medic's one held item (8 kg, carried like a repair kit). With it the medic:
  - revives a downed body in 2.0 s, up to 40 HP, where anyone else takes 5.0 s for 25 HP;
  - heals a wounded body in the field, 4.0 HP/s up to 75 HP, from 20 doses;
  - stabilizes a critical body for transport, so it can be carried to a bed without getting worse.
- **The medical console** is one glance-first console (`bridge-stations` 8.0): the ship's plan with
  every body as a dot coloured by its state, a bar per bed, CALL TO MEDBAY, and the doses left.
- **Calls for a medic.** A downed player's crew panel gets a MEDIC button that marks them on the
  medic's plan and on their own HUD, with the distance.

## Capabilities

### New Capabilities
- `medical`: the medical officer role, tended beds, the medkit, the medical console and the call
  for a medic.

### Modified Capabilities
None. `crew-on-deck`'s health states, revive and medbay rules stay as they are for everyone else;
this change adds the medic's rates on top of them, through the same functions.

## Impact

- `crew-on-deck`: the held items table gains the medkit; section 8's bed rates gain the tended rate
  (one function, `crew::heal_rate(bed, tended_by)`, so the console's preview is the bed's real rate,
  CLAUDE.md 6.1).
- `bridge-stations`: a role `medical` and its console; the station count grows by one, off the bridge.
- `reference-ship-tern`: a station `medical` in the medbay, seat beside the beds.
- `power-grid`: the console's draw (0.5 kW, on the medbay's bus).
- Pi 5 budget: one more console of the same kind (UI draw calls only); one 8 kg prop. No new rooms.
- Recommendation taken (ask only with screenshots) for every number here.
