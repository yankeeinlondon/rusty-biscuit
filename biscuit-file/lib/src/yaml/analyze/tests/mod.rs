mod analysis;
mod anchors;
mod classification_gate;
mod diagnostic;
mod duplicate_keys;
mod edit_set;
mod line_endings;
mod lints;
mod locate;
mod multi_document;
mod normalization;
mod reserved_indicator;
mod scan;
mod whitespace;

/// Reconstructs the expected output solely from accepted repair spans and
/// asserts every untouched byte is identical to the original (the
/// source-preservation invariant).
pub(crate) fn assert_untouched_bytes_preserved(
    original: &str,
    outcome: &super::EditSetOutcome,
) {
    let mut expected = String::with_capacity(outcome.source.len());
    let mut cursor = 0;
    for repair in &outcome.audit.applied {
        expected.push_str(&original[cursor..repair.span.start]);
        expected.push_str(&repair.replacement);
        cursor = repair.span.end;
    }
    expected.push_str(&original[cursor..]);
    assert_eq!(
        outcome.source, expected,
        "output must be reconstructable from accepted spans and original bytes only"
    );
}
