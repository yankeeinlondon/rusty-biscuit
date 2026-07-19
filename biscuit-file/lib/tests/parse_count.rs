//! Parse-count instrumentation (acceptance C-8): clean input parses exactly
//! once, candidate-free input reparses zero times, and candidate-bearing
//! input reparses only for the combined safety proof.

use biscuit_file::{analyze_parse_count, analyze_yaml, reset_analyze_parse_count};
use serial_test::serial;

#[test]
#[serial]
fn clean_input_parses_once() {
    reset_analyze_parse_count();
    let analysis = analyze_yaml("key: value\n");
    assert!(analysis.is_clean());
    assert_eq!(analyze_parse_count(), 1);
}

#[test]
#[serial]
fn candidate_free_input_reparses_zero_times() {
    // Trailing whitespace inside a block scalar is scalar content, so no
    // candidate exists and the document is never reparsed.
    reset_analyze_parse_count();
    let source = "script: |\n  echo hi  \n";
    let analysis = analyze_yaml(source);
    assert!(analysis.is_clean());
    assert_eq!(analyze_parse_count(), 1);
}

#[test]
#[serial]
fn candidate_bearing_input_reparses_once_for_combined_proof() {
    reset_analyze_parse_count();
    let analysis = analyze_yaml("key: value  \n");
    assert!(!analysis.is_clean());
    assert_eq!(analyze_parse_count(), 2);
}

#[test]
#[serial]
fn unparseable_flagship_parses_original_and_candidate() {
    reset_analyze_parse_count();
    let analysis = analyze_yaml("title: @daily-report");
    assert!(!analysis.is_parseable());
    assert_eq!(analyze_parse_count(), 2);
}

#[test]
#[serial]
fn multi_candidate_input_proves_combined_in_one_reparse() {
    reset_analyze_parse_count();
    let analysis = analyze_yaml("\u{FEFF}key :  [ 80,443 ]  \r\n");
    assert!(!analysis.is_clean());
    // One initial parse plus one combined-proof reparse.
    assert_eq!(analyze_parse_count(), 2);
}
