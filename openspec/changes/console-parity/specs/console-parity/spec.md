# console-parity

## ADDED Requirements

### Requirement: An engine console shows what its approved mockup shows

Every console the engine draws SHALL show each panel and control of its approved mockup, in the same place within
4 lp on the 1280 x 720 canvas, with the same label or icon, and SHALL show no control the mockup does not have. A
control whose rule the engine does not have yet SHALL be drawn in the mockup's unavailable state.

#### Scenario: Helm in the engage state
- **WHEN** `tools/consoles/parity.py` compares the mockup's and the engine's layout manifests of Helm in the
  `engage` state
- **THEN** every mockup item is in the engine within 4 lp with its label and icon, the engine has no item the mockup
  lacks, and the check passes

#### Scenario: A control the mockup does not have
- **WHEN** the engine draws a Weapons Free bar that the approved Tactical mockup does not have
- **THEN** the parity check fails and names it

### Requirement: The consoles' look comes from shared data

The console palette, the console style and the icons SHALL come from `data/ui/palette.json`,
`data/ui/console_style.json` and `data/ui/icons.json`, read by both `docs/mockups/consoles.html` and the engine, and
the engine SHALL draw console text in the mockup's typeface.

#### Scenario: Changing a colour
- **WHEN** the `warn` colour in `data/ui/palette.json` changes
- **THEN** the mockup and the engine both draw it in the new colour, with no other file edited

### Requirement: A console is done only when its pictures are signed off

An engine console SHALL be called done only when the parity check passes and the owner has signed off side-by-side
captures of the mockup and the engine in the same named states.

#### Scenario: The drill's consoles
- **WHEN** Helm and Tactical are rebuilt
- **THEN** the comparison page shows the mockup and the engine in each named state, and the change records the
  owner's sign-off before its tasks are checked
