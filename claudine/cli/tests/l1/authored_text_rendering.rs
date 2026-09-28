//! Authored text reaches the agent and the terminal as written: an
//! interpolation literal converts in every composed file, and author text in
//! the dry-run header or a diagnostic keeps every character.

use std::path::Path;

use crate::common;

use common::{CliProcessFixture, strip_ansi, write, write_dry_run_provider_stub};

/// `claudine compose --dry-run <document>`; returns (stdout, stderr), both
/// ANSI-stripped.
fn dry_run(fixture: &CliProcessFixture, document: &Path, columns: &str) -> (String, String) {
    let output = fixture
        .command()
        .env("COLUMNS", columns)
        .args(["compose", "--dry-run"])
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

const DESCRIPTION: &str =
    "1. **_pr/open.md** opens it.\n2. **_pr/triage.md** triages it.\n\nSee `_pr/_report.md`.";

fn description_document(fixture: &CliProcessFixture) -> std::path::PathBuf {
    let indented: String = DESCRIPTION
        .lines()
        .map(|line| {
            if line.is_empty() {
                "\n".to_string()
            } else {
                format!("    {line}\n")
            }
        })
        .collect();
    let document = fixture.cwd().join("described.md");
    write(
        &document,
        &format!("---\ndescription: |-\n{indented}---\nhello\n"),
    );
    document
}

/// The value-column text of the header table's `Description` row, one entry
/// per rendered line, right-trimmed.
fn description_cell(stderr: &str) -> Vec<String> {
    let mut cell = Vec::new();
    let mut inside = false;
    for line in stderr.lines() {
        let cells: Vec<&str> = line.split('│').collect();
        if cells.len() < 4 {
            continue;
        }
        let field = cells[1].trim();
        if field == "Description" {
            inside = true;
        } else if !field.is_empty() {
            inside = false;
        }
        if inside {
            cell.push(cells[2].trim().to_string());
        }
    }
    cell
}

#[test]
fn header_description_renders_exactly_as_authored() {
    let fixture = CliProcessFixture::named("header-description-exact");
    let document = description_document(&fixture);

    let (_, stderr) = dry_run(&fixture, &document, "200");
    assert_eq!(
        description_cell(&stderr).join("\n"),
        DESCRIPTION,
        "{stderr}"
    );
    assert!(!stderr.contains("</i>"), "{stderr}");
}

#[test]
fn header_description_keeps_every_character_at_a_narrow_width() {
    let fixture = CliProcessFixture::named("header-description-narrow");
    let document = description_document(&fixture);

    let (_, stderr) = dry_run(&fixture, &document, "40");
    let rendered = description_cell(&stderr).join(" ");
    let words = |text: &str| {
        text.split_whitespace()
            .map(str::to_string)
            .collect::<Vec<_>>()
    };
    assert_eq!(words(&rendered), words(DESCRIPTION), "{stderr}");
    assert!(!stderr.contains("</i>"), "{stderr}");
}

#[test]
fn diagnostic_quotes_an_underscored_root_exactly() {
    let fixture = CliProcessFixture::named("diagnostic-underscored-root");
    write_dry_run_provider_stub(fixture.bin_dir(), "goose");
    let document = fixture.cwd().join("diag.md");
    write(
        &document,
        "---\nstart:\n    info: \"count is {{ _loop_countx }}\"\n---\nhello\n",
    );

    let output = fixture
        .command()
        .env("COLUMNS", "400")
        .args(["compose", "--goose"])
        .arg(&document)
        .assert()
        .failure()
        .get_output()
        .clone();
    let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr));
    assert!(
        stderr.contains("unknown root '_loop_countx' in '{{ _loop_countx }}'"),
        "{stderr}"
    );
}
