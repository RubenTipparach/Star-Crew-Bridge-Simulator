---
name: owner-survey
description: Put questions for the owner into the Star Crew survey Claude Doc they fill in, never into chat, and fold their answers back into the OpenSpec changes. Use whenever a change, a mockup or a gate has a question only the owner can decide ("Open questions", a choice between mockup options, a gate that needs sign-off), whenever you would otherwise ask the owner anything in a reply, and whenever the owner says they answered ("I filled in the survey", "answered the doc").
metadata:
  author: Star Crew (Claude Code), ported from Pale-Blue-Dot's and Undercity's owner-survey skills
  version: "1.0"
---

# Ask the owner with a survey, never in chat

The owner, in Pale-Blue-Dot, 2026-09-27: "you need some sort of survey form for me to fill out
... that claude doc thing was good for that, maybe that should be a skill", and in Undercity:
"always ask questions inside of survey artifacts". A question asked in chat gets lost in a long
thread. In the survey it sits beside its options, a recommendation and something to look at,
with a place to answer.

The current survey's link is in `docs/design/README.md`, under "Survey". Keep editing it rather
than starting another (CLAUDE.md section 13).

## When to ask, and when not to

The owner's latest word on this (Pale-Blue-Dot, 2026-09-29): "approve, dont ask me questions
unless you have screenshots for me to review lol". So a question goes in the survey only with
something to look at:

- a gate is ready, with its screenshots or mockup;
- a choice best judged by eye, with screenshots or mockup shots of each option;
- a fork in the road that changes every later change (the engine's language was one), with the
  comparison the owner needs beside it.

Everything else is not asked. Take your recommendation, record it in the change's
`## Open questions` table as "recommendation taken (ask only with screenshots)", and say so in
your reply.

## The survey doc

It is a Claude Doc, made and edited through the Claude Docs connector. Load the `docs` skill,
or call the connector's `guide`, before the first docs call, and follow the connector's own
instructions: read from your last revision, and write one section per call.

If the session has no Claude Docs connector, say so in one line. Then publish the same tables
as an artifact page with an answer field per question. Load `artifact-capabilities` first, so
the answers are saved where you can read them back. Move them into the Claude Doc the next time
the connector is available.

- **How to answer** comes first, and says how a blank answer is read: a blank is the
  recommendation accepted, recorded as "(recommendation accepted)" so nobody later mistakes it
  for the owner's own words.
- **One section per change**, in the order of `docs/design/vision.md`'s change table. Each
  section has a one-line lead saying what the owner needs to know, a link to the mockup or
  screenshots it turns on, then one table:

  | # | Question | Options | My recommendation | Your answer |
  | --- | --- | --- | --- | --- |

  - `#` is the stable id from the change's `## Open questions` table. Prefixes in use: E
    engine-stack, M netcode-and-sessions, B bridge-stations, C crew-on-deck, P power-grid,
    L life-support, D damage-control, F ship-frames, W weapons-and-shields, H
    shuttle-bay-and-fighters, N flight-and-navigation, K deck-pipeline, G light-baking, T reference-ship-tern,
    S surface-materials, V wall-panels, U ceilings-and-trims, R floor-panels, A command-suite.
  - A question gives the fact it turns on, with its number and unit: "The hangar holds 1,682.4
    m^3 of air; pumping it down takes 207 seconds, venting it takes 3 seconds".
  - Options are short and separated by " / ".
  - The recommendation is one option, with the reason in a clause.
  - "Your answer" is left blank for the owner.
- **Already decided** comes last: date, decision, where it went, newest first. A question
  moves there once answered, and is never asked again.
- Plain words, from the player's side: what they will see or do, not the code.

## Reading the answers back

1. Read the doc from the revision you last saw (the connector's `read` with `sinceRev`). Do not
   re-read it whole.
2. For each answered row, quote the owner's words into the change it shapes: the proposal's
   Why or the design's decisions. Update the tasks it moves, then `openspec validate --all`.
3. Move the row to Already decided, naming where it went.
4. Reply in the doc's comment thread for any answer you are unsure how to read. Do not guess.

## Do not

- Ask the same question in chat and in the survey.
- Put a question in the survey that the code, the layout checker or a measurement can answer.
  Answer it yourself, and record the finding.
- Ask a question with nothing to look at. Take the recommendation instead.
- Start implementation on an unanswered question that changes behaviour. Planning may go on,
  with the recommendation marked as provisional.
