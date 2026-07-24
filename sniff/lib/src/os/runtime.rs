use serde::{Deserialize, Serialize};

/// Runtime environment layered over the host operating system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeEnvironment {
    /// A native operating-system runtime.
    Native,
    /// Windows Subsystem for Linux 1.
    Wsl1,
    /// Windows Subsystem for Linux 2.
    Wsl2,
}

impl RuntimeEnvironment {
    /// Whether the runtime is either generation of WSL.
    pub fn is_wsl(self) -> bool {
        matches!(self, Self::Wsl1 | Self::Wsl2)
    }
}

impl std::fmt::Display for RuntimeEnvironment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Native => write!(f, "Native"),
            Self::Wsl1 => write!(f, "WSL 1"),
            Self::Wsl2 => write!(f, "WSL 2"),
        }
    }
}

/// Detects whether this process runs natively, under WSL 1, or under WSL 2.
///
/// Linux kernel markers identify WSL. `microsoft-standard` is the WSL 2
/// kernel family; older Microsoft kernels without that marker are WSL 1.
pub fn detect_runtime_environment() -> RuntimeEnvironment {
    if !cfg!(target_os = "linux") {
        return RuntimeEnvironment::Native;
    }

    let version = std::fs::read_to_string("/proc/version").unwrap_or_default();
    let osrelease = std::fs::read_to_string("/proc/sys/kernel/osrelease").unwrap_or_default();
    runtime_environment_from_markers(&version, &osrelease)
}

pub(crate) fn runtime_environment_from_markers(
    version: &str,
    osrelease: &str,
) -> RuntimeEnvironment {
    let markers = format!("{version}\n{osrelease}").to_lowercase();

    if markers.contains("wsl2") || markers.contains("microsoft-standard") {
        RuntimeEnvironment::Wsl2
    } else if markers.contains("microsoft") || markers.contains("wsl") {
        RuntimeEnvironment::Wsl1
    } else {
        RuntimeEnvironment::Native
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_wsl2_markers() {
        assert_eq!(
            runtime_environment_from_markers(
                "Linux version 5.15.0-microsoft-standard-WSL2",
                ""
            ),
            RuntimeEnvironment::Wsl2
        );
        assert_eq!(
            runtime_environment_from_markers("", "4.19.128-microsoft-standard"),
            RuntimeEnvironment::Wsl2
        );
    }

    #[test]
    fn classifies_wsl1_markers() {
        assert_eq!(
            runtime_environment_from_markers("Linux version 4.4.0-Microsoft", ""),
            RuntimeEnvironment::Wsl1
        );
        assert_eq!(
            runtime_environment_from_markers("", "WSL"),
            RuntimeEnvironment::Wsl1
        );
    }

    #[test]
    fn classifies_native_linux_and_missing_proc() {
        assert_eq!(
            runtime_environment_from_markers(
                "Linux version 6.8.0-generic",
                "6.8.0-generic"
            ),
            RuntimeEnvironment::Native
        );
        assert_eq!(
            runtime_environment_from_markers("", ""),
            RuntimeEnvironment::Native
        );
    }

    #[test]
    fn serializes_as_stable_machine_value() {
        assert_eq!(
            serde_json::to_string(&RuntimeEnvironment::Wsl2).unwrap(),
            "\"wsl2\""
        );
    }
}
