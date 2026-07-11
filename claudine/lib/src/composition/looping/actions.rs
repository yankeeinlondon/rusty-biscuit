//! Loop frontmatter action engine.

use darkmatter::markdown::compose::expression::{
    EvaluationLookup, ExpressionFinder, evaluate, parse, scalar_string,
};
use serde_json::{Map, Number, Value};

use super::super::error::CompositionError;
use super::super::json_util::json_type_name;
use super::super::types::{AmbientVariable, LoopAction};

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
    /// When `lookup` is `Some`, any `{{ ... }}` templates inside value-bearing
    /// actions (`set`, `append`, `prepend`, `merge`) are rendered against the
    /// provided evaluation lookup before the mutation is applied. The rendered
    /// output is re-parsed as JSON so that numeric, boolean, and `null` results
    /// land as their proper JSON types; non-JSON output falls back to a string.
    ///
    /// When `lookup` is `None`, action values are stored verbatim — preserving
    /// pre-template-rendering behavior for callers that lack iteration state.
    ///
    /// ## Errors
    ///
    /// Returns a contextual loop action error when the mutation is invalid.
    pub fn apply_action(
        &mut self,
        action: &LoopAction,
        action_index: usize,
        lookup: Option<&dyn EvaluationLookup>,
    ) -> Result<(), CompositionError> {
        let context = ActionContext {
            iteration: self.iteration,
            action_index,
            total_actions: self.total_actions,
        };
        apply_action_with_context(&mut self.frontmatter, action, context, lookup)
    }

    /// Commit the staged frontmatter copy.
    pub fn commit(self) -> Value {
        Value::Object(self.frontmatter)
    }

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
/// all-or-nothing. This entrypoint does not perform template rendering; values
/// containing `{{ ... }}` placeholders are stored verbatim.
pub fn apply_action(
    fm: &mut Map<String, Value>,
    action: &LoopAction,
) -> Result<(), CompositionError> {
    apply_action_with_context(fm, action, ActionContext::default(), None)
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
    lookup: Option<&dyn EvaluationLookup>,
) -> Result<(), CompositionError> {
    match action {
        LoopAction::Increment(prop) => apply_increment_with_context(fm, prop, context),
        LoopAction::Decrement(prop) => apply_decrement_with_context(fm, prop, context),
        LoopAction::Set { prop, value } => {
            let rendered = render_action_value(value, lookup, context)?;
            apply_set(fm, prop, &rendered, context)
        }
        LoopAction::Append { prop, value } => {
            let rendered = render_action_value(value, lookup, context)?;
            apply_append(fm, prop, &rendered)
        }
        LoopAction::Prepend { prop, value } => {
            let rendered = render_action_value(value, lookup, context)?;
            apply_prepend(fm, prop, &rendered)
        }
        LoopAction::Merge { prop, value } => {
            let rendered = render_action_value(value, lookup, context)?;
            apply_merge(fm, prop, &rendered, context)
        }
    }
}

/// Recursively render `{{ ... }}` templates in a `serde_json::Value`.
///
/// When `lookup` is `None`, returns a clone of the input unchanged.
///
/// String leaves containing templates are rendered through Darkmatter's
/// expression engine using the provided lookup, then re-parsed as JSON when
/// possible so numeric, boolean, and `null` template results land as their
/// proper JSON types. Strings without templates and non-string scalars are
/// passed through. Arrays and objects are walked recursively.
fn render_action_value(
    value: &Value,
    lookup: Option<&dyn EvaluationLookup>,
    context: ActionContext,
) -> Result<Value, CompositionError> {
    let Some(lookup) = lookup else {
        return Ok(value.clone());
    };
    render_value_with_lookup(value, lookup, context)
}

fn render_value_with_lookup(
    value: &Value,
    lookup: &dyn EvaluationLookup,
    context: ActionContext,
) -> Result<Value, CompositionError> {
    match value {
        Value::String(raw) => render_string_with_lookup(raw, lookup, context),
        Value::Array(arr) => arr
            .iter()
            .map(|v| render_value_with_lookup(v, lookup, context))
            .collect::<Result<Vec<_>, _>>()
            .map(Value::Array),
        Value::Object(obj) => {
            let mut new = Map::with_capacity(obj.len());
            for (key, val) in obj {
                new.insert(key.clone(), render_value_with_lookup(val, lookup, context)?);
            }
            Ok(Value::Object(new))
        }
        other => Ok(other.clone()),
    }
}

fn render_string_with_lookup(
    raw: &str,
    lookup: &dyn EvaluationLookup,
    context: ActionContext,
) -> Result<Value, CompositionError> {
    let locations = ExpressionFinder::find_all_plain(raw);
    if locations.is_empty() {
        return Ok(Value::String(raw.to_string()));
    }

    // When a single template span fills the entire string (e.g. `{{count}}`),
    // preserve the typed evaluation result directly.
    let single_span =
        locations.len() == 1 && locations[0].start == 0 && locations[0].end == raw.len();

    // `evaluate` is generic over a `Sized` lookup, so wrap the trait object in
    // a thin newtype that re-implements the trait by delegation.
    let sized_lookup = SizedLookup(lookup);

    let mut output = String::with_capacity(raw.len());
    let mut cursor = 0usize;
    let mut typed_single: Option<Value> = None;

    for loc in &locations {
        output.push_str(&raw[cursor..loc.start]);
        let expr_text = loc.expression.trim();
        let expr = parse(expr_text).map_err(|e| {
            invalid_action(
                context,
                format!("invalid template `{{{{{expr_text}}}}}` in loop action: {e}"),
            )
        })?;
        let value = evaluate(&expr, &sized_lookup).map_err(|e| {
            invalid_action(
                context,
                format!("failed to evaluate template `{{{{{expr_text}}}}}`: {e}"),
            )
        })?;
        if single_span {
            typed_single = Some(value);
        } else {
            output.push_str(&scalar_string(&value));
        }
        cursor = loc.end;
    }

    if let Some(value) = typed_single {
        return Ok(value);
    }

    output.push_str(&raw[cursor..]);
    Ok(serde_json::from_str(&output).unwrap_or(Value::String(output)))
}

/// Sized newtype around a borrowed `&dyn EvaluationLookup` so it can be passed
/// to `evaluate`, whose generic parameter requires `Sized`.
struct SizedLookup<'a>(&'a dyn EvaluationLookup);

impl<'a> EvaluationLookup for SizedLookup<'a> {
    fn get(&self, path: &str) -> Option<Value> {
        self.0.get(path)
    }

    fn get_string(&self, path: &str) -> String {
        self.0.get_string(path)
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
                value_excerpt: value_error_excerpt(value),
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
                value_excerpt: value_error_excerpt(value),
            })?
        }
    };
    fm.insert(prop.to_string(), next);
    Ok(())
}

fn value_error_excerpt(value: &Value) -> String {
    const MAX_WIDTH: usize = 80;

    if let Value::String(raw) = value
        && raw.contains("{{")
        && raw.contains("}}")
    {
        return format!(
            r#""{}" (unresolved template — the loop seed failed to resolve this property)"#,
            raw
        );
    }

    let json = serde_json::to_string(value).unwrap_or_else(|_| "<unserializable>".into());
    if json.len() > MAX_WIDTH {
        format!("{}…", &json[..MAX_WIDTH])
    } else {
        json
    }
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
    if super::super::reserved::SET_ACTION_BLOCKLIST.contains(&prop)
        || AmbientVariable::is_reserved(prop)
    {
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn object(value: Value) -> Map<String, Value> {
        value.as_object().unwrap().clone()
    }

    /// Minimal lookup that resolves a flat string→Value map. Used to drive the
    /// template-rendering tests without spinning up a full LoopExpressionLookup.
    struct MapLookup(Map<String, Value>);

    impl EvaluationLookup for MapLookup {
        fn get(&self, path: &str) -> Option<Value> {
            self.0.get(path).cloned()
        }
    }

    fn ctx() -> ActionContext {
        ActionContext::default()
    }

    // ── template rendering ────────────────────────────────────────────

    #[test]
    fn render_single_template_preserves_number_type() {
        let lookup = MapLookup(object(json!({"count": 3})));
        let value = Value::String("{{count}}".into());
        let rendered = render_action_value(&value, Some(&lookup), ctx()).unwrap();
        assert_eq!(rendered, json!(3));
    }

    #[test]
    fn render_single_template_preserves_bool_type() {
        let lookup = MapLookup(object(json!({"flag": true})));
        let value = Value::String("{{flag}}".into());
        let rendered = render_action_value(&value, Some(&lookup), ctx()).unwrap();
        assert_eq!(rendered, json!(true));
    }

    #[test]
    fn render_single_template_preserves_string_type() {
        let lookup = MapLookup(object(json!({"name": "alice"})));
        let value = Value::String("{{name}}".into());
        let rendered = render_action_value(&value, Some(&lookup), ctx()).unwrap();
        assert_eq!(rendered, json!("alice"));
    }

    #[test]
    fn render_template_inside_string_concatenates_to_string() {
        let lookup = MapLookup(object(json!({"count": 3})));
        let value = Value::String("iter-{{count}}".into());
        let rendered = render_action_value(&value, Some(&lookup), ctx()).unwrap();
        assert_eq!(rendered, json!("iter-3"));
    }

    #[test]
    fn render_multiple_templates_join_with_text_to_string() {
        let lookup = MapLookup(object(json!({"a": 1, "b": 2})));
        let value = Value::String("{{a}}+{{b}}".into());
        let rendered = render_action_value(&value, Some(&lookup), ctx()).unwrap();
        assert_eq!(rendered, json!("1+2"));
    }

    #[test]
    fn render_with_no_lookup_returns_input_unchanged() {
        let value = Value::String("{{count}}".into());
        let rendered = render_action_value(&value, None, ctx()).unwrap();
        assert_eq!(rendered, json!("{{count}}"));
    }

    #[test]
    fn render_string_without_templates_passes_through() {
        let lookup = MapLookup(object(json!({})));
        let value = Value::String("plain literal".into());
        let rendered = render_action_value(&value, Some(&lookup), ctx()).unwrap();
        assert_eq!(rendered, json!("plain literal"));
    }

    #[test]
    fn render_object_walks_into_string_leaves() {
        let lookup = MapLookup(object(json!({"count": 7, "stage": "review"})));
        let value = json!({
            "phase": "{{stage}}",
            "remaining": "{{count}}",
            "literal": 42
        });
        let rendered = render_action_value(&value, Some(&lookup), ctx()).unwrap();
        assert_eq!(
            rendered,
            json!({
                "phase": "review",
                "remaining": 7,
                "literal": 42
            })
        );
    }

    #[test]
    fn render_array_walks_into_string_leaves() {
        let lookup = MapLookup(object(json!({"x": 1, "y": 2})));
        let value = json!(["{{x}}", "{{y}}", "static"]);
        let rendered = render_action_value(&value, Some(&lookup), ctx()).unwrap();
        assert_eq!(rendered, json!([1, 2, "static"]));
    }

    #[test]
    fn render_invalid_template_returns_invalid_action_error() {
        let lookup = MapLookup(object(json!({})));
        let value = Value::String("{{ a &&& b }}".into());
        let err = render_action_value(&value, Some(&lookup), ctx()).unwrap_err();
        assert!(
            matches!(err, CompositionError::InvalidAction { ref message, .. } if message.contains("invalid template")),
            "got {err}"
        );
    }

    // ── set/append integration with rendering ─────────────────────────

    #[test]
    fn set_with_lookup_stores_typed_number() {
        let lookup = MapLookup(object(json!({"count": 4})));
        let mut fm = Map::new();
        let mut stage = ActionStaging::new(&fm.clone(), 1, 1);
        stage
            .apply_action(
                &LoopAction::Set {
                    prop: "stamp".into(),
                    value: Value::String("{{count}}".into()),
                },
                1,
                Some(&lookup),
            )
            .unwrap();
        fm = stage.commit_map();
        assert_eq!(fm.get("stamp"), Some(&json!(4)));
    }

    #[test]
    fn set_without_lookup_stores_raw_template_string() {
        let mut fm = Map::new();
        let mut stage = ActionStaging::new(&fm.clone(), 1, 1);
        stage
            .apply_action(
                &LoopAction::Set {
                    prop: "stamp".into(),
                    value: Value::String("{{count}}".into()),
                },
                1,
                None,
            )
            .unwrap();
        fm = stage.commit_map();
        assert_eq!(fm.get("stamp"), Some(&json!("{{count}}")));
    }

    #[test]
    fn append_with_lookup_renders_template_into_string() {
        let lookup = MapLookup(object(json!({"count": 2, "out": "ran-2"})));
        let mut fm = object(json!({"log": ""}));
        let mut stage = ActionStaging::new(&fm.clone(), 2, 1);
        stage
            .apply_action(
                &LoopAction::Append {
                    prop: "log".into(),
                    value: Value::String("iter {{count}}: {{out}}\n".into()),
                },
                1,
                Some(&lookup),
            )
            .unwrap();
        fm = stage.commit_map();
        assert_eq!(fm.get("log"), Some(&json!("iter 2: ran-2\n")));
    }

    #[test]
    fn merge_with_lookup_renders_inside_object_value() {
        let lookup = MapLookup(object(json!({"count": 5})));
        let mut fm = object(json!({"state": {"phase": "draft"}}));
        let mut stage = ActionStaging::new(&fm.clone(), 5, 1);
        stage
            .apply_action(
                &LoopAction::Merge {
                    prop: "state".into(),
                    value: json!({"iteration": "{{count}}", "phase": "review"}),
                },
                1,
                Some(&lookup),
            )
            .unwrap();
        fm = stage.commit_map();
        assert_eq!(
            fm.get("state"),
            Some(&json!({"phase": "review", "iteration": 5}))
        );
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
                found,
                ..
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
        for prop in ["loop", "replace", "_loop_count", "_loop_is_first"] {
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
        stage.apply_action(&actions[0], 1, None).unwrap();
        let err = stage.apply_action(&actions[1], 2, None).unwrap_err();

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
            stage.apply_action(action, index + 1, None).unwrap();
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
                found,
                ..
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
                found,
                ..
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
                found,
                ..
            } if property == "counter" && found == "string"
        ));
    }

    #[test]
    fn increment_error_includes_unresolved_template_excerpt() {
        let template = "{{ frontmatter(plan, 'start_phase') || 1 }}";
        let mut fm = object(json!({"phase": template}));
        let err = apply_increment(&mut fm, "phase").unwrap_err();
        let (property, found, excerpt) = match err {
            CompositionError::InvalidIncrementType {
                property,
                found,
                value_excerpt,
                ..
            } => (property, found, value_excerpt),
            other => panic!("expected InvalidIncrementType, got {other}"),
        };
        assert_eq!(property, "phase");
        assert_eq!(found, "string");
        assert!(
            excerpt.contains("{{ frontmatter(plan, 'start_phase')"),
            "excerpt should quote the template start: {excerpt}"
        );
        assert!(
            excerpt.contains("unresolved template"),
            "excerpt should explain the stage: {excerpt}"
        );
    }

    #[test]
    fn increment_error_includes_resolved_non_numeric_excerpt() {
        let mut fm = object(json!({"area": "claudine-cli"}));
        let err = apply_increment(&mut fm, "area").unwrap_err();
        let (property, found, excerpt) = match err {
            CompositionError::InvalidIncrementType {
                property,
                found,
                value_excerpt,
                ..
            } => (property, found, value_excerpt),
            other => panic!("expected InvalidIncrementType, got {other}"),
        };
        assert_eq!(property, "area");
        assert_eq!(found, "string");
        assert!(excerpt.contains("claudine-cli"), "excerpt should quote the value: {excerpt}");
    }

    #[test]
    fn decrement_error_includes_unresolved_template_excerpt() {
        let template = "{{ frontmatter(plan, 'start_phase') || 1 }}";
        let mut fm = object(json!({"phase": template}));
        let err = apply_decrement(&mut fm, "phase").unwrap_err();
        let (property, found, excerpt) = match err {
            CompositionError::InvalidDecrementType {
                property,
                found,
                value_excerpt,
                ..
            } => (property, found, value_excerpt),
            other => panic!("expected InvalidDecrementType, got {other}"),
        };
        assert_eq!(property, "phase");
        assert_eq!(found, "string");
        assert!(
            excerpt.contains("{{ frontmatter(plan, 'start_phase')"),
            "excerpt should quote the template start: {excerpt}"
        );
        assert!(
            excerpt.contains("unresolved template"),
            "excerpt should explain the stage: {excerpt}"
        );
    }

    #[test]
    fn decrement_error_includes_resolved_non_numeric_excerpt() {
        let mut fm = object(json!({"area": "claudine-cli"}));
        let err = apply_decrement(&mut fm, "area").unwrap_err();
        let (property, found, excerpt) = match err {
            CompositionError::InvalidDecrementType {
                property,
                found,
                value_excerpt,
                ..
            } => (property, found, value_excerpt),
            other => panic!("expected InvalidDecrementType, got {other}"),
        };
        assert_eq!(property, "area");
        assert_eq!(found, "string");
        assert!(excerpt.contains("claudine-cli"), "excerpt should quote the value: {excerpt}");
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
