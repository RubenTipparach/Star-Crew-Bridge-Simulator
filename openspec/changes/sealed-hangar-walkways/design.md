# Design: sealed walkways beside the shuttle

## Context

- `reference-ship-tern`: the hangar (POI 15) is three brushes: the floor, x -5 to 5, z -18 to 0, y -3.5
  to 3.0 (1,170 m3), and two deck B galleries over the launch bays, x 5 to 10.4 and -5 to -10.4, y 0 to
  3 (about 256 m3 each). The landing (fixture `hangar_landing`, 10 x 2 m at z -1) with the bay control
  station stands inside the floor brush. Total 1,682.4 m3.
- `life-support` sections 13 and 14: a hangar pump-down to 5 kPa takes 206.5 s; an emergency vent loses
  2,019 kg, 74 % of the reserve; the pumps refuse to start with an unsuited person inside.
- `crew-on-deck` section 5: an ordinary door can be overridden by hand across more than 20 kPa, which
  is how a body is pulled out.

## Goals / Non-Goals

**Goals**
- Nobody in the walkways beside the shuttle can be blown out by a launch, suited or not.
- A launch takes less time and less air, and leaves deck B's route open.

**Non-Goals**
- The fighters' launch bays: each already has its own pressure door and pump-down.
- Moving the shuttle or the pad.

## Decisions

### 1. Three compartments carved from the hangar

| Compartment | From | Volume | Holds |
| --- | --- | ---: | --- |
| `gallery_p` | The port gallery brush | about 256 m3 | The walkway, its doors to engineering and the spine |
| `gallery_s` | The starboard gallery brush | about 256 m3 | The same, mirrored |
| `bay_control_booth` | The landing, 10 x 2 m at z -1, 2.5 m high | about 50 m3 | The bay control station, its window over the pad |
| `hangar` (what remains) | The floor brush less the booth | about 1,120 m3 | The pad, the shuttle |

### 2. Walls, windows and doors

- Each gallery's inner edge (x 5 and x -5, z -18 to 0) becomes a pressure wall with two windows each
  1.2 m high and 6 m long at eye height, frames per `deck-pipeline` 5a.
- The booth has a window 8 m long over the pad.
- A pressure door (1.0 x 2.2 m) from each gallery to the hangar floor's stair, and one from the booth;
  `life-support`'s interlock keeps them shut below 80 kPa on the hangar side, and no override opens them
  (unlike an ordinary door, `crew-on-deck` section 5).

### 3. A launch

1. The checklist asks only that the hangar floor be clear or everyone on it suited.
2. The pumps run on about 1,120 m3: pump-down about 138 s to 5 kPa (`life-support` reruns it with its own
   pump curve; 206.5 s x 1,120 / 1,682).
3. The pad doors open; the shuttle leaves; the crew in the galleries and the booth stay in air, at a window.
4. An emergency vent loses about 1,345 kg (2,019 kg x 1,120 / 1,682), about half the reserve.

### 4. Routes

The galleries keep their doors to engineering (`p_gallery_eng_p`, `p_gallery_eng_s`) and to the spine, so
the forward ship and engineering stay joined on deck B through a launch; `deck-access`'s route checks are
rerun on the patched layout.

## Risks / Trade-offs

- **Less of the shuttle seen up close.** The windows keep it in view; the hangar floor is still walked
  when it is in air.
- **Numbers by proportion.** The pump-down and vent figures scale with volume here; `life-support`'s own
  model gives the ones that ship.

## Pi 5 budget

Five windows (about 300 triangles with frames), three compartments in the portal graph, no new lights
beyond the booth's two lamps.
