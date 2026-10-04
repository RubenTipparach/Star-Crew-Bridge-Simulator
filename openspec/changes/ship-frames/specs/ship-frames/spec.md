# Ship Frames

## Purpose

How a ship's interior is decoupled from its motion through space: the named frames and their
transforms, the fixed interior, what the inertial dampers let the crew feel, the floating
origin, the rendering composition of interior, windows and viewscreen, ship attachments, the
hand-off of craft between frames, and docking.

## ADDED Requirements

### Requirement: Every pose names its frame
Every position, orientation and velocity in the simulation, on the wire and in a save SHALL
carry the id of its frame: `sys`, `ship:<id>`, `int:<ship id>`, `near:<ship id>`,
`mount:<ship id>:<mount id>` or `craft:<craft id>`. Conversions between frames SHALL be made only
by the `sc-core::frames` functions. A message or save entry whose frame id does not resolve SHALL
be rejected and logged, never applied in another frame.

#### Scenario: A pose for a craft that no longer exists
- **WHEN** a client receives a pose in frame `craft:swift_9` and no craft `swift_9` exists
- **THEN** the pose is dropped, a warning naming the frame is logged, and no other body moves

#### Scenario: Two encodings of the same point agree
- **WHEN** a point on deck B is converted interior to system and back through `frames`
- **THEN** it returns within 0.1 mm of where it started, with the ship 1 AU from the star

### Requirement: The interior does not move with the ship
Crew bodies, loose objects, craft on cradles and missiles in tubes SHALL be simulated in their
ship's interior frame, with gravity of the gravity generator's output along -Y. The ship's
position, velocity, orientation, angular velocity and acceleration SHALL NOT enter interior
physics except through the dampers' residual and the hit shake. Gravity from bodies on rails
SHALL NOT enter interior physics.

#### Scenario: A hard turn with the dampers at full power
- **WHEN** the Tern yaws at 18 deg/s with the dampers at full supply and health
- **THEN** a crew member standing on the bridge stays within 1 cm of where they stood, and
  their interior coordinates are unchanged by the turn

#### Scenario: Falling toward a planet
- **WHEN** the Tern coasts toward a planet with a local gravity of 3 m/s^2 and no thrust
- **THEN** no residual is applied to any compartment from that gravity

### Requirement: The dampers' residual is computed per compartment and applied on purpose
Each tick the simulation SHALL compute for every compartment the demand `a_p + alpha x (r - c) +
w x (w x (r - c))`, the dampers' cancellation following it with a 0.05 s lag and limited to the
capacity `25 m/s^2 x supply ratio x dampers' health`, and the residual (demand minus
cancellation). A residual under 0.3 m/s^2 SHALL have no effect. A larger residual SHALL be
applied to the crew in that compartment as an acceleration opposite to it and to their cameras
as a spring offset of at most 0.12 m and a tilt of at most 10 degrees, added after the look
rotation, which itself is never smoothed. The tuning SHALL live in data with units.

#### Scenario: Half-powered dampers on a full burn
- **WHEN** the Tern burns at 15 m/s^2 with the dampers at a supply ratio of 0.5 and full health
- **THEN** every compartment's steady residual is 2.5 m/s^2 aft, standing crew sway without
  stumbling, and seated crew see only a camera sway

#### Scenario: Unpowered dampers on a full burn
- **WHEN** the Tern burns at 15 m/s^2 with no power to the dampers
- **THEN** the residual is 15 m/s^2 aft everywhere, over twice the grip limit, and every
  standing crew member is knocked down

#### Scenario: Look input stays raw under a lurch
- **WHEN** a gunner moves the mouse by 10 pixels during a lurch
- **THEN** the look direction changes by exactly the sensitivity times 10 pixels relative to the
  head, and the lurch only offsets and tilts the head

### Requirement: Hits shake the ship by distance and energy
A weapon hit SHALL add trauma to every compartment of `min(1, E / 20 MJ) / (1 + d / 15 m)`, where
`E` is a quarter of the energy the shields absorbed plus all the energy reaching the hull, and
`d` is the distance from the hull point to the compartment. Trauma SHALL decay at 1.5 per second
and drive a seeded camera shake of at most 2.5 degrees and 0.04 m.

#### Scenario: A missile through the bow face
- **WHEN** 30 MJ of a Gannet's blast reaches the hull at the bow
- **THEN** the torpedo room's trauma rises by more than the engineering bay's, and both shakes
  are identical on every client that renders them

### Requirement: System coordinates are f64 and drawn relative to the eye
System positions and simulation time SHALL be stored as `f64`. Every exterior position drawn in
a frame SHALL be converted to `f32` relative to the eye's system position, with the subtraction in
`f64`. Collision near a ship SHALL use that ship's near frame. The near exterior layer SHALL end
at 20 km from the eye; bodies beyond it SHALL be drawn in the far layer.

#### Scenario: A fight at 40 AU
- **WHEN** the Tern fights at 40 AU from the star
- **THEN** a fighter 50 m from the eye is drawn with less than 0.01 px of positional error

### Requirement: The bridge view composes exterior, windows and interior in a fixed order
A frame with the eye in an interior SHALL be drawn in this order: the viewscreen and feed
targets, the exterior far layer, the exterior near layer, a depth clear, the interior, glass and
overlays, then UI. The exterior layers SHALL be skipped when no portal to space is visible, and
otherwise restricted to the visible space portals by a scissor rectangle and a stencil mask. All
passes of a frame SHALL use one ship pose. The own ship's exterior model SHALL be drawn with
back-face culling so it is invisible from inside the hull.

#### Scenario: A room with no window
- **WHEN** the eye is in the mess, whose portals lead only to other compartments
- **THEN** the frame draws no exterior pass at all

#### Scenario: Looking out of the bridge window during a roll
- **WHEN** the Tern rolls at 20 deg/s and the captain looks through the port window
- **THEN** the stars turn in the window while the bridge's walls, floor and consoles stay still
  on screen, and no hull face is drawn in the window

### Requirement: The viewscreen is a steerable render target
The bridge viewscreen SHALL show a 1024 x 512 picture rendered from a virtual camera in the ship
frame (at the sensor array by default) at 30 Hz, steerable to the six ship axes, a tracked
contact or an external view, slewing at no more than 90 deg/s. At most two secondary feeds of 512 x
256 at 15 Hz SHALL exist, rendering on alternate frames with the far LODs.

#### Scenario: Tracking a fighter
- **WHEN** science sets the viewscreen to track a contact 30 degrees off the bow
- **THEN** the picture slews to it in about a third of a second and follows it, while the
  bridge around the screen stays level

### Requirement: Attachments are posed in the ship frame
Turret heads, barrels, tube doors, bay doors, cradles, the lift pad and any craft riding a cradle
SHALL hold their pose in the ship frame (angles or an interior pose) and SHALL be drawn with the
ship's pose. An attachment SHALL NOT hold an `f64` pose of its own.

#### Scenario: A fighter half out of the hull
- **WHEN** the port cradle has lowered Swift 1 through the drop door and the Tern rolls
- **THEN** Swift 1 rolls with the Tern exactly, seen from inside the bay and from outside

### Requirement: Craft cross between frames at one tick boundary
A craft, missile or escape pod SHALL change between the interior frame and the system frame only
in the hand-off phase of a server tick, in ascending stable id. On release its system velocity
SHALL be the ship's centre-of-mass velocity plus the rotation term at its point plus its own
interior velocity; on capture its relative velocity SHALL be absorbed by the capturing mechanism
as an impulse on the ship. No snapshot SHALL hold an object in both frames.

#### Scenario: Releasing a fighter while the ship turns
- **WHEN** Swift 1 is released with a 4 m/s ejection while the Tern moves at 200 m/s and yaws at 10
  deg/s
- **THEN** Swift 1's system velocity equals the Tern's velocity plus the yaw rate crossed with
  its offset from the centre of mass plus 4 m/s ventral, and its position is continuous to 0.1 mm

#### Scenario: One snapshot, one frame
- **WHEN** the snapshot of the release tick is taken
- **THEN** it lists Swift 1 among the exterior bodies and not on the port cradle

### Requirement: Another ship's interior is drawn only when the player is in it or docked to it
A client SHALL NOT load or draw another vessel's interior unless a local player is aboard it, or it
is docked to the player's vessel and visible through the docking portal. Docking SHALL make the
smaller vessel an attachment of the larger with their +Y axes aligned, and a crew member crossing
the docking portal SHALL change frame at the tick they cross its plane.

#### Scenario: A raider corvette alongside
- **WHEN** the raider corvette passes 200 m off the Tern's bow
- **THEN** only its exterior model is drawn, and no interior data for it is loaded

#### Scenario: Walking across a docking port
- **WHEN** a crew member walks from the Tern's airlock into a docked station module
- **THEN** at the tick their centre crosses the portal plane their frame becomes the station's
  interior, and their feet stay on a floor with gravity along -Y on both sides

### Requirement: The exterior passes stay inside their share of the Pi 5 budget
In any interior view, the viewscreen, one secondary feed and the two exterior layers together
SHALL submit at most 90,000 triangles and 50 draw calls at the engine-stack content ceilings.

#### Scenario: The bridge in a full engagement
- **WHEN** the Tern fights a corvette, twelve fighters, eight missiles and 1,024 projectiles, seen
  from the captain's seat with the viewscreen and one feed active
- **THEN** the frame's exterior passes report at most 90,000 triangles and 50 draw calls
