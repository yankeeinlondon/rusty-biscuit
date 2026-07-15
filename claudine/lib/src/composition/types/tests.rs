use super::*;
use serde_json::json;

#[test]
fn sequence_step_overlay_first_step() {
    let overlay = SequenceStepOverlay {
        state: json!("one"),
        previous_state: serde_json::Value::Null,
        next_state: json!("two"),
        is_first: true,
        is_last: false,
        step: 1,
        total_steps: 3,
    };
    assert!(overlay.is_first);
    assert!(!overlay.is_last);
    assert_eq!(overlay.step, 1);
    assert_eq!(overlay.total_steps, 3);
    assert!(overlay.previous_state.is_null());
}

#[test]
fn sequence_step_overlay_last_step() {
    let overlay = SequenceStepOverlay {
        state: json!("three"),
        previous_state: json!("two"),
        next_state: serde_json::Value::Null,
        is_first: false,
        is_last: true,
        step: 3,
        total_steps: 3,
    };
    assert!(!overlay.is_first);
    assert!(overlay.is_last);
    assert_eq!(overlay.step, 3);
    assert!(overlay.next_state.is_null());
}

#[test]
fn sequence_step_overlay_as_overrides_reserves_keys() {
    let overlay = SequenceStepOverlay {
        state: json!("one"),
        previous_state: serde_json::Value::Null,
        next_state: json!("two"),
        is_first: true,
        is_last: false,
        step: 1,
        total_steps: 3,
    };
    let overrides = overlay.as_set_overrides(None);
    let obj = overrides.as_object().unwrap();
    assert_eq!(obj.get("state"), Some(&json!("one")));
    assert_eq!(obj.get("is_first"), Some(&json!(true)));
    assert_eq!(obj.get("is_last"), Some(&json!(false)));
    assert_eq!(obj.get("step"), Some(&json!(1)));
    assert_eq!(obj.get("total_steps"), Some(&json!(3)));
    assert!(obj.get("previous_state").unwrap().is_null());
    assert_eq!(obj.get("next_state"), Some(&json!("two")));
}

#[test]
fn sequence_step_overlay_merges_user_set_but_reserved_wins() {
    let overlay = SequenceStepOverlay {
        state: json!("one"),
        previous_state: serde_json::Value::Null,
        next_state: json!("two"),
        is_first: true,
        is_last: false,
        step: 1,
        total_steps: 3,
    };
    let user_set = json!({
        "color": "red",
        "state": "should-be-overridden",
        "step": 99
    });
    let overrides = overlay.as_set_overrides(Some(user_set));
    let obj = overrides.as_object().unwrap();
    // User key preserved
    assert_eq!(obj.get("color"), Some(&json!("red")));
    // Reserved keys overwritten by overlay
    assert_eq!(obj.get("state"), Some(&json!("one")));
    assert_eq!(obj.get("step"), Some(&json!(1)));
}

#[test]
fn sequence_plan_display_source() {
    let plan = SequencePlan {
        source: SequenceSource::Inline,
        steps: vec![],
        document_fail_fast: true,
    };
    assert!(matches!(plan.source, SequenceSource::Inline));

    let plan2 = SequencePlan {
        source: SequenceSource::External {
            path: std::path::PathBuf::from("data.yaml"),
        },
        steps: vec![],
        document_fail_fast: false,
    };
    assert!(!plan2.document_fail_fast);
}
