# Tasks: concord-skin

The owner asked on 2026-11-02 ("redo the UI", "adopt the concord skin here").

## 1. Guide and write-up

- [x] 1.1 The `game-ui-frontend` skill copied from Fleet Command with an "In Star Crew" note; provenance recorded.
- [x] 1.2 Proposal, design and spec delta.

## 2. Mockup

- [x] 2.1 `docs/mockups/concord-ui.html`: every screen of design 5 in Concord beside today's look, from one set of
  tokens and kit pieces; shots in `docs/screenshots/mockups/concord-ui/` at 1920 x 1080 and 985 x 455.
- [x] 2.2 Published; survey 2 asks CS1 and CS2 with the shots. Concord UI: https://claude.ai/artifact/39yivDM2Sx6Sd5sHWP5jGy;
  Concord Station Consoles (`docs/mockups/consoles-concord.html`): https://claude.ai/artifact/VoMx8o4BVQnVskbtv6Buej.

## 3. Engine (after CS1 is approved)

- [ ] 3.1 Antonio in `assets/fonts/antonio/` with its licence; `tools/ui/fonts.py` checks it.
- [ ] 3.2 `look.rs` tokens to Concord's palette and three sizes; the kit pieces in the `vg` kit by name.
- [ ] 3.3 The menus, the HUD on foot, the Esc menu, the profile and loading drawn from the kit; captures beside the
  mockup.
- [ ] 3.4 `consoles.html` in Concord (design 6), then the engine's consoles; the parity check passes.
- [ ] 3.5 The web build's smoke test and a playtest capture of every screen.
