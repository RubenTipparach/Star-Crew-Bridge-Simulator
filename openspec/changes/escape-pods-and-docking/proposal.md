# Proposal: escape pods and a docking port

## Why

The owner, 2026-10-08: "escape pods and decompression chambers for entry/exit into space or for external
docking purposes".

The Tern has one airlock (POI 24, deck C, off cargo) used for EVA, and no escape pods: only a Swift's
ejection pod exists (`shuttle-bay-and-fighters`). `ship-frames` already says how docking works (docking
portals hand bodies from one interior frame to another), but the Tern has no docking port mount.

## What Changes

- **Escape pods**: three pods of four seats, twelve in all, for eight crew and the four damage control
  team members. Two on deck B beside the spine and one on deck A by the bridge, each through a hatch.
  The captain's ABANDON SHIP arms them; a pod launches 10 s after its hatch seals, or at once by its own
  red handle.
- **A docking port**: the airlock's outer door gets a docking collar (a mount `docking_port`), so the
  same chamber serves EVA and docking. A docked craft's interior joins the Tern's through the airlock's
  cycle, or straight through when both sides are at pressure.
- **A second airlock** on deck A, aft, for EVA when the first is lost or busy (`deck-access`'s rule:
  more than one way, since parts of the ship can be damaged).

## Capabilities

### New Capabilities
- `escape-and-docking`: escape pods, abandoning ship, the docking port and the second airlock.

### Modified Capabilities
None now; `reference-ship-tern` (the layout and a mount), `life-support` (the second airlock's cycle),
`ship-frames` (a pod is a craft handed to the exterior frame) and `bridge-stations` (ABANDON SHIP on the
captain's console) take this change's deltas when it is built.

## Impact

- A layout patch: three pod bays with hatches, the second airlock and its doors, the mount.
- `security-station`: the docking port and the airlocks are entry points.
- Pi 5 budget: each pod is a prop of about 800 triangles and, launched, a craft in the exterior scene.
- Recommendation taken (ask only with screenshots); the layout patch will come with shots for the owner.
