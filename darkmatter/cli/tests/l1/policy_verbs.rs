//! The policy verbs `--allow`, `--deny`, and `--disable` end to end: each
//! registry mapping, usage errors (unknown, empty, unsupported, opposite
//! selections, baseline conflicts, host denials) before any effect,
//! precedence between `all` and a specific identifier, blocking and
//! stopping, and no document on failure.
//!
//! Effects are observed with the portable lifecycle `ensure_file` action, so
//! every row runs on every OS.

use crate::common;

use common::{CliProcessFixture, MockHttpResponse, mock_http_server, unwrapped};
use predicates::prelude::*;

/// A document whose only problem is a missing hyperlink target.
const MISSING_LINK: &str = "# Title\n\nSee [the guide](./missing-guide.md).\n";

/// A scalar `ctx`, which only `ctx-override` recovers, and nothing else. The
/// baseline types `ctx` as an object, so these runs pass
/// `--disable baseline-schema` to reach the recovery.
const SCALAR_CTX: &str = "---\nctx: 5\n---\nBody\n";

/// A category 3 call in the body.
const CATEGORY_3: &str = "---\n$schema:\n  label: string\nlabel: \"7\"\n---\nBody {{ min(label, 5) }}\n";

/// `initialize` writes `marker.md` before the run composes anything.
const MARKED: &str = "---\ninitialize:\n  - ensure_file: [\"marker.md\", \"x\"]\n---\nBody\n";

fn stderr_of(output: &std::process::Output) -> String {
    unwrapped(&String::from_utf8_lossy(&output.stderr))
}

// ── Usage errors ───────────────────────────────────────────────────────

/// Asserts `md <args> <document>` is a usage error that wrote nothing,
/// named `needle`, and ran no `initialize`.
fn assert_usage_error(fixture: &CliProcessFixture, args: &[&str], needle: &str) {
    let document = fixture.write_file("cwd/doc.md", MARKED);
    let output = fixture.command().args(args).arg(&document).output().unwrap();
    let stderr = stderr_of(&output);
    assert_eq!(output.status.code(), Some(2), "{args:?}: {stderr}");
    assert!(output.stdout.is_empty(), "{args:?}: {}", String::from_utf8_lossy(&output.stdout));
    assert!(stderr.contains(needle), "{args:?}: expected `{needle}` in {stderr}");
    assert!(!fixture.cwd().join("marker.md").exists(), "{args:?} ran `initialize`");
}

#[test]
fn opposite_selections_are_usage_errors_in_either_order_before_any_effect() {
    let fixture = CliProcessFixture::named("policy_opposites");
    for args in [
        &["compose", "--allow", "ctx-override", "--deny", "ctx-override"][..],
        &["compose", "--deny", "ctx-override", "--allow", "ctx-override"],
        &["compose", "--allow", "all", "--deny", "all"],
        &["compose", "--deny", "all", "--allow", "all"],
    ] {
        assert_usage_error(&fixture, args, "keep one");
    }
}

#[test]
fn a_recovery_denied_by_its_code_conflicts_with_its_allowance() {
    let fixture = CliProcessFixture::named("policy_code_spelling");
    for args in [
        &["compose", "--allow", "missing-hyperlinks", "--deny", "dm.reference.missing_hyperlink"][..],
        &["compose", "--deny", "dm.shell.timeout_recovered", "--allow", "shell-timeout"],
    ] {
        assert_usage_error(&fixture, args, "select the same recovery");
    }
}

#[test]
fn unknown_and_empty_identifiers_are_usage_errors() {
    let fixture = CliProcessFixture::named("policy_unknown");
    assert_usage_error(&fixture, &["compose", "--allow", "missing-everything"], "unknown identifier `missing-everything`");
    assert_usage_error(&fixture, &["compose", "--deny", "dm.nothing.here"], "unknown identifier `dm.nothing.here`");
    assert_usage_error(&fixture, &["compose", "--allow", ""], "needs an identifier");
    assert_usage_error(&fixture, &["clean", "--disable", "colors"], "unknown identifier `colors`");
}

#[test]
fn identifiers_a_verb_or_command_does_not_take_are_usage_errors() {
    let fixture = CliProcessFixture::named("policy_unsupported");
    assert_usage_error(&fixture, &["compose", "--allow", "dm.expression.uncertain_call"], "never a diagnostic code");
    assert_usage_error(&fixture, &["compose", "--disable", "all"], "takes no `all`");
    assert_usage_error(&fixture, &["compose", "--deny", "baseline-schema"], "use `--disable baseline-schema`");
    assert_usage_error(&fixture, &["compose", "--allow", "style"], "can only be denied");
    assert_usage_error(&fixture, &["render", "--deny", "shell-timeout"], "does not apply to `md render`");
    assert_usage_error(&fixture, &["schema", "validate", "--disable", "style"], "does not apply to `md schema validate`");
}

#[test]
fn a_host_is_an_allowance_only() {
    let fixture = CliProcessFixture::named("policy_host_denial");
    assert_usage_error(&fixture, &["compose", "--deny", "host=example.com"], "allowance only");
    assert_usage_error(&fixture, &["compose", "--disable", "host=example.com"], "use `--allow host=HOST`");
    assert_usage_error(&fixture, &["compose", "--allow", "host="], "needs a host");
    assert_usage_error(&fixture, &["compose", "--allow", "host"], "`host=HOST`");
}

#[test]
fn verbs_a_command_does_not_declare_are_usage_errors() {
    let fixture = CliProcessFixture::named("policy_undeclared_verb");
    assert_usage_error(&fixture, &["clean", "--deny", "all"], "--deny");
    assert_usage_error(&fixture, &["schema", "validate", "--allow", "all"], "--allow");
    // A top-level option beside a subcommand fails as every such option does.
    let document = fixture.write_file("cwd/doc.md", MARKED);
    fixture
        .command()
        .args(["--deny", "style", "compose"])
        .arg(&document)
        .assert()
        .failure()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("cannot be combined with top-level options: --deny"));
    assert!(!fixture.cwd().join("marker.md").exists());
}

#[test]
fn disabling_the_baseline_conflicts_with_an_explicit_baseline() {
    let fixture = CliProcessFixture::named("policy_baseline_conflict");
    let baseline = fixture.write_file("cwd/baseline.yaml", "$schema:\n  title: string\n");
    let baseline = baseline.to_str().unwrap();
    assert_usage_error(&fixture, &["compose", "--baseline-schema", baseline, "--disable", "baseline-schema"], "cannot be used with");
    assert_usage_error(&fixture, &["clean", "--disable", "baseline-schema", "--baseline-schema", baseline], "cannot be used with");
    assert_usage_error(&fixture, &["schema", "validate", "--schema", baseline, "--disable", "baseline-schema"], "cannot be used with");
}

#[test]
fn every_removed_flag_is_a_usage_error_naming_its_replacement() {
    let fixture = CliProcessFixture::named("policy_removed_flags");
    for (command, flag, replacement) in [
        (&["compose"][..], "--allow-missing-hyperlinks", "--allow missing-hyperlinks"),
        (&["compose"], "--allow-missing-image-refs", "--allow missing-image-refs"),
        (&["compose"], "--allow-missing-transclusions", "--allow missing-transclusions"),
        (&["compose"], "--allow-any-missing-reference", "--allow missing-hyperlinks --allow missing-image-refs"),
        (&["compose"], "--allow-ctx-override", "--allow ctx-override"),
        (&["compose"], "--allow-invalid-frontmatter-assignment", "--allow invalid-frontmatter-assignment"),
        (&["compose"], "--allow-reassigned-frontmatter-property", "--allow reassigned-frontmatter-property"),
        (&["compose"], "--allow-shell-timeout", "--allow shell-timeout"),
        (&["compose"], "--allow-host=example.com", "--allow host=HOST"),
        (&["compose"], "--no-baseline-schema", "--disable baseline-schema"),
        (&["clean"], "--no-baseline-schema", "--disable baseline-schema"),
        (&["schema", "validate"], "--no-baseline-schema", "--disable baseline-schema"),
        (&["compose"], "--no-schema-triggers", "--disable schema-triggers"),
        (&["clean"], "--no-schema-triggers", "--disable schema-triggers"),
        (&["schema", "validate"], "--no-schema-triggers", "--disable schema-triggers"),
        (&[], "--strict-style", "--deny style"),
        (&["render"], "--strict-style", "--deny style"),
    ] {
        let args: Vec<&str> = command.iter().copied().chain([flag]).collect();
        assert_usage_error(&fixture, &args, &format!("use `{replacement}"));
    }
}

#[test]
fn repeated_identical_selections_are_harmless() {
    let fixture = CliProcessFixture::named("policy_repeated");
    let document = fixture.write_file("cwd/doc.md", SCALAR_CTX);
    fixture
        .command()
        .args(["compose", "--allow", "ctx-override", "--allow", "ctx-override"])
        .args(["--allow", "host=example.com", "--allow", "host=example.com"])
        .args(["--disable", "schema-triggers", "--disable", "schema-triggers", "--disable", "baseline-schema"])
        .arg(&document)
        .assert()
        .success()
        .stdout(predicate::str::contains("Body"));
    fixture
        .command()
        .args(["compose", "--deny", "dm.expression.uncertain_call", "--deny", "dm.expression.uncertain_call"])
        .arg(&document)
        .args(["--allow", "ctx-override", "--disable", "baseline-schema"])
        .assert()
        .success();
}

// ── Recoveries and precedence ──────────────────────────────────────────

#[test]
fn each_missing_reference_allowance_recovers_its_own_kind_only() {
    let fixture = CliProcessFixture::named("policy_missing_references");
    let link = fixture.write_file("cwd/link.md", MISSING_LINK);
    let image = fixture.write_file("cwd/image.md", "# Title\n\n![logo](./missing-logo.png)\n");
    let part = fixture.write_file("cwd/part.md", "# Title\n\n::file ./missing-part.md\n\nAfter\n");

    for (document, allowance, kept) in [
        (&link, "missing-hyperlinks", "[the guide](missing-guide.md)"),
        (&image, "missing-image-refs", "![logo](missing-logo.png)"),
        (&part, "missing-transclusions", "After"),
    ] {
        fixture.command().arg("compose").arg(document).assert().code(2).stdout(predicate::str::is_empty());
        fixture
            .command()
            .args(["compose", "--allow", allowance])
            .arg(document)
            .assert()
            .success()
            .stdout(predicate::str::contains(kept));
    }
    // An allowance covers its kind only.
    fixture.command().args(["compose", "--allow", "missing-image-refs"]).arg(&link).assert().code(2);
    // The transclusion directive is removed, not left in the output.
    fixture
        .command()
        .args(["compose", "--allow", "missing-transclusions"])
        .arg(&part)
        .assert()
        .success()
        .stdout(predicate::str::contains("::file").not());
}

#[test]
fn a_specific_denial_beats_allow_all_in_either_order() {
    let fixture = CliProcessFixture::named("policy_specific_denial");
    let document = fixture.write_file("cwd/doc.md", MISSING_LINK);
    fixture
        .command()
        .args(["compose", "--allow", "all"])
        .arg(&document)
        .assert()
        .success()
        .stdout(predicate::str::contains("the guide"));
    for args in [
        &["compose", "--allow", "all", "--deny", "missing-hyperlinks"][..],
        &["compose", "--deny", "missing-hyperlinks", "--allow", "all"],
        &["compose", "--allow", "all", "--deny", "dm.reference.missing_hyperlink"],
    ] {
        fixture.command().args(args).arg(&document).assert().code(2).stdout(predicate::str::is_empty());
    }
}

#[test]
fn a_specific_recovery_beats_deny_all_in_either_order() {
    let fixture = CliProcessFixture::named("policy_specific_recovery");
    let document = fixture.write_file("cwd/doc.md", SCALAR_CTX);
    fixture
        .command()
        .args(["compose", "--deny", "all", "--disable", "baseline-schema"])
        .arg(&document)
        .assert()
        .failure()
        .stdout(predicate::str::is_empty());
    for args in [
        &["compose", "--disable", "baseline-schema", "--deny", "all", "--allow", "ctx-override"][..],
        &["compose", "--disable", "baseline-schema", "--allow", "ctx-override", "--deny", "all"],
    ] {
        let output = fixture.command().args(args).arg(&document).output().unwrap();
        let stderr = stderr_of(&output);
        assert_eq!(output.status.code(), Some(0), "{args:?}: {stderr}");
        assert_eq!(String::from_utf8_lossy(&output.stdout).trim_end(), "Body", "{args:?}");
        // The recovery is still reported; it just does not block.
        assert!(stderr.contains("Document ctx was not an object"), "{args:?}: {stderr}");
    }
}

#[test]
fn an_independently_denied_diagnostic_still_blocks_a_recovered_command() {
    let fixture = CliProcessFixture::named("policy_independent_denial");
    let document = fixture.write_file("cwd/doc.md", &CATEGORY_3.replace("label: \"7\"\n", "label: \"7\"\nctx: 5\n"));
    fixture.command().args(["compose", "--allow", "ctx-override", "--disable", "baseline-schema"]).arg(&document).assert().success();
    let output = fixture
        .command()
        .args(["compose", "--disable", "baseline-schema", "--allow", "all", "--deny", "dm.expression.uncertain_call"])
        .arg(&document)
        .output()
        .unwrap();
    let stderr = stderr_of(&output);
    assert_eq!(output.status.code(), Some(1), "{stderr}");
    assert!(output.stdout.is_empty());
    assert!(stderr.contains("dm.expression.uncertain_call"), "{stderr}");
}

#[test]
fn allow_all_grants_no_unnamed_host() {
    let fixture = CliProcessFixture::named("policy_allow_all_host");
    let server = mock_http_server(vec![MockHttpResponse { status: 200, body: "Remote body\n", cache_control: None }]);
    let url = server.url("/remote.md");
    fixture
        .command()
        .args(["compose", "-", "--allow", "all"])
        .write_stdin(format!("# Local\n\n::file {url}\n"))
        .assert()
        .failure()
        .stdout(predicate::str::contains("Remote body").not());
    assert_eq!(server.request_count(), 0, "no request without `--allow host=`");
}

#[test]
fn deny_all_keeps_explicit_host_consent() {
    let fixture = CliProcessFixture::named("policy_deny_all_host");
    let server = mock_http_server(vec![MockHttpResponse { status: 200, body: "Remote body\n", cache_control: None }]);
    let url = server.url("/remote.md");
    fixture
        .command()
        .args(["compose", "-", "--deny", "all", "--allow", "host=127.0.0.1"])
        .write_stdin(format!("# Local\n\n::file {url}\n"))
        .assert()
        .success()
        .stdout(predicate::str::contains("Remote body"));
    assert_eq!(server.request_count(), 1);
}

#[test]
fn allow_all_authorizes_no_shell_command() {
    let fixture = CliProcessFixture::named("policy_allow_all_shell");
    let document = fixture.write_file("cwd/doc.md", "# Title\n\n::shell echo never-approved\n");
    fixture
        .command()
        .args(["compose", "--allow", "all"])
        .arg(&document)
        .assert()
        .failure()
        .stdout(predicate::str::contains("never-approved").not());
}

#[test]
fn allow_all_recovers_a_shell_timeout_as_an_empty_string() {
    let fixture = CliProcessFixture::named("policy_allow_all_timeout");
    let document = fixture.write_file("cwd/doc.md", "# Test\n::shell sleep 2\nAfter\n");
    fixture.write_file("cwd/.darkmatter-shell-whitelist", "prefix sleep\n");
    // A real `sleep` shell process is required to exercise timeout recovery.
    let output = fixture
        .command_builder()
        .host_path()
        .build()
        .args(["compose", "--timeout", "1", "--allow", "all"])
        .arg(&document)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr).split_whitespace().collect::<Vec<_>>().join(" ");
    assert_eq!(output.status.code(), Some(0), "{stderr}");
    assert_eq!(stdout.trim_end(), "# Test\nAfter", "the command's output is empty: {stdout:?}");
    assert!(stderr.contains("replaced with an empty string"), "{stderr}");
}

// ── Disabled features ──────────────────────────────────────────────────

#[test]
fn a_disabled_baseline_stays_disabled_under_deny_all() {
    let fixture = CliProcessFixture::named("policy_disabled_baseline");
    // The baseline types `title` as a string.
    let document = fixture.write_file("cwd/doc.md", "---\ntitle: [1, 2]\n---\nBody\n");
    fixture.command().args(["schema", "validate"]).arg(&document).assert().code(1);
    fixture
        .command()
        .args(["compose", "--deny", "all", "--disable", "baseline-schema"])
        .arg(&document)
        .assert()
        .success()
        .stdout(predicate::str::contains("Body"));
    // Skipped checks are not described as passing validation.
    let output = fixture
        .command()
        .args(["schema", "validate", "--disable", "baseline-schema", "--format", "json"])
        .arg(&document)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0), "{}", stderr_of(&output));
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["schema"], serde_json::Value::Null, "{report}");
}

// ── Blocking and stopping ──────────────────────────────────────────────

#[test]
fn deny_all_fails_a_category_3_document_with_no_document_in_any_mode() {
    let fixture = CliProcessFixture::named("policy_category_3");
    let document = fixture.write_file("cwd/doc.md", CATEGORY_3);
    fixture.command().arg("compose").arg(&document).assert().success().stdout("Body 5\n");
    for output in ["markdown", "json", "html"] {
        let result = fixture
            .command_builder()
            // Skips launching a viewer, so a regression cannot open one.
            .application_input("MD_DRY_RUN", "1")
            .build()
            .args(["compose", "--deny", "all", "--show", "--output", output])
            .arg(&document)
            .output()
            .unwrap();
        let stderr = stderr_of(&result);
        assert_eq!(result.status.code(), Some(1), "{output}: {stderr}");
        assert!(result.stdout.is_empty(), "{output}: {}", String::from_utf8_lossy(&result.stdout));
        assert!(stderr.contains("blocking diagnostics"), "{output}: {stderr}");
        assert!(stderr.contains("category 3"), "the warning keeps its message: {stderr}");
        assert!(!stderr.contains("Failed to open"), "no viewer launch: {stderr}");
    }
    // No temp viewer file was published either.
    let published = std::fs::read_dir(fixture.tmp_dir())
        .map(|entries| entries.filter_map(Result::ok).any(|entry| entry.file_name().to_string_lossy().starts_with("md-output-")))
        .unwrap_or(false);
    assert!(!published, "a failed composition publishes no viewer file");
}

#[test]
fn a_passive_blocking_diagnostic_stops_before_initialize() {
    let fixture = CliProcessFixture::named("policy_passive_stop");
    let document = fixture.write_file(
        "cwd/doc.md",
        "---\n$schema:\n  label: string\nlabel: \"7\"\ninitialize:\n  - ensure_file: [\"marker.md\", \"x\"]\ncomposed:\n  - when: min(label, 5) > 100\n    then:\n      - stdout: never\n---\nBody\n",
    );
    fixture
        .command()
        .args(["compose", "--deny", "all"])
        .arg(&document)
        .assert()
        .code(1)
        .stdout(predicate::str::is_empty());
    assert!(!fixture.cwd().join("marker.md").exists(), "the lifecycle call check is passive and runs first");
    // The control: without the denial `initialize` runs.
    fixture.command().arg("compose").arg(&document).assert().success();
    assert!(fixture.cwd().join("marker.md").exists());
}

#[test]
fn a_blocking_diagnostic_found_after_an_effect_stops_later_work_without_rollback() {
    let fixture = CliProcessFixture::named("policy_after_effect");
    // `initialize` runs before the body is checked; `composed` and the
    // document come after it.
    let document = fixture.write_file(
        "cwd/doc.md",
        "---\n$schema:\n  label: string\nlabel: \"7\"\ninitialize:\n  - ensure_file: [\"before.md\", \"x\"]\ncomposed:\n  - ensure_file: [\"after.md\", \"x\"]\n---\nBody {{ min(label, 5) }}\n",
    );
    let output = fixture.command().args(["compose", "--deny", "dm.expression.uncertain_call"]).arg(&document).output().unwrap();
    let stderr = stderr_of(&output);
    assert_ne!(output.status.code(), Some(0), "{stderr}");
    assert!(output.stdout.is_empty(), "{}", String::from_utf8_lossy(&output.stdout));
    assert!(stderr.contains("dm.expression.uncertain_call"), "{stderr}");
    assert!(fixture.cwd().join("before.md").exists(), "the earlier effect is not rolled back");
    assert!(!fixture.cwd().join("after.md").exists(), "later work does not run");
}

#[test]
fn a_denied_missing_link_inside_inserted_text_blocks() {
    let fixture = CliProcessFixture::named("policy_inserted_link");
    // The link is produced by an expression, so it is a warning, not an error.
    let document = fixture.write_file("cwd/doc.md", "---\nlink: \"[gone](./gone.md)\"\n---\nSee {{ link }}\n");
    fixture.command().arg("compose").arg(&document).assert().success();
    fixture.command().args(["compose", "--deny", "missing-hyperlinks"]).arg(&document).assert().code(2).stdout(predicate::str::is_empty());
    fixture
        .command()
        .args(["compose", "--deny", "all", "--allow", "missing-hyperlinks"])
        .arg(&document)
        .assert()
        .success();
}

#[test]
fn deny_style_and_deny_all_block_a_style_issue_on_every_render_path() {
    let fixture = CliProcessFixture::named("policy_style");
    let document = fixture.write_file("cwd/doc.md", "---\nstyle:\n  page:\n    bogus-key: 1\n---\n# Doc\n");
    fixture.command().arg(&document).args(["--output", "html"]).assert().success();
    for args in [
        &["--deny", "style", "--output", "html"][..],
        &["--deny", "all", "--output", "html"],
    ] {
        fixture.command().arg(&document).args(args).assert().failure().stdout(predicate::str::is_empty());
    }
    fixture.command().args(["render", "--deny", "style", "--output", "html"]).arg(&document).assert().failure();
    fixture.command().args(["compose", "--deny", "style", "--output", "html"]).arg(&document).assert().failure();
}

// ── Help and completions ───────────────────────────────────────────────

#[test]
fn help_explains_that_an_allowance_controls_recovery_only() {
    let fixture = CliProcessFixture::named("policy_help");
    let output = fixture.command().args(["compose", "--help"]).output().unwrap();
    let help = String::from_utf8_lossy(&output.stdout).split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(help.contains("An allowance controls recovery only"), "{help}");
    assert!(help.contains("shell-timeout:"), "the registry's identifiers are listed: {help}");
    assert!(help.contains("dm.expression.uncertain_call"), "the deny codes are listed: {help}");
}

#[test]
fn completions_offer_the_registry_identifiers_per_command() {
    let fixture = CliProcessFixture::named("policy_completions");
    let complete = |words: &[&str]| {
        let output = fixture
            .command()
            .env("COMPLETE", "fish")
            .arg("--")
            .args(words)
            .output()
            .unwrap();
        String::from_utf8_lossy(&output.stdout).into_owned()
    };
    let allow = complete(&["md", "compose", "--allow", ""]);
    for id in ["all", "missing-hyperlinks", "shell-timeout", "host="] {
        assert!(allow.lines().any(|line| line.starts_with(id)), "`{id}` in {allow}");
    }
    assert!(!allow.contains("style"), "{allow}");
    let disable = complete(&["md", "clean", "--disable", ""]);
    assert!(disable.contains("baseline-schema") && disable.contains("schema-triggers"), "{disable}");
    assert!(!disable.lines().any(|line| line.starts_with("all")), "{disable}");
    let deny = complete(&["md", "render", "--deny", ""]);
    assert!(deny.lines().any(|line| line.starts_with("style")), "{deny}");
}
