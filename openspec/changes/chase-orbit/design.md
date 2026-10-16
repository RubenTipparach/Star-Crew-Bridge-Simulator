# Design: a smooth chase camera, and ORBIT round the ship

The owner's words are in the proposal.

| Decided elsewhere | Where |
| --- | --- |
| CHASE a toggle on the camera chosen; the chase camera behind the ship, the ship low and centred | `chase-camera` 1 |
| The captain's camera, CHASE and zoom on the bridge's main viewscreen; `console::on_screen` the one rule | `chase-camera` 2, `captains-viewscreen` 1 |
| Zoom a lens on whichever camera is on | `viewscreen-zoom` 2 |
| The bracket projected from where the feed's camera stands | `target-bracket` 1 |
| Snapshots at 20 Hz from a 30 Hz server, drawn `INTERP_DELAY_S` (0.1 s) behind | `netcode-and-sessions`, `sc-server` |

## 1. What makes the turn rough: measured

Measured before changing anything (CLAUDE.md 4), with a measurement instrument kept out of the commit: a line a frame
from the drill giving the frame's time and step, the two snapshots' ticks and the blend between them, and the Tern's
heading and pitch as drawn. The Tern's own turn is smooth: its rates are reached at `ang_accel_deg_s2` (12 degrees a
second squared in yaw and pitch, `data/ships/tern/flight.json`), so the rate drawn should change slowly.

| Run | Frames | Frame step | Rendered turn rate's swing about its 0.3 s mean, while turning | Frames on the newest snapshot alone |
| --- | --- | --- | --- | --- |
| Pi 5, release, windowed, real time (`--solo --station helm --bot --window`, 100 s) | 4129 | 21.7 ms median, 37.7 ms 95th percentile | 11 % median, **42 %** 95th percentile | 0 % |
| Pi 5, the film (`--headless --solo --station captain --film`, the clip's own run) | 1164 | 33.3 ms (fixed) | 9 % median, **83 %** 95th percentile | **33 %** |

The film's frames show the pattern plainly: the heading drawn advances 6.2, 10.6 and 8.8 milliradians on three frames
in turn, again and again, while the ship turns at a steady rate: a 10 Hz shudder. The cause is the interpolation's
clock, not the ship:

- **Snapshots are timed by arrival.** `Session::interpolation` places each snapshot at the moment the client received
  it and blends at "now minus 0.1 s" by the client's wall clock. Twenty snapshots a second from thirty ticks are taken
  one and two ticks apart in turn (33 and 67 ms of game time), but in single player the server is ticked from the
  client's frame loop and a frame that runs two ticks hands over two snapshots stamped with the same instant: the blend
  treats 67 ms of game as 0 ms and the next 33 ms as a whole frame. Across a network the arrival times carry the
  network's jitter the same way.
- **The film runs on its own clock.** The film steps the game 1/30 s a frame however long a frame takes to draw, so the
  wall clock that times the blend has nothing to do with the game's: a third of its frames show the newest snapshot
  with no blend at all.

And the camera makes it worse: the chase camera is welded to the ship (`Seat::feed_camera` and the bridge's
`render_feed` multiply the camera by the Tern's rotation), so the whole picture takes every step.

## 2. Snapshots on the server's clock

Each snapshot already carries its `tick`. The session keeps a **playback clock** in server seconds (`tick x DT`):

- `Session::advance(dt)`, called once a frame with the frame's own step (the film's 1/30 s, the game's frame time),
  moves it on by `dt`, then pulls it toward its goal, the newest snapshot's time less `INTERP_DELAY_S`, by
  `1 - exp(-dt / PLAYBACK_SYNC_S)` of the difference (`PLAYBACK_SYNC_S` = 1 s), and never past the newest snapshot. A
  clock more than `PLAYBACK_SNAP_S` (0.5 s) from its goal (the first snapshot, a new round's tick, a long stall) jumps
  to it.
- `Session::interpolation` blends the two snapshots whose ticks bracket the playback clock, by tick time.

So the pace of what is drawn is the frame's own step, whatever the snapshots' arrival does; the pull only takes out
drift between the client's clock and the server's (in single player, none: they are one clock). A tick that goes back
(a new drill) clears the older snapshots. The constants sit beside `INTERP_DELAY_S`, the netcode's own.

## 3. The chase camera follows with a lag

The camera's rig turns after the ship in the **system frame**, a critically damped follow of the ship's rotation:

```text
y  = rotation vector of (rig * ship_before^-1)      the rig's offset from the ship, system frame
u  = rotation vector of (ship_now * ship_before^-1) / dt   the ship's turn over the frame, taken as even
w  = 2 / follow_tau_s
z  = y + 2u/w,  vz = v - u                           about a target turning at u, a still target's equation
temp = (vz + w z) dt
z' = (z + temp) e^(-w dt),  vz' = (vz - w temp) e^(-w dt)
y' = z' - 2u/w,  v' = vz' + u
rig' = exp(y') * ship_now
```

This is the exact solution of the critically damped follow `x'' = -2 w x' - w^2 (x - ship)` over the step (the
"SmoothCD" form with the exact exponential, shifted for a target turning evenly through the frame), so a still target or
a steady turn lands on the same answer for any number of frames of any length: **framerate independent** (measured by
its test: 20, 30 and 144 frames a second within 0.05 degrees of 240). From rest
it is `y(t) = y0 (1 + w t) e^(-w t)`, which falls to zero without crossing it: **no overshoot**. Turning at a steady
rate `r` the rig settles `follow_tau_s x r` behind: at the Tern's 18 degrees a second in yaw, **7.2 degrees** at
`follow_tau_s` = 0.4 s, enough to see the ship swing in the picture and not so much that the target leaves it.

The pose the camera takes (where it stands and the way it looks, ship axes, `console::feed_view`) is turned by the
rig's offset from the ship (`ship^-1 * rig`) instead of the ship's rotation. The bow camera does not lag: its offset is
eased in and out with CHASE (section 4), so with CHASE off it is the ship's rotation exactly, as before. A rig that has
not run for over a second (a console shown again) starts at the ship's rotation.

## 4. Camera changes ease

When the pose the camera wants changes identity (a camera pressed, CHASE or ORBIT on or off) the camera eases from the
pose it last showed to the new one over `blend_s` (0.5 s), with the smoothstep `k = 3s^2 - 2s^3` of the share `s` of
`blend_s` gone:

- where it stands: straight between the two (the bow camera stands at `BOW_EYE`);
- the way it looks: the two orientations slerped;
- the field of view: straight between the two (chase's 100 degrees across and the bow's 70);
- the lag of section 3: weighted by `k` toward the chase end, so it fades in as CHASE comes on and out as it goes off.

A zoom is a lens and stays immediate (`viewscreen-zoom` 2). The Tern's hull is drawn in the feed whenever the camera is
not wholly the bow camera.

## 5. ORBIT

**The button.** ORBIT goes in the camera widget's pad, a third row under CHASE: the same 38 x 25 keycap with a glyph,
`orbit` (a ring round a dot with an arrow on it), lit while ORBIT is on and in the unavailable state while CHASE is off.
The approved mockup gets it first (`docs/mockups/consoles.html`, CLAUDE.md 10). Its tip: "Orbit the ship".

**The camera.** An orbit is a look line round the ship: a heading `yaw_deg` (0 along the bow, positive to starboard,
all the way round) and an elevation `pitch_deg` (held within `orbit.pitch_limit_deg`, 80 degrees, short of the poles)
and a distance `distance_m` (held to `orbit.distance_m`, 90 to 600 m). The camera stands as the chase camera does on
that line, through the same function (`console::chase_camera` with the orbit's distance): behind the ship along the
line, raised until the ship sits low, looking along it. So the orbit is the chase camera with its line set by hand,
and turning ORBIT on starts from the chase camera's own line and distance: nothing moves until the player drags. The
orbit is in ship axes, so it keeps its place round the ship as the ship flies, and the lag of section 3 applies.

**The controls** (the console's own input; the feed is drawn under the console's look band):

| Input | Does |
| --- | --- |
| ORBIT, or O (`data/input/bindings.json` `viewscreen.orbit`) | On or off, while CHASE is on |
| Drag on the viewscreen (mouse or one finger) | Across its width: `orbit.drag_deg_across` (360 degrees) of heading; down its height: half that of elevation. The ship turns the way the pointer goes |
| Wheel on the viewscreen | One notch in: the distance over `orbit.wheel_step` (1.15); out: times it |
| Pinch on the viewscreen | The distance over the fingers' spread's ratio |

The hand's numbers are the orbit's goal; the orbit drawn follows them with the same damper as section 3 at
`orbit.smooth_tau_s` (0.15 s), so a drag glides and stops softly. ORBIT off, or CHASE off, eases back through section
4. O is free at every station (the stations' keys are E, Tab, M, Y, G, T, L, Q, X, W, S, A, D, R, F, Z, C, Space, Left
Ctrl and the arrows; V and F3 are the drill's), so it is one binding for every console.

## 6. The captain's orbit on the main viewscreen

`Command::MainView { feed, chase, zoom, orbit }`: `orbit` is `None`, or his orbit's goal in three bytes,
`[heading, elevation, distance]`:

| Byte | Maps to | Step |
| --- | --- | --- |
| heading | 0-255 over 0-360 degrees | 1.4 degrees |
| elevation | 0-255 over -80 to +80 degrees (the limit) | 0.63 degrees |
| distance | 0-255 over 90-600 m, logarithmic | 0.74 % |

`sc_core::exterior::Orbit::quantize` and `unquantize` are the one mapping; the server checks only that it is three
bytes. The captain's seat sends it when ORBIT, the camera, CHASE or zoom change, and while he drags at most
`orbit.send_hz` (10) times a second when the bytes change, and once more when he lets go. `Bridge::main_orbit` holds it
and the snapshot carries it (four bytes: a flag and the three).

`console::on_screen(view, main, own)` stays the one rule: while the captain holds the viewscreen every panel shows his
camera, CHASE, zoom and orbit; otherwise each console its own, and the bridge's main viewscreen the captain's. The
captain's own console always shows his own orbit, so his drag is not delayed by the round trip. Every receiving rig
smooths the orbit it is given (section 5's damper), so the steps of the bytes and the 10 Hz updates glide; the heading
is unwrapped to the nearest turn of what is shown.

The protocol goes from 16 to 17; an older client is refused at Hello as for every protocol change, so no client is ever
given a message it cannot read.

## 7. One camera state

`console::rig::FeedRig` holds the follow, the blend and the orbit, and `FeedRig::step(now, ship rotation, want)` returns
the camera this frame: where it stands and its axes in the ship's frame, its field of view, and whether the hull is
drawn. The seat steps its rig once a frame in `Seat::view` and puts the result in `ConsoleView::cam`; the 3D feed
(`DrillApp::render_feed`) turns it into the system frame by the ship's rotation, and the console's bracket
(`look_band`) projects the target through it. The bridge's main viewscreen has its own rig in the drill, stepped where
it is drawn. A console fixture (the parity check) has no rig: its bracket uses the camera `feed_view` gives, which is
the rig's settled pose.

## Pi 5 budget

| Cost | Amount |
| --- | --- |
| Rendering | None new: the hull is drawn while the camera is not wholly the bow camera, as it is drawn now while CHASE is on (0.5 s more of it while CHASE eases off) |
| CPU | A few quaternion operations and three exponentials a frame for each rig stepped (the console shown and the main viewscreen) |
| Memory | About 200 bytes a rig |
| Network | Four bytes a snapshot; while the captain drags, a 9-byte command at most 10 times a second |

## Open questions

| Id | Question | Recommendation | Status |
| --- | --- | --- | --- |
| O1 | Where ORBIT sits | Under CHASE, a third row of the pad, the same keycap | Recommendation taken (ask only with screenshots); the mockup and the shots show it |
| O2 | Whether ORBIT also works with CHASE off (round the bow camera) | No: an orbit is a camera outside the ship | Recommendation taken |
| O3 | Whether the orbit is kept when CHASE goes off and on | No: ORBIT goes off with CHASE, and starts again from the chase camera | Recommendation taken |
