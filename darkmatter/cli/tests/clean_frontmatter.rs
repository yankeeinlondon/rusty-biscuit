//! Level-1 CLI coverage for `md clean`'s invalid-frontmatter pipeline.
//!
//! This target owns the document-shaped rows of the acceptance matrix — D-1
//! through D-4 and D-6 through D-11. The `--json` envelope (D-5) lives in
//! `clean_json.rs` and the schema-resolution contract (D-10) in
//! `clean_schema.rs`, because both need fixture trees this target does not.
//!
//! See `darkmatter/features/2026-07-14-invalid-frontmatter/spec.md`.

mod common;

use common::md_cmd;
use predicates::prelude::*;
use std::fs;

/// Writes `content` into a named `.md` file inside a fresh temp dir.
///
/// A real directory (rather than `NamedTempFile`) is what lets the schema tier
/// resolve trigger roots and `$schema` file references relative to the
/// document, matching how `md clean` is invoked in practice.
fn doc(content: &str) -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("doc.md");
    fs::write(&path, content).unwrap();
    (dir, path)
}

/// Loads one of the feature's four canonical baseline documents.
///
/// Acceptance row G reuses these verbatim across D-1..D-9 so pre- and
/// post-change behavior is comparable against the same bytes.
fn baseline(name: &str) -> String {
    let features = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("features");
    let feature = "2026-07-14-invalid-frontmatter";
    let active = features.join(feature).join("baselines").join(name);
    let path = if active.exists() {
        active
    } else {
        features
            .join("_completed")
            .join(feature)
            .join("baselines")
            .join(name)
    };
    fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read baseline {}: {e}", path.display()))
}

/// Runs `md clean` on `path` and returns STDOUT.
fn clean_to_string(path: &std::path::Path) -> String {
    let assert = md_cmd().arg("clean").arg(path).assert().success();
    String::from_utf8(assert.get_output().stdout.clone()).unwrap()
}

fn assert_normalized_reserved_indicator_document(output: &str) {
    assert!(
        output.starts_with("---\n"),
        "expected canonical opening delimiter: {output:?}"
    );
    assert!(output.contains("title: \"@daily-report\"\n---\n"));
    assert!(!output.contains('\u{feff}'), "UTF-8 BOM must be removed");
    assert!(!output.contains('\r'), "frontmatter must use LF line endings");
}

/// D-1: the flagship case — an unquoted scalar opening with a reserved
/// indicator makes the document unparseable, and `md clean` repairs it.
#[test]
fn test_clean_repairs_reserved_indicator_scalar() {
    let (_dir, path) = doc("---\ntitle: @daily-report\n---\n\n# Daily Report\n\nBody.\n");

    md_cmd()
        .arg("clean")
        .arg(&path)
        .assert()
        .success()
        .stdout(predicate::str::contains("title: \"@daily-report\""))
        .stdout(predicate::str::contains("# Daily Report"));
}

/// D-3: the same repair through `--save`, written back to the file.
#[test]
fn test_clean_save_repairs_reserved_indicator_scalar() {
    let (_dir, path) = doc("---\ntitle: @daily-report\n---\n\n# Daily Report\n\nBody.\n");

    md_cmd().arg("clean").arg(&path).arg("--save").assert().success();

    let saved = fs::read_to_string(&path).unwrap();
    assert!(
        saved.contains("title: \"@daily-report\""),
        "expected quoted scalar in saved file, got:\n{saved}"
    );
}

/// D-2: stdin gets the same repair as file input.
#[test]
fn test_clean_stdin_repairs_reserved_indicator_scalar() {
    md_cmd()
        .args(["clean", "-"])
        .write_stdin("---\ntitle: @daily-report\n---\n\n# Daily Report\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("title: \"@daily-report\""));
}

#[test]
fn test_clean_file_repairs_bom_frontmatter() {
    let (_dir, path) = doc("\u{feff}---\ntitle: @daily-report\n---\n\n# Daily Report\n");

    assert_normalized_reserved_indicator_document(&clean_to_string(&path));
}

#[test]
fn test_clean_stdin_repairs_bom_frontmatter() {
    let assert = md_cmd()
        .args(["clean", "-"])
        .write_stdin("\u{feff}---\ntitle: @daily-report\n---\n\n# Daily Report\n")
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();

    assert_normalized_reserved_indicator_document(&stdout);
}

#[test]
fn test_clean_save_repairs_bom_frontmatter() {
    let (_dir, path) = doc("\u{feff}---\ntitle: @daily-report\n---\n\n# Daily Report\n");

    md_cmd()
        .arg("clean")
        .arg(&path)
        .arg("--save")
        .assert()
        .success();

    assert_normalized_reserved_indicator_document(&fs::read_to_string(path).unwrap());
}

#[test]
fn test_clean_file_repairs_lone_cr_frontmatter() {
    let (_dir, path) = doc("---\rtitle: @daily-report\r---\r\r# Daily Report\r");

    assert_normalized_reserved_indicator_document(&clean_to_string(&path));
}

#[test]
fn test_clean_stdin_repairs_lone_cr_frontmatter() {
    let assert = md_cmd()
        .args(["clean", "-"])
        .write_stdin("---\rtitle: @daily-report\r---\r\r# Daily Report\r")
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();

    assert_normalized_reserved_indicator_document(&stdout);
}

#[test]
fn test_clean_save_repairs_lone_cr_frontmatter() {
    let (_dir, path) = doc("---\rtitle: @daily-report\r---\r\r# Daily Report\r");

    md_cmd()
        .arg("clean")
        .arg(&path)
        .arg("--save")
        .assert()
        .success();

    assert_normalized_reserved_indicator_document(&fs::read_to_string(path).unwrap());
}

/// D-9: `md clean`'s output is a fixed point — re-cleaning reproduces it byte
/// for byte, including the repaired frontmatter.
#[test]
fn test_clean_output_is_idempotent() {
    let (_dir, path) = doc("---\ntitle: @daily-report\ntags:  [a,  b]\n---\n\n# Title\n\nBody.\n");

    let first = md_cmd().arg("clean").arg(&path).assert().success();
    let once = String::from_utf8(first.get_output().stdout.clone()).unwrap();

    let (_dir2, path2) = doc(&once);
    let second = md_cmd().arg("clean").arg(&path2).assert().success();
    let twice = String::from_utf8(second.get_output().stdout.clone()).unwrap();

    assert_eq!(once, twice, "md clean must be a fixed point");
}

/// Frontmatter comments and key order survive a repair, proving the pipeline
/// patches source spans instead of reserializing the parsed value.
#[test]
fn test_clean_preserves_untouched_frontmatter_ranges() {
    let (_dir, path) = doc(
        "---\n# a leading comment\nzebra: last-alphabetically\ntitle: @daily-report\n---\n\n# Body\n",
    );

    md_cmd()
        .arg("clean")
        .arg(&path)
        .assert()
        .success()
        .stdout(predicate::str::contains("# a leading comment"))
        .stdout(predicate::str::contains("title: \"@daily-report\""))
        // Reserialization would sort `title` above `zebra`.
        .stdout(predicate::str::contains(
            "zebra: last-alphabetically\ntitle:",
        ));
}

/// D-8: a document with no frontmatter is passed through untouched. There is
/// no frontmatter block, so no YAML analysis, schema resolution, or
/// trigger-schema git-root walk can run.
#[test]
fn test_clean_no_frontmatter_is_unchanged() {
    let (_dir, path) = doc("# Just A Body\n\nNo frontmatter here.\n");

    md_cmd()
        .arg("clean")
        .arg(&path)
        .assert()
        .success()
        .stdout(predicate::str::contains("# Just A Body"))
        .stdout(predicate::str::contains("---").not());
}

/// D-8: an empty frontmatter block also bypasses the pipeline.
#[test]
fn test_clean_empty_frontmatter_succeeds() {
    let (_dir, path) = doc("---\n---\n\n# Body\n");

    md_cmd().arg("clean").arg(&path).assert().success();
}

/// D-7: YAML no repair can prove stays an error — exit 1, and `--save` must
/// leave the file byte-identical.
#[test]
fn test_clean_unrepairable_yaml_exits_one_and_leaves_file_untouched() {
    let source = "---\ntitle: [unclosed\n  nested: {also broken\n---\n\n# Body\n";
    let (_dir, path) = doc(source);

    md_cmd().arg("clean").arg(&path).assert().failure();

    md_cmd()
        .arg("clean")
        .arg(&path)
        .arg("--save")
        .assert()
        .failure();

    assert_eq!(
        fs::read_to_string(&path).unwrap(),
        source,
        "a failed clean must not modify the file"
    );
}

/// D-11: broken YAML inside a fenced body block is never analyzed or mutated.
#[test]
fn test_clean_ignores_yaml_in_fenced_body_blocks() {
    let (_dir, path) = doc(
        "---\ntitle: Fine\n---\n\n# Examples\n\n```yaml\ntitle: @daily-report\nbroken: [unclosed\n```\n",
    );

    md_cmd()
        .arg("clean")
        .arg(&path)
        .assert()
        .success()
        // Untouched: still unquoted inside the fence.
        .stdout(predicate::str::contains("title: @daily-report"))
        .stdout(predicate::str::contains("broken: [unclosed"));
}

/// D-1 (I-F4 regression): with no schema constraining `release`, the ambiguous
/// scalar `1.20` is reported but never rewritten. Quoting it would need schema
/// proof; coercing it to the number `1.2` would silently drop a digit.
#[test]
fn test_clean_leaves_unconstrained_ambiguous_scalar_byte_identical() {
    let source = baseline("coercible.md");
    let (_dir, path) = doc(&source);

    let assert = md_cmd().arg("clean").arg(&path).assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();

    assert!(
        stdout.contains("release: 1.20"),
        "unconstrained scalar must survive verbatim, got:\n{stdout}"
    );
    assert!(!stdout.contains("1.2\n"), "must not coerce to 1.2:\n{stdout}");
    assert!(!stdout.contains("\"1.20\""), "must not quote without schema proof");
}

/// D-1: frontmatter repair and ordinary body cleanup happen in the same run.
#[test]
fn test_clean_applies_body_cleanup_alongside_frontmatter_repair() {
    let (_dir, path) = doc("---\ntitle: @daily-report\n---\n\n# Body   \n\n-  Alpha\n-  Beta\n");

    md_cmd()
        .arg("clean")
        .arg(&path)
        .assert()
        .success()
        .stdout(predicate::str::contains("title: \"@daily-report\""))
        .stdout(predicate::str::contains("- Alpha"))
        .stdout(predicate::str::contains("-  Alpha").not())
        .stdout(predicate::str::contains("# Body   ").not());
}

/// D-2: stdin with no `-` marker at all takes the same repair path.
#[test]
fn test_clean_implicit_stdin_repairs_reserved_indicator_scalar() {
    md_cmd()
        .arg("clean")
        .write_stdin(baseline("invalid-reserved.md"))
        .assert()
        .success()
        .stdout(predicate::str::contains("title: \"@daily-report\""));
}

/// D-4: `--save` over originally-invalid frontmatter must not claim the
/// document was unchanged. The delta is computed between two *repaired*
/// documents, so it cannot see the frontmatter rewrite; the command reports
/// that repair at the text level instead.
#[test]
fn test_clean_save_reports_frontmatter_repair_rather_than_no_changes() {
    let (_dir, path) = doc(&baseline("invalid-reserved.md"));

    let assert = md_cmd()
        .arg("clean")
        .arg(&path)
        .arg("--save")
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();

    assert!(
        stdout.contains("Frontmatter: repairs applied"),
        "save must surface the frontmatter repair, got:\n{stdout}"
    );
    assert!(
        !stdout.contains("No changes"),
        "the run must not claim nothing changed, got:\n{stdout}"
    );
    assert!(
        fs::read_to_string(&path).unwrap().contains("\"@daily-report\""),
        "the file really was rewritten"
    );
}

/// D-4: a document whose frontmatter needed no repair gets no such notice.
#[test]
fn test_clean_save_omits_repair_notice_for_valid_frontmatter() {
    let (_dir, path) = doc(&baseline("clean-fm.md"));

    let assert = md_cmd()
        .arg("clean")
        .arg(&path)
        .arg("--save")
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();

    assert!(
        !stdout.contains("Frontmatter: repairs applied"),
        "valid frontmatter must not report a repair, got:\n{stdout}"
    );
}

/// D-6: report-only findings are suggestions on STDERR. The exit code stays 0
/// and STDOUT carries only the document — channel separation is the contract,
/// because `md clean` is piped into files and pre-commit hooks.
#[test]
fn test_clean_suggestions_go_to_stderr_with_exit_zero() {
    let (_dir, path) = doc("---\ntitle:\n---\n\n# Body\n");

    let assert = md_cmd().arg("clean").arg(&path).assert().success();
    let output = assert.get_output();
    let stdout = String::from_utf8(output.stdout.clone()).unwrap();
    let stderr = String::from_utf8(output.stderr.clone()).unwrap();

    assert_eq!(output.status.code(), Some(0), "suggestions must not fail the run");
    assert!(
        stderr.contains("suspicious-empty-value"),
        "expected the suggestion on stderr, got:\n{stderr}"
    );
    assert!(
        stderr.contains("frontmatter suggestions"),
        "expected the suggestions header on stderr, got:\n{stderr}"
    );
    // The header and the list must not arrive glued onto one line.
    assert!(
        stderr.contains("frontmatter suggestions (not applied)\n"),
        "header needs its own line, got:\n{stderr:?}"
    );

    assert!(
        !stdout.contains("suspicious-empty-value"),
        "suggestion text must never reach stdout, got:\n{stdout}"
    );
    assert!(
        !stdout.contains("frontmatter suggestions"),
        "suggestion header must never reach stdout, got:\n{stdout}"
    );
    assert!(stdout.contains("# Body"), "stdout still carries the document");
}

/// D-6: a finding is reported exactly once. The syntax tier can run a second
/// pass after its own repairs restore parseability, and re-reporting that
/// pass's restatements would print every suggestion twice.
#[test]
fn test_clean_reports_each_suggestion_once_across_both_syntax_passes() {
    // The reserved indicator forces a repair, which unlocks the second pass;
    // the empty value is visible to both.
    let (_dir, path) = doc("---\ntitle: @daily-report\nempty:\n---\n\n# Body\n");

    let assert = md_cmd().arg("clean").arg(&path).assert().success();
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).unwrap();

    assert_eq!(
        stderr.matches("suspicious-empty-value").count(),
        1,
        "each finding is reported once, got:\n{stderr}"
    );
}

/// D-6: deterministic repairs are applied silently — they are not suggestions.
#[test]
fn test_clean_does_not_suggest_repairs_it_applied() {
    let (_dir, path) = doc(&baseline("invalid-reserved.md"));

    let assert = md_cmd().arg("clean").arg(&path).assert().success();
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).unwrap();

    assert!(
        !stderr.contains("reserved-indicator"),
        "an applied repair must not also be suggested, got:\n{stderr}"
    );
}

/// D-6: a document with nothing to report writes nothing to STDERR.
#[test]
fn test_clean_clean_document_writes_nothing_to_stderr() {
    let (_dir, path) = doc(&baseline("clean-fm.md"));

    let assert = md_cmd().arg("clean").arg(&path).assert().success();
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).unwrap();

    assert!(stderr.is_empty(), "expected silent stderr, got:\n{stderr}");
}

/// D-7: unrepairable frontmatter from stdin also exits 1, with the parse error
/// on stderr and nothing on stdout.
#[test]
fn test_clean_unrepairable_stdin_exits_one_with_error_on_stderr() {
    let assert = md_cmd()
        .args(["clean", "-"])
        .write_stdin("---\ntitle: [unclosed\n---\n\n# Body\n")
        .assert()
        .failure();
    let output = assert.get_output();

    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty(), "no partial document on stdout");
    assert!(
        String::from_utf8(output.stderr.clone())
            .unwrap()
            .contains("frontmatter parse failed"),
        "the parse error is preserved"
    );
}

/// D-9: every fixture class is a fixed point under `md clean`.
#[test]
fn test_clean_is_idempotent_across_fixture_classes() {
    let cases = [
        ("no-frontmatter", baseline("no-fm.md")),
        ("clean-frontmatter", baseline("clean-fm.md")),
        ("invalid-reserved", baseline("invalid-reserved.md")),
        ("unconstrained-coercible", baseline("coercible.md")),
        (
            "whitespace-and-repair",
            "---\ntitle: @daily-report\ntags:  [a,  b]\n---\n\n# Title\n\nBody.\n".to_string(),
        ),
        (
            "report-only-findings",
            "---\ntitle: Fine\nempty:\n---\n\n# Title\n".to_string(),
        ),
    ];

    for (label, source) in cases {
        let (_dir, path) = doc(&source);
        let once = clean_to_string(&path);

        let (_dir2, path2) = doc(&once);
        let twice = clean_to_string(&path2);

        assert_eq!(once, twice, "`{label}` is not a fixed point");
    }
}

/// D-11: the fenced body block is preserved byte for byte, not merely left
/// unquoted. `contains` alone would pass even if the engine reflowed the fence.
#[test]
fn test_clean_leaves_fenced_yaml_body_block_byte_identical() {
    let fence = "```yaml\ntitle: @daily-report\nbroken: [unclosed\nempty:\n```\n";
    let (_dir, path) = doc(&format!("---\ntitle: Fine\n---\n\n# Examples\n\n{fence}"));

    let assert = md_cmd().arg("clean").arg(&path).assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();

    assert!(
        stdout.contains(fence),
        "the fenced block must survive byte for byte, got:\n{stdout}"
    );
}

/// D-11: the body fence is genuinely outside analysis — it produces no
/// diagnostics of its own. The frontmatter here is clean, so any suggestion at
/// all would mean the fence had been analyzed.
#[test]
fn test_clean_fenced_yaml_body_block_produces_no_diagnostics() {
    let (_dir, path) = doc(
        "---\ntitle: Fine\n---\n\n# Examples\n\n```yaml\ntitle: @daily-report\nempty:\ndup: 1\ndup: 2\n```\n",
    );

    let assert = md_cmd().arg("clean").arg(&path).assert().success();
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).unwrap();

    assert!(
        stderr.is_empty(),
        "body fences must not reach the analyzer, got:\n{stderr}"
    );
}
