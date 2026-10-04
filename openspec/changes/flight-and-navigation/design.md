# Design: flight and navigation

Status: **proposed** (2026-10-04). Nothing here is built. Power draws are `power-grid`'s (marked "see
power-grid"); this design's thrusts scale with the supply ratio it delivers, so a change there
rescales nothing here. Budget numbers are against `openspec/changes/engine-stack/design.md` section 5
(the Pi 5 table, the one source).

## Context

The Tern is 84 m long, 24 m in the beam and about 14 m deep (`data/ships/tern/layout.json`): an
octagonal loft from z -42 to +42 m, with the reactor through three decks in engineering, the impulse
drive and inertial dampers in the drive section astern, two main engines at [+/-3.2, 1.0, -42.0]
facing aft, and the bridge 30 m forward of the centre of mass.

Pale-Blue-Dot's flight invariants are adopted: bodies on analytic rails; ships Newtonian with local
gravity falloff, configurable linear and angular limits and dampers; `f64` system coordinates; no
patched conics, manoeuvre nodes, transfer windows or n-body ships. In this game "inertial dampers"
are the crew-comfort system of `ship-frames`; what Pale-Blue-Dot calls dampeners is here the
**flight assist**.

## Goals / Non-Goals

**Goals:**
- One flight model for every ship and craft (the Tern, the Swift, the Petrel, the Jackal, the Hound),
  parameterized by data.
- A ship that is honest Newtonian underneath and easy to fly with assist on.
- Helm's choices visible to the crew: the dampers, the bow, the steadiness a launch needs.
- A mission crosses a star system in minutes, not days.

**Non-Goals:**
- Orbital mechanics for ships (no Kepler propagation of ships, no transfer planning).
- Atmospheric flight and landing on planets.
- Relativity.

## Decisions

### 1. The Tern as a rigid body

**Mass table** (estimates; `reference-ship-tern` and `power-grid` may refine the items):

| Item | Mass | Position (ship frame, m) |
| --- | ---: | --- |
| Structure, decks, fittings | 2,000 t | Spread evenly over the hull envelope (21,566 m^3, 93 kg/m^3) |
| Fusion reactor | 450 t | [0.0, 1.5, -25.0] |
| Impulse drive | 300 t | [0.0, 1.5, -37.0] |
| Shield generator | 60 t | [4.2, -2.5, 3.0] |
| Swift x 2 and Petrel | 44 t | The cradles and the pad |
| Gannets x 12 | 13 t | The magazine |
| Stores, water, craft propellant | 133 t | [0.0, -2.0, 8.0] |
| **Total** | **3,000 t** | |

**Centre of mass**: (0.09, 1.02, -8.74) m: aft of the layout origin, because the reactor and drive are
heavy and aft.

**Inertia tensor** about the centre of mass (integrated over the hull loft on a 0.5 m grid with the
point masses; a tool recomputes it from the layout and this table):

| | x | y | z |
| --- | ---: | ---: | ---: |
| x (pitch) | 1.367e9 | 9.5e5 | -3.3e6 |
| y (yaw) | 9.5e5 | 1.416e9 | 1.6e7 |
| z (roll) | -3.3e6 | 1.6e7 | 1.106e8 |

In kg m^2. The products of inertia are under 1.2% of the diagonal; the model keeps the full tensor
anyway (it costs nothing). A ship 12 times longer than it is deep turns in pitch and yaw about 12
times harder than it rolls.

**Mass changes**: launching a craft, firing a missile and burning craft propellant change the mass,
centre of mass and inertia at that tick (`ship-frames` section 10).

### 2. Propulsion from power

| Thruster | Force or torque | Acceleration | Rated draw (see power-grid) |
| --- | ---: | ---: | ---: |
| Main engines (2) | 2 x 22.5 MN along +Z | 15 m/s^2 (1.5 g) | 60 MW at full thrust, 2 MW standing by |
| Reverse thrusters (bow) | 15 MN along -Z | 5 m/s^2 | Part of RCS |
| RCS translation | 12 MN along +/-X and +/-Y | 4 m/s^2 | Part of RCS |
| RCS torque, pitch | 2.9e8 N m | 12 deg/s^2 | Part of RCS |
| RCS torque, yaw | 3.0e8 N m | 12 deg/s^2 | Part of RCS |
| RCS torque, roll | 5.8e7 N m | 30 deg/s^2 | Part of RCS |
| RCS | | | 8 MW at full use |

**Thrust follows power.** Each thruster group's available force is `F_rated x s`, where `s` is the
supply ratio power-grid delivers to it (1 at nominal; above 1 only if power-grid allows
over-allocation, with its heat). Its draw follows use: `P = P_standby + P_rated x |F_used| / F_rated`.
So a ship at rest draws little, a full burn shows on engineering's console, and an engineer who cuts
the drive's breaker leaves helm with RCS only.

**Slew**: the main engines ramp from zero to full in 0.5 s (30 m/s^3); RCS in 0.1 s. That slew is what
`ship-frames`' dampers lag turns into a brief sway.

**Thrust line**: the engines' combined line sits 0.02 m below the centre of mass and 0.09 m to
starboard of it, so a full burn makes a pitch torque of about 0.9 MN m and a yaw torque of about
4 MN m, which the assist trims with about 1.3% of the RCS's yaw authority.

### 3. Integration

At the server's 30 Hz tick, in `f64` in the system frame (`ship-frames`):

```text
F   = sum of thruster forces (rotated to sys) + collision impulses / dt
g   = local gravity at P_cm (section 8)
V  += (F / m + g) dt
P  += V dt                                      semi-implicit Euler
tau = sum of thruster torques (ship axes)
w  += I^-1 (tau - w x (I w)) dt                  Euler's equations, gyroscopic term kept
q   = normalize(q + 0.5 dt q * (0, w))
a_p = F / m                                      proper acceleration, sent to the dampers
alpha = (w_new - w_old) / dt
```

Bodies near the ship are integrated the same way; collisions are found in the near frame
(`ship-frames` section 6) and resolved as impulses (section 11).

### 4. Flight assist

Three modes, chosen by helm (and by a fighter's pilot, with the craft's data):

| Mode | Stick (pitch, yaw, roll) | Throttle | Strafe | Gravity |
| --- | --- | --- | --- | --- |
| **Full** (default) | Rotation rate set point | Forward speed set point, -100 to +400 m/s | Lateral and vertical speed set points | Compensated |
| **Rotation only** | Rotation rate set point | Thrust (fraction of full) | Thrust | Not compensated |
| **Off** | Torque (fraction of full) | Thrust | Thrust | Not compensated |

**The controller** (the same function for every ship and craft):

```text
linear, full assist:
  v_target = V_ref + R v_set                     V_ref: reference body's or target's velocity
  a_cmd    = (v_target - V) / tau_v - g          tau_v = 1.0 s
  a_cmd    -> ship axes, clamped per axis to the thrust limits and the damper-safe limit
angular, full and rotation-only:
  alpha_cmd = (w_set - w) / tau_w                tau_w = 0.25 s
  tau_cmd   = I alpha_cmd + w x (I w)            clamped per axis to the RCS torques
```

**Rate limits under assist**: pitch and yaw 18 deg/s, roll 36 deg/s (Tern); Swift 90, 60 and 180
deg/s. The **assist speed cap** is 400 m/s relative to the reference for the Tern (320 m/s for the
Swift, 250 m/s for the Jackal); with assist off or rotation-only there is no cap, only momentum.

**What it feels like**, Tern, full assist, nominal power:

| Manoeuvre | Time | Distance |
| --- | ---: | ---: |
| 0 to 400 m/s | 27 s | 5.3 km |
| 400 m/s to rest with the reverse thrusters | 80 s | 16 km |
| 400 m/s to rest by turning about and burning | 38 s (11.5 s turn, 27 s burn) | 10 km |
| Turn 180 degrees | 11.5 s | |
| Roll 180 degrees | 6.2 s | |
| Strafe 50 m sideways from rest to rest | 7.1 s | |

The assist turns the ship about to brake when the speed error exceeds 100 m/s and helm has not
locked the heading (a "flip and burn"), because the main engines are three times the reverse
thrusters.

**Reference frame**: velocities are held relative to the reference, which is the smallest body on
rails whose influence radius contains the ship (else the star), or the designated target in "match
velocity" mode. Helm's speed readout names its reference ("312 m/s rel. Corvid II").

### 5. The damper-safe limit (coupling to ship-frames)

The dampers cancel up to `C = 25 m/s^2 x supply x health` (`ship-frames` section 4). On by default, the
**damper-safe limit** makes the assist keep the predicted demand inside `0.9 C` in every compartment:

```text
for each compartment k (centroid r_k, relative to the centre of mass):
  D_k(lambda) = lambda (a_cmd + alpha_cmd x r_k) + w x (w x r_k)
choose the largest lambda in [0, 1] with |D_k(lambda)| <= 0.9 C for all k
  (each k is a quadratic inequality in lambda; take the smallest bound)
if even lambda = 0 fails (spinning too fast for the dampers), command only rotation-rate reduction
apply lambda a_cmd and lambda alpha_cmd
```

It calls `ship-frames`' `damper_step` demand function (one implementation). When it binds, helm's
throttle shows "damper limit" and the achieved acceleration. **Helm may switch it off** (with a
confirmation) for emergency manoeuvres; the crew then feel the full residual. With the dampers at
half supply (`C` 12.5 m/s^2) the limit allows a forward burn of 11.25 m/s^2.

Assist off and rotation-only modes apply the limit too, unless it is switched off: a helm flying
Newtonian still cannot throw the crew without choosing to.

### 6. Helm controls

Every input device drives the helm (CLAUDE.md 10), and all of them produce the same commands
(`netcode-and-sessions` input channel: pitch, yaw, roll, throttle, strafe x, strafe y, buttons):

| Action | Keyboard and mouse | Gamepad | HOTAS |
| --- | --- | --- | --- |
| Throttle | W / S ramp (hold), X to zero, Tab to full | Right trigger forward, left trigger reverse | Throttle axis, absolute |
| Pitch | R / F, or the mouse as a virtual stick | Left stick Y | Stick Y |
| Yaw | A / D, or the mouse as a virtual stick | Left stick X | Twist or pedals |
| Roll | Q / E | Bumpers | Stick X |
| Strafe lateral | Z / C | Right stick X | Hat X |
| Strafe vertical | Space / Ctrl | Right stick Y | Hat Y |
| Assist mode | V cycles | D-pad up | Button |
| Match target velocity | M | D-pad right | Button |
| Autopilot engage / release | P | Y | Button |
| All stop (full assist: set point zero) | Backspace | B (hold) | Button |
| Damper-safe limit | Console button with confirm | | |
| Jump: spool / abort | Console buttons | | |

The mouse "virtual stick" sets a rotation rate from the cursor's offset from centre, with a dead
zone; it is a helm command, not a player's look (Pale-Blue-Dot's raw-look rule is for looking, and
it holds: the seated helm's head still looks with raw mouse displacement when the console is
released).

### 7. In-system travel: the jump drive (recommended)

| Quantity | Value |
| --- | ---: |
| Energy to spool | 800 MJ |
| Spool draw | 40 MW at full allocation (assumed, see power-grid): 20 s; longer with less power (`t = 800 MJ / P_delivered`) |
| Alignment | The bow within 5 degrees of the jump vector and turning under 2 deg/s for the last 3 s of the spool |
| Mass lock | No spool or jump within 3 radii of a planet or moon's centre, 5 km of a ship over 500 t (the Hound locks the Tern), or 2 km of a station |
| Range | Any point of interest in the system |
| Arrival | At the point of interest's arrival point (authored at least 5 km from anything), scattered within 1.5 km (seeded) |
| Cooldown | 60 s after arrival |
| Velocity | Kept relative to the reference: `V_after = V_ref(dest) + (V_before - V_ref(src))` |
| Jump field | Own craft in formation within 1.5 km jump with the ship, keeping their offsets; anything else stays behind and persists |
| Transit | Instant, at one tick boundary: the ship's `f64` position is moved (`ship-frames`: attachments and the interior are untouched); a one-tick 5 m/s^2 jolt through the dampers is the jump's signature |

**Why a jump and not a cruise drive**:

| | Jump drive (recommended) | Cruise drive (speed up to 0.01 c) |
| --- | --- | --- |
| Crew work | Engineering finds 40 MW (from weapons or shields), helm aligns and holds, flight ops recalls craft, captain picks the destination: everyone has a job for 20 s | Helm steers for minutes; others wait |
| Mission time | 20 s plus the spool | Minutes per AU at the edge of playability |
| Frames | One rebase at a tick; nothing moves fast near anything | Very high speeds near bodies; constant far-layer motion; collisions at 3,000 km/s |
| Escape and pursuit | Mass lock makes "drive the corvette off before you can jump" a goal | Interdiction rules needed |
| Network | One event | A pose that changes by thousands of km per tick |

**Helm automation** spools only on the captain's order (or tactical's when there is no captain).

### 8. Bodies on rails and local gravity

- Each star system is a data file: the star, planets, moons, stations and points of interest. Each
  body has Keplerian elements about its parent (semi-major axis, eccentricity, inclination, node,
  argument, mean anomaly at epoch, the parent's `mu`), a radius, `mu = GM`, an influence radius and a
  mass-lock radius. Positions and velocities at time `t` are analytic: Kepler's equation by Newton's
  method (five iterations in `f64`). No body is integrated; none is perturbed by ships.
- **Local gravity**: `g = sum over bodies of mu / r^2 x f(r / r_inf)`, toward each body, where `f` is 1
  inside half the influence radius and falls by a smoothstep to 0 at it. Typical mission points sit
  where `g` is under 0.5 m/s^2 (an Earth-like planet's `g` is 0.1 m/s^2 at 10 radii), so holding
  station costs little thrust.
- Gravity acts on the whole ship, so the crew do not feel it (`ship-frames` section 3); thrust that
  holds against it is felt (proper acceleration), and the dampers cancel it.

### 9. Navigation

**System map** (helm and captain; science sees contacts on it): a top-down view of the system on its
reference plane with a logarithmic zoom from 1 km to 100 AU: bodies with their orbits, points of
interest (stations, beacons, asteroid fields, wrecks, mission markers), sensor contacts (from
science), the ship's velocity vector and its straight-line-plus-gravity path for the next 60 s, mass
lock circles, the jump target and spool state, waypoints and the course.

**Local plot** (helm): 1 to 50 km around the ship, its heading and velocity, contacts with their
vectors, the tubes' 60 and 90 degree launch cones (to point the bow for tactical), the turret arcs
seen from above, the recovery corridor when a craft is inbound, and time-to-collision warnings for
any body whose straight-line closest approach is under 100 m within 30 s.

### 10. Autopilot and docking

Every mode drives the flight assist (section 4), so the autopilot obeys the same limits as helm:

| Mode | Does |
| --- | --- |
| Hold station | Velocity zero relative to the reference or the target |
| Approach | Fly to within a set distance of a point or target and match its velocity; desired closing speed `min(cap, 0.8 sqrt(2 x 5 m/s^2 x d))`, flipping to burn when that is faster |
| Orbit | Circle a target at a radius and speed |
| Point bow | Keep the bow on the designated target (for the tubes), with or without holding position |
| Follow course | Approach each waypoint in turn |
| Dock | Fly a docking corridor (below) |
| Evade (automation) | Lateral and vertical jinks of 3 m/s^2 changing every 4-8 s (seeded), turning the strongest shield face to the main threat |

**Docking**: a docking port is a mount with a facing and an up vector. The corridor is the port's axis:
aligned within 2 degrees and rolled so the two ships' +Y agree (`ship-frames` section 12), closing at
no more than 2 m/s inside 100 m, 0.5 m/s inside 10 m and 0.2 m/s at contact. Contact inside 0.3 m and
those limits latches the ports; anything faster bounces as a collision. Helm can dock by hand with
these limits shown, or hand it to the autopilot. Auto recovery of craft (`shuttle-bay-and-fighters`)
is the same mode with the bay's capture point as the port.

**Helm automation** (no helm player): point bow on the designated target and evade under fire, hold
station otherwise, dock and approach on order; reaction 2 s, damper-safe always on, assist always
full. It never jumps and never launches craft on its own.

### 11. Collisions

| Body | Shape |
| --- | --- |
| Tern | The convex hull of the hull loft (80 vertices; the loft is slightly concave at z +37 m, so the hull is a little larger than the loft) plus four spheres for the turret pods |
| Hound, stations, asteroids | Convex hulls or compounds of a few |
| Swift, Jackal, Petrel | Capsules |
| Missiles, pods | Spheres |
| Planets, moons | Spheres |

Broad phase in the near frame on a 100 m grid; narrow phase GJK with EPA for depth; swept tests for
fast small bodies. **Response**: an impulse at the contact point with restitution 0.2 and friction
0.3, through both bodies' inertia. **Damage**: the energy lost,
`E = 0.5 x mu_red x v_n^2 x (1 - e^2)` with `mu_red = m1 m2 / (m1 + m2)`, is split equally between the
two as kinetic hull hits that bypass the shields (`weapons-and-shields` section 12). **Crew**: the
impulse divided by the mass and the tick is the proper acceleration of that tick, so about half of it
reaches the crew through the dampers' lag (`ship-frames` section 4).

Example: the Tern ramming a Jackal at 100 m/s: 43 MJ lost, 21.5 MJ to each (the Jackal is destroyed);
the Tern's velocity changes by 0.36 m/s in one tick, a 10.8 m/s^2 jolt of which about 5.5 m/s^2 is
felt for that tick, plus the shake from 21.5 MJ at the hull. A ship meeting a planet faster than
20 m/s is destroyed.

### 12. The Pi 5 budget this change spends

**Server CPU** (2 ms per ship per tick on one Cortex-A76): the Tern's integration, assist and
damper-safe solve over 30 compartments, under 0.02 ms; up to 128 bodies integrated with broad-phase
collision, under 0.2 ms; about 20 bodies on rails (Kepler solves), under 0.01 ms. **Client**: the own
ship's pose interpolation and a pilot's fighter prediction through the same model (netcode section 5),
negligible. **Rendering**: the system map and local plot are 2D UI (`bridge-stations`), a few batched
calls; planets are far-layer proxies (`ship-frames`). **Network**: the own ship pose is already in
netcode's table (48 bytes at 20 Hz); helm's inputs ride the input channel; a jump is one reliable
event. **Memory**: a system file of a few kB; fixed slots for 128 bodies.

### 13. Data

| File | Holds |
| --- | --- |
| `data/ships/tern/flight.json` | The mass table, thrusts and torques, rated and standby draws, slews, rate limits, assist time constants and speed cap, the damper-safe margin (0.9) |
| `data/ships/tern/jump.json` | Spool energy and draw, alignment, mass-lock radii by body class, arrival scatter, cooldown, field radius, jolt |
| `data/systems/<id>.json` | Bodies on rails, points of interest with arrival points |
| `data/craft/*.json`, `data/enemies.json` | Craft and enemy flight data, same schema |

The inertia tensor and centre of mass are computed from the layout's hull and the mass table by a
tool (validate the real artifact, CLAUDE.md 6.6), never typed in.

## Layout note (proposed for reference-ship-tern)

The jump drive needs a place. Proposed addition to `systems`, in the drive section beside the impulse
drive and the dampers (inside the `drive` box):

```json
{ "id": "jump_drive", "name": "Jump drive", "kind": "propulsion", "compartment": "drive", "center_m": [-4.0, 0.0, -36.0] }
```

## Risks / Trade-offs

- **Full assist hides Newton.** A player who never turns it off never meets momentum. Mitigation: the
  speed cap and the flip-and-burn make momentum visible; tutorials teach rotation-only.
- **The damper-safe limit could feel like a nerf.** Mitigation: helm sees "damper limit" and the reason,
  and engineering can raise it with power; helm can turn it off.
- **The jump makes space small.** Mitigation: mass lock makes it a decision; travel within a point of
  interest is still flown.
- **The mass table is a guess.** The inertia it gives is within a factor of about 1.5 of any sensible
  distribution; rates and accelerations are set by thrust and torque data, which are tuned in play.

## Mockup shots

| Shot | Shows |
| --- | --- |
| `chase` | The Tern flying in combat from behind: engines lit, turrets working |
| `decoupling-split` | The ship rolling and turning outside while the bridge holds level, with the dampers' demand and capacity |

## Open questions

Ids N (navigation). Questions with a shot go to the owner's survey; the rest are recommendations taken
(ask only with screenshots, CLAUDE.md 13).

| Id | Question and the fact behind it | Options | Recommendation | Shot |
| --- | --- | --- | --- | --- |
| N1 | How does the ship cross a system? 1 AU at 1.5 g takes about two days. | (a) Jump drive with spool, alignment and mass lock. (b) Cruise drive. (c) Both. | (a): every station has a job in a jump, and nothing moves fast near anything | none: recommendation taken (ask only with screenshots) |
| N2 | Do craft come along on a jump? | (a) Own craft in formation within 1.5 km. (b) Only stowed craft. | (a): a fighter is not stranded by a helm in a hurry | none: recommendation taken (ask only with screenshots) |
| N3 | Is the damper-safe limit on by default? With it on, helm cannot throw the crew without choosing to. | (a) On, helm can switch it off. (b) Off by default. | (a) | `decoupling-split` |
| N4 | What is the Tern's assist speed cap? | (a) 400 m/s. (b) 250 m/s (fighters always faster). (c) None. | (a): the Tern can disengage from fighters' guns (1,200 m range) but not outrun a missile | none: recommendation taken (ask only with screenshots) |
| N5 | Does the assist flip the ship to brake? The main engines are three times the reverse thrusters. | (a) Yes, unless helm locks the heading. (b) Never. | (a) | none: recommendation taken (ask only with screenshots) |
