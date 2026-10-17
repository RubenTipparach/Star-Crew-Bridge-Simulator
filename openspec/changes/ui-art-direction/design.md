# Design: UI art direction

Everything here is presented in `docs/mockups/ui-art-direction.html`, before and after, with its shots in
`docs/screenshots/mockups/ui-art-direction/` (named `ui-art-direction-<screen>-<before|after>[-size].png`; "shot X"
below means that file). Nothing in this change is built. The consoles keep their approved look (`consoles.html`,
`console-parity`): this is everything around them.

## 1. What reads as machine-made today

Looked at in fresh headless captures of the release client (`sc-client --headless --hand-over-test`,
`--lobby-test`, 2026-10-15) and the committed engine and browser captures. The shots `*-before*` are these.

| Screen | What gives it away |
| --- | --- |
| All menus | egui's stock look: its default typeface (Ubuntu Light, a different family from the consoles' Barlow Semi Condensed), pill buttons with 20 to 32 px radii, the stock window frame with a rounded translucent fill. A tool's defaults read as no decision. |
| All menus | Three accents fighting: amber (the title, the main button), lilac (focus, the subtitle, selected options) and navy slate. Focus is lilac on an amber button, so the eye has two "look here" colours. |
| Front menu (shot menu-before) | A centred box floating on a stock planet that is not this game's; the subtitle "SCS TERN" in a third colour; every button the same size and weight. |
| Single Player (shot lobby-before) | Greyed pills for stations that do not exist (Comms, Flight ops); dev words as buttons ("Roll", "TOUR"); four colours on one panel; the main action and the secondary one differ only in fill. |
| Options (lobby capture menu-3) | Labels centred in a column, sliders in egui's stock style, values in a third alignment. |
| Key strip (shot controls-before) | Running text in a monospace face under the game, keys as small amber chips inside sentences, a dev note ("A browser frame rate says nothing about a Pi"), a lighting key any player can press. |
| Briefing (shot briefing-before) | A see-through layer over the Helm console: dials, numbers and buttons show through the words. Everything the same weight: title, summary, orders, crew, the countdown and helper text all compete. "(bot)" after names, "automation" as a name, "Lt. Player", "A bot plays this station", "Laser stbd", "ENGAGING IN 4". A row of eight station buttons that look like tabs. |
| Top bar (shots hud-before, hud-before-2) | A navy bar across the whole top with the mission title, the phase enum uppercased (MUSTER, PATROL, ENGAGE) and a running clock; eight "Station name" pairs always on screen, which overlap the title in the browser; "BRIDGE (V)" along the bottom. |
| Walking prompt (shot prompt-before) | "E  Take Captain from Lt. Sa'ral": floating amber text with the key spelled into the sentence, no backing, so it fights the scene. |
| Nameplates | "(NPC)" after every bot's name (owner-requested, see 16). |
| Menu on Esc (shot pause-before) | The same rounded translucent box over the console; lilac focus; the console's band still reading "Lt. Player". |
| Consoles' status strip | "Lt. Player · HELM", always-zero counters, "-- MW" with "not simulated in this drill" tips. |
| Missing moments | No loading screen, no first-time hint, no transitions: the menu cuts to the ship. |

The typical machine-made tells, named: one size and weight for everything; small grey text everywhere; generic dark
navy panels with thin borders and rounded corners; labels on everything; translucent panels with the game showing
through text; no hierarchy; debug readouts; key hints as running text.

## 2. What shipped games do (research)

Each observation was read at its source on 2026-10-15. Where a source could not be opened it is not used.

| Game | What we take | Source |
| --- | --- | --- |
| Star Trek: Bridge Crew | The lobby is a briefing room aboard where the crew meet and the Captain picks the mission; each station owns one domain; legibility over nostalgia (the Original bridge's unlabelled "candy-colored buttons" are the review's complaint). | https://gaming.trekcore.com/startrekbridgecrew/review.html, https://www.gamereactor.eu/star-trek-bridge-crew-review/ |
| FTL | One fixed layout, each zone with one job (hull upper left, crew down the left edge, power lower left, target right); a pause that keeps you in the game; a full set of screens (menu, options, loading, help, resume, game over). | https://pixeltwelve.com/guides/ftl-first-run-pause-power-targeting-repairs, https://store.steampowered.com/app/212680/, https://interfaceingame.com/games/ftl-faster-than-light/ |
| Artemis Spaceship Bridge Simulator | Main menu of three verbs; the lobby is choose a ship, choose consoles, "Ready to Play", with a light per connected console; an in-game menu of four items; red alert is a klaxon and red light on every deck. | https://shared.fastly.steamstatic.com/store_item_assets/steam/apps/247350/manuals/Artemis_Manual_1.70-rev5.pdf |
| Elite Dangerous | One accent ("bright orange with blue highlights for contrast"); the cockpit layout was the starting point "from where everything else evolved"; GUI audio designed with the visuals; a separate keybinding screen with presets. Its translucent holo panels are the counter-example to 6. | https://www.windowscentral.com/how-change-your-hud-color-elite-dangerous-pc-only, https://www.uxmatters.com/mt/archives/2015/09/an-interview-with-louise-mclennan-designer-of-elite-dangerous.php, https://community.elitedangerous.com/en/node/274, https://interfaceingame.com/games/elite-dangerous/ |
| Mass Effect | Prompts appear only when the player can act, as a button icon; one accent through menus and holograms; loading screens that carry art and information instead of dead time. | https://www.pcworld.com/article/524494/mass_effect_2_interview_p2.html, https://www.gamedeveloper.com/design/mass-effect-meaning-through-design, https://www.engadget.com/2009-12-06-mass-effect-2-ditching-elevators-for-load-screens.html |
| Hardspace: Shipbreaker | A limited palette and a mostly diegetic HUD; a redesign from "overlapping UI elements" to "redesigned information architecture"; the work order is the briefing, popping up on the HUD; a warning: a "minuscule HUD indicator" gets missed. | https://news.xbox.com/en-us/2022/08/23/how-blackbird-interactive-crafted-an-immersive-future-for-hardspace-shipbreaker/, https://vidhistuff.com/games/hardspace-shipbreaker/, https://gamingnexus.com/Article/6128/Hardspace-Shipbreaker-Preview |
| Xbox Accessibility Guidelines 101, 102 | Text body height at least 18 px on PC at 1080p (26 px on a console); contrast 4.5:1 for standard text, 3:1 for large; offer a solid background behind on-screen text. | https://devdocs.xbox.com/build/game-principles/accessibility/xag-deep-dives/xag-101-text-display.md, https://devdocs.xbox.com/build/game-principles/accessibility/xag-deep-dives/xag-102-contrast.md |
| Microsoft, designing for TV | Keep essential UI out of the outer 5 % of the screen: 48 x 27 epx at 960 x 540, so 96 x 54 px at 1080p. | https://learn.microsoft.com/en-us/windows/apps/design/devices/designing-for-tv |

The principles this change takes, and who supports each:

1. **One strong hierarchy, each zone one job** (FTL, Bridge Crew, Shipbreaker's redesign).
2. **Few type sizes, one family** (all six; Elite and Mass Effect hold one face through menus and HUD).
3. **A deliberate palette with one accent** (Elite, Mass Effect, Shipbreaker); state colours stay for state.
4. **Solid panels behind text** (XAG 102; Elite's translucency is what we avoid).
5. **Keys drawn as keycaps, shown only when they do something** (Mass Effect's icon prompts).
6. **Diegetic framing and sound** (Bridge Crew's briefing room, Artemis's klaxon, Elite's GUI audio, Shipbreaker's work order).
7. **Safe-area margins and minimum sizes** (Microsoft TV, XAG 101).
8. **Short verb lists for menus** (Artemis, FTL).
9. **Loading carries art and one useful thing** (Mass Effect 2).
10. **Controls on their own screen, grouped by activity, per device** (Elite's keybinding screen; Artemis's manual).

## 3. The reference frame and anchors

The owner, 2026-10-15: "buttons and text need to scale with reference pixels at 1920x1080". Every screen is laid out
in **reference pixels** on a 1920 x 1080 frame. At a screen of `w` x `h` pixels every UI size is multiplied by

    s = min(w / 1920, h / 1080)

so nothing leaves the screen at any aspect. Groups hang from **anchors**: top left, top right, bottom left, bottom
right, middle left, middle right, centre and bottom centre. An anchored group is laid out in reference pixels and
scaled by `s` about its anchor point, so a corner group stays in its corner on an ultrawide screen and a centred frame
stays centred on a 4:3 one. Full-screen backgrounds (key art, the 3D view, scrims, solid backdrops) are not UI: they
cover the whole screen (`max` of the two ratios), and a world mark (the station marker) follows the world picture and
keeps its own size by `s`.

The mockup shows it at 1920 x 1080, 1280 x 720, 1024 x 768 (4:3), 2560 x 1080 (ultrawide) and 985 x 455 (a short
embed, the itch.io player in a small window): shots `menu-`, `controls-`, `briefing-`, `prompt-` and
`pause-after-<size>`. Nothing is off screen at any of them.

The frame and the scale are one source already: `data/ui/reference.json` (`starcrew.ui-reference/1`,
`browser-page-fit` design 5), read by `sc-client`'s `ui.rs`, which centres the frame and gives the extra space to the
edges. This change adds the anchors on top of it: a group anchored to an edge or a corner hangs from the screen's
edge, not the centred frame's, so on an ultrawide screen the objective card sits in the real corner. The anchors are
one function beside the scale, and every screen calls them.

The **safe area** is 96 px left and right and 54 px top and bottom (reference): nothing a player reads sits outside it.
The **grid** is 8 px: every gap, padding and size is a multiple of 8, except hairlines (1 px), the accent bar (6 px)
and keycap depth (4 px).

## 4. Type

One family, the consoles': **Barlow Semi Condensed**, 600 for body and 700 for everything else, with tabular figures
(`assets/fonts/barlow-semi-condensed`, already baked for egui by `tools/ui/fonts.py`). Ubuntu Light leaves the game.

| Role | Reference px | Weight | Case and tracking | Used for |
| --- | --- | --- | --- | --- |
| Display | 72 | 700 | Caps, 0.03 em | A mission's name, a count that matters (Launch in 0:04) |
| Title | 40 | 700 | Caps, 0.05 em | Menu items, screen titles, the primary button |
| Body | 28 | 600 | Sentence case | Everything read: objectives, orders, names, prompts, rows |
| Label | 20 | 700 | Caps, 0.16 em | Section labels, keycap letters, small states |

Four sizes and no more on any screen. The game's wordmark (STAR CREW, 128 px) is art, not type. A label is never
grey body text: small text is either a caps label or it is body size. XAG 101 asks for 18 px body height on PC at
1080p; Body is well above, Label (20 px, bold caps) meets it at 1080p and falls below it under 1080p by the owner's
scale rule (13 px at 720p, 8 px in the 985 x 455 embed): open question Q8.

## 5. Palette roles

Built on the consoles' own colours (`data/ui/console_style.json`), so the screens and the consoles are one game.

| Role | Hex | Contrast on panel | Use |
| --- | --- | --- | --- |
| ground | #070a0f | | Solid backdrops, the briefing, loading's foot |
| panel | #0c121a | | Every panel (opaque) |
| raised | #141d29 | | Focus fill, the selected row, your orders |
| line | #1f2a37 | | Hairlines, dividers, empty bars |
| line2 | #2f3d4f | | Edges of fields and secondary buttons |
| text | #e8eef6 | 16.1:1 | Anything read |
| dim | #98a8bc | 7.8:1 | Secondary words, unfocused items (the consoles' #6f7f94 is 4.6:1 and stays on the consoles) |
| faint | #5d6c80 | 3.5:1 | Disabled, and large text only (XAG 102's 3:1) |
| accent | #6fd3ff (ice) | 11.1:1 | Focus bars, the primary button, section labels that lead, progress; the only "look here" colour |
| ink | #04121a | 11.2:1 on accent | Text on the accent |
| ok, warn, danger | #3ddc84, #ffc542, #ff4757 | | State only, always with a shape (CLAUDE.md 10) |
| roles | console_style roles | | Station icons only (helm blue, tactical red, engineering amber...) |

The accent is a choice the owner makes: ice (the Tern's livery cyan, far from every state colour) or amber (today's
title colour, Elite's direction, but next to warn and the emergency orange). Shots `lobby-after` and
`lobby-after-amber`, question Q1. Recommendation: ice.

## 6. Panels

- **Opaque.** No panel shows the game through its text (XAG 102; the briefing's fault today). A scrim behind a menu is
  a separate full-screen fill at 84 to 90 %, never the panel itself.
- **Square.** Corner radius 0 on panels and buttons; 4 px only on keycaps. No pills.
- **One hairline** (`line`, 1 px) and no shadows or glows.
- **Focus** is the accent bar (6 px, left edge) plus `raised` fill plus text going from `dim` to `text`. Nothing else
  changes size or colour on focus.
- **Primary action**: one per screen, accent fill with `ink` text at Title size (Beam aboard). Secondary: hairline
  `line2` box with Label text.
- **Unavailable**: the control drawn at 45 % with a lock icon, no explanation (CLAUDE.md 10's unavailable state).

## 7. Keycaps and prompts

- A key is drawn as a **keycap**: 44 px tall, at least 44 wide, 12 px side padding, `#18212c` fill, 1 px `#3a4859`
  edge with a 4 px bottom edge, 4 px radius, the key's name in Label weight. Mouse actions are a keycap with a mouse
  glyph (look, click with the left button lit, drag, wheel). A held key has HOLD before it (16 px, `dim`). Touch
  shows its own buttons' names, never keyboard letters.
- A key is **never spelled into a sentence** ("E  Take Captain from ..."), never in running text, and never on screen
  when it does nothing.
- **Prompts** (shot prompt-after): a solid plate below the centre dot, the thing's name as a Label over a keycap and a
  verb in Body. One line of verbs at most; two keys side by side when there are two (the lift: [E] Up, [Q] Down). The
  plate fades in over 120 ms and out over 200 ms. Today's prompts map to:

| Today (file) | After |
| --- | --- |
| `E  Sit at Helm` (`drill.rs` 2205) | HELM, [E] Sit down |
| `E  Take Captain from Lt. Sa'ral` (`drill.rs` 2203) | CAPTAIN, [E] Relieve Lt. Sa'ral |
| `{name} holds {st}`, `{name} is on the way to {st}` (2204, 2200) | CAPTAIN, Lt. Sa'ral (no key: nothing to do) |
| `E  Repair: {title}` (689) | the machine's name, [E] Repair |
| `E  Take an extinguisher` (690), `Bracket empty` (691) | EXTINGUISHER, [E] Take; EXTINGUISHER, Empty |
| `FIRE   F  Spray it   (agent 80%)` (692) | FIRE, HOLD [F] Spray, with the agent as a bar under the plate |
| `FIRE   An extinguisher hangs by the door` (693) | FIRE, Find an extinguisher |
| `Extinguisher 80%` (697) | a small bar beside the dot while one is carried |
| `E  Up        Q  Down`, `E  Call lift` (`onfoot.rs` 154, 157) | LIFT, [E] Up [Q] Down; LIFT, [E] Call |

The touch layer's `touch_prompt` (`touch.rs` 584) keeps working: the prompt becomes data (a label, a key, a verb)
rather than a string to strip letters from.

## 8. Motion and sound

| Moment | Motion (ms) | Sound |
| --- | --- | --- |
| Focus moves | 90, the bar slides | a short tick |
| A panel opens | 160, fade and a 12 px rise | none |
| Screen to screen (menus) | 250, fade through `ground` | confirm on the press |
| Loading out | 400, fade from `ground` into the ship | the ship's hum rising |
| Briefing at the console | 350, lowers into the desk; the bridge stays lit behind | none |
| The console rising, sitting down | 600 (`CONSOLE_RISE_S`, `drill.rs` 274, unchanged) | the seat's own sound |
| A prompt | in 120, out 200 | none (a chirp only for the first-time hint) |
| Esc menu | 160 scrim and side panel; the ship runs on | a soft open |

No bounce, overshoot or blur. Never a cut to black between walking and working (CLAUDE.md, the law); the only fades
through a solid colour are between menus and into the ship from loading. Sounds are proposed here and made with the
sound change; until then the engine plays none.

## 9. Style in data

The tokens live in one place, the mockup's `const UI` (reference size, safe area, grid, type sizes, colours, the
alternative accent, motion), as the consoles' style lives in `consoles.html`. The reference size is not copied: it
is `data/ui/reference.json`'s, and the tool checks the mockup's `reference_px` against it. After approval a tool
(`tools/ui/ui_style.py`, after `console_style.py`) writes `data/ui/ui_style.json` from it and `--check`s that the
committed file is what it writes; the engine loads the file at startup, and a missing or bad field stops it with the
path and the field (CLAUDE.md 6.5). No colour or size is written in `sc-client` code.

## 10. Options > Controls

The owner: the key strip goes off every screen ("this text should be remocved from the main screen and any screen")
into Options "as a square panel". Shots `controls-after`, `controls-console-after`, `controls-touch-after`.

Options becomes three pages on a left rail: **Display** (full screen or window, field of view), **Game** (names on or
off) and **Controls** (look speed, and the bindings). Controls is a square panel (880 x 880 reference px) with three
tabs. Each row is an action name in Body and its keycaps on the right, under a Label heading. Bindings are read-only
in this change (rebinding: Q10).

The bindings, from the client's code (2026-10-15), with what the strip missed marked *new*:

| Page, group | Action | Keys | Where |
| --- | --- | --- | --- |
| On foot, Move | Look | mouse (click to capture) | `main.rs`, `drill.rs` |
| | Walk | W A S D | `onfoot.rs` 100 |
| | Run | hold Shift | `onfoot.rs` 102 |
| | Jump | Space | `onfoot.rs` 103 |
| | Fly (explore only) | F | `main.rs` 1414 (Q5) |
| On foot, Use | Use, sit, repair | E | `onfoot.rs` 104, `drill.rs` 554, 595 |
| | Climb down | Q | `onfoot.rs` 105 |
| | Extinguisher (*new*) | hold F, or hold click | `drill.rs` 600 |
| On foot, Lift | Up a deck, Down a deck, Call | E, Q, E | `onfoot.rs` 154, 157 |
| On foot, Ship map | Open, Turn, Zoom, Next deck | M, drag, wheel, Tab | `main.rs` 1367, 1382 |
| On foot, General | Menu (*new*), Screenshot (*new*) | Esc, F12 | `main.rs` 1358, 1413 |
| At a console, Any | Press a control (*new*) | click | `seat.rs` |
| | Stand up (*new*) | hold Backspace (hold E too, except at Helm) | `drill.rs` 706 |
| | Acknowledge order (*new*) | Y | `seat.rs` 1589 |
| | Next console (*new*) | Tab | `drill.rs` 968 |
| | Hand to the crew (*new*) | M | `drill.rs` 960 |
| | Bridge view (*new*) | V | `drill.rs` 1674 |
| At a console, Helm (*new*) | Speed, Yaw, Pitch, Roll, Strafe, Rise and sink, All stop | W S, A D, R F, Q E, Z C, Space Ctrl, X | `seat.rs` 1527-1542 |
| At a console, Tactical (*new*) | Lock target, Load a tube, Fire | T, L, hold G | `seat.rs` 1565-1576 |
| At a console, Lasers (*new*) | Fire | hold G | `seat.rs` 1491 |
| At a console, Gunner (*new*) | Aim, Fire, Next target | arrows, hold Space, Tab | `seat.rs` 1501-1507 |
| At a console, Science (*new*) | Scan | Q | `seat.rs` 1582 |
| At a console, Captain | Normal lighting, Red alert, Emergency power | 1, 2, 3 | `main.rs` 1410-1412 today |
| Touch | Move, Look, Use, Jump, Climb down, Ship map, Menu; at a console tap, hold for details, hold Stand | the `touch-controls` layout | `touch.rs` |

- **Lights belong to the Captain** (owner, 2026-10-15: "lighting can only be changed by the captain"). The keys 1, 2
  and 3 act only from the captain's seat, the same command as the ALERT panel's buttons; anywhere else they do
  nothing and are not listed. The web agent is enforcing the rule in the game; this page shows it.
- **Dropped from the list as dev controls**: Tab to let the mouse go while walking (`main.rs` 1377; Esc's menu frees
  the mouse), F3's network overlay (`drill.rs` 1675; Q11).
- The helm's Q and E roll while walking's Q and E use and climb: the pages are separate because the same keys mean
  different things seated and standing.

## 11. The screens

| Screen | Shots | After |
| --- | --- | --- |
| Front menu | menu-before, menu-after (and four sizes) | The wordmark and four Title items in a left-hung column on the Tern's own key art (`outside-port-bow`, darkened to the left), the first item focused. No subtitle in a third colour. A version label bottom right, faint. |
| Single Player | lobby-before, lobby-after, lobby-after-amber | Esc Back, Your officer (rank, name, a dice for a new name), Station as cards with the consoles' role icons, unbuilt stations locked (Q4), one primary Beam aboard, Explore the ship as secondary (Q5). The officer's name is the player's: no default "Lt. Player". |
| Options > Controls | controls-before, controls-after, controls-console-after, controls-touch-after | Section 10. |
| Briefing, eight crew | briefing-before, briefing-after (and four sizes) | A solid screen on the console (the console is up; this is its screen before the round), a band of the system's art fading into `ground`: Training drill 1, the mission in Display, the summary, Objectives with diamonds, Your orders in a raised panel with the station's icon, Crew as eight rows (role icon, full station name, officer, a check when ready, YOU on your own row, Ship's computer with a faint icon where no one sits), Launch in 0:04 with a bar. No station buttons (a row is chosen by clicking it), no helper text. Q7. |
| HUD on the bridge | hud-before, hud-before-2, hud-after | One objective card in the top-left safe corner and a centre dot. The roster, the phase name and the clock leave the HUD (the roster to the Esc menu); an alert chip appears top right only at red alert or emergency power. Q6. |
| Walking prompt | prompt-before, prompt-after (and four sizes) | Section 7. |
| First-time hint | hint-after | Once, on the first time aboard: a card (First steps, "Walk to the helm and sit down.", W A S D Move, mouse Look, E Sit) and a marker over the player's station with its distance. It leaves when they sit, and a flag in `settings/` keeps it gone. |
| Menu on Esc | pause-before, pause-after (and four sizes) | A solid side panel over a 90 % scrim: the mission, Resume, **Leave station** (seated only), Options, Controls, Leave ship (Return to main menu or Quit to desktop, each asking once, as `game-menu` says), your name with your station's icon. On the right, the objectives with their state and the crew roster. The ship keeps running. |
| Console band | console-before, console-after, console-after-holding | **Leave station** in the band's right end, before the alert state and the clock: the Stand key as a keycap inside a ring that fills while held (E, or Backspace at Helm). Clicked, it stands up as the held key does. The rest of the console is untouched. |
| Loading | loading-after | Key art, the mission's name, one tip with its keycap, "Boarding the Tern" over a thin bar. No percentages or file names. |
| Transitions | flow-after | The six moments in order with their motion (section 8). |

## 12. The player-facing text rule

**No internal names, debug values, placeholder names or dev notes on a player's screen.** A player sees what their
character would: names of things in the game's words, numbers they act on, and nothing about how the game is built.
Every string the UI draws comes from data or a short list of UI words, both checked by 14.

The current offenders (found 2026-10-15; paths under `crates/sc-client/src/` unless given). The engine pass removes or
rewrites every one; those marked **Q** were asked for by the owner or a design and are questions, not removals.

| Screen | File: line | String | Why |
| --- | --- | --- | --- |
| Front menu | `lobby.rs` 171 | `SCS TERN` subtitle | internal ship id as a subtitle (Q9) |
| Single Player | `lobby.rs` 18, 296, 301 | `Comms`, `Flight ops` greyed | unbuilt features (Q4) |
| Single Player | `lobby.rs` 286 | `Roll` | dev word (a dice icon) |
| Single Player | `lobby.rs` 324 | `TOUR` | dev mode as a main action (Q5) |
| Multiplayer | `lobby.rs` 337 | `192.168.1.20` hint | raw address |
| Multiplayer | `lobby.rs` 350-355 | `CODE`, `Join` always off | unbuilt matchmaker (unavailable state, no text) |
| Tour HUD | `main.rs` 619, 908 | `Any` as a station | internal value |
| Connecting | `drill.rs` 2339 | `Waiting for the drill` | internal mode name |
| Connecting | `drill.rs` 2340, 491 | `Connecting to {server}`, `Cannot find {server}` | raw address |
| Window | `main.rs` 1498, 1534, 1573, 1611 | `Star Crew: drill`, `: single player`, `: repairs`, `: console fixture` | internal names |
| Briefing | `drill.rs` 2508 | `automation` as a name | internal word (Ship's computer) |
| Briefing | `drill.rs` 2509 | `{name} (bot)` | dev tag |
| Briefing | `drill.rs` 2544 | `A bot plays this station` | helper text |
| Briefing | `drill.rs` 2523 | `ENGAGING IN {n}` | phase language (Launch in) |
| Briefing | `drill.rs` 2515, 2528 | `reading`, `CREW SITTING` | internal states |
| Briefing | `drill.rs` 2495; `sc-core/src/combat/mod.rs` 182-183 | `LASER STBD ORDERS` | abbreviation (Starboard laser) |
| Briefing | `data/missions/drill-hound.json` 5 | `Drill 1: Hound at the Shoals` | dev label in a title |
| Top bar | `drill.rs` 2391 | `{title}   {PHASE}   {mm:ss}` | enum name and running clock |
| Top bar | `drill.rs` 2395-2396 | `{Station} {name}` or `auto` | always-on roster, internal word |
| Walking | `drill.rs` 2407 | `BRIDGE (V)` | view label with a key |
| Walking | `drill.rs` 689-697, 2200-2205; `onfoot.rs` 154, 157 | the prompts of section 7 | running-text keys, raw numbers, `Bracket empty` |
| Nameplates | `nameplate.rs` 45 | ` (NPC)` | dev tag (**Q3**: owner asked, 2026-10-10) |
| Network | `drill.rs` 1675, 2370-2371 | `RTT {n} ms  LOSS {x}%` | debug values (**Q11**: console-parity put them there) |
| Console strip | `console/mod.rs` 1465 | `{op} · {WORD}` (`Lt. Player · HELM`) | debug identity line |
| Console strip | `console/mod.rs` 1418, 1437, 1453, 1458, 1462 | hull %, MW, m/s, fire and breach counts at 0 | raw numbers nobody acts on |
| Console strip | `console/mod.rs` 1448-1449 | `-- MW`, `Power: not simulated in this drill` | dev note |
| Console band | `console/mod.rs` 1147; `console/objectives.rs` 17 | `mm:ss` clock | running clock (kept beside Leave station until Q6) |
| Viewscreen | `console/mod.rs` 1315 | `VIEWSCREEN · {FWD,TARGET}` | raw feed code |
| Consoles | `console/mod.rs` 1113 | `... (its console is not in this drill yet)` | dev note |
| Tooltips | `console/mod.rs` 1008, 1068, 1134; `helm.rs` 102, 148; `kit.rs` 568, 586, 1584, 1626; `science.rs` 206; `gunner.rs` 211, 213 | `(Tab)`, `(M)`, `(Y)`, `(W, S)`, `(X)`, `(scroll)`, `(A, D, R, F)`, `(Q, E)`, `Scan (Q)`, `TAB` | keys in text (a keycap in the tooltip) |
| Helm | `helm.rs` 271, 279; `kit.rs` 1165; `mod.rs` 790 | the ORIENT quaternion and its tip | **Q2**: owner asked, 2026-10-07 |
| Helm | `helm.rs` 247 | `ROLL IN YAW` | dev warning |
| Helm | `helm.rs` 243, 132 | `{axis} rate {x} deg/s`, `Strafe: drag (Z, C, Space, Ctrl). Now ...` | raw numbers, keys in text |
| Tactical, Science, Gunner | `seat.rs` 44, 345; `tactical.rs` 47; `science.rs` 179; `mod.rs` 1398; `gunner.rs` 150, 171, 231 | `T1`, `J{n}` | internal ids (**Q12**) |
| Tactical | `tactical.rs` 72, 166, 175 | `corvette, hostile`, turret letter, `PD` | raw ids, abbreviations |
| Consoles | `drill.rs` 1973 | `Bot` as an operator's name | dev tag |
| Captain | `console/captain.rs` 265 | `laser port: automation` | raw id, internal word |
| Captain | `console/captain.rs` 508, 528 | `... not simulated in this drill` | dev note |
| Engineering | `engineering.rs` 831, 873 | `Not simulated in this drill: ... not built yet` | dev note |
| Engineering | `engineering.rs` 136, 635, 813, 815; `schematic.rs` 875, 994-1053 | `Priority 0: never shed`, `Hot leg ... scram ...`, `factory settings in {n} s`, `Live runs: ...`, `(tap to switch)` | internal values, jargon, helper text |
| Debrief | `drill.rs` 2627-2628 | `Next drill in {n} s` | dev loop wording, wrong for the patrol |
| Debrief | `drill.rs` 2592-2597 | `{h} of {s} ({n} %)`, `{n} MJ` | raw numbers |
| Refusals | `sc-core/src/combat/mod.rs` 749, 750, 756 | `Bad value`, `No such control`, `Not now` | dev error wording |
| Command line | `main.rs` 1584, 1528, 1530 | `Lt. Player`, `Observer`, `{Station} officer` | placeholder names (launches from the command line only) |

Not reached from the menu, so not counted: the `--repairs` launch's `mockup` tag and guide text (`repairs.rs` 365,
374) and `sc-repairs`'s "NOT IN THE ENGINE YET" (`kit.rs` 1193).

## 13. The release bar

The owner, 2026-10-15: "think shipped product". A screen is held to what a player sees in a released game on Steam.

**A player should see**: the game's own names for things; numbers they act on; one thing to do next, said once; their
own name; every screen reachable and every screen leavable; something happening at every moment.

**What would embarrass them**, and is a bug:

- debug values, view labels ("BRIDGE (V)"), internal ids ("T1", "PD"), enum names (MUSTER), raw units nobody acts on;
- placeholder names ("Lt. Player", "automation", "Bot") and dev tags ("(bot)");
- dev notes on screen ("not simulated in this drill", "not built yet", "A bot plays this station");
- abbreviations ("Laser stbd", "Eng");
- unfinished models: placeholder stations, an iris that stands open, empty berths;
- overlapping or cut-off text (the top bar's roster over the title in the browser);
- clicks that miss: a control whose hit area is not what is drawn;
- softlocks: a screen with no way back, a state with nothing to press;
- long dead stretches with nothing to do: the 5 minute undock at 5 m/s;
- key hints as running text on the game's screen.

## 14. The release check (proposed, task 5)

A step in `scripts/check.sh`, built after approval, that fails the build on the bar's checkable half:

1. **Forbidden strings.** `data/ui/forbidden_strings.json` lists patterns (`(bot)`, `(NPC)` unless Q3 keeps it,
   `automation`, `Lt. Player`, `not simulated`, `not built`, `drill` in a title, `stbd`, `TODO`, `placeholder`, a
   bare enum in caps, `RTT`, an IP address). A test runs every screen's UI in the headless client and collects every
   string egui lays out (the text batch), and fails on any match with the screen's name and the string.
2. **Every reachable screen, two sizes.** A scripted walk through every screen the menus and the game can reach (the
   front menu's pages, Single Player, Options' pages, connecting, loading, briefing, walking, each prompt, the first-time
   hint, each console seated, the Esc menu and its confirms, the debrief) at 1280 x 720 and 1920 x 1080, writing
   captures to `docs/screenshots/engine/release/`.
3. **Overlap and clipping.** For each capture, the laid-out text rectangles (from egui's output, not from pixels) must
   not intersect one another, must sit inside the screen and inside their panel, and must not be cut by an ellipsis
   unless the band allows one (CLAUDE.md 10's fixed bands).

A failure names the screen, the size and the strings; the captures are the owner's to look at.

## 15. Pi 5 budget

- **Draw calls**: egui batches by texture; the busiest screen (the Esc menu with the roster over the console) draws
  the console's existing calls plus about 30 for the panel and rows. The briefing, which replaces the console's
  content, is under 60. Estimated against the table's 300 UI calls (`engine-stack`); measured on the Pi in task 4.
- **No blur, no render targets, no shaders**: scrims and gradients are vertex-coloured quads, the ring is a mesh.
- **Textures**: the key art and the loading art at 1280 x 720 (the 3D pass's size), RGBA, about 3.7 MB each as
  uploaded (7.4 MB together), loaded only while a menu or loading is up and freed on boarding, so nothing of them is
  resident in the game. In game: nothing new beyond the font.
- **Text**: one family, two weights, already loaded for the consoles; Ubuntu leaves memory.

## 16. Questions only the owner can settle

| Id | Question | Shots | Recommendation | Answer |
| --- | --- | --- | --- | --- |
| Q1 | Accent: ice (the livery cyan) or amber (today's title colour)? | lobby-after, lobby-after-amber | Ice: far from every state colour | Approved, the owner, 2026-10-16 (survey UI1): "approve" |
| Q2 | Helm's quaternion (asked for on 2026-10-07): keep it at rest, or show it on hover or focus? | briefing-before (it shows through), pause-after (behind the scrim) | On hover and focus, with the heading, pitch and roll at rest | Approved, the owner, 2026-10-16 (survey UI2): "approve" |
| Q3 | "(NPC)" after bots' names (asked for on 2026-10-10): keep the text, or a small crew-AI icon? | hud-before-2 | The faint icon the briefing uses | Approved, the owner, 2026-10-16 (survey UI3): "approve" |
| Q4 | Stations not built yet (Comms, Flight ops): locked cards, or hidden until they exist? | lobby-after | Hidden: a shipped game does not show what it does not have | Approved, the owner, 2026-10-16 (survey UI4): "approve" |
| Q5 | Explore the ship and its Fly key: keep in the shipped menu? | lobby-after, controls-after | Keep Explore, drop Fly from the shipped build | Approved, the owner, 2026-10-16 (survey UI5): "approve" |
| Q6 | HUD: roster and mission clock off the walking HUD (roster in the Esc menu), alert chip only at alert? And the console band's clock? | hud-after, pause-after, console-after | Yes; the band's clock only when a mission has a time limit | Approved, the owner, 2026-10-16 (survey UI6): "approve" |
| Q7 | Briefing as a solid full screen on the console, rather than over it? | briefing-after | Yes | Approved, the owner, 2026-10-16 (survey UI7): "approve" |
| Q8 | Under 1080p text shrinks with the scale rule (Label 13 px at 720p, 8 px in the small embed). Keep it pure, or never draw text under 12 px (panels grow)? | controls-after-985x455, controls-after-1280x720 | A 12 px floor, with a Text size option later | Approved, the owner, 2026-10-16 (survey UI8): "approve" |
| Q9 | Subtitle "Bridge simulator" and a version label instead of "SCS TERN"? | menu-after | Yes | Approved, the owner, 2026-10-16 (survey UI9): "approve" |
| Q10 | Rebinding keys in Controls now, or read-only first? | controls-after | Read-only first | Approved, the owner, 2026-10-16 (survey UI10): "approve" |
| Q11 | F3's round trip and loss (from console-parity): keep as a hidden debug overlay, off in release builds? | none (not on screen at rest) | Off in release builds | Approved, the owner, 2026-10-16 (survey UI11): "approve" |
| Q12 | Track ids (T1, J1) on Tactical, Science and the gunner: keep (as a warship would) or names (Hound, Fighter 1)? | console-after (T1 1.5 km on the viewscreen) | Keep T1 style ids: they are what the crew call out | Approved, the owner, 2026-10-16 (survey UI12): "approve" |

## 17. Not in this change

The consoles' own panels (approved; `console-parity` owns them), sound assets, rebinding, gamepad prompts (the client
reads no pads yet), and localisation.

## 18. The leftovers, built (2026-10-17)

The owner, 2026-10-17, of tasks 3.9, 3.10 and Q6's clock rule: "do this!". Of task 4.1: "disregard pi measurements".

### 18a. Motion: the focus bar and the briefing (task 3.9)

The durations are section 8's, already in `data/ui/ui_style.json` (`motion_ms`). What the table did not say is the
curve, and it is now data beside them: `motion_ease`, one curve per moment, written by `tools/ui/ui_style.py` from the
mockup's `const UI` like the rest. Three curves, none with bounce or overshoot (section 8):

| Curve | t (0..1) to k | Used by |
| --- | --- | --- |
| `linear` | k = t | every fade: a panel, a screen, loading, a prompt |
| `out_cubic` | k = 1 - (1 - t)^3 | the focus bar: it leaves at once and settles on the new item |
| `in_out_cubic` | k = 4t^3 under t = 0.5, else 1 - (-2t + 2)^3 / 2 | the briefing lowering, the console rising |

One module (`sc-client/src/motion.rs`) holds the curves, the clamp of elapsed time over a duration and the slide
between two positions; every timed UI moment calls it (the fades of 3.9's built half included), so no screen keeps
its own arithmetic.

- **The focus bar slides** (90 ms, `out_cubic`). The accent bar and the raised fill are one slab that moves from the
  item that had focus to the item that has it, whatever moved the focus (arrows, the pointer, a click). The words
  change colour at once: only the slab moves. A focus that moves again mid-slide starts from where the slab is, never
  from the old item. The front menu, the Esc menu and Options' page rail.
- **The briefing lowers into the desk** (350 ms, `in_out_cubic`). When the round starts (Countdown ends, or START
  MISSION), the briefing's solid screen slides down out of the console's screen, under the band, uncovering the
  console that was behind it the whole time; it shows Launch in 0:00 with a full bar while it goes and takes no
  presses. The band stays where it is, and the bridge stays lit behind (the law: no cut).
- **The console rising** keeps its curve and reads its 600 ms from `motion_ms.console_rise` instead of a second copy
  in `drill.rs` (`CONSOLE_RISE_S`).
- **Reduced motion**: the approved design has no such option, so none is added here; it would be an Options row,
  which needs the mockup first. Every motion is under 0.4 s and none moves the camera.

### 18b. Console wording (task 3.10)

The consoles' own words, in the approved mockups first (`consoles.html`; Engineering's full schematic is
`power-grid.html`), then the engine. A tooltip's keys are keycaps (section 7): a tip is its words and then its keys in
square brackets, `Next target [Tab]`, and the tip draws each bracket as a keycap after the words (`[hold]` is the
HOLD word, `[wheel]` the mouse wheel's glyph). In the engine a key that is a binding's field is named by the field
(`[@helm.roll_port]`), so the tip shows what the game reads; the mockup writes the key. Two tips named the wrong keys
(the helm's roll on Q and E and its strafe on Z and C, both moved by leave-station): they now come from the bindings.

| Where | Old | New |
| --- | --- | --- |
| Band, crew tab | `LASER PORT`, `LASER STBD` | `PORT LASER`, `STARBOARD LASER` |
| Band, tip | `Your console (Tab)` | `Your console [Tab]` |
| Band, tip | `{op} runs it (M)` | `{op} runs it [M]` |
| Band, tip | `You run it, through {op} (M)` | `You give the orders, {op} carries them out [M]` |
| Band, tip | `Acknowledge (Y)` | `Acknowledge [Y]` |
| Band, tip | `Leave the station (hold E)` | `Leave the station [hold] [E]` |
| Viewscreen | `VIEWSCREEN · FWD`, `· STBD` | `VIEWSCREEN · FORWARD`, `· STARBOARD` (AFT, PORT, TARGET, CHASE as they were) |
| Viewscreen, tips | `Viewscreen: fwd`, `Viewscreen: stbd` | `Viewscreen: forward`, `Viewscreen: starboard` |
| Scanner and plot, tips | `Closer (scroll)`, `Further (scroll)` | `Closer [wheel]`, `Further [wheel]` |
| Helm, tip | `Forward speed: drag (W, S)` | `Forward speed: drag [W] [S]` |
| Helm, tip | `Strafe: drag (Left, Right, Space, Ctrl). Now {x} m/s to starboard, {y} m/s up` | `Sideways and up: drag, now {x} m/s starboard, {y} m/s up [Left] [Right] [Space] [Ctrl]` |
| Helm, tip | `All stop: speed and drift to zero (X)` | `All stop: speed and drift to zero [X]` |
| Helm, tip | `Stick: yaw and pitch (A, D, R, F). Pull back for nose up` | `Stick: yaw and pitch, pull back for nose up [A] [D] [R] [F]` |
| Helm, tip | `Flipper: roll left or right (Q, E)` (engine), `(Z, C)` (mockup) | `Roll: drag left or right [Z] [C]` |
| Helm, tips | `yaw rate 3.2 deg/s` | `Yaw: turning 3.2° a second` (and Pitch, Roll) |
| Helm, ATTITUDE | `ROLL IN YAW` | `NOSE VERTICAL`, with the tip `Nose straight up or down: roll turns like yaw` |
| Tactical, TURRETS | `PD` | `DEFEND` |
| Tactical, tip | `Turret: tap to change mode` | `Auto: fires at the target in its arc`, `Target: fires at the locked target`, `Defend: shoots down incoming missiles`, `Hold: does not fire`, each then `; tap to change` |
| Tactical, tip | `corvette, hostile` | `Corvette, hostile` |
| Gunner | `TAB` (button) | `NEXT` |
| Gunner, tip | `Next target (Tab)` | `Next target [Tab]` |
| Science, tip | `Scan (Q)` | `Scan [Q]` |
| Science, SHIELDS model | `STBD` | `STARBOARD` |
| Engineering, POWER | `EMERG` | `EMERGENCY` |
| Engineering, tips | `Priority 0: never shed`, `Priority: 1 is fed first` | `Always powered`, `Priority: 1 is powered first and cut last` |
| Engineering, REACTOR | `SCRAM` (button) | `STOP` |
| Engineering, REACTOR | `SCRAM` (the core's word when it is down) | `OFF` |
| Engineering, tip | `Scram: tap, then tap again` | `Emergency stop: tap, then tap again` |
| Engineering, tips | `Hot leg {K} · {flow}`, `Hot leg to the chiller: {n}%`, `Cold leg {K}`, `Bypass: {n}%` | `Hot coolant {K} · {flow}`, `Through the chiller: {n}%`, `Cooled coolant {K}`, `Past the chiller: {n}%` |
| Engineering, tip | `Hot leg {K} (band a-b, scram c) · cold leg {K}` | `Hot coolant {K} (safe a-b, stops at c) · cooled {K}` |
| Engineering, tips | `Makeup valve: ...`, `Makeup open ...` | `Top-up valve: ...`, `Top-up open ...` |
| Engineering, tip | `Cables cut: damage control splices them` | `Cables cut: damage control repairs them` |
| Engineering, tip | `Tripped: factory settings in {n} s` | `Tripped: the grid resets in {n} s` |
| Engineering, tip | `Live runs: {a} of the {b} the protection allows` | `Live cables: {a} of {b} before the grid trips` |
| Schematic | `factory settings in {n} s` (under TRIPPED) | `resets in {n} s` |
| Schematic, tip | `Live runs against what the main protection allows; one more trips the grid` | `Live cables of the most the grid carries: one more trips it` |
| Schematic, tips | `The schematic as a circuit board, a board a deck`, `Back to the Engineering console, where this schematic is its side panel` | `The power plan, one board a deck`, `Back to the Engineering console` |
| Schematic, legend | `BOXES (tap to switch)`, `RUNS (tap to switch)` | `POWER BOXES`, `CABLES` |
| Schematic, board | `OFFLINE (DAMAGE)` | `DAMAGED` |

Kept, and why: `T1` and `J1` (Q12); YAW, PITCH and ROLL (the Controls page's own words, what a flight game says);
port, starboard, bow and stern (the ship's sides, as the station names say them); the turret letters D, V, P, S (on
the turret's own dial, beside the station tab that names the gunner); SHORT and NO POWER on the board's legend.

### 18c. The mission clock (Q6)

Q6's rule, approved: the band's clock shows only when a mission has a time limit. Both missions had
`engage_limit_s: 600`, so the clock always showed. Is that limit gameplay? No:

- Nothing tells the player about it: no objective, briefing line or order mentions time, and the drill's objectives
  are destroy the Hound, survive and shoot down both fighters.
- What it does is end a fight nobody can finish (a crew that never closes, a bot run left going) with the enemy
  withdrawing; in the patrol the route goes on after it either way.
- A clock running up from 0:00 with no number to beat is a debug readout (design 13), and a player would read it as
  a deadline that does not exist.

So it is a **safety timeout**, and the data says so: `safety_timeout_s` (600 s, server side, never drawn) replaces
`engage_limit_s`, and a mission may give a separate `time_limit_s` that the crew are told about. Neither mission has
one. The server ends the fight at the earlier of the two (the enemy withdraws, as before). The band shows a clock only
with a `time_limit_s`, and then it counts down what is left, since a limit is read as time remaining; the console's
view carries `limit_s` (none) instead of a `timed` flag, and the mockup's band follows the same rule.

### 18d. Measuring (task 4.1)

Closed without the 1 GB Pi's numbers: the owner, 2026-10-17, "disregard pi measurements". The 8 GB Pi's counts in 4.1
stand as counts.
