# Tasks: ui-art-direction

## 1. Write-up

- [x] 1.1 The audit of today's screens (design 1) from fresh headless captures and the committed engine and browser captures.
- [x] 1.2 Research of shipped games with sources (design 2): Star Trek: Bridge Crew, FTL, Artemis, Elite Dangerous, Mass Effect, Hardspace: Shipbreaker, XAG 101 and 102, Microsoft's TV safe area.
- [x] 1.3 Proposal, design and spec delta; the bindings read from `sc-client` (design 10); every offending string with its file (design 12); the release bar (13).
- [x] 1.4 The mockup `docs/mockups/ui-art-direction.html`: eleven screens before and after on the 1920 x 1080 reference frame, five window sizes, the accent alternative; 45 shots in `docs/screenshots/mockups/ui-art-direction/`, looked at and shrunk.
- [x] 1.5 The owner approves the mockup and answers Q1 to Q12 (design 16). Approved, the owner, 2026-10-16: "approve"; Q1 to Q12 (survey UI1 to UI12)
  each "approve", the recommendation (design 16).

## 2. Style in data (after approval)

- [ ] 2.1 `tools/ui/ui_style.py` writes `data/ui/ui_style.json` from the mockup's `const UI` and `--check`s it; added to `scripts/check.sh` beside `console_style.py`.
- [ ] 2.2 The client loads the style at startup, refusing a missing or bad field with its path; egui's visuals (fills, strokes, radii, spacing) set from it; Ubuntu Light removed.
- [ ] 2.3 The reference-frame scale and anchors as one function (with the web agent's), used by every screen.

## 3. Screens (after approval, each with captures beside the mockup's)

- [ ] 3.1 Front menu and Single Player (`lobby.rs`): left-hung column, key art, station cards, one primary action.
- [ ] 3.2 Options pages and Controls (`options.rs`): Display, Game, Controls; the three binding pages from one bindings table in data, the lighting keys under Captain.
- [ ] 3.3 Briefing on a solid screen (`drill.rs`); full station names in `sc-core` (`Station::name`); mission titles without "Drill 1:".
- [ ] 3.4 HUD: the objective card and centre dot; roster and clock off the walking HUD; the alert chip at alert only.
- [ ] 3.5 Prompts as data (label, keys, verb) for `drill.rs`, `onfoot.rs` and `touch.rs`; the plate and its fades.
- [ ] 3.6 Esc menu side panel with Leave station when seated, objectives and roster (`game_menu.rs`).
- [ ] 3.7 Leave station in the console band with the hold ring (with the stand agent's button).
- [ ] 3.8 Loading screen and the first-time hint with its settings flag.
- [ ] 3.9 Transitions of design 8.
- [ ] 3.10 Every string of design 12 removed or rewritten, per the owner's answers to Q2, Q3, Q11 and Q12.

## 4. Measure

- [ ] 4.1 UI draw calls and texture memory of the menus, the briefing and the Esc menu on the Pi 5 (1 GB), against design 15's estimates.

## 5. Release check (proposed; build after approval)

- [ ] 5.1 `data/ui/forbidden_strings.json` and a headless test that collects every string the UI lays out on every reachable screen and fails on a forbidden one, naming the screen and the string.
- [ ] 5.2 A scripted walk through every reachable screen at 1280 x 720 and 1920 x 1080, captures to `docs/screenshots/engine/release/`.
- [ ] 5.3 An overlap and clipping check on the laid-out text rectangles of each capture (inside the screen, inside the panel, no two intersecting, no ellipsis outside an allowed band).
- [ ] 5.4 The three as one step in `scripts/check.sh`.
