# Design: ship frames

Status: **proposed** (2026-10-04). Nothing here is built. Numbers marked "see power-grid" are
assumptions this change makes about another change's values; that change is their source.
Budget numbers come from `openspec/changes/engine-stack/design.md`, section 5, the Pi budget
table (the one source; the floor is a Raspberry Pi 5 with 1 GB as the client, with a 4 GB Pi 5
able to host as the dedicated server, owner 2026-10-04); this design states what it spends
against them. Low poly is the style, so the budgets are ceilings, not targets.

## Context

The Tern is an 84 m frigate (`data/ships/tern/layout.json`). Its crew walk three decks, sit at
fourteen stations, climb into four turret pods and drop out of two launch bays in fighters,
while the ship flies Newtonian in a star system (`flight-and-navigation`). The owner asked for
an engine that decouples "the bridge and internal ship aspects from the exterior of the ship".

Three observations shape the design:

- **The crew and the ship need different physics.** Crew need a character controller on a
  static floor with gravity down. The ship needs a rigid body with six degrees of freedom.
  Putting both in one world makes every manoeuvre a crew catastrophe and every crew step a
  precision problem.
- **But the crew must feel the ship.** A bridge sim where nothing shakes when a missile hits,
  and the floor never lurches when the dampers fail, has no stakes. So the feeling is computed
  and applied on purpose, from the ship's real motion, through the inertial dampers.
- **A Pi 5 renders both in one frame.** The bridge view shows the room, the space outside
  its windows and a viewscreen picture, on OpenGL ES 3.0 within 200,000 triangles and 300
  draw calls (and it should spend far less: the game "doesn't need high end graphics").

Pale-Blue-Dot's rules are adopted as the frame discipline: `f64` system coordinates and orbital
time; `f32` in bounded local frames with the origin subtracted before the cast; poses,
interpolation history, physics state and frame-relative velocities rebased together at a tick
boundary; passengers simulated in the vessel's local frame.

## Goals / Non-Goals

**Goals:**
- Crew never move with the ship's physics; what they feel is a computed residual.
- One function for each frame transform, used by the server, the client, the renderer and the
  tools.
- A craft crosses between the interior and the system frame at one tick, with no visible pop
  and with its velocity carried over.
- The bridge view composes interior, windows and viewscreen inside the Pi 5 budget, on
  GLES 3.0 with no extension.
- Precision limits stated as numbers, with the rule that keeps every drawn thing inside them.

**Non-Goals:**
- Rotating habitats or spin gravity: the Tern's gravity is artificial and along -Y.
- Relativistic effects, light delay, or n-body motion for ships (Pale-Blue-Dot's rule: ships
  are Newtonian with local gravity falloff; `flight-and-navigation`).
- The body response itself (how a crew member stumbles, falls or floats): `crew-on-deck` owns
  it and consumes the residual defined here.
- Wire encodings: `netcode-and-sessions` owns them and encodes the frames named here.

## Decisions

### 1. The frames

| Frame | Id on the wire and in saves | Origin | Axes | Position type | Holds |
| --- | --- | --- | --- | --- | --- |
| System | `sys` | The star (system barycentre) | Fixed: the system's reference plane, +Y its north | `f64` metres; time `f64` seconds | Bodies on rails (analytic orbits), every ship's and free craft's pose, saves |
| Ship | `ship:<id>` | The layout origin (centreline, deck B floor, hangar forward wall) | Ship axes: +X port, +Y dorsal, +Z bow | Pose in `sys`: position `f64`, orientation unit quaternion (`f64` on the server) | The ship as a rigid body |
| Interior | `int:<ship id>` | Same point as the ship frame | Same axes as the ship frame | `f32` metres (within 60 m of the origin) | Decks, crew, loose objects, fixtures, craft on cradles, missiles in tubes and on racks |
| Near | `near:<ship id>` | The ship's centre of mass at the current tick | System axes | `f32` metres (within 20 km) | Collision tests near the ship, bodies sent to that ship's crew (`netcode-and-sessions`, "relative to own ship") |
| Mount | `mount:<ship id>:<mount id>` | The mount's `center_m` | Mount axes: +Y along the mount's `facing`, then turret yaw and elevation | Angles, `f32` radians | Turret heads, the turret sight camera, the viewscreen camera |
| Craft | `craft:<craft id>` | The craft's centre of mass | Craft axes: +Z nose, +Y up | `f32` metres | The fighter cockpit and its pilot, the shuttle cabin |
| Camera | `cam` | The eye, this frame only | View axes | `f32` | Rendering only. Never simulated, never sent. |

**The interior and ship frames share geometry but not dynamics.** They have the same origin and
axes, so a point on a deck has the same coordinates in both. They differ in what is simulated:
the ship frame is a rigid body that accelerates and rotates in the system; the interior frame
is treated as an inertial room with gravity -Y, and the only ship motion it receives is the
dampers' residual (section 4). That distinction is the decoupling.

**Rule: a frame is named, never implied.** A type carries its frame: `SysPos(f64x3)`,
`IntPos(f32x3)`, `NearPos(f32x3)`; a pose is `{ frame, position, orientation }`. Converting is a
call to `frames::`, never inline arithmetic (CLAUDE.md 6.1: the frame transforms exist once).

### 2. The transforms

Notation: the ship's state is its centre-of-mass position `P_cm` (`f64`, sys), orientation `q`
(rotation matrix `R`), velocity `V_cm` (sys), angular velocity `w` (ship axes), and the centre
of mass in ship coordinates `c` (from the mass table in `flight-and-navigation`: (0.09, 1.02,
-8.74) m for the Tern). The ship frame origin in sys is `P_s = P_cm - R c`.

```text
interior -> system   P   = P_s + R p                         (computed in f64)
system -> interior   p   = R^T (P - P_s)                     (f64, then cast to f32)
velocity of an interior point, in sys
                     V   = V_cm + R (w x (p - c)) + R v_int
system -> near       n   = f32(P - P_cm)                     (subtract in f64, then cast)
camera relative      r   = f32(P - P_eye)                    (subtract in f64, then cast)
```

The ship's rotation is about its centre of mass, so a point's velocity uses `p - c`, not `p`.
`v_int` is the point's own velocity in the interior frame (a cradle lowering a fighter, a
missile in its tube's ejection stroke).

### 3. The interior is a fixed space

- Crew bodies, loose objects (extinguishers, repair kits, a missile on a trolley) and every
  interior mechanism step in the interior frame at the server's 30 Hz tick, against the deck's
  collision brushes (`deck-pipeline`), with gravity `g_int = (0, -g_art, 0)`.
- `g_art` is the gravity generator's output: 9.81 m/s^2 at full power, falling with its supply
  (the generator's draw and supply are power-grid's; zero-g movement is crew-on-deck's).
- The ship's acceleration, rotation and velocity never enter interior physics, except through
  the residual of section 4 and the shake of section 5. Gravity from planets never enters at
  all: the ship and everything in it fall together, so it is not felt (only proper
  acceleration is).
- The decks, being static, are compiled once (`deck-pipeline`) and never transformed on the
  CPU. A crew position is a small `f32` (or `int16` centimetres on the wire, netcode section 4).

### 4. The inertial dampers: what the crew feel

The dampers (system `inertial_dampers` in the drive section) cancel the ship's proper
acceleration inside the hull, up to their capacity. What they cannot cancel is the residual,
and the residual is applied to crew and cameras.

**Per tick, per compartment `k`** (30 compartments on the Tern; the point is the compartment's
centroid `r_k` in ship coordinates):

```text
demand      D_k = a_p + alpha x (r_k - c) + w x (w x (r_k - c))        ship axes, m/s^2
              a_p    proper acceleration of the centre of mass: thrust, collisions and weapon
                     impulses divided by mass; gravity excluded (free fall is not felt)
              alpha  angular acceleration, w angular velocity (ship axes, rad/s^2, rad/s)
capacity    C   = C_rated * s_d * h_d
              C_rated = 25 m/s^2; s_d = supply ratio from power-grid (0..1);
              h_d = dampers' health from damage-control (0..1)
cancel      A_k <- clampLen( A_k + (D_k - A_k) * (1 - exp(-dt / tau_d)), C )
              tau_d = 0.05 s (the dampers' lag), dt = 1/30 s
residual    E_k = D_k - A_k
felt        F_k = E_k if |E_k| >= 0.3 m/s^2, else 0
```

A crew member in compartment `k` is pushed by `-F_k` (thrown opposite to the ship's
acceleration). The dampers draw 2 MW standing by plus up to 6 MW in proportion to
`max_k |A_k| / C_rated` (assumed, see power-grid), so a hard manoeuvre shows on engineering's
console as a draw spike.

**What the residual does** (the body response is crew-on-deck's; these are the values this
change proposes to it):

| Residual, horizontal part `h = |F_k with y removed|` | Standing or walking crew | Seated crew |
| --- | --- | --- |
| Under 0.3 m/s^2 | Nothing | Nothing |
| 0.3 m/s^2 to grip (`mu * g_art` = 0.6 x 9.81 = 5.9 m/s^2) | Camera sway and tilt; walking speed x (1 - 0.5 h / grip) | Camera sway and tilt |
| Grip to 2 x grip (5.9-11.8 m/s^2) | Stumble: input off 0.5 s, the body slides at `(h - grip)` along `-F_k` | Camera sway, clamped |
| Over 2 x grip (11.8 m/s^2) | Knocked down 1.5 s; a wall struck above 3 m/s injures (crew-on-deck) | Strapped in: camera only |
| Vertical: felt gravity `g_art + F_k.y` under 2 m/s^2 | Feet lose grip: zero-g movement (crew-on-deck) | Strapped in |
| Vertical: felt gravity over 25 m/s^2 | Forced crouch | Camera only |

**The camera lurch** (every crew camera, and a fighter pilot's with the craft's own dampers):

```text
head offset  o'' = -F_k - k_h o - c_h o'        k_h = (2 pi 1.5 Hz)^2 = 88.8 s^-2
                                                c_h = 2 * 0.7 * sqrt(k_h) = 13.2 s^-1
             |o| <= 0.12 m
head tilt    theta = 0.5 * atan(|F_h| / g_art) about the axis perpendicular to F_h,
             through the same spring, |theta| <= 10 deg
```

The offset and tilt are added after the look rotation. **Mouse and stick look are never
smoothed or eased** (Pale-Blue-Dot's rule: raw displacement each frame); the lurch is an
additive offset on top, so aiming stays exact relative to the head.

**Worked numbers for the Tern** (centre of mass and inertia from `flight-and-navigation`;
manoeuvre limits from its helm table; compartments from the layout):

| Manoeuvre | Bridge | Torpedo room | Drive | Port pod | Hangar | Worst compartment |
| --- | ---: | ---: | ---: | ---: | ---: | --- |
| Full burn, 15 m/s^2 | 15.0 | 15.0 | 15.0 | 15.0 | 15.0 | all equal |
| Yaw start, 12 deg/s^2 | 7.2 | 8.0 | 5.7 | 3.7 | 0.1 | torpedo room, 8.0 |
| Yaw at 18 deg/s | 3.4 | 3.8 | 2.7 | 1.7 | 0.0 | torpedo room, 3.8 |
| Full burn and a yaw reversal at full rate | 13.7 | | 18.6 | 11.3 | 15.0 | drive, 18.6 |
| Roll reversal, 30 deg/s^2 at 36 deg/s | 2.6 | | 0.3 | 8.0 | 0.8 | starboard pod, 8.1 |

Demand in m/s^2. With the dampers at full supply and health (`C` = 25 m/s^2) none of these is
felt in steady state. With the dampers at half supply (12.5 m/s^2) a full burn leaves 2.5 m/s^2
(a sway) and the worst case 6.1 m/s^2 in the drive section (a stumble). With a quarter (6.25
m/s^2) a full burn leaves 8.75 m/s^2: anyone standing stumbles, and the worst case (12.35 m/s^2)
knocks people down. Without dampers a full burn floors the whole crew. That is the gameplay:
engineering's power to the dampers decides how hard helm can fly with people standing up, and
`flight-and-navigation`'s "damper-safe" limit (section 6 there) keeps helm inside `C` unless
helm turns it off.

**Transients.** The lag `tau_d` lets fast changes through. A throttle slammed from 0 to full,
which the drive ramps over 0.5 s (30 m/s^3, `flight-and-navigation`), leaves `30 x 0.05` =
1.5 m/s^2 for half a second at full damper power: a brief sway, never a stumble. A collision
or a missile impulse arrives inside one tick, so about half of it (`exp(-dt/tau_d)` = 0.51) is
felt for that tick before the dampers catch up: impacts always get through.

### 5. Hit shake

A hit adds trauma `T_k` in [0, 1] to every compartment, and the shake is drawn from it:

```text
on a hit:   T_k += min(1, E_shake / 20 MJ) / (1 + d_k / 15 m)
            E_shake = 0.25 * energy absorbed by the shields + 1.0 * energy reaching the hull
            d_k = distance from the hull point to compartment k's centroid
each tick:  T_k = max(0, T_k - 1.5 s^-1 * dt)
camera:     shake angle = 2.5 deg * T^2, offset = 0.04 m * T^2,
            direction from 1-D value noise at 15 Hz, seeded by (session, ship, tick of the hit)
crew:       T_k > 0.7 on a hit: standing crew stumble as for a residual over grip
```

Hull points and energies come from `weapons-and-shields` (the damage resolution); what the hit
does to compartments and systems is `damage-control`'s.

### 6. Precision: `f64` system, `f32` local, the floating origin

`f32` carries 24 bits of mantissa; `f64` carries 53. The error of a coordinate stored at
distance `d` from its origin is half a unit in the last place:

| Distance from the origin | `f32` step | `f32` error | Pixels, for a thing 2 m from the eye drawn with that origin | `f64` step | What lives there |
| --- | ---: | ---: | ---: | ---: | --- |
| 50 m | 3.8 um | 1.9 um | 0.001 | 7e-15 m | The whole interior |
| 1 km | 61 um | 31 um | 0.02 | 1e-13 m | Close combat |
| 10 km | 0.98 mm | 0.49 mm | 0.26 | 2e-12 m | The near layer's typical reach |
| 20 km | 1.95 mm | 0.98 mm | 0.51 | 4e-12 m | The near layer's limit |
| 50 km | 3.9 mm | 1.95 mm | 1.0 | 7e-12 m | Visible jitter starts |
| 1,000 km | 63 mm | 31 mm | 16 | 1e-10 m | Planet scale: never `f32` |
| 1 AU | 16 km | 8.2 km | 4 million | 31 um | System scale: `f64` only |
| 40 AU | 524 km | 262 km | | 1 mm | The edge of a system: `f64` still holds 1 mm |

Pixels assume 1280 px across a 70 degree field of view (about 1,050 px per radian).

**Rules that follow:**
- **System positions and time are `f64`, always.** At 40 AU `f64` still resolves 1 mm, and
  mission time in `f64` seconds resolves well under a microsecond for centuries.
- **The render origin is the eye.** Every frame, every exterior position is
  `f32(P - P_eye)` with the subtraction in `f64` on the CPU. GLES 3.0 has no `f64` in shaders;
  model-view matrices are built in `f64` and cast once. Error is then about `d * 6e-8` at
  distance `d` from the eye: never visible.
- **Nothing is drawn with an origin more than 10 km from it** (0.26 px at 2 m). The near layer
  ends at 20 km; beyond it, bodies are drawn in the far layer (section 7).
- **The interior never needs the origin moved.** It spans 84 m; `f32` resolves 4 um there.
- **Collision near a ship runs in its near frame** (`f32`, within 20 km: under 1 mm). The
  server holds the authoritative `f64` and builds near-frame positions each tick, so there is
  no stored `f32` state to rebase: the rebase is implicit in the subtraction, every tick.
- **Orientation** is a unit quaternion, renormalized every tick on the server (`f64`), sent as
  netcode's 32-bit smallest-three. Its error (about 1e-4 rad) turns the stars through a window
  by 0.1 px: invisible.

**Depth precision.** The depth step at view depth `z` is about `z^2 / (n * 2^bits)` for near plane
`n`. OpenGL ES 3.0 guarantees a 24-bit depth renderbuffer (with an 8-bit stencil), so 24 bits is
the floor:

| Layer, 24-bit depth | Near plane | Far plane | Step at 10 m | at 100 m | at 1 km | at 5 km | at 20 km |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Interior | 0.05 m | 120 m | 0.12 mm | 12 mm | | | |
| Exterior near | 1 m | 20 km | 6 um | 0.6 mm | 60 mm | 1.5 m | 24 m |

Both layers are comfortable: bodies near each other at 20 km are tens of metres apart, and the
interior's longest sight line is the 84 m keel. Reversed depth with a float buffer would help
only with `glClipControl`, which ES 3.0 lacks, so it is not used. Beyond 20 km bodies go to the
far layer, which needs no depth at all.

### 7. Rendering composition, one frame

The frame is drawn in this order (engine-stack section 7: "exterior first, then interior, then
glass, then UI"). Every pass uses **one ship pose per rendered frame**, interpolated or
predicted once (`netcode-and-sessions` section 5), so the windows, the viewscreen and the hull
cannot disagree.

| # | Pass | Camera | Depth | What is drawn |
| --- | --- | --- | --- | --- |
| 1 | Viewscreen and feed targets (section 8) | Virtual camera in the ship frame, composed with the ship pose | Its own depth buffer | Passes 2 and 3 into the 1024 x 512 viewscreen texture at 30 Hz; at most one 512 x 256 secondary feed per frame |
| 2 | Exterior far layer | The eye's exterior orientation only (no translation) | Test and write off; stencil test = window | Stars (points, one call), the sun (a billboard), planets and moons as scaled proxies (direction and angular size true, drawn at a fixed 1,000 m in a 2,000 m depth range), bodies beyond 20 km as point sprites |
| 3 | Exterior near layer | The eye composed with the ship pose, origin at the eye | Near 1 m, far 20 km; stencil test = window | The own ship's exterior model and attachments, other ships, craft, missiles, the instanced projectile billboards, shield flashes, engine glow |
| 4 | Depth clear | | Clear | Inside the window scissor rectangle (the stencil is kept for pass 6) |
| 5 | Interior | The eye in the interior frame | Near 0.05 m, far 120 m | The compartments portal culling finds (`deck-pipeline`), crew, fixtures, the viewscreen quad sampling pass 1's texture |
| 6 | Glass and overlays | Interior | Test on, write off | Window glass (one translucent call, stencil test = window), console screens' emissive quads |
| 7 | UI | Screen | Off | Console panels and HUD (`bridge-stations`) |

**Windows are a stencil mask inside a scissor rectangle.** Portal culling already projects
every visible portal to a screen rectangle. When the eye is in the interior:

1. If no portal to `space` (a window, an open bay door, an open airlock) is visible, passes 2 to
   4 are skipped: a room with no window pays nothing for the exterior.
2. Otherwise the scissor is set to the union of those rectangles, and the visible space portals'
   polygons (the window quads, from the deck file) are drawn into the stencil (value 1, colour
   and depth writes off). That is one draw call of a few triangles.
3. Passes 2 and 3 draw with the stencil test "equal 1", so the exterior's fill is spent on the
   window pixels exactly.
4. The depth clear (pass 4) is scissored; the interior pass draws over everything, its walls
   leaving the window openings as holes, and a crew member standing in front of a window simply
   draws over the exterior there.

OpenGL ES 3.0 guarantees the 8-bit stencil with the 24-bit depth buffer, so this is the floor,
not an extension. The scissor alone (without the stencil) is a correct fallback, since the
interior covers everything outside the openings; it only spends more fill (question F4).

**The own hull is invisible from inside.** The exterior model is drawn with back-face culling,
and the eye in the interior is inside the hull's closed surface, so every hull face points away
from it and is culled. A window therefore shows space and anything outside the hull (the bow's
sensor dome, another ship, a turret seen from the turret access), never the inside of the hull.
`reference-ship-tern` keeps window portals within the hull's convex region so this holds; the
exterior model's build checks that the hull mesh is closed and outward wound (see "Proposed kit
fix" below).

**Exterior views** use the same passes with the interior pass replaced:

| View | Eye frame | Passes | Notes |
| --- | --- | --- | --- |
| Bridge, seated or walking | Interior | 1, 2, 3 (stencil masked), 4, 5, 6, 7 | The most common view |
| Turret sight | Mount (turret yaw and elevation) | 2, 3 full screen, 7 (sight reticle and lead pip as UI) | The gunner sits in the static pod; the view is the turret head's sight (question F2) |
| Fighter cockpit | Craft | 2, 3 full screen, 4, cockpit mesh (about 1,200 triangles, 2 calls, in the craft frame), 7 | The cockpit is the fighter's interior: same decoupling, same lurch formula with the fighter's dampers |
| Chase or external | System, following a body | 2, 3, 7 | Spectators, the debrief, helm's optional external view on the viewscreen |

### 8. The viewscreen and the secondary feeds

- **The viewscreen** is a fixture (`viewscreen`, 6.0 x 2.4 m on the bridge's forward wall) whose
  picture is a 1024 x 512 RGBA8 colour texture with a 24-bit depth and 8-bit stencil
  renderbuffer: 2 MB plus 2 MB of GPU memory (engine-stack section 5: one viewscreen target at
  1024 x 512, at most 30 Hz). The engine-stack table says "about 3 MB with depth", which is a
  16-bit depth buffer; a 24-bit depth is stored in 32 bits, so this design counts 4 MB and asks
  engine-stack's table to say so (or to adopt a 16-bit depth for the viewscreen, which its
  1 m to 20 km range does not suit).
- **Its camera** is a virtual camera in the ship frame: mounted at the sensor array
  ([0.0, 1.0, 41.5] m, the bow) looking along +Z by default. Science (or the captain) steers it:
  forward, aft, port, starboard, dorsal, ventral, tracking a contact, or an external chase view
  of the Tern. Steering slews at 90 deg/s so the picture never jumps. Zoom 10 to 70 degrees of
  vertical field of view.
- **Refresh 30 Hz**: every other frame at the 60 fps target, every frame at the 30 fps floor;
  sampled every frame. The frame that renders it is the frame the budget counts (question F3).
- **Secondary feeds**: up to two 512 x 256 targets (1 MB each with depth) that a console shows as
  a picture: tactical's view down a turret's sight, flight ops' view from a fighter's nose camera.
  Each refreshes at 15 Hz, and they alternate frames, so at most one renders in any frame. They
  draw the exterior with the far LOD (section 14), never the interior.
- These three are the only render targets on the bridge. Console screens are UI drawn by the
  engine's immediate-mode layer (CLAUDE.md 10), never render targets; a console that shows a feed
  samples its texture in a UI quad.

### 9. Ship attachments

Anything that moves with the ship but also moves on it is an **attachment**: turret heads and
barrels, missile tube doors, bay drop doors, cradles, the shuttle lift pad, and a craft riding a
cradle. An attachment's pose is stored in the ship frame (`mount:` angles or an interior pose)
and drawn in the exterior pass with the ship's pose. It never holds an `f64` pose of its own,
so it can never drift from its ship.

- **A craft on a cradle is an interior object** even when the cradle has lowered it half out of
  the hull: it is posed in the interior frame and drawn in both passes (in the interior pass
  for the crew in the bay, in the exterior pass for anyone outside). It becomes an exterior
  body only at release (section 10).
- **Draw cost:** the Tern's static exterior is one call; its moving attachments are drawn
  instanced by part (turret head x 4, barrel x 8, door leaf x 6, cradle and pad x 3, tube door
  x 2), each instance with a matrix composed from the ship pose and its ship-frame pose: five
  calls. Engine glow is one additive call. The whole ship is about 7 draw calls.

### 10. Hand-off between the interior and the system frame

A craft (fighter, shuttle), a missile or an ejected pilot's pod crosses frames at **one tick
boundary**. The server's tick order makes it exact:

1. Apply commands in a stable order (`netcode-and-sessions`, section 6).
2. Step the interior: crew, cradles, hoists, tubes (interior frame).
3. Step the exterior: ships, free craft, missiles, projectiles (system frame `f64`; collisions in
   near frames).
4. **Hand-off phase**, in ascending stable id: every object flagged for release in step 2
   becomes exterior; every object flagged for capture in step 3 becomes interior.
5. Damage resolution, systems sub-step (every third tick), snapshot.

**Release (interior to system)**, with the ship's state at the end of step 3:

```text
P_craft   = P_s + R p_int
V_craft   = V_cm + R (w x (p_int - c)) + R v_int          v_int: cradle stroke plus ejection
q_craft   = q * q_int
w_craft   = R^-1-free: w_craft (craft axes) = q_int^-1 (w) + w_int   (w_int = 0 on a cradle)
mass      the ship's mass drops by the craft's; its centre of mass and inertia are recomputed
```

**Capture (system to interior)** is the inverse:

```text
p_int     = R^T (P_craft - P_s)
v_rel     = R^T (V_craft - V_cm) - w x (p_int - c)        relative velocity at the capture point
```

The capture mechanism absorbs `v_rel` (it must be under the capture limit,
`shuttle-bay-and-fighters`) as an impulse on the ship (fed to the dampers as `a_p` for that tick)
and the craft is then posed by the cradle. **A snapshot never holds an object in both sets**:
the snapshot of tick `n` shows it in the frame it has after step 4 of tick `n`.

**No visible pop.** The conversion error is the `f64` step at the ship's distance from the star
plus the `f32` step at 60 m: under 0.1 mm at 1 AU. The client interpolates across the change by
converting the older sample into the newer sample's frame (using the ship pose it already has for
that tick) before interpolating, never the other way round.

**Turret bolts** are never interior objects: they are born in the system frame at the muzzle
with the muzzle point's velocity (`V_cm + R (w x (p_muzzle - c))`) plus the bolt's speed along
the barrel (`weapons-and-shields`).

### 11. Network: every pose names its frame

This change names the frames; `netcode-and-sessions` encodes them. The agreement:

| What | Frame | Encoding (netcode-and-sessions, section 4) |
| --- | --- | --- |
| Crew avatars | `int:<ship>` | 3 x int16 centimetres |
| Own ship pose | `sys`, relative to the session's origin body | 3 x `f64`, quaternion 32 bit, velocities |
| Other bodies near the ship | `near:<own ship>` | 3 x `f32` metres, quaternion 32 bit, velocity 3 x int16 |
| Craft on cradles, missiles in tubes | `int:<ship>` | The cradle's or tube's state, not a pose |
| Turret heads | `mount:<ship>:<mount>` | Yaw and elevation, 2 x int16 |
| Projectiles | `sys` | Not sent: a reliable "fired" event, simulated on the client |

A message whose frame id does not resolve (a ship or craft that no longer exists) is dropped and
logged, never applied in some other frame.

### 12. Other ships, docking and boarding

- **Another ship's interior is never rendered** unless a local player is aboard it, or it is
  docked to the player's ship and visible through the docking portal. An NPC ship (the raider
  corvette, `weapons-and-shields`) has no interior at all: an exterior model and a systems
  summary.
- **Docking** joins two vessels rigidly. The smaller becomes an attachment of the larger: its
  pose is stored in the larger's ship frame, and it stops being integrated. Docking ports align
  the two ships' +Y axes, so artificial gravity is continuous across the joint. The docking
  portal joins the two compartment graphs (atmosphere flows, `life-support`; crew path across).
- **Crossing a docking portal** changes a crew member's frame at the tick their centre crosses
  the portal plane: `p_B = T_BA p_A`, with `T_BA` the fixed docked transform. Their velocity is
  transformed by the same rotation.
- **A second interior's decks** load when docking completes (not when a crew member crosses),
  inside a second-interior allowance this change requests from engine-stack's buffer budget:
  12 MB of the 64 MB of vertex and index buffers (a station module or derelict of about 10
  compartments).
- **Boarders** (an enemy party through the airlock or a breach) are crew bodies in the boarded
  ship's interior frame, simulated exactly like its own crew.

### 13. Saves

A save stores each object in the frame it is in: a docked craft as `{ frame: "int:tern", cradle:
"cradle_p", state }`; a craft in flight as `{ frame: "sys", P, q, V, w }` in `f64`; a crew member
as `{ frame: "int:tern", p, yaw }`. Loading converts nothing. Save versioning and migration are
CLAUDE.md 7's rule.

### 14. The Pi 5 budget this change spends

**Rendering**, per view, against 200,000 triangles and 300 draw calls per frame, all passes
(engine-stack section 5). The content sizes are engine-stack's ceilings: own ship 12,000
triangles within 2 km (3,000 to 8 km, 600 beyond), fighter 1,500 (400 at the far LOD), missile
200, projectiles instanced (two triangles each, up to 1,024, one call); the raider corvette is
6,000 (`weapons-and-shields`). Low-poly art will sit well under these; the table shows that even
at the ceilings the composition fits.

| View, in combat at the ceilings | Viewscreen (30 Hz) | One secondary feed (15 Hz, far LOD) | Exterior far | Exterior near | Interior and crew | Glass and UI | Total |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Bridge | 31,400 tris, 10 calls | 12,000, 12 | 2,500, 4 | 43,400, 18 (stencil masked) | about 40,000, 40 (deck-pipeline, crew-on-deck) | 500, 14 | about 130,000 tris, 98 calls |
| Turret sight | | | 2,500, 4 | 43,400, 17 | | 200, 6 | about 46,000, 27 |
| Fighter cockpit | | | 2,500, 4 | 43,400, 17 | 1,200, 2 (cockpit) | 200, 6 | about 47,000, 29 |
| Chase | | | 2,500, 4 | 43,400, 17 | | 200, 6 | about 46,000, 27 |

"Exterior near, in combat" is the own ship (12,000, about 7 calls with its instanced
attachments), a raider corvette (6,000, 2 calls), twelve fighters (18,000, one instanced call),
eight missiles (1,600, one call), 1,024 projectile billboards (2,048, one call), a shield flash
(about 1,200 when visible, one call) and engine glow (two calls). The viewscreen's forward camera
sits at the bow tip, so frustum culling drops the own ship from its pass (12,000 less).

**This change's own share** in an interior view (viewscreen, one feed, exterior far and near) is
held to **90,000 triangles and 50 draw calls** at the content ceilings, leaving at least 110,000
triangles and 250 calls to the interior, crew and UI.

**Against the 60 fps plan.** engine-stack asks scenes to plan at about half the ceiling (about
100,000 triangles and 150 calls) for 60 frames a second. The table above is every content item
at its ceiling at once, which is the busy moment the 200,000 ceiling exists for, not the plan.
With low-poly content the same frame sits near the plan: the exterior mockup's Tern is 2,026
triangles at LOD0 in 6 draw calls (against the 12,000 ceiling), its Jackal about 120 and its
Gannet about 70 (against 1,500 and 200), so a bridge view in a full engagement drawn with the
mockup's models spends about 25,000 triangles on the exterior passes instead of 90,000.

**Fill.** 3D renders at 1280 x 720 (921,600 pixels). The exterior passes in the bridge view cover
only the window pixels (two 2.8 x 1.2 m windows: under 10% of the screen from the captain's seat).
The viewscreen target is 524,288 pixels (57% of a frame) at 30 Hz, so about 28% of a frame's fill
on average at the 60 fps target; a secondary feed is 131,072 pixels at 15 Hz.

**Memory.** GPU: the viewscreen target 4 MB (colour 2 MB, depth and stencil 2 MB; section 8 on
engine-stack's "about 3 MB"), two feeds 1 MB each, instance buffers for attachments and projectiles 128 kB: about 6 MB of the 96 MB texture
budget. CPU: none beyond fixed pools (128 exterior bodies, 1,024 projectiles).

**CPU.** Frame transforms: 128 exterior bodies x one `f64` subtract and a matrix compose, about
0.02 ms per frame on a Cortex-A76. Dampers: 30 compartments x a few dozen flops per tick on the
server, under 0.01 ms against its 2 ms per ship per tick.

**Network.** Nothing new: the frames ride netcode's existing groups.

### 15. One implementation

Each of these exists once in `sc-core::frames` and is called by the server, the client's
prediction and interpolation, the renderer and the tools:

| Function | Used by |
| --- | --- |
| `interior_to_system`, `system_to_interior`, `point_velocity` | Hand-off, muzzles, the viewscreen camera, the renderer's camera compose |
| `to_near`, `to_camera_relative` | Collision, netcode encoding, rendering |
| `damper_step` (demand, cancel, residual per compartment) | The server, the client's own-avatar prediction, the mockup's numbers, the helm's damper-safe limit (`flight-and-navigation`) |
| `shake_add`, `shake_step` | Server and client |
| `release`, `capture` | Fighters, shuttle, missiles, pods |

## Proposed kit fix

`docs/mockups/lib/shipkit.js` `hullGeometry()` winds its side faces outward but both end caps
inward: the stern cap is built with `flip = false` and its normal points to +Z (inward), the bow
cap with `flip = true` and its normal points to -Z (inward). The exterior mockup corrects the
winding locally after building the geometry, testing every triangle against the loft's axis: it
rewinds exactly the 16 cap triangles (8 per cap) and no side face, which confirms the finding. The fix in the kit is to swap the two flags:
`cap(rings[0], true); cap(rings[rings.length - 1], false);`. The engine's exterior mesh
generator must test the same property (every face's normal points away from the hull's
interior), which is a task below.

## Risks / Trade-offs

- **The residual is a design, not physics.** Real dampers do not exist; the numbers are tuned
  for play. Risk: crew find the lurch annoying. Mitigation: the felt threshold and the camera
  gains are data (`data/ships/tern/dampers.json`), and question F1 asks with a shot.
- **The viewscreen costs fill.** At 1024 x 512 and 30 Hz it is about a quarter of a frame's
  pixels on average. Mitigation: the probe measures it on a Pi 5; the target drops to 768 x 384
  if fill binds, a data change.
- **The stencil pass** adds one draw call and a state change per frame with windows. Mitigation:
  the scissor-only path is correct too, and is the fallback if the probe finds the stencil slow.
- **Docked ships double the interior.** Mitigation: the 12 MB second-interior allowance, and
  docking targets are authored small.
- **Hand-off with a lagging client.** A client 100 ms behind sees a fighter leave its cradle
  100 ms late; the pilot's own client predicts the fighter from the release tick it receives.
  The pilot's controls take effect only after release (the cradle holds the craft until then),
  so there is nothing to mispredict before it.

## Mockup shots

`docs/mockups/exterior.html` presents this change:

| Shot | Shows |
| --- | --- |
| `decoupling-split` | Left: the exterior, the Tern rolled and turning against a fixed camera. Right: the bridge interior, level and steady, red alert; its windows and viewscreen show the turning sky. The overlay gives the dampers' demand, capacity and residual. |
| `decoupling-normal` | The same at normal lighting, a different moment of the roll. |
| `gunner-view` | The turret sight as an exterior view from the mount frame. |
| `fighter-cockpit` | The cockpit pass over the exterior, in the craft frame. |
| `fighter-drop` | A craft on its cradle, an attachment, lowered through the drop door, then released. |

## Open questions

Ids F (frames). Questions with a shot go to the owner's survey; the rest are recommendations
taken (ask only with screenshots, CLAUDE.md 13).

| Id | Question and the fact behind it | Options | Recommendation | Shot |
| --- | --- | --- | --- | --- |
| F1 | Should crew feel anything when the dampers are at full power? A throttle slam leaves 1.5 m/s^2 for 0.5 s through the dampers' lag. | (a) Yes: a brief sway and a 2 degree head tilt on slams and hits. (b) No: full dampers cancel everything except hits. | (a): it tells every crew member the ship is moving, without ever moving their feet. | `decoupling-split` |
| F2 | What does a seated gunner see? The pods are fixed compartments in the layout. | (a) The turret head's sight as a full-screen exterior view with a reticle; the pod stays still. (b) The pod's interior rotates with the turret and the gunner looks through its canopy. | (a): no rotating compartment, no collision against a moving room, the cheapest exterior view. | `gunner-view` |
| F3 | How often does the viewscreen refresh? The budget allows 30 Hz at 1024 x 512. | (a) 30 Hz: every other frame at 60 fps. (b) Every frame (60 Hz). (c) 15 Hz. | (a): smooth enough for a screen, and half of (b)'s cost. | `decoupling-split` |
| F4 | How are windows cut into the exterior? | (a) A stencil mask of the visible window polygons inside their scissor rectangle. (b) The scissor rectangle alone. | (a), with (b) as the fallback if the probe finds the stencil slow. | none: recommendation taken (ask only with screenshots) |
| F5 | When two vessels dock, which frame holds the joint? | (a) The smaller becomes an attachment of the larger. (b) Both stay free bodies held by a constraint. | (a): no constraint solver, no drift, one pose. | none: recommendation taken (ask only with screenshots) |
