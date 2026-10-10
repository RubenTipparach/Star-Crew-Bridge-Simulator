# Proposal: repair mini-games, and the medic's treatment

## Why

The owner, 2026-10-08: "lets think about how we're gonna do repairs for various systems, every component that needs
repairs will have a different mini game", naming the gravity generator ("without it everyone has to float around
slowly"), the mess hall ("crew will starve between missions"), life support ("we'll all suffocate"), the fighters,
the shuttle, the reactor core, the impulse engines, the warp pylons ("that need EVA to repair, cant do it in combat"),
the toilets and showers, the medbay, "ALL the weapons!", the shield generator, the sensors and "electrical conduits
throughout the ship"; then "by think about I mean make JS mockups of these, also we need a medbay minigame for healing
crew, someone needs to be medical".

`damage-control` 6 and 6a already say a repair is done in person, with a kit, at a rate (1.8% of integrity a second
for an officer, 0.6% for a rating) that one function owns. What the player's hands do during those seconds is
undecided: today it would be holding a key. This change makes it a short game of its own for each kind of system,
without moving the rate the board previews for bots and teams.

The owner, 2026-10-09, after playing them: "If a repair needs more than one round, like say you need 3 rounds to
repair something I would prefer that repair minigame to be harder variants of the same minigame", and "I also dont
like waiting for the slider to slowly go up, if I repair something it should go up in health immediately".

## What Changes

- **One rule for every mini-game** (design 1, revised 2026-10-09): a repair is a few rounds of the system's game; a
  finished round raises the system's health by its share at once, a fumble costs a set share and the system's own
  hazard (a spark, a vent of gas, a heat spike). Ratings and bots repair without a game, at their rate.
- **Rounds get harder** (design 1a, owner 2026-10-09): every round of a job is the same game at the next level, never
  a different activity.
- **A mini-game per system** (design 2): seventeen, each drawn from what the machine is, all playable with a mouse or
  a pad in a few seconds a step, one screen each, glance first.
- **The medic's treatment** (design 3): triage on a body chart, the right tool for each wound, a steady trace to close
  it, with the vitals that `medical-officer`'s rates already set.
- **What a broken system does to the crew** (design 4): gravity off makes everyone float slowly; the galley down means
  hunger between missions; life support down means the air runs out on `life-support`'s clock; the heads down is a
  morale and comfort hit. The ones without a rule today are proposed here.
- **Warp pylons** (design 5): the Tern has none. Proposed as a hull addition that can only be repaired on EVA and only
  out of combat.
- **Mockups**: `docs/mockups/repairs.html`, every mini-game playable in the browser, flat 2D like the consoles.

## Impact

- `damage-control` (6, 6a: the steps and fumbles sit on its rates; its board shows steps), `medical-officer` (2, 3:
  treatment on its rates), `life-support`, `power-grid` (conduits), `weapons-and-shields` (turrets, tubes, shield
  emitters), `shuttle-bay-and-fighters`, `engineering-fitout` (reactor, impulse units), `ship-interactables` (the heads
  and showers), `crew-on-deck` (zero gravity movement), `crew-npcs` (bots repair without a game).
- The engine builds each mini-game in the UI layer (`lobby` 3) once the owner has played the mockups.
