//! Package manager detection — type aliases for language and OS package managers.

use crate::programs::enums::{LanguagePackageManager, OsPackageManager};
use crate::programs::category_detector::CategoryDetector;

/// Language-specific package managers found on the system.
pub type InstalledLanguagePackageManagers = CategoryDetector<LanguagePackageManager>;

/// Operating system package managers found on the system.
pub type InstalledOsPackageManagers = CategoryDetector<OsPackageManager>;
