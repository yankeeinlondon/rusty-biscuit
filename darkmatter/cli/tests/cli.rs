use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;
use std::io::Write;

/// Helper to create a `md` command from cargo bin.
fn md_cmd() -> assert_cmd::Command {
    cargo_bin_cmd!("md")
}

/// Creates a temporary markdown file with the given content.
fn md_file(content: &str) -> tempfile::NamedTempFile {
    use std::io::Write;
    let mut tmp = tempfile::NamedTempFile::new().unwrap();
    write!(tmp, "{}", content).unwrap();
    tmp
}

// =============================================================================
//                          BASIC FUNCTIONALITY TESTS
// =============================================================================

#[test]
fn test_help_flag() {
    md_cmd()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("markdown"))
        .stdout(predicate::str::contains("render"))
        .stdout(predicate::str::contains("clean"))
        .stdout(predicate::str::contains("compose"))
        .stdout(predicate::str::contains("toc"))
        .stdout(predicate::str::contains("delta"))
        .stdout(predicate::str::contains("get"))
        .stdout(predicate::str::contains("set"))
        .stdout(predicate::str::contains("hash"))
        .stdout(predicate::str::contains("graph"));
}

#[test]
fn test_version_flag() {
    md_cmd()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("md"));
}

#[test]
fn test_stdin_rendering_auto_non_tty_outputs_markdown() {
    md_cmd()
        .arg("-")
        .write_stdin("# Hello\n\nWorld")
        .assert()
        .success()
        .stdout(predicate::str::contains("# Hello"))
        .stdout(predicate::str::contains("World"));
}

#[test]
fn test_file_rendering() {
    let tmp = md_file("# Test File\n\nSome content here.\n");

    md_cmd()
        .arg(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("# Test File"));
}

#[test]
fn test_file_not_found() {
    md_cmd()
        .arg("/tmp/nonexistent-darkmatter-test-file.md")
        .assert()
        .failure();
}

#[test]
fn test_output_markdown_alias_text() {
    md_cmd()
        .args(["--output", "text", "-"])
        .write_stdin("# Alias Test")
        .assert()
        .success()
        .stdout(predicate::str::contains("# Alias Test"));
}

#[test]
fn test_output_html() {
    md_cmd()
        .args(["--output", "html", "-"])
        .write_stdin("# Hello\n\nWorld")
        .assert()
        .success()
        .stdout(predicate::str::contains("<style>"))
        .stdout(predicate::str::contains("<h1>Hello</h1>"));
}

#[test]
fn test_output_json_alias_ast() {
    md_cmd()
        .args(["--output", "ast", "-"])
        .write_stdin("# Hello\n\nWorld")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"type\""));
}

#[test]
fn test_show_option_with_markdown_output() {
    md_cmd()
        .env("MD_DRY_RUN", "1")
        .args(["--output", "markdown", "--show", "-"])
        .write_stdin("# Show Test")
        .assert()
        .success();
}

#[test]
fn test_removed_flags_are_rejected() {
    for flag in [
        "--html",
        "--show-html",
        "--ast",
        "--json",
        "--no-images",
        "--toc",
        "--delta",
        "--clean",
        "--clean-save",
        "--fm-merge-with",
        "--fm-defaults",
    ] {
        md_cmd()
            .args([flag, "-"])
            .write_stdin("# Test")
            .assert()
            .failure()
            .stderr(predicate::str::contains("unexpected argument"));
    }
}

#[test]
fn test_subcommand_rejects_render_options() {
    md_cmd()
        .args(["--output", "html", "toc", "-"])
        .write_stdin("# Test")
        .assert()
        .failure()
        .stderr(predicate::str::contains("subcommands cannot be combined"));
}

// =============================================================================
//                          SUBCOMMAND TESTS
// =============================================================================

#[test]
fn test_toc_subcommand_output() {
    md_cmd()
        .args(["toc", "-"])
        .write_stdin("# Top\n\n## Section A\n\n## Section B\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("Top"))
        .stdout(predicate::str::contains("Section A"));
}

#[test]
fn test_toc_subcommand_json_output() {
    md_cmd()
        .args(["toc", "--json", "-"])
        .write_stdin("# Top\n\n## Section A\n\n## Section B\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"structure\""));
}

#[test]
fn test_toc_subcommand_ignores_tab_indented_frontmatter() {
    let input = "---\nprompt: |-\n\tLine one\n\tLine two\nlast_updated: 2026-02-27\n---\n# macOS Audio\n\n## Details\n";

    md_cmd()
        .args(["toc", "-"])
        .write_stdin(input)
        .assert()
        .success()
        .stdout(predicate::str::contains("macOS Audio"))
        .stdout(predicate::str::contains("Details"))
        .stdout(predicate::str::contains("last_updated").not());
}

#[test]
fn test_delta_subcommand_output() {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path().join("base.md");
    let updated = dir.path().join("updated.md");

    std::fs::write(&base, "# Title\n\nHello\n").unwrap();
    std::fs::write(&updated, "# Title\n\nHello there\n").unwrap();

    md_cmd()
        .arg("delta")
        .arg(&base)
        .arg(&updated)
        .assert()
        .success()
        .stdout(predicate::str::contains("Modified"));
}

#[test]
fn test_delta_subcommand_json_output() {
    let dir = tempfile::tempdir().unwrap();
    let base = dir.path().join("base.md");
    let updated = dir.path().join("updated.md");

    std::fs::write(&base, "# Title\n\nHello\n").unwrap();
    std::fs::write(&updated, "# Title\n\nHello there\n").unwrap();

    md_cmd()
        .arg("delta")
        .arg(&base)
        .arg(&updated)
        .arg("--json")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"classification\""));
}

// =============================================================================
//                          CLEAN SUBCOMMAND TESTS
// =============================================================================

#[test]
fn test_clean_subcommand_stdin() {
    md_cmd()
        .args(["clean", "-"])
        .write_stdin("# Hello\n\nWorld")
        .assert()
        .success()
        .stdout(predicate::str::contains("# Hello"))
        .stdout(predicate::str::contains("World"));
}

#[test]
fn test_clean_subcommand_file() {
    let tmp = md_file("# Hello \n\nWorld  \n");

    md_cmd()
        .arg("clean")
        .arg(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("# Hello"))
        .stdout(predicate::str::contains("World"));
}

#[test]
fn test_clean_subcommand_indent() {
    md_cmd()
        .args(["clean", "-", "--indent", "4"])
        .write_stdin("- Parent\n  - Child\n    - Grandchild\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("\n    - Child"))
        .stdout(predicate::str::contains("\n        - Grandchild"));
}

#[test]
fn test_clean_subcommand_rejects_invalid_indent() {
    md_cmd()
        .args(["clean", "-", "--indent", "3"])
        .write_stdin("- Parent\n  - Child\n")
        .assert()
        .failure()
        .stderr(predicate::str::contains("indent must be one of: 2, 4, 8"));
}

#[test]
fn test_clean_subcommand_save_in_place_reports_delta() {
    let mut tmp = tempfile::NamedTempFile::new().unwrap();
    write!(tmp, "# Hello \n\nWorld  \n").unwrap();

    md_cmd()
        .arg("clean")
        .arg(tmp.path())
        .arg("--save")
        .assert()
        .success()
        .stdout(predicate::str::contains("Whitespace changes only"));

    let updated = std::fs::read_to_string(tmp.path()).unwrap();
    assert!(updated.contains("# Hello"));
    assert!(updated.contains("World"));
    assert!(!updated.contains("# Hello "));
    assert!(!updated.contains("World  "));
    assert!(updated.ends_with('\n'));
    assert!(!updated.ends_with("\n\n"));
}

#[test]
fn test_clean_subcommand_save_verbose_after_subcommand_shows_visual_diff() {
    let mut tmp = tempfile::NamedTempFile::new().unwrap();
    write!(tmp, "# Hello \n\nWorld  \n").unwrap();

    md_cmd()
        .args(["clean", "--save", "-v"])
        .arg(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Whitespace only"))
        .stdout(predicate::str::contains("original"))
        .stdout(predicate::str::contains("updated"))
        .stdout(predicate::str::contains("Content Visual Diff:").not());
}

#[test]
fn test_save_shorthand_cleans_in_place_and_reports_delta() {
    let mut tmp = tempfile::NamedTempFile::new().unwrap();
    write!(tmp, "# Hello \n\nWorld  \n").unwrap();

    md_cmd()
        .arg(tmp.path())
        .arg("--save")
        .assert()
        .success()
        .stdout(predicate::str::contains("Whitespace changes only"));

    let updated = std::fs::read_to_string(tmp.path()).unwrap();
    assert!(updated.contains("# Hello"));
    assert!(updated.contains("World"));
    assert!(!updated.contains("# Hello "));
    assert!(!updated.contains("World  "));
}

#[test]
fn test_clean_save_rejects_stdin() {
    md_cmd()
        .args(["clean", "-", "--save"])
        .write_stdin("# Hello\n\nWorld\n")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "--save requires an input file path (stdin is not supported)",
        ));
}

// =============================================================================
//                          READ SUBCOMMAND TESTS
// =============================================================================

#[test]
fn test_render_explicit() {
    md_cmd()
        .args(["render", "-"])
        .write_stdin("# Hello\n\nWorld")
        .assert()
        .success()
        .stdout(predicate::str::contains("# Hello"))
        .stdout(predicate::str::contains("World"));
}

#[test]
fn test_render_default_backward_compat() {
    // md file.md (no subcommand) still works
    let mut tmp = tempfile::NamedTempFile::new().unwrap();
    writeln!(tmp, "# Backward\n\nCompat test.").unwrap();

    md_cmd()
        .arg(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("# Backward"));
}

#[test]
fn test_render_explicit_with_output() {
    md_cmd()
        .args(["render", "--output", "html", "-"])
        .write_stdin("# Hello\n\nWorld")
        .assert()
        .success()
        .stdout(predicate::str::contains("<h1>Hello</h1>"));
}

// =============================================================================
//                          COMPOSE SUBCOMMAND TESTS
// =============================================================================

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
        .stdout(predicate::str::contains("<h1>Hello</h1>"));
}

#[test]
fn test_compose_perf_emits_report_to_stderr() {
    md_cmd()
        .args(["compose", "-", "--perf"])
        .write_stdin("# Hello\n\nWorld")
        .assert()
        .success()
        .stdout(predicate::str::contains("Hello"))
        .stdout(predicate::str::contains("World"))
        .stderr(predicate::str::contains("Command Setup"))
        .stderr(predicate::str::contains("Compose Pipeline"))
        .stderr(predicate::str::contains("elapsed"));
}

#[test]
fn test_compose_without_perf_no_report_on_stderr() {
    md_cmd()
        .args(["compose", "-"])
        .write_stdin("# Hello\n\nWorld")
        .assert()
        .success()
        .stdout(predicate::str::contains("Hello"))
        .stderr(predicate::str::contains("Command Setup").not());
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

#[test]
fn test_compose_invalid_state() {
    md_cmd()
        .args(["compose", "-", "--state", "bad json"])
        .write_stdin("# Test")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid JSON"));
}

#[test]
fn test_compose_state_requires_json_object() {
    md_cmd()
        .args(["compose", "-", "--state", "[1,2,3]"])
        .write_stdin("# Test")
        .assert()
        .failure()
        .stderr(predicate::str::contains("expected a JSON object"));
}

#[test]
fn test_compose_with_set_overwrites_frontmatter() {
    md_cmd()
        .args([
            "compose",
            "-",
            "--set",
            r#"{"name":"Bob"}"#,
            "--frontmatter",
        ])
        .write_stdin("---\nname: Alice\n---\n# Hello {{ name }}")
        .assert()
        .success()
        .stdout(predicate::str::contains("Hello Bob"))
        .stdout(predicate::str::contains("name: Bob"));
}

#[test]
fn test_compose_with_set_adds_missing_keys() {
    md_cmd()
        .args(["compose", "-", "--set", r#"{"name":"Bob"}"#])
        .write_stdin("# Hello {{ name }}")
        .assert()
        .success()
        .stdout(predicate::str::contains("Hello Bob"));
}

#[test]
fn test_compose_set_and_state_combined() {
    // --state fills defaults, --set overwrites; --set wins on overlap
    md_cmd()
        .args([
            "compose",
            "-",
            "--state",
            r#"{"greeting":"Hi","name":"Alice"}"#,
            "--set",
            r#"{"name":"Bob"}"#,
        ])
        .write_stdin("# {{ greeting }} {{ name }}")
        .assert()
        .success()
        .stdout(predicate::str::contains("Hi Bob"));
}

// =============================================================================
//              COMPOSE SHORTHAND SETTER TESTS
// =============================================================================

#[test]
fn test_compose_shorthand_basic_file_input() {
    let tmp = md_file("# Hello {{ iteration }}\n");
    md_cmd()
        .args(["compose"])
        .arg(tmp.path())
        .arg("iteration=1")
        .assert()
        .success()
        .stdout(predicate::str::contains("Hello 1"));
}

#[test]
fn test_compose_shorthand_basic_stdin() {
    md_cmd()
        .args(["compose", "iteration=1"])
        .write_stdin("# Hello {{ iteration }}")
        .assert()
        .success()
        .stdout(predicate::str::contains("Hello 1"));
}

#[test]
fn test_compose_shorthand_multiple_setters_mixed_types() {
    md_cmd()
        .args(["compose", "iteration=1", "draft=false", "name=Alice"])
        .write_stdin("{{ iteration }} {{ draft }} {{ name }}")
        .assert()
        .success()
        .stdout(predicate::str::contains("1 false Alice"));
}

#[test]
fn test_compose_shorthand_json5_value() {
    md_cmd()
        .args(["compose", r#"meta={author:"Alice"}"#])
        .write_stdin("{{ meta.author }}")
        .assert()
        .success()
        .stdout(predicate::str::contains("Alice"));
}

#[test]
fn test_compose_shorthand_participates_in_validation() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    std::fs::create_dir(temp_dir.path().join("features")).unwrap();
    std::fs::write(
        temp_dir.path().join("features/my-plan.md"),
        "# My Plan\n\nPlan content here.",
    )
    .unwrap();

    let template_path = temp_dir.path().join("template.md");
    std::fs::write(&template_path, "# Task\n\n::file features/{{plan}}\n").unwrap();

    md_cmd()
        .arg("compose")
        .arg(&template_path)
        .arg("plan=my-plan.md")
        .assert()
        .success()
        .stdout(predicate::str::contains("Plan content here."));
}

#[test]
fn test_compose_shorthand_wins_over_state() {
    md_cmd()
        .args([
            "compose",
            "-",
            "--state",
            r#"{"iteration":0}"#,
            "iteration=1",
        ])
        .write_stdin("{{ iteration }}")
        .assert()
        .success()
        .stdout(predicate::str::contains("1"));
}

#[test]
fn test_compose_shorthand_wins_over_set() {
    md_cmd()
        .args(["compose", "-", "--set", r#"{"iteration":1}"#, "iteration=2"])
        .write_stdin("{{ iteration }}")
        .assert()
        .success()
        .stdout(predicate::str::contains("2"));
}

#[test]
fn test_compose_shorthand_duplicate_keys_last_write_wins() {
    md_cmd()
        .args(["compose", "iteration=1", "iteration=2"])
        .write_stdin("{{ iteration }}")
        .assert()
        .success()
        .stdout(predicate::str::contains("2"));
}

#[test]
fn test_compose_shorthand_empty_value() {
    md_cmd()
        .args(["compose", "empty="])
        .write_stdin("'{{ empty }}'")
        .assert()
        .success()
        .stdout(predicate::str::contains("''"));
}

#[test]
fn test_compose_shorthand_empty_key_errors() {
    md_cmd()
        .args(["compose", "=value"])
        .write_stdin("# Test")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid setter '=value'"));
}

#[test]
fn test_compose_shorthand_numeric_leading_key_is_treated_as_input_path() {
    md_cmd()
        .args(["compose", "9key=value"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Failed to load"))
        .stderr(predicate::str::contains("9key=value"));
}

#[test]
fn test_compose_shorthand_setter_before_file_input() {
    let tmp = md_file("# Hello {{ iteration }}\n");
    md_cmd()
        .args(["compose", "iteration=1"])
        .arg(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Hello 1"));
}

#[test]
fn test_compose_shorthand_multiple_non_setter_tokens_error() {
    let tmp = md_file("# Test\n");
    md_cmd()
        .args(["compose"])
        .arg(tmp.path())
        .arg("other.md")
        .assert()
        .failure()
        .stderr(predicate::str::contains("expected at most one input path"));
}

#[test]
fn test_compose_shorthand_path_escape_hatch() {
    let tmp = md_file("# Content\n");
    let path_str = format!("./{}", tmp.path().file_name().unwrap().to_string_lossy());
    md_cmd()
        .args(["compose"])
        .arg(&path_str)
        .arg("key=val")
        .current_dir(tmp.path().parent().unwrap())
        .assert()
        .success()
        .stdout(predicate::str::contains("Content"));
}

#[test]
fn test_compose_set_invalid_json() {
    md_cmd()
        .args(["compose", "-", "--set", "bad json"])
        .write_stdin("# Test")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid JSON"));
}

#[test]
fn test_compose_set_requires_json_object() {
    md_cmd()
        .args(["compose", "-", "--set", "[1,2,3]"])
        .write_stdin("# Test")
        .assert()
        .failure()
        .stderr(predicate::str::contains("expected a JSON object"));
}

// =============================================================================
//              SET OVERLAY TRANCLUSION CLI TESTS
// =============================================================================

#[test]
fn test_set_overlay_child_interpolation() {
    let dir = tempfile::TempDir::new().unwrap();
    std::fs::write(
        dir.path().join("parent.md"),
        r#"::file child.md set.name="Bob""#,
    )
    .unwrap();
    std::fs::write(
        dir.path().join("child.md"),
        "---\nname: Alice\n---\n\nHello {{ name }}\n",
    )
    .unwrap();

    md_cmd()
        .arg("compose")
        .arg(dir.path().join("parent.md"))
        .assert()
        .success()
        .stdout(predicate::str::contains("Hello Bob"))
        .stdout(predicate::str::contains("Alice").not());
}

#[test]
fn test_set_overlay_strict_rejects_invalid() {
    let dir = tempfile::TempDir::new().unwrap();
    std::fs::write(dir.path().join("parent.md"), r#"::file child.md set=42"#).unwrap();
    std::fs::write(dir.path().join("child.md"), "body\n").unwrap();

    md_cmd()
        .arg("compose")
        .arg(dir.path().join("parent.md"))
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid frontmatter assignment"));
}

#[test]
fn test_set_overlay_permissive_invalid_warns() {
    let dir = tempfile::TempDir::new().unwrap();
    std::fs::write(
        dir.path().join("parent.md"),
        r#"::file child.md set=42 set.name="Bob""#,
    )
    .unwrap();
    std::fs::write(
        dir.path().join("child.md"),
        "---\nname: Alice\n---\n\n{{ name }}\n",
    )
    .unwrap();

    md_cmd()
        .arg("compose")
        .arg(dir.path().join("parent.md"))
        .arg("--allow-invalid-frontmatter-assignment")
        .assert()
        .success()
        .stdout(predicate::str::contains("Bob"))
        .stdout(predicate::str::contains("Alice").not());
}

#[test]
fn test_set_overlay_strict_rejects_reassigned() {
    let dir = tempfile::TempDir::new().unwrap();
    std::fs::write(
        dir.path().join("parent.md"),
        r#"::file child.md set.name="Bob" set.name="Mary""#,
    )
    .unwrap();
    std::fs::write(dir.path().join("child.md"), "---\n---\nbody\n").unwrap();

    md_cmd()
        .arg("compose")
        .arg(dir.path().join("parent.md"))
        .assert()
        .failure()
        .stderr(predicate::str::contains("reassigned frontmatter property"));
}

#[test]
fn test_set_overlay_permissive_reassigned_warns_and_rightmost_wins() {
    let dir = tempfile::TempDir::new().unwrap();
    std::fs::write(
        dir.path().join("parent.md"),
        r#"::file child.md set.name="Bob" set.name="Mary""#,
    )
    .unwrap();
    std::fs::write(
        dir.path().join("child.md"),
        "---\nname: Alice\n---\n\n{{ name }}\n",
    )
    .unwrap();

    md_cmd()
        .arg("compose")
        .arg(dir.path().join("parent.md"))
        .arg("--allow-reassigned-frontmatter-property")
        .assert()
        .success()
        .stdout(predicate::str::contains("Mary"))
        .stdout(predicate::str::contains("Bob").not())
        .stdout(predicate::str::contains("Alice").not());
}

// =============================================================================
//                  FRONTMATTER INTERPOLATION TESTS
// =============================================================================

#[test]
fn test_compose_frontmatter_interpolation_basic() {
    md_cmd()
        .args(["compose", "-"])
        .write_stdin("---\nbase: /docs\nspec: \"{{base}}/spec.md\"\n---\nSpec: {{spec}}")
        .assert()
        .success()
        .stdout(predicate::str::contains("Spec: /docs/spec.md"));
}

#[test]
fn test_compose_frontmatter_interpolation_nested_state() {
    md_cmd()
        .args([
            "compose",
            "-",
            "--state",
            r#"{"meta":{"base":"/root","author":"Parent"}}"#,
        ])
        .write_stdin("---\nmeta:\n  author: Local\nspec: \"{{meta.base}}/spec.md\"\n---\n{{spec}}")
        .assert()
        .success()
        .stdout(predicate::str::contains("/root/spec.md"));
}

#[test]
fn test_compose_frontmatter_interpolation_ctx_in_frontmatter_only() {
    // ctx.today referenced only in frontmatter — must still resolve
    md_cmd()
        .args(["compose", "-"])
        .write_stdin("---\nstamp: \"{{ctx.today}}\"\n---\nDate: {{stamp}}")
        .assert()
        .success()
        // The date should not be empty
        .stdout(predicate::str::contains("Date: ").and(predicate::str::contains("Date: \n").not()));
}

// =============================================================================
//                FRONTMATTER FALLBACK INTERPOLATION TESTS
// =============================================================================

#[test]
fn test_compose_frontmatter_double_pipe_fallback() {
    // || in frontmatter interpolation should work the same as | (fallback operator).
    // When the variable is empty, the fallback value should be used.
    md_cmd()
        .args(["compose", "-"])
        .write_stdin(
            "---\nplan: \"\"\nresolved: '{{plan || \"plan.md\"}}'\n---\nFile: {{resolved}}",
        )
        .assert()
        .success()
        .stdout(predicate::str::contains("File: plan.md"));
}

#[test]
fn test_compose_frontmatter_double_pipe_with_set_value() {
    // When --set provides a non-empty value, it should take precedence over the fallback
    md_cmd()
        .args(["compose", "-", "--set", r#"{"plan":"custom.md"}"#])
        .write_stdin(
            "---\nplan: \"\"\nresolved: '{{plan || \"plan.md\"}}'\n---\nFile: {{resolved}}",
        )
        .assert()
        .success()
        .stdout(predicate::str::contains("File: custom.md"));
}

#[test]
fn test_compose_frontmatter_nested_quotes_in_interpolation() {
    // Regression test: double quotes inside {{ }} expressions in YAML
    // frontmatter values (e.g., {{ plan || "plan.md" }}) should not break
    // YAML parsing. The frontmatter parser protects expressions before parsing.
    md_cmd()
        .args([
            "compose",
            "-",
            "--set",
            r#"{"topic":"refactor","phase":1}"#,
            "--frontmatter",
        ])
        .write_stdin(
            "---\ntopic: \"\"\nplan: \"\"\nresolved: \"prefix/{{topic}}/{{plan || \"plan.md\"}}\"\n---\nBody: {{topic}}",
        )
        .assert()
        .success()
        .stdout(predicate::str::contains("resolved: prefix/refactor/plan.md"))
        .stdout(predicate::str::contains("Body: refactor"))
        // Must NOT produce double frontmatter
        .stdout(predicate::str::contains("---\n---").not());
}

// =============================================================================
//              TRANSCLUSION + INTERPOLATION VALIDATION TESTS
// =============================================================================

#[test]
fn test_compose_set_variables_available_during_validation() {
    // Regression test: --set variables must be available during reference
    // validation so that interpolated transclusion paths resolve correctly.
    // Previously, validation ran before --set was parsed, causing
    // `::file features/{{plan}}` to resolve to `features/` (empty plan).
    let temp_dir = tempfile::TempDir::new().unwrap();

    // Create the target file that will be transcluded
    std::fs::create_dir(temp_dir.path().join("features")).unwrap();
    std::fs::write(
        temp_dir.path().join("features/my-plan.md"),
        "# My Plan\n\nPlan content here.",
    )
    .unwrap();

    // Create a template that uses --set variable in a ::file directive
    let template_path = temp_dir.path().join("template.md");
    std::fs::write(&template_path, "# Task\n\n::file features/{{plan}}\n").unwrap();

    md_cmd()
        .arg("compose")
        .arg(&template_path)
        .args(["--set", r#"{"plan":"my-plan.md"}"#])
        .assert()
        .success()
        .stdout(predicate::str::contains("Plan content here."));
}

#[test]
fn test_compose_state_variables_available_during_validation() {
    // Same as above but using --state instead of --set
    let temp_dir = tempfile::TempDir::new().unwrap();

    std::fs::create_dir(temp_dir.path().join("docs")).unwrap();
    std::fs::write(
        temp_dir.path().join("docs/readme.md"),
        "# Readme\n\nReadme content.",
    )
    .unwrap();

    let template_path = temp_dir.path().join("template.md");
    std::fs::write(&template_path, "# Docs\n\n::file docs/{{doc}}\n").unwrap();

    md_cmd()
        .arg("compose")
        .arg(&template_path)
        .args(["--state", r#"{"doc":"readme.md"}"#])
        .assert()
        .success()
        .stdout(predicate::str::contains("Readme content."));
}

// =============================================================================
//                      CTX OVERRIDE BEHAVIOR TESTS
// =============================================================================

#[test]
fn test_compose_scalar_ctx_without_allow_override_fails() {
    md_cmd()
        .args(["compose", "-"])
        .write_stdin("---\nctx: hello\n---\n# Test {{ ctx.today }}")
        .assert()
        .failure()
        .stderr(predicate::str::contains("CtxMergeError"))
        .stderr(predicate::str::contains("JSON object"));
}

#[test]
fn test_compose_scalar_ctx_with_allow_override_succeeds() {
    // --allow-ctx-override downgrades the error to a warning
    md_cmd()
        .args(["compose", "-", "--allow-ctx-override"])
        .write_stdin("---\nctx: hello\n---\n# Test")
        .assert()
        .success()
        .stderr(predicate::str::contains("Document ctx was not an object"));
}

#[test]
fn test_compose_object_ctx_collision_emits_warning() {
    // A document with an object ctx that collides with runtime keys should
    // succeed but emit a collision warning on stderr.
    md_cmd()
        .args(["compose", "-"])
        .write_stdin("---\nctx:\n  today: custom-value\n---\n# Test")
        .assert()
        .success()
        .stderr(predicate::str::contains(
            "conflict with those provided by Darkmatter",
        ));
}

// =============================================================================
//                          HASH SUBCOMMAND TESTS
// =============================================================================

#[test]
fn test_hash_default_outputs_two_hashes() {
    // Default mode: frontmatter_hash-body_hash (each 16 hex chars)
    md_cmd()
        .args(["hash", "-"])
        .write_stdin("---\ntitle: Test\n---\n# Hello\n\nWorld")
        .assert()
        .success()
        .stdout(predicate::str::is_match(r"^[0-9a-f]{16}-[0-9a-f]{16}\n$").unwrap());
}

#[test]
fn test_hash_body_only() {
    md_cmd()
        .args(["hash", "--body", "-"])
        .write_stdin("---\ntitle: Test\n---\n# Hello\n\nWorld")
        .assert()
        .success()
        .stdout(predicate::str::is_match(r"^[0-9a-f]{16}\n$").unwrap());
}

#[test]
fn test_hash_frontmatter_only() {
    md_cmd()
        .args(["hash", "--frontmatter", "-"])
        .write_stdin("---\ntitle: Test\n---\n# Hello\n\nWorld")
        .assert()
        .success()
        .stdout(predicate::str::is_match(r"^[0-9a-f]{16}\n$").unwrap());
}

#[test]
fn test_hash_no_frontmatter() {
    // Document with no frontmatter should still produce a valid hash pair
    md_cmd()
        .args(["hash", "-"])
        .write_stdin("# Hello\n\nWorld")
        .assert()
        .success()
        .stdout(predicate::str::is_match(r"^[0-9a-f]{16}-[0-9a-f]{16}\n$").unwrap());
}

#[test]
fn test_hash_deterministic() {
    // Same input should produce the same hash
    let result1 = md_cmd()
        .args(["hash", "-"])
        .write_stdin("---\ntitle: Test\n---\n# Hello\n\nWorld")
        .output()
        .unwrap();
    let result2 = md_cmd()
        .args(["hash", "-"])
        .write_stdin("---\ntitle: Test\n---\n# Hello\n\nWorld")
        .output()
        .unwrap();

    assert_eq!(result1.stdout, result2.stdout);
}

#[test]
fn test_hash_frontmatter_reordering() {
    // Frontmatter with different key ordering should produce the same hash
    let result1 = md_cmd()
        .args(["hash", "--frontmatter", "-"])
        .write_stdin("---\ntitle: Hello\nauthor: Alice\n---\n# Content")
        .output()
        .unwrap();
    let result2 = md_cmd()
        .args(["hash", "--frontmatter", "-"])
        .write_stdin("---\nauthor: Alice\ntitle: Hello\n---\n# Content")
        .output()
        .unwrap();

    assert_eq!(result1.stdout, result2.stdout);
}

#[test]
fn test_hash_body_whitespace_insensitive() {
    // Body with different whitespace should produce the same hash (non-strict)
    let result1 = md_cmd()
        .args(["hash", "--body", "-"])
        .write_stdin("# Hello\n\nWorld")
        .output()
        .unwrap();
    let result2 = md_cmd()
        .args(["hash", "--body", "-"])
        .write_stdin("# Hello\n\n\nWorld")
        .output()
        .unwrap();

    assert_eq!(result1.stdout, result2.stdout);
}

#[test]
fn test_hash_strict_whitespace_sensitive() {
    // With --strict, different whitespace should produce different hashes
    let result1 = md_cmd()
        .args(["hash", "--body", "--strict", "-"])
        .write_stdin("# Hello\n\nWorld")
        .output()
        .unwrap();
    let result2 = md_cmd()
        .args(["hash", "--body", "--strict", "-"])
        .write_stdin("# Hello\n\n\nWorld")
        .output()
        .unwrap();

    assert_ne!(result1.stdout, result2.stdout);
}

#[test]
fn test_hash_strict_frontmatter_differs_from_normalized() {
    // Strict and non-strict use different serialization strategies, so their
    // hashes should differ (strict uses serde_yaml, non-strict uses sorted canonical JSON)
    let input = "---\ntitle: Hello\nauthor: Alice\n---\n# Content";
    let strict = md_cmd()
        .args(["hash", "--frontmatter", "--strict", "-"])
        .write_stdin(input)
        .output()
        .unwrap();
    let normal = md_cmd()
        .args(["hash", "--frontmatter", "-"])
        .write_stdin(input)
        .output()
        .unwrap();

    assert_ne!(strict.stdout, normal.stdout);
}

#[test]
fn test_hash_from_file() {
    let tmp = md_file("---\ntitle: File Test\n---\n# Hello\n\nWorld\n");

    md_cmd()
        .arg("hash")
        .arg(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::is_match(r"^[0-9a-f]{16}-[0-9a-f]{16}\n$").unwrap());
}

// =============================================================================
//                      HASH DIRECTORY MODE TESTS
// =============================================================================

/// Helper: create a temp directory with markdown files and return the dir.
fn create_hash_dir() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("a.md"),
        "---\ntitle: Alpha\n---\n# Alpha\n\nFirst file.",
    )
    .unwrap();
    std::fs::write(
        dir.path().join("b.md"),
        "---\ntitle: Beta\n---\n# Beta\n\nSecond file.",
    )
    .unwrap();
    // Nested subdirectory
    std::fs::create_dir(dir.path().join("sub")).unwrap();
    std::fs::write(
        dir.path().join("sub/c.md"),
        "---\ntitle: Gamma\n---\n# Gamma\n\nThird file.",
    )
    .unwrap();
    // Non-markdown file (should be ignored)
    std::fs::write(dir.path().join("notes.txt"), "not markdown").unwrap();
    dir
}

#[test]
fn test_hash_directory_default() {
    let dir = create_hash_dir();

    md_cmd()
        .arg("hash")
        .arg(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::is_match(r"^[0-9a-f]{16}-[0-9a-f]{16}\n$").unwrap());
}

#[test]
fn test_hash_directory_body_only() {
    let dir = create_hash_dir();

    md_cmd()
        .args(["hash", "--body"])
        .arg(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::is_match(r"^[0-9a-f]{16}\n$").unwrap());
}

#[test]
fn test_hash_directory_frontmatter_only() {
    let dir = create_hash_dir();

    md_cmd()
        .args(["hash", "--frontmatter"])
        .arg(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::is_match(r"^[0-9a-f]{16}\n$").unwrap());
}

#[test]
fn test_hash_directory_deterministic() {
    let dir = create_hash_dir();

    let result1 = md_cmd().arg("hash").arg(dir.path()).output().unwrap();
    let result2 = md_cmd().arg("hash").arg(dir.path()).output().unwrap();

    assert_eq!(result1.stdout, result2.stdout);
}

#[test]
fn test_hash_directory_differs_from_single_file() {
    let dir = create_hash_dir();

    let dir_result = md_cmd().arg("hash").arg(dir.path()).output().unwrap();
    let file_result = md_cmd()
        .arg("hash")
        .arg(dir.path().join("a.md"))
        .output()
        .unwrap();

    // Directory aggregate should differ from any single file hash
    assert_ne!(dir_result.stdout, file_result.stdout);
}

#[test]
fn test_hash_directory_skips_hidden_dirs() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.md"), "# Visible").unwrap();
    std::fs::create_dir(dir.path().join(".hidden")).unwrap();
    std::fs::write(dir.path().join(".hidden/secret.md"), "# Secret").unwrap();

    // Hash with only visible file
    let with_hidden = md_cmd().arg("hash").arg(dir.path()).output().unwrap();

    // Hash a dir that only has the visible file (no hidden dir)
    let dir2 = tempfile::tempdir().unwrap();
    std::fs::write(dir2.path().join("a.md"), "# Visible").unwrap();

    let without_hidden = md_cmd().arg("hash").arg(dir2.path()).output().unwrap();

    assert_eq!(with_hidden.stdout, without_hidden.stdout);
}

#[test]
fn test_hash_directory_strict() {
    let dir = create_hash_dir();

    let normal = md_cmd().arg("hash").arg(dir.path()).output().unwrap();
    let strict = md_cmd()
        .args(["hash", "--strict"])
        .arg(dir.path())
        .output()
        .unwrap();

    // Strict and normal should produce different hashes (different normalization)
    assert_ne!(normal.stdout, strict.stdout);
}

#[test]
fn test_hash_directory_ignores_non_markdown() {
    // A directory with only non-markdown files should still produce a valid hash
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("notes.txt"), "not markdown").unwrap();
    std::fs::write(dir.path().join("data.json"), "{}").unwrap();

    md_cmd()
        .arg("hash")
        .arg(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::is_match(r"^[0-9a-f]{16}-[0-9a-f]{16}\n$").unwrap());
}

// =============================================================================
//                          GET SUBCOMMAND TESTS
// =============================================================================

const FM_DOC: &str = "---\ntitle: Hello World\nauthor: Alice\ncount: 42\ntags:\n  - rust\n  - cli\n---\n# Content\n\nBody text.";
const FM_DOC_TAB_INDENT: &str = "---\nprompt: |-\n\tLine one\n\tLine two\nlast_updated: 2026-02-27\nmodel: Gemini 3 Pro\n---\n# Content\n";

#[test]
fn test_get_single_property_string() {
    md_cmd()
        .args(["get", "-", "title"])
        .write_stdin(FM_DOC)
        .assert()
        .success()
        .stdout(predicate::str::contains("\"Hello World\""));
}

#[test]
fn test_get_single_property_number() {
    md_cmd()
        .args(["get", "-", "count"])
        .write_stdin(FM_DOC)
        .assert()
        .success()
        .stdout(predicate::str::contains("42"));
}

#[test]
fn test_get_single_property_array() {
    md_cmd()
        .args(["get", "-", "tags"])
        .write_stdin(FM_DOC)
        .assert()
        .success()
        .stdout(predicate::str::contains("rust"))
        .stdout(predicate::str::contains("cli"));
}

#[test]
fn test_get_missing_property_returns_empty_string() {
    md_cmd()
        .args(["get", "-", "nonexistent"])
        .write_stdin(FM_DOC)
        .assert()
        .success()
        .stdout(predicate::str::contains("\"\""));
}

#[test]
fn test_get_multiple_properties_returns_object() {
    md_cmd()
        .args(["get", "-", "title", "author"])
        .write_stdin(FM_DOC)
        .assert()
        .success()
        .stdout(predicate::str::contains("\"title\""))
        .stdout(predicate::str::contains("\"Hello World\""))
        .stdout(predicate::str::contains("\"author\""))
        .stdout(predicate::str::contains("\"Alice\""));
}

#[test]
fn test_get_multiple_with_missing_includes_empty_string() {
    md_cmd()
        .args(["get", "-", "title", "missing"])
        .write_stdin(FM_DOC)
        .assert()
        .success()
        .stdout(predicate::str::contains("\"title\""))
        .stdout(predicate::str::contains("\"Hello World\""))
        .stdout(predicate::str::contains("\"missing\""))
        .stdout(predicate::str::contains("\"\""));
}

#[test]
fn test_get_json5_output() {
    md_cmd()
        .args(["get", "--json5", "-", "title", "count"])
        .write_stdin(FM_DOC)
        .assert()
        .success()
        // JSON5 uses unquoted keys for valid identifiers
        .stdout(predicate::str::contains("title:"))
        .stdout(predicate::str::contains("count:"));
}

#[test]
fn test_get_yaml_output() {
    md_cmd()
        .args(["get", "--yaml", "-", "title"])
        .write_stdin(FM_DOC)
        .assert()
        .success()
        .stdout(predicate::str::contains("Hello World"));
}

#[test]
fn test_get_toml_output() {
    md_cmd()
        .args(["get", "--toml", "-", "title", "count"])
        .write_stdin(FM_DOC)
        .assert()
        .success()
        .stdout(predicate::str::contains("title"))
        .stdout(predicate::str::contains("Hello World"))
        .stdout(predicate::str::contains("count"))
        .stdout(predicate::str::contains("42"));
}

#[test]
fn test_get_from_file() {
    let tmp = md_file("---\nversion: 2\n---\n# Doc\n");

    md_cmd()
        .args(["get"])
        .arg(tmp.path())
        .arg("version")
        .assert()
        .success()
        .stdout(predicate::str::contains("2"));
}

#[test]
fn test_get_no_frontmatter_returns_empty_string() {
    md_cmd()
        .args(["get", "-", "title"])
        .write_stdin("# No frontmatter")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"\""));
}

#[test]
fn test_get_tab_indented_frontmatter_property_is_populated() {
    md_cmd()
        .args(["get", "-", "last_updated"])
        .write_stdin(FM_DOC_TAB_INDENT)
        .assert()
        .success()
        .stdout(predicate::str::contains("\"2026-02-27\""));
}

// -- raw flag tests --

#[test]
fn test_get_raw_string_unquoted() {
    md_cmd()
        .args(["get", "--raw", "-", "title"])
        .write_stdin(FM_DOC)
        .assert()
        .success()
        .stdout("Hello World\n");
}

#[test]
fn test_get_raw_number() {
    md_cmd()
        .args(["get", "--raw", "-", "count"])
        .write_stdin(FM_DOC)
        .assert()
        .success()
        .stdout("42\n");
}

#[test]
fn test_get_raw_null_returns_empty() {
    md_cmd()
        .args(["get", "--raw", "-", "nonexistent"])
        .write_stdin("---\nnonexistent: null\n---\n# Doc")
        .assert()
        .success()
        .stdout("\n");
}

#[test]
fn test_get_raw_array_one_per_line() {
    md_cmd()
        .args(["get", "--raw", "-", "tags"])
        .write_stdin(FM_DOC)
        .assert()
        .success()
        .stdout("rust\ncli\n");
}

#[test]
fn test_get_raw_object_key_value_lines() {
    md_cmd()
        .args(["get", "--raw", "-", "title", "count"])
        .write_stdin(FM_DOC)
        .assert()
        .success()
        .stdout(predicate::str::contains("title: Hello World"))
        .stdout(predicate::str::contains("count: 42"));
}

// -- compact flag tests --

#[test]
fn test_get_compact_array() {
    md_cmd()
        .args(["get", "--compact", "-", "tags"])
        .write_stdin(FM_DOC)
        .assert()
        .success()
        .stdout("[\"rust\",\"cli\"]\n");
}

#[test]
fn test_get_compact_object() {
    md_cmd()
        .args(["get", "--compact", "-", "title", "count"])
        .write_stdin(FM_DOC)
        .assert()
        .success()
        .stdout(predicate::str::contains("\"title\":\"Hello World\""))
        .stdout(predicate::str::contains("\"count\":42"));
}

#[test]
fn test_get_compact_scalar_unchanged() {
    md_cmd()
        .args(["get", "--compact", "-", "title"])
        .write_stdin(FM_DOC)
        .assert()
        .success()
        .stdout("\"Hello World\"\n");
}

// =============================================================================
//                          SET SUBCOMMAND TESTS
// =============================================================================

#[test]
fn test_set_string_value_via_stdin() {
    md_cmd()
        .args(["set", "-", "title", "New Title"])
        .write_stdin("---\ntitle: Old Title\n---\n# Content\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("title: New Title"))
        .stdout(predicate::str::contains("# Content"));
}

#[test]
fn test_set_adds_new_property_via_stdin() {
    md_cmd()
        .args(["set", "-", "author", "Alice"])
        .write_stdin("---\ntitle: Hello\n---\n# Content\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("author: Alice"))
        .stdout(predicate::str::contains("title: Hello"));
}

#[test]
fn test_set_numeric_value() {
    md_cmd()
        .args(["set", "-", "count", "42"])
        .write_stdin("---\ntitle: Test\n---\n# Content\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("count: 42"));
}

#[test]
fn test_set_boolean_value() {
    md_cmd()
        .args(["set", "-", "draft", "true"])
        .write_stdin("---\ntitle: Test\n---\n# Content\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("draft: true"));
}

#[test]
fn test_set_json_array_value() {
    md_cmd()
        .args(["set", "-", "tags", r#"["rust","cli"]"#])
        .write_stdin("---\ntitle: Test\n---\n# Content\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("rust"))
        .stdout(predicate::str::contains("cli"));
}

#[test]
fn test_set_creates_frontmatter_when_none_exists() {
    md_cmd()
        .args(["set", "-", "title", "Brand New"])
        .write_stdin("# No Frontmatter\n\nJust content.\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("title: Brand New"))
        .stdout(predicate::str::contains("# No Frontmatter"));
}

#[test]
fn test_set_updates_file_in_place() {
    let mut tmp = tempfile::NamedTempFile::new().unwrap();
    write!(tmp, "---\ntitle: Original\n---\n# Content\n").unwrap();

    md_cmd()
        .arg("set")
        .arg(tmp.path())
        .args(["title", "Updated", "--save"])
        .assert()
        .success()
        .stdout(predicate::str::is_empty());

    let updated = std::fs::read_to_string(tmp.path()).unwrap();
    assert!(updated.contains("title: Updated"));
    assert!(updated.contains("# Content"));
    assert!(!updated.contains("Original"));
}

#[test]
fn test_set_without_save_does_not_mutate_file() {
    let mut tmp = tempfile::NamedTempFile::new().unwrap();
    write!(tmp, "---\ntitle: Original\n---\n# Content\n").unwrap();

    md_cmd()
        .arg("set")
        .arg(tmp.path())
        .args(["title", "Updated"])
        .assert()
        .success()
        .stdout(predicate::str::contains("title: Updated"))
        .stdout(predicate::str::contains("# Content"));

    // File should be unchanged
    let on_disk = std::fs::read_to_string(tmp.path()).unwrap();
    assert!(on_disk.contains("title: Original"));
}

#[test]
fn test_set_preserves_body_content() {
    let input =
        "---\ntitle: Test\n---\n# Heading\n\nParagraph with **bold** text.\n\n- list item\n";
    md_cmd()
        .args(["set", "-", "version", "2"])
        .write_stdin(input)
        .assert()
        .success()
        .stdout(predicate::str::contains("# Heading"))
        .stdout(predicate::str::contains("**bold**"))
        .stdout(predicate::str::contains("- list item"));
}

#[test]
fn test_get_requires_at_least_one_prop() {
    md_cmd()
        .args(["get", "-"])
        .write_stdin(FM_DOC)
        .assert()
        .failure();
}

// =============================================================================
//                      RM SUBCOMMAND TESTS
// =============================================================================

#[test]
fn test_rm_removes_single_property() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("test.md");
    std::fs::write(
        &file,
        "---\ntitle: Hello\nauthor: Alice\ndate: 2024-01-01\n---\n\n# Content\n",
    )
    .unwrap();

    md_cmd()
        .args(["rm", file.to_str().unwrap(), "author"])
        .assert()
        .success();

    let content = std::fs::read_to_string(&file).unwrap();
    assert!(!content.contains("author:"));
    assert!(content.contains("title: Hello"));
    assert!(content.contains("date: 2024-01-01"));
    assert!(content.contains("# Content"));
}

#[test]
fn test_rm_removes_multiple_properties() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("test.md");
    std::fs::write(
        &file,
        "---\ntitle: Hello\nauthor: Alice\ndate: 2024-01-01\ntags: [rust, cli]\n---\n\n# Content\n",
    )
    .unwrap();

    md_cmd()
        .args(["rm", file.to_str().unwrap(), "author", "tags"])
        .assert()
        .success();

    let content = std::fs::read_to_string(&file).unwrap();
    assert!(!content.contains("author:"));
    assert!(!content.contains("tags:"));
    assert!(content.contains("title: Hello"));
    assert!(content.contains("date: 2024-01-01"));
    assert!(content.contains("# Content"));
}

#[test]
fn test_rm_nonexistent_key_fails() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("test.md");
    std::fs::write(
        &file,
        "---\ntitle: Hello\nauthor: Alice\n---\n\n# Content\n",
    )
    .unwrap();

    md_cmd()
        .args(["rm", file.to_str().unwrap(), "missing"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found in frontmatter"));
}

#[test]
fn test_rm_partial_nonexistent_fails() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("test.md");
    std::fs::write(
        &file,
        "---\ntitle: Hello\nauthor: Alice\n---\n\n# Content\n",
    )
    .unwrap();

    md_cmd()
        .args(["rm", file.to_str().unwrap(), "title", "missing"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found in frontmatter"));

    // File should be unchanged when command fails
    let content = std::fs::read_to_string(&file).unwrap();
    assert!(content.contains("title: Hello"));
    assert!(content.contains("author: Alice"));
}

#[test]
fn test_rm_with_json_output() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("test.md");
    std::fs::write(
        &file,
        "---\ntitle: Hello\nauthor: Alice\ndate: 2024-01-01\n---\n\n# Content\n",
    )
    .unwrap();

    let output = md_cmd()
        .args(["rm", file.to_str().unwrap(), "author", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json_str = String::from_utf8_lossy(&output);
    assert!(json_str.contains("\"removed\""));
    assert!(json_str.contains("\"remaining\""));
    assert!(json_str.contains("\"filename\""));
    assert!(json_str.contains("\"author\""));
    assert!(json_str.contains("\"title\""));
    assert!(json_str.contains("\"date\""));
}

#[test]
fn test_rm_with_verbose_output() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("test.md");
    std::fs::write(
        &file,
        "---\ntitle: Hello\nauthor: Alice\ndate: 2024-01-01\n---\n\n# Content\n",
    )
    .unwrap();

    md_cmd()
        .args(["rm", file.to_str().unwrap(), "author", "-v"])
        .assert()
        .success()
        .stderr(predicate::str::contains("removed"));
}

#[test]
fn test_rm_preserves_body_content() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("test.md");
    let body = "# Heading\n\nParagraph with **bold** text.\n\n- list item\n";
    std::fs::write(
        &file,
        format!("---\ntitle: Test\nauthor: Alice\n---\n\n{}", body),
    )
    .unwrap();

    md_cmd()
        .args(["rm", file.to_str().unwrap(), "author"])
        .assert()
        .success();

    let content = std::fs::read_to_string(&file).unwrap();
    assert!(content.contains("# Heading"));
    assert!(content.contains("**bold**"));
    assert!(content.contains("- list item"));
}

#[test]
fn test_rm_requires_at_least_one_prop() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("test.md");
    std::fs::write(&file, "---\ntitle: Hello\n---\n\n# Content\n").unwrap();

    md_cmd()
        .args(["rm", file.to_str().unwrap()])
        .assert()
        .failure();
}

// =============================================================================
//                      SHELL EXPANSION TESTS
// =============================================================================

#[test]
fn test_compose_with_whitelisted_command_succeeds() {
    let temp_dir = tempfile::TempDir::new().unwrap();

    // Write a markdown file with a shell directive
    let md_path = temp_dir.path().join("test.md");
    std::fs::write(&md_path, "# Test\n::shell echo hello\n").unwrap();

    // Write a whitelist
    let whitelist_path = temp_dir.path().join(".darkmatter-shell-whitelist");
    std::fs::write(&whitelist_path, "prefix echo\n").unwrap();

    md_cmd()
        .arg("compose")
        .arg(&md_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("hello"));
}

#[test]
fn test_compose_with_blacklisted_command_fails() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let md_path = temp_dir.path().join("test.md");
    std::fs::write(&md_path, "# Test\n::shell rm -rf /\n").unwrap();

    md_cmd()
        .arg("compose")
        .arg(&md_path)
        .assert()
        .failure()
        .stderr(predicate::str::contains("Blacklisted").or(predicate::str::contains("dangerous")));
}

#[test]
fn test_compose_stdin_unapproved_command_fails_with_guidance() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let whitelist_path = temp_dir.path().join(".darkmatter-shell-whitelist");

    md_cmd()
        .current_dir(temp_dir.path())
        .env("HOME", temp_dir.path())
        .arg("compose")
        .arg("-")
        .write_stdin("# Test\n::shell echo hello\n")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "Approval required for 'echo hello'.",
        ))
        .stderr(predicate::str::contains(
            "To allow in non-interactive mode, add one of these to",
        ))
        .stderr(predicate::str::contains(
            whitelist_path.display().to_string(),
        ))
        .stderr(predicate::str::contains("exact echo hello"))
        .stderr(predicate::str::contains("prefix echo"));
}

#[test]
fn test_compose_file_unapproved_command_fails_with_guidance() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let md_path = temp_dir.path().join("test.md");
    let whitelist_path = temp_dir.path().join(".darkmatter-shell-whitelist");

    std::fs::write(&md_path, "# Test\n::shell echo hello\n").unwrap();

    md_cmd()
        .current_dir(temp_dir.path())
        .arg("compose")
        .arg(&md_path)
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "Approval required for 'echo hello'.",
        ))
        .stderr(predicate::str::contains(
            "To allow in non-interactive mode, add one of these to",
        ))
        .stderr(predicate::str::contains(
            whitelist_path.display().to_string(),
        ))
        .stderr(predicate::str::contains("exact echo hello"))
        .stderr(predicate::str::contains("prefix echo"));
}

#[test]
fn test_compose_with_nonexistent_command_fails() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let md_path = temp_dir.path().join("test.md");
    std::fs::write(&md_path, "# Test\n::shell nonexistent_command_xyz\n").unwrap();

    // Write whitelist to approve the command
    let whitelist_path = temp_dir.path().join(".darkmatter-shell-whitelist");
    std::fs::write(&whitelist_path, "prefix nonexistent_command_xyz\n").unwrap();

    md_cmd()
        .arg("compose")
        .arg(&md_path)
        .assert()
        .failure()
        .stderr(predicate::str::contains("Command not found"));
}

#[test]
fn test_compose_timeout_flag_fails_timed_out_shell() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let md_path = temp_dir.path().join("test.md");
    let whitelist_path = temp_dir.path().join(".darkmatter-shell-whitelist");

    std::fs::write(&md_path, "# Test\n::shell sleep 2\n").unwrap();
    std::fs::write(&whitelist_path, "prefix sleep\n").unwrap();

    md_cmd()
        .arg("compose")
        .arg(&md_path)
        .arg("--timeout")
        .arg("1")
        .assert()
        .failure()
        .stderr(predicate::str::contains("timed out"));
}

#[test]
fn test_compose_allow_shell_timeout_emits_warning() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let md_path = temp_dir.path().join("test.md");
    let whitelist_path = temp_dir.path().join(".darkmatter-shell-whitelist");

    std::fs::write(&md_path, "# Test\n::shell sleep 2\nAfter\n").unwrap();
    std::fs::write(&whitelist_path, "prefix sleep\n").unwrap();

    md_cmd()
        .arg("compose")
        .arg(&md_path)
        .arg("--timeout")
        .arg("1")
        .arg("--allow-shell-timeout")
        .assert()
        .success()
        .stdout(predicate::str::contains("# Test"))
        .stdout(predicate::str::contains("After"))
        .stderr(predicate::str::contains("timed out"))
        .stderr(predicate::str::contains("replaced with an empty"));
}

// =============================================================================
//                     VALIDATE REFS CLI TESTS (rec #15)
// =============================================================================

#[test]
fn validate_refs_text_output() {
    let dir = tempfile::TempDir::new().unwrap();
    let md_path = dir.path().join("test.md");
    std::fs::write(&md_path, "# Heading\n\n[link](https://example.com)\n").unwrap();

    md_cmd()
        .arg("validate")
        .arg("refs")
        .arg(&md_path)
        .assert()
        .success();
}

#[test]
fn validate_refs_json_output() {
    let dir = tempfile::TempDir::new().unwrap();
    let md_path = dir.path().join("test.md");
    std::fs::write(&md_path, "[link](https://example.com)\n").unwrap();

    let output = md_cmd()
        .arg("validate")
        .arg("refs")
        .arg(&md_path)
        .arg("--format")
        .arg("json")
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // JSON output should be parseable
    let _: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap_or_else(|e| {
        panic!("Expected valid JSON, got error: {e}\nOutput: {stdout}");
    });
}

#[test]
fn validate_refs_nonzero_exit_on_errors() {
    let dir = tempfile::TempDir::new().unwrap();
    let md_path = dir.path().join("test.md");
    std::fs::write(&md_path, "[broken](./nonexistent.md)\n").unwrap();

    md_cmd()
        .arg("validate")
        .arg("refs")
        .arg(&md_path)
        .assert()
        .failure();
}

#[test]
fn validate_refs_with_fragments() {
    let dir = tempfile::TempDir::new().unwrap();
    let md_path = dir.path().join("test.md");
    std::fs::write(&md_path, "# Hello\n\n[link](#hello)\n").unwrap();

    md_cmd()
        .arg("validate")
        .arg("refs")
        .arg(&md_path)
        .arg("--fragments")
        .assert()
        .success();
}

#[test]
fn validate_refs_graph_mermaid() {
    let dir = tempfile::TempDir::new().unwrap();
    let md_path = dir.path().join("test.md");
    std::fs::write(&md_path, "# Test\n\n[link](https://example.com)\n").unwrap();

    let output = md_cmd()
        .arg("validate")
        .arg("refs")
        .arg(&md_path)
        .arg("--graph")
        .arg("mermaid")
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("flowchart TD"),
        "Expected mermaid flowchart output, got: {stdout}"
    );
}

#[test]
fn validate_refs_graph_dot() {
    let dir = tempfile::TempDir::new().unwrap();
    let md_path = dir.path().join("test.md");
    std::fs::write(&md_path, "# Test\n\n[link](https://example.com)\n").unwrap();

    let output = md_cmd()
        .arg("validate")
        .arg("refs")
        .arg(&md_path)
        .arg("--graph")
        .arg("dot")
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("digraph"),
        "Expected dot graph output, got: {stdout}"
    );
}

// =============================================================================
//                          GRAPH COMMAND TESTS
// =============================================================================

#[test]
fn test_graph_basic() {
    let mut tmp = tempfile::NamedTempFile::new().unwrap();
    writeln!(
        tmp,
        "# Test\n\n[link](https://example.com)\n\n![img](./logo.png)"
    )
    .unwrap();

    md_cmd()
        .arg("graph")
        .arg(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("example.com"))
        .stdout(predicate::str::contains("logo.png"));
}

#[test]
fn test_graph_follow() {
    let dir = tempfile::TempDir::new().unwrap();
    let parent = dir.path().join("parent.md");
    let child = dir.path().join("child.md");
    std::fs::write(&parent, "# Parent\n\n::file child.md").unwrap();
    std::fs::write(&child, "# Child\n\n[link](https://child.example.com)").unwrap();

    md_cmd()
        .arg("graph")
        .arg(&parent)
        .arg("--follow")
        .assert()
        .success()
        .stdout(predicate::str::contains("parent.md"))
        .stdout(predicate::str::contains("child.md"));
}

#[test]
fn test_graph_validate_valid() {
    let dir = tempfile::TempDir::new().unwrap();
    let md_path = dir.path().join("valid.md");
    let linked = dir.path().join("linked.md");
    std::fs::write(&md_path, "# Valid\n\n[link](./linked.md)").unwrap();
    std::fs::write(&linked, "# Linked").unwrap();

    md_cmd()
        .arg("graph")
        .arg(&md_path)
        .arg("--validate")
        .assert()
        .success()
        .stdout(predicate::str::contains("valid"))
        .stdout(predicate::str::contains("0 issues"));
}

#[test]
fn test_graph_validate_invalid() {
    let tmp = md_file("# Test\n\n[broken](./nonexistent.md)\n");

    let output = md_cmd()
        .arg("graph")
        .arg(tmp.path())
        .arg("--validate")
        .output()
        .unwrap();

    assert_eq!(
        output.status.code(),
        Some(2),
        "expected exit code 2 for validation errors"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("[missing]"),
        "expected [missing] suffix in output"
    );
    assert!(
        stdout.contains("1 issues"),
        "expected issue count in summary"
    );
}

#[test]
fn test_graph_follow_toc_linking() {
    let dir = tempfile::TempDir::new().unwrap();
    let parent = dir.path().join("root.md");
    let child = dir.path().join("child.md");
    std::fs::write(&parent, "# Root\n\n::toc-linking child.md").unwrap();
    std::fs::write(
        &child,
        "# Child\n\n## Section A\n\n## Section B\n\n[link](https://child.example.com)",
    )
    .unwrap();

    md_cmd()
        .arg("graph")
        .arg(&parent)
        .arg("--follow")
        .assert()
        .success()
        .stdout(predicate::str::contains("root.md"))
        .stdout(predicate::str::contains("child.md"))
        .stdout(predicate::str::contains("child.example.com"));
}

#[test]
fn test_graph_follow_validate_child_broken_link() {
    let dir = tempfile::TempDir::new().unwrap();
    let parent = dir.path().join("root.md");
    let child = dir.path().join("child.md");
    std::fs::write(&parent, "# Root\n\n::toc-linking child.md").unwrap();
    std::fs::write(&child, "# Child\n\n[broken](./missing.md)").unwrap();

    let output = md_cmd()
        .arg("graph")
        .arg(&parent)
        .arg("--follow")
        .arg("--validate")
        .output()
        .unwrap();

    assert_eq!(
        output.status.code(),
        Some(2),
        "expected exit code 2 when followed child has a broken link"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("[missing]"),
        "expected [missing] suffix for broken link in child"
    );
}

#[test]
fn test_graph_follow_multiple_prologues() {
    let dir = tempfile::TempDir::new().unwrap();
    let root = dir.path().join("root.md");
    let a = dir.path().join("a.md");
    let b = dir.path().join("b.md");
    std::fs::write(&root, "---\nprologue:\n  - a.md\n  - b.md\n---\n\n# Root").unwrap();
    std::fs::write(&a, "# A\n\n[a-link](https://a.example.com)").unwrap();
    std::fs::write(&b, "# B\n\n[b-link](https://b.example.com)").unwrap();

    md_cmd()
        .arg("graph")
        .arg(&root)
        .arg("--follow")
        .assert()
        .success()
        .stdout(predicate::str::contains("a.md"))
        .stdout(predicate::str::contains("b.md"))
        .stdout(predicate::str::contains("a.example.com"))
        .stdout(predicate::str::contains("b.example.com"));
}

#[test]
fn test_graph_follow_epilogue() {
    let dir = tempfile::TempDir::new().unwrap();
    let root = dir.path().join("root.md");
    let epilogue = dir.path().join("epilogue.md");
    std::fs::write(
        &root,
        "---\nepilogue: epilogue.md\n---\n\n# Root\n\n[main](https://main.example.com)",
    )
    .unwrap();
    std::fs::write(
        &epilogue,
        "# Epilogue\n\n[epi-link](https://epilogue.example.com)",
    )
    .unwrap();

    md_cmd()
        .arg("graph")
        .arg(&root)
        .arg("--follow")
        .assert()
        .success()
        .stdout(predicate::str::contains("root.md"))
        .stdout(predicate::str::contains("epilogue.md"))
        .stdout(predicate::str::contains("epilogue.example.com"));
}

#[test]
fn test_graph_file_not_found() {
    md_cmd()
        .arg("graph")
        .arg("/nonexistent/file.md")
        .assert()
        .failure();
}

#[test]
fn test_graph_help() {
    md_cmd()
        .arg("graph")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--follow"))
        .stdout(predicate::str::contains("--validate"));
}

// =============================================================================
//                          UNTESTED FLAG COVERAGE
// =============================================================================

#[test]
fn test_list_themes() {
    md_cmd()
        .arg("--list-themes")
        .assert()
        .success()
        .stdout(predicate::str::contains("Available themes"))
        .stdout(predicate::str::contains("github"))
        .stdout(predicate::str::contains("solarized"));
}

#[test]
fn test_completions_bash() {
    md_cmd()
        .args(["--completions", "bash"])
        .assert()
        .success()
        .stdout(predicate::str::is_empty().not());
}

#[test]
fn test_completions_zsh() {
    md_cmd()
        .args(["--completions", "zsh"])
        .assert()
        .success()
        .stdout(predicate::str::is_empty().not());
}

#[test]
fn test_completions_fish() {
    md_cmd()
        .args(["--completions", "fish"])
        .assert()
        .success()
        .stdout(predicate::str::is_empty().not());
}

#[test]
fn test_line_numbers_html_output() {
    let input = "```rust\nfn main() {}\n```";
    md_cmd()
        .args(["--output", "html", "--line-numbers", "-"])
        .write_stdin(input)
        .assert()
        .success()
        .stdout(predicate::str::contains("<style>"))
        .stdout(predicate::str::contains("main"));
}

#[test]
fn test_compose_compact() {
    let input = "---\n---\n\n- item 1\n\n- item 2\n\n- item 3";
    md_cmd()
        .args(["compose", "--compact", "-"])
        .write_stdin(input)
        .assert()
        .success()
        .stdout(predicate::str::contains("- item 1\n- item 2\n- item 3"));
}

#[test]
fn test_compose_loose() {
    let input = "---\n---\n\n- item 1\n- item 2\n- item 3";
    md_cmd()
        .args(["compose", "--loose", "-"])
        .write_stdin(input)
        .assert()
        .success()
        .stdout(predicate::str::contains("- item 1\n\n- item 2\n\n- item 3"));
}

#[test]
fn test_graph_json_output() {
    let dir = tempfile::TempDir::new().unwrap();
    let file = dir.path().join("test.md");
    std::fs::write(&file, "# Test\n\n[link](https://example.com)").unwrap();

    md_cmd()
        .args(["graph", "--json"])
        .arg(&file)
        .assert()
        .success()
        .stdout(predicate::str::contains("{"))
        .stdout(predicate::str::contains("\"references\""))
        .stdout(predicate::str::contains("example.com"));
}

// =============================================================================
//                  BLOCK RENDERING END-TO-END TESTS
// =============================================================================

/// End-to-end test for the CLI block rendering path.
///
/// Creates a transclusion cycle (A includes B, B includes A), runs
/// `md compose`, and asserts that the CLI:
/// - exits with a non-zero code,
/// - emits the error type name (`TransclusionError`) on stderr,
/// - emits a human-readable summary (`cycle detected`),
/// - emits a hint-tagged token from the rendered block.
#[test]
fn test_block_rendering_transclusion_cycle_tty() {
    let dir = tempfile::TempDir::new().unwrap();
    let a = dir.path().join("a.md");
    let b = dir.path().join("b.md");
    std::fs::write(&a, "# A\n\n::file b.md\n").unwrap();
    std::fs::write(&b, "# B\n\n::file a.md\n").unwrap();

    md_cmd()
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
    let dir = tempfile::TempDir::new().unwrap();
    let a = dir.path().join("a.md");
    let b = dir.path().join("b.md");
    std::fs::write(&a, "# A\n\n::file b.md\n").unwrap();
    std::fs::write(&b, "# B\n\n::file a.md\n").unwrap();

    let output = md_cmd().arg("compose").arg(&a).output().unwrap();

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
