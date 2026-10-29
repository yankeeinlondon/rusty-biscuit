//! Literal hints are classified before any prediction state is built.
//!
//! The uncounted cases pin that the literal fast path predicts exactly what
//! an equivalent expression predicts, at every precedence layer, and that a
//! literal never escapes the runtime-write and late-value analysis. The
//! counted cases (`work-counters`) pin that the literal path builds no
//! provider expression state while the expression path still builds it fresh
//! for each task.

use std::path::Path;

use darkmatter::markdown::FrontmatterMap;
use serde_json::{Map, Value, json};
use tempfile::TempDir;

use super::super::{HintLayer, HintLayers, HintState, LaunchDecision, ProviderPrediction, predict};
use super::{build, prediction, sequence, static_agent, write};

/// The selection a prediction makes: its class and its static hints.
fn selection(prediction: &ProviderPrediction) -> (super::TaskProviderClass, Map<String, Value>) {
    (prediction.class(), prediction.static_hints())
}

/// Which layer decided each key, and at which property.
fn sources(prediction: &ProviderPrediction) -> Vec<(HintLayer, String)> {
    [&prediction.agent, &prediction.model]
        .into_iter()
        .map(|state| match state {
            HintState::Static { source, .. } => (source.layer, source.property.clone()),
            other => panic!("expected a static hint, got {other:?}"),
        })
        .collect()
}

#[test]
fn a_literal_and_an_equivalent_expression_predict_the_same_selection() {
    let dir = TempDir::new().unwrap();
    // The prompt document's own frontmatter.
    write(
        dir.path(),
        "literal.md",
        "---\nagent: codex\nmodel: big\n---\n\nReview.\n",
    );
    write(
        dir.path(),
        "expression.md",
        "---\npick: codex\nsize: big\nagent: \"{{ pick }}\"\nmodel: \"{{ size }}\"\n---\n\nReview.\n",
    );
    let literal = build(
        &sequence(
            dir.path(),
            json!([{ "name": "r", "prompt": "literal.md" }]),
            "",
        ),
        json!({}),
    );
    let literal = prediction(&literal, 0).clone();
    let expression = build(
        &sequence(
            dir.path(),
            json!([{ "name": "r", "prompt": "expression.md" }]),
            "",
        ),
        json!({}),
    );
    let expression = prediction(&expression, 0).clone();
    assert_eq!(selection(&literal), selection(&expression));
    assert_eq!(sources(&literal), sources(&expression));
    assert_eq!(
        sources(&literal),
        [
            (HintLayer::Document, "agent".into()),
            (HintLayer::Document, "model".into())
        ]
    );
    // A literal reads nothing; the expression records what it reads.
    assert!(
        literal.dependencies.is_empty(),
        "{:?}",
        literal.dependencies
    );
    assert!(!expression.dependencies.is_empty());

    // The task's `with`.
    write(dir.path(), "plain.md", "---\ntitle: p\n---\n\nPlain.\n");
    let literal = build(
        &sequence(
            dir.path(),
            json!([{ "name": "r", "prompt": "plain.md", "with": { "agent": "codex", "model": "big" } }]),
            "",
        ),
        json!({}),
    );
    let literal = prediction(&literal, 0).clone();
    let expression = build(
        &sequence(
            dir.path(),
            json!([{ "name": "r", "prompt": "plain.md", "with": { "agent": "{{ pick }}", "model": "{{ size }}" } }]),
            "pick: codex\nsize: big\n",
        ),
        json!({}),
    );
    let expression = prediction(&expression, 0).clone();
    assert_eq!(selection(&literal), selection(&expression));
    assert_eq!(sources(&literal), sources(&expression));
    assert_eq!(
        sources(&literal)[0],
        (HintLayer::With, "tasks[0].with.agent".into())
    );
}

/// Each layer outranks the ones below it whichever of them is a literal and
/// whichever an expression, and a lower layer's expression still reads the
/// value a higher layer supplies.
#[test]
fn precedence_holds_between_literals_and_expressions_at_every_layer() {
    let dir = TempDir::new().unwrap();
    write(
        dir.path(),
        "literal.md",
        "---\nagent: opencode\n---\n\nReview.\n",
    );
    write(
        dir.path(),
        "late.md",
        "---\nagent: \"{{ seq.outputs[0] }}\"\n---\n\nReview.\n",
    );
    write(
        dir.path(),
        "reads.md",
        "---\npick: opencode\nagent: \"{{ pick }}\"\n---\n\nReview.\n",
    );
    let run = |document: &str, with: Value, extra: &str, setters: Value| {
        let path = sequence(
            dir.path(),
            json!([{ "name": "first", "shell": "echo x" }, { "name": "r", "prompt": document, "with": with }]),
            extra,
        );
        prediction(&build(&path, setters), 1).agent.clone()
    };
    let agent = |state: &HintState| static_agent(state).0.clone();
    let layer = |state: &HintState| static_agent(state).1;

    // A `with` literal over a document expression that is late.
    let state = run("late.md", json!({ "agent": "codex" }), "", json!({}));
    assert_eq!(
        (agent(&state), layer(&state)),
        (json!("codex"), HintLayer::With)
    );
    // A `with` expression over a document literal, evaluated or late.
    let state = run(
        "literal.md",
        json!({ "agent": "{{ chosen }}" }),
        "chosen: codex\n",
        json!({}),
    );
    assert_eq!(
        (agent(&state), layer(&state)),
        (json!("codex"), HintLayer::With)
    );
    let state = run(
        "literal.md",
        json!({ "agent": "{{ seq.outputs[0] }}" }),
        "",
        json!({}),
    );
    assert!(
        matches!(
            state,
            HintState::DecidedAtLaunch {
                reason: LaunchDecision::LateValue,
                ..
            }
        ),
        "{state:?}"
    );
    // A caller setter's literal over a `with` expression and a document
    // expression, both late.
    let state = run(
        "late.md",
        json!({ "agent": "{{ seq.outputs[0] }}" }),
        "",
        json!({ "agent": "gemini" }),
    );
    assert_eq!(
        (agent(&state), layer(&state)),
        (json!("gemini"), HintLayer::CallerSetter)
    );
    let state = run("late.md", json!({}), "", json!({ "agent": "gemini" }));
    assert_eq!(
        (agent(&state), layer(&state)),
        (json!("gemini"), HintLayer::CallerSetter)
    );
    // A document expression over the literal a caller setter and a `with`
    // supply for the key it reads: the setter wins that key, so the
    // expression reads its value.
    let state = run(
        "reads.md",
        json!({ "pick": "codex" }),
        "",
        json!({ "pick": "gemini" }),
    );
    assert_eq!(
        (agent(&state), layer(&state)),
        (json!("gemini"), HintLayer::Document)
    );
    let state = run("reads.md", json!({ "pick": "codex" }), "", json!({}));
    assert_eq!(
        (agent(&state), layer(&state)),
        (json!("codex"), HintLayer::Document)
    );
}

/// A text-valued key is not a dependency-free literal when the hint reads it
/// and the graph writes it, or when it is the written hint itself; a literal
/// hint is not deferred by an unrelated write.
#[test]
fn a_runtime_written_text_key_keeps_its_hint_decided_at_launch() {
    let dir = TempDir::new().unwrap();
    write(
        dir.path(),
        "writes.md",
        "---\nsuccess:\n  - set:\n      pick: opencode\n---\n\nFirst.\n",
    );
    write(
        dir.path(),
        "reads.md",
        "---\npick: codex\nagent: \"{{ pick }}\"\n---\n\nSecond.\n",
    );
    write(
        dir.path(),
        "literal.md",
        "---\npick: codex\nagent: claude\nmodel: big\n---\n\nThird.\n",
    );
    let path = sequence(
        dir.path(),
        json!([
            { "name": "first", "prompt": "writes.md" },
            { "name": "second", "prompt": "reads.md" },
            { "name": "third", "prompt": "literal.md" }
        ]),
        "",
    );
    let graph = build(&path, json!({}));
    let second = prediction(&graph, 1);
    assert!(
        matches!(
            second.agent,
            HintState::DecidedAtLaunch {
                reason: LaunchDecision::RuntimeWritten,
                ..
            }
        ),
        "{second:?}"
    );
    // Literal hints beside the written key read nothing the graph writes.
    let third = prediction(&graph, 2);
    assert_eq!(static_agent(&third.agent).0, &json!("claude"));
    assert_eq!(static_agent(&third.model).0, &json!("big"));
    assert!(third.dependencies.is_empty(), "{third:?}");

    // The written key is the literal hint itself.
    write(
        dir.path(),
        "writes.md",
        "---\nsuccess:\n  - set:\n      model: small\n---\n\nFirst.\n",
    );
    let graph = build(&path, json!({}));
    let third = prediction(&graph, 2);
    assert!(
        matches!(
            third.model,
            HintState::DecidedAtLaunch {
                reason: LaunchDecision::RuntimeWritten,
                ..
            }
        ),
        "{third:?}"
    );
}

fn layers<'a>(
    document: &'a FrontmatterMap,
    with: &'a Map<String, Value>,
    setters: &'a Map<String, Value>,
) -> HintLayers<'a> {
    static EMPTY: std::sync::LazyLock<Map<String, Value>> = std::sync::LazyLock::new(Map::new);
    static FAILED: std::sync::LazyLock<
        std::collections::HashMap<String, crate::composition::lifecycle_context::LifecycleCause>,
    > = std::sync::LazyLock::new(Default::default);
    static STEP: std::sync::LazyLock<darkmatter::markdown::compose::SharedFrontmatter> =
        std::sync::LazyLock::new(Default::default);
    HintLayers {
        document,
        document_path: Path::new("/doc.md"),
        with,
        // Only literal `with` values reach these direct cases.
        with_evaluated: with,
        with_deferred: &[],
        with_failed: &FAILED,
        with_origin: Path::new("/seq.md"),
        task_property: "tasks[0]",
        step_frontmatter: &STEP,
        caller_setters: if setters.is_empty() { &EMPTY } else { setters },
        sequence_path: Path::new("/seq.md"),
    }
}

/// A document of `filler` plain keys beside `hints`.
fn document(hints: Value, filler: usize) -> FrontmatterMap {
    let mut document: FrontmatterMap = (0..filler)
        .map(|index| (format!("field_{index}"), Value::String("x".repeat(64))))
        .collect();
    document.extend(hints.as_object().unwrap().clone());
    document
}

/// The evaluator `predict` calls for an evaluable document value: it answers
/// `codex` and counts its calls.
fn evaluator(
    calls: &mut usize,
) -> impl FnMut(
    &Value,
    &Map<String, Value>,
) -> Result<
    Result<Value, crate::composition::lifecycle_context::LifecycleCause>,
    crate::composition::CompositionError,
> + '_ {
    move |_, _| {
        *calls += 1;
        Ok(Ok(json!("codex")))
    }
}

/// Late roots in a document value defer without reaching the evaluator.
#[test]
fn a_late_document_value_is_decided_at_launch_without_evaluation() {
    for late in [
        "{{ seq.outputs[0] }}",
        "{{ loop.count }}",
        "{{ group.name }}",
        "{{ current.branch }}",
    ] {
        let document = document(json!({ "agent": late }), 4);
        let empty = Map::new();
        let mut calls = 0;
        let prediction = predict(
            &layers(&document, &empty, &empty),
            &mut evaluator(&mut calls),
        )
        .unwrap();
        assert!(
            matches!(
                prediction.agent,
                HintState::DecidedAtLaunch {
                    reason: LaunchDecision::LateValue,
                    ..
                }
            ),
            "{late}"
        );
        assert_eq!(calls, 0, "{late}");
    }
}

#[cfg(feature = "work-counters")]
mod counted {
    use super::*;
    use crate::work_counters::{Work, WorkSnapshot, snapshot};

    /// Predict over `layers`, with the prediction work it did.
    fn counted(layers: &HintLayers<'_>, calls: &mut usize) -> (ProviderPrediction, WorkSnapshot) {
        let before = snapshot();
        let prediction = predict(layers, &mut evaluator(calls)).unwrap();
        (prediction, snapshot().since(&before))
    }

    fn assert_no_state(work: &WorkSnapshot, case: &str) {
        assert_eq!(work.get(Work::PredictionStateBuild), 0, "{case}");
        assert_eq!(work.get(Work::PredictionWithStateNodes), 0, "{case}");
    }

    /// A literal at any layer builds and copies no provider expression state,
    /// and never reaches the evaluator.
    #[test]
    fn a_literal_hint_at_any_layer_builds_no_prediction_state() {
        let empty = Map::new();
        let large = document(json!({ "agent": "opencode", "model": "big" }), 200);
        let mut calls = 0;

        let (prediction, work) = counted(&layers(&large, &empty, &empty), &mut calls);
        assert_eq!(
            sources(&prediction),
            [
                (HintLayer::Document, "agent".into()),
                (HintLayer::Document, "model".into())
            ]
        );
        assert_no_state(&work, "document");

        let with = json!({ "agent": "codex", "model": "small" })
            .as_object()
            .cloned()
            .unwrap();
        let (prediction, work) = counted(&layers(&large, &with, &empty), &mut calls);
        assert_eq!(
            sources(&prediction)[0],
            (HintLayer::With, "tasks[0].with.agent".into())
        );
        assert_no_state(&work, "with");

        let setters = json!({ "agent": "gemini", "model": "tiny" })
            .as_object()
            .cloned()
            .unwrap();
        let (prediction, work) = counted(&layers(&large, &with, &setters), &mut calls);
        assert_eq!(
            sources(&prediction)[0],
            (HintLayer::CallerSetter, "agent".into())
        );
        assert_no_state(&work, "caller setter");

        // A literal over an expression in a lower layer reads nothing below it.
        let expression = document(
            json!({ "agent": "{{ pick }}", "model": "{{ size }}", "pick": "x", "size": "y" }),
            200,
        );
        let (_, work) = counted(&layers(&expression, &with, &empty), &mut calls);
        assert_no_state(&work, "with literal over document expression");
        assert_eq!(calls, 0);
    }

    /// A document expression builds its states fresh for each task, once per
    /// task however many keys read one, and predicts what the literal does.
    #[test]
    fn a_document_expression_builds_fresh_state_for_each_task() {
        let empty = Map::new();
        let expression = document(
            json!({ "pick": "codex", "agent": "{{ pick }}", "model": "{{ pick }}" }),
            200,
        );
        let literal = document(json!({ "agent": "codex", "model": "codex" }), 200);
        let mut calls = 0;
        let (from_literal, _) = counted(&layers(&literal, &empty, &empty), &mut calls);
        for task in 0..3 {
            let (from_expression, work) = counted(&layers(&expression, &empty, &empty), &mut calls);
            // The layered state the reads are classified over, and the
            // early-binding state the values are evaluated against.
            assert_eq!(work.get(Work::PredictionStateBuild), 2, "task {task}");
            assert_eq!(
                selection(&from_expression),
                selection(&from_literal),
                "task {task}"
            );
            assert_eq!(
                sources(&from_expression),
                sources(&from_literal),
                "task {task}"
            );
        }
        assert_eq!(calls, 6);
    }

    /// A late or failing document expression still builds what its analysis
    /// needs: the layered state, and for an evaluation the early state too.
    #[test]
    fn late_and_failing_document_expressions_keep_their_analysis() {
        let empty = Map::new();
        let late = document(json!({ "agent": "{{ seq.outputs[0] }}" }), 10);
        let mut calls = 0;
        let (late_prediction, work) = counted(&layers(&late, &empty, &empty), &mut calls);
        assert!(matches!(
            late_prediction.agent,
            HintState::DecidedAtLaunch {
                reason: LaunchDecision::LateValue,
                ..
            }
        ));
        assert_eq!(work.get(Work::PredictionStateBuild), 1);
        assert_eq!(calls, 0);

        // Through the walk, so the real evaluator reports the failure.
        let dir = TempDir::new().unwrap();
        write(
            dir.path(),
            "failing.md",
            "---\nagent: \"{{ no_such_function(1) }}\"\nmodel: big\n---\n\nWork.\n",
        );
        let path = sequence(
            dir.path(),
            json!([{ "name": "r", "prompt": "failing.md" }]),
            "",
        );
        let before = snapshot();
        let graph = build(&path, json!({}));
        let work = snapshot().since(&before);
        let failed = prediction(&graph, 0);
        assert!(
            matches!(&failed.agent, HintState::Failed { source, cause }
                if source.property == "agent" && cause.to_string().contains("no_such_function")),
            "{failed:?}"
        );
        assert!(failed.failure().is_some());
        assert_eq!(work.get(Work::PredictionStateBuild), 2);
    }

    /// Through the preflight walk: literal prompt tasks build no prediction
    /// state and no early-binding evaluation state at any sequence size;
    /// expression tasks build two states per task.
    #[test]
    fn preflight_builds_prediction_state_only_for_expression_tasks() {
        let filler: String = (0..50)
            .map(|index| format!("field_{index}: {}\n", "x".repeat(64)))
            .collect();
        for tasks in [10u64, 50] {
            let dir = TempDir::new().unwrap();
            write(
                dir.path(),
                "literal.md",
                &format!("---\nagent: codex\nmodel: big\n{filler}---\n\nWork.\n"),
            );
            write(
                dir.path(),
                "expression.md",
                &format!("---\npick: codex\nagent: \"{{{{ pick }}}}\"\n{filler}---\n\nWork.\n"),
            );
            write(
                dir.path(),
                "hintless.md",
                &format!("---\n{filler}---\n\nWork.\n"),
            );
            let mut early = Vec::new();
            for (document, builds) in [("hintless.md", 0), ("literal.md", 0), ("expression.md", 2)]
            {
                let steps: Vec<Value> = (0..tasks)
                    .map(|index| json!({ "name": format!("t-{index}"), "prompt": document }))
                    .collect();
                let path = sequence(dir.path(), Value::Array(steps), "");
                let before = snapshot();
                let graph = build(&path, json!({}));
                let work = snapshot().since(&before);
                if document != "hintless.md" {
                    for step in 0..tasks as usize {
                        assert_eq!(
                            static_agent(&prediction(&graph, step).agent).0,
                            &json!("codex"),
                            "{document} step {step}"
                        );
                    }
                }
                assert_eq!(
                    work.get(Work::PredictionStateBuild),
                    builds * tasks,
                    "{document} × {tasks}"
                );
                early.push(work.get(Work::EarlyBindingStateNodes));
            }
            // A literal hint adds no early-binding evaluation state over a
            // document with no hint; an evaluated one does.
            assert_eq!(early[0], early[1], "{tasks} tasks");
            assert!(early[2] > early[1], "{tasks} tasks: {early:?}");
        }
    }
}
