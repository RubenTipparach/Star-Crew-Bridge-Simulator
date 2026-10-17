# Tasks: warp-to-mission

The owner asked on 2026-10-17 ("Missions also shouldn't start right away. Helm needs to warp somewhere and warping
somewhere takes a few seconds."; the proposal).

## 1. Write-up

- [x] 1.1 Proposal, design and spec delta, reusing FN 7's jump, PM 11.5's streaks and NM's JUMP panel.
- [x] 1.2 `docs/mockups/consoles.html`: helm's JUMP panel in ORIENT's cell in its three states, the band chip on
  another console; shots `warp-ready`, `warp-spool`, `warp-arrived`, `warp-band-tactical`, looked at; the contact sheet
  `docs/screenshots/mockups/warp-to-mission/contact-sheet.png`.

## 2. Gate

- [ ] 2.1 The owner approves the mockup (W2, W3, W6 with the contact sheet in the next survey).

## 3. Build (after approval)

- [ ] 3.1 Data: `drill-hound.json` `departure`, `jump.json` `departure_charge_mj`, `bindings.json` `helm.jump`, each
  validated.
- [ ] 3.2 `sc-core::combat`: START MISSION to Departure; the drill as a one-jump patrol from its departure; the
  departure attitude from the two places; the abort back to the charge at the departure; `patrol::spool_left_s`; the
  helm bot's and automation's wait; Departure with nobody seated back to Muster; tests (design 11).
- [ ] 3.3 `sc-net`: the patrol block on the drill's snapshot, its seconds left, `PROTOCOL` 19; round-trip tests.
- [ ] 3.4 `sc-client`: the JUMP panel in ORIENT's cell, the band chip, the key; parity states within the limits.
- [ ] 3.5 PM 11.5's stretch, flash and settle on the viewscreen and in every window; captures.
- [ ] 3.6 A playtest on foot (design 11) and captures in `docs/screenshots/engine/warp-to-mission/`.
