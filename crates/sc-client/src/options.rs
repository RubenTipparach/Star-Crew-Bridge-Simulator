//! The player's options (openspec/changes/front-menu design 4), kept in `settings/options.json` between sessions.
//!
//! It lives in the client because it is the client's: how the window is shown. It is player data, so a file that does
//! not parse is repaired to the defaults with a warning, never a stop (CLAUDE.md 6.5).

use serde::{Deserialize, Serialize};

/// Where the options are kept.
pub const OPTIONS_FILE: &str = "settings/options.json";

/// How the game's window is shown.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum Display {
    /// The whole screen (the default, and the Pi's).
    #[default]
    Fullscreen,
    /// A window on the desktop.
    Window,
}

/// Everything Options sets.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(deny_unknown_fields)]
pub struct Options {
    /// Full screen or a window.
    #[serde(default)]
    pub display: Display,
}

impl Options {
    /// The options in `text`, or the defaults and why when it does not parse.
    pub fn parse(text: &str) -> (Self, Option<String>) {
        match serde_json::from_str(text) {
            Ok(o) => (o, None),
            Err(e) => (Self::default(), Some(format!("{OPTIONS_FILE}: {e}; using the defaults"))),
        }
    }

    /// The kept options: the defaults when there is no file, repaired when it is damaged.
    pub fn load() -> Self {
        let Ok(text) = std::fs::read_to_string(OPTIONS_FILE) else { return Self::default() };
        let (o, warning) = Self::parse(&text);
        if let Some(w) = warning {
            eprintln!("sc-client: {w}");
        }
        o
    }

    /// Keep them for the next session (a failure is said, not fatal).
    pub fn keep(&self) {
        let r = std::fs::create_dir_all("settings")
            .and_then(|()| std::fs::write(OPTIONS_FILE, serde_json::to_string_pretty(self).unwrap_or_default() + "\n"));
        if let Err(e) = r {
            eprintln!("sc-client: {OPTIONS_FILE}: {e} (the options are not kept)");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_damaged_options_file_loads_the_defaults_and_says_why() {
        let (o, warning) = Options::parse("{\"display\": \"sideways\"");
        assert_eq!(o, Options::default(), "player data is repaired, never a stop (CLAUDE.md 6.5)");
        assert!(warning.is_some_and(|w| w.contains(OPTIONS_FILE)), "the warning names the file");
    }

    #[test]
    fn a_window_is_kept_and_read_back() {
        let o = Options { display: Display::Window };
        let (back, warning) = Options::parse(&serde_json::to_string(&o).unwrap());
        assert_eq!((back, warning), (o, None));
        assert_eq!(Options::parse("{}").0, Options::default(), "a missing field takes its default");
    }
}
