# Skill provenance

Where each skill in this directory came from, so a later copy from the source can be compared
(CLAUDE.md sections 6.6 and 16).

| Skill | Source | Revision | Copied | Changes here |
| --- | --- | --- | --- | --- |
| `openspec-*`, `../commands/opsx/*` | OpenSpec CLI 1.14.0 (`openspec init --tools claude`, profile with `verify` added) | 1.14.0 | 2026-10-04 | None. Tool-owned: `openspec update` rewrites them. |
| `owner-survey` | Pale-Blue-Dot `.claude/skills/owner-survey`, with Undercity's (`fps-game-demo`) "never in chat" | Pale-Blue-Dot `5fcf83a`, fps-game-demo `f6cd25c` | 2026-10-04 | Rewritten for Star Crew: survey link location, question id prefixes, the owner's "ask only with screenshots" rule. |
| `obs-record` | Pale-Blue-Dot `.claude/skills/obs-record` (with `scripts/obs_record.py`) | `5fcf83a` | 2026-10-04 | An "In Star Crew" note after the front matter; the rest verbatim. |
| `blender-csg-levels` | fps-game-demo `.claude/skills/blender-csg-levels` (with `kit.json`, `references/`, `scripts/`, `templates/`) | `f6cd25c` | 2026-10-04 | An "In Star Crew" note after the front matter; the rest verbatim. |
| `blender-humanoid-characters` | fps-game-demo `.claude/skills/blender-humanoid-characters` (with `kit.json`, `references/`, `scripts/`, `templates/`) | `f6cd25c` | 2026-10-04 | An "In Star Crew" note after the front matter; the rest verbatim. |
| `threejs-mockups` | New in Star Crew | | 2026-10-04 | |
| `light-baking` | New in Star Crew (the `light-baking` change's how-to) | | 2026-10-04 | |
| `material-maker` | New in Star Crew, pipeline adapted from fps-game-demo `tools/material_maker` (which has no skill); per-file provenance in `tools/materials/README.md` | `f6cd25c` | 2026-10-05 | |
| `blender-hard-surface` | New in Star Crew (the how-to of `tools/blender/build_bridge_props.py`, written from scratch); its cutter rules adapted from `blender-csg-levels` and fps-game-demo `tools/blender/build_props.py` (bevelled parts joined into one prop), no code copied | `f6cd25c` | 2026-10-05 | |

Not copied: Pale-Blue-Dot's `perf-measure`. Its rules (release build, nothing else running, old
against new in one sitting, report the spread) are in CLAUDE.md section 12; the skill itself
drives Pale-Blue-Dot's `tools/perf_suite.py`, which has no counterpart here yet. A Star Crew
version follows the `sc-probe` (engine-stack task 2).
