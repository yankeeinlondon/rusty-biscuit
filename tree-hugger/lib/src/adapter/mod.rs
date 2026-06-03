use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::shared::{
    Diagnostic, DiagnosticCategory, DiagnosticConfidence, DiagnosticMetadata, DiagnosticSeverity,
    DiagnosticSource, ProgrammingLanguage,
};

pub mod oxlint;
pub use oxlint::OxlintAdapter;

/// Configuration for an external diagnostic adapter.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AdapterConfig {
    /// Explicit path to the external tool binary.
    pub tool_path: Option<PathBuf>,
    /// Explicit path to the tool's configuration file.
    pub config_path: Option<PathBuf>,
    /// Additional arguments to pass to the tool.
    pub extra_args: Vec<String>,
    /// Environment variables to set when invoking the tool.
    pub env: HashMap<String, String>,
    /// Whether to treat missing tools as fatal errors.
    pub strict: bool,
    /// Timeout for tool invocation.
    pub timeout: Option<Duration>,
}

/// Result of running an external diagnostic adapter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterResult {
    /// Normalized diagnostics from the external tool.
    pub diagnostics: Vec<Diagnostic>,
    /// Metadata about the adapter run.
    pub metadata: AdapterMetadata,
    /// Whether the adapter was able to run successfully.
    pub success: bool,
    /// Error message if the adapter failed but was not strict.
    pub error_message: Option<String>,
}

/// Metadata describing an adapter run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterMetadata {
    /// Name of the external tool (e.g. "oxlint").
    pub tool_name: String,
    /// Version of the external tool.
    pub version: Option<String>,
    /// Configuration files used by the tool.
    pub config_files: Vec<PathBuf>,
    /// Working directory for the tool invocation.
    pub working_directory: PathBuf,
    /// Exit status of the tool process.
    pub exit_status: Option<i32>,
    /// Elapsed time for the tool invocation.
    pub elapsed_time_ms: u64,
    /// Cache status if caching was used.
    pub cache_status: AdapterCacheStatus,
    /// Whether the tool is available.
    pub tool_available: bool,
    /// Whether fixes are available for any diagnostic.
    pub fixes_available: bool,
}

impl Default for AdapterMetadata {
    fn default() -> Self {
        Self {
            tool_name: String::new(),
            version: None,
            config_files: Vec::new(),
            working_directory: PathBuf::from("."),
            exit_status: None,
            elapsed_time_ms: 0,
            cache_status: AdapterCacheStatus::NotUsed,
            tool_available: false,
            fixes_available: false,
        }
    }
}

/// Cache status for adapter results.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AdapterCacheStatus {
    /// Result was not cached.
    NotUsed,
    /// Result was read from cache.
    Hit,
    /// Result was computed and written to cache.
    Miss,
    /// Cache entry was stale and recomputed.
    Stale,
}

/// Trait for external diagnostic adapters.
///
/// Adapters delegate linting to external tools and normalize their output
/// into Tree Hugger diagnostics.
pub trait ExternalDiagnosticAdapter: Send + Sync {
    /// Returns the name of this adapter.
    fn name(&self) -> &str;

    /// Returns the version of the underlying tool, if known.
    fn version(&self) -> Option<String>;

    /// Checks whether the external tool is available.
    fn is_available(&self) -> bool;

    /// Returns the languages this adapter supports.
    fn supported_languages(&self) -> Vec<ProgrammingLanguage>;

    /// Runs the adapter on the given files.
    ///
    /// ## Arguments
    /// - `files`: Resolved paths to source files.
    /// - `project_root`: Root directory of the project.
    /// - `language`: Language of the files.
    /// - `config`: Adapter-specific configuration.
    ///
    /// ## Returns
    /// Normalized diagnostics and adapter metadata.
    fn run(
        &self,
        files: &[PathBuf],
        project_root: &Path,
        language: ProgrammingLanguage,
        config: &AdapterConfig,
    ) -> Result<AdapterResult, AdapterError>;
}

/// Errors that can occur when running an external adapter.
#[derive(Debug, Clone, thiserror::Error)]
pub enum AdapterError {
    #[error("Tool '{tool}' not found")]
    ToolNotFound { tool: String },

    #[error("Tool '{tool}' version {found} is incompatible (expected {expected})")]
    IncompatibleVersion {
        tool: String,
        expected: String,
        found: String,
    },

    #[error("Failed to run tool '{tool}': {message}")]
    ExecutionFailed { tool: String, message: String },

    #[error("Failed to parse tool output: {message}")]
    ParseFailed { message: String },

    #[error("Unsupported language {language} for adapter {adapter}")]
    UnsupportedLanguage {
        language: ProgrammingLanguage,
        adapter: String,
    },

    #[error("Configuration error: {message}")]
    ConfigError { message: String },
}

/// Creates a diagnostic from an external tool's finding.
///
/// Preserves the original rule ID in the diagnostic metadata.
pub fn external_diagnostic(
    message: String,
    range: crate::shared::CodeRange,
    severity: DiagnosticSeverity,
    rule_id: String,
    category: DiagnosticCategory,
    confidence: DiagnosticConfidence,
    _tool_name: String,
) -> Diagnostic {
    Diagnostic {
        kind: crate::shared::DiagnosticKind::Lint,
        message,
        range,
        severity,
        rule: Some(rule_id),
        context: None,
        metadata: Some(DiagnosticMetadata {
            category,
            confidence,
            source: DiagnosticSource::ExternalTool,
            default_severity: severity,
            effective_severity: severity,
            is_enabled_by_default: true,
            requires_experimental_semantics: false,
        }),
    }
}

/// Maps an external tool severity string to Tree Hugger severity.
pub fn map_severity(external: &str) -> DiagnosticSeverity {
    match external.to_ascii_lowercase().as_str() {
        "error" | "fatal" => DiagnosticSeverity::Error,
        "warning" | "warn" => DiagnosticSeverity::Warning,
        _ => DiagnosticSeverity::Info,
    }
}

/// Maps an external tool category string to Tree Hugger category.
pub fn map_category(external: &str) -> DiagnosticCategory {
    match external.to_ascii_lowercase().as_str() {
        "correctness" | "syntax" => DiagnosticCategory::Correctness,
        "suspicious" => DiagnosticCategory::Suspicious,
        "style" | "convention" => DiagnosticCategory::Style,
        "performance" | "perf" => DiagnosticCategory::Performance,
        "pedantic" => DiagnosticCategory::Pedantic,
        "restriction" => DiagnosticCategory::Restriction,
        _ => DiagnosticCategory::Suspicious,
    }
}
