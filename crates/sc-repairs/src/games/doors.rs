//! The doors repair (repair-minigames design 2), to be ported from `docs/mockups/repairs/doors.js`.

use crate::kit::{Game, Unported};

/// Not in the engine yet.
pub const PORTED: bool = false;

/// A fresh game.
pub fn new() -> Box<dyn Game> {
    Box::new(Unported("doors"))
}
