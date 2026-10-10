# Co-op drill, all five stations: cloud captures, 2026-10-10

`sc-server --listen 127.0.0.1:7701 --seed 1` and five `sc-client --headless --connect 127.0.0.1:7701 --station
helm|tactical|engineering|science|captain --bot --shots DIR` clients, debug builds, in a Claude Code cloud session
(lavapipe, no GPU, no Pi): round 0, victory in 69 s. 1920 x 1080, saved as JPEG. Each shot is taken at a fixed time
into its phase (`drill.rs`, `scripted_shots`). A cloud render says what the consoles look like, never how fast they
draw (CLAUDE.md 12). The same drill on a Pi 5 is the confirming run.

| Shot | Shows |
| --- | --- |
| `1-briefing-*` | Muster: the situation, objectives, the station's orders, the crew. Captain: the Science seat still on automation (its bot joined 0.3 s after the shot) |
| `2-countdown-*` | Countdown |
| `3-engage-a-*` | 6 s into Engage: the Hound closing; Science scanning it, the Captain's condition gone to red alert |
| `4-engage-b-*` | 22 s: bolts both ways. Science's shield view: one arrow a bearing, its angle clear of the others |
| `5-engage-c-*` | 40 s: the fight at the Hound's orbit; hits through a face drawn red |
| `6-debrief-*` | Debrief: the result and the numbers |

Engineering is drawn unavailable (dimmed, nothing to press) because the drill has no power grid yet
(`openspec/changes/console-parity` design 10).
