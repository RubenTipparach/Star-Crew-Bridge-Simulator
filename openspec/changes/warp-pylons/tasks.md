# Tasks: warp-pylons

The owner asked for it on 2026-10-11 ("or go outside to fix pylons (which needs fixing if they are damaged in
combat)"). Built after `tern-exterior` and after Combat 2's `ship-damage` lands on main.

## 1. Write-up

- [x] 1.1 Proposal, design and spec delta.

## 2. The model

- [ ] 2.1 The airlock's outer door and its aperture in `build_ship_exteriors.py`, every livery and far level rebuilt.
- [ ] 2.2 The coil segments and handhold rings as named index ranges in the manifest.
- [ ] 2.3 The portal check covers the airlock door.

## 3. Damage and warp

- [ ] 3.1 Pylon and nacelle hit volumes in `damage.json`, checked against the model's assemblies.
- [ ] 3.2 The volume test in the shot's path, the two parts and their capability; tests.
- [ ] 3.3 Warp refused or slowed by the pylons; Helm's and the Captain's mockups first, then the consoles.

## 4. Outside

- [ ] 4.1 The hull's walk triangles in the deck file; the boots mode in `sc-core::walk` with its tests.
- [ ] 4.2 The airlock cycle in the engine; the door refused in combat.
- [ ] 4.3 The tether, the handholds and re-clipping.
- [ ] 4.4 The repair point on each pylon's coil; the pylons game docked at its bolts; the segment drawn dark until mended.

## 5. Verification

- [ ] 5.1 Captures: out of the airlock, along the flank, on the pylon, docked, the coil glowing again.
- [ ] 5.2 `scripts/check.sh`; the Pi 5 measures a body outside (owner, on hardware).
- [ ] 5.3 The owner signs off (O7, survey 2).
