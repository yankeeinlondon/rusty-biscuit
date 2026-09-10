//! Selection-hint parsing for composition frontmatter.
//!
//! Parses the `agent`, `model`, and `interactive` frontmatter properties into
//! the typed [`EffectiveSelectionHints`] the composition pipeline resolves
//! against. Distinct from prompt preparation (`prepare.rs`): these functions
//! only classify frontmatter values, they never compose the document.

use super::error::CompositionError;
use super::json_util::json_type_name;
use super::types::{AgentHint, EffectiveSelectionHints, ModelHint};
use crate::provider::Provider;

/// Parse selection hints (`agent`, `model`) from a raw Markdown frontmatter
/// without composing the document.
///
/// The CLI calls this for *eager* target resolution — knowing which
/// provider will run before composition templates render, so `{{env.AGENT}}`
/// and similar references resolve correctly during body/inline rendering.
/// Only untemplated literal values are recognized here; templated values
/// fall through to post-compose resolution.
///
/// ## Errors
///
/// Returns a [`CompositionError`] if either field is present but holds an
/// unsupported type, or if `agent` references an unknown provider.
pub fn parse_selection_hints_from_frontmatter(
    fm: &darkmatter::markdown::Frontmatter,
) -> Result<EffectiveSelectionHints, CompositionError> {
    let map = fm.as_map();
    let agent_full = map
        .get("agent")
        .map_or(Ok(ParsedAgentHint::default()), parse_agent_hint_full)?;
    let agent = agent_full.to_agent_hint();
    let model = map.get("model").map_or(Ok(None), parse_model_hint)?;
    let interactive = map
        .get("interactive")
        .map_or(Ok(None), parse_interactive_hint)?;
    Ok(EffectiveSelectionHints {
        agent,
        model,
        interactive,
        agent_invalid: agent_full.invalid,
        agent_was_list: agent_full.is_list,
    })
}

/// Raw parse result for an `agent` frontmatter value.
///
/// Preserves fuzzy-matched providers alongside unknown strings so invalid
/// entries can be surfaced as non-fatal state rather than aborting
/// composition. The `is_list` flag distinguishes a single value from an
/// array so a one-element list is still rendered as a list suggestion.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct ParsedAgentHint {
    pub(super) valid: Vec<Provider>,
    pub(super) invalid: Vec<String>,
    pub(super) is_list: bool,
}

impl ParsedAgentHint {
    pub(super) fn to_agent_hint(&self) -> Option<AgentHint> {
        if self.valid.is_empty() {
            return None;
        }
        if self.is_list {
            Some(AgentHint::List(self.valid.clone()))
        } else {
            Some(AgentHint::Single(self.valid[0]))
        }
    }
}

/// Parse the `agent` frontmatter value while preserving invalid entries.
///
/// Strings that do not match a known provider are collected in
/// [`ParsedAgentHint::invalid`] rather than raising an error. Type errors
/// (non-string array entries, booleans, numbers, objects) are still fatal
/// because they cannot be meaningfully classified.
pub(super) fn parse_agent_hint_full(
    value: &serde_json::Value,
) -> Result<ParsedAgentHint, CompositionError> {
    let mut result = ParsedAgentHint::default();
    match value {
        serde_json::Value::String(s) => {
            if let Some(provider) = Provider::fuzzy_match_cli_name(s) {
                result.valid.push(provider);
            } else {
                result.invalid.push(s.clone());
            }
        }
        serde_json::Value::Array(arr) => {
            result.is_list = true;
            for item in arr {
                match item {
                    serde_json::Value::String(s) => {
                        if let Some(provider) = Provider::fuzzy_match_cli_name(s) {
                            result.valid.push(provider);
                        } else {
                            result.invalid.push(s.clone());
                        }
                    }
                    other => {
                        return Err(CompositionError::AgentHintWrongType(
                            json_type_name(other).to_string(),
                        ));
                    }
                }
            }
        }
        serde_json::Value::Null => {}
        other => {
            return Err(CompositionError::AgentHintWrongType(
                json_type_name(other).to_string(),
            ));
        }
    }
    Ok(result)
}

/// Parse the `model` frontmatter value into a typed `ModelHint`.
///
/// Accepts a single string or an array of strings. Model identifiers
/// are stored verbatim; validation against provider catalogs happens
/// later in the resolution pipeline.
pub(super) fn parse_model_hint(
    value: &serde_json::Value,
) -> Result<Option<ModelHint>, CompositionError> {
    match value {
        serde_json::Value::String(s) => Ok(Some(ModelHint::Single(s.clone()))),
        serde_json::Value::Array(arr) => {
            let mut models = Vec::with_capacity(arr.len());
            for item in arr {
                match item {
                    serde_json::Value::String(s) => models.push(s.clone()),
                    other => {
                        return Err(CompositionError::ModelHintWrongType(
                            json_type_name(other).to_string(),
                        ));
                    }
                }
            }
            if models.is_empty() {
                Ok(None)
            } else {
                Ok(Some(ModelHint::List(models)))
            }
        }
        serde_json::Value::Null => Ok(None),
        other => Err(CompositionError::ModelHintWrongType(
            json_type_name(other).to_string(),
        )),
    }
}

/// Parse the `interactive` frontmatter value into an optional boolean.
///
/// Accepts `true`, `false`, or `null` (treated as absent). Anything else
/// is a typed error naming the offending JSON type.
pub fn parse_interactive_hint(
    value: &serde_json::Value,
) -> Result<Option<bool>, CompositionError> {
    match value {
        serde_json::Value::Bool(b) => Ok(Some(*b)),
        serde_json::Value::Null => Ok(None),
        other => Err(CompositionError::InteractiveHintWrongType(
            json_type_name(other).to_string(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use darkmatter::markdown::{Frontmatter, Markdown};
    use serde_json::json;

    #[test]
    fn parse_interactive_hint_accepts_true_false_and_null() {
        assert_eq!(parse_interactive_hint(&json!(true)).unwrap(), Some(true));
        assert_eq!(parse_interactive_hint(&json!(false)).unwrap(), Some(false));
        assert_eq!(parse_interactive_hint(&json!(null)).unwrap(), None);
    }

    #[test]
    fn parse_interactive_hint_rejects_non_booleans() {
        for value in [
            json!("true"),
            json!(42),
            json!([true]),
            json!({"interactive": true}),
        ] {
            let err = parse_interactive_hint(&value).unwrap_err();
            match err {
                CompositionError::InteractiveHintWrongType(found) => {
                    assert_eq!(found, json_type_name(&value));
                }
                other => panic!("expected InteractiveHintWrongType for {value}, got {other:?}"),
            }
        }
    }

    #[test]
    fn parse_selection_hints_from_frontmatter_reads_interactive() {
        let mut fm = Frontmatter::new();
        fm.insert("interactive", json!(true)).unwrap();
        let md = Markdown::with_frontmatter(fm, "Content");

        let hints = parse_selection_hints_from_frontmatter(md.frontmatter()).unwrap();
        assert_eq!(hints.interactive, Some(true));
    }
}
