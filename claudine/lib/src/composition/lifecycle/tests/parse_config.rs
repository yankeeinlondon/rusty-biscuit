//! parse config lifecycle tests.

use super::*;

#[test]
fn parses_valid_lifecycle_config() {
    let frontmatter = json!({
        "start": {
            "message": "Starting composition..."
        },
        "success": {
            "say": "All done!",
            "effect": "confirmation"
        }
    });

    let config = parse_lifecycle_config(&frontmatter, dummy_path()).unwrap();
    assert!(config.start.is_some());
    assert_eq!(
        config.start.as_ref().unwrap().message.as_deref(),
        Some("Starting composition...")
    );

    assert!(config.success.is_some());
    let success = config.success.as_ref().unwrap();
    assert_eq!(success.say.as_deref(), Some("All done!"));
    assert_eq!(success.effect.as_deref(), Some("confirmation"));
}

#[test]
fn rejects_both_say_and_say_first() {
    let frontmatter = json!({
        "start": {
            "say": "Starting",
            "say_first": "Also starting"
        }
    });

    let result = parse_lifecycle_config(&frontmatter, dummy_path());
    assert!(matches!(
        result,
        Err(CompositionError::LifecycleSayConflict(_))
    ));
}

#[test]
fn trims_empty_strings_to_none() {
    let frontmatter = json!({
        "start": {
            "message": "   ",
            "say": ""
        }
    });

    let config = parse_lifecycle_config(&frontmatter, dummy_path()).unwrap();
    let start = config.start.as_ref().unwrap();
    assert!(start.message.is_none());
    assert!(start.say.is_none());
}

#[test]
fn rejects_unknown_keys() {
    let frontmatter = json!({
        "start": {
            "message": "Starting",
            "unknown_field": "value"
        }
    });

    let result = parse_lifecycle_config(&frontmatter, dummy_path());
    assert!(result.is_err());
}

#[test]
fn rejects_unknown_effect_name() {
    let frontmatter = json!({
        "start": {
            "effect": "nonexistent-effect"
        }
    });

    let result = parse_lifecycle_config(&frontmatter, dummy_path());
    assert!(matches!(
        result,
        Err(CompositionError::LifecycleUnknownEffect(_, _))
    ));
}

#[test]
fn say_plus_effect_is_valid() {
    let frontmatter = json!({
        "success": {
            "say": "Done!",
            "effect": "confirmation"
        }
    });

    let config = parse_lifecycle_config(&frontmatter, dummy_path()).unwrap();
    let success = config.success.as_ref().unwrap();
    assert_eq!(success.say.as_deref(), Some("Done!"));
    assert_eq!(success.effect.as_deref(), Some("confirmation"));
}

#[test]
fn say_first_plus_effect_is_valid() {
    let frontmatter = json!({
        "success": {
            "say_first": "Starting now",
            "effect": "confirmation"
        }
    });

    let config = parse_lifecycle_config(&frontmatter, dummy_path()).unwrap();
    let success = config.success.as_ref().unwrap();
    assert_eq!(success.say_first.as_deref(), Some("Starting now"));
    assert_eq!(success.effect.as_deref(), Some("confirmation"));
}

#[test]
fn empty_frontmatter_returns_default() {
    let frontmatter = json!({});
    let config = parse_lifecycle_config(&frontmatter, dummy_path()).unwrap();
    assert!(config.is_empty());
}

#[test]
fn non_object_frontmatter_returns_default() {
    let frontmatter = json!("not an object");
    let config = parse_lifecycle_config(&frontmatter, dummy_path()).unwrap();
    assert!(config.is_empty());
}

#[test]
fn null_lifecycle_property_is_skipped() {
    let frontmatter = json!({
        "start": null,
        "success": {
            "message": "Done"
        }
    });

    let config = parse_lifecycle_config(&frontmatter, dummy_path()).unwrap();
    assert!(config.start.is_none());
    assert!(config.success.is_some());
}

#[test]
fn frontmatter_with_non_lifecycle_keys_is_fine() {
    let frontmatter = json!({
        "title": "My Composition",
        "agent": "claude",
        "start": {
            "message": "Starting"
        }
    });

    let config = parse_lifecycle_config(&frontmatter, dummy_path()).unwrap();
    assert!(config.start.is_some());
}

#[test]
fn status_state_mapping() {
    assert_eq!(LifecycleSignal::Start.status_state(), StatusState::Info);
    assert_eq!(
        LifecycleSignal::Success.status_state(),
        StatusState::Success
    );
    assert_eq!(LifecycleSignal::Blocked.status_state(), StatusState::Error);
    assert_eq!(LifecycleSignal::Failure.status_state(), StatusState::Error);
}

#[test]
fn property_names() {
    assert_eq!(LifecycleSignal::Start.property_name(), "start");
    assert_eq!(LifecycleSignal::Success.property_name(), "success");
    assert_eq!(LifecycleSignal::Blocked.property_name(), "blocked");
    assert_eq!(LifecycleSignal::Failure.property_name(), "failure");
}

#[test]
fn lifecycle_config_get() {
    let fm = json!({
        "start": { "stderr": "Starting" },
        "failure": { "stderr": "Failed" }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    assert!(config.get(LifecycleSignal::Start).is_some());
    assert!(config.get(LifecycleSignal::Success).is_none());
    assert!(config.get(LifecycleSignal::Blocked).is_none());
    assert!(config.get(LifecycleSignal::Failure).is_some());
}

#[test]
fn lifecycle_config_is_empty() {
    let empty = LifecycleConfig::default();
    assert!(empty.is_empty());

    let fm = json!({ "start": { "stderr": "Go" } });
    let non_empty = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    assert!(!non_empty.is_empty());
}

#[test]
fn all_seven_signals_have_canonical_property_names() {
    assert_eq!(LifecycleSignal::Initialize.property_name(), "initialize");
    assert_eq!(LifecycleSignal::Start.property_name(), "start");
    assert_eq!(LifecycleSignal::Success.property_name(), "success");
    assert_eq!(LifecycleSignal::Blocked.property_name(), "blocked");
    assert_eq!(LifecycleSignal::Failure.property_name(), "failure");
    assert_eq!(LifecycleSignal::Finalize.property_name(), "finalize");
    assert_eq!(LifecycleSignal::Loop.property_name(), "loop");
}

#[test]
fn signal_all_iterates_in_canonical_order() {
    let names: Vec<&'static str> =
        LifecycleSignal::ALL.iter().map(|s| s.property_name()).collect();
    assert_eq!(
        names,
        vec![
            "initialize",
            "start",
            "success",
            "blocked",
            "failure",
            "finalize",
            "loop",
        ]
    );
}

#[test]
fn signal_can_carry_error_matrix() {
    // No-error events.
    for event in [
        LifecycleSignal::Initialize,
        LifecycleSignal::Start,
        LifecycleSignal::Success,
        LifecycleSignal::Loop,
    ] {
        assert!(
            !event.can_carry_error(),
            "{event:?} should not be able to carry an error"
        );
    }
    // Err-capable events.
    for event in [
        LifecycleSignal::Blocked,
        LifecycleSignal::Failure,
        LifecycleSignal::Finalize,
    ] {
        assert!(
            event.can_carry_error(),
            "{event:?} should be able to carry an error"
        );
    }
}

#[test]
fn parses_initialize_finalize_top_level_events() {
    let fm = json!({
        "initialize": { "stderr": "composing" },
        "finalize": { "stderr": "cleanup" }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    assert_eq!(
        config.initialize.as_ref().unwrap().stderr.as_deref(),
        Some("composing")
    );
    assert_eq!(
        config.finalize.as_ref().unwrap().stderr.as_deref(),
        Some("cleanup")
    );
    assert_eq!(
        config
            .get(LifecycleSignal::Initialize)
            .unwrap()
            .stderr
            .as_deref(),
        Some("composing")
    );
    assert_eq!(
        config
            .get(LifecycleSignal::Finalize)
            .unwrap()
            .stderr
            .as_deref(),
        Some("cleanup")
    );
}

#[test]
fn parses_info_warn_and_success_top_level_fields() {
    let fm = json!({
        "start": { "info": "composing" },
        "failure": { "warn": "watch out" },
        "success": { "success": "all done" }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    assert_eq!(
        config.start.as_ref().unwrap().info.as_deref(),
        Some("composing")
    );
    assert_eq!(
        config.failure.as_ref().unwrap().warn.as_deref(),
        Some("watch out")
    );
    assert_eq!(
        config.success.as_ref().unwrap().success.as_deref(),
        Some("all done")
    );
}

#[test]
fn extracts_loop_lifecycle_concerns() {
    let fm = json!({
        "loop": {
            "while": "phase < total",
            "action": "increment(phase)",
            "say": "iterate",
            "stderr": "looping"
        }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    let concerns = config.loop_concerns.as_ref().expect("loop concerns");
    assert_eq!(concerns.say.as_deref(), Some("iterate"));
    assert_eq!(concerns.stderr.as_deref(), Some("looping"));
    // `while` and `action` are iteration controls, not lifecycle
    // concerns, so they do not appear on the notification.
    assert_eq!(
        config.get(LifecycleSignal::Loop).unwrap().say.as_deref(),
        Some("iterate")
    );
}

#[test]
fn empty_stack_is_normalized_to_none() {
    let fm = json!({
        "start": { "stack": [] }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    assert!(config.stacks.start.is_none());
    assert!(config.stack(LifecycleSignal::Start).is_none());
}

#[test]
fn empty_frontmatter_yields_empty_seven_event_config() {
    let fm = json!({});
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    assert!(config.is_empty());
    for s in LifecycleSignal::ALL {
        assert!(config.get(s).is_none(), "expected {s:?} to be None");
        assert!(config.stack(s).is_none(), "expected stack for {s:?} to be None");
    }
}

#[test]
fn parse_lifecycle_config_handles_non_object_frontmatter() {
    let fm = json!("scalar");
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    assert!(config.is_empty());
}

#[test]
fn null_event_property_is_skipped() {
    let fm = json!({
        "initialize": null,
        "start": { "stderr": "go" }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    assert!(config.initialize.is_none());
    assert!(config.start.is_some());
}

#[test]
fn loop_concerns_stack_uses_loop_signal_for_placement() {
    // `Skip` is the one placement-restricted action (`initialize` only),
    // so it is invalid in the `loop` event.
    let fm = json!({
        "loop": {
            "while": "true",
            "stack": [{"action": "skip"}]
        }
    });
    let err = parse_lifecycle_config(&fm, dummy_path()).unwrap_err();
    match err {
        CompositionError::LifecycleActionPlacement { event, action, .. } => {
            assert_eq!(event, "loop");
            assert_eq!(action, "skip");
        }
        other => panic!("expected LifecycleActionPlacement, got: {other:?}"),
    }
}

#[test]
fn stop_is_valid_in_every_event() {
    for s in LifecycleSignal::ALL {
        let fm = if s == LifecycleSignal::Loop {
            json!({
                "loop": {
                    "while": "true",
                    "stack": [{"action": "stop"}]
                }
            })
        } else {
            json!({
                s.property_name(): {"stack": [{"action": "stop"}]}
            })
        };
        let config = parse_lifecycle_config(&fm, dummy_path());
        assert!(
            config.is_ok(),
            "`stop` should be valid in {s:?}, got: {:?}",
            config.err()
        );
    }
}

