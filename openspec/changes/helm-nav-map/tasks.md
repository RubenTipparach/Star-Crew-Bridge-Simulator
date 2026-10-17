# Tasks: helm-nav-map

The owner asked on 2026-10-16 ("Does the helmsman have a navigation map?", "Like where to warp to next") and set the
three system rules the same day (the proposal).

## 1. Write-up

- [x] 1.1 Proposal, design (with the research of section 2) and spec deltas.
- [x] 1.2 `docs/mockups/helm-nav-map.html`: the NAV tab's states, the captain's view, the kinds; shots looked at.
- [x] 1.3 The owner's survey: NM1, NM2, NM4, NM5, NM6, NM10 with the shots (CLAUDE.md 13). In Star Crew Owner Survey 2, section "Helm NAV map", 2026-10-16.

## 2. Build (after approval)

- [ ] 2.1 `sc-core::system`: `BodyKind`, `parent`, `rings`, `belts`, `parking_point`, `target_clear`; the rule checks of
  design 11.2 in `Validate`, with tests (a file without a gas giant, with one asteroid, with two rocky worlds, with a
  bad parent, each refused).
- [ ] 2.2 `sc-tools check-data`: no two system files share a sky; the sky file exists.
- [ ] 2.3 `data/space/systems/shoals.json`: design 11.3's delta; `data/ships/tern/jump.json` `parking_radii`.
- [ ] 2.4 `sc-core::combat::patrol`: the target, preselection, `Command::NavTarget` and its refusals, the jump to the
  target, stops left open when passed over; tests (a free point in Cinder's lock refused `MassLocked`; a plot during a
  spool refused `NotReady`; preselection after each arrival; a jump to a station off the route).
- [ ] 2.5 `sc-net`: the command's tag, the snapshot's target byte, `NavPlotted`, the protocol bump; round-trip tests.
- [ ] 2.6 The ALIGN autopilot mode (FN 10's point bow on the jump vector), with a test that it brings the bow inside 5
  degrees and under 2 deg/s.
- [ ] 2.7 `sc-client`: `console/navmap.rs` (the map), the NAV tab on helm and captain, TARGET, JUMP, ORDER, the
  viewscreen's target diamond; the parity states `nav-docked`, `nav-aligning`, `nav-refused`, `nav-captain` pass
  `tools/consoles/compare.py`.
- [ ] 2.8 Captures in `docs/screenshots/engine/helm-nav-map/`, and a playtest that walks to helm, sits, plots and jumps.

## 3. Gate

- [ ] 3.1 The owner signs off the mockup and engine captures side by side.
