//! A Markdown document saved whole lays out its frontmatter for the file it
//! is written to: the lifecycle frontmatter verbs (`EffectEngine::save`) take
//! the preference Sniff resolves for the destination, and output with no
//! destination (`Markdown::as_string`) nests four spaces.

use biscuit_file::canonicalize_simplified;
use darkmatter::config::write::destination_indentation;
use darkmatter::effects::EffectEngine;
use darkmatter::markdown::Markdown;
use serde_json::{Value, json};
use std::path::PathBuf;

/// A Git worktree whose `.editorconfig` asks for two-space Markdown.
fn two_space_repository() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let root = canonicalize_simplified(dir.path()).unwrap();
    git2::Repository::init(&root).unwrap();
    std::fs::write(root.join(".editorconfig"), "root = true\n\n[*.md]\nindent_style = space\nindent_size = 2\n").unwrap();
    (dir, root)
}

fn frontmatter(text: &str) -> Value {
    let md = Markdown::try_from_content(text.to_string()).unwrap();
    serde_json::to_value(md.frontmatter().as_map()).unwrap()
}

fn nested() -> Value {
    json!({"tags": ["a", "b"], "owner": {"name": "x"}})
}

#[test]
fn a_lifecycle_save_takes_the_destinations_editorconfig_width_for_a_relative_path() {
    let (_dir, root) = two_space_repository();
    std::fs::create_dir(root.join("notes")).unwrap();
    std::fs::write(root.join("notes/doc.md"), "---\ntitle: T\n---\nBody\n").unwrap();
    let engine = EffectEngine::builder().mutation_root(&root).build();

    engine.set_frontmatter("notes/doc.md", "meta", nested()).unwrap();

    let text = std::fs::read_to_string(root.join("notes/doc.md")).unwrap();
    // The whole-document rewrite drops the body's final line break, as it
    // always has; only the frontmatter layout is under test here.
    assert_eq!(text, "---\ntitle: T\nmeta:\n  tags:\n    - a\n    - b\n  owner:\n    name: x\n---\nBody");
    assert_eq!(frontmatter(&text), json!({"title": "T", "meta": nested()}));
}

#[test]
fn a_lifecycle_save_into_a_file_it_just_created_uses_the_destination_width() {
    let (_dir, root) = two_space_repository();
    let engine = EffectEngine::builder().mutation_root(&root).build();
    engine.ensure_dir("fresh").unwrap();
    engine.ensure_file("fresh/doc.md").unwrap();

    engine.set_frontmatter("fresh/doc.md", "meta", nested()).unwrap();

    let text = std::fs::read_to_string(root.join("fresh/doc.md")).unwrap();
    assert_eq!(text, "---\nmeta:\n  tags:\n    - a\n    - b\n  owner:\n    name: x\n---\n");
    assert_eq!(frontmatter(&text), json!({"meta": nested()}));
}

#[test]
fn two_identical_lifecycle_saves_leave_the_bytes_unchanged() {
    let (_dir, root) = two_space_repository();
    std::fs::write(root.join("doc.md"), "---\ntitle: T\n---\nBody\n").unwrap();
    let engine = EffectEngine::builder().mutation_root(&root).build();

    engine.set_frontmatter("doc.md", "meta", nested()).unwrap();
    let first = std::fs::read(root.join("doc.md")).unwrap();
    engine.set_frontmatter("doc.md", "meta", nested()).unwrap();

    assert_eq!(std::fs::read(root.join("doc.md")).unwrap(), first);
}

#[test]
fn a_not_yet_existing_destination_resolves_its_worktrees_preference() {
    let (_dir, root) = two_space_repository();
    let destination = root.join("missing/dir/doc.md");
    let mut md = Markdown::new("Body\n".to_string());
    md.fm_insert("meta", nested()).unwrap();

    let text = md.as_string_with_indentation(destination_indentation(&destination));

    assert_eq!(text, "---\nmeta:\n  tags:\n    - a\n    - b\n  owner:\n    name: x\n---\nBody\n");
    assert_eq!(frontmatter(&text), json!({"meta": nested()}));
}

#[test]
fn output_with_no_destination_nests_four_spaces() {
    let mut md = Markdown::new("Body\n".to_string());
    md.fm_insert("meta", nested()).unwrap();

    let text = md.as_string();

    assert_eq!(text, "---\nmeta:\n    tags:\n        - a\n        - b\n    owner:\n        name: x\n---\nBody\n");
    assert_eq!(frontmatter(&text), json!({"meta": nested()}));
}

/// Authored spelling the whole-document rewrite would lose: quotes, a
/// comment, and the body's final line break.
const AUTHORED_SAME: &str = "---\ntitle: 'same' # keep\n---\nBody\n";

/// `doc.md` holding [`AUTHORED_SAME`] in a two-space worktree, dated in the
/// past so a rewrite of identical bytes would still show in its mtime.
fn authored_document() -> (tempfile::TempDir, PathBuf, EffectEngine, std::time::SystemTime) {
    let (dir, root) = two_space_repository();
    let path = root.join("doc.md");
    std::fs::write(&path, AUTHORED_SAME).unwrap();
    let past = std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_000_000_000);
    std::fs::File::options().write(true).open(&path).unwrap().set_modified(past).unwrap();
    let engine = EffectEngine::builder().mutation_root(&root).build();
    (dir, path, engine, past)
}

fn assert_untouched(path: &std::path::Path, past: std::time::SystemTime) {
    assert_eq!(std::fs::read_to_string(path).unwrap(), AUTHORED_SAME);
    assert_eq!(std::fs::metadata(path).unwrap().modified().unwrap(), past, "the file was rewritten");
}

#[test]
fn setting_a_property_to_the_value_it_holds_leaves_the_file_unwritten() {
    let (_dir, path, engine, past) = authored_document();

    let prior = engine.set_frontmatter("doc.md", "title", json!("same")).unwrap();

    assert_eq!(prior, json!("same"));
    assert_untouched(&path, past);
}

#[test]
fn merging_values_that_already_hold_leaves_the_file_unwritten() {
    let (_dir, path, engine, past) = authored_document();

    let merged = engine.merge_frontmatter("doc.md", json!({"title": "same"})).unwrap();

    assert_eq!(merged, json!({"title": "same"}));
    assert_untouched(&path, past);
}

#[test]
fn deleting_an_absent_property_leaves_the_file_unwritten() {
    let (_dir, path, engine, past) = authored_document();

    let removed = engine.delete_frontmatter("doc.md", "missing").unwrap();

    assert_eq!(removed, Value::Null);
    assert_untouched(&path, past);
}

#[test]
fn control_a_changed_value_is_still_written_with_the_destination_width() {
    let (_dir, path, engine, _past) = authored_document();

    let prior = engine.set_frontmatter("doc.md", "meta", nested()).unwrap();

    assert_eq!(prior, Value::Null);
    let text = std::fs::read_to_string(&path).unwrap();
    assert_eq!(text, "---\ntitle: same\nmeta:\n  tags:\n    - a\n    - b\n  owner:\n    name: x\n---\nBody");
}
