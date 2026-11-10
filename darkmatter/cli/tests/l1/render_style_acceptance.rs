//! Acceptance examples of the render style surface, spelled exactly as the
//! user would type them, through the real binary.
//!
//! Each criterion is covered more broadly elsewhere (`render_style_matrix`,
//! `layout_*`, `output_envelope`); these pin the documented inputs
//! themselves so a change to any one of them is caught at the example a user
//! would copy.

use std::process::Output;

use serde_json::Value;

use crate::common::CliProcessFixture;

fn run(fixture: &CliProcessFixture, args: &[&str]) -> Output {
    fixture.command_builder().application_input("MD_DRY_RUN", "fail").build().args(args).output().expect("md runs")
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// The diagnostic codes of a data output's one envelope, with its exit status.
fn codes(output: &Output) -> (Option<i32>, Vec<String>) {
    assert!(output.stderr.is_empty(), "a data output owns stderr: {}", stderr(output));
    let value: Value = serde_json::from_slice(&output.stdout).unwrap_or_else(|error| panic!("{error}: {}", stdout(output)));
    let codes = value["diagnostics"]
        .as_array()
        .unwrap_or_else(|| panic!("no diagnostics: {value:#}"))
        .iter()
        .map(|record| format!("{}:{}", record["severity"].as_str().unwrap(), record["code"].as_str().unwrap()))
        .collect();
    (output.status.code(), codes)
}

/// `style="{ 'max-width': 50, margin: 4 }"` caps the content at 50 cells and
/// adds four cells and four rows of margin, without padding a short line.
#[test]
fn an_inline_max_width_and_margin_cap_the_content_without_filling_it() {
    let fixture = CliProcessFixture::named("acceptance-inline-cap");
    fixture.write_file("cwd/doc.md", &format!("Short.\n\n{}\n", "word ".repeat(40)));

    let output = run(&fixture, &["render", "doc.md", "style={ 'max-width': 50, margin: 4 }", "--width", "100", "--output", "plain"]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let text = stdout(&output);
    let lines: Vec<&str> = text.split('\n').collect();

    assert!(lines[..4].iter().all(|line| line.is_empty()), "four margin rows above: {text:?}");
    assert_eq!(lines[4], "    Short.", "a short line is indented by the margin and not filled: {text:?}");
    let words: Vec<&str> = lines.iter().copied().filter(|line| line.contains("word")).collect();
    assert!(words.len() > 1, "the paragraph wraps at the cap: {text:?}");
    for line in &words {
        assert!(line.starts_with("    word"), "{line:?}");
        assert!(line.chars().count() <= 4 + 50, "content past the 50-cell cap: {line:?}");
    }
    // The last line's own newline, then four margin rows.
    assert_eq!(text.len() - text.trim_end_matches('\n').len(), 1 + 4, "four margin rows below: {text:?}");
}

/// `style.margin=4` and `style="{ margin: 4 }"` are the same request.
#[test]
fn a_path_setter_and_its_inline_object_render_the_same_bytes() {
    let fixture = CliProcessFixture::named("acceptance-setter-forms");
    fixture.write_file("cwd/doc.md", "# Title\n\nBody text.\n");

    let path = run(&fixture, &["doc.md", "style.margin=4", "--output", "plain"]);
    let object = run(&fixture, &["doc.md", "style={ margin: 4 }", "--output", "plain"]);
    assert_eq!(path.status.code(), Some(0), "{}", stderr(&path));
    assert!(stdout(&path).contains("    Title"), "the margin applied: {:?}", stdout(&path));
    assert_eq!(stdout(&path), stdout(&object));
    assert_eq!(object.status.code(), Some(0));
}

const MISSPELLED: &str = "---\nstyle:\n  magin: 4\n---\n# T\n";
const OPEN: &str = "---\n$schema:\n  style:\n    \"<string>\": unknown\nstyle:\n  magin: 4\n---\n# T\n";

/// A frontmatter `style.magin` is a warning to `md render` and a schema
/// failure to `md compose`.
#[test]
fn a_misspelled_frontmatter_style_key_warns_in_render_and_fails_in_compose() {
    let fixture = CliProcessFixture::named("acceptance-magin");
    fixture.write_file("cwd/doc.md", MISSPELLED);

    let render = run(&fixture, &["render", "doc.md", "--output", "json"]);
    assert_eq!(codes(&render), (Some(0), vec!["warning:dm.style.unknown_key".to_owned()]));

    let compose = run(&fixture, &["compose", "doc.md", "--output", "json"]);
    assert_eq!(codes(&compose), (Some(1), vec!["error:dm.schema.unknown_key".to_owned()]));
}

/// With `"<string>": unknown` declared under `style`, neither command reports
/// the same key.
#[test]
fn an_open_style_schema_admits_the_key_in_render_and_compose() {
    let fixture = CliProcessFixture::named("acceptance-magin-open");
    fixture.write_file("cwd/doc.md", OPEN);

    assert_eq!(codes(&run(&fixture, &["render", "doc.md", "--output", "json"])), (Some(0), vec![]));
    assert_eq!(codes(&run(&fixture, &["compose", "doc.md", "--output", "json"])), (Some(0), vec![]));
}

/// `style.hr.margin=4` is an unknown property: the run fails and prints
/// nothing, while `style.hr.padding-left=4` is the supported placement.
#[test]
fn an_hr_margin_setter_fails_as_an_unknown_property() {
    let fixture = CliProcessFixture::named("acceptance-hr-margin");
    fixture.write_file("cwd/doc.md", "Above.\n\n---\n\nBelow.\n");

    let output = run(&fixture, &["doc.md", "style.hr.margin=4", "--output", "plain"]);
    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert!(output.stdout.is_empty(), "nothing is rendered: {}", stdout(&output));
    assert!(stderr(&output).contains("`style.hr.margin`"), "{}", stderr(&output));

    let (status, records) = codes(&run(&fixture, &["doc.md", "style.hr.margin=4", "--output", "json"]));
    assert_eq!((status, records), (Some(1), vec!["error:dm.schema.unknown_key".to_owned()]));
}

/// `style="{ margin: "` is not a mapping: an ordinary schema type mismatch
/// naming `style`, with the setter as its origin and no file location.
#[test]
fn a_malformed_whole_style_setter_is_a_schema_type_mismatch() {
    let fixture = CliProcessFixture::named("acceptance-malformed-style");
    fixture.write_file("cwd/doc.md", "# Notes\n");

    let output = run(&fixture, &["doc.md", "style={ margin: ", "--output", "json"]);
    let value: Value = serde_json::from_slice(&output.stdout).unwrap_or_else(|error| panic!("{error}: {}", stdout(&output)));
    let (status, records) = codes(&output);
    assert_eq!((status, records), (Some(1), vec!["error:dm.schema.type_mismatch".to_owned()]));
    let record = &value["diagnostics"][0];
    let message = record["message"].as_str().expect("a message");
    assert!(message.contains("`style` takes a mapping"), "{record:#}");
    assert!(message.contains("`style={ margin: `"), "{record:#}");
    assert!(record["source"].is_null(), "a setter is in no file: {record:#}");
}

/// A repeated whole-style setter replaces the earlier assignment before it is
/// read, so the earlier one's duplicate key does not fail the render.
#[test]
fn a_superseded_duplicate_key_style_setter_is_not_read() {
    let fixture = CliProcessFixture::named("acceptance-superseded-style");
    fixture.write_file("cwd/doc.md", "# Notes\n");

    let output = run(&fixture, &["doc.md", "style={ margin: 1, margin: 2 }", "style={ margin: 3 }", "--output", "json"]);
    let value: Value = serde_json::from_slice(&output.stdout).unwrap_or_else(|error| panic!("{error}: {}", stdout(&output)));
    let (status, records) = codes(&output);
    assert_eq!((status, records), (Some(0), Vec::<String>::new()), "{value:#}");
    assert_eq!(value["outcome"], "success", "{value:#}");
}

/// A setter typo under a document's own style schema suggests that schema's
/// key, never a built-in one it rejects.
#[test]
fn a_custom_style_schema_setter_typo_suggests_the_custom_key() {
    let fixture = CliProcessFixture::named("acceptance-custom-style-suggestion");
    fixture.write_file("cwd/doc.md", "---\n$schema:\n    style:\n        accent: string\n---\n# Notes\n");

    let messages = |setter: &str| {
        let output = run(&fixture, &["doc.md", setter, "--output", "json"]);
        let value: Value = serde_json::from_slice(&output.stdout).unwrap_or_else(|error| panic!("{error}: {}", stdout(&output)));
        let (status, records) = codes(&output);
        assert_eq!((status, records), (Some(1), vec!["error:dm.schema.unknown_key".to_string()]), "{value:#}");
        value["diagnostics"][0]["message"].as_str().expect("a message").to_string()
    };
    let message = messages("style.acent=blue");
    assert!(message.contains("did you mean `style.accent`?"), "{message}");
    let message = messages("style.magin=4");
    assert!(!message.contains("did you mean"), "{message}");
}
