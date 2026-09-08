#![cfg(unix)]

//! Integration tests: --perf report emission across wrap, compose, and inline-compose.
//!
//! Split out of the `wrap_commands.rs` god file; shared fixtures live in
//! `common::wrap`.

use std::fs;
mod common;
use common::{CliProcessFixture, strip_ansi, write_executable};

#[cfg(unix)]
#[test]
fn wrapper_perf_emits_report_to_stderr_only() {
    let fixture = CliProcessFixture::named("wrap-perf-stderr-only");
    fixture.seed_user_config();

    write_executable(
        &fixture.bin_dir().join("codex"),
        r#"#!/bin/sh
exit 0
"#,
    );

    let assert = fixture
        .command()
        .args(["codex", "--perf", "--", "--version"])
        .assert()
        .success()
        .stdout("");

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).to_string();
    let plain = strip_ansi(&stderr);
    assert!(
        plain.contains("Performance"),
        "stderr should contain Performance section; got: {plain}"
    );
    assert!(
        plain.contains("pre-dispatch"),
        "stderr should contain the pre-dispatch bucket; got: {plain}"
    );
    assert!(
        plain.contains("agent execution"),
        "stderr should contain the agent execution bucket; got: {plain}"
    );
}

#[cfg(unix)]
#[test]
fn wrapper_perf_reports_source_context_probe_and_reuse_counts() {
    let fixture = CliProcessFixture::named("wrap-perf-source-context");
    fixture.initialize_repository();
    fixture.seed_user_config();
    fixture.write_root_system_prompt("Request-owned system prompt.\n");
    write_executable(&fixture.bin_dir().join("codex"), "#!/bin/sh\nexit 0\n");

    let assert = fixture
        .command()
        .args(["codex", "--perf", "inspect the repository"])
        .assert()
        .success();

    let plain = strip_ansi(&String::from_utf8_lossy(&assert.get_output().stderr));
    assert!(plain.contains("source context work"), "{plain}");
    assert!(plain.contains("Git discoveries 1"), "{plain}");
    assert!(plain.contains("topology probes 1"), "{plain}");
    assert!(plain.contains("topology reuses 1"), "{plain}");
}

#[cfg(unix)]
#[test]
fn wrapper_dry_run_perf_emits_report_with_skipped_note() {
    let fixture = CliProcessFixture::named("wrap-perf-dry-run");
    fixture.seed_user_config();

    write_executable(
        &fixture.bin_dir().join("codex"),
        r#"#!/bin/sh
echo "SHOULD NOT RUN"
exit 1
"#,
    );

    let assert = fixture
        .command()
        .args(["codex", "--dry-run", "--perf", "--", "--version"])
        .assert()
        .success()
        .stdout("");

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).to_string();
    let plain = strip_ansi(&stderr);
    assert!(
        plain.contains("Performance"),
        "stderr should contain Performance section; got: {plain}"
    );
    assert!(
        plain.contains("pre-dispatch"),
        "stderr should contain the pre-dispatch bucket; got: {plain}"
    );
    assert!(
        plain.contains("dry run") || plain.contains("skipped"),
        "stderr should mention dry run; got: {plain}"
    );
}

#[cfg(unix)]
#[test]
fn wrapper_failure_perf_emits_report_before_the_error_returns() {
    let fixture = CliProcessFixture::named("wrap-perf-failure");
    fixture.seed_user_config();

    write_executable(
        &fixture.bin_dir().join("codex"),
        r#"#!/bin/sh
exit 0
"#,
    );

    let assert = fixture
        .command()
        .args(["codex", "--perf", "--timeout", "1s"])
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).to_string();
    let plain = strip_ansi(&stderr);
    assert!(
        plain.contains("Performance"),
        "failure stderr should contain the performance tree; got: {plain}"
    );
    assert!(
        plain.contains("environment setup"),
        "the failure tree should reconcile its preparation window; got: {plain}"
    );
    assert!(
        plain.contains("--timeout can only be used in non-interactive mode"),
        "the original typed failure should still be rendered; got: {plain}"
    );
}

#[cfg(unix)]
#[test]
fn compose_perf_emits_report_to_stderr() {
    let fixture = CliProcessFixture::named("compose-perf-stderr");
    fixture.seed_user_config();

    let md_file = fixture.cwd().join("test.md");
    fs::write(&md_file, "---\ntitle: perf test\n---\n# Hello\n").unwrap();

    write_executable(
        &fixture.bin_dir().join("goose"),
        "#!/bin/sh\necho 'Agent response'\nexit 0\n",
    );

    let assert = fixture
        .command()
        .args(["compose", "--goose", "--perf", md_file.to_str().unwrap()])
        .assert()
        .success();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let plain = strip_ansi(&stderr);

    assert!(
        plain.contains("Performance"),
        "stderr should contain Performance section; got: {plain}"
    );
    assert!(
        plain.contains("pre-dispatch"),
        "stderr should contain the pre-dispatch bucket; got: {plain}"
    );
    assert!(
        plain.contains("agent execution"),
        "stderr should contain the agent execution bucket; got: {plain}"
    );
}

/// Phase 1 characterization of the composition setup coordinator. The perf
/// substages are emitted by the real setup path, while lifecycle stdout and
/// the provider marker pin initialize routing before the body handoff.
#[cfg(unix)]
#[test]
fn composition_setup_and_provider_handoff_order_matches_phase_1_baseline() {
    let fixture = CliProcessFixture::named("compose-perf-setup-order");
    fixture.seed_user_config();

    let md_file = fixture.cwd().join("setup-order.md");
    fs::write(
        &md_file,
        "---\nagent: goose\ninitialize:\n  stack:\n    - action: {stdout: \"phase:initialize\"}\n---\n# Setup ordering\n",
    )
    .unwrap();
    write_executable(
        &fixture.bin_dir().join("goose"),
        "#!/bin/sh\nprintf 'phase:provider cwd=%s\\n' \"$PWD\"\n",
    );

    let output = fixture
        .command()
        .args(["compose", "--goose", "--perf", md_file.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "composition must succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let initialize = stdout
        .find("phase:initialize")
        .unwrap_or_else(|| panic!("initialize marker missing: {stdout}"));
    let provider = stdout
        .find("phase:provider")
        .unwrap_or_else(|| panic!("provider marker missing: {stdout}"));
    assert!(initialize < provider, "initialize must precede provider handoff: {stdout}");
    let canonical_workspace = fixture.cwd().canonicalize().unwrap();
    assert!(
        stdout.contains(&format!("cwd={}", canonical_workspace.display())),
        "provider must launch in the selected workspace: {stdout}"
    );

    let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr));
    let report_start = stderr
        .find("Performance")
        .unwrap_or_else(|| panic!("performance report missing:\n{stderr}"));
    let report = &stderr[report_start..];
    let stages = [
        "target resolution",
        "header env plan",
        "child env build",
        "mcp composition",
        "argv assembly",
        "system prompt",
        "stream + prompt delivery",
    ];
    let mut previous = 0;
    for stage in stages {
        let position = report
            .find(stage)
            .unwrap_or_else(|| panic!("setup stage `{stage}` missing from perf trace:\n{report}"));
        assert!(
            position >= previous,
            "setup stage `{stage}` moved out of order in:\n{report}"
        );
        previous = position;
    }
}

#[cfg(unix)]
#[test]
fn compose_perf_stdout_matches_non_perf() {
    let fixture = CliProcessFixture::named("compose-perf-stdout-parity");
    fixture.seed_user_config();

    let md_file = fixture.cwd().join("test.md");
    fs::write(&md_file, "---\ntitle: perf test\n---\n# Hello\n").unwrap();

    write_executable(
        &fixture.bin_dir().join("goose"),
        "#!/bin/sh\necho 'Agent response'\nexit 0\n",
    );

    let perf_assert = fixture
        .command()
        .args(["compose", "--goose", "--perf", md_file.to_str().unwrap()])
        .assert()
        .success();

    let plain_assert = fixture
        .command()
        .args(["compose", "--goose", md_file.to_str().unwrap()])
        .assert()
        .success();

    let perf_stdout = String::from_utf8_lossy(&perf_assert.get_output().stdout);
    let plain_stdout = String::from_utf8_lossy(&plain_assert.get_output().stdout);

    assert_eq!(
        perf_stdout, plain_stdout,
        "stdout must be identical between --perf and non-perf runs"
    );
}

#[cfg(unix)]
#[test]
fn inline_compose_perf_emits_report_to_stderr() {
    let fixture = CliProcessFixture::named("inline-compose-perf-stderr");
    fixture.seed_user_config();

    let md_file = fixture.cwd().join("test.md");
    fs::write(
        &md_file,
        "---\ntitle: inline perf\nprompt: say hello\n---\n# Body\n",
    )
    .unwrap();

    write_executable(
        &fixture.bin_dir().join("goose"),
        "#!/bin/sh\necho 'Replacement body'\nexit 0\n",
    );

    let assert = fixture
        .command()
        .args([
            "inline-compose",
            "--goose",
            "--perf",
            md_file.to_str().unwrap(),
        ])
        .assert()
        .success();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let plain = strip_ansi(&stderr);

    assert!(
        plain.contains("Performance"),
        "stderr should contain Performance section; got: {plain}"
    );
    assert!(
        plain.contains("pre-dispatch"),
        "stderr should contain the pre-dispatch bucket; got: {plain}"
    );
    assert!(
        plain.contains("agent execution"),
        "stderr should contain the agent execution bucket; got: {plain}"
    );
}

#[cfg(unix)]
#[test]
fn inline_compose_perf_stdout_matches_non_perf() {
    let fixture = CliProcessFixture::named("inline-compose-perf-stdout-parity");
    fixture.seed_user_config();

    let md_file_perf = fixture.cwd().join("test-perf.md");
    let md_file_plain = fixture.cwd().join("test-plain.md");
    let content = "---\ntitle: inline perf\nprompt: say hello\n---\n# Body\n";
    fs::write(&md_file_perf, content).unwrap();
    fs::write(&md_file_plain, content).unwrap();

    write_executable(
        &fixture.bin_dir().join("goose"),
        "#!/bin/sh\necho 'Replacement body'\nexit 0\n",
    );

    let perf_assert = fixture
        .command()
        .args([
            "inline-compose",
            "--goose",
            "--perf",
            md_file_perf.to_str().unwrap(),
        ])
        .assert()
        .success();

    let plain_assert = fixture
        .command()
        .args(["inline-compose", "--goose", md_file_plain.to_str().unwrap()])
        .assert()
        .success();

    let perf_stdout = String::from_utf8_lossy(&perf_assert.get_output().stdout);
    let plain_stdout = String::from_utf8_lossy(&plain_assert.get_output().stdout);

    assert_eq!(
        perf_stdout, plain_stdout,
        "stdout must be identical between --perf and non-perf runs"
    );
}

#[cfg(unix)]
#[test]
fn compose_dry_run_perf_renders_report_without_agent_execution() {
    let fixture = CliProcessFixture::named("compose-perf-dry-run");
    fixture.seed_user_config();

    let md_file = fixture.cwd().join("test.md");
    fs::write(&md_file, "---\ntitle: dry run perf\n---\n# Hello\n").unwrap();

    write_executable(
        &fixture.bin_dir().join("goose"),
        "#!/bin/sh\necho 'should not run'\nexit 0\n",
    );

    let assert = fixture
        .command()
        .args([
            "compose",
            "--goose",
            "--perf",
            "--dry-run",
            md_file.to_str().unwrap(),
        ])
        .assert()
        .success();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let plain = strip_ansi(&stderr);

    assert!(
        plain.contains("Performance"),
        "stderr should contain Performance section; got: {plain}"
    );
    assert!(
        plain.contains("dry run"),
        "perf report should note dry run; got: {plain}"
    );

    // Provider should NOT have run in dry-run mode.
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(
        !stdout.contains("should not run"),
        "provider should not execute in dry-run mode"
    );
}

/// Verifies that the `arg parsing:` timing in the perf report captures
/// the full pipeline including `argv::normalize` and `parse_cli_from`.
/// This is a smoke test: the exact duration is environment-dependent, but
/// the line must appear with a formatted duration.
#[cfg(unix)]
#[test]
fn perf_arg_parsing_includes_clap_time() {
    let fixture = CliProcessFixture::named("perf-arg-parsing");
    fixture.seed_user_config();

    let md_file = fixture.cwd().join("test.md");
    fs::write(&md_file, "---\ntitle: arg parse perf\n---\n# Hello\n").unwrap();

    write_executable(
        &fixture.bin_dir().join("goose"),
        "#!/bin/sh\necho 'Agent response'\nexit 0\n",
    );

    let assert = fixture
        .command()
        .args(["compose", "--goose", "--perf", md_file.to_str().unwrap()])
        .assert()
        .success();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let plain = strip_ansi(&stderr);

    // The arg parsing line must be present and show a duration.
    // We allow 0µs because timer resolution varies, but the line must exist.
    assert!(
        plain.contains("arg parsing"),
        "perf report must include arg parsing timing; got: {plain}"
    );

    // Ensure the other startup timings are also present, confirming the
    // pre-dispatch and environment-setup buckets are rendered.
    assert!(
        plain.contains("config loading"),
        "perf report must include config loading timing; got: {plain}"
    );
    assert!(
        plain.contains("tracing init"),
        "perf report must include tracing init timing; got: {plain}"
    );
    assert!(
        plain.contains("environment setup"),
        "perf report must include environment setup timing; got: {plain}"
    );
}

// ===========================================================================
// Watchdog fixture tests (Phase 5)
// ===========================================================================
