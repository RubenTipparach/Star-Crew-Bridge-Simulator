# Tasks

Nothing here is started. The mockup and its screenshots (section 1) are documentation tooling and
come with the write-up (CLAUDE.md 4); everything from section 2 on is engine work, taken on a
separate request once the owner approves the mockup (CLAUDE.md 10).

## 1. Write-up and mockup

- [x] 1.1 Proposal, design and spec deltas for `bridge-stations`.
- [x] 1.2 `docs/mockups/bridge.html`: the bridge from the layout, live viewscreen and windows, three lighting states, walk mode, every bridge console as an overlay matching the wireframes.
- [x] 1.3 Screenshots in `docs/screenshots/mockups/bridge-*.png`, looked at.
- [ ] 1.4 Put questions B1-B6 in the owner survey with their shots; fold the answers back into this change.

## 2. Data

- [ ] 2.1 `data/stations.json`: roles, merge lists, automation tuning with units, order verbs, threat weights, auto-condition ranges, NPC post list, body swap settings; matched to the layout's stations by id.
- [ ] 2.2 `data/consoles/*.json` for helm, tactical, engineering, science, captain, comms, flight ops, engineering bay, damage board, bay control.
- [ ] 2.3 `data/input/bindings.json` and `data/ui/theme.json`.
- [ ] 2.4 Console validator: inside the grid, no overlap, the grid filled, text bands fit; a test per rule.

## 3. Core (`sc-core`)

- [ ] 3.1 The seat table: operator and occupant per station; claim, release, relieve, swap, reservations; test `swapping_away_hands_the_helm_to_automation_holding_course`.
- [ ] 3.2 Automation: per-station behaviour, competence from data, the refusal list, the computer core dependency; tests per refusal.
- [ ] 3.3 Merge resolution from merge lists; tests for one to eight players.
- [ ] 3.4 Condition, auto-condition with hysteresis, brace, orders and their lifecycle, `automation::can_execute`.
- [ ] 3.5 Threat score; test against a table of contacts.

## 4. Client (`sc-client`, `sc-render`)

- [ ] 4.1 The immediate-mode UI layer: canvas, bands, grid, widgets, guarded controls, focus, scrolling, clipping; at most 4 draw calls.
- [ ] 4.2 The console layouts and their previews, each calling its resolver.
- [ ] 4.3 The look band viewport and the look-up mode.
- [ ] 4.4 The viewscreen render target, feeds and the skip rule (with `ship-frames`).
- [ ] 4.5 Order chime, klaxon and console sounds.

## 5. Verification

- [ ] 5.1 Headless captures of every console and the bridge in three lighting states in `docs/screenshots/`.
- [ ] 5.2 Pi 5 probe run of the bridge scene: triangles, draw calls, frame time p50/p95/p99 standing and seated (owner, on hardware).
- [ ] 5.3 Move each requirement into `openspec/specs/bridge-stations/` with the test that proves it.
