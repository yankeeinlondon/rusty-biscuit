//! Type definitions for the markdown module.

use std::ops::Range;
use std::path::PathBuf;

use biscuit_file::YamlParseError;
use biscuit_terminal::components::status_block::StatusBlock;
use biscuit_terminal::errors::BlockError;
use biscuit_terminal::terminal::Terminal;
use indexmap::IndexMap;
use thiserror::Error;

use crate::markdown::compose::expression::ExpressionError;
use crate::markdown::errors::blocks;
use crate::markdown::schemas::ValidationProblem;

use biscuit_terminal::errors::SourceContext;

/// Type alias for frontmatter data.
///
/// Uses `IndexMap` to preserve insertion order so that frontmatter keys
/// are serialized in the same order they appeared in the source document.
pub type FrontmatterMap = IndexMap<String, serde_json::Value>;

/// Where an interpolation error's failing expression physically lives.
///
/// `OnDisk` carries a real [`SourceContext`] for a frontmatter region that maps
/// to a file, so the renderer can show a focused, line-numbered excerpt.
/// `Effective` is the late-binding fallback — DM2 event-time resolution and body
/// text have no stable on-disk locus to slice, so the resolved/expression text is
/// carried instead. Modeling both keeps the excerpt renderer total: it never
/// fabricates line numbers for a region that does not exist on disk.
#[derive(Debug, Clone)]
pub enum SourceRef {
    /// Compose-time: the error maps to a real frontmatter region in a file.
    OnDisk(SourceContext),
    /// Late-binding or body text: no stable on-disk locus; carry the text.
    Effective {
        /// The resolved value or raw expression to display in lieu of a slice.
        rendered: String,
        /// The frontmatter key the expression originated from, when known.
        origin_key: Option<String>,
    },
}

/// Errors that can occur when working with Markdown documents.
#[derive(Error, Debug)]
pub enum MarkdownError {
    /// Failed to parse frontmatter YAML.
    #[error("Failed to parse frontmatter in {}: {source}", .ctx.display.display())]
    FrontmatterParse {
        ctx: SourceContext,
        #[source]
        source: YamlParseError,
    },

    /// The document opens with a near-miss frontmatter fence (e.g. `----`)
    /// wrapping YAML-shaped content. Frontmatter fences must be exactly `---`.
    #[error("frontmatter fence must be exactly `---`, found `{found}` on line {line} in {}", .ctx.display.display())]
    FrontmatterFenceMismatch {
        /// Boxed to keep `MarkdownError` small (a `SourceContext` is large),
        /// matching the boxing convention of the other heavy variants.
        ctx: Box<SourceContext>,
        found: String,
        line: usize,
    },

    /// Failed to merge frontmatter.
    #[error("Failed to merge frontmatter: {0}")]
    FrontmatterMerge(String),

    /// Failed to load file.
    #[error("Failed to load file: {0}")]
    FileLoad(#[from] std::io::Error),

    /// Failed to fetch URL.
    #[error("Failed to fetch URL: {0}")]
    UrlFetch(#[from] reqwest::Error),

    /// Failed to load theme.
    #[error("Failed to load theme: {0}")]
    ThemeLoad(String),

    /// Failed to parse AST.
    #[error("Failed to parse AST: {0}")]
    AstParse(String),

    /// Invalid line range.
    #[error("Invalid line range: {0}")]
    InvalidLineRange(String),

    /// Serialization error.
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Transform pipeline error.
    #[error("Transform error: {0}")]
    Transform(String),

    /// A frontmatter or body `{{ … }}` interpolation failed to evaluate.
    ///
    /// The typed evaluation `cause` is preserved verbatim; this wrapper adds only
    /// *scope* — which frontmatter key (`None` for body text), the expression
    /// span, and where it lives ([`SourceRef`]). The rendered block derives its
    /// headline and hint from `cause`, never from the mechanism word
    /// "interpolation", so the author sees the root cause (e.g. an invalid file
    /// path) rather than the layer that surfaced it.
    #[error("interpolation of `{expression}` failed: {cause}")]
    Interpolation {
        /// The frontmatter key whose whole value failed, or `None` for body text.
        key: Option<String>,
        /// The `{{ … }}` span text that failed.
        expression: String,
        /// Where the failing expression physically lives. Boxed to keep
        /// `MarkdownError` small (the `SourceContext` it can carry is large),
        /// matching the boxing convention of the other heavy variants.
        source: Box<SourceRef>,
        /// The typed evaluation cause. Boxed for the same size reason.
        #[source]
        cause: Box<ExpressionError>,
    },

    /// Transclusion pipeline error.
    #[error("Transclusion error: {0}")]
    Transclusion(#[from] Box<crate::markdown::compose::TransclusionError>),

    /// TOC linking pipeline error.
    #[error("TOC linking error: {0}")]
    TocLinking(#[from] crate::markdown::compose::TocLinkingError),

    /// File-links directive pipeline error.
    #[error("File-links error: {0}")]
    FileLinks(#[from] crate::markdown::compose::FileLinksError),

    /// Shell expansion pipeline error.
    #[error("Shell expansion failed: {0}")]
    ShellExpansion(#[from] Box<crate::markdown::compose::ShellExpansionError>),

    /// Page block pipeline error.
    #[error("Page block error: {0}")]
    PageBlock(#[from] Box<crate::markdown::compose::page_blocks::PageBlockError>),

    /// Shell block pipeline error.
    #[error("Shell block error: {0}")]
    ShellBlock(#[from] Box<crate::markdown::compose::ShellBlockError>),

    /// Reference analysis error.
    #[error("Reference error: {0}")]
    Reference(#[from] Box<crate::markdown::reference::ReferenceError>),

    /// Context merge error (invalid user ctx).
    #[error("Context error: {0}")]
    CtxMerge(#[from] crate::markdown::compose::context::merge::CtxMergeError),

    /// A document's stored `hash` property could not be parsed.
    #[error("Malformed stored hash in '{property}': {reason}")]
    MalformedStoredHash {
        /// The frontmatter property the malformed hash was read from.
        property: String,
        /// Why parsing failed, phrased for a CLI user to act on.
        reason: String,
    },

    /// A disclosure block was malformed at render time.
    ///
    /// Raised by the block-extension processor when a `::disclosure` region
    /// violates the summary/body rules or is missing a required delimiter.
    #[error("Malformed disclosure block: {reason}")]
    MalformedDisclosure {
        /// Human-readable reason the block was rejected.
        reason: String,
        /// Byte range of the disclosure region in the source document.
        range: Range<usize>,
    },

    /// The render-tree document renderer rejected the document.
    ///
    /// Raised when [`Markdown::as_html`](crate::markdown::Markdown::as_html) or
    /// [`Markdown::as_terminal`](crate::markdown::Markdown::as_terminal) routes
    /// through the render tree and the shared renderer returns a fatal
    /// [`RenderError`](renderable::tree::RenderError) (structural validation
    /// failure, or a strict-mode rejection). Non-fatal fold/render diagnostics
    /// are not surfaced here — they stay on the tree pipeline's diagnostic
    /// channel.
    #[error("Render-tree error: {0}")]
    RenderTree(#[from] renderable::tree::RenderError),

    /// Schema validation failed during compose.
    #[error("Schema validation failed for {path:?}: {summary}")]
    SchemaValidationFailed {
        /// Source file or "<stdin>".
        path: PathBuf,
        /// Validation problems reported by the schema subsystem.
        problems: Vec<ValidationProblem>,
        /// Short one-line summary for the top-level message.
        summary: String,
        /// Document description from frontmatter, when present.
        description: Option<String>,
        /// Underlying schema-preparation error, when the failure originated
        /// in schema parsing, resolution, conversion, baseline merge, or
        /// validator construction. `None` when the schema was prepared
        /// successfully but the frontmatter did not satisfy it (in which case
        /// the failure detail lives in `problems`).
        ///
        /// Boxed as `dyn Error` rather than `Box<SchemaError>` so that
        /// [`std::error::Error::source`] yields the inner
        /// [`SchemaError`](crate::markdown::schemas::SchemaError) directly:
        /// `err.source().and_then(|e| e.downcast_ref::<SchemaError>())`
        /// recovers the original. A `Box<SchemaError>` field would surface the
        /// `Box` itself as the trait object, so that downcast would miss.
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync + 'static>>,
    },
}

impl From<crate::markdown::compose::TransclusionError> for MarkdownError {
    fn from(err: crate::markdown::compose::TransclusionError) -> Self {
        MarkdownError::Transclusion(Box::new(err))
    }
}

impl From<crate::markdown::compose::ShellExpansionError> for MarkdownError {
    fn from(err: crate::markdown::compose::ShellExpansionError) -> Self {
        MarkdownError::ShellExpansion(Box::new(err))
    }
}

impl From<crate::markdown::compose::page_blocks::PageBlockError> for MarkdownError {
    fn from(err: crate::markdown::compose::page_blocks::PageBlockError) -> Self {
        MarkdownError::PageBlock(Box::new(err))
    }
}

impl From<crate::markdown::compose::ShellBlockError> for MarkdownError {
    fn from(err: crate::markdown::compose::ShellBlockError) -> Self {
        MarkdownError::ShellBlock(Box::new(err))
    }
}

impl From<crate::markdown::reference::ReferenceError> for MarkdownError {
    fn from(err: crate::markdown::reference::ReferenceError) -> Self {
        MarkdownError::Reference(Box::new(err))
    }
}

/// Result type for markdown operations.
pub type MarkdownResult<T> = Result<T, MarkdownError>;

impl MarkdownError {
    /// Anchors a [`MarkdownError::Interpolation`] to a real on-disk frontmatter
    /// region so the rendered block can show an OSC8-linked prompt file and a
    /// focused YAML excerpt.
    ///
    /// The interpolation error is built deep in the engine with a
    /// [`SourceRef::Effective`] placeholder, because the engine has only the
    /// expression text — not the document's [`SourceContext`]. This method is
    /// called at the compose-pipeline boundary, where the document *is* in
    /// scope, to upgrade that placeholder to [`SourceRef::OnDisk`]. A
    /// `SourceContext` whose `display` path is `"unknown"` (in-memory/stdin
    /// compose with no file locus) leaves the error untouched so the renderer
    /// keeps the late-binding presentation rather than linking a non-file.
    ///
    /// Errors that are not `Interpolation`, or whose `source` is already
    /// `OnDisk`, pass through unchanged.
    pub(crate) fn with_on_disk_source(self, ctx: &SourceContext) -> Self {
        match self {
            MarkdownError::Interpolation {
                key,
                expression,
                source,
                cause,
            } => {
                let source = match *source {
                    SourceRef::Effective { .. }
                        if ctx.display != std::path::Path::new("unknown") =>
                    {
                        Box::new(SourceRef::OnDisk(ctx.clone()))
                    }
                    other => Box::new(other),
                };
                MarkdownError::Interpolation {
                    key,
                    expression,
                    source,
                    cause,
                }
            }
            other => other,
        }
    }
}

impl BlockError for MarkdownError {
    fn status_block(&self, term: &Terminal) -> StatusBlock {
        match self {
            // Delegating variants: surface the sub-error's block directly.
            MarkdownError::Transclusion(inner) => inner.status_block(term),
            MarkdownError::ShellExpansion(inner) => inner.status_block(term),
            MarkdownError::PageBlock(inner) => inner.status_block(term),
            MarkdownError::ShellBlock(inner) => inner.status_block(term),
            MarkdownError::TocLinking(inner) => inner.status_block(term),
            MarkdownError::FileLinks(inner) => inner.status_block(term),
            MarkdownError::Reference(inner) => inner.status_block(term),
            MarkdownError::CtxMerge(inner) => inner.status_block(term),

            // Leaf variants own their block shape.
            MarkdownError::FrontmatterParse { ctx, source } => {
                blocks::frontmatter_parse_block(ctx.clone(), source)
            }
            MarkdownError::FrontmatterFenceMismatch { ctx, found, line } => {
                blocks::frontmatter_fence_mismatch_block(ctx.as_ref().clone(), found, *line)
            }
            MarkdownError::FrontmatterMerge(message) => blocks::frontmatter_merge_block(message),
            MarkdownError::FileLoad(source) => blocks::file_load_block(source),
            MarkdownError::UrlFetch(source) => blocks::url_fetch_block(source),
            MarkdownError::ThemeLoad(message) => blocks::theme_load_block(message),
            MarkdownError::AstParse(message) => blocks::ast_parse_block(message),
            MarkdownError::InvalidLineRange(message) => blocks::invalid_line_range_block(message),
            MarkdownError::Serialization(source) => blocks::serialization_block(source),
            MarkdownError::Transform(message) => blocks::transform_block(message),
            MarkdownError::Interpolation {
                key,
                expression,
                source,
                cause,
            } => blocks::interpolation_block(key.as_deref(), expression, source, cause),
            MarkdownError::RenderTree(source) => blocks::render_tree_block(&source.to_string()),
            MarkdownError::MalformedStoredHash { property, reason } => {
                blocks::malformed_stored_hash_block(property, reason)
            }
            MarkdownError::MalformedDisclosure { reason, range } => {
                blocks::malformed_disclosure_block(reason, range)
            }
            MarkdownError::SchemaValidationFailed {
                path,
                problems,
                summary,
                description,
                // The preparation source is preserved for `Error::source()`
                // programmatic recovery; the styled block renders `summary`
                // and `problems` only.
                source: _,
            } => blocks::schema_validation_failed_block(path, problems, summary, description),
        }
    }

    fn block_source(&self) -> Option<&(dyn BlockError + 'static)> {
        // Delegating variants already return the inner block from
        // `status_block`, so returning `Some(inner)` here would double-render
        // the same block under a "Caused by:" caption. Only leaf variants
        // that wrap a foreign error type and have no sub-block would surface
        // a cause — none of today's leaf variants do.
        None
    }
}
