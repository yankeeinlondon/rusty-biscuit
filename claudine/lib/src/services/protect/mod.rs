pub mod catalog;
pub mod config;
pub mod decision;
pub mod matcher;
pub mod path;
pub mod report;

// Re-exports for public API surface
pub use config::ProtectConfig;

pub use decision::{ProtectDecision, ProtectMatch, ProtectOutcome};
