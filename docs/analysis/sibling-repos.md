# The owner's other repositories: what Star Crew takes from each

Read on 2026-10-04 at the revisions named. Each section says what was adopted (and where it now
lives), what was not, and why. `star-crew-64` has its own document: `star-crew-64.md`.

## Pale-Blue-Dot (`5fcf83a`)

A voxel planet explorer with assisted flight: Rust, Bevy 0.18.1 and Avian 0.6.1, with an
engine-independent `pbd-core` crate and a `pbd-app` adapter. Eighty-odd OpenSpec changes, a
performance rig with a checked-in baseline, and the owner's most developed working rules.

**Adopted** (CLAUDE.md section in brackets):
- `CLAUDE.md` as the only copy of the rules, `AGENTS.md` as a pointer, `openspec/config.yaml`
  pointing at it (preamble).
- OpenSpec's split: specs are what the game does, changes are what is designed; specs move with
  the code (3).
- Write it up before touching code, with the measurement-instrument exception (4).
- An engine-independent core; rendering and platform APIs never become core dependencies (6.2).
- One authoritative implementation of a rule across single player and multiplayer (6.3).
- Tuning in validated data with units; zero is a value, not a sentinel (6.5).
- `f64` system coordinates, `f32` local frames, origin subtracted before the cast, frame
  identity explicit; passengers simulate in their vessel's local frame (7).
- Saved games survive every change; formats migrate forward (7).
- Performance measured the repeatable way: release build, nothing else running, old against
  new in one sitting, the spread reported, never call an unmeasured design a speedup (12).
- Cloud sessions do not measure frame time but do render captures (12).
- Screenshots stand in for videos until the owner asks for video (12).
- End every reply with artifact links; ask with the survey, only with screenshots; never
  schedule PR check-ins (13).
- Mockups are lit by their own lamps (adapted as "lit by their own fixtures", 11).
- The `owner-survey` and `obs-record` skills.

**Not adopted:**
- **Bevy and Avian.** Bevy renders through wgpu, whose OpenGL backend needs OpenGL ES 3.0; the
  Pi 3's VideoCore IV stops at ES 2.0 (`openspec/changes/engine-stack`).
- The voxel engine, planets and weather: a different game.
- The hex-size gold standard: a rule about Tenebris planets, not ships.

## fps-game-demo: Undercity on Brushfire tech (`f6cd25c`)

An immersive sim in Godot 4.7 .NET (C#) with an engine-independent `Undercity.Core` library,
three reference arena levels (Godot CSG, TrenchBroom, Blender) and a scripted Blender level and
character pipeline.

**Adopted:**
- No em or en dashes anywhere, with the grep that checks it (5).
- Avoid divergent code paths at all costs; what the interface previews is computed by the code
  that resolves it (6.1).
- SOLID, deterministic checks, seeded randomness, stable order (6.4).
- Code conventions: file contracts, tests as sentences, a bug fix brings its regression test,
  unknown data keys are errors, authored data fails loudly while player data is repaired,
  validate the real artifact, borrowed code keeps its provenance (6.6).
- Level rules: one layout source shared by the design map and the build; no z-fighting, with a
  checker that refuses bad geometry; people stand clear of the level, tested with the body's
  own collider (8).
- UI rules: mockup first, fixed-size panels, menus name things (10).
- Validation records say what was and was not proven (12).
- "Systems documents are deep: formulas, tuning tables, data schemas, UI mockups and
  walkthroughs. A sketch is not a design." The changes here are written to that standard.
- The `blender-csg-levels` and `blender-humanoid-characters` skills, as references.

**Not adopted:**
- **Godot.** Its Compatibility renderer needs OpenGL 3.3 or ES 3.0.
- The owner's verdict there that Blender beats TrenchBroom and Godot CSG for levels is carried
  into `openspec/changes/deck-pipeline` as evidence, not as a rule: decks are smaller and more
  regular than a city hub, and a generator from the layout may serve better.

## What all three agree on

- A design is written down, numbered and argued before it is built.
- The rules of the game live in a core that knows nothing about the engine.
- Data is the source; generators write files; editors and mockups read the same source.
- What the owner sees (screenshots, mockups, videos) is how a step is judged.
