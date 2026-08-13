//! End-to-end sequence coverage for invocation-anchored prepared context.

use std::fs;
use std::path::{Path, PathBuf};

use pulldown_cmark::{Event, Options, Parser, TagEnd};
use tempfile::TempDir;

mod common;
use common::strip_ansi;

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

/// Path of the launch repository for a fixture rooted at `workspace_root`.
///
/// Fails loudly rather than degrading quietly: a fixture that stopped crossing
/// the separator-then-`.` boundary would still pass every assertion below while
/// no longer reproducing the regression.
fn launch_repository(workspace_root: &Path) -> PathBuf {
    let repo = workspace_root
        .join(HIDDEN_LAUNCH_PARENT)
        .join("launch-repository");
    let boundary = format!("{}{HIDDEN_LAUNCH_PARENT}", std::path::MAIN_SEPARATOR);
    assert!(
        repo.to_string_lossy().contains(boundary.as_str()),
        "the launch repository must sit behind a `{boundary}` boundary: {}",
        repo.display()
    );
    repo
}

/// Literal text of a composed Markdown document.
///
/// An interpolated scalar is serialized with whatever escaping its literal text
/// requires, so the bytes the CLI prints and the values they denote are two
/// different strings. Inline code contributes its content without its
/// delimiters, so a value that escaped into a code span is reported as the text
/// it is *not*, rather than being reassembled back into a passing comparison.
/// Path as the product projects it, never as `std::fs::canonicalize` spells it.
///
/// On Windows `canonicalize` returns the verbatim `\\?\` form, which is a
/// comparison/cache-key shape that never reaches a composed `ctx.*` value. A
/// fixture that expected it would demand output the product must not produce.
fn projected_path(path: &Path) -> PathBuf {
    dunce::canonicalize(path).unwrap_or_else(|_| dunce::simplified(path).to_path_buf())
}

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

struct Repositories {
    _root: TempDir,
    launch_repo: PathBuf,
    launch_area: PathBuf,
    source_repo: PathBuf,
    home: PathBuf,
}

impl Repositories {
    fn new() -> Self {
        let root = TempDir::new().unwrap();
        let launch_repo = launch_repository(root.path());
        let launch_area = launch_repo.join("alpha/lib");
        let source_repo = root.path().join("source-repository");
        let home = root.path().join("home");
        fs::create_dir_all(launch_area.join("src")).unwrap();
        fs::create_dir_all(launch_repo.join("beta/lib/src")).unwrap();
        fs::create_dir_all(&source_repo).unwrap();
        fs::create_dir_all(&home).unwrap();
        init_repo(&launch_repo);
        init_repo(&source_repo);
        fs::write(
            launch_repo.join("Cargo.toml"),
            "[workspace]\nmembers = [\"alpha/lib\", \"beta/lib\"]\nresolver = \"2\"\n",
        )
        .unwrap();
        fs::write(
            launch_area.join("Cargo.toml"),
            "[package]\nname = \"alpha-lib\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
        )
        .unwrap();
        fs::write(launch_area.join("src/lib.rs"), "").unwrap();
        fs::write(
            launch_repo.join("beta/lib/Cargo.toml"),
            "[package]\nname = \"beta-lib\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
        )
        .unwrap();
        fs::write(launch_repo.join("beta/lib/src/lib.rs"), "").unwrap();
        let launch_repo = projected_path(&launch_repo);
        let launch_area = projected_path(&launch_area);
        let source_repo = projected_path(&source_repo);
        let home = projected_path(&home);
        Self {
            _root: root,
            launch_repo,
            launch_area,
            source_repo,
            home,
        }
    }

    fn run(&self, document: &Path, extra: &[&str]) -> std::process::Output {
        let mut args = vec![
            "sequence",
            "--dry-run",
            "--claude",
            "--model",
            "claude-sonnet-4-6",
            "--yolo",
        ];
        args.extend_from_slice(extra);
        args.push(document.to_str().unwrap());
        assert_cmd::Command::cargo_bin("claudine")
            .unwrap()
            .env("NO_COLOR", "1")
            .env("HOME", &self.home)
            .current_dir(&self.launch_area)
            .args(args)
            .assert()
            .get_output()
            .clone()
    }
}

fn init_repo(path: &Path) {
    fs::create_dir_all(path).unwrap();
    let status = std::process::Command::new("git")
        .args(["init", "--quiet"])
        .current_dir(path)
        .status()
        .expect("git must be available for repository-context tests");
    assert!(status.success());
}

fn plain(bytes: &[u8]) -> String {
    strip_ansi(&String::from_utf8_lossy(bytes))
}

#[test]
fn external_sequence_uses_launch_facts_and_source_relative_schema_and_file_reads() {
    let repos = Repositories::new();
    fs::write(
        repos.launch_area.join("schema.yaml"),
        "$schema:\n  marker: number(required)\n",
    )
    .unwrap();
    fs::write(
        repos.launch_area.join("marker.md"),
        "---\nvalue: launch-marker\n---\nLaunch.\n",
    )
    .unwrap();
    fs::write(
        repos.source_repo.join("schema.yaml"),
        "kind: schema\ntypes:\n  marker: string(required)\n",
    )
    .unwrap();
    fs::write(
        repos.source_repo.join("marker.md"),
        "---\nvalue: source-marker\n---\nSource.\n",
    )
    .unwrap();
    let sequence = repos.source_repo.join("sequence.md");
    fs::write(
        &sequence,
        r#"---
$schema: ./schema.yaml
marker: source-value
sequence:
  - alpha
---
Step area=[{{ ctx.area }}] root=[{{ ctx.repo_root }}] agent=[{{ ctx.agent }}] model=[{{ ctx.model }}] env-agent=[{{ env.AGENT }}] env-model=[{{ env.MODEL }}] file=[{{ frontmatter('marker.md', 'value') }}].
"#,
    )
    .unwrap();

    let output = repos.run(&sequence, &[]);
    let stdout = plain(&output.stdout);
    let text = parsed_text(&stdout);
    let stderr = plain(&output.stderr);
    assert!(output.status.success(), "stderr:\n{stderr}");
    assert!(text.contains("area=[alpha-lib]"), "stdout:\n{stdout}");
    assert!(
        text.contains(&format!("root=[{}]", repos.launch_repo.display())),
        "expected root=[{}]\nparsed:\n{text}\nraw stdout:\n{stdout}",
        repos.launch_repo.display()
    );
    for expected in [
        "agent=[claude]",
        "model=[claude-sonnet-4-6]",
        "env-agent=[claude]",
        "env-model=[claude-sonnet-4-6]",
        "file=[source-marker]",
    ] {
        assert!(text.contains(expected), "missing `{expected}`; stdout:\n{stdout}");
    }
    assert!(!text.contains("launch-marker"), "stdout:\n{stdout}");
}

#[test]
fn graph_preflight_rejects_all_target_identity_roots_across_yaml_scalar_forms() {
    let repos = Repositories::new();
    for (name, shell, root) in [
        ("double", "\"echo {{ ctx.agent }}\"", "ctx.agent"),
        ("single", "'echo {{ ctx.model }}'", "ctx.model"),
        ("array", "[\"echo {{ env.AGENT }}\"]", "env.AGENT"),
        ("folded", ">-\n        echo {{ env.MODEL }}", "env.MODEL"),
    ] {
        let sequence = repos.source_repo.join(format!("{name}.md"));
        fs::write(
            &sequence,
            format!(
                "---\nsequence:\n  - name: {name}\n    shell: {shell}\n---\nBody.\n"
            ),
        )
        .unwrap();

        let output = repos.run(&sequence, &[]);
        let stderr = plain(&output.stderr);
        assert!(!output.status.success(), "`{root}` unexpectedly passed");
        assert!(stderr.contains(root), "`{root}` not named; stderr:\n{stderr}");
        assert!(stderr.contains("task-scoped"), "stderr:\n{stderr}");
        assert!(stderr.contains("preflight"), "stderr:\n{stderr}");
    }
}

#[test]
fn serial_and_parallel_prompt_tasks_keep_target_identity_and_launch_facts() {
    let repos = Repositories::new();
    for name in ["serial-a", "serial-b", "parallel-a", "parallel-b"] {
        fs::write(
            repos.source_repo.join(format!("{name}.md")),
            format!(
                "Task {name} area=[{{{{ ctx.area }}}}] root=[{{{{ ctx.repo_root }}}}] \
                 agent=[{{{{ ctx.agent }}}}] model=[{{{{ ctx.model }}}}] \
                 env-agent=[{{{{ env.AGENT }}}}] env-model=[{{{{ env.MODEL }}}}].\n"
            ),
        )
        .unwrap();
    }
    let sequence = repos.source_repo.join("groups.md");
    fs::write(
        &sequence,
        r#"---
sequence:
  - name: serial
    group:
      name: serial-group
      execution: serial
      tasks:
        - prompt: serial-a.md
        - prompt: serial-b.md
  - name: parallel
    group:
      name: parallel-group
      execution: parallel
      tasks:
        - prompt: parallel-a.md
        - prompt: parallel-b.md
---
Unused body.
"#,
    )
    .unwrap();

    let output = repos.run(&sequence, &[]);
    let stdout = plain(&output.stdout);
    let text = parsed_text(&stdout);
    let stderr = plain(&output.stderr);
    assert!(output.status.success(), "stderr:\n{stderr}");
    for name in ["serial-a", "serial-b", "parallel-a", "parallel-b"] {
        assert!(text.contains(&format!("Task {name}")), "stdout:\n{stdout}");
    }
    for expected in [
        "area=[alpha-lib]",
        " agent=[claude]",
        " model=[claude-sonnet-4-6]",
        " env-agent=[claude]",
        " env-model=[claude-sonnet-4-6]",
    ] {
        assert_eq!(
            text.matches(expected).count(),
            4,
            "`{expected}` did not reach every task; stdout:\n{stdout}"
        );
    }
    assert_eq!(
        text.matches(&format!("root=[{}]", repos.launch_repo.display()))
            .count(),
        4,
        "launch root changed in a task; expected root=[{}]\nparsed:\n{text}\nraw stdout:\n{stdout}",
        repos.launch_repo.display()
    );
    // The leak check reads the parsed text too: an escaped separator would hide
    // a source-repository path from a raw substring search on Windows.
    assert!(!text.contains(&repos.source_repo.display().to_string()));
}
