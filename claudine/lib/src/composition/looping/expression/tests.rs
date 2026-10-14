use super::*;
use darkmatter::markdown::compose::expression::EvaluationLookup;
use serde_json::json;

mod resolution_context;

fn map(value: Value) -> Map<String, Value> {
    value.as_object().unwrap().clone()
}

fn ambient() -> LoopAmbient {
    LoopAmbient::new(1, true, false, "", 0)
}

#[test]
fn resolves_the_loop_global_bare_dotted_and_by_static_bracket() {
    let fm = map(json!({}));
    let ambient = LoopAmbient::new(3, false, true, "done", 7);
    let lookup = LoopExpressionLookup::new(&fm, &ambient, crate::test_support::process_context(), std::path::Path::new("prompt.md"));

    assert_eq!(lookup.get("loop.count"), Some(json!(3)));
    assert_eq!(lookup.get("loop.is_first"), Some(json!(false)));
    assert_eq!(lookup.get("loop.is_last"), Some(json!(true)));
    assert_eq!(lookup.get("loop.last_output"), Some(json!("done")));
    assert_eq!(lookup.get("loop.last_exit_code"), Some(json!(7)));
    assert_eq!(
        lookup.get("loop"),
        Some(json!({
            "count": 3,
            "is_first": false,
            "is_last": true,
            "last_output": "done",
            "last_exit_code": 7,
        }))
    );
    for (condition, holds) in [
        ("loop['count'] == 3", true),
        ("loop['last_output'] == 'done'", true),
        ("loop.is_last && !loop.is_first", true),
        ("loop.last_exit_code == 0", false),
    ] {
        assert_eq!(
            evaluate_condition(&LoopCondition::While(condition.into()), &lookup).unwrap(),
            holds,
            "`{condition}`"
        );
    }
}

#[test]
fn the_loop_global_never_falls_back_to_frontmatter_and_fm_loop_is_configuration() {
    // `loop.<member>` reads the global, so a member the global lacks is
    // `null` even where the `loop:` block has a same-named key; the block
    // stays readable as `fm.loop`. Unprefixed user properties are untouched.
    let fm = map(json!({
        "iteration": 99,
        "is_first": false,
        "loop": {"while": "true", "max": 4, "count": "configured"},
    }));
    let ambient = ambient();
    let lookup = LoopExpressionLookup::new(&fm, &ambient, crate::test_support::process_context(), std::path::Path::new("prompt.md"));

    assert_eq!(lookup.get("iteration"), Some(json!(99)));
    assert_eq!(lookup.get("is_first"), Some(json!(false)));
    assert_eq!(lookup.get("loop.count"), Some(json!(1)));
    assert_eq!(lookup.get("loop.max"), None);
    assert_eq!(lookup.get("fm.loop.max"), Some(json!(4)));
    assert_eq!(lookup.get("fm.loop.count"), Some(json!("configured")));
    assert!(
        evaluate_condition(&LoopCondition::While("!loop.max && fm.loop.max == 4".into()), &lookup).unwrap()
    );
}

#[test]
fn a_condition_reading_a_retired_loop_name_is_removed_grammar() {
    let fm = map(json!({"_loop_count": 1, "_loop_notes": "kept"}));
    let ambient = ambient();
    let path = std::path::Path::new("prompt.md");
    let lookup = LoopExpressionLookup::new(&fm, &ambient, crate::test_support::process_context(), path);
    for (condition, kind, removed, replacement) in [
        (LoopCondition::While("_loop_count < 3".into()), "while", "_loop_count", "use `loop.count`"),
        (LoopCondition::Until("fm._loop_is_last".into()), "until", "_loop_is_last", "use `loop.is_last`"),
        (
            LoopCondition::While("fm['_loop_last_exit_code'] == 0".into()),
            "while",
            "_loop_last_exit_code",
            "use `loop.last_exit_code`",
        ),
    ] {
        let error = evaluate_condition(&condition, &lookup).unwrap_err();
        let CompositionError::RemovedGrammar { source_path, path: authored, removed: got, replacement: hint } = &error else {
            panic!("expected RemovedGrammar, got {error:?}");
        };
        assert_eq!(source_path, path);
        assert_eq!(authored, &format!("loop.{kind}"));
        assert_eq!(got, removed);
        assert_eq!(hint, replacement);
    }
    // A literal and an unrelated property are data.
    assert!(
        evaluate_condition(&LoopCondition::While("_loop_notes == 'kept'".into()), &lookup).unwrap()
    );
    assert!(
        evaluate_condition(&LoopCondition::While("'_loop_count' != ''".into()), &lookup).unwrap()
    );
}

#[test]
fn resolves_top_level_and_nested_frontmatter() {
    let fm = map(json!({
        "counter": 4,
        "stage": "review",
        "state": {"inner": {"done": true}}
    }));
    let ambient = ambient();
    let lookup = LoopExpressionLookup::new(&fm, &ambient, crate::test_support::process_context(), std::path::Path::new("prompt.md"));

    assert_eq!(lookup.get("counter"), Some(json!(4)));
    assert_eq!(lookup.get("stage"), Some(json!("review")));
    assert_eq!(lookup.get("state.inner.done"), Some(json!(true)));
}

#[test]
fn resolves_environment_variables() {
    let fm = map(json!({}));
    let ambient = ambient();
    let lookup = LoopExpressionLookup::new(&fm, &ambient, crate::test_support::process_context(), std::path::Path::new("prompt.md"));

    assert_eq!(
        lookup.get("env.PATH").is_some(),
        std::env::var("PATH").is_ok()
    );
}

#[test]
fn evaluates_while_condition() {
    let ambient = ambient();
    let fm = map(json!({"counter": 3}));
    let lookup = LoopExpressionLookup::new(&fm, &ambient, crate::test_support::process_context(), std::path::Path::new("prompt.md"));
    assert!(evaluate_condition(&LoopCondition::While("counter < 5".into()), &lookup).unwrap());

    let fm = map(json!({"counter": 5}));
    let lookup = LoopExpressionLookup::new(&fm, &ambient, crate::test_support::process_context(), std::path::Path::new("prompt.md"));
    assert!(!evaluate_condition(&LoopCondition::While("counter < 5".into()), &lookup).unwrap());
}

#[test]
fn evaluates_until_condition_as_continue_decision() {
    let ambient = ambient();
    let fm = map(json!({"done": false}));
    let lookup = LoopExpressionLookup::new(&fm, &ambient, crate::test_support::process_context(), std::path::Path::new("prompt.md"));
    assert!(evaluate_condition(&LoopCondition::Until("done".into()), &lookup).unwrap());

    let fm = map(json!({"done": true}));
    let lookup = LoopExpressionLookup::new(&fm, &ambient, crate::test_support::process_context(), std::path::Path::new("prompt.md"));
    assert!(!evaluate_condition(&LoopCondition::Until("done".into()), &lookup).unwrap());
}

#[test]
fn evaluates_ambient_and_env_conditions() {
    let fm = map(json!({"counter": 1}));
    let ambient = ambient();
    let lookup = LoopExpressionLookup::new(&fm, &ambient, crate::test_support::process_context(), std::path::Path::new("prompt.md"));

    assert!(
        evaluate_condition(&LoopCondition::While("loop.count == 1".into()), &lookup).unwrap()
    );
    assert!(
        evaluate_condition(
            &LoopCondition::While("loop.is_first == true".into()),
            &lookup
        )
        .unwrap()
    );
    assert_eq!(
        evaluate_condition(&LoopCondition::While("env.PATH != \"\"".into()), &lookup).unwrap(),
        std::env::var("PATH").is_ok_and(|value| !value.is_empty())
    );
}

#[test]
fn resolves_fm_namespace_against_frontmatter() {
    let fm = map(json!({
        "build": "from-frontmatter",
        "config": {"retries": 3},
        "fm": {"child": "literal-fm"},
        "doc": {"child": "ordinary-doc"},
    }));
    let ambient = ambient();
    let lookup = LoopExpressionLookup::new(&fm, &ambient, crate::test_support::process_context(), std::path::Path::new("prompt.md"));

    assert_eq!(lookup.get("fm.build"), Some(json!("from-frontmatter")));
    assert_eq!(lookup.get("fm.config.retries"), Some(json!(3)));
    assert_eq!(lookup.get("fm"), Some(Value::Object(fm.clone())));
    assert_eq!(lookup.get("fm.fm.child"), Some(json!("literal-fm")));
    // Missing fm.* values do not fall back to env/ambient.
    assert_eq!(lookup.get("fm.missing"), None);

    // `doc` is an ordinary frontmatter property.
    assert_eq!(lookup.get("doc.child"), Some(json!("ordinary-doc")));
    assert_eq!(lookup.get("doc"), Some(json!({"child": "ordinary-doc"})));
}

#[test]
fn read_side_functions_resolve_against_the_document_directory() {
    let dir = tempfile::TempDir::new().unwrap();
    std::fs::write(dir.path().join("artifact"), "ready").unwrap();
    let fm = map(json!({}));
    let ambient = ambient();
    let context = crate::test_support::context_at(dir.path());
    let source_path = dir.path().join("prompt.md");

    // `file_exists` resolves against the directory of the document.
    let lookup = LoopExpressionLookup::new(&fm, &ambient, &context, &source_path);
    assert!(lookup.resolution_context().is_some());

    // `until` continues while the expression is falsy. The artifact exists,
    // so `file_exists` is truthy and the loop should stop (continue=false).
    assert!(
        !evaluate_condition(
            &LoopCondition::Until("file_exists('artifact')".into()),
            &lookup
        )
        .unwrap()
    );
    // A missing artifact is falsy, so the `until` loop keeps going.
    assert!(
        evaluate_condition(
            &LoopCondition::Until("file_exists('missing')".into()),
            &lookup
        )
        .unwrap()
    );
}

/// The launch-area fallback (`file_ref_fallback_dir`) is diagnostic-only: a
/// reference authored inside a document resolves against `base_dir`, not the
/// launch area (D2). A file present ONLY under the launch area does not
/// resolve even when the fallback is set, while a file under `base_dir`
/// does — and the fallback is still carried on the resolution context for
/// diagnostics.
#[test]
fn file_ref_fallback_dir_is_diagnostic_only_not_a_resolution_candidate() {
    let prompt_dir = tempfile::TempDir::new().unwrap();
    let launch_dir = tempfile::TempDir::new().unwrap();

    // One artifact under the launch area, one under the document dir.
    std::fs::write(launch_dir.path().join("spec.md"), "# spec\n").unwrap();
    std::fs::write(prompt_dir.path().join("local.md"), "# local\n").unwrap();

    let fm = map(json!({}));
    let ambient = ambient();

    let context = crate::test_support::context_at(prompt_dir.path());
    let source_path = prompt_dir.path().join("prompt.md");
    let lookup = LoopExpressionLookup::new(&fm, &ambient, &context, &source_path)
        .with_file_ref_fallback_dir(Some(launch_dir.path()));

    // A file present only under the launch area does NOT resolve, so the
    // `until` condition keeps going (file_exists is falsy).
    assert!(
        evaluate_condition(
            &LoopCondition::Until("file_exists('spec.md')".into()),
            &lookup
        )
        .unwrap(),
        "a launch-area-only file must not resolve; the fallback is diagnostic-only"
    );

    // A file under base_dir resolves as the source-local candidate, so the `until`
    // condition stops (file_exists is truthy).
    assert!(
        !evaluate_condition(
            &LoopCondition::Until("file_exists('local.md')".into()),
            &lookup
        )
        .unwrap(),
        "a file under base_dir resolves; `until` stops"
    );

    // The fallback is still propagated into the resolution context for
    // diagnostics, even though it is not a resolution candidate.
    let ctx = lookup
        .resolution_context()
        .expect("resolution context is present");
    assert_eq!(
        ctx.file_ref_fallback_dir,
        Some(launch_dir.path().to_path_buf())
    );
}

/// A condition that cannot be parsed reports the `while`/`until` keyword and
/// the authored text, and keeps the Darkmatter `ParseError` recoverable
/// through the chain (spec §L1) rather than flattening it into prose.
#[test]
fn parse_errors_carry_the_typed_parse_cause() {
    let fm = map(json!({}));
    let ambient = ambient();
    let lookup = LoopExpressionLookup::new(&fm, &ambient, crate::test_support::process_context(), std::path::Path::new("prompt.md"));
    let err = evaluate_condition(&LoopCondition::While("counter <".into()), &lookup)
        .expect_err("condition should fail to parse");

    let CompositionError::LoopExpressionInvalid {
        kind,
        condition,
        source,
    } = &err
    else {
        panic!("expected LoopExpressionInvalid, got {err:?}");
    };
    assert_eq!(kind, "while");
    assert_eq!(condition, "counter <");
    assert!(matches!(source, LoopExpressionCause::Parse(_)));
    assert!(err.to_string().contains("failed to parse loop.while"));

    // `LoopExpressionCause` is `#[error(transparent)]`, so it *replaces* the
    // concrete error rather than adding a hop to it: the stage is recovered
    // by downcasting to the cause enum, not by walking one level deeper.
    let cause = std::error::Error::source(&err).expect("the typed cause is on the chain");
    assert!(cause.downcast_ref::<LoopExpressionCause>().is_some());
}

/// The evaluate stage is discriminated from the parse stage by the cause's
/// arm, and both project through the same variant.
#[test]
fn evaluate_errors_carry_the_typed_evaluate_cause() {
    let fm = map(json!({}));
    let ambient = ambient();
    let lookup = LoopExpressionLookup::new(&fm, &ambient, crate::test_support::process_context(), std::path::Path::new("prompt.md"));
    let err = evaluate_condition(&LoopCondition::Until("no_such_function()".into()), &lookup)
        .expect_err("condition should fail to evaluate");

    let CompositionError::LoopExpressionInvalid { kind, source, .. } = &err else {
        panic!("expected LoopExpressionInvalid, got {err:?}");
    };
    assert_eq!(kind, "until");
    assert!(matches!(source, LoopExpressionCause::Evaluate(_)));
    assert!(err.to_string().contains("failed to evaluate loop.until"));
}
