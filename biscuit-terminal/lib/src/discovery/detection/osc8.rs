use std::env;

use super::dimensions::is_tty;

/// Detect if the terminal supports OSC8 hyperlinks.
///
/// OSC8 allows embedding clickable URLs in terminal output using
/// escape sequences: `\x1b]8;;URL\x07text\x1b]8;;\x07`
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::discovery::detection::osc8_link_support;
///
/// if osc8_link_support() {
///     println!("\x1b]8;;https://rust-lang.org\x07Rust Homepage\x1b]8;;\x07");
/// } else {
///     println!("Visit: https://rust-lang.org");
/// }
/// ```
pub fn osc8_link_support() -> bool {
    if !is_tty() {
        return false;
    }

    if let Ok(term_program) = env::var("TERM_PROGRAM") {
        match term_program.as_str() {
            "iTerm.app" | "kitty" | "WezTerm" | "Alacritty" | "ghostty" | "warp"
            | "WarpTerminal" | "vscode" | "gnome-terminal" => {
                return true;
            }
            _ => {}
        }
    }

    if env::var("VTE_VERSION").is_ok() {
        return true;
    }

    if env::var("WT_SESSION").is_ok() {
        return true;
    }

    let term = env::var("TERM").unwrap_or_default();
    if term.contains("kitty") || term.contains("wezterm") || term.contains("alacritty") {
        return true;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_osc8_link_support_returns_bool() {
        let _ = osc8_link_support();
    }
}
