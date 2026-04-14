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
//! use sniff::programs::{ProgramsInfo, ProgramMetadata, Editor};
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
//! use sniff::programs::{find_program_with_source, ExecutableSource};
//!
//! // Find VS Code - checks PATH first, then macOS app bundles
//! if let Some((path, source)) = find_program_with_source("code") {
//!     match source {
//!         ExecutableSource::Path => println!("Found in PATH: {}", path.display()),
//!         ExecutableSource::MacOsAppBundle => println!("Found as macOS app: {}", path.display()),
//!         ExecutableSource::WindowsAppPaths => println!("Found via App Paths: {}", path.display()),
//!         ExecutableSource::WindowsInstallRoot => println!("Found under install root: {}", path.display()),
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
pub mod host_capability;
pub mod install_interview;
pub mod install_plan;
pub mod installer;
pub mod inventory;
pub mod macos_bundle;
pub mod pkg_mngrs;
pub mod schema;
pub mod terminal_apps;
pub mod tts_clients;
pub mod types;
pub mod utilities;
#[cfg(target_os = "windows")]
pub(crate) mod windows_apps;

use serde::{Deserialize, Serialize};
use tracing::{info_span, instrument};

pub use ai_cli::InstalledAiClients;
pub use editors::InstalledEditors;
pub use enums::{
    AiCli, CategoryEnum, Editor, HeadlessAudio, LanguagePackageManager, OsPackageManager,
    TerminalApp, TtsClient, Utility,
};
pub use find_program::{
    ExecutableIndex, find_program, find_program_with_source, find_programs_parallel,
    find_programs_with_source_from_index, find_programs_with_source_parallel,
};
pub use headless_audio::InstalledHeadlessAudio;
pub use host_capability::{
    CACHE_SCHEMA_VERSION, HostCapabilities, HostCapabilityCacheFile, default_cache_path,
    load_host_capabilities_from, save_host_capabilities_to,
};
pub use install_interview::{
    InstallInterviewDelegate, InstallInterviewEvent, InstallInterviewInput,
    InstallInterviewOptions, InstallInterviewOutcome, InstallOutputStream, InstallStatusKind,
    RetryChoice, RetryPrompt, RetryPromptChoice, run_install_interview,
};
pub use install_plan::{InstallPlan, InstallPlanOption, InstallPlanReason, build_install_plan};
pub use installer::{
    InstallOptions, InstallResult, execute_install, execute_versioned_install, get_install_command,
    get_versioned_install_command,
};
pub use inventory::Program;
pub use macos_bundle::{find_macos_app_bundle, get_app_bundle_name};
pub use pkg_mngrs::{InstalledLanguagePackageManagers, InstalledOsPackageManagers};
pub use schema::{ProgramError, ProgramInfo, ProgramMetadata, VersionFlag, VersionParseStrategy};
pub use terminal_apps::InstalledTerminalApps;
pub use tts_clients::InstalledTtsClients;
pub use types::{CategoryDetector, ExecutableSource, InstallationMethod, ProgramDetector};
pub use utilities::InstalledUtilities;

/// Complete programs detection result.
///
/// Contains detection results for all supported program categories:
/// editors, utilities, package managers, TTS clients, terminal apps, headless audio players,
/// and AI CLI tools.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
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
    /// Builds a shared executable index once (scanning all PATH dirs and macOS app bundles),
    /// then detects all 8 categories in parallel using Rayon's `join` API. Each category
    /// uses the shared index for O(1) lookups instead of repeated filesystem traversal.
    ///
    /// ## Performance
    ///
    /// The shared index eliminates redundant filesystem scans:
    /// - PATH scan: once (instead of 8x per category)
    /// - macOS bundle check: once (instead of 8x per category)
    /// - Subsequent lookups: O(1) HashMap access
    #[instrument(skip_all)]
    pub fn detect() -> Self {
        use std::sync::Arc;

        // Build the shared executable index once
        let index = {
            let _span = info_span!("build_executable_index").entered();
            Arc::new(ExecutableIndex::build())
        };

        // Parallelize category detection in pairs using rayon::join
        let (editors, utilities) = rayon::join(
            || InstalledEditors::new_with_index(&index),
            || InstalledUtilities::new_with_index(&index),
        );

        let (language_package_managers, os_package_managers) = rayon::join(
            || InstalledLanguagePackageManagers::new_with_index(&index),
            || InstalledOsPackageManagers::new_with_index(&index),
        );

        let (tts_clients, terminal_apps) = rayon::join(
            || InstalledTtsClients::new_with_index(&index),
            || InstalledTerminalApps::new_with_index(&index),
        );

        let (headless_audio, ai_clients) = rayon::join(
            || InstalledHeadlessAudio::new_with_index(&index),
            || InstalledAiClients::new_with_index(&index),
        );

        Self {
            editors,
            utilities,
            language_package_managers,
            os_package_managers,
            tts_clients,
            terminal_apps,
            headless_audio,
            ai_clients,
        }
    }

    /// Re-check program availability for all categories.
    ///
    /// Delegates to [`Self::detect()`] so the shared executable index is
    /// built once and reused across all categories, matching the optimized
    /// initial-detection path.
    pub fn refresh(&mut self) {
        *self = Self::detect();
    }
}
