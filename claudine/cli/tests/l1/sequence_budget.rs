//! End-to-end enforcement of `claudine sequence --budget-ledger`.
//!
//! Every test drives the real `claudine` binary against a fake `goose`
//! provider compiled from Rust source, so the same fixtures run on macOS,
//! Linux, native Windows, and WSL2 (a `.cmd` shim cannot carry Claudine's
//! prompt argument on Windows). The fake records each launch in a directory
//! and behaves according to a per-launch plan, so assertions count real
//! provider processes rather than trusting the ledger's own arithmetic.
//!
//! No credentials, network, audio, or terminal windows are involved.

use crate::common;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Stdio};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use common::{CliProcessFixture, strip_ansi};
use serde_json::Value;

const EXHAUSTED: i32 = 76;
const BLOCKED: i32 = 77;

/// Behaviors, one per launch (1-based), separated by commas; launches past
/// the end of the plan succeed:
///
/// - `ok` exits 0;
/// - `fail` exits 7;
/// - `sleep:<ms>` sleeps, then exits 0;
/// - `hang` sleeps for ten minutes.
///
/// Each launch writes `launch-<n>.txt` (its prompt) and `pid-<n>.txt` into
/// `FAKE_DIR`, and `done-<n>.txt` only when it exits on its own.
const FAKE_GOOSE: &str = r#"
use std::{env, fs, thread, time::Duration};

fn main() {
    let dir = std::path::PathBuf::from(env::var_os("FAKE_DIR").expect("FAKE_DIR"));
    fs::create_dir_all(&dir).unwrap();
    let mut n = 1;
    while fs::OpenOptions::new().write(true).create_new(true).open(dir.join(format!("pid-{n}.txt"))).is_err() {
        n += 1;
    }
    fs::write(dir.join(format!("pid-{n}.txt")), std::process::id().to_string()).unwrap();
    let mut prompt = String::new();
    let mut previous = None;
    for argument in env::args().skip(1) {
        if previous.as_deref() == Some("-t") {
            prompt = argument.clone();
        }
        previous = Some(argument);
    }
    fs::write(dir.join(format!("launch-{n}.txt")), &prompt).unwrap();
    let plan = env::var("FAKE_PLAN").unwrap_or_default();
    let step = plan.split(',').nth(n - 1).unwrap_or("ok").trim().to_string();
    let code = if step == "fail" {
        7
    } else if step == "hang" {
        thread::sleep(Duration::from_secs(600));
        0
    } else if let Some(ms) = step.strip_prefix("sleep:") {
        thread::sleep(Duration::from_millis(ms.parse().unwrap()));
        0
    } else {
        0
    };
    fs::write(dir.join(format!("done-{n}.txt")), code.to_string()).unwrap();
    std::process::exit(code);
}
"#;

fn fake_goose_binary() -> &'static Path {
    static BINARY: OnceLock<PathBuf> = OnceLock::new();
    BINARY.get_or_init(|| {
        let dir = tempfile::Builder::new()
            .prefix("claudine-budget-fake-")
            .tempdir()
            .unwrap()
            .keep();
        let source = dir.join("goose.rs");
        fs::write(&source, FAKE_GOOSE).unwrap();
        let output = dir.join(format!("goose{}", std::env::consts::EXE_SUFFIX));
        let rustc = std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
        let status = std::process::Command::new(rustc)
            .args(["--edition=2024", "-O"])
            .arg(&source)
            .arg("-o")
            .arg(&output)
            .status()
            .expect("rustc must be available to build the fake provider");
        assert!(status.success(), "failed to compile the fake goose provider");
        output
    })
}

struct Rig {
    fixture: CliProcessFixture,
    fake_dir: PathBuf,
}

impl Rig {
    fn new(name: &str) -> Self {
        let fixture = CliProcessFixture::named(name);
        let target = fixture
            .bin_dir()
            .join(format!("goose{}", std::env::consts::EXE_SUFFIX));
        fs::copy(fake_goose_binary(), &target).unwrap();
        let fake_dir = fixture.cwd().join("fake");
        Self { fixture, fake_dir }
    }

    fn path(&self, relative: &str) -> PathBuf {
        self.fixture.cwd().join(relative)
    }

    fn write(&self, relative: &str, content: &str) -> PathBuf {
        let path = self.path(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, content).unwrap();
        path
    }

    fn init(&self, ledger: &str, platform: &str, seconds: u64, invocations: u64, extra: &[&str]) {
        let output = self
            .fixture
            .command()
            .args(["budget", "init"])
            .arg(self.path(ledger))
            .args(["--run-id", "2026-09-17-0000abcd", "--platform", platform])
            .args(["--max-seconds", &seconds.to_string()])
            .args(["--max-invocations", &invocations.to_string()])
            .args(extra)
            .output()
            .unwrap();
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    }

    fn budget(&self, args: &[&str]) -> std::process::Output {
        self.fixture.command().arg("budget").args(args).output().unwrap()
    }

    fn sequence_command(&self, ledger: &str, sequence: &Path, plan: &str) -> assert_cmd::Command {
        let mut command = self.fixture.command();
        command
            .env("FAKE_DIR", &self.fake_dir)
            .env("FAKE_PLAN", plan)
            .env("CLAUDINE_KILL_GRACE", "1s")
            .args(["sequence", "--goose", "--budget-ledger"])
            .arg(self.path(ledger))
            .arg(sequence);
        command
    }

    fn run(&self, ledger: &str, sequence: &Path, plan: &str) -> (i32, String) {
        let output = self.sequence_command(ledger, sequence, plan).output().unwrap();
        (
            output.status.code().unwrap_or(-1),
            strip_ansi(&String::from_utf8_lossy(&output.stderr)),
        )
    }

    fn spawn(&self, ledger: &str, sequence: &Path, plan: &str) -> Child {
        let mut command = self.fixture.command_std();
        command
            .env("FAKE_DIR", &self.fake_dir)
            .env("FAKE_PLAN", plan)
            .env("CLAUDINE_KILL_GRACE", "1s")
            .args(["sequence", "--goose", "--budget-ledger"])
            .arg(self.path(ledger))
            .arg(sequence)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        command.spawn().unwrap()
    }

    fn ledger(&self, ledger: &str) -> Value {
        serde_json::from_slice(&fs::read(self.path(ledger)).unwrap()).unwrap()
    }

    fn launches(&self) -> usize {
        fs::read_dir(&self.fake_dir)
            .map(|entries| {
                entries
                    .flatten()
                    .filter(|e| e.file_name().to_string_lossy().starts_with("launch-"))
                    .count()
            })
            .unwrap_or(0)
    }

    fn prompt(&self, n: usize) -> String {
        fs::read_to_string(self.fake_dir.join(format!("launch-{n}.txt"))).unwrap()
    }

    fn wait_for(&self, file: &str, within: Duration) -> PathBuf {
        let path = self.fake_dir.join(file);
        let deadline = Instant::now() + within;
        while !path.exists() || fs::read_to_string(&path).unwrap_or_default().is_empty() {
            assert!(Instant::now() < deadline, "timed out waiting for {}", path.display());
            std::thread::sleep(Duration::from_millis(50));
        }
        path
    }
}

fn event_kinds(ledger: &Value) -> Vec<String> {
    ledger["events"]
        .as_array()
        .unwrap()
        .iter()
        .map(|event| event["kind"].as_str().unwrap().to_string())
        .collect()
}

fn count(ledger: &Value, kind: &str) -> usize {
    event_kinds(ledger).iter().filter(|k| *k == kind).count()
}

fn process_alive(pid: u32) -> bool {
    use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};
    let pid = Pid::from_u32(pid);
    let mut system = System::new();
    system.refresh_processes_specifics(
        ProcessesToUpdate::Some(&[pid]),
        true,
        ProcessRefreshKind::nothing(),
    );
    system
        .process(pid)
        .is_some_and(|process| process.status() != sysinfo::ProcessStatus::Zombie)
}

/// A research-shaped run: separately prepared discovery and reconciliation
/// inputs, a failing reconciliation recovered by one lifecycle retry, a
/// source-list pass, a validation shell task, and an independent reviewer.
fn research_sequence(rig: &Rig) -> PathBuf {
    rig.write(
        "inputs/discovery/prompt.md",
        "Discover sources. DISCOVERY-INPUT: identification, questions, schema.\n",
    );
    rig.write(
        "inputs/reconcile/prompt.md",
        "---\nfailure:\n  stack:\n    - action:\n        - action: retry\n          max_attempts: 1\n---\n\
         Reconcile. RECONCILE-INPUT PRIOR-PROSE CURATED-LINK.\n",
    );
    rig.write("inputs/maintain/prompt.md", "Maintain the curated list.\n");
    rig.write("inputs/review/prompt.md", "Independently review the evidence.\n");
    rig.write(
        "run.md",
        "---\nsequence:\n  - name: discovery\n    prompt: \"./inputs/discovery/prompt.md\"\n  \
         - name: reconcile\n    prompt: \"./inputs/reconcile/prompt.md\"\n  \
         - name: maintain\n    prompt: \"./inputs/maintain/prompt.md\"\n  \
         - name: validate\n    shell: \"echo validated\"\n  \
         - name: review\n    prompt: \"./inputs/review/prompt.md\"\n---\n",
    )
}

/// Three one-launch steps.
fn three_steps(rig: &Rig) -> PathBuf {
    rig.write("steps/one.md", "Step one.\n");
    rig.write("steps/two.md", "Step two.\n");
    rig.write("steps/three.md", "Step three.\n");
    rig.write(
        "three.md",
        "---\nsequence:\n  - name: one\n    prompt: \"./steps/one.md\"\n  \
         - name: two\n    prompt: \"./steps/two.md\"\n  \
         - name: three\n    prompt: \"./steps/three.md\"\n---\n",
    )
}

#[test]
fn init_requires_both_positive_limits_and_a_sequence_refuses_a_missing_ledger() {
    let rig = Rig::new("budget-init");
    let ledger = rig.path("runs/discord/budget.json");
    let ledger_arg = ledger.to_str().unwrap();
    for args in [
        vec!["--max-seconds", "60"],
        vec!["--max-invocations", "5"],
        vec!["--max-seconds", "0", "--max-invocations", "5"],
        vec!["--max-seconds", "60", "--max-invocations", "0"],
    ] {
        let mut full = vec!["init", ledger_arg, "--run-id", "r", "--platform", "discord"];
        full.extend(args.iter().copied());
        let output = rig.budget(&full);
        assert_eq!(output.status.code(), Some(2), "{args:?} must be a usage error");
        assert!(!ledger.exists(), "{args:?} must not create a ledger");
    }

    let sequence = three_steps(&rig);
    let (code, stderr) = rig.run("runs/discord/budget.json", &sequence, "");
    assert_eq!(code, 1, "{stderr}");
    assert!(stderr.contains("claudine budget init"), "{stderr}");
    assert_eq!(rig.launches(), 0, "no worker may launch without a ledger");
}

#[test]
fn every_pass_retry_and_reviewer_launch_is_charged_to_one_ledger() {
    let rig = Rig::new("budget-research");
    rig.init("budget.json", "discord", 120, 10, &[]);
    let sequence = research_sequence(&rig);

    // Launch 2 (the first reconciliation) fails; its lifecycle retry is launch
    // 3. `--yolo` approves the validation shell task without a terminal.
    let output = rig
        .sequence_command("budget.json", &sequence, "ok,fail,ok,ok,ok")
        .arg("--yolo")
        .output()
        .unwrap();
    let code = output.status.code().unwrap_or(-1);
    let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr));
    assert_eq!(code, 0, "{stderr}");
    assert_eq!(rig.launches(), 5);

    let ledger = rig.ledger("budget.json");
    assert_eq!(ledger["used"]["invocations"], 5, "{ledger:#}");
    assert!(ledger["used"]["active_ms"].as_u64().unwrap() > 0);
    assert_eq!(count(&ledger, "admitted"), 5);
    assert_eq!(count(&ledger, "spawned"), 5, "every child identity is recorded");
    assert_eq!(count(&ledger, "settled"), 5);
    assert_eq!(ledger["state"], "suspended", "a finished run awaits human review");
    assert_eq!(ledger["stop_reason"], "awaiting human review");
    assert_eq!(ledger["runs"], 1);
    assert_eq!(ledger["stage"], "review");
    assert!(ledger["in_flight"].as_array().unwrap().is_empty());
    assert!(ledger["segment"].is_null());
    let stages: Vec<&str> = ledger["events"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|e| e["kind"] == "stage")
        .map(|e| e["detail"].as_str().unwrap())
        .collect();
    assert_eq!(stages, ["discovery", "reconcile", "maintain", "validate", "review"]);

    // Discovery saw only its prepared input; reconciliation saw the rest.
    let discovery = rig.prompt(1);
    assert!(discovery.contains("DISCOVERY-INPUT"), "{discovery}");
    for leaked in ["PRIOR-PROSE", "CURATED-LINK", "RECONCILE-INPUT"] {
        assert!(!discovery.contains(leaked), "discovery received {leaked}");
    }
    for n in [2, 3] {
        let reconcile = rig.prompt(n);
        assert!(reconcile.contains("PRIOR-PROSE") && reconcile.contains("CURATED-LINK"));
    }
    assert!(stderr.contains("5/10 invocations"), "{stderr}");
}

#[test]
fn exhaustion_before_launch_stops_dispatch_and_only_a_grant_adds_allowance() {
    let rig = Rig::new("budget-exhaust");
    rig.init("budget.json", "slack", 120, 2, &[]);
    let sequence = three_steps(&rig);

    let (code, stderr) = rig.run("budget.json", &sequence, "");
    assert_eq!(code, EXHAUSTED, "{stderr}");
    assert_eq!(rig.launches(), 2, "the third launch must be refused");
    assert!(stderr.contains("remote model work and billing may continue"), "{stderr}");
    let ledger = rig.ledger("budget.json");
    assert_eq!(ledger["state"], "exhausted");
    assert_eq!(ledger["stage"], "three", "the incomplete stage is preserved");
    assert_eq!(ledger["used"]["invocations"], 2);
    assert!(
        ledger["stop_reason"]
            .as_str()
            .unwrap()
            .starts_with("budget exhausted at stage `three`: 2/2 invocations"),
        "{ledger:#}"
    );
    // With no invocation left the run stops at the step boundary, before the
    // step prepares a launch.
    assert_eq!(count(&ledger, "exhausted"), 1);
    assert_eq!(count(&ledger, "refused"), 0);

    // A restart does not reset the allowance.
    let (code, _) = rig.run("budget.json", &sequence, "");
    assert_eq!(code, EXHAUSTED);
    assert_eq!(rig.launches(), 2);
    assert_eq!(rig.ledger("budget.json")["runs"], 1);

    // Resumption cannot reopen it; a recorded grant can.
    assert!(!rig.budget(&["resume", "budget.json", "--operator", "ken"]).status.success());
    let ledger_path = rig.path("budget.json");
    let ledger_arg = ledger_path.to_str().unwrap();
    let granted = rig.budget(&[
        "grant", ledger_arg, "--operator", "ken", "--reason", "one recovery", "--invocations", "1",
    ]);
    assert!(granted.status.success(), "{}", String::from_utf8_lossy(&granted.stderr));
    let (code, _) = rig.run("budget.json", &sequence, "");
    assert_eq!(code, EXHAUSTED);
    assert_eq!(rig.launches(), 3, "exactly the granted launch ran");
    let ledger = rig.ledger("budget.json");
    assert_eq!(ledger["used"]["invocations"], 3);
    assert_eq!(ledger["runs"], 2);
    assert_eq!(ledger["grants"][0]["operator"], "ken");
}

#[test]
fn a_running_worker_is_stopped_at_the_shared_deadline() {
    let rig = Rig::new("budget-deadline");
    rig.init("budget.json", "telegram", 2, 10, &["--heartbeat", "0.2s"]);
    let sequence = three_steps(&rig);

    let started = Instant::now();
    let (code, stderr) = rig.run("budget.json", &sequence, "hang");
    let elapsed = started.elapsed();
    assert_eq!(code, EXHAUSTED, "{stderr}");
    assert!(elapsed < Duration::from_secs(60), "the worker outlived the budget: {elapsed:?}");
    assert_eq!(rig.launches(), 1, "no later step may launch");
    assert!(!rig.fake_dir.join("done-1.txt").exists(), "the worker was stopped, not finished");
    let pid: u32 = fs::read_to_string(rig.fake_dir.join("pid-1.txt")).unwrap().parse().unwrap();
    assert!(!process_alive(pid), "the local worker process must be gone");

    let ledger = rig.ledger("budget.json");
    assert_eq!(ledger["state"], "exhausted");
    assert_eq!(ledger["stage"], "one");
    assert!(ledger["used"]["active_ms"].as_u64().unwrap() >= 2_000, "{ledger:#}");
    assert_eq!(ledger["used"]["invocations"], 1);
    let settled = ledger["events"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["kind"] == "settled")
        .unwrap();
    assert!(
        settled["detail"].as_str().unwrap().contains("remote model work and billing unverified"),
        "{settled:#}"
    );
}

#[test]
fn automatic_retry_backoff_is_charged_and_capped_to_the_remaining_budget() {
    let rig = Rig::new("budget-backoff");
    rig.init("budget.json", "whatsapp", 3, 10, &["--heartbeat", "0.2s"]);
    rig.write(
        "retry.md",
        "---\nfailure:\n  stack:\n    - action:\n        - action: retry\n          max_attempts: 3\n          delay: 60s\n---\nWork.\n",
    );
    let sequence = rig.write(
        "run.md",
        "---\nsequence:\n  - name: work\n    prompt: \"./retry.md\"\n---\n",
    );

    let started = Instant::now();
    let (code, stderr) = rig.run("budget.json", &sequence, "fail,fail,fail,fail");
    assert_eq!(code, EXHAUSTED, "{stderr}");
    assert!(started.elapsed() < Duration::from_secs(40), "the 60 s backoff was not capped");
    assert_eq!(rig.launches(), 1, "the retry after the capped wait is refused");
    let ledger = rig.ledger("budget.json");
    assert!(ledger["used"]["active_ms"].as_u64().unwrap() >= 3_000, "the wait is charged");
    assert_eq!(ledger["used"]["invocations"], 1);
    assert_eq!(count(&ledger, "refused"), 1);
}

#[test]
fn a_suspended_ledger_is_not_charged_and_needs_an_operator_to_resume() {
    let rig = Rig::new("budget-suspend");
    rig.init("budget.json", "signal", 120, 10, &[]);
    let sequence = three_steps(&rig);

    let (code, stderr) = rig.run("budget.json", &sequence, "");
    assert_eq!(code, 0, "{stderr}");
    let after_first = rig.ledger("budget.json");
    assert_eq!(after_first["state"], "suspended");

    std::thread::sleep(Duration::from_millis(1_200));
    let (code, stderr) = rig.run("budget.json", &sequence, "");
    assert_eq!(code, BLOCKED, "{stderr}");
    assert!(stderr.contains("suspended"), "{stderr}");
    assert_eq!(rig.launches(), 3);
    let blocked = rig.ledger("budget.json");
    assert_eq!(blocked["used"], after_first["used"], "suspension time is not charged");

    let ledger_path = rig.path("budget.json");
    let resumed = rig.budget(&["resume", ledger_path.to_str().unwrap(), "--operator", "ken"]);
    assert!(resumed.status.success());
    let (code, _) = rig.run("budget.json", &sequence, "");
    assert_eq!(code, 0);
    let ledger = rig.ledger("budget.json");
    assert_eq!(ledger["used"]["invocations"], 6, "consumption accumulates across runs");
    assert_eq!(ledger["runs"], 2);

    // The explicit operator pause behaves the same.
    let resumed = rig.budget(&["resume", ledger_path.to_str().unwrap(), "--operator", "ken"]);
    assert!(resumed.status.success());
    let suspended = rig.budget(&["suspend", ledger_path.to_str().unwrap(), "--reason", "legal review"]);
    assert!(suspended.status.success());
    let (code, _) = rig.run("budget.json", &sequence, "");
    assert_eq!(code, BLOCKED);
    let shown = rig.budget(&["show", ledger_path.to_str().unwrap(), "--json"]);
    let shown: Value = serde_json::from_slice(&shown.stdout).unwrap();
    assert_eq!(shown["stop_reason"], "legal review");
    assert!(!String::from_utf8_lossy(&rig.budget(&["show", ledger_path.to_str().unwrap(), "--json"]).stdout).contains('\u{1b}'));
}

#[test]
fn a_crashed_runner_is_charged_conservatively_and_its_orphan_is_stopped() {
    let rig = Rig::new("budget-crash");
    rig.init("budget.json", "discord", 600, 10, &["--heartbeat", "0.2s"]);
    let sequence = three_steps(&rig);

    let mut runner = rig.spawn("budget.json", &sequence, "hang");
    let pid_file = rig.wait_for("pid-1.txt", Duration::from_secs(60));
    let worker: u32 = fs::read_to_string(pid_file).unwrap().parse().unwrap();
    // Let a few heartbeats land, then kill the runner without any cleanup.
    std::thread::sleep(Duration::from_millis(800));
    runner.kill().unwrap();
    runner.wait().unwrap();

    let crashed = rig.ledger("budget.json");
    assert_eq!(crashed["state"], "active", "{crashed:#}");
    assert_eq!(crashed["in_flight"][0]["pid"], worker);
    let persisted_ms = crashed["used"]["active_ms"].as_u64().unwrap();

    // A restart recovers the crash but refuses to launch until resumed.
    let (code, stderr) = rig.run("budget.json", &sequence, "");
    assert_eq!(code, BLOCKED, "{stderr}");
    assert!(stderr.contains("interrupted"), "{stderr}");
    assert_eq!(rig.launches(), 1);
    let recovered = rig.ledger("budget.json");
    assert_eq!(recovered["state"], "interrupted");
    assert_eq!(recovered["used"]["invocations"], 1, "the crashed launch is not refunded");
    assert!(recovered["used"]["active_ms"].as_u64().unwrap() >= persisted_ms + 200);
    assert!(recovered["in_flight"].as_array().unwrap().is_empty());
    assert_eq!(count(&recovered, "crash_recovered"), 1);
    // On Unix the orphan survives its wrapper and recovery must kill it; on
    // Windows the Job Object already took it down with the wrapper.
    let deadline = Instant::now() + Duration::from_secs(10);
    while process_alive(worker) {
        assert!(Instant::now() < deadline, "orphaned worker {worker} is still running");
        std::thread::sleep(Duration::from_millis(100));
    }
    let reason = recovered["stop_reason"].as_str().unwrap();
    #[cfg(unix)]
    assert!(reason.contains("orphaned process tree terminated"), "{reason}");
    #[cfg(windows)]
    assert!(
        reason.contains("orphaned process tree terminated") || reason.contains("no longer running"),
        "{reason}"
    );

    let ledger_path = rig.path("budget.json");
    assert!(rig.budget(&["resume", ledger_path.to_str().unwrap(), "--operator", "ken"]).status.success());
    let (code, stderr) = rig.run("budget.json", &sequence, "");
    assert_eq!(code, 0, "{stderr}");
    let ledger = rig.ledger("budget.json");
    assert_eq!(ledger["used"]["invocations"], 4);
    assert_eq!(ledger["runs"], 2);
}

#[test]
fn runs_sharing_an_exclusive_lock_execute_one_platform_at_a_time() {
    let rig = Rig::new("budget-exclusive");
    rig.init("runs/discord/budget.json", "discord", 120, 10, &["--exclusive-lock", "../fleet.lock"]);
    rig.init("runs/slack/budget.json", "slack", 120, 10, &["--exclusive-lock", "../fleet.lock"]);
    rig.write("one.md", "One.\n");
    let sequence = rig.write(
        "run.md",
        "---\nsequence:\n  - name: only\n    prompt: \"./one.md\"\n---\n",
    );

    let mut discord = rig.spawn("runs/discord/budget.json", &sequence, "sleep:3000");
    rig.wait_for("pid-1.txt", Duration::from_secs(60));
    let (code, stderr) = rig.run("runs/slack/budget.json", &sequence, "");
    assert_eq!(code, BLOCKED, "{stderr}");
    assert!(stderr.contains("held by another run"), "{stderr}");
    assert_eq!(rig.launches(), 1, "the second platform must not launch");
    assert_eq!(rig.ledger("runs/slack/budget.json")["used"]["invocations"], 0);
    assert_eq!(rig.ledger("runs/slack/budget.json")["state"], "stopped");

    assert!(discord.wait().unwrap().success());
    let (code, stderr) = rig.run("runs/slack/budget.json", &sequence, "");
    assert_eq!(code, 0, "{stderr}");
    assert_eq!(rig.ledger("runs/slack/budget.json")["used"]["invocations"], 1);
}

#[test]
fn a_ledger_refuses_interactive_and_dry_run_modes_before_charging() {
    let rig = Rig::new("budget-modes");
    rig.init("budget.json", "discord", 120, 10, &[]);
    let sequence = three_steps(&rig);
    let before = rig.ledger("budget.json");
    for flag in ["--interactive", "--dry-run"] {
        let output = rig
            .fixture
            .command()
            .env("FAKE_DIR", &rig.fake_dir)
            .args(["sequence", "--goose", flag, "--budget-ledger"])
            .arg(rig.path("budget.json"))
            .arg(&sequence)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(1), "{flag}");
        let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr));
        assert!(stderr.contains("--budget-ledger cannot be used with"), "{flag}: {stderr}");
    }
    assert_eq!(rig.launches(), 0);
    assert_eq!(rig.ledger("budget.json"), before, "a refused mode touches nothing");
}

#[test]
fn without_a_ledger_a_sequence_writes_no_budget_state() {
    let rig = Rig::new("budget-none");
    let sequence = three_steps(&rig);
    let output = rig
        .fixture
        .command()
        .env("FAKE_DIR", &rig.fake_dir)
        .args(["sequence", "--goose"])
        .arg(&sequence)
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(rig.launches(), 3);
    let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr));
    assert!(!stderr.contains("budget:"), "{stderr}");
    let stray: Vec<_> = walk(rig.fixture.cwd())
        .into_iter()
        .filter(|path| {
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            name.ends_with(".lock") || name == "budget.json"
        })
        .collect();
    assert!(stray.is_empty(), "{stray:?}");
}

fn walk(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    for entry in fs::read_dir(root).unwrap().flatten() {
        let path = entry.path();
        if path.is_dir() {
            found.extend(walk(&path));
        } else {
            found.push(path);
        }
    }
    found
}
