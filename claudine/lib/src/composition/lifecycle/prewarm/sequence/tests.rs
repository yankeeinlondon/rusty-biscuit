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

// Retained task-stage runtime coverage uses explicit internal models after
// the public parser has validated the current authoring grammar.
fn legacy_graph_for(path: &str) -> (ResolvedCompositionSource, PreflightGraph) {
    use crate::composition::sequence::preflight::runtime_fixture::RuntimeFixture;

    let source = crate::composition::resolve_fixture_source(path).expect("fixture resolves");
    let directory = source.resolved_path.parent().unwrap();
    let fixture = RuntimeFixture::new(&source, directory);
    let resolution = crate::test_support::context_for(&source.resolved_path);
    let plan = resolve_sequence_plan(&fixture.source, &resolution)
        .expect("current grammar resolves")
        .expect("fixture declares a sequence");
    let context = ComposeContext::capture_for_content(directory, "");
    let graph = fixture.build(&plan, context, &resolution).expect("internal preflight model builds");
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
    let (source, graph) = legacy_graph_for(&sequence);
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
                { "say": "step {{ seq }}" },
            ],
        }),
        "Shared work.\n",
    );
    let sequence = write_source(
        dir.path(),
        "seq.md",
        &json!({
            "sequence": [
                { "name": "with-params", "prompt": "shared.md", "with": { "topic": "a task topic" } },
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

#[tokio::test]
#[serial_test::serial]
async fn container_events_are_collected_with_their_lexical_frontmatter() {
    let _dry_run = test_toolkit::EnvGuard::set_safe("PLAYA_DRY_RUN", "0");
    let dir = TempDir::new().unwrap();
    write_source(dir.path(), "prompt.md", &json!({ "failure": [{ "say": "prompt" }] }), "Work.\n");
    write_source(dir.path(), "nested.md", &json!({
        "title": "Child",
        "failure": [{ "say": "referenced {{ fm.title }}" }],
        "sequence": [{ "name": "child", "prompt": "prompt.md" }],
    }), "Child.\n");
    let path = write_source(dir.path(), "seq.md", &json!({
        "title": "Root",
        "initialize": [{ "say": "never prepare initialize" }],
        "failure": [{ "say": "root {{ fm.title }}" }],
        "sequence": [{
            "name": "group",
            "group": {
                "failure": [
                    { "say": "group {{ fm.title }}" },
                    { "say": "late {{ group.failed }}" },
                ],
                "tasks": [
                    { "name": "inline", "sequence": [{ "name": "prompt", "prompt": "prompt.md" }],
                      "failure": [{ "say": "inline {{ fm.title }}" }] },
                    { "name": "referenced", "sequence": "nested.md" },
                ],
            },
        }],
    }), "Root.\n");
    let (source, graph) = graph_for(&path);
    let fake = FakePreparer::recording();
    let worker = PrewarmWorker::deferred_with(false, Arc::new(fake.clone()));

    assert_eq!(submit(&worker.handle(), &source, &graph), 5);
    fake.wait_for_calls(5).await;
    drop(worker);
    assert_eq!(sorted(fake.texts()), strings(&[
        "root Root", "group Root", "inline Root", "referenced Child", "prompt",
    ]));

    let fake = FakePreparer::recording();
    let worker = PrewarmWorker::deferred_with(false, Arc::new(fake.clone()));
    assert_eq!(submit_sequence_preflight(
        &worker.handle(), &GlobalSettings::default(), &graph, &source,
        ["title".to_string()], &ComposeContext::capture_minimal(),
    ), 1, "caller-supplied keys stay late in every lexical container");
    fake.wait_for_calls(1).await;
    drop(worker);
    assert_eq!(fake.texts(), ["prompt"]);
}

/// Under an explicit provider and voice, task `setup:`/`teardown:` stacks (a
/// top-level task, a serial member, a parallel member) are prepared with what
/// task stacks play with, and prompt document lifecycles with the user's
/// configuration. The same text on both paths is two candidates.
#[tokio::test]
#[serial_test::serial]
async fn task_stacks_use_task_playback_and_prompt_lifecycles_use_the_user_configuration() {
    use crate::composition::lifecycle::tts_config_from_settings;
    use crate::composition::sequence::task::task_playback_settings;
    use crate::events::TtsSettings;

    let _dry_run = test_toolkit::EnvGuard::set_safe("PLAYA_DRY_RUN", "0");
    let dir = TempDir::new().unwrap();
    for name in ["a.md", "m.md", "p.md"] {
        write_source(
            dir.path(),
            name,
            &json!({ "failure": [{ "say": format!("{name} failed") }, { "say": "shared phrase" }] }),
            "Work.\n",
        );
    }
    let stacks = |label: &str| {
        json!({
            "setup": [{ "when": "false", "then": [{ "say": format!("{label} setup") }, { "say": "shared phrase" }] }],
            "teardown": [{ "when": "false", "then": [{ "say": format!("{label} teardown") }] }],
        })
    };
    let task = |name: &str, prompt: &str, label: &str| {
        let mut task = stacks(label);
        task["name"] = json!(name);
        task["prompt"] = json!(prompt);
        task
    };
    let sequence = write_source(
        dir.path(),
        "seq.md",
        &json!({
            "sequence": [
                task("top", "a.md", "top"),
                { "name": "serial", "group": { "execution": "serial", "tasks": [task("member", "m.md", "serial member")] } },
                { "name": "parallel", "group": { "execution": "parallel", "tasks": [task("leg", "p.md", "parallel member")] } },
            ],
        }),
        "Body.\n",
    );
    let (source, graph) = legacy_graph_for(&sequence);
    let user = GlobalSettings {
        tts: Some(TtsSettings {
            provider: Some("kokoro".into()),
            voice: Some("af_heart".into()),
            rate: None,
        }),
        ..GlobalSettings::default()
    };
    let fake = FakePreparer::recording();
    let worker = PrewarmWorker::deferred_with(false, Arc::new(fake.clone()));

    let queued = submit_sequence_preflight(
        &worker.handle(),
        &user,
        &graph,
        &source,
        Vec::new(),
        &ComposeContext::capture_minimal(),
    );
    assert_eq!(queued, 11);
    fake.wait_for_calls(11).await;
    drop(worker);

    let task_config = tts_config_from_settings(task_playback_settings().tts.as_ref());
    let user_config = tts_config_from_settings(user.tts.as_ref());
    assert_ne!(task_config, user_config, "the control: the two paths differ");
    let mut by_config: Vec<(String, bool)> = fake
        .calls()
        .into_iter()
        .map(|call| {
            assert!(
                call.config == task_config || call.config == user_config,
                "{}: an unexpected config {:?}",
                call.text,
                call.config
            );
            (call.text, call.config == task_config)
        })
        .collect();
    by_config.sort();
    let expected_task = [
        "top setup",
        "top teardown",
        "serial member setup",
        "serial member teardown",
        "parallel member setup",
        "parallel member teardown",
        "shared phrase",
    ];
    let expected_prompt = ["a.md failed", "m.md failed", "p.md failed", "shared phrase"];
    let mut expected: Vec<(String, bool)> = expected_task
        .iter()
        .map(|text| ((*text).to_string(), true))
        .chain(expected_prompt.iter().map(|text| ((*text).to_string(), false)))
        .collect();
    expected.sort();
    assert_eq!(by_config, expected);
}

/// A task's `setup:`/`teardown:` phrases preflight had to skip (`env`, `ctx`,
/// and a frontmatter value only a template produces) are submitted once the
/// task's context exists, resolved against it and with the task playback
/// configuration. A per-step overlay key and a key the handle is scoped with
/// stay late, the sequence document's `initialize` is never walked, and the
/// literal preflight already queued is not queued again.
#[tokio::test]
#[serial_test::serial]
async fn task_stacks_resolve_with_the_tasks_captured_inputs() {
    use crate::composition::lifecycle::tts_config_from_settings;

    let _dry_run = test_toolkit::EnvGuard::set_safe("PLAYA_DRY_RUN", "0");
    let dir = TempDir::new().unwrap();
    write_source(dir.path(), "a.md", &json!({ "title": "A" }), "Work.\n");
    let guarded = |label: &str| {
        json!([{ "when": "false", "then": [
            { "say": "literal control" },
            { "say": format!("{label} env {{{{ env.PREWARM_NAME }}}}") },
            { "say": format!("{label} ctx {{{{ ctx.today }}}}") },
            { "say": format!("{label} derived {{{{ derived }}}}") },
            { "say": format!("{label} state {{{{ seq }}}}") },
            { "say": format!("{label} topic {{{{ topic }}}}") },
        ] }])
    };
    let sequence = write_source(
        dir.path(),
        "seq.md",
        &json!({
            "project": "Widget",
            "topic": "authored topic",
            "derived": "{{ project }} ready",
            "initialize": [{ "say": "initializing {{ project }}" }],
            "sequence": [
                { "name": "one", "prompt": "a.md", "setup": guarded("setup"), "teardown": guarded("teardown") },
            ],
        }),
        "Body.\n",
    );
    let (source, graph) = legacy_graph_for(&sequence);
    let task = graph.steps[0].task.as_ref().expect("the step declares a task");
    let fake = FakePreparer::recording();
    let worker = PrewarmWorker::deferred_with(false, Arc::new(fake.clone()));
    let handle = worker.handle().with_injected_keys(["topic".to_string()]);

    assert_eq!(submit(&handle, &source, &graph), 1, "preflight queues only the literal");

    // What composition and the task's capture establish.
    let frontmatter = json!({
        "project": "Widget",
        "topic": "authored topic",
        "derived": "Widget ready",
        "state": { "name": "one" },
        "initialize": [{ "say": "initializing {{ project }}" }],
    });
    let mut context = ComposeContext::capture_minimal();
    context.env_mut().insert("PREWARM_NAME".to_string(), "Ada".to_string());
    let today = context.today().to_string();

    let queued = submit_task_stacks(
        &handle,
        &GlobalSettings::default(),
        task,
        frontmatter.as_object().unwrap(),
        &context,
    );
    assert_eq!(queued, 6);
    fake.wait_for_calls(7).await;
    drop(worker);

    let task_config = tts_config_from_settings(task_playback_settings().tts.as_ref());
    assert!(fake.calls().iter().all(|call| call.config == task_config));
    let mut expected = vec!["literal control".to_string()];
    for label in ["setup", "teardown"] {
        expected.push(format!("{label} env Ada"));
        expected.push(format!("{label} ctx {today}"));
        expected.push(format!("{label} derived Widget ready"));
    }
    expected.sort();
    assert_eq!(sorted(fake.texts()), expected);
}

/// The walk reads the sequence document's frontmatter through one shared
/// view: each container's submission copies none of the sibling definitions
/// that frontmatter holds, so the copied nodes grow linearly with the
/// container count (the walk's one decode of the document). A value still
/// holding a template stays late, and an `fm` read is still early.
#[cfg(feature = "work-counters")]
#[test]
fn nested_container_submissions_share_the_sequence_frontmatter() {
    use crate::work_counters::{Work, snapshot};

    let mut copied = Vec::new();
    for containers in [10usize, 55, 100] {
        let dir = TempDir::new().unwrap();
        // No template inside the definitions, so the `sequence:` list is
        // readable state that a per-container copy would include.
        let children: Vec<Value> = (0..containers)
            .map(|index| json!({
                "name": format!("c{index}"),
                "start": [{ "say": "container starts" }],
                "sequence": [{ "name": "work", "shell": format!("echo {index}") }],
            }))
            .collect();
        let sequence = write_source(
            dir.path(),
            "seq.md",
            &json!({
                "title": "Release",
                "pending": "{{ title }} later",
                "success": [{ "say": "{{ pending }} pending" }],
                "finalize": [{ "say": "{{ title }} done" }],
                "sequence": [{ "name": "outer", "sequence": children }],
            }),
            "Body.\n",
        );
        let (source, graph) = graph_for(&sequence);
        let fake = FakePreparer::recording();
        let worker = PrewarmWorker::start_with(Arc::new(fake.clone()));

        let before = snapshot();
        let queued = submit(&worker.handle(), &source, &graph);
        let work = snapshot().since(&before);

        assert_eq!(queued, 2, "each early text is queued once; the templated value stays late");
        assert_eq!(work.get(Work::PrewarmStateBuild), containers as u64 + 1, "one resolution per container and the root's");
        copied.push(work.get(Work::PrewarmStateNodes));
        let prepared = tokio::runtime::Builder::new_current_thread().enable_time().build().unwrap()
            .block_on(async { tokio::time::timeout(std::time::Duration::from_secs(5), fake.wait_for_calls(2)).await });
        assert!(prepared.is_ok(), "the queued phrases were never prepared");
        drop(worker);
        assert_eq!(sorted(fake.texts()), strings(&["Release done", "container starts"]));
    }
    let [small, middle, large] = copied[..] else { unreachable!() };
    assert!(small > 0, "the walk's decode of the document is counted");
    assert_eq!(middle - small, large - middle, "{small} → {middle} → {large}");
}
