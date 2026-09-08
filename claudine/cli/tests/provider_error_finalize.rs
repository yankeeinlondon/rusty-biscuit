//! Level-1 process coverage for a finalize `error` after exhausted remediation.

mod common;

use common::CliProcessFixture;
use common::write;
#[cfg(unix)]
use common::write_executable;
use std::fs;
use std::path::Path;

const FINDINGS_REPORT: &str = "---\nstatus: findings\nprovider: claude\nfindings:\n  - invalid research remains\n---\n";

fn write_persistently_invalid_claude(bin_dir: &Path) {
    #[cfg(unix)]
    write_executable(
        &bin_dir.join("claude"),
        r#"#!/bin/sh
printf 'provider-ran\n' >> "$CLAUDINE_INVOCATIONS"
printf '%s\n' '{"type":"system","subtype":"init","session_id":"session-1","model":"claude-test"}'
printf '%s\n' '{"type":"result","subtype":"success","result":"research unchanged","session_id":"session-1","is_error":false}'
exit 0
"#,
    );

    #[cfg(windows)]
    write(
        &bin_dir.join("claude.cmd"),
        "@echo off\r\n\
echo provider-ran>>\"%CLAUDINE_INVOCATIONS%\"\r\n\
echo {\"type\":\"system\",\"subtype\":\"init\",\"session_id\":\"session-1\",\"model\":\"claude-test\"}\r\n\
echo {\"type\":\"result\",\"subtype\":\"success\",\"result\":\"research unchanged\",\"session_id\":\"session-1\",\"is_error\":false}\r\n\
exit /b 0\r\n",
    );
}

#[test]
fn exhausted_resumes_fail_command_and_preserve_findings_report() {
    let fixture = CliProcessFixture::named("provider-error-finalize");
    fixture.seed_user_config();
    write_persistently_invalid_claude(fixture.bin_dir());

    let findings = fixture.cwd().join("findings.md");
    write(&findings, FINDINGS_REPORT);
    let invocations = fixture.cwd().join("invocations.log");
    let prompt = fixture.cwd().join("research.md");
    let findings_yaml = serde_json::to_string(&findings.to_string_lossy()).expect("quote path");
    write(
        &prompt,
        &format!(
            r#"---
findings: {findings_yaml}
success:
  stack:
    - when: "frontmatter(findings, 'status') == 'findings'"
      action:
        - action: resume
          message: "Correct the remaining deterministic findings."
          max_attempts: 2
finalize:
  stack:
    - when: "frontmatter(findings, 'status') != 'clean'"
      action:
        - error: "deterministic gate did not reach a clean outcome"
---
Research remains invalid.
"#,
        ),
    );

    let assertion = fixture
        .command()
        .env("CLAUDINE_INVOCATIONS", &invocations)
        .args(["compose", "--claude", prompt.to_str().expect("UTF-8 prompt path")])
        .assert()
        .failure();

    assert_eq!(
        fs::read_to_string(&invocations)
            .expect("provider invocation log")
            .lines()
            .count(),
        3,
        "the initial run and both resume attempts must execute"
    );
    assert_eq!(
        fs::read_to_string(&findings).expect("durable findings report"),
        FINDINGS_REPORT,
        "finalize must not mutate the machine-readable findings report"
    );
    let stderr = String::from_utf8_lossy(&assertion.get_output().stderr);
    assert!(
        stderr.contains("deterministic gate did not reach a clean outcome"),
        "the command failure should retain the authored finalize reason: {stderr}"
    );
}
