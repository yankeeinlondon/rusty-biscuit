//! Type definitions for the transclusion phase.

use crate::markdown::compose::ComposeSource;
use biscuit_terminal::errors::SourceContext;
use std::ops::Range;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Source info derived from a compose source.
#[derive(Debug, Clone, Default)]
pub struct TransclusionSource {
    /// Current local file, if known.
    pub file: Option<PathBuf>,
    /// Current URL source, if known.
    pub url: Option<url::Url>,
}

impl TransclusionSource {
    /// Builds transclusion source info from a compose source.
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
#[derive(Debug, Clone, PartialEq, Eq)]
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

    /// Origin of the `set` and one-off `replace` values: data when the
    /// directive's options hold text an earlier stage inserted (for example
    /// `set.note="{{ note }}"` after body interpolation), so the child never
    /// scans it again.
    pub values_origin: crate::markdown::compose::OverrideOrigin,

    /// The directive's local `interpolate=` value: `false` only when the
    /// author wrote a false boolean word on `::file` or `::url`. Absent,
    /// `null`, and true words all store `true`, so those spellings compare
    /// equal. A `false` here can only disable the child's prose
    /// interpolation; `true` never re-enables what an ancestor disabled.
    pub interpolate: bool,
}

// Hand-written because `interpolate` defaults to `true`: the prologue and
// epilogue children are built from `BlockOptions::default()` and must inherit
// the parent's setting rather than disable it.
impl Default for BlockOptions {
    fn default() -> Self {
        Self {
            replace: ReplaceOption::default(),
            quotation: None,
            disclosure: None,
            when_expr: None,
            exclude: Vec::new(),
            unknown_options: Vec::new(),
            set_object: None,
            set_properties: Vec::new(),
            deferred_set_errors: Vec::new(),
            values_origin: crate::markdown::compose::OverrideOrigin::default(),
            interpolate: true,
        }
    }
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
        use biscuit_terminal::components::prose::{LineBreaks, Prose};
        use biscuit_terminal::components::status::StatusState;
        use biscuit_terminal::components::status_block::StatusBlock;
        use biscuit_terminal::errors::{ErrorHeader, StatusBlockExt};

        match self {
            Self::InvalidAssignment { raw, reason, line } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new(
                    "DeferredSetError",
                    "invalid set assignment",
                ))
                .body(Prose::new(format!(
                    "<dim>Line:</dim> {line}\n<dim>Value:</dim> <cyan>{raw}</cyan>\n<dim>Reason:</dim> {reason}"
                )).with_line_breaks(LineBreaks::Hard))
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

        if self.stack.len() >= self.max_depth {
            // The refused node is never entered, so a caller that recovers
            // from the error has no `exit` to pair with it.
            self.deepest_seen = self.deepest_seen.max(self.stack.len() + 1);
            return Err(TransclusionError::MaxDepthExceeded {
                max_depth: self.max_depth,
            });
        }

        self.stack.push(DependencyNode { id, path, line });
        self.deepest_seen = self.deepest_seen.max(self.stack.len());

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

    /// Path of the root document for the current recursive compose run.
    pub(crate) fn root_path(&self) -> Option<&Path> {
        self.stack.first().map(|node| node.path.as_path())
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
    ///
    /// `path` is canonical and is the target's identity. `resolved` is the
    /// path the opening document's context resolved the reference to, before
    /// canonicalization; the target's own references resolve from it, so it
    /// keeps the spelling of the tree it was found in.
    File { path: PathBuf, id: String, resolved: PathBuf },
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

/// ` at line N`, or nothing for the unknown line `0`.
pub(crate) fn at_line(line: usize) -> String {
    if line == 0 { String::new() } else { format!(" at line {line}") }
}

/// Where an inclusion whose target failed was written: the file holding it
/// and, when proven, the authored position of its body directive's opener or
/// of its `prologue`/`epilogue` value (the list entry, for a list).
///
/// Carried by [`TransclusionError::Located`] so every reader of that file's
/// inclusion (composition, its pre-flight walk, the reference graph and
/// validation) reports the same place, a nested child's included.
#[derive(Debug, Clone, Eq)]
pub struct InclusionSite {
    file: PathBuf,
    authored: Option<crate::markdown::types::AuthoredSpan>,
    /// The frontmatter key that wrote the inclusion (`prologue:`), when it
    /// is one; a body directive names its own.
    construct: Option<&'static str>,
}

impl InclusionSite {
    /// The site of the body directive at `site` of `markdown`; `None` for a
    /// document with no file, whose body line is its only coordinate.
    pub(crate) fn of_directive(
        markdown: &crate::markdown::Markdown,
        site: &crate::markdown::compose::site_scope::DirectiveSite,
    ) -> Option<Self> {
        use crate::markdown::compose::site_scope::DirectiveSite;
        match site {
            DirectiveSite::Body => None,
            DirectiveSite::Authored(context, opener) => {
                Some(Self { file: context.display.clone(), authored: Some(opener.clone()), construct: None })
            }
            DirectiveSite::Unproven => file_of(markdown).map(|file| Self { file, authored: None, construct: None }),
        }
    }

    /// The site of `markdown`'s `key` value reading `value`, entry `index` of
    /// a list; the file alone when the value's position is not proven, and
    /// `None` for a document with no file.
    pub(crate) fn of_frontmatter_value(
        markdown: &crate::markdown::Markdown,
        key: &str,
        index: usize,
        value: &str,
    ) -> Option<Self> {
        let file = file_of(markdown)?;
        let in_list = markdown.frontmatter().as_map().get(key).is_some_and(serde_json::Value::is_array);
        let authored = markdown.loaded_source().and_then(|loaded| {
            let range = frontmatter_value_range(&loaded.text, key, in_list.then_some(index), value)?;
            crate::markdown::types::AuthoredSpan::locate(&loaded.text, range)
        });
        let construct = match key {
            "prologue" => Some("prologue:"),
            "epilogue" => Some("epilogue:"),
            _ => None,
        };
        Some(Self { file, authored, construct })
    }

    /// `file`, at `authored` when it is proven.
    pub fn new(file: PathBuf, authored: Option<crate::markdown::types::AuthoredSpan>) -> Self {
        Self { file, authored, construct: None }
    }

    /// The file that wrote the inclusion.
    pub fn file(&self) -> &Path {
        &self.file
    }

    /// The frontmatter construct that wrote the inclusion (`prologue:`,
    /// `epilogue:`); `None` for a body directive.
    pub(crate) fn construct(&self) -> Option<&'static str> {
        self.construct
    }

    /// The inclusion's authored position in [`file`](Self::file); `None` when
    /// it is not proven (a body the file no longer describes).
    pub fn authored(&self) -> Option<&crate::markdown::types::AuthoredSpan> {
        self.authored.as_ref()
    }
}

/// A site is its file and position; the construct only words it.
impl PartialEq for InclusionSite {
    fn eq(&self, other: &Self) -> bool {
        self.file == other.file && self.authored == other.authored
    }
}

impl std::fmt::Display for InclusionSite {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.authored {
            Some(at) => write!(f, "{}:{}:{}", self.file.display(), at.line(), at.column()),
            None => write!(f, "{}", self.file.display()),
        }
    }
}

/// `error`, raised opening the inclusion target `path`, at the inclusion
/// `site`: a file that cannot be read is the target's I/O failure (its code
/// stays `dm.input.read_failure`) and a target failure is placed there. A
/// child's own failure (its frontmatter, say) and any failure with no site
/// are unchanged.
pub(crate) fn target_failure_at(
    error: crate::markdown::MarkdownError,
    path: &Path,
    site: Option<InclusionSite>,
) -> crate::markdown::MarkdownError {
    use crate::markdown::MarkdownError;
    match error {
        MarkdownError::FileLoad(source) if site.is_some() => read_failure_at(source, path, site).into(),
        MarkdownError::Transclusion(inner) => (*inner).at_inclusion(site).into(),
        other => other,
    }
}

/// `source`, the failure reading `path`, a file a directive or key selected,
/// as that target's I/O failure (code `dm.input.read_failure`), naming
/// `path`, at the inclusion `site` that selected it. The one rule for every
/// read of a selected target: the child load above, and the heading read of
/// a `::toc-linking` target in composition and in the reference graph.
pub(crate) fn read_failure_at(source: std::io::Error, path: &Path, site: Option<InclusionSite>) -> TransclusionError {
    TransclusionError::Io(std::io::Error::new(source.kind(), format!("cannot read {}: {source}", path.display())))
        .at_inclusion(site)
}

/// The file `markdown` was loaded from.
fn file_of(markdown: &crate::markdown::Markdown) -> Option<PathBuf> {
    match markdown.source() {
        Some(crate::markdown::compose::ComposeSource::File(path)) => Some(path.clone()),
        _ => None,
    }
}

/// The byte range of a `prologue:`/`epilogue:` value reading `expected`
/// (`index` for one item of a list) in the loaded document `text`; `None`
/// when it cannot be proven. A leaf is located within its own property, so a
/// construct elsewhere the whole-frontmatter locator cannot read does not cost
/// it its place.
pub(crate) fn frontmatter_value_range(
    text: &str,
    key: &str,
    index: Option<usize>,
    expected: &str,
) -> Option<std::ops::Range<usize>> {
    use crate::markdown::hash::{FrontmatterPathSegment, locate_frontmatter_leaves, locate_frontmatter_value_range};
    let mut path = vec![FrontmatterPathSegment::Key(key.to_string())];
    path.extend(index.map(FrontmatterPathSegment::Index));
    match locate_frontmatter_leaves(text, std::slice::from_ref(&path)) {
        Ok(mut leaves) => Some(leaves.pop()?.range),
        Err(_) => locate_frontmatter_value_range(text, key, index, expected),
    }
}

/// Errors produced by transclusion.
#[derive(Debug, Error)]
pub enum TransclusionError {
    /// An inclusion's target failure (`source`: an invalid or unresolvable
    /// reference, a missing file, a refused URL) at the inclusion `site` that
    /// named the target. Code, recovery, and failure class are `source`'s;
    /// read it with [`without_site`](Self::without_site).
    #[error("{site}: {source}")]
    Located {
        site: Box<InclusionSite>,
        source: Box<TransclusionError>,
    },

    /// `line` is one-based in `ctx`; `0` means unknown (a `prologue` or
    /// `epilogue` field whose key is in no file, or a body directive in a file
    /// whose body map does not prove where it was written), and the message
    /// then names no line.
    #[error("Failed to parse directive{}: {message}", at_line(*.line))]
    ParseDirective {
        ctx: Box<SourceContext>,
        line: usize,
        message: String,
        caret_col: Option<usize>,
        /// A body directive's opener when `ctx` is its loaded file, which
        /// makes `line` an authored line; `None` when `line` counts the
        /// current body or names a frontmatter key.
        authored: Option<crate::markdown::types::AuthoredSpan>,
    },

    #[error(
        "Invalid {} reference '{reference}' in {} at line {line}",
        .directive_kind.as_str(),
        .ctx.display.display()
    )]
    InvalidReference {
        ctx: Box<SourceContext>,
        reference: String,
        line: usize,
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

    #[error("Failed to evaluate condition '{expr}' at line {line}: {cause}")]
    ConditionEval {
        ctx: Box<SourceContext>,
        expr: String,
        /// The directive's line in `ctx`.
        line: usize,
        /// The directive's opener when `ctx` is its loaded file, which makes
        /// `line` an authored line; `None` when `ctx` is the current body.
        authored: Option<crate::markdown::types::AuthoredSpan>,
        #[source]
        cause: Box<crate::markdown::compose::expression::ExpressionError>,
    },

    #[error("Failed to parse condition '{expr}' at line {line}: {message}")]
    ConditionParse {
        ctx: Box<SourceContext>,
        expr: String,
        /// The directive's line in `ctx`.
        line: usize,
        message: String,
        /// As for [`Self::ConditionEval`].
        authored: Option<crate::markdown::types::AuthoredSpan>,
    },

    #[error("Heading re-leveling failed: {0}")]
    Relevel(String),

    #[error("Remote URL transclusion is disabled: {url}")]
    UrlExecutionDisabled { url: String },

    #[error("Remote fetch failed for {url}: {reason}")]
    RemoteFetchFailed { url: String, reason: String },

    #[error(
        "Invalid frontmatter assignment on '::file' directive at line {line}: {reason} (value: {raw})"
    )]
    InvalidFrontmatterAssignment {
        ctx: Box<SourceContext>,
        line: usize,
        raw: String,
        reason: String,
    },

    #[error(
        "Invalid reassigned frontmatter property '{name}' on '::file' directive at line {line}"
    )]
    InvalidReassignedFrontmatterProperty {
        ctx: Box<SourceContext>,
        line: usize,
        name: String,
    },

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("URL parse error: {0}")]
    UrlParse(#[from] url::ParseError),

    #[error("File reference error: {0}")]
    FileReference(#[from] biscuit_file::FileReferenceError),

    #[error("JSON parse error in transclusion option: {0}")]
    Json(#[from] serde_json::Error),
}

/// `text` as a status block's hint reads: a capitalized sentence.
fn capitalized(text: &str) -> String {
    let mut chars = text.chars();
    let mut sentence: String = chars.next().map(|first| first.to_uppercase().chain(chars).collect()).unwrap_or_default();
    if !sentence.ends_with(['.', ')']) {
        sentence.push('.');
    }
    sentence
}

/// The payload of the [`TransclusionError::Io`] a file reference raises when
/// no candidate resolved to a file.
///
/// It stays an `Io` error of kind `NotFound` (callers match on that), and this
/// type is what lets [`TransclusionError::resolution_failure`] tell a no-match
/// from a real I/O failure without reading the message. Its message carries
/// the literal-glob hint (see [`crate::markdown::errors::with_glob_hint`]).
#[derive(Debug)]
pub(crate) struct TargetNotFound {
    pub(crate) reference: String,
    /// What wrote the reference: the directive (`::file`, `::code`), or the
    /// key (`prologue:`) once the failure is placed at a frontmatter value's
    /// [`InclusionSite`].
    pub(crate) construct: Option<&'static str>,
    /// The resolver's evidence: the directory it resolved against and every
    /// candidate it probed.
    pub(crate) resolution: Option<Box<biscuit_file::DetailedResolution>>,
}

impl std::fmt::Display for TargetNotFound {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&crate::markdown::errors::with_glob_hint(
            format!("File not found: {}", self.reference),
            biscuit_file::ResolutionFailure::NoMatch,
            &self.reference,
        ))
    }
}

impl std::error::Error for TargetNotFound {}

impl TransclusionError {
    /// The no-match error for the file reference spelled `reference`.
    pub(crate) fn target_not_found(reference: impl Into<String>) -> Self {
        Self::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            TargetNotFound { reference: reference.into(), construct: None, resolution: None },
        ))
    }

    /// The no-match error of the `kind` directive whose reference
    /// `resolution` resolved to no file, keeping the resolver's evidence.
    pub(crate) fn target_missing(kind: DirectiveKind, resolution: biscuit_file::DetailedResolution) -> Self {
        let construct = match kind {
            DirectiveKind::File => "::file",
            DirectiveKind::Code => "::code",
            DirectiveKind::Url => "::url",
        };
        Self::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            TargetNotFound {
                reference: resolution.raw().to_string(),
                construct: Some(construct),
                resolution: Some(Box::new(resolution)),
            },
        ))
    }

    /// The no-match evidence of a missing target, past any inclusion site.
    pub(crate) fn target_not_found_details(&self) -> Option<&TargetNotFound> {
        match self.without_site() {
            Self::Io(source) => source.get_ref()?.downcast_ref::<TargetNotFound>(),
            _ => None,
        }
    }

    /// The file-reference failure class, when a file reference raised this
    /// error: a resolution error's own class, or
    /// [`ResolutionFailure::NoMatch`](biscuit_file::ResolutionFailure::NoMatch)
    /// when no candidate resolved to a file.
    pub fn resolution_failure(&self) -> Option<biscuit_file::ResolutionFailure> {
        match self {
            Self::Located { source, .. } => source.resolution_failure(),
            Self::FileReference(source) => Some(source.resolution_failure()),
            Self::Io(source)
                if source.get_ref().is_some_and(|inner| inner.is::<TargetNotFound>()) =>
            {
                Some(biscuit_file::ResolutionFailure::NoMatch)
            }
            _ => None,
        }
    }

    /// The failure itself, past a [`Self::Located`] inclusion site.
    pub fn without_site(&self) -> &Self {
        match self {
            Self::Located { source, .. } => source,
            other => other,
        }
    }

    /// The inclusion site a [`Self::Located`] failure names.
    pub fn inclusion_site(&self) -> Option<&InclusionSite> {
        match self {
            Self::Located { site, .. } => Some(site),
            _ => None,
        }
    }

    /// This target failure at the inclusion `site`; unchanged when `site` is
    /// `None` or the failure already names one (a nested child's).
    #[must_use]
    pub(crate) fn at_inclusion(self, site: Option<InclusionSite>) -> Self {
        match (self, site) {
            (located @ Self::Located { .. }, _) | (located, None) => located,
            (mut source, Some(site)) => {
                // A `prologue`/`epilogue` value resolves as a `::file` would;
                // a missing target names the key that wrote it.
                if let (Some(construct), Self::Io(io)) = (site.construct(), &mut source)
                    && let Some(missing) = io.get_mut().and_then(|inner| inner.downcast_mut::<TargetNotFound>())
                {
                    missing.construct = Some(construct);
                }
                Self::Located { site: Box::new(site), source: Box::new(source) }
            }
        }
    }

    /// A body directive's parse failure, whose `line` counts `markdown`'s
    /// current body, placed where the directive was written
    /// ([`DirectiveSite`](crate::markdown::compose::site_scope::DirectiveSite)).
    /// `line_offset` is what the parser added to each body line. Any other
    /// error is unchanged.
    #[must_use]
    pub(crate) fn at_authored_directive(
        mut self,
        markdown: &crate::markdown::Markdown,
        origin: Option<&crate::markdown::compose::body_origin::BodyOrigin>,
        line_offset: usize,
    ) -> Self {
        if let Self::ParseDirective { ctx, line, authored, .. } = &mut self
            && *line > line_offset
        {
            crate::markdown::compose::site_scope::DirectiveSite::of_line(markdown, origin, *line - line_offset)
                .place_in(ctx, line, authored);
        }
        self
    }
}

impl biscuit_terminal::errors::BlockError for TransclusionError {
    fn status_block(
        &self,
        _term: &biscuit_terminal::terminal::Terminal,
    ) -> biscuit_terminal::components::status_block::StatusBlock {
        use biscuit_terminal::components::prose::{LineBreaks, Prose};
        use biscuit_terminal::components::status::StatusState;
        use biscuit_terminal::components::status_block::StatusBlock;
        use biscuit_terminal::errors::{ErrorHeader, StatusBlockExt};

        match self {
            // The failure's own block, led by where the inclusion was written.
            TransclusionError::Located { site, source } => {
                use crate::markdown::compose::site_scope::{line_row, status_rows};
                let file = format!("<dim>File:</dim> {}", Prose::escape_text(&site.file.display().to_string()));
                let line = line_row(site.authored.as_ref().map_or(0, |at| at.line()));
                source
                    .status_block(_term)
                    .prepend_body_line(Prose::new(status_rows([Some(file), line])).with_line_breaks(LineBreaks::Hard))
            }

            TransclusionError::ParseDirective {
                ctx,
                line,
                message,
                caret_col,
                ..
            } => {
                let mut body = if *line == 0 {
                    vec![Prose::new(format!("Directive parsing failed in {}.", ctx.linked_path_prose().content()))]
                } else {
                    vec![Prose::new("Directive parsing failed here:"), ctx.excerpt_prose(*line, 1, "md")]
                };

                if let Some(col) = caret_col {
                    body.push(Prose::new(format!(
                        "<dim>Gutter:</dim> Column {col} is near the error."
                    )));
                }

                StatusBlock::new(StatusState::Error)
                    .error_header(ErrorHeader::new("TransclusionError", "directive parse failed"))
                    .body(body)
                    // The hint is Prose markup: `\` before the newline keeps
                    // the two lines apart (a bare newline is a soft break).
                    .hint(format!(
                        "Error: {message}\\\nCheck syntax: <cyan>::file path=\"...\"</cyan>"
                    ))
            }

            TransclusionError::InvalidReference {
                ctx,
                reference,
                line,
                directive_kind,
            } => {
                let body = vec![
                    Prose::new(format!(
                        "Invalid {} reference <red><b>{}</b></red> found in:",
                        directive_kind.as_str(),
                        Prose::escape_text(reference)
                    )),
                    ctx.excerpt_prose(*line, 2, "md"),
                ];

                StatusBlock::new(StatusState::Error)
                    .error_header(ErrorHeader::new(
                        "TransclusionError",
                        &format!(
                            "The reference in {} could not be resolved.",
                            ctx.linked_path_prose().content()
                        ),
                    ))
                    .body(body)
                    .hint("Verify the path exists and matches your configuration limits.")
            }

            TransclusionError::MissingSourceContext { reference, line } => {
                StatusBlock::new(StatusState::Error)
                    .error_header(ErrorHeader::new(
                        "TransclusionError",
                        "missing source context",
                    ))
                    .body(vec![
                        Prose::new(format!(
                            "Could not resolve <cyan>{}</cyan> at line {line}.",
                            Prose::escape_text(reference)
                        )),
                        Prose::new(
                            "<dim>Note:</dim> Relative transclusions require a file-backed source.",
                        ),
                    ])
                    .hint("Try using an absolute path or `@/` repo-root reference.")
            }

            TransclusionError::UnsupportedReferenceType { reference } => {
                StatusBlock::new(StatusState::Error)
                    .error_header(ErrorHeader::new(
                        "TransclusionError",
                        "unsupported reference type",
                    ))
                    .body(format!(
                        "The reference <cyan>{}</cyan> uses an unsupported scheme or syntax.",
                        Prose::escape_text(reference)
                    ))
                    .hint(
                        "Supported: <dim>./relative, implicit, @magic, &repo-root, \
                         ^repo-scoped, ~/home, vault:, %, {{ENV}}</dim>",
                    )
            }

            TransclusionError::UnsupportedFileType { path } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("TransclusionError", "unsupported file type"))
                .body(format!(
                    "File <cyan>{}</cyan> is not a supported markdown document.",
                    path.display()
                ))
                .hint("Transclusion targets must have <dim>.md</dim> or <dim>.markdown</dim> extensions."),

            TransclusionError::NonTextCodeSource { path } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("TransclusionError", "binary code source"))
                .body(format!(
                    "The code source <cyan>{}</cyan> is not valid UTF-8 text.",
                    path.display()
                ))
                .hint("::code transclusion only supports text files."),

            TransclusionError::CycleDetected { chain } => {
                let mut body = vec![Prose::new("Chain:")];
                for (path, line) in chain {
                    body.push(Prose::new(format!(
                        "  <blue>{}</blue> <dim>:line {}</dim>",
                        path.display(),
                        line
                    )));
                }

                StatusBlock::new(StatusState::Error)
                    .error_header(ErrorHeader::new("TransclusionError", "cycle detected"))
                    .body(body)
                    .hint("Break the cycle by removing one transclusion edge.")
            }

            TransclusionError::MaxDepthExceeded { max_depth } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("TransclusionError", "recursion limit hit"))
                .body(format!("Transclusion depth exceeded the limit of <cyan>{max_depth}</cyan>."))
                .hint("Increase <dim>max_depth</dim> in frontmatter or simplify the document tree."),

            TransclusionError::ConditionEval {
                ctx,
                expr,
                line,
                cause,
                ..
            } => {
                let body = vec![
                    Prose::new(format!(
                        "Failed to evaluate <cyan>when=\"{}\"</cyan>:",
                        Prose::escape_text(expr)
                    )),
                    ctx.excerpt_prose(*line, 1, "md"),
                ];

                StatusBlock::new(StatusState::Error)
                    .error_header(ErrorHeader::new(
                        "TransclusionError",
                        "condition evaluation failed",
                    ))
                    .body(body)
                    .hint(crate::markdown::errors::hint_rows(&format!(
                        "Error: {}",
                        Prose::escape_text_outside_code_spans(&cause.to_string())
                    )))
            }

            TransclusionError::ConditionParse {
                ctx,
                expr,
                line,
                message,
                ..
            } => {
                let body = vec![
                    Prose::new(format!(
                        "Failed to parse <cyan>when=\"{}\"</cyan>:",
                        Prose::escape_text(expr)
                    )),
                    ctx.excerpt_prose(*line, 1, "md"),
                ];

                StatusBlock::new(StatusState::Error)
                    .error_header(ErrorHeader::new("TransclusionError", "condition parse failed"))
                    .body(body)
                    .hint(format!("Error: {}", Prose::escape_text_outside_code_spans(message)))
            }

            TransclusionError::Relevel(message) => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("TransclusionError", "re-leveling failed"))
                .body(Prose::escape_text_outside_code_spans(message))
                .hint("Check that transcluded headings do not push past H6."),

            TransclusionError::UrlExecutionDisabled { url } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new(
                    "TransclusionError",
                    "remote transclusion disabled",
                ))
                .body(format!(
                    "The URL <cyan>{}</cyan> was not fetched: the composing program turned remote transclusion off.",
                    Prose::escape_text(url)
                ))
                .hint("Remote transclusion is an option of the program composing this document; `md compose` enables it, and the `network` permission then decides each fetch."),

            TransclusionError::RemoteFetchFailed { url, reason } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("TransclusionError", "remote fetch failed"))
                .body(format!(
                    "Fetching <cyan>{}</cyan> failed: {}",
                    Prose::escape_text(url),
                    Prose::escape_text_outside_code_spans(reason)
                ))
                .hint("Check the URL, the allowed-hosts policy, and network availability."),

            TransclusionError::InvalidFrontmatterAssignment {
                ctx,
                line,
                raw,
                reason,
            } => {
                let body = vec![
                    Prose::new("Invalid frontmatter override in directive:"),
                    ctx.excerpt_prose(*line, 1, "md"),
                    Prose::new(format!(
                        "<dim>Value:</dim> <red>{}</red>",
                        Prose::escape_text(raw)
                    )),
                ];

                StatusBlock::new(StatusState::Error)
                    .error_header(ErrorHeader::new(
                        "TransclusionError",
                        "invalid frontmatter assignment",
                    ))
                    .body(body)
                    .hint(format!("Error: {}", Prose::escape_text_outside_code_spans(reason)))
            }

            TransclusionError::InvalidReassignedFrontmatterProperty { ctx, line, name } => {
                let body = vec![
                    Prose::new(format!(
                        "Duplicate assignment to property <red><b>{}</b></red> found in:",
                        Prose::escape_text(name)
                    )),
                    ctx.excerpt_prose(*line, 1, "md"),
                ];

                StatusBlock::new(StatusState::Error)
                    .error_header(ErrorHeader::new(
                        "TransclusionError",
                        "duplicate frontmatter property",
                    ))
                    .body(body)
                    .hint("Use only one assignment per property name on a directive line.")
            }

            TransclusionError::Io(source) => {
                // A not-found message carries its own `hint:` row.
                let mut body =
                    vec![Prose::new(source.to_string()).with_line_breaks(LineBreaks::Hard)];
                body.extend(self.resolution_failure().map(crate::markdown::errors::resolution_failure_row));
                if self.resolution_failure() != Some(biscuit_file::ResolutionFailure::NoMatch) {
                    return StatusBlock::new(StatusState::Error)
                        .error_header(ErrorHeader::new("TransclusionError", "I/O failure"))
                        .body(body)
                        .hint("Check file existence and permissions.");
                }
                // A proven no-match: the resolver's evidence names the
                // resolved file and the one to create.
                let missing = self.target_not_found_details();
                let built = missing.and_then(|missing| {
                    let resolution = missing.resolution.as_deref()?;
                    crate::findings::reference::missing_target_unlisted("", missing.construct.unwrap_or("::file"), resolution)
                });
                let details = built.as_ref().and_then(|built| built.reference_details.as_ref());
                if let Some(resolved) = details.and_then(|details| details.resolved.as_deref()) {
                    body.insert(1, Prose::new(format!("<dim>Resolved:</dim> {}", Prose::escape_text(resolved))));
                }
                let header = match missing.and_then(|missing| missing.construct) {
                    Some(construct) => format!("file not found in {construct}"),
                    None => "file not found".to_string(),
                };
                let hint = built
                    .and_then(|built| built.hint)
                    .unwrap_or_else(|| "create the file, or correct the reference".to_string());
                StatusBlock::new(StatusState::Error)
                    .error_header(ErrorHeader::new("TransclusionError", &header))
                    .body(body)
                    .hint(Prose::escape_text_outside_code_spans(&capitalized(&hint)))
            }

            TransclusionError::UrlParse(source) => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("TransclusionError", "URL parse failure"))
                .body(source.to_string())
                .hint("Verify the URL scheme and format (e.g., https://example.com)."),

            TransclusionError::FileReference(source) => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new(
                    "TransclusionError",
                    "file reference failure",
                ))
                .body(vec![
                    Prose::new(source.to_string()),
                    crate::markdown::errors::resolution_failure_row(source.resolution_failure()),
                ])
                .hint("Check sigil usage: `@` magic, `&` repository root, `^` repository-scoped."),

            TransclusionError::Json(source) => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new(
                    "TransclusionError",
                    "JSON5 parse failure in options",
                ))
                .body(Prose::new(format!(
                    "{source}\n<dim>Position:</dim> line {}, column {}",
                    source.line(),
                    source.column()
                )).with_line_breaks(LineBreaks::Hard))
                .hint("Directive options use JSON5; check braces, quoting, and commas."),
        }
    }
}
