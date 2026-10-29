//! Passive collection-region model.

use std::ops::Range;
use std::path::Path;

use super::scopes::BodyScopes;
use crate::markdown::compose::expression::{BindingView, Diagnostic};
use crate::markdown::compose::parse_utils::MappedQuotedValue;
use crate::markdown::compose::pipeline::operations::ComposeOperationSet;
use crate::markdown::span::Spanned;

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

/// A quoted callback or condition value: the text decoded once, plus the
/// authored location of every decoded byte.
///
/// Expression parse errors are reported against `text`; project them through
/// [`authored_range`](Self::authored_range) so an escaped quote does not shift
/// an editor range.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuotedExpression {
    /// The decoded value: quotes removed, escapes resolved.
    pub text: String,
    /// Authored byte span of the whole value, quotes included.
    pub span: Range<usize>,
    pub(crate) mapped: MappedQuotedValue,
    /// Added to the mapped offsets to reach scanned-text offsets.
    pub(crate) origin: usize,
}

impl QuotedExpression {
    /// The authored byte range covering `decoded` in [`text`](Self::text), or
    /// `None` when it lies outside the value.
    pub fn authored_range(&self, decoded: Range<usize>) -> Option<Range<usize>> {
        let range = self.mapped.authored_range(decoded)?;
        Some(range.start + self.origin..range.end + self.origin)
    }
}

/// The direction of a `sort-order` option.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortOrder {
    /// `sort-order="asc"`, the default.
    Asc,
    /// `sort-order="desc"`.
    Desc,
}

/// The validated header of an opener line.
///
/// Every callback and condition has passed its grammar check; nothing here has
/// been evaluated or type-checked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollectionHeader {
    /// The dotted source path, as authored.
    pub source: Spanned<String>,
    /// The positional `"pattern => expression"` callback.
    pub callback: Option<QuotedExpression>,
    /// `when="..."`: the block-level condition.
    pub when: Option<QuotedExpression>,
    /// `filter="..."`: the per-item selection.
    pub filter: Option<QuotedExpression>,
    /// `sort="..."`: the key selector.
    pub sort: Option<QuotedExpression>,
    /// `sort-order`; only present together with `sort`.
    pub sort_order: Option<Spanned<SortOrder>>,
    /// `while="..."`: stop before the first falsey result.
    pub while_predicate: Option<QuotedExpression>,
    /// `until="..."`: stop before the first truthy result.
    pub until_predicate: Option<QuotedExpression>,
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
    /// The validated header.
    pub header: CollectionHeader,
    /// The body, excluding both wrapper lines.
    pub body_span: Range<usize>,
    /// 1-based line of the opener.
    pub start_line: usize,
    /// 1-based line of the closer.
    pub end_line: usize,
    /// Regions nested in the body.
    pub children: Vec<CollectionRegion>,
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

/// An untyped source a recovery boolean waived: reported once at the
/// authored source, never per iteration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveredSource {
    /// Which directive.
    pub kind: CollectionKind,
    /// The source reference as authored.
    pub reference: String,
    /// Authored byte span of the source reference in the scanned text.
    pub span: Range<usize>,
    /// The warning, ready for a report.
    pub diagnostic: Diagnostic,
}

/// The result of passive analysis.
#[derive(Debug, Clone, Default)]
pub struct CollectionAnalysis {
    /// Every region, outermost first.
    pub regions: Vec<CollectionRegion>,
    /// The scope inventory.
    pub scopes: BodyScopes,
    /// Findings shared with library and editor consumers: one warning per
    /// recovered source.
    pub diagnostics: Vec<Diagnostic>,
    /// The recovered sources behind `diagnostics`, with their authored spans.
    pub recovered: Vec<RecoveredSource>,
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
