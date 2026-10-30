//! When a document-level validation locates its problems.
//!
//! The [`PositionMap`] itself, and how a problem is located in it, are the
//! library's; this decides when the map of a Markdown document is built.

use std::cell::OnceCell;

use super::{PositionMap, ValidationProblem};
use crate::markdown::Markdown;

/// The source positions one validation locates its final problems with.
///
/// Problems are built without coordinates and located together once the
/// validation has settled which of them it reports: excused template
/// problems, losing root-union arms, and replaced union reports are never
/// located. A `Deferred` source builds its [`PositionMap`] from the document
/// the first time a report holds a problem and reuses it for any later one, so
/// a report with no problems builds none and one with many builds one.
pub(super) enum ProblemPositions<'a> {
    /// A map the caller already built (possibly the empty map).
    Built(&'a PositionMap),
    /// The map of `source`, built on first use.
    Deferred { source: &'a Markdown, map: OnceCell<PositionMap> },
}

impl<'a> ProblemPositions<'a> {
    /// Positions drawn from `source`'s authored frontmatter, built only if a
    /// problem needs one (see [`super::positions_for`]).
    pub(super) fn deferred(source: &'a Markdown) -> Self {
        Self::Deferred { source, map: OnceCell::new() }
    }

    /// Locates every problem of a settled report; builds nothing when
    /// `problems` is empty.
    pub(super) fn locate_all(&self, problems: &mut [ValidationProblem]) {
        if problems.is_empty() {
            return;
        }
        let map = match self {
            Self::Built(map) => *map,
            Self::Deferred { source, map } => map.get_or_init(|| super::positions_for(source)),
        };
        map.locate_all(problems);
    }
}
