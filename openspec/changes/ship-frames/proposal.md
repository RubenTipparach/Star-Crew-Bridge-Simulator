# Proposal: ship frames, the interior decoupled from the exterior

## Why

The owner, 2026-10-04: "Ideally a custom engine because our ship needs to decouple the bridge
and internal ship aspects from the exterior of the ship." This change is that decoupling, and
it is the reason the engine is ours (`engine-stack`, proposal, fact 3).

A crew of four to eight walks the Tern's decks while the ship rolls, turns at 18 degrees a
second and burns at 15 m/s^2. If the crew stood in the same physics world as the hull, three
things would break at once:

1. **The crew would be thrown around by every manoeuvre**, through the walls, because a
   character controller cannot hold its footing on a floor that accelerates at 1.5 g and
   rotates under it. Helm would be flying the crew, not the ship.
2. **Precision would fail.** The Tern travels a star system tens of AU across. A crew member's
   position stored as `f32` at 1 AU from the origin is quantized to 16 km. The decks need
   millimetres.
3. **The network would pay for it.** An avatar's position in the system frame changes every
   tick even when the avatar stands still, because the ship moves. In a fixed interior it
   changes only when the avatar walks (`netcode-and-sessions`, section 5).

star-crew-64 solved this by keeping the interior and the exterior as two separate scenes and
showing the exterior only in a 120 x 90 picture-in-picture (`docs/analysis/star-crew-64.md`,
section 4). It kept the right idea, separate frames, but drove the two from different state and
never let a craft cross between them. Pale-Blue-Dot's rules state the general form: `f64`
system coordinates, bounded `f32` local frames with the origin subtracted before the cast,
rebasing at a tick boundary, and passengers simulated in the station's local frame
(`/home/user/Pale-Blue-Dot/CLAUDE.md`, "Non-negotiable world and flight invariants").

## What Changes

- **Named frames.** A system frame (`f64` metres and seconds, the star at the origin, bodies on
  analytic rails), a ship frame (the ship's pose in the system: `f64` position, quaternion
  orientation), an interior frame (ship-local `f32` metres, static decks, artificial gravity
  along -Y), a near frame (system axes, origin at a ship: collision and the network), craft
  and turret frames, and the camera. Every pose, velocity, network message and save names its
  frame.
- **The interior is a fixed space.** Crew, loose objects, craft on cradles, missiles in tubes
  and every interior fixture live in the interior frame. The ship's motion never moves them
  through physics.
- **The inertial dampers.** What the crew feel of the ship's motion is computed per
  compartment: the demand (the proper acceleration at that point of the hull), the dampers'
  cancellation (limited by their rated capacity, their power and their health, with a 0.05 s
  lag), and the residual. The residual is applied on purpose to crew bodies and to cameras
  as a lurch; hits add a shake. Formulas and constants are in the design.
- **Rendering composition per frame on OpenGL ES 3.0** (the Raspberry Pi 5 floor,
  `engine-stack`). Viewscreen render-to-texture, then the exterior far layer and near layer
  drawn from the eye transformed by the ship's pose (camera relative `f32`, masked by a stencil
  to the visible windows), then a depth clear, then the interior pass, then glass and UI.
  Turret sights, fighter cockpits and chase views use the same passes.
- **The viewscreen** is a 1024 x 512 render target from a steerable virtual camera in the ship
  frame, refreshed at 30 Hz; up to two secondary feeds (a turret sight or a fighter's camera on
  a console) are 512 x 256 at 15 Hz.
- **Floating origin and precision.** The camera's eye is the render origin every frame;
  collision near a ship uses that ship's near frame. Tables of `f32` and depth-buffer error
  at each range set the limits (near layer to 20 km, origin never more than 10 km from what
  is drawn with it).
- **Ship attachments.** Turret heads, doors, cradles and a craft riding a cradle are posed in
  the ship frame and drawn with the ship's pose; they never hold an `f64` pose of their own.
- **Hand-off at a tick boundary.** A fighter, shuttle or missile leaves the interior frame for
  the system frame at one tick, with the ship's velocity at that point plus its ejection
  velocity, and returns on recovery the same way. A snapshot never holds a craft in both.
- **Other ships.** Another ship's interior is never rendered unless a local player is aboard it
  or it is docked and seen through the docking portal. Docking makes the smaller vessel an
  attachment of the larger; crew crossing a docking portal change frame at that tick.

## Capabilities

### New Capabilities

- `ship-frames`: the frames and their transforms, the fixed interior, the inertial dampers'
  residual and the hit shake, the floating origin and precision limits, the rendering
  composition (exterior, depth clear, interior, viewscreen), ship attachments, the hand-off of
  craft between frames, and docking between ships.

### Modified Capabilities

None.

## Impact

- `sc-core`: a `frames` module that owns every frame transform (the one implementation that
  CLAUDE.md 6.1 names), the dampers model and the hand-off phase of the tick.
- `sc-render`: the pass order and the window scissor, the viewscreen target, the far and near
  exterior layers, attachment drawing.
- `sc-net` and `netcode-and-sessions`: frame ids on every pose; this change states the frames,
  netcode states their encoding.
- `crew-on-deck`: consumes the per-compartment residual and shake (how a body stumbles, falls
  or floats is its rule).
- `power-grid`: the dampers' draw (its `inertial_dampers` load: 8 MW nominal, 2 MW standing by)
  and supply ratio.
- `damage-control`: the dampers' health; hull hits feed the shake.
- `shuttle-bay-and-fighters`, `weapons-and-shields`, `flight-and-navigation`: use the hand-off,
  the attachments and the dampers coupling defined here.
- Mockup: `docs/mockups/exterior.html`, shot `decoupling-split` (and the gunner, cockpit and
  fighter-drop shots).
