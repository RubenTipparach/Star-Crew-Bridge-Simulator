# Tasks: browser-single-player

The owner, 2026-10-13: "I launched the game in single player where is the FUCKIN GAME!" (the proposal).

## 1. Write-up

- [x] 1.1 Proposal, design and spec delta; `single-player` design 7 and tasks 3.1 and 3.3, and `front-menu` design 2,
  point here.

## 2. Build

- [x] 2.1 `sc-net`: the local link inside `ServerNet` and `ClientNet`, `LocalDoor`, `Dial`; `Session::connect` takes a
  `Dial`; the local link's test (design 8).
- [x] 2.2 `sc-server`: `Listen::Local` and `DrillServer::dial`; silence counted on both clocks (design 4), with its
  three tests (`tests/local_link.rs`).
- [x] 2.3 `sc-client::solo` hosts over the local link, on the desktop and in the browser; the in-process single-player
  test passes over it and leaves no thread behind.
- [x] 2.4 The browser's Beam aboard starts the game, played on foot (the `web && Solo` gate goes).
- [x] 2.5 `tools/web/smoke.mjs` plays into a round; the browser captures beside the desktop's
  (`docs/screenshots/web/single-player/`): the body walking to its station, sitting, the console coming up.
- [ ] 2.6 `scripts/check.sh` and `tools/web/build.sh` pass; the merge deploys it to itch.io.

## 3. Gate

- [ ] 3.1 The owner plays single player on itch.io and signs off the browser captures.
- [ ] 3.2 A Pi 5 client hosting single player over the local link: frame time and memory against today's.
