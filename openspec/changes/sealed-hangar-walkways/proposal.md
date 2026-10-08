# Proposal: sealed walkways beside the shuttle

## Why

The owner, 2026-10-08: "fighters seem like they can leave if they want to because they each have their
own bay, but the main shuttle is in an area that bridges two sections of the ship, so the area needs to
be like vacuume proof, the two walk ways on the side of the shuttle I mean, so if a shuttle were to
leave players wouldnt get sucked out".

Today the hangar is one air volume of 1,682.4 m3 that includes the two deck B galleries over the launch
bays, about 256 m3 each, and the landing with bay control. Its doors to the rest of the ship are
ordinary doors, which a crew member can force across a pressure difference. Launching the Petrel
empties all of it: everyone in the galleries must be suited or out, the pump-down takes 206.5 s, and the
deck B route between the forward ship and engineering through the galleries is closed while it lasts.

## What Changes

- **The galleries become their own compartments**, `gallery_p` and `gallery_s`, sealed from the hangar
  floor by a glazed pressure wall: long windows onto the shuttle, so the crew watch a launch from air.
- **The landing and bay control** move into a glazed control booth at the forward end, its own small
  compartment `bay_control_booth` between the two galleries, with a window over the pad.
- **Pressure doors** where the galleries and the booth meet the hangar floor's stairs: closed and
  interlocked whenever the hangar is below 80 kPa, as the airlock's doors are.
- **The pumped volume shrinks** to the hangar floor less the booth, about 1,120 m3, so a pump-down is
  about 138 s instead of 206.5 s and an emergency vent loses about 1,345 kg of air instead of 2,019 kg
  (scaled by volume; `life-support` reruns them with its own model before this is built).
- **The deck B route stays open during a launch**: the galleries keep their doors to engineering and the
  spine, so the ship is never cut in two on deck B.

## Capabilities

### New Capabilities
- `hangar-walkways`: the sealed galleries, the control booth, their windows and pressure doors, and the
  launch that leaves them in air.

### Modified Capabilities
None now; `reference-ship-tern` (the layout), `life-support` (the pump-down) and
`shuttle-bay-and-fighters` (who must be suited for a launch) take this change's deltas when it is built.

## Impact

- `data/ships/tern/layout.json` through a patch: three compartments carved from the hangar, five
  windows, four pressure doors.
- `life-support`: the hangar's volume, its pump-down and vent numbers, the new compartments' air.
- `shuttle-bay-and-fighters`: the Petrel's launch checklist no longer needs the galleries cleared.
- Pi 5 budget: about 60 triangles of glazing and frames per window, 5 windows; three more compartments
  in portal culling (no new draw calls beyond their geometry).
- Recommendation taken (ask only with screenshots) for every number here; the layout patch will come
  with shots for the owner.
