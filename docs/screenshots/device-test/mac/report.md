# Device test: MacBook (primary test machine), 2026-10-09

Asked for by the cloud coordinator on the owner's order: build, test and run the game windowed.
Frame time was not measured (CLAUDE.md section 12: desktop numbers are never the measure).

## Machine

- Mac16,5, Apple M4 Max, 64 GB, macOS 15.6.1 (24G90)
- rustc 1.97.1, cargo 1.97.1, Xcode command-line tools 16.4 (clang 17), cmake 4.2.3, Node 25.1.0
- GL as reported by the client: `4.1 Metal - 89.4 on Apple M4 Max (cocoa)`

## Commit

`d2614d6` on `main` ("Merge pull request #4"). No source file was changed.

## Build: fails as committed, passes with a link flag

`cargo build --release` fails to link `sc-client` and `sc-probe`:

```
Undefined symbols for architecture arm64:
  "___isPlatformVersionAtLeast", referenced from:
    -[SDL3Cocoa_WindowListener mouseMoved:] in libsdl3_sys-....rlib(SDL_cocoawindow.m.o)
```

SDL's Objective-C uses `@available`, which calls `__isPlatformVersionAtLeast` from clang's
runtime (`libclang_rt.osx.a`). rustc links with `-nodefaultlibs`, so that library is not linked.
The linker also warns that SDL and sokol objects were built for macOS 15.5 while the link
targets 11.0. Setting `MACOSX_DEPLOYMENT_TARGET=15.5` alone did not fix it.

Workaround used for every step below (environment only):

```sh
RUSTFLAGS="-C link-arg=$(clang -print-resource-dir)/lib/darwin/libclang_rt.osx.a" cargo build --release
```

With it the build passes; the clean build with SDL from source took about 46 s plus 42 s for the
relink with the new flags. A lasting fix (for example linking `clang_rt.osx` from `sdl3-sys`'s or
`sc-client`'s build script on macOS, or a `.cargo/config.toml` entry) is left to a write-up.

## Tests: `cargo test --workspace --release --no-fail-fast`

| Target | Result |
|---|---|
| sc-core unit tests | 49 passed |
| sc-tools unit tests | 3 passed |
| sc-client, sc-net, sc-probe, sc-render, sc-server unit tests | 0 tests |
| sc-client `tests/render.rs` (headless) | **FAILED**: `SDL_CreateWindow: Could not initialize OpenGL / GLES library` |

The render test runs on SDL's `offscreen` video driver, which needs EGL and OpenGL ES (Mesa's
llvmpipe in a cloud session). macOS has neither, so this test cannot run here as written. It is
an environment limit, not a rendering fault: the windowed runs below render on Metal's GL 4.1.

## Ship file

`npm install -g playwright`, `npx playwright install chromium`, then:

```
export: build/deck/tern (39 rooms, 522051 vertices, 102.8 MB)
deckc: ./compiled/tern.deck: 52 compartments, 175611 triangles, 219671 vertices (6.2 MB),
  167 texture layers of 256 px with 9 mips (58.4 MB), a walk world of 93757 triangles, 70.0 MB in all
```

## Run a: `sc-client --window --walk-test captures/mac-walk` (exit 0, 14 s)

```
sc-client: 4.1 Metal - 89.4 on Apple M4 Max (cocoa)
sc-client: compiled/tern.deck with 45 compartments, 175611 triangles, 167 texture layers
sc-client: walk world of 93641 triangles built in 17 ms; 6 ladders, 3 hatches, 58 doors
sc-client: walk grid of 7884 walkable cells on 3 decks built in 30 ms
sc-client: 9 bot crew aboard
sc-client: drawing 175611 triangles a frame, 64 draws (no culling yet)
sc-client: wrote 10 captures (walk-1..5, lift-1..5)
sc-client: walk test: arrived at [0.0035684546, -3.5, 6.0147214], 0.02 m from the route's end
```

Captures: `walk/`.

## Run b: `sc-client --window --shots captures/mac-shots` (exit 0, 12 s)

Same start-up lines as run a, then 24 captures: 8 views (bridge forward, captain, helm chairs,
viewscreen, port window, corridor B, engineering mezzanine, engineering lower) in the normal,
red alert and emergency states. Captures: `shots/`.

## Anything wrong

- The build does not link on macOS without the flag above.
- The headless render test cannot run on macOS.
- deckc reports 52 compartments; the client reports 45 when it loads the same file. Possibly two
  different counts (for example compartments with geometry), not checked.
- The captures looked right where spot-checked (`shots/bridge-forward-normal.png`,
  `walk/walk-5-deck-C-corridor.png`).
