# console-parity

## ADDED Requirements

### Requirement: An engine console draws what its approved mockup draws

For every state the console mockup saves (`docs/screenshots/parity/<shot>.json`, written by
`tools/consoles/parity.mjs` beside `<shot>-mockup.png`), the engine SHALL draw that state with the drill's own console
code (`sc-client --console-fixture`), and `tools/consoles/compare.py` SHALL find no region of the console (title band,
look band, each panel, status strip) with more than 10 % of its ink missing, and no shot with more than 2.5 % of its
pixels differing. The viewscreen's inside SHALL be left out of the comparison. `scripts/check.sh` SHALL run the
comparison.

#### Scenario: Helm and Tactical in the saved states
- **WHEN** `scripts/check.sh` draws the seven saved states (`helm`, `helm-orient`, `helm-stick`, `red-helm-evading`,
  `tactical`, `red-tactical`, `red-tactical-turned`) in the engine and compares them with the mockup's pictures
- **THEN** every region is at most 10 % missing, every shot at most 2.5 % differing, and the check passes

#### Scenario: A panel drawn empty
- **WHEN** the engine's picture of Helm has its ORIENT panel drawn without its controls
- **THEN** ORIENT is about 86 % missing and the check fails, naming the shot and the panel

#### Scenario: A control moved
- **WHEN** a panel's controls are drawn 4 lp from where the mockup draws them
- **THEN** that panel is about 20 % missing and the check fails

### Requirement: The consoles' look is read out of the mockup

The console colours, the role colours and the icons SHALL be read out of `docs/mockups/consoles.html` and
`docs/mockups/lib/shipkit.js` by `tools/ui/console_style.py` into `data/ui/console_style.json`, which the engine
compiles in, and `tools/ui/console_style.py --check` SHALL fail when the file is not what the mockup says.

#### Scenario: Changing a colour in the mockup
- **WHEN** the `warn` colour in the mockup's `const C` changes and the data file is not regenerated
- **THEN** `tools/ui/console_style.py --check` fails, and after the tool is rerun the engine draws the new colour with
  no Rust edited

### Requirement: The engine sets console text in the mockup's typeface

The engine SHALL draw console text in Barlow Semi Condensed 600 and 700 with tabular figures and the face's kerning,
from TrueType files that `tools/ui/fonts.py` writes from the mockup's inlined woff2 files, and
`tools/ui/fonts.py --check` SHALL fail when a committed file is not what the tool writes.

#### Scenario: The font files
- **WHEN** `python3 tools/ui/fonts.py --check` runs
- **THEN** both TrueType files match what the tool writes from the woff2 files, and it passes

### Requirement: A console is done only when its pictures are signed off

An engine console SHALL be called done only when the parity check passes and the owner has signed off side-by-side
captures of the mockup and the engine in the same saved states.

#### Scenario: The drill's consoles
- **WHEN** Helm and Tactical are rebuilt
- **THEN** the comparison page shows the mockup and the engine in each saved state and in the live drill, and the
  change records the owner's sign-off before task 3.4 is checked
