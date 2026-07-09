//! OS-target resolution for config-file location.

use serde::Serialize;

use crate::discovery::os_detection::{OsType, detect_os_type, is_wsl1, is_wsl2};

/// The resolution bucket for config-file locations.
///
/// Distinct from [`OsType`], which carries BSDs, illumos, etc. but has no WSL
/// split. BSD / illumos / unknown map to [`ConfigOsTarget::Linux`] here because
/// they follow XDG-style config layout for resolution purposes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ConfigOsTarget {
    /// Native Linux (and, for resolution, BSD / illumos / unknown Unix-likes).
    Linux,
    /// macOS.
    MacOS,
    /// Windows.
    Windows,
    /// Windows Subsystem for Linux, version 1.
    Wsl1,
    /// Windows Subsystem for Linux, version 2.
    Wsl2,
}

/// Maps the running host to a single config-resolution bucket.
///
/// Windows → [`ConfigOsTarget::Windows`]; macOS → [`ConfigOsTarget::MacOS`];
/// Linux under WSL 1 → [`ConfigOsTarget::Wsl1`]; Linux under WSL 2 →
/// [`ConfigOsTarget::Wsl2`]; any other Linux / BSD / illumos / unknown →
/// [`ConfigOsTarget::Linux`].
pub fn current_config_os_target() -> ConfigOsTarget {
    match detect_os_type() {
        OsType::Windows => ConfigOsTarget::Windows,
        OsType::MacOS => ConfigOsTarget::MacOS,
        OsType::Linux => {
            if is_wsl1() {
                ConfigOsTarget::Wsl1
            } else if is_wsl2() {
                ConfigOsTarget::Wsl2
            } else {
                ConfigOsTarget::Linux
            }
        }
        // BSDs, illumos, Android, iOS, and unknown all resolve XDG-style.
        _ => ConfigOsTarget::Linux,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_current_config_os_target_matches_host() {
        let target = current_config_os_target();

        #[cfg(target_os = "macos")]
        assert_eq!(target, ConfigOsTarget::MacOS);

        #[cfg(target_os = "windows")]
        assert_eq!(target, ConfigOsTarget::Windows);

        #[cfg(target_os = "linux")]
        assert!(matches!(
            target,
            ConfigOsTarget::Linux | ConfigOsTarget::Wsl1 | ConfigOsTarget::Wsl2
        ));

        // Never panics; silence unused on exotic targets.
        let _ = target;
    }
}
