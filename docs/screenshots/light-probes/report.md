# Device test: light probes on the Mac, 2026-10-10

Asked for by the cloud coordinator on the owner's order (light probes so crew figures take their
room's light). No code was changed. Frame time was not measured.

## Machine

Mac16,5, Apple M4 Max, 64 GB, macOS 15.6.1. GL as the client reports it:
`4.1 Metal - 89.4 on Apple M4 Max (cocoa)`. Commit `b736439` on
`claude/starcrew-bridge-sim-docs-85s176`.

Every cargo step used the macOS link workaround from `docs/screenshots/device-test/mac/report.md`
on `device-test/mac` (the build does not link SDL on macOS without it):
`RUSTFLAGS="-C link-arg=$(clang -print-resource-dir)/lib/darwin/libclang_rt.osx.a"`.

## Deck rebuild: passed, numbers as expected

```
probes: 11118 in 38 rooms, 0 rooms baked here, the rest from the cache
export: build/deck/tern (39 rooms, 522051 vertices, 103.4 MB)
deckc: ./compiled/tern.deck: 52 compartments, 175611 triangles, 219671 vertices (6.2 MB),
  167 texture layers of 256 px with 9 mips (58.4 MB), a walk world of 93757 triangles,
  11118 light probes (9399 valid, 0.61 MB), 70.6 MB in all
```

## scripts/check.sh: stops at step 3 on macOS

| Step | Result |
|---|---|
| 1. cargo fmt | passed |
| 2. cargo clippy, warnings as errors | passed |
| 3. cargo test | **failed** at `sc-client` `tests/render.rs`: `SDL_CreateWindow: Could not initialize OpenGL / GLES library`. Everything else passed (sc-core 56, sc-tools 3) |
| 4. openspec validate | not run: the OpenSpec CLI is not installed on this Mac |
| 5. dash check | ok |
| 6. ship layouts | ok |
| 7. mockups hold the current layout | ok |
| 8. shader modules | ok: 4 shader modules current |
| 9. engine data | ok: 5 engine data files valid |
| 10. lighting data | ok: 10 fixture types, bake settings valid |

Steps 4 to 10 were run one by one after the script stopped. The render test uses SDL's offscreen
driver, which needs EGL and OpenGL ES; macOS has neither. It is the same environment limit as the
first Mac test, not a probe fault.

## Captures

`--headless --shots` fails on macOS for the same reason (`Could not initialize OpenGL / GLES
library`). The same shot set was written windowed instead:
`sc-client --window --shots docs/screenshots/light-probes/`, exit 0, 36 captures (the 8 fixed
views and `bot-0` to `bot-3`, each in normal, red alert and emergency). The client printed
`11118 light probes in 38 rooms (0.61 MB)`.

Step 5 (walking up to bots in the windowed game and pressing F12) was **not done**: it needs a
person at the keyboard, and this session cannot drive the game's input.

## What the bot close-ups show

Mean colour (RGB, 0-255) of each bot's torso and head and of a wall patch beside it, measured from
the PNGs:

| Shot | Torso | Head | Wall |
|---|---|---|---|
| bot-0 normal | 89, 30, 24 | 87, 71, 58 | 26, 27, 26 |
| bot-0 red alert | 96, 9, 7 | 94, 21, 18 | 34, 9, 8 |
| bot-0 emergency | 69, 13, 4 | 68, 29, 9 | 29, 15, 5 |
| bot-1 normal | 92, 34, 28 | 90, 77, 67 | 59, 51, 39 |
| bot-1 red alert | 117, 10, 8 | 113, 23, 20 | 75, 27, 15 |
| bot-1 emergency | 102, 18, 5 | 99, 42, 11 | 62, 30, 8 |
| bot-2 normal | 73, 61, 24 | 66, 61, 58 | 16, 15, 13 |
| bot-2 red alert | 104, 20, 7 | 93, 20, 18 | 20, 4, 4 |
| bot-2 emergency | 90, 35, 4 | 81, 35, 10 | 15, 7, 2 |
| bot-3 normal | 82, 68, 26 | 73, 67, 62 | 20, 18, 16 |
| bot-3 red alert | 113, 21, 8 | 102, 21, 19 | 23, 5, 5 |
| bot-3 emergency | 99, 38, 5 | 89, 38, 11 | 16, 8, 3 |

What works:

- **The tint follows the lighting state.** Bots go red at red alert and amber in emergency, as
  their rooms do. It is not one fixed light everywhere.
- **The room shows.** In normal light the bots in the lamp-lit rooms (bot-0, bot-1) are warmer
  than those in the darker engineering spaces (bot-2, bot-3).

What looks wrong:

- **Bots are too bright for dark rooms.** bot-2 and bot-3 stand in rooms whose walls read about
  15-20 per channel, yet their heads read about 60-70: they look lit by a lamp the room does
  not have. Some of that may be the walls' dark textures, but by eye the figures still stand out
  against the room.
- **Bots dim less than their room in emergency.** At bot-2 the wall's brightness falls to about
  55% of normal; the head's to about 75%. Bot-0's head falls further, but its red channel stays
  close to normal.
- **The red channel rises above normal** at red alert and emergency on every bot (bot-2 torso:
  73 normal, 104 red alert, 90 emergency), so a bot in emergency light can read brighter red than
  the same bot in normal light. Possibly the probe colour is normalised or the alert tint is
  added on top rather than replacing the white light; not checked in code.
