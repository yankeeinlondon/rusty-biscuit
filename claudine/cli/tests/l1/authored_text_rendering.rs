//! Authored text reaches the agent and the terminal as written: an
//! interpolation literal converts in every composed file, and author text in
//! the dry-run header or a diagnostic keeps every character.

use std::path::Path;

use crate::common;

use common::{CliProcessFixture, strip_ansi, write, write_dry_run_provider_stub};

/// `claudine compose --dry-run <document>`; returns (stdout, stderr), both
/// ANSI-stripped.
fn dry_run(fixture: &CliProcessFixture, document: &Path, columns: &str) -> (String, String) {
    dry_run_with(fixture, document, columns, &[])
}

fn dry_run_with(fixture: &CliProcessFixture, document: &Path, columns: &str, args: &[&str]) -> (String, String) {
    let output = fixture
        .command()
        .env("COLUMNS", columns)
        .args(["compose", "--dry-run"])
        .args(args)
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

/// The value-column text of the header table's `field` row, one entry per
/// rendered line, trimmed.
fn field_cell(stderr: &str, field: &str) -> Vec<String> {
    let mut cell = Vec::new();
    let mut inside = false;
    for line in stderr.lines() {
        let cells: Vec<&str> = line.split('│').collect();
        if cells.len() < 4 {
            continue;
        }
        let label = cells[1].trim();
        if label == field {
            inside = true;
        } else if !label.is_empty() {
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
        field_cell(&stderr, "Description").join("\n"),
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
    let rendered = field_cell(&stderr, "Description").join(" ");
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

/// A file name that opens and closes `_` emphasis if it is read as markup.
const UNDERSCORED_FILE: &str = "_pr_open_.md";
/// A `model` value carrying emphasis delimiters, a tag, and a code span.
const MARKUP_MODEL: &str = "_opus_ </i> `claude_x` *y*";
/// A frontmatter `name`, which replaces the file name as the document label.
const MARKUP_NAME: &str = "`_pr/open.md` </i> _draft_";

fn header_document(fixture: &CliProcessFixture, file: &str, name: Option<&str>) -> std::path::PathBuf {
    let document = fixture.cwd().join(file);
    let name = name.map_or(String::new(), |name| format!("name: '{name}'\n"));
    write(&document, &format!("---\n{name}model: '{MARKUP_MODEL}'\n---\nhello\n"));
    document
}

#[test]
fn header_document_label_and_model_render_exactly_as_authored() {
    let fixture = CliProcessFixture::named("header-label-model");
    // The `Model` row shows a model only once a provider is resolved.
    write_dry_run_provider_stub(fixture.bin_dir(), "claude");
    let from_file = header_document(&fixture, UNDERSCORED_FILE, None);
    let named = header_document(&fixture, "named.md", Some(MARKUP_NAME));

    for columns in ["200", "40"] {
        for (document, label) in [(&from_file, UNDERSCORED_FILE), (&named, MARKUP_NAME)] {
            let (_, stderr) = dry_run_with(&fixture, document, columns, &["--claude"]);
            let words = |cell: Vec<String>| cell.join(" ").split_whitespace().collect::<Vec<_>>().join(" ");
            assert_eq!(words(field_cell(&stderr, "Document")), label, "{columns} columns:\n{stderr}");
            assert_eq!(words(field_cell(&stderr, "Model")), MARKUP_MODEL, "{columns} columns:\n{stderr}");
        }
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
