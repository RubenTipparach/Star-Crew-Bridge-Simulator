# Proposal: the browser page is a game screen

## Why

The owner, 2026-10-15, after playing the itch.io build (built from main): "I'm stuck, I cant press the captain button,
going full screen in browser is broken, mouse is stuck offset, its basically unlpayable on itch", "it needs some
serious fixing in single player mode", "I also dont like the controls text in the bottom, makes it look messy",
"input controls can be in the options menu", and, on a shot of the page's key strip: "this text should be remocved
from the main screen and any screen, it hsould be in optiopns menu as a square panel". And: "you gotta get out of
prototype mindset, and think shipped product".

Their shot shows the briefing too big for the canvas: its top and right are cut off, the Laser stbd and Gunner
buttons are off the screen, and a click does not land where the pointer is.

The cause (design 1): the page and SDL both own the canvas's drawing buffer. The page's `fit()` resizes it on every
window resize; SDL, whose window was made neither resizable nor high pixel density, keeps the size it read when it
opened. So after a resize or full screen the engine draws a frame of the old size into the corner of a new buffer,
and SDL scales the mouse by the old size: the cut-off panels and the offset pointer.

## What Changes

- **SDL alone owns the canvas** (design 1): in the browser the window is opened resizable and high pixel density, and
  the page's `fit()` goes. The frame's size and the window's size follow every resize and full screen, so the UI's
  scale and the mouse stay right. The pixel ratio stays capped at 2, as `fit()` capped it.
- **The page is only the game** (design 2): the key strip under the canvas, its "same engine as the Pi 5" note
  included, is removed; the canvas fills the page. The keys move to a square panel in Options once its mockup (being
  made separately) is approved; nothing replaces them before then.
- **The release proves it** (design 3): `tools/web/smoke.mjs` resizes the page and enters full screen at a device
  pixel ratio of 1 and of 2, clicks the briefing's Captain button where it is drawn, and fails if the click misses.
- **Single player judged as a released game** (design 4): a pass through the browser's single player, front menu to
  standing up from a console, and every bug it found that leaves a player stuck, confused or looking at a broken
  frame fixed with its test.

## Impact

- `crates/sc-client/src/platform.rs`: the window's flags in the browser.
- `web/index.html`: no `fit()`, no strip; the pixel ratio cap.
- `tools/web/smoke.mjs`: the resize, full screen and click checks.
- Pi 5: none. The desktop and Pi client open the same window as today; only the browser build changes.
