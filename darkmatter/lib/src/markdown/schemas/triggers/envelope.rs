//! Trigger-schema envelope parsing.
//!
//! A trigger schema is a standalone YAML file claiming the
//! `kind: trigger-schema` envelope — the same envelope-claiming pattern as
//! `kind: schema`. The envelope carries:
//!
//! - `match:` (required) — the match-expression payload parsed by
//!   [`super::grammar`].
//! - `$schema:` (optional, deferred) — the payload reference layered onto
//!   matching documents. Resolution is deferred to Phase 4; this module
//!   records the raw authored value without resolving it.
//!
//! Unknown top-level keys are rejected (never silently ignored). The envelope
//! performs no I/O: it validates shape, parses the match grammar, and runs
//! the vacuous-trigger lint, then hands the parsed result back to the caller.
//!
//! See `darkmatter/features/2026-07-10-schema-triggers/spec.md`.

use serde_yaml_ng::Value as YamlValue;

use crate::markdown::schemas::errors::SchemaError;

use super::grammar::{MatchArms, parse_match_arms};
use super::lint;

/// The `kind:` value that claims a trigger-schema envelope.
pub const ENVELOPE_KIND: &str = "trigger-schema";

/// The recognized top-level keys of a trigger-schema envelope.
const KNOWN_KEYS: &[&str] = &["kind", "match", "$schema"];

/// A parsed `kind: trigger-schema` envelope.
///
/// The match payload is fully parsed and linted. The `$schema:` payload
/// reference is recorded raw — resolution (and merge-compatibility gating)
/// happens in Phase 4's effective-schema assembly.
#[derive(Debug, Clone, PartialEq)]
pub struct TriggerEnvelope {
    /// The parsed, lint-clean match arms (OR'd).
    pub match_arms: MatchArms,
    /// The raw `$schema:` value, when present. Deferred to Phase 4.
    pub payload: Option<YamlValue>,
}

/// Classifies and parses a `kind: trigger-schema` envelope from YAML source.
///
/// Returns `Ok(None)` when the document does not claim the trigger-schema
/// envelope (no `kind: trigger-schema`). Once the envelope is claimed, a
/// malformed payload (missing `match:`, bad match grammar, vacuous trigger,
/// unknown top-level keys) is a hard [`SchemaError`].
///
/// Does **not** resolve the `$schema:` payload — that is Phase 4's job.
pub fn parse_trigger_envelope(value: &YamlValue) -> Result<Option<TriggerEnvelope>, SchemaError> {
    let Some(map) = value.as_mapping() else {
        return Ok(None);
    };

    let kind_value = map
        .get(YamlValue::String("kind".into()))
        .and_then(YamlValue::as_str);
    if kind_value != Some(ENVELOPE_KIND) {
        return Ok(None);
    }

    // Reject unknown top-level keys.
    let unknown: Vec<String> = map
        .keys()
        .filter_map(YamlValue::as_str)
        .filter(|key| !KNOWN_KEYS.contains(key))
        .map(str::to_string)
        .collect();
    if !unknown.is_empty() {
        return Err(SchemaError::TriggerMatch {
            message: format!(
                "trigger-schema envelope supports only `kind`, `match`, and `$schema`; found \
                 unknown keys: {}",
                unknown.join(", ")
            ),
        });
    }

    // `match:` is required.
    let match_value = map
        .get(YamlValue::String("match".into()))
        .ok_or_else(|| SchemaError::TriggerMatch {
            message: "trigger-schema envelope requires a `match:` key".into(),
        })?;

    let match_arms = parse_match_arms(match_value)?;
    // Vacuous-trigger lint: one vacuous arm makes the whole trigger vacuous.
    lint::lint(&match_arms)?;

    let payload = map
        .get(YamlValue::String("$schema".into()))
        .cloned();

    Ok(Some(TriggerEnvelope {
        match_arms,
        payload,
    }))
}

/// Convenience wrapper that parses YAML source first.
pub fn parse_trigger_envelope_from_str(source: &str) -> Result<Option<TriggerEnvelope>, SchemaError> {
    let value: YamlValue = serde_yaml_ng::from_str(source).map_err(|err| {
        SchemaError::TriggerMatch {
            message: format!("trigger-schema envelope is not valid YAML: {err}"),
        }
    })?;
    parse_trigger_envelope(&value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ignores_non_trigger_document() {
        let v: YamlValue = serde_yaml_ng::from_str("$schema:\n  title: string").unwrap();
        assert!(parse_trigger_envelope(&v).unwrap().is_none());
    }

    #[test]
    fn ignores_kind_schema() {
        let v: YamlValue =
            serde_yaml_ng::from_str("kind: schema\ntypes:\n  title: string").unwrap();
        assert!(parse_trigger_envelope(&v).unwrap().is_none());
    }

    #[test]
    fn parses_minimal_envelope() {
        let v: YamlValue =
            serde_yaml_ng::from_str("kind: trigger-schema\nmatch:\n  prompt: string(required)")
                .unwrap();
        let env = parse_trigger_envelope(&v).unwrap().unwrap();
        assert_eq!(env.match_arms.0.len(), 1);
        assert!(env.payload.is_none());
    }

    #[test]
    fn records_payload_raw() {
        let v: YamlValue = serde_yaml_ng::from_str(
            "kind: trigger-schema\nmatch:\n  prompt: string(required)\n$schema: claudine.yaml",
        )
        .unwrap();
        let env = parse_trigger_envelope(&v).unwrap().unwrap();
        assert_eq!(env.payload.as_ref().and_then(YamlValue::as_str), Some("claudine.yaml"));
    }

    #[test]
    fn rejects_missing_match() {
        let v: YamlValue =
            serde_yaml_ng::from_str("kind: trigger-schema\n$schema: claudine.yaml").unwrap();
        let err = parse_trigger_envelope(&v).unwrap_err();
        assert!(matches!(err, SchemaError::TriggerMatch { .. }));
    }

    #[test]
    fn rejects_unknown_key() {
        let v: YamlValue = serde_yaml_ng::from_str(
            "kind: trigger-schema\nmatch:\n  prompt: string(required)\nfoo: bar",
        )
        .unwrap();
        let err = parse_trigger_envelope(&v).unwrap_err();
        assert!(matches!(err, SchemaError::TriggerMatch { .. }));
    }

    #[test]
    fn rejects_vacuous_trigger() {
        let v: YamlValue =
            serde_yaml_ng::from_str("kind: trigger-schema\nmatch:\n  maybe: string").unwrap();
        let err = parse_trigger_envelope(&v).unwrap_err();
        assert!(matches!(err, SchemaError::TriggerVacuousArm));
    }
}
