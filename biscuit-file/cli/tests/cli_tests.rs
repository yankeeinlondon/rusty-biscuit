use assert_cmd::Command;
use predicates::prelude::*;

fn bf() -> Command {
    assert_cmd::Command::cargo_bin("bf").unwrap()
}

fn fixture(name: &str) -> String {
    format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"))
}

/// Runs a successful `bf` invocation and returns its stdout without the
/// trailing newline.
///
/// The reference assertions below need the value itself, not a substring
/// predicate: whether a path is absolute is a question for `std::path`, and the
/// `starts_with("/")` these tests used to spell it with is a claim only a
/// Unix-rooted path can satisfy.
fn stdout_line(args: &[&str]) -> String {
    let output = bf().args(args).output().unwrap();
    assert!(
        output.status.success(),
        "`bf {}` failed: {}",
        args.join(" "),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .expect("bf writes UTF-8")
        .trim_end()
        .to_string()
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

// ── JSON5 format conversion ─────────────────────────────────────────

#[test]
fn json5_to_json_default() {
    bf().arg(fixture("sample.json5"))
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""name": "example""#));
}

#[test]
fn json5_to_yaml() {
    bf().arg(fixture("sample.json5"))
        .arg("--yaml")
        .assert()
        .success()
        .stdout(predicate::str::contains("name: example"));
}

#[test]
fn json5_to_toml() {
    bf().arg(fixture("sample.json5"))
        .arg("--toml")
        .assert()
        .success()
        .stdout(predicate::str::contains("name = \"example\""));
}

#[test]
fn json5_to_json5() {
    bf().arg(fixture("sample.json5"))
        .arg("--json5")
        .assert()
        .success()
        .stdout(predicate::str::contains("example"));
}

#[test]
fn json_to_json5() {
    bf().arg(fixture("sample.json"))
        .arg("--json5")
        .assert()
        .success()
        .stdout(predicate::str::contains("example"));
}

#[test]
fn toml_to_json5() {
    bf().arg(fixture("sample.toml"))
        .arg("--json5")
        .assert()
        .success()
        .stdout(predicate::str::contains("example"));
}

#[test]
fn yaml_to_json5() {
    bf().arg(fixture("sample.yaml"))
        .arg("--json5")
        .assert()
        .success()
        .stdout(predicate::str::contains("example"));
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
        .stderr(predicate::str::contains(
            "--text and --md are only supported for PDF files",
        ));
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
fn stdin_json5_to_json() {
    bf().arg("--input-format")
        .arg("json5")
        .arg("--json")
        .write_stdin("{ greeting: 'hello' }")
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""greeting": "hello""#));
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

// ── Compact output ──────────────────────────────────────────────────

#[test]
fn json_compact_single_line() {
    bf().arg(fixture("sample.json"))
        .arg("--json")
        .arg("--compact")
        .assert()
        .success()
        .stdout(predicate::str::starts_with("{\""))
        .stdout(predicate::function(|s: &str| s.trim().lines().count() == 1));
}

#[test]
fn json5_compact_unquoted_keys() {
    bf().arg(fixture("sample.json"))
        .arg("--json5")
        .arg("--compact")
        .assert()
        .success()
        .stdout(predicate::str::contains("name: 'example'"))
        .stdout(predicate::function(|s: &str| s.trim().lines().count() == 1));
}

#[test]
fn json5_input_compact_json_output() {
    bf().arg(fixture("sample.json5"))
        .arg("--json")
        .arg("--compact")
        .assert()
        .success()
        .stdout(predicate::function(|s: &str| s.trim().lines().count() == 1));
}

#[test]
fn json5_input_compact_json5_output() {
    bf().arg(fixture("sample.json5"))
        .arg("--json5")
        .arg("--compact")
        .assert()
        .success()
        .stdout(predicate::str::contains("name: 'example'"))
        .stdout(predicate::function(|s: &str| s.trim().lines().count() == 1));
}

#[test]
fn toml_to_json_compact() {
    bf().arg(fixture("sample.toml"))
        .arg("--json")
        .arg("--compact")
        .assert()
        .success()
        .stdout(predicate::function(|s: &str| s.trim().lines().count() == 1));
}

#[test]
fn yaml_to_json5_compact() {
    bf().arg(fixture("sample.yaml"))
        .arg("--json5")
        .arg("--compact")
        .assert()
        .success()
        .stdout(predicate::str::contains("name: 'example'"))
        .stdout(predicate::function(|s: &str| s.trim().lines().count() == 1));
}

#[test]
fn default_json_is_pretty_not_compact() {
    bf().arg(fixture("sample.json"))
        .assert()
        .success()
        .stdout(predicate::function(|s: &str| s.trim().lines().count() > 1));
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

// ── File reference resolution ────────────────────────────────────────

/// The resolved path is printed as portable text, so the `/`-separated
/// expectations below hold on every host rather than only on Unix.
#[test]
fn reference_resolves_relative_path_to_absolute() {
    let resolved = stdout_line(&["reference", "./Cargo.toml"]);
    assert!(
        resolved.ends_with("biscuit-file/cli/Cargo.toml"),
        "expected a portable spelling, got {resolved:?}"
    );
    assert!(
        std::path::Path::new(&resolved).is_absolute(),
        "expected an absolute path, got {resolved:?}"
    );
}

#[test]
fn reference_alias_ref_works() {
    let resolved = stdout_line(&["ref", "./Cargo.toml"]);
    assert!(
        resolved.ends_with("biscuit-file/cli/Cargo.toml"),
        "expected a portable spelling, got {resolved:?}"
    );
}

#[test]
fn reference_nonexistent_exits_1() {
    bf().arg("reference")
        .arg("./nonexistent-file-that-does-not-exist.md")
        .assert()
        .code(1);
}

#[test]
fn reference_relative_cwd() {
    bf().arg("reference")
        .arg("--relative-cwd")
        .arg("./Cargo.toml")
        .assert()
        .success()
        .stdout(predicate::str::is_match("^Cargo\\.toml\n$").unwrap());
}

#[test]
fn reference_vault_no_roots_exits_2() {
    bf().arg("reference")
        .arg("vault:note.md")
        .assert()
        .code(2)
        .stderr(predicate::str::contains("vault"));
}

#[test]
fn reference_add_vault_searches_vault() {
    let dir = std::env::temp_dir().join("bf-test-vault");
    let _ = std::fs::create_dir_all(&dir);
    let note_path = dir.join("note.md");
    std::fs::write(&note_path, "# Test").unwrap();

    bf().arg("reference")
        .arg("--add-vault")
        .arg(dir.to_str().unwrap())
        .arg("vault:note.md")
        .assert()
        .success()
        .stdout(predicate::str::contains("note.md"));

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn reference_implicit_relative_bare_filename() {
    // Bare filename (no `./` prefix) is ImplicitRelative. Phase 4 resolves it
    // repository-root first: `Cargo.toml` exists both in the CLI crate and at
    // the workspace root, so the workspace-root copy wins over the source-local
    // one, end-to-end through the binary.
    let resolved = stdout_line(&["reference", "Cargo.toml"]);
    assert!(
        !resolved.ends_with("biscuit-file/cli/Cargo.toml"),
        "the workspace-root copy should win, got {resolved:?}"
    );
    assert!(
        resolved.ends_with("/Cargo.toml"),
        "expected a portable spelling, got {resolved:?}"
    );
    assert!(
        std::path::Path::new(&resolved).is_absolute(),
        "expected an absolute path, got {resolved:?}"
    );
}

#[test]
fn reference_implicit_relative_falls_back_to_git_root() {
    // `CLAUDE.md` lives at the repo root, not inside `biscuit-file/cli`.
    // Implicit relative resolution should fall back to the git root.
    let resolved = stdout_line(&["reference", "CLAUDE.md"]);
    assert!(
        resolved.ends_with("/CLAUDE.md"),
        "expected the repo-root copy in a portable spelling, got {resolved:?}"
    );
    assert!(
        std::path::Path::new(&resolved).is_absolute(),
        "expected an absolute path, got {resolved:?}"
    );
}

#[test]
fn reference_invalid_syntax_exits_2() {
    bf().arg("reference")
        .arg("{{invalid-name}}/foo.md")
        .assert()
        .code(2)
        .stderr(predicate::str::contains("invalid variable name"));
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

// ── Debug flag ──────────────────────────────────────────────────────

#[test]
fn debug_flag_produces_stderr_output() {
    bf().arg("--debug")
        .arg(fixture("sample.toml"))
        .assert()
        .success()
        .stderr(
            predicate::str::contains("processing input")
                .or(predicate::str::contains("biscuit_file")),
        );
}
