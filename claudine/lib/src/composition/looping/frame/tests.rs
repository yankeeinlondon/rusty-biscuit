//! The frame's pass state and decision order, through its public methods.

use serde_json::json;

use super::*;
use crate::composition::types::{LoopAction, LoopCondition};

fn config(condition: LoopCondition, max: Option<usize>) -> LoopConfig {
    LoopConfig {
        condition,
        actions: vec![LoopAction::Increment("n".into())],
        max_iterations: max,
        max_template: None,
        on_rate_limit: None,
    }
}

fn decide(frame: &LoopFrame, frontmatter: &Map<String, Value>) -> LoopDecision {
    let ambient = frame.ambient();
    let lookup = LoopExpressionLookup::new(
        frontmatter,
        &ambient,
        crate::test_support::process_context(),
        Path::new("loop.md"),
    );
    frame.decide(&lookup).expect("condition evaluates")
}

fn fm(n: i64) -> Map<String, Value> {
    json!({ "n": n }).as_object().unwrap().clone()
}

#[test]
fn entry_values_hold_before_any_pass_for_every_result_kind() {
    for (kind, empty) in [
        (LoopResultKind::Document, json!("")),
        (LoopResultKind::Sequence, json!([])),
        (LoopResultKind::Group, json!({})),
    ] {
        let frame = LoopFrame::new(&config(LoopCondition::Until("true".into()), Some(3)), None, kind, Path::new("x"), "loop").unwrap();
        let ambient = frame.ambient().to_value();
        assert_eq!(ambient["count"], json!(1), "{kind:?}");
        assert_eq!(ambient["is_first"], json!(true), "{kind:?}");
        assert_eq!(ambient["is_last"], json!(false), "{kind:?}");
        assert_eq!(ambient["last_output"], empty, "{kind:?}");
        assert_eq!(ambient["last_exit_code"], json!(0), "{kind:?}");
    }
}

#[test]
fn is_last_follows_the_cap_alone_and_never_predicts_the_condition() {
    // A condition already met would have made the old engine predict the
    // last pass; the frame reports only `count == max`.
    let one = LoopFrame::new(&config(LoopCondition::Until("true".into()), Some(1)), None, LoopResultKind::Document, Path::new("x"), "loop").unwrap();
    assert!(one.ambient().is_last, "max 1: the first pass is the last permitted");
    let mut three = LoopFrame::new(&config(LoopCondition::Until("true".into()), Some(3)), None, LoopResultKind::Document, Path::new("x"), "loop").unwrap();
    assert!(!three.ambient().is_last);
    three.advance();
    assert!(!three.ambient().is_last);
    three.advance();
    assert!(three.ambient().is_last);
}

#[test]
fn cap_precedence_is_override_then_authored_then_default() {
    let authored = config(LoopCondition::While("true".into()), Some(4));
    assert_eq!(LoopFrame::new(&authored, Some(2), LoopResultKind::Group, Path::new("x"), "loop").unwrap().max(), 2);
    assert_eq!(LoopFrame::new(&authored, None, LoopResultKind::Group, Path::new("x"), "loop").unwrap().max(), 4);
    let bare = config(LoopCondition::While("true".into()), None);
    assert_eq!(
        LoopFrame::new(&bare, None, LoopResultKind::Group, Path::new("x"), "loop").unwrap().max(),
        super::super::engine::DEFAULT_MAX_ITERATIONS
    );
}

#[test]
fn an_unresolved_templated_cap_is_refused_at_its_authored_location() {
    let mut templated = config(LoopCondition::While("true".into()), None);
    templated.max_template = Some("{{ cap }}".into());
    let error = LoopFrame::new(&templated, None, LoopResultKind::Sequence, Path::new("seq.md"), "sequence[1].loop")
        .unwrap_err();
    assert!(
        matches!(&error, CompositionError::LoopInvalid { property, .. } if property == "sequence[1].loop.max"),
        "{error:?}"
    );
}

#[test]
fn the_condition_is_read_before_the_cap_is_classified() {
    let mut frame = LoopFrame::new(&config(LoopCondition::Until("n >= 2".into()), Some(2)), None, LoopResultKind::Document, Path::new("x"), "loop").unwrap();
    assert!(matches!(decide(&frame, &fm(1)), LoopDecision::Continue));
    frame.advance();
    // Met on the final permitted pass: success, not exhaustion.
    assert!(matches!(decide(&frame, &fm(2)), LoopDecision::Complete));
    // Still unmet on the final permitted pass: typed exhaustion.
    match decide(&frame, &fm(1)) {
        LoopDecision::Exhausted(CompositionError::LoopLimitExceeded { cap, iteration, .. }) => {
            assert_eq!((cap, iteration), (2, 2));
        }
        other => panic!("expected exhaustion, got {other:?}"),
    }
}

#[test]
fn record_pass_exposes_the_typed_result_to_the_next_read() {
    let mut frame = LoopFrame::new(&config(LoopCondition::Until("true".into()), Some(3)), None, LoopResultKind::Group, Path::new("x"), "loop").unwrap();
    frame.record_pass(json!({ "lint": "ok" }), 0);
    let ambient = frame.ambient().to_value();
    assert_eq!(ambient["last_output"], json!({ "lint": "ok" }));
    assert_eq!(ambient["count"], json!(1), "recording a pass does not advance the count");
}

#[test]
fn condition_and_action_failures_are_located_at_the_block_and_pass() {
    let frame = LoopFrame::new(&config(LoopCondition::Until("n >=".into()), Some(3)), None, LoopResultKind::Sequence, Path::new("seq.md"), "loop").unwrap();
    let ambient = frame.ambient();
    let values = fm(1);
    let lookup = LoopExpressionLookup::new(&values, &ambient, crate::test_support::process_context(), Path::new("seq.md"));
    let error = frame.decide(&lookup).unwrap_err();
    assert!(
        matches!(&error, CompositionError::LoopEvaluationFailed { property, iteration: 1, .. } if property == "loop.until"),
        "{error:?}"
    );

    let not_a_number = json!({ "n": "text" }).as_object().unwrap().clone();
    let error = frame.stage_actions(&not_a_number, &lookup).unwrap_err();
    assert!(
        matches!(&error, CompositionError::LoopEvaluationFailed { property, iteration: 1, source, .. }
            if property == "loop.action" && matches!(**source, CompositionError::InvalidIncrementType { .. })),
        "{error:?}"
    );
}

#[test]
fn a_retried_pass_reads_the_previous_pass_result_again() {
    let mut frame = LoopFrame::new(&config(LoopCondition::Until("true".into()), Some(3)), None, LoopResultKind::Sequence, Path::new("x"), "loop").unwrap();
    frame.record_pass(json!(["a1"]), 0);
    frame.advance();
    // The failed attempt reports, then the pass retries, twice.
    frame.record_pass(json!(["broken"]), 1);
    frame.retry_pass();
    assert_eq!(*frame.ambient().last_output, json!(["a1"]));
    frame.record_pass(json!(["broken again"]), 1);
    frame.retry_pass();
    assert_eq!((Value::clone(&frame.ambient().last_output), frame.ambient().last_exit_code), (json!(["a1"]), 0));
    assert_eq!(frame.count(), 2, "a retry never advances the count");
}

/// Every ambient shares the frame's recorded result rather than copying it,
/// so building the `loop` global for an event or a page is cheap however
/// large the previous result is.
#[test]
fn every_ambient_shares_the_frame_result_without_copying_it() {
    let mut frame = LoopFrame::new(&config(LoopCondition::Until("true".into()), Some(3)), None, LoopResultKind::Document, Path::new("x"), "loop").unwrap();
    frame.record_pass(json!("x".repeat(1024)), 0);
    let first = frame.ambient();
    let second = frame.ambient();
    assert!(std::sync::Arc::ptr_eq(&first.last_output, &second.last_output));
    assert!(std::ptr::eq(frame.last_output(), first.last_output.as_ref()));
    assert!(std::sync::Arc::ptr_eq(&first.last_output, &first.clone().last_output), "a cloned ambient shares it too");
}

#[test]
fn a_break_recorded_through_any_ambient_of_the_frame_reaches_it_and_the_first_wins() {
    let frame = LoopFrame::new(&config(LoopCondition::Until("false".into()), Some(3)), None, LoopResultKind::Sequence, Path::new("x"), "loop").unwrap();
    assert_eq!(frame.pending_break(), None);
    // Each pass builds a new ambient; every one shares the frame's slot.
    frame.ambient().breaks.record(LoopBreak { reason: Some("first".into()), code: 2 });
    frame.ambient().breaks.record(LoopBreak { reason: Some("second".into()), code: 3 });
    assert_eq!(frame.pending_break(), Some(LoopBreak { reason: Some("first".into()), code: 2 }));
    assert_eq!(frame.pending_break().unwrap().summary(), "loop broken: first (code 2)");
    // An isolated copy (a concurrent leg's) does not reach the frame.
    let leg = frame.ambient().with_isolated_breaks();
    assert_eq!(leg, frame.ambient(), "the copy keeps the loop's values");
    frame.discard_break();
    leg.breaks.record(LoopBreak::default());
    assert_eq!(frame.pending_break(), None);
    assert_eq!(LoopBreak::default().summary(), "loop broken (code 0)");
}
