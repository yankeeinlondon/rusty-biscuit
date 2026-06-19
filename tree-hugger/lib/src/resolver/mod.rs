use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::shared::{DiagnosticConfidence, ProgrammingLanguage};

/// Project-level context required for semantic resolution.
///
/// Resolvers use this context to understand module boundaries,
/// dependencies, and generated code conventions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectContext {
    /// Absolute path to the project root directory.
    pub root_path: PathBuf,
    /// Detected package manifest files (e.g. Cargo.toml, package.json).
    pub manifests: Vec<PathBuf>,
    /// Language-specific configuration files (e.g. tsconfig.json, pyproject.toml).
    pub language_configs: Vec<PathBuf>,
    /// Hints about external dependencies (crate names, npm packages, etc.).
    pub dependency_hints: Vec<String>,
    /// File patterns that mark generated or vendored code.
    pub generated_file_markers: Vec<String>,
    /// Target environment or runtime (e.g. "node", "browser", "deno").
    pub target_environment: Option<String>,
}

impl ProjectContext {
    /// Creates a minimal project context from a root path.
    pub fn new(root_path: impl AsRef<Path>) -> Self {
        Self {
            root_path: root_path.as_ref().to_path_buf(),
            manifests: Vec::new(),
            language_configs: Vec::new(),
            dependency_hints: Vec::new(),
            generated_file_markers: Vec::new(),
            target_environment: None,
        }
    }
}

/// Metadata describing resolver capabilities and confidence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolverMetadata {
    /// Unique identifier for the resolver (e.g. "rust-cargo", "js-oxlint").
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Semantic version of the resolver.
    pub version: String,
    /// Languages this resolver supports.
    pub supported_languages: Vec<ProgrammingLanguage>,
    /// Highest confidence this resolver can provide.
    pub max_confidence: DiagnosticConfidence,
    /// Scope of analysis this resolver performs.
    pub scope: ResolverScope,
    /// Whether the resolver requires project context to function.
    pub needs_project_context: bool,
}

/// Scope of analysis a resolver can perform.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResolverScope {
    /// File-local only (same as current semantic checks).
    FileLocal,
    /// Module-level (understands imports/exports within a package).
    Module,
    /// Project-level (cross-package, understands dependency graph).
    Project,
    /// Workspace-level (monorepo-aware).
    Workspace,
}

/// Trait for language-specific semantic resolvers.
///
/// Resolvers enrich symbol records with cross-file and project-level
/// understanding. They are gated behind explicit enablement until corpus
/// precision is measured.
pub trait SemanticResolver: Send + Sync {
    /// Returns metadata describing this resolver's capabilities.
    fn metadata(&self) -> &ResolverMetadata;

    /// Checks whether this resolver is available for the given context.
    fn is_available(&self, context: &ProjectContext) -> bool;

    /// Resolves symbols and references with project context.
    ///
    /// Returns resolved references and any diagnostics produced during
    /// resolution. Diagnostics are gated until proven precise.
    fn resolve(
        &self,
        context: &ProjectContext,
        file_path: &Path,
        language: ProgrammingLanguage,
    ) -> Result<ResolverOutput, ResolverError>;
}

/// Output from a semantic resolver.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResolverOutput {
    /// Diagnostics produced by the resolver.
    pub diagnostics: Vec<crate::shared::Diagnostic>,
    /// Resolved import mappings.
    pub resolved_imports: Vec<ResolvedImport>,
    /// Symbols that could not be resolved (with confidence).
    pub unresolved_symbols: Vec<UnresolvedSymbol>,
}

/// A resolved import with its target information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedImport {
    /// The imported symbol name.
    pub name: String,
    /// The source module or package.
    pub source: String,
    /// Whether the import was successfully resolved.
    pub is_resolved: bool,
    /// Confidence in this resolution.
    pub confidence: DiagnosticConfidence,
}

/// An unresolved symbol with context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnresolvedSymbol {
    /// The symbol name.
    pub name: String,
    /// Source location.
    pub location: crate::shared::CodeRange,
    /// Confidence that this is truly undefined.
    pub confidence: DiagnosticConfidence,
    /// Reason for the confidence level.
    pub reason: String,
}

/// Errors that can occur during semantic resolution.
#[derive(Debug, Clone, thiserror::Error)]
pub enum ResolverError {
    #[error("Resolver {resolver_id} is not available for {language}")]
    NotAvailable {
        resolver_id: String,
        language: ProgrammingLanguage,
    },

    #[error("Project context required but not provided")]
    MissingProjectContext,

    #[error("Resolution failed: {0}")]
    ResolutionFailed(String),

    #[error("Resolver version incompatible: expected {expected}, found {found}")]
    VersionIncompatible { expected: String, found: String },
}
