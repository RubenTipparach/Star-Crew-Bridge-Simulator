# Tasks: sky-behind-solids

The owner asked on 2026-10-15 (the proposal).

## 1. Write-up

- [x] 1.1 Proposal, design and spec delta; the audit of every place stars are drawn (design 0).

## 2. Build

- [x] 2.1 `sc_render::sky_layer_depth` on the sky pipeline; `sc_render::depth` with the outside's lens, far plane and
  reach; the feed and the bridge take them; unit tests.
- [x] 2.2 Stations measure their reach and refuse a draw distance the far plane cannot hold; regression test.
- [x] 2.3 The render test: a ship before a starry sky through a feed and a bridge window, the sky first and last.
- [x] 2.4 Mockups draw their sky layer first; `tools/mockups/skycheck.mjs`.
- [x] 2.5 Captures before and after in `docs/screenshots/engine/sky-behind-solids/` (the render test's frames).
- [ ] 2.6 The patrol's `p6-pair-far` capture of Tamsin Station from 76 km, once the scripted patrol runs again.

## 3. Gate

- [ ] 3.1 The owner looks at the captures.
- [ ] 3.2 The star-lines pass, when built, takes `sky_layer_depth` and joins the render test's `sky_layer`.
