//! Typed collection failures.

use biscuit_terminal::components::status::StatusState;
use biscuit_terminal::components::status_block::StatusBlock;
use biscuit_terminal::errors::{BlockError, ErrorHeader, StatusBlockExt};
use biscuit_terminal::terminal::Terminal;

use super::diagnostics;
use super::types::CollectionKind;
use crate::markdown::compose::expression::ExpressionError;
use crate::markdown::compose::expression::iteration::{IterationError, IterationPath};
use crate::markdown::types::SourceRef;

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
    #[error("{message}")]
    Malformed {
        /// Where.
        site: CollectionSite,
        /// What is wrong.
        message: String,
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
    #[error("{0}")]
    Budget(Box<IterationError>),
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
            Self::CallbackFailed { .. } => diagnostics::CALLBACK_FAILED_CODE,
            Self::Budget(error) => error.kind.code(),
        }
    }

    /// This failure located at `source` instead: the same finding re-anchored
    /// to a richer authored context. A budget failure has no site and is
    /// returned unchanged.
    #[must_use]
    pub fn with_source(mut self, source: SourceRef) -> Self {
        match &mut self {
            Self::Malformed { site, .. }
            | Self::UntypedSource { site, .. }
            | Self::InvalidSource { site, .. }
            | Self::InvalidCallback { site, .. }
            | Self::PatternMismatch { site, .. }
            | Self::InvalidSortKey { site, .. }
            | Self::CallbackFailed { site, .. } => *site.source = source,
            Self::Budget(_) => {}
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
            Self::Budget(_) => None,
        }
    }

    /// The original item positions of a runtime failure.
    pub fn iteration_path(&self) -> IterationPath {
        match self {
            Self::Budget(error) => error.path.clone(),
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
