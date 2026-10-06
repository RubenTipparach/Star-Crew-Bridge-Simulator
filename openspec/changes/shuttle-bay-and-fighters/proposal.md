# Proposal: a working shuttle bay, launch bays and the craft they hold

## Why

The owner, 2026-10-04: the ship has "a fully working shuttle bay", and "some players can even
launch as a fighter if they wanted to". A player who walks from the bridge to the launch bay,
climbs into a Swift, waits while the bay is pumped down, drops through the floor and flies alongside
the ship they just left is the moment this game exists for. It is also the clearest test of the
decoupling (`ship-frames`): the fighter is part of the interior until the cradle lets go, and part
of the system from then on.

It is a cooperation system by design (`docs/design/vision.md`, pillar 4): "flight ops cannot launch
until the bay is empty of air, and the bay cannot empty while someone stands in it unsuited". Helm
must hold the ship steady for a launch and a recovery; engineering's power runs the pumps and the
cradles; life support owns the air that leaves.

## What Changes

- **The bays from the layout.** Two launch bays on deck C (`launch_bay_p`, `launch_bay_s`), each a
  Swift on a cradle over a ventral drop door, and the double-height hangar with the Petrel on its
  lift pad over the pad door, bay control on the forward landing. The doors sit at the bay floors;
  a 2.3 m trunk reaches the keel.
- **A launch sequence with times**: board 6 s, preflight 8 s (overlapping the pump-down), bay
  pump-down to 5 kPa 23.6 s (life-support's number), drop door 4 s, cradle 3.5 s, release with a
  4 m/s ejection, clear. 37.6 s from a pilot in the bay to release when cold; 8 s from the "ready 1"
  posture; 0.5 s hot. (Corrected 2026-10-04 from an assumed 30 s pump-down and 44 s, and 2026-10-05
  from 28.6 s and 42.6 s when life-support reran its pump-down on layout v2.)
- **Recovery**: an approach corridor under the keel with speed limits, a recovery hold on the helm,
  capture within 0.6 m and 1.0 m/s, the hand-off back into the interior, raise, close, repressurize.
- **Flight ops and bay control**: what they see (pressures, doors, cradles, craft, interlocks with
  reasons, the approach) and do (postures, launch, abort, vent, override, recover, task fighters).
- **Failures**: a jammed drop door, a breached bay, the ship manoeuvring hard during a launch or a
  recovery, a cradle without power, a pressure door that will not seal, a craft too damaged to
  capture.
- **The Swift fighter**: 8 t, 60 m/s^2, six-degree-of-freedom Newtonian flight with the same flight
  assist as the Tern (`flight-and-navigation`), twin guns and two Dart missiles, a shield bubble,
  propellant for about 250 s of full thrust, 1,500 triangles.
- **The Petrel shuttle**: four seats, 28 t, launched from the hangar pad only with the whole hangar
  depressurized; away teams, rescue, cargo, EVA support.
- **Automation**: with no pilot, a fighter flies as a drone at reduced competence, launched only on
  an order from flight ops (or tactical when flight ops is merged), and recalls itself at bingo fuel.
- **Rearm and refuel** on the cradle: about 40 s for a full turnaround.

## Capabilities

### New Capabilities

- `hangar-and-craft`: bay operations (postures, launch, recovery, interlocks, failures), flight ops
  and bay control, the Swift and the Petrel, drones, rearm and refuel.

### Modified Capabilities

None.

## Impact

- `sc-core::hangar` (bay sequences, interlocks, cradles, doors, postures), `sc-core::craft` (Swift,
  Petrel, drones; flight through `flight-and-navigation`'s one model).
- Data: `data/ships/tern/bays.json`, `data/craft/swift.json`, `data/craft/petrel.json`.
- `life-support`: bay pump-down, venting and repressurization times and air lost (quoted from its
  section 13).
- `power-grid`: pumps, cradles, doors, the lift pad, refuelling and charging (assumed here).
- `ship-frames`: release and capture hand-off, attachments, the cockpit view.
- `weapons-and-shields`: the Swift's guns and Darts, its shield bubble, the damage resolution.
- `flight-and-navigation`: the craft flight model and assist, the recovery hold, the docking
  autopilot used for auto recovery.
- `bridge-stations`: the flight ops console and bay control; `crew-on-deck`: boarding, suits, EVA.
- `reference-ship-tern`: a proposed layout addition for the drop trunks (design, "Layout notes").
- Mockup: `docs/mockups/exterior.html` shots `fighter-drop`, `fighter-recovery`, `fighter-cockpit`.
