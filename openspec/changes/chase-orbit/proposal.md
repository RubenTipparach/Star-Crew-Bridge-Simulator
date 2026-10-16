# Proposal: a smooth chase camera, and ORBIT round the ship

## Why

The owner, 2026-10-16, after watching the captain's viewscreen clip with CHASE on: "can we get some smooth ness to the
rotation there? also can I get a button to freely orbit in chase came mode on the dash board? so I can orbit around
the ship like that?"

Two things make the turn look rough today (design 1 has the measurements):

- **The ships step.** The client draws the ships between two snapshots, timed by when each snapshot *arrived*, not by
  the server tick it was taken at. The server takes 20 snapshots a second from 30 ticks, so they are 1 and 2 ticks
  apart in turn, and a frame that runs two ticks (single player ticks the server from the frame loop) stamps two
  snapshots with one time. Measured on the Pi 5, the rendered turn rate swings 42 % (95th percentile) about its own
  0.3 s mean while the ship turns at a steady rate; in the recorded film it swings 83 %, a 10 Hz shudder.
- **The camera is welded to the ship.** The chase camera turns rigidly with the Tern, so every wobble of the turn is
  the whole picture's, and changing camera or toggling CHASE cuts.

## What Changes

- **Snapshots are drawn on the server's clock** (design 2): the client plays the snapshots back by their tick, on a
  playback clock that runs at the client's frame time and is pulled gently toward the newest snapshot. One rule for
  every ship, body and effect drawn between snapshots.
- **The chase camera follows the turn with a lag** (design 3): a critically damped follow in the system frame, time
  constant `follow_tau_s` (0.4 s), framerate independent, never overshooting. The ship is seen to turn in the picture
  and the camera swings round after it. The bow camera stays on the bow.
- **Camera changes ease** (design 4): a new camera, CHASE on or off, ORBIT on or off, eases from the old picture to the
  new over `blend_s` (0.5 s) instead of cutting.
- **ORBIT** (design 5): a new button in the camera widget, under CHASE, available while CHASE is on. With it on, a drag
  on the viewscreen (mouse or finger) turns the camera round the ship, all the way round and up and down short of the
  poles, the wheel or a pinch moves it in and out, and the O key (`data/input/bindings.json`) turns it on and off.
  Turning it off eases back to the chase camera.
- **The captain's orbit is on the main viewscreen** (design 6): `Command::MainView` carries his orbit, quantized to
  three bytes, and `console::on_screen` stays the one rule for what every panel and the main viewscreen show.
- **One camera state** (design 7): the console's target bracket is projected through the same smoothed camera the 3D
  feed is drawn from.

## Impact

- `sc-net::session`: the playback clock, interpolation by tick. `sc-net::msg`: `MainView`'s orbit and the snapshot's
  `main_orbit`; the protocol goes from 16 to 17 (an older client is refused at Hello, as for every protocol change).
- `sc-core::combat`: `Command::MainView { orbit }`, `Bridge::main_orbit`. `sc-core::exterior`: `viewscreen.chase`'s
  `follow_tau_s`, `blend_s` and `orbit` block, validated.
- `sc-client`: `console::rig` (the damper, the blend, the orbit), the seat's ORBIT and its drag, wheel, pinch and key,
  the feed and the bracket from one camera, the bridge's main viewscreen's own rig, `touch::ConsoleFinger` pinch.
- `docs/mockups/consoles.html`: the ORBIT button (new UI, so the mockup first), its icon in `data/ui/console_style.json`.
- Pi 5 budget: no new geometry or draw calls (the hull is drawn while CHASE is on, as now); a few hundred floating
  point operations a frame; three bytes a snapshot and a command at most 10 times a second while the captain drags.
