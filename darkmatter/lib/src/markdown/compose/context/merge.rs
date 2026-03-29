//! Merge logic for user-defined `ctx` with runtime `ctx`.

use serde_json::{Map, Value};
use thiserror::Error;

use super::diagnostics::ContextMergeDiagnostic;

/// Error returned when user ctx is invalid and `allow_override` is false.
#[derive(Debug, Error)]
pub enum CtxMergeError {
    /// Document defines `ctx` as a non-object value.
    #[error("document `ctx` must be a JSON object, but found {kind}")]
    InvalidUserCtx {
        /// The JSON type of the invalid ctx value.
        kind: String,
    },
}

/// Result of merging user ctx with runtime ctx.
#[derive(Debug)]
pub struct CtxMergeResult {
    /// The merged ctx value (always an object).
    pub merged_ctx: Value,
    /// Diagnostics emitted during the merge.
    pub diagnostics: Vec<ContextMergeDiagnostic>,
}

/// Merges user-defined `ctx` with runtime `ctx`.
///
/// ## Merge policy
///
/// 1. No user `ctx` → insert runtime `ctx` directly. No diagnostics.
/// 2. User `ctx` is an object → deep-merge user into runtime (runtime wins on collision).
///    Emits `UserCtxMerged { had_key_collisions }`.
/// 3. User `ctx` is not an object:
///    - `allow_override = false` → returns `Err(InvalidUserCtx)`
///    - `allow_override = true` → uses runtime ctx, emits `InvalidUserCtxReplaced`
pub fn merge_ctx(
    user_ctx: Option<&Value>,
    runtime_ctx: Value,
    allow_override: bool,
) -> Result<CtxMergeResult, CtxMergeError> {
    let mut diagnostics = Vec::new();

    let merged = match user_ctx {
        None | Some(Value::Null) => {
            // No user ctx: use runtime directly
            runtime_ctx
        }
        Some(Value::Object(user_obj)) => {
            // User ctx is an object: deep-merge (runtime wins)
            let runtime_obj = match &runtime_ctx {
                Value::Object(m) => m.clone(),
                _ => Map::new(),
            };

            let had_collisions = user_obj.keys().any(|k| runtime_obj.contains_key(k));

            // Start with user ctx, overlay runtime (runtime wins)
            let mut merged = user_obj.clone();
            for (key, value) in runtime_obj {
                merged.insert(key, value);
            }

            diagnostics.push(ContextMergeDiagnostic::UserCtxMerged {
                had_key_collisions: had_collisions,
            });

            Value::Object(merged)
        }
        Some(other) => {
            if !allow_override {
                return Err(CtxMergeError::InvalidUserCtx {
                    kind: json_type_name(other).to_string(),
                });
            }
            diagnostics.push(ContextMergeDiagnostic::InvalidUserCtxReplaced);
            runtime_ctx
        }
    };

    Ok(CtxMergeResult {
        merged_ctx: merged,
        diagnostics,
    })
}

fn json_type_name(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn runtime_ctx() -> Value {
        json!({
            "today": "2024-06-15",
            "year": "2024",
            "os": "macOS"
        })
    }

    #[test]
    fn no_user_ctx_returns_runtime() {
        let result = merge_ctx(None, runtime_ctx(), false).unwrap();
        assert_eq!(result.merged_ctx, runtime_ctx());
        assert!(result.diagnostics.is_empty());
    }

    #[test]
    fn null_user_ctx_returns_runtime() {
        let result = merge_ctx(Some(&Value::Null), runtime_ctx(), false).unwrap();
        assert_eq!(result.merged_ctx, runtime_ctx());
        assert!(result.diagnostics.is_empty());
    }

    #[test]
    fn user_object_no_collisions() {
        let user = json!({"custom_key": "custom_value"});
        let result = merge_ctx(Some(&user), runtime_ctx(), false).unwrap();

        let merged = result.merged_ctx.as_object().unwrap();
        assert_eq!(merged.get("custom_key"), Some(&json!("custom_value")));
        assert_eq!(merged.get("today"), Some(&json!("2024-06-15")));

        assert_eq!(
            result.diagnostics,
            vec![ContextMergeDiagnostic::UserCtxMerged {
                had_key_collisions: false
            }]
        );
    }

    #[test]
    fn user_object_with_collisions_runtime_wins() {
        let user = json!({"today": "overridden", "custom": "value"});
        let result = merge_ctx(Some(&user), runtime_ctx(), false).unwrap();

        let merged = result.merged_ctx.as_object().unwrap();
        // Runtime wins on collision
        assert_eq!(merged.get("today"), Some(&json!("2024-06-15")));
        // User non-colliding key preserved
        assert_eq!(merged.get("custom"), Some(&json!("value")));

        assert_eq!(
            result.diagnostics,
            vec![ContextMergeDiagnostic::UserCtxMerged {
                had_key_collisions: true
            }]
        );
    }

    #[test]
    fn user_string_without_override_errors() {
        let user = json!("hello");
        let result = merge_ctx(Some(&user), runtime_ctx(), false);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("string"));
    }

    #[test]
    fn user_string_with_override_warns() {
        let user = json!("hello");
        let result = merge_ctx(Some(&user), runtime_ctx(), true).unwrap();
        assert_eq!(result.merged_ctx, runtime_ctx());
        assert_eq!(
            result.diagnostics,
            vec![ContextMergeDiagnostic::InvalidUserCtxReplaced]
        );
    }

    #[test]
    fn user_array_without_override_errors() {
        let user = json!([1, 2, 3]);
        let result = merge_ctx(Some(&user), runtime_ctx(), false);
        assert!(result.is_err());
    }

    #[test]
    fn user_array_with_override_warns() {
        let user = json!([1, 2, 3]);
        let result = merge_ctx(Some(&user), runtime_ctx(), true).unwrap();
        assert_eq!(result.merged_ctx, runtime_ctx());
        assert_eq!(
            result.diagnostics,
            vec![ContextMergeDiagnostic::InvalidUserCtxReplaced]
        );
    }
}
