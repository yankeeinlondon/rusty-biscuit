use biscuit_file::YamlParseError;
use biscuit_terminal::components::status::StatusState;
use biscuit_terminal::components::status_block::StatusBlock;
use biscuit_terminal::errors::{BlockError, ErrorHeader, StatusBlockExt};
use biscuit_terminal::terminal::Terminal;
use darkmatter::markdown::MarkdownError;
use serde_json::{Value, json};
use std::path::PathBuf;
use std::sync::Arc;

use crate::diagnostics::{Category, Diagnostic, Disposition, Origin, code_spec, null_detail_for};
use crate::provider::Provider;

/// Heterogeneous lower-layer cause of a policy-engine parse failure.
///
/// Carried as the typed `#[source]` of [`ClaudineError::PolicyNativeParse`] and
/// [`ClaudineError::PolicyCliParse`] so a handler can recover the concrete
/// parser error via [`std::error::Error::source`] instead of re-parsing the
/// flattened message. Four unrelated parsers feed those two variants — one per
/// provider backend's native config format — which is why this is an enum
/// rather than a single concrete field.
///
/// The two TOML arms are boxed, mirroring `MarkdownLoadCause::Parse`: at 88
/// bytes each they are the only members that matter, and unboxed they push
/// `ClaudineError` past `clippy::result_large_err` across the whole crate. The
/// other two are 8 bytes and stay inline. Boxing costs nothing here — the arm
/// itself is the discriminant, so recovery is by matching it, never by
/// downcasting `Error::source()`.
#[derive(Debug, thiserror::Error)]
pub enum PolicyParseCause {
    /// A JSON config failed to parse (Claude, Gemini, OpenCode, Qwen).
    #[error(transparent)]
    Json(#[from] serde_json::Error),

    /// A TOML config failed to parse through `toml_edit` (Codex, Gemini).
    #[error(transparent)]
    Toml(#[from] Box<toml_edit::TomlError>),

    /// A TOML config failed to parse through `toml`'s deserializer (Kimi).
    ///
    /// Distinct from [`PolicyParseCause::Toml`]: Kimi's backend reads through
    /// `toml::from_str`, which reports a `toml::de::Error`, not `toml_edit`'s.
    #[error(transparent)]
    TomlDe(#[from] Box<toml::de::Error>),

    /// A YAML config failed to parse (Goose).
    #[error(transparent)]
    Yaml(#[from] YamlParseError),
}

/// Heterogeneous lower-layer cause of a configuration validation failure.
///
/// Carried as the typed `#[source]` of
/// [`ClaudineError::ConfigValidationWithCause`]. Modeled on
/// `composition::MarkdownLoadCause`.
#[derive(Debug, thiserror::Error)]
pub enum ConfigCause {
    /// A config file or script could not be read.
    #[error(transparent)]
    Read(#[from] std::io::Error),

    /// A JSON value failed to deserialize into its typed shape.
    #[error(transparent)]
    Json(#[from] serde_json::Error),

    /// A JSON5 document failed to parse.
    #[error(transparent)]
    Json5(#[from] biscuit_file::Json5Error),
}

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

    /// Configuration file failed semantic validation, with a typed cause.
    ///
    /// `Display` and every [`Diagnostic`] facet are deliberately identical to
    /// [`ClaudineError::ConfigValidation`]'s — `config_validation_twins_agree`
    /// locks that. The variants are separate only because `ConfigValidation` is
    /// a tuple newtype with nowhere to put a `#[source]`, and widening it would
    /// churn its ~50 causeless call sites for no gain.
    #[error("config validation error: {message}")]
    ConfigValidationWithCause {
        /// The same prose [`ClaudineError::ConfigValidation`] would carry.
        message: String,
        /// The typed failure that prose describes.
        #[source]
        source: ConfigCause,
    },

    /// Requested provider is not available or not detected.
    #[error("provider not available: {0}")]
    ProviderNotAvailable(String),

    /// Error during template interpolation.
    #[error("template error: {0}")]
    TemplateError(String),

    /// Error during template interpolation, with a typed cause.
    ///
    /// Twin of [`ClaudineError::TemplateError`] for the same reason
    /// [`ClaudineError::ConfigValidationWithCause`] twins `ConfigValidation`;
    /// `template_error_twins_agree` locks the two in step.
    ///
    /// It therefore inherits `internal.bug`, which is wrong for its only
    /// caller — a user's malformed mapper regex is not a Claudine bug. The
    /// correct code is `config.invalid`, as [`ClaudineError::RegexError`] and
    /// [`ClaudineError::ProtectRuleParse`] already use. Fixing it here would
    /// change `err.code` mid-migration, which spec §D10 forbids.
    #[error("template error: {message}")]
    TemplateErrorWithCause {
        /// The same prose [`ClaudineError::TemplateError`] would carry.
        message: String,
        /// The typed failure that prose describes.
        #[source]
        source: regex::Error,
    },

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
        /// The parser failure `message` describes.
        ///
        /// `None` where the variant reports a layer payload *type* mismatch
        /// rather than a parse — an internal invariant break that no parser
        /// produced, so there is genuinely nothing to retain.
        #[source]
        source: Option<PolicyParseCause>,
    },

    /// Policy engine CLI override parse failure.
    #[error("policy engine CLI parse for {provider}: {message}")]
    PolicyCliParse {
        /// Provider.
        provider: crate::provider::Provider,
        /// Parse error message.
        message: String,
        /// The parser failure `message` describes, when one occurred.
        ///
        /// `None` for the provider-mismatch and payload-type arms, which no
        /// parser produced.
        #[source]
        source: Option<PolicyParseCause>,
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
        /// The write failure `message` describes.
        ///
        /// `None` where the plan was rejected before any write was attempted.
        #[source]
        source: Option<std::io::Error>,
    },

    /// Policy engine context is ambiguous.
    #[error("policy engine ambiguous context: {0}")]
    PolicyAmbiguousContext(String),

    // --- System prompt errors ---
    /// Launch context detection failed (git or repo detection error).
    ///
    /// Carries the typed `sniff` failure as its `#[source]` rather than a
    /// flattened message, so a caller can recover which probe failed.
    #[error("launch context detection failed: {0}")]
    LaunchContextDetection(#[source] Arc<sniff::SniffError>),

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
    fn status_block(&self, term: &Terminal) -> StatusBlock {
        // Semantic over a typed Darkmatter cause: this layer owns
        // `composition.failed` because a Darkmatter error carries no facets,
        // but the cause holds the path, line, and transclusion chain, so the
        // block is built from it rather than flattened to one line here.
        if let ClaudineError::SystemPromptComposition(md) = self {
            return md.status_block(term);
        }
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
            | ClaudineError::ConfigValidationWithCause { .. }
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
            | ClaudineError::TemplateErrorWithCause { .. }
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
                base["path"] = json!(biscuit_file::to_portable_string(path));
            }
            // `io.network` declares `url`, `message`. The typed source carries
            // no parsed URL to surface, so only `message` is populated.
            ClaudineError::HttpError(_) | ClaudineError::UrlError(_) => {
                base["message"] = json!(self.to_string());
            }
            // `config.invalid` declares `field`, `message`.
            ClaudineError::ConfigValidation(message)
            | ClaudineError::ConfigValidationWithCause { message, .. }
            | ClaudineError::ProtectInvalidPolicy(message)
            | ClaudineError::ProtectEnforcementMapping(message)
            | ClaudineError::PolicySourceDiscovery(message)
            | ClaudineError::PolicyAmbiguousContext(message) => {
                base["message"] = json!(message);
            }
            ClaudineError::PolicyApplyFailed { path, message, .. } => {
                base["field"] = json!(biscuit_file::to_portable_string(path));
                base["message"] = json!(message);
            }
            ClaudineError::PolicyNativeParse {
                source_id, message, ..
            } => {
                base["field"] = json!(source_id);
                base["message"] = json!(message);
            }
            ClaudineError::PolicyCliParse {
                provider, message, ..
            } => {
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
            | ClaudineError::TemplateErrorWithCause { message, .. }
            | ClaudineError::ReportingPathUnavailable(message) => {
                base["message"] = json!(message);
            }
            ClaudineError::LaunchContextDetection(source) => {
                base["message"] = json!(source.to_string());
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
mod source_chain_tests {
    use std::error::Error;

    use super::*;

    fn json_error() -> serde_json::Error {
        serde_json::from_str::<Value>("{oops").unwrap_err()
    }

    /// Built through a binding rather than a literal: `clippy::invalid_regex`
    /// const-evaluates a literal argument and denies the malformed pattern this
    /// test needs.
    fn regex_error() -> regex::Error {
        let pattern = String::from("(unclosed");
        regex::Regex::new(&pattern).unwrap_err()
    }

    /// The whole point of the `#[source]` fields this module adds: the concrete
    /// parser error must be recoverable after the value is erased to
    /// `dyn Error`, not merely readable inside the flattened `message`.
    ///
    /// `PolicyParseCause` is `#[error(transparent)]`, like the in-repo
    /// `MarkdownLoadCause` it copies, so it *replaces* the parser error in the
    /// chain rather than adding a link above it — `serde_json::Error` is not
    /// separately reachable via a second `source()` hop. Downcasting to the
    /// cause enum and matching its arm is the recovery path.
    #[test]
    fn policy_native_parse_publishes_its_parser_error_as_a_source() {
        let err = ClaudineError::PolicyNativeParse {
            source_id: "user".to_owned(),
            message: "boom".to_owned(),
            source: Some(PolicyParseCause::Json(json_error())),
        };

        let erased: &(dyn Error + 'static) = &err;
        let cause = erased.source().expect("a source is published");
        let recovered = cause
            .downcast_ref::<PolicyParseCause>()
            .unwrap_or_else(|| panic!("source is not the typed cause: {cause}"));

        let PolicyParseCause::Json(json) = recovered else {
            panic!("wrong arm: {recovered:?}");
        };
        // The concrete parser error, not a re-parse of the message.
        assert!(json.is_syntax(), "{json}");
    }

    #[test]
    fn policy_cli_parse_publishes_its_parser_error_as_a_source() {
        let err = ClaudineError::PolicyCliParse {
            provider: Provider::Claude,
            message: "boom".to_owned(),
            source: Some(PolicyParseCause::Json(json_error())),
        };
        assert!(
            (&err as &(dyn Error + 'static))
                .source()
                .and_then(|c| c.downcast_ref::<PolicyParseCause>())
                .is_some()
        );
    }

    #[test]
    fn policy_apply_failed_publishes_its_io_error_as_a_source() {
        let err = ClaudineError::PolicyApplyFailed {
            path: PathBuf::from("/x/settings.json"),
            message: "denied".to_owned(),
            source: Some(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "denied",
            )),
        };

        let cause = (&err as &(dyn Error + 'static))
            .source()
            .expect("a source is published");
        let io = cause
            .downcast_ref::<std::io::Error>()
            .expect("source is the concrete `io::Error`");
        assert_eq!(io.kind(), std::io::ErrorKind::PermissionDenied);
    }

    /// The causeless arms are honest, not lazy: a layer payload *type* mismatch
    /// is an internal invariant break that no parser produced.
    #[test]
    fn a_causeless_policy_parse_publishes_no_source() {
        let err = ClaudineError::PolicyNativeParse {
            source_id: "user".to_owned(),
            message: "Claude layer payload type mismatch".to_owned(),
            source: None,
        };
        assert!((&err as &(dyn Error + 'static)).source().is_none());
    }

    #[test]
    fn config_validation_with_cause_publishes_its_cause() {
        for cause in [
            ConfigCause::Read(std::io::Error::other("read")),
            ConfigCause::Json(json_error()),
        ] {
            let err = ClaudineError::ConfigValidationWithCause {
                message: "boom".to_owned(),
                source: cause,
            };
            assert!(
                (&err as &(dyn Error + 'static))
                    .source()
                    .and_then(|c| c.downcast_ref::<ConfigCause>())
                    .is_some()
            );
        }
    }

    #[test]
    fn template_error_with_cause_publishes_its_regex_error() {
        let err = ClaudineError::TemplateErrorWithCause {
            message: "boom".to_owned(),
            source: regex_error(),
        };
        assert!(
            (&err as &(dyn Error + 'static))
                .source()
                .and_then(|c| c.downcast_ref::<regex::Error>())
                .is_some()
        );
    }

    /// `ConfigValidationWithCause` exists only to hold a `#[source]` beside the
    /// prose `ConfigValidation` already carried. It duplicates the `Display`
    /// prefix to do that, so this locks the two against drift: every observable
    /// projection must agree for the same message.
    #[test]
    fn config_validation_twins_agree() {
        let plain = ClaudineError::ConfigValidation("bad field".to_owned());
        let sourced = ClaudineError::ConfigValidationWithCause {
            message: "bad field".to_owned(),
            source: ConfigCause::Read(std::io::Error::other("x")),
        };

        assert_eq!(plain.to_string(), sourced.to_string());
        assert_eq!(plain.code(), sourced.code());
        assert_eq!(plain.category(), sourced.category());
        assert_eq!(plain.disposition(), sourced.disposition());
        assert_eq!(plain.origin(), sourced.origin());
        assert_eq!(plain.detail(), sourced.detail());
    }

    /// The same drift lock for the `TemplateError` twin.
    #[test]
    fn template_error_twins_agree() {
        let plain = ClaudineError::TemplateError("bad regex".to_owned());
        let sourced = ClaudineError::TemplateErrorWithCause {
            message: "bad regex".to_owned(),
            source: regex_error(),
        };

        assert_eq!(plain.to_string(), sourced.to_string());
        assert_eq!(plain.code(), sourced.code());
        assert_eq!(plain.category(), sourced.category());
        assert_eq!(plain.disposition(), sourced.disposition());
        assert_eq!(plain.origin(), sourced.origin());
        assert_eq!(plain.detail(), sourced.detail());
    }

    /// Adding a `#[source]` beside a prose field must not move the machine
    /// surface — spec §D10 permits richer detail, not renamed or re-valued
    /// detail. These are the exact projections the pre-migration variants made.
    #[test]
    fn adding_a_source_leaves_the_detail_projection_unmoved() {
        let native = ClaudineError::PolicyNativeParse {
            source_id: "user".to_owned(),
            message: "expected value".to_owned(),
            source: Some(PolicyParseCause::Json(json_error())),
        };
        assert_eq!(native.code(), "config.invalid");
        assert_eq!(native.detail()["field"], json!("user"));
        assert_eq!(native.detail()["message"], json!("expected value"));

        let apply = ClaudineError::PolicyApplyFailed {
            path: PathBuf::from("/x/settings.json"),
            message: "denied".to_owned(),
            source: Some(std::io::Error::other("denied")),
        };
        assert_eq!(apply.code(), "config.invalid");
        assert_eq!(apply.detail()["field"], json!("/x/settings.json"));
        assert_eq!(apply.detail()["message"], json!("denied"));
    }

    /// A registered code must never project a top-level `null` (spec §D7).
    #[test]
    fn every_new_variant_projects_a_catalog_shaped_detail() {
        for err in [
            ClaudineError::ConfigValidationWithCause {
                message: "x".to_owned(),
                source: ConfigCause::Json(json_error()),
            },
            ClaudineError::TemplateErrorWithCause {
                message: "x".to_owned(),
                source: regex_error(),
            },
            ClaudineError::PolicyNativeParse {
                source_id: "x".to_owned(),
                message: "x".to_owned(),
                source: Some(PolicyParseCause::Json(json_error())),
            },
            ClaudineError::PolicyCliParse {
                provider: Provider::Claude,
                message: "x".to_owned(),
                source: None,
            },
            ClaudineError::PolicyApplyFailed {
                path: PathBuf::from("x"),
                message: "x".to_owned(),
                source: Some(std::io::Error::other("x")),
            },
        ] {
            assert!(
                code_spec(err.code()).is_some(),
                "`{err:?}` → `{}` is not a locked catalog code",
                err.code()
            );
            assert!(
                err.detail().is_object(),
                "`{err:?}` projects a top-level non-object detail: {}",
                err.detail()
            );
        }
    }
}

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
