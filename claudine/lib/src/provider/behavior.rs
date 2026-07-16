//! Focused behavior traits for genuinely dynamic per-provider operations.
//!
//! Each trait absorbs a slice of per-domain dispatch that previously lived as
//! `match Provider { ... }` arms scattered across the lib crate. After Phase 4
//! every provider domain (stream, MCP, adapters, configurators) routes through
//! the four traits in this module so the only authoritative `match Provider`
//! site in the lib crate is the central registry in
//! [`crate::provider::registry`].
//!
//! Every provider implements all four traits. Default implementations encode
//! "not supported" semantics so call sites never need `Option` unwrapping; a
//! provider only overrides methods it actually supports.

use std::path::{Path, PathBuf};

use serde_json::Value;

use super::errors::{ConfigError, McpError};
use crate::hook_adapters::ProviderAdapter;
use crate::config::AgentConfigurator;
use crate::error::Result;
use crate::events::AgenticEvent;
use crate::mcp::export::ExportServer;
use crate::mcp::inject::McpInjector;
use crate::mcp::state::Scope;
use crate::mcp::types::McpServer;
use crate::stream::ParserConfig;
use crate::stream::parser::SemanticStreamParser;
use crate::stream::semantic::SemanticEventSink;

/// Boxed [`SemanticEventSink`] used for trait-object-friendly parser
/// construction. Any concrete sink can be boxed and accepted via the blanket
/// `impl<T: SemanticEventSink + ?Sized> SemanticEventSink for Box<T>`.
pub type BoxedSemanticEventSink = Box<dyn SemanticEventSink + Send + 'static>;

/// Cross-cutting dynamic behavior shared across all per-domain dispatch.
///
/// Houses any operations that are inherently dynamic (run-time arguments,
/// owned return values) and don't fit cleanly into the more focused MCP /
/// adapter / configurator surfaces.
pub trait ProviderBehavior: Send + Sync + std::fmt::Debug + 'static {
    /// Whether the given raw JSON payload is recognized as originating from
    /// this provider.
    fn detect_from_payload(&self, raw: &Value) -> bool;

    /// Construct a [`SemanticStreamParser`] for this provider's structured
    /// output stream. Providers without a native semantic parser fall back
    /// to a degenerate Claude-shaped parser to match the legacy behavior of
    /// [`crate::stream::create_semantic_parser`].
    fn create_semantic_parser(
        &self,
        sink: BoxedSemanticEventSink,
        config: ParserConfig,
    ) -> Box<dyn SemanticStreamParser> {
        crate::stream::providers::for_provider(super::Provider::Claude, sink, config)
    }
}

/// MCP import / state / injection / export behavior.
///
/// Methods default to "not supported" semantics so providers without MCP
/// behavior compile without overrides. Providers that support MCP override
/// the methods they need; the dispatch helpers in `crate::mcp::*` route
/// through these methods.
pub trait McpBehavior: Send + Sync + std::fmt::Debug + 'static {
    /// Whether this provider participates in any MCP workflow at all.
    /// Defaults to `false`.
    fn supported(&self) -> bool {
        false
    }

    /// Returns [`McpError::NotSupported`] by default.
    fn ensure_supported(&self) -> std::result::Result<(), McpError> {
        if self.supported() {
            Ok(())
        } else {
            Err(McpError::NotSupported)
        }
    }

    /// Returns the runtime injector for this provider, if one exists.
    /// Defaults to `None`.
    fn runtime_injector(&self) -> Option<Box<dyn McpInjector>> {
        None
    }

    /// Discover provider-native MCP config locations (user + optional repo).
    fn discover_configs(&self, repo_root: Option<&Path>) -> Vec<(PathBuf, Scope)> {
        let _ = repo_root;
        Vec::new()
    }

    /// Parse provider-native MCP config into `(native_name, McpServer)` pairs.
    fn parse_config(&self, config_path: &Path) -> Result<Vec<(String, McpServer)>> {
        let _ = config_path;
        Ok(Vec::new())
    }

    /// Resolve the native MCP config path for a scope, when supported.
    fn native_config_path(&self, scope: &Scope) -> Option<PathBuf> {
        let _ = scope;
        None
    }

    /// Read the names of MCP servers currently in the native config, if any.
    fn read_existing_native_servers(&self, config_path: &Path) -> Result<Vec<String>> {
        let _ = config_path;
        Ok(Vec::new())
    }

    /// Write managed MCP servers to the native config. Default returns
    /// [`crate::error::ClaudineError::McpProviderNotSupported`].
    fn write_native_config(
        &self,
        servers: &[ExportServer<'_>],
        config_path: &Path,
        managed_names: &[String],
    ) -> Result<()> {
        let _ = (servers, config_path, managed_names);
        Err(crate::error::ClaudineError::McpProviderNotSupported {
            provider: self.provider_for_error(),
            reason: "export not implemented".into(),
        })
    }

    /// Provider identity used when constructing default error returns. The
    /// default is `Provider::Claude` solely to keep this trait object-safe;
    /// every concrete impl overrides it.
    fn provider_for_error(&self) -> super::Provider {
        super::Provider::Claude
    }
}

/// Inbound payload parsing behavior. Each provider's behavior implementor
/// returns a `&'static dyn ProviderAdapter` that handles structured event
/// parsing and protect-decision mapping.
pub trait AdapterBehavior: Send + Sync + std::fmt::Debug + 'static {
    /// Whether the given raw JSON payload is recognized as originating from
    /// this provider. Defaults to `false`.
    fn detect(&self, raw: &Value) -> bool {
        let _ = raw;
        false
    }

    /// Attempt to parse a raw JSON payload into a canonical [`AgenticEvent`].
    ///
    /// Defaults to `None`; providers that support non-hook capture methods
    /// (stream parsing, wire proxy, wrapper) may override this to extract
    /// events from provider-native payloads.
    fn parse_payload(&self, raw: &Value) -> Option<AgenticEvent> {
        let _ = raw;
        None
    }

    /// Returns the static [`ProviderAdapter`] implementation for this
    /// provider. The default panics; every provider behavior implementor
    /// must override it.
    fn provider_adapter(&self) -> &'static dyn ProviderAdapter {
        panic!("provider_adapter() not implemented for this provider behavior")
    }
}

/// Hook registration / configuration-file mutation behavior. Each provider
/// behavior implementor returns a boxed [`AgentConfigurator`] used by the
/// hook registration pipeline.
pub trait ConfiguratorBehavior: Send + Sync + std::fmt::Debug + 'static {
    /// Whether this provider supports any form of hook registration.
    /// Defaults to `false`.
    fn hooks_supported(&self) -> bool {
        false
    }

    /// Returns [`ConfigError::NotSupported`] by default.
    fn ensure_supported(&self) -> std::result::Result<(), ConfigError> {
        if self.hooks_supported() {
            Ok(())
        } else {
            Err(ConfigError::NotSupported)
        }
    }

    /// Returns a boxed [`AgentConfigurator`] for this provider. Every
    /// provider behavior implementor must override this; the default
    /// panics so missing overrides surface immediately.
    fn agent_configurator(&self) -> Box<dyn AgentConfigurator> {
        panic!("agent_configurator() not implemented for this provider behavior")
    }
}
