//! Tests for the shared edit-set utility (acceptance A-5, Phase 2 scope):
//! UTF-8 boundary validation, out-of-range and overlap rejection,
//! end-of-source-toward-beginning application, and audit records.

use super::super::{EditRejection, YamlRepair, apply_edit_set};

fn repair(start: usize, end: usize, replacement: &str) -> YamlRepair {
    YamlRepair {
        span: start..end,
        replacement: replacement.to_string(),
        explanation: "test candidate".to_string(),
    }
}

#[test]
fn test_single_edit_applies() {
    let outcome = apply_edit_set("title: @daily-report", &[repair(7, 20, "\"@daily-report\"")]);
    assert_eq!(outcome.source, "title: \"@daily-report\"");
    assert_eq!(outcome.audit.applied.len(), 1);
    assert!(outcome.audit.rejected.is_empty());
    assert!(outcome.changed());
}

#[test]
fn test_no_edits_returns_source_unchanged() {
    let outcome = apply_edit_set("key: value", &[]);
    assert_eq!(outcome.source, "key: value");
    assert!(!outcome.changed());
    assert!(outcome.audit.applied.is_empty());
    assert!(outcome.audit.rejected.is_empty());
}

#[test]
fn test_multiple_edits_apply_end_to_beginning() {
    // Two independent spans; applying end-first keeps the earlier span valid.
    let source = "a: [ 1,2 ]\nb:  x";
    let edits = vec![
        repair(3, 10, "[1, 2]"), // earlier span
        repair(14, 16, " y"),    // later span
    ];
    let outcome = apply_edit_set(source, &edits);
    assert_eq!(outcome.source, "a: [1, 2]\nb:  y");
    assert_eq!(outcome.audit.applied.len(), 2);
}

#[test]
fn test_argument_order_does_not_change_result() {
    // Edits given latest-first must produce the same patched source.
    let source = "a: [ 1,2 ]\nb:  x";
    let outcome = apply_edit_set(
        source,
        &[repair(14, 16, " y"), repair(3, 10, "[1, 2]")],
    );
    assert_eq!(outcome.source, "a: [1, 2]\nb:  y");
}

#[test]
fn test_out_of_range_end_rejected() {
    let outcome = apply_edit_set("abc", &[repair(0, 10, "x")]);
    assert_eq!(outcome.source, "abc");
    assert_eq!(outcome.audit.rejected.len(), 1);
    assert_eq!(outcome.audit.rejected[0].reason, EditRejection::OutOfRange);
}

#[test]
fn test_inverted_span_rejected() {
    let outcome = apply_edit_set("abc", &[repair(2, 1, "x")]);
    assert_eq!(outcome.source, "abc");
    assert_eq!(outcome.audit.rejected[0].reason, EditRejection::OutOfRange);
}

#[test]
fn test_span_past_end_of_empty_source_rejected() {
    let outcome = apply_edit_set("", &[repair(0, 1, "x")]);
    assert_eq!(outcome.audit.rejected[0].reason, EditRejection::OutOfRange);
}

#[test]
fn test_non_utf8_boundary_rejected() {
    // 'é' occupies bytes 2..4; a span ending mid-character must be rejected.
    let source = "abécd";
    let outcome = apply_edit_set(source, &[repair(2, 3, "x")]);
    assert_eq!(outcome.source, source);
    assert_eq!(
        outcome.audit.rejected[0].reason,
        EditRejection::NonUtf8Boundary
    );

    // A span starting mid-character is rejected the same way.
    let outcome = apply_edit_set(source, &[repair(3, 4, "x")]);
    assert_eq!(
        outcome.audit.rejected[0].reason,
        EditRejection::NonUtf8Boundary
    );
}

#[test]
fn test_multibyte_aligned_span_applies() {
    let source = "key: 日本語";
    let outcome = apply_edit_set(source, &[repair(5, 14, "\"日本語\"")]);
    assert_eq!(outcome.source, "key: \"日本語\"");
    assert_eq!(outcome.audit.applied.len(), 1);
}

#[test]
fn test_overlapping_edit_rejected_first_wins() {
    let source = "abcdefghij";
    let outcome = apply_edit_set(source, &[repair(2, 6, "XX"), repair(4, 8, "YY")]);
    assert_eq!(outcome.source, "abXXghij");
    assert_eq!(outcome.audit.applied.len(), 1);
    assert_eq!(outcome.audit.rejected.len(), 1);
    assert_eq!(outcome.audit.rejected[0].reason, EditRejection::Overlap);
    assert_eq!(outcome.audit.rejected[0].repair.span, 4..8);
}

#[test]
fn test_identical_span_rejected_as_overlap() {
    let outcome = apply_edit_set("abc", &[repair(0, 2, "X"), repair(0, 2, "Y")]);
    assert_eq!(outcome.source, "Xc");
    assert_eq!(outcome.audit.rejected[0].reason, EditRejection::Overlap);
}

#[test]
fn test_adjacent_spans_do_not_overlap() {
    // a.end == b.start is not an intersection; both apply.
    let outcome = apply_edit_set("abcd", &[repair(0, 2, "X"), repair(2, 4, "Y")]);
    assert_eq!(outcome.source, "XY");
    assert_eq!(outcome.audit.applied.len(), 2);
}

#[test]
fn test_nested_span_rejected() {
    let outcome = apply_edit_set("abcdef", &[repair(1, 5, "X"), repair(2, 3, "Y")]);
    assert_eq!(outcome.source, "aXf");
    assert_eq!(outcome.audit.rejected[0].reason, EditRejection::Overlap);
}

#[test]
fn test_zero_length_insertion_applies() {
    let outcome = apply_edit_set("key: value", &[repair(10, 10, "\n")]);
    assert_eq!(outcome.source, "key: value\n");
    assert_eq!(outcome.audit.applied.len(), 1);
}

#[test]
fn test_insertion_at_span_edge_is_not_overlap() {
    // Insertion exactly at the start or end of an accepted span is legal.
    let outcome = apply_edit_set("abcd", &[repair(1, 3, "X"), repair(3, 3, "!")]);
    assert_eq!(outcome.source, "aX!d");
    assert_eq!(outcome.audit.applied.len(), 2);
}

#[test]
fn test_two_insertions_same_offset_deterministic() {
    // Both apply; the later candidate's text lands first (documented order).
    let first = repair(2, 2, "A");
    let second = repair(2, 2, "B");
    let outcome = apply_edit_set("xyz", &[first, second]);
    assert_eq!(outcome.source, "xyBAz");
    assert_eq!(outcome.audit.applied.len(), 2);
}

#[test]
fn test_audit_is_in_stable_source_order() {
    // Candidates deliberately unordered; audit lists ascending spans.
    let source = "abcdefghij";
    let outcome = apply_edit_set(
        source,
        &[
            repair(8, 9, "H"),
            repair(0, 30, "too-far"), // out of range, span starts at 0
            repair(4, 5, "E"),
            repair(1, 3, "BC"),
        ],
    );
    let applied_spans: Vec<_> = outcome
        .audit
        .applied
        .iter()
        .map(|r| (r.span.start, r.span.end))
        .collect();
    assert_eq!(applied_spans, vec![(1, 3), (4, 5), (8, 9)]);
    let rejected_spans: Vec<_> = outcome
        .audit
        .rejected
        .iter()
        .map(|r| (r.repair.span.start, r.repair.span.end))
        .collect();
    assert_eq!(rejected_spans, vec![(0, 30)]);
    assert_eq!(outcome.source, "aBCdEfghHj");
}

#[test]
fn test_untouched_bytes_preserved() {
    // Every byte outside an applied span must be identical to the input.
    let source = "one: 1\n two :é\r\nthree: @x\n";
    let edits = vec![repair(23, 25, "\"@x\"")];
    let outcome = apply_edit_set(source, &edits);
    assert_eq!(outcome.source.len(), source.len() + 2);
    assert_eq!(&outcome.source[..23], &source[..23]);
    assert_eq!(&outcome.source[27..], &source[25..]);
    assert_eq!(&outcome.source[23..27], "\"@x\"");
}

#[test]
fn test_crlf_spans_apply_byte_accurately() {
    let source = "a: 1\r\nb: @x\r\n";
    // "b: @x" starts at byte 6; "@x" is bytes 9..11.
    let outcome = apply_edit_set(source, &[repair(9, 11, "\"@x\"")]);
    assert_eq!(outcome.source, "a: 1\r\nb: \"@x\"\r\n");
}
