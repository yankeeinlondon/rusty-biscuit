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

/// The report record of the one trigger warning in `report`.
fn the_trigger_record(report: &ComposeReport) -> crate::markdown::compose::expression::Diagnostic {
    let [warning] = trigger_warnings(&report.warnings)[..] else {
        panic!("one trigger warning expected: {:?}", report.warnings);
    };
    crate::markdown::compose::report_diagnostics::warning_diagnostic(warning)
}

#[test]
fn a_malformed_envelope_reports_the_trigger_file_at_its_yaml_error() {
    let repo = Repo::new();
    std::fs::remove_file(repo.trigger()).unwrap();
    let trigger = repo.write("schemas/broken.yaml", "kind: schema-trigger\nmatch: [unclosed\n");
    let prompt = repo.write("prompts/plain.md", "# Plain\n");

    let (_, report) = repo.compose(&prompt, ComposeOptions::new()).expect("composes");

    let record = the_trigger_record(&report);
    assert_eq!(record.code, SCHEMA_INVALID_TRIGGER_CODE);
    assert_eq!(record.severity, crate::markdown::compose::expression::Severity::Warning);
    assert_eq!(
        record.source,
        Some(crate::markdown::compose::expression::DiagnosticSource {
            file: Some(biscuit_file::canonicalize_simplified(&trigger).unwrap()),
            line: 3,
            column: 1,
        })
    );
    assert!(record.message.contains("at line 3, column 1"), "{}", record.message);
    let [warning] = trigger_warnings(&report.warnings)[..] else { unreachable!() };
    assert_eq!(warning.line_number, None, "the trigger's line is not a line of the prompt");
}

#[test]
fn a_trigger_failure_without_a_position_reports_no_source() {
    let repo = Repo::new();
    let prompt = repo.write("prompts/plain.md", "# Plain\n");

    let (_, report) = repo.compose(&prompt, ComposeOptions::new()).expect("composes");

    let record = the_trigger_record(&report);
    assert_eq!(record.code, SCHEMA_INVALID_TRIGGER_CODE);
    assert_eq!(record.source, None, "no position is invented for unsupported envelope keys");
}

#[test]
fn a_payload_failure_names_the_payload_but_never_places_it_on_the_trigger() {
    let repo = Repo::new();
    std::fs::remove_file(repo.trigger()).unwrap();
    repo.write("schemas/host.yaml", "kind: schema-trigger\nmatch:\n  title: string(required)\n$schema: ./payload.yaml\n");
    let payload = repo.write("schemas/payload.yaml", "$schema:\n  notes: [unclosed\n");
    let prompt = repo.write("prompts/plain.md", "# Plain\n");

    let (_, report) = repo.compose(&prompt, ComposeOptions::new()).expect("composes");

    let record = the_trigger_record(&report);
    assert_eq!(record.source, None, "the payload's line is not a line of the trigger");
    let payload = biscuit_file::canonicalize_simplified(&payload).unwrap();
    assert!(record.message.contains(&payload.display().to_string()), "{}", record.message);
}

#[test]
fn an_embedded_trigger_record_has_no_source_even_with_a_position() {
    let finding = crate::markdown::schemas::InvalidTriggerFinding::new(
        crate::markdown::schemas::TriggerSource::Embedded { bundle: "host".into(), name: "broken.yaml".into() },
        crate::markdown::schemas::errors::SchemaError::TriggerEnvelopeYaml {
            message: "did not find expected ',' or ']'".into(),
            line: Some(3),
            column: Some(1),
        },
    );
    assert_eq!(finding.position(), Some((3, 1)));

    let warning = ComposeWarning::from_invalid_trigger(&finding, "/prompts/plain.md");

    assert_eq!(warning.path, None);
    let record = crate::markdown::compose::report_diagnostics::warning_diagnostic(&warning);
    assert_eq!(record.source, None, "an embedded document is never a local file");
    assert_eq!(record.message, finding.message());
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

/// Each composition is its own invocation: the second one, which reuses the
/// process's cached payloads and file revisions, reports both skipped
/// triggers again (an envelope failure and a payload failure).
#[test]
fn a_new_composition_reports_the_skipped_triggers_again() {
    let repo = Repo::new();
    repo.write("schemas/missing-payload.yaml", "kind: schema-trigger\nmatch:\n  title: string(required)\n$schema: ./absent.yaml\n");
    repo.write("schemas/valid.yaml", "kind: schema-trigger\nmatch:\n  title: string(required)\n$schema:\n  level: number\n");
    let prompt = repo.write("prompts/again.md", "---\ntitle: Again\nlevel: 2\n---\n# {{ title }}\n");

    let reported: Vec<Vec<Option<PathBuf>>> = (0..2)
        .map(|_| {
            let (composed, report) = repo.compose(&prompt, ComposeOptions::new()).expect("composes");
            assert!(composed.content().contains("# Again"), "{}", composed.content());
            trigger_warnings(&report.warnings).iter().map(|warning| warning.path.clone()).collect()
        })
        .collect();

    let expected = vec![Some(repo.trigger()), Some(repo.root.join("schemas/missing-payload.yaml"))];
    assert_eq!(reported[0], expected, "discovery order, once each");
    assert_eq!(reported[1], expected, "a new invocation reports them again");
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

/// The request [`Repo::compose`] composes with.
fn discovery_request(repo: &Repo, prompt: &Path) -> crate::markdown::compose::ComposeRequest {
    let context = biscuit_file::FileResolutionContext::new(prompt.parent().unwrap())
        .with_repository_root(&repo.root)
        .with_source_path(prompt);
    crate::markdown::compose::test_request_in(
        ComposeOptions::new().with_source_file(prompt).with_trigger_schemas(true),
        context,
    )
}

/// An inspection-only pre-flight walk reports the trigger it skipped once,
/// however many walked documents discover it.
#[test]
fn a_preflight_walk_reports_the_skipped_trigger_once() {
    let repo = Repo::new();
    let prompt = repo.write("prompts/parent.md", "# Parent\n\n::file ./child.md\n");
    repo.write("prompts/child.md", "Child body\n");

    let report = Markdown::try_from(prompt.as_path()).unwrap().compose_preflight(&discovery_request(&repo, &prompt)).unwrap();

    let warnings = trigger_warnings(&report.warnings);
    assert_eq!(warnings.len(), 1, "{:?}", report.warnings);
    assert_eq!(warnings[0].path.as_deref(), Some(repo.trigger().as_path()));
}

/// A host that only asks for the effective schema still learns which
/// trigger discovery skipped.
#[test]
fn the_effective_schema_reports_the_skipped_trigger() {
    let repo = Repo::new();
    let prompt = repo.write("prompts/plain.md", "---\ntitle: Plain\n---\nBody\n");

    let (_, warnings) = crate::markdown::compose::effective_schema_for(
        &Markdown::try_from(prompt.as_path()).unwrap(),
        &discovery_request(&repo, &prompt),
    )
    .unwrap();

    let triggers = trigger_warnings(&warnings);
    assert_eq!(triggers.len(), 1, "{warnings:?}");
    assert_eq!(triggers[0].path.as_deref(), Some(repo.trigger().as_path()));
}

/// Reference validation prepares every transcluded child in that child's own
/// context, so its report carries the root's trigger and one only a child's
/// package holds, once each, in discovery order.
#[test]
fn reference_validation_reports_root_and_child_only_triggers_once() {
    use crate::markdown::reference::ReferenceGraphOptions;
    use crate::markdown::reference::validate::ReferenceValidationOptions;

    let repo = Repo::new();
    let child_trigger = repo.write("alpha/schemas/notes.yaml", MEMORY_TRIGGER);
    repo.write("alpha/docs/child.md", "Child body\n");
    let prompt = repo.write(
        "prompts/parent.md",
        "# Parent\n\n::file ../alpha/docs/child.md\n\n::file ../alpha/docs/child.md\n",
    );

    // The observation that makes `alpha` a package, as `md` captures it.
    let catalog = biscuit_file::RepositoryScopeCatalog::new(
        &repo.root,
        Vec::new(),
        vec![repo.root.join("alpha")],
        biscuit_file::PackageAreaFallback::None,
    )
    .unwrap();
    let context = biscuit_file::FileResolutionContext::new(prompt.parent().unwrap())
        .with_repository_root(&repo.root)
        .with_repository_scope_catalog(catalog)
        .with_source_path(&prompt);
    let request = crate::markdown::compose::test_request_in(
        ComposeOptions::new().with_source_file(&prompt).with_trigger_schemas(true),
        context,
    );

    let report = Markdown::try_from(prompt.as_path())
        .unwrap()
        .validate_references(ReferenceValidationOptions::with_graph(ReferenceGraphOptions::with_compose(&request)))
        .unwrap();

    let paths: Vec<_> = trigger_warnings(&report.trigger_warnings).iter().map(|warning| warning.path.clone()).collect();
    assert_eq!(paths, [Some(repo.trigger()), Some(child_trigger)], "{:?}", report.trigger_warnings);
}

/// A repository whose only invalid trigger belongs to package `alpha`, beside
/// a valid `alpha/docs/child.md`: no trigger is in a root document's schema
/// roots, so only a walk into the child finds it.
struct ChildOnly {
    repo: Repo,
    trigger: PathBuf,
}

impl ChildOnly {
    fn new() -> Self {
        let repo = Repo::new();
        std::fs::remove_file(repo.trigger()).unwrap();
        let trigger = repo.write("alpha/schemas/memory.yaml", MEMORY_TRIGGER);
        repo.write("alpha/docs/child.md", "Child body\n");
        Self { repo, trigger }
    }

    /// `options` composed at `prompt` with trigger discovery, `alpha`
    /// observed as a package as `md` observes it.
    fn request(&self, prompt: &Path, options: ComposeOptions) -> crate::markdown::compose::ComposeRequest {
        let root = &self.repo.root;
        let catalog = biscuit_file::RepositoryScopeCatalog::new(
            root,
            Vec::new(),
            vec![root.join("alpha")],
            biscuit_file::PackageAreaFallback::None,
        )
        .unwrap();
        let context = biscuit_file::FileResolutionContext::new(prompt.parent().unwrap())
            .with_repository_root(root)
            .with_repository_scope_catalog(catalog)
            .with_source_path(prompt);
        crate::markdown::compose::test_request_in(
            options.with_source_file(prompt).with_trigger_schemas(true),
            context,
        )
    }

    fn prompt(&self, body: &str) -> PathBuf {
        self.repo.write("docs/prompt.md", body)
    }
}

fn trigger_paths(warnings: &[ComposeWarning]) -> Vec<Option<PathBuf>> {
    trigger_warnings(warnings).iter().map(|warning| warning.path.clone()).collect()
}

const SKIPPED_CHILD: &str = "Parent body\n\n::file ../alpha/docs/child.md when=false\n";

/// A later sibling's failure keeps the trigger an earlier, successfully
/// walked sibling found, after the root's own, once each.
#[test]
fn a_failed_walk_keeps_an_earlier_siblings_trigger() {
    let fixture = ChildOnly::new();
    let root_trigger = fixture.repo.write("schemas/memory.yaml", MEMORY_TRIGGER);
    fixture.repo.write("docs/bad.md", "---\n$schema: ./missing.yaml\n---\nBad\n");
    let prompt = fixture.prompt("Parent\n\n::file ../alpha/docs/child.md\n\n::file ./bad.md\n");
    let request = fixture.request(&prompt, ComposeOptions::new());

    Markdown::try_from(prompt.as_path()).unwrap().compose_preflight(&request).expect_err("bad.md's schema is missing");

    let warnings = request.options().take_preflight_failure_warnings();
    assert_eq!(trigger_paths(&warnings), [Some(root_trigger), Some(fixture.trigger.clone())], "{warnings:?}");
}

/// The approval wrapper hands a successful walk's warnings to its caller,
/// including a trigger only a child composition skips holds.
#[test]
fn preflight_approvals_carry_a_skipped_childs_trigger() {
    let fixture = ChildOnly::new();
    let prompt = fixture.prompt(SKIPPED_CHILD);

    let approvals = Markdown::try_from(prompt.as_path())
        .unwrap()
        .compose_preflight_approvals(&fixture.request(&prompt, ComposeOptions::new()), None)
        .unwrap();

    assert_eq!(trigger_paths(&approvals.warnings), [Some(fixture.trigger.clone())]);
}

/// Every policy error after a successful walk leaves the walk's warnings for
/// the caller.
fn assert_policy_error_keeps_the_trigger(
    body: &str,
    child_only: bool,
    prepare: impl FnOnce(&Repo),
    expect: impl FnOnce(&ShellExpansionError) -> bool,
) {
    assert_handled_policy_error_keeps_the_trigger(body, child_only, prepare, None, expect);
}

/// [`assert_policy_error_keeps_the_trigger`] with `handler` asked for what
/// the policy files leave undecided.
fn assert_handled_policy_error_keeps_the_trigger(
    body: &str,
    child_only: bool,
    prepare: impl FnOnce(&Repo),
    handler: Option<Arc<dyn ShellApprovalHandler>>,
    expect: impl FnOnce(&ShellExpansionError) -> bool,
) {
    let fixture = ChildOnly::new();
    let trigger = if child_only {
        fixture.trigger.clone()
    } else {
        std::fs::remove_file(&fixture.trigger).unwrap();
        fixture.repo.write("schemas/memory.yaml", MEMORY_TRIGGER)
    };
    prepare(&fixture.repo);
    let prompt = fixture.prompt(body);
    let request = fixture.request(&prompt, ComposeOptions::new());

    let error = Markdown::try_from(prompt.as_path())
        .unwrap()
        .compose_preflight_approvals(&request, handler)
        .expect_err("the policy rejects the shell");

    assert!(expect(&error), "{error:?}");
    let warnings = request.options().take_preflight_failure_warnings();
    assert_eq!(trigger_paths(&warnings), [Some(trigger)], "{warnings:?}");
}

const ROOT_SHELL: &str = "Parent body\n\n::shell echo review-probe\n";
const CHILD_AND_SHELL: &str = "Parent body\n\n::file ../alpha/docs/child.md\n\n::shell echo review-probe\n";

#[test]
fn approval_required_keeps_the_root_trigger() {
    assert_policy_error_keeps_the_trigger(ROOT_SHELL, false, |_| {}, |error| {
        matches!(error, ShellExpansionError::ApprovalRequired { .. })
    });
}

#[test]
fn approval_required_keeps_a_child_only_trigger() {
    assert_policy_error_keeps_the_trigger(CHILD_AND_SHELL, true, |_| {}, |error| {
        matches!(error, ShellExpansionError::ApprovalRequired { .. })
    });
}

fn blacklist_probe(repo: &Repo) {
    repo.write(".darkmatter-shell-blacklist", "exact echo review-probe\n");
}

#[test]
fn a_user_blacklist_keeps_the_root_trigger() {
    assert_policy_error_keeps_the_trigger(ROOT_SHELL, false, blacklist_probe, |error| {
        matches!(error, ShellExpansionError::Blacklisted { .. })
    });
}

#[test]
fn a_user_blacklist_keeps_a_child_only_trigger() {
    assert_policy_error_keeps_the_trigger(CHILD_AND_SHELL, true, blacklist_probe, |error| {
        matches!(error, ShellExpansionError::Blacklisted { .. })
    });
}

#[test]
fn the_builtin_blacklist_keeps_a_child_only_trigger() {
    let body = "Parent body\n\n::file ../alpha/docs/child.md\n\n::shell rm -rf /\n";
    assert_policy_error_keeps_the_trigger(body, true, |_| {}, |error| {
        matches!(error, ShellExpansionError::Blacklisted { .. })
    });
}

/// A pre-approved composition reports the trigger its approval-membership
/// walk found in a child it then skips, once, beside what it reports itself.
#[test]
fn a_pre_approved_composition_reports_a_skipped_childs_trigger() {
    let fixture = ChildOnly::new();
    let prompt = fixture.prompt(&format!("{SKIPPED_CHILD}\n::shell echo review-probe\n"));
    let options = ComposeOptions::new()
        .with_shell_timeout(std::time::Duration::from_secs(30))
        .with_pre_approved_commands(std::collections::HashSet::from(["echo review-probe".to_string()]));

    let (composed, report) = Markdown::try_from(prompt.as_path())
        .unwrap()
        .compose_with_warnings(&fixture.request(&prompt, options))
        .unwrap();

    assert!(composed.content().contains("review-probe"), "{}", composed.content());
    assert!(!composed.content().contains("Child body"), "{}", composed.content());
    assert_eq!(trigger_paths(&report.warnings), [Some(fixture.trigger.clone())], "{:?}", report.warnings);
}

/// The membership walk and composition both find an active child's trigger;
/// it is reported once.
#[test]
fn a_pre_approved_composition_reports_an_active_childs_trigger_once() {
    let fixture = ChildOnly::new();
    let prompt = fixture.prompt("Parent body\n\n::file ../alpha/docs/child.md\n");
    let options = ComposeOptions::new().with_pre_approved_commands(std::collections::HashSet::<String>::new());

    let (composed, report) = Markdown::try_from(prompt.as_path())
        .unwrap()
        .compose_with_warnings(&fixture.request(&prompt, options))
        .unwrap();

    assert!(composed.content().contains("Child body"), "{}", composed.content());
    assert_eq!(trigger_paths(&report.warnings), [Some(fixture.trigger.clone())], "{:?}", report.warnings);
}

/// Public pre-flight over the skipped child warns once (inspection control).
#[test]
fn a_preflight_walk_reports_a_skipped_childs_trigger_once() {
    let fixture = ChildOnly::new();
    let prompt = fixture.prompt(SKIPPED_CHILD);

    let report = Markdown::try_from(prompt.as_path())
        .unwrap()
        .compose_preflight(&fixture.request(&prompt, ComposeOptions::new()))
        .unwrap();

    assert_eq!(trigger_paths(&report.warnings), [Some(fixture.trigger.clone())]);
}

/// Answers every approval request with `Deny`, counting the requests.
struct DenyingApproval(Arc<AtomicUsize>);

impl ShellApprovalHandler for DenyingApproval {
    fn approve(&self, _request: ShellApprovalRequest) -> Result<ShellApprovalDecision, ShellExpansionError> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Ok(ShellApprovalDecision::Deny)
    }
}

#[test]
fn a_handler_denial_keeps_a_child_only_trigger() {
    let asked = Arc::new(AtomicUsize::new(0));
    let handler: Arc<dyn ShellApprovalHandler> = Arc::new(DenyingApproval(Arc::clone(&asked)));
    assert_handled_policy_error_keeps_the_trigger(CHILD_AND_SHELL, true, |_| {}, Some(handler), |error| {
        matches!(error, ShellExpansionError::Denied { .. })
    });
    assert_eq!(asked.load(Ordering::SeqCst), 1, "the handler decided the shell");
}

/// A whitelist that is not UTF-8 cannot be read on any OS.
#[test]
fn an_unreadable_whitelist_keeps_a_child_only_trigger() {
    let unreadable = |repo: &Repo| {
        std::fs::write(repo.root.join(".darkmatter-shell-whitelist"), [0xff, 0xfe, b'\n']).unwrap();
    };
    assert_policy_error_keeps_the_trigger(CHILD_AND_SHELL, true, unreadable, |error| {
        matches!(error, ShellExpansionError::PolicyIo { .. })
    });
}

/// A blocking policy over every trigger code this suite denies.
fn denials() -> [BlockingPolicy; 2] {
    [BlockingPolicy::default().deny_code(SCHEMA_INVALID_TRIGGER_CODE), BlockingPolicy::default().deny_all()]
}

/// The one blocking warning of a policy stop: the trigger at `trigger`, still
/// a warning and still about the trigger file.
fn assert_stopped_on(error: &MarkdownError, trigger: &Path) {
    let MarkdownError::BlockedDiagnostics { blocking, .. } = error else {
        panic!("a policy stop, not a schema-load error: {error:?}");
    };
    let [blocked] = trigger_warnings(blocking)[..] else {
        panic!("the trigger warning blocks: {blocking:?}");
    };
    assert_eq!(blocked.path.as_deref(), Some(trigger));
    let record = crate::markdown::compose::report_diagnostics::warning_diagnostic(blocked);
    assert_eq!(record.severity, crate::markdown::compose::expression::Severity::Warning);
}

/// A denied trigger only a skipped child holds stops the approval walk
/// before the handler is asked to approve anything, with the walk's
/// warnings left for the caller.
#[test]
fn a_denied_child_only_trigger_stops_the_approval_walk_before_any_prompt() {
    let fixture = ChildOnly::new();
    let prompt = fixture.prompt(&format!("{SKIPPED_CHILD}\n::shell echo review-probe-effect\n"));
    for policy in denials() {
        let asked = Arc::new(AtomicUsize::new(0));
        let request = fixture.request(&prompt, ComposeOptions::new().with_blocking_policy(policy));

        let error = Markdown::try_from(prompt.as_path())
            .unwrap()
            .compose_preflight_approvals(&request, Some(Arc::new(CountingApproval(Arc::clone(&asked)))))
            .expect_err("the denied trigger stops the walk");

        let ShellExpansionError::Preflight(stop) = &error else {
            panic!("the existing pre-flight policy stop: {error:?}");
        };
        assert_stopped_on(stop, &fixture.trigger);
        assert_eq!(asked.load(Ordering::SeqCst), 0, "no prompt before the stop");
        let warnings = request.options().take_preflight_failure_warnings();
        assert_eq!(trigger_paths(&warnings), [Some(fixture.trigger.clone())], "{warnings:?}");
    }
}

/// A `git init` of `marker` beside the prompt: an effect any OS can observe.
fn marker_command(fixture: &ChildOnly, marker: &str) -> (String, PathBuf) {
    (format!("git init --quiet {marker}"), fixture.repo.root.join("docs").join(marker))
}

/// The exact approvals the walk of `prompt` finds, as a host computes them
/// under the default policy.
fn exact_approvals(fixture: &ChildOnly, prompt: &Path) -> std::collections::HashSet<String> {
    Markdown::try_from(prompt)
        .unwrap()
        .compose_preflight(&fixture.request(prompt, ComposeOptions::new()))
        .unwrap()
        .approval_set()
        .into_iter()
        .collect()
}

/// Composes `prompt` with its exact approvals under `policy`.
fn compose_pre_approved(
    fixture: &ChildOnly,
    prompt: &Path,
    policy: BlockingPolicy,
) -> Result<(Markdown, ComposeReport), Box<ComposeFailure>> {
    let options = ComposeOptions::new()
        .with_shell_timeout(std::time::Duration::from_secs(30))
        .with_blocking_policy(policy)
        .with_pre_approved_commands(exact_approvals(fixture, prompt));
    Markdown::try_from(prompt).unwrap().compose_with_warnings(&fixture.request(prompt, options))
}

/// The library's own membership gate stops on a denied trigger only a
/// skipped child holds before the approved command runs, whatever host
/// computed the approvals; under the default policy the same command runs
/// and the warning is reported.
fn assert_pre_approved_stop(document: impl Fn(&str) -> String) {
    let fixture = ChildOnly::new();
    for (index, policy) in denials().into_iter().enumerate() {
        let (command, marker) = marker_command(&fixture, &format!("denied-{index}"));
        let prompt = fixture.prompt(&document(&command));

        let failure = compose_pre_approved(&fixture, &prompt, policy).expect_err("the denied trigger stops");

        assert_stopped_on(&failure.error, &fixture.trigger);
        assert!(!marker.exists(), "the approved command ran before the stop");
    }

    let (command, marker) = marker_command(&fixture, "allowed");
    let prompt = fixture.prompt(&document(&command));
    let (_, report) = compose_pre_approved(&fixture, &prompt, BlockingPolicy::default()).unwrap();
    assert!(marker.exists(), "the approved command runs under the default policy");
    assert_eq!(report.shell_expansions_applied + report.frontmatter_shell_expansions_applied, 1);
    assert_eq!(trigger_paths(&report.warnings), [Some(fixture.trigger.clone())], "{:?}", report.warnings);
}

#[test]
fn a_denied_child_only_trigger_stops_a_pre_approved_body_shell() {
    assert_pre_approved_stop(|command| format!("Parent body\n\n::shell {command}\n\n::file ../alpha/docs/child.md when=false\n"));
}

#[test]
fn a_denied_child_only_trigger_stops_a_pre_approved_frontmatter_shell() {
    assert_pre_approved_stop(|command| format!("---\nstamp: \"$({command})\"\n---\n{SKIPPED_CHILD}"));
}

/// A frontmatter `as_markdown` call runs the pre-approved gate before the
/// root reaches its own: a gate rejection carries the trigger only the
/// skipped child holds, once.
#[test]
fn a_failed_nested_gate_keeps_a_child_only_trigger() {
    let fixture = ChildOnly::new();
    let prompt = fixture.prompt(&format!(
        "---\nintro: '{{{{ as_markdown(\"Nested\") }}}}'\n---\n{SKIPPED_CHILD}\n::shell echo unapproved\n"
    ));
    let options = ComposeOptions::new().with_pre_approved_commands(std::collections::HashSet::<String>::new());

    let failure = Markdown::try_from(prompt.as_path())
        .unwrap()
        .compose_with_warnings(&fixture.request(&prompt, options))
        .expect_err("the unapproved shell fails the gate");

    assert!(failure.error.pre_approval_violation().is_some(), "{:?}", failure.error);
    assert_eq!(trigger_paths(&failure.warnings), [Some(fixture.trigger.clone())], "{:?}", failure.warnings);
}

/// The nested gate judges its walk's warnings under the request's policy:
/// a denied child-only trigger stops the request before the nested content
/// or any approved command runs.
#[test]
fn a_denied_child_only_trigger_stops_the_nested_gate() {
    let fixture = ChildOnly::new();
    for (index, policy) in denials().into_iter().enumerate() {
        let (command, marker) = marker_command(&fixture, &format!("nested-{index}"));
        let prompt = fixture.prompt(&format!(
            "---\nintro: '{{{{ as_markdown(\"::shell {command}\") }}}}'\n---\n{SKIPPED_CHILD}"
        ));

        let failure = compose_pre_approved(&fixture, &prompt, policy).expect_err("the denied trigger stops");

        assert_stopped_on(&failure.error, &fixture.trigger);
        assert!(!marker.exists(), "the nested command ran before the stop");
        assert_eq!(trigger_paths(&failure.warnings), [Some(fixture.trigger.clone())], "{:?}", failure.warnings);
    }
}
