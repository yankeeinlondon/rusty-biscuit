//! Integration tests: compose argument validation, retired-flag rejection, and dry-run stdout/stderr separation.
//!
//! Split out of the `wrap_commands.rs` god file; shared fixtures live in
//! `common::wrap`.

use predicates::str::contains;
use std::fs;
use crate::common;
use common::{CliProcessFixture, strip_ansi, write_dry_run_provider_stub};
#[cfg(unix)]
use common::write_executable;

fn assert_retired_wrapper_flag(flag: &str, replacement: &str) {
    let fixture = CliProcessFixture::named("wrap-retired-flag");
    fixture.seed_user_config();
    write_dry_run_provider_stub(fixture.bin_dir(), "claude");

    fixture
        .command()
        .args(["claude", flag, "file.md"])
        .assert()
        .failure()
        .stdout(predicates::str::is_empty())
        .stderr(contains(format!("{flag} has been retired")))
        .stderr(contains(replacement));
}

#[test]
fn compose_requires_positional_arg() {
    let fixture = CliProcessFixture::named("compose-requires-positional");
    let assert = fixture
        .command()
        .args(["compose"])
        .assert()
        .code(2);

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let plain = strip_ansi(&stderr);
    assert!(plain.contains("ARG"), "usage should show ARG positional");
}

#[test]
fn compose_missing_file_with_setter_only() {
    let fixture = CliProcessFixture::named("compose-setter-only");
    let assert = fixture
        .command()
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
    let fixture = CliProcessFixture::named("compose-empty-key-setter");
    let assert = fixture
        .command()
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
    let fixture = CliProcessFixture::named("compose-multiple-candidates");
    let a = fixture.cwd().join("a.md");
    let b = fixture.cwd().join("b.md");
    fs::write(&a, "---\n---\nbody\n").unwrap();
    fs::write(&b, "---\n---\nbody\n").unwrap();

    let assert = fixture
        .command()
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
    let fixture = CliProcessFixture::named("compose-nonexistent-file");
    fixture
        .command()
        .args(["compose", "/nonexistent/path/to/file.md"])
        .assert()
        .code(1);
}

#[test]
fn compose_rejects_non_markdown_file() {
    let fixture = CliProcessFixture::named("compose-non-markdown-file");
    let txt_file = fixture.cwd().join("file.txt");
    fs::write(&txt_file, "hello").unwrap();

    fixture
        .command()
        .args(["compose", txt_file.to_str().unwrap()])
        .assert()
        .code(1);
}

#[cfg(unix)]
#[test]
fn compose_missing_explicit_system_prompt_fails_visibly() {
    let fixture = CliProcessFixture::named("compose-missing-system-prompt");
    let md_file = fixture.cwd().join("prompt.md");
    let missing_prompt = fixture.cwd().join("missing-system-prompt.md");
    fixture.seed_user_config();
    fs::write(&md_file, "---\ntitle: test\n---\nHello compose\n").unwrap();

    write_executable(&fixture.bin_dir().join("codex"), "#!/bin/sh\nexit 0\n");

    fixture
        .command()
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
    let fixture = CliProcessFixture::named("compose-no-cross-provider-retry");
    let codex_marker = fixture.cwd().join("codex-launched");
    let claude_marker = fixture.cwd().join("claude-launched");
    fixture.seed_user_config();

    let md_file = fixture.cwd().join("test.md");
    fs::write(&md_file, "---\ntitle: test\n---\nPrompt body\n").unwrap();

    // Provider that exits with error code 42
    write_executable(
        &fixture.bin_dir().join("codex"),
        r#"#!/bin/sh
: > "$CODEX_MARKER"
exit 42
"#,
    );

    // Also install a "claude" that succeeds -- if retry happened, we'd see code 0
    write_executable(
        &fixture.bin_dir().join("claude"),
        r#"#!/bin/sh
: > "$CLAUDE_MARKER"
exit 0
"#,
    );

    // Explicitly select codex. It exits 42. No fallback to claude.
    fixture
        .command()
        .env("CODEX_MARKER", &codex_marker)
        .env("CLAUDE_MARKER", &claude_marker)
        .args(["compose", "--codex", md_file.to_str().unwrap()])
        .assert()
        .code(42);

    assert!(codex_marker.exists(), "the selected Codex shim must launch");
    assert!(
        !claude_marker.exists(),
        "Claudine must not launch a fallback Claude provider"
    );
}

#[test]
fn old_compose_inline_command_is_unknown() {
    // Verify that the old `compose-inline` command no longer exists
    let fixture = CliProcessFixture::named("compose-inline-retired-subcommand");
    fixture
        .command()
        .args(["compose-inline", "file.md"])
        .assert()
        .code(2); // clap returns 2 for unrecognized subcommands
}

#[test]
fn retired_compose_flag_rejected_in_wrapper() {
    assert_retired_wrapper_flag("--compose", "claudine compose --<provider> <file>");
}

#[test]
fn retired_frontmatter_prompt_flag_rejected_in_wrapper() {
    assert_retired_wrapper_flag(
        "--frontmatter-prompt",
        "claudine inline-compose --<provider> <file>",
    );
}

#[test]
fn retired_prompt_file_flag_rejected_in_wrapper() {
    assert_retired_wrapper_flag(
        "--prompt-file",
        "the provider CLI directly (claudine compose has different semantics)",
    );
}

/// Data/status discipline: under `compose --dry-run` the composed body is the
/// *only* thing on **stdout**; the finalized frontmatter and the metadata
/// table land on **stderr**. Verifies the two streams never cross.
#[cfg(unix)]
#[test]
fn compose_dry_run_body_only_on_stdout_metadata_on_stderr() {
    let fixture = CliProcessFixture::named("compose-dry-run-stream-discipline");
    fixture.seed_user_config();

    let md_file = fixture.cwd().join("doc.md");
    fs::write(
        &md_file,
        "---\nname: disc-doc\ndescription: a discipline doc\nagent: goose\n---\nBODY_MARKER_QQQ\n",
    )
    .unwrap();

    write_executable(&fixture.bin_dir().join("goose"), "#!/bin/sh\nexit 0\n");

    let output = fixture
        .command()
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
        let fixture = CliProcessFixture::named("compose-dry-run-quiet-silent");
        fixture.seed_user_config();

        let md_file = fixture.cwd().join("doc.md");
        fs::write(
            &md_file,
            "---\nname: qs-doc\nagent: goose\n---\nBODY_MARKER_QQQ\n",
        )
        .unwrap();

        write_executable(&fixture.bin_dir().join("goose"), "#!/bin/sh\nexit 0\n");

        let output = fixture
            .command()
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
    let fixture = CliProcessFixture::named("compose-initialize-when-raise");
    fixture.seed_user_config();

    let md_file = fixture.cwd().join("prompt.md");
    fs::write(
        &md_file,
        "---\nname: late-bind\nagent: goose\ninitialize:\n  stack:\n    \
         - when: \"missing_root == true\"\n      action: {stderr: \"ready\"}\n---\nBODY_MARKER_QQQ\n",
    )
    .unwrap();

    // A stub provider must exist on PATH for preflight, but the initialize
    // raise halts the run before it is ever launched.
    write_executable(&fixture.bin_dir().join("goose"), "#!/bin/sh\nexit 0\n");

    let output = fixture
        .command()
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
    let fixture = CliProcessFixture::named("compose-initialize-error-failure-raise");
    fixture.seed_user_config();

    let md_file = fixture.cwd().join("prompt.md");
    fs::write(
        &md_file,
        "---\nname: explicit-error\nagent: goose\ninitialize:\n  stack:\n    \
         - action: {error: \"preflight refused\"}\nfailure:\n  stderr: \"fail\"\n  stack:\n    \
         - when: \"missing_root == true\"\n      action: {stderr: \"unreachable\"}\n---\nBODY_MARKER_QQQ\n",
    )
    .unwrap();

    // A stub provider must exist on PATH for preflight, but the initialize
    // error + failure raise halts the run before it is ever launched.
    write_executable(&fixture.bin_dir().join("goose"), "#!/bin/sh\nexit 0\n");

    let output = fixture
        .command()
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
    let fixture = CliProcessFixture::named("compose-success-when-raise");
    fixture.seed_user_config();

    let md_file = fixture.cwd().join("prompt.md");
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
    write_executable(&fixture.bin_dir().join("goose"), "#!/bin/sh\nexit 0\n");

    let output = fixture
        .command()
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
    let fixture = CliProcessFixture::named("compose-dry-run-missing-file");
    fixture.seed_user_config();

    write_executable(&fixture.bin_dir().join("goose"), "#!/bin/sh\nexit 0\n");

    let output = fixture
        .command()
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

/// The nested-span regression fixtures are byte-identical copies of
/// `prompts/_reviews/review-spec-inline.md` and `prompts/commit.md` at
/// `cd6e036c4^`, before the prompt-only repair. Later tests prove validation
/// rejects exactly these defects, so a "repaired" fixture would make them
/// vacuous; this pins the defect bytes that must survive.
#[test]
fn nested_span_regression_fixtures_preserve_pre_fix_defects() {
    let review = include_str!("../fixtures/nested_span_regression/review-spec-inline.md");
    for defect in [
        r#"? "The review of the draft specification file in {{ctx.area}} has completed""#,
        r#": "The review of the draft specification file in the {{ctx.repo_name}} repo has completed""#,
        r#"? "The inline review of the draft specification {{ title_case(without_date(parent_dir(spec))) }} in the {{ctx.area}} package area failed to complete!""#,
        r#": "The inline review {{ title_case(without_date(parent_dir(spec))) }} in the {{ctx.repo_name}} repo failed to complete!""#,
    ] {
        assert!(review.contains(defect), "review fixture lost: {defect}");
    }
    assert_eq!(review.matches("say: |-").count(), 2);

    let commit = include_str!("../fixtures/nested_span_regression/commit.md");
    for defect in [
        "? 'are all part of the {{ctx.dirty_package_areas}} package area'",
        r": 'are spread across {{length(ctx.dirty_package_areas)}}:\n {{as_unordered_list(ctx.dirty_package_areas)}}'",
    ] {
        assert!(commit.contains(defect), "commit fixture lost: {defect}");
    }
    assert!(commit.contains("resides_in: |-"));
}

// -- nested spans in single-pass lifecycle literals (spec D2/D4) ------------

const INCIDENT: &str = include_str!("../fixtures/nested_span_regression/review-spec-inline.md");

/// A lifecycle whose `success.say` nests a span inside a quoted literal.
const DEFECT_LIFECYCLE: &str =
    "success:\n    say: \"{{ ok ? 'done in {{area}}' : 'failed' }}\"\n";

/// Install a `goose` stub that appends one line to `$PROVIDER_MARKER` per run.
fn install_marker_provider(fixture: &CliProcessFixture) -> std::path::PathBuf {
    #[cfg(unix)]
    write_executable(
        &fixture.bin_dir().join("goose"),
        "#!/bin/sh\necho run >> \"$PROVIDER_MARKER\"\necho done\nexit 0\n",
    );
    // Compiled rather than a `.cmd`: Rust refuses to spawn a batch file whose
    // arguments contain newlines, and Claudine delivers the composed prompt as
    // one `-t` argument, so a shim stub fails the launch with "batch file
    // arguments are invalid" before the test's own assertion is reached
    // (measured on `build-win-native`, 2026-09-22). Mirrors
    // `common/review_router.rs` and `compose_caller_file_provenance.rs`.
    #[cfg(windows)]
    {
        let source = fixture.bin_dir().join("goose-marker.rs");
        fs::write(
            &source,
            r##"fn main() {
    let marker = std::env::var("PROVIDER_MARKER").expect("PROVIDER_MARKER");
    let mut log = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(marker)
        .expect("open provider marker");
    std::io::Write::write_all(&mut log, b"run\n").expect("record the run");
    println!("done");
}
"##,
        )
        .unwrap();
        let output = std::process::Command::new("rustc")
            .arg("--edition=2024")
            .arg(&source)
            .arg("-o")
            .arg(fixture.bin_dir().join("goose.exe"))
            .output()
            .expect("rustc must build the Windows provider fixture");
        assert!(
            output.status.success(),
            "provider fixture compilation failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    fixture.cwd().join("provider-runs.log")
}

fn provider_runs(marker: &std::path::Path) -> usize {
    fs::read_to_string(marker).map_or(0, |log| log.lines().count())
}

/// Run `claudine` with `args` and the marker env, returning `(success, stderr)`
/// with block gutters removed and wrapped lines rejoined by single spaces.
fn run_with_marker(
    fixture: &CliProcessFixture,
    marker: &std::path::Path,
    args: &[&str],
) -> (bool, String) {
    let output = fixture
        .command()
        .env("PROVIDER_MARKER", marker)
        .args(args)
        .output()
        .unwrap();
    let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr))
        .replace('┃', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    assert!(
        fixture.audio_spool().read_dir().map_or(true, |mut d| d.next().is_none()),
        "no lifecycle audio may be queued"
    );
    (output.status.success(), stderr)
}

fn assert_nested_span_rejection(stderr: &str, property: &str) {
    assert!(
        stderr.contains("nested interpolation inside a string literal"),
        "stderr must carry the nested-span error; stderr was:\n{stderr}"
    );
    assert!(stderr.contains(property), "stderr must name `{property}`:\n{stderr}");
}

fn write_incident(fixture: &CliProcessFixture) -> std::path::PathBuf {
    let incident = fixture.cwd().join("review-spec-inline.md");
    fs::write(&incident, INCIDENT).unwrap();
    fs::write(fixture.cwd().join("draft-spec.md"), "# Draft\n").unwrap();
    incident
}

/// Acceptance 1: the pre-fix incident is refused at prepare time with the
/// property, the literal, and the `+` rewrite, and no provider starts — for a
/// real run and for `--dry-run` alike.
#[test]
fn compose_rejects_the_pre_fix_incident_before_any_provider_starts() {
    let fixture = CliProcessFixture::named("nested-span-compose");
    fixture.seed_user_config();
    let marker = install_marker_provider(&fixture);
    let incident = write_incident(&fixture);
    let incident = incident.to_str().unwrap();

    for dry_run in [false, true] {
        let mut args = vec!["compose", "--goose"];
        if dry_run {
            args.push("--dry-run");
        }
        args.extend([incident, "spec=draft-spec.md"]);
        let (success, stderr) = run_with_marker(&fixture, &marker, &args);
        assert!(!success, "dry_run={dry_run}: the incident must be refused:\n{stderr}");
        assert_nested_span_rejection(&stderr, "success.say");
        assert!(
            stderr.contains("The review of the draft specification file in {{ctx.area}} has completed"),
            "dry_run={dry_run}: stderr must quote the literal:\n{stderr}"
        );
        assert!(
            stderr.contains(r#"" + ctx.area + ""#),
            "dry_run={dry_run}: stderr must print the `+` rewrite:\n{stderr}"
        );
        assert!(!stderr.contains("resolve the missing"), "{stderr}");
        assert_eq!(provider_runs(&marker), 0, "dry_run={dry_run}: no provider may start");
    }
}

#[test]
fn inline_compose_rejects_a_nested_span_before_any_provider_starts() {
    let fixture = CliProcessFixture::named("nested-span-inline");
    fixture.seed_user_config();
    let marker = install_marker_provider(&fixture);
    let doc = fixture.cwd().join("inline.md");
    let original = format!("---\nprompt: Write the summary.\n{DEFECT_LIFECYCLE}---\nBody\n");
    fs::write(&doc, &original).unwrap();

    let (success, stderr) =
        run_with_marker(&fixture, &marker, &["inline-compose", "--goose", doc.to_str().unwrap()]);
    assert!(!success, "{stderr}");
    assert_nested_span_rejection(&stderr, "success.say");
    assert_eq!(provider_runs(&marker), 0);
    assert_eq!(fs::read_to_string(&doc).unwrap(), original, "the document is untouched");
}

/// Acceptance 2: a `proxy` handoff prepares its target before launch, so the
/// incident is refused when the entry document proxies to it.
#[test]
fn proxy_to_the_incident_is_rejected_before_any_provider_starts() {
    let fixture = CliProcessFixture::named("nested-span-proxy");
    fixture.seed_user_config();
    let marker = install_marker_provider(&fixture);
    write_incident(&fixture);
    let entry = fixture.cwd().join("entry.md");
    fs::write(
        &entry,
        "---\ninitialize:\n    stack:\n        - action:\n              proxy: ./review-spec-inline.md\n---\nEntry body\n",
    )
    .unwrap();

    let (success, stderr) = run_with_marker(
        &fixture,
        &marker,
        &["compose", "--goose", entry.to_str().unwrap(), "spec=draft-spec.md"],
    );
    assert!(!success, "{stderr}");
    assert_nested_span_rejection(&stderr, "success.say");
    assert_eq!(provider_runs(&marker), 0);
}

/// Acceptance 2: a statically referenced step document is pre-scanned, so
/// neither step one nor step two ever starts.
#[test]
fn sequence_referencing_the_incident_in_step_two_starts_no_step() {
    let fixture = CliProcessFixture::named("nested-span-sequence-ref");
    fixture.seed_user_config();
    let marker = install_marker_provider(&fixture);
    fs::write(fixture.cwd().join("first.md"), "---\nagent: goose\n---\nFirst step\n").unwrap();
    fs::write(
        fixture.cwd().join("second.md"),
        format!("---\nagent: goose\n{DEFECT_LIFECYCLE}---\nSecond step\n"),
    )
    .unwrap();
    let sequence = fixture.cwd().join("sequence.md");
    fs::write(
        &sequence,
        "---\nagent: goose\nsequence:\n    - name: first\n      prompt: ./first.md\n    - name: second\n      prompt: second.md\n---\nSequence body\n",
    )
    .unwrap();

    for dry_run in [false, true] {
        let mut args = vec!["sequence", "--goose"];
        if dry_run {
            args.push("--dry-run");
        }
        args.push(sequence.to_str().unwrap());
        let (success, stderr) = run_with_marker(&fixture, &marker, &args);
        assert!(!success, "dry_run={dry_run}: {stderr}");
        assert_nested_span_rejection(&stderr, "success.say");
        assert!(stderr.contains("second.md"), "the referenced document is named:\n{stderr}");
        assert_eq!(provider_runs(&marker), 0, "dry_run={dry_run}: no step may start");
    }
}

/// Phase 1c prepares every step of the sequence document through the shared
/// validator, so a defect in the sequence's own lifecycle starts no step.
#[test]
fn sequence_document_with_a_nested_span_starts_no_step() {
    let fixture = CliProcessFixture::named("nested-span-sequence-self");
    fixture.seed_user_config();
    let marker = install_marker_provider(&fixture);
    let sequence = fixture.cwd().join("sequence.md");
    fs::write(
        &sequence,
        format!(
            "---\nagent: goose\n{DEFECT_LIFECYCLE}sequence:\n    - one\n    - two\n---\nStep {{{{ state }}}}\n"
        ),
    )
    .unwrap();

    let (success, stderr) =
        run_with_marker(&fixture, &marker, &["sequence", "--goose", sequence.to_str().unwrap()]);
    assert!(!success, "{stderr}");
    assert_nested_span_rejection(&stderr, "success.say");
    assert_eq!(provider_runs(&marker), 0);
}

/// Predicates, action operands, and `proxy … with` values are single-pass
/// surfaces too.
#[test]
fn predicates_operands_and_proxy_with_values_are_rejected_before_launch() {
    let fixture = CliProcessFixture::named("nested-span-surfaces");
    fixture.seed_user_config();
    let marker = install_marker_provider(&fixture);
    let cases = [
        (
            "start:\n    stack:\n        - when: \"label == 'in {{area}}'\"\n          action: stop\n",
            "start.stack[0].when",
        ),
        (
            "start:\n    stack:\n        - action:\n              info: \"{{ ok ? 'in {{area}}' : 'x' }}\"\n",
            "start.stack[0].action[0].message",
        ),
        (
            "failure:\n    stack:\n        - action:\n              action: proxy\n              target: ./next.md\n              with:\n                  note: \"{{ 'in {{area}}' }}\"\n",
            "failure.stack[0].action[0].with.note",
        ),
        (
            "loop:\n    while: \"status != 'at {{step}}'\"\n    max: 2\n",
            "loop.while",
        ),
    ];
    for (index, (lifecycle, property)) in cases.into_iter().enumerate() {
        let doc = fixture.cwd().join(format!("surface-{index}.md"));
        fs::write(&doc, format!("---\n{lifecycle}---\nBody\n")).unwrap();
        let (success, stderr) =
            run_with_marker(&fixture, &marker, &["compose", "--goose", doc.to_str().unwrap()]);
        assert!(!success, "{property}: {stderr}");
        assert_nested_span_rejection(&stderr, property);
        assert_eq!(provider_runs(&marker), 0, "{property}: no provider may start");
    }
}

#[test]
fn colliding_proxy_overlay_paths_are_rejected_before_launch() {
    let fixture = CliProcessFixture::named("nested-span-proxy-path-collision");
    fixture.seed_user_config();
    let marker = install_marker_provider(&fixture);
    let doc = fixture.cwd().join("collision.md");
    fs::write(
        &doc,
        "---\nfailure:\n    stack:\n        - action:\n              action: proxy\n              target: ./next.md\n              with:\n                  a:\n                      b: \"{{ ok ? 'bad {{ x }}' : 'fine' }}\"\n                  \"a.b\": fine\n---\nBody\n",
    )
    .unwrap();

    let (success, stderr) =
        run_with_marker(&fixture, &marker, &["compose", "--goose", doc.to_str().unwrap()]);
    assert!(!success, "the colliding nested defect must be refused: {stderr}");
    assert_nested_span_rejection(&stderr, "failure.stack[0].action[0].with.a.b");
    assert_eq!(provider_runs(&marker), 0, "no provider may start");
}

/// Acceptance 4: synthesized action bodies, mixed strings, ordinary
/// whole-value frontmatter, and the valid `+` form still launch the provider.
#[test]
fn synthesized_mixed_and_ordinary_frontmatter_values_still_launch() {
    let fixture = CliProcessFixture::named("nested-span-negative-controls");
    fixture.seed_user_config();
    let marker = install_marker_provider(&fixture);
    let doc = fixture.cwd().join("controls.md");
    fs::write(
        &doc,
        "---\narea: claudine\nresides_in: \"{{ area ? 'in {{area}}' : 'nowhere' }}\"\nstart:\n    info: \"starting in {{area}}\"\n    stack:\n        - action:\n              - info: \"running {{area}}\"\n              - action: info\n                message: \"Deployed {{area}}\"\nsuccess:\n    info: \"a {{ area ? 'in {{area}}' : 'x' }} b\"\n    stdout: \"{{ 'done in ' + area }}\"\n---\nBody {{resides_in}}\n",
    )
    .unwrap();

    let (success, stderr) =
        run_with_marker(&fixture, &marker, &["compose", "--goose", doc.to_str().unwrap()]);
    assert!(success, "valid forms must run:\n{stderr}");
    assert_eq!(provider_runs(&marker), 1, "the provider launches exactly once");
    assert!(!stderr.contains("nested interpolation"), "{stderr}");
}

/// D4: a frontmatter value that holds template text reaches the event-time
/// guard, which names the lifecycle key and the typed reason and selects the
/// concatenate/`{{{ … }}}` hint rather than the missing-path one.
#[test]
fn surviving_span_at_event_time_names_the_property_and_specific_hint() {
    let fixture = CliProcessFixture::named("nested-span-runtime-backstop");
    fixture.seed_user_config();
    let marker = install_marker_provider(&fixture);
    let doc = fixture.cwd().join("backstop.md");
    fs::write(
        &doc,
        "---\ntmpl: \"{{{ctx.repo_name}}}\"\nstart:\n    info: \"{{ tmpl }}\"\n---\nBody\n",
    )
    .unwrap();

    let (success, stderr) =
        run_with_marker(&fixture, &marker, &["compose", "--goose", doc.to_str().unwrap()]);
    assert!(!success, "{stderr}");
    assert!(stderr.contains("lifecycle evaluation error"), "{stderr}");
    assert!(stderr.contains("start.info"), "the property is named:\n{stderr}");
    assert!(
        stderr.contains("still contains `{{ctx.repo_name}}` after every interpolation pass"),
        "the typed reason is rendered:\n{stderr}"
    );
    assert!(stderr.contains("concatenate with `+`"), "{stderr}");
    assert!(!stderr.contains("resolve the missing"), "the old hint must not appear:\n{stderr}");
    assert_eq!(provider_runs(&marker), 0);
}

/// `retry` and `resume` re-read the document and run canonical preparation
/// again, so a defect written by the first provider run is refused before a
/// second provider start.
#[cfg(unix)]
#[test]
fn reentry_refuses_a_defect_introduced_by_the_previous_attempt() {
    let fixture = CliProcessFixture::named("nested-span-reentry");
    fixture.seed_user_config();
    let marker = fixture.cwd().join("provider-runs.log");
    // `resume` needs a captured session, so its provider is a Claude stub that
    // reports one on the stream before failing.
    let claude_init = "echo '{\"type\":\"system\",\"subtype\":\"init\",\"session_id\":\"s-1\",\"model\":\"m\",\"tools\":[]}'\n";
    let cases = [
        ("retry", "goose", "", "failure:\n    stack:\n        - action:\n              retry: 2\n", 1),
        ("resume", "claude", claude_init, "failure:\n    stack:\n        - action:\n              resume: continue\n", 1),
    ];
    for (label, provider, stream, recovery, exit_code) in cases {
        let _ = fs::remove_file(&marker);
        let doc = fixture.cwd().join(format!("{label}.md"));
        let defect_doc = fixture.cwd().join(format!("{label}-defect.md"));
        fs::write(&doc, format!("---\n{recovery}---\nBody\n")).unwrap();
        fs::write(&defect_doc, format!("---\n{recovery}{DEFECT_LIFECYCLE}---\nBody\n")).unwrap();
        // The first run swaps in the defective lifecycle, then fails (or, for
        // the loop, succeeds) so the recovery path re-enters preparation.
        write_executable(
            &fixture.bin_dir().join(provider),
            &format!(
                "#!/bin/sh\necho run >> \"$PROVIDER_MARKER\"\ncp '{}' '{}'\n{stream}echo done\nexit {exit_code}\n",
                defect_doc.display(),
                doc.display()
            ),
        );

        let flag = format!("--{provider}");
        let (success, stderr) =
            run_with_marker(&fixture, &marker, &["compose", &flag, doc.to_str().unwrap()]);
        assert!(!success, "{label}: {stderr}");
        assert_nested_span_rejection(&stderr, "success.say");
        assert_eq!(
            provider_runs(&marker),
            1,
            "{label}: the re-entry must be refused before a second provider start:\n{stderr}"
        );
    }
}

/// A document loop parses its lifecycle once, in the seed preparation, and
/// every iteration reuses that stamped config; the seed preparation is
/// therefore where a loop document's defect is refused, before iteration 1.
#[test]
fn loop_document_with_a_nested_span_starts_no_iteration() {
    let fixture = CliProcessFixture::named("nested-span-loop");
    fixture.seed_user_config();
    let marker = install_marker_provider(&fixture);
    let doc = fixture.cwd().join("loop.md");
    fs::write(
        &doc,
        format!("---\nloop:\n    while: \"true\"\n    max: 3\n{DEFECT_LIFECYCLE}---\nBody\n"),
    )
    .unwrap();

    let (success, stderr) =
        run_with_marker(&fixture, &marker, &["compose", "--goose", doc.to_str().unwrap()]);
    assert!(!success, "{stderr}");
    assert_nested_span_rejection(&stderr, "success.say");
    assert_eq!(provider_runs(&marker), 0);
}
