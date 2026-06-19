mod common;

use common::md_cmd;
use predicates::prelude::*;

#[test]
fn test_compose_basic() {
    md_cmd()
        .args(["compose", "-"])
        .write_stdin("# Hello\n\nWorld")
        .assert()
        .success()
        .stdout(predicate::str::contains("Hello"))
        .stdout(predicate::str::contains("World"));
}

#[test]
fn test_compose_markdown_plus_renders_disclosure_as_details() {
    // `md compose --output markdown-plus` must route the composed document
    // through the MarkdownPlus fold, emitting `<details>`/`<summary>` HTML
    // rather than preserving the `::disclosure` DSL verbatim.
    md_cmd()
        .args(["compose", "--output", "markdown-plus", "-"])
        .write_stdin(
            "::disclosure\nLicense *Agreement*\n::details\nKeep your **hands** off.\n::end-disclosure\n",
        )
        .assert()
        .success()
        .stdout(predicate::str::contains("<details>"))
        .stdout(predicate::str::contains("<summary>"))
        .stdout(predicate::str::contains("</summary>"))
        .stdout(predicate::str::contains("</details>"))
        .stdout(predicate::str::contains("::disclosure").not());
}


#[test]
fn test_compose_with_state() {
    md_cmd()
        .args(["compose", "-", "--state", r#"{"name":"Alice"}"#])
        .write_stdin("# Hello {{ name }}")
        .assert()
        .success()
        .stdout(predicate::str::contains("Hello Alice"));
}

#[test]
fn test_compose_output_html() {
    md_cmd()
        .args(["compose", "-", "--output", "html"])
        .write_stdin("# Hello\n\nWorld")
        .assert()
        .success()
        .stdout(predicate::str::contains("<h1 id=\"hello\">Hello</h1>"));
}


#[test]
fn test_compose_strips_frontmatter() {
    md_cmd()
        .args(["compose", "-"])
        .write_stdin("---\ntitle: Test\n---\n# Hello\n\nWorld")
        .assert()
        .success()
        .stdout(predicate::str::contains("# Hello"))
        .stdout(predicate::str::contains("World"))
        .stdout(predicate::str::contains("---").not());
}

/// Regression test for silent YAML parse failure when frontmatter values
/// contain shell substitutions with nested double quotes.
///
/// Before the fix, `"$(cmd "arg")"` broke the YAML parser and the
/// `impl From<String> for Markdown` fallback left the entire `---...---`
/// block inside `content()`, so default compose output leaked the raw
/// frontmatter block.
#[test]
fn test_compose_strips_frontmatter_when_values_contain_shell_substitution() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let md_path = temp_dir.path().join("test.md");
    std::fs::write(
        &md_path,
        "---\nreview: \"\"\ndir: \"$(dirname \"{{review}}\")\"\n---\nBody: {{review}}\n",
    )
    .unwrap();

    // Approve shell commands so the pipeline runs to completion.
    let whitelist_path = temp_dir.path().join(".darkmatter-shell-whitelist");
    std::fs::write(&whitelist_path, "prefix dirname\n").unwrap();

    md_cmd()
        .current_dir(temp_dir.path())
        .arg("compose")
        .arg(&md_path)
        .arg("review=docs/foo.md")
        .assert()
        .success()
        .stdout(predicate::str::contains("Body: docs/foo.md"))
        .stdout(predicate::str::contains("---").not());
}

/// Regression test for double-frontmatter output.
///
/// Before the fix, `--frontmatter` emitted the state-populated frontmatter
/// on top of a raw, unparsed frontmatter block that still lived in
/// `content()`, producing two `---...---` fences. The fix makes parsing
/// succeed so exactly one frontmatter block is emitted.
#[test]
fn test_compose_frontmatter_flag_emits_single_block_with_nested_quotes() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let md_path = temp_dir.path().join("test.md");
    std::fs::write(
        &md_path,
        "---\nreview: \"\"\ndir: \"$(dirname \"{{review}}\")\"\n---\nBody\n",
    )
    .unwrap();

    let whitelist_path = temp_dir.path().join(".darkmatter-shell-whitelist");
    std::fs::write(&whitelist_path, "prefix dirname\n").unwrap();

    let output = md_cmd()
        .current_dir(temp_dir.path())
        .arg("compose")
        .arg("--frontmatter")
        .arg(&md_path)
        .arg("review=docs/foo.md")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let stdout = String::from_utf8(output).unwrap();

    // Count frontmatter fence pairs: each frontmatter block has two `---`
    // lines. Before the fix we saw four (two fences). Expect exactly two.
    let fence_count = stdout.lines().filter(|line| line.trim() == "---").count();
    assert_eq!(
        fence_count, 2,
        "expected exactly one frontmatter block (two fences), got {fence_count}.\nstdout:\n{stdout}"
    );
    assert!(
        stdout.contains("review: docs/foo.md"),
        "state-populated review should appear in frontmatter.\nstdout:\n{stdout}"
    );
}

/// Regression test for silent shell-expansion skip.
///
/// Before the fix, malformed frontmatter meant `$(...)` in values was
/// never discovered for execution. This test proves shell expansion runs
/// on frontmatter values authored with nested quotes by observing the
/// expanded result in the `--frontmatter` output.
#[test]
fn test_compose_runs_frontmatter_shell_with_nested_quotes() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let md_path = temp_dir.path().join("test.md");
    std::fs::write(
        &md_path,
        "---\npath: \"docs/foo.md\"\ndir: \"$(dirname \"{{path}}\")\"\n---\nok\n",
    )
    .unwrap();

    let whitelist_path = temp_dir.path().join(".darkmatter-shell-whitelist");
    std::fs::write(&whitelist_path, "prefix dirname\n").unwrap();

    md_cmd()
        .current_dir(temp_dir.path())
        .arg("compose")
        .arg("--frontmatter")
        .arg(&md_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("dir: docs"));
}


