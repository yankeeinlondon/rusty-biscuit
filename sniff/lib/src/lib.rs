use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub mod error;
pub mod filesystem;
pub mod hardware;
pub mod network;
pub mod os;
pub mod package;
pub mod programs;

pub use error::{Result, SniffError};
pub use filesystem::FilesystemInfo;
pub use hardware::HardwareInfo;
pub use network::NetworkInfo;
pub use programs::{ProgramMetadata, ProgramsInfo};

// Re-export key OS types from the os module for convenience.
// The canonical path is `sniff_lib::os::*`.
pub use os::OsInfo;

/// Complete system detection result.
///
/// Contains OS, hardware, network, and filesystem information gathered
/// by the sniff library. All fields are optional to allow partial
/// detection when using flags.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SniffResult {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub os: Option<OsInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hardware: Option<HardwareInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network: Option<NetworkInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filesystem: Option<FilesystemInfo>,
}

/// Configuration for the detect operation.
///
/// Use the builder pattern to customize detection behavior.
///
/// ## Examples
///
/// ```
/// use sniff_lib::SniffConfig;
/// use std::path::PathBuf;
///
/// let config = SniffConfig::new()
///     .base_dir(PathBuf::from("/some/path"))
///     .skip_network();
/// ```
#[derive(Debug, Clone, Default)]
pub struct SniffConfig {
    /// Base directory for filesystem analysis
    pub base_dir: Option<PathBuf>,
    /// Include CPU usage sampling (takes ~200ms)
    pub include_cpu_usage: bool,
    /// Enable deep git inspection (network operations for remote info)
    pub deep: bool,
    /// Skip OS detection
    pub skip_os: bool,
    /// Skip hardware detection
    pub skip_hardware: bool,
    /// Skip network detection
    pub skip_network: bool,
    /// Skip filesystem detection
    pub skip_filesystem: bool,
}

impl SniffConfig {
    /// Create a new configuration with defaults.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the base directory for filesystem analysis.
    pub fn base_dir(mut self, path: PathBuf) -> Self {
        self.base_dir = Some(path);
        self
    }

    /// Enable CPU usage sampling.
    pub fn include_cpu_usage(mut self, include: bool) -> Self {
        self.include_cpu_usage = include;
        self
    }

    /// Enable deep git inspection (fetches remote branch info, checks if behind).
    pub fn deep(mut self, enable: bool) -> Self {
        self.deep = enable;
        self
    }

    /// Skip OS detection.
    pub fn skip_os(mut self) -> Self {
        self.skip_os = true;
        self
    }

    /// Skip hardware detection.
    pub fn skip_hardware(mut self) -> Self {
        self.skip_hardware = true;
        self
    }

    /// Skip network detection.
    pub fn skip_network(mut self) -> Self {
        self.skip_network = true;
        self
    }

    /// Skip filesystem detection.
    pub fn skip_filesystem(mut self) -> Self {
        self.skip_filesystem = true;
        self
    }
}

/// Detect system information with default configuration.
///
/// This is a convenience function that calls `detect_with_config`
/// with default settings.
///
/// ## Examples
///
/// ```no_run
/// use sniff_lib::detect;
///
/// let result = detect().unwrap();
/// if let Some(os) = result.os {
///     println!("OS: {}", os.name);
/// }
/// ```
pub fn detect() -> Result<SniffResult> {
    detect_with_config(SniffConfig::default())
}

/// Detect system information with custom configuration.
///
/// ## Examples
///
/// ```no_run
/// use sniff_lib::{detect_with_config, SniffConfig};
/// use std::path::PathBuf;
///
/// let config = SniffConfig::new()
///     .base_dir(PathBuf::from("."))
///     .skip_network();
///
/// let result = detect_with_config(config).unwrap();
/// ```
pub fn detect_with_config(config: SniffConfig) -> Result<SniffResult> {
    let os = if config.skip_os {
        None
    } else {
        Some(os::detect_os()?)
    };

    let hardware = if config.skip_hardware {
        None
    } else if config.include_cpu_usage {
        Some(hardware::detect_hardware_with_usage()?)
    } else {
        Some(hardware::detect_hardware()?)
    };

    let network = if config.skip_network {
        None
    } else {
        Some(network::detect_network()?)
    };

    let filesystem = if config.skip_filesystem {
        None
    } else {
        let base = config
            .base_dir
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
        Some(filesystem::detect_filesystem(&base, config.deep)?)
    };

    Ok(SniffResult {
        os,
        hardware,
        network,
        filesystem,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_returns_result() {
        let result = detect();
        assert!(result.is_ok());
    }

    #[test]
    fn test_skip_hardware_returns_none() {
        let config = SniffConfig::new().skip_hardware();
        let result = detect_with_config(config).unwrap();
        assert!(result.hardware.is_none());
    }

    #[test]
    fn test_skip_network_returns_none() {
        let config = SniffConfig::new().skip_network();
        let result = detect_with_config(config).unwrap();
        assert!(result.network.is_none());
    }

    #[test]
    fn test_skip_filesystem_returns_none() {
        let config = SniffConfig::new().skip_filesystem();
        let result = detect_with_config(config).unwrap();
        assert!(result.filesystem.is_none());
    }

    #[test]
    fn test_config_builder_pattern() {
        let config = SniffConfig::new()
            .base_dir(PathBuf::from("."))
            .include_cpu_usage(true)
            .skip_network();

        assert!(config.base_dir.is_some());
        assert!(config.include_cpu_usage);
        assert!(config.skip_network);
    }

    #[test]
    fn test_detect_with_base_dir() {
        let config = SniffConfig::new().base_dir(PathBuf::from("."));
        let result = detect_with_config(config).unwrap();
        assert!(result.filesystem.is_some());
    }

    // Regression test: OS should be skipped when skip_os is set
    // Bug: When using --filesystem flag, OS section was still displayed
    #[test]
    fn test_skip_os_returns_none() {
        let config = SniffConfig::new().skip_os();
        let result = detect_with_config(config).unwrap();
        assert!(result.os.is_none(), "OS should be None when skip_os is set");
    }

    // Regression test: OS should be present by default
    #[test]
    fn test_os_present_by_default() {
        let config = SniffConfig::new();
        let result = detect_with_config(config).unwrap();
        assert!(result.os.is_some(), "OS should be Some by default");
    }

    // Regression test: Combining skip_os with other sections should work correctly
    #[test]
    fn test_skip_os_with_filesystem_only() {
        let config = SniffConfig::new().skip_os().skip_hardware().skip_network();
        let result = detect_with_config(config).unwrap();
        assert!(result.os.is_none(), "OS should be None when skipped");
        assert!(
            result.hardware.is_none(),
            "Hardware should be None when skipped"
        );
        assert!(
            result.network.is_none(),
            "Network should be None when skipped"
        );
        assert!(
            result.filesystem.is_some(),
            "Filesystem should be Some when not skipped"
        );
    }

    // Regression test: Multiple skip flags including OS
    #[test]
    fn test_multiple_skip_flags_including_os() {
        let config = SniffConfig::new()
            .skip_os()
            .skip_hardware()
            .skip_network()
            .skip_filesystem();
        let result = detect_with_config(config).unwrap();
        assert!(result.os.is_none());
        assert!(result.hardware.is_none());
        assert!(result.network.is_none());
        assert!(result.filesystem.is_none());
    }
}
