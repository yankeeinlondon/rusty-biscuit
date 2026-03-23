//! Type definitions for Stage 2 transclusion.

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
/// Tracks recursion depth and cycle detection across potentially
/// concurrent transclusion resolution. The `active` set is shared
/// via `Arc<Mutex>` so that concurrent children can detect cycles
/// against siblings and ancestors.
#[derive(Debug, Clone)]
pub struct TransclusionRuntime {
    /// Per-branch call stack for depth tracking.
    stack: Vec<DependencyNode>,

    /// Maximum recursion depth allowed.
    pub max_depth: usize,

    /// Deepest depth observed across all branches.
    pub deepest_seen: usize,

    /// Active source IDs for cycle detection (shared across threads).
    active: std::sync::Arc<std::sync::Mutex<std::collections::HashSet<String>>>,
}

impl TransclusionRuntime {
    /// Creates a new runtime with the given depth limit.
    pub fn new(max_depth: usize) -> Self {
        Self {
            stack: Vec::new(),
            max_depth,
            deepest_seen: 0,
            active: std::sync::Arc::new(std::sync::Mutex::new(
                std::collections::HashSet::new(),
            )),
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

        // Check shared active set for cross-thread cycle detection
        {
            let active = self.active.lock().unwrap();
            if active.contains(&id) {
                return Err(TransclusionError::CycleDetected {
                    chain: vec![id],
                });
            }
        }

        // Register in both local stack and shared active set
        {
            let mut active = self.active.lock().unwrap();
            active.insert(id.clone());
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
        if let Some(node) = self.stack.pop() {
            let mut active = self.active.lock().unwrap();
            active.remove(&node.id);
        }
    }

    /// Current recursion depth.
    pub fn depth(&self) -> usize {
        self.stack.len()
    }

    /// Creates a child runtime sharing cycle detection but with an
    /// independent depth counter starting one level deeper.
    pub fn clone_for_child(&self) -> Self {
        Self {
            stack: self.stack.clone(),
            max_depth: self.max_depth,
            deepest_seen: self.deepest_seen,
            active: self.active.clone(),
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

/// Errors produced by Stage 2 transclusion.
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
