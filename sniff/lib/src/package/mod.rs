//! Package manager detection and abstraction layer.
//!
//! This module provides unified types for working with both operating system-level
//! and language ecosystem package managers. It includes:
//!
//! - [`OsPackageManager`]: System-level package managers (apt, homebrew, pacman, etc.)
//! - [`LanguagePackageManager`]: Language ecosystem package managers (npm, cargo, pip, etc.)
//! - [`PackageManager`]: Unified wrapper enum for both types
//! - [`PackageManagerShape`]: Trait for package manager operations
//!
//! ## Examples
//!
//! ```
//! use sniff_lib::package::{OsPackageManager, LanguagePackageManager, PackageManager};
//!
//! let os_mgr = PackageManager::Os(OsPackageManager::Homebrew);
//! let lang_mgr = PackageManager::Language(LanguagePackageManager::Cargo);
//!
//! println!("OS manager: {}", os_mgr);
//! println!("Language manager: {}", lang_mgr);
//! ```

mod language;
mod manager;
mod network;
mod os;
mod registry;
mod stubs;

// Re-export all public types for API stability
pub use language::LanguagePackageManager;
pub use manager::{BoxFuture, PackageManager, PackageManagerShape};
pub use network::{
    BunNetwork, CargoNetwork, NpmNetwork, PnpmNetwork, YarnNetwork, enrich_dependencies,
    enrich_dependency,
};
pub use os::OsPackageManager;
pub use registry::{get_package_manager, is_registered, registered_managers};
pub use stubs::PackageInfo;
