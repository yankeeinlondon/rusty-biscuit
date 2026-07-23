use super::*;
use darkmatter::markdown::{Frontmatter, Markdown};
use serde_json::json;
use std::fs;
use tempfile::TempDir;

fn make_source(frontmatter: &[(&str, serde_json::Value)]) -> ResolvedCompositionSource {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("loop.md");
    let mut fm = Frontmatter::new();
    for (key, value) in frontmatter {
        fm.insert(key, value.clone()).unwrap();
    }
    let md = Markdown::with_frontmatter(fm, "Body");
    fs::write(&file, md.as_string()).unwrap();
    let original_text = fs::read_to_string(&file).unwrap();
    let markdown: Markdown = original_text.clone().into();
    ResolvedCompositionSource {
        original_ref: file.to_string_lossy().to_string(),
        resolved_path: file,
        original_text,
        markdown,
    }
}

#[test]
fn singular_action_key_is_accepted() {
    let source = make_source(&[(
        "loop",
        json!({"until": "done", "action": "increment(counter)"}),
    )]);
    let config = resolve_loop_config(&source).unwrap().unwrap();
    assert_eq!(
        config.actions,
        vec![LoopAction::Increment("counter".into())]
    );
}

#[test]
fn plural_actions_key_is_accepted_as_alias() {
    let source = make_source(&[(
        "loop",
        json!({"until": "done", "actions": "increment(counter)"}),
    )]);
    let config = resolve_loop_config(&source).unwrap().unwrap();
    assert_eq!(
        config.actions,
        vec![LoopAction::Increment("counter".into())]
    );
}

#[test]
fn action_and_actions_together_are_rejected() {
    let source = make_source(&[(
        "loop",
        json!({
            "until": "done",
            "action": "increment(a)",
            "actions": "increment(b)"
        }),
    )]);
    let err = resolve_loop_config(&source).unwrap_err();
    let CompositionError::LoopInvalid(message) = err else {
        panic!("expected LoopInvalid");
    };
    assert!(
        message.contains("aliases; specify only one"),
        "got: {message}"
    );
}

#[test]
fn unknown_loop_key_without_suggestion_lists_valid_keys() {
    let source = make_source(&[("loop", json!({"until": "done", "frequency": 5}))]);
    let err = resolve_loop_config(&source).unwrap_err();
    let CompositionError::LoopInvalid(message) = err else {
        panic!("expected LoopInvalid");
    };
    assert!(
        message.contains("unknown `loop.frequency` key"),
        "got: {message}"
    );
    assert!(message.contains("actions"), "got: {message}");
    assert!(!message.contains("did you mean"), "got: {message}");
}

#[test]
fn no_loop_returns_none() {
    let source = make_source(&[("title", json!("No loop"))]);
    assert!(resolve_loop_config(&source).unwrap().is_none());
}

#[test]
fn loop_stdout_is_accepted_as_lifecycle_concern() {
    // `loop.stdout` is a lifecycle-concern key, not an iteration control.
    // `resolve_loop_config` must accept it (ignoring it as a concern) rather
    // than rejecting it as an unknown `LoopInvalid` key. The `while` key
    // keeps the iteration controls valid.
    let source = make_source(&[("loop", json!({"while": "true", "stdout": "hello"}))]);
    let config = resolve_loop_config(&source).unwrap();
    assert!(config.is_some());
}

#[test]
fn scalar_dsl_action_parses() {
    let source = make_source(&[(
        "loop",
        json!({
            "while": "counter < 3",
            "actions": "increment(counter)",
            "max": 3,
            "fail_fast": false
        }),
    )]);

    let config = resolve_loop_config(&source).unwrap().unwrap();
    assert_eq!(config.condition, LoopCondition::While("counter < 3".into()));
    assert_eq!(
        config.actions,
        vec![LoopAction::Increment("counter".into())]
    );
    assert_eq!(config.max_iterations, Some(3));
    assert_eq!(config.fail_fast, Some(false));
}

#[test]
fn list_of_dsl_actions_parses() {
    let source = make_source(&[(
        "loop",
        json!({
            "until": "done",
            "actions": [
                "append(log, {\"event\":\"tick\"})",
                "set(done, true)"
            ]
        }),
    )]);

    let config = resolve_loop_config(&source).unwrap().unwrap();
    assert_eq!(config.condition, LoopCondition::Until("done".into()));
    assert_eq!(
        config.actions,
        vec![
            LoopAction::Append {
                prop: "log".into(),
                value: json!({"event": "tick"})
            },
            LoopAction::Set {
                prop: "done".into(),
                value: json!(true)
            }
        ]
    );
}

#[test]
fn structured_actions_parse() {
    let source = make_source(&[(
        "loop",
        json!({
            "while": "counter < 5",
            "actions": [
                {"op": "merge", "prop": "state", "value": {"phase": "review"}},
                {"op": "decrement", "prop": "remaining"}
            ]
        }),
    )]);

    let config = resolve_loop_config(&source).unwrap().unwrap();
    assert_eq!(
        config.actions,
        vec![
            LoopAction::Merge {
                prop: "state".into(),
                value: json!({"phase": "review"})
            },
            LoopAction::Decrement("remaining".into())
        ]
    );
}

#[test]
fn condition_keys_are_mutually_exclusive() {
    let source = make_source(&[("loop", json!({"while": "true", "until": "done"}))]);
    let err = resolve_loop_config(&source).unwrap_err();
    assert!(
        matches!(err, CompositionError::LoopInvalid(msg) if msg.contains("mutually exclusive"))
    );
}

#[test]
fn condition_is_required() {
    let source = make_source(&[("loop", json!({"actions": "increment(counter)"}))]);
    let err = resolve_loop_config(&source).unwrap_err();
    assert!(
        matches!(err, CompositionError::LoopInvalid(msg) if msg.contains("either `while` or `until`"))
    );
}

#[test]
fn invalid_action_syntax_fails() {
    let source = make_source(&[(
        "loop",
        json!({"while": "true", "actions": ["increment(counter)", "wat"]}),
    )]);
    let err = resolve_loop_config(&source).unwrap_err();
    assert!(
        matches!(err, CompositionError::LoopInvalid(ref msg) if msg.contains("loop.actions[1]")),
        "got {err}"
    );
}

#[test]
fn max_must_be_positive() {
    let source = make_source(&[("loop", json!({"while": "true", "max": 0}))]);
    let err = resolve_loop_config(&source).unwrap_err();
    assert!(
        matches!(err, CompositionError::LoopInvalid(msg) if msg.contains("greater than zero"))
    );
}

#[test]
fn fail_fast_must_be_boolean() {
    let source = make_source(&[("loop", json!({"while": "true", "fail_fast": "false"}))]);
    let err = resolve_loop_config(&source).unwrap_err();
    assert!(
        matches!(err, CompositionError::LoopInvalid(msg) if msg.contains("loop.fail_fast"))
    );
}

#[test]
fn on_rate_limit_pause_parses() {
    let source = make_source(&[("loop", json!({"while": "true", "on_rate_limit": "pause"}))]);
    let config = resolve_loop_config(&source).unwrap().unwrap();
    assert_eq!(config.on_rate_limit, Some(OnRateLimit::Pause));
}

#[test]
fn on_rate_limit_abort_parses() {
    let source = make_source(&[("loop", json!({"while": "true", "on_rate_limit": "abort"}))]);
    let config = resolve_loop_config(&source).unwrap().unwrap();
    assert_eq!(config.on_rate_limit, Some(OnRateLimit::Abort));
}

#[test]
fn on_rate_limit_continue_parses() {
    let source = make_source(&[(
        "loop",
        json!({"while": "true", "on_rate_limit": "continue"}),
    )]);
    let config = resolve_loop_config(&source).unwrap().unwrap();
    assert_eq!(config.on_rate_limit, Some(OnRateLimit::Continue));
}

#[test]
fn on_rate_limit_unknown_value_is_rejected() {
    let source = make_source(&[("loop", json!({"while": "true", "on_rate_limit": "halt"}))]);
    let err = resolve_loop_config(&source).unwrap_err();
    let CompositionError::LoopInvalid(message) = err else {
        panic!("expected LoopInvalid");
    };
    assert!(
        message.contains("loop.on_rate_limit") && message.contains("halt"),
        "got: {message}"
    );
}

#[test]
fn on_rate_limit_non_string_is_rejected() {
    let source = make_source(&[("loop", json!({"while": "true", "on_rate_limit": true}))]);
    let err = resolve_loop_config(&source).unwrap_err();
    assert!(
        matches!(err, CompositionError::LoopInvalid(msg) if msg.contains("must be a string"))
    );
}

#[test]
fn on_rate_limit_typo_suggests_canonical_key() {
    let source = make_source(&[("loop", json!({"while": "true", "on-rate-limit": "abort"}))]);
    let err = resolve_loop_config(&source).unwrap_err();
    let CompositionError::LoopInvalid(message) = err else {
        panic!("expected LoopInvalid");
    };
    assert!(message.contains("on_rate_limit"), "got: {message}");
}

#[test]
fn on_rate_limit_default_is_none() {
    let source = make_source(&[("loop", json!({"while": "true"}))]);
    let config = resolve_loop_config(&source).unwrap().unwrap();
    assert_eq!(config.on_rate_limit, None);
}

#[test]
fn action_object_value_is_required_for_value_ops() {
    let source = make_source(&[(
        "loop",
        json!({"while": "true", "actions": {"op": "set", "prop": "done"}}),
    )]);
    let err = resolve_loop_config(&source).unwrap_err();
    assert!(
        matches!(err, CompositionError::LoopInvalid(msg) if msg.contains("missing `value`"))
    );
}

#[test]
fn max_must_be_positive_integer() {
    let source = make_source(&[("loop", json!({"while": "true", "max": "five"}))]);
    let err = resolve_loop_config(&source).unwrap_err();
    assert!(
        matches!(err, CompositionError::LoopInvalid(msg) if msg.contains("loop.max") && msg.contains("positive integer"))
    );
}

#[test]
fn actions_must_be_string_object_or_list() {
    let source = make_source(&[("loop", json!({"while": "true", "actions": 42}))]);
    let err = resolve_loop_config(&source).unwrap_err();
    assert!(
        matches!(err, CompositionError::LoopInvalid(msg) if msg.contains("loop.actions") && msg.contains("string, object, or list"))
    );
}

#[test]
fn invalid_dsl_action_missing_closing_paren() {
    let source = make_source(&[(
        "loop",
        json!({"while": "true", "actions": "increment(counter"}),
    )]);
    let err = resolve_loop_config(&source).unwrap_err();
    assert!(
        matches!(err, CompositionError::LoopInvalid(ref msg) if msg.contains("missing closing")),
        "got {err}"
    );
}

#[test]
fn invalid_dsl_action_unknown_op() {
    let source = make_source(&[("loop", json!({"while": "true", "actions": "unknown(prop)"}))]);
    let err = resolve_loop_config(&source).unwrap_err();
    assert!(
        matches!(err, CompositionError::LoopInvalid(ref msg) if msg.contains("unknown loop action op")),
        "got {err}"
    );
}

#[test]
fn invalid_dsl_action_wrong_arg_count() {
    let source = make_source(&[("loop", json!({"while": "true", "actions": "increment()"}))]);
    let err = resolve_loop_config(&source).unwrap_err();
    assert!(
        matches!(err, CompositionError::LoopInvalid(ref msg) if msg.contains("expects 1 argument")),
        "got {err}"
    );
}

#[test]
fn structured_action_missing_op() {
    let source = make_source(&[(
        "loop",
        json!({"while": "true", "actions": {"prop": "counter"}}),
    )]);
    let err = resolve_loop_config(&source).unwrap_err();
    assert!(matches!(err, CompositionError::LoopInvalid(msg) if msg.contains("missing `op`")));
}

#[test]
fn structured_action_missing_prop() {
    let source = make_source(&[(
        "loop",
        json!({"while": "true", "actions": {"op": "increment"}}),
    )]);
    let err = resolve_loop_config(&source).unwrap_err();
    assert!(
        matches!(err, CompositionError::LoopInvalid(msg) if msg.contains("missing `prop`"))
    );
}

#[test]
fn structured_action_unknown_op() {
    let source = make_source(&[(
        "loop",
        json!({"while": "true", "actions": {"op": "unknown", "prop": "counter"}}),
    )]);
    let err = resolve_loop_config(&source).unwrap_err();
    assert!(
        matches!(err, CompositionError::LoopInvalid(ref msg) if msg.contains("unknown loop action op")),
        "got {err}"
    );
}

#[test]
fn empty_loop_object_requires_condition() {
    let source = make_source(&[("loop", json!({}))]);
    let err = resolve_loop_config(&source).unwrap_err();
    assert!(
        matches!(err, CompositionError::LoopInvalid(ref msg) if msg.contains("either `while` or `until`"))
    );
}

#[test]
fn loop_must_be_an_object() {
    let source = make_source(&[("loop", json!("while counter < 3"))]);
    let err = resolve_loop_config(&source).unwrap_err();
    assert!(
        matches!(err, CompositionError::LoopInvalid(ref msg) if msg.contains("must be an object")),
        "got {err}"
    );
}

#[test]
fn scalar_object_action_parses_json_value() {
    let source = make_source(&[(
        "loop",
        json!({
            "while": "counter < 3",
            "actions": "set(counter, 5)"
        }),
    )]);

    let config = resolve_loop_config(&source).unwrap().unwrap();
    assert_eq!(
        config.actions,
        vec![LoopAction::Set {
            prop: "counter".into(),
            value: json!(5)
        }]
    );
}

#[test]
fn scalar_string_action_parses_quoted_string() {
    let source = make_source(&[(
        "loop",
        json!({
            "while": "counter < 3",
            "actions": "set(msg, 'hello world')"
        }),
    )]);

    let config = resolve_loop_config(&source).unwrap().unwrap();
    assert_eq!(
        config.actions,
        vec![LoopAction::Set {
            prop: "msg".into(),
            value: json!("hello world")
        }]
    );
}

// ── Control-variable extraction tests ────────────────────────────────

fn control_config(condition: &str, actions: Vec<LoopAction>) -> LoopConfig {
    LoopConfig {
        condition: LoopCondition::Until(condition.to_string()),
        actions,
        max_iterations: None,
        fail_fast: None,
        on_rate_limit: None,
    }
}

#[test]
fn extract_control_variables_repro_shape() {
    let config = control_config(
        "phase > total_phases",
        vec![LoopAction::Increment("phase".into())],
    );
    assert_eq!(
        extract_control_variables(&config),
        vec!["phase".to_string(), "total_phases".to_string()]
    );
}

#[test]
fn extract_control_variables_action_value_template() {
    let config = control_config(
        "phase < max",
        vec![LoopAction::Set {
            prop: "next".into(),
            value: json!("{{ phase + 1 }}"),
        }],
    );
    assert_eq!(
        extract_control_variables(&config),
        vec!["max".to_string(), "next".to_string(), "phase".to_string()]
    );
}

#[test]
fn extract_control_variables_excludes_reserved_namespaces() {
    let config = control_config("_loop_count < 3 && env.DEBUG", vec![]);
    assert!(extract_control_variables(&config).is_empty());
}

#[test]
fn extract_control_variables_dotted_condition_path() {
    let config = control_config("state.done", vec![]);
    assert_eq!(
        extract_control_variables(&config),
        vec!["state".to_string()]
    );
}

#[test]
fn extract_control_variables_empty_identity() {
    let config = control_config("true", vec![]);
    assert!(extract_control_variables(&config).is_empty());
}

#[test]
fn extract_control_variables_lifts_doc_namespace_head() {
    let config = control_config("doc.counter < doc.total", vec![]);
    assert_eq!(
        extract_control_variables(&config),
        vec!["counter".to_string(), "total".to_string()]
    );
}

#[test]
fn extract_control_variables_dotted_doc_path_lifts_head_only() {
    let config = control_config("doc.config.retries > 0", vec![]);
    assert_eq!(
        extract_control_variables(&config),
        vec!["config".to_string()]
    );
}

#[test]
fn extract_control_variables_bare_doc_lifts_nothing() {
    let config = control_config("doc && counter < 3", vec![]);
    assert_eq!(
        extract_control_variables(&config),
        vec!["counter".to_string()]
    );
}

#[test]
fn extract_control_variables_doc_and_action_target_merge() {
    let config = control_config(
        "doc.counter < doc.total",
        vec![LoopAction::Increment("counter".into())],
    );
    assert_eq!(
        extract_control_variables(&config),
        vec!["counter".to_string(), "total".to_string()]
    );
}
