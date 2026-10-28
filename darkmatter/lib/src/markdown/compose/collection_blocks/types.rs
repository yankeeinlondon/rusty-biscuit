//! Passive collection-region model.

use std::ops::Range;
use std::path::Path;

use crate::markdown::compose::expression::{BindingView, Diagnostic, TypeScope};
use crate::markdown::compose::pipeline::operations::ComposeOperationSet;

/// Which directive a region is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CollectionKind {
    /// `::map` / `::end-map`: the body sees the transformed value.
    Map,
    /// `::filter` / `::end-filter`: the body sees the original value.
    Filter,
}

impl CollectionKind {
    /// The opening directive keyword.
    pub const fn opener(self) -> &'static str {
        match self {
            Self::Map => "map",
            Self::Filter => "filter",
        }
    }

    /// The closing directive keyword.
    pub const fn closer(self) -> &'static str {
        match self {
            Self::Map => "end-map",
            Self::Filter => "end-filter",
        }
    }
}

/// One paired region with exact byte spans in the scanned text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollectionRegion {
    /// Which directive.
    pub kind: CollectionKind,
    /// The whole region, opener line through closer line.
    pub span: Range<usize>,
    /// The opener line's header text, after the keyword.
    pub header_span: Range<usize>,
    /// The body, excluding both wrapper lines.
    pub body_span: Range<usize>,
    /// 1-based line of the opener.
    pub start_line: usize,
    /// 1-based line of the closer.
    pub end_line: usize,
    /// Regions nested in the body.
    pub children: Vec<CollectionRegion>,
}

/// The scope inventory of one body: which locals and guard facts hold at each
/// cursor or site.
///
/// This is the one authority for in-body scopes. `page_blocks::BodyGuards` is
/// its conditional-only projection.
#[derive(Debug, Clone, Default)]
pub struct BodyScopes {
    _private: (),
}

impl BodyScopes {
    /// An inventory with no scoped regions: every offset sees the site scope.
    pub const fn empty() -> Self {
        Self { _private: () }
    }

    /// The scope at `offset`, derived from the enclosing `site`.
    pub fn scope_at<'d>(&self, site: TypeScope<'d>, _offset: usize) -> TypeScope<'d> {
        site
    }
}

/// What passive analysis may read; it holds no runtime provider or
/// resolution service.
#[derive(Debug, Clone)]
pub struct CollectionAnalysisOptions<'a> {
    /// The enabled operations; disabled wrappers are inert text.
    pub operations: ComposeOperationSet,
    /// The host's declared globals, if any.
    pub binding_view: Option<&'a BindingView>,
    /// Report an untyped `::map` source as a warning rather than an error.
    pub allow_untyped_map_variables: bool,
    /// Report an untyped `::filter` source as a warning rather than an error.
    pub allow_untyped_filter_variables: bool,
    /// The authored file, for located findings.
    pub source_path: Option<&'a Path>,
}

/// The result of passive analysis.
#[derive(Debug, Clone, Default)]
pub struct CollectionAnalysis {
    /// Every region, outermost first.
    pub regions: Vec<CollectionRegion>,
    /// The scope inventory.
    pub scopes: BodyScopes,
    /// Findings shared with library and editor consumers.
    pub diagnostics: Vec<Diagnostic>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds_name_their_distinct_closers() {
        assert_eq!((CollectionKind::Map.opener(), CollectionKind::Map.closer()), ("map", "end-map"));
        assert_eq!((CollectionKind::Filter.opener(), CollectionKind::Filter.closer()), ("filter", "end-filter"));
    }
}
