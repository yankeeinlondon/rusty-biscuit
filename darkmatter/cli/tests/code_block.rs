//! Phase 4: `md code-block` CLI tests.
//!
//! Exercises the new `md code-block <file | content>` subcommand end-to-end:
//! file input, literal content input, language selection, theme override,
//! line numbering, highlighted line ranges, and the parity guarantee that
//! `md code-block` direct output equals a fenced code block in a
//! `DarkmatterPage` for the same parameters.
//!
//! The terminal-output tests assert structural invariants (ANSI escapes
//! present, the language label appears, a specified line is highlighted)
//! rather than byte-exact strings so the suite stays stable across
//! downstream rendering tweaks while still failing on real regressions.

use predicates::prelude::*;
use std::io::Write;

/// Helper to create a `md` command from cargo bin.
fn md_cmd() -> assert_cmd::Command {
    assert_cmd::Command::cargo_bin("md").unwrap()
}

/// Creates a temporary file with the given content and suffix.
///
/// `suffix` controls the file extension — important for file-input tests
/// that rely on `CodeBlock::from_source_file`'s extension-based language
/// resolution to fire.
fn code_file(content: &str, suffix: &str) -> tempfile::NamedTempFile {
    let mut tmp = tempfile::NamedTempFile::with_suffix(suffix).unwrap();
    tmp.write_all(content.as_bytes()).unwrap();
    tmp.flush().unwrap();
    tmp
}

/// Strips ANSI CSI escapes (`ESC[ … letter`) from a string.
///
/// A lightweight regex is enough: the only escapes the code-block renderer
/// emits are color SGRs (`\x1b[…m`) and the clear-to-EOL sequence
/// (`\x1b[K`). Test assertions look at plain text so a regression that
/// moves tokens between spans (and inserts more SGRs) doesn't fail the
/// structural checks.
fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' && chars.peek() == Some(&'[') {
            chars.next();
            for next in chars.by_ref() {
                if next.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// Strips ANSI and asserts the result contains a substring. Returns the
/// stripped string so callers can chain further assertions without
/// re-stripping.
fn assert_ansi_contains(stdout: &[u8], needle: &str) -> String {
    let raw = String::from_utf8_lossy(stdout);
    let plain = strip_ansi(&raw);
    assert!(
        plain.contains(needle),
        "expected stripped stdout to contain {needle:?}\n  stripped: {plain:?}"
    );
    plain
}

// =============================================================================
//                              HELP / LISTING
// =============================================================================

#[test]
fn code_block_help_lists_all_options() {
    md_cmd()
        .args(["code-block", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("INPUT"))
        .stdout(predicate::str::contains("--file"))
        .stdout(predicate::str::contains("--content"))
        .stdout(predicate::str::contains("--language"))
        .stdout(predicate::str::contains("--theme"))
        .stdout(predicate::str::contains("--title"))
        .stdout(predicate::str::contains("--line-numbering"))
        .stdout(predicate::str::contains("--highlight"))
        .stdout(predicate::str::contains("--output"));
}

#[test]
fn code_block_subcommand_appears_in_top_level_help() {
    md_cmd()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("code-block"));
}

// =============================================================================
//                               FILE INPUT
// =============================================================================

#[test]
fn code_block_renders_rust_file_with_explicit_language() {
    let tmp = code_file("fn main() {\n    println!(\"hi\");\n}\n", ".rs");

    let output = md_cmd()
        .args(["code-block"])
        .arg(tmp.path())
        .args(["--language", "rust"])
        .assert()
        .success();
    let stdout = output.get_output().stdout.clone();
    let raw = String::from_utf8_lossy(&stdout);
    assert!(raw.contains("\x1b["), "expected ANSI in output");
    let plain = assert_ansi_contains(&stdout, "main");
    assert!(plain.contains("println"));
    assert!(plain.contains("hi"));
}

#[test]
fn code_block_renders_file_inferred_from_extension() {
    // The CLI should resolve the language from the .py extension when
    // --language is omitted.
    let tmp = code_file("def greet():\n    print('hello')\n", ".py");

    let output = md_cmd()
        .args(["code-block"])
        .arg(tmp.path())
        .assert()
        .success();
    let stdout = output.get_output().stdout.clone();
    let plain = assert_ansi_contains(&stdout, "def greet");
    assert!(plain.contains("print"));
    assert!(plain.contains("hello"));
}

#[test]
fn code_block_explicit_file_flag_treats_input_as_path() {
    // --file must error when the path does not exist, even if the user
    // happened to type literal content that happens to be a valid path.
    md_cmd()
        .args(["code-block", "fn main() {}", "--file"])
        .assert()
        .failure();
}

#[test]
fn code_block_file_flag_reads_existing_file() {
    let tmp = code_file("fn a() {}\nfn b() {}\n", ".rs");
    let output = md_cmd()
        .args(["code-block", "--file"])
        .arg(tmp.path())
        .assert()
        .success();
    let stdout = output.get_output().stdout.clone();
    let plain = assert_ansi_contains(&stdout, "fn a");
    assert!(plain.contains("fn b"));
}

// =============================================================================
//                            LITERAL CONTENT INPUT
// =============================================================================

#[test]
fn code_block_renders_literal_content_with_language_flag() {
    let output = md_cmd()
        .args([
            "code-block",
            "fn main() { println!(\"hi\"); }",
            "--language",
            "rust",
        ])
        .assert()
        .success();
    let stdout = output.get_output().stdout.clone();
    let raw = String::from_utf8_lossy(&stdout);
    assert!(raw.contains("\x1b["), "expected ANSI in output");
    assert_ansi_contains(&stdout, "main");
}

#[test]
fn code_block_content_flag_treats_input_as_literal() {
    // The string "examples/sample.rs" reads as a file path, but with
    // --content it must be rendered verbatim.
    let output = md_cmd()
        .args(["code-block", "examples/sample.rs", "--content", "--language", "text"])
        .assert()
        .success();
    let stdout = output.get_output().stdout.clone();
    assert_ansi_contains(&stdout, "examples/sample.rs");
}

#[test]
fn code_block_falls_back_to_literal_when_path_does_not_exist() {
    let output = md_cmd()
        .args([
            "code-block",
            "fn standalone_literal() {}",
            "--language",
            "rust",
        ])
        .assert()
        .success();
    let stdout = output.get_output().stdout.clone();
    assert_ansi_contains(&stdout, "standalone_literal");
}

// =============================================================================
//                              LANGUAGE OPTIONS
// =============================================================================

#[test]
fn code_block_language_alias_resolves() {
    // `py` is the documented alias for Python.
    md_cmd()
        .args(["code-block", "x = 1", "--language", "py", "--output", "html"])
        .assert()
        .success()
        .stdout(predicate::str::contains("language-python"));
}

#[test]
fn code_block_yml_alias_resolves_to_yaml() {
    md_cmd()
        .args([
            "code-block",
            "k: v",
            "--language",
            "yml",
            "--output",
            "html",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("language-yaml"));
}

#[test]
fn code_block_unknown_language_renders_as_text() {
    // An unknown language token still renders, just with no syntax
    // highlighting (the panel is text-only).
    md_cmd()
        .args([
            "code-block",
            "some content",
            "--language",
            "definitely-not-a-language",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("some content"));
}

// =============================================================================
//                              METADATA OPTIONS
// =============================================================================

#[test]
fn code_block_title_appears_in_header_row() {
    md_cmd()
        .args([
            "code-block",
            "fn main() {}",
            "--language",
            "rust",
            "--title",
            "Greeting",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Greeting"));
}

#[test]
fn code_block_line_numbering_renders_gutter() {
    let output = md_cmd()
        .args([
            "code-block",
            "fn a() {}\nfn b() {}\nfn c() {}",
            "--language",
            "rust",
            "--line-numbering",
        ])
        .assert()
        .success();
    let stdout = output.get_output().stdout.clone();
    let raw = String::from_utf8_lossy(&stdout);
    assert!(raw.contains("\x1b["), "expected ANSI in output");
    let plain = strip_ansi(&raw);
    // The "1" gutter marker should appear in the body (as part of "1 │").
    assert!(
        plain.contains('1'),
        "expected gutter to include line 1, got: {plain:?}"
    );
}

#[test]
fn code_block_highlight_range_changes_panel_background() {
    // The highlight background color differs from the default panel
    // background, so two captures with different highlight lines must
    // differ. This locks in the rendering plumbing reaching the body.
    let make_block = |highlight: &str| {
        let mut cmd = md_cmd();
        cmd.args([
            "code-block",
            "fn a() {}\nfn b() {}\nfn c() {}\nfn d() {}",
            "--language",
            "rust",
            "--line-numbering",
            "--highlight",
            highlight,
        ]);
        cmd
    };

    let highlight_first = make_block("1").assert().success();
    let highlight_last = make_block("4").assert().success();
    let stdout1 = String::from_utf8(highlight_first.get_output().stdout.clone()).unwrap();
    let stdout4 = String::from_utf8(highlight_last.get_output().stdout.clone()).unwrap();
    assert_ne!(
        stdout1, stdout4,
        "different highlight ranges must produce different ANSI backgrounds"
    );
}

#[test]
fn code_block_highlight_invalid_range_errors() {
    // "1-2-3" is structurally invalid: a range must be start-end, not three parts.
    md_cmd()
        .args([
            "code-block",
            "fn a() {}\nfn b() {}",
            "--language",
            "rust",
            "--highlight",
            "1-2-3",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--highlight"));
}

#[test]
fn code_block_highlight_inverted_range_errors() {
    // Start > end is an invalid range.
    md_cmd()
        .args([
            "code-block",
            "fn a() {}\nfn b() {}",
            "--language",
            "rust",
            "--highlight",
            "3-1",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--highlight"));
}

#[test]
fn code_block_highlight_non_numeric_errors() {
    md_cmd()
        .args([
            "code-block",
            "fn a() {}\nfn b() {}",
            "--language",
            "rust",
            "--highlight",
            "abc",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--highlight"));
}

// =============================================================================
//                                OUTPUT MODES
// =============================================================================

#[test]
fn code_block_html_output_emits_pre_code_class() {
    md_cmd()
        .args([
            "code-block",
            "fn main() {}",
            "--language",
            "rust",
            "--output",
            "html",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("<pre>"))
        .stdout(predicate::str::contains("<code"))
        .stdout(predicate::str::contains("language-rust"));
}

#[test]
fn code_block_html_output_escapes_special_chars() {
    md_cmd()
        .args([
            "code-block",
            "x = \"<script>alert(1)</script>\"",
            "--language",
            "py",
            "--output",
            "html",
        ])
        .assert()
        .success()
        // The literal `<script>` text must not survive unescaped.
        .stdout(predicate::str::contains("&lt;script&gt;").or(predicate::str::contains("&#60;script&#62;")));
}

#[test]
fn code_block_markdown_output_emits_fence_with_language() {
    md_cmd()
        .args([
            "code-block",
            "fn main() {}",
            "--language",
            "rust",
            "--output",
            "markdown",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("```rust\n"))
        .stdout(predicate::str::contains("fn main"))
        .stdout(predicate::str::contains("```\n"));
}

#[test]
fn code_block_markdown_output_preserves_title_and_line_numbering() {
    md_cmd()
        .args([
            "code-block",
            "fn a() {}\nfn b() {}",
            "--language",
            "rust",
            "--output",
            "markdown",
            "--title",
            "Demo",
            "--line-numbering",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("```rust title=\"Demo\" line-numbering=true"));
}

#[test]
fn code_block_markdown_output_preserves_highlight() {
    md_cmd()
        .args([
            "code-block",
            "fn a() {}\nfn b() {}\nfn c() {}",
            "--language",
            "rust",
            "--output",
            "markdown",
            "--highlight",
            "1,3",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("highlight=1,3"));
}

#[test]
fn code_block_terminal_output_contains_ansi() {
    // Default output is terminal; ANSI escapes should be present.
    md_cmd()
        .args(["code-block", "fn main() {}", "--language", "rust"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\x1b["));
}

// =============================================================================
//                       THEME OVERRIDE & COLOR MODES
// =============================================================================

#[test]
fn code_block_theme_override_changes_terminal_output() {
    // review-1 finding 1: `--theme` must actually reach the renderer and
    // change the resolved terminal output — not be a silent no-op. `github`
    // and `nord` are distinct themes, so their highlighted SGR colors must
    // differ for the same source.
    let capture = |theme: &str| {
        let mut cmd = md_cmd();
        cmd.args([
            "code-block",
            "fn demo() -> usize { 42 }",
            "--language",
            "rust",
            "--theme",
            theme,
        ]);
        cmd.assert().success()
    };

    let github = capture("github");
    let nord = capture("nord");
    let stdout_github =
        String::from_utf8(github.get_output().stdout.clone()).unwrap();
    let stdout_nord =
        String::from_utf8(nord.get_output().stdout.clone()).unwrap();
    assert!(stdout_github.contains("\x1b["));
    assert!(stdout_nord.contains("\x1b["));
    // The plain-text body survives the strip, proving the renderer ran.
    let plain_github = strip_ansi(&stdout_github);
    let plain_nord = strip_ansi(&stdout_nord);
    assert!(plain_github.contains("fn demo"));
    assert!(plain_nord.contains("fn demo"));
    // The whole point of the override: two distinct themes must not collapse
    // to identical ANSI output. A no-op `--theme` would make these equal.
    assert_ne!(
        stdout_github, stdout_nord,
        "distinct --theme values must produce distinct terminal output",
    );
}

#[test]
fn code_block_theme_override_changes_html_output() {
    // review-1 finding 1: `--theme` must change the resolved HTML output.
    // `github` and `nord` paint different syntax colors, so the emitted
    // `<span style="color: …">` markup must differ for the same source.
    let capture = |theme: &str| {
        let mut cmd = md_cmd();
        cmd.args([
            "code-block",
            "fn demo() -> usize { 42 }",
            "--language",
            "rust",
            "--theme",
            theme,
            "--output",
            "html",
        ]);
        cmd.assert().success()
    };

    let github = capture("github");
    let nord = capture("nord");
    let stdout_github =
        String::from_utf8(github.get_output().stdout.clone()).unwrap();
    let stdout_nord =
        String::from_utf8(nord.get_output().stdout.clone()).unwrap();
    assert!(stdout_github.contains("language-rust"));
    assert!(stdout_nord.contains("language-rust"));
    // Tokens are split across `<span>` elements by the syntax highlighter.
    let plain = strip_html_tags(&stdout_github);
    assert!(plain.contains("fn"));
    assert!(plain.contains("demo"));
    assert!(plain.contains("usize"));
    assert!(plain.contains("42"));
    // A no-op `--theme` would emit identical markup for both themes.
    assert_ne!(
        stdout_github, stdout_nord,
        "distinct --theme values must produce distinct HTML syntax colors",
    );
}

// =============================================================================
//                   ENVIRONMENT THEME FALLBACK (no --theme)
// =============================================================================

#[test]
fn code_block_theme_env_changes_terminal_output_without_flag() {
    // review-2 finding 1: with no `--theme`, the `THEME` env var must drive the
    // resolved code theme on the direct `md code-block` terminal surface. The
    // page path bakes env in at construction, but the page-less direct surface
    // resolves env in the render hook. `github` and `nord` are distinct themes,
    // so their highlighted ANSI must differ for the same source.
    let capture = |theme: &str| {
        let mut cmd = md_cmd();
        cmd.env_remove("NO_COLOR")
            .env_remove("CODE_THEME")
            .env("THEME", theme)
            .args(["code-block", "fn demo() -> usize { 42 }", "--language", "rust"]);
        cmd.assert().success()
    };

    let github = String::from_utf8(capture("github").get_output().stdout.clone()).unwrap();
    let nord = String::from_utf8(capture("nord").get_output().stdout.clone()).unwrap();
    assert!(github.contains("\x1b[") && nord.contains("\x1b["));
    assert_ne!(
        github, nord,
        "THEME must drive the code-block terminal theme when --theme is absent",
    );
}

#[test]
fn code_block_code_theme_env_changes_html_output_without_flag() {
    // review-2 finding 1: `CODE_THEME` must drive the resolved code theme on the
    // direct `md code-block` browser surface when `--theme` is absent.
    let capture = |theme: &str| {
        let mut cmd = md_cmd();
        cmd.env_remove("THEME")
            .env("CODE_THEME", theme)
            .args([
                "code-block",
                "fn demo() -> usize { 42 }",
                "--language",
                "rust",
                "--output",
                "html",
            ]);
        cmd.assert().success()
    };

    let dracula = String::from_utf8(capture("dracula").get_output().stdout.clone()).unwrap();
    let github = String::from_utf8(capture("github").get_output().stdout.clone()).unwrap();
    assert!(dracula.contains("language-rust") && github.contains("language-rust"));
    assert_ne!(
        dracula, github,
        "CODE_THEME must drive the code-block HTML theme when --theme is absent",
    );
}

#[test]
fn code_block_theme_flag_wins_over_theme_env() {
    // review-2 finding 1: an explicit `--theme` must override `THEME`, so the
    // resolved output is identical regardless of the env value.
    let capture = |env_theme: &str| {
        let mut cmd = md_cmd();
        cmd.env_remove("NO_COLOR")
            .env_remove("CODE_THEME")
            .env("THEME", env_theme)
            .args([
                "code-block",
                "fn demo() -> usize { 42 }",
                "--language",
                "rust",
                "--theme",
                "nord",
            ]);
        cmd.assert().success()
    };

    let with_github_env =
        String::from_utf8(capture("github").get_output().stdout.clone()).unwrap();
    let with_nord_env = String::from_utf8(capture("nord").get_output().stdout.clone()).unwrap();
    assert_eq!(
        with_github_env, with_nord_env,
        "--theme must win over THEME, making the output env-independent",
    );
}

// =============================================================================
//                   PARITY WITH FENCED CODE IN DARKMATTERPAGE
// =============================================================================
//
// Same-surface parity is the oracle here (review-4 finding 2). The strong
// terminal and browser direct-vs-fenced parity cases — including combined
// title / line-numbering / highlight / theme metadata — live in-process at the
// lib level (`darkmatter::markdown::code_block` tests:
// `fenced_block_with_metadata_and_theme_equals_direct_terminal` /
// `..._browser`), which is the correct level: terminal parity needs a real
// `Terminal` target, and the implicit CLI render path only emits markdown (not
// terminal) on a non-TTY. The CLI test below covers the HTML target
// end-to-end, comparing the byte-exact `.code-block` fragment rather than mere
// wrapper/body presence.

#[test]
fn code_block_html_output_matches_fenced_markdown_fence() {
    // SAME-SURFACE HTML-to-HTML parity (review-4 finding 2): both surfaces
    // render the same code to the HTML target through `render_html_code_block`,
    // so the `.code-block` fragment must be byte-for-byte identical — not merely
    // both containing the wrapper and body, which cannot catch a real
    // theme/metadata divergence between the two paths. An explicit `--theme`
    // pins the theme on both invocations so the comparison is independent of
    // any ambient `CODE_THEME` / `THEME`.
    let code = "fn a() {}\nfn b() {}\n";

    let direct = md_cmd()
        .args([
            "code-block",
            code,
            "--language",
            "rust",
            "--theme",
            "one-half",
            "--output",
            "html",
        ])
        .assert()
        .success();
    let direct_stdout =
        String::from_utf8(direct.get_output().stdout.clone()).unwrap();

    let fence = md_cmd()
        .args(["--output", "html", "--code-theme", "one-half"])
        .write_stdin(format!("```rust\n{code}```\n"))
        .assert()
        .success();
    let fence_stdout =
        String::from_utf8(fence.get_output().stdout.clone()).unwrap();

    assert_eq!(
        extract_code_block_fragment(&direct_stdout),
        extract_code_block_fragment(&fence_stdout),
        "direct `md code-block` HTML must produce a byte-identical .code-block fragment \
         to the fenced markdown fence;\n  direct: {direct_stdout}\n  fence: {fence_stdout}"
    );
}

/// Strips HTML tags from a string. Used to compare code bodies that have
/// been split across `<span>` boundaries in rendered HTML.
fn strip_html_tags(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_tag = false;
    for c in s.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    out
}

/// Extracts the `.code-block` HTML fragment (optionally preceded by its
/// `.code-block-title`) so direct-vs-fenced parity can be asserted on the
/// stable code-panel markup, independent of any surrounding page wrapper or
/// stylesheet. The code-block `<div>` has no nested `<div>` children (its body
/// is a `<table>` or `<pre>`), so the first `</div>` after it is its close.
fn extract_code_block_fragment(html: &str) -> String {
    let block_marker = "<div class=\"code-block\">";
    let block_start = html
        .find(block_marker)
        .unwrap_or_else(|| panic!("code-block div not found in: {html}"));
    let title_marker = "<div class=\"code-block-title\">";
    let start = html[..block_start].rfind(title_marker).unwrap_or(block_start);
    let close_rel = html[block_start..]
        .find("</div>")
        .unwrap_or_else(|| panic!("code-block close not found in: {html}"));
    let end = block_start + close_rel + "</div>".len();
    html[start..end].to_string()
}
