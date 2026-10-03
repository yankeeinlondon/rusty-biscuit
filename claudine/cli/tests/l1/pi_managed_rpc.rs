//! Pi's managed RPC launch through the real wrapper (`claudine pi "<task>"`),
//! against the compiled `claudine-fake-pi` (`tests/bin/fake_pi`) so every
//! path runs deterministically on macOS, Linux, and Windows. The installed-Pi
//! counterpart is `tests/real/real_pi_managed_rpc.rs`.
//!
//! Each case checks what the user sees (exit code, stdout, stderr) and what
//! Pi received (argv and every RPC command), so a readiness failure is shown
//! to fall back before submission and a submitted task is shown never to be
//! replayed.

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
        let target = fixture.bin_dir().join(format!("pi{}", std::env::consts::EXE_SUFFIX));
        fs::copy(biscuit_test_harness::bin_exe!("claudine-fake-pi"), &target).unwrap();
        let dir = fixture.workspace_path().join("fake-pi");
        Self { fixture, dir }
    }

    fn run(&self, plan: &str) -> std::process::Output {
        self.run_with(plan, &[])
    }

    fn run_with(&self, plan: &str, env: &[(&str, &str)]) -> std::process::Output {
        let mut command = self.fixture.command();
        command
            .args(["pi", TASK])
            .env("FAKE_PI_DIR", &self.dir)
            .env("FAKE_PI_PLAN", plan)
            .timeout(Duration::from_secs(60));
        for (key, value) in env {
            command.env(key, value);
        }
        command.output().unwrap()
    }

    fn argv(&self, launch: u32) -> Option<Vec<String>> {
        let text = fs::read_to_string(self.dir.join(format!("argv-{launch}.json"))).ok()?;
        Some(serde_json::from_str(&text).unwrap())
    }

    fn commands(&self, launch: u32) -> Vec<Value> {
        fs::read_to_string(self.dir.join(format!("commands-{launch}.jsonl")))
            .unwrap_or_default()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect()
    }

    fn command_kinds(&self, launch: u32) -> Vec<String> {
        self.commands(launch).iter().map(|command| command["type"].as_str().unwrap().to_string()).collect()
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

#[test]
fn a_task_runs_over_rpc_and_the_settled_run_ends() {
    let rig = Rig::new("pi-rpc-ok");
    let output = rig.run("ok");
    assert!(output.status.success(), "{}", describe(&output));
    assert!(text(&output.stdout).contains(&format!("ACK:{TASK}")), "{}", describe(&output));
    assert!(!text(&output.stderr).contains("JSON stream instead"), "{}", describe(&output));

    let argv = rig.argv(1).expect("Pi launched");
    let pair = argv.windows(2).any(|pair| pair == ["--mode", "rpc"]);
    assert!(pair && argv.iter().any(|arg| arg == "--no-approve"), "{argv:?}");
    for injected in ["-p", "--print", "--no-extensions", "--no-skills", "--no-prompt-templates", "--no-context-files"] {
        assert!(!argv.iter().any(|arg| arg == injected), "{injected} in {argv:?}");
    }

    let commands = rig.commands(1);
    assert_eq!(commands[0]["type"], "get_state", "readiness comes first");
    assert_eq!(commands[1]["type"], "prompt");
    assert_eq!(commands[1]["message"], TASK, "the task is the one prompt");
    assert_eq!(rig.command_kinds(1).iter().filter(|kind| *kind == "prompt").count(), 1);
    assert!(rig.argv(2).is_none(), "no second launch");
}

#[test]
fn a_refused_readiness_check_falls_back_to_the_json_stream_before_submission() {
    let rig = Rig::new("pi-rpc-refuse-ready");
    let output = rig.run("refuse-ready");
    let stderr = flat(&output.stderr);
    assert!(output.status.success(), "{}", describe(&output));
    assert!(stderr.contains("Pi's readiness check failed: Pi refused it (fake refusal)"), "{stderr}");
    assert!(stderr.contains("JSON stream instead") && stderr.contains("cannot be steered"), "{stderr}");
    assert_eq!(rig.command_kinds(1), ["get_state"], "the task never reached the RPC child");

    let fallback = rig.argv(2).expect("the JSON launch ran");
    assert!(fallback.windows(2).any(|pair| pair == ["--mode", "json"]), "{fallback:?}");
    assert!(fallback.iter().any(|arg| arg == "-p") && fallback.iter().any(|arg| arg == "--no-approve"), "{fallback:?}");
    assert_eq!(fs::read_to_string(rig.dir.join("stdin-2.txt")).unwrap(), TASK, "the task is the JSON launch's stdin");
    assert!(text(&output.stdout).contains(&format!("ACK:{TASK}")), "{}", describe(&output));
}

#[test]
fn a_child_that_exits_before_it_is_ready_falls_back_with_its_stderr_shown() {
    let rig = Rig::new("pi-rpc-exit-early");
    let output = rig.run("exit-early");
    let stderr = flat(&output.stderr);
    assert!(output.status.success(), "{}", describe(&output));
    assert!(stderr.contains("fake pi: this build cannot start RPC mode"), "{stderr}");
    // Whether Pi's exit or the failed write is noticed first depends on
    // timing; either way the reason is shown before the fallback.
    assert!(stderr.contains("JSON stream instead"), "{stderr}");
    assert!(rig.argv(2).is_some(), "the JSON launch ran");
    assert!(rig.commands(1).is_empty(), "the exited child never read a command");
}

#[test]
fn a_crash_after_submission_fails_the_run_and_is_never_replayed() {
    let rig = Rig::new("pi-rpc-crash");
    let output = rig.run("crash");
    assert!(!output.status.success(), "{}", describe(&output));
    assert_eq!(rig.command_kinds(1), ["get_state", "prompt"]);
    assert!(rig.argv(2).is_none(), "an ambiguous submission is never replayed or replaced");
}

#[test]
fn an_extension_dialog_is_cancelled_never_approved() {
    let rig = Rig::new("pi-rpc-dialog");
    let output = rig.run("dialog");
    assert!(output.status.success(), "{}", describe(&output));
    assert!(text(&output.stdout).contains("ACK:cancelled"), "{}", describe(&output));
    assert!(flat(&output.stderr).contains("the request was cancelled"), "{}", describe(&output));
    let answer = rig.commands(1).into_iter().find(|command| command["type"] == "extension_ui_response").unwrap();
    assert_eq!(answer, serde_json::json!({"type": "extension_ui_response", "id": "ui-1", "cancelled": true}));
}

#[test]
fn an_unanswerable_ui_request_fails_the_run_as_input_required() {
    let rig = Rig::new("pi-rpc-custom-ui");
    let output = rig.run("custom-ui");
    let stderr = flat(&output.stderr);
    assert!(!output.status.success(), "{}", describe(&output));
    assert!(stderr.contains("waiting for `custom` input"), "{stderr}");
    assert!(!rig.command_kinds(1).iter().any(|kind| kind == "extension_ui_response"), "no answer is invented");
}

// ---------------------------------------------------------------------------
// Automatic repetition help. The shipped policy blocks Pi's managed profile,
// so help is unavailable: the run must say so once, send nothing, and still
// stop on the unchanged repetition schedule.
// ---------------------------------------------------------------------------

const UNAVAILABLE_NOTICE: &str = "cannot send automatic steering to this session";

/// Where the repetition stop is reported, at the default limit's exact count
/// (so the stop's schedule is shown unchanged).
fn repetition_stop(stderr: &str) -> Option<usize> {
    stderr.find("runaway repetition detected (cycle length 1, 30 repeats)")
}

#[test]
fn automatic_help_that_cannot_be_sent_warns_once_before_the_unchanged_repetition_stop() {
    let rig = Rig::new("pi-rpc-auto-unavailable");
    let output = rig.run("repeat");
    let stderr = flat(&output.stderr);
    assert!(!output.status.success(), "{}", describe(&output));
    let notice = stderr.find(UNAVAILABLE_NOTICE).unwrap_or_else(|| panic!("no notice: {}", describe(&output)));
    assert_eq!(stderr.matches(UNAVAILABLE_NOTICE).count(), 1, "deduplicated: {stderr}");
    assert!(stderr.contains("keep enforcing the repetition limit of 30"), "{stderr}");
    let stop = repetition_stop(&stderr[notice + UNAVAILABLE_NOTICE.len()..]);
    assert!(stop.is_some(), "the repetition stop follows the notice: {stderr}");
    assert_eq!(rig.command_kinds(1).iter().filter(|kind| *kind == "steer" || *kind == "prompt").count(), 1,
        "only the task reached Pi; no automatic message was sent");
    assert!(rig.argv(2).is_none(), "nothing was relaunched");
    let echoed = text(&output.stdout).matches("I will try the same fix again.").count();
    assert_eq!(echoed, 29, "the tripping line is suppressed, as before: {}", describe(&output));
}

#[test]
fn automatic_help_turned_off_by_environment_prints_nothing_and_stops_the_same() {
    for value in ["off", " FALSE ", "0", "no"] {
        let rig = Rig::new("pi-rpc-auto-off");
        let output = rig.run_with("repeat", &[("CLAUDINE_AUTO_STEER", value)]);
        let stderr = flat(&output.stderr);
        assert!(!output.status.success(), "{value:?}: {}", describe(&output));
        assert!(!stderr.contains(UNAVAILABLE_NOTICE), "{value:?}: {stderr}");
        assert!(repetition_stop(&stderr).is_some(), "{value:?}: the guard still stops the run: {stderr}");
    }
}

#[test]
fn automatic_help_turned_off_in_user_configuration_prints_nothing_and_stops_the_same() {
    let rig = Rig::new("pi-rpc-auto-config-off");
    crate::common::write(
        &rig.fixture.home().join(".claudine/config.json"),
        r#"{ "steering": { "automatic": { "enabled": false } } }"#,
    );
    let output = rig.run("repeat");
    let stderr = flat(&output.stderr);
    assert!(!output.status.success(), "{}", describe(&output));
    assert!(!stderr.contains(UNAVAILABLE_NOTICE), "{stderr}");
    assert!(repetition_stop(&stderr).is_some(), "{stderr}");

    // The environment overrides the configured opt-out.
    let rig_on = Rig::new("pi-rpc-auto-config-env-on");
    crate::common::write(
        &rig_on.fixture.home().join(".claudine/config.json"),
        r#"{ "steering": { "automatic": { "enabled": false } } }"#,
    );
    let output = rig_on.run_with("repeat", &[("CLAUDINE_AUTO_STEER", "on")]);
    assert!(flat(&output.stderr).contains(UNAVAILABLE_NOTICE), "{}", describe(&output));
}

#[test]
fn a_malformed_auto_steer_value_fails_before_pi_is_launched() {
    for value in ["", "maybe", "2"] {
        let rig = Rig::new("pi-rpc-auto-malformed");
        let output = rig.run_with("repeat", &[("CLAUDINE_AUTO_STEER", value)]);
        let stderr = flat(&output.stderr);
        assert!(!output.status.success(), "{value:?}: {}", describe(&output));
        assert!(stderr.contains("CLAUDINE_AUTO_STEER"), "{value:?}: {stderr}");
        assert!(rig.argv(1).is_none(), "{value:?}: Pi must not be launched");
    }
}

#[test]
fn a_malformed_configured_value_fails_before_pi_is_launched() {
    let rig = Rig::new("pi-rpc-auto-config-null");
    crate::common::write(&rig.fixture.home().join(".claudine/config.json"), r#"{ "steering": { "automatic": { "enabled": null } } }"#);
    let output = rig.run("repeat");
    assert!(!output.status.success(), "{}", describe(&output));
    assert!(rig.argv(1).is_none(), "Pi must not be launched: {}", describe(&output));
}
