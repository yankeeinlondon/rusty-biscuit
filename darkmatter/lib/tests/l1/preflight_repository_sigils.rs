//! Pre-flight resolves `&` and `^` against the request's repository.
//!
//! Pre-flight runs before the compose pipeline, so it must prepare its options
//! the way the pipeline does. Before it did, its transclusion resolver had no
//! repository root and every `&`/`^` target failed with "requires a
//! repository containing reference CWD", which stopped `md compose` before the
//! pipeline was reached.
//!
//! Every fixture is a git repository holding two targets at its root and a
//! document two directories down that transcludes them, one through each
//! sigil, plus a `^` path into a sibling directory (the shape that failed in
//! the original report). Each target runs a distinct `::shell` command, so a
//! command in the approval set proves pre-flight reached that target.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::preflight::PreflightResolvedTarget;
use darkmatter::markdown::compose::shell_expansion::types::{
    ShellApprovalDecision, ShellApprovalHandler, ShellApprovalRequest,
};
use darkmatter::markdown::compose::{
    ComposeOptions, ShellExpansionError, collect_shell_commands,
};
use tempfile::TempDir;

const DOCUMENT: &str = "\
# Guide

::file &amp-target.md

::file ^caret-target.md

::file ^pkg/docs/cli/index.md
";

const EXPECTED_COMMANDS: [&str; 3] = ["echo from-amp", "echo from-caret", "echo from-cli-index"];

struct Fixture {
    _dir: TempDir,
    repo: PathBuf,
    launch_dir: PathBuf,
    document: PathBuf,
}

fn fixture() -> Fixture {
    let dir = TempDir::new().expect("temp dir");
    let repo = dir.path().join("repo");
    std::fs::create_dir_all(&repo).expect("repo dir");
    gix::init(&repo).expect("initialize repository");
    let files = [
        ("amp-target.md", "::shell echo from-amp\n"),
        ("caret-target.md", "::shell echo from-caret\n"),
        ("pkg/docs/cli/index.md", "::shell echo from-cli-index\n"),
        ("pkg/docs/guide/doc.md", DOCUMENT),
    ];
    for (relative, content) in files {
        let path = repo.join(relative);
        std::fs::create_dir_all(path.parent().unwrap()).expect("fixture parent");
        std::fs::write(&path, content).expect("fixture file");
    }
    Fixture {
        launch_dir: repo.join("pkg"),
        document: repo.join("pkg/docs/guide/doc.md"),
        repo,
        _dir: dir,
    }
}

fn markdown(fixture: &Fixture) -> Markdown {
    Markdown::try_from(fixture.document.as_path()).expect("read fixture document")
}

/// The two request shapes a caller builds with no file-resolution context:
/// `md compose`'s request anchored at its launch directory, and the bare
/// constructor, whose request repository is the test process's own CWD.
fn option_rows(fixture: &Fixture) -> Vec<(&'static str, ComposeOptions)> {
    let md = markdown(fixture);
    vec![
        (
            "for_document at the launch directory",
            ComposeOptions::for_document(&fixture.launch_dir, &md).with_source_file(&fixture.document),
        ),
        ("new", ComposeOptions::new().with_source_file(&fixture.document)),
    ]
}

fn canonical(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).expect("canonicalize fixture path")
}

#[test]
fn compose_preflight_resolves_repository_sigils_from_a_nested_document() {
    let fixture = fixture();
    let md = markdown(&fixture);
    for (row, options) in option_rows(&fixture) {
        let report = md
            .compose_preflight(&crate::request_support::request(options.clone()))
            .unwrap_or_else(|error| panic!("{row}: preflight failed: {error}"));

        let approval: HashSet<String> = report.approval_set().into_iter().collect();
        for command in EXPECTED_COMMANDS {
            assert!(approval.contains(command), "{row}: missing {command} in {approval:?}");
        }

        let resolved: Vec<PathBuf> = report
            .preflight_graph
            .edges
            .iter()
            .map(|edge| match &edge.resolved_target {
                PreflightResolvedTarget::File { path, .. } => canonical(path),
                PreflightResolvedTarget::Url(url) => panic!("{row}: unexpected URL target {url}"),
            })
            .collect();
        let expected: Vec<PathBuf> = ["amp-target.md", "caret-target.md", "pkg/docs/cli/index.md"]
            .iter()
            .map(|relative| canonical(&fixture.repo.join(relative)))
            .collect();
        assert_eq!(resolved, expected, "{row}: resolved targets");
    }
}

#[test]
fn collect_shell_commands_resolves_repository_sigils_from_a_nested_document() {
    let fixture = fixture();
    let md = markdown(&fixture);
    for (row, options) in option_rows(&fixture) {
        let entries = collect_shell_commands(&md, &crate::request_support::request(options.clone()))
            .unwrap_or_else(|error| panic!("{row}: collection failed: {error}"));
        let collected: Vec<&str> = entries.iter().map(|entry| entry.normalized.as_str()).collect();
        assert_eq!(collected, EXPECTED_COMMANDS, "{row}");
    }
}

struct AllowAll;

impl ShellApprovalHandler for AllowAll {
    fn approve(&self, _request: ShellApprovalRequest) -> Result<ShellApprovalDecision, ShellExpansionError> {
        Ok(ShellApprovalDecision::AllowOnce)
    }
}

#[test]
fn compose_preflight_approvals_resolves_repository_sigils_from_a_nested_document() {
    let fixture = fixture();
    let md = markdown(&fixture);
    let policy = TempDir::new().expect("policy dir");
    for (row, options) in option_rows(&fixture) {
        let options = options.with_shell_policy_root(policy.path());
        let approvals = md
            .compose_preflight_approvals(&crate::request_support::request(options.clone()), Some(Arc::new(AllowAll)))
            .unwrap_or_else(|error| panic!("{row}: approvals failed: {error}"));
        let expected: HashSet<String> = EXPECTED_COMMANDS.iter().map(ToString::to_string).collect();
        assert_eq!(approvals.pre_approved_commands, expected, "{row}");
        assert_eq!(approvals.stats.total_discovered, 3, "{row}");
    }
}

/// A target missing from the repository root still fails, and names the
/// repository-root reference rather than the missing-repository precondition.
#[test]
fn compose_preflight_reports_a_missing_repository_target_as_not_found() {
    let fixture = fixture();
    std::fs::remove_file(fixture.repo.join("amp-target.md")).expect("remove target");
    let md = markdown(&fixture);
    for (row, options) in option_rows(&fixture) {
        let error = md
            .compose_preflight(&crate::request_support::request(options.clone()))
            .expect_err("a missing `&` target must fail pre-flight")
            .to_string();
        assert!(
            !error.contains("requires a repository containing reference CWD"),
            "{row}: pre-flight still lacks the repository: {error}"
        );
        assert!(error.contains("amp-target.md"), "{row}: error must name the target: {error}");
    }
}
