//! Composition beside a schema trigger discovery skipped as invalid: the
//! document composes, the trigger is reported once as a coded warning about
//! the trigger file, and only an explicit `--deny` makes it blocking.

use super::*;
use crate::markdown::compose::policy::BlockingPolicy;
use crate::markdown::compose::shell_expansion::{ShellApprovalDecision, ShellApprovalHandler, ShellApprovalRequest};
use crate::markdown::compose::{ComposeFailure, ShellExpansionError};
use crate::markdown::schemas::SCHEMA_INVALID_TRIGGER_CODE;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

/// The trigger from the original incident: unsupported envelope keys and a
/// null `match:`.
const MEMORY_TRIGGER: &str = "kind: schema-trigger\nname: Memory Trigger\ndescription: TODO\nmatch:\n";

/// A repository with the incident trigger at `schemas/memory.yaml`.
struct Repo {
    _dir: tempfile::TempDir,
    root: PathBuf,
}

impl Repo {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let root = biscuit_file::canonicalize_simplified(dir.path()).unwrap();
        std::fs::create_dir_all(root.join(".git")).unwrap();
        let repo = Self { _dir: dir, root };
        repo.write("schemas/memory.yaml", MEMORY_TRIGGER);
        repo
    }

    fn write(&self, relative: &str, contents: &str) -> PathBuf {
        let path = self.root.join(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, contents).unwrap();
        path
    }

    fn trigger(&self) -> PathBuf {
        self.root.join("schemas/memory.yaml")
    }

    /// Composes `prompt` with trigger discovery as `md compose` runs it.
    fn compose(&self, prompt: &Path, options: ComposeOptions) -> Result<(Markdown, ComposeReport), Box<ComposeFailure>> {
        let context = biscuit_file::FileResolutionContext::new(prompt.parent().unwrap())
            .with_repository_root(&self.root)
            .with_source_path(prompt);
        Markdown::try_from(prompt)
            .unwrap()
            .compose_with_warnings(&crate::markdown::compose::test_request_in(
                options.with_source_file(prompt).with_trigger_schemas(true),
                context,
            ))
    }
}

fn trigger_warnings(warnings: &[ComposeWarning]) -> Vec<&ComposeWarning> {
    warnings
        .iter()
        .filter(|warning| warning.code.as_deref() == Some(SCHEMA_INVALID_TRIGGER_CODE))
        .collect()
}

/// Counts approvals, so a test can prove a `$(...)` never ran.
struct CountingApproval(Arc<AtomicUsize>);

impl ShellApprovalHandler for CountingApproval {
    fn approve(&self, _request: ShellApprovalRequest) -> Result<ShellApprovalDecision, ShellExpansionError> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Ok(ShellApprovalDecision::AllowOnce)
    }
}

#[test]
fn a_prompt_without_schema_composes_beside_the_incident_trigger() {
    let repo = Repo::new();
    let prompt = repo.write("prompts/merge-conflicts.md", "---\ntitle: Merge\n---\n# {{ title }}\n");

    let (composed, report) = repo.compose(&prompt, ComposeOptions::new()).expect("an invalid trigger never fails composition");

    assert!(composed.content().contains("# Merge"), "{}", composed.content());
    let [warning] = trigger_warnings(&report.warnings)[..] else {
        panic!("one trigger warning expected: {:?}", report.warnings);
    };
    assert_eq!(warning.path.as_deref(), Some(repo.trigger().as_path()), "the warning is about the trigger");
    assert_eq!(warning.source.as_deref(), Some("darkmatter.schema"));
    assert_eq!(warning.stage, "schema_validation");
    assert!(warning.message.starts_with("ignoring invalid schema trigger"), "{}", warning.message);
    assert!(warning.message.contains("found unknown keys: name, description"), "{}", warning.message);
    assert!(!warning.message.contains("merge-conflicts.md"), "never blames the prompt: {}", warning.message);
    assert_eq!(
        warning.invalid_trigger_source(),
        Some(crate::markdown::schemas::TriggerSource::File(repo.trigger())),
        "a host recognizes the trigger by its discovery identity"
    );
    assert_eq!(ComposeWarning::new("schema_validation", "other").invalid_trigger_source(), None);
}

#[test]
fn a_transcluded_child_reports_the_shared_trigger_once() {
    let repo = Repo::new();
    let prompt = repo.write("prompts/parent.md", "# Parent\n\n::file ./child.md\n");
    repo.write("prompts/child.md", "Child body\n");

    let (composed, report) = repo.compose(&prompt, ComposeOptions::new()).unwrap();

    assert!(composed.content().contains("Child body"));
    assert_eq!(trigger_warnings(&report.warnings).len(), 1, "{:?}", report.warnings);
}

#[test]
fn the_warning_is_kept_when_the_authored_schema_fails_to_load() {
    let repo = Repo::new();
    let prompt = repo.write("prompts/bad-schema.md", "---\n$schema: ./missing.yaml\n---\nbody\n");

    let failure = repo.compose(&prompt, ComposeOptions::new()).expect_err("an authored schema that cannot load fails");

    assert!(
        matches!(failure.error, MarkdownError::SchemaValidationFailed { .. }),
        "the authored schema stays an error: {:?}",
        failure.error
    );
    assert_eq!(trigger_warnings(&failure.warnings).len(), 1, "{:?}", failure.warnings);
}

#[test]
fn a_valid_trigger_beside_the_invalid_one_still_enforces_its_schema() {
    let repo = Repo::new();
    repo.write(
        "schemas/a-gate.yaml",
        "kind: schema-trigger\nmatch:\n  gate: string(required)\n$schema:\n  count: number(required)\n",
    );
    let prompt = repo.write("prompts/gated.md", "---\ngate: yes\ncount: many\n---\nbody\n");

    let failure = repo.compose(&prompt, ComposeOptions::new()).expect_err("the valid trigger's schema rejects `count`");

    assert!(matches!(failure.error, MarkdownError::SchemaValidationFailed { .. }), "{:?}", failure.error);
    assert_eq!(trigger_warnings(&failure.warnings).len(), 1, "{:?}", failure.warnings);
}

fn assert_denied_before_any_effect(policy: BlockingPolicy) {
    let repo = Repo::new();
    let prompt = repo.write("prompts/effect.md", "---\nstamp: \"$(echo ran)\"\n---\n{{ stamp }}\n");
    let approvals = Arc::new(AtomicUsize::new(0));
    let options = ComposeOptions::new()
        .with_blocking_policy(policy)
        .with_shell_approval_handler(Arc::new(CountingApproval(Arc::clone(&approvals))));

    let failure = repo.compose(&prompt, options).expect_err("a denied trigger warning stops composition");

    let MarkdownError::BlockedDiagnostics { blocking, .. } = &failure.error else {
        panic!("a policy stop, not a schema-load error: {:?}", failure.error);
    };
    let [blocked] = trigger_warnings(blocking)[..] else {
        panic!("the trigger warning blocks: {blocking:?}");
    };
    assert_eq!(blocked.path.as_deref(), Some(repo.trigger().as_path()));
    assert_eq!(approvals.load(Ordering::SeqCst), 0, "no effect ran before the stop");
}

#[test]
fn denying_the_code_stops_before_the_first_effect() {
    assert_denied_before_any_effect(BlockingPolicy::default().deny_code(SCHEMA_INVALID_TRIGGER_CODE));
}

#[test]
fn denying_all_stops_before_the_first_effect() {
    assert_denied_before_any_effect(BlockingPolicy::default().deny_all());
}

#[test]
fn disabled_trigger_discovery_reports_nothing() {
    let repo = Repo::new();
    let prompt = repo.write("prompts/plain.md", "---\ntitle: Plain\n---\n# {{ title }}\n");
    let context = biscuit_file::FileResolutionContext::new(prompt.parent().unwrap())
        .with_repository_root(&repo.root)
        .with_source_path(&prompt);

    let (_, report) = Markdown::try_from(prompt.as_path())
        .unwrap()
        .compose_with(&crate::markdown::compose::test_request_in(
            ComposeOptions::new().with_source_file(&prompt).with_trigger_schemas(false),
            context,
        ))
        .unwrap();

    assert!(trigger_warnings(&report.warnings).is_empty(), "{:?}", report.warnings);
}
