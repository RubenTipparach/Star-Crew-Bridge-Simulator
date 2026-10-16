# Proposal: UI art direction, a shipped game's screens around the consoles

## Why

The owner, playing the itch.io build (2026-10-15):

- "you should cleanup the ui so itlooks like a proper game, and not like the vibe coded look you AIs make"
- "I also dont like the controls text in the bottom, makes it look messy", "input controls can be in the options menu",
  and of the key strip: "this text should be remocved from the main screen and any screen", to go in Options "as a
  square panel"
- "you gotta get out of prototype mindset, and think shipped product"
- "buttons and text need to scale with reference pixels at 1920x1080"
- "lighting can only be changed by the captain"
- a "Leave station" button in the console's top bar

Today every screen around the consoles is drawn with egui's defaults: its stock typeface (Ubuntu Light, a different
family from the consoles' Barlow Semi Condensed), rounded pill buttons, a lilac focus on amber buttons over navy, a
centred box floating on a stock planet. The briefing is a see-through layer over a live console. The HUD runs a
mission phase name and a clock across the top with eight crew always on screen, and the browser page adds a strip of
key hints in running text. About 75 strings on player screens are internal names, debug values or dev notes ("(bot)",
"automation", "Lt. Player", "BRIDGE (V)", "Not simulated in this drill"). Design section 1 lists what reads as made
by a machine, screen by screen; section 12 lists every offending string with its file.

## What Changes

- **One art direction for every screen around the consoles** (design 3 to 9): a 1920 x 1080 reference frame scaled
  by min(width / 1920, height / 1080) with groups anchored to the screen's edges; one typeface (the consoles'), four
  type sizes, two weights; a palette of roles with one accent; opaque square panels; an 8 px grid inside a 96 x 54 px
  safe area; keys drawn as keycaps; short motion and UI sound. The consoles keep their approved look (`consoles.html`,
  `console-parity`); only the band's new Leave station button is added to them.
- **Redesigned screens** (design 11), each in the mockup before and after: the front menu, Single Player, Options with
  a Controls page, the briefing with eight crew on a solid screen, the HUD on the bridge, walking prompts, the
  first-time hint, the menu on Esc (Resume, Leave station when seated, Options, Controls, Leave ship), the console
  band's Leave station, a loading screen, and the transitions between them.
- **Options > Controls** (design 10): a square panel with three pages (on foot, at a console, touch) listing the
  client's real bindings as keycaps beside short action names, grouped; the lighting keys under Captain only. The key
  strip leaves every screen (the web build's strip is the web agent's, already being removed).
- **A player-facing text rule** (design 12): no internal names, debug values, placeholder names or dev notes on a
  player's screen, with every current offender listed for the engine pass.
- **A release bar and a release check** (design 13 and 14): what a player should see and what would embarrass them;
  a proposed `scripts/check.sh` step that refuses forbidden strings in the UI's text and shoots every reachable screen
  at 1280 x 720 and 1920 x 1080 for overlapping or clipped text. Proposed, not built.
- **Style in data** (design 9): `data/ui/ui_style.json`, generated from the mockup's tokens by a tool that `--check`s
  it, as `console_style.json` is from `consoles.html`.

## Impact

- `sc-client`: `lobby.rs` (front menu, Single Player, Multiplayer), `options.rs` (pages and Controls), `game_menu.rs`
  (the side panel, Leave station), `drill.rs` (briefing, top bar, prompts, loading, first-time hint), `onfoot.rs`
  (prompt text), `nameplate.rs`, `console/mod.rs` (band and status strip strings), `ui.rs` (the scale function and
  the style's load). The engine's scale function is the web agent's (one function, the same rule).
- Data: `data/ui/ui_style.json` (new, generated), `data/missions/*.json` (titles without "Drill 1:"), the station
  display names (`sc-core` `Station::name`: "Port laser", "Starboard laser").
- Changes it touches: `front-menu`, `lobby`, `game-menu`, `crew-nameplates`, `touch-controls` (the Controls page's
  touch layout is its layout), `objectives-tab`, `single-player`, `console-parity` (the band's button).
- Pi 5 cost (design 15, estimated until measured): the busiest screen adds about 30 UI draw calls to a console's,
  against the table's 300; no blur and no render targets; two 1280 x 720 key-art textures (about 7.4 MB together)
  loaded only on the menus and loading, freed on boarding.
- Mockup: `docs/mockups/ui-art-direction.html`; shots in `docs/screenshots/mockups/ui-art-direction/`.
