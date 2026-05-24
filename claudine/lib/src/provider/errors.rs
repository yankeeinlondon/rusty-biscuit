//! Domain-specific error stubs surfaced by behavior traits.
//!
//! These error types are intentionally minimal during Phase 1. Phase 4 will
//! grow them to cover real MCP and configurator failure modes.

use thiserror::Error;

/// Errors returned by [`McpBehavior`](super::McpBehavior) operations.
#[derive(Debug, Error)]
pub enum McpError {
    /// MCP is not supported for this provider.
    #[error("MCP not supported for this provider")]
    NotSupported,
}

/// Errors returned by [`ConfiguratorBehavior`](super::ConfiguratorBehavior)
/// operations.
#[derive(Debug, Error)]
pub enum ConfigError {
    /// Hook configuration is not supported for this provider.
    #[error("hook configuration not supported for this provider")]
    NotSupported,
}
