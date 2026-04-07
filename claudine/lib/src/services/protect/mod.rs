pub mod catalog;
pub mod config;
pub mod decision;
pub mod matcher;
pub mod observe;
pub mod path;
pub mod report;
pub mod service;

// Re-exports for public API surface
pub use catalog::{ProtectPlatform, RuleGroup, ScanSurface};
pub use config::{CustomPattern, ProtectConfig, ProtectRuleToggles, RuleGroupConfig};
pub use decision::{ProtectDecision, ProtectMatch, ProtectOutcome};
pub use observe::extract_protect_request;
pub use report::format_blocked_message;
pub use service::{ProtectRequest, ProtectService};
