//! Type definitions for the transclusion phase.

use crate::markdown::compose::ComposeSource;
use std::ops::Range;
use std::path::PathBuf;
use thiserror::Error;

/// Supported directive kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DirectiveKind {
    /// `::file` markdown transclusion.
    File,
    /// `::code` source-code transclusion.
    Code,
    /// `::url` remote transclusion.
    Url,
}

impl DirectiveKind {
    /// Returns the canonical directive keyword.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::File => "file",
            Self::Code => "code",
            Self::Url => "url",
        }
    }
}

/// Replace option behavior for a directive.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum ReplaceOption {
    /// Default inheritance semantics.
    #[default]
    InheritDefault,
    /// Parent replace map takes precedence over child replace map.
    ParentWins,
    /// A one-off replace map for this transclusion only.
    OneOff(serde_json::Map<String, serde_json::Value>),
}

/// Parsed options for a block directive.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BlockOptions {
    /// Replace behavior override.
    pub replace: ReplaceOption,

    /// Quotation wrapper.
    ///
    /// `None` disables quotation. `Some("")` means `quotation=true`.
    pub quotation: Option<String>,

    /// Disclosure wrapper summary text.
    pub disclosure: Option<String>,

    /// Optional `when` expression.
    pub when_expr: Option<String>,

    /// Heading sections to exclude from transcluded content.
    pub exclude: Vec<String>,

    /// Unknown options captured for warning reporting.
    pub unknown_options: Vec<String>,

    /// Object-form `set=<json5-object>` payload, if present.
    ///
    /// Applied as the middle layer of the three-layer frontmatter
    /// override merge on top of the child's authored frontmatter.
    pub set_object: Option<serde_json::Map<String, serde_json::Value>>,

    /// Property-form `set.NAME=<json5-value>` payloads, in directive order.
    ///
    /// Ordering is preserved so deterministic last-write tracing
    /// is possible under permissive duplicate-handling mode.
    pub set_properties: Vec<(String, serde_json::Value)>,

    /// Deferred grammar errors for `set=`/`set.NAME=` clauses.
    ///
    /// The parser records invalid or reassigned set clauses here rather
    /// than raising them inline. The engine stage, which has
    /// `ComposeOptions` in scope, then decides whether each deferred
    /// error is fatal (strict, default) or downgraded to a warning
    /// (permissive flags). Sibling clauses that are independently valid
    /// remain in `set_object` / `set_properties` so they can still apply
    /// under permissive mode.
    pub deferred_set_errors: Vec<DeferredSetError>,
}

/// A parser-recorded set-override error whose severity is resolved later.
///
/// Captured by the directive parser when a `set=<value>` RHS is not
/// a JSON5 object or when the same `set.NAME=` appears twice on one
/// directive line. Resolution into a hard error or a warning happens
/// during transclusion when the governing `ComposeOptions` are in scope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeferredSetError {
    /// Object-form RHS did not parse as a JSON5 object.
    InvalidAssignment {
        /// Raw RHS as seen on the directive line (with any stripped quotes re-wrapped).
        raw: String,
        /// Human-readable reason (e.g., "expected JSON5 object").
        reason: String,
    },

    /// A property-form name (or `"<object>"`) was assigned more than once.
    ReassignedProperty {
        /// The conflicting property name, or `"<object>"` for object-form.
        name: String,
    },
}

/// Parsed directive from markdown body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockDirective {
    /// Directive kind.
    pub kind: DirectiveKind,

    /// Raw target token.
    pub raw_target: String,

    /// Parsed options.
    pub options: BlockOptions,

    /// Replacement span in source content.
    pub span: Range<usize>,

    /// 1-based line number of directive.
    pub line: usize,
}

/// Runtime dependency node for cycle detection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependencyNode {
    /// Canonical identifier (path or URL).
    pub id: String,
}

/// Runtime state for recursive transclusion.
///
/// Tracks recursion depth and cycle detection using per-branch ancestry.
/// Sibling branches may legitimately depend on the same target, so
/// only repetition within the current branch stack is treated as a cycle.
#[derive(Debug, Clone)]
pub struct TransclusionRuntime {
    /// Per-branch call stack for depth tracking.
    stack: Vec<DependencyNode>,

    /// Maximum recursion depth allowed.
    pub max_depth: usize,

    /// Deepest depth observed across all branches.
    pub deepest_seen: usize,
}

impl TransclusionRuntime {
    /// Creates a new runtime with the given depth limit.
    pub fn new(max_depth: usize) -> Self {
        Self {
            stack: Vec::new(),
            max_depth,
            deepest_seen: 0,
        }
    }

    /// Enters a dependency node and validates cycle/depth constraints.
    pub fn enter(&mut self, id: String) -> Result<(), TransclusionError> {
        // Check local stack for cycle (provides chain for error message)
        if let Some(idx) = self.stack.iter().position(|n| n.id == id) {
            let mut chain: Vec<String> = self.stack[idx..].iter().map(|n| n.id.clone()).collect();
            chain.push(id);
            return Err(TransclusionError::CycleDetected { chain });
        }

        self.stack.push(DependencyNode { id });
        self.deepest_seen = self.deepest_seen.max(self.stack.len());

        if self.stack.len() > self.max_depth {
            return Err(TransclusionError::MaxDepthExceeded {
                max_depth: self.max_depth,
            });
        }

        Ok(())
    }

    /// Exits the current dependency node.
    pub fn exit(&mut self) {
        self.stack.pop();
    }

    /// Current recursion depth.
    pub fn depth(&self) -> usize {
        self.stack.len()
    }

    /// Creates a child runtime with the same ancestry stack.
    pub fn clone_for_child(&self) -> Self {
        Self {
            stack: self.stack.clone(),
            max_depth: self.max_depth,
            deepest_seen: self.deepest_seen,
        }
    }

    /// Merges a child's stats back into this runtime.
    pub fn merge_child(&mut self, child: &Self) {
        self.deepest_seen = self.deepest_seen.max(child.deepest_seen);
    }
}

/// Resolved transclusion target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolvedTarget {
    /// Local filesystem path.
    File { path: PathBuf, id: String },
    /// Remote URL target.
    Url { url: url::Url, id: String },
}

impl ResolvedTarget {
    /// Returns the stable dependency ID for cycle detection.
    pub fn id(&self) -> &str {
        match self {
            Self::File { id, .. } => id,
            Self::Url { id, .. } => id,
        }
    }
}

/// Parsed frontmatter transclusion references.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FrontmatterRefs {
    /// Prologue references to prepend.
    pub prologue: Vec<String>,
    /// Epilogue references to append.
    pub epilogue: Vec<String>,
}

/// Errors produced by transclusion.
#[derive(Debug, Error)]
pub enum TransclusionError {
    #[error("Failed to parse directive at line {line}: {message}")]
    ParseDirective { line: usize, message: String },

    #[error("Invalid reference '{reference}' at line {line}")]
    InvalidReference { reference: String, line: usize },

    #[error("Missing source context to resolve '{reference}' at line {line}")]
    MissingSourceContext { reference: String, line: usize },

    #[error("Unsupported reference type: {reference}")]
    UnsupportedReferenceType { reference: String },

    #[error("Unsupported file type for transclusion: {path}")]
    UnsupportedFileType { path: PathBuf },

    #[error("Code transclusion source is not UTF-8 text: {path}")]
    NonTextCodeSource { path: PathBuf },

    #[error("Transclusion cycle detected: {chain:?}")]
    CycleDetected { chain: Vec<String> },

    #[error("Maximum transclusion depth exceeded (max: {max_depth})")]
    MaxDepthExceeded { max_depth: usize },

    #[error("Failed to evaluate condition '{expr}' at line {line}: {message}")]
    ConditionEval {
        expr: String,
        line: usize,
        message: String,
    },

    #[error("Failed to parse condition '{expr}' at line {line}: {message}")]
    ConditionParse {
        expr: String,
        line: usize,
        message: String,
    },

    #[error("Heading re-leveling failed: {0}")]
    Relevel(String),

    #[error("Remote URL transclusion is disabled: {url}")]
    UrlExecutionDisabled { url: String },

    #[error(
        "Invalid frontmatter assignment on '::file' directive at line {line}: {reason} (value: {raw})"
    )]
    InvalidFrontmatterAssignment {
        line: usize,
        raw: String,
        reason: String,
    },

    #[error(
        "Invalid reassigned frontmatter property '{name}' on '::file' directive at line {line}"
    )]
    InvalidReassignedFrontmatterProperty { line: usize, name: String },

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("URL parse error: {0}")]
    UrlParse(#[from] url::ParseError),

    #[error("File reference error: {0}")]
    FileReference(#[from] biscuit_file::FileReferenceError),

    #[error("JSON parse error in transclusion option: {0}")]
    Json(#[from] serde_json::Error),
}

/// Source info derived from a compose source.
#[derive(Debug, Clone, Default)]
pub struct SourceContext {
    /// Current local file, if known.
    pub file: Option<PathBuf>,
    /// Current URL source, if known.
    pub url: Option<url::Url>,
}

impl SourceContext {
    /// Builds source context from a compose source.
    pub fn from_source(source: &ComposeSource) -> Self {
        match source {
            ComposeSource::Unknown => Self::default(),
            ComposeSource::File(path) => Self {
                file: Some(path.clone()),
                url: None,
            },
            ComposeSource::Url(url) => Self {
                file: None,
                url: Some(url.clone()),
            },
        }
    }
}
