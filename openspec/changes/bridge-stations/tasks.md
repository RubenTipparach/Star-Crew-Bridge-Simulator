# Tasks

Nothing here is started. The mockup and its screenshots (section 1) are documentation tooling and
come with the write-up (CLAUDE.md 4); everything from section 2 on is engine work, taken on a
separate request once the owner approves the mockup (CLAUDE.md 10).

## 1. Write-up and mockup

- [x] 1.1 Proposal, design and spec deltas for `bridge-stations`.
- [x] 1.2 `docs/mockups/bridge.html`: the bridge from the layout, live viewscreen and windows, three lighting states, walk mode, every bridge console as an overlay matching the wireframes.
- [x] 1.3 Screenshots in `docs/screenshots/mockups/bridge-*.png`, looked at.
- [ ] 1.4 Put questions B1-B6 in the owner survey with their shots; fold the answers back into this change.
- [x] 1.5 The owner's Star Trek references described (`docs/analysis/star-trek-bridges.md`); three bridge variants as data (`tools/bridge_variants.py`, `data/ships/tern/bridge_variants.json`), drawn by `docs/mockups/bridge-variants.html` with Blender-built consoles; design section 11a.
- [ ] 1.6 Put B11 (which bridge) in the survey with the variant shots; when the owner picks, patch the layout with the variant (platforms as a layout field the checker validates), rewrite 11.1 from it and rerun `crew-on-deck`'s bridge routes.
- [ ] 1.7 Console faces (design 11.6):
  - [x] 1.7.1 Bake each station's console to a main and an upper 256 x 128 px screen image (`tools/mockups/console_screens.py`, `assets/textures/screens/`), reusing the wall panels' generic screen and key images.
  - [x] 1.7.2 Keyboard wells in the wall banks; levers, stick, guarded buttons, breakers and faders as station variants of the props (`tools/blender/build_bridge_props.py`), within budget; the variants file takes a station's variant.
  - [x] 1.7.3 Draw the screens and key panels in `bridge-variants.html` (one atlas, one draw), with close-up shots at helm's desk and engineering's bank.
  - [ ] 1.7.4 Ask B12 in the survey with the shots.

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

- [ ] 4.1 The console layer on egui and `egui_glow` (engine-stack E3): canvas, bands, fixed panel rectangles with their own clip rectangles, the theme, widgets, guarded controls, gamepad focus, scrolling, clipping; at most 16 draw calls and 6,000 triangles.
- [ ] 4.2 The console layouts and their previews, each calling its resolver.
- [ ] 4.3 The look band viewport and the look-up mode.
- [ ] 4.4 The viewscreen render target (1024 x 512, up to 30 Hz), feeds and the skip rule; the one secondary feed a console may show (512 x 256, up to 15 Hz) (with `ship-frames`).
- [ ] 4.5 Order chime, klaxon and console sounds.

## 5. Verification

- [ ] 5.1 Headless captures of every console and the bridge in three lighting states in `docs/screenshots/`.
- [ ] 5.2 Pi 5 probe run of the bridge scene: triangles, draw calls, frame time p50/p95/p99 standing and seated (owner, on hardware).
- [ ] 5.3 Move each requirement into `openspec/specs/bridge-stations/` with the test that proves it.
