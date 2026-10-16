# Design: the browser page is a game screen

## Context

The owner's words are in the proposal. Two more, the same day, set the rule of design 5: "another issue is that
buttons go off screen, remember, buttons and text need to scale with reference pixels at 1920x1080", and "that way we
dont need to worry about the actual pixel scale being 1-1 all the time, if we stick with virtual pixels as ref".

| Decided elsewhere | Where |
| --- | --- |
| The browser build: Emscripten, WebGL 2, SDL3's emscripten driver, the page `web/index.html`, deployed to itch.io on every merge | `engine-stack` 10a, 10b |
| The browser hosts single player; `tools/web/smoke.mjs` plays into a round before every push | `browser-single-player` |
| A touch screen: the page fills the screen, the touch layer is drawn by the engine | `touch-controls` 5 |
| Options and its rows | `front-menu` 4, `game-menu` 3 |
| Consoles are drawn on a 1280 x 720 canvas of layout points, the mockups' canvas | `bridge-stations` 8.0, `console-parity` |
| A new screen needs an approved mockup first | CLAUDE.md 10 |

## 1. Who owns the canvas

What happened on itch.io (measured in SDL 3.4.18's emscripten driver, `src/video/emscripten/`):

1. The page's CSS sizes the canvas (`width: 100%; height: 100%`), so when SDL makes the window it finds an external
   size and takes the canvas's CSS size as the window's size (`Emscripten_CreateWindow`, `external_size`).
2. `platform::open` made the window with neither `SDL_WINDOW_RESIZABLE` nor `SDL_WINDOW_HIGH_PIXEL_DENSITY`. SDL's
   resize handler (`Emscripten_HandleResize`) follows a resize only for a window that is resizable, full screen or
   filling the document; this one was none of them, so SDL's window size stayed what it was at the start.
3. The page's `fit()` set the canvas's drawing buffer to its CSS size times the device pixel ratio (capped at 2), on
   load and on every window resize.

So after the page was resized, or itch's full screen button pressed, the drawing buffer had the new size and SDL's
window the old one. The frame's size (`SDL_GetWindowSizeInPixels`, the window's size times SDL's pixel ratio, 1) and
the UI's points came from the old size: the engine drew a frame of the old size into the bottom left of the new
buffer, so the top and the right were cut off (the briefing's Laser stbd and Gunner off the screen). SDL scales the
mouse by its window's width over the canvas's CSS width, the old size over the new, so the pointer landed off from
where it was drawn. With a device pixel ratio above 1, `fit()` made the buffer bigger than SDL's window from the first
frame, so a laptop at 150 % or a phone was broken before any resize.

The fix: one owner. In the browser the window is opened `RESIZABLE | HIGH_PIXEL_DENSITY`. SDL then follows every
resize (it reads the canvas's CSS size and sets the drawing buffer to it times the pixel ratio), takes full screen
through the same handler, and its window size, pixel size and mouse scale agree. The page's `fit()` goes: SDL alone
sets the drawing buffer. The desktop and the Pi open the window as they do today.

The pixel ratio: SDL uses the device's whole ratio, where `fit()` capped it at 2. A phone at 3 would draw 2.25 times
the pixels it drew before, for a picture the eye cannot tell apart. So the page keeps the cap, in one place: before
the engine starts it caps the `devicePixelRatio` SDL reads at 2. That keeps the buffer the size `fit()` made it, so
the change spends nothing new; nothing is measured because nothing changed.

The frame follows: `platform::app_iterate` reads the window's size and its size in pixels every frame and hands both
to the app (`Frame.width`, `height`, `window_w`, `window_h`), and the UI's mapping (design 5) is made from them every
frame, so a resize is in the next frame's layout and mouse.

## 2. The page is only the game

The key strip under the canvas goes, with its "same engine as the Pi 5 client" note: the canvas fills the page edge to
edge, with no border, padding or margin, so itch's frame shows only the game. Itch draws its own full screen button
over the frame's bottom right corner; with the strip gone that corner is the game's, and nothing the player must press
sits there (design 4 lists what does).

The keys move to Options, as a square panel (the owner: "it hsould be in optiopns menu as a square panel"). That
panel is a new screen, so its mockup comes first (CLAUDE.md 10); the mockup is being made separately, and the panel is
built once the owner approves it. Until then the page shows no keys; the in-game prompts ("E  Sit at Helm") stay,
because they are the game telling the player what E does where they stand.

## 3. The release proves it

`tools/web/smoke.mjs --fit` (run by `tools/web/release.sh` with the other two runs) opens the page at each size of
design 5's table, at a device pixel ratio of 1 and of 2, and enters full screen once. At each:

1. it waits for the engine to say its window's size (`sc-client: window W x H, PW x PH pixels`, said on each
   change), and fails unless the window's size is the canvas's CSS size and its pixel size the drawing buffer's (the
   frame follows the page);
2. it reads where the engine drew SINGLE PLAYER (`sc-client: ui <label> at x y w h`, window coordinates, said
   whenever a marked control moves), fails if any of it is off the page, clicks its centre with the mouse, and fails
   unless the press lands (`sc-client: ui clicked <label>`); then BACK the same way;
3. in full screen it clicks SINGLE PLAYER and BEAM ABOARD and boards the game; at itch's embed size and a pixel ratio
   of 2 it opens the game menu with Esc and clicks Options, Back and Resume the same way.

The controls a player presses mark themselves (`ui::mark`): the front menu's buttons, BACK, BEAM ABOARD and TOUR, the
game menu's rows, and the briefing's station buttons. The click is placed by what the engine says it drew, never by a
hand-typed position, so the test fails exactly when drawing and hit testing disagree with the page.

## 4. Single player, judged as a released game

The owner: "you gotta get out of prototype mindset, and think shipped product". Anything that leaves a player stuck,
confused or looking at a broken frame is a bug to fix: a click that misses, a screen with no way back, no feedback
after a press, a cut-off panel, a frame that never stops loading, a console error. Every flow has a way out: Esc, and
the game menu's way back to the front menu. No dev or debug text reaches the page.

The pass: headless Chromium through Playwright, as the smoke test does, at 1280 x 720 and 1920 x 1080, at a device
pixel ratio of 1 and of 2, and in full screen: the front menu, Single Player, Beam aboard, the briefing, picking
stations including Captain, walking to a station, sitting, using its console, standing up. Every shot is looked at.
The findings and what was done with each are in section 6; the shots are in `docs/screenshots/web/fit/`.

## 5. One reference frame for every screen

The rule (owner, above): every screen is laid out on a virtual reference frame of 1920 x 1080 reference pixels, and
one mapping, the same for every screen, scales that frame to the real output: the scale is
`min(width / 1920, height / 1080)`, so the whole frame always fits, centred. At any other aspect the extra space is at
the edges: what is anchored to an edge (a top band, a bottom bar, a corner button) sticks to that edge, and what is
laid out on the frame stays inside it. Nothing is ever cut off, text and buttons scale with the frame, and nothing in
a screen's code knows the device's pixels, its pixel ratio or the window's real size. The same mapping, inverted,
turns the mouse's and a finger's window coordinates into the frame's coordinates, so drawing and hit testing agree at
any size and any pixel density. (Unity calls this a Canvas Scaler on "scale with screen size", matching width or
height, whichever is smaller.)

What the code did: `ui::points_of` scaled by the height alone (a screen 720 points high), so a screen narrower than
16:9, or the short itch embed, put the right of a wide panel off the screen. The same rule was written again in
`deckrepair.rs`, `repairs.rs` and the console's own `console::fit`, and the briefing placed itself with its own
`CANVAS` origin, centred across but pinned to the top. Five copies of one rule.

**The units.** The layouts keep their numbers. Every console, the briefing and the repair games are drawn in layout
points on the 1280 x 720 canvas the mockups draw and the parity fixtures compare, and 1280 x 720 is the 1920 x 1080
reference frame at exactly two thirds on both axes. A layout point is 1.5 reference pixels, so converting once in the
mapping puts every pixel exactly where rewriting every number by 1.5 would, without touching thousands of layout
numbers, every mockup and every parity fixture for no change on the screen. `data/ui/reference.json` holds both
sizes (`reference_px` 1920 x 1080, `layout_pt` 1280 x 720), and the loader refuses a file whose two sizes are not one
shape.

**The one function.** `ui::frame_in(screen)` fits the reference frame into a screen and centres it: its origin and its
scale. `ui::Fit::of(frame)` applies it to the output's pixels: the pixels a layout point (the scale), the screen's
size in layout points (always at least 1280 x 720), the reference frame's rectangle on it, and the window to layout
point conversion for the mouse and the fingers. `Ui::run` makes a `Fit` every frame and every screen asks it:

| Screen | Before | After |
| --- | --- | --- |
| Front menu, Options, game menu, debrief (egui areas) | 720 points high, width from the aspect | the fitted screen: anchored areas stay on their edge or centre, and the screen is never narrower than the frame |
| The briefing and the bridge band | `CANVAS` origin, centred across, pinned to the top | the reference frame's rectangle |
| Consoles | `console::fit`, a second copy | `ui::frame_in` |
| Repairs menu, deck repair | their own height rule and window conversion | `Fit` |
| Touch layer | window coordinates, scaled by the screen's shorter side over 390 | layout points through `Fit`; its sizes are a 390 px high reference frame's |

A test (`ui::no_screen_reads_the_output_size`) reads every UI module's source and fails if one reads the window's size
or makes a scale from the frame's pixels outside `ui.rs` (the renderer's present calls, captures and the 3D camera's
aspect are not layout and are allowed by name). Tests at each output of the table below check that every button and
text the front menu, the briefing and the bridge band draw lies inside the screen, and that a pointer at a button's
centre in window coordinates lands on it.

| Output | Why |
| --- | --- |
| 1920 x 1080 | The reference |
| 1280 x 720 | The Pi's screen |
| 1366 x 768 | The commonest laptop |
| 2560 x 1440 | 1440p |
| 1024 x 768 | 4:3 |
| 2560 x 1080 | Ultrawide |
| 985 x 455 | itch's embed |
| 844 x 390 | A phone held sideways |

Console parity: the consoles draw on the same 1280 x 720 canvas at the same scale as before at 16:9 (the fixtures are
1280 x 720), so the parity check's numbers do not move.

## 6. Findings

The pass of design 4, 2026-10-15, on the Pi 5 in headless Chromium (SwiftShader, and the Pi's own GPU through ANGLE
for the walk), before and after this change. The captures are in `docs/screenshots/web/fit/`.

Blockers, each fixed with its test:

| # | What the player met | Fix | Test |
| --- | --- | --- | --- |
| 1 | After a resize, itch.io's full screen, or at a pixel ratio above 1, the frame was drawn at its old size in the bottom left (the top and right cut off) and the mouse landed off where it pointed | Design 1 | `smoke.mjs --fit`; `ui::a_click_lands_where_it_was_drawn_at_any_pixel_ratio` |
| 2 | On a screen narrower than 16:9 or a short embed, wide panels ran off the right edge | Design 5 | `ui::the_whole_frame_fits_every_output_centred`, the briefing and band tests at nine outputs, `touch::every_button_is_whole_on_every_screen_and_found_where_drawn` |
| 3 | The page showed a key strip, a note about the Pi and a loading line about Rust, sokol and a 9 MB download (the deck is 19 MB) | Design 2: the page is only the canvas; the loading line says Loading | `smoke.mjs --fit` sees only the canvas |
| 4 | Exit in the browser's game menu stopped the engine: a frozen frame and no way back | No Exit in the browser; Return to menu stays | `game_menu::the_browser_has_no_exit_to_leave_a_frozen_page` |
| 5 | The browser's front menu showed QUIT greyed out, a button that does nothing | Not shown in the browser | the front menu at every size in `smoke.mjs --fit` |
| 6 | Options' Display read "Full screen" in a window and did nothing when pressed | Not shown in the browser (the page's own full screen does it) | `options::the_browser_shows_no_display_choice_that_does_nothing` |
| 7 | Coming back from Options the game menu's rows were drawn a frame off centre (egui places an anchored window by its size of the frame before), so a click there missed | Fixed-size panels are placed by their size (`ui::centred`) | `game_menu::the_rows_are_where_they_settle_on_the_first_frame_back_from_options` |
| 8 | Single player on foot: the round began while the player was still walking from their muster place, and a player who sat before the last bot waited for a READY that is not on screen on foot, so the round never began | On foot the Muster waits for every person to be seated, and sitting in Muster is a person's READY (`coop-drill` 1's Muster row) | `combat::on_foot_the_round_waits_for_the_person_to_sit_and_sitting_is_their_ready` |
| 9 | "I cant press the captain button": a bot's seat was off in the briefing | Fixed by `relieve-bots` (a person takes a bot's seat) | there |

Left for the UI pass (style, and engine strings it is redesigning): the `(NPC)` after every nameplate, `(bot)` in the
briefing's crew list, the `BRIDGE (V)` label, the helm's quaternion readout, the band's `Helm auto` grid, the
`Extinguisher 100%` line, the game menu's veil not dimming the top band, and the F3 network line (off unless pressed).
Left for design: boarding gives no word on where to go (the player stands in a corridor at MUSTER with nothing saying
"your station is on the bridge"); it needs a mockup. The keys' Options panel waits for its mockup (design 2).

The release's smoke runs follow the new Muster: the keys run and the touch run both walk the body up to the bridge
and sit before the round can reach Engage (the keys run once waited for Engage with the player standing in the
corridor), and both walk past the extinguisher locker rather than stopping at its "E  Take an extinguisher".

Not measured: frame time. The headless runs show the browser at 2 to 4 frames a second under SwiftShader and 12 on the
Pi's GPU, which says nothing about a player's machine (CLAUDE.md 2).
