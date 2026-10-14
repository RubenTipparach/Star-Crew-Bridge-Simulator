# Design: the chase camera, and the captain's camera on the main viewscreen

The owner's words are in the proposal.

| Decided elsewhere | Where |
| --- | --- |
| The viewscreen's six cameras, TARGET FWD CHASE PORT AFT STBD, and their buttons | `console-parity` (the approved `consoles.html`), `combat::FEEDS` |
| The captain holds the viewscreen: his camera on every console | `bridge-stations`, `console-parity` 10.2 |
| The Tern's hull, compiled with the deck and drawn in the engine | `tern-exterior` 4 |

## 1. CHASE

Where it stands, in the ship's frame (`console::chase_camera`):

1. **The line.** From the target through the ship's centre (the locked contact; the bow's line with none).
2. **Behind.** `distance_m` (160 m; the Tern is about 85 m long) back along the line from the ship's centre.
3. **The ship low.** Raised, square to its look, until the ship's centre sits `ship_low` (0.45) of the half height
   below the middle of the picture: the owner's "y offset towards bottom of the screen".
4. **Round the ship.** Swung round the ship's centre by the player's angles: up to `yaw_max_deg` (40 degrees) to
   either side, then up to `elevation_max_deg` (8 degrees) up or down. Dragging across the camera widget while CHASE
   is on covers the yaw's whole range, dragging down it the elevation's. Recommendation taken (ask only with
   screenshots): the widget is the control, so no new button is added to the approved console.
5. **On the target.** It looks at the target from where it stands, so the target is in the middle whatever the
   angle, "same as first person camera" (TARGET). With no target it looks along the bow.
6. **A wider lens.** `fov_across_deg` (100 degrees across at zoom 1, against the bow cameras' 70).

Why the angles are held where they are: a camera that keeps the target in the middle and the ship in the picture can
only stand as far off the line as the picture is wide. Swung 96 degrees round, the first build stood beside the ship
and the Tern was behind the camera (the test `chase_keeps_the_target_in_the_middle_and_the_ship_low_from_every_angle`
caught it). The feed is 614 x 194, about three times as wide as it is tall, so the swing is mostly to the sides: with
the wider lens the picture is about 100 degrees across and 41 degrees high, which takes 40 degrees of yaw and 8 of
elevation with the target in the middle and the ship inside the lower half.

The numbers live in `data/space/exterior.json` (`viewscreen.chase`) and are validated on load. The feed draws the
Tern's hull from where the camera stands, after the sky and the Hound, with its own wider field of view. The console's bracket on the target keeps its sum (TARGET's direction): CHASE looks at
the same target, from 160 m behind a target kilometres away, so the bracket sits where the camera puts it.

## 2. The main viewscreen

`Command::MainView(feed)`, the captain's: the camera on the bridge's main viewscreen (`Bridge::main_view`, FWD to
start), sent whenever he presses a camera on his console. The bridge's viewscreens show the held camera while he
holds the viewscreen (as before), `main_view` otherwise. The snapshot carries it; the protocol goes to 13. A CHASE on
the main viewscreen stands at the captain's angles on his own client and behind the ship on everyone else's (the
angles are a console's, not the server's).

## 3. The video

`sc-client --headless --solo --station captain --film --shots DIR` plays a single-player round seated as Captain and,
from 2 s into Engage, writes every frame: FWD, TARGET, CHASE behind, CHASE swung round the ship, then the bridge's
main viewscreen showing the captain's CHASE. `tools/video/film.sh DIR OUT.mp4` captions each shot and encodes it at
30 frames a second. Rendered in a cloud session on llvmpipe at the drill's fixed step: it shows what the cameras
draw, not how smoothly.

## Pi 5 budget

| Cost | Amount |
| --- | --- |
| Rendering | CHASE draws the hull into the feed target (the feed's own size) on top of what the feed draws now: the hull's triangles once more per frame while CHASE is on. Measured on the Pi in task 3.2 |
| CPU, memory | None worth stating: a camera's sums each frame, two bytes more a snapshot |
| Network | One byte a snapshot (`main_view`), one command when the captain changes camera |

## Open questions

| Id | Question | Recommendation | Status |
| --- | --- | --- | --- |
| C1 | How the player picks the chase angle | Drag the existing camera widget while CHASE is on | Recommendation taken (ask only with screenshots); the video shows it |
| C2 | Whether the captain's camera choice also goes to every console without holding the viewscreen | No: only the bridge's main viewscreen; holding still pushes it to every console | Recommendation taken (ask only with screenshots) |
