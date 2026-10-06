# Flight and Navigation

## Purpose

How the Tern and every craft fly: one Newtonian six-degree-of-freedom model with flight assist and a
damper-safe limit, thrust that follows power, helm controls, bodies on rails with local gravity, the
jump drive between points of interest, navigation, autopilot, docking and collisions.

## ADDED Requirements

### Requirement: One flight model for every ship and craft
Every ship and craft SHALL be integrated by one function at the 30 Hz tick in `f64` in the system frame:
semi-implicit Euler for position and velocity, Euler's rotation equations with the gyroscopic term for
angular velocity, and a renormalized quaternion for orientation, parameterized by its data (mass table,
inertia, thrusts, torques, limits). Its centre of mass and inertia SHALL be computed from the layout's
hull and the mass table, never typed in.

#### Scenario: The Tern's inertia
- **WHEN** the inertia tool runs on the Tern's layout and mass table
- **THEN** it reports a mass of 3,000 t, a centre of mass at (0.09, 1.02, -8.74) m within 0.05 m, and
  diagonal moments of 1.37e9, 1.42e9 and 1.11e8 kg m^2 within 2%

#### Scenario: A Swift and the Tern share the integrator
- **WHEN** the Swift's flight data is loaded into the same function
- **THEN** a full burn accelerates it at 60 m/s^2 and no craft-specific integration code exists

### Requirement: Thrust follows the power delivered
Each thruster group's available force or torque SHALL be its rated value times the supply ratio
power-grid delivers to it, and its draw SHALL be its standby draw plus the rest of its rated draw (rated
less standby) times the fraction of rated force used, as power-grid's demand rule. The main engines SHALL ramp to full in 0.5 s and RCS in 0.1 s.

#### Scenario: Half power to the drive
- **WHEN** the drive's supply ratio is 0.5 and helm demands a full burn
- **THEN** the Tern accelerates at 7.5 m/s^2 and the drive draws half its rated 16 MW, 8 MW

### Requirement: Flight assist has three modes
Full assist SHALL hold the commanded rotation rates and the commanded velocity relative to the
reference (the smallest body on rails whose influence radius holds the ship, or a matched target),
compensating gravity, with time constants 1.0 s (linear) and 0.25 s (angular) and a speed cap of 400 m/s
for the Tern. Rotation-only assist SHALL hold rotation rates and pass throttle and strafe through as
thrust. Assist off SHALL pass the stick through as torque and the throttle as thrust.

#### Scenario: Coasting with assist off
- **WHEN** helm switches assist off at 200 m/s and centres every control
- **THEN** the Tern keeps 200 m/s and its rotation, changed only by gravity

#### Scenario: Turning about to brake
- **WHEN** helm sets 0 m/s at 400 m/s in full assist without a heading lock
- **THEN** the Tern turns about and burns, and stops in about 38 s instead of 80 s on reverse thrusters

### Requirement: The damper-safe limit keeps the crew on their feet unless helm chooses otherwise
While the damper-safe limit is on (the default), the assist SHALL scale helm's commanded linear and
angular accelerations so that the dampers' predicted demand, computed by `ship-frames`' function, stays
within 0.9 times the dampers' capacity in every compartment, and helm SHALL see when it binds. Helm SHALL
be able to switch it off with a confirmation, in every assist mode.

#### Scenario: Half-powered dampers
- **WHEN** the dampers' supply ratio is 0.5 and helm demands a full burn with the limit on
- **THEN** the Tern accelerates at 11.25 m/s^2, helm's throttle shows "damper limit", and nobody aboard
  stumbles

#### Scenario: Emergency manoeuvre
- **WHEN** helm switches the limit off and makes the same burn
- **THEN** the Tern accelerates at 15 m/s^2 and the crew feel a 2.5 m/s^2 residual

### Requirement: Every input device drives the helm
Keyboard and mouse, gamepads and HOTAS devices SHALL all map to the same helm commands (pitch, yaw, roll,
throttle, two strafes and buttons), and any connected device SHALL drive the seated helm.

#### Scenario: A pad on the helm
- **WHEN** a player sits at helm with only a gamepad connected
- **THEN** the left stick pitches and yaws, the triggers set the throttle and the bumpers roll

### Requirement: Bodies ride analytic rails and pull with local gravity
Planets, moons and stations SHALL move on Keplerian rails evaluated analytically in `f64`, never
integrated or perturbed by ships. A ship SHALL feel gravity `mu / r^2` toward each body, scaled by a
smoothstep from 1 at half that body's influence radius to 0 at it.

#### Scenario: Leaving a planet's influence
- **WHEN** the Tern crosses a planet's influence radius outward
- **THEN** that planet's pull on it has fallen smoothly to zero, with no step in acceleration

### Requirement: The jump drive crosses a system at a tick
A jump SHALL need 800 MJ spooled at the power delivered (40 s at the 20 MW full allocation), the bow
within 5 degrees of the jump vector and turning under 2 deg/s for the last 3 s, and no mass lock (3 radii
of a planet or moon, 5 km of a ship over 500 t, 2 km of a station). It SHALL move the ship at one tick to
the destination's arrival point within a seeded 1.5 km, keep its velocity relative to the reference,
carry own craft in formation within 1.5 km, leave everything else behind, and cool down for 60 s.

#### Scenario: Mass locked by the corvette
- **WHEN** helm spools the jump with a Hound 3 km away
- **THEN** the spool refuses with "mass lock: Hound 3.0 km" and nothing is spent

#### Scenario: A fighter outside the field
- **WHEN** the Tern jumps with Swift 1 4 km away
- **THEN** Swift 1 stays where it was with its fuel and damage, and flight ops lists it as left behind

### Requirement: Autopilot and docking obey the assist's limits
Every autopilot mode (hold station, approach, orbit, point bow, follow course, dock, evade) SHALL drive the
flight assist and obey its limits, the damper-safe limit included. Docking SHALL require alignment within
2 degrees with the two ships' +Y agreeing, closing under 2 m/s inside 100 m, 0.5 m/s inside 10 m and
0.2 m/s at contact; contact within those limits SHALL latch the ports and anything faster SHALL be a
collision.

#### Scenario: Docking too fast
- **WHEN** a ship reaches a docking port at 0.6 m/s
- **THEN** it bounces as a collision and the ports do not latch

### Requirement: Collisions are impulses that damage both bodies and jolt the crew
Exterior collisions SHALL be detected in the near frame between convex shapes, capsules and spheres and
resolved by an impulse with restitution 0.2 and friction 0.3. The energy lost SHALL be split equally
between the two bodies as kinetic hull hits that bypass the shields, and the impulse SHALL enter the
dampers as that tick's proper acceleration.

#### Scenario: Ramming a fighter
- **WHEN** the Tern strikes a Jackal head on at 100 m/s
- **THEN** about 43 MJ is lost, the Jackal is destroyed, the Tern takes a 21.5 MJ kinetic hull hit, and
  the crew feel a one-tick jolt of about 5.5 m/s^2
