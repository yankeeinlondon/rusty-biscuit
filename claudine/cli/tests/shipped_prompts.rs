use darkmatter::markdown::Markdown;
use std::path::{Path, PathBuf};

mod common;
use common::{augmented_path, write, write_executable};

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("claudine/cli parent")
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

fn collect_markdown_files(dir: &Path, files: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", dir.display()))
    {
        let path = entry.expect("prompt directory entry").path();
        if path.is_dir() {
            collect_markdown_files(&path, files);
        } else if path.extension().is_some_and(|extension| extension == "md") {
            files.push(path);
        }
    }
}

#[test]
fn shipped_prompt_corpus_parses_frontmatter() {
    let prompts = workspace_root().join("prompts");
    let mut files = Vec::new();
    collect_markdown_files(&prompts, &mut files);
    files.sort();

    assert!(!files.is_empty(), "the shipped prompt corpus must not be empty");
    let failures = files
        .iter()
        .filter_map(|path| {
            Markdown::try_from(path.as_path())
                .err()
                .map(|error| format!("{}: {error}", path.display()))
        })
        .collect::<Vec<_>>();
    assert!(
        failures.is_empty(),
        "shipped prompts with invalid Markdown/frontmatter:\n{}",
        failures.join("\n")
    );
}

#[cfg(unix)]
#[test]
fn shipped_implement_prompt_runs_real_router_target() {
    let fixture = tempfile::tempdir().expect("fixture directory");
    let feature = fixture.path().join("features/2026-07-20-router-fixture");
    let review = feature.join("review.md");
    write(&review, "---\nimplemented: false\n---\n# Router fixture\n");

    let bin_dir = fixture.path().join("bin");
    std::fs::create_dir_all(&bin_dir).expect("provider bin directory");
    let capture = fixture.path().join("delivered-prompt.txt");
    write_executable(
        &bin_dir.join("claude"),
        r#"#!/bin/sh
{
  for arg in "$@"; do
    if [ -f "$arg" ]; then cat "$arg"; else printf '%s\n' "$arg"; fi
  done
  cat
} >> "$CLAUDINE_PROMPT_CAPTURE" 2>/dev/null
exit 0
"#,
    );

    let root = workspace_root();
    let router = root.join("prompts/implement.md");
    let review_arg = format!("review={}", review.display());
    assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", fixture.path())
        .env("PATH", augmented_path(&bin_dir))
        .env("CLAUDINE_PROMPT_CAPTURE", &capture)
        .current_dir(&root)
        .args([
            "compose",
            "--claude",
            router.to_str().expect("UTF-8 router path"),
            &review_arg,
        ])
        .assert()
        .success();

    let delivered = std::fs::read_to_string(&capture).expect("captured provider prompt");
    assert!(
        delivered.contains("Implementation of Review Findings"),
        "the shipped router must deliver the resolved target prompt:\n{delivered}"
    );
    assert!(
        !delivered.contains("This prompt should never be reached"),
        "the router body must not reach the provider after proxy handoff:\n{delivered}"
    );
}
