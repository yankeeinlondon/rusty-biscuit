//! Program detection module for identifying installed software.
//!
//! This module provides comprehensive detection of installed programs across
//! multiple categories including editors, utilities, package managers, TTS clients,
//! and terminal emulators.
//!
//! ## Categories
//!
//! - **Editors**: Text editors and IDEs (Vim, VS Code, IntelliJ, etc.)
//! - **Utilities**: Modern CLI tools (ripgrep, bat, fzf, etc.)
//! - **Language Package Managers**: npm, cargo, pip, etc.
//! - **OS Package Managers**: brew, apt, dnf, etc.
//! - **TTS Clients**: Text-to-speech tools (say, espeak, piper, etc.)
//! - **Terminal Apps**: Terminal emulators (alacritty, kitty, wezterm, etc.)
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

pub mod types;
pub mod inventory;
pub mod editors;
pub mod enums;
pub mod find_program;
pub mod headless_audio;
pub mod pkg_mngrs;
pub mod schema;
pub mod terminal_apps;
pub mod tts_clients;
pub mod utilities;

use serde::{Deserialize, Serialize};

pub use editors::InstalledEditors;
pub use enums::{
    Editor, HeadlessAudio, LanguagePackageManager, OsPackageManager, TerminalApp, TtsClient,
    Utility,
};
pub use headless_audio::InstalledHeadlessAudio;
pub use pkg_mngrs::{InstalledLanguagePackageManagers, InstalledOsPackageManagers};
pub use schema::{ProgramError, ProgramInfo, ProgramMetadata, VersionFlag, VersionParseStrategy};
pub use terminal_apps::InstalledTerminalApps;
pub use tts_clients::InstalledTtsClients;
pub use utilities::InstalledUtilities;

/// Complete programs detection result.
///
/// Contains detection results for all supported program categories:
/// editors, utilities, package managers, TTS clients, terminal apps, and headless audio players.
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
    }
}
