# Star Crew Bridge Simulator: rules for Claude

Star Crew is a starship bridge simulator played online with friends. A core crew of four,
possibly more, runs one ship: they walk a full 3D bridge and the decks behind it, sit at
stations, route power, keep the air breathable, repair damage, man or automate turrets, load
missiles and launch fighters. It runs on a **custom engine** whose floor is a **Raspberry Pi 5
with 1 GB of RAM**, with a 4 GB Pi 5 as the main server, in a low-poly style.

These rules are binding. When a rule here conflicts with a general habit, a design doc or a
tool's default, this file wins. They were adapted from the owner's other projects
(Pale-Blue-Dot, the Undercity/Brushfire `fps-game-demo` and `star-crew-64`), and every rule
taken from there says so. See `docs/analysis/` for what was learned from each.

**This file is the only copy of these rules.** `AGENTS.md` is a pointer to it and must stay
one. Write a rule here, never there. Two files that have to agree are two files that won't, and
the one that is wrong is always the one nobody is reading. `openspec/config.yaml` points here
for the same reason.

Contents:

1. Owner direction (standing instructions)
2. The Raspberry Pi 5 floor
3. Spec-driven work: OpenSpec
4. Write it up before touching code
5. Writing style
6. Architecture and code quality
7. Ship and simulation invariants
8. Deck (level) design rules
9. Art: low poly
10. UI work
11. Mockups
12. Verification
13. Working with the owner
14. Skills in this repository
15. Design references
16. Documented exceptions

---

## 1. Owner direction (standing instructions)

From the owner's brief, 2026-10-04. Quotes are the owner's words.

- **This is the main repository.** "starcrew bridge sim will be our main repo here."
  `star-crew-64` (the N64 prototype of the same game) is reference only.
- **The game.** "A bridge simulator game that can be played with friends online. We'll aim for
  a core of 4 players, possibly more can join. The game simulates a starship bridge with
  multiple stations and activities the crew can do to make sure the ship is operating at peak
  capacity. Some players can even launch as a fighter if they wanted to."
- **Peak simulation.** "We're aiming for peak simulation systems here, so things like a full 3D
  starship bridge is vital to making this game work." The ship has:
  - a full floor plan;
  - a starship energy simulation system;
  - life support;
  - an engineering bay;
  - a fully working shuttle bay;
  - manable or automated turrets and missile launchers.
- **A custom engine.** "Ideally a custom engine because our ship needs to decouple the bridge
  and internal ship aspects from the exterior of the ship."
- **Low poly.** "I'm all for low poly aesthetics."
- **The hard requirement.** "I want this to run on pi3 1gb ram! So definitely bsp style level
  design would be on the table!", corrected the same day: **"I'm sorry we're running on a pi5
  1gb-4gb, 4 GB can be used as main server too"** (section 2). The target is a Raspberry Pi 5:
  1 GB is the client floor, and a 4 GB Pi 5 can be the main server. Brush-built, portal-culled
  decks stay the approach.
- **The stack is decided, and the graphics stay modest** (owner, 2026-10-04, on the
  engine-stack recommendation): "That's fine..this game doesn't need high end graphics. Your
  stack sounds like a solid plan". The engine is Rust with SDL3 and glow on OpenGL ES 3.0
  (`openspec/changes/engine-stack`). Spend effort on the simulation, not on rendering
  features the game does not need.
- **Documentation and mockups first.** "First we need to do some extreme documentation and
  mockups in 3js." Mockups are three.js pages (section 11).

## 2. The Raspberry Pi 5 floor

**Hard requirement** (owner, 2026-10-04: "we're running on a pi5 1gb-4gb, 4 GB can be used as
main server too"). The client runs on a **Raspberry Pi 5 with 1 GB of RAM**: four Cortex-A76
cores at 2.4 GHz, LPDDR4X shared between the CPU and the GPU, and a VideoCore VII GPU with
OpenGL ES 3.1 and Vulkan. A **4 GB Pi 5 can be the main server**, running the authoritative
simulation headless. A design that only works on a desktop is not a design for this game. (The
first designs were written for a Pi 3 the same morning; anything still saying "Pi 3" is stale.)

- **Memory is the hard limit, not speed.** On the 1 GB client the OS, the GPU's allocations and
  the game share 1 GB. Every process has a fixed allocation in the budget table.
- **The renderer's feature floor is OpenGL ES 3.0** (GLSL ES 3.00): instancing, vertex array
  objects, 32-bit indices, multiple render targets, uniform buffers and texture arrays are
  allowed. Compute, geometry and tessellation shaders and float render targets are not assumed.
  An extension is used only behind a fallback that has been measured on a Pi 5.
- **Every change states its Pi 5 budget.** A change that adds geometry, draw calls, memory,
  render targets, simulation work or network traffic says what it spends against the budget
  table in `openspec/changes/engine-stack/design.md` ("The Pi 5 budget"), and the table is the
  one source for those numbers. When the table moves into `openspec/specs/`, the rule follows
  it. The budgets are ceilings: low poly is the style.
- **The budget is provisional until measured on a Pi.** The numbers start as estimates from
  published figures. The first engine step is a probe that measures them on real hardware, and
  the table is corrected from that measurement, never the other way round.
- **Desktop numbers are never the measure.** A desktop frame time, a three.js mockup's frame
  rate or a cloud session's lavapipe render says nothing about the Pi. Mockups show the budget
  they would spend (section 11), not that it fits.
- **Memory is a budget, not a hope.** The client, the server and the GPU each have a fixed
  allocation in the table. Allocate up front, use arenas and pools for per-frame and
  per-entity data, and fail loudly at startup when a ship or a scene does not fit.

## 3. Spec-driven work: OpenSpec

Requirements and in-flight design live in `openspec/`, driven by the OpenSpec CLI
(`npm install -g @fission-ai/openspec`, Node 20.19+; https://openspec.dev). The split is the
point (adopted from Pale-Blue-Dot and Undercity):

- **`openspec/specs/<capability>/spec.md` is what the game DOES.** Every requirement in it is
  true today and pinned by a passing check: a unit test, a data validation, a measurement or a
  capture. A requirement for behaviour the game doesn't have yet does not belong there, however
  certain the plan is.
- **`openspec/changes/<name>/` is what is designed but not built.** It holds:
  - `proposal.md`: why, and what changes;
  - `design.md`: how, with the numbers, the formulas and the Pi 5 budget it spends;
  - `tasks.md`: the checklist;
  - spec deltas under `specs/<capability>/spec.md`, using `## ADDED / MODIFIED / REMOVED
    Requirements`, with `### Requirement:` headings, SHALL statements and `#### Scenario:`
    WHEN/THEN blocks.
- **`openspec/changes/archive/`** takes a change once its deltas are merged into the main
  specs and its work is real.

| Command | Does |
|---|---|
| `/opsx:explore` | Think the problem through |
| `/opsx:propose` | Write the planning artifacts, then stop |
| `/opsx:apply` | Implement (a separate request) |
| `/opsx:verify` | Check the implementation against the change |
| `/opsx:archive` | Close the change when it lands |

- **Keep changes small and single-purpose.** One change per capability, never an umbrella.
- **Validate before every push.** `openspec validate --all` must pass.
- **Specs move with the code.** Move a requirement from a change into `openspec/specs/` in the
  same commit that makes it true and adds the check that proves it, never ahead of it.
- **The design hub presents the changes; it is not a second source.** `docs/design/README.md`
  and the mockups show the changes, and each section names the change it presents. When a
  change moves, update its presentation in the same commit.

## 4. Write it up before touching code

**Standing instruction (adopted from Pale-Blue-Dot and Undercity).** The write-up comes first:

1. Investigate and measure.
2. Put the finding and the plan in `openspec/`: a proposal, a design and the spec deltas.
3. Stop there.

Editing source is a separate step, taken after the write-up exists and on a request to take it.
A change argued in a document can be read, disagreed with and redirected for the cost of
reading it. The same change argued in a diff has already been made.

**One exception: a measurement instrument**, meaning code whose only job is to produce a number
the write-up needs (the Pi 5 probe, a layout checker that reports volumes).
- Say in the write-up what is measured and why before writing it.
- Keep it to the instrument. Never let "I needed to measure" carry a behaviour change in with it.

**Mockups and documentation tooling are not engine code.** A three.js mockup, the layout
checker and the mockup inliner exist to make the write-up readable and are built with it.

Do not rename or rescale a tuning value, alter a default, or refactor toward a plan before the
plan is written down.

## 5. Writing style

**Never use em dashes (U+2014) or en dashes (U+2013), anywhere** (adopted from Undercity).
This covers docs, code comments, commit messages, PR bodies, in-game text, UI strings, logs and
chat replies.

| Instead of a dash | Use |
|---|---|
| Introducing a definition or explanation | A colon |
| A parenthetical aside | Commas or parentheses |
| A hard break between two clauses | A period, and two sentences |
| A range of numbers | A plain hyphen: `10-25 m` |

The repository holds none of them outside tool-owned files (section 16):

```sh
LC_ALL=C.UTF-8 grep -rnIP '\x{2014}|\x{2013}' --exclude-dir=.git --exclude-dir=.claude . && echo FAIL
```

- **Numbers carry units**, in SI unless a table says otherwise: metres, seconds, kilograms,
  kilopascals, kelvin or degrees Celsius (say which), megawatts and megajoules. A number a
  reader must act on names its unit.
- **Plain words, from the player's side** in anything the owner reads: what they will see or
  do, then how it works.

## 6. Architecture and code quality

The engine and its crate layout are decided in `openspec/changes/engine-stack`. Until that
change is built these rules are the target every design is written against.

### 6.1 One implementation of every rule

**Avoid divergent code paths that share functionality at all costs** (Undercity's
highest-priority rule).

- **Same behaviour, same code.** If two places need the same behaviour, they call the same
  code: no copies and no near copies.
- **Parameterize, don't clone.** A second caller that needs a variation extends the shared
  implementation.
- **Search first.** Before writing anything new, look for an existing implementation.
- **Report duplication.** If you find duplication, say so. Never add a third copy.

Concretely, each of these exists once: the power flow solve, the atmosphere step, the damage
resolution, the heat model, a weapon's fire rule, the compartment graph, and the frame
transforms. **What a console previews is computed by the code that resolves it**: the power a
breaker will deliver, the time a compartment takes to depressurize, a turret's hit chance. A
preview that disagrees with the outcome is the bug this rule prevents.

### 6.2 An engine-independent simulation core

- **The ship simulation lives in a core crate with no rendering, windowing, audio or network
  dependency.** It owns power, life support, heat, damage, weapons, flight, crew state and the
  compartment graph. It runs headless and is tested without a GPU.
- **The server runs the core; clients render it.** The dedicated server is the core plus a
  network adapter. The game client adapts core state into draw calls, sound and UI. That
  direction never reverses: the core never calls the renderer.
- **Rendering and platform APIs never become core dependencies** (from Pale-Blue-Dot).

### 6.3 One authoritative simulation

- **The server is authoritative** over every ship system, every exterior body and every
  door, breaker and valve. A client sends intents (commands) and renders state; it predicts
  only its own avatar and a fighter it flies.
- **One code path for every game.** A solo game is a local server with one client. There is
  no single-player shortcut that a networked game skips (Pale-Blue-Dot: "one authoritative
  implementation of a gameplay rule across single player and future multiplayer").

### 6.4 Determinism and stable order

- **The simulation steps at a fixed rate in seconds**, never in frames (star-crew-64 counted
  frames at 60 Hz, `docs/analysis/star-crew-64.md`).
- **Randomness is seeded and explicit.** Anything random takes a seed derived from the session
  seed, the entity's stable id and the purpose.
- **Order is stable.** A hash map is fine for keyed lookup, but never let hash order choose an
  outcome, an id, a save layout or a network message order.

### 6.5 Tuning lives in data

- **No inline tuning.** Never hardcode a tuning value in simulation code.
- **Data files carry units and are validated.** Tuning lives in committed, diffable files under
  `data/` with the unit in the key (`capacity_mj`, `flow_kg_s`, `range_m`) or the schema.
- **One source for each default.** A missing field inherits the code default. A present zero
  is zero, never a "use the default" sentinel. An unknown key is an error.
- **A data file that fails to parse or validate stops startup** with its path and field. It
  never falls back to defaults silently.
- **Authored data fails loudly; player data is repaired.** A damaged save loads as the nearest
  legal state with a logged warning, never as an illegal one.

### 6.6 Code conventions (from Pale-Blue-Dot and Undercity)

- **Document the contract.** Every file opens with a comment saying what it owns and why it
  lives where it does. Every public item is documented. Units go in the name or the doc.
- **Tests sit with the rule and read as sentences**
  (`a_breach_empties_the_hangar_in_under_a_minute`), one behaviour per test. A non-obvious
  assertion carries a message saying why it matters.
- **A bug fix brings its regression test**, committed with the fix.
- **Warnings are errors, and style is checked** by the language's formatter and linter.
- **Guard the edges.** Check indices before use, reject non-finite numbers where they enter
  (data loads, network messages, physics results, saves), and handle a capacity limit whole:
  an operation that doesn't fit changes nothing.
- **Validate the real artifact.** When two things must agree (Rust and GLSL layouts, a deck
  file and the layout it came from, a network schema on two ends), a check loads the actual
  artifacts and compares them, never two hand-written copies of an expected value.
- **Borrowed code keeps its provenance**: source repository and revision beside it. A reference
  checkout is never part of a build.

## 7. Ship and simulation invariants

- **The interior is decoupled from the exterior.** A ship's decks are a fixed local space.
  Crew walk, sit and fight in ship-local coordinates; the ship's position, rotation and
  acceleration in space never move them through physics. What they feel of the ship's motion
  (a lurch when the inertial dampers are overloaded, a shake on a hit) is an effect the
  simulation computes and applies on purpose. The design is `openspec/changes/ship-frames`.
- **Frames are explicit.** System positions and orbital time are `f64`; rendering and local
  collision are `f32` in a bounded local frame, with the origin subtracted before the cast
  (Pale-Blue-Dot). Every pose, velocity, network message and save names its frame. A craft
  that leaves the ship (a fighter, a shuttle, a missile) is handed from the ship's interior
  frame to the exterior at one tick boundary, with its velocity carried over.
- **One compartment graph.** A ship's compartments and the portals between them (doors,
  hatches, vents, breaches, windows) are one graph. Visibility, atmosphere, fire, sound, crew
  pathing and damage propagation all read it. Nothing keeps a second room list.
- **Systems are simulated, not faked.** Power flows from a source through buses and breakers to
  a load and is limited by what the source and the path can carry. Air moves between
  compartments by pressure difference through open portals. A readout on a console shows the
  simulated value. If a number on a screen is not the number the simulation uses, that is a
  bug.
- **Every station works with or without a player.** A station without a player is run by
  automation at a stated, lower competence, or is merged onto another console. Four players is
  the core crew; a fifth to eighth player takes a seat that automation was holding. One player
  can play.
- **A ship persists.** Leaving a station changes who operates it, never what it holds. A
  fighter that is not in the bay still exists, with its fuel, ammunition and damage. Save
  formats are versioned and migrate forward on load (Pale-Blue-Dot's "saved games survive
  every change").

## 8. Deck (level) design rules

The pipeline is decided in `openspec/changes/deck-pipeline`; these rules hold whatever it
becomes.

- **One layout source per ship.** A ship's floor plan (decks, compartments, portals, stations,
  system placements, mounts, craft) lives in one data file, `data/ships/<id>/layout.json`. The
  mockups, the design maps, the deck build and the simulation all read it, so they cannot
  disagree (Undercity's rule 7.1). `tools/layout_check.py` validates it.
- **Brush-style, compiled offline.** Decks are authored as convex brushes and compiled into
  compartments, portals, collision brushes and baked vertex lighting. Runtime visibility is
  portal culling through the compartment graph. Nothing about a deck is computed at runtime
  that could be compiled.
- **No z-fighting, ever** (Undercity 7.2). Two surfaces never share a plane while overlapping
  and facing the same way. Frame props have inset clear openings; trims stop at the faces they
  meet; deliberately parallel surfaces sit at least 1 cm apart. The deck compiler refuses a
  deck that fails the check.
- **People stand clear of the deck** (Undercity 7.4). Every station seat, spawn point and
  ladder is tested against the crew collider, not a hand-typed size.
- **Every compartment states its numbers**: volume in cubic metres, floor area, the triangle
  and draw-call cost of its geometry, and the portals that leave it.

## 9. Art: low poly

- **Flat-shaded, vertex-coloured low poly.** No normal maps, no PBR, no per-pixel lighting
  beyond an emissive term. Lighting is baked into vertex colours, with one set per lighting
  state (normal, red alert, emergency power) blended by a per-compartment uniform.
- **Light is baked offline, from fixtures that are data** (owner, 2026-10-04: "Some sort of
  tool or skill for light mapping/static light baking would be nice too"). How a bake is
  computed (shadows, emissive surfaces, bounce, vertex lighting or lightmaps, light probes for
  moving things) is `openspec/changes/light-baking`; where the result lives in a deck is
  `deck-pipeline`. The `light-baking` skill says how to bake and judge one.
- **Few, small textures.** One palette atlas and a small set of decal and screen textures,
  committed as PNG sources. Nearest-neighbour sampling, no mipmapped blur on palette swatches.
- **Meshes are files built by committed generators** (star-crew-64 and Undercity). A generator
  writes the source mesh deterministically; the build bakes it.
- **Every asset has a triangle budget** in the Pi 5 table, and the build refuses one over
  budget.

## 10. UI work

Adopted from Undercity section 8 and star-crew-64's UI text rules.

- **Mockup first.** Get the owner's approval of a mockup before implementing any new screen,
  console or layout. Not needed for bug fixes or text fixes within an approved design.
- **Consoles are full-screen 2D when seated.** A player seated at a station sees the station's
  console UI; the bridge stays visible behind or beside it. Console UI is drawn by the engine's
  immediate-mode UI, not rendered into textures on the bridge every frame.
- **Panels are a fixed size, and content never changes it.** Text is clipped with an ellipsis;
  unbounded lists scroll inside their panel. Reserve a band for each text element and verify
  the bands don't overlap.
- **Every input device drives every menu.** A lobby or a station accepts any connected
  keyboard, mouse or pad, not only the first.
- **Menus name things, they don't explain them.** A row is a label and a control. Reasons
  belong in `docs/`.

## 11. Mockups

- **Mockups are three.js pages** in `docs/mockups/`, one page per subject, built to be opened
  from disk and published as claude.ai artifacts. three.js comes from the jsDelivr CDN through
  an import map, pinned to one version for every mockup.
- **They read the one layout source.** A mockup never hand-places a room. `tools/mockups/
  inline.py` copies `data/ships/<id>/layout.json` and the shared `docs/mockups/lib/shipkit.js`
  into each page between marker comments, and its `--check` mode fails when a page holds a
  stale copy.
- **They are lit by their own fixtures** (adapted from Pale-Blue-Dot's "all your mockups
  should have lighting at night"). A ship interior is lit by its own lamps, screens and
  strips, and every mockup of an interior shows at least the normal and the red-alert lighting
  states. Space is dark; the sun is one light.
- **They show their Pi 5 cost.** Each mockup displays its visible triangles and draw calls
  against the Pi 5 budget, from `renderer.info`. It says plainly that a desktop frame rate is
  not a Pi measurement.
- **They present a change.** Each mockup names the OpenSpec change it presents, and its legend
  numbers match the layout's points of interest.
- **They are seen before they are shown.** Screenshot every mockup headless
  (`tools/mockups/shoot.mjs`) and look at the shots before calling it done. The shots go in
  `docs/screenshots/`.

## 12. Verification

Before claiming anything is done, run what applies:

| Check | Command |
|---|---|
| Specs | `openspec validate --all` |
| Dash check | Section 5 |
| Ship layouts | `python3 tools/layout_check.py` |
| Mockups hold the current layout | `python3 tools/mockups/inline.py --check` |
| Mockup screenshots | `node tools/mockups/shoot.mjs` |
| Engine (once it exists) | the format, lint and test commands `engine-stack` defines |

- **Know what is proven.** Distinguish implemented, validated and proposed work in docs, PRs
  and replies. A design is not a feature, a green test is not a visual sign-off, and a mockup
  is not a Pi measurement.
- **Every major step ends in a capture the owner can check**, before the next step starts
  (Pale-Blue-Dot and Undercity). Screenshots in `docs/screenshots/` stand in for video until
  the owner asks for video; a video, when made, names what each shot shows.
- **Performance is measured on the Pi, the repeatable way.** A release build, real time,
  nothing else running; old build against new interleaved in the same sitting; machine,
  resolution, build, repeats, percentiles and the spread between repeats reported. A
  difference smaller than the spread is not a result. Never call an unmeasured design a
  speedup (Pale-Blue-Dot's `perf-measure`).
- **Cloud sessions** have no GPU and no Pi. They don't measure frame time; say so in the PR.
  They do render: headless captures of what changed go in `docs/screenshots/`.

## 13. Working with the owner

- **End every reply with the artifact links** (owner rule in Pale-Blue-Dot and Undercity:
  "always give me links to artifacts generated or updated since last prompt at end of
  messages"). Every reply ends with a short list: each claude.ai artifact published,
  republished or edited since the owner's last message, with one line on what is new. If
  nothing was published or edited, say so in one line.
- **Ask with the survey, never in chat** (owner rule in Pale-Blue-Dot and Undercity). Open
  questions go into the survey Claude Doc, with options, a recommendation and an answer
  column. The `owner-survey` skill says how. The reply gives the survey link and the new
  question ids, not the questions.
  - **Ask only with something to look at** (the owner's latest word on this, Pale-Blue-Dot
    2026-09-29: "dont ask me questions unless you have screenshots for me to review"). A
    question goes in the survey with its mockup or screenshots beside it. Anything else: take
    the recommendation, record it in the change as "recommendation taken (ask only with
    screenshots)", and say so in the reply.
  - The current survey's link is in `docs/design/README.md`. Keep editing it rather than
    starting another.
- **Never schedule a PR check-in** (owner rule in Pale-Blue-Dot: "you should never rearm pr
  checkins"). No `send_later`, routine, `/loop` or timer that wakes the session to re-read a PR.
  A PR is looked at when the owner asks, when its own events arrive, or as part of work in hand.

## 14. Skills in this repository

| Skill | Use |
|---|---|
| `openspec-*` and `/opsx:*` | The OpenSpec workflow (section 3). Tool-owned: `openspec update` rewrites them. |
| `owner-survey` | Questions for the owner (section 13). |
| `threejs-mockups` | Building, inlining, screenshotting and publishing a mockup (section 11). |
| `light-baking` | Placing lamps, baking and judging static light, the three lighting states (section 9). |
| `obs-record` | Recording a window with OBS on the owner's machine, for videos of the running game. |
| `blender-csg-levels` | Undercity's scripted Blender CSG level kit, kept as the reference for the deck pipeline. Its Godot export does not apply here; see its "In Star Crew" note. |
| `blender-humanoid-characters` | Undercity's character kit, kept as the reference for crew bodies. Its budgets are a desktop's, not a Pi's; see its "In Star Crew" note. |

Provenance for every copied skill is in `.claude/skills/PROVENANCE.md`.

## 15. Design references

`docs/references.md` lists the bridge simulators, ship-systems games and engines this design
borrows from (Artemis, EmptyEpsilon, Starship Horizons, Pulsar, FTL, Barotrauma, Space Station
13, Quake, Descent and others), and what each one is cited for.

- **Cite, don't recall.** When a decision rests on how a reference does something, name the
  source in the design doc rather than asserting it from memory.
- **Take the shape, not the text.** Names, text, maps and art belong to their owners. We
  inherit system design, never content.

## 16. Documented exceptions

Nothing is an exception until it is listed here with its reason.

- **Tool-owned files keep their tool's text.** The OpenSpec CLI writes
  `.claude/skills/openspec-*` and `.claude/commands/opsx/*`, and `openspec update` rewrites
  them. The dash check excludes `.claude/` for that reason. Don't hand-edit them.
- **Copied skills keep their source's text** below their "In Star Crew" note, so a later copy
  from the source can be diffed. They are listed in `.claude/skills/PROVENANCE.md`.
