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
- `sc-net` (messages, link, the `str0m` transport, the client session) and `sc-server` (the drill server as a
  library and a binary) built on Pi 1 (tasks 3.1-3.4). Findings while building:
  - `str0m` 0.24.1's default features pull `aws-lc-sys`, a large C build; `default-features = false` with
    `rust-crypto` is pure Rust and built on the Pi with no extra packages.
  - Each peer gets its own UDP socket on a random port, as `str0m`'s `http-post` example does, rather than one shared
    7701 port: no demultiplexing to write, and on a LAN no firewall is in the way. Design 4.2 says 7701; the server
    listens on 7700 TCP only, and the UDP ports are whatever the OS gives.
  - Channels are opened in band (DCEP) by the client with labels, rather than pre-negotiated by id; the server maps
    them by label. Simpler for a first cut; the 8-byte header still rides on the unreliable ones.
  - `tests/lan_drill.rs`: a server and two bot clients over loopback WebRTC win the drill, and win it again with
    10 % loss and 50 ms of delay on the unreliable channels (19 s for both, the server at four times real speed).
- `sc-bot`, the headless bot crew member (`sc-server/src/bot.rs`, shared with the test and, next, the client's
  `--bot`). First real-time run on Pi 1, three processes over the LAN address 192.168.0.210: `sc-server` and two
  `sc-bot`s. Round 0: victory in 70 s; Tern bolts 227/712 hit, Hound bolts 112/344, missiles 5/6, Tern hull 92 of
  120 MJ; round trip 15 ms on one machine, no snapshot lost.
- **Hardening finding.** A bot restarted on Pi 1 could not take the helm for 15 s: the dead process's connection
  still held the seat until WebRTC's consent checks gave up on it. The design's 5 s silence rule
  (`netcode-and-sessions` section 2) was not built; now it is (`SILENT_S`), with a test that a client that stops
  sending loses its seat to automation within 6 s. Bots also claim their station in any phase now, so one that
  joins mid-fight takes over from automation at once instead of waiting for the next Muster.
- The client's drill mode (tasks 4.1-4.3, Pi 1): `sc-client --connect HOST --station helm|tactical [--bot]` opens
  the briefing, the station's console over the bow view and the debrief. Findings while building it:
  - The Hound at fighting range (1.4-1.8 km) is about 15 px tall in the bow view: findable with a bracket, not
    readable. A **target camera** inset in the look band (a camera feed, `bridge-stations` 8.0) keeps it about half
    the inset tall at any range, drawn through the renderer's existing screen quad.
  - The 3D target is 16:9; on the owner's 21:9 monitor it was stretched. The drill's projection now takes the
    window's aspect, so the stretch cancels.
  - The models' winding test caught the burst sphere inside out (it still drew, from its inner far side).
  - The 140-triangle Hound is lit by the probe program's ambient cube built from the sun, so it turns into and out
    of the light as it flies; bolts, missiles and bursts are flat emissive colours.
  - The lobby's Join a crew still does not take an address (task 4.1 says so): `--connect` is the way in for now.
- Demo video recorded on Pi 1 (`docs/videos/2026-10-10-coop-drill-pi5.mp4`, task 5.3 for one machine): the Helm
  client full screen with its bot, a Tactical `sc-bot`, the server, one whole round and the next briefing.
  `wf-recorder` captured about 15 frames a second at 1280 x 720 (the Pi copies and encodes the screen while the
  game draws); the game held 26 FPS median. First takes lost the briefing (the recorder started late) and the
  debrief (`--rounds 1` stops the server as the round ends, so the client never sees the debrief: use `--seconds`).
- **Network finding** (`docs/benchmarks/2026-10-10-coop-drill-lan/pi1.md`): 80 kbit/s down per client, exactly the
  budget, against the design's 75 estimate; 7-8 kbit/s up. The whole snapshot at 20 Hz and a reliable event per
  bolt are the cost. Server tick 0.014 ms median.

## 2026-10-10, afternoon: the crew on foot (design 9)

- The owner asked why the demo had no bots walking the ship, and said a bot should get up and walk to another
  station when it needs to operate one. The drill had left "the walk to the seat" out on purpose (design 8); design 9
  takes it up.
- Bodies follow a path at the walk speed (kinematic, not the walk body's physics): the bridge is one deck and the
  path comes from the walk grid, so a body never needs a collision response here. A player steering their own body
  with W A S D will need the physics body; that is the ship walk's step, not this one.
- `compiled/tern.deck` on Pi 1 was version 4 and the engine now reads 5: rebuilt with `sc-tools deckc` (6 min on the
  Pi 5, 70 MB). The rebuild carries 0 light probes (the probe bake is its own step), so figures take the fallback
  light.
- The bridge's walk grid: 569 cells, built in 300 ms on the Pi 5 with the deck read; every seat-to-seat path found on
  the grid. The muster point to the Helm seat is about 3 s, Helm to Tactical about 2 s with getting up and sitting.
- Bodies cost 10 bytes each in the snapshot (slot, centimetre position, heading, posture, the station walked to).
- The loopback test (`on_foot_a_lone_tactical_bot_walks_to_helm_and_a_second_bot_takes_tactical`): a bot that asked
  for Tactical sits there, gets up after Helm has stood empty 5 s and walks to it; a second bot that asked for Helm
  takes Tactical instead.
- **Two machines** (task 5.1, `docs/benchmarks/2026-10-10-coop-drill-lan/pi2.md`): Pi 2 came back online and ran the
  Tactical bot, built from source there (4 min 45 s for `sc-bot` alone on the 4 GB Pi 5). Six rounds, six victories in
  49-70 s; 84 kbit/s down per client, nothing dropped. A prebuilt bundle served from Pi 1 was refused by Pi 2's session's
  permission check: the way to another machine is git and its own build.
- **Pi 1's Wi-Fi dropped mid-drill** (15:16:26 to 15:18:29, "ssid-not-found"): the server lost all three clients,
  Pi 2's first and the local ones too, since they reach it by its Wi-Fi address. The server held and automation
  finished the round (a defeat). Both bots quit: a closed transport only showed after about 24 s, and `sc-bot` exited
  on it. Now the session counts seconds since anything arrived (`Session::silent_s`), and `sc-bot` treats 5 s of it
  (the server's own `SILENT_S`) or a closed connection as lost and tries again every 3 s as a fresh crew member;
  checked by stopping a server under a bot and starting another (rejoined 3 s after the 5 s silence).
