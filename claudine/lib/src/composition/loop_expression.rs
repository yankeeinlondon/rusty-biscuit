//! Loop condition expression evaluation.

use darkmatter::markdown::compose::expression::{
    EvaluationLookup, evaluate, is_truthy, parse_condition,
};
use serde_json::{Map, Value};

use super::error::CompositionError;
use super::types::LoopCondition;

/// Ambient values injected into loop condition and prompt evaluation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoopAmbient {
    /// 1-based iteration index.
    pub iteration: usize,
    /// Whether this is the first iteration.
    pub is_first: bool,
    /// Whether this iteration is expected to be the last.
    pub is_last: bool,
    /// Captured output from the previous iteration.
    pub last_output: String,
    /// Exit code from the previous iteration.
    pub last_exit_code: i32,
}

impl LoopAmbient {
    /// Construct ambient values for an iteration.
    pub fn new(
        iteration: usize,
        is_first: bool,
        is_last: bool,
        last_output: impl Into<String>,
        last_exit_code: i32,
    ) -> Self {
        Self {
            iteration,
            is_first,
            is_last,
            last_output: last_output.into(),
            last_exit_code,
        }
    }
}

/// Darkmatter expression lookup for loop frontmatter plus ambient variables.
///
/// Resolution order:
///
/// 1. `env.NAME`
/// 2. Ambient variables (`iteration`, `is_first`, `is_last`, `last_output`, `last_exit_code`)
/// 3. Frontmatter properties, including nested object paths via `.`
#[derive(Debug, Clone, Copy)]
pub struct LoopExpressionLookup<'a> {
    frontmatter: &'a Map<String, Value>,
    ambient: &'a LoopAmbient,
}

impl<'a> LoopExpressionLookup<'a> {
    /// Create a lookup over the current loop state.
    pub fn new(frontmatter: &'a Map<String, Value>, ambient: &'a LoopAmbient) -> Self {
        Self {
            frontmatter,
            ambient,
        }
    }
}

impl EvaluationLookup for LoopExpressionLookup<'_> {
    fn get(&self, path: &str) -> Option<Value> {
        if let Some(env_key) = path.strip_prefix("env.") {
            return resolve_env(env_key);
        }

        if let Some(value) = resolve_ambient(path, self.ambient) {
            return Some(value);
        }

        if let Some(value) = resolve_boolean_literal(path) {
            return Some(value);
        }

        resolve_frontmatter(self.frontmatter, path)
    }
}

/// Evaluate a loop condition and return whether the loop should continue.
///
/// `while` conditions continue when the expression is truthy. `until`
/// conditions continue while the expression is falsy.
///
/// ## Errors
///
/// Returns `LoopInvalid` when the condition cannot be parsed or evaluated.
pub fn evaluate_condition(
    condition: &LoopCondition,
    lookup: &LoopExpressionLookup<'_>,
) -> Result<bool, CompositionError> {
    let (kind, source) = match condition {
        LoopCondition::While(source) => ("while", source),
        LoopCondition::Until(source) => ("until", source),
    };

    let parsed = parse_condition(source).map_err(|error| {
        CompositionError::LoopInvalid(format!("failed to parse loop.{kind} `{source}`: {error}"))
    })?;
    let value = evaluate(&parsed, lookup).map_err(|error| {
        CompositionError::LoopInvalid(format!(
            "failed to evaluate loop.{kind} `{source}`: {error}"
        ))
    })?;
    let truthy = is_truthy(&value);

    Ok(match condition {
        LoopCondition::While(_) => truthy,
        LoopCondition::Until(_) => !truthy,
    })
}

fn resolve_env(name: &str) -> Option<Value> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return None;
    }
    std::env::var(trimmed).ok().map(Value::String)
}

fn resolve_ambient(path: &str, ambient: &LoopAmbient) -> Option<Value> {
    match path {
        "iteration" => Some(Value::Number(ambient.iteration.into())),
        "is_first" => Some(Value::Bool(ambient.is_first)),
        "is_last" => Some(Value::Bool(ambient.is_last)),
        "last_output" => Some(Value::String(ambient.last_output.clone())),
        "last_exit_code" => Some(Value::Number(ambient.last_exit_code.into())),
        _ => None,
    }
}

fn resolve_boolean_literal(path: &str) -> Option<Value> {
    match path {
        "true" => Some(Value::Bool(true)),
        "false" => Some(Value::Bool(false)),
        _ => None,
    }
}

fn resolve_frontmatter(frontmatter: &Map<String, Value>, path: &str) -> Option<Value> {
    if path.trim().is_empty() {
        return None;
    }

    if let Some(value) = frontmatter.get(path) {
        return Some(value.clone());
    }

    let mut parts = path.split('.');
    let head = parts.next()?;
    let mut current = frontmatter.get(head)?.clone();
    for part in parts {
        current = match current {
            Value::Object(map) => map.get(part).cloned()?,
            _ => return None,
        };
    }
    Some(current)
}

#[cfg(test)]
mod tests {
    use super::*;
    use darkmatter::markdown::compose::expression::EvaluationLookup;
    use serde_json::json;

    fn map(value: Value) -> Map<String, Value> {
        value.as_object().unwrap().clone()
    }

    fn ambient() -> LoopAmbient {
        LoopAmbient::new(1, true, false, "", 0)
    }

    #[test]
    fn resolves_ambient_variables() {
        let fm = map(json!({}));
        let ambient = LoopAmbient::new(3, false, true, "done", 7);
        let lookup = LoopExpressionLookup::new(&fm, &ambient);

        assert_eq!(lookup.get("iteration"), Some(json!(3)));
        assert_eq!(lookup.get("is_first"), Some(json!(false)));
        assert_eq!(lookup.get("is_last"), Some(json!(true)));
        assert_eq!(lookup.get("last_output"), Some(json!("done")));
        assert_eq!(lookup.get("last_exit_code"), Some(json!(7)));
    }

    #[test]
    fn ambient_variables_shadow_frontmatter() {
        let fm = map(json!({"iteration": 99, "is_first": false}));
        let ambient = ambient();
        let lookup = LoopExpressionLookup::new(&fm, &ambient);

        assert_eq!(lookup.get("iteration"), Some(json!(1)));
        assert_eq!(lookup.get("is_first"), Some(json!(true)));
    }

    #[test]
    fn resolves_top_level_and_nested_frontmatter() {
        let fm = map(json!({
            "counter": 4,
            "stage": "review",
            "state": {"inner": {"done": true}}
        }));
        let ambient = ambient();
        let lookup = LoopExpressionLookup::new(&fm, &ambient);

        assert_eq!(lookup.get("counter"), Some(json!(4)));
        assert_eq!(lookup.get("stage"), Some(json!("review")));
        assert_eq!(lookup.get("state.inner.done"), Some(json!(true)));
    }

    #[test]
    fn resolves_environment_variables() {
        let fm = map(json!({}));
        let ambient = ambient();
        let lookup = LoopExpressionLookup::new(&fm, &ambient);

        assert_eq!(
            lookup.get("env.PATH").is_some(),
            std::env::var("PATH").is_ok()
        );
    }

    #[test]
    fn evaluates_while_condition() {
        let ambient = ambient();
        let fm = map(json!({"counter": 3}));
        let lookup = LoopExpressionLookup::new(&fm, &ambient);
        assert!(evaluate_condition(&LoopCondition::While("counter < 5".into()), &lookup).unwrap());

        let fm = map(json!({"counter": 5}));
        let lookup = LoopExpressionLookup::new(&fm, &ambient);
        assert!(!evaluate_condition(&LoopCondition::While("counter < 5".into()), &lookup).unwrap());
    }

    #[test]
    fn evaluates_until_condition_as_continue_decision() {
        let ambient = ambient();
        let fm = map(json!({"done": false}));
        let lookup = LoopExpressionLookup::new(&fm, &ambient);
        assert!(evaluate_condition(&LoopCondition::Until("done".into()), &lookup).unwrap());

        let fm = map(json!({"done": true}));
        let lookup = LoopExpressionLookup::new(&fm, &ambient);
        assert!(!evaluate_condition(&LoopCondition::Until("done".into()), &lookup).unwrap());
    }

    #[test]
    fn evaluates_ambient_and_env_conditions() {
        let fm = map(json!({"counter": 1}));
        let ambient = ambient();
        let lookup = LoopExpressionLookup::new(&fm, &ambient);

        assert!(
            evaluate_condition(&LoopCondition::While("iteration == 1".into()), &lookup).unwrap()
        );
        assert!(
            evaluate_condition(&LoopCondition::While("is_first == true".into()), &lookup).unwrap()
        );
        assert_eq!(
            evaluate_condition(&LoopCondition::While("env.PATH != \"\"".into()), &lookup).unwrap(),
            std::env::var("PATH").is_ok_and(|value| !value.is_empty())
        );
    }

    #[test]
    fn parse_errors_are_loop_invalid() {
        let fm = map(json!({}));
        let ambient = ambient();
        let lookup = LoopExpressionLookup::new(&fm, &ambient);
        let err = evaluate_condition(&LoopCondition::While("counter <".into()), &lookup)
            .expect_err("condition should fail to parse");

        assert!(
            matches!(err, CompositionError::LoopInvalid(message) if message.contains("loop.while"))
        );
    }
}
