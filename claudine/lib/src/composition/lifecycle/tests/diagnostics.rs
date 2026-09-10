//! diagnostics lifecycle tests.

use super::*;

#[test]
fn lifecycle_invalid_error_renders_as_block_error() {
    use biscuit_terminal::errors::BlockError;
    use biscuit_terminal::utils::escape_codes::strip_escape_codes;

    let frontmatter = json!({
        "success": {
            "speak": "hello"
        }
    });

    let err =
        parse_lifecycle_config(&frontmatter, Path::new("prompts/sentrux.md")).unwrap_err();
    let CompositionError::LifecycleInvalid {
        property,
        unknown_field,
        expected_fields,
        source_file,
        ..
    } = &err
    else {
        panic!("expected LifecycleInvalid, got {err:?}");
    };

    assert_eq!(property, "success");
    assert_eq!(unknown_field.as_deref(), Some("speak"));
    assert_eq!(source_file, Path::new("prompts/sentrux.md"));
    assert!(expected_fields.contains(&"say".to_string()));
    assert!(expected_fields.contains(&"say_first".to_string()));
    assert!(expected_fields.contains(&"effect".to_string()));

    let rendered = strip_escape_codes(err.report_block_error_optimistic(Some(80)));
    assert!(
        rendered.contains("success.speak"),
        "dotted property should appear: {rendered}"
    );
    assert!(
        rendered.contains("sentrux.md"),
        "file name should appear: {rendered}"
    );
    assert!(
        rendered.contains("say"),
        "expected fields should list 'say': {rendered}"
    );
}

#[test]
fn parse_serde_unknown_field_extracts_field_and_expected() {
    let frontmatter = json!({
        "failure": {
            "bogus_field": true
        }
    });

    let err = parse_lifecycle_config(&frontmatter, dummy_path()).unwrap_err();
    let CompositionError::LifecycleInvalid {
        property,
        unknown_field,
        expected_fields,
        ..
    } = &err
    else {
        panic!("expected LifecycleInvalid, got {err:?}");
    };

    assert_eq!(property, "failure");
    assert_eq!(unknown_field.as_deref(), Some("bogus_field"));
    assert!(!expected_fields.is_empty());
    assert!(expected_fields.contains(&"say".to_string()));
}

#[test]
fn stack_as_map_reports_sequence_mismatch_not_unknown_property() {
    use biscuit_terminal::errors::BlockError;
    use biscuit_terminal::utils::escape_codes::strip_escape_codes;

    // `stack:` authored as a map (its items missing the leading `-`)
    // rather than a YAML list. This is a type mismatch, NOT an
    // unknown-field error, so no field name / "Expected one of" catalog
    // must be fabricated.
    let frontmatter = json!({
        "initialize": {
            "stack": {
                "when": "phase >= total_phases",
                "action": [{ "warn": "too big" }]
            }
        }
    });

    let err = parse_lifecycle_config(&frontmatter, dummy_path()).unwrap_err();
    let CompositionError::LifecycleInvalid {
        property,
        message,
        unknown_field,
        expected_fields,
        ..
    } = &err
    else {
        panic!("expected LifecycleInvalid, got {err:?}");
    };

    assert_eq!(property, "initialize");
    assert!(unknown_field.is_none());
    assert!(expected_fields.is_empty());
    assert!(
        message.contains("expected a sequence"),
        "raw serde message should be preserved: {message}"
    );

    let rendered = strip_escape_codes(err.report_block_error_optimistic(Some(80)));
    assert!(
        !rendered.contains("Unknown property"),
        "must not fabricate an unknown-property diagnostic: {rendered}"
    );
    assert!(
        !rendered.contains("Expected one of"),
        "must not fabricate a field catalog: {rendered}"
    );
    assert!(
        rendered.contains("stack"),
        "hint should point at the `stack` list shape: {rendered}"
    );
}

// =====================================================================
// Phase 2: extended event inventory, lifecycle concerns, stacks
// =====================================================================

#[test]
fn frontmatter_excerpt_included_for_placement_error() {
    // The `WithFrontmatter` wrapper is applied at the render boundary
    // (CLI handlers), not at the parse site. Here we only verify that the
    // underlying placement error carries the property name needed for
    // frontmatter highlighting.
    let fm = json!({
        "start": {"stack": [{"action": "skip"}]}
    });
    let err = parse_lifecycle_config(&fm, dummy_path()).unwrap_err();
    match err {
        CompositionError::LifecycleActionPlacement {
            property, event, ..
        } => {
            // The stack item is at index 0, so the annotated property
            // path is `start.stack[0]`. Frontmatter highlighting falls
            // back to the top-level `start` key when no per-stack-item
            // line is found.
            assert!(property.starts_with("start"), "got: {property}");
            assert_eq!(event, "start");
        }
        other => panic!("expected placement error, got: {other:?}"),
    }
}

// =====================================================================
// Phase 3: lifecycle context, static scans, shell-audit collection
// =====================================================================

// -- err static scan ---------------------------------------------------

#[test]
fn err_in_start_stack_when_clause_is_rejected() {
    // `err` is forbidden in `start` (a no-error event) — even inside a
    // `when:` condition.
    let fm = json!({
        "start": {
            "stack": [
                {"when": "err != null", "action": {"say": "has error"}}
            ]
        }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    let err = validate_no_err_in_no_error_events(&config, dummy_path()).unwrap_err();
    match err {
        CompositionError::LifecycleErrNotAvailable { event, property, .. } => {
            assert_eq!(event, "start");
            assert!(property.contains("when"), "got: {property}");
        }
        other => panic!("expected LifecycleErrNotAvailable, got: {other:?}"),
    }
}

#[test]
fn err_member_access_in_single_text_arg_is_literal() {
    // A positional scalar value is literal text by default — `err.msg` is
    // the text, not the `err` global. There is nothing to reject. To
    // reference the error in an error-carrying event, interpolate instead:
    // `{ say: "{{err.msg}}" }`.
    let fm = json!({
        "start": {
            "stack": [{"action": {"say": "err.msg"}}]
        }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    assert!(validate_no_err_in_no_error_events(&config, dummy_path()).is_ok());
}

#[test]
fn err_in_single_text_arg_is_literal_across_no_error_events() {
    // A positional scalar value is literal text in every no-error event —
    // the err-availability guard only governs expression surfaces (e.g.
    // `when:` clauses), not literal message bodies.
    for ev in ["initialize", "success"] {
        let fm = json!({
            ev: {"stack": [{"action": {"say": "err"}}]}
        });
        let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
        assert!(
            validate_no_err_in_no_error_events(&config, dummy_path()).is_ok(),
            "bare `err` in a {ev} message arg should be literal, not rejected"
        );
    }
    // Loop concerns live under `loop:`.
    let fm = json!({
        "loop": {
            "while": "true",
            "stack": [{"action": {"say": "err"}}]
        }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    assert!(validate_no_err_in_no_error_events(&config, dummy_path()).is_ok());
}

#[test]
fn err_in_blocked_failure_finalize_is_allowed() {
    // `err` is permitted in error-carrying events.
    for event in ["blocked", "failure", "finalize"] {
        let fm = json!({
            event: {"stack": [{"action": {"say": "err.msg"}}]}
        });
        let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
        let result = validate_no_err_in_no_error_events(&config, dummy_path());
        assert!(
            result.is_ok(),
            "err should be allowed in {event}, got: {:?}",
            result.err()
        );
    }
}

#[test]
fn doc_err_escape_hatch_is_allowed_everywhere() {
    // `doc.err` reaches a literal frontmatter property, not the lifecycle
    // global, so it is permitted even in no-error events.
    for event in ["initialize", "start", "success", "loop"] {
        let fm = if event == "loop" {
            json!({
                "loop": {
                    "while": "true",
                    "stack": [{"action": {"say": "doc.err"}}]
                }
            })
        } else {
            json!({
                event: {"stack": [{"action": {"say": "doc.err"}}]}
            })
        };
        let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
        let result = validate_no_err_in_no_error_events(&config, dummy_path());
        assert!(
            result.is_ok(),
            "doc.err should be allowed in {event}, got: {:?}",
            result.err()
        );
    }
}

#[test]
fn err_in_control_reason_single_text_arg_is_literal() {
    // `error` with a positional scalar value takes its reason literally, so
    // `err.msg` is text, not a reference to the `err` global and is not
    // rejected.
    let fm = json!({
        "start": {
            "stack": [{"action": {"error": "err.msg"}}]
        }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    assert!(validate_no_err_in_no_error_events(&config, dummy_path()).is_ok());
}

#[test]
fn err_in_shell_command_single_text_arg_is_literal() {
    // `shell` with a positional scalar value takes its command literally, so
    // `err.msg` is text, not an `err`-global reference.
    let fm = json!({
        "loop": {
            "while": "true",
            "stack": [{"action": {"shell": "err.msg"}}]
        }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    assert!(validate_no_err_in_no_error_events(&config, dummy_path()).is_ok());
}

// -- err static scan over interpolation spans (C4) --------------------

#[test]
fn err_interpolation_span_in_top_level_field_rejected_in_no_error_event() {
    // Late binding (C4): a top-level field reaches `err` only through a
    // `{{ … }}` span, and `err` is still forbidden in a no-error event.
    let fm = json!({ "start": { "message": "❌️  {{err.msg}}" } });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    let err = validate_no_err_in_no_error_events(&config, dummy_path()).unwrap_err();
    match err {
        CompositionError::LifecycleErrNotAvailable { event, property, .. } => {
            assert_eq!(event, "start");
            assert_eq!(property, "start.message");
        }
        other => panic!("expected LifecycleErrNotAvailable, got: {other:?}"),
    }
}

#[test]
fn err_interpolation_span_in_stack_message_rejected_in_no_error_event() {
    // A positional scalar message body is literal text, but its `{{ … }}`
    // span still reaches the `err` global and must be rejected in `start`.
    let fm = json!({
        "start": { "stack": [{"action": {"message": "❌️  {{err.msg}}"}}] }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    let err = validate_no_err_in_no_error_events(&config, dummy_path()).unwrap_err();
    match err {
        CompositionError::LifecycleErrNotAvailable { event, property, .. } => {
            assert_eq!(event, "start");
            assert!(property.starts_with("start.stack"), "got: {property}");
        }
        other => panic!("expected LifecycleErrNotAvailable, got: {other:?}"),
    }
}

#[test]
fn timing_and_current_interpolation_allowed_in_no_error_events() {
    // `timing`/`current` are allowed everywhere, including no-error events.
    let fm = json!({
        "start": { "message": "took {{timing.document_ms}}ms on {{current.ctx.agent}}" }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    assert!(validate_no_err_in_no_error_events(&config, dummy_path()).is_ok());
}

#[test]
fn err_interpolation_span_allowed_in_error_carrying_event() {
    // The same `{{err.msg}}` span is fine in `failure` (an error event).
    let fm = json!({ "failure": { "message": "❌️  {{err.msg}}" } });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    assert!(validate_no_err_in_no_error_events(&config, dummy_path()).is_ok());
}

// -- deferred effect validation (C4) ----------------------------------

#[test]
fn effect_field_with_interpolation_skips_prepare_validation() {
    // An `effect: "{{name}}"` cannot be checked against the catalog at parse
    // time, so it parses cleanly and is validated at event-time instead.
    let fm = json!({ "success": { "effect": "{{effect_name}}" } });
    assert!(parse_lifecycle_config(&fm, dummy_path()).is_ok());
}

#[test]
fn effect_field_literal_unknown_name_still_rejected_at_prepare() {
    // A literal (interpolation-free) unknown effect name is still rejected
    // at parse time.
    let fm = json!({ "success": { "effect": "nonexistent-effect-xyz" } });
    let err = parse_lifecycle_config(&fm, dummy_path()).unwrap_err();
    assert!(matches!(
        err,
        CompositionError::LifecycleUnknownEffect(_, _)
    ));
}

// -- stack leak scan ---------------------------------------------------

#[test]
fn stack_string_literal_with_interpolation_span_is_leak() {
    // A string literal inside a parsed expression that contains a
    // surviving `{{ … }}` span is a leak — the literal is passed through
    // verbatim to the evaluated result.
    let fm = json!({
        "start": {
            "stack": [{"action": {"say": "leaked {{ broken( }}"}}]
        }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    let err = validate_no_interpolation_leaks(&config, dummy_path(), &[]).unwrap_err();
    match err {
        CompositionError::LifecycleInterpolationLeak { property, .. } => {
            assert!(
                property.starts_with("start.stack"),
                "expected stack property, got: {property}"
            );
        }
        other => panic!("expected LifecycleInterpolationLeak, got: {other:?}"),
    }
}

#[test]
fn top_level_info_field_leak_is_caught() {
    // The `info` field is now covered by the leak scan (Phase 2 added
    // the field; Phase 3 extends the scan to cover it).
    let config = LifecycleConfig {
        start: Some(LifecycleNotification {
            info: Some("leaked {{ broken( }}".to_string()),
            ..Default::default()
        }),
        ..Default::default()
    };
    let err = validate_no_interpolation_leaks(&config, dummy_path(), &[]).unwrap_err();
    match err {
        CompositionError::LifecycleInterpolationLeak { property, .. } => {
            assert_eq!(property, "start.info");
        }
        other => panic!("expected leak, got: {other:?}"),
    }
}

#[test]
fn top_level_warn_field_leak_is_caught() {
    let config = LifecycleConfig {
        start: Some(LifecycleNotification {
            warn: Some("leaked {{ broken( }}".to_string()),
            ..Default::default()
        }),
        ..Default::default()
    };
    let err = validate_no_interpolation_leaks(&config, dummy_path(), &[]).unwrap_err();
    assert!(matches!(
        err,
        CompositionError::LifecycleInterpolationLeak { property, .. } if property == "start.warn"
    ));
}

#[test]
fn initialize_finalize_loop_top_level_leaks_are_caught() {
    // All seven events are now covered.
    for event in ["initialize", "finalize"] {
        let config = LifecycleConfig {
            initialize: if event == "initialize" {
                Some(LifecycleNotification {
                    stderr: Some("leaked {{ broken( }}".to_string()),
                    ..Default::default()
                })
            } else {
                None
            },
            finalize: if event == "finalize" {
                Some(LifecycleNotification {
                    stderr: Some("leaked {{ broken( }}".to_string()),
                    ..Default::default()
                })
            } else {
                None
            },
            ..Default::default()
        };
        let result = validate_no_interpolation_leaks(&config, dummy_path(), &[]);
        match result {
            Err(CompositionError::LifecycleInterpolationLeak { property, .. })
                if property.starts_with(event) => {}
            other => panic!("expected leak for {event}, got: {other:?}"),
        }
    }
}

// -- stack undefined-variable scan -------------------------------------

#[test]
fn stack_undefined_variable_in_when_clause_is_rejected() {
    let fm = json!({
        "start": {
            "stack": [
                {"when": "missing_var == 'x'", "action": {"say": "hi"}}
            ]
        }
    });
    let raw = fm_from_json(fm.clone());
    let effective = json!({});
    let lifecycle = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    let err = validate_no_undefined_lifecycle_variables(&raw, &effective, &lifecycle, dummy_path())
        .unwrap_err();
    match err {
        CompositionError::LifecycleUndefinedVariable { property, variable, .. } => {
            assert!(property.contains("when"), "got: {property}");
            assert_eq!(variable, "missing_var");
        }
        other => panic!("expected undefined variable, got: {other:?}"),
    }
}

#[test]
fn stack_err_global_is_not_undefined_in_failure() {
    // `err` is a lifecycle global in stack expressions, so it must not
    // trip the undefined-variable scan (the err static scan handles
    // misuse).
    let fm = json!({
        "failure": {
            "stack": [{"action": {"say": "err.msg"}}]
        }
    });
    let raw = fm_from_json(fm.clone());
    let effective = json!({});
    let lifecycle = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    let result = validate_no_undefined_lifecycle_variables(&raw, &effective, &lifecycle, dummy_path());
    assert!(result.is_ok(), "err should not be undefined, got: {:?}", result.err());
}

#[test]
fn stack_timing_and_current_globals_are_not_undefined() {
    let fm = json!({
        "start": {
            "stack": [
                {"action": {"say": "timing.document_ms"}},
                {"action": {"say": "current.ctx.agent"}}
            ]
        }
    });
    let raw = fm_from_json(fm.clone());
    let effective = json!({});
    let lifecycle = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    let result = validate_no_undefined_lifecycle_variables(&raw, &effective, &lifecycle, dummy_path());
    assert!(result.is_ok(), "got: {:?}", result.err());
}

#[test]
fn stack_bare_token_in_action_arg_is_literal_not_undefined_variable() {
    // A positional scalar value is literal text by default, so a bare token
    // is not an undefined-variable reference. Real references go through a
    // whole-value `{{ … }}` span.
    let fm = json!({
        "start": {
            "stack": [{"action": {"say": "missing_var"}}]
        }
    });
    let raw = fm_from_json(fm.clone());
    let effective = json!({});
    let lifecycle = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    assert!(
        validate_no_undefined_lifecycle_variables(&raw, &effective, &lifecycle, dummy_path())
            .is_ok(),
        "a bare token in a literal message arg is not a variable reference"
    );
}

// -- lifecycle globals vs body/frontmatter interpolation --------------

#[test]
fn late_binding_global_in_top_level_field_is_a_known_root() {
    // Late binding (C4 / 5.3): `err`/`timing`/`current` are known roots in
    // top-level communication fields just like in stack surfaces — they
    // resolve at event-time, not against frontmatter — so the
    // undefined-variable scan does not flag a bare reference. (Placement
    // misuse — `err` in a no-error event — is caught separately by
    // `validate_no_err_in_no_error_events`.)
    for global in ["err", "timing", "current"] {
        let raw = fm_from_json(json!({
            "failure": { "message": format!("x: {{{{ {global} }}}}") }
        }));
        let effective = json!({});
        let result = validate_no_undefined_lifecycle_variables(
            &raw,
            &effective,
            &LifecycleConfig::default(),
            dummy_path(),
        );
        assert!(result.is_ok(), "`{global}` is a known root; got: {result:?}");
    }
}

#[test]
fn bare_err_in_top_level_field_passes_when_frontmatter_defines_it() {
    // When frontmatter has a literal `err` property, `{{ err }}` in a
    // top-level field resolves to it — the lifecycle global does not
    // interfere.
    let raw = fm_from_json(json!({
        "start": { "message": "error: {{ err }}" }
    }));
    let effective = json!({ "err": "literal-value" });
    let result = validate_no_undefined_lifecycle_variables(
        &raw,
        &effective,
        &LifecycleConfig::default(),
        dummy_path(),
    );
    assert!(result.is_ok(), "got: {:?}", result.err());
}

