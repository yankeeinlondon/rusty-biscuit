mod common;

use common::{CliProcessFixture, md_file};
use predicates::prelude::*;
use std::io::Write;

#[test]
fn test_stdin_rendering_auto_non_tty_outputs_markdown() {
    let fixture = CliProcessFixture::named("test_stdin_rendering_auto_non_tty_outputs_markdown");
    fixture
        .command()
        .arg("-")
        .write_stdin("# Hello\n\nWorld")
        .assert()
        .success()
        .stdout(predicate::str::contains("# Hello"))
        .stdout(predicate::str::contains("World"));
}

#[test]
fn test_file_rendering() {
    let fixture = CliProcessFixture::named("test_file_rendering");
    let tmp = md_file("# Test File\n\nSome content here.\n");

    fixture
        .command()
        .arg(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("# Test File"));
}

#[test]
fn test_file_not_found() {
    let fixture = CliProcessFixture::named("test_file_not_found");
    fixture
        .command()
        .arg("/tmp/nonexistent-darkmatter-test-file.md")
        .assert()
        .failure();
}

#[test]
fn test_output_markdown_alias_text() {
    let fixture = CliProcessFixture::named("test_output_markdown_alias_text");
    fixture
        .command()
        .args(["--output", "text", "-"])
        .write_stdin("# Alias Test")
        .assert()
        .success()
        .stdout(predicate::str::contains("# Alias Test"));
}

#[test]
fn test_output_html() {
    let fixture = CliProcessFixture::named("test_output_html");
    fixture
        .command()
        .args(["--output", "html", "-"])
        .write_stdin("# Hello\n\nWorld")
        .assert()
        .success()
        .stdout(predicate::str::contains("<style>"))
        .stdout(predicate::str::contains("<h1 id=\"hello\">Hello</h1>"));
}

#[test]
fn test_output_html_alias_browser() {
    let fixture = CliProcessFixture::named("test_output_html_alias_browser");
    fixture
        .command()
        .args(["--output", "browser", "-"])
        .write_stdin("# Hello\n\nWorld")
        .assert()
        .success()
        .stdout(predicate::str::contains("<h1 id=\"hello\">Hello</h1>"));
}

#[test]
fn test_output_markdown_plus_renders_disclosure_as_html_details() {
    let fixture =
        CliProcessFixture::named("test_output_markdown_plus_renders_disclosure_as_html_details");
    let input = "::disclosure Summary\n::details\nBody\n::end-disclosure";
    fixture
        .command()
        .args(["--output", "markdown-plus", "-"])
        .write_stdin(input)
        .assert()
        .success()
        .stdout(predicate::str::contains("<details>"))
        .stdout(predicate::str::contains("<summary>"))
        .stdout(predicate::str::contains("Summary"))
        .stdout(predicate::str::contains("</summary>"))
        .stdout(predicate::str::contains("Body"))
        .stdout(predicate::str::contains("</details>"));
}

#[test]
fn test_output_json_alias_ast() {
    let fixture = CliProcessFixture::named("test_output_json_alias_ast");
    // `--output ast` serializes the render-tree `Document` (`md.as_document()`),
    // whose node discriminant is `kind` (not `type`); `root` is the top-level
    // document node.
    fixture
        .command()
        .args(["--output", "ast", "-"])
        .write_stdin("# Hello\n\nWorld")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"root\""))
        .stdout(predicate::str::contains("\"kind\""))
        .stdout(predicate::str::contains("\"heading\""));
}

#[test]
fn test_show_option_with_markdown_output() {
    let fixture = CliProcessFixture::named("test_show_option_with_markdown_output");
    fixture
        .command()
        .env("MD_DRY_RUN", "1")
        .args(["--output", "markdown", "--show", "-"])
        .write_stdin("# Show Test")
        .assert()
        .success();
}

#[test]
fn test_render_explicit() {
    let fixture = CliProcessFixture::named("test_render_explicit");
    fixture
        .command()
        .args(["render", "-"])
        .write_stdin("# Hello\n\nWorld")
        .assert()
        .success()
        .stdout(predicate::str::contains("# Hello"))
        .stdout(predicate::str::contains("World"));
}

#[test]
fn test_render_default_backward_compat() {
    let fixture = CliProcessFixture::named("test_render_default_backward_compat");
    // md file.md (no subcommand) still works
    let mut tmp = tempfile::NamedTempFile::new().unwrap();
    writeln!(tmp, "# Backward\n\nCompat test.").unwrap();

    fixture
        .command()
        .arg(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("# Backward"));
}

#[test]
fn test_render_explicit_with_output() {
    let fixture = CliProcessFixture::named("test_render_explicit_with_output");
    fixture
        .command()
        .args(["render", "--output", "html", "-"])
        .write_stdin("# Hello\n\nWorld")
        .assert()
        .success()
        .stdout(predicate::str::contains("<h1 id=\"hello\">Hello</h1>"));
}

#[test]
fn test_line_numbers_html_output() {
    let fixture = CliProcessFixture::named("test_line_numbers_html_output");
    let input = "```rust\nfn main() {}\n```";
    // Use `--line-numbers=true` to avoid the optional-arg ambiguity with the
    // `-` stdin marker positional. The bare form `--line-numbers -` would let
    // clap consume `-` as the optional value.
    fixture
        .command()
        .args(["--output", "html", "--line-numbers=true", "-"])
        .write_stdin(input)
        .assert()
        .success()
        .stdout(predicate::str::contains("<style>"))
        .stdout(predicate::str::contains("main"));
}

/// - emits the error type name (`TransclusionError`) on stderr,
/// - emits a human-readable summary (`cycle detected`),
/// - emits a hint-tagged token from the rendered block.
#[test]
fn test_block_rendering_transclusion_cycle_tty() {
    let fixture = CliProcessFixture::named("test_block_rendering_transclusion_cycle_tty");
    let dir = tempfile::TempDir::new().unwrap();
    let a = dir.path().join("a.md");
    let b = dir.path().join("b.md");
    std::fs::write(&a, "# A\n\n::file b.md\n").unwrap();
    std::fs::write(&b, "# B\n\n::file a.md\n").unwrap();

    fixture
        .command()
        .arg("compose")
        .arg(&a)
        .assert()
        .failure()
        .stderr(predicate::str::contains("TransclusionError"))
        .stderr(predicate::str::contains("cycle detected"))
        .stderr(predicate::str::contains("Break the cycle"));
}

/// Non-TTY block rendering: the same cycle error must still produce
/// readable plain text (optimistic 80-column render) when stderr is
/// piped. `assert_cmd` runs commands with piped stdio by default, so
/// this test naturally exercises the non-TTY branch in `main.rs`.
#[test]
fn test_block_rendering_transclusion_cycle_non_tty() {
    let fixture = CliProcessFixture::named("test_block_rendering_transclusion_cycle_non_tty");
    let dir = tempfile::TempDir::new().unwrap();
    let a = dir.path().join("a.md");
    let b = dir.path().join("b.md");
    std::fs::write(&a, "# A\n\n::file b.md\n").unwrap();
    std::fs::write(&b, "# B\n\n::file a.md\n").unwrap();

    let output = fixture.command().arg("compose").arg(&a).output().unwrap();

    assert!(!output.status.success(), "expected non-zero exit code");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("cycle detected"),
        "stderr should contain human-readable summary in non-TTY mode\nstderr:\n{stderr}"
    );
    assert!(
        stderr.contains("Break the cycle"),
        "stderr should contain hint from rendered block in non-TTY mode\nstderr:\n{stderr}"
    );
}

#[test]
fn render_accepts_code_block_flag() {
    let fixture = CliProcessFixture::named("render_accepts_code_block_flag");
    let tmp = md_file("# Title\n\n```rust\nfn main() {}\n```\n");
    for mode in ["inverse", "dark", "light", "same"] {
        fixture
            .command()
            .args(["--code-block", mode])
            .arg(tmp.path())
            .assert()
            .success()
            .stdout(predicate::str::contains("Title"));
    }
}

#[test]
fn render_rejects_invalid_code_block_value() {
    let fixture = CliProcessFixture::named("render_rejects_invalid_code_block_value");
    let tmp = md_file("# Title\n");
    fixture
        .command()
        .args(["--code-block", "sideways"])
        .arg(tmp.path())
        .assert()
        .failure();
}
