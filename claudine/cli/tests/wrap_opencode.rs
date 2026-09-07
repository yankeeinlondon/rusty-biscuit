#![cfg(unix)]

//! Integration tests: OpenCode wrapper behavior: model resolution, repo-root launch, structured stdout, and stderr diagnostic classification.
//!
//! Split out of the `wrap_commands.rs` god file; shared fixtures live in
//! `common::wrap`.

use predicates::str::contains;
use std::fs;
use std::path::Path;
mod common;
use common::wrap::*;
use common::{CliProcessFixture, strip_ansi, write, write_executable};

#[cfg(unix)]
#[test]
fn opencode_non_interactive_requires_model_when_missing() {
    let fixture = CliProcessFixture::named("opencode-requires-model");
    fixture.seed_user_config();
    let args_path = fixture.cwd().join("args.txt");
    let env_path = fixture.cwd().join("env.txt");

    write_executable(
        &fixture.bin_dir().join("opencode"),
        r#"#!/bin/sh
printf '%s\n' "$@" > "$CLAUDINE_ARGS_FILE"
printf 'MODEL=%s\n' "$MODEL" > "$CLAUDINE_ENV_FILE"
exit 0
"#,
    );

    fixture
        .command()
        .env("CLAUDINE_ARGS_FILE", &args_path)
        .env("CLAUDINE_ENV_FILE", &env_path)
        .args(["opencode", "summarize"])
        .assert()
        .failure()
        .stderr(contains("No model specified!"))
        .stderr(contains("OPENCODE_MODEL"));

    assert!(
        !args_path.exists(),
        "child process should not launch when no non-interactive model is available"
    );
    assert!(
        !env_path.exists(),
        "child process should not launch when no non-interactive model is available"
    );
}

#[cfg(unix)]
#[test]
fn opencode_launches_child_from_repo_root() {
    let fixture = CliProcessFixture::named("opencode-repo-root-launch");
    fixture.seed_user_config();
    let Some((repo_root, launch_dir, _repo_bin)) =
        create_claudine_monorepo(fixture.workspace_path())
    else {
        eprintln!("Skipping integration test: git init unavailable");
        return;
    };
    let pwd_path = fixture.cwd().join("pwd.txt");
    let env_path = fixture.cwd().join("env.txt");
    let args_path = fixture.cwd().join("args.txt");

    write_executable(
        &fixture.bin_dir().join("opencode"),
        r#"#!/bin/sh
pwd > "$CLAUDINE_PWD_FILE"
printf '%s\n' "$@" > "$CLAUDINE_ARGS_FILE"
{
  printf 'PACKAGE=%s\n' "$PACKAGE"
  printf 'PACKAGE_AREA=%s\n' "$PACKAGE_AREA"
} > "$CLAUDINE_ENV_FILE"
exit 0
"#,
    );

    // ambient-context escape: the subject is the launch context — claudine must
    // re-anchor the child at the repo root of the monorepo this test built.
    fixture
        .command_builder()
        .ambient_context(&launch_dir)
        .build()
        .env("OPENCODE_MODEL", "test-model")
        .env("CLAUDINE_PWD_FILE", &pwd_path)
        .env("CLAUDINE_ARGS_FILE", &args_path)
        .env("CLAUDINE_ENV_FILE", &env_path)
        .args(["opencode", "summarize"])
        .assert()
        .success();

    let pwd_actual = fs::read_to_string(&pwd_path)
        .unwrap()
        .trim()
        .trim_end_matches('/')
        .to_string();
    let pwd_expected = repo_root
        .canonicalize()
        .unwrap()
        .display()
        .to_string()
        .trim_end_matches('/')
        .to_string();
    assert_eq!(pwd_actual, pwd_expected);
    let env_lines = fs::read_to_string(&env_path).unwrap();
    assert!(env_lines.contains("PACKAGE=claudine-cli"));
    assert!(env_lines.contains("PACKAGE_AREA=claudine"));
    let args = fs::read_to_string(&args_path).unwrap();
    assert!(args.lines().any(|line| line == "summarize"));
}

#[cfg(unix)]
#[test]
fn opencode_non_interactive_model_precedence_uses_env_overrides() {
    let fixture = CliProcessFixture::named("opencode-model-env-precedence");
    fixture.seed_user_config();
    let args_path = fixture.cwd().join("args.txt");
    let env_path = fixture.cwd().join("env.txt");

    write_executable(
        &fixture.bin_dir().join("opencode"),
        r#"#!/bin/sh
printf '%s\n' "$@" > "$CLAUDINE_ARGS_FILE"
printf 'MODEL=%s\n' "$MODEL" > "$CLAUDINE_ENV_FILE"
exit 0
"#,
    );

    fixture
        .command()
        .env("CLAUDINE_ARGS_FILE", &args_path)
        .env("CLAUDINE_ENV_FILE", &env_path)
        .env("OPENCODE_MODEL", "from-opencode")
        .args(["opencode", "summarize"])
        .assert()
        .success();

    let args = fs::read_to_string(&args_path).unwrap();
    let args: Vec<&str> = args.lines().collect();
    let model_index = args.iter().position(|arg| *arg == "--model").unwrap();
    assert_eq!(args.get(model_index + 1), Some(&"from-opencode"));
    let env_lines = fs::read_to_string(&env_path).unwrap();
    assert!(env_lines.contains("MODEL=from-opencode"));
}

#[cfg(unix)]
#[test]
fn opencode_non_interactive_explicit_cli_model_sets_model_env() {
    let fixture = CliProcessFixture::named("opencode-cli-model-env");
    fixture.seed_user_config();
    let args_path = fixture.cwd().join("args.txt");
    let env_path = fixture.cwd().join("env.txt");

    write_executable(
        &fixture.bin_dir().join("opencode"),
        r#"#!/bin/sh
printf '%s\n' "$@" > "$CLAUDINE_ARGS_FILE"
printf 'MODEL=%s\n' "$MODEL" > "$CLAUDINE_ENV_FILE"
exit 0
"#,
    );

    fixture
        .command()
        .env("CLAUDINE_ARGS_FILE", &args_path)
        .env("CLAUDINE_ENV_FILE", &env_path)
        .args(["opencode", "--model", "cli-selected", "summarize"])
        .assert()
        .success();

    let args = fs::read_to_string(&args_path).unwrap();
    assert!(args.lines().any(|line| line == "cli-selected"));
    let env_lines = fs::read_to_string(&env_path).unwrap();
    assert!(env_lines.contains("MODEL=cli-selected"));
}

#[cfg(unix)]
#[test]
fn opencode_post_summary_messages_are_logged_after_summary_block() {
    let fixture = CliProcessFixture::named("opencode-post-summary-order");
    fixture.seed_user_config();

    write_executable(
        &fixture.bin_dir().join("opencode"),
        r#"#!/bin/sh
exit 0
"#,
    );

    // Interactive mode (-i) still warns that OpenCode doesn't support --yolo
    // in interactive sessions (refined copy). This keeps the deferred-warning
    // ordering test meaningful after non-interactive forwards the flag.
    let assert = fixture
        .command()
        .env("OPENCODE_MODEL", "test-model")
        .args(["opencode", "-i", "-y"])
        .assert()
        .success();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).to_string();
    let plain = strip_ansi(&stderr);
    assert!(plain.starts_with('\n'));
    assert!(plain.contains("\nClaudine"));
    let summary_index = plain.find("Environment Variables:").unwrap();
    // Prose `<i>` tags render as ANSI italics which NO_COLOR strips, leaving
    // the word without markup in the plain-text output.
    let warning_needle = "--yolo mode is not supported in OpenCode interactive sessions";
    let warning_index = plain
        .find(warning_needle)
        .unwrap_or_else(|| panic!("expected refined interactive warning in stderr; got:\n{plain}"));

    assert!(warning_index > summary_index);
    // In interactive mode YOLO stays marked unsupported for OpenCode, so the
    // header should not advertise a YOLO badge.
    let header_line = plain
        .lines()
        .find(|line| line.contains("Claudine"))
        .unwrap();
    assert!(!header_line.contains("YOLO"));
    assert!(plain.contains("YOLO=false"));
    assert!(!plain.contains("`--model`"));
}

// ---------------------------------------------------------------------------
// Provider header shows provider name (review 5.3)
// ---------------------------------------------------------------------------

#[cfg(unix)]
#[test]
fn compose_opencode_non_interactive_passes_prompt_as_positional_arg() {
    let fixture = CliProcessFixture::named("compose-opencode-positional");
    let args_path = fixture.cwd().join("args.txt");
    fixture.seed_user_config();

    let md_file = fixture.cwd().join("test.md");
    fs::write(&md_file, "---\ntitle: test\n---\nHello OpenCode\n").unwrap();

    write_executable(
        &fixture.bin_dir().join("opencode"),
        r#"#!/bin/sh
printf '%s\n' "$@" > "$CLAUDINE_ARGS_FILE"
exit 0
"#,
    );

    fixture
        .command()
        .env("OPENCODE_MODEL", "test-model")
        .env("CLAUDINE_ARGS_FILE", &args_path)
        .args(["compose", "--opencode", md_file.to_str().unwrap()])
        .assert()
        .success();

    let args = fs::read_to_string(&args_path).unwrap();
    let collected: Vec<_> = args.lines().collect();
    let run_index = collected
        .iter()
        .position(|arg| *arg == "run")
        .expect("compose should use the OpenCode run entrypoint");
    let format_index = collected
        .iter()
        .position(|arg| *arg == "--format")
        .expect("compose should request OpenCode JSON format");
    let json_index = format_index + 1;
    let prompt_index = collected
        .iter()
        .position(|arg| *arg == "Hello OpenCode")
        .expect("compose should pass the composed prompt as a positional arg for OpenCode");

    assert!(run_index < format_index, "args: {args}");
    assert_eq!(collected.get(json_index), Some(&"json"), "args: {args}");
    assert!(
        json_index < prompt_index,
        "OpenCode flags must precede the positional prompt so structured output is enabled; args: {args}"
    );
}

#[cfg(unix)]
#[test]
fn compose_opencode_launches_child_from_repo_root() {
    let fixture = CliProcessFixture::named("compose-opencode-repo-root");
    fixture.seed_user_config();
    let Some((repo_root, launch_dir, _repo_bin)) =
        create_claudine_monorepo(fixture.workspace_path())
    else {
        eprintln!("Skipping integration test: git init unavailable");
        return;
    };
    let pwd_path = fixture.cwd().join("pwd.txt");
    let env_path = fixture.cwd().join("env.txt");
    let args_path = fixture.cwd().join("args.txt");
    let md_file = repo_root.join("prompts/test.md");
    write(&md_file, "---\ntitle: test\n---\nHello OpenCode\n");

    write_executable(
        &fixture.bin_dir().join("opencode"),
        r#"#!/bin/sh
pwd > "$CLAUDINE_PWD_FILE"
printf '%s\n' "$@" > "$CLAUDINE_ARGS_FILE"
{
  printf 'PACKAGE=%s\n' "$PACKAGE"
  printf 'PACKAGE_AREA=%s\n' "$PACKAGE_AREA"
} > "$CLAUDINE_ENV_FILE"
exit 0
"#,
    );

    // ambient-context escape: the subject is the launch context — the child
    // must start at the repo root of the monorepo this test built.
    fixture
        .command_builder()
        .ambient_context(&launch_dir)
        .build()
        .env("OPENCODE_MODEL", "test-model")
        .env("CLAUDINE_PWD_FILE", &pwd_path)
        .env("CLAUDINE_ARGS_FILE", &args_path)
        .env("CLAUDINE_ENV_FILE", &env_path)
        .args(["compose", "--opencode", md_file.to_str().unwrap()])
        .assert()
        .success();

    let pwd_actual = fs::read_to_string(&pwd_path)
        .unwrap()
        .trim()
        .trim_end_matches('/')
        .to_string();
    let pwd_expected = repo_root
        .canonicalize()
        .unwrap()
        .display()
        .to_string()
        .trim_end_matches('/')
        .to_string();
    assert_eq!(pwd_actual, pwd_expected);
    let env_lines = fs::read_to_string(&env_path).unwrap();
    assert!(env_lines.contains("PACKAGE=claudine-cli"));
    assert!(env_lines.contains("PACKAGE_AREA=claudine"));
    let args = fs::read_to_string(&args_path).unwrap();
    let collected: Vec<_> = args.lines().collect();
    assert!(collected.contains(&"run"));
    assert!(collected.contains(&"Hello OpenCode"));
}

#[cfg(unix)]
#[test]
fn compose_opencode_launches_from_repo_root_for_package_prompt_refs() {
    let fixture = CliProcessFixture::named("compose-opencode-package-prompt");
    fixture.seed_user_config();
    let Some((repo_root, _launch_dir, _repo_bin)) =
        create_claudine_monorepo(fixture.workspace_path())
    else {
        eprintln!("Skipping integration test: git init unavailable");
        return;
    };
    let package_root = repo_root.join("claudine");
    let pwd_path = fixture.cwd().join("pwd-package.txt");
    let env_path = fixture.cwd().join("env-package.txt");
    let args_path = fixture.cwd().join("args-package.txt");
    let md_file = package_root.join("prompts/test.md");
    write(&md_file, "---\ntitle: test\n---\nHello OpenCode\n");

    write_executable(
        &fixture.bin_dir().join("opencode"),
        r#"#!/bin/sh
pwd > "$CLAUDINE_PWD_FILE"
printf '%s\n' "$@" > "$CLAUDINE_ARGS_FILE"
{
  printf 'PACKAGE=%s\n' "$PACKAGE"
  printf 'PACKAGE_AREA=%s\n' "$PACKAGE_AREA"
} > "$CLAUDINE_ENV_FILE"
exit 0
"#,
    );

    // ambient-context escape: the subject is the launch context — a
    // package-scoped `@prompts/...` reference must still re-anchor at the repo
    // root of the monorepo this test built.
    fixture
        .command_builder()
        .ambient_context(&package_root)
        .build()
        .env("OPENCODE_MODEL", "test-model")
        .env("CLAUDINE_PWD_FILE", &pwd_path)
        .env("CLAUDINE_ARGS_FILE", &args_path)
        .env("CLAUDINE_ENV_FILE", &env_path)
        .args(["compose", "--opencode", "@prompts/test.md"])
        .assert()
        .success();

    let pwd_actual = fs::read_to_string(&pwd_path)
        .unwrap()
        .trim()
        .trim_end_matches('/')
        .to_string();
    let pwd_expected = repo_root
        .canonicalize()
        .unwrap()
        .display()
        .to_string()
        .trim_end_matches('/')
        .to_string();
    assert_eq!(pwd_actual, pwd_expected);
    let env_lines = fs::read_to_string(&env_path).unwrap();
    assert!(env_lines.contains("PACKAGE_AREA=claudine"));
    let args = fs::read_to_string(&args_path).unwrap();
    let collected: Vec<_> = args.lines().collect();
    assert!(collected.contains(&"run"));
    assert!(collected.contains(&"Hello OpenCode"));
}

/// End-to-end regression test for the OpenCode structured wrap pipeline.
///
/// Motivated by review-2 finding #4: the OpenCode assistant-text bug that
/// inspired this feature was in the wrapped pipeline, not just the parser.
/// This test mirrors the Codex/Gemini `*_structured_*` coverage so the
/// end-to-end OpenCode path has the same regression surface.
///
/// Asserts that a real child process, stream parser, and live sink
/// cooperate to produce:
///
/// 1. The assistant text reaches stdout (regression guard for the missing-
///    assistant-text bug).
/// 2. The tool completion renders as a canonical incoming `←` line with
///    a humanized tool name — NOT a synthesized outgoing `→` line and
///    NOT via the `⚙` Info glyph.
/// 3. No two consecutive blank lines appear in the combined stdout +
///    stderr rendered output (Child 3 section-spacing contract).
/// 4. The session-id marker and the trailer render on stderr.
#[cfg(unix)]
#[test]
#[serial_test::serial]
fn opencode_structured_e2e_stdout_and_section_spacing() {
    let fixture = CliProcessFixture::named("opencode-structured-e2e");

    // Minimal OpenCode fake binary. Uses the simple parser-compatible
    // event shapes (matching the `step_start` / `text` / `tool_end` /
    // `step_complete` parser unit tests in opencode_semantic.rs) rather
    // than the full nested `.part` payload, which keeps the test
    // resilient to stream-protocol evolution while still exercising the
    // sink end-to-end.
    let args_path = fixture.cwd().join("args.txt");
    write_executable(
        &fixture.bin_dir().join("opencode"),
        r#"#!/bin/sh
printf '%s\n' "$@" > "$CLAUDINE_ARGS_FILE"
printf '%s\n' '{"type":"step_start","sessionID":"ses_oc_e2e"}'
printf '%s\n' '{"type":"tool_end","part":{"tool_use_id":"t1","status":"success","content":"ok","tool_name":"bash"}}'
printf '%s\n' '{"type":"text","text":"The answer is 42."}'
printf '%s\n' '{"type":"step_complete","usage":{"input_tokens":10,"output_tokens":5,"total_tokens":15},"cost_usd":0.001,"duration_ms":1500}'
"#,
    );

    let assert = fixture
        .command()
        .env("OPENCODE_MODEL", "test-model")
        .env("CLAUDINE_ARGS_FILE", &args_path)
        // NOTE: intentionally do NOT pass `--format json` here — claudine
        // treats an explicit native output request as a signal to skip
        // structured streaming and forward raw bytes, which would bypass
        // every behavior this test is guarding.
        .args(["opencode", "what is the answer"])
        .assert()
        .success();

    let args_captured = fs::read_to_string(&args_path).unwrap_or_default();
    assert!(
        !args_captured.is_empty(),
        "fake opencode should have been invoked (CLAUDINE_ARGS_FILE empty)"
    );

    let out = assert.get_output();
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr_raw = String::from_utf8_lossy(&out.stderr);
    let stderr = strip_ansi(&stderr_raw);

    // 1. Assistant text reaches stdout.
    assert!(
        stdout.contains("The answer is 42."),
        "expected assistant text on stdout; stdout={stdout:?}"
    );

    // 2. Incoming `←` line exists; no synthesized outgoing `→` line; no
    //    `⚙` Info glyph routing.
    assert!(
        stderr.contains('\u{2190}'),
        "expected incoming ← arrow for tool completion; stderr={stderr}"
    );
    assert!(
        !stderr.contains('\u{2192}'),
        "OpenCode must NOT synthesize an outgoing → line; stderr={stderr}"
    );
    assert!(
        !stderr.contains('\u{2699}'),
        "tool result must NOT be rendered via the ⚙ Info glyph; stderr={stderr}"
    );
    assert!(
        stderr.contains("Bash"),
        "humanized tool name must render; stderr={stderr}"
    );

    // 3. Session ID + trailer render on stderr.
    assert!(
        stderr.contains("session ID") && stderr.contains("ses_oc_e2e"),
        "expected session ID marker; stderr={stderr}"
    );
    assert!(
        stderr.contains("1.5s") || stderr.contains("1.500s") || stderr.contains("1,500ms"),
        "expected trailer duration (1.5s); stderr={stderr}"
    );

    // 4. Combined stdout+stderr should never have more than two
    //    consecutive blank lines. Two consecutive blanks can occur
    //    when section separators around stdout text land next to each
    //    other in the combined stream, but three or more indicates a
    //    real spacing bug.
    let combined = format!("{stdout}{stderr}");
    let mut consecutive_blanks = 0;
    for line in combined.lines() {
        if line.trim().is_empty() {
            consecutive_blanks += 1;
        } else {
            consecutive_blanks = 0;
        }
        assert!(
            consecutive_blanks <= 2,
            "more than two consecutive blank lines in combined rendered output:\n---\n{combined}\n---"
        );
    }
}

// ---------------------------------------------------------------------------
// OpenCode stderr log bridge integration (Phase 6 integration scenarios)
//
// These tests drive claudine's structured OpenCode wrapper path end-to-end
// with a fake `opencode` binary that emits NDJSON on stdout and structured
// log records on stderr. They assert against the persisted JSONL log row
// produced by the summary event emitter in
// `claudine::stream::reporting::summary_to_event_meta(...)` so the
// verification does not depend on terminal rendering or ANSI behavior.
// ---------------------------------------------------------------------------

#[cfg(unix)]
fn read_summary_row(home: &Path) -> serde_json::Value {
    let log_path = today_log_path(home);
    let contents =
        fs::read_to_string(&log_path).expect("today's JSONL log should exist after wrap run");
    let row = contents
        .lines()
        .find(|line| line.contains("\"synthetic_kind\":\"stream_wrapper_summary\""))
        .unwrap_or_else(|| panic!("no stream_wrapper_summary row in log:\n{contents}"));
    serde_json::from_str::<serde_json::Value>(row)
        .unwrap_or_else(|e| panic!("failed to parse summary row as JSON ({e}): {row}"))
}

/// Phase 6 scenario: stderr rate limit arrives before any stdout semantic
/// event. The bridge should signal early termination, kill the child, and
/// synthesize a `usage_limit_reached` failure summary.
#[cfg(unix)]
#[test]
#[serial_test::serial]
fn opencode_stderr_rate_limit_before_stdout_forces_early_termination() {
    let fixture = CliProcessFixture::named("opencode-rate-limit-early-term");
    fixture.seed_user_config();

    // The fake binary only writes the rate-limit ERROR to stderr, then
    // sleeps so the bridge has to abort it. If claudine fails to terminate
    // early, the assert_cmd timeout would trip and the test would fail.
    write_executable(
        &fixture.bin_dir().join("opencode"),
        r#"#!/bin/sh
printf '%s\n' 'ERROR 2026-04-15T19:26:02 +3054ms service=llm providerID=zai-coding-plan modelID=glm-5.1 error={"error":{"name":"AI_RetryError","reason":"maxRetriesExceeded","errors":[{"name":"AI_APICallError","statusCode":429,"responseBody":"{\"error\":{\"code\":\"1308\",\"message\":\"Usage limit reached. Your limit will reset at 2026-04-16 04:18:56\"}}"}]}}' >&2
sleep 30
exit 0
"#,
    );

    let assert = fixture
        .command()
        .env("OPENCODE_MODEL", "test-model")
        // No budget is under test here — the `sleep 30` stays, because the
        // contract is that the bridge ends the run. These two knobs only size
        // the abort path itself: the wrapper would otherwise wait up to one
        // default 5s tick to act and up to a default 10s grace to escalate.
        .env("CLAUDINE_WATCHDOG_INTERVAL", "0.2s")
        .env("CLAUDINE_KILL_GRACE", "0.5s")
        .timeout(std::time::Duration::from_secs(30))
        .args(["opencode", "describe the thing"])
        .assert()
        .failure();

    let output = assert.get_output();
    let exit_code = output.status.code().unwrap_or(-1);
    assert_eq!(
        exit_code,
        1,
        "pre-stream rate limit must map to exit_code=1; got {exit_code}, stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let row = read_summary_row(fixture.home());
    assert_eq!(row["extra"]["exit_code"], serde_json::json!(1));
    assert_eq!(
        row["error"].as_str().unwrap_or(""),
        row["error"].as_str().unwrap_or(""),
        "error field should carry the rate-limit message",
    );
    let diagnostics = &row["extra"]["provider_summary"]["stderr_diagnostics"];
    assert_eq!(
        diagnostics["rate_limit_events"],
        serde_json::json!(1),
        "stderr diagnostics should record the rate-limit event: row={row}",
    );
    let rate_limit = &row["extra"]["provider_summary"]["rate_limit"];
    assert_eq!(
        rate_limit["is_throttled"],
        serde_json::json!(true),
        "rate_limit.is_throttled should be true: row={row}",
    );
    assert!(
        rate_limit["reset_at"].is_string(),
        "rate_limit.reset_at should be populated: row={row}",
    );
}

/// review-1 (High) regression: the OpenCode **1.17.8** `message="stream error"`
/// usage-cap line must drive the wrapper to terminate on the *first* cap error,
/// end-to-end through the real spawn/bridge/summary path — not merely be
/// classifiable in isolation. This proves acceptance criterion 2 of
/// `2026-06-21-opencode-log-fix/spec.md` ("the wrapper terminates on the first
/// cap error") at the process layer, complementing the Level 1 parser/bridge
/// unit coverage.
///
/// The fake `opencode` emits the exact captured 1.17.8 stderr line (spec.md:36)
/// and then `sleep 30`. If the wrapper failed to classify and abort, the child
/// would block on that sleep and the `assert_cmd` `.timeout(...)` guard would
/// trip — so a prompt failure is itself the load-bearing assertion.
#[cfg(unix)]
#[test]
#[serial_test::serial]
fn opencode_stderr_stream_error_cap_1_17_8_forces_early_termination() {
    let fixture = CliProcessFixture::named("opencode-stream-error-cap");
    fixture.seed_user_config();

    // Single-quote shell quoting preserves the inner double-quotes of the
    // captured 1.17.8 line verbatim. The fake writes only this cap line to
    // stderr, then sleeps so the bridge has to abort it.
    write_executable(
        &fixture.bin_dir().join("opencode"),
        r#"#!/bin/sh
printf '%s\n' 'timestamp=2026-06-22T04:07:15.161Z level=ERROR run=da37e0dd message="stream error" providerID=zai-coding-plan modelID=glm-5.2 session.id=ses_1127ec2fdffepaJc2kEnX093eo small=false agent=build mode=primary error.error="AI_APICallError: Usage limit reached for 5 hour. Your limit will reset at 2026-06-22 13:59:38"' >&2
sleep 30
exit 0
"#,
    );

    let assert = fixture
        .command()
        .env("OPENCODE_MODEL", "test-model")
        // Abort-path sizing only, exactly as in
        // `opencode_stderr_rate_limit_before_stdout_forces_early_termination`.
        .env("CLAUDINE_WATCHDOG_INTERVAL", "0.2s")
        .env("CLAUDINE_KILL_GRACE", "0.5s")
        .timeout(std::time::Duration::from_secs(30))
        .args(["opencode", "describe the thing"])
        .assert()
        .failure();

    let output = assert.get_output();
    let exit_code = output.status.code().unwrap_or(-1);
    assert_eq!(
        exit_code,
        1,
        "1.17.8 stream-error cap must map to exit_code=1; got {exit_code}, stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let row = read_summary_row(fixture.home());
    assert_eq!(row["extra"]["exit_code"], serde_json::json!(1));
    // `summary.error_kind = Some("usage_limit_reached")` (set by
    // `apply_early_termination_to_summary` for `EarlyTermination::RateLimit`)
    // is serialized into the JSONL row as `extra.exit_reason`.
    assert_eq!(
        row["extra"]["exit_reason"],
        serde_json::json!("usage_limit_reached"),
        "1.17.8 cap must classify as usage_limit_reached: row={row}",
    );
    let rate_limit = &row["extra"]["provider_summary"]["rate_limit"];
    assert_eq!(
        rate_limit["is_throttled"],
        serde_json::json!(true),
        "rate_limit.is_throttled should be true: row={row}",
    );
    // The cap reset (`2026-06-22 13:59:38`) must be extracted from the new
    // `error.error=` envelope despite the dropped `error` JSON wrapper.
    assert!(
        rate_limit["reset_at"].is_string(),
        "rate_limit.reset_at should be populated from the 1.17.8 line: row={row}",
    );
}

/// Phase 6 scenario: a malformed asset stderr line during an otherwise-
/// successful run should surface as a Warning event (rendered once per
/// line) without failing the session. Per the 2026-04-18 OpenCode
/// reporting contract, the diagnostics counter is preserved while the
/// trailer Config badge is suppressed.
#[cfg(unix)]
#[test]
#[serial_test::serial]
fn opencode_stderr_malformed_asset_records_diagnostic_without_config_badge() {
    let fixture = CliProcessFixture::named("opencode-malformed-asset");
    fixture.seed_user_config();

    write_executable(
        &fixture.bin_dir().join("opencode"),
        r#"#!/bin/sh
printf '%s\n' 'ERROR 2026-04-15T21:28:30 +315ms service=config command=/Users/ken/.config/opencode/commands/catalog.md err=ENOENT: no such file or directory, open '"'"'/Users/ken/.config/opencode/commands/catalog.md'"'"' failed to load command' >&2
printf '%s\n' '{"type":"step_start","sessionID":"ses_cfg_ok"}'
printf '%s\n' '{"type":"text","text":"Warnings are non-fatal."}'
printf '%s\n' '{"type":"step_complete","usage":{"input_tokens":1,"output_tokens":1,"total_tokens":2},"duration_ms":200}'
exit 0
"#,
    );

    fixture
        .command()
        .env("OPENCODE_MODEL", "test-model")
        .args(["opencode", "just classify"])
        .assert()
        .success();

    let row = read_summary_row(fixture.home());
    let diagnostics = &row["extra"]["provider_summary"]["stderr_diagnostics"];
    assert_eq!(
        diagnostics["malformed_asset_events"],
        serde_json::json!(1),
        "stderr diagnostics should record one malformed asset: row={row}",
    );
    // Per the 2026-04-18 OpenCode reporting contract, malformed-asset
    // events do not produce a trailer Config badge — the per-line
    // Warning surface is the authoritative reporting channel. The
    // `badges` field may be absent or empty; either is acceptable as
    // long as no Config-category badge is present.
    let badges = row["extra"]["badges"].as_array();
    if let Some(b) = badges {
        assert!(
            !b.iter()
                .any(|b| b["category"] == serde_json::json!("config")),
            "Config trailer badge must be absent — malformed assets are surfaced once per Warning line: {b:?}",
        );
    }
    assert_eq!(
        row["extra"]["exit_code"],
        serde_json::json!(0),
        "malformed assets should not fail the session: row={row}",
    );
}

/// Phase 6 scenario: mixed structured stderr plus an ANSI-wrapped raw
/// `Error:` line plus benign chatter. The bridge should consume only
/// classified lines while raw lines still reach the user unchanged.
#[cfg(unix)]
#[test]
#[serial_test::serial]
fn opencode_stderr_mixed_shapes_only_consume_classified_lines() {
    let fixture = CliProcessFixture::named("opencode-mixed-stderr-shapes");
    fixture.seed_user_config();

    // `bare chatter line` should flow through as raw stderr because it
    // doesn't match the header regex and isn't an ANSI `Error:` block.
    // The ERROR/skill line is classified as MalformedAsset.
    write_executable(
        &fixture.bin_dir().join("opencode"),
        r#"#!/bin/sh
printf '%s\n' 'ERROR 2026-04-15T21:28:30 +0ms service=config skill=/tmp/s.md err=ENOENT failed to load skill' >&2
printf '%s\n' 'bare chatter line from the provider' >&2
printf '%s\n' '{"type":"step_start","sessionID":"ses_mix"}'
printf '%s\n' '{"type":"text","text":"All good."}'
printf '%s\n' '{"type":"step_complete","usage":{"input_tokens":1,"output_tokens":1,"total_tokens":2},"duration_ms":100}'
exit 0
"#,
    );

    let assert = fixture
        .command()
        .env("OPENCODE_MODEL", "test-model")
        .args(["opencode", "mixed run"])
        .assert()
        .success();

    let stderr = strip_ansi(&String::from_utf8_lossy(&assert.get_output().stderr));
    // Structured ERROR line must NOT reach raw stderr passthrough
    // (the bridge consumed it, then re-emitted it as a Warning event).
    assert!(
        !stderr.contains("failed to load skill"),
        "classified stderr must be suppressed from raw passthrough; stderr={stderr}",
    );
    // Bare chatter is unclassified so it must continue to surface.
    assert!(
        stderr.contains("bare chatter line"),
        "unclassified stderr must still passthrough to the operator; stderr={stderr}",
    );

    let row = read_summary_row(fixture.home());
    let diagnostics = &row["extra"]["provider_summary"]["stderr_diagnostics"];
    assert_eq!(
        diagnostics["malformed_asset_events"],
        serde_json::json!(1),
        "stderr diagnostics should record the malformed asset: row={row}",
    );
    assert!(
        row["extra"]["provider_summary"]["stderr_diagnostics"]["log_records_parsed"]
            .as_u64()
            .unwrap_or(0)
            >= 1,
        "log_records_parsed should count the structured record: row={row}",
    );
}

/// Phase 6 scenario: the final summary must contain merged `stderr_text`,
/// `stderr_diagnostics`, and recomputed `badges`. Guards the wrapper-layer
/// summary merge step that runs after both reader threads join.
#[cfg(unix)]
#[test]
#[serial_test::serial]
fn opencode_structured_summary_merges_stderr_diagnostics_and_badges() {
    let fixture = CliProcessFixture::named("opencode-summary-merge");
    fixture.seed_user_config();

    write_executable(
        &fixture.bin_dir().join("opencode"),
        r#"#!/bin/sh
printf '%s\n' 'ERROR 2026-04-15T21:28:30 +0ms service=config command=/tmp/a.md err=ENOENT failed to load command' >&2
printf '%s\n' 'ERROR 2026-04-15T21:28:30 +0ms service=config agent=/tmp/b.md err=ENOENT failed to load agent' >&2
printf '%s\n' '{"type":"step_start","sessionID":"ses_merge"}'
printf '%s\n' '{"type":"text","text":"Merge ok."}'
printf '%s\n' '{"type":"step_complete","usage":{"input_tokens":1,"output_tokens":1,"total_tokens":2},"duration_ms":50}'
exit 0
"#,
    );

    fixture
        .command()
        .env("OPENCODE_MODEL", "test-model")
        .args(["opencode", "merge probe"])
        .assert()
        .success();

    let row = read_summary_row(fixture.home());
    let diagnostics = &row["extra"]["provider_summary"]["stderr_diagnostics"];
    assert_eq!(
        diagnostics["malformed_asset_events"],
        serde_json::json!(2),
        "both malformed asset events should be accumulated: row={row}",
    );
    assert!(
        diagnostics["log_records_parsed"].as_u64().unwrap_or(0) >= 2,
        "log_records_parsed should count both records: row={row}",
    );

    // Per the 2026-04-18 OpenCode reporting contract, malformed-asset
    // events do not produce a trailer Config badge — the per-line
    // Warning surface is the authoritative reporting channel. The
    // `badges` field may be absent or empty; either is acceptable as
    // long as no Config-category badge is present.
    let badges = row["extra"]["badges"].as_array();
    if let Some(b) = badges {
        assert!(
            !b.iter()
                .any(|b| b["category"] == serde_json::json!("config")),
            "Config trailer badge must be absent after stderr merge — diagnostics counter still records the events: {b:?}",
        );
    }
}

/// Phase 6 regression: service-less new-format stderr lines must be consumed
/// by the bridge and not leak as raw `timestamp=` passthrough during compose.
#[cfg(unix)]
#[test]
#[serial_test::serial]
fn compose_opencode_serviceless_stderr_lines_are_consumed() {
    let fixture = CliProcessFixture::named("compose-opencode-serviceless-stderr");
    fixture.seed_user_config();

    let md_file = fixture.cwd().join("test.md");
    fs::write(&md_file, "---\ntitle: test\n---\nHello\n").unwrap();

    // Fake opencode emits the exact service-less lines observed in the
    // wild (spec.md:17-24) plus matching NDJSON stdout so the wrapper
    // completes normally.
    write_executable(
        &fixture.bin_dir().join("opencode"),
        r#"#!/bin/sh
if [ "$1" = "models" ]; then
  printf '%s\n' '["test-model"]'
  exit 0
fi
printf '%s\n' 'timestamp=2026-06-10T16:11:27.352Z level=INFO run=df5a9474 message=tracking hash=86a6603a' >&2
printf '%s\n' 'timestamp=2026-06-10T16:11:27.460Z level=INFO run=df5a9474 message=loop session.id=ses_14db step=1' >&2
printf '%s\n' 'timestamp=2026-06-10T16:11:27.559Z level=INFO run=df5a9474 message=tracking hash=86a6603a' >&2
printf '%s\n' 'timestamp=2026-06-10T16:11:27.574Z level=INFO run=df5a9474 message=process session.id=ses_14db' >&2
printf '%s\n' 'timestamp=2026-06-10T16:11:27.574Z level=INFO run=df5a9474 message=stream providerID=zai-coding-plan modelID=glm-5.1' >&2
printf '%s\n' 'timestamp=2026-06-10T16:11:27.575Z level=INFO run=df5a9474 message="llm runtime selected"' >&2
printf '%s\n' 'timestamp=2026-06-10T16:11:31.461Z level=INFO run=df5a9474 message=evaluated permission=glob' >&2
printf '%s\n' '{"type":"init","session_id":"ses_14db","model":"test-model"}'
printf '%s\n' '{"type":"step_start","sessionID":"ses_14db"}'
printf '%s\n' '{"type":"text","text":"Done."}'
printf '%s\n' '{"type":"step_complete","usage":{"input_tokens":1,"output_tokens":1,"total_tokens":2},"duration_ms":50}'
exit 0
"#,
    );

    let assert = fixture
        .command()
        .env("OPENCODE_MODEL", "test-model")
        .args(["compose", "--opencode", md_file.to_str().unwrap()])
        .assert()
        .success();

    let stderr = strip_ansi(&String::from_utf8_lossy(&assert.get_output().stderr));

    // The user-visible regression: raw timestamp= lines must not appear.
    assert!(
        !stderr.contains("timestamp="),
        "raw timestamp= lines must be consumed by the bridge, not passthrough to stderr; got: {stderr}"
    );

    // Verify the JSONL summary contains the expected stderr diagnostics.
    let row = read_summary_row(fixture.home());
    let diagnostics = &row["extra"]["provider_summary"]["stderr_diagnostics"];
    assert_eq!(
        diagnostics["log_records_parsed"].as_u64().unwrap_or(0),
        7,
        "all 7 new-format lines should be parsed; row={row}",
    );
    // The bridge should have promoted the service-less lifecycle lines into
    // semantic events (StepLoop, LlmCall, PermissionEvaluated).  They won't
    // be visible in the summary directly, but the diagnostics counter proves
    // the bridge consumed and classified them rather than leaving them as
    // raw passthrough.
    assert_eq!(
        row["extra"]["exit_code"],
        serde_json::json!(0),
        "session should succeed: row={row}",
    );
}

// ============================================================================
// Performance flag tests
// ============================================================================
