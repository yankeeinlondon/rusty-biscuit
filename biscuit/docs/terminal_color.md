# Color Depth

To allow crates interested in working with terminals, we will leverage the `termini` crate and expose the following utility functions.


```rust
use std::env;
use termini::TermInfo;

/// Detects the color depth of the terminal.
/// Prioritizes COLORTERM for True Color (16.7M), then falls back to TermInfo.
fn color_depth() -> u32 {
    // 1. Check for True Color via environment variable
    if let Ok(val) = env::var("COLORTERM") {
        if val == "truecolor" || val == "24bit" {
            return 16_777_216;
        }
    }

    // 2. Fallback to terminfo via termini
    TermInfo::from_env()
        .ok()
        .and_then(|info| info.colors)
        .map(|c| c as u32)
        .unwrap_or(0) // 0 indicates no color support or unknown
}

/// Checks if the terminal supports setting the foreground color via ANSI/terminfo.
fn supports_setting_foreground() -> bool {
    match TermInfo::from_env() {
        Ok(info) => {
            // Check if the terminal has the 'setaf' (Set Ansi Foreground) capability
            // In termini, this is represented by the existence of the string capability.
            info.set_a_foreground.is_some() || info.set_foreground.is_some()
        }
        Err(_) => false,
    }
}
```

