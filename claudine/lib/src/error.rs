use biscuit_file::YamlParseError;
use biscuit_terminal::components::status::StatusState;
use biscuit_terminal::components::status_block::StatusBlock;
use biscuit_terminal::errors::{BlockError, ErrorHeader, StatusBlockExt};
use biscuit_terminal::terminal::Terminal;
use darkmatter::markdown::MarkdownError;
use serde_json::{Value, json};
use std::path::PathBuf;

use crate::diagnostics::{Category, Diagnostic, Disposition, Origin, code_spec, null_detail_for};
use crate::provider::Provider;
/// All errors that can occur within the Claudine library.
#[derive(Debug, thiserror::Error)]
pub enum ClaudineError {
    /// I/O error during file or directory operations.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Failed to parse JSON content.
    #[error("JSON parse error: {0}")]
    JsonParse(#[from] serde_json::Error),

    /// Failed to parse TOML content.
    #[error("TOML parse error: {0}")]
    TomlParse(#[from] toml_edit::TomlError),

    /// Failed to parse YAML content (e.g., SKILL.md frontmatter).
    #[error("YAML parse error: {0}")]
    YamlParse(#[from] YamlParseError),

    /// Configuration file not found at expected path.
    #[error("config not found: {0}")]
    ConfigNotFound(PathBuf),

    /// Configuration file failed semantic validation.
    #[error("config validation error: {0}")]
    ConfigValidation(String),

    /// Requested provider is not available or not detected.
    #[error("provider not available: {0}")]
    ProviderNotAvailable(String),

    /// Error during template interpolation.
    #[error("template error: {0}")]
    TemplateError(String),

    /// Error during skill/command linking.
    #[error("linking error: {0}")]
    LinkingError(String),

    /// HTTP request failed (e.g., log server POST).
    #[error("HTTP error: {0}")]
    HttpError(#[from] reqwest::Error),

    /// File lock could not be acquired within timeout.
    #[error("lock timeout on {path}")]
    LockError {
        /// Path that could not be locked.
        path: PathBuf,
    },

    /// SQLite access failed while working with the reporting index.
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    /// Date or timestamp parsing failed.
    #[error("date/time parse error: {0}")]
    ChronoParse(#[from] chrono::ParseError),

    /// Regex compilation failed.
    #[error("invalid regex pattern: {0}")]
    RegexError(#[from] regex::Error),

    /// URL parsing failed.
    #[error("invalid URL: {0}")]
    UrlError(#[from] url::ParseError),

    /// Provider adapter parse/format error.
    #[error("adapter error: {0}")]
    Adapter(#[from] crate::hook_adapters::AdapterError),

    /// Provider does not support automatic config creation.
    #[error("config creation not supported for provider: {provider}")]
    ConfigCreationNotSupported {
        /// Provider name.
        provider: Provider,
    },

    /// Protect rule pattern failed to parse as regex.
    #[error("protect rule parse error for pattern `{pattern}`: {source}")]
    ProtectRuleParse {
        /// Rule regex pattern.
        pattern: String,
        /// Regex parser error.
        source: regex::Error,
    },

    /// Protect policy is semantically invalid.
    #[error("protect policy invalid: {0}")]
    ProtectInvalidPolicy(String),

    /// Failed mapping a protect outcome to provider-native enforcement.
    #[error("protect enforcement mapping error: {0}")]
    ProtectEnforcementMapping(String),

    /// Required Claudine reporting path could not be determined.
    #[error("reporting path unavailable: {0}")]
    ReportingPathUnavailable(String),

    /// Reporting date range is invalid.
    #[error("invalid reporting date range: {from} > {to}")]
    InvalidReportingDateRange {
        /// Inclusive range start.
        from: String,
        /// Inclusive range end.
        to: String,
    },

    /// MCP catalog file not found.
    #[error("MCP catalog not found at ~/.claudine/mcp/catalog.json")]
    McpCatalogNotFound,

    /// MCP server not found in the catalog.
    #[error("MCP server not found: {id}")]
    McpServerNotFound {
        /// Server ID that was looked up.
        id: String,
    },

    /// Alias conflicts with an existing server ID or alias.
    #[error("alias `{alias}` conflicts with existing server `{existing_id}`")]
    McpAliasConflict {
        /// The alias that was being added.
        alias: String,
        /// The existing server ID or alias owner.
        existing_id: String,
    },

    /// Query matched multiple servers ambiguously.
    #[error("ambiguous MCP match for `{query}`: {}", candidates.join(", "))]
    McpAmbiguousMatch {
        /// The query string that produced multiple matches.
        query: String,
        /// The candidate server IDs.
        candidates: Vec<String>,
    },

    /// Import detected a naming conflict across providers.
    #[error("MCP import conflict for `{name}` across providers: {}", providers.join(", "))]
    McpImportConflict {
        /// The conflicting server name.
        name: String,
        /// Providers that define this name differently.
        providers: Vec<String>,
    },

    /// Provider does not support the requested MCP operation.
    #[error("MCP not supported for {provider}: {reason}")]
    McpProviderNotSupported {
        /// Provider name.
        provider: Provider,
        /// Why the operation is not supported.
        reason: String,
    },

    // --- PolicyEngine errors ---
    /// Policy engine backend is not registered for this provider.
    #[error("policy engine: no backend for {0}")]
    PolicyBackendUnavailable(crate::provider::Provider),

    /// Policy engine source discovery failed.
    #[error("policy engine source discovery: {0}")]
    PolicySourceDiscovery(String),

    /// Policy engine native config parse failure.
    #[error("policy engine native parse for source `{source_id}`: {message}")]
    PolicyNativeParse {
        /// Source identifier.
        source_id: String,
        /// Parse error message.
        message: String,
    },

    /// Policy engine CLI override parse failure.
    #[error("policy engine CLI parse for {provider}: {message}")]
    PolicyCliParse {
        /// Provider.
        provider: crate::provider::Provider,
        /// Parse error message.
        message: String,
    },

    /// Policy engine query is unsupported by the provider backend.
    #[error("policy engine unsupported query for {provider}: {query}")]
    PolicyUnsupportedQuery {
        /// Provider.
        provider: crate::provider::Provider,
        /// Query description.
        query: String,
    },

    /// Policy engine mutation is unsupported by the provider backend.
    #[error("policy engine unsupported mutation for {provider}: {op}")]
    PolicyUnsupportedMutation {
        /// Provider.
        provider: crate::provider::Provider,
        /// Operation description.
        op: String,
    },

    /// Policy engine mutation apply failed.
    #[error("policy engine apply failed at {path}: {message}")]
    PolicyApplyFailed {
        /// File path.
        path: PathBuf,
        /// Error message.
        message: String,
    },

    /// Policy engine context is ambiguous.
    #[error("policy engine ambiguous context: {0}")]
    PolicyAmbiguousContext(String),

    // --- System prompt errors ---
    /// Launch context detection failed (git or repo detection error).
    #[error("launch context detection failed: {0}")]
    LaunchContextDetection(String),

    /// System prompt file not found.
    #[error("system prompt file not found: {0}")]
    SystemPromptFileNotFound(String),

    /// System prompt composition through Darkmatter failed.
    ///
    /// Carries the typed `MarkdownError` so the CLI's top-level walker can
    /// render a rich `BlockError` report (path, line, hint, transclusion
    /// chain, etc.) instead of a flat string.
    #[error("system prompt composition failed: {0}")]
    SystemPromptComposition(#[from] MarkdownError),
}

impl BlockError for ClaudineError {
    fn status_block(&self, _term: &Terminal) -> StatusBlock {
        // Claudine's library errors are reported by the CLI's top-level walker,
        // which renders the typed `Display` text; the block-style report here
        // carries the same message under the variant-derived code so the
        // `Diagnostic` supertrait has a uniform human surface.
        StatusBlock::new(StatusState::Error)
            .error_header(ErrorHeader::new("ClaudineError", self.code()))
            .body(self.to_string())
    }
}

impl Diagnostic for ClaudineError {
    fn code(&self) -> &'static str {
        match self {
            // `provider.*` — the agent run as infrastructure.
            ClaudineError::ProviderNotAvailable(_) => "provider.unavailable",
            ClaudineError::Adapter(_) => "provider.stream_error",
            // `io.*` — filesystem / network plumbing.
            ClaudineError::Io(e) => match e.kind() {
                std::io::ErrorKind::PermissionDenied => "io.permission_denied",
                _ => "io.read_failed",
            },
            ClaudineError::HttpError(_) | ClaudineError::UrlError(_) => "io.network",
            ClaudineError::Sqlite(_) | ClaudineError::SystemPromptFileNotFound(_) => {
                "io.read_failed"
            }
            ClaudineError::LockError { .. } | ClaudineError::LinkingError(_) => "io.write_failed",
            // `config.*` — Claudine/user configuration is invalid.
            ClaudineError::ConfigNotFound(_)
            | ClaudineError::ConfigValidation(_)
            | ClaudineError::JsonParse(_)
            | ClaudineError::TomlParse(_)
            | ClaudineError::YamlParse(_)
            | ClaudineError::ChronoParse(_)
            | ClaudineError::RegexError(_)
            | ClaudineError::ProtectRuleParse { .. }
            | ClaudineError::ProtectInvalidPolicy(_)
            | ClaudineError::ProtectEnforcementMapping(_)
            | ClaudineError::PolicyBackendUnavailable(_)
            | ClaudineError::PolicySourceDiscovery(_)
            | ClaudineError::PolicyNativeParse { .. }
            | ClaudineError::PolicyCliParse { .. }
            | ClaudineError::PolicyApplyFailed { .. }
            | ClaudineError::PolicyAmbiguousContext(_) => "config.invalid",
            ClaudineError::McpCatalogNotFound
            | ClaudineError::McpServerNotFound { .. }
            | ClaudineError::McpAliasConflict { .. }
            | ClaudineError::McpAmbiguousMatch { .. }
            | ClaudineError::McpImportConflict { .. } => "config.mcp_invalid",
            // `usage.*` — Rust API misuse / unsupported operations.
            ClaudineError::ConfigCreationNotSupported { .. }
            | ClaudineError::McpProviderNotSupported { .. }
            | ClaudineError::PolicyUnsupportedQuery { .. }
            | ClaudineError::PolicyUnsupportedMutation { .. } => "usage.unsupported",
            // `composition.*` — delegate a system-prompt compose failure.
            ClaudineError::SystemPromptComposition(_) => "composition.failed",
            // Everything else is an unclassified internal condition.
            ClaudineError::TemplateError(_)
            | ClaudineError::LaunchContextDetection(_)
            | ClaudineError::ReportingPathUnavailable(_)
            | ClaudineError::InvalidReportingDateRange { .. } => "internal.bug",
        }
    }

    fn category(&self) -> Category {
        code_spec(self.code())
            .map(|spec| spec.category)
            .unwrap_or(Category::Internal)
    }

    fn disposition(&self) -> Disposition {
        code_spec(self.code())
            .map(|spec| spec.disposition)
            .unwrap_or(Disposition::Unrecoverable)
    }

    fn origin(&self) -> Origin {
        code_spec(self.code())
            .map(|spec| spec.origin)
            .unwrap_or(Origin::Internal)
    }

    fn detail(&self) -> Value {
        // The base object carries every field the mapped code declares,
        // pre-seeded to `null`, so an unavailable optional still projects its
        // key (error-catalog §2.5; the catalog makes the field set handleable).
        // Each explicit arm overwrites the keys it can populate; variants with
        // no extractable specifics keep the all-`null` base.
        let mut base = null_detail_for(self.code());

        match self {
            // `provider.unavailable` declares `provider`, `path`. The variant
            // only knows the provider name; the executable path is unknown.
            ClaudineError::ProviderNotAvailable(provider) => {
                base["provider"] = json!(provider);
            }
            // `io.read_failed` declares `path`.
            ClaudineError::SystemPromptFileNotFound(path) => {
                base["path"] = json!(path);
            }
            // `io.write_failed` declares `path`.
            ClaudineError::LockError { path } => {
                base["path"] = json!(path.to_string_lossy());
            }
            // `io.network` declares `url`, `message`. The typed source carries
            // no parsed URL to surface, so only `message` is populated.
            ClaudineError::HttpError(_) | ClaudineError::UrlError(_) => {
                base["message"] = json!(self.to_string());
            }
            // `config.invalid` declares `field`, `message`.
            ClaudineError::ConfigValidation(message)
            | ClaudineError::ProtectInvalidPolicy(message)
            | ClaudineError::ProtectEnforcementMapping(message)
            | ClaudineError::PolicySourceDiscovery(message)
            | ClaudineError::PolicyAmbiguousContext(message) => {
                base["message"] = json!(message);
            }
            ClaudineError::PolicyApplyFailed { path, message } => {
                base["field"] = json!(path.to_string_lossy());
                base["message"] = json!(message);
            }
            ClaudineError::PolicyNativeParse { source_id, message } => {
                base["field"] = json!(source_id);
                base["message"] = json!(message);
            }
            ClaudineError::PolicyCliParse { provider, message } => {
                base["field"] = json!(provider.to_string());
                base["message"] = json!(message);
            }
            ClaudineError::ProtectRuleParse { pattern, .. } => {
                base["field"] = json!(pattern);
                base["message"] = json!(self.to_string());
            }
            // The remaining `config.invalid` variants carry a typed parse
            // source but no distinct `field`; surface the display message.
            ClaudineError::ConfigNotFound(_)
            | ClaudineError::JsonParse(_)
            | ClaudineError::TomlParse(_)
            | ClaudineError::YamlParse(_)
            | ClaudineError::ChronoParse(_)
            | ClaudineError::RegexError(_)
            | ClaudineError::PolicyBackendUnavailable(_) => {
                base["message"] = json!(self.to_string());
            }
            // `config.mcp_invalid` declares `server`, `message`.
            ClaudineError::McpServerNotFound { id } => {
                base["server"] = json!(id);
                base["message"] = json!(self.to_string());
            }
            ClaudineError::McpAliasConflict { existing_id, .. } => {
                base["server"] = json!(existing_id);
                base["message"] = json!(self.to_string());
            }
            ClaudineError::McpAmbiguousMatch { query, .. } => {
                base["server"] = json!(query);
                base["message"] = json!(self.to_string());
            }
            ClaudineError::McpImportConflict { name, .. } => {
                base["server"] = json!(name);
                base["message"] = json!(self.to_string());
            }
            ClaudineError::McpCatalogNotFound => {
                base["message"] = json!(self.to_string());
            }
            // `usage.unsupported` declares `operation`, `provider`.
            ClaudineError::McpProviderNotSupported { provider, reason } => {
                base["operation"] = json!(reason);
                base["provider"] = json!(provider.to_string());
            }
            ClaudineError::ConfigCreationNotSupported { provider } => {
                base["operation"] = json!(self.to_string());
                base["provider"] = json!(provider.to_string());
            }
            ClaudineError::PolicyUnsupportedQuery { provider, query } => {
                base["operation"] = json!(query);
                base["provider"] = json!(provider.to_string());
            }
            ClaudineError::PolicyUnsupportedMutation { provider, op } => {
                base["operation"] = json!(op);
                base["provider"] = json!(provider.to_string());
            }
            // `internal.bug` declares `message`.
            ClaudineError::TemplateError(message)
            | ClaudineError::LaunchContextDetection(message)
            | ClaudineError::ReportingPathUnavailable(message) => {
                base["message"] = json!(message);
            }
            ClaudineError::InvalidReportingDateRange { .. } => {
                base["message"] = json!(self.to_string());
            }
            // `provider.stream_error` (`Adapter`), `io.read_failed`
            // (`Sqlite`), `io.permission_denied` / `io.read_failed` (`Io`),
            // `io.write_failed` (`LinkingError`), and the
            // `composition.failed`-mapped `SystemPromptComposition` all carry
            // no field the registry declares that is separately extractable
            // here; the all-`null` base already satisfies their key set.
            _ => {}
        }

        base
    }
}

/// Convenience type alias for Claudine results.
pub type Result<T> = std::result::Result<T, ClaudineError>;

#[cfg(test)]
mod diagnostic_tests {
    use super::*;

    #[test]
    fn provider_not_available_classifies_as_provider_unavailable() {
        let err = ClaudineError::ProviderNotAvailable("codex".to_string());
        assert_eq!(err.code(), "provider.unavailable");
        assert_eq!(err.category(), Category::Provider);
        assert_eq!(err.origin(), Origin::Environment);
        assert_eq!(err.detail()["provider"], json!("codex"));
    }

    #[test]
    fn io_permission_denied_classifies_as_io_permission() {
        let err = ClaudineError::Io(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "denied",
        ));
        assert_eq!(err.code(), "io.permission_denied");
        assert_eq!(err.category(), Category::Io);
    }

    #[test]
    fn generic_io_classifies_as_io_read() {
        let err = ClaudineError::Io(std::io::Error::other("boom"));
        assert_eq!(err.code(), "io.read_failed");
    }

    #[test]
    fn config_validation_classifies_as_config_invalid() {
        let err = ClaudineError::ConfigValidation("bad field".to_string());
        assert_eq!(err.code(), "config.invalid");
        assert_eq!(err.detail()["message"], json!("bad field"));
    }

    #[test]
    fn unsupported_operation_classifies_as_usage_unsupported() {
        let err = ClaudineError::McpProviderNotSupported {
            provider: Provider::Goose,
            reason: "no MCP".to_string(),
        };
        assert_eq!(err.code(), "usage.unsupported");
        assert_eq!(err.detail()["operation"], json!("no MCP"));
    }

    #[test]
    fn every_variant_maps_to_a_locked_catalog_code() {
        // Spot-check across families that each `code()` is a real catalog row.
        for err in [
            ClaudineError::ProviderNotAvailable("x".into()),
            ClaudineError::ConfigValidation("x".into()),
            ClaudineError::McpCatalogNotFound,
            ClaudineError::TemplateError("x".into()),
            ClaudineError::LinkingError("x".into()),
        ] {
            assert!(
                code_spec(err.code()).is_some(),
                "`{err:?}` → `{}` is not a locked catalog code",
                err.code()
            );
        }
    }
}
