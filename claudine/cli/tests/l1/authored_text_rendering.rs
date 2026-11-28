//! Authored text reaches the agent and the terminal as written: an
//! interpolation literal converts in every composed file, and a file name in
//! the dry-run report, a run's header, or a diagnostic keeps every character.

use std::path::Path;

use crate::common;

use common::{CliProcessFixture, strip_ansi, write, write_dry_run_provider_stub};

/// `claudine compose --dry-run --claude <document>` with a `claude` on `PATH`
/// (the check only looks it up), so a clean document exits 0; returns
/// (stdout, stderr), both ANSI-stripped.
fn dry_run(fixture: &CliProcessFixture, document: &Path, columns: &str) -> (String, String) {
    write_dry_run_provider_stub(fixture.bin_dir(), "claude");
    let output = fixture
        .command()
        .env("COLUMNS", columns)
        .args(["compose", "--dry-run", "--claude"])
        .arg(document)
        .assert()
        .success()
        .get_output()
        .clone();
    (
        strip_ansi(&String::from_utf8_lossy(&output.stdout)),
        strip_ansi(&String::from_utf8_lossy(&output.stderr)),
    )
}

const LITERAL_ONLY: &str = "only a literal: {{{ expr }}}\n";

#[test]
fn literal_only_prompt_converts_when_composed_directly() {
    let fixture = CliProcessFixture::named("literal-only-direct");
    let document = fixture.cwd().join("literal.md");
    write(&document, LITERAL_ONLY);

    let (stdout, _) = dry_run(&fixture, &document, "120");
    assert_eq!(stdout.trim(), "only a literal: {{ expr }}");
}

#[test]
fn literal_only_partial_converts_when_transcluded() {
    let fixture = CliProcessFixture::named("literal-only-transcluded");
    write(&fixture.cwd().join("literal.md"), LITERAL_ONLY);
    let parent = fixture.cwd().join("parent.md");
    write(&parent, "parent\n\n::file ./literal.md\n");

    let (stdout, _) = dry_run(&fixture, &parent, "120");
    assert_eq!(stdout.trim(), "parent\n\nonly a literal: {{ expr }}");
}

/// Control: a literal beside a real span converts too.
#[test]
fn literal_beside_a_real_span_converts() {
    let fixture = CliProcessFixture::named("literal-beside-span");
    let document = fixture.cwd().join("mixed.md");
    write(&document, "---\nx: 1\n---\nreal {{ x }} and {{{ expr }}}\n");

    let (stdout, _) = dry_run(&fixture, &document, "120");
    assert_eq!(stdout.trim(), "real 1 and {{ expr }}");
}

#[test]
fn diagnostic_quotes_an_underscored_root_exactly() {
    let fixture = CliProcessFixture::named("diagnostic-underscored-root");
    write_dry_run_provider_stub(fixture.bin_dir(), "goose");
    let document = fixture.cwd().join("diag.md");
    write(&document, "---\ntitle: t\n---\ncount is {{ _loop_countx }}\n");

    let output = fixture
        .command()
        .env("COLUMNS", "400")
        .args(["compose", "--goose"])
        .arg(&document)
        .output()
        .unwrap();
    let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr));
    assert!(
        stderr.contains("`_loop_countx` at ") && stderr.contains("is an undeclared document property"),
        "the advisory quotes the root verbatim: {stderr}"
    );
}

/// A file name that opens and closes `_` emphasis if it is read as markup.
const UNDERSCORED_FILE: &str = "_pr_open_.md";
/// The dry-run report names the document by its file name exactly, at a
/// width that would wrap it and one that would not.
mod check_report_names_an_underscored_file_exactly {
    use super::*;

    #[test]
    fn at_200_columns() {
        assert_report_names_the_file("200");
    }

    #[test]
    fn at_40_columns() {
        assert_report_names_the_file("40");
    }

    fn assert_report_names_the_file(columns: &str) {
        let fixture = CliProcessFixture::named("check-report-file-name");
        let document = fixture.cwd().join(UNDERSCORED_FILE);
        write(&document, "hello\n");

        let (stdout, stderr) = dry_run(&fixture, &document, columns);
        let flat = stderr.split_whitespace().collect::<Vec<_>>().join(" ");
        assert!(flat.contains(&format!("Dry run of {UNDERSCORED_FILE} (no commands run)")), "{columns} columns:\n{stderr}");
        assert_eq!(stdout.trim(), "hello");
    }
}

/// A real run's header names the prompt file, and its source-file status line
/// names the resolved file; both keep an underscored file name as written.
#[test]
fn run_header_and_source_status_keep_an_underscored_file_name() {
    let fixture = CliProcessFixture::named("run-header-file-name");
    fixture.seed_user_config();
    common::drain_interrupt::write_one_line_claude(fixture.bin_dir());
    let document = fixture.cwd().join(UNDERSCORED_FILE);
    write(&document, "hello\n");

    let output = fixture
        .command()
        .env("COLUMNS", "200")
        .args(["compose", "--claude", UNDERSCORED_FILE])
        .assert()
        .success()
        .get_output()
        .clone();
    let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr));
    let flat = stderr.split_whitespace().collect::<Vec<_>>().join(" ");

    assert!(flat.contains(&format!("prompt sourced from {UNDERSCORED_FILE}")), "{stderr}");
    assert!(
        flat.contains(&format!("the file reference was resolved to {UNDERSCORED_FILE} file")),
        "{stderr}"
    );
}
