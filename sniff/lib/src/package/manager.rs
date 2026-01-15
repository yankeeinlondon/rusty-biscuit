//! Unified package manager types and traits.
//!
//! This module provides the [`PackageManager`] enum that wraps both OS and language
//! package managers, along with the [`PackageManagerShape`] trait for defining
//! package manager operations.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::future::Future;

use super::language::LanguagePackageManager;
use super::os::OsPackageManager;
use super::stubs::PackageInfo;
use crate::Result;

/// Unified package manager type encompassing both OS and language package managers.
///
/// This wrapper enum provides a single type for working with any kind of
/// package manager, whether it's an operating system-level manager like apt
/// or homebrew, or a language ecosystem manager like npm or cargo.
///
/// ## Examples
///
/// ```
/// use sniff_lib::package::{PackageManager, OsPackageManager, LanguagePackageManager};
///
/// let managers: Vec<PackageManager> = vec![
///     PackageManager::Os(OsPackageManager::Apt),
///     PackageManager::Language(LanguagePackageManager::Npm),
///     PackageManager::Os(OsPackageManager::Homebrew),
///     PackageManager::Language(LanguagePackageManager::Cargo),
/// ];
///
/// for mgr in &managers {
///     println!("{} (executable: {})", mgr, mgr.executable_name());
/// }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum PackageManager {
    /// Operating system-level package manager
    Os(OsPackageManager),
    /// Language ecosystem package manager
    Language(LanguagePackageManager),
}

impl PackageManager {
    /// Returns the command-line executable name for this package manager.
    ///
    /// ## Examples
    ///
    /// ```
    /// use sniff_lib::package::{PackageManager, OsPackageManager, LanguagePackageManager};
    ///
    /// let apt = PackageManager::Os(OsPackageManager::Apt);
    /// assert_eq!(apt.executable_name(), "apt");
    ///
    /// let npm = PackageManager::Language(LanguagePackageManager::Npm);
    /// assert_eq!(npm.executable_name(), "npm");
    /// ```
    #[must_use]
    pub const fn executable_name(&self) -> &'static str {
        match self {
            PackageManager::Os(os) => os.executable_name(),
            PackageManager::Language(lang) => lang.executable_name(),
        }
    }

    /// Returns whether this is an OS-level package manager.
    #[must_use]
    pub const fn is_os(&self) -> bool {
        matches!(self, PackageManager::Os(_))
    }

    /// Returns whether this is a language ecosystem package manager.
    #[must_use]
    pub const fn is_language(&self) -> bool {
        matches!(self, PackageManager::Language(_))
    }
}

impl fmt::Display for PackageManager {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PackageManager::Os(os) => write!(f, "{os}"),
            PackageManager::Language(lang) => write!(f, "{lang}"),
        }
    }
}

impl From<OsPackageManager> for PackageManager {
    fn from(os: OsPackageManager) -> Self {
        PackageManager::Os(os)
    }
}

impl From<LanguagePackageManager> for PackageManager {
    fn from(lang: LanguagePackageManager) -> Self {
        PackageManager::Language(lang)
    }
}

// ============================================================================
// PackageManagerShape Trait
// ============================================================================

/// Boxed future type for async trait methods.
///
/// This type alias provides dyn-compatible async method returns.
pub type BoxFuture<'a, T> = std::pin::Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Trait defining the interface for package manager operations.
///
/// This trait is dyn-compatible (object-safe) by using boxed futures for
/// async methods. Implementors must be `Send + Sync` to support concurrent
/// operations and storage in static registries.
///
/// ## Examples
///
/// ```ignore
/// use sniff_lib::package::{PackageManagerShape, PackageInfo};
///
/// async fn check_package(mgr: &dyn PackageManagerShape, name: &str) {
///     if mgr.is_available() {
///         match mgr.find_package(name).await {
///             Ok(Some(info)) => println!("Found: {} v{}", info.name, info.version),
///             Ok(None) => println!("Package not found"),
///             Err(e) => eprintln!("Error: {}", e),
///         }
///     }
/// }
/// ```
pub trait PackageManagerShape: Send + Sync {
    /// Returns the executable name for this package manager.
    fn executable_name(&self) -> &'static str;

    /// Checks if this package manager is available on the system.
    ///
    /// This performs a filesystem check (no process spawning) to determine
    /// if the package manager executable exists in the PATH.
    fn is_available(&self) -> bool;

    /// Finds a package by name, returning metadata if found.
    ///
    /// ## Arguments
    ///
    /// * `name` - The package name to search for
    ///
    /// ## Returns
    ///
    /// - `Ok(Some(PackageInfo))` if the package is found
    /// - `Ok(None)` if the package is not found
    /// - `Err(_)` if an error occurred during the search
    fn find_package(&self, name: &str) -> BoxFuture<'_, Result<Option<PackageInfo>>>;

    /// Gets the latest version of a package.
    ///
    /// ## Arguments
    ///
    /// * `name` - The package name to query
    ///
    /// ## Returns
    ///
    /// - `Ok(Some(version))` if the package is found with a version
    /// - `Ok(None)` if the package is not found or has no version
    /// - `Err(_)` if an error occurred during the query
    fn latest_version(&self, name: &str) -> BoxFuture<'_, Result<Option<String>>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_package_manager_wrapper() {
        let os_mgr = PackageManager::Os(OsPackageManager::Apt);
        let lang_mgr = PackageManager::Language(LanguagePackageManager::Npm);

        assert!(os_mgr.is_os());
        assert!(!os_mgr.is_language());
        assert!(!lang_mgr.is_os());
        assert!(lang_mgr.is_language());

        assert_eq!(os_mgr.executable_name(), "apt");
        assert_eq!(lang_mgr.executable_name(), "npm");
    }

    #[test]
    fn test_package_manager_display() {
        let os_mgr = PackageManager::Os(OsPackageManager::Homebrew);
        let lang_mgr = PackageManager::Language(LanguagePackageManager::Cargo);

        assert_eq!(format!("{os_mgr}"), "brew");
        assert_eq!(format!("{lang_mgr}"), "cargo");
    }

    #[test]
    fn test_package_manager_from_impls() {
        let os: PackageManager = OsPackageManager::Apt.into();
        let lang: PackageManager = LanguagePackageManager::Npm.into();

        assert!(matches!(os, PackageManager::Os(OsPackageManager::Apt)));
        assert!(matches!(
            lang,
            PackageManager::Language(LanguagePackageManager::Npm)
        ));
    }

    #[test]
    fn test_serde_roundtrip_wrapper() {
        let mgr = PackageManager::Language(LanguagePackageManager::Poetry);
        let json = serde_json::to_string(&mgr).unwrap();
        let restored: PackageManager = serde_json::from_str(&json).unwrap();
        assert_eq!(mgr, restored);
    }
}
