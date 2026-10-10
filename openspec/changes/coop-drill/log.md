# Log: the first co-op drill

Newest last. What was done, what was found, on which machine and commit.

## 2026-10-10

- Owner's ask (proposal). Pi 1 (Pi 5 8 GB, 192.168.0.210) leads; Pi 2 (Pi 5 4 GB, `tom-lander-4`,
  192.168.0.216) reported in, then went offline; the owner moved the second seat to the Mac ("pi2 might be offline,
  so just carry on between this pi and the mac").
- The home internet link ran at about 1.5 KB/s with 50 % packet loss to the internet all afternoon while the LAN was
  fine (2 ms to the router). Consequences: big downloads (crates, toolchains) are slow, so the Pis share one build
  over the LAN; the benchmark PR (#10) went up with three JPEG captures instead of twelve PNGs.
- Write-up first (CLAUDE.md 4): proposal, design, tasks, spec delta.
- `sc-core::combat` and `::automation` built (tasks 2.1-2.3) on Pi 1. Sixteen tests, among them: the hit
  chance predicts the hit rate (4,000 bolts at a predicted 50 %, observed inside 47-53 %), two bots win, the same
  seed gives the same hash, automation never fires a missile.
- **Balance finding.** With the Hound on the twin pulse cannon of `weapons-and-shields`, the Tern could not lose:
  the Hound's hits (about 1.9 MJ/s) stayed under the shields' 3 MJ/s regeneration, so the hull was never touched, and
  automation alone won in about 115 s. Two placeholders fixed it, both in data: the Hound carries a heavy pulse cannon
  (2.6 MJ a bolt, 3 a second) standing in for its Lance missiles, which are not built; and its orbit's axis leans
  50 deg, so it passes above and below the Tern's plane and only a helm keeps all four turrets bearing. Three seeds
  each, release build on Pi 1:

  | Helm | Tactical | Result | Engage | Tern hull left |
  | --- | --- | --- | ---: | ---: |
  | bot | bot | 3 victories | 60-66 s | 75-103 of 120 MJ |
  | bot | automation | 2 victories, 1 defeat | 125-128 s | 0-43 MJ |
  | automation | bot | 3 defeats | 267-315 s | 0 |
  | automation | automation | 3 defeats | 296-385 s | 0 |

  Tactical alone fires no missile: without a helm the bow is never on the Hound, which is the co-op point of the
  drill. Tests pin the first and last rows.
