//! Static registry for package manager implementations.
//!
//! This module provides a global registry of package manager implementations
//! using [`LazyLock`] for thread-safe lazy initialization.
//!
//! ## Examples
//!
//! ```
//! use sniff_lib::package::{get_package_manager, LanguagePackageManager, PackageManagerShape};
//!
//! if let Some(npm) = get_package_manager(&LanguagePackageManager::Npm.into()) {
//!     println!("npm available: {}", npm.is_available());
//! }
//! ```

use std::collections::HashMap;
use std::sync::LazyLock;

use super::network::{BunNetwork, CargoNetwork, NpmNetwork, PnpmNetwork, YarnNetwork};
use super::stubs::{AptStub, HomebrewStub, PipStub};
use super::{LanguagePackageManager, OsPackageManager, PackageManager, PackageManagerShape};

// ============================================================================
// Static Registry
// ============================================================================

/// Wrapper type for storing trait objects in the registry.
///
/// This struct wraps a `Box<dyn PackageManagerShape>` and provides
/// the `PackageManagerShape` implementation by delegation.
struct RegistryEntry {
    inner: Box<dyn PackageManagerShape>,
}

impl RegistryEntry {
    fn new<T: PackageManagerShape + 'static>(impl_: T) -> Self {
        Self {
            inner: Box::new(impl_),
        }
    }
}

/// Static registry mapping package managers to their implementations.
///
/// This is lazily initialized on first access and provides thread-safe
/// access to package manager implementations.
static PACKAGE_MANAGER_REGISTRY: LazyLock<HashMap<PackageManager, RegistryEntry>> =
    LazyLock::new(|| {
        let mut map = HashMap::new();

        // OS Package Managers
        map.insert(
            PackageManager::Os(OsPackageManager::Apt),
            RegistryEntry::new(AptStub),
        );
        map.insert(
            PackageManager::Os(OsPackageManager::Homebrew),
            RegistryEntry::new(HomebrewStub),
        );

        // Language Package Managers (using network implementations where available)
        map.insert(
            PackageManager::Language(LanguagePackageManager::Npm),
            RegistryEntry::new(NpmNetwork::new()),
        );
        map.insert(
            PackageManager::Language(LanguagePackageManager::Pnpm),
            RegistryEntry::new(PnpmNetwork::new()),
        );
        map.insert(
            PackageManager::Language(LanguagePackageManager::Yarn),
            RegistryEntry::new(YarnNetwork::new()),
        );
        map.insert(
            PackageManager::Language(LanguagePackageManager::YarnClassic),
            RegistryEntry::new(YarnNetwork::new()),
        );
        map.insert(
            PackageManager::Language(LanguagePackageManager::YarnBerry),
            RegistryEntry::new(YarnNetwork::new()),
        );
        map.insert(
            PackageManager::Language(LanguagePackageManager::Bun),
            RegistryEntry::new(BunNetwork::new()),
        );
        map.insert(
            PackageManager::Language(LanguagePackageManager::Cargo),
            RegistryEntry::new(CargoNetwork::new()),
        );
        map.insert(
            PackageManager::Language(LanguagePackageManager::Pip),
            RegistryEntry::new(PipStub),
        );

        map
    });

// ============================================================================
// Public API
// ============================================================================

/// Gets a package manager implementation from the registry.
///
/// Returns a reference to the [`PackageManagerShape`] implementation for the
/// given package manager, if one is registered.
///
/// ## Examples
///
/// ```
/// use sniff_lib::package::{get_package_manager, LanguagePackageManager, PackageManager};
///
/// let npm_key = PackageManager::Language(LanguagePackageManager::Npm);
/// if let Some(npm) = get_package_manager(&npm_key) {
///     println!("npm executable: {}", npm.executable_name());
///     println!("npm available: {}", npm.is_available());
/// }
/// ```
///
/// ## Returns
///
/// - `Some(&dyn PackageManagerShape)` if the package manager is registered
/// - `None` if no implementation is registered for this package manager
#[must_use]
pub fn get_package_manager(manager: &PackageManager) -> Option<&'static dyn PackageManagerShape> {
    PACKAGE_MANAGER_REGISTRY
        .get(manager)
        .map(|entry| entry.inner.as_ref())
}

/// Checks if a package manager has a registered implementation.
///
/// ## Examples
///
/// ```
/// use sniff_lib::package::{LanguagePackageManager, PackageManager};
///
/// let cargo = PackageManager::Language(LanguagePackageManager::Cargo);
/// // Cargo is registered
/// ```
#[must_use]
pub fn is_registered(manager: &PackageManager) -> bool {
    PACKAGE_MANAGER_REGISTRY.contains_key(manager)
}

/// Returns the list of all registered package managers.
///
/// ## Examples
///
/// ```
/// use sniff_lib::package::registered_managers;
///
/// for mgr in registered_managers() {
///     println!("Registered: {}", mgr);
/// }
/// ```
#[must_use]
pub fn registered_managers() -> Vec<&'static PackageManager> {
    PACKAGE_MANAGER_REGISTRY.keys().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_npm_from_registry() {
        let npm = PackageManager::Language(LanguagePackageManager::Npm);
        let impl_ = get_package_manager(&npm);
        assert!(impl_.is_some());
        assert_eq!(impl_.unwrap().executable_name(), "npm");
    }

    #[test]
    fn test_get_cargo_from_registry() {
        let cargo = PackageManager::Language(LanguagePackageManager::Cargo);
        let impl_ = get_package_manager(&cargo);
        assert!(impl_.is_some());
        assert_eq!(impl_.unwrap().executable_name(), "cargo");
    }

    #[test]
    fn test_get_apt_from_registry() {
        let apt = PackageManager::Os(OsPackageManager::Apt);
        let impl_ = get_package_manager(&apt);
        assert!(impl_.is_some());
        assert_eq!(impl_.unwrap().executable_name(), "apt");
    }

    #[test]
    fn test_get_homebrew_from_registry() {
        let brew = PackageManager::Os(OsPackageManager::Homebrew);
        let impl_ = get_package_manager(&brew);
        assert!(impl_.is_some());
        assert_eq!(impl_.unwrap().executable_name(), "brew");
    }

    #[test]
    fn test_unregistered_manager_returns_none() {
        // Maven is not registered (no stub implementation)
        let maven = PackageManager::Language(LanguagePackageManager::Maven);
        let impl_ = get_package_manager(&maven);
        assert!(impl_.is_none());
    }

    #[test]
    fn test_is_registered() {
        let npm = PackageManager::Language(LanguagePackageManager::Npm);
        let maven = PackageManager::Language(LanguagePackageManager::Maven);

        assert!(is_registered(&npm));
        assert!(!is_registered(&maven));
    }

    #[test]
    fn test_registered_managers_not_empty() {
        let managers = registered_managers();
        assert!(!managers.is_empty());
        // We registered at least 10 managers
        assert!(managers.len() >= 8);
    }

    #[test]
    fn test_all_js_managers_registered() {
        let npm = PackageManager::Language(LanguagePackageManager::Npm);
        let pnpm = PackageManager::Language(LanguagePackageManager::Pnpm);
        let yarn = PackageManager::Language(LanguagePackageManager::Yarn);
        let bun = PackageManager::Language(LanguagePackageManager::Bun);

        assert!(is_registered(&npm));
        assert!(is_registered(&pnpm));
        assert!(is_registered(&yarn));
        assert!(is_registered(&bun));
    }

    #[test]
    fn test_yarn_variants_registered() {
        let yarn = PackageManager::Language(LanguagePackageManager::Yarn);
        let classic = PackageManager::Language(LanguagePackageManager::YarnClassic);
        let berry = PackageManager::Language(LanguagePackageManager::YarnBerry);

        assert!(is_registered(&yarn));
        assert!(is_registered(&classic));
        assert!(is_registered(&berry));
    }
}
