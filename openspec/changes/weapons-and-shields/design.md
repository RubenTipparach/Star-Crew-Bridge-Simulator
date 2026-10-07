# Design: weapons and shields

Status: **proposed** (2026-10-04). Nothing here is built. Power draws are `power-grid`'s
(`data/ships/tern/power.json`): it owns every draw and the heat loop, and this design's formulas
take the supply ratio it delivers (0 to 1, or up to 1.5 at its overdrive setpoints) so a change
there rescales nothing here. Corrected 2026-10-04 to power-grid's figures: the hoist and loaders
draw 0.03 MW together, not 1.5 MW and 2 MW (section 10). Budget numbers are against
`openspec/changes/engine-stack/design.md` section 5 (the Pi 5 table, the one source).

## Context

The Tern (`data/ships/tern/layout.json`) carries four turret mounts (`turret_dorsal`
[0.0, 9.0, 2.0] facing +Y, `turret_ventral` [0.0, -7.0, 3.0] facing -Y, `turret_port`
[13.5, 1.25, 4.0] facing +X, `turret_stbd` [-13.5, 1.25, 4.0] facing -X), each with a gunner's
pod, and two bow missile tubes (`tube_1` [2.0, 1.5, 37.0] and `tube_2` [-2.0, 1.5, 37.0], facing
+Z) fed from the magazine on deck C by the hoist (`p_hoist`) into the torpedo room on deck B. The
shield generator is in `shield_room` on deck C; the turret capacitor banks are in the turret
access rooms.

From star-crew-64 (`docs/analysis/star-crew-64.md`): six faces, bow +Z, stern -Z, port +X,
starboard -X, dorsal +Y, ventral -Y; the face is the dominant axis of the hit's local offset; only
the face hit absorbs, and overflow hits the hull. Its shields were 40 per face with 1/s
regeneration; its phaser added 8 heat of 100 and its torpedo 25; its fighter orbited, ran in,
fired one unled shot and retreated for 10 s. Those numbers were frames and abstract points; the
ones below are seconds, metres and megajoules, and the 40 MJ face is a deliberate nod.

## Goals / Non-Goals

**Goals:**
- Every weapon number in SI with its formula, in data.
- A gunner beats automation, and automation is still useful.
- One lead solver, one hit-chance estimate, one damage resolution, one fire rule per weapon.
- Every station's choice matters to another's: power to capacitors or shields, the bow for the
  tubes, the face balance toward the threat, hands in the torpedo room.

**Non-Goals:**
- What a hull hit does to compartments, systems and crew (`damage-control`).
- The power and heat network itself (`power-grid`); the console layouts (`bridge-stations`).
- Boarding actions, mines, beam weapons, cloaking: later changes if ever.

## Decisions

### 1. Energy is the unit of damage

A weapon delivers energy in megajoules. A shield face holds energy, a hull hit carries energy,
and `damage-control` turns hull energy into structural and system damage. Capacitors, warheads,
shields and heat all count in MJ and MW, so engineering's console can show one honest flow from
the reactor to a bolt to a face.

### 2. The twin pulse cannon

| Quantity | Value | Notes |
| --- | ---: | --- |
| Barrels | 2 | Fire alternately |
| Rate of fire | 4 bolts/s per turret | 0.25 s between bolts, each barrel at 2 bolts/s |
| Bolt speed | 1,500 m/s | Relative to the muzzle; the bolt also carries the muzzle point's velocity (`ship-frames` section 10) |
| Bolt life | 1.6 s | Maximum range 2,400 m |
| Effective range | 1,500 m | Where automation's fire discipline usually allows fire against fighters |
| Hit test | Swept sphere, radius 0.3 m | Against the target's hit volume (its shield ellipsoid, or its hull when shields are down) |
| Energy drawn from the capacitor | 1.5 MJ per bolt | |
| Energy delivered on a hit | 1.2 MJ | 80% |
| Waste heat into the turret's sink | 0.3 MJ per bolt | |
| Capacitor | 24 MJ | One bank per turret, in the turret access room |
| Charging | 4 MW nominal, 6 MW at a 150% setpoint | `power-grid`'s turret load; times the supply ratio |
| Heat sink | 9 MJ | Dissipates 0.6 MW into the coolant loop while coolant flows (`power-grid`) |
| Heat lockout | At 100% (9 MJ); resumes at 60% | |
| Yaw | Unlimited (slip ring), 60 deg/s, 240 deg/s^2 | |
| Elevation | -10 to +90 deg from the mount plane, 45 deg/s, 180 deg/s^2 | |
| Point-defence fuse | Bursts within 1.0 m of a missile | Only in point-defence mode |

**What sustained fire costs.** Firing draws 6 MW (4 bolts x 1.5 MJ) against a 4 MW charge, so a
full capacitor gives 12 s of full-rate fire, after which the rate falls to what the charge
sustains (2.67 bolts/s). Heat rises at 1.2 MW against 0.6 MW of dissipation, so the sink locks the
turret after 15 s of full-rate fire from cold, and never at the charge-limited rate (0.8 MW in).
Engineering can raise the charge (at a 150% setpoint, 6 MW, it matches sustained fire, with
`power-grid`'s overdrive heat and wear), and lose it to a breaker trip (`power-grid`); a damaged
coolant loop removes the dissipation and makes heat the limit.

### 3. Mount frames and arcs

A turret's frame (`ship-frames`, `mount:` frame): **up** `u` is the mount's `facing`; the yaw
reference `f0` is the ship's +Z (bow) in the mount plane; `s0 = u x f0`. The barrel direction for
yaw `psi` and elevation `theta` is

```text
d = cos(theta) (cos(psi) f0 + sin(psi) s0) + sin(theta) u
```

**Arcs come from the hull.** A turret may fire along `d` only if (a) `theta` is within -10 to +90
degrees and (b) a ray from the turret's pivot along `d` clears the hull loft and the other mounts
for 200 m. An offline tool (`sc-tools`, from the layout) bakes (b) into a mask of 5 degree bins
(72 in yaw x 20 in elevation = 1,440 bits per turret) that the fire interlock reads at the
barrel's current direction, and that tactical's console draws as the turret's arc. Measured from
the layout's hull loft (a scratch ray test over 2,000 evenly spread directions, to become that
tool):

| Turret | Pivot (m) | Facing | Share of the sphere it can fire into |
| --- | --- | --- | ---: |
| Dorsal | [0.0, 9.0, 2.0] | +Y | 53.3% |
| Ventral | [0.0, -7.0, 3.0] | -Y | 54.8% |
| Port | [13.5, 1.25, 4.0] | +X | 58.2% |
| Starboard | [-13.5, 1.25, 4.0] | -X | 57.9% |

| Coverage | Share of all directions |
| --- | ---: |
| By at least one turret | 100% |
| By at least two turrets | 100% |
| By three or more | 23.8% |

| 30 degree cone around | Bow | Stern | Port | Starboard | Dorsal | Ventral |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Mean turrets that bear | 2.55 | 2.34 | 2.27 | 2.26 | 2.44 | 2.43 |
| Fewest turrets that bear | 2 | 2 | 2 | 2 | 2 | 2 |

The exterior mockup bakes the same masks at load with the barrel's pivot at the trunnion, 1.15 m
out from the mount centre along its facing (pod wall 0.25 m, ring 0.35 m, trunnion 0.55 m), and
measures 56.7%, 57.6%, 58.7% and 58.5% (dorsal, ventral, port, starboard), 100% covered by two or
more and 30.7% by three or more. Its dorsal and ventral shares are about 3 points above the
table's, most likely because the table's scratch test cast from `center_m`; either way the
conclusion holds, and the arc baker takes the pivot from the mount's data, never from `center_m`
alone.

So there is no blind spot, and helm can bring a third turret to bear by rolling. The dorsal and
ventral turrets have a keyhole straight up their facing (yaw rate needed to track a target
through the zenith is unbounded); the side turrets cover those directions.

### 4. The fire rule and the projectile

One fire rule for every gun (turret, fighter gun, enemy gun), parameterized by its data:

1. A gun may fire when its cooldown has elapsed, its capacitor holds the bolt's energy, its heat
   is below lockout, its supply is on, its arc mask allows the barrel direction, and (manned) the
   trigger is held or (automated) the fire discipline of section 7 passes.
2. The bolt is born in the system frame at the muzzle, with velocity `V_muzzle + 1,500 m/s x d`,
   where `V_muzzle` is the ship's point velocity at the muzzle (`ship-frames`).
3. The capacitor loses 1.5 MJ, the sink gains 0.3 MJ, the barrel alternates.
4. Each tick the bolt moves and is tested as a 0.3 m swept sphere against hit volumes in a uniform
   grid of 100 m cells. A hit goes to the damage resolution (section 11) with 1.2 MJ; a bolt that
   outlives 1.6 s is removed.

Projectiles are not bodies: they live in a fixed pool of 1,024 (engine-stack), cost no exterior
body slot, and are never sent as state. A reliable "fired" event (shooter, mount, tick,
quantized direction) spawns the same bolt on every client (`netcode-and-sessions`).

### 5. The lead solver (one implementation)

For a gun at muzzle position `m` with muzzle velocity `V_m`, bolt speed `s`, and a target at `p`
with velocity `v` (both system frame, from the sensor track):

```text
r = p - m,  u = v - V_m
solve (u.u - s^2) t^2 + 2 (r.u) t + r.r = 0 for the smallest t > 0
aim point  a = p + v t   (in the frame moving with the muzzle: r + u t)
aim dir    d = (r + u t) / |r + u t|
no positive root: no solution (the target outruns the bolt); the pip is not shown
```

Used by: the gunner's lead pip, automated turrets, point defence, fighters' gunsights (the Swift's
and enemies'), and tactical's console. It ignores the target's acceleration on purpose; the
target's manoeuvre during the bolt's flight is what the hit-chance estimate models as spread.

### 6. Manned turrets

- The gunner climbs into the pod (`crew-on-deck`) and sits at the gunner seat; the view becomes
  the turret sight (`ship-frames` section 7, question F2 there).
- **Aim**: the mouse, a stick or a pad moves an aim point on the sight; the turret slews toward it
  at its traverse limits, so the gunner can never aim faster than the turret can turn. Two zoom
  levels: 60 and 20 degrees of vertical field of view.
- **The sight shows**: the reticle (where the barrels point now), the lead pip of the selected
  target (section 5), the hit chance (section 8) using the gunner's own aim error (the RMS angle
  between the barrel and the lead solution over the last second), range and closure, the
  capacitor and heat bars, the arc edge when the hull blocks, and off-screen arrows to threats
  (star-crew-64's arrows).
- **Fire** while the trigger is held, under the fire rule. The gunner chooses any target in arc;
  tactical's designated target is marked.
- A gunner's turret costs no computer core load (section 7).

### 7. Automated turrets

An automated turret is the station without a player (CLAUDE.md 7: automation at a stated, lower
competence).

| Behaviour | Value |
| --- | --- |
| Target choice | Tactical's priority list (below), within the arc mask and 2,400 m |
| Acquisition (reaction) | 0.5 s from a target entering the arc or becoming top priority |
| Aim error | Gaussian, `sigma_aim = 0.15 deg + 0.02 x (target angular rate in deg/s)` |
| Fire discipline | Fires when the estimated hit chance is at least 4% (keeps the capacitor for shots that can land) |
| Computer core load | 6% of the core per automated turret, 9% in point-defence mode (assumed; the core's model is bridge-stations') |
| Overloaded core | When the core's demand `D` exceeds its capacity `K`, reaction and `sigma_aim` are multiplied by `D / K` |

**Tactical's priority list** (one list for all turrets, or one per turret):

1. Missiles inbound within 1,200 m (point defence).
2. Fighters attacking this turret's side (in attack run or fire toward the Tern).
3. Tactical's designated target.
4. The nearest hostile in arc.

**Turret modes** (tactical sets each): automatic (the list), point defence only, fighters only,
designated target only, hold fire. A manned turret ignores its mode.

### 8. The hit-chance estimate (one implementation)

```text
P_hit = 1 - exp( -R^2 / (2 sigma^2) )
sigma^2 = (sigma_aim x d)^2 + (0.5 x a_m x t_f^2)^2 + sigma_track^2
  R           the target's hit radius (shield bubble or ellipsoid seen from the gun), m
  d           range, m;  t_f  bolt flight time from the lead solver, s
  sigma_aim   the gun's aim error, rad: automation's formula (section 7), or a gunner's measured RMS
  a_m         the target's manoeuvre, RMS acceleration over the last 2 s of its track, m/s^2
  sigma_track sensor track error, 0.5 m + 0.002 x d (science's sensors; worse when damaged)
```

| Case (automated turret) | `sigma_aim` | `sigma` | Flight time | Hit chance per bolt | Hits a second |
| --- | ---: | ---: | ---: | ---: | ---: |
| Jackal crossing at 200 m/s, 800 m, flying straight | 0.44 deg | 6.5 m | 0.53 s | 13.6% | 0.54 |
| Jackal crossing at 200 m/s, 800 m, jinking at 30 m/s^2 | 0.44 deg | 7.7 m | 0.53 s | 9.7% | 0.39 |
| Jackal crossing at 200 m/s, 1,500 m, jinking | 0.30 deg | 17.3 m | 1.00 s | 2.0% (holds fire) | 0 |
| Jackal on its attack run, head on, 400 m, jinking | 0.19 deg | 2.1 m | 0.27 s | 73.7% | 2.95 |
| Hound corvette at 1,800 m, 80 m/s | 0.20 deg | 7.8 m | 1.20 s | 55.8% | 2.23 |
| Hound corvette at 1,200 m, 80 m/s | 0.23 deg | 5.6 m | 0.80 s | 79.2% | 3.17 |

So a fighter that orbits at range is hard to hit, and one that runs in to fire is in danger: the
attack run is the fighter's risk, as in star-crew-64. **The estimate is calibrated, not exact**: a
test flies seeded engagements and requires the observed hit rate in each 10% band of predicted
chance to fall within that band plus or minus 5 points. The same function drives the gunner's
readout, the automation's fire discipline and tactical's console, so the preview and the
outcome come from one model (CLAUDE.md 6.1).

### 9. Point defence

A turret in point-defence mode (or automatic, with a missile at the top of its list) fires bolts
fused to burst within 1.0 m of a missile, with `sigma_aim` 0.12 deg (point-defence fire control).
Missiles are fragile: any bolt that reaches one destroys it. Against a missile closing at 1,000
m/s from 1,200 m (about five bolts per turret before impact):

| Turrets on one missile | Chance the missile is stopped |
| ---: | ---: |
| 1 | 56% |
| 2 | 81% |

A salvo of two Gannets against a ship with one point-defence turret gets at least one missile
through about 80% of the time; that is why missiles are fired in pairs. Point defence also shoots
at fighters inside 400 m when no missile is inbound.

### 10. The Gannet missile

| Quantity | Value | Notes |
| --- | ---: | --- |
| Mass | 1,100 kg | Warhead 200 kg |
| Length, diameter | 4.2 m, 0.5 m | Fits the tube and the hoist (1.2 x 3.0 m opening) |
| Ejection | 30 m/s from the tube | Plus the tube's point velocity (`ship-frames` hand-off) |
| Motor ignition | 0.5 s after ejection | 15 m clear of the bow |
| Boost | 120 m/s^2 for 6 s | |
| Sustain | 40 m/s^2 for 20 s | |
| Speed gained at burnout | 1,520 m/s | Relative to launch |
| Self-destruct | 60 s after launch | |
| Guidance | Proportional navigation, `a = N V_c dLOS/dt`, N = 4, lateral limit 150 m/s^2 | |
| Seeker | 30 degrees either side of its nose, 10 km acquisition | Lock before launch (tactical's track) or lock on after launch |
| Off-boresight launch | Up to 90 degrees from the bow | Beyond 60 degrees it loses about 2 s turning; helm points the bow |
| Fuse | Contact, or proximity within 5 m of the target's hit volume | |
| Warhead | 60 MJ at contact, falling linearly to 0 at 30 m | 50 MJ at the proximity limit |
| Structure | 0.4 MJ | One pulse bolt destroys it |
| Hit volume | Sphere 0.3 m | Point-defence bolts burst within 1.0 m |
| Triangles | 200 | engine-stack's missile ceiling; exhaust is a billboard |

**Time of flight** (eject, coast 0.5 s, boost, sustain; from a standing start):

| Range | 1 km | 2 km | 5 km | 10 km | 20 km |
| --- | ---: | ---: | ---: | ---: | ---: |
| Time | 4.3 s | 6.0 s | 9.7 s | 14.8 s | 22.9 s |
| Speed at that point | 490 m/s | 690 m/s | 880 m/s | 1,080 m/s | 1,410 m/s |

**From the magazine to the tube.**

| Step | Who | Time | Notes |
| --- | --- | ---: | --- |
| Magazine to a ready rack | The hoist (automatic) | 20 s per missile | One hoist, one missile at a time; the hoist and loaders draw 0.03 MW together (`power-grid`'s `missile_hoist` load; corrected 2026-10-04 from an assumed 1.5 MW) |
| Ready rack to the tube, autoloader | Automatic | 18 s | Breech open 3 s, ram 8 s, seal 4 s, umbilical 3 s; on the same 0.03 MW load (corrected 2026-10-04 from an assumed 2 MW) |
| Ready rack to the tube, crew hands-on | A crew member at the breech | 10 s | Three actions at the breech panel; crew halve the jam chance |
| Ready rack to the tube, no power | A crew member with the hand crank | 40 s | |
| Arm | Tactical | 3 s | Warhead arm, seeker spin-up, track hand-off |
| Safe (armed back to loaded) | Tactical | 2 s | |
| Fire | Tactical | Door open 1 s; the tube is empty 1.5 s after launch | |
| Clear a jam | A crew member at the breech | 20 s | Automation cannot clear a jam |

Sustained rate: two tubes share one hoist, so after the first pair (two missiles staged on the
ready racks at the start of a mission) the hoist is the limit: one missile every 20 s.

**Reloading is a crew and time decision, not a power one** (`power-grid` question P5): at 0.03 MW
the hoist and loaders are under 0.1% of the reactor's 48 MW, so engineering never trades them
against shields or turrets. Their supply ratio still matters: reload time grows as 1 / supply, a
load that browns out below 0.8 raises the jam chance (below), and with no power at all the crew
load by hand crank.

**Tube states:**

| State | Entered when | Leaves to |
| --- | --- | --- |
| Empty | A launch completes, or a missile is unloaded | Loading, when a ready rack holds a missile and a load starts |
| Loading | A load starts | Loaded when it completes; Jammed on a jam roll at completion |
| Loaded | Loading completes, or an armed missile is safed | Arming (tactical arms); Empty (unload back to the rack, 18 s) |
| Arming | Tactical arms | Armed after 3 s |
| Armed | Arming completes | Fired (tactical fires); Loaded (safed, 2 s) |
| Fired | Tactical fires | Empty 1.5 s later; the missile is handed to the system frame at the tick of launch |
| Jammed | A jam roll at the end of a load | Loaded, after a crew member clears it (20 s) |
| Damaged | Damage-control marks the tube damaged | Empty, after repair; a missile inside stays until then |

**Jam chance per load** (seeded by session, tube and load count):
`p_jam = 0.01 + 0.25 x (1 - h_room) + 0.3 x b`, where `h_room` is the torpedo room's systems health
from `damage-control` and `b` is 1 if the loader's supply ratio fell below 0.8 during the load
(`power-grid`'s brownout), else 0; a crew-assisted load halves it.

**Launch.** The missile is an interior object until the tick it leaves the tube; then it is handed
to the system frame with the tube's point velocity plus 30 m/s along the tube (`ship-frames`
section 10).

**Magazine**: 12 Gannets at the start of a campaign; resupply between missions (vision.md).
Two are on the ready racks and none in the tubes at mission start, unless the debrief state says
otherwise. A hit that bleeds into the magazine is damage-control's (cook-off is its rule).

### 11. Shields: six faces

| Quantity | Value | Notes |
| --- | ---: | --- |
| Shape | Ellipsoid, semi-axes 17.0 x 13.0 x 54.0 m, centred at [0.0, 1.5, -2.0] | The smallest axis-aligned ellipsoid that holds the hull loft, the mounts and the pods with 1 m to spare (computed from the layout) |
| Faces | Bow +Z, stern -Z, port +X, starboard -X, dorsal +Y, ventral -Y | star-crew-64's order |
| Total capacity | 240 MJ x generator health | 40 MJ per face when balanced |
| Generator draw | 12 MW nominal, 18 MW at a 150% setpoint | `power-grid`'s shield generator load |
| Regeneration | `0.25 x P_delivered` = 3 MJ/s at nominal | Shared across faces (below) |
| Holding | Below 15% of nominal supply the faces cannot hold and each decays at 2 MJ/s | |
| Shunt rate | 10 MJ/s between faces, 20% lost | When balance changes |

**The face of a hit** (star-crew-64's dominant axis, normalized so the long hull does not make
everything the bow or stern):

```text
o = R^T (P_hit - P_s) - c_shield           ship axes, metres
n = (o.x / 17.0, o.y / 13.0, o.z / 54.0)
face = the axis with the largest |n|, with its sign
```

Without the normalization, a hit on the port side 20 m forward of midships would count as the bow
(|z| 20 > |x| 12); with it, it is port (0.71 against 0.37), which is what the crew see.

**The shield view (owner, 2026-10-07).** The owner: "shields should display a full 3d model of the
ship and shield facings, and where enemies attack are from what angles". Science's SHIELDS panel, and
the middle of tactical's plot, draw the shield in 3D:

- **The ship** is the hull loft from the layout (the octagonal sections of `hull.sections`), as
  low-poly lines: 10 sections of 8 corners, about 140 triangles, the same shape the deck plan draws.
- **The bubble** is this section's ellipsoid, cut into its six faces by the rule above: each patch of
  the bubble takes the face `shield_face(o)` gives its centre, the function that resolves a hit (one
  implementation, CLAUDE.md 6.1). A face is filled by its charge (green, amber, red at 50 % and
  20 %), thicker where its weight is higher, and flashes when hit.
- **Where the attacks come from**: each hostile in sensor range draws a dashed line into the bubble
  from its direction; each hit in the last 8 s draws an arrow from where it came, fading, onto the face
  it struck, labelled with its bearing and elevation in ship axes (bearing to starboard from the bow,
  elevation up; `040 +12`). An inbound missile draws a red chevron on its line.
- **The view** turns by drag (yaw about the ship's +Y, then tilt), starts from aft, above and to port,
  and has one button back to that view. Tactical's copy is fixed at the plot's own tilt.

On the Pi it is one small 3D viewport in the console pass: the hull lines and the ellipsoid of the
flash (about 1,200 triangles, face per vertex, six opacities as uniforms), two draw calls, no
render target. The mockup draws it in SVG with the same projection.

**Balance (science).** Science sets a weight per face, 0.5 to 2.0 (default 1). A face's capacity
is `240 MJ x h_gen x w_f / sum(w)`, clamped to 15-80 MJ. Regeneration goes to faces in proportion
to `w_f x (capacity_f - charge_f)`, so the weakest favoured face fills first. When a face's
capacity drops below its charge, the excess shunts to faces below capacity at 10 MJ/s, losing
20%. Presets: balanced, bow, stern, broadside port, broadside starboard, and "toward target" (the
face toward the designated target at 2.0, its opposite at 0.5).

**Frequency (science).** Four bands, A to D. An attacker's weapons have a band, which a science
scan reveals. A face whose band matches the incoming weapon loses 25% less (`E x 0.75`).
Retuning takes 4 s, during which the shields absorb `E x 1.25`.

**Automation.** With no science player (merged or automated), balance follows "toward target" when
tactical designates one, else balanced, and frequency stays on its last setting (automation does
not scan for bands).

### 12. The damage resolution (one implementation)

For a weapon impact on any ship or craft (the Tern, the Hound, a Jackal's single bubble, a Swift):

1. **Find the face.** The impact point on the hit volume gives the face (section 11). A craft with
   one bubble has one face.
2. **Absorb.** `E_face = E x band factor`. If the face holds at least `E_face`, it loses `E_face`
   and nothing reaches the hull. Otherwise the face goes to 0 and
   `E_hull = (E_face - charge) / band factor` continues.
3. **Find the hull point.** The projectile's path is continued from the shield to the hull's
   collision mesh (a blast takes the hull point nearest its centre).
4. **Report the hull hit** to `damage-control`:
   `HullHit { ship, tick, point_m (ship frame f32), normal, direction, energy_mj, kind: pulse | blast | kinetic, source }`.
5. **Shake** (`ship-frames` section 5): a quarter of the absorbed energy plus all of `E_hull`.
6. **Show it.** The face flashes for 0.4 s with opacity `0.5 x min(1, E / 5 MJ)`, coloured by its
   charge (blue full, amber below half, red below a fifth); a hull hit adds sparks.

Collisions are kinetic and bypass the shields (`flight-and-navigation` sends them straight to step
3). A missile's blast that overlaps two faces is resolved on the face nearest its centre.

### 13. Enemies for the early missions

**Jackal raider fighter.** 7 m, 9 t, 540 kN (60 m/s^2), turns 90 deg/s, cruise 180 m/s, top 250
m/s under assist. Twin light guns: 2 bolts/s each (4 together), 1.0 MJ delivered, 1,200 m/s, range
1,200 m, band B. Shield: one bubble, radius 3.5 m, 4 MJ, regenerating 0.4 MJ/s after 4 s without
a hit. Hull 3 MJ. 1,500 triangles (400 at the far LOD). Six pulse cannon hits (7.2 MJ) kill it from full.

Its behaviour is star-crew-64's state machine, in seconds and metres, with the flaw it lists (an
unclamped orbit spring) fixed:

| State | Does | Leaves when |
| --- | --- | --- |
| Orbit | Circles the Tern at 800 m and 180 m/s, clockwise or anticlockwise (seeded, half each way); radial correction `clamp(0.05 s^-2 x (r - 800 m), +/-20 m/s^2)` | After 3-5 s (seeded) |
| Attack run | Turns to the Tern and accelerates to 250 m/s, jinking at 30 m/s^2 every 0.6-1.4 s once a turret tracks it | Within 400 m, or after 6 s |
| Fire | Holds its line for 2.0 s and fires its guns led by the lead solver with `sigma_aim` 0.6 deg (rookie) or 0.3 deg (veteran) | After 2.0 s |
| Retreat | Breaks off to 1,500 m with a random jink | After 8 s, back to orbit |

Timers start at a seeded phase so a group never moves in lockstep. A Jackal at 25% hull or less
retreats for good. Against the Tern (hit volume about 14 m across) its rookie fire lands about
99% of bolts at 400 m and 73% at 800 m: about 8 MJ per firing pass.

**Hound raider corvette.** 45 m, 1,200 t, 6,000 triangles (1,500 and 300 at the far LODs). Six
shield faces of 50 MJ (300 MJ), regenerating 2 MJ/s, band C; hull structure 150 MJ (damage on NPC
ships is a hull pool; they have no interior, `ship-frames` section 12). Two light turrets (3
bolts/s, 1.0 MJ, 1,400 m/s, range 2,000 m), one Lance missile launcher (40 MJ warhead, 900 m/s,
reload 30 s, 6 in its magazine, otherwise as a Gannet), one point-defence turret. Behaviour
(star-crew-64's capital ship): holds an 1,800 m orbit around the Tern at 80 m/s, turns its
strongest face toward the Tern, fires turret bursts of four every 3 s per turret, launches a Lance
every 30 s, retreats at 25% hull.

**Target drone** (tutorial): stationary or on a slow track, no shield, hull 2 MJ, 200 triangles.

**Time to kill** (from the hit-chance table; sustained, before regeneration):

| Attacker | Target | Damage rate | Time |
| --- | --- | ---: | ---: |
| One automated turret | Jackal crossing at 800 m, straight | 0.65 MJ/s | about 11 s |
| Two automated turrets | Jackal on its attack run, 400 m | 7.1 MJ/s | about 1 s |
| Three automated turrets | Hound at 1,800 m, all on one face | about 8 MJ/s | about 26 s (face then hull); about 70 s if it turns fresh faces |
| Two Gannets (one through its point defence) | Hound | 60 MJ | a face, or 10 MJ into the hull |
| Four Jackals | The Tern | about 1.8 MJ/s, spread over the faces | Shields hold at nominal regeneration (3 MJ/s) |
| Hound (two turrets at 1,800 m and Lances) | The Tern, one face | about 3 MJ/s plus 40 MJ every 30 s | One face down in about 25 s unless science rebalances or helm turns |

**Mission rosters** (the mission format is a later change; these are the first four):

| Mission | Enemies | Teaches |
| --- | --- | --- |
| 1, Patrol | 2 Jackals (rookie) | Turrets, automation, the hit chance, shields |
| 2, Convoy | 4 Jackals in two waves (rookie, then veteran) | Face balance, point defence, launching a fighter |
| 3, Strike | 1 Hound, 2 Jackals | Missiles from magazine to impact, helm pointing for the tubes |
| 4, Holdout | 6 Jackals, then a Hound | Everything at once; power trade-offs |

The exterior body limit (128 near the ship) and the projectile pool (1,024) hold these with room
to spare: mission 4 peaks at about 9 ships and craft, 8 missiles and 300 projectiles.

### 14. What tactical and science do

`bridge-stations` owns the consoles; this change needs these controls on them:

- **Tactical**: designate a target; set the priority list and each turret's mode; see each turret's
  arc, target, capacitor, heat and hit chance, and who is manning it; load, arm, safe and fire each
  tube; choose lock before launch or after; see the magazine, ready racks and hoist; request crew to
  the torpedo room.
- **Science**: shield face weights and presets, frequency band, scans that reveal an enemy's band;
  the face charges and the hit flashes on a ship diagram.

### 15. The Pi 5 budget this change spends

**Rendering** (engine-stack ceilings): projectiles instanced, two triangles each, up to 1,024 in one
call (2,048 triangles); missile exhaust and explosions as more instances of the same billboard
buffer; missiles 200 triangles each, instanced per type (one call); the shield flash one ellipsoid
of about 1,200 triangles with a per-vertex face index and six face opacities as uniforms (one call,
drawn only while a face is flashing; the exterior mockup instead picks the face per fragment from
the unit-sphere coordinate, which is section 11's normalized offset, and gets exact face edges for
the same triangles on GLES 3.0); turret heads and barrels are the Tern's attachments
(`ship-frames` section 9). Enemy models: Jackal 1,500 (instanced, one call for all), Hound 6,000.
A full engagement adds about 30,000 triangles and 6 calls to the exterior near layer.

**Server CPU** (2 ms per ship per tick on one Cortex-A76): 1,024 projectiles against a 100 m grid,
about 3,000 swept tests a tick, under 0.1 ms; four turrets scoring up to 128 bodies and evaluating
the hit chance, under 0.05 ms; 16 missiles' guidance, negligible. Under 0.2 ms in all.

**Network**: a fired event is about 12 bytes; a full engagement fires about 35 bolts a second
(the Tern's four turrets at 4/s, a third of twelve Jackals firing, the Hound): about 3.4 kbit/s.
Hit events (about 12 bytes, about 10 a second): 1 kbit/s. Shield faces (6 x 8 bits at 5 Hz) and
tube states (on change): negligible. About **4.5 kbit/s**, which `netcode-and-sessions` should add
to its table (it lists projectiles at 0 bytes because they are events).

**Memory**: the projectile pool (1,024 x 48 bytes = 48 kB), missiles (16 slots), arc masks (4 x 180
bytes), all fixed pools.

### 16. Data

| File | Holds |
| --- | --- |
| `data/weapons.json` | Twin pulse cannon, Swift guns, Jackal guns, Hound turrets; Gannet, Lance; units in keys (`bolt_speed_m_s`, `energy_drawn_mj`, `capacitor_mj`, `charge_mw`) |
| `data/ships/tern/shields.json` | Ellipsoid, total capacity, draw, efficiency, holding threshold, shunt rate and loss, band retune time |
| `data/ships/tern/magazine.json` | Capacity, hoist and loader times, jam chance terms |
| `data/enemies.json` | Jackal, Hound, target drone: masses, thrust, shields, hull, weapons, behaviour timers |
| `data/missions/*.json` | Rosters (format: a later change) |

## Risks / Trade-offs

- **Automation could make gunners pointless**, or gunners make automation pointless. Mitigation:
  automation's `sigma_aim` and reaction are data, the calibration test pins the estimate, and the
  owner judges from the gunner-view shot (question W1).
- **Point defence could make missiles useless.** Mitigation: the 56% single-turret figure is
  tuned so pairs matter; the proximity radius is data.
- **The capacitor and heat model may be too much for a casual gunner.** Mitigation: the sight shows
  both as bars, and running dry only slows the rate.
- **Fired events grow with the battle.** Mitigation: the 1,024 pool caps them; netcode measures.

## Mockup shots

| Shot | Shows |
| --- | --- |
| `broadside` | The Tern's turrets engaging circling Jackals, pulse bolts as billboards, a point-defence intercept |
| `missile-launch` | A Gannet leaving tube 1, ignited, with the bow and the tube doors |
| `gunner-view` | The dorsal turret sight: reticle, lead pip, hit chance, capacitor and heat |
| `shields-hit` | A face of the shield ellipsoid flashing on a hit, with the six face charges |
| `chase` | The whole ship in combat from behind |

## Open questions

Ids W (weapons). Questions with a shot go to the owner's survey; the rest are recommendations
taken (ask only with screenshots, CLAUDE.md 13).

| Id | Question and the fact behind it | Options | Recommendation | Shot |
| --- | --- | --- | --- | --- |
| W1 | How much better is a gunner than automation? Automation hits a straight-flying Jackal at 800 m 14% of the time per bolt. | (a) Automation as specified (0.15 deg plus 0.02 x angular rate, 0.5 s reaction). (b) Weaker automation (double the error). (c) Stronger. | (a), tuned after playtests with the calibration test holding. | `gunner-view` |
| W2 | Do gunners see the lead pip? star-crew-64 had no reticle at all. | (a) Lead pip and hit chance on the sight. (b) Reticle only. | (a): aiming at a pip is the skill; the hit chance teaches the fire discipline. | `gunner-view` |
| W3 | How do shield faces read from outside? | (a) A translucent ellipsoid whose face segment flashes on a hit. (b) A hull-hugging glow. | (a): the six faces are visible as a shape, and it is one cheap mesh. | `shields-hit` |
| W4 | Shield frequency bands: keep them? They give science a second job beside face balance. | (a) Four bands, a 25% match bonus. (b) No bands. | (a). | none: recommendation taken (ask only with screenshots) |
| W5 | Can crew load faster than the autoloader? | (a) Crew 10 s against automatic 18 s, and only crew clear jams. (b) Crew only when the loader is down. | (a): a reason to stand in the torpedo room, which is the point of a walkable ship. | none: recommendation taken (ask only with screenshots) |
| W6 | Do bolts inherit the ship's velocity? | (a) Yes, Newtonian (the muzzle point's velocity). (b) No, arcade. | (a): consistent with the flight model and the lead solver. | none: recommendation taken (ask only with screenshots) |
