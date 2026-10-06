# Tasks: ship props

## 1. Write-up, props and mockup

- [x] 1.1 Tour every compartment in first person and list what the first designs left (design section 1).
- [x] 1.2 Build the machinery set (`tools/blender/build_machinery_props.py`), each prop within its budget, `--check` reproducible; render its contact sheet and look at it.
- [x] 1.3 Furnish the crew rooms (`tools/crew_rooms.py`, `data/ships/tern/crew_rooms.json`), checked on the layout, the suite and deck access.
- [x] 1.4 The deck plan places the bridge's and the suite's props, every station's console and chair, every system's machine and the craft; a block only where no set models it.
- [x] 1.5 The kit: lamp lenses centred, ladders through floor hatches, the lift car's materials; propkit's glowing strips and shared space views.
- [ ] 1.6 Tour again, look, fix what is found; the Pi 5 budget table (design section 5) from the deck plan's counts.

## 2. When the layout takes the patches

- [ ] 2.1 `reference-ship-tern` T4 sets system sizes; rebuild the machines to them.
- [ ] 2.2 The deck compiler (`deckc`) places props from the same rules and data, and counts them per compartment against `deck-pipeline` section 11.

## 3. Verification

- [ ] 3.1 Captures of each machine room and the crew rooms in the three lighting states (owner).
- [ ] 3.2 Move each requirement into `openspec/specs/ship-props/` with the check that proves it.
