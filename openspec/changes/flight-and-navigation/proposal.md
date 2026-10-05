# Proposal: the Tern's flight model, helm and navigation in a star system

## Why

Helm is one of the four core stations (`docs/design/vision.md`): it flies the ship, navigates,
docks and evades, and it is the station every other one leans on. Tactical needs the bow pointed
for the tubes; flight ops needs the ship steady for a launch; engineering's power to the drive and
the dampers decides how hard helm can fly with crew standing up (`ship-frames`).

star-crew-64's helm was two numbers lerped toward the stick in a plane, with damping and a speed
cap counted in frames (`docs/analysis/star-crew-64.md`). This game needs a ship that flies in
three dimensions, keeps its momentum, turns with real inertia and still answers a player who has
never flown Newtonian: so a physical model with a flight assist on top. Pale-Blue-Dot's rules set
the frame: planets and stations on analytic rails; ships with local gravity falloff, Newtonian
motion, configurable linear and angular limits and dampers; no patched conics, manoeuvre nodes,
transfer windows or n-body ship simulation (`/home/user/Pale-Blue-Dot/CLAUDE.md`).

Missions cross a star system in 20 to 45 minutes, and a star system is tens of AU wide: at 1.5 g
crossing 1 AU takes about two days. So the ship also needs a way between points of interest, and
that choice (a jump or a cruise drive) shapes navigation, engineering and the frames.

## What Changes

- **The Tern as a rigid body**: 3,000 t, centre of mass and inertia tensor estimated from the hull
  loft and a mass table (structure spread over the 21,600 m^3 envelope, the reactor, drive and other
  heavy items as point masses).
- **Propulsion from power**: two main engines (45 MN, 15 m/s^2), reverse thrusters (5 m/s^2), RCS
  translation (4 m/s^2) and torque (12 deg/s^2 in pitch and yaw, 30 in roll), each scaling with the
  supply ratio power-grid delivers.
- **Newtonian six-degree-of-freedom flight** at the 30 Hz tick in `f64`, with local gravity falloff
  from bodies on rails, and **three assist modes**: full (velocity and rotation hold, 400 m/s cap),
  rotation only, and off.
- **The damper-safe limit**: on by default, the assist scales helm's commands so the dampers'
  predicted demand stays inside their capacity; helm can switch it off for emergency manoeuvres.
- **Helm controls** for keyboard and mouse, gamepad and HOTAS, all driving the same commands.
- **In-system travel by jump drive** (recommended over a cruise drive): 800 MJ spooled in 40 s at
  the full 20 MW allocation (sized 2026-10-04 to fit `power-grid`'s 48 MW reactor), mass lock,
  alignment, a 1.5 km field that carries craft in formation, arrival at a point of interest.
- **Navigation**: a system map and a local plot; autopilot modes (hold station, approach, orbit,
  point bow, follow course, dock, evade); docking corridors and limits.
- **Collisions** between ships, craft, missiles and bodies: convex shapes, impulse response, kinetic
  damage that bypasses shields, and the jolt through the dampers.

## Capabilities

### New Capabilities

- `flight-and-navigation`: the ship and craft flight model, flight assist and its damper-safe
  limit, helm controls, bodies on rails and local gravity, the jump drive, navigation, autopilot,
  docking and collisions.

### Modified Capabilities

None.

## Impact

- `sc-core::flight` (the one flight model for ships and craft), `sc-core::nav` (bodies on rails,
  jump drive, autopilot, docking), `sc-core::collide` (exterior collision).
- Data: `data/ships/tern/flight.json`, `data/ships/tern/jump.json`, `data/systems/<id>.json`.
- `ship-frames`: the frames these bodies live in; the dampers' demand that the damper-safe limit
  predicts; the jump as a rebase at a tick boundary.
- `power-grid`: the drive (16 MW nominal, 24 MW at overdrive) and RCS (two 4 MW blocks) draws are
  its; the jump spool's 20 MW is proposed here to fit its reactor, as a priority 3 load on the drive
  panel.
- `weapons-and-shields`: tubes need the bow; collisions feed the damage resolution as kinetic hits.
- `shuttle-bay-and-fighters`: craft fly this model; the recovery hold; auto recovery uses docking.
- `bridge-stations`: the helm console and navigation map; `netcode-and-sessions`: helm inputs.
- `reference-ship-tern`: a proposed `jump_drive` system in the drive section (design, "Layout note").
- Mockup: `docs/mockups/exterior.html` shots `chase` and `decoupling-split`.
