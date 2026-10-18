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
