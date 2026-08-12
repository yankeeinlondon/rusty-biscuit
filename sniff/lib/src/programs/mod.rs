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
//! - **Notification Helpers**: Desktop notification utilities (terminal-notifier, alerter, snoretoast, burnttoast, dunstify, notify-send)
//! - **Test Runners**: Project test runners (cargo test, vitest, pytest, go test, etc.)
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
//!         ExecutableSource::ProjectLocal => println!("Found in project-local bin: {}", path.display()),
//!     }
//! }
//! ```
//!
//! ### Platform Behavior
//!
//! - **macOS**: Full bundle detection support
//! - **Linux/Windows**: Bundle detection returns `None` (PATH-only)

pub mod categories;
pub mod category_detector;
pub mod contract;
pub mod enums;
pub mod find_program;
pub mod host_capability;
pub mod install;
pub mod inventory;
pub mod local_bin;
pub mod macos_bundle;
pub mod notification_helpers;
pub mod schema;
pub mod test_runner;
pub mod test_runner_spec;
pub mod types;
#[cfg(target_os = "windows")]
pub(crate) mod windows_apps;

use serde::{Deserialize, Serialize};
use tracing::{info_span, instrument};

pub use crate::executable_index::{ExecutableIndex, find_programs_with_source_from_index};
pub use categories::{
    InstalledAiClients, InstalledEditors, InstalledHeadlessAudio, InstalledLanguagePackageManagers,
    InstalledOsPackageManagers, InstalledTerminalApps, InstalledTtsClients, InstalledUtilities,
};
pub use category_detector::CategoryDetector;
pub use contract::{
    ExecutableSource, InstallationMethod, PrereqProbe, ProgramError, SystemPrerequisite,
};
pub use enums::{
    AiCli, CategoryEnum, Editor, HeadlessAudio, LanguagePackageManager, NotificationHelper,
    OsPackageManager, TerminalApp, TestRunner, TtsClient, Utility,
};
pub use find_program::{
    find_program, find_program_with_source, find_programs_parallel,
    find_programs_with_source_parallel,
};
pub use host_capability::{
    CACHE_SCHEMA_VERSION, HostCapabilities, HostCapabilityCacheFile, default_cache_path,
    load_host_capabilities_from, save_host_capabilities_to,
};
pub use install::{
    InstallCapturedOutcome, InstallCapturedResult, InstallInterviewDelegate, InstallInterviewEvent,
    InstallInterviewInput, InstallInterviewOptions, InstallInterviewOutcome, InstallOptions,
    InstallOutputStream, InstallPlan, InstallPlanOption, InstallPlanReason, InstallResult,
    InstallStatusKind, RetryChoice, RetryPrompt, RetryPromptChoice, build_install_announcement,
    build_install_failure_status, build_install_plan, build_install_success_status,
    build_install_timeout_warning, build_retry_choice_prose, build_retry_quit_prose,
    execute_install, execute_versioned_install,
    get_install_command, get_versioned_install_command, run_install_interview,
};
pub use inventory::Program;
pub use local_bin::LocalBinIndex;
pub use macos_bundle::{find_macos_app_bundle, get_app_bundle_name};
pub use notification_helpers::InstalledNotificationHelpers;
pub use schema::{ProgramInfo, ProgramMetadata, VersionFlag, VersionParseStrategy};
pub use test_runner::{
    InstalledTestRunners, TestRunnerEntry, detect_test_runners, resolve_test_runner,
};
pub use test_runner_spec::{
    Availability, InvocationClass, RunnerKind, TestRunnerEcosystem, TestRunnerSpec,
};
pub use types::ProgramDetector;

/// Complete programs detection result.
///
/// Contains detection results for all supported program categories:
/// editors, utilities, package managers, TTS clients, terminal apps, headless audio players,
/// AI CLI tools, notification helpers, and test runners.
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

    /// Desktop notification helper utilities installed on the system.
    pub notification_helpers: InstalledNotificationHelpers,

    /// Test runners resolved against the host (PATH, project-local bins, and
    /// parent binaries). Unlike the other categories, this field uses
    /// [`Availability`] discriminators rather than a bare `installed: bool`
    /// because test runners live in many places (see `test-runner-strategy.md`).
    pub test_runners: InstalledTestRunners,
}

impl ProgramsInfo {
    /// Detect all installed programs across all categories.
    ///
    /// Builds a shared executable index once (scanning all PATH dirs and macOS app bundles),
    /// then detects all 10 categories in parallel using Rayon's `join` API. Each category
    /// uses the shared index for O(1) lookups instead of repeated filesystem traversal.
    /// The test-runner category additionally probes project-local bin directories via
    /// [`LocalBinIndex`] (cwd-sensitive) before falling back to PATH and parent-binary
    /// resolution.
    ///
    /// ## Performance
    ///
    /// The shared index eliminates redundant filesystem scans:
    /// - PATH scan: once (instead of 10x per category)
    /// - macOS bundle check: once (instead of 10x per category)
    /// - Subsequent lookups: O(1) HashMap access
    #[instrument(skip_all)]
    pub fn detect() -> Self {
        use std::sync::Arc;

        // Build the shared executable index once. Bulk detection benefits from
        // an eager PATH scan so per-program lookups become O(1) HashMap probes
        // instead of repeated PATH traversals via `which`.
        let index = {
            let _span = info_span!("build_executable_index").entered();
            Arc::new(ExecutableIndex::build_eager_path())
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

        let (notification_helpers, test_runners) = rayon::join(
            || InstalledNotificationHelpers::new_with_index(&index),
            || detect_test_runners(Arc::clone(&index)),
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
            notification_helpers,
            test_runners,
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
