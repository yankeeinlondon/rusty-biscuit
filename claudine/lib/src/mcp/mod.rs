//! Provider-agnostic MCP catalog, defaults, provider state, import/export,
//! and runtime injection.
//!
//! See [`claudine/docs/topics/mcp-catalog.md`](../../../docs/topics/mcp-catalog.md)
//! for the normalized catalog model (storage layout, defaults precedence,
//! sync state) and
//! [`claudine/docs/topics/mcp-mode.md`](../../../docs/topics/mcp-mode.md)
//! for wrapper-time injection (`--mcp`, `--use`, provider rollout matrix).

pub mod catalog;
pub mod defaults;
pub mod export;
pub mod import;
pub mod inject;
pub mod session;
pub mod state;
pub mod types;
pub mod validation;

pub use types::*;
