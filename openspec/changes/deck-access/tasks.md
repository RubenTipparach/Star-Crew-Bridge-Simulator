# Tasks: deck access

## 1. Write-up, patch and mockup

- [x] 1.1 Measure how the decks connect and what one lost compartment cuts off, today and with the command suite (`tools/deck_access.py`, `single_losses`).
- [x] 1.2 Proposal, design and spec deltas, from the owner's "can we get starways on the sides? elevators?" and "we need to install multiple ways to go up and down since various parts of the ship can be damaged" (2026-10-06).
- [x] 1.3 `tools/deck_access.py` writes `data/ships/tern/deck_access.json` (after the suite) and checks it: the layout rules, the suite's furniture in its carved rooms and clear of every door, nothing inside a trunk; the single losses with and without the lift's power; the route times.
- [x] 1.4 The kit draws spiral stairs and a lift car (`ShipKit.buildSpiralStair`, `buildLiftCar`); the deck plan shows the proposal by default, with Ship buttons for the suite alone and today.
- [x] 1.5 Shots of the towers, the lift and the deck plans, looked at; survey Y1 with them, Y2 and Y3 recorded as recommendations taken; T3 answered.
  The first shots showed the towers' lower flights near black: the kit's lamp rule gave a 10 m shaft one lamp at its top. A trunk now takes a lamp a deck (design section 2, "Light"); the costs in section 6 are remeasured with it.

## 2. Apply the patch (with `command-suite`'s task 2)

- [ ] 2.1 `layout.json` takes the patch; `layout_check.py` learns the `trunk` kind and the `spiral_stair` and `lift` fixtures.
- [ ] 2.2 `power.json` (the lift's load), `atmosphere.json` and `damage.json` (three compartments) take it.
- [ ] 2.3 Rerun every plan-dependent number (design section 7); `crew-on-deck` models a spiral stair and the lift, and `tools/walk_times.py` routes by them.

## 3. The deck build (`deckc`)

- [ ] 3.1 Compile the trunks, spiral stairs and lift; tests `one_lost_compartment_cuts_off_only_dead_ends`, `a_tower_opens_both_ways`, `the_lift_stops_without_power`.

## 4. Verification

- [ ] 4.1 Captures of the towers and the lift in the three lighting states; triangles on the Pi 5 probe (owner).
- [ ] 4.2 Move each requirement into `openspec/specs/deck-access/` with the check that proves it.
