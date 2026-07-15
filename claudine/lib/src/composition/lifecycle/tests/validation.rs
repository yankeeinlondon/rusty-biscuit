//! validation lifecycle tests.

use super::*;

#[test]
fn scan_rejects_pre_checks_removed_key() {
    let frontmatter = json!({
        "pre_checks": [{"command": "test"}],
        "start": { "message": "ok" }
    });
    let (key, replacement) = scan_removed_validation_keys(&frontmatter).unwrap();
    assert_eq!(key, "pre_checks");
    assert!(replacement.contains("initialize"), "replacement: {replacement}");
}

#[test]
fn scan_rejects_post_checks_removed_key() {
    let frontmatter = json!({
        "post_checks": [{"command": "test"}],
        "success": { "message": "ok" }
    });
    let (key, replacement) = scan_removed_validation_keys(&frontmatter).unwrap();
    assert_eq!(key, "post_checks");
    assert!(replacement.contains("success"), "replacement: {replacement}");
}

#[test]
fn scan_rejects_handle_removed_key() {
    let frontmatter = json!({
        "handle": "shell('fix')",
        "start": { "message": "ok" }
    });
    let (key, replacement) = scan_removed_validation_keys(&frontmatter).unwrap();
    assert_eq!(key, "handle");
    assert!(replacement.contains("shell"), "replacement: {replacement}");
}

#[test]
fn scan_rejects_deviate_removed_key() {
    let frontmatter = json!({
        "deviate": "shell('fix')",
        "start": { "message": "ok" }
    });
    let (key, replacement) = scan_removed_validation_keys(&frontmatter).unwrap();
    assert_eq!(key, "deviate");
    assert!(replacement.contains("retry"), "replacement: {replacement}");
}

#[test]
fn scan_rejects_handle_timeout_removed_key() {
    let frontmatter = json!({
        "handle_timeout": [{"action": "retry"}],
        "failure": { "message": "ok" }
    });
    let (key, replacement) = scan_removed_validation_keys(&frontmatter).unwrap();
    assert_eq!(key, "handle_timeout");
    assert!(replacement.contains("blocked"), "replacement: {replacement}");
}

#[test]
fn scan_rejects_handle_inline_body_unchanged_removed_key() {
    let frontmatter = json!({
        "handle_inline_body_unchanged": [{"action": "retry"}],
        "failure": { "message": "ok" }
    });
    let (key, replacement) = scan_removed_validation_keys(&frontmatter).unwrap();
    assert_eq!(key, "handle_inline_body_unchanged");
    assert!(replacement.contains("failure"), "replacement: {replacement}");
}

#[test]
fn scan_allows_handle_underscore_without_suffix() {
    // `handle_` with no suffix is not one of the removed keys; only exact
    // `handle` and `handle_<non-empty>` are rejected.
    let frontmatter = json!({
        "handle_": { "message": "ok" }
    });
    assert!(scan_removed_validation_keys(&frontmatter).is_none());
}

#[test]
fn scan_returns_none_for_clean_frontmatter() {
    let frontmatter = json!({
        "start": { "message": "ok" }
    });
    assert!(scan_removed_validation_keys(&frontmatter).is_none());
}

#[test]
fn validation_is_the_dispatch_gate_for_leaked_lifecycle() {
    // The `LifecycleRunGuard` does not re-validate; it dispatches whatever
    // string the config holds. The contract "no side effect dispatches a
    // leaked expression" is upheld by `validate_no_interpolation_leaks`
    // running in the prepare layer, *before* a guard is ever built. This
    // test proves both halves of that boundary against the fake emitter.
    let leaked = parse_lifecycle_config(
        &json!({ "start": { "message": "{{ broken( }}" } }),
        dummy_path(),
    )
    .unwrap();

    // 1. Validation rejects the leaked config — the production choke point.
    let err = validate_no_interpolation_leaks(&leaked, dummy_path(), &[]).unwrap_err();
    assert!(matches!(
        err,
        CompositionError::LifecycleInterpolationLeak { .. }
    ));

    // 2. A guard built from that same config WOULD dispatch the raw span
    //    (the message reaches the emitter verbatim), confirming the guard
    //    itself is not the gate — only the prepare-layer validation is.
    let (settings, messaging, term) = test_ctx();
    let ctx = LifecycleRuntimeContext {
        settings: &settings,
        messaging: &messaging,
        term: &term,
        source_path: Path::new("/tmp/test.md"),
        repo_root: None,
        launch_area: None,
        context: None,
    };
    let emitter = RecordingEmitter::new();
    {
        let mut guard = make_guard(&leaked, &ctx, &emitter);
        guard.emit_start_once();
        guard.defuse();
    }
    assert!(
        emitter.actions().iter().any(|a| matches!(
            a,
            EmittedAction::Message { text } if text.contains("{{ broken(")
        )),
        "guard does not self-gate; validation must run before a guard exists"
    );
}

#[test]
fn undefined_bare_variable_flags_missing_root() {
    let effective = json!({ "area": "claudine" });
    let defined = effective.as_object();
    assert_eq!(undefined_bare_variable("missing", defined), Some("missing"));
    assert_eq!(undefined_bare_variable("area", defined), None);
    // Nested miss under a defined root is treated as defined.
    assert_eq!(undefined_bare_variable("area.sub", defined), None);
    // Runtime namespaces resolve outside the frontmatter.
    assert_eq!(undefined_bare_variable("ctx.area", defined), None);
    assert_eq!(undefined_bare_variable("env.HOME", defined), None);
    assert_eq!(undefined_bare_variable("doc", defined), None);
    assert_eq!(undefined_bare_variable("doc.area", defined), None);
}

#[test]
fn undefined_lifecycle_variable_is_rejected() {
    let raw = fm_from_json(json!({
        "start": { "message": "before {{ missing_lifecycle_var }} after" }
    }));
    let effective = json!({ "start": { "message": "before  after" } });

    let err =
        validate_no_undefined_lifecycle_variables(&raw, &effective, &LifecycleConfig::default(), dummy_path()).unwrap_err();
    match err {
        CompositionError::LifecycleUndefinedVariable {
            property, variable, ..
        } => {
            assert_eq!(property, "start.message");
            assert_eq!(variable, "missing_lifecycle_var");
        }
        other => panic!("expected LifecycleUndefinedVariable, got: {other:?}"),
    }
}

#[test]
fn defined_and_namespaced_lifecycle_variables_pass() {
    let raw = fm_from_json(json!({
        "start": { "message": "{{ area }} on {{ ctx.today }}" },
        "success": { "say": "{{ missing || 'fallback' }}" },
    }));
    let effective = json!({ "area": "claudine" });

    assert!(validate_no_undefined_lifecycle_variables(&raw, &effective, &LifecycleConfig::default(), dummy_path()).is_ok());
}

#[test]
fn undefined_variable_inside_function_call_is_rejected() {
    // The original broken prompt used `parent_dir(review)`: a bare undefined
    // variable as a function argument must fail preparation, not collapse to
    // an empty string the way the whole-span-only guard let it.
    let raw = fm_from_json(json!({
        "start": { "message": "before {{ parent_dir(missing_review) }} after" }
    }));
    let effective = json!({ "area": "claudine" });

    let err =
        validate_no_undefined_lifecycle_variables(&raw, &effective, &LifecycleConfig::default(), dummy_path()).unwrap_err();
    match err {
        CompositionError::LifecycleUndefinedVariable {
            property, variable, ..
        } => {
            assert_eq!(property, "start.message");
            assert_eq!(variable, "missing_review");
        }
        other => panic!("expected LifecycleUndefinedVariable, got: {other:?}"),
    }
}

#[test]
fn undefined_variable_inside_fallback_argument_passes() {
    // Fallback semantics tolerate the undefined operand even when it is
    // wrapped in a function call, so the whole subtree is skipped.
    let raw = fm_from_json(json!({
        "start": { "message": "{{ parent_dir(missing) || 'home' }}" }
    }));
    let effective = json!({ "area": "claudine" });

    assert!(validate_no_undefined_lifecycle_variables(&raw, &effective, &LifecycleConfig::default(), dummy_path()).is_ok());
}

#[test]
fn undefined_variable_in_ternary_condition_is_rejected() {
    let raw = fm_from_json(json!({
        "start": { "message": "{{ missing == 'x' ? 'a' : 'b' }}" }
    }));
    let effective = json!({});

    let err =
        validate_no_undefined_lifecycle_variables(&raw, &effective, &LifecycleConfig::default(), dummy_path()).unwrap_err();
    match err {
        CompositionError::LifecycleUndefinedVariable {
            property, variable, ..
        } => {
            assert_eq!(property, "start.message");
            assert_eq!(variable, "missing");
        }
        other => panic!("expected LifecycleUndefinedVariable, got: {other:?}"),
    }
}

#[test]
fn undefined_variable_in_ternary_truthy_condition_is_rejected() {
    let raw = fm_from_json(json!({
        "start": { "message": "{{ missing ? 'a' : 'b' }}" }
    }));
    let effective = json!({});

    let err =
        validate_no_undefined_lifecycle_variables(&raw, &effective, &LifecycleConfig::default(), dummy_path()).unwrap_err();
    match err {
        CompositionError::LifecycleUndefinedVariable {
            property, variable, ..
        } => {
            assert_eq!(property, "start.message");
            assert_eq!(variable, "missing");
        }
        other => panic!("expected LifecycleUndefinedVariable, got: {other:?}"),
    }
}

#[test]
fn defined_condition_with_undefined_branch_operands_passes() {
    // Ternary branches intentionally tolerate undefined operands; only the
    // condition is checked.
    let raw = fm_from_json(json!({
        "start": { "message": "{{ defined ? missing : also_missing }}" }
    }));
    let effective = json!({ "defined": true });

    assert!(validate_no_undefined_lifecycle_variables(&raw, &effective, &LifecycleConfig::default(), dummy_path()).is_ok());
}

#[test]
fn undefined_variable_in_index_is_rejected() {
    let raw = fm_from_json(json!({
        "start": { "message": "{{ missing[0] }}" }
    }));
    let effective = json!({});

    let err =
        validate_no_undefined_lifecycle_variables(&raw, &effective, &LifecycleConfig::default(), dummy_path()).unwrap_err();
    match err {
        CompositionError::LifecycleUndefinedVariable { variable, .. } => {
            assert_eq!(variable, "missing");
        }
        other => panic!("expected LifecycleUndefinedVariable, got: {other:?}"),
    }
}

#[test]
fn undefined_variable_in_member_access_is_rejected() {
    let raw = fm_from_json(json!({
        "start": { "message": "{{ missing.foo }}" }
    }));
    let effective = json!({});

    let err =
        validate_no_undefined_lifecycle_variables(&raw, &effective, &LifecycleConfig::default(), dummy_path()).unwrap_err();
    match err {
        CompositionError::LifecycleUndefinedVariable { variable, .. } => {
            assert_eq!(variable, "missing");
        }
        other => panic!("expected LifecycleUndefinedVariable, got: {other:?}"),
    }
}

#[test]
fn defined_variable_inside_function_call_passes() {
    let raw = fm_from_json(json!({
        "start": { "message": "{{ parent_dir(area) }}" }
    }));
    let effective = json!({ "area": "/repo/claudine" });

    assert!(validate_no_undefined_lifecycle_variables(&raw, &effective, &LifecycleConfig::default(), dummy_path()).is_ok());
}


