#![cfg(unix)]

//! Real CLI regression coverage for launch-context anchoring.

use std::fs;
use std::path::{Path, PathBuf};

mod common;
use common::{TestWorkspace, augmented_path, strip_ansi, write, write_executable};

#[derive(Debug)]
struct Observation {
    prompt: String,
    stderr: String,
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

fn run_document(
    launch_dir: &Path,
    home: &Path,
    bin_dir: &Path,
    document: &Path,
    capture: &Path,
) -> Observation {
    let assertion = assert_cmd::Command::cargo_bin("claudine")
        .unwrap()
        .current_dir(launch_dir)
        .env("HOME", home)
        .env("PATH", augmented_path(bin_dir))
        .env("NO_COLOR", "1")
        .env("COLUMNS", "240")
        .env("CLAUDINE_RENDEZVOUS_REPORT", "false")
        .env("CLAUDINE_STDIN_FILE", capture)
        .args(["compose", "--codex"])
        .arg(document)
        .assert()
        .success();

    Observation {
        prompt: fs::read_to_string(capture).expect("fake Codex must receive the composed prompt"),
        stderr: strip_ansi(&String::from_utf8_lossy(&assertion.get_output().stderr)),
    }
}

fn run_inline_document(
    launch_dir: &Path,
    home: &Path,
    bin_dir: &Path,
    document: &Path,
    capture: &Path,
) -> Observation {
    let assertion = assert_cmd::Command::cargo_bin("claudine")
        .unwrap()
        .current_dir(launch_dir)
        .env("HOME", home)
        .env("PATH", augmented_path(bin_dir))
        .env("NO_COLOR", "1")
        .env("COLUMNS", "240")
        .env("CLAUDINE_RENDEZVOUS_REPORT", "false")
        .env("CLAUDINE_STDIN_FILE", capture)
        .args(["inline-compose", "--goose"])
        .arg(document)
        .assert()
        .success();

    Observation {
        prompt: fs::read_to_string(capture).expect("fake Goose must receive the inline prompt"),
        stderr: strip_ansi(&String::from_utf8_lossy(&assertion.get_output().stderr)),
    }
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
    let actual = fs::canonicalize(actual)
        .unwrap_or_else(|_| panic!("`{label}` did not contain a usable path: {actual:?}"));
    let expected = fs::canonicalize(expected).unwrap();
    assert_eq!(actual, expected, "wrong `{label}` path in:\n{text}");
}

fn assert_launch_anchored(
    case: &str,
    observation: &Observation,
    expected_area: &str,
    expected_repo: Option<&Path>,
) {
    assert_eq!(
        field(&observation.prompt, "baseline.body.area"),
        expected_area,
        "wrong body area for {case}"
    );
    assert_eq!(
        field(&observation.prompt, "baseline.frontmatter.area"),
        expected_area
    );
    if observation.prompt.contains("baseline.preflight.area=[") {
        assert_eq!(
            field(&observation.prompt, "baseline.preflight.area"),
            expected_area,
            "preflight-expanded command bytes must use the epoch snapshot"
        );
    }
    match expected_repo {
        Some(expected_repo) => {
            assert_path_field(&observation.prompt, "baseline.body.repo", expected_repo);
            assert_path_field(
                &observation.prompt,
                "baseline.frontmatter.repo",
                expected_repo,
            );
        }
        None => {
            assert_eq!(field(&observation.prompt, "baseline.body.repo"), "");
            assert_eq!(field(&observation.prompt, "baseline.frontmatter.repo"), "");
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

#[test]
fn cli_uses_launch_context_across_launch_source_matrix() {
    let workspace = TestWorkspace::named("ctx-launch-anchor-matrix");
    // macOS spells its temporary root through both `/var` and `/private/var`.
    // Use one concrete spelling throughout this baseline so the only variable
    // under test is launch-versus-source anchoring.
    let workspace_root = fs::canonicalize(workspace.path()).unwrap();
    let launch_repo = workspace_root.join("launch-repo");
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

    let cases = [
        (
            "launch-area",
            launch_area.join("prompt.md"),
        ),
        (
            "opposing-area",
            opposing_area.join("prompt.md"),
        ),
        (
            "external-repo",
            external_repo.join("prompt.md"),
        ),
        ("repo-root", launch_repo.join("prompt.md")),
    ];

    for (name, document) in cases {
        write(
            &document,
            &baseline_document(Some("alpha-lib"), Some(&launch_repo)),
        );
        let capture = workspace_root.join(format!("{name}-prompt.txt"));
        let observation = run_document(&launch_area, &home, &bin_dir, &document, &capture);
        assert_launch_anchored(name, &observation, "alpha-lib", Some(&launch_repo));
    }
}

#[test]
fn cli_keeps_launch_repository_facts_absent_when_source_is_in_repo() {
    let workspace = TestWorkspace::named("ctx-launch-anchor-inverse");
    let workspace_root = fs::canonicalize(workspace.path()).unwrap();
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
    let workspace_root = fs::canonicalize(workspace.path()).unwrap();
    let launch_repo = workspace_root.join("launch-repo");
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
    assert_eq!(field(&observation.prompt, "resolved.spec"), "spec.md");
    assert!(observation.prompt.contains("SOURCE SPEC"));
    assert!(!observation.prompt.contains("LAUNCH DECOY"));
    assert_path_field(&observation.prompt, "launch.repo", &launch_repo);
}

#[test]
fn inline_cli_uses_launch_context_across_launch_source_matrix() {
    let workspace = TestWorkspace::named("ctx-launch-anchor-inline-matrix");
    let workspace_root = fs::canonicalize(workspace.path()).unwrap();
    let launch_repo = workspace_root.join("launch-repo");
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

    for (name, document) in [
        ("launch-area", launch_area.join("inline.md")),
        ("opposing-area", opposing_area.join("inline.md")),
        ("repo-root", launch_repo.join("inline.md")),
        ("external-repo", external_repo.join("inline.md")),
    ] {
        write(&document, &inline_document(&launch_repo));
        let capture = workspace_root.join(format!("inline-{name}-prompt.txt"));
        let observation = run_inline_document(&launch_area, &home, &bin_dir, &document, &capture);
        assert_launch_anchored(name, &observation, "alpha-lib", Some(&launch_repo));
    }
}

#[test]
fn cli_loop_reuses_launch_context_for_root_and_package_prompt_copies() {
    let workspace = TestWorkspace::named("ctx-launch-anchor-loop-pair");
    let workspace_root = fs::canonicalize(workspace.path()).unwrap();
    let launch_repo = workspace_root.join("launch-repo");
    let (launch_area, _) = write_monorepo(&launch_repo);
    write(&launch_repo.join(".darkmatter-shell-whitelist"), "prefix printf\n");

    let home = workspace_root.join("home");
    let bin_dir = workspace_root.join("bin");
    fs::create_dir_all(home.join(".claudine")).unwrap();
    write(&home.join(".claudine/config.json"), "{}");
    stage_counting_codex(&bin_dir);

    for (name, document) in [
        ("repo-root", launch_repo.join("loop.md")),
        ("launch-area", launch_area.join("loop.md")),
    ] {
        write(&document, &loop_document(&launch_repo));
        let capture = workspace_root.join(format!("loop-{name}-prompt.txt"));
        let count = workspace_root.join(format!("loop-{name}-count.txt"));
        let assertion = assert_cmd::Command::cargo_bin("claudine")
            .unwrap()
            .current_dir(&launch_area)
            .env("HOME", &home)
            .env("PATH", augmented_path(&bin_dir))
            .env("NO_COLOR", "1")
            .env("COLUMNS", "240")
            .env("CLAUDINE_RENDEZVOUS_REPORT", "false")
            .env("CLAUDINE_STDIN_FILE", &capture)
            .env("CLAUDINE_COUNT_FILE", &count)
            .args(["compose", "--codex"])
            .arg(&document)
            .assert()
            .success();
        let observation = Observation {
            prompt: fs::read_to_string(&capture).expect("fake Codex received loop prompt"),
            stderr: strip_ansi(&String::from_utf8_lossy(&assertion.get_output().stderr)),
        };

        assert_launch_anchored(name, &observation, "alpha-lib", Some(&launch_repo));
        assert_eq!(fs::read_to_string(count).unwrap(), "2", "wrong loop count for {name}");
    }
}
