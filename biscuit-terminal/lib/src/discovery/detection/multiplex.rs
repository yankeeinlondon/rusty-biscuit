use std::env;

use serde::{Deserialize, Serialize};

/// Type of terminal multiplexing support available.
///
/// All detected multiplexers support their full capability set (split, resize,
/// focus, tabs). Individual capability fields were removed because every variant
/// always returned `true` for all fields — no version-based detection exists.
///
/// ## Detection
///
/// Detection is based on environment variables:
/// - `TMUX` - Set when running inside tmux
/// - `ZELLIJ` - Set when running inside Zellij
/// - `TERM_PROGRAM` - Identifies terminals with native multiplexing (Kitty, WezTerm, Ghostty)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MultiplexSupport {
    /// No multiplexing support available
    None,
    /// Native multiplexing built into the terminal emulator (Kitty, WezTerm, Ghostty)
    Native,
    /// tmux multiplexer detected
    Tmux,
    /// Zellij multiplexer detected
    Zellij,
}

/// Detects the type of terminal multiplexing support available.
///
/// This function checks environment variables to determine if a multiplexer
/// is active (tmux, Zellij) or if the terminal emulator has native multiplexing
/// capabilities (Kitty, WezTerm, Ghostty).
///
/// ## Detection Order
///
/// 1. **tmux** - Checks `TMUX` environment variable
/// 2. **Zellij** - Checks `ZELLIJ` environment variable
/// 3. **Native** - Checks `TERM_PROGRAM` for terminals with built-in multiplexing
/// 4. **None** - No multiplexing detected
///
/// ## Returns
///
/// A [`MultiplexSupport`] enum variant indicating the detected multiplexer
/// and its capabilities.
///
/// ## Examples
///
/// ```no_run
/// use biscuit_terminal::discovery::detection::{multiplex_support, MultiplexSupport};
///
/// match multiplex_support() {
///     MultiplexSupport::Tmux => println!("Running inside tmux"),
///     MultiplexSupport::Zellij => println!("Running inside Zellij"),
///     MultiplexSupport::Native => println!("Terminal has native multiplexing"),
///     MultiplexSupport::None => println!("No multiplexing support detected"),
/// }
/// ```
pub fn multiplex_support() -> MultiplexSupport {
    if env::var("TMUX").is_ok() {
        return MultiplexSupport::Tmux;
    }

    if env::var("ZELLIJ").is_ok() {
        return MultiplexSupport::Zellij;
    }

    if let Ok(term_program) = env::var("TERM_PROGRAM") {
        match term_program.as_str() {
            "kitty" | "WezTerm" | "ghostty" => {
                return MultiplexSupport::Native;
            }
            _ => {}
        }
    }

    let term = env::var("TERM").unwrap_or_default();
    if term.contains("kitty") || term.contains("wezterm") || term.contains("ghostty") {
        return MultiplexSupport::Native;
    }

    MultiplexSupport::None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_multiplex_support_eq() {
        assert_eq!(MultiplexSupport::None, MultiplexSupport::None);
        assert_eq!(MultiplexSupport::Tmux, MultiplexSupport::Tmux);
        assert_ne!(MultiplexSupport::None, MultiplexSupport::Tmux);
    }

    #[test]
    fn test_multiplex_support_serialize() {
        let mux = MultiplexSupport::Tmux;
        let json = serde_json::to_string(&mux).unwrap();
        assert!(json.contains("Tmux"));
    }

    #[test]
    fn test_multiplex_support_returns_variant() {
        let _ = multiplex_support();
    }
}
