//! Parse-count instrumentation (acceptance C-8): clean input parses exactly
//! once, candidate-free input reparses zero times, and candidate-bearing
//! input reparses only for the combined safety proof.

use biscuit_file::{Yaml, analyze_parse_count, analyze_yaml, reset_analyze_parse_count};
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
fn retained_analysis_serves_both_views_from_one_scan() {
    let yaml = Yaml::from_str("key: value  \n").unwrap();

    reset_analyze_parse_count();
    let analysis = yaml.analyze().unwrap();
    let _ = analysis.diagnostics();
    let _ = analysis.repairs().count();
    let _ = analysis.apply();
    let retained = analyze_parse_count();

    // The shorthand pair scans the same source once each, so it costs a
    // second analysis the retained-analysis path never pays for.
    reset_analyze_parse_count();
    let _ = yaml.diagnose();
    let _ = yaml.repair_candidates();
    assert_eq!(analyze_parse_count(), retained * 2);
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
