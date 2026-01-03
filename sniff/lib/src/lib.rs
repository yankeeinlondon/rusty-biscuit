use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub mod error;
pub mod filesystem;
pub mod hardware;
pub mod network;

pub use error::{Result, SniffError};
pub use hardware::HardwareInfo;
pub use network::NetworkInfo;
pub use filesystem::FilesystemInfo;

/// Complete system detection result.
///
/// Contains hardware, network, and filesystem information gathered
/// by the sniff library.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SniffResult {
    pub hardware: HardwareInfo,
    pub network: NetworkInfo,
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
/// println!("OS: {}", result.hardware.os.name);
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
    let hardware = if config.skip_hardware {
        HardwareInfo::default()
    } else if config.include_cpu_usage {
        hardware::detect_hardware_with_usage()?
    } else {
        hardware::detect_hardware()?
    };

    let network = if config.skip_network {
        NetworkInfo::default()
    } else {
        network::detect_network()?
    };

    let filesystem = if config.skip_filesystem {
        None
    } else {
        let base = config.base_dir.unwrap_or_else(|| {
            std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
        });
        Some(filesystem::detect_filesystem(&base)?)
    };

    Ok(SniffResult { hardware, network, filesystem })
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
    fn test_skip_hardware_returns_default() {
        let config = SniffConfig::new().skip_hardware();
        let result = detect_with_config(config).unwrap();
        assert!(result.hardware.os.name.is_empty());
    }

    #[test]
    fn test_skip_network_returns_default() {
        let config = SniffConfig::new().skip_network();
        let result = detect_with_config(config).unwrap();
        assert!(result.network.interfaces.is_empty());
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
        let config = SniffConfig::new()
            .base_dir(PathBuf::from("."));
        let result = detect_with_config(config).unwrap();
        assert!(result.filesystem.is_some());
    }
}
