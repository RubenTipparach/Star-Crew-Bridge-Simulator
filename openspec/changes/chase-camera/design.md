# Design: the chase camera, and the captain's camera on the main viewscreen

The owner's words are in the proposal.

| Decided elsewhere | Where |
| --- | --- |
| The viewscreen's six cameras, TARGET FWD CHASE PORT AFT STBD, and their buttons | `console-parity` (the approved `consoles.html`), `combat::FEEDS` |
| The captain holds the viewscreen: his camera on every console | `bridge-stations`, `console-parity` 10.2 |
| The Tern's hull, compiled with the deck and drawn in the engine | `tern-exterior` 4 |

## 1. CHASE, a toggle

The first build made CHASE a sixth camera that stood behind the ship and could be swung round it by dragging the
camera widget. The owner, on its frames: "that chase camera is off to the side of the ship. chase should be a toggle,
not a selection". So CHASE is a toggle on the camera chosen, and there is no swing: the "different angles" are the
cameras themselves.

- **The cameras choose, CHASE toggles.** TARGET, FWD, PORT, AFT and STBD choose where the viewscreen looks, as
  before. CHASE turns the chase camera on or off for whichever is chosen, and its button is lit while it is on.
- **Behind the ship.** With chase on, the camera stands `distance_m` (160 m; the Tern is about 85 m long) back from
  the ship's centre along the line it looks (TARGET: the line from the target through the ship).
- **The ship low and centred.** Raised, square to its look, until the ship's centre sits `ship_low` (0.45) of the half
  height below the middle of the picture, the owner's "y offset towards bottom of the screen", and in the middle
  across. Looking at a near target turns the camera down as it rises, which lifts the ship back up the picture (the
  first video's main-screen shot, the Hound close abeam), so the lift is settled on the camera's final aim, a few
  small steps, and a test holds the ship low for targets nearer than the chase distance.
- **On the target.** TARGET with chase on looks at the target from where it stands, so the target is in the middle,
  "same as first person camera". The others look their own way. With no target, TARGET looks along the bow.
- **A wider lens.** `fov_across_deg` (100 degrees across at zoom 1, against the bow cameras' 70), so the ship fits.

One function computes every viewscreen's camera (`console::feed_view`): each console's and the bridge's main one.
The numbers live in `data/space/exterior.json` (`viewscreen.chase`) and are validated on load. The feed draws the
Tern's hull from where the camera stands, after the sky and the Hound. The console's bracket on the target keeps its
sum: TARGET with chase on looks at the same target from 160 m behind a target kilometres away.

## 2. The main viewscreen

`Command::MainView { feed, chase }`, the captain's: the camera on the bridge's main viewscreen and its chase toggle
(`Bridge::main_view`, FWD to start, and `main_chase`), sent whenever he presses a camera or CHASE on his console. The
bridge's viewscreens show the held camera while he holds the viewscreen (as before), `main_view` otherwise, chasing
when `main_chase` is on. The snapshot carries both; the protocol goes to 13.

## 3. The video

`sc-client --headless --solo --station captain --film --shots DIR` plays a single-player round seated as Captain and,
writes the walk to the captain's chair and the countdown (sped up), then from 2 s into Engage every frame at 30 a
second: FWD with chase off and on, TARGET, PORT, AFT and STBD with chase on, TARGET with it off, and the bridge's main
viewscreen showing the captain's TARGET with chase on. `tools/video/film.sh DIR OUT.mp4` captions each shot and encodes it at
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
| C1 | CHASE a sixth camera with a swing, or a toggle | A toggle on the camera chosen | Decided by the owner, 2026-10-14: "chase should be a toggle, not a selection" |
| C2 | Whether the captain's camera choice also goes to every console without holding the viewscreen | No: only the bridge's main viewscreen; holding still pushes it to every console | Recommendation taken (ask only with screenshots) |
| C3 | The ship low on every chase view, or only TARGET's | Every chase view: the same framing whatever the camera | Recommendation taken (ask only with screenshots); the screenshots show each |
