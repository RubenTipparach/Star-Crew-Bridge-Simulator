# Proposal: turrets, missiles and six shield faces

## Why

The owner, 2026-10-04: the ship has "manable or automated turrets and missile launchers", and
the game aims for "peak simulation systems". Weapons are where the crew's cooperation shows
most: helm points the bow so tactical's tubes bear, engineering decides whether the turrets'
capacitors or the shields get the power, science balances the shield faces toward the threat,
and a crew member in the torpedo room loads faster than the autoloader and is the only one who
can clear a jam (`docs/design/vision.md`, pillar 4).

star-crew-64 proved the shape (`docs/analysis/star-crew-64.md`): six shield faces chosen by the
dominant local axis, only overflow bleeding through to the hull, a cheap shot and a heavy shot
with heat, a forward firing arc that makes helm and weapons cooperate, and a fighter state
machine of orbit, attack run, fire and retreat. It got the units wrong (frames, abstract hit
points, no energy), had no aim reticle, and science balanced shields by a rhythm game. This
change keeps the shape and makes every number physical: energies in megajoules, rates in
seconds, ranges in metres, power drawn from the grid.

## What Changes

- **Four twin pulse cannon turrets** at the layout's mounts (dorsal, ventral, port, starboard):
  traverse 60 deg/s in yaw and 45 deg/s in elevation (-10 to +90 degrees), 4 bolts a second at
  1,500 m/s, 1.2 MJ delivered per hit, a 24 MJ capacitor charged at 4 MW, a heat sink with a
  lockout. Arcs come from the mount and a ray test against the hull loft: each turret covers
  53-58% of the sphere, and every direction is covered by at least two turrets.
- **Manned or automated.** A gunner in the pod aims with mouse, stick or pad through the turret
  sight, with a lead pip and a hit-chance readout. An automated turret follows tactical's
  priority list with a 0.5 s reaction, an aim error that grows with the target's angular rate,
  fire discipline, and a load on the computer core.
- **One lead solver and one hit-chance estimate**, used by gunners' sights, automation, point
  defence, enemy AI and tactical's console (CLAUDE.md 6.1: a turret's hit chance is previewed by
  the code that resolves it), with a calibration test.
- **Point defence.** Turrets in point-defence mode fire proximity-fused bolts at missiles inside
  1,200 m: one turret stops a single missile about half the time, two about four times in five.
- **Gannet missiles**: twelve in the magazine, a hoist to two ready racks, an autoloader (18 s) or
  crew hands-on (10 s), tube states (empty, loading, loaded, arming, armed, fired, jammed,
  damaged), proportional navigation, 60 MJ warheads, interception by point defence.
- **Six shield faces** on an ellipsoid around the hull, chosen by star-crew-64's dominant axis in
  coordinates normalized by the ellipsoid; 240 MJ in total, regenerating at 3 MJ/s from 12 MW
  (see power-grid); face balance and frequency set by science; only overflow reaches the hull,
  as a hull hit (point, direction, energy, kind) that `damage-control` resolves.
- **One damage resolution** for every ship and craft: the Tern, the raider corvette and a single
  bubble on a fighter use the same code with different data.
- **An early enemy roster**: the Jackal raider fighter (star-crew-64's state machine in seconds
  and metres), the Hound raider corvette, and a target drone for the tutorial; mission rosters
  for the first four missions; time-to-kill tables.

## Capabilities

### New Capabilities

- `weapons-and-shields`: turrets and their arcs, manned and automated fire, the lead solver and
  hit-chance estimate, point defence, missiles from magazine to impact, the six shield faces and
  the damage resolution that feeds damage-control, and the early enemy roster.

### Modified Capabilities

None.

## Impact

- `sc-core::weapons` (turrets, projectiles, missiles, lead solver, hit chance), `sc-core::shields`
  and `sc-core::damage` (the one damage resolution), `sc-core::ai` (enemy behaviour).
- Data: `data/weapons.json`, `data/ships/tern/shields.json`, `data/ships/tern/magazine.json`,
  `data/enemies.json`, `data/missions/*.json` rosters.
- `power-grid`: turret capacitor charging, shield generator, loader and hoist draws (assumed
  here, see power-grid); heat inputs to the coolant loop.
- `damage-control`: receives hull hits; supplies the health of the shield generator, the
  torpedo room, the tubes and the turrets.
- `bridge-stations`: tactical's and science's console controls; the computer core's load.
- `ship-frames`: muzzle velocities, the missile hand-off, hit shake.
- `netcode-and-sessions`: fire and hit events (about 4 kbit/s in a full engagement, to add to its
  table).
- Mockup: `docs/mockups/exterior.html` shots `broadside`, `missile-launch`, `gunner-view`,
  `shields-hit`.
