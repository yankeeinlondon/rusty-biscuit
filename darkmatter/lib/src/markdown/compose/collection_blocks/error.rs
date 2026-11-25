//! Typed collection failures.

use biscuit_terminal::components::status::StatusState;
use biscuit_terminal::components::status_block::StatusBlock;
use biscuit_terminal::errors::{BlockError, ErrorHeader, StatusBlockExt};
use biscuit_terminal::terminal::Terminal;

use super::diagnostics;
use super::types::CollectionKind;
use crate::markdown::compose::expression::ExpressionError;
use crate::markdown::compose::expression::iteration::{IterationError, IterationPath};
use crate::markdown::types::{MarkdownError, SourceRef};

/// Where a collection failure happened: the authored location and, for a
/// runtime failure, the original positions of the items being processed.
#[derive(Debug, Clone)]
pub struct CollectionSite {
    /// The authored source, boxed to keep `MarkdownError` small.
    pub source: Box<SourceRef>,
    /// Original indices or keys, outermost first; empty for a static finding.
    pub iteration_path: IterationPath,
}

/// A collection region failed to parse, type-check, or run.
///
/// Every variant is request-fatal in the pipeline: no partial collection
/// result is returned after an unhandled failure.
#[derive(Debug, Clone, thiserror::Error)]
pub enum CollectionError {
    /// Invalid region, header, or options.
    #[error("{message}{}", super::super::block_pairs::note_suffix(note))]
    Malformed {
        /// Where.
        site: CollectionSite,
        /// What is wrong, without a line.
        message: String,
        /// Another line a pairing reason refers to (such as where a region
        /// a closer misses was opened), counted in the scanned body; the
        /// compose stage moves it to the loaded file's line, or drops it
        /// when that line is not proven.
        note: Option<String>,
    },

    /// The source has no iterable declaration.
    #[error("`{reference}` has no declared iterable type")]
    UntypedSource {
        /// Where.
        site: CollectionSite,
        /// Which directive.
        kind: CollectionKind,
        /// The source reference as authored.
        reference: String,
    },

    /// The source is known to be unsuitable, or was a scalar when reached.
    #[error("{message}")]
    InvalidSource {
        /// Where.
        site: CollectionSite,
        /// Which directive.
        kind: CollectionKind,
        /// The source reference as authored.
        reference: String,
        /// Why it is unsuitable.
        message: String,
    },

    /// Callback grammar, or a parameter that collides with a protected name.
    #[error("{message}")]
    InvalidCallback {
        /// Where.
        site: CollectionSite,
        /// What is wrong.
        message: String,
    },

    /// Input or result destructuring is incompatible with the item type.
    #[error("{message}")]
    PatternMismatch {
        /// Where.
        site: CollectionSite,
        /// What is incompatible.
        message: String,
    },

    /// A sort key is null, missing, a boolean, a container, or mixed.
    #[error("{message}")]
    InvalidSortKey {
        /// Where.
        site: CollectionSite,
        /// What is wrong with the key.
        message: String,
    },

    /// A reached callback failed and nothing handled it.
    #[error("callback failed: {cause}")]
    CallbackFailed {
        /// Where.
        site: CollectionSite,
        /// The underlying typed cause.
        #[source]
        cause: Box<ExpressionError>,
    },

    /// A request allowance ran out.
    ///
    /// The counters that detect exhaustion know only the iteration path; the
    /// stage that charged them locates the failure at the authored site whose
    /// work was being paid for (see [`Self::with_source`]).
    #[error("{error}")]
    Budget {
        /// The authored site whose work exhausted the allowance, once the
        /// charging stage located it; its path is `error.path`.
        site: Option<CollectionSite>,
        /// The typed exhaustion, with the original items being processed.
        error: Box<IterationError>,
    },
}

impl CollectionError {
    /// A pairing failure whose note's line, counted in `markdown`'s scanned
    /// body (over `data`, its inserted bytes), is moved to the loaded file's
    /// line where `origin` proves it and dropped where it does not; a
    /// document with no file keeps the body's count. The same body and data
    /// fail the same way, so the note is rescanned.
    #[must_use]
    pub(crate) fn with_file_note(
        self,
        markdown: &crate::markdown::Markdown,
        origin: Option<&super::super::body_origin::BodyOrigin>,
        data: Option<&super::super::body_origin::DataRanges>,
    ) -> Self {
        let Self::Malformed { site, message, note: Some(note) } = self else { return self };
        if !matches!(markdown.source(), Some(super::super::ComposeSource::File(_))) {
            return Self::Malformed { site, message, note: Some(note) };
        }
        let data = data.filter(|data| data.describes(markdown.content()));
        let note = match super::super::block_pairs::scan_block_pairs_in(markdown.content(), data) {
            Err(scan) if scan.to_string() == message => {
                scan.note_with(|line| super::super::site_scope::DirectiveSite::file_line(markdown, origin, line))
            }
            _ => None,
        };
        Self::Malformed { site, message, note }
    }
}

impl CollectionError {
    /// The registered diagnostic code.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Malformed { .. } => diagnostics::MALFORMED_CODE,
            Self::UntypedSource { kind: CollectionKind::Map, .. } => diagnostics::UNTYPED_MAP_SOURCE_CODE,
            Self::UntypedSource { kind: CollectionKind::Filter, .. } => diagnostics::UNTYPED_FILTER_SOURCE_CODE,
            Self::InvalidSource { .. } => diagnostics::INVALID_SOURCE_CODE,
            Self::InvalidCallback { .. } => diagnostics::INVALID_CALLBACK_CODE,
            Self::PatternMismatch { .. } => diagnostics::PATTERN_MISMATCH_CODE,
            Self::InvalidSortKey { .. } => diagnostics::INVALID_SORT_KEY_CODE,
            Self::CallbackFailed { cause, .. } => match cause.without_callbacks() {
                ExpressionError::IterationLimit(error) => error.kind.code(),
                _ => diagnostics::CALLBACK_FAILED_CODE,
            },
            Self::Budget { error, .. } => error.kind.code(),
        }
    }

    /// An exhaustion not yet located at an authored site.
    pub fn budget(error: IterationError) -> Self {
        Self::Budget { site: None, error: Box::new(error) }
    }

    /// Whether this is an exhaustion no stage has located yet.
    pub fn is_unlocated_budget(&self) -> bool {
        matches!(self, Self::Budget { site: None, .. })
    }

    /// This failure located at `source` instead: the same finding re-anchored
    /// to a richer authored context. An unlocated budget failure gains a site
    /// whose path is the exhaustion's.
    #[must_use]
    pub fn with_source(mut self, source: SourceRef) -> Self {
        match &mut self {
            Self::Malformed { site, .. }
            | Self::UntypedSource { site, .. }
            | Self::InvalidSource { site, .. }
            | Self::InvalidCallback { site, .. }
            | Self::PatternMismatch { site, .. }
            | Self::InvalidSortKey { site, .. }
            | Self::CallbackFailed { site, .. }
            | Self::Budget { site: Some(site), .. } => *site.source = source,
            Self::Budget { site, error } => {
                *site = Some(CollectionSite { source: Box::new(source), iteration_path: error.path.clone() });
            }
        }
        self
    }

    /// The failure's location, when it has one.
    pub fn site(&self) -> Option<&CollectionSite> {
        match self {
            Self::Malformed { site, .. }
            | Self::UntypedSource { site, .. }
            | Self::InvalidSource { site, .. }
            | Self::InvalidCallback { site, .. }
            | Self::PatternMismatch { site, .. }
            | Self::InvalidSortKey { site, .. }
            | Self::CallbackFailed { site, .. } => Some(site),
            Self::Budget { site, .. } => site.as_ref(),
        }
    }

    /// The original item positions of a runtime failure.
    pub fn iteration_path(&self) -> IterationPath {
        match self {
            Self::Budget { error, .. } => error.path.clone(),
            Self::CallbackFailed { site, cause } => site.iteration_path.joined(&cause.callback_path()),
            other => other.site().map(|site| site.iteration_path.clone()).unwrap_or_default(),
        }
    }
}

impl BlockError for CollectionError {
    fn status_block(&self, _term: &Terminal) -> StatusBlock {
        let path = self.iteration_path();
        let body = if path.is_empty() {
            self.to_string()
        } else {
            format!("{self}\nat {path}")
        };
        StatusBlock::new(StatusState::Error)
            .error_header(ErrorHeader::new("CollectionError", self.code()))
            .body(body)
    }
}

/// A later body stage's failure at a site a `::map` / `::filter` body
/// repeated: the underlying error, unchanged, with the authored site and the
/// original items of the copy the site ran in.
///
/// Wrapping never changes what the failure is. Its message, diagnostic code,
/// and recoverability are `error`'s: recovery decisions read
/// [`MarkdownError::without_iteration`], and the error chain continues into
/// `error`. A failure inside a transcluded child that a repeated `::file`
/// included is wrapped once per enclosing document, outermost first.
#[derive(Debug)]
pub struct IterationFailure {
    /// The authored site in the loaded file, when the stage proved one.
    pub location: Option<Box<SourceRef>>,
    /// The original items of the copy, outermost first; never empty.
    pub iteration_path: IterationPath,
    /// The failure itself.
    pub error: Box<MarkdownError>,
}

impl std::fmt::Display for IterationFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.error.fmt(f)
    }
}

impl std::error::Error for IterationFailure {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(self.error.as_ref())
    }
}

impl BlockError for IterationFailure {
    fn status_block(&self, term: &Terminal) -> StatusBlock {
        self.error.status_block(term).hint(format!("Iteration path: {}", self.iteration_path))
    }
}
