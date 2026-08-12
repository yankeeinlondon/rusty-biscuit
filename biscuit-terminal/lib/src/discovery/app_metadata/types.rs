//! Static metadata model for terminal-app configuration and environment facts.
//!
//! Every type here describes *vendor-determined, version-stable* facts held as
//! `&'static` data (candidate paths, env-var names, dot locators). None of it
//! reflects what a particular host actually has — that is the job of the
//! [`resolver`](super::resolver), which reads the live filesystem and
//! environment. See `features/2026-06-29-app-metadata/spec.md` for the design.

use std::path::PathBuf;

use serde::Serialize;

use super::os_target::ConfigOsTarget;

/// The on-disk format of a terminal's configuration file.
///
/// Determines how a [`SettingLocator`]'s dot path is interpreted (see
/// [`SettingLocator::path`]) and which extraction strategy applies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ConfigFormat {
    /// Flat `key value` lines (kitty).
    KittyConf,
    /// Flat `key = value` lines (ghostty, foot/ini-ish).
    KeyValue,
    /// TOML (alacritty).
    Toml,
    /// YAML (contour, legacy `alacritty.yml`).
    Yaml,
    /// Lua (wezterm) — not statically parseable; locator-only.
    Lua,
    /// Apple property list, XML or binary (iTerm2, Apple Terminal).
    Plist,
    /// JSON (Windows Terminal).
    Json,
    /// JSON5 / JSONC-tolerant settings (VS Code).
    Json5,
    /// GNOME Terminal / gsettings — managed by dconf, no flat file.
    Dconf,
    /// No parseable config file and outside the app coverage floor.
    None,
}

/// Filesystem shape expected for a config candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ConfigCandidateKind {
    /// Candidate resolves only when it is a regular file.
    File,
    /// Candidate resolves only when it is a directory.
    Directory,
}

/// Whether a config-relocating environment variable points at a directory or a file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ConfigEnvKind {
    /// The variable holds a directory; the app's canonical filename is joined onto it.
    Dir,
    /// The variable holds the config file path directly.
    File,
}

/// Advisory hint for the shape of an extracted setting value.
///
/// v1 extraction returns raw values only; this is display metadata, not a parsing
/// contract, and never coerces or validates the extracted value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ValueKind {
    /// Free-form string.
    String,
    /// Numeric value.
    Number,
    /// Color (hex, name, or rgb triple).
    Color,
    /// Boolean.
    Bool,
    /// Filesystem path.
    Path,
    /// One of a fixed set of named values.
    Enum,
}

/// A single config-file location template with portable tokens.
///
/// The template is expanded at resolve time. Supported tokens: `~`, `$HOME`,
/// `$XDG_CONFIG_HOME`, `$APPDATA`, `$LOCALAPPDATA`, `$USER`, and the WSL helper
/// `$WIN_HOME` (the Windows user profile surfaced under `/mnt/c`). A template may
/// contain a single `*` wildcard segment (Windows Terminal package glob); it is
/// expanded against the filesystem, first match wins. Candidates whose tokens
/// resolve to nothing (unset/empty/unknown) are dropped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ConfigCandidate {
    /// Path template with portable tokens (see the type-level docs).
    pub template: &'static str,
    /// Whether this candidate points at a file or directory.
    pub kind: ConfigCandidateKind,
    /// Format override for this specific candidate, when it differs from the
    /// app's primary config format.
    pub format: Option<ConfigFormat>,
    /// Optional human-readable note about this candidate.
    pub note: Option<&'static str>,
}

/// An environment variable that relocates a terminal's config, in priority order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ConfigLocationEnv {
    /// The environment variable name (e.g. `KITTY_CONFIG_DIRECTORY`).
    pub var: &'static str,
    /// Whether the variable points at a directory or a file.
    pub kind: ConfigEnvKind,
    /// Optional human-readable note.
    pub note: Option<&'static str>,
}

/// Ordered config-file candidates broken out per OS target.
///
/// Each list is 1:M and first-existing wins at resolve time. The buckets mirror
/// [`ConfigOsTarget`](super::ConfigOsTarget).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct OsConfigLocations {
    /// Candidates for a native Linux / BSD / illumos / unknown host.
    pub linux: &'static [ConfigCandidate],
    /// Candidates for macOS.
    pub macos: &'static [ConfigCandidate],
    /// Candidates for Windows.
    pub windows: &'static [ConfigCandidate],
    /// Candidates for WSL 1.
    pub wsl1: &'static [ConfigCandidate],
    /// Candidates for WSL 2.
    pub wsl2: &'static [ConfigCandidate],
}

impl OsConfigLocations {
    /// All-empty locations, for metadata-only apps (e.g. GNOME Terminal under dconf).
    pub const EMPTY: OsConfigLocations = OsConfigLocations {
        linux: &[],
        macos: &[],
        windows: &[],
        wsl1: &[],
        wsl2: &[],
    };

    /// The candidate list for a given OS target.
    pub fn for_target(&self, target: ConfigOsTarget) -> &'static [ConfigCandidate] {
        match target {
            ConfigOsTarget::Linux => self.linux,
            ConfigOsTarget::MacOS => self.macos,
            ConfigOsTarget::Windows => self.windows,
            ConfigOsTarget::Wsl1 => self.wsl1,
            ConfigOsTarget::Wsl2 => self.wsl2,
        }
    }
}

/// Where a named setting lives *inside* the config file.
///
/// The `path` is interpreted against the owning [`ConfigMetadata::format`]:
/// nested keys for structured formats (Toml/Yaml/Json/Json5/Plist), or a flat
/// key looked up verbatim for `KittyConf` / `KeyValue`. For `Lua` it is a logical
/// pointer only (value extraction is not attempted in v1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct SettingLocator {
    /// Dot path (structured) or flat key (flat formats).
    pub path: &'static str,
    /// Advisory hint for the expected value shape.
    pub value_kind: ValueKind,
    /// Optional human-readable note.
    pub note: Option<&'static str>,
}

/// Locators for the named settings a terminal exposes in its config file.
///
/// v1 guarantees the six core locators (`ipc`, `font`, `font_size`, `theme`,
/// `background_color`, `opacity`) are populated for supported apps; the extended
/// locators are populated opportunistically and are `None` otherwise. A `None`
/// extended locator is not a coverage gap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct SettingLocators {
    // --- v1 core (guaranteed for supported apps) ---
    /// IPC mechanism as declared in the config (e.g. kitty `allow_remote_control`).
    pub ipc: Option<SettingLocator>,
    /// Primary font family.
    pub font: Option<SettingLocator>,
    /// Font size.
    pub font_size: Option<SettingLocator>,
    /// Active theme.
    pub theme: Option<SettingLocator>,
    /// Background color.
    pub background_color: Option<SettingLocator>,
    /// Window/background opacity.
    pub opacity: Option<SettingLocator>,
    // --- v1 extended (opportunistic) ---
    /// Foreground/text color.
    pub foreground_color: Option<SettingLocator>,
    /// Cursor color.
    pub cursor_color: Option<SettingLocator>,
    /// Cursor style/shape.
    pub cursor_style: Option<SettingLocator>,
    /// Selection foreground/background colors.
    pub selection_colors: Option<SettingLocator>,
    /// Named color scheme/palette.
    pub color_scheme: Option<SettingLocator>,
    /// Bold-variant font.
    pub bold_font: Option<SettingLocator>,
    /// Italic-variant font.
    pub italic_font: Option<SettingLocator>,
    /// Line height / cell height.
    pub line_height: Option<SettingLocator>,
    /// Window padding.
    pub window_padding: Option<SettingLocator>,
    /// Scrollback buffer size in lines.
    pub scrollback_lines: Option<SettingLocator>,
    /// Shell/command launched by the terminal.
    pub shell_program: Option<SettingLocator>,
}

impl SettingLocators {
    /// All-`None` locators, a convenient base for seeding an app with only a few settings.
    pub const EMPTY: SettingLocators = SettingLocators {
        ipc: None,
        font: None,
        font_size: None,
        theme: None,
        background_color: None,
        opacity: None,
        foreground_color: None,
        cursor_color: None,
        cursor_style: None,
        selection_colors: None,
        color_scheme: None,
        bold_font: None,
        italic_font: None,
        line_height: None,
        window_padding: None,
        scrollback_lines: None,
        shell_program: None,
    };
}

/// Config-location + config-content metadata for one terminal app.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ConfigMetadata {
    /// The config file format, governing setting extraction.
    pub format: ConfigFormat,
    /// Config-file candidates per OS target.
    pub locations: OsConfigLocations,
    /// Config-relocating environment variables, in priority order.
    pub location_env: &'static [ConfigLocationEnv],
    /// Where named settings live inside the config file.
    pub settings: SettingLocators,
}

/// Runtime values a terminal exports into the environment.
///
/// Each field is an *ordered candidate list* (first set wins), because some facts
/// have aliases or vary by terminal version. These values are only *meaningful*
/// when the queried app is the current terminal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct EnvFactMap {
    /// Process id (e.g. `KITTY_PID`).
    pub pid: &'static [&'static str],
    /// Window id (e.g. `KITTY_WINDOW_ID`).
    pub window_id: &'static [&'static str],
    /// Pane id (e.g. `WEZTERM_PANE`).
    pub pane_id: &'static [&'static str],
    /// Public key (e.g. `KITTY_PUBLIC_KEY`).
    pub public_key: &'static [&'static str],
    /// Live IPC address (e.g. `KITTY_LISTEN_ON`, `WEZTERM_UNIX_SOCKET`).
    pub ipc_address: &'static [&'static str],
    /// Session id (e.g. `ITERM_SESSION_ID`, `WT_SESSION`).
    pub session_id: &'static [&'static str],
    /// Config directory (e.g. `KITTY_CONFIG_DIRECTORY`).
    pub config_dir: &'static [&'static str],
    /// Resources/installation directory (e.g. `GHOSTTY_RESOURCES_DIR`).
    pub resources_dir: &'static [&'static str],
    /// Version string (e.g. `TERM_PROGRAM_VERSION`).
    pub version: &'static [&'static str],
    /// Profile name (e.g. `ITERM_PROFILE`).
    pub profile: &'static [&'static str],
}

impl EnvFactMap {
    /// All-empty env facts, for apps that export nothing discoverable.
    pub const EMPTY: EnvFactMap = EnvFactMap {
        pid: &[],
        window_id: &[],
        pane_id: &[],
        public_key: &[],
        ipc_address: &[],
        session_id: &[],
        config_dir: &[],
        resources_dir: &[],
        version: &[],
        profile: &[],
    };
}

/// Top-level static metadata for one terminal app.
///
/// The `bin_name` / `bundle_id` probe targets are *declared* here but the library
/// never performs install detection with them — that is a CLI-only concern that
/// delegates to `sniff` (the library stays sniff-free).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct TerminalAppMetadata {
    /// Config-location + content metadata.
    pub config: ConfigMetadata,
    /// Runtime environment facts.
    pub env_facts: EnvFactMap,
    /// Binary name for `PATH` probing (CLI install detection), if any.
    pub bin_name: Option<&'static str>,
    /// macOS `.app` bundle id for bundle probing (CLI install detection), if any.
    pub bundle_id: Option<&'static str>,
}

/// The config file this host is actually using, plus why it resolved.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ResolvedConfig {
    /// The resolved config-file path.
    pub path: PathBuf,
    /// The format to use when reading this resolved file.
    pub format: ConfigFormat,
    /// How the path was chosen.
    pub source: ConfigSource,
}

/// Provenance for a [`ResolvedConfig`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum ConfigSource {
    /// Resolved via an environment override (the variable name).
    EnvVar(&'static str),
    /// Resolved from the OS target's candidate list at this index.
    Candidate(usize),
}
