//! Integration tests for the list command.
//!
//! This test suite verifies end-to-end functionality of the list command,
//! including discovery, filtering, and formatting.

use research_lib::list::{
    apply_filters, discover_topics, format_json, format_terminal, ResearchOutput,
};
use std::path::PathBuf;

/// Returns the path to the test fixtures directory.
fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/.research/library")
}

#[test]
fn test_discover_all_topics() {
    let topics = discover_topics(fixtures_dir()).expect("Failed to discover topics");

    // We have 4 fixture topics: complete-topic, incomplete-topic, corrupt-metadata, rust-analyzer
    assert_eq!(topics.len(), 4, "Should discover exactly 4 topics");

    // Verify topic names are present
    let names: Vec<&str> = topics.iter().map(|t| t.name.as_str()).collect();
    assert!(names.contains(&"complete-topic"));
    assert!(names.contains(&"incomplete-topic"));
    assert!(names.contains(&"corrupt-metadata"));
    assert!(names.contains(&"rust-analyzer"));
}

#[test]
fn test_complete_topic_has_no_issues() {
    let topics = discover_topics(fixtures_dir()).expect("Failed to discover topics");

    let complete = topics
        .iter()
        .find(|t| t.name == "complete-topic")
        .expect("complete-topic not found");

    assert_eq!(complete.topic_type, "library");
    assert_eq!(
        complete.description,
        Some("A complete test library with all files present".to_string())
    );
    assert!(
        complete.missing_output.is_empty(),
        "Should have all outputs"
    );
    assert!(
        complete.missing_underlying.is_empty(),
        "Should have all underlying docs"
    );
    assert!(!complete.has_issues(), "Should have no issues");
}

#[test]
fn test_incomplete_topic_has_critical_issues() {
    let topics = discover_topics(fixtures_dir()).expect("Failed to discover topics");

    let incomplete = topics
        .iter()
        .find(|t| t.name == "incomplete-topic")
        .expect("incomplete-topic not found");

    assert_eq!(incomplete.topic_type, "framework");
    assert_eq!(
        incomplete.description,
        Some("An incomplete test framework with missing files".to_string())
    );
    // Should be missing all output deliverables
    assert_eq!(
        incomplete.missing_output.len(),
        3,
        "Should be missing all 3 outputs"
    );
    assert!(incomplete
        .missing_output
        .contains(&ResearchOutput::DeepDive));
    assert!(incomplete.missing_output.contains(&ResearchOutput::Brief));
    assert!(incomplete.missing_output.contains(&ResearchOutput::Skill));

    // Should be missing most underlying docs (we only created overview.md)
    assert!(
        incomplete.missing_underlying.len() >= 4,
        "Should be missing at least 4 underlying docs"
    );

    assert!(
        incomplete.has_critical_issues(),
        "Should have critical issues"
    );
    assert!(incomplete.has_issues(), "Should have issues");
}

#[test]
fn test_corrupt_metadata_topic_marked_as_missing() {
    let topics = discover_topics(fixtures_dir()).expect("Failed to discover topics");

    let corrupt = topics
        .iter()
        .find(|t| t.name == "corrupt-metadata")
        .expect("corrupt-metadata not found");

    assert_eq!(
        corrupt.topic_type, "library",
        "Should use default type when metadata missing"
    );
    assert_eq!(corrupt.description, None, "Should have no description");
    assert!(
        corrupt.has_critical_issues(),
        "Should have critical issues due to missing metadata"
    );
}

#[test]
fn test_filter_by_single_pattern() {
    let topics = discover_topics(fixtures_dir()).expect("Failed to discover topics");

    let patterns = vec!["complete*".to_string()];
    let filtered = apply_filters(topics, &patterns, &[]).expect("Failed to apply filters");

    assert_eq!(filtered.len(), 1, "Should match exactly 1 topic");
    assert_eq!(filtered[0].name, "complete-topic");
}

#[test]
fn test_filter_by_multiple_patterns() {
    let topics = discover_topics(fixtures_dir()).expect("Failed to discover topics");

    let patterns = vec!["complete*".to_string(), "rust-*".to_string()];
    let filtered = apply_filters(topics, &patterns, &[]).expect("Failed to apply filters");

    assert_eq!(filtered.len(), 2, "Should match 2 topics");

    let names: Vec<&str> = filtered.iter().map(|t| t.name.as_str()).collect();
    assert!(names.contains(&"complete-topic"));
    assert!(names.contains(&"rust-analyzer"));
}

#[test]
fn test_filter_by_type() {
    let topics = discover_topics(fixtures_dir()).expect("Failed to discover topics");

    let types = vec!["framework".to_string()];
    let filtered = apply_filters(topics, &[], &types).expect("Failed to apply filters");

    assert_eq!(filtered.len(), 1, "Should match exactly 1 framework");
    assert_eq!(filtered[0].name, "incomplete-topic");
    assert_eq!(filtered[0].topic_type, "framework");
}

#[test]
fn test_filter_by_multiple_types() {
    let topics = discover_topics(fixtures_dir()).expect("Failed to discover topics");

    let types = vec!["library".to_string(), "framework".to_string()];
    let filtered = apply_filters(topics, &[], &types).expect("Failed to apply filters");

    // Should match: complete-topic (library), incomplete-topic (framework),
    // corrupt-metadata (library default), rust-analyzer (library)
    assert_eq!(filtered.len(), 4, "Should match all topics");
}

#[test]
fn test_filter_by_pattern_and_type() {
    let topics = discover_topics(fixtures_dir()).expect("Failed to discover topics");

    let patterns = vec!["*topic".to_string()];
    let types = vec!["library".to_string()];
    let filtered = apply_filters(topics, &patterns, &types).expect("Failed to apply filters");

    // Should match: complete-topic (library)
    // Should NOT match: incomplete-topic (framework - wrong type),
    //                   corrupt-metadata (library but doesn't end with "topic"),
    //                   rust-analyzer (library but no "topic" in name)
    assert_eq!(filtered.len(), 1, "Should match 1 topic");
    assert_eq!(filtered[0].name, "complete-topic");
}

#[test]
fn test_filter_empty_result() {
    let topics = discover_topics(fixtures_dir()).expect("Failed to discover topics");

    let patterns = vec!["nonexistent*".to_string()];
    let filtered = apply_filters(topics, &patterns, &[]).expect("Failed to apply filters");

    assert_eq!(filtered.len(), 0, "Should match no topics");
}

#[test]
fn test_json_output_format() {
    let topics = discover_topics(fixtures_dir()).expect("Failed to discover topics");

    let json = format_json(&topics).expect("Failed to format as JSON");

    // Basic JSON validation
    assert!(
        json.starts_with('['),
        "JSON should start with array bracket"
    );
    assert!(json.ends_with(']'), "JSON should end with array bracket");
    assert!(
        json.contains("complete-topic"),
        "JSON should contain topic name"
    );
    assert!(json.contains("library"), "JSON should contain topic type");

    // Verify it can be parsed back
    let parsed: Vec<serde_json::Value> = serde_json::from_str(&json).expect("JSON should be valid");
    assert_eq!(parsed.len(), 4, "Should have 4 topics in JSON");
}

#[test]
fn test_terminal_output_format() {
    let topics = discover_topics(fixtures_dir()).expect("Failed to discover topics");

    let output = format_terminal(&topics, false, true);

    // Verify topic names appear in output
    assert!(
        output.contains("complete-topic"),
        "Output should contain complete-topic"
    );
    assert!(
        output.contains("incomplete-topic"),
        "Output should contain incomplete-topic"
    );
    assert!(
        output.contains("rust-analyzer"),
        "Output should contain rust-analyzer"
    );

    // Verify it's not empty
    assert!(!output.is_empty(), "Terminal output should not be empty");
}

#[test]
fn test_terminal_output_empty_topics() {
    let topics = Vec::new();
    let output = format_terminal(&topics, false, true);

    assert!(
        output.is_empty(),
        "Empty topic list should produce empty output"
    );
}

#[test]
fn test_discover_nonexistent_directory() {
    let result = discover_topics(PathBuf::from("/nonexistent/directory/path"));

    assert!(
        result.is_err(),
        "Should return error for nonexistent directory"
    );
}

#[test]
fn test_no_filters_returns_all() {
    let topics = discover_topics(fixtures_dir()).expect("Failed to discover topics");
    let original_count = topics.len();

    let filtered = apply_filters(topics, &[], &[]).expect("Failed to apply filters");

    assert_eq!(
        filtered.len(),
        original_count,
        "No filters should return all topics"
    );
}

#[test]
fn test_pattern_matching_is_case_sensitive() {
    let topics = discover_topics(fixtures_dir()).expect("Failed to discover topics");

    // Test with uppercase pattern - should NOT match lowercase topic name
    let patterns = vec!["COMPLETE*".to_string()];
    let filtered = apply_filters(topics.clone(), &patterns, &[]).expect("Failed to apply filters");

    assert_eq!(
        filtered.len(),
        0,
        "Pattern matching is case-sensitive, so uppercase pattern should not match"
    );

    // Test with correct case
    let patterns_correct = vec!["complete*".to_string()];
    let filtered_correct =
        apply_filters(topics, &patterns_correct, &[]).expect("Failed to apply filters");

    assert_eq!(
        filtered_correct.len(),
        1,
        "Pattern with correct case should match"
    );
    assert_eq!(filtered_correct[0].name, "complete-topic");
}

#[test]
fn test_case_insensitive_type_matching() {
    let topics = discover_topics(fixtures_dir()).expect("Failed to discover topics");

    // Test with uppercase type
    let types = vec!["FRAMEWORK".to_string()];
    let filtered = apply_filters(topics, &[], &types).expect("Failed to apply filters");

    assert_eq!(
        filtered.len(),
        1,
        "Type matching should be case-insensitive"
    );
    assert_eq!(filtered[0].name, "incomplete-topic");
}
