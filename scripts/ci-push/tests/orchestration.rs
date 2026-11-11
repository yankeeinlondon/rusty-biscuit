//! `ci_push::run` end to end at its public boundary, with every session
//! boundary faked: the review ledger, the order of reviews, gates, and
//! publication, and every rule that blocks a push or withholds evidence.
//!
//! The fake answers the Python helpers the way the real ones do on success
//! (the planner writes a resolved plan, `--apply-to` writes the reviewed
//! plan, `record-cells` prints a receipt), and each failure is injected one
//! step at a time.

use std::cell::Cell;
use std::collections::HashMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::ExitStatus;

use chrono::NaiveDate;
use gix::ObjectId;
use rstest::rstest;

use crate::ci_push::git::{GitError, HeadIdentity, RemoteTipError};
use crate::ci_push::process::{CancelSignal, Completed, ProcessError, Termination};
use crate::ci_push::python::PythonSelectionError;
use crate::ci_push::worktree::{CleanupFailure, CreateError};
use crate::ci_push::{
    BlockReason, ContextSource, Diagnostic, PushMode, PushOutcome, PushRequest, PushServices,
    PushSession, ReviewStatus, ScopePublication, TreeSource, Validation, WithholdReason, run,
};

const MACOS_EXECUTING: &str = include_str!("../../ci/fixtures/pre-push/plans/plan-macos-executing.json");
const WSL_EXECUTING: &str = include_str!("../../ci/fixtures/pre-push/plans/plan-wsl-executing.json");
const WSL_REUSED: &str = include_str!("../../ci/fixtures/pre-push/plans/plan-wsl-reused.json");
const WSL_ABSENT: &str = include_str!("../../ci/fixtures/pre-push/plans/plan-wsl-absent.json");
const TWO_OPEN: &str = include_str!("../../ci/fixtures/pre-push/pull-requests/two-open.json");
const NONE_OPEN: &str = include_str!("../../ci/fixtures/pre-push/pull-requests/none.json");

const HEAD: &str = "1111111111111111111111111111111111111111";
const OTHER: &str = "2222222222222222222222222222222222222222";
const MAIN_TIP: &str = "3333333333333333333333333333333333333333";
const BASE_TIP: &str = "4444444444444444444444444444444444444444";
const ZERO: &str = "0000000000000000000000000000000000000000";
const GITHUB: &str = "git@github.com:owner/repo.git";
const ELSEWHERE: &str = "https://example.com/owner/repo.git";
const ARGUMENTS: &[u8] = b"--deleted\0gone.rs\0--\0kept.rs\0";

fn id(hex: &str) -> ObjectId {
    ObjectId::from_hex(hex.as_bytes()).unwrap()
}

fn status(code: i32) -> ExitStatus {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        ExitStatus::from_raw(code << 8)
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::ExitStatusExt;
        ExitStatus::from_raw(code as u32)
    }
}

fn exited(code: i32, stdout: impl Into<Vec<u8>>, stderr: &str) -> Completed {
    Completed {
        termination: Termination::Exited(status(code)),
        stdout: stdout.into(),
        stderr: stderr.as_bytes().to_vec(),
        incomplete: false,
    }
}

/// Every boundary the session crossed, in order.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Call {
    SelectPython,
    RemoteTip(String),
    EnsureCommit(ObjectId, String),
    ArgumentsFile(ObjectId, ObjectId),
    PullRequests(String, String),
    Materialize(ObjectId),
    Release,
    /// The helper's script file name, its subcommand or first flag, and
    /// every argument.
    Python { script: String, cwd: PathBuf, args: Vec<String> },
    Note(ObjectId),
    Gates { selection: Vec<String>, variables: HashMap<String, String> },
}

/// What `gh pr list` answers.
enum Gh {
    Missing,
    Fails(&'static str),
    Lists(&'static str),
}

/// What `just pre-push` does.
#[derive(Clone)]
struct Gates {
    code: i32,
    manifest: bool,
    plan: bool,
    marker: bool,
    signal: Option<CancelSignal>,
    head_after: Option<ObjectId>,
    dirty_after: bool,
    report: Option<(&'static str, &'static str)>,
}

impl Default for Gates {
    fn default() -> Self {
        Gates {
            code: 0,
            manifest: true,
            plan: true,
            marker: false,
            signal: None,
            head_after: None,
            dirty_after: false,
            report: Some(("alpha-L1.xml", "<testsuites/>")),
        }
    }
}

struct Fake {
    _temp: tempfile::TempDir,
    root: PathBuf,
    home: PathBuf,
    url: Option<String>,
    head: ObjectId,
    dirty: bool,
    python_missing: bool,
    tips: HashMap<String, ObjectId>,
    /// Commits `ensure_commit` cannot find.
    missing: Vec<ObjectId>,
    gh: Gh,
    /// The planner's plan per reviewed revision; MACOS_EXECUTING otherwise.
    plans: HashMap<ObjectId, &'static str>,
    /// `(step, nth occurrence from 1, exit code)`.
    fail: Option<(&'static str, usize, i32)>,
    /// A signal arrives while this step runs.
    signal_during: Option<(&'static str, CancelSignal)>,
    unreadable_review: bool,
    gates: Gates,
    environment: Option<&'static str>,
    variables: HashMap<String, OsString>,
    note_fails: bool,
    cancelled: Cell<Option<CancelSignal>>,
    /// The `--arguments-file` contents each planner run was handed.
    arguments_seen: Vec<Vec<u8>>,
    counts: HashMap<&'static str, usize>,
    worktrees: usize,
    calls: Vec<Call>,
    emitted: Vec<String>,
}

impl Fake {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("checkout");
        let home = temp.path().join("home");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::create_dir_all(&home).unwrap();
        let mut tips = HashMap::new();
        tips.insert("main".to_owned(), id(MAIN_TIP));
        let mut variables = HashMap::new();
        variables.insert(
            "BISCUIT_CI_CONSTRAINTS_DIR".to_owned(),
            temp.path().join("constraints").into_os_string(),
        );
        Fake {
            root,
            home,
            _temp: temp,
            url: Some(ELSEWHERE.to_owned()),
            head: id(HEAD),
            dirty: false,
            python_missing: false,
            tips,
            missing: Vec::new(),
            gh: Gh::Lists(NONE_OPEN),
            plans: HashMap::new(),
            fail: None,
            signal_during: None,
            unreadable_review: false,
            gates: Gates::default(),
            environment: Some("macos-latest"),
            variables,
            note_fails: false,
            cancelled: Cell::new(None),
            arguments_seen: Vec::new(),
            counts: HashMap::new(),
            worktrees: 0,
            calls: Vec::new(),
            emitted: Vec::new(),
        }
    }

    fn constraint(&self, name: &str, record: &str) {
        let directory = PathBuf::from(&self.variables["BISCUIT_CI_CONSTRAINTS_DIR"]);
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(directory.join(name), record).unwrap();
    }

    fn evidence_root(&self) -> PathBuf {
        self.home.join(".rusty-biscuit").join("ci-evidence")
    }

    /// The helper calls, as `script subcommand` labels.
    fn helpers(&self) -> Vec<String> {
        self.calls
            .iter()
            .filter_map(|call| match call {
                Call::Python { script, args, .. } => Some(format!("{script} {}", args[1])),
                _ => None,
            })
            .collect()
    }

    fn gate_calls(&self) -> Vec<&Call> {
        self.calls.iter().filter(|call| matches!(call, Call::Gates { .. })).collect()
    }

    fn notes(&self) -> usize {
        self.calls.iter().filter(|call| matches!(call, Call::Note(_))).count()
    }

    fn python_args(&self, label: &str) -> Vec<Vec<String>> {
        self.calls
            .iter()
            .filter_map(|call| match call {
                Call::Python { script, args, .. } if format!("{script} {}", args[1]) == label => {
                    Some(args.clone())
                }
                _ => None,
            })
            .collect()
    }

    fn step(&mut self, step: &'static str) -> Option<i32> {
        let count = self.counts.entry(step).or_insert(0);
        *count += 1;
        if let Some((name, signal)) = self.signal_during {
            if name == step {
                self.cancelled.set(Some(signal));
            }
        }
        match self.fail {
            Some((name, nth, code)) if name == step && nth == *count => Some(code),
            _ => None,
        }
    }
}

fn value<'a>(args: &'a [String], flag: &str) -> &'a str {
    let at = args.iter().position(|arg| arg == flag).unwrap_or_else(|| panic!("no {flag} in {args:?}"));
    &args[at + 1]
}

impl PushSession for Fake {
    fn root(&self) -> &Path {
        &self.root
    }

    fn remote_url(&self) -> Option<&str> {
        self.url.as_deref()
    }

    fn head(&mut self) -> Result<HeadIdentity, GitError> {
        Ok(HeadIdentity {
            commit: self.head,
            branch: Some("refs/heads/feature".to_owned()),
        })
    }

    fn is_dirty(&mut self) -> Result<bool, GitError> {
        Ok(self.dirty)
    }

    fn select_python(&mut self) -> Result<(), PythonSelectionError> {
        self.calls.push(Call::SelectPython);
        if self.python_missing {
            Err(PythonSelectionError::NoInterpreter(Vec::new()))
        } else {
            Ok(())
        }
    }

    fn remote_tip(&mut self, branch: &str) -> Result<ObjectId, RemoteTipError> {
        self.calls.push(Call::RemoteTip(branch.to_owned()));
        self.tips.get(branch).copied().ok_or_else(|| RemoteTipError::Unreadable {
            branch: branch.to_owned(),
            remote: "origin".to_owned(),
            url: self.url.clone().unwrap_or_default(),
        })
    }

    fn ensure_commit(&mut self, commit: ObjectId, branch: &str) -> Result<bool, RemoteTipError> {
        self.calls.push(Call::EnsureCommit(commit, branch.to_owned()));
        Ok(!self.missing.contains(&commit))
    }

    fn arguments_file(&mut self, base: ObjectId, head: ObjectId) -> Result<Vec<u8>, GitError> {
        self.calls.push(Call::ArgumentsFile(base, head));
        Ok(ARGUMENTS.to_vec())
    }

    fn list_pull_requests(&mut self, repository: &str, branch: &str) -> Result<Completed, ProcessError> {
        self.calls.push(Call::PullRequests(repository.to_owned(), branch.to_owned()));
        match self.gh {
            Gh::Missing => Err(ProcessError {
                program: "gh".into(),
                source: std::io::ErrorKind::NotFound.into(),
            }),
            Gh::Fails(stderr) => Ok(exited(1, Vec::new(), stderr)),
            Gh::Lists(json) => Ok(exited(0, json, "")),
        }
    }

    fn materialize(&mut self, commit: ObjectId) -> Result<PathBuf, CreateError> {
        self.calls.push(Call::Materialize(commit));
        self.worktrees += 1;
        let path = self.root.parent().unwrap().join(format!("tree-{}", self.worktrees));
        std::fs::create_dir_all(&path).unwrap();
        Ok(path)
    }

    fn release_tree(&mut self) -> Result<(), CleanupFailure> {
        self.calls.push(Call::Release);
        Ok(())
    }

    fn python(&mut self, cwd: &Path, args: &[OsString]) -> Result<Completed, ProcessError> {
        let args: Vec<String> = args.iter().map(|arg| arg.to_string_lossy().into_owned()).collect();
        let script = Path::new(&args[0]).file_name().unwrap().to_string_lossy().into_owned();
        self.calls.push(Call::Python {
            script: script.clone(),
            cwd: cwd.to_owned(),
            args: args.clone(),
        });
        let step = match (script.as_str(), args[1].as_str()) {
            ("affected_scope.py", "--event") => "plan",
            ("affected_scope.py", "--apply-to") => "apply",
            ("local_evidence.py", "verify") => "verify",
            ("local_evidence.py", "scope-record") => "scope-record",
            ("local_evidence.py", "record-cells") => "record-cells",
            ("observation_notes.py", "publish-receipt") => "publish-receipt",
            other => panic!("unexpected helper {other:?}"),
        };
        if let Some(code) = self.step(step) {
            return Ok(exited(code, Vec::new(), &format!("{step} refused")));
        }
        if let Some(signal) = self.cancelled.get() {
            return Ok(Completed {
                termination: Termination::Cancelled(signal),
                stdout: Vec::new(),
                stderr: Vec::new(),
                incomplete: false,
            });
        }
        let stdout: Vec<u8> = match step {
            "plan" => {
                if args.iter().any(|arg| arg == "--arguments-file") {
                    self.arguments_seen.push(std::fs::read(value(&args, "--arguments-file")).unwrap());
                }
                let head = id(value(&args, "--head"));
                let plan = self.plans.get(&head).copied().unwrap_or(MACOS_EXECUTING);
                std::fs::write(value(&args, "--plan-out"), plan).unwrap();
                b"{\"scope\": \"doc\"}\n".to_vec()
            }
            "verify" => {
                std::fs::write(value(&args, "--rejections"), "[]").unwrap();
                b"[]\n".to_vec()
            }
            "apply" => {
                let review = if self.unreadable_review {
                    format!("{{\"base\": \"{MAIN_TIP}\", \"head\": \"{HEAD}\", \"cells\": 7}}")
                } else {
                    std::fs::read_to_string(value(&args, "--apply-to")).unwrap()
                };
                std::fs::write(value(&args, "--plan-out"), review).unwrap();
                Vec::new()
            }
            "scope-record" => b"{\"receipt\": \"scope\"}\n".to_vec(),
            "record-cells" => b"{\"cells\": [{}, {}]}\n".to_vec(),
            _ => Vec::new(),
        };
        Ok(exited(0, stdout, ""))
    }

    fn render_plan(&mut self, plan: &[u8]) -> anyhow::Result<String> {
        Ok(format!("(rendered {} bytes)", plan.len()))
    }

    fn publish_scope_note(&mut self, _receipt: &[u8], head: ObjectId) -> Result<(), String> {
        self.calls.push(Call::Note(head));
        if self.note_fails {
            Err("the remote refused the notes ref".to_owned())
        } else {
            Ok(())
        }
    }

    fn run_gates(
        &mut self,
        selection: &[String],
        variables: &[(OsString, OsString)],
        staging: &Path,
    ) -> Result<Completed, ProcessError> {
        let variables: HashMap<String, String> = variables
            .iter()
            .map(|(name, value)| (name.to_string_lossy().into_owned(), value.to_string_lossy().into_owned()))
            .collect();
        self.calls.push(Call::Gates {
            selection: selection.to_vec(),
            variables: variables.clone(),
        });
        assert!(staging.is_dir());
        let gates = self.gates.clone();
        let reports = PathBuf::from(&variables["BISCUIT_CI_REPORTS_OUT"]);
        if let Some((name, body)) = gates.report {
            std::fs::write(reports.join(name), body).unwrap();
        }
        if gates.manifest {
            std::fs::write(reports.join("manifest.jsonl"), "{\"cell\": \"alpha\"}\n").unwrap();
        }
        if gates.plan {
            std::fs::write(&variables["BISCUIT_CI_PLAN_OUT"], MACOS_EXECUTING).unwrap();
        }
        if gates.marker {
            std::fs::write(reports.join("interrupted"), "").unwrap();
        }
        if let Some(head) = gates.head_after {
            self.head = head;
        }
        if gates.dirty_after {
            self.dirty = true;
        }
        if let Some(signal) = gates.signal {
            self.cancelled.set(Some(signal));
            return Ok(Completed {
                termination: Termination::Cancelled(signal),
                stdout: Vec::new(),
                stderr: Vec::new(),
                incomplete: false,
            });
        }
        Ok(exited(gates.code, Vec::new(), ""))
    }

    fn environment(&mut self) -> Option<String> {
        self.environment.map(str::to_owned)
    }

    fn home(&self) -> Option<PathBuf> {
        Some(self.home.clone())
    }

    fn variable(&self, name: &str) -> Option<OsString> {
        self.variables.get(name).cloned()
    }

    fn today(&self) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 11, 10).unwrap()
    }

    fn cancelled(&self) -> Option<CancelSignal> {
        self.cancelled.get()
    }

    fn emit(&mut self, diagnostic: &Diagnostic) {
        self.emitted.push(diagnostic.to_string());
    }
}

struct Services(Fake);

impl PushServices for Services {
    fn object_hash(&mut self, _cwd: &Path) -> anyhow::Result<gix::hash::Kind> {
        Ok(gix::hash::Kind::Sha1)
    }

    fn emit(&mut self, diagnostic: &Diagnostic) {
        self.0.emitted.push(diagnostic.to_string());
    }

    fn session(
        &mut self,
        _request: &PushRequest,
        body: &mut dyn FnMut(&mut dyn PushSession) -> anyhow::Result<()>,
    ) -> anyhow::Result<()> {
        body(&mut self.0)
    }
}

fn push(fake: Fake, mode: &str, selection: Option<&str>, input: &str) -> (PushOutcome, Fake) {
    let request = PushRequest {
        cwd: fake.root.clone(),
        remote_name: "origin".to_owned(),
        remote_url: fake.url.clone(),
        input: input.as_bytes().to_vec(),
        mode: Some(mode.into()),
        selection: selection.map(OsString::from),
    };
    let mut services = Services(fake);
    let outcome = run(request, &mut services).expect("no boundary failed");
    (outcome, services.0)
}

fn line(local_ref: &str, local: &str, remote_ref: &str, remote: &str) -> String {
    format!("{local_ref} {local} {remote_ref} {remote}\n")
}

fn feature(local: &str) -> String {
    line("refs/heads/feature", local, "refs/heads/feature", ZERO)
}

fn main_push(remote: &str) -> String {
    line("refs/heads/main", HEAD, "refs/heads/main", remote)
}

fn blocked(outcome: &PushOutcome) -> &BlockReason {
    outcome
        .diagnostics
        .iter()
        .find_map(|diagnostic| match diagnostic {
            Diagnostic::Blocked(reason) => Some(reason),
            _ => None,
        })
        .unwrap_or_else(|| panic!("not blocked: {:#?}", outcome.diagnostics))
}

fn problem(outcome: &PushOutcome) -> String {
    match blocked(outcome) {
        BlockReason::Review { problem, .. } => problem.clone(),
        other => panic!("not a review failure: {other:?}"),
    }
}

fn record(environment: &str, extra: &str) -> String {
    format!(
        "{{\"environment\": \"{environment}\", \"reason\": \"no {environment} here\", \"owner\": \"ken\", \"expiry\": \"2099-01-01\"{extra}}}"
    )
}

// ---------------------------------------------------------------- review --

#[test]
fn updates_are_reviewed_in_git_order_and_skipped_updates_are_not() {
    let input = [
        line("refs/tags/v1", OTHER, "refs/tags/v1", ZERO),
        feature(HEAD),
        line("(delete)", ZERO, "refs/heads/old", OTHER),
        line("refs/heads/second", OTHER, "refs/heads/second", ZERO),
    ]
    .concat();
    let (outcome, fake) = push(Fake::new(), "scope-only", None, &input);
    assert_eq!(outcome.exit_code, 0, "{:#?}", outcome.diagnostics);
    let ledger: Vec<(usize, ObjectId, ContextSource)> = outcome
        .reviews
        .iter()
        .map(|review| (review.context.update, review.trigger, review.context.source.clone()))
        .collect();
    assert_eq!(
        ledger,
        [
            (1, id(HEAD), ContextSource::Provisional { incoming: false }),
            (3, id(OTHER), ContextSource::Provisional { incoming: false }),
        ]
    );
    assert!(outcome.reviews.iter().all(|review| review.status == ReviewStatus::Passed));
    assert_eq!(fake.helpers().iter().filter(|label| label.ends_with("--event")).count(), 2);
    // Each update is planned against the remote's main with its own path set.
    assert_eq!(
        fake.calls.iter().filter(|call| matches!(call, Call::ArgumentsFile(..))).cloned().collect::<Vec<_>>(),
        [Call::ArgumentsFile(id(MAIN_TIP), id(HEAD)), Call::ArgumentsFile(id(MAIN_TIP), id(OTHER))]
    );
}

#[test]
fn a_push_to_main_is_compared_with_the_remote_previous_tip_and_starts_a_run() {
    let (outcome, fake) = push(Fake::new(), "auto", None, &main_push(MAIN_TIP));
    assert_eq!(outcome.exit_code, 0, "{:#?}", outcome.diagnostics);
    let review = &outcome.reviews[0];
    assert_eq!(review.context.source, ContextSource::MainPush);
    assert_eq!(review.context.base, id(MAIN_TIP));
    assert!(fake.calls.contains(&Call::EnsureCommit(id(MAIN_TIP), "main".to_owned())));
    let planner = &fake.python_args("affected_scope.py --event")[0];
    assert_eq!(value(planner, "--event"), "push");
    assert_eq!(value(planner, "--base"), MAIN_TIP);
    // Auto resolved to strict because the push starts a run.
    assert_eq!(outcome.gates.as_ref().unwrap().mode, PushMode::Strict);
}

#[test]
fn a_remote_main_tip_that_cannot_be_fetched_blocks() {
    let mut fake = Fake::new();
    fake.missing.push(id(MAIN_TIP));
    let (outcome, fake) = push(fake, "auto", None, &main_push(MAIN_TIP));
    assert_eq!(outcome.exit_code, 2);
    assert!(problem(&outcome).contains("not available locally even after fetching origin"));
    assert!(fake.helpers().is_empty());
}

#[test]
fn a_branch_creating_push_of_main_plans_full_scope_and_its_gates_replan() {
    let (outcome, fake) = push(Fake::new(), "strict", None, &main_push(ZERO));
    assert_eq!(outcome.exit_code, 0, "{:#?}", outcome.diagnostics);
    let planner = &fake.python_args("affected_scope.py --event")[0];
    assert!(planner.contains(&"--all".to_owned()));
    assert!(!planner.contains(&"--arguments-file".to_owned()));
    assert!(!fake.calls.iter().any(|call| matches!(call, Call::ArgumentsFile(..))));
    // No receipt binds an all-zero base, so nothing is recorded or
    // published, and the gates replan rather than consume the plan.
    assert!(fake.python_args("local_evidence.py scope-record").is_empty());
    assert_eq!(outcome.scope, ScopePublication::NotRecorded);
    assert_eq!(outcome.gates.as_ref().unwrap().plan_in, None);
    assert_eq!(
        outcome.validation,
        Some(Validation::Withheld(WithholdReason::NoReviewedComparison))
    );
}

#[test]
fn every_open_pull_request_is_reviewed_in_gh_order() {
    let mut fake = Fake::new();
    fake.url = Some(GITHUB.to_owned());
    fake.gh = Gh::Lists(TWO_OPEN);
    let bases: HashMap<String, ObjectId> = parse_two_open().into_iter().collect();
    for (branch, tip) in &bases {
        fake.tips.insert(branch.clone(), *tip);
    }
    let (outcome, fake) = push(fake, "auto", None, &feature(HEAD));
    assert_eq!(outcome.exit_code, 0, "{:#?}", outcome.diagnostics);
    assert!(fake.calls.contains(&Call::PullRequests("owner/repo".to_owned(), "feature".to_owned())));
    let sources: Vec<ContextSource> = outcome.reviews.iter().map(|review| review.context.source.clone()).collect();
    let expected: Vec<ContextSource> = parse_two_open_numbers()
        .into_iter()
        .map(|(number, base_branch)| ContextSource::PullRequest {
            number,
            base_branch,
            incoming: false,
        })
        .collect();
    assert_eq!(sources, expected);
    // An open pull request starts a run, so auto gates strictly.
    assert_eq!(outcome.gates.as_ref().unwrap().mode, PushMode::Strict);
    // One note binds HEAD's first context only.
    let recorded = fake.python_args("local_evidence.py scope-record");
    assert_eq!(recorded.len(), 1);
    assert_eq!(value(&recorded[0], "--base"), outcome.reviews[0].context.base.to_string());
}

/// The fixture's `(number, base)` pairs, with a distinct tip per base.
fn parse_two_open_numbers() -> Vec<(u64, String)> {
    crate::ci_push::parse_pull_requests(TWO_OPEN.as_bytes(), "two-open.json")
        .unwrap()
        .into_iter()
        .map(|pull_request| (pull_request.number, pull_request.base_branch))
        .collect()
}

fn parse_two_open() -> Vec<(String, ObjectId)> {
    parse_two_open_numbers()
        .into_iter()
        .enumerate()
        .map(|(index, (_, base))| (base, id(&format!("{:x}", index + 5).repeat(40))))
        .collect()
}

#[test]
fn a_base_this_push_also_updates_is_reviewed_at_both_states() {
    let mut fake = Fake::new();
    fake.url = Some(GITHUB.to_owned());
    fake.gh = Gh::Lists(r#"[{"number": 7, "baseRefName": "develop"}]"#);
    fake.tips.insert("develop".to_owned(), id(BASE_TIP));
    let input = [feature(HEAD), line("refs/heads/develop", OTHER, "refs/heads/develop", BASE_TIP)].concat();
    let (outcome, _) = push(fake, "scope-only", None, &input);
    assert_eq!(outcome.exit_code, 0, "{:#?}", outcome.diagnostics);
    let first: Vec<(ObjectId, ContextSource)> = outcome
        .reviews
        .iter()
        .filter(|review| review.context.update == 0)
        .map(|review| (review.context.base, review.context.source.clone()))
        .collect();
    let pull_request = |incoming| ContextSource::PullRequest {
        number: 7,
        base_branch: "develop".to_owned(),
        incoming,
    };
    assert_eq!(first, [(id(BASE_TIP), pull_request(false)), (id(OTHER), pull_request(true))]);
}

#[test]
fn a_base_created_by_the_same_push_has_only_its_incoming_state() {
    let mut fake = Fake::new();
    fake.url = Some(GITHUB.to_owned());
    fake.gh = Gh::Lists(r#"[{"number": 7, "baseRefName": "develop"}]"#);
    let input = [feature(HEAD), line("refs/heads/develop", OTHER, "refs/heads/develop", ZERO)].concat();
    let (outcome, _) = push(fake, "scope-only", None, &input);
    assert_eq!(outcome.exit_code, 0, "{:#?}", outcome.diagnostics);
    assert_eq!(outcome.reviews[0].context.base, id(OTHER));
    assert_eq!(
        outcome.reviews[0].context.source,
        ContextSource::PullRequest {
            number: 7,
            base_branch: "develop".to_owned(),
            incoming: true
        }
    );
    assert_eq!(outcome.reviews.iter().filter(|review| review.context.update == 0).count(), 1);
}

#[test]
fn a_push_deleting_an_open_pull_requests_base_is_refused() {
    let mut fake = Fake::new();
    fake.url = Some(GITHUB.to_owned());
    fake.gh = Gh::Lists(r#"[{"number": 7, "baseRefName": "develop"}]"#);
    fake.tips.insert("develop".to_owned(), id(BASE_TIP));
    let input = [feature(HEAD), line("(delete)", ZERO, "refs/heads/develop", BASE_TIP)].concat();
    let (outcome, fake) = push(fake, "warn", None, &input);
    assert_eq!(outcome.exit_code, 2);
    let text = problem(&outcome);
    assert!(text.contains("pull request #7 into develop"), "{text}");
    assert!(text.contains("this push also deletes develop"), "{text}");
    assert!(fake.helpers().is_empty() && fake.gate_calls().is_empty() && fake.notes() == 0);
}

#[rstest]
#[case::gh_missing(Gh::Missing, "`gh` is not installed")]
#[case::gh_fails(Gh::Fails("HTTP 401: Bad credentials\n(gh auth login)"), "failed with: HTTP 401: Bad credentials (gh auth login)")]
#[case::gh_unreadable(Gh::Lists(r#"{"number": 7}"#), "returned something ci-push could not read")]
#[case::gh_bad_number(Gh::Lists(r#"[{"number": 0, "baseRefName": "main"}]"#), "returned something ci-push could not read")]
fn a_pull_request_lookup_that_fails_blocks_every_mode(#[case] gh: Gh, #[case] expected: &str) {
    for mode in ["scope-only", "warn", "auto"] {
        let mut fake = Fake::new();
        fake.url = Some(GITHUB.to_owned());
        fake.gh = match &gh {
            Gh::Missing => Gh::Missing,
            Gh::Fails(text) => Gh::Fails(text),
            Gh::Lists(text) => Gh::Lists(text),
        };
        let (outcome, fake) = push(fake, mode, None, &feature(HEAD));
        assert_eq!(outcome.exit_code, 2, "{mode}");
        assert!(problem(&outcome).contains(expected), "{mode}: {}", problem(&outcome));
        assert!(fake.helpers().is_empty() && fake.gate_calls().is_empty() && fake.notes() == 0);
    }
}

#[test]
fn an_unrecognized_github_url_fails_closed() {
    let mut fake = Fake::new();
    fake.url = Some("https://github.com/owner".to_owned());
    let (outcome, fake) = push(fake, "scope-only", None, &feature(HEAD));
    assert_eq!(outcome.exit_code, 2);
    assert!(problem(&outcome).contains("names github.com in a form this hook does not read"));
    assert!(!fake.calls.iter().any(|call| matches!(call, Call::PullRequests(..))));
}

#[test]
fn a_non_github_remote_never_asks_gh_and_gets_a_provisional_plan() {
    let (outcome, fake) = push(Fake::new(), "auto", None, &feature(HEAD));
    assert_eq!(outcome.exit_code, 0);
    assert!(!fake.calls.iter().any(|call| matches!(call, Call::PullRequests(..))));
    assert_eq!(outcome.reviews[0].context.source, ContextSource::Provisional { incoming: false });
    assert_eq!(outcome.reviews[0].context.base, id(MAIN_TIP));
    // A provisional plan starts no run: auto runs no gate.
    assert!(outcome.diagnostics.contains(&Diagnostic::NoGateRuns));
    assert!(fake.gate_calls().is_empty());
}

#[test]
fn a_github_branch_with_no_open_pull_request_gets_a_provisional_plan() {
    let mut fake = Fake::new();
    fake.url = Some(GITHUB.to_owned());
    let (outcome, fake) = push(fake, "auto", None, &feature(HEAD));
    assert_eq!(outcome.exit_code, 0);
    assert_eq!(fake.calls.iter().filter(|call| matches!(call, Call::PullRequests(..))).count(), 1);
    assert_eq!(outcome.reviews[0].context.source, ContextSource::Provisional { incoming: false });
    assert!(outcome.diagnostics.contains(&Diagnostic::NoGateRuns));
}

#[test]
fn a_provisional_base_that_cannot_be_read_blocks() {
    let mut fake = Fake::new();
    fake.tips.clear();
    let (outcome, _) = push(fake, "scope-only", None, &feature(HEAD));
    assert_eq!(outcome.exit_code, 2);
    assert!(problem(&outcome).contains("provisional plan against the remote's main cannot be resolved"));
}

#[test]
fn the_planner_gets_the_committed_arguments_file_and_no_constraint_store() {
    let (outcome, fake) = push(Fake::new(), "scope-only", None, &feature(HEAD));
    assert_eq!(outcome.exit_code, 0);
    let planner = &fake.python_args("affected_scope.py --event")[0];
    assert_eq!(value(planner, "--event"), "pull_request");
    assert_eq!(value(planner, "--head"), HEAD);
    assert_eq!(fake.arguments_seen, [ARGUMENTS.to_vec()]);
    assert!(!planner.iter().any(|arg| arg.contains("constraint")));
    // Verify, apply, and scope recording read the plan the planner wrote.
    let plan = value(planner, "--plan-out");
    assert_eq!(value(&fake.python_args("local_evidence.py verify")[0], "--plan"), plan);
    assert_eq!(value(&fake.python_args("affected_scope.py --apply-to")[0], "--apply-to"), plan);
    assert_eq!(value(&fake.python_args("local_evidence.py scope-record")[0], "--plan"), plan);
}


#[rstest]
#[case::plan("plan", "the planner could not resolve the committed scope of 111111111")]
#[case::verify("verify", "published evidence could not be verified against the resolved plan of 111111111")]
#[case::apply("apply", "the verified evidence could not be applied to the resolved plan of 111111111")]
fn a_helper_failure_blocks_naming_the_revision_and_the_command(#[case] step: &'static str, #[case] expected: &str) {
    let mut fake = Fake::new();
    fake.fail = Some((step, 1, 3));
    let (outcome, fake) = push(fake, "warn", None, &feature(HEAD));
    assert_eq!(outcome.exit_code, 2);
    let text = problem(&outcome);
    assert!(text.contains(expected), "{text}");
    assert!(text.contains("exited with status 3") && text.contains(&format!("{step} refused")), "{text}");
    assert!(text.contains("`python ") && text.contains(".py "), "{text}");
    assert_eq!(outcome.reviews[0].status, ReviewStatus::Blocked);
    assert!(fake.gate_calls().is_empty() && fake.notes() == 0);
}

#[test]
fn a_later_context_failure_publishes_nothing_and_runs_no_gate() {
    // HEAD's own context records a scope receipt; the next update's review
    // fails, so neither the note nor any gate may follow.
    let mut fake = Fake::new();
    fake.fail = Some(("verify", 2, 1));
    let input = [feature(HEAD), line("refs/heads/second", OTHER, "refs/heads/second", ZERO)].concat();
    let (outcome, fake) = push(fake, "strict", None, &input);
    assert_eq!(outcome.exit_code, 2);
    assert_eq!(fake.python_args("local_evidence.py scope-record").len(), 1);
    assert_eq!(fake.notes(), 0);
    assert!(fake.gate_calls().is_empty());
    assert_eq!(outcome.scope, ScopePublication::NotRecorded);
    let statuses: Vec<ReviewStatus> = outcome.reviews.iter().map(|review| review.status).collect();
    assert_eq!(statuses, [ReviewStatus::Passed, ReviewStatus::Blocked]);
    assert!(matches!(blocked(&outcome), BlockReason::Review { update: Some(update), .. } if update == "refs/heads/second -> refs/heads/second"));
}

#[test]
fn python_that_cannot_be_found_blocks_before_any_review() {
    let mut fake = Fake::new();
    fake.python_missing = true;
    let (outcome, fake) = push(fake, "scope-only", None, &feature(HEAD));
    assert_eq!(outcome.exit_code, 2);
    assert!(problem(&outcome).contains("need Python"));
    assert_eq!(fake.calls, [Call::SelectPython]);
}

#[rstest]
#[case::not_head(OTHER, false, "222222222 is not HEAD")]
#[case::dirty(HEAD, true, "the checkout has uncommitted changes")]
fn a_revision_that_is_not_the_clean_checkout_is_reviewed_in_a_temporary_tree(
    #[case] pushed: &str,
    #[case] dirty: bool,
    #[case] why: &str,
) {
    let mut fake = Fake::new();
    fake.dirty = dirty;
    let (outcome, fake) = push(fake, "scope-only", None, &feature(pushed));
    assert_eq!(outcome.exit_code, 0, "{:#?}", outcome.diagnostics);
    assert_eq!(outcome.reviews[0].tree, TreeSource::Temporary);
    let tree = fake.root.parent().unwrap().join("tree-1");
    let planner = fake.calls.iter().find_map(|call| match call {
        Call::Python { script, cwd, args } if script == "affected_scope.py" && args[1] == "--event" => Some((cwd.clone(), args[0].clone())),
        _ => None,
    });
    let (cwd, script) = planner.unwrap();
    assert_eq!(cwd, tree);
    assert!(Path::new(&script).starts_with(&tree));
    // The tree is released after the update's last context.
    let position = |wanted: &Call| fake.calls.iter().position(|call| call == wanted).unwrap();
    assert!(position(&Call::Materialize(id(pushed))) < position(&Call::Release));
    assert!(fake.emitted.iter().any(|line| line.contains(why)));
}

#[test]
fn the_clean_checkout_is_planned_in_place() {
    let (outcome, fake) = push(Fake::new(), "scope-only", None, &feature(HEAD));
    assert_eq!(outcome.reviews[0].tree, TreeSource::Checkout);
    assert!(!fake.calls.iter().any(|call| matches!(call, Call::Materialize(_) | Call::Release)));
}

#[test]
fn a_temporary_tree_is_released_when_its_review_fails() {
    let mut fake = Fake::new();
    fake.fail = Some(("apply", 1, 1));
    let (outcome, fake) = push(fake, "scope-only", None, &feature(OTHER));
    assert_eq!(outcome.exit_code, 2);
    assert_eq!(fake.calls.last(), Some(&Call::Release));
}

#[test]
fn a_signal_during_review_blocks_with_its_status_and_runs_nothing_after() {
    let mut fake = Fake::new();
    fake.signal_during = Some(("verify", CancelSignal::Terminate));
    let (outcome, fake) = push(fake, "warn", None, &feature(HEAD));
    assert_eq!(outcome.exit_code, 143);
    assert_eq!(blocked(&outcome), &BlockReason::Interrupted { exit_code: 143 });
    assert!(fake.python_args("affected_scope.py --apply-to").is_empty());
    assert!(fake.gate_calls().is_empty() && fake.notes() == 0);
}

// ----------------------------------------------------------- constraints --

#[rstest]
#[case::auto("auto")]
#[case::strict("strict")]
#[case::warn("warn")]
#[case::scope_only("scope-only")]
fn an_executing_prohibited_cell_blocks_every_mode(#[case] mode: &str) {
    let mut fake = Fake::new();
    fake.plans.insert(id(HEAD), WSL_EXECUTING);
    fake.constraint("wsl.json", &record("wsl2-ubuntu", ""));
    let (outcome, fake) = push(fake, mode, None, &main_push(MAIN_TIP));
    assert_eq!(outcome.exit_code, 1);
    match blocked(&outcome) {
        BlockReason::Constraint { update, refusals } => {
            assert_eq!(update, "refs/heads/main -> refs/heads/main");
            assert_eq!(refusals, &["wsl2-ubuntu: no wsl2-ubuntu here (set by ken, expires 2099-01-01)"]);
        }
        other => panic!("{other:?}"),
    }
    assert!(fake.gate_calls().is_empty() && fake.notes() == 0);
}

#[rstest]
#[case::reused(WSL_REUSED)]
#[case::absent(WSL_ABSENT)]
fn a_reused_or_absent_prohibited_environment_does_not_block(#[case] plan: &'static str) {
    let mut fake = Fake::new();
    fake.plans.insert(id(HEAD), plan);
    fake.constraint("wsl.json", &record("wsl2-ubuntu", ""));
    let (outcome, _) = push(fake, "scope-only", None, &feature(HEAD));
    assert_eq!(outcome.exit_code, 0, "{:#?}", outcome.diagnostics);
}

#[test]
fn an_expired_prohibition_is_reported_and_does_not_block() {
    let mut fake = Fake::new();
    fake.plans.insert(id(HEAD), WSL_EXECUTING);
    fake.constraint("wsl.json", &record("wsl2-ubuntu", "").replace("2099-01-01", "2026-11-09"));
    let (outcome, fake) = push(fake, "scope-only", None, &feature(HEAD));
    assert_eq!(outcome.exit_code, 0);
    assert!(fake.emitted.iter().any(|line| line.starts_with("ci-constraints: ignoring expired constraint")));
}

#[rstest]
#[case::remote_name("feature", 1)]
#[case::local_name("work", 1)]
#[case::another_branch("unrelated", 0)]
fn a_renamed_refspec_binds_a_record_under_either_branch_name(#[case] branch: &str, #[case] exit_code: u8) {
    let mut fake = Fake::new();
    fake.plans.insert(id(HEAD), WSL_EXECUTING);
    fake.constraint("wsl.json", &record("wsl2-ubuntu", &format!(", \"branch\": \"{branch}\"")));
    let input = line("refs/heads/work", HEAD, "refs/heads/feature", ZERO);
    let (outcome, _) = push(fake, "scope-only", None, &input);
    assert_eq!(outcome.exit_code, exit_code, "{:#?}", outcome.diagnostics);
    assert_eq!(outcome.reviews[0].branches, ["feature", "work"]);
}

#[rstest]
#[case::this_remote(ELSEWHERE, 1)]
#[case::another_remote("https://example.com/owner/other.git", 0)]
fn the_repository_identity_is_the_remote_being_pushed_to(#[case] repository: &str, #[case] exit_code: u8) {
    let mut fake = Fake::new();
    fake.plans.insert(id(HEAD), WSL_EXECUTING);
    fake.constraint("wsl.json", &record("wsl2-ubuntu", &format!(", \"repository\": \"{repository}\"")));
    let (outcome, _) = push(fake, "scope-only", None, &feature(HEAD));
    assert_eq!(outcome.exit_code, exit_code, "{:#?}", outcome.diagnostics);
}

#[test]
fn a_malformed_record_binds_and_names_its_field() {
    let fake = Fake::new();
    fake.constraint("bad.json", &record("wsl2-ubuntu", ", \"branch\": 7"));
    let (outcome, _) = push(fake, "scope-only", None, &feature(HEAD));
    assert_eq!(outcome.exit_code, 1);
    let BlockReason::Constraint { refusals, .. } = blocked(&outcome) else { panic!() };
    assert!(refusals[0].contains("bad.json") && refusals[0].contains("branch"), "{refusals:?}");
}

#[test]
fn an_unreadable_reviewed_plan_blocks_even_with_an_empty_store() {
    let mut fake = Fake::new();
    fake.unreadable_review = true;
    let (outcome, fake) = push(fake, "scope-only", None, &feature(HEAD));
    assert_eq!(outcome.exit_code, 1);
    let BlockReason::Constraint { refusals, .. } = blocked(&outcome) else { panic!() };
    assert!(refusals[0].contains("review.json") && refusals[0].contains("cells"), "{refusals:?}");
    assert_eq!(fake.notes(), 0);
}

// ------------------------------------------------------------ scope note --

#[test]
fn scope_is_published_after_every_review_and_before_auto_decides() {
    let input = [feature(HEAD), line("refs/heads/second", OTHER, "refs/heads/second", ZERO)].concat();
    let (outcome, fake) = push(Fake::new(), "auto", None, &input);
    assert_eq!(outcome.exit_code, 0);
    assert_eq!(outcome.scope, ScopePublication::Published { base: id(MAIN_TIP) });
    let note = fake.calls.iter().position(|call| *call == Call::Note(id(HEAD))).unwrap();
    let last_helper = fake.calls.iter().rposition(|call| matches!(call, Call::Python { .. })).unwrap();
    assert!(last_helper < note, "the note waits for every review");
    assert!(fake.gate_calls().is_empty());
}

#[test]
fn a_dirty_checkout_still_publishes_its_committed_scope() {
    let mut fake = Fake::new();
    fake.dirty = true;
    let (outcome, _) = push(fake, "strict", None, &main_push(MAIN_TIP));
    assert_eq!(outcome.scope, ScopePublication::Published { base: id(MAIN_TIP) });
    assert_eq!(outcome.validation, Some(Validation::Withheld(WithholdReason::InitialDirty)));
}

#[test]
fn a_failed_scope_publication_is_named_and_leaves_the_exit_code_alone() {
    let mut fake = Fake::new();
    fake.note_fails = true;
    let (outcome, fake) = push(fake, "scope-only", None, &feature(HEAD));
    assert_eq!(outcome.exit_code, 0);
    assert!(matches!(&outcome.scope, ScopePublication::Failed { message, .. } if message.contains("refused")));
    assert!(fake.emitted.iter().any(|line| line.contains("Scope evidence PUBLICATION failed for 111111111")));
}

#[test]
fn a_refused_scope_recording_still_feeds_the_reviewed_plan_to_the_gates() {
    // `scope-record` refuses a base that is not an ancestor; the comparison
    // itself is still the reviewed one.
    let mut fake = Fake::new();
    fake.fail = Some(("scope-record", 1, 1));
    let (outcome, fake) = push(fake, "strict", None, &main_push(MAIN_TIP));
    assert_eq!(outcome.exit_code, 0);
    assert_eq!(outcome.scope, ScopePublication::NotRecorded);
    assert_eq!(fake.notes(), 0);
    assert!(outcome.gates.as_ref().unwrap().plan_in.is_some());
    assert!(fake.emitted.iter().any(|line| line.contains("Scope evidence RECORDING failed")));
}

// ----------------------------------------------------------------- gates --

fn gate_variables(fake: &Fake) -> HashMap<String, String> {
    match fake.gate_calls().as_slice() {
        [Call::Gates { variables, .. }] => variables.clone(),
        other => panic!("expected one gate run, got {other:?}"),
    }
}

#[test]
fn a_clean_head_feeds_its_reviewed_plan_to_the_gates_and_publishes_validation() {
    let (outcome, fake) = push(Fake::new(), "strict", None, &main_push(MAIN_TIP));
    assert_eq!(outcome.exit_code, 0, "{:#?}", outcome.diagnostics);
    let variables = gate_variables(&fake);
    let review = value(&fake.python_args("affected_scope.py --apply-to")[0], "--plan-out").to_owned();
    assert_eq!(variables["BISCUIT_CI_PLAN_IN"], review);
    for name in ["BISCUIT_CI_SCOPE_OUT", "BISCUIT_CI_PLAN_OUT", "BISCUIT_CI_REPORTS_OUT"] {
        assert!(variables.contains_key(name), "{name}");
    }
    // The receipt is recorded against the same context's raw plan and base.
    let record = &fake.python_args("local_evidence.py record-cells")[0];
    let raw = value(&fake.python_args("affected_scope.py --event")[0], "--plan-out").to_owned();
    assert_eq!(value(record, "--plan"), raw);
    assert_eq!(value(record, "--base"), MAIN_TIP);
    assert_eq!(value(record, "--environment"), "macos-latest");
    let report_dir = fake.evidence_root().join(HEAD).join("macos-latest");
    assert_eq!(value(record, "--report-dir"), report_dir.to_string_lossy());
    assert_eq!(
        outcome.validation,
        Some(Validation::Published {
            environment: "macos-latest".to_owned(),
            cells: Some(2),
            report_dir: report_dir.clone(),
        })
    );
    assert!(report_dir.join("alpha-L1.xml").is_file(), "reports are retained before the receipt");
    assert_eq!(outcome.diagnostics.last(), Some(&Diagnostic::GatesPassed));
}

#[rstest]
#[case::override_(Some(" alpha  beta "), false, WithholdReason::SelectionOverride)]
#[case::dirty(None, true, WithholdReason::InitialDirty)]
fn an_override_or_dirty_checkout_replans_and_publishes_no_validation(
    #[case] selection: Option<&str>,
    #[case] dirty: bool,
    #[case] reason: WithholdReason,
) {
    let mut fake = Fake::new();
    fake.dirty = dirty;
    let (outcome, fake) = push(fake, "strict", selection, &main_push(MAIN_TIP));
    assert_eq!(outcome.exit_code, 0);
    let Call::Gates { selection: selectors, variables } = fake.gate_calls()[0].clone() else { panic!() };
    assert!(!variables.contains_key("BISCUIT_CI_PLAN_IN"), "the gates replan");
    if selection.is_some() {
        assert_eq!(selectors, ["alpha", "beta"]);
    }
    assert_eq!(outcome.validation, Some(Validation::Withheld(reason)));
    assert!(fake.python_args("local_evidence.py record-cells").is_empty());
    // The override narrows the gates only, never the review.
    assert_eq!(fake.helpers().iter().filter(|label| label.ends_with("--event")).count(), 1);
}

#[test]
fn a_push_that_does_not_carry_head_reviews_its_revision_and_withholds_validation() {
    let input = line("refs/heads/main", OTHER, "refs/heads/main", MAIN_TIP);
    let (outcome, fake) = push(Fake::new(), "strict", None, &input);
    assert_eq!(outcome.exit_code, 0);
    assert_eq!(outcome.reviews[0].trigger, id(OTHER));
    assert_eq!(outcome.scope, ScopePublication::NotRecorded);
    assert!(!gate_variables(&fake).contains_key("BISCUIT_CI_PLAN_IN"));
    assert_eq!(outcome.validation, Some(Validation::Withheld(WithholdReason::OutgoingHeadMissing)));
}

#[test]
fn a_strict_failure_publishes_its_complete_evidence_then_blocks() {
    let mut fake = Fake::new();
    fake.gates.code = 3;
    let (outcome, fake) = push(fake, "strict", None, &main_push(MAIN_TIP));
    assert_eq!(outcome.exit_code, 3);
    assert_eq!(blocked(&outcome), &BlockReason::GateFailed { exit_code: 3 });
    assert_eq!(fake.python_args("observation_notes.py publish-receipt").len(), 1);
    assert!(matches!(outcome.validation, Some(Validation::Published { .. })));
    // Publication comes before the verdict.
    let published = outcome.diagnostics.iter().position(|diagnostic| matches!(diagnostic, Diagnostic::Info(text) if text.starts_with("Published 2"))).unwrap();
    let verdict = outcome.diagnostics.iter().position(|diagnostic| matches!(diagnostic, Diagnostic::Blocked(_))).unwrap();
    assert!(published < verdict);
}

#[test]
fn warn_lets_an_ordinary_failure_through_and_still_publishes_it() {
    let mut fake = Fake::new();
    fake.gates.code = 1;
    let (outcome, _) = push(fake, "warn", None, &main_push(MAIN_TIP));
    assert_eq!(outcome.exit_code, 0);
    assert_eq!(outcome.diagnostics.last(), Some(&Diagnostic::WarnedFailure { exit_code: 1 }));
    assert!(outcome.diagnostics.last().unwrap().to_string().contains("--no-verify"));
    assert!(matches!(outcome.validation, Some(Validation::Published { .. })));
}

#[rstest]
#[case::gate_exit_130(Gates { code: 130, ..Gates::default() }, 130)]
#[case::interrupted_marker(Gates { code: 1, marker: true, ..Gates::default() }, 130)]
#[case::hook_signalled(Gates { signal: Some(CancelSignal::Hangup), ..Gates::default() }, 129)]
fn an_interrupted_run_blocks_even_in_warn_and_withholds_validation(#[case] gates: Gates, #[case] exit_code: u8) {
    for mode in ["warn", "strict"] {
        let mut fake = Fake::new();
        fake.gates = gates.clone();
        let (outcome, fake) = push(fake, mode, None, &main_push(MAIN_TIP));
        assert_eq!(outcome.exit_code, exit_code, "{mode}");
        assert_eq!(blocked(&outcome), &BlockReason::Interrupted { exit_code });
        assert_eq!(outcome.validation, Some(Validation::Withheld(WithholdReason::Interrupted)));
        assert!(fake.python_args("local_evidence.py record-cells").is_empty());
        assert!(!fake.evidence_root().exists(), "nothing is retained for an interrupted run");
    }
}

#[rstest]
#[case::head_moved(Gates { head_after: Some(id(OTHER)), ..Gates::default() }, WithholdReason::HeadChanged)]
#[case::dirtied(Gates { dirty_after: true, ..Gates::default() }, WithholdReason::FinalDirty)]
#[case::no_plan(Gates { plan: false, ..Gates::default() }, WithholdReason::NoLocalPlan)]
#[case::no_manifest(Gates { manifest: false, ..Gates::default() }, WithholdReason::EmptyManifest)]
fn a_run_that_is_not_the_outgoing_trees_withholds_validation(#[case] gates: Gates, #[case] reason: WithholdReason) {
    let mut fake = Fake::new();
    fake.gates = gates;
    let (outcome, fake) = push(fake, "strict", None, &main_push(MAIN_TIP));
    assert_eq!(outcome.exit_code, 0);
    assert_eq!(outcome.validation, Some(Validation::Withheld(reason)));
    assert!(fake.python_args("local_evidence.py record-cells").is_empty());
    assert!(fake.emitted.iter().any(|line| line.starts_with("No validation receipt:")));
}

#[test]
fn a_host_without_a_ci_environment_withholds_validation() {
    let mut fake = Fake::new();
    fake.environment = None;
    let (outcome, _) = push(fake, "strict", None, &main_push(MAIN_TIP));
    assert_eq!(outcome.validation, Some(Validation::Withheld(WithholdReason::UnsupportedOs)));
}

// -------------------------------------------------------------- evidence --

#[test]
fn retry_reports_overlay_the_durable_directory() {
    let fake = Fake::new();
    let report_dir = fake.evidence_root().join(HEAD).join("macos-latest");
    std::fs::create_dir_all(&report_dir).unwrap();
    std::fs::write(report_dir.join("reused-L2.xml"), "earlier").unwrap();
    let (outcome, _fake) = push(fake, "strict", None, &main_push(MAIN_TIP));
    assert_eq!(outcome.exit_code, 0);
    assert_eq!(std::fs::read_to_string(report_dir.join("reused-L2.xml")).unwrap(), "earlier");
    assert!(report_dir.join("alpha-L1.xml").is_file());
}

#[test]
fn a_relative_evidence_root_resolves_against_the_checkout() {
    let mut fake = Fake::new();
    fake.variables.insert("BISCUIT_CI_EVIDENCE_DIR".to_owned(), "evidence".into());
    let (outcome, fake) = push(fake, "strict", None, &main_push(MAIN_TIP));
    let expected = fake.root.join("evidence").join(HEAD).join("macos-latest");
    assert!(matches!(&outcome.validation, Some(Validation::Published { report_dir, .. }) if *report_dir == expected));
    assert!(expected.join("alpha-L1.xml").is_file());
}

#[test]
fn reports_that_cannot_be_retained_publish_no_receipt_and_stay_readable() {
    let mut fake = Fake::new();
    let blocker = fake.root.parent().unwrap().join("not-a-directory");
    std::fs::write(&blocker, "").unwrap();
    fake.variables.insert("BISCUIT_CI_EVIDENCE_DIR".to_owned(), blocker.into_os_string());
    let (outcome, fake) = push(fake, "strict", None, &main_push(MAIN_TIP));
    assert_eq!(outcome.exit_code, 0);
    assert_eq!(outcome.validation, Some(Validation::Withheld(WithholdReason::ReportRetentionFailed)));
    assert!(fake.python_args("local_evidence.py record-cells").is_empty());
    let kept = fake
        .emitted
        .iter()
        .find_map(|line| line.strip_prefix("The run's staged reports remain at "))
        .map(|path| PathBuf::from(path.trim_end_matches('.')))
        .expect("the staging path is printed");
    assert!(kept.join("alpha-L1.xml").is_file());
    std::fs::remove_dir_all(kept.parent().unwrap()).unwrap();
}

#[rstest]
#[case::recorded_locally(("publish-receipt", 1, 4), "RecordedLocally")]
#[case::publish_failed(("publish-receipt", 1, 2), "PublishFailed")]
#[case::record_refused(("record-cells", 1, 1), "RecordFailed")]
fn a_publication_failure_is_advisory_and_keeps_the_reports(
    #[case] fail: (&'static str, usize, i32),
    #[case] expected: &str,
) {
    let mut fake = Fake::new();
    fake.fail = Some(fail);
    let (outcome, fake) = push(fake, "strict", None, &main_push(MAIN_TIP));
    assert_eq!(outcome.exit_code, 0);
    let validation = format!("{:?}", outcome.validation.unwrap());
    assert!(validation.starts_with(expected), "{validation}");
    assert!(fake.evidence_root().join(HEAD).join("macos-latest").join("alpha-L1.xml").is_file());
}

#[test]
fn the_environment_follows_the_host_os() {
    use crate::ci_push::ci_environment;
    use sniff::os::OsType;
    assert_eq!(ci_environment(&OsType::MacOS, "24.0.0"), Some("macos-latest"));
    assert_eq!(ci_environment(&OsType::Windows, "10.0.26100"), Some("windows-latest"));
    assert_eq!(ci_environment(&OsType::Linux, "5.15.167.4-Microsoft-standard-WSL2"), Some("wsl2-ubuntu"));
    assert_eq!(ci_environment(&OsType::Linux, "6.8.0-45-generic"), Some("ubuntu-latest"));
    assert_eq!(ci_environment(&OsType::FreeBSD, "14.1"), None);
}
