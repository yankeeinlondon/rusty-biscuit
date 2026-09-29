//! `real_` tier: Codex's managed app-server launch through Claudine's actual
//! wrapper (`claudine codex "<task>"`), against the installed Codex and a
//! deterministic scripted model (`tests/common/codex_model.rs`). Every run
//! has its own `CODEX_HOME`, so no user configuration, credential, history,
//! or session is involved, and no network beyond loopback.
//!
//! What only a real Codex can show:
//!
//! - an ordinary structured run is carried by the app-server (the model sees
//!   the `claudine` originator, not `codex_exec`) with exec's rendered answer
//!   and exit code, and no fallback;
//! - the same task forced onto `codex exec` (an option with no app-server
//!   equivalent) answers the same way, and a failing model fails both;
//! - a run exec refuses (outside a Git repository without
//!   `--skip-git-repo-check`) is still refused, by exec itself;
//! - automatic repetition help reaches the model through the production
//!   wrapper, the model recovers, and the run ends normally.
//!
//! ## How to run
//!
//! Opt-in: skips unless `CLAUDINE_CONTRACT_REAL=1` and `codex` is on `PATH`.
//!
//! ```sh
//! just test-real real_codex_app_server::   # in claudine/
//! ```

use std::path::PathBuf;
use std::process::{Child, Output};
use std::time::{Duration, Instant};

use crate::common;
use common::CliProcessFixture;
use common::codex_model::{Reply, ScriptedModel};

/// Inside nextest's per-test limit, so a stuck run fails with its output.
const RUN_DEADLINE: Duration = Duration::from_secs(60);

/// The installed Codex, or `None` (with the reason printed) when this opt-in
/// tier cannot run here.
fn real_codex() -> Option<PathBuf> {
    if std::env::var("CLAUDINE_CONTRACT_REAL").as_deref() != Ok("1") {
        eprintln!("skipping real_codex_app_server (set CLAUDINE_CONTRACT_REAL=1 to run)");
        return None;
    }
    let Ok(codex) = which::which("codex") else {
        eprintln!("skipping real_codex_app_server (binary `codex` not on PATH)");
        return None;
    };
    let version = std::process::Command::new(&codex).arg("--version").output().expect("codex --version");
    eprintln!("real_codex_app_server: {} is {}", codex.display(), String::from_utf8_lossy(&version.stdout).trim());
    Some(codex)
}

/// One disposable Codex project.
struct CodexProject {
    fixture: CliProcessFixture,
    codex_home: PathBuf,
}

impl CodexProject {
    fn new(name: &str) -> Self {
        let fixture = CliProcessFixture::named(name);
        fixture.seed_user_config();
        let codex_home = fixture.workspace_path().join("codex-home");
        std::fs::create_dir_all(&codex_home).unwrap();
        Self { fixture, codex_home }
    }

    /// `claudine codex -- <options> <task>` against `model`. Codex is a Node
    /// launcher, so the child keeps the host `PATH`.
    fn command(&self, model: &ScriptedModel, options: &[&str], task: &str) -> std::process::Command {
        let mut command = self.fixture.command_builder().host_path().build_std();
        command.args(["codex", "--"]).args(options).args(model.codex_args()).arg(task);
        command.env("CODEX_HOME", &self.codex_home).env_remove("CODEX_SQLITE_HOME").env_remove("OPENAI_API_KEY");
        command
    }

    fn run(&self, model: &ScriptedModel, options: &[&str], task: &str) -> Output {
        let mut child = self
            .command(model, options, task)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .expect("launch claudine");
        wait_for(&mut child, "the wrapped Codex run to end");
        child.wait_with_output().unwrap()
    }
}

/// Waits for `child` to exit; kills it and fails the test, with its output,
/// past the run deadline.
fn wait_for(child: &mut Child, what: &str) {
    let until = Instant::now() + RUN_DEADLINE;
    while child.try_wait().unwrap().is_none() {
        if Instant::now() >= until {
            let _ = child.kill();
            let output = std::mem::replace(child, spawn_nothing()).wait_with_output().unwrap();
            panic!("timed out waiting for {what}\nstdout:\n{}\nstderr:\n{}", text(&output.stdout), text(&output.stderr));
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn spawn_nothing() -> Child {
    std::process::Command::new(std::env::current_exe().unwrap())
        .arg("--list")
        .stdout(std::process::Stdio::null())
        .spawn()
        .unwrap()
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

fn describe(output: &Output) -> String {
    format!("exit {:?}\nstdout:\n{}\nstderr:\n{}", output.status.code(), text(&output.stdout), text(&output.stderr))
}

/// A tool round, then a final answer.
fn tool_then_answer(answer: &'static str) -> ScriptedModel {
    ScriptedModel::scripted(vec![Reply::shell("echo tool-ran"), Reply::message(answer)], Reply::message(answer))
}

const MANAGED: &str = "claudine";
const EXEC: &str = "codex_exec";
const FALLBACK_WARNING: &str = "`codex exec` instead";

#[test]
fn real_codex_app_server_runs_a_task_like_exec() {
    let Some(_codex) = real_codex() else { return };
    let project = CodexProject::new("real-codex-app-server-task");

    let model = tool_then_answer("FINAL_NONCE from the managed run");
    let managed = project.run(&model, &["--skip-git-repo-check"], "run a tool, then answer");
    assert!(managed.status.success(), "{}", describe(&managed));
    assert!(text(&managed.stdout).contains("FINAL_NONCE from the managed run"), "{}", describe(&managed));
    assert!(!text(&managed.stderr).contains(FALLBACK_WARNING), "{}", describe(&managed));
    assert_eq!(model.originators(), [MANAGED, MANAGED], "both model requests came from the managed app-server");
    assert!(model.requests()[1].contains("tool-ran"), "the tool's output reached the model");

    // `--ignore-rules` has no app-server equivalent, so this run stays on exec.
    let model = tool_then_answer("FINAL_NONCE from the exec run");
    let exec = project.run(&model, &["--skip-git-repo-check", "--ignore-rules"], "run a tool, then answer");
    assert!(exec.status.success(), "{}", describe(&exec));
    assert!(text(&exec.stdout).contains("FINAL_NONCE from the exec run"), "{}", describe(&exec));
    assert_eq!(model.originators(), [EXEC, EXEC], "the unmapped run used codex exec");
}

#[test]
fn real_codex_app_server_fails_a_failed_turn_like_exec() {
    let Some(_codex) = real_codex() else { return };
    let project = CodexProject::new("real-codex-app-server-failure");

    let model = ScriptedModel::scripted(Vec::new(), Reply::Fail);
    let managed = project.run(&model, &["--skip-git-repo-check"], "answer");
    assert!(model.originators().iter().all(|origin| origin == MANAGED), "{:?}", model.originators());

    let model = ScriptedModel::scripted(Vec::new(), Reply::Fail);
    let exec = project.run(&model, &["--skip-git-repo-check", "--ignore-rules"], "answer");
    assert!(model.originators().iter().all(|origin| origin == EXEC), "{:?}", model.originators());

    assert!(!exec.status.success(), "exec fails a failed turn: {}", describe(&exec));
    assert_eq!(managed.status.code(), exec.status.code(), "the managed run exits as exec does:\n{}", describe(&managed));
}

/// The model repeats one line per tool round until Claudine's automatic
/// warning reaches it, then changes course. On the granted platform (macOS,
/// Codex 0.157.1) the warning is sent before the unchanged stop limit and the
/// run recovers; anywhere else help is unavailable and the stop still ends it.
#[test]
fn real_codex_app_server_automatic_help_lets_a_repeating_run_recover() {
    let Some(_codex) = real_codex() else { return };
    let project = CodexProject::new("real-codex-app-server-auto-help");
    let model = ScriptedModel::start(Box::new(|requests: &[String]| {
        if requests.last().unwrap().contains("Claudine has detected repeated output") {
            Reply::message("RECOVERED: switching approach instead of repeating.")
        } else {
            Reply::Calls {
                text: "Checking the same file again.".into(),
                calls: vec![("exec_command".into(), serde_json::json!({"cmd": "true"}))],
            }
        }
    }));
    let output = project.run(&model, &["--skip-git-repo-check"], "fix the bug");
    let stderr = text(&output.stderr).split_whitespace().collect::<Vec<_>>().join(" ");
    let granted = cfg!(target_os = "macos") && codex_version().as_deref() == Some("0.157.1");
    if granted {
        assert!(output.status.success(), "the run recovered: {}", describe(&output));
        assert!(stderr.contains("sent the agent a warning"), "{stderr}");
        assert!(!stderr.contains("runaway repetition detected"), "the stop was never reached: {stderr}");
        assert!(text(&output.stdout).contains("RECOVERED"), "{}", describe(&output));
        let warned = model.requests().iter().position(|body| body.contains("Claudine has detected repeated output")).unwrap();
        assert!(warned < 30, "warned before the stop limit, at request {warned}");
        assert_eq!(model.requests().len(), warned + 1, "the turn ended right after the warning");
    } else {
        assert!(!output.status.success(), "{}", describe(&output));
        assert!(stderr.contains("cannot send automatic steering"), "{stderr}");
        assert!(stderr.contains("runaway repetition detected (cycle length 1, 30 repeats)"), "{stderr}");
    }
}

fn codex_version() -> Option<String> {
    let output = std::process::Command::new("codex").arg("--version").output().ok()?;
    text(&output.stdout).split_whitespace().last().map(str::to_string)
}

#[test]
fn real_codex_app_server_leaves_an_untrusted_directory_to_exec() {
    let Some(_codex) = real_codex() else { return };
    let project = CodexProject::new("real-codex-app-server-untrusted");
    let model = tool_then_answer("never");
    let output = project.run(&model, &[], "answer");
    assert!(!output.status.success(), "exec refuses outside a Git repository: {}", describe(&output));
    assert!(model.requests().is_empty(), "no model request was made");
    assert!(!text(&output.stderr).contains(FALLBACK_WARNING), "decided before launch, not a fallback");
}
