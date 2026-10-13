# Proposal: touch controls, so the itch.io build plays on a phone or a tablet

## Why

The owner, 2026-10-13: "does the itch io game work with mobile? can you make it work with mobile?"

Today it does not, though it loads. A current phone browser has WebGL 2, so the engine starts, draws the Tern and
shows the front menu, and SDL turns a tap into a mouse click, so the menu's buttons answer. The game stops there:

- **Walking needs a keyboard.** The body moves on W A S D, uses on E, goes down on Q, runs on Shift and jumps on
  Space (`sc-client::main`, `onfoot`). A phone has none of them.
- **Looking needs pointer lock.** The camera turns on relative mouse motion, and only after a click captures the
  mouse (`platform::capture_mouse`). A finger is not a mouse: no pointer is locked, and a drag never turns the view.
- **The client never reads a finger.** `platform.rs` handles keys, mouse, wheel and text; SDL's finger events are
  dropped, so two thumbs at once (walk and look) cannot exist.
- **The page is a desktop page.** It has no viewport tag, keeps the key strip under the canvas, lets a drag scroll
  or zoom the page, and does nothing about a phone held upright.
- **Seated consoles are drawn for a mouse.** On a phone held sideways the 1280 x 720 console is scaled to about
  0.55, so the 24 lp floor of a control (CLAUDE.md section 10) becomes about 13 CSS px, and a value shown on hover
  has no hover to show it.

The law (CLAUDE.md) holds on a phone as everywhere: the player walks to the station, sits, and the console comes
up from the desk. Touch controls must carry the walk, not replace it with a station menu.

## What Changes

- **Fingers reach the client** (design 2): `platform.rs` reads SDL's finger events into `Event::Touch`, and the
  synthesized mouse events are turned off while touch is in use, so a thumb is never counted twice.
- **An on-foot touch layer** (design 3, the mockup): a floating move stick under the left thumb (pushed to its rim
  is a run), a drag anywhere on the right half to look, a large USE button that is E (held, it is hold-E: stand
  up), a DOWN button that is Q, a JUMP button, and MAP and MENU buttons in the top corners. It feeds the same
  intents the keyboard does: no second walk.
- **Seated, the console takes taps** (design 4): a tap is a click, a long press (0.5 s) shows the tooltip that a
  hover shows on the desktop, and the hit area of every control grows to at least 44 CSS px without moving or
  resizing what is drawn. A STAND button replaces holding E.
- **The page fits a phone** (design 5): viewport tag, no page scroll or zoom over the canvas, the key strip hidden
  on a touch screen, full screen on the first tap where the browser allows it, and a "turn the phone sideways" card
  when held upright.
- **itch.io is told** (design 6): the project is marked mobile friendly in landscape. That is a setting on the
  itch.io dashboard, made by the owner.
- **The release proves it** (design 7): `tools/web/smoke.mjs` plays a phone-sized, touch-only run (walk to Helm with
  the stick, sit with USE, tap a console control, stand) and captures it.

## Impact

- `sc-client::platform`: `Event::Touch`, the touch-mouse hint.
- `sc-client`: a new `touch.rs` (the layer: stick, look, buttons, drawn with the existing UI pen); `main.rs` and
  `drill.rs` take its intents beside the keyboard's; `seat.rs` takes the long press and the larger hit areas.
- `web/index.html`: the phone page.
- `tools/web/smoke.mjs`: the touch run.
- `docs/mockups/touch-controls.html`: the mockup, for approval before any engine code.
- Pi 5: nothing unless a touch screen is plugged in; then a few UI draw calls (design 8).
