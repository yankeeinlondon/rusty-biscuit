//! Free functions for building [`ChoiceInput<String>`] values from
//! common string inputs.
//!
//! Callers frequently want to build a list of options from a literal
//! CSV, a markdown bullet list, or a key/value dictionary supplied by
//! the user. These helpers take that repetitive work off the caller.
//!
//! ## Examples
//!
//! ```
//! use tui_chrome::helpers::choice_builders::choose_one_from_csv;
//!
//! let input = choose_one_from_csv("colour", "Pick a colour", "Red, Green , Blue");
//! assert_eq!(input.options.len(), 3);
//! assert_eq!(input.options[0].label, "Red");
//! assert_eq!(input.options[0].value, "Red");
//! ```

use thiserror::Error;

use crate::components::choose::{ChoiceInput, ChoiceOption, SelectionMode};

/// Errors produced by dictionary-based choice builders.
#[derive(Debug, Error)]
pub enum ChoiceBuilderError {
    /// The input was not valid YAML/JSON.
    #[error("failed to parse dictionary: {0}")]
    Parse(String),

    /// The top-level value was not an object/mapping.
    #[error("dictionary input must be a mapping/object")]
    NotAMapping,
}

/// Builds a single-select input from a comma-separated string.
///
/// Each CSV entry is trimmed. Empty entries are dropped. The option
/// `id`, `label`, and `value` are all set to the trimmed entry.
pub fn choose_one_from_csv(
    id: impl Into<String>,
    prompt: impl Into<String>,
    csv: &str,
) -> ChoiceInput<String> {
    ChoiceInput::new(id, prompt).with_options(choice_options_from_csv(csv))
}

/// Builds a multi-select input from a comma-separated string.
///
/// Semantics match [`choose_one_from_csv`] except that the selection
/// mode is [`SelectionMode::Multiple`].
pub fn choose_many_from_csv(
    id: impl Into<String>,
    prompt: impl Into<String>,
    csv: &str,
) -> ChoiceInput<String> {
    ChoiceInput::new(id, prompt)
        .with_selection_mode(SelectionMode::Multiple)
        .with_options(choice_options_from_csv(csv))
}

/// Builds a single-select input from a markdown bullet/numbered list.
///
/// Accepted bullet markers: `-`, `*`, `+`. Accepted ordered-list
/// markers: `N.` and `N)` for any integer `N`. Indentation is ignored;
/// non-matching lines are skipped. Each item's text becomes both the
/// `label` and `value`.
pub fn choose_one_from_markdown_list(
    id: impl Into<String>,
    prompt: impl Into<String>,
    markdown: &str,
) -> ChoiceInput<String> {
    ChoiceInput::new(id, prompt).with_options(choice_options_from_markdown_list(markdown))
}

/// Builds a multi-select input from a markdown bullet/numbered list.
pub fn choose_many_from_markdown_list(
    id: impl Into<String>,
    prompt: impl Into<String>,
    markdown: &str,
) -> ChoiceInput<String> {
    ChoiceInput::new(id, prompt)
        .with_selection_mode(SelectionMode::Multiple)
        .with_options(choice_options_from_markdown_list(markdown))
}

/// Builds a single-select input from a YAML or JSON mapping.
///
/// Each key becomes the option `label`; each corresponding value is
/// stringified and becomes the option `value`. The `id` is the key as
/// supplied. Insertion order is preserved via `serde_yaml_ng`'s
/// ordered mapping representation.
///
/// ## Errors
///
/// Returns [`ChoiceBuilderError::Parse`] when the input is not valid
/// YAML/JSON. Returns [`ChoiceBuilderError::NotAMapping`] when the
/// top-level value is not a mapping/object.
pub fn choose_one_from_dictionary(
    id: impl Into<String>,
    prompt: impl Into<String>,
    yaml_or_json: &str,
) -> Result<ChoiceInput<String>, ChoiceBuilderError> {
    Ok(ChoiceInput::new(id, prompt).with_options(choice_options_from_dictionary(yaml_or_json)?))
}

/// Builds a multi-select input from a YAML or JSON mapping.
///
/// Semantics match [`choose_one_from_dictionary`] except that the
/// selection mode is [`SelectionMode::Multiple`].
pub fn choose_many_from_dictionary(
    id: impl Into<String>,
    prompt: impl Into<String>,
    yaml_or_json: &str,
) -> Result<ChoiceInput<String>, ChoiceBuilderError> {
    Ok(ChoiceInput::new(id, prompt)
        .with_selection_mode(SelectionMode::Multiple)
        .with_options(choice_options_from_dictionary(yaml_or_json)?))
}

/// Parses comma-separated text into choice options.
///
/// This is the canonical CSV parser used by both the library builders
/// and the `question` CLI source resolver.
pub fn choice_options_from_csv(csv: &str) -> Vec<ChoiceOption<String>> {
    csv.split(',')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .map(|entry| ChoiceOption::new(entry, entry, entry.to_string()))
        .collect()
}

/// Parses markdown bullet or numbered list items into choice options.
///
/// Non-list lines are ignored. This is the canonical markdown-list
/// parser used by both the library builders and the `question` CLI
/// `--options-from-file` compatibility path.
pub fn choice_options_from_markdown_list(markdown: &str) -> Vec<ChoiceOption<String>> {
    markdown
        .lines()
        .filter_map(parse_markdown_list_line)
        .map(|text| ChoiceOption::new(text.clone(), text.clone(), text))
        .collect()
}

fn parse_markdown_list_line(line: &str) -> Option<String> {
    let trimmed = line.trim_start();
    if let Some(rest) = strip_bullet_prefix(trimmed) {
        let value = rest.trim();
        if value.is_empty() {
            return None;
        }
        return Some(value.to_string());
    }
    if let Some(rest) = strip_numbered_prefix(trimmed) {
        let value = rest.trim();
        if value.is_empty() {
            return None;
        }
        return Some(value.to_string());
    }
    None
}

fn strip_bullet_prefix(line: &str) -> Option<&str> {
    for marker in ["- ", "* ", "+ "] {
        if let Some(rest) = line.strip_prefix(marker) {
            return Some(rest);
        }
    }
    None
}

fn strip_numbered_prefix(line: &str) -> Option<&str> {
    let digit_count = line.chars().take_while(char::is_ascii_digit).count();
    if digit_count == 0 {
        return None;
    }
    let (digits, rest) = line.split_at(digit_count);
    // digits must parse as a valid unsigned integer (implicit from
    // take_while above).
    let _: u32 = digits.parse().ok()?;
    let rest = rest.strip_prefix('.').or_else(|| rest.strip_prefix(')'))?;
    let rest = rest.strip_prefix(' ')?;
    Some(rest)
}

/// Parses a YAML or JSON mapping into choice options.
///
/// Keys become labels and values become submitted values. This is the
/// canonical dictionary parser used by both the library builders and
/// the `question` CLI `--options-from-dictionary` compatibility path.
pub fn choice_options_from_dictionary(
    input: &str,
) -> Result<Vec<ChoiceOption<String>>, ChoiceBuilderError> {
    let value: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(input).map_err(|e| ChoiceBuilderError::Parse(e.to_string()))?;
    let mapping = match value {
        serde_yaml_ng::Value::Mapping(m) => m,
        _ => return Err(ChoiceBuilderError::NotAMapping),
    };
    Ok(mapping
        .into_iter()
        .map(|(key, value)| {
            let id = yaml_value_to_string(&key);
            let label = id.clone();
            let typed = yaml_value_to_string(&value);
            ChoiceOption::new(id, label, typed)
        })
        .collect())
}

fn yaml_value_to_string(value: &serde_yaml_ng::Value) -> String {
    match value {
        serde_yaml_ng::Value::Null => String::new(),
        serde_yaml_ng::Value::Bool(b) => b.to_string(),
        serde_yaml_ng::Value::Number(n) => n.to_string(),
        serde_yaml_ng::Value::String(s) => s.clone(),
        other => serde_yaml_ng::to_string(other)
            .unwrap_or_default()
            .trim()
            .to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csv_splits_and_trims_entries() {
        let input = choose_one_from_csv("c", "Pick", "Red, Green , Blue");
        assert_eq!(input.selection_mode, SelectionMode::Single);
        assert_eq!(input.options.len(), 3);
        assert_eq!(input.options[0].id, "Red");
        assert_eq!(input.options[0].label, "Red");
        assert_eq!(input.options[0].value, "Red");
        assert_eq!(input.options[1].label, "Green");
    }

    #[test]
    fn csv_drops_empty_entries() {
        let input = choose_one_from_csv("c", "P", "a, ,b,");
        assert_eq!(input.options.len(), 2);
        assert_eq!(input.options[0].value, "a");
        assert_eq!(input.options[1].value, "b");
    }

    #[test]
    fn many_csv_uses_multiple_selection_mode() {
        let input = choose_many_from_csv("c", "Pick many", "a,b");
        assert_eq!(input.selection_mode, SelectionMode::Multiple);
        assert_eq!(input.options.len(), 2);
    }

    #[test]
    fn markdown_list_accepts_dash_star_plus_bullets() {
        let md = "- one\n* two\n+ three\n";
        let input = choose_one_from_markdown_list("x", "P", md);
        assert_eq!(input.options.len(), 3);
        assert_eq!(input.options[0].value, "one");
        assert_eq!(input.options[1].value, "two");
        assert_eq!(input.options[2].value, "three");
    }

    #[test]
    fn markdown_list_accepts_numbered_lists_with_period_or_paren() {
        let md = "1. alpha\n2) beta\n";
        let input = choose_one_from_markdown_list("x", "P", md);
        assert_eq!(input.options.len(), 2);
        assert_eq!(input.options[0].value, "alpha");
        assert_eq!(input.options[1].value, "beta");
    }

    #[test]
    fn markdown_list_skips_non_list_lines() {
        let md = "header\n\n- one\njust some text\n* two\n";
        let input = choose_one_from_markdown_list("x", "P", md);
        assert_eq!(input.options.len(), 2);
    }

    #[test]
    fn markdown_list_ignores_indentation() {
        let md = "    - indented\n  * also indented\n";
        let input = choose_one_from_markdown_list("x", "P", md);
        assert_eq!(input.options.len(), 2);
        assert_eq!(input.options[0].value, "indented");
    }

    #[test]
    fn many_markdown_list_uses_multiple_selection_mode() {
        let input = choose_many_from_markdown_list("x", "P", "- a\n- b");
        assert_eq!(input.selection_mode, SelectionMode::Multiple);
    }

    #[test]
    fn dictionary_parses_yaml_mapping() {
        let yaml = "red: R\ngreen: G\nblue: B\n";
        let input = choose_one_from_dictionary("c", "P", yaml).unwrap();
        assert_eq!(input.options.len(), 3);
        assert_eq!(input.options[0].id, "red");
        assert_eq!(input.options[0].label, "red");
        assert_eq!(input.options[0].value, "R");
    }

    #[test]
    fn dictionary_parses_json_object() {
        let json = "{\"red\": \"R\", \"green\": \"G\"}";
        let input = choose_one_from_dictionary("c", "P", json).unwrap();
        assert_eq!(input.options.len(), 2);
        assert_eq!(input.options[0].id, "red");
        assert_eq!(input.options[0].value, "R");
    }

    #[test]
    fn dictionary_stringifies_non_string_values() {
        let yaml = "one: 1\ntwo: true\nthree: null\n";
        let input = choose_one_from_dictionary("c", "P", yaml).unwrap();
        assert_eq!(input.options[0].value, "1");
        assert_eq!(input.options[1].value, "true");
        assert_eq!(input.options[2].value, "");
    }

    #[test]
    fn dictionary_rejects_non_mapping_input() {
        let yaml = "- just\n- a list\n";
        let err = choose_one_from_dictionary("c", "P", yaml).unwrap_err();
        assert!(matches!(err, ChoiceBuilderError::NotAMapping));
    }

    #[test]
    fn dictionary_accepts_strict_json_object() {
        let json = "{\"red\": \"R\", \"green\": \"G\"}";
        let input = choose_one_from_dictionary("c", "P", json).unwrap();
        assert_eq!(input.options.len(), 2);
        assert_eq!(input.options[0].id, "red");
        assert_eq!(input.options[0].value, "R");
        assert_eq!(input.options[1].id, "green");
        assert_eq!(input.options[1].value, "G");
    }

    #[test]
    fn dictionary_surfaces_parse_errors() {
        let garbage = "{unterminated: mapping";
        let err = choose_one_from_dictionary("c", "P", garbage).unwrap_err();
        assert!(matches!(err, ChoiceBuilderError::Parse(_)));
    }

    #[test]
    fn many_dictionary_parses_yaml_mapping() {
        let yaml = "red: R\ngreen: G\nblue: B\n";
        let input = choose_many_from_dictionary("c", "P", yaml).unwrap();
        assert_eq!(input.selection_mode, SelectionMode::Multiple);
        assert_eq!(input.options.len(), 3);
        assert_eq!(input.options[0].id, "red");
        assert_eq!(input.options[0].label, "red");
        assert_eq!(input.options[0].value, "R");
        assert_eq!(input.options[1].value, "G");
        assert_eq!(input.options[2].value, "B");
    }

    #[test]
    fn many_dictionary_parses_json_object() {
        let json = "{\"red\": \"R\", \"green\": \"G\"}";
        let input = choose_many_from_dictionary("c", "P", json).unwrap();
        assert_eq!(input.selection_mode, SelectionMode::Multiple);
        assert_eq!(input.options.len(), 2);
        assert_eq!(input.options[0].id, "red");
        assert_eq!(input.options[0].value, "R");
        assert_eq!(input.options[1].id, "green");
        assert_eq!(input.options[1].value, "G");
    }

    #[test]
    fn many_dictionary_rejects_non_mapping() {
        let yaml = "- just\n- a list\n";
        let err = choose_many_from_dictionary("c", "P", yaml).unwrap_err();
        assert!(matches!(err, ChoiceBuilderError::NotAMapping));
    }

    #[test]
    fn many_dictionary_surfaces_parse_errors() {
        let garbage = "{unterminated: mapping";
        let err = choose_many_from_dictionary("c", "P", garbage).unwrap_err();
        assert!(matches!(err, ChoiceBuilderError::Parse(_)));
    }
}
