use serde::{Deserialize, Serialize};

/// Operating system information.
///
/// Contains details about the operating system, kernel version,
/// architecture, and hostname.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OsInfo {
    /// Operating system name (e.g., "macOS", "Linux", "Windows")
    pub name: String,
    /// Operating system version (short form)
    pub version: String,
    /// Long OS version with additional details (e.g., "macOS 14.5 Sonoma")
    pub long_version: Option<String>,
    /// Linux distribution ID (e.g., "ubuntu", "fedora", "arch")
    /// None on non-Linux systems
    pub distribution: Option<String>,
    /// Kernel version
    pub kernel: String,
    /// System architecture (e.g., "x86_64", "aarch64")
    pub arch: String,
    /// System hostname
    pub hostname: String,
}
