//! Stub implementations for package manager operations.
//!
//! This module provides stub implementations of [`PackageManagerShape`] for core
//! package managers. These stubs return `Ok(None)` for all operations and serve
//! as placeholders for future implementation.
//!
//! ## Implemented Stubs
//!
//! - **OS Managers**: Apt, Homebrew
//! - **Language Managers**: Npm, Pnpm, Yarn, Bun, Cargo, Pip

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use super::{BoxFuture, LanguagePackageManager, OsPackageManager, PackageManagerShape};
use crate::os::{command_exists_in_path, get_path_dirs};
use crate::Result;

// ============================================================================
// PackageInfo Struct
// ============================================================================

/// Information about a package from a package registry.
///
/// Contains basic metadata retrieved from a package manager query.
///
/// ## Examples
///
/// ```
/// use sniff_lib::package::PackageInfo;
///
/// let info = PackageInfo {
///     name: "lodash".to_string(),
///     version: "4.17.21".to_string(),
///     description: Some("Lodash modular utilities".to_string()),
/// };
///
/// println!("{} v{}", info.name, info.version);
/// if let Some(desc) = &info.description {
///     println!("  {}", desc);
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackageInfo {
    /// Package name
    pub name: String,
    /// Package version
    pub version: String,
    /// Package description (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Checks if an executable exists in the system PATH.
fn executable_exists(name: &str) -> bool {
    let path_dirs = get_path_dirs();
    command_exists_in_path(name, &path_dirs).is_some()
}

/// Gets the full path to an executable if it exists.
#[allow(dead_code)]
fn executable_path(name: &str) -> Option<PathBuf> {
    let path_dirs = get_path_dirs();
    command_exists_in_path(name, &path_dirs)
}

// ============================================================================
// OS Package Manager Stubs
// ============================================================================

/// Stub implementation for APT package manager.
///
/// APT (Advanced Package Tool) is the primary package manager for
/// Debian-based Linux distributions like Ubuntu and Debian.
#[derive(Debug, Clone, Copy, Default)]
pub struct AptStub;

impl PackageManagerShape for AptStub {
    fn executable_name(&self) -> &'static str {
        OsPackageManager::Apt.executable_name()
    }

    fn is_available(&self) -> bool {
        executable_exists(self.executable_name())
    }

    fn find_package(&self, _name: &str) -> BoxFuture<'_, Result<Option<PackageInfo>>> {
        // Stub: return None for now
        // Future implementation would use `apt search` or `apt-cache show`
        Box::pin(async { Ok(None) })
    }

    fn latest_version(&self, _name: &str) -> BoxFuture<'_, Result<Option<String>>> {
        // Stub: return None for now
        // Future implementation would use `apt-cache policy`
        Box::pin(async { Ok(None) })
    }
}

/// Stub implementation for Homebrew package manager.
///
/// Homebrew is the most popular package manager for macOS, also
/// available on Linux.
#[derive(Debug, Clone, Copy, Default)]
pub struct HomebrewStub;

impl PackageManagerShape for HomebrewStub {
    fn executable_name(&self) -> &'static str {
        OsPackageManager::Homebrew.executable_name()
    }

    fn is_available(&self) -> bool {
        // Check known Homebrew locations first, then PATH
        let apple_silicon = std::path::Path::new("/opt/homebrew/bin/brew").is_file();
        let intel = std::path::Path::new("/usr/local/bin/brew").is_file();
        apple_silicon || intel || executable_exists(self.executable_name())
    }

    fn find_package(&self, _name: &str) -> BoxFuture<'_, Result<Option<PackageInfo>>> {
        // Stub: return None for now
        // Future implementation would use `brew info --json`
        Box::pin(async { Ok(None) })
    }

    fn latest_version(&self, _name: &str) -> BoxFuture<'_, Result<Option<String>>> {
        // Stub: return None for now
        // Future implementation would use `brew info --json`
        Box::pin(async { Ok(None) })
    }
}

// ============================================================================
// Language Package Manager Stubs
// ============================================================================

/// Stub implementation for npm package manager.
///
/// npm is the default package manager for Node.js and the largest
/// software registry in the world.
///
/// Note: Prefer `NpmNetwork` when the `network` feature is enabled.
#[derive(Debug, Clone, Copy, Default)]
#[allow(dead_code)]
pub struct NpmStub;

impl PackageManagerShape for NpmStub {
    fn executable_name(&self) -> &'static str {
        LanguagePackageManager::Npm.executable_name()
    }

    fn is_available(&self) -> bool {
        executable_exists(self.executable_name())
    }

    fn find_package(&self, _name: &str) -> BoxFuture<'_, Result<Option<PackageInfo>>> {
        // Stub: return None for now
        // Future implementation would use `npm view <name> --json`
        Box::pin(async { Ok(None) })
    }

    fn latest_version(&self, _name: &str) -> BoxFuture<'_, Result<Option<String>>> {
        // Stub: return None for now
        // Future implementation would use `npm view <name> version`
        Box::pin(async { Ok(None) })
    }
}

/// Stub implementation for pnpm package manager.
///
/// pnpm is a fast, disk space efficient package manager that uses
/// a content-addressable filesystem to store all files from all
/// module directories on a disk.
///
/// Note: Prefer `PnpmNetwork` when the `network` feature is enabled.
#[derive(Debug, Clone, Copy, Default)]
#[allow(dead_code)]
pub struct PnpmStub;

impl PackageManagerShape for PnpmStub {
    fn executable_name(&self) -> &'static str {
        LanguagePackageManager::Pnpm.executable_name()
    }

    fn is_available(&self) -> bool {
        executable_exists(self.executable_name())
    }

    fn find_package(&self, _name: &str) -> BoxFuture<'_, Result<Option<PackageInfo>>> {
        // Stub: return None for now
        // Future implementation would use npm registry API (same as npm)
        Box::pin(async { Ok(None) })
    }

    fn latest_version(&self, _name: &str) -> BoxFuture<'_, Result<Option<String>>> {
        // Stub: return None for now
        Box::pin(async { Ok(None) })
    }
}

/// Stub implementation for Yarn package manager.
///
/// Yarn is a package manager that focuses on speed, security, and
/// determinism. This stub covers both Yarn Classic (v1) and Yarn Berry (v2+).
///
/// Note: Prefer `YarnNetwork` when the `network` feature is enabled.
#[derive(Debug, Clone, Copy, Default)]
#[allow(dead_code)]
pub struct YarnStub;

impl PackageManagerShape for YarnStub {
    fn executable_name(&self) -> &'static str {
        LanguagePackageManager::Yarn.executable_name()
    }

    fn is_available(&self) -> bool {
        executable_exists(self.executable_name())
    }

    fn find_package(&self, _name: &str) -> BoxFuture<'_, Result<Option<PackageInfo>>> {
        // Stub: return None for now
        // Future implementation would use `yarn info <name> --json`
        Box::pin(async { Ok(None) })
    }

    fn latest_version(&self, _name: &str) -> BoxFuture<'_, Result<Option<String>>> {
        // Stub: return None for now
        Box::pin(async { Ok(None) })
    }
}

/// Stub implementation for Bun package manager.
///
/// Bun is an all-in-one JavaScript runtime and toolkit that includes
/// a fast npm-compatible package manager.
///
/// Note: Prefer `BunNetwork` when the `network` feature is enabled.
#[derive(Debug, Clone, Copy, Default)]
#[allow(dead_code)]
pub struct BunStub;

impl PackageManagerShape for BunStub {
    fn executable_name(&self) -> &'static str {
        LanguagePackageManager::Bun.executable_name()
    }

    fn is_available(&self) -> bool {
        executable_exists(self.executable_name())
    }

    fn find_package(&self, _name: &str) -> BoxFuture<'_, Result<Option<PackageInfo>>> {
        // Stub: return None for now
        // Future implementation would use npm registry API
        Box::pin(async { Ok(None) })
    }

    fn latest_version(&self, _name: &str) -> BoxFuture<'_, Result<Option<String>>> {
        // Stub: return None for now
        Box::pin(async { Ok(None) })
    }
}

/// Stub implementation for Cargo package manager.
///
/// Cargo is Rust's official package manager and build system,
/// using crates.io as its default registry.
///
/// Note: Prefer `CargoNetwork` when the `network` feature is enabled.
#[derive(Debug, Clone, Copy, Default)]
#[allow(dead_code)]
pub struct CargoStub;

impl PackageManagerShape for CargoStub {
    fn executable_name(&self) -> &'static str {
        LanguagePackageManager::Cargo.executable_name()
    }

    fn is_available(&self) -> bool {
        executable_exists(self.executable_name())
    }

    fn find_package(&self, _name: &str) -> BoxFuture<'_, Result<Option<PackageInfo>>> {
        // Stub: return None for now
        // Future implementation would use crates.io API
        Box::pin(async { Ok(None) })
    }

    fn latest_version(&self, _name: &str) -> BoxFuture<'_, Result<Option<String>>> {
        // Stub: return None for now
        // Future implementation would use crates.io API
        Box::pin(async { Ok(None) })
    }
}

/// Stub implementation for pip package manager.
///
/// pip is Python's standard package installer, using PyPI
/// (Python Package Index) as its default repository.
#[derive(Debug, Clone, Copy, Default)]
pub struct PipStub;

impl PackageManagerShape for PipStub {
    fn executable_name(&self) -> &'static str {
        LanguagePackageManager::Pip.executable_name()
    }

    fn is_available(&self) -> bool {
        // Check for pip, pip3, and python -m pip
        executable_exists("pip") || executable_exists("pip3")
    }

    fn find_package(&self, _name: &str) -> BoxFuture<'_, Result<Option<PackageInfo>>> {
        // Stub: return None for now
        // Future implementation would use PyPI JSON API
        Box::pin(async { Ok(None) })
    }

    fn latest_version(&self, _name: &str) -> BoxFuture<'_, Result<Option<String>>> {
        // Stub: return None for now
        // Future implementation would use PyPI JSON API
        Box::pin(async { Ok(None) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_package_info_creation() {
        let info = PackageInfo {
            name: "test-package".to_string(),
            version: "1.0.0".to_string(),
            description: Some("A test package".to_string()),
        };

        assert_eq!(info.name, "test-package");
        assert_eq!(info.version, "1.0.0");
        assert_eq!(info.description, Some("A test package".to_string()));
    }

    #[test]
    fn test_package_info_serde() {
        let info = PackageInfo {
            name: "serde".to_string(),
            version: "1.0.203".to_string(),
            description: Some("A generic serialization/deserialization framework".to_string()),
        };

        let json = serde_json::to_string(&info).unwrap();
        let restored: PackageInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(info, restored);
    }

    #[test]
    fn test_package_info_serde_no_description() {
        let info = PackageInfo {
            name: "minimal".to_string(),
            version: "0.1.0".to_string(),
            description: None,
        };

        let json = serde_json::to_string(&info).unwrap();
        // description should be omitted when None
        assert!(!json.contains("description"));

        let restored: PackageInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(info, restored);
    }

    #[test]
    fn test_stub_executable_names() {
        assert_eq!(AptStub.executable_name(), "apt");
        assert_eq!(HomebrewStub.executable_name(), "brew");
        assert_eq!(NpmStub.executable_name(), "npm");
        assert_eq!(PnpmStub.executable_name(), "pnpm");
        assert_eq!(YarnStub.executable_name(), "yarn");
        assert_eq!(BunStub.executable_name(), "bun");
        assert_eq!(CargoStub.executable_name(), "cargo");
        assert_eq!(PipStub.executable_name(), "pip");
    }

    // Note: We can't easily test is_available() without mocking the filesystem,
    // but we can verify the methods don't panic
    #[test]
    fn test_stub_is_available_no_panic() {
        // These should not panic regardless of whether the tools are installed
        let _ = AptStub.is_available();
        let _ = HomebrewStub.is_available();
        let _ = NpmStub.is_available();
        let _ = PnpmStub.is_available();
        let _ = YarnStub.is_available();
        let _ = BunStub.is_available();
        let _ = CargoStub.is_available();
        let _ = PipStub.is_available();
    }

    #[tokio::test]
    async fn test_npm_stub_find_package_returns_none() {
        let stub = NpmStub;
        let result = stub.find_package("lodash").await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_cargo_stub_find_package_returns_none() {
        let stub = CargoStub;
        let result = stub.find_package("serde").await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_pip_stub_latest_version_returns_none() {
        let stub = PipStub;
        let result = stub.latest_version("requests").await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }
}
