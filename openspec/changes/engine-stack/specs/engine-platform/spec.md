# Engine Platform

## Purpose

The hardware the game must run on, the renderer's feature floor, the boundaries between the
engine's crates, and the budget every other capability spends against.

## ADDED Requirements

### Requirement: The game runs on a Raspberry Pi 3 within its budget
The release client SHALL run on a Raspberry Pi 3 Model B with 1 GB of RAM, full screen at
1280 x 720 without a desktop session, at or above 30 frames a second on the reference ship's
bridge, within the memory, triangle and draw-call budgets of the Pi 3 budget table, as
measured by `sc-probe` and the frame log on that hardware.

#### Scenario: The bridge at red alert
- **WHEN** a crew of four is on the Tern's bridge at red alert with two enemy ships on the viewscreen, on a Pi 3
- **THEN** the frame time p95 is at most 33.3 ms and the client's resident memory is at most the table's client allocation

#### Scenario: A measurement from a desktop
- **WHEN** a frame time is reported from a desktop GPU, a cloud session or a three.js mockup
- **THEN** it is not accepted as evidence for this requirement

### Requirement: The renderer needs nothing above OpenGL ES 2.0
The renderer SHALL use only OpenGL ES 2.0 core features and GLSL ES 1.00 shaders, and SHALL use
any extension only behind a fallback path that has been measured on a Pi 3.

#### Scenario: Instancing is unavailable
- **WHEN** the GL context offers no instancing extension
- **THEN** every pass still draws correctly, by batching into shared buffers

#### Scenario: The same shaders on a desktop
- **WHEN** the client runs on desktop OpenGL 2.1
- **THEN** it compiles the same shader sources with a define prelude and draws the same frame

### Requirement: The simulation core is engine-independent
Every gameplay rule SHALL live in `sc-core`, which SHALL NOT depend on rendering, windowing,
audio, networking or file I/O, and SHALL be testable with `cargo test` on a machine without a
GPU.

#### Scenario: Testing the core alone
- **WHEN** `cargo test -p sc-core` runs on a headless machine
- **THEN** every core test builds and runs

#### Scenario: A rule needed by the server and a console
- **WHEN** the engineering console previews the power a breaker will deliver
- **THEN** it calls the same `sc-core` function the server uses to deliver it

### Requirement: One budget table
The Pi 3 budget SHALL be defined in exactly one document, and every tool or page that displays
or checks a budget number SHALL be checked against that document's machine-readable marker.

#### Scenario: A drifted mockup meter
- **WHEN** `docs/mockups/lib/shipkit.js` gives a triangle budget different from the table's marker
- **THEN** `python3 tools/mockups/inline.py --check` fails and names both values

### Requirement: Memory is allocated by budget at startup
The client and the server SHALL size their pools at startup from the loaded ship and the budget
table, SHALL refuse to start with a message naming what does not fit when the total exceeds the
allocation, and SHALL NOT allocate on the heap per frame in steady state.

#### Scenario: A ship too large for the budget
- **WHEN** a ship whose compiled decks exceed the vertex buffer budget is loaded
- **THEN** startup stops with a message naming the ship, the budget and the size

#### Scenario: Steady state
- **WHEN** the debug build's counting allocator watches 600 frames on the bridge after loading
- **THEN** it counts no allocations

### Requirement: Every check runs from one script
`scripts/check.sh` SHALL run formatting, lints with warnings as errors, workspace tests,
`openspec validate --all`, the dash check, the layout check and the mockup inline check, in
that order, stopping at the first failure.

#### Scenario: A failing lint
- **WHEN** clippy reports a warning
- **THEN** the script stops there with a non-zero status, before the tests run
