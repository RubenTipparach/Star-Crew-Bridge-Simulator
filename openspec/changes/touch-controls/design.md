# Design: touch-controls

The proposal says why. This says how, with the numbers. Nothing here is built in the engine yet.

The mockup is `docs/mockups/touch-controls.html`, and it is played, not looked at: deck A of the Tern (the command
deck's rooms) walked in first person with this design's touch layer, on a phone or, on a desktop, with the mouse as a
finger. The stick appears under the left thumb and runs at its rim, a drag on the right looks, USE names what is in
reach and sits the body at a bridge seat, the station's console (`consoles.html`, embedded) rises from the desk and
takes taps and long presses, and STAND held stands the body up. MAP and MENU sit in the top corners; the menu holds
the lighting states. The body is `lib/shipwalk.js`'s, the one walk every mockup uses, driven through its `external`
input. Its shots are `docs/screenshots/mockups/touch-controls-*.png`.

## 1. What a phone player does

The same game as the desktop, in the same order: the front menu, Beam aboard, the body on the Tern, the walk to the
station, sitting down, the console, standing up, the walk to the next job (CLAUDE.md, the law). Only the hands
change: thumbs on glass instead of keys and a mouse.

Held sideways only. A phone held upright is about 390 x 844 CSS px: the 16:9 view would be a strip a third of the
screen tall, and a console would be scaled to 0.3. Upright, the page shows a card asking for the phone to be turned
(the mockup's `upright` shot) and the game keeps running behind it.

## 2. Fingers in the platform layer

- `platform.rs` handles `SDL_EVENT_FINGER_DOWN`, `_MOTION`, `_UP` and `_CANCELED` into
  `Event::Touch { id, phase, x, y }`, `x` and `y` in window points (SDL gives 0-1 of the window; the layer
  multiplies by the window's size so the hit tests match the mouse's).
- `SDL_HINT_TOUCH_MOUSE_EVENTS` is set to `0` once the first finger lands, so a tap is not also a click. A mouse
  used afterwards (a tablet with a trackpad) turns the touch layer off again until the next finger: the layer is
  shown only while touch is the last input used, the same rule as "every input device drives every menu"
  (CLAUDE.md section 10).
- Up to 4 fingers are tracked by id, in a fixed array (no allocation per frame). A fifth is ignored.

## 3. On foot: the touch layer

Sizes are in CSS px at a phone's 844 x 390 landscape and scale with the shorter side of the window (390 px is 1.0).
The layer's zones, left to right:

| Zone | Where | Does | Same as |
|---|---|---|---|
| Move stick | Left 40 % of the screen below the top 15 % | Appears where the thumb lands: a 120 px ring, a 48 px knob. Knob offset over 60 px is the walk vector; a dead zone of 8 px. Held at the rim (over 54 px) for 0.3 s is a run, and the ring turns amber. | W A S D, Shift |
| Look | Right 60 %, anywhere a button is not | A drag turns the view: 0.006 rad per px, scaled by the options' look sensitivity. No pointer lock. | Mouse motion |
| USE | Bottom right, 88 px circle, 24 px from the edges | A tap is E. Held, it is E held: standing up takes the seats' own `stand_hold_s` (`data/stations.json`), the same hold as the key. Its label is the verb of what is in reach ("Sit", "Repair", "Take Captain"), the same text the on-foot prompt already shows; with nothing in reach it is dim. | E |
| DOWN | Left of USE, 60 px | Q: go down a ladder, a lift's down. Shown only when it does something. | Q |
| JUMP | Above USE, 60 px | Space. | Space |
| MAP | Top left, 48 px | M: the ship map; a drag turns it, a pinch zooms it, a tap on a deck's tab is Tab. | M, wheel, Tab |
| MENU | Top right, 48 px | Escape: the game menu. | Esc |

- **Everything goes through the code the keyboard uses.** The layer turns fingers into the same `held` and
  `pressed` key sets and the same look deltas `main.rs` and `drill.rs` already read (CLAUDE.md 6.1). The walk speed
  of a half-pushed stick comes from the body's own speed curve (`Body` in `onfoot`), scaled by the stick's length;
  nothing about walking is tuned here except the stick's feel, which goes in `data/input/touch.json`
  (`ring_px`, `dead_px`, `run_hold_s`, `look_rad_px`), validated like every data file.
- **The prompt stays where it is.** The on-foot prompt in the view ("E  Take Captain from Lt. Sa'ral") drops its key
  letter on touch and the USE button carries the verb. Nothing new is written on the screen.
- **Lift and ladder**: up is USE, down is DOWN, as the keys are today.
- **Fire, carrying crew, repairs on deck**: every on-foot verb is E today, so USE covers them; spraying an
  extinguisher is USE held, as E held is.
- **Flying the free camera (F)** is a developer key and gets no button.

- **Nothing else floats over the game** (owner, 2026-10-13, on the mockup: "way too many popups to be usefule"): no
  help panel, cost meter or title over the view. The help, the lighting states and the cost live in the game menu.

## 4. Seated: the console by finger

- **The console fills the screen, on black, and the room is not drawn behind it** (owner, 2026-10-13: "when dashboard
  is full screen, room shouldnt be rendering", "background of dashboard should be balck to save on rendering perf").
  The room is drawn while the body sits down and stands up (sit_s and stand_s, 0.4 s each), so the move is seen; once
  the console is up the frame draws only the console and its black bars, and the 3D pass is skipped until STAND.
- **A tap is a click**, at the same point, through the same `Hit` test as the mouse (`seat.rs`). A drag is a mouse
  drag (faders, the shield dial, the plan view).
- **Hit areas grow, drawings do not.** On touch, each control's hit rectangle is inflated to at least 44 CSS px on
  its shorter side (Apple's and Google's minimum touch target), clipped where two would overlap so the nearer centre
  wins. The console looks exactly like its approved mockup (CLAUDE.md section 10, `console-parity`); only what a
  finger can hit changes.
- **Long press is hover.** Held 0.5 s without moving more than 10 px, a control shows the tooltip the mouse shows on
  hover, and lifting the finger then does not press it.
- **STAND** is a 48 px button in the black bar beside the console where the screen's aspect leaves room (a phone is
  wider than 16:9), and over the title band's left end where it does not. It is
  hold-E. This is a control the console mockups lack, so it goes into `docs/mockups/consoles.html` for approval
  with this change.
- **Repair games** (`sc-repairs`) already take a pointer with `TOUCH_R` = 30 px reach; a finger is their pointer.
- **Text** (crew chat, a lobby name): SDL's text input on Emscripten raises the phone's keyboard; the chat box sits
  above it. To be checked on a real phone (task 4.3).

## 5. The page

`web/index.html` on a touch screen (`(pointer: coarse)` and no fine pointer):

- `<meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover">`, and
  `touch-action: none` with `overscroll-behavior: none` on the canvas, so a drag never scrolls or zooms the page.
- The key strip under the canvas is hidden; the canvas fills the screen inside the safe-area insets (a notch).
- The first tap asks for full screen (`requestFullscreen`) where the browser has it (Android Chrome, iPadOS
  Safari). iPhone Safari has no element full screen; there the page fills the window and the address bar collapses
  in landscape.
- Upright, the turn-sideways card. `screen.orientation.lock("landscape")` is tried once in full screen; it is
  refused on most browsers and the card covers that.
- The drawing buffer keeps its cap of 2x device pixels. A phone's 3x display draws at 2x.

## 6. itch.io

On the itch.io project page (Edit game, then Embed options) the owner ticks **Mobile friendly** and picks
**Landscape**, and keeps the **Fullscreen button**. Without the first, itch.io shows phones a "not available on
mobile" notice in place of the game. This is a dashboard setting, not a file the release can push.

## 7. How it is proven

- **Unit tests** in `sc-client`, read as sentences: `a_thumb_at_the_ring_rim_walks_at_full_speed`,
  `a_thumb_held_at_the_rim_runs`, `a_tap_on_use_is_one_press_of_e`, `use_held_stands_the_player_up`,
  `a_finger_on_the_look_side_turns_the_view_not_the_body`, `a_tap_is_not_also_a_click`,
  `a_long_press_shows_the_tooltip_and_presses_nothing`, `a_tap_near_a_small_control_hits_it`.
- **The release's smoke test** (`tools/web/smoke.mjs`) adds a touch run: Chromium with `hasTouch` and `isMobile`
  at 844 x 390, touch events only (no keyboard, no mouse), Beam aboard, the stick walks the body to Helm, USE sits,
  a tap sets a throttle, USE held stands up. A step that fails fails the release. Captures go in
  `docs/screenshots/web/touch/`, walking included (CLAUDE.md section 12).
- **A real phone**, Android and iPhone, when the owner has one at hand: emulated touch in Chromium is not Safari.
  Until then the PR says so.

## 8. Pi 5 budget

- On the Pi 5 client nothing changes unless a touch screen is attached: the layer is hidden until a finger lands.
- With a touch screen (the official Pi display), the on-foot layer is 2 UI draw calls (rings and buttons in one
  vertex buffer, their labels in the text batch), under 800 triangles (the mockup counts 768 with the stick and a look drag), against the table's 300 draw calls and its
  2 ms of UI CPU (`engine-stack` design, "The Pi 5 budget"). No memory beyond a fixed 4-finger array and the layer's
  vertex buffer, under 16 KB. No network traffic: the layer makes the same intents the keyboard does.
- Seated, the 3D pass is skipped (section 4): the frame is the console's UI draw calls alone, which gives back the
  room's triangles and draws for as long as a player sits, on the Pi and on a phone alike.
- A phone's frame rate says nothing about a Pi (CLAUDE.md section 2).

## 8a. As built (2026-10-14)

- **The layer appears at the first finger.** The engine cannot ask the page whether the screen is a touch screen,
  so the layer shows from the first touch and hides at the next key or mouse use.
- **MAP is the drill's overview** of the bridge (its V key): the drill has no deck map yet. The map's pinch and deck
  tabs come with one.
- **DOWN shows in a lift's car**, where the prompt names Q. A ladder has no prompt yet, so DOWN does not show at one;
  the keyboard's Q still works there.
- **The console's finger** presses the nearest control within 22 px (`console.reach_px`) of where it lands, at the
  control's centre; a press that moves more than 10 px, or is held 0.5 s, presses nothing, and the held one shows the
  control's tip at the finger. A drag, a hold or a tap-to-drag control (the schematic's) takes the finger at once,
  exactly as the mouse.
- **Seated, nothing of the room is drawn** once the console is up: the console and its viewscreen's feed on black,
  as the drill already drew it; the bridge is drawn only while the console rises.
- **The client says what the hands did** in its log: `touch layer on`, `prompt: ...` when the on-foot prompt
  changes, `sat at <station>` and `stood up`. The smoke test steers the walk by them.
- **A fix the touch layer surfaced:** the on-foot prompt offered "Sit at" a seat an NPC was still walking to, which
  the server refuses as taken; with the keyboard that was a silent E, but USE lit up with a promise it could not keep.
  The prompt now follows the server's claim rule (`seat_prompt` in `drill.rs`, tested): a seat someone walks to says
  who is on the way, and USE stays dark until they sit.
- **Not on touch yet:** the front menu's TOUR (its menus tap, its walk does not) and the phone's keyboard for text
  (task 4.3).

## 9. Not in this change

- Gamepads (the desktop has none yet either; `bridge-stations` 6 names the pad's sticks).
- Gyro look. A drag is the look for now.
- Re-laying out consoles for small screens. A console is its approved mockup at any size; a phone gets bigger hit
  areas and long press, not a second console.

## 10. Recommendations taken (ask only with screenshots)

- Landscape only, upright shows a card. Recommendation taken; the mockup's `upright` shot shows it.
- A floating stick (it appears under the thumb) over a fixed one: thumbs land in different places on different
  phones. Recommendation taken; the owner can try it in the mockup and say otherwise.
