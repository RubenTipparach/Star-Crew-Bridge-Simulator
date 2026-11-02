# Design: the Concord skin

Status: **mockup approved** (owner, 2026-11-02: "ui implementation looks good"). The mockups are
`docs/mockups/concord-ui.html` and `docs/mockups/consoles-concord.html`; the engine work (tasks 3) is next.

Source: Fleet Command at `a45a1bb`, `openspec/changes/console-skins/design.md` ("Concord: LCARS's principles,
Starfleet's palette", "The kit") and the `concord` skin in `docs/mockups/console-skins.html`. Fleet Command's own
sources for the research are LCARS's Wikipedia article and Bracer Jack's *LCARS Manifesto* and *Creating a Coherent
LCARS Interface*; its palette was sampled by k-means from Starfleet Command's and Starfleet Academy's screens (the
screenshots are not committed there or here).

## 1. The language, as Fleet Command wrote it

LCARS was designed by Michael Okuda for Star Trek's ship interfaces; the look is minimal so that it reads as far
beyond our own technology. Concord takes its principles, in our own words, and **none of its artwork, its typeface,
its numbers decor or its colour set** (LCARS is a studio's copyrighted design; CLAUDE.md section 15, "take the shape,
not the text"):

1. **The sweep is the language.** One shape, an elbow that joins a thick bar to a thin one, frames every group of
   controls. A frame always changes thickness at a turn, thick to thin or thin to thick, never the same.
2. **The rounded cap ends a bar**, and because it ends something it reads as a button. Buttons are pills; a button
   whose label is long grows a flat side and keeps one cap (a tab).
3. **Break the frame into buttons** rather than adding buttons inside it, unless a segment is too thin to touch. A
   frame that only frames is waste. Buttons in one frame all look alike.
4. **Flat and vector.** No bevel, emboss or painted light. Colour glows on black.
5. **Colour with meaning, five at most on a screen**, each colour one role. Two text colours: normal and highlight.
6. **Three type sizes**: title, sub-header, data. One typeface, condensed capitals, labels right-aligned.
7. **Spacing on two constants**: the gap between frames and the gap inside a frame.
8. **The interface never rearranges itself under the hand.** A new task is a new screen; changes are quick fades
   under 150 ms, never cross-fades or slides.
9. **Icons are open circles and arcs in one even line. Nothing pointed** (Fleet Command: from the saucer and the
   circle of species).

Most of these are Star Crew's rules already (CLAUDE.md section 10: fixed panels, colour means state, the main action
the biggest control, glance first). Concord gives them one shape.

## 2. The palette

Fleet Command's sampled colours, kept exactly, so the two games' Concord is one Concord:

| Name | Hex | Role (one meaning each) | Replaces in Star Crew (`ui-art-direction` tokens) |
| --- | --- | --- | --- |
| Deep | `#1b2a6b` | Idle frames, unlit segments, tracks | `line`, `line2`, `key`, `key_edge`, `raised` |
| Signal | `#2f62e0` | Live frames and keys | the panel edges, the keys' faces |
| Sky | `#3fa9f0` | Readouts, live data, focus | `accent` `#6fd3ff` |
| Accord | `#2fbf5a` | Ready, armed, agreed, OK | `ok` `#3ddc84` |
| Caution | `#f0b43c` | Warning, yellow alert | `warn` `#ffc542` |
| Alert | `#e8352b` | Red alert and danger only, never decoration | `danger` `#ff4757` |
| Lilac | `#b7a6f2` | Highlighted text, a pressed key | `accent_alt` `#f2a046` |
| Text | `#eef3ff` | Normal text | `text` `#e8eef6` |
| Black | `#000000` | The ground: everything glows on black | `ground` `#070a0f`, `panel` `#0c121a` |

`dim` (`#98a8bc`) becomes Lilac at 70% where a second text colour is needed, and `faint` goes: Concord has two text
colours. Labels on a lit key are black, as on a back-lit panel.

**Five at most on a screen** (rule 5). A screen at rest uses Deep, Signal, Sky, Text and one state colour; Caution
and Alert appear only when something is wrong, which is when they must be seen.

**The stations' role colours** (`roles`: helm blue, tactical red, engineering amber, science green and so on) were
a second colour system. Under Concord a station is told apart by its **icon and its name**, and its role colour
survives only as the cap at the end of its console's title band and the tint of its icon. Red and green are kept
for state (Alert, Accord), so Tactical's role red becomes Lilac and Science's green becomes Sky, and no station
colour can be read as a warning. Recommendation taken (ask only with screenshots); question CS2 shows it.

## 3. Type

**Antonio** (Vernon Adams, SIL Open Font License, on Google Fonts), the face Fleet Command's Concord uses: tall
condensed capitals that fit a label in a narrow pill. It replaces Barlow Semi Condensed everywhere. The engine
inlines it as Barlow is today (`assets/fonts/antonio/`, two weights: 600 and 700, about 100 kB each, OFL text beside
them).

Three sizes on the 1920 x 1080 reference (rule 6), from `ui-art-direction`'s four:

| Size | px | Was | Used for |
| --- | --- | --- | --- |
| Title | 40 | title 40 | screen titles, the objective, a console's title band |
| Sub-header | 28 | body 28 | menu items, key labels, card text |
| Data | 20 | label 20 | small labels, values, the version |

The wordmark (72) stays art, not type (`ui-art-direction` design 4). Labels are right-aligned on keys and in title
bands (LCARS's habit); running text (the briefing, a hint) is left-aligned. Capitals for labels and titles; the
briefing's body and chat stay in sentence case, which Antonio sets well at 28 px.

**Legibility at the itch.io embed** (985 x 455, 0.42 of the reference): Antonio at 20 px draws at about 8.5 px
there, as Barlow did. The mockup's shots at 985 x 455 are the check; if a 20 px label does not read, the floor rises
to 24 px for that screen size only (question CS3).

## 4. The kit

Every screen draws only these pieces, by name (Fleet Command's one-kit rule): the look lives in one place, and no
screen draws its own frame or button.

| Piece | Shape in Concord | Used by |
| --- | --- | --- |
| `frame.panel` | The sweep: a thick bar down the left, outer-rounded elbows into thin arms along the top and bottom, each arm ending in a small Sky cap | every menu panel, a console's panels, the Esc menu, the briefing |
| `frame.inset` | A thin Deep rounded outline with one Sky tick | text fields, lists, readout wells |
| `button` (idle, focus, pressed, off) | **The pill**: round caps both ends, Signal; focus Sky; pressed Lilac; off Deep with Text at 45% | every key |
| `button.primary` | The pill at the primary height (`sz.primary_h`), Sky, its label black | Beam aboard, Join, Start mission |
| `tab` (idle, active) | A pill that grew a flat side: one cap, Deep; active Accord | Options' tabs, the console's look band |
| `bar.track`, `bar.fill`, `bar.fill.alert` | A Deep rounded track, a Sky fill, an Alert fill | loading, hull and shield readouts, progress |
| `strip.title` | A tall rounded block sweeping into a thin bar with a cap: the console's title band and a menu's header | consoles, menus |
| `keycap` | A small pill outlined in Signal, its letter in Text | prompts, hints, Controls |
| `card.objective` | A short sweep (thick left bar, thin top arm) round the objective | the HUD on foot |
| `divider` | A thin Deep bar with Signal caps | between stacked rows |
| `lamp` (green, yellow, red) | A disc in Accord, Caution or Alert in a Deep ring | alert state, readiness |
| `pip` | A small pill, lit Sky or unlit Deep | crew seats in the room finder, rounds |
| `crest` | The Concord mark: five worlds round an open circle crossed by one orbit (Fleet Command's, original) | the front menu, loading |
| `icon.*` | Open circles and arcs, one even 3 px line, nothing pointed | every icon (`ui-art-direction`'s set redrawn) |

Pieces are vector, drawn by the engine's `vg` kit (no images to load), so they scale to any window and cost no
texture memory.

## 5. The screens

The layouts, positions and sizes stay as `ui-art-direction` approved them (rule: a skin swaps the look, never the
layout). What each screen becomes:

| Screen | `ui-art-direction` state | In Concord |
| --- | --- | --- |
| Front menu | `menu` | The wordmark, the crest beside it; the items as pills down a Signal sweep, the focused one Sky; the version bottom right in Data |
| Single Player | `lobby` | The officer card in a sweep; rank and name in insets; the station picker as tabs with icons; Beam aboard the primary pill |
| Multiplayer, the room finder | `browser` | The crew list in a sweep: rows split by dividers, the phase as a lamp (Accord muster, Alert fight, Deep debrief) with its word, seats as eight pips; Join the primary pill; Create crew a pill; the code in an inset beside its own Join pill |
| Options, Controls | `controls` | Tabs along a sweep's top arm; rows with right-aligned labels; keycaps as Concord keycaps |
| Briefing | `briefing` | A sweep round the mission text; Start mission the primary pill |
| HUD on foot | `hud`, `prompt`, `hint`, `compass`, `minimap` | The objective in a `card.objective`; prompts as a keycap and a Sub-header label; the hint in a small sweep; the compass ticks in Sky on a Deep arc; the minimap's frame a sweep |
| Esc menu | `pause`, `crewname` | A sweep holding pills (Resume, Options, Leave station, Return to menu); the crew's name with its pencil and the join code in insets |
| Profile, name | `profile`, `name` | Insets for the name, the dice a round pill, rank as tabs |
| Loading | `loading` | The crest, "Boarding the Tern" in Sub-header, the bar as `bar.track` and `bar.fill` |
| A console's frame | `console` | The title band is `strip.title` with the station's role cap; the status strip a thin bar with lamps; each of the four panels a `frame.panel`; keys pills. The instruments inside keep their drawings in the new palette |

## 6. Consoles: the frame now, the instruments with their mockup

The consoles' instruments (`docs/mockups/consoles.html`) are each station's approved specification, held by the
parity check (`console-parity`). This change gives them Concord's frame, keys and palette; their drawings (the
lever, the scanner, the shield ring, the faders) keep their shapes. The order:

1. `consoles.html` takes Concord's tokens and kit (its `UI` tokens and `vg` pieces), so the mockup stays the
   specification; its shots are retaken.
2. The engine's consoles follow, and the parity check passes against the new mockup.

## 7. In the engine

- `crates/sc-client/src/look.rs` holds the tokens; it takes Concord's palette and three sizes.
- The kit pieces are functions in the `vg` kit, one per piece, named as in section 4; screens call them and draw no
  frame or button of their own.
- `tools/ui/fonts.py` and `tools/ui/console_style.py` check Antonio and the tokens against the mockups, as they
  check Barlow and today's tokens.
- **Costs**: two font files (about 200 kB in the web build's download), no textures, no new draw calls beyond the
  shapes drawn today (a sweep is one path, as a rounded panel is now).

## 8. Open questions

Asked in survey 2 with the mockup's screenshots (CLAUDE.md section 13).

| # | Question | Options | Recommendation | Status |
| --- | --- | --- | --- | --- |
| CS1 | The Concord skin on every screen, as the mockup draws it | Approve / change it (say what) | Approve | **Approved** (owner, 2026-11-02: "ui implementation looks good") |
| CS2 | The stations' role colours: only the title band's cap and the icon's tint, red and green kept for state | As drawn / keep the role colours wider | As drawn: colour means state everywhere | Asked, with shots |
| CS3 | Antonio's floor at the itch.io embed: 20 px (8.5 px on screen) | Keep 20 / raise to 24 at that size | Keep 20 unless the 985 x 455 shots do not read | Recommendation taken (ask only with screenshots); the shots go with CS1 |
