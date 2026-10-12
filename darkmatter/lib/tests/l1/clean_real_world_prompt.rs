//! Cleanup over a real shipped prompt: `prompts/commit.md` as it was at blob
//! `464e2d90e`, a document the old serializer corrupted (it re-wrapped prose
//! and moved `::block` directives out of their lists).
//!
//! Under the defaults its only edit is a `text` label on its one unlabeled
//! fence: every hard wrap, directive line, and list stays where the author put
//! it, and the cleaned document composes to the same bytes as the original.
//!
//! Composition is fixture-isolated: a fresh empty Git repository in a
//! temporary directory, with stand-ins for the two files the prompt
//! transcludes. No shell command runs and nothing is read remotely.

use std::convert::Infallible;
use std::path::Path;

use darkmatter::markdown::Markdown;
use darkmatter::markdown::cleanup::rules::MarkdownRuleOverrides;
use darkmatter::markdown::cleanup::{CleanOutcome, CleanRequest, CleanResult, clean_document};
use darkmatter::markdown::compose::{ComposeContext, ComposeOptions};
use darkmatter::markdown::schemas::CleanSchemaConfig;
use pulldown_cmark::{Event, Parser};
use serde_json::json;
use tempfile::TempDir;

const COMMIT_PROMPT: &str = include_str!("../fixtures/clean/prompts-commit-464e2d90e.md");
const PROVENANCE: &str = include_str!("../fixtures/clean/prompts-commit-464e2d90e.provenance.yaml");

/// The one line the defaults change: the example commit's unlabeled fence.
const UNLABELED_FENCE: &str = "    - Example Commit:\n\n        ```\n";
const LABELED_FENCE: &str = "    - Example Commit:\n\n        ```text\n";

fn clean(source: &str) -> CleanResult {
    let overrides = MarkdownRuleOverrides::default();
    let request = CleanRequest { source, overrides: &overrides, workspace: None, document_path: None };
    clean_document(&request, || {
        Ok::<_, Infallible>(CleanSchemaConfig::new(crate::request_support::cwd_context()))
    })
    .expect("clean runs")
}

/// Composes `source` as `commit.md` in `repo`, with `state` as external state.
fn compose(repo: &Path, source: &str, state: serde_json::Value) -> String {
    let path = repo.join("commit.md");
    std::fs::write(&path, source).unwrap();
    let options = ComposeOptions::new_with_context(ComposeContext::capture_for_content(repo, source))
        .with_source_file(path.clone())
        .with_external_state(state);
    let (composed, _) = Markdown::try_from(path.as_path())
        .expect("the prompt loads")
        .compose_with(&crate::request_support::request(options))
        .expect("the prompt composes");
    composed.content().to_string()
}

fn isolated_repo() -> TempDir {
    let dir = TempDir::new().unwrap();
    gix::init(dir.path()).expect("initialize fixture repository");
    std::fs::write(dir.path().join("_headless-orchestration.md"), "Headless stand-in.\n").unwrap();
    std::fs::create_dir_all(dir.path().join(".claudine/memory")).unwrap();
    std::fs::write(dir.path().join(".claudine/memory/commits.md"), "Lessons stand-in.\n").unwrap();
    dir
}

fn soft_breaks(markdown: &str) -> usize {
    Parser::new(markdown).filter(|event| matches!(event, Event::SoftBreak)).count()
}

#[test]
fn the_fixture_is_the_recorded_blob_byte_for_byte() {
    let recorded: serde_yaml_ng::Value = serde_yaml_ng::from_str(PROVENANCE).expect("provenance parses");
    assert_eq!(recorded["source"].as_str(), Some("prompts/commit.md"));
    let blob = gix::objs::compute_hash(gix::hash::Kind::Sha1, gix::objs::Kind::Blob, COMMIT_PROMPT.as_bytes())
        .expect("hash the fixture")
        .to_string();
    assert_eq!(Some(blob.as_str()), recorded["blob"].as_str());
    assert!(blob.starts_with("464e2d90e"), "{blob}");
}

#[test]
fn defaults_accept_the_prompt_and_only_label_its_unlabeled_fence() {
    assert_eq!(COMMIT_PROMPT.matches(UNLABELED_FENCE).count(), 1, "the fixture has one unlabeled fence");
    let result = clean(COMMIT_PROMPT);

    assert_eq!(result.outcome, CleanOutcome::Changed);
    assert!(
        result.diagnostics.iter().all(|diagnostic| diagnostic.fallback.is_none()),
        "the guard accepted the candidate: {:?}",
        result.diagnostics
    );
    assert!(result.applied.is_empty(), "no frontmatter repair is applied: {:?}", result.applied);
    assert_eq!(result.text, COMMIT_PROMPT.replacen(UNLABELED_FENCE, LABELED_FENCE, 1));

    // Every hard wrap the author wrote is still a line break.
    assert_eq!(result.text.lines().count(), COMMIT_PROMPT.lines().count());
    assert!(soft_breaks(COMMIT_PROMPT) > 0, "the prompt has wrapped prose to keep");
    assert_eq!(soft_breaks(&result.text), soft_breaks(COMMIT_PROMPT));

    let again = clean(&result.text);
    assert_eq!(again.outcome, CleanOutcome::Unchanged);
    assert_eq!(again.text, result.text);
}

#[test]
fn the_cleaned_prompt_composes_to_the_same_bytes_as_the_original() {
    let cleaned = clean(COMMIT_PROMPT).text;
    let repo = isolated_repo();
    // `message` gates two `::block` regions, one inside a list.
    for state in [json!({}), json!({ "message": "Tidy the cleanup docs" })] {
        let original = compose(repo.path(), COMMIT_PROMPT, state.clone());
        let after = compose(repo.path(), &cleaned, state.clone());
        assert!(after.contains("# Commit Staged Files"), "{after}");
        assert!(after.contains("Lessons stand-in.") && after.contains("Headless stand-in."));
        assert!(after.lines().all(|line| !line.trim_start().starts_with("::")), "directives are consumed:\n{after}");
        assert_eq!(after.contains("Tidy the cleanup docs"), state.get("message").is_some());
        assert_eq!(after, original, "state {state}");
    }
}
