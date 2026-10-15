# Tasks: crew-chat

- [x] 1.1 Proposal, design and spec delta from the First Mission Gate's steps 1 and 2 (2026-10-11).
- [x] 1.1a The owner's 2026-10-14 ask folded in: the kinds (ORDER, ACK, INTENT, REPORT), the vocabulary as data, who
  says what and never invented, the bot captain's orders, the pacing, the log window (2026-10-14).
- [ ] 1.2 The mockup (`docs/mockups/mission-room.html`, states 1, 3, 4, 7 and 8) approved by the owner.
- [x] 2.1 The messages (`ClientMsg::Chat`, `ServerMsg::Chat`, `PROTOCOL` 14), the server's kept lines sent to a late
  joiner, the drill's lines (automation, damage control, the computer), with tests.
- [x] 2.1a `data/crew/chat.json` and its checks at load, part of the drill's data.
- [x] 2.2 What a bot says, and the bot captain's orders, with tests.
- [x] 2.2a The client prints each line it receives (`sc-client: chat: ...`) until the screens are built.
- [ ] 2.3 The room's CHAT column, the in-game strip, the log (T) and the field, at parity with the approved mockup.
