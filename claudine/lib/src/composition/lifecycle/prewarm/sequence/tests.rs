//! Sequence preflight submits every early phrase its graph reaches, judged
//! against each task's inputs, before any step runs.

use std::fs;
use std::path::Path;
use std::sync::Arc;

use darkmatter::markdown::compose::ComposeContext;
use serde_json::{Value, json};
use tempfile::TempDir;

use super::*;
use crate::composition::lifecycle::prewarm::PrewarmWorker;
use crate::composition::lifecycle::prewarm::fakes::FakePreparer;
use crate::composition::sequence::resolve_sequence_plan;
use crate::composition::{PreflightGraph, build_preflight_graph};

fn write_source(dir: &Path, name: &str, frontmatter: &Value, body: &str) -> String {
    let mut text = String::from("---\n");
    for (key, value) in frontmatter.as_object().expect("frontmatter is a mapping") {
        text.push_str(&format!("{key}: {}\n", serde_json::to_string(value).unwrap()));
    }
    text.push_str("---\n\n");
    text.push_str(body);
    let path = dir.join(name);
    fs::write(&path, text).unwrap();
    path.display().to_string()
}

fn graph_for(path: &str) -> (ResolvedCompositionSource, PreflightGraph) {
    let source = crate::composition::resolve_fixture_source(path).expect("fixture resolves");
    let context = crate::test_support::context_for(&source.resolved_path);
    let plan = resolve_sequence_plan(&source, &context)
        .expect("plan resolves")
        .expect("fixture declares a sequence");
    let graph = build_preflight_graph(&plan, &source, &context).expect("preflight passes");
    (source, graph)
}

fn submit(handle: &PrewarmHandle, source: &ResolvedCompositionSource, graph: &PreflightGraph) -> usize {
    submit_sequence_preflight(
        handle,
        &GlobalSettings::default(),
        graph,
        source,
        Vec::new(),
        &ComposeContext::capture_minimal(),
    )
}

fn sorted(mut texts: Vec<String>) -> Vec<String> {
    texts.sort();
    texts
}

fn strings(values: &[&str]) -> Vec<String> {
    let mut strings: Vec<String> = values.iter().map(ToString::to_string).collect();
    strings.sort();
    strings
}

/// A three-prompt sequence: each document's early phrase, each task's
/// `setup:`/`teardown:`, and the sequence document's own lifecycle (one step
/// runs its body) are all queued by one call, which returns although the
/// first preparation never finishes.
#[tokio::test]
#[serial_test::serial]
async fn every_early_phrase_of_three_prompts_is_queued_before_any_step_runs() {
    let _dry_run = test_toolkit::EnvGuard::set_safe("PLAYA_DRY_RUN", "0");
    let dir = TempDir::new().unwrap();
    for (name, title) in [("a.md", "Alpha"), ("b.md", "Beta"), ("c.md", "Gamma")] {
        write_source(
            dir.path(),
            name,
            &json!({
                "title": title,
                "success": [{ "say": "{{ title }} is done" }],
                "failure": [{ "say": "{{ title }} failed: {{ err.msg }}" }],
            }),
            "Do the work.\n",
        );
    }
    let sequence = write_source(
        dir.path(),
        "seq.md",
        &json!({
            "project": "Widget",
            "finalize": [{ "say": "{{ project }} sequence over" }],
            "sequence": [
                {
                    "name": "one",
                    "prompt": "a.md",
                    "setup": [{ "say": "preparing {{ project }}" }],
                    "teardown": [{ "when": "false", "then": [{ "say": "never true, still warmed" }] }],
                },
                { "name": "two", "prompt": "b.md" },
                { "name": "three", "prompt": "c.md" },
                { "name": "four" },
            ],
        }),
        "The sequence body.\n",
    );
    let (source, graph) = graph_for(&sequence);
    let fake = FakePreparer::blocking();
    let worker = PrewarmWorker::deferred_with(false, Arc::new(fake.clone()));

    let queued = submit(&worker.handle(), &source, &graph);

    assert_eq!(queued, 6);
    fake.wait_for_calls(1).await;
    assert_eq!(fake.calls().len(), 1, "preparation is serial and still blocked");
    fake.release(6);
    fake.wait_for_calls(6).await;
    assert_eq!(
        sorted(fake.texts()),
        strings(&[
            "Widget sequence over",
            "preparing Widget",
            "never true, still warmed",
            "Alpha is done",
            "Beta is done",
            "Gamma is done",
        ])
    );
}

/// One document reached by two tasks is judged against each task's inputs; a
/// value preflight could only obtain by evaluating a template, and a read of
/// `env`, `ctx`, or a per-step overlay key, stay for the document's own
/// preparation.
#[tokio::test]
#[serial_test::serial]
async fn task_inputs_unresolved_values_and_captures_are_left_late() {
    let _dry_run = test_toolkit::EnvGuard::set_safe("PLAYA_DRY_RUN", "0");
    let dir = TempDir::new().unwrap();
    write_source(
        dir.path(),
        "shared.md",
        &json!({
            "name": "Shared",
            "topic": "default topic",
            "derived": "{{ name }} copy",
            "probe": "$(touch never-run)",
            "success": [
                { "say": "{{ name }} is done" },
                { "say": "about {{ topic }}" },
                { "say": "derived {{ derived }}" },
                { "say": "probe {{ probe }}" },
                { "say": "home {{ env.HOME }}" },
                { "say": "area {{ ctx.area }}" },
                { "say": "step {{ state }}" },
            ],
        }),
        "Shared work.\n",
    );
    let sequence = write_source(
        dir.path(),
        "seq.md",
        &json!({
            "sequence": [
                { "name": "with-params", "prompt": "shared.md", "params": { "topic": "a task topic" } },
                { "name": "without", "prompt": "shared.md" },
            ],
        }),
        "Body.\n",
    );
    let (source, graph) = graph_for(&sequence);
    let fake = FakePreparer::recording();
    let worker = PrewarmWorker::deferred_with(false, Arc::new(fake.clone()));

    assert_eq!(submit(&worker.handle(), &source, &graph), 2);
    fake.wait_for_calls(2).await;
    assert_eq!(
        sorted(fake.texts()),
        strings(&["Shared is done", "about default topic"]),
        "`topic` is late only for the task that passes it",
    );
    assert!(!dir.path().join("never-run").exists(), "no shell ran");
}

#[test]
fn a_disabled_handle_walks_nothing() {
    let dir = TempDir::new().unwrap();
    write_source(dir.path(), "a.md", &json!({ "success": [{ "say": "done" }] }), "x\n");
    let sequence = write_source(
        dir.path(),
        "seq.md",
        &json!({ "sequence": [{ "name": "one", "prompt": "a.md" }] }),
        "Body.\n",
    );
    let (source, graph) = graph_for(&sequence);
    let fake = FakePreparer::recording();
    let worker = PrewarmWorker::deferred_with(true, Arc::new(fake.clone()));

    assert_eq!(submit(&worker.handle(), &source, &graph), 0);
    drop(worker);
    assert!(fake.calls().is_empty());
}
