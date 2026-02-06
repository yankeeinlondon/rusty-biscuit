use assert_cmd::cargo::cargo_bin_cmd;
use assert_cmd::Command;
use predicates::prelude::*;

fn bf() -> Command {
    cargo_bin_cmd!("bf")
}

fn fixture(name: &str) -> String {
    format!(
        "{}/tests/fixtures/{name}",
        env!("CARGO_MANIFEST_DIR")
    )
}

// ── Format conversion (file input) ──────────────────────────────────

#[test]
fn toml_to_json_default() {
    bf().arg(fixture("sample.toml"))
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""name": "example""#));
}

#[test]
fn toml_to_yaml() {
    bf().arg(fixture("sample.toml"))
        .arg("--yaml")
        .assert()
        .success()
        .stdout(predicate::str::contains("name: example"));
}

#[test]
fn yaml_to_json() {
    bf().arg(fixture("sample.yaml"))
        .arg("--json")
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""name": "example""#));
}

#[test]
fn yaml_to_toml() {
    bf().arg(fixture("sample.yaml"))
        .arg("--toml")
        .assert()
        .success()
        .stdout(predicate::str::contains("name = \"example\""));
}

#[test]
fn json_to_yaml() {
    bf().arg(fixture("sample.json"))
        .arg("--yaml")
        .assert()
        .success()
        .stdout(predicate::str::contains("name: example"));
}

#[test]
fn json_to_toml() {
    bf().arg(fixture("sample.json"))
        .arg("--toml")
        .assert()
        .success()
        .stdout(predicate::str::contains("name = \"example\""));
}

#[test]
fn json_default_pretty_prints() {
    bf().arg(fixture("sample.json"))
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""name": "example""#));
}

// ── Markdown frontmatter extraction ─────────────────────────────────

#[test]
fn markdown_yaml_frontmatter_to_json() {
    bf().arg(fixture("yaml-frontmatter.md"))
        .arg("--json")
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""title": "Test Document""#))
        .stdout(predicate::str::contains(r#""author": "Jane Doe""#));
}

#[test]
fn markdown_yaml_frontmatter_to_toml() {
    bf().arg(fixture("yaml-frontmatter.md"))
        .arg("--toml")
        .assert()
        .success()
        .stdout(predicate::str::contains("title = \"Test Document\""))
        .stdout(predicate::str::contains("author = \"Jane Doe\""));
}

#[test]
fn markdown_yaml_frontmatter_to_yaml() {
    bf().arg(fixture("yaml-frontmatter.md"))
        .arg("--yaml")
        .assert()
        .success()
        .stdout(predicate::str::contains("title: Test Document"))
        .stdout(predicate::str::contains("author: Jane Doe"));
}

#[test]
fn markdown_yaml_frontmatter_default_is_json() {
    bf().arg(fixture("yaml-frontmatter.md"))
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""title": "Test Document""#));
}

#[test]
fn markdown_toml_frontmatter_to_json() {
    bf().arg(fixture("toml-frontmatter.md"))
        .arg("--json")
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""title": "TOML Frontmatter""#))
        .stdout(predicate::str::contains(r#""draft": true"#))
        .stdout(predicate::str::contains(r#""weight": 42"#));
}

#[test]
fn markdown_toml_frontmatter_to_yaml() {
    bf().arg(fixture("toml-frontmatter.md"))
        .arg("--yaml")
        .assert()
        .success()
        .stdout(predicate::str::contains("title: TOML Frontmatter"))
        .stdout(predicate::str::contains("draft: true"));
}

#[test]
fn markdown_toml_frontmatter_to_toml() {
    bf().arg(fixture("toml-frontmatter.md"))
        .arg("--toml")
        .assert()
        .success()
        .stdout(predicate::str::contains("title = \"TOML Frontmatter\""))
        .stdout(predicate::str::contains("draft = true"))
        .stdout(predicate::str::contains("weight = 42"));
}

#[test]
fn markdown_no_frontmatter_errors() {
    bf().arg(fixture("no-frontmatter.md"))
        .arg("--json")
        .assert()
        .failure()
        .stderr(predicate::str::contains("No frontmatter found"));
}

#[test]
fn markdown_text_output_rejected() {
    bf().arg(fixture("yaml-frontmatter.md"))
        .arg("--text")
        .assert()
        .failure()
        .stderr(predicate::str::contains("--text and --md are only supported for PDF files"));
}

// ── STDIN support ───────────────────────────────────────────────────

#[test]
fn stdin_json_to_yaml() {
    bf().arg("--input-format")
        .arg("json")
        .arg("--yaml")
        .write_stdin(r#"{"greeting": "hello"}"#)
        .assert()
        .success()
        .stdout(predicate::str::contains("greeting: hello"));
}

#[test]
fn stdin_yaml_to_json() {
    bf().arg("--input-format")
        .arg("yaml")
        .arg("--json")
        .write_stdin("greeting: hello\n")
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""greeting": "hello""#));
}

#[test]
fn stdin_toml_to_json() {
    bf().arg("--input-format")
        .arg("toml")
        .arg("--json")
        .write_stdin("greeting = \"hello\"\n")
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""greeting": "hello""#));
}

#[test]
fn stdin_markdown_to_json() {
    bf().arg("--input-format")
        .arg("markdown")
        .arg("--json")
        .write_stdin("---\ntitle: From STDIN\n---\n\n# Body\n")
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""title": "From STDIN""#));
}

#[test]
fn stdin_dash_is_equivalent() {
    bf().arg("-")
        .arg("--input-format")
        .arg("json")
        .arg("--yaml")
        .write_stdin(r#"{"key": "value"}"#)
        .assert()
        .success()
        .stdout(predicate::str::contains("key: value"));
}

#[test]
fn stdin_missing_input_format_errors() {
    bf().write_stdin(r#"{"key": "value"}"#)
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "--input-format is required when reading from STDIN",
        ));
}

#[test]
fn stdin_dash_missing_input_format_errors() {
    bf().arg("-")
        .write_stdin(r#"{"key": "value"}"#)
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "--input-format is required when reading from STDIN",
        ));
}

// ── Mutual exclusivity ──────────────────────────────────────────────

#[test]
fn mutually_exclusive_output_flags() {
    bf().arg(fixture("sample.json"))
        .arg("--json")
        .arg("--yaml")
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot be used with"));
}

// ── Input format override ───────────────────────────────────────────

#[test]
fn input_format_override() {
    // Feed YAML content but with a .json extension doesn't matter if we force input-format
    bf().arg(fixture("sample.yaml"))
        .arg("--input-format")
        .arg("yaml")
        .arg("--json")
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""name": "example""#));
}

// ── Unknown file type ───────────────────────────────────────────────

#[test]
fn unknown_extension_errors() {
    // Create a temp file with unknown extension
    let dir = std::env::temp_dir();
    let path = dir.join("bf-test-unknown.xyz");
    std::fs::write(&path, "some content").unwrap();

    bf().arg(path.to_str().unwrap())
        .assert()
        .failure()
        .stderr(predicate::str::contains("Unknown file type"));

    let _ = std::fs::remove_file(&path);
}
