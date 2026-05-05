//! Headless audio detection — type alias for `CategoryDetector<HeadlessAudio>`.

use crate::programs::enums::HeadlessAudio;
use crate::programs::category_detector::CategoryDetector;

/// Headless audio players found on the system.
pub type InstalledHeadlessAudio = CategoryDetector<HeadlessAudio>;
