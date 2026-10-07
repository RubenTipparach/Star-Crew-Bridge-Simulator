# Tasks: engineering fit-out

## 1. Write-up, props, data and mockup

- [x] 1.1 Walk engineering in first person and record what it holds (`docs/screenshots/mockups/engineering-before/`).
- [x] 1.2 Write the plant, the room by level, the pipe rules and the budget (design sections 2-7).
- [x] 1.3 Build the engineering set (`tools/blender/build_engineering_props.py`): 21 props within budget, ports in the manifest, atlases baked, `--check` reproducible; look at the contact sheet.
- [x] 1.4 Write and check the fit-out (`tools/engineering_fitout.py`, `data/ships/tern/engineering.json`): machines, runs, railings, the ring catwalk, the coolant pumps' system centre; the section 4 rules pass.
- [x] 1.5 Re-route the five feeders in `power.json` onto the trays (design section 4); the systems page draws them.
- [x] 1.6 The kit sweeps runs (tubes, elbows, flanges, collars, hangers) and draws railings along polylines; the deck plan places the machines, the control desk at `eng_main`, and dresses the mezzanine and catwalk floors.
- [x] 1.7 Measure engineering's triangles against 30,000 and correct design section 7's table; zfight under 0.1 m^2.
- [x] 1.8 Walk shots after, beside the before shots; look at them and fix what reads badly.
- [ ] 1.9 The pipework layer (design section 4): six rows baked by the panel build; the runs take them.

## 2. Elsewhere

- [ ] 2.1 `engine-stack` and `deck-pipeline`: engineering's ceiling of 30,000 in their tables.
- [ ] 2.2 `power-grid` decides whether any machine of design section 2's "not simulated" list becomes simulated.
- [ ] 2.3 `reference-ship-tern` T4: systems get their machines; the coolant pumps become two.

## 3. Verification

- [ ] 3.1 The owner walks engineering in the deck plan and judges it.
- [ ] 3.2 Move each requirement into `openspec/specs/engineering-room/` with the check that proves it.
