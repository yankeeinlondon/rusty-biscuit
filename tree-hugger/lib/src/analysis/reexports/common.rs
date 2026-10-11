//! Helpers every language extractor shares: entry construction, report
//! assembly and the whole-file limit.

use tree_sitter::Node;

use crate::file::extract::text_span_for_node;
use crate::shared::{
    ReexportCoverage, ReexportCoverageState, ReexportEntry, ReexportEvidence, ReexportForm,
    ReexportLimit, ReexportLimitReason, ReexportReport, TextSpan,
};

/// An entry with every optional role empty; callers set what the construct
/// states. `span` is the specifier or clause, `statement_span` the whole
/// forwarding statement.
pub(super) fn entry(
    form: ReexportForm,
    evidence: ReexportEvidence,
    span: TextSpan,
    statement_span: TextSpan,
) -> ReexportEntry {
    ReexportEntry {
        form,
        evidence,
        exported_name: None,
        original_name: None,
        local_name: None,
        source: None,
        excluded_names: Vec::new(),
        scope: None,
        restriction: None,
        type_only: false,
        conditional: false,
        caveats: Vec::new(),
        import_span: None,
        span,
        statement_span,
    }
}

/// A limit over `node`'s span.
pub(super) fn limit(reason: ReexportLimitReason, node: Node<'_>) -> ReexportLimit {
    ReexportLimit { reason, span: text_span_for_node(node) }
}

/// The report for `entries` and `limits`: entries in source order with one
/// entry per `span`, limits ascending by (`span.start_byte`, `reason`) with
/// exact duplicates removed, and the state `limited` exactly when a limit
/// exists.
pub(super) fn finish(mut entries: Vec<ReexportEntry>, mut limits: Vec<ReexportLimit>) -> ReexportReport {
    entries.sort_by_key(|entry| (entry.span.start_byte, entry.span.end_byte));
    entries.dedup_by(|later, earlier| later.span == earlier.span);
    for entry in &mut entries {
        entry.caveats.sort();
        entry.caveats.dedup();
    }
    limits.sort_by(|a, b| {
        (a.span.start_byte, a.reason, a.span.end_byte).cmp(&(b.span.start_byte, b.reason, b.span.end_byte))
    });
    limits.dedup();
    let state = if limits.is_empty() { ReexportCoverageState::Complete } else { ReexportCoverageState::Limited };
    ReexportReport { coverage: ReexportCoverage { state, limits }, entries }
}

/// Every `ERROR` node and `MISSING` node under `root`, outermost first and
/// without descending into one. Subtrees that parsed cleanly are skipped.
pub(super) fn damaged_nodes(root: Node<'_>) -> Vec<Node<'_>> {
    let mut found = Vec::new();
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        if !node.has_error() && !node.is_missing() {
            continue;
        }
        if node.is_error() || node.is_missing() {
            found.push(node);
            continue;
        }
        let mut cursor = node.walk();
        stack.extend(node.children(&mut cursor));
    }
    found.sort_by_key(|node| node.start_byte());
    found
}
