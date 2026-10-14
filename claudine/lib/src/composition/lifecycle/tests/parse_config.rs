//! parse config lifecycle tests.

use super::*;
use actions::LifecycleNode;

/// The `(verb, literal text)` of every root-level communication action in the
/// stack of `signal`, in execution order.
fn communications(config: &LifecycleConfig, signal: LifecycleSignal) -> Vec<(&'static str, String)> {
    config
        .stack(signal)
        .iter()
        .filter_map(|node| match node {
            LifecycleNode::Action(LifecycleAction {
                kind: LifecycleActionKind::Communication(comm),
                ..
            }) => match &comm.text {
                Expr::StringLiteral(text) => Some((comm.channel.verb(), text.clone())),
                other => panic!("expected a literal payload, got {other:?}"),
            },
            _ => None,
        })
        .collect()
}

fn pairs(expected: &[(&'static str, &str)]) -> Vec<(&'static str, String)> {
    expected
        .iter()
        .map(|(verb, text)| (*verb, text.to_string()))
        .collect()
}

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
    assert_eq!(
        communications(&config, LifecycleSignal::Start),
        pairs(&[("message", "Starting composition...")])
    );
    assert_eq!(
        communications(&config, LifecycleSignal::Success),
        pairs(&[("effect", "confirmation"), ("say", "All done!")])
    );
}

#[test]
fn rejects_say_and_speak_in_one_dictionary() {
    let frontmatter = json!({
        "start": {
            "say": "Starting",
            "speak": "Starting now"
        }
    });

    let result = parse_lifecycle_config(&frontmatter, dummy_path());
    assert!(matches!(
        result,
        Err(CompositionError::LifecycleSayConflict(_))
    ));
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
fn a_dictionary_plays_its_effect_before_speech() {
    let frontmatter = json!({
        "success": {
            "say": "Done!",
            "effect": "confirmation"
        }
    });

    let config = parse_lifecycle_config(&frontmatter, dummy_path()).unwrap();
    assert_eq!(
        communications(&config, LifecycleSignal::Success),
        pairs(&[("effect", "confirmation"), ("say", "Done!")])
    );
}

#[test]
fn a_list_speaks_before_its_effect_when_authored_that_way() {
    let frontmatter = json!({ "success": [{ "say": "Starting now" }, { "effect": "confirmation" }] });

    let config = parse_lifecycle_config(&frontmatter, dummy_path()).unwrap();
    assert_eq!(
        communications(&config, LifecycleSignal::Success),
        pairs(&[("say", "Starting now"), ("effect", "confirmation")])
    );
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
    assert!(config.stack(LifecycleSignal::Start).is_empty());
    assert!(!config.stack(LifecycleSignal::Success).is_empty());
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
    assert_eq!(
        communications(&config, LifecycleSignal::Start),
        pairs(&[("message", "Starting")])
    );
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
fn lifecycle_config_stack_is_empty_for_absent_events() {
    let fm = json!({
        "start": { "stderr": "Starting" },
        "failure": { "stderr": "Failed" }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    assert!(!config.stack(LifecycleSignal::Start).is_empty());
    assert!(config.stack(LifecycleSignal::Success).is_empty());
    assert!(config.stack(LifecycleSignal::Blocked).is_empty());
    assert!(!config.stack(LifecycleSignal::Failure).is_empty());
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
        communications(&config, LifecycleSignal::Initialize),
        pairs(&[("stderr", "composing")])
    );
    assert_eq!(
        communications(&config, LifecycleSignal::Finalize),
        pairs(&[("stderr", "cleanup")])
    );
}

#[test]
fn parses_info_warn_and_success_verbs() {
    let fm = json!({
        "start": { "info": "composing" },
        "failure": { "warn": "watch out" },
        "success": { "success": "all done" }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    assert_eq!(
        communications(&config, LifecycleSignal::Start),
        pairs(&[("info", "composing")])
    );
    assert_eq!(
        communications(&config, LifecycleSignal::Failure),
        pairs(&[("warn", "watch out")])
    );
    assert_eq!(
        communications(&config, LifecycleSignal::Success),
        pairs(&[("success", "all done")])
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
    // `while` and `action` are iteration controls, not lifecycle concerns,
    // so only the two verbs reach the loop stack, in canonical order.
    assert_eq!(
        communications(&config, LifecycleSignal::Loop),
        pairs(&[("stderr", "looping"), ("say", "iterate")])
    );
}

#[test]
fn empty_stack_is_the_empty_slice() {
    let fm = json!({ "start": [] });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    assert!(config.stacks.start.is_empty());
    assert!(config.stack(LifecycleSignal::Start).is_empty());
}

#[test]
fn empty_frontmatter_yields_empty_seven_event_config() {
    let fm = json!({});
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    assert!(config.is_empty());
    for s in LifecycleSignal::ALL {
        assert!(config.stack(s).is_empty(), "expected stack for {s:?} to be empty");
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
    assert!(config.stack(LifecycleSignal::Initialize).is_empty());
    assert!(!config.stack(LifecycleSignal::Start).is_empty());
}

#[test]
fn loop_gate_uses_loop_signal_for_placement() {
    // `Skip` is the one placement-restricted action (`initialize` only),
    // so it is invalid in the `loop` event.
    let fm = json!({
        "loop": {
            "while": "true",
            "gate": ["skip"]
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
                    "gate": ["stop"]
                }
            })
        } else {
            json!({ s.property_name(): ["stop"] })
        };
        let config = parse_lifecycle_config(&fm, dummy_path());
        assert!(
            config.is_ok(),
            "`stop` should be valid in {s:?}, got: {:?}",
            config.err()
        );
    }
}
