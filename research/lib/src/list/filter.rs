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
        let glob = Glob::new(pattern).map_err(|e| FilterError::InvalidPattern {
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
                additional_files: vec![],
                missing_underlying: vec![],
                missing_output: vec![],
                missing_metadata: false,
                location: PathBuf::from("/test/foo-library"),
            },
            TopicInfo {
                name: "bar-framework".to_string(),
                topic_type: "framework".to_string(),
                description: Some("A bar framework".to_string()),
                additional_files: vec![],
                missing_underlying: vec![],
                missing_output: vec![],
                missing_metadata: false,
                location: PathBuf::from("/test/bar-framework"),
            },
            TopicInfo {
                name: "baz-software".to_string(),
                topic_type: "software".to_string(),
                description: Some("Baz software".to_string()),
                additional_files: vec![],
                missing_underlying: vec![],
                missing_output: vec![],
                missing_metadata: false,
                location: PathBuf::from("/test/baz-software"),
            },
            TopicInfo {
                name: "foobar-lib".to_string(),
                topic_type: "library".to_string(),
                description: Some("Foobar library".to_string()),
                additional_files: vec![],
                missing_underlying: vec![],
                missing_output: vec![],
                missing_metadata: false,
                location: PathBuf::from("/test/foobar-lib"),
            },
            TopicInfo {
                name: "rust-library".to_string(),
                topic_type: "library".to_string(),
                description: Some("Rust library".to_string()),
                additional_files: vec![],
                missing_underlying: vec![],
                missing_output: vec![],
                missing_metadata: false,
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
        assert!(
            filtered
                .iter()
                .all(|t| t.topic_type == "library" || t.topic_type == "framework")
        );
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
}
