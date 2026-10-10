# Tasks: repairs on deck

## 1. Write-up and mockup

- [x] 1.1 Proposal, design and spec delta from the owner's "how we would perform these mechanics in the games 3d
  environment" (2026-10-09).
- [x] 1.2 `docs/mockups/repairs-on-deck.html`: engineering in 3D, the pump and the valve board docked, the covers unscrewed in 3D, the real games over
  the live room, rounds and fumbles shown in the room; shots in `docs/screenshots/repairs-on-deck/`.
- [x] 1.3 Design 3b: the covers baked in Blender (`tools/blender/build_repair_covers.py`) and inlined into the page;
  point at the opening and click to open the game, Esc back to it; shots.
- [x] 1.4 Design 3b revised (owner: "an image of like wiring or circuit board behind the panel"): the machine's
  insides, a circuit board and a terminal box with its loom, baked by the same tool in place of the game stills
  (the stills and their capture tool removed); shots.
- [x] 1.5 Design 3c prototypes: the coolant pump's terminal box and the valve cabinet's back door built into their
  meshes (`tools/blender/build_service_prototypes.py`, `assets/models/service_proto/`), covers and screws as their
  own props placed from `service.json`; `docs/mockups/service-panels.html` with shots.
- [x] 1.7 Design 3d prototypes: the hardware modelled into both bays (the terminal box's rail, blocks, fuses,
  contactor, relay and wires; the board's parts on a bare-board bake whose part list places them), the parts written
  to `service.json` as hit targets, the faulty one broken and smoking; `service-panels.html` lights a part under the
  pointer, says a sound one tests fine, and opens the 2D game from the faulty one; shots.
- [x] 1.8 Design 3e prototypes: each bay's hardware its own prop on a mounting plate (the machine's atlas back to its
  own colours), the faulty part a prop of its own; `service-panels.html` pulls it into the tray, fits a part from the
  pouch (a wrong one fails its test and comes back), then the whole bay is the calibrate target that opens the 2D game;
  shots.
- [ ] 1.9 Design 3f: tactical's console damaged in the deck plan's walk (sparks, scorch decals from
  `tools/decals/build_decals.py`, the screen's static), docked, the cover, the burnt card pulled and the right one
  fitted, the conduits game, the screen booting; `lib/stationrepair.js`; shots.
- [ ] 1.6 After the owner approves 1.5: the bays in the engineering props' builds, and the repair page docking at
  the side or back where they are.

## 2. Data

- [ ] 2.1 The `repair` use point kind and service faces on the engineering props' Blender builds.
- [ ] 2.2 `data/ships/tern/repairs.json` with jobs, rounds, levels and a measured `min_round_s`; validated.

## 3. Engine

- [ ] 3.1 `sc-core`: the job's rounds, the intents of section 6 and their checks; tests that a round lands at once and
  a round under `min_round_s` is refused.
- [ ] 3.2 `sc-client`: docking, the camera cut-in, the panel over the darkened live view, look up, leaving.
- [ ] 3.3 The machine's state cues (lamp, sound, effects) by band, and the round and fumble events.
