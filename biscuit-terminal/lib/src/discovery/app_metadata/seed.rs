//! Seed metadata table for the terminals biscuit-terminal knows about.
//!
//! Every entry is `&'static`, vendor-determined data. The table is resolved by
//! `match self` over [`TerminalApp`] (no separate key type); variants the table
//! does not cover — `Wast`, `Other(_)` — map to `None`.
//!
//! Coverage floor: every app for which the legacy `get_terminal_config_path`
//! returned `Some` (WezTerm, Kitty, Ghostty, Alacritty, iTerm2, Apple Terminal,
//! Konsole, Foot, Contour, Warp, VS Code) contributes at least one candidate, so
//! the back-compat wrapper never regresses `Some -> None`. Windows Terminal is
//! new; GNOME Terminal is metadata-only (`Dconf`, empty candidate list).

use super::types::{
    ConfigCandidate, ConfigCandidateKind, ConfigEnvKind, ConfigFormat, ConfigLocationEnv,
    ConfigMetadata, EnvFactMap, OsConfigLocations, SettingLocator, SettingLocators,
    TerminalAppMetadata, ValueKind,
};
use crate::discovery::detection::TerminalApp;

/// A locator with no note (the common case).
const fn loc(path: &'static str, value_kind: ValueKind) -> Option<SettingLocator> {
    Some(SettingLocator {
        path,
        value_kind,
        note: None,
    })
}

/// A locator with an explanatory note for combined, absent, or external values.
const fn loc_note(
    path: &'static str,
    value_kind: ValueKind,
    note: &'static str,
) -> Option<SettingLocator> {
    Some(SettingLocator {
        path,
        value_kind,
        note: Some(note),
    })
}

/// A candidate template with no note.
const fn cand(template: &'static str) -> ConfigCandidate {
    ConfigCandidate {
        template,
        kind: ConfigCandidateKind::File,
        format: None,
        note: None,
    }
}

/// A candidate template whose on-disk format differs from the app's primary format.
const fn cand_format(
    template: &'static str,
    format: ConfigFormat,
    note: &'static str,
) -> ConfigCandidate {
    ConfigCandidate {
        template,
        kind: ConfigCandidateKind::File,
        format: Some(format),
        note: Some(note),
    }
}

/// A directory candidate whose contained config content uses the declared format.
const fn cand_dir_format(
    template: &'static str,
    format: ConfigFormat,
    note: &'static str,
) -> ConfigCandidate {
    ConfigCandidate {
        template,
        kind: ConfigCandidateKind::Directory,
        format: Some(format),
        note: Some(note),
    }
}

/// The static metadata for `app`, or `None` for uncovered variants.
///
/// See the module docs for the coverage floor this table must satisfy.
pub(crate) fn metadata_for(app: &TerminalApp) -> Option<&'static TerminalAppMetadata> {
    match app {
        TerminalApp::Kitty => Some(&KITTY),
        TerminalApp::Wezterm => Some(&WEZTERM),
        TerminalApp::Alacritty => Some(&ALACRITTY),
        TerminalApp::Ghostty => Some(&GHOSTTY),
        TerminalApp::ITerm2 => Some(&ITERM2),
        TerminalApp::AppleTerminal => Some(&APPLE_TERMINAL),
        TerminalApp::WindowsTerminal => Some(&WINDOWS_TERMINAL),
        TerminalApp::Warp => Some(&WARP),
        TerminalApp::GnomeTerminal => Some(&GNOME_TERMINAL),
        TerminalApp::VsCode => Some(&VS_CODE),
        TerminalApp::Konsole => Some(&KONSOLE),
        TerminalApp::Foot => Some(&FOOT),
        TerminalApp::Contour => Some(&CONTOUR),
        TerminalApp::Wast | TerminalApp::Other(_) => None,
    }
}

// --- Kitty (KittyConf) -----------------------------------------------------

static KITTY_XDG: &[ConfigCandidate] = &[cand("$XDG_CONFIG_HOME/kitty/kitty.conf")];

static KITTY: TerminalAppMetadata = TerminalAppMetadata {
    config: ConfigMetadata {
        format: ConfigFormat::KittyConf,
        locations: OsConfigLocations {
            linux: KITTY_XDG,
            macos: KITTY_XDG,
            windows: &[],
            wsl1: KITTY_XDG,
            wsl2: KITTY_XDG,
        },
        location_env: &[ConfigLocationEnv {
            var: "KITTY_CONFIG_DIRECTORY",
            kind: ConfigEnvKind::Dir,
            note: None,
        }],
        settings: SettingLocators {
            ipc: loc("allow_remote_control", ValueKind::Enum),
            font: loc("font_family", ValueKind::String),
            font_size: loc("font_size", ValueKind::Number),
            theme: loc_note(
                "include",
                ValueKind::String,
                "themes are commonly loaded through include files",
            ),
            background_color: loc("background", ValueKind::Color),
            opacity: loc("background_opacity", ValueKind::Number),
            foreground_color: loc("foreground", ValueKind::Color),
            cursor_color: loc("cursor", ValueKind::Color),
            cursor_style: loc("cursor_shape", ValueKind::Enum),
            selection_colors: loc("selection_background", ValueKind::Color),
            color_scheme: None,
            bold_font: loc("bold_font", ValueKind::String),
            italic_font: loc("italic_font", ValueKind::String),
            line_height: None,
            window_padding: loc("window_padding_width", ValueKind::Number),
            scrollback_lines: loc("scrollback_lines", ValueKind::Number),
            shell_program: loc("shell", ValueKind::String),
        },
    },
    env_facts: EnvFactMap {
        pid: &["KITTY_PID"],
        window_id: &["KITTY_WINDOW_ID"],
        pane_id: &[],
        public_key: &["KITTY_PUBLIC_KEY"],
        ipc_address: &["KITTY_LISTEN_ON"],
        session_id: &[],
        config_dir: &["KITTY_CONFIG_DIRECTORY"],
        resources_dir: &["KITTY_INSTALLATION_DIR"],
        version: &["TERM_PROGRAM_VERSION"],
        profile: &[],
    },
    bin_name: Some("kitty"),
    bundle_id: Some("net.kovidgoyal.kitty"),
};

// --- WezTerm (Lua, locator-only) -------------------------------------------

static WEZTERM_CANDIDATES: &[ConfigCandidate] = &[
    cand("$XDG_CONFIG_HOME/wezterm/wezterm.lua"),
    cand("~/.wezterm.lua"),
];

static WEZTERM: TerminalAppMetadata = TerminalAppMetadata {
    config: ConfigMetadata {
        format: ConfigFormat::Lua,
        locations: OsConfigLocations {
            linux: WEZTERM_CANDIDATES,
            macos: WEZTERM_CANDIDATES,
            windows: WEZTERM_CANDIDATES,
            wsl1: WEZTERM_CANDIDATES,
            wsl2: WEZTERM_CANDIDATES,
        },
        location_env: &[ConfigLocationEnv {
            var: "WEZTERM_CONFIG_FILE",
            kind: ConfigEnvKind::File,
            note: None,
        }],
        // Lua is not statically parseable; these are logical pointers only (§6).
        settings: SettingLocators {
            ipc: loc("unix_domains", ValueKind::String),
            font: loc("font", ValueKind::String),
            font_size: loc("font_size", ValueKind::Number),
            theme: loc("color_scheme", ValueKind::String),
            background_color: loc("colors.background", ValueKind::Color),
            opacity: loc("window_background_opacity", ValueKind::Number),
            foreground_color: loc("colors.foreground", ValueKind::Color),
            cursor_color: loc("colors.cursor_bg", ValueKind::Color),
            cursor_style: loc("default_cursor_style", ValueKind::Enum),
            selection_colors: None,
            color_scheme: loc("color_scheme", ValueKind::String),
            bold_font: None,
            italic_font: None,
            line_height: loc("line_height", ValueKind::Number),
            window_padding: loc("window_padding", ValueKind::Number),
            scrollback_lines: loc("scrollback_lines", ValueKind::Number),
            shell_program: loc("default_prog", ValueKind::String),
        },
    },
    env_facts: EnvFactMap {
        pid: &[],
        window_id: &[],
        pane_id: &["WEZTERM_PANE"],
        public_key: &[],
        ipc_address: &["WEZTERM_UNIX_SOCKET"],
        session_id: &[],
        config_dir: &["WEZTERM_CONFIG_DIR"],
        resources_dir: &[],
        version: &["TERM_PROGRAM_VERSION"],
        profile: &[],
    },
    bin_name: Some("wezterm"),
    bundle_id: Some("com.github.wez.wezterm"),
};

// --- Alacritty (Toml) ------------------------------------------------------

static ALACRITTY_UNIX: &[ConfigCandidate] = &[
    cand("$XDG_CONFIG_HOME/alacritty/alacritty.toml"),
    cand_format(
        "$XDG_CONFIG_HOME/alacritty/alacritty.yml",
        ConfigFormat::Yaml,
        "legacy YAML config",
    ),
    cand("~/.alacritty.toml"),
    cand_format("~/.alacritty.yml", ConfigFormat::Yaml, "legacy YAML config"),
];

static ALACRITTY_WINDOWS: &[ConfigCandidate] = &[
    cand("$APPDATA/alacritty/alacritty.toml"),
    cand_format(
        "$APPDATA/alacritty/alacritty.yml",
        ConfigFormat::Yaml,
        "legacy YAML config",
    ),
];

static ALACRITTY: TerminalAppMetadata = TerminalAppMetadata {
    config: ConfigMetadata {
        format: ConfigFormat::Toml,
        locations: OsConfigLocations {
            linux: ALACRITTY_UNIX,
            macos: ALACRITTY_UNIX,
            windows: ALACRITTY_WINDOWS,
            wsl1: ALACRITTY_UNIX,
            wsl2: ALACRITTY_UNIX,
        },
        location_env: &[ConfigLocationEnv {
            var: "XDG_CONFIG_HOME",
            kind: ConfigEnvKind::Dir,
            note: None,
        }],
        settings: SettingLocators {
            ipc: loc_note(
                "ipc",
                ValueKind::String,
                "no config-declared IPC setting; live IPC is reported from environment facts when present",
            ),
            font: loc("font.normal.family", ValueKind::String),
            font_size: loc("font.size", ValueKind::Number),
            theme: loc_note("import", ValueKind::String, "themes are commonly imported from external files"),
            background_color: loc("colors.primary.background", ValueKind::Color),
            opacity: loc("window.opacity", ValueKind::Number),
            foreground_color: loc("colors.primary.foreground", ValueKind::Color),
            cursor_color: loc("colors.cursor.cursor", ValueKind::Color),
            cursor_style: loc("cursor.style.shape", ValueKind::Enum),
            selection_colors: loc("colors.selection.background", ValueKind::Color),
            color_scheme: None,
            bold_font: loc("font.bold.family", ValueKind::String),
            italic_font: loc("font.italic.family", ValueKind::String),
            line_height: None,
            window_padding: loc("window.padding.x", ValueKind::Number),
            scrollback_lines: loc("scrolling.history", ValueKind::Number),
            shell_program: loc("terminal.shell.program", ValueKind::Path),
        },
    },
    env_facts: EnvFactMap {
        pid: &[],
        window_id: &["ALACRITTY_WINDOW_ID"],
        pane_id: &[],
        public_key: &[],
        ipc_address: &["ALACRITTY_SOCKET"],
        session_id: &[],
        config_dir: &[],
        resources_dir: &[],
        version: &["TERM_PROGRAM_VERSION"],
        profile: &[],
    },
    bin_name: Some("alacritty"),
    bundle_id: Some("org.alacritty"),
};

// --- Ghostty (KeyValue) ----------------------------------------------------

static GHOSTTY_MACOS: &[ConfigCandidate] = &[
    cand("~/Library/Application Support/com.mitchellh.ghostty/config"),
    cand("$XDG_CONFIG_HOME/ghostty/config"),
];

static GHOSTTY_LINUX: &[ConfigCandidate] = &[cand("$XDG_CONFIG_HOME/ghostty/config")];

static GHOSTTY: TerminalAppMetadata = TerminalAppMetadata {
    config: ConfigMetadata {
        format: ConfigFormat::KeyValue,
        locations: OsConfigLocations {
            linux: GHOSTTY_LINUX,
            macos: GHOSTTY_MACOS,
            windows: &[],
            wsl1: GHOSTTY_LINUX,
            wsl2: GHOSTTY_LINUX,
        },
        location_env: &[],
        settings: SettingLocators {
            ipc: loc_note(
                "ipc",
                ValueKind::String,
                "no config-declared IPC setting; Ghostty does not expose a v1 IPC locator",
            ),
            font: loc("font-family", ValueKind::String),
            font_size: loc("font-size", ValueKind::Number),
            theme: loc("theme", ValueKind::String),
            background_color: loc("background", ValueKind::Color),
            opacity: loc("background-opacity", ValueKind::Number),
            foreground_color: loc("foreground", ValueKind::Color),
            cursor_color: loc("cursor-color", ValueKind::Color),
            cursor_style: loc("cursor-style", ValueKind::Enum),
            selection_colors: loc("selection-background", ValueKind::Color),
            color_scheme: None,
            bold_font: None,
            italic_font: None,
            line_height: loc("adjust-cell-height", ValueKind::Number),
            window_padding: loc("window-padding-x", ValueKind::Number),
            scrollback_lines: loc("scrollback-limit", ValueKind::Number),
            shell_program: loc("command", ValueKind::String),
        },
    },
    env_facts: EnvFactMap {
        pid: &[],
        window_id: &[],
        pane_id: &[],
        public_key: &[],
        ipc_address: &[],
        session_id: &[],
        config_dir: &[],
        resources_dir: &["GHOSTTY_RESOURCES_DIR"],
        version: &["TERM_PROGRAM_VERSION"],
        profile: &[],
    },
    bin_name: Some("ghostty"),
    bundle_id: Some("com.mitchellh.ghostty"),
};

// --- iTerm2 (Plist, macOS) -------------------------------------------------

static ITERM2_MACOS: &[ConfigCandidate] =
    &[cand("~/Library/Preferences/com.googlecode.iterm2.plist")];

static ITERM2: TerminalAppMetadata = TerminalAppMetadata {
    config: ConfigMetadata {
        format: ConfigFormat::Plist,
        locations: OsConfigLocations {
            linux: &[],
            macos: ITERM2_MACOS,
            windows: &[],
            wsl1: &[],
            wsl2: &[],
        },
        location_env: &[],
        // iTerm2 stores per-profile settings under the "New Bookmarks" array;
        // these locators target the first profile (§6, best-effort).
        settings: SettingLocators {
            ipc: loc_note(
                "New Bookmarks.0.Guid",
                ValueKind::String,
                "profile identity only; iTerm2 does not expose config-declared IPC",
            ),
            font: Some(SettingLocator {
                path: "New Bookmarks.0.Normal Font",
                value_kind: ValueKind::String,
                note: Some("first profile; font+size combined"),
            }),
            font_size: loc_note(
                "New Bookmarks.0.Normal Font",
                ValueKind::Number,
                "first profile; font+size combined",
            ),
            theme: loc_note(
                "New Bookmarks.0.Guid",
                ValueKind::String,
                "iTerm2 profile acts as the theme boundary; color preset name is not persisted here",
            ),
            background_color: loc("New Bookmarks.0.Background Color", ValueKind::Color),
            opacity: loc("New Bookmarks.0.Transparency", ValueKind::Number),
            foreground_color: loc("New Bookmarks.0.Foreground Color", ValueKind::Color),
            cursor_color: loc("New Bookmarks.0.Cursor Color", ValueKind::Color),
            cursor_style: None,
            selection_colors: loc("New Bookmarks.0.Selection Color", ValueKind::Color),
            color_scheme: None,
            bold_font: None,
            italic_font: None,
            line_height: None,
            window_padding: None,
            scrollback_lines: loc("New Bookmarks.0.Scrollback Lines", ValueKind::Number),
            shell_program: loc("New Bookmarks.0.Command", ValueKind::String),
        },
    },
    env_facts: EnvFactMap {
        pid: &[],
        window_id: &[],
        pane_id: &[],
        public_key: &[],
        ipc_address: &[],
        session_id: &["ITERM_SESSION_ID"],
        config_dir: &[],
        resources_dir: &[],
        version: &["TERM_PROGRAM_VERSION"],
        profile: &["ITERM_PROFILE"],
    },
    bin_name: None,
    bundle_id: Some("com.googlecode.iterm2"),
};

// --- Apple Terminal (Plist, macOS) -----------------------------------------

static APPLE_TERMINAL_MACOS: &[ConfigCandidate] =
    &[cand("~/Library/Preferences/com.apple.Terminal.plist")];

static APPLE_TERMINAL: TerminalAppMetadata = TerminalAppMetadata {
    config: ConfigMetadata {
        format: ConfigFormat::Plist,
        locations: OsConfigLocations {
            linux: &[],
            macos: APPLE_TERMINAL_MACOS,
            windows: &[],
            wsl1: &[],
            wsl2: &[],
        },
        location_env: &[],
        // Settings live under a deeply nested "Window Settings" dict keyed by
        // profile name, so these locators use a placeholder profile segment.
        settings: SettingLocators {
            ipc: loc_note(
                "Window Settings.<profile>",
                ValueKind::String,
                "no config-declared IPC setting; Terminal.app does not expose terminal IPC",
            ),
            font: loc_note(
                "Window Settings.<profile>.Font",
                ValueKind::String,
                "profile key is dynamic",
            ),
            font_size: loc_note(
                "Window Settings.<profile>.Font",
                ValueKind::Number,
                "profile key is dynamic; font+size combined",
            ),
            theme: loc_note(
                "Window Settings.<profile>",
                ValueKind::String,
                "profile key is dynamic; the profile itself is the theme boundary",
            ),
            background_color: loc_note(
                "Window Settings.<profile>.BackgroundColor",
                ValueKind::Color,
                "profile key is dynamic",
            ),
            opacity: loc_note(
                "Window Settings.<profile>.BackgroundColor",
                ValueKind::Number,
                "profile key is dynamic; opacity is encoded with the background color",
            ),
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
        },
    },
    env_facts: EnvFactMap {
        pid: &[],
        window_id: &[],
        pane_id: &[],
        public_key: &[],
        ipc_address: &[],
        session_id: &["TERM_SESSION_ID"],
        config_dir: &[],
        resources_dir: &[],
        version: &["TERM_PROGRAM_VERSION"],
        profile: &[],
    },
    bin_name: None,
    bundle_id: Some("com.apple.Terminal"),
};

// --- Windows Terminal (Json) -----------------------------------------------

static WINDOWS_TERMINAL_WIN: &[ConfigCandidate] = &[cand(
    "$LOCALAPPDATA\\Packages\\Microsoft.WindowsTerminal_*\\LocalState\\settings.json",
)];

static WINDOWS_TERMINAL_WSL: &[ConfigCandidate] = &[cand(
    "$WIN_HOME/AppData/Local/Packages/Microsoft.WindowsTerminal_*/LocalState/settings.json",
)];

static WINDOWS_TERMINAL: TerminalAppMetadata = TerminalAppMetadata {
    config: ConfigMetadata {
        format: ConfigFormat::Json,
        locations: OsConfigLocations {
            linux: &[],
            macos: &[],
            windows: WINDOWS_TERMINAL_WIN,
            wsl1: WINDOWS_TERMINAL_WSL,
            wsl2: WINDOWS_TERMINAL_WSL,
        },
        location_env: &[],
        settings: SettingLocators {
            ipc: loc_note(
                "ipc",
                ValueKind::String,
                "no config-declared IPC setting; live session identity is reported from WT_SESSION",
            ),
            font: loc("profiles.defaults.font.face", ValueKind::String),
            font_size: loc("profiles.defaults.font.size", ValueKind::Number),
            theme: loc("theme", ValueKind::String),
            background_color: loc("profiles.defaults.background", ValueKind::Color),
            opacity: loc("profiles.defaults.opacity", ValueKind::Number),
            foreground_color: loc("profiles.defaults.foreground", ValueKind::Color),
            cursor_color: loc("profiles.defaults.cursorColor", ValueKind::Color),
            cursor_style: loc("profiles.defaults.cursorShape", ValueKind::Enum),
            selection_colors: loc("profiles.defaults.selectionBackground", ValueKind::Color),
            color_scheme: loc("profiles.defaults.colorScheme", ValueKind::String),
            bold_font: None,
            italic_font: None,
            line_height: None,
            window_padding: loc("profiles.defaults.padding", ValueKind::String),
            scrollback_lines: loc("profiles.defaults.historySize", ValueKind::Number),
            shell_program: loc("profiles.defaults.commandline", ValueKind::String),
        },
    },
    env_facts: EnvFactMap {
        pid: &[],
        window_id: &[],
        pane_id: &[],
        public_key: &[],
        ipc_address: &[],
        session_id: &["WT_SESSION"],
        config_dir: &[],
        resources_dir: &[],
        version: &[],
        profile: &["WT_PROFILE_ID"],
    },
    bin_name: Some("wt"),
    bundle_id: None,
};

// --- Warp -------------------------------------------------------------------
//
// `warp_config_path` points at the `~/.warp` directory (user themes and launch
// configs, which are YAML). Warp's primary settings are managed in-app and are
// not exposed as a parseable flat file, so settings are locator-only; the
// candidate is kept to hold the coverage floor (§8 Warp note).

static WARP_CANDIDATES: &[ConfigCandidate] = &[cand_dir_format(
    "~/.warp",
    ConfigFormat::Yaml,
    "Warp config directory; user themes and launch configs are YAML, primary settings are managed in-app",
)];

static WARP: TerminalAppMetadata = TerminalAppMetadata {
    config: ConfigMetadata {
        format: ConfigFormat::Yaml,
        locations: OsConfigLocations {
            linux: WARP_CANDIDATES,
            macos: WARP_CANDIDATES,
            windows: &[],
            wsl1: WARP_CANDIDATES,
            wsl2: WARP_CANDIDATES,
        },
        location_env: &[],
        settings: SettingLocators {
            ipc: loc_note(
                "in-app.ipc",
                ValueKind::String,
                "primary setting is managed in-app and not exposed in the Warp config directory",
            ),
            font: loc_note(
                "in-app.font",
                ValueKind::String,
                "primary setting is managed in-app and not exposed in the Warp config directory",
            ),
            font_size: loc_note(
                "in-app.font_size",
                ValueKind::Number,
                "primary setting is managed in-app and not exposed in the Warp config directory",
            ),
            theme: loc_note(
                "themes",
                ValueKind::String,
                "user themes may exist under ~/.warp/themes; the active theme is managed in-app",
            ),
            background_color: loc_note(
                "themes",
                ValueKind::Color,
                "user themes may exist under ~/.warp/themes; the active background is managed in-app",
            ),
            opacity: loc_note(
                "in-app.opacity",
                ValueKind::Number,
                "primary setting is managed in-app and not exposed in the Warp config directory",
            ),
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
        },
    },
    env_facts: EnvFactMap {
        pid: &[],
        window_id: &[],
        pane_id: &[],
        public_key: &[],
        ipc_address: &[],
        session_id: &[],
        config_dir: &[],
        resources_dir: &[],
        version: &["TERM_PROGRAM_VERSION"],
        profile: &[],
    },
    bin_name: None,
    bundle_id: Some("dev.warp.Warp-Stable"),
};

// --- GNOME Terminal (Dconf, metadata-only) ---------------------------------

static GNOME_TERMINAL: TerminalAppMetadata = TerminalAppMetadata {
    config: ConfigMetadata {
        format: ConfigFormat::Dconf,
        // No flat file: GNOME Terminal is managed by dconf/gsettings (§8).
        locations: OsConfigLocations::EMPTY,
        location_env: &[],
        settings: SettingLocators {
            ipc: loc_note(
                "org.gnome.Terminal.Legacy",
                ValueKind::String,
                "no config-declared IPC setting; GNOME Terminal settings are managed by dconf",
            ),
            font: loc_note(
                "org.gnome.Terminal.Legacy.Profile.font",
                ValueKind::String,
                "dconf/gsettings locator-only",
            ),
            font_size: loc_note(
                "org.gnome.Terminal.Legacy.Profile.font",
                ValueKind::Number,
                "dconf/gsettings locator-only; font+size combined",
            ),
            theme: loc_note(
                "org.gnome.Terminal.Legacy.Profile.use-theme-colors",
                ValueKind::Bool,
                "dconf/gsettings locator-only",
            ),
            background_color: loc_note(
                "org.gnome.Terminal.Legacy.Profile.background-color",
                ValueKind::Color,
                "dconf/gsettings locator-only",
            ),
            opacity: loc_note(
                "org.gnome.Terminal.Legacy.Profile.background-transparency-percent",
                ValueKind::Number,
                "dconf/gsettings locator-only",
            ),
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
        },
    },
    env_facts: EnvFactMap {
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
    },
    bin_name: Some("gnome-terminal"),
    bundle_id: None,
};

// --- VS Code (Json5 / JSONC) -----------------------------------------------

static VS_CODE_MACOS: &[ConfigCandidate] =
    &[cand("~/Library/Application Support/Code/User/settings.json")];
static VS_CODE_LINUX: &[ConfigCandidate] = &[cand("$XDG_CONFIG_HOME/Code/User/settings.json")];
static VS_CODE_WINDOWS: &[ConfigCandidate] = &[cand("$APPDATA/Code/User/settings.json")];

static VS_CODE: TerminalAppMetadata = TerminalAppMetadata {
    config: ConfigMetadata {
        format: ConfigFormat::Json5,
        locations: OsConfigLocations {
            linux: VS_CODE_LINUX,
            macos: VS_CODE_MACOS,
            windows: VS_CODE_WINDOWS,
            wsl1: VS_CODE_LINUX,
            wsl2: VS_CODE_LINUX,
        },
        location_env: &[],
        settings: SettingLocators {
            ipc: loc_note(
                "terminal.integrated",
                ValueKind::String,
                "VS Code's integrated terminal does not expose config-declared terminal IPC",
            ),
            font: loc("terminal.integrated.fontFamily", ValueKind::String),
            font_size: loc("terminal.integrated.fontSize", ValueKind::Number),
            theme: loc("workbench.colorTheme", ValueKind::String),
            background_color: loc_note(
                "workbench.colorCustomizations.terminal.background",
                ValueKind::Color,
                "optional override; otherwise inherited from the active workbench theme",
            ),
            opacity: loc_note(
                "window.opacity",
                ValueKind::Number,
                "window-level setting, not specific to the integrated terminal",
            ),
            foreground_color: None,
            cursor_color: None,
            cursor_style: loc("terminal.integrated.cursorStyle", ValueKind::Enum),
            selection_colors: None,
            color_scheme: None,
            bold_font: None,
            italic_font: None,
            line_height: loc("terminal.integrated.lineHeight", ValueKind::Number),
            window_padding: None,
            scrollback_lines: loc("terminal.integrated.scrollback", ValueKind::Number),
            shell_program: None,
        },
    },
    env_facts: EnvFactMap {
        pid: &[],
        window_id: &[],
        pane_id: &[],
        public_key: &[],
        ipc_address: &[],
        session_id: &[],
        config_dir: &[],
        resources_dir: &[],
        version: &["TERM_PROGRAM_VERSION"],
        profile: &[],
    },
    bin_name: Some("code"),
    bundle_id: Some("com.microsoft.VSCode"),
};

// --- Konsole (KeyValue) ----------------------------------------------------

static KONSOLE_LINUX: &[ConfigCandidate] = &[cand("$XDG_CONFIG_HOME/konsolerc")];

static KONSOLE: TerminalAppMetadata = TerminalAppMetadata {
    config: ConfigMetadata {
        format: ConfigFormat::KeyValue,
        locations: OsConfigLocations {
            linux: KONSOLE_LINUX,
            macos: &[],
            windows: &[],
            wsl1: KONSOLE_LINUX,
            wsl2: KONSOLE_LINUX,
        },
        location_env: &[],
        // konsolerc holds global state; per-terminal appearance lives in separate
        // *.profile files, so appearance locators point at that external profile scope.
        settings: SettingLocators {
            ipc: loc_note(
                "ipc",
                ValueKind::String,
                "no config-declared IPC setting; Konsole does not expose a v1 IPC locator",
            ),
            font: loc_note("Appearance.Font", ValueKind::String, "stored in the active *.profile file"),
            font_size: loc_note(
                "Appearance.Font",
                ValueKind::Number,
                "stored in the active *.profile file; font+size combined",
            ),
            theme: loc_note("Desktop Entry.DefaultProfile", ValueKind::String, "active profile selector"),
            background_color: loc_note(
                "Appearance.ColorScheme",
                ValueKind::Color,
                "profile points at an external color-scheme file",
            ),
            opacity: loc_note(
                "Appearance.ColorScheme",
                ValueKind::Number,
                "profile points at an external color-scheme file; opacity is scheme-defined when present",
            ),
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
        },
    },
    env_facts: EnvFactMap {
        pid: &[],
        window_id: &[],
        pane_id: &[],
        public_key: &[],
        ipc_address: &[],
        session_id: &[],
        config_dir: &[],
        resources_dir: &[],
        version: &["KONSOLE_VERSION"],
        profile: &[],
    },
    bin_name: Some("konsole"),
    bundle_id: None,
};

// --- Foot (KeyValue / ini) -------------------------------------------------

static FOOT_LINUX: &[ConfigCandidate] = &[cand("$XDG_CONFIG_HOME/foot/foot.ini")];

static FOOT: TerminalAppMetadata = TerminalAppMetadata {
    config: ConfigMetadata {
        format: ConfigFormat::KeyValue,
        locations: OsConfigLocations {
            linux: FOOT_LINUX,
            macos: &[],
            windows: &[],
            wsl1: FOOT_LINUX,
            wsl2: FOOT_LINUX,
        },
        location_env: &[],
        settings: SettingLocators {
            ipc: loc_note(
                "ipc",
                ValueKind::String,
                "no config-declared IPC setting; foot does not expose a v1 IPC locator",
            ),
            font: Some(SettingLocator {
                path: "font",
                value_kind: ValueKind::String,
                note: Some("family and size combined, e.g. 'Fira Code:size=11'"),
            }),
            font_size: loc_note(
                "font",
                ValueKind::Number,
                "family and size combined, e.g. 'Fira Code:size=11'",
            ),
            theme: loc_note(
                "include",
                ValueKind::String,
                "themes are commonly loaded through include files",
            ),
            background_color: loc("background", ValueKind::Color),
            opacity: loc("alpha", ValueKind::Number),
            foreground_color: loc("foreground", ValueKind::Color),
            cursor_color: None,
            cursor_style: loc("style", ValueKind::Enum),
            selection_colors: None,
            color_scheme: None,
            bold_font: loc("font-bold", ValueKind::String),
            italic_font: loc("font-italic", ValueKind::String),
            line_height: None,
            window_padding: loc("pad", ValueKind::String),
            scrollback_lines: loc("lines", ValueKind::Number),
            shell_program: loc("shell", ValueKind::Path),
        },
    },
    env_facts: EnvFactMap::EMPTY,
    bin_name: Some("foot"),
    bundle_id: None,
};

// --- Contour (Yaml) --------------------------------------------------------

static CONTOUR_MACOS: &[ConfigCandidate] =
    &[cand("~/Library/Application Support/contour/contour.yml")];
static CONTOUR_LINUX: &[ConfigCandidate] = &[cand("$XDG_CONFIG_HOME/contour/contour.yml")];
static CONTOUR_WINDOWS: &[ConfigCandidate] = &[cand("$LOCALAPPDATA/contour/contour.yml")];

static CONTOUR: TerminalAppMetadata = TerminalAppMetadata {
    config: ConfigMetadata {
        format: ConfigFormat::Yaml,
        locations: OsConfigLocations {
            linux: CONTOUR_LINUX,
            macos: CONTOUR_MACOS,
            windows: CONTOUR_WINDOWS,
            wsl1: CONTOUR_LINUX,
            wsl2: CONTOUR_LINUX,
        },
        location_env: &[],
        settings: SettingLocators {
            ipc: loc_note(
                "ipc",
                ValueKind::String,
                "no config-declared IPC setting; Contour does not expose a v1 IPC locator",
            ),
            font: loc("profiles.main.font.regular", ValueKind::String),
            font_size: loc("profiles.main.font.size", ValueKind::Number),
            theme: loc_note("profiles.main.colors", ValueKind::String, "color-scheme reference"),
            background_color: loc_note(
                "profiles.main.colors.default.background",
                ValueKind::Color,
                "background may be inherited from the referenced color scheme",
            ),
            opacity: loc("profiles.main.background.opacity", ValueKind::Number),
            foreground_color: None,
            cursor_color: None,
            cursor_style: loc("profiles.main.cursor.shape", ValueKind::Enum),
            selection_colors: None,
            color_scheme: loc("profiles.main.colors", ValueKind::String),
            bold_font: None,
            italic_font: None,
            line_height: None,
            window_padding: None,
            scrollback_lines: loc("profiles.main.history.limit", ValueKind::Number),
            shell_program: loc("profiles.main.shell", ValueKind::Path),
        },
    },
    env_facts: EnvFactMap::EMPTY,
    bin_name: Some("contour"),
    bundle_id: None,
};

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::ConfigOsTarget;

    #[test]
    fn metadata_entries_have_v1_core_setting_locators() {
        let apps = [
            TerminalApp::Kitty,
            TerminalApp::Wezterm,
            TerminalApp::Alacritty,
            TerminalApp::Ghostty,
            TerminalApp::ITerm2,
            TerminalApp::AppleTerminal,
            TerminalApp::WindowsTerminal,
            TerminalApp::Warp,
            TerminalApp::GnomeTerminal,
            TerminalApp::VsCode,
            TerminalApp::Konsole,
            TerminalApp::Foot,
            TerminalApp::Contour,
        ];

        let mut missing = Vec::new();
        for app in apps {
            let metadata = metadata_for(&app).expect("metadata-covered app should have metadata");
            let settings = metadata.config.settings;
            for (field, locator) in [
                ("ipc", settings.ipc),
                ("font", settings.font),
                ("font_size", settings.font_size),
                ("theme", settings.theme),
                ("background_color", settings.background_color),
                ("opacity", settings.opacity),
            ] {
                if locator.is_none() {
                    missing.push(format!("{app:?}.{field}"));
                }
            }
        }

        assert!(
            missing.is_empty(),
            "metadata-covered apps must define all v1 core locators: {}",
            missing.join(", ")
        );
    }

    #[test]
    fn floor_bound_apps_do_not_use_none_format() {
        let floor = [
            TerminalApp::Wezterm,
            TerminalApp::Kitty,
            TerminalApp::Ghostty,
            TerminalApp::Alacritty,
            TerminalApp::ITerm2,
            TerminalApp::AppleTerminal,
            TerminalApp::Konsole,
            TerminalApp::Foot,
            TerminalApp::Contour,
            TerminalApp::Warp,
            TerminalApp::VsCode,
        ];

        for app in floor {
            let metadata = metadata_for(&app).expect("floor-bound app should have metadata");
            assert_ne!(
                metadata.config.format,
                ConfigFormat::None,
                "{app:?} must not use ConfigFormat::None"
            );

            for target in [
                ConfigOsTarget::Linux,
                ConfigOsTarget::MacOS,
                ConfigOsTarget::Windows,
                ConfigOsTarget::Wsl1,
                ConfigOsTarget::Wsl2,
            ] {
                for candidate in metadata.config.locations.for_target(target) {
                    assert_ne!(
                        candidate.format,
                        Some(ConfigFormat::None),
                        "{app:?} {target:?} candidate must not override to ConfigFormat::None"
                    );
                }
            }
        }
    }
}
