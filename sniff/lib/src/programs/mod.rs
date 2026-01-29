//! Program detection module for identifying installed software.
//!
//! This module provides comprehensive detection of installed programs across
//! multiple categories including editors, utilities, package managers, TTS clients,
//! terminal emulators, and AI CLI tools.
//!
//! ## Categories
//!
//! - **Editors**: Text editors and IDEs (Vim, VS Code, IntelliJ, etc.)
//! - **Utilities**: Modern CLI tools (ripgrep, bat, fzf, etc.)
//! - **Language Package Managers**: npm, cargo, pip, etc.
//! - **OS Package Managers**: brew, apt, dnf, etc.
//! - **TTS Clients**: Text-to-speech tools (say, espeak, piper, etc.)
//! - **Terminal Apps**: Terminal emulators (alacritty, kitty, wezterm, etc.)
//! - **Headless Audio**: Background audio players (afplay, pacat, aplay, etc.)
//! - **AI CLI Tools**: AI-powered coding assistants (claude, aider, goose, etc.)
//!
//! ## Usage
//!
//! ```no_run
//! use sniff_lib::programs::{ProgramsInfo, ProgramMetadata, Editor};
//!
//! // Detect all installed programs
//! let programs = ProgramsInfo::detect();
//!
//! // Check specific categories
//! for editor in programs.editors.installed() {
//!     println!("{}: {}", editor.display_name(), editor.website());
//! }
//!
//! // Get path and version for a specific program
//! if let Some(path) = programs.editors.path(Editor::Vim) {
//!     println!("Vim found at: {}", path.display());
//! }
//! ```
//!
//! ## Enums and Metadata
//!
//! Each program category has a corresponding enum (e.g., `Editor`, `Utility`)
//! that implements the `ProgramMetadata` trait, providing:
//!
//! - `binary_name()` - The executable name
//! - `display_name()` - Human-readable name
//! - `description()` - Brief description
//! - `website()` - Official website URL
//! - `path()` - Path to the binary if installed
//! - `version()` - Version string if available
//!
//! ## macOS App Bundle Detection
//!
//! On macOS, some applications are installed as `.app` bundles rather than
//! command-line executables in PATH. This module provides fallback detection
//! for these bundles when the traditional PATH lookup fails.
//!
//! ### How It Works
//!
//! When using [`find_program_with_source`] or [`find_programs_with_source_parallel`],
//! the detection follows this order:
//!
//! 1. **PATH lookup** (priority) - Traditional executable search
//! 2. **macOS app bundles** (fallback) - Searches `/Applications` and `~/Applications`
//!
//! The [`ExecutableSource`] enum indicates how the program was discovered:
//! - [`ExecutableSource::Path`] - Found via PATH lookup
//! - [`ExecutableSource::MacOsAppBundle`] - Found as a macOS `.app` bundle
//!
//! ### Supported Applications
//!
//! The bundle detection includes mappings for common applications:
//! - **Editors**: VS Code (`code`), Cursor, Zed
//! - **Terminals**: WezTerm, Alacritty, kitty, iTerm2, Ghostty
//! - **Browsers**: Brave, Chrome, Firefox
//! - **Media**: VLC, Spotify
//! - **Communication**: Slack, Discord
//!
//! ### Example
//!
//! ```no_run
//! use sniff_lib::programs::{find_program_with_source, ExecutableSource};
//!
//! // Find VS Code - checks PATH first, then macOS app bundles
//! if let Some((path, source)) = find_program_with_source("code") {
//!     match source {
//!         ExecutableSource::Path => println!("Found in PATH: {}", path.display()),
//!         ExecutableSource::MacOsAppBundle => println!("Found as macOS app: {}", path.display()),
//!     }
//! }
//! ```
//!
//! ### Platform Behavior
//!
//! - **macOS**: Full bundle detection support
//! - **Linux/Windows**: Bundle detection returns `None` (PATH-only)

pub mod ai_cli;
pub mod editors;
pub mod enums;
pub mod find_program;
pub mod headless_audio;
pub mod installer;
pub mod inventory;
pub mod macos_bundle;
pub mod pkg_mngrs;
pub mod schema;
pub mod terminal_apps;
pub mod tts_clients;
pub mod types;
pub mod utilities;

use serde::{Deserialize, Serialize};

pub use ai_cli::InstalledAiClients;
pub use editors::InstalledEditors;
pub use enums::{
    AiCli, Editor, HeadlessAudio, LanguagePackageManager, OsPackageManager, TerminalApp, TtsClient,
    Utility,
};
pub use find_program::{
    find_program, find_program_with_source, find_programs_parallel,
    find_programs_with_source_parallel,
};
pub use headless_audio::InstalledHeadlessAudio;
pub use installer::{
    execute_install, execute_versioned_install, get_install_command,
    get_versioned_install_command, InstallOptions, InstallResult,
};
pub use inventory::{Program, PROGRAM_LOOKUP};
pub use macos_bundle::{find_macos_app_bundle, get_app_bundle_name};
pub use pkg_mngrs::{InstalledLanguagePackageManagers, InstalledOsPackageManagers};
pub use schema::{ProgramError, ProgramInfo, ProgramMetadata, VersionFlag, VersionParseStrategy};
pub use terminal_apps::InstalledTerminalApps;
pub use tts_clients::InstalledTtsClients;
pub use types::{ExecutableSource, InstallationMethod, ProgramDetails, ProgramDetector};
pub use utilities::InstalledUtilities;

/// Complete programs detection result.
///
/// Contains detection results for all supported program categories:
/// editors, utilities, package managers, TTS clients, terminal apps, headless audio players,
/// and AI CLI tools.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProgramsInfo {
    /// Text editors and IDEs installed on the system.
    pub editors: InstalledEditors,

    /// Modern command-line utilities installed on the system.
    pub utilities: InstalledUtilities,

    /// Language-specific package managers installed on the system.
    pub language_package_managers: InstalledLanguagePackageManagers,

    /// Operating system package managers installed on the system.
    pub os_package_managers: InstalledOsPackageManagers,

    /// Text-to-speech clients installed on the system.
    pub tts_clients: InstalledTtsClients,

    /// Terminal emulator applications installed on the system.
    pub terminal_apps: InstalledTerminalApps,

    /// Headless audio players installed on the system.
    pub headless_audio: InstalledHeadlessAudio,

    /// AI-powered CLI coding tools installed on the system.
    pub ai_clients: InstalledAiClients,
}

impl ProgramsInfo {
    /// Detect all installed programs across all categories.
    ///
    /// This runs detection in parallel for all program categories.
    pub fn detect() -> Self {
        Self {
            editors: InstalledEditors::new(),
            utilities: InstalledUtilities::new(),
            language_package_managers: InstalledLanguagePackageManagers::new(),
            os_package_managers: InstalledOsPackageManagers::new(),
            tts_clients: InstalledTtsClients::new(),
            terminal_apps: InstalledTerminalApps::new(),
            headless_audio: InstalledHeadlessAudio::new(),
            ai_clients: InstalledAiClients::new(),
        }
    }

    /// Re-check program availability for all categories.
    pub fn refresh(&mut self) {
        self.editors.refresh();
        self.utilities.refresh();
        self.language_package_managers.refresh();
        self.os_package_managers.refresh();
        self.tts_clients.refresh();
        self.terminal_apps.refresh();
        self.headless_audio.refresh();
        self.ai_clients.refresh();
    }
}
