//! diagnostics lifecycle tests.

use super::*;

#[test]
fn unknown_verb_in_a_dictionary_renders_as_block_error() {
    use biscuit_terminal::errors::BlockError;
    use biscuit_terminal::utils::escape_codes::strip_escape_codes;

    let frontmatter = json!({
        "success": {
            "sucess": "hello"
        }
    });

    let err =
        parse_lifecycle_config(&frontmatter, Path::new("prompts/sentrux.md")).unwrap_err();
    let CompositionError::LifecycleUnknownVerb {
        source_path,
        property,
        verb,
        rewrite,
    } = &err
    else {
        panic!("expected LifecycleUnknownVerb, got {err:?}");
    };

    assert_eq!(property, "success.sucess");
    assert_eq!(verb, "sucess");
    assert!(rewrite.contains("`success`"), "{rewrite}");
    assert_eq!(source_path, Path::new("prompts/sentrux.md"));

    let rendered = strip_escape_codes(err.report_block_error_optimistic(Some(80)));
    assert!(
        rendered.contains("success.sucess"),
        "the authored path should appear: {rendered}"
    );
    assert!(
        rendered.contains("sentrux.md"),
        "file name should appear: {rendered}"
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
    let fm = json!({ "start": ["skip"] });
    let err = parse_lifecycle_config(&fm, dummy_path()).unwrap_err();
    match err {
        CompositionError::LifecycleActionPlacement {
            property, event, ..
        } => {
            assert_eq!(property, "start[0].skip");
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
    let fm = json!({ "start": [{ "when": "err != null", "then": [{ "say": "has error" }] }] });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    let err = validate_no_err_in_no_error_events(&fm, &config, dummy_path()).unwrap_err();
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
    let fm = json!({ "start": [{ "say": "err.msg" }] });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    assert!(validate_no_err_in_no_error_events(&fm, &config, dummy_path()).is_ok());
}

#[test]
fn err_in_single_text_arg_is_literal_across_no_error_events() {
    // A positional scalar value is literal text in every no-error event —
    // the err-availability guard only governs expression surfaces (e.g.
    // `when:` clauses), not literal message bodies.
    for ev in ["initialize", "success"] {
        let fm = json!({ ev: [{"say": "err"}] });
        let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
        assert!(
            validate_no_err_in_no_error_events(&fm, &config, dummy_path()).is_ok(),
            "bare `err` in a {ev} message arg should be literal, not rejected"
        );
    }
    // Loop concerns live under `loop:`.
    let fm = json!({
        "loop": {
            "while": "true",
            "gate": [{"say": "err"}]
        }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    assert!(validate_no_err_in_no_error_events(&fm, &config, dummy_path()).is_ok());
}

#[test]
fn err_in_blocked_failure_finalize_is_allowed() {
    // `err` is permitted in error-carrying events.
    for event in ["blocked", "failure", "finalize"] {
        let fm = json!({ event: [{"say": "err.msg"}] });
        let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
        let result = validate_no_err_in_no_error_events(&fm, &config, dummy_path());
        assert!(
            result.is_ok(),
            "err should be allowed in {event}, got: {:?}",
            result.err()
        );
    }
}

#[test]
fn fm_err_escape_hatch_is_allowed_everywhere() {
    // `fm.err` reaches a literal frontmatter property, not the lifecycle
    // global, so it is permitted even in no-error events.
    for event in ["initialize", "start", "success", "loop"] {
        let fm = if event == "loop" {
            json!({
                "loop": {
                    "while": "true",
                    "gate": [{"say": "fm.err"}]
                }
            })
        } else {
            json!({ event: [{"say": "fm.err"}] })
        };
        let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
        let result = validate_no_err_in_no_error_events(&fm, &config, dummy_path());
        assert!(
            result.is_ok(),
            "fm.err should be allowed in {event}, got: {:?}",
            result.err()
        );
    }
}

#[test]
fn err_in_control_reason_single_text_arg_is_literal() {
    // `error` with a positional scalar value takes its reason literally, so
    // `err.msg` is text, not a reference to the `err` global and is not
    // rejected.
    let fm = json!({ "start": [{ "error": "err.msg" }] });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    assert!(validate_no_err_in_no_error_events(&fm, &config, dummy_path()).is_ok());
}

#[test]
fn err_in_shell_command_single_text_arg_is_literal() {
    // `shell` with a positional scalar value takes its command literally, so
    // `err.msg` is text, not an `err`-global reference.
    let fm = json!({
        "loop": {
            "while": "true",
            "gate": [{"shell": "err.msg"}]
        }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    assert!(validate_no_err_in_no_error_events(&fm, &config, dummy_path()).is_ok());
}

// -- err static scan over interpolation spans (C4) --------------------

#[test]
fn err_interpolation_span_in_root_dictionary_rejected_in_no_error_event() {
    // Late binding (C4): a root-dictionary verb reaches `err` only through a
    // `{{ … }}` span, and `err` is still forbidden in a no-error event.
    let fm = json!({ "start": { "message": "❌️  {{err.msg}}" } });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    let err = validate_no_err_in_no_error_events(&fm, &config, dummy_path()).unwrap_err();
    match err {
        CompositionError::LifecycleErrNotAvailable { event, property, .. } => {
            assert_eq!(event, "start");
            assert_eq!(property, "start.message.text");
        }
        other => panic!("expected LifecycleErrNotAvailable, got: {other:?}"),
    }
}

#[test]
fn err_interpolation_span_in_stack_message_rejected_in_no_error_event() {
    // A positional scalar message body is literal text, but its `{{ … }}`
    // span still reaches the `err` global and must be rejected in `start`.
    let fm = json!({ "start": [{ "message": "❌️  {{err.msg}}" }] });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    let err = validate_no_err_in_no_error_events(&fm, &config, dummy_path()).unwrap_err();
    match err {
        CompositionError::LifecycleErrNotAvailable { event, property, .. } => {
            assert_eq!(event, "start");
            assert!(property.starts_with("start[0].message"), "got: {property}");
        }
        other => panic!("expected LifecycleErrNotAvailable, got: {other:?}"),
    }
}

#[test]
fn timing_and_current_interpolation_allowed_in_no_error_events() {
    // `timing`/`current` are allowed everywhere, including no-error events.
    let fm = json!({
        "start": { "message": "took {{timing.document_ms}}ms on {{current.agent}}" }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    assert!(validate_no_err_in_no_error_events(&fm, &config, dummy_path()).is_ok());
}

#[test]
fn err_interpolation_span_allowed_in_error_carrying_event() {
    // The same `{{err.msg}}` span is fine in `failure` (an error event).
    let fm = json!({ "failure": { "message": "❌️  {{err.msg}}" } });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    assert!(validate_no_err_in_no_error_events(&fm, &config, dummy_path()).is_ok());
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

