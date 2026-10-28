//! Layer-3 collection overlay: request-time reading of `::map` / `::filter`
//! regions.
//!
//! Everything here is a thin projection of Darkmatter's shared region
//! analysis: [`BodyScopes`] answers which locals and facts hold at a byte
//! offset, [`analyze_collection_blocks`] reports the static defects
//! composition rejects, and the lenient scan explains headers that do not
//! validate. DMLS keeps no type resolver of its own; this module only maps
//! authored byte spans (including escaped callback values) back to document
//! offsets and groups the results for the providers. Nothing reads the
//! filesystem, runs a callback, or resolves a target.

use std::ops::Range;
use std::path::Path;

use darkmatter::markdown::SourceRef;
use darkmatter::markdown::compose::collection_blocks::{
    BodyScopes, CollectionAnalysisOptions, CollectionRegion, ExpressionRole, ExpressionSite, LenientCollectionScan,
    analyze_collection_blocks, diagnostics as codes, scan_collection_regions_lenient,
};
use darkmatter::markdown::compose::collection_blocks::QuotedExpression;
use darkmatter::markdown::compose::expression::{BindingView, TypeScope};
use darkmatter::markdown::compose::ComposeOperationSet;

/// The scope inventory of `text` for every in-body consumer: the enclosing
/// `::block when` guards and the locals of every enclosing `::map` /
/// `::filter` body. Offsets are document offsets.
pub fn scopes(text: &str) -> BodyScopes {
    BodyScopes::scan(text, &ComposeOperationSet::all())
}

/// Whether `text` could contain a collection wrapper; a cheap gate before the
/// scanners run.
pub fn may_contain_collections(text: &str) -> bool {
    text.contains("::map") || text.contains("::filter") || text.contains("::end-map") || text.contains("::end-filter")
}

/// One static finding of the shared analysis, ranged in the document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegionFinding {
    /// The registered diagnostic code (the library's, unchanged).
    pub code: String,
    /// Whether the finding is a warning (a waived untyped source) rather than
    /// an error.
    pub warning: bool,
    /// Document byte range of the finding.
    pub span: Range<usize>,
    /// The library's message.
    pub message: String,
}

/// The header, structural, and typing findings of every region of `text`,
/// each once, at the span composition reports.
///
/// `site` is the declaration-only scope of the whole body; `view` the host's
/// declared globals. The result is the library's own: the first strict defect
/// ([`analyze_collection_blocks`]) plus every other header problem the lenient
/// scan recovers, deduplicated by code and start.
pub fn region_findings(text: &str, site: &TypeScope<'_>, view: Option<&BindingView>) -> Vec<RegionFinding> {
    if !may_contain_collections(text) {
        return Vec::new();
    }
    let scan = scan_collection_regions_lenient(text);
    let mut found: Vec<RegionFinding> = Vec::new();
    let mut push = |finding: RegionFinding| {
        if !found.iter().any(|known| known.code == finding.code && known.span.start == finding.span.start) {
            found.push(finding);
        }
    };

    for header in &scan.headers {
        for problem in &header.problems {
            push(RegionFinding {
                code: (if problem.callback { codes::INVALID_CALLBACK_CODE } else { codes::MALFORMED_CODE }).to_string(),
                warning: false,
                span: widen(text, problem.span.clone()),
                message: problem.message.clone(),
            });
        }
    }

    let options = CollectionAnalysisOptions {
        operations: ComposeOperationSet::all(),
        binding_view: view,
        allow_untyped_map_variables: false,
        allow_untyped_filter_variables: false,
        // Names the site so the strict parser locates its findings; the
        // location is only read back as a byte range.
        source_path: Some(Path::new("document")),
    };
    match analyze_collection_blocks(text, site, &options) {
        Ok(analysis) => {
            for recovered in analysis.recovered {
                push(RegionFinding {
                    code: recovered.diagnostic.code,
                    warning: true,
                    span: widen(text, recovered.span),
                    message: recovered.diagnostic.message,
                });
            }
        }
        Err(error) => {
            // A structural failure of a shared pairing is the block
            // diagnostics' to report unless the line is a collection marker.
            let structural_elsewhere = scan.structural.as_ref().is_some_and(|structural| {
                let line = structural_line(structural);
                !is_collection_marker_line(text, line)
            });
            if !structural_elsewhere
                && let Some(SourceRef::OnDiskSpan { span, .. }) = error.site().map(|site| site.source.as_ref())
            {
                push(RegionFinding {
                    code: error.code().to_string(),
                    warning: false,
                    span: span.range(),
                    message: error.to_string(),
                });
            }
        }
    }
    found.sort_by_key(|finding| finding.span.start);
    found
}

fn structural_line(error: &darkmatter::markdown::compose::directives_api::BlockScanError) -> usize {
    use darkmatter::markdown::compose::directives_api::BlockScanError;
    match error {
        BlockScanError::UnmatchedEnd { line }
        | BlockScanError::UnterminatedBlock { line, .. }
        | BlockScanError::TrailingContent { line, .. } => *line,
    }
}

/// Whether the 1-based `line` of `text` starts with a collection marker.
fn is_collection_marker_line(text: &str, line: usize) -> bool {
    let Some(content) = text.lines().nth(line.saturating_sub(1)) else {
        return false;
    };
    let trimmed = content.trim_start().trim_start_matches('>').trim_start();
    ["::map", "::end-map", "::filter", "::end-filter"].iter().any(|marker| {
        trimmed.strip_prefix(marker).is_some_and(|rest| rest.is_empty() || rest.starts_with(char::is_whitespace))
    })
}

/// An empty `span` widened by one character so an editor can draw it.
fn widen(text: &str, span: Range<usize>) -> Range<usize> {
    if !span.is_empty() {
        return span;
    }
    match text.get(span.start..).and_then(|rest| rest.chars().next()) {
        Some(next) => span.start..span.start + next.len_utf8(),
        None => text[..span.start.min(text.len())]
            .char_indices()
            .next_back()
            .map_or(span.clone(), |(index, _)| index..span.start),
    }
}

/// An authored expression of a region together with its quoted authored text.
pub struct AuthoredSite<'d> {
    /// The library's site: role, decoded text, parsed expression, and scope.
    pub site: ExpressionSite<'d>,
    /// The quoted value as authored, which maps decoded offsets back.
    pub quoted: QuotedExpression,
}

impl AuthoredSite<'_> {
    /// The document range of the decoded `span` of this site's text; the
    /// whole quoted value when an escape makes the span unmappable.
    pub fn authored(&self, span: Range<usize>) -> Range<usize> {
        self.quoted.authored_range(span).unwrap_or_else(|| self.quoted.span.clone())
    }

    /// The decoded offset of the character at document `offset`, or the text's
    /// length at its end, or `None` outside the value.
    pub fn decoded_offset(&self, offset: usize) -> Option<usize> {
        decoded_offset(&self.quoted, offset)
    }
}

/// The decoded offset of the character at document `offset` in `quoted`.
pub fn decoded_offset(quoted: &QuotedExpression, offset: usize) -> Option<usize> {
    let mut last_end = None;
    for (index, character) in quoted.text.char_indices() {
        let range = quoted.authored_range(index..index + character.len_utf8())?;
        if range.start <= offset && offset < range.end {
            return Some(index);
        }
        last_end = Some(range.end);
    }
    (last_end == Some(offset) || (quoted.text.is_empty() && offset == quoted.span.start + 1)).then_some(quoted.text.len())
}

/// Every expression site of every region, each with the scope it is
/// evaluated in under `site` and the quoted text it was decoded from.
pub fn authored_sites<'d>(
    scopes: &BodyScopes,
    scan: &LenientCollectionScan,
    site: &TypeScope<'d>,
) -> Vec<AuthoredSite<'d>> {
    if scan.regions.is_empty() {
        return Vec::new();
    }
    scopes
        .expression_sites(site)
        .into_iter()
        .filter_map(|expression| {
            let region = find_region(&scan.regions, &expression.region)?;
            let quoted = quoted_for(region, expression.role)?.clone();
            Some(AuthoredSite { site: expression, quoted })
        })
        .collect()
}

fn find_region<'r>(regions: &'r [CollectionRegion], span: &Range<usize>) -> Option<&'r CollectionRegion> {
    regions.iter().find_map(|region| {
        if &region.span == span { Some(region) } else { find_region(&region.children, span) }
    })
}

fn quoted_for(region: &CollectionRegion, role: ExpressionRole) -> Option<&QuotedExpression> {
    let header = &region.header;
    match role {
        ExpressionRole::Gate => header.when.as_ref(),
        ExpressionRole::Positional => header.callback.as_ref(),
        ExpressionRole::Filter => header.filter.as_ref(),
        ExpressionRole::Sort => header.sort.as_ref(),
        ExpressionRole::While => header.while_predicate.as_ref(),
        ExpressionRole::Until => header.until_predicate.as_ref(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marker_lines_are_recognized_behind_a_quote_marker() {
        let text = "> ::map items\nplain\n::end-map\n::end-block x\n";
        assert!(is_collection_marker_line(text, 1));
        assert!(is_collection_marker_line(text, 3));
        assert!(!is_collection_marker_line(text, 2));
        assert!(!is_collection_marker_line(text, 4));
    }

    #[test]
    fn empty_spans_widen_by_one_character() {
        assert_eq!(widen("aé", 1..1), 1..3);
        assert_eq!(widen("ab", 2..2), 1..2);
        assert_eq!(widen("ab", 0..1), 0..1);
    }
}
