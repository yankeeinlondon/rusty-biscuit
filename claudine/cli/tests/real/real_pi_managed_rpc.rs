//! `real_` tier: Pi's managed RPC launch through Claudine's actual wrapper
//! (`claudine pi "<task>"`), against the installed Pi and a deterministic
//! local model (`tests/fixtures/steering/pi-probe.ts`). No network,
//! credentials, or existing user session is involved; every run has its own
//! Pi agent directory, session directory, and home.
//!
//! What only a real Pi can show:
//!
//! - the RPC task runs and the wrapper ends it at `agent_settled`, with the
//!   ordinary rendered output and exit code, and no fallback;
//! - user-level extensions, skills, prompt templates, and context files stay
//!   enabled (project-local ones are governed by Claudine's existing
//!   `--no-approve` trust flag, which is unchanged);
//! - a tool batch keeps stdin open until the whole run settles;
//! - an extension dialog is cancelled rather than answered, and a task an
//!   extension handles without a model turn still ends the run;
//! - killing Pi mid-tool fails the run and leaves no tool process behind
//!   (Unix; the tool is Pi's built-in `bash`).
//!
//! ## How to run
//!
//! Opt-in: skips unless `CLAUDINE_CONTRACT_REAL=1` and `pi` (0.84.4 or later)
//! is on `PATH`.
//!
//! ```sh
//! just test-real real_pi_managed_rpc::   # in claudine/
//! ```

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Output};
use std::time::{Duration, Instant};

use crate::common;
use common::{CliProcessFixture, write};

/// Inside nextest's per-test limit, so a stuck run fails with its output.
const RUN_DEADLINE: Duration = Duration::from_secs(20);

/// The installed Pi, or `None` (with the reason printed) when this opt-in
/// tier cannot run here.
fn real_pi() -> Option<PathBuf> {
    if std::env::var("CLAUDINE_CONTRACT_REAL").as_deref() != Ok("1") {
        eprintln!("skipping real_pi_managed_rpc (set CLAUDINE_CONTRACT_REAL=1 to run)");
        return None;
    }
    let Ok(pi) = which::which("pi") else {
        eprintln!("skipping real_pi_managed_rpc (binary `pi` not on PATH)");
        return None;
    };
    let version = std::process::Command::new(&pi).arg("--version").output().expect("pi --version");
    eprintln!("real_pi_managed_rpc: {} is Pi {}", pi.display(), String::from_utf8_lossy(&version.stdout).trim());
    Some(pi)
}

/// One disposable Pi project: its agent directory carries the fixture model,
/// extension, skill, template, and context, all at user level.
struct PiProject {
    fixture: CliProcessFixture,
    agent: PathBuf,
    probe: PathBuf,
}

impl PiProject {
    fn new(name: &str, extension: &str) -> Self {
        let fixture = CliProcessFixture::named(name);
        fixture.seed_user_config();
        let root = fixture.workspace_path().to_path_buf();
        let agent = root.join("agent");
        let probe = root.join("probe");
        let sessions = root.join("sessions");
        for dir in [&agent, &probe, &sessions] {
            fs::create_dir_all(dir).unwrap();
        }
        let extension = biscuit_test_harness::manifest_dir!().join("tests/fixtures/steering").join(extension);
        let (provider, model) = if extension.ends_with("pi-bash-cleanup-probe.ts") {
            ("claudine-bash-cleanup-probe", "fixture")
        } else {
            ("claudine-probe", "fixture")
        };
        let settings = serde_json::json!({
            "defaultProvider": provider,
            "defaultModel": model,
            "defaultThinkingLevel": "off",
            "extensions": [extension],
            "sessionDir": sessions,
        });
        write(&agent.join("settings.json"), &settings.to_string());
        write(&agent.join("AGENTS.md"), "Fixture context marker: CONTEXT_NONCE\n");
        write(&agent.join("prompts/probe-template.md"), "---\ndescription: Fixture template\n---\nTEMPLATE_NONCE\n");
        write(
            &agent.join("skills/probe-skill/SKILL.md"),
            "---\nname: probe-skill\ndescription: Fixture skill\n---\nSKILL_NONCE\n",
        );
        Self { fixture, agent, probe }
    }

    fn marker(&self, name: &str) -> PathBuf {
        self.probe.join(name)
    }

    /// `claudine pi <task>` with Pi's fixture environment. Pi is a Node
    /// program, so the child keeps the host `PATH`.
    fn command(&self, task: &str) -> std::process::Command {
        let mut command = self.fixture.command_builder().host_path().build_std();
        command
            .args(["pi", task])
            .env("PI_CODING_AGENT_DIR", &self.agent)
            .env("PI_OFFLINE", "1")
            .env("PI_TELEMETRY", "0")
            .env("CLAUDINE_PI_PROBE_DIR", &self.probe)
            .env("CLAUDINE_PI_CLEANUP_DIR", &self.probe)
            .env("CLAUDINE_PI_CLEANUP_MARKER", format!("claudine-pi-cleanup-{}", std::process::id()));
        command
    }

    fn run(&self, task: &str) -> Output {
        let mut child = self
            .command(task)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .expect("launch claudine");
        wait_for(&mut child, RUN_DEADLINE, "the wrapped Pi run to end");
        child.wait_with_output().unwrap()
    }
}

/// Waits for `child` to exit; kills it and fails the test, with its output,
/// past `deadline`.
fn wait_for(child: &mut Child, deadline: Duration, what: &str) {
    let until = Instant::now() + deadline;
    while child.try_wait().unwrap().is_none() {
        if Instant::now() >= until {
            let _ = child.kill();
            let _ = child.wait();
            let (stdout, stderr) = drain(child);
            panic!("timed out waiting for {what}\nstdout:\n{stdout}\nstderr:\n{stderr}");
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// Whatever `child` wrote to its piped stdout and stderr.
fn drain(child: &mut Child) -> (String, String) {
    let read = |pipe: Option<&mut dyn std::io::Read>| {
        let mut out = String::new();
        if let Some(pipe) = pipe {
            let _ = pipe.read_to_string(&mut out);
        }
        out
    };
    (
        read(child.stdout.as_mut().map(|pipe| pipe as &mut dyn std::io::Read)),
        read(child.stderr.as_mut().map(|pipe| pipe as &mut dyn std::io::Read)),
    )
}

/// Waits for `path` while `child` runs; an early exit fails with its output.
fn wait_for_file(child: &mut Child, path: &Path, what: &str) {
    let until = Instant::now() + RUN_DEADLINE;
    while !path.is_file() {
        if child.try_wait().unwrap().is_some() {
            let (stdout, stderr) = drain(child);
            panic!("the run ended before {what}\nstdout:\n{stdout}\nstderr:\n{stderr}");
        }
        if Instant::now() >= until {
            let _ = child.kill();
            let probe = path.parent().map(|dir| fs::read_dir(dir).map(|entries| {
                entries.filter_map(Result::ok).map(|entry| entry.file_name().to_string_lossy().into_owned()).collect::<Vec<_>>()
            }));
            panic!("timed out waiting for {what}; probe directory holds {probe:?}");
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// What the fixture model saw on its last answering turn.
fn probe_transcript(project: &PiProject) -> String {
    fs::read_to_string(project.marker("probe.json")).unwrap_or_else(|_| "<no probe.json>".into())
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

fn assert_no_fallback(stderr: &str) {
    assert!(!stderr.contains("JSON stream instead"), "the RPC launch must not fall back:\n{stderr}");
}

#[test]
fn real_pi_managed_rpc_runs_a_task_to_settlement_with_resources_enabled() {
    let Some(_pi) = real_pi() else { return };
    let project = PiProject::new("real-pi-rpc-task", "pi-probe.ts");
    let output = project.run("TEMPLATE_NONCE: reply with the acknowledgement.");
    let (stdout, stderr) = (text(&output.stdout), text(&output.stderr));
    assert!(output.status.success(), "exit {:?}\nstdout:\n{stdout}\nstderr:\n{stderr}", output.status.code());
    assert!(stdout.contains("ACK:TEMPLATE_NONCE"), "the rendered answer reaches stdout:\n{stdout}");
    assert_no_fallback(&stderr);
    assert!(
        project.marker("context-observed").is_file(),
        "the user context file reached the model\nprobe: {}\nstderr:\n{stderr}",
        probe_transcript(&project)
    );

    for (task, marker, answer) in [
        ("/probe-template", "template-observed", "ACK:TEMPLATE_NONCE"),
        ("/skill:probe-skill", "skill-observed", "ACK:SKILL_NONCE"),
    ] {
        let output = project.run(task);
        let stdout = text(&output.stdout);
        assert!(output.status.success(), "{task}: {}", text(&output.stderr));
        assert!(project.marker(marker).is_file(), "{task} expanded through Pi's own resources");
        assert!(stdout.contains(answer), "{task}:\n{stdout}");
    }
}

#[test]
fn real_pi_managed_rpc_keeps_stdin_open_through_a_tool_batch() {
    let Some(_pi) = real_pi() else { return };
    let project = PiProject::new("real-pi-rpc-batch", "pi-probe.ts");
    let mut child = project
        .command("PROBE_TOOL_BATCH")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    wait_for_file(&mut child, &project.marker("tool-a-started"), "the first tool");
    std::thread::sleep(Duration::from_millis(300));
    assert!(child.try_wait().unwrap().is_none(), "the run must not end while a tool is held");
    fs::write(project.marker("release-a"), "release").unwrap();
    wait_for_file(&mut child, &project.marker("tool-b-started"), "the second tool");
    fs::write(project.marker("release-b"), "release").unwrap();
    wait_for(&mut child, RUN_DEADLINE, "the batch run to settle");
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success(), "{}", text(&output.stderr));
    assert!(project.marker("tool-a-finished").is_file() && project.marker("tool-b-finished").is_file());
    assert!(text(&output.stdout).contains("ACK:"), "the turn after the batch answered");
}

#[test]
fn real_pi_managed_rpc_cancels_extension_dialogs_and_ends_handled_tasks() {
    let Some(_pi) = real_pi() else { return };
    let project = PiProject::new("real-pi-rpc-ui", "pi-probe.ts");

    let output = project.run("/probe-ui");
    let stderr = text(&output.stderr);
    assert!(output.status.success(), "{stderr}");
    assert_eq!(
        fs::read_to_string(project.marker("ui-confirm-result")).unwrap().trim(),
        "false",
        "the dialog was cancelled, never approved"
    );
    assert!(stderr.contains("the request was cancelled"), "the cancellation is reported:\n{stderr}");

    let started = Instant::now();
    let output = project.run("/probe-handled");
    assert!(output.status.success(), "{}", text(&output.stderr));
    assert!(project.marker("extension-handled").is_file());
    assert!(!project.marker("model-call-count").is_file(), "no model turn ran");
    assert!(started.elapsed() < Duration::from_secs(30), "a handled task still ends the run promptly");
}

/// Pi's `bash` tool runs a marked `sleep`; killing Pi mid-tool must fail the
/// run and leave no tool behind.
#[cfg(unix)]
#[test]
fn real_pi_managed_rpc_provider_crash_fails_the_run_and_leaves_no_tool() {
    let Some(_pi) = real_pi() else { return };
    let project = PiProject::new("real-pi-rpc-crash", "pi-bash-cleanup-probe.ts");
    let mut child = project
        .command("run the cleanup probe")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    wait_for_file(&mut child, &project.marker("shell.pid"), "the tool to start");
    let tool: i32 = fs::read_to_string(project.marker("shell.pid")).unwrap().trim().parse().unwrap();
    // Reaps the tool however the test ends, so a failure cannot leak it.
    struct Reap(i32);
    impl Drop for Reap {
        fn drop(&mut self) {
            // SAFETY: signals only the fixture's own marked tool process.
            unsafe { libc::kill(self.0, libc::SIGKILL) };
        }
    }
    let _reap = Reap(tool);
    // Crash mid-tool, not in the instant the tool starts: the wrapper records
    // descendants by periodic scan, and a tool that starts and escapes
    // between two scans is outside what it can see.
    std::thread::sleep(Duration::from_secs(1));
    let provider = parent_of(tool).expect("the tool's parent is Pi");
    // SAFETY: `provider` is the Pi child this test's own wrapper launched.
    assert_eq!(unsafe { libc::kill(provider, libc::SIGKILL) }, 0);

    wait_for(&mut child, RUN_DEADLINE, "the wrapper to notice the crash");
    let output = child.wait_with_output().unwrap();
    assert!(!output.status.success(), "a crashed provider fails the run");
    let until = Instant::now() + Duration::from_secs(5);
    // SAFETY: probe only.
    while unsafe { libc::kill(tool, 0) } == 0 {
        assert!(Instant::now() < until, "the tool outlived its crashed provider and the wrapper");
        std::thread::sleep(Duration::from_millis(50));
    }
}

#[cfg(unix)]
fn parent_of(pid: i32) -> Option<i32> {
    let output = std::process::Command::new("/bin/ps").args(["-o", "ppid=", "-p", &pid.to_string()]).output().ok()?;
    text(&output.stdout).trim().parse().ok()
}
