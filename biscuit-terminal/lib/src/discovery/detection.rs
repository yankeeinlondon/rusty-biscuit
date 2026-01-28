use std::env;

use serde::{Deserialize, Serialize};
use terminal_size::{Height, Width, terminal_size};
use termini::{NumberCapability, StringCapability, TermInfo};

/// The type of image support (if any) of a terminal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ImageSupport {
    None,
    /// the highest quality image support comes from the
    /// [Kitty Graphics Protocol](https://sw.kovidgoyal.net/kitty/graphics-protocol/).
    ///
    /// This is now supported in:
    ///
    /// - Kitty
    /// - WezTerm
    /// - Warp
    /// - iTerm2
    /// - Ghostty
    /// - Konsole
    /// - wast
    Kitty,
    /// one of the earlier image formats but slowly being phased out,
    /// even it's originator iTERM2 now supports the Kitty protocol.
    ITerm,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TerminalApp {
    AppleTerminal,
    Contour,
    Foot,
    GnomeTerminal,
    Kitty,
    Alacritty,
    Wezterm,
    Konsole,
    ITerm2,
    Warp,
    Ghostty,
    Wast,
    VsCode,
    Other(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ColorDepth {
    /// no color support
    None,
    /// 8 colors
    Minimal,
    /// 16 colors (8 normal plus "bright" variants)
    Basic,
    /// 256 color palette (8 bit)
    Enhanced,
    /// 16 million colors (24 bit)
    TrueColor,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ColorMode {
    /// the background color is light, and text characters must be dark
    /// to provide adequate contrast
    Light,
    /// the background color is dark, and text characters must be light
    /// to provide the adequate contrast
    Dark,
    Unknown,
}

/// Represents support for various underline style variants.
///
/// Modern terminals (Kitty, WezTerm, Alacritty, etc.) support extended underline
/// styles using SGR sub-parameters (e.g., `\e[4:3m` for curly underlines).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnderlineSupport {
    /// Straight/single underline (`\e[4:1m` or `\e[4m`).
    pub straight: bool,
    /// Double underline (`\e[4:2m`).
    pub double: bool,
    /// Curly/squiggly underline (`\e[4:3m`) - commonly used for LSP errors.
    pub curly: bool,
    /// Dotted underline (`\e[4:4m`).
    pub dotted: bool,
    /// Dashed underline (`\e[4:5m`).
    pub dashed: bool,
    /// Whether underlines can be colored independently (`\e[58:2::R:G:Bm`).
    pub colored: bool,
}

/// Represents the type of terminal multiplexing support available.
///
/// Terminal multiplexers allow splitting terminal windows into multiple panes,
/// managing persistent sessions, and providing advanced navigation features.
///
/// ## Detection
///
/// Detection is based on environment variables:
/// - `TMUX` - Set when running inside tmux
/// - `ZELLIJ` - Set when running inside Zellij
/// - `TERM_PROGRAM` - Identifies terminals with native multiplexing (Kitty, WezTerm, Ghostty)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MultiplexSupport {
    /// No multiplexing support available
    None,
    /// Native multiplexing built into the terminal emulator
    ///
    /// Supported by:
    /// - Kitty (GPU-accelerated layouts: splits, stack, grid, tall, fat)
    /// - WezTerm (multiplexer domains with SSH, WSL, local support)
    /// - Ghostty (native platform integration)
    ///
    /// Note: Native multiplexing typically loses sessions on terminal close,
    /// unlike tmux which provides persistent sessions.
    Native {
        /// Whether the terminal can split the current window/pane horizontally or vertically
        split_window: bool,
        /// Whether the terminal can resize panes
        resize_pane: bool,
        /// Whether the terminal can change focus to another pane
        focus_pane: bool,
        /// Whether the terminal supports multiple tabs or windows
        multiple_tabs: bool,
    },
    /// tmux multiplexer detected
    ///
    /// tmux is the standard terminal multiplexer for persistent sessions.
    /// Features include:
    /// - Horizontal/vertical splits
    /// - Pane resizing
    /// - Session persistence (detaches survive terminal close)
    /// - Multiple windows per session
    ///
    /// Configuration: `~/.tmux.conf`
    Tmux {
        /// Whether tmux can split windows horizontally or vertically
        split_window: bool,
        /// Whether tmux can resize panes
        resize_pane: bool,
        /// Whether tmux can change focus to another pane
        focus_pane: bool,
        /// Whether tmux supports multiple windows (tabs) within a session
        multiple_windows: bool,
        /// Whether tmux sessions persist after closing the terminal
        session_persistence: bool,
        /// Whether tmux supports detaching and reattaching to sessions
        detach_session: bool,
    },
    /// Zellij multiplexer detected
    ///
    /// Modern multiplexer written in Rust with advanced features:
    /// - Layout system with KDL configuration
    /// - Session resurrection
    /// - WebAssembly plugins
    /// - Floating panes
    ///
    /// Configuration: `~/.config/zellij/config.kdl`
    Zellij {
        /// Whether Zellij can split windows horizontally or vertically
        split_window: bool,
        /// Whether Zellij can resize panes
        resize_pane: bool,
        /// Whether Zellij can change focus to another pane
        focus_pane: bool,
        /// Whether Zellij supports multiple tabs
        multiple_tabs: bool,
        /// Whether Zellij sessions can be resurrected after closing
        session_resurrection: bool,
        /// Whether Zellij supports floating panes
        floating_panes: bool,
        /// Whether Zellij supports detaching and reattaching to sessions
        detach_session: bool,
    },
}

/// Detect the terminal's color depth capability.
///
/// Detection strategy:
/// 1. Check `COLORTERM` environment variable for "truecolor" or "24bit"
/// 2. Query terminfo `MaxColors` capability
/// 3. Default to `ColorDepth::None` if detection fails
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::discovery::detection::{color_depth, ColorDepth};
///
/// match color_depth() {
///     ColorDepth::TrueColor => println!("24-bit color (16M colors)"),
///     ColorDepth::Enhanced => println!("256 colors"),
///     ColorDepth::Basic => println!("16 colors"),
///     ColorDepth::Minimal => println!("8 colors"),
///     ColorDepth::None => println!("No color support"),
/// }
/// ```
pub fn color_depth() -> ColorDepth {
    // Check COLORTERM environment variable first
    if let Ok(colorterm) = env::var("COLORTERM") {
        let colorterm_lower = colorterm.to_lowercase();
        if colorterm_lower == "truecolor" || colorterm_lower == "24bit" {
            tracing::info!(
                color_depth = ?ColorDepth::TrueColor,
                source = "COLORTERM",
                colorterm = %colorterm,
                "Detected truecolor support from COLORTERM env var"
            );
            return ColorDepth::TrueColor;
        }
    }

    // Fallback to terminfo
    match TermInfo::from_env() {
        Ok(term_info) => {
            // Query the MaxColors capability
            let depth = term_info
                .number_cap(NumberCapability::MaxColors)
                .map(|n| n as u32)
                .unwrap_or(0);

            let color_depth = match depth {
                d if d >= 16_777_216 => ColorDepth::TrueColor,
                d if d >= 256 => ColorDepth::Enhanced,
                d if d >= 16 => ColorDepth::Basic,
                d if d >= 8 => ColorDepth::Minimal,
                _ => ColorDepth::None,
            };

            tracing::info!(
                ?color_depth,
                source = "terminfo",
                "Detected color depth from terminfo"
            );
            color_depth
        }
        Err(e) => {
            tracing::info!(
                color_depth = ?ColorDepth::None,
                source = "fallback",
                error = %e,
                "Failed to query terminfo, defaulting to no color"
            );
            ColorDepth::None
        }
    }
}

/// Whether the terminal is in "light" or "dark" mode.
///
/// Detection strategy:
/// 1. Try to get background color from OSC queries and determine from luminance
/// 2. Check `DARK_MODE` environment variable
/// 3. On macOS, check `AppleInterfaceStyle` system preference
/// 4. Default to Dark (most common for terminal users)
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::discovery::detection::{color_mode, ColorMode};
///
/// match color_mode() {
///     ColorMode::Light => println!("Light mode - use dark text"),
///     ColorMode::Dark => println!("Dark mode - use light text"),
///     ColorMode::Unknown => println!("Unknown - use default colors"),
/// }
/// ```
pub fn color_mode() -> ColorMode {
    // Try to get background color and determine from luminance
    if let Some(bg) = crate::discovery::osc_queries::bg_color() {
        let luminance = bg.luminance();
        if luminance > 0.5 {
            return ColorMode::Light;
        } else {
            return ColorMode::Dark;
        }
    }

    // Check common environment variables
    if let Ok(mode) = env::var("DARK_MODE") {
        if mode == "0" || mode.to_lowercase() == "false" {
            return ColorMode::Light;
        }
        if mode == "1" || mode.to_lowercase() == "true" {
            return ColorMode::Dark;
        }
    }

    // macOS: Check AppleInterfaceStyle
    #[cfg(target_os = "macos")]
    {
        if let Ok(output) = std::process::Command::new("defaults")
            .args(["read", "-g", "AppleInterfaceStyle"])
            .output()
        {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                if stdout.trim().to_lowercase() == "dark" {
                    return ColorMode::Dark;
                }
            }
            // If command succeeds but no "Dark" value, it's Light mode
            // (AppleInterfaceStyle is only set when Dark mode is active)
            return ColorMode::Light;
        }
    }

    // Default to Dark (most common for terminal users)
    ColorMode::Dark
}

/// Check if stdout is connected to a TTY (terminal).
///
/// Returns `false` when output is piped or redirected to a file.
/// Useful for deciding whether to use colors/formatting.
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::discovery::detection::is_tty;
///
/// if is_tty() {
///     println!("\x1b[32mColored output\x1b[0m");
/// } else {
///     println!("Plain output (piped or redirected)");
/// }
/// ```
pub fn is_tty() -> bool {
    use std::io::IsTerminal;
    std::io::stdout().is_terminal()
}

/// Detect the current terminal emulator application.
///
/// Detection uses environment variables in this order:
/// 1. `TERM_PROGRAM` - Set by most modern terminals
/// 2. `WT_SESSION` - Windows Terminal indicator
/// 3. `TERM` - Fallback for terminals that set this
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::discovery::detection::{get_terminal_app, TerminalApp};
///
/// match get_terminal_app() {
///     TerminalApp::Wezterm => println!("Running in WezTerm"),
///     TerminalApp::Kitty => println!("Running in Kitty"),
///     TerminalApp::ITerm2 => println!("Running in iTerm2"),
///     TerminalApp::Ghostty => println!("Running in Ghostty"),
///     TerminalApp::Other(name) => println!("Running in: {}", name),
///     _ => println!("Running in another terminal"),
/// }
/// ```
pub fn get_terminal_app() -> TerminalApp {
    if let Ok(term_program) = env::var("TERM_PROGRAM") {
        match term_program.as_str() {
            "Apple_Terminal" => return TerminalApp::AppleTerminal,
            "iterm2" | "iTerm.app" => return TerminalApp::ITerm2,
            "vscode" => return TerminalApp::VsCode,
            "warp" => return TerminalApp::Warp,
            "ghostty" => return TerminalApp::Ghostty,
            "kitty" => return TerminalApp::Kitty,
            "Alacritty" => return TerminalApp::Alacritty,
            "WezTerm" => return TerminalApp::Wezterm,
            "gnome-terminal" => return TerminalApp::GnomeTerminal,
            "konsole" => return TerminalApp::Konsole,
            _ => {}
        }
    }

    if env::var("WT_SESSION").is_ok() {
        return TerminalApp::Other("Windows Terminal".to_string());
    }

    let term = env::var("TERM").unwrap_or_default();
    match term.as_str() {
        "xterm-kitty" | "kitty" => TerminalApp::Kitty,
        "alacritty" => TerminalApp::Alacritty,
        "wezterm" => TerminalApp::Wezterm,
        "ghostty" => TerminalApp::Ghostty,
        "foot" => TerminalApp::Foot,
        "contour" => TerminalApp::Contour,
        _ => TerminalApp::Other(term),
    }
}

/// Get the terminal width in columns.
///
/// Returns 80 as a fallback if detection fails.
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::discovery::detection::terminal_width;
///
/// let width = terminal_width();
/// println!("Terminal is {} columns wide", width);
/// ```
pub fn terminal_width() -> u32 {
    dimensions().0
}

/// Get the terminal height in rows.
///
/// Returns 24 as a fallback if detection fails.
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::discovery::detection::terminal_height;
///
/// let height = terminal_height();
/// println!("Terminal is {} rows tall", height);
/// ```
pub fn terminal_height() -> u32 {
    dimensions().1
}

/// Get the terminal dimensions as (width, height) in characters.
///
/// Returns (80, 24) as a fallback if detection fails.
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::discovery::detection::dimensions;
///
/// let (width, height) = dimensions();
/// println!("Terminal size: {}x{}", width, height);
/// ```
pub fn dimensions() -> (u32, u32) {
    terminal_size()
        .map(|(Width(w), Height(h))| (w as u32, h as u32))
        .unwrap_or((80, 24))
}

/// Detect image display support in the terminal.
///
/// Returns the highest quality image protocol supported:
/// - `Kitty` - Kitty Graphics Protocol (highest quality)
/// - `ITerm` - iTerm2 image protocol (legacy)
/// - `None` - No image support
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::discovery::detection::{image_support, ImageSupport};
///
/// match image_support() {
///     ImageSupport::Kitty => println!("Kitty graphics protocol supported"),
///     ImageSupport::ITerm => println!("iTerm2 image protocol supported"),
///     ImageSupport::None => println!("No image support"),
/// }
/// ```
pub fn image_support() -> ImageSupport {
    if !is_tty() {
        return ImageSupport::None;
    }

    if let Ok(term_program) = env::var("TERM_PROGRAM") {
        match term_program.as_str() {
            "kitty" | "WezTerm" | "Warp" | "ghostty" | "konsole" | "wast" | "iTerm.app" => {
                return ImageSupport::Kitty;
            }
            _ => {}
        }
    }

    let term = env::var("TERM").unwrap_or_default();
    if term.contains("kitty") {
        return ImageSupport::Kitty;
    }

    ImageSupport::None
}

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
            "iTerm.app" | "kitty" | "WezTerm" | "Alacritty" | "ghostty" | "warp" | "vscode"
            | "gnome-terminal" => {
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
///     MultiplexSupport::Tmux { split_window: true, .. } => {
///         println!("Running inside tmux with split support");
///     }
///     MultiplexSupport::Native { .. } => {
///         println!("Terminal has native multiplexing");
///     }
///     MultiplexSupport::None => {
///         println!("No multiplexing support detected");
///     }
///     _ => {}
/// }
/// ```
pub fn multiplex_support() -> MultiplexSupport {
    // Check for tmux first (most common persistent multiplexer)
    if env::var("TMUX").is_ok() {
        return MultiplexSupport::Tmux {
            split_window: true,
            resize_pane: true,
            focus_pane: true,
            multiple_windows: true,
            session_persistence: true,
            detach_session: true,
        };
    }

    // Check for Zellij
    if env::var("ZELLIJ").is_ok() {
        return MultiplexSupport::Zellij {
            split_window: true,
            resize_pane: true,
            focus_pane: true,
            multiple_tabs: true,
            session_resurrection: true,
            floating_panes: true,
            detach_session: true,
        };
    }

    // Check for native multiplexing in terminal emulators
    if let Ok(term_program) = env::var("TERM_PROGRAM") {
        match term_program.as_str() {
            // Kitty has native multiplexing with layouts
            "kitty" => {
                return MultiplexSupport::Native {
                    split_window: true,
                    resize_pane: true,
                    focus_pane: true,
                    multiple_tabs: true,
                };
            }
            // WezTerm has multiplexer domains (SSH, WSL, local)
            "WezTerm" => {
                return MultiplexSupport::Native {
                    split_window: true,
                    resize_pane: true,
                    focus_pane: true,
                    multiple_tabs: true,
                };
            }
            // Ghostty has native multiplexing
            "ghostty" => {
                return MultiplexSupport::Native {
                    split_window: true,
                    resize_pane: true,
                    focus_pane: true,
                    multiple_tabs: true,
                };
            }
            _ => {}
        }
    }

    // Check TERM variable for terminals with native multiplexing
    let term = env::var("TERM").unwrap_or_default();
    if term.contains("kitty") || term.contains("wezterm") || term.contains("ghostty") {
        return MultiplexSupport::Native {
            split_window: true,
            resize_pane: true,
            focus_pane: true,
            multiple_tabs: true,
        };
    }

    // No multiplexing detected
    MultiplexSupport::None
}

/// Detect extended underline style support.
///
/// Modern terminals support various underline styles beyond the basic
/// straight underline, including curly (for LSP errors), double, dotted,
/// and dashed styles. Some terminals also support colored underlines.
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::discovery::detection::underline_support;
///
/// let support = underline_support();
/// if support.curly {
///     // Use curly underline for errors (common in editors)
///     println!("\x1b[4:3m\x1b[58:2::255:0:0mError text\x1b[0m");
/// } else if support.straight {
///     println!("\x1b[4mUnderlined text\x1b[0m");
/// }
/// ```
pub fn underline_support() -> UnderlineSupport {
    use std::io::IsTerminal;

    let none = UnderlineSupport {
        straight: false,
        double: false,
        curly: false,
        dotted: false,
        dashed: false,
        colored: false,
    };

    // If stdout is not a TTY, no styling
    if !std::io::stdout().is_terminal() {
        return UnderlineSupport {
            straight: false,
            double: false,
            curly: false,
            dotted: false,
            dashed: false,
            colored: false,
        };
    }

    // Check for dumb terminal
    let term = env::var("TERM").unwrap_or_default();
    if term == "dumb" {
        return none;
    }

    // Check if basic underline is supported via terminfo
    let has_basic_underline = TermInfo::from_env()
        .map(|ti| {
            ti.utf8_string_cap(StringCapability::EnterUnderlineMode)
                .is_some()
        })
        .unwrap_or(false);

    // Helper for terminals with full extended underline support
    let full_support = || UnderlineSupport {
        straight: true,
        double: true,
        curly: true,
        dotted: true,
        dashed: true,
        colored: true,
    };

    // Helper for terminals with straight underline only
    let basic_only = || UnderlineSupport {
        straight: true,
        double: false,
        curly: false,
        dotted: false,
        dashed: false,
        colored: false,
    };

    // 1. Check TERM_PROGRAM for known terminal emulators
    if let Ok(term_program) = env::var("TERM_PROGRAM") {
        match term_program.as_str() {
            // Full Kitty-style underline support
            "kitty" | "WezTerm" | "Alacritty" | "ghostty" | "contour" | "foot" | "iTerm.app" => {
                return full_support();
            }
            // VTE-based terminals (GNOME Terminal 3.44+, Tilix) - full support
            "gnome-terminal" | "tilix" => {
                return full_support();
            }
            // Konsole has colored underlines but limited style support
            "konsole" => {
                return UnderlineSupport {
                    straight: true,
                    double: true,
                    curly: false, // Konsole doesn't support curly as of 2024
                    dotted: false,
                    dashed: false,
                    colored: true,
                };
            }
            // Apple Terminal - basic underline only
            "Apple_Terminal" => {
                return basic_only();
            }
            // VS Code terminal - full support
            "vscode" => {
                return full_support();
            }
            _ => {}
        }
    }

    // 2. Check for Windows Terminal
    if env::var("WT_SESSION").is_ok() {
        return full_support();
    }

    // 3. Check TERM for known terminal patterns
    match term.as_str() {
        // Full extended underline support
        "xterm-kitty" | "kitty" | "kitty-direct" | "wezterm" | "alacritty" | "alacritty-direct"
        | "ghostty" | "foot" | "foot-direct" | "contour" => {
            return full_support();
        }
        // Basic underline via common terminal types
        "xterm-256color"
        | "xterm-direct"
        | "tmux-256color"
        | "screen-256color"
        | "rxvt-unicode-256color" => {
            // These may or may not support extended underlines depending on
            // the actual terminal behind them. Return basic only to be safe.
            if has_basic_underline {
                return basic_only();
            }
        }
        _ => {}
    }

    // 4. Fall back to terminfo for basic support
    if has_basic_underline {
        return basic_only();
    }

    none
}

/// Detect if the terminal supports italic text rendering.
///
/// This function uses a multi-layer detection strategy:
///
/// 1. **Terminfo** (authoritative): Checks for `EnterItalicsMode` (`sitm`) capability
/// 2. **TERM_PROGRAM**: Recognizes modern terminal emulators known to support italics
/// 3. **TERM**: Falls back to pattern matching for common terminal types
///
/// This layered approach compensates for outdated terminfo databases.
///
/// ## Returns
///
/// - `true` if the terminal supports italic text
/// - `false` if stdout is not a TTY, TERM is "dumb", or no support is detected
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::discovery::detection::italics_support;
///
/// if italics_support() {
///     println!("\x1b[3mThis text is italic!\x1b[23m");
/// } else {
///     println!("This text has no styling");
/// }
/// ```
pub fn italics_support() -> bool {
    use std::io::IsTerminal;

    // If stdout is not a TTY, don't use styling
    if !std::io::stdout().is_terminal() {
        return false;
    }

    // Check for dumb terminal
    let term = env::var("TERM").unwrap_or_default();
    if term == "dumb" {
        return false;
    }

    // 1. Query terminfo for EnterItalicsMode (sitm) capability (authoritative)
    if let Ok(term_info) = TermInfo::from_env()
        && term_info
            .utf8_string_cap(StringCapability::EnterItalicsMode)
            .is_some()
    {
        return true;
    }

    // 2. Check TERM_PROGRAM for known terminal emulators that support italics
    if let Ok(term_program) = env::var("TERM_PROGRAM") {
        let dominated = matches!(
            term_program.as_str(),
            "iTerm.app"
                | "Apple_Terminal"
                | "Alacritty"
                | "kitty"
                | "WezTerm"
                | "vscode"
                | "Hyper"
                | "Tabby"
                | "Rio"
        );
        if dominated {
            return true;
        }
    }

    // 3. Check for Windows Terminal (uses WT_SESSION env var)
    if env::var("WT_SESSION").is_ok() {
        return true;
    }

    // 4. Fallback: check TERM for patterns indicating modern terminals
    let dominated = matches!(
        term.as_str(),
        "xterm-256color"
            | "xterm-direct"
            | "alacritty"
            | "alacritty-direct"
            | "kitty"
            | "kitty-direct"
            | "wezterm"
            | "tmux-256color"
            | "screen-256color"
    );
    if dominated {
        return true;
    }

    false
}
