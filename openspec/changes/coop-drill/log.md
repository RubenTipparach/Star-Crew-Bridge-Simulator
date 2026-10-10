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
