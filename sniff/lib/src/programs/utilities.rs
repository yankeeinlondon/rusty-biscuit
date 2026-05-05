//! Utility detection — type alias for `CategoryDetector<Utility>`.

use crate::programs::enums::Utility;
use crate::programs::category_detector::CategoryDetector;

/// Modern command-line utilities found on the system.
pub type InstalledUtilities = CategoryDetector<Utility>;
