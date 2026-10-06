---
name: obs-record
description: Record an application window with OBS Studio, convert it to MP4, and optionally drop it into a Google Drive (or any synced) folder. Use when the user wants a video or screen recording of a game, demo, test run or app ("record a flight", "capture a video of X", "make a clip and upload it"). It can launch the program itself, start timing on a line of its output, stop after a duration or on another output line, and records at the window's full size unless told to scale it down.
metadata:
  author: Pale Blue Dot (Claude Code)
  version: "1.1"
---

## In Star Crew

Copied from Pale-Blue-Dot (`.claude/skills/obs-record` at `5fcf83a`, 2026-10-04); provenance in
`.claude/skills/PROVENANCE.md`. The recorder (`scripts/obs_record.py`) is general: it records
any window on the owner's Windows machine with OBS. Everything below this note is the source's
text, kept so a later copy can be diffed; its "Pale Blue Dot recipes" are examples of how a
recipe section looks, not commands for this repository.

- Star Crew has no game build yet. When `sc-client` exists, add a "Star Crew recipes" section
  here (the exe, the window, a start line in its output, how to open a scripted scene) and
  leave the Pale-Blue-Dot recipes as they are.
- A cloud session cannot run OBS. It makes screenshots of mockups with
  `tools/mockups/shoot.mjs` instead (CLAUDE.md section 11); videos of the running game are the
  owner's to record with this skill.
- Smoothness is judged on the Pi 5 itself (CLAUDE.md section 2). A desktop recording shows what
  the game looks like, not how it runs on the floor hardware.

# Record a window with OBS

`scripts/obs_record.py` does the whole job in one command. It restores everything it changes:
- OBS's video settings, current scene and recording format;
- the temporary scene it creates;
- OBS's WebSocket server switch;
- OBS itself: if OBS wasn't running, the script starts it and closes it again.

## Before the first run on a machine

- **OBS:** OBS Studio 28 or later, opened at least once. The obs-websocket password must be set in **Tools → WebSocket Server Settings**; the script reads it from OBS's config and never prints it.
- **Python packages:** `pip install obsws-python imageio-ffmpeg`. imageio-ffmpeg bundles ffmpeg, so no separate install is needed.
- **WebSocket switch:** if the WebSocket server is switched off and OBS is closed, the script turns it on for the run and back off after. If OBS is **open** with the server off, the script stops and says so. Ask the user to close OBS or enable the server; do not edit the config under a running OBS, because OBS overwrites it on exit.

## Usage

```bash
python .claude/skills/obs-record/scripts/obs_record.py \
  --launch "<program and args>" --window-exe <program>.exe \
  --start-on "<regex in its output>" --duration <seconds> \
  --fps 60 --out <name>.mp4 [--copy-to "<folder>"] [--contact-sheet] [--check-cadence]
```

- **Target:** `--window-exe` (matches the executable) or `--window-title` (substring of the title). Without `--launch`, the window must already be open.
- **Timing:** recording starts once the window is captured. `--duration` counts from the `--start-on` line when given, otherwise from the start. `--stop-on` ends it early on an output line, and `--max-seconds` is a hard cap.
- **Loading screens are cut automatically.** Flat-colour frames at the start (a white or black window while the program loads) are detected and removed with a frame-exact re-encode. `--keep-blank-lead` keeps them.
- **Size:** the canvas takes the window's own size and records at **full size by default**; the owner wants footage at full size. `--scale 0.5` records at half, only when asked.
- **Program path:** a relative `--launch` path is resolved from the folder you run the script in (or `PATH`), and the program then runs in its own folder. A path that does not exist stops the script before OBS starts.
- **Output:** OBS records MKV (safe if anything crashes) and the script remuxes it to MP4 without re-encoding. `--contact-sheet` also writes `<out>_frames.png` with six frames across the video. Use it to **look at the result before you call it done**: a menu or dialog over the scene, a black capture or the wrong window are all easy to miss otherwise.
- **Upload:** `--copy-to` copies the MP4 into a folder. For Google Drive, use the Google Drive for Desktop folder (for example `G:/My Drive/<folder>`); Drive for Desktop uploads it. The Drive connector's create-file tool takes file contents inline, which cannot carry a video, so confirm the upload by searching Drive for the file title instead.

## Before every recording

- **Nothing heavy running.** A cargo build or a benchmark (including another Claude session's on the same machine) steals frames and skews the cadence. Ask any other session to pause first, and tell it when the recording is done.
- **The code you mean to film.** `git fetch` and check the branch holds the work the owner wants to see, then rebuild. A recording of a stale build is wasted.
- **No other copy of the program open.** The script refuses, because OBS cannot tell two windows of one program apart.

## Troubleshooting

Run the script in the foreground, or with `python -u` and its output sent to a file. Do not kill it with an outer timeout: a killed run cannot clean up. The next run undoes a killed run's WebSocket change from its backup file, `config.json.obs-record-backup`.

- **"OBS Studio did not properly shut down / Run in Safe Mode?"** OBS was force-closed. Safe Mode disables WebSockets, so the script cannot connect. The script clears OBS's stale markers in `%APPDATA%/obs-studio/.sentinel` before it launches OBS, and closes OBS politely. Never start OBS with `--minimize-to-tray`: a tray-only OBS cannot be closed politely.
- **"OBS is already running" dialog** while no OBS window exists: a dead `obs64.exe` is still listed with status "Unknown" (seen after a force-close during a capture). It blocks new instances until it clears; if it has not cleared, a reboot does it.
- **"the captured window reports no size"**: the window is minimized or never drew a frame.

## Pale Blue Dot recipes

The release app is `target/release/pbd-app.exe`, and its window belongs to `pbd-app.exe`.
- **Open a named world:** a plain launch opens the saves menu. Pass `--world <name>` to open a world directly. The world's save lands in `target/release/saves/<name>`; delete it after a throwaway run.
- **Record with `--no-vsync` and `--time 10`**, like `tools/perf_suite.py`. With vsync on, the game's frames arrive unevenly on a laptop (RTX 3060 Laptop, 2026-09-25: 60 frames a second on average, but 23% of them over 17.5 ms), and the video repeats frames: cloud-hop filmed at a median of 56 new frames a second, 12 of 43 seconds at 59+. Without vsync the game renders well past 60 and OBS picks up a new frame on nearly every tick: median 59, 28 of 43 seconds at 59+. `--time 10` pins the hour to daylight.
- **Loading:** the game's window is plain white for about 4 s while it builds the planet; the automatic trim removes it.
- **Tour to the far side of the planet:** `--launch "target/release/pbd-app.exe --tour --world obs-demo" --window-exe pbd-app.exe --start-on "weather spun up" --duration 37.5`. The tour cruises at 600 m/s, 1 km up, and `--verify-flight` measures a full lap at 64.7 s, so the far side is about 34 s after the flight starts.
- **Cloud fly-through (the scenic route):** into a cloud bank at the cloud's middle height, then low over the land to a landing. Use a fresh world, because a saved one skips the weather spin-up:
  ```bash
  rm -rf target/release/saves/scenic-rec
  python .claude/skills/obs-record/scripts/obs_record.py \n    --launch "target/release/pbd-app.exe --route scenic --world scenic-rec --time 10 --no-vsync" \n    --window-exe pbd-app.exe --start-on ROUTE_LIFTOFF --stop-on ROUTE_COMPLETE \n    --duration 90 --fps 60 --out output/scenic.mp4 --contact-sheet --check-cadence
  rm -rf target/release/saves/scenic-rec
  ```
  Check the game log (`output/scenic.log`) for `scenic route: into N m of cloud` to confirm it found cloud. The flight is about 50 s. The cadence check undercounts inside the cloud (near-uniform frames), so judge those seconds by eye.
- **The other scenarios** launch the same way with the perf suite's arguments (`tools/perf_suite.py` `SCENARIOS`): `--route clouds` (cloud-hop, low through broken cloud), `--route scenic --rain 1` (storm), `--route far-side`, and `--walk-distance 1000` (start on `WALK_START`, stop on `WALK_DONE`; the walk quits by itself).
- **Drive:** copy to `G:/My Drive/dungeon-crawler-2026/` as `pale-blue-dot-<what>-<height>p60.mp4`, for example `pale-blue-dot-scenic-cloud-flythrough-900p60.mp4` for a 1440x900 window.
- **Timing a new route:** get the duration from `--verify-flight` or from the route's own numbers, not by guessing.
