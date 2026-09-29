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
//!   wrapper, the model recovers, and the run ends normally;
//! - a manual `claudine steer --session` from a separate process reaches the
//!   running wrapper through a real local Rendezvous daemon, is delivered to
//!   the model exactly once, and is never replayed after the run ends
//!   (`manual_steer`, which needs the `daemon-tests` feature).
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

/// The model ignores the warning and keeps repeating: the warning is sent
/// once, and the run still stops at the unchanged limit.
#[test]
fn real_codex_app_server_continued_repetition_still_stops_at_the_limit() {
    let Some(_codex) = real_codex() else { return };
    let project = CodexProject::new("real-codex-app-server-auto-ignored");
    let model = ScriptedModel::start(Box::new(|_: &[String]| Reply::Calls {
        text: "Checking the same file again.".into(),
        calls: vec![("exec_command".into(), serde_json::json!({"cmd": "true"}))],
    }));
    let output = project.run(&model, &["--skip-git-repo-check"], "fix the bug");
    let stderr = text(&output.stderr).split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(!output.status.success(), "{}", describe(&output));
    let stop = stderr
        .find("runaway repetition detected (cycle length 1, 30 repeats)")
        .unwrap_or_else(|| panic!("the stop keeps its schedule: {stderr}"));
    let warned = model.requests().iter().filter(|body| body.contains("Claudine has detected repeated output")).count();
    if cfg!(target_os = "macos") && codex_version().as_deref() == Some("0.157.1") {
        let sent = stderr.find("sent the agent a warning").unwrap_or_else(|| panic!("{stderr}"));
        assert!(sent < stop, "the warning comes before the stop: {stderr}");
        assert!(warned >= 1, "the warning reached the model");
        let first = model.requests().iter().position(|body| body.contains("Claudine has detected repeated output")).unwrap();
        assert_eq!(
            model.requests()[first].matches("Claudine has detected repeated output").count(),
            1,
            "one warning for one episode"
        );
    } else {
        assert_eq!(warned, 0, "no grant here: {stderr}");
        assert!(stderr.contains("cannot send automatic steering"), "{stderr}");
    }
}

fn codex_version() -> Option<String> {
    let output = std::process::Command::new("codex").arg("--version").output().ok()?;
    text(&output.stdout).split_whitespace().last().map(str::to_string)
}

/// A manual send from `claudine steer`, routed by a real local daemon to a
/// real `claudine codex` run.
#[cfg(feature = "daemon-tests")]
mod manual_steer {
    use super::*;
    use rendezvous_core::local_endpoint::test_support::{endpoint_env_value, private_endpoint};
    use serde_json::Value;

    const NONCE: &str = "STEER_NONCE_8d41";
    /// The secret half of the message: it reaches the model, and the audit
    /// log keeps only its mask.
    const SECRET: &str = "ghp_0123456789abcdefghij0123456789abcd";

    /// The daemon in this test process, on an endpoint private to it.
    struct Daemon {
        runtime: tokio::runtime::Runtime,
        handle: Option<rendezvous_daemon::server::ServerHandle>,
        endpoint: std::ffi::OsString,
        _dir: tempfile::TempDir,
    }

    impl Daemon {
        fn start() -> Self {
            // A short directory: a Unix socket path has a ~100-byte limit.
            let dir = tempfile::tempdir().unwrap();
            let endpoint = private_endpoint(dir.path(), "steer");
            let runtime = tokio::runtime::Builder::new_multi_thread().enable_all().build().unwrap();
            let config = rendezvous_daemon::server::DaemonConfig::with_data_dir(dir.path().join("data"))
                .with_in_memory_projection()
                .without_networking();
            let handle = {
                let _enter = runtime.enter();
                rendezvous_daemon::local_transport::spawn_local_server(endpoint.clone(), config).expect("spawn daemon")
            };
            runtime.block_on(async {
                let until = Instant::now() + Duration::from_secs(10);
                while rendezvous_client::connect(&endpoint).await.is_err() {
                    assert!(Instant::now() < until, "the daemon never accepted a connection");
                    tokio::time::sleep(Duration::from_millis(25)).await;
                }
            });
            Self { runtime, handle: Some(handle), endpoint: endpoint_env_value(&endpoint), _dir: dir }
        }
    }

    impl Drop for Daemon {
        fn drop(&mut self) {
            if let Some(handle) = self.handle.take() {
                let _ = self.runtime.block_on(async { tokio::time::timeout(Duration::from_secs(10), handle.shutdown()).await });
            }
        }
    }

    /// `claudine steer <args>` from a separate process, pointed at `daemon`
    /// and at an empty Claude registry (native rows are not this test's).
    fn steer(project: &CodexProject, daemon: &Daemon, args: &[&str]) -> (Option<i32>, Value, String) {
        let mut command = project.fixture.command_builder().build_std();
        command
            .arg("steer")
            .args(args)
            .env("RENDEZVOUS_ENDPOINT", &daemon.endpoint)
            .env("CLAUDE_CONFIG_DIR", project.fixture.home().join(".claude"));
        let output = command.output().expect("run claudine steer");
        let document = serde_json::from_slice(&output.stdout).unwrap_or(Value::Null);
        (output.status.code(), document, text(&output.stderr))
    }

    fn codex_rows(listing: &Value) -> Vec<Value> {
        listing["sessions"].as_array().into_iter().flatten().filter(|row| row["provider"] == "codex").cloned().collect()
    }

    /// Every model request without the nonce runs one more short, distinct
    /// tool (distinct so the repetition guard stays quiet), keeping the turn
    /// working; the first request carrying it gets the final answer.
    fn working_until_steered() -> ScriptedModel {
        ScriptedModel::start(Box::new(|requests: &[String]| {
            let last = requests.last().unwrap();
            if last.contains(NONCE) {
                Reply::message(format!("ACKNOWLEDGED {NONCE}"))
            } else if requests.len() > 60 {
                Reply::message("GAVE_UP without a steering message")
            } else {
                Reply::shell(format!("sleep 1 # step {}", requests.len()))
            }
        }))
    }

    #[test]
    fn real_codex_app_server_manual_steer_reaches_the_running_wrapper_once() {
        let Some(_codex) = real_codex() else { return };
        let daemon = Daemon::start();
        let project = CodexProject::new("real-codex-manual-steer");
        let model = working_until_steered();
        let mut run = project
            .command(&model, &["--skip-git-repo-check"], "keep checking until told otherwise")
            .env("RENDEZVOUS_ENDPOINT", &daemon.endpoint)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .expect("launch claudine");

        // The wrapper registers with the daemon once its task is running.
        let until = Instant::now() + RUN_DEADLINE;
        let row = loop {
            let (code, listing, stderr) = steer(&project, &daemon, &["--list", "--json"]);
            assert_eq!(code, Some(0), "{stderr}");
            if let Some(row) = codex_rows(&listing).into_iter().find(|row| row["state"] == "working" && row["provider_version"].is_string()) {
                break row;
            }
            assert!(run.try_wait().unwrap().is_none(), "the run ended before it was listed");
            assert!(Instant::now() < until, "the running wrapper was never listed: {listing}");
            std::thread::sleep(Duration::from_millis(200));
        };
        let id = row["id"].as_str().unwrap().to_string();
        assert!(id.starts_with("managed:"), "{row}");
        assert_eq!(row["origins"], serde_json::json!(["managed"]));
        assert_eq!(row["launch_profile"], "managed-app-server");

        let message = format!("{NONCE}: stop checking and answer now. token={SECRET}");
        let granted = cfg!(target_os = "macos") && codex_version().as_deref() == Some("0.157.1");
        let (code, receipt, stderr) = steer(&project, &daemon, &["--session", &id, "--json", &message]);

        if !granted {
            // No grant here: listed with its reason, and nothing is sent.
            assert_eq!(row["availability"], "unavailable", "{row}");
            assert_eq!(code, Some(1), "{stderr}");
            assert_eq!(receipt["outcome"], "unavailable", "{receipt}");
            let _ = run.kill();
            let _ = run.wait();
            assert!(!model.saw(NONCE), "an unavailable session receives nothing");
            return;
        }

        assert_eq!(row["availability"], "non_interrupting", "{row}");
        assert_eq!(row["operation"], "steer_active_turn", "{row}");
        assert_eq!(code, Some(0), "{receipt}\n{stderr}");
        assert_eq!(receipt["target"], id.as_str());
        assert_eq!(receipt["operation"], "steer_active_turn");
        assert_eq!(receipt["mechanism"], "app-server-steer");
        assert_eq!(receipt["outcome"], "queued", "Codex's answer establishes scheduling: {receipt}");
        assert_eq!(receipt["receipt"], "queued");
        assert!(!receipt.to_string().contains(SECRET), "the receipt never echoes the message: {receipt}");

        wait_for(&mut run, "the steered run to end");
        let output = run.wait_with_output().unwrap();
        assert!(output.status.success(), "{}", describe(&output));
        assert!(text(&output.stdout).contains(&format!("ACKNOWLEDGED {NONCE}")), "{}", describe(&output));
        let requests = model.requests();
        let last = requests.last().unwrap();
        assert!(last.contains(SECRET), "the model received the original, unmasked message");
        assert_eq!(last.matches(NONCE).count(), 1, "the message is in the conversation exactly once");
        assert_eq!(requests.iter().filter(|body| body.contains(NONCE)).count(), 1, "the turn ended right after it");

        // The run is gone: its registration is removed, and the same ID
        // sends nothing now (never replayed, never redirected).
        let until = Instant::now() + Duration::from_secs(10);
        while codex_rows(&steer(&project, &daemon, &["--list", "--json"]).1).iter().any(|row| row["id"] == id.as_str()) {
            assert!(Instant::now() < until, "the ended run was never unlisted");
            std::thread::sleep(Duration::from_millis(100));
        }
        let (code, late, _) = steer(&project, &daemon, &["--session", &id, "--json", &message]);
        assert_eq!(code, Some(1));
        assert_eq!(late["outcome"], "unavailable", "{late}");
        assert_eq!(model.requests().len(), requests.len(), "nothing reached the model after the run ended");

        // The audit keeps the full message with the secret masked.
        let audit = project.fixture.home().join(".claudine").join("logs").join("steering");
        let logged: String = std::fs::read_dir(&audit)
            .unwrap_or_else(|error| panic!("steering audit at {}: {error}", audit.display()))
            .map(|entry| std::fs::read_to_string(entry.unwrap().path()).unwrap())
            .collect();
        assert!(logged.contains(NONCE), "the audit records the message");
        assert!(!logged.contains(SECRET), "the audit never holds the secret");
        assert!(logged.contains("****"), "the secret is masked");
    }
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
