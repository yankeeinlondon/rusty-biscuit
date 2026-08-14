//! Notification helper detection for desktop notification utilities.
//!
//! Detects six third-party notification helper CLIs across three OSes:
//! - macOS: `terminal-notifier`, `alerter`
//! - Windows: `snoretoast`, `burnttoast` (PowerShell module)
//! - Linux: `dunstify`, `notify-send`
//!
//! On Linux, also probes the active notification daemon via D-Bus.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::executable_index::ExecutableIndex;
use crate::programs::category_detector::CategoryDetector;
use crate::programs::contract::ExecutableSource;
use crate::programs::enums::NotificationHelper;

/// Information about the active Linux notification daemon.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotificationDaemon {
    /// Daemon name, e.g. "dunst", "GNOME Shell", "mako".
    pub name: String,
    /// Vendor string (may be empty).
    pub vendor: Option<String>,
    /// Daemon version string.
    pub version: Option<String>,
}

/// Detected notification helpers on the host system.
///
/// Wraps a `CategoryDetector<NotificationHelper>` and adds Linux-specific
/// active-daemon detection.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstalledNotificationHelpers {
    #[serde(flatten)]
    detector: CategoryDetector<NotificationHelper>,
    /// Active notification daemon on Linux (detected via D-Bus).
    pub active_daemon: Option<NotificationDaemon>,
}

impl InstalledNotificationHelpers {
    /// Detect installed notification helpers by scanning PATH.
    pub fn new() -> Self {
        let detector = CategoryDetector::new();
        let active_daemon = detect_active_daemon();
        Self {
            detector,
            active_daemon,
        }
    }

    /// Detect installed notification helpers using a pre-built executable index.
    pub fn new_with_index(index: &ExecutableIndex) -> Self {
        let detector = CategoryDetector::new_with_index(index);
        let active_daemon = detect_active_daemon();
        Self {
            detector,
            active_daemon,
        }
    }

    /// Returns true if the specified helper is installed.
    pub fn is_installed(&self, helper: NotificationHelper) -> bool {
        self.detector.is_installed(helper)
    }

    /// Returns the path to the specified helper's binary if installed.
    pub fn path(&self, helper: NotificationHelper) -> Option<PathBuf> {
        self.detector.path(helper)
    }

    /// Returns the path and source of the specified helper if installed.
    pub fn path_with_source(
        &self,
        helper: NotificationHelper,
    ) -> Option<(PathBuf, ExecutableSource)> {
        self.detector.path_with_source(helper)
    }

    /// Returns the version of the specified helper if available.
    ///
    /// ## Errors
    ///
    /// Returns an error if the helper is not installed or version detection fails.
    pub fn version(
        &self,
        helper: NotificationHelper,
    ) -> Result<String, crate::programs::ProgramError> {
        self.detector.version(helper)
    }

    /// Returns the official website URL for the specified helper.
    pub fn website(&self, helper: NotificationHelper) -> &'static str {
        self.detector.website(helper)
    }

    /// Returns a one-line description of the specified helper.
    pub fn description(&self, helper: NotificationHelper) -> &'static str {
        self.detector.description(helper)
    }

    /// Returns a list of all installed notification helpers.
    pub fn installed(&self) -> Vec<NotificationHelper> {
        self.detector.installed()
    }

    /// Builder method: mark a helper as installed with given path and source.
    ///
    /// Useful for testing.
    pub fn with_program(
        mut self,
        helper: NotificationHelper,
        path: PathBuf,
        source: ExecutableSource,
    ) -> Self {
        self.detector = self.detector.with_program(helper, path, source);
        self
    }
}

/// Detect the active Linux notification daemon via D-Bus.
#[cfg(target_os = "linux")]
fn detect_active_daemon() -> Option<NotificationDaemon> {
    let conn = zbus::blocking::Connection::session().ok()?;
    let reply = conn
        .call_method(
            Some("org.freedesktop.Notifications"),
            "/org/freedesktop/Notifications",
            Some("org.freedesktop.Notifications"),
            "GetServerInformation",
            &(),
        )
        .ok()?;
    let (name, vendor, version, _spec): (String, String, String, String) =
        reply.body().deserialize().ok()?;
    Some(NotificationDaemon {
        name,
        vendor: Some(vendor).filter(|s| !s.is_empty()),
        version: Some(version).filter(|s| !s.is_empty()),
    })
}

/// Non-Linux hosts have no D-Bus notification daemon.
#[cfg(not(target_os = "linux"))]
fn detect_active_daemon() -> Option<NotificationDaemon> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_has_all_none() {
        let helpers = InstalledNotificationHelpers::default();
        use strum::IntoEnumIterator;
        for helper in NotificationHelper::iter() {
            assert!(
                !helpers.is_installed(helper),
                "{:?} should not be installed by default",
                helper
            );
        }
    }

    #[test]
    #[cfg(not(target_os = "linux"))]
    fn test_default_active_daemon_is_none_on_non_linux() {
        let helpers = InstalledNotificationHelpers::default();
        assert!(helpers.active_daemon.is_none());
    }

    #[test]
    fn test_with_program_marks_installed() {
        let helpers = InstalledNotificationHelpers::default().with_program(
            NotificationHelper::Dunstify,
            PathBuf::from("/usr/bin/dunstify"),
            ExecutableSource::Path,
        );
        assert!(helpers.is_installed(NotificationHelper::Dunstify));
        assert!(!helpers.is_installed(NotificationHelper::NotifySend));
    }

    #[test]
    fn test_path_returns_none_for_default() {
        let helpers = InstalledNotificationHelpers::default();
        assert!(helpers.path(NotificationHelper::TerminalNotifier).is_none());
    }

    #[test]
    fn test_installed_returns_empty_for_default() {
        let helpers = InstalledNotificationHelpers::default();
        assert!(helpers.installed().is_empty());
    }

    #[test]
    fn test_serialize_produces_entries() {
        let helpers = InstalledNotificationHelpers::default();
        let json = serde_json::to_string(&helpers).unwrap();
        assert!(json.contains("\"installed\":false"));
        assert!(json.contains("\"terminal_notifier\":{"));
    }

    #[test]
    fn test_website_returns_static_str() {
        let helpers = InstalledNotificationHelpers::default();
        let website = helpers.website(NotificationHelper::TerminalNotifier);
        assert!(!website.is_empty());
        assert!(website.starts_with("http"));
    }

    #[test]
    fn test_description_returns_static_str() {
        let helpers = InstalledNotificationHelpers::default();
        let desc = helpers.description(NotificationHelper::Alerter);
        assert!(!desc.is_empty());
    }

    #[test]
    fn test_version_returns_not_found_for_uninstalled() {
        use crate::programs::ProgramError;
        let helpers = InstalledNotificationHelpers::default();
        let result = helpers.version(NotificationHelper::SnoreToast);
        assert!(result.is_err());
        if let Err(ProgramError::NotFound(name)) = result {
            assert_eq!(name, "snoretoast");
        } else {
            panic!("Expected NotFound error");
        }
    }

    #[test]
    fn test_clone_produces_equal_struct() {
        let helpers = InstalledNotificationHelpers::default();
        let cloned = helpers.clone();
        assert_eq!(helpers, cloned);
    }
}
