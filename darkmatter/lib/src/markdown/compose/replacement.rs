//! Text replacement engine for the compose pipeline.
//!
//! This module implements deterministic source-first text replacement using
//! the `replace` map from effective state (merged frontmatter + external state).
//!
//! ## Algorithm
//!
//! The replacement uses a left-to-right scanner with deterministic overlap resolution:
//!
//! 1. Build replacement rules from the `replace` map
//! 2. Filter out empty keys and non-scalar values
//! 3. Sort rules by: key length descending, then key lexicographically ascending
//! 4. Scan content character-by-character, matching longest key at each position
//! 5. Single pass (non-recursive): replacement output is not re-scanned
//!
//! ## Examples
//!
//! ```
//! use darkmatter::markdown::compose::{EffectiveState, EffectiveStateBuilder};
//! use darkmatter::markdown::compose::replacement::apply_replacements;
//! use serde_json::json;
//! use std::collections::HashMap;
//!
//! let mut fm = HashMap::new();
//! fm.insert("replace".to_string(), json!({"foo": "bar", "foobar": "baz"}));
//!
//! let state = EffectiveStateBuilder::new()
//!     .with_frontmatter(fm)
//!     .build();
//!
//! let (result, count) = apply_replacements("foo and foobar", &state);
//! assert_eq!(result, "bar and baz");
//! assert_eq!(count, 2);
//! ```

use super::EffectiveState;
use serde_json::Value;

/// A replacement rule with key and coerced string value.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ReplacementRule {
    key: String,
    key_char_len: usize,
    value: String,
}

/// Applies text replacements to content based on the `replace` map in state.
///
/// Returns the composed content and the number of replacements made.
///
/// ## Rules
///
/// - `replace` must be a map/dictionary; otherwise returns unchanged content
/// - Keys are literal and case-sensitive
/// - Empty-string keys are ignored
/// - Values must be scalars (string, number, boolean, null)
/// - `null` maps to empty string `""`
/// - Non-scalar values (arrays, objects) are ignored
///
/// ## Overlap Resolution
///
/// When multiple keys could match at a position:
/// - Longest key wins
/// - Ties break lexicographically (stable output across runs)
///
/// ## Examples
///
/// ```
/// use darkmatter::markdown::compose::{EffectiveState, EffectiveStateBuilder};
/// use darkmatter::markdown::compose::replacement::apply_replacements;
/// use serde_json::json;
/// use std::collections::HashMap;
///
/// let mut fm = HashMap::new();
/// fm.insert("replace".to_string(), json!({"foo": "bar"}));
///
/// let state = EffectiveStateBuilder::new()
///     .with_frontmatter(fm)
///     .build();
///
/// let (result, count) = apply_replacements("foo is foo", &state);
/// assert_eq!(result, "bar is bar");
/// assert_eq!(count, 2);
/// ```
pub fn apply_replacements(content: &str, state: &EffectiveState) -> (String, usize) {
    // Get replacement rules from state
    let rules = match build_replacement_rules(state) {
        Some(rules) if !rules.is_empty() => rules,
        _ => return (content.to_string(), 0),
    };

    // Apply replacements with single-pass scanner
    scan_and_replace(content, &rules)
}

/// Builds sorted replacement rules from the effective state.
///
/// Returns `None` if `replace` is not a map or is empty.
/// Rules are sorted by key length descending, then key ascending.
fn build_replacement_rules(state: &EffectiveState) -> Option<Vec<ReplacementRule>> {
    let replace_map = state.get_replace_map()?;

    let mut rules: Vec<ReplacementRule> = replace_map
        .iter()
        .filter_map(|(key, value)| {
            // Skip empty keys
            if key.is_empty() {
                return None;
            }

            // Coerce scalar values to string, skip non-scalars
            let string_value = coerce_to_string(value)?;

            Some(ReplacementRule {
                key: key.clone(),
                key_char_len: key.chars().count(),
                value: string_value,
            })
        })
        .collect();

    if rules.is_empty() {
        return None;
    }

    // Sort by key length descending, then key ascending for ties
    rules.sort_by(|a, b| {
        let len_cmp = b.key.len().cmp(&a.key.len());
        if len_cmp == std::cmp::Ordering::Equal {
            a.key.cmp(&b.key)
        } else {
            len_cmp
        }
    });

    Some(rules)
}

/// Coerces a JSON value to a string for replacement.
///
/// - `null` -> `""`
/// - `string` -> the string
/// - `number` -> string representation
/// - `bool` -> `"true"` or `"false"`
/// - `array/object` -> `None` (not supported)
fn coerce_to_string(value: &Value) -> Option<String> {
    match value {
        Value::Null => Some(String::new()),
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        Value::Array(_) | Value::Object(_) => None,
    }
}

/// Scans content and applies replacements in a single pass.
///
/// At each position, tries to match the longest key first (rules are pre-sorted).
/// Replacement output is NOT re-scanned (non-recursive).
fn scan_and_replace(content: &str, rules: &[ReplacementRule]) -> (String, usize) {
    let mut result = String::with_capacity(content.len());
    let mut replacement_count = 0;

    // Char boundary lookup table so we can slice by byte offset without
    // rebuilding a prefix string at each cursor step.
    let mut char_starts: Vec<usize> = content.char_indices().map(|(idx, _)| idx).collect();
    char_starts.push(content.len());
    let content_len = char_starts.len().saturating_sub(1);
    let mut pos = 0;

    while pos < content_len {
        let byte_pos = char_starts[pos];
        let remaining = &content[byte_pos..];

        // Try to match rules in order (longest first due to sorting)
        let matched = rules.iter().find(|rule| remaining.starts_with(&rule.key));

        if let Some(rule) = matched {
            // Found a match - append replacement value
            result.push_str(&rule.value);
            replacement_count += 1;
            // Advance by the number of chars in the key
            pos += rule.key_char_len;
        } else {
            // No match - append current character
            let next_byte_pos = char_starts[pos + 1];
            result.push_str(&content[byte_pos..next_byte_pos]);
            pos += 1;
        }
    }

    (result, replacement_count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::compose::{EffectiveStateBuilder, ComposeContext};
    use serde_json::json;
    use std::collections::HashMap;

    fn test_context() -> ComposeContext {
        ComposeContext::fixed_for_testing()
    }

    fn make_state_with_replace(replace: Value) -> EffectiveState {
        let mut fm = HashMap::new();
        fm.insert("replace".to_string(), replace);

        EffectiveStateBuilder::new()
            .with_frontmatter(fm)
            .with_context(test_context())
            .build()
    }

    // ============================================
    // Basic replacement tests
    // ============================================

    #[test]
    fn test_simple_replacement() {
        let state = make_state_with_replace(json!({"foo": "bar"}));

        let (result, count) = apply_replacements("foo", &state);

        assert_eq!(result, "bar");
        assert_eq!(count, 1);
    }

    #[test]
    fn test_multiple_occurrences() {
        let state = make_state_with_replace(json!({"foo": "bar"}));

        let (result, count) = apply_replacements("foo and foo and foo", &state);

        assert_eq!(result, "bar and bar and bar");
        assert_eq!(count, 3);
    }

    #[test]
    fn test_multiple_rules() {
        let state = make_state_with_replace(json!({"foo": "bar", "baz": "qux"}));

        let (result, count) = apply_replacements("foo and baz", &state);

        assert_eq!(result, "bar and qux");
        assert_eq!(count, 2);
    }

    #[test]
    fn test_no_matches() {
        let state = make_state_with_replace(json!({"foo": "bar"}));

        let (result, count) = apply_replacements("hello world", &state);

        assert_eq!(result, "hello world");
        assert_eq!(count, 0);
    }

    #[test]
    fn test_empty_content() {
        let state = make_state_with_replace(json!({"foo": "bar"}));

        let (result, count) = apply_replacements("", &state);

        assert_eq!(result, "");
        assert_eq!(count, 0);
    }

    // ============================================
    // Overlap resolution tests
    // ============================================

    #[test]
    fn test_overlap_longest_wins() {
        let state = make_state_with_replace(json!({"foo": "short", "foobar": "long"}));

        let (result, count) = apply_replacements("foobar", &state);

        assert_eq!(result, "long");
        assert_eq!(count, 1);
    }

    #[test]
    fn test_overlap_both_match_different_positions() {
        let state = make_state_with_replace(json!({"foo": "1", "foobar": "2"}));

        let (result, count) = apply_replacements("foo and foobar", &state);

        assert_eq!(result, "1 and 2");
        assert_eq!(count, 2);
    }

    #[test]
    fn test_overlap_lexicographic_tiebreaker() {
        // When keys have same length, lexicographically first wins
        let state = make_state_with_replace(json!({"abc": "1", "abd": "2"}));

        // Both have length 3, "abc" < "abd" lexicographically
        // But they don't overlap in content, so both should match their own
        let (result, count) = apply_replacements("abc abd", &state);

        assert_eq!(result, "1 2");
        assert_eq!(count, 2);
    }

    // ============================================
    // Non-recursive replacement test
    // ============================================

    #[test]
    fn test_non_recursive_replacement() {
        // Replacement output should NOT be re-scanned
        let state = make_state_with_replace(json!({"foo": "foobar", "foobar": "baz"}));

        let (result, count) = apply_replacements("foo", &state);

        // "foo" -> "foobar", but "foobar" should NOT then become "baz"
        assert_eq!(result, "foobar");
        assert_eq!(count, 1);
    }

    // ============================================
    // Value type coercion tests
    // ============================================

    #[test]
    fn test_null_value_becomes_empty() {
        let state = make_state_with_replace(json!({"foo": null}));

        let (result, count) = apply_replacements("foo bar", &state);

        assert_eq!(result, " bar");
        assert_eq!(count, 1);
    }

    #[test]
    fn test_number_value_coerced() {
        let state = make_state_with_replace(json!({"num": 42}));

        let (result, count) = apply_replacements("num", &state);

        assert_eq!(result, "42");
        assert_eq!(count, 1);
    }

    #[test]
    fn test_boolean_value_coerced() {
        let state = make_state_with_replace(json!({"yes": true, "no": false}));

        let (result, count) = apply_replacements("yes and no", &state);

        assert_eq!(result, "true and false");
        assert_eq!(count, 2);
    }

    #[test]
    fn test_float_value_coerced() {
        let state = make_state_with_replace(json!({"pi": 3.14159}));

        let (result, count) = apply_replacements("pi", &state);

        assert_eq!(result, "3.14159");
        assert_eq!(count, 1);
    }

    #[test]
    fn test_array_value_ignored() {
        let state = make_state_with_replace(json!({"arr": [1, 2, 3]}));

        let (result, count) = apply_replacements("arr", &state);

        assert_eq!(result, "arr");
        assert_eq!(count, 0);
    }

    #[test]
    fn test_object_value_ignored() {
        let state = make_state_with_replace(json!({"obj": {"nested": "value"}}));

        let (result, count) = apply_replacements("obj", &state);

        assert_eq!(result, "obj");
        assert_eq!(count, 0);
    }

    // ============================================
    // Key validation tests
    // ============================================

    #[test]
    fn test_empty_key_ignored() {
        let state = make_state_with_replace(json!({"": "empty", "foo": "bar"}));

        let (result, count) = apply_replacements("foo", &state);

        assert_eq!(result, "bar");
        assert_eq!(count, 1);
    }

    #[test]
    fn test_case_sensitive_keys() {
        let state = make_state_with_replace(json!({"Foo": "bar"}));

        let (result, count) = apply_replacements("foo Foo FOO", &state);

        // Only "Foo" should match
        assert_eq!(result, "foo bar FOO");
        assert_eq!(count, 1);
    }

    // ============================================
    // Missing/invalid replace map tests
    // ============================================

    #[test]
    fn test_no_replace_key() {
        let fm = HashMap::new();
        let state = EffectiveStateBuilder::new()
            .with_frontmatter(fm)
            .with_context(test_context())
            .build();

        let (result, count) = apply_replacements("foo", &state);

        assert_eq!(result, "foo");
        assert_eq!(count, 0);
    }

    #[test]
    fn test_replace_not_a_map() {
        let state = make_state_with_replace(json!("not a map"));

        let (result, count) = apply_replacements("foo", &state);

        assert_eq!(result, "foo");
        assert_eq!(count, 0);
    }

    #[test]
    fn test_replace_is_array() {
        let state = make_state_with_replace(json!(["foo", "bar"]));

        let (result, count) = apply_replacements("foo", &state);

        assert_eq!(result, "foo");
        assert_eq!(count, 0);
    }

    #[test]
    fn test_replace_is_null() {
        let state = make_state_with_replace(json!(null));

        let (result, count) = apply_replacements("foo", &state);

        assert_eq!(result, "foo");
        assert_eq!(count, 0);
    }

    #[test]
    fn test_empty_replace_map() {
        let state = make_state_with_replace(json!({}));

        let (result, count) = apply_replacements("foo", &state);

        assert_eq!(result, "foo");
        assert_eq!(count, 0);
    }

    // ============================================
    // Unicode handling tests
    // ============================================

    #[test]
    fn test_unicode_content_and_keys() {
        let state = make_state_with_replace(json!({"日本語": "Japanese", "emoji": "🎉"}));

        let (result, count) = apply_replacements("Hello 日本語 and emoji!", &state);

        assert_eq!(result, "Hello Japanese and 🎉!");
        assert_eq!(count, 2);
    }

    #[test]
    fn test_unicode_replacement_values() {
        let state = make_state_with_replace(json!({"hello": "こんにちは", "party": "🎊🎉"}));

        let (result, count) = apply_replacements("hello party", &state);

        assert_eq!(result, "こんにちは 🎊🎉");
        assert_eq!(count, 2);
    }

    #[test]
    fn test_multibyte_character_boundaries() {
        // Ensure we don't break on multibyte character boundaries
        let state = make_state_with_replace(json!({"café": "coffee"}));

        let (result, count) = apply_replacements("I love café", &state);

        assert_eq!(result, "I love coffee");
        assert_eq!(count, 1);
    }

    // ============================================
    // Edge case tests
    // ============================================

    #[test]
    fn test_replacement_at_start() {
        let state = make_state_with_replace(json!({"Hello": "Hi"}));

        let (result, count) = apply_replacements("Hello world", &state);

        assert_eq!(result, "Hi world");
        assert_eq!(count, 1);
    }

    #[test]
    fn test_replacement_at_end() {
        let state = make_state_with_replace(json!({"world": "universe"}));

        let (result, count) = apply_replacements("Hello world", &state);

        assert_eq!(result, "Hello universe");
        assert_eq!(count, 1);
    }

    #[test]
    fn test_adjacent_replacements() {
        let state = make_state_with_replace(json!({"ab": "X", "cd": "Y"}));

        let (result, count) = apply_replacements("abcd", &state);

        assert_eq!(result, "XY");
        assert_eq!(count, 2);
    }

    #[test]
    fn test_replacement_contains_special_chars() {
        let state = make_state_with_replace(json!({"<tag>": "[TAG]", "&amp;": "&"}));

        let (result, count) = apply_replacements("<tag> &amp; more", &state);

        assert_eq!(result, "[TAG] & more");
        assert_eq!(count, 2);
    }

    #[test]
    fn test_key_is_substring_of_another() {
        // "a" is substring of "ab" and "abc"
        let state = make_state_with_replace(json!({"a": "1", "ab": "2", "abc": "3"}));

        let (result, count) = apply_replacements("abc ab a", &state);

        // "abc" matches first (longest), then "ab", then "a"
        assert_eq!(result, "3 2 1");
        assert_eq!(count, 3);
    }

    // ============================================
    // Integration-style tests
    // ============================================

    #[test]
    fn test_realistic_markdown_replacement() {
        let state = make_state_with_replace(json!({
            "PROJECT_NAME": "Darkmatter",
            "VERSION": "1.0.0",
            "AUTHOR": "Test Author"
        }));

        let content = r#"# PROJECT_NAME

Version: VERSION

By: AUTHOR

PROJECT_NAME is a markdown library."#;

        let (result, count) = apply_replacements(content, &state);

        assert!(result.contains("# Darkmatter"));
        assert!(result.contains("Version: 1.0.0"));
        assert!(result.contains("By: Test Author"));
        assert!(result.contains("Darkmatter is a markdown library."));
        assert_eq!(count, 4);
    }

    #[test]
    fn test_mixed_valid_invalid_rules() {
        // Mix of valid and invalid rules
        let state = make_state_with_replace(json!({
            "valid": "yes",
            "": "empty key ignored",
            "array": [1, 2, 3],
            "object": {"nested": true},
            "null": null,
            "number": 42
        }));

        let content = "valid array object null number";

        let (result, count) = apply_replacements(content, &state);

        // "valid" -> "yes", "null" -> "", "number" -> "42"
        // "array" and "object" stay unchanged
        assert_eq!(result, "yes array object  42");
        assert_eq!(count, 3);
    }
}
