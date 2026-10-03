//! Level-1 rehearsal of the shipped PR flow (`prompts/pr.md` and `prompts/_pr/`)
//! against a stub provider and a disposable repository.
//!
//! Each route runs the shipped prompts, unedited, in a fresh repository whose
//! `origin` is a `github.com` URL rewritten by `url.<bare>.insteadOf` to a local
//! bare repository: `remote_vendor()` reads `github`, and every fetch and push
//! lands in the bare repository. Nothing reaches the network or a real
//! provider, and `PLAYA_DRY_RUN` keeps every lifecycle sound silent.
//!
//! The provider stub is a two-line wrapper that re-executes this test binary in
//! a helper mode, so the stage logic is portable Rust. It recognizes a stage
//! by the heading of the prompt it reads on stdin (the interactive triage
//! stage, whose prompt arrives in argv, by `INTERACTIVE=true`), edits the
//! shared report the way an agent following the report contract would, and
//! appends the stage to a trail the assertions read. Each prompt read on stdin
//! is also saved, numbered by its place in the trail, so a route can assert
//! what each stage's agent was told.

use crate::common;

use common::{CliProcessFixture, write};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;

use biscuit_file::serde_yaml_ng::{Mapping, Value};

/// A positional argument: libtest reads it as a name filter, which under
/// `--exact` matches nothing, so the harness ignores it.
const HELPER_ARG: &str = "pr-flow-stub=";

/// The shipped prompts a PR run can reach, staged into each repository.
const PROMPTS: &[(&str, &str)] = &[
    ("prompts/pr.md", include_str!("../../../../prompts/pr.md")),
    ("prompts/commit.md", include_str!("../../../../prompts/commit.md")),
    ("prompts/_pr/dirty.md", include_str!("../../../../prompts/_pr/dirty.md")),
    ("prompts/_pr/push.md", include_str!("../../../../prompts/_pr/push.md")),
    ("prompts/_pr/_facts.md", include_str!("../../../../prompts/_pr/_facts.md")),
    ("prompts/_pr/diagnose.md", include_str!("../../../../prompts/_pr/diagnose.md")),
    ("prompts/_pr/_diagnosis.md", include_str!("../../../../prompts/_pr/_diagnosis.md")),
    ("prompts/_pr/triage.md", include_str!("../../../../prompts/_pr/triage.md")),
    ("prompts/_pr/fix.md", include_str!("../../../../prompts/_pr/fix.md")),
    ("prompts/_pr/open.md", include_str!("../../../../prompts/_pr/open.md")),
    ("prompts/_pr/_report.md", include_str!("../../../../prompts/_pr/_report.md")),
];

/// What the stub does at each stage of one route.
#[derive(Serialize, Deserialize)]
struct Scenario {
    git: PathBuf,
    repo: PathBuf,
    report: PathBuf,
    trail: PathBuf,
    /// Where each stdin prompt is saved, as `{trail position}-{stage}.md`.
    prompts: PathBuf,
    /// A path the commit stage writes and leaves uncommitted.
    commit_leaves: Option<String>,
    /// Whether the pre-push hook lets the push through.
    push_passes: bool,
    /// A branch the push stage creates first, as it must when started on the base.
    create_branch: Option<String>,
    /// Whether each fix attempt, in order, verifies.
    fix_attempts: Vec<bool>,
    /// The caller's answer in the interactive triage session.
    triage_wants_fix: bool,
}

// ── Stub provider ──────────────────────────────────────────────────────────

/// What a re-executed copy of this binary does instead of testing.
#[test]
fn stub_entrypoint() {
    let Some(scenario) = std::env::args().find_map(|arg| arg.strip_prefix(HELPER_ARG).map(PathBuf::from))
    else {
        return;
    };
    let scenario: Scenario =
        serde_json::from_str(&std::fs::read_to_string(scenario).expect("read scenario")).expect("parse scenario");
    let stage = if std::env::var("INTERACTIVE").as_deref() == Ok("true") {
        "triage"
    } else {
        let mut prompt = String::new();
        std::io::Read::read_to_string(&mut std::io::stdin(), &mut prompt).expect("read prompt");
        let stage = [
            ("# Push a Branch for a Pull Request", "push"),
            ("# Diagnose a Blocked Push", "diagnose"),
            ("# Fix Local Test Failures", "fix"),
            ("# Open the Pull Request", "open"),
            ("# Commit Staged Files", "commit"),
        ]
        .into_iter()
        .find(|(heading, _)| prompt.lines().any(|line| line.trim_end() == *heading))
        .map_or("unknown", |(_, stage)| stage);
        let position = std::fs::read_to_string(&scenario.trail).unwrap_or_default().lines().count() + 1;
        std::fs::create_dir_all(&scenario.prompts).expect("create prompt directory");
        std::fs::write(scenario.prompts.join(format!("{position:02}-{stage}.md")), &prompt).expect("save prompt");
        stage
    };
    append_line(&scenario.trail, stage);
    let git = |args: &[&str]| run_git(&scenario.git, &scenario.repo, args);
    match stage {
        "push" => {
            if let Some(branch) = &scenario.create_branch {
                git(&["switch", "-q", "-c", branch]);
            }
            let branch = git(&["rev-parse", "--abbrev-ref", "HEAD"]);
            let head = git(&["rev-parse", "HEAD"]);
            if scenario.push_passes {
                git(&["push", "-q", "-u", "origin", "HEAD"]);
                update_report(&scenario.report, &[
                    ("status", "passed".into()),
                    ("branch", branch.into()),
                    ("head", head.into()),
                    ("summary", "Pushed; the hook passed.".into()),
                ]);
            } else {
                let log = scenario.report.with_extension("push.log");
                std::fs::write(&log, "L1 claudine-cli failed\n").expect("write push log");
                update_report(&scenario.report, &[
                    ("status", "local_failed".into()),
                    ("branch", branch.into()),
                    ("head", head.into()),
                    ("log", log.to_string_lossy().into_owned().into()),
                    ("summary", "The L1 claudine-cli gate failed.".into()),
                ]);
            }
        }
        "diagnose" => update_report(&scenario.report, &[
            ("status", "diagnosed".into()),
            ("summary", "One test on this branch regressed.".into()),
            ("issue", "Restore fixed.txt so the regressed test passes.".into()),
            ("failures", Value::Sequence(Vec::new())),
        ]),
        "triage" => update_report(&scenario.report, &[("fix_requested", scenario.triage_wants_fix.into())]),
        "fix" => {
            let attempt = report_value(&scenario.report, "fix_attempts")
                .and_then(|value| value.as_u64())
                .expect("fix.md counts attempts before the agent starts");
            let verifies = scenario.fix_attempts.get(attempt as usize - 1).copied().unwrap_or(false);
            if verifies {
                std::fs::write(scenario.repo.join("fixed.txt"), "fixed\n").expect("write repair");
                git(&["add", "fixed.txt"]);
                update_report(&scenario.report, &[
                    ("status", "fixed".into()),
                    ("staged", Value::Sequence(vec!["fixed.txt".into()])),
                    ("summary", format!("Repaired on attempt {attempt}.").into()),
                ]);
            }
        }
        "open" => update_report(&scenario.report, &[
            ("status", "pr_opened".into()),
            ("pr_url", "https://github.com/example/pr-rehearsal/pull/1".into()),
        ]),
        "commit" => {
            git(&["commit", "-q", "-m", "rehearsal commit"]);
            if let Some(path) = &scenario.commit_leaves {
                std::fs::write(scenario.repo.join(path), "left behind\n").expect("write leftover");
            }
        }
        other => panic!("the stub did not recognize the stage it was launched for: {other}"),
    }
}

fn run_git(git: &Path, repo: &Path, args: &[&str]) -> String {
    let mut command = Command::new(git);
    // A suite run from a Git hook inherits `GIT_DIR` and friends, which would
    // point these commands at the checkout instead of the rehearsal repository.
    for key in common::GIT_PLUMBING_VARS {
        command.env_remove(key);
    }
    let output = command
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .expect("run git");
    assert!(
        output.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

fn append_line(path: &Path, line: &str) {
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .expect("open trail");
    writeln!(file, "{line}").expect("append trail");
}

/// The report split into its frontmatter mapping and body.
fn read_report(path: &Path) -> (Mapping, String) {
    let text = std::fs::read_to_string(path).unwrap_or_default().replace("\r\n", "\n");
    let Some(rest) = text.strip_prefix("---\n") else {
        return (Mapping::new(), text);
    };
    let (yaml, body) = rest.split_once("\n---\n").unwrap_or((rest.trim_end_matches("---\n"), ""));
    let mapping = biscuit_file::serde_yaml_ng::from_str::<Option<Mapping>>(yaml)
        .expect("report frontmatter parses")
        .unwrap_or_default();
    (mapping, body.to_string())
}

fn report_value(path: &Path, key: &str) -> Option<Value> {
    read_report(path).0.get(key).cloned()
}

fn update_report(path: &Path, entries: &[(&str, Value)]) {
    let (mut mapping, body) = read_report(path);
    for (key, value) in entries {
        mapping.insert(Value::from(*key), value.clone());
    }
    let yaml = biscuit_file::serde_yaml_ng::to_string(&mapping).expect("serialize report");
    std::fs::write(path, format!("---\n{yaml}---\n{body}")).expect("write report");
}

// ── Fixture ────────────────────────────────────────────────────────────────

struct Rehearsal {
    fixture: CliProcessFixture,
    scenario: Scenario,
    bare: PathBuf,
}

struct Outcome {
    code: Option<i32>,
    output: String,
    trail: Vec<String>,
}

/// Where the route starts.
enum Start {
    /// `feat/demo`, one commit ahead of `origin/main`.
    FeatureBranch,
    /// `main` itself, one unpushed commit ahead of `origin/main`.
    BaseBranch,
}

impl Rehearsal {
    fn new(name: &str, start: Start) -> Self {
        let fixture = CliProcessFixture::named(name);
        fixture.seed_user_config();
        let git = which::which("git").expect("the PR flow rehearsal needs git");
        let repo = fixture.cwd().to_path_buf();
        let bare = fixture.workspace_path().join("remote.git");
        let git_in = |dir: &Path, args: &[&str]| run_git(&git, dir, args);

        git_in(fixture.workspace_path(), &["init", "-q", "--bare", &portable(&bare)]);
        git_in(&repo, &["init", "-q"]);
        git_in(&repo, &["symbolic-ref", "HEAD", "refs/heads/main"]);
        for (key, value) in [
            ("user.email", "rehearsal@example.com"),
            ("user.name", "Rehearsal"),
            ("commit.gpgsign", "false"),
            ("core.autocrlf", "false"),
        ] {
            git_in(&repo, &["config", key, value]);
        }
        let origin = "https://github.com/example/pr-rehearsal.git";
        git_in(&repo, &["remote", "add", "origin", origin]);
        git_in(&repo, &["config", &format!("url.{}.insteadOf", portable(&bare)), origin]);

        for (path, content) in PROMPTS {
            write(&repo.join(path), content);
        }
        write(&repo.join(".claudine/memory/commits.md"), "# Commit lessons\n");
        write(&repo.join(".gitignore"), ".claudine/tmp/\n");
        write(&repo.join(".darkmatter-shell-whitelist"), "prefix git\nprefix just\n");
        write(&repo.join("README.md"), "rehearsal\n");
        git_in(&repo, &["add", "-A"]);
        git_in(&repo, &["commit", "-q", "-m", "initial"]);
        git_in(&repo, &["push", "-q", "-u", "origin", "main"]);
        if matches!(start, Start::FeatureBranch) {
            git_in(&repo, &["checkout", "-q", "-b", "feat/demo"]);
        }
        write(&repo.join("work.txt"), "work\n");
        git_in(&repo, &["add", "work.txt"]);
        git_in(&repo, &["commit", "-q", "-m", "feat: the work to propose"]);

        let scenario = Scenario {
            git: git.clone(),
            repo: repo.clone(),
            report: repo.join(".claudine/tmp/pr/report.md"),
            trail: fixture.workspace_path().join("trail.txt"),
            prompts: fixture.workspace_path().join("prompts"),
            commit_leaves: None,
            push_passes: true,
            create_branch: None,
            fix_attempts: vec![true],
            triage_wants_fix: true,
        };
        let rehearsal = Self { fixture, scenario, bare };
        rehearsal.install_command_stubs(true);
        rehearsal
    }

    /// Writes `gh`, `git`, `just`, and the provider wrapper into the fixture
    /// `bin`, which is the child's whole `PATH`.
    fn install_command_stubs(&self, with_just: bool) {
        let bin = self.fixture.bin_dir();
        let git = portable(&self.scenario.git);
        write_stub(bin, "gh", "exit 0", "exit /b 0");
        // A lifecycle `shell` action runs through `sh -c` on Unix (`cmd.exe`
        // on Windows), resolved from `PATH` like any other program.
        #[cfg(unix)]
        write_stub(bin, "sh", "exec /bin/sh \"$@\"", "");
        write_stub(
            bin,
            "git",
            &format!("exec '{git}' \"$@\""),
            &format!("\"{git}\" %*\r\nexit /b %ERRORLEVEL%"),
        );
        if with_just {
            write_stub(bin, "just", "echo 'plan: nothing to run'", "echo plan: nothing to run\r\nexit /b 0");
        } else {
            for name in ["just", "just.cmd"] {
                let _ = std::fs::remove_file(bin.join(name));
            }
        }
        let exe = portable(&std::env::current_exe().expect("current test executable"));
        let module = module_path!()
            .split_once("::")
            .map(|(_, rest)| rest)
            .expect("module path has a crate segment");
        let scenario = portable(&self.fixture.workspace_path().join("scenario.json"));
        let log = portable(&self.fixture.workspace_path().join("stub.log"));
        let init = r#"{"type":"system","subtype":"init","session_id":"rehearsal","model":"stub"}"#;
        let result = r#"{"type":"result","subtype":"success","result":"done","session_id":"rehearsal","is_error":false}"#;
        write_stub(
            bin,
            "claude",
            &format!(
                "'{exe}' --exact {module}::stub_entrypoint --nocapture '{HELPER_ARG}{scenario}' >/dev/null 2>>'{log}' || exit 1\n\
                 printf '%s\\n' '{init}'\n\
                 printf '%s\\n' '{result}'"
            ),
            &format!(
                "\"{exe}\" --exact {module}::stub_entrypoint --nocapture \"{HELPER_ARG}{scenario}\" >NUL 2>>\"{log}\"\r\n\
                 if errorlevel 1 exit /b 1\r\n\
                 echo {init}\r\n\
                 echo {result}\r\n\
                 exit /b 0"
            ),
        );
    }

    fn make_dirty(&self) {
        write(&self.scenario.repo.join("wip.txt"), "uncommitted\n");
    }

    fn run(&self, args: &[&str]) -> Outcome {
        write(
            &self.fixture.workspace_path().join("scenario.json"),
            &serde_json::to_string(&self.scenario).expect("serialize scenario"),
        );
        let report = format!("report={}", portable(&self.scenario.report));
        let output = self
            .fixture
            .command_builder()
            // Every program the flow runs is a stub in the fixture `bin`, so a
            // missing tool is really missing, on every host.
            .fake_only_path()
            .build()
            .args(["compose", "--claude", "prompts/pr.md", &report])
            .args(args)
            .output()
            .expect("claudine runs");
        assert!(
            !self.fixture.audio_spool().exists(),
            "a lifecycle sound escaped PLAYA_DRY_RUN into {}",
            self.fixture.audio_spool().display()
        );
        Outcome {
            code: output.status.code(),
            output: format!(
                "{}{}\n--- stub log ---\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr),
                std::fs::read_to_string(self.fixture.workspace_path().join("stub.log")).unwrap_or_default()
            ),
            trail: std::fs::read_to_string(&self.scenario.trail)
                .unwrap_or_default()
                .lines()
                .map(str::to_string)
                .collect(),
        }
    }

    /// Every prompt `stage` read on stdin, in launch order.
    fn prompts(&self, stage: &str) -> Vec<String> {
        let suffix = format!("-{stage}.md");
        let mut paths: Vec<PathBuf> = std::fs::read_dir(&self.scenario.prompts)
            .map(|entries| entries.flatten().map(|entry| entry.path()).collect())
            .unwrap_or_default();
        paths.retain(|path| path.file_name().and_then(|name| name.to_str()).is_some_and(|name| name.ends_with(&suffix)));
        paths.sort();
        paths
            .iter()
            .map(|path| std::fs::read_to_string(path).expect("read saved prompt").replace("\r\n", "\n"))
            .collect()
    }

    /// The one prompt `stage` read.
    #[track_caller]
    fn prompt(&self, stage: &str) -> String {
        let mut prompts = self.prompts(stage);
        assert_eq!(prompts.len(), 1, "exactly one `{stage}` prompt");
        prompts.remove(0)
    }

    fn report(&self, key: &str) -> Option<Value> {
        report_value(&self.scenario.report, key)
    }

    fn status(&self) -> Option<String> {
        self.report("status").and_then(|value| value.as_str().map(str::to_string))
    }

    /// The branches the bare repository holds, as `name sha` lines.
    fn remote_branches(&self) -> Vec<String> {
        run_git(&self.scenario.git, &self.bare, &["for-each-ref", "--format=%(refname:short)", "refs/heads"])
            .lines()
            .map(str::to_string)
            .collect()
    }

    fn git(&self, args: &[&str]) -> String {
        run_git(&self.scenario.git, &self.scenario.repo, args)
    }
}

fn portable(path: &Path) -> String {
    biscuit_file::to_portable_string(path)
}

/// A command on the fixture `PATH`: a `/bin/sh` script, or a `.cmd` on Windows.
fn write_stub(bin: &Path, name: &str, unix_body: &str, windows_body: &str) {
    #[cfg(unix)]
    {
        let _ = windows_body;
        common::write_executable(&bin.join(name), &format!("#!/bin/sh\n{unix_body}\n"));
    }
    #[cfg(windows)]
    {
        let _ = unix_body;
        write(&bin.join(format!("{name}.cmd")), &format!("@echo off\r\n{windows_body}\r\n"));
    }
}

/// Fragments of the note `_pr/dirty.md` and `_pr/fix.md` once appended to the
/// commit handoff's `message`, saying the staged-file list was captured before
/// the files were staged.
const STALE_HANDOFF_TEXT: &[&str] = &[
    "staged-file list printed in this prompt",
    "captured before",
    "is out of date",
    "for the real list",
    "At handoff it was",
];

/// The two branches of `_pr/_facts.md`'s uncommitted-changes fact.
const CLEAN_TREE: &str = "The working tree is clean.";
const DIRTY_TREE: &str = "These paths have uncommitted changes";

#[track_caller]
fn assert_no_stale_handoff_text(prompt: &str) {
    for fragment in STALE_HANDOFF_TEXT {
        assert!(!prompt.contains(fragment), "stale handoff text `{fragment}`:\n{prompt}");
    }
}

/// `commit.md` states the staged count and lists each staged path.
#[track_caller]
fn assert_staged_list(prompt: &str, staged: &[&str]) {
    let text = collapse_whitespace(prompt);
    assert!(
        text.contains(&format!("There are {} staged files to commit", staged.len())),
        "{prompt}"
    );
    for path in staged {
        assert!(prompt.lines().any(|line| line.trim() == format!("- {path}")), "`{path}` listed:\n{prompt}");
    }
}

fn collapse_whitespace(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[track_caller]
fn assert_route(outcome: &Outcome, succeeded: bool, trail: &[&str]) {
    assert_eq!(
        outcome.code == Some(0),
        succeeded,
        "exit {:?}, expected {}:\n{}",
        outcome.code,
        if succeeded { "success" } else { "failure" },
        outcome.output
    );
    assert_eq!(outcome.trail, trail, "stages that launched an agent:\n{}", outcome.output);
}

// ── Routes ─────────────────────────────────────────────────────────────────

#[test]
fn clean_tree_pushes_and_opens_the_pull_request() {
    let rehearsal = Rehearsal::new("pr-flow-clean", Start::FeatureBranch);
    let outcome = rehearsal.run(&[]);
    assert_route(&outcome, true, &["push", "open"]);
    assert_eq!(rehearsal.status().as_deref(), Some("pr_opened"));
    assert!(rehearsal.remote_branches().contains(&"feat/demo".to_string()));
}

#[test]
fn dirty_tree_with_commit_commits_then_pushes() {
    let rehearsal = Rehearsal::new("pr-flow-dirty-commit", Start::FeatureBranch);
    rehearsal.make_dirty();
    let outcome = rehearsal.run(&["uncommitted=commit"]);
    assert_route(&outcome, true, &["commit", "push", "open"]);
    assert_eq!(rehearsal.status().as_deref(), Some("pr_opened"));
    assert_eq!(rehearsal.git(&["status", "--porcelain"]), "", "the commit took every change");
    // The push stage saw the commit the earlier stage made in the same run.
    assert_eq!(rehearsal.git(&["rev-list", "--count", "origin/main..origin/feat/demo"]), "2");
    // The commit prompt lists what is staged now, not the start-of-run
    // snapshot (where `wip.txt` was untracked), so no stale-list caveat rides
    // in `message`.
    let commit = rehearsal.prompt("commit");
    assert_no_stale_handoff_text(&commit);
    assert!(
        commit.contains("These are the uncommitted changes that were in the working tree when a pull request was requested."),
        "{commit}"
    );
    assert_staged_list(&commit, &["wip.txt"]);
    // The push prompt's facts are read after the commit.
    let push = rehearsal.prompt("push");
    assert!(push.contains(CLEAN_TREE), "{push}");
    assert!(!push.contains(DIRTY_TREE), "{push}");
    assert!(!push.contains("wip.txt"), "{push}");
}

/// A path that appears only after the run started, left behind by the commit
/// stage, is what the push stage's facts report.
#[test]
fn a_path_the_commit_stage_leaves_dirty_is_listed_in_the_push_prompt() {
    let mut rehearsal = Rehearsal::new("pr-flow-commit-leaves", Start::FeatureBranch);
    rehearsal.scenario.commit_leaves = Some("leftover.txt".to_string());
    rehearsal.make_dirty();
    let outcome = rehearsal.run(&["uncommitted=commit"]);
    assert_route(&outcome, true, &["commit", "push", "open"]);
    let push = rehearsal.prompt("push");
    let facts = collapse_whitespace(&push);
    assert!(facts.contains(&format!("{DIRTY_TREE} (staged, unstaged, or untracked): leftover.txt.")), "{push}");
    assert!(!push.contains(CLEAN_TREE), "{push}");
    assert!(!push.contains("wip.txt"), "the committed path is gone:\n{push}");
}

#[test]
fn dirty_tree_with_abort_stops_before_any_agent() {
    let rehearsal = Rehearsal::new("pr-flow-dirty-abort", Start::FeatureBranch);
    rehearsal.make_dirty();
    let outcome = rehearsal.run(&["uncommitted=abort"]);
    assert_route(&outcome, false, &[]);
    assert_eq!(rehearsal.status().as_deref(), Some("error"));
    assert!(outcome.output.contains("stopped at the caller's request"), "{}", outcome.output);
    assert!(rehearsal.git(&["status", "--porcelain"]).contains("wip.txt"));
}

#[test]
fn dirty_tree_without_an_answer_or_a_terminal_names_the_question() {
    let rehearsal = Rehearsal::new("pr-flow-dirty-no-tty", Start::FeatureBranch);
    rehearsal.make_dirty();
    let outcome = rehearsal.run(&[]);
    assert_route(&outcome, false, &[]);
    assert!(
        outcome.output.contains("- `uncommitted`: enum(commit|abort)"),
        "the error names the unanswered property:\n{}",
        outcome.output
    );
    assert!(rehearsal.git(&["status", "--porcelain"]).contains("wip.txt"));
}

#[test]
fn starting_on_the_base_branch_pushes_a_new_branch_and_leaves_the_base_alone() {
    let mut rehearsal = Rehearsal::new("pr-flow-base", Start::BaseBranch);
    rehearsal.scenario.create_branch = Some("feat/from-base".to_string());
    let base_before = run_git(&rehearsal.scenario.git, &rehearsal.bare, &["rev-parse", "main"]);
    let outcome = rehearsal.run(&["branch=feat/from-base"]);
    assert_route(&outcome, true, &["push", "open"]);
    assert_eq!(
        rehearsal.report("branch").and_then(|value| value.as_str().map(str::to_string)).as_deref(),
        Some("feat/from-base")
    );
    assert!(rehearsal.remote_branches().contains(&"feat/from-base".to_string()));
    assert_eq!(run_git(&rehearsal.scenario.git, &rehearsal.bare, &["rev-parse", "main"]), base_before);
}

#[test]
fn blocked_push_with_ask_triages_then_fixes_and_commits() {
    let mut rehearsal = Rehearsal::new("pr-flow-blocked-ask", Start::FeatureBranch);
    rehearsal.scenario.push_passes = false;
    let outcome = rehearsal.run(&["on_failure=ask"]);
    assert_route(&outcome, true, &["push", "diagnose", "triage", "fix", "commit"]);
    assert_eq!(rehearsal.git(&["log", "-1", "--format=%s"]), "rehearsal commit");
    assert!(!rehearsal.remote_branches().contains(&"feat/demo".to_string()), "the hook blocked the push");
}

#[test]
fn blocked_push_with_fix_fixes_and_commits_without_asking() {
    let mut rehearsal = Rehearsal::new("pr-flow-blocked-fix", Start::FeatureBranch);
    rehearsal.scenario.push_passes = false;
    let outcome = rehearsal.run(&["on_failure=fix"]);
    assert_route(&outcome, true, &["push", "diagnose", "fix", "commit"]);
    assert_eq!(rehearsal.report("fix_attempts").and_then(|value| value.as_u64()), Some(1));
    assert!(outcome.output.contains("Run the PR prompt again"), "{}", outcome.output);
    let commit = rehearsal.prompt("commit");
    assert_no_stale_handoff_text(&commit);
    assert!(
        commit.contains("These staged files repair local test failures that blocked the push of `feat/demo`. Repaired on attempt 1."),
        "the handoff carries the fix report's summary:\n{commit}"
    );
    assert_staged_list(&commit, &["fixed.txt"]);
}

#[test]
fn blocked_push_with_stop_ends_with_the_diagnosis() {
    let mut rehearsal = Rehearsal::new("pr-flow-blocked-stop", Start::FeatureBranch);
    rehearsal.scenario.push_passes = false;
    let outcome = rehearsal.run(&["on_failure=stop"]);
    assert_route(&outcome, true, &["push", "diagnose"]);
    assert_eq!(rehearsal.status().as_deref(), Some("diagnosed"));
    assert!(outcome.output.contains("is paused with its diagnosis"), "{}", outcome.output);
}

#[test]
fn a_fix_that_verifies_on_a_later_attempt_is_committed() {
    let mut rehearsal = Rehearsal::new("pr-flow-fix-later", Start::FeatureBranch);
    rehearsal.scenario.push_passes = false;
    rehearsal.scenario.fix_attempts = vec![false, true];
    let outcome = rehearsal.run(&["on_failure=fix"]);
    assert_route(&outcome, true, &["push", "diagnose", "fix", "fix", "commit"]);
    assert_eq!(rehearsal.report("fix_attempts").and_then(|value| value.as_u64()), Some(2));
    assert_eq!(rehearsal.git(&["log", "-1", "--format=%s"]), "rehearsal commit");
}

#[test]
fn a_fix_that_never_verifies_fails_after_its_budget() {
    let mut rehearsal = Rehearsal::new("pr-flow-fix-never", Start::FeatureBranch);
    rehearsal.scenario.push_passes = false;
    rehearsal.scenario.fix_attempts = vec![false, false, false];
    let outcome = rehearsal.run(&["on_failure=fix"]);
    assert_route(&outcome, false, &["push", "diagnose", "fix", "fix", "fix"]);
    assert_eq!(rehearsal.report("fix_attempts").and_then(|value| value.as_u64()), Some(3));
    assert_eq!(
        outcome.output.matches("the fix did not verify within 3 attempts").count(),
        1,
        "one rendered error, no duplicate warning:\n{}",
        outcome.output
    );
    // `fix.md` once also raised a `warn` repeating the exhaustion message,
    // which is not rendered to the terminal.
    assert!(
        !collapse_whitespace(&outcome.output).contains("are still not fixed after 3 attempts"),
        "the exhaustion is not repeated as a warning:\n{}",
        outcome.output
    );
}

/// The push stage's facts come from `_pr/_facts.md`. A program a fact needs
/// that is missing stops the stage before the agent launches, exactly as the
/// same shell block written inline in `push.md` does.
#[test]
fn a_fact_that_cannot_be_gathered_stops_the_push_stage() {
    let rehearsal = Rehearsal::new("pr-flow-facts-missing", Start::FeatureBranch);
    rehearsal.install_command_stubs(false);
    let outcome = rehearsal.run(&[]);
    assert_route(&outcome, false, &[]);
    assert!(outcome.output.contains("_facts.md"), "the error names the partial:\n{}", outcome.output);
    assert!(!outcome.output.contains("Could not transclude"), "{}", outcome.output);
}
