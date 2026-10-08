//! Caller regressions use the shipped CLI and portable compiled providers.
#![cfg(feature = "test-fixtures")]
use std::fs;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use crate::common::CliProcessFixture;

struct Release(std::path::PathBuf);
impl Drop for Release {
    fn drop(&mut self) { let _ = fs::write(self.0.join("release"), b"release"); }
}

fn chain(fixture: &CliProcessFixture, provider: &str) -> Command {
    let program = fixture.command_std().get_program().to_owned();
    let exit = fixture.workspace_path().join("wrapper-exit.txt");
    let marker = fixture.workspace_path().join("next-command.txt");
    let options = if provider == "codex" { "-- --skip-git-repo-check review" } else { "-- review" };
    #[cfg(not(windows))]
    let mut command = {
        let script = format!("{} {provider} {options}; code=$?; printf '%s' \"$code\" > {}; if [ \"$code\" -eq 0 ]; then printf next > {}; fi",
            crate::common::sh_quote(&program.to_string_lossy()), crate::common::sh_quote(&exit.to_string_lossy()), crate::common::sh_quote(&marker.to_string_lossy()));
        let mut command = Command::new("sh");
        command.args(["-c", &script]);
        command
    };
    #[cfg(windows)]
    let mut command = {
        let script = fixture.workspace_path().join("chain.cmd");
        fs::write(&script, format!("@echo off\r\n\"{}\" {provider} {options}\r\nset code=%errorlevel%\r\n>\"{}\" echo %code%\r\nif %code%==0 >\"{}\" echo next\r\nexit /b 0\r\n", program.to_string_lossy().replace('%', "%%"), exit.to_string_lossy().replace('%', "%%"), marker.to_string_lossy().replace('%', "%%"))).unwrap();
        let mut command = Command::new("cmd.exe");
        command.args(["/D", "/V:OFF", "/C"]).arg(script);
        command
    };
    // Claudine is a grandchild of the shell; apply the same hermetic policy.
    fixture.command_builder().apply_policy_to(&mut command);
    command.stdout(Stdio::null()).stderr(Stdio::null());
    command
}

fn run(provider: &str, scenario: &str) -> serde_json::Value {
    let fixture = CliProcessFixture::named("completion-observations");
    fs::copy(biscuit_test_harness::bin_exe!("claudine-fake-completion"),
        fixture.bin_dir().join(format!("{provider}{}", std::env::consts::EXE_SUFFIX))).unwrap();
    let directory = fixture.workspace_path().join(scenario);
    fs::create_dir(&directory).unwrap();
    let _release = Release(directory.clone());
    let mut command = chain(&fixture, provider);
    command.env("CLAUDINE_TEST_COMPLETION_FIXTURE", &directory)
        .env("FAKE_COMPLETION_DIR", &directory).env("FAKE_COMPLETION_PROVIDER", provider);
    let mut child = command.spawn().unwrap();
    let deadline = Instant::now() + Duration::from_secs(20);
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() { break status; }
        if Instant::now() >= deadline {
            let _ = child.kill(); let _ = child.wait();
            panic!("{provider}/{scenario}: wrapper did not settle");
        }
        std::thread::sleep(Duration::from_millis(5));
    };
    assert!(status.success());
    assert!(directory.join("ready").exists(), "{provider}/{scenario}: stall reached");
    assert_eq!(fs::read_to_string(directory.join("native-exit.txt")).unwrap(), "0");
    let code = fs::read_to_string(fixture.workspace_path().join("wrapper-exit.txt")).unwrap();
    let code: i32 = code.trim().parse().unwrap();
    assert_eq!(code, if scenario == "answer-callback" { 1 } else { 0 }, "{provider}/{scenario}");
    assert_eq!(fixture.workspace_path().join("next-command.txt").exists(), code == 0);
    assert!(!fixture.audio_spool().exists());
    let snapshot: serde_json::Value = serde_json::from_slice(&fs::read(directory.join("observation.json")).unwrap()).unwrap();
    assert_eq!(snapshot["provider"], provider);
    assert_eq!(snapshot["exit_code"], 0);
    assert!(snapshot["observation"]["retained"]["raw_output"].as_str().unwrap().contains("review-answer"));
    snapshot
}

#[test]
fn fixture_handshakes_retain_answers_before_both_callback_stalls() {
    for provider in ["claude", "codex"] {
        for scenario in ["completion-callback", "answer-callback"] {
            let snapshot = run(provider, scenario);
            let observation = &snapshot["observation"];
            assert_eq!(observation["verdict_received"], scenario == "completion-callback");
            assert!(observation["retained"]["response_text"].as_str().unwrap().contains("review-answer"));
            assert_eq!(observation["retained"]["response_complete"], scenario == "completion-callback");
            assert_eq!(observation["stdout"]["operation"], "lifecycle_callback");
        }
    }
}

#[test]
fn fixture_separates_queued_output_from_unfinished_delivery() {
    for provider in ["claude", "codex"] {
        let snapshot = run(provider, "output-delivery");
        assert_eq!(snapshot["observation"]["output_worker"]["active"]["operation"], "terminal_delivery");
        assert!(snapshot["observation"]["verdict_received"].as_bool().unwrap());
    }
}

#[test]
fn invalid_or_missing_fixture_scenarios_fail_explicitly() {
    let fixture = CliProcessFixture::named("invalid-completion-fixture");
    for path in [fixture.workspace_path().join("unknown"), fixture.workspace_path().join("answer-callback")] {
        fixture.command().arg("--help").env("CLAUDINE_TEST_COMPLETION_FIXTURE", path).assert().failure();
    }
}
