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
    let CompositionError::LoopInvalid { message, .. } = err else {
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
    let CompositionError::LoopInvalid { message, .. } = err else {
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
            "max": 3
        }),
    )]);

    let config = resolve_loop_config(&source).unwrap().unwrap();
    assert_eq!(config.condition, LoopCondition::While("counter < 3".into()));
    assert_eq!(
        config.actions,
        vec![LoopAction::Increment("counter".into())]
    );
    assert_eq!(config.max_iterations, Some(3));
    assert_eq!(config.max_template, None);
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
        matches!(err, CompositionError::LoopInvalid { message: msg, .. } if msg.contains("mutually exclusive"))
    );
}

#[test]
fn condition_is_required() {
    let source = make_source(&[("loop", json!({"actions": "increment(counter)"}))]);
    let err = resolve_loop_config(&source).unwrap_err();
    assert!(
        matches!(err, CompositionError::LoopInvalid { message: msg, .. } if msg.contains("either `while` or `until`"))
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
        matches!(err, CompositionError::LoopInvalid { message: ref msg, .. } if msg.contains("loop.actions[1]")),
        "got {err}"
    );
}

#[test]
fn max_must_be_positive() {
    let source = make_source(&[("loop", json!({"while": "true", "max": 0}))]);
    let err = resolve_loop_config(&source).unwrap_err();
    assert!(
        matches!(err, CompositionError::LoopInvalid { message: msg, .. } if msg.contains("greater than zero"))
    );
}

#[test]
fn fail_fast_in_any_shape_is_a_removed_key() {
    for value in [json!(true), json!(false), json!(null), json!("false")] {
        let source = make_source(&[("loop", json!({"while": "true", "fail_fast": value}))]);
        let err = resolve_loop_config(&source).unwrap_err();
        let CompositionError::RemovedGrammar { path, removed, replacement, source_path } = &err else {
            panic!("expected RemovedGrammar for {value}, got {err:?}");
        };
        assert_eq!(path, "loop.fail_fast");
        assert_eq!(removed, "fail_fast");
        assert_eq!(source_path, &source.resolved_path);
        assert!(
            replacement.contains("unless a `failure` event handles it")
                && replacement.contains("`fail_fast` is a group key"),
            "names the replacement: {replacement}"
        );
    }
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
    let CompositionError::LoopInvalid { message, .. } = err else {
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
        matches!(err, CompositionError::LoopInvalid { message: msg, .. } if msg.contains("must be a string"))
    );
}

#[test]
fn on_rate_limit_typo_suggests_canonical_key() {
    let source = make_source(&[("loop", json!({"while": "true", "on-rate-limit": "abort"}))]);
    let err = resolve_loop_config(&source).unwrap_err();
    let CompositionError::LoopInvalid { message, .. } = err else {
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
        matches!(err, CompositionError::LoopInvalid { message: msg, .. } if msg.contains("missing `value`"))
    );
}

#[test]
fn max_must_be_positive_integer() {
    let source = make_source(&[("loop", json!({"while": "true", "max": "five"}))]);
    let err = resolve_loop_config(&source).unwrap_err();
    assert!(
        matches!(err, CompositionError::LoopInvalid { message: msg, .. } if msg.contains("loop.max") && msg.contains("positive integer"))
    );
}

#[test]
fn actions_must_be_string_object_or_list() {
    let source = make_source(&[("loop", json!({"while": "true", "actions": 42}))]);
    let err = resolve_loop_config(&source).unwrap_err();
    assert!(
        matches!(err, CompositionError::LoopInvalid { message: msg, .. } if msg.contains("loop.actions") && msg.contains("string, object, or list"))
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
        matches!(err, CompositionError::LoopInvalid { message: ref msg, .. } if msg.contains("missing closing")),
        "got {err}"
    );
}

#[test]
fn invalid_dsl_action_unknown_op() {
    let source = make_source(&[("loop", json!({"while": "true", "actions": "unknown(prop)"}))]);
    let err = resolve_loop_config(&source).unwrap_err();
    assert!(
        matches!(err, CompositionError::LoopInvalid { message: ref msg, .. } if msg.contains("unknown loop action op")),
        "got {err}"
    );
}

#[test]
fn invalid_dsl_action_wrong_arg_count() {
    let source = make_source(&[("loop", json!({"while": "true", "actions": "increment()"}))]);
    let err = resolve_loop_config(&source).unwrap_err();
    assert!(
        matches!(err, CompositionError::LoopInvalid { message: ref msg, .. } if msg.contains("expects 1 argument")),
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
    assert!(matches!(err, CompositionError::LoopInvalid { message: msg, .. } if msg.contains("missing `op`")));
}

#[test]
fn structured_action_missing_prop() {
    let source = make_source(&[(
        "loop",
        json!({"while": "true", "actions": {"op": "increment"}}),
    )]);
    let err = resolve_loop_config(&source).unwrap_err();
    assert!(
        matches!(err, CompositionError::LoopInvalid { message: msg, .. } if msg.contains("missing `prop`"))
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
        matches!(err, CompositionError::LoopInvalid { message: ref msg, .. } if msg.contains("unknown loop action op")),
        "got {err}"
    );
}

#[test]
fn empty_loop_object_requires_condition() {
    let source = make_source(&[("loop", json!({}))]);
    let err = resolve_loop_config(&source).unwrap_err();
    assert!(
        matches!(err, CompositionError::LoopInvalid { message: ref msg, .. } if msg.contains("either `while` or `until`"))
    );
}

#[test]
fn loop_must_be_an_object() {
    let source = make_source(&[("loop", json!("while counter < 3"))]);
    let err = resolve_loop_config(&source).unwrap_err();
    assert!(
        matches!(err, CompositionError::LoopInvalid { message: ref msg, .. } if msg.contains("must be an object")),
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
        max_template: None,
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
    // `loop` is the host-declared global, never a frontmatter control
    // variable, however it is read.
    let config = control_config(
        "loop.count < 3 && loop['is_last'] && env.DEBUG",
        vec![],
    );
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
fn extract_control_variables_lifts_fm_namespace_head() {
    let config = control_config("fm.counter < fm.total", vec![]);
    assert_eq!(
        extract_control_variables(&config),
        vec!["counter".to_string(), "total".to_string()]
    );
}

#[test]
fn extract_control_variables_dotted_fm_path_lifts_head_only() {
    let config = control_config("fm.config.retries > 0", vec![]);
    assert_eq!(
        extract_control_variables(&config),
        vec!["config".to_string()]
    );
}

#[test]
fn extract_control_variables_bare_fm_lifts_nothing() {
    let config = control_config("fm && counter < 3", vec![]);
    assert_eq!(
        extract_control_variables(&config),
        vec!["counter".to_string()]
    );
}

#[test]
fn extract_control_variables_fm_and_action_target_merge() {
    let config = control_config(
        "fm.counter < fm.total",
        vec![LoopAction::Increment("counter".into())],
    );
    assert_eq!(
        extract_control_variables(&config),
        vec!["counter".to_string(), "total".to_string()]
    );
}

// ── Located errors, key checks, and owners ───────────────────────────

fn loop_error(loop_value: serde_json::Value) -> (CompositionError, ResolvedCompositionSource) {
    let source = make_source(&[("loop", loop_value)]);
    let err = resolve_loop_config(&source).unwrap_err();
    (err, source)
}

/// The `(source_path, property, message)` of a `LoopInvalid`.
fn invalid_parts(err: &CompositionError) -> (&std::path::Path, &str, &str) {
    let CompositionError::LoopInvalid { source_path, property, message } = err else {
        panic!("expected LoopInvalid, got {err:?}");
    };
    (source_path, property, message)
}

#[test]
fn loop_null_is_rejected_naming_the_document() {
    let (err, source) = loop_error(json!(null));
    let (path, property, message) = invalid_parts(&err);
    assert_eq!(path, source.resolved_path);
    assert_eq!(property, "loop");
    assert!(message.contains("got null"), "got: {message}");
    assert!(err.to_string().contains("loop.md"), "got: {err}");
}

#[test]
fn null_controls_are_rejected_at_their_key() {
    for (key, block) in [
        ("loop.until", json!({"until": null})),
        ("loop.while", json!({"while": null})),
        ("loop.max", json!({"while": "true", "max": null})),
        ("loop.action", json!({"while": "true", "action": null})),
        ("loop.actions", json!({"while": "true", "actions": null})),
        ("loop.on_rate_limit", json!({"while": "true", "on_rate_limit": null})),
    ] {
        let (err, source) = loop_error(block);
        let (path, property, message) = invalid_parts(&err);
        assert_eq!(path, source.resolved_path, "{key}");
        assert_eq!(property, key);
        assert!(message.contains(key) && message.contains("null"), "{key}: {message}");
    }
}

#[test]
fn empty_condition_is_rejected() {
    for block in [json!({"until": ""}), json!({"while": "  "})] {
        let (err, _) = loop_error(block);
        let (_, _, message) = invalid_parts(&err);
        assert!(message.contains("must not be empty"), "got: {message}");
    }
}

#[test]
fn both_conditions_are_rejected_naming_both() {
    let (err, _) = loop_error(json!({"until": "a", "while": "b"}));
    let (_, property, message) = invalid_parts(&err);
    assert_eq!(property, "loop");
    assert!(message.contains("`loop.while` and `loop.until`"), "got: {message}");
}

#[test]
fn both_action_aliases_are_rejected_at_actions() {
    let (err, _) = loop_error(json!({"until": "a", "action": "increment(n)", "actions": []}));
    let (_, property, _) = invalid_parts(&err);
    assert_eq!(property, "loop.actions");
}

#[test]
fn empty_action_string_is_rejected() {
    let (err, _) = loop_error(json!({"until": "a", "action": ""}));
    let (_, property, message) = invalid_parts(&err);
    assert_eq!(property, "loop.action");
    assert!(message.contains("use `[]` for no action"), "got: {message}");
}

#[test]
fn empty_action_list_is_an_explicit_no_op() {
    let source = make_source(&[("loop", json!({"until": "a", "action": []}))]);
    assert!(resolve_loop_config(&source).unwrap().unwrap().actions.is_empty());
}

#[test]
fn action_list_error_names_the_element() {
    let (err, _) = loop_error(json!({"until": "a", "action": ["increment(n)", 5]}));
    let (_, property, _) = invalid_parts(&err);
    assert_eq!(property, "loop.action[1]");
}

#[test]
fn misspelled_key_gets_an_edit_distance_hint() {
    for (typo, expected) in [("untill", "until"), ("whlie", "while"), ("gaet", "gate"), ("stdrr", "stderr")] {
        let (err, _) = loop_error(json!({"until": "a", typo: "x"}));
        let (_, property, message) = invalid_parts(&err);
        assert_eq!(property, format!("loop.{typo}"));
        assert!(message.contains(&format!("did you mean `{expected}`?")), "{typo}: {message}");
    }
}

#[test]
fn fail_fast_is_neither_suggested_nor_listed() {
    let (err, _) = loop_error(json!({"until": "a", "failfast": true}));
    let (_, _, message) = invalid_parts(&err);
    assert!(!message.contains("fail_fast"), "got: {message}");
}

#[test]
fn templated_max_waits_for_resolution() {
    let source = make_source(&[("loop", json!({"until": "a", "max": "{{ cap }}"}))]);
    let config = resolve_loop_config(&source).unwrap().unwrap();
    assert_eq!(config.max_iterations, None);
    assert_eq!(config.max_template.as_deref(), Some("{{ cap }}"));
}

#[test]
fn mixed_text_max_is_not_a_template() {
    let (err, _) = loop_error(json!({"until": "a", "max": "{{ cap }}0"}));
    let (_, property, message) = invalid_parts(&err);
    assert_eq!(property, "loop.max");
    assert!(message.contains("positive integer"), "got: {message}");
}

/// Reads frontmatter values by bare name, as a cap template does.
struct ValuesLookup(serde_json::Map<String, serde_json::Value>);

impl darkmatter::markdown::compose::expression::EvaluationLookup for ValuesLookup {
    fn get(&self, path: &str) -> Option<serde_json::Value> {
        self.0.get(path).cloned()
    }
}

fn resolve_cap(values: serde_json::Value) -> Result<LoopConfig, CompositionError> {
    let source = make_source(&[("loop", json!({"until": "a", "max": "{{ cap }}"}))]);
    let mut config = resolve_loop_config(&source).unwrap().unwrap();
    let lookup = ValuesLookup(values.as_object().cloned().unwrap());
    resolve_templated_loop_max(&mut config, &lookup, &LoopLocation::document_root(&source.resolved_path))
        .map(|()| config)
}

#[test]
fn templated_max_resolves_once_to_a_cap() {
    for values in [json!({"cap": 4}), json!({"cap": "4"}), json!({"cap": " 4 "})] {
        let config = resolve_cap(values.clone()).unwrap();
        assert_eq!(config.max_iterations, Some(4), "{values}");
        assert_eq!(config.max_template, None);
    }
}

#[test]
fn templated_max_rejects_values_that_are_not_a_cap() {
    for (values, expected) in [
        (json!({}), "resolved to null"),
        (json!({"cap": null}), "resolved to null"),
        (json!({"cap": ""}), "empty value"),
        (json!({"cap": 0}), "greater than zero"),
        (json!({"cap": "0"}), "greater than zero"),
        (json!({"cap": -1}), "expected a positive integer"),
        (json!({"cap": 1.5}), "expected a positive integer"),
        (json!({"cap": "four"}), "expected a positive integer"),
        (json!({"cap": 1e30}), "expected a positive integer"),
        (json!({"cap": "99999999999999999999999"}), "expected a positive integer"),
        (json!({"cap": [4]}), "expected a positive integer"),
    ] {
        let err = resolve_cap(values.clone()).unwrap_err();
        let (_, property, message) = invalid_parts(&err);
        assert_eq!(property, "loop.max", "{values}");
        assert!(
            message.contains("`loop.max` template `{{ cap }}`") && message.contains(expected),
            "{values}: {message}"
        );
    }
}

#[test]
fn a_configuration_without_a_template_is_left_alone() {
    let source = make_source(&[("loop", json!({"until": "a", "max": 3}))]);
    let mut config = resolve_loop_config(&source).unwrap().unwrap();
    let lookup = ValuesLookup(serde_json::Map::new());
    resolve_templated_loop_max(&mut config, &lookup, &LoopLocation::document_root(&source.resolved_path))
        .unwrap();
    assert_eq!(config.max_iterations, Some(3));
}

fn container_location() -> LoopLocation {
    LoopLocation {
        source_path: std::path::PathBuf::from("steps.yaml"),
        property: "tasks[2].loop".to_string(),
    }
}

#[test]
fn container_loops_reject_on_rate_limit() {
    for owner in [LoopOwner::Group, LoopOwner::Sequence] {
        let block = json!({"until": "a", "on_rate_limit": "pause"});
        let err = parse_loop_block(&block, owner, &container_location()).unwrap_err();
        let CompositionError::LoopRateLimitPolicyNotAccepted { source_path, property, owner: named } = &err else {
            panic!("expected LoopRateLimitPolicyNotAccepted, got {err:?}");
        };
        assert_eq!(source_path, std::path::Path::new("steps.yaml"));
        assert_eq!(property, "tasks[2].loop.on_rate_limit");
        assert_eq!(*named, owner);
        assert!(err.to_string().contains(&format!("on a {owner} loop")), "got: {err}");
    }
}

#[test]
fn document_loops_accept_on_rate_limit() {
    let block = json!({"until": "a", "on_rate_limit": "abort"});
    let parsed = parse_loop_block(&block, LoopOwner::Document, &LoopLocation::document_root("doc.md")).unwrap();
    assert_eq!(parsed.config.on_rate_limit, Some(OnRateLimit::Abort));
}

#[test]
fn container_loops_parse_the_document_grammar() {
    let block = json!({
        "until": "n >= 3",
        "action": ["increment(n)"],
        "max": "{{ cap }}",
        "stderr": "pass done",
    });
    let document = parse_loop_block(&block, LoopOwner::Document, &LoopLocation::document_root("doc.md")).unwrap();
    for owner in [LoopOwner::Group, LoopOwner::Sequence] {
        let container = parse_loop_block(&block, owner, &container_location()).unwrap();
        assert_eq!(container.config, document.config, "{owner}");
        assert_eq!(container.gate.len(), 1, "{owner}: the loose verb is the gate");
    }
}

#[test]
fn container_errors_are_rooted_at_the_block() {
    let location = container_location();
    let cases = [
        (json!({"untill": "a"}), "tasks[2].loop.untill"),
        (json!({"until": "a", "max": 0}), "tasks[2].loop.max"),
        (json!({"until": null}), "tasks[2].loop.until"),
        (json!({}), "tasks[2].loop"),
    ];
    for (block, expected) in cases {
        let err = parse_loop_block(&block, LoopOwner::Sequence, &location).unwrap_err();
        let (path, property, message) = invalid_parts(&err);
        assert_eq!(path, std::path::Path::new("steps.yaml"));
        assert_eq!(property, expected, "{block}");
        assert!(message.contains(expected), "{block}: {message}");
    }
}

#[test]
fn container_typo_never_suggests_on_rate_limit() {
    let block = json!({"until": "a", "on-rate-limit": "pause"});
    let err = parse_loop_block(&block, LoopOwner::Group, &container_location()).unwrap_err();
    let (_, _, message) = invalid_parts(&err);
    assert!(!message.contains("on_rate_limit"), "got: {message}");
}

#[test]
fn container_fail_fast_is_a_removed_key() {
    let block = json!({"until": "a", "fail_fast": false});
    let err = parse_loop_block(&block, LoopOwner::Group, &container_location()).unwrap_err();
    assert!(
        matches!(&err, CompositionError::RemovedGrammar { path, .. } if path == "tasks[2].loop.fail_fast"),
        "got {err:?}"
    );
}

#[test]
fn container_gate_errors_name_the_block() {
    let location = container_location();
    let mixed = json!({"until": "a", "gate": [], "stderr": "hi"});
    let err = parse_loop_block(&mixed, LoopOwner::Group, &location).unwrap_err();
    assert!(err.to_string().contains("tasks[2].loop.stderr"), "got: {err}");
    let when = json!({"until": "a", "when": "true"});
    let err = parse_loop_block(&when, LoopOwner::Sequence, &location).unwrap_err();
    assert!(err.to_string().contains("tasks[2].loop.when") || format!("{err:?}").contains("tasks[2].loop.when"), "got: {err:?}");
}

#[test]
fn a_cap_template_that_fails_to_evaluate_keeps_its_typed_cause() {
    let source = make_source(&[("loop", json!({"until": "a", "max": "{{ nope() }}"}))]);
    let mut config = resolve_loop_config(&source).unwrap().unwrap();
    let lookup = ValuesLookup(serde_json::Map::new());
    let err = resolve_templated_loop_max(&mut config, &lookup, &LoopLocation::document_root(&source.resolved_path))
        .unwrap_err();
    let CompositionError::LoopMaxTemplateInvalid { property, template, source: cause, .. } = &err else {
        panic!("expected LoopMaxTemplateInvalid, got {err:?}");
    };
    assert_eq!(property, "loop.max");
    assert_eq!(template, "{{ nope() }}");
    assert_eq!(cause.stage(), "evaluate");
    assert!(err.to_string().contains("loop.md"), "got: {err}");
}
