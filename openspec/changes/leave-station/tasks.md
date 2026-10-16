# Tasks: leave-station

The owner asked on 2026-10-15 (the proposal).

## 1. Write-up

- [x] 1.1 Proposal, design and spec delta, with what stopped a player getting up (design 1).

## 2. Build

- [x] 2.1 `data/input/bindings.json` and `bindings.rs`, validated at start; tests.
- [x] 2.2 `stand.rs`: the hold, armed only by a press in the seat; tests.
- [x] 2.3 Helm's roll on Z and C, the strafe on the arrows; test.
- [x] 2.4 LEAVE STATION in the title band, its keycap and ring; a click stands; test.
- [ ] 2.5 The touch layer's STAND goes; a tap on LEAVE STATION stands. (Today a tap on LEAVE STATION stands and the
  touch STAND holds E, the same hold; removing STAND waits on `browser-page-fit`'s touch rework of `touch.rs`.)
- [x] 2.6 `consoles.html`: the button, Helm's keys and tip; the parity states; parity passes.

## 3. Gate

- [x] 3.1 The walking playtest: up from a station mid-Engage, walk on.
- [ ] 3.2 The owner looks at the mockup and engine captures of the title band.
