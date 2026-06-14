//! Adapter-owned JSON Schema (Draft 2020-12) validation.
//!
//! Prompt augmentation and JSON extraction for structured requests live in the
//! shared `unchained-ai` execution surface so the adapter and the native
//! `Prompt` path cannot drift. This module retains the two pieces the contract
//! crate owns: compiling the request schema (the contract does not bundle a
//! schema engine) and re-validating the extracted value against it.

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

#[cfg(test)]
mod tests {
    use super::*;

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
