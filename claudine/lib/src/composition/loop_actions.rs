//! Loop frontmatter action engine.

use serde_json::{Map, Number, Value};

use super::error::CompositionError;
use super::types::{AmbientVariable, LoopAction};

/// Staged frontmatter mutations for one loop iteration.
#[derive(Debug, Clone)]
pub struct ActionStaging {
    frontmatter: Map<String, Value>,
    iteration: usize,
    total_actions: usize,
}

impl ActionStaging {
    /// Create a staged copy of the current frontmatter.
    pub fn new(frontmatter: &Map<String, Value>, iteration: usize, total_actions: usize) -> Self {
        Self {
            frontmatter: frontmatter.clone(),
            iteration,
            total_actions,
        }
    }

    /// Apply one action to the staged copy.
    ///
    /// ## Errors
    ///
    /// Returns a contextual loop action error when the mutation is invalid.
    pub fn apply_action(
        &mut self,
        action: &LoopAction,
        action_index: usize,
    ) -> Result<(), CompositionError> {
        let context = ActionContext {
            iteration: self.iteration,
            action_index,
            total_actions: self.total_actions,
        };
        apply_action_with_context(&mut self.frontmatter, action, context)
    }

    /// Commit the staged frontmatter copy.
    pub fn commit(self) -> Value {
        Value::Object(self.frontmatter)
    }

    /// Commit the staged frontmatter copy as a map.
    pub fn commit_map(self) -> Map<String, Value> {
        self.frontmatter
    }
}

#[derive(Debug, Clone, Copy)]
struct ActionContext {
    iteration: usize,
    action_index: usize,
    total_actions: usize,
}

impl Default for ActionContext {
    fn default() -> Self {
        Self {
            iteration: 1,
            action_index: 1,
            total_actions: 1,
        }
    }
}

/// Apply a single loop action without staging.
///
/// Most callers should prefer [`ActionStaging`] so a multi-action iteration is
/// all-or-nothing.
pub fn apply_action(
    fm: &mut Map<String, Value>,
    action: &LoopAction,
) -> Result<(), CompositionError> {
    apply_action_with_context(fm, action, ActionContext::default())
}

/// Increment a frontmatter property by one.
///
/// Missing and null properties become `1`; numeric strings are parsed and
/// stored back as JSON numbers.
pub fn apply_increment(fm: &mut Map<String, Value>, prop: &str) -> Result<(), CompositionError> {
    apply_increment_with_context(fm, prop, ActionContext::default())
}

/// Decrement a frontmatter property by one.
///
/// Missing and null properties become `-1`; numeric strings are parsed and
/// stored back as JSON numbers.
pub fn apply_decrement(fm: &mut Map<String, Value>, prop: &str) -> Result<(), CompositionError> {
    apply_decrement_with_context(fm, prop, ActionContext::default())
}

fn apply_action_with_context(
    fm: &mut Map<String, Value>,
    action: &LoopAction,
    context: ActionContext,
) -> Result<(), CompositionError> {
    match action {
        LoopAction::Increment(prop) => apply_increment_with_context(fm, prop, context),
        LoopAction::Decrement(prop) => apply_decrement_with_context(fm, prop, context),
        LoopAction::Set { prop, value } => apply_set(fm, prop, value, context),
        LoopAction::Append { prop, value } => apply_append(fm, prop, value),
        LoopAction::Prepend { prop, value } => apply_prepend(fm, prop, value),
        LoopAction::Merge { prop, value } => apply_merge(fm, prop, value, context),
    }
}

fn apply_increment_with_context(
    fm: &mut Map<String, Value>,
    prop: &str,
    context: ActionContext,
) -> Result<(), CompositionError> {
    let next = match fm.get(prop) {
        None | Some(Value::Null) => Value::Number(Number::from(1)),
        Some(value) => {
            increment_value(value).ok_or_else(|| CompositionError::InvalidIncrementType {
                iteration: context.iteration,
                action_index: context.action_index,
                total_actions: context.total_actions,
                property: prop.to_string(),
                found: json_type_name(value).to_string(),
            })?
        }
    };
    fm.insert(prop.to_string(), next);
    Ok(())
}

fn apply_decrement_with_context(
    fm: &mut Map<String, Value>,
    prop: &str,
    context: ActionContext,
) -> Result<(), CompositionError> {
    let next = match fm.get(prop) {
        None | Some(Value::Null) => Value::Number(Number::from(-1)),
        Some(value) => {
            decrement_value(value).ok_or_else(|| CompositionError::InvalidDecrementType {
                iteration: context.iteration,
                action_index: context.action_index,
                total_actions: context.total_actions,
                property: prop.to_string(),
                found: json_type_name(value).to_string(),
            })?
        }
    };
    fm.insert(prop.to_string(), next);
    Ok(())
}

fn increment_value(value: &Value) -> Option<Value> {
    add_one(value, 1.0)
}

fn decrement_value(value: &Value) -> Option<Value> {
    add_one(value, -1.0)
}

fn add_one(value: &Value, delta: f64) -> Option<Value> {
    match value {
        Value::Number(number) => add_number(number, delta),
        Value::String(raw) => {
            if let Ok(int) = raw.parse::<i64>() {
                return Some(Value::Number(Number::from(int + delta as i64)));
            }
            let parsed = raw.parse::<f64>().ok()?;
            number_from_f64(parsed + delta)
        }
        _ => None,
    }
}

fn add_number(number: &Number, delta: f64) -> Option<Value> {
    if let Some(int) = number.as_i64() {
        return Some(Value::Number(Number::from(int + delta as i64)));
    }
    if let Some(uint) = number.as_u64() {
        if delta.is_sign_negative() && uint == 0 {
            return Some(Value::Number(Number::from(-1)));
        }
        if delta.is_sign_positive() {
            return Some(Value::Number(Number::from(uint + 1)));
        }
        return uint.checked_sub(1).map(Number::from).map(Value::Number);
    }
    number_from_f64(number.as_f64()? + delta)
}

fn number_from_f64(value: f64) -> Option<Value> {
    Number::from_f64(value).map(Value::Number)
}

fn apply_set(
    fm: &mut Map<String, Value>,
    prop: &str,
    value: &Value,
    context: ActionContext,
) -> Result<(), CompositionError> {
    reject_reserved_property(prop, context)?;
    fm.insert(prop.to_string(), value.clone());
    Ok(())
}

fn apply_append(
    fm: &mut Map<String, Value>,
    prop: &str,
    value: &Value,
) -> Result<(), CompositionError> {
    let existing = existing_text(fm.get(prop));
    let addition = append_fragment(&existing, value)?;
    fm.insert(
        prop.to_string(),
        Value::String(format!("{existing}{addition}")),
    );
    Ok(())
}

fn apply_prepend(
    fm: &mut Map<String, Value>,
    prop: &str,
    value: &Value,
) -> Result<(), CompositionError> {
    let existing = existing_text(fm.get(prop));
    let addition = prepend_fragment(&existing, value)?;
    fm.insert(
        prop.to_string(),
        Value::String(format!("{addition}{existing}")),
    );
    Ok(())
}

fn apply_merge(
    fm: &mut Map<String, Value>,
    prop: &str,
    value: &Value,
    context: ActionContext,
) -> Result<(), CompositionError> {
    let new_object = value.as_object().ok_or_else(|| {
        invalid_action(
            context,
            format!(
                "`merge({prop}, value)` requires value to be an object, got {}",
                json_type_name(value)
            ),
        )
    })?;

    match fm.get_mut(prop) {
        None | Some(Value::Null) => {
            fm.insert(prop.to_string(), Value::Object(new_object.clone()));
            Ok(())
        }
        Some(Value::Object(existing)) => {
            for (key, value) in new_object {
                existing.insert(key.clone(), value.clone());
            }
            Ok(())
        }
        Some(existing) => Err(invalid_action(
            context,
            format!(
                "`merge({prop}, value)` requires `{prop}` to be an object or null, got {}",
                json_type_name(existing)
            ),
        )),
    }
}

fn reject_reserved_property(prop: &str, context: ActionContext) -> Result<(), CompositionError> {
    if prop == "loop" || prop == "replace" || AmbientVariable::is_reserved(prop) {
        return Err(invalid_action(
            context,
            format!("`{prop}` is reserved and cannot be set by a loop action"),
        ));
    }
    Ok(())
}

fn invalid_action(context: ActionContext, message: String) -> CompositionError {
    CompositionError::InvalidAction {
        iteration: context.iteration,
        action_index: context.action_index,
        total_actions: context.total_actions,
        message,
    }
}

fn existing_text(value: Option<&Value>) -> String {
    match value {
        None | Some(Value::Null) => String::new(),
        Some(Value::String(value)) => value.clone(),
        Some(Value::Number(value)) => value.to_string(),
        Some(Value::Bool(value)) => value.to_string(),
        Some(Value::Array(_)) | Some(Value::Object(_)) => compact_json(value.unwrap()),
    }
}

fn append_fragment(existing: &str, value: &Value) -> Result<String, CompositionError> {
    match value {
        Value::Null => Ok(jsonl_empty_placeholder(existing).to_string()),
        Value::String(value) if value.is_empty() => {
            Ok(jsonl_empty_placeholder(existing).to_string())
        }
        Value::String(value) => Ok(value.clone()),
        Value::Number(value) => Ok(value.to_string()),
        Value::Bool(value) => Ok(value.to_string()),
        Value::Object(_) | Value::Array(_) => Ok(format!("\n{}", compact_json(value))),
    }
}

fn prepend_fragment(existing: &str, value: &Value) -> Result<String, CompositionError> {
    match value {
        Value::Null => Ok(format!("{}\n", jsonl_empty_placeholder(existing))),
        Value::String(value) if value.is_empty() => {
            Ok(format!("{}\n", jsonl_empty_placeholder(existing)))
        }
        Value::String(value) => Ok(value.clone()),
        Value::Number(value) => Ok(value.to_string()),
        Value::Bool(value) => Ok(value.to_string()),
        Value::Object(_) | Value::Array(_) => Ok(format!("{}\n", compact_json(value))),
    }
}

fn jsonl_empty_placeholder(existing: &str) -> &'static str {
    let first_line = existing.lines().find(|line| !line.trim().is_empty());
    match first_line.and_then(|line| serde_json::from_str::<Value>(line).ok()) {
        Some(Value::Object(_)) => "{}",
        _ => "[]",
    }
}

fn compact_json(value: &Value) -> String {
    serde_json::to_string(value).expect("serializing serde_json::Value cannot fail")
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

    fn object(value: Value) -> Map<String, Value> {
        value.as_object().unwrap().clone()
    }

    #[test]
    fn increment_sets_missing_and_null_to_one() {
        let mut fm = Map::new();
        apply_increment(&mut fm, "counter").unwrap();
        assert_eq!(fm.get("counter"), Some(&json!(1)));

        fm.insert("counter".into(), Value::Null);
        apply_increment(&mut fm, "counter").unwrap();
        assert_eq!(fm.get("counter"), Some(&json!(1)));
    }

    #[test]
    fn increment_accepts_numbers_and_numeric_strings() {
        let mut fm = object(json!({"int": 5, "str": "5", "float": 1.5}));
        apply_increment(&mut fm, "int").unwrap();
        apply_increment(&mut fm, "str").unwrap();
        apply_increment(&mut fm, "float").unwrap();

        assert_eq!(fm.get("int"), Some(&json!(6)));
        assert_eq!(fm.get("str"), Some(&json!(6)));
        assert_eq!(fm.get("float"), Some(&json!(2.5)));
    }

    #[test]
    fn increment_rejects_non_numeric_strings() {
        let mut fm = object(json!({"counter": "abc"}));
        let err = apply_increment(&mut fm, "counter").unwrap_err();
        assert!(matches!(
            err,
            CompositionError::InvalidIncrementType {
                iteration: 1,
                action_index: 1,
                total_actions: 1,
                property,
                found
            } if property == "counter" && found == "string"
        ));
    }

    #[test]
    fn decrement_sets_missing_and_accepts_numeric_strings() {
        let mut fm = object(json!({"counter": "5"}));
        apply_decrement(&mut fm, "missing").unwrap();
        apply_decrement(&mut fm, "counter").unwrap();

        assert_eq!(fm.get("missing"), Some(&json!(-1)));
        assert_eq!(fm.get("counter"), Some(&json!(4)));
    }

    #[test]
    fn set_rejects_reserved_properties() {
        for prop in ["loop", "replace", "iteration"] {
            let mut fm = Map::new();
            let err = apply_action(
                &mut fm,
                &LoopAction::Set {
                    prop: prop.into(),
                    value: json!(true),
                },
            )
            .unwrap_err();
            assert!(
                matches!(err, CompositionError::InvalidAction { ref message, .. } if message.contains("reserved")),
                "got {err}"
            );
        }
    }

    #[test]
    fn set_assigns_new_value() {
        let mut fm = object(json!({"stage": "draft"}));
        apply_action(
            &mut fm,
            &LoopAction::Set {
                prop: "stage".into(),
                value: json!("review"),
            },
        )
        .unwrap();
        assert_eq!(fm.get("stage"), Some(&json!("review")));
    }

    #[test]
    fn append_handles_scalars_and_json_values() {
        let mut fm = object(json!({"log": "start"}));
        apply_action(
            &mut fm,
            &LoopAction::Append {
                prop: "log".into(),
                value: json!(true),
            },
        )
        .unwrap();
        apply_action(
            &mut fm,
            &LoopAction::Append {
                prop: "log".into(),
                value: json!({"event": "tick"}),
            },
        )
        .unwrap();
        apply_action(
            &mut fm,
            &LoopAction::Append {
                prop: "log".into(),
                value: json!([1, 2]),
            },
        )
        .unwrap();

        assert_eq!(
            fm.get("log"),
            Some(&json!("starttrue\n{\"event\":\"tick\"}\n[1,2]"))
        );
    }

    #[test]
    fn append_empty_preserves_jsonl_shape() {
        let mut fm = object(json!({"objects": "{\"a\":1}", "arrays": "[1]"}));
        apply_action(
            &mut fm,
            &LoopAction::Append {
                prop: "objects".into(),
                value: Value::Null,
            },
        )
        .unwrap();
        apply_action(
            &mut fm,
            &LoopAction::Append {
                prop: "arrays".into(),
                value: json!(""),
            },
        )
        .unwrap();

        assert_eq!(fm.get("objects"), Some(&json!("{\"a\":1}{}")));
        assert_eq!(fm.get("arrays"), Some(&json!("[1][]")));
    }

    #[test]
    fn prepend_is_append_in_reverse_for_json_values() {
        let mut fm = object(json!({"log": "tail"}));
        apply_action(
            &mut fm,
            &LoopAction::Prepend {
                prop: "log".into(),
                value: json!({"event": "tick"}),
            },
        )
        .unwrap();

        assert_eq!(fm.get("log"), Some(&json!("{\"event\":\"tick\"}\ntail")));
    }

    #[test]
    fn merge_shallow_merges_and_replaces_arrays() {
        let mut fm = object(json!({"state": {"a": 1, "items": [1], "keep": true}}));
        apply_action(
            &mut fm,
            &LoopAction::Merge {
                prop: "state".into(),
                value: json!({"b": 2, "items": [2]}),
            },
        )
        .unwrap();

        assert_eq!(
            fm.get("state"),
            Some(&json!({"a": 1, "b": 2, "items": [2], "keep": true}))
        );
    }

    #[test]
    fn merge_rejects_non_object_target_or_value() {
        let mut fm = object(json!({"state": "nope"}));
        let err = apply_action(
            &mut fm,
            &LoopAction::Merge {
                prop: "state".into(),
                value: json!({"b": 2}),
            },
        )
        .unwrap_err();
        assert!(matches!(err, CompositionError::InvalidAction { .. }));

        let mut fm = Map::new();
        let err = apply_action(
            &mut fm,
            &LoopAction::Merge {
                prop: "state".into(),
                value: json!("nope"),
            },
        )
        .unwrap_err();
        assert!(matches!(err, CompositionError::InvalidAction { .. }));
    }

    #[test]
    fn staging_discards_partial_mutations_on_error() {
        let fm = object(json!({"counter": 1, "bad": "abc"}));
        let original = fm.clone();
        let actions = [
            LoopAction::Increment("counter".into()),
            LoopAction::Increment("bad".into()),
            LoopAction::Set {
                prop: "stage".into(),
                value: json!("done"),
            },
        ];

        let mut stage = ActionStaging::new(&fm, 7, actions.len());
        stage.apply_action(&actions[0], 1).unwrap();
        let err = stage.apply_action(&actions[1], 2).unwrap_err();

        assert!(matches!(
            err,
            CompositionError::InvalidIncrementType {
                iteration: 7,
                action_index: 2,
                total_actions: 3,
                ..
            }
        ));
        assert_eq!(fm, original);
    }

    #[test]
    fn staging_commits_after_all_actions_succeed() {
        let fm = object(json!({"counter": 1}));
        let actions = [
            LoopAction::Increment("counter".into()),
            LoopAction::Set {
                prop: "stage".into(),
                value: json!("done"),
            },
        ];

        let mut stage = ActionStaging::new(&fm, 1, actions.len());
        for (index, action) in actions.iter().enumerate() {
            stage.apply_action(action, index + 1).unwrap();
        }

        assert_eq!(stage.commit(), json!({"counter": 2, "stage": "done"}));
        assert_eq!(fm, object(json!({"counter": 1})));
    }

    #[test]
    fn increment_rejects_boolean() {
        let mut fm = object(json!({"flag": true}));
        let err = apply_increment(&mut fm, "flag").unwrap_err();
        assert!(matches!(
            err,
            CompositionError::InvalidIncrementType {
                iteration: 1,
                action_index: 1,
                total_actions: 1,
                property,
                found
            } if property == "flag" && found == "boolean"
        ));
    }

    #[test]
    fn decrement_rejects_boolean() {
        let mut fm = object(json!({"flag": true}));
        let err = apply_decrement(&mut fm, "flag").unwrap_err();
        assert!(matches!(
            err,
            CompositionError::InvalidDecrementType {
                iteration: 1,
                action_index: 1,
                total_actions: 1,
                property,
                found
            } if property == "flag" && found == "boolean"
        ));
    }

    #[test]
    fn decrement_rejects_non_numeric_string() {
        let mut fm = object(json!({"counter": "abc"}));
        let err = apply_decrement(&mut fm, "counter").unwrap_err();
        assert!(matches!(
            err,
            CompositionError::InvalidDecrementType {
                iteration: 1,
                action_index: 1,
                total_actions: 1,
                property,
                found
            } if property == "counter" && found == "string"
        ));
    }

    #[test]
    fn append_to_empty_string() {
        let mut fm = object(json!({"log": ""}));
        apply_action(
            &mut fm,
            &LoopAction::Append {
                prop: "log".into(),
                value: json!("first"),
            },
        )
        .unwrap();
        assert_eq!(fm.get("log"), Some(&json!("first")));
    }

    #[test]
    fn prepend_to_empty_string() {
        let mut fm = object(json!({"log": ""}));
        apply_action(
            &mut fm,
            &LoopAction::Prepend {
                prop: "log".into(),
                value: json!("first"),
            },
        )
        .unwrap();
        assert_eq!(fm.get("log"), Some(&json!("first")));
    }

    #[test]
    fn prepend_handles_scalars_and_json_values() {
        let mut fm = object(json!({"log": "tail"}));
        apply_action(
            &mut fm,
            &LoopAction::Prepend {
                prop: "log".into(),
                value: json!(true),
            },
        )
        .unwrap();
        apply_action(
            &mut fm,
            &LoopAction::Prepend {
                prop: "log".into(),
                value: json!({"event": "tick"}),
            },
        )
        .unwrap();
        apply_action(
            &mut fm,
            &LoopAction::Prepend {
                prop: "log".into(),
                value: json!([1, 2]),
            },
        )
        .unwrap();

        assert_eq!(
            fm.get("log"),
            Some(&json!("[1,2]\n{\"event\":\"tick\"}\ntruetail"))
        );
    }

    #[test]
    fn prepend_empty_preserves_jsonl_shape() {
        let mut fm = object(json!({"objects": "{\"a\":1}", "arrays": "[1]"}));
        apply_action(
            &mut fm,
            &LoopAction::Prepend {
                prop: "objects".into(),
                value: Value::Null,
            },
        )
        .unwrap();
        apply_action(
            &mut fm,
            &LoopAction::Prepend {
                prop: "arrays".into(),
                value: json!(""),
            },
        )
        .unwrap();

        assert_eq!(fm.get("objects"), Some(&json!("{}\n{\"a\":1}")));
        assert_eq!(fm.get("arrays"), Some(&json!("[]\n[1]")));
    }

    #[test]
    fn merge_onto_null_creates_object() {
        let mut fm = object(json!({"state": null}));
        apply_action(
            &mut fm,
            &LoopAction::Merge {
                prop: "state".into(),
                value: json!({"a": 1}),
            },
        )
        .unwrap();
        assert_eq!(fm.get("state"), Some(&json!({"a": 1})));
    }

    #[test]
    fn merge_onto_missing_creates_object() {
        let mut fm = Map::new();
        apply_action(
            &mut fm,
            &LoopAction::Merge {
                prop: "state".into(),
                value: json!({"a": 1}),
            },
        )
        .unwrap();
        assert_eq!(fm.get("state"), Some(&json!({"a": 1})));
    }
}
