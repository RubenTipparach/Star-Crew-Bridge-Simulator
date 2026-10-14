# Proposal: the chase camera, and the captain's camera on the main viewscreen

## Why

The owner, 2026-10-14: "One change is like you to make is the ability to toggle camera to chase mode on the station
dash board. I'm gonna reiterate. Main viewing screen is whatever is on the captains main viewer in his dashboard.
Chase camera can be from different angles along as focus on target, same as firat person camera. Secondly, the ship
should have a y offset towards bottom of the screen when target camera is selected. Make these changes render a video
of these changes so I can approve them".

Every console's viewscreen already has a CHASE button (the approved `consoles.html`), but it drew what FWD draws: the
camera on the bow, the Tern never in the picture. And the bridge's big viewscreen showed the captain's camera only
while he "held" the viewscreen; otherwise it showed the bow.

## What Changes

- **CHASE is a chase camera** (design 1): outside the ship, behind it on the line from the target through the ship,
  looking at the target, the Tern in the picture and low on the screen. The player swings it round the ship by
  dragging the camera widget while CHASE is on; wherever it stands, the target stays in the middle.
- **The main viewscreen is the captain's** (design 2): the bridge's big viewscreen shows whatever camera is on the
  captain's console. Holding the viewscreen still puts his camera on every station's console as well.
- **A video tool** (design 3): `sc-client --film` writes the frames of a scripted run through the cameras, and
  `tools/video/film.sh` makes the captioned MP4 the owner approves.

## Impact

- `sc-core::combat`: `Command::MainView`, `Bridge::main_view`. `sc-net`: the command and the snapshot field;
  the protocol goes from 12 to 13.
- `sc-core::exterior`: `viewscreen.chase` in `data/space/exterior.json`.
- `sc-client`: the chase camera (`console::chase_camera`), the seat's chase angles and drag, the feed drawing the hull,
  the bridge's viewscreen reading `main_view`.
- No console layout changes: CHASE's button was already in the approved mockup.
