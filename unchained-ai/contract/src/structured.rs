//! Structured-output support: prompt augmentation, JSON extraction, and
//! adapter-owned JSON Schema (Draft 2020-12) validation.
//!
//! v1 uses a uniform, provider-independent prompt-and-parse strategy: the
//! schema is validated up front, the prompt is augmented to request a single
//! conforming JSON value, the value is extracted from the assistant text
//! (tolerating code fences and surrounding prose), and the result is validated
//! against the schema. The contract does not bundle a schema engine, so
//! validation lives here.

use biscuit_contract::inference::{InferenceError, InferenceErrorKind};
use jsonschema::{Draft, Validator};
use serde_json::Value;

use crate::error::inference_error;

/// Compile a request schema, rejecting an invalid one as `InvalidRequest`.
pub(crate) fn compile_schema(schema: &Value) -> Result<Validator, InferenceError> {
    jsonschema::options()
        .with_draft(Draft::Draft202012)
        .build(schema)
        .map_err(|err| {
            inference_error(
                InferenceErrorKind::InvalidRequest,
                format!("invalid JSON Schema: {err}"),
            )
        })
}

/// Append a JSON-only instruction describing the required schema.
///
/// This is adapter-owned framing, distinct from the untrusted prompt: it tells
/// the model to emit exactly one JSON value with no surrounding prose.
pub(crate) fn augment_prompt(prompt: &str, schema: &Value) -> String {
    let schema_text = serde_json::to_string_pretty(schema)
        .unwrap_or_else(|_| schema.to_string());
    format!(
        "{prompt}\n\n---\n\nReturn your answer as a single JSON value that validates \
against this JSON Schema. Output only the JSON value — no prose, no explanation, \
no Markdown code fences.\n\nJSON Schema:\n{schema_text}"
    )
}

/// Extract a single JSON value from assistant text.
///
/// Accepts the whole text as one JSON value, or — tolerating surrounding prose
/// and code fences — exactly one balanced `{...}`/`[...]` span that parses as
/// JSON. Empty output, no parseable value, or *more than one* parseable value
/// is `InvalidResponse`: a response carrying multiple JSON values is ambiguous
/// and must never be silently narrowed to the first.
pub(crate) fn extract_json(text: &str) -> Result<Value, InferenceError> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err(invalid_response(
            "provider returned empty output for a structured request",
        ));
    }

    // A response that is exactly one JSON value: `serde_json` rejects trailing
    // tokens, so this branch is already proven unique.
    if let Ok(value) = serde_json::from_str::<Value>(trimmed) {
        return Ok(value);
    }

    // Permissive fallback. Scanning balanced object/array spans also covers
    // fenced ```json blocks, whose braces sit inside the fence; require exactly
    // one span to parse as JSON.
    let mut found: Option<Value> = None;
    for span in balanced_spans(trimmed) {
        if let Ok(value) = serde_json::from_str::<Value>(span) {
            if found.is_some() {
                return Err(invalid_response(
                    "provider output contained more than one JSON value; expected a single value",
                ));
            }
            found = Some(value);
        }
    }

    found.ok_or_else(|| invalid_response("provider output was not a single JSON value"))
}

/// Validate an extracted value against the compiled schema.
pub(crate) fn validate_instance(
    validator: &Validator,
    instance: &Value,
) -> Result<(), InferenceError> {
    if let Some(error) = validator.iter_errors(instance).next() {
        return Err(invalid_response(format!(
            "structured response did not satisfy the schema: {error}"
        )));
    }
    Ok(())
}

fn invalid_response(message: impl Into<String>) -> InferenceError {
    inference_error(InferenceErrorKind::InvalidResponse, message)
}

/// Return every top-level balanced `{...}`/`[...]` span, in source order.
fn balanced_spans(text: &str) -> Vec<&str> {
    let bytes = text.as_bytes();
    let mut spans = Vec::new();
    let mut cursor = 0;
    while let Some(offset) = bytes[cursor..].iter().position(|&b| b == b'{' || b == b'[') {
        let start = cursor + offset;
        let open = bytes[start];
        let close = if open == b'{' { b'}' } else { b']' };
        match span_end(bytes, start, open, close) {
            Some(end) => {
                spans.push(&text[start..=end]);
                cursor = end + 1;
            }
            None => cursor = start + 1,
        }
    }
    spans
}

/// Index of the byte closing the balanced span opened at `start`, if it closes.
fn span_end(bytes: &[u8], start: usize, open: u8, close: u8) -> Option<usize> {
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    for (index, &byte) in bytes.iter().enumerate().skip(start) {
        if in_string {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                in_string = false;
            }
            continue;
        }
        match byte {
            b'"' => in_string = true,
            b if b == open => depth += 1,
            b if b == close => {
                depth -= 1;
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_json_accepts_exact_value() {
        let value = extract_json(r#"{"answer": 42}"#).unwrap();
        assert_eq!(value["answer"], 42);
    }

    #[test]
    fn extract_json_strips_fenced_block() {
        let value = extract_json("```json\n{\n  \"answer\": 42\n}\n```").unwrap();
        assert_eq!(value["answer"], 42);
    }

    #[test]
    fn extract_json_rejects_multiple_values() {
        let err = extract_json("{\"a\":1}\n{\"b\":2}").unwrap_err();
        assert_eq!(err.kind, InferenceErrorKind::InvalidResponse);
    }

    #[test]
    fn extract_json_rejects_empty_output() {
        let err = extract_json("   ").unwrap_err();
        assert_eq!(err.kind, InferenceErrorKind::InvalidResponse);
    }

    #[test]
    fn validate_instance_rejects_schema_violation() {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "answer": { "type": "integer" }
            },
            "required": ["answer"]
        });
        let validator = compile_schema(&schema).unwrap();
        let bad = serde_json::json!({"answer": "not an integer"});
        let err = validate_instance(&validator, &bad).unwrap_err();
        assert_eq!(err.kind, InferenceErrorKind::InvalidResponse);
    }

    #[test]
    fn compile_schema_rejects_invalid_schema() {
        let schema = serde_json::json!({ "type": 42 });
        let err = compile_schema(&schema).unwrap_err();
        assert_eq!(err.kind, InferenceErrorKind::InvalidRequest);
    }
}
