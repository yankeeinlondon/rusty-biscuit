//! Terminal app detection — type alias for `CategoryDetector<TerminalApp>`.

use crate::programs::enums::TerminalApp;
use crate::programs::category_detector::CategoryDetector;

/// Terminal emulator applications found on the system.
pub type InstalledTerminalApps = CategoryDetector<TerminalApp>;
