//! The commands whose `terminal` output is a component override, at a width with
//! no columns: the result keeps its natural size and nothing in it wraps, with
//! `--width 40` as the control on the same input.

use crate::common;

use common::CliProcessFixture;
use std::process::Output;

const LONG: &str = "one two three four five six seven eight nine ten eleven twelve thirteen fourteen fifteen sixteen";

fn run(fixture: &CliProcessFixture, args: &[&str]) -> Output {
    fixture.command().args(args).output().expect("md runs")
}

fn stdout_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

/// Stdout of a run that had to succeed.
fn succeeded(output: &Output) -> String {
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    stdout_of(output)
}

fn widest_line(text: &str) -> usize {
    text.lines().map(|line| line.chars().count()).max().unwrap_or(0)
}

/// `text` has `LONG` whole, on one line, and nothing near a filled width.
fn assert_natural(text: &str) {
    assert!(text.lines().any(|line| line.contains(LONG)), "wrapped:\n{text}");
    assert!(widest_line(text) < LONG.len() + 200, "filled:\n{text}");
    assert!(text.len() < 16 * 1024, "{} bytes", text.len());
}

/// Marks the launch directory a repository without spawning `git`: trigger
/// schemas are discovered within one.
fn init_repository(fixture: &CliProcessFixture) {
    fixture.write_file("cwd/.git/HEAD", "ref: refs/heads/main\n");
    fixture.write_file("cwd/.git/config", "[core]\n\trepositoryformatversion = 0\n");
    std::fs::create_dir_all(fixture.cwd().join(".git/objects")).expect("objects");
    std::fs::create_dir_all(fixture.cwd().join(".git/refs")).expect("refs");
}

fn document(name: &str) -> CliProcessFixture {
    let fixture = CliProcessFixture::named(name);
    fixture.write_file("cwd/doc.md", &format!("# {LONG}\n\nbody\n"));
    fixture.write_file("cwd/other.md", &format!("# {LONG}\n\nbody\n\n## added {LONG}\n\nmore\n"));
    fixture
}

#[test]
fn toc_at_no_width() {
    let fixture = document("width-overrides-toc");
    assert_natural(&succeeded(&run(&fixture, &["toc", "doc.md", "--output", "plain", "--width", "unlimited"])));
}

#[test]
fn toc_at_source() {
    let fixture = document("width-overrides-toc-source");
    assert_natural(&succeeded(&run(&fixture, &["toc", "doc.md", "--output", "plain", "--width", "source"])));
}

#[test]
fn delta_at_no_width() {
    let fixture = document("width-overrides-delta");
    let text = succeeded(&run(&fixture, &["delta", "doc.md", "other.md", "--output", "plain", "--width", "unlimited"]));
    assert_natural(&text);
}

#[test]
fn graph_at_no_width() {
    let fixture = document("width-overrides-graph");
    let long_name = format!("{}.md", LONG.replace(' ', "-"));
    fixture.write_file(format!("cwd/{long_name}"), "child\n");
    fixture.write_file("cwd/linker.md", &format!("[child](./{long_name})\n"));
    let text = succeeded(&run(&fixture, &["graph", "linker.md", "--output", "plain", "--width", "unlimited"]));
    assert!(text.contains(&long_name), "the link keeps its whole target:\n{text}");
    assert!(widest_line(&text) < long_name.len() + 200, "{text}");
}

#[test]
fn graph_truncates_at_a_column_count() {
    let fixture = document("width-overrides-graph-control");
    let long_name = format!("{}.md", LONG.replace(' ', "-"));
    fixture.write_file(format!("cwd/{long_name}"), "child\n");
    fixture.write_file("cwd/linker.md", &format!("[child](./{long_name})\n"));
    let text = succeeded(&run(&fixture, &["graph", "linker.md", "--output", "plain", "--width", "40"]));
    assert!(!text.contains(&long_name), "{text}");
}

#[test]
fn code_block_at_no_width() {
    let fixture = document("width-overrides-code-block");
    let text = succeeded(&run(&fixture, &["code-block", "--content", LONG, "--language", "text", "--output", "plain", "--width", "unlimited"]));
    assert_natural(&text);
}

#[test]
fn schema_about_at_no_width() {
    let fixture = document("width-overrides-schema-about");
    let text = succeeded(&run(&fixture, &["schema", "about", "--output", "plain", "--width", "unlimited"]));
    assert!(text.len() < 200_000, "{} bytes", text.len());
    assert!(widest_line(&text) < 2_000, "{}", widest_line(&text));
    assert!(text.contains("Constraint Vocabulary"), "{text}");
}

#[test]
fn schema_about_keeps_its_margins_at_a_column_count() {
    let fixture = document("width-overrides-schema-about-control");
    let text = succeeded(&run(&fixture, &["schema", "about", "--output", "plain", "--width", "60"]));
    assert!(widest_line(&text) <= 60, "{}", widest_line(&text));
}

#[test]
fn schema_validate_at_no_width() {
    let fixture = CliProcessFixture::named("width-overrides-schema-validate");
    let long_name = format!("{}.md", LONG.replace(' ', "-"));
    fixture.write_file(format!("cwd/{long_name}"), "---\ntitle: T\n---\nbody\n");
    let text = succeeded(&run(&fixture, &["schema", "validate", &long_name, "--output", "plain", "--width", "unlimited"]));
    assert!(text.contains(&long_name), "{text}");
    assert!(widest_line(&text) < long_name.len() + 200, "{text}");
}

#[test]
fn clean_save_reports_its_delta_at_no_width() {
    let fixture = CliProcessFixture::named("width-overrides-clean-save");
    let original = format!("Intro:\n- {LONG}\n");
    let path = fixture.write_file("cwd/doc.md", &original);
    let text = succeeded(&run(&fixture, &["clean", "doc.md", "--save", "--output", "plain", "--width", "unlimited"]));
    assert!(!text.is_empty(), "the save reports what it did");
    assert!(widest_line(&text) < LONG.len() + 200, "{text}");
    assert_ne!(std::fs::read_to_string(path).expect("the document"), original);
}

#[test]
fn clean_save_reports_a_frontmatter_repair_at_no_width() {
    let fixture = CliProcessFixture::named("width-overrides-clean-save-repair");
    fixture.write_file("cwd/doc.md", "---\ntitle: @daily-report\n---\nbody\n");
    let text = succeeded(&run(&fixture, &["clean", "doc.md", "--save", "--output", "plain", "--width", "unlimited"]));
    assert!(text.contains("Frontmatter: repairs applied"), "{text}");
    assert!(widest_line(&text) < 200, "{text}");
}

#[test]
fn schema_triggers_at_no_width() {
    let fixture = CliProcessFixture::named("width-overrides-schema-triggers");
    init_repository(&fixture);
    let long_name = format!("{}.md", LONG.replace(' ', "-"));
    fixture.write_file(format!("cwd/{long_name}"), "---\ntitle: T\n---\nbody\n");
    let text = succeeded(&run(&fixture, &["schema", "triggers", &long_name, "--output", "plain", "--width", "unlimited"]));
    assert!(text.contains("Schema roots"), "{text}");
    assert!(text.lines().any(|line| line.contains("Document:") && line.contains(&long_name)), "{text}");
    assert!(widest_line(&text) < 2_000, "{text}");
}

#[test]
fn schema_triggers_wraps_a_root_at_a_column_count() {
    let fixture = CliProcessFixture::named("width-overrides-schema-triggers-control");
    init_repository(&fixture);
    fixture.write_file("cwd/doc.md", "---\ntitle: T\n---\nbody\n");
    let unlimited = succeeded(&run(&fixture, &["schema", "triggers", "doc.md", "--output", "plain", "--width", "unlimited"]));
    let narrow = succeeded(&run(&fixture, &["schema", "triggers", "doc.md", "--output", "plain", "--width", "20"]));
    assert!(narrow.lines().count() > unlimited.lines().count(), "{narrow}\n---\n{unlimited}");
}
