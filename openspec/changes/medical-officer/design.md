# Design: a playable medical officer

## Context

`crew-on-deck` section 7 gives every body 100 HP, the states healthy, wounded (under 50), downed (0)
and critical (0 after 120 s), a 5.0 s revive by hand to 25 HP, and field recovery of 0.2 HP/s to
50 HP. Section 8 gives the medbay two beds healing 2.0 HP/s powered and 0.5 HP/s unpowered, reviving
a downed or critical body after 20 s. C4 chose revive by hand over a medkit for everyone; this change
gives a medkit to one role only, so C4 holds for the rest of the crew.

The owner, 2026-10-08: "a player can play as the medical officer to heal players".

## Goals / Non-Goals

**Goals**
- A role worth a player's evening: faster revives, field healing, tended beds, a console that shows
  who is hurt and where.
- Nothing about healing becomes impossible without a medic: the ship of four still works.

**Non-Goals**
- Disease, surgery, drugs and long-term injury beyond `crew-on-deck`'s C3 (a critical body ends the
  mission at 80 max HP).
- NPC medics. An NPC watch body does not take this role (it does not take stations today).

## Decisions

### 1. The role and its seat

- Role `medical`, station `medical`, compartment `medbay`, seat beside the beds facing them. A player
  takes it from the crew panel or by swapping to it from any seat (`bridge-stations`, swapping).
- The medic is not chained to the seat: like a gunner leaving a turret, they walk out with the
  medkit, and the console waits.

### 2. Rates

| Act | Anyone | The medic | Why |
| --- | ---: | ---: | --- |
| Revive a downed body by hand | 5.0 s, up to 25 HP | 2.0 s with the medkit, up to 40 HP | The medic is fast under fire; 40 HP is still wounded, so the bed still matters |
| Field healing | 0.2 HP/s to 50 HP, after 30 s unhurt | 4.0 HP/s to 75 HP, one dose per 25 HP | Gets a crew member back to running in seconds, not minutes |
| Bed, powered | 2.0 HP/s | 6.0 HP/s while the medic is within 3 m or seated at the console | 25 to 100 HP: 37.5 s alone, 12.5 s tended |
| Bed revive (downed or critical) | 20 s | 8 s tended | |
| Bed, unpowered | 0.5 HP/s | 1.5 HP/s tended | |
| Critical body | Can be carried; it stays critical | Stabilized with the medkit in 3.0 s: it can be carried without its window running down | |

- **The medkit**: 8 kg, one-handed, held like a repair kit (`crew-on-deck` section 6). 20 doses;
  the medbay's cabinet refills it in 4 s. A revive costs one dose, field healing one dose per 25 HP
  given, a stabilize one dose.
- **One rule for every rate.** `crew::heal_rate(bed_or_body, tender)` is the bed's and the field's
  rate, and the console's bars and the HUD's "full in 12 s" read it, so a preview never disagrees
  with the outcome (CLAUDE.md 6.1).

### 3. The medical console

Glance first (`bridge-stations` 8.0), four panels:
1. **CREW**: the ship's three decks small, every body a dot: green healthy, amber wounded, red
   downed (pulsing), red with a cross critical; a ring round a body that called for a medic. Tap one
   to see its HP, state, the window left and the walk time to it.
2. **BEDS**: a bar per bed, its body's name, "full in 12 s", powered or not.
3. **KIT**: doses left as a fill, REFILL when at the cabinet.
4. **CALL**: CALL TO MEDBAY (a ship-wide request that marks the medbay on every HUD) and the count of
   downed and critical bodies, the biggest numbers on the console.

### 4. Calling a medic

A downed player sees MEDIC on their crew panel. Pressing it rings the body on the medic's CREW plan
and on the medic's HUD with its distance; automation logs it on the damage board when no player
holds the role.

## Risks / Trade-offs

- **A medic makes death cheap.** Bounded: one medkit, 20 doses, and the medic can be downed too.
- **A ship without a medic must not be punished.** It is not: every rule of `crew-on-deck` still holds.

## Pi 5 budget

One more console of the existing kind: about 80 UI draw calls while it is shown, none otherwise. The
medkit is one prop under 300 triangles. No network change beyond one more held-item value (the
action nibble has 4 spare values, `crew-on-deck` section 14).
