//! Lifecycle and composition bindings resolve `loop.<member>` off the
//! ambient; only a bare `loop` read (or a read the evaluator cannot split
//! into a member, such as bracket access) builds the whole object.

use super::*;

fn session_over<'a>(
    document: &'a Document,
    ambient: &LoopAmbient,
) -> EvaluationSession<'a> {
    let (view, entries) = runtime_bindings_in_loop(
        LifecycleScope::Event(LifecycleSignal::Success),
        LifecycleValues::default(),
        Some(ambient),
    );
    EvaluationSession::associate(view, document, entries).unwrap()
}

/// Every read shape agrees with the whole object: members, nested and indexed
/// paths into the previous result, and names the object lacks.
#[test]
fn lifecycle_loop_member_reads_equal_the_whole_object_projection() {
    let document = Document::shadowing();
    for last_output in [
        json!("text"),
        json!({"status": "ok", "nothing": null, "items": [1, 2], "nested": {"k": "v"}}),
        json!(["a", "b"]),
        Value::Null,
    ] {
        let ambient = LoopAmbient::new(2, false, true, last_output.clone(), 3);
        let session = session_over(&document, &ambient);
        let read = |text: &str| evaluate(&parse(text).unwrap(), &session).unwrap();
        let whole = read("loop");
        assert_eq!(whole, ambient.to_value());
        for (text, expected) in [
            ("loop.count", json!(2)),
            ("loop.is_first", json!(false)),
            ("loop.is_last", json!(true)),
            ("loop.last_exit_code", json!(3)),
            ("loop.last_output", last_output.clone()),
            ("loop.last_output.status", whole["last_output"].get("status").cloned().unwrap_or(Value::Null)),
            ("loop.last_output.nested.k", whole["last_output"].pointer("/nested/k").cloned().unwrap_or(Value::Null)),
            ("loop.last_output.nothing", Value::Null),
            ("loop.count.value", Value::Null),
            ("loop.missing", Value::Null),
            ("loop.Count", Value::Null),
            ("loop['count']", json!(2)),
            ("loop['last_output']", last_output.clone()),
            ("loop.last_output[0]", whole["last_output"].get(0).cloned().unwrap_or(Value::Null)),
        ] {
            assert_eq!(read(text), expected, "`{text}` over {last_output}");
        }
    }
}

#[cfg(feature = "work-counters")]
mod work_counts {
    use super::*;
    use crate::work_counters::{Work, snapshot};

    /// One megabyte of text, the size at which copying the previous result
    /// dominated a scalar read.
    fn large_text() -> String {
        "x".repeat(1024 * 1024)
    }

    fn materializations(read: impl FnOnce()) -> u64 {
        let before = snapshot();
        read();
        snapshot().since(&before).get(Work::LoopAmbientMaterialize)
    }

    fn large_ambient() -> LoopAmbient {
        LoopAmbient::new(3, false, true, large_text(), 7)
    }

    #[test]
    fn lifecycle_loop_binding_construction_builds_no_loop_object() {
        let ambient = large_ambient();
        assert_eq!(
            materializations(|| drop(runtime_bindings_in_loop(
                LifecycleScope::Event(LifecycleSignal::Success),
                LifecycleValues::default(),
                Some(&ambient),
            ))),
            0
        );
    }

    #[test]
    fn composition_loop_global_construction_builds_no_loop_object() {
        let ambient = large_ambient();
        assert_eq!(
            materializations(|| drop(composition_host_globals(
                EnclosingLoop::Iteration(&ambient),
                json!({}),
                None,
            ))),
            0
        );
    }

    #[test]
    fn lifecycle_loop_scalar_reads_build_no_loop_object_beside_a_large_result() {
        let document = Document::shadowing();
        let ambient = large_ambient();
        let session = session_over(&document, &ambient);
        let read = |text: &str| evaluate(&parse(text).unwrap(), &session).unwrap();
        for (text, expected) in [
            ("loop.count", json!(3)),
            ("loop.is_first", json!(false)),
            ("loop.last_exit_code", json!(7)),
            ("loop.missing", Value::Null),
            ("loop.count >= 2 && !loop.is_first", json!(true)),
        ] {
            let mut value = None;
            assert_eq!(materializations(|| value = Some(read(text))), 0, "`{text}`");
            assert_eq!(value, Some(expected), "`{text}`");
        }
        let prepared = prepare_value(
            &json!("pass {{ loop.count }} exit {{ loop.last_exit_code }}"),
            AuthoredMode::InterpolatedValue,
        )
        .unwrap();
        let mut rendered = None;
        assert_eq!(
            materializations(|| rendered = Some(
                darkmatter::markdown::compose::expression::evaluate_prepared(&prepared, &session).unwrap()
            )),
            0
        );
        assert_eq!(rendered, Some(json!("pass 3 exit 7")));

        // The requested large value is copied, but the object is not built.
        let mut output = None;
        assert_eq!(materializations(|| output = Some(read("loop.last_output"))), 0);
        assert_eq!(output, Some(Value::String(large_text())));
    }

    /// A whole-root read builds the object once per session, and later member
    /// reads in that session reuse it.
    #[test]
    fn a_bare_lifecycle_loop_read_builds_the_object_once_per_session() {
        let document = Document::shadowing();
        let ambient = large_ambient();
        let session = session_over(&document, &ambient);
        let read = |text: &str| evaluate(&parse(text).unwrap(), &session).unwrap();

        let mut whole = None;
        assert_eq!(materializations(|| whole = Some(read("loop"))), 1);
        assert_eq!(whole, Some(ambient.to_value()));
        assert_eq!(materializations(|| drop(read("loop['count']"))), 0, "cached for the session");
        assert_eq!(materializations(|| drop(read("loop.count"))), 0);

        let fresh = session_over(&document, &ambient);
        assert_eq!(
            materializations(|| drop(evaluate(&parse("loop['count']").unwrap(), &fresh).unwrap())),
            1,
            "bracket access reads the whole root"
        );
    }
}
