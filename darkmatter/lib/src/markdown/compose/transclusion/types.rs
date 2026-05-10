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
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum DeferredSetError {
    /// Object-form RHS did not parse as a JSON5 object.
    #[error("invalid set assignment at line {line}: {reason} (value: {raw})")]
    InvalidAssignment {
        /// Raw RHS as seen on the directive line (with any stripped quotes re-wrapped).
        raw: String,
        /// Human-readable reason (e.g., "expected JSON5 object").
        reason: String,
        /// 1-based directive line number.
        line: usize,
    },

    /// A property-form name (or `"<object>"`) was assigned more than once.
    #[error("reassigned set property '{name}'")]
    ReassignedProperty {
        /// The conflicting property name, or `"<object>"` for object-form.
        name: String,
    },
}

impl biscuit_terminal::errors::BlockError for DeferredSetError {
    fn status_block(
        &self,
        _term: &biscuit_terminal::terminal::Terminal,
    ) -> biscuit_terminal::components::status_block::StatusBlock {
        use biscuit_terminal::components::status::StatusState;
        use biscuit_terminal::components::status_block::StatusBlock;
        use biscuit_terminal::errors::{ErrorHeader, StatusBlockExt};

        match self {
            Self::InvalidAssignment { raw, reason, line } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new(
                    "DeferredSetError",
                    "invalid set assignment",
                ))
                .body(format!(
                    "<dim>Line:</dim> {line}\n<dim>Value:</dim> <cyan>{raw}</cyan>\n<dim>Reason:</dim> {reason}"
                ))
                .hint(
                    "Use a JSON5 object like <cyan>set={ key: \"value\" }</cyan> or a property form like <cyan>set.key=\"value\"</cyan>.",
                ),

            Self::ReassignedProperty { name } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new(
                    "DeferredSetError",
                    "reassigned set property",
                ))
                .body(format!("<dim>Property:</dim> <cyan>{name}</cyan>"))
                .hint(
                    "Each <cyan>set.NAME=</cyan> clause must appear only once per directive line.",
                ),
        }
    }
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
    /// Resolved file path (or empty for URLs/synthetic sources).
    pub path: PathBuf,
    /// 1-based line number where the transclusion directive appeared.
    pub line: usize,
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
    pub fn enter(
        &mut self,
        id: String,
        path: PathBuf,
        line: usize,
    ) -> Result<(), TransclusionError> {
        // Check local stack for cycle (provides chain for error message)
        if let Some(idx) = self.stack.iter().position(|n| n.id == id) {
            let mut chain: Vec<(PathBuf, usize)> = self.stack[idx..]
                .iter()
                .map(|n| (n.path.clone(), n.line))
                .collect();
            chain.push((path.clone(), line));
            return Err(TransclusionError::CycleDetected { chain });
        }

        self.stack.push(DependencyNode { id, path, line });
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
    ParseDirective {
        line: usize,
        message: String,
        caret_col: Option<usize>,
    },

    #[error(
        "Invalid {} reference '{reference}' in {} at line {line}",
        .directive_kind.as_str(),
        .source_file.display()
    )]
    InvalidReference {
        reference: String,
        line: usize,
        source_file: PathBuf,
        directive_kind: DirectiveKind,
    },

    #[error("Missing source context to resolve '{reference}' at line {line}")]
    MissingSourceContext { reference: String, line: usize },

    #[error("Unsupported reference type: {reference}")]
    UnsupportedReferenceType { reference: String },

    #[error("Unsupported file type for transclusion: {path}")]
    UnsupportedFileType { path: PathBuf },

    #[error("Code transclusion source is not UTF-8 text: {path}")]
    NonTextCodeSource { path: PathBuf },

    #[error("Transclusion cycle detected: {}", .chain.iter().map(|(p, l)| format!("{}:{}", p.display(), l)).collect::<Vec<_>>().join(" -> "))]
    CycleDetected { chain: Vec<(PathBuf, usize)> },

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

impl biscuit_terminal::errors::BlockError for TransclusionError {
    fn status_block(
        &self,
        _term: &biscuit_terminal::terminal::Terminal,
    ) -> biscuit_terminal::components::status_block::StatusBlock {
        use biscuit_terminal::components::status::StatusState;
        use biscuit_terminal::components::status_block::StatusBlock;
        use biscuit_terminal::errors::{ErrorHeader, StatusBlockExt};

        match self {
            TransclusionError::ParseDirective { line, message, .. } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("TransclusionError", "directive parse failed"))
                .body(format!("<dim>Line:</dim> {line}\n<dim>Message:</dim> {message}"))
                .hint("Expected syntax: <cyan>::file ./path.md</cyan> | <cyan>::code ./file.rs</cyan>."),

            TransclusionError::InvalidReference { reference, line, source_file, directive_kind } => {
                let kind_str = match directive_kind {
                    DirectiveKind::File => "::file",
                    DirectiveKind::Code => "::code",
                    DirectiveKind::Url => "::url",
                };
                StatusBlock::new(StatusState::Error)
                    .error_header(ErrorHeader::new("TransclusionError", "invalid reference"))
                    .body(format!(
                        "<dim>Source file:</dim> <cyan>{}</cyan>\n<dim>Line:</dim> {line}\n<dim>Directive:</dim> {kind_str}\n<dim>Reference:</dim> <cyan>{reference}</cyan>",
                        source_file.display()
                    ))
                    .hint("Use a relative path, absolute path, or <cyan>https://</cyan> URL.")
            }

            TransclusionError::MissingSourceContext { reference, line } => {
                StatusBlock::new(StatusState::Error)
                    .error_header(ErrorHeader::new(
                        "TransclusionError",
                        "missing source context",
                    ))
                    .body(format!(
                        "<dim>Reference:</dim> <cyan>{reference}</cyan>\n<dim>Line:</dim> {line}"
                    ))
                    .hint("Load the document from a file or URL so relative refs can resolve.")
            }

            TransclusionError::UnsupportedReferenceType { reference } => {
                StatusBlock::new(StatusState::Error)
                    .error_header(ErrorHeader::new(
                        "TransclusionError",
                        "unsupported reference type",
                    ))
                    .body(format!("<dim>Reference:</dim> <cyan>{reference}</cyan>"))
                    .hint("Supported types: local file paths, HTTP(S) URLs.")
            }

            TransclusionError::UnsupportedFileType { path } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new(
                    "TransclusionError",
                    "unsupported file type",
                ))
                .body(format!("<dim>Path:</dim> <cyan>{}</cyan>", path.display()))
                .hint("Use <cyan>::file</cyan> for markdown or <cyan>::code</cyan> for text/source."),

            TransclusionError::NonTextCodeSource { path } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("TransclusionError", "non-text code source"))
                .body(format!("<dim>Path:</dim> <cyan>{}</cyan>", path.display()))
                .hint("Only UTF-8 text files can be transcluded with <cyan>::code</cyan>."),

            TransclusionError::CycleDetected { chain } => {
                let chain_display = chain
                    .iter()
                    .enumerate()
                    .map(|(i, (path, line))| {
                        format!(
                            "  {}. <cyan>{}</cyan> <dim>:line {}</dim>",
                            i + 1,
                            path.display(),
                            line
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                StatusBlock::new(StatusState::Error)
                    .error_header(ErrorHeader::new("TransclusionError", "cycle detected"))
                    .body(format!("<dim>Chain:</dim>\n{chain_display}"))
                    .hint("Break the cycle by removing one transclusion edge.")
            }

            TransclusionError::MaxDepthExceeded { max_depth } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new(
                    "TransclusionError",
                    "max recursion depth exceeded",
                ))
                .body(format!("<dim>Max depth:</dim> {max_depth}"))
                .hint("Flatten the transclusion tree or raise the configured max depth."),

            TransclusionError::ConditionEval { expr, line, message } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new(
                    "TransclusionError",
                    "condition evaluation failed",
                ))
                .body(format!(
                    "<dim>Expression:</dim> <cyan>{expr}</cyan>\n<dim>Line:</dim> {line}\n<dim>Message:</dim> {message}"
                ))
                .hint("Check that every variable referenced is present in the effective state."),

            TransclusionError::ConditionParse { expr, line, message } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new(
                    "TransclusionError",
                    "condition parse failed",
                ))
                .body(format!(
                    "<dim>Expression:</dim> <cyan>{expr}</cyan>\n<dim>Line:</dim> {line}\n<dim>Message:</dim> {message}"
                ))
                .hint("Operators: <cyan>&&  ||  !  ==  !=  >  >=  <  <=  +  -  *  /  %  []  .</cyan> plus helpers like <cyan>HasKey</cyan>, <cyan>Contains</cyan>, <cyan>Length</cyan>, <cyan>min/max/abs</cyan>, <cyan>first/last</cyan>, <cyan>IsString/IsNumber/IsArray/IsNull/IsObject/IsEmpty</cyan>, <cyan>StartsWith/EndsWith</cyan>, <cyan>Lower/Upper/Capitalize</cyan>, case helpers, and date validators (<cyan>IsDate</cyan>, <cyan>IsToday</cyan>, etc.)."),

            TransclusionError::Relevel(message) => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("TransclusionError", "re-leveling failed"))
                .body(message.clone())
                .hint("Pick a target heading level that fits within H1–H6."),

            TransclusionError::UrlExecutionDisabled { url } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new(
                    "TransclusionError",
                    "remote transclusion disabled",
                ))
                .body(format!("<dim>URL:</dim> <cyan>{url}</cyan>"))
                .hint("Enable URL transclusion in the compose options to allow this request."),

            TransclusionError::InvalidFrontmatterAssignment { line, raw, reason } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new(
                    "TransclusionError",
                    "invalid frontmatter assignment",
                ))
                .body(format!(
                    "<dim>Line:</dim> {line}\n<dim>Value:</dim> <cyan>{raw}</cyan>\n<dim>Reason:</dim> {reason}"
                ))
                .hint("Run with <cyan>--allow-invalid-frontmatter-assignment</cyan> to downgrade to a warning."),

            TransclusionError::InvalidReassignedFrontmatterProperty { line, name } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new(
                    "TransclusionError",
                    "reassigned frontmatter property",
                ))
                .body(format!(
                    "<dim>Line:</dim> {line}\n<dim>Property:</dim> <cyan>{name}</cyan>"
                ))
                .hint("Run with <cyan>--allow-reassigned-frontmatter-property</cyan> to downgrade to a warning."),

            TransclusionError::Io(source) => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("TransclusionError", "I/O error"))
                .body(format!("<dim>Kind:</dim> {:?}\n{source}", source.kind()))
                .hint("Confirm the referenced file exists and is readable."),

            TransclusionError::UrlParse(source) => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("TransclusionError", "URL parse error"))
                .body(format!("{source}"))
                .hint("Use a fully qualified URL including the <cyan>https://</cyan> scheme."),

            TransclusionError::FileReference(source) => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new(
                    "TransclusionError",
                    "file reference error",
                ))
                .body(format!("{source}"))
                .hint("Verify the reference string and surrounding compose source."),

            TransclusionError::Json(source) => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new(
                    "TransclusionError",
                    "directive option parse error",
                ))
                .body(format!(
                    "{source}\n<dim>Position:</dim> line {}, column {}",
                    source.line(),
                    source.column()
                ))
                .hint("Directive options use JSON5; check braces, quoting, and commas."),
        }
    }
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
