//! Per-task provider prediction through the public preflight walk: each
//! classification state, the layer that wins, graph identities, and the
//! purity of prediction (nothing late, circular, or shell is evaluated).

use std::fs;
use std::path::Path;

use serde_json::{Value, json};
use tempfile::TempDir;

use super::super::*;
use super::{HintLayer, HintState, LaunchDecision, TaskProviderClass};
use crate::composition::sequence::resolve_sequence_plan;

fn write(dir: &Path, name: &str, text: &str) {
    let path = dir.join(name);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, text).unwrap();
}

/// A sequence document whose steps are `sequence`, as YAML flow.
fn sequence(dir: &Path, sequence: Value, extra: &str) -> String {
    let text = format!("---\n{extra}sequence: {}\n---\n\nBody.\n", serde_json::to_string(&sequence).unwrap());
    write(dir, "seq.md", &text);
    dir.join("seq.md").display().to_string()
}

/// Build the graph with `setters` as the caller's values.
fn build(path: &str, setters: Value) -> PreflightGraph {
    let source = crate::composition::resolve_fixture_source(path).unwrap();
    let resolution = crate::test_support::context_for(&source.resolved_path);
    let plan = resolve_sequence_plan(&source, &resolution).unwrap().expect("a sequence");
    let context = darkmatter::markdown::compose::ComposeContext::capture_for_content(
        source.resolved_path.parent().unwrap(),
        "",
    );
    let mut loader = Loader::new(&source, context, resolution);
    loader.caller_setters = setters.as_object().cloned().unwrap_or_default();
    loader.walk_plan(&plan).unwrap();
    loader.finish()
}

fn prediction(graph: &PreflightGraph, step: usize) -> &super::ProviderPrediction {
    graph.steps[step].task.as_ref().unwrap().provider.as_ref().expect("a prompt task predicts")
}

fn static_agent(state: &HintState) -> (&Value, HintLayer, &str) {
    match state {
        HintState::Static { value, source } => (value, source.layer, source.property.as_str()),
        other => panic!("expected a static hint, got {other:?}"),
    }
}

#[test]
fn a_literal_document_agent_is_predicted_from_the_document() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "review.md", "---\nagent: opencode\nmodel: big\n---\n\nReview.\n");
    let path = sequence(dir.path(), json!([{ "name": "review", "prompt": "review.md" }]), "");
    let graph = build(&path, json!({}));
    let prediction = prediction(&graph, 0);
    assert_eq!(prediction.class(), TaskProviderClass::Predicted);
    assert_eq!(static_agent(&prediction.agent), (&json!("opencode"), HintLayer::Document, "agent"));
    assert_eq!(prediction.static_hints(), json!({ "agent": "opencode", "model": "big" }).as_object().cloned().unwrap());
}

#[test]
fn a_task_with_agent_outranks_the_document_and_names_its_property() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "review.md", "---\nagent: opencode\n---\n\nReview.\n");
    let path = sequence(
        dir.path(),
        json!([{ "name": "review", "prompt": "review.md", "with": { "agent": "codex" } }]),
        "",
    );
    let graph = build(&path, json!({}));
    assert_eq!(
        static_agent(&prediction(&graph, 0).agent),
        (&json!("codex"), HintLayer::With, "tasks[0].with.agent")
    );
}

#[test]
fn a_caller_setter_outranks_the_task_with() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "review.md", "---\nagent: opencode\n---\n\nReview.\n");
    let path = sequence(
        dir.path(),
        json!([{ "name": "review", "prompt": "review.md", "with": { "agent": "codex" } }]),
        "",
    );
    let graph = build(&path, json!({ "agent": "gemini" }));
    assert_eq!(static_agent(&prediction(&graph, 0).agent), (&json!("gemini"), HintLayer::CallerSetter, "agent"));
}

#[test]
fn a_document_agent_template_over_static_values_is_evaluated() {
    let dir = TempDir::new().unwrap();
    // The document's template reads a key the task's `with` supplies.
    write(dir.path(), "review.md", "---\npick: claude\nagent: \"{{ pick }}\"\n---\n\nReview.\n");
    let path = sequence(
        dir.path(),
        json!([{ "name": "review", "prompt": "review.md", "with": { "pick": "codex" } }]),
        "",
    );
    let graph = build(&path, json!({}));
    assert_eq!(static_agent(&prediction(&graph, 0).agent), (&json!("codex"), HintLayer::Document, "agent"));
}

#[test]
fn a_task_without_an_agent_is_missing_a_provider_but_keeps_its_model() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "review.md", "---\nmodel: small\n---\n\nReview.\n");
    let path = sequence(dir.path(), json!([{ "name": "review", "prompt": "review.md" }]), "");
    let graph = build(&path, json!({}));
    let prediction = prediction(&graph, 0);
    assert_eq!(prediction.class(), TaskProviderClass::Missing);
    assert_eq!(prediction.static_hints(), json!({ "model": "small" }).as_object().cloned().unwrap());
}

#[test]
fn a_hint_reading_a_later_output_is_decided_at_launch() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "review.md", "---\ntitle: r\n---\n\nReview.\n");
    let path = sequence(
        dir.path(),
        json!([
            { "name": "pick", "shell": "echo codex" },
            { "name": "review", "prompt": "review.md", "with": { "agent": "{{ seq.outputs[0] }}" } }
        ]),
        "",
    );
    let graph = build(&path, json!({}));
    let prediction = prediction(&graph, 1);
    assert_eq!(prediction.class(), TaskProviderClass::DecidedAtLaunch);
    assert!(matches!(prediction.agent, HintState::DecidedAtLaunch { reason: LaunchDecision::LateValue, .. }), "{prediction:?}");
}

#[test]
fn a_hint_reading_a_key_the_graph_writes_at_runtime_is_decided_at_launch() {
    let dir = TempDir::new().unwrap();
    // The first document's `success` stack writes `chosen`; the second task's
    // `with.agent` reads it. Preflight sees the static default only.
    write(dir.path(), "first.md", "---\nsuccess:\n  - set:\n      chosen: opencode\n---\n\nFirst.\n");
    write(dir.path(), "second.md", "---\ntitle: s\n---\n\nSecond.\n");
    let path = sequence(
        dir.path(),
        json!([
            { "name": "first", "prompt": "first.md" },
            { "name": "second", "prompt": "second.md", "with": { "agent": "{{ chosen }}" } }
        ]),
        "chosen: codex\n",
    );
    let graph = build(&path, json!({}));
    assert!(
        matches!(prediction(&graph, 1).agent, HintState::DecidedAtLaunch { reason: LaunchDecision::RuntimeWritten, .. }),
        "{:?}",
        prediction(&graph, 1)
    );
    // A runtime write of `agent` itself moves every static `agent` to the launch.
    write(dir.path(), "first.md", "---\nagent: claude\nsuccess:\n  - set:\n      agent: opencode\n---\n\nFirst.\n");
    let graph = build(&path, json!({}));
    assert!(matches!(prediction(&graph, 0).agent, HintState::DecidedAtLaunch { reason: LaunchDecision::RuntimeWritten, .. }));
}

#[test]
fn a_hint_reading_its_own_identity_is_circular() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "review.md", "---\nagent: \"{{ env.AGENT }}\"\n---\n\nReview.\n");
    let path = sequence(dir.path(), json!([{ "name": "review", "prompt": "review.md" }]), "");
    let graph = build(&path, json!({}));
    let prediction = prediction(&graph, 0);
    assert!(matches!(&prediction.agent, HintState::Circular { variable, .. } if variable == "env.AGENT"), "{prediction:?}");
    assert!(prediction.failure().is_some());
    // Through another key, and for `model`.
    write(dir.path(), "review.md", "---\nbase: \"{{ env.MODEL }}\"\nmodel: \"{{ base }}\"\n---\n\nReview.\n");
    let graph = build(&path, json!({}));
    assert!(matches!(&prediction_of(&graph).model, HintState::Circular { variable, .. } if variable == "env.MODEL"));
}

fn prediction_of(graph: &PreflightGraph) -> &super::ProviderPrediction {
    prediction(graph, 0)
}

#[test]
fn a_shell_substitution_is_never_evaluated_at_preflight() {
    let dir = TempDir::new().unwrap();
    let marker = dir.path().join("ran");
    write(
        dir.path(),
        "review.md",
        &format!("---\npick: \"$(touch {})\"\nagent: \"{{{{ pick }}}}\"\n---\n\nReview.\n", marker.display()),
    );
    let path = sequence(dir.path(), json!([{ "name": "review", "prompt": "review.md" }]), "");
    let graph = build(&path, json!({}));
    assert!(matches!(prediction(&graph, 0).agent, HintState::DecidedAtLaunch { reason: LaunchDecision::ShellSubstitution, .. }));
    assert!(!marker.exists(), "prediction ran a shell substitution");
}

#[test]
fn an_otherwise_static_hint_that_fails_to_evaluate_is_a_failure() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "review.md", "---\nagent: \"{{ no_such_function(1) }}\"\n---\n\nReview.\n");
    let path = sequence(dir.path(), json!([{ "name": "review", "prompt": "review.md" }]), "");
    let graph = build(&path, json!({}));
    assert!(matches!(prediction(&graph, 0).agent, HintState::Failed { .. }), "{:?}", prediction(&graph, 0));
}

/// Only an evaluable value reaches the evaluator: literals, late values,
/// circular reads, and shell substitutions never call it.
#[test]
fn prediction_calls_the_evaluator_only_for_an_evaluable_template() {
    let document = json!({
        "literal": "codex",
        "late": "{{ seq.outputs[0] }}",
        "circular": "{{ env.AGENT }}",
        "shell": "$(pick)",
        "current": "{{ current.branch }}",
    });
    let document = document.as_object().unwrap();
    for key in ["literal", "late", "circular", "shell", "current"] {
        let mut frontmatter = serde_json::Map::new();
        frontmatter.insert("agent".into(), document[key].clone());
        let empty = serde_json::Map::new();
        let layers = super::HintLayers {
            document: &frontmatter,
            document_path: Path::new("/doc.md"),
            with: &empty,
            with_evaluated: &empty,
            with_deferred: &[],
            with_failed: &Default::default(),
            with_origin: Path::new("/seq.md"),
            task_property: "tasks[0]",
            step_frontmatter: &Default::default(),
            caller_setters: &empty,
            sequence_path: Path::new("/seq.md"),
        };
        let mut evaluate = |_: &Value, _: &std::collections::HashMap<String, Value>| -> Result<
            Result<Value, crate::composition::lifecycle_context::LifecycleCause>,
            crate::composition::CompositionError,
        > { panic!("`{key}` must not be evaluated at preflight") };
        super::predict(&layers, &mut evaluate).unwrap();
    }
}

#[test]
fn every_task_gets_a_distinct_graph_identity() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "review.md", "---\nagent: codex\n---\n\nReview.\n");
    write(dir.path(), "task.yaml", "kind: task\nprompt: review.md\n");
    let path = sequence(
        dir.path(),
        json!([
            { "name": "a", "task": "task.yaml" },
            { "name": "b", "task": "task.yaml" },
            { "name": "c", "group": { "tasks": [
                { "name": "same", "prompt": "review.md" },
                { "name": "same-too", "sequence": [{ "name": "same", "prompt": "review.md" }] }
            ] } }
        ]),
        "",
    );
    let graph = build(&path, json!({}));
    let identity = |task: &PreflightTask| task.identity.as_str().to_string();
    let a = graph.steps[0].task.as_ref().unwrap();
    let b = graph.steps[1].task.as_ref().unwrap();
    assert_eq!((identity(a), identity(b)), ("tasks[0]".into(), "tasks[1]".into()));
    let PreflightAction::Group(group) = &graph.steps[2].task.as_ref().unwrap().action else { panic!("a group") };
    assert_eq!(identity(&group.tasks[0]), "tasks[2].group.tasks[0]");
    let PreflightAction::Sequence(nested) = &group.tasks[1].action else { panic!("a sequence") };
    assert_eq!(identity(nested.steps[0].task.as_ref().unwrap()), "tasks[2].group.tasks[1].sequence[0]");
}

fn failed_at(state: &HintState) -> (&str, String) {
    match state {
        HintState::Failed { source, cause } => (source.property.as_str(), cause.to_string()),
        other => panic!("expected a failed hint, got {other:?}"),
    }
}

/// An ordinary expression error in a static `with` selection input is a
/// located failure, never a value decided at launch.
#[test]
fn a_with_selection_input_that_fails_to_evaluate_is_a_failure() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "plain.md", "---\ntitle: p\n---\n\nPlain.\n");
    for key in ["agent", "model"] {
        let path = sequence(
            dir.path(),
            json!([
                { "name": "first", "prompt": "plain.md" },
                { "name": "second", "prompt": "plain.md", "with": { key: "{{ no_such_function() }}" } }
            ]),
            "agent: claude\n",
        );
        let graph = build(&path, json!({}));
        let prediction = prediction(&graph, 1);
        let state = if key == "agent" { &prediction.agent } else { &prediction.model };
        let (property, cause) = failed_at(state);
        assert_eq!(property, format!("tasks[1].with.{key}"));
        assert!(cause.contains("no_such_function"), "{cause}");
        assert!(prediction.failure().is_some());
        // Shell discovery still treats the entry as bound at the task's turn.
        let reference = graph.prompt_references.iter().find(|reference| reference.with.is_empty()
            && reference.deferred_with == vec![key.to_string()]);
        assert!(reference.is_some(), "{:?}", graph.prompt_references);
    }
}

/// A document hint reading a `with` entry that failed is that entry's failure,
/// not the document's default for it and not a late value.
#[test]
fn a_document_hint_reading_a_failed_with_entry_is_that_entrys_failure() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "review.md", "---\npick: claude\nagent: \"{{ pick }}\"\n---\n\nReview.\n");
    let path = sequence(
        dir.path(),
        json!([{ "name": "review", "prompt": "review.md", "with": { "pick": "{{ no_such_function() }}" } }]),
        "",
    );
    let graph = build(&path, json!({}));
    let (property, cause) = failed_at(&prediction(&graph, 0).agent);
    assert_eq!(property, "tasks[0].with.pick");
    assert!(cause.contains("no_such_function"), "{cause}");
    // A caller setter replacing the entry means the document never reads it.
    let graph = build(&path, json!({ "pick": "codex" }));
    assert_eq!(static_agent(&prediction(&graph, 0).agent).0, &json!("codex"));
}

/// A failed `with` entry no selection input reads changes no prediction.
#[test]
fn a_failed_with_entry_no_hint_reads_leaves_the_prediction_alone() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "review.md", "---\nagent: codex\n---\n\nReview {{ topic }}.\n");
    let path = sequence(
        dir.path(),
        json!([{ "name": "review", "prompt": "review.md", "with": { "topic": "{{ no_such_function() }}" } }]),
        "",
    );
    let graph = build(&path, json!({}));
    let prediction = prediction(&graph, 0);
    assert_eq!(static_agent(&prediction.agent).0, &json!("codex"));
    assert!(prediction.failure().is_none());
}

/// Genuine late values in `with` stay decided at launch, and the provider's
/// own identity stays circular, even though none evaluates at preflight.
#[test]
fn late_with_selection_inputs_remain_decided_at_launch() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "plain.md", "---\ntitle: p\n---\n\nPlain.\n");
    for (key, value) in [
        ("agent", "{{ seq.outputs[0] }}"),
        ("model", "{{ seq.outputs[0] }}"),
        ("agent", "{{ current.branch }}"),
        ("agent", "{{ loop.count }}"),
    ] {
        let path = sequence(
            dir.path(),
            json!([
                { "name": "pick", "shell": "echo codex" },
                { "name": "run", "prompt": "plain.md", "with": { key: value } }
            ]),
            "",
        );
        let graph = build(&path, json!({}));
        let prediction = prediction(&graph, 1);
        let state = if key == "agent" { &prediction.agent } else { &prediction.model };
        assert!(matches!(state, HintState::DecidedAtLaunch { reason: LaunchDecision::LateValue, .. }), "{value}: {state:?}");
        assert!(prediction.failure().is_none(), "{value}");
    }
    // A concurrent leg reading its group's values.
    let path = sequence(
        dir.path(),
        json!([{ "name": "batch", "group": { "tasks": [
            { "name": "leg", "prompt": "plain.md", "with": { "agent": "{{ group.name }}" } }
        ] } }]),
        "",
    );
    let graph = build(&path, json!({}));
    let PreflightAction::Group(group) = &graph.steps[0].task.as_ref().unwrap().action else { panic!("a group") };
    let leg = group.tasks[0].provider.as_ref().unwrap();
    assert!(matches!(leg.agent, HintState::DecidedAtLaunch { reason: LaunchDecision::LateValue, .. }), "{leg:?}");
    // The selected provider's own identity.
    for (key, variable) in [("agent", "env.AGENT"), ("model", "env.MODEL")] {
        let path = sequence(
            dir.path(),
            json!([{ "name": "run", "prompt": "plain.md", "with": { key: format!("{{{{ {variable} }}}}") } }]),
            "",
        );
        let graph = build(&path, json!({}));
        let prediction = prediction(&graph, 0);
        let state = if key == "agent" { &prediction.agent } else { &prediction.model };
        assert!(matches!(state, HintState::Circular { variable: read, .. } if read == variable), "{state:?}");
    }
}

/// `null` is no `agent` hint, in every layer, so the task is missing a
/// provider; an empty string is an invalid name, still the task's own hint.
/// `[]` breaks the launch schema's rule that an `agent` list needs at least
/// one entry, so in the document or `with` it is a failure located there; a
/// caller setter's `[]` is no hint. The deciding layer still wins: an empty
/// caller or `with` value masks the document's `agent`.
#[test]
fn an_empty_agent_hint_is_missing_a_provider_or_a_located_failure() {
    let dir = TempDir::new().unwrap();
    for (empty, expected) in [
        (json!(null), TaskProviderClass::Missing),
        (json!([]), TaskProviderClass::Predicted),
        (json!(""), TaskProviderClass::Predicted),
    ] {
        let flow = serde_json::to_string(&empty).unwrap();
        let empty_list = empty == json!([]);
        // The prompt document's own frontmatter.
        write(dir.path(), "empty.md", &format!("---\nagent: {flow}\nmodel: {flow}\n---\n\nEmpty.\n"));
        let path = sequence(dir.path(), json!([{ "name": "a", "prompt": "empty.md" }]), "agent: claude\n");
        let graph = build(&path, json!({}));
        let task = prediction(&graph, 0);
        assert_eq!(task.class(), expected, "document {flow}: {task:?}");
        if empty_list {
            assert!(matches!(&task.agent, HintState::EmptyAgentList { source }
                if source.property == "agent"
                    && source.layer == HintLayer::Document
                    && source.source_path.ends_with("empty.md")));
            // `model: []` is valid and means no model.
            assert!(matches!(&task.model, HintState::Static { value, .. } if value == &json!([])));
        } else {
            assert!(task.failure().is_none(), "document {flow}");
        }
        // The task's `with`, over a document naming a provider.
        write(dir.path(), "named.md", "---\nagent: gemini\n---\n\nNamed.\n");
        let path = sequence(
            dir.path(),
            json!([{ "name": "a", "prompt": "named.md", "with": { "agent": empty.clone(), "model": empty.clone() } }]),
            "agent: claude\n",
        );
        let graph = build(&path, json!({}));
        let task = prediction(&graph, 0);
        assert_eq!(task.class(), expected, "with {flow}: {task:?}");
        if empty_list {
            assert!(matches!(&task.agent, HintState::EmptyAgentList { source }
                if source.property == "tasks[0].with.agent" && source.layer == HintLayer::With));
            assert!(task.failure().is_some());
        } else {
            assert_eq!(static_agent(&task.agent).1, HintLayer::With, "with {flow}");
            assert!(task.failure().is_none(), "with {flow}");
        }
        // A caller setter, over both: `[]` is no hint here.
        let graph = build(&path, json!({ "agent": empty.clone() }));
        let task = prediction(&graph, 0);
        let setter_expected = if empty_list { TaskProviderClass::Missing } else { expected };
        assert_eq!(task.class(), setter_expected, "setter {flow}: {task:?}");
        assert!(task.failure().is_none(), "{flow}");
    }
}

/// A hint reading a key only a lifecycle `set` creates (it has no value before
/// that write) is decided at launch, whether the task's `with` or its prompt
/// document reads it.
#[test]
fn a_hint_reading_a_key_only_a_runtime_write_creates_is_decided_at_launch() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "first.md", "---\nsuccess:\n  - set:\n      chosen: opencode\n---\n\nFirst.\n");
    write(dir.path(), "plain.md", "---\ntitle: s\n---\n\nSecond.\n");
    write(dir.path(), "reads.md", "---\nvia: \"{{ chosen }}\"\nagent: \"{{ via }}\"\n---\n\nSecond.\n");
    for second in [
        json!({ "name": "second", "prompt": "plain.md", "with": { "agent": "{{ chosen }}" } }),
        json!({ "name": "second", "prompt": "reads.md" }),
    ] {
        let path = sequence(dir.path(), json!([{ "name": "first", "prompt": "first.md" }, second.clone()]), "");
        let graph = build(&path, json!({}));
        let prediction = prediction(&graph, 1);
        assert!(
            matches!(prediction.agent, HintState::DecidedAtLaunch { reason: LaunchDecision::RuntimeWritten, .. }),
            "{second}: {prediction:?}"
        );
        assert_eq!(prediction.class(), TaskProviderClass::DecidedAtLaunch, "{second}");
    }
    // A key nothing writes stays an ordinary read of an absent value.
    let path = sequence(
        dir.path(),
        json!([{ "name": "only", "prompt": "plain.md", "with": { "agent": "{{ never_written }}" } }]),
        "",
    );
    let graph = build(&path, json!({}));
    assert!(!matches!(prediction(&graph, 0).agent, HintState::DecidedAtLaunch { .. }), "{:?}", prediction(&graph, 0));
}
