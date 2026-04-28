//! Focused behavior traits for genuinely dynamic per-provider operations.
//!
//! Each trait is intentionally minimal during Phase 1 of the centralized
//! providers refactor. Phase 4 will extend the trait surfaces to absorb the
//! per-domain dispatch in `stream`, `mcp`, `adapters`, and `config`.
//!
//! Every provider implements all four behavior traits. Default
//! implementations encode "not supported" semantics so call sites never need
//! `Option` unwrapping; a provider only overrides methods it actually
//! supports.

use serde_json::Value;

use super::errors::{ConfigError, McpError};

/// Cross-cutting dynamic behavior shared across all per-domain dispatch.
///
/// Phase 4 will extend this trait with `create_semantic_parser` and any
/// other dynamic operations that take runtime arguments or return owned
/// values.
pub trait ProviderBehavior: Send + Sync + std::fmt::Debug + 'static {
    /// Whether the given raw JSON payload is recognized as originating from
    /// this provider. Defaults to `false`.
    fn detect_from_payload(&self, raw: &Value) -> bool {
        let _ = raw;
        false
    }
}

/// MCP import / state / injection / export behavior.
///
/// Phase 4 will replace these stubs with the real per-method signatures
/// pulled from [`crate::mcp`]. For Phase 1 the trait only carries
/// "not supported" defaults so the four-trait scaffolding compiles.
pub trait McpBehavior: Send + Sync + std::fmt::Debug + 'static {
    /// Whether this provider participates in any MCP workflow at all.
    /// Defaults to `false`.
    fn supported(&self) -> bool {
        false
    }

    /// Returns [`McpError::NotSupported`] by default.
    fn ensure_supported(&self) -> Result<(), McpError> {
        if self.supported() {
            Ok(())
        } else {
            Err(McpError::NotSupported)
        }
    }
}

/// Inbound payload parsing behavior.
///
/// Phase 4 will absorb the per-provider parser dispatch from
/// [`crate::adapters`]. Phase 1 keeps the trait as a no-op detection stub.
pub trait AdapterBehavior: Send + Sync + std::fmt::Debug + 'static {
    /// Whether the given raw JSON payload is recognized as originating from
    /// this provider. Defaults to `false`.
    fn detect(&self, raw: &Value) -> bool {
        let _ = raw;
        false
    }
}

/// Hook registration / configuration-file mutation behavior.
///
/// Phase 4 will absorb the per-provider configurator dispatch from
/// [`crate::config`]. Phase 1 keeps the trait as a "not supported" stub.
pub trait ConfiguratorBehavior: Send + Sync + std::fmt::Debug + 'static {
    /// Whether this provider supports any form of hook registration.
    /// Defaults to `false`.
    fn hooks_supported(&self) -> bool {
        false
    }

    /// Returns [`ConfigError::NotSupported`] by default.
    fn ensure_supported(&self) -> Result<(), ConfigError> {
        if self.hooks_supported() {
            Ok(())
        } else {
            Err(ConfigError::NotSupported)
        }
    }
}
