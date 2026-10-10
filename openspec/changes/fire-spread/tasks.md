# Tasks: fire-spread

## 1. Write-up and mockup

- [x] 1.1 Proposal, design and spec delta from the owner's "a js mockup of putting out fires ... a fire propagation sim" (2026-10-08).
- [x] 1.2 `data/ships/tern/atmosphere.json` `fire.cells`: cell size, fuel classes, rise, dose weights, agent time; validated.
- [x] 1.3 `docs/mockups/lib/firespread.js`: the cell model (design 1-2, 5); `shipsystems.js` takes a room's heat release from it.
- [x] 1.4 `tools/mockups/fire_cases.mjs`: the table's cases with the cell model, careful and careless aim; calibrate design 4's target.
- [x] 1.5 `docs/mockups/fire.html`: walk and board views (design 6), normal and red alert, Pi cost; inlined by `tools/mockups/inline.py`.
- [x] 1.5a Venting as the captain's call (design 6a): the guarded vent with its preview of who is inside, the 5 s warning, harm as oxygen and heat fall; the mockup's vent and the harness's vented case.
- [ ] 1.6 Shots in `docs/screenshots/fire/` (taken 2026-10-09); published for the owner.
- [ ] 1.7 Design 6b: `fire.outbreaks` in `atmosphere.json`, `FireSpread.outbreaks` (seeded, validated), `lib/firefight.js`
  (flames, spray, safety points) moved out of `fire.html`, and outbreaks in `deck-plan.html`'s walk; shots.

## 2. Engine (after the owner has played it)

- [ ] 2.1 `sc-core::fire`: the cell model beside the atmosphere step, with the spec's scenarios as tests.
- [ ] 2.2 The harness's table replaces `damage-control`'s section 3 table in the same commit.
