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

- **CHASE is a toggle** (design 1; the owner on the first video's frames, 2026-10-14: "that chase camera is off to
  the side of the ship. chase should be a toggle, not a selection"): with it on, whichever camera is chosen (TARGET,
  FWD, PORT, AFT, STBD) is seen from behind the ship, the Tern in the middle across and low on the screen. TARGET with
  chase on looks past the ship at the target, the target in the middle: "focus on target, same as first person
  camera".
- **The main viewscreen is the captain's** (design 2): the bridge's big viewscreen shows whatever camera is on the
  captain's console. Holding the viewscreen still puts his camera on every station's console as well.
- **A video tool** (design 3): `sc-client --film` writes the frames of a scripted run through the cameras, and
  `tools/video/film.sh` makes the captioned MP4 the owner approves.

## Impact

- `sc-core::combat`: `Command::MainView { feed, chase }`, `Bridge::main_view` and `main_chase`. `sc-net`: the command and the snapshot field;
  the protocol goes from 12 to 13.
- `sc-core::exterior`: `viewscreen.chase` in `data/space/exterior.json`.
- `sc-client`: the chase camera (`console::feed_view`, `console::chase_camera`), the seat's chase toggle, the feed
  drawing the hull, the bridge's viewscreen reading `main_view` and `main_chase`.
- No console layout changes: CHASE's button was already in the approved mockup; it now lights while chase is on
  instead of being one of the six choices.
