//! Shell commands judged against the captured `permissions.shell-commands`:
//! the built-in blacklist refuses before any allow, approval, or cache; an
//! exclusion (`!` entry) of any layer asks instead of pre-approving; the
//! repository widens only through committed content; an "always allow"
//! choice saves to the user configuration and approves the rest of the run
//! whether or not it could be saved; and an invalid block runs nothing.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};

use darkmatter::lifecycle::executor::{ShellRunError, ShellRunner};
use darkmatter::lifecycle::md_host::{CommandOutput, MdShellRunner};
use darkmatter::lifecycle::run::ApprovalEpoch;
use darkmatter::markdown::compose::shell_expansion::template::ShellPreApprovals;
use darkmatter::markdown::compose::shell_expansion::types::{
    ShellApprovalDecision, ShellApprovalHandler, ShellApprovalRequest, ShellExpansionError,
};
use darkmatter::markdown::compose::{
    ComposeContext, ComposeOperation, ComposeOptions, ComposeReport, ComposeRequest, RequestSnapshot,
};
use darkmatter::markdown::{Markdown, MarkdownError};
use tempfile::TempDir;

/// Answers every prompt with one decision and records each request.
struct Scripted {
    decision: ShellApprovalDecision,
    requests: Mutex<Vec<ShellApprovalRequest>>,
}

impl Scripted {
    fn new(decision: ShellApprovalDecision) -> Arc<Self> {
        Arc::new(Self { decision, requests: Mutex::new(Vec::new()) })
    }

    fn requests(&self) -> Vec<ShellApprovalRequest> {
        self.requests.lock().unwrap().clone()
    }
}

impl ShellApprovalHandler for Scripted {
    fn approve(&self, request: ShellApprovalRequest) -> Result<ShellApprovalDecision, ShellExpansionError> {
        self.requests.lock().unwrap().push(request);
        Ok(self.decision.clone())
    }
}

/// A document `body` in `dir`, whose home is `dir` too, composed through the
/// body and frontmatter shell stages.
fn doc_options(dir: &Path, rules: &[&str]) -> ComposeOptions {
    ComposeOptions::new_with_context(ComposeContext::capture_for_content(dir, ""))
        .only(&[ComposeOperation::FrontmatterShellExpansion, ComposeOperation::ShellExpansion])
        .with_source_file(dir.join("doc.md"))
        .with_config_snapshot(crate::request_support::shell_config(dir, rules))
        .with_shell_working_directory(dir)
}

fn compose(dir: &Path, body: &str, options: ComposeOptions) -> Result<(Markdown, ComposeReport), MarkdownError> {
    let path = dir.join("doc.md");
    std::fs::write(&path, body).unwrap();
    Markdown::try_from(path.as_path()).unwrap().compose_with(&crate::request_support::request(options))
}

fn shell_error(error: &MarkdownError) -> &ShellExpansionError {
    match error {
        MarkdownError::ShellExpansion(inner) => inner,
        other => panic!("expected a shell error, got {other}"),
    }
}

fn is_blacklisted(error: &MarkdownError) -> bool {
    matches!(shell_error(error), ShellExpansionError::Blacklisted { .. })
}

/// The `!` entry an approval-required error names.
fn exclusion_of(error: &ShellExpansionError) -> Option<String> {
    match error {
        ShellExpansionError::ApprovalRequired { exclusion, .. } => exclusion.as_ref().map(|exclusion| exclusion.entry.clone()),
        other => panic!("expected an approval-required error, got {other:?}"),
    }
}

/// A program on every host's search path that the built-in blacklist does
/// not refuse and that prints its arguments.
const ECHO: &str = "echo";

// ── The built-in blacklist wins ─────────────────────────────────────────

#[test]
fn the_builtin_blacklist_refuses_a_command_an_allow_rule_names() {
    let dir = TempDir::new().unwrap();
    let options = doc_options(dir.path(), &["prefix rm", "exact rm -rf scratch"]);
    let error = compose(dir.path(), "::shell rm -rf scratch\n", options).unwrap_err();
    assert!(is_blacklisted(&error), "{error:?}");
}

#[test]
fn the_builtin_blacklist_refuses_before_an_approve_everything_handler_is_asked() {
    let dir = TempDir::new().unwrap();
    let handler = Scripted::new(ShellApprovalDecision::AllowOnce);
    let options = doc_options(dir.path(), &[]).with_shell_approval_handler(handler.clone());
    let error = compose(dir.path(), "::shell rm -rf scratch\n", options).unwrap_err();
    assert!(is_blacklisted(&error), "{error:?}");
    assert!(handler.requests().is_empty(), "nothing may approve a refused command");
}

#[test]
fn the_builtin_blacklist_refuses_a_pre_approved_command() {
    let dir = TempDir::new().unwrap();
    let approved: ShellPreApprovals = ["rm -rf scratch".to_string()].into_iter().collect();
    let options = doc_options(dir.path(), &[]).with_pre_approved_commands(approved);
    let error = compose(dir.path(), "::shell rm -rf scratch\n", options).unwrap_err();
    assert!(is_blacklisted(&error), "{error:?}");
}

/// An earlier approval of one segment, cached for the run, cannot carry a
/// chain whose other segment the built-in blacklist refuses.
#[test]
fn a_cached_approval_of_one_segment_cannot_carry_a_refused_segment() {
    let dir = TempDir::new().unwrap();
    let handler = Scripted::new(ShellApprovalDecision::AllowOnce);
    let options = doc_options(dir.path(), &[]).with_shell_approval_handler(handler.clone());
    let body = format!("::shell {ECHO} ok\n\n::shell {ECHO} ok && rm -rf scratch\n");
    let error = compose(dir.path(), &body, options).unwrap_err();
    assert!(is_blacklisted(&error), "{error:?}");
    assert_eq!(handler.requests().len(), 1, "only the first command was asked about");
}

#[test]
fn the_builtin_blacklist_refuses_a_template_rule_and_a_dangerous_filled_value() {
    let dir = TempDir::new().unwrap();
    let options = doc_options(dir.path(), &["template rm {{ string }}"]);
    let error = compose(dir.path(), "---\ntarget: scratch\n---\n::shell rm {{ target }}\n", options).unwrap_err();
    assert!(is_blacklisted(&error), "{error:?}");

    let dir = TempDir::new().unwrap();
    let options = doc_options(dir.path(), &["template git {{ string }}"]);
    let error = compose(dir.path(), "---\naction: \"a;b\"\n---\n::shell git {{ action }}\n", options.clone());
    assert!(!matches!(&error, Err(error) if is_blacklisted(error)), "a safe value is not refused: {error:?}");
    let error = compose(dir.path(), "---\naction: reset\n---\n::shell git {{ action }}\n", options).unwrap_err();
    assert!(is_blacklisted(&error), "{error:?}");
}

struct Discard;

impl CommandOutput for Discard {
    fn stdout(&self, _: &str) {}
    fn stderr(&self, _: &str) {}
}

#[test]
fn the_builtin_blacklist_refuses_an_approved_lifecycle_command() {
    let dir = TempDir::new().unwrap();
    let request = crate::request_support::request(doc_options(dir.path(), &["prefix rm"]));
    let approved: ShellPreApprovals = ["rm -rf scratch".to_string()].into_iter().collect();
    let never = || false;
    let runner = MdShellRunner {
        permission: Ok(ApprovalEpoch(1)),
        approved: &approved,
        request: &request,
        source_path: &dir.path().join("doc.md"),
        output: &Discard,
        interrupted: &never,
    };
    match runner.run("rm -rf scratch") {
        Err(ShellRunError::Value { source }) => {
            assert!(matches!(source, ShellExpansionError::Blacklisted { .. }), "{source:?}");
        }
        other => panic!("expected the built-in refusal, got {other:?}"),
    }
}

// ── Exclusions ask ──────────────────────────────────────────────────────

#[test]
fn a_user_exclusion_makes_an_allowed_command_ask_and_once_approves_it() {
    let dir = TempDir::new().unwrap();
    let rules = ["prefix echo", "!exact echo secret"];
    let error = compose(dir.path(), "::shell echo secret\n", doc_options(dir.path(), &rules)).unwrap_err();
    assert_eq!(exclusion_of(shell_error(&error)).as_deref(), Some("!exact echo secret"));

    // An allowed neighbour still runs.
    let (composed, _) = compose(dir.path(), "::shell echo public\n", doc_options(dir.path(), &[])).unwrap();
    assert!(composed.content().contains("public"));

    let handler = Scripted::new(ShellApprovalDecision::AllowOnce);
    let options = doc_options(dir.path(), &[]).with_shell_approval_handler(handler.clone());
    let (composed, _) = compose(dir.path(), "::shell echo secret\n", options).unwrap();
    assert!(composed.content().contains("secret"));
    let requests = handler.requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].exclusion.as_ref().map(|exclusion| exclusion.entry.as_str()), Some("!exact echo secret"));
}

#[test]
fn a_noninteractive_preflight_of_an_excluded_command_requires_approval() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("doc.md");
    std::fs::write(&path, "::shell echo secret\n").unwrap();
    let request = crate::request_support::request(doc_options(dir.path(), &["prefix echo", "!prefix echo"]));
    let error = Markdown::try_from(path.as_path()).unwrap().compose_preflight_approvals(&request, None).unwrap_err();
    assert_eq!(exclusion_of(&error).as_deref(), Some("!prefix echo"));
}

#[test]
fn a_chain_with_one_excluded_segment_asks_about_the_whole_chain() {
    let dir = TempDir::new().unwrap();
    let rules = ["prefix echo", "!exact echo two"];
    let error = compose(dir.path(), "::shell echo one && echo two\n", doc_options(dir.path(), &rules)).unwrap_err();
    assert_eq!(exclusion_of(shell_error(&error)).as_deref(), Some("!exact echo two"));

    let handler = Scripted::new(ShellApprovalDecision::AllowOnce);
    let options = doc_options(dir.path(), &[]).with_shell_approval_handler(handler.clone());
    let (composed, _) = compose(dir.path(), "::shell echo one && echo two\n", options).unwrap();
    assert!(composed.content().contains("two"));
    assert_eq!(handler.requests().len(), 1);
}

/// Runs git with a fixture-local identity, so the caller's hooks and signing
/// configuration never reach the fixture.
fn git(root: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(["-c", "user.name=fixture", "-c", "user.email=fixture@example.com", "-c", "commit.gpgsign=false"])
        .args(args)
        .current_dir(root)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .status()
        .expect("git must be runnable");
    assert!(status.success(), "git {args:?} failed");
}

/// A repository whose committed `.darkmatter.yaml` holds `committed` and
/// whose working copy holds `worktree` (`None`: as committed), with an empty
/// home beside it; the request is prepared from the repository root, as
/// `md compose` prepares it.
struct Repository {
    _temp: TempDir,
    root: PathBuf,
    home: PathBuf,
}

impl Repository {
    fn new(committed: &str, worktree: Option<&str>) -> Self {
        let temp = TempDir::new().unwrap();
        let base = biscuit_file::canonicalize_simplified(temp.path()).unwrap();
        let root = base.join("repo");
        let home = base.join("home");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::create_dir_all(&home).unwrap();
        git(&root, &["init", "-q"]);
        std::fs::write(root.join(".darkmatter.yaml"), committed).unwrap();
        git(&root, &["add", ".darkmatter.yaml"]);
        git(&root, &["commit", "-q", "-m", "policy"]);
        if let Some(worktree) = worktree {
            std::fs::write(root.join(".darkmatter.yaml"), worktree).unwrap();
        }
        Self { _temp: temp, root, home }
    }

    /// Composes `body` without an approval handler.
    fn compose(&self, body: &str) -> Result<(Markdown, ComposeReport), MarkdownError> {
        let path = self.root.join("doc.md");
        std::fs::write(&path, body).unwrap();
        let options = ComposeOptions::new_with_context(ComposeContext::capture_for_content(&self.root, ""))
            .only(&[ComposeOperation::ShellExpansion])
            .with_source_file(&path)
            .with_shell_working_directory(&self.root);
        let snapshot = RequestSnapshot::new(&self.root).with_home(Some(self.home.clone()));
        let request = ComposeRequest::prepare(options, &snapshot).unwrap();
        Markdown::try_from(path.as_path()).unwrap().compose_with(&request)
    }
}

fn shell_commands(entries: &[&str]) -> String {
    let mut text = String::from("permissions:\n  shell-commands:\n");
    for entry in entries {
        text.push_str(&format!("    - {}\n", serde_json::to_string(entry).unwrap()));
    }
    text
}

#[test]
fn a_committed_repository_allow_runs_and_a_head_exclusion_asks() {
    let allowed = Repository::new(&shell_commands(&["prefix echo"]), None);
    let (composed, _) = allowed.compose("::shell echo committed\n").unwrap();
    assert!(composed.content().contains("committed"));

    // An exclusion committed at HEAD asks even when the working copy has
    // removed it: a removal widens, so it waits for a commit.
    let excluded = Repository::new(
        &shell_commands(&["prefix echo", "!exact echo committed"]),
        Some(&shell_commands(&["prefix echo"])),
    );
    let error = excluded.compose("::shell echo committed\n").unwrap_err();
    assert_eq!(exclusion_of(shell_error(&error)).as_deref(), Some("!exact echo committed"));
}

#[test]
fn an_uncommitted_allow_is_not_effective_and_an_uncommitted_exclusion_is() {
    let widened = Repository::new("{}\n", Some(&shell_commands(&["prefix echo"])));
    let error = widened.compose("::shell echo widened\n").unwrap_err();
    assert_eq!(exclusion_of(shell_error(&error)), None, "no layer allowed it: {error:?}");

    let narrowed = Repository::new(
        &shell_commands(&["prefix echo"]),
        Some(&shell_commands(&["prefix echo", "!exact echo narrowed"])),
    );
    let error = narrowed.compose("::shell echo narrowed\n").unwrap_err();
    assert_eq!(exclusion_of(shell_error(&error)).as_deref(), Some("!exact echo narrowed"));
    let (composed, _) = narrowed.compose("::shell echo other\n").unwrap();
    assert!(composed.content().contains("other"));
}

// ── "Always allow" saves to the user configuration ──────────────────────

#[test]
fn always_allow_saves_to_the_user_file_and_applies_for_the_rest_of_the_run() {
    let dir = TempDir::new().unwrap();
    let original = "# my settings\npermissions:\n  network: deny # stays\n";
    std::fs::write(dir.path().join(".darkmatter.yaml"), original).unwrap();
    let before = crate::request_support::saved_shell_rules(dir.path());
    assert!(before.is_empty(), "{before:?}");

    let handler = Scripted::new(ShellApprovalDecision::AllowCommandPersist);
    let options = doc_options(dir.path(), &[]).with_shell_approval_handler(handler.clone());
    let (composed, report) = compose(dir.path(), "::shell echo first\n\n::shell echo second\n", options).unwrap();
    assert!(composed.content().contains("first") && composed.content().contains("second"));
    assert_eq!(handler.requests().len(), 1, "the saved rule approves the second command");
    assert!(report.warnings.iter().all(|warning| warning.code.as_deref() != Some("dm.config.not_saved")));

    assert_eq!(crate::request_support::saved_shell_rules(dir.path()), ["prefix echo"]);
    let after = std::fs::read_to_string(dir.path().join(".darkmatter.yaml")).unwrap();
    assert!(after.starts_with("# my settings\npermissions:\n  network: deny # stays\n"), "{after}");

    // The next run reads it: nothing is asked.
    let handler = Scripted::new(ShellApprovalDecision::Deny);
    let options = doc_options(dir.path(), &[]).with_shell_approval_handler(handler.clone());
    compose(dir.path(), "::shell echo third\n", options).unwrap();
    assert!(handler.requests().is_empty());
}

#[test]
fn an_always_allow_an_exclusion_still_matches_approves_the_run_and_reports_not_saved() {
    let dir = TempDir::new().unwrap();
    let rules = ["!exact echo secret"];
    let handler = Scripted::new(ShellApprovalDecision::AllowExactPersist);
    let options = doc_options(dir.path(), &rules).with_shell_approval_handler(handler.clone());
    let before = std::fs::read(dir.path().join(".darkmatter.yaml")).unwrap();
    let (composed, report) = compose(dir.path(), "::shell echo secret\n\n::shell echo secret\n", options).unwrap();
    assert_eq!(composed.content().matches("secret").count(), 2);
    assert_eq!(handler.requests().len(), 1, "the run's approval covers the repeat");
    let not_saved: Vec<_> =
        report.warnings.iter().filter(|warning| warning.code.as_deref() == Some("dm.config.not_saved")).collect();
    assert_eq!(not_saved.len(), 1, "{:?}", report.warnings);
    assert!(not_saved[0].message.contains("!exact echo secret"), "{}", not_saved[0].message);
    assert_eq!(std::fs::read(dir.path().join(".darkmatter.yaml")).unwrap(), before, "the exclusion stays");
}

#[test]
fn without_a_home_an_always_allow_approves_the_run_and_reports_not_saved() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("doc.md");
    std::fs::write(&path, "::shell echo homeless\n\n::shell echo homeless\n").unwrap();
    let handler = Scripted::new(ShellApprovalDecision::AllowExactPersist);
    let options = ComposeOptions::new_with_context(ComposeContext::capture_for_content(dir.path(), ""))
        .only(&[ComposeOperation::ShellExpansion])
        .with_source_file(&path)
        .with_shell_working_directory(dir.path())
        .with_shell_approval_handler(handler.clone());
    let request = ComposeRequest::prepare(options, &RequestSnapshot::new(dir.path()).with_home(None)).unwrap();
    let (composed, report) = Markdown::try_from(path.as_path()).unwrap().compose_with(&request).unwrap();
    assert_eq!(composed.content().matches("homeless").count(), 2);
    assert_eq!(handler.requests().len(), 1);
    assert!(handler.requests()[0].save_target.is_none());
    assert!(report.warnings.iter().any(|warning| warning.code.as_deref() == Some("dm.config.not_saved")), "{:?}", report.warnings);
    assert!(!dir.path().join(".darkmatter.yaml").exists());
}

// ── An invalid block runs nothing ───────────────────────────────────────

#[test]
fn an_invalid_permissions_block_fails_every_command_with_the_config_error() {
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join(".darkmatter.yaml"), "permissions:\n  shell-commands:\n    - \"prefix\"\n").unwrap();
    let handler = Scripted::new(ShellApprovalDecision::AllowOnce);
    let options = doc_options(dir.path(), &[]).with_shell_approval_handler(handler.clone());
    let error = compose(dir.path(), "::shell echo anything\n", options.clone()).unwrap_err();
    assert!(matches!(shell_error(&error), ShellExpansionError::Config(_)), "{error:?}");
    assert!(handler.requests().is_empty(), "an invalid policy asks nothing");

    let path = dir.path().join("doc.md");
    let preflight = Markdown::try_from(path.as_path())
        .unwrap()
        .compose_preflight_approvals(&crate::request_support::request(options), Some(handler.clone()))
        .unwrap_err();
    assert!(matches!(preflight, ShellExpansionError::Config(_)), "{preflight:?}");
    let codes: Vec<String> = darkmatter::markdown::compose::report_diagnostics::error_diagnostics(&preflight)
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect();
    assert_eq!(codes, ["dm.config.invalid"], "the located configuration problem is reported");
}
