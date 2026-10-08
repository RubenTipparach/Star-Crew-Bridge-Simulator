# Tasks: ship props

## 1. Write-up, props and mockup

- [x] 1.1 Tour every compartment in first person and list what the first designs left (design section 1).
- [x] 1.2 Build the machinery set (`tools/blender/build_machinery_props.py`), each prop within its budget, `--check` reproducible; render its contact sheet and look at it.
- [x] 1.3 Furnish the crew rooms (`tools/crew_rooms.py`, `data/ships/tern/crew_rooms.json`), checked on the layout, the suite and deck access.
- [x] 1.4 The deck plan places the bridge's and the suite's props, every station's console and chair, every system's machine and the craft; a block only where no set models it.
- [x] 1.5 The kit: lamp lenses centred, ladders through floor hatches, the lift car's materials; propkit's glowing strips and shared space views.
- [x] 1.6 Tour again, look, fix what is found; the Pi 5 budget table (design section 5) from the deck plan's counts.
- [x] 1.7 No z-fighting (design section 4a): `tools/mockups/zfight.mjs`, every mockup under 0.1 m^2 of fighting surfaces.
- [ ] 1.8 Seats (design section 4b): the two upholstery layers, the remodelled captain's and crew chairs, tinted per seat in the pages; renders and shots looked at.
- [ ] 1.9 A custom atlas per prop (design section 4c): bridge set, then machinery, then suite; the pages map them; contact sheets and shots looked at.

- [x] 1.10 Doors (design section 4g): the `doors` set in Blender, one prop per leaf size within budget, `--check` reproducible; the lift car's `door_m` in its data; the page slides each leaf whole and clips it at its jamb; shots of every size shut, half open and open, looked at.

## 2. When the layout takes the patches

- [ ] 2.1 `reference-ship-tern` T4 sets system sizes; rebuild the machines to them.
- [ ] 2.2 The deck compiler (`deckc`) places props from the same rules and data, and counts them per compartment against `deck-pipeline` section 11.

## 3. Verification

- [ ] 3.1 Captures of each machine room and the crew rooms in the three lighting states (owner).
- [ ] 3.2 Move each requirement into `openspec/specs/ship-props/` with the check that proves it.
- [x] 3.3 The owner's walk of 2026-10-08 (design 4d): platform steps as ramps, quick starts and stops, a jump, the lift car's panels, door leaves with their rules, free-standing panels' backs, the floor holes lined, round machines round, two impulse units, twelve berths; shots `docs/screenshots/mockups/walk-fixes-2026-10-08/`.
