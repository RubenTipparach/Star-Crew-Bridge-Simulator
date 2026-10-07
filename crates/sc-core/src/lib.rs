//! `sc-core`: the Star Crew ship simulation (openspec/changes/engine-stack, design section 3).
//!
//! It owns every gameplay rule, stepped at a fixed rate in seconds, with stable ids, seeded
//! randomness and a replay hash, and the data schemas the game and the tools share. It lives in
//! its own crate because the server runs it, the client previews with it and the tests run it, all
//! without a GPU: it never reads a file, opens a socket, calls GL or SDL, or reads the clock
//! (CLAUDE.md 6.2). Callers hand it text and elapsed seconds.

pub mod arena;
pub mod clock;
pub mod data;
pub mod deck;
pub mod replay;
pub mod rng;
pub mod vertex;
