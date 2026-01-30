//! Filtering logic for research topics.
//!
//! This module provides filtering functionality for discovered research topics,
//! supporting glob pattern matching and type filtering.

use crate::list::types::TopicInfo;
use globset::{Glob, GlobSetBuilder};
use thiserror::Error;

/// Errors that can occur during filtering operations.
#[derive(Debug, Error)]
pub enum FilterError {
    /// Invalid glob pattern provided.
    #[error("Invalid glob pattern: {pattern}")]
    InvalidPattern {
        /// The invalid pattern string.
        pattern: String,
        /// The underlying error from globset.
        #[source]
        source: globset::Error,
    },
}

/// Applies filtering to a list of topics based on patterns and types.
///
/// # Arguments
///
/// * `topics` - The list of topics to filter
/// * `patterns` - Glob patterns to match against topic names (OR logic)
/// * `types` - Types to match against topic types (OR logic)
///
/// # Returns
///
/// A filtered list of topics that match at least one pattern (if patterns provided)
/// AND at least one type (if types provided).
///
/// # Examples
///
/// ```
/// use research_lib::list::filter::apply_filters;
/// use research_lib::list::types::TopicInfo;
/// use std::path::PathBuf;
///
/// let topics = vec![
///     TopicInfo::new("foo-lib".to_string(), PathBuf::from("/test/foo-lib")),
///     TopicInfo::new("bar-lib".to_string(), PathBuf::from("/test/bar-lib")),
/// ];
///
/// // Filter by pattern
/// let filtered = apply_filters(topics.clone(), &["foo*".to_string()], &[]).unwrap();
/// assert_eq!(filtered.len(), 1);
/// assert_eq!(filtered[0].name, "foo-lib");
/// ```
pub fn apply_filters(
    topics: Vec<TopicInfo>,
    patterns: &[String],
    types: &[String],
) -> Result<Vec<TopicInfo>, FilterError> {
    // If no filters provided, return all topics
    if patterns.is_empty() && types.is_empty() {
        return Ok(topics);
    }

    // Build glob matcher for patterns (case-insensitive)
    let pattern_matcher = if !patterns.is_empty() {
        Some(build_glob_matcher(patterns)?)
    } else {
        None
    };

    // Convert types to lowercase for case-insensitive matching
    let normalized_types: Vec<String> = types.iter().map(|t| t.to_lowercase()).collect();

    // Filter topics
    let filtered = topics
        .into_iter()
        .filter(|topic| {
            // Check pattern match (OR logic: match any pattern)
            let pattern_match = if let Some(ref matcher) = pattern_matcher {
                matcher.is_match(&topic.name)
            } else {
                true // No pattern filter, so pass
            };

            // Check type match (OR logic: match any type)
            let type_match = if !normalized_types.is_empty() {
                normalized_types.contains(&topic.topic_type.to_lowercase())
            } else {
                true // No type filter, so pass
            };

            // Both filters must pass (AND logic between filter categories)
            pattern_match && type_match
        })
        .collect();

    Ok(filtered)
}

/// Builds a case-insensitive glob matcher from a list of patterns.
///
/// Patterns without glob metacharacters (`*`, `?`, `[`, `]`, `{`, `}`) are
/// automatically wrapped in `*...*` to enable substring matching.
///
/// ## Examples
///
/// - `"wire"` → `"*wire*"` (matches any topic containing "wire")
/// - `"wire*"` → `"wire*"` (matches topics starting with "wire")
/// - `"*wire"` → `"*wire"` (matches topics ending with "wire")
///
/// # Arguments
///
/// * `patterns` - The glob patterns to compile
///
/// # Returns
///
/// A compiled GlobSet that can efficiently match against multiple patterns.
///
/// # Errors
///
/// Returns `FilterError::InvalidPattern` if any pattern is invalid.
fn build_glob_matcher(patterns: &[String]) -> Result<globset::GlobSet, FilterError> {
    let mut builder = GlobSetBuilder::new();

    for pattern in patterns {
        // Auto-wrap patterns without glob metacharacters in *...*
        let normalized_pattern = if has_glob_metacharacters(pattern) {
            pattern.clone()
        } else {
            format!("*{}*", pattern)
        };

        let glob = Glob::new(&normalized_pattern).map_err(|e| FilterError::InvalidPattern {
            pattern: pattern.clone(),
            source: e,
        })?;

        // Add case-insensitive version
        builder.add(glob);
    }

    builder.build().map_err(|e| FilterError::InvalidPattern {
        pattern: "multiple patterns".to_string(),
        source: e,
    })
}

/// Checks if a pattern contains glob metacharacters.
///
/// Returns `true` if the pattern contains any of: `*`, `?`, `[`, `]`, `{`, `}`
fn has_glob_metacharacters(pattern: &str) -> bool {
    pattern
        .chars()
        .any(|c| matches!(c, '*' | '?' | '[' | ']' | '{' | '}'))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn create_test_topics() -> Vec<TopicInfo> {
        vec![
            TopicInfo {
                name: "foo-library".to_string(),
                topic_type: "library".to_string(),
                description: Some("A foo library".to_string()),
                language: None,
                additional_files: vec![],
                missing_underlying: vec![],
                missing_output: vec![],
                needs_migration: false,
                location: PathBuf::from("/test/foo-library"),
            },
            TopicInfo {
                name: "bar-framework".to_string(),
                topic_type: "framework".to_string(),
                description: Some("A bar framework".to_string()),
                language: None,
                additional_files: vec![],
                missing_underlying: vec![],
                missing_output: vec![],
                needs_migration: false,
                location: PathBuf::from("/test/bar-framework"),
            },
            TopicInfo {
                name: "baz-software".to_string(),
                topic_type: "software".to_string(),
                description: Some("Baz software".to_string()),
                language: None,
                additional_files: vec![],
                missing_underlying: vec![],
                missing_output: vec![],
                needs_migration: false,
                location: PathBuf::from("/test/baz-software"),
            },
            TopicInfo {
                name: "foobar-lib".to_string(),
                topic_type: "library".to_string(),
                description: Some("Foobar library".to_string()),
                language: None,
                additional_files: vec![],
                missing_underlying: vec![],
                missing_output: vec![],
                needs_migration: false,
                location: PathBuf::from("/test/foobar-lib"),
            },
            TopicInfo {
                name: "rust-library".to_string(),
                topic_type: "library".to_string(),
                description: Some("Rust library".to_string()),
                language: None,
                additional_files: vec![],
                missing_underlying: vec![],
                missing_output: vec![],
                needs_migration: false,
                location: PathBuf::from("/test/rust-library"),
            },
        ]
    }

    #[test]
    fn test_no_filters_returns_all() {
        let topics = create_test_topics();
        let count = topics.len();
        let filtered = apply_filters(topics, &[], &[]).unwrap();
        assert_eq!(filtered.len(), count);
    }

    #[test]
    fn test_single_pattern_match() {
        let topics = create_test_topics();
        let filtered = apply_filters(topics, &["foo-library".to_string()], &[]).unwrap();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].name, "foo-library");
    }

    #[test]
    fn test_multiple_pattern_match() {
        let topics = create_test_topics();
        let filtered = apply_filters(
            topics,
            &["foo-library".to_string(), "bar-framework".to_string()],
            &[],
        )
        .unwrap();
        assert_eq!(filtered.len(), 2);
        assert!(filtered.iter().any(|t| t.name == "foo-library"));
        assert!(filtered.iter().any(|t| t.name == "bar-framework"));
    }

    #[test]
    fn test_wildcard_pattern_prefix() {
        let topics = create_test_topics();
        let filtered = apply_filters(topics, &["foo*".to_string()], &[]).unwrap();
        assert_eq!(filtered.len(), 2); // foo-library, foobar-lib
        assert!(filtered.iter().any(|t| t.name == "foo-library"));
        assert!(filtered.iter().any(|t| t.name == "foobar-lib"));
    }

    #[test]
    fn test_wildcard_pattern_suffix() {
        let topics = create_test_topics();
        let filtered = apply_filters(topics, &["*library".to_string()], &[]).unwrap();
        assert_eq!(filtered.len(), 2); // foo-library, rust-library
        assert!(filtered.iter().any(|t| t.name == "foo-library"));
        assert!(filtered.iter().any(|t| t.name == "rust-library"));
    }

    #[test]
    fn test_wildcard_pattern_contains() {
        let topics = create_test_topics();
        let filtered = apply_filters(topics, &["*bar*".to_string()], &[]).unwrap();
        assert_eq!(filtered.len(), 2); // bar-framework, foobar-lib
        assert!(filtered.iter().any(|t| t.name == "bar-framework"));
        assert!(filtered.iter().any(|t| t.name == "foobar-lib"));
    }

    #[test]
    fn test_single_type_filter() {
        let topics = create_test_topics();
        let filtered = apply_filters(topics, &[], &["library".to_string()]).unwrap();
        assert_eq!(filtered.len(), 3); // foo-library, foobar-lib, rust-library
        assert!(filtered.iter().all(|t| t.topic_type == "library"));
    }

    #[test]
    fn test_multiple_type_filter() {
        let topics = create_test_topics();
        let filtered = apply_filters(
            topics,
            &[],
            &["library".to_string(), "framework".to_string()],
        )
        .unwrap();
        assert_eq!(filtered.len(), 4); // All except baz-software
        assert!(filtered
            .iter()
            .all(|t| t.topic_type == "library" || t.topic_type == "framework"));
    }

    #[test]
    fn test_combined_pattern_and_type_filter() {
        let topics = create_test_topics();
        let filtered =
            apply_filters(topics, &["foo*".to_string()], &["library".to_string()]).unwrap();
        assert_eq!(filtered.len(), 2); // foo-library, foobar-lib (both match pattern and type)
        assert!(filtered.iter().any(|t| t.name == "foo-library"));
        assert!(filtered.iter().any(|t| t.name == "foobar-lib"));
    }

    #[test]
    fn test_pattern_no_matches() {
        let topics = create_test_topics();
        let filtered = apply_filters(topics, &["nonexistent*".to_string()], &[]).unwrap();
        assert_eq!(filtered.len(), 0);
    }

    #[test]
    fn test_type_no_matches() {
        let topics = create_test_topics();
        let filtered = apply_filters(topics, &[], &["nonexistent".to_string()]).unwrap();
        assert_eq!(filtered.len(), 0);
    }

    #[test]
    fn test_case_insensitive_type_matching() {
        let topics = create_test_topics();
        let filtered = apply_filters(topics, &[], &["LIBRARY".to_string()]).unwrap();
        assert_eq!(filtered.len(), 3); // Should match despite case difference
        assert!(filtered.iter().all(|t| t.topic_type == "library"));
    }

    #[test]
    fn test_multiple_wildcard_patterns() {
        let topics = create_test_topics();
        let filtered =
            apply_filters(topics, &["foo*".to_string(), "*framework".to_string()], &[]).unwrap();
        assert_eq!(filtered.len(), 3); // foo-library, foobar-lib, bar-framework
        assert!(filtered.iter().any(|t| t.name == "foo-library"));
        assert!(filtered.iter().any(|t| t.name == "foobar-lib"));
        assert!(filtered.iter().any(|t| t.name == "bar-framework"));
    }

    #[test]
    fn test_invalid_glob_pattern() {
        let topics = create_test_topics();
        let result = apply_filters(topics, &["[invalid".to_string()], &[]);
        assert!(result.is_err());
        match result {
            Err(FilterError::InvalidPattern { pattern, .. }) => {
                assert_eq!(pattern, "[invalid");
            }
            _ => panic!("Expected InvalidPattern error"),
        }
    }

    #[test]
    fn test_combined_filter_no_matches() {
        let topics = create_test_topics();
        // Pattern matches but type doesn't
        let filtered =
            apply_filters(topics, &["foo*".to_string()], &["software".to_string()]).unwrap();
        assert_eq!(filtered.len(), 0);
    }

    // Regression tests for substring matching without wildcards (bug fix)
    #[test]
    fn test_substring_match_without_wildcards() {
        // Regression test: `research list wire` should match topics containing "wire"
        let topics = create_test_topics();
        let filtered = apply_filters(topics, &["lib".to_string()], &[]).unwrap();
        // Should match: foo-library, foobar-lib, rust-library (all contain "lib")
        assert_eq!(filtered.len(), 3);
        assert!(filtered.iter().any(|t| t.name == "foo-library"));
        assert!(filtered.iter().any(|t| t.name == "foobar-lib"));
        assert!(filtered.iter().any(|t| t.name == "rust-library"));
    }

    #[test]
    fn test_substring_match_single_word() {
        // Regression test: Plain word should match as substring
        let topics = create_test_topics();
        let filtered = apply_filters(topics, &["bar".to_string()], &[]).unwrap();
        // Should match: bar-framework, foobar-lib (both contain "bar")
        assert_eq!(filtered.len(), 2);
        assert!(filtered.iter().any(|t| t.name == "bar-framework"));
        assert!(filtered.iter().any(|t| t.name == "foobar-lib"));
    }

    #[test]
    fn test_explicit_wildcard_not_wrapped() {
        // Ensure patterns with wildcards are NOT double-wrapped
        let topics = create_test_topics();
        let filtered = apply_filters(topics, &["foo*".to_string()], &[]).unwrap();
        // Should match: foo-library, foobar-lib (starts with "foo")
        assert_eq!(filtered.len(), 2);
        assert!(filtered.iter().any(|t| t.name == "foo-library"));
        assert!(filtered.iter().any(|t| t.name == "foobar-lib"));
    }

    #[test]
    fn test_suffix_wildcard_not_wrapped() {
        // Ensure suffix wildcards work correctly
        let topics = create_test_topics();
        let filtered = apply_filters(topics, &["*framework".to_string()], &[]).unwrap();
        // Should match: bar-framework (ends with "framework")
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].name, "bar-framework");
    }

    #[test]
    fn test_exact_match_with_wildcards_around() {
        // User provides explicit wildcards for substring matching
        let topics = create_test_topics();
        let filtered = apply_filters(topics, &["*rust*".to_string()], &[]).unwrap();
        // Should match: rust-library
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].name, "rust-library");
    }

    #[test]
    fn test_has_glob_metacharacters() {
        // Test the helper function
        assert!(!has_glob_metacharacters("wire"));
        assert!(!has_glob_metacharacters("foo-bar"));
        assert!(has_glob_metacharacters("wire*"));
        assert!(has_glob_metacharacters("*wire"));
        assert!(has_glob_metacharacters("wi?re"));
        assert!(has_glob_metacharacters("[wire]"));
        assert!(has_glob_metacharacters("{wire}"));
        assert!(has_glob_metacharacters("wire[123]"));
    }
}
