# Proposal: recorded playtests of every seat, and of a retreat and repair

## Why

The owner, 2026-10-23, after getting stuck in the turret seat: "I could t climb down a ladder when I was in turret
seat", then: "Make sure pi 1 runs a recorded test of getting in and out of every seat on the ship. Run a test with a
more powerful ship that can damage your ship. Retreat the player ship and repair it by putting out fires and playing
minigames".

The scripted player (`--playtest`, a capture instrument: CLAUDE.md 4) already walks, sits, stands, fights fires and
plays repair games, but no script visits every seat, and the drill's Hound rarely hurts the Tern enough to set it on
fire. There is no retreat order: a retreat is the helm turning about (FLIP) and running at full speed.

## What Changes

- **`--playtest-seats`** (design 1): the scripted player walks to every station aboard in turn, sits, works its
  console a few seconds, stands and walks to the next, ladders and the gunner's pod included; each seat logs `sat`
  and `stood` (or what stopped it), and the run ends after the last seat.
- **A heavier enemy** (design 2): the Mastiff, a heavy corvette class in `data/enemies.json` that can hurt the Tern, and
  a mission against it, `data/missions/drill-mastiff.json`.
- **`--playtest-retreat`** (design 3): at the helm through the opening exchange, then FLIP and full speed away; up from
  the seat to fight every fire and play every repair game the hits left (with `--all-hands`, so no damage-control
  team does it for them).
- **Recorded runs** (design 4): PI-1 records the seat tour in real time; the cloud session films the retreat.

## Impact

- `sc-client::playtest`: two scripts and two console presses (FLIP, the throttle to full); `drill.rs` presses them;
  `main.rs` takes the flags. Nothing a player does changes.
- `data/enemies.json`, `data/missions/drill-mastiff.json`: new data; the shipped drill is unchanged.
- Cost: none in the game; the runs are test captures.
