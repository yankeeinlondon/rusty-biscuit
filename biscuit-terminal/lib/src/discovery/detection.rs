use std::env;

use serde::{Deserialize, Serialize};
use terminal_size::{Height, Width, terminal_size};
use termini::{NumberCapability, StringCapability, TermInfo};

/// The type of image support (if any) of a terminal
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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

/// Detailed result of image support detection, including the reason for the decision.
///
/// This is useful for debugging why a particular image protocol was selected
/// or why images are not supported.
#[derive(Debug, Clone)]
pub struct ImageSupportResult {
    /// The detected image support level
    pub support: ImageSupport,
    /// Human-readable reason for the detection result
    pub reason: String,
    /// The detection method used (e.g., "viuer", "env_heuristic", "tty_check")
    pub method: String,
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

/// Client information for a host which was established over an SSH connection.
///
/// Parsed from the `SSH_CLIENT` environment variable which has the format:
/// `<client_ip> <client_port> <server_port>`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshClient {
    /// The IP address or DNS name of the client connecting
    pub host: String,

    /// The port which the host is communicating back to the client on
    pub source_port: u32,

    /// The port the client used to connect to the host (typically 22)
    pub server_port: u32,

    /// The TTY path for the SSH session (from `SSH_TTY`)
    pub tty_path: Option<String>,
}

/// Client information for a host which was established over a Mosh connection.
///
/// Mosh (Mobile Shell) provides a more resilient remote connection that
/// handles intermittent connectivity and roaming.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoshClient {
    /// The connection string from `MOSH_CONNECTION`
    pub connection: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Connection {
    /// This terminal connection is a local connection
    Local,
    /// This terminal is using a SSH connection
    SshClient(SshClient),
    MoshClient(MoshClient),
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
    // 1. Check TERM_PROGRAM environment variable (most reliable when set)
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

    // 2. Check terminal-specific environment variables
    if env::var("WT_SESSION").is_ok() {
        return TerminalApp::Other("Windows Terminal".to_string());
    }
    if env::var("KITTY_WINDOW_ID").is_ok() || env::var("KITTY_PID").is_ok() {
        return TerminalApp::Kitty;
    }
    if env::var("WEZTERM_PANE").is_ok() || env::var("WEZTERM_UNIX_SOCKET").is_ok() {
        return TerminalApp::Wezterm;
    }
    if env::var("ITERM_SESSION_ID").is_ok() || env::var("ITERM_PROFILE").is_ok() {
        return TerminalApp::ITerm2;
    }
    if env::var("GHOSTTY_RESOURCES_DIR").is_ok() {
        return TerminalApp::Ghostty;
    }
    // Alacritty sets these environment variables
    if env::var("ALACRITTY_WINDOW_ID").is_ok()
        || env::var("ALACRITTY_SOCKET").is_ok()
        || env::var("ALACRITTY_LOG").is_ok()
    {
        return TerminalApp::Alacritty;
    }

    // 3. Check TERM variable
    let term = env::var("TERM").unwrap_or_default();
    match term.as_str() {
        "xterm-kitty" | "kitty" => return TerminalApp::Kitty,
        "alacritty" => return TerminalApp::Alacritty,
        "wezterm" => return TerminalApp::Wezterm,
        "ghostty" => return TerminalApp::Ghostty,
        "foot" | "foot-extra" => return TerminalApp::Foot,
        "contour" => return TerminalApp::Contour,
        _ => {}
    }

    TerminalApp::Other(term)
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
/// ## Detection Strategy
///
/// When the `viuer` feature is enabled (default), this function uses viuer's
/// runtime detection which actually queries the terminal:
/// 1. `viuer::get_kitty_support()` - Probes for Kitty Graphics Protocol
/// 2. `viuer::is_iterm_supported()` - Checks for iTerm2 inline images
///
/// Falls back to environment variable heuristics when viuer detection
/// returns no support or when the `viuer` feature is disabled.
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
    image_support_with_reason().support
}

/// Detect image display support with detailed reasoning.
///
/// This function provides the same detection as [`image_support()`] but also
/// returns information about why a particular protocol was selected or why
/// images are not supported. Useful for debugging detection issues.
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::discovery::detection::image_support_with_reason;
///
/// let result = image_support_with_reason();
/// println!("Support: {:?}", result.support);
/// println!("Reason: {}", result.reason);
/// println!("Method: {}", result.method);
/// ```
pub fn image_support_with_reason() -> ImageSupportResult {
    // First check: must be a TTY
    if !is_tty() {
        return ImageSupportResult {
            support: ImageSupport::None,
            reason: "stdout is not a TTY (piped or redirected)".to_string(),
            method: "tty_check".to_string(),
        };
    }

    // When viuer feature is enabled, use its runtime detection first
    // viuer actually queries the terminal, so it's more accurate than env heuristics
    #[cfg(feature = "viuer")]
    {
        use viuer::{KittySupport, get_kitty_support, is_iterm_supported};

        // Check for Kitty Graphics Protocol support
        match get_kitty_support() {
            KittySupport::Local | KittySupport::Remote => {
                let support_type = match get_kitty_support() {
                    KittySupport::Local => "local files only",
                    KittySupport::Remote => "full remote support",
                    KittySupport::None => unreachable!(),
                };
                tracing::debug!(
                    image_support = "Kitty",
                    kitty_level = support_type,
                    method = "viuer",
                    "Detected Kitty graphics protocol via viuer"
                );
                return ImageSupportResult {
                    support: ImageSupport::Kitty,
                    reason: format!("viuer detected Kitty graphics protocol ({})", support_type),
                    method: "viuer".to_string(),
                };
            }
            KittySupport::None => {
                tracing::trace!(
                    method = "viuer",
                    "viuer reports no Kitty support, checking iTerm2"
                );
            }
        }

        // Check for iTerm2 inline images support
        if is_iterm_supported() {
            tracing::debug!(
                image_support = "ITerm",
                method = "viuer",
                "Detected iTerm2 inline images via viuer"
            );
            return ImageSupportResult {
                support: ImageSupport::ITerm,
                reason: "viuer detected iTerm2 inline images support".to_string(),
                method: "viuer".to_string(),
            };
        }

        tracing::trace!(
            method = "viuer",
            "viuer reports no image protocol support, falling back to env heuristics"
        );
    }

    // Fallback: environment variable heuristics
    // These are less accurate but work when viuer detection fails or is disabled
    image_support_from_env()
}

/// Detect image support using environment variable heuristics only.
///
/// This is used as a fallback when viuer detection is not available or fails.
/// It checks `TERM_PROGRAM` and `TERM` environment variables to infer support.
fn image_support_from_env() -> ImageSupportResult {
    // Check TERM_PROGRAM for known terminals
    if let Ok(term_program) = env::var("TERM_PROGRAM") {
        match term_program.as_str() {
            // Terminals with Kitty Graphics Protocol support
            "kitty" | "WezTerm" | "Warp" | "ghostty" | "konsole" | "wast" => {
                tracing::debug!(
                    image_support = "Kitty",
                    term_program = %term_program,
                    method = "env_heuristic",
                    "Detected Kitty support from TERM_PROGRAM"
                );
                return ImageSupportResult {
                    support: ImageSupport::Kitty,
                    reason: format!(
                        "TERM_PROGRAM={} indicates Kitty graphics protocol support",
                        term_program
                    ),
                    method: "env_heuristic".to_string(),
                };
            }
            // iTerm2 - can use either protocol, but prefer its native protocol
            // when viuer didn't detect Kitty support
            "iTerm.app" | "iterm2" => {
                tracing::debug!(
                    image_support = "ITerm",
                    term_program = %term_program,
                    method = "env_heuristic",
                    "Detected iTerm2 support from TERM_PROGRAM"
                );
                return ImageSupportResult {
                    support: ImageSupport::ITerm,
                    reason: format!(
                        "TERM_PROGRAM={} indicates iTerm2 inline images support",
                        term_program
                    ),
                    method: "env_heuristic".to_string(),
                };
            }
            _ => {}
        }
    }

    // Check ITERM_SESSION_ID for iTerm2 detection
    if env::var("ITERM_SESSION_ID").is_ok() || env::var("ITERM_PROFILE").is_ok() {
        tracing::debug!(
            image_support = "ITerm",
            method = "env_heuristic",
            "Detected iTerm2 from session environment variables"
        );
        return ImageSupportResult {
            support: ImageSupport::ITerm,
            reason: "ITERM_SESSION_ID or ITERM_PROFILE indicates iTerm2".to_string(),
            method: "env_heuristic".to_string(),
        };
    }

    // Check TERM variable for kitty
    let term = env::var("TERM").unwrap_or_default();
    if term.contains("kitty") {
        tracing::debug!(
            image_support = "Kitty",
            term = %term,
            method = "env_heuristic",
            "Detected Kitty support from TERM variable"
        );
        return ImageSupportResult {
            support: ImageSupport::Kitty,
            reason: format!("TERM={} indicates Kitty graphics protocol support", term),
            method: "env_heuristic".to_string(),
        };
    }

    // No image support detected
    tracing::debug!(
        image_support = "None",
        method = "env_heuristic",
        "No image protocol support detected"
    );
    ImageSupportResult {
        support: ImageSupport::None,
        reason: "No image protocol support detected from environment".to_string(),
        method: "env_heuristic".to_string(),
    }
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

// Re-export OSC support functions from osc_queries module for API compatibility
pub use crate::discovery::osc_queries::{osc10_support, osc11_support, osc12_support};

/// Detect whether the terminal session is a remote connection (SSH, Mosh) or local.
///
/// ## Detection Strategy
///
/// 1. Check `SSH_CLIENT` environment variable for SSH connections
/// 2. Check `MOSH_CONNECTION` for Mosh connections
/// 3. Default to `Connection::Local` if no remote indicators
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::discovery::detection::{detect_connection, Connection};
///
/// match detect_connection() {
///     Connection::Local => println!("Running locally"),
///     Connection::SshClient(ssh) => println!("SSH from {}", ssh.host),
///     Connection::MoshClient(mosh) => println!("Mosh connection: {}", mosh.connection),
/// }
/// ```
pub fn detect_connection() -> Connection {
    // Check for Mosh first (it also sets SSH_CLIENT sometimes)
    if let Ok(mosh_conn) = env::var("MOSH_CONNECTION")
        && !mosh_conn.is_empty() {
            return Connection::MoshClient(MoshClient {
                connection: mosh_conn,
            });
        }

    // Check for SSH connection
    // SSH_CLIENT format: "client_ip client_port server_port"
    if let Ok(ssh_client) = env::var("SSH_CLIENT") {
        let parts: Vec<&str> = ssh_client.split_whitespace().collect();
        if parts.len() >= 3
            && let (Ok(source_port), Ok(server_port)) =
                (parts[1].parse::<u32>(), parts[2].parse::<u32>())
            {
                let tty_path = env::var("SSH_TTY").ok();
                return Connection::SshClient(SshClient {
                    host: parts[0].to_string(),
                    source_port,
                    server_port,
                    tty_path,
                });
            }
    }

    Connection::Local
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    /// Helper to set environment variables with automatic cleanup.
    ///
    /// In Rust 2024, env::set_var and env::remove_var are unsafe because
    /// modifying environment variables can cause undefined behavior when
    /// other threads are reading them. In test code, we use serial_test
    /// to ensure these tests run sequentially.
    struct ScopedEnv {
        vars: Vec<(String, Option<String>)>,
    }

    impl ScopedEnv {
        fn new() -> Self {
            Self { vars: Vec::new() }
        }

        fn set(&mut self, key: &str, value: &str) {
            let old = env::var(key).ok();
            self.vars.push((key.to_string(), old));
            // SAFETY: Tests using ScopedEnv are marked with #[serial] to prevent
            // concurrent access to environment variables.
            unsafe { env::set_var(key, value) };
        }

        fn remove(&mut self, key: &str) {
            let old = env::var(key).ok();
            self.vars.push((key.to_string(), old));
            // SAFETY: Tests using ScopedEnv are marked with #[serial] to prevent
            // concurrent access to environment variables.
            unsafe { env::remove_var(key) };
        }
    }

    impl Drop for ScopedEnv {
        fn drop(&mut self) {
            for (key, old_value) in self.vars.drain(..).rev() {
                // SAFETY: Tests using ScopedEnv are marked with #[serial] to prevent
                // concurrent access to environment variables.
                unsafe {
                    match old_value {
                        Some(v) => env::set_var(&key, v),
                        None => env::remove_var(&key),
                    }
                }
            }
        }
    }

    // ========================================================================
    // ImageSupport enum tests
    // ========================================================================

    #[test]
    fn test_image_support_eq() {
        assert_eq!(ImageSupport::None, ImageSupport::None);
        assert_eq!(ImageSupport::Kitty, ImageSupport::Kitty);
        assert_eq!(ImageSupport::ITerm, ImageSupport::ITerm);
        assert_ne!(ImageSupport::None, ImageSupport::Kitty);
        assert_ne!(ImageSupport::Kitty, ImageSupport::ITerm);
    }

    #[test]
    fn test_image_support_debug() {
        let debug_none = format!("{:?}", ImageSupport::None);
        assert!(debug_none.contains("None"));

        let debug_kitty = format!("{:?}", ImageSupport::Kitty);
        assert!(debug_kitty.contains("Kitty"));

        let debug_iterm = format!("{:?}", ImageSupport::ITerm);
        assert!(debug_iterm.contains("ITerm"));
    }

    #[test]
    fn test_image_support_clone() {
        let support = ImageSupport::Kitty;
        let cloned = support.clone();
        assert_eq!(support, cloned);
    }

    // ========================================================================
    // ImageSupportResult struct tests
    // ========================================================================

    #[test]
    fn test_image_support_result_fields() {
        let result = ImageSupportResult {
            support: ImageSupport::Kitty,
            reason: "test reason".to_string(),
            method: "test_method".to_string(),
        };

        assert_eq!(result.support, ImageSupport::Kitty);
        assert_eq!(result.reason, "test reason");
        assert_eq!(result.method, "test_method");
    }

    #[test]
    fn test_image_support_result_debug() {
        let result = ImageSupportResult {
            support: ImageSupport::ITerm,
            reason: "viuer detected iTerm2".to_string(),
            method: "viuer".to_string(),
        };

        let debug = format!("{:?}", result);
        assert!(debug.contains("ITerm"));
        assert!(debug.contains("viuer"));
    }

    #[test]
    fn test_image_support_result_clone() {
        let result = ImageSupportResult {
            support: ImageSupport::None,
            reason: "not a tty".to_string(),
            method: "tty_check".to_string(),
        };

        let cloned = result.clone();
        assert_eq!(cloned.support, result.support);
        assert_eq!(cloned.reason, result.reason);
        assert_eq!(cloned.method, result.method);
    }

    // ========================================================================
    // image_support_from_env tests (environment heuristics)
    // ========================================================================

    #[test]
    #[serial]
    fn test_image_support_from_env_kitty_term_program() {
        let mut env = ScopedEnv::new();
        env.set("TERM_PROGRAM", "kitty");
        env.remove("ITERM_SESSION_ID");
        env.remove("ITERM_PROFILE");

        let result = image_support_from_env();
        assert_eq!(result.support, ImageSupport::Kitty);
        assert!(result.reason.contains("TERM_PROGRAM"));
        assert_eq!(result.method, "env_heuristic");
    }

    #[test]
    #[serial]
    fn test_image_support_from_env_wezterm() {
        let mut env = ScopedEnv::new();
        env.set("TERM_PROGRAM", "WezTerm");
        env.remove("ITERM_SESSION_ID");
        env.remove("ITERM_PROFILE");

        let result = image_support_from_env();
        assert_eq!(result.support, ImageSupport::Kitty);
        assert!(result.reason.contains("WezTerm"));
    }

    #[test]
    #[serial]
    fn test_image_support_from_env_ghostty() {
        let mut env = ScopedEnv::new();
        env.set("TERM_PROGRAM", "ghostty");
        env.remove("ITERM_SESSION_ID");
        env.remove("ITERM_PROFILE");

        let result = image_support_from_env();
        assert_eq!(result.support, ImageSupport::Kitty);
        assert!(result.reason.contains("ghostty"));
    }

    #[test]
    #[serial]
    fn test_image_support_from_env_iterm2_term_program() {
        let mut env = ScopedEnv::new();
        env.set("TERM_PROGRAM", "iTerm.app");
        env.remove("ITERM_SESSION_ID");
        env.remove("ITERM_PROFILE");

        let result = image_support_from_env();
        assert_eq!(result.support, ImageSupport::ITerm);
        assert!(result.reason.contains("iTerm"));
    }

    #[test]
    #[serial]
    fn test_image_support_from_env_iterm2_session_id() {
        let mut env = ScopedEnv::new();
        env.remove("TERM_PROGRAM");
        env.set("ITERM_SESSION_ID", "w0t0p0:12345678-1234-1234-1234-123456789abc");
        env.remove("ITERM_PROFILE");

        let result = image_support_from_env();
        assert_eq!(result.support, ImageSupport::ITerm);
        assert!(result.reason.contains("ITERM_SESSION_ID"));
    }

    #[test]
    #[serial]
    fn test_image_support_from_env_iterm2_profile() {
        let mut env = ScopedEnv::new();
        env.remove("TERM_PROGRAM");
        env.remove("ITERM_SESSION_ID");
        env.set("ITERM_PROFILE", "Default");

        let result = image_support_from_env();
        assert_eq!(result.support, ImageSupport::ITerm);
        assert!(result.reason.contains("ITERM_PROFILE"));
    }

    #[test]
    #[serial]
    fn test_image_support_from_env_kitty_term_var() {
        let mut env = ScopedEnv::new();
        env.remove("TERM_PROGRAM");
        env.remove("ITERM_SESSION_ID");
        env.remove("ITERM_PROFILE");
        env.set("TERM", "xterm-kitty");

        let result = image_support_from_env();
        assert_eq!(result.support, ImageSupport::Kitty);
        assert!(result.reason.contains("TERM="));
        assert!(result.reason.contains("kitty"));
    }

    #[test]
    #[serial]
    fn test_image_support_from_env_none() {
        let mut env = ScopedEnv::new();
        env.remove("TERM_PROGRAM");
        env.remove("ITERM_SESSION_ID");
        env.remove("ITERM_PROFILE");
        env.set("TERM", "xterm-256color");

        let result = image_support_from_env();
        assert_eq!(result.support, ImageSupport::None);
        assert!(result.reason.contains("No image protocol"));
    }

    // ========================================================================
    // image_support and image_support_with_reason tests
    // ========================================================================

    #[test]
    fn test_image_support_returns_support_field() {
        // This test verifies that image_support() returns the same value
        // as image_support_with_reason().support
        let simple = image_support();
        let detailed = image_support_with_reason();
        assert_eq!(simple, detailed.support);
    }

    #[test]
    fn test_image_support_with_reason_has_non_empty_fields() {
        let result = image_support_with_reason();

        // Reason should always be non-empty
        assert!(!result.reason.is_empty(), "Reason should not be empty");

        // Method should always be non-empty
        assert!(!result.method.is_empty(), "Method should not be empty");

        // Method should be one of the expected values
        let valid_methods = ["tty_check", "viuer", "env_heuristic"];
        assert!(
            valid_methods.contains(&result.method.as_str()),
            "Method '{}' should be one of {:?}",
            result.method,
            valid_methods
        );
    }

    // ========================================================================
    // Feature flag tests
    // ========================================================================

    #[test]
    #[cfg(feature = "viuer")]
    fn test_viuer_feature_enabled() {
        // When viuer feature is enabled, the detection should work
        // Note: we can't test the actual viuer detection results in unit tests
        // because they depend on the runtime terminal environment
        let result = image_support_with_reason();

        // Should complete without panic
        assert!(!result.reason.is_empty());
    }

    #[test]
    #[cfg(not(feature = "viuer"))]
    fn test_viuer_feature_disabled() {
        // When viuer is disabled, should fall back to env heuristics
        let result = image_support_with_reason();

        // If not a TTY, method will be "tty_check"
        // Otherwise, method should be "env_heuristic"
        assert!(
            result.method == "tty_check" || result.method == "env_heuristic",
            "Without viuer, method should be tty_check or env_heuristic, got: {}",
            result.method
        );
    }
}
