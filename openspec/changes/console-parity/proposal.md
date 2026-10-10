# Proposal: the engine's consoles look like their approved mockups

## Why

The owner, 2026-10-10, on the stations side by side (https://claude.ai/artifact/7fVi9j3e4Vkt8AFqrQUzLp): "Do you see
that stations are not appearing the same as their approved mockups? This is a problem."

They are not the same. The co-op drill (`coop-drill`, PR #13) built Helm and Tactical in the engine, and each keeps the
approved console's four panel names and little else:

- **The look band is gone.** The approved consoles have:
  - a viewscreen strip with its camera pad and the ship's attitude thumbnail;
  - the navball;
  - the station chips and the captain's order in the title band.

  The engine draws its panels over a full-screen 3D view, with a target camera inset that no mockup has.
- **Most controls are missing.** Helm lacks:
  - the strafe pad and the scanner's range zoom;
  - the autopilot modes (Hold, Course, Chase, Match, Evade);
  - the joystick and the twist lever;
  - the orient thumbwheels, Go and Flip.

  Tactical lacks the contact list, the 3D shield plot, each turret's dial, gunner and mode, the hit ring on Fire and
  the magazine pips.
- **Controls the mockups do not have were added:** Lock and Relock, a Weapons Free bar, shield presets and tube states
  as words, a drill header, round trip and packet loss on the console.
- **The look is different:** another typeface, plainer panels, no icons, and mixed-case labels where the mockups use
  letter-spaced capitals with a coloured tick.

The cause is in how the drill was made, not in one line of code:
- **The design was words, not the picture.** `coop-drill` design 5 described the consoles in words from
  `bridge-stations` 8, and the engine was built from those words.
- **Two copies.** The approved mockup (`docs/mockups/consoles.html`, SVG) and the engine (`crates/sc-client/src/drill.rs`,
  egui) are two hand-written copies of one screen.
- **Nothing compares them.** The drill's captures were judged on their own (CLAUDE.md 6.6: "validate the real
  artifact"; 10: "Mockup first").

## What changes

1. **The approved mockup is the screen's specification.** An engine screen shows what its approved mockup shows, in
   the same places, with the same look. A control the mockup does not have is not added in the engine; it goes into
   the mockup first and is approved there. Where the engine cannot yet do what a control does, the control is still
   drawn, in the mockup's unavailable state.
2. **One look, read out of the mockup.** The colours, role colours and icons are read out of the mockup's own text
   into `data/ui/console_style.json`, which the engine compiles in, and the engine uses the mockup's typeface
   (`assets/fonts/barlow-semi-condensed`, made into TrueType by `tools/ui/fonts.py`). Both have a `--check`.
3. **A check that compares the real pictures:**
   - The mockup saves the state each of its named shots draws; the engine draws the same state with the drill's own
     console code.
   - `tools/consoles/compare.py` lays the two pictures side by side and fails when a region of the console is missing
     part of what the mockup draws (a control missing, extra, recoloured or moved).
   - The side-by-side pictures go to the owner for sign-off.

   The check runs in `scripts/check.sh`.
4. **Helm and Tactical rebuilt to parity:**
   - the look band, title band and status strip, and every panel of both consoles;
   - the drill's extras mapped to the controls the mockups already have (design 5);
   - the core features the missing controls need (autopilot modes, strafe, orient and Go, Flip, turret modes),
     added to `sc-core::combat` with tests.
5. **Engineering, Science and Captain** are built from their mockups when their simulations exist, under the same
   check.

## What it touches

- `data/ui/console_style.json` (new), written by `tools/ui/console_style.py`; `assets/fonts/barlow-semi-condensed/*.ttf`
  (new), written by `tools/ui/fonts.py`.
- `docs/mockups/consoles.html`: `window.consoleState()`, the state a named shot draws. Its pictures are unchanged.
- `crates/sc-client`: the console drawing moves out of `drill.rs` into `console/` and `vg.rs`, following the mockup;
  `seat.rs` turns the drill's state into the console's; `sc-client --console-fixture` draws a saved state.
- `crates/sc-render`: the viewscreen feed is drawn into the look band's rectangle under the UI.
- `crates/sc-core/src/combat` and `crates/sc-net`: the commands and rules the missing controls need, with tests, and
  protocol 2 to carry them.
- `tools/consoles/parity.mjs`, `tools/consoles/compare.py`; steps 11-13 of `scripts/check.sh`.
- `CLAUDE.md` 10 and 12: the rule and the check's rows.
- `coop-drill`: its design points here for the consoles' look and for the rules added.
