//! Real CLI regression coverage for launch-context anchoring.

use std::fs;
use std::path::{Path, PathBuf};

use pulldown_cmark::{Event, Options, Parser, TagEnd};
use rstest::rstest;

mod common;
use common::{TestWorkspace, augmented_path, strip_ansi, write};
#[cfg(unix)]
use common::write_executable;

/// Real directory the launch repository is created beneath.
///
/// The leading `.` is the point of the fixture: it puts a path separator
/// immediately before a `.` in every composed `ctx.repo_root`, which on Windows
/// is the `\.` sequence CommonMark consumes as an escape unless interpolation
/// is literal. The backtick pair earns the same coverage on Unix, where the
/// separator is `/` and nothing else in a temporary path is Markdown syntax —
/// without it a lost literal-interpolation guard could only be caught on
/// Windows.
const HIDDEN_LAUNCH_PARENT: &str = ".tmp`ZZZ`";

#[derive(Debug)]
struct Observation {
    /// Markdown bytes exactly as the provider received them.
    prompt: String,
    /// `prompt` parsed back to its literal text.
    ///
    /// An interpolated scalar is serialized with whatever escaping its literal
    /// text requires, so the provider's bytes and the value they denote are two
    /// different strings. Value assertions read this; spelling assertions read
    /// `prompt`.
    prompt_text: String,
    stderr: String,
}

impl Observation {
    fn new(prompt: String, stderr: String) -> Self {
        let prompt_text = parsed_text(&prompt);
        Self {
            prompt,
            prompt_text,
            stderr,
        }
    }
}

/// Path of the launch repository for a fixture rooted at `workspace_root`.
///
/// Fails loudly rather than degrading quietly: a fixture that stopped crossing
/// the separator-then-`.` boundary would still pass every assertion below while
/// no longer reproducing the regression.
fn launch_repository(workspace_root: &Path) -> PathBuf {
    let repo = workspace_root.join(HIDDEN_LAUNCH_PARENT).join("launch-repo");
    let boundary = format!("{}{HIDDEN_LAUNCH_PARENT}", std::path::MAIN_SEPARATOR);
    assert!(
        repo.to_string_lossy().contains(boundary.as_str()),
        "the launch repository must sit behind a `{boundary}` boundary: {}",
        repo.display()
    );
    repo
}

/// Literal text of a composed Markdown prompt.
///
/// Inline code contributes its content without its delimiters, so a value that
/// escaped into a code span is reported as the text it is *not*, rather than
/// being reassembled back into a passing comparison.
fn parsed_text(markdown: &str) -> String {
    let mut text = String::new();
    for event in Parser::new_ext(markdown, Options::all() - Options::ENABLE_SMART_PUNCTUATION) {
        match event {
            Event::Text(chunk) => text.push_str(&chunk),
            Event::Code(code) => text.push_str(&code),
            Event::SoftBreak | Event::HardBreak | Event::End(TagEnd::Paragraph) => {
                text.push('\n');
            }
            _ => {}
        }
    }
    text
}

/// CommonMark source spelling of a scalar that must survive as literal text.
///
/// Derived from the value so one expectation covers every platform: on Windows
/// each separator doubles, on Unix only the fixture's backtick pair is escaped.
fn commonmark_source_spelling(value: &str) -> String {
    value.replace('\\', r"\\").replace('`', r"\`")
}

fn init_repo(root: &Path) {
    fs::create_dir_all(root).unwrap();
    let output = std::process::Command::new("git")
        .args(["init", "--quiet", "--initial-branch=main"])
        .current_dir(root)
        .output()
        .expect("initialize Git fixture");
    assert!(
        output.status.success(),
        "git init failed at {}: {}",
        root.display(),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn write_monorepo(root: &Path) -> (PathBuf, PathBuf) {
    init_repo(root);
    write(
        &root.join("Cargo.toml"),
        "[workspace]\nresolver = \"2\"\nmembers = [\"alpha/lib\", \"beta/lib\"]\n",
    );
    for (area, package) in [("alpha", "alpha-lib"), ("beta", "beta-lib")] {
        write(
            &root.join(area).join("lib/Cargo.toml"),
            &format!(
                "[package]\nname = \"{package}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n"
            ),
        );
        write(&root.join(area).join("lib/src/lib.rs"), "");
    }
    (root.join("alpha/lib"), root.join("beta/lib"))
}

fn baseline_document(expected_area: Option<&str>, expected_repo: Option<&Path>) -> String {
    let area_condition = expected_area.map_or_else(
        || "!ctx.area".to_string(),
        |area| format!("ctx.area == {area:?}"),
    );
    let repo = expected_repo.map(|path| path.display().to_string());
    let repo_json = serde_json::to_string(&repo).unwrap();
    let repo_condition = expected_repo.map_or_else(
        || "!ctx.repo_root".to_string(),
        |_| "ctx.repo_root == expected_launch_repo".to_string(),
    );
    let condition = format!("{area_condition} && {repo_condition}");

    format!(
        r#"---
title: ctx launch anchor baseline
expected_launch_repo: {repo_json}
frontmatter_area: "{{{{ ctx.area }}}}"
frontmatter_repo: "{{{{ ctx.repo_root }}}}"
preflight_area: "$(printf '{{{{ ctx.area }}}}')"
success:
  warn: "baseline.lifecycle.area=[{{{{ ctx.area }}}}] baseline.lifecycle.repo=[{{{{ ctx.repo_root }}}}]"
  stack:
    - when: {condition:?}
      action: {{warn: "baseline.when.launch-context=true area=[{{{{ ctx.area }}}}] baseline.when.repo=[{{{{ ctx.repo_root }}}}]"}}
---
baseline.body.area=[{{{{ ctx.area }}}}] baseline.body.repo=[{{{{ ctx.repo_root }}}}]
baseline.frontmatter.area=[{{{{ frontmatter_area }}}}] baseline.frontmatter.repo=[{{{{ frontmatter_repo }}}}]
baseline.preflight.area=[{{{{ preflight_area }}}}]
"#
    )
}

#[cfg(unix)]
fn stage_codex(bin_dir: &Path) {
    fs::create_dir_all(bin_dir).unwrap();
    write_executable(
        &bin_dir.join("codex"),
        r#"#!/bin/sh
/bin/cat > "$CLAUDINE_STDIN_FILE"
exit 0
"#,
    );
}

#[cfg(windows)]
fn stage_codex(bin_dir: &Path) {
    stage_windows_provider(bin_dir, "codex");
}

#[cfg(unix)]
fn stage_goose(bin_dir: &Path) {
    fs::create_dir_all(bin_dir).unwrap();
    write_executable(
        &bin_dir.join("goose"),
        r#"#!/bin/sh
for arg in "$@"; do
  printf '%s\n' "$arg" >> "$CLAUDINE_STDIN_FILE"
done
/bin/cat >> "$CLAUDINE_STDIN_FILE"
printf 'Generated body\n'
exit 0
"#,
    );
}

#[cfg(windows)]
fn stage_goose(bin_dir: &Path) {
    stage_windows_provider(bin_dir, "goose");
}

#[cfg(unix)]
fn stage_counting_codex(bin_dir: &Path) {
    fs::create_dir_all(bin_dir).unwrap();
    write_executable(
        &bin_dir.join("codex"),
        r#"#!/bin/sh
count=0
if [ -f "$CLAUDINE_COUNT_FILE" ]; then
  IFS= read -r count < "$CLAUDINE_COUNT_FILE"
fi
count=$((count + 1))
printf '%s' "$count" > "$CLAUDINE_COUNT_FILE"
/bin/cat > "$CLAUDINE_STDIN_FILE"
exit 0
"#,
    );
}

#[cfg(windows)]
fn stage_counting_codex(bin_dir: &Path) {
    stage_windows_provider(bin_dir, "codex");
}

#[cfg(windows)]
fn stage_printf(bin_dir: &Path) {
    write(
        &bin_dir.join("printf.cmd"),
        "@echo off\r\n<nul set /p \"=%~1\"\r\nexit /b 0\r\n",
    );
}

#[cfg(windows)]
fn provider_fixture_executable() -> PathBuf {
    if let Some(path) = std::env::var_os("CLAUDINE_PROVIDER_FIXTURE_EXE").map(PathBuf::from) {
        assert!(
            path.is_file(),
            "CLAUDINE_PROVIDER_FIXTURE_EXE does not name a file: {}",
            path.display()
        );
        return path;
    }

    let current = std::env::current_exe().expect("resolve current nextest executable");
    let mut profile_dir = current
        .parent()
        .expect("nextest executable must have a parent")
        .to_path_buf();
    if profile_dir.file_name().and_then(|name| name.to_str()) == Some("deps") {
        profile_dir.pop();
    }
    let fixture = profile_dir.join("examples").join(format!(
        "ctx_launch_anchor_provider_fixture{}",
        std::env::consts::EXE_SUFFIX
    ));
    assert!(
        fixture.is_file(),
        "native provider fixture is missing at {}; run `cd claudine && just test-cli`",
        fixture.display()
    );
    fixture
}

#[cfg(windows)]
fn stage_windows_provider(bin_dir: &Path, provider: &str) {
    fs::create_dir_all(bin_dir).unwrap();
    fs::copy(
        provider_fixture_executable(),
        bin_dir.join(format!("{provider}{}", std::env::consts::EXE_SUFFIX)),
    )
    .expect("stage native provider fixture");
    stage_printf(bin_dir);
}

#[cfg(windows)]
#[test]
fn staged_printf_returns_expected_stdout_with_zero_status() {
    let workspace = TestWorkspace::named("ctx-launch-anchor-printf");
    let bin_dir = workspace.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();
    stage_printf(&bin_dir);

    let output = std::process::Command::new(bin_dir.join("printf.cmd"))
        .arg("alpha-lib")
        .output()
        .expect("run staged printf.cmd");

    assert_eq!(output.stdout, b"alpha-lib");
    assert!(
        output.status.success(),
        "printf.cmd returned {:?}: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[cfg(windows)]
#[test]
fn native_provider_fixture_preserves_argv_and_stdin_without_a_runtime_toolchain() {
    let workspace = TestWorkspace::named("ctx-launch-anchor-native-provider");
    let fixture = provider_fixture_executable();
    let empty_path = workspace.path().join("runtime-path");
    fs::create_dir_all(&empty_path).unwrap();

    // The toolchain-free contract is proved by handing every spawn below a PATH
    // containing nothing, not by probing for `cargo` here. Windows resolves a
    // child's program name against the PARENT's PATH, so `Command::new("cargo")
    // .env("PATH", empty)` still finds an installed cargo and the probe asserts
    // the host's setup rather than the fixture's independence.
    for tool in ["cargo.exe", "rustc.exe", "cargo", "rustc"] {
        assert!(
            !empty_path.join(tool).exists(),
            "the runtime PATH handed to the fixture must expose no Rust toolchain, found {tool}"
        );
    }

    let capture = workspace.path().join("goose-capture.bin");
    let args_capture = workspace.path().join("goose-args.json");
    let provider_args = [
        "--prompt",
        "baseline.body.area=[alpha-lib]\nquoted=[\"C:\\Users\\runneradmin\\AppData\\Local\\Temp\\.tmpQP6rk1\\launch-repository\"]",
        "separate argv element",
    ];
    let goose_stdin = b"goose stdin line one\r\ngoose stdin \"quoted\"\n";
    let mut goose = std::process::Command::new(&fixture)
        .env("PATH", &empty_path)
        .env("CLAUDINE_PROVIDER_FIXTURE_MODE", "goose")
        .env("CLAUDINE_STDIN_FILE", &capture)
        .env("CLAUDINE_PROVIDER_FIXTURE_ARGS_FILE", &args_capture)
        .args(provider_args)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("run native Goose fixture");
    use std::io::Write as _;
    goose.stdin.take().unwrap().write_all(goose_stdin).unwrap();
    let goose_output = goose.wait_with_output().unwrap();

    assert!(goose_output.status.success());
    assert_eq!(goose_output.stdout, b"Generated body\n");
    let captured_args: Vec<String> =
        serde_json::from_slice(&fs::read(args_capture).unwrap()).unwrap();
    assert_eq!(captured_args, provider_args);
    let mut expected_goose_capture = provider_args.join("\n").into_bytes();
    expected_goose_capture.push(b'\n');
    expected_goose_capture.extend_from_slice(goose_stdin);
    assert_eq!(fs::read(capture).unwrap(), expected_goose_capture);

    let codex_capture = workspace.path().join("codex-stdin.bin");
    let codex_stdin = b"codex line one\r\ncodex \"quoted\"\n\0tail";
    let mut codex = std::process::Command::new(fixture)
        .env("PATH", &empty_path)
        .env("CLAUDINE_PROVIDER_FIXTURE_MODE", "codex")
        .env("CLAUDINE_STDIN_FILE", &codex_capture)
        .stdin(std::process::Stdio::piped())
        .spawn()
        .expect("run native Codex fixture");
    codex.stdin.take().unwrap().write_all(codex_stdin).unwrap();
    assert!(codex.wait().unwrap().success());
    assert_eq!(fs::read(codex_capture).unwrap(), codex_stdin);
}

fn loop_document(expected_repo: &Path) -> String {
    let repo_json = serde_json::to_string(&expected_repo.display().to_string()).unwrap();
    format!(
        r#"---
expected_launch_repo: {repo_json}
frontmatter_area: "{{{{ ctx.area }}}}"
frontmatter_repo: "{{{{ ctx.repo_root }}}}"
preflight_area: "$(printf '{{{{ ctx.area }}}}')"
phase: 1
loop:
  until: "phase > 1"
  action: "increment(phase)"
success:
  warn: "baseline.lifecycle.area=[{{{{ ctx.area }}}}] baseline.lifecycle.repo=[{{{{ ctx.repo_root }}}}]"
  stack:
    - when: "ctx.area == 'alpha-lib' && ctx.repo_root == expected_launch_repo"
      action: {{warn: "baseline.when.launch-context=true area=[{{{{ ctx.area }}}}] baseline.when.repo=[{{{{ ctx.repo_root }}}}]"}}
---
baseline.body.area=[{{{{ ctx.area }}}}] baseline.body.repo=[{{{{ ctx.repo_root }}}}]
baseline.frontmatter.area=[{{{{ frontmatter_area }}}}] baseline.frontmatter.repo=[{{{{ frontmatter_repo }}}}]
baseline.preflight.area=[{{{{ preflight_area }}}}]
"#
    )
}

fn inline_document(expected_repo: &Path) -> String {
    let repo_json = serde_json::to_string(&expected_repo.display().to_string()).unwrap();
    format!(
        r#"---
expected_launch_repo: {repo_json}
frontmatter_area: "{{{{ ctx.area }}}}"
frontmatter_repo: "{{{{ ctx.repo_root }}}}"
preflight_area: "$(printf '{{{{ ctx.area }}}}')"
prompt: |
  baseline.body.area=[{{{{ ctx.area }}}}] baseline.body.repo=[{{{{ ctx.repo_root }}}}]
  baseline.frontmatter.area=[{{{{ frontmatter_area }}}}] baseline.frontmatter.repo=[{{{{ frontmatter_repo }}}}]
  baseline.preflight.area=[{{{{ preflight_area }}}}]
success:
  warn: "baseline.lifecycle.area=[{{{{ ctx.area }}}}] baseline.lifecycle.repo=[{{{{ ctx.repo_root }}}}]"
  stack:
    - when: "ctx.area == 'alpha-lib' && ctx.repo_root == expected_launch_repo"
      action: {{warn: "baseline.when.launch-context=true area=[{{{{ ctx.area }}}}] baseline.when.repo=[{{{{ ctx.repo_root }}}}]"}}
---
Original body.
"#
    )
}

fn cli_command(launch_dir: &Path, home: &Path, bin_dir: &Path) -> assert_cmd::Command {
    let mut command = assert_cmd::Command::cargo_bin("claudine").unwrap();
    command
        .current_dir(launch_dir)
        .env("HOME", home)
        .env("USERPROFILE", home)
        .env("APPDATA", home)
        .env("LOCALAPPDATA", home)
        .env("PATH", augmented_path(bin_dir))
        .env("PATHEXT", ".COM;.EXE;.BAT;.CMD")
        .env("NO_COLOR", "1")
        .env("COLUMNS", "240")
        .env("CLAUDINE_RENDEZVOUS_REPORT", "false");
    command
}

fn run_document(
    launch_dir: &Path,
    home: &Path,
    bin_dir: &Path,
    document: &Path,
    capture: &Path,
) -> Observation {
    let assertion = cli_command(launch_dir, home, bin_dir)
        .env("CLAUDINE_STDIN_FILE", capture)
        .env("CLAUDINE_PROVIDER_FIXTURE_MODE", "codex")
        .args(["compose", "--codex"])
        .arg(document)
        .assert()
        .success();

    Observation::new(
        fs::read_to_string(capture).expect("fake Codex must receive the composed prompt"),
        strip_ansi(&String::from_utf8_lossy(&assertion.get_output().stderr)),
    )
}

fn run_inline_document(
    launch_dir: &Path,
    home: &Path,
    bin_dir: &Path,
    document: &Path,
    capture: &Path,
) -> Observation {
    let assertion = cli_command(launch_dir, home, bin_dir)
        .env("CLAUDINE_STDIN_FILE", capture)
        .env("CLAUDINE_PROVIDER_FIXTURE_MODE", "goose")
        .args(["inline-compose", "--goose"])
        .arg(document)
        .assert()
        .success();

    Observation::new(
        fs::read_to_string(capture).expect("fake Goose must receive the inline prompt"),
        strip_ansi(&String::from_utf8_lossy(&assertion.get_output().stderr)),
    )
}

fn field<'a>(text: &'a str, label: &str) -> &'a str {
    let prefix = format!("{label}=[");
    let start = text
        .find(&prefix)
        .unwrap_or_else(|| panic!("missing `{label}` marker in:\n{text}"))
        + prefix.len();
    let rest = &text[start..];
    let end = rest
        .find(']')
        .unwrap_or_else(|| panic!("unterminated `{label}` marker in:\n{text}"));
    &rest[..end]
}

fn assert_path_field(text: &str, label: &str, expected: &Path) {
    let actual = field(text, label);
    let actual = projected_path(Path::new(actual));
    let expected = projected_path(expected);
    assert_eq!(actual, expected, "wrong `{label}` path in:\n{text}");
}

fn projected_path(path: &Path) -> PathBuf {
    dunce::canonicalize(path).unwrap_or_else(|_| dunce::simplified(path).to_path_buf())
}

fn assert_launch_anchored(
    case: &str,
    observation: &Observation,
    expected_area: &str,
    expected_repo: Option<&Path>,
) {
    assert_eq!(
        field(&observation.prompt_text, "baseline.body.area"),
        expected_area,
        "wrong body area for {case}"
    );
    assert_eq!(
        field(&observation.prompt_text, "baseline.frontmatter.area"),
        expected_area
    );
    if observation.prompt_text.contains("baseline.preflight.area=[") {
        assert_eq!(
            field(&observation.prompt_text, "baseline.preflight.area"),
            expected_area,
            "preflight-expanded command bytes must use the epoch snapshot"
        );
    }
    match expected_repo {
        Some(expected_repo) => {
            assert_path_field(&observation.prompt_text, "baseline.body.repo", expected_repo);
            assert_path_field(
                &observation.prompt_text,
                "baseline.frontmatter.repo",
                expected_repo,
            );
        }
        None => {
            assert_eq!(field(&observation.prompt_text, "baseline.body.repo"), "");
            assert_eq!(
                field(&observation.prompt_text, "baseline.frontmatter.repo"),
                ""
            );
        }
    }

    assert_eq!(
        field(&observation.stderr, "baseline.lifecycle.area"),
        expected_area,
        "lifecycle interpolation must freeze the same baseline as the body"
    );
    match expected_repo {
        Some(expected_repo) => {
            assert_path_field(&observation.stderr, "baseline.lifecycle.repo", expected_repo)
        }
        None => assert_eq!(field(&observation.stderr, "baseline.lifecycle.repo"), ""),
    }
    assert!(
        observation.stderr.contains("baseline.when.launch-context=true"),
        "the launch-context `when:` branch did not run:\n{}",
        observation.stderr
    );
    assert_eq!(
        field(&observation.stderr, "baseline.when.launch-context=true area"),
        expected_area
    );
    match expected_repo {
        Some(expected_repo) => {
            assert_path_field(&observation.stderr, "baseline.when.repo", expected_repo)
        }
        None => assert_eq!(field(&observation.stderr, "baseline.when.repo"), ""),
    }
}

#[rstest]
#[case::launch_area("launch-area")]
#[case::opposing_area("opposing-area")]
#[case::external_repository("external-repo")]
#[case::repository_root("repo-root")]
fn cli_uses_launch_context_across_launch_source_matrix(#[case] source: &str) {
    let workspace = TestWorkspace::named("ctx-launch-anchor-matrix");
    // macOS spells its temporary root through both `/var` and `/private/var`.
    // Use one concrete spelling throughout this baseline so the only variable
    // under test is launch-versus-source anchoring.
    let workspace_root = projected_path(workspace.path());
    let launch_repo = launch_repository(&workspace_root);
    let (launch_area, opposing_area) = write_monorepo(&launch_repo);
    write(&launch_repo.join(".darkmatter-shell-whitelist"), "prefix printf\n");
    let external_repo = workspace_root.join("external-repo");
    init_repo(&external_repo);
    write(&external_repo.join(".darkmatter-shell-whitelist"), "prefix printf\n");

    let home = workspace_root.join("home");
    let bin_dir = workspace_root.join("bin");
    fs::create_dir_all(home.join(".claudine")).unwrap();
    write(&home.join(".claudine/config.json"), "{}");
    stage_codex(&bin_dir);

    let document = match source {
        "launch-area" => launch_area.join("prompt.md"),
        "opposing-area" => opposing_area.join("prompt.md"),
        "external-repo" => external_repo.join("prompt.md"),
        "repo-root" => launch_repo.join("prompt.md"),
        _ => panic!("unsupported launch-source fixture: {source}"),
    };

    write(
        &document,
        &baseline_document(Some("alpha-lib"), Some(&launch_repo)),
    );
    let capture = workspace_root.join(format!("{source}-prompt.txt"));
    let observation = run_document(&launch_area, &home, &bin_dir, &document, &capture);
    assert_launch_anchored(source, &observation, "alpha-lib", Some(&launch_repo));
}

#[test]
fn cli_keeps_launch_repository_facts_absent_when_source_is_in_repo() {
    let workspace = TestWorkspace::named("ctx-launch-anchor-inverse");
    let workspace_root = projected_path(workspace.path());
    let launch_dir = workspace_root.join("outside-all-repositories");
    let home = workspace_root.join("home");
    let bin_dir = workspace_root.join("bin");
    fs::create_dir_all(&launch_dir).unwrap();
    fs::create_dir_all(home.join(".claudine")).unwrap();
    write(&home.join(".claudine/config.json"), "{}");
    stage_codex(&bin_dir);

    let document = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/ctx_launch_anchor/in_repo.md");
    let capture = workspace_root.join("inverse-prompt.txt");
    let observation = run_document(&launch_dir, &home, &bin_dir, &document, &capture);

    assert_launch_anchored("inverse", &observation, "", None);
}

#[test]
fn cli_keeps_eager_schema_files_source_relative_and_ctx_launch_relative() {
    let workspace = TestWorkspace::named("ctx-launch-anchor-schema");
    let workspace_root = projected_path(workspace.path());
    let launch_repo = launch_repository(&workspace_root);
    let (launch_area, _) = write_monorepo(&launch_repo);
    let source_repo = workspace_root.join("source-repo");
    init_repo(&source_repo);

    write(&launch_repo.join("spec.md"), "LAUNCH DECOY\n");
    let source_spec = source_repo.join("spec.md");
    write(&source_spec, "SOURCE SPEC\n");
    let document = source_repo.join("prompt.md");
    write(
        &document,
        "---\n$schema:\n  spec: 'file(eager; required)'\nspec: spec.md\n---\nresolved.spec=[{{ spec }}] launch.repo=[{{ ctx.repo_root }}]\n::file {{ spec }}\n",
    );

    let home = workspace_root.join("home");
    let bin_dir = workspace_root.join("bin");
    fs::create_dir_all(home.join(".claudine")).unwrap();
    write(&home.join(".claudine/config.json"), "{}");
    stage_codex(&bin_dir);
    let capture = workspace_root.join("schema-prompt.txt");

    let observation = run_document(&launch_area, &home, &bin_dir, &document, &capture);
    assert_eq!(field(&observation.prompt_text, "resolved.spec"), "spec.md");
    assert!(observation.prompt_text.contains("SOURCE SPEC"));
    assert!(!observation.prompt.contains("LAUNCH DECOY"));
    assert_path_field(&observation.prompt_text, "launch.repo", &launch_repo);
}

#[test]
fn cli_provider_receives_exact_projected_ctx_repo_root_text() {
    let workspace = TestWorkspace::named("ctx-launch-anchor-exact-provider-path");
    let workspace_root = projected_path(workspace.path());
    let launch_repo = launch_repository(&workspace_root);
    let (launch_area, _) = write_monorepo(&launch_repo);
    let document = launch_repo.join("exact-path.md");
    write(&document, "provider.repo=[{{ ctx.repo_root }}]\n");

    let home = workspace_root.join("home");
    let bin_dir = workspace_root.join("bin");
    fs::create_dir_all(home.join(".claudine")).unwrap();
    write(&home.join(".claudine/config.json"), "{}");
    stage_codex(&bin_dir);
    let capture = workspace_root.join("exact-path-prompt.txt");

    let observation = run_document(&launch_area, &home, &bin_dir, &document, &capture);
    let expected = projected_path(&launch_repo)
        .to_string_lossy()
        .into_owned();

    assert_eq!(
        field(&observation.prompt_text, "provider.repo"),
        expected,
        "the normal CLI/provider path must deliver ctx.repo_root as exact literal text; \
         provider received:\n{}",
        observation.prompt
    );
    assert_eq!(
        field(&observation.prompt, "provider.repo"),
        commonmark_source_spelling(&expected),
        "the serialized source must escape exactly what CommonMark would otherwise consume"
    );
    assert!(
        !observation.prompt.contains(r"\\?\"),
        "provider prompt leaked a Windows verbatim path: {}",
        observation.prompt
    );
}

#[rstest]
#[case::launch_area("launch-area")]
#[case::opposing_area("opposing-area")]
#[case::external_repository("external-repo")]
#[case::repository_root("repo-root")]
fn inline_cli_uses_launch_context_across_launch_source_matrix(#[case] source: &str) {
    let workspace = TestWorkspace::named("ctx-launch-anchor-inline-matrix");
    let workspace_root = projected_path(workspace.path());
    let launch_repo = launch_repository(&workspace_root);
    let (launch_area, opposing_area) = write_monorepo(&launch_repo);
    write(&launch_repo.join(".darkmatter-shell-whitelist"), "prefix printf\n");
    let external_repo = workspace_root.join("external-repo");
    init_repo(&external_repo);
    write(&external_repo.join(".darkmatter-shell-whitelist"), "prefix printf\n");

    let home = workspace_root.join("home");
    let bin_dir = workspace_root.join("bin");
    fs::create_dir_all(home.join(".claudine")).unwrap();
    write(&home.join(".claudine/config.json"), "{}");
    stage_goose(&bin_dir);

    let document = match source {
        "launch-area" => launch_area.join("inline.md"),
        "opposing-area" => opposing_area.join("inline.md"),
        "repo-root" => launch_repo.join("inline.md"),
        "external-repo" => external_repo.join("inline.md"),
        _ => panic!("unsupported inline launch-source fixture: {source}"),
    };

    write(&document, &inline_document(&launch_repo));
    let capture = workspace_root.join(format!("inline-{source}-prompt.txt"));
    let observation = run_inline_document(&launch_area, &home, &bin_dir, &document, &capture);
    assert_launch_anchored(source, &observation, "alpha-lib", Some(&launch_repo));
}

#[rstest]
#[case::repository_root("repo-root")]
#[case::package_area("launch-area")]
fn cli_loop_reuses_launch_context_for_root_and_package_prompt_copies(#[case] source: &str) {
    let workspace = TestWorkspace::named("ctx-launch-anchor-loop-pair");
    let workspace_root = projected_path(workspace.path());
    let launch_repo = launch_repository(&workspace_root);
    let (launch_area, _) = write_monorepo(&launch_repo);
    write(&launch_repo.join(".darkmatter-shell-whitelist"), "prefix printf\n");

    let home = workspace_root.join("home");
    let bin_dir = workspace_root.join("bin");
    fs::create_dir_all(home.join(".claudine")).unwrap();
    write(&home.join(".claudine/config.json"), "{}");
    stage_counting_codex(&bin_dir);

    let document = match source {
        "repo-root" => launch_repo.join("loop.md"),
        "launch-area" => launch_area.join("loop.md"),
        _ => panic!("unsupported loop launch-source fixture: {source}"),
    };

    write(&document, &loop_document(&launch_repo));
    let capture = workspace_root.join(format!("loop-{source}-prompt.txt"));
    let count = workspace_root.join(format!("loop-{source}-count.txt"));
    let assertion = cli_command(&launch_area, &home, &bin_dir)
        .env("CLAUDINE_STDIN_FILE", &capture)
        .env("CLAUDINE_COUNT_FILE", &count)
        .env("CLAUDINE_PROVIDER_FIXTURE_MODE", "counting-codex")
        .args(["compose", "--codex"])
        .arg(&document)
        .assert()
        .success();
    let observation = Observation::new(
        fs::read_to_string(&capture).expect("fake Codex received loop prompt"),
        strip_ansi(&String::from_utf8_lossy(&assertion.get_output().stderr)),
    );

    assert_launch_anchored(source, &observation, "alpha-lib", Some(&launch_repo));
    assert_eq!(
        fs::read_to_string(count).unwrap(),
        "2",
        "wrong loop count for {source}"
    );
}
