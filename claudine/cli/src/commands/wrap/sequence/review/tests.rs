//! Per-task provider selection over real preflight graphs, and how the review
//! screen's rows map back onto it.
//!
//! Each fixture is a sequence written to a temp directory, walked by the
//! library's preflight, collected into sites, and selected against a fake
//! installed-provider snapshot; nothing launches.

use std::cell::Cell;
use std::fs;
use std::io;
use std::path::Path;

use biscuit_tui::prelude::{ABORTED_KIND, CANCELLED_KIND};
use claudine::composition::{
    AgentResolutionState, CompositionError, EffectiveSelectionHints, InstalledProviderSnapshot,
    ModelResolutionReason, ProviderResolutionReason, ResolvedExecutionTarget, SequenceTaskProviderProblem,
};
use claudine::composition::sequence::preflight::TaskIdentity;
use claudine::invocation_context::DocumentEnvironment;
use claudine::model_catalog::ModelCatalogService;
use claudine::provider::Provider;
use serde_json::{Value, json};
use tempfile::TempDir;

use super::super::providers::{self, SelectionInputs, TaskSelections, TaskTarget};
use super::*;

fn write(dir: &Path, name: &str, text: &str) {
    let path = dir.join(name);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, text).unwrap();
}

fn snapshot(installed: &[Provider]) -> InstalledProviderSnapshot {
    InstalledProviderSnapshot {
        runnable: installed.to_vec(),
        excluded: Default::default(),
        all_installed: installed.to_vec(),
        binary_paths: Default::default(),
    }
}

/// Select every site of the sequence `steps` (with `extra` frontmatter)
/// against `installed`, optionally under `--<provider>`.
#[allow(clippy::result_large_err)]
fn select_in(
    dir: &Path,
    steps: Value,
    extra: &str,
    installed: &[Provider],
    explicit: Option<Provider>,
) -> Result<TaskSelections, CompositionError> {
    write(
        dir,
        "seq.md",
        &format!("---\n{extra}sequence: {}\n---\n\nBody.\n", serde_json::to_string(&steps).unwrap()),
    );
    let context = crate::request::test_context_at(dir);
    let path = dir.join("seq.md").display().to_string();
    let source = claudine::composition::resolve_composition_source_in_context(&path, &context).unwrap();
    let plan = claudine::composition::resolve_sequence_plan(&source, &context).unwrap().unwrap();
    let graph = claudine::composition::build_preflight_graph(&plan, &source, &context).unwrap();
    let root_hints = claudine::composition::parse_selection_hints_from_frontmatter(source.markdown.frontmatter()).unwrap();
    let sites = providers::collect_sites(&graph, &plan, &source.resolved_path, &root_hints);
    let snapshot = snapshot(installed);
    let catalog = ModelCatalogService::new();
    let environment = DocumentEnvironment::default();
    providers::select(
        sites,
        &SelectionInputs {
            snapshot: &snapshot,
            catalog: &catalog,
            cli_model: None,
            explicit_provider: explicit,
            favorite: None,
            environment: &environment,
            silent: true,
            dry_run: false,
        },
    )
}

fn identity(text: &str) -> TaskIdentity {
    // Identities are built only by preflight; find the one a test names.
    let mut parts = text.split('.');
    let root = parts.next().unwrap();
    let index: usize = root.trim_start_matches("tasks[").trim_end_matches(']').parse().unwrap();
    let mut identity = TaskIdentity::root(index);
    let rest: Vec<&str> = parts.collect();
    let mut position = 0;
    while position < rest.len() {
        match rest[position] {
            "group" => {
                let leg: usize = rest[position + 1].trim_start_matches("tasks[").trim_end_matches(']').parse().unwrap();
                identity = identity.group_leg(leg);
                position += 2;
            }
            child => {
                let index: usize = child.trim_start_matches("sequence[").trim_end_matches(']').parse().unwrap();
                identity = identity.sequence_child(index);
                position += 1;
            }
        }
    }
    identity
}

fn target<'a>(selections: &'a TaskSelections, at: &str) -> &'a TaskTarget {
    &selections.get(&identity(at)).unwrap_or_else(|| panic!("no site at {at}")).target
}

fn provider_of(target: &TaskTarget) -> Option<Provider> {
    target.planned().map(|target| target.provider)
}

const ALL: &[Provider] = &[Provider::Claude, Provider::Codex, Provider::OpenCode, Provider::Gemini];

// -- classification ----------------------------------------------------------

/// The report's sequence: no document `agent`, one task naming `codex`
/// through `with`, one running a prompt that declares `opencode`. Both are
/// their own provider, so nothing is missing and nothing gates the run.
#[test]
fn tasks_that_name_their_providers_need_no_document_agent() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "plain.md", "---\ntitle: p\n---\n\nPlain.\n");
    write(dir.path(), "review.md", "---\nagent: opencode\n---\n\nReview.\n");
    let selections = select_in(
        dir.path(),
        json!([
            { "name": "implement", "prompt": "plain.md", "with": { "agent": "codex" } },
            { "name": "review", "prompt": "review.md" },
            { "name": "lint", "shell": "true" }
        ]),
        "",
        ALL,
        None,
    )
    .unwrap();
    assert!(matches!(target(&selections, "tasks[0]"), TaskTarget::Own(t) if t.provider == Provider::Codex));
    assert!(matches!(target(&selections, "tasks[1]"), TaskTarget::Own(t) if t.provider == Provider::OpenCode));
    assert_eq!(selections.iter().count(), 2, "a shell task is no site");
    assert!(selections.first_unresolved().is_none());
    assert!(!needs_review(true, &selections));
}

#[test]
fn a_task_without_a_hint_takes_the_nearest_enclosing_sequence_agent() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "plain.md", "---\ntitle: p\n---\n\nPlain.\n");
    write(dir.path(), "nested/inner.md", "---\nagent: gemini\nsequence:\n  - name: deep\n    prompt: ../plain.md\n---\n");
    let selections = select_in(
        dir.path(),
        json!([
            { "name": "outer", "prompt": "plain.md" },
            { "name": "inner", "sequence": "nested/inner.md" }
        ]),
        "agent: claude\n",
        ALL,
        None,
    )
    .unwrap();
    assert!(matches!(target(&selections, "tasks[0]"), TaskTarget::Fallback(t) if t.provider == Provider::Claude));
    // The nested file's own `agent` applies to its tasks ahead of the root's.
    assert!(matches!(target(&selections, "tasks[1].sequence[0]"), TaskTarget::Fallback(t) if t.provider == Provider::Gemini));
}

#[test]
fn a_task_missing_a_provider_with_no_enclosing_agent_is_unresolved() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "plain.md", "---\ntitle: p\n---\n\nPlain.\n");
    let selections = select_in(dir.path(), json!([{ "name": "a", "prompt": "plain.md" }]), "", ALL, None).unwrap();
    assert!(matches!(
        target(&selections, "tasks[0]"),
        TaskTarget::Unresolved { state: AgentResolutionState::NoAgent, own_hint: false, .. }
    ));
    assert!(needs_review(true, &selections));
    assert!(!needs_review(false, &selections), "without a terminal the run fails instead");
}

#[test]
fn a_hint_decided_at_launch_keeps_the_enclosing_agent_only_as_a_fallback() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "plain.md", "---\ntitle: p\n---\n\nPlain.\n");
    let selections = select_in(
        dir.path(),
        json!([
            { "name": "pick", "shell": "echo codex" },
            { "name": "run", "prompt": "plain.md", "with": { "agent": "{{ seq.outputs[0] }}" } }
        ]),
        "agent: claude\n",
        ALL,
        None,
    )
    .unwrap();
    assert!(matches!(target(&selections, "tasks[1]"), TaskTarget::AtLaunch(Some(t)) if t.provider == Provider::Claude));
    assert!(selections.first_unresolved().is_none(), "a dynamic hint never gates the run");
}

#[test]
fn an_explicit_provider_flag_decides_every_task() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "review.md", "---\nagent: opencode\n---\n\nReview.\n");
    write(dir.path(), "plain.md", "---\ntitle: p\n---\n\nPlain.\n");
    let selections = select_in(
        dir.path(),
        json!([{ "name": "a", "prompt": "review.md" }, { "name": "b", "prompt": "plain.md" }]),
        "",
        ALL,
        Some(Provider::Gemini),
    )
    .unwrap();
    for at in ["tasks[0]", "tasks[1]"] {
        assert!(matches!(
            target(&selections, at),
            TaskTarget::Own(t) if t.provider == Provider::Gemini && t.provider_reason == ProviderResolutionReason::ExplicitFlag
        ));
    }
}

#[test]
fn list_hints_follow_the_canonical_installed_provider_rule() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "plain.md", "---\ntitle: p\n---\n\nPlain.\n");
    // One installed member: selected, never the uninstalled first entry.
    let selections = select_in(
        dir.path(),
        json!([{ "name": "a", "prompt": "plain.md", "with": { "agent": ["gemini", "codex"] } }]),
        "",
        &[Provider::Codex],
        None,
    )
    .unwrap();
    assert_eq!(provider_of(target(&selections, "tasks[0]")), Some(Provider::Codex));
    // Two installed: the task's own ambiguous list is reviewed as the task's.
    let selections = select_in(
        dir.path(),
        json!([{ "name": "a", "prompt": "plain.md", "with": { "agent": ["gemini", "codex"] } }]),
        "",
        &[Provider::Codex, Provider::Gemini],
        None,
    )
    .unwrap();
    assert!(matches!(target(&selections, "tasks[0]"), TaskTarget::Unresolved { own_hint: true, .. }));
}

// -- early failure -------------------------------------------------------------

fn selection_failure(error: CompositionError) -> (String, String, Vec<String>, SequenceTaskProviderProblem) {
    match error {
        CompositionError::SequenceTaskProviderSelection { task, property, containers, problem, .. } => {
            (task, property, containers, *problem)
        }
        other => panic!("expected a task provider selection failure, got {other:?}"),
    }
}

/// A task naming a provider that is not installed fails before any task
/// runs, even behind a `when:` that is false, naming the task, the property
/// that names it, and the enclosing group.
#[test]
fn an_uninstalled_provider_fails_preflight_even_behind_a_false_when() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "plain.md", "---\ntitle: p\n---\n\nPlain.\n");
    let error = select_in(
        dir.path(),
        json!([{ "name": "g", "group": { "name": "review", "tasks": [
            { "name": "late", "prompt": "plain.md", "when": "false", "with": { "agent": "gemini" } }
        ] } }]),
        "",
        &[Provider::Claude],
        None,
    )
    .unwrap_err();
    let (_, property, containers, problem) = selection_failure(error);
    assert_eq!(property, "tasks[0].group.tasks[0].with.agent");
    assert_eq!(containers, ["group `review`"]);
    assert!(matches!(
        problem,
        SequenceTaskProviderProblem::Unresolvable { state: AgentResolutionState::SingleNotInstalled { provider: Provider::Gemini }, .. }
    ));
}

#[test]
fn invalid_hint_shapes_fail_as_the_task() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "plain.md", "---\ntitle: p\n---\n\nPlain.\n");
    for (agent, expected) in [
        (json!("nope"), "unknown"),
        (json!(7), "type"),
        (json!(["codex", 7]), "type"),
        (json!([7]), "type"),
    ] {
        let error = select_in(
            dir.path(),
            json!([{ "name": "a", "prompt": "plain.md", "with": { "agent": agent.clone() } }]),
            "",
            ALL,
            None,
        )
        .unwrap_err();
        let (_, property, _, problem) = selection_failure(error);
        assert_eq!(property, "tasks[0].with.agent", "{agent}");
        match expected {
            "unknown" => assert!(
                matches!(problem, SequenceTaskProviderProblem::Unresolvable { state: AgentResolutionState::SingleInvalid { .. }, .. }),
                "{agent}: {problem:?}"
            ),
            _ => assert!(matches!(problem, SequenceTaskProviderProblem::WrongType { key: "agent", .. }), "{agent}: {problem:?}"),
        }
    }
}

#[test]
fn a_hint_reading_its_own_identity_fails_as_circular() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "self.md", "---\nagent: \"{{ env.AGENT }}\"\n---\n\nSelf.\n");
    let error = select_in(dir.path(), json!([{ "name": "a", "prompt": "self.md" }]), "", ALL, None).unwrap_err();
    let (_, property, _, problem) = selection_failure(error);
    assert_eq!(property, "agent");
    assert_eq!(problem, SequenceTaskProviderProblem::Circular("env.AGENT".into()));
}

// -- review ------------------------------------------------------------------

fn chosen(provider: Provider) -> ResolvedExecutionTarget {
    ResolvedExecutionTarget {
        provider,
        provider_reason: ProviderResolutionReason::SequenceReview,
        model: None,
        model_reason: ModelResolutionReason::SequenceReview,
    }
}

/// Own-provider tasks are read-only rows; only the missing task is editable,
/// and its choice is stored by identity as a fallback, so repeated names in
/// nested containers never share it.
#[test]
fn review_edits_only_missing_tasks_and_keys_choices_by_identity() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "plain.md", "---\ntitle: p\n---\n\nPlain.\n");
    write(dir.path(), "review.md", "---\nagent: opencode\n---\n\nReview.\n");
    let mut selections = select_in(
        dir.path(),
        json!([
            { "name": "same", "prompt": "plain.md" },
            { "name": "own", "prompt": "review.md" },
            { "name": "nest", "sequence": [{ "name": "same", "prompt": "plain.md" }] }
        ]),
        "",
        ALL,
        None,
    )
    .unwrap();
    let shown = Cell::new(Vec::new());
    let reviewed = review_selections(&mut selections, |drafts| {
        shown.set(drafts.iter().map(|d| (d.step_name.clone(), d.read_only.clone())).collect());
        Ok(vec![chosen(Provider::Codex), chosen(Provider::Gemini)])
    })
    .unwrap();
    assert!(reviewed);
    assert_eq!(
        shown.take(),
        [("same".to_string(), None), ("own".to_string(), Some("OpenCode".to_string())), ("same".to_string(), None)]
    );
    assert!(matches!(target(&selections, "tasks[0]"), TaskTarget::Fallback(t) if t.provider == Provider::Codex));
    assert!(matches!(target(&selections, "tasks[1]"), TaskTarget::Own(t) if t.provider == Provider::OpenCode));
    assert!(matches!(target(&selections, "tasks[2].sequence[0]"), TaskTarget::Fallback(t) if t.provider == Provider::Gemini));
}

#[test]
fn leaving_the_review_starts_nothing_and_a_wrong_row_count_is_an_error() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "plain.md", "---\ntitle: p\n---\n\nPlain.\n");
    let steps = json!([{ "name": "a", "prompt": "plain.md" }]);
    for kind in [ABORTED_KIND, CANCELLED_KIND] {
        let mut selections = select_in(dir.path(), steps.clone(), "", ALL, None).unwrap();
        assert!(!review_selections(&mut selections, |_| Err(io::Error::new(kind, "left"))).unwrap());
    }
    let mut selections = select_in(dir.path(), steps.clone(), "", ALL, None).unwrap();
    let error = review_selections(&mut selections, |_| Ok(Vec::new())).unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    assert!(selections.first_unresolved().is_some(), "a failed review changes nothing");
    let mut selections = select_in(dir.path(), steps, "", ALL, None).unwrap();
    let error = review_selections(&mut selections, |_| Err(io::Error::other("tty gone"))).unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::Other);
}

/// A body step composes its sequence document, so the document's own hints
/// are the step's: an unusable `agent` there is the document's to review,
/// never a task failure.
#[test]
fn a_body_step_uses_the_sequence_documents_agent() {
    let dir = TempDir::new().unwrap();
    let selections = select_in(dir.path(), json!(["first", "second"]), "agent: codex\n", ALL, None).unwrap();
    assert_eq!(provider_of(target(&selections, "tasks[1]")), Some(Provider::Codex));
    let selections = select_in(dir.path(), json!(["first"]), "agent: nope\n", ALL, None).unwrap();
    assert!(matches!(
        target(&selections, "tasks[0]"),
        TaskTarget::Unresolved { state: AgentResolutionState::SingleInvalid { .. }, own_hint: false, .. }
    ));
    let _ = EffectiveSelectionHints::default();
}
