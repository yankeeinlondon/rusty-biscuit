pub mod catalog;
pub mod config;
pub mod decision;
mod downgrade;
pub mod evaluate;
pub mod explain;
pub mod intent;
pub mod matcher;
pub mod observe;
pub mod path;
pub mod redact;
pub mod report;
pub mod request;
pub mod service;
pub mod state;

// Re-exports for public API surface
pub use config::ProtectConfig;

pub use decision::{ProtectDecision, ProtectMatch, ProtectOutcome};
