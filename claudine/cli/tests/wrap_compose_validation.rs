//! Integration tests: compose argument validation, retired-flag rejection, and dry-run stdout/stderr separation.
//!
//! Split out of the `wrap_commands.rs` god file; shared fixtures live in
//! `common::wrap`.

use predicates::str::contains;
use std::fs;
use tempfile::tempdir;
mod common;
use common::wrap::*;
use common::{augmented_path, strip_ansi, write_executable};

#[test]
fn compose_requires_positional_arg() {
    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .args(["compose"])
        .assert()
        .code(2);

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let plain = strip_ansi(&stderr);
    assert!(plain.contains("ARG"), "usage should show ARG positional");
}

#[test]
fn compose_missing_file_with_setter_only() {
    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .args(["compose", "key=val"])
        .assert()
        .code(1);
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let plain = strip_ansi(&stderr);
    assert!(
        plain.contains("missing file reference"),
        "expected missing-file error, got: {plain}"
    );
}

#[test]
fn compose_empty_key_setter_errors() {
    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .args(["compose", "=foo"])
        .assert()
        .code(1);
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let plain = strip_ansi(&stderr);
    assert!(
        plain.contains("setter key must not be empty"),
        "expected empty-key setter error, got: {plain}"
    );
}

#[test]
fn compose_multiple_file_candidates_errors() {
    let workspace = tempdir().unwrap();
    let a = workspace.path().join("a.md");
    let b = workspace.path().join("b.md");
    fs::write(&a, "---\n---\nbody\n").unwrap();
    fs::write(&b, "---\n---\nbody\n").unwrap();

    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .args(["compose", a.to_str().unwrap(), b.to_str().unwrap()])
        .assert()
        .code(1);
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let plain = strip_ansi(&stderr);
    assert!(
        plain.contains("multiple"),
        "expected multiple-file error, got: {plain}"
    );
}

#[test]
fn compose_rejects_nonexistent_file() {
    assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .args(["compose", "/nonexistent/path/to/file.md"])
        .assert()
        .code(1);
}

#[test]
fn compose_rejects_non_markdown_file() {
    let workspace = tempdir().unwrap();
    let txt_file = workspace.path().join("file.txt");
    fs::write(&txt_file, "hello").unwrap();

    assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .args(["compose", txt_file.to_str().unwrap()])
        .assert()
        .code(1);
}

#[cfg(unix)]
#[test]
fn compose_missing_explicit_system_prompt_fails_visibly() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    let md_file = workspace.path().join("prompt.md");
    let missing_prompt = workspace.path().join("missing-system-prompt.md");
    fs::create_dir_all(&path_dir).unwrap();
    seed_minimal_config(workspace.path());
    fs::write(&md_file, "---\ntitle: test\n---\nHello compose\n").unwrap();

    write_executable(&path_dir.join("codex"), "#!/bin/sh\nexit 0\n");

    assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", &path_dir)
        .args([
            "compose",
            "--codex",
            "--append-system-prompt",
            missing_prompt.to_str().unwrap(),
            md_file.to_str().unwrap(),
        ])
        .assert()
        .code(1)
        // The path is named, but a `StatusBlock` word-wraps it at the terminal
        // width, so match the file name's tail rather than the whole path — the
        // assertion is "the operator is told which file", not "the path is on
        // one line".
        .stderr(contains("system prompt file not found"))
        .stderr(contains("system-prompt.md"))
        // `ClaudineError` reaches the walker through the diagnostic registry,
        // so this renders a coded block rather than the generic `Error:` line.
        .stderr(contains("io.read_failed"));
}

#[cfg(unix)]
#[test]
fn no_cross_provider_retry_after_launch() {
    // Verifies that after a provider is launched and fails, Claudine
    // does NOT automatically retry with another provider. The exit code
    // from the single provider invocation is returned directly.
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    seed_minimal_config(workspace.path());

    let md_file = workspace.path().join("test.md");
    fs::write(&md_file, "---\ntitle: test\n---\nPrompt body\n").unwrap();

    // Provider that exits with error code 42
    write_executable(
        &path_dir.join("codex"),
        r#"#!/bin/sh
exit 42
"#,
    );

    // Also install a "claude" that succeeds -- if retry happened, we'd see code 0
    write_executable(
        &path_dir.join("claude"),
        r#"#!/bin/sh
exit 0
"#,
    );

    // Explicitly select codex. It exits 42. No fallback to claude.
    assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", &path_dir)
        .args(["compose", "--codex", md_file.to_str().unwrap()])
        .assert()
        .code(42);
}

#[test]
fn old_compose_inline_command_is_unknown() {
    // Verify that the old `compose-inline` command no longer exists
    assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .args(["compose-inline", "file.md"])
        .assert()
        .code(2); // clap returns 2 for unrecognized subcommands
}

#[test]
fn retired_compose_flag_rejected_in_wrapper() {
    assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .args(["claude", "--compose", "file.md"])
        .assert()
        .failure()
        .stderr(contains("--compose has been retired"))
        .stderr(contains("claudine compose"));
}

#[test]
fn retired_frontmatter_prompt_flag_rejected_in_wrapper() {
    assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .args(["claude", "--frontmatter-prompt", "file.md"])
        .assert()
        .failure()
        .stderr(contains("--frontmatter-prompt has been retired"))
        .stderr(contains("claudine inline-compose"));
}

#[test]
fn retired_prompt_file_flag_rejected_in_wrapper() {
    assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .args(["claude", "--prompt-file", "file.md"])
        .assert()
        .failure()
        .stderr(contains("--prompt-file has been retired"))
        .stderr(contains("claudine compose"));
}

/// Data/status discipline: under `compose --dry-run` the composed body is the
/// *only* thing on **stdout**; the finalized frontmatter and the metadata
/// table land on **stderr**. Verifies the two streams never cross.
#[cfg(unix)]
#[test]
fn compose_dry_run_body_only_on_stdout_metadata_on_stderr() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    seed_minimal_config(workspace.path());

    let md_file = workspace.path().join("doc.md");
    fs::write(
        &md_file,
        "---\nname: disc-doc\ndescription: a discipline doc\nagent: goose\n---\nBODY_MARKER_QQQ\n",
    )
    .unwrap();

    write_executable(&path_dir.join("goose"), "#!/bin/sh\nexit 0\n");

    let output = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .args(["compose", "--goose", "--dry-run", md_file.to_str().unwrap()])
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = strip_ansi(&String::from_utf8_lossy(&output.stdout));
    let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr));

    // stdout: body only — the composed body, none of the metadata-table
    // labels or frontmatter that belong on stderr.
    assert!(
        stdout.contains("BODY_MARKER_QQQ"),
        "stdout should carry the composed body; stdout was:\n{stdout}"
    );
    for leak in ["YOLO", "Document", "Field", "Agent", "name:"] {
        assert!(
            !stdout.contains(leak),
            "stdout must not contain the `{leak}` metadata leaked from stderr; stdout was:\n{stdout}"
        );
    }

    // stderr: horizontal rule, heading, frontmatter (YAML) + metadata table.
    let hr_lines: Vec<&str> = stderr
        .lines()
        .filter(|l| l.len() >= 10 && l.chars().all(|c| c == '╌' || c == '-'))
        .collect();
    assert!(
        !hr_lines.is_empty(),
        "stderr should contain a horizontal rule; stderr was:\n{stderr}"
    );
    assert!(
        stderr.contains("Frontmatter") && stderr.contains("resolved"),
        "stderr should carry the 'Frontmatter (resolved):' heading; stderr was:\n{stderr}"
    );
    assert!(
        stderr.contains("name:") && stderr.contains("description:"),
        "stderr should carry the highlighted frontmatter; stderr was:\n{stderr}"
    );
    for label in ["Document", "Agent", "Model", "YOLO"] {
        assert!(
            stderr.contains(label),
            "stderr should carry the `{label}` metadata-table row; stderr was:\n{stderr}"
        );
    }
}

/// `--quiet` and `--silent` have no effect on `compose --dry-run` output:
/// the body still lands on stdout and the full metadata block on stderr.
#[cfg(unix)]
#[test]
fn compose_dry_run_quiet_and_silent_are_no_op() {
    for flag in ["--quiet", "--silent"] {
        let workspace = tempdir().unwrap();
        let path_dir = workspace.path().join("bin");
        fs::create_dir_all(&path_dir).unwrap();
        seed_minimal_config(workspace.path());

        let md_file = workspace.path().join("doc.md");
        fs::write(
            &md_file,
            "---\nname: qs-doc\nagent: goose\n---\nBODY_MARKER_QQQ\n",
        )
        .unwrap();

        write_executable(&path_dir.join("goose"), "#!/bin/sh\nexit 0\n");

        let output = assert_cmd::Command::cargo_bin("claudine").unwrap()
            .env("NO_COLOR", "1")
            .env("HOME", workspace.path())
            .env("PATH", augmented_path(&path_dir))
            .args([
                "compose",
                "--goose",
                "--dry-run",
                flag,
                md_file.to_str().unwrap(),
            ])
            .output()
            .unwrap();

        assert!(
            output.status.success(),
            "compose dry-run {flag} should succeed; stderr was:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = strip_ansi(&String::from_utf8_lossy(&output.stdout));
        let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr));

        assert!(
            stdout.contains("BODY_MARKER_QQQ"),
            "{flag} must not suppress the composed body on stdout; stdout was:\n{stdout}"
        );
        assert!(
            stderr.contains("YOLO") && stderr.contains("name:"),
            "{flag} must not suppress the dry-run metadata on stderr; stderr was:\n{stderr}"
        );
        assert!(
            stderr.contains("Frontmatter") && stderr.contains("resolved"),
            "{flag} must not suppress the dry-run heading on stderr; stderr was:\n{stderr}"
        );
    }
}

/// Late-binding lifecycle evaluation error (process-level, non-interactive):
/// an `initialize` stack whose `when:` guard references an undefined root
/// *raises* at event time. Under DM2 strict mode this is a crashed expression,
/// not a clean `false` guard, so the run must surface a styled
/// `lifecycle evaluation error` to **stderr** and exit **non-zero** — never a
/// silent success. This is the setup-phase end-to-end proof for the
/// late-binding-error fix (the terminal-phase paths need a real provider run
/// and are covered by the L1 orchestration tests).
#[cfg(unix)]
#[test]
fn compose_initialize_when_evaluation_error_exits_non_zero() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    seed_minimal_config(workspace.path());

    let md_file = workspace.path().join("prompt.md");
    fs::write(
        &md_file,
        "---\nname: late-bind\nagent: goose\ninitialize:\n  stack:\n    \
         - when: \"missing_root == true\"\n      action: {stderr: \"ready\"}\n---\nBODY_MARKER_QQQ\n",
    )
    .unwrap();

    // A stub provider must exist on PATH for preflight, but the initialize
    // raise halts the run before it is ever launched.
    write_executable(&path_dir.join("goose"), "#!/bin/sh\nexit 0\n");

    let output = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .args(["compose", "--goose", md_file.to_str().unwrap()])
        .output()
        .unwrap();

    assert!(
        !output.status.success(),
        "a late-binding evaluation error must exit non-zero; stderr was:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr));
    assert!(
        stderr.contains("evaluation error") && stderr.contains("initialize"),
        "stderr must name the lifecycle evaluation error and the event; stderr was:\n{stderr}"
    );
    // Distinguishes a crashed guard from a clean `false` — the user must not
    // confuse a swallowed raise with a deliberately-skipped branch.
    assert!(
        stderr.contains("crashed expression"),
        "stderr must distinguish a crashed guard from a clean false; stderr was:\n{stderr}"
    );
    // The provider was never launched: the composed body never reached stdout.
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("BODY_MARKER_QQQ"),
        "the body must not be sent after an initialize raise; stdout was:\n{stdout}"
    );
}

/// Late-binding lifecycle evaluation error swallowing — regression for the
/// previously-broken explicit-`error(...)` catch path. When `initialize.error`
/// routes the run to `failure` and the catch `failure.when:` guard references
/// an undefined root, the run must surface the FAILURE evaluation error (the
/// latest lifecycle crash) to stderr and exit non-zero — not swallow it and
/// return only the original `error(...)` reason. This is the explicit-control
/// counterpart to `compose_initialize_when_evaluation_error_exits_non_zero`
/// (which covers the evaluation-error-triggered path).
#[cfg(unix)]
#[test]
fn compose_initialize_error_with_failure_raise_surfaces_failure_evaluation_error() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    seed_minimal_config(workspace.path());

    let md_file = workspace.path().join("prompt.md");
    fs::write(
        &md_file,
        "---\nname: explicit-error\nagent: goose\ninitialize:\n  stack:\n    \
         - action: {error: \"preflight refused\"}\nfailure:\n  stderr: \"fail\"\n  stack:\n    \
         - when: \"missing_root == true\"\n      action: {stderr: \"unreachable\"}\n---\nBODY_MARKER_QQQ\n",
    )
    .unwrap();

    // A stub provider must exist on PATH for preflight, but the initialize
    // error + failure raise halts the run before it is ever launched.
    write_executable(&path_dir.join("goose"), "#!/bin/sh\nexit 0\n");

    let output = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .args(["compose", "--goose", md_file.to_str().unwrap()])
        .output()
        .unwrap();

    assert!(
        !output.status.success(),
        "an evaluation error in the failure catch event must exit non-zero; stderr was:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr));
    assert!(
        stderr.contains("evaluation error"),
        "stderr must mention the lifecycle evaluation error; stderr was:\n{stderr}"
    );
    assert!(
        stderr.contains("failure"),
        "stderr must name the failure event (the catch event that raised); stderr was:\n{stderr}"
    );
    // The original `error(...)` reason alone must NOT be the only thing on
    // stderr — the catch-event raise replaces it as the surfaced error.
    assert!(
        stderr.contains("crashed expression"),
        "stderr must distinguish the crashed guard from a clean false; stderr was:\n{stderr}"
    );
    // The provider was never launched: the composed body never reached stdout.
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("BODY_MARKER_QQQ"),
        "the body must not be sent after an initialize error + failure raise; stdout was:\n{stdout}"
    );
}

/// Decision #2 ordering proof (process-level): a terminal-phase `success.when`
/// that *raises* must surface its styled `lifecycle evaluation error` to stderr
/// **at the point of error — before the catch `finalize` event fires** — and
/// exactly **once**. The provider runs and exits 0 (so `success` fires), the
/// first `success` guard references an undefined root (a crashed expression),
/// and `finalize.stderr` writes a recognizable marker. The assertion is a byte
/// offset ordering: the evaluation-error text must appear earlier in captured
/// stderr than the `finalize` marker, proving the original crash is visible
/// before any `finalize` output. Also asserts a non-zero exit and a single
/// emission (count == 1), proving the outer renderer does not double-emit.
#[cfg(unix)]
#[test]
fn compose_success_when_evaluation_error_surfaces_before_finalize_marker() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    seed_minimal_config(workspace.path());

    let md_file = workspace.path().join("prompt.md");
    // `success` first guard raises (undefined root under DM2 strict mode);
    // `finalize.stderr` emits a marker the catch event prints to stderr.
    fs::write(
        &md_file,
        "---\nname: late-bind-success\nagent: goose\nsuccess:\n  stack:\n    \
         - when: \"missing_root == true\"\n      action: {stderr: \"unreachable\"}\n\
         finalize:\n  stderr: \"FINALIZE_MARKER_ZZZ\"\n---\nBODY_MARKER_QQQ\n",
    )
    .unwrap();

    // The provider runs and exits 0, so the terminal `success` event fires and
    // its first `when:` guard raises.
    write_executable(&path_dir.join("goose"), "#!/bin/sh\nexit 0\n");

    let output = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .args(["compose", "--goose", md_file.to_str().unwrap()])
        .output()
        .unwrap();

    assert!(
        !output.status.success(),
        "a terminal-phase evaluation error must exit non-zero; stderr was:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr));

    // The styled evaluation-error block names the event and the surface.
    let eval_offset = stderr.find("evaluation error").unwrap_or_else(|| {
        panic!("stderr must carry the lifecycle evaluation error; stderr was:\n{stderr}")
    });
    let marker_offset = stderr.find("FINALIZE_MARKER_ZZZ").unwrap_or_else(|| {
        panic!("the finalize catch event must run and write its marker; stderr was:\n{stderr}")
    });

    // Decision #2: the crash is emitted at the point of error, ahead of any
    // `finalize` output — earlier byte offset proves the ordering.
    assert!(
        eval_offset < marker_offset,
        "the lifecycle evaluation error must be emitted BEFORE the finalize marker \
         (eval@{eval_offset}, finalize@{marker_offset}); stderr was:\n{stderr}"
    );

    // Exactly one styled emission — the early emit suppresses the outer
    // renderer's duplicate.
    let header_count = stderr.matches("lifecycle evaluation error").count();
    assert_eq!(
        header_count, 1,
        "the lifecycle evaluation error must be emitted exactly once; got {header_count}; \
         stderr was:\n{stderr}"
    );
}

/// Error surface (compose): a missing source file under `--dry-run` renders
/// the error to **stderr**, exits **non-zero**, and leaves stdout clean.
///
/// In non-TTY sessions the ENTER-path autocomplete fallback is unavailable,
/// so the error surfaces as `AutocompleteNotInteractive` rather than the
/// original `FileNotFound`.
#[cfg(unix)]
#[test]
fn compose_dry_run_missing_file_errors_to_stderr_with_clean_stdout() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    seed_minimal_config(workspace.path());

    write_executable(&path_dir.join("goose"), "#!/bin/sh\nexit 0\n");

    let output = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .args(["compose", "--goose", "--dry-run", "does-not-exist.md"])
        .output()
        .unwrap();

    assert!(
        !output.status.success(),
        "missing-file dry-run must exit non-zero"
    );
    assert!(
        output.stdout.is_empty(),
        "stdout must stay clean on error; stdout was:\n{}",
        String::from_utf8_lossy(&output.stdout)
    );
    let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr));
    assert!(
        stderr.contains("autocomplete not available")
            || stderr.contains("Autocomplete requires"),
        "non-TTY missing file must report autocomplete unavailable; stderr was:\n{stderr}"
    );
}
