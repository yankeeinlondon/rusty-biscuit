use crate::common;

use common::{CliProcessFixture, md_file};
use predicates::prelude::*;
use std::io::Write;

#[test]
fn default_render_is_an_alias_with_the_same_options() {
    let fixture = CliProcessFixture::named("render-alias-options");
    let path = fixture.write_file("cwd/doc.md", "# Alias\n\n- parent\n    - child\n");
    for format in ["terminal", "plain", "markdown", "markdown-plus", "html", "json"] {
        let mut outputs = Vec::new();
        for prefix in [&[][..], &["render"][..]] {
            let output = fixture
                .command()
                .args(prefix)
                .arg(&path)
                .args(["--output", format, "--indent", "2", "--feature", "no-mermaid-rendering"])
                .assert()
                .success()
                .get_output()
                .clone();
            assert!(output.stderr.is_empty(), "{}", String::from_utf8_lossy(&output.stderr));
            assert!(!output.stdout.is_empty());
            outputs.push(output.stdout);
        }
        assert_eq!(outputs[0], outputs[1], "format: {format}");
        if format == "markdown" {
            assert!(String::from_utf8_lossy(&outputs[0]).contains("- parent\n  - child"));
        }
    }
}

#[test]
fn render_features_are_never_combined_across_levels() {
    use clap::Parser;
    use darkmatter_cli::args::{Cli, Command, Feature};

    let cli = Cli::try_parse_from(["md", "doc.md", "--feature", "embed-mermaid-font,no-mermaid-rendering"]).unwrap();
    for _ in 0..2 {
        let Command::Render(options) = cli.command_or_render() else {
            panic!("expected render");
        };
        assert_eq!(options.features, [Feature::EmbedMermaidFont, Feature::NoMermaidRendering]);
    }

    // A root `--feature` before `render` is a usage error, not a prefix of
    // `render`'s own list.
    let cli = Cli::try_parse_from(["md", "--feature", "embed-mermaid-font", "render", "doc.md", "--feature", "no-mermaid-rendering"])
        .unwrap();
    let Command::Render(options) = cli.command_or_render() else {
        panic!("expected render");
    };
    assert_eq!(options.features, [Feature::NoMermaidRendering]);
    assert!(darkmatter_cli::commands::validate_subcommand_usage(&cli).is_err());
}

#[test]
fn test_stdin_rendering_on_a_pipe_outputs_plain_text() {
    let fixture = CliProcessFixture::named("test_stdin_rendering_on_a_pipe_outputs_plain_text");
    fixture
        .command()
        .arg("-")
        .write_stdin("# Hello\n\nWorld")
        .assert()
        .success()
        .stdout(predicate::str::contains("Hello"))
        .stdout(predicate::str::contains("# Hello").not())
        .stdout(predicate::str::contains("World"));
}

#[test]
fn test_file_rendering() {
    let fixture = CliProcessFixture::named("test_file_rendering");
    let tmp = md_file("# Test File\n\nSome content here.\n");

    fixture
        .command()
        .arg(tmp.path())
        .args(["--output", "markdown"])
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
fn test_output_text_alias_is_removed() {
    let fixture = CliProcessFixture::named("test_output_text_alias_is_removed");
    fixture
        .command()
        .args(["--output", "text", "-"])
        .write_stdin("# Alias Test")
        .assert()
        .code(2)
        .stdout("")
        .stderr(predicate::str::contains("invalid value 'text'"));
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
fn test_output_ast_value_is_removed() {
    let fixture = CliProcessFixture::named("test_output_ast_value_is_removed");
    fixture
        .command()
        .args(["--output", "ast", "-"])
        .write_stdin("# Hello\n\nWorld")
        .assert()
        .code(2)
        .stdout("")
        .stderr(predicate::str::contains("invalid value 'ast'"));
}

#[test]
fn test_show_option_with_markdown_output() {
    let fixture = CliProcessFixture::named("test_show_option_with_markdown_output");
    fixture
        .command_builder()
        .application_input("MD_DRY_RUN", "1")
        .build()
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
        .stdout(predicate::str::contains("Hello"))
        .stdout(predicate::str::contains("# Hello").not())
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
        .stdout(predicate::str::contains("Backward"))
        .stdout(predicate::str::contains("# Backward").not());
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

/// The code panel's mode is a document style value (`style.page.code.mode`);
/// `--code-block` belongs to `md schema about` alone.
#[test]
fn render_accepts_every_frontmatter_code_mode() {
    let fixture = CliProcessFixture::named("render_accepts_every_frontmatter_code_mode");
    for mode in ["inverse", "dark", "light", "same"] {
        let tmp = md_file(&format!(
            "---\nstyle:\n  page:\n    code:\n      mode: {mode}\n---\n# Title\n\n```rust\nfn main() {{}}\n```\n"
        ));
        fixture
            .command()
            .arg(tmp.path())
            .assert()
            .success()
            .stdout(predicate::str::contains("Title"));
    }
}

#[test]
fn render_rejects_an_invalid_frontmatter_code_mode() {
    let fixture = CliProcessFixture::named("render_rejects_an_invalid_frontmatter_code_mode");
    let tmp = md_file("---\nstyle:\n  page:\n    code:\n      mode: sideways\n---\n# Title\n");
    fixture
        .command()
        .arg(tmp.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("style.page.code.mode"));
}

#[test]
fn render_code_block_flag_is_an_unknown_argument() {
    let fixture = CliProcessFixture::named("render_code_block_flag_is_an_unknown_argument");
    let tmp = md_file("# Title\n");
    for prefix in [&[][..], &["render"][..]] {
        fixture
            .command()
            .args(prefix)
            .arg(tmp.path())
            .args(["--code-block", "dark"])
            .assert()
            .code(2)
            .stdout(predicate::str::is_empty())
            .stderr(predicate::str::contains("unexpected argument '--code-block'"));
    }
}
