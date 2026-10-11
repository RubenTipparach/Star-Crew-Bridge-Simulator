//! `sc-repairs`: the repair mini-games in the engine (openspec/changes/repair-minigames design 8).
//!
//! Each game is ported from its mockup, `docs/mockups/repairs/<id>.js`, which stays the design tool: the same
//! machine, the same move, the same numbers, with its knobs by level in `data/repairs/<id>.json`. The kit is
//! `kit.js`'s engine twin: the job's bar, the cover's screws, the guide card, the input and the frame. The job itself
//! (what a round is worth, a fumble's cost, the server's checks) is `sc_core::repair`, so the menu, the ship and the
//! server agree.
//!
//! The crate draws egui shapes and nothing else: no GL, no SDL, no files. The client hands it a painter and a
//! rectangle; a test hands it nothing and plays a game to the end with its steady hand.

pub mod games;
pub mod icons;
pub mod kit;
pub mod pen;

pub use kit::{Ctx, Game, Input, Options, Runner};
pub use pen::Pen;
