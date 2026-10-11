# Design: warp pylons that break in combat and are mended outside

The owner, 2026-10-11: "or go outside to fix pylons (which needs fixing if they are damaged in combat)". Built after
`tern-exterior`, which puts the hull in the engine, and after Combat 2's `ship-damage` lands on main (PR #19).

## 1. The pylons on the model

Measured from `assets/models/exteriors/liveries/cobalt/tern.glb` (`tern-exterior` design 2): each pylon leaves the
hull's flank at about z 2-8 m, level at y 1.2 m (deck B's floor is y 0), and sweeps out to |x| 16-19 m, where its
nacelle runs from z 13 m to z -44 m, out to |x| 22.4 m. The manifest names the assemblies (`tern.warp_pylon_-1`,
`tern.warp_nacelle_-1`, the field and frame pieces, `tern.nacelle_-1.service_band_0..4`, and the same with `1`); +x is
port, so `_1` is port and `_-1` starboard.

## 2. The pylons take hits

`ship-damage` 1 finds where a shot lands: the shield, then the hull's lofted octagons. The pylons stand outside the
octagons, so today a shot through one goes on to the hull or misses.

- **Volumes.** Each pylon gets two hit volumes in `damage.json`: an oriented box round the pylon (root to tip) and a
  capsule along the nacelle, measured from the model's assemblies and checked against them (`tools/` reads the glb and
  fails when a volume no longer covers its assembly).
- **The test.** After the shield and before the hull, the path is tested against the four volumes; the first it meets
  takes the energy left, as the armour section would, at the pylon's armour (`damage.json`, a thin fairing: about a
  quarter of a hull section's). What passes through goes on along the path to the hull.
- **Parts.** `warp_pylon_port` and `warp_pylon_starboard`, integrity 0-100, capability by `ship-damage` 2's curve (1 at
  75 or more, integrity / 75 between, 0 below 25). The cascade (section 3) reaches them only through the warp feed's
  node, which is inside the hull.

## 3. Warp needs both pylons

- A jump is refused while either pylon's capability is 0; between, the spool takes its time divided by the lower
  pylon's capability.
- Helm's and the Captain's consoles show the two pylons' states in the warp control's place (a mockup first, CLAUDE.md
  10); the damage board lists them with the other parts.
- The first mission's last step (`first-mission-gate` E, warp out) therefore waits for a damaged pylon to be mended.

## 4. The airlock opens onto the hull

The layout's airlock is on deck C off cargo, its outer pressure door (`p_airlock_outer`) at x -10.8 m, z 12 m, facing
starboard, 1.0 x 2.2 m. The model has no door there.

- `tools/blender/build_ship_exteriors.py` cuts the aperture and adds a frame and a leaf at `p_airlock_outer`, as it does
  the drop doors; every livery and both far detail levels are rebuilt.
- `tools/exterior_windows.py --check` grows into the portal check: the airlock's outer door meets its aperture as a
  window meets its own (`tern-exterior` design 6).
- The cycle is `life-support` 14's: out, 39.3 s to 5 kPa and the door's 2.0 s; in, 2.0 s and a 14.1 s fill.

## 5. Walking the hull

`crew-on-deck` 10 (survey 1's C6, boots and tether, taken): a suited body outside stays in ship coordinates and walks
the hull's outer surface with magnetic boots at 1.0 m/s.

- **The surface.** The hull's own triangles, compiled into the deck file by `tern-exterior`'s `deckc`, become a second
  walk world (the outside's), beside the rooms'.
- **Boots.** The body's up is the normal of the triangle under its feet, eased over 0.2 s across an edge, so it walks
  from the flank onto the pylon's top and along it; a step off a surface it cannot reach (the pylon's tip, the bow)
  stops it at the edge.
- **The tether.** 30 m from the airlock's ring, drawn as a line. The starboard pylon's root is about 15 m of hull from
  the door; the port pylon's is about 50 m, over the roof. Handholds along both routes (a ring every 10 m, placed by
  the generator with the model) let the body clip on again, and the tether then runs from the last ring.
- **The demand.** Outside, the dampers do not reach (`crew-on-deck` 10): boots hold to 6 m/s^2 of the ship's demand,
  beyond which the body hangs on its tether until the demand falls.

## 6. The repair

- The job (Combat 2's `warp_pylon_port`, `warp_pylon_starboard`, place `outside`) has its point at the damaged coil
  segment, on the pylon's top; the body docks there as at any repair point (`repairs-on-deck` 3).
- The pylons game starts at its bolts: its first move, clipping from handhold to handhold, was the walk
  (`repairs-on-deck` 8 put the walk inside the game; with the hull walkable it is walked instead).
- Done, the segment glows again and the pylon's integrity returns as the game's result says.

## 7. Not in combat

`repair-minigames` 5: the outer door will not open while any hostile is within weapons range, and the pylons game
shows it is refused. A body already outside when a hostile comes in range is told to come back; nothing drags it in.

## 8. Seeing the damage

The model draws each pylon's coil as one piece today. The generator writes the coil's segments (five service bands a
nacelle) as their own index ranges in the manifest, so the client draws a damaged segment dark and sparking, and a
destroyed pylon's whole coil dark.

## 9. The Pi 5 budget

| Cost | Amount |
| --- | --- |
| Collision | The hull's 4,440 triangles in the outside's walk world, about 0.2 MB |
| Draws | One tether line; the suited figure as a crew figure (3,000 triangles at most) |
| Server | Four volume tests a shot; the boots step once a tick for a body outside |

## Open questions

| Id | Question | Recommendation | Status |
| --- | --- | --- | --- |
| O4 | Warp with one pylon damaged: refused below 25 integrity, slower spool between 25 and 75 | As designed | Recommendation taken (ask only with screenshots) |
| O5 | The port pylon reached over the roof with handholds every 10 m and a 30 m tether | As designed | Recommendation taken (ask only with screenshots) |
| O6 | The walk replaces the pylons game's handhold step | Yes | Recommendation taken (ask only with screenshots) |
| O7 | The walk outside and the repair: approve the captures | Approve | With the captures once built |
