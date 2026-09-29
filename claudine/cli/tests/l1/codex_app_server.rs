//! Codex's managed app-server launch through the real wrapper
//! (`claudine codex "<task>"`), against the compiled `claudine-fake-codex`
//! (`tests/bin/fake_codex`) so every path runs deterministically on macOS,
//! Linux, and Windows. The installed-Codex counterparts are
//! `tests/real/real_codex_app_server.rs` and the `real_codex_protocol` tests
//! beside the session.
//!
//! Each case checks what the user sees (exit code, stdout, stderr) and what
//! Codex received (argv and every JSON-RPC message), so a readiness failure is
//! shown to fall back before submission and a submitted task is shown never
//! to be replayed.

use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use serde_json::Value;

use crate::common::CliProcessFixture;

const TASK: &str = "review the plan";

struct Rig {
    fixture: CliProcessFixture,
    dir: PathBuf,
}

impl Rig {
    fn new(name: &str) -> Self {
        let fixture = CliProcessFixture::named(name);
        let target = fixture.bin_dir().join(format!("codex{}", std::env::consts::EXE_SUFFIX));
        fs::copy(biscuit_test_harness::bin_exe!("claudine-fake-codex"), &target).unwrap();
        let dir = fixture.workspace_path().join("fake-codex");
        Self { fixture, dir }
    }

    /// `claudine codex -- <options> <task>` with `plan`.
    fn run(&self, plan: &str, options: &[&str], env: &[(&str, &str)]) -> std::process::Output {
        let mut command = self.fixture.command();
        command
            .args(["codex", "--"])
            .args(options)
            .arg(TASK)
            .env("FAKE_CODEX_DIR", &self.dir)
            .env("FAKE_CODEX_PLAN", plan)
            .timeout(Duration::from_secs(60));
        for (key, value) in env {
            command.env(key, value);
        }
        command.output().unwrap()
    }

    /// A run exec's own trust check allows outside a Git repository.
    fn run_trusted(&self, plan: &str) -> std::process::Output {
        self.run(plan, &["--skip-git-repo-check"], &[])
    }

    fn argv(&self, launch: u32) -> Option<Vec<String>> {
        let text = fs::read_to_string(self.dir.join(format!("argv-{launch}.json"))).ok()?;
        Some(serde_json::from_str(&text).unwrap())
    }

    fn messages(&self, launch: u32) -> Vec<Value> {
        fs::read_to_string(self.dir.join(format!("messages-{launch}.jsonl")))
            .unwrap_or_default()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect()
    }

    /// Method names in order; answers to Codex's own requests appear as
    /// `answer`.
    fn methods(&self, launch: u32) -> Vec<String> {
        self.messages(launch)
            .iter()
            .map(|message| message["method"].as_str().map_or_else(|| "answer".to_string(), str::to_string))
            .collect()
    }
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// `bytes` with every whitespace run collapsed, so an assertion does not
/// depend on where the status renderer wrapped a line.
fn flat(bytes: &[u8]) -> String {
    text(bytes).split_whitespace().collect::<Vec<_>>().join(" ")
}

fn describe(output: &std::process::Output) -> String {
    format!("exit {:?}\nstdout:\n{}\nstderr:\n{}", output.status.code(), text(&output.stdout), text(&output.stderr))
}

const FALLBACK_WARNING: &str = "`codex exec` instead";

#[test]
fn a_task_runs_over_the_app_server_and_the_settled_run_ends() {
    let rig = Rig::new("codex-app-server-ok");
    let output = rig.run_trusted("ok");
    assert!(output.status.success(), "{}", describe(&output));
    assert!(text(&output.stdout).contains(&format!("ACK:{TASK}")), "the final message reaches stdout: {}", describe(&output));
    assert!(!text(&output.stderr).contains(FALLBACK_WARNING), "{}", describe(&output));

    let argv = rig.argv(1).expect("Codex launched");
    assert_eq!(&argv[..3], ["app-server", "--listen", "stdio://"], "{argv:?}");
    for exec_only in ["exec", "--json", "--skip-git-repo-check", "--output-last-message"] {
        assert!(!argv.iter().any(|arg| arg == exec_only), "{exec_only} in {argv:?}");
    }

    assert_eq!(rig.methods(1), ["initialize", "initialized", "thread/start", "turn/start", "thread/read"]);
    let messages = rig.messages(1);
    assert_eq!(messages[2]["params"]["approvalPolicy"], "never", "exec never asks for approval");
    assert_eq!(messages[3]["params"]["input"][0]["text"], TASK, "the task is the one turn");
    assert_eq!(messages[3]["params"]["threadId"], "fake-thread");
    assert!(rig.argv(2).is_none(), "no second launch");
}

#[test]
fn a_refused_initialize_falls_back_to_exec_before_submission() {
    let rig = Rig::new("codex-app-server-refuse-initialize");
    let output = rig.run_trusted("refuse-initialize");
    let stderr = flat(&output.stderr);
    assert!(output.status.success(), "{}", describe(&output));
    assert!(stderr.contains("Codex did not initialize: Codex refused it (fake refusal"), "{stderr}");
    assert!(stderr.contains(FALLBACK_WARNING) && stderr.contains("cannot be steered"), "{stderr}");
    assert_eq!(rig.methods(1), ["initialize"], "the task never reached the app-server");

    let fallback = rig.argv(2).expect("the exec launch ran");
    assert_eq!(fallback[0], "exec", "{fallback:?}");
    assert!(fallback.iter().any(|arg| arg == "--skip-git-repo-check"), "the exec argv is unchanged: {fallback:?}");
    assert_eq!(fs::read_to_string(rig.dir.join("stdin-2.txt")).unwrap(), TASK, "the task is exec's stdin");
    assert!(text(&output.stdout).contains(&format!("ACK:{TASK}")), "{}", describe(&output));
}

#[test]
fn a_child_that_exits_before_it_is_ready_falls_back_with_its_stderr_shown() {
    let rig = Rig::new("codex-app-server-exit-early");
    let output = rig.run_trusted("exit-early");
    let stderr = flat(&output.stderr);
    assert!(output.status.success(), "{}", describe(&output));
    assert!(stderr.contains("fake codex: this build has no app-server"), "{stderr}");
    assert!(stderr.contains(FALLBACK_WARNING), "{stderr}");
    assert_eq!(rig.argv(2).expect("the exec launch ran")[0], "exec");
}

#[test]
fn an_approval_request_is_refused_as_exec_refuses_it_never_approved() {
    let rig = Rig::new("codex-app-server-approval");
    let output = rig.run_trusted("approval");
    assert!(output.status.success(), "{}", describe(&output));
    assert!(text(&output.stdout).contains("ACK:refused"), "{}", describe(&output));
    let answer = rig.messages(1).into_iter().find(|message| message["id"] == 0).expect("the request was answered");
    assert_eq!(answer["error"]["code"], -32000, "{answer}");
    assert!(answer.get("result").is_none(), "never approved: {answer}");
}

#[test]
fn an_unreadable_request_fails_the_run_as_input_required() {
    let rig = Rig::new("codex-app-server-unreadable-request");
    let output = rig.run_trusted("unreadable-request");
    assert!(!output.status.success(), "{}", describe(&output));
    assert!(flat(&output.stderr).contains("could not read"), "{}", describe(&output));
    assert!(!rig.methods(1).iter().any(|method| method == "answer"), "no answer is invented");
}

#[test]
fn a_failed_turn_fails_the_run_like_exec() {
    let rig = Rig::new("codex-app-server-fail");
    let output = rig.run_trusted("fail");
    assert_eq!(output.status.code(), Some(1), "exec exits 1 for a failed turn: {}", describe(&output));
    assert!(flat(&output.stderr).contains("fake model failure"), "{}", describe(&output));
    assert!(rig.argv(2).is_none(), "a submitted task is never replayed");
}

#[test]
fn a_refused_task_fails_the_run_and_is_never_replayed() {
    let rig = Rig::new("codex-app-server-refuse-task");
    let output = rig.run_trusted("refuse-task");
    assert!(!output.status.success(), "{}", describe(&output));
    assert!(flat(&output.stderr).contains("Codex refused the task"), "{}", describe(&output));
    assert_eq!(rig.methods(1).iter().filter(|method| *method == "turn/start").count(), 1);
    assert!(rig.argv(2).is_none(), "no fallback after submission");
}

#[test]
fn options_without_an_app_server_equivalent_stay_on_exec() {
    // Outside a Git repository exec refuses to run without the flag; the
    // decision is made before launch, so it is not a fallback.
    let rig = Rig::new("codex-app-server-untrusted");
    let output = rig.run("ok", &[], &[]);
    assert_eq!(rig.argv(1).expect("Codex launched")[0], "exec");
    assert!(!text(&output.stderr).contains(FALLBACK_WARNING), "{}", describe(&output));

    let rig = Rig::new("codex-app-server-unmapped");
    let output = rig.run("ok", &["--skip-git-repo-check", "--ignore-rules"], &[]);
    assert!(output.status.success(), "{}", describe(&output));
    let argv = rig.argv(1).expect("Codex launched");
    assert_eq!(argv[0], "exec", "{argv:?}");
    assert!(argv.iter().any(|arg| arg == "--ignore-rules"), "{argv:?}");
    assert!(rig.argv(2).is_none());
}

// ---------------------------------------------------------------------------
// Automatic repetition help. The reviewed policy grants `turn/steer` for this
// profile on macOS at Codex 0.157.1 only, so the fake's version decides.
// ---------------------------------------------------------------------------

const SENT_NOTICE: &str = "sent the agent a warning";
const UNAVAILABLE_NOTICE: &str = "cannot send automatic steering to this session";

/// Where the repetition stop is reported, at the default limit's exact count.
fn repetition_stop(stderr: &str) -> Option<usize> {
    stderr.find("runaway repetition detected (cycle length 1, 30 repeats)")
}

fn steers(rig: &Rig) -> Vec<Value> {
    rig.messages(1).into_iter().filter(|message| message["method"] == "turn/steer").collect()
}

#[test]
fn automatic_help_follows_the_granted_version_and_keeps_the_stop_schedule() {
    // An ungranted version: help is unavailable, said once, nothing sent.
    let rig = Rig::new("codex-app-server-auto-ungranted");
    let output = rig.run("repeat", &["--skip-git-repo-check"], &[("FAKE_CODEX_VERSION", "0.157.0")]);
    let stderr = flat(&output.stderr);
    assert!(!output.status.success(), "{}", describe(&output));
    assert_eq!(stderr.matches(UNAVAILABLE_NOTICE).count(), 1, "{stderr}");
    assert!(steers(&rig).is_empty(), "no automatic message without a grant");
    assert!(repetition_stop(&stderr).is_some(), "{stderr}");

    // The granted version.
    let rig = Rig::new("codex-app-server-auto-granted");
    let output = rig.run("repeat", &["--skip-git-repo-check"], &[]);
    let stderr = flat(&output.stderr);
    assert!(!output.status.success(), "the repetition still stops the run: {}", describe(&output));
    let stop = repetition_stop(&stderr).unwrap_or_else(|| panic!("the stop keeps its schedule: {stderr}"));
    if cfg!(target_os = "macos") {
        let steers = steers(&rig);
        assert_eq!(steers.len(), 1, "one warning for one episode: {steers:?}");
        let steer = &steers[0]["params"];
        assert_eq!(steer["expectedTurnId"], "fake-turn", "guarded by the running turn");
        let message = steer["input"][0]["text"].as_str().unwrap();
        assert!(message.starts_with("Claudine has detected repeated output"), "{message}");
        assert!(message.contains("the same line 15 times in a row"), "warned at half the stop limit: {message}");
        let sent = stderr.find(SENT_NOTICE).unwrap_or_else(|| panic!("the confirmed send is reported: {stderr}"));
        assert!(sent < stop, "the warning comes before the stop: {stderr}");
    } else {
        assert!(steers(&rig).is_empty(), "the grant is macOS-only");
        assert_eq!(stderr.matches(UNAVAILABLE_NOTICE).count(), 1, "{stderr}");
    }
}
